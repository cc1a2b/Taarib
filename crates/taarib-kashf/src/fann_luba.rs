//! فنّ اللعبة — the artwork already sitting inside the install, found by name
//! and measured without ever decoding it.
//!
//! Rung 3 of [`crate::silsila`] has two halves. [`crate::ayquna`] takes the
//! executable apart for its icon; this module reads what the *engine* shipped as
//! ordinary files. It is both cheaper and usually better artwork: a Ren'Py main
//! menu or an RPG Maker title screen is a real illustration painted for the
//! game, where an icon is 256 pixels of logo that will be blown up past
//! [`crate::silsila::Silsila::AQSA_TAKBIR`] and refused.
//!
//! ## Why nothing here decodes an image
//!
//! Every candidate's size comes from [`image::ImageReader::into_dimensions`],
//! which reads a PNG's `IHDR` or a JPEG's `SOF` marker and stops. Fully
//! decoding each candidate to find out how big it is would mean decompressing
//! hundreds of megabytes across a library — an RPG Maker game alone ships
//! dozens of 1280×720 title screens — in order to throw nearly all of it away
//! one comparison later. The header is between forty and a few hundred bytes,
//! and it answers the only question this module asks.
//!
//! Two formats have no header reader in `image` because this workspace builds
//! it with `png`, `jpeg` and `webp` only. `.ico` and `.bmp` are measured by
//! [`crate::ayquna::abaad_ico`] and [`crate::ayquna::abaad_bmp`], which read the
//! same few bytes and are in that module because that is where the decoders
//! that follow them live.
//!
//! ## What each engine ships, and where
//!
//! | engine | the files worth looking at |
//! | --- | --- |
//! | Unity | `<Game>_Data/splash*.png` |
//! | Unreal | `<Proj>/Content/Splash/Splash.bmp`, `EdSplash.bmp`, `<Proj>/Resources/*.ico`, `<Proj>/Binaries/Win64/*.ico` |
//! | Godot | `icon.png`, and whatever `project.godot` names for the icon and the boot splash |
//! | GameMaker | `icon.ico` beside the executable, `splash.png` |
//! | Ren'Py | `game/gui/window_icon.png`, `game/gui/main_menu.png`, `icon.ico` |
//! | RPG Maker | `Game.ico`, `www/icon/icon.png`, `icon/icon.png`, `img/titles1/*.png` |
//! | Electron / NW.js | `resources/app/icon.png`, `*.ico` beside the executable |
//! | anything | `cover.*`, `capsule.*`, `header.*`, `logo.png`, `boxart.*`, `folder.jpg` |
//!
//! ## Four things this deliberately does not open
//!
//! **Unity's `globalgamemanagers`.** The player-settings splash image lives in
//! it, and reading it means the serialized-file type tree, the class database
//! for the exact Unity version, and a `PPtr` chase into a `.assets` bundle —
//! which is Phase 6's asset reader, not a pre-pass that is supposed to cost a
//! few `stat` calls. `<Game>_Data/splash*.png`, when a project has one, is a
//! plain file and is taken.
//!
//! **Electron's `resources/app.asar`.** An asar is a real archive with a JSON
//! header and a concatenated payload, and opening it here would mean an archive
//! reader living in the discovery crate for the sake of one icon that
//! `resources/app/icon.png` and the executable's own `RT_GROUP_ICON` almost
//! always also carry.
//!
//! **SVG, anywhere.** Godot's default `icon.svg`, a `scalable/apps` theme entry,
//! a vector logo beside the executable. There is no rasterizer in this
//! workspace and writing one is a project. When an SVG is the only thing
//! present it is logged at debug with its path, so a diagnostics trail says
//! which file was passed over rather than reporting nothing at all.
//!
//! **Archives of every other kind** — `.pck`, `.pak`, `.rpa`, `.dat`, `.win`.
//! Same argument as the asar, multiplied by seven engines.
//!
//! ## Bounded, because an install is not a trusted directory
//!
//! A game directory can hold half a million files, can contain a symlink loop,
//! and can be on a network mount that answers slowly. The walk has a depth cap,
//! an entry cap, a per-directory cap, a candidate cap, and an exclusion list for
//! the handful of directory names that are known to hold six figures of assets
//! and no artwork. Symlinks are not followed.

use std::cmp::Reverse;
use std::ffi::OsStr;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use image::ImageReader;
use walkdir::WalkDir;

use crate::ayquna::{abaad_bmp, abaad_ico, iqra_maqati};
use crate::fahs::MasdarSura;
use crate::silsila::{MartabatSura, MurashahSura, NawSura};

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

/// How deep the walk descends below the install root.
///
/// Three. Every path in the table above is reachable within three levels of
/// *some* directory the walk sees, because the walk collects anchors rather
/// than leaf files: `Content` is found at depth one or two and
/// `Content/Splash/Splash.bmp` is then a direct probe from it. Going deeper
/// would multiply the entry count by the branching factor of an asset tree for
/// no path this module knows the name of.
const HADD_UMQ: usize = 3;

/// How many directory entries the walk looks at before it stops.
///
/// Twenty thousand. A Unity or Unreal install has far more than this in total,
/// which is exactly why the walk is capped: the artwork is in the first few
/// hundred entries of the top three levels or it is not there, and grinding
/// through a `Content/Paks` tree to prove it costs a user a visible pause on
/// every library refresh.
const HADD_MUDAKHALAT: usize = 20_000;

/// How many entries a single bounded `read_dir` looks at.
///
/// Four thousand covers `img/titles1` — an RPG Maker game with a hundred title
/// screens is unusual and a thousand is unheard of — while stopping a directory
/// that somebody filled with a million files from turning one probe into a
/// minute of `readdir`.
const HADD_MUDAKHALAT_MUJALLAD: usize = 4096;

/// How many anchor directories of each kind are remembered.
///
/// Eight. A game has one `Content` directory; a game bundled with its own
/// editor or with three DLC projects has a handful. Past eight, the walk has
/// found something that is not a game layout and probing all of it is work
/// nobody asked for.
const HADD_MARASI: usize = 8;

/// How many candidates are returned.
///
/// Twenty-four. [`crate::silsila::Silsila::hall`] takes the first that passes,
/// and this list is sorted largest first, so anything past the first few is
/// only ever reached when everything before it was the wrong shape. Twenty-four
/// is generous room for that and a hard stop on a directory full of `.ico`
/// files.
const HADD_MURASHAHAT: usize = 24;

/// How many candidate paths are gathered before measuring begins.
///
/// Measuring is one `open` and one header read per path, so the gathering cap
/// is what actually bounds the syscall count. Four times the return cap leaves
/// room for candidates that turn out to be unreadable or the wrong shape.
const HADD_MUJAMMAA: usize = HADD_MURASHAHAT * 4;

/// How many bytes are read from an `.ico` or `.bmp` to measure it.
///
/// Eight kibibytes. A `BITMAPFILEHEADER` plus `BITMAPINFOHEADER` is 54 bytes
/// and an `ICONDIR` with the maximum entry count is 1030; the rest is slack so
/// that a file with an unusual header size is still measured rather than
/// refused.
const HADD_TARWISA: u64 = 8 * 1024;

/// Largest `project.godot` read.
const HADD_HAJM_MASHRUE: u64 = 512 * 1024;

/// Largest side, in pixels, a candidate may declare.
///
/// Sixteen thousand three hundred and eighty-four. Past this the file is not
/// artwork: it is a texture atlas or a header that is lying, and either way it
/// is not going on a card.
const HADD_BUD: u32 = 16_384;

/// Largest pixel count a candidate may declare.
///
/// Forty megapixels, matching [`crate::suwar`]'s ceiling, so that a candidate
/// this module offers cannot be one the artwork store will refuse to import a
/// moment later. Offering a picture the next stage is guaranteed to reject
/// would be a diagnostics trail that says "taken" followed by a blank card.
const HADD_BIKSILAT: u64 = 40_000_000;

// ---------------------------------------------------------------------------
// What to look for
// ---------------------------------------------------------------------------

/// Exact paths relative to the install root, with the label each one earns.
///
/// These are `stat` calls, not searches, and they exist next to the walk rather
/// than inside it because a path carries information a file name does not:
/// `www/icon/icon.png` is an RPG Maker MV icon and `icon/icon.png` is an MZ
/// one, and both are just `icon.png` to a name matcher.
const MASARAT_MABASHIRA: &[(&str, &str)] = &[
    // --- Ren'Py -----------------------------------------------------------
    ("game/gui/window_icon.png", "Ren'Py window icon"),
    ("game/gui/main_menu.png", "Ren'Py main menu art"),
    ("game/gui/game_menu.png", "Ren'Py game menu art"),
    ("game/gui/overlay/main_menu.png", "Ren'Py main menu overlay"),
    // --- RPG Maker --------------------------------------------------------
    ("Game.ico", "RPG Maker game icon"),
    ("www/icon/icon.png", "RPG Maker MV icon"),
    ("icon/icon.png", "RPG Maker MZ icon"),
    // --- Godot ------------------------------------------------------------
    ("icon.png", "Godot project icon"),
    // --- GameMaker --------------------------------------------------------
    ("splash.png", "GameMaker splash"),
    // --- Electron / NW.js -------------------------------------------------
    ("resources/app/icon.png", "Electron application icon"),
    ("resources/app/build/icon.png", "Electron build icon"),
    ("package.nw/icon.png", "NW.js application icon"),
    // --- generic, and the ones storefront tooling leaves behind ------------
    ("icon.ico", "Windows icon at the install root"),
    ("cover.png", "cover art at the install root"),
    ("cover.jpg", "cover art at the install root"),
    ("cover.webp", "cover art at the install root"),
    ("capsule.png", "storefront capsule at the install root"),
    ("capsule.jpg", "storefront capsule at the install root"),
    ("header.jpg", "storefront header at the install root"),
    ("header.png", "storefront header at the install root"),
    ("logo.png", "logo at the install root"),
    ("logo.jpg", "logo at the install root"),
    ("boxart.png", "box art at the install root"),
    ("boxart.jpg", "box art at the install root"),
    ("boxart.jpeg", "box art at the install root"),
    ("boxart.webp", "box art at the install root"),
    ("folder.jpg", "folder art at the install root"),
    ("folder.png", "folder art at the install root"),
];

/// File names the walk picks up wherever it finds them, with a generic label.
///
/// Generic on purpose. The walk has no path context, so `icon.png` three levels
/// down is described as an application icon and its actual location goes in the
/// same line — `application icon: launcher/assets/icon.png` reads correctly and
/// claiming an engine would not.
const ASMAA_MUHIMMA: &[(&str, &str)] = &[
    ("icon.png", "application icon"),
    ("icon.ico", "Windows icon"),
    ("game.ico", "RPG Maker game icon"),
    ("app.ico", "application icon"),
    ("splash.png", "splash image"),
    ("splash.bmp", "Unreal splash"),
    ("edsplash.bmp", "Unreal editor splash"),
    ("window_icon.png", "Ren'Py window icon"),
    ("main_menu.png", "Ren'Py main menu art"),
    ("title.png", "title screen"),
    ("cover.png", "cover art"),
    ("cover.jpg", "cover art"),
    ("capsule.png", "storefront capsule"),
    ("header.jpg", "storefront header"),
    ("logo.png", "logo"),
    ("boxart.png", "box art"),
    ("boxart.jpg", "box art"),
    ("folder.jpg", "folder art"),
];

/// Directory names the walk never descends into.
///
/// Each holds five to six figures of files and no artwork, and each is on the
/// direct path between an install root and the three levels this walk cares
/// about — so without the list the entry cap is spent inside one of them before
/// the walk ever reaches `Content/Splash`.
const MUJALLADAT_MUSTATHNAT: &[&str] = &[
    // Unity: streamed assets, the managed assembly set, the IL2CPP metadata
    // tree, and the bundled Mono runtime.
    "StreamingAssets",
    "Managed",
    "il2cpp_data",
    "MonoBleedingEdge",
    "Plugins",
    // Unreal: the cooked content archives and the shader cache.
    "Paks",
    "Movies",
    "DerivedDataCache",
    // Electron and Node: locales, the module tree, the ANGLE shader compiler.
    "locales",
    "node_modules",
    "swiftshader",
    // Ren'Py's Python runtime tree, and version control metadata.
    "lib",
    ".git",
    ".svn",
];

/// The extensions a candidate may have.
///
/// Exactly the five this crate can measure: three that `image` reads headers
/// for, plus the two [`crate::ayquna`] measures itself. An extension outside
/// this set is not offered, because a candidate whose size cannot be read is a
/// candidate [`MurashahSura::kaf`] would have to wave through unchecked.
const IMTIDADAT: &[&str] = &["png", "jpg", "jpeg", "webp", "ico", "bmp"];

/// The keys `project.godot` may name artwork under, across Godot 3 and 4.
///
/// Godot moved the boot splash between `[application]` groups across major
/// versions and kept the old key readable, so both spellings are tried and
/// whichever exists wins.
const MAFATIH_GODOT: &[(&str, &str)] = &[
    ("config/icon", "Godot project icon"),
    ("config/windows_native_icon", "Godot Windows icon"),
    ("config/macos_native_icon", "Godot macOS icon"),
    ("boot_splash/image", "Godot boot splash"),
    ("config/boot_splash/image", "Godot boot splash"),
];

/// The `project.godot` group those keys live in.
const MAQTA_GODOT: &str = "application";

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Every piece of artwork the install itself ships, largest first.
///
/// Never fails and never logs a warning. A game with no artwork of its own is
/// the ordinary case for most engines, and the cascade falls through to rung 4
/// and then to the generated plate — which is a designed state, not an error.
/// What *is* recorded, at debug level, is anything that was found and passed
/// over for a stated reason: an SVG, a file whose header would not parse, a
/// picture too large to be artwork.
///
/// The list is sorted by pixel count, descending, because
/// [`crate::silsila::Silsila::hall`] takes candidates within a rung in the
/// order the caller supplied and the biggest usable picture is the one that
/// survives the upscale ceiling.
#[must_use]
pub fn fann_min_tathbeet(jidhr: &Path, tanfidhi: Option<&Path>) -> Vec<MurashahSura> {
    let mut mujammaa: Vec<(PathBuf, String)> = Vec::new();

    // Order matters and is the dedup policy: the first description a path gets
    // is the one it keeps, and the direct probes know more about a path than
    // the name matcher does.
    dumm(&mut mujammaa, murashahat_mubashira(jidhr));

    let hasad = imsah(jidhr);
    dumm(&mut mujammaa, murashahat_marasi(jidhr, &hasad));
    dumm(&mut mujammaa, murashahat_godot(jidhr, &hasad));
    dumm(&mut mujammaa, murashahat_tanfidhi(jidhr, tanfidhi));
    dumm(&mut mujammaa, hasad.malafat);

    // Everything gathered is measured, and the cap is applied *after* the sort.
    // Truncating first would cap by probe order, which is specificity order, and
    // a game with forty title screens would have its 1280×720 illustration cut
    // in favour of a 32-pixel icon that happened to be probed earlier. The
    // gathering cap already bounds this to at most `HADD_MUJAMMAA` header reads.
    let mut murashahat: Vec<MurashahSura> = Vec::with_capacity(mujammaa.len());
    for (masar, wasf) in mujammaa {
        let Some((ard, irtifa)) = abaad_malaf(&masar) else {
            continue;
        };
        murashahat.push(MurashahSura::bi_abaad(
            MasdarSura::Malaf(masar),
            MartabatSura::MinAlLuba,
            ard,
            irtifa,
            wasf,
        ));
    }

    murashahat.sort_by_key(|murashah| Reverse(masaha(murashah)));
    murashahat.truncate(HADD_MURASHAHAT);
    murashahat
}

/// The same candidates, grouped by which of a game's three pictures each one
/// could be.
///
/// This is the shape [`crate::silsila::admij`] folds and
/// [`crate::silsila::Silsila::hall_al_kul`] resolves, so it is what a caller
/// driving the cascade wants. The classification is
/// [`NawSura::tunasib`] and nothing else — there is exactly one aspect-ratio
/// test in this crate and it lives in `silsila`, because two tests that agreed
/// today would disagree the first time either was tuned.
#[must_use]
pub fn murashahat_fann(jidhr: &Path, tanfidhi: Option<&Path>) -> Vec<(NawSura, Vec<MurashahSura>)> {
    let mut ghilaf = Vec::new();
    let mut batl = Vec::new();
    let mut shiar = Vec::new();

    for murashah in fann_min_tathbeet(jidhr, tanfidhi) {
        let Some((ard, irtifa)) = murashah.abaad else {
            continue;
        };
        match naw_min_abaad(ard, irtifa) {
            NawSura::Ghilaf => ghilaf.push(murashah),
            NawSura::Batl => batl.push(murashah),
            NawSura::Shiar => shiar.push(murashah),
        }
    }

    vec![
        (NawSura::Ghilaf, ghilaf),
        (NawSura::Batl, batl),
        (NawSura::Shiar, shiar),
    ]
}

/// Which picture a candidate's shape makes it.
///
/// Cover first, then hero, then logo as the catch-all — and the catch-all is
/// the load-bearing part. A square 512-pixel application icon matches none of
/// the three ratios, and [`MurashahSura::kaf`] accepts an arbitrary shape only
/// for [`NawSura::Shiar`]. Filing it anywhere else would mean discarding the
/// one picture most indie games actually have.
fn naw_min_abaad(ard: u32, irtifa: u32) -> NawSura {
    if NawSura::Ghilaf.tunasib(ard, irtifa) {
        NawSura::Ghilaf
    } else if NawSura::Batl.tunasib(ard, irtifa) {
        NawSura::Batl
    } else {
        NawSura::Shiar
    }
}

/// A candidate's pixel count, for the sort.
fn masaha(murashah: &MurashahSura) -> u64 {
    murashah.abaad.map_or(0, |(ard, irtifa)| {
        u64::from(ard).saturating_mul(u64::from(irtifa))
    })
}

/// Appends candidates, skipping paths already gathered and honouring the
/// gathering cap.
///
/// First description wins, which is why the callers are ordered from most
/// specific to least: a file found both as `www/icon/icon.png` and as
/// `icon.png` is described as an RPG Maker MV icon rather than as "an
/// application icon".
fn dumm(mujammaa: &mut Vec<(PathBuf, String)>, jadeeda: Vec<(PathBuf, String)>) {
    for (masar, wasf) in jadeeda {
        if mujammaa.len() >= HADD_MUJAMMAA {
            return;
        }
        if mujammaa.iter().any(|(mawjud, _)| *mawjud == masar) {
            continue;
        }
        mujammaa.push((masar, wasf));
    }
}

// ---------------------------------------------------------------------------
// The direct probes
// ---------------------------------------------------------------------------

/// The fixed relative paths, `stat`ed one by one.
///
/// Around thirty `stat` calls against a directory that is almost certainly in
/// the page cache already, which is cheaper than one `readdir` of a Unity
/// `_Data` folder and far cheaper than the walk. This runs first so that its
/// precise labels win the dedup.
fn murashahat_mubashira(jidhr: &Path) -> Vec<(PathBuf, String)> {
    let mut murashahat = Vec::with_capacity(MASARAT_MABASHIRA.len());
    for (nisbi_masar, wasm) in MASARAT_MABASHIRA {
        let masar = jidhr.join(nisbi_masar);
        if masar.is_file() {
            murashahat.push((masar, format!("{wasm}: {nisbi_masar}")));
        }
    }

    // Godot's own template ships `icon.svg`, and a project that never replaced
    // it has no raster icon at all. Naming it in the log is the difference
    // between "no artwork" and "artwork exists in a format this build cannot
    // rasterize".
    for nisbi_masar in ["icon.svg", "logo.svg", "game/gui/window_icon.svg"] {
        let masar = jidhr.join(nisbi_masar);
        if masar.is_file() {
            sajjil_svg(&masar);
        }
    }

    murashahat
}

/// Artwork reached from an anchor directory the walk found.
///
/// Every entry here needs a directory whose *name* identifies the engine and a
/// file whose name is only meaningful inside it. `Splash.bmp` on its own says
/// nothing; `Content/Splash/Splash.bmp` is Unreal's splash screen, and that is
/// the whole reason the walk collects anchors instead of files.
fn murashahat_marasi(jidhr: &Path, hasad: &HasadMasah) -> Vec<(PathBuf, String)> {
    let mut murashahat: Vec<(PathBuf, String)> = Vec::new();

    // Unity. `<Game>_Data/splash*.png` is a plain file when a project has one;
    // the player-settings splash inside `globalgamemanagers` is not, and see
    // the module header for why it stays closed.
    for marsa in &hasad.bayanat {
        for masar in mudakhalat(marsa, |ism| {
            ism.starts_with("splash") && imtidad_maqbul(ism)
        }) {
            let wasf = format!("Unity splash: {}", nisbi(jidhr, &masar));
            murashahat.push((masar, wasf));
        }
    }

    // Unreal. The splash lives under the project's `Content`, not under the
    // install root, which is why it is anchored rather than probed directly.
    for marsa in &hasad.muhtawa {
        for (ism, wasm) in [
            ("Splash.bmp", "Unreal splash"),
            ("EdSplash.bmp", "Unreal editor splash"),
        ] {
            let masar = marsa.join("Splash").join(ism);
            if masar.is_file() {
                let wasf = format!("{wasm}: {}", nisbi(jidhr, &masar));
                murashahat.push((masar, wasf));
            }
        }
    }

    // `<Proj>/Resources/*.ico` — Unreal's packaged icon, and the same directory
    // an Electron build writes its icon into.
    for marsa in &hasad.mawarid {
        for masar in mudakhalat(marsa, |ism| imtidad_huwa(ism, "ico")) {
            let wasf = format!("packaged resource icon: {}", nisbi(jidhr, &masar));
            murashahat.push((masar, wasf));
        }
    }

    // `<Proj>/Binaries/Win64/*.ico`, and the two sibling target directories a
    // shipped Unreal build can use instead.
    for marsa in &hasad.thanaiyat {
        for manassa in ["Win64", "Win32", "WinGDK"] {
            for masar in mudakhalat(&marsa.join(manassa), |ism| imtidad_huwa(ism, "ico")) {
                let wasf = format!("Unreal binaries icon: {}", nisbi(jidhr, &masar));
                murashahat.push((masar, wasf));
            }
        }
    }

    // RPG Maker title screens. This is the best artwork in the whole table: a
    // full-bleed illustration drawn for the game at the resolution the game
    // runs at, rather than an icon.
    for marsa in &hasad.anawin {
        for masar in mudakhalat(marsa, imtidad_maqbul) {
            let wasf = format!("RPG Maker title screen: {}", nisbi(jidhr, &masar));
            murashahat.push((masar, wasf));
        }
    }

    // Ren'Py's `game/gui`, reached as an anchor for the projects that nest it
    // one level deeper than the direct probe expects.
    for marsa in &hasad.wajihat {
        for (ism, wasm) in [
            ("window_icon.png", "Ren'Py window icon"),
            ("main_menu.png", "Ren'Py main menu art"),
            ("game_menu.png", "Ren'Py game menu art"),
        ] {
            let masar = marsa.join(ism);
            if masar.is_file() {
                let wasf = format!("{wasm}: {}", nisbi(jidhr, &masar));
                murashahat.push((masar, wasf));
            }
        }
    }

    murashahat.truncate(HADD_MUJAMMAA);
    murashahat
}

/// Artwork `project.godot` names.
///
/// Godot exports put the project settings inside the `.pck`, so a
/// `project.godot` on disk means an unexported or a loosely packaged project —
/// which is exactly the case where there is no `.exe` icon worth having either,
/// so it is worth the read.
///
/// `project.godot` is INI-like and is read with [`crate::ayquna::iqra_maqati`],
/// the same reader the `.desktop` path uses. What differs is entirely in the
/// values, which is why the reader deliberately does not interpret them: a
/// Godot value is a quoted string holding a `res://` URL, where a `.desktop`
/// value is bare text with backslash escapes. Both are unwrapped by their own
/// caller.
fn murashahat_godot(jidhr: &Path, hasad: &HasadMasah) -> Vec<(PathBuf, String)> {
    let mut mashari: Vec<PathBuf> = Vec::new();
    let jidhri = jidhr.join("project.godot");
    if jidhri.is_file() {
        mashari.push(jidhri);
    }
    for mashru in &hasad.mashari {
        if !mashari.contains(mashru) {
            mashari.push(mashru.clone());
        }
    }

    let mut murashahat: Vec<(PathBuf, String)> = Vec::new();
    for mashru in mashari.iter().take(HADD_MARASI) {
        let Some(mujallad) = mashru.parent() else {
            continue;
        };
        let Some(nass) = iqra_nass_bi_hadd(mashru, HADD_HAJM_MASHRUE) else {
            continue;
        };
        let malaf = iqra_maqati(&nass);

        for (miftah, wasm) in MAFATIH_GODOT {
            // The group first, then anywhere, because Godot 3 and Godot 4 do
            // not agree about which `[application]` sub-path the boot splash
            // lives under and both spellings appear in the wild.
            let Some(khaam) = malaf
                .qeema(MAQTA_GODOT, miftah)
                .or_else(|| malaf.qeema_ay(miftah))
            else {
                continue;
            };
            let Some(qeema) = fukk_qeemat_godot(khaam) else {
                continue;
            };
            let Some(masar) = masar_min_res(mujallad, &qeema) else {
                continue;
            };
            if imtidad_saghir(&masar).as_deref() == Some("svg") {
                sajjil_svg(&masar);
                continue;
            }
            if masar.is_file() {
                murashahat.push((masar, format!("{wasm}: {qeema}")));
            }
        }
    }

    murashahat.truncate(HADD_MUJAMMAA);
    murashahat
}

/// Artwork sitting beside the executable itself.
///
/// GameMaker writes `icon.ico` here, Electron and NW.js leave the packager's
/// `.ico` here, and an Unreal shipping build puts the packaged icon in the same
/// directory as the binary. The executable's *stem* is also tried, because a
/// game called `Celeste.exe` shipped next to `Celeste.png` is a pattern several
/// itch.io build tools produce.
fn murashahat_tanfidhi(jidhr: &Path, tanfidhi: Option<&Path>) -> Vec<(PathBuf, String)> {
    let Some(tanfidhi) = tanfidhi else {
        return Vec::new();
    };
    let Some(mujallad) = tanfidhi.parent() else {
        return Vec::new();
    };

    let mut murashahat: Vec<(PathBuf, String)> = Vec::new();

    for masar in mudakhalat(mujallad, |ism| imtidad_huwa(ism, "ico")) {
        let wasf = format!("icon beside the executable: {}", nisbi(jidhr, &masar));
        murashahat.push((masar, wasf));
    }

    if let Some(ism_jidhr) = tanfidhi.file_stem().and_then(OsStr::to_str) {
        for imtidad in IMTIDADAT {
            let masar = mujallad.join(format!("{ism_jidhr}.{imtidad}"));
            if masar.is_file() {
                let wasf = format!("image named after the executable: {}", nisbi(jidhr, &masar));
                murashahat.push((masar, wasf));
            }
        }
        let matjah = mujallad.join(format!("{ism_jidhr}.svg"));
        if matjah.is_file() {
            sajjil_svg(&matjah);
        }
    }

    murashahat.truncate(HADD_MUJAMMAA);
    murashahat
}

// ---------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------

/// What one bounded pass over the install root produced.
#[derive(Debug, Default)]
struct HasadMasah {
    /// Files whose bare name is one of [`ASMAA_MUHIMMA`], already labelled.
    malafat: Vec<(PathBuf, String)>,
    /// Unity's `<Game>_Data` directories.
    bayanat: Vec<PathBuf>,
    /// Unreal's `Content` directories.
    muhtawa: Vec<PathBuf>,
    /// `Resources` directories, which Unreal and Electron both use.
    mawarid: Vec<PathBuf>,
    /// Unreal's `Binaries` directories.
    thanaiyat: Vec<PathBuf>,
    /// RPG Maker's `titles1` directories.
    anawin: Vec<PathBuf>,
    /// Ren'Py's `gui` directories.
    wajihat: Vec<PathBuf>,
    /// Every `project.godot` found.
    mashari: Vec<PathBuf>,
}

/// One bounded pass over the install root.
///
/// Sorted by file name so that two scans of the same directory produce the same
/// list in the same order. That is not tidiness: the description that ends up
/// on a card is chosen by this order, and a card whose artwork silently changed
/// between two scans of an unchanged directory is a bug nobody can reproduce.
///
/// Symlinks are not followed. A game directory containing a link back to its
/// own parent is not hypothetical — Proton prefixes and Wine `dosdevices` trees
/// are full of them — and `walkdir` will happily recurse until the depth cap,
/// producing the same files under a dozen names.
fn imsah(jidhr: &Path) -> HasadMasah {
    let mut hasad = HasadMasah::default();
    let mut adad = 0_usize;

    let sayr = WalkDir::new(jidhr)
        .max_depth(HADD_UMQ)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(qabil_lil_nuzul);

    for mudkhal in sayr.flatten() {
        adad = adad.saturating_add(1);
        if adad > HADD_MUDAKHALAT {
            tracing::debug!(
                jidhr = %jidhr.display(),
                hadd = HADD_MUDAKHALAT,
                "the artwork walk hit its entry cap and stopped"
            );
            break;
        }

        let masar = mudkhal.path();
        let Some(ism) = masar.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let saghir = ism.to_ascii_lowercase();

        if mudkhal.file_type().is_dir() {
            // Depth zero is the install root itself, whose name means nothing.
            if mudkhal.depth() == 0 {
                continue;
            }
            if saghir.ends_with("_data") {
                dumm_marsa(&mut hasad.bayanat, masar);
            } else if saghir == "content" {
                dumm_marsa(&mut hasad.muhtawa, masar);
            } else if saghir == "resources" {
                dumm_marsa(&mut hasad.mawarid, masar);
            } else if saghir == "binaries" {
                dumm_marsa(&mut hasad.thanaiyat, masar);
            } else if saghir == "titles1" {
                dumm_marsa(&mut hasad.anawin, masar);
            } else if saghir == "gui" {
                dumm_marsa(&mut hasad.wajihat, masar);
            }
            continue;
        }

        if saghir == "project.godot" {
            dumm_marsa(&mut hasad.mashari, masar);
            continue;
        }
        if imtidad_huwa(&saghir, "svg") {
            sajjil_svg(masar);
            continue;
        }
        if hasad.malafat.len() >= HADD_MUJAMMAA {
            continue;
        }
        if let Some((_, wasm)) = ASMAA_MUHIMMA
            .iter()
            .find(|(matlub, _)| *matlub == saghir.as_str())
        {
            let wasf = format!("{wasm}: {}", nisbi(jidhr, masar));
            hasad.malafat.push((masar.to_path_buf(), wasf));
        }
    }

    hasad
}

/// Whether the walk descends into an entry.
///
/// Files always pass — the filter's only job is to keep the walk out of the
/// directories in [`MUJALLADAT_MUSTATHNAT`]. The root is exempt, because an
/// install root that happens to be called `Plugins` is still the directory the
/// caller asked about.
fn qabil_lil_nuzul(mudkhal: &walkdir::DirEntry) -> bool {
    if mudkhal.depth() == 0 || !mudkhal.file_type().is_dir() {
        return true;
    }
    let Some(ism) = mudkhal.file_name().to_str() else {
        return true;
    };
    !MUJALLADAT_MUSTATHNAT
        .iter()
        .any(|mahjub| ism.eq_ignore_ascii_case(mahjub))
}

/// Records an anchor, up to the per-kind cap.
fn dumm_marsa(marasi: &mut Vec<PathBuf>, masar: &Path) {
    if marasi.len() < HADD_MARASI {
        marasi.push(masar.to_path_buf());
    }
}

// ---------------------------------------------------------------------------
// Measuring, and the small readers
// ---------------------------------------------------------------------------

/// A candidate's dimensions, from its header and nothing else.
///
/// PNG, JPEG and WebP go through [`ImageReader::into_dimensions`], which parses
/// the `IHDR`, the `SOF` marker or the `VP8X` chunk and stops. `.ico` and
/// `.bmp` have no reader in `image` as this workspace builds it and are
/// measured from a short prefix of the file by [`crate::ayquna`].
///
/// Returns [`None`] for anything unreadable, unmeasurable, or larger than a
/// picture has any business being — logged at debug with the path, because a
/// candidate silently dropped is a candidate nobody can ask about later.
fn abaad_malaf(masar: &Path) -> Option<(u32, u32)> {
    let imtidad = imtidad_saghir(masar)?;
    let (ard, irtifa) = match imtidad.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => ImageReader::open(masar)
            .ok()?
            .with_guessed_format()
            .ok()?
            .into_dimensions()
            .ok()?,
        "ico" | "cur" => abaad_ico(&tarwisa(masar)?)?,
        "bmp" => abaad_bmp(&tarwisa(masar)?)?,
        _ => return None,
    };

    let biksilat = u64::from(ard).saturating_mul(u64::from(irtifa));
    if ard == 0 || irtifa == 0 || ard > HADD_BUD || irtifa > HADD_BUD || biksilat > HADD_BIKSILAT {
        tracing::debug!(
            masar = %masar.display(),
            ard,
            irtifa,
            "an artwork candidate declares dimensions outside the limits and was passed over"
        );
        return None;
    }
    Some((ard, irtifa))
}

/// The first [`HADD_TARWISA`] bytes of a file.
fn tarwisa(masar: &Path) -> Option<Vec<u8>> {
    let malaf = std::fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(HADD_TARWISA).read_to_end(&mut bayt).ok()?;
    (!bayt.is_empty()).then_some(bayt)
}

/// A whole small text file, lossily decoded.
///
/// Lossy because `project.godot` is written by an editor that has shipped on
/// three platforms for a decade and a stray byte in a comment is not a reason
/// to lose the icon path six lines below it.
fn iqra_nass_bi_hadd(masar: &Path, hadd: u64) -> Option<String> {
    let malaf = std::fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(hadd).read_to_end(&mut bayt).ok()?;
    (!bayt.is_empty()).then(|| String::from_utf8_lossy(&bayt).into_owned())
}

/// A file's extension, lowercased.
fn imtidad_saghir(masar: &Path) -> Option<String> {
    masar
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
}

/// Whether a file name carries a given extension, in any case.
fn imtidad_huwa(ism: &str, imtidad: &str) -> bool {
    Path::new(ism)
        .extension()
        .is_some_and(|mawjud| mawjud.eq_ignore_ascii_case(imtidad))
}

/// Whether a lowercased file name ends in an extension this crate can measure.
fn imtidad_maqbul(ism: &str) -> bool {
    IMTIDADAT.iter().any(|imtidad| {
        ism.len() > imtidad.len().saturating_add(1)
            && ism.ends_with(imtidad)
            && ism
                .get(..ism.len().saturating_sub(imtidad.len()))
                .is_some_and(|asas| asas.ends_with('.'))
    })
}

/// Files in one directory that satisfy a predicate, sorted, bounded.
///
/// The predicate is given the **lowercased** name, so every caller above can be
/// written with lowercase literals and still match `SPLASH.PNG` on a
/// case-sensitive filesystem — which is what a Windows game unpacked onto ext4
/// under Proton actually looks like.
///
/// The result is sorted so that a directory of forty title screens always
/// yields the same first candidate.
fn mudakhalat(mujallad: &Path, mustahiq: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(qari) = std::fs::read_dir(mujallad) else {
        return Vec::new();
    };
    let mut masarat: Vec<PathBuf> = Vec::new();
    for mudkhal in qari.take(HADD_MUDAKHALAT_MUJALLAD).flatten() {
        if masarat.len() >= HADD_MUJAMMAA {
            break;
        }
        let masar = mudkhal.path();
        let Some(ism) = masar.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let saghir = ism.to_ascii_lowercase();
        if imtidad_huwa(&saghir, "svg") {
            sajjil_svg(&masar);
            continue;
        }
        if !mustahiq(&saghir) || !masar.is_file() {
            continue;
        }
        masarat.push(masar);
    }
    masarat.sort();
    masarat
}

/// A candidate's path relative to the install root, for the description.
///
/// Falls back to the whole path when the candidate is somehow outside the root,
/// which is better than a description that silently loses where the file was.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    masar
        .strip_prefix(jidhr)
        .unwrap_or(masar)
        .display()
        .to_string()
}

/// Unwraps a `project.godot` value.
///
/// Godot writes every string setting quoted. The quotes are stripped only as a
/// matched pair, so a value that is missing one is left exactly as written
/// rather than being half-unwrapped into a path that does not exist.
fn fukk_qeemat_godot(khaam: &str) -> Option<String> {
    let maqsus = khaam.trim();
    let dakhil = maqsus
        .strip_prefix('"')
        .and_then(|baqi| baqi.strip_suffix('"'))
        .unwrap_or(maqsus);
    (!dakhil.is_empty()).then(|| dakhil.to_owned())
}

/// Turns a Godot resource reference into a real path.
///
/// `res://` is the project root, which on disk is the directory holding
/// `project.godot`. A `uid://` reference is refused: it is an opaque identifier
/// resolved through `.godot/uid_cache.bin`, a binary index that only the editor
/// writes, and guessing at it would be guessing. A bare relative path is
/// accepted, because hand-edited project files contain them.
///
/// Any reference containing `..` is refused outright. The value comes out of a
/// file inside the game directory, and a settings file that escapes its own
/// project root is either broken or trying something.
fn masar_min_res(mujallad: &Path, qeema: &str) -> Option<PathBuf> {
    if qeema.starts_with("uid://") {
        return None;
    }
    let nisbi_masar = qeema.strip_prefix("res://").unwrap_or(qeema);
    if nisbi_masar.is_empty() || nisbi_masar.contains("..") || nisbi_masar.contains("://") {
        return None;
    }
    if Path::new(nisbi_masar).is_absolute() {
        return None;
    }
    Some(mujallad.join(nisbi_masar))
}

/// Records an SVG that was found and passed over.
///
/// Debug rather than a warning, and never an error on the result. Most games
/// that ship an SVG also ship a raster icon, and a scan that emitted a warning
/// for every Godot project's default `icon.svg` would train users to ignore
/// warnings. The line exists so that the one game where the SVG was the only
/// artwork can be explained.
fn sajjil_svg(masar: &Path) {
    tracing::debug!(
        masar = %masar.display(),
        "artwork found in SVG, which this workspace has no rasterizer for"
    );
}
