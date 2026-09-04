//! التوصيل — how translated text reaches a Godot 3 game, which is the half the
//! takeover has always been waiting for.
//!
//! [`crate::istila`] hooks `Font::draw`, `Font::draw_char` and
//! `Font::get_string_size`, copies the string the game is about to draw, and
//! shapes it. Read that sentence again with the emphasis where it belongs: **the
//! string the game is about to draw**. A takeover installed on a game whose
//! strings are still English intercepts English, [`crate::istila::yahtaj`]
//! answers `false` because there is no Arabic in it, and every hook forwards to
//! the engine. The takeover is not idle because it failed; it is idle because
//! nothing ever put Arabic into the game.
//!
//! Putting Arabic into the game is this module. It is the Godot 3 counterpart of
//! [`crate::khadim_nusus`], and the two engines make it a genuinely different
//! job rather than the same job with different spellings.
//!
//! ## What Godot 3 and Godot 4 do not share
//!
//! | | Godot 3 | Godot 4 |
//! | --- | --- | --- |
//! | locale keys | `locale/test`, `locale/fallback`, `locale/translations` | the same three under `internationalization/` |
//! | list literal | `PoolStringArray( … )` | `PackedStringArray( … )`, which 3.2 cannot parse |
//! | translation class | `Translation`, `PHashTranslation` | `Translation`, `OptimizedTranslation` |
//! | `messages` | `PoolStringArray` of alternating source and target | `Dictionary` of `StringName` |
//! | resource format | version 3, fourteen reserved words | version 4+, flags and a UID carved out of them |
//! | additive package | **none — nothing mounts a second pack** | a `.pck` the extension mounts |
//! | root direction | no such setting | `root_node_layout_direction` |
//! | text driver | no text server at all | `text_driver` |
//!
//! The row that decides this module's shape is the sixth. Godot 4's delivery is
//! an additive `.pck` layered over the game's; Godot 3 has
//! `ProjectSettings.load_resource_pack` too, but nothing calls it at startup —
//! the engine mounts exactly one package, the one named after the executable,
//! and that is the game's own. A patch package for a Godot 3 game would be a
//! file nothing opens.
//!
//! What Godot 3 *does* do at startup, in `TranslationServer::setup`, is walk
//! `locale/translations` and hand each entry to `ResourceLoader::load`. That
//! loader takes a `user://` path and an absolute filesystem path as readily as a
//! `res://` one — `FileAccess::create_for_path` branches on the prefix and falls
//! through to the ordinary filesystem — so a translation resource written
//! *beside* the game, named by a setting in the override file the engine already
//! reads, arrives with nothing injected into the process at all. That is this
//! module's rung one, and it is the same shape as Godot 4's: the cheapest rung
//! is the one that survives an anti-tamper system, a game update, and a player
//! who does not want a library in their game.
//!
//! ## The ladder
//!
//! 1. **Configuration.** The generated `.translation` at [`MawdiTarjama`]'s
//!    location, and [`MALAF_TAJAWUZ`] beside the executable naming it, forcing
//!    the locale, and preserving the game's own translation list. Nothing is
//!    injected and uninstalling is deleting two files.
//! 2. **Command line.** `--language ar`, which Godot 3 has spelled `-l` since
//!    3.0 and which outlives a launcher that regenerates its configuration. Thin
//!    on purpose: it carries the locale and cannot reach the translation list,
//!    so it is only ever a repair for rung one's locale half.
//! 3. **The takeover.** [`crate::istila`], armed through
//!    [`crate::bidaya::thabbit_istila`]. This rung does not deliver text — rung
//!    one does — it makes the delivered text *legible*, and without it Godot 3
//!    draws the Arabic rung one delivered as isolated letters left to right.
//!
//! Rungs one and two are honest about the fact that they are not enough on their
//! own, and [`SijillTawseel`] records the shaping question separately from the
//! delivery question for exactly that reason. A patch that reports "installed"
//! while the text on screen is unreadable would be the worst outcome available
//! here, and it is the one this module refuses to produce.
//!
//! ## Nothing here opens a game file
//!
//! [`TawseelThalith`] holds a [`Tarjama`] and writes two files: the generated
//! resource and the override. The game's own package is never opened, never
//! rewritten and never even named — the game's existing translation list is
//! *passed in* by the caller that read it, for the same reason
//! [`crate::khadim_nusus::KhadimNusus::bi_tarjamat_luba`] takes one: the
//! `locale/translations` setting replaces the whole list rather than adding to
//! it, and a patch that took away every language the game shipped with in
//! exchange for Arabic is a worse patch than none.

use std::path::{Path, PathBuf};

use taarib_usus::masarat;

use crate::khadim_nusus::{
    Lahja, MadkhalIdad, MalafTajawuz, QeemaIdad, lugha_wasm, wasm_maqbul,
};
use crate::khata::{KhataGodot, tul_u64};
use crate::pck::tarjama::{JeelMawrid, Tarjama, TarjamaMurakkaza};

/// Godot's user-side project-settings override, beside the executable.
///
/// The same file name Godot 4 uses, read by a different loader from a different
/// namespace of keys. Godot 3 reads it in `ProjectSettings::_setup`, from the
/// directory the executable lives in, immediately after the package it found
/// beside that executable — so every key in it overrides the packed
/// `project.binary` for the launch that reads it.
pub use taarib_mustalahat::muharrik::asmaa_muharrik::TAJAWUZ_GODOT as MALAF_TAJAWUZ;

/// The locale the patch registers and asks for.
///
/// Plain `ar`. Godot 3's `TranslationServer::get_message` walks from the exact
/// locale to its two-letter prefix, so a translation filed under `ar` is reached
/// by a player whose system reports `ar_EG` or `ar_MA`, and one filed under
/// `ar_SA` is reached by neither.
pub const WASM_ARABI: &str = "ar";

// ---------------------------------------------------------------------------
// Godot 3's own project settings
// ---------------------------------------------------------------------------

/// Forces the locale the project runs in.
///
/// `locale/test`, read by `TranslationServer::setup` before the system locale
/// is consulted. **Not** `internationalization/locale/test`: that namespace is
/// Godot 4's, and a Godot 3 engine handed it stores an unknown setting nothing
/// reads while its own key keeps its default.
pub const MIFTAH_THAQAFA_IKHTIBAR: &str = "locale/test";

/// The locale used when the system locale has no translation.
pub const MIFTAH_THAQAFA_IHTIYAT: &str = "locale/fallback";

/// The list of `.translation` resources the engine loads at start.
///
/// A `PoolStringArray`. `TranslationServer::load_translations` hands every entry
/// to `ResourceLoader::load`, which accepts `res://`, `user://` and absolute
/// filesystem paths — that last fact is the whole of rung one.
///
/// The value replaces the list rather than extending it, so
/// [`TawseelThalith::bi_tarjamat_luba`] exists to write the game's own entries
/// back beside the patch's.
pub const MIFTAH_TARJAMAT: &str = "locale/translations";

/// The `GDNative` libraries Godot 3 loads and calls as singletons.
///
/// A `PoolStringArray` of `res://` paths to `.gdnlib` resources. This is how the
/// takeover's library is entered at all — see [`crate::bidaya`] — and it is
/// written here only so that a patch that adds one does not delete whatever the
/// game already listed.
pub const MIFTAH_MUFRADAT: &str = "gdnative/singletons";

/// The default font for every control the theme reaches.
///
/// A path to a `Font` **resource**, not to a font file: Godot 3 wants a
/// `BitmapFont` or a `DynamicFont`, and a `.ttf` named here is a resource the
/// loader refuses. Written only when the caller supplies such a path, and never
/// invented — see [`TawseelThalith::bi_khatt`].
pub const MIFTAH_KHATT_SIMA: &str = "gui/theme/custom_font";

/// The launch option that carries the locale.
///
/// `--language`, spelled `-l` as well, present in every Godot 3 since 3.0.
pub const KHIYAR_THAQAFA: &str = "--language";

// ---------------------------------------------------------------------------
// Where the resource goes
// ---------------------------------------------------------------------------

/// The largest generated translation resource this module will write.
///
/// Sixty-four mebibytes, the same ceiling the Godot 4 path puts on the
/// translation it hands the engine. A `.translation` for a very large script is
/// single-digit megabytes.
pub const AQSA_HAJM_TARJAMA: u64 = 64 * 1024 * 1024;

/// Where the generated `.translation` is written, and what the setting calls it.
///
/// Two facts that must agree and are easy to let drift apart: the place on disk
/// this module writes, and the string `locale/translations` carries. They are
/// one value here so that a caller cannot supply one without the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawdiTarjama {
    mutlaq: PathBuf,
    marja: String,
}

impl MawdiTarjama {
    /// A location inside the game's own writable user directory.
    ///
    /// `jidhr` is that directory as the operating system resolves it — Godot 3
    /// puts it under the platform's data directory, named from
    /// `application/config/name` — and `nisbi` is the path within it, which
    /// becomes a `user://` reference in the setting.
    ///
    /// Preferred over an absolute path because a `user://` reference does not
    /// change when the game is moved, and because the directory is one Godot
    /// itself created and the player's own saves already live in.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when `nisbi` is empty, absolute, or carries
    /// a `..` component — a reference the engine would resolve outside the user
    /// directory is a patch writing somewhere it was not asked to.
    pub fn mustakhdim(jidhr: &Path, nisbi: &str) -> Result<Self, KhataGodot> {
        let munaddaf = nisbi.trim_start_matches('/');
        if munaddaf.is_empty()
            || Path::new(munaddaf).is_absolute()
            || munaddaf.split(['/', '\\']).any(|juz| juz == "..")
        {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{nisbi:?} is not a path inside the game's user directory, and a \
                     translation reference that leaves it would name a file the patch was \
                     never asked to write"
                ),
            });
        }
        let mut mutlaq = jidhr.to_path_buf();
        for juz in munaddaf.split(['/', '\\']).filter(|juz| !juz.is_empty()) {
            mutlaq.push(juz);
        }
        Ok(Self { mutlaq, marja: format!("user://{munaddaf}") })
    }

    /// A location named by its absolute path on disk.
    ///
    /// Godot 3's `ResourceLoader::load` takes one: `FileAccess::create_for_path`
    /// sends anything that is neither `res://` nor `user://` to the ordinary
    /// filesystem, and `ProjectSettings::localize_path` leaves a path outside
    /// the resource directory as it found it. Offered for the installer that
    /// would rather keep every file it wrote in one directory beside the game
    /// than in the player's data directory.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the path is relative — which the
    /// engine would resolve against `res://` and not find — or when it is not
    /// valid UTF-8, since the setting is text.
    pub fn mutlaq(masar: &Path) -> Result<Self, KhataGodot> {
        if !masar.is_absolute() {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{} is a relative path, and Godot resolves a relative translation entry \
                     against res:// — which is inside the game's own package",
                    masar.display()
                ),
            });
        }
        let Some(marja) = masar.to_str() else {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{} is not valid UTF-8, and the project setting that would name it is text",
                    masar.display()
                ),
            });
        };
        Ok(Self { mutlaq: masar.to_path_buf(), marja: marja.to_owned() })
    }

    /// Where the file is written.
    #[must_use]
    pub fn mutlaq_masar(&self) -> &Path {
        &self.mutlaq
    }

    /// The string `locale/translations` carries.
    #[must_use]
    pub fn marja(&self) -> &str {
        &self.marja
    }
}

// ---------------------------------------------------------------------------
// The ladder, and the record of which rung achieved what
// ---------------------------------------------------------------------------

/// One rung of the Godot 3 ladder.
///
/// Named separately from [`crate::khadim_nusus::Rutba`] rather than shared with
/// it, because the third rung is a different thing on each engine and a report
/// that called the Godot 3 takeover an "extension" would be describing Godot 4's
/// delivery to somebody reading about a Godot 3 game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RutbatThalith {
    /// The generated resource and [`MALAF_TAJAWUZ`]. Nothing is injected.
    Idadat,
    /// Launch options, for a launcher that rewrites its configuration.
    SatrAwamir,
    /// [`crate::istila`], through the `GDNative` library.
    Istila,
}

impl RutbatThalith {
    /// A stable short name for logs and diagnostics bundles.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Idadat => "idadat",
            Self::SatrAwamir => "satr-awamir",
            Self::Istila => "istila",
        }
    }

    /// The rung's ordinal, one-based, as the ladder is documented.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Idadat => 1,
            Self::SatrAwamir => 2,
            Self::Istila => 3,
        }
    }

    /// Whether this rung puts code inside the game's process.
    #[must_use]
    pub const fn muqtahim(self) -> bool {
        matches!(self, Self::Istila)
    }
}

/// What one rung did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatijatRutbaThalith {
    /// Which rung.
    pub rutba: RutbatThalith,
    /// Whether the rung's effect was established rather than merely attempted.
    ///
    /// For rung one that means the bytes were written and read back; it does
    /// **not** mean the engine has acted on them, because Godot 3 reads
    /// `override.cfg` during its own startup and nothing rereads it afterwards.
    pub muakkada: bool,
    /// One sentence naming what happened, for the log and the bundle.
    pub mulahaza: String,
}

/// The record of one concern's walk down the ladder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SijillTawseel {
    /// Every rung attempted, in the order attempted.
    pub rutab: Vec<NatijatRutbaThalith>,
    /// The rung whose effect was established, if any.
    pub nafidha: Option<RutbatThalith>,
}

impl SijillTawseel {
    /// Records a rung.
    pub fn sajjil(
        &mut self,
        rutba: RutbatThalith,
        muakkada: bool,
        mulahaza: impl Into<String>,
    ) {
        if muakkada && self.nafidha.is_none() {
            self.nafidha = Some(rutba);
        }
        self.rutab.push(NatijatRutbaThalith { rutba, muakkada, mulahaza: mulahaza.into() });
    }

    /// Whether any rung was established.
    #[must_use]
    pub const fn muakkad(&self) -> bool {
        self.nafidha.is_some()
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
///
/// Three concerns and not one verdict, because they fail independently and the
/// combination decides what a player sees. Arabic delivered and not shaped is
/// unreadable text; Arabic shaped and not delivered is the game's own language
/// drawn correctly; both is the patch working.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NatijatTawseel {
    /// The generated `.translation` reaching a place the engine loads from.
    pub mawrid: SijillTawseel,
    /// The locale being asked for.
    pub thaqafa: SijillTawseel,
    /// Whether anything will shape the Arabic once it arrives.
    pub rasm: SijillTawseel,
}

impl NatijatTawseel {
    /// Whether the text will arrive **and** be legible.
    ///
    /// Both, deliberately. Either alone is a patch a player would report as
    /// broken, and a summary that answered `true` for one of them would be the
    /// summary that let it ship.
    #[must_use]
    pub const fn muakkad(&self) -> bool {
        self.mawrid.muakkad() && self.thaqafa.muakkad() && self.rasm.muakkad()
    }

    /// Whether the translated text reaches the game, whatever it then looks like.
    #[must_use]
    pub const fn wusul(&self) -> bool {
        self.mawrid.muakkad() && self.thaqafa.muakkad()
    }
}

// ---------------------------------------------------------------------------
// The configuration
// ---------------------------------------------------------------------------

/// Everything the Godot 3 path delivers, and the ladder that delivers it.
///
/// Built by the caller from the patch and walked once by
/// [`TawseelThalith::hayyi`]. Nothing here reads a game file: the translation is
/// an owned [`Tarjama`], the game's own translation list and singleton list are
/// passed in, and the two paths written are the generated resource and the
/// override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TawseelThalith {
    tajawuz: MalafTajawuz,
    mawdi: MawdiTarjama,
    tarjama: Option<Tarjama>,
    murakkaza: bool,
    tarjamat_luba: Vec<String>,
    mufradat_luba: Vec<String>,
    gdnlib: Option<String>,
    khatt: Option<String>,
    wasm: String,
    muharrik: (u32, u32),
    yastawli: bool,
}

impl TawseelThalith {
    /// Builds the configuration against one game's override file and one place
    /// to put the generated resource.
    ///
    /// Defaults: the `ar` locale, the plain `Translation` form, the engine
    /// version recorded as 3.0 — the oldest 3.x, so that every 3.x accepts the
    /// resource, since Godot refuses a file whose recorded engine major is above
    /// its own and compares nothing else — and no takeover, because whether one
    /// is available is the caller's answer and not this module's guess.
    #[must_use]
    pub fn jadeed(tajawuz: MalafTajawuz, mawdi: MawdiTarjama) -> Self {
        Self {
            tajawuz,
            mawdi,
            tarjama: None,
            murakkaza: false,
            tarjamat_luba: Vec::new(),
            mufradat_luba: Vec::new(),
            gdnlib: None,
            khatt: None,
            wasm: WASM_ARABI.to_owned(),
            muharrik: (3, 0),
            yastawli: false,
        }
    }

    /// Supplies the messages the patch delivers.
    ///
    /// The locale on the [`Tarjama`] is overwritten with this configuration's at
    /// generation time, so a translation lifted out of one game and delivered to
    /// another does not carry the first game's spelling of Arabic into the
    /// second's `TranslationServer`.
    #[must_use]
    pub fn bi_tarjama(mut self, tarjama: Tarjama) -> Self {
        self.tarjama = Some(tarjama);
        self
    }

    /// Generates the hash-table form (`PHashTranslation`) instead of the plain
    /// message list.
    ///
    /// Off by default. The two are equally correct to the engine and it loads
    /// either; the plain form keeps the source strings in the file, which means
    /// a patch can be read back and audited, and the hash-table form does not —
    /// it stores only hashes of the sources, so nothing can enumerate it
    /// afterwards. That is a real cost to pay for a lookup that is already fast
    /// enough, and it is paid only when asked for.
    #[must_use]
    pub const fn bi_murakkaza(mut self, murakkaza: bool) -> Self {
        self.murakkaza = murakkaza;
        self
    }

    /// The game's own `locale/translations` entries, so rung one keeps them.
    #[must_use]
    pub fn bi_tarjamat_luba(mut self, tarjamat: Vec<String>) -> Self {
        self.tarjamat_luba = tarjamat;
        self
    }

    /// The game's own `gdnative/singletons` entries, so rung one keeps them.
    #[must_use]
    pub fn bi_mufradat_luba(mut self, mufradat: Vec<String>) -> Self {
        self.mufradat_luba = mufradat;
        self
    }

    /// The `res://` path of the `.gdnlib` the takeover is entered through.
    ///
    /// Written into `gdnative/singletons` beside whatever the game already
    /// listed. [`None`] leaves the setting alone entirely rather than writing a
    /// list holding only the game's own entries, because rewriting a setting to
    /// the value it already had is a change an uninstall then has to undo.
    #[must_use]
    pub fn bi_gdnlib(mut self, masar: Option<String>) -> Self {
        self.gdnlib = masar;
        self
    }

    /// A Godot 3 `Font` resource to install as the theme default.
    ///
    /// A path to a `BitmapFont` or `DynamicFont` **resource**, which a patch has
    /// to have built; a `.ttf` path here is a file the engine's theme loader
    /// refuses. Left [`None`] by every caller that installs the takeover, since
    /// the takeover draws from Taarib's own atlas and never asks the engine for
    /// a font at all.
    #[must_use]
    pub fn bi_khatt(mut self, masar: Option<String>) -> Self {
        self.khatt = masar;
        self
    }

    /// Overrides the locale tag.
    #[must_use]
    pub fn bi_wasm(mut self, wasm: impl Into<String>) -> Self {
        self.wasm = wasm.into();
        self
    }

    /// The engine version recorded in the generated resource's header.
    ///
    /// Godot refuses a resource whose recorded major version is above its own
    /// and ignores the minor, so the safe value is the lowest 3.x — which is the
    /// default. Passing the game's own is fine and slightly more honest in a
    /// diagnostics bundle.
    #[must_use]
    pub const fn bi_muharrik(mut self, muharrik: (u32, u32)) -> Self {
        self.muharrik = muharrik;
        self
    }

    /// Whether the takeover is going to be installed in this game.
    ///
    /// The caller's answer, because only the caller knows whether the build can
    /// be hooked — see [`crate::isdar::Bina::masar`]. It changes nothing this
    /// module writes; it changes what the report says about whether the Arabic
    /// this module delivers will be legible, which is the one thing a player
    /// needs told before they install.
    #[must_use]
    pub const fn bi_istila(mut self, yastawli: bool) -> Self {
        self.yastawli = yastawli;
        self
    }

    /// The locale this configuration asks for.
    #[must_use]
    pub fn wasm(&self) -> &str {
        &self.wasm
    }

    /// Where the generated resource goes.
    #[must_use]
    pub const fn mawdi(&self) -> &MawdiTarjama {
        &self.mawdi
    }

    /// The override file this writes.
    #[must_use]
    pub const fn tajawuz(&self) -> &MalafTajawuz {
        &self.tajawuz
    }

    /// The translation list rung one writes: the game's own, then the patch's.
    #[must_use]
    pub fn qaimat_tarjamat(&self) -> Vec<String> {
        let mut qaima = self.tarjamat_luba.clone();
        let marja = self.mawdi.marja();
        if self.tarjama.is_some() && !qaima.iter().any(|mawjud| mawjud == marja) {
            qaima.push(marja.to_owned());
        }
        qaima
    }

    /// The singleton list rung one writes, or [`None`] to leave the key alone.
    #[must_use]
    pub fn qaimat_mufradat(&self) -> Option<Vec<String>> {
        let gdnlib = self.gdnlib.as_ref()?;
        let mut qaima = self.mufradat_luba.clone();
        if !qaima.iter().any(|mawjud| mawjud == gdnlib) {
            qaima.push(gdnlib.clone());
        }
        Some(qaima)
    }

    /// The project settings this configuration owns, in write order.
    #[must_use]
    pub fn madakhil(&self) -> Vec<MadkhalIdad> {
        let mut madakhil = Vec::with_capacity(5);
        let tarjamat = self.qaimat_tarjamat();
        if !tarjamat.is_empty() {
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
        if let Some(khatt) = self.khatt.as_ref() {
            madakhil.push(MadkhalIdad::jadeed(MIFTAH_KHATT_SIMA, QeemaIdad::Nass(khatt.clone())));
        }
        if let Some(mufradat) = self.qaimat_mufradat() {
            madakhil.push(MadkhalIdad::jadeed(MIFTAH_MUFRADAT, QeemaIdad::Qaima(mufradat)));
        }
        madakhil
    }

    /// The generated `.translation`, as the bytes that go on disk.
    ///
    /// A Godot 3 resource in every respect the two generations disagree about:
    /// format version 3, fourteen reserved words, `PoolStringArray` messages,
    /// and — when [`TawseelThalith::bi_murakkaza`] asked for the hash table —
    /// the class name `PHashTranslation`, which is the only one Godot 3 has.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when no translation was supplied, and
    /// [`KhataGodot::HajmMufrit`] when the generated resource is above
    /// [`AQSA_HAJM_TARJAMA`]. Everything
    /// [`TarjamaMurakkaza::min_tarjama`] and the resource writer raise passes
    /// through: a message list holding one source twice, a table above a
    /// ceiling, a string longer than its length field.
    pub fn mawrid(&self) -> Result<Vec<u8>, KhataGodot> {
        let Some(asl) = self.tarjama.as_ref() else {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: "no translation was supplied, so there is nothing to deliver and \
                        writing a settings file that named an absent resource would leave the \
                        game reporting a missing file at every launch"
                    .to_owned(),
            });
        };
        let mut tarjama = asl.clone();
        tarjama.dhaa_thaqafa(&self.wasm);
        let bayt = if self.murakkaza {
            TarjamaMurakkaza::min_tarjama(&tarjama)?
                .ila_bayt(JeelMawrid::Thalith, self.muharrik)?
        } else {
            tarjama.ila_bayt(JeelMawrid::Thalith, self.muharrik)?
        };
        let tul = tul_u64(bayt.len());
        if tul > AQSA_HAJM_TARJAMA {
            return Err(KhataGodot::HajmMufrit {
                haql: "the generated translation resource",
                qeema: tul,
                saqf: AQSA_HAJM_TARJAMA,
            });
        }
        Ok(bayt)
    }

    /// Rung one, first half: write the generated resource where the setting will
    /// name it.
    ///
    /// The write is atomic and the parent directory is created, because the
    /// user directory may not exist before the game's first launch and a
    /// half-written resource is a game that reports a corrupt file on every
    /// start.
    ///
    /// # Errors
    ///
    /// Everything [`TawseelThalith::mawrid`] raises, and
    /// [`KhataGodot::KhataMalaf`] naming the path when the directory cannot be
    /// created or the file cannot be written.
    pub fn rutbat_mawrid(&self) -> Result<u64, KhataGodot> {
        let bayt = self.mawrid()?;
        let masar = self.mawdi.mutlaq_masar();
        if let Some(mujallad) = masar.parent() {
            std::fs::create_dir_all(mujallad).map_err(|sabab| KhataGodot::KhataMalaf {
                masar: mujallad.to_path_buf(),
                sabab,
            })?;
        }
        masarat::kitaba_dharra(masar, &bayt).map_err(|khata| KhataGodot::ImtidadMarfud {
            sabab: format!("{} could not be written: {}", masar.display(), khata.li_sijill()),
        })?;
        Ok(tul_u64(bayt.len()))
    }

    /// Rung one, second half: write the override file in Godot 3's spelling.
    ///
    /// # Errors
    ///
    /// Whatever [`MalafTajawuz::aktub_bi`] refuses, unchanged, so the caller can
    /// name the path in its own message.
    pub fn rutbat_idadat(&self) -> Result<(), KhataGodot> {
        self.tajawuz.aktub_bi(Lahja::Thalith, &self.madakhil())
    }

    /// Rung two: the launch options that carry the same request.
    ///
    /// One option, and only the locale. Godot 3 has no launch option that sets
    /// an arbitrary project setting, so the translation list cannot be reached
    /// from a command line at all — saying so is more useful than offering
    /// options that do nothing.
    #[must_use]
    pub fn rutbat_satr(&self) -> Vec<String> {
        if self.wasm.trim().is_empty() {
            return Vec::new();
        }
        vec![format!("{KHIYAR_THAQAFA} {}", self.wasm)]
    }

    /// Reads the written resource back and confirms it is what was written.
    ///
    /// The read-back this rung can actually perform. It cannot ask the engine
    /// anything — Godot 3 reads `override.cfg` and its translations during its
    /// own startup, long before any library a patch installs is entered — so
    /// what is checked is that the bytes on disk parse as a Godot 3 translation
    /// resource of the expected class and locale, and answer to a message the
    /// patch put in it. A file that passes that is a file the engine's own
    /// loader will accept, because it is the same shape read by the same reader
    /// this crate round-trips against engine-written resources.
    fn tahaqquq_mawrid(&self) -> Result<usize, String> {
        let masar = self.mawdi.mutlaq_masar();
        let bayt = std::fs::read(masar).map_err(|sabab| format!("{} could not be read back: {sabab}", masar.display()))?;
        let asl = self.tarjama.as_ref().ok_or_else(|| "no translation was supplied".to_owned())?;
        let awwal = asl.rasail().first();
        if self.murakkaza {
            let jadwal = TarjamaMurakkaza::min_bayt(&bayt)
                .map_err(|khata| format!("the written resource did not read back: {khata}"))?;
            if !wasm_maqbul(jadwal.thaqafa(), &self.wasm) {
                return Err(format!(
                    "the written resource carries the locale {:?} and {:?} was asked for",
                    jadwal.thaqafa(),
                    self.wasm
                ));
            }
            if let Some((masdar, hadaf)) = awwal {
                match jadwal.ibhath(masdar) {
                    Ok(Some(wujid)) if &wujid == hadaf => {}
                    Ok(other) => {
                        return Err(format!(
                            "the written hash table answers {other:?} for the patch's first \
                             source string, and the engine's lookup is this one"
                        ));
                    }
                    Err(khata) => return Err(format!("the written hash table refused: {khata}")),
                }
            }
            Ok(asl.adad())
        } else {
            let tarjama = Tarjama::min_bayt(&bayt)
                .map_err(|khata| format!("the written resource did not read back: {khata}"))?;
            if !wasm_maqbul(tarjama.thaqafa(), &self.wasm) {
                return Err(format!(
                    "the written resource carries the locale {:?} and {:?} was asked for",
                    tarjama.thaqafa(),
                    self.wasm
                ));
            }
            if tarjama.adad() != asl.adad() {
                return Err(format!(
                    "{} messages were written and {} read back",
                    asl.adad(),
                    tarjama.adad()
                ));
            }
            if let Some((masdar, hadaf)) = awwal
                && tarjama.ibhath(masdar) != Some(hadaf.as_str())
            {
                return Err(
                    "the written resource does not answer with the patch's own first \
                     translation"
                        .to_owned(),
                );
            }
            Ok(tarjama.adad())
        }
    }

    /// Walks the ladder and reports which rung achieved each concern.
    ///
    /// Rung one is attempted in order and stops if its first half does not
    /// finish: the resource is written and read back, and only then is the
    /// override written, because an override naming a file that is not there is
    /// a game that logs a missing resource at every launch and has had its own
    /// translation list replaced with one carrying a dead entry. Rung two is
    /// offered to the installer. Rung three is not performed here: arming the
    /// takeover means
    /// being inside the game's process with this build's draw addresses in hand,
    /// which is [`crate::bidaya`]'s business. What is recorded for it is whether
    /// it is going to happen, because that is what decides whether the Arabic
    /// this module delivers will be readable.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] naming every route that was tried, and only
    /// when the translated text does not reach the game by any of them. A Godot
    /// 3 game that refuses all of this keeps running in its own language, which
    /// is a warning and not a failure.
    pub fn hayyi(&self) -> Result<NatijatTawseel, KhataGodot> {
        let mut natija = NatijatTawseel::default();
        let masar_mawrid = self.mawdi.mutlaq_masar().display().to_string();
        let masar_tajawuz = self.tajawuz.masar().display().to_string();

        let kutiba = match self.rutbat_mawrid() {
            Ok(hajm) => {
                match self.tahaqquq_mawrid() {
                    Ok(adad) => {
                        natija.mawrid.sajjil(
                            RutbatThalith::Idadat,
                            true,
                            format!(
                                "{adad} message(s) written to {masar_mawrid} as {hajm} bytes \
                                 of Godot 3 resource, read back and confirmed to answer with \
                                 the patch's own translation"
                            ),
                        );
                        true
                    }
                    Err(sabab) => {
                        natija.mawrid.sajjil(RutbatThalith::Idadat, false, sabab);
                        false
                    }
                }
            }
            Err(khata) => {
                natija.mawrid.sajjil(RutbatThalith::Idadat, false, khata.to_string());
                false
            }
        };

        if kutiba {
            match self.rutbat_idadat() {
                Ok(()) => natija.thaqafa.sajjil(
                    RutbatThalith::Idadat,
                    true,
                    format!(
                        "{masar_tajawuz} written: locale {} forced, and the translation list \
                         carries the game's own {} entry/entries beside the patch's. Godot \
                         reads this file during startup, so it applies at the next launch",
                        self.wasm,
                        self.tarjamat_luba.len()
                    ),
                ),
                Err(khata) => {
                    natija.thaqafa.sajjil(RutbatThalith::Idadat, false, khata.to_string());
                }
            }
        } else {
            // The override is not written when the resource it would name did
            // not land. Godot resolves every `locale/translations` entry at
            // startup and logs a failure for each one it cannot open, so an
            // override written on its own would turn a patch that did nothing
            // into a patch that makes the game complain at every launch — and
            // would additionally have replaced the game's own translation list
            // with one carrying a dead entry.
            natija.thaqafa.sajjil(
                RutbatThalith::Idadat,
                false,
                format!(
                    "{masar_tajawuz} was not written, because the resource it would have \
                     named did not land: an override listing a translation the engine cannot \
                     open is a game that reports a missing file at every launch"
                ),
            );
        }

        let khiyarat = self.rutbat_satr();
        natija.thaqafa.sajjil(
            RutbatThalith::SatrAwamir,
            false,
            format!(
                "{} launch option(s) offered to the installer, which carry the locale and \
                 cannot reach the translation list",
                khiyarat.len()
            ),
        );
        natija.mawrid.sajjil(
            RutbatThalith::SatrAwamir,
            false,
            "Godot 3 has no launch option that adds a translation resource".to_owned(),
        );

        natija.rasm.sajjil(
            RutbatThalith::Idadat,
            false,
            "no project setting makes Godot 3 shape Arabic; it has no text server and draws \
             each character through Font::draw_char in logical order"
                .to_owned(),
        );
        natija.rasm.sajjil(
            RutbatThalith::SatrAwamir,
            false,
            "no launch option makes Godot 3 shape Arabic either".to_owned(),
        );
        natija.rasm.sajjil(
            RutbatThalith::Istila,
            self.yastawli,
            if self.yastawli {
                "the takeover is installed in this game, so the delivered Arabic is shaped, \
                 ordered and drawn by Taarib"
                    .to_owned()
            } else {
                "no takeover is installed in this game, so the delivered Arabic will be drawn \
                 by Godot 3 itself: every letter in its isolated form, left to right. The \
                 text arrives and is not readable until the takeover or the glyph transport \
                 is in place"
                    .to_owned()
            },
        );

        if natija.wusul() {
            Ok(natija)
        } else {
            Err(KhataGodot::ImtidadMarfud { sabab: sabab_shamil(&natija) })
        }
    }

    /// Removes what rung one wrote: the override file, if it is Taarib's, and
    /// the generated resource.
    ///
    /// Returns whether each was removed. The override is only removed when it
    /// carries Taarib's own marker — see [`MalafTajawuz::tarajua`] — and the
    /// resource is removed unconditionally, because it is a file that exists
    /// only because this module wrote it and its absence is the uninstalled
    /// state.
    ///
    /// # Errors
    ///
    /// Whatever [`MalafTajawuz::tarajua`] refuses, and
    /// [`KhataGodot::KhataMalaf`] when the resource is there and cannot be
    /// deleted.
    pub fn tarajua(&self) -> Result<(bool, bool), KhataGodot> {
        let tajawuz = self.tajawuz.tarajua()?;
        let masar = self.mawdi.mutlaq_masar();
        let mawrid = match std::fs::remove_file(masar) {
            Ok(()) => true,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => false,
            Err(sabab) => {
                return Err(KhataGodot::KhataMalaf { masar: masar.to_path_buf(), sabab });
            }
        };
        Ok((tajawuz, mawrid))
    }
}

/// Every concern's rung notes joined into one sentence, for an error's `sabab`.
fn sabab_shamil(natija: &NatijatTawseel) -> String {
    format!(
        "resource — {}. locale — {}. shaping — {}.",
        natija.mawrid.sabab(),
        natija.thaqafa.sabab(),
        natija.rasm.sabab()
    )
}

/// Which spelling of Arabic to use, given what the game already advertises.
///
/// The game's own when it has one, so the patch fills the entry the game's
/// language menu already draws rather than adding a second Arabic beside it, and
/// [`WASM_ARABI`] otherwise. The same rule the Godot 4 path applies, restated
/// here because Godot 3 spells a locale with an underscore and the comparison
/// has to be on the language subtag either way.
#[must_use]
pub fn wasm_mufaddal(matah: &[String]) -> String {
    matah
        .iter()
        .find(|wasm| lugha_wasm(wasm) == WASM_ARABI)
        .map_or_else(|| WASM_ARABI.to_owned(), |wasm| wasm.trim().to_owned())
}
