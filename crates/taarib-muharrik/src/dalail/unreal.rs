//! أنريل — Unreal Engine, read from its containers rather than from its code.
//!
//! Unreal is the easiest engine in the coverage matrix to *recognise* and one of
//! the hardest to *date*. Recognising it takes one directory: nothing but Unreal
//! ships `Engine/Binaries` next to a `<Project>/Content/Paks`. Dating it is
//! harder, because the engine's own containers version themselves on their own
//! schedule — a pak version covers a range of engine releases, sometimes a very
//! wide one, and the widest of them straddles the UE4/UE5 line entirely.
//!
//! ## What this detector is competent to see
//!
//! Five things, four of them read from headers and footers:
//!
//! 1. **The containers.** A `.pak` footer, which carries a magic, a version, the
//!    index offset and size, and — from version 4 onward — whether the index is
//!    AES-encrypted. A UE5 IoStore `.utoc` header, which carries its own magic,
//!    its own version, and container flags including encryption.
//! 2. **The project.** Unreal's packaged layout names the project twice: once as
//!    the directory holding `Content/Paks`, and once in the shipping
//!    executable's name, `<Project>-Win64-Shipping.exe`. Phase 8 needs that name
//!    to write its patch pak into the right `Content/Paks` directory.
//! 3. **Localization.** `Content/Localization/` and loose `.locres` files, whose
//!    presence tells Phase 8's reinjection path it has somewhere to land, and
//!    whose absence is itself worth reporting.
//! 4. **Encryption.** A pak whose index is encrypted is unreadable without a key
//!    the user supplies. That is the difference between "Taarib can extract this
//!    game's text" and "Taarib needs a key first", and the capability report is
//!    not allowed to promise the first when the truth is the second.
//! 5. **The build's own version.** `Engine/Build/Build.version`, which Unreal's
//!    build system writes and the staging step ships, names the engine version
//!    as three integers. See [`IsdarBina`].
//!
//! ## Exact versions and derived ranges are not the same kind of fact
//!
//! Everything this detector reads out of a container is a **range**: a pak
//! version covers several engine releases, and the widest of them covers two
//! major versions. Two sources are **point values** instead — the executable's
//! `++UE4+Release-4.27` tag, which `dalail::thunai` reads, and this module's
//! `Engine/Build/Build.version`. A point value is what the build wrote about
//! itself; a range is an inference this project made from a container format's
//! version number.
//!
//! **A derived range must never outrank an exact version, whatever the weight of
//! the observation that carried it.** Inside this detector that rule is enforced
//! in [`istinbat_isdar`], which lets `Build.version` overwrite both container
//! ranges. Across detectors it is enforced in [`crate::tahdid`], which is the
//! only place that sees more than one detector's answer — a rule stated here and
//! nowhere else would be a rule that holds until the moment two detectors
//! disagree, which is the moment it matters.
//!
//! ## Division of labour with the binary detector
//!
//! `dalail::thunai` searches a bounded window of the game's executable for
//! Unreal's own version tags — `++UE4+Release-4.27`, `++UE5+Release-5.3` — which
//! the engine embeds through `FEngineVersion`. That source is better than every
//! *range* in this file, for the reason above.
//!
//! So this detector does not go looking for those tags, and does not read the
//! executable at all beyond its name. It reads containers, which the binary
//! detector does not; it names the project directory, which the binary detector
//! cannot; and it reads `Build.version`, which is the only exact source a title
//! whose executable carries no branch tag has — not a hypothetical, it is what
//! Little Nightmares ships. The three are additive by construction rather than
//! by agreement, and when they disagree — a `.pak` that says 4.25 inside a game
//! whose binary says 4.27 — `tahdid` reports the conflict instead of averaging
//! it, because a game shipping a pak built by an older engine is a real thing
//! that happens and the discrepancy is the interesting part.
//!
//! ## Division of labour with the directory-shape detector
//!
//! `dalail::binya` walks the game once and matches directory shapes for every
//! engine at once, Unreal included: it sees `Engine/Binaries`, the shipping
//! executable, the staging manifests, `Content/Paks`, and the bare existence of
//! `.pak` and `.utoc` files. This detector sees several of the same things and
//! records them again, which is deliberate rather than sloppy.
//!
//! Two reasons. First, [`crate::tahdid`] combines by taking the strongest weight
//! per family rather than by summing, so a fact observed twice cannot inflate a
//! confidence; it only appears twice in the evidence trail, where a maintainer
//! diagnosing a wrong identification benefits from seeing that two independent
//! detectors agreed. Second, this detector's observations are not the same
//! observations: it names the *project directory* Phase 8 has to write into,
//! reads the containers rather than counting them, and reports encryption and
//! localization, none of which a directory walk can see. Removing the
//! overlapping half would couple this file to the exact contents of another one.
//!
//! ## The `.pak` footer, exactly as it is written
//!
//! `FPakInfo` is serialized at the very end of the file, in this order. Nothing
//! precedes it that has to be parsed, which is why a 50 GB pak costs one seek
//! and one 512-byte read here.
//!
//! | field | bytes | present when |
//! | --- | --- | --- |
//! | `EncryptionKeyGuid` | 16 | version >= 7 |
//! | `bEncryptedIndex` | 1 | always; meaningful from version 4 |
//! | `Magic` = `0x5A6F12E1` | 4 | always |
//! | `Version` | 4 | always |
//! | `IndexOffset` | 8 | always |
//! | `IndexSize` | 8 | always |
//! | `IndexHash` (SHA-1) | 20 | always |
//! | `bIndexIsFrozen` | 1 | version == 9 only |
//! | `CompressionMethods` | 32 x 4, or 32 x 5 | version >= 8 |
//!
//! Everything is little-endian. The consequence of that layout is that the magic
//! sits at a *known distance from the end of the file*, and that distance is
//! itself a version discriminator.
//!
//! The distance is measured from the end of the file to the **first byte of the
//! magic**, so it counts the magic's own four bytes: 4 + 4 + 8 + 8 + 20 = 44 for
//! the base layout, plus 32 bytes for each compression-method slot from version
//! 8, plus one more byte for version 9's `bIndexIsFrozen`. Counting only the
//! bytes *after* the magic gives four numbers that are each four short, and a
//! reader built on those four numbers never matches a real pak.
//!
//! | distance from EOF to the magic | footer size | versions |
//! | --- | --- | --- |
//! | 44 | 45 / 61 | 1-6 (45), 7 (61) |
//! | 172 | 189 | 8 with four compression slots |
//! | 204 | 221 | 8 with five slots, 10, 11 |
//! | 205 | 222 | 9 |
//!
//! Two of the four are checked against shipped containers rather than derived
//! only on paper: Little Nightmares' version 3 pak puts the magic at EOF-44, and
//! Little Nightmares Enhanced Edition's version 11 pak puts it at EOF-204.
//!
//! This reader checks those four positions first, and only if none of them holds
//! the magic does it scan the last [`HAJM_DHAYL_PAK`] bytes backwards for it.
//! The scan exists for pak versions written after this build — a version 12 that
//! moved the magic would otherwise read as "not a pak" — and it validates harder
//! than the table path does, because a backwards scan through arbitrary bytes
//! will eventually find any four-byte pattern you ask it for.
//!
//! ## Pak version to engine version, and why it is a range
//!
//! Unreal bumps `FPakInfo::PakFile_Version_Latest` when the container format
//! changes, which is not when the engine is released. Several engine releases
//! therefore share one pak version, and one pak version — 11 — spans the entire
//! UE4-to-UE5 boundary. **A pak version alone can never tell you UE4 from UE5 at
//! the top of the range.**
//!
//! | pak | engine range | how firm |
//! | --- | --- | --- |
//! | 1 | 4.0 and earlier | ordering only |
//! | 2 | 4.0 - 4.2 | ordering only |
//! | 3 | 4.3 - 4.15 | the longest-lived version; a very wide range |
//! | 4 | 4.16 | narrow |
//! | 5 | 4.17 - 4.19 | narrow-ish |
//! | 6 | 4.20 | narrow |
//! | 7 | 4.21 | narrow |
//! | 8, four slots | 4.22 | the four-slot block is the discriminator |
//! | 8, five slots | 4.23 - 4.24 | firm |
//! | 9 | 4.25 | firm; `bIndexIsFrozen` exists only here |
//! | 10 | 4.26 | firm |
//! | 11 | 4.27, or any UE5 | cannot be narrowed from the pak at all |
//!
//! **The whole table is inferred**, from the order in which the version enum
//! gained its members and from what shipped games carry, not from a published
//! mapping — Epic does not publish one. Only the ordering is certain. This
//! detector therefore emits an [`IsdarMuharrik`] only when the range sits inside
//! one major version, marks the raw string `derived` so no downstream reader
//! mistakes it for something found in the game, and stays silent at version 11
//! unless an IoStore container settles the major.
//!
//! ## The `.utoc` header
//!
//! UE5's IoStore splits a container into a table of contents (`.utoc`) and the
//! data it indexes (`.ucas`). The TOC begins with `FIoStoreTocHeader`, 144 bytes:
//!
//! | offset | field | bytes |
//! | --- | --- | --- |
//! | 0 | magic `-==--==--==--==-` | 16 |
//! | 16 | `Version` | 1 |
//! | 17 | reserved | 3 |
//! | 20 | `TocHeaderSize` (144) | 4 |
//! | 24 | `TocEntryCount` | 4 |
//! | 28 | `TocCompressedBlockEntryCount` | 4 |
//! | 32 | `TocCompressedBlockEntrySize` | 4 |
//! | 36 | `CompressionMethodNameCount` | 4 |
//! | 40 | `CompressionMethodNameLength` | 4 |
//! | 44 | `CompressionBlockSize` | 4 |
//! | 48 | `DirectoryIndexSize` | 4 |
//! | 52 | `PartitionCount` | 4 |
//! | 56 | `ContainerId` | 8 |
//! | 64 | `EncryptionKeyGuid` | 16 |
//! | 80 | `ContainerFlags` | 1 |
//! | 81 | reserved | 3 |
//! | 84 | `TocChunkPerfectHashSeedsCount` | 4 |
//! | 88 | `PartitionSize` | 8 |
//! | 96 | `TocChunksWithoutPerfectHashCount` | 4 |
//! | 100 | reserved | 44 |
//!
//! `ContainerFlags` is a bitfield: compressed 1, encrypted 2, signed 4, indexed
//! 8, on-demand 16.
//!
//! **IoStore is not exclusively UE5.** It landed in 4.25 and shipped on console
//! in 4.26 and 4.27, so a `.utoc` is strong evidence of UE5 rather than proof of
//! it, and the TOC version is what separates the two: versions 1 and 2 predate
//! UE5, version 3 (`PartitionSize`) and up are UE5-era. That mapping is inferred
//! from the order of `EIoStoreTocVersion`'s members; the release each landed in
//! is not published, so the ranges here are deliberately wide and the binary's
//! own `++UE5+Release-x.y` tag overrides every one of them.
//!
//! ## What Phase 8 gets out of this
//!
//! Phase 8 writes an additive patch pak into `<Project>/Content/Paks/` and
//! mounts it above the game's own. It needs the project directory to do that,
//! and it gets it two ways, both stable:
//!
//! - [`FahisUnreal::mashru`], which returns the project directory relative to
//!   the game root, and which Phase 8 can call directly.
//! - The `mawqi` of the `Content/Paks` observation in the evidence trail, whose
//!   first component is the project name. That is a contract, not a
//!   coincidence: the evidence survives into the database and the diagnostics
//!   bundle, so a patch built months later can be checked against the layout it
//!   was built for.
//!
//! It also gets the *real* executable. Unreal's root `<Project>.exe` is a shim
//! that relaunches `<Project>/Binaries/Win64/<Project>-Win64-Shipping.exe`, and
//! injecting into the shim injects into a process that has already exited. This
//! detector reports the real binary in [`HasilatFahs::tanfidhi`] whenever it
//! finds one.
//!
//! **The suffix is not required.** Publishers rename the shipping binary, and a
//! resolution that insisted on `-<Platform>-Shipping` would leave a renamed one
//! unread by every part of the product — Little Nightmares ships
//! `Atlas/Binaries/Win64/LittleNightmares.exe` and nothing with the suffix
//! anywhere. [`tanfidhi_shipping`] matches the suffix first and falls back to the
//! largest plausible executable in the project's own platform directory, and the
//! two reach the evidence trail as different claims: the first is proof of
//! Unreal, the second is only an answer to "which process".
//!
//! ## Weights
//!
//! | observation | weight | why |
//! | --- | --- | --- |
//! | validated `.pak` footer | 92 | magic, version and index bounds all agree |
//! | validated `.utoc` header | 95 | nothing else writes that magic |
//! | `Engine/Binaries` directory | 85 | strong, and Unreal's alone in practice |
//! | `<Project>/Content/Paks` directory | 88 | the packaged layout, exactly |
//! | `Engine/Build/Build.version` | 90 | the build's own exact version, in a file only Unreal has |
//! | `<Project>-Win64-Shipping.exe` | 80 | generated by Unreal's build system |
//! | loose `.locres` file | 70 | an Unreal-only format |
//! | a renamed binary under `<Project>/Binaries` | 45 | says which process, not which engine |
//! | `Manifest_*Files_*.txt` at root | 50 | Unreal's staging leaves them behind |
//! | `.utoc` that would not parse | 40 | the extension is Unreal's; the file is not readable |
//! | `.pak` extension alone | 15 | Chromium, Quake and several installers use it too |
//! | derived detail (encryption, culture) | 30 | already implied by the container it came from |
//! | observed absence of `.locres` | 5 | keeps the report honest; not an argument for Unreal |

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use taarib_mustalahat::muharrik::{
    AilatMuharrik, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel,
};
use taarib_usus::khata::Natija;
use taarib_usus::manassa::Mimariya;

use crate::dalail::imtidad;
use crate::fahs::{AQSA_MADAKHIL, Fahis, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

/// `FPakInfo::PakFile_Magic`, little-endian in the file.
const TAWQI_PAK: u32 = 0x5A6F_12E1;

/// `FIoStoreTocHeader::TocMagicImg`, sixteen bytes of alternating hyphens and
/// equals signs. Chosen by Epic to be unmistakable, and it is.
const TAWQI_UTOC: &[u8; 16] = b"-==--==--==--==-";

/// The highest pak version this build has a mapping for.
///
/// A higher one is read, reported, and treated as UE5-era rather than refused,
/// because refusing would mean a game built on an engine released after this
/// binary reports no engine at all.
const AQSA_NUSKHAT_PAK: u32 = 11;

/// The highest pak version the backwards scan will accept as plausible.
///
/// Headroom over [`AQSA_NUSKHAT_PAK`] for formats written after this build,
/// bounded so that four arbitrary bytes followed by a large integer cannot pass
/// for a footer.
const AQSA_NUSKHAT_MAQBULA: u32 = 24;

/// How many bytes at the end of a `.pak` are read.
///
/// The largest footer this build knows is 222 bytes. Half a kilobyte covers it,
/// covers a future version that grows the compression-method block, and is one
/// read regardless of whether the pak is four megabytes or fifty gigabytes.
pub const HAJM_DHAYL_PAK: usize = 512;

/// The size of `FIoStoreTocHeader`, which every observed TOC version writes.
pub const HAJM_TARWISAT_UTOC: usize = 144;

/// `Engine/Build/Build.version`, relative to the game root.
///
/// Unreal's build system writes it and the staging step copies it into the
/// packaged game. It is the only file in a shipped Unreal title that names the
/// engine version as three integers rather than as a range or a branch string.
const MASAR_ISDAR_BINA: [&str; 3] = ["Engine", "Build", "Build.version"];

/// The most of `Build.version` that is read.
///
/// Four kilobytes. The real file is around a hundred and thirty bytes; the cap
/// is here because the path is one a game directory controls and a reader that
/// trusts a path's contents to be small is a reader that allocates whatever it
/// is handed.
const HAJM_ISDAR_BINA: usize = 4096;

/// Where the pak magic sits relative to the end of the file, and which versions
/// put it there.
///
/// Derived from the footer layout in the module documentation, and confirmed
/// against shipped containers at both ends of the table. A candidate that holds
/// the magic but whose version is not listed against that distance is rejected:
/// the two facts have to agree, and when they do not, what was found was four
/// coincidental bytes.
///
/// **Each distance counts the magic itself.** The magic starts at
/// `file_length - distance`, so the smallest entry is 44 and not 40 — see the
/// module documentation for the arithmetic and for what going four bytes short
/// costs.
const MAWADI_TAWQI_PAK: [(u64, &[u32]); 4] = [
    (44, &[1, 2, 3, 4, 5, 6, 7]),
    (172, &[8]),
    (204, &[8, 10, 11]),
    (205, &[9]),
];

/// The shortest distance from the end of a pak at which a magic can begin.
///
/// `FPakInfo` writes the magic, the version, the index offset, the index size
/// and a twenty-byte hash, and no version has ever written fewer fields than
/// that. A magic closer to the end than this cannot be followed by a complete
/// footer, so the backwards scan starts here rather than at a lower guess.
const AQALL_IZAHAT_TAWQI: u64 = 44;

/// The compression-method slot counts the two eight-and-later layouts carry.
///
/// Version 8 shipped with four name slots and then grew to five, which is the
/// only thing separating 4.22 from 4.23; versions 10 and 11 kept the five.
const KHANAT_ARBA: usize = 4;
/// The five-slot compression-method block, as [`KHANAT_ARBA`].
const KHANAT_KHAMS: usize = 5;

/// How many `.pak` files are examined before the detector stops looking.
///
/// A chunked game ships hundreds of `pakchunk*.pak`, and they all carry the same
/// footer version. Reading six settles the question; reading three hundred just
/// spends the user's disk.
const AQSA_HAWIYAT: usize = 6;

/// How many `.locres` files are counted before counting stops.
const AQSA_LOCRES: usize = 64;

/// How many directory levels below the game root are searched for the project
/// directory.
///
/// One finds the standard layout. Two finds the games whose store wrapper adds a
/// directory above the project — which is common enough on Epic and GOG to be
/// worth the second `read_dir`.
const UMQ_BAHTH_MASHRU: usize = 2;

/// The shipping-executable suffixes Unreal's build system generates, and the
/// architecture each implies.
///
/// The architecture here is read from a *name*, which is weaker than reading a
/// PE or ELF header, and `dalail::thunai` does the latter. This table exists for
/// the case where discovery named the root shim and the real binary was never
/// parsed by anyone.
const LAWAHIQ_SHIPPING: [(&str, Mimariya); 7] = [
    ("-Win64-Shipping.exe", Mimariya::X8664),
    ("-WinGDK-Shipping.exe", Mimariya::X8664),
    ("-Win32-Shipping.exe", Mimariya::X86),
    ("-WinArm64-Shipping.exe", Mimariya::Aarch64),
    ("-Linux-Shipping", Mimariya::X8664),
    ("-LinuxArm64-Shipping", Mimariya::Aarch64),
    ("-Mac-Shipping", Mimariya::X8664),
];

/// The platform directories under `<Project>/Binaries/`, the architecture each
/// implies, and the extension an executable carries there.
///
/// The extension is what makes the name-blind fallback in [`tanfidhi_shipping`]
/// safe. A real `Atlas/Binaries/Win64` ships `libScePad.dll`, `tbb12.dll` and a
/// fifty-megabyte `OpenImageDenoise.dll` beside the game; without the `.exe`
/// filter the largest-file rule would pick the denoiser. An empty extension
/// means the platform's executables carry none, which is true of Linux and of
/// the binary inside a macOS bundle.
const MUJALLADAT_TANFIDH: [(&str, Mimariya, &str); 6] = [
    ("Win64", Mimariya::X8664, "exe"),
    ("WinGDK", Mimariya::X8664, "exe"),
    ("Win32", Mimariya::X86, "exe"),
    ("WinArm64", Mimariya::Aarch64, "exe"),
    ("Linux", Mimariya::X8664, ""),
    ("Mac", Mimariya::X8664, ""),
];

/// The smallest file the name-blind fallback will call a shipping binary.
///
/// Four mebibytes. A monolithic Unreal build links the whole engine and has
/// never been smaller than tens of megabytes; the root shim that relaunches it
/// is a couple of hundred kilobytes, and a crash reporter or a prerequisite
/// installer sitting in the same directory is smaller still. The floor is set
/// well under any real shipping binary and well over every launcher stub seen,
/// so it separates them without needing either one's name.
const AQALL_HAJM_TANFIDH: u64 = 4 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Bounded reading
//
// Nothing in this module reads a whole file. A pak is a shipping container that
// routinely passes fifty gigabytes; a `.ucas` is larger still. Every read below
// is a seek to a known place and a fixed number of bytes, and every offset that
// came out of a file is checked against that file's real length before it is
// used for anything — a corrupt footer claiming an index at 2^40 has to produce
// the absence of evidence, not an allocation.
//
// These use `File::seek` and `read_exact` rather than `memmap2`. Mapping is the
// better tool when a file is read repeatedly at scattered offsets; here each
// file is touched once at one place, mapping would cost an `unsafe` block whose
// soundness depends on nobody truncating the file underneath us, and a game
// directory is exactly the place where a launcher's updater might.
// ---------------------------------------------------------------------------

/// Reads up to `hajm` bytes from the end of a file.
///
/// Returns the bytes and the file's real length. The bytes are the *tail*, so a
/// position `p` inside them corresponds to file offset `tul - bayt.len() + p`.
fn iqra_dhayl(masar: &Path, hajm: usize) -> Option<(Vec<u8>, u64)> {
    let mut malaf = File::open(masar).ok()?;
    let tul = malaf.metadata().ok()?.len();
    let miqdar = tul.min(u64::try_from(hajm).ok()?);
    let bidaya = tul.checked_sub(miqdar)?;
    let _ = malaf.seek(SeekFrom::Start(bidaya)).ok()?;
    let mut bayt = vec![0_u8; usize::try_from(miqdar).ok()?];
    malaf.read_exact(&mut bayt).ok()?;
    Some((bayt, tul))
}

/// Reads up to `hajm` bytes from the start of a file.
fn iqra_rass(masar: &Path, hajm: usize) -> Option<(Vec<u8>, u64)> {
    let mut malaf = File::open(masar).ok()?;
    let tul = malaf.metadata().ok()?.len();
    let miqdar = usize::try_from(tul.min(u64::try_from(hajm).ok()?)).ok()?;
    let mut bayt = vec![0_u8; miqdar];
    malaf.read_exact(&mut bayt).ok()?;
    Some((bayt, tul))
}

/// Little-endian `u32` at a position, or nothing if the slice is too short.
fn u32_min(bayt: &[u8], mawdi: usize) -> Option<u32> {
    let qita: [u8; 4] = bayt.get(mawdi..mawdi.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(qita))
}

/// Little-endian `u64` at a position, or nothing if the slice is too short.
fn u64_min(bayt: &[u8], mawdi: usize) -> Option<u64> {
    let qita: [u8; 8] = bayt.get(mawdi..mawdi.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(qita))
}

/// One directory entry, reduced to the three facts a detector needs.
#[derive(Debug, Clone)]
struct Madkhal {
    /// The entry's own name.
    ism: String,
    /// Its full path.
    masar: PathBuf,
    /// Whether it is a directory. Symbolic links never reach here.
    mujallad: bool,
}

/// The entry budget for one probe.
///
/// Shared across every directory this detector opens, so that a game with a
/// hundred thousand files costs a bounded amount of work in total rather than a
/// bounded amount per directory.
#[derive(Debug)]
struct Mizaniya {
    /// How many entries may still be looked at.
    mutabaqqi: usize,
}

impl Mizaniya {
    /// A fresh budget of [`AQSA_MADAKHIL`] entries.
    const fn jadeeda() -> Self {
        Self {
            mutabaqqi: AQSA_MADAKHIL,
        }
    }

    /// Lists one directory, spending from the budget.
    ///
    /// A directory that cannot be read yields nothing, which is the correct
    /// reading of a permission failure: the absence of evidence. Symbolic links
    /// are skipped rather than followed, so a link inside the game pointing at
    /// `/` cannot turn a probe into a filesystem walk.
    fn madakhil(&mut self, masar: &Path) -> Vec<Madkhal> {
        let mut natija = Vec::new();
        if self.mutabaqqi == 0 {
            return natija;
        }
        let Ok(qira) = std::fs::read_dir(masar) else {
            return natija;
        };
        for madkhal in qira {
            if self.mutabaqqi == 0 {
                break;
            }
            self.mutabaqqi = self.mutabaqqi.saturating_sub(1);
            let Ok(madkhal) = madkhal else { continue };
            let Ok(naw) = madkhal.file_type() else {
                continue;
            };
            if naw.is_symlink() {
                continue;
            }
            let Some(ism) = madkhal.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            natija.push(Madkhal {
                ism,
                masar: madkhal.path(),
                mujallad: naw.is_dir(),
            });
        }
        natija
    }
}

/// Whether a path is a real directory rather than a symbolic link to one.
///
/// `Path::is_dir` follows links, which is exactly what a probe must not do: a
/// link named `Content` pointing at another game would make this detector
/// describe an engine the game does not use. `symlink_metadata` does not
/// follow, so a link fails this test whatever it points at.
fn mujallad_haqiqi(masar: &Path) -> bool {
    std::fs::symlink_metadata(masar).is_ok_and(|bayanat| bayanat.is_dir())
}

/// Whether a path's extension matches, ignoring case.
fn imtidad_huwa(masar: &Path, imtidad: &str) -> bool {
    masar
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|mawjud| mawjud.eq_ignore_ascii_case(imtidad))
}

// ---------------------------------------------------------------------------
// The .pak footer
// ---------------------------------------------------------------------------

/// A `.pak` file's footer, read from the last bytes of the file.
///
/// Everything here comes from `FPakInfo` as the engine wrote it. Nothing is
/// interpreted beyond bounds-checking: the index is not read, the entries are
/// not walked, and the file is not decrypted. Phase 8 does all of that, with a
/// key when the user has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhaylPak {
    /// The container format version, 1 through 11 for everything this build has
    /// seen, higher for a format written after it.
    pub nuskha: u32,
    /// Where the index starts, already checked to lie inside the file.
    pub mawdi_fahras: u64,
    /// How long the index is, already checked to fit inside the file.
    pub hajm_fahras: u64,
    /// Whether the index is AES-encrypted. False for versions below 4, which
    /// had no such flag and no such feature.
    pub mushaffar: bool,
    /// The encryption key's GUID as Unreal writes it, thirty-two uppercase hex
    /// digits, when the pak names one. Version 7 and up only, and only when it
    /// is not all zeroes — a zero GUID means the project's default key.
    pub muarrif_miftah: Option<String>,
    /// How many compression-method name slots the footer carries: 0 below
    /// version 8, then 4 or 5. The count separates 4.22 from 4.23.
    pub khanat_dagt: usize,
    /// The file's real length on disk.
    pub hajm_malaf: u64,
    /// How far the magic sat from the end of the file, which is what identified
    /// the footer layout.
    pub izahat_tawqi: u64,
    /// Whether the footer was found by the backwards scan rather than at one of
    /// the four known distances — true only for a format newer than this build.
    pub bil_mash: bool,
}

impl DhaylPak {
    /// Reads the footer of a `.pak`, or reports that there is not one.
    ///
    /// One seek and one read of [`HAJM_DHAYL_PAK`] bytes, whatever the file's
    /// size. A file that is too short, will not open, holds no magic, or holds a
    /// magic whose version and index bounds contradict it, returns [`None`] —
    /// which is the absence of evidence and never an error.
    #[must_use]
    pub fn iqra(masar: &Path) -> Option<Self> {
        let (dhayl, tul) = iqra_dhayl(masar, HAJM_DHAYL_PAK)?;
        for (izaha, nusakh) in MAWADI_TAWQI_PAK {
            if let Some(dhayl_pak) = Self::min_mawdi(&dhayl, tul, izaha, Some(nusakh), false) {
                return Some(dhayl_pak);
            }
        }
        Self::bil_mash(&dhayl, tul)
    }

    /// Scans the tail backwards for a magic at a distance this build does not
    /// know, for a pak format written after it.
    ///
    /// Held to a stricter standard than the table path, because a backwards
    /// scan over half a kilobyte of arbitrary data will find any four bytes
    /// eventually: the version has to be plausible, the index has to fit inside
    /// the file, and the index has to be non-empty.
    fn bil_mash(dhayl: &[u8], tul: u64) -> Option<Self> {
        let mut izaha: u64 = AQALL_IZAHAT_TAWQI;
        let hadd = u64::try_from(dhayl.len()).ok()?;
        while izaha <= hadd {
            if let Some(dhayl_pak) = Self::min_mawdi(dhayl, tul, izaha, None, true) {
                return Some(dhayl_pak);
            }
            izaha = izaha.checked_add(1)?;
        }
        None
    }

    /// Parses a footer candidate at one distance from the end of the file.
    ///
    /// `nusakh` is the set of versions that distance is valid for, or [`None`]
    /// when the caller is scanning and has no expectation. `sarim` demands a
    /// non-empty index, which the scan path needs and the table path does not —
    /// an empty pak is legal, just useless.
    fn min_mawdi(
        dhayl: &[u8],
        tul: u64,
        izaha: u64,
        nusakh: Option<&[u32]>,
        sarim: bool,
    ) -> Option<Self> {
        let tul_dhayl = u64::try_from(dhayl.len()).ok()?;
        let mawdi = usize::try_from(tul_dhayl.checked_sub(izaha)?).ok()?;

        if u32_min(dhayl, mawdi)? != TAWQI_PAK {
            return None;
        }
        let nuskha = u32_min(dhayl, mawdi.checked_add(4)?)?;
        match nusakh {
            Some(masmuha) if !masmuha.contains(&nuskha) => return None,
            Some(_) => {},
            None => {
                if nuskha == 0 || nuskha > AQSA_NUSKHAT_MAQBULA {
                    return None;
                }
            },
        }

        let mawdi_fahras = u64_min(dhayl, mawdi.checked_add(8)?)?;
        let hajm_fahras = u64_min(dhayl, mawdi.checked_add(16)?)?;
        // The whole point of the bounds check: a footer is data from a file that
        // may be truncated, corrupt, or not a pak at all, and an index offset is
        // about to be trusted. It is trusted only after it is proven to name a
        // region that exists.
        if mawdi_fahras.checked_add(hajm_fahras)? > tul {
            return None;
        }
        if sarim && hajm_fahras == 0 {
            return None;
        }

        let mushaffar = mawdi
            .checked_sub(1)
            .and_then(|mawdi| dhayl.get(mawdi))
            .is_some_and(|bayt| *bayt != 0 && nuskha >= 4);
        let muarrif_miftah = if nuskha >= 7 {
            Self::muarrif_miftah(dhayl, mawdi)
        } else {
            None
        };
        let khanat_dagt = match izaha {
            172 => KHANAT_ARBA,
            204 | 205 => KHANAT_KHAMS,
            _ => usize::from(nuskha >= 8).checked_mul(KHANAT_KHAMS)?,
        };

        Some(Self {
            nuskha,
            mawdi_fahras,
            hajm_fahras,
            mushaffar,
            muarrif_miftah,
            khanat_dagt,
            hajm_malaf: tul,
            izahat_tawqi: izaha,
            bil_mash: nusakh.is_none(),
        })
    }

    /// Reads the sixteen bytes of `EncryptionKeyGuid` that precede
    /// `bEncryptedIndex`, and formats them the way Unreal prints an `FGuid`:
    /// four big-endian-printed 32-bit words, thirty-two uppercase hex digits, no
    /// separators. That is the form a user sees in their project's crypto
    /// settings, so it is the form Taarib asks them for.
    fn muarrif_miftah(dhayl: &[u8], mawdi: usize) -> Option<String> {
        let bidaya = mawdi.checked_sub(17)?;
        let awwal = u32_min(dhayl, bidaya)?;
        let thani = u32_min(dhayl, bidaya.checked_add(4)?)?;
        let thalith = u32_min(dhayl, bidaya.checked_add(8)?)?;
        let rabi = u32_min(dhayl, bidaya.checked_add(12)?)?;
        if awwal == 0 && thani == 0 && thalith == 0 && rabi == 0 {
            return None;
        }
        Some(format!("{awwal:08X}{thani:08X}{thalith:08X}{rabi:08X}"))
    }

    /// The engine version range this container's format implies.
    #[must_use]
    pub const fn mada(&self) -> MadaIsdar {
        mada_min_pak(self.nuskha, self.khanat_dagt)
    }

    /// Whether the index cannot be read without a key the user supplies.
    ///
    /// Phase 8 refuses to write a patch pak for a game whose own index it
    /// cannot read, and the capability report says so before the user commits
    /// to anything.
    #[must_use]
    pub const fn yahtaj_miftah(&self) -> bool {
        self.mushaffar
    }
}

/// A version range, because a container format almost never names one release.
///
/// Both bounds are inclusive. `adna == aqsa` means the range collapsed to a
/// point, which happens for pak versions 4, 6, 7, 9 and 10 and for nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadaIsdar {
    /// The lowest engine version the format is known to have been written by.
    pub adna: (u16, u16),
    /// The highest.
    pub aqsa: (u16, u16),
}

impl MadaIsdar {
    /// Whether both ends of the range sit in the same major version, which is
    /// the only case where this detector is willing to name one.
    #[must_use]
    pub const fn majur_wahid(self) -> bool {
        self.adna.0 == self.aqsa.0
    }

    /// The range as an [`IsdarMuharrik`], or nothing when it spans two majors.
    ///
    /// The raw string is marked `derived` and names the container version it
    /// came from, so that no downstream reader — a signature database, a patch
    /// compatibility check, a bug report — mistakes an inference for a version
    /// string found inside the game. The numeric fields carry the *lower* bound,
    /// which is the conservative choice: an adapter that gates a feature on
    /// "4.25 or later" should not enable it on the strength of a guess.
    #[must_use]
    pub fn isdar(self, masdar: &str) -> Option<IsdarMuharrik> {
        if !self.majur_wahid() {
            return None;
        }
        let khaam = if self.adna == self.aqsa {
            format!("UE {}.{} ({masdar}, derived)", self.adna.0, self.adna.1)
        } else {
            format!(
                "UE {}.{}-{}.{} ({masdar}, derived)",
                self.adna.0, self.adna.1, self.aqsa.0, self.aqsa.1
            )
        };
        // The only reading in the product that is inferred rather than read: a
        // container format version names a span of releases, so these numbers
        // are the bottom of a range and not a version any file states.
        Some(IsdarMuharrik {
            kabir: self.adna.0,
            sagheer: self.adna.1,
            tasheeh: 0,
            khaam,
            mushtaqq: true,
        })
    }
}

/// The pak version to engine version mapping from the module documentation.
///
/// `khanat` is the compression-method slot count, which is the only thing that
/// separates 4.22 from 4.23 and 4.24 — they share pak version 8 and differ only
/// in whether the footer reserves four name slots or five.
///
/// A version above [`AQSA_NUSKHAT_PAK`] is a format written after this build.
/// It is reported as UE5-and-later rather than refused, because an engine
/// released after this binary is still an Unreal game and the user is still
/// entitled to a capability report.
const fn mada_min_pak(nuskha: u32, khanat: usize) -> MadaIsdar {
    match nuskha {
        1 => MadaIsdar {
            adna: (4, 0),
            aqsa: (4, 0),
        },
        2 => MadaIsdar {
            adna: (4, 0),
            aqsa: (4, 2),
        },
        3 => MadaIsdar {
            adna: (4, 3),
            aqsa: (4, 15),
        },
        4 => MadaIsdar {
            adna: (4, 16),
            aqsa: (4, 16),
        },
        5 => MadaIsdar {
            adna: (4, 17),
            aqsa: (4, 19),
        },
        6 => MadaIsdar {
            adna: (4, 20),
            aqsa: (4, 20),
        },
        7 => MadaIsdar {
            adna: (4, 21),
            aqsa: (4, 21),
        },
        8 if khanat == 4 => MadaIsdar {
            adna: (4, 22),
            aqsa: (4, 22),
        },
        8 => MadaIsdar {
            adna: (4, 23),
            aqsa: (4, 24),
        },
        9 => MadaIsdar {
            adna: (4, 25),
            aqsa: (4, 25),
        },
        10 => MadaIsdar {
            adna: (4, 26),
            aqsa: (4, 26),
        },
        // Version 11 is the one that cannot be narrowed: 4.27 wrote it, and so
        // does every UE5 release this build knows of. The range deliberately
        // spans two majors so that `isdar` refuses to name one.
        11 => MadaIsdar {
            adna: (4, 27),
            aqsa: (5, 99),
        },
        _ => MadaIsdar {
            adna: (5, 0),
            aqsa: (5, 99),
        },
    }
}

// ---------------------------------------------------------------------------
// Engine/Build/Build.version
//
// The only exact engine version a packaged Unreal game is guaranteed to be able
// to state about itself, and the only one available at all for a title whose
// executable carries no `FEngineVersion` branch tag — which is not a rare
// build, it is what Little Nightmares ships. Everything else this module reads
// is a container format's version number, and a container format's version
// number covers a range of engine releases.
// ---------------------------------------------------------------------------

/// `Build.version` as the engine's build system writes it.
///
/// Only the three numbers are read. `Changelist`, `IsLicenseeVersion`,
/// `BranchName` and the compatible-version fields a newer engine adds are
/// ignored rather than refused, so a field appearing after this build was
/// written cannot turn a readable version into no version at all.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct MalafIsdarBina {
    #[serde(rename = "MajorVersion")]
    kabir: i64,
    #[serde(rename = "MinorVersion")]
    sagheer: i64,
    #[serde(rename = "PatchVersion", default)]
    tasheeh: i64,
}

/// An exact engine version, read rather than inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsdarBina {
    /// Major version.
    pub kabir: u16,
    /// Minor version.
    pub sagheer: u16,
    /// Patch version.
    pub tasheeh: u16,
}

impl IsdarBina {
    /// Reads `<root>/Engine/Build/Build.version`.
    ///
    /// Returns [`None`] for every ordinary reason a file is not there or not
    /// readable, and also for a version whose major is not 4 or 5. The last one
    /// matters: this file is JSON with three integer fields and no magic, so a
    /// coincidental `MajorVersion` of 2021 would otherwise be reported as an
    /// engine version with more confidence than anything else in this module.
    #[must_use]
    pub fn iqra(jidhr: &Path) -> Option<Self> {
        let mut masar = jidhr.to_path_buf();
        for juz in MASAR_ISDAR_BINA {
            masar.push(juz);
        }
        let (bayt, tul) = iqra_rass(&masar, HAJM_ISDAR_BINA)?;
        // A file longer than the cap was truncated by the read, and truncated
        // JSON does not parse; refusing it here says so before serde has to.
        if tul > u64::try_from(HAJM_ISDAR_BINA).ok()? {
            return None;
        }
        let malaf: MalafIsdarBina = serde_json::from_slice(&bayt).ok()?;
        let kabir = u16::try_from(malaf.kabir).ok()?;
        if kabir != 4 && kabir != 5 {
            return None;
        }
        Some(Self {
            kabir,
            sagheer: u16::try_from(malaf.sagheer).ok()?,
            tasheeh: u16::try_from(malaf.tasheeh).unwrap_or(0),
        })
    }

    /// The version as the rest of the product carries one.
    ///
    /// The raw string is the three numbers and the file they came from, and it
    /// is deliberately *not* marked `derived`: this is a point value the build
    /// wrote about itself, which is the same class of fact as the executable's
    /// `++UE4+Release-` tag and a different class from every range in this
    /// module.
    #[must_use]
    pub fn isdar(self) -> IsdarMuharrik {
        IsdarMuharrik {
            kabir: self.kabir,
            sagheer: self.sagheer,
            tasheeh: self.tasheeh,
            khaam: format!(
                "UE {}.{}.{} (Engine/Build/Build.version)",
                self.kabir, self.sagheer, self.tasheeh
            ),
            mushtaqq: false,
        }
    }

    /// Which generation this version belongs to.
    #[must_use]
    pub const fn jeel(self) -> JeelUnreal {
        if self.kabir >= 5 {
            JeelUnreal::Khamis
        } else {
            JeelUnreal::Rabi
        }
    }
}

// ---------------------------------------------------------------------------
// The .utoc header
// ---------------------------------------------------------------------------

/// A UE5 IoStore table-of-contents header, read from the first 144 bytes of a
/// `.utoc`.
///
/// The `.ucas` beside it holds the data and has no header of its own worth
/// reading, so it is treated here as corroboration for the `.utoc` rather than
/// as a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TarwisatUtoc {
    /// `EIoStoreTocVersion`. 1 and 2 predate UE5; 3 and up are UE5-era.
    pub nuskha: u8,
    /// The header size the file declares. 144 in every version observed.
    pub hajm_tarwisa: u32,
    /// How many chunks the TOC indexes.
    pub adad_madakhil: u32,
    /// The compression block size in bytes.
    pub hajm_kutla: u32,
    /// How many partitions the container is split across.
    pub adad_aqsam: u32,
    /// The container's own identifier.
    pub muarrif_hawiya: u64,
    /// `EIoContainerFlags`, raw.
    pub aalam: u8,
}

impl TarwisatUtoc {
    /// `EIoContainerFlags::Compressed`.
    const ALAM_MADGHUT: u8 = 1 << 0;
    /// `EIoContainerFlags::Encrypted`.
    const ALAM_MUSHAFFAR: u8 = 1 << 1;
    /// `EIoContainerFlags::Signed`.
    const ALAM_MAMHUR: u8 = 1 << 2;
    /// `EIoContainerFlags::Indexed`.
    const ALAM_MUFAHRAS: u8 = 1 << 3;

    /// Reads a `.utoc` header, or reports that the file is not one.
    ///
    /// The magic is decisive — sixteen bytes of `-==-` repeated is not a pattern
    /// that occurs by accident — but it is not trusted alone: the declared
    /// header size has to be sane and the file has to be at least as long as the
    /// header it claims.
    #[must_use]
    pub fn iqra(masar: &Path) -> Option<Self> {
        let (rass, tul) = iqra_rass(masar, HAJM_TARWISAT_UTOC)?;
        if !rass.starts_with(TAWQI_UTOC) {
            return None;
        }
        if tul < u64::try_from(HAJM_TARWISAT_UTOC).ok()? {
            return None;
        }
        let nuskha = *rass.get(16)?;
        let hajm_tarwisa = u32_min(&rass, 20)?;
        // 144 is what every observed version writes. The band around it accepts
        // a future header that grew without accepting a file whose first bytes
        // merely happened to match.
        if nuskha == 0 || !(64..=4096).contains(&hajm_tarwisa) {
            return None;
        }
        Some(Self {
            nuskha,
            hajm_tarwisa,
            adad_madakhil: u32_min(&rass, 24)?,
            hajm_kutla: u32_min(&rass, 44)?,
            adad_aqsam: u32_min(&rass, 52)?,
            muarrif_hawiya: u64_min(&rass, 56)?,
            aalam: *rass.get(80)?,
        })
    }

    /// Whether the container's chunks are AES-encrypted.
    #[must_use]
    pub const fn mushaffara(&self) -> bool {
        self.aalam & Self::ALAM_MUSHAFFAR != 0
    }

    /// Whether the container's chunks are compressed. Relevant to Phase 8
    /// because Oodle-compressed chunks are decompressed through the game's own
    /// decompressor, never through one Taarib has no licence to ship.
    #[must_use]
    pub const fn madghuta(&self) -> bool {
        self.aalam & Self::ALAM_MADGHUT != 0
    }

    /// Whether the container is signed, which means the game verifies its own
    /// chunk hashes at runtime.
    #[must_use]
    pub const fn mamhura(&self) -> bool {
        self.aalam & Self::ALAM_MAMHUR != 0
    }

    /// Whether the container carries a directory index, without which a chunk
    /// can be read but not named.
    #[must_use]
    pub const fn mufahrasa(&self) -> bool {
        self.aalam & Self::ALAM_MUFAHRAS != 0
    }

    /// The engine version range this TOC version implies.
    ///
    /// **Inferred, and widely.** `EIoStoreTocVersion`'s member order is public;
    /// the release each member landed in is not. Only the split at version 3 is
    /// firm enough to lean on: `PartitionSize` is a UE5-era addition, and a TOC
    /// below it is IoStore as it existed in 4.25 through 4.27.
    #[must_use]
    pub const fn mada(&self) -> MadaIsdar {
        match self.nuskha {
            1 => MadaIsdar {
                adna: (4, 25),
                aqsa: (4, 26),
            },
            2 => MadaIsdar {
                adna: (4, 26),
                aqsa: (5, 0),
            },
            3 => MadaIsdar {
                adna: (5, 0),
                aqsa: (5, 0),
            },
            4 | 5 => MadaIsdar {
                adna: (5, 0),
                aqsa: (5, 2),
            },
            6 => MadaIsdar {
                adna: (5, 3),
                aqsa: (5, 3),
            },
            7 => MadaIsdar {
                adna: (5, 4),
                aqsa: (5, 4),
            },
            8 => MadaIsdar {
                adna: (5, 5),
                aqsa: (5, 5),
            },
            _ => MadaIsdar {
                adna: (5, 5),
                aqsa: (5, 99),
            },
        }
    }

    /// Whether this TOC settles the UE4-versus-UE5 question on its own.
    ///
    /// Version 3 and up do. Versions 1 and 2 do not, and saying so is the
    /// difference between an honest report and a confident wrong one — IoStore
    /// shipped on console in 4.26 and 4.27, and those containers exist.
    #[must_use]
    pub const fn tahsim_khamis(&self) -> bool {
        self.nuskha >= 3
    }
}

/// Which generation of Unreal a game belongs to.
///
/// Phase 8 uses one adapter with two code paths, and this is what it dispatches
/// on: UE5's IoStore containers, its `FText` changes, and its Slate font service
/// differences all hang off it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JeelUnreal {
    /// Unreal Engine 4.
    Rabi,
    /// Unreal Engine 5.
    Khamis,
}

impl JeelUnreal {
    /// The generation's name as the report writes it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Rabi => "Unreal Engine 4",
            Self::Khamis => "Unreal Engine 5",
        }
    }
}

// ---------------------------------------------------------------------------
// The packaged layout
//
// A packaged Unreal game looks like this, and has since 4.0:
//
//   <root>/<Project>.exe                          a shim, not the game
//   <root>/Engine/Binaries/Win64/*.dll            the engine's own modules
//   <root>/Manifest_NonUFSFiles_Win64.txt         staging leftovers
//   <root>/<Project>/Binaries/Win64/
//              <Project>-Win64-Shipping.exe       the real process
//   <root>/<Project>/Content/Paks/
//              <Project>-WindowsNoEditor.pak      UE4 naming
//              pakchunk0-Windows.pak              chunked builds
//              global.utoc / global.ucas          UE5 IoStore
//   <root>/<Project>/Content/Localization/Game/<culture>/<Project>.locres
//
// Two things bend that shape often enough to plan for. Store wrappers add a
// directory above the project, which is why the search goes two levels deep.
// And macOS puts the shipping binary inside `<Project>.app/Contents/MacOS/`,
// which is a directory where every other platform has a file.
// ---------------------------------------------------------------------------

/// Everything the directory layout yielded, before any of it becomes evidence.
#[derive(Debug, Default)]
struct BinyaUnreal {
    /// The project directory, relative to the game root.
    mashru: Option<PathBuf>,
    /// The absolute `Content/Paks` directory, when there is one.
    mujallad_hawiyat: Option<PathBuf>,
    /// Every `.pak` found there, capped at [`AQSA_HAWIYAT`].
    paks: Vec<PathBuf>,
    /// Every `.utoc` found there, capped at [`AQSA_HAWIYAT`].
    utocs: Vec<PathBuf>,
    /// How many `.ucas` files sat beside them.
    adad_ucas: usize,
    /// The executable the packaged layout points at.
    tanfidhi: Option<TanfidhiMashru>,
    /// Whether `Engine/Binaries` exists.
    binaryat_muharrik: bool,
    /// The staging manifests left at the root, by name.
    manifestat: Vec<String>,
    /// The `Content/Localization` directory, when there is one.
    mujallad_taarib: Option<PathBuf>,
    /// How many loose `.locres` files were counted, capped at [`AQSA_LOCRES`].
    adad_locres: usize,
    /// The culture directories seen under `Content/Localization/*/`.
    thaqafat: Vec<String>,
}

impl BinyaUnreal {
    /// Whether the layout is Unreal-shaped without anything having confirmed it.
    ///
    /// A directory holding both `Binaries` and `Content` but no `Content/Paks`,
    /// no `Engine/Binaries` and no shipping executable. That is what a partially
    /// installed game looks like, and what a game whose developer happened to
    /// name two directories the way Unreal does also looks like. Worth
    /// recording, worth almost nothing as evidence.
    const fn unreal_shakl(&self) -> bool {
        self.mashru.is_some() && self.mujallad_hawiyat.is_none() && !self.binaryat_muharrik
    }
}

/// Walks the layout once and reports what it found.
fn tafahhus_binya(jidhr: &Path, mizaniya: &mut Mizaniya) -> BinyaUnreal {
    let mut binya = BinyaUnreal::default();
    let judhur_awwaliya = mizaniya.madakhil(jidhr);

    for madkhal in &judhur_awwaliya {
        if madkhal.mujallad && madkhal.ism.eq_ignore_ascii_case("Engine") {
            binya.binaryat_muharrik = mujallad_haqiqi(&madkhal.masar.join("Binaries"));
        } else if !madkhal.mujallad
            && madkhal.ism.starts_with("Manifest_")
            && imtidad(&madkhal.ism, "txt")
        {
            binya.manifestat.push(madkhal.ism.clone());
        }
    }

    let Some(masar_mashru) = mujallad_mashru(jidhr, &judhur_awwaliya, mizaniya) else {
        return binya;
    };
    binya.mashru = masar_mashru.strip_prefix(jidhr).ok().map(Path::to_path_buf);

    let mujallad_muhtawa = masar_mashru.join("Content");
    let mujallad_hawiyat = mujallad_muhtawa.join("Paks");
    if mujallad_haqiqi(&mujallad_hawiyat) {
        iltiqat_hawiyat(&mujallad_hawiyat, mizaniya, &mut binya);
        binya.mujallad_hawiyat = Some(mujallad_hawiyat);
    }

    let mujallad_taarib = mujallad_muhtawa.join("Localization");
    if mujallad_haqiqi(&mujallad_taarib) {
        iltiqat_taarib(&mujallad_taarib, mizaniya, &mut binya);
        binya.mujallad_taarib = Some(mujallad_taarib);
    }

    binya.tanfidhi = tanfidhi_shipping(&masar_mashru, mizaniya);
    binya
}

/// Finds the project directory: the one holding `Content/Paks`, or failing
/// that, the one holding both `Binaries` and `Content`.
///
/// The second test exists for games shipped without a pak — a cooked loose
/// build, which is rare in a store but common in a demo — and it is deliberately
/// second, because `Content/Paks` is the thing Phase 8 actually writes into.
fn mujallad_mashru(jidhr: &Path, madakhil: &[Madkhal], mizaniya: &mut Mizaniya) -> Option<PathBuf> {
    let mut tabaqa: Vec<PathBuf> = madakhil
        .iter()
        .filter(|madkhal| madkhal.mujallad && !madkhal.ism.eq_ignore_ascii_case("Engine"))
        .map(|madkhal| madkhal.masar.clone())
        .collect();

    for _ in 0..UMQ_BAHTH_MASHRU {
        if let Some(mawjud) = tabaqa
            .iter()
            .find(|masar| mujallad_haqiqi(&masar.join("Content").join("Paks")))
        {
            return Some(mawjud.clone());
        }
        if let Some(mawjud) = tabaqa.iter().find(|masar| {
            mujallad_haqiqi(&masar.join("Binaries")) && mujallad_haqiqi(&masar.join("Content"))
        }) {
            return Some(mawjud.clone());
        }
        // Nothing at this level. Descend one, which is what a store wrapper
        // that added a directory above the project looks like.
        let mut ab_ad: Vec<PathBuf> = Vec::new();
        for masar in &tabaqa {
            for madkhal in mizaniya.madakhil(masar) {
                if madkhal.mujallad {
                    ab_ad.push(madkhal.masar);
                }
            }
        }
        if ab_ad.is_empty() {
            break;
        }
        tabaqa = ab_ad;
    }

    // The root can itself be the project directory when a game was staged
    // without its wrapper level.
    if mujallad_haqiqi(&jidhr.join("Content").join("Paks")) {
        return Some(jidhr.to_path_buf());
    }
    None
}

/// Collects the containers in `Content/Paks`, stopping at [`AQSA_HAWIYAT`] of
/// each kind.
fn iltiqat_hawiyat(mujallad: &Path, mizaniya: &mut Mizaniya, binya: &mut BinyaUnreal) {
    for madkhal in mizaniya.madakhil(mujallad) {
        if madkhal.mujallad {
            continue;
        }
        if imtidad_huwa(&madkhal.masar, "pak") {
            if binya.paks.len() < AQSA_HAWIYAT {
                binya.paks.push(madkhal.masar);
            }
        } else if imtidad_huwa(&madkhal.masar, "utoc") {
            if binya.utocs.len() < AQSA_HAWIYAT {
                binya.utocs.push(madkhal.masar);
            }
        } else if imtidad_huwa(&madkhal.masar, "ucas") {
            binya.adad_ucas = binya.adad_ucas.saturating_add(1);
        }
    }
}

/// Counts loose `.locres` files and names the cultures they sit under.
///
/// Two levels below `Content/Localization`: the target (`Game`, `Engine`, or a
/// plugin's own) and then the culture (`en`, `fr`, `ar`). Deeper than that is
/// not a layout Unreal produces.
fn iltiqat_taarib(mujallad: &Path, mizaniya: &mut Mizaniya, binya: &mut BinyaUnreal) {
    for hadaf in mizaniya.madakhil(mujallad) {
        if !hadaf.mujallad {
            continue;
        }
        for thaqafa in mizaniya.madakhil(&hadaf.masar) {
            if !thaqafa.mujallad {
                if imtidad_huwa(&thaqafa.masar, "locres") {
                    binya.adad_locres = binya.adad_locres.saturating_add(1);
                }
                continue;
            }
            if !binya.thaqafat.contains(&thaqafa.ism) {
                binya.thaqafat.push(thaqafa.ism.clone());
            }
            if binya.adad_locres >= AQSA_LOCRES {
                continue;
            }
            for malaf in mizaniya.madakhil(&thaqafa.masar) {
                if !malaf.mujallad && imtidad_huwa(&malaf.masar, "locres") {
                    binya.adad_locres = binya.adad_locres.saturating_add(1);
                }
            }
        }
    }
}

/// The executable the packaged layout points at, and how it was recognised.
///
/// The distinction is not cosmetic. A name carrying Unreal's own
/// `-<Platform>-Shipping` suffix is generated by the engine's build system and
/// is proof of Unreal on its own; a file found only by *where it sits* is a
/// strong guess about which process to inject into and no evidence at all about
/// which engine wrote it. The two therefore reach the evidence trail with
/// different sentences and different weights, and this flag is what keeps them
/// apart.
#[derive(Debug, Clone)]
struct TanfidhiMashru {
    /// The executable.
    masar: PathBuf,
    /// The architecture the platform directory implies.
    mimariya: Mimariya,
    /// Whether the file's own name carried the shipping suffix.
    bi_lahiqa: bool,
}

/// Finds the real process under `<Project>/Binaries/<Platform>/`.
///
/// Two passes, in this order, because they are not equally good evidence:
///
/// 1. **`<Project>-<Platform>-Shipping[.exe]`.** The name is the evidence:
///    Unreal's build system generates it, no other engine does, and it names
///    both the project and the architecture. On macOS the entry is a `.app`
///    bundle and the binary inside it is what gets injected, so that is what is
///    returned. Every platform directory is searched before the second pass
///    starts, so a game that has the suffix somewhere never falls back.
/// 2. **The largest plausible executable in the platform directory.** Renaming
///    the shipping binary is a shipping practice, not a corner case: Little
///    Nightmares ships `Atlas/Binaries/Win64/LittleNightmares.exe` and no file
///    with the suffix anywhere. Without this pass that game's real binary is
///    never opened by anyone — discovery names it only by luck, and when
///    discovery names the root shim instead, nothing reads the game's own code
///    at all.
///
/// The second pass is bounded by three things rather than by a name: the
/// directory has to be one of Unreal's own platform directories, the file has to
/// carry that platform's executable extension, and it has to be at least
/// [`AQALL_HAJM_TANFIDH`]. A file whose stem matches the project directory wins
/// a tie, then the largest wins.
fn tanfidhi_shipping(mujallad_mashru: &Path, mizaniya: &mut Mizaniya) -> Option<TanfidhiMashru> {
    let binaryat = mujallad_mashru.join("Binaries");
    if !mujallad_haqiqi(&binaryat) {
        return None;
    }

    let manassat: Vec<(PathBuf, Mimariya, &str)> = MUJALLADAT_TANFIDH
        .iter()
        .map(|(ism, mimariya, lahiqa)| (binaryat.join(ism), *mimariya, *lahiqa))
        .filter(|(masar, _, _)| mujallad_haqiqi(masar))
        .collect();

    for (mujallad, _, _) in &manassat {
        if let Some(mawjud) = bi_lahiqat_shipping(mujallad, mizaniya) {
            return Some(mawjud);
        }
    }

    let ism_mashru = mujallad_mashru
        .file_name()
        .and_then(std::ffi::OsStr::to_str);
    for (mujallad, mimariya, lahiqa) in &manassat {
        if let Some(masar) = akbar_tanfidhi(mujallad, lahiqa, ism_mashru, mizaniya) {
            return Some(TanfidhiMashru {
                masar,
                mimariya: *mimariya,
                bi_lahiqa: false,
            });
        }
    }
    None
}

/// The first pass: a name Unreal's build system generated.
fn bi_lahiqat_shipping(mujallad: &Path, mizaniya: &mut Mizaniya) -> Option<TanfidhiMashru> {
    for madkhal in mizaniya.madakhil(mujallad) {
        if !madkhal.mujallad {
            if let Some((_, mimariya)) = LAWAHIQ_SHIPPING
                .iter()
                .find(|(lahiqa, _)| madkhal.ism.ends_with(lahiqa))
            {
                return Some(TanfidhiMashru {
                    masar: madkhal.masar,
                    mimariya: *mimariya,
                    bi_lahiqa: true,
                });
            }
            continue;
        }
        if imtidad(&madkhal.ism, "app") {
            let jawf = madkhal.masar.join("Contents").join("MacOS");
            if let Some(dakhil) = mizaniya
                .madakhil(&jawf)
                .into_iter()
                .find(|malaf| !malaf.mujallad)
                .map(|malaf| malaf.masar)
            {
                return Some(TanfidhiMashru {
                    masar: dakhil,
                    mimariya: Mimariya::X8664,
                    bi_lahiqa: true,
                });
            }
        }
    }
    None
}

/// The second pass: the largest plausible executable in one platform directory.
///
/// `lahiqa` is that platform's executable extension, empty when its executables
/// carry none. `ism_mashru` is the project directory's name, which breaks a tie
/// in favour of `<Project>.exe` — the shape a renamed shipping binary most often
/// takes.
fn akbar_tanfidhi(
    mujallad: &Path,
    lahiqa: &str,
    ism_mashru: Option<&str>,
    mizaniya: &mut Mizaniya,
) -> Option<PathBuf> {
    let mut afdal: Option<(bool, u64, PathBuf)> = None;
    for madkhal in mizaniya.madakhil(mujallad) {
        if madkhal.mujallad || !tanfidhi_muhtamal(&madkhal.ism, lahiqa) {
            continue;
        }
        // `symlink_metadata`, not `metadata`: the entry is already known not to
        // be a link, and not following one keeps a link planted in the game
        // directory from deciding the answer with somebody else's file size.
        let Ok(bayanat) = std::fs::symlink_metadata(&madkhal.masar) else {
            continue;
        };
        let hajm = bayanat.len();
        if hajm < AQALL_HAJM_TANFIDH {
            continue;
        }
        let jidhr_ism = madkhal
            .ism
            .rsplit_once('.')
            .map_or(madkhal.ism.as_str(), |(qabl, _)| qabl);
        let yutabiq = ism_mashru.is_some_and(|mashru| jidhr_ism.eq_ignore_ascii_case(mashru));
        let murashah = (yutabiq, hajm, madkhal.masar);
        if afdal
            .as_ref()
            .is_none_or(|mawjud| (mawjud.0, mawjud.1) < (murashah.0, murashah.1))
        {
            afdal = Some(murashah);
        }
    }
    afdal.map(|(_, _, masar)| masar)
}

/// Whether a file name could be an executable for a platform.
///
/// An empty `lahiqa` means the platform's executables carry no extension, which
/// also rules out the shared libraries and data files that do carry one — a
/// Linux `Binaries/Linux` holds `libsteam_api.so` beside the game.
fn tanfidhi_muhtamal(ism: &str, lahiqa: &str) -> bool {
    if lahiqa.is_empty() {
        return !ism.contains('.');
    }
    imtidad(ism, lahiqa)
}

impl JeelUnreal {
    /// Decides UE4 from UE5 using only what the containers said.
    ///
    /// Returns [`None`] where the containers genuinely cannot settle it, which
    /// is one specific and very common case: pak version 11 with no IoStore
    /// container beside it. That is 4.27 and it is also every UE5 release with
    /// IoStore switched off, and the only thing that separates them is the
    /// version tag `dalail::thunai` reads out of the executable.
    #[must_use]
    pub const fn min_hawiyat(utoc: Option<&TarwisatUtoc>, pak: Option<&DhaylPak>) -> Option<Self> {
        if let Some(utoc) = utoc
            && utoc.tahsim_khamis()
        {
            return Some(Self::Khamis);
        }
        match pak {
            Some(pak) if pak.nuskha <= 10 && !pak.bil_mash => Some(Self::Rabi),
            Some(_) | None => None,
        }
    }
}

// ---------------------------------------------------------------------------
// The detector
// ---------------------------------------------------------------------------

/// The Unreal detector.
///
/// Holds no state, is cheap to construct, and can be run in any order against
/// any game — including games that are not Unreal, which is what it says about
/// almost all of them.
#[derive(Debug, Default, Clone, Copy)]
pub struct FahisUnreal;

impl FahisUnreal {
    /// A detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// The project directory, relative to the game root.
    ///
    /// Phase 8 calls this to know where to write its patch pak: the target is
    /// `<root>/<returned>/Content/Paks/`, and the project *name* is the last
    /// component. Returns [`None`] when the game has no Unreal-shaped project
    /// directory, which for a non-Unreal game is every time.
    #[must_use]
    pub fn mashru(jidhr: &Path) -> Option<PathBuf> {
        let mut mizaniya = Mizaniya::jadeeda();
        let madakhil = mizaniya.madakhil(jidhr);
        let mutlaq = mujallad_mashru(jidhr, &madakhil, &mut mizaniya)?;
        mutlaq.strip_prefix(jidhr).ok().map(Path::to_path_buf)
    }

    /// Collects loose `.pak` files at the root and one level under it.
    ///
    /// Only used when no `Content/Paks` was found. This is where the weak
    /// evidence lives: a file named `.pak` outside Unreal's own layout is as
    /// likely to be Chromium's resource bundle, a Quake-era archive, or an
    /// installer's payload as it is to be Unreal's.
    fn paks_shariduun(jidhr: &Path, mizaniya: &mut Mizaniya) -> Vec<PathBuf> {
        let mut paks = Vec::new();
        for madkhal in mizaniya.madakhil(jidhr) {
            if paks.len() >= AQSA_HAWIYAT {
                break;
            }
            if madkhal.mujallad {
                for dakhil in mizaniya.madakhil(&madkhal.masar) {
                    if !dakhil.mujallad
                        && imtidad_huwa(&dakhil.masar, "pak")
                        && paks.len() < AQSA_HAWIYAT
                    {
                        paks.push(dakhil.masar);
                    }
                }
            } else if imtidad_huwa(&madkhal.masar, "pak") {
                paks.push(madkhal.masar);
            }
        }
        paks
    }
}

/// A path relative to the game root, as the evidence trail records it.
fn nisbi(jidhr: &Path, masar: &Path) -> Option<String> {
    masar
        .strip_prefix(jidhr)
        .ok()
        .map(|nisbi| nisbi.display().to_string())
}

impl Fahis for FahisUnreal {
    fn ism(&self) -> &'static str {
        "unreal"
    }

    /// # Errors
    ///
    /// Returns [`KhataMuharrik::JidhrMafqud`] when the game directory is not
    /// there. Nothing else here is an error: a pak that will not parse, a
    /// `.utoc` from a version this build has never seen, a `Content/Paks`
    /// directory that cannot be listed — every one of those is the absence of
    /// evidence, and the correct report is what *was* seen.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        if !siyaq.jidhr.is_dir() {
            return Err(KhataMuharrik::JidhrMafqud {
                jidhr: siyaq.jidhr.to_path_buf(),
            }
            .into());
        }

        let mut hasila = HasilatFahs::la_shay();
        let mut mizaniya = Mizaniya::jadeeda();
        let binya = tafahhus_binya(siyaq.jidhr, &mut mizaniya);

        if binya.binaryat_muharrik {
            hasila.sajjil_aila(
                AilatMuharrik::Unreal,
                NawDaleel::BinyatMujallad,
                "Engine/Binaries exists: the engine's own modules ship beside the game, which \
                 is Unreal's packaged layout and nothing else's",
                Some("Engine/Binaries".to_owned()),
                85,
            );
        }
        if let Some(mashru) = binya.mashru.as_ref()
            && binya.mujallad_hawiyat.is_some()
        {
            let mawqi = mashru.join("Content").join("Paks").display().to_string();
            // An empty relative path means the game root is itself the project
            // directory, which happens when a game was staged without its
            // wrapper level. Phase 8 joins it and lands in the right place
            // either way; only the sentence needs a name to print.
            let ism_mashru = if mashru.as_os_str().is_empty() {
                "the game root".to_owned()
            } else {
                mashru.display().to_string()
            };
            hasila.sajjil_aila(
                AilatMuharrik::Unreal,
                NawDaleel::BinyatMujallad,
                format!(
                    "the project directory \"{ism_mashru}\" holds Content/Paks with {} pak and \
                     {} IoStore container(s); Phase 8 writes its patch pak here",
                    binya.paks.len(),
                    binya.utocs.len()
                ),
                Some(mawqi),
                88,
            );
        }
        if !binya.manifestat.is_empty() {
            hasila.sajjil_aila(
                AilatMuharrik::Unreal,
                NawDaleel::BinyatMujallad,
                format!(
                    "{} staging manifest(s) left at the root by Unreal's packaging step: {}",
                    binya.manifestat.len(),
                    binya.manifestat.join(", ")
                ),
                None,
                50,
            );
        }
        if let Some(tanfidhi) = binya.tanfidhi.as_ref() {
            let mawqi = nisbi(siyaq.jidhr, &tanfidhi.masar);
            if tanfidhi.bi_lahiqa {
                hasila.sajjil_aila(
                    AilatMuharrik::Unreal,
                    NawDaleel::BinyatMujallad,
                    "the shipping executable Unreal's build system generates; the root \
                     executable is a shim that relaunches it, so injection targets this one",
                    mawqi,
                    80,
                );
            } else {
                // Deliberately not `sajjil_aila`. The file was recognised by
                // where it sits, and where it sits is already recorded as
                // Unreal's layout by the observations above; claiming the family
                // a second time from a name that says nothing would be the same
                // fact counted twice under a stronger label than it earns.
                hasila.sajjil(
                    NawDaleel::BinyatMujallad,
                    "the largest executable in the project's own Binaries directory, which is \
                     where Unreal's packaged layout puts the real process; the name carries no \
                     -Shipping suffix, so this says which process to attach to and nothing \
                     about which engine built it",
                    mawqi,
                    45,
                );
            }
            hasila.tanfidhi = Some(tanfidhi.masar.clone());
            hasila.mimariya = Some(tanfidhi.mimariya);
        }

        let mut paks = binya.paks.clone();
        let kharij_binya = paks.is_empty() && binya.mujallad_hawiyat.is_none();
        if kharij_binya {
            paks = Self::paks_shariduun(siyaq.jidhr, &mut mizaniya);
        }

        let dhayl = qira_paks(siyaq.jidhr, &paks, kharij_binya, &mut hasila);
        let tarwisa = qira_utocs(siyaq.jidhr, &binya, &mut hasila);
        let bina = qira_isdar_bina(siyaq.jidhr, &mut hasila);
        istinbat_isdar(bina, dhayl.as_ref(), tarwisa.as_ref(), &mut hasila);
        taqreer_taarib(siyaq.jidhr, &binya, &mut hasila);

        if hasila.aila == Some(AilatMuharrik::Unreal) {
            hasila.khalfiya = Some(KhalfiyaBarmajiya::UnrealNative);
            // Slate is not optional and not a guess: it is Unreal's own
            // interface toolkit, it is linked into every shipping build, and
            // every `FText` the player reads goes through it. There is no
            // Unreal game without it, which is why this is the one text system
            // asserted from the identification alone.
            hasila.daa_itar(ItarNusus::Slate);
        } else if binya.unreal_shakl() {
            hasila.sajjil(
                NawDaleel::BinyatMujallad,
                "the directory has Unreal-shaped parts but no container, no Engine/Binaries and \
                 no shipping executable confirmed it",
                None,
                10,
            );
        }

        Ok(hasila)
    }
}

/// Reads pak footers until one parses, and records what they said.
///
/// `kharij_binya` marks the case where the paks were found loose rather than in
/// a `Content/Paks` directory, which is the only case where the extension alone
/// is worth anything — and it is worth 15.
fn qira_paks(
    jidhr: &Path,
    paks: &[PathBuf],
    kharij_binya: bool,
    hasila: &mut HasilatFahs,
) -> Option<DhaylPak> {
    let mut maqru: Option<DhaylPak> = None;
    for masar in paks.iter().take(AQSA_HAWIYAT) {
        let Some(dhayl) = DhaylPak::iqra(masar) else {
            continue;
        };
        let mawqi = nisbi(jidhr, masar);
        hasila.sajjil_aila(
            AilatMuharrik::Unreal,
            NawDaleel::TarwisatHawiya,
            format!(
                "pak footer: magic 0x5A6F12E1 at {} bytes from the end, format version {}, \
                 index {} bytes at offset {}{}",
                dhayl.izahat_tawqi,
                dhayl.nuskha,
                dhayl.hajm_fahras,
                dhayl.mawdi_fahras,
                if dhayl.bil_mash {
                    ", found by scan: this footer layout is newer than this build"
                } else {
                    ""
                }
            ),
            mawqi.clone(),
            92,
        );
        if dhayl.nuskha > AQSA_NUSKHAT_PAK {
            hasila.sajjil(
                NawDaleel::TarwisatHawiya,
                format!(
                    "pak format version {} is above {}, the highest this build has a mapping \
                     for; the container is still readable and the game is still Unreal, but the \
                     engine version behind it cannot be narrowed here",
                    dhayl.nuskha, AQSA_NUSKHAT_PAK
                ),
                mawqi.clone(),
                25,
            );
        }
        if dhayl.mushaffar {
            hasila.sajjil(
                NawDaleel::TarwisatHawiya,
                match dhayl.muarrif_miftah.as_ref() {
                    Some(muarrif) => format!(
                        "the pak index is AES-encrypted under key {muarrif}; the game's text \
                         cannot be extracted until the user supplies that key"
                    ),
                    None => "the pak index is AES-encrypted under the project's default key; \
                             the game's text cannot be extracted until the user supplies it"
                        .to_owned(),
                },
                mawqi,
                30,
            );
        }
        maqru = Some(dhayl);
        break;
    }

    if maqru.is_none() && !paks.is_empty() {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            if kharij_binya {
                format!(
                    "{} file(s) with a .pak extension, none of which carries an Unreal footer; \
                     Chromium resource bundles, Quake-era archives and several installers use \
                     the same extension, so on its own it means very little",
                    paks.len()
                )
            } else {
                format!(
                    "{} pak file(s) in Content/Paks whose footer would not parse: truncated, \
                     mid-download, or a container format newer than this build",
                    paks.len()
                )
            },
            None,
            15,
        );
    }
    maqru
}

/// Reads `.utoc` headers until one parses, and records what they said.
fn qira_utocs(jidhr: &Path, binya: &BinyaUnreal, hasila: &mut HasilatFahs) -> Option<TarwisatUtoc> {
    let mut maqru: Option<TarwisatUtoc> = None;
    for masar in binya.utocs.iter().take(AQSA_HAWIYAT) {
        let Some(tarwisa) = TarwisatUtoc::iqra(masar) else {
            continue;
        };
        let mawqi = nisbi(jidhr, masar);
        hasila.sajjil_aila(
            AilatMuharrik::Unreal,
            NawDaleel::TarwisatHawiya,
            format!(
                "IoStore table of contents: magic -==--==--==--==-, TOC version {}, {} chunks, \
                 {} partition(s), {}-byte compression blocks",
                tarwisa.nuskha, tarwisa.adad_madakhil, tarwisa.adad_aqsam, tarwisa.hajm_kutla
            ),
            mawqi.clone(),
            95,
        );
        let mut sifat: Vec<&str> = Vec::new();
        if tarwisa.madghuta() {
            sifat.push("compressed");
        }
        if tarwisa.mushaffara() {
            sifat.push("encrypted");
        }
        if tarwisa.mamhura() {
            sifat.push("signed");
        }
        if tarwisa.mufahrasa() {
            sifat.push("indexed");
        }
        if !sifat.is_empty() {
            hasila.sajjil(
                NawDaleel::TarwisatHawiya,
                format!(
                    "IoStore container flags: {}{}",
                    sifat.join(", "),
                    if tarwisa.mushaffara() {
                        " — the chunks cannot be read until the user supplies the AES key"
                    } else {
                        ""
                    }
                ),
                mawqi,
                30,
            );
        }
        maqru = Some(tarwisa);
        break;
    }

    if maqru.is_none() && !binya.utocs.is_empty() {
        hasila.sajjil_aila(
            AilatMuharrik::Unreal,
            NawDaleel::BinyatMujallad,
            format!(
                "{} .utoc file(s) whose header would not parse; the extension is IoStore's and \
                 nobody else's, so the game is Unreal, but the container is not readable here",
                binya.utocs.len()
            ),
            None,
            40,
        );
    }
    if binya.adad_ucas > 0 {
        hasila.sajjil_aila(
            AilatMuharrik::Unreal,
            NawDaleel::BinyatMujallad,
            format!(
                "{} .ucas data container(s) beside the table of contents",
                binya.adad_ucas
            ),
            None,
            60,
        );
    }
    maqru
}

/// Reads `Engine/Build/Build.version` and records what it said.
///
/// Weight 90, the same as the executable's `++UE4+Release-` tag in
/// `dalail::thunai`, and for the same reason: both are point values a build
/// wrote about itself, and nothing but Unreal ships a file at this path with
/// these fields. It sits below the container headers on purpose — a footer read
/// out of a fifty-gigabyte pak is stronger evidence *that the game is Unreal*
/// than a small JSON file is, even though the JSON file is far better evidence
/// of *which* Unreal.
fn qira_isdar_bina(jidhr: &Path, hasila: &mut HasilatFahs) -> Option<IsdarBina> {
    let bina = IsdarBina::iqra(jidhr)?;
    hasila.sajjil_aila(
        AilatMuharrik::Unreal,
        NawDaleel::BayanatMudmaja,
        format!(
            "Engine/Build/Build.version names engine version {}.{}.{} exactly; Unreal's build \
             system writes this file and the staging step ships it, so this is a point value \
             and not a range inferred from a container format",
            bina.kabir, bina.sagheer, bina.tasheeh
        ),
        Some(MASAR_ISDAR_BINA.join("/")),
        90,
    );
    Some(bina)
}

/// Turns what the game said about its own version into one, or into a plainly
/// stated refusal to name one.
///
/// **Exactness outranks weight.** `Build.version` wins over both container
/// ranges whatever their evidential weight, because a range is an inference over
/// several releases and a point value is what the build wrote. The same rule
/// across detectors — the executable's `++UE4+Release-` tag against a range from
/// this one — lives in [`crate::tahdid`], which is where the detectors meet.
fn istinbat_isdar(
    bina: Option<IsdarBina>,
    dhayl: Option<&DhaylPak>,
    tarwisa: Option<&TarwisatUtoc>,
    hasila: &mut HasilatFahs,
) {
    if let Some(tarwisa) = tarwisa {
        let mada = tarwisa.mada();
        let masdar = format!("IoStore TOC v{}", tarwisa.nuskha);
        if let Some(isdar) = mada.isdar(&masdar) {
            hasila.sajjil(
                NawDaleel::TarwisatHawiya,
                format!(
                    "TOC version {} maps to {isdar}; the mapping is inferred from the order of \
                     EIoStoreTocVersion's members, not from a published table, so it is a range \
                     and the executable's own version tag overrides it",
                    tarwisa.nuskha
                ),
                None,
                45,
            );
            hasila.isdar = Some(isdar);
        }
    }

    if let Some(dhayl) = dhayl {
        let mada = dhayl.mada();
        let masdar = format!("pak v{}", dhayl.nuskha);
        match mada.isdar(&masdar) {
            Some(isdar) => {
                hasila.sajjil(
                    NawDaleel::TarwisatHawiya,
                    format!(
                        "pak format version {} was written by {isdar}; several engine releases \
                         share one pak version, so this is a range and not a build",
                        dhayl.nuskha
                    ),
                    None,
                    45,
                );
                if hasila.isdar.is_none() {
                    hasila.isdar = Some(isdar);
                }
            },
            None => hasila.sajjil(
                NawDaleel::TarwisatHawiya,
                format!(
                    "pak format version {} spans UE 4.27 and every UE5 release, and no IoStore \
                     container settles it; the executable's ++UE4/++UE5 tag is the only source \
                     that can",
                    dhayl.nuskha
                ),
                None,
                30,
            ),
        }
    }

    // Last, so it overwrites both ranges. The observations above stay in the
    // trail — a maintainer diagnosing a wrong identification needs to see that
    // the pak said 4.27-or-UE5 and the build file said 4.27 — but the version
    // the product reports is the one the game stated about itself.
    if let Some(bina) = bina {
        hasila.isdar = Some(bina.isdar());
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            format!(
                "the engine version is taken from Build.version rather than from a container \
                 format, which places this game on {}",
                bina.jeel().ism()
            ),
            None,
            60,
        );
        return;
    }

    if let Some(jeel) = JeelUnreal::min_hawiyat(tarwisa, dhayl) {
        hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            format!("the containers place this game on {}", jeel.ism()),
            None,
            60,
        );
    }
}

/// Reports what the localization layout promises Phase 8, including when it
/// promises nothing.
fn taqreer_taarib(jidhr: &Path, binya: &BinyaUnreal, hasila: &mut HasilatFahs) {
    if binya.adad_locres > 0 {
        let mawqi = binya
            .mujallad_taarib
            .as_deref()
            .and_then(|masar| nisbi(jidhr, masar));
        hasila.sajjil_aila(
            AilatMuharrik::Unreal,
            NawDaleel::BayanatMudmaja,
            format!(
                "{} loose .locres file(s) under Content/Localization across {} culture \
                 director(ies){}; .locres is Unreal's own compiled string table and no other \
                 engine writes one",
                binya.adad_locres,
                binya.thaqafat.len(),
                if binya.thaqafat.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", binya.thaqafat.join(", "))
                }
            ),
            mawqi,
            70,
        );
        return;
    }

    if hasila.aila != Some(AilatMuharrik::Unreal) {
        return;
    }
    // Absence, recorded as absence. It is not evidence against Unreal — the
    // normal packaged layout puts every .locres inside the pak — but it decides
    // what the capability report may promise: with loose files, Phase 8's
    // reinjection has somewhere to land without touching a container; without
    // them, it has to read the pak first, and if that pak is encrypted it
    // cannot.
    hasila.sajjil(
        NawDaleel::BinyatMujallad,
        if binya.mujallad_taarib.is_some() {
            "Content/Localization exists but holds no loose .locres file; the game's strings \
             live inside its containers, so extraction has to read the pak"
        } else {
            "no Content/Localization directory and no loose .locres file; the game's strings \
             live inside its containers, so extraction has to read the pak"
        },
        None,
        5,
    );
}
