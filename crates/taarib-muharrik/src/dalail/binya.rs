//! بنية المجلد — what a game's own directory shape says about its engine.
//!
//! The cheapest detector and the broadest one. It never opens a file: it lists
//! directories and matches what it finds against the layouts engines are
//! *obliged* to ship, because the engine's own runtime has to find them too. A
//! Unity player looks for `Foo_Data` beside itself before it draws a frame; an
//! Unreal build mounts `Content/Paks`; Ren'Py imports `renpy`; NW.js opens
//! `package.json`. None of that is convention a developer can rename away
//! without breaking their own game, which is what makes directory shape worth
//! reading first.
//!
//! It is also wrong often enough that it never concludes. A launcher that ships
//! a second engine beside the game, a Godot title wrapped in Electron, an RPG
//! Maker MZ project that is JavaScript inside NW.js inside a Windows binary —
//! all of those produce two or three true observations at once, and resolving
//! them is [`crate::tahdid`]'s job, not this file's.
//!
//! ## One walk
//!
//! Every shape below is matched against a single breadth-first listing of the
//! game directory, collected once and then queried in memory. Eight independent
//! walks of a game with a hundred thousand asset files is eight times the I/O
//! for the same answer, and on a network share or a spinning disk that is the
//! difference between a library scan the user waits through and one they do
//! not.
//!
//! Breadth-first rather than depth-first on purpose: the entry budget is spent
//! on the shallow entries first, and engine markers are shallow by
//! construction.
//!
//! The walk stops at [`AQSA_UMQ`], with one narrow exception described at
//! [`MUJALLADAT_AMEEQA`]. When it stops at [`AQSA_MADAKHIL`] instead of
//! finishing, that fact is recorded as evidence in its own right — a truncated
//! walk is never presented as a complete one.
//!
//! ## The name binding, which is the most useful fact in Unity detection
//!
//! Unity's player does not search for its data directory. It derives the name:
//! `Foo.exe` loads `Foo_Data`, `Foo.x86_64` loads `Foo_Data`, and
//! `Foo.app/Contents/MacOS/Foo` loads `Foo.app/Contents/Resources/Data`. The
//! binding is the engine's own rule rather than a habit, so it reads in both
//! directions, and the reverse direction is the valuable one: finding
//! `Foo_Data` names the game's executable even when discovery never did. When
//! this detector learns an executable that way it reports it in
//! [`HasilatFahs::tanfidhi`], and every later stage — the binary detector, the
//! injector, the installer — gets a real path instead of a guess.
//!
//! ## Weights
//!
//! The scale is the one [`HasilatFahs::sajjil`] documents: 90 and up for a file
//! only one engine ever ships, 40 to 60 for a file several engines share, below
//! 30 for a name that merely suggests something.
//!
//! | shape | weight | why |
//! | --- | ---: | --- |
//! | `Foo_Data/` beside `Foo.exe` | 95 | the engine's own naming rule, in both directions |
//! | `Foo_Data/Managed/Assembly-CSharp.dll` | 95 | the game's own compiled script assembly |
//! | `Foo_Data/il2cpp_data/Metadata/global-metadata.dat` | 95 | IL2CPP ships nothing else like it |
//! | `www/js/rpg_core.js`, `js/rmmz_core.js` | 95 | the runtime file the editor writes verbatim |
//! | `Game.rgss3a` | 95 | the RGSS3 archive, and only VX Ace writes one |
//! | `data.win`, `game.unx` | 92 | the GameMaker FORM container under its platform name |
//! | `resources/app.asar` | 92 | Electron's archive format, used by nothing else |
//! | `Foo.pck` beside `Foo.exe` | 92 | Godot's pack, bound to the executable by name |
//! | `renpy/__init__.py` | 92 | the engine package the game imports at startup |
//! | `UnityPlayer.dll` and friends | 90 | the player runtime itself |
//! | `Foo_Data/globalgamemanagers`, `data.unity3d` | 90 | Unity's own serialized root |
//! | `Engine/Binaries/` | 85 | present in every shipped Unreal build |
//! | `nw.pak` | 88 | NW.js, distinct from plain Electron |
//! | `RGSS300.dll` | 85 | the VX Ace interpreter shipped beside the game |
//! | `*.utoc`, `*.ucas` | 88 | an IoStore container pair, which is how UE5 packages content |
//! | `Content/Paks/*.pak` | 85 | the archive an Unreal build mounts at startup |
//! | `lib/py3-*` | 80 | Ren'Py's bundled interpreter, named by version and platform |
//! | `chrome_100_percent.pak` | 85 | a Chromium resource pack |
//! | `audiogroup*.dat` | 70 | GameMaker's split audio groups |
//! | `icudtl.dat` | 60 | Chromium, but also anything else embedding ICU |
//! | `Foo.console.exe` | 65 | Godot 4's Windows console companion |
//! | `Game.ini` | 25 | an RPG Maker name, but the proof is inside the file |
//! | a lone executable and nothing else | 20 | possibly a Godot build with the pack appended |
//!
//! ## What this detector deliberately does not do
//!
//! It never opens a file. `package.json` is matched as a *shape* — it exists,
//! and `www/` or `resources/app/` sits with it — and never read; `Game.ini` is
//! noted without checking it for `RGSS3`. Reading file contents is the third
//! evidence source's job, and splitting it that way is what lets this one stay
//! fast enough to run over an entire library.
//!
//! It also never ranks executables. Choosing the game's binary out of a folder
//! full of installers, launchers and crash handlers is discovery's problem and
//! discovery solves it, in `taarib_kashf::matajir::yadawi::rattib_tanfidhiyat`.
//! The only executable this file will ever name is one an engine's own naming
//! rule *proves*, which is a different thing from a ranking and cannot disagree
//! with one.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use taarib_mustalahat::muharrik::{
    AilatMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel, WajihaRusum,
};
use taarib_usus::khata::{Natija, Tafsir};

use crate::dalail::imtidad;
use crate::fahs::{AQSA_MADAKHIL, AQSA_UMQ, Fahis, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

/// How many levels below [`AQSA_UMQ`] a marker directory may be expanded.
///
/// Two, and only along [`MUJALLADAT_AMEEQA`], which puts the deepest reachable
/// entry at five levels below the game root.
pub const AQSA_UMQ_ZAID: usize = 2;

/// Directory names that buy their own subtree one extra level of walk.
///
/// [`AQSA_UMQ`] is three because three levels is where engine markers live, and
/// deepening it in general would cost real time on an asset tree while finding
/// nothing. Four shapes in the coverage matrix sit one level past it anyway:
/// `<Game>_Data/il2cpp_data/Metadata/global-metadata.dat`,
/// `Engine/Binaries/Win64/…`, `<Project>/Content/Paks/*.pak`, and the macOS
/// `<Game>.app/Contents/Resources/Data` layout.
///
/// So the extra level is granted by *name*, to directories that are themselves
/// engine markers. An asset tree is not called `il2cpp_data`, so the deepening
/// costs nothing where the ceiling was protecting anything, and the entry
/// budget still bounds it absolutely.
pub const MUJALLADAT_AMEEQA: &[&str] = &[
    "app",
    "binaries",
    "contents",
    "frameworks",
    "il2cpp_data",
    "js",
    "lib",
    "macos",
    "managed",
    "metadata",
    "paks",
    "plugins",
    "resources",
    "renpy",
    "win32",
    "win64",
    "www",
];

/// Executable extensions an engine's name binding can resolve to.
///
/// Windows first because it is where the binding is used most, then the two
/// suffixes Unity gives a Linux player, then the bare name a Unix build carries
/// when it has no extension at all.
const IMTIDADAT_TANFIDH: &[&str] = &["exe", "x86_64", "x86", "app", ""];

/// Detection by directory shape.
///
/// Holds no state and is safe to construct per probe; the walk it performs
/// lives entirely inside one call to [`Fahis::ifhas`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisBinya;

impl FahisBinya {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Fahis for FahisBinya {
    fn ism(&self) -> &'static str {
        "binya"
    }

    /// # Errors
    ///
    /// Only when the game's root directory cannot be listed at all — it does
    /// not exist, or permission is denied on it. Everything below that is the
    /// absence of evidence: a subdirectory that will not open is skipped, and
    /// the shapes that did match are still reported.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let mashhad = imsah(siyaq.jidhr)?;
        let mut hasad =
            Hasad { tanfidhi_maruf: siyaq.tanfidhi.is_some(), ..Hasad::default() };

        if mashhad.mabtur {
            let khata = KhataMuharrik::TajawuzHadd {
                jidhr: siyaq.jidhr.to_path_buf(),
                adad: u32::try_from(mashhad.adad).unwrap_or(u32::MAX),
            };
            tracing::warn!(
                jidhr = %siyaq.jidhr.display(),
                adad = mashhad.adad,
                "directory walk hit its entry ceiling; the shape evidence is partial"
            );
            hasad.sajjil(khata.injilizi(), None, 0);
        }

        unity(siyaq, &mashhad, &mut hasad);
        unreal(&mashhad, &mut hasad);
        godot(&mashhad, &mut hasad);
        rpg_maker(&mashhad, &mut hasad);
        renpy(&mashhad, &mut hasad);
        game_maker(&mashhad, &mut hasad);
        chromium(&mashhad, &mut hasad);
        rusum(&mashhad, &mut hasad);
        mufrad(&mashhad, &mut hasad);

        Ok(hasad.ikhtim())
    }
}

// ---------------------------------------------------------------------------
// what one walk collected
// ---------------------------------------------------------------------------

/// One entry the walk saw.
///
/// Three representations of the same thing, kept together because each match
/// below wants a different one and none of them should have to allocate to get
/// it: the relative path as the filesystem spells it (for evidence and for
/// derived lookups), the file name folded to ASCII lowercase (for prefix and
/// suffix tests), and the absolute path (for [`HasilatFahs::tanfidhi`]).
#[derive(Debug, Clone)]
struct Madkhal {
    /// Path relative to the game root, `/`-separated, in the case the
    /// filesystem reported.
    nisbi: String,
    /// The file name alone, folded to ASCII lowercase.
    ism: String,
    /// The absolute path.
    masar: PathBuf,
    /// Whether it is a directory.
    mujallad: bool,
    /// How far below the root it sits; 1 is an immediate child.
    umq: usize,
}

impl Madkhal {
    /// The name of the directory holding this entry, as the filesystem spells
    /// it, or an empty string when the entry sits at the game root.
    fn abb(&self) -> &str {
        let Some(qata) = self.nisbi.rfind('/') else {
            return "";
        };
        let abb = self.nisbi.get(..qata).unwrap_or_default();
        abb.rsplit('/').next().unwrap_or_default()
    }

    /// Whether the directory holding this entry is called `ism`, compared
    /// without regard to case.
    fn fi(&self, ism: &str) -> bool {
        self.abb().eq_ignore_ascii_case(ism)
    }

    /// Whether this entry's relative path is exactly `matlub`, compared without
    /// regard to case.
    ///
    /// Case-insensitively because engine markers are spelled inconsistently
    /// across exports and because the same game must be found on Windows and on
    /// Linux; the comparison allocates nothing.
    fn huwa(&self, matlub: &str) -> bool {
        self.nisbi.eq_ignore_ascii_case(matlub)
    }

    /// The file name with `lahiqa` removed from its end, when it ends that way.
    fn bidun_lahiqa(&self, lahiqa: &str) -> Option<&str> {
        self.ism.strip_suffix(lahiqa)
    }
}

/// Everything one walk of the game directory found.
#[derive(Debug, Default)]
struct Mashhad {
    /// Every entry, in breadth-first order.
    madakhil: Vec<Madkhal>,
    /// How many entries were looked at.
    adad: usize,
    /// Whether the walk stopped at [`AQSA_MADAKHIL`] rather than finishing.
    mabtur: bool,
}

impl Mashhad {
    /// Every entry, for the matches that scan rather than look up.
    fn kul(&self) -> impl Iterator<Item = &Madkhal> {
        self.madakhil.iter()
    }

    /// The entry at exactly this relative path, if the walk saw one.
    fn ind(&self, nisbi: &str) -> Option<&Madkhal> {
        self.madakhil.iter().find(|madkhal| madkhal.huwa(nisbi))
    }

    /// Whether the walk saw a directory at exactly this relative path.
    fn mujallad(&self, nisbi: &str) -> Option<&Madkhal> {
        self.ind(nisbi).filter(|madkhal| madkhal.mujallad)
    }

    /// The first file directly at the game root whose lowercase name is `ism`.
    ///
    /// Restricted to the root deliberately, and used only for the names that
    /// mean something *because* they are at the root: a `package.json` two
    /// directories down is one of a thousand node modules, and `Game.ini` two
    /// directories down is a configuration file for a tool the game bundles.
    fn bism(&self, ism: &str) -> Option<&Madkhal> {
        self.madakhil
            .iter()
            .find(|madkhal| !madkhal.mujallad && madkhal.umq == 1 && madkhal.ism == ism)
    }

    /// The first file anywhere the walk reached whose lowercase name is `ism`.
    fn bism_ayn(&self, ism: &str) -> Option<&Madkhal> {
        self.madakhil.iter().find(|madkhal| !madkhal.mujallad && madkhal.ism == ism)
    }

    /// The first file anywhere matching any of `asmaa`.
    fn bi_ahad(&self, asmaa: &[&str]) -> Option<&Madkhal> {
        asmaa.iter().find_map(|ism| self.bism_ayn(ism))
    }

    /// The first file called `ism` sitting inside a directory called `abb`.
    ///
    /// The form most of the shapes below actually need: `rpg_core.js` inside
    /// `js`, `app.asar` inside `resources`, `__init__.py` inside `renpy`. It
    /// finds the marker whether the game ships it at the root or three levels
    /// down inside a macOS bundle, which is the same marker either way.
    fn fi(&self, abb: &str, ism: &str) -> Option<&Madkhal> {
        self.madakhil
            .iter()
            .find(|madkhal| !madkhal.mujallad && madkhal.ism == ism && madkhal.fi(abb))
    }

    /// The first directory called `ism` sitting inside a directory called
    /// `abb`.
    fn mujallad_fi(&self, abb: &str, ism: &str) -> Option<&Madkhal> {
        self.madakhil
            .iter()
            .find(|madkhal| madkhal.mujallad && madkhal.ism == ism && madkhal.fi(abb))
    }
}

// ---------------------------------------------------------------------------
// the walk
// ---------------------------------------------------------------------------

/// Lists the game directory once, breadth-first, within both ceilings.
///
/// Symbolic links are recorded as entries and never descended into, so a game
/// directory holding a link to `/` — which installers really do leave behind —
/// costs one entry rather than the machine.
///
/// # Errors
///
/// Only for the game root itself: [`KhataMuharrik::JidhrMafqud`] when it is not
/// there and [`KhataMuharrik::TaadhurQiraatJidhr`] when it will not open. A
/// subdirectory that will not open is skipped in silence, because a game with
/// one unreadable folder is still a game whose engine can be identified.
fn imsah(jidhr: &Path) -> Natija<Mashhad> {
    let jidhr_qaima = std::fs::read_dir(jidhr).map_err(|sabab| {
        if sabab.kind() == std::io::ErrorKind::NotFound {
            KhataMuharrik::JidhrMafqud { jidhr: jidhr.to_path_buf() }
        } else {
            KhataMuharrik::TaadhurQiraatJidhr { jidhr: jidhr.to_path_buf(), sabab }
        }
    })?;

    let mut mashhad = Mashhad::default();
    let mut saff: VecDeque<(PathBuf, String, usize, usize)> = VecDeque::new();
    adrij(&mut mashhad, &mut saff, jidhr_qaima, "", 1, AQSA_UMQ);

    while let Some((masar, nisbi, umq, hadd)) = saff.pop_front() {
        if mashhad.mabtur {
            break;
        }
        let Ok(qaima) = std::fs::read_dir(&masar) else {
            continue;
        };
        adrij(&mut mashhad, &mut saff, qaima, &nisbi, umq, hadd);
    }

    Ok(mashhad)
}

/// Records one directory's entries and queues the subdirectories worth opening.
///
/// `umq` is the depth of the entries about to be recorded, and `hadd` is the
/// deepest level anything in this subtree may be recorded at.
fn adrij(
    mashhad: &mut Mashhad,
    saff: &mut VecDeque<(PathBuf, String, usize, usize)>,
    qaima: std::fs::ReadDir,
    nisbi_abb: &str,
    umq: usize,
    hadd: usize,
) {
    for madkhal in qaima.flatten() {
        if mashhad.adad >= AQSA_MADAKHIL {
            mashhad.mabtur = true;
            return;
        }
        mashhad.adad = mashhad.adad.saturating_add(1);

        let Ok(naw) = madkhal.file_type() else {
            continue;
        };
        let khaam = madkhal.file_name().to_string_lossy().into_owned();
        if khaam.is_empty() {
            continue;
        }
        let ism = khaam.to_ascii_lowercase();
        let nisbi = if nisbi_abb.is_empty() {
            khaam
        } else {
            format!("{nisbi_abb}/{khaam}")
        };
        let masar = madkhal.path();
        let mujallad = naw.is_dir();

        if mujallad && !naw.is_symlink() {
            // A marker directory buys its own subtree one level, never more
            // than `AQSA_UMQ_ZAID` past the ceiling in total.
            let hadd_ibn = if mujallad_ameeq(&ism) {
                hadd.max(umq.saturating_add(1)).min(AQSA_UMQ.saturating_add(AQSA_UMQ_ZAID))
            } else {
                hadd
            };
            if umq < hadd_ibn {
                saff.push_back((masar.clone(), nisbi.clone(), umq.saturating_add(1), hadd_ibn));
            }
        }

        mashhad.madakhil.push(Madkhal { nisbi, ism, masar, mujallad, umq });
    }
}

// ---------------------------------------------------------------------------
// accumulating observations without concluding
// ---------------------------------------------------------------------------

/// What the matches below write into.
///
/// [`HasilatFahs`] carries one engine family per detector, and this detector
/// covers nine of them, so a game that is genuinely two things at once — a
/// Godot build inside an Electron wrapper, an RPG Maker MZ export that is also
/// NW.js — would otherwise silently lose whichever family was recorded second.
///
/// So family claims accumulate here with the weight that earned them, every
/// observation is recorded as evidence with its engine named in plain words,
/// and only at the end does the strongest claim become the single
/// [`HasilatFahs::aila`]. Nothing is averaged and nothing is dropped:
/// [`crate::tahdid`] sees the losing claim in the evidence and can act on it.
#[derive(Debug, Default)]
struct Hasad {
    /// What is returned.
    hasila: HasilatFahs,
    /// Every family claimed, with the weight behind it, in the order claimed.
    ailat: Vec<(AilatMuharrik, u8)>,
    /// Every scripting backend claimed, with the weight behind it.
    ///
    /// Tracked apart from the family rather than carried on it, because the two
    /// are established by different files at different strengths. `Foo_Data`
    /// beside `Foo.exe` proves Unity at 95 and says nothing about the backend;
    /// `global-metadata.dat` proves IL2CPP. Tying the backend to whichever
    /// family claim happened to win would throw the second fact away.
    khalfiyat: Vec<(KhalfiyaBarmajiya, u8)>,
    /// Discovery already named the game's executable, so nothing here proposes
    /// another one. Ranking executables is discovery's job and it does it once.
    tanfidhi_maruf: bool,
}

impl Hasad {
    /// Records an observation that names no engine.
    fn sajjil(&mut self, wasf: impl Into<String>, mawqi: Option<String>, wazn: u8) {
        self.hasila.sajjil(NawDaleel::BinyatMujallad, wasf, mawqi, wazn);
    }

    /// Records an observation and the engine family it points at.
    fn aila(
        &mut self,
        aila: AilatMuharrik,
        khalfiya: Option<KhalfiyaBarmajiya>,
        wasf: impl Into<String>,
        mawqi: Option<String>,
        wazn: u8,
    ) {
        self.sajjil(wasf, mawqi, wazn);
        self.ailat.push((aila, wazn.min(100)));
        if let Some(khalfiya) = khalfiya {
            self.khalfiyat.push((khalfiya, wazn.min(100)));
        }
    }

    /// Names the executable an engine's own naming rule proved, unless
    /// discovery already named one.
    fn tanfidhi(&mut self, masar: &Path) {
        if !self.tanfidhi_maruf && self.hasila.tanfidhi.is_none() {
            self.hasila.tanfidhi = Some(masar.to_path_buf());
        }
    }

    /// Adds a text system this shape guarantees is present.
    fn itar(&mut self, itar: ItarNusus) {
        self.hasila.daa_itar(itar);
    }

    /// Settles on the strongest family and backend claims and returns the
    /// result.
    ///
    /// Strictly greater wins, so a tie leaves the earlier claim standing and
    /// the same directory always produces the same answer.
    fn ikhtim(mut self) -> HasilatFahs {
        self.hasila.aila = aqwa(&self.ailat);
        self.hasila.khalfiya = aqwa(&self.khalfiyat);
        self.hasila
    }
}

/// The strongest claim in a list, or `None` when nothing was claimed.
fn aqwa<T: Copy>(murashahat: &[(T, u8)]) -> Option<T> {
    let mut afdal: Option<(T, u8)> = None;
    for murashah in murashahat {
        if afdal.is_none_or(|(_, wazn)| murashah.1 > wazn) {
            afdal = Some(*murashah);
        }
    }
    afdal.map(|(qeema, _)| qeema)
}

/// Confirms one path below an entry the walk already found.
///
/// Not a second walk and not a search: a single existence check on a path built
/// from a path the walk itself produced plus literal components written in this
/// file. It exists for the markers whose *name* is derivable from something
/// already seen but whose *depth* is past where the walk stops — the macOS
/// Unity layout being the whole of the case.
fn tahta(asas: &Path, ajzaa: &[&str]) -> Option<PathBuf> {
    let mut masar = asas.to_path_buf();
    for juz in ajzaa {
        masar.push(juz);
    }
    masar.exists().then_some(masar)
}

/// Whether a directory of this name earns its subtree an extra level.
///
/// The named list plus every macOS application bundle, because a bundle's name
/// is the game's name and only its `.app` suffix is fixed.
fn mujallad_ameeq(ism: &str) -> bool {
    MUJALLADAT_AMEEQA.contains(&ism) || imtidad(ism, "app")
}

/// Whether a file name carries an extension a game executable ever has.
///
/// A name with no extension at all passes, because that is what a Linux or
/// macOS binary usually looks like.
fn imtidad_tanfidhi(ism: &str) -> bool {
    match ism.rsplit_once('.') {
        Some((_, imtidad)) => IMTIDADAT_TANFIDH.contains(&imtidad),
        None => true,
    }
}

/// Whether this entry is an executable called `jidhr_ism`, under any of the
/// suffixes an engine's name binding resolves to.
///
/// A `.app` bundle counts even though it is a directory: on macOS the bundle
/// *is* the executable as far as every naming rule is concerned.
fn tanfidhi_bism(madkhal: &Madkhal, jidhr_ism: &str) -> bool {
    let Some(baqi) = madkhal.ism.strip_prefix(jidhr_ism) else {
        return false;
    };
    let imtidad = baqi.strip_prefix('.').unwrap_or(baqi);
    if !baqi.is_empty() && imtidad == baqi {
        return false;
    }
    if madkhal.mujallad && imtidad != "app" {
        return false;
    }
    IMTIDADAT_TANFIDH.contains(&imtidad)
}

// ---------------------------------------------------------------------------
// Unity
// ---------------------------------------------------------------------------

/// `Foo_Data/` standing beside an executable called `Foo`.
///
/// The strongest shape in the whole file, and the reason is that it is not a
/// correlation: Unity's player derives the name of its data directory from its
/// own file name and fails to start if the derived name is not there. Seeing
/// both halves is seeing the engine's own rule satisfied.
const WAZN_RIBAT_UNITY: u8 = 95;

/// A `*_Data/` directory with no executable of the matching name beside it.
///
/// Still Unity — nothing else ships that suffix — but the binding is broken,
/// which usually means the player was renamed by a repacker or the executable
/// sits outside the walk. Worth less because the name it would have proved is
/// exactly what is missing.
const WAZN_BAYANAT_UNITY: u8 = 70;

/// `Assembly-CSharp.dll`, `global-metadata.dat`, and the other files that exist
/// in Unity builds and nowhere else.
const WAZN_UNITY_QATI: u8 = 95;

/// `UnityPlayer.dll` and its Linux and macOS spellings.
const WAZN_MUSHAGHGHIL_UNITY: u8 = 90;

/// `globalgamemanagers` or `data.unity3d` inside the data directory.
const WAZN_JIDHR_UNITY: u8 = 90;

/// The player runtime, under every name the three platforms give it.
const ASMAA_MUSHAGHGHIL_UNITY: &[&str] =
    &["unityplayer.dll", "unityplayer.so", "libunityplayer.so", "unityplayer.dylib"];

/// The IL2CPP runtime, under every name the three platforms give it.
const ASMAA_IL2CPP: &[&str] =
    &["gameassembly.dll", "libgameassembly.so", "gameassembly.dylib"];

/// Managed assemblies whose presence names a text system Taarib can take over.
///
/// NGUI is deliberately absent: it is distributed as source and compiles into
/// `Assembly-CSharp.dll`, so there is no file to find and the adapter resolves
/// it by type at runtime instead.
const MAKTABAT_NUSUS: &[(&str, ItarNusus)] = &[
    ("unity.textmeshpro.dll", ItarNusus::TextMeshPro),
    ("textmeshpro.dll", ItarNusus::TextMeshPro),
    ("unityengine.ui.dll", ItarNusus::UnityUiText),
    ("unityengine.uimodule.dll", ItarNusus::UnityUiText),
    ("fairygui.dll", ItarNusus::FairyGui),
    ("unityengine.textrenderingmodule.dll", ItarNusus::TextMesh),
];

/// `GameAssembly.dll` beside the game, before the metadata file confirms it.
const WAZN_IL2CPP: u8 = 92;

/// An `il2cpp_data/` directory whose metadata file was not reachable.
const WAZN_IL2CPP_NAQIS: u8 = 88;

/// A `Managed/` directory holding assemblies, before any one is named.
const WAZN_MANAGED: u8 = 80;

/// Matches every Unity shape, in both backends and on all three platforms.
fn unity(siyaq: &SiyaqFahs<'_>, mashhad: &Mashhad, hasad: &mut Hasad) {
    // The data directories found, each as the relative path evidence will name
    // and the absolute path the leaf checks below build on.
    let mut mujalladat: Vec<(String, PathBuf)> = Vec::new();

    // 1 — the name binding, read from the data directory towards the
    //     executable, which is the direction that produces a fact discovery
    //     did not have.
    for madkhal in mashhad.kul().filter(|madkhal| madkhal.mujallad) {
        let Some(jidhr_ism) = madkhal.bidun_lahiqa("_data") else {
            continue;
        };
        if jidhr_ism.is_empty() {
            continue;
        }
        mujalladat.push((madkhal.nisbi.clone(), madkhal.masar.clone()));

        let qarin =
            mashhad.kul().find(|akhar| akhar.umq == madkhal.umq && tanfidhi_bism(akhar, jidhr_ism));
        match qarin {
            Some(tanfidhi) => {
                hasad.aila(
                    AilatMuharrik::Unity,
                    None,
                    format!(
                        "Unity: the data directory {} is named after the executable {}, which is \
                         the player's own rule for finding it",
                        madkhal.nisbi, tanfidhi.nisbi
                    ),
                    Some(madkhal.nisbi.clone()),
                    WAZN_RIBAT_UNITY,
                );
                hasad.tanfidhi(&tanfidhi.masar);
            },
            None => hasad.aila(
                AilatMuharrik::Unity,
                None,
                format!(
                    "Unity: the data directory {} is present, but no executable called {} sits \
                     beside it",
                    madkhal.nisbi, jidhr_ism
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_BAYANAT_UNITY,
            ),
        }
    }

    // 2 — the same binding read from the other end, for the one case the walk
    //     cannot cover: discovery named the executable and the walk ran out of
    //     entries before it reached the directory named after it.
    if mujalladat.is_empty()
        && let Some(tanfidhi) = siyaq.tanfidhi
        && let Some(jidhr_ism) = tanfidhi.file_stem()
        && let Some(abb) = tanfidhi.parent()
    {
        let ism = format!("{}_Data", jidhr_ism.to_string_lossy());
        if let Some(masar) = tahta(abb, &[ism.as_str()]) {
            hasad.aila(
                AilatMuharrik::Unity,
                None,
                format!("Unity: {ism} sits beside the executable discovery named"),
                Some(ism.clone()),
                WAZN_RIBAT_UNITY,
            );
            mujalladat.push((ism, masar));
        }
    }

    // 3 — the macOS bundle carries the same binding under a different
    //     spelling: `Contents/Resources/Data` beside `Contents/MacOS/<name>`.
    for madkhal in mashhad.kul().filter(|madkhal| madkhal.mujallad && madkhal.ism == "data") {
        if !madkhal.fi("resources") {
            continue;
        }
        mujalladat.push((madkhal.nisbi.clone(), madkhal.masar.clone()));
        let hazma = madkhal.nisbi.split('/').next().unwrap_or_default().to_ascii_lowercase();
        let jidhr_ism = hazma.strip_suffix(".app").unwrap_or(&hazma).to_owned();
        hasad.aila(
            AilatMuharrik::Unity,
            None,
            format!("Unity: the macOS bundle {} holds the player's data directory", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RIBAT_UNITY,
        );
        if let Some(tanfidhi) =
            mashhad.kul().find(|akhar| akhar.fi("macos") && tanfidhi_bism(akhar, &jidhr_ism))
        {
            hasad.tanfidhi(&tanfidhi.masar);
        }
    }

    unity_bayanat(&mujalladat, hasad);
    unity_mushaghghil(mashhad, hasad);
    unity_managed(mashhad, hasad);
}

/// Reads the files inside each data directory that name the scripting backend.
///
/// Every check here is one existence test on a path built from a directory the
/// walk already produced plus component names Unity itself fixes. Doing it this
/// way rather than by looking the paths up in the walk is what makes the macOS
/// bundle layout — where the same files sit three levels deeper — cost exactly
/// the same as the Windows one.
fn unity_bayanat(mujalladat: &[(String, PathBuf)], hasad: &mut Hasad) {
    for (nisbi, masar) in mujalladat {
        for ism in ["globalgamemanagers", "data.unity3d"] {
            if tahta(masar, &[ism]).is_some() {
                hasad.aila(
                    AilatMuharrik::Unity,
                    None,
                    format!("Unity: {nisbi}/{ism} is the player's own serialized root"),
                    Some(format!("{nisbi}/{ism}")),
                    WAZN_JIDHR_UNITY,
                );
            }
        }

        if tahta(masar, &["Managed", "Assembly-CSharp.dll"]).is_some() {
            hasad.aila(
                AilatMuharrik::Unity,
                Some(KhalfiyaBarmajiya::Mono),
                format!(
                    "Unity on Mono: {nisbi}/Managed/Assembly-CSharp.dll is the game's own \
                     compiled script assembly"
                ),
                Some(format!("{nisbi}/Managed/Assembly-CSharp.dll")),
                WAZN_UNITY_QATI,
            );
        } else if tahta(masar, &["Managed"]).is_some() {
            hasad.aila(
                AilatMuharrik::Unity,
                Some(KhalfiyaBarmajiya::Mono),
                format!("Unity on Mono: {nisbi}/Managed holds managed assemblies"),
                Some(format!("{nisbi}/Managed")),
                WAZN_MANAGED,
            );
        }

        if tahta(masar, &["il2cpp_data", "Metadata", "global-metadata.dat"]).is_some() {
            hasad.aila(
                AilatMuharrik::Unity,
                Some(KhalfiyaBarmajiya::Il2cpp),
                format!(
                    "Unity on IL2CPP: {nisbi}/il2cpp_data/Metadata/global-metadata.dat is the \
                     ahead-of-time metadata the native build reads"
                ),
                Some(format!("{nisbi}/il2cpp_data/Metadata/global-metadata.dat")),
                WAZN_UNITY_QATI,
            );
        } else if tahta(masar, &["il2cpp_data"]).is_some() {
            hasad.aila(
                AilatMuharrik::Unity,
                Some(KhalfiyaBarmajiya::Il2cpp),
                format!("Unity on IL2CPP: {nisbi}/il2cpp_data is present"),
                Some(format!("{nisbi}/il2cpp_data")),
                WAZN_IL2CPP_NAQIS,
            );
        }
    }
}

/// Finds the player runtime and the IL2CPP runtime by file name.
fn unity_mushaghghil(mashhad: &Mashhad, hasad: &mut Hasad) {
    for ism in ASMAA_MUSHAGHGHIL_UNITY {
        if let Some(madkhal) = mashhad.bism_ayn(ism) {
            hasad.aila(
                AilatMuharrik::Unity,
                None,
                format!("Unity: {} is the player runtime itself", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_MUSHAGHGHIL_UNITY,
            );
        }
    }
    for ism in ASMAA_IL2CPP {
        if let Some(madkhal) = mashhad.bism_ayn(ism) {
            hasad.aila(
                AilatMuharrik::Unity,
                Some(KhalfiyaBarmajiya::Il2cpp),
                format!(
                    "Unity on IL2CPP: {} is the game's managed code compiled to a native binary",
                    madkhal.nisbi
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_IL2CPP,
            );
        }
    }
}

/// Names the text systems a Mono build's assemblies prove are present.
///
/// Only for the layouts where `Managed/` falls inside the walk, which is every
/// Windows and Linux build and not the macOS bundle; on macOS the same answer
/// comes from the adapter resolving the types at runtime, so nothing is lost
/// beyond a line in the capability report.
fn unity_managed(mashhad: &Mashhad, hasad: &mut Hasad) {
    for madkhal in mashhad.kul().filter(|madkhal| madkhal.fi("managed")) {
        for (ism, itar) in MAKTABAT_NUSUS {
            if madkhal.ism == *ism {
                hasad.itar(*itar);
                hasad.sajjil(
                    format!("Unity: {} is present, so {} is reachable", madkhal.nisbi, ism),
                    Some(madkhal.nisbi.clone()),
                    WAZN_MANAGED,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unreal
// ---------------------------------------------------------------------------

/// `<Project>/Binaries/Win64/<Project>-Win64-Shipping.exe` and its siblings on
/// the other platforms.
///
/// A shipping binary's name is produced by the build tool from the target name,
/// the platform and the configuration; nothing but Unreal writes it, and it
/// names the executable the launcher's own shortcut points at.
const WAZN_SHIPPING: u8 = 92;

/// UE5's IoStore container pair.
const WAZN_IOSTORE: u8 = 88;

/// A mounted `.pak` under `Content/Paks`.
const WAZN_PAK: u8 = 85;

/// `Engine/Binaries/`, which every shipped Unreal build carries.
const WAZN_MUJALLAD_UNREAL: u8 = 85;

/// `Content/Paks/` with nothing in it the walk could name.
const WAZN_PAKS_FARIGH: u8 = 82;

/// A `.uproject` file, which means the tree is a project rather than a build.
const WAZN_UPROJECT: u8 = 80;

/// `Manifest_UFSFiles_Win64.txt` and the rest of the staging manifests.
const WAZN_BAYAN_UNREAL: u8 = 70;

/// Matches the Unreal shapes, UE4 and UE5 alike.
///
/// No version is claimed here. UE4 and UE5 differ in this layout only by the
/// presence of IoStore containers, and a UE5 title can still ship plain `.pak`
/// files, so the version comes from the executable's own `FEngineVersion` tag
/// and from the container headers, both of which are somebody else's evidence
/// source.
fn unreal(mashhad: &Mashhad, hasad: &mut Hasad) {
    let mut wujid = false;

    if let Some(madkhal) = mashhad.mujallad_fi("engine", "binaries") {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Unreal,
            Some(KhalfiyaBarmajiya::UnrealNative),
            format!("Unreal: {} is the engine's own binary tree", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_MUJALLAD_UNREAL,
        );
    }

    for madkhal in mashhad.kul().filter(|madkhal| !madkhal.mujallad) {
        let jidhr_ism = madkhal.ism.strip_suffix(".exe").unwrap_or(&madkhal.ism);
        if !jidhr_ism.ends_with("-shipping") {
            continue;
        }
        wujid = true;
        hasad.aila(
            AilatMuharrik::Unreal,
            Some(KhalfiyaBarmajiya::UnrealNative),
            format!(
                "Unreal: {} is a shipping build produced by the Unreal build tool",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            WAZN_SHIPPING,
        );
        hasad.tanfidhi(&madkhal.masar);
    }

    for madkhal in mashhad.kul() {
        if madkhal.mujallad {
            if madkhal.ism == "paks" && madkhal.fi("content") {
                wujid = true;
                hasad.aila(
                    AilatMuharrik::Unreal,
                    Some(KhalfiyaBarmajiya::UnrealNative),
                    format!("Unreal: {} is where a build mounts its content", madkhal.nisbi),
                    Some(madkhal.nisbi.clone()),
                    WAZN_PAKS_FARIGH,
                );
            }
            continue;
        }

        if imtidad(&madkhal.ism, "utoc") || imtidad(&madkhal.ism, "ucas") {
            wujid = true;
            hasad.aila(
                AilatMuharrik::Unreal,
                Some(KhalfiyaBarmajiya::UnrealNative),
                format!(
                    "Unreal: {} belongs to an IoStore container pair, which is how UE5 packages \
                     content",
                    madkhal.nisbi
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_IOSTORE,
            );
        } else if imtidad(&madkhal.ism, "pak") && madkhal.fi("paks") {
            wujid = true;
            hasad.aila(
                AilatMuharrik::Unreal,
                Some(KhalfiyaBarmajiya::UnrealNative),
                format!("Unreal: {} is a mounted content archive", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_PAK,
            );
        } else if imtidad(&madkhal.ism, "uproject") {
            wujid = true;
            hasad.aila(
                AilatMuharrik::Unreal,
                Some(KhalfiyaBarmajiya::UnrealNative),
                format!(
                    "Unreal: {} describes a project rather than a packaged build",
                    madkhal.nisbi
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_UPROJECT,
            );
        } else if madkhal.ism.starts_with("manifest_") && imtidad(&madkhal.ism, "txt") {
            wujid = true;
            hasad.aila(
                AilatMuharrik::Unreal,
                Some(KhalfiyaBarmajiya::UnrealNative),
                format!(
                    "Unreal: {} is a staging manifest left by the packaging step",
                    madkhal.nisbi
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_BAYAN_UNREAL,
            );
        }
    }

    if wujid {
        // Slate is not optional in Unreal. Every widget, every menu and every
        // subtitle goes through it, which is why the Unreal adapter corrects
        // the engine's own shaping rather than taking drawing over.
        hasad.itar(ItarNusus::Slate);
    }
}

// ---------------------------------------------------------------------------
// Godot
// ---------------------------------------------------------------------------

/// A `.pck` whose name matches an executable beside it.
///
/// Godot's binary looks for a pack named after itself before it looks anywhere
/// else, so this is the same kind of naming rule as Unity's, and it names the
/// executable the same way.
const WAZN_RIBAT_GODOT: u8 = 92;

/// A `.pck` that no executable beside it is named after.
const WAZN_PCK: u8 = 85;

/// `.godot/` or `project.godot` — a project tree rather than an export.
const WAZN_MASHRU_GODOT: u8 = 90;

/// `Foo.console.exe` beside `Foo.exe`.
const WAZN_GODOT_TARAF: u8 = 65;

/// A `data_<name>_<platform>_<arch>/` directory beside the executable.
const WAZN_GODOT_SHARP: u8 = 55;

/// Matches the Godot shapes, both exported and unexported.
///
/// The engine's major version is not claimed. Godot 3 and Godot 4 produce the
/// same directory shape and differ in the pack header version, which is the
/// container detector's evidence, not this one's — and the difference decides
/// tier 1 against tier 2, so guessing it here would be guessing at the answer
/// the user is shown.
fn godot(mashhad: &Mashhad, hasad: &mut Hasad) {
    for madkhal in mashhad.kul().filter(|madkhal| !madkhal.mujallad) {
        let Some(jidhr_ism) = madkhal.ism.strip_suffix(".pck") else {
            continue;
        };
        let qarin =
            mashhad.kul().find(|akhar| akhar.umq == madkhal.umq && tanfidhi_bism(akhar, jidhr_ism));
        match qarin {
            Some(tanfidhi) => {
                hasad.aila(
                    AilatMuharrik::Godot,
                    None,
                    format!(
                        "Godot: the pack {} is named after the executable {}, which is how the \
                         binary finds it",
                        madkhal.nisbi, tanfidhi.nisbi
                    ),
                    Some(madkhal.nisbi.clone()),
                    WAZN_RIBAT_GODOT,
                );
                hasad.tanfidhi(&tanfidhi.masar);
            },
            None => hasad.aila(
                AilatMuharrik::Godot,
                None,
                format!("Godot: {} is an engine pack file", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_PCK,
            ),
        }
    }

    for nisbi in [".godot", "project.godot"] {
        if let Some(madkhal) = mashhad.ind(nisbi) {
            hasad.aila(
                AilatMuharrik::Godot,
                Some(KhalfiyaBarmajiya::GdScript),
                format!(
                    "Godot: {} belongs to an unexported project, so the game runs from source \
                     rather than from a pack",
                    madkhal.nisbi
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_MASHRU_GODOT,
            );
        }
    }

    for madkhal in mashhad.kul().filter(|madkhal| !madkhal.mujallad) {
        let Some(jidhr_ism) = madkhal.ism.strip_suffix(".console.exe") else {
            continue;
        };
        if mashhad.kul().any(|akhar| akhar.umq == madkhal.umq && tanfidhi_bism(akhar, jidhr_ism)) {
            hasad.aila(
                AilatMuharrik::Godot,
                None,
                format!(
                    "Godot: {} is the console companion a Godot 4 Windows export writes beside \
                     the game",
                    madkhal.nisbi
                ),
                Some(madkhal.nisbi.clone()),
                WAZN_GODOT_TARAF,
            );
        }
    }

    for madkhal in mashhad.kul() {
        if !madkhal.mujallad || madkhal.umq != 1 || !madkhal.ism.starts_with("data_") {
            continue;
        }
        if !madkhal.ism.contains("_x86") && !madkhal.ism.contains("_arm") {
            continue;
        }
        hasad.aila(
            AilatMuharrik::Godot,
            Some(KhalfiyaBarmajiya::GodotCSharp),
            format!(
                "Godot with C#: {} is the assembly directory a .NET export writes beside the \
                 executable",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            WAZN_GODOT_SHARP,
        );
    }
}

// ---------------------------------------------------------------------------
// RPG Maker
// ---------------------------------------------------------------------------

/// `www/js/rpg_core.js`, `js/rmmz_core.js`, `Game.rgss3a`.
///
/// The editor writes these verbatim into every export and the runtime loads
/// them by that exact name. They also separate the three generations from each
/// other, which matters more than usual here: MV and MZ take different adapters
/// and VX Ace takes a third.
const WAZN_RPG_QATI: u8 = 95;

/// `RGSS300.dll`, the VX Ace interpreter shipped beside the game.
const WAZN_RGSS: u8 = 85;

/// `www/data/System.json` and `data/System.json`.
const WAZN_BAYANAT_RPG: u8 = 88;

/// `js/plugins/`, which is where an MV or MZ export keeps its plugin scripts.
const WAZN_IDAFAT_RPG: u8 = 60;

/// `Game.ini`, whose name is an RPG Maker name and whose proof is inside it.
const WAZN_INI_RPG: u8 = 25;

/// Matches all three RPG Maker generations Taarib covers.
///
/// The `RGSS3` line inside `Game.ini` and the NW.js fields inside
/// `package.json` are both named in the coverage matrix and both deliberately
/// left alone here: this detector does not open files. The shapes it can see
/// are enough to tell the generations apart, and the file contents confirm the
/// version through the embedded-metadata source.
fn rpg_maker(mashhad: &Mashhad, hasad: &mut Hasad) {
    // Matched by the directory that holds them rather than by a path from the
    // root, so that `www/js/rpg_core.js` and the same file inside a macOS
    // bundle's `Contents/Resources/app.nw/js/` are one rule.
    let mv = mashhad.fi("js", "rpg_core.js");
    let mz = mashhad.fi("js", "rmmz_core.js");

    if let Some(madkhal) = mv {
        hasad.aila(
            AilatMuharrik::RpgMakerMv,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("RPG Maker MV: {} is the runtime core the editor exports", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RPG_QATI,
        );
        hasad.itar(ItarNusus::NafidhatRpg);
        hasad.itar(ItarNusus::Canvas);
    }
    if let Some(madkhal) = mz {
        hasad.aila(
            AilatMuharrik::RpgMakerMz,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("RPG Maker MZ: {} is the runtime core the editor exports", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RPG_QATI,
        );
        hasad.itar(ItarNusus::NafidhatRpg);
        hasad.itar(ItarNusus::Canvas);
    }

    // `System.json` is the same file in both generations and sits inside
    // `data/` in both — under `www/` for MV and at the root for MZ — so the
    // core file that fired above is what decides which family it belongs to.
    // With neither core file present it is still an RPG Maker database, and it
    // is reported as MV, the generation that ships the layout it is in.
    if let Some(madkhal) = mashhad.fi("data", "system.json") {
        let aila =
            if mz.is_some() { AilatMuharrik::RpgMakerMz } else { AilatMuharrik::RpgMakerMv };
        hasad.aila(
            aila,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("RPG Maker: {} is the project's own database file", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_BAYANAT_RPG,
        );
    }

    if let Some(madkhal) = mashhad.mujallad_fi("js", "plugins") {
        hasad.sajjil(
            format!("RPG Maker: {} holds the project's plugin scripts", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_IDAFAT_RPG,
        );
    }

    if let Some(madkhal) = mashhad.bism("game.rgss3a") {
        hasad.aila(
            AilatMuharrik::RpgMakerVxAce,
            Some(KhalfiyaBarmajiya::Ruby),
            format!("RPG Maker VX Ace: {} is the RGSS3 archive", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RPG_QATI,
        );
        hasad.itar(ItarNusus::NafidhatRpg);
    }

    if let Some(madkhal) = mashhad.bism("rgss300.dll") {
        hasad.aila(
            AilatMuharrik::RpgMakerVxAce,
            Some(KhalfiyaBarmajiya::Ruby),
            format!("RPG Maker VX Ace: {} is the RGSS3 interpreter", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RGSS,
        );
        hasad.itar(ItarNusus::NafidhatRpg);
    }

    if let Some(madkhal) = mashhad.bism("game.ini") {
        hasad.sajjil(
            format!(
                "{} is named the way RPG Maker names its configuration; whether it declares \
                 RGSS3 is inside the file, not in its name",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            WAZN_INI_RPG,
        );
    }
}

// ---------------------------------------------------------------------------
// Ren'Py
// ---------------------------------------------------------------------------

/// `renpy/__init__.py`, the package the game imports before it does anything.
const WAZN_RENPY_QATI: u8 = 92;

/// A `renpy/` directory, or a `.rpa` archive under `game/`.
const WAZN_RENPY: u8 = 90;

/// `lib/py3-linux-x86_64/` and the other bundled interpreter directories.
const WAZN_LIB_RENPY: u8 = 80;

/// Compiled script objects under `game/`.
const WAZN_RPYC: u8 = 85;

/// Matches the Ren'Py shapes.
fn renpy(mashhad: &Mashhad, hasad: &mut Hasad) {
    if let Some(madkhal) = mashhad.fi("renpy", "__init__.py") {
        hasad.aila(
            AilatMuharrik::Renpy,
            Some(KhalfiyaBarmajiya::Python),
            format!("Ren'Py: {} is the engine package the game imports", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RENPY_QATI,
        );
        hasad.itar(ItarNusus::NassRenpy);
    } else if let Some(madkhal) = mashhad.madakhil.iter().find(|m| m.mujallad && m.ism == "renpy") {
        hasad.aila(
            AilatMuharrik::Renpy,
            Some(KhalfiyaBarmajiya::Python),
            format!("Ren'Py: {} is the engine's own directory", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RENPY,
        );
        hasad.itar(ItarNusus::NassRenpy);
    }

    for madkhal in mashhad.kul().filter(|madkhal| !madkhal.mujallad && madkhal.fi("game")) {
        if imtidad(&madkhal.ism, "rpa") {
            hasad.aila(
                AilatMuharrik::Renpy,
                Some(KhalfiyaBarmajiya::Python),
                format!("Ren'Py: {} is an engine archive", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_RENPY,
            );
            hasad.itar(ItarNusus::NassRenpy);
        } else if imtidad(&madkhal.ism, "rpyc") {
            hasad.aila(
                AilatMuharrik::Renpy,
                Some(KhalfiyaBarmajiya::Python),
                format!("Ren'Py: {} is a compiled script", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_RPYC,
            );
            hasad.itar(ItarNusus::NassRenpy);
        }
    }

    // The interpreter directory is named for the Python generation and the
    // target: `py3-linux-x86_64` on Ren'Py 8, `py2-windows-i686` on Ren'Py 7,
    // and a bare `windows-i686` on the releases between them.
    for madkhal in mashhad.kul() {
        if !madkhal.mujallad || !madkhal.fi("lib") {
            continue;
        }
        let mustahdaf = madkhal.ism.starts_with("py2-")
            || madkhal.ism.starts_with("py3-")
            || madkhal.ism.starts_with("windows-")
            || madkhal.ism.starts_with("linux-")
            || madkhal.ism.starts_with("mac-")
            || madkhal.ism.starts_with("darwin-");
        if !mustahdaf {
            continue;
        }
        hasad.aila(
            AilatMuharrik::Renpy,
            Some(KhalfiyaBarmajiya::Python),
            format!("Ren'Py: {} is the bundled interpreter for one target", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_LIB_RENPY,
        );
        hasad.itar(ItarNusus::NassRenpy);
    }
}

// ---------------------------------------------------------------------------
// GameMaker
// ---------------------------------------------------------------------------

/// `data.win` and `game.unx` — the FORM container under its platform name.
const WAZN_GAMEMAKER: u8 = 92;

/// `game.ios`, the same container on Apple platforms.
const WAZN_GAMEMAKER_IOS: u8 = 88;

/// `audiogroup1.dat` and the rest of the split audio groups.
const WAZN_AUDIOGROUP: u8 = 70;

/// The FORM container, under each name a GameMaker export gives it.
const ASMAA_GAMEMAKER: &[(&str, u8)] =
    &[("data.win", WAZN_GAMEMAKER), ("game.unx", WAZN_GAMEMAKER), ("game.ios", WAZN_GAMEMAKER_IOS)];

/// Matches the GameMaker shapes.
fn game_maker(mashhad: &Mashhad, hasad: &mut Hasad) {
    let mut wujid = false;
    for (ism, wazn) in ASMAA_GAMEMAKER {
        let Some(madkhal) = mashhad.bism_ayn(ism) else {
            continue;
        };
        wujid = true;
        hasad.aila(
            AilatMuharrik::GameMaker,
            Some(KhalfiyaBarmajiya::GameMakerVm),
            format!(
                "GameMaker: {} is the FORM container holding the game's rooms, sprites and \
                 bytecode",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            *wazn,
        );
    }

    for madkhal in mashhad.kul() {
        if madkhal.mujallad
            || !madkhal.ism.starts_with("audiogroup")
            || !imtidad(&madkhal.ism, "dat")
        {
            continue;
        }
        wujid = true;
        hasad.aila(
            AilatMuharrik::GameMaker,
            Some(KhalfiyaBarmajiya::GameMakerVm),
            format!("GameMaker: {} is a split audio group", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_AUDIOGROUP,
        );
    }

    if wujid {
        hasad.itar(ItarNusus::RasmGameMaker);
    }
}

// ---------------------------------------------------------------------------
// Electron and NW.js
// ---------------------------------------------------------------------------

/// `resources/app.asar` — Electron's own archive format, used by nothing else.
const WAZN_ASAR: u8 = 92;

/// `nw.dll` and its Unix spellings: NW.js rather than plain Electron.
const WAZN_NW: u8 = 90;

/// `nw.pak`, which NW.js ships and Electron does not.
const WAZN_NW_PAK: u8 = 88;

/// `chrome_100_percent.pak` and the rest of the Chromium resource packs.
const WAZN_CHROMIUM: u8 = 85;

/// `resources/app/package.json` — the same application, left unpacked.
const WAZN_ASAR_MAFTUH: u8 = 85;

/// `LICENSES.chromium.html`, which every Chromium embedder ships.
const WAZN_RUKHSA_CHROMIUM: u8 = 70;

/// `icudtl.dat`: Chromium, but also every other program embedding ICU.
const WAZN_ICU: u8 = 60;

/// A `package.json` at the root of something that also looks like a web app.
const WAZN_PACKAGE_JSON: u8 = 60;

/// `v8_context_snapshot.bin` and `snapshot_blob.bin`.
const WAZN_LAQTA_V8: u8 = 55;

/// The NW.js runtime library, under each platform's spelling.
const ASMAA_NW: &[&str] = &["nw.dll", "libnw.so", "nw.dylib"];

/// Chromium resource packs, which are the same file under two names.
const ASMAA_CHROMIUM: &[&str] = &["chrome_100_percent.pak", "chrome_200_percent.pak"];

/// Snapshot blobs, which every Chromium embedder writes beside its binary.
const ASMAA_LAQTA: &[&str] = &["v8_context_snapshot.bin", "snapshot_blob.bin"];

/// Matches the Electron and NW.js shapes.
///
/// These fire *alongside* another engine rather than instead of it, and that is
/// correct: an RPG Maker MZ desktop export is JavaScript inside NW.js inside a
/// Windows binary, and all three of those statements are true at once. The
/// evidence records each one and [`crate::tahdid`] decides which one the
/// adapter is dispatched on.
fn chromium(mashhad: &Mashhad, hasad: &mut Hasad) {
    let mut wujid = false;

    if let Some(madkhal) = mashhad.fi("resources", "app.asar") {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Electron,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("Electron: {} is the application archive the runtime mounts", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_ASAR,
        );
    } else if let Some(madkhal) = mashhad.fi("app", "package.json") {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Electron,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("Electron: {} is an unpacked application directory", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_ASAR_MAFTUH,
        );
    }

    for ism in ASMAA_NW {
        if let Some(madkhal) = mashhad.bism_ayn(ism) {
            wujid = true;
            hasad.aila(
                AilatMuharrik::Electron,
                Some(KhalfiyaBarmajiya::JavaScript),
                format!("NW.js: {} is the NW.js runtime library", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_NW,
            );
        }
    }

    if let Some(madkhal) = mashhad.bism_ayn("nw.pak") {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Electron,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("NW.js: {} is NW.js's own resource pack", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_NW_PAK,
        );
    }

    if let Some(madkhal) = mashhad.bi_ahad(ASMAA_CHROMIUM) {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Electron,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("Chromium: {} is a browser resource pack", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_CHROMIUM,
        );
    }

    if let Some(madkhal) = mashhad.bism_ayn("licenses.chromium.html") {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Electron,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!("Chromium: {} ships with every Chromium embedder", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_RUKHSA_CHROMIUM,
        );
    }

    if let Some(madkhal) = mashhad.bism_ayn("icudtl.dat") {
        hasad.sajjil(
            format!(
                "{} is Chromium's ICU data, which also ships with anything else embedding ICU",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            WAZN_ICU,
        );
    }

    if let Some(madkhal) = mashhad.bi_ahad(ASMAA_LAQTA) {
        hasad.sajjil(
            format!("{} is a V8 startup snapshot", madkhal.nisbi),
            Some(madkhal.nisbi.clone()),
            WAZN_LAQTA_V8,
        );
    }

    // A bare `package.json` says nothing on its own — it is the most common
    // file name in software. Beside `www/` or an unpacked `resources/app/` it
    // is the manifest an NW.js or Electron runtime opens at startup, and the
    // fields inside it that name the runtime are the metadata source's to read.
    if let Some(madkhal) = mashhad.bism("package.json")
        && (mashhad.mujallad("www").is_some()
            || mashhad.mujallad_fi("resources", "app").is_some())
    {
        wujid = true;
        hasad.aila(
            AilatMuharrik::Electron,
            Some(KhalfiyaBarmajiya::JavaScript),
            format!(
                "{} sits beside a web application directory, which is the shape a desktop \
                 JavaScript runtime loads",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            WAZN_PACKAGE_JSON,
        );
    }

    if wujid {
        hasad.itar(ItarNusus::Dom);
    }
}

// ---------------------------------------------------------------------------
// graphics, of which a directory listing knows almost nothing
// ---------------------------------------------------------------------------

/// The Direct3D 12 Agility SDK redistributable a game ships to pin its runtime.
const WAZN_AGILITY: u8 = 40;

/// A Vulkan loader shipped inside the game tree.
const WAZN_VULKAN_MUJAWIR: u8 = 35;

/// Reports the two graphics facts a directory shape can actually establish.
///
/// Almost nothing belongs here, and the omissions are deliberate. A `d3d11.dll`
/// or a `dxgi.dll` sitting beside a game is nearly always a proxy library
/// installed by a mod, an overlay or a frame generator — it is evidence about
/// what the *user* installed, not about what the game links, and reading it as
/// a graphics API would put a wrong line in the capability report. The
/// authoritative answer comes from the executable's import table, which is the
/// binary detector's evidence.
fn rusum(mashhad: &Mashhad, hasad: &mut Hasad) {
    if let Some(madkhal) = mashhad.bism_ayn("d3d12core.dll") {
        hasad.hasila.daa_rusum(WajihaRusum::D3d12);
        hasad.sajjil(
            format!(
                "{} is the Direct3D 12 Agility SDK runtime, which a game ships only when it \
                 renders with Direct3D 12",
                madkhal.nisbi
            ),
            Some(madkhal.nisbi.clone()),
            WAZN_AGILITY,
        );
    }

    for ism in ["libvulkan.so.1", "libvulkan.so", "libmoltenvk.dylib"] {
        if let Some(madkhal) = mashhad.bism_ayn(ism) {
            hasad.hasila.daa_rusum(WajihaRusum::Vulkan);
            hasad.sajjil(
                format!("{} is a Vulkan loader shipped inside the game", madkhal.nisbi),
                Some(madkhal.nisbi.clone()),
                WAZN_VULKAN_MUJAWIR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// the game that is one file
// ---------------------------------------------------------------------------

/// How many entries a game directory may hold and still be called a lone
/// executable.
///
/// An export with its pack appended leaves the executable and very little else:
/// a console companion on Windows, a saved-games folder the first run creates,
/// a readme somebody added. Eight is generous enough to survive all of that and
/// tight enough that a real game tree never qualifies.
const AQSA_MUFRAD: usize = 8;

/// A directory shape that is *consistent with* an appended pack, and proves
/// nothing.
const WAZN_MUFRAD: u8 = 20;

/// Notices the shape a self-contained export leaves behind.
///
/// Godot can append its `.pck` to the end of the executable, and several other
/// toolchains embed their content the same way. When they do, there is no
/// separate file to find and the directory shape collapses to a single binary —
/// which is not evidence of an engine, because a directory holding one
/// executable is also what a tiny native game looks like.
///
/// So this records the *possibility* structurally, with no family claimed, and
/// stops there. Confirming it means opening the executable and reading the last
/// twelve bytes for a pack footer, which [`crate::dalail::thunai`] does, and
/// then reading the pack's own header for the version that separates Godot 3
/// from Godot 4, which the container-header source does. Recording the shape at
/// all is what tells the resolver to keep looking rather than settle for an
/// unknown engine.
fn mufrad(mashhad: &Mashhad, hasad: &mut Hasad) {
    if !hasad.ailat.is_empty() || mashhad.mabtur {
        return;
    }
    let judhur: Vec<&Madkhal> = mashhad.kul().filter(|madkhal| madkhal.umq == 1).collect();
    if judhur.len() > AQSA_MUFRAD {
        return;
    }
    let Some(tanfidhi) = judhur
        .iter()
        .find(|madkhal| !madkhal.mujallad && imtidad_tanfidhi(&madkhal.ism))
    else {
        return;
    };

    hasad.sajjil(
        format!(
            "the game directory holds {} and little else, which is the shape an export leaves \
             when its content pack is appended to the executable rather than shipped beside it; \
             the executable's own footer decides whether it is",
            tanfidhi.nisbi
        ),
        Some(tanfidhi.nisbi.clone()),
        WAZN_MUFRAD,
    );
}
