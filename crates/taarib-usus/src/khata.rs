//! نموذج الأخطاء — the error model.
//!
//! Every failure in Taarib is a value carrying four things a user or a
//! maintainer will eventually need:
//!
//! 1. a stable machine code ([`Ramz`], rendered `TAARIB-E-1042`),
//! 2. a sentence in Arabic, written for the person using the application,
//! 3. the same sentence in English,
//! 4. a concrete next action ([`Khutwa`]) the interface can turn into a button.
//!
//! Nothing here formats an error into a string at construction time. Context
//! is captured as structured values ([`QeemaSiyaq`]) and causes nest as values
//! ([`Khata::sabab`]), so the same error renders three different ways — one
//! line for a log, a full chain for a diagnostics bundle, and a plain sentence
//! for a person — without any of the three losing information the others kept.
//!
//! Concrete errors are declared per domain as ordinary enums implementing
//! [`Tafsir`], then converted at the crate boundary with [`khata_min`].

use std::collections::BTreeMap;
use std::fmt;
use std::panic::Location;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Lugha;

/// The result type used by every fallible function in Taarib.
pub type Natija<T> = Result<T, Khata>;

/// The error-code registry.
///
/// Codes are permanent. A code that has been shipped is never reused for a
/// different meaning, because users paste them into bug reports and search for
/// them years later. Each domain owns a block; a new error takes the next free
/// number in its own block and nothing else.
pub mod arqam {
    /// `taarib-usus` — paths and the on-disk layout.
    pub const MASARAT: u16 = 1000;
    /// `taarib-usus` — configuration.
    pub const IDADAT: u16 = 1100;
    /// `taarib-usus` — schema versioning and migration.
    pub const MUKHATTAT: u16 = 1200;
    /// `taarib-usus` — platform, process and compatibility layer.
    pub const MANASSA: u16 = 1300;
    /// `taarib-usus` — diagnostics and logging.
    pub const SIJILL: u16 = 1400;
    /// `taarib-saff` — the Arabic text engine.
    pub const SAFF: u16 = 2000;
    /// `taarib-saff` — font resources and validation.
    pub const KHATT: u16 = 2500;
    /// `taarib-lawha` — the glyph atlas.
    pub const LAWHA: u16 = 3000;
    /// `taarib-jisr` — the native ABI boundary.
    pub const JISR: u16 = 3100;
    /// `taarib-makhzan` — the local database.
    pub const MAKHZAN: u16 = 3200;
    // 3300 was `taarib-barid`, an inter-process transport that was never built
    // and was deleted in Phase 23: an adapter hands a capture session to Studio
    // as a file, which `taarib-istikhraj::iltiqat` reads. The number is left
    // unallocated rather than reused, so an old log line naming a 33xx code
    // stays traceable to something that no longer exists.
    /// `taarib-kashf` — game discovery.
    pub const KASHF: u16 = 4000;
    /// `taarib-muharrik` — engine identification.
    pub const MUHARRIK: u16 = 4100;
    /// `taarib-haqn` — injection and framework installation.
    pub const HAQN: u16 = 4200;
    /// The engine adapters.
    pub const MUHAWWIL: u16 = 4300;
    /// `taarib-istikhraj` — text extraction.
    pub const ISTIKHRAJ: u16 = 5000;
    /// `taarib-tarjama` — the translation pipeline.
    pub const TARJAMA: u16 = 5100;
    /// `taarib-ruqaa` — the patch container format.
    pub const RUQAA: u16 = 6000;
    /// `taarib-tarqee` — the patch compiler.
    pub const TARQEE: u16 = 6100;
    /// `taarib-tathbeet` — installation and rollback.
    pub const TATHBEET: u16 = 6200;
    /// `taarib-aman` — the safety layer.
    pub const AMAN: u16 = 6300;
    /// `taarib-khatm` — signing and verification.
    pub const KHATM: u16 = 6400;
    /// `taarib-mustawda` — the registry client.
    pub const MUSTAWDA: u16 = 7000;
    /// `taarib-taqdeem` — submission and review.
    pub const TAQDEEM: u16 = 7100;
    /// `taarib-warsha` — the collaborative workspace.
    pub const WARSHA: u16 = 7200;
    /// `taarib-tabaqa` — the universal graphics overlay.
    pub const TABAQA: u16 = 8000;
    /// `taarib-tahdith` — application self-update.
    pub const TAHDITH: u16 = 8100;
    /// `taarib-tajmee` — artifact staging.
    pub const TAJMEE: u16 = 8200;
    /// `taarib-tilqai` — the one-button automatic pipeline.
    pub const TILQAI: u16 = 8300;
    /// `taarib-studio` — the desktop application itself.
    pub const STUDIO: u16 = 9000;
}

/// A stable, permanent error code.
///
/// Renders as `TAARIB-E-1042`. The numeric part is allocated from the block
/// its domain owns in [`arqam`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(
    feature = "mukhattatat",
    derive(schemars::JsonSchema),
    schemars(extend("pattern" = r"^TAARIB-E-\d{4}$"))
)]
pub struct Ramz(
    // The wire form is the rendered code, `TAARIB-E-1042`, not the number:
    // a code is an identifier users copy and search for, and a bare integer in
    // a log or a bug report is worth nothing to them.
    #[cfg_attr(feature = "wajiha", specta(type = String))]
    #[cfg_attr(feature = "mukhattatat", schemars(with = "String"))]
    u16,
);

impl Ramz {
    /// Builds a code from its numeric part.
    #[must_use]
    pub const fn jadeed(raqm: u16) -> Self {
        Self(raqm)
    }

    /// The numeric part, for grouping and comparison.
    #[must_use]
    pub const fn raqm(self) -> u16 {
        self.0
    }

    /// The domain block this code belongs to, useful for routing a failure to
    /// the screen that can act on it.
    #[must_use]
    pub const fn kutla(self) -> u16 {
        // Truncating division is the operation, not a rounding accident: 4213
        // belongs to band 4200, and the remainder is exactly what is being
        // discarded.
        self.0.saturating_sub(self.0 % 100)
    }
}

impl fmt::Display for Ramz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TAARIB-E-{:04}", self.0)
    }
}

impl Serialize for Ramz {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Ramz {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let nass = String::deserialize(d)?;
        let raqm = nass
            .rsplit('-')
            .next()
            .and_then(|t| t.parse::<u16>().ok())
            .ok_or_else(|| serde::de::Error::custom("expected TAARIB-E-NNNN"))?;
        Ok(Self(raqm))
    }
}

/// How badly a failure hurts, which decides how loudly the interface says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Khutura {
    /// Worth recording, invisible to the user.
    Maluma,
    /// The operation continued with a reduced result the user should know about.
    Tanbeeh,
    /// The operation failed. The user asked for something and did not get it.
    Khatar,
    /// The operation failed and left something that needs attention — a partial
    /// install, an unrestored backup, a corrupted store.
    Fadih,
}

/// Which part of Settings a [`Khutwa::FathIdadat`] should open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum QismIdadat {
    /// Launcher locations.
    Manassat,
    /// Fonts.
    Khutut,
    /// Machine translation providers and credentials.
    Muzawwidun,
    /// Where patches are stored.
    Takhzin,
    /// Registry sources, mirrors and offline shares.
    Masadir,
    /// Update behaviour.
    Tahdith,
    /// Interface language and digits.
    Lugha,
    /// Diagnostics level and log retention.
    Tashkhis,
    /// Arabization behaviour: whether a patch replaces the game's official
    /// language rather than sitting beside it.
    Taareeb,
}

/// What kind of path the user is being asked to point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MasarMatlub {
    /// The folder a game is installed in.
    MujalladLuba,
    /// A game's executable.
    MalafTanfidhi,
    /// A launcher's installation folder.
    MujalladManassa,
    /// Where Taarib keeps downloaded patches.
    MujalladRuqaa,
    /// A font file.
    MalafKhatt,
    /// A patch file to import.
    MalafRuqaa,
    /// A recorded capture session.
    ///
    /// Written by a play-through with capture armed, and read back by the
    /// automatic run. Distinct from every other target here because it is not a
    /// place a game or a font lives: it is one run's own recording, and the
    /// screen that offers it is that game's automatic run.
    MalafJalsa,
}

/// The one concrete thing the user can do next.
///
/// This is not advice text — it is a value the interface turns into a button,
/// so an error can never arrive with nothing actionable attached to it.
///
/// Three of these name something only the user can do, and only outside Taarib:
/// [`Khutwa::TahaqquqSalamatLuba`] belongs to the game's own launcher,
/// [`Khutwa::ManhSalahiya`] to the operating system, and
/// [`Khutwa::IblaghLilMusahim`] to whatever the contributor published a way to
/// be reached by — a patch listing carries a display name and no address. The
/// interface states those three as their own directive rather than dressing
/// them as controls it cannot make perform anything. Every other value here
/// resolves to a control that does the thing, and adding one that does not is
/// how this type stops meaning what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum Khutwa {
    /// Nothing to do; the message is complete on its own.
    LaShay,
    /// Try the same operation again — transient network or file lock.
    AadaMuhawala,
    /// Rescan the library.
    AadaFahsMaktaba,
    /// Re-probe this game's engine.
    AadaFahsMuharrik,
    /// Point Taarib at a path it could not find.
    IkhtiyarMasar {
        /// What the user should choose.
        matlub: MasarMatlub,
    },
    /// Choose a different font — the current one cannot render Arabic.
    IkhtiyarKhattAakhar,
    /// Open a section of Settings.
    FathIdadat {
        /// The section to open.
        qism: QismIdadat,
    },
    /// Open Diagnostics.
    FathTashkhis,
    /// Open the overflow report for the current patch or submission.
    FathTaqreerTajawuz,
    /// Open the offending strings in the workspace.
    FathNusus,
    /// Open this game's overlay: its capture regions and its reading history.
    ///
    /// The tier-3 answer, offered wherever a game is outside what the adapters
    /// reach or a stored region no longer fits the surface it was drawn on.
    FathTabaqa,
    /// Open this game's automatic run.
    FathTilqai,
    /// Update Taarib itself — the data is newer than this build understands.
    TahdithTaarib,
    /// Reinstall the framework for this game.
    IadatTarkibIttar,
    /// Uninstall the patch and restore the game.
    IlghaTathbeet,
    /// Re-match the patch against the game's new build.
    IadatMutabaqaBina,
    /// Verify the game's files through its own launcher.
    TahaqquqSalamatLuba,
    /// Contact the patch's contributor.
    IblaghLilMusahim,
    /// Send a diagnostics bundle to the project owner.
    IblaghLilMalik,
    /// Free disk space and retry.
    TahrirMasaha,
    /// Grant Taarib the permission the operating system refused.
    ManhSalahiya,
}

/// A structured context value attached to an error.
///
/// Context is captured as data, never as an interpolated sentence, so a log
/// filter can match on it, a diagnostics bundle can present it as a table, and
/// the interface can render a path as a clickable path rather than as text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", content = "qeema", rename_all = "snake_case")]
pub enum QeemaSiyaq {
    /// Free text: a name, an identifier, a reason reported by another system.
    Nass(String),
    /// A signed number.
    Raqm(#[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))] i64),
    /// A byte count, rendered with a unit and the user's digits.
    Hajm(#[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))] u64),
    /// A fraction or a measurement in pixels.
    Kasr(f64),
    /// A flag.
    Munt(bool),
    /// A filesystem path, rendered isolated left-to-right inside Arabic text.
    Masar(PathBuf),
    /// A list of names — files that failed, strings that overflowed.
    Qaima(Vec<String>),
}

impl From<&str> for QeemaSiyaq {
    fn from(q: &str) -> Self {
        Self::Nass(q.to_owned())
    }
}

impl From<String> for QeemaSiyaq {
    fn from(q: String) -> Self {
        Self::Nass(q)
    }
}

impl From<i64> for QeemaSiyaq {
    fn from(q: i64) -> Self {
        Self::Raqm(q)
    }
}

impl From<u32> for QeemaSiyaq {
    fn from(q: u32) -> Self {
        Self::Raqm(i64::from(q))
    }
}

impl From<usize> for QeemaSiyaq {
    fn from(q: usize) -> Self {
        Self::Hajm(q as u64)
    }
}

impl From<u64> for QeemaSiyaq {
    fn from(q: u64) -> Self {
        Self::Hajm(q)
    }
}

impl From<f64> for QeemaSiyaq {
    fn from(q: f64) -> Self {
        Self::Kasr(q)
    }
}

impl From<bool> for QeemaSiyaq {
    fn from(q: bool) -> Self {
        Self::Munt(q)
    }
}

impl From<&Path> for QeemaSiyaq {
    fn from(q: &Path) -> Self {
        Self::Masar(q.to_path_buf())
    }
}

impl From<PathBuf> for QeemaSiyaq {
    fn from(q: PathBuf) -> Self {
        Self::Masar(q)
    }
}

impl From<Vec<String>> for QeemaSiyaq {
    fn from(q: Vec<String>) -> Self {
        Self::Qaima(q)
    }
}

impl fmt::Display for QeemaSiyaq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nass(q) => f.write_str(q),
            Self::Raqm(q) => write!(f, "{q}"),
            Self::Hajm(q) => write!(f, "{q}"),
            Self::Kasr(q) => write!(f, "{q}"),
            Self::Munt(q) => write!(f, "{q}"),
            Self::Masar(q) => write!(f, "{}", q.display()),
            Self::Qaima(q) => write!(f, "[{}]", q.join(", ")),
        }
    }
}

/// Where in the source an error was constructed, captured automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Mawqi {
    /// The source file, relative to the workspace root.
    pub malaf: String,
    /// The line number.
    pub satr: u32,
}

impl Mawqi {
    #[track_caller]
    fn hali() -> Self {
        let l = Location::caller();
        Self {
            malaf: l.file().to_owned(),
            satr: l.line(),
        }
    }
}

impl fmt::Display for Mawqi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.malaf, self.satr)
    }
}

/// A failure, ready to be logged, bundled, or shown to a person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Khata {
    /// The permanent code.
    pub ramz: Ramz,
    /// How badly it hurts.
    pub khutura: Khutura,
    /// The sentence shown to an Arabic-speaking user.
    pub arabi: String,
    /// The same sentence in English.
    pub injilizi: String,
    /// The one thing the user can do next.
    pub khutwa: Khutwa,
    /// Structured context, deterministically ordered.
    pub siyaq: BTreeMap<String, QeemaSiyaq>,
    /// The failure underneath this one, as a value.
    ///
    /// Spelled `Khata` rather than `Self`: `specta::Type` expands into an item
    /// outside this one, where `Self` does not name anything.
    #[expect(clippy::use_self, reason = "see above")]
    pub sabab: Option<Box<Khata>>,
    /// Where this error was constructed.
    pub mawqi: Option<Mawqi>,
}

impl Khata {
    /// Builds an error from its four required parts.
    #[must_use]
    #[track_caller]
    pub fn jadeeda(
        ramz: u16,
        khutura: Khutura,
        arabi: impl Into<String>,
        injilizi: impl Into<String>,
        khutwa: Khutwa,
    ) -> Self {
        Self {
            ramz: Ramz::jadeed(ramz),
            khutura,
            arabi: arabi.into(),
            injilizi: injilizi.into(),
            khutwa,
            siyaq: BTreeMap::new(),
            sabab: None,
            mawqi: Some(Mawqi::hali()),
        }
    }

    /// Builds an error from anything that can explain itself.
    #[must_use]
    #[track_caller]
    pub fn min_tafsir<T: Tafsir + ?Sized>(tafsir: &T) -> Self {
        Self {
            ramz: tafsir.ramz(),
            khutura: tafsir.khutura(),
            arabi: tafsir.arabi(),
            injilizi: tafsir.injilizi(),
            khutwa: tafsir.khutwa(),
            siyaq: tafsir.siyaq(),
            sabab: None,
            mawqi: Some(Mawqi::hali()),
        }
    }

    /// Attaches a piece of structured context.
    #[must_use]
    pub fn ma(mut self, miftah: &str, qeema: impl Into<QeemaSiyaq>) -> Self {
        let _ = self.siyaq.insert(miftah.to_owned(), qeema.into());
        self
    }

    /// Nests the failure that caused this one.
    #[must_use]
    pub fn bi_sabab(mut self, sabab: Self) -> Self {
        self.sabab = Some(Box::new(sabab));
        self
    }

    /// Raises the severity, for a caller that knows the failure matters more
    /// than the layer that produced it could tell.
    #[must_use]
    pub const fn bi_khutura(mut self, khutura: Khutura) -> Self {
        self.khutura = khutura;
        self
    }

    /// Replaces the next action, for a caller that knows a better one.
    #[must_use]
    pub const fn bi_khutwa(mut self, khutwa: Khutwa) -> Self {
        self.khutwa = khutwa;
        self
    }

    /// Whether this error, or anything under it, carries the given code.
    #[must_use]
    pub fn yahmil(&self, ramz: Ramz) -> bool {
        self.ramz == ramz || self.sabab.as_ref().is_some_and(|s| s.yahmil(ramz))
    }

    /// The deepest failure in the chain — the thing that actually went wrong.
    #[must_use]
    pub fn asl(&self) -> &Self {
        self.sabab.as_ref().map_or(self, |s| s.asl())
    }

    /// One line for a log: code, English sentence, context, location.
    #[must_use]
    pub fn li_sijill(&self) -> String {
        use std::fmt::Write as _;

        let mut out = format!("{} {}", self.ramz, self.injilizi);
        for (k, v) in &self.siyaq {
            let _ = write!(out, " {k}={v}");
        }
        if let Some(m) = &self.mawqi {
            let _ = write!(out, " @{m}");
        }
        if let Some(s) = &self.sabab {
            out.push_str(" <- ");
            out.push_str(&s.li_sijill());
        }
        out
    }

    /// The full chain, indented, for a diagnostics bundle.
    #[must_use]
    pub fn li_hazma(&self) -> String {
        let mut out = String::new();
        self.aktub_hazma(&mut out, 0);
        out
    }

    fn aktub_hazma(&self, out: &mut String, umq: usize) {
        use std::fmt::Write as _;

        let badiya = "  ".repeat(umq);
        let _ = writeln!(
            out,
            "{badiya}{} [{:?}] {}",
            self.ramz, self.khutura, self.injilizi
        );
        let _ = writeln!(out, "{badiya}  ar: {}", self.arabi);
        if let Some(m) = &self.mawqi {
            let _ = writeln!(out, "{badiya}  at: {m}");
        }
        for (k, v) in &self.siyaq {
            let _ = writeln!(out, "{badiya}  {k}: {v}");
        }
        if let Some(s) = &self.sabab {
            let _ = writeln!(out, "{badiya}  caused by:");
            s.aktub_hazma(out, umq + 2);
        }
    }

    /// The rendering a person reads, in the language they chose.
    #[must_use]
    pub fn lil_mustakhdim(&self, lugha: Lugha) -> RisalatMustakhdim {
        RisalatMustakhdim {
            ramz: self.ramz,
            khutura: self.khutura,
            nass: match lugha {
                Lugha::Arabi => self.arabi.clone(),
                Lugha::Injilizi => self.injilizi.clone(),
            },
            khutwa: self.khutwa.clone(),
        }
    }
}

impl fmt::Display for Khata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.ramz, self.injilizi)
    }
}

impl std::error::Error for Khata {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.sabab
            .as_ref()
            .map(|s| -> &(dyn std::error::Error + 'static) { s.as_ref() })
    }
}

/// What the interface shows: a code, a severity, a sentence, and a button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct RisalatMustakhdim {
    /// The permanent code, shown small, copyable, always in Latin digits.
    pub ramz: Ramz,
    /// Drives the colour and the icon.
    pub khutura: Khutura,
    /// The sentence, in the user's language.
    pub nass: String,
    /// The action the button performs.
    pub khutwa: Khutwa,
}

/// Something that can explain itself to a person.
///
/// Domain crates implement this on their own error enums, keeping the variant
/// data structured, and convert at the boundary with [`khata_min`].
pub trait Tafsir: std::error::Error + Send + Sync + 'static {
    /// The permanent code for this variant.
    fn ramz(&self) -> Ramz;

    /// The Arabic sentence a user reads.
    fn arabi(&self) -> String;

    /// The English sentence a user reads.
    fn injilizi(&self) -> String;

    /// How badly this variant hurts. Defaults to a plain failure.
    fn khutura(&self) -> Khutura {
        Khutura::Khatar
    }

    /// What the user can do next. Defaults to nothing actionable, which every
    /// variant that can reach a screen is expected to override.
    fn khutwa(&self) -> Khutwa {
        Khutwa::LaShay
    }

    /// Structured context carried by this variant.
    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        BTreeMap::new()
    }
}

/// Generates `impl From<$t> for Khata` for a domain error enum.
///
/// A blanket `impl<T: Tafsir> From<T> for Khata` would collide with the
/// reflexive `From<T> for T` in core, so the conversion is generated per type
/// instead — which also keeps the conversion visible at the crate boundary
/// where it happens.
#[macro_export]
macro_rules! khata_min {
    ($($t:ty),+ $(,)?) => {
        $(
            impl ::core::convert::From<$t> for $crate::khata::Khata {
                #[track_caller]
                fn from(q: $t) -> Self {
                    $crate::khata::Khata::min_tafsir(&q)
                }
            }
        )+
    };
}

/// Builds structured context out of a [`std::io::Error`] without stringifying
/// it: the kind is captured as a stable name and the raw OS code is kept.
#[must_use]
pub fn siyaq_io(khata: &std::io::Error) -> BTreeMap<String, QeemaSiyaq> {
    let mut siyaq = BTreeMap::new();
    let _ = siyaq.insert(
        "io".to_owned(),
        QeemaSiyaq::Nass(format!("{:?}", khata.kind())),
    );
    if let Some(raw) = khata.raw_os_error() {
        let _ = siyaq.insert("os".to_owned(), QeemaSiyaq::Raqm(i64::from(raw)));
    }
    siyaq
}

/// Maps an I/O failure onto the action that actually resolves it, which is
/// almost never "try again": a missing file needs a path, a denied file needs a
/// permission, and a full disk needs space.
#[must_use]
pub fn khutwa_io(khata: &std::io::Error, matlub: MasarMatlub) -> Khutwa {
    use std::io::ErrorKind as E;
    match khata.kind() {
        E::NotFound => Khutwa::IkhtiyarMasar { matlub },
        E::PermissionDenied => Khutwa::ManhSalahiya,
        E::StorageFull | E::QuotaExceeded => Khutwa::TahrirMasaha,
        E::Interrupted | E::TimedOut | E::WouldBlock | E::ResourceBusy => Khutwa::AadaMuhawala,
        _ => Khutwa::FathTashkhis,
    }
}
