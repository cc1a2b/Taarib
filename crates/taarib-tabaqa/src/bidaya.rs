//! البداية — the payload bootstrap, and the one place the overlay is allowed to
//! start itself.
//!
//! `taarib-mudkhal` opens every payload sitting beside it and calls
//! [`taarib_bidaya`] on the ones that export it. This module is that function
//! for the graphics tier, and it implements `docs/bidaya.md` in the order that
//! document sets out: where am I, what am I inside, is there a patch, was the
//! disclosure acknowledged, install what the adapter needs, hand over.
//!
//! ## Why the handover happens from a hook and not from here
//!
//! A backend cannot be built until the game has presented a frame. Every
//! constructor in this crate takes something that only exists after the
//! renderer is up — [`crate::d3d11::KhattafD3D11::min_silsila`] takes the
//! game's swap chain, [`crate::d3d12::KhattafD3D12::min_silsila_wa_saff`] takes
//! its swap chain *and* the command queue it presents on, and
//! [`crate::gl::KhattafGl::jadeed`] resolves entry points through a context
//! that is only current on the render thread. `taarib_bidaya` runs on the
//! loader's thread before the game has finished starting, so none of the three
//! can be built here.
//!
//! So this module installs the hook that catches the first present, and the
//! backend plus [`crate::wajiha::Tabaqa::shaghghil`] run from inside it. That
//! is the whole reason the frame hook exists at bootstrap: not to draw — there
//! is nothing to draw yet — but to be the first moment at which a backend can
//! honestly be constructed.
//!
//! ## Vulkan is not reached from here
//!
//! The Vulkan loader calls `vkNegotiateLoaderLayerInterfaceVersion` in
//! [`crate::vulkan`] directly, which is why that one path has worked all along.
//! Initialising it a second way from here would put two overlays on one
//! presentation queue, so the module search below does not contain Vulkan at
//! all, and the start path stands down if it finds the layer already live —
//! which is exactly what a DXVK or vkd3d title looks like from inside a
//! `d3d11.dll` that is really Vulkan underneath.
//!
//! ## The log is the only surface there is
//!
//! A payload inside a game has no console, no window and no channel to Studio.
//! Every step that can decline writes one line to `<own dir>/taarib.sijill`,
//! capped, and the four states the contract names — declined, refused, failed,
//! started — are kept distinct because they mean different things to whoever
//! reads that file.

use core::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use taarib_haqn::khatf::Masar;
use taarib_haqn::mawqi::{mujallad_nafsi, qaidat_wahda, ramz_wahda};
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_usus::masarat::Masarat;

use crate::khataf::{AaddadDukhul, AslMahfuz, HirasatDukhul};
use crate::sidq::{BasmatIfsah, Iqrar, mahfuz_salih};
use crate::wajiha::{Khattaf, Tabaqa, WajihatRusum};

#[cfg(windows)]
use taarib_haqn::jadwal::{KhatfJadwal, jadwal_min_wajiha};
#[cfg(windows)]
use windows::Win32::Foundation::{HMODULE, HWND, RECT};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0,
};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11CreateDeviceAndSwapChain, ID3D11Device,
};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D12::{
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
    D3D12CreateDevice, ID3D12CommandQueue, ID3D12Device,
};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_PRESENT_PARAMETERS, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT,
    DXGI_SWAP_EFFECT_DISCARD, DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
    IDXGIFactory, IDXGISwapChain, IDXGISwapChain1, IDXGISwapChain3,
};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D9::{
    D3D_SDK_VERSION, D3DADAPTER_DEFAULT, D3DCREATE_FPU_PRESERVE, D3DCREATE_MULTITHREADED,
    D3DCREATE_NOWINDOWCHANGES, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL,
    D3DFMT_UNKNOWN, D3DPRESENT_PARAMETERS, D3DSWAPEFFECT_DISCARD, Direct3DCreate9,
    IDirect3DDevice9, IDirect3DDevice9Ex,
};
#[cfg(windows)]
use windows::core::{BOOL, HRESULT, Interface};

#[cfg(windows)]
use crate::d3d11::KhattafD3D11;
#[cfg(windows)]
use crate::d3d8::{KhattafD3D8, TarkeebD3D8};
#[cfg(windows)]
use crate::d3d10::KhattafD3D10;
#[cfg(windows)]
use crate::d3d9::KhattafD3D9;
#[cfg(windows)]
use crate::d3d12::KhattafD3D12;
#[cfg(windows)]
use crate::khataf::{
    KHANAT_ISTIAADA, KHANAT_ISTIAADA_MUMTADDA, KHANAT_PRESENT, KHANAT_PRESENT1, KHANAT_RESIZE,
    KHANAT_TANFEEDH, KHANAT_TAQDEEM9, KHANAT_TAQDEEM_MUMTADD, NafidhaMuaqqata,
    nafidha_muaqqata,
};

/// The log this payload appends to, beside the module itself.
///
/// Named by `docs/bidaya.md`. Deliberately not the loader's `mudkhal.sijill`:
/// two writers appending to one file inside a game would interleave lines from
/// different processes' threads, and the loader's log is about loading while
/// this one is about what the overlay decided.
const ISM_SIJILL: &str = "taarib.sijill";

/// The cap on that log, past which it is truncated before the next append.
///
/// A game may be launched hundreds of times with this payload in place, and a
/// payload that grew a log without bound inside somebody's game directory would
/// be a payload that eventually fills their disk.
const AQSA_SIJILL: u64 = 262_144;

/// The file the Studio writes the disclosure acknowledgement into.
///
/// The record is `{"basma": <u64>, "waqt": "<RFC 3339>"}` and is read, never
/// written, from inside a game.
use crate::sidq::ISM_MALAF_IQRAR as ISM_IQRAR_IFSAH;

/// The extension `taarib_tathbeet::masar_tathbeet` writes a patch under.
const LAHIQAT_RUQAA: &str = "ruqaa";

/// The subdirectory of a game that belongs to Taarib.
///
/// The same value as `taarib_tathbeet::mawdi::MUJALLAD_TAARIB`, spelled again
/// rather than depended on: the installer crate pulls in discovery, signing and
/// backup machinery that has no business inside a game's address space, and the
/// only use this module has for the name is deciding whether the directory it
/// was loaded from is a game's `taarib/` or the game's own executable directory.
const MUJALLAD_TAARIB: &str = "taarib";

/// The modules that answer for each graphics API, in the order they are asked.
///
/// D3D12 before D3D11 because a game linking both DXGI generations is almost
/// always a D3D12 game with a D3D11 compatibility path it does not present
/// through, and OpenGL last because `opengl32.dll` is loaded by a great many
/// Windows processes that never draw with it. Vulkan is absent by design; see
/// the module documentation.
#[cfg(windows)]
const WAHDAT: &[(WajihatRusum, &str)] = &[
    (WajihatRusum::Direct3D12, "d3d12.dll"),
    (WajihatRusum::Direct3D11, "d3d11.dll"),
    // Between the eleventh generation and the ninth, because a Direct3D 10 game
    // presents through the same DXGI swap chain an eleventh-generation one
    // does and is separated from it only by the device that swap chain hands
    // back — which `crate::d3d10` asks for on the first frame.
    (WajihatRusum::Direct3D10, "d3d10.dll"),
    // Below both DXGI generations, because `d3d9.dll` is what a translation
    // layer maps into a process that is really presenting through something
    // else, and because a few newer games ship a Direct3D 9 fallback renderer
    // they never select. Reached only when neither newer generation is loaded,
    // which is exactly when it is the real renderer.
    (WajihatRusum::Direct3D9, "d3d9.dll"),
    // Last of the Direct3D generations. A `d3d8.dll` beside a game is very
    // often a community replacement translating to Direct3D 9 or Vulkan, which
    // loads `d3d9.dll` as well — so this entry is reached only when no newer
    // generation answered, and `crate::d3d8` reports the wrapper as a caveat
    // rather than as a different API.
    (WajihatRusum::Direct3D8, "d3d8.dll"),
    (WajihatRusum::OpenGl, "opengl32.dll"),
];

/// The modules that answer for each graphics API, in the order they are asked.
#[cfg(not(windows))]
const WAHDAT: &[(WajihatRusum, &str)] = &[(WajihatRusum::OpenGl, "libGL.so.1")];

/// The module and symbol the OpenGL buffer swap is detoured at.
#[cfg(windows)]
const RAMZ_TABDIL: (&str, &str) = ("opengl32.dll", "wglSwapBuffers");

/// The module and symbol the OpenGL buffer swap is detoured at.
#[cfg(not(windows))]
const RAMZ_TABDIL: (&str, &str) = ("libGL.so.1", "glXSwapBuffers");

// ---------------------------------------------------------------------------
// The symbol
// ---------------------------------------------------------------------------

/// The entry point `taarib-mudkhal` calls once, after this module is mapped.
///
/// Never panics and never unwinds across the boundary: the whole body is
/// wrapped, and a failure becomes a logged refusal and a return rather than an
/// abort inside somebody's game.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn taarib_bidaya() {
    ibda();
}

/// The entry point `taarib-mudkhal` calls once, after this module is mapped.
///
/// Never panics and never unwinds across the boundary: the whole body is
/// wrapped, and a failure becomes a logged refusal and a return rather than an
/// abort inside somebody's game.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn taarib_bidaya() {
    ibda();
}

/// The body of the entry point, with the unwind boundary around it.
///
/// `extern "system"` and `extern "C"` are the same convention on every target
/// in the matrix except 32-bit Windows, which is why the exported symbol is
/// declared twice and its body written once.
fn ibda() {
    // A second call would install a second set of hooks over the first, and the
    // second unhook would then write one thunk's saved original over the other.
    if DUIYA.swap(true, Ordering::AcqRel) {
        if let Some(mujallad) = mujallad_nafsi() {
            sajjil(&mujallad, "declined: taarib_bidaya was called more than once");
        }
        return;
    }

    // The result is discarded rather than reported: there is nowhere to report
    // it to, the payload's own log is written inside `hayyi`, and a panic that
    // escaped here would abort the player's game.
    let _ = std::panic::catch_unwind(AssertUnwindSafe(hayyi));
}

/// The six steps, in the contract's order.
fn hayyi() {
    // 1. Where am I. Resolved from an address inside this module: `current_exe`
    //    is the game's and the working directory is its launcher's.
    let Some(mujallad) = mujallad_nafsi() else {
        // Nothing can be logged without a directory to log into, and there is
        // no second place to look: a payload that cannot locate its own bytes
        // has nothing beside it either.
        return;
    };

    match tarkib(&mujallad) {
        Ok(satr) => sajjil(&mujallad, &satr),
        Err(radd) => sajjil(&mujallad, &radd.satr()),
    }
}

/// Steps two to five: decide, validate, and install.
///
/// Everything installed lives in one value, so the `?` on any later step drops
/// it and the game is left exactly as it was. On success that value moves into
/// [`TARKIB`] and the handover waits for the first frame.
fn tarkib(mujallad: &Path) -> Result<String, Radd> {
    // 2. What am I inside.
    let Some((wajiha, wahda)) = wajiha_mustakhdama() else {
        return Err(Radd::imtinaa(
            "no graphics module this tier can take over is loaded in this process; the overlay \
             has nothing to attach to and the game is untouched",
        ));
    };

    // 3. The patch.
    let (ruqaa, ism_ruqaa) = ruqaa_mujawira(mujallad)?;

    // 4. The disclosure. Read, never minted: `Iqrar::baad_ard` is given the
    //    fingerprint that was actually recorded against text the user was
    //    shown, and there is no other constructor.
    let luba = ism_luba(mujallad);
    let iqrar = iqrar_mahfuz(&luba)?;

    // 5. Resolve what the adapter needs.
    let mut mabni = Tarkib::jadeed(mujallad.to_path_buf(), wajiha, ruqaa, iqrar);
    if let Err(radd) = rakkib(&mut mabni) {
        taqaad(mabni);
        return Err(radd);
    }

    *TARKIB.lock() = Some(mabni);
    // Published only now: a present arriving between the first hook going live
    // and this assignment must find the start path closed rather than find no
    // installation and conclude the payload had failed.
    JAHIZ.store(true, Ordering::Release);

    Ok(format!(
        "installed: {wajiha} found through {wahda}, patch {ism_ruqaa}, game {luba}; the overlay \
         starts on the game's first frame"
    ))
}

// ---------------------------------------------------------------------------
// Step 2 — what am I inside
// ---------------------------------------------------------------------------

/// The graphics API this process actually uses, and the module that answered.
///
/// Suggestive rather than conclusive, exactly as [`crate::khataf::hal_yumkin`]
/// says: a game can link `d3d11.dll` and render with something else. What
/// confirms it is a live swap chain arriving at the hook, which is the next
/// step and not this one.
fn wajiha_mustakhdama() -> Option<(WajihatRusum, &'static str)> {
    for &(wajiha, wahda) in WAHDAT {
        if qaidat_wahda(wahda).is_some() {
            return Some((wajiha, wahda));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Step 3 — the patch
// ---------------------------------------------------------------------------

/// The patch beside this payload, held open for the session.
///
/// Memory-mapped rather than read: the alternative is a hundred megabytes of
/// atlas on a game's heap at launch, in a process that may be a 32-bit build
/// already short of address space. [`MalafRuqaa`] is what
/// `taarib_ruqaa::qari` documents as the adapter-inside-a-game form, and the
/// mapping lives as long as the installation does.
fn ruqaa_mujawira(mujallad: &Path) -> Result<(MalafRuqaa, String), Radd> {
    let Ok(madakhil) = std::fs::read_dir(mujallad) else {
        return Err(Radd::rafd(format!(
            "{} cannot be listed, so whether a patch is installed cannot be answered",
            mujallad.display()
        )));
    };

    let mut masarat: Vec<PathBuf> = madakhil
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| {
            masar.extension().is_some_and(|lahiqa| lahiqa.eq_ignore_ascii_case(LAHIQAT_RUQAA))
                && masar.is_file()
        })
        .collect();
    // Sorted so that a directory holding more than one patch resolves to the
    // same one on every launch. An arbitrary directory order would mean a game
    // that translated differently between two runs of the same installation.
    masarat.sort();

    let Some(masar) = masarat.first() else {
        return Err(Radd::imtinaa(format!(
            "no patch is installed beside this payload ({}/*.ruqaa), so there is nothing to \
             translate with",
            mujallad.display()
        )));
    };

    let ism = masar
        .file_name()
        .map_or_else(|| masar.display().to_string(), |ism| ism.to_string_lossy().into_owned());
    let malaf = MalafRuqaa::iftah(masar).map_err(|khata| {
        Radd::rafd(format!("{} is present and does not open as a patch: {khata}", masar.display()))
    })?;

    let ziyada = masarat.len().saturating_sub(1);
    let ism = if ziyada == 0 { ism } else { format!("{ism} ({ziyada} more beside it, unused)") };
    Ok((malaf, ism))
}

// ---------------------------------------------------------------------------
// Step 4 — the disclosure
// ---------------------------------------------------------------------------

/// The acknowledgement as the Studio records it.
///
/// Only the two fields this payload needs are named. A record carrying more is
/// read successfully and the rest ignored, because a newer Studio adding a
/// field must not stop an older payload from honouring a consent the user
/// really gave.
#[derive(Debug, serde::Deserialize)]
struct QaydIfsah {
    /// The fingerprint of the disclosure text that was displayed.
    basma: u64,
    /// When it was acknowledged, RFC 3339.
    waqt: String,
}

/// The persisted acknowledgement, validated against the text this build ships.
///
/// This is the one step that must not be routed around. The overlay cannot be
/// started without an [`Iqrar`], an [`Iqrar`] cannot be built without the
/// fingerprint of text that was displayed, and the only fingerprint this
/// function has is the one it genuinely read out of the file the Studio wrote
/// after showing the disclosure. Absent is a decline; a fingerprint that is not
/// this build's is a refusal, because the user agreed to a different set of
/// statements and has to be shown the current ones.
fn iqrar_mahfuz(luba: &str) -> Result<Iqrar, Radd> {
    let masarat = Masarat::iktashif().map_err(|khata| {
        Radd::rafd(format!(
            "Taarib's data directory cannot be resolved from inside this game, so the disclosure \
             acknowledgement cannot be read: {khata}"
        ))
    })?;
    let masar = masarat.jidhr_bayanat().join(ISM_IQRAR_IFSAH);

    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Err(Radd::imtinaa(
                "the tier-3 disclosure has not been acknowledged on this machine; enable the \
                 overlay in Taarib Studio, which shows it in full, and launch the game again",
            ));
        },
        Err(sabab) => {
            return Err(Radd::rafd(format!(
                "{} exists and does not read: {sabab}",
                masar.display()
            )));
        },
    };

    let qayd: QaydIfsah = serde_json::from_slice(&bayt).map_err(|sabab| {
        Radd::rafd(format!(
            "{} is not an acknowledgement this build understands: {sabab}",
            masar.display()
        ))
    })?;

    let basma = BasmatIfsah::min_raqm(qayd.basma);
    if !mahfuz_salih(basma) {
        return Err(Radd::rafd(format!(
            "the recorded acknowledgement {basma} is of an earlier wording of the disclosure, not \
             the one this build shows; acknowledge it again in Taarib Studio"
        )));
    }

    let lahza = qayd.waqt.parse::<jiff::Timestamp>().map_err(|sabab| {
        Radd::rafd(format!(
            "the acknowledgement records \"{}\" as the moment it was given, which is not a \
             timestamp: {sabab}",
            qayd.waqt
        ))
    })?;
    // A record from before the epoch is a machine with a dead clock, not a
    // forgery; the epoch is the reading that makes every later event look later.
    let lahza = u64::try_from(lahza.as_second()).unwrap_or(0);

    Iqrar::baad_ard(basma, lahza, luba).map_err(|khata| {
        Radd::rafd(format!("the recorded acknowledgement was not accepted: {khata}"))
    })
}

/// What to call this game in the acknowledgement and the log.
///
/// The installation manifest that carries the library's own name for a game
/// lives in Taarib's data root, keyed by an identity this process has no way to
/// learn. What a payload does know is the directory it came from, so the game
/// is named by its own folder: `<game>/taarib/` answers `<game>`, and a payload
/// deployed beside the executable answers that directory's name.
fn ism_luba(mujallad: &Path) -> String {
    let ism = mujallad.file_name().map(|ism| ism.to_string_lossy().into_owned());
    match ism {
        Some(ism) if ism == MUJALLAD_TAARIB => mujallad
            .parent()
            .and_then(Path::file_name)
            .map_or_else(|| ism.clone(), |jidhr| jidhr.to_string_lossy().into_owned()),
        Some(ism) => ism,
        None => mujallad.display().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Step 5 — what the adapter needs
// ---------------------------------------------------------------------------

/// Installs the hook that catches the first present.
fn rakkib(mabni: &mut Tarkib) -> Result<(), Radd> {
    match mabni.wajiha {
        #[cfg(windows)]
        WajihatRusum::Direct3D11 | WajihatRusum::Direct3D12 | WajihatRusum::Direct3D10 => {
            rakkib_dxgi(mabni)
        },
        #[cfg(windows)]
        WajihatRusum::Direct3D9 => rakkib_d3d9(mabni),
        #[cfg(windows)]
        WajihatRusum::Direct3D8 => rakkib_d3d8(mabni),
        WajihatRusum::OpenGl => rakkib_gl(mabni),
        wajiha => Err(Radd::fashal(format!(
            "{wajiha} has no bootstrap on this platform, which the module search should have \
             made unreachable"
        ))),
    }
}

/// Detours the platform's buffer swap.
///
/// OpenGL has no method table to write into: the swap is an exported function,
/// so the mechanism is a trampoline detour at its address rather than a vtable
/// slot. [`Masar`] is what `docs/bidaya.md` names for exactly that case.
fn rakkib_gl(mabni: &mut Tarkib) -> Result<(), Radd> {
    let (wahda, ism_ramz) = RAMZ_TABDIL;
    let Some(qaida) = qaidat_wahda(wahda) else {
        return Err(Radd::imtinaa(format!("{wahda} is no longer loaded in this process")));
    };

    // SAFETY: `qaida` is the base `qaidat_wahda` just reported for a module the
    // game itself keeps mapped for as long as it renders, which is the
    // precondition `ramz_wahda` states.
    let Some(hadaf) = (unsafe { ramz_wahda(qaida, ism_ramz) }) else {
        return Err(Radd::rafd(format!("{wahda} does not export {ism_ramz}")));
    };

    // SAFETY: `hadaf` is the entry point `ramz_wahda` resolved for
    // `wglSwapBuffers`/`glXSwapBuffers`; `thunk_tabdil` is a `'static` function
    // in this module declared with that function's exact signature and calling
    // convention for this platform, and it calls the original through
    // `ASL_TABDIL`, which is stored below before the detour can be reached.
    let masar = unsafe {
        Masar::jadeed(hadaf.cast_const(), (thunk_tabdil as *const ()).cast::<c_void>(), ism_ramz)
    }
    .map_err(|khata| Radd::fashal(format!("{ism_ramz} could not be detoured: {khata}")))?;

    ASL_TABDIL.ihfaz(masar.asl().cast_mut());
    mabni.masar = Some(masar);
    Ok(())
}

/// Replaces the swap chain's presentation slots, and D3D12's queue submission.
///
/// There is no API for "give me the vtable of the swap chain this process is
/// using", so the documented technique is the one [`crate::khataf`] describes:
/// create one against a hidden window, read the pointers out of it, and destroy
/// it. Those pointers are the game's, because a vtable belongs to the class and
/// not to the instance.
#[cfg(windows)]
fn rakkib_dxgi(mabni: &mut Tarkib) -> Result<(), Radd> {
    let nafidha = nafidha_muaqqata().map_err(|khata| {
        Radd::rafd(format!(
            "no hidden window could be created to read the swap chain's method table: {khata}"
        ))
    })?;

    // The dummy is made through the generation the game is using rather than
    // through whichever one is convenient. A DXGI swap chain's method table is
    // DXGI's and is almost certainly the same object class either way — but
    // "almost certainly" is not a thing to hook a stranger's game on, and
    // creating it through the right runtime costs one more small function.
    let (silsila, saff) = match mabni.wajiha {
        WajihatRusum::Direct3D12 => silsila_d3d12(&nafidha)?,
        WajihatRusum::Direct3D10 => (silsila_d3d10(&nafidha)?, None),
        _ => (silsila_d3d11(&nafidha)?, None),
    };

    // Built as a local: every `?` below drops it, and the `Drop` on
    // `KhatfJadwal` puts back every slot replaced before the failure.
    let mut jadwal = KhatfJadwal::jadeed("IDXGISwapChain".to_owned());

    // SAFETY: `silsila` is a live COM interface this function just created, so
    // its first machine word is its vtable pointer.
    let lawh = unsafe { jadwal_min_wajiha(silsila.as_raw()) };

    // SAFETY: `lawh` is the vtable of an `IDXGISwapChain` instance, so it has
    // more than `KHANAT_PRESENT` entries; `thunk_present` is a `'static`
    // function with `Present`'s exact signature and calling convention, and it
    // calls the original through `ASL_PRESENT`, which is stored immediately
    // below and before any game thread can reach the slot.
    let asl = unsafe {
        jadwal.ikhtif(
            lawh,
            KHANAT_PRESENT,
            thunk_present as *const () as *mut c_void,
            "IDXGISwapChain::Present",
        )
    }
    .map_err(|khata| Radd::fashal(format!("IDXGISwapChain::Present: {khata}")))?;
    ASL_PRESENT.ihfaz(asl);

    // SAFETY: as above, for `ResizeBuffers` at `KHANAT_RESIZE` and
    // `thunk_taghyeer`, which carries that method's signature and calls the
    // original through `ASL_TAGHYEER`.
    let asl = unsafe {
        jadwal.ikhtif(
            lawh,
            KHANAT_RESIZE,
            thunk_taghyeer as *const () as *mut c_void,
            "IDXGISwapChain::ResizeBuffers",
        )
    }
    .map_err(|khata| Radd::fashal(format!("IDXGISwapChain::ResizeBuffers: {khata}")))?;
    ASL_TAGHYEER.ihfaz(asl);

    // `Present1` only when the object really implements `IDXGISwapChain1`. The
    // slot index is derived from that interface's layout, so hooking it on an
    // object that does not implement it would be writing past the table.
    match silsila.cast::<IDXGISwapChain1>() {
        Ok(silsila1) => {
            // SAFETY: `silsila1` is the same object under an interface whose
            // table has more than `KHANAT_PRESENT1` entries — proved by the
            // `QueryInterface` that produced it — and `thunk_present1` carries
            // `Present1`'s exact signature and calls the original through
            // `ASL_PRESENT1`.
            let asl = unsafe {
                jadwal.ikhtif(
                    jadwal_min_wajiha(silsila1.as_raw()),
                    KHANAT_PRESENT1,
                    thunk_present1 as *const () as *mut c_void,
                    "IDXGISwapChain1::Present1",
                )
            }
            .map_err(|khata| Radd::fashal(format!("IDXGISwapChain1::Present1: {khata}")))?;
            ASL_PRESENT1.ihfaz(asl);
        },
        Err(khata) => {
            // Not fatal: a game presenting through the older interface is
            // caught by `Present`, and saying so is better than refusing.
            sajjil(
                &mabni.mujallad,
                &format!(
                    "note: this DXGI does not offer IDXGISwapChain1, so only Present is hooked \
                     ({khata})"
                ),
            );
        },
    }
    mabni.jadwal = Some(HirasatJadwal(jadwal));

    if let Some(saff) = saff {
        let mut khatf_saff = KhatfJadwal::jadeed("ID3D12CommandQueue".to_owned());
        // SAFETY: `saff` is a live `ID3D12CommandQueue` this function created,
        // whose table has more than `KHANAT_TANFEEDH` entries;
        // `thunk_tanfeedh` carries `ExecuteCommandLists`'s exact signature and
        // calls the original through `ASL_TANFEEDH`, stored below.
        let asl = unsafe {
            khatf_saff.ikhtif(
                jadwal_min_wajiha(saff.as_raw()),
                KHANAT_TANFEEDH,
                thunk_tanfeedh as *const () as *mut c_void,
                "ID3D12CommandQueue::ExecuteCommandLists",
            )
        }
        .map_err(|khata| {
            Radd::fashal(format!("ID3D12CommandQueue::ExecuteCommandLists: {khata}"))
        })?;
        ASL_TANFEEDH.ihfaz(asl);
        mabni.saff = Some(HirasatJadwal(khatf_saff));
    }

    Ok(())
}

/// Replaces the Direct3D 9 device's presentation and reset slots.
///
/// The same technique as [`rakkib_dxgi`] and the same reason: there is no API
/// that hands over the vtable of the device a game is using, so a throwaway one
/// is created against the hidden window, its pointers are read, and it is
/// destroyed. A vtable belongs to the class and not to the instance, so those
/// pointers are the game's.
///
/// Four slots rather than two, and the pairing is what matters. `Present` is
/// where the overlay draws; `Reset` is where the game recovers a **lost
/// device**, and it fails while the overlay holds a state block — so hooking
/// `Present` without `Reset` would make the first alt-tab out of exclusive
/// fullscreen break the game rather than the overlay. `PresentEx` and `ResetEx`
/// are the same two methods on an extended device, hooked in addition rather
/// than instead: a 9Ex device exposes both pairs and a game may call either.
#[cfg(windows)]
fn silsila_d3d10(nafidha: &NafidhaMuaqqata) -> Result<IDXGISwapChain, Radd> {
    use windows::Win32::Graphics::Direct3D10::{
        D3D10_DRIVER_TYPE_HARDWARE, D3D10_SDK_VERSION, D3D10CreateDeviceAndSwapChain, ID3D10Device,
    };

    let wasf = wasf_silsila(nafidha, DXGI_SWAP_EFFECT_DISCARD, 1);
    let mut silsila: Option<IDXGISwapChain> = None;
    let mut jihaz: Option<ID3D10Device> = None;

    // SAFETY: every pointer argument addresses a local that outlives the call;
    // the description names the hidden window this function was handed, which
    // is alive for the whole call; and the flag word is zero, which asks for no
    // debug layer — that is not installed on a player's machine.
    unsafe {
        D3D10CreateDeviceAndSwapChain(
            None,
            D3D10_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            0,
            D3D10_SDK_VERSION,
            Some(&raw const wasf),
            Some(&raw mut silsila),
            Some(&raw mut jihaz),
        )
    }
    .map_err(|khata| {
        Radd::rafd(format!(
            "a throwaway D3D10 swap chain could not be created to read its method table: {khata}"
        ))
    })?;

    silsila.ok_or_else(|| {
        Radd::fashal("D3D10 reported success and produced no swap chain".to_owned())
    })
}

/// Replaces `Present` and `Reset` in a Direct3D 8 device's method table.
///
/// The capability report is written to the log **before** the throwaway device
/// is made, so a machine where nothing can be hooked says why in the file the
/// player can read rather than only refusing.
///
/// `crate::d3d8::TarkeebD3D8::rakkib` verifies the ninety-six-slot table
/// against that device before it writes a single slot, which is the one thing
/// this path does that none of the others needs: nothing binds Direct3D 8, so
/// the table is hand-transcribed and is checked rather than trusted.
#[cfg(windows)]
fn rakkib_d3d8(mabni: &mut Tarkib) -> Result<(), Radd> {
    for satr in crate::d3d8::qudra().sutur() {
        sajjil(&mabni.mujallad, &format!("capability: {satr}"));
    }

    let muaqqat = crate::d3d8::jihaz_muaqqat().map_err(|khata| {
        Radd::rafd(format!(
            "no throwaway Direct3D 8 device could be created to read its method table: {khata}"
        ))
    })?;

    // SAFETY: `muaqqat` holds a live `IDirect3DDevice8` it created and owns for
    // the whole of this call, so its method table — which belongs to the class
    // and not to the instance, and is therefore the game's own — is live.
    // `nida_taqdeem_8` and `nida_tasfir_8` are `fn` items in this module and
    // outlive any installation.
    let tarkeeb = unsafe {
        TarkeebD3D8::rakkib(muaqqat.jihaz(), nida_taqdeem_8, nida_tasfir_8)
    }
    .map_err(|khata| Radd::fashal(format!("IDirect3DDevice8: {khata}")))?;

    mabni.thabit8 = Some(HirasatD3D8(tarkeeb));
    Ok(())
}

/// What `crate::d3d8`'s `Present` thunk calls when a frame reaches it.
#[cfg(windows)]
fn nida_taqdeem_8(jihaz: *mut c_void) {
    shaghghil_min_itar(MasdarKhalfiya::D3D8(jihaz));
}

/// What `crate::d3d8`'s `Reset` thunk calls before the game's own reset.
///
/// A `D3DSBT_ALL` state block does not survive a device reset, and the only
/// moment it can be released is before the call reaches the runtime — which is
/// the same shape as the DXGI `ResizeBuffers` hook above and reaches the
/// backend through the same trait method.
#[cfg(windows)]
fn nida_tasfir_8() {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // `try_lock`: this runs on a render thread, and blocking it behind a
        // start attempt on another thread would be a visible stall in the game.
        if let Some(mut hirasa) = TARKIB.try_lock()
            && let Some(tabaqa) = hirasa.as_mut().and_then(|mabni| mabni.tabaqa.as_mut())
        {
            let _ = tabaqa.qabl_taghyeer_hajm();
        }
    }));
}

#[cfg(windows)]
fn rakkib_d3d9(mabni: &mut Tarkib) -> Result<(), Radd> {
    let nafidha = nafidha_muaqqata().map_err(|khata| {
        Radd::rafd(format!(
            "no hidden window could be created to read the Direct3D 9 device's method table: \
             {khata}"
        ))
    })?;

    let jihaz = jihaz_d3d9(&nafidha)?;

    // Built as a local: every `?` below drops it, and the `Drop` on
    // `KhatfJadwal` puts back every slot replaced before the failure.
    let mut jadwal = KhatfJadwal::jadeed("IDirect3DDevice9".to_owned());

    // SAFETY: `jihaz` is a live COM interface this function just created, so its
    // first machine word is its vtable pointer.
    let lawh = unsafe { jadwal_min_wajiha(jihaz.as_raw()) };

    // SAFETY: `lawh` is the vtable of an `IDirect3DDevice9` instance, which has
    // more than `KHANAT_TAQDEEM9` entries; `thunk_taqdeem9` is a `'static`
    // function carrying `Present`'s exact signature and calling convention, and
    // it calls the original through `ASL_TAQDEEM9`, which is stored immediately
    // below and before any game thread can reach the slot.
    let asl = unsafe {
        jadwal.ikhtif(
            lawh,
            KHANAT_TAQDEEM9,
            thunk_taqdeem9 as *const () as *mut c_void,
            "IDirect3DDevice9::Present",
        )
    }
    .map_err(|khata| Radd::fashal(format!("IDirect3DDevice9::Present: {khata}")))?;
    ASL_TAQDEEM9.ihfaz(asl);

    // SAFETY: as above, for `Reset` at `KHANAT_ISTIAADA` and `thunk_istiaada9`,
    // which carries that method's signature and calls the original through
    // `ASL_ISTIAADA9`.
    let asl = unsafe {
        jadwal.ikhtif(
            lawh,
            KHANAT_ISTIAADA,
            thunk_istiaada9 as *const () as *mut c_void,
            "IDirect3DDevice9::Reset",
        )
    }
    .map_err(|khata| Radd::fashal(format!("IDirect3DDevice9::Reset: {khata}")))?;
    ASL_ISTIAADA9.ihfaz(asl);

    // The extended pair only when the object really implements
    // `IDirect3DDevice9Ex`. Those slot indices are derived from that interface's
    // layout, so writing them on a device that does not implement it would be
    // writing seventeen entries past the end of the table.
    match jihaz.cast::<IDirect3DDevice9Ex>() {
        Ok(mumtadd) => {
            // SAFETY: `mumtadd` is the same object under an interface whose
            // table has more than `KHANAT_ISTIAADA_MUMTADDA` entries — proved by
            // the `QueryInterface` that produced it — and both thunks carry
            // their methods' exact signatures.
            let lawh_mumtadd = unsafe { jadwal_min_wajiha(mumtadd.as_raw()) };
            // SAFETY: as above, for `PresentEx`.
            let asl = unsafe {
                jadwal.ikhtif(
                    lawh_mumtadd,
                    KHANAT_TAQDEEM_MUMTADD,
                    thunk_taqdeem_mumtadd as *const () as *mut c_void,
                    "IDirect3DDevice9Ex::PresentEx",
                )
            }
            .map_err(|khata| Radd::fashal(format!("IDirect3DDevice9Ex::PresentEx: {khata}")))?;
            ASL_TAQDEEM_MUMTADD.ihfaz(asl);

            // SAFETY: as above, for `ResetEx`.
            let asl = unsafe {
                jadwal.ikhtif(
                    lawh_mumtadd,
                    KHANAT_ISTIAADA_MUMTADDA,
                    thunk_istiaada_mumtadda as *const () as *mut c_void,
                    "IDirect3DDevice9Ex::ResetEx",
                )
            }
            .map_err(|khata| Radd::fashal(format!("IDirect3DDevice9Ex::ResetEx: {khata}")))?;
            ASL_ISTIAADA_MUMTADDA.ihfaz(asl);
        },
        Err(khata) => {
            // Not fatal, and the common case: a game built against the original
            // Direct3D 9 runtime has no extended interface and is caught by the
            // two slots already hooked.
            sajjil(
                &mabni.mujallad,
                &format!(
                    "note: this Direct3D 9 is not extended, so only Present and Reset are \
                     hooked ({khata})"
                ),
            );
        },
    }

    mabni.jadwal = Some(HirasatJadwal(jadwal));
    Ok(())
}

/// A throwaway Direct3D 9 device, for its method table alone.
///
/// Three of the four behaviour flags are defensive rather than functional, and
/// each of them is a way this function could otherwise damage the game it is
/// running inside.
///
/// `D3DCREATE_FPU_PRESERVE` is the sharp one. Creating a Direct3D 9 device
/// without it switches the process's x87 control word to single precision, for
/// the whole process and for every thread — so a game doing its own physics in
/// double precision would start producing different numbers because Taarib
/// created a device it immediately threw away. It is the single most damaging
/// side effect anything in this crate can have on a game, and it is one flag.
///
/// `D3DCREATE_NOWINDOWCHANGES` stops the runtime from moving or resizing the
/// window it is given, which here is the hidden one and would be harmless — but
/// the flag costs nothing and the failure it prevents is invisible.
///
/// `D3DCREATE_SOFTWARE_VERTEXPROCESSING` because this device never draws, and
/// asking for hardware vertex processing is asking a driver for a resource on a
/// card the game is already using.
#[cfg(windows)]
fn jihaz_d3d9(nafidha: &NafidhaMuaqqata) -> Result<IDirect3DDevice9, Radd> {
    // SAFETY: `Direct3DCreate9` takes the SDK version and nothing else, and
    // answers `None` rather than failing when the runtime is not present.
    let Some(tisaa) = (unsafe { Direct3DCreate9(D3D_SDK_VERSION) }) else {
        return Err(Radd::rafd(
            "Direct3DCreate9 produced no interface, so this process has no usable Direct3D 9 \
             runtime to read a method table from"
                .to_owned(),
        ));
    };

    let mut muallimat = D3DPRESENT_PARAMETERS {
        BackBufferWidth: 8,
        BackBufferHeight: 8,
        // Unknown is permitted windowed and asks the runtime for the desktop's
        // own format, which is the one format guaranteed to be creatable.
        BackBufferFormat: D3DFMT_UNKNOWN,
        BackBufferCount: 1,
        SwapEffect: D3DSWAPEFFECT_DISCARD,
        hDeviceWindow: nafidha.maqbad(),
        Windowed: BOOL::from(true),
        ..Default::default()
    };

    let mut jihaz: Option<IDirect3DDevice9> = None;
    let aalam = (D3DCREATE_SOFTWARE_VERTEXPROCESSING
        | D3DCREATE_FPU_PRESERVE
        | D3DCREATE_NOWINDOWCHANGES
        | D3DCREATE_MULTITHREADED)
        .cast_unsigned();
    // SAFETY: `tisaa` is the live interface from above, the presentation
    // parameters are a fully initialised local naming the hidden window this
    // function was handed, and the out-parameter addresses a local `None`.
    unsafe {
        tisaa.CreateDevice(
            D3DADAPTER_DEFAULT,
            D3DDEVTYPE_HAL,
            nafidha.maqbad(),
            aalam,
            &raw mut muallimat,
            &raw mut jihaz,
        )
    }
    .map_err(|khata| {
        Radd::rafd(format!(
            "a throwaway Direct3D 9 device could not be created to read its method table: {khata}"
        ))
    })?;

    jihaz.ok_or_else(|| {
        Radd::fashal("Direct3D 9 reported success and produced no device".to_owned())
    })
}

/// The throwaway swap chain description both dummies are built from.
#[cfg(windows)]
fn wasf_silsila(
    nafidha: &NafidhaMuaqqata,
    athar_tabdil: DXGI_SWAP_EFFECT,
    hawajiz: u32,
) -> DXGI_SWAP_CHAIN_DESC {
    DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: 8,
            Height: 8,
            RefreshRate: DXGI_RATIONAL { Numerator: 0, Denominator: 1 },
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
            Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
        },
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: hawajiz,
        OutputWindow: nafidha.maqbad(),
        Windowed: BOOL::from(true),
        SwapEffect: athar_tabdil,
        Flags: 0,
    }
}

/// A throwaway D3D11 device and swap chain, for their method table alone.
#[cfg(windows)]
fn silsila_d3d11(nafidha: &NafidhaMuaqqata) -> Result<IDXGISwapChain, Radd> {
    let wasf = wasf_silsila(nafidha, DXGI_SWAP_EFFECT_DISCARD, 1);
    let mut silsila: Option<IDXGISwapChain> = None;
    let mut jihaz: Option<ID3D11Device> = None;

    // SAFETY: every pointer argument addresses a local that outlives the call;
    // the description names the hidden window this function was handed, which
    // is alive for the whole call; and no flag here asks for the debug layer,
    // which is not installed on a player's machine.
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0]),
            D3D11_SDK_VERSION,
            Some(&raw const wasf),
            Some(&raw mut silsila),
            Some(&raw mut jihaz),
            None,
            None,
        )
    }
    .map_err(|khata| {
        Radd::rafd(format!(
            "a throwaway D3D11 swap chain could not be created to read its method table: {khata}"
        ))
    })?;

    silsila.ok_or_else(|| {
        Radd::fashal("D3D11 reported success and produced no swap chain".to_owned())
    })
}

/// A throwaway D3D12 device, queue and swap chain, for their method tables.
///
/// The queue is returned as well as the swap chain because D3D12 needs two
/// tables: there is no API that answers "which queue presents this swap chain",
/// so the game's own queue has to be captured as it submits.
#[cfg(windows)]
fn silsila_d3d12(
    nafidha: &NafidhaMuaqqata,
) -> Result<(IDXGISwapChain, Option<ID3D12CommandQueue>), Radd> {
    let mut jihaz: Option<ID3D12Device> = None;
    // SAFETY: the out-parameter addresses a local initialised to `None`, and
    // `D3D12CreateDevice` writes through it only on success.
    unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &raw mut jihaz) }.map_err(
        |khata| Radd::rafd(format!("a throwaway D3D12 device could not be created: {khata}")),
    )?;
    let Some(jihaz) = jihaz else {
        return Err(Radd::fashal("D3D12 reported success and named no device".to_owned()));
    };

    let wasf_saff = D3D12_COMMAND_QUEUE_DESC {
        Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
        Priority: 0,
        Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
        NodeMask: 0,
    };
    // SAFETY: `jihaz` is the live device from above and the description is a
    // fully initialised local that outlives the call.
    let saff: ID3D12CommandQueue = unsafe { jihaz.CreateCommandQueue(&raw const wasf_saff) }
        .map_err(|khata| {
            Radd::rafd(format!("a throwaway D3D12 command queue could not be created: {khata}"))
        })?;

    // SAFETY: `CreateDXGIFactory1` takes no arguments and returns an owned
    // interface or an error.
    let masna: IDXGIFactory = unsafe { CreateDXGIFactory1() }
        .map_err(|khata| Radd::rafd(format!("no DXGI factory could be created: {khata}")))?;

    // The flip model, because D3D12 accepts nothing else, and two buffers
    // because the flip model requires at least two.
    let wasf = wasf_silsila(nafidha, DXGI_SWAP_EFFECT_FLIP_DISCARD, 2);
    let mut silsila: Option<IDXGISwapChain> = None;
    // SAFETY: the queue is live, the description addresses a local that
    // outlives the call, and the out-parameter addresses a local `None`.
    unsafe { masna.CreateSwapChain(&saff, &raw const wasf, &raw mut silsila) }
        .ok()
        .map_err(|khata| {
            Radd::rafd(format!(
                "a throwaway D3D12 swap chain could not be created to read its method table: \
                 {khata}"
            ))
        })?;
    let Some(silsila) = silsila else {
        return Err(Radd::fashal("DXGI reported success and produced no swap chain".to_owned()));
    };

    // Refused here rather than at the first frame: the D3D12 backend is built
    // from an `IDXGISwapChain3`, and a DXGI that cannot produce one is a
    // machine the overlay cannot draw on whatever else happens.
    silsila.cast::<IDXGISwapChain3>().map_err(|khata| {
        Radd::rafd(format!(
            "this DXGI does not offer IDXGISwapChain3, which the D3D12 backend needs: {khata}"
        ))
    })?;

    Ok((silsila, Some(saff)))
}

// ---------------------------------------------------------------------------
// Step 6 — the handover, from inside the first frame
// ---------------------------------------------------------------------------

/// Which hook the start attempt came in through.
#[derive(Debug, Clone, Copy)]
enum MasdarKhalfiya {
    /// The `this` pointer of the swap chain the game is presenting.
    #[cfg(windows)]
    Dxgi(*mut c_void),
    /// The `this` pointer of the Direct3D 9 device the game is presenting on.
    ///
    /// A device rather than a swap chain, because Direct3D 9 predates DXGI: the
    /// implicit swap chain is reached *through* the device and there is no
    /// separate object for a hook to sit on.
    #[cfg(windows)]
    D3D9(*mut c_void),
    /// The `this` pointer of the Direct3D 8 device the game is presenting on.
    ///
    /// A device for the same reason Direct3D 9's is one, and one generation
    /// earlier: `Present` is method fifteen of `IDirect3DDevice8` and there is
    /// no swap chain interface between the game and the screen at all.
    #[cfg(windows)]
    D3D8(*mut c_void),
    /// An OpenGL context, current on the thread that is swapping buffers.
    Gl,
}

/// What one start attempt concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nateeja {
    /// The overlay has control.
    Bada,
    /// Not yet possible, and worth trying again on the next frame.
    Intizar,
    /// It will not happen this session.
    Radd,
}

/// Runs at most one start attempt at a time, and at most one that concludes.
fn shaghghil_min_itar(masdar: MasdarKhalfiya) {
    if !JAHIZ.load(Ordering::Acquire) || INTAHAT.load(Ordering::Acquire) {
        return;
    }
    // A game with two presenting threads would otherwise run two attempts
    // against one acknowledgement, and the second would find it already spent.
    if MUNSHAGHIL.swap(true, Ordering::AcqRel) {
        return;
    }

    // A panic inside the start path settles the question rather than being
    // retried: the next frame would panic in the same place, and unwinding out
    // of an `extern "system"` thunk aborts the player's game.
    if std::panic::catch_unwind(AssertUnwindSafe(|| ihdar(masdar))).is_err() {
        INTAHAT.store(true, Ordering::Release);
        let mahjuz = TARKIB.lock().take();
        if let Some(mabni) = mahjuz {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| taqaad(mabni)));
        }
    }

    MUNSHAGHIL.store(false, Ordering::Release);
}

/// One start attempt, and what its outcome does to the installation.
fn ihdar(masdar: MasdarKhalfiya) {
    match jarrib_tashghil(masdar) {
        Nateeja::Bada => INTAHAT.store(true, Ordering::Release),
        // The queue has not been submitted on yet; the next frame tries again.
        Nateeja::Intizar => {},
        Nateeja::Radd => {
            INTAHAT.store(true, Ordering::Release);
            let mahjuz = TARKIB.lock().take();
            if let Some(mabni) = mahjuz {
                taqaad(mabni);
            }
        },
    }
}

/// Builds the backend and hands it to [`Tabaqa::shaghghil`].
fn jarrib_tashghil(masdar: MasdarKhalfiya) -> Nateeja {
    let mut hirasa = TARKIB.lock();
    let Some(mabni) = hirasa.as_mut() else {
        return Nateeja::Radd;
    };
    let mujallad = mabni.mujallad.clone();

    // A translated Direct3D — DXVK, vkd3d — presents through a `d3d11.dll` that
    // is Vulkan underneath, and Taarib's own layer is already on that queue.
    // Two overlays on one presentation path is one overlay too many, and the
    // layer is the better of the two: it is enumerated by the loader and
    // removed by deleting a manifest.
    if !crate::vulkan::ajhiza().is_empty() {
        sajjil(
            &mujallad,
            &Radd::imtinaa(
                "the Vulkan layer is already live in this process, so this path would be a \
                 second overlay on the same queue; the layer keeps it",
            )
            .satr(),
        );
        return Nateeja::Radd;
    }

    let natija = match masdar {
        #[cfg(windows)]
        MasdarKhalfiya::Dxgi(silsila) => match mabni.wajiha {
            WajihatRusum::Direct3D12 => khattaf_d3d12(silsila),
            WajihatRusum::Direct3D10 => khattaf_d3d10(&mujallad, silsila),
            _ => khattaf_d3d11(silsila),
        },
        #[cfg(windows)]
        MasdarKhalfiya::D3D9(jihaz) => khattaf_d3d9(&mujallad, jihaz),
        #[cfg(windows)]
        MasdarKhalfiya::D3D8(jihaz) => khattaf_d3d8(&mujallad, jihaz),
        MasdarKhalfiya::Gl => khattaf_gl(),
    };

    let khattaf = match natija {
        Ok(Some(khattaf)) => khattaf,
        Ok(None) => {
            if !NUBBIHA.swap(true, Ordering::AcqRel) {
                sajjil(
                    &mujallad,
                    "waiting: the game has presented a frame and has not yet submitted on a \
                     direct command queue, which D3D12 needs before a backend can exist",
                );
            }
            return Nateeja::Intizar;
        },
        Err(radd) => {
            sajjil(&mujallad, &radd.satr());
            return Nateeja::Radd;
        },
    };

    let Some(iqrar) = mabni.iqrar.take() else {
        sajjil(
            &mujallad,
            &Radd::fashal("the acknowledgement was already spent by an earlier attempt").satr(),
        );
        return Nateeja::Radd;
    };

    match Tabaqa::shaghghil(khattaf, iqrar) {
        Ok(tabaqa) => {
            for satr in tabaqa.athar() {
                sajjil(&mujallad, &format!("started: {satr}"));
            }
            mabni.tabaqa = Some(tabaqa);
            Nateeja::Bada
        },
        Err(khata) => {
            let radd = Radd::fashal(format!("the overlay refused to start: {khata}"));
            sajjil(&mujallad, &radd.satr());
            Nateeja::Radd
        },
    }
}

/// The Direct3D 11 backend, over the swap chain the game just presented.
#[cfg(windows)]
fn khattaf_d3d11(silsila: *mut c_void) -> Result<Option<Box<dyn Khattaf>>, Radd> {
    // SAFETY: `silsila` is the `this` of an `IDXGISwapChain` method call in
    // progress, so it points at a live instance the game owns for at least the
    // duration of that call, and the borrow taken here does not outlive it.
    let Some(wajiha) = (unsafe { IDXGISwapChain::from_raw_borrowed(&silsila) }) else {
        return Err(Radd::fashal("the present hook was reached with a null swap chain"));
    };
    let khattaf = KhattafD3D11::min_silsila(wajiha)
        .map_err(|khata| Radd::fashal(format!("the D3D11 backend could not be built: {khata}")))?;
    Ok(Some(Box::new(khattaf)))
}

/// The Direct3D 12 backend, once the game's command queue has been seen.
#[cfg(windows)]
fn khattaf_d3d12(silsila: *mut c_void) -> Result<Option<Box<dyn Khattaf>>, Radd> {
    // SAFETY: as `khattaf_d3d11` — the `this` of a method call in progress.
    let Some(wajiha) = (unsafe { IDXGISwapChain::from_raw_borrowed(&silsila) }) else {
        return Err(Radd::fashal("the present hook was reached with a null swap chain"));
    };
    let silsila3 = wajiha.cast::<IDXGISwapChain3>().map_err(|khata| {
        Radd::fashal(format!("the game's swap chain is not an IDXGISwapChain3: {khata}"))
    })?;

    let hirasa = SAFF.lock();
    let Some(saff) = hirasa.as_ref() else {
        return Ok(None);
    };
    let khattaf = KhattafD3D12::min_silsila_wa_saff(&silsila3, saff)
        .map_err(|khata| Radd::fashal(format!("the D3D12 backend could not be built: {khata}")))?;
    Ok(Some(Box::new(khattaf)))
}

/// The Direct3D 9 backend, over the device the game just presented on.
///
/// The capability report is produced here rather than at the first draw and is
/// written into the payload's log before anything else happens, because half of
/// what it says — whether the device can be lost, whether the backbuffer can be
/// read for recognition — is knowable only from a live device and is exactly
/// what a player would otherwise experience as an overlay that draws nothing.
#[cfg(windows)]
fn khattaf_d3d9(mujallad: &Path, jihaz: *mut c_void) -> Result<Option<Box<dyn Khattaf>>, Radd> {
    // SAFETY: `jihaz` is the `this` of an `IDirect3DDevice9` method call in
    // progress, so it points at a live device the game owns for at least the
    // duration of that call, and the borrow taken here does not outlive it.
    let Some(wajiha) = (unsafe { IDirect3DDevice9::from_raw_borrowed(&jihaz) }) else {
        return Err(Radd::fashal("the present hook was reached with a null device"));
    };
    let khattaf = KhattafD3D9::min_jihaz(wajiha)
        .map_err(|khata| Radd::fashal(format!("the D3D9 backend could not be built: {khata}")))?;

    match khattaf.qudra() {
        Ok(taqrir) => {
            for satr in taqrir.sutur() {
                sajjil(mujallad, &format!("capability: {satr}"));
            }
        },
        // A device that will not describe itself right now is a device that will
        // describe itself on a later frame. The overlay still starts; the report
        // is what is missing, and saying so is better than refusing over it.
        Err(khata) => sajjil(
            mujallad,
            &format!("capability: the Direct3D 9 device would not describe itself yet: {khata}"),
        ),
    }

    Ok(Some(Box::new(khattaf)))
}

/// The Direct3D 10 backend over the swap chain the game just presented — or the
/// Direct3D 11 backend, when the device says that is what this game is.
///
/// The module search names Direct3D 10 whenever `d3d10.dll` is mapped and no
/// newer generation's module is, which is also what an eleventh-generation game
/// looks like while its `d3d11.dll` is still delay-loaded and Direct2D has
/// already brought `d3d10.dll` in. The two generations share one DXGI hook, so
/// the swap chain that reached it is asked which device it has, and the answer
/// picks the backend. Three outcomes, kept apart on purpose:
///
/// * a Direct3D 10 device — this backend;
/// * a Direct3D 11 device — [`khattaf_d3d11`] over the same swap chain, logged
///   as the capability it is, because nothing went wrong;
/// * neither — a *decline*, not a failure. A twelfth-generation game whose
///   module was not loaded when the search ran needs the command-queue hook
///   this session never installed, and no retry on this session changes that.
///
/// What this must never do is what it once did: report the second and third
/// as "the D3D10 backend could not be built", which unhooked everything and gave
/// up for the process lifetime over a game the eleventh backend would have
/// drawn on with the hook already in place.
#[cfg(windows)]
fn khattaf_d3d10(
    mujallad: &Path,
    silsila: *mut c_void,
) -> Result<Option<Box<dyn Khattaf>>, Radd> {
    // SAFETY: `silsila` is the `this` of an `IDXGISwapChain` method call in
    // progress, so it points at a live instance the game owns for at least the
    // duration of that call, and the borrow taken here does not outlive it.
    let Some(wajiha) = (unsafe { IDXGISwapChain::from_raw_borrowed(&silsila) }) else {
        return Err(Radd::fashal("the present hook was reached with a null swap chain"));
    };
    for satr in crate::d3d10::qudra().sutur() {
        sajjil(mujallad, &format!("capability: {satr}"));
    }
    let imtinaa = match KhattafD3D10::min_silsila(wajiha) {
        Ok(khattaf) => return Ok(Some(Box::new(khattaf))),
        Err(khata @ crate::khata::KhataTabaqa::ApiGhayrMadum { .. }) => khata,
        Err(khata) => {
            return Err(Radd::fashal(format!("the D3D10 backend could not be built: {khata}")));
        },
    };
    sajjil(mujallad, &format!("capability: Direct3D 10: declined — {imtinaa}"));

    // SAFETY: `wajiha` is the live swap chain borrowed above, for the duration
    // of the same call. `GetDevice` is a QueryInterface for the requested IID
    // and returns an owned reference or an error; nothing is written through a
    // raw pointer, and the reference is dropped here.
    if unsafe { wajiha.GetDevice::<ID3D11Device>() }.is_ok() {
        sajjil(
            mujallad,
            "capability: Direct3D 11: the swap chain's device is an ID3D11Device, so the \
             eleventh backend takes this game over the hook already in place",
        );
        return khattaf_d3d11(silsila);
    }
    Err(Radd::imtinaa(
        "this swap chain's device is neither an ID3D10Device nor an ID3D11Device. It is most \
         likely a Direct3D 12 game whose d3d12.dll was not yet loaded when the module search \
         ran; that generation needs the command-queue hook, which this session did not \
         install. Nothing failed, and nothing on this session can draw on it",
    ))
}

/// The Direct3D 8 backend, over the device the game just presented on.
///
/// The backend's own trace lines go into the log the moment it exists, because
/// two of them are facts a player needs and nothing else reports: whether the
/// device is a *pure* one, on which no state may be read back, and whether the
/// `d3d8.dll` answering is Microsoft's or a community replacement.
#[cfg(windows)]
fn khattaf_d3d8(mujallad: &Path, jihaz: *mut c_void) -> Result<Option<Box<dyn Khattaf>>, Radd> {
    // SAFETY: `jihaz` is the `this` of an `IDirect3DDevice8` method call in
    // progress, so it points at a live device the game owns for at least the
    // duration of that call. `min_jihaz` takes a reference of its own before it
    // keeps the pointer past that call.
    let khattaf = unsafe { KhattafD3D8::min_jihaz(jihaz) }
        .map_err(|khata| Radd::fashal(format!("the D3D8 backend could not be built: {khata}")))?;
    for satr in khattaf.athar() {
        sajjil(mujallad, &format!("capability: {satr}"));
    }
    Ok(Some(Box::new(khattaf)))
}

/// The OpenGL backend for whichever kind of context this game has.
///
/// Two backends answer for OpenGL and the module list cannot separate them:
/// every OpenGL process loads the same module whether its context is a 4.6 core
/// profile or a 1.1 compatibility one. `crate::gl_thabit::ikhtar` asks the
/// context instead, prefers the modern backend on 3.0 and later, and falls back
/// to the fixed-function one when that refuses or when the context is older.
fn khattaf_gl() -> Result<Option<Box<dyn Khattaf>>, Radd> {
    let khattaf = crate::gl_thabit::ikhtar()
        .map_err(|khata| Radd::fashal(format!("the OpenGL backend could not be built: {khata}")))?;
    Ok(Some(khattaf))
}

// ---------------------------------------------------------------------------
// The thunks
// ---------------------------------------------------------------------------

/// `IDXGISwapChain::Present`, as a callable pointer.
///
/// Written as an alias so the `transmute` that produces one names both of its
/// types outright. A transmute whose target is inferred from a binding is a
/// transmute that changes meaning when the binding does.
#[cfg(windows)]
type DallatTaqdeem = unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT;

/// `IDXGISwapChain1::Present1`, as a callable pointer.
#[cfg(windows)]
type DallatTaqdeem1 =
    unsafe extern "system" fn(*mut c_void, u32, u32, *const DXGI_PRESENT_PARAMETERS) -> HRESULT;

/// `IDXGISwapChain::ResizeBuffers`, as a callable pointer.
#[cfg(windows)]
type DallatTaghyeer =
    unsafe extern "system" fn(*mut c_void, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

/// `ID3D12CommandQueue::ExecuteCommandLists`, as a callable pointer.
#[cfg(windows)]
type DallatTanfeedh = unsafe extern "system" fn(*mut c_void, u32, *const *mut c_void);

/// `wglSwapBuffers`, as a callable pointer.
#[cfg(windows)]
type DallatTabdil = unsafe extern "system" fn(*mut c_void) -> BOOL;

/// `glXSwapBuffers`, as a callable pointer.
#[cfg(not(windows))]
type DallatTabdil = unsafe extern "C" fn(*mut c_void, core::ffi::c_ulong);

/// `IDXGISwapChain::Present`, which is where the overlay first becomes possible.
#[cfg(windows)]
unsafe extern "system" fn thunk_present(silsila: *mut c_void, fasil: u32, alam: u32) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    shaghghil_min_itar(MasdarKhalfiya::Dxgi(silsila));

    let asl = ASL_PRESENT.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: `asl` is the non-null pointer this module read out of the slot it
    // replaced, so it is `Present`'s own entry point with this exact signature
    // and calling convention, and the module owning it is still mapped because
    // the game is inside a call to it.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTaqdeem>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified.
    unsafe { asl(silsila, fasil, alam) }
}

/// `IDXGISwapChain1::Present1`.
#[cfg(windows)]
unsafe extern "system" fn thunk_present1(
    silsila: *mut c_void,
    fasil: u32,
    alam: u32,
    mualimat: *const DXGI_PRESENT_PARAMETERS,
) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    shaghghil_min_itar(MasdarKhalfiya::Dxgi(silsila));

    let asl = ASL_PRESENT1.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: as `thunk_present`, for `Present1`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTaqdeem1>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified, including
    // its own present-parameters pointer, which this module never reads.
    unsafe { asl(silsila, fasil, alam, mualimat) }
}

/// `IDXGISwapChain::ResizeBuffers`, hooked so the resize can succeed.
///
/// `ResizeBuffers` fails with `DXGI_ERROR_INVALID_CALL` while anybody holds a
/// reference to a backbuffer, and both Direct3D backends hold one for the whole
/// session. Without this the first resolution change in a game with the overlay
/// installed would fail a call that has never once failed it.
#[cfg(windows)]
unsafe extern "system" fn thunk_taghyeer(
    silsila: *mut c_void,
    adad: u32,
    ard: u32,
    irtifa: u32,
    sigha: DXGI_FORMAT,
    alam: u32,
) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // `try_lock`: this runs on a render thread, and blocking it behind a
        // start attempt on another thread would be a visible stall in the game.
        if let Some(mut hirasa) = TARKIB.try_lock()
            && let Some(tabaqa) = hirasa.as_mut().and_then(|mabni| mabni.tabaqa.as_mut())
        {
            let _ = tabaqa.qabl_taghyeer_hajm();
        }
    }));

    let asl = ASL_TAGHYEER.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: as `thunk_present`, for `ResizeBuffers`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTaghyeer>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified.
    unsafe { asl(silsila, adad, ard, irtifa, sigha, alam) }
}

/// `ID3D12CommandQueue::ExecuteCommandLists`, hooked to learn the queue.
///
/// D3D12 has no way to get the command queue from a swap chain, and the overlay
/// cannot submit its own command list without one. Capturing the first direct
/// queue the game submits on is the documented technique and the only one
/// available.
#[cfg(windows)]
unsafe extern "system" fn thunk_tanfeedh(
    saff: *mut c_void,
    adad: u32,
    qawaim: *const *mut c_void,
) {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    if !SAFF_JAHIZ.load(Ordering::Acquire) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: `saff` is the `this` of an `ID3D12CommandQueue` method
            // call in progress, so it points at a live queue the game owns for
            // at least the duration of that call.
            if let Some(wajiha) = unsafe { ID3D12CommandQueue::from_raw_borrowed(&saff) } {
                // SAFETY: `wajiha` is that live queue; `GetDesc` fills a
                // description the binding owns on the stack.
                let wasf = unsafe { wajiha.GetDesc() };
                // Copy and compute queues never present. A backend submitting
                // on one would draw into a buffer nobody displays.
                if wasf.Type == D3D12_COMMAND_LIST_TYPE_DIRECT
                    && let Some(mut hirasa) = SAFF.try_lock()
                    && hirasa.is_none()
                {
                    // Cloned rather than remembered as an address: the
                    // reference this takes is what keeps the queue alive
                    // between here and the next present.
                    *hirasa = Some(wajiha.clone());
                    SAFF_JAHIZ.store(true, Ordering::Release);
                }
            }
        }));
    }

    let asl = ASL_TANFEEDH.iqra();
    if asl.is_null() {
        return;
    }
    // SAFETY: as `thunk_present`, for `ExecuteCommandLists`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTanfeedh>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified, including
    // its own command-list array, which this module never reads.
    unsafe { asl(saff, adad, qawaim) };
}

/// `IDirect3DDevice9::Present`, as a callable pointer.
///
/// Five parameters, and every one of them is passed through untouched. The two
/// rectangles and the region are the game's own partial-presentation request;
/// the window override is how a game presents into a window other than the one
/// the device was created against. This module reads none of them.
#[cfg(windows)]
type DallatTaqdeem9 = unsafe extern "system" fn(
    *mut c_void,
    *const RECT,
    *const RECT,
    HWND,
    *const c_void,
) -> HRESULT;

/// `IDirect3DDevice9Ex::PresentEx`, as a callable pointer.
///
/// `Present`'s five parameters plus a flags word.
#[cfg(windows)]
type DallatTaqdeemMumtadd = unsafe extern "system" fn(
    *mut c_void,
    *const RECT,
    *const RECT,
    HWND,
    *const c_void,
    u32,
) -> HRESULT;

/// `IDirect3DDevice9::Reset`, as a callable pointer.
#[cfg(windows)]
type DallatIstiaada9 = unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT;

/// `IDirect3DDevice9Ex::ResetEx`, as a callable pointer.
#[cfg(windows)]
type DallatIstiaadaMumtadda =
    unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> HRESULT;

/// `IDirect3DDevice9::Present`, which is where the overlay first becomes
/// possible on this API.
#[cfg(windows)]
unsafe extern "system" fn thunk_taqdeem9(
    jihaz: *mut c_void,
    masdar: *const RECT,
    hadaf: *const RECT,
    nafidha: HWND,
    mintaqa: *const c_void,
) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    shaghghil_min_itar(MasdarKhalfiya::D3D9(jihaz));

    let asl = ASL_TAQDEEM9.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: `asl` is the non-null pointer this module read out of the slot it
    // replaced, so it is `Present`'s own entry point with this exact signature
    // and calling convention, and the module owning it is still mapped because
    // the game is inside a call to it.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTaqdeem9>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified.
    unsafe { asl(jihaz, masdar, hadaf, nafidha, mintaqa) }
}

/// `IDirect3DDevice9Ex::PresentEx`.
#[cfg(windows)]
unsafe extern "system" fn thunk_taqdeem_mumtadd(
    jihaz: *mut c_void,
    masdar: *const RECT,
    hadaf: *const RECT,
    nafidha: HWND,
    mintaqa: *const c_void,
    aalam: u32,
) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    shaghghil_min_itar(MasdarKhalfiya::D3D9(jihaz));

    let asl = ASL_TAQDEEM_MUMTADD.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: as `thunk_taqdeem9`, for `PresentEx`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTaqdeemMumtadd>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified.
    unsafe { asl(jihaz, masdar, hadaf, nafidha, mintaqa, aalam) }
}

/// `IDirect3DDevice9::Reset`, hooked so a lost device can be recovered.
///
/// This is the D3D9 counterpart of [`thunk_taghyeer`] and it matters more.
/// `Reset` is refused with `D3DERR_INVALIDCALL` while anybody holds a
/// `D3DPOOL_DEFAULT` resource, an explicit render target **or a state block** —
/// and the overlay holds a state block for the whole session, because that is
/// how it saves and restores the game's own device state. Without this hook the
/// first alt-tab out of exclusive fullscreen would leave the game unable to
/// recover its device, which is a game that never comes back rather than an
/// overlay that stops drawing.
#[cfg(windows)]
unsafe extern "system" fn thunk_istiaada9(
    jihaz: *mut c_void,
    muallimat: *mut c_void,
) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    atliq_qabl_istiaada();

    let asl = ASL_ISTIAADA9.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: as `thunk_taqdeem9`, for `Reset`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatIstiaada9>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified, including
    // its own presentation parameters, which this module never reads.
    unsafe { asl(jihaz, muallimat) }
}

/// `IDirect3DDevice9Ex::ResetEx`.
///
/// An extended device is not lost by an alt-tab, but `ResetEx` still destroys
/// every `D3DPOOL_DEFAULT` resource and is still refused while a state block
/// exists — so the release this performs is the same one.
#[cfg(windows)]
unsafe extern "system" fn thunk_istiaada_mumtadda(
    jihaz: *mut c_void,
    muallimat: *mut c_void,
    namat: *mut c_void,
) -> HRESULT {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    atliq_qabl_istiaada();

    let asl = ASL_ISTIAADA_MUMTADDA.iqra();
    if asl.is_null() {
        return HRESULT(0);
    }
    // SAFETY: as `thunk_taqdeem9`, for `ResetEx`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatIstiaadaMumtadda>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified.
    unsafe { asl(jihaz, muallimat, namat) }
}

/// Tells the live overlay to let go of everything a `Reset` would refuse over.
///
/// Written once and called from both reset thunks, because the two differ only
/// in the signature they forward and getting the release into one of them and
/// not the other would be a bug that only appears on extended devices.
///
/// `try_lock`: this runs on the game's render thread, and blocking it behind a
/// start attempt on another thread would be a stall the player feels. A lock
/// that is held means a start attempt is in progress, which means no overlay
/// exists yet, which means there is nothing to release.
#[cfg(windows)]
fn atliq_qabl_istiaada() {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Some(mut hirasa) = TARKIB.try_lock()
            && let Some(tabaqa) = hirasa.as_mut().and_then(|mabni| mabni.tabaqa.as_mut())
        {
            let _ = tabaqa.qabl_taghyeer_hajm();
        }
    }));
}

/// `wglSwapBuffers`, the moment an OpenGL context is guaranteed current.
#[cfg(windows)]
unsafe extern "system" fn thunk_tabdil(siyaq: *mut c_void) -> BOOL {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    shaghghil_min_itar(MasdarKhalfiya::Gl);

    let asl = ASL_TABDIL.iqra();
    if asl.is_null() {
        return BOOL::from(false);
    }
    // SAFETY: `asl` is the trampoline `retour` built for `wglSwapBuffers`,
    // which has this exact signature; it stays mapped because the `Masar` that
    // owns it is retained for the life of the process even when the detour is
    // removed.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTabdil>(asl) };
    // SAFETY: the device context is the one the game passed, unmodified.
    unsafe { asl(siyaq) }
}

/// `glXSwapBuffers`, the moment an OpenGL context is guaranteed current.
#[cfg(not(windows))]
unsafe extern "C" fn thunk_tabdil(aard: *mut c_void, satih: core::ffi::c_ulong) {
    let _hirasa = HirasatDukhul::udkhul(&DUKHUL);
    shaghghil_min_itar(MasdarKhalfiya::Gl);

    let asl = ASL_TABDIL.iqra();
    if asl.is_null() {
        return;
    }
    // SAFETY: `asl` is the trampoline `retour` built for `glXSwapBuffers`,
    // which has this exact signature; it stays mapped because the `Masar` that
    // owns it is retained for the life of the process even when the detour is
    // removed.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTabdil>(asl) };
    // SAFETY: the display and drawable are the ones the game passed, unmodified.
    unsafe { asl(aard, satih) };
}

// ---------------------------------------------------------------------------
// What was installed, and what it takes to put it back
// ---------------------------------------------------------------------------

/// A vtable hook set that may cross to the render thread.
#[derive(Debug)]
#[cfg(windows)]
struct HirasatJadwal(KhatfJadwal);

#[cfg(windows)]
#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "the field the lint names records the vtable slots this hook replaced, which is \
              exactly what the claim below is about: they are not `Send` on their own and this \
              wrapper asserts that moving them is sound because they are only ever read and \
              written under the page guard. A thread-safe type would assert something different \
              and would not make the underlying hook any more shareable"
)]
// SAFETY: `KhatfJadwal` is not `Send` on its own because it records the
// addresses of the slots it replaced. It never dereferences them outside the
// verified volatile read-and-write that `taarib_haqn::hirasa` performs with the
// page guard held, so moving the record to the render thread conveys no
// capability the vtable does not already give every thread in the process. This
// module guarantees the rest: the value lives in one static behind a mutex and
// is never aliased.
unsafe impl Send for HirasatJadwal {}

/// A Direct3D 8 installation that may cross to the render thread.
#[cfg(windows)]
#[derive(Debug)]
struct HirasatD3D8(TarkeebD3D8);

#[cfg(windows)]
#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "the field the lint names holds the addresses of two method-table slots, which is \
              exactly what the claim below is about: they are not `Send` on their own and this \
              wrapper asserts that moving them is sound because they are only ever read and \
              written under the page guard. A thread-safe type would assert something different \
              and would not make the underlying hook any more shareable"
)]
// SAFETY: as `HirasatJadwal`. The record holds the addresses of the two slots
// it replaced and never dereferences them outside the verified volatile
// read-and-write `taarib_haqn::hirasa` performs with the page guard held, so
// moving it to the render thread conveys no capability the method table does
// not already give every thread in the process. This module guarantees the
// rest: the value lives in one static behind a mutex and is never aliased.
unsafe impl Send for HirasatD3D8 {}

/// Everything this bootstrap installed, in one value.
///
/// One value because `docs/bidaya.md` requires that anything installed before a
/// failure is removed before returning, and the `Drop` on [`Masar`] and
/// `KhatfJadwal` guarantees exactly that as long as the bootstrap holds them
/// somewhere that is dropped on the failure path.
#[derive(Debug)]
struct Tarkib {
    /// Where this payload was loaded from, and where its log is written.
    mujallad: PathBuf,
    /// Which API the module search answered with.
    wajiha: WajihatRusum,
    /// The patch, mapped for the session and reached through [`bi_ruqaa`].
    ruqaa: MalafRuqaa,
    /// The swap chain's replaced slots.
    #[cfg(windows)]
    jadwal: Option<HirasatJadwal>,
    /// The command queue's replaced slot, on D3D12.
    #[cfg(windows)]
    saff: Option<HirasatJadwal>,
    /// The Direct3D 8 device's two replaced slots.
    ///
    /// Its own type rather than a `HirasatJadwal` because `crate::d3d8` owns
    /// both the installation and the verification that precedes it, and because
    /// unhooking it also clears the callbacks this module handed over.
    #[cfg(windows)]
    thabit8: Option<HirasatD3D8>,
    /// The OpenGL buffer-swap detour.
    masar: Option<Masar>,
    /// The acknowledgement, until the one overlay it enables consumes it.
    iqrar: Option<Iqrar>,
    /// The overlay, once the first frame produced one.
    tabaqa: Option<Tabaqa>,
}

impl Tarkib {
    /// An installation with nothing installed yet.
    const fn jadeed(
        mujallad: PathBuf,
        wajiha: WajihatRusum,
        ruqaa: MalafRuqaa,
        iqrar: Iqrar,
    ) -> Self {
        Self {
            mujallad,
            wajiha,
            ruqaa,
            #[cfg(windows)]
            jadwal: None,
            #[cfg(windows)]
            saff: None,
            #[cfg(windows)]
            thabit8: None,
            masar: None,
            iqrar: Some(iqrar),
            tabaqa: None,
        }
    }

    /// Removes every hook, leaving the game as it was found.
    ///
    /// The saved originals are deliberately **not** cleared. A call already
    /// inside a thunk must still present the game's frame, and restoring a slot
    /// stops new calls from entering while doing nothing about one that is
    /// already past the entry — which is the same reason
    /// [`crate::khataf::AaddadDukhul`] exists.
    fn fukk(&mut self) {
        if let Some(tabaqa) = self.tabaqa.as_mut() {
            let _ = tabaqa.aghliq();
        }
        #[cfg(windows)]
        if let Some(thabit8) = self.thabit8.as_mut() {
            let _ = thabit8.0.fukk();
        }
        #[cfg(windows)]
        if let Some(saff) = self.saff.as_mut() {
            let _ = saff.0.fukk();
        }
        #[cfg(windows)]
        if let Some(jadwal) = self.jadwal.as_mut() {
            let _ = jadwal.0.fukk();
        }
        if let Some(masar) = self.masar.as_ref() {
            let _ = masar.fukk();
        }
    }
}

/// Unhooks an installation and keeps the value alive for the rest of the run.
///
/// Retained rather than dropped, and the difference is a crash: [`Masar`] owns
/// the trampoline the thunk jumps into to reach the original, and freeing that
/// while another thread is executing it would be freeing code mid-instruction.
/// Unhooking is what makes the game untouched; keeping the value is what makes
/// the calls already in flight land somewhere that still exists.
fn taqaad(mut mabni: Tarkib) {
    mabni.fukk();
    MUTAQAAIDA.lock().push(mabni);
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// The three ways a bootstrap can end without the overlay running.
///
/// Kept distinct because they mean different things to whoever reads the log:
/// declined is "not that kind of game", refused is "something you can fix", and
/// failed is "a hook or a read did not do what it said".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HalatRadd {
    /// This is not that kind of game, or no patch is installed.
    Imtinaa,
    /// Something is wrong that the user could fix.
    Rafd,
    /// A hook or a read failed unexpectedly.
    Fashal,
}

impl HalatRadd {
    /// The word the log line begins with.
    const fn ism(self) -> &'static str {
        match self {
            Self::Imtinaa => "declined",
            Self::Rafd => "refused",
            Self::Fashal => "failed",
        }
    }
}

/// One refusal: which state, and the sentence that explains it.
#[derive(Debug)]
struct Radd {
    /// Which of the three states this is.
    hala: HalatRadd,
    /// What happened, in one sentence, with the remedy where there is one.
    sabab: String,
}

impl Radd {
    /// This is not that kind of game, or no patch is installed.
    fn imtinaa(sabab: impl Into<String>) -> Self {
        Self { hala: HalatRadd::Imtinaa, sabab: sabab.into() }
    }

    /// Something is wrong that the user could fix.
    fn rafd(sabab: impl Into<String>) -> Self {
        Self { hala: HalatRadd::Rafd, sabab: sabab.into() }
    }

    /// A hook or a read failed unexpectedly.
    fn fashal(sabab: impl Into<String>) -> Self {
        Self { hala: HalatRadd::Fashal, sabab: sabab.into() }
    }

    /// The one line this refusal writes.
    fn satr(&self) -> String {
        format!("{}: {}", self.hala.ism(), self.sabab)
    }
}

// ---------------------------------------------------------------------------
// The log
// ---------------------------------------------------------------------------

/// Appends one line beside this payload, truncating the log when it is full.
///
/// Every failure here is swallowed. This runs inside somebody's game, and a
/// payload that could not write its own log has no business interrupting a
/// launch to say so.
fn sajjil(mujallad: &Path, satr: &str) {
    use std::io::Write as _;

    let masar = mujallad.join(ISM_SIJILL);
    if std::fs::metadata(&masar).is_ok_and(|bayan| bayan.len() > AQSA_SIJILL) {
        let _ = std::fs::remove_file(&masar);
    }
    let Ok(mut malaf) = std::fs::OpenOptions::new().create(true).append(true).open(&masar) else {
        return;
    };
    let _ = writeln!(malaf, "tabaqa: {satr}");
}

// ---------------------------------------------------------------------------
// Reaching what was started
// ---------------------------------------------------------------------------

/// Runs `amal` against the live overlay, or answers [`None`] when there is none.
///
/// The one way anything else in this crate reaches what the bootstrap started.
/// A closure rather than a guard so the mutex stays private: an overlay handed
/// out as a borrow would be an overlay two frame paths could hold at once.
///
/// Must not be called from inside `amal` itself, and must not be called from a
/// thunk that is already inside a start attempt on the same thread.
pub fn bi_tabaqa<T>(amal: impl FnOnce(&mut Tabaqa) -> T) -> Option<T> {
    let mut hirasa = TARKIB.lock();
    let tabaqa = hirasa.as_mut()?.tabaqa.as_mut()?;
    Some(amal(tabaqa))
}

/// Runs `amal` against the patch this payload holds open beside itself.
///
/// The mapping is the one `taarib_tathbeet::masar_tathbeet` wrote into the
/// game's own directory, validated at bootstrap and held for the session. Every
/// translation the overlay draws comes out of it, so it is handed out the same
/// way the overlay is: through a closure, so the mapping cannot outlive the
/// installation that owns it.
pub fn bi_ruqaa<T>(amal: impl FnOnce(&MalafRuqaa) -> T) -> Option<T> {
    let hirasa = TARKIB.lock();
    let mabni = hirasa.as_ref()?;
    Some(amal(&mabni.ruqaa))
}

/// Whether the overlay has control of this process's frames.
#[must_use]
pub fn hal_bada() -> bool {
    TARKIB.lock().as_ref().is_some_and(|mabni| mabni.tabaqa.is_some())
}

/// Which graphics API this payload attached to, once it has attached to one.
///
/// The live overlay's own answer once there is one, because the backend that
/// answered can differ from the module the search named — a Direct3D 10 search
/// result drawn by the eleventh backend, an OpenGL one drawn by the
/// fixed-function backend. Before attachment it is the search's answer, which
/// is the only one there is.
#[must_use]
pub fn wajiha_hiya() -> Option<WajihatRusum> {
    TARKIB
        .lock()
        .as_ref()
        .map(|mabni| mabni.tabaqa.as_ref().map_or(mabni.wajiha, Tabaqa::wajiha))
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Whether [`taarib_bidaya`] has already been entered.
static DUIYA: AtomicBool = AtomicBool::new(false);

/// Whether the installation is published and the start path may run.
static JAHIZ: AtomicBool = AtomicBool::new(false);

/// Whether a start attempt is running right now.
static MUNSHAGHIL: AtomicBool = AtomicBool::new(false);

/// Whether the question is settled, one way or the other.
static INTAHAT: AtomicBool = AtomicBool::new(false);

/// Whether the wait-for-a-queue line has already been written.
static NUBBIHA: AtomicBool = AtomicBool::new(false);

/// The live installation.
static TARKIB: Mutex<Option<Tarkib>> = Mutex::new(None);

/// Installations that have been unhooked and are kept mapped regardless.
static MUTAQAAIDA: Mutex<Vec<Tarkib>> = Mutex::new(Vec::new());

/// How many calls are currently inside this module's thunks.
static DUKHUL: AaddadDukhul = AaddadDukhul::jadeed();

/// The captured `IDXGISwapChain::Present`.
#[cfg(windows)]
static ASL_PRESENT: AslMahfuz = AslMahfuz::jadeed();

/// The captured `IDXGISwapChain1::Present1`.
#[cfg(windows)]
static ASL_PRESENT1: AslMahfuz = AslMahfuz::jadeed();

/// The captured `IDXGISwapChain::ResizeBuffers`.
#[cfg(windows)]
static ASL_TAGHYEER: AslMahfuz = AslMahfuz::jadeed();

/// The captured `ID3D12CommandQueue::ExecuteCommandLists`.
#[cfg(windows)]
static ASL_TANFEEDH: AslMahfuz = AslMahfuz::jadeed();

/// The captured `IDirect3DDevice9::Present`.
#[cfg(windows)]
static ASL_TAQDEEM9: AslMahfuz = AslMahfuz::jadeed();

/// The captured `IDirect3DDevice9::Reset`.
#[cfg(windows)]
static ASL_ISTIAADA9: AslMahfuz = AslMahfuz::jadeed();

/// The captured `IDirect3DDevice9Ex::PresentEx`.
#[cfg(windows)]
static ASL_TAQDEEM_MUMTADD: AslMahfuz = AslMahfuz::jadeed();

/// The captured `IDirect3DDevice9Ex::ResetEx`.
#[cfg(windows)]
static ASL_ISTIAADA_MUMTADDA: AslMahfuz = AslMahfuz::jadeed();

/// The trampoline that reaches the platform's real buffer swap.
static ASL_TABDIL: AslMahfuz = AslMahfuz::jadeed();

/// The game's direct command queue, captured as it submitted.
#[cfg(windows)]
static SAFF: Mutex<Option<ID3D12CommandQueue>> = Mutex::new(None);

/// Whether that queue has been captured, so the hook stops taking the lock.
#[cfg(windows)]
static SAFF_JAHIZ: AtomicBool = AtomicBool::new(false);
