//! وكلاء التحميل — the loader slots beside a game's executable, and who holds them.
//!
//! A Windows game loads a module by name, and the loader looks in the
//! executable's own directory before it looks in `System32`. That is the whole
//! mechanism behind every DLL-proxy mod: put a file called `dinput8.dll` beside
//! the game, export what the real one exports, forward each call to the system
//! copy, and do your own work on the way past. `taarib-mudkhal` is exactly such
//! a proxy, published as `version.dll`.
//!
//! Which means the slot is a resource, and two mods cannot share one. A second
//! `dinput8.dll` written over the first does not chain — it *replaces* it, the
//! first mod is silently gone, and its own files are left behind pointing at a
//! loader that no longer exists. So before Taarib places its loader it has to
//! know what is already beside the game.
//!
//! ## What counts as a slot, and what does not
//!
//! [`WUKALA_NIZAM`] is the list this module answers from, and its membership
//! rule is narrow on purpose: a module that **ships with Windows and is not in
//! any redistributable**. Nothing in a game's own installation puts a
//! `winmm.dll` or a `dsound.dll` next to the executable, because there is
//! nowhere legitimate for one to come from — so a file with that name beside a
//! game is a proxy, with no guessing involved.
//!
//! The names deliberately left out are the ones a game *does* ship legitimately
//! and that a naive "is this a system-sounding DLL" test would flag: the
//! DirectX end-user redistributables (`d3dx9_43.dll`, `x3daudio1_7.dll`,
//! `xinput1_3.dll`, `xaudio2_*.dll`, `d3dcompiler_*.dll`), the Visual C++
//! runtimes (`msvcp*.dll`, `vcruntime*.dll`), and middleware
//! (`steam_api.dll`, `binkw32.dll`, `openal32.dll`). Every one of those is
//! normal beside a game and none is reported here. A survey that cried proxy
//! over `xinput1_3.dll` would be a survey nobody reads.
//!
//! ## Naming the mod, and refusing to guess
//!
//! "Something owns `dxgi.dll`" is a fact the user can do nothing with.
//! "ReShade is installed here" is one they can act on, and the difference is
//! worth the evidence it costs. [`tashkhis`] answers the second question from
//! two kinds of fact and no others:
//!
//! * **Companions** — a file or directory beside the proxy that only one
//!   product creates. `reshade-shaders/` is ReShade's. `enbseries.ini` is
//!   ENBSeries'. `doorstop_config.ini` is Unity Doorstop's. These cost one
//!   directory listing, which has already happened.
//! * **Marks** — a byte string inside the module itself, searched for in both
//!   ASCII and the UTF-16 that a Windows version resource is stored in. A file
//!   that contains `ENBSeries` is ENBSeries in a way that a file merely *named*
//!   `d3d9.dll` is not.
//!
//! A mark outweighs a companion, because a companion can be left behind by a
//! product that was removed and a mark cannot. Both together outweigh either.
//! And when two products tie — which happens, because Special K ships ReShade
//! integration and therefore contains the string `ReShade` — the answer is
//! [`HuwiyatWakeel::Multabis`] naming both candidates, never a coin toss.
//! Nothing identified at all is [`HuwiyatWakeel::Majhul`], reported by name and
//! size exactly as before.
//!
//! ## The second list, and why `xinput1_3.dll` is on it
//!
//! [`WUKALA_NIZAM`]'s membership rule — ships with Windows, in no
//! redistributable — is what lets a *name* be proof. It also excludes the
//! single most common name Ultimate ASI Loader is installed under, because
//! `xinput1_3.dll` really is a DirectX redistributable a game may ship.
//!
//! [`WUKALA_MUSHTARAKA`] is the answer: names that a game may legitimately
//! carry **and** that the loader ecosystem also takes. A file with one of those
//! names is reported **only when [`tashkhis`] names the product holding it**.
//! Identified, it is a mod and the user is told; unidentified, it is Microsoft's
//! own file and saying anything about it would be the survey crying wolf. That
//! is the whole of the rule, and it is why the identification had to exist
//! before the list could.

use std::io::Read as _;
use std::path::Path;

/// Windows modules that a game's own installation has no legitimate way to
/// place beside its executable.
///
/// Base names, lower-case, without the `.dll`. Every one of these ships as part
/// of Windows and appears in no redistributable a game may carry, so a file
/// with one of these names in a game directory was put there by a mod. The list
/// is the union of the slots the mod-loading ecosystem actually uses — the
/// graphics ones for `ReShade` and its relatives, the input and audio ones for
/// the ASI loaders, `winhttp` for BepInEx, `version` for Taarib's own loader.
///
/// `opengl32` carries one honest caveat: a handful of 1990s titles shipped a
/// vendor `MiniGL` driver under that name. Such a file would be reported here as
/// a slot in use, which is true — the loader really would prefer it — even
/// though nobody modded that game.
///
/// `dbghelp` used to be on this list and was **moved to
/// [`WUKALA_MUSHTARAKA`] against evidence**. A read-only sweep of thirty
/// installed games found four `dbghelp.dll` files and all four were Microsoft's
/// own — `Windows Image Helper`, 250-odd exports at ordinal base 1101,
/// forwarding `MiniDumpWriteDump` to `dbgcore` — three of them at Unreal
/// Engine's standard `Engine/Binaries/ThirdParty/DbgHelp/` and one at a game's
/// root beside its other vendor DLLs. A name that is genuine four times out of
/// four is not a name that proves a mod, whatever the membership rule says in
/// principle.
pub const WUKALA_NIZAM: [&str; 20] = [
    "d3d8",
    "d3d9",
    "d3d10",
    "d3d11",
    "d3d12",
    "dciman32",
    "ddraw",
    "dinput",
    "dinput8",
    "dsound",
    "dxgi",
    "iphlpapi",
    "msacm32",
    "opengl32",
    "version",
    "winhttp",
    "wininet",
    "winmm",
    "ws2_32",
    "wsock32",
];

/// Names a game may legitimately ship **and** that the loader ecosystem takes.
///
/// Reported only when [`tashkhis`] names the product holding one. The whole
/// point of [`WUKALA_NIZAM`] is that a name alone is proof; for everything here
/// a name alone is nothing, because Microsoft's DirectX redistributable really
/// does put `xinput1_3.dll` beside a game and RAD's video codec really is
/// `bink2w64.dll`. Identification is what separates the two, so these names
/// could not be surveyed at all until there was something to identify them
/// with.
///
/// `xinput1_3` is the reason this list exists. It is the single most common
/// name Ultimate ASI Loader is installed under and it is also a file half the
/// library ships, so the choice was to miss the most common loader or to cry
/// proxy over Microsoft's redistributable. Naming the product is the third
/// answer.
///
/// `dbghelp` is here on measured evidence rather than principle — see
/// [`WUKALA_NIZAM`].
pub const WUKALA_MUSHTARAKA: [&str; 8] = [
    "bink2w64",
    "binkw32",
    "dbghelp",
    "vorbisfile",
    "vorbishooked",
    "xinput1_3",
    "xinput1_4",
    "xlive",
];

/// The extension a proxy's own configuration or log carries, when it has one.
///
/// A proxy that reads settings names them after itself — `dinput8.ini` beside
/// `dinput8.dll` — because the file it is impersonating decides its own name.
/// Finding one is what turns "a module with a system name is here" into "a mod
/// with settings is here", and it is the detail that makes the line the user
/// reads before an install specific enough to act on.
const LAWAHIQ_RAFIQ: [&str; 5] = ["ini", "log", "cfg", "toml", "json"];

/// A mod-loading product this module can name from evidence on disk.
///
/// Families rather than vendors, and the distinction is deliberate. There is no
/// single "ASI loader" product — ThirteenAG's Ultimate ASI Loader and Alexander
/// Blade's GTA V loader are different programs that do the same job and are
/// installed under the same names — so [`Self::MuhammilAsi`] names what it can
/// prove and [`HuwiyatWakeel::Maaruf`] carries the marks that proved it. Where
/// one product owns its shape outright, as ReShade and ENBSeries do, the
/// variant is that product.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AilatWakeel {
    /// An `.asi` plugin loader: Ultimate ASI Loader, or Alexander Blade's.
    MuhammilAsi,
    /// ReShade.
    ///
    /// Spelled on the wire rather than left to the case convention, which would
    /// derive `re_shade` from the variant's internal capitals and put a name no
    /// product uses into a payload the interface reads.
    #[serde(rename = "reshade")]
    ReShade,
    /// ENBSeries.
    Enb,
    /// Special K.
    SpecialK,
    /// Unity Doorstop, which is how BepInEx gets in.
    Doorstop,
    /// DXVK, translating Direct3D to Vulkan.
    Dxvk,
    /// `d3d8to9`, translating Direct3D 8 to Direct3D 9.
    D3d8to9,
    /// `re4_tweaks`, for the Resident Evil 4 UHD port.
    Re4Tweaks,
    /// Taarib's own loader, which is a reinstall rather than a collision.
    Taarib,
}

/// How a product gets onto the presentation path, which decides whether the
/// overlay can share it.
///
/// This is the field the whole presentation-hook question turns on, and the
/// three answers are genuinely different rather than shades of one risk.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TariqatKhatf {
    /// It hands the game an object of its own.
    ///
    /// The game's swap chain or device pointer is the product's C++ class, not
    /// the system's, so its method table is the product's too. An overlay that
    /// finds a method table the documented way — make a device of your own,
    /// read the pointers out — reads the *system* table, hooks it, and is never
    /// called, because the game is not holding an object of that class. Nothing
    /// crashes and nothing draws.
    Ghilaf,
    /// It patches the system function where it lives.
    ///
    /// The method table still holds the system entry point and the patch is
    /// behind it, so an overlay that hooks the table sits in front of the patch
    /// and both run. This is the order that composes.
    Ruqaa,
    /// Either, depending on how it was installed.
    Kilahuma,
    /// It replaces the implementation outright.
    ///
    /// DXVK's `d3d9.dll` *is* Direct3D 9 in that process. There is no original
    /// behind it and nothing is hooked: the method table an overlay reads from
    /// a device of its own is DXVK's, because `Direct3DCreate9` resolved to
    /// DXVK for the overlay exactly as it did for the game. One implementation,
    /// one table, and the hook composes.
    Istibdal,
    /// It does not touch presentation at all.
    LaShay,
}

impl AilatWakeel {
    /// The product's name, as its own authors spell it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::MuhammilAsi => "an ASI plugin loader",
            Self::ReShade => "ReShade",
            Self::Enb => "ENBSeries",
            Self::SpecialK => "Special K",
            Self::Doorstop => "BepInEx (Unity Doorstop)",
            Self::Dxvk => "DXVK",
            Self::D3d8to9 => "d3d8to9",
            Self::Re4Tweaks => "re4_tweaks",
            Self::Taarib => "Taarib's own loader",
        }
    }

    /// The same, in Arabic.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self {
            Self::MuhammilAsi => "مُحمِّل إضافات ASI",
            Self::ReShade => "ReShade",
            Self::Enb => "ENBSeries",
            Self::SpecialK => "Special K",
            Self::Doorstop => "‏BepInEx (عبر Doorstop)",
            Self::Dxvk => "DXVK",
            Self::D3d8to9 => "d3d8to9",
            Self::Re4Tweaks => "re4_tweaks",
            Self::Taarib => "مُحمِّل تعريب نفسه",
        }
    }

    /// How it reaches presentation, if it does.
    #[must_use]
    pub const fn khatf(self) -> TariqatKhatf {
        match self {
            // Both wrap. ReShade's proxy returns its own `IDXGISwapChain` and
            // ENBSeries its own `IDirect3DDevice9`, which is what lets either
            // run its effect passes between the game's last draw and the
            // present it never actually makes.
            Self::ReShade | Self::Enb => TariqatKhatf::Ghilaf,
            // Special K does both, and which one depends on whether it was
            // installed as a local proxy or through its global injector.
            Self::SpecialK => TariqatKhatf::Kilahuma,
            Self::Dxvk | Self::D3d8to9 => TariqatKhatf::Istibdal,
            // `re4_tweaks` draws an ImGui panel over the game, so it is on the
            // presentation path — by patching, not by wrapping.
            Self::Re4Tweaks | Self::Taarib => TariqatKhatf::Ruqaa,
            Self::MuhammilAsi | Self::Doorstop => TariqatKhatf::LaShay,
        }
    }

    /// Whether the product documents a way to load a second proxy of the same
    /// name behind itself, and what it is called.
    ///
    /// This is the only thing that turns a refusal into something a user can
    /// act on. Taarib will not take a taken slot, but a product that chain-loads
    /// can be *told* to load Taarib, and naming the mechanism is the difference
    /// between "no" and "no, and here is the door".
    #[must_use]
    pub const fn tasalsul(self) -> Option<&'static str> {
        match self {
            Self::MuhammilAsi => Some(
                "it loads every `.asi` beside it, so a plugin needs no slot of its own, and \
                 it will load a displaced original renamed to `<name>Hooked.dll`",
            ),
            Self::ReShade => Some(
                "set `EnableProxyLibrary=1` and `ProxyLibrary=<file>` under `[PROXY]` in its \
                 `ReShade.ini` and it chain-loads that file",
            ),
            Self::Enb => Some(
                "set `EnableProxyLibrary=true` and `ProxyLibrary=<file>` under `[PROXY]` in \
                 its `enblocal.ini`, which takes exactly one library",
            ),
            Self::SpecialK => Some(
                "add an `[Import.<name>]` section with `Filename=` and `When=Lazy` to its ini \
                 and it loads that module itself",
            ),
            Self::Doorstop => Some(
                "it loads a displaced original renamed to `<name>_alt.dll`, which is in its \
                 source but in none of its documentation",
            ),
            Self::Dxvk | Self::D3d8to9 | Self::Re4Tweaks | Self::Taarib => None,
        }
    }

    /// The same, in Arabic.
    #[must_use]
    pub const fn tasalsul_arabi(self) -> Option<&'static str> {
        match self {
            Self::MuhammilAsi => {
                Some("وهو يحمّل كل ملف asi. بجانبه، فالإضافة لا تحتاج اسمًا خاصًّا بها.")
            }
            Self::ReShade => Some(
                "واضبط EnableProxyLibrary=1 وProxyLibrary=<ملف> في قسم [PROXY] من ReShade.ini يحمّله بعده.",
            ),
            Self::Enb => Some(
                "واضبط EnableProxyLibrary=true وProxyLibrary=<ملف> في قسم [PROXY] من enblocal.ini، وهو يقبل مكتبة واحدة لا أكثر.",
            ),
            Self::SpecialK => Some(
                "وأضف قسم [Import.<اسم>] فيه Filename= وWhen=Lazy إلى إعداداته يحمّل الوحدة بنفسه.",
            ),
            Self::Doorstop => Some(
                "وهو يحمّل الأصل المُزاح إن سُمّي <الاسم>_alt.dll، وهذا في مصدره لا في توثيقه.",
            ),
            Self::Dxvk | Self::D3d8to9 | Self::Re4Tweaks | Self::Taarib => None,
        }
    }
}

/// A file or directory whose presence beside a proxy is evidence of a product.
#[derive(Debug, Clone, Copy)]
enum RafiqWakeel {
    /// An exact name, matched case-insensitively, file or directory.
    Ism(&'static str),
    /// Any entry carrying this extension — `.asi` for a plugin loader.
    Imtidad(&'static str),
}

/// One product's signature: where it installs, what it leaves beside itself,
/// and what it says inside its own module.
struct TawqeeWakeel {
    aila: AilatWakeel,
    /// The slots this product is ever installed as, folded as
    /// [`WUKALA_NIZAM`] spells them.
    ///
    /// This is a guard and not a hint. `re4_tweaks` contains the string `dxvk`
    /// because it *reports* whether DXVK is in use, so a bare mark search would
    /// call Resident Evil 4's `dinput8.dll` a Direct3D translation layer.
    /// DXVK is never installed as `dinput8`, so the guard answers it.
    slotat: &'static [&'static str],
    rifaq: &'static [RafiqWakeel],
    /// Byte strings that appear in this product's own module.
    ///
    /// Matched case-insensitively in ASCII and in the UTF-16 a Windows version
    /// resource stores its strings as. Every one of these was chosen because it
    /// was **observed** in a real installation or because it is the product's
    /// own name as its version resource carries it — never because it seemed
    /// likely.
    basmat: &'static [&'static str],
}

/// Every product this module can name, and what proves each one.
static TAWAQEE: &[TawqeeWakeel] = &[
    TawqeeWakeel {
        aila: AilatWakeel::MuhammilAsi,
        // Ultimate ASI Loader ships under all of these; Alexander Blade's
        // loader is the `dinput8`/`xinput` case. `version` is on the list,
        // which is what makes this the family most likely to hold Taarib's own
        // slot.
        slotat: &[
            "binkw32",
            "bink2w64",
            "d3d8",
            "d3d9",
            "d3d11",
            "ddraw",
            "dinput8",
            "dsound",
            "dxgi",
            "msacm32",
            "version",
            "vorbisfile",
            "vorbishooked",
            "winhttp",
            "wininet",
            "winmm",
            "xinput1_3",
            "xinput1_4",
            "xlive",
        ],
        rifaq: &[
            RafiqWakeel::Imtidad("asi"),
            RafiqWakeel::Ism("asiloader.log"),
            RafiqWakeel::Ism("ScriptHookV.dll"),
            RafiqWakeel::Ism("ScriptHookV.log"),
            RafiqWakeel::Ism("scripts"),
            RafiqWakeel::Ism("plugins"),
            RafiqWakeel::Ism("update"),
            RafiqWakeel::Ism("packages"),
            RafiqWakeel::Ism("global.ini"),
        ],
        // `asiloader` and `Asi loader` are what Alexander Blade's build
        // actually contains — observed, in both ASCII and UTF-16, in Grand
        // Theft Auto V's `dinput8.dll` whose version resource reads
        // `GTA V *.asi plugin loader`. The capitalisation is his, not the
        // ecosystem's, which is why matching here is case-insensitive.
        basmat: &[
            "Ultimate ASI Loader",
            "ThirteenAG",
            "asiloader",
            "Asi loader",
            "Alexander Blade",
        ],
    },
    TawqeeWakeel {
        aila: AilatWakeel::ReShade,
        slotat: &[
            "d3d9", "d3d10", "d3d11", "d3d12", "ddraw", "dinput8", "dxgi", "opengl32",
        ],
        rifaq: &[
            RafiqWakeel::Ism("ReShade.ini"),
            RafiqWakeel::Ism("ReShade.log"),
            RafiqWakeel::Ism("ReShadePreset.ini"),
            RafiqWakeel::Ism("ReShade64.dll"),
            RafiqWakeel::Ism("ReShade32.dll"),
            RafiqWakeel::Ism("reshade-shaders"),
        ],
        // `ReShade32.dll`/`ReShade64.dll` is what its version resource carries
        // as `OriginalFilename` whatever name it was installed under, which is
        // the one string that survives a rename.
        basmat: &["ReShade", "reshade-shaders", "EffectSearchPaths", "ReShade64.dll"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::Enb,
        slotat: &["d3d8", "d3d9", "d3d11", "d3d12", "ddraw", "dxgi"],
        rifaq: &[
            RafiqWakeel::Ism("enbseries.ini"),
            RafiqWakeel::Ism("enblocal.ini"),
            RafiqWakeel::Ism("enbhost.exe"),
            RafiqWakeel::Ism("enbseries"),
            RafiqWakeel::Ism("enbcache"),
            // The DirectX 11 wrapper ships a patched shader compiler under this
            // name. `46e` is not a Microsoft version — that suffix exists
            // nowhere else and is the single most specific ENBSeries artifact
            // on disk.
            RafiqWakeel::Ism("d3dcompiler_46e.dll"),
        ],
        basmat: &["ENBSeries", "enbseries.ini", "enblocal.ini", "Boris Vorontsov"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::SpecialK,
        slotat: &["d3d8", "d3d9", "d3d11", "ddraw", "dinput8", "dxgi", "opengl32"],
        rifaq: &[
            RafiqWakeel::Ism("SpecialK.ini"),
            RafiqWakeel::Ism("SpecialK64.dll"),
            RafiqWakeel::Ism("SpecialK32.dll"),
            RafiqWakeel::Ism("SpecialK"),
            // A marker file it writes to claim a directory, and a name nothing
            // else on a disk has any reason to use.
            RafiqWakeel::Ism("SpecialK.central"),
        ],
        // `SK_` is deliberately not here. It was tried against this library and
        // hit `Menyoo.asi`, a GTA V trainer with no connection to Special K, as
        // a substring of an unrelated symbol. A three-character mark is not
        // evidence.
        basmat: &["SpecialK", "Special K", "Kaldaien"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::Doorstop,
        // `dxgi` is Doorstop 4's third proxy. It exports `CreateDXGIFactory`
        // and friends as pure forwarding stubs and hooks nothing graphical, so
        // a Doorstop in that slot is a mod loader wearing a graphics name —
        // which is exactly why naming the product matters more than the slot.
        slotat: &["winhttp", "version", "dxgi"],
        rifaq: &[
            RafiqWakeel::Ism("doorstop_config.ini"),
            RafiqWakeel::Ism(".doorstop_version"),
            RafiqWakeel::Ism("doorstop_libs"),
            RafiqWakeel::Ism("BepInEx"),
            RafiqWakeel::Ism("changelog.txt"),
            RafiqWakeel::Ism("run_bepinex.sh"),
        ],
        basmat: &["doorstop", "BepInEx", "NeighTools"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::Dxvk,
        slotat: &["d3d8", "d3d9", "d3d10", "d3d11", "dxgi"],
        rifaq: &[RafiqWakeel::Ism("dxvk.conf"), RafiqWakeel::Imtidad("dxvk-cache")],
        // `DXVK` is its `CompanyName` and the copyright line is its
        // `LegalCopyright`; both are in the version resource of all five
        // modules it ships.
        basmat: &["DxvkInstance", "zlib/libpng license", "DXVK"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::D3d8to9,
        slotat: &["d3d8"],
        // Its log carries no executable name, which is what separates it from
        // DXVK's `<exe>_d3d8.log` in the same slot.
        rifaq: &[RafiqWakeel::Ism("d3d8.log")],
        // Its version resource says `ProductName = ReShade`, inherited from
        // ReShade's template and frozen at 3.0.0 — which is why `ReShade` is
        // not among the marks admitted for the `d3d8` slot and this one is.
        basmat: &["d3d8to9"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::Re4Tweaks,
        slotat: &["dinput8", "winmm"],
        rifaq: &[RafiqWakeel::Ism("re4_tweaks")],
        basmat: &["re4_tweaks"],
    },
    TawqeeWakeel {
        aila: AilatWakeel::Taarib,
        slotat: &["version"],
        // The name `taarib-mudkhal` writes its own log under, compiled into it
        // as an ASCII literal. `taarib_tabaqa::istitlaa` asks the same question
        // of the same string from inside a game process; the two are separate
        // because a crate loaded into somebody's address space must not depend
        // on the installer, and they must not disagree.
        rifaq: &[RafiqWakeel::Ism("mudkhal.sijill"), RafiqWakeel::Ism("taarib.sijill")],
        basmat: &["mudkhal.sijill"],
    },
];

/// Section names a packer leaves in a module it compressed.
///
/// Resident Evil 4's `winmm.dll` is packed with UPX, so its strings — including
/// its version resource — are compressed and no mark search can reach them.
/// That module is reported unidentified, which is correct, and saying *why* is
/// the difference between a survey that failed and a survey that was defeated.
const ALAMAT_HAZM: [&str; 2] = ["UPX0", "UPX1"];

/// One loader slot beside a game's executable, and the file holding it.
///
/// Serialisable because this is the one thing in the plan the user has to be
/// shown *before* they agree to an install, which means it crosses into the
/// Studio's command layer; a hand-written projection of four plain fields would
/// be a second place for the same four fields to drift.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct WakeelQaim {
    /// The file's name exactly as it is on disk, which is not necessarily the
    /// spelling [`WUKALA_NIZAM`] uses: Windows filesystems are case-insensitive
    /// and a mod's installer writes whatever case it likes.
    pub ism: String,

    /// The slot, folded to the spelling in [`WUKALA_NIZAM`].
    pub wakeel: String,

    /// The file's size in bytes.
    pub hajm: u64,

    /// Files beside it sharing its stem — `dinput8.ini`, `dinput8.log` — sorted
    /// by name.
    pub rifaq: Vec<String>,

    /// Which product this is, when the evidence names one.
    ///
    /// Last field on purpose: [`masah`] sorts these and the sort must stay
    /// ordered by slot name, not by whatever the identification happened to
    /// return.
    pub huwiya: HuwiyatWakeel,
}

/// What proved a product's identity.
///
/// Carried rather than collapsed into a boolean because a user who is told
/// "ReShade is installed here" is entitled to ask how that was known, and
/// because a wrong answer is only debuggable if the evidence travels with it.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case", tag = "naw", content = "qeema")]
pub enum DalilWakeel {
    /// A byte string found inside the module itself.
    Basma(String),
    /// A file or directory beside it that only this product creates.
    Rafiq(String),
}

impl DalilWakeel {
    /// The evidence as one phrase, for a report.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::Basma(nass) => format!("the string \"{nass}\" inside it"),
            Self::Rafiq(ism) => format!("{ism} beside it"),
        }
    }

    /// The same phrase in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Basma(nass) => format!("النص \"{nass}\" داخله"),
            Self::Rafiq(ism) => format!("{ism} بجانبه"),
        }
    }
}

/// Whose loader a slot holds.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case", tag = "hala")]
pub enum HuwiyatWakeel {
    /// One product, and what proved it.
    Maaruf {
        /// The product.
        aila: AilatWakeel,
        /// Every piece of evidence found for it, marks before companions.
        adilla: Vec<DalilWakeel>,
    },

    /// More than one product fits the evidence equally well.
    ///
    /// Reported rather than resolved. Special K ships ReShade integration and
    /// therefore contains the string `ReShade`, so a tie between those two is a
    /// real outcome and not a gap in the table — and a survey that broke the tie
    /// by picking one would be a survey that guesses.
    Multabis {
        /// The products that tied, sorted.
        murashshahun: Vec<AilatWakeel>,
    },

    /// Nothing named it.
    Majhul {
        /// Whether the module is packed, which is why nothing could be read out
        /// of it. `false` means the module was readable and simply said nothing
        /// this module recognises.
        mahzum: bool,
    },
}

impl HuwiyatWakeel {
    /// The product, when exactly one was named.
    #[must_use]
    pub const fn aila(&self) -> Option<AilatWakeel> {
        match self {
            Self::Maaruf { aila, .. } => Some(*aila),
            Self::Multabis { .. } | Self::Majhul { .. } => None,
        }
    }

    /// Whether this slot holds Taarib's own loader.
    #[must_use]
    pub const fn huwa_taarib(&self) -> bool {
        matches!(self.aila(), Some(AilatWakeel::Taarib))
    }

    /// The identity as one clause, appended to a slot's description.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::Maaruf { aila, adilla } => {
                let kayf: Vec<String> =
                    adilla.iter().map(DalilWakeel::wasf_injilizi).collect();
                if kayf.is_empty() {
                    aila.ism().to_owned()
                } else {
                    format!("{}, from {}", aila.ism(), kayf.join(" and "))
                }
            }
            Self::Multabis { murashshahun } => format!(
                "either {} — the evidence fits both equally and Taarib will not guess",
                murashshahun
                    .iter()
                    .map(|aila| aila.ism().to_owned())
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
            Self::Majhul { mahzum: true } => {
                "an unidentified proxy, packed, so nothing can be read out of it".to_owned()
            }
            Self::Majhul { mahzum: false } => "an unidentified proxy".to_owned(),
        }
    }

    /// The same clause in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Maaruf { aila, adilla } => {
                let kayf: Vec<String> = adilla.iter().map(DalilWakeel::wasf_arabi).collect();
                if kayf.is_empty() {
                    aila.ism_arabi().to_owned()
                } else {
                    format!("{}، عُرف من {}", aila.ism_arabi(), kayf.join(" و"))
                }
            }
            Self::Multabis { murashshahun } => format!(
                "إمّا {} — والدليل يحتملهما معًا، ولا يخمّن تعريب",
                murashshahun
                    .iter()
                    .map(|aila| aila.ism_arabi().to_owned())
                    .collect::<Vec<_>>()
                    .join(" أو ")
            ),
            Self::Majhul { mahzum: true } => {
                "وكيل غير معروف، وهو مضغوط فلا يمكن قراءة شيء منه".to_owned()
            }
            Self::Majhul { mahzum: false } => "وكيل غير معروف".to_owned(),
        }
    }
}

impl WakeelQaim {
    /// The entry as one line of English, for the install report and the
    /// confirmation screen.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let hajm = self.hajm;
        let huwiya = self.huwiya.wasf_injilizi();
        if self.rifaq.is_empty() {
            format!("{} ({hajm} byte(s)) — {huwiya}", self.ism)
        } else {
            format!(
                "{} ({hajm} byte(s), with {}) — {huwiya}",
                self.ism,
                self.rifaq.join(", ")
            )
        }
    }

    /// The same line in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let hajm = self.hajm;
        let huwiya = self.huwiya.wasf_arabi();
        if self.rifaq.is_empty() {
            format!("{} ({hajm} بايت) — {huwiya}", self.ism)
        } else {
            format!("{} ({hajm} بايت، ومعه {}) — {huwiya}", self.ism, self.rifaq.join("، "))
        }
    }
}

/// Whether a file name occupies one of the slots in [`WUKALA_NIZAM`].
///
/// Case-folded on both halves, because the name on disk is whatever a mod's
/// installer wrote and Windows treats `DInput8.DLL` and `dinput8.dll` as one
/// file.
#[must_use]
pub fn wakeel_nizam(ism: &str) -> Option<&'static str> {
    let (asas, imtidad) = ism.rsplit_once('.')?;
    if !imtidad.eq_ignore_ascii_case("dll") {
        return None;
    }
    WUKALA_NIZAM.into_iter().find(|maaruf| asas.eq_ignore_ascii_case(maaruf))
}

/// Whether a file name occupies one of the slots in [`WUKALA_MUSHTARAKA`].
///
/// A hit here is **not** evidence of a mod on its own — see that list — and the
/// survey reports one only once [`tashkhis`] has named the product.
#[must_use]
pub fn wakeel_mushtarak(ism: &str) -> Option<&'static str> {
    let (asas, imtidad) = ism.rsplit_once('.')?;
    if !imtidad.eq_ignore_ascii_case("dll") {
        return None;
    }
    WUKALA_MUSHTARAKA.into_iter().find(|maaruf| asas.eq_ignore_ascii_case(maaruf))
}

/// Every loader slot in use in one directory, sorted by slot name.
///
/// A directory that does not exist holds no slots and is reported as such
/// rather than as a failure: this runs while a plan is being built, and a
/// missing directory is answered by the step that needed it to exist, not by
/// the survey that walked past it.
///
/// # Errors
///
/// Whatever reading the directory raises, apart from
/// [`std::io::ErrorKind::NotFound`]. An entry whose metadata cannot be read is
/// skipped rather than failing the survey — it is a file the installer will not
/// be writing over, and refusing to plan an install because one unrelated file
/// in the game directory could not be stat'ed would be a refusal with no
/// subject.
pub fn masah(mujallad: &Path) -> std::io::Result<Vec<WakeelQaim>> {
    let qira = match std::fs::read_dir(mujallad) {
        Ok(qira) => qira,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(sabab) => return Err(sabab),
    };

    let mut asmaa: Vec<String> = Vec::new();
    let mut mujalladat: Vec<String> = Vec::new();
    for madkhal in qira {
        let madkhal = madkhal?;
        let Ok(naw) = madkhal.file_type() else { continue };
        let Some(ism) = madkhal.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        // A directory called `version.dll` is not a module the loader can map,
        // so it holds no slot — but a directory called `reshade-shaders` is the
        // single best piece of evidence there is about the module beside it, so
        // both kinds are listed and only files are treated as slots.
        if naw.is_file() {
            asmaa.push(ism);
        } else if naw.is_dir() {
            mujalladat.push(ism);
        }
    }

    let mut qaima: Vec<WakeelQaim> = Vec::new();
    for ism in &asmaa {
        let (wakeel, nizami) = match wakeel_nizam(ism) {
            Some(wakeel) => (wakeel, true),
            None => match wakeel_mushtarak(ism) {
                Some(wakeel) => (wakeel, false),
                None => continue,
            },
        };
        let masar = mujallad.join(ism);
        let hajm = std::fs::metadata(&masar).map_or(0, |bayan| bayan.len());
        let huwiya = tashkhis(&masar, wakeel, ism, &asmaa, &mujalladat, nizami);
        // A shared name that nothing identified is Microsoft's own file, and
        // reporting it would be the survey crying proxy over a redistributable
        // — the exact failure `WUKALA_MUSHTARAKA` exists to avoid.
        if !nizami && huwiya.aila().is_none() {
            continue;
        }
        qaima.push(WakeelQaim {
            ism: ism.clone(),
            wakeel: wakeel.to_owned(),
            hajm,
            rifaq: rifaq_bi_asas(&asmaa, wakeel),
            huwiya,
        });
    }
    qaima.sort();
    Ok(qaima)
}

/// Names the product holding one slot, from marks inside it and companions
/// beside it.
///
/// Scoring, and it is the whole of the judgement this module makes: a product
/// scores two for any mark found inside the module and one for any companion
/// found beside it. The highest unique score wins, a tie at the top is
/// [`HuwiyatWakeel::Multabis`], and a top score of zero is
/// [`HuwiyatWakeel::Majhul`].
///
/// A mark outweighs a companion because a companion survives its product's
/// removal — a `ReShade.ini` left behind after ReShade was deleted would
/// otherwise rename whatever proxy took the slot next — and a mark cannot be
/// left behind by anything but the module it is in. Both together outweigh
/// either, which is what settles a module that mentions a competitor: Special K
/// carries the string `ReShade`, and `SpecialK.ini` beside it is the extra point
/// that decides.
///
/// Only products whose [`TawqeeWakeel::slotat`] contains this slot are
/// considered at all. That guard is doing real work rather than tidying: the
/// `re4_tweaks` proxy in Resident Evil 4 contains both `DXVK` and `dxvk`,
/// because it detects and reports whether DXVK is in use, and DXVK is never
/// installed as `dinput8`.
///
/// ## Why a shared name needs a mark
///
/// `nizami` says whether the name is one of [`WUKALA_NIZAM`]'s, where the file
/// is a proxy whatever else is true, or one of [`WUKALA_MUSHTARAKA`]'s, where it
/// is probably the game's own. For the second kind **companions are not
/// admitted as evidence at all** and only a mark inside the module can name it.
///
/// That rule was written against a failure, not in the abstract. Companions are
/// evidence about a *directory*, not about a file: put one `.asi` plugin beside
/// a game and every module in that directory scores a point for being an ASI
/// loader — including the `xinput1_3.dll` Microsoft's DirectX redistributable
/// put there, which would then be reported as a mod. For a name that is a proxy
/// by construction that is merely a sharper description of something already
/// being reported. For a name that is usually Microsoft's it is the survey
/// crying wolf, which is the one outcome [`WUKALA_MUSHTARAKA`] exists to
/// prevent.
#[must_use]
pub fn tashkhis(
    masar: &Path,
    wakeel: &str,
    ism: &str,
    asmaa: &[String],
    mujalladat: &[String],
    nizami: bool,
) -> HuwiyatWakeel {
    let murashshahun: Vec<&TawqeeWakeel> = TAWAQEE
        .iter()
        .filter(|tawqee| tawqee.slotat.iter().any(|slot| slot.eq_ignore_ascii_case(wakeel)))
        .collect();
    if murashshahun.is_empty() {
        return HuwiyatWakeel::Majhul { mahzum: false };
    }

    let mut matlub: Vec<&'static str> = Vec::new();
    for tawqee in &murashshahun {
        for basma in tawqee.basmat {
            if !matlub.contains(basma) {
                matlub.push(basma);
            }
        }
    }
    for alama in ALAMAT_HAZM {
        matlub.push(alama);
    }
    let wujidat = basmat_fi_malaf(masar, &matlub);
    let mahzum = ALAMAT_HAZM.iter().any(|alama| wujidat.contains(alama));

    let mut natija: Vec<(AilatWakeel, u8, Vec<DalilWakeel>)> = Vec::new();
    for tawqee in &murashshahun {
        let mut adilla: Vec<DalilWakeel> = Vec::new();
        let mut darajat = 0_u8;
        for basma in tawqee.basmat {
            if wujidat.contains(basma) {
                adilla.push(DalilWakeel::Basma((*basma).to_owned()));
                darajat = 2;
            }
        }
        if !nizami && darajat == 0 {
            continue;
        }
        let mut rifaq = rifaq_mawjuda(tawqee.rifaq, ism, asmaa, mujalladat);
        if !rifaq.is_empty() {
            darajat = darajat.saturating_add(1);
            adilla.append(&mut rifaq);
        }
        if darajat > 0 {
            natija.push((tawqee.aila, darajat, adilla));
        }
    }

    let Some(aala) = natija.iter().map(|(_, darajat, _)| *darajat).max() else {
        return HuwiyatWakeel::Majhul { mahzum };
    };
    let mut fayizun: Vec<(AilatWakeel, Vec<DalilWakeel>)> = natija
        .into_iter()
        .filter(|(_, darajat, _)| *darajat == aala)
        .map(|(aila, _, adilla)| (aila, adilla))
        .collect();
    if fayizun.len() == 1
        && let Some((aila, adilla)) = fayizun.pop()
    {
        return HuwiyatWakeel::Maaruf { aila, adilla };
    }
    let mut murashshahun: Vec<AilatWakeel> =
        fayizun.into_iter().map(|(aila, _)| aila).collect();
    murashshahun.sort_unstable();
    HuwiyatWakeel::Multabis { murashshahun }
}

/// The entry in a survey holding one slot, if any.
///
/// ## Why there is no function beside this that picks a different slot
///
/// The obvious feature is missing on purpose. When `version.dll` is taken —
/// which an ASI loader takes more often than anything else, because `version`
/// is one of the names Ultimate ASI Loader ships under — Taarib could look for
/// a free name in [`WUKALA_NIZAM`] and deploy there instead. It does not, and
/// the reason is not caution:
///
/// **A proxy in a name the game never loads is an install that reports success
/// and does nothing.** The Windows loader resolves a module from beside the
/// executable only when something asks for that module. `version.dll` works
/// because the game, or the runtime beside it, imports `GetFileVersionInfoW`;
/// `dciman32.dll` sitting free in the same directory is free precisely because
/// nothing in that process will ever ask for it. Moving there would produce a
/// deployed payload, a written manifest, a green install report and a game with
/// no Arabic in it — the exact shape of the defect this crate already carries a
/// comment about, where a foreign proxy was read as "already installed" and the
/// install silently deployed nothing.
///
/// Whether an alternative name is loaded by *this* game is knowable, but only
/// from the executable's import table, which lives in `taarib-tabaqa` behind a
/// crate boundary this one must not cross — an installer that pulled in a
/// module loaded into game processes to read a PE header would be the wrong
/// dependency in the wrong direction.
///
/// So the answer is a refusal, and the refusal is made useful instead by
/// [`AilatWakeel::tasalsul`]: three of the four products that most often hold
/// `version.dll` can be told to load a second module themselves, and an ASI
/// loader needs no slot from Taarib at all because it loads every `.asi` beside
/// it. Naming the product is what makes that sentence writable.
#[must_use]
pub fn shaghil_slot<'a>(qaima: &'a [WakeelQaim], wakeel: &str) -> Option<&'a WakeelQaim> {
    qaima.iter().find(|qaim| qaim.ism.eq_ignore_ascii_case(wakeel))
}

/// The companions of one product that are actually in this directory.
///
/// The occupying file is excluded from its own evidence. Doorstop's companion
/// list names `winhttp.dll` because that file beside a `BepInEx/` is what
/// Doorstop looks like from outside — but a `winhttp.dll` proving *itself* is
/// not evidence of anything, and letting it would score every slot one point
/// for existing.
fn rifaq_mawjuda(
    matlub: &[RafiqWakeel],
    ism: &str,
    asmaa: &[String],
    mujalladat: &[String],
) -> Vec<DalilWakeel> {
    let mut wujidat: Vec<DalilWakeel> = Vec::new();
    for rafiq in matlub {
        match rafiq {
            RafiqWakeel::Ism(matlub) => {
                let mawjud = asmaa
                    .iter()
                    .chain(mujalladat.iter())
                    .find(|mawjud| {
                        mawjud.eq_ignore_ascii_case(matlub)
                            && !mawjud.eq_ignore_ascii_case(ism)
                    })
                    .cloned();
                if let Some(mawjud) = mawjud {
                    wujidat.push(DalilWakeel::Rafiq(mawjud));
                }
            }
            RafiqWakeel::Imtidad(matlub) => {
                let mawjud = asmaa
                    .iter()
                    .find(|mawjud| {
                        mawjud.rsplit_once('.').is_some_and(|(_, imtidad)| {
                            imtidad.eq_ignore_ascii_case(matlub)
                        }) && !mawjud.eq_ignore_ascii_case(ism)
                    })
                    .cloned();
                if let Some(mawjud) = mawjud {
                    wujidat.push(DalilWakeel::Rafiq(mawjud));
                }
            }
        }
    }
    wujidat
}

/// The configuration and log files a proxy named after its slot, sorted.
fn rifaq_bi_asas(asmaa: &[String], wakeel: &str) -> Vec<String> {
    let mut rifaq: Vec<String> = asmaa
        .iter()
        .filter(|ism| {
            ism.rsplit_once('.').is_some_and(|(asas, imtidad)| {
                asas.eq_ignore_ascii_case(wakeel)
                    && LAWAHIQ_RAFIQ.iter().any(|maqbul| imtidad.eq_ignore_ascii_case(maqbul))
            })
        })
        .cloned()
        .collect();
    rifaq.sort();
    rifaq
}

/// How much of a module is read at a time.
const HAJM_QITA: usize = 256 * 1024;

/// The largest module this module will scan for marks.
///
/// Sixty-four mebibytes is four times the largest proxy observed in a real
/// library — `re4_tweaks` at eleven and a half — and the cap exists because this
/// runs over a directory the user chose. Past it the module is left
/// unidentified rather than read, which costs a name and never a hang.
const AQSA_MASH: u64 = 64 * 1024 * 1024;

/// Which of the given marks appear in a module, in ASCII or in UTF-16.
///
/// Read in bounded chunks with an overlap the length of the longest mark, so a
/// mark that straddles a chunk boundary is still found and the memory this
/// holds does not grow with the file. A module that cannot be opened, or is past
/// [`AQSA_MASH`], yields nothing — which reads as "unidentified", the safe
/// direction, because the decision resting on this is whether to *name* a mod
/// and never whether to overwrite one.
fn basmat_fi_malaf(masar: &Path, basmat: &[&'static str]) -> Vec<&'static str> {
    let mut wujidat: Vec<&'static str> = Vec::new();
    if basmat.is_empty() {
        return wujidat;
    }
    let Ok(bayan) = std::fs::metadata(masar) else { return wujidat };
    if !bayan.is_file() || bayan.len() > AQSA_MASH {
        return wujidat;
    }
    let Ok(malaf) = std::fs::File::open(masar) else { return wujidat };

    // Twice the longest mark, because the UTF-16 form of a mark is twice as
    // long as the ASCII one and either may straddle the boundary.
    let tadakhul = basmat.iter().map(|basma| basma.len()).max().unwrap_or(0).saturating_mul(2);
    let mut qari = std::io::BufReader::new(malaf);
    let mut mukhazzan: Vec<u8> = Vec::with_capacity(HAJM_QITA.saturating_add(tadakhul));
    let mut qita = vec![0_u8; HAJM_QITA];

    loop {
        let tul = match qari.read(&mut qita) {
            Ok(0) | Err(_) => break,
            Ok(tul) => tul,
        };
        mukhazzan.extend_from_slice(qita.get(..tul).unwrap_or(&[]));
        for basma in basmat {
            if !wujidat.contains(basma) && fihi_basma(&mukhazzan, basma.as_bytes()) {
                wujidat.push(basma);
            }
        }
        if wujidat.len() == basmat.len() {
            break;
        }
        let zayid = mukhazzan.len().saturating_sub(tadakhul);
        if zayid > 0 {
            mukhazzan.drain(..zayid);
        }
    }
    wujidat
}

/// Whether a mark appears in a buffer, in either encoding a module stores one.
///
/// Case-insensitive on both, and that is a correction from evidence rather than
/// caution: the mark that identifies Alexander Blade's ASI loader is spelled
/// `asiloader` and `Asi loader` in the binary, and a case-sensitive search for
/// the ecosystem's own `ASI Loader` finds neither.
fn fihi_basma(kawm: &[u8], basma: &[u8]) -> bool {
    yahwi_ascii(kawm, basma) || yahwi_utf16(kawm, basma)
}

/// Whether a buffer contains a byte string, ignoring ASCII case.
fn yahwi_ascii(kawm: &[u8], basma: &[u8]) -> bool {
    if basma.is_empty() || kawm.len() < basma.len() {
        return false;
    }
    kawm.windows(basma.len()).any(|nafidha| nafidha.eq_ignore_ascii_case(basma))
}

/// The same, for the UTF-16 a Windows version resource stores its strings in.
///
/// Every second byte must be zero, which is what a Latin-script string looks
/// like in UTF-16LE and what makes this cheap to reject.
fn yahwi_utf16(kawm: &[u8], basma: &[u8]) -> bool {
    let tul = basma.len().saturating_mul(2);
    if tul == 0 || kawm.len() < tul {
        return false;
    }
    kawm.windows(tul).any(|nafidha| {
        basma.iter().enumerate().all(|(fahras, harf)| {
            let mawdi = fahras.saturating_mul(2);
            nafidha.get(mawdi).is_some_and(|bayt| bayt.eq_ignore_ascii_case(harf))
                && nafidha.get(mawdi.saturating_add(1)) == Some(&0)
        })
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              honouring it here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use std::fs;
    use std::path::Path;

    use super::{AilatWakeel, HuwiyatWakeel, WakeelQaim, masah, wakeel_nizam};

    /// Writes a module carrying a product's marks the way a real one does.
    ///
    /// Marks go in as UTF-16 because that is where a real one keeps them: a
    /// Windows version resource stores `CompanyName` and `ProductName` in
    /// UTF-16, and the strings that actually identified Grand Theft Auto V's
    /// ASI loader and Resident Evil 4's `re4_tweaks` in a real library were
    /// found in exactly that encoding. An ASCII fixture would pass while
    /// leaving the encoding that matters unproven.
    fn wahda(jidhr: &Path, ism: &str, basmat: &[&str]) {
        let mut jism: Vec<u8> = b"MZ\x90\x00".to_vec();
        for basma in basmat {
            jism.extend(basma.encode_utf16().flat_map(u16::to_le_bytes));
            jism.push(0);
        }
        jism.resize(jism.len().max(1024), 0x90);
        fs::write(jidhr.join(ism), jism).expect("a fixture module");
    }

    /// The survey entry for one name, or a panic naming what was found instead.
    fn wajid(qaima: &[WakeelQaim], ism: &str) -> WakeelQaim {
        qaima
            .iter()
            .find(|qaim| qaim.ism == ism)
            .cloned()
            .unwrap_or_else(|| panic!("{ism} was not surveyed; found {qaima:?}"))
    }

    /// The product a slot was identified as, or `None`.
    fn aila(qaima: &[WakeelQaim], ism: &str) -> Option<AilatWakeel> {
        wajid(qaima, ism).huwiya.aila()
    }

    #[test]
    fn reshade_yusamma_min_basmatih_wa_rifaqih() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "dxgi.dll", &["ReShade", "reshade-shaders"]);
        fs::write(muaqqat.path().join("ReShade.ini"), b"[GENERAL]").expect("its settings");
        fs::create_dir(muaqqat.path().join("reshade-shaders")).expect("its shader tree");

        let qaima = masah(muaqqat.path()).expect("the survey");
        assert_eq!(aila(&qaima, "dxgi.dll"), Some(AilatWakeel::ReShade));
        let satr = wajid(&qaima, "dxgi.dll").wasf_injilizi();
        assert!(satr.contains("ReShade"), "the report names the product: {satr}");
        assert!(satr.contains("the string"), "and says what proved it: {satr}");
    }

    #[test]
    fn enb_yusamma_min_ini_wahdah() {
        // No mark inside the module at all — ENBSeries builds do not always
        // carry a version resource — so this leans on the companion alone,
        // which is the weaker evidence and must still be enough when nothing
        // competes with it.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "d3d9.dll", &[]);
        fs::write(muaqqat.path().join("enbseries.ini"), b"[PROXY]").expect("its settings");
        fs::write(muaqqat.path().join("enblocal.ini"), b"[LIMITER]").expect("more settings");

        let qaima = masah(muaqqat.path()).expect("the survey");
        assert_eq!(aila(&qaima, "d3d9.dll"), Some(AilatWakeel::Enb));
    }

    #[test]
    fn basma_taghlib_rafiqan_matruka() {
        // Special K's module really does contain the string `ReShade`, because
        // it ships ReShade integration. A `SpecialK.ini` beside it is the extra
        // point that decides, and this is the case the scoring exists for.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "dxgi.dll", &["ReShade", "SpecialK"]);
        fs::write(muaqqat.path().join("SpecialK.ini"), b"[SpecialK.System]")
            .expect("its settings");

        let qaima = masah(muaqqat.path()).expect("the survey");
        assert_eq!(
            aila(&qaima, "dxgi.dll"),
            Some(AilatWakeel::SpecialK),
            "a product that names itself and leaves its own file beats one merely mentioned"
        );
    }

    #[test]
    fn tasawi_aladilla_yubligh_wala_yukhammin() {
        // Two products' settings, nothing inside the module to separate them.
        // The honest answer is both names, and a survey that picked one would
        // be a survey that guesses.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "d3d9.dll", &[]);
        fs::write(muaqqat.path().join("ReShade.ini"), b"[GENERAL]").expect("one product's");
        fs::write(muaqqat.path().join("enbseries.ini"), b"[PROXY]").expect("another's");

        let qaima = masah(muaqqat.path()).expect("the survey");
        let huwiya = wajid(&qaima, "d3d9.dll").huwiya;
        assert_eq!(
            huwiya,
            HuwiyatWakeel::Multabis {
                murashshahun: vec![AilatWakeel::ReShade, AilatWakeel::Enb],
            },
            "an even tie is reported as one"
        );
        assert!(huwiya.wasf_injilizi().contains("will not guess"));
    }

    #[test]
    fn slot_yahjuz_alaila_falaa_tusamma_dxvk_fi_dinput8() {
        // Resident Evil 4's `re4_tweaks` proxy contains both `DXVK` and `dxvk`,
        // because it detects and reports whether DXVK is in use. DXVK is never
        // installed as `dinput8`, and the slot guard is what stops a real
        // string from producing a false product.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "dinput8.dll", &["DxvkInstance", "re4_tweaks"]);

        let qaima = masah(muaqqat.path()).expect("the survey");
        assert_eq!(aila(&qaima, "dinput8.dll"), Some(AilatWakeel::Re4Tweaks));
    }

    #[test]
    fn ism_mushtarak_la_yublagh_illa_muaarrafan() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        // Microsoft's own DirectX redistributable, which half a Steam library
        // carries, and Microsoft's own `dbghelp.dll`, which every one of four
        // real installations turned out to be.
        wahda(muaqqat.path(), "xinput1_3.dll", &["Microsoft Corporation"]);
        wahda(muaqqat.path(), "dbghelp.dll", &["Windows Image Helper"]);
        assert_eq!(
            masah(muaqqat.path()).expect("the survey"),
            Vec::new(),
            "a shared name nothing identified is the game's own file and is not reported"
        );

        // The same name, now holding Ultimate ASI Loader. The capitalisation is
        // Alexander Blade's own — his build spells it `asiloader`, not
        // `ASI Loader` — which is why matching ignores case.
        wahda(muaqqat.path(), "xinput1_4.dll", &["asiloader"]);
        fs::write(muaqqat.path().join("NativeTrainer.asi"), b"x").expect("a plugin");
        let qaima = masah(muaqqat.path()).expect("the survey");
        assert_eq!(
            qaima.iter().map(|qaim| qaim.ism.as_str()).collect::<Vec<_>>(),
            vec!["xinput1_4.dll"],
            "identified, the same class of name is a mod and is reported"
        );
        assert_eq!(aila(&qaima, "xinput1_4.dll"), Some(AilatWakeel::MuhammilAsi));
    }

    #[test]
    fn wahda_mahzuma_tublagh_bisabab_ajzaha() {
        // Resident Evil 4's `winmm.dll` is UPX-packed with an empty version
        // resource, so no mark search can reach anything. Reporting it as
        // unidentified is right; saying *why* is the difference between a
        // survey that failed and one that was defeated.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "winmm.dll", &["UPX0", "UPX1"]);

        let qaima = masah(muaqqat.path()).expect("the survey");
        assert_eq!(
            wajid(&qaima, "winmm.dll").huwiya,
            HuwiyatWakeel::Majhul { mahzum: true }
        );
        assert!(wajid(&qaima, "winmm.dll").wasf_injilizi().contains("packed"));
    }

    #[test]
    fn shakl_alnaql_thabit() {
        // The confirmation screen reads this over the command bridge, so the
        // spelling of every key and every variant is a contract with the
        // interface rather than an implementation detail. Asserted whole
        // because a rename that silently drops a field would show a screen with
        // a blank where the mod's name should be.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "dxgi.dll", &["ReShade"]);
        fs::create_dir(muaqqat.path().join("reshade-shaders")).expect("its shader tree");

        let qaima = masah(muaqqat.path()).expect("the survey");
        let qaim = wajid(&qaima, "dxgi.dll");
        assert_eq!(
            serde_json::to_value(&qaim).expect("the survey entry serialises"),
            serde_json::json!({
                "ism": "dxgi.dll",
                "wakeel": "dxgi",
                "hajm": 1024,
                "rifaq": [],
                "huwiya": {
                    "hala": "maaruf",
                    "aila": "reshade",
                    "adilla": [
                        { "naw": "basma", "qeema": "ReShade" },
                        { "naw": "rafiq", "qeema": "reshade-shaders" },
                    ],
                },
            })
        );
    }

    #[test]
    fn wakeel_majhul_yublagh_bilism_walhajm() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        wahda(muaqqat.path(), "dsound.dll", &["something nobody has heard of"]);

        let qaima = masah(muaqqat.path()).expect("the survey");
        let qaim = wajid(&qaima, "dsound.dll");
        assert_eq!(qaim.huwiya, HuwiyatWakeel::Majhul { mahzum: false });
        let satr = qaim.wasf_injilizi();
        assert!(satr.contains("dsound.dll") && satr.contains("1024 byte"), "{satr}");
        assert!(satr.contains("unidentified proxy"), "named as unidentified, not guessed: {satr}");
    }

    /// The layout of `Resident Evil 4`'s `Bin32/` on the machine this was
    /// written against: `re4_tweaks` holding two slots, the game's own files
    /// beside it, and Taarib's slot free.
    ///
    /// Names and sizes only — none of these is the real file, and nothing here
    /// reads one.
    fn bin32(jidhr: &Path) {
        for (ism, hajm) in [
            ("bio4.exe", 9_139_840_usize),
            ("dinput8.dll", 11_760_128),
            ("dinput8.ini", 22_044),
            ("dinput8.log", 42_726),
            ("steam_api.dll", 106_408),
            ("steam_appid.txt", 6),
            ("winmm.dll", 80_384),
        ] {
            fs::write(jidhr.join(ism), vec![0_u8; hajm.min(64)]).expect("writing a fixture");
        }
        fs::create_dir(jidhr.join("re4_tweaks")).expect("the mod's own directory");
        fs::write(jidhr.join("re4_tweaks/trainer.ini"), b"x").expect("a file inside it");
    }

    #[test]
    fn al_wakeel_yuaraf_bila_hala() {
        assert_eq!(wakeel_nizam("dinput8.dll"), Some("dinput8"));
        assert_eq!(wakeel_nizam("DInput8.DLL"), Some("dinput8"));
        assert_eq!(wakeel_nizam("version.dll"), Some("version"));
        // The redistributables a game ships legitimately. Reporting any of
        // these would make the survey noise.
        for barii in [
            "steam_api.dll",
            "d3dx9_43.dll",
            "xinput1_3.dll",
            "x3daudio1_7.dll",
            "msvcp140.dll",
            "vcruntime140.dll",
            "binkw32.dll",
        ] {
            assert_eq!(wakeel_nizam(barii), None, "{barii} is a game's own file, not a slot");
        }
        // Not a module at all.
        assert_eq!(wakeel_nizam("dinput8.ini"), None);
        assert_eq!(wakeel_nizam("dinput8"), None);
    }

    #[test]
    fn masah_yajid_huqn_re4_tweaks_wala_yalmis_malaffat_alluba() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        bin32(muaqqat.path());

        let qaima = masah(muaqqat.path()).expect("the survey reads a directory that exists");
        let asmaa: Vec<&str> = qaima.iter().map(|wakeel| wakeel.ism.as_str()).collect();
        assert_eq!(
            asmaa,
            vec!["dinput8.dll", "winmm.dll"],
            "re4_tweaks holds two slots; the game's own files hold none"
        );

        let dinput = qaima.first().expect("the first slot");
        assert_eq!(dinput.wakeel, "dinput8");
        assert_eq!(
            dinput.rifaq,
            vec!["dinput8.ini".to_owned(), "dinput8.log".to_owned()],
            "the proxy's settings and its log are named after the slot it stands in"
        );
        // The second proxy carries no settings of its own, and saying so is the
        // difference between a survey and a guess.
        let winmm = qaima.get(1).expect("the second slot");
        assert!(winmm.rifaq.is_empty());

        // Taarib's own slot is free in this game, which is the finding the
        // install decision rests on.
        assert!(
            !qaima.iter().any(|wakeel| wakeel.wakeel == "version"),
            "nothing in this directory holds the slot taarib-mudkhal is published as"
        );
    }

    #[test]
    fn wakeel_taarib_yuzhar_hin_yushghal() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        bin32(muaqqat.path());
        fs::write(muaqqat.path().join("version.dll"), b"another mod's loader")
            .expect("a foreign proxy in Taarib's slot");

        let qaima = masah(muaqqat.path()).expect("the survey");
        let taarib = qaima
            .iter()
            .find(|wakeel| wakeel.wakeel == "version")
            .expect("the occupied slot is reported");
        assert_eq!(taarib.hajm, 20);
        assert!(taarib.wasf_injilizi().contains("version.dll"));
        assert!(taarib.wasf_arabi().contains("بايت"));
    }

    #[test]
    fn mujallad_ghayr_mawjud_la_yahmil_shayan() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        let ghayib = muaqqat.path().join("la-yujad");
        assert_eq!(
            masah(&ghayib).expect("a directory that is not there holds no slots"),
            Vec::new(),
            "a missing directory is answered by whoever needed it, not by the survey"
        );
    }

    #[test]
    fn mujallad_bi_ism_wakeel_laysa_wahda() {
        // A *directory* called `dxgi.dll` is not something the loader can map,
        // and reporting it as an occupied slot would refuse an install over a
        // file that cannot hold one.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        fs::create_dir(muaqqat.path().join("dxgi.dll")).expect("a directory with a module's name");
        assert_eq!(masah(muaqqat.path()).expect("the survey"), Vec::new());
    }
}
