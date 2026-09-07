//! يونيتي — what Unity writes about itself, read out of Unity's own files.
//!
//! Every other Unity signal is circumstantial. A `*_Data` directory is a
//! directory name. `UnityPlayer.dll` in an import table is an import table.
//! Both are worth having and neither tells you the one thing that decides which
//! adapter runs and whether it can resolve anything once it is running: the
//! engine's own version, and how the game's code was compiled. Those two facts
//! live inside files Unity itself wrote, in formats Unity has never published,
//! and this module reads them.
//!
//! What it answers, in the order the answers matter:
//!
//! 1. **The engine version**, out of the header of `globalgamemanagers`,
//!    `data.unity3d`, or `mainData`. Phase 7's resolver is a three-rung ladder
//!    — generated metadata, runtime-export lookup, then binary signature
//!    scanning — and the signature database is keyed on
//!    `(engine version range, text system version range, architecture,
//!    platform)`. Without the version the third rung has nothing to select on,
//!    which means an IL2CPP game whose metadata this build cannot bind is
//!    unreachable rather than merely difficult.
//! 2. **The scripting backend**, which decides *which adapter binary loads at
//!    all* — Phase 6's Mono plugin or Phase 7's IL2CPP plugin. This is the only
//!    detector in Phase 5 where the answer changes the dispatch target rather
//!    than the confidence in it.
//! 3. **The IL2CPP metadata version**, an integer that is *not* derivable from
//!    the Unity version and that Phase 7's signature database is keyed on
//!    separately.
//! 4. **The text systems present**, which decide how much of the game Taarib
//!    can reach and therefore what the capability report is allowed to promise.
//!
//! It observes and does not conclude. [`crate::tahdid`] decides.
//!
//! ## Reading headers, not files
//!
//! `data.unity3d` is routinely a gigabyte and occasionally several.
//! `global-metadata.dat` is commonly 20–120 MB. `resources.assets` is whatever
//! the game's artists made it. Nothing here reads a whole one. Every read is a
//! window: an explicit offset, an explicit length, a bounds check before the
//! slice, and `.get()` rather than `[]` so that a truncated or hostile file
//! produces `None` and the evidence gathered so far, never a panic.
//!
//! Files are opened with `File::open` and seek-read rather than memory-mapped,
//! which is a deliberate departure from the crate's general preference. A probe
//! runs across a user's whole library, and a library is a place where Steam is
//! mid-update on one title while another is being scanned. A mapped page whose
//! backing file is truncated underneath it raises `SIGBUS`, which no `Result`
//! catches and which would take Studio down with it. Six bounded `pread`s per
//! game cost nothing measurable and cannot do that.
//!
//! ## The three byte layouts this module depends on
//!
//! None of these is documented by Unity. Each is stated here in full so that a
//! maintainer reading a wrong answer can check the layout against the file
//! rather than against the code, and so that the parts that are *inference*
//! rather than *fact* are visible as such.
//!
//! ### `SerializedFile` — `globalgamemanagers`, `mainData`, `*.assets`
//!
//! Header fields are **big-endian**, which is the single most common way to get
//! this format wrong; the object data that follows may be either, and the
//! endianness byte says which. Taarib never reads the data section.
//!
//! ```text
//! offset  size  field            notes
//! 0x00    4     metadataSize     u32 BE
//! 0x04    4     fileSize         u32 BE   whole file, in the classic header
//! 0x08    4     version          u32 BE   the *format* version, not Unity's
//! 0x0C    4     dataOffset       u32 BE
//! 0x10    1     endianness       present only when version >= 9
//! 0x11    3     reserved         present only when version >= 9
//! ```
//!
//! When `version >= 22` (Unity 2020.2 and later, the "large files" header) the
//! four fields above are written twice: the first copy is a legacy stub, almost
//! always zero, and the real values follow the reserved bytes as wider types.
//!
//! ```text
//! 0x14    4     metadataSize     u32 BE
//! 0x18    8     fileSize         i64 BE
//! 0x20    8     dataOffset       i64 BE
//! 0x28    8     unknown          i64 BE, observed zero
//! ```
//!
//! The Unity version string is the first thing in the metadata that follows,
//! NUL-terminated ASCII, and it is present when `version >= 7`:
//!
//! - `version >= 22` → the string starts at **0x30**.
//! - `9 <= version < 22` → the string starts at **0x14**.
//! - `7 <= version < 9` → the header is 16 bytes with no endianness byte, the
//!   metadata block sits at the *end* of the file, and the string starts at
//!   `fileSize - metadataSize + 1`. **This last case is reconstructed from how
//!   the open-source readers handle it and has not been checked here against a
//!   real Unity 3.x build.** It is implemented because it costs one extra
//!   bounded read and because reporting a version is worth more than tidiness;
//!   it is flagged as low confidence where it fires.
//! - `version < 7` → no version string exists. The container is still reported.
//!
//! ### `UnityFS` — `data.unity3d`
//!
//! A bundle, not a `SerializedFile`: a different format that happens to live at
//! the path a `SerializedFile` used to. Both occur, so both are tried, and the
//! signature decides which reader runs.
//!
//! ```text
//! offset  size  field                        notes
//! 0x00    var   signature                    NUL-terminated ASCII
//! ..      4     version                      u32 BE, the bundle format version
//! ..      var   unityVersion                 NUL-terminated, generation string
//! ..      var   unityRevision                NUL-terminated, the real version
//! ..      8     size                         i64 BE
//! ..      4     compressedBlocksInfoSize     u32 BE
//! ..      4     uncompressedBlocksInfoSize   u32 BE
//! ..      4     flags                        u32 BE
//! ```
//!
//! The signature is one of `UnityFS`, `UnityWeb`, `UnityRaw`, `UnityArchive`.
//! Only the two strings are read here; everything after them describes block
//! compression, which is the extractor's problem and not this module's.
//!
//! The two strings are not interchangeable. `unityVersion` is a *generation*
//! marker and is usually the literal placeholder `5.x.x`, which carries no
//! minor or patch. `unityRevision` is the build that wrote the bundle —
//! `2021.3.16f1`. The revision is tried first and the generation only as a
//! fallback, which is why a bundle whose revision is missing still yields a
//! major number instead of nothing.
//!
//! ### `global-metadata.dat` — the IL2CPP metadata header
//!
//! ```text
//! offset  size  field                    notes
//! 0x00    4     sanity                   u32 LE, 0xFAB11BAF
//! 0x04    4     version                  i32 LE, the metadata version
//! 0x08    4     stringLiteralOffset      i32 LE
//! 0x0C    4     stringLiteralSize        i32 LE, bytes, not entries
//! 0x10    4     stringLiteralDataOffset  i32 LE
//! 0x14    4     stringLiteralDataSize    i32 LE, bytes
//! 0x18    4     stringOffset             i32 LE
//! 0x1C    4     stringSize               i32 LE, bytes
//! ```
//!
//! Everything past 0x20 is a long run of further offset/size pairs — events,
//! properties, methods, parameters, fields, generic containers, type
//! definitions — whose *order changes between metadata versions*. This module
//! reads nothing past 0x20, and that is the point: **the first eight words have
//! held the same meaning at the same positions in every metadata version from
//! 16 through 31**, which is what makes a version-independent reader possible
//! at all. That stability is an observation across the published versions of
//! the il2cpp headers, not a guarantee from Unity, so every field read from it
//! is range-checked against the real file length before it is used.
//!
//! The two string regions are different things and the difference matters:
//!
//! - `stringOffset`/`stringSize` is the **identifier heap**: a flat run of
//!   NUL-terminated UTF-8 holding every namespace, type, method, field and
//!   parameter name that survived managed stripping. Type names live here. This
//!   is the region text-system detection scans.
//! - `stringLiteralDataOffset`/`stringLiteralDataSize` holds the **C# string
//!   literals** from the game's own code, indexed by the `(length, dataIndex)`
//!   pairs at `stringLiteralOffset`. Useful corroboration and nothing more: a
//!   literal mentioning a type name proves the source mentioned it, not that
//!   the type is in the build.
//!
//! ## What a miss means
//!
//! Nothing. Every scan here has a ceiling, so a needle that is not found may
//! simply be past it, and a text system that is not reported is not a text
//! system that is absent. The capability report is allowed to name what was
//! found; it is never allowed to promise that nothing else exists.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use taarib_mustalahat::muharrik::{
    AilatMuharrik, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel,
};
use taarib_usus::khata::Natija;

use crate::fahs::{AQSA_MADAKHIL, Fahis, HAJM_TARWISA, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

// ---------------------------------------------------------------------------
// Bounds. Every one of these is a ceiling, never a target.
// ---------------------------------------------------------------------------

/// Bytes taken from the front of an asset container.
///
/// A `SerializedFile` header is at most 0x30 bytes and the version string that
/// follows it is under thirty; a `UnityFS` header with both of its strings is
/// under 128. Four kibibytes is far above either and far below anything that
/// costs a page fault worth noticing. It is written against [`HAJM_TARWISA`]
/// rather than beside it, so the crate-wide ceiling is enforced by
/// construction and not by a comment that can go stale.
const HAJM_TARWISA_HAWIYA: usize = if HAJM_TARWISA < 4 * 1024 {
    HAJM_TARWISA
} else {
    4 * 1024
};

/// Bytes taken from the front of `global-metadata.dat`.
///
/// The eight words this module trusts end at 0x20. Two hundred and fifty-six
/// covers them with room for the file to be inspected in a hex dump next to the
/// layout documented above.
const HAJM_TARWISA_BAYANAT: usize = 256;

/// The most bytes one needle scan will read out of one file.
///
/// Deliberately larger than [`HAJM_TARWISA`], because a scan is not a header
/// read: the identifier heap of a large IL2CPP game is genuinely several
/// megabytes and the answer is not at its front. It is still a hard ceiling,
/// and where the file itself declares a smaller region the declared size wins.
const HAJM_MASAH: u64 = 8 * 1024 * 1024;

/// One chunk of a needle scan, so that scanning eight megabytes never holds
/// eight megabytes.
const HAJM_QITA: usize = 256 * 1024;

/// The overlap carried between scan chunks.
///
/// A needle straddling a chunk boundary would otherwise be missed, so each
/// chunk is re-examined with this many trailing bytes of its predecessor in
/// front of it. It must exceed the longest needle; the longest in this file is
/// `TextMeshProUGUI\0` at sixteen bytes.
const TADAKHUL_QITA: usize = 64;

/// The longest byte run accepted as a Unity version string.
///
/// Real strings are eleven or twelve bytes (`2021.3.16f1`, `6000.0.23f1`).
/// Custom builds append a suffix — `2020.3.33f1-DWP`, `2021.3.16f1c1` for the
/// Chinese fork — and thirty-two holds every one of those while refusing to
/// treat a run of arbitrary binary as text.
const AQSA_ISDAR: usize = 32;

/// Bytes read from a small text file that is read whole.
///
/// `app.info` is two lines. `boot.config` is a few dozen. Eight kibibytes is a
/// ceiling for the case where one of them is not what its name says.
const HAJM_NASS: u64 = 8 * 1024;

/// The largest `SerializedFile` format version this reader will believe.
///
/// Real values run 5 through 22. A file claiming a hundred is not a
/// `SerializedFile`, and refusing it here is what stops a random binary from
/// producing a "version string" out of its own bytes.
const AQSA_SIGHAT_HAWIYA: u32 = 99;

// ---------------------------------------------------------------------------
// Weights.
//
// Named rather than written inline, so that the reasoning is reviewable in one
// place and so that changing what an observation is worth is a one-line change
// with a comment attached to it. The scale is the one documented on
// `HasilatFahs::sajjil`: a file only one engine ever ships is 90 and up, a file
// several engines share is 40 to 60, a name that merely suggests something is
// below 30.
// ---------------------------------------------------------------------------

/// `global-metadata.dat` opens with `0xFAB11BAF`.
///
/// The strongest single observation this detector can make, and the strongest
/// in the whole phase. It is a 32-bit constant at offset zero of a file at a
/// fixed path under a directory only Unity's IL2CPP build pipeline creates.
/// Nothing else writes that number there. It is 97 rather than 100 only
/// because a detector is not permitted to conclude, and because a file can be
/// copied into a directory by something other than the build that made it.
const WAZN_SIHR_BAYANAT: u8 = 97;

/// A container header parsed and a plausible Unity version string recovered
/// from immediately behind it.
///
/// Near-conclusive because two independent things had to agree: the header's
/// own fields had to be self-consistent and inside the file, and the bytes at
/// the position the format puts the version string at had to be a version
/// string. Random data does not do both.
const WAZN_ISDAR_HAWIYA: u8 = 95;

/// `Managed/Assembly-CSharp.dll`.
///
/// Conclusive for the backend and silent about everything else. It says the
/// game's own code is managed and on disk, which is exactly the fact that
/// selects Phase 6's adapter over Phase 7's, and it says nothing at all about
/// which Unity built it.
const WAZN_ASSEMBLY_CSHARP: u8 = 93;

/// A Unity container header recognised at offset zero, with no version string
/// behind it.
///
/// Either a `UnityFS`-family signature or a `SerializedFile` header whose
/// fields are self-consistent and inside the file. Recognising the container
/// and failing to read a version out of it is the normal outcome for a build
/// old enough to predate the embedded string, and it is still worth 92: the
/// container is Unity's and nothing else writes one.
const WAZN_TAWQI_HAWIYA: u8 = 92;

/// A text system named by a managed assembly that only that system ships.
const WAZN_ITAR_MUJAMMA: u8 = 90;

/// `GameAssembly.dll`, `.so` or `.dylib` beside the executable.
///
/// A Unity-specific name, and only a name: no byte of it has been read at this
/// point. Strong enough to matter, deliberately weaker than the metadata magic
/// that confirms it.
const WAZN_GAME_ASSEMBLY: u8 = 88;

/// The metadata version integer, once the magic has already been accepted.
///
/// Weighted for what it is worth downstream rather than for how hard it was to
/// read: this integer is a key into Phase 7's signature database, and an
/// IL2CPP game whose metadata version is unknown to the interop layer is
/// precisely the game the third rung of the resolver exists for.
const WAZN_ISDAR_BAYANAT: u8 = 85;

/// A `MonoBleedingEdge` or `Mono` runtime directory, in either of the two
/// places a player build puts it. [`mawqi_mono`] says where those are.
const WAZN_MONO_BLEEDING: u8 = 82;

/// The `*_Data` directory itself.
///
/// Strong shape evidence, and knowingly overlapping with `FahisBinya`. It is
/// recorded here anyway because this detector has to locate the directory
/// before it can read anything at all, and an observation that was made and
/// then dropped is an observation missing from the diagnostics bundle.
const WAZN_MUJALLAD_BAYANAT: u8 = 80;

/// `Managed/mscorlib.dll` with no `Assembly-CSharp.dll` beside it.
///
/// Mono, with the game's code in an assembly under some other name — which
/// happens, and which the Mono adapter handles, so the backend answer is still
/// worth reporting.
const WAZN_MSCORLIB: u8 = 78;

/// `Resources/unity_builtin_extra` or `Resources/unity default resources`.
const WAZN_BUILTIN_EXTRA: u8 = 75;

/// `global-metadata.dat` exists and does not begin with the expected magic.
///
/// Still strong evidence of IL2CPP, because the path is Unity's own and
/// nothing else puts a file there. What it is *not* is readable: the metadata
/// has been encrypted or restructured by a protector, and the value of this
/// observation is that it tells Phase 7 to skip the first two rungs of the
/// resolver rather than failing on them.
const WAZN_BAYANAT_MUBHAMA: u8 = 72;

/// An exact NUL-terminated type name found in the IL2CPP identifier heap.
///
/// Not conclusive: the heap holds names that survived managed stripping, which
/// includes names referenced but never instantiated. It is real evidence that
/// the type is in the build and weak evidence that the game draws with it.
const WAZN_IBARA_BAYANAT: u8 = 70;

/// The same type name found by scanning `Assembly-CSharp.dll` as raw bytes.
///
/// Weaker than the heap hit on purpose. This scan does not parse the CLI
/// metadata and therefore cannot tell a name in the `#Strings` heap from the
/// same characters inside a string literal or an embedded resource. It is how
/// NGUI and `FairyGUI` are found at all — both are distributed as source and
/// compile into the game's own assembly, leaving no file to name them.
const WAZN_IBARA_MUJAMMA: u8 = 62;

/// Both scripting backends present in one install.
///
/// Recorded as its own observation rather than resolved here. A broken
/// install, a mid-migration build, or a Mono game with an IL2CPP directory
/// left behind by a previous version are all real, they are not the same
/// thing, and choosing between them is `tahdid`'s job.
const WAZN_TANAQUD: u8 = 60;

/// `TextMeshPro` corroborated by a resource name rather than by an assembly.
const WAZN_TMP_MAWARID: u8 = 55;

/// `*_Data/boot.config`.
const WAZN_BOOT_CONFIG: u8 = 50;

/// `*_Data/app.info`.
///
/// The weakest observation this detector makes, and kept for what it adds
/// rather than what it proves: two lines of text, company name then product
/// name, written by the player build. The name is not reserved and the format
/// is not distinctive, so it corroborates and never decides.
const WAZN_APP_INFO: u8 = 45;

/// An engine module assembly that most builds ship regardless of what the game
/// does with it.
const WAZN_WAHDA_NASS: u8 = 35;

/// A type name found among the game's own C# string literals.
///
/// Below the line where an observation means anything on its own, and kept
/// there deliberately. The literal blob holds constants the game's source
/// wrote; a constant naming a type proves the source mentioned the name, not
/// that the type survived stripping or that anything draws with it.
const WAZN_IQTIBAS: u8 = 28;

/// `UnityEngine.IMGUIModule.dll`.
///
/// Present in very nearly every Unity build, which is what makes it almost
/// worthless as a discriminator. It is recorded because IMGUI text is text a
/// user can see, and it maps to no [`ItarNusus`] because Taarib has no IMGUI
/// takeover to promise.
const WAZN_IMGUI: u8 = 15;

// ---------------------------------------------------------------------------
// The detector
// ---------------------------------------------------------------------------

/// Reads Unity's own files rather than guessing from the directory layout.
///
/// The engine version comes out of a container header, the scripting backend
/// out of the install's shape, the IL2CPP metadata version out of
/// `global-metadata.dat`, and the text systems out of whichever of the managed
/// assemblies or the identifier heap the build actually has.
#[derive(Debug, Clone, Copy, Default)]
pub struct FahisUnity;

impl Fahis for FahisUnity {
    fn ism(&self) -> &'static str {
        "unity"
    }

    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        if !siyaq.jidhr.exists() {
            return Err(KhataMuharrik::JidhrMafqud {
                jidhr: siyaq.jidhr.to_path_buf(),
            }
            .into());
        }

        let mut hasila = HasilatFahs::la_shay();
        let bayanat = mawqi_bayanat(siyaq, &mut hasila)?;

        // The backend is read before the version because it is the answer that
        // changes which adapter runs, and because the metadata header and the
        // `Managed` listing it produces are reused by the text-system stage
        // rather than read a second time.
        let qiraa = khalfiya(siyaq, bayanat.as_ref(), &mut hasila);

        if let Some(bayanat) = bayanat.as_ref() {
            isdar(siyaq, bayanat, &mut hasila);
            anzimat_nusus(siyaq, &qiraa, &mut hasila);
            qarain(siyaq, bayanat, &mut hasila);
        }

        Ok(hasila)
    }
}

// ---------------------------------------------------------------------------
// Locating `*_Data`
// ---------------------------------------------------------------------------

/// The game's data directory, once found, with its entries already listed.
///
/// Listed once and passed down, because every stage after this one asks the
/// same directory a different question and `read_dir` on a network share is not
/// free. The index is case-insensitive for the reason
/// [`SiyaqFahs::yujad`] is: `data.unity3d` and `Data.unity3d` are the same file
/// to a Windows game and two different files to a Linux filesystem.
#[derive(Debug)]
struct MujalladBayanat {
    /// The directory's path relative to the game root, as evidence records it.
    /// Every absolute path below it is rebuilt through [`SiyaqFahs::dakhil`]
    /// from this string, so containment is checked once per path rather than
    /// assumed after the first join.
    nisbi: String,
    /// What is in it.
    fahras: Fahras,
}

impl MujalladBayanat {
    /// A path inside the data directory, relative to the game root.
    fn tahta(&self, dhayl: &str) -> String {
        format!("{}/{dhayl}", self.nisbi)
    }
}

/// Finds the game's data directory and records having found it.
///
/// Three routes, tried in order of exactness. The first is the only rule Unity
/// actually guarantees: the player build names the directory after the player
/// *binary*, so `Game.exe` is always beside `Game_Data`. The second covers
/// macOS, where the binary lives inside a bundle and the directory drops its
/// prefix entirely. The third lists the root, which is what happens when
/// discovery named no executable, when the user renamed one, or when the game
/// ships a launcher whose name is not the player's.
///
/// # Errors
///
/// Only when the root itself cannot be listed, which is the one condition the
/// [`Fahis`] contract treats as fatal. Everything below the root degrades into
/// the absence of evidence.
fn mawqi_bayanat(
    siyaq: &SiyaqFahs<'_>,
    hasila: &mut HasilatFahs,
) -> Natija<Option<MujalladBayanat>> {
    if let Some(tanfidhi) = siyaq.tanfidhi
        && let Some(jidhr_ism) = tanfidhi.file_stem().and_then(OsStr::to_str)
        && let Some(bayanat) = jarrib_bayanat(siyaq, &format!("{jidhr_ism}_Data"))
    {
        sajjil_bayanat(&bayanat, hasila);
        return Ok(Some(bayanat));
    }

    if let Some(bayanat) = jarrib_bayanat(siyaq, "Contents/Resources/Data") {
        sajjil_bayanat(&bayanat, hasila);
        return Ok(Some(bayanat));
    }

    let jidhr =
        fahras_mujallad(siyaq.jidhr).map_err(|sabab| KhataMuharrik::TaadhurQiraatJidhr {
            jidhr: siyaq.jidhr.to_path_buf(),
            sabab,
        })?;

    let murashahat = jidhr.mujalladat_bi_lahiqa("_data");
    if let Some(mukhtar) = ikhtar_bayanat(&murashahat, siyaq.ism) {
        let Some(bayanat) = jarrib_bayanat(siyaq, mukhtar) else {
            return Ok(None);
        };
        sajjil_bayanat(&bayanat, hasila);
        if murashahat.len() > 1 {
            hasila.sajjil(
                NawDaleel::BinyatMujallad,
                format!(
                    "{} data directories under one root: {}. The one matching the game's own \
                     name was taken; the others are reported so a wrong choice is visible.",
                    murashahat.len(),
                    murashahat.join(", ")
                ),
                None,
                0,
            );
        }
        istantij_tanfidhi(siyaq, &jidhr, mukhtar, hasila);
        return Ok(Some(bayanat));
    }

    for hazma in jidhr.mujalladat_bi_lahiqa(".app") {
        let nisbi = format!("{hazma}/Contents/Resources/Data");
        if let Some(bayanat) = jarrib_bayanat(siyaq, &nisbi) {
            sajjil_bayanat(&bayanat, hasila);
            return Ok(Some(bayanat));
        }
    }

    Ok(None)
}

/// Opens a candidate data directory, listing it if it is there.
fn jarrib_bayanat(siyaq: &SiyaqFahs<'_>, nisbi: &str) -> Option<MujalladBayanat> {
    let masar = siyaq.dakhil(nisbi).ok()?;
    if !masar.is_dir() {
        return None;
    }
    let mahtawa = fahras_mujallad(&masar).unwrap_or_default();
    Some(MujalladBayanat {
        nisbi: nisbi.to_owned(),
        fahras: mahtawa,
    })
}

/// Picks between several `*_Data` directories under one root.
///
/// More than one is not rare: a game that ships its own launcher, a demo left
/// beside the full build, a Unity-built installer next to the game it
/// installed. The one whose prefix matches the game's own name is the game's;
/// with no match, the first is taken and the alternatives are reported.
fn ikhtar_bayanat<'a>(murashahat: &[&'a str], ism: &str) -> Option<&'a str> {
    let matlub = mubassat(ism);
    if !matlub.is_empty() {
        for murashah in murashahat {
            let hadd = murashah.len().saturating_sub("_data".len());
            let Some(bidaya) = murashah.get(..hadd) else {
                continue;
            };
            if mubassat(bidaya) == matlub {
                return Some(murashah);
            }
        }
    }
    murashahat.first().copied()
}

/// A name reduced to its ASCII alphanumerics in lower case, so that
/// `My Game_Data` and `MyGame.exe` compare equal.
fn mubassat(ism: &str) -> String {
    ism.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|harf| harf.to_ascii_lowercase())
        .collect()
}

/// Names the player binary that goes with a data directory, when discovery did
/// not name one.
///
/// The inverse of the rule that found the directory, and worth doing because
/// `X_Data` beside `X.exe` is the most reliable statement about a Unity game's
/// executable that exists — more reliable than picking the largest binary or
/// the one matching the store's title, which is what a generic guess would do.
fn istantij_tanfidhi(
    siyaq: &SiyaqFahs<'_>,
    jidhr: &Fahras,
    mujallad: &str,
    hasila: &mut HasilatFahs,
) {
    if siyaq.tanfidhi.is_some() {
        return;
    }
    let Some(bidaya) = mujallad.get(..mujallad.len().saturating_sub("_data".len())) else {
        return;
    };
    // Lowered before the lookup, because the index is keyed on the lowered
    // name and `bidaya` came off the directory with its own spelling intact.
    let khafid = bidaya.to_ascii_lowercase();
    for lahiqa in [".exe", ".x86_64", ".x86", ""] {
        let murashah = format!("{khafid}{lahiqa}");
        if let Some(haqiqi) = jidhr.malaf(&murashah)
            && let Ok(masar) = siyaq.dakhil(haqiqi)
        {
            hasila.tanfidhi = Some(masar);
            hasila.sajjil(
                NawDaleel::BinyatMujallad,
                format!("player binary `{haqiqi}` named by the data directory beside it"),
                Some(haqiqi.to_owned()),
                WAZN_MUJALLAD_BAYANAT,
            );
            return;
        }
    }
}

/// Records the data directory, and records what could not be read about it.
fn sajjil_bayanat(bayanat: &MujalladBayanat, hasila: &mut HasilatFahs) {
    hasila.sajjil_aila(
        AilatMuharrik::Unity,
        NawDaleel::BinyatMujallad,
        format!("Unity player data directory `{}`", bayanat.nisbi),
        Some(bayanat.nisbi.clone()),
        WAZN_MUJALLAD_BAYANAT,
    );
    if !bayanat.fahras.maqru {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "the data directory is there and could not be listed, so nothing inside it was \
             examined and the absence of any finding below means nothing",
            Some(bayanat.nisbi.clone()),
            0,
        );
    } else if bayanat.fahras.qutia {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "the data directory holds more than {AQSA_MADAKHIL} entries; the listing \
                 stopped there and anything past it was not examined"
            ),
            Some(bayanat.nisbi.clone()),
            0,
        );
    }
}

// ---------------------------------------------------------------------------
// The engine version
// ---------------------------------------------------------------------------

/// The containers tried for a version string, in descending order of how
/// likely they are to be both present and cheap.
///
/// Names are lower case because [`Fahras::malaf`] matches on the lowered form.
///
/// `globalgamemanagers` first: it is a bare `SerializedFile`, it is small, and
/// its version string is the one Unity's own player reads. `data.unity3d`
/// second, because a build made with the compressed-player option has no
/// `globalgamemanagers` on disk at all — it is inside the bundle — and the
/// bundle header carries the version anyway. `maindata` is the Unity 4 and
/// earlier spelling. The rest are fallbacks that carry the same string and are
/// only reached when the first three are missing or unreadable.
const HAWIYAT: &[&str] = &[
    "globalgamemanagers",
    "data.unity3d",
    "maindata",
    "globalgamemanagers.assets",
    "level0",
    "resources.assets",
    "sharedassets0.assets",
];

/// Signatures a Unity bundle can open with.
const TAWQIAT_HAZMA: &[&str] = &["UnityFS", "UnityWeb", "UnityRaw", "UnityArchive"];

/// What one container header gave up.
#[derive(Debug)]
struct QiraatHawiya {
    /// The container, described for the evidence line.
    wasf: String,
    /// The version string exactly as it was on disk, when there was one.
    khaam: Option<String>,
    /// Whether the string's position was the reconstructed pre-format-9 one
    /// rather than a position this reader is sure of.
    mustanbat: bool,
}

/// Reads the engine version out of whichever container has it.
///
/// Every container that is recognised is recorded, whether or not it yielded a
/// version, because "this is a Unity container and its version string was not
/// where the format puts it" is a diagnosable statement and silence is not.
/// The search stops at the first version that parses; an unparseable string is
/// still recorded verbatim and the next container is tried.
fn isdar(siyaq: &SiyaqFahs<'_>, bayanat: &MujalladBayanat, hasila: &mut HasilatFahs) {
    for ism in HAWIYAT {
        let Some(haqiqi) = bayanat.fahras.malaf(ism) else {
            continue;
        };
        let nisbi = bayanat.tahta(haqiqi);
        let Ok(masar) = siyaq.dakhil(&nisbi) else {
            continue;
        };
        let Some(nafidha) = iqra_nafidha(&masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(qiraa) = iqra_hawiya(&masar, &nafidha) else {
            continue;
        };

        let Some(khaam) = qiraa.khaam.as_deref().filter(|nass| yushbih_isdar(nass)) else {
            let dhayl = qiraa.khaam.as_deref().map_or_else(String::new, |nass| {
                format!(", and what stands where the version does is `{nass}`")
            });
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::TarwisatHawiya,
                format!(
                    "{}, with no engine version behind its header{dhayl}",
                    qiraa.wasf
                ),
                Some(nisbi),
                WAZN_TAWQI_HAWIYA,
            );
            continue;
        };

        let tanbih = if qiraa.mustanbat {
            ", read from the tail-metadata position used by pre-format-9 files, which this \
             build has never verified against a real one"
        } else {
            ""
        };
        hasila.sajjil_aila(
            AilatMuharrik::Unity,
            NawDaleel::BayanatMudmaja,
            format!(
                "engine version `{khaam}` in the header of a {}{tanbih}",
                qiraa.wasf
            ),
            Some(nisbi),
            if qiraa.mustanbat {
                WAZN_TAWQI_HAWIYA
            } else {
                WAZN_ISDAR_HAWIYA
            },
        );

        if let Some(mufassal) = hallil_isdar(khaam) {
            hasila.isdar = Some(mufassal);
            return;
        }

        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            format!(
                "the version string `{khaam}` does not split into major.minor, so it is kept \
                 as written and no numeric version is claimed from it"
            ),
            None,
            0,
        );
    }
}

/// Reads whichever container is actually at a path.
///
/// The file name does not decide the format and never has: `data.unity3d` is a
/// bundle in every build that ships one, `globalgamemanagers` is a bare
/// `SerializedFile` in every build that ships one, and a game that swapped them
/// would still be a game. The signature decides, and where the signature is
/// not one of the four known bundle spellings the `SerializedFile` reader still
/// gets its turn — its own field checks are strict enough to say no.
fn iqra_hawiya(masar: &Path, nafidha: &[u8]) -> Option<QiraatHawiya> {
    if nafidha.starts_with(b"Unity")
        && let Some(qiraa) = iqra_hazma(nafidha)
    {
        return Some(qiraa);
    }
    iqra_serialized(masar, nafidha)
}

/// Reads a `SerializedFile` header.
///
/// The layout, and which of the three version-string positions applies, is
/// documented at the top of this module. Every field is validated against
/// every other before any of it is trusted: metadata cannot be larger than the
/// file that contains it, the data section cannot start past the end, and the
/// endianness byte is one of two values. Those checks are what stop an
/// arbitrary binary from producing a plausible-looking version string out of
/// its own bytes.
fn iqra_serialized(masar: &Path, nafidha: &[u8]) -> Option<QiraatHawiya> {
    let hajm_bayanat = raqm32_kabir(nafidha, 0x00)?;
    let hajm_malaf = raqm32_kabir(nafidha, 0x04)?;
    let sigha = raqm32_kabir(nafidha, 0x08)?;
    let izahat_bayanat = raqm32_kabir(nafidha, 0x0C)?;

    if sigha == 0 || sigha > AQSA_SIGHAT_HAWIYA {
        return None;
    }
    let wasf = format!("SerializedFile container, format version {sigha}");

    if sigha >= 22 {
        let hajm_kabir = raqm32_kabir(nafidha, 0x14)?;
        let tul_kabir = raqm64_kabir(nafidha, 0x18)?;
        let izaha_kabira = raqm64_kabir(nafidha, 0x20)?;
        if tul_kabir <= 0 || izaha_kabira < 0 || i64::from(hajm_kabir) > tul_kabir {
            return None;
        }
        let khaam = nass_muntahi(nafidha, 0x30, AQSA_ISDAR);
        return Some(QiraatHawiya {
            wasf,
            khaam,
            mustanbat: false,
        });
    }

    if hajm_malaf == 0 || hajm_bayanat > hajm_malaf || izahat_bayanat > hajm_malaf {
        return None;
    }
    if sigha < 7 {
        return Some(QiraatHawiya {
            wasf,
            khaam: None,
            mustanbat: false,
        });
    }
    if sigha >= 9 {
        if nafidha.get(0x10).copied()? > 1 {
            return None;
        }
        let khaam = nass_muntahi(nafidha, 0x14, AQSA_ISDAR);
        return Some(QiraatHawiya {
            wasf,
            khaam,
            mustanbat: false,
        });
    }

    // Formats 7 and 8 keep their metadata at the end of the file, behind the
    // single endianness byte. One more bounded read, at a computed offset that
    // the checks above have already confined to inside the file.
    let izaha = u64::from(hajm_malaf)
        .checked_sub(u64::from(hajm_bayanat))?
        .checked_add(1)?;
    let dhayl = iqra_nafidha(masar, izaha, AQSA_ISDAR.saturating_add(1))?;
    let khaam = nass_muntahi(&dhayl, 0, AQSA_ISDAR);
    Some(QiraatHawiya {
        wasf,
        khaam,
        mustanbat: true,
    })
}

/// Reads a `UnityFS`-family bundle header.
///
/// Only the signature, the format version and the two version strings. The
/// block and directory information behind them is compressed and is the
/// extractor's problem; this module never needs it and never reads it, which is
/// why a two-gigabyte bundle costs one four-kilobyte read.
fn iqra_hazma(nafidha: &[u8]) -> Option<QiraatHawiya> {
    let (tawqi, baad_tawqi) = nass_muntahi_maa_mawqi(nafidha, 0, 16)?;
    if !TAWQIAT_HAZMA.contains(&tawqi.as_str()) {
        return None;
    }
    let sigha = raqm32_kabir(nafidha, baad_tawqi)?;
    let bidayat_jeel = baad_tawqi.checked_add(4)?;
    let (jeel, baad_jeel) = nass_muntahi_maa_mawqi(nafidha, bidayat_jeel, AQSA_ISDAR)?;
    let (muraja, _) = nass_muntahi_maa_mawqi(nafidha, baad_jeel, AQSA_ISDAR)?;
    let wasf = format!("`{tawqi}` bundle, format version {sigha}");

    // The revision is the build that wrote the bundle; the generation string is
    // usually the placeholder `5.x.x`. Prefer whichever actually parses, and
    // fall back to keeping the revision verbatim so the raw string survives.
    let khaam = if hallil_isdar(&muraja).is_some() {
        Some(muraja)
    } else if hallil_isdar(&jeel).is_some() {
        Some(jeel)
    } else if yushbih_isdar(&muraja) {
        Some(muraja)
    } else {
        None
    };
    Some(QiraatHawiya {
        wasf,
        khaam,
        mustanbat: false,
    })
}

/// Whether a run of printable ASCII is shaped like a Unity version at all.
///
/// Deliberately loose. It rejects a container's stray bytes and accepts every
/// version string Unity has ever written, including the ones this build has
/// never seen, because a version this build cannot classify is still the most
/// valuable single fact about the game.
fn yushbih_isdar(khaam: &str) -> bool {
    khaam.starts_with(|harf: char| harf.is_ascii_digit()) && khaam.contains('.')
}

/// Splits a Unity version string into numbers, keeping the original.
///
/// `2021.3.16f1` becomes `(2021, 3, 16)`, `6000.0.23f1` becomes
/// `(6000, 0, 23)`, `5.6.7f1` becomes `(5, 6, 7)`. A trailing patch component
/// that is missing entirely defaults to zero; a major or minor that is not
/// numeric fails, which is exactly what rejects the `5.x.x` generation
/// placeholder without special-casing it.
///
/// There is no table of known versions anywhere in this function, and there
/// must not be: the adapters are built to take a version they have never seen
/// and fall back to signature scanning, and a parser that only accepted
/// versions this build shipped with would take that away from them.
fn hallil_isdar(khaam: &str) -> Option<IsdarMuharrik> {
    let munaqqa = khaam.trim();
    if munaqqa.is_empty() || munaqqa.len() > AQSA_ISDAR {
        return None;
    }
    let mut ajzaa = munaqqa.split('.');
    let kabir = raqm_bidaya(ajzaa.next()?)?;
    let sagheer = raqm_bidaya(ajzaa.next()?)?;
    let tasheeh = ajzaa.next().and_then(raqm_bidaya).unwrap_or(0);
    Some(IsdarMuharrik {
        kabir,
        sagheer,
        tasheeh,
        khaam: munaqqa.to_owned(),
        mushtaqq: false,
    })
}

/// The leading run of ASCII digits in a version component.
fn raqm_bidaya(juz: &str) -> Option<u16> {
    let arqam: String = juz.chars().take_while(char::is_ascii_digit).collect();
    if arqam.is_empty() {
        return None;
    }
    arqam.parse::<u16>().ok()
}

// ---------------------------------------------------------------------------
// The scripting backend
// ---------------------------------------------------------------------------

/// The magic `global-metadata.dat` opens with.
///
/// Little-endian, so the bytes on disk are `AF 1B B1 FA`. Nothing else uses
/// it, which is why it is the strongest observation in this file.
const SIHR_BAYANAT: u32 = 0xFAB1_1BAF;

/// Names the IL2CPP native library ships under, across platforms.
///
/// `GameAssembly` is Unity's desktop naming on all three; `libil2cpp` is the
/// Android and embedded spelling, kept because a game directory copied off a
/// device is still a game directory somebody will point Taarib at.
const MAKTABAT_IL2CPP: &[&str] = &[
    "GameAssembly.dll",
    "GameAssembly.so",
    "GameAssembly.dylib",
    "libil2cpp.so",
];

/// What was found where `global-metadata.dat` should be.
#[derive(Debug)]
enum HalatBayanat {
    /// The magic matched and the header was read.
    Maqrua(TarwisatBayanat),
    /// The file is there and does not open with the magic. Encrypted,
    /// restructured by a protector, or not the file its name claims.
    Mubhama {
        /// The first four bytes, as they were.
        sihr: u32,
    },
    /// The file is there and not one byte of it could be read.
    Mughlaqa,
}

/// The eight words at the front of `global-metadata.dat` that this build
/// trusts, and where they point.
#[derive(Debug)]
struct TarwisatBayanat {
    /// Path relative to the game root, for the evidence line.
    nisbi: String,
    /// Absolute path, for the heap scans.
    masar: PathBuf,
    /// The metadata version integer, as written.
    isdar: i32,
    /// The identifier heap — `(offset, size)` — when its declared bounds are
    /// inside the file. Namespace, type, method and field names live here.
    asmaa: Option<(u64, u64)>,
    /// The string-literal data blob, on the same condition. The game's own C#
    /// string constants.
    iqtibasat: Option<(u64, u64)>,
}

/// Everything the backend stage found that a later stage needs again.
#[derive(Debug, Default)]
struct QiraatKhalfiya {
    /// The IL2CPP metadata header, when there was one and it was readable.
    bayanat: Option<TarwisatBayanat>,
    /// The `Managed` directory's relative path and listing, when there is one.
    mudara: Option<(String, Fahras)>,
}

/// Determines how the game's code runs, and refuses to guess when the install
/// says two things at once.
///
/// This is the one answer in Phase 5 that changes which adapter binary loads
/// rather than how confident the report is, so both families of evidence are
/// gathered in full before either is acted on. An install carrying both is a
/// real thing — a mid-migration build, a Mono game with an `il2cpp_data`
/// directory left behind by a previous version, a half-finished copy — and
/// those are not the same fault. Naming one silently would send an adapter at
/// a game it cannot patch and produce a failure with no evidence behind it.
fn khalfiya(
    siyaq: &SiyaqFahs<'_>,
    bayanat: Option<&MujalladBayanat>,
    hasila: &mut HasilatFahs,
) -> QiraatKhalfiya {
    let mut qiraa = QiraatKhalfiya::default();
    let mut wazn_il2cpp = 0_u8;
    let mut wazn_mono = 0_u8;

    for ism in MAKTABAT_IL2CPP {
        if siyaq.yujad(ism) {
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::BinyatMujallad,
                format!("IL2CPP native library `{ism}` beside the player"),
                Some((*ism).to_owned()),
                WAZN_GAME_ASSEMBLY,
            );
            wazn_il2cpp = wazn_il2cpp.max(WAZN_GAME_ASSEMBLY);
            break;
        }
    }

    // macOS keeps it in the bundle's Frameworks directory rather than beside
    // the data directory, so it is looked for relative to the bundle root the
    // data directory already told us.
    if wazn_il2cpp == 0
        && let Some(bayanat) = bayanat
        && let Some(jidhr_hazma) = bayanat.nisbi.strip_suffix("Contents/Resources/Data")
    {
        let nisbi = format!("{jidhr_hazma}Contents/Frameworks/GameAssembly.dylib");
        if siyaq.yujad(&nisbi) {
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::BinyatMujallad,
                "IL2CPP native library `GameAssembly.dylib` in the player's bundle",
                Some(nisbi),
                WAZN_GAME_ASSEMBLY,
            );
            wazn_il2cpp = wazn_il2cpp.max(WAZN_GAME_ASSEMBLY);
        }
    }

    if let Some(bayanat) = bayanat {
        wazn_il2cpp = wazn_il2cpp.max(sajjil_bayanat_il2cpp(siyaq, bayanat, &mut qiraa, hasila));
        wazn_mono = wazn_mono.max(sajjil_mono(siyaq, bayanat, &mut qiraa, hasila));
    }

    match (wazn_il2cpp > 0, wazn_mono > 0) {
        (true, true) => hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "both scripting backends are present in one install — IL2CPP evidence at \
                 weight {wazn_il2cpp}, Mono evidence at weight {wazn_mono}. Neither is \
                 claimed here. A broken install, a build caught mid-migration, and an \
                 IL2CPP directory left behind by a previous version all look like this, \
                 and sending the wrong adapter at any of them fails with no evidence."
            ),
            None,
            WAZN_TANAQUD,
        ),
        (true, false) => hasila.khalfiya = Some(KhalfiyaBarmajiya::Il2cpp),
        (false, true) => hasila.khalfiya = Some(KhalfiyaBarmajiya::Mono),
        (false, false) => {},
    }

    qiraa
}

/// Looks for `il2cpp_data/Metadata/global-metadata.dat`, reads its header, and
/// records what it found. Returns the weight of the strongest IL2CPP
/// observation made, or zero for none.
fn sajjil_bayanat_il2cpp(
    siyaq: &SiyaqFahs<'_>,
    bayanat: &MujalladBayanat,
    qiraa: &mut QiraatKhalfiya,
    hasila: &mut HasilatFahs,
) -> u8 {
    let ajzaa = ["il2cpp_data", "metadata", "global-metadata.dat"];
    let Some((nisbi, masar)) = masar_tahta(siyaq, bayanat, &ajzaa) else {
        return 0;
    };

    match halat_bayanat(nisbi.clone(), masar) {
        HalatBayanat::Maqrua(tarwisa) => {
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::BayanatMudmaja,
                format!("IL2CPP metadata magic {SIHR_BAYANAT:#010X} at offset zero"),
                Some(nisbi.clone()),
                WAZN_SIHR_BAYANAT,
            );
            let madda = madda_isdar_bayanat(tarwisa.isdar).map_or_else(
                || {
                    ". This build has no Unity range for that number, which is itself the \
                     useful part: the IL2CPP adapter should go straight to binary signature \
                     scanning rather than spend two rungs failing to bind generated metadata."
                        .to_owned()
                },
                |madda| format!(", which corresponds approximately to {madda}"),
            );
            hasila.sajjil(
                NawDaleel::BayanatMudmaja,
                format!("IL2CPP metadata version {}{madda}", tarwisa.isdar),
                Some(nisbi),
                WAZN_ISDAR_BAYANAT,
            );
            qiraa.bayanat = Some(tarwisa);
            WAZN_SIHR_BAYANAT
        },
        HalatBayanat::Mubhama { sihr } => {
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::BayanatMudmaja,
                format!(
                    "IL2CPP metadata present and unreadable: it opens with {sihr:#010X} \
                     rather than {SIHR_BAYANAT:#010X}, so it has been encrypted or \
                     restructured. The path is Unity's own, so the backend answer stands; \
                     what is lost is the metadata version and every type name in it, and \
                     the adapter must resolve by binary signature alone."
                ),
                Some(nisbi),
                WAZN_BAYANAT_MUBHAMA,
            );
            WAZN_BAYANAT_MUBHAMA
        },
        HalatBayanat::Mughlaqa => {
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::BinyatMujallad,
                "IL2CPP metadata file present and not readable at all",
                Some(nisbi),
                WAZN_BAYANAT_MUBHAMA,
            );
            WAZN_BAYANAT_MUBHAMA
        },
    }
}

/// Records the Mono side of the install. Returns the strongest weight, or zero.
fn sajjil_mono(
    siyaq: &SiyaqFahs<'_>,
    bayanat: &MujalladBayanat,
    qiraa: &mut QiraatKhalfiya,
    hasila: &mut HasilatFahs,
) -> u8 {
    let mut aqwa = 0_u8;

    if let Some(mawqi) = mawqi_mono(siyaq, bayanat) {
        hasila.sajjil_aila(
            AilatMuharrik::Unity,
            NawDaleel::BinyatMujallad,
            format!(
                "Mono runtime directory `{}` {}",
                mawqi.ism, mawqi.wasf_mawdi
            ),
            Some(mawqi.nisbi),
            WAZN_MONO_BLEEDING,
        );
        aqwa = aqwa.max(WAZN_MONO_BLEEDING);
    }

    let Some((nisbi_mudara, mudara)) = mujallad_tahta(siyaq, bayanat, "managed") else {
        return aqwa;
    };

    if let Some(haqiqi) = mudara.malaf("assembly-csharp.dll") {
        hasila.sajjil_aila(
            AilatMuharrik::Unity,
            NawDaleel::BinyatMujallad,
            format!(
                "managed game assembly `{haqiqi}` on disk, which settles the backend and \
                 says nothing about the engine version"
            ),
            Some(format!("{nisbi_mudara}/{haqiqi}")),
            WAZN_ASSEMBLY_CSHARP,
        );
        aqwa = aqwa.max(WAZN_ASSEMBLY_CSHARP);
    } else if let Some(haqiqi) = mudara.malaf("mscorlib.dll") {
        hasila.sajjil_aila(
            AilatMuharrik::Unity,
            NawDaleel::BinyatMujallad,
            format!(
                "managed runtime assembly `{haqiqi}` with no `Assembly-CSharp.dll` beside \
                 it — Mono, with the game's own code under some other assembly name"
            ),
            Some(format!("{nisbi_mudara}/{haqiqi}")),
            WAZN_MSCORLIB,
        );
        aqwa = aqwa.max(WAZN_MSCORLIB);
    }

    qiraa.mudara = Some((nisbi_mudara, mudara));
    aqwa
}

/// The names a player build gives the embedded Mono runtime's directory.
///
/// `MonoBleedingEdge` is Unity's fork, shipped since Unity 5; `Mono` is the
/// stock runtime the Unity 4 players carried.
const MUJALLADAT_MONO: &[&str] = &["monobleedingedge", "mono"];

/// A Mono runtime directory, and which of the two layouts it turned up in.
#[derive(Debug)]
struct MawqiMono {
    /// Its path relative to the game root, as evidence records it.
    nisbi: String,
    /// Its name as the filesystem spells it.
    ism: String,
    /// Where it was, phrased for the evidence line.
    wasf_mawdi: &'static str,
}

/// Finds the Mono runtime directory in either place a player build puts it.
///
/// Inside `*_Data` is the Unity 4 shape. Beside it — `MonoBleedingEdge/` next
/// to the executable, at the game's root — is what every desktop player since
/// Unity 5 ships, and it is the shape actually installed on machines: the Unity
/// 2022 and Unity 6 Mono builds this was checked against both keep it there,
/// with nothing Mono-shaped under the data directory at all.
///
/// Both are searched because both exist, and the miss was not a wrong answer —
/// `Managed/Assembly-CSharp.dll` settles the backend either way — but a silence
/// in the evidence list. That list is what a user reads to understand a verdict
/// and what a maintainer reads to correct one, and a report that names Mono
/// without naming the runtime it found is a report that cannot be checked.
fn mawqi_mono(siyaq: &SiyaqFahs<'_>, bayanat: &MujalladBayanat) -> Option<MawqiMono> {
    for ism in MUJALLADAT_MONO {
        if let Some(haqiqi) = bayanat.fahras.mujallad(ism) {
            return Some(MawqiMono {
                nisbi: bayanat.tahta(haqiqi),
                ism: haqiqi.to_owned(),
                wasf_mawdi: "under the data directory",
            });
        }
    }

    let (nisbi_mujawir, mujawir) = mujawir_bayanat(siyaq, bayanat)?;
    for ism in MUJALLADAT_MONO {
        let Some(haqiqi) = mujawir.mujallad(ism) else {
            continue;
        };
        let nisbi = if nisbi_mujawir.is_empty() {
            haqiqi.to_owned()
        } else {
            format!("{nisbi_mujawir}/{haqiqi}")
        };
        return Some(MawqiMono {
            nisbi,
            ism: haqiqi.to_owned(),
            wasf_mawdi: "beside the data directory, where the player build keeps it",
        });
    }
    None
}

/// Lists the directory the data directory itself sits in.
///
/// For the ordinary desktop layout that is the game's root, whose relative path
/// is the empty string — which [`SiyaqFahs::dakhil`] refuses by design, an empty
/// path being no path — so the root is opened directly. Nothing is joined onto
/// a name found here: the caller builds a string for the evidence trail and
/// never a path to read, so containment is not weakened by the shortcut.
fn mujawir_bayanat(siyaq: &SiyaqFahs<'_>, bayanat: &MujalladBayanat) -> Option<(String, Fahras)> {
    let nisbi = bayanat
        .nisbi
        .rsplit_once('/')
        .map_or("", |(bidaya, _)| bidaya);
    let masar = if nisbi.is_empty() {
        siyaq.jidhr.to_path_buf()
    } else {
        siyaq.dakhil(nisbi).ok()?
    };
    let mahtawa = fahras_mujallad(&masar).ok()?;
    Some((nisbi.to_owned(), mahtawa))
}

/// Reads the front of `global-metadata.dat` and classifies what is there.
///
/// Only the eight words documented at the top of this module are read, and
/// every offset that comes out of them is range-checked against the real file
/// length before anything is done with it. A region whose bounds do not fit
/// inside the file is dropped rather than trusted; the version integer beside
/// it stays, because it is useful on its own and a bad offset does not make it
/// wrong.
fn halat_bayanat(nisbi: String, masar: PathBuf) -> HalatBayanat {
    let Some(nafidha) = iqra_nafidha(&masar, 0, HAJM_TARWISA_BAYANAT) else {
        return HalatBayanat::Mughlaqa;
    };
    let Some(sihr) = raqm32_sagheer(&nafidha, 0x00) else {
        return HalatBayanat::Mughlaqa;
    };
    if sihr != SIHR_BAYANAT {
        return HalatBayanat::Mubhama { sihr };
    }
    let Some(isdar) = sahih32_sagheer(&nafidha, 0x04) else {
        return HalatBayanat::Mubhama { sihr };
    };

    let tul = tul_malaf(&masar).unwrap_or(0);
    let iqtibasat = mintaqa(&nafidha, 0x10, tul);
    let asmaa = mintaqa(&nafidha, 0x18, tul);
    HalatBayanat::Maqrua(TarwisatBayanat {
        nisbi,
        masar,
        isdar,
        asmaa,
        iqtibasat,
    })
}

/// One `(offset, size)` pair out of the metadata header, accepted only when it
/// describes a region that is actually inside the file.
///
/// The lower bound is 0x20 rather than the header's real length because the
/// header's real length changes with the metadata version and this reader
/// deliberately does not know it. Everything before 0x20 is the eight words
/// themselves, so a region claiming to start inside them is wrong under any
/// version.
fn mintaqa(nafidha: &[u8], izaha: usize, tul: u64) -> Option<(u64, u64)> {
    let bidaya = u64::try_from(sahih32_sagheer(nafidha, izaha)?).ok()?;
    let hajm = u64::try_from(sahih32_sagheer(nafidha, izaha.checked_add(4)?)?).ok()?;
    if hajm == 0 || bidaya < 0x20 || tul == 0 {
        return None;
    }
    if bidaya.checked_add(hajm)? > tul {
        return None;
    }
    Some((bidaya, hajm))
}

/// The Unity range a metadata version roughly corresponds to.
///
/// **Approximate, and community-derived rather than published.** The mapping
/// is not one-to-one in either direction: one metadata version spans several
/// Unity releases, Unity's own point releases have changed the metadata
/// without changing this integer, and the sub-versions the tooling calls 24.1
/// through 24.5 are all the number 24 in the file. It is here to make the
/// evidence line readable by a human and it never sets [`HasilatFahs::isdar`],
/// which comes from the engine's own container header and nothing else.
const fn madda_isdar_bayanat(isdar: i32) -> Option<&'static str> {
    match isdar {
        16 => Some("Unity 5.2"),
        19 => Some("Unity 5.3.0 to 5.3.2"),
        20 => Some("Unity 5.3.3 to 5.4"),
        21 => Some("Unity 5.5"),
        22 => Some("Unity 5.6"),
        23 => Some("Unity 2017.1"),
        24 => Some("Unity 2017.2 through 2019.4"),
        27 => Some("Unity 2020.1 through 2021.1"),
        29 => Some("Unity 2021.2 through 2022.2"),
        31 => Some("Unity 2022.3 and later, including Unity 6"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Text systems
// ---------------------------------------------------------------------------

/// One text system and the names that give it away.
#[derive(Debug)]
struct AlamatItar {
    /// The system itself.
    itar: ItarNusus,
    /// How an evidence line names it.
    ism: &'static str,
    /// Fragments of managed assembly file names, lower case, matched as
    /// substrings so that `Unity.TextMeshPro.dll`, `TextMeshPro.dll` and the
    /// old asset-store `TextMeshPro-1.0.55.56.0b12.dll` all match one entry.
    mujammaat: &'static [&'static str],
    /// Exact type names, matched with their NUL terminator inside a string
    /// heap and without it inside a literal blob.
    anwa: &'static [&'static str],
}

/// Every text system this detector can name, and how.
///
/// The terminator is why `TextMesh` is safe to look for at all. Both string
/// regions in `global-metadata.dat` and the `#Strings` heap of a managed
/// assembly hold NUL-terminated names, so `TextMesh\0` matches the world-space
/// component and does not match `TextMeshProUGUI` — which a plain substring
/// search would, and would then report world-space text in every `TextMeshPro`
/// game in the library.
///
/// `FontData` is the marker for legacy UI text rather than `Text`, which is
/// far too common a name to mean anything. `UnityEngine.UI.FontData` is
/// referenced from `UnityEngine.UI.Text` and from nothing else, so its
/// survival through managed stripping is a direct statement that UI Text is in
/// the build rather than merely that uGUI is.
///
/// NGUI and `FairyGUI` have no assembly of their own in most games: both are
/// distributed as source and compile into `Assembly-CSharp.dll`. The type
/// names are the only route to them, which is why the byte scans exist.
const ALAMAT: &[AlamatItar] = &[
    AlamatItar {
        itar: ItarNusus::TextMeshPro,
        ism: "TextMeshPro",
        mujammaat: &["textmeshpro"],
        anwa: &["TMP_Text", "TextMeshProUGUI", "TMP_FontAsset"],
    },
    AlamatItar {
        itar: ItarNusus::UnityUiText,
        ism: "Unity UI Text",
        mujammaat: &["unityengine.ui.dll"],
        anwa: &["FontData"],
    },
    AlamatItar {
        itar: ItarNusus::NGui,
        ism: "NGUI",
        mujammaat: &["ngui"],
        anwa: &["NGUITools", "UILabel"],
    },
    AlamatItar {
        itar: ItarNusus::FairyGui,
        ism: "FairyGUI",
        mujammaat: &["fairygui"],
        anwa: &["GTextField", "GBasicTextField"],
    },
    AlamatItar {
        itar: ItarNusus::TextMesh,
        ism: "world-space TextMesh",
        mujammaat: &[],
        anwa: &["TextMesh"],
    },
];

/// Finds every text system the install will admit to, by whichever route the
/// build leaves open.
///
/// A Mono game names them in file names. An IL2CPP game has no managed
/// assemblies at all, so the same question is put to the identifier heap
/// inside `global-metadata.dat`. Both routes run when both are available,
/// because a game can be Mono and still hide NGUI inside its own assembly.
fn anzimat_nusus(siyaq: &SiyaqFahs<'_>, qiraa: &QiraatKhalfiya, hasila: &mut HasilatFahs) {
    if let Some((nisbi, mudara)) = qiraa.mudara.as_ref() {
        mujammaat_nusus(nisbi, mudara, hasila);
    }
    if let Some(tarwisa) = qiraa.bayanat.as_ref() {
        kawm_asmaa(tarwisa, hasila);
        kawm_iqtibasat(tarwisa, hasila);
    }
    if let Some((nisbi, mudara)) = qiraa.mudara.as_ref() {
        masah_mujamma(siyaq, nisbi, mudara, hasila);
    }
}

/// Names text systems from the managed assemblies on disk.
fn mujammaat_nusus(nisbi_mudara: &str, mudara: &Fahras, hasila: &mut HasilatFahs) {
    for alama in ALAMAT {
        for juz in alama.mujammaat {
            let Some(haqiqi) = mudara.malafat_bi_juz(juz).first().copied() else {
                continue;
            };
            hasila.daa_itar(alama.itar);
            hasila.sajjil(
                NawDaleel::BinyatMujallad,
                format!("{} assembly `{haqiqi}` on disk", alama.ism),
                Some(format!("{nisbi_mudara}/{haqiqi}")),
                WAZN_ITAR_MUJAMMA,
            );
            break;
        }
    }

    if let Some(haqiqi) = mudara.malaf("unityengine.textrenderingmodule.dll") {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "`{haqiqi}` present, which is what world-space TextMesh and the legacy \
                 text generator need. Most builds ship it whether or not the game uses \
                 either, so it is recorded and no text system is claimed from it alone."
            ),
            Some(format!("{nisbi_mudara}/{haqiqi}")),
            WAZN_WAHDA_NASS,
        );
    }

    if let Some(haqiqi) = mudara.malaf("unityengine.imguimodule.dll") {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "`{haqiqi}` present, so IMGUI text can appear on screen. Very nearly every \
                 build ships it, and Taarib has no IMGUI takeover, so this maps to no text \
                 system and is recorded only so the report is not silent about text it \
                 cannot reach."
            ),
            Some(format!("{nisbi_mudara}/{haqiqi}")),
            WAZN_IMGUI,
        );
    }
}

/// One needle in a byte scan, and which text system it belongs to.
#[derive(Debug)]
struct IbaraMatluba {
    /// The bytes searched for.
    bayt: Vec<u8>,
    /// The type name without its terminator, for the evidence line.
    ism: &'static str,
    /// Index into [`ALAMAT`].
    raqm: usize,
}

/// Where a byte scan looked, and what a hit there is worth.
#[derive(Debug)]
struct MasdarMasah<'a> {
    /// The region, named in prose for the evidence line.
    wasf: &'a str,
    /// The file, relative to the game root.
    nisbi: &'a str,
    /// What one hit is worth.
    wazn: u8,
    /// Whether a hit is allowed to name a text system, or only to corroborate
    /// one. False for any region that is not a heap of NUL-terminated names.
    yusammi: bool,
}

/// Builds the needle set for a scan, leaving out systems already found.
///
/// `bi_sifr` appends the NUL terminator, which is what makes a match an exact
/// type name rather than a substring. A region that is not a name heap gets
/// the un-terminated form, and then every name that is a prefix of another
/// name is dropped, because without a terminator it would match the longer one
/// and report a text system the game does not have.
fn ibarat_matluba(hasila: &HasilatFahs, bi_sifr: bool) -> Vec<IbaraMatluba> {
    let mut ibarat = Vec::new();
    for (raqm, alama) in ALAMAT.iter().enumerate() {
        if hasila.itarat.contains(&alama.itar) {
            continue;
        }
        for &naw in alama.anwa {
            if !bi_sifr && sabiqa_li_ghayriha(naw) {
                continue;
            }
            let mut bayt = naw.as_bytes().to_vec();
            if bi_sifr {
                bayt.push(0);
            }
            // The scan carries `TADAKHUL_QITA` bytes of overlap between
            // chunks, so a needle longer than that could straddle a boundary
            // and be missed. Refusing it is honest; silently missing it is not.
            if bayt.len() <= TADAKHUL_QITA {
                ibarat.push(IbaraMatluba {
                    bayt,
                    ism: naw,
                    raqm,
                });
            }
        }
    }
    ibarat
}

/// Whether a type name is a prefix of another name in [`ALAMAT`].
fn sabiqa_li_ghayriha(naw: &str) -> bool {
    ALAMAT
        .iter()
        .flat_map(|alama| alama.anwa.iter())
        .any(|akhar| *akhar != naw && akhar.starts_with(naw))
}

/// Turns a scan's hits into evidence.
fn sajjil_masah(
    natija: &MasahNatija,
    ibarat: &[IbaraMatluba],
    masdar: &MasdarMasah<'_>,
    hasila: &mut HasilatFahs,
) {
    for (mawqi, ibara) in ibarat.iter().enumerate() {
        if !natija.wujida.get(mawqi).copied().unwrap_or(false) {
            continue;
        }
        let Some(alama) = ALAMAT.get(ibara.raqm) else {
            continue;
        };
        let tahaffuz = if masdar.yusammi {
            String::new()
        } else {
            ", which corroborates and does not name it: the region is not a heap of \
             NUL-terminated names, so a longer name containing this one matches too"
                .to_owned()
        };
        hasila.sajjil(
            NawDaleel::TawqiThunai,
            format!(
                "`{}` in {}, pointing at {}{tahaffuz}",
                ibara.ism, masdar.wasf, alama.ism
            ),
            Some(masdar.nisbi.to_owned()),
            masdar.wazn,
        );
        if masdar.yusammi {
            hasila.daa_itar(alama.itar);
        }
    }
}

/// Scans the IL2CPP identifier heap for the type names that identify each text
/// system.
///
/// This is the whole answer for an IL2CPP game. There are no managed assembly
/// files to read, and the heap is where every namespace, type, method and field
/// name that survived managed stripping ends up — which makes a hit here a
/// statement about what is actually in the shipped build rather than about what
/// the project once referenced.
fn kawm_asmaa(tarwisa: &TarwisatBayanat, hasila: &mut HasilatFahs) {
    let Some((izaha, hajm)) = tarwisa.asmaa else {
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            "the metadata header's identifier heap does not describe a region inside the \
             file, so no type name was looked for in it and the absence of any text system \
             below means nothing",
            Some(tarwisa.nisbi.clone()),
            0,
        );
        return;
    };

    let ibarat = ibarat_matluba(hasila, true);
    if ibarat.is_empty() {
        return;
    }
    let hadd = hajm.min(HAJM_MASAH);
    let Some(natija) = masah(&tarwisa.masar, izaha, hadd, &ibarat) else {
        return;
    };

    let masdar = MasdarMasah {
        wasf: "the IL2CPP identifier heap",
        nisbi: &tarwisa.nisbi,
        wazn: WAZN_IBARA_BAYANAT,
        yusammi: true,
    };
    sajjil_masah(&natija, &ibarat, &masdar, hasila);

    if hajm > hadd {
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            format!(
                "the identifier heap is {hajm} bytes and the scan stopped after {}; a text \
                 system named only past that point was not seen",
                natija.maqru
            ),
            Some(tarwisa.nisbi.clone()),
            0,
        );
    }
}

/// Scans the IL2CPP string-literal blob, which is a much weaker witness.
///
/// These are the game's own C# string constants, not its type names. A
/// constant that happens to spell a type name tells you the source mentioned
/// it — in a log line, in a resource path, in a comparison — and nothing about
/// whether the type is in the build. It is read because it costs one more
/// bounded scan and because on a heavily stripped build it is occasionally the
/// only thing left, and it is weighted so that it can never decide anything.
fn kawm_iqtibasat(tarwisa: &TarwisatBayanat, hasila: &mut HasilatFahs) {
    let Some((izaha, hajm)) = tarwisa.iqtibasat else {
        return;
    };
    let ibarat = ibarat_matluba(hasila, false);
    if ibarat.is_empty() {
        return;
    }
    let hadd = hajm.min(HAJM_MASAH);
    let Some(natija) = masah(&tarwisa.masar, izaha, hadd, &ibarat) else {
        return;
    };
    let masdar = MasdarMasah {
        wasf: "the game's own C# string literals",
        nisbi: &tarwisa.nisbi,
        wazn: WAZN_IQTIBAS,
        yusammi: false,
    };
    sajjil_masah(&natija, &ibarat, &masdar, hasila);
}

/// Scans the game's own managed assembly for text systems that ship as source.
///
/// NGUI and `FairyGUI` are distributed as `.cs` files and compile straight into
/// `Assembly-CSharp.dll`, so no file on disk is ever named after either. The
/// only trace they leave in a Mono build is their type names inside that
/// assembly's `#Strings` heap.
///
/// The scan does not parse the CLI metadata to find that heap. Locating it
/// properly means walking the PE data directory to the CLI header, the CLI
/// header to the metadata root, and the root's stream table to `#Strings` —
/// four format dependencies, each versioned, to narrow a search that a bounded
/// byte scan answers directly. The cost of not parsing is precision: a hit
/// could be a string literal or an embedded resource rather than a type name,
/// which is exactly why this route is weighted below the IL2CPP heap.
fn masah_mujamma(
    siyaq: &SiyaqFahs<'_>,
    nisbi_mudara: &str,
    mudara: &Fahras,
    hasila: &mut HasilatFahs,
) {
    let Some(haqiqi) = mudara.malaf("assembly-csharp.dll") else {
        return;
    };
    let nisbi = format!("{nisbi_mudara}/{haqiqi}");
    let Ok(masar) = siyaq.dakhil(&nisbi) else {
        return;
    };

    let ibarat = ibarat_matluba(hasila, true);
    if ibarat.is_empty() {
        return;
    }
    let tul = tul_malaf(&masar).unwrap_or(0);
    let hadd = if tul == 0 {
        HAJM_MASAH
    } else {
        tul.min(HAJM_MASAH)
    };
    let Some(natija) = masah(&masar, 0, hadd, &ibarat) else {
        return;
    };

    let masdar = MasdarMasah {
        wasf: "the game's own managed assembly",
        nisbi: &nisbi,
        wazn: WAZN_IBARA_MUJAMMA,
        yusammi: true,
    };
    sajjil_masah(&natija, &ibarat, &masdar, hasila);

    if tul > hadd {
        hasila.sajjil(
            NawDaleel::TawqiThunai,
            format!(
                "the game's assembly is {tul} bytes and the scan stopped after {}; a text \
                 system named only past that point was not seen",
                natija.maqru
            ),
            Some(nisbi),
            0,
        );
    }
}

// ---------------------------------------------------------------------------
// Corroboration
// ---------------------------------------------------------------------------

/// Reads the small files the Unity player writes about itself.
///
/// Deliberately narrow, and deliberately not a second directory-shape
/// detector: `FahisBinya` owns layout and there is no value in two detectors
/// reporting the same `StreamingAssets` folder. What is read here is the
/// *content* of files Unity itself wrote, which is a different question and one
/// nothing else in the phase asks.
fn qarain(siyaq: &SiyaqFahs<'_>, bayanat: &MujalladBayanat, hasila: &mut HasilatFahs) {
    if let Some(haqiqi) = bayanat.fahras.malaf("app.info") {
        let nisbi = bayanat.tahta(haqiqi);
        if let Ok(masar) = siyaq.dakhil(&nisbi)
            && let Some(nass) = iqra_nass(&masar)
        {
            let mut sutur = nass.lines().map(str::trim).filter(|satr| !satr.is_empty());
            let sharika = sutur.next().unwrap_or_default();
            let muntaj = sutur.next().unwrap_or_default();
            if !sharika.is_empty() || !muntaj.is_empty() {
                hasila.sajjil_aila(
                    AilatMuharrik::Unity,
                    NawDaleel::BayanatMudmaja,
                    format!(
                        "`app.info` names the product `{muntaj}` by `{sharika}` — the \
                         two-line file the Unity player build writes beside its data"
                    ),
                    Some(nisbi),
                    WAZN_APP_INFO,
                );
            }
        }
    }

    if let Some(haqiqi) = bayanat.fahras.malaf("boot.config") {
        let nisbi = bayanat.tahta(haqiqi);
        if let Ok(masar) = siyaq.dakhil(&nisbi)
            && let Some(nass) = iqra_nass(&masar)
        {
            let mutaalliqa: Vec<&str> = nass
                .lines()
                .map(str::trim)
                .filter(|satr| yahummu_khalfiya(satr))
                .take(4)
                .collect();
            let dhayl = if mutaalliqa.is_empty() {
                String::new()
            } else {
                format!(", carrying {}", mutaalliqa.join("; "))
            };
            hasila.sajjil_aila(
                AilatMuharrik::Unity,
                NawDaleel::BayanatMudmaja,
                format!(
                    "`boot.config`, the settings file the Unity player reads before it \
                     starts{dhayl}"
                ),
                Some(nisbi),
                WAZN_BOOT_CONFIG,
            );
        }
    }

    mawarid(siyaq, bayanat, hasila);
}

/// Whether a `boot.config` line says anything about the scripting backend.
///
/// Opportunistic, and reported as such. Unity has never guaranteed that the
/// backend appears in this file and most builds do not name it, so a line that
/// mentions it is worth quoting into the evidence and the absence of one is
/// worth nothing at all.
fn yahummu_khalfiya(satr: &str) -> bool {
    let khafid = satr.to_ascii_lowercase();
    khafid.contains("il2cpp") || khafid.contains("mono") || khafid.starts_with("scripting")
}

/// Reads the player's own resources for corroboration of `TextMeshPro`.
///
/// Two places, because the asset is in one or the other depending on how the
/// build was made. A loose `Resources` directory beside the data is the easy
/// case. The usual case is that the `TextMeshPro` settings asset and its font
/// assets were serialized into `resources.assets` at build time and no file on
/// disk is named after them at all, which is why the container is scanned as
/// well — with the un-terminated needle set and a weight that cannot decide
/// anything, because a serialized container is not a name heap.
fn mawarid(siyaq: &SiyaqFahs<'_>, bayanat: &MujalladBayanat, hasila: &mut HasilatFahs) {
    if let Some((nisbi_mawarid, mahtawa)) = mujallad_tahta(siyaq, bayanat, "resources") {
        for ism in ["unity_builtin_extra", "unity default resources"] {
            if let Some(haqiqi) = mahtawa.malaf(ism) {
                hasila.sajjil_aila(
                    AilatMuharrik::Unity,
                    NawDaleel::BinyatMujallad,
                    format!("`{haqiqi}`, a resource file only the Unity player ships"),
                    Some(format!("{nisbi_mawarid}/{haqiqi}")),
                    WAZN_BUILTIN_EXTRA,
                );
            }
        }
        for juz in ["textmesh", "tmp settings", "tmp_"] {
            if let Some(haqiqi) = mahtawa.malafat_bi_juz(juz).first().copied() {
                hasila.daa_itar(ItarNusus::TextMeshPro);
                hasila.sajjil(
                    NawDaleel::BinyatMujallad,
                    format!(
                        "`{haqiqi}` under the player's own `Resources` directory, which is \
                         where a TextMeshPro settings asset lands when the build leaves it \
                         as a loose file"
                    ),
                    Some(format!("{nisbi_mawarid}/{haqiqi}")),
                    WAZN_TMP_MAWARID,
                );
                break;
            }
        }
    }

    let Some(haqiqi) = bayanat.fahras.malaf("resources.assets") else {
        return;
    };
    let nisbi = bayanat.tahta(haqiqi);
    let Ok(masar) = siyaq.dakhil(&nisbi) else {
        return;
    };
    let ibarat = ibarat_matluba(hasila, false);
    if ibarat.is_empty() {
        return;
    }
    let tul = tul_malaf(&masar).unwrap_or(0);
    let hadd = if tul == 0 {
        HAJM_MASAH
    } else {
        tul.min(HAJM_MASAH)
    };
    let Some(natija) = masah(&masar, 0, hadd, &ibarat) else {
        return;
    };
    let masdar = MasdarMasah {
        wasf: "the player's serialized resource container",
        nisbi: &nisbi,
        wazn: WAZN_TMP_MAWARID,
        yusammi: false,
    };
    sajjil_masah(&natija, &ibarat, &masdar, hasila);
}

// ---------------------------------------------------------------------------
// Directories
// ---------------------------------------------------------------------------

/// One directory's entries, indexed for case-insensitive lookup.
///
/// Listed once and asked many questions, because every stage of this detector
/// asks the same directory something different and `read_dir` on a network
/// share or a spinning disk is not free. Lookups are case-insensitive for the
/// reason [`SiyaqFahs::yujad`] is: `Managed` and `managed` are the same
/// directory to the Windows game that shipped it and two different directories
/// to the Linux filesystem it was copied onto.
#[derive(Debug, Default)]
struct Fahras {
    /// `(lowered name, name as the filesystem spells it, is a directory)`.
    madakhil: Vec<(String, String, bool)>,
    /// Whether the listing succeeded at all.
    maqru: bool,
    /// Whether it stopped at [`AQSA_MADAKHIL`] with entries left over.
    qutia: bool,
}

impl Fahras {
    /// The real spelling of a file whose lowered name is `ism`.
    fn malaf(&self, ism: &str) -> Option<&str> {
        self.madakhil
            .iter()
            .find(|(khafid, _, mujallad)| !*mujallad && khafid == ism)
            .map(|(_, haqiqi, _)| haqiqi.as_str())
    }

    /// The real spelling of a directory whose lowered name is `ism`.
    fn mujallad(&self, ism: &str) -> Option<&str> {
        self.madakhil
            .iter()
            .find(|(khafid, _, mujallad)| *mujallad && khafid == ism)
            .map(|(_, haqiqi, _)| haqiqi.as_str())
    }

    /// Every file whose lowered name contains `juz`.
    fn malafat_bi_juz(&self, juz: &str) -> Vec<&str> {
        self.madakhil
            .iter()
            .filter(|(khafid, _, mujallad)| !*mujallad && khafid.contains(juz))
            .map(|(_, haqiqi, _)| haqiqi.as_str())
            .collect()
    }

    /// Every directory whose lowered name ends with `lahiqa`.
    fn mujalladat_bi_lahiqa(&self, lahiqa: &str) -> Vec<&str> {
        self.madakhil
            .iter()
            .filter(|(khafid, _, mujallad)| *mujallad && khafid.ends_with(lahiqa))
            .map(|(_, haqiqi, _)| haqiqi.as_str())
            .collect()
    }
}

/// Lists one directory, bounded.
///
/// The ceiling is [`AQSA_MADAKHIL`] and hitting it is recorded rather than
/// hidden, because a capability report built from a truncated listing must
/// never be presented as though it were complete.
///
/// A symbolic link is classified by what it points at rather than by being a
/// link, which matches how [`SiyaqFahs::yujad`] already behaves. Containment
/// is not this function's job and is not weakened by that choice: every path
/// built from a name found here goes back through [`SiyaqFahs::dakhil`].
fn fahras_mujallad(masar: &Path) -> Result<Fahras, std::io::Error> {
    let mut madakhil = Vec::new();
    let mut qutia = false;
    for (adad, madkhal) in fs::read_dir(masar)?.enumerate() {
        if adad >= AQSA_MADAKHIL {
            qutia = true;
            break;
        }
        let Ok(madkhal) = madkhal else { continue };
        let khaam = madkhal.file_name();
        let Some(ism) = khaam.to_str() else { continue };
        let mujallad = match madkhal.file_type() {
            Ok(naw) if naw.is_symlink() => {
                fs::metadata(madkhal.path()).is_ok_and(|wasf| wasf.is_dir())
            },
            Ok(naw) => naw.is_dir(),
            Err(_) => false,
        };
        madakhil.push((ism.to_ascii_lowercase(), ism.to_owned(), mujallad));
    }
    Ok(Fahras {
        madakhil,
        maqru: true,
        qutia,
    })
}

/// Resolves a chain of names under the data directory, case-insensitively.
///
/// Every component but the last must be a directory and the last must be a
/// file. Returns the path relative to the game root — spelled as the
/// filesystem spells it, not as this module asked for it — and the absolute
/// path, rebuilt through [`SiyaqFahs::dakhil`].
fn masar_tahta(
    siyaq: &SiyaqFahs<'_>,
    bayanat: &MujalladBayanat,
    ajzaa: &[&str],
) -> Option<(String, PathBuf)> {
    let (akhir, awail) = ajzaa.split_last()?;
    let mut nisbi = bayanat.nisbi.clone();
    let mut mahtawa: Option<Fahras> = None;
    for juz in awail {
        let haqiqi = mahtawa
            .as_ref()
            .unwrap_or(&bayanat.fahras)
            .mujallad(juz)?
            .to_owned();
        nisbi = format!("{nisbi}/{haqiqi}");
        let masar = siyaq.dakhil(&nisbi).ok()?;
        mahtawa = Some(fahras_mujallad(&masar).ok()?);
    }
    let haqiqi = mahtawa
        .as_ref()
        .unwrap_or(&bayanat.fahras)
        .malaf(akhir)?
        .to_owned();
    let nisbi = format!("{nisbi}/{haqiqi}");
    let masar = siyaq.dakhil(&nisbi).ok()?;
    Some((nisbi, masar))
}

/// Resolves one directory under the data directory and lists it.
fn mujallad_tahta(
    siyaq: &SiyaqFahs<'_>,
    bayanat: &MujalladBayanat,
    ism: &str,
) -> Option<(String, Fahras)> {
    let haqiqi = bayanat.fahras.mujallad(ism)?;
    let nisbi = bayanat.tahta(haqiqi);
    let masar = siyaq.dakhil(&nisbi).ok()?;
    let mahtawa = fahras_mujallad(&masar).ok()?;
    Some((nisbi, mahtawa))
}

// ---------------------------------------------------------------------------
// Bounded reads
// ---------------------------------------------------------------------------

/// Reads at most `hadd` bytes of a file starting at `izaha`.
///
/// The one place in this module that opens a file for a header. A short read
/// is not a failure: the caller is handed whatever was there and every reader
/// above bounds-checks before it slices, so a file truncated to nine bytes
/// yields nine bytes and then a `None` from the first field that does not fit.
fn iqra_nafidha(masar: &Path, izaha: u64, hadd: usize) -> Option<Vec<u8>> {
    let mut malaf = File::open(masar).ok()?;
    if izaha > 0 && malaf.seek(SeekFrom::Start(izaha)).is_err() {
        return None;
    }
    let mut bayt = Vec::new();
    let _ = malaf
        .take(u64::try_from(hadd).ok()?)
        .read_to_end(&mut bayt)
        .ok()?;
    Some(bayt)
}

/// A file's length, when it can be had.
fn tul_malaf(masar: &Path) -> Option<u64> {
    Some(fs::metadata(masar).ok()?.len())
}

/// Reads a small text file whole, bounded by [`HAJM_NASS`].
///
/// Decoded lossily. `app.info` carries a product name that can be in any
/// language and was written by a build machine with its own idea of encoding;
/// one byte that is not UTF-8 must cost a character, not the observation.
fn iqra_nass(masar: &Path) -> Option<String> {
    let malaf = File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(HAJM_NASS).read_to_end(&mut bayt).ok()?;
    Some(String::from_utf8_lossy(&bayt).into_owned())
}

/// What a byte scan found.
#[derive(Debug)]
struct MasahNatija {
    /// One flag per needle, in the order they were given.
    wujida: Vec<bool>,
    /// How many bytes were actually examined.
    maqru: u64,
}

/// Scans a bounded region of a file for a set of byte needles.
///
/// Chunked, so scanning eight megabytes never holds eight megabytes, and
/// overlapped by [`TADAKHUL_QITA`] bytes so a needle lying across a chunk
/// boundary is still found. Every needle is checked to fit inside that overlap
/// before the scan starts, which is what makes the guarantee real rather than
/// probable.
///
/// The search itself is a sliding comparison per needle rather than anything
/// cleverer. With the needle sets in this module — sixteen names at most, each
/// rejected on its first byte almost everywhere — that is a handful of passes
/// over at most eight megabytes, it early-exits the moment every needle has
/// been found, and it brings in no dependency to do it.
fn masah(masar: &Path, izaha: u64, hadd: u64, ibarat: &[IbaraMatluba]) -> Option<MasahNatija> {
    if ibarat.is_empty() || hadd == 0 {
        return None;
    }
    let mut malaf = File::open(masar).ok()?;
    if izaha > 0 && malaf.seek(SeekFrom::Start(izaha)).is_err() {
        return None;
    }

    let mut wujida = vec![false; ibarat.len()];
    let mut sabiq: Vec<u8> = Vec::new();
    let mut maqru: u64 = 0;
    let mut qita = vec![0_u8; HAJM_QITA];
    let hajm_qita = u64::try_from(HAJM_QITA).ok()?;

    while maqru < hadd {
        let matlub = usize::try_from(hadd.saturating_sub(maqru).min(hajm_qita)).ok()?;
        let hissa = qita.get_mut(..matlub)?;
        let adad = iqra_kamil(&mut malaf, hissa)?;
        if adad == 0 {
            break;
        }
        maqru = maqru.saturating_add(u64::try_from(adad).ok()?);

        let mut nafidha = std::mem::take(&mut sabiq);
        nafidha.extend_from_slice(qita.get(..adad)?);
        for (mawqi, ibara) in ibarat.iter().enumerate() {
            if wujida.get(mawqi).copied().unwrap_or(false) {
                continue;
            }
            if yahtawi(&nafidha, &ibara.bayt)
                && let Some(alam) = wujida.get_mut(mawqi)
            {
                *alam = true;
            }
        }
        if wujida.iter().all(|alam| *alam) || adad < matlub {
            break;
        }
        let khalf = nafidha.len().saturating_sub(TADAKHUL_QITA);
        sabiq = nafidha.get(khalf..).unwrap_or_default().to_vec();
    }

    Some(MasahNatija { wujida, maqru })
}

/// Fills a buffer, or reads until the file ends.
fn iqra_kamil(malaf: &mut File, hissa: &mut [u8]) -> Option<usize> {
    let mut kulli = 0_usize;
    while kulli < hissa.len() {
        let baqi = hissa.get_mut(kulli..)?;
        match malaf.read(baqi) {
            Ok(0) => break,
            Ok(adad) => kulli = kulli.checked_add(adad)?,
            Err(khata) if khata.kind() == std::io::ErrorKind::Interrupted => {},
            Err(_) => return None,
        }
    }
    Some(kulli)
}

/// Whether a byte run contains another.
fn yahtawi(kawm: &[u8], ibara: &[u8]) -> bool {
    if ibara.is_empty() || ibara.len() > kawm.len() {
        return false;
    }
    kawm.windows(ibara.len()).any(|nafidha| nafidha == ibara)
}

// ---------------------------------------------------------------------------
// Fields
//
// Four readers, each taking an offset and checking it. Endianness is per
// format and not per file: `SerializedFile` and `UnityFS` write their header
// fields big-endian, `global-metadata.dat` writes its little-endian, and the
// two are never read with the wrong one because they never share a function.
// ---------------------------------------------------------------------------

/// A big-endian `u32` at a checked offset.
fn raqm32_kabir(qita: &[u8], izaha: usize) -> Option<u32> {
    let bayt = qita.get(izaha..izaha.checked_add(4)?)?;
    <[u8; 4]>::try_from(bayt).ok().map(u32::from_be_bytes)
}

/// A big-endian `i64` at a checked offset.
fn raqm64_kabir(qita: &[u8], izaha: usize) -> Option<i64> {
    let bayt = qita.get(izaha..izaha.checked_add(8)?)?;
    <[u8; 8]>::try_from(bayt).ok().map(i64::from_be_bytes)
}

/// A little-endian `u32` at a checked offset.
fn raqm32_sagheer(qita: &[u8], izaha: usize) -> Option<u32> {
    let bayt = qita.get(izaha..izaha.checked_add(4)?)?;
    <[u8; 4]>::try_from(bayt).ok().map(u32::from_le_bytes)
}

/// A little-endian `i32` at a checked offset.
///
/// Signed because the IL2CPP metadata header declares its offsets and sizes as
/// `int32_t`, and a negative one is how a corrupt or deliberately mangled
/// header presents itself. Reading it as unsigned would turn `-1` into four
/// billion and hand a bounds check a number that passes.
fn sahih32_sagheer(qita: &[u8], izaha: usize) -> Option<i32> {
    let bayt = qita.get(izaha..izaha.checked_add(4)?)?;
    <[u8; 4]>::try_from(bayt).ok().map(i32::from_le_bytes)
}

/// A NUL-terminated run of printable ASCII, and the offset just past its
/// terminator.
///
/// Printable ASCII rather than UTF-8 on purpose. Every string this module
/// reads is a container signature or a version, both of which Unity writes as
/// ASCII with no spaces in them, so refusing anything else is what stops a run
/// of arbitrary bytes from being reported as a version string. A run that
/// reaches `aqsa` without a terminator is refused for the same reason.
fn nass_muntahi_maa_mawqi(qita: &[u8], bidaya: usize, aqsa: usize) -> Option<(String, usize)> {
    let baqi = qita.get(bidaya..)?;
    let mut tul = 0_usize;
    loop {
        let harf = baqi.get(tul).copied()?;
        if harf == 0 {
            break;
        }
        if !harf.is_ascii_graphic() {
            return None;
        }
        tul = tul.checked_add(1)?;
        if tul > aqsa {
            return None;
        }
    }
    let nass = std::str::from_utf8(baqi.get(..tul)?).ok()?.to_owned();
    Some((nass, bidaya.checked_add(tul)?.checked_add(1)?))
}

/// The same, where an empty string is the absence of one.
fn nass_muntahi(qita: &[u8], bidaya: usize, aqsa: usize) -> Option<String> {
    let (nass, _) = nass_muntahi_maa_mawqi(qita, bidaya, aqsa)?;
    if nass.is_empty() { None } else { Some(nass) }
}
