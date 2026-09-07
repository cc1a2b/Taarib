//! إصدار — which Godot this is, and therefore which of the two adapters runs.
//!
//! Every other engine in this product has one strategy with version-specific
//! corrections. Godot has **two strategies that share nothing but a package
//! format**, and this module is the switch between them. Getting it wrong is not
//! a degraded translation; it is running a full text takeover against an engine
//! that shapes correctly by itself, or declining to take over an engine that
//! cannot shape at all.
//!
//! So the answer is drawn from the package header, which every Godot game has
//! and which states the engine's major version as a field rather than as an
//! inference. Phase 5 already reads that header to identify the game in the
//! first place, including the awkward parts — the trailing record of a package
//! embedded in an executable, the format-version-against-engine-version
//! disagreement that a repacked game produces — so this module calls into
//! [`taarib_muharrik::dalail::godot`] rather than restating any of it.
//!
//! What this module adds is the decision: given a generation, which delivery
//! this adapter can offer, and what to tell the user before they install.
//!
//! ## Why the generation is not guessed from the executable
//!
//! It could be: Godot 3 and Godot 4 export templates have different exported
//! symbols and different embedded version strings. But a Godot game is very
//! often shipped as an executable with the package embedded inside it, and the
//! package header is *already being read* to find the content — so taking the
//! generation from the same read costs nothing and cannot disagree with itself.
//! Two sources for one fact is two chances to be inconsistent.

use std::path::{Path, PathBuf};

use taarib_muharrik::dalail::godot::{FahisGodot, JeelGodot, TarwisatPck};
use taarib_mustalahat::muharrik::IsdarMuharrik;

use crate::khata::KhataGodot;

/// Which of this crate's two adapters applies, and how it is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Masar {
    /// Godot 4: register a font, set a locale, and stop.
    ///
    /// The engine's `TextServerAdvanced` shapes correctly, so Taarib adds a
    /// font and a translation and changes nothing about how text is drawn.
    /// Delivered as a `GDExtension` library plus an additive resource pack.
    Khadim,

    /// Godot 3, by interception: Taarib shapes, positions and draws.
    ///
    /// The engine has no text server at all, so this is a full takeover through
    /// hooks on the drawing path.
    Istila,

    /// Godot 3, by glyph transport: for an export template that can be neither
    /// extended nor hooked.
    ///
    /// The last resort, and the one that costs the user something real —
    /// transported text is not searchable, selectable or copyable. Never chosen
    /// while [`Masar::Istila`] is available.
    Naql,
}

impl Masar {
    /// The name for a log line and the capability report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Khadim => "Godot 4 text server",
            Self::Istila => "Godot 3 takeover",
            Self::Naql => "Godot 3 glyph transport",
        }
    }

    /// What a player should be told about this path before they install.
    ///
    /// The transport's sentence names its cost plainly. A user who installs a
    /// patch and later discovers they cannot copy a quest name out of the game
    /// has been surprised by something they should have been told, and burying
    /// it would be the kind of omission this product does not make.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Khadim => {
                "The engine shapes Arabic correctly on its own. Taarib supplies the font and \
                 the translation and changes nothing else, so the text behaves exactly like \
                 the game's own."
            },
            Self::Istila => {
                "This version of the engine cannot shape Arabic, so Taarib draws the text \
                 itself. The result is correct and behaves normally on screen."
            },
            Self::Naql => {
                "This game's build can be neither extended nor intercepted, so Arabic is \
                 delivered as a replacement glyph table. The text will look correct and will \
                 NOT be searchable, selectable or copyable inside the game."
            },
        }
    }

    /// Whether this path costs the user a capability they had before.
    #[must_use]
    pub const fn yukallif(self) -> bool {
        matches!(self, Self::Naql)
    }
}

/// Everything this adapter needs to know before it acts.
#[derive(Debug, Clone)]
pub struct Bina {
    /// The game's root directory.
    pub jidhr: PathBuf,
    /// Where the package is — a `.pck` beside the executable, or the executable
    /// itself when the package is embedded.
    pub hazma: PathBuf,
    /// The package header, as Phase 5 read it.
    pub tarwisa: TarwisatPck,
    /// Which engine generation this is.
    pub jeel: JeelGodot,
    /// The engine version, when the header stated a usable one.
    pub isdar: Option<IsdarMuharrik>,
    /// Whether the package is encrypted, when that can be told.
    pub mushaffara: Option<bool>,
    /// What each step reported, for the diagnostics bundle.
    pub athar: Vec<String>,
}

impl Bina {
    /// Whether the package is embedded inside the executable.
    ///
    /// Consequential for delivery rather than for reading: an embedded package
    /// is never modified, and the patch pack is mounted alongside it. A patch
    /// that rewrote an executable would be a patch that trips every integrity
    /// check a platform has.
    #[must_use]
    pub const fn mudmaja(&self) -> bool {
        self.tarwisa.mudmaja
    }

    /// Which adapter applies, given what the process can reach.
    ///
    /// `yumkin_khatf` is the caller's answer to "can this build be hooked?",
    /// which only the injected side can establish — a stripped, statically
    /// linked, custom-built export template resolves nothing, and that is
    /// exactly the case the transport exists for. Passing it in rather than
    /// deciding it here keeps this module free of process-inspection code and
    /// keeps the decision in one place.
    #[must_use]
    pub const fn masar(&self, yumkin_khatf: bool) -> Masar {
        match self.jeel {
            JeelGodot::Rabi => Masar::Khadim,
            JeelGodot::Thalith if yumkin_khatf => Masar::Istila,
            JeelGodot::Thalith => Masar::Naql,
        }
    }

    /// The build as a sentence, for a log line.
    #[must_use]
    pub fn wasf(&self) -> String {
        let isdar = self
            .isdar
            .as_ref()
            .map_or_else(|| self.jeel.ism().to_owned(), |isdar| isdar.khaam.clone());
        format!(
            "{isdar}, {} package{}{}",
            if self.mudmaja() {
                "embedded"
            } else {
                "external"
            },
            match self.mushaffara {
                Some(true) => ", encrypted",
                Some(false) => "",
                None => ", encryption unknown",
            },
            if self.tarwisa.tanaqud() {
                " (the package's format and engine fields disagree, which a repacked game does)"
            } else {
                ""
            }
        )
    }
}

/// Establishes which Godot this is.
///
/// # Errors
///
/// [`KhataGodot::IsdarMajhul`] when no package could be found or its header
/// could not be read, and [`KhataGodot::MuharrikGhayrMadum`] when the header
/// states a generation this adapter has no strategy for — Godot 2, or a version
/// beyond 4 whose text server this build has not been taught about.
///
/// Refusing rather than guessing is deliberate here in a way it is not
/// elsewhere in this product: the two strategies are mutually exclusive and
/// running the wrong one is worse than running neither.
pub fn afhas(jidhr: &Path, tanfidhi: Option<&Path>) -> Result<Bina, KhataGodot> {
    let mut athar = Vec::new();

    let Some((hazma, tarwisa)) = FahisGodot::hazma(jidhr, tanfidhi) else {
        return Err(KhataGodot::IsdarMajhul {
            sabab: format!(
                "no Godot package was found under {} and none was embedded in the executable",
                jidhr.display()
            ),
        });
    };
    athar.push(format!(
        "package at {} ({}), format version {}, engine {}.{}.{}",
        hazma.display(),
        if tarwisa.mudmaja {
            "embedded"
        } else {
            "external"
        },
        tarwisa.sigha,
        tarwisa.kabir,
        tarwisa.sagheer,
        tarwisa.tasheeh
    ));

    let Some(jeel) = tarwisa.jeel() else {
        return Err(KhataGodot::MuharrikGhayrMadum {
            wujid: format!("{}.{}.{}", tarwisa.kabir, tarwisa.sagheer, tarwisa.tasheeh),
        });
    };
    athar.push(format!("{}: {}", jeel.ism(), jeel.wasf()));

    if tarwisa.tanaqud() {
        // Worth a line rather than a refusal. A repacked game legitimately has
        // a package written by one tool and an engine field from another, and
        // the generation is taken from the engine field because that is what
        // will actually run the code.
        athar.push(
            "the package's format version and engine version disagree, which is what a \
             repacked or third-party-tooled game looks like; the engine field wins"
                .to_owned(),
        );
    }

    let mushaffara = tarwisa.mushaffara();
    if mushaffara == Some(true) {
        athar.push("the package is encrypted; reading it needs the game's own key".to_owned());
    }

    let isdar = tarwisa.isdar();
    Ok(Bina {
        jidhr: jidhr.to_path_buf(),
        hazma,
        tarwisa,
        jeel,
        isdar,
        mushaffara,
        athar,
    })
}
