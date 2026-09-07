//! خادم النصوص — the Godot 4 path, which is configuration and nothing else.
//!
//! Godot 4 ships `TextServerAdvanced`: `HarfBuzz` for shaping, ICU for the
//! bidirectional algorithm and for line breaking, and it is correct. It joins
//! Arabic, it orders a mixed Arabic and Latin line the way the Unicode
//! bidirectional algorithm says to, and it positions marks from the font's own
//! `GPOS`. **So Taarib does not shape on this path at all.** There is no shaping
//! call in this module, no glyph loop, and no atlas lookup, and if one ever
//! appears here it is a bug rather than a feature: it would be a second,
//! worse implementation of something the engine already does correctly, and the
//! two would disagree the first time a font's `GSUB` did something clever.
//!
//! What is left once shaping is somebody else's problem is four settings:
//!
//! 1. A font that carries Arabic, created at runtime from the patch's own bytes.
//! 2. That font assigned as the theme's default and into whichever named theme
//!    types and entries the patch lists.
//! 3. A layout direction and a text direction on the controls that draw text.
//! 4. A translation the engine loads through its own `TranslationServer`, with
//!    the locale set to `ar`.
//!
//! ## Where the bytes come from, and where they emphatically do not
//!
//! [`KhattRuqaa`] holds `Vec<u8>` and has no constructor that takes a path.
//! That is Decision 5 made structural rather than promised: this module cannot
//! open a game's font because there is no code path through which a file name
//! could reach it. The bytes arrive from the patch, are handed to Godot as a
//! `FontFile` built in memory, and the game's own font files are never opened,
//! enumerated, replaced or read. The same is true of the translation: the `pck`
//! module generates the `.translation` resource and this module receives it as
//! a byte slice.
//!
//! The one file this module writes is [`MALAF_TAJAWUZ`], which is Godot's own
//! documented user-side override for project settings. It is not a game asset,
//! it is not inside any `.pck`, the engine creates no such file itself, and
//! uninstalling is deleting it. If a file of that name already exists and does
//! not carry [`ALAMAT_TAARIB`], this module refuses to touch it — somebody
//! else's override is somebody else's, and clobbering it would break a game in
//! a way that has nothing to do with Arabic.
//!
//! ## The ladder, cheapest first
//!
//! The same shape as the Unreal adapter's, for the same reason: the rung that
//! needs nothing inside the process is the rung that survives an anti-tamper
//! system, a game update, and a player who does not want a library injected
//! into their game.
//!
//! 1. **Configuration.** [`MALAF_TAJAWUZ`] beside the executable. Godot reads it
//!    during `ProjectSettings` initialisation and every key in it overrides the
//!    matching key in the packed `project.godot`. Locale, translation list,
//!    root layout direction, text driver and default theme font are all plain
//!    project settings, so this one file carries the entire patch on a game that
//!    honours it.
//! 2. **Command line.** `--language ar`, which is a documented Godot launch
//!    option and outlives a launcher that regenerates its configuration. This
//!    rung is honestly thin: Godot 4 has no launch option that sets an arbitrary
//!    project setting, so the command line carries the locale and nothing else.
//!    The font and the theme cannot be reached this way at all.
//! 3. **The extension.** A `GDExtension` registered at the scene initialisation
//!    level, doing the same four things through the live engine, for the game
//!    that ships its own `override.cfg`, or applies saved settings after start,
//!    or was exported with settings baked past the point where an override
//!    applies.
//!
//! Every rung is verified by reading the value back where a read-back exists,
//! and [`SijillGodot`] records the rung whose effect was *confirmed* rather than
//! the rung that was attempted. Four separate records, one per concern, because
//! "the font took and the locale did not" is the common outcome and a single
//! verdict would hide it.
//!
//! ## The seam
//!
//! [`Musajjil`] is this module's only view of a running game. `imtidad.rs`
//! implements it over the `GDExtension` interface; nothing here knows how. Three
//! methods are required — is there a live engine, set a project setting, read
//! one back — and everything else carries a default body that refuses, so a
//! binding that reached `ProjectSettings` and nothing else still compiles and
//! still drives rungs one and two, which are the preferred rungs anyway.

use std::path::{Path, PathBuf};

use taarib_usus::masarat;

use crate::khata::{KhataGodot, tul_u64};

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// The locale the patch registers and asks for.
///
/// `ar`, not `ar_SA` and not `ar_001`. Godot's `TranslationServer` resolves a
/// requested locale by stripping subtags from the right, so every `ar_XX`
/// request passes through `ar` and no `ar_XX` request passes through any other
/// regional spelling. Registering `ar_SA` would be invisible to a player whose
/// system reports `ar_EG`, and asking for it would additionally select Saudi
/// number and date formatting for somebody in Morocco — a policy decision this
/// adapter is not allowed to make.
pub const WASM_ARABI: &str = "ar";

/// Accepted when the game already spells Arabic this way. Never introduced.
pub const WASM_ARABI_SA: &str = "ar_SA";

/// CLDR's Modern Standard Arabic. Accepted, never introduced.
pub const WASM_ARABI_ALAMI: &str = "ar_001";

// ---------------------------------------------------------------------------
// Project settings
// ---------------------------------------------------------------------------

/// Forces the locale the project runs in.
///
/// Godot's `TranslationServer::init` reads this before it reads the system
/// locale, so a non-empty value here is the locale the game starts in whatever
/// the player's system says. Documented around testing, applied unconditionally
/// in the engine's own source, and that is why it is rung one's main lever.
pub const MIFTAH_THAQAFA_IKHTIBAR: &str = "internationalization/locale/test";

/// The locale used when the system locale has no translation.
///
/// Set alongside the forced locale so that a build which ignores the forced one
/// — or a player who changes it back through the game's own menu and then to
/// something unsupported — still lands in Arabic rather than in the game's
/// original language.
pub const MIFTAH_THAQAFA_IHTIYAT: &str = "internationalization/locale/fallback";

/// The list of `.translation` resources the engine loads at start.
///
/// A `PackedStringArray`. Additive in effect but not in syntax: the value
/// replaces the whole list, so rung one writes the game's own entries back
/// beside the patch's — and writes the key **only** when the caller has read
/// them. See [`TarjamatLuba`] for why "not read" is not an empty list.
pub const MIFTAH_TARJAMAT: &str = "internationalization/locale/translations";

/// The layout direction the root window starts in.
///
/// **Not the same numbering as [`IttijahTakhtit`].** This setting has no
/// "inherited" case — the root inherits from nothing — so its values are
/// shifted by one against the `Control` enum: see [`IttijahJidhr`]. Writing a
/// `Control` value into this key sets left-to-right on a game that asked for
/// locale-driven mirroring, which looks exactly like the patch not being
/// installed.
pub const MIFTAH_ITTIJAH_JIDHR: &str = "internationalization/rendering/root_node_layout_direction";

/// Mirrors every control regardless of locale. Deliberately never written.
///
/// Named here so that the next person to look for it finds the reason rather
/// than the setting. It mirrors controls the game positions by hand as well as
/// the ones it lays out, and it does so in every locale, including the game's
/// original one. That is a change to a game's user interface that Arabic does
/// not require: with the locale set to `ar` and the root direction left on
/// [`IttijahJidhr::Thaqafa`], Godot mirrors what it laid out and leaves alone
/// what it did not.
pub const MIFTAH_ITTIJAH_QASRI: &str =
    "internationalization/rendering/force_right_to_left_layout_direction";

/// Selects which text server the engine builds.
///
/// The value is the server's own reported name, and the one that matters is
/// [`ISM_KHADIM_MUTAQADDIM`]. A game exported with the fallback server has no
/// shaping and no bidi at all, and on such a build this whole path is the wrong
/// path — the patch is a Godot 3 shaped patch wearing a Godot 4 engine.
pub const MIFTAH_KHADIM: &str = "internationalization/rendering/text_driver";

/// The default font for every control the theme reaches.
///
/// A resource path, and the resource lives in the patch's own `.pck`, mounted
/// over the game's at a higher priority. It is never a path into the game's own
/// assets: rung one points at what the patch shipped, and rung three does not
/// use a path at all.
pub const MIFTAH_KHATT_SIMA: &str = "gui/theme/custom_font";

/// `TextServerAdvanced`'s reported name, which is what [`MIFTAH_KHADIM`] takes.
pub const ISM_KHADIM_MUTAQADDIM: &str = "ICU / HarfBuzz / Graphite";

/// `TextServerFallback`'s reported name — no shaping, no bidi.
pub const ISM_KHADIM_IHTIYAT: &str = "Fallback";

/// Where the patch's font is mounted inside the patch package.
///
/// Only rung one uses it, and only as a value written into a setting. Nothing
/// in this module opens it.
pub const MASAR_KHATT_RUQAA: &str = "res://taarib/khatt.fontfile";

/// Where the patch's generated translation is mounted inside the package.
pub const MASAR_TARJAMA_RUQAA: &str = "res://taarib/ar.translation";

// ---------------------------------------------------------------------------
// The override file
// ---------------------------------------------------------------------------

/// Godot's user-side project-settings override, beside the executable.
///
/// The engine looks for this file next to the binary and applies every key in
/// it over the packed `project.godot`. It is the whole of rung one, it is a
/// file the engine never writes itself, and uninstalling the configuration rung
/// is deleting it.
pub use taarib_mustalahat::muharrik::asmaa_muharrik::TAJAWUZ_GODOT as MALAF_TAJAWUZ;

/// The first line of an override file Taarib wrote.
///
/// Present so that rung one can tell its own file from somebody else's without
/// parsing either. A file that does not open with this line is not rewritten
/// and not merged into — it is left exactly as it is and the rung declines,
/// because a game that shipped its own override has a reason for it and
/// discovering that reason is not worth a silently broken game.
pub const ALAMAT_TAARIB: &str = "; taarib:tajawuz";

/// The largest override file this module will read back.
///
/// An `override.cfg` is a few hundred bytes. A quarter of a megabyte is far
/// past anything real and is checked against the file's length before the
/// contents are decoded, because the file sits in a directory a game, a
/// launcher and a player all write to.
pub const AQSA_HAJM_TAJAWUZ: u64 = 256 * 1024;

/// The largest font the patch may carry.
///
/// Sixteen megabytes holds a very generous Arabic font with a full Latin
/// complement; a CJK-sized collection would be larger and is not what this path
/// installs. Checked before the bytes are handed to the engine, because
/// `FontFile::set_data` copies them into the game's address space and a length
/// nobody bounded is a length an attacker chose.
pub const AQSA_HAJM_KHATT: u64 = 16 * 1024 * 1024;

/// The largest generated translation resource the patch may carry.
pub const AQSA_HAJM_TARJAMA: u64 = 64 * 1024 * 1024;

/// The four-byte tags an OpenType or TrueType container may begin with.
///
/// `0x00010000` for TrueType outlines, `OTTO` for compact font format outlines,
/// `true` for the older Apple spelling, `ttcf` for a collection, and the two
/// web font wrappers, which Godot's `FreeType` build reads. This is a framing
/// check and not a parse: nothing here reads a table, a glyph or a `cmap`. It
/// exists so that a patch carrying the wrong bytes is refused by name here
/// rather than by `FreeType` eight frames deeper, where the only symptom is text
/// drawn in the game's original font.
pub const SIHR_KHATT: [[u8; 4]; 6] = [
    [0x00, 0x01, 0x00, 0x00],
    *b"OTTO",
    *b"true",
    *b"ttcf",
    *b"wOFF",
    *b"wOF2",
];

// ---------------------------------------------------------------------------
// Directions
// ---------------------------------------------------------------------------

/// `Control.TextDirection` — how a control orders the text inside itself.
///
/// This is not the bidirectional algorithm and does not replace it. It is the
/// *paragraph* direction the algorithm starts from, which decides where a line
/// with no strong character goes and which side neutral runs at the edges fall
/// to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IttijahNass {
    /// Take the parent's. Godot's default for most controls.
    Warith,
    /// Detect from the text's first strong character.
    ///
    /// What this adapter sets. Forcing right-to-left instead would reverse the
    /// Latin runs inside Arabic sentences — version numbers, player names, key
    /// bindings, file paths — and a game whose Arabic is right and whose build
    /// number reads backwards looks broken in a way that is harder to explain
    /// than no patch at all.
    Tilqai,
    /// Force left to right.
    Yasar,
    /// Force right to left.
    Yameen,
}

impl IttijahNass {
    /// The value Godot's `Control.TextDirection` enum uses.
    #[must_use]
    pub const fn qeema(self) -> i64 {
        match self {
            Self::Warith => 0,
            Self::Tilqai => 1,
            Self::Yasar => 2,
            Self::Yameen => 3,
        }
    }

    /// A stable short name for logs and diagnostics bundles.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Warith => "inherited",
            Self::Tilqai => "auto",
            Self::Yasar => "ltr",
            Self::Yameen => "rtl",
        }
    }
}

/// `Control.LayoutDirection` — which way a control arranges its children.
///
/// Mirroring, not text: which edge a label sits against, which way a progress
/// bar fills, which side a scrollbar takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IttijahTakhtit {
    /// Take the parent's.
    Warith,
    /// Follow the application locale — right to left once the locale is `ar`.
    ///
    /// The value this adapter sets, and the only one whose numbering is stable
    /// across every Godot 4 release: 4.3 renamed it from `LOCALE` to
    /// `APPLICATION_LOCALE` and added `SYSTEM_LOCALE` after the two forced
    /// values, so 4 means one thing on 4.3 and does not exist on 4.0. One is
    /// one everywhere.
    Thaqafa,
    /// Force left to right.
    Yasar,
    /// Force right to left.
    Yameen,
}

impl IttijahTakhtit {
    /// The value Godot's `Control.LayoutDirection` enum uses.
    #[must_use]
    pub const fn qeema(self) -> i64 {
        match self {
            Self::Warith => 0,
            Self::Thaqafa => 1,
            Self::Yasar => 2,
            Self::Yameen => 3,
        }
    }

    /// A stable short name for logs.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Warith => "inherited",
            Self::Thaqafa => "locale",
            Self::Yasar => "ltr",
            Self::Yameen => "rtl",
        }
    }
}

/// The root window's layout direction, in the project setting's own numbering.
///
/// A separate type from [`IttijahTakhtit`] on purpose. The setting has no
/// "inherited" case, so the engine's own reader maps 1 to left-to-right and 2
/// to right-to-left — one *less* than the `Control` enum at every value. The
/// two are one apart, both are small integers, and nothing at the call site
/// would catch the confusion: writing `IttijahTakhtit::Thaqafa`'s 1 into this
/// key forces left-to-right on a game the patch just asked to follow the
/// locale, and the game then looks unpatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IttijahJidhr {
    /// Follow the application locale. What this adapter writes.
    Thaqafa,
    /// Force left to right.
    Yasar,
    /// Force right to left.
    Yameen,
    /// Follow the operating system's locale rather than the application's.
    ///
    /// Added in Godot 4.3. Written by nothing here; a build older than 4.3
    /// reads the value as out of range and falls back to the application
    /// locale, which is where this adapter wanted to be anyway.
    ThaqafatNizam,
}

impl IttijahJidhr {
    /// The value [`MIFTAH_ITTIJAH_JIDHR`] takes.
    #[must_use]
    pub const fn qeema(self) -> i64 {
        match self {
            Self::Thaqafa => 0,
            Self::Yasar => 1,
            Self::Yameen => 2,
            Self::ThaqafatNizam => 3,
        }
    }

    /// The equivalent `Control` value, for the live rung, which speaks the
    /// other numbering.
    #[must_use]
    pub const fn takhtit(self) -> IttijahTakhtit {
        match self {
            Self::Thaqafa | Self::ThaqafatNizam => IttijahTakhtit::Thaqafa,
            Self::Yasar => IttijahTakhtit::Yasar,
            Self::Yameen => IttijahTakhtit::Yameen,
        }
    }

    /// A stable short name for logs.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Thaqafa => "locale",
            Self::Yasar => "ltr",
            Self::Yameen => "rtl",
            Self::ThaqafatNizam => "system-locale",
        }
    }
}

// ---------------------------------------------------------------------------
// What the patch supplies
// ---------------------------------------------------------------------------

/// The patch's font, as bytes.
///
/// There is no constructor here that takes a path, and that absence is the
/// enforcement of Decision 5 rather than a comment about it. A game's own font
/// cannot reach this type, so no code below can load one by accident, and a
/// future change that wanted to would have to add the door first — which is a
/// change a reviewer sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhattRuqaa {
    ism: String,
    bayt: Vec<u8>,
}

impl KhattRuqaa {
    /// Takes the patch's font bytes under a name the theme entries refer to.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when the bytes exceed [`AQSA_HAJM_KHATT`],
    /// refused before the engine is asked to copy them, and
    /// [`KhataGodot::KhattMarfud`] when they are empty or do not open with one
    /// of [`SIHR_KHATT`] — a framing check, not a parse, so that a patch built
    /// with the wrong section in it is named here instead of surfacing as text
    /// silently drawn in the game's own font.
    pub fn jadeed(ism: impl Into<String>, bayt: Vec<u8>) -> Result<Self, KhataGodot> {
        let tul = tul_u64(bayt.len());
        if tul > AQSA_HAJM_KHATT {
            return Err(KhataGodot::HajmMufrit {
                haql: "patch font length",
                qeema: tul,
                saqf: AQSA_HAJM_KHATT,
            });
        }
        let Some(sihr) = bayt.first_chunk::<4>() else {
            return Err(KhataGodot::KhattMarfud {
                sabab: format!(
                    "the patch carries {tul} bytes of font data, which is too few to carry \
                     even a container tag"
                ),
            });
        };
        if !SIHR_KHATT.contains(sihr) {
            return Err(KhataGodot::KhattMarfud {
                sabab: format!(
                    "the patch's font opens with {sihr:02x?}, which is none of the TrueType, \
                     OpenType, collection or web font container tags Godot's FreeType reads"
                ),
            });
        }
        Ok(Self { ism: ism.into(), bayt })
    }

    /// The name the theme entries refer to this font by.
    #[must_use]
    pub fn ism(&self) -> &str {
        &self.ism
    }

    /// The bytes, for the binding that hands them to `FontFile`.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }

    /// How many bytes, for the log line that records the registration.
    #[must_use]
    pub fn hajm(&self) -> u64 {
        tul_u64(self.bayt.len())
    }
}

/// One theme entry the patch wants the font assigned into.
///
/// Godot's `Theme` is keyed by (entry name, theme type): `("font", "Label")`,
/// `("font", "Button")`, `("normal_font", "RichTextLabel")`. A game with a
/// custom theme names its own types, and the patch lists them because this
/// adapter makes no policy — it assigns what it was told to assign and does not
/// guess at a game's widget vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalSima {
    /// The theme type, e.g. `Label`.
    pub naw: String,
    /// The entry within that type, e.g. `font`.
    pub madkhal: String,
}

impl MadkhalSima {
    /// Names one theme entry.
    #[must_use]
    pub fn jadeed(naw: impl Into<String>, madkhal: impl Into<String>) -> Self {
        Self { naw: naw.into(), madkhal: madkhal.into() }
    }

    /// The pair as one string, for logs and for the record.
    #[must_use]
    pub fn ism(&self) -> String {
        format!("{}/{}", self.naw, self.madkhal)
    }
}

// ---------------------------------------------------------------------------
// The seam to the extension
// ---------------------------------------------------------------------------

/// The live engine, as this module needs to see it.
///
/// Implemented by `imtidad.rs` over the `GDExtension` interface. Declared here so
/// that neither file waits on the other, and deliberately small: only the first
/// three methods must be implemented, and every method below them refuses by
/// default, because a binding that reached `ProjectSettings` and no further is
/// still a binding that lets rungs one and two run and report honestly.
pub trait Musajjil: Send + Sync {
    /// Whether the implementation has a usable handle on the engine right now.
    ///
    /// Checked before rung three is attempted, so a library loaded into a
    /// process whose scene level has not initialised yet reports the
    /// configuration rung rather than a failure.
    fn muhayya(&self) -> bool;

    /// Writes a project setting in the running process.
    ///
    /// The value is typed rather than stringly, because `set_setting` takes a
    /// `Variant` and a game that was handed the string `"0"` where it expected
    /// an integer stores a string: the read-back then succeeds, the engine's own
    /// reader silently takes its default, and the rung reports a success nobody
    /// got.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the extension has no handle on the
    /// engine, and [`KhataGodot::ThaqafaMarfuda`] when the engine was reached
    /// and refused the write.
    fn daa_idad(&self, miftah: &str, qeema: &QeemaIdad) -> Result<(), KhataGodot>;

    /// Reads a project setting back.
    ///
    /// The read-back is what separates "was written" from "took effect", and
    /// every rung in this module is judged on it.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the engine cannot be reached or does
    /// not carry the setting.
    fn iqra_idad(&self, miftah: &str) -> Result<String, KhataGodot>;

    /// Builds a `FontFile` from the patch's bytes and keeps it under a name.
    ///
    /// The bytes are always the patch's own — see [`KhattRuqaa`]. The engine
    /// copies them into its own allocation, so the caller's buffer may go away
    /// afterwards.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::KhattMarfud`] when the engine rejected the bytes, and
    /// [`KhataGodot::ImtidadMarfud`] when the binding does not reach `ClassDB`.
    fn sajjil_khatt(&self, ism: &str, bayt: &[u8]) -> Result<(), KhataGodot> {
        let _ = (ism, bayt);
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not construct engine objects".to_owned(),
        })
    }

    /// Assigns a registered font as the default theme font.
    ///
    /// # Errors
    ///
    /// As [`Musajjil::sajjil_khatt`], plus [`KhataGodot::KhattMarfud`] when no
    /// font is registered under that name.
    fn asnid_khatt_iftiradi(&self, ism: &str) -> Result<(), KhataGodot> {
        let _ = ism;
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the theme database".to_owned(),
        })
    }

    /// Assigns a registered font into one named theme type and entry.
    ///
    /// # Errors
    ///
    /// As [`Musajjil::asnid_khatt_iftiradi`].
    fn asnid_khatt_naw(&self, madkhal: &MadkhalSima, ism: &str) -> Result<(), KhataGodot> {
        let _ = (madkhal, ism);
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the theme database".to_owned(),
        })
    }

    /// Loads a generated `Translation` resource for a locale.
    ///
    /// `bayt` is the resource the `pck` module produced, passed as a slice
    /// rather than as a path so that this module never names a file. The
    /// implementation hands it to the engine, which parses it with its own
    /// reader and adds it to the `TranslationServer`.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ThaqafaMarfuda`] when the engine rejected the resource,
    /// and [`KhataGodot::ImtidadMarfud`] when the binding does not reach the
    /// translation server.
    fn hammil_tarjama(&self, wasm: &str, bayt: &[u8]) -> Result<(), KhataGodot> {
        let _ = (wasm, bayt);
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the translation server".to_owned(),
        })
    }

    /// Makes a locale active — `TranslationServer::set_locale`.
    ///
    /// # Errors
    ///
    /// As [`Musajjil::hammil_tarjama`].
    fn thabbit_thaqafa(&self, wasm: &str) -> Result<(), KhataGodot> {
        let _ = wasm;
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the translation server".to_owned(),
        })
    }

    /// The locale the process considers active.
    ///
    /// # Errors
    ///
    /// As [`Musajjil::hammil_tarjama`].
    fn thaqafa_haliya(&self) -> Result<String, KhataGodot> {
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the translation server".to_owned(),
        })
    }

    /// Sets the root window's layout direction in the live scene tree.
    ///
    /// Takes a [`IttijahTakhtit`] and not a [`IttijahJidhr`], because the live
    /// call goes through `Window::set_layout_direction`, which speaks the
    /// `Control` numbering. The conversion happens once, at
    /// [`IttijahJidhr::takhtit`], rather than at each call site.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the binding does not reach the scene
    /// tree, which before the scene initialisation level it does not.
    fn thabbit_ittijah(&self, takhtit: IttijahTakhtit) -> Result<(), KhataGodot> {
        let _ = takhtit;
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the scene tree".to_owned(),
        })
    }

    /// Sets both directions on one named `Control`.
    ///
    /// `masar_uqda` is a scene-tree path the patch lists. The patch lists them
    /// rather than this module walking the tree, for the same reason the theme
    /// entries are listed: a game's node names are the game's, and an adapter
    /// that guessed at them would either miss the controls that matter or
    /// rewrite ones it was never asked to touch.
    ///
    /// Most games need none of these. A `Control` whose text direction is left
    /// at [`IttijahNass::Warith`] resolves up to the root window, and the root
    /// window's direction is what [`Musajjil::thabbit_ittijah`] already set. The
    /// entries exist for the control a game hard-coded to left-to-right in its
    /// own scene, which no setting can reach.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the binding does not reach the scene
    /// tree, and when no node exists at that path — which is a patch naming a
    /// node this game does not have, and is worth reporting by name.
    fn thabbit_ittijah_uqda(
        &self,
        masar_uqda: &str,
        takhtit: IttijahTakhtit,
        nass: IttijahNass,
    ) -> Result<(), KhataGodot> {
        let _ = (masar_uqda, takhtit, nass);
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the scene tree".to_owned(),
        })
    }

    /// The name the running text server reports.
    ///
    /// Compared against [`ISM_KHADIM_MUTAQADDIM`]. A game running
    /// [`ISM_KHADIM_IHTIYAT`] has no shaping and no bidi, and that is worth
    /// saying out loud in the report rather than discovering from a screenshot.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the binding does not reach the text
    /// server.
    fn ism_khadim(&self) -> Result<String, KhataGodot> {
        Err(KhataGodot::ImtidadMarfud {
            sabab: "this binding does not reach the text server".to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Rungs, and the record of which one fired
// ---------------------------------------------------------------------------

/// One rung of the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rutba {
    /// [`MALAF_TAJAWUZ`] beside the executable. Nothing is injected.
    Idadat,
    /// Launch options, for a launcher that rewrites its configuration.
    SatrAwamir,
    /// The `GDExtension`, live in the process.
    Imtidad,
}

impl Rutba {
    /// A stable short name for logs and diagnostics bundles.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Idadat => "idadat",
            Self::SatrAwamir => "satr-awamir",
            Self::Imtidad => "imtidad",
        }
    }

    /// The rung's ordinal, one-based, as the ladder is documented.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Idadat => 1,
            Self::SatrAwamir => 2,
            Self::Imtidad => 3,
        }
    }

    /// Whether this rung puts code inside the game's process.
    #[must_use]
    pub const fn muqtahim(self) -> bool {
        matches!(self, Self::Imtidad)
    }
}

/// What one rung did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatijatRutba {
    /// Which rung.
    pub rutba: Rutba,
    /// Whether the rung's effect was confirmed by reading the value back.
    ///
    /// A rung that could not be verified is not a rung that failed: rung one
    /// cannot be verified at all until the game next starts, because
    /// `override.cfg` is read during engine initialisation and nothing rereads
    /// it afterwards.
    pub muakkada: bool,
    /// One sentence naming what happened, for the log and the bundle.
    pub mulahaza: String,
}

/// The record of one concern's walk down the ladder.
///
/// Four of these come back from a run, one each for the font, the theme, the
/// direction and the locale, because "the font took and the locale did not" is
/// the ordinary outcome on a game that ships its own settings, and one combined
/// verdict would report it as either a success or a failure and be wrong twice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SijillGodot {
    /// Every rung attempted, in the order attempted.
    pub rutab: Vec<NatijatRutba>,
    /// The rung whose effect was confirmed, if any.
    pub nafidha: Option<Rutba>,
}

impl SijillGodot {
    /// Records a rung.
    pub fn sajjil(&mut self, rutba: Rutba, muakkada: bool, mulahaza: impl Into<String>) {
        if muakkada && self.nafidha.is_none() {
            self.nafidha = Some(rutba);
        }
        self.rutab.push(NatijatRutba { rutba, muakkada, mulahaza: mulahaza.into() });
    }

    /// Whether any rung was confirmed.
    #[must_use]
    pub const fn muakkad(&self) -> bool {
        self.nafidha.is_some()
    }

    /// Whether any rung ran at all, confirmed or not.
    #[must_use]
    pub const fn juribat(&self) -> bool {
        !self.rutab.is_empty()
    }

    /// Every rung's note joined into one sentence, for an error's `sabab`.
    #[must_use]
    pub fn sabab(&self) -> String {
        if self.rutab.is_empty() {
            return "no rung was attempted".to_owned();
        }
        self.rutab
            .iter()
            .map(|natija| {
                let rutba = natija.rutba;
                format!("rung {} ({}): {}", rutba.raqm(), rutba.ism(), natija.mulahaza)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// What a whole run produced: one record per concern.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NatijatKhadim {
    /// Registering the patch's font.
    pub khatt: SijillGodot,
    /// Assigning it into the theme's default and named entries.
    pub sima: SijillGodot,
    /// The layout and text direction.
    pub ittijah: SijillGodot,
    /// The translation and the locale.
    pub thaqafa: SijillGodot,
    /// The text server the game reports running, when it could be read.
    ///
    /// [`None`] when no rung reached the engine. `Some` carrying anything other
    /// than [`ISM_KHADIM_MUTAQADDIM`] is the one genuinely bad answer on this
    /// path: it means the game was exported with the fallback text server and
    /// gets no shaping from the engine at all.
    pub khadim: Option<String>,
}

impl NatijatKhadim {
    /// Whether the running text server is the one that shapes.
    ///
    /// [`None`] when it could not be read, which is every offline run.
    #[must_use]
    pub fn khadim_yushakkil(&self) -> Option<bool> {
        self.khadim.as_ref().map(|ism| ism == ISM_KHADIM_MUTAQADDIM)
    }

    /// Whether every concern was confirmed by a read-back.
    #[must_use]
    pub const fn muakkad(&self) -> bool {
        self.khatt.muakkad() && self.sima.muakkad() && self.ittijah.muakkad()
            && self.thaqafa.muakkad()
    }
}

// ---------------------------------------------------------------------------
// One project setting
// ---------------------------------------------------------------------------

/// Which generation's spelling a settings file is written in.
///
/// The override file's grammar is shared between the two engines and its
/// *vocabulary* is not. Godot 4's `VariantParser` accepts `PackedStringArray`,
/// `PoolStringArray` and `StringArray`; Godot 3.2's accepts only the last two,
/// and 3.6 gained `PackedStringArray` as a compatibility alias late in the
/// line. So a file written with Godot 4's spelling parses on some Godot 3
/// builds and not on others, and the failure is the worst kind available here:
/// the parser gives up on that one value, the key ends up unset, and a game
/// whose translation list was the only thing the patch had to change starts
/// with no translation and no message. The dialect is therefore an argument
/// wherever a value becomes text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lahja {
    /// Godot 4: `PackedStringArray( … )`.
    #[default]
    Rabi,
    /// Godot 3: `PoolStringArray( … )`, which every 3.x parses.
    Thalith,
}

impl Lahja {
    /// The constructor name a packed string array is written with.
    #[must_use]
    pub const fn ism_qaima(self) -> &'static str {
        match self {
            Self::Rabi => "PackedStringArray",
            Self::Thalith => "PoolStringArray",
        }
    }
}

/// A project setting's value, at the type the engine stores it as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QeemaIdad {
    /// A `String` — a locale tag, a resource path, a text server name.
    Nass(String),
    /// An `int` — the root layout direction, and nothing else here.
    Raqm(i64),
    /// A `PackedStringArray` — the translation list.
    Qaima(Vec<String>),
}

impl QeemaIdad {
    /// The value as Godot's config format writes it, in one generation's
    /// spelling.
    ///
    /// Every string goes through [`iqtibas`], which is not decoration: these
    /// values come out of a patch a stranger produced, and a path containing a
    /// quote and a newline would otherwise close the string, end the line, and
    /// let the rest of the field become further keys in the section — a config
    /// injection into a file the engine executes settings from.
    #[must_use]
    pub fn nass_idad_bi(&self, lahja: Lahja) -> String {
        match self {
            Self::Nass(nass) => iqtibas(nass),
            Self::Raqm(raqm) => raqm.to_string(),
            Self::Qaima(qaima) => {
                let dakhil =
                    qaima.iter().map(|nass| iqtibas(nass)).collect::<Vec<_>>().join(", ");
                format!("{}({dakhil})", lahja.ism_qaima())
            }
        }
    }

    /// The value in Godot 4's spelling — [`QeemaIdad::nass_idad_bi`] with
    /// [`Lahja::Rabi`].
    #[must_use]
    pub fn nass_idad(&self) -> String {
        self.nass_idad_bi(Lahja::Rabi)
    }

    /// The value as a read-back would report it.
    ///
    /// A list joins on a comma with no brackets, because what comes back from
    /// the engine is the array's own string form and the comparison this module
    /// makes is "does it carry what we asked for", not "is it byte-identical".
    #[must_use]
    pub fn muqarana(&self) -> String {
        match self {
            Self::Nass(nass) => nass.clone(),
            Self::Raqm(raqm) => raqm.to_string(),
            Self::Qaima(qaima) => qaima.join(", "),
        }
    }
}

/// A Godot string literal, escaped so that it cannot end early.
///
/// Backslash and quote are escaped; the three whitespace characters that would
/// end a line are turned into their escapes; every other C0 control is dropped
/// rather than escaped, because none of them belongs in a locale tag or a
/// resource path and passing one through would only move the question to the
/// engine's own parser.
#[must_use]
pub fn iqtibas(nass: &str) -> String {
    let mut makhraj = String::with_capacity(nass.len() + 2);
    makhraj.push('"');
    for harf in nass.chars() {
        match harf {
            '\\' => makhraj.push_str("\\\\"),
            '"' => makhraj.push_str("\\\""),
            '\n' => makhraj.push_str("\\n"),
            '\r' => makhraj.push_str("\\r"),
            '\t' => makhraj.push_str("\\t"),
            _ if harf.is_control() => {}
            _ => makhraj.push(harf),
        }
    }
    makhraj.push('"');
    makhraj
}

/// One `Key=Value` Taarib writes into the override file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalIdad {
    /// The full project-setting path, e.g. `internationalization/locale/test`.
    pub miftah: String,
    /// Its value.
    pub qeema: QeemaIdad,
}

impl MadkhalIdad {
    /// Builds an entry.
    #[must_use]
    pub fn jadeed(miftah: impl Into<String>, qeema: QeemaIdad) -> Self {
        Self { miftah: miftah.into(), qeema }
    }

    /// The section this setting lives in — everything before the first slash.
    ///
    /// Godot's config format splits a setting path exactly here and nowhere
    /// else: `internationalization/locale/test` is the key `locale/test` in the
    /// section `internationalization`, not a three-level tree. A writer that
    /// split on the last slash would produce a file the engine reads as three
    /// settings nobody asked for.
    #[must_use]
    pub fn qism(&self) -> &str {
        self.miftah.split_once('/').map_or(self.miftah.as_str(), |(awwal, _)| awwal)
    }

    /// The key within the section — everything after the first slash.
    #[must_use]
    pub fn miftah_qism(&self) -> &str {
        self.miftah.split_once('/').map_or(self.miftah.as_str(), |(_, baqi)| baqi)
    }

    /// The line as it is written, in one generation's spelling.
    #[must_use]
    pub fn satr_bi(&self, lahja: Lahja) -> String {
        format!("{}={}", self.miftah_qism(), self.qeema.nass_idad_bi(lahja))
    }

    /// The line in Godot 4's spelling.
    #[must_use]
    pub fn satr(&self) -> String {
        self.satr_bi(Lahja::Rabi)
    }
}

// ---------------------------------------------------------------------------
// The override file
// ---------------------------------------------------------------------------

/// What is at [`MALAF_TAJAWUZ`] right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HalatTajawuz {
    /// No such file. The ordinary case, and the one rung one wants.
    Ghaib,
    /// A file opening with [`ALAMAT_TAARIB`]. A previous install, replaceable.
    Lana,
    /// A file somebody else wrote. Not touched, not merged into, not read past
    /// its first line.
    Ghareeb,
}

impl HalatTajawuz {
    /// Whether rung one may write.
    #[must_use]
    pub const fn qabil_lil_kitaba(self) -> bool {
        matches!(self, Self::Ghaib | Self::Lana)
    }
}

/// How many entries the case-folding lookup below will look at.
///
/// A game directory is somebody else's data, and an unbounded read of one is a
/// promise about memory this code cannot keep. Four thousand is the ceiling
/// every other directory walk in this product uses.
const AQSA_MUTABAQA: usize = 4_096;

/// Resolves one child name against a directory, ignoring letter case.
///
/// This crate's only case-folding filesystem lookup, and it has exactly one
/// caller — [`MalafTajawuz::fi_mujallad`]. Everything else here is handed an
/// already-resolved path, and the package locator finds a `.pck` by extension
/// across a tree walk rather than by joining a name, so neither needs it.
///
/// The exact spelling is tried first, so the ordinary case costs no directory
/// read. `symlink_metadata` rather than `exists`: a broken symlink is still an
/// entry occupying that name, and writing over it because its target is missing
/// would be a different bug.
fn ibn_bila_hala(mujallad: &Path, ism: &str) -> Option<PathBuf> {
    let mubashir = mujallad.join(ism);
    if mubashir.symlink_metadata().is_ok() {
        return Some(mubashir);
    }
    std::fs::read_dir(mujallad)
        .ok()?
        .take(AQSA_MUTABAQA)
        .flatten()
        .find(|madkhal| madkhal.file_name().to_string_lossy().eq_ignore_ascii_case(ism))
        .map(|madkhal| madkhal.path())
}

/// Godot's user-side project-settings override, as Taarib owns it.
///
/// Whole-file, never merged. The Unreal adapter merges into `Engine.ini`
/// because that file is the game's and a game's players hand-edit it; this file
/// is not the game's — the engine never creates it and a shipped Godot game
/// does not have one — so the honest handling is to own it entirely or leave it
/// entirely alone. That also makes undo exact: uninstalling rung one is
/// deleting one file, with no possibility of leaving a line behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalafTajawuz {
    masar: PathBuf,
}

impl MalafTajawuz {
    /// Names the override file beside a game's executable.
    #[must_use]
    pub fn jadeed(masar: impl Into<PathBuf>) -> Self {
        Self { masar: masar.into() }
    }

    /// Names it from the directory the executable lives in.
    ///
    /// Resolved through [`ibn_bila_hala`] rather than joined literally, and the
    /// reason is [`Self::hala`] rather than the write. A game directory that
    /// already holds an `Override.cfg` somebody else wrote holds a file this
    /// module must not touch — but a literal `override.cfg` join reports
    /// [`HalatTajawuz::Ghaib`] against it, rung one writes its own file beside
    /// it, and under Wine, where an exact match wins a case-insensitive lookup,
    /// the engine then reads Taarib's and silently loses every setting the other
    /// file carried. When nothing is there the spelling asked for is kept, which
    /// is the lower-case name Godot itself documents.
    #[must_use]
    pub fn fi_mujallad(mujallad: &Path) -> Self {
        Self::jadeed(
            ibn_bila_hala(mujallad, MALAF_TAJAWUZ)
                .unwrap_or_else(|| mujallad.join(MALAF_TAJAWUZ)),
        )
    }

    /// The path this writes to.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// What is there now.
    ///
    /// Reads at most [`AQSA_HAJM_TAJAWUZ`] bytes and only to look at the first
    /// line. This is the one read this module performs, it is of a file the
    /// engine treats as a user-side override, and it is not a game asset.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::KhataMalaf`] when the file exists and cannot be read, and
    /// [`KhataGodot::HajmMufrit`] when it is larger than
    /// [`AQSA_HAJM_TAJAWUZ`] — refused before its bytes are decoded, because
    /// this file sits in a directory a game, a launcher and a player all write
    /// to.
    pub fn hala(&self) -> Result<HalatTajawuz, KhataGodot> {
        let bayt = match std::fs::read(&self.masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(HalatTajawuz::Ghaib);
            }
            Err(sabab) => {
                return Err(KhataGodot::KhataMalaf { masar: self.masar.clone(), sabab });
            }
        };
        let tul = tul_u64(bayt.len());
        if tul > AQSA_HAJM_TAJAWUZ {
            return Err(KhataGodot::HajmMufrit {
                haql: "override.cfg length",
                qeema: tul,
                saqf: AQSA_HAJM_TAJAWUZ,
            });
        }
        // Byte-wise rather than through a UTF-8 decode: a foreign file in an
        // encoding this module does not read is still a foreign file, and
        // refusing to decode it would be refusing to answer a question that has
        // an obvious answer.
        Ok(if bayt.starts_with(ALAMAT_TAARIB.as_bytes()) {
            HalatTajawuz::Lana
        } else {
            HalatTajawuz::Ghareeb
        })
    }

    /// The file's whole text, for a set of entries, in Godot 4's spelling.
    #[must_use]
    pub fn nass(madakhil: &[MadkhalIdad]) -> String {
        Self::nass_bi(Lahja::Rabi, madakhil)
    }

    /// The file's whole text, for a set of entries, in one generation's
    /// spelling.
    ///
    /// Sections in first-appearance order, keys in the order given. Nothing is
    /// sorted, because the order the caller wrote the entries in is the order a
    /// maintainer reading the file expects them in.
    #[must_use]
    pub fn nass_bi(lahja: Lahja, madakhil: &[MadkhalIdad]) -> String {
        let mut nass = String::with_capacity(256);
        nass.push_str(ALAMAT_TAARIB);
        nass.push_str(
            "\n; Written by Taarib. Delete this file to remove the configuration rung.\n",
        );
        let mut aqsam: Vec<&str> = Vec::new();
        for madkhal in madakhil {
            let qism = madkhal.qism();
            if !aqsam.contains(&qism) {
                aqsam.push(qism);
            }
        }
        for qism in aqsam {
            nass.push_str("\n[");
            nass.push_str(qism);
            nass.push_str("]\n\n");
            for madkhal in madakhil.iter().filter(|madkhal| madkhal.qism() == qism) {
                nass.push_str(&madkhal.satr_bi(lahja));
                nass.push('\n');
            }
        }
        nass
    }

    /// Writes the entries, replacing a previous Taarib file and refusing a
    /// foreign one.
    ///
    /// The write is atomic — a temporary file in the same directory, flushed and
    /// renamed — because a half-written `override.cfg` is a game that will not
    /// start, and "the patch stopped the game from launching" is a worse outcome
    /// than every failure this module is trying to prevent.
    ///
    /// # Errors
    ///
    /// Whatever [`MalafTajawuz::hala`] refuses; [`KhataGodot::ImtidadMarfud`]
    /// when a foreign override file is present, which closes this route without
    /// closing the ladder, or when there are no entries to write — a rung that
    /// wrote nothing has established nothing, and reporting it as written was
    /// how "handed nothing" read as success; and [`KhataGodot::KhataMalaf`]
    /// when the write itself fails.
    pub fn aktub(&self, madakhil: &[MadkhalIdad]) -> Result<(), KhataGodot> {
        self.aktub_bi(Lahja::Rabi, madakhil)
    }

    /// Writes the entries in one generation's spelling.
    ///
    /// # Errors
    ///
    /// As [`MalafTajawuz::aktub`].
    pub fn aktub_bi(&self, lahja: Lahja, madakhil: &[MadkhalIdad]) -> Result<(), KhataGodot> {
        if madakhil.is_empty() {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{} was not written: there were no settings to put in it, and a rung that \
                     writes nothing establishes nothing",
                    self.masar.display()
                ),
            });
        }
        let hala = self.hala()?;
        if !hala.qabil_lil_kitaba() {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{} already exists and was not written by Taarib, so it belongs to the \
                     game or to another tool and was left untouched",
                    self.masar.display()
                ),
            });
        }
        masarat::kitaba_dharra_nass(&self.masar, &Self::nass_bi(lahja, madakhil)).map_err(|khata| {
            KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{} could not be written: {}",
                    self.masar.display(),
                    khata.li_sijill()
                ),
            }
        })
    }

    /// Deletes the override file, if it is Taarib's.
    ///
    /// Returns whether anything was removed. A foreign file is left alone and
    /// reported as not removed rather than as an error: uninstall should not
    /// fail because somebody else's configuration exists.
    ///
    /// # Errors
    ///
    /// Whatever [`MalafTajawuz::hala`] refuses, and [`KhataGodot::KhataMalaf`]
    /// when the file is Taarib's and cannot be deleted.
    pub fn tarajua(&self) -> Result<bool, KhataGodot> {
        match self.hala()? {
            HalatTajawuz::Ghaib | HalatTajawuz::Ghareeb => Ok(false),
            HalatTajawuz::Lana => match std::fs::remove_file(&self.masar) {
                Ok(()) => Ok(true),
                Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(sabab) => {
                    Err(KhataGodot::KhataMalaf { masar: self.masar.clone(), sabab })
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Locale matching
// ---------------------------------------------------------------------------

/// The primary language subtag of a locale name, lowercased.
///
/// Godot spells locales with an underscore — `ar_SA` — and the wider world
/// spells them with a hyphen, so both separators are accepted. A patch built
/// from a BCP-47 tag and a game reporting Godot's spelling must compare equal
/// or the read-back re-asserts a locale that is already active.
#[must_use]
pub fn lugha_wasm(wasm: &str) -> String {
    wasm.trim().split(['-', '_']).next().unwrap_or("").to_ascii_lowercase()
}

/// Whether an active locale satisfies a request for another.
///
/// `ar_SA` satisfies `ar`: Godot's fallback walk reaches the patch's
/// translation from either, and a game that answered `ar_SA` to a request for
/// `ar` has done what was asked. Treating that as a miss would make the ladder
/// keep re-setting a locale the engine had already accepted, and would report a
/// failure to the user on a game that is displaying Arabic.
#[must_use]
pub fn wasm_maqbul(hali: &str, matlub: &str) -> bool {
    let matlub = matlub.trim();
    if matlub.is_empty() {
        return false;
    }
    hali.trim().eq_ignore_ascii_case(matlub) || lugha_wasm(hali) == lugha_wasm(matlub)
}

/// Which spelling of Arabic to use, given what the game already advertises.
///
/// The game's own spelling when it has one, so that the patch selects the entry
/// the game's language menu already draws rather than adding a second Arabic
/// beside it, and [`WASM_ARABI`] otherwise.
#[must_use]
pub fn wasm_mufaddal(matah: &[String]) -> String {
    matah
        .iter()
        .find(|wasm| lugha_wasm(wasm) == WASM_ARABI)
        .map_or_else(|| WASM_ARABI.to_owned(), |wasm| wasm.trim().to_owned())
}

// ---------------------------------------------------------------------------
// The game's own translation list
// ---------------------------------------------------------------------------

/// The game's own [`MIFTAH_TARJAMAT`] entries, as the caller found them.
///
/// Two states and no default, on purpose. The key replaces the whole list
/// rather than adding to it, so a list written from a default nobody filled
/// would take every language the game shipped with away in exchange for Arabic
/// — and the first caller to forget a builder method would get exactly that by
/// omission. So there is no builder method: the caller says at construction
/// which of the two it has, and rung one writes the key only in the first case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TarjamatLuba {
    /// Read out of the game's packed settings. Empty means the game declares
    /// none, which is a fact about the game and not the absence of one.
    Maqrua(Vec<String>),
    /// Not read, and why: an encrypted package, a `project.binary` this build
    /// could not parse, a caller with no package in hand at all. Rung one
    /// leaves the key alone, because writing it would replace a list nobody has
    /// seen.
    LamTuqra {
        /// One sentence a user can act on.
        sabab: String,
    },
}

impl TarjamatLuba {
    /// Whether the list was read, so the key may be written.
    #[must_use]
    pub const fn maqrua(&self) -> bool {
        matches!(self, Self::Maqrua(_))
    }

    /// The entries, empty when the list was not read.
    #[must_use]
    pub fn madakhil(&self) -> &[String] {
        match self {
            Self::Maqrua(qaima) => qaima,
            Self::LamTuqra { .. } => &[],
        }
    }

    /// Why the list was not read, when it was not.
    #[must_use]
    pub fn sabab(&self) -> Option<&str> {
        match self {
            Self::Maqrua(_) => None,
            Self::LamTuqra { sabab } => Some(sabab),
        }
    }
}

// ---------------------------------------------------------------------------
// The configuration
// ---------------------------------------------------------------------------

/// Everything the Godot 4 path installs, and the ladder that installs it.
///
/// Built by the caller from the patch, walked once by [`KhadimNusus::hayyi`].
/// Nothing here reads a game file, and the font and translation are owned byte
/// buffers rather than paths for exactly that reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhadimNusus {
    tajawuz: MalafTajawuz,
    khatt: Option<KhattRuqaa>,
    masar_khatt: String,
    anwa: Vec<MadkhalSima>,
    tarjama: Option<Vec<u8>>,
    masar_tarjama: String,
    tarjamat_luba: TarjamatLuba,
    wasm: String,
    jidhr: IttijahJidhr,
    nass: IttijahNass,
    uqad: Vec<String>,
    yafrid_khadim: bool,
}

impl KhadimNusus {
    /// Builds the configuration against one game's override file and the
    /// game's own translation list.
    ///
    /// Defaults: the `ar` locale, the locale-driven root direction, automatic
    /// text direction, and the advanced text server forced on. No font, no
    /// theme entries and no translation until the caller supplies the patch's.
    /// The translation list has no default — see [`TarjamatLuba`].
    #[must_use]
    pub fn jadeed(tajawuz: MalafTajawuz, tarjamat_luba: TarjamatLuba) -> Self {
        Self {
            tajawuz,
            khatt: None,
            masar_khatt: MASAR_KHATT_RUQAA.to_owned(),
            anwa: Vec::new(),
            tarjama: None,
            masar_tarjama: MASAR_TARJAMA_RUQAA.to_owned(),
            tarjamat_luba,
            wasm: WASM_ARABI.to_owned(),
            jidhr: IttijahJidhr::Thaqafa,
            nass: IttijahNass::Tilqai,
            uqad: Vec::new(),
            yafrid_khadim: true,
        }
    }

    /// Adds a `Control` node path the patch wants both directions set on.
    #[must_use]
    pub fn bi_uqda(mut self, masar_uqda: impl Into<String>) -> Self {
        self.uqad.push(masar_uqda.into());
        self
    }

    /// Replaces the whole list of node paths.
    #[must_use]
    pub fn bi_uqad(mut self, uqad: Vec<String>) -> Self {
        self.uqad = uqad;
        self
    }

    /// Supplies the patch's font.
    #[must_use]
    pub fn bi_khatt(mut self, khatt: KhattRuqaa) -> Self {
        self.khatt = Some(khatt);
        self
    }

    /// Overrides where the font resource lives inside the patch package.
    #[must_use]
    pub fn bi_masar_khatt(mut self, masar: impl Into<String>) -> Self {
        self.masar_khatt = masar.into();
        self
    }

    /// Adds one theme type and entry the font is assigned into.
    #[must_use]
    pub fn bi_madkhal_sima(mut self, madkhal: MadkhalSima) -> Self {
        self.anwa.push(madkhal);
        self
    }

    /// Replaces the whole list of theme entries.
    #[must_use]
    pub fn bi_anwa_sima(mut self, anwa: Vec<MadkhalSima>) -> Self {
        self.anwa = anwa;
        self
    }

    /// Supplies the generated `Translation` resource's bytes.
    ///
    /// A byte slice and not a path: the `pck` module produces the resource and
    /// this module hands it to the engine, so no file name for a translation
    /// ever exists on this side of the seam.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when the resource is larger than
    /// [`AQSA_HAJM_TARJAMA`], refused before the engine copies it.
    pub fn bi_tarjama(mut self, bayt: Vec<u8>) -> Result<Self, KhataGodot> {
        let tul = tul_u64(bayt.len());
        if tul > AQSA_HAJM_TARJAMA {
            return Err(KhataGodot::HajmMufrit {
                haql: "patch translation length",
                qeema: tul,
                saqf: AQSA_HAJM_TARJAMA,
            });
        }
        self.tarjama = Some(bayt);
        Ok(self)
    }

    /// Overrides where the translation resource lives inside the package.
    #[must_use]
    pub fn bi_masar_tarjama(mut self, masar: impl Into<String>) -> Self {
        self.masar_tarjama = masar.into();
        self
    }

    /// Overrides the locale tag — see [`wasm_mufaddal`].
    #[must_use]
    pub fn bi_wasm(mut self, wasm: impl Into<String>) -> Self {
        self.wasm = wasm.into();
        self
    }

    /// Overrides the root window's layout direction.
    #[must_use]
    pub const fn bi_jidhr(mut self, jidhr: IttijahJidhr) -> Self {
        self.jidhr = jidhr;
        self
    }

    /// Overrides the text direction the controls start from.
    #[must_use]
    pub const fn bi_ittijah_nass(mut self, nass: IttijahNass) -> Self {
        self.nass = nass;
        self
    }

    /// Turns the text-driver override off.
    ///
    /// Left on by default. It costs one line and it is the difference between a
    /// game that shapes and a game that does not on a build exported with the
    /// fallback server.
    #[must_use]
    pub const fn bi_khadim(mut self, yafrid: bool) -> Self {
        self.yafrid_khadim = yafrid;
        self
    }

    /// The locale this configuration asks for.
    #[must_use]
    pub fn wasm(&self) -> &str {
        &self.wasm
    }

    /// The theme entries the font is assigned into.
    #[must_use]
    pub fn anwa(&self) -> &[MadkhalSima] {
        &self.anwa
    }

    /// The text direction the controls start from.
    #[must_use]
    pub const fn ittijah_nass(&self) -> IttijahNass {
        self.nass
    }

    /// The override file this writes.
    #[must_use]
    pub const fn tajawuz(&self) -> &MalafTajawuz {
        &self.tajawuz
    }

    /// The game's own translation list, as it was supplied.
    #[must_use]
    pub const fn tarjamat_luba(&self) -> &TarjamatLuba {
        &self.tarjamat_luba
    }

    /// The translation list rung one writes — the game's own, then the
    /// patch's — or [`None`] when the game's own was never read.
    ///
    /// The game's entries come first because [`MIFTAH_TARJAMAT`] replaces the
    /// list rather than extending it, and a game that lost its own languages
    /// because it gained Arabic is a worse patch than no patch. [`None`] rather
    /// than a shorter list for the same reason: a list nobody read cannot be
    /// written back, so the key is not written at all.
    #[must_use]
    pub fn qaimat_tarjamat(&self) -> Option<Vec<String>> {
        let TarjamatLuba::Maqrua(luba) = &self.tarjamat_luba else {
            return None;
        };
        let mut qaima = luba.clone();
        if self.tarjama.is_some() && !qaima.iter().any(|masar| masar == &self.masar_tarjama) {
            qaima.push(self.masar_tarjama.clone());
        }
        Some(qaima)
    }

    /// What rung one has to say about the translation list when it left the
    /// key alone, or nothing when it did not.
    fn mulahazat_tarjamat(&self) -> String {
        match self.tarjamat_luba.sabab() {
            Some(sabab) if self.tarjama.is_some() => format!(
                "; {MIFTAH_TARJAMAT} was left unwritten because the game's own list was not \
                 read ({sabab}) and the key replaces the whole list, so the patch's translation \
                 reaches the engine only through the extension"
            ),
            _ => String::new(),
        }
    }

    /// The project settings this configuration owns, in write order.
    #[must_use]
    pub fn madakhil(&self) -> Vec<MadkhalIdad> {
        let mut madakhil = Vec::with_capacity(6);
        if let Some(tarjamat) = self.qaimat_tarjamat()
            && !tarjamat.is_empty()
        {
            madakhil.push(MadkhalIdad::jadeed(MIFTAH_TARJAMAT, QeemaIdad::Qaima(tarjamat)));
        }
        if !self.wasm.trim().is_empty() {
            madakhil.push(MadkhalIdad::jadeed(
                MIFTAH_THAQAFA_IKHTIBAR,
                QeemaIdad::Nass(self.wasm.clone()),
            ));
            madakhil.push(MadkhalIdad::jadeed(
                MIFTAH_THAQAFA_IHTIYAT,
                QeemaIdad::Nass(self.wasm.clone()),
            ));
        }
        madakhil.push(MadkhalIdad::jadeed(
            MIFTAH_ITTIJAH_JIDHR,
            QeemaIdad::Raqm(self.jidhr.qeema()),
        ));
        if self.yafrid_khadim {
            madakhil.push(MadkhalIdad::jadeed(
                MIFTAH_KHADIM,
                QeemaIdad::Nass(ISM_KHADIM_MUTAQADDIM.to_owned()),
            ));
        }
        if self.khatt.is_some() {
            madakhil.push(MadkhalIdad::jadeed(
                MIFTAH_KHATT_SIMA,
                QeemaIdad::Nass(self.masar_khatt.clone()),
            ));
        }
        madakhil
    }

    /// Rung one: write the override file.
    ///
    /// # Errors
    ///
    /// Whatever [`MalafTajawuz::aktub`] refuses, unchanged, so the caller can
    /// name the path in its own message.
    pub fn rutbat_idadat(&self) -> Result<(), KhataGodot> {
        self.tajawuz.aktub(&self.madakhil())
    }

    /// Rung two: the launch options that carry the same request.
    ///
    /// One option, and only the locale. Godot 4 has no launch option that sets
    /// an arbitrary project setting — no `--set`, no ini override — so the font,
    /// the theme and the direction cannot be reached from a command line at all.
    /// Saying so is more useful than offering options that do nothing.
    #[must_use]
    pub fn rutbat_satr(&self) -> Vec<String> {
        if self.wasm.trim().is_empty() {
            return Vec::new();
        }
        vec![format!("--language {}", self.wasm)]
    }

    /// Rung three, the settings: write [`KhadimNusus::madakhil`] live.
    ///
    /// The same keys rung one wrote into a file, written again into the running
    /// `ProjectSettings`. Not redundant: on a game that already had its own
    /// `override.cfg`, or that was launched before the file existed, this is the
    /// only place they land, and the root layout direction and the default theme
    /// font are both read *after* scene initialisation, which is where this
    /// extension runs.
    ///
    /// Returns how many were written, how many read back equal, and what each
    /// refusal said. A read-back that matches proves the setting is stored; it
    /// does not prove the engine acted on it, and [`MIFTAH_KHADIM`] is the case
    /// that makes the difference concrete — the text server is built during
    /// engine start, so writing its name here stores a value nothing will read
    /// until the next launch.
    fn rutbat_idadat_hayya(
        &self,
        musajjil: &dyn Musajjil,
    ) -> (usize, usize, Vec<String>) {
        let mut kutibat = 0_usize;
        let mut muakkada = 0_usize;
        let mut fashila: Vec<String> = Vec::new();
        for madkhal in &self.madakhil() {
            match musajjil.daa_idad(&madkhal.miftah, &madkhal.qeema) {
                Ok(()) => {
                    kutibat = kutibat.saturating_add(1);
                    let matlub = madkhal.qeema.muqarana();
                    if matches!(
                        musajjil.iqra_idad(&madkhal.miftah),
                        Ok(hali) if hali.trim() == matlub
                    ) {
                        muakkada = muakkada.saturating_add(1);
                    }
                }
                Err(khata) => fashila.push(format!("{}: {khata}", madkhal.miftah)),
            }
        }
        (kutibat, muakkada, fashila)
    }

    /// Rung three, the font: build a `FontFile` from the patch's bytes.
    fn rutbat_khatt(&self, musajjil: &dyn Musajjil, natija: &mut NatijatKhadim) {
        let Some(khatt) = self.khatt.as_ref() else {
            natija.khatt.sajjil(Rutba::Imtidad, false, "the patch carries no font");
            return;
        };
        match musajjil.sajjil_khatt(khatt.ism(), khatt.bayt()) {
            // The engine offers no read-back for a registered font: there is no
            // setting to query and no name to look up. What it does offer is a
            // refusal, so an accepted `set_data` of bytes that already passed
            // the container check is the confirmation available, and it is
            // recorded as one rather than pretended away.
            Ok(()) => natija.khatt.sajjil(
                Rutba::Imtidad,
                true,
                format!(
                    "{} bytes accepted by FontFile as \"{}\"",
                    khatt.hajm(),
                    khatt.ism()
                ),
            ),
            Err(khata) => natija.khatt.sajjil(Rutba::Imtidad, false, khata.to_string()),
        }
    }

    /// Rung three, the theme: the default font, then each named entry.
    fn rutbat_sima(&self, musajjil: &dyn Musajjil, natija: &mut NatijatKhadim) {
        let Some(khatt) = self.khatt.as_ref() else {
            natija.sima.sajjil(Rutba::Imtidad, false, "the patch carries no font to assign");
            return;
        };
        let iftiradi = musajjil.asnid_khatt_iftiradi(khatt.ism());
        let mut fashila: Vec<String> = Vec::new();
        let mut najihat = 0_usize;
        for madkhal in &self.anwa {
            match musajjil.asnid_khatt_naw(madkhal, khatt.ism()) {
                Ok(()) => najihat = najihat.saturating_add(1),
                Err(khata) => fashila.push(format!("{}: {khata}", madkhal.ism())),
            }
        }
        match iftiradi {
            Ok(()) => natija.sima.sajjil(
                Rutba::Imtidad,
                true,
                format!(
                    "assigned as the theme default and into {najihat} of {} named entries{}",
                    self.anwa.len(),
                    if fashila.is_empty() {
                        String::new()
                    } else {
                        format!(" (refused: {})", fashila.join("; "))
                    }
                ),
            ),
            Err(khata) => natija.sima.sajjil(
                Rutba::Imtidad,
                najihat > 0,
                format!(
                    "the theme default was refused ({khata}); {najihat} of {} named entries \
                     took",
                    self.anwa.len()
                ),
            ),
        }
    }

    /// Rung three, the direction: the root window, then each listed control.
    fn rutbat_ittijah(&self, musajjil: &dyn Musajjil, natija: &mut NatijatKhadim) {
        let takhtit = self.jidhr.takhtit();
        let mut fashila: Vec<String> = Vec::new();
        let mut najihat = 0_usize;
        for masar_uqda in &self.uqad {
            match musajjil.thabbit_ittijah_uqda(masar_uqda, takhtit, self.nass) {
                Ok(()) => najihat = najihat.saturating_add(1),
                Err(khata) => fashila.push(format!("{masar_uqda}: {khata}")),
            }
        }
        match musajjil.thabbit_ittijah(takhtit) {
            Ok(()) => natija.ittijah.sajjil(
                Rutba::Imtidad,
                true,
                format!(
                    "the root window follows {} and {najihat} of {} listed controls were set \
                     to {}{}",
                    takhtit.ism(),
                    self.uqad.len(),
                    self.nass.ism(),
                    if fashila.is_empty() {
                        String::new()
                    } else {
                        format!(" (refused: {})", fashila.join("; "))
                    }
                ),
            ),
            Err(khata) => natija.ittijah.sajjil(
                Rutba::Imtidad,
                najihat > 0,
                format!("the root window was refused ({khata}); {najihat} controls took"),
            ),
        }
    }

    /// Rung three, the locale: load the translation, then set the locale.
    fn rutbat_thaqafa(&self, musajjil: &dyn Musajjil, natija: &mut NatijatKhadim) {
        if let Some(bayt) = self.tarjama.as_ref() {
            // A refusal here is noted and not fatal to the concern. The engine's
            // own loader reaches the same resource through the patch package and
            // the list rung one wrote, so a binding that cannot hand it over in
            // memory has not necessarily cost anything — and stopping here would
            // additionally skip the locale, which is the part that has no other
            // route inside a running process.
            if let Err(khata) = musajjil.hammil_tarjama(&self.wasm, bayt) {
                natija.thaqafa.sajjil(
                    Rutba::Imtidad,
                    false,
                    format!(
                        "the translation was not handed over in memory ({khata}); the engine \
                         loads it from the patch package instead"
                    ),
                );
            }
        }
        // Already Arabic? Then nothing is written into the process. The whole
        // point of the ladder is that rung three usually does not have to run.
        if matches!(musajjil.thaqafa_haliya(), Ok(hali) if wasm_maqbul(&hali, &self.wasm)) {
            natija.thaqafa.sajjil(
                Rutba::Imtidad,
                true,
                format!("{} was already active; only the translation was added", self.wasm),
            );
            return;
        }
        match musajjil.thabbit_thaqafa(&self.wasm) {
            Ok(()) => {
                let muakkada = matches!(
                    musajjil.thaqafa_haliya(),
                    Ok(hali) if wasm_maqbul(&hali, &self.wasm)
                );
                natija.thaqafa.sajjil(
                    Rutba::Imtidad,
                    muakkada,
                    if muakkada {
                        format!("{} is active and confirmed by read-back", self.wasm)
                    } else {
                        format!(
                            "{} was set and the read-back reports another locale, which is a \
                             game applying its own saved language after start",
                            self.wasm
                        )
                    },
                );
            }
            Err(khata) => natija.thaqafa.sajjil(Rutba::Imtidad, false, khata.to_string()),
        }
    }

    /// Walks the ladder and reports which rung achieved each concern.
    ///
    /// `musajjil` is [`None`] for the offline installer, which has no process to
    /// look at: it writes the override file, records rung one as unverified on
    /// all four concerns, and returns. Inside the game it is [`Some`], the
    /// override is still written first — so the next launch needs no extension
    /// at all — and the read-backs decide what rung three still has to do.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] naming every route that was tried, and only
    /// when rung one did not write and no concern was confirmed. It is a warning
    /// in the error contract rather than a failure, because a Godot 4 game that
    /// refuses all of this keeps running in its own language.
    pub fn hayyi(
        &self,
        musajjil: Option<&dyn Musajjil>,
    ) -> Result<NatijatKhadim, KhataGodot> {
        let mut natija = NatijatKhadim::default();
        let masar = self.tajawuz.masar().display().to_string();

        let idadat_najahat = match self.rutbat_idadat() {
            Ok(()) => {
                let mulahaza =
                    format!("{masar} written; every key applies at the next launch");
                natija.khatt.sajjil(Rutba::Idadat, false, format!("{mulahaza} (font path)"));
                natija.sima.sajjil(Rutba::Idadat, false, format!("{mulahaza} (theme font)"));
                natija.ittijah.sajjil(
                    Rutba::Idadat,
                    false,
                    format!("{mulahaza} (root direction {})", self.jidhr.ism()),
                );
                natija.thaqafa.sajjil(
                    Rutba::Idadat,
                    false,
                    format!("{mulahaza} (locale {}){}", self.wasm, self.mulahazat_tarjamat()),
                );
                true
            }
            Err(khata) => {
                let sabab = khata.to_string();
                natija.khatt.sajjil(Rutba::Idadat, false, sabab.clone());
                natija.sima.sajjil(Rutba::Idadat, false, sabab.clone());
                natija.ittijah.sajjil(Rutba::Idadat, false, sabab.clone());
                natija.thaqafa.sajjil(Rutba::Idadat, false, sabab);
                false
            }
        };

        let khiyarat = self.rutbat_satr();
        natija.thaqafa.sajjil(
            Rutba::SatrAwamir,
            false,
            format!("{} launch option(s) offered to the installer", khiyarat.len()),
        );
        for sijill in [&mut natija.khatt, &mut natija.sima, &mut natija.ittijah] {
            sijill.sajjil(
                Rutba::SatrAwamir,
                false,
                "Godot 4 has no launch option that reaches this",
            );
        }

        let Some(musajjil) = musajjil else {
            if idadat_najahat {
                return Ok(natija);
            }
            return Err(KhataGodot::ImtidadMarfud { sabab: sabab_shamil(&natija) });
        };

        if !musajjil.muhayya() {
            let sabab = "the extension reports no handle on this process yet".to_owned();
            for sijill in [
                &mut natija.khatt,
                &mut natija.sima,
                &mut natija.ittijah,
                &mut natija.thaqafa,
            ] {
                sijill.sajjil(Rutba::Imtidad, false, sabab.clone());
            }
            if idadat_najahat {
                return Ok(natija);
            }
            return Err(KhataGodot::ImtidadMarfud { sabab: sabab_shamil(&natija) });
        }

        natija.khadim = musajjil.ism_khadim().ok();
        if natija.khadim_yushakkil() == Some(false) {
            tracing::warn!(
                khadim = natija.khadim.as_deref().unwrap_or(""),
                "this game runs Godot's fallback text server, which does no shaping and no \
                 bidi; the font and locale will install and the text will not join"
            );
        }

        let (kutibat, muakkada, fashila) = self.rutbat_idadat_hayya(musajjil);
        let mulahaza = format!(
            "{kutibat} project setting(s) written live, {muakkada} read back equal{}; a \
             stored setting is not the same as an applied one, so this is recorded and not \
             counted as confirmation",
            if fashila.is_empty() {
                String::new()
            } else {
                format!(" (refused: {})", fashila.join("; "))
            }
        );
        for sijill in [
            &mut natija.khatt,
            &mut natija.sima,
            &mut natija.ittijah,
            &mut natija.thaqafa,
        ] {
            sijill.sajjil(Rutba::Imtidad, false, mulahaza.clone());
        }

        self.rutbat_khatt(musajjil, &mut natija);
        self.rutbat_sima(musajjil, &mut natija);
        self.rutbat_ittijah(musajjil, &mut natija);
        self.rutbat_thaqafa(musajjil, &mut natija);

        let shay_muakkad = natija.khatt.muakkad()
            || natija.sima.muakkad()
            || natija.ittijah.muakkad()
            || natija.thaqafa.muakkad();
        if shay_muakkad || idadat_najahat {
            Ok(natija)
        } else {
            Err(KhataGodot::ImtidadMarfud { sabab: sabab_shamil(&natija) })
        }
    }

    /// Removes the configuration rung: deletes the override file if it is
    /// Taarib's.
    ///
    /// Rung three needs no undo. A `GDExtension` that is not loaded has changed
    /// nothing, and this path installs no hook and rewrites no game file, so
    /// uninstalling the whole Godot 4 adapter is deleting the patch package,
    /// the extension library and this one file.
    ///
    /// # Errors
    ///
    /// Whatever [`MalafTajawuz::tarajua`] refuses.
    pub fn tarajua(&self) -> Result<bool, KhataGodot> {
        self.tajawuz.tarajua()
    }
}

/// Every concern's rung notes joined into one sentence, for an error's `sabab`.
///
/// A free function rather than a method because it reads only the record: a
/// refusal message that also consulted the configuration could describe a rung
/// that never ran.
fn sabab_shamil(natija: &NatijatKhadim) -> String {
    format!(
        "font — {}. theme — {}. direction — {}. locale — {}.",
        natija.khatt.sabab(),
        natija.sima.sabab(),
        natija.ittijah.sabab(),
        natija.thaqafa.sabab()
    )
}
