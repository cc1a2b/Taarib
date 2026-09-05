//! القدرة — what the overlay will be able to do here, decided before it is
//! enabled rather than discovered while somebody is playing.
//!
//! The four backends that came first could afford to answer this implicitly:
//! DXGI and Vulkan compose through the game's own device on every presentation
//! mode there is, so "can the overlay draw over this game?" had one answer and
//! it was yes. The three older APIs do not have that luxury, and each of them
//! fails in a way the player cannot attribute to anything:
//!
//! * a fixed-function OpenGL game whose `wglSwapBuffers` runs on a thread with
//!   **no current context** takes every overlay call as a no-op and presents a
//!   frame with nothing on it, sixty times a second, silently;
//! * a colour-index OpenGL pixel format has no path from a premultiplied RGBA
//!   atlas to the screen at all;
//! * a Direct3D 8 game presenting a **multisampled** backbuffer can be drawn
//!   over and cannot be read, because that API has no resolve;
//! * a Direct3D 10 swap chain whose device is really Direct3D 11 must be left
//!   to the eleventh backend rather than half-composed by the tenth.
//!
//! So this module is the shape of that answer: a verdict, the presentation mode
//! it was reached about, and a list of reasons in both languages. Nothing here
//! decides anything on its own — each backend produces one of these — and
//! nothing here is a dialog. It is a value the interface renders on the screen
//! where the user chooses whether to turn the overlay on, which is the only
//! place a limitation of this kind can honestly appear.
//!
//! ## Why the reasons are carried rather than formatted
//!
//! A verdict with a single sentence attached would force every producer to
//! decide which of its findings mattered most. A game can be both "this device
//! is a community wrapper" and "this backbuffer cannot be read", and those are
//! two different sentences with two different remedies. So the reasons are a
//! list, each carrying its own verdict, and [`QudratTarkeeb::hukm`] is the
//! worst of them.

use core::fmt;

use crate::wajiha::WajihatRusum;

/// What the overlay will be able to do on this API, in this process.
///
/// Ordered from best to worst, and [`Ord`] is derived so that the worst of a
/// list is `max()` and nothing has to spell the precedence twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HukmQudra {
    /// The overlay will draw, read the screen, and restore what it changed.
    Kamila,
    /// The overlay will draw, and something it normally does it will not do.
    ///
    /// The reason says which. A game whose backbuffer cannot be read is the
    /// common case: Arabic can be drawn over it, and nothing can be recognised
    /// on it, which for a tier that reads the screen means the overlay is a
    /// control panel and no translation.
    Naqisa,
    /// The overlay will not be able to draw at all.
    Mustaheela,
}

impl HukmQudra {
    /// The word the interface shows beside the verdict.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Kamila => "supported",
            Self::Naqisa => "supported with limits",
            Self::Mustaheela => "not supported",
        }
    }

    /// The same word in Arabic.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self {
            Self::Kamila => "مدعومة",
            Self::Naqisa => "مدعومة بحدود",
            Self::Mustaheela => "غير مدعومة",
        }
    }

    /// Whether the overlay may be offered at all.
    #[must_use]
    pub const fn qabila(self) -> bool {
        !matches!(self, Self::Mustaheela)
    }
}

impl fmt::Display for HukmQudra {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj.write_str(self.ism())
    }
}

/// One finding, in both languages, with its own verdict.
///
/// Both languages are carried rather than one being derived from the other,
/// because these are shown to a player rather than logged: an English sentence
/// machine-translated into Arabic on the screen where somebody decides whether
/// to install an Arabic patch would be the least convincing thing in the
/// product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SababQudra {
    /// What this finding means for the overlay.
    pub hukm: HukmQudra,
    /// The sentence in Arabic.
    pub arabi: String,
    /// The same sentence in English.
    pub injilizi: String,
}

impl SababQudra {
    /// A finding that costs nothing.
    #[must_use]
    pub fn kamila(arabi: impl Into<String>, injilizi: impl Into<String>) -> Self {
        Self { hukm: HukmQudra::Kamila, arabi: arabi.into(), injilizi: injilizi.into() }
    }

    /// A finding that narrows what the overlay will do.
    #[must_use]
    pub fn naqisa(arabi: impl Into<String>, injilizi: impl Into<String>) -> Self {
        Self { hukm: HukmQudra::Naqisa, arabi: arabi.into(), injilizi: injilizi.into() }
    }

    /// A finding that stops the overlay.
    #[must_use]
    pub fn mustaheela(arabi: impl Into<String>, injilizi: impl Into<String>) -> Self {
        Self { hukm: HukmQudra::Mustaheela, arabi: arabi.into(), injilizi: injilizi.into() }
    }
}

/// Whether an exclusive-fullscreen game can be composed over, and why.
///
/// The question that separates a device-level overlay from a window-level one,
/// and the reason this is a field rather than another [`SababQudra`]: it has
/// the same answer for every game on a given API, it is the single most common
/// thing anybody asks about an overlay, and an interface should be able to
/// state it without reading a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilShasha {
    /// Composition happens through the game's own device, so the presentation
    /// mode is irrelevant.
    ///
    /// True on every API in this crate. The overlay draws into the backbuffer
    /// the game is about to present, before it presents it, so there is no
    /// second window for the compositor to drop and nothing for exclusive mode
    /// to exclude. This is what a layered window over a fullscreen game cannot
    /// do, and it is why this tier hooks presentation rather than creating a
    /// window of its own.
    KhilalAlJihaz,
    /// There is no drawable to compose into on this platform at all.
    LaShay,
}

impl MilShasha {
    /// The sentence the interface shows about exclusive fullscreen.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::KhilalAlJihaz => {
                "the overlay draws into the frame the game is about to present, so exclusive \
                 fullscreen is composed over exactly as windowed mode is"
            }
            Self::LaShay => "this platform has no drawable for the overlay to compose into",
        }
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::KhilalAlJihaz => {
                "تُرسم الطبقة داخل الإطار الذي توشك اللعبة على عرضه، فوضع ملء الشاشة الحصري \
                 يُعامَل كوضع النافذة تمامًا."
            }
            Self::LaShay => "لا يوجد سطح رسم على هذه المنصة تُركَّب عليه الطبقة.",
        }
    }
}

/// Everything that can be established about one API before the overlay starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QudratTarkeeb {
    /// Which API this is about.
    pub wajiha: WajihatRusum,
    /// How exclusive fullscreen behaves here.
    pub mil_shasha: MilShasha,
    /// Every finding, in the order they were made.
    pub asbab: Vec<SababQudra>,
}

impl QudratTarkeeb {
    /// A report with no findings yet.
    #[must_use]
    pub const fn jadeeda(wajiha: WajihatRusum, mil_shasha: MilShasha) -> Self {
        Self { wajiha, mil_shasha, asbab: Vec::new() }
    }

    /// Adds a finding.
    #[must_use]
    pub fn maa(mut self, sabab: SababQudra) -> Self {
        self.asbab.push(sabab);
        self
    }

    /// The worst of the findings, which is the verdict.
    ///
    /// [`HukmQudra::Kamila`] when there are none, because a report with nothing
    /// to say about an API this build implements is a report that found nothing
    /// wrong — not a report that found nothing.
    #[must_use]
    pub fn hukm(&self) -> HukmQudra {
        self.asbab.iter().map(|sabab| sabab.hukm).max().unwrap_or(HukmQudra::Kamila)
    }

    /// Whether the overlay may be offered for this API here.
    #[must_use]
    pub fn qabila(&self) -> bool {
        self.hukm().qabila()
    }

    /// Every finding at or above a verdict, for an interface that shows only
    /// what limits the user.
    pub fn asbab_min(&self, adna: HukmQudra) -> impl Iterator<Item = &SababQudra> {
        self.asbab.iter().filter(move |sabab| sabab.hukm >= adna)
    }

    /// The report as the lines a diagnostics bundle carries.
    #[must_use]
    pub fn sutur(&self) -> Vec<String> {
        let mut sutur = vec![
            format!("{}: {}", self.wajiha.ism(), self.hukm()),
            format!("{}: {}", self.wajiha.ism(), self.mil_shasha.wasf()),
        ];
        sutur.extend(
            self.asbab
                .iter()
                .map(|sabab| format!("{}: {} — {}", self.wajiha.ism(), sabab.hukm, sabab.injilizi)),
        );
        sutur
    }
}
