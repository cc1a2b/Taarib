//! الواجهة — the shape every graphics backend has, and the contract it signs.
//!
//! Five APIs, five genuinely different implementations, one interface. This
//! module is that interface and nothing else: it contains no Direct3D, no
//! OpenGL, no Vulkan, and it compiles on every platform including the ones where
//! none of the five exists. That is deliberate. The overlay's lifecycle, its
//! frame budget accounting, its surface-change handling and its disable-on-fault
//! rule are the same on all five, and a backend that reimplemented any of them
//! would be a backend that could get them subtly wrong on one API only.
//!
//! The fifth is [`crate::d3d9`], and it is the one that pressed hardest on this
//! interface without changing it. Its device can be *lost*, which arrives here
//! as the [`KhataTabaqa::SathTaghayyar`] every other backend already produces
//! for a resize; its state block has to be released before the game's own
//! `Reset`, which is the [`Khattaf::atliq_sath`] DXGI already needed for
//! `ResizeBuffers`. Both facts were already expressible, which is the test a
//! shared interface passes or fails.
//!
//! ## The frame contract
//!
//! Every backend is called at exactly one moment: after the game has finished
//! drawing and before the frame is presented. Inside that window it must do four
//! things, in this order, and the ordering is not negotiable:
//!
//! 1. **Save** whatever renderer state it is about to change.
//! 2. **Draw** the overlay through the game's own device.
//! 3. **Restore** every piece of state it saved.
//! 4. **Report** how long that took.
//!
//! Step 3 has no failure mode that permits continuing. A backend that cannot
//! restore state returns [`KhataTabaqa::HalaGhayrMustaada`], and
//! [`Tabaqa::itar`] treats it as terminal: the overlay disables itself, unhooks,
//! and the session's remaining frames render exactly as the game drew them.
//! Retrying is not offered, because the state the retry would run against is
//! already wrong.
//!
//! ## Why the trait does not own the hook
//!
//! [`Khattaf`] draws. It does not install itself, and it does not know how it
//! was reached — the hook is [`crate::khataf`]'s, and passing an already-hooked
//! context in means a backend can be exercised against a device created for the
//! purpose without a game being involved. It also means the five backends
//! contain no unsafe hooking code between them; all of that lives in one module
//! where it can be read as a unit.
//!
//! ## Budget, not best effort
//!
//! [`MeezaniyatItar`] is a hard per-frame budget in microseconds, checked before
//! the overlay draws rather than measured after it. A frame the overlay would
//! blow the budget on is a frame the overlay skips — the text stays on screen
//! from the previous draw and nothing stutters. This is the difference between
//! "the overlay costs about two milliseconds" and "the overlay costs about two
//! milliseconds except when it doesn't", and only the first is a sentence the
//! control panel can honestly display.

use std::fmt;

use crate::khata::KhataTabaqa;
use crate::sidq::Iqrar;

/// Which graphics API a backend speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WajihatRusum {
    /// Direct3D 9, hooked at `IDirect3DDevice9::Present` and `Reset`.
    ///
    /// `Reset` is hooked for a reason DXGI has no equivalent of: a D3D9 device
    /// can be *lost*, `Reset` is how the game recovers it, and `Reset` fails
    /// while the overlay holds a state block or any `D3DPOOL_DEFAULT` resource.
    /// See [`crate::d3d9`].
    Direct3D9,
    /// Direct3D 11, hooked at `IDXGISwapChain::Present` and `Present1`.
    Direct3D11,
    /// Direct3D 12, hooked at `IDXGISwapChain3::Present1` with the command
    /// queue captured through `ID3D12CommandQueue::ExecuteCommandLists`.
    Direct3D12,
    /// OpenGL, hooked at the platform's buffer swap.
    OpenGl,
    /// Vulkan, reached as a layer rather than a hook.
    Vulkan,
    /// Direct3D 8, hooked at `IDirect3DDevice8::Present` and `Reset`.
    ///
    /// Not a smaller Direct3D 9. Presentation is a method of the device, there
    /// is no swap chain interface to reach it through, and the state the
    /// overlay disturbs is saved with a state block rather than read back —
    /// because a device created `D3DCREATE_PUREDEVICE` refuses to report its
    /// own state at all. See [`crate::d3d8`].
    Direct3D8,
    /// Direct3D 10, hooked at `IDXGISwapChain::Present` like Direct3D 11.
    ///
    /// The same hook, a different device: DXGI owns the swap chain on both
    /// generations, so the hook that catches an eleventh-generation game
    /// catches a tenth-generation one unchanged and only the device the swap
    /// chain hands back differs. See [`crate::d3d10`].
    Direct3D10,
    /// Fixed-function OpenGL, hooked at the same buffer swap as [`Self::OpenGl`].
    ///
    /// A separate API rather than a mode of the modern backend. A pre-shader
    /// context has no `glCreateShader`, no vertex array object and no buffer
    /// object; it has a matrix stack, a texture environment and an attribute
    /// stack, and none of those appears in the modern backend's save list. See
    /// [`crate::gl_thabit`].
    OpenGlThabit,
}

impl WajihatRusum {
    /// The name that appears in a log line and the capability report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Direct3D9 => "Direct3D 9",
            Self::Direct3D11 => "Direct3D 11",
            Self::Direct3D12 => "Direct3D 12",
            Self::OpenGl => "OpenGL",
            Self::Vulkan => "Vulkan",
            Self::Direct3D8 => "Direct3D 8",
            Self::Direct3D10 => "Direct3D 10",
            Self::OpenGlThabit => "OpenGL (fixed function)",
        }
    }

    /// Whether this API is reached through a documented extension point rather
    /// than by rewriting somebody's function table.
    ///
    /// True for Vulkan alone, and it is the reason Vulkan is preferred when a
    /// game offers both: a layer is enumerated by the loader, is removed by
    /// deleting a manifest, and never resembles a process attaching to another
    /// process. Every other API here is a vtable write, which works and is
    /// correct and is still the second-best way to get there.
    #[must_use]
    pub const fn tabaqa_rasmiya(self) -> bool {
        matches!(self, Self::Vulkan)
    }

    /// The modules whose presence suggests this API is in use.
    ///
    /// Suggestive rather than conclusive — a game can link `d3d11.dll` and
    /// render with Vulkan — which is why [`crate::khataf`] confirms by finding a
    /// live swap chain rather than by trusting this list. It exists to order the
    /// search so the common case is checked first.
    #[must_use]
    pub const fn maktabat(self) -> &'static [&'static str] {
        match self {
            // No DXGI. A D3D9 process that also loads `dxgi.dll` is loading it
            // for something else — the desktop duplication API, or a launcher —
            // and naming it here would make every D3D11 game look like a D3D9
            // one to a probe that stopped at the first match.
            Self::Direct3D9 => &["d3d9.dll"],
            Self::Direct3D11 => &["d3d11.dll", "dxgi.dll"],
            Self::Direct3D12 => &["d3d12.dll", "dxgi.dll"],
            Self::OpenGl => &["opengl32.dll", "libGL.so.1", "libEGL.so.1"],
            Self::Vulkan => &["vulkan-1.dll", "libvulkan.so.1"],
            // No DXGI either, and for the opposite reason to D3D9's: Direct3D 8
            // predates it entirely, so a process carrying both `d3d8.dll` and
            // `dxgi.dll` is a modern process that loaded a wrapper, not a
            // Direct3D 8 game.
            Self::Direct3D8 => &["d3d8.dll"],
            // `d3d10_1.dll` as well as `d3d10.dll`: a Direct3D 10.1 game links
            // only the newer one, and its device answers a `QueryInterface` for
            // `ID3D10Device` because `ID3D10Device1` derives from it.
            Self::Direct3D10 => &["d3d10.dll", "d3d10_1.dll", "dxgi.dll"],
            // The same modules as the modern backend, because they are the same
            // modules. What separates the two is what the *context* offers, and
            // that is a question no module list can answer — see
            // [`crate::gl_thabit::ikhtar`], which asks the context instead.
            Self::OpenGlThabit => &["opengl32.dll", "libGL.so.1"],
        }
    }

    /// Every API this build implements, in the order they are probed.
    ///
    /// Vulkan first because it is the only one with a documented extension
    /// point, then D3D12 and D3D11 — a game linking both DXGI generations is
    /// almost always a D3D12 game with a D3D11 compatibility path it does not
    /// present through — then D3D9, then OpenGL last, because `opengl32.dll` is
    /// loaded by a great many Windows processes that never draw with it.
    ///
    /// D3D9 sits below the two newer Direct3D generations rather than above
    /// them because `d3d9.dll` is what a translation layer such as DXVK or
    /// d9vk maps into a process that is really presenting through something
    /// else, and because a handful of newer games ship a D3D9 fallback renderer
    /// they never select. Below D3D12 and D3D11 it is reached only when neither
    /// of those is present, which is exactly when it is the real renderer.
    /// The three older APIs sit below the generation each of them precedes, and
    /// fixed-function OpenGL sits last of all. That ordering is not seniority:
    /// `opengl32.dll` is loaded by a great many Windows processes that never
    /// draw with it, and it is loaded by *every* OpenGL process whether the
    /// context is a modern core profile or a compatibility one — so the module
    /// list cannot separate the two GL backends and the probe reaches the
    /// modern one first. [`crate::gl_thabit::ikhtar`] is what actually decides,
    /// by asking the context which entry points it has.
    #[must_use]
    pub const fn jamee() -> [Self; 8] {
        [
            Self::Vulkan,
            Self::Direct3D12,
            Self::Direct3D11,
            Self::Direct3D10,
            Self::Direct3D9,
            Self::Direct3D8,
            Self::OpenGl,
            Self::OpenGlThabit,
        ]
    }
}

impl fmt::Display for WajihatRusum {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj.write_str(self.ism())
    }
}

/// The dimensions and format of the surface the game is presenting.
///
/// Rebuilt whenever the swap chain changes rather than cached across a resize,
/// because every overlay resource sized against it is invalid the moment it
/// changes. See [`KhataTabaqa::SathTaghayyar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasfSath {
    /// Width in pixels.
    pub ard: u32,
    /// Height in pixels.
    pub irtifa: u32,
    /// The backbuffer format, as the API names it, for the capture path.
    pub sigha: SighatSath,
    /// Whether the surface's colour values are stored non-linearly.
    ///
    /// Load-bearing for the overlay's blend: drawing premultiplied alpha into an
    /// sRGB target without accounting for the encoding produces text that is
    /// visibly too dark against a light background, which is the single most
    /// common way an overlay looks wrong while every individual step looks right.
    pub sirgb: bool,
}

impl WasfSath {
    /// Whether a normalized rectangle lands on at least one pixel.
    ///
    /// Regions are stored normalized so a resolution change does not invalidate
    /// them; this is where that promise is checked against a real surface.
    #[must_use]
    pub fn yasa(self, nisbi: MustatilNisbi) -> bool {
        self.ard > 0
            && self.irtifa > 0
            && nisbi.salih()
            && nisbi.fi_bikselat(self.ard, self.irtifa).is_some()
    }
}

/// The backbuffer's pixel format, reduced to what the capture path must know.
///
/// Deliberately not a passthrough of any API's enum. The recognizer needs three
/// facts — channel order, whether there is an alpha channel, and how many bits
/// per channel — and reducing forty D3D formats to those three here means the
/// conversion code is written once instead of once per backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SighatSath {
    /// Eight bits per channel, blue-green-red-alpha order.
    Bgra8,
    /// Eight bits per channel, red-green-blue-alpha order.
    Rgba8,
    /// Ten bits per colour channel and two of alpha, red-green-blue order.
    Rgb10a2,
    /// Sixteen-bit float per channel, red-green-blue-alpha order.
    Rgba16f,
}

impl SighatSath {
    /// Bytes one pixel occupies.
    #[must_use]
    pub const fn bayt_lil_biksel(self) -> u32 {
        match self {
            Self::Bgra8 | Self::Rgba8 | Self::Rgb10a2 => 4,
            Self::Rgba16f => 8,
        }
    }

    /// The name that appears in [`KhataTabaqa::SighaGhayrMaduma`].
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Bgra8 => "B8G8R8A8",
            Self::Rgba8 => "R8G8B8A8",
            Self::Rgb10a2 => "R10G10B10A2",
            Self::Rgba16f => "R16G16B16A16_FLOAT",
        }
    }

    /// Whether values in this format are outside the zero-to-one range.
    ///
    /// True for the float format, which is what an HDR game presents. Capture
    /// has to tone-map before recognition rather than clamping: clamping an HDR
    /// frame produces a white rectangle where the dialogue box was, and the
    /// recognizer reads nothing from it.
    #[must_use]
    pub const fn wasi_al_mada(self) -> bool {
        matches!(self, Self::Rgba16f)
    }
}

/// A rectangle in normalized surface coordinates.
///
/// Zero to one on both axes, origin top-left. Stored this way rather than in
/// pixels for one reason that matters more than it sounds: a player draws a
/// region around a dialogue box at 1920×1080, then switches to fullscreen at
/// 2560×1440, and the region still frames the dialogue box. A pixel rectangle
/// would be somewhere in the upper left and the player would have to draw it
/// again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MustatilNisbi {
    /// Left edge, zero to one.
    pub yasar: f32,
    /// Top edge, zero to one.
    pub aala: f32,
    /// Width, zero to one.
    pub ard: f32,
    /// Height, zero to one.
    pub irtifa: f32,
}

impl MustatilNisbi {
    /// The whole surface.
    pub const KAAMIL: Self = Self { yasar: 0.0, aala: 0.0, ard: 1.0, irtifa: 1.0 };

    /// A rectangle from its edges, clamped into range.
    ///
    /// Clamped rather than refused because this is what a mouse drag produces:
    /// a player dragging off the edge of the window means "to the edge", and
    /// rejecting the drag would be rejecting the obvious intent.
    #[must_use]
    pub fn min_hudud(yasar: f32, aala: f32, yameen: f32, asfal: f32) -> Self {
        let (yasar, yameen) = if yasar <= yameen { (yasar, yameen) } else { (yameen, yasar) };
        let (aala, asfal) = if aala <= asfal { (aala, asfal) } else { (asfal, aala) };
        let yasar = yasar.clamp(0.0, 1.0);
        let aala = aala.clamp(0.0, 1.0);
        Self {
            yasar,
            aala,
            ard: (yameen.clamp(0.0, 1.0) - yasar).max(0.0),
            irtifa: (asfal.clamp(0.0, 1.0) - aala).max(0.0),
        }
    }

    /// Whether this rectangle is inside the surface and has area.
    ///
    /// A zero-area rectangle is not an error anywhere it is produced — it is
    /// what a click without a drag makes — but it is never a region worth
    /// capturing, and this is the one place that judgement is written.
    #[must_use]
    pub fn salih(self) -> bool {
        self.ard > 0.0
            && self.irtifa > 0.0
            && self.yasar >= 0.0
            && self.aala >= 0.0
            && self.yasar + self.ard <= 1.0 + f32::EPSILON
            && self.aala + self.irtifa <= 1.0 + f32::EPSILON
    }

    /// This rectangle in pixels on a surface of the given size.
    ///
    /// [`None`] when the result would have no area — a region three pixels tall
    /// on a small window rounds to nothing, and returning an empty rectangle for
    /// the capture path to discover would push the check to every caller.
    #[must_use]
    pub fn fi_bikselat(self, ard: u32, irtifa: u32) -> Option<MustatilBiksel> {
        let bikselat = |nisba: f32, madaa: u32| -> u32 {
            let khaam = nisba * madaa_f32(madaa);
            if khaam <= 0.0 {
                0
            } else {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped to [0, madaa] as f32 immediately above, so the value is a \
                              non-negative integer below u32::MAX before the conversion"
                )]
                {
                    khaam.min(madaa_f32(madaa)).round() as u32
                }
            }
        };
        let yasar = bikselat(self.yasar, ard);
        let aala = bikselat(self.aala, irtifa);
        let araad = bikselat(self.ard, ard).min(ard.saturating_sub(yasar));
        let irtifaa = bikselat(self.irtifa, irtifa).min(irtifa.saturating_sub(aala));
        if araad == 0 || irtifaa == 0 {
            return None;
        }
        Some(MustatilBiksel { yasar, aala, ard: araad, irtifa: irtifaa })
    }
}

/// A dimension as a float, without a lossy-looking cast at every call site.
///
/// `u32` to `f32` loses precision above 2^24, which is 16.7 million — four
/// orders of magnitude past any surface dimension that will ever exist. The
/// conversion is written once, here, with that stated, so the lint's warning is
/// answered in one place rather than suppressed in five.
const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// A rectangle in surface pixels, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MustatilBiksel {
    /// Left edge in pixels.
    pub yasar: u32,
    /// Top edge in pixels.
    pub aala: u32,
    /// Width in pixels.
    pub ard: u32,
    /// Height in pixels.
    pub irtifa: u32,
}

impl MustatilBiksel {
    /// How many pixels this rectangle covers.
    #[must_use]
    pub const fn adad_bikselat(self) -> u64 {
        (self.ard as u64).saturating_mul(self.irtifa as u64)
    }

    /// The byte length of a tightly packed copy in the given format.
    #[must_use]
    pub const fn tul_bayt(self, sigha: SighatSath) -> u64 {
        self.adad_bikselat().saturating_mul(sigha.bayt_lil_biksel() as u64)
    }

}

/// The overlay's per-frame time budget, in microseconds.
///
/// Checked before drawing rather than measured after. A frame whose remaining
/// budget will not cover the last measured draw is a frame the overlay skips:
/// the previously drawn text is already on screen from the game's own
/// perspective only if the overlay redraws it, so a skip means one frame without
/// Arabic rather than a stutter the player feels. Given the choice between a
/// dropped overlay frame and a dropped game frame, this product drops its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeezaniyatItar {
    /// The ceiling on one overlay draw, in microseconds.
    pub saqf_mikro: u32,
    /// The last draw's measured cost, in microseconds.
    pub akhir_mikro: u32,
    /// How many frames have been skipped because the budget would not cover the
    /// draw.
    pub matruka: u64,
    /// How many frames have been drawn.
    pub marsuma: u64,
}

impl MeezaniyatItar {
    /// Two milliseconds, the default ceiling.
    ///
    /// Chosen against the frame time it is a fraction of rather than as a round
    /// number: at 60 Hz a frame is 16,667 microseconds, so two milliseconds is
    /// twelve percent — visible in a frame time graph, not visible in play. At
    /// 144 Hz it is twenty-nine percent, which is why the control panel shows
    /// the real number and lets it be lowered.
    pub const SAQF_IFTIRADI: u32 = 2_000;

    /// A fresh budget at the default ceiling.
    #[must_use]
    pub const fn jadeeda() -> Self {
        Self { saqf_mikro: Self::SAQF_IFTIRADI, akhir_mikro: 0, matruka: 0, marsuma: 0 }
    }

    /// Whether this frame can afford the overlay.
    ///
    /// The first frame always can — there is no measurement yet, and refusing to
    /// draw until a measurement exists would mean never drawing.
    #[must_use]
    pub const fn yasmah(&self) -> bool {
        self.akhir_mikro == 0 || self.akhir_mikro <= self.saqf_mikro
    }

    /// Records a completed draw.
    pub const fn sajjil_rasm(&mut self, mikro: u32) {
        self.akhir_mikro = mikro;
        self.marsuma = self.marsuma.saturating_add(1);
    }

    /// Records a frame the overlay stood down from.
    pub const fn sajjil_tark(&mut self) {
        self.matruka = self.matruka.saturating_add(1);
    }

    /// The sentence the control panel displays, in microseconds and percent of
    /// a 60 Hz frame.
    #[must_use]
    pub fn wasf(&self) -> String {
        let nisba = if self.akhir_mikro == 0 {
            0.0
        } else {
            f64::from(self.akhir_mikro) * 100.0 / 16_667.0
        };
        format!(
            "{} µs per frame ({nisba:.1}% of a 60 Hz frame); {} drawn, {} skipped over budget",
            self.akhir_mikro, self.marsuma, self.matruka
        )
    }
}

/// What the overlay is asked to draw this frame.
///
/// Produced by [`crate::manatiq`] out of what [`crate::qira`] recognized and
/// what the translation pipeline returned, and consumed by whichever backend is
/// live. It carries shaped, positioned output — never text — because shaping
/// happens in `saff` and a backend that received a string would be a second
/// place Arabic could be laid out.
#[derive(Debug, Clone)]
pub struct LawhatRasm {
    /// The quads to draw, in submission order.
    pub qitaat: Vec<QitaRasm>,
    /// The surface these were positioned against.
    ///
    /// Compared against the live surface before drawing. A batch built for a
    /// surface that has since changed is discarded rather than scaled: scaled
    /// text is blurry text, and the next frame's batch will be correct.
    pub sath: WasfSath,
}

impl LawhatRasm {
    /// An empty batch for a surface.
    #[must_use]
    pub const fn khaliya(sath: WasfSath) -> Self {
        Self { qitaat: Vec::new(), sath }
    }

    /// Whether there is anything to draw.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.qitaat.is_empty()
    }
}

/// One textured quad: a glyph from Taarib's atlas, or a backing plate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QitaRasm {
    /// Where it goes on the surface, in pixels.
    pub mawdi: MustatilBiksel,
    /// Where its texture is in the atlas, normalized.
    ///
    /// [`None`] for a solid backing plate, which samples nothing and is the one
    /// case where [`QitaRasm::lawn`] is the whole appearance.
    pub khareeta: Option<MustatilNisbi>,
    /// Premultiplied linear RGBA, zero to one.
    pub lawn: [f32; 4],
}

/// A backend that draws the overlay through one graphics API.
///
/// Implemented five times. Every implementation is `unsafe` internally and none
/// of that reaches this trait: the methods here take already-validated
/// parameters and return values, and the unsafety of talking to a device lives
/// behind them with its invariants written at each block.
///
/// [`fmt::Debug`] is a supertrait rather than a manual impl on `dyn Khattaf`,
/// so a backend that forgets it does not compile — which is the same rule
/// `missing_debug_implementations` applies to every other type in this
/// workspace, extended to the one place a trait object would have escaped it.
/// A diagnostics bundle that could not name which backend was live would be a
/// bundle missing the first thing anybody asks.
pub trait Khattaf: Send + fmt::Debug {
    /// Which API this is.
    fn wajiha(&self) -> WajihatRusum;

    /// The surface the game is presenting right now.
    ///
    /// Re-read every frame rather than cached. A cached surface description is
    /// how an overlay ends up drawing a 1920-wide batch onto a 2560-wide
    /// backbuffer for the one frame between a resize and its notification.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SathTaghayyar`] when the swap chain is mid-transition and
    /// has no valid surface this frame, which is recovered from by skipping the
    /// frame.
    fn sath(&self) -> Result<WasfSath, KhataTabaqa>;

    /// Builds or rebuilds every resource sized against the surface.
    ///
    /// Called once at startup and again after every [`KhataTabaqa::SathTaghayyar`].
    /// An implementation must release the old resources before allocating the
    /// new ones and must tolerate being called when nothing was allocated yet.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] naming which resource, with the API's own
    /// account of why.
    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa>;

    /// Uploads the glyph atlas the overlay draws from.
    ///
    /// Called when the atlas changes, which is rare — a font size change, a
    /// script the atlas did not yet cover. The bytes are premultiplied RGBA8 in
    /// row order, `ard * 4` bytes per row.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the texture cannot be created or
    /// written, and [`KhataTabaqa::HajmMufrit`] when the atlas is larger than
    /// this build will upload.
    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa>;

    /// Draws one batch, saving and restoring renderer state around it.
    ///
    /// The whole contract of this crate in one method. An implementation:
    ///
    /// * saves every piece of state it is about to change;
    /// * draws;
    /// * restores all of it, unconditionally, including on the error paths.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] for a failure that leaves state consistent
    /// — the caller skips the frame and tries again — and
    /// [`KhataTabaqa::HalaGhayrMustaada`] for one that does not, which is
    /// terminal and disables the overlay.
    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa>;

    /// Copies the presented image, or the part of it a region names.
    ///
    /// Returns tightly packed bytes in the surface's own format, which the
    /// recognizer converts. Backends convert on the GPU where the API allows it
    /// and copy raw where it does not; either way what comes back here is
    /// packed, because a caller reasoning about row pitch is a caller that will
    /// eventually get it wrong on one backend.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::IltiqatFashil`] when the read-back does not complete,
    /// [`KhataTabaqa::SighaGhayrMaduma`] for a format with no conversion, and
    /// [`KhataTabaqa::MintaqaKharij`] when the rectangle is not on the surface.
    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa>;

    /// Releases only what is sized against the current surface, immediately.
    ///
    /// Called from inside `IDXGISwapChain::ResizeBuffers` and from inside
    /// `IDirect3DDevice9::Reset`, **before** either reaches the runtime, and
    /// defaulted to doing nothing because only Direct3D needs it. The reason it
    /// exists at all is specific and not guessable, and the two generations
    /// arrive at it from different directions.
    ///
    /// On DXGI, `ResizeBuffers` returns `DXGI_ERROR_INVALID_CALL` while
    /// *anybody* holds a reference to a backbuffer, and both newer Direct3D
    /// backends hold one for the whole session — D3D11 in its render target
    /// view, D3D12 in every frame's resource slot.
    ///
    /// On Direct3D 9, `Reset` is refused while anybody holds a
    /// `D3DPOOL_DEFAULT` resource, an explicit render target, an additional
    /// swap chain **or a state block** — and the state block is exactly the
    /// object [`crate::d3d9`] keeps between frames to save and restore the
    /// game's own device state with. It is the one D3D9 overlays get wrong,
    /// because nothing about a state block looks like a resource.
    ///
    /// In both cases a game that resized or reset with the overlay installed
    /// would get a refusal from a call that has never once refused it, and
    /// would have no way to attribute that to Taarib.
    ///
    /// OpenGL has no backbuffer object to hold and Vulkan replaces the whole
    /// swapchain rather than resizing one, so both leave this alone. The default
    /// is here rather than in two `impl` blocks precisely so that reading this
    /// doc is how somebody learns why the other two do nothing.
    ///
    /// After this returns, the backend must behave as though it had never been
    /// initialized: [`Khattaf::irsim`] rebuilds against the batch's surface, or
    /// [`Khattaf::hayyi`] is called again first. Either is correct; neither may
    /// draw through the released resources.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when a resource cannot be released, which
    /// for D3D12 includes a fence that has not confirmed the GPU is finished
    /// with what is about to be freed — the one case where releasing anyway
    /// would be worse than reporting.
    fn atliq_sath(&mut self) -> Result<(), KhataTabaqa> {
        Ok(())
    }

    /// Releases everything this backend allocated.
    ///
    /// Called on shutdown and on the terminal error path. Must be safe to call
    /// twice and safe to call when initialization never completed — the second
    /// case is the common one, because a backend that failed to initialize is
    /// exactly the backend that gets torn down.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when a resource cannot be released, which
    /// is reported and not acted on: there is nothing useful to do about it, and
    /// the process is ending or the overlay is off either way.
    fn ahmil(&mut self) -> Result<(), KhataTabaqa>;
}

/// The overlay's own state, independent of which backend is under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatTabaqa {
    /// Hooked, and drawing.
    Amila,
    /// Hooked, and standing down at the user's request.
    Mutawaqqifa,
    /// Not hooked, and will not be again this session.
    ///
    /// Reached only from a terminal fault. Distinct from
    /// [`HalatTabaqa::Mutawaqqifa`] because the user can resume from that one
    /// and cannot resume from this one — and telling them the difference is the
    /// point of having two states.
    Muattala,
}

impl HalatTabaqa {
    /// Whether the overlay draws in this state.
    #[must_use]
    pub const fn tarsim(self) -> bool {
        matches!(self, Self::Amila)
    }

    /// Whether the user can turn it back on.
    #[must_use]
    pub const fn qabila_lil_istinaf(self) -> bool {
        matches!(self, Self::Mutawaqqifa)
    }

    /// The name shown in the control panel.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Amila => "on",
            Self::Mutawaqqifa => "paused",
            Self::Muattala => "disabled after a fault",
        }
    }
}

/// The overlay: one backend, its budget, its state, and the disclosure that let
/// it start.
///
/// Owns the lifecycle rules that are identical across all five APIs, so that a
/// backend is only ever asked to draw and never asked to decide whether it
/// should.
#[derive(Debug)]
pub struct Tabaqa {
    khattaf: Box<dyn Khattaf>,
    hala: HalatTabaqa,
    meezaniya: MeezaniyatItar,
    sath: Option<WasfSath>,
    iqrar: Iqrar,
    athar: Vec<String>,
}

impl Tabaqa {
    /// Starts the overlay over a backend, with the disclosure acknowledged.
    ///
    /// [`Iqrar`] is taken by value and there is no other constructor. That is
    /// the enforcement: an overlay cannot exist without a disclosure having been
    /// shown, and the disclosure cannot be forged without displaying it. See
    /// [`crate::sidq`].
    ///
    /// # Errors
    ///
    /// Whatever [`Khattaf::sath`] and [`Khattaf::hayyi`] refuse. A backend that
    /// cannot describe its surface or build its resources is torn down here
    /// rather than left half-initialized, and the caller gets the refusal.
    pub fn shaghghil(mut khattaf: Box<dyn Khattaf>, iqrar: Iqrar) -> Result<Self, KhataTabaqa> {
        let mut athar = vec![iqrar.satr_sijill(), format!("backend: {}", khattaf.wajiha())];
        let sath = match khattaf.sath().and_then(|sath| {
            khattaf.hayyi(sath)?;
            Ok(sath)
        }) {
            Ok(sath) => sath,
            Err(khata) => {
                // Torn down here rather than dropped half-built. A backend whose
                // `hayyi` failed part way through has allocated some of what it
                // needs and none of what it does not, and `ahmil` is documented
                // to handle exactly that.
                let _ = khattaf.ahmil();
                return Err(khata);
            }
        };
        athar.push(format!(
            "surface {}×{} {}{}",
            sath.ard,
            sath.irtifa,
            sath.sigha.ism(),
            if sath.sirgb { " sRGB" } else { "" }
        ));
        Ok(Self {
            khattaf,
            hala: HalatTabaqa::Amila,
            meezaniya: MeezaniyatItar::jadeeda(),
            sath: Some(sath),
            iqrar,
            athar,
        })
    }

    /// Which API is under this overlay.
    #[must_use]
    pub fn wajiha(&self) -> WajihatRusum {
        self.khattaf.wajiha()
    }

    /// The overlay's state.
    #[must_use]
    pub const fn hala(&self) -> HalatTabaqa {
        self.hala
    }

    /// The frame budget and what it has measured.
    #[must_use]
    pub const fn meezaniya(&self) -> &MeezaniyatItar {
        &self.meezaniya
    }

    /// The surface the last frame validated against.
    ///
    /// [`None`] before the first frame and after [`Tabaqa::qabl_taghyeer_hajm`],
    /// which is exactly when there is no answer rather than a stale one.
    ///
    /// This exists because a batch has to be *built* against the surface
    /// [`Tabaqa::itar`] will check it against, and the producer cannot ask the
    /// backend directly — [`Tabaqa`] owns it. Without this a caller would either
    /// keep a second copy of the surface, which is the copy that goes stale, or
    /// build against a guess and have every frame discarded.
    #[must_use]
    pub const fn sath(&self) -> Option<WasfSath> {
        self.sath
    }

    /// Changes the per-frame ceiling, in microseconds.
    ///
    /// Clamped to a floor of one hundred microseconds. A ceiling of zero would
    /// mean an overlay that never draws while claiming to be on, which is worse
    /// than an overlay the user turned off — they would see nothing and have no
    /// reason to look at the budget.
    pub const fn ihdud(&mut self, saqf_mikro: u32) {
        self.meezaniya.saqf_mikro = if saqf_mikro < 100 { 100 } else { saqf_mikro };
    }

    /// The disclosure this overlay was started with.
    #[must_use]
    pub const fn iqrar(&self) -> &Iqrar {
        &self.iqrar
    }

    /// What has happened to this overlay, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Stands the overlay down without unhooking.
    pub fn awqif(&mut self) {
        if matches!(self.hala, HalatTabaqa::Amila) {
            self.hala = HalatTabaqa::Mutawaqqifa;
            self.athar.push("paused by the user".to_owned());
        }
    }

    /// Resumes a paused overlay.
    ///
    /// Does nothing to a disabled one. A fault that reached
    /// [`HalatTabaqa::Muattala`] is not recovered from by asking again.
    pub fn istanif(&mut self) {
        if matches!(self.hala, HalatTabaqa::Mutawaqqifa) {
            self.hala = HalatTabaqa::Amila;
            self.athar.push("resumed by the user".to_owned());
        }
    }

    /// Draws one frame, applying every rule that is the same on all five APIs.
    ///
    /// In order: is the overlay on, does the budget allow it, has the surface
    /// changed, is there anything to draw, and was the batch built for the
    /// surface that is live now. Only then is the backend asked to draw.
    ///
    /// `mikro` is the caller's measurement of the last draw, in microseconds.
    /// The clock is the hook's, not this crate's: this runs on a game's render
    /// thread, where the platform's timer is already being read by the code that
    /// called us.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] from a recoverable backend failure, after
    /// which the next frame is attempted. [`KhataTabaqa::HalaGhayrMustaada`]
    /// after the overlay has already disabled itself — returned so the caller
    /// can unhook, not so it can retry.
    pub fn itar(&mut self, lawha: &LawhatRasm, mikro: u32) -> Result<(), KhataTabaqa> {
        if !self.hala.tarsim() {
            return Ok(());
        }
        if !self.meezaniya.yasmah() {
            self.meezaniya.sajjil_tark();
            return Ok(());
        }

        let sath = match self.khattaf.sath() {
            Ok(sath) => sath,
            Err(khata) => {
                // A surface mid-transition is a skipped frame, not a fault. The
                // resources are dropped so the next successful `sath` rebuilds
                // them rather than drawing through resources sized for a surface
                // that no longer exists.
                self.sath = None;
                return Err(khata);
            }
        };
        if self.sath != Some(sath) {
            self.athar.push(format!("surface changed to {}×{}", sath.ard, sath.irtifa));
            self.khattaf.hayyi(sath)?;
            self.sath = Some(sath);
            // The batch in hand was positioned against the old surface. Drawing
            // it now would place text by the previous resolution's pixels; the
            // next frame's batch is built against the new one.
            self.meezaniya.sajjil_tark();
            return Ok(());
        }

        if lawha.khali() {
            return Ok(());
        }
        if lawha.sath != sath {
            self.meezaniya.sajjil_tark();
            return Ok(());
        }

        match self.khattaf.irsim(lawha) {
            Ok(()) => {
                self.meezaniya.sajjil_rasm(mikro);
                Ok(())
            }
            Err(khata @ KhataTabaqa::HalaGhayrMustaada { .. }) => {
                // Terminal, and terminal immediately. The game is now drawing
                // with state the overlay left behind, and every further frame
                // compounds it. Nothing here retries.
                self.hala = HalatTabaqa::Muattala;
                self.athar.push(format!("disabled after a state fault: {khata}"));
                let _ = self.khattaf.ahmil();
                Err(khata)
            }
            Err(khata) => {
                self.meezaniya.sajjil_tark();
                Err(khata)
            }
        }
    }

    /// Tells the backend the surface is about to be resized under it.
    ///
    /// Called from the `ResizeBuffers` hook, before the call reaches the
    /// runtime. Two things happen and both are necessary:
    ///
    /// 1. the backend releases its backbuffer references, without which the
    ///    game's own `ResizeBuffers` fails with `DXGI_ERROR_INVALID_CALL`;
    /// 2. this overlay forgets the surface it validated against, so the next
    ///    frame rebuilds rather than drawing through resources sized for a
    ///    backbuffer that no longer exists.
    ///
    /// The second half is the one that is easy to omit and impossible to notice
    /// in review. A borderless-fullscreen transition on the same monitor resizes
    /// to the *same* width, height and format — so [`Tabaqa::itar`]'s
    /// surface-changed check sees nothing, and without this the overlay would
    /// hold released resources it believed were current.
    ///
    /// # Errors
    ///
    /// As [`Khattaf::atliq_sath`]. A failure here is reported and the resize
    /// still proceeds: the game's frame matters more than the overlay's, and an
    /// overlay that blocked a resolution change would be a worse defect than one
    /// that stopped drawing.
    pub fn qabl_taghyeer_hajm(&mut self) -> Result<(), KhataTabaqa> {
        self.sath = None;
        self.athar.push("surface release requested before a resize".to_owned());
        self.khattaf.atliq_sath()
    }

    /// Captures a region of the presented frame.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MintaqaKharij`] when the region does not land on the live
    /// surface, plus whatever [`Khattaf::iltaqit`] refuses.
    pub fn iltaqit(&mut self, mintaqa: MustatilNisbi, ism: &str) -> Result<Vec<u8>, KhataTabaqa> {
        let sath = self.khattaf.sath()?;
        let Some(bikselat) = mintaqa.fi_bikselat(sath.ard, sath.irtifa) else {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: ism.to_owned(),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        };
        self.khattaf.iltaqit(bikselat)
    }

    /// Uploads a new glyph atlas.
    ///
    /// # Errors
    ///
    /// As [`Khattaf::arfa_lawha`].
    pub fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        self.khattaf.arfa_lawha(bayt, ard, irtifa)
    }

    /// Shuts the overlay down and releases the backend's resources.
    ///
    /// # Errors
    ///
    /// As [`Khattaf::ahmil`], which is reported rather than acted on.
    pub fn aghliq(&mut self) -> Result<(), KhataTabaqa> {
        self.hala = HalatTabaqa::Muattala;
        self.sath = None;
        self.athar.push("shut down".to_owned());
        self.khattaf.ahmil()
    }
}
