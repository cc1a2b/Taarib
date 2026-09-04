//! فولكان — the overlay as a *layer*, which is the only one of the four that is
//! invited rather than installed.
//!
//! Direct3D and OpenGL are reached by rewriting somebody's function table. It
//! works, it is correct, and it is indistinguishable at the instruction level
//! from what a cheat does — which is why an anti-cheat system cannot tell them
//! apart and why a user cannot remove one without trusting the thing that
//! installed it. Vulkan has a documented extension point: a JSON manifest in a
//! directory the loader scans, naming a library that exports three functions.
//! The loader enumerates it, inserts it into the dispatch chain, and removes it
//! when the file is deleted. Nothing is patched, nothing is written into another
//! module's memory, and `vulkan-1.dll` never has a byte changed.
//!
//! That is the whole reason [`WajihatRusum::jamee`] probes Vulkan first.
//!
//! ## The dispatch chain, and why it cannot be faked
//!
//! A layer is not a hook. When the application calls `vkQueuePresentKHR`, the
//! loader's trampoline looks the command up in a per-object dispatch table and
//! lands in the *first* layer's implementation; that layer does its work and
//! calls the *next* layer's implementation, and so on down to the driver. A
//! layer therefore has to know two things it is not simply handed:
//!
//! 1. **Who is next.** Found by walking `pNext` on `VkInstanceCreateInfo` and
//!    `VkDeviceCreateInfo` for a `VK_STRUCTURE_TYPE_LOADER_*_CREATE_INFO` whose
//!    `function` is `VK_LAYER_LINK_INFO`, reading `pfnNextGetInstanceProcAddr`
//!    out of it — and then **advancing the link** so the next layer down finds
//!    its own. Forgetting the advance makes every layer below this one resolve
//!    against this one, which is an infinite recursion on the first call.
//! 2. **Which object is which.** Dispatchable handles — `VkInstance`,
//!    `VkPhysicalDevice`, `VkDevice`, `VkQueue`, `VkCommandBuffer` — begin with
//!    a pointer to the loader's dispatch table, and that pointer is the same for
//!    every object descended from the same parent. So a `VkQueue`'s first word
//!    identifies the `VkDevice` it came from, which is how
//!    the present hook finds its device's state from a handle that carries
//!    no device. This is the documented "loader key" and it is the one piece of
//!    the protocol that reads like a hack and is not.
//!
//! ## The semaphore rewiring
//!
//! This is the part that is subtle, and getting it wrong produces tearing,
//! validation errors, or a frame where the overlay is drawn *underneath* the
//! game because the driver was still writing when we read.
//!
//! The application presents with `pWaitSemaphores = S` — the semaphores its own
//! rendering signals when the frame is finished. If the overlay simply submitted
//! its draw and then let the present go through unchanged, the present would
//! wait on `S` and be free to run the moment the game's rendering finished,
//! while the overlay's own submission was still in flight. The image would be
//! presented before the Arabic was drawn into it, intermittently, depending on
//! scheduling.
//!
//! So the overlay inserts itself into the dependency chain instead of running
//! beside it:
//!
//! | step | waits on | signals |
//! | --- | --- | --- |
//! | the game's rendering | (its own) | `S` |
//! | **the overlay's submission** | **`S`**, at `COLOR_ATTACHMENT_OUTPUT` | **`T`** |
//! | the present | **`T`** | — |
//!
//! `T` is per swapchain image, and the submission is fenced per swapchain image
//! too, because the command buffer for image *n* cannot be re-recorded until the
//! previous frame that used image *n* has finished with it. The present info the
//! next layer receives is a **copy** with `pWaitSemaphores` replaced — the
//! application's own structure is `const` and is never written to.
//!
//! When the overlay does not draw — the budget was blown, the batch was empty,
//! the user paused it — no submission happens, `T` is not signalled, and the
//! present goes down the chain with the application's original `S`. A layer that
//! rewired unconditionally would deadlock the first time it skipped a frame.
//!
//! ## `LOAD_OP_LOAD`, and the frame that is already there
//!
//! The overlay's render pass loads the colour attachment rather than clearing
//! it. That is the entire difference between an overlay and a black screen: the
//! swapchain image at present time contains the game's finished frame, and the
//! overlay's job is to draw two hundred glyphs on top of it. The initial layout
//! is `PRESENT_SRC_KHR` and the final layout is `PRESENT_SRC_KHR`, because the
//! image arrives ready to present and must leave ready to present — a render
//! pass that transitioned to `COLOR_ATTACHMENT_OPTIMAL` and stopped there would
//! hand the presentation engine an image in the wrong layout.
//!
//! ## The SPIR-V is supplied, not compiled here
//!
//! [`KhattafVulkan`] takes its two shader modules as SPIR-V words. This crate
//! does not embed a GLSL compiler, and the reason is the same reason it does not
//! embed a font rasterizer: it is a library that gets loaded into somebody
//! else's game, and `glslang` is twelve megabytes of code that would be mapped
//! into every Vulkan process on the machine to compile two shaders that never
//! change. The GLSL those modules are built from is [`MASDAR_RAAS`] and
//! [`MASDAR_QITA`], kept in this file so the source and the pipeline that
//! consumes it cannot drift apart, and the workspace's asset step compiles them.
//! Passing them in is the same discipline `taarib-muhawwil-godot` applies to
//! hook addresses: the thing that can only be known outside is passed in, rather
//! than guessed inside.

use core::ffi::{CStr, c_char, c_void};
use core::fmt;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use ash::vk::{self, Handle};
use parking_lot::Mutex;

use crate::khata::KhataTabaqa;
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WajihatRusum, WasfSath,
};

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// The layer's name, as the loader and `vulkaninfo` will report it.
///
/// `VK_LAYER_` prefix, vendor tag, layer tag — the convention the loader
/// documents. It is a stable identifier: a user who has put it in
/// `VK_INSTANCE_LAYERS` or in a per-game launch option must not have that break
/// across a Taarib release.
pub const ISM_TABAQA: &str = "VK_LAYER_taarib_tabaqa";

/// The environment variable that turns the implicit layer on.
///
/// An implicit layer is enumerated for **every** Vulkan application on the
/// machine, which is exactly what makes it convenient and exactly what makes it
/// dangerous. Gating it behind a variable the launcher sets means a manifest
/// left behind by a failed uninstall is inert rather than active, and that a
/// Vulkan application Taarib has nothing to do with never loads this library
/// into its address space.
pub const MUTAGHAYYIR_TAFEEL: &str = "TAARIB_TABAQA";

/// The environment variable that forces the layer off.
///
/// The loader requires an implicit layer to declare one, and it is the escape
/// hatch a player uses when the overlay is what is breaking their game: setting
/// it means the library is not loaded at all, which is stronger than any
/// in-process disable the overlay could offer itself.
pub const MUTAGHAYYIR_TATEEL: &str = "TAARIB_TABAQA_MUATTALA";

/// The loader layer interface version this layer implements.
///
/// Version 2 is what supports `vkNegotiateLoaderLayerInterfaceVersion` and
/// uniquely-named exported entry points. Version 1 is the older protocol where
/// a layer had to export `vkGetInstanceProcAddr` under exactly that name, which
/// collides when two layers are statically linked into one library.
const ISDAR_WAJIHAT_TABAQA: u32 = 2;

/// `LAYER_NEGOTIATE_INTERFACE_STRUCT`.
const NAW_TAFAWUD: i32 = 1;

/// `VK_LAYER_LINK_INFO`.
const WAZIFA_RABT: i32 = 0;

// ---------------------------------------------------------------------------
// The manifest
// ---------------------------------------------------------------------------

/// The layer manifest, exactly as the installer writes it to disk.
///
/// It lives here, next to the code, because the two have to agree about five
/// things and there is no build step that checks them: the layer's name, the API
/// version it was written against, the three uniquely-named entry points in the
/// `functions` block, and the two environment variables. A manifest kept in the
/// installer would be a copy of these facts that the compiler cannot see, and
/// the failure when it drifted would be the loader silently declining to load
/// the layer — with no error, because a manifest naming a symbol that does not
/// exist is a manifest the loader skips.
///
/// `library_path` is the one field that is genuinely per-installation. This
/// constant carries the Linux file name; [`bayan`] produces the same document
/// with a real path substituted, and the installer calls that.
pub const NASS_BAYAN: &str = r#"{
    "file_format_version": "1.2.0",
    "layer": {
        "name": "VK_LAYER_taarib_tabaqa",
        "type": "GLOBAL",
        "library_path": "./libtaarib_tabaqa.so",
        "api_version": "1.3.281",
        "implementation_version": "1",
        "description":
            "Taarib overlay: draws shaped Arabic over the presented frame. Needs TAARIB_TABAQA.",
        "functions": {
            "vkNegotiateLoaderLayerInterfaceVersion":
                "taaribTabaqaNegotiateLoaderLayerInterfaceVersion",
            "vkGetInstanceProcAddr": "taaribTabaqaGetInstanceProcAddr",
            "vkGetDeviceProcAddr": "taaribTabaqaGetDeviceProcAddr"
        },
        "enable_environment": {
            "TAARIB_TABAQA": "1"
        },
        "disable_environment": {
            "TAARIB_TABAQA_MUATTALA": "1"
        }
    }
}
"#;

/// The manifest with a real library path in it.
///
/// The path is JSON-escaped rather than interpolated raw, because on Windows it
/// is `C:\Program Files\Taarib\taarib_tabaqa.dll` and a backslash in a JSON
/// string is an escape introducer. A manifest written without this produces
/// `\T` and `\P`, which is invalid JSON, which the loader reports as nothing at
/// all — it skips manifests it cannot parse in silence.
#[must_use]
pub fn bayan(masar_maktaba: &str) -> String {
    let mut masar = String::with_capacity(masar_maktaba.len() + 8);
    for harf in masar_maktaba.chars() {
        match harf {
            '\\' => masar.push_str("\\\\"),
            '"' => masar.push_str("\\\""),
            '\n' => masar.push_str("\\n"),
            '\r' => masar.push_str("\\r"),
            '\t' => masar.push_str("\\t"),
            // Control characters have no place in a path and would produce
            // invalid JSON if one arrived; they are dropped rather than emitted,
            // because a path containing one is already not a path.
            harf if (harf as u32) < 0x20 => {}
            harf => masar.push(harf),
        }
    }
    NASS_BAYAN.replace("./libtaarib_tabaqa.so", &masar)
}

// ---------------------------------------------------------------------------
// The loader's own structures, which no binding crate declares
// ---------------------------------------------------------------------------

/// `VkLayerInstanceLink` — one link in the instance dispatch chain.
///
/// The loader builds one of these per layer and threads them together. A layer
/// reads its own link's `pfnNextGetInstanceProcAddr` and then writes `pNext`
/// back into the create-info's link pointer so the next layer down reads its
/// own. That write is not optional and is not a courtesy: it is how the chain
/// advances.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(
    dead_code,
    reason = "gpdpa_talee is never read and cannot be removed: it is the third word of the \
              loader's own structure, and a Rust transcription missing it would make every \
              subsequent link in the chain read from the wrong offset"
)]
struct RabtTabaqatMithal {
    /// The next layer's link.
    talee: *mut Self,
    /// The next layer's `vkGetInstanceProcAddr`.
    gipa_talee: Option<vk::PFN_vkGetInstanceProcAddr>,
    /// The next layer's `vk_layerGetPhysicalDeviceProcAddr`, which this layer
    /// does not intercept and passes through untouched.
    gpdpa_talee: *mut c_void,
}

/// `VkLayerDeviceLink` — one link in the device dispatch chain.
#[repr(C)]
#[derive(Clone, Copy)]
struct RabtTabaqatJihaz {
    /// The next layer's link.
    talee: *mut Self,
    /// The next layer's `vkGetInstanceProcAddr`, needed because
    /// `vkCreateDevice` is an instance-level command.
    gipa_talee: Option<vk::PFN_vkGetInstanceProcAddr>,
    /// The next layer's `vkGetDeviceProcAddr`.
    gdpa_talee: Option<vk::PFN_vkGetDeviceProcAddr>,
}

/// The union inside `VkLayerInstanceCreateInfo`.
///
/// Two pointers wide, because `layerDevice` is its largest member. Modelled as a
/// real `union` rather than as the one field this layer reads, so that the
/// structure's size and alignment match what the loader allocated — a struct
/// declaring only `pLayerInfo` would be one pointer short, and every field the
/// loader wrote after it would land somewhere else.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(
    dead_code,
    reason = "three of the four members are never read and all four must be declared: the \
              union's size is its largest member's, and omitting them would make the \
              structure narrower than the one the loader allocated"
)]
union IttihadMithal {
    /// `pLayerInfo`, when `function` is `VK_LAYER_LINK_INFO`.
    rabt: *mut RabtTabaqatMithal,
    /// `pfnSetInstanceLoaderData`, when it is `VK_LOADER_DATA_CALLBACK`.
    tahyiat_bayanat: *mut c_void,
    /// `layerDevice`, when it is `VK_LOADER_LAYER_CREATE_DEVICE_CALLBACK`.
    jihaz_tabaqa: [*mut c_void; 2],
    /// `loaderFeatures`, when it is `VK_LOADER_FEATURES`.
    khasais: u32,
}

/// `VkLayerInstanceCreateInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
struct MalumatInshaTabaqatMithal {
    /// `VK_STRUCTURE_TYPE_LOADER_INSTANCE_CREATE_INFO`.
    naw: vk::StructureType,
    /// The rest of the chain.
    talee: *const c_void,
    /// Which of the union's members is live.
    wazifa: i32,
    /// The union.
    ittihad: IttihadMithal,
}

/// The union inside `VkLayerDeviceCreateInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(
    dead_code,
    reason = "the loader-data callback member is never read and must still be declared, for \
              the same layout reason as IttihadMithal"
)]
union IttihadJihaz {
    /// `pLayerInfo`, when `function` is `VK_LAYER_LINK_INFO`.
    rabt: *mut RabtTabaqatJihaz,
    /// `pfnSetDeviceLoaderData`, when it is `VK_LOADER_DATA_CALLBACK`.
    tahyiat_bayanat: *mut c_void,
}

/// `VkLayerDeviceCreateInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
struct MalumatInshaTabaqatJihaz {
    /// `VK_STRUCTURE_TYPE_LOADER_DEVICE_CREATE_INFO`.
    naw: vk::StructureType,
    /// The rest of the chain.
    talee: *const c_void,
    /// Which of the union's members is live.
    wazifa: i32,
    /// The union.
    ittihad: IttihadJihaz,
}

/// `VkNegotiateLayerInterface`, the version-two handshake.
///
/// The loader allocates it, fills `sType` and its own
/// `loaderLayerInterfaceVersion`, and hands it over; the layer writes back the
/// version it implements and the three entry points the loader should use. This
/// is what lets a layer export its functions under names that cannot collide
/// with another layer's in the same library.
#[repr(C)]
#[allow(
    dead_code,
    reason = "pNext is reserved by the protocol and must occupy its word; reading or writing \
              it is exactly what the loader forbids"
)]
struct WajihatTafawud {
    /// `LAYER_NEGOTIATE_INTERFACE_STRUCT`.
    naw: i32,
    /// Reserved by the protocol; untouched.
    talee: *mut c_void,
    /// In: the loader's version. Out: this layer's.
    isdar: u32,
    /// This layer's `vkGetInstanceProcAddr`.
    gipa: Option<vk::PFN_vkGetInstanceProcAddr>,
    /// This layer's `vkGetDeviceProcAddr`.
    gdpa: Option<vk::PFN_vkGetDeviceProcAddr>,
    /// This layer's `vk_layerGetPhysicalDeviceProcAddr`, which is null because
    /// this layer intercepts no physical-device command.
    gpdpa: *mut c_void,
}

/// The dispatch key of a dispatchable handle.
///
/// Every dispatchable Vulkan object starts with a pointer to the loader's
/// dispatch table, and every object created from the same parent shares it. That
/// is what makes a `VkQueue` sufficient to find its `VkDevice`'s state, which is
/// the whole reason the `vkQueuePresentKHR` hook works at all — the present call
/// carries a queue and a swapchain and no device.
///
/// # Safety
///
/// `maqbad` must be a live dispatchable Vulkan handle. Reading the first
/// pointer-sized word of anything else is a read of unrelated memory.
unsafe fn miftah(maqbad: u64) -> usize {
    if maqbad == 0 {
        return 0;
    }
    let jidhr = core::ptr::with_exposed_provenance::<*const c_void>(maqbad_usize(maqbad));
    // SAFETY: the caller guarantees `maqbad` is a live dispatchable handle,
    // whose first pointer-sized word is the loader's dispatch table pointer by
    // the loader's own ABI. One aligned pointer is read and nothing else.
    let jadwal = unsafe { jidhr.read() };
    jadwal.expose_provenance()
}

/// A Vulkan handle's numeric value as a machine address.
///
/// `u64::try_from` in the other direction is exact; this direction is the one
/// that needs saying, because on a 32-bit target a dispatchable handle is a
/// 32-bit pointer widened to `u64` by `Handle::as_raw`, and narrowing it back is
/// lossless for exactly the values that came from a pointer.
fn maqbad_usize(maqbad: u64) -> usize {
    usize::try_from(maqbad).unwrap_or(0)
}

/// The head every `pNext` node begins with.
///
/// Vulkan's extension chain is a linked list of structures whose first two
/// fields are always a type tag and the next pointer. Walking it means reading
/// exactly those two out of a pointer whose real type is not yet known, which is
/// what this is for and the only thing it is used for.
#[repr(C)]
struct RaasBunya {
    /// The structure's type tag.
    naw: vk::StructureType,
    /// The next node, or null.
    talee: *const c_void,
}

/// Walks a `pNext` chain for the loader's instance link information.
///
/// # Safety
///
/// `talee` must be null or the head of a well-formed Vulkan `pNext` chain — that
/// is, every node must begin with a `VkStructureType` and a `pNext` pointer, and
/// every node the chain reaches must be live for the duration of the walk. This
/// is guaranteed for a chain the loader handed to `vkCreateInstance`.
unsafe fn jid_rabt_mithal(mut talee: *const c_void) -> Option<*mut MalumatInshaTabaqatMithal> {
    while !talee.is_null() {
        let raas = talee.cast::<RaasBunya>();
        // SAFETY: the caller guarantees every node in the chain begins with the
        // two fields `RaasBunya` declares and is live; this reads only those.
        let (naw, baad) = unsafe { ((*raas).naw, (*raas).talee) };
        if naw == vk::StructureType::LOADER_INSTANCE_CREATE_INFO {
            let uqda = talee.cast::<MalumatInshaTabaqatMithal>().cast_mut();
            // SAFETY: the tag says this node is a `VkLayerInstanceCreateInfo`,
            // so reading its `function` discriminant is reading a field the
            // loader wrote.
            if unsafe { (*uqda).wazifa } == WAZIFA_RABT {
                return Some(uqda);
            }
        }
        talee = baad;
    }
    None
}

/// Walks a `pNext` chain for the loader's device link information.
///
/// # Safety
///
/// As [`jid_rabt_mithal`], for a chain handed to `vkCreateDevice`.
unsafe fn jid_rabt_jihaz(mut talee: *const c_void) -> Option<*mut MalumatInshaTabaqatJihaz> {
    while !talee.is_null() {
        let raas = talee.cast::<RaasBunya>();
        // SAFETY: as in `jid_rabt_mithal`.
        let (naw, baad) = unsafe { ((*raas).naw, (*raas).talee) };
        if naw == vk::StructureType::LOADER_DEVICE_CREATE_INFO {
            let uqda = talee.cast::<MalumatInshaTabaqatJihaz>().cast_mut();
            // SAFETY: as in `jid_rabt_mithal`.
            if unsafe { (*uqda).wazifa } == WAZIFA_RABT {
                return Some(uqda);
            }
        }
        talee = baad;
    }
    None
}

/// A resolved entry point as the untyped pointer `ash`'s loaders want.
///
/// `vkGetInstanceProcAddr` returns `PFN_vkVoidFunction`, which in `ash` is an
/// `Option<unsafe extern "system" fn()>`, and every loader in `ash` wants a
/// `*const c_void`. The conversion is a pointer-to-pointer cast rather than a
/// transmute, so it stays a cast the compiler checks.
fn ka_muashir(dalla: vk::PFN_vkVoidFunction) -> *const c_void {
    dalla.map_or_else(core::ptr::null, |dalla| (dalla as *const ()).cast::<c_void>())
}

// ---------------------------------------------------------------------------
// The dispatch tables
// ---------------------------------------------------------------------------

/// What the layer knows about one `VkInstance`.
struct JadwalMithal {
    /// The instance, loaded against the next layer's resolver.
    mithal: ash::Instance,
    /// The next layer's `vkGetInstanceProcAddr`.
    gipa_talee: vk::PFN_vkGetInstanceProcAddr,
    /// The next layer's `vkDestroyInstance`.
    itlaf: Option<vk::PFN_vkDestroyInstance>,
    /// The next layer's `vkGetPhysicalDeviceSurfaceCapabilitiesKHR`.
    ///
    /// Resolved so the swapchain hook can *ask* whether `TRANSFER_SRC` is a
    /// supported usage before requesting it, rather than requesting it and
    /// retrying on failure. The retry is what an overlay reaches for first and
    /// it is wrong: `vkCreateSwapchainKHR` retires `oldSwapchain` even when it
    /// fails, so the second attempt would pass a retired swapchain and be
    /// invalid usage on every resize.
    qudurat_sath: Option<vk::PFN_vkGetPhysicalDeviceSurfaceCapabilitiesKHR>,
}

/// Which queue of a device the application asked for, and from which family.
///
/// Recorded by intercepting `vkGetDeviceQueue`, because `vkQueuePresentKHR`
/// hands the layer a `VkQueue` and nothing else, and a command pool has to be
/// created for the queue family the command buffer will be submitted on. There
/// is no query that goes from a queue handle back to its family; the only way to
/// know is to have watched the queue being retrieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaburMasjjal {
    /// The queue handle the application holds.
    tabur: vk::Queue,
    /// Its family index.
    usra: u32,
}

/// What the layer knows about one `VkDevice`.
struct JadwalJihaz {
    /// The device, loaded against the next layer's resolver.
    jihaz: ash::Device,
    /// The instance record this device descends from.
    ///
    /// Held as a strong reference rather than looked up by key at each use: a
    /// game that destroys its instance while a swapchain hook is still running
    /// on another thread would otherwise have the lookup return nothing, and the
    /// dispatch pointers in this table would be the only thing left describing
    /// how to call down.
    sahib: Arc<JadwalMithal>,
    /// The physical device it was created from.
    jihaz_madi: vk::PhysicalDevice,
    /// Its memory types, read once at creation.
    dhakira: vk::PhysicalDeviceMemoryProperties,
    /// The next layer's `vkGetDeviceProcAddr`.
    gdpa_talee: vk::PFN_vkGetDeviceProcAddr,
    /// The next layer's `vkDestroyDevice`.
    itlaf: Option<vk::PFN_vkDestroyDevice>,
    /// The next layer's `vkCreateSwapchainKHR`, absent on a device without the
    /// swapchain extension — a compute-only device, which this layer leaves
    /// entirely alone.
    insha_silsila: Option<vk::PFN_vkCreateSwapchainKHR>,
    /// The next layer's `vkDestroySwapchainKHR`.
    itlaf_silsila: Option<vk::PFN_vkDestroySwapchainKHR>,
    /// The next layer's `vkGetSwapchainImagesKHR`.
    suwar_silsila: Option<vk::PFN_vkGetSwapchainImagesKHR>,
    /// The next layer's `vkQueuePresentKHR`.
    taqdeem: Option<vk::PFN_vkQueuePresentKHR>,
    /// The next layer's `vkGetDeviceQueue`.
    ///
    /// Intercepted only to watch: the queue is passed straight through and the
    /// family it came from is recorded, because there is no way to recover a
    /// queue's family from the queue handle afterwards.
    jib_tabur: Option<vk::PFN_vkGetDeviceQueue>,
    /// The parts that change while the application runs.
    hala: Mutex<HalatJihaz>,
}

/// The mutable half of a device's record.
#[derive(Debug, Default)]
struct HalatJihaz {
    /// Every swapchain this device currently has, and how it was created.
    salasil: Vec<WasfSilsila>,
    /// Every queue the application has retrieved, with its family.
    tawabir: Vec<TaburMasjjal>,
    /// What has happened, for the diagnostics bundle.
    athar: Vec<String>,
}

/// A swapchain as it was created, plus the images the loader gave back.
///
/// Captured at `vkCreateSwapchainKHR` because there is no query that returns a
/// swapchain's create-info afterwards. Without this the overlay would have to
/// guess the format and the extent, and the format decides whether the shader
/// writes linear or encoded values — which is the difference between text that
/// looks right and text that looks washed out on exactly the games that use an
/// sRGB swapchain.
#[derive(Debug, Clone)]
pub struct WasfSilsila {
    /// The swapchain handle.
    pub silsila: vk::SwapchainKHR,
    /// Its images, in the order `vkGetSwapchainImagesKHR` reported them.
    pub suwar: Vec<vk::Image>,
    /// The format it was created with.
    pub sigha: vk::Format,
    /// The colour space it was created with, kept for the report.
    pub fada_lawn: vk::ColorSpaceKHR,
    /// Its extent in pixels.
    pub imtidad: vk::Extent2D,
    /// What it is allowed to be used for, which decides whether capture is
    /// possible at all — a swapchain created without `TRANSFER_SRC` cannot be
    /// copied out of, and that is a refusal rather than a workaround.
    pub istikhdam: vk::ImageUsageFlags,
}

impl WasfSilsila {
    /// The surface description the overlay renders against.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SighaGhayrMaduma`] for a swapchain format outside the five
    /// this build converts, which is what an HDR10 or a packed-16 surface
    /// produces.
    pub fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        let (sigha, sirgb) = sigha_min_vulkan(self.sigha)?;
        Ok(WasfSath { ard: self.imtidad.width, irtifa: self.imtidad.height, sigha, sirgb })
    }
}

/// Maps a `VkFormat` onto the three facts the capture path needs.
///
/// The list is short on purpose. Every swapchain format a desktop driver
/// actually offers is in it, and a format that is not is refused by name rather
/// than approximated — a capture converted from a format nobody verified is a
/// capture the recognizer reads as noise, and "the overlay reads nothing on this
/// game" is a harder bug to find than "this format is not supported".
///
/// # Errors
///
/// [`KhataTabaqa::SighaGhayrMaduma`] naming the format as Vulkan names it.
pub fn sigha_min_vulkan(sigha: vk::Format) -> Result<(SighatSath, bool), KhataTabaqa> {
    match sigha {
        vk::Format::B8G8R8A8_UNORM => Ok((SighatSath::Bgra8, false)),
        vk::Format::B8G8R8A8_SRGB => Ok((SighatSath::Bgra8, true)),
        vk::Format::R8G8B8A8_UNORM => Ok((SighatSath::Rgba8, false)),
        vk::Format::R8G8B8A8_SRGB => Ok((SighatSath::Rgba8, true)),
        vk::Format::A2B10G10R10_UNORM_PACK32 | vk::Format::A2R10G10B10_UNORM_PACK32 => {
            Ok((SighatSath::Rgb10a2, false))
        }
        vk::Format::R16G16B16A16_SFLOAT => Ok((SighatSath::Rgba16f, false)),
        _ => Err(KhataTabaqa::SighaGhayrMaduma { sigha: format!("VkFormat({})", sigha.as_raw()) }),
    }
}

/// Every instance and device the layer has seen, keyed by loader dispatch key.
///
/// A `Mutex` rather than a lock-free structure, and held for as short a time as
/// the operation allows: the entries are cloned out as `Arc`s and the lock is
/// released before anything touches the GPU, because holding a global lock
/// across a `vkQueueSubmit` would serialise every device in a multi-GPU process
/// behind whichever one is slowest.
static SIJILL: OnceLock<Mutex<Sijill>> = OnceLock::new();

/// The registry's contents.
#[derive(Default)]
struct Sijill {
    /// Instances by loader key.
    mithalat: HashMap<usize, Arc<JadwalMithal>>,
    /// Devices by loader key.
    ajhiza: HashMap<usize, Arc<JadwalJihaz>>,
}

/// The registry, created on first use.
fn sijill() -> &'static Mutex<Sijill> {
    SIJILL.get_or_init(|| Mutex::new(Sijill::default()))
}

/// The device record a dispatchable handle belongs to.
///
/// # Safety
///
/// `maqbad` must be a live dispatchable handle created from a device — a
/// `VkDevice`, a `VkQueue` or a `VkCommandBuffer`.
unsafe fn jihaz_min_maqbad(maqbad: u64) -> Option<Arc<JadwalJihaz>> {
    // SAFETY: the caller guarantees the handle is a live dispatchable object, so
    // its first word is the loader dispatch pointer every object descended from
    // one device shares.
    let miftah = unsafe { miftah(maqbad) };
    sijill().lock().ajhiza.get(&miftah).map(Arc::clone)
}

/// The instance record a dispatchable handle belongs to.
///
/// # Safety
///
/// `maqbad` must be a live dispatchable handle created from an instance — a
/// `VkInstance` or a `VkPhysicalDevice`.
unsafe fn mithal_min_maqbad(maqbad: u64) -> Option<Arc<JadwalMithal>> {
    // SAFETY: as in `jihaz_min_maqbad`, for the instance's dispatch key.
    let miftah = unsafe { miftah(maqbad) };
    sijill().lock().mithalat.get(&miftah).map(Arc::clone)
}

// ---------------------------------------------------------------------------
// Resolving down the chain
// ---------------------------------------------------------------------------

/// Reinterprets a resolved entry point as its own function-pointer type.
///
/// # Safety
///
/// `T` must be the function-pointer type the Vulkan specification gives the
/// command that produced `dalla`.
unsafe fn ka_dalla<T: Copy>(dalla: vk::PFN_vkVoidFunction) -> Option<T> {
    let muashir = ka_muashir(dalla);
    if muashir.is_null() || size_of::<T>() != size_of::<*const c_void>() {
        return None;
    }
    // SAFETY: `T` is pointer-sized by the check above and the caller guarantees
    // it is the command's own signature. `transmute_copy` reads exactly that
    // many bytes out of a live pointer-sized local.
    Some(unsafe { core::mem::transmute_copy::<*const c_void, T>(&muashir) })
}

/// One instance-level command, from the next layer down.
///
/// # Safety
///
/// `gipa` must be a live `vkGetInstanceProcAddr` from the chain, `mithal` a live
/// instance or [`vk::Instance::null`] for a global command, and `T` the
/// command's own function-pointer type.
unsafe fn hall_mithal<T: Copy>(
    gipa: vk::PFN_vkGetInstanceProcAddr,
    mithal: vk::Instance,
    ism: &CStr,
) -> Option<T> {
    // SAFETY: the caller guarantees the resolver and the instance are live, and
    // `ism` is a NUL-terminated name that outlives the call.
    let dalla = unsafe { gipa(mithal, ism.as_ptr()) };
    // SAFETY: the caller guarantees `T` matches the command named by `ism`.
    unsafe { ka_dalla::<T>(dalla) }
}

/// One device-level command, from the next layer down.
///
/// # Safety
///
/// `gdpa` must be a live `vkGetDeviceProcAddr` from the chain, `jihaz` a live
/// device, and `T` the command's own function-pointer type.
unsafe fn hall_jihaz<T: Copy>(
    gdpa: vk::PFN_vkGetDeviceProcAddr,
    jihaz: vk::Device,
    ism: &CStr,
) -> Option<T> {
    // SAFETY: the caller guarantees the resolver and the device are live, and
    // `ism` is a NUL-terminated name that outlives the call.
    let dalla = unsafe { gdpa(jihaz, ism.as_ptr()) };
    // SAFETY: the caller guarantees `T` matches the command named by `ism`.
    unsafe { ka_dalla::<T>(dalla) }
}

// ---------------------------------------------------------------------------
// The present handler the overlay installs
// ---------------------------------------------------------------------------

/// What the present hook tells the overlay, and what it expects back.
///
/// The one field that travels upward is [`SiyaqTaqdeem::isharat_khuruj`]. Left
/// as [`None`] it means "the overlay did not draw this frame", and the present
/// goes down the chain with the application's own wait semaphores untouched.
/// Filled in it means "the overlay submitted work that must finish first", and
/// the present is rewritten to wait on that semaphore instead. The asymmetry is
/// deliberate: the default is the behaviour that is correct when the overlay
/// does nothing, so a handler that returns early cannot deadlock a present.
#[derive(Debug)]
pub struct SiyaqTaqdeem {
    /// The device the swapchain belongs to.
    pub jihaz: vk::Device,
    /// The queue the application is presenting on.
    pub tabur: vk::Queue,
    /// That queue's family, or [`None`] when the application never retrieved
    /// the queue through `vkGetDeviceQueue` and the layer therefore never saw
    /// which family it came from.
    pub usra: Option<u32>,
    /// The swapchain being presented.
    pub silsila: vk::SwapchainKHR,
    /// Which of its images.
    pub fahras_sura: u32,
    /// The semaphores the present would have waited on.
    pub intizar: Vec<vk::Semaphore>,
    /// The semaphore the present must wait on instead, when the overlay drew.
    pub isharat_khuruj: Option<vk::Semaphore>,
}

/// The overlay's present handler.
pub type MunadiTaqdeem = Box<dyn Fn(&mut SiyaqTaqdeem) + Send + Sync + 'static>;

/// The installed handler, if any.
static MUNADI: OnceLock<Mutex<Option<MunadiTaqdeem>>> = OnceLock::new();

/// The handler slot, created on first use.
fn munadi() -> &'static Mutex<Option<MunadiTaqdeem>> {
    MUNADI.get_or_init(|| Mutex::new(None))
}

/// Installs the handler the present hook calls once per presented frame.
///
/// The layer knows the protocol and the overlay knows what to draw, and this is
/// the seam between them. It is a callback rather than the layer owning a
/// [`crate::wajiha::Tabaqa`] because the overlay's lifecycle — the disclosure,
/// the budget, the pause — belongs to the controller, and a layer that owned it
/// would be a layer that had to reimplement all of it to be testable.
pub fn sajjil_munadi(handler: MunadiTaqdeem) {
    *munadi().lock() = Some(handler);
}

/// Removes the handler, so presents pass straight through.
///
/// Called when the overlay disables itself. After this the layer is still in the
/// dispatch chain and still costs one indirect call per present, and it draws
/// nothing — which is the correct posture for an overlay that has faulted,
/// because unloading a layer from inside a live dispatch chain is not something
/// the loader supports.
pub fn ilgh_munadi() {
    *munadi().lock() = None;
}

/// Everything a caller needs to build a [`KhattafVulkan`] for one device.
///
/// Handed out rather than reached for: the renderer takes this by value and
/// never consults the registry, which is what lets it be built against a device
/// created by a test instead of by a game.
#[derive(Clone)]
pub struct BinaVulkan {
    /// The device, loaded against the next layer's resolver.
    pub jihaz: ash::Device,
    /// The physical device, for nothing but the record.
    pub jihaz_madi: vk::PhysicalDevice,
    /// Its memory types, for [`ikhtar_naw_dhakira`].
    pub dhakira: vk::PhysicalDeviceMemoryProperties,
    /// The swapchain to draw into.
    pub silsila: WasfSilsila,
}

impl fmt::Debug for BinaVulkan {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj
            .debug_struct("BinaVulkan")
            .field("jihaz", &self.jihaz.handle())
            .field("jihaz_madi", &self.jihaz_madi)
            .field("silsila", &self.silsila)
            .finish_non_exhaustive()
    }
}

/// The most recently created swapchain on a device, with everything the
/// renderer needs alongside it.
///
/// # Errors
///
/// [`KhataTabaqa::SathTaghayyar`] when the device has no swapchain yet, which is
/// what the window between `vkCreateDevice` and the game's first
/// `vkCreateSwapchainKHR` looks like from here.
pub fn bina(jihaz: vk::Device) -> Result<BinaVulkan, KhataTabaqa> {
    // SAFETY: `jihaz` is a `VkDevice` the caller obtained from this layer's own
    // records or from the application, so it is a live dispatchable handle.
    let Some(jadwal) = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) }) else {
        return Err(KhataTabaqa::SathTaghayyar {
            sabab: "this device was not created through the Taarib layer, so the layer has no \
                    dispatch table for it"
                .to_owned(),
        });
    };
    let silsila = jadwal.hala.lock().salasil.last().cloned().ok_or_else(|| {
        KhataTabaqa::SathTaghayyar {
            sabab: "the device has no swapchain yet; the game has not called \
                    vkCreateSwapchainKHR"
                .to_owned(),
        }
    })?;
    Ok(BinaVulkan {
        jihaz: jadwal.jihaz.clone(),
        jihaz_madi: jadwal.jihaz_madi,
        dhakira: jadwal.dhakira,
        silsila,
    })
}

/// Every device the layer currently has a dispatch table for.
///
/// Ordering is not meaningful — it is a hash map's — which is why the caller
/// that wants "the device the game renders with" asks for the one that has a
/// swapchain rather than the first one here.
#[must_use]
pub fn ajhiza() -> Vec<vk::Device> {
    sijill().lock().ajhiza.values().map(|jadwal| jadwal.jihaz.handle()).collect()
}

/// What the layer has recorded about a device, for the diagnostics bundle.
#[must_use]
pub fn athar_jihaz(jihaz: vk::Device) -> Vec<String> {
    // SAFETY: `jihaz` is a `VkDevice` handle supplied by the caller, which is a
    // live dispatchable object for as long as the caller holds it.
    let Some(jadwal) = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) }) else {
        return Vec::new();
    };
    let hala = jadwal.hala.lock();
    hala.athar.clone()
}

// ---------------------------------------------------------------------------
// The intercepted commands
// ---------------------------------------------------------------------------

/// `vkCreateInstance`, which is where the instance chain is walked.
///
/// Four things happen here and three of them are mandated by the loader rather
/// than chosen: find this layer's link, take the next layer's resolver out of
/// it, **advance the link so the next layer down finds its own**, and only then
/// call down. The advance is the step that is easy to leave out and impossible
/// to leave out twice, because without it every layer below this one resolves
/// its "next" pointer to this layer and the first call recurses until the stack
/// is gone.
extern "system" fn insha_mithal(
    malumat: *const vk::InstanceCreateInfo<'_>,
    mukhassis: *const vk::AllocationCallbacks<'_>,
    mukhraj: *mut vk::Instance,
) -> vk::Result {
    if malumat.is_null() || mukhraj.is_null() {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    // SAFETY: the loader guarantees `malumat` points at a live
    // `VkInstanceCreateInfo` for the duration of the call, and its `pNext` is
    // either null or a well-formed extension chain.
    let talee = unsafe { (*malumat).p_next };
    // SAFETY: as above — the chain came from the loader.
    let Some(uqda) = (unsafe { jid_rabt_mithal(talee) }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };

    // SAFETY: `uqda` is a `VkLayerInstanceCreateInfo` whose `function` field was
    // read as `VK_LAYER_LINK_INFO`, which is precisely the discriminant that
    // makes `pLayerInfo` the live member of the union.
    let rabt = unsafe { (*uqda).ittihad.rabt };
    if rabt.is_null() {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    // SAFETY: the loader allocated this link and keeps it live for the call.
    let (gipa_talee, rabt_baad) = unsafe { ((*rabt).gipa_talee, (*rabt).talee) };
    let Some(gipa_talee) = gipa_talee else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: writing the loader's own link pointer forward is the documented
    // way to advance the chain, and the field is writable for the call.
    unsafe {
        (*uqda).ittihad.rabt = rabt_baad;
    }

    // SAFETY: `gipa_talee` is the next layer's resolver, and `vkCreateInstance`
    // is a global command, which is resolved against a null instance.
    let Some(insha) = (unsafe {
        hall_mithal::<vk::PFN_vkCreateInstance>(
            gipa_talee,
            vk::Instance::null(),
            c"vkCreateInstance",
        )
    }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };

    // SAFETY: every pointer is passed through exactly as the application gave
    // it, to the next layer's own implementation of the same command.
    let natija = unsafe { insha(malumat, mukhassis, mukhraj) };
    if natija != vk::Result::SUCCESS {
        return natija;
    }

    // SAFETY: `vkCreateInstance` succeeded, so it wrote a live instance handle
    // into `mukhraj`.
    let maqbad = unsafe { *mukhraj };
    let hall = |ism: &CStr| {
        // SAFETY: `gipa_talee` is the next layer's resolver and `maqbad` the
        // instance it was taken for; `ism` is a NUL-terminated name `ash` owns
        // for the duration of the callback.
        unsafe { ka_muashir(gipa_talee(maqbad, ism.as_ptr())) }
    };
    // SAFETY: `gipa_talee` resolves against the instance just created, which is
    // what `ash`'s loader needs. The tables it builds for Vulkan 1.1 and 1.3 may
    // contain stubs on a 1.0 instance; this layer calls none of those commands.
    let mithal = unsafe { ash::Instance::load_with(hall, maqbad) };
    // SAFETY: the instance is live, and both names below are commands of that
    // instance whose requested types are their own signatures. The surface
    // capabilities query is absent on an instance created without
    // `VK_KHR_surface`, which is a headless application and is handled by the
    // `Option`.
    let (itlaf, qudurat_sath) = unsafe {
        (
            hall_mithal::<vk::PFN_vkDestroyInstance>(gipa_talee, maqbad, c"vkDestroyInstance"),
            hall_mithal::<vk::PFN_vkGetPhysicalDeviceSurfaceCapabilitiesKHR>(
                gipa_talee,
                maqbad,
                c"vkGetPhysicalDeviceSurfaceCapabilitiesKHR",
            ),
        )
    };

    // SAFETY: `maqbad` is a live dispatchable instance handle.
    let miftah = unsafe { miftah(maqbad.as_raw()) };
    let _ = sijill()
        .lock()
        .mithalat
        .insert(miftah, Arc::new(JadwalMithal { mithal, gipa_talee, itlaf, qudurat_sath }));
    vk::Result::SUCCESS
}

/// `vkDestroyInstance`, which forgets the table before the object goes away.
///
/// The order matters: the entry is removed from the registry *before* the next
/// layer destroys the instance, because after the destroy the handle's dispatch
/// pointer is dangling and the key computed from it would name nothing — or,
/// worse, would collide with a key the loader hands out for the next instance
/// created at the same address.
extern "system" fn itlaf_mithal(
    mithal: vk::Instance,
    mukhassis: *const vk::AllocationCallbacks<'_>,
) {
    // SAFETY: the application guarantees the instance is live at the moment it
    // asks for it to be destroyed.
    let miftah = unsafe { miftah(mithal.as_raw()) };
    let jadwal = sijill().lock().mithalat.remove(&miftah);
    if let Some(jadwal) = jadwal
        && let Some(itlaf) = jadwal.itlaf
    {
        // SAFETY: the instance is still live — nothing below has destroyed it —
        // and `itlaf` is the next layer's own `vkDestroyInstance`.
        unsafe { itlaf(mithal, mukhassis) };
    }
}

/// `vkCreateDevice`, which is where the device chain is walked.
///
/// Same shape as [`insha_mithal`], with one addition: the swapchain commands are
/// resolved here rather than lazily, so that a device created without
/// `VK_KHR_swapchain` is recognised immediately as one the overlay has nothing
/// to do with. A compute-only device in a game that also has a graphics one is
/// common, and a layer that discovered the absence at present time would be a
/// layer that had already allocated for it.
extern "system" fn insha_jihaz(
    jihaz_madi: vk::PhysicalDevice,
    malumat: *const vk::DeviceCreateInfo<'_>,
    mukhassis: *const vk::AllocationCallbacks<'_>,
    mukhraj: *mut vk::Device,
) -> vk::Result {
    if malumat.is_null() || mukhraj.is_null() {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    // SAFETY: the loader guarantees `malumat` is a live `VkDeviceCreateInfo`
    // with a well-formed `pNext` chain for the duration of the call.
    let talee = unsafe { (*malumat).p_next };
    // SAFETY: as above.
    let Some(uqda) = (unsafe { jid_rabt_jihaz(talee) }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: the node's `function` was read as `VK_LAYER_LINK_INFO`, which
    // makes `pLayerInfo` the live union member.
    let rabt = unsafe { (*uqda).ittihad.rabt };
    if rabt.is_null() {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    // SAFETY: the loader allocated this link and keeps it live for the call.
    let (gipa_talee, gdpa_talee, rabt_baad) =
        unsafe { ((*rabt).gipa_talee, (*rabt).gdpa_talee, (*rabt).talee) };
    let (Some(gipa_talee), Some(gdpa_talee)) = (gipa_talee, gdpa_talee) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: advancing the loader's link is the documented protocol.
    unsafe {
        (*uqda).ittihad.rabt = rabt_baad;
    }

    // SAFETY: a physical device is dispatchable and shares its instance's
    // loader key, which is how the instance record is found from it.
    let sahib = unsafe { mithal_min_maqbad(jihaz_madi.as_raw()) };
    let maqbad_mithal = sahib.as_ref().map_or(vk::Instance::null(), |jadwal| {
        jadwal.mithal.handle()
    });

    // SAFETY: `vkCreateDevice` is an instance-level command, resolved against
    // the instance this physical device came from.
    let Some(insha) = (unsafe {
        hall_mithal::<vk::PFN_vkCreateDevice>(gipa_talee, maqbad_mithal, c"vkCreateDevice")
    }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: every argument is the application's own, passed through unchanged.
    let natija = unsafe { insha(jihaz_madi, malumat, mukhassis, mukhraj) };
    if natija != vk::Result::SUCCESS {
        return natija;
    }
    // SAFETY: the create succeeded, so a live device handle was written.
    let maqbad = unsafe { *mukhraj };

    let Some(sahib) = sahib else {
        // The device was created from a physical device whose instance this
        // layer never saw, which means the layer was inserted into the device
        // chain and not the instance chain. The device is left entirely alone
        // rather than half-tracked: an overlay that cannot read memory
        // properties cannot allocate, and a record that could not be used would
        // only make a later lookup succeed misleadingly.
        return vk::Result::SUCCESS;
    };

    let hall = |ism: &CStr| {
        // SAFETY: `gdpa_talee` is the next layer's resolver and `maqbad` the
        // device it was taken for; `ism` is a NUL-terminated name `ash` owns for
        // the duration of the callback.
        unsafe { ka_muashir(gdpa_talee(maqbad, ism.as_ptr())) }
    };
    // SAFETY: `gdpa_talee` resolves against the device just created.
    let jihaz = unsafe { ash::Device::load_with(hall, maqbad) };
    // SAFETY: the physical device is live and belongs to this instance.
    let dhakira = unsafe { sahib.mithal.get_physical_device_memory_properties(jihaz_madi) };

    // SAFETY: every name below is a command of the device just created, and each
    // requested type is that command's own signature from the specification.
    let (itlaf, insha_silsila, itlaf_silsila, suwar_silsila, taqdeem, jib_tabur) = unsafe {
        (
            hall_jihaz::<vk::PFN_vkDestroyDevice>(gdpa_talee, maqbad, c"vkDestroyDevice"),
            hall_jihaz::<vk::PFN_vkCreateSwapchainKHR>(
                gdpa_talee,
                maqbad,
                c"vkCreateSwapchainKHR",
            ),
            hall_jihaz::<vk::PFN_vkDestroySwapchainKHR>(
                gdpa_talee,
                maqbad,
                c"vkDestroySwapchainKHR",
            ),
            hall_jihaz::<vk::PFN_vkGetSwapchainImagesKHR>(
                gdpa_talee,
                maqbad,
                c"vkGetSwapchainImagesKHR",
            ),
            hall_jihaz::<vk::PFN_vkQueuePresentKHR>(gdpa_talee, maqbad, c"vkQueuePresentKHR"),
            hall_jihaz::<vk::PFN_vkGetDeviceQueue>(gdpa_talee, maqbad, c"vkGetDeviceQueue"),
        )
    };

    let mut hala = HalatJihaz::default();
    hala.athar.push(format!(
        "device created with {} memory types; swapchain support: {}",
        dhakira.memory_type_count,
        if insha_silsila.is_some() { "yes" } else { "no, this device is left alone" }
    ));

    // SAFETY: `maqbad` is a live dispatchable device handle.
    let miftah = unsafe { miftah(maqbad.as_raw()) };
    let _ = sijill().lock().ajhiza.insert(
        miftah,
        Arc::new(JadwalJihaz {
            jihaz,
            sahib,
            jihaz_madi,
            dhakira,
            gdpa_talee,
            itlaf,
            insha_silsila,
            itlaf_silsila,
            suwar_silsila,
            taqdeem,
            jib_tabur,
            hala: Mutex::new(hala),
        }),
    );
    vk::Result::SUCCESS
}

/// `vkDestroyDevice`, which forgets the table before the object goes away.
extern "system" fn itlaf_jihaz(jihaz: vk::Device, mukhassis: *const vk::AllocationCallbacks<'_>) {
    // SAFETY: the application guarantees the device is live at the moment it
    // asks for it to be destroyed.
    let miftah = unsafe { miftah(jihaz.as_raw()) };
    let jadwal = sijill().lock().ajhiza.remove(&miftah);
    if let Some(jadwal) = jadwal
        && let Some(itlaf) = jadwal.itlaf
    {
        // SAFETY: the device is still live and `itlaf` is the next layer's own
        // `vkDestroyDevice`.
        unsafe { itlaf(jihaz, mukhassis) };
    }
}

/// `vkGetDeviceQueue`, watched so a queue can be traced back to its family.
///
/// Nothing is changed. The call goes down the chain and the answer is recorded,
/// because `vkQueuePresentKHR` gives the layer a queue handle and a command pool
/// has to be created for that queue's *family*. There is no Vulkan query that
/// goes backwards from a queue to a family index, so watching the retrieval is
/// the only way to know — and a layer that guessed "family zero" would build a
/// command pool that fails validation on every game whose present queue is not
/// the first one.
extern "system" fn jib_tabur(
    jihaz: vk::Device,
    usra: u32,
    fahras: u32,
    mukhraj: *mut vk::Queue,
) {
    // Both early returns leave the application without its queue, and there is
    // no better answer available: reaching either means this layer is in the
    // dispatch chain for a device it never saw created, so it holds no pointer
    // to the next layer's implementation and has nothing to call down to. The
    // loader does not put a layer in a device's chain without routing that
    // device's `vkCreateDevice` through it, so neither is reachable in a
    // correctly assembled chain.
    //
    // SAFETY: the application guarantees the device handle is live.
    let Some(jadwal) = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) }) else {
        return;
    };
    let Some(jib) = jadwal.jib_tabur else {
        return;
    };
    // SAFETY: every argument is the application's own and `jib` is the next
    // layer's `vkGetDeviceQueue`.
    unsafe { jib(jihaz, usra, fahras, mukhraj) };
    if mukhraj.is_null() {
        return;
    }
    // SAFETY: the call above succeeded and wrote a queue handle, because
    // `vkGetDeviceQueue` cannot fail for a family and index the device was
    // created with.
    let tabur = unsafe { *mukhraj };
    if tabur.is_null() {
        return;
    }
    let mut hala = jadwal.hala.lock();
    if !hala.tawabir.iter().any(|masjjal| masjjal.tabur == tabur) {
        hala.tawabir.push(TaburMasjjal { tabur, usra });
    }
}

/// `vkCreateSwapchainKHR`, where the surface's real description is captured.
///
/// Two things are done and only one of them is passive.
///
/// The passive one is the record: format, colour space, extent, usage and the
/// image list. There is no Vulkan query that returns a swapchain's create-info
/// after the fact, so a layer that did not capture it here would have to guess
/// the format — and the format is what decides whether the fragment shader
/// writes linear or sRGB-encoded values.
///
/// The active one is `TRANSFER_SRC_BIT`. Reading a region of the presented frame
/// means copying out of a swapchain image, and an image can only be a transfer
/// source if the swapchain was created with that usage — which a game has no
/// reason to ask for. So the layer asks for it *on the game's behalf*, and it
/// asks only after `vkGetPhysicalDeviceSurfaceCapabilitiesKHR` says the surface
/// supports it. Requesting an unsupported usage would fail the creation
/// outright, and the obvious repair — retry without it — is invalid, because a
/// failed `vkCreateSwapchainKHR` still retires `oldSwapchain` and the retry
/// would pass a retired handle. Asking first is the only version of this that is
/// correct on a resize.
///
/// When the surface refuses, the record says so and [`Khattaf::iltaqit`] refuses
/// in turn with a sentence naming the reason. The overlay still draws; it simply
/// cannot read.
extern "system" fn insha_silsila(
    jihaz: vk::Device,
    malumat: *const vk::SwapchainCreateInfoKHR<'_>,
    mukhassis: *const vk::AllocationCallbacks<'_>,
    mukhraj: *mut vk::SwapchainKHR,
) -> vk::Result {
    // SAFETY: the application guarantees the device handle is live.
    let Some(jadwal) = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    let Some(insha) = jadwal.insha_silsila else {
        return vk::Result::ERROR_EXTENSION_NOT_PRESENT;
    };
    if malumat.is_null() || mukhraj.is_null() {
        // SAFETY: the arguments are passed through exactly as given; a null
        // create-info is the application's error to receive back from the
        // driver, not this layer's to substitute for.
        return unsafe { insha(jihaz, malumat, mukhassis, mukhraj) };
    }

    // SAFETY: the loader guarantees a live `VkSwapchainCreateInfoKHR` for the
    // duration of the call, and the structure is `Copy`, so this is a shallow
    // copy that keeps every pointer — `pNext`, the queue family list — pointing
    // at the application's own memory, which outlives the call.
    let mut nuskha = unsafe { *malumat };
    let matlub = nuskha.image_usage | vk::ImageUsageFlags::TRANSFER_SRC;
    let mut madum = None;
    if !nuskha.image_usage.contains(vk::ImageUsageFlags::TRANSFER_SRC) {
        if let Some(qudurat) = jadwal.sahib.qudurat_sath {
            let mut qudra = vk::SurfaceCapabilitiesKHR::default();
            // SAFETY: the physical device and the surface are both live — the
            // surface is the one the application is about to create a swapchain
            // for — and `qudra` is one live, aligned structure to write into.
            let natija =
                unsafe { qudurat(jadwal.jihaz_madi, nuskha.surface, &raw mut qudra) };
            if natija == vk::Result::SUCCESS
                && qudra.supported_usage_flags.contains(vk::ImageUsageFlags::TRANSFER_SRC)
            {
                nuskha.image_usage = matlub;
            } else {
                madum = Some(format!(
                    "the surface does not support TRANSFER_SRC ({natija}), so the overlay can \
                     draw on this swapchain but cannot read from it"
                ));
            }
        } else {
            madum = Some(
                "the instance has no VK_KHR_surface entry point, so TRANSFER_SRC support could \
                 not be checked and was not requested"
                    .to_owned(),
            );
        }
    }

    let qadeema = nuskha.old_swapchain;
    // SAFETY: `nuskha` is a live create-info for the call, differing from the
    // application's only in `imageUsage`, and every other argument is passed
    // through unchanged.
    let natija = unsafe { insha(jihaz, &raw const nuskha, mukhassis, mukhraj) };

    let mut hala = jadwal.hala.lock();
    if !qadeema.is_null() {
        // The old swapchain is retired whether this call succeeded or not, so
        // its record goes either way. Keeping it would leave the overlay
        // drawing into images that the presentation engine has let go of.
        hala.salasil.retain(|wasf| wasf.silsila != qadeema);
    }
    if natija != vk::Result::SUCCESS {
        hala.athar.push(format!("vkCreateSwapchainKHR failed: {natija}"));
        return natija;
    }

    // SAFETY: the create succeeded, so a live swapchain handle was written.
    let silsila = unsafe { *mukhraj };
    let suwar = jadwal.suwar_silsila.map_or_else(Vec::new, |jib| {
        // SAFETY: the device and the swapchain are both live, and the two-call
        // idiom below hands the driver a count it wrote itself alongside a
        // buffer of exactly that many elements.
        unsafe { jid_suwar(jib, jihaz, silsila) }
    });

    if let Some(madum) = madum {
        hala.athar.push(madum);
    }
    hala.athar.push(format!(
        "swapchain {}×{} format {} with {} image(s)",
        nuskha.image_extent.width,
        nuskha.image_extent.height,
        nuskha.image_format.as_raw(),
        suwar.len()
    ));
    hala.salasil.push(WasfSilsila {
        silsila,
        suwar,
        sigha: nuskha.image_format,
        fada_lawn: nuskha.image_color_space,
        imtidad: nuskha.image_extent,
        istikhdam: nuskha.image_usage,
    });
    natija
}

/// The swapchain's images, through the two-call idiom.
///
/// # Safety
///
/// `jib` must be the chain's `vkGetSwapchainImagesKHR`, and `jihaz` and
/// `silsila` must be live.
unsafe fn jid_suwar(
    jib: vk::PFN_vkGetSwapchainImagesKHR,
    jihaz: vk::Device,
    silsila: vk::SwapchainKHR,
) -> Vec<vk::Image> {
    let mut adad: u32 = 0;
    // SAFETY: the caller guarantees the handles are live; a null image pointer
    // with a live count pointer is the documented way to ask for the count.
    let natija = unsafe { jib(jihaz, silsila, &raw mut adad, core::ptr::null_mut()) };
    if natija != vk::Result::SUCCESS || adad == 0 {
        return Vec::new();
    }
    let Ok(saa) = usize::try_from(adad) else {
        return Vec::new();
    };
    let mut suwar = vec![vk::Image::null(); saa];
    // SAFETY: `suwar` has exactly `adad` elements and `adad` is the count the
    // driver just reported, so the write stays inside the allocation.
    let natija = unsafe { jib(jihaz, silsila, &raw mut adad, suwar.as_mut_ptr()) };
    if natija != vk::Result::SUCCESS {
        return Vec::new();
    }
    // A driver is permitted to report fewer images on the second call than on
    // the first. Truncating rather than trusting the original count is what
    // keeps the framebuffer array from containing a null image.
    suwar.truncate(usize::try_from(adad).unwrap_or(0));
    suwar
}

/// `vkDestroySwapchainKHR`, which forgets the record before the object goes.
extern "system" fn itlaf_silsila(
    jihaz: vk::Device,
    silsila: vk::SwapchainKHR,
    mukhassis: *const vk::AllocationCallbacks<'_>,
) {
    // SAFETY: the application guarantees the device handle is live.
    let Some(jadwal) = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) }) else {
        return;
    };
    jadwal.hala.lock().salasil.retain(|wasf| wasf.silsila != silsila);
    if let Some(itlaf) = jadwal.itlaf_silsila {
        // SAFETY: the swapchain is still live — nothing below has destroyed it —
        // and `itlaf` is the next layer's own `vkDestroySwapchainKHR`.
        unsafe { itlaf(jihaz, silsila, mukhassis) };
    }
}

/// `vkQueuePresentKHR`, where the overlay draws and the semaphores are rewired.
///
/// The sequence, in order, with the reason each step is where it is:
///
/// 1. **Find the device from the queue.** A queue shares its device's loader
///    dispatch pointer, and that is the only link between the two that this call
///    provides.
/// 2. **Find a swapchain the layer knows.** A present may carry several — a game
///    driving two monitors does — and the overlay draws on the first one it has
///    a record for, passing the rest through. Drawing on all of them would put
///    the same subtitles on both screens at coordinates measured for one.
/// 3. **Call the handler**, giving it the image index and the application's own
///    wait semaphores. It draws, or it does not.
/// 4. **Rewire, or do not.** If the handler returned a semaphore, the present
///    info is *copied* — the application's is `const` — and its wait list is
///    replaced by that one semaphore. If it did not, the original pointer goes
///    down the chain untouched, which is what makes a skipped overlay frame cost
///    nothing and deadlock nothing.
extern "system" fn taqdeem(tabur: vk::Queue, malumat: *const vk::PresentInfoKHR<'_>) -> vk::Result {
    // SAFETY: the application guarantees the queue handle is live, and a queue
    // is a dispatchable object sharing its device's loader key.
    let Some(jadwal) = (unsafe { jihaz_min_maqbad(tabur.as_raw()) }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    let Some(taqdeem_talee) = jadwal.taqdeem else {
        return vk::Result::ERROR_EXTENSION_NOT_PRESENT;
    };
    if malumat.is_null() {
        // SAFETY: passed through exactly as given.
        return unsafe { taqdeem_talee(tabur, malumat) };
    }

    let Some(ishara) = ishtaghil_taqdeem(&jadwal, tabur, malumat) else {
        // SAFETY: the overlay drew nothing, so the application's own present
        // info goes down the chain byte for byte.
        return unsafe { taqdeem_talee(tabur, malumat) };
    };

    // SAFETY: `malumat` is a live `VkPresentInfoKHR` and the structure is
    // `Copy`, so this is a shallow copy whose `pNext`, `pSwapchains`,
    // `pImageIndices` and `pResults` still point at the application's memory,
    // which outlives the call.
    let mut nuskha = unsafe { *malumat };
    let intizar = [ishara];
    nuskha.wait_semaphore_count = 1;
    nuskha.p_wait_semaphores = intizar.as_ptr();
    // SAFETY: `nuskha` is live for the call and `intizar` outlives it; the
    // semaphore in it is the one the overlay's submission signals, so the
    // present now waits on the overlay rather than beside it.
    unsafe { taqdeem_talee(tabur, &raw const nuskha) }
}

/// Runs the overlay for one present, returning the semaphore to wait on.
///
/// Split out of [`taqdeem`] so that the raw-pointer reads of the present info
/// happen in one place with one set of invariants, and so the borrow of the
/// handler is released before the next layer is called — a handler that took a
/// long time would otherwise hold a global lock across another layer's present.
fn ishtaghil_taqdeem(
    jadwal: &JadwalJihaz,
    tabur: vk::Queue,
    malumat: *const vk::PresentInfoKHR<'_>,
) -> Option<vk::Semaphore> {
    // SAFETY: the application guarantees `malumat` is a live `VkPresentInfoKHR`
    // for the duration of the call.
    let (adad, salasil, faharis, adad_intizar, muashir_intizar) = unsafe {
        (
            (*malumat).swapchain_count,
            (*malumat).p_swapchains,
            (*malumat).p_image_indices,
            (*malumat).wait_semaphore_count,
            (*malumat).p_wait_semaphores,
        )
    };
    let saa = usize::try_from(adad).ok()?;
    if saa == 0 || salasil.is_null() || faharis.is_null() {
        return None;
    }
    // SAFETY: `swapchainCount` is documented to be the length of both arrays,
    // and the application allocated them; the slices borrow them only for this
    // function, which returns before the present goes down the chain.
    let (salasil, faharis) = unsafe {
        (
            core::slice::from_raw_parts(salasil, saa),
            core::slice::from_raw_parts(faharis, saa),
        )
    };

    let marufa = {
        let hala = jadwal.hala.lock();
        salasil
            .iter()
            .zip(faharis.iter())
            .find(|(silsila, _)| hala.salasil.iter().any(|wasf| wasf.silsila == **silsila))
            .map(|(silsila, fahras)| (*silsila, *fahras))
    };
    let (silsila, fahras_sura) = marufa?;

    let intizar = if adad_intizar == 0 || muashir_intizar.is_null() {
        Vec::new()
    } else {
        let saa_intizar = usize::try_from(adad_intizar).ok()?;
        // SAFETY: `waitSemaphoreCount` is the length of `pWaitSemaphores`, which
        // the application allocated and keeps live across the present.
        unsafe { core::slice::from_raw_parts(muashir_intizar, saa_intizar) }.to_vec()
    };

    let usra = {
        let hala = jadwal.hala.lock();
        hala.tawabir.iter().find(|masjjal| masjjal.tabur == tabur).map(|masjjal| masjjal.usra)
    };

    let mut siyaq = SiyaqTaqdeem {
        jihaz: jadwal.jihaz.handle(),
        tabur,
        usra,
        silsila,
        fahras_sura,
        intizar,
        isharat_khuruj: None,
    };

    let dalil = munadi().lock();
    let handler = dalil.as_ref()?;
    handler(&mut siyaq);
    siyaq.isharat_khuruj
}

// ---------------------------------------------------------------------------
// The exported entry points
// ---------------------------------------------------------------------------

/// One of this layer's own commands, as the loader's untyped function pointer.
///
/// The loader asks for a command by name and casts what it gets back to that
/// command's signature, which is what makes `PFN_vkVoidFunction` sound: the
/// round trip is name-to-pointer-to-name, and the name is the type.
fn ka_amma(muashir: *const c_void) -> vk::PFN_vkVoidFunction {
    if muashir.is_null() {
        return None;
    }
    // SAFETY: every address passed here is the address of one of this module's
    // own `extern "system"` functions, taken by coercing the function item to
    // the exact `PFN_*` type the Vulkan specification gives that command. It is
    // a live, non-null code address, and the loader casts it back to that same
    // signature before calling it.
    Some(unsafe { core::mem::transmute::<*const c_void, unsafe extern "system" fn()>(muashir) })
}

/// A function item as the untyped pointer [`ka_amma`] wants.
///
/// The binding and the cast are separate steps on purpose: the binding coerces
/// this module's safe `extern "system"` item to the specification's own `PFN_*`
/// type, which is where a signature mismatch is caught at compile time, and the
/// cast is the pointer cast that erases it.
macro_rules! amma {
    ($dalla:expr, $sigha:ty) => {{
        let dalla: $sigha = $dalla;
        ka_amma((dalla as *const ()).cast::<c_void>())
    }};
}

/// The instance-level commands this layer implements.
///
/// `vkCreateInstance` and `vkGetInstanceProcAddr` must be answerable with a null
/// instance, because the loader asks for them before any instance exists — that
/// is how the chain is bootstrapped.
fn dalla_mithal(ism: &CStr) -> Option<vk::PFN_vkVoidFunction> {
    let dalla = match ism.to_bytes() {
        b"vkGetInstanceProcAddr" => {
            amma!(vkGetInstanceProcAddr, vk::PFN_vkGetInstanceProcAddr)
        }
        b"vkCreateInstance" => amma!(insha_mithal, vk::PFN_vkCreateInstance),
        b"vkDestroyInstance" => amma!(itlaf_mithal, vk::PFN_vkDestroyInstance),
        b"vkCreateDevice" => amma!(insha_jihaz, vk::PFN_vkCreateDevice),
        b"vkGetDeviceProcAddr" => amma!(vkGetDeviceProcAddr, vk::PFN_vkGetDeviceProcAddr),
        _ => return None,
    };
    Some(dalla)
}

/// The device-level commands this layer implements.
///
/// Answered from `vkGetInstanceProcAddr` as well as from
/// `vkGetDeviceProcAddr`, because the loader is permitted to resolve device
/// commands through either and a layer that answered only one of them would be
/// bypassed by an application that used the other.
fn dalla_jihaz(ism: &CStr) -> Option<vk::PFN_vkVoidFunction> {
    let dalla = match ism.to_bytes() {
        b"vkGetDeviceProcAddr" => amma!(vkGetDeviceProcAddr, vk::PFN_vkGetDeviceProcAddr),
        b"vkDestroyDevice" => amma!(itlaf_jihaz, vk::PFN_vkDestroyDevice),
        b"vkGetDeviceQueue" => amma!(jib_tabur, vk::PFN_vkGetDeviceQueue),
        b"vkCreateSwapchainKHR" => amma!(insha_silsila, vk::PFN_vkCreateSwapchainKHR),
        b"vkDestroySwapchainKHR" => amma!(itlaf_silsila, vk::PFN_vkDestroySwapchainKHR),
        b"vkQueuePresentKHR" => amma!(taqdeem, vk::PFN_vkQueuePresentKHR),
        _ => return None,
    };
    Some(dalla)
}

/// The body both exported instance resolvers share.
fn hall_amr_mithal(mithal: vk::Instance, ism: *const c_char) -> vk::PFN_vkVoidFunction {
    if ism.is_null() {
        return None;
    }
    // SAFETY: the loader guarantees `ism` is a NUL-terminated ASCII command name
    // that outlives the call.
    let ism = unsafe { CStr::from_ptr(ism) };
    if let Some(dalla) = dalla_mithal(ism) {
        return dalla;
    }
    if let Some(dalla) = dalla_jihaz(ism) {
        return dalla;
    }
    if mithal.is_null() {
        // A command this layer does not implement, asked for before there is an
        // instance. There is no chain to forward down yet, and returning null is
        // the documented answer.
        return None;
    }
    // SAFETY: the loader guarantees a live instance handle.
    let jadwal = (unsafe { mithal_min_maqbad(mithal.as_raw()) })?;
    // SAFETY: the instance is live and `gipa_talee` is the next layer's own
    // resolver, taken out of the link the loader supplied at creation.
    unsafe { (jadwal.gipa_talee)(mithal, ism.as_ptr()) }
}

/// The body both exported device resolvers share.
fn hall_amr_jihaz(jihaz: vk::Device, ism: *const c_char) -> vk::PFN_vkVoidFunction {
    if ism.is_null() {
        return None;
    }
    // SAFETY: the loader guarantees `ism` is a NUL-terminated ASCII command name
    // that outlives the call.
    let ism = unsafe { CStr::from_ptr(ism) };
    if let Some(dalla) = dalla_jihaz(ism) {
        return dalla;
    }
    if jihaz.is_null() {
        return None;
    }
    // SAFETY: the loader guarantees a live device handle.
    let jadwal = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) })?;
    // SAFETY: the device is live and `gdpa_talee` is the next layer's own
    // resolver, taken out of the link the loader supplied at creation.
    unsafe { (jadwal.gdpa_talee)(jihaz, ism.as_ptr()) }
}

/// `vkGetInstanceProcAddr`, exported under the name the loader expects.
///
/// # Panics
///
/// Never. Nothing in the path this function takes allocates fallibly, indexes,
/// divides or unwraps, which is deliberate: this runs on a game's thread with
/// the loader's stack frame beneath it, and unwinding out of an `extern
/// "system"` function into C is undefined behaviour rather than a crash somebody
/// can read.
#[unsafe(no_mangle)]
pub extern "system" fn vkGetInstanceProcAddr(
    mithal: vk::Instance,
    ism: *const c_char,
) -> vk::PFN_vkVoidFunction {
    hall_amr_mithal(mithal, ism)
}

/// `vkGetDeviceProcAddr`, exported under the name the loader expects.
///
/// # Panics
///
/// Never, for the reason given on [`vkGetInstanceProcAddr`].
#[unsafe(no_mangle)]
pub extern "system" fn vkGetDeviceProcAddr(
    jihaz: vk::Device,
    ism: *const c_char,
) -> vk::PFN_vkVoidFunction {
    hall_amr_jihaz(jihaz, ism)
}

/// The uniquely-named instance resolver the manifest's `functions` block names.
///
/// Exported alongside the standard name rather than instead of it. Two layers
/// statically linked into one library cannot both export `vkGetInstanceProcAddr`
/// — the second one silently loses — so interface version 2 lets a layer publish
/// a name that cannot collide and declare it in the manifest. Keeping the
/// standard name too means the layer still works under a loader that predates
/// the negotiation call.
#[unsafe(no_mangle)]
pub extern "system" fn taaribTabaqaGetInstanceProcAddr(
    mithal: vk::Instance,
    ism: *const c_char,
) -> vk::PFN_vkVoidFunction {
    hall_amr_mithal(mithal, ism)
}

/// The uniquely-named device resolver the manifest's `functions` block names.
#[unsafe(no_mangle)]
pub extern "system" fn taaribTabaqaGetDeviceProcAddr(
    jihaz: vk::Device,
    ism: *const c_char,
) -> vk::PFN_vkVoidFunction {
    hall_amr_jihaz(jihaz, ism)
}

/// The interface-version handshake, and where this layer states what it is.
///
/// The loader hands over a `VkNegotiateLayerInterface` carrying the highest
/// version it supports; the layer writes back the version it implements, which
/// must not be higher, along with its three resolvers. Declining — returning
/// anything but `VK_SUCCESS` — is how a layer says "not on this loader", and it
/// is the correct answer for a loader older than version 2, because this layer's
/// manifest names uniquely-exported functions that such a loader will not look
/// for.
///
/// `bunya` is typed as a raw pointer rather than as the structure, because the
/// structure is the *loader's* declaration and not Vulkan's: no binding crate
/// generates it, and exporting Taarib's private transcription of it in a public
/// signature would publish a type nobody outside this file can construct.
///
/// # Panics
///
/// Never. See [`vkGetInstanceProcAddr`].
#[unsafe(no_mangle)]
pub extern "system" fn vkNegotiateLoaderLayerInterfaceVersion(
    bunya: *mut c_void,
) -> vk::Result {
    tafawud(bunya)
}

/// The uniquely-named negotiation entry point the manifest names.
#[unsafe(no_mangle)]
pub extern "system" fn taaribTabaqaNegotiateLoaderLayerInterfaceVersion(
    bunya: *mut c_void,
) -> vk::Result {
    tafawud(bunya)
}

/// The negotiation both exported entry points share.
fn tafawud(bunya: *mut c_void) -> vk::Result {
    if bunya.is_null() {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    let bunya = bunya.cast::<WajihatTafawud>();
    // SAFETY: the loader guarantees `bunya` points at a live
    // `VkNegotiateLayerInterface` it allocated and filled, whose layout is the
    // one `WajihatTafawud` transcribes, and which stays live for the call.
    let (naw, isdar_muhammil) = unsafe { ((*bunya).naw, (*bunya).isdar) };
    if naw != NAW_TAFAWUD {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    if isdar_muhammil < ISDAR_WAJIHAT_TABAQA {
        // The loader is older than the protocol this layer's manifest is written
        // against. Declining is honest: the alternative is being loaded by a
        // loader that will not look for the uniquely-named exports the manifest
        // declares, and would therefore find no entry points at all.
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    // SAFETY: the same live structure, written through the fields the protocol
    // reserves for the layer's answer.
    unsafe {
        (*bunya).isdar = ISDAR_WAJIHAT_TABAQA;
        (*bunya).gipa = Some(vkGetInstanceProcAddr);
        (*bunya).gdpa = Some(vkGetDeviceProcAddr);
        (*bunya).gpdpa = core::ptr::null_mut();
    }
    vk::Result::SUCCESS
}

// ---------------------------------------------------------------------------
// The shaders, as the source the SPIR-V is built from
// ---------------------------------------------------------------------------

/// Compiles one GLSL stage to SPIR-V words.
///
/// Vulkan is the only API in this crate with no compiler available to it at
/// runtime. Direct3D resolves `D3DCompile` out of `d3dcompiler_47.dll`, OpenGL
/// hands GLSL to the driver, and a Vulkan device takes SPIR-V and nothing in the
/// loader will produce it. The three ways out were hand-assembling SPIR-V words,
/// which nobody can review; requiring a build step with a C++ shader toolchain,
/// which this workspace otherwise has no reason to carry; or compiling in Rust.
/// This is the third.
///
/// Run once per overlay, at construction, on whatever thread built it — never
/// on the presentation path. A shader compile inside `vkQueuePresentKHR` would
/// be several milliseconds in the middle of somebody's frame.
///
/// SPIR-V 1.0 is targeted deliberately rather than the newest the toolchain can
/// emit: it is what a Vulkan 1.0 device accepts, and the overlay's two stages
/// use nothing a later version added. Emitting 1.5 would refuse to load on
/// hardware this overlay is exactly the fallback for.
///
/// # Errors
///
/// [`KhataTabaqa::MawridFashil`] carrying the compiler's own message. A failure
/// here is a defect in the constants immediately below rather than anything
/// about the user's machine, so the message is passed through unedited — a
/// summarised parser error is a bug report nobody can act on.
fn ila_spirv(masdar: &str, marhala: naga::ShaderStage) -> Result<Vec<u32>, KhataTabaqa> {
    use naga::back::spv;
    use naga::front::glsl;
    use naga::valid::{Capabilities, ValidationFlags, Validator};

    let khata = |marhalat: &str, sabab: String| KhataTabaqa::MawridFashil {
        mawrid: "shader module",
        sabab: format!("{marhalat} the overlay's {marhala:?} stage: {sabab}"),
    };

    let mut wajiha = glsl::Frontend::default();
    let khiyarat = glsl::Options::from(marhala);
    let wahda = wajiha
        .parse(&khiyarat, masdar)
        .map_err(|sabab| khata("parsing", format!("{sabab:?}")))?;

    // Validated before emission rather than trusting the frontend. A module that
    // parses and is not valid produces SPIR-V a driver rejects at
    // `vkCreateShaderModule`, which surfaces as a resource failure with no
    // indication that a shader was the cause.
    let malumat = Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&wahda)
        .map_err(|sabab| khata("validating", format!("{sabab:?}")))?;

    let mut khiyarat_spv = spv::Options {
        lang_version: (1, 0),
        flags: spv::WriterFlags::empty(),
        ..spv::Options::default()
    };
    // The debug flag is what puts the GLSL identifiers into the module. Left off
    // because those names ship inside a library that loads into other people's
    // processes, and a smaller module is one fewer thing for an anti-tamper
    // scanner to have an opinion about.
    khiyarat_spv.flags.remove(spv::WriterFlags::DEBUG);

    let nuqtat = spv::PipelineOptions { shader_stage: marhala, entry_point: "main".to_owned() };
    spv::write_vec(&wahda, &malumat, &khiyarat_spv, Some(&nuqtat))
        .map_err(|sabab| khata("emitting SPIR-V for", format!("{sabab:?}")))
}

/// The vertex stage's GLSL, from which [`BinaVulkan::shifrat_raas`] is compiled.
///
/// Kept here rather than in the asset directory alone so that the pipeline's
/// vertex input description and the shader's `layout(location = …)` list are
/// four lines apart in one file. They have to agree exactly — a mismatch is a
/// pipeline that links, draws, and puts the colour where the position should be
/// — and there is no build step that checks them against each other.
pub const MASDAR_RAAS: &str = "#version 450 core\n\
layout(location = 0) in vec2 mawdi;\n\
layout(location = 1) in vec2 khareeta;\n\
layout(location = 2) in vec4 lawn;\n\
layout(location = 3) in float nasij;\n\
layout(location = 0) out vec2 khareeta_marra;\n\
layout(location = 1) out vec4 lawn_marra;\n\
layout(location = 2) out float nasij_marra;\n\
layout(push_constant) uniform Thawabit {\n\
    vec2 qiyas;\n\
    int tarmiz_sirgb;\n\
    int hashw;\n\
} thawabit;\n\
void main() {\n\
    khareeta_marra = khareeta;\n\
    lawn_marra = lawn;\n\
    nasij_marra = nasij;\n\
    gl_Position = vec4(mawdi * thawabit.qiyas - vec2(1.0), 0.0, 1.0);\n\
}\n";

/// The fragment stage's GLSL, from which [`BinaVulkan::shifrat_qita`] is built.
///
/// Same two ideas as the OpenGL backend's fragment stage, for the same reasons:
/// the atlas is sampled or not according to a per-vertex flag so that plates and
/// glyphs go out in one draw call, and the linear-to-sRGB transfer is applied
/// here only when the swapchain format is *not* one of the `_SRGB` ones. A
/// Vulkan `_SRGB` swapchain format makes the hardware encode on write, and a
/// shader that encoded as well would produce text that is visibly washed out
/// against a dark background — the same value taken through the curve twice.
pub const MASDAR_QITA: &str = "#version 450 core\n\
layout(location = 0) in vec2 khareeta_marra;\n\
layout(location = 1) in vec4 lawn_marra;\n\
layout(location = 2) in float nasij_marra;\n\
layout(location = 0) out vec4 natija;\n\
layout(set = 0, binding = 0) uniform sampler2D lawha;\n\
layout(push_constant) uniform Thawabit {\n\
    vec2 qiyas;\n\
    int tarmiz_sirgb;\n\
    int hashw;\n\
} thawabit;\n\
vec3 ila_sirgb(vec3 khatti) {\n\
    vec3 amin = max(khatti, vec3(0.0));\n\
    vec3 munkhafid = amin * 12.92;\n\
    vec3 murtafi = 1.055 * pow(amin, vec3(1.0 / 2.4)) - 0.055;\n\
    return mix(murtafi, munkhafid, step(amin, vec3(0.0031308)));\n\
}\n\
void main() {\n\
    vec4 asas = mix(vec4(1.0), texture(lawha, khareeta_marra), nasij_marra);\n\
    vec4 mazuj = asas * lawn_marra;\n\
    if (thawabit.tarmiz_sirgb != 0 && mazuj.a > 0.0) {\n\
        vec3 mufakkak = mazuj.rgb / mazuj.a;\n\
        mazuj = vec4(ila_sirgb(mufakkak) * mazuj.a, mazuj.a);\n\
    }\n\
    natija = mazuj;\n\
}\n";

// ---------------------------------------------------------------------------
// Sizes and shapes
// ---------------------------------------------------------------------------

/// The largest atlas this build will upload, in bytes.
const AQSA_LAWHA: u64 = 128 * 1024 * 1024;

/// The most quads one batch may carry.
const AQSA_QITAAT: usize = 16_384;

/// The largest read-back this build will stage, in bytes.
///
/// Sixty-four mebibytes is a 4096×4096 region at RGBA8 — larger than any screen
/// the recognizer is asked to read and far larger than the dialogue-box regions
/// it actually gets. The ceiling exists because the staging buffer is device
/// memory allocated inside somebody's game, and a region whose size came out of
/// a corrupted config file must not become a request for a gigabyte.
const AQSA_ILTIQAT: u64 = 64 * 1024 * 1024;

/// How long a fence wait may block the render thread, in nanoseconds.
///
/// One second. A fence that has not been signalled after a second means the GPU
/// has stopped — a hang, a device loss, a driver that is about to be reset — and
/// waiting forever would turn that into a frozen game with Taarib's name on the
/// task manager entry. Timing out lets the overlay report and stand down while
/// the game is still the thing that failed.
const MUHLAT_QAYD: u64 = 1_000_000_000;

/// One vertex, laid out exactly as the pipeline's attribute descriptions say.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
#[allow(
    dead_code,
    reason = "no Rust code reads these fields and none ever will: they are written by the \
              batch builder and read by the GPU, through the offsets the vertex attribute \
              descriptions carry. A field the compiler calls unread is one the vertex shader \
              is consuming"
)]
struct RaasQita {
    /// Position in surface pixels, origin top-left.
    mawdi: [f32; 2],
    /// Atlas coordinate, normalized.
    khareeta: [f32; 2],
    /// Premultiplied linear RGBA.
    lawn: [f32; 4],
    /// One when this quad samples the atlas, zero when it is a solid plate.
    nasij: f32,
}

/// The push-constant block, byte for byte as both shaders declare it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
struct ThawabitRasm {
    /// Two over the surface width and two over its height.
    qiyas: [f32; 2],
    /// One when the shader must apply the sRGB transfer itself.
    tarmiz_sirgb: i32,
    /// Padding, so the block is sixteen bytes on every implementation.
    ///
    /// Vulkan requires a push-constant range's size to be a multiple of four and
    /// permits an implementation to guarantee only 128 bytes total; sixteen with
    /// an explicit tail is what makes the Rust struct, the GLSL block and the
    /// range size agree without anyone having to reason about std430 padding.
    hashw: i32,
}

/// What one present gave the renderer, and what it has done with it since.
#[derive(Debug, Clone)]
struct SiyaqItar {
    /// The queue the frame is being presented on.
    tabur: vk::Queue,
    /// That queue's family, when the layer saw it retrieved.
    usra: Option<u32>,
    /// Which swapchain image.
    fahras_sura: u32,
    /// What the next submission must wait on.
    ///
    /// Starts as the application's own present-wait semaphores and is replaced
    /// by this renderer's own signal semaphore after each submission. That is
    /// what lets a capture *and* a draw both happen in one frame: each consumes
    /// the pending wait and republishes its own, so the chain stays a chain
    /// instead of two submissions racing for the same binary semaphore.
    intizar: Vec<vk::Semaphore>,
    /// Whether anything has been submitted for this frame.
    sallam: bool,
}

/// Picks a memory type satisfying both the requirement mask and the properties.
///
/// There is no correct hardcoded index and there never was. A discrete GPU
/// reports device-local types first and host-visible ones later; an integrated
/// one reports types that are both; a machine with a resizable BAR reports a
/// host-visible device-local type that neither of the others has. An overlay
/// that assumed index zero or index one would allocate in the wrong heap on two
/// of those three and fail outright on the third.
///
/// Returns the first index that is set in `matlub` — the bitmask
/// `vkGetImageMemoryRequirements` or `vkGetBufferMemoryRequirements` returned —
/// **and** carries every flag in `khasais`. First rather than best, because the
/// specification orders memory types so that earlier indices are the ones an
/// implementation prefers for the properties they advertise.
#[must_use]
pub fn ikhtar_naw_dhakira(
    dhakira: &vk::PhysicalDeviceMemoryProperties,
    matlub: u32,
    khasais: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let adad = usize::try_from(dhakira.memory_type_count).unwrap_or(0);
    dhakira
        .memory_types
        .iter()
        .take(adad)
        .enumerate()
        .filter_map(|(fahras, naw)| u32::try_from(fahras).ok().map(|fahras| (fahras, naw)))
        .find(|(fahras, naw)| {
            1u32.checked_shl(*fahras).is_some_and(|bit| matlub & bit != 0)
                && naw.property_flags.contains(khasais)
        })
        .map(|(fahras, _)| fahras)
}

/// A failed Vulkan call as the error the trait returns.
fn khata_mawrid(mawrid: &'static str, natija: vk::Result) -> KhataTabaqa {
    KhataTabaqa::MawridFashil { mawrid, sabab: format!("{natija}") }
}

/// A pixel count as a float, for the projection and the vertices.
const fn qeema_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and glyph coordinates are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

// ---------------------------------------------------------------------------
// Host-visible buffers
// ---------------------------------------------------------------------------

/// A buffer in `HOST_VISIBLE | HOST_COHERENT` memory, mapped for its whole life.
///
/// Mapped once and never unmapped. `vkMapMemory` is not free — it takes a driver
/// lock and on some implementations a page-table walk — and doing it twice a
/// frame on a render thread for a buffer that is written every frame is cost for
/// nothing. `HOST_COHERENT` is what makes it correct without a flush: the write
/// is visible to the device without `vkFlushMappedMemoryRanges`, which is the
/// call an overlay forgets and then spends a week finding.
struct MahfazaMazjura {
    /// The buffer.
    mahfaza: vk::Buffer,
    /// Its memory.
    dhakira: vk::DeviceMemory,
    /// How many bytes it holds.
    saa: u64,
    /// The mapped address, as an integer.
    ///
    /// Held as a `usize` rather than a `*mut u8` for one concrete reason:
    /// [`Khattaf`] requires `Send`, a raw pointer field would take it away, and
    /// an `unsafe impl Send` would be asserting something about every other
    /// field of the renderer at the same time in order to say something about
    /// this one. The address is turned back into a pointer at each use with
    /// `with_exposed_provenance_mut`, which is what the provenance API exists
    /// for. Zero means unmapped.
    unwan: usize,
}

impl MahfazaMazjura {
    /// An allocation that does not exist yet.
    const fn khaliya() -> Self {
        Self { mahfaza: vk::Buffer::null(), dhakira: vk::DeviceMemory::null(), saa: 0, unwan: 0 }
    }

    /// Whether anything has been allocated.
    const fn mawjuda(&self) -> bool {
        self.saa != 0
    }

    /// Copies bytes into the mapping.
    ///
    /// Returns `false` when the buffer is smaller than the write, which the
    /// caller answers by growing it rather than by writing a prefix — a partial
    /// vertex buffer draws garbage triangles rather than fewer glyphs.
    fn iktub(&self, bayt: &[u8]) -> bool {
        if self.unwan == 0 || u64::try_from(bayt.len()).unwrap_or(u64::MAX) > self.saa {
            return false;
        }
        let hadaf = core::ptr::with_exposed_provenance_mut::<u8>(self.unwan);
        // SAFETY: `unwan` is the address `vkMapMemory` returned for this
        // allocation and the allocation is still mapped, so the whole of it is
        // writable; the length check above keeps the copy inside it. Source and
        // destination cannot overlap, because the destination is device memory
        // the host has never handed to Rust as a reference.
        unsafe { core::ptr::copy_nonoverlapping(bayt.as_ptr(), hadaf, bayt.len()) };
        true
    }

    /// Reads bytes out of the mapping.
    fn iqra(&self, tul: usize) -> Vec<u8> {
        if self.unwan == 0 || u64::try_from(tul).unwrap_or(u64::MAX) > self.saa {
            return Vec::new();
        }
        let masdar = core::ptr::with_exposed_provenance::<u8>(self.unwan);
        let mut khuruj = vec![0u8; tul];
        // SAFETY: as in `iktub`, in the other direction — `tul` bytes were
        // checked to be inside the mapping, and `khuruj` is exactly that many
        // writable bytes in a fresh allocation that cannot overlap it.
        unsafe { core::ptr::copy_nonoverlapping(masdar, khuruj.as_mut_ptr(), tul) };
        khuruj
    }
}

// ---------------------------------------------------------------------------
// The renderer
// ---------------------------------------------------------------------------

/// The Vulkan overlay renderer.
///
/// Owns a render pass that loads rather than clears, one framebuffer per
/// swapchain image, a pipeline with premultiplied-alpha blending, a descriptor
/// set for the glyph atlas, streaming vertex and index buffers in host-coherent
/// memory, and — per swapchain image — a command buffer, a fence and two
/// semaphores. The per-image duplication is not caution: a command buffer cannot
/// be re-recorded while the GPU is still executing it, and with two or three
/// images in flight the frame that is being recorded is not the frame that is
/// executing.
pub struct KhattafVulkan {
    /// The device every handle below belongs to.
    jihaz: ash::Device,
    /// The physical device, kept for the diagnostics line.
    jihaz_madi: vk::PhysicalDevice,
    /// Its memory types.
    dhakira: vk::PhysicalDeviceMemoryProperties,
    /// The swapchain being drawn into, as it was created.
    silsila: WasfSilsila,
    /// The surface derived from it.
    sath: WasfSath,

    /// The vertex stage's SPIR-V.
    shifrat_raas: Vec<u32>,
    /// The fragment stage's SPIR-V.
    shifrat_qita: Vec<u32>,

    /// The render pass, `LOAD_OP_LOAD` on a `PRESENT_SRC_KHR` attachment.
    mamarr: vk::RenderPass,
    /// The descriptor set layout for the atlas sampler.
    wahdat_wasf: vk::DescriptorSetLayout,
    /// The pool the one descriptor set comes from.
    hawd_wasf: vk::DescriptorPool,
    /// The descriptor set the pipeline binds.
    majmuat_wasf: vk::DescriptorSet,
    /// The pipeline layout, carrying the push-constant range.
    takhtit: vk::PipelineLayout,
    /// The graphics pipeline.
    khatt: vk::Pipeline,
    /// The atlas sampler.
    akhidh: vk::Sampler,

    /// One view per swapchain image.
    manazir: Vec<vk::ImageView>,
    /// One framebuffer per swapchain image.
    itarat: Vec<vk::Framebuffer>,
    /// The command pool, created for the presenting queue's family.
    hawd_awamir: vk::CommandPool,
    /// Which family that pool was created for.
    usrat_awamir: Option<u32>,
    /// One draw command buffer per swapchain image.
    awamir_rasm: Vec<vk::CommandBuffer>,
    /// One capture command buffer per swapchain image.
    awamir_iltiqat: Vec<vk::CommandBuffer>,
    /// One draw fence per swapchain image.
    quyud_rasm: Vec<vk::Fence>,
    /// One capture fence per swapchain image.
    quyud_iltiqat: Vec<vk::Fence>,
    /// One draw semaphore per swapchain image.
    isharat_rasm: Vec<vk::Semaphore>,
    /// One capture semaphore per swapchain image.
    isharat_iltiqat: Vec<vk::Semaphore>,

    /// The streaming vertex buffer.
    ruus_gpu: MahfazaMazjura,
    /// The streaming index buffer.
    faharis_gpu: MahfazaMazjura,
    /// The read-back staging buffer.
    iltiqat_gpu: MahfazaMazjura,

    /// The glyph atlas image.
    lawha: vk::Image,
    /// Its memory.
    dhakirat_lawha: vk::DeviceMemory,
    /// Its view.
    manzar_lawha: vk::ImageView,
    /// Its dimensions.
    qiyas_lawha: (u32, u32),
    /// The staging buffer the atlas was uploaded through.
    raf_lawha: MahfazaMazjura,

    /// The present this frame, once the hook has declared it.
    siyaq: Option<SiyaqItar>,

    /// The vertex staging buffer, reused across frames.
    ruus: Vec<RaasQita>,
    /// The index staging buffer, reused across frames.
    faharis: Vec<u32>,
    /// What has happened here, for the diagnostics bundle.
    athar: Vec<String>,
}

impl fmt::Debug for KhattafVulkan {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj
            .debug_struct("KhattafVulkan")
            .field("jihaz", &self.jihaz.handle())
            .field("jihaz_madi", &self.jihaz_madi)
            .field("silsila", &self.silsila.silsila)
            .field("sath", &self.sath)
            .field("suwar", &self.silsila.suwar.len())
            .field("khatt", &self.khatt)
            .field("qiyas_lawha", &self.qiyas_lawha)
            .field("usrat_awamir", &self.usrat_awamir)
            .finish_non_exhaustive()
    }
}

impl KhattafVulkan {
    /// Prepares a renderer over one device and one swapchain.
    ///
    /// Nothing is created here — [`Khattaf::hayyi`] does that, on the thread the
    /// game presents from. What this does is refuse early: a swapchain whose
    /// format has no entry in [`sigha_min_vulkan`] is a swapchain this build
    /// cannot capture from and cannot blend correctly into, and finding that out
    /// at construction is better than finding it out at the first frame.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SighaGhayrMaduma`] for an unmapped swapchain format, and
    /// [`KhataTabaqa::MawridFashil`] when the swapchain reported no images at
    /// all, which is what a driver that failed `vkGetSwapchainImagesKHR` leaves
    /// behind.
    pub fn jadeed(bina: BinaVulkan) -> Result<Self, KhataTabaqa> {
        let sath = bina.silsila.sath()?;
        if bina.silsila.suwar.is_empty() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "swapchain images",
                sabab: "the swapchain reported no images, so there is nothing to draw into"
                    .to_owned(),
            });
        }
        // Compiled here, at construction, rather than taken as a parameter.
        // Direct3D resolves `D3DCompile` out of the OS and OpenGL compiles in
        // the driver; Vulkan has neither, and a backend that could not produce
        // its own shaders would be a backend that cannot draw without somebody
        // else's build step. The cost is paid once per overlay, not per frame.
        let shifrat_raas = ila_spirv(MASDAR_RAAS, naga::ShaderStage::Vertex)?;
        let shifrat_qita = ila_spirv(MASDAR_QITA, naga::ShaderStage::Fragment)?;

        let athar = vec![format!(
            "swapchain {}×{} {} ({} image(s)), capture {}",
            sath.ard,
            sath.irtifa,
            sath.sigha.ism(),
            bina.silsila.suwar.len(),
            if bina.silsila.istikhdam.contains(vk::ImageUsageFlags::TRANSFER_SRC) {
                "available"
            } else {
                "unavailable: the swapchain has no TRANSFER_SRC usage"
            }
        )];

        Ok(Self {
            jihaz: bina.jihaz,
            jihaz_madi: bina.jihaz_madi,
            dhakira: bina.dhakira,
            silsila: bina.silsila,
            sath,
            shifrat_raas,
            shifrat_qita,
            mamarr: vk::RenderPass::null(),
            wahdat_wasf: vk::DescriptorSetLayout::null(),
            hawd_wasf: vk::DescriptorPool::null(),
            majmuat_wasf: vk::DescriptorSet::null(),
            takhtit: vk::PipelineLayout::null(),
            khatt: vk::Pipeline::null(),
            akhidh: vk::Sampler::null(),
            manazir: Vec::new(),
            itarat: Vec::new(),
            hawd_awamir: vk::CommandPool::null(),
            usrat_awamir: None,
            awamir_rasm: Vec::new(),
            awamir_iltiqat: Vec::new(),
            quyud_rasm: Vec::new(),
            quyud_iltiqat: Vec::new(),
            isharat_rasm: Vec::new(),
            isharat_iltiqat: Vec::new(),
            ruus_gpu: MahfazaMazjura::khaliya(),
            faharis_gpu: MahfazaMazjura::khaliya(),
            iltiqat_gpu: MahfazaMazjura::khaliya(),
            lawha: vk::Image::null(),
            dhakirat_lawha: vk::DeviceMemory::null(),
            manzar_lawha: vk::ImageView::null(),
            qiyas_lawha: (0, 0),
            raf_lawha: MahfazaMazjura::khaliya(),
            siyaq: None,
            ruus: Vec::new(),
            faharis: Vec::new(),
            athar,
        })
    }

    /// What has happened to this renderer, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Declares the present this frame belongs to.
    ///
    /// Called by the layer's present handler before [`Khattaf::irsim`] and
    /// before [`Khattaf::iltaqit`], because both need three facts that only the
    /// present carries: which queue, which swapchain image, and which semaphores
    /// the application's rendering signals. Without it the renderer has a
    /// pipeline and no idea where to point it, and both methods refuse rather
    /// than guessing an image index.
    pub fn ibda_taqdeem(
        &mut self,
        tabur: vk::Queue,
        usra: Option<u32>,
        fahras_sura: u32,
        intizar: Vec<vk::Semaphore>,
    ) {
        self.siyaq = Some(SiyaqItar { tabur, usra, fahras_sura, intizar, sallam: false });
    }

    /// The semaphore the present must wait on, or [`None`] if nothing was done.
    ///
    /// [`None`] is the answer that means "present exactly as the application
    /// asked". It is returned whenever no submission happened this frame — the
    /// batch was empty, the budget was spent, the user paused the overlay — and
    /// returning a semaphore in that case would be handing the presentation
    /// engine something nothing will ever signal.
    #[must_use]
    pub fn isharat_taqdeem(&self) -> Option<vk::Semaphore> {
        let siyaq = self.siyaq.as_ref()?;
        if siyaq.sallam { siyaq.intizar.first().copied() } else { None }
    }

    /// Ends the frame, dropping the present context.
    ///
    /// Called by the handler after it has read [`Self::isharat_taqdeem`]. The
    /// context is deliberately not left behind for the next frame: a stale image
    /// index is how an overlay draws into the image the presentation engine is
    /// currently scanning out.
    pub fn anh_taqdeem(&mut self) {
        self.siyaq = None;
    }
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

impl KhattafVulkan {
    /// Creates a host-visible, host-coherent buffer and maps it for good.
    fn ansha_mahfaza(
        &self,
        saa: u64,
        istikhdam: vk::BufferUsageFlags,
        mawrid: &'static str,
    ) -> Result<MahfazaMazjura, KhataTabaqa> {
        if saa == 0 {
            return Ok(MahfazaMazjura::khaliya());
        }
        let malumat = vk::BufferCreateInfo::default()
            .size(saa)
            .usage(istikhdam)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        // SAFETY: `malumat` is a live create-info for the call and describes a
        // buffer with no queue-family list, which is what `EXCLUSIVE` requires.
        let mahfaza = unsafe { self.jihaz.create_buffer(&malumat, None) }
            .map_err(|natija| khata_mawrid(mawrid, natija))?;

        // SAFETY: the buffer was just created on this device and has not been
        // destroyed.
        let mutatallabat = unsafe { self.jihaz.get_buffer_memory_requirements(mahfaza) };
        let khasais =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let Some(naw) =
            ikhtar_naw_dhakira(&self.dhakira, mutatallabat.memory_type_bits, khasais)
        else {
            // SAFETY: the buffer is live and unbound, so destroying it here
            // releases everything this function allocated.
            unsafe { self.jihaz.destroy_buffer(mahfaza, None) };
            return Err(KhataTabaqa::MawridFashil {
                mawrid,
                sabab: format!(
                    "no memory type is both HOST_VISIBLE and HOST_COHERENT among the {} this \
                     device reports and the {:#x} the buffer accepts",
                    self.dhakira.memory_type_count, mutatallabat.memory_type_bits
                ),
            });
        };

        let hajz = vk::MemoryAllocateInfo::default()
            .allocation_size(mutatallabat.size)
            .memory_type_index(naw);
        // SAFETY: `hajz` is live for the call and names a memory type index the
        // search above proved is in range and accepted by this buffer.
        let dhakira = match unsafe { self.jihaz.allocate_memory(&hajz, None) } {
            Ok(dhakira) => dhakira,
            Err(natija) => {
                // SAFETY: the buffer is live and unbound.
                unsafe { self.jihaz.destroy_buffer(mahfaza, None) };
                return Err(khata_mawrid(mawrid, natija));
            }
        };

        // SAFETY: both handles are live, the allocation is at least
        // `mutatallabat.size` bytes and of an accepted type, and neither has
        // been bound before.
        if let Err(natija) = unsafe { self.jihaz.bind_buffer_memory(mahfaza, dhakira, 0) } {
            // SAFETY: both handles are live and the bind failed, so neither is
            // in use by anything.
            unsafe {
                self.jihaz.destroy_buffer(mahfaza, None);
                self.jihaz.free_memory(dhakira, None);
            }
            return Err(khata_mawrid(mawrid, natija));
        }

        // SAFETY: the allocation is live, host-visible, and not already mapped.
        let muashir = match unsafe {
            self.jihaz.map_memory(dhakira, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        } {
            Ok(muashir) => muashir,
            Err(natija) => {
                // SAFETY: both handles are live and nothing is mapped.
                unsafe {
                    self.jihaz.destroy_buffer(mahfaza, None);
                    self.jihaz.free_memory(dhakira, None);
                }
                return Err(khata_mawrid(mawrid, natija));
            }
        };

        Ok(MahfazaMazjura {
            mahfaza,
            dhakira,
            saa: mutatallabat.size,
            unwan: muashir.expose_provenance(),
        })
    }

    /// Unmaps, destroys and forgets a buffer, if it exists.
    ///
    /// # Safety
    ///
    /// The device must be idle with respect to this buffer — no submission that
    /// references it may still be executing.
    unsafe fn atlif_mahfaza(&self, mahfaza: &mut MahfazaMazjura) {
        if !mahfaza.mawjuda() {
            *mahfaza = MahfazaMazjura::khaliya();
            return;
        }
        // SAFETY: the caller guarantees nothing in flight references this
        // buffer. The allocation was mapped by `ansha_mahfaza` and has not been
        // unmapped, and both handles were created on this device.
        unsafe {
            if mahfaza.unwan != 0 {
                self.jihaz.unmap_memory(mahfaza.dhakira);
            }
            self.jihaz.destroy_buffer(mahfaza.mahfaza, None);
            self.jihaz.free_memory(mahfaza.dhakira, None);
        }
        *mahfaza = MahfazaMazjura::khaliya();
    }

    /// Grows a streaming buffer when the batch outgrew it.
    ///
    /// # Safety
    ///
    /// As [`Self::atlif_mahfaza`]: the buffer being replaced must not be
    /// referenced by anything still executing.
    unsafe fn wassi_mahfaza(
        &self,
        mahfaza: &mut MahfazaMazjura,
        matlub: u64,
        istikhdam: vk::BufferUsageFlags,
        mawrid: &'static str,
    ) -> Result<(), KhataTabaqa> {
        if mahfaza.saa >= matlub && mahfaza.mawjuda() {
            return Ok(());
        }
        // Grown to one and a half times what is needed, so a batch that creeps
        // upward by a glyph a frame reallocates a handful of times rather than
        // once per frame — and every reallocation here is a device allocation
        // inside somebody's game.
        #[expect(
            clippy::integer_division,
            reason = "a growth factor of one and a half, where truncating the halving is the \
                      intended rounding and cannot underflow a u64"
        )]
        let jadeed = matlub.saturating_add(matlub / 2).max(4096);
        // SAFETY: the caller guarantees the old buffer is not in use.
        unsafe { self.atlif_mahfaza(mahfaza) };
        *mahfaza = self.ansha_mahfaza(jadeed, istikhdam, mawrid)?;
        Ok(())
    }

    /// Creates the command pool for a queue family, replacing any older one.
    ///
    /// The family is not known until the first present: `vkQueuePresentKHR`
    /// hands over a queue, and the family it belongs to is whatever the layer
    /// saw when the application retrieved it. A command pool created for the
    /// wrong family produces command buffers that fail validation on submit, so
    /// the pool is created here — late, once, and again if the game ever
    /// presents on a queue from a different family, which a game that moves
    /// presentation to a dedicated present queue mid-session does.
    fn tahaqqaq_hawd(&mut self, usra: u32) -> Result<(), KhataTabaqa> {
        if self.usrat_awamir == Some(usra) && !self.hawd_awamir.is_null() {
            return Ok(());
        }
        if !self.hawd_awamir.is_null() {
            // SAFETY: replacing a live pool means every command buffer from it
            // is about to be freed, so nothing referencing them may still be
            // executing. `vkDeviceWaitIdle` is the only guarantee available at
            // this point that says so.
            let _ = unsafe { self.jihaz.device_wait_idle() };
            // SAFETY: the device is idle and the pool was created on it;
            // destroying a pool frees every command buffer allocated from it.
            unsafe { self.jihaz.destroy_command_pool(self.hawd_awamir, None) };
            self.hawd_awamir = vk::CommandPool::null();
            self.awamir_rasm.clear();
            self.awamir_iltiqat.clear();
        }

        let malumat = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(usra);
        // SAFETY: `malumat` is live for the call and names a family index the
        // application itself created a queue on.
        let hawd = unsafe { self.jihaz.create_command_pool(&malumat, None) }
            .map_err(|natija| khata_mawrid("command pool", natija))?;
        self.hawd_awamir = hawd;
        self.usrat_awamir = Some(usra);

        let adad = self.silsila.suwar.len();
        let Ok(adad_gl) = u32::try_from(adad) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "command buffers",
                sabab: format!("{adad} swapchain images is more than a u32 can count"),
            });
        };
        let malumat = vk::CommandBufferAllocateInfo::default()
            .command_pool(hawd)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(adad_gl);
        // SAFETY: the pool is live and `malumat` is live for the call.
        self.awamir_rasm = unsafe { self.jihaz.allocate_command_buffers(&malumat) }
            .map_err(|natija| khata_mawrid("draw command buffers", natija))?;
        // SAFETY: as above; a second allocation from the same live pool.
        self.awamir_iltiqat = unsafe { self.jihaz.allocate_command_buffers(&malumat) }
            .map_err(|natija| khata_mawrid("capture command buffers", natija))?;
        self.athar.push(format!(
            "command pool created for queue family {usra} with {adad} draw and {adad} capture \
             command buffer(s)"
        ));
        Ok(())
    }

    /// Creates the render pass that draws over the finished frame.
    ///
    /// Every field of the attachment description is load-bearing:
    ///
    /// * `load_op = LOAD` — the frame the game just rendered is *in* this image,
    ///   and `CLEAR` would replace it with a colour. This one word is the
    ///   difference between an overlay and a blank screen.
    /// * `store_op = STORE` — what the overlay draws has to survive the pass.
    /// * `initial_layout = final_layout = PRESENT_SRC_KHR` — the image arrives
    ///   ready to present, because the game already transitioned it, and it must
    ///   leave that way, because the presentation engine is next. A render pass
    ///   that left it in `COLOR_ATTACHMENT_OPTIMAL` would hand the presentation
    ///   engine an image in a layout it cannot read.
    /// * `samples = TYPE_1` — a swapchain image is never multisampled; the
    ///   game's own multisampling was resolved before this point.
    fn ansha_mamarr(&mut self) -> Result<(), KhataTabaqa> {
        let murfaqat = [vk::AttachmentDescription::default()
            .format(self.silsila.sigha)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
        let isharat = [vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
        let marahil = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&isharat)];
        // Two dependencies rather than one. The first orders the game's own
        // colour writes before the overlay's; the second orders the overlay's
        // before whatever comes after the pass, which is the presentation
        // engine's read. Declaring only the first is the mistake that produces
        // an overlay that is present in a screenshot and absent on screen.
        let taabuiyat = [
            vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(
                    vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                ),
            vk::SubpassDependency::default()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::empty()),
        ];
        let malumat = vk::RenderPassCreateInfo::default()
            .attachments(&murfaqat)
            .subpasses(&marahil)
            .dependencies(&taabuiyat);
        // SAFETY: every slice referenced by `malumat` is a live local that
        // outlives the call, which is what the borrow on `RenderPassCreateInfo`
        // encodes and the driver relies on.
        self.mamarr = unsafe { self.jihaz.create_render_pass(&malumat, None) }
            .map_err(|natija| khata_mawrid("render pass", natija))?;
        Ok(())
    }

    /// Creates the descriptor set layout, its pool, its one set, and the sampler.
    ///
    /// One set, one binding, one texture. The overlay samples exactly one image
    /// — Taarib's own glyph atlas — and a descriptor pool sized for anything
    /// more would be reserving device memory inside a stranger's game for
    /// descriptors that will never be allocated.
    fn ansha_wasf(&mut self) -> Result<(), KhataTabaqa> {
        let irtibatat = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
        let malumat = vk::DescriptorSetLayoutCreateInfo::default().bindings(&irtibatat);
        // SAFETY: `irtibatat` outlives the call, which is the borrow the
        // create-info carries.
        self.wahdat_wasf = unsafe { self.jihaz.create_descriptor_set_layout(&malumat, None) }
            .map_err(|natija| khata_mawrid("descriptor set layout", natija))?;

        let ahjam = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)];
        let malumat =
            vk::DescriptorPoolCreateInfo::default().pool_sizes(&ahjam).max_sets(1);
        // SAFETY: `ahjam` outlives the call.
        self.hawd_wasf = unsafe { self.jihaz.create_descriptor_pool(&malumat, None) }
            .map_err(|natija| khata_mawrid("descriptor pool", natija))?;

        let wahadat = [self.wahdat_wasf];
        let malumat = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.hawd_wasf)
            .set_layouts(&wahadat);
        // SAFETY: the pool and the layout are both live and `wahadat` outlives
        // the call; the pool was created with room for exactly this one set.
        let majmuat = unsafe { self.jihaz.allocate_descriptor_sets(&malumat) }
            .map_err(|natija| khata_mawrid("descriptor set", natija))?;
        self.majmuat_wasf = majmuat.into_iter().next().ok_or_else(|| {
            KhataTabaqa::MawridFashil {
                mawrid: "descriptor set",
                sabab: "vkAllocateDescriptorSets returned no sets for a request of one".to_owned(),
            }
        })?;

        // Linear filtering and clamp to edge, for the same two reasons the
        // OpenGL backend gives: the atlas is sampled at close to one texel per
        // pixel so a mip chain would be a third more memory that is never
        // selected, and `REPEAT` at a cell's edge samples a texel from the far
        // side of the atlas and draws a sliver of an unrelated letter.
        let malumat = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0.0)
            .max_lod(0.0)
            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK);
        // SAFETY: `malumat` is a live create-info for the call and enables no
        // feature — anisotropy, comparison, unnormalized coordinates — that
        // would need a device feature to have been requested.
        self.akhidh = unsafe { self.jihaz.create_sampler(&malumat, None) }
            .map_err(|natija| khata_mawrid("atlas sampler", natija))?;
        Ok(())
    }

    /// Creates the pipeline layout and the graphics pipeline.
    ///
    /// Viewport and scissor are dynamic state. That is not a habit: the
    /// alternative bakes the surface size into the pipeline, and a pipeline
    /// rebuilt on every resolution change means two SPIR-V modules compiled
    /// again during a fullscreen transition, on the render thread, in the middle
    /// of the stall the transition already is.
    fn ansha_khatt(&mut self) -> Result<(), KhataTabaqa> {
        let malumat = vk::ShaderModuleCreateInfo::default().code(&self.shifrat_raas);
        // SAFETY: the SPIR-V slice outlives the call. Its contents are the
        // caller's responsibility — a driver handed malformed SPIR-V is
        // undefined behaviour that no layer can defend against, which is why
        // `BinaVulkan` documents that these words come from the workspace's own
        // shader build and not from a file the game supplied.
        let wahdat_raas = unsafe { self.jihaz.create_shader_module(&malumat, None) }
            .map_err(|natija| khata_mawrid("vertex shader module", natija))?;
        let malumat = vk::ShaderModuleCreateInfo::default().code(&self.shifrat_qita);
        // SAFETY: as above, for the fragment stage.
        let wahdat_qita = match unsafe { self.jihaz.create_shader_module(&malumat, None) } {
            Ok(wahda) => wahda,
            Err(natija) => {
                // SAFETY: the vertex module is live and is not referenced by any
                // pipeline, because the pipeline has not been created.
                unsafe { self.jihaz.destroy_shader_module(wahdat_raas, None) };
                return Err(khata_mawrid("fragment shader module", natija));
            }
        };

        let natija = self.ansha_khatt_bi_wahdat(wahdat_raas, wahdat_qita);
        // A shader module is only needed while the pipeline is being created —
        // the pipeline keeps whatever it needs from it — so both are destroyed
        // here on success as well as on failure. Keeping them would be keeping
        // two SPIR-V blobs alive in a game's device memory for the rest of the
        // session for nothing.
        // SAFETY: both modules are live, and either the pipeline was created
        // from them (in which case it no longer references them) or it was not.
        unsafe {
            self.jihaz.destroy_shader_module(wahdat_raas, None);
            self.jihaz.destroy_shader_module(wahdat_qita, None);
        }
        natija
    }

    /// The pipeline itself, once both shader modules exist.
    fn ansha_khatt_bi_wahdat(
        &mut self,
        wahdat_raas: vk::ShaderModule,
        wahdat_qita: vk::ShaderModule,
    ) -> Result<(), KhataTabaqa> {
        let Ok(hajm_thawabit) = u32::try_from(size_of::<ThawabitRasm>()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "pipeline layout",
                sabab: "the push-constant block does not fit a u32".to_owned(),
            });
        };
        let mudayat = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(hajm_thawabit)];
        let wahadat = [self.wahdat_wasf];
        let malumat = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&wahadat)
            .push_constant_ranges(&mudayat);
        // SAFETY: both slices outlive the call and the layout handle in
        // `wahadat` is live.
        self.takhtit = unsafe { self.jihaz.create_pipeline_layout(&malumat, None) }
            .map_err(|natija| khata_mawrid("pipeline layout", natija))?;

        let Ok(khatwa) = u32::try_from(size_of::<RaasQita>()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "vertex input",
                sabab: "the vertex stride does not fit a u32".to_owned(),
            });
        };
        let irtibatat = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(khatwa)
            .input_rate(vk::VertexInputRate::VERTEX)];
        // The offsets come from `offset_of!` rather than from literals, so that
        // adding a field to `RaasQita` cannot leave this description talking
        // about the previous layout while still compiling.
        let sifat = [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(izaha_u32(core::mem::offset_of!(RaasQita, mawdi))),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(izaha_u32(core::mem::offset_of!(RaasQita, khareeta))),
            vk::VertexInputAttributeDescription::default()
                .location(2)
                .binding(0)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(izaha_u32(core::mem::offset_of!(RaasQita, lawn))),
            vk::VertexInputAttributeDescription::default()
                .location(3)
                .binding(0)
                .format(vk::Format::R32_SFLOAT)
                .offset(izaha_u32(core::mem::offset_of!(RaasQita, nasij))),
        ];
        let dukhul = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&irtibatat)
            .vertex_attribute_descriptions(&sifat);
        let tajmee = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let manazir = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let tamtheel = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let ainat = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        // Premultiplied alpha: the source is added whole and the destination is
        // scaled by one minus the source's alpha. The colours arriving from
        // `saff` are already premultiplied, which is what makes this the correct
        // pair rather than `SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA` — using those on
        // premultiplied colour darkens every anti-aliased glyph edge.
        let murfaqat = [vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let mazj = vk::PipelineColorBlendStateCreateInfo::default().attachments(&murfaqat);
        let halat = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let mutaghayyira = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&halat);

        let marahil = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(wahdat_raas)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(wahdat_qita)
                .name(c"main"),
        ];
        let malumat = [vk::GraphicsPipelineCreateInfo::default()
            .stages(&marahil)
            .vertex_input_state(&dukhul)
            .input_assembly_state(&tajmee)
            .viewport_state(&manazir)
            .rasterization_state(&tamtheel)
            .multisample_state(&ainat)
            .color_blend_state(&mazj)
            .dynamic_state(&mutaghayyira)
            .layout(self.takhtit)
            .render_pass(self.mamarr)
            .subpass(0)];
        // SAFETY: every structure and slice `malumat` points at is a live local
        // that outlives the call, and the layout, render pass and both shader
        // modules are live handles on this device. No pipeline cache is used,
        // which `PipelineCache::null` denotes.
        let khutut = unsafe {
            self.jihaz.create_graphics_pipelines(vk::PipelineCache::null(), &malumat, None)
        };
        let khutut = match khutut {
            Ok(khutut) => khutut,
            Err((_, natija)) => return Err(khata_mawrid("graphics pipeline", natija)),
        };
        self.khatt = khutut.into_iter().next().ok_or_else(|| {
            KhataTabaqa::MawridFashil {
                mawrid: "graphics pipeline",
                sabab: "vkCreateGraphicsPipelines returned no pipelines for a request of one"
                    .to_owned(),
            }
        })?;
        Ok(())
    }

    /// Creates one view and one framebuffer per swapchain image.
    fn ansha_itarat(&mut self) -> Result<(), KhataTabaqa> {
        let mada = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);
        for sura in &self.silsila.suwar {
            let malumat = vk::ImageViewCreateInfo::default()
                .image(*sura)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.silsila.sigha)
                .components(vk::ComponentMapping::default())
                .subresource_range(mada);
            // SAFETY: the image is one the swapchain reported and is live for as
            // long as the swapchain is; `malumat` outlives the call.
            let manzar = unsafe { self.jihaz.create_image_view(&malumat, None) }
                .map_err(|natija| khata_mawrid("swapchain image view", natija))?;
            self.manazir.push(manzar);

            let murfaqat = [manzar];
            let malumat = vk::FramebufferCreateInfo::default()
                .render_pass(self.mamarr)
                .attachments(&murfaqat)
                .width(self.silsila.imtidad.width)
                .height(self.silsila.imtidad.height)
                .layers(1);
            // SAFETY: the render pass and the view are live and the extent is
            // the swapchain's own, which is what the render pass was built
            // against; `murfaqat` outlives the call.
            let itar = unsafe { self.jihaz.create_framebuffer(&malumat, None) }
                .map_err(|natija| khata_mawrid("framebuffer", natija))?;
            self.itarat.push(itar);
        }
        Ok(())
    }

    /// Creates one fence and one semaphore per swapchain image, twice over.
    ///
    /// The fences start **signalled**. The first frame waits on them before it
    /// records, and a fence created unsignalled would make that first wait block
    /// for a submission that never happened — a hang at startup, on the render
    /// thread, that looks exactly like the game freezing on launch.
    fn ansha_muzamana(&mut self) -> Result<(), KhataTabaqa> {
        let malumat_qayd =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let malumat_ishara = vk::SemaphoreCreateInfo::default();
        for _ in 0..self.silsila.suwar.len() {
            // SAFETY: both create-infos are live locals for the calls, and
            // neither carries a pointer to anything else.
            let (qayd_rasm, qayd_iltiqat, ishara_rasm, ishara_iltiqat) = unsafe {
                (
                    self.jihaz.create_fence(&malumat_qayd, None),
                    self.jihaz.create_fence(&malumat_qayd, None),
                    self.jihaz.create_semaphore(&malumat_ishara, None),
                    self.jihaz.create_semaphore(&malumat_ishara, None),
                )
            };
            self.quyud_rasm
                .push(qayd_rasm.map_err(|natija| khata_mawrid("draw fence", natija))?);
            self.quyud_iltiqat
                .push(qayd_iltiqat.map_err(|natija| khata_mawrid("capture fence", natija))?);
            self.isharat_rasm
                .push(ishara_rasm.map_err(|natija| khata_mawrid("draw semaphore", natija))?);
            self.isharat_iltiqat.push(
                ishara_iltiqat.map_err(|natija| khata_mawrid("capture semaphore", natija))?,
            );
        }
        Ok(())
    }

    /// Builds every surface-sized resource, in the order they depend on.
    ///
    /// Written as five statements rather than a chain of `and_then`, because
    /// each step needs `&mut self` and a chain would ask the borrow checker for
    /// five simultaneous mutable borrows of the same value. The order is the
    /// dependency order: the pipeline needs the render pass and the descriptor
    /// layout, and the framebuffers need the render pass.
    fn ansha_kul(&mut self) -> Result<(), KhataTabaqa> {
        self.ansha_mamarr()?;
        self.ansha_wasf()?;
        self.ansha_khatt()?;
        self.ansha_itarat()?;
        self.ansha_muzamana()?;
        Ok(())
    }
}

/// A field offset as the `u32` a vertex attribute description takes.
///
/// Saturates rather than wrapping. A `RaasQita` is thirty-six bytes, so the
/// saturation is unreachable; writing it as a fallible conversion rather than an
/// `as` is what keeps the workspace's cast lints denied in the one place a wrong
/// number would silently draw the colour where the position belongs.
fn izaha_u32(izaha: usize) -> u32 {
    u32::try_from(izaha).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// Release
// ---------------------------------------------------------------------------

impl KhattafVulkan {
    /// Destroys everything sized against the swapchain.
    ///
    /// `vkDeviceWaitIdle` first, unconditionally. Every object below may be
    /// referenced by a command buffer the GPU has not finished with, and Vulkan
    /// gives no way to ask "is this framebuffer still in use?" — the only
    /// available answer is "nothing is in use", and that is what this waits for.
    /// It is a stall, it happens on a resolution change, and a resolution change
    /// already stalls for far longer inside the driver.
    fn atlif_sath(&mut self) {
        // SAFETY: the device is live; a wait-idle touches no memory of ours and
        // is the guarantee every destroy below depends on. A device that has
        // been lost returns an error rather than blocking, which is why the
        // result is discarded — there is nothing to do about it and the objects
        // still have to go.
        let _ = unsafe { self.jihaz.device_wait_idle() };

        // SAFETY: every handle below was created on this device by the matching
        // `ansha_*` method and has not been destroyed, and the wait above
        // guarantees no submission still references any of them. A null handle
        // is defined as a no-op for each of these calls, which is what makes the
        // repeat call safe rather than merely tolerated.
        unsafe {
            for itar in self.itarat.drain(..) {
                self.jihaz.destroy_framebuffer(itar, None);
            }
            for manzar in self.manazir.drain(..) {
                self.jihaz.destroy_image_view(manzar, None);
            }
            for qayd in self.quyud_rasm.drain(..) {
                self.jihaz.destroy_fence(qayd, None);
            }
            for qayd in self.quyud_iltiqat.drain(..) {
                self.jihaz.destroy_fence(qayd, None);
            }
            for ishara in self.isharat_rasm.drain(..) {
                self.jihaz.destroy_semaphore(ishara, None);
            }
            for ishara in self.isharat_iltiqat.drain(..) {
                self.jihaz.destroy_semaphore(ishara, None);
            }
            if !self.khatt.is_null() {
                self.jihaz.destroy_pipeline(self.khatt, None);
                self.khatt = vk::Pipeline::null();
            }
            if !self.takhtit.is_null() {
                self.jihaz.destroy_pipeline_layout(self.takhtit, None);
                self.takhtit = vk::PipelineLayout::null();
            }
            if !self.hawd_wasf.is_null() {
                // Destroying the pool frees the set allocated from it, so the
                // set handle is forgotten rather than freed separately.
                self.jihaz.destroy_descriptor_pool(self.hawd_wasf, None);
                self.hawd_wasf = vk::DescriptorPool::null();
                self.majmuat_wasf = vk::DescriptorSet::null();
            }
            if !self.wahdat_wasf.is_null() {
                self.jihaz.destroy_descriptor_set_layout(self.wahdat_wasf, None);
                self.wahdat_wasf = vk::DescriptorSetLayout::null();
            }
            if !self.akhidh.is_null() {
                self.jihaz.destroy_sampler(self.akhidh, None);
                self.akhidh = vk::Sampler::null();
            }
            if !self.mamarr.is_null() {
                self.jihaz.destroy_render_pass(self.mamarr, None);
                self.mamarr = vk::RenderPass::null();
            }
            if !self.hawd_awamir.is_null() {
                self.jihaz.destroy_command_pool(self.hawd_awamir, None);
                self.hawd_awamir = vk::CommandPool::null();
            }
        }
        self.awamir_rasm.clear();
        self.awamir_iltiqat.clear();
        self.usrat_awamir = None;
    }

    /// Destroys the atlas image and everything that carried it.
    ///
    /// # Safety
    ///
    /// The device must be idle with respect to the atlas — no submission that
    /// samples it may still be executing.
    unsafe fn atlif_lawha(&mut self) {
        // SAFETY: the caller guarantees nothing in flight samples the atlas, and
        // every handle below was created on this device by `arfa_lawha`.
        unsafe {
            if !self.manzar_lawha.is_null() {
                self.jihaz.destroy_image_view(self.manzar_lawha, None);
                self.manzar_lawha = vk::ImageView::null();
            }
            if !self.lawha.is_null() {
                self.jihaz.destroy_image(self.lawha, None);
                self.lawha = vk::Image::null();
            }
            if !self.dhakirat_lawha.is_null() {
                self.jihaz.free_memory(self.dhakirat_lawha, None);
                self.dhakirat_lawha = vk::DeviceMemory::null();
            }
        }
        // The staging buffer is taken out of `self` before it is destroyed,
        // because `atlif_mahfaza` borrows `self` immutably and would otherwise
        // be handed a mutable borrow of one of its fields at the same time.
        let mut raf = core::mem::replace(&mut self.raf_lawha, MahfazaMazjura::khaliya());
        // SAFETY: the caller guarantees the device is idle with respect to the
        // atlas, and the staging buffer is only ever referenced by the upload
        // that the atlas replaced.
        unsafe { self.atlif_mahfaza(&mut raf) };
        self.qiyas_lawha = (0, 0);
    }

    /// Points the descriptor set at the atlas that exists now.
    ///
    /// Called after every atlas upload *and* after every surface rebuild,
    /// because the surface rebuild destroys the descriptor pool and the set with
    /// it. A rebuild that forgot this leaves a pipeline sampling a descriptor
    /// nothing was written into, which on a well-behaved driver is transparent
    /// black — an overlay that draws its backing plates and none of its text.
    fn aktub_wasf(&self) {
        if self.majmuat_wasf.is_null() || self.manzar_lawha.is_null() || self.akhidh.is_null() {
            return;
        }
        let suwar = [vk::DescriptorImageInfo::default()
            .sampler(self.akhidh)
            .image_view(self.manzar_lawha)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let kitabat = [vk::WriteDescriptorSet::default()
            .dst_set(self.majmuat_wasf)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&suwar)];
        // SAFETY: every handle in the write is live, `suwar` and `kitabat` are
        // live locals that outlive the call, and the set is not in use by a
        // command buffer that is executing — the only writes happen inside
        // `hayyi` and `arfa_lawha`, both of which run between frames.
        unsafe { self.jihaz.update_descriptor_sets(&kitabat, &[]) };
    }

    /// Records the swapchain the layer is now presenting.
    ///
    /// A resize destroys the old swapchain and creates a new one with new
    /// images, and every framebuffer and image view this renderer holds belongs
    /// to the old one. Calling this marks them stale; the next
    /// [`Khattaf::hayyi`] rebuilds against the new images.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SighaGhayrMaduma`] when the new swapchain's format is not
    /// one this build handles, which a game switching to HDR output produces.
    pub fn hadith_silsila(&mut self, silsila: WasfSilsila) -> Result<(), KhataTabaqa> {
        let sath = silsila.sath()?;
        if silsila.silsila == self.silsila.silsila && sath == self.sath {
            return Ok(());
        }
        self.athar.push(format!(
            "swapchain replaced: {}×{} becomes {}×{}",
            self.sath.ard, self.sath.irtifa, sath.ard, sath.irtifa
        ));
        self.atlif_sath();
        self.silsila = silsila;
        self.sath = sath;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

impl KhattafVulkan {
    /// Turns the batch into vertices and indices, touching no Vulkan object.
    ///
    /// Split from the submission for the same reason the OpenGL backend splits
    /// it: everything that can go wrong about a batch goes wrong here, before a
    /// fence has been waited on or a command buffer reset, so a bad batch costs
    /// a skipped frame rather than a stall and a half-recorded command buffer.
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
                // A plate samples nothing, and its coordinates are still written
                // rather than left alone: an attribute nobody wrote is an
                // attribute holding whatever the previous frame's batch put in
                // that buffer slot.
                None => (0.0, 0.0, 0.0, 0.0, 0.0),
            };
            let Ok(asas) = u32::try_from(self.ruus.len()) else {
                break;
            };
            self.ruus.push(RaasQita { mawdi: [yasar, aala], khareeta: [u0, v0], lawn, nasij });
            self.ruus.push(RaasQita { mawdi: [yameen, aala], khareeta: [u1, v0], lawn, nasij });
            self.ruus.push(RaasQita { mawdi: [yameen, asfal], khareeta: [u1, v1], lawn, nasij });
            self.ruus.push(RaasQita { mawdi: [yasar, asfal], khareeta: [u0, v1], lawn, nasij });
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

    /// The push-constant block as the bytes `vkCmdPushConstants` takes.
    ///
    /// Built field by field through `to_ne_bytes` rather than by viewing the
    /// struct as bytes. Both would work — [`ThawabitRasm`] is `#[repr(C)]` over
    /// four-byte scalars with no padding — and this way there is no unsafe block
    /// whose soundness rests on a claim about padding that a future field could
    /// quietly break.
    fn thawabit_bayt(&self) -> Vec<u8> {
        let thawabit = ThawabitRasm {
            qiyas: [
                2.0 / qeema_f32(self.sath.ard).max(1.0),
                2.0 / qeema_f32(self.sath.irtifa).max(1.0),
            ],
            tarmiz_sirgb: i32::from(!self.sath.sirgb),
            hashw: 0,
        };
        let [qiyas_x, qiyas_y] = thawabit.qiyas;
        let mut khaam = Vec::with_capacity(size_of::<ThawabitRasm>());
        khaam.extend_from_slice(&qiyas_x.to_ne_bytes());
        khaam.extend_from_slice(&qiyas_y.to_ne_bytes());
        khaam.extend_from_slice(&thawabit.tarmiz_sirgb.to_ne_bytes());
        khaam.extend_from_slice(&thawabit.hashw.to_ne_bytes());
        khaam
    }

    /// Uploads the batch, records the overlay's command buffer, and submits it.
    ///
    /// The submission is the point of the whole file. It waits on whatever
    /// `siyaq.intizar` holds — the application's own render-finished semaphores
    /// on the first submission of the frame, this renderer's previous signal on
    /// any after it — at `COLOR_ATTACHMENT_OUTPUT`, which is the earliest stage
    /// that could write the image and therefore the latest stage it is correct
    /// to wait at. It signals a semaphore of its own and republishes that as the
    /// pending wait, so the present the layer is about to make waits on the
    /// overlay rather than racing it.
    fn arsil_rasm(&mut self, siyaq: &mut SiyaqItar) -> Result<(), KhataTabaqa> {
        let Ok(fahras) = usize::try_from(siyaq.fahras_sura) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "swapchain image index",
                sabab: format!("image index {} does not fit a usize", siyaq.fahras_sura),
            });
        };
        let (Some(itar), Some(amr), Some(qayd), Some(ishara)) = (
            self.itarat.get(fahras).copied(),
            self.awamir_rasm.get(fahras).copied(),
            self.quyud_rasm.get(fahras).copied(),
            self.isharat_rasm.get(fahras).copied(),
        ) else {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "the present named image {fahras}, and this renderer has resources for {} \
                     image(s); the swapchain was replaced without the overlay being told",
                    self.itarat.len()
                ),
            });
        };

        let bayt_ruus = size_of_val(self.ruus.as_slice());
        let bayt_faharis = size_of_val(self.faharis.as_slice());
        let (Ok(tul_ruus), Ok(tul_faharis)) =
            (u64::try_from(bayt_ruus), u64::try_from(bayt_faharis))
        else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "vertex buffer",
                sabab: "the batch does not fit a VkDeviceSize".to_owned(),
            });
        };

        // The fence is waited on **before** anything is rewritten or
        // reallocated. It is signalled when the previous frame that used this
        // image finished, which is the only guarantee available that the GPU is
        // no longer reading these buffers or executing this command buffer.
        //
        // SAFETY: `qayd` is this image's own fence, created signalled so the
        // first frame does not block on a submission that never happened.
        unsafe { self.jihaz.wait_for_fences(&[qayd], true, MUHLAT_QAYD) }
            .map_err(|natija| khata_mawrid("draw fence wait", natija))?;

        // The two streaming buffers are taken out of `self` while they are
        // grown, because `wassi_mahfaza` borrows `self` immutably to reach the
        // device and cannot also be handed a mutable borrow of one of its
        // fields. They go back in unconditionally, including on the error path,
        // so a failed growth does not leak the allocation that succeeded.
        let mut ruus_gpu = core::mem::replace(&mut self.ruus_gpu, MahfazaMazjura::khaliya());
        let mut faharis_gpu =
            core::mem::replace(&mut self.faharis_gpu, MahfazaMazjura::khaliya());
        // SAFETY: the fence wait above proves nothing in flight reads either
        // buffer, which is exactly what `wassi_mahfaza` requires of its caller.
        let numuw = unsafe {
            let awwal = self.wassi_mahfaza(
                &mut ruus_gpu,
                tul_ruus,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                "vertex buffer",
            );
            let thani = self.wassi_mahfaza(
                &mut faharis_gpu,
                tul_faharis,
                vk::BufferUsageFlags::INDEX_BUFFER,
                "index buffer",
            );
            awwal.and(thani)
        };
        self.ruus_gpu = ruus_gpu;
        self.faharis_gpu = faharis_gpu;
        numuw?;

        // SAFETY: `RaasQita` and `u32` are `#[repr(C)]`/primitive types with no
        // padding and no uninitialised byte, and `size_of_val` bytes of a live
        // slice viewed as `u8` stays inside the same allocation at an alignment
        // of one.
        let (khaam_ruus, khaam_faharis) = unsafe {
            (
                core::slice::from_raw_parts(self.ruus.as_ptr().cast::<u8>(), bayt_ruus),
                core::slice::from_raw_parts(self.faharis.as_ptr().cast::<u8>(), bayt_faharis),
            )
        };
        if !self.ruus_gpu.iktub(khaam_ruus) || !self.faharis_gpu.iktub(khaam_faharis) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "vertex buffer",
                sabab: "the streaming buffers did not accept the batch after being grown to \
                        fit it, which means the mapping was lost"
                    .to_owned(),
            });
        }

        // SAFETY: the fence proves this image's previous submission finished, so
        // the fence may be reset and the command buffer re-recorded.
        unsafe { self.jihaz.reset_fences(&[qayd]) }
            .map_err(|natija| khata_mawrid("draw fence reset", natija))?;

        let thawabit = self.thawabit_bayt();
        let mada = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.silsila.imtidad,
        };
        let manzur = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: qeema_f32(self.silsila.imtidad.width),
            height: qeema_f32(self.silsila.imtidad.height),
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let Ok(adad_faharis) = u32::try_from(self.faharis.len()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "index buffer",
                sabab: format!("{} indices is more than a u32 can count", self.faharis.len()),
            });
        };

        let bidaya = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let mamarr = vk::RenderPassBeginInfo::default()
            .render_pass(self.mamarr)
            .framebuffer(itar)
            .render_area(mada);
        // SAFETY: `amr` is this image's command buffer, allocated from a pool
        // created with `RESET_COMMAND_BUFFER`, and the fence proves the GPU has
        // finished with it. Every handle recorded into it is live, the
        // framebuffer belongs to the render pass being begun, and the draw reads
        // exactly the `adad_faharis` indices that were written into the index
        // buffer above — each of which addresses a vertex written in the same
        // pass. The push-constant slice is sixteen bytes and the layout declares
        // a sixteen-byte range at offset zero.
        let tasjeel = unsafe {
            let mut hasila =
                self.jihaz.reset_command_buffer(amr, vk::CommandBufferResetFlags::empty());
            if hasila.is_ok() {
                hasila = self.jihaz.begin_command_buffer(amr, &bidaya);
            }
            if hasila.is_ok() {
                self.jihaz.cmd_begin_render_pass(amr, &mamarr, vk::SubpassContents::INLINE);
                self.jihaz.cmd_bind_pipeline(
                    amr,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.khatt,
                );
                self.jihaz.cmd_set_viewport(amr, 0, &[manzur]);
                self.jihaz.cmd_set_scissor(amr, 0, &[mada]);
                self.jihaz.cmd_push_constants(
                    amr,
                    self.takhtit,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    &thawabit,
                );
                self.jihaz.cmd_bind_descriptor_sets(
                    amr,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.takhtit,
                    0,
                    &[self.majmuat_wasf],
                    &[],
                );
                self.jihaz.cmd_bind_vertex_buffers(amr, 0, &[self.ruus_gpu.mahfaza], &[0]);
                self.jihaz.cmd_bind_index_buffer(
                    amr,
                    self.faharis_gpu.mahfaza,
                    0,
                    vk::IndexType::UINT32,
                );
                self.jihaz.cmd_draw_indexed(amr, adad_faharis, 1, 0, 0, 0);
                self.jihaz.cmd_end_render_pass(amr);
                hasila = self.jihaz.end_command_buffer(amr);
            }
            hasila
        };
        tasjeel.map_err(|natija| khata_mawrid("draw command buffer", natija))?;

        let marahil = vec![
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            siyaq.intizar.len()
        ];
        let awamir = [amr];
        let isharat = [ishara];
        let irsal = [vk::SubmitInfo::default()
            .wait_semaphores(&siyaq.intizar)
            .wait_dst_stage_mask(&marahil)
            .command_buffers(&awamir)
            .signal_semaphores(&isharat)];
        // SAFETY: the wait list and the stage mask are the same length, which is
        // what `VkSubmitInfo` requires; every semaphore in the wait list is one
        // the application's own present was going to wait on, so each is either
        // already signalled or will be; the command buffer has been ended; and
        // the queue is the one the present hook is executing on, which the
        // application must externally synchronise around its own present.
        unsafe { self.jihaz.queue_submit(siyaq.tabur, &irsal, qayd) }
            .map_err(|natija| khata_mawrid("overlay submission", natija))?;

        siyaq.intizar = vec![ishara];
        siyaq.sallam = true;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The atlas
// ---------------------------------------------------------------------------

impl KhattafVulkan {
    /// Creates the atlas image and uploads the glyph pages into it.
    ///
    /// The upload is a staging buffer, a copy, and two layout transitions —
    /// `UNDEFINED` to `TRANSFER_DST_OPTIMAL` before the copy and
    /// `TRANSFER_DST_OPTIMAL` to `SHADER_READ_ONLY_OPTIMAL` after it. `UNDEFINED`
    /// as the source layout is correct and is not laziness: the image's previous
    /// contents are being replaced wholesale, and transitioning *from* the real
    /// old layout would ask the driver to preserve data that is about to be
    /// overwritten.
    ///
    /// It waits for the copy to finish before returning, on a fence. That is a
    /// stall on the render thread and it is the right trade: an atlas upload
    /// happens when the font size changes or a new script appears, which is a
    /// handful of times in a session, and the alternative is keeping the staging
    /// buffer alive across frames to be freed by a callback that does not exist.
    fn arfa_nasij(
        &mut self,
        bayt: &[u8],
        ard: u32,
        irtifa: u32,
        siyaq: &SiyaqItar,
    ) -> Result<(), KhataTabaqa> {
        // SAFETY: replacing the atlas means destroying an image a previously
        // submitted frame may still be sampling, and `vkDeviceWaitIdle` is the
        // only guarantee that says it is not.
        let _ = unsafe { self.jihaz.device_wait_idle() };
        // SAFETY: the wait above makes the device idle, which is exactly what
        // `atlif_lawha` requires.
        unsafe { self.atlif_lawha() };

        let malumat = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D { width: ard, height: irtifa, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        // SAFETY: `malumat` is a live create-info for the call describing a
        // plain two-dimensional image with no queue-family list, which is what
        // `EXCLUSIVE` requires.
        self.lawha = unsafe { self.jihaz.create_image(&malumat, None) }
            .map_err(|natija| khata_mawrid("glyph atlas image", natija))?;

        // SAFETY: the image was just created on this device.
        let mutatallabat = unsafe { self.jihaz.get_image_memory_requirements(self.lawha) };
        let Some(naw) = ikhtar_naw_dhakira(
            &self.dhakira,
            mutatallabat.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas image",
                sabab: format!(
                    "no DEVICE_LOCAL memory type accepts this image among the {:#x} it allows",
                    mutatallabat.memory_type_bits
                ),
            });
        };
        let hajz = vk::MemoryAllocateInfo::default()
            .allocation_size(mutatallabat.size)
            .memory_type_index(naw);
        // SAFETY: `hajz` is live for the call and names a memory type the search
        // above proved is in range and accepted by this image.
        self.dhakirat_lawha = unsafe { self.jihaz.allocate_memory(&hajz, None) }
            .map_err(|natija| khata_mawrid("glyph atlas memory", natija))?;
        // SAFETY: both handles are live, the allocation is large enough and of
        // an accepted type, and the image has not been bound before.
        unsafe { self.jihaz.bind_image_memory(self.lawha, self.dhakirat_lawha, 0) }
            .map_err(|natija| khata_mawrid("glyph atlas memory binding", natija))?;

        let mada = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);
        let malumat = vk::ImageViewCreateInfo::default()
            .image(self.lawha)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .components(vk::ComponentMapping::default())
            .subresource_range(mada);
        // SAFETY: the image is live and bound, and `malumat` outlives the call.
        self.manzar_lawha = unsafe { self.jihaz.create_image_view(&malumat, None) }
            .map_err(|natija| khata_mawrid("glyph atlas view", natija))?;

        let Ok(tul) = u64::try_from(bayt.len()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas staging buffer",
                sabab: "the atlas does not fit a VkDeviceSize".to_owned(),
            });
        };
        self.raf_lawha = self.ansha_mahfaza(
            tul,
            vk::BufferUsageFlags::TRANSFER_SRC,
            "glyph atlas staging buffer",
        )?;
        if !self.raf_lawha.iktub(bayt) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas staging buffer",
                sabab: "the staging buffer would not accept the atlas bytes".to_owned(),
            });
        }

        self.naql_lawha(ard, irtifa, siyaq)?;
        self.qiyas_lawha = (ard, irtifa);
        self.aktub_wasf();
        // The staging buffer has served its only purpose and the copy has been
        // waited on, so it goes now rather than staying mapped for the rest of
        // the session. An atlas is measured in tens of megabytes.
        let mut raf = core::mem::replace(&mut self.raf_lawha, MahfazaMazjura::khaliya());
        // SAFETY: `naql_lawha` waited on its fence, so the copy that read this
        // buffer has completed and nothing references it.
        unsafe { self.atlif_mahfaza(&mut raf) };
        self.athar.push(format!("glyph atlas uploaded at {ard}×{irtifa}"));
        Ok(())
    }

    /// Records and submits the atlas copy, and waits for it.
    fn naql_lawha(
        &mut self,
        ard: u32,
        irtifa: u32,
        siyaq: &SiyaqItar,
    ) -> Result<(), KhataTabaqa> {
        let Some(amr) = self.awamir_iltiqat.first().copied() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas upload command buffer",
                sabab: "the command pool has no buffers, which means the presenting queue's \
                        family was never established"
                    .to_owned(),
            });
        };
        let Some(qayd) = self.quyud_iltiqat.first().copied() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas upload fence",
                sabab: "the synchronisation objects were not created, which means `hayyi` has \
                        not run"
                    .to_owned(),
            });
        };

        let mada = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);
        let ila_naql = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.lawha)
            .subresource_range(mada);
        let ila_qira = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.lawha)
            .subresource_range(mada);
        let mintaqa = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D { width: ard, height: irtifa, depth: 1 });
        let bidaya = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        // SAFETY: `amr` comes from a pool created with `RESET_COMMAND_BUFFER`
        // and is not in flight — the fence below is reset and waited on around
        // this submission and nothing else uses this buffer while an upload is
        // in progress. Every handle recorded is live, the buffer holds
        // `ard * irtifa * 4` bytes and the copy names exactly that region, and
        // both barriers name the same image and subresource the copy writes.
        let tasjeel = unsafe {
            let mut hasila =
                self.jihaz.reset_command_buffer(amr, vk::CommandBufferResetFlags::empty());
            if hasila.is_ok() {
                hasila = self.jihaz.begin_command_buffer(amr, &bidaya);
            }
            if hasila.is_ok() {
                self.jihaz.cmd_pipeline_barrier(
                    amr,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ila_naql],
                );
                self.jihaz.cmd_copy_buffer_to_image(
                    amr,
                    self.raf_lawha.mahfaza,
                    self.lawha,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[mintaqa],
                );
                self.jihaz.cmd_pipeline_barrier(
                    amr,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ila_qira],
                );
                hasila = self.jihaz.end_command_buffer(amr);
            }
            hasila
        };
        tasjeel.map_err(|natija| khata_mawrid("glyph atlas upload recording", natija))?;

        let awamir = [amr];
        let irsal = [vk::SubmitInfo::default().command_buffers(&awamir)];
        // SAFETY: the command buffer has been ended, the fence is this
        // renderer's own, and the queue is the one the present hook is executing
        // on. The submission waits on nothing because it reads only a staging
        // buffer the host has just written and an image nothing else touches.
        unsafe {
            self.jihaz
                .reset_fences(&[qayd])
                .map_err(|natija| khata_mawrid("glyph atlas upload fence", natija))?;
            self.jihaz
                .queue_submit(siyaq.tabur, &irsal, qayd)
                .map_err(|natija| khata_mawrid("glyph atlas upload submission", natija))?;
            self.jihaz
                .wait_for_fences(&[qayd], true, MUHLAT_QAYD)
                .map_err(|natija| khata_mawrid("glyph atlas upload wait", natija))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

impl KhattafVulkan {
    /// Copies one rectangle of the presented image into host memory.
    ///
    /// The layout dance is `PRESENT_SRC_KHR` → `TRANSFER_SRC_OPTIMAL` →
    /// `PRESENT_SRC_KHR`, and both halves are required. The image arrives ready
    /// to present, because that is what the game left it as; it has to be a
    /// transfer source for the copy; and it has to be handed back ready to
    /// present, because the present is the next thing that happens to it. A
    /// capture that transitioned only on the way in leaves the presentation
    /// engine an image in `TRANSFER_SRC_OPTIMAL`, which is a validation error on
    /// a good driver and a corrupt frame on a bad one.
    ///
    /// The submission joins the same semaphore chain the draw does: it waits on
    /// whatever is pending and republishes its own signal, so a frame that both
    /// captures and draws produces two submissions in order rather than two
    /// submissions racing.
    fn iqra_mintaqa(
        &mut self,
        siyaq: &mut SiyaqItar,
        mintaqa: MustatilBiksel,
    ) -> Result<Vec<u8>, KhataTabaqa> {
        if !self.silsila.istikhdam.contains(vk::ImageUsageFlags::TRANSFER_SRC) {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the swapchain was created without TRANSFER_SRC usage and the surface \
                        does not support adding it, so its images cannot be copied out of"
                    .to_owned(),
            });
        }
        let Ok(fahras) = usize::try_from(siyaq.fahras_sura) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!("image index {} does not fit a usize", siyaq.fahras_sura),
            });
        };
        let (Some(sura), Some(amr), Some(qayd), Some(ishara)) = (
            self.silsila.suwar.get(fahras).copied(),
            self.awamir_iltiqat.get(fahras).copied(),
            self.quyud_iltiqat.get(fahras).copied(),
            self.isharat_iltiqat.get(fahras).copied(),
        ) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the present named image {fahras} and this renderer has resources for {}",
                    self.silsila.suwar.len()
                ),
            });
        };

        let bayt_biksel = u64::from(self.sath.sigha.bayt_lil_biksel());
        let Some(tul) = u64::from(mintaqa.ard)
            .checked_mul(u64::from(mintaqa.irtifa))
            .and_then(|bikselat| bikselat.checked_mul(bayt_biksel))
        else {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "capture region",
                qeema: u64::MAX,
                saqf: AQSA_ILTIQAT,
            });
        };
        if tul > AQSA_ILTIQAT {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "capture region",
                qeema: tul,
                saqf: AQSA_ILTIQAT,
            });
        }
        let Ok(tul_usize) = usize::try_from(tul) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the region does not fit in memory on this target".to_owned(),
            });
        };

        // SAFETY: the fence below is waited on before the staging buffer is
        // read, and the buffer is grown here only when the previous capture has
        // already completed — which the wait on this same fence at the end of
        // the previous call established.
        unsafe { self.jihaz.wait_for_fences(&[qayd], true, MUHLAT_QAYD) }
            .map_err(|natija| khata_mawrid("capture fence wait", natija))?;

        let mut iltiqat_gpu =
            core::mem::replace(&mut self.iltiqat_gpu, MahfazaMazjura::khaliya());
        // SAFETY: the wait above proves the previous capture finished, so
        // nothing in flight references the buffer being replaced.
        let numuw = unsafe {
            self.wassi_mahfaza(
                &mut iltiqat_gpu,
                tul,
                vk::BufferUsageFlags::TRANSFER_DST,
                "capture staging buffer",
            )
        };
        self.iltiqat_gpu = iltiqat_gpu;
        numuw?;

        let (Ok(x), Ok(y)) = (i32::try_from(mintaqa.yasar), i32::try_from(mintaqa.aala)) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the region's pixel coordinates do not fit a VkOffset3D".to_owned(),
            });
        };
        let (ard, irtifa) = (mintaqa.ard, mintaqa.irtifa);

        let mada = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);
        let ila_naql = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_READ)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(sura)
            .subresource_range(mada);
        let ila_ard = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ)
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(sura)
            .subresource_range(mada);
        // `buffer_row_length` and `buffer_image_height` of zero mean "tightly
        // packed to the region's own extent", which is exactly what
        // `Khattaf::iltaqit` promises its caller. Setting them to the *surface's*
        // width — the intuitive mistake — would copy the region with the full
        // screen's pitch and hand the recognizer an image sheared diagonally.
        let mintaqa_naql = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image_offset(vk::Offset3D { x, y, z: 0 })
            .image_extent(vk::Extent3D { width: ard, height: irtifa, depth: 1 });
        let bidaya = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        // SAFETY: `amr` is this image's capture command buffer, from a pool
        // created with `RESET_COMMAND_BUFFER`, and the fence wait above proves
        // the GPU has finished with it. The swapchain image is live and in
        // `PRESENT_SRC_KHR`, which is the layout the game left it in and the one
        // the first barrier names as old. The destination buffer holds exactly
        // `tul` bytes and the copy writes exactly that many.
        let tasjeel = unsafe {
            let mut hasila =
                self.jihaz.reset_command_buffer(amr, vk::CommandBufferResetFlags::empty());
            if hasila.is_ok() {
                hasila = self.jihaz.begin_command_buffer(amr, &bidaya);
            }
            if hasila.is_ok() {
                self.jihaz.cmd_pipeline_barrier(
                    amr,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ila_naql],
                );
                self.jihaz.cmd_copy_image_to_buffer(
                    amr,
                    sura,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    self.iltiqat_gpu.mahfaza,
                    &[mintaqa_naql],
                );
                self.jihaz.cmd_pipeline_barrier(
                    amr,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ila_ard],
                );
                hasila = self.jihaz.end_command_buffer(amr);
            }
            hasila
        };
        tasjeel.map_err(|natija| khata_mawrid("capture command buffer", natija))?;

        let marahil =
            vec![vk::PipelineStageFlags::TRANSFER; siyaq.intizar.len()];
        let awamir = [amr];
        let isharat = [ishara];
        let irsal = [vk::SubmitInfo::default()
            .wait_semaphores(&siyaq.intizar)
            .wait_dst_stage_mask(&marahil)
            .command_buffers(&awamir)
            .signal_semaphores(&isharat)];
        // SAFETY: the wait list and its stage mask are the same length, the
        // command buffer has been ended, and the queue is the presenting one the
        // hook is running on.
        unsafe {
            self.jihaz
                .reset_fences(&[qayd])
                .map_err(|natija| khata_mawrid("capture fence reset", natija))?;
            self.jihaz
                .queue_submit(siyaq.tabur, &irsal, qayd)
                .map_err(|natija| khata_mawrid("capture submission", natija))?;
        }
        siyaq.intizar = vec![ishara];
        siyaq.sallam = true;

        // SAFETY: the fence was submitted with the copy and is signalled when it
        // completes, which is what makes reading the host mapping below a read
        // of finished data rather than a race with the GPU.
        unsafe { self.jihaz.wait_for_fences(&[qayd], true, MUHLAT_QAYD) }
            .map_err(|natija| khata_mawrid("capture fence wait", natija))?;

        let khuruj = self.iltiqat_gpu.iqra(tul_usize);
        if khuruj.len() != tul_usize {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the staging buffer returned {} bytes for a {tul_usize}-byte region, which \
                     means its mapping was lost",
                    khuruj.len()
                ),
            });
        }
        Ok(khuruj)
    }
}

// ---------------------------------------------------------------------------
// The contract
// ---------------------------------------------------------------------------

impl Khattaf for KhattafVulkan {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::Vulkan
    }

    /// The surface, straight out of the swapchain's own create-info.
    ///
    /// This is the one backend that does not have to infer anything. The format,
    /// the extent and the colour space were all named by the application when it
    /// created the swapchain, and the layer captured them at that moment — so
    /// there is no query, no viewport heuristic and no platform call here, only
    /// a value that was written down when it was true.
    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        if self.silsila.imtidad.width == 0 || self.silsila.imtidad.height == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "the swapchain measures {}×{}, which a window minimised or mid-resize \
                     produces",
                    self.silsila.imtidad.width, self.silsila.imtidad.height
                ),
            });
        }
        self.silsila.sath()
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        let mutawaqqa = self.silsila.sath()?;
        if mutawaqqa != sath {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "the overlay was asked to build for {}×{} while the swapchain is {}×{}; the \
                     layer must call `hadith_silsila` before `hayyi` when the swapchain is \
                     replaced",
                    sath.ard, sath.irtifa, mutawaqqa.ard, mutawaqqa.irtifa
                ),
            });
        }

        self.atlif_sath();
        self.sath = sath;
        if let Err(khata) = self.ansha_kul() {
            // A half-built renderer is torn down rather than left for the next
            // frame to trip over: the draw path branches on the pipeline being
            // non-null and has no branch for a pipeline that exists without
            // framebuffers.
            self.atlif_sath();
            return Err(khata);
        }

        // The descriptor pool was destroyed and rebuilt above, taking the
        // descriptor set with it, so an atlas that was already uploaded has to
        // be pointed at again. Forgetting this is how a resize turns the
        // overlay's text invisible while its backing plates keep drawing.
        self.aktub_wasf();
        self.athar.push(format!(
            "resources built for {}×{} across {} swapchain image(s)",
            sath.ard,
            sath.irtifa,
            self.silsila.suwar.len()
        ));
        Ok(())
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

        let Some(siyaq) = self.siyaq.take() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: "there is no present in progress, and the upload needs the presenting \
                        queue; call `ibda_taqdeem` from the layer's present handler first"
                    .to_owned(),
            });
        };
        // The upload submits and waits on its own fence rather than joining the
        // frame's semaphore chain, so `siyaq` goes back untouched: an atlas
        // upload must not make the present wait on a semaphore, because it
        // happens between frames and not inside one.
        let natija = match siyaq.usra {
            Some(usra) => match self.tahaqqaq_hawd(usra) {
                Ok(()) => self.arfa_nasij(bayt, ard, irtifa, &siyaq),
                Err(khata) => Err(khata),
            },
            None => Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: "the presenting queue's family is unknown, because the game never \
                        retrieved that queue through vkGetDeviceQueue"
                    .to_owned(),
            }),
        };
        self.siyaq = Some(siyaq);
        natija
    }

    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if lawha.khali() {
            return Ok(());
        }
        if lawha.qitaat.len() > AQSA_QITAAT {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: format!(
                    "the batch carries {} quads, above this build's ceiling of {AQSA_QITAAT}",
                    lawha.qitaat.len()
                ),
            });
        }
        if self.khatt.is_null() || self.itarat.is_empty() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: "the overlay was asked to draw before its pipeline and framebuffers \
                        existed, which means `hayyi` has not run or failed"
                    .to_owned(),
            });
        }
        if self.manzar_lawha.is_null() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: "no glyph atlas has been uploaded, and the fragment shader samples one \
                        for every quad that is not a backing plate"
                    .to_owned(),
            });
        }

        let Some(mut siyaq) = self.siyaq.take() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: "there is no present in progress; the layer's present handler calls \
                        `ibda_taqdeem` before asking the overlay to draw"
                    .to_owned(),
            });
        };
        let Some(usra) = siyaq.usra else {
            self.siyaq = Some(siyaq);
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "command pool",
                sabab: "the presenting queue's family is unknown, because the game never \
                        retrieved that queue through vkGetDeviceQueue"
                    .to_owned(),
            });
        };

        let natija = match self.tahaqqaq_hawd(usra) {
            Ok(()) => {
                self.ibni_dufaa(lawha);
                if self.faharis.is_empty() {
                    // Every quad had a zero dimension, which a shaped line with
                    // no visible glyphs produces. Submitting a draw of nothing
                    // would still rewire the present's semaphores.
                    Ok(())
                } else {
                    self.arsil_rasm(&mut siyaq)
                }
            }
            Err(khata) => Err(khata),
        };
        self.siyaq = Some(siyaq);
        natija
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
        match (mintaqa.yasar.checked_add(mintaqa.ard), mintaqa.aala.checked_add(mintaqa.irtifa))
        {
            (Some(yameen), Some(asfal)) if yameen <= sath.ard && asfal <= sath.irtifa => {}
            _ => {
                return Err(KhataTabaqa::MintaqaKharij {
                    mintaqa: wasf(),
                    ard: sath.ard,
                    irtifa: sath.irtifa,
                });
            }
        }

        let Some(mut siyaq) = self.siyaq.take() else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "there is no present in progress, so there is no image to read; the \
                        layer's present handler calls `ibda_taqdeem` first"
                    .to_owned(),
            });
        };
        let natija = match siyaq.usra {
            Some(usra) => match self.tahaqqaq_hawd(usra) {
                Ok(()) => self.iqra_mintaqa(&mut siyaq, mintaqa),
                Err(khata) => Err(khata),
            },
            None => Err(KhataTabaqa::IltiqatFashil {
                sabab: "the presenting queue's family is unknown, because the game never \
                        retrieved that queue through vkGetDeviceQueue"
                    .to_owned(),
            }),
        };
        self.siyaq = Some(siyaq);
        natija
    }

    /// Waits for the device, then destroys everything in reverse order.
    ///
    /// `vkDeviceWaitIdle` first is not a precaution that could be dropped for
    /// speed. Destroying a framebuffer, a pipeline or a semaphore that a
    /// submitted command buffer still references is undefined behaviour, and at
    /// the moment this is called — a shutdown, or the terminal fault path —
    /// there is by definition a frame in flight. Idempotent because every handle
    /// is nulled as it goes and a null handle is a no-op for every destroy call
    /// Vulkan has.
    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        // SAFETY: the device is live for as long as this renderer holds handles
        // from it. A lost device returns an error instead of blocking, which is
        // reported below rather than acted on.
        let intizar = unsafe { self.jihaz.device_wait_idle() };

        // SAFETY: the wait above — or its failure, which means the device is
        // gone and nothing is executing on it either — establishes that nothing
        // in flight references the atlas.
        unsafe { self.atlif_lawha() };
        self.atlif_sath();

        let mut ruus = core::mem::replace(&mut self.ruus_gpu, MahfazaMazjura::khaliya());
        let mut faharis = core::mem::replace(&mut self.faharis_gpu, MahfazaMazjura::khaliya());
        let mut iltiqat = core::mem::replace(&mut self.iltiqat_gpu, MahfazaMazjura::khaliya());
        // SAFETY: as above; the device is idle with respect to all three
        // streaming buffers.
        unsafe {
            self.atlif_mahfaza(&mut ruus);
            self.atlif_mahfaza(&mut faharis);
            self.atlif_mahfaza(&mut iltiqat);
        }

        self.siyaq = None;
        self.ruus = Vec::new();
        self.faharis = Vec::new();

        match intizar {
            Ok(()) => {
                self.athar.push("released every Vulkan object the overlay created".to_owned());
                Ok(())
            }
            Err(natija) => {
                let sabab = format!(
                    "vkDeviceWaitIdle reported {natija} before the overlay released its \
                     objects; they were released anyway, because a lost device is not \
                     executing anything either"
                );
                self.athar.push(sabab.clone());
                Err(KhataTabaqa::MawridFashil { mawrid: "the overlay's Vulkan objects", sabab })
            }
        }
    }
}
