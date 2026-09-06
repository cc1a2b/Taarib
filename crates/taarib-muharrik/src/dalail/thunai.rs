//! توقيع ثنائي — what a game's own binaries say about the engine that built
//! them.
//!
//! Directory shape is what an engine leaves *around* itself. This is what it
//! leaves *inside* itself, and it is harder to fake: a renamed executable still
//! imports `UnityPlayer.dll`, a repackaged game still carries the engine
//! version string its linker embedded, and a stripped build still names its
//! sections.
//!
//! Four kinds of reading, in descending order of how much they are worth:
//!
//! 1. **Imported modules.** The strongest signal a binary carries. An import is
//!    a hard dependency resolved by the loader before the process runs; a
//!    Windows binary importing `UnityPlayer.dll` will not start without it. It
//!    is also where the graphics API comes from, which nothing else in this
//!    crate can establish.
//! 2. **Exported symbols.** `GameAssembly.dll` exports `il2cpp_init`;
//!    `UnityPlayer.dll` exports `UnityMain`; a Mono runtime exports
//!    `mono_jit_init_version`. Each of those names one runtime and nothing
//!    else.
//! 3. **Embedded version strings.** Unreal writes `++UE4+Release-4.27` and
//!    `++UE5+Release-5.3` style branch tags, Unity writes `2021.3.16f1`, Godot
//!    writes `Godot Engine v4.2.1.stable.official`. These give the *version*,
//!    which is what a signature database keys on and what decides whether an
//!    adapter has a code path for the build.
//! 4. **Section names**, which are distinctive in exactly one situation: the
//!    executable is packed or protected. `UPX0`, `.vmp0`, `.themida` and their
//!    relatives are not evidence about the engine, they are the explanation for
//!    why there is no evidence about the engine, and saying so in the
//!    diagnostics is worth more than saying nothing.
//!
//! ## Reading the file: mapped, not read
//!
//! A shipped game executable is routinely 100 MB and occasionally 300; an
//! IL2CPP `GameAssembly.dll` can pass 500. Reading one into a `Vec<u8>` for
//! every game in a library of two hundred is hundreds of gigabytes of pointless
//! I/O and a resident-set spike per game, so anything past [`HADD_KHARITA`] is
//! memory-mapped and only the pages actually inspected are ever touched.
//!
//! **The hazard that choice creates, stated plainly.** A memory map is not a
//! read. If the file's backing store goes away underneath the mapping — a
//! network share drops, an external drive is unplugged mid-scan, another
//! process truncates the file — then touching a page raises `SIGBUS` on Unix or
//! an in-page-error exception on Windows *instead of* returning an error, and
//! Rust has no safe way to catch either. The exposure is bounded rather than
//! eliminated, in five specific ways:
//!
//! - Files below [`HADD_KHARITA`] are read instead of mapped, because for a
//!   small file the read is cheap and it has an error path where a map does
//!   not. Most game binaries that are not the main executable fall here.
//! - The map's lifetime is one function call. It is created, the bounded
//!   windows below are inspected, everything worth keeping is copied out as
//!   owned `String`s, and the map is dropped before the detector returns.
//!   Nothing borrowed from a mapping ever escapes into the result.
//! - Only the pages inside the search windows are touched: the headers, the
//!   import and export tables, the constant-data sections up to
//!   [`NAFIDHAT_BAHTH`], and the last few bytes of the file. A 300 MB
//!   executable costs address space and a few megabytes of faulted pages, so
//!   the window in which a fault is even possible is a fraction of the file.
//! - Every read goes through a bounds-checked slice, so a file that shrank
//!   between the `stat` and the map yields `None` rather than a read past the
//!   end of the mapping.
//! - At most [`AQSA_MULHAQAT`] neighbouring libraries are opened per game, so
//!   the number of live mappings during a probe is a small constant.
//!
//! ## A file that will not parse is not a failure
//!
//! `object` returning an error on a packed, encrypted, protected or simply
//! corrupt executable is the *absence of evidence*, not an error condition. The
//! detector records that the file did not parse — which is itself useful, since
//! it explains a thin report — and returns everything it learned from the other
//! binaries beside it. The same is true of a missing import table, an
//! unreadable section, and a version string that is not there.
//!
//! ## Which executable
//!
//! This detector does not rank executables. Choosing the game's binary out of a
//! folder holding an installer, a launcher, a crash handler and a
//! redistributable is a scored decision that discovery already makes, in
//! `taarib_kashf::matajir::yadawi::rattib_tanfidhiyat`, and a second scoring
//! function here would be a second answer to a question that already has one.
//!
//! So: when discovery named an executable, that is the executable. When it did
//! not, this detector resolves one only from an engine's own naming *rule* —
//! `Foo_Data` implies `Foo`, `Foo.app` implies `Foo.app/Contents/MacOS/Foo` —
//! which is a proof rather than a ranking and agrees with
//! `rattib_tanfidhiyat` by construction, since those are the same two facts it
//! scores highest. When neither applies, the executable is left unnamed and the
//! detector reads the engine libraries beside it instead, which is where most
//! of the evidence lives anyway.

use std::path::{Path, PathBuf};

use object::read::{Object as _, ObjectSection as _};
use taarib_mustalahat::muharrik::{
    AilatMuharrik, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel, WajihaRusum,
};
use taarib_usus::khata::Natija;
use taarib_usus::manassa::Mimariya;

use crate::fahs::{Fahis, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

/// Above this size a binary is memory-mapped; at or below it, it is read.
///
/// Eight megabytes is where the trade turns over. Below it the read costs a few
/// milliseconds and buys a real error path for a file that disappears; above
/// it, reading means copying tens or hundreds of megabytes to look at a few,
/// which is the whole reason `memmap2` is a dependency of this crate.
pub const HADD_KHARITA: u64 = 8 * 1024 * 1024;

/// How many bytes of one binary are searched for embedded version strings.
///
/// The budget is spent only on the sections that hold constant data —
/// [`AQSAM_THAWABIT`] — because that is where a linker puts string literals,
/// and never on code, resources, relocations or debug information. Sixteen
/// megabytes covers those sections *in full* for every Unity player, every
/// Godot binary, every NW.js and Electron build, and every RPG Maker and
/// GameMaker runner: their constant-data sections are single-digit megabytes.
///
/// It does not always cover them in full for a large Unreal shipping binary,
/// whose `.rdata` can run past it. That is an accepted, bounded miss rather
/// than a silent one: the tag is searched from the start of each constant
/// section in link order, the version is *also* available from the packaging
/// manifests and container headers that the other two evidence sources read,
/// and the alternative — scanning 300 MB per game across a whole library — is
/// not a trade this product makes.
pub const NAFIDHAT_BAHTH: usize = 16 * 1024 * 1024;

/// How many neighbouring libraries are opened for one game.
///
/// Four is enough for every layout in the coverage matrix, which never puts
/// more than that many named engine libraries beside a game: a Unity IL2CPP
/// build has `UnityPlayer` and `GameAssembly`, a Mono build has `UnityPlayer`
/// and its Mono runtime, an NW.js build has `nw` and its Chromium libraries.
/// It also bounds how many mappings are live at once.
pub const AQSA_MULHAQAT: usize = 4;

/// How many entries of the game root this detector will look at.
///
/// It needs the root listing for one purpose — resolving an executable from an
/// engine's naming rule — and a game root with more than five hundred entries
/// directly inside it is a game whose executable is not going to be found by
/// looking harder.
const AQSA_MADAKHIL_JIDHR: usize = 512;

/// The largest file this detector will fall back to reading whole in order to
/// answer the architecture question.
///
/// The fallback runs only when `object` refused the file, which in practice
/// means a universal Mach-O binary — the one shape `object::File::parse` will
/// not take and [`taarib_usus::manassa::mimariyat_malaf`] handles explicitly.
/// Past this size the architecture is left unreported rather than paid for with
/// a quarter-gigabyte read.
const HADD_QIRA_IHTIYATI: u64 = 256 * 1024 * 1024;

/// Detection by binary signature.
///
/// Holds no state; every mapping and every parse lives inside one call to
/// [`Fahis::ifhas`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisThunai;

impl FahisThunai {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Fahis for FahisThunai {
    fn ism(&self) -> &'static str {
        "thunai"
    }

    /// # Errors
    ///
    /// Only when the game's root directory cannot be listed at all. A binary
    /// that will not open, will not parse, or carries nothing recognisable is
    /// the absence of evidence and is reported as such.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let judhur = qaimat_jidhr(siyaq.jidhr)?;
        let mut hasad = Hasad::default();

        if let Some(masar) = hall_tanfidhi(siyaq, &judhur) {
            if siyaq.tanfidhi.is_none() {
                hasad.hasila.tanfidhi = Some(masar.clone());
            }
            iqra(siyaq.jidhr, &masar, true, &mut hasad);
        }

        for jar in jiran(&judhur) {
            iqra(siyaq.jidhr, &jar, false, &mut hasad);
        }

        Ok(hasad.ikhtim())
    }
}

// ---------------------------------------------------------------------------
// finding what to read
// ---------------------------------------------------------------------------

/// One entry directly inside the game root.
#[derive(Debug, Clone)]
struct MadkhalJidhr {
    /// The file name, folded to ASCII lowercase.
    ism: String,
    /// The absolute path.
    masar: PathBuf,
    /// Whether it is a directory.
    mujallad: bool,
}

/// Lists the game root, once, bounded.
///
/// # Errors
///
/// [`KhataMuharrik::JidhrMafqud`] when the directory is not there and
/// [`KhataMuharrik::TaadhurQiraatJidhr`] when it will not open. Nothing below
/// the root can produce an error from here, because nothing below the root is
/// listed.
fn qaimat_jidhr(jidhr: &Path) -> Natija<Vec<MadkhalJidhr>> {
    let qaima = std::fs::read_dir(jidhr).map_err(|sabab| {
        if sabab.kind() == std::io::ErrorKind::NotFound {
            KhataMuharrik::JidhrMafqud { jidhr: jidhr.to_path_buf() }
        } else {
            KhataMuharrik::TaadhurQiraatJidhr { jidhr: jidhr.to_path_buf(), sabab }
        }
    })?;

    let mut madakhil = Vec::new();
    for madkhal in qaima.flatten().take(AQSA_MADAKHIL_JIDHR) {
        let Ok(naw) = madkhal.file_type() else {
            continue;
        };
        madakhil.push(MadkhalJidhr {
            ism: madkhal.file_name().to_string_lossy().to_ascii_lowercase(),
            masar: madkhal.path(),
            mujallad: naw.is_dir(),
        });
    }
    Ok(madakhil)
}

/// The executable to read, when one can be had without guessing.
///
/// Discovery's answer first. Failing that, only what an engine's own naming
/// rule proves: a `*_Data` directory names the player beside it, and a `.app`
/// bundle names the binary inside it. Nothing here scores, compares sizes or
/// prefers one candidate over another — see the module header for why.
fn hall_tanfidhi(siyaq: &SiyaqFahs<'_>, judhur: &[MadkhalJidhr]) -> Option<PathBuf> {
    if let Some(masar) = siyaq.tanfidhi {
        // On macOS the thing discovery names is often the bundle rather than
        // the binary, and a bundle is a directory that no parser will read.
        return Some(dakhil_hazma(masar).unwrap_or_else(|| masar.to_path_buf()));
    }

    // Unity's rule: `Foo_Data` beside `Foo`.
    for madkhal in judhur.iter().filter(|madkhal| madkhal.mujallad) {
        let Some(jidhr_ism) = madkhal.ism.strip_suffix("_data") else {
            continue;
        };
        if jidhr_ism.is_empty() {
            continue;
        }
        let qarin = judhur.iter().find(|akhar| {
            !akhar.mujallad
                && IMTIDADAT_TANFIDH.iter().any(|imtidad| {
                    if imtidad.is_empty() {
                        akhar.ism == jidhr_ism
                    } else {
                        akhar.ism.strip_prefix(jidhr_ism).and_then(|baqi| baqi.strip_prefix('.'))
                            == Some(*imtidad)
                    }
                })
        });
        if let Some(qarin) = qarin {
            return Some(qarin.masar.clone());
        }
    }

    // The macOS bundle's rule: `Foo.app/Contents/MacOS/Foo`.
    judhur
        .iter()
        .filter(|madkhal| {
            madkhal.mujallad
                && madkhal
                    .ism
                    .rsplit_once('.')
                    .is_some_and(|(_, imtidad)| imtidad.eq_ignore_ascii_case("app"))
        })
        .find_map(|madkhal| dakhil_hazma(&madkhal.masar))
}

/// The real binary inside a macOS application bundle.
///
/// `Foo.app/Contents/MacOS/Foo` first, since that is what the bundle's own
/// `Info.plist` almost always names, and any single file in `Contents/MacOS`
/// second, for the bundles whose executable is not named after them.
fn dakhil_hazma(masar: &Path) -> Option<PathBuf> {
    let ism = masar.file_name()?.to_string_lossy().into_owned();
    if !ism.to_ascii_lowercase().ends_with(".app") {
        return None;
    }
    let jidhr_ism = ism.get(..ism.len().checked_sub(4)?)?;
    let macos = masar.join("Contents").join("MacOS");

    let musamma = macos.join(jidhr_ism);
    if musamma.is_file() {
        return Some(musamma);
    }
    std::fs::read_dir(&macos)
        .ok()?
        .flatten()
        .take(AQSA_MADAKHIL_JIDHR)
        .map(|madkhal| madkhal.path())
        .find(|masar| masar.is_file())
}

/// Executable suffixes an engine's name binding resolves to.
const IMTIDADAT_TANFIDH: &[&str] = &["exe", "x86_64", "x86", ""];

/// Engine libraries worth parsing when they sit beside the game.
///
/// A closed list, matched by exact lowercase file name. It is closed on purpose:
/// parsing every shared library in a game directory would mean parsing sixty
/// files for the two that carry evidence, and the ones that carry evidence are
/// all named by their engine rather than by their game.
const ASMAA_JIRAN: &[&str] = &[
    "unityplayer.dll",
    "unityplayer.so",
    "libunityplayer.so",
    "unityplayer.dylib",
    "gameassembly.dll",
    "libgameassembly.so",
    "gameassembly.dylib",
    "nw.dll",
    "libnw.so",
    "libcef.dll",
    "libcef.so",
    "rgss300.dll",
];

/// The engine libraries present, capped at [`AQSA_MULHAQAT`].
fn jiran(judhur: &[MadkhalJidhr]) -> Vec<PathBuf> {
    judhur
        .iter()
        .filter(|madkhal| !madkhal.mujallad && ASMAA_JIRAN.contains(&madkhal.ism.as_str()))
        .map(|madkhal| madkhal.masar.clone())
        .take(AQSA_MULHAQAT)
        .collect()
}

// ---------------------------------------------------------------------------
// reading one binary
// ---------------------------------------------------------------------------

/// A parsed binary, under the one concrete shape this file uses.
type Kaen<'a> = object::read::File<'a, &'a [u8]>;

/// The file did not parse at all.
///
/// Recorded rather than dropped. It is not evidence about the engine, it is the
/// reason the rest of the report is thin, and a maintainer reading a wrong
/// identification needs to see it.
const WAZN_LA_YUQRA: u8 = 10;

/// What the readers below write into.
///
/// [`HasilatFahs`] carries one engine family and one backend per detector, and
/// this detector reads several binaries per game — the executable and up to
/// [`AQSA_MULHAQAT`] engine libraries — each of which can claim something
/// different. A Unity game that also links CEF for its launcher web view claims
/// Unity at 95 and Electron at 75, and writing straight into the result would
/// leave whichever came last.
///
/// So claims accumulate here with the weight behind them, every one of them is
/// recorded as evidence with the import or symbol that produced it named
/// verbatim, and the strongest becomes the single answer at the end. Nothing is
/// averaged: the losing claim is in the evidence, at full weight, for
/// [`crate::tahdid`] to weigh against what the other detectors saw.
#[derive(Debug, Default)]
struct Hasad {
    /// What is returned.
    hasila: HasilatFahs,
    /// Every family claimed, with the weight behind it.
    ailat: Vec<(AilatMuharrik, u8)>,
    /// Every backend claimed, with the weight behind it.
    khalfiyat: Vec<(KhalfiyaBarmajiya, u8)>,
}

impl Hasad {
    /// Records an observation that names no engine.
    fn sajjil(&mut self, wasf: impl Into<String>, mawqi: Option<String>, wazn: u8) {
        self.hasila.sajjil(NawDaleel::TawqiThunai, wasf, mawqi, wazn);
    }

    /// Records an observation and the engine family it names.
    fn aila(
        &mut self,
        aila: AilatMuharrik,
        wasf: impl Into<String>,
        mawqi: Option<String>,
        wazn: u8,
    ) {
        self.sajjil(wasf, mawqi, wazn);
        self.ailat.push((aila, wazn.min(100)));
    }

    /// Records a claim about how the game's code runs.
    fn khalfiya(&mut self, khalfiya: KhalfiyaBarmajiya, wazn: u8) {
        self.khalfiyat.push((khalfiya, wazn.min(100)));
    }

    /// Records a version, keeping the first one read.
    ///
    /// First rather than strongest because the searches are ordered so that an
    /// engine's own version tag is read before anything else, and because there
    /// is nothing here to rank a second reading by: every observation this
    /// detector makes is a byte sweep, so a later one is not better evidence
    /// than an earlier one.
    ///
    /// It is emphatically *not* true that only one version string is present in
    /// a game's binaries — a shipped `UnityPlayer.dll` carries the build's own
    /// stamp several times over and two vendor floor constants besides. What
    /// keeps that from deciding the answer is [`iqra_basmat_unity`] refusing the
    /// shape those constants are written in, and `tahdid` outranking every
    /// sweep with a version read from a record the engine wrote.
    fn isdar(&mut self, isdar: IsdarMuharrik) {
        if self.hasila.isdar.is_none() {
            self.hasila.isdar = Some(isdar);
        }
    }

    /// Settles on the strongest claims and returns the result.
    ///
    /// Strictly greater wins, so a tie leaves the earlier claim standing and
    /// the same game always produces the same answer.
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

/// The bytes of one binary, however they were obtained.
///
/// Both arms deref to a plain `[u8]` and every reader below works on that, so
/// nothing outside [`iftah`] knows or cares which one it got.
#[derive(Debug)]
enum Bayanat {
    /// Read into memory, which is what a file below [`HADD_KHARITA`] gets.
    Maqru(Vec<u8>),
    /// Mapped, which is what everything larger gets.
    Marsuma(memmap2::Mmap),
}

impl Bayanat {
    /// The bytes.
    fn bayt(&self) -> &[u8] {
        match self {
            Self::Maqru(bayt) => bayt,
            Self::Marsuma(kharita) => kharita,
        }
    }
}

/// Opens a binary, mapping it only when it is large enough for that to pay.
///
/// Returns `None` for anything that is not a readable, non-empty regular file,
/// which is the correct answer for a directory, a dangling link, a file the
/// user cannot read, and a zero-length placeholder left by an interrupted
/// download.
fn iftah(masar: &Path) -> Option<Bayanat> {
    let bayanat = std::fs::metadata(masar).ok()?;
    if !bayanat.is_file() {
        return None;
    }
    let hajm = bayanat.len();
    if hajm == 0 {
        return None;
    }
    if hajm <= HADD_KHARITA {
        return std::fs::read(masar).ok().map(Bayanat::Maqru);
    }

    let malaf = std::fs::File::open(masar).ok()?;
    // SAFETY: `Mmap::map` is unsafe because the mapping's contents are outside
    // Rust's control for as long as it lives: another process may truncate or
    // overwrite the file, and touching a page whose backing store has become
    // unreachable raises a signal rather than returning an error. Neither can
    // be prevented here, so both are bounded instead, exactly as the module
    // header sets out — the mapping lives only until this probe of this file
    // returns, nothing borrowed from it escapes, every access goes through a
    // bounds-checked slice, and only the pages inside the search windows are
    // ever touched.
    let kharita = unsafe { memmap2::Mmap::map(&malaf) }.ok()?;
    Some(Bayanat::Marsuma(kharita))
}

/// Reads one binary and records everything it has to say.
///
/// `raisi` marks the game's own executable, which is the only file whose
/// architecture is the game's architecture and the only one whose tail is worth
/// checking for an appended content pack.
fn iqra(jidhr: &Path, masar: &Path, raisi: bool, hasad: &mut Hasad) {
    let Some(bayanat) = iftah(masar) else {
        return;
    };
    let bayt = bayanat.bayt();
    let ism = nisbi(jidhr, masar);

    // The pack footer sits at the very end of the file and needs no parse, so
    // it is read before anything is given the chance to refuse the file.
    if raisi {
        dhayl_godot(bayt, &ism, hasad);
    }

    let Ok(kaen) = object::read::File::parse(bayt) else {
        hasad.sajjil(
            format!(
                "{ism} is not a PE, ELF or Mach-O image this build can parse. It is packed, \
                 protected, encrypted or a universal binary, so it carries no readable imports \
                 or version strings and everything reported here came from elsewhere"
            ),
            Some(ism.clone()),
            WAZN_LA_YUQRA,
        );
        if raisi {
            mimariya_ihtiyatiya(masar, hasad);
        }
        return;
    };

    if raisi && hasad.hasila.mimariya.is_none() {
        hasad.hasila.mimariya = mimariya_min(&kaen);
    }

    aqsam(&kaen, &ism, hasad);
    mustawradat(&kaen, &ism, hasad);
    musaddarat(&kaen, &ism, hasad);
    nusus(&kaen, bayt, &ism, hasad);
}

/// A path as the evidence trail names it: relative to the game's root.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    masar.strip_prefix(jidhr).unwrap_or(masar).to_string_lossy().into_owned()
}

/// The architecture, from the header `object` already parsed.
///
/// **On not writing this a third time.** `taarib_usus::manassa::mimariyat_malaf`
/// reads the same three headers by hand, and it is the authority: it is what
/// decides which Taarib library can be loaded into the game, and Phase 15 calls
/// it. This function does not reimplement it and does not compete with it — it
/// is a single `match` from an enum `object` has already produced, containing
/// no offsets, no magic numbers and no machine constants, so there is exactly
/// one place in the workspace where those are written down and it is not here.
///
/// It is preferred over calling `mimariyat_malaf` for one reason: that function
/// takes a path and reads the whole file, which for the 300 MB executable this
/// detector has deliberately mapped rather than read would undo the entire
/// point of mapping it. The header is already parsed and already resident; the
/// answer costs nothing.
///
/// `mimariyat_malaf` remains the fallback, in [`mimariya_ihtiyatiya`], and the
/// fallback is not decorative: a universal Mach-O binary is a shape
/// `object::File::parse` refuses and `mimariyat_malaf` has an explicit policy
/// for, and universal binaries are ordinary on macOS.
fn mimariya_min(kaen: &Kaen<'_>) -> Option<Mimariya> {
    match kaen.architecture() {
        object::Architecture::I386 => Some(Mimariya::X86),
        object::Architecture::X86_64 => Some(Mimariya::X8664),
        object::Architecture::Aarch64 => Some(Mimariya::Aarch64),
        _ => None,
    }
}

/// The architecture for a file `object` would not take.
///
/// Bounded by [`HADD_QIRA_IHTIYATI`], because this is the one read in the
/// detector that is not a mapped one and it must not become a way to read a
/// quarter-gigabyte binary by accident.
fn mimariya_ihtiyatiya(masar: &Path, hasad: &mut Hasad) {
    let hajm = std::fs::metadata(masar).map_or(u64::MAX, |bayanat| bayanat.len());
    if hajm > HADD_QIRA_IHTIYATI {
        return;
    }
    if let Ok(mimariya) = taarib_usus::manassa::mimariyat_malaf(masar) {
        hasad.hasila.mimariya = Some(mimariya);
    }
}

// ---------------------------------------------------------------------------
// section names
// ---------------------------------------------------------------------------

/// A packer's or protector's own section name.
///
/// Weighted as a mid-strength observation because that is exactly what it is:
/// it establishes something true and useful about the file without saying
/// anything at all about the engine.
const WAZN_HIMAYA: u8 = 40;

/// Section names that belong to a packer or a protector rather than to a
/// compiler.
///
/// A hit here is the answer to "why does this executable import nothing?".
/// Every one of these rewrites or encrypts the original image, so the import
/// table `object` can see is a stub and the string literals are gone until the
/// unpacker runs at load time — which this crate never does, because probing
/// never executes a game's code.
const AQSAM_HIMAYA: &[(&str, &str)] = &[
    ("upx0", "UPX"),
    ("upx1", "UPX"),
    ("upx2", "UPX"),
    (".vmp0", "VMProtect"),
    (".vmp1", "VMProtect"),
    (".vmp2", "VMProtect"),
    (".themida", "Themida"),
    (".winlice", "WinLicense"),
    (".enigma1", "the Enigma Protector"),
    (".enigma2", "the Enigma Protector"),
    (".aspack", "ASPack"),
    (".petite", "Petite"),
    (".bind", "a Steam or Enigma wrapper"),
    (".mpress1", "MPRESS"),
];

/// Sections that hold constant data, which is where a linker puts strings.
///
/// PE first, then ELF, then the two Mach-O sections that matter. The order is
/// the order the budget is spent in, and it is the order these appear in real
/// images.
pub const AQSAM_THAWABIT: &[&str] =
    &[".rdata", ".rodata", ".data.rel.ro", "__const", "__cstring"];

/// Reports the section names that mean something, which is almost none of them.
///
/// A game's `.text`, `.data` and `.reloc` say nothing that its imports do not
/// say better, so the only names read here are the ones that belong to a
/// packer.
fn aqsam(kaen: &Kaen<'_>, ism: &str, hasad: &mut Hasad) {
    let mut mawjud: Vec<&str> = Vec::new();
    for qism in kaen.sections() {
        let Ok(ism_qism) = qism.name() else {
            continue;
        };
        let saghir = ism_qism.to_ascii_lowercase();
        for (matlub, adat) in AQSAM_HIMAYA {
            if saghir == *matlub && !mawjud.contains(adat) {
                mawjud.push(*adat);
            }
        }
    }
    for adat in mawjud {
        hasad.sajjil(
            format!(
                "{ism} carries the section names {adat} writes, so the image is packed and its \
                 real imports and strings are not readable without running it"
            ),
            Some(ism.to_owned()),
            WAZN_HIMAYA,
        );
    }
}

// ---------------------------------------------------------------------------
// imported modules
// ---------------------------------------------------------------------------

/// What one imported module name means.
#[derive(Debug, Clone, Copy)]
struct DalalatMaktaba {
    /// The name to look for, lowercase.
    ibra: &'static str,
    /// Matched as a substring of the whole import name rather than against the
    /// file name alone.
    ///
    /// Needed because the three platforms spell the same dependency three
    /// ways: `vulkan-1.dll` is a file name, `libvulkan.so.1` carries a version
    /// suffix, and a macOS framework arrives as the whole install path
    /// `/System/Library/Frameworks/Metal.framework/Versions/A/Metal`.
    juzi: bool,
    /// The engine family it points at, when it points at one.
    aila: Option<AilatMuharrik>,
    /// The scripting backend it implies.
    khalfiya: Option<KhalfiyaBarmajiya>,
    /// The graphics API it proves.
    rusum: Option<WajihaRusum>,
    /// What the observation says, in plain words, with the import named.
    wasf: &'static str,
    /// How much it is worth.
    wazn: u8,
}

/// Every import worth recognising.
///
/// Ordered by what it establishes rather than alphabetically: engine runtimes,
/// then scripting runtimes, then graphics, then the text libraries an engine
/// links when it already shapes text for itself. The Direct3D entries run
/// oldest generation to newest inside that block, which is the order they are
/// read in and the order the report lists them in.
///
/// Every graphics entry names one API and only ever the API the module *is*.
/// Nothing here infers a renderer from a redistributable a game ships beside
/// itself — that is [`crate::dalail::binya`]'s evidence, at its own weight, and
/// the difference between the two is the difference between a fact and an
/// inference. See [`rusum_maarufa`] for the invariant this list has to satisfy.
const DALALAT: &[DalalatMaktaba] = &[
    DalalatMaktaba {
        ibra: "unityplayer",
        juzi: true,
        aila: Some(AilatMuharrik::Unity),
        khalfiya: None,
        rusum: None,
        wasf: "the Unity player runtime, which the loader resolves before the game starts",
        wazn: 95,
    },
    DalalatMaktaba {
        ibra: "gameassembly",
        juzi: true,
        aila: Some(AilatMuharrik::Unity),
        khalfiya: Some(KhalfiyaBarmajiya::Il2cpp),
        rusum: None,
        wasf: "the game's managed code compiled ahead of time into a native library by IL2CPP",
        wazn: 95,
    },
    DalalatMaktaba {
        ibra: "mono-2.0-bdwgc",
        juzi: true,
        aila: Some(AilatMuharrik::Unity),
        khalfiya: Some(KhalfiyaBarmajiya::Mono),
        rusum: None,
        wasf: "Unity's own fork of the Mono runtime, which only a Unity Mono build links",
        wazn: 90,
    },
    DalalatMaktaba {
        ibra: "monobdwgc",
        juzi: true,
        aila: Some(AilatMuharrik::Unity),
        khalfiya: Some(KhalfiyaBarmajiya::Mono),
        rusum: None,
        wasf: "Unity's own fork of the Mono runtime under its Linux name",
        wazn: 90,
    },
    DalalatMaktaba {
        ibra: "mono-2.0",
        juzi: true,
        aila: None,
        khalfiya: Some(KhalfiyaBarmajiya::Mono),
        rusum: None,
        wasf: "a Mono runtime, which several engines embed and which is not Unity's on its own",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "rgss300.dll",
        juzi: false,
        aila: Some(AilatMuharrik::RpgMakerVxAce),
        khalfiya: Some(KhalfiyaBarmajiya::Ruby),
        rusum: None,
        wasf: "the RGSS3 interpreter, which is RPG Maker VX Ace and nothing else",
        wazn: 92,
    },
    DalalatMaktaba {
        ibra: "nw.dll",
        juzi: false,
        aila: Some(AilatMuharrik::Electron),
        khalfiya: Some(KhalfiyaBarmajiya::JavaScript),
        rusum: None,
        wasf: "the NW.js runtime, so the interface is a web document drawn by Chromium",
        wazn: 88,
    },
    DalalatMaktaba {
        ibra: "libnw.so",
        juzi: false,
        aila: Some(AilatMuharrik::Electron),
        khalfiya: Some(KhalfiyaBarmajiya::JavaScript),
        rusum: None,
        wasf: "the NW.js runtime under its Linux name",
        wazn: 88,
    },
    DalalatMaktaba {
        ibra: "libcef",
        juzi: true,
        aila: Some(AilatMuharrik::Electron),
        khalfiya: Some(KhalfiyaBarmajiya::JavaScript),
        rusum: None,
        wasf: "the Chromium Embedded Framework, so the interface is a web document",
        wazn: 75,
    },
    DalalatMaktaba {
        ibra: "python3",
        juzi: true,
        aila: None,
        khalfiya: Some(KhalfiyaBarmajiya::Python),
        rusum: None,
        wasf: "a Python 3 runtime, which for a game usually means Ren'Py",
        wazn: 65,
    },
    DalalatMaktaba {
        ibra: "python2",
        juzi: true,
        aila: None,
        khalfiya: Some(KhalfiyaBarmajiya::Python),
        rusum: None,
        wasf: "a Python 2 runtime, which for a game means an older Ren'Py release",
        wazn: 65,
    },
    DalalatMaktaba {
        ibra: "d3d8.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d8),
        wasf: "Direct3D 8, so the overlay tier attaches at the device's own Present",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "d3d9.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d9),
        wasf: "Direct3D 9, so the overlay tier attaches at the device's own Present and Reset",
        wazn: 60,
    },
    // The D3DX9 utility library, versioned `d3dx9_24` through `d3dx9_43`, which
    // is why this one is a substring. It is corroboration rather than proof: it
    // is a helper a D3D9 renderer links and not the API itself, and a tool that
    // never draws can link it too. It earns its place because it survives where
    // `d3d9.dll` does not — a game reaching Direct3D 9 through a wrapper shipped
    // beside it imports the wrapper's name and still imports this.
    DalalatMaktaba {
        ibra: "d3dx9_",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d9),
        wasf: "the D3DX9 utility library, which only a Direct3D 9 renderer links",
        wazn: 45,
    },
    DalalatMaktaba {
        ibra: "d3d10.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d10),
        wasf: "Direct3D 10, so the overlay tier attaches through a DXGI swap chain",
        wazn: 60,
    },
    // Matched exactly rather than as a prefix of the entry above, so that a
    // Direct3D 10.1 game — which links only this one — is read, and so that
    // `d3d10.dll` cannot claim it twice. Both resolve to the same value: a 10.1
    // device answers a `QueryInterface` for `ID3D10Device`, so one backend
    // draws through either.
    DalalatMaktaba {
        ibra: "d3d10_1.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d10),
        wasf: "Direct3D 10.1, which the overlay tier reaches through the same DXGI swap chain \
               as Direct3D 10",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "d3d11.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d11),
        wasf: "Direct3D 11, so the overlay tier attaches through a D3D11 swap chain",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "d3d12.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::D3d12),
        wasf: "Direct3D 12, so the overlay tier attaches through a D3D12 command queue",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "vulkan-1.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::Vulkan),
        wasf: "the Vulkan loader",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "libvulkan.so",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::Vulkan),
        wasf: "the Vulkan loader under its Linux name",
        wazn: 60,
    },
    DalalatMaktaba {
        ibra: "opengl32.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::OpenGl),
        wasf: "OpenGL",
        wazn: 55,
    },
    DalalatMaktaba {
        ibra: "libgl.so",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::OpenGl),
        wasf: "OpenGL under its Linux name",
        wazn: 50,
    },
    DalalatMaktaba {
        ibra: "metal.framework",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: Some(WajihaRusum::Metal),
        wasf: "Metal, which is the only route to the screen on current macOS",
        wazn: 70,
    },
    DalalatMaktaba {
        ibra: "dxgi.dll",
        juzi: false,
        aila: None,
        khalfiya: None,
        rusum: None,
        wasf: "DXGI, which says the game presents through Direct3D without saying which version",
        wazn: 30,
    },
    DalalatMaktaba {
        ibra: "sdl2",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: None,
        wasf: "SDL2, which is used by so many engines that it narrows nothing on its own",
        wazn: 25,
    },
    DalalatMaktaba {
        ibra: "harfbuzz",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: None,
        wasf: "HarfBuzz, so the engine already shapes complex scripts and needs correcting \
               rather than replacing",
        wazn: 50,
    },
    DalalatMaktaba {
        ibra: "fribidi",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: None,
        wasf: "FriBidi, so the engine already resolves bidirectional text",
        wazn: 50,
    },
    DalalatMaktaba {
        ibra: "freetype",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: None,
        wasf: "FreeType, so the engine rasterizes its own glyphs from font files",
        wazn: 45,
    },
    DalalatMaktaba {
        ibra: "icuuc",
        juzi: true,
        aila: None,
        khalfiya: None,
        rusum: None,
        wasf: "ICU, which an engine links for segmentation and normalization",
        wazn: 45,
    },
];

/// Every graphics API some import in [`DALALAT`] can establish.
///
/// Exists so that the one rule binding this crate's vocabulary to the overlay's
/// backends is checkable rather than remembered. A backend can be built, and a
/// value for it can be added to [`WajihaRusum`], and the game that uses it will
/// still report nothing unless some import names it here — which is exactly the
/// state Direct3D 8, 9 and 10 were in: three backends drawing, three values
/// missing, and a Direct3D 9 game reporting an empty graphics list with
/// `d3d9.dll` sitting in its import table.
///
/// `crates/taarib-muharrik/tests/mufradat_rusum.rs` asserts against this, so
/// the ninth backend cannot be added quietly.
#[must_use]
pub fn rusum_maarufa() -> Vec<WajihaRusum> {
    let mut rusum: Vec<WajihaRusum> = Vec::new();
    for dalala in DALALAT {
        if let Some(wajiha) = dalala.rusum
            && !rusum.contains(&wajiha)
        {
            rusum.push(wajiha);
        }
    }
    rusum
}

/// Reads the import table and records what each recognised module means.
///
/// The three formats disagree about what an "import" is, and the disagreement
/// is handled by looking at both halves of every entry. A PE import names its
/// DLL; a Mach-O import names the dylib's install path; an ELF import names a
/// symbol and, where the binary uses symbol versioning, the shared object that
/// version came from. Matching the library half against a file name and the
/// whole string against a substring covers all three.
fn mustawradat(kaen: &Kaen<'_>, ism: &str, hasad: &mut Hasad) {
    let Ok(mustawradat) = kaen.imports() else {
        return;
    };

    let mut ruyat: Vec<&'static str> = Vec::new();
    for mustawrad in mustawradat {
        // The reader yields a Result per entry: a malformed import table is a
        // fact about the file, and one bad entry never fails the whole probe.
        let Ok(mustawrad) = mustawrad else { continue };
        let kamil = String::from_utf8_lossy(mustawrad.library()).to_ascii_lowercase();
        if kamil.is_empty() {
            continue;
        }
        let malaf = kamil.rsplit(['/', '\\']).next().unwrap_or(&kamil);
        for dalala in DALALAT {
            let mutabiq =
                if dalala.juzi { kamil.contains(dalala.ibra) } else { malaf == dalala.ibra };
            if !mutabiq || ruyat.contains(&dalala.ibra) {
                continue;
            }
            ruyat.push(dalala.ibra);
            sajjil_dalala(dalala, ism, &kamil, hasad);
        }
    }
}

/// Records one recognised import against the result.
fn sajjil_dalala(dalala: &DalalatMaktaba, ism: &str, kamil: &str, hasad: &mut Hasad) {
    let wasf = format!("{ism} imports {kamil}: {}", dalala.wasf);
    match dalala.aila {
        Some(aila) => hasad.aila(aila, wasf, Some(ism.to_owned()), dalala.wazn),
        None => hasad.sajjil(wasf, Some(ism.to_owned()), dalala.wazn),
    }
    if let Some(khalfiya) = dalala.khalfiya {
        hasad.khalfiya(khalfiya, dalala.wazn);
    }
    if let Some(wajiha) = dalala.rusum {
        hasad.hasila.daa_rusum(wajiha);
    }
}

// ---------------------------------------------------------------------------
// exported symbols
// ---------------------------------------------------------------------------

/// One exported symbol that names a runtime.
#[derive(Debug, Clone, Copy)]
struct DalalatRamz {
    /// The exported symbol name, exactly.
    ibra: &'static str,
    /// The engine family it names.
    aila: Option<AilatMuharrik>,
    /// The backend it names.
    khalfiya: Option<KhalfiyaBarmajiya>,
    /// What it means, in plain words.
    wasf: &'static str,
    /// How much it is worth.
    wazn: u8,
}

/// Exported symbols worth recognising.
///
/// Short by design. A game's own executable exports nothing useful — it is the
/// program, not a library — so everything here is found on the engine libraries
/// beside it, and each entry is a symbol that exists for exactly one runtime.
const RUMUZ: &[DalalatRamz] = &[
    DalalatRamz {
        ibra: "il2cpp_init",
        aila: Some(AilatMuharrik::Unity),
        khalfiya: Some(KhalfiyaBarmajiya::Il2cpp),
        wasf: "the IL2CPP runtime's own entry point",
        wazn: 90,
    },
    DalalatRamz {
        ibra: "il2cpp_class_from_name",
        aila: Some(AilatMuharrik::Unity),
        khalfiya: Some(KhalfiyaBarmajiya::Il2cpp),
        wasf: "the IL2CPP runtime's type lookup, which is how an adapter resolves a method",
        wazn: 88,
    },
    DalalatRamz {
        ibra: "UnityMain",
        aila: Some(AilatMuharrik::Unity),
        khalfiya: None,
        wasf: "the Unity player's own entry point",
        wazn: 88,
    },
    DalalatRamz {
        ibra: "mono_jit_init_version",
        aila: None,
        khalfiya: Some(KhalfiyaBarmajiya::Mono),
        wasf: "a Mono runtime's initializer",
        wazn: 80,
    },
    DalalatRamz {
        ibra: "mono_domain_assembly_open",
        aila: None,
        khalfiya: Some(KhalfiyaBarmajiya::Mono),
        wasf: "a Mono runtime's assembly loader",
        wazn: 75,
    },
];

/// Reads the export table for the symbols that name a runtime.
fn musaddarat(kaen: &Kaen<'_>, ism: &str, hasad: &mut Hasad) {
    let Ok(musaddarat) = kaen.exports() else {
        return;
    };
    for musaddar in musaddarat {
        // The reader yields a Result per entry: a malformed table is a fact
        // about the file, and one bad entry never fails the whole scan.
        let Ok(musaddar) = musaddar else { continue };
        // An export identified only by ordinal carries no name, and every
        // signal here is a name — nothing to match, so nothing to weigh.
        let ism_wa_raqm = musaddar.name();
        let Some(ism_khaam) = ism_wa_raqm.name() else { continue };
        let Ok(ism_ramz) = std::str::from_utf8(ism_khaam) else {
            continue;
        };
        for dalala in RUMUZ {
            if ism_ramz != dalala.ibra {
                continue;
            }
            let wasf = format!("{ism} exports {ism_ramz}: {}", dalala.wasf);
            match dalala.aila {
                Some(aila) => hasad.aila(aila, wasf, Some(ism.to_owned()), dalala.wazn),
                None => hasad.sajjil(wasf, Some(ism.to_owned()), dalala.wazn),
            }
            if let Some(khalfiya) = dalala.khalfiya {
                hasad.khalfiya(khalfiya, dalala.wazn);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// embedded strings
// ---------------------------------------------------------------------------

/// Unity's own version stamp, whose shape no other engine writes.
const WAZN_ISDAR_UNITY: u8 = 85;

/// A pack whose footer says it is appended to this executable.
const WAZN_DHAYL: u8 = 92;

/// How many characters of the text following a match are read as a version.
///
/// Long enough for `4.2.1.stable.official.b09f793f5` and short enough that a
/// match landing in the middle of an unrelated string cannot drag a paragraph
/// into the evidence trail.
const AQSA_DHAYL: usize = 48;

/// One string worth looking for, and what finding it means.
#[derive(Debug, Clone, Copy)]
struct BasmatNass {
    /// The characters to look for.
    ibra: &'static str,
    /// The engine family it names.
    aila: Option<AilatMuharrik>,
    /// The backend it names.
    khalfiya: Option<KhalfiyaBarmajiya>,
    /// Text systems its presence establishes.
    itarat: &'static [ItarNusus],
    /// Whether the printable text after the match is this engine's version.
    isdar: bool,
    /// What it means, in plain words.
    wasf: &'static str,
    /// How much it is worth.
    wazn: u8,
}

/// Every embedded string worth recognising.
///
/// Ordered so that an engine's own version tag is searched before anything
/// else, because [`Hasad::isdar`] keeps the first version it is given and the
/// engine's is the one a signature database keys on. Every needle is ASCII, and
/// each is searched in both the narrow and the UTF-16 form for the reason
/// [`yutabiq`] documents.
const BASMAT: &[BasmatNass] = &[
    BasmatNass {
        ibra: "++UE5+Release-",
        aila: Some(AilatMuharrik::Unreal),
        khalfiya: Some(KhalfiyaBarmajiya::UnrealNative),
        itarat: &[ItarNusus::Slate],
        isdar: true,
        wasf: "the engine branch tag Unreal writes into its own FEngineVersion, naming UE5",
        wazn: 90,
    },
    BasmatNass {
        ibra: "++UE4+Release-",
        aila: Some(AilatMuharrik::Unreal),
        khalfiya: Some(KhalfiyaBarmajiya::UnrealNative),
        itarat: &[ItarNusus::Slate],
        isdar: true,
        wasf: "the engine branch tag Unreal writes into its own FEngineVersion, naming UE4",
        wazn: 90,
    },
    BasmatNass {
        ibra: "Godot Engine v",
        aila: Some(AilatMuharrik::Godot),
        khalfiya: None,
        itarat: &[],
        isdar: true,
        wasf: "the banner Godot prints for itself, which carries its full version",
        wazn: 90,
    },
    BasmatNass {
        ibra: "TextServerAdvanced",
        aila: Some(AilatMuharrik::Godot),
        khalfiya: None,
        itarat: &[ItarNusus::GodotLabel, ItarNusus::GodotRichText],
        isdar: false,
        wasf: "Godot 4's complex-script text server, which means the engine can already shape \
               Arabic and needs enabling rather than replacing",
        wazn: 80,
    },
    BasmatNass {
        ibra: "Electron/",
        aila: Some(AilatMuharrik::Electron),
        khalfiya: Some(KhalfiyaBarmajiya::JavaScript),
        itarat: &[ItarNusus::Dom],
        isdar: true,
        wasf: "the Electron fragment of the runtime's own user agent string",
        wazn: 80,
    },
    BasmatNass {
        ibra: "+++UE5",
        aila: Some(AilatMuharrik::Unreal),
        khalfiya: Some(KhalfiyaBarmajiya::UnrealNative),
        itarat: &[ItarNusus::Slate],
        isdar: false,
        wasf: "an Unreal build-version marker naming UE5, without a readable release tag",
        wazn: 70,
    },
    BasmatNass {
        ibra: "+++UE4",
        aila: Some(AilatMuharrik::Unreal),
        khalfiya: Some(KhalfiyaBarmajiya::UnrealNative),
        itarat: &[ItarNusus::Slate],
        isdar: false,
        wasf: "an Unreal build-version marker naming UE4, without a readable release tag",
        wazn: 70,
    },
    BasmatNass {
        ibra: "renpy.bootstrap",
        aila: Some(AilatMuharrik::Renpy),
        khalfiya: Some(KhalfiyaBarmajiya::Python),
        itarat: &[ItarNusus::NassRenpy],
        isdar: false,
        wasf: "the module Ren'Py's launcher imports to start the game",
        wazn: 70,
    },
    BasmatNass {
        ibra: "RGSS3",
        aila: Some(AilatMuharrik::RpgMakerVxAce),
        khalfiya: Some(KhalfiyaBarmajiya::Ruby),
        itarat: &[ItarNusus::NafidhatRpg],
        isdar: false,
        wasf: "the RGSS3 marker RPG Maker VX Ace writes into its player",
        wazn: 70,
    },
    BasmatNass {
        ibra: "YoYo Games",
        aila: Some(AilatMuharrik::GameMaker),
        khalfiya: Some(KhalfiyaBarmajiya::GameMakerVm),
        itarat: &[ItarNusus::RasmGameMaker],
        isdar: false,
        wasf: "the GameMaker runner's own attribution string",
        wazn: 60,
    },
    BasmatNass {
        ibra: "nwjs",
        aila: Some(AilatMuharrik::Electron),
        khalfiya: Some(KhalfiyaBarmajiya::JavaScript),
        itarat: &[ItarNusus::Dom],
        isdar: false,
        wasf: "an NW.js marker, so the interface is a web document",
        wazn: 55,
    },
    BasmatNass {
        ibra: "SlateCore",
        aila: Some(AilatMuharrik::Unreal),
        khalfiya: Some(KhalfiyaBarmajiya::UnrealNative),
        itarat: &[ItarNusus::Slate],
        isdar: false,
        wasf: "the name Unreal registers its Slate module under",
        wazn: 55,
    },
    BasmatNass {
        ibra: "GDPC",
        aila: Some(AilatMuharrik::Godot),
        khalfiya: None,
        itarat: &[],
        isdar: false,
        wasf: "the four bytes Godot uses as its pack magic, compiled into the binary that mounts \
               packs",
        wazn: 35,
    },
    BasmatNass {
        ibra: "GODOT",
        aila: Some(AilatMuharrik::Godot),
        khalfiya: None,
        itarat: &[],
        isdar: false,
        wasf: "an uppercase Godot marker, which the engine writes in several places and which on \
               its own only suggests",
        wazn: 35,
    },
];

/// Searches the constant-data windows for every recognised string.
fn nusus<'a>(kaen: &Kaen<'a>, bayt: &'a [u8], ism: &str, hasad: &mut Hasad) {
    let manatiq = nawafidh(kaen, bayt);
    let mut mawaqi: Vec<Option<(usize, MawqiBasma)>> = vec![None; BASMAT.len()];
    for (raqm_nafidha, nafidha) in manatiq.iter().enumerate() {
        ibhath_jami(nafidha, raqm_nafidha, &mut mawaqi);
    }

    for (raqm, basma) in BASMAT.iter().enumerate() {
        let Some(Some((raqm_nafidha, mawqi))) = mawaqi.get(raqm).copied() else {
            continue;
        };
        let Some(nafidha) = manatiq.get(raqm_nafidha) else {
            continue;
        };

        let dhayl =
            if basma.isdar { dhayl_nass(nafidha, mawqi, AQSA_DHAYL) } else { String::new() };
        let wasf = if dhayl.is_empty() {
            format!("{ism} contains \"{}\": {}", basma.ibra, basma.wasf)
        } else {
            format!("{ism} contains \"{}{dhayl}\": {}", basma.ibra, basma.wasf)
        };

        match basma.aila {
            Some(aila) => hasad.aila(aila, wasf, Some(ism.to_owned()), basma.wazn),
            None => hasad.sajjil(wasf, Some(ism.to_owned()), basma.wazn),
        }
        if let Some(khalfiya) = basma.khalfiya {
            hasad.khalfiya(khalfiya, basma.wazn);
        }
        for itar in basma.itarat {
            hasad.hasila.daa_itar(*itar);
        }
        if basma.isdar
            && let Some(isdar) = isdar_min_nass(&dhayl, format!("{}{dhayl}", basma.ibra))
        {
            hasad.isdar(isdar);
        }
    }

    unity_isdar(&manatiq, ism, hasad);
}

/// Collects the windows worth searching, spending [`NAFIDHAT_BAHTH`] across
/// them.
///
/// Constant-data sections only, in the order the linker laid them out, because
/// that is where string literals live and because a game's `.text` is tens of
/// megabytes of instructions that will never contain `++UE5+Release-`. When a
/// binary has no section this file recognises — a stripped ELF, an unusual
/// Mach-O layout — the fallback is the head of the file itself, which is where
/// a small binary keeps everything anyway.
fn nawafidh<'a>(kaen: &Kaen<'a>, bayt: &'a [u8]) -> Vec<&'a [u8]> {
    let mut mizaniya = NAFIDHAT_BAHTH;
    let mut nawafidh: Vec<&'a [u8]> = Vec::new();

    for qism in kaen.sections() {
        if mizaniya == 0 {
            break;
        }
        let Ok(ism) = qism.name() else {
            continue;
        };
        if !AQSAM_THAWABIT.iter().any(|matlub| ism.eq_ignore_ascii_case(matlub)) {
            continue;
        }
        let Ok(bayanat) = qism.data() else {
            continue;
        };
        let tul = bayanat.len().min(mizaniya);
        let Some(hissa) = bayanat.get(..tul) else {
            continue;
        };
        mizaniya = mizaniya.saturating_sub(tul);
        nawafidh.push(hissa);
    }

    if nawafidh.is_empty()
        && let Some(hissa) = bayt.get(..bayt.len().min(NAFIDHAT_BAHTH))
    {
        nawafidh.push(hissa);
    }
    nawafidh
}

/// Where a needle was found, and how it was encoded.
#[derive(Debug, Clone, Copy)]
struct MawqiBasma {
    /// Index of the first byte after the match.
    bad: usize,
    /// The match was UTF-16, two bytes per character.
    thunai: bool,
}

/// Finds every recognised string in one walk of one window.
///
/// One walk rather than one per needle. Fourteen needles in two encodings would
/// otherwise mean twenty-eight scans of a window that can be sixteen megabytes,
/// and across a library of two hundred games that is minutes of wall clock
/// spent re-reading pages that have already been read — which, on a mapped
/// file, is also twenty-eight chances to fault instead of one.
///
/// Dispatching on the first byte is what makes the single walk cheap: one
/// lookup in a 256-entry table per position, and the needles are only compared
/// at the positions where some needle could start. `mawaqi` carries one slot
/// per needle and the first match wins, so a needle already found in an earlier
/// window is skipped.
fn ibhath_jami(nafidha: &[u8], raqm_nafidha: usize, mawaqi: &mut [Option<(usize, MawqiBasma)>]) {
    let awail = awail_basmat();
    let mut mawdi = 0usize;
    while mawdi < nafidha.len() {
        let Some(&bayt) = nafidha.get(mawdi) else {
            break;
        };
        if awail.get(usize::from(bayt)).copied().unwrap_or(false) {
            for (raqm, basma) in BASMAT.iter().enumerate() {
                let Some(khana) = mawaqi.get_mut(raqm) else {
                    continue;
                };
                if khana.is_some() || basma.ibra.as_bytes().first() != Some(&bayt) {
                    continue;
                }
                if let Some(mawqi) = yutabiq(nafidha, mawdi, basma.ibra) {
                    *khana = Some((raqm_nafidha, mawqi));
                }
            }
        }
        mawdi = mawdi.saturating_add(1);
    }
}

/// Which bytes any recognised string can begin with.
///
/// The two encodings share a first byte — UTF-16 stores `+` as `+\0` — so one
/// table serves both.
fn awail_basmat() -> [bool; 256] {
    let mut awail = [false; 256];
    for basma in BASMAT {
        if let Some(&bayt) = basma.ibra.as_bytes().first()
            && let Some(khana) = awail.get_mut(usize::from(bayt))
        {
            *khana = true;
        }
    }
    awail
}

/// Tries one ASCII needle at one position, in either encoding a compiler emits.
///
/// Unreal's `TEXT()` macro produces UTF-16 string literals on every platform it
/// targets, so `++UE5+Release-5.3` is stored as `+\0+\0U\0E\0…` and a plain
/// byte comparison finds nothing. Unity's and Godot's version stamps are narrow.
/// Trying narrow first and wide second is the whole difference between reading
/// an Unreal version and reporting none.
fn yutabiq(nafidha: &[u8], mawdi: usize, ibra: &str) -> Option<MawqiBasma> {
    let nihaya = mawdi.checked_add(ibra.len())?;
    if nafidha.get(mawdi..nihaya) == Some(ibra.as_bytes()) {
        return Some(MawqiBasma { bad: nihaya, thunai: false });
    }

    let mut wasee = mawdi;
    for matlub in ibra.bytes() {
        if harf(nafidha, wasee, true) != Some(matlub) {
            return None;
        }
        wasee = wasee.checked_add(2)?;
    }
    Some(MawqiBasma { bad: wasee, thunai: true })
}

/// One character at a byte offset, in the encoding a match was found in.
///
/// For a UTF-16 match the high byte must be zero, which is what keeps the wide
/// reader from walking off a needle into unrelated bytes.
fn harf(nafidha: &[u8], mawqi: usize, thunai: bool) -> Option<u8> {
    let bayt = *nafidha.get(mawqi)?;
    if thunai && nafidha.get(mawqi.checked_add(1)?) != Some(&0) {
        return None;
    }
    Some(bayt)
}

/// Reads the printable text following a match, in the encoding it was found in.
fn dhayl_nass(nafidha: &[u8], mawqi: MawqiBasma, aqsa: usize) -> String {
    let khatwa = if mawqi.thunai { 2 } else { 1 };
    let mut nass = String::new();
    let mut mawdi = mawqi.bad;
    while nass.len() < aqsa {
        let Some(bayt) = harf(nafidha, mawdi, mawqi.thunai) else {
            break;
        };
        if !bayt.is_ascii_graphic() {
            break;
        }
        nass.push(char::from(bayt));
        mawdi = mawdi.saturating_add(khatwa);
    }
    nass
}

/// Parses a leading `major.minor.patch` out of a version tail.
///
/// The raw string is kept whole, because engine version strings carry more than
/// three numbers and the extra part is exactly what a signature database keys
/// on.
fn isdar_min_nass(dhayl: &str, khaam: String) -> Option<IsdarMuharrik> {
    let mut ajzaa = dhayl.split(|ramz: char| !ramz.is_ascii_digit());
    let kabir = ajzaa.next()?.parse::<u16>().ok()?;
    let sagheer = ajzaa.next().and_then(|juz| juz.parse::<u16>().ok()).unwrap_or(0);
    let tasheeh = ajzaa.next().and_then(|juz| juz.parse::<u16>().ok()).unwrap_or(0);
    Some(IsdarMuharrik { kabir, sagheer, tasheeh, khaam, mushtaqq: false })
}

// ---------------------------------------------------------------------------
// Unity's version stamp
// ---------------------------------------------------------------------------

/// Finds Unity's own version stamp, which is a shape rather than a needle.
///
/// Unity writes `2021.3.16f1`, `5.6.7f1`, `2019.4.40f1`, `6000.0.23f1` — three
/// numbers, then a release-type letter, then a revision. Nothing else in a game
/// binary looks like that, which is why it is worth scanning for a *pattern*
/// here rather than for a literal: the version cannot be listed in advance, and
/// there is no fixed marker string beside it that survives across the fifteen
/// years of releases this product has to identify.
///
/// The major number is checked against the ranges Unity has actually used —
/// 3 to 5, the year-numbered releases, and the 6000 series — which is what
/// stops a date, a build number or a coordinate pair from being read as an
/// engine version. The release-type letter is checked too, and
/// [`iqra_basmat_unity`] says why that one matters more.
///
/// The first accepted shape in the windows wins. That is enough here and is not
/// relied on to be right on its own: a stamp swept out of a binary carries
/// [`NawDaleel::TawqiThunai`], and `tahdid` ranks any version read from a
/// record the engine wrote above it.
fn unity_isdar(nawafidh: &[&[u8]], ism: &str, hasad: &mut Hasad) {
    let Some(khaam) = nawafidh.iter().find_map(|nafidha| basmat_unity(nafidha)) else {
        return;
    };
    hasad.aila(
        AilatMuharrik::Unity,
        format!(
            "{ism} contains the version stamp \"{khaam}\", which is the form Unity and only \
             Unity writes",
        ),
        Some(ism.to_owned()),
        WAZN_ISDAR_UNITY,
    );
    if let Some(isdar) = isdar_min_nass(&khaam, khaam.clone()) {
        hasad.isdar(isdar);
    }
}

/// Scans one window for the Unity version shape, in one walk.
///
/// Both encodings are tried at each position rather than in two passes, for the
/// reason [`ibhath_jami`] gives: a second walk of a sixteen-megabyte window buys
/// nothing that a second test at each position does not.
fn basmat_unity(nafidha: &[u8]) -> Option<String> {
    let mut mawdi = 0usize;
    while mawdi < nafidha.len() {
        for thunai in [false, true] {
            let khatwa = if thunai { 2 } else { 1 };
            let Some(bayt) = harf(nafidha, mawdi, thunai) else {
                continue;
            };
            if !bayt.is_ascii_digit() {
                continue;
            }
            // A stamp never starts in the middle of a longer number, so a digit
            // or a dot immediately before it disqualifies the position.
            let qabl = mawdi.checked_sub(khatwa).and_then(|sabiq| harf(nafidha, sabiq, thunai));
            if qabl.is_some_and(|sabiq| sabiq.is_ascii_digit() || sabiq == b'.') {
                continue;
            }
            if let Some(khaam) = iqra_basmat_unity(nafidha, mawdi, thunai) {
                return Some(khaam);
            }
        }
        mawdi = mawdi.saturating_add(1);
    }
    None
}

/// Reads one candidate stamp at a position, or rejects it.
fn iqra_basmat_unity(nafidha: &[u8], bidaya: usize, thunai: bool) -> Option<String> {
    let khatwa = if thunai { 2 } else { 1 };

    let (kabir, mawdi) = raqm(nafidha, bidaya, thunai, 4)?;
    if !isdar_unity_maqbul(kabir) {
        return None;
    }
    if harf(nafidha, mawdi, thunai)? != b'.' {
        return None;
    }
    let (sagheer, mawdi) = raqm(nafidha, mawdi.checked_add(khatwa)?, thunai, 2)?;
    if harf(nafidha, mawdi, thunai)? != b'.' {
        return None;
    }
    let (tasheeh, mawdi) = raqm(nafidha, mawdi.checked_add(khatwa)?, thunai, 3)?;

    // `f` final, `p` patch, `b` beta. The alpha stream is refused, and refusing
    // it is what this scan is worth: `X.Y.0a1` is how Unity spells the first
    // build of a release line, which is also how it spells the floor constants
    // it compiles into every player. `2018.3.0a1` sits in the serializer's
    // "written by a newer version of Unity" message table and `5.0.0a1` beside
    // the build-settings check, in the shipped runtime of a Unity 6 game and a
    // Unity 2022 game alike, both of them ahead of the build's own stamp in the
    // same section. A game shipped on an alpha is close to unheard of, and a
    // scan with no anchor cannot tell one from a constant, so the shape that
    // means both is not read as either.
    let naw = harf(nafidha, mawdi, thunai)?;
    if !matches!(naw, b'f' | b'p' | b'b') {
        return None;
    }
    let (tanqih, _) = raqm(nafidha, mawdi.checked_add(khatwa)?, thunai, 2)?;

    Some(format!("{kabir}.{sagheer}.{tasheeh}{}{tanqih}", char::from(naw)))
}

/// Whether a major version is one Unity has ever shipped.
///
/// 2018 stays in the range on purpose. Two real games misreported themselves as
/// Unity 2018 from this scan, and the year was never the wrong part — 2018.1
/// through 2018.4 are shipping release lines with shipping games behind them,
/// and dropping the year would trade one class of wrong answer for a worse one.
/// What was wrong was the alpha suffix, and that is refused where it is read.
const fn isdar_unity_maqbul(kabir: u32) -> bool {
    matches!(kabir, 3..=5 | 2017..=2035 | 6000..=6999)
}

/// Reads up to `aqsa` decimal digits, returning the value and the offset just
/// past them.
fn raqm(nafidha: &[u8], bidaya: usize, thunai: bool, aqsa: usize) -> Option<(u32, usize)> {
    let khatwa = if thunai { 2 } else { 1 };
    let mut qeema: u32 = 0;
    let mut adad = 0usize;
    let mut mawdi = bidaya;
    while adad < aqsa {
        let Some(bayt) = harf(nafidha, mawdi, thunai) else {
            break;
        };
        if !bayt.is_ascii_digit() {
            break;
        }
        qeema = qeema.checked_mul(10)?.checked_add(u32::from(bayt - b'0'))?;
        adad = adad.saturating_add(1);
        mawdi = mawdi.saturating_add(khatwa);
    }
    (adad > 0).then_some((qeema, mawdi))
}

// ---------------------------------------------------------------------------
// a pack appended to the executable
// ---------------------------------------------------------------------------

/// The magic a Godot export writes at the very end of an executable whose
/// content pack was appended to it.
const SIHR_DHAYL: [u8; 4] = *b"GDPC";

/// How many bytes of footer the check reads: an eight-byte length and the
/// four-byte magic.
const TUL_DHAYL: usize = 12;

/// Reads the last twelve bytes of the game's executable for a pack footer.
///
/// This is the shape [`crate::dalail::binya`] can only guess at. When a Godot
/// export embeds its pack there is no `.pck` file beside the executable and the
/// directory holds a single binary, which is also what a small native game
/// looks like — so the directory detector records the possibility and stops.
/// The footer settles it: pack bytes, then the pack's length as a
/// little-endian 64-bit integer, then this magic at end of file.
///
/// It settles only *that a pack is there*. The pack's own header — version 1
/// against version 2, which separates Godot 3 from Godot 4 and therefore
/// decides tier 2 against tier 1 — is inside the pack, and reading it belongs
/// to the container-header evidence source rather than to this one.
fn dhayl_godot(bayt: &[u8], ism: &str, hasad: &mut Hasad) {
    let Some(bidaya) = bayt.len().checked_sub(TUL_DHAYL) else {
        return;
    };
    let Some(dhayl) = bayt.get(bidaya..) else {
        return;
    };
    if dhayl.get(8..) != Some(SIHR_DHAYL.as_slice()) {
        return;
    }
    let Some(hajm) = dhayl.get(..8).and_then(|thamania| <[u8; 8]>::try_from(thamania).ok()) else {
        return;
    };
    // A pack cannot be zero bytes and cannot be longer than the file it is
    // appended to, so either says the magic landed on a coincidence.
    let hajm = u64::from_le_bytes(hajm);
    if hajm == 0 || u64::try_from(bidaya).is_ok_and(|hadd| hajm > hadd) {
        return;
    }

    hasad.aila(
        AilatMuharrik::Godot,
        format!(
            "the last bytes of {ism} are a Godot pack footer declaring a {hajm}-byte pack \
             appended to the executable, which is why no .pck file sits beside it",
        ),
        Some(ism.to_owned()),
        WAZN_DHAYL,
    );
}
