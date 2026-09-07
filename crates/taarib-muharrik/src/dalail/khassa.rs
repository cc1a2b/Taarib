//! المحرّكات الخاصة — six in-house engines, each read out of the containers it
//! ships and, where the binary carries one, the name the engine gives itself.
//!
//! None of these six is licensed to anybody, none has a plugin system, and four
//! of them have no published name at all. What they do have is file framing
//! their own runtime must be able to find, which is why a detector can name them
//! from bytes even when the executable has been put behind a protector.
//!
//! | family | the observation that names it |
//! | --- | --- |
//! | [`AilatMuharrik::Frostbite`] | a shipped DLL whose version resource `ProductName` is `Frostbite` |
//! | [`AilatMuharrik::BlackSpace`] | `bin64/pers.exe` whose `ProductName` is `BlackSpace version` |
//! | [`AilatMuharrik::Alchemy`] | `IGA\x1A` archives beside `IGZ` object files |
//! | [`AilatMuharrik::Dantelion`] | `Dantelion2` in the executable's read-only data, and `DCX\0` |
//! | [`AilatMuharrik::Rage`] | `RPF7` archives, and `[RAGE]` in the executable's read-only data |
//! | [`AilatMuharrik::Snowdrop`] | `sdf.sdftoc` opening `WEST` and naming `massive` inside it |
//!
//! Every number below was read off a shipped install rather than taken from a
//! wiki, and every count is given so that a claim resting on one file is visible
//! as one.
//!
//! ## Frostbite
//!
//! ### The version resource, which is the whole signature
//!
//! ```text
//! Engine.Render.Core2.PlatformPcDx12.dll   1 581 936 B
//!    CompanyName     "Electronic Arts"
//!    ProductName     "Frostbite"                          <-- the engine
//!    ProductVersion  "2.42.5"                             <-- and its release
//!    FileDescription "Frostbite rendering platform support"
//! ```
//!
//! This is the only one of the six that publishes a release number a reader can
//! check, so it is the only one this module turns into an
//! [`IsdarMuharrik`], and it is turned into one with `mushtaqq` left false
//! because it was read rather than inferred.
//!
//! The sibling `Engine.Render.Core2.PlatformVulkan.dll` carries **no** version
//! resource at all, which is why the module reads several `Engine.*` DLLs rather
//! than the first one it finds.
//!
//! ### `.toc` — the archive index. 87 of 87 agree.
//!
//! ```text
//! offset  size  field      notes
//! 0x00    3     magic      `00 D1 CE` — `0xD1CE` is DICE's own joke and the
//!                          only part of this header that is a constant
//! 0x03    1     variant    `01` in all 87 files here; the byte is recorded and
//!                          nothing is claimed about what it selects
//! 0x04    4     zero       four zero bytes, in all 87
//! ```
//!
//! Counted across `Data/layout.toc`, `Data/initfs_Win32`, the 41 tables under
//! `Data/Win32/`, and the same three kinds again under `Patch/`. The four zero
//! bytes behind the tag are half the strength of this signature: three arbitrary
//! bytes on their own would be a coincidence a compressed file could land on.
//!
//! ## Pearl Abyss `BlackSpace`
//!
//! ### The version resource
//!
//! ```text
//! bin64/pers.exe  3 989 400 B
//!    ProductName    "BlackSpace version"     <-- the engine
//!    LegalCopyright "Pearlabyss Corp"
//!    InternalName   "PERS.exe"
//!    FileVersion    "26.4.13.1"
//! ```
//!
//! `26.4.13.1` is **not** reported as the engine's version. It is the file
//! version of the patcher binary that happens to carry the engine's name in a
//! neighbouring field, `CompanyName` on that binary is empty, and a build stamp
//! presented as an engine release is the confident wrong answer this module
//! refuses everywhere.
//!
//! ### `.pamt` — the metadata table. 30 of 34 agree.
//!
//! ```text
//! offset  size  field    notes
//! 0x00    4     hash     different in every file; not read
//! 0x04    4     count    u32 LE, 1 to 36 across the set
//! 0x08    4     magic    u32 LE 0x610E0232 — the constant
//! 0x0C    4     zero
//! ```
//!
//! The other four `.pamt` files open with sixteen zero bytes and are stepped
//! over rather than counted against the signature: a file carrying the extension
//! and not the header is exactly what the header check is for.
//!
//! ### `.paz` — the content container. 44 of 192 open with `PAR `.
//!
//! The rest are raw payloads written straight into the extension: 73 begin
//! `DDS `, 10 `RIFF`, 3 `BKHD`. So `.paz` is a *name* for content rather than a
//! format, and only the ones that really are Pearl Abyss archives are recorded —
//! at a weight that says a three-letter tag plus a space is weaker evidence than
//! a 32-bit constant.
//!
//! ## Vicarious Visions Alchemy
//!
//! Two container tags, inherited from Intrinsic Graphics' middleware and never
//! renamed:
//!
//! ```text
//! archives/*.pak            49 47 41 1A  0B 00 00 00   "IGA\x1A", version 11
//!                           171 of 171 agree
//! MemoryConfiguration.igz   01 5A 47 49  0A 00 00 00   "IGZ\x01" byte-swapped,
//!                                                      version 10
//! ```
//!
//! **The `.pak` extension is the trap here.** It is also Unreal's, and a
//! detector that stopped at the extension would send an Unreal pak reader at a
//! file it cannot read one byte of. This module reads the tag and says what it
//! found, precisely so that the next reader does not have to take the claim on
//! trust — the same job [`crate::dalail::bio4`]'s `.arc` note does for MT
//! Framework.
//!
//! ## `FromSoftware` `Dantelion2`
//!
//! ### The engine naming itself
//!
//! The compiler put it there. Assertion strings in the shipped executable's
//! `.rdata` carry the engine's own build path:
//!
//! ```text
//! DARK SOULS: REMASTERED  N:\FRPG\Source\Dantelion2\dist\Include\dantelion2/...
//! ELDEN RING              W:\GR\RootBranch\Source\Library\Dantelion2\...
//! ```
//!
//! 41 occurrences in each, the first 196 239 bytes into `.rdata` in one image
//! and 100 368 bytes into it in the other — which is why the marker scan below
//! is bounded in megabytes rather than reading a whole executable.
//!
//! ### `DCX\0` — the compression wrapper. 4 576 of 4 576 agree in one game.
//!
//! ```text
//! offset  size  field       notes
//! 0x00    4     magic       `DCX\0`
//! 0x04    4     unknown     u32 BE — 0x00010000 in DARK SOULS, 0x00011000 in
//!                           ELDEN RING, so it is not read
//! 0x08    4     dcs offset  u32 BE — points at a `DCS\0` block
//! 0x0C    4     dcp offset  u32 BE — points at a `DCP\0` block
//! 0x10    4     unknown     u32 BE
//! 0x14    4     unknown     u32 BE
//! ...
//! dcp+4   4     format      four characters: `DFLT` here, `KRAK` in ELDEN RING
//! ```
//!
//! The two offsets are the signature and the magic is only the way in: a file
//! that says where its two sub-blocks are and is right about both is not a file
//! that landed on `DCX\0` by accident. Every one of the 4 576 `.dcx` files in
//! DARK SOULS: REMASTERED has a byte-identical first 24 bytes, and ELDEN RING's
//! `Game/Data2.bdt` — a twenty-gigabyte archive, not a `.dcx` at all — opens
//! with the same header shape and different values, which is why the check is
//! structural and not a byte comparison.
//!
//! ### `BND3` and `BDF3` — the archive pair
//!
//! Four characters and then an eight-character version string the engine writes
//! as text: `BND309G17X51`, `BDF307D7R6`. Recorded below `DCX\0` because eight
//! characters of arbitrary text is a weaker claim than two offsets that resolve.
//!
//! ### Where the text is
//!
//! `msg/<LANGUAGE>/item.msgbnd.dcx` and `menu.msgbnd.dcx`, ten languages of
//! them. Recorded as an observation for the reader, because it is the first
//! thing anybody asking what Taarib could do to this engine needs to know.
//!
//! ## Rockstar RAGE
//!
//! ### `RPF7` — the archive. 181 of 181 agree.
//!
//! ```text
//! offset  size  field       notes
//! 0x00    4     magic       `7FPR` on disk — `RPF7` stored little-endian
//! 0x04    4     entries     u32 LE, 34 to 8 641 across the set
//! 0x08    4     names       u32 LE, the name-heap length
//! 0x0C    4     encryption  u32 LE — 0x0FEFFFFF in 179 files and `OPEN`
//!                           (0x4E45504F) in 2. Both are accepted and no third
//!                           value is, because no third value was seen.
//! ```
//!
//! ### The engine naming itself
//!
//! `[RAGE] netKxThrPool %u` and `[RAGE] netThrPool %u` — log format strings in
//! `.rdata`, 3 502 341 bytes into the section. A bracketed subsystem tag is a
//! weaker kind of naming than a version resource field, and it is weighted as
//! one; it is only ever looked for in a game whose archives already said RAGE.
//!
//! ### The two smaller containers
//!
//! `title.rgl` opens `RGLM` and `rpf.cache` opens `HSHR`. Neither is understood
//! past its tag and neither is read past it.
//!
//! ## Ubisoft Snowdrop
//!
//! The hardest of the six from the executable and the easiest from the data.
//! `afop.exe` is 355 902 136 bytes behind a protector that rewrote its section
//! table into `.xpdata`, `.udata`, `.xtext`, `.ecode` and eleven more, so this
//! module does not read it at all. The data root says everything:
//!
//! ```text
//! rogue/sdf/pc/data/
//!   sdf.sdftoc      60 225 240 B   57 45 53 54 29 00 00 00 …   "WEST", version 41
//!                                  and the ASCII `massive` at offset 0x30
//!   sdf.sdfver             437 B   "0: 2318387 174095262542 …"
//!   1 380 × *.sdfdata              42 45 52 47 29 00 00 00     "BERG", version 41
//!   1 381 × *.sdfdata.hash
//! ```
//!
//! `WEST` and `BERG` are Massive Entertainment's own pair and the format version
//! behind each is the same number, which is the part that makes the two one
//! observation rather than two coincidences. The studio name inside the table of
//! contents is the third: `massive` is not a string another engine's index
//! carries.
//!
//! The data root is four directories below the game, one level past
//! [`crate::fahs::AQSA_UMQ`], because Snowdrop nests it under the project's own
//! codename. It is reached by *naming* each level rather than by walking — root,
//! then a directory holding `sdf`, then `sdf`'s platform directories, then
//! `data` — so a game that is not Snowdrop pays one listing per top-level
//! directory and nothing else.
//!
//! ## What this module deliberately does not read
//!
//! **Imports.** Every one of these games links a graphics module and several
//! link a compression library; all of that belongs to [`crate::dalail::thunai`],
//! which owns that evidence source for every engine.
//!
//! **The archives themselves.** Not one byte past a header is read. Four of
//! these six keep their text inside a container this build has no reader for,
//! and a detector that opened one would be doing the extractor's job badly.
//!
//! **A version, for five of the six.** `26.4.13.1` on a Pearl Abyss patcher,
//! `2.7.0.0` on `eldenring.exe`, `1.0.1158.13` on `GTA5_Enhanced.exe`,
//! container format versions 41, 11 and 10 — every one of them is recorded as
//! evidence and none is turned into an engine version, because a number that
//! parses is not the same thing as a number that means what a reader will assume
//! it means.
//!
//! ## What a miss means
//!
//! Nothing. Every listing, every descent and every file this module opens is
//! bounded, so a container that exists past those bounds was not looked at
//! rather than found absent.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use taarib_mustalahat::muharrik::{AilatMuharrik, IsdarMuharrik, NawDaleel};
use taarib_usus::khata::Natija;

use crate::dalail::imtidad;
use crate::dalail::tanfidhi::{
    self, BayanTanfidhi, bayan as bayan_tanfidhi, iqra_nafidha, raqm32_sagheer,
};
use crate::fahs::{Fahis, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

// ---------------------------------------------------------------------------
// Bounds. Every one is a ceiling, never a target.
// ---------------------------------------------------------------------------

/// Bytes taken from the front of a container file.
///
/// The longest header any detector here reads is the 44 bytes a `DCX\0` wrapper
/// needs before its compression tag. Sixty-four leaves room to look at the file
/// beside the layouts above in a hex dump without reading anything that could be
/// called content.
const HAJM_TARWISA_HAWIYA: usize = 64;

/// Bytes taken from the front of a `.sdftoc`, which carries its studio name
/// behind a fixed header rather than at offset zero.
const HAJM_TARWISA_TOC: usize = 128;

/// The most of a read-only-data section a marker scan will read.
///
/// Eight megabytes. The three markers this module looks for sit 100 368,
/// 196 239 and 3 502 341 bytes into their images' `.rdata`, so eight covers all
/// three with room — and it is only ever read for a game whose containers have
/// already said which engine this is.
const HAJM_MASH_QISM: usize = 8 * 1024 * 1024;

/// How much of a section is carried between chunks of a marker scan, so that a
/// marker straddling a chunk boundary is still found.
const TADAKHUL_MASH: usize = 64;

/// How many entries of any one directory a scan will look at.
const AQSA_MADAKHIL_MUJALLAD: usize = 256;

/// How many files of each kind a scan will keep, and therefore open.
///
/// Three. One would make every signature rest on whichever file the filesystem
/// listed first; a hundred would read a hundred headers to learn what the third
/// already said.
const HISSAT_SINF: usize = 3;

/// How many subdirectories of a data directory a directed descent will enter.
///
/// Eight. Frostbite ships two platform directories under `Data/`, Snowdrop one
/// under `sdf/`, and this engine family's layouts do not fan out — a directory
/// with more than eight children at this level is not the one being looked for.
const AQSA_FURU: usize = 8;

/// How many binaries a detector will open a version resource in.
///
/// Four. Crimson Desert ships three executables beside each other and only one
/// of them carries the engine's name; EA SPORTS FC ships three `Engine.*` DLLs
/// and one of the three has no version resource at all.
const AQSA_MULAFFAT_ISDAR: usize = 4;

// ---------------------------------------------------------------------------
// Frostbite
// ---------------------------------------------------------------------------

/// The `ProductName` the engine's own rendering DLL declares.
const ISM_FROSTBITE: &str = "Frostbite";

/// The name prefix EA gives the engine's shipped modules, folded to lower case.
const SABIQAT_MUKAWWIN_FROSTBITE: &str = "engine.";

/// The three bytes at the head of every Frostbite table of contents.
///
/// `0xD1CE` is the studio's own joke and the only constant in the header.
const SIHR_TOC: [u8; 3] = [0x00, 0xD1, 0xCE];

/// The engine's data directory, folded to lower case.
const MUJALLAD_BAYANAT_FROSTBITE: &str = "data";

/// The patch directory beside it, folded to lower case.
const MUJALLAD_TARQEE_FROSTBITE: &str = "patch";

/// The name of the table of contents at the top of the data directory.
const MALAF_TARTEEB: &str = "layout.toc";

// ---------------------------------------------------------------------------
// Pearl Abyss BlackSpace
// ---------------------------------------------------------------------------

/// The word the engine's own binary puts in `ProductName`.
const ISM_BLACKSPACE: &str = "BlackSpace";

/// The company the same resource names.
const SHARIKAT_BLACKSPACE: &str = "PEARLABYSS";

/// The binary directory the engine ships its executables in, folded to lower
/// case.
const MUJALLAD_TANFIDH_BLACKSPACE: &str = "bin64";

/// The metadata directory beside the numbered content directories.
const MUJALLAD_META_BLACKSPACE: &str = "meta";

/// The 32-bit constant at offset 8 of every `.pamt` metadata table.
const SIHR_PAMT: u32 = 0x610E_0232;

/// The four-character tag a Pearl Abyss archive opens with.
const SIHR_PAZ: [u8; 4] = *b"PAR ";

/// The exact length of the engine's version stamp, `meta/0.paver`.
const TUL_PAVER: u64 = 10;

/// The most records a `.pamt` header may claim before the file is refused.
const AQSA_QUYUD_PAMT: u32 = 1_000_000;

// ---------------------------------------------------------------------------
// Vicarious Visions Alchemy
// ---------------------------------------------------------------------------

/// The archive tag, as the bytes appear on disk.
const SIHR_IGA: [u8; 4] = [0x49, 0x47, 0x41, 0x1A];

/// The object-file tag, byte-swapped exactly as the shipped files store it.
const SIHR_IGZ: [u8; 4] = [0x01, 0x5A, 0x47, 0x49];

/// The directory the archives live in, folded to lower case.
const MUJALLAD_ARSHIF_ALCHEMY: &str = "archives";

/// The bootstrap file the engine reads at startup.
const MALAF_IQLA_ALCHEMY: &str = "build.xml";

/// The element that file opens with, and a field only this engine's bootstrap
/// carries.
const IBARAT_IQLA_ALCHEMY: [&str; 2] = ["<Bootstrap>", "SaveFileGameIdentifier"];

/// The most bytes of the bootstrap file that are read.
const HAJM_IQLA_ALCHEMY: usize = 4 * 1024;

/// The highest container version either tag may declare before the file is
/// refused. The shipped files say 10 and 11.
const AQSA_ISDAR_IG: u32 = 64;

// ---------------------------------------------------------------------------
// FromSoftware Dantelion
// ---------------------------------------------------------------------------

/// The engine's own library name, as the compiler wrote it into the image.
const ALAMAT_DANTELION: &[u8] = b"Dantelion2";

/// The compression wrapper's tag.
const SIHR_DCX: [u8; 4] = *b"DCX\0";

/// The tag of the block the wrapper's header points at first.
const SIHR_DCS: [u8; 4] = *b"DCS\0";

/// The tag of the block it points at second, whose four bytes of tail name the
/// compression.
const SIHR_DCP: [u8; 4] = *b"DCP\0";

/// The archive tag, and the archive-data tag beside it.
const SIHR_BND: [u8; 4] = *b"BND3";

/// The detached-data half of the same pair.
const SIHR_BDF: [u8; 4] = *b"BDF3";

/// How many characters of version text the engine writes behind `BND3`/`BDF3`.
const TUL_ISDAR_BND: usize = 8;

/// The subdirectory ELDEN RING puts the whole game under, folded to lower case.
const MUJALLAD_LUBA_DANTELION: &str = "game";

/// The directories this engine's layout is recognised by, folded to lower case.
///
/// `msg` and `param` decide the weight and the rest are recorded: the first is
/// where every string in the game lives and the second is the engine's own
/// parameter tables, and a directory holding both is this engine's layout rather
/// than a folder somebody named after a game.
const MUJALLADAT_DANTELION: &[&str] = &[
    "msg", "param", "paramdef", "mtd", "chr", "menu", "facegen", "parts", "map",
];

/// The directories a container scan will descend into, folded to lower case.
const MUJALLADAT_MASH_DANTELION: &[&str] = &["msg", "chr", "menu", "facegen", "parts", "map"];

/// The stamp that carries every rule the engine loads at startup.
const MALAF_TANZEEM_DANTELION: &str = "regulation.bin";

/// The company the executable's version resource names.
const SHARIKAT_DANTELION: &str = "FROMSOFTWARE";

// ---------------------------------------------------------------------------
// Rockstar RAGE
// ---------------------------------------------------------------------------

/// The archive tag, as the bytes appear on disk: `RPF7` stored little-endian.
const SIHR_RPF: [u8; 4] = *b"7FPR";

/// The two encryption tags the shipped archives carry at offset 12.
///
/// 179 of 181 files here say the first and two say the second. A third value is
/// not accepted because a third value was not seen.
const AQNAAT_RPF: [u32; 2] = [0x0FEF_FFFF, 0x4E45_504F];

/// The most entries an archive header may claim before the file is refused.
const AQSA_QUYUD_RPF: u32 = 4_000_000;

/// The engine's own subsystem tag, as its log format strings write it.
const ALAMAT_RAGE: &[u8] = b"[RAGE]";

/// The startup list's tag.
const SIHR_RGL: [u8; 4] = *b"RGLM";

/// The archive cache's tag.
const SIHR_CACHE_RPF: [u8; 4] = *b"HSHR";

/// The startup list's file name, folded to lower case.
const MALAF_RGL: &str = "title.rgl";

/// The archive cache's file name, folded to lower case.
const MALAF_CACHE_RPF: &str = "rpf.cache";

/// The company the executable's version resource names.
const SHARIKAT_RAGE: &str = "ROCKSTAR";

// ---------------------------------------------------------------------------
// Ubisoft Snowdrop
// ---------------------------------------------------------------------------

/// The table of contents' tag.
const SIHR_SDFTOC: [u8; 4] = *b"WEST";

/// The content chunks' tag. The same format version stands behind both.
const SIHR_SDFDATA: [u8; 4] = *b"BERG";

/// The studio's own name, as ASCII inside the table of contents' header.
const ALAMAT_MASSIVE: &[u8] = b"massive";

/// The directory the engine's platform trees hang under, folded to lower case.
const MUJALLAD_SDF: &str = "sdf";

/// The leaf directory the chunks live in, folded to lower case.
const MUJALLAD_BAYANAT_SDF: &str = "data";

/// The highest format version either tag may declare before the file is refused.
/// The shipped files say 41.
const AQSA_ISDAR_SDF: u32 = 1024;

/// How many entries of the engine's data directory the table-of-contents scan
/// will look at.
///
/// The one place in this module where [`AQSA_MADAKHIL_MUJALLAD`] is not enough,
/// and it was measured rather than guessed at. Snowdrop keeps its single table
/// of contents in the same directory as every content chunk — 2 763 entries in
/// the install this was written against — and the filesystem hands
/// `sdf.sdftoc` back at **position 2 760 of 2 763**. A 256-entry index is a
/// ceiling written for a directory of kinds; against a directory of chunks it
/// silently drops this engine's strongest single observation and leaves the
/// weaker one behind it to answer alone, which is exactly what happened the
/// first time this detector was run against the real game.
///
/// So the table of contents is looked for by streaming the directory instead,
/// with a ceiling that covers a whole one, stopping the moment the file is
/// found. It costs one listing, it happens only for a game whose chunks have
/// already been seen, and it buys the difference between naming this engine
/// from one tag and naming it from a tag plus the studio's own name inside the
/// same header.
const AQSA_MADAKHIL_SDF: usize = 8192;

// ---------------------------------------------------------------------------
// Weights.
//
// The scale is the one documented on `HasilatFahs::sajjil` and on
// `crate::tahdid`: a file only one engine ever ships is 90 and up, a file
// several engines share is 40 to 60, a name that merely suggests something is
// below 30. Named here rather than written inline so the reasoning is
// reviewable in one place.
// ---------------------------------------------------------------------------

/// A shipped module whose version resource declares `ProductName` as
/// `Frostbite`.
///
/// The strongest observation in this module and the cheapest. The engine names
/// itself in a structure Microsoft's resource compiler wrote and EA filled in,
/// and it names its release in the field beside it. 95 rather than 100 because a
/// detector is not permitted to conclude, and because a DLL can be copied
/// somewhere by something other than the game that shipped it.
const WAZN_ISM_FROSTBITE: u8 = 95;

/// An `IGA\x1A` archive or an `IGZ` object file whose version fits.
///
/// A four-byte tag ending in a control character — which is what stops it being
/// a word — plus a version field in range. The pair together is conclusive and
/// [`crate::tahdid`] is where the two agreeing is worth more than either.
const WAZN_SIHR_IG: u8 = 93;

/// A `DCX\0` wrapper whose own header resolves to `DCS\0` and `DCP\0`.
///
/// Structure rather than a constant: two offsets read out of the header, both
/// landing on the tag they are supposed to land on, and a four-character
/// compression name behind the second. A file does not do that by accident.
const WAZN_TARWISAT_DCX: u8 = 93;

/// An `RPF7` archive whose encryption tag is one of the two the shipped set
/// carries.
const WAZN_SIHR_RPF: u8 = 92;

/// `bin64/pers.exe` declaring `ProductName` as `BlackSpace version`.
///
/// The engine naming itself, in a binary shipped beside the game rather than in
/// the game's own — which is exactly one step weaker than
/// [`WAZN_ISM_FROSTBITE`], because the module carrying the name is not the
/// module doing the rendering.
const WAZN_ISM_BLACKSPACE: u8 = 92;

/// A `sdf.sdftoc` opening `WEST`, with `massive` inside its header.
///
/// Two independent things in one file: an arbitrary four-character tag at offset
/// zero, and the studio that wrote the engine spelled out behind it.
const WAZN_TARWISAT_SDFTOC: u8 = 92;

/// A `.sdfdata` chunk opening `BERG` at the same format version as the table of
/// contents that indexes it.
const WAZN_SIHR_SDFDATA: u8 = 90;

/// `Dantelion2` in the executable's read-only data.
///
/// The engine naming itself, put there by the compiler rather than by a
/// packager: these are assertion strings carrying the engine's own source path.
/// Level with [`crate::dalail::bio4`]'s internal-name signature and for the same
/// reason — conclusive about which codebase this is and silent about everything
/// else.
const WAZN_ALAMAT_DANTELION: u8 = 90;

/// A `.pamt` metadata table carrying the 32-bit constant at offset 8.
const WAZN_SIHR_PAMT: u8 = 88;

/// A `.toc` opening with `00 D1 CE` and four zero bytes.
///
/// Below the version resource above it because three bytes plus four zeros is
/// less than a field that spells the engine's name, and above everything below
/// it because those seven bytes are the same in all 87 files here.
const WAZN_SIHR_TOC: u8 = 88;

/// `[RAGE]` in the executable's read-only data.
///
/// The engine naming itself, in a log format string rather than in a structured
/// field. That is a real difference: a version resource is written by a tool
/// that knows what the fields mean, and a bracketed tag is written by whoever
/// typed the format string. Held below the archive magic for it.
const WAZN_ALAMAT_RAGE: u8 = 85;

/// A `BND3` or `BDF3` archive with eight characters of version text behind the
/// tag.
const WAZN_SIHR_BND: u8 = 80;

/// `title.rgl` opening `RGLM`.
const WAZN_SIHR_RGL: u8 = 80;

/// A `.paz` that really is a Pearl Abyss archive.
///
/// Three letters and a space. `PAR` is an abbreviation several unrelated formats
/// reach for, so this sits at the bottom of the band a container tag earns, and
/// it is only recorded for a file whose extension already said `.paz`.
const WAZN_SIHR_PAZ: u8 = 75;

/// A game holding a `.bhd`/`.bdt` archive pair beside `regulation.bin`.
const WAZN_BINYA_DANTELION_ARSHIF: u8 = 76;

/// The directory set this engine's data layout always has.
const WAZN_BINYA_DANTELION: u8 = 74;

/// The engine's data directory, holding the interior shape it always holds.
///
/// `Data/` with `layout.toc` in it and a platform directory of tables under it.
/// Strong shape evidence and only shape evidence: no byte of any table has been
/// read at this point.
const WAZN_BINYA_FROSTBITE: u8 = 72;

/// Numbered content directories beside `bin64/` and `meta/`.
const WAZN_BINYA_BLACKSPACE: u8 = 72;

/// `rpf.cache` opening `HSHR`.
const WAZN_SIHR_CACHE_RPF: u8 = 70;

/// The version resource's company field naming the studio that wrote the engine.
///
/// Corroboration only, and never allowed to name the family on its own. Every
/// title a studio publishes carries the same company in the same field,
/// including the ones built on somebody else's engine, so this says who made the
/// game and not what it was made with.
const WAZN_SHARIKA: u8 = 65;

/// `meta/0.paver`, present and exactly ten bytes long.
const WAZN_PAVER: u8 = 65;

/// The archives directory holding nothing but this engine's archives.
const WAZN_MUJALLAD_ARSHIF: u8 = 60;

/// The engine's bootstrap file, with the two things only it carries inside it.
const WAZN_IQLA_ALCHEMY: u8 = 50;

/// Where the text is, recorded for the reader rather than for the arithmetic.
const WAZN_MAWDI_NUSUS: u8 = 40;

// ---------------------------------------------------------------------------
// The detectors
// ---------------------------------------------------------------------------

/// Every detector in this module, in declaration order.
///
/// One per engine rather than one detector answering six ways, because
/// [`HasilatFahs`] carries a single family and a detector that could name more
/// than one would be a detector deciding between them — which
/// [`crate::tahdid`] is the only place allowed to do.
#[must_use]
pub fn jamee() -> Vec<Box<dyn Fahis>> {
    vec![
        Box::new(FahisFrostbite::jadeed()),
        Box::new(FahisBlackSpace::jadeed()),
        Box::new(FahisAlchemy::jadeed()),
        Box::new(FahisDantelion::jadeed()),
        Box::new(FahisRage::jadeed()),
        Box::new(FahisSnowdrop::jadeed()),
    ]
}

/// Detection of DICE's Frostbite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisFrostbite;

/// Detection of Pearl Abyss's `BlackSpace`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisBlackSpace;

/// Detection of Vicarious Visions' Alchemy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisAlchemy;

/// Detection of `FromSoftware`'s `Dantelion2`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisDantelion;

/// Detection of Rockstar's RAGE.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisRage;

/// Detection of Massive Entertainment's Snowdrop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisSnowdrop;

impl FahisFrostbite {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl FahisBlackSpace {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl FahisAlchemy {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl FahisDantelion {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl FahisRage {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl FahisSnowdrop {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Directories
// ---------------------------------------------------------------------------

/// One directory's entries, indexed for case-insensitive lookup.
///
/// The same shape [`crate::dalail::bio4`] and [`crate::dalail::unity`] keep, and
/// for the same reason: these names come off a Windows filesystem and are asked
/// for by a module that spells them however it likes, so `Data`, `data` and
/// `DATA` have to be one directory rather than three.
#[derive(Debug, Default)]
struct Fahras {
    /// `(lowered name, name as the filesystem spells it, is a directory)`.
    madakhil: Vec<(String, String, bool)>,
    /// The path this index was taken of, so a lookup can build a full path.
    masar: PathBuf,
    /// That path relative to the game root, as evidence records it.
    nisbi: String,
}

impl Fahras {
    /// The real spelling of a directory whose lowered name is `ism`.
    fn mujallad(&self, ism: &str) -> Option<&str> {
        self.madakhil
            .iter()
            .find(|(khafid, _, mujallad)| *mujallad && khafid == ism)
            .map(|(_, haqiqi, _)| haqiqi.as_str())
    }

    /// The real spelling of a file whose lowered name is `ism`.
    fn malaf(&self, ism: &str) -> Option<&str> {
        self.madakhil
            .iter()
            .find(|(khafid, _, mujallad)| !*mujallad && khafid == ism)
            .map(|(_, haqiqi, _)| haqiqi.as_str())
    }

    /// Every file here whose extension is `imtidadat`, at most `hadd` of them.
    fn malaffat(&self, imtidadat: &[&str], hadd: usize) -> Vec<Murashah> {
        let mut kharij: Vec<Murashah> = Vec::new();
        for (_, haqiqi, mujallad) in &self.madakhil {
            if *mujallad || kharij.len() >= hadd {
                continue;
            }
            if imtidadat.iter().any(|matlub| imtidad(haqiqi, matlub)) {
                kharij.push(self.wasal(haqiqi));
            }
        }
        kharij
    }

    /// Every subdirectory here, at most `hadd` of them, in listing order.
    fn mujalladat(&self, hadd: usize) -> Vec<String> {
        self.madakhil
            .iter()
            .filter(|(_, _, mujallad)| *mujallad)
            .take(hadd)
            .map(|(_, haqiqi, _)| haqiqi.clone())
            .collect()
    }

    /// How many subdirectories here have names of exactly `tul` ASCII digits.
    ///
    /// Pearl Abyss numbers its content directories `0000` upward and leaves gaps
    /// in the sequence, so they are counted rather than enumerated: the count is
    /// what a reader can check against the install and the names carry nothing
    /// more.
    fn marqumat(&self, tul: usize) -> usize {
        self.madakhil
            .iter()
            .filter(|(khafid, _, mujallad)| {
                *mujallad && khafid.len() == tul && khafid.bytes().all(|harf| harf.is_ascii_digit())
            })
            .count()
    }

    /// Whether any file here has one of these extensions.
    fn fihi_imtidad(&self, imtidadat: &[&str]) -> bool {
        self.madakhil.iter().any(|(_, haqiqi, mujallad)| {
            !*mujallad && imtidadat.iter().any(|matlub| imtidad(haqiqi, matlub))
        })
    }

    /// One entry of this directory, as a candidate the caller can open.
    fn wasal(&self, ism: &str) -> Murashah {
        let nisbi = if self.nisbi.is_empty() {
            ism.to_owned()
        } else {
            format!("{}/{ism}", self.nisbi)
        };
        Murashah {
            nisbi,
            masar: self.masar.join(ism),
        }
    }
}

/// One file a scan kept, and where it is.
#[derive(Debug, Clone)]
struct Murashah {
    /// Path relative to the game root, as evidence records it.
    nisbi: String,
    /// Absolute path, for the header read.
    masar: PathBuf,
}

/// Lists one directory under the game root, bounded, returning an empty index
/// when it will not list or when the path escapes the root.
///
/// An unreadable directory is the absence of evidence rather than a failure:
/// [`Fahis::ifhas`] is only allowed to fail for a root that is not there. Every
/// path is built through [`SiyaqFahs::dakhil`], so a game directory holding a
/// symbolic link to somewhere else cannot make a probe read outside the game.
fn fahras(siyaq: &SiyaqFahs<'_>, nisbi: &str) -> Fahras {
    let masar = if nisbi.is_empty() {
        siyaq.jidhr.to_path_buf()
    } else {
        match siyaq.dakhil(nisbi) {
            Ok(masar) => masar,
            Err(_) => return Fahras::default(),
        }
    };
    let mut fahras = Fahras {
        madakhil: Vec::new(),
        masar: masar.clone(),
        nisbi: nisbi.to_owned(),
    };
    let Ok(madakhil) = fs::read_dir(&masar) else {
        return fahras;
    };
    for (adad, madkhal) in madakhil.enumerate() {
        if adad >= AQSA_MADAKHIL_MUJALLAD {
            break;
        }
        let Ok(madkhal) = madkhal else { continue };
        let khaam = madkhal.file_name();
        let Some(ism) = khaam.to_str() else { continue };
        let mujallad = huwa_mujallad(&madkhal);
        fahras
            .madakhil
            .push((ism.to_ascii_lowercase(), ism.to_owned(), mujallad));
    }
    fahras
}

/// The index of a directory named `ism` inside `walid`, when there is one.
fn fahras_faree(siyaq: &SiyaqFahs<'_>, walid: &Fahras, ism: &str) -> Option<Fahras> {
    let haqiqi = walid.mujallad(ism)?;
    let nisbi = if walid.nisbi.is_empty() {
        haqiqi.to_owned()
    } else {
        format!("{}/{haqiqi}", walid.nisbi)
    };
    Some(fahras(siyaq, &nisbi))
}

/// Whether a directory entry is a directory, following a symbolic link to
/// whatever it points at.
fn huwa_mujallad(madkhal: &fs::DirEntry) -> bool {
    match madkhal.file_type() {
        Ok(naw) if naw.is_symlink() => fs::metadata(madkhal.path()).is_ok_and(|wasf| wasf.is_dir()),
        Ok(naw) => naw.is_dir(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Bounded reads and fields
// ---------------------------------------------------------------------------

/// A file's length, when it can be had.
fn tul_malaf(masar: &Path) -> Option<u64> {
    Some(fs::metadata(masar).ok()?.len())
}

/// A big-endian `u32` at a checked offset.
fn raqm32_kabir(qita: &[u8], izaha: usize) -> Option<u32> {
    let bayt = qita.get(izaha..izaha.checked_add(4)?)?;
    <[u8; 4]>::try_from(bayt).ok().map(u32::from_be_bytes)
}

/// Whether the four bytes at `izaha` are exactly `sihr`.
fn sihr_fi(qita: &[u8], izaha: usize, sihr: [u8; 4]) -> bool {
    qita.get(izaha..izaha.saturating_add(4))
        .is_some_and(|nafidha| nafidha == sihr)
}

/// Whether `kawm` holds `ibra` anywhere, and where.
fn mawdi(kawm: &[u8], ibra: &[u8]) -> Option<usize> {
    if ibra.is_empty() || ibra.len() > kawm.len() {
        return None;
    }
    kawm.windows(ibra.len()).position(|nafidha| nafidha == ibra)
}

/// The first four bytes at `izaha` as text, when every one of them is printable
/// ASCII.
///
/// Used for the compression name behind a `DCX\0` header and the version text
/// behind `BND3`, both of which are written as characters rather than as
/// numbers. A run with a byte outside the printable range is not a tag and is
/// refused rather than escaped into the evidence line.
fn nass_ascii(qita: &[u8], izaha: usize, tul: usize) -> Option<String> {
    let bayt = qita.get(izaha..izaha.checked_add(tul)?)?;
    if !bayt.iter().all(u8::is_ascii_graphic) {
        return None;
    }
    Some(String::from_utf8_lossy(bayt).into_owned())
}

// ---------------------------------------------------------------------------
// The sections behind the version resource
// ---------------------------------------------------------------------------

/// Searches a named PE section for a byte marker, bounded, and returns how far
/// into the section it was found.
///
/// Streamed in chunks with an overlap, so a marker lying across a chunk boundary
/// is still found and the whole section is never held in memory. This is the
/// most expensive thing in the module and it is only ever reached for a game
/// whose containers have already named the engine — the two callers gate it that
/// way, and a marker scan run on every game in a library would be exactly the
/// cost [`crate::dalail::bio4`] documents its gate to avoid.
fn alama_fi_qism(masar: &Path, ism_qism: &[u8], alama: &[u8]) -> Option<u64> {
    let qism = tanfidhi::qism(masar, ism_qism)?;
    let hadd = qism.hajm.min(HAJM_MASH_QISM);
    let mut malaf = File::open(masar).ok()?;
    malaf.seek(SeekFrom::Start(qism.mawdi)).ok()?;

    let mut sabiq: Vec<u8> = Vec::new();
    let mut maqru: usize = 0;
    while maqru < hadd {
        let baqi = hadd.saturating_sub(maqru);
        let mut qita: Vec<u8> = Vec::new();
        let hissa = baqi.min(1024 * 1024);
        let _ = (&mut malaf)
            .take(u64::try_from(hissa).ok()?)
            .read_to_end(&mut qita)
            .ok()?;
        if qita.is_empty() {
            return None;
        }
        let bidayat_qita = maqru.saturating_sub(sabiq.len());
        sabiq.extend_from_slice(&qita);
        if let Some(mawqi) = mawdi(&sabiq, alama) {
            return u64::try_from(bidayat_qita.saturating_add(mawqi)).ok();
        }
        maqru = maqru.saturating_add(qita.len());
        let ibqa = sabiq.len().saturating_sub(TADAKHUL_MASH);
        sabiq = sabiq.split_off(ibqa);
    }
    None
}

/// A dotted version string, parsed into three numbers without inventing any.
///
/// **Every** component has to be a number, and there have to be between one and
/// four of them. That is stricter than it needs to be for the one field this is
/// called on, and the strictness is the point: `1.0.0RELEASE_DEV.0` parses
/// perfectly well as `1.0.0` under a lenient reader, and a build tag silently
/// becoming a release number is the exact failure
/// [`crate::dalail::bio4`] refuses to make. Missing components are zero, which
/// is what `2.42` means.
const AQSA_AJZA_ISDAR: usize = 4;

/// See [`AQSA_AJZA_ISDAR`].
fn hallil_isdar(khaam: &str) -> Option<IsdarMuharrik> {
    let mut arqam: [u16; 3] = [0, 0, 0];
    let mut adad = 0_usize;
    for juz in khaam.split('.') {
        let raqm = juz.parse::<u16>().ok()?;
        if adad >= AQSA_AJZA_ISDAR {
            return None;
        }
        if let Some(khana) = arqam.get_mut(adad) {
            *khana = raqm;
        }
        adad = adad.saturating_add(1);
    }
    if adad == 0 {
        return None;
    }
    let (kabir, sagheer, tasheeh) = (
        arqam.first().copied().unwrap_or(0),
        arqam.get(1).copied().unwrap_or(0),
        arqam.get(2).copied().unwrap_or(0),
    );
    Some(IsdarMuharrik {
        kabir,
        sagheer,
        tasheeh,
        khaam: khaam.to_owned(),
        mushtaqq: false,
    })
}

/// Records a company or copyright field that names the studio behind an engine.
///
/// Corroboration and never an identification. Factored out because five of the
/// six detectors make the same observation about a different studio and the
/// sentence has to say the same thing every time: this names who made the game,
/// not what it was made with.
fn sharikat_mutabiqa(bayan: &BayanTanfidhi, nisbi: &str, ibra: &str, hasila: &mut HasilatFahs) {
    // Both fields, not the first that has a value. DARK SOULS: REMASTERED is
    // why: its `CompanyName` is the publisher, `NAMCO BANDAI Games Inc.`, and
    // the studio that wrote the engine appears only in `LegalCopyright`. A
    // reader that stopped at the first populated field would find the wrong
    // company and record nothing.
    let maydan = [bayan.sharika.as_deref(), bayan.huquq.as_deref()]
        .into_iter()
        .flatten()
        .find(|qeema| qeema.to_ascii_uppercase().contains(ibra));
    let Some(qeema) = maydan else { return };
    hasila.sajjil(
        NawDaleel::BayanatMudmaja,
        format!(
            "the binary's version resource names `{qeema}`, which is the studio that \
             published this game and not, on its own, the engine it was built with"
        ),
        Some(nisbi.to_owned()),
        WAZN_SHARIKA,
    );
}

/// Records a version string and refuses to read an engine version out of it.
///
/// Every one of the five engines below has a binary carrying three or four
/// numbers that parse perfectly well and mean the game's build, the patcher's
/// build or the container's format. Recording them at weight zero keeps them in
/// the evidence trail, where a maintainer can see what was rejected and why.
fn isdar_ghayr_muharrik(khaam: &str, nisbi: &str, sabab: &str, hasila: &mut HasilatFahs) {
    hasila.sajjil(
        NawDaleel::BayanatMudmaja,
        format!(
            "the version string `{khaam}` was read here and is deliberately not reported as \
             this engine's version: {sabab}. Three numbers presented as an engine release \
             where there is none would be a confident wrong answer."
        ),
        Some(nisbi.to_owned()),
        0,
    );
}

/// The largest `.exe` in a directory, which is the game's own binary in every
/// layout this module meets.
///
/// A heuristic, and named as one. It is right here because the three games whose
/// executables this module reads ship theirs beside a small launcher or a small
/// protector stub — 87 MB against 4 MB, 56 MB against 1.4 MB, 380 MB against
/// 4 MB — and it is only ever used after the game's containers have already
/// named the engine, so the worst a wrong pick costs is a marker that is not
/// found.
fn akbar_tanfidhi(fahras: &Fahras) -> Option<Murashah> {
    let mut afdal: Option<(u64, Murashah)> = None;
    for murashah in fahras.malaffat(&["exe"], AQSA_MADAKHIL_MUJALLAD) {
        let Some(tul) = tul_malaf(&murashah.masar) else {
            continue;
        };
        if afdal.as_ref().is_none_or(|(akbar, _)| tul > *akbar) {
            afdal = Some((tul, murashah));
        }
    }
    afdal.map(|(_, murashah)| murashah)
}

/// The root's index, or the error [`Fahis::ifhas`] is allowed to return.
///
/// # Errors
///
/// Only when the game's root directory is not there.
fn jidhr(siyaq: &SiyaqFahs<'_>) -> Natija<Fahras> {
    if siyaq.jidhr.exists() {
        Ok(fahras(siyaq, ""))
    } else {
        Err(KhataMuharrik::JidhrMafqud {
            jidhr: siyaq.jidhr.to_path_buf(),
        }
        .into())
    }
}

// ---------------------------------------------------------------------------
// Frostbite
// ---------------------------------------------------------------------------

impl Fahis for FahisFrostbite {
    fn ism(&self) -> &'static str {
        "frostbite"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there. A root that will not
    /// list, a DLL that will not open and a header that will not parse are all
    /// the absence of evidence, which is [`HasilatFahs::la_shay`].
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let jidhr = jidhr(siyaq)?;
        let mut hasila = HasilatFahs::la_shay();

        // The gate: one listing of the game root. This engine keeps its whole
        // content set under a single directory and ships its own modules beside
        // the executable, so a root with neither is not a Frostbite game and
        // nothing else in this detector runs.
        let bayanat = fahras_faree(siyaq, &jidhr, MUJALLAD_BAYANAT_FROSTBITE);
        let mukawwinat = mukawwinat_frostbite(&jidhr);
        if bayanat.is_none() && mukawwinat.is_empty() {
            return Ok(hasila);
        }

        ism_frostbite(&mukawwinat, &mut hasila);
        for far in [MUJALLAD_BAYANAT_FROSTBITE, MUJALLAD_TARQEE_FROSTBITE] {
            let Some(fahras) = fahras_faree(siyaq, &jidhr, far) else {
                continue;
            };
            tawqi_toc(siyaq, &fahras, &mut hasila);
        }
        if let Some(bayanat) = bayanat.as_ref() {
            binyat_frostbite(bayanat, &mut hasila);
        }
        Ok(hasila)
    }
}

/// The engine's own shipped modules at the game root, at most four.
fn mukawwinat_frostbite(jidhr: &Fahras) -> Vec<Murashah> {
    let mut kharij: Vec<Murashah> = Vec::new();
    for (khafid, haqiqi, mujallad) in &jidhr.madakhil {
        if *mujallad || kharij.len() >= AQSA_MULAFFAT_ISDAR {
            continue;
        }
        if khafid.starts_with(SABIQAT_MUKAWWIN_FROSTBITE) && imtidad(haqiqi, "dll") {
            kharij.push(jidhr.wasal(haqiqi));
        }
    }
    kharij
}

/// Reads the version resource of each shipped module until one names the engine.
///
/// Several rather than the first, because the sibling
/// `Engine.Render.Core2.PlatformVulkan.dll` in the install this was written
/// against carries no version resource at all — a detector that stopped at the
/// first module it found would answer on which one the filesystem listed first.
fn ism_frostbite(mukawwinat: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in mukawwinat {
        let Some(bayan) = bayan_tanfidhi(&murashah.masar) else {
            continue;
        };
        let Some(ism) = bayan.ism_muntaj.as_deref() else {
            continue;
        };
        if !ism.eq_ignore_ascii_case(ISM_FROSTBITE) {
            continue;
        }
        if let Some(mimariya) = bayan.mimariya {
            hasila.mimariya = Some(mimariya);
        }
        hasila.sajjil_aila(
            AilatMuharrik::Frostbite,
            NawDaleel::BayanatMudmaja,
            format!(
                "a module shipped beside the game declares `ProductName` as `{ism}` in its \
                 Windows version resource — the engine naming itself in a structure a resource \
                 compiler wrote and the publisher filled in"
            ),
            Some(murashah.nisbi.clone()),
            WAZN_ISM_FROSTBITE,
        );
        sharikat_mutabiqa(&bayan, &murashah.nisbi, "ELECTRONIC ARTS", hasila);

        // The one version in this module that is reported, and the reason is
        // that it is the only one that was read rather than inferred: the field
        // beside the engine's own name, on the engine's own module.
        if let Some(khaam) = bayan.isdar_muntaj.as_deref()
            && let Some(isdar) = hallil_isdar(khaam)
        {
            hasila.sajjil(
                NawDaleel::BayanatMudmaja,
                format!(
                    "the same resource gives `ProductVersion` as `{khaam}`, which is this \
                     engine's own release number and is reported as read rather than derived"
                ),
                Some(murashah.nisbi.clone()),
                WAZN_ISM_FROSTBITE,
            );
            hasila.isdar = Some(isdar);
        }
        return;
    }
}

/// Reads the tables of contents under a data or patch directory.
fn tawqi_toc(siyaq: &SiyaqFahs<'_>, bayanat: &Fahras, hasila: &mut HasilatFahs) {
    if hasila
        .dalail
        .iter()
        .any(|daleel| daleel.wazn == WAZN_SIHR_TOC)
    {
        return;
    }
    let mut murashahun = bayanat.malaffat(&["toc"], HISSAT_SINF);
    for far in bayanat.mujalladat(AQSA_FURU) {
        if murashahun.len() >= HISSAT_SINF {
            break;
        }
        let nisbi = format!("{}/{far}", bayanat.nisbi);
        murashahun.extend(fahras(siyaq, &nisbi).malaffat(&["toc"], HISSAT_SINF));
    }

    for murashah in murashahun.iter().take(HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(sinf) = tarwisat_toc(&nafidha) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Frostbite,
            NawDaleel::TarwisatHawiya,
            format!(
                "an archive index opening `00 D1 CE {sinf:02X}` with four zero bytes behind \
                 it — this engine's own table-of-contents header, whose three constant bytes \
                 spell the studio that wrote it. The fourth byte is recorded and nothing is \
                 claimed about what it selects."
            ),
            Some(murashah.nisbi.clone()),
            WAZN_SIHR_TOC,
        );
        return;
    }
}

/// Whether a table of contents is one, and which variant byte it carries.
fn tarwisat_toc(nafidha: &[u8]) -> Option<u8> {
    if !nafidha.starts_with(&SIHR_TOC) {
        return None;
    }
    if raqm32_sagheer(nafidha, 4)? != 0 {
        return None;
    }
    nafidha.get(3).copied()
}

/// Records the engine's data directory and the shape inside it.
fn binyat_frostbite(bayanat: &Fahras, hasila: &mut HasilatFahs) {
    let tarteeb = bayanat.malaf(MALAF_TARTEEB).is_some();
    let furu = bayanat.mujalladat(AQSA_FURU);
    if !tarteeb || furu.is_empty() {
        return;
    }
    hasila.sajjil_aila(
        AilatMuharrik::Frostbite,
        NawDaleel::BinyatMujallad,
        format!(
            "`{}` in the shape this engine reads it: `{MALAF_TARTEEB}` at the top and {} \
             platform director{} of archive indexes under it",
            bayanat.nisbi,
            furu.len(),
            if furu.len() == 1 { "y" } else { "ies" }
        ),
        Some(bayanat.nisbi.clone()),
        WAZN_BINYA_FROSTBITE,
    );
}

// ---------------------------------------------------------------------------
// Pearl Abyss BlackSpace
// ---------------------------------------------------------------------------

impl Fahis for FahisBlackSpace {
    fn ism(&self) -> &'static str {
        "blackspace"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let jidhr = jidhr(siyaq)?;
        let mut hasila = HasilatFahs::la_shay();

        // The gate: this engine lays its content out as numbered directories at
        // the game root with a metadata directory beside them. Neither on its
        // own opens the gate, because `meta` is a name anything might use and a
        // numbered directory is a shape anything might have.
        let marqumat = jidhr.marqumat(4);
        let meta = fahras_faree(siyaq, &jidhr, MUJALLAD_META_BLACKSPACE);
        if marqumat == 0 || meta.is_none() {
            return Ok(hasila);
        }

        ism_blackspace(siyaq, &jidhr, &mut hasila);
        hawiyat_blackspace(siyaq, &jidhr, &mut hasila);
        if let Some(meta) = meta.as_ref() {
            paver(meta, &mut hasila);
        }
        binyat_blackspace(&jidhr, marqumat, &mut hasila);
        Ok(hasila)
    }
}

/// Reads the executables beside the game until one names the engine.
fn ism_blackspace(siyaq: &SiyaqFahs<'_>, jidhr: &Fahras, hasila: &mut HasilatFahs) {
    let Some(bin) = fahras_faree(siyaq, jidhr, MUJALLAD_TANFIDH_BLACKSPACE) else {
        return;
    };
    for murashah in bin.malaffat(&["exe"], AQSA_MULAFFAT_ISDAR) {
        let Some(bayan) = bayan_tanfidhi(&murashah.masar) else {
            continue;
        };
        let Some(ism) = bayan.ism_muntaj.as_deref() else {
            continue;
        };
        if !ism
            .to_ascii_uppercase()
            .contains(&ISM_BLACKSPACE.to_ascii_uppercase())
        {
            continue;
        }
        if let Some(mimariya) = bayan.mimariya {
            hasila.mimariya = Some(mimariya);
        }
        hasila.sajjil_aila(
            AilatMuharrik::BlackSpace,
            NawDaleel::BayanatMudmaja,
            format!(
                "a binary shipped beside the game declares `ProductName` as `{ism}` — the \
                 engine naming itself, in the version resource of the tool that patches the \
                 game rather than in the game's own"
            ),
            Some(murashah.nisbi.clone()),
            WAZN_ISM_BLACKSPACE,
        );
        sharikat_mutabiqa(&bayan, &murashah.nisbi, SHARIKAT_BLACKSPACE, hasila);
        if let Some(khaam) = bayan.isdar_malaf.as_deref() {
            isdar_ghayr_muharrik(
                khaam,
                &murashah.nisbi,
                "it is the file version of the binary that happens to carry the engine's name \
                 in a neighbouring field, and this studio publishes no release numbering for \
                 the engine itself",
                hasila,
            );
        }
        return;
    }
}

/// Opens the content containers in the first numbered directories.
fn hawiyat_blackspace(siyaq: &SiyaqFahs<'_>, jidhr: &Fahras, hasila: &mut HasilatFahs) {
    let mut pamt: Vec<Murashah> = Vec::new();
    let mut paz: Vec<Murashah> = Vec::new();
    for far in jidhr.mujalladat(AQSA_MADAKHIL_MUJALLAD) {
        if pamt.len() >= HISSAT_SINF && paz.len() >= HISSAT_SINF {
            break;
        }
        if far.len() != 4 || !far.bytes().all(|harf| harf.is_ascii_digit()) {
            continue;
        }
        let fahras = fahras(siyaq, &far);
        pamt.extend(fahras.malaffat(&["pamt"], HISSAT_SINF));
        paz.extend(fahras.malaffat(&["paz"], HISSAT_SINF));
    }

    for murashah in pamt.iter().take(HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(adad) = tarwisat_pamt(&nafidha) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::BlackSpace,
            NawDaleel::TarwisatHawiya,
            format!(
                "a metadata table carrying the constant {SIHR_PAMT:#010X} at offset eight with \
                 four zero bytes behind it, declaring {adad} record{}. The four bytes in front \
                 of the constant differ in every file and are not read.",
                if adad == 1 { "" } else { "s" }
            ),
            Some(murashah.nisbi.clone()),
            WAZN_SIHR_PAMT,
        );
        break;
    }

    for murashah in paz.iter().take(HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        if !sihr_fi(&nafidha, 0, SIHR_PAZ) {
            continue;
        }
        hasila.sajjil_aila(
            AilatMuharrik::BlackSpace,
            NawDaleel::TarwisatHawiya,
            "a content container opening `PAR ` — this studio's archive. Only a minority of \
             the files under this extension are archives at all; the rest are raw payloads \
             written straight into it, which is why the header decides and the extension does \
             not."
                .to_owned(),
            Some(murashah.nisbi.clone()),
            WAZN_SIHR_PAZ,
        );
        break;
    }
}

/// Whether a metadata table's header is one, and how many records it claims.
fn tarwisat_pamt(nafidha: &[u8]) -> Option<u32> {
    if raqm32_sagheer(nafidha, 8)? != SIHR_PAMT {
        return None;
    }
    if raqm32_sagheer(nafidha, 12)? != 0 {
        return None;
    }
    let adad = raqm32_sagheer(nafidha, 4)?;
    (adad > 0 && adad <= AQSA_QUYUD_PAMT).then_some(adad)
}

/// Records the engine's version stamp, which is a length rather than a header.
fn paver(meta: &Fahras, hasila: &mut HasilatFahs) {
    for murashah in meta.malaffat(&["paver"], HISSAT_SINF) {
        if tul_malaf(&murashah.masar) != Some(TUL_PAVER) {
            continue;
        }
        hasila.sajjil_aila(
            AilatMuharrik::BlackSpace,
            NawDaleel::BinyatMujallad,
            format!(
                "the engine's version stamp beside its metadata, {TUL_PAVER} bytes exactly. \
                 What the ten bytes mean is not understood and they are not read: the file's \
                 name, its place and its length are the observation."
            ),
            Some(murashah.nisbi),
            WAZN_PAVER,
        );
        return;
    }
}

/// Records the layout: numbered content directories, a binary directory and a
/// metadata directory.
fn binyat_blackspace(jidhr: &Fahras, marqumat: usize, hasila: &mut HasilatFahs) {
    if jidhr.mujallad(MUJALLAD_TANFIDH_BLACKSPACE).is_none() {
        return;
    }
    hasila.sajjil_aila(
        AilatMuharrik::BlackSpace,
        NawDaleel::BinyatMujallad,
        format!(
            "{marqumat} four-digit content directories at the game's root beside \
             `{MUJALLAD_TANFIDH_BLACKSPACE}/` and `{MUJALLAD_META_BLACKSPACE}/` — this \
             engine's layout, in which the numbers are content sets and the sequence has gaps"
        ),
        None,
        WAZN_BINYA_BLACKSPACE,
    );
}

// ---------------------------------------------------------------------------
// Vicarious Visions Alchemy
// ---------------------------------------------------------------------------

impl Fahis for FahisAlchemy {
    fn ism(&self) -> &'static str {
        "alchemy"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let jidhr = jidhr(siyaq)?;
        let mut hasila = HasilatFahs::la_shay();

        // The gate: an object file at the root, or an archives directory. Both
        // are cheap and either is enough, because this engine ships its startup
        // object beside the executable and everything else under one directory.
        let arshif = fahras_faree(siyaq, &jidhr, MUJALLAD_ARSHIF_ALCHEMY);
        if !jidhr.fihi_imtidad(&["igz"]) && arshif.is_none() {
            return Ok(hasila);
        }

        igz(&jidhr, &mut hasila);
        if let Some(arshif) = arshif.as_ref() {
            iga(arshif, &mut hasila);
        }
        iqla_alchemy(&jidhr, &mut hasila);
        Ok(hasila)
    }
}

/// The object file the engine reads at startup.
fn igz(jidhr: &Fahras, hasila: &mut HasilatFahs) {
    for murashah in jidhr.malaffat(&["igz"], HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(isdar) = sihr_wa_isdar(&nafidha, SIHR_IGZ) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Alchemy,
            NawDaleel::TarwisatHawiya,
            format!(
                "an object file opening `01 5A 47 49` — this engine's `IGZ` tag with its bytes \
                 swapped, exactly as the shipped files store it — declaring format version \
                 {isdar}"
            ),
            Some(murashah.nisbi),
            WAZN_SIHR_IG,
        );
        return;
    }
}

/// The archives, and what they are not.
///
/// The one observation here that exists to refute something as well as to
/// establish it. `.pak` is Unreal's archive extension too, and a reader who
/// meets a hundred and seventy of them will reach for that conclusion; the tag
/// says otherwise, and the evidence trail should say so without anybody having
/// to open a hex editor.
fn iga(arshif: &Fahras, hasila: &mut HasilatFahs) {
    let mut wujida = false;
    for murashah in arshif.malaffat(&["pak"], HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(isdar) = sihr_wa_isdar(&nafidha, SIHR_IGA) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Alchemy,
            NawDaleel::TarwisatHawiya,
            format!(
                "a file named `.pak` that opens with `IGA\\x1A` at format version {isdar} — \
                 this engine's archive, not an Unreal pak. The extension is shared and the \
                 format is not, which is the single thing most likely to be mistaken about \
                 this engine."
            ),
            Some(murashah.nisbi),
            WAZN_SIHR_IG,
        );
        wujida = true;
        break;
    }
    if !wujida {
        return;
    }
    hasila.sajjil(
        NawDaleel::BinyatMujallad,
        format!(
            "`{}`, where this engine keeps every archive the game loads",
            arshif.nisbi
        ),
        Some(arshif.nisbi.clone()),
        WAZN_MUJALLAD_ARSHIF,
    );
}

/// A four-byte tag with a plausible format version behind it.
fn sihr_wa_isdar(nafidha: &[u8], sihr: [u8; 4]) -> Option<u32> {
    if !sihr_fi(nafidha, 0, sihr) {
        return None;
    }
    let isdar = raqm32_sagheer(nafidha, 4)?;
    (isdar > 0 && isdar <= AQSA_ISDAR_IG).then_some(isdar)
}

/// The engine's bootstrap file, read as text and only as far as its two markers.
fn iqla_alchemy(jidhr: &Fahras, hasila: &mut HasilatFahs) {
    let Some(haqiqi) = jidhr.malaf(MALAF_IQLA_ALCHEMY) else {
        return;
    };
    let murashah = jidhr.wasal(haqiqi);
    let Some(bayt) = iqra_nafidha(&murashah.masar, 0, HAJM_IQLA_ALCHEMY) else {
        return;
    };
    if !IBARAT_IQLA_ALCHEMY
        .iter()
        .all(|ibra| mawdi(&bayt, ibra.as_bytes()).is_some())
    {
        return;
    }
    hasila.sajjil(
        NawDaleel::BinyatMujallad,
        format!(
            "`{haqiqi}`, the file this engine reads at startup, carrying both `{}` and `{}` \
             inside it. A file of this name is generic; the two fields inside it are not.",
            IBARAT_IQLA_ALCHEMY.first().copied().unwrap_or_default(),
            IBARAT_IQLA_ALCHEMY.get(1).copied().unwrap_or_default()
        ),
        Some(murashah.nisbi),
        WAZN_IQLA_ALCHEMY,
    );
}

// ---------------------------------------------------------------------------
// FromSoftware Dantelion
// ---------------------------------------------------------------------------

impl Fahis for FahisDantelion {
    fn ism(&self) -> &'static str {
        "dantelion"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let jidhr = jidhr(siyaq)?;
        let mut hasila = HasilatFahs::la_shay();

        // Two layouts, one detector. The older generation puts the whole data
        // tree at the game's root and the newer one puts it a directory down,
        // so the root listing decides which of the two is being looked at
        // before anything else is read.
        let qaida = match fahras_faree(siyaq, &jidhr, MUJALLAD_LUBA_DANTELION) {
            Some(faree) if bawwabat_dantelion(&faree) => faree,
            _ if bawwabat_dantelion(&jidhr) => jidhr,
            _ => return Ok(hasila),
        };

        hawiyat_dantelion(siyaq, &qaida, &mut hasila);
        binyat_dantelion(&qaida, &mut hasila);
        tanfidhi_dantelion(&qaida, &mut hasila);
        Ok(hasila)
    }
}

/// Whether a directory is the base of this engine's data tree.
///
/// Either an archive sitting in it, or the two directories the layout is
/// recognised by. Both halves are needed because the two generations differ:
/// the newer ships four archive pairs and no data tree at all, and the older
/// ships a data tree and keeps every archive one level inside it.
fn bawwabat_dantelion(qaida: &Fahras) -> bool {
    if qaida.fihi_imtidad(&["bdt", "bhd", "dcx"]) {
        return true;
    }
    MUJALLADAT_DANTELION
        .iter()
        .take(2)
        .all(|ism| qaida.mujallad(ism).is_some())
}

/// Opens the compression wrappers and archives this engine frames its data in.
fn hawiyat_dantelion(siyaq: &SiyaqFahs<'_>, qaida: &Fahras, hasila: &mut HasilatFahs) {
    let mut murashahun = qaida.malaffat(&["dcx", "bdt"], HISSAT_SINF);
    let mut arshifat = qaida.malaffat(&["fgbnd", "tpfbdt", "bnd"], HISSAT_SINF);
    let mut nusus: Option<String> = None;

    for ism in MUJALLADAT_MASH_DANTELION {
        if murashahun.len() >= HISSAT_SINF && arshifat.len() >= HISSAT_SINF && nusus.is_some() {
            break;
        }
        let Some(faree) = fahras_faree(siyaq, qaida, ism) else {
            continue;
        };
        murashahun.extend(faree.malaffat(&["dcx"], HISSAT_SINF));
        arshifat.extend(faree.malaffat(&["fgbnd", "tpfbdt", "bnd"], HISSAT_SINF));
        // One more level, and only under the directory the strings live in:
        // this engine keeps them one language directory deep and nothing else
        // this detector wants is further down than that.
        if *ism != "msg" {
            continue;
        }
        for lugha in faree.mujalladat(AQSA_FURU) {
            let nisbi = format!("{}/{lugha}", faree.nisbi);
            let dakhil = fahras(siyaq, &nisbi);
            if dakhil.fihi_imtidad(&["dcx"]) && nusus.is_none() {
                nusus = Some(faree.nisbi.clone());
            }
            murashahun.extend(dakhil.malaffat(&["dcx"], HISSAT_SINF));
        }
    }

    // The text's location is recorded only once a wrapper has actually parsed.
    // On its own it is a directory called `msg` with files whose extension is
    // `.dcx`, and an extension is not evidence — a game shipping either by
    // coincidence would otherwise buy an observation out of this detector.
    let wujida = dcx(&murashahun, hasila);
    bnd(&arshifat, hasila);
    if !wujida {
        return;
    }
    if let Some(nisbi) = nusus {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "`{nisbi}`, where this engine keeps every string the player reads — one \
                 directory per language, each holding compressed archives of message tables"
            ),
            Some(nisbi),
            WAZN_MAWDI_NUSUS,
        );
    }
}

/// The `DCX\0` compression wrapper, checked by resolving its own two offsets.
///
/// Returns whether one of the candidates really was one, because the location of
/// the game's text is only worth recording once it has been.
fn dcx(murashahun: &[Murashah], hasila: &mut HasilatFahs) -> bool {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(daght) = tarwisat_dcx(&nafidha) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Dantelion,
            NawDaleel::TarwisatHawiya,
            format!(
                "a `DCX\\0` compression wrapper whose own header resolves: the offset at 0x08 \
                 lands on `DCS\\0`, the offset at 0x0C lands on `DCP\\0`, and the four \
                 characters behind the second name the compression as `{daght}`. Two offsets \
                 that both land where the header says is structure rather than coincidence."
            ),
            Some(murashah.nisbi.clone()),
            WAZN_TARWISAT_DCX,
        );
        return true;
    }
    false
}

/// Reads a `DCX\0` header and returns the compression name behind it.
///
/// The two unknown words at 0x10 and 0x14 are not read, and the word at 0x04 is
/// not read either: it is 0x00010000 in one generation and 0x00011000 in the
/// other, so whatever it is, it is not a version this module can compare.
fn tarwisat_dcx(nafidha: &[u8]) -> Option<String> {
    if !sihr_fi(nafidha, 0, SIHR_DCX) {
        return None;
    }
    let dcs = usize::try_from(raqm32_kabir(nafidha, 8)?).ok()?;
    let dcp = usize::try_from(raqm32_kabir(nafidha, 12)?).ok()?;
    if !sihr_fi(nafidha, dcs, SIHR_DCS) || !sihr_fi(nafidha, dcp, SIHR_DCP) {
        return None;
    }
    nass_ascii(nafidha, dcp.checked_add(4)?, 4)
}

/// The `BND3`/`BDF3` archive pair, and the version text behind the tag.
fn bnd(murashahun: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some((sihr, isdar)) = tarwisat_bnd(&nafidha) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Dantelion,
            NawDaleel::TarwisatHawiya,
            format!(
                "an archive opening `{sihr}` with `{isdar}` behind it — this engine writes its \
                 archive version as {TUL_ISDAR_BND} characters of text rather than as a number"
            ),
            Some(murashah.nisbi.clone()),
            WAZN_SIHR_BND,
        );
        return;
    }
}

/// Whether an archive header is one of the pair, and what it says.
fn tarwisat_bnd(nafidha: &[u8]) -> Option<(String, String)> {
    let sihr = if sihr_fi(nafidha, 0, SIHR_BND) {
        SIHR_BND
    } else if sihr_fi(nafidha, 0, SIHR_BDF) {
        SIHR_BDF
    } else {
        return None;
    };
    let isdar = nass_ascii(nafidha, 4, TUL_ISDAR_BND)?;
    Some((String::from_utf8_lossy(&sihr).into_owned(), isdar))
}

/// Records the layout: the archive pairs, the rule stamp, and the data tree.
fn binyat_dantelion(qaida: &Fahras, hasila: &mut HasilatFahs) {
    let azwaj = qaida.fihi_imtidad(&["bhd"]) && qaida.fihi_imtidad(&["bdt"]);
    if azwaj && qaida.malaf(MALAF_TANZEEM_DANTELION).is_some() {
        hasila.sajjil_aila(
            AilatMuharrik::Dantelion,
            NawDaleel::BinyatMujallad,
            format!(
                "`.bhd`/`.bdt` archive pairs beside `{MALAF_TANZEEM_DANTELION}` — the newer \
                 generation of this engine's layout, where the index half of every pair is \
                 encrypted and the stamp beside them carries the rules the game loads at \
                 startup"
            ),
            Some(qaida.nisbi.clone()),
            WAZN_BINYA_DANTELION_ARSHIF,
        );
    }

    let mawjuda: Vec<&str> = MUJALLADAT_DANTELION
        .iter()
        .filter(|ism| qaida.mujallad(ism).is_some())
        .copied()
        .collect();
    if mawjuda.len() < 4 {
        return;
    }
    hasila.sajjil_aila(
        AilatMuharrik::Dantelion,
        NawDaleel::BinyatMujallad,
        format!(
            "this engine's data tree, with {} inside it — the older generation's layout, where \
             every kind of data is a directory the runtime looks up by name",
            mawjuda.join(", ")
        ),
        Some(qaida.nisbi.clone()),
        WAZN_BINYA_DANTELION,
    );
}

/// Reads the engine's own name out of the executable, and the studio's out of
/// its version resource.
fn tanfidhi_dantelion(qaida: &Fahras, hasila: &mut HasilatFahs) {
    let Some(murashah) = akbar_tanfidhi(qaida) else {
        return;
    };
    if let Some(bayan) = bayan_tanfidhi(&murashah.masar) {
        if let Some(mimariya) = bayan.mimariya {
            hasila.mimariya = Some(mimariya);
        }
        sharikat_mutabiqa(&bayan, &murashah.nisbi, SHARIKAT_DANTELION, hasila);
        if let Some(khaam) = bayan.isdar_muntaj.as_deref() {
            isdar_ghayr_muharrik(
                khaam,
                &murashah.nisbi,
                "it is the game's own product version, and this engine carries no version \
                 numbering of its own anywhere in the install",
                hasila,
            );
        }
    }

    let Some(mawdi) = alama_fi_qism(&murashah.masar, b".rdata", ALAMAT_DANTELION) else {
        return;
    };
    hasila.sajjil_aila(
        AilatMuharrik::Dantelion,
        NawDaleel::TawqiThunai,
        format!(
            "`{}` appears {mawdi} bytes into the executable's read-only data — this engine's \
             own library name, written into the image by the compiler as part of the assertion \
             paths of its own source tree. That is the engine naming itself, and it is the only \
             name this engine has: the studio publishes none.",
            String::from_utf8_lossy(ALAMAT_DANTELION)
        ),
        Some(murashah.nisbi.clone()),
        WAZN_ALAMAT_DANTELION,
    );
}

// ---------------------------------------------------------------------------
// Rockstar RAGE
// ---------------------------------------------------------------------------

impl Fahis for FahisRage {
    fn ism(&self) -> &'static str {
        "rage"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let jidhr = jidhr(siyaq)?;
        let mut hasila = HasilatFahs::la_shay();

        // The gate: one listing of the game root. This engine puts its archives
        // beside the executable, and a game with none of them is not one.
        if !jidhr.fihi_imtidad(&["rpf"]) {
            return Ok(hasila);
        }

        let wujida = rpf(&jidhr, &mut hasila);
        hawiyat_saghira_rage(&jidhr, &mut hasila);
        if wujida {
            tanfidhi_rage(&jidhr, &mut hasila);
        }
        Ok(hasila)
    }
}

/// The archives. Returns whether one of them really was one.
fn rpf(jidhr: &Fahras, hasila: &mut HasilatFahs) -> bool {
    for murashah in jidhr.malaffat(&["rpf"], HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some((adad, qina)) = tarwisat_rpf(&nafidha) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Rage,
            NawDaleel::TarwisatHawiya,
            format!(
                "an archive whose first four bytes are `RPF7` stored little-endian, declaring \
                 {adad} entries with the encryption tag {qina:#010X} behind them. Both the tag \
                 and the encryption value are this engine's own and neither is shared with \
                 anything else that ships in a game directory."
            ),
            Some(murashah.nisbi),
            WAZN_SIHR_RPF,
        );
        return true;
    }
    false
}

/// Whether an archive header is one, how many entries it claims, and which
/// encryption tag it carries.
fn tarwisat_rpf(nafidha: &[u8]) -> Option<(u32, u32)> {
    if !sihr_fi(nafidha, 0, SIHR_RPF) {
        return None;
    }
    let adad = raqm32_sagheer(nafidha, 4)?;
    if adad == 0 || adad > AQSA_QUYUD_RPF {
        return None;
    }
    let qina = raqm32_sagheer(nafidha, 12)?;
    AQNAAT_RPF.contains(&qina).then_some((adad, qina))
}

/// The startup list and the archive cache, neither understood past its tag.
fn hawiyat_saghira_rage(jidhr: &Fahras, hasila: &mut HasilatFahs) {
    for (ism, sihr, wazn, wasf) in [
        (
            MALAF_RGL,
            SIHR_RGL,
            WAZN_SIHR_RGL,
            "the list this engine reads at startup",
        ),
        (
            MALAF_CACHE_RPF,
            SIHR_CACHE_RPF,
            WAZN_SIHR_CACHE_RPF,
            "the index cache the engine keeps beside its archives",
        ),
    ] {
        let Some(haqiqi) = jidhr.malaf(ism) else {
            continue;
        };
        let murashah = jidhr.wasal(haqiqi);
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        if !sihr_fi(&nafidha, 0, sihr) {
            continue;
        }
        hasila.sajjil_aila(
            AilatMuharrik::Rage,
            NawDaleel::TarwisatHawiya,
            format!(
                "`{haqiqi}` — {wasf} — opening with `{}`. Nothing behind the tag is understood \
                 and nothing behind it was read.",
                String::from_utf8_lossy(&sihr)
            ),
            Some(murashah.nisbi),
            wazn,
        );
    }
}

/// Reads the engine's own subsystem tag out of the executable.
fn tanfidhi_rage(jidhr: &Fahras, hasila: &mut HasilatFahs) {
    let Some(murashah) = akbar_tanfidhi(jidhr) else {
        return;
    };
    if let Some(bayan) = bayan_tanfidhi(&murashah.masar) {
        if let Some(mimariya) = bayan.mimariya {
            hasila.mimariya = Some(mimariya);
        }
        sharikat_mutabiqa(&bayan, &murashah.nisbi, SHARIKAT_RAGE, hasila);
        if let Some(khaam) = bayan.isdar_muntaj.as_deref() {
            isdar_ghayr_muharrik(
                khaam,
                &murashah.nisbi,
                "it is the game's own product version, which this studio increments per title \
                 update and not per engine release",
                hasila,
            );
        }
    }

    let Some(mawdi) = alama_fi_qism(&murashah.masar, b".rdata", ALAMAT_RAGE) else {
        return;
    };
    hasila.sajjil_aila(
        AilatMuharrik::Rage,
        NawDaleel::TawqiThunai,
        format!(
            "`{}` appears {mawdi} bytes into the executable's read-only data, tagging this \
             engine's own log format strings. A bracketed subsystem tag is a weaker kind of \
             naming than a version-resource field — whoever typed the format string chose it — \
             so it is weighted below the archive header and is only looked for at all in a game \
             whose archives already said this engine.",
            String::from_utf8_lossy(ALAMAT_RAGE)
        ),
        Some(murashah.nisbi.clone()),
        WAZN_ALAMAT_RAGE,
    );
}

// ---------------------------------------------------------------------------
// Ubisoft Snowdrop
// ---------------------------------------------------------------------------

impl Fahis for FahisSnowdrop {
    fn ism(&self) -> &'static str {
        "snowdrop"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        let jidhr = jidhr(siyaq)?;
        let mut hasila = HasilatFahs::la_shay();
        let Some(bayanat) = judhur_sdf(siyaq, &jidhr) else {
            return Ok(hasila);
        };

        sdftoc(&bayanat, &mut hasila);
        sdfdata(&bayanat, &mut hasila);
        Ok(hasila)
    }
}

/// Finds the engine's data root by naming every level of it.
///
/// The path is `<project>/sdf/<platform>/data`, four directories below the game
/// and one past [`crate::fahs::AQSA_UMQ`]. It is reached by name rather than by
/// walking, so a game that is not Snowdrop pays one listing of its root plus one
/// listing per top-level directory and stops the moment a level does not hold
/// `sdf` — and a game that is one costs three more listings after that.
///
/// The platform directory and the leaf are both optional on the way down: the
/// chunks are taken from whichever of the two levels actually holds them, so a
/// layout that skips one is still found.
fn judhur_sdf(siyaq: &SiyaqFahs<'_>, jidhr: &Fahras) -> Option<Fahras> {
    let mut judhur: Vec<Fahras> = Vec::new();
    if let Some(sdf) = fahras_faree(siyaq, jidhr, MUJALLAD_SDF) {
        judhur.push(sdf);
    }
    for far in jidhr.mujalladat(AQSA_MADAKHIL_MUJALLAD) {
        if !judhur.is_empty() {
            break;
        }
        let mashru = fahras(siyaq, &far);
        if let Some(sdf) = fahras_faree(siyaq, &mashru, MUJALLAD_SDF) {
            judhur.push(sdf);
        }
    }

    let sdf = judhur.into_iter().next()?;
    for mansa in sdf.mujalladat(AQSA_FURU) {
        let nisbi = format!("{}/{mansa}", sdf.nisbi);
        let fahras = fahras(siyaq, &nisbi);
        if fahras.fihi_imtidad(&["sdftoc", "sdfdata"]) {
            return Some(fahras);
        }
        if let Some(waraq) = fahras_faree(siyaq, &fahras, MUJALLAD_BAYANAT_SDF)
            && waraq.fihi_imtidad(&["sdftoc", "sdfdata"])
        {
            return Some(waraq);
        }
    }
    None
}

/// Streams a directory for the first file with one of these extensions.
///
/// Separate from [`Fahras`] because it answers a different question: the index
/// is a picture of what a directory holds, bounded so that picture stays cheap,
/// while this is a search for one named thing in a directory whose other
/// contents are of no interest. See [`AQSA_MADAKHIL_SDF`] for the measurement
/// that made the difference matter.
fn awwal_bi_imtidad(fahras: &Fahras, imtidadat: &[&str], hadd: usize) -> Option<Murashah> {
    let madakhil = fs::read_dir(&fahras.masar).ok()?;
    for (adad, madkhal) in madakhil.enumerate() {
        if adad >= hadd {
            break;
        }
        let Ok(madkhal) = madkhal else { continue };
        if huwa_mujallad(&madkhal) {
            continue;
        }
        let khaam = madkhal.file_name();
        let Some(ism) = khaam.to_str() else { continue };
        if imtidadat.iter().any(|matlub| imtidad(ism, matlub)) {
            return Some(fahras.wasal(ism));
        }
    }
    None
}

/// The table of contents, and the studio name inside it.
fn sdftoc(bayanat: &Fahras, hasila: &mut HasilatFahs) {
    let mut murashahun = bayanat.malaffat(&["sdftoc"], HISSAT_SINF);
    if murashahun.is_empty() {
        murashahun.extend(awwal_bi_imtidad(bayanat, &["sdftoc"], AQSA_MADAKHIL_SDF));
    }
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_TOC) else {
            continue;
        };
        let Some(isdar) = sihr_wa_isdar_sdf(&nafidha, SIHR_SDFTOC) else {
            continue;
        };
        let Some(mawdi) = mawdi(&nafidha, ALAMAT_MASSIVE) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Snowdrop,
            NawDaleel::TarwisatHawiya,
            format!(
                "this engine's table of contents: `WEST` at offset zero, format version \
                 {isdar}, and the ASCII name `{}` at offset {mawdi} inside the same header — \
                 the studio that wrote the engine, spelled out in its own index. Two \
                 independent things in one file, which is why this outranks a tag on its own.",
                String::from_utf8_lossy(ALAMAT_MASSIVE)
            ),
            Some(murashah.nisbi),
            WAZN_TARWISAT_SDFTOC,
        );
        return;
    }
}

/// The content chunks the table of contents indexes.
fn sdfdata(bayanat: &Fahras, hasila: &mut HasilatFahs) {
    for murashah in bayanat.malaffat(&["sdfdata"], HISSAT_SINF) {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(isdar) = sihr_wa_isdar_sdf(&nafidha, SIHR_SDFDATA) else {
            continue;
        };
        hasila.sajjil_aila(
            AilatMuharrik::Snowdrop,
            NawDaleel::TarwisatHawiya,
            format!(
                "a content chunk opening `BERG` at format version {isdar} — the other half of \
                 this engine's pair, carrying the same version number as the table of contents \
                 beside it. Nothing behind the eight bytes is understood and nothing behind \
                 them was read."
            ),
            Some(murashah.nisbi),
            WAZN_SIHR_SDFDATA,
        );
        return;
    }
}

/// A four-byte tag with a plausible format version behind it.
fn sihr_wa_isdar_sdf(nafidha: &[u8], sihr: [u8; 4]) -> Option<u32> {
    if !sihr_fi(nafidha, 0, sihr) {
        return None;
    }
    let isdar = raqm32_sagheer(nafidha, 4)?;
    (isdar > 0 && isdar <= AQSA_ISDAR_SDF).then_some(isdar)
}

#[cfg(test)]
mod ikhtibarat {
    use std::io::Write as _;

    use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

    use super::*;
    use crate::dalail::tanfidhi::qeemat_mawrid;
    use crate::dalail::tanfidhi::suwar::{Sura, sawwir};

    // -----------------------------------------------------------------------
    // Fixtures.
    //
    // Every byte below was copied out of a shipped install on the machine this
    // module was written on — EA SPORTS FC 26, Crimson Desert Enhanced, Crash
    // Bandicoot N. Sane Trilogy, DARK SOULS: REMASTERED, ELDEN RING, Grand
    // Theft Auto V Enhanced and Avatar: Frontiers of Pandora — and none of it
    // was reconstructed from a specification. Where a fixture is padded out
    // behind the header the padding is zeros and is never read: what a fixture
    // has to reproduce is the header, and inventing a body would be inventing
    // evidence.
    // -----------------------------------------------------------------------

    /// The `StringFileInfo` block of `Engine.Render.Core2.PlatformPcDx12.dll`,
    /// verbatim: 582 bytes lifted out of the shipped DLL's `.rsrc` section,
    /// starting at the `String` structure header in front of `CompanyName`.
    ///
    /// It is here rather than reconstructed because of `InternalName`, which in
    /// this real resource has a **zero-length value**. That is the case a reader
    /// that only looks behind the key gets wrong, and it is the reason the shared
    /// reader in `crate::dalail::tanfidhi` reads `wValueLength` first.
    const MAWARID_FROSTBITE: &str = "\
        40001000010043006F006D00700061006E0079004E0061006D0065000000000045006C0065006300\
        740072006F006E0069006300200041007200740073000000720025000100460069006C0065004400\
        650073006300720069007000740069006F006E0000000000460072006F0073007400620069007400\
        65002000720065006E0064006500720069006E006700200070006C006100740066006F0072006D00\
        200073007500700070006F0072007400000000002E0007000100460069006C006500560065007200\
        730069006F006E000000000032002E00340032002E0035000000000020000000010049006E007400\
        650072006E0061006C004E0061006D00650000007400280001004C006500670061006C0043006F00\
        7000790072006900670068007400000043006F007000790072006900670068007400200028004300\
        290020003200300032003000200045006C0065006300740072006F006E0069006300200041007200\
        74007300200049006E0063002E0000002800000001004F0072006900670069006E0061006C004600\
        69006C0065006E0061006D006500000034000A000100500072006F0064007500630074004E006100\
        6D00650000000000460072006F007300740062006900740065000000320007000100500072006F00\
        6400750063007400560065007200730069006F006E00000032002E00340032002E00350000000000\
        440000000100560061007200460069006C00650049006E0066006F00000000002400040000005400\
        720061006E0073006C006100740069006F006E000000";

    /// The `StringFileInfo` block of `bin64/pers.exe`, verbatim: 550 bytes out
    /// of the shipped executable's `.rsrc`.
    ///
    /// `CompanyName` here is an empty value too, and `LegalCopyright` carries
    /// the studio instead — which is why [`sharikat_mutabiqa`] falls back from
    /// the first field to the second.
    const MAWARID_BLACKSPACE: &str = "\
        22000100010043006F006D00700061006E0079004E0061006D006500000000000000000032000500\
        0100460069006C0065004400650073006300720069007000740069006F006E000000000050004500\
        520053000000000034000A000100460069006C006500560065007200730069006F006E0000000000\
        320036002E0034002E00310033002E003100000032000900010049006E007400650072006E006100\
        6C004E0061006D006500000050004500520053002E00650078006500000000004400100001004C00\
        6500670061006C0043006F007000790072006900670068007400000050006500610072006C006100\
        6200790073007300200043006F007200700000002A00010001004C006500670061006C0054007200\
        6100640065006D00610072006B00730000000000000000003A00090001004F007200690067006900\
        6E0061006C00460069006C0065006E0061006D006500000050004500520053002E00650078006500\
        00000000460013000100500072006F0064007500630074004E0061006D0065000000000042006C00\
        610063006B00530070006100630065002000760065007200730069006F006E000000000038000A00\
        0100500072006F006400750063007400560065007200730069006F006E000000320036002E003400\
        2E00310033002E003100000038000800010041007300730065006D0062006C007900200056006500\
        7200730069006F006E00000031002E0030002E0030002E00310000000000";

    /// `FC 26/Data/layout.toc`. All 87 tables in that install open the same way.
    const TARWISAT_TOC: [u8; 16] = [
        0x00, 0xD1, 0xCE, 0x01, 0x00, 0x00, 0x00, 0x00, 0x12, 0xAD, 0xA6, 0x5E, 0x5B, 0x32, 0x64,
        0xF6,
    ];

    /// `Crimson Desert/0000/0.pamt`, whose constant sits at offset eight.
    const TARWISAT_PAMT: [u8; 16] = [
        0x40, 0x3D, 0xE6, 0x54, 0x24, 0x00, 0x00, 0x00, 0x32, 0x02, 0x0E, 0x61, 0x00, 0x00, 0x00,
        0x00,
    ];

    /// `Crimson Desert/0000/0.paz`, one of the 44 that really are archives.
    const TARWISAT_PAZ: [u8; 16] = [
        0x50, 0x41, 0x52, 0x20, 0x02, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    /// `Crimson Desert/meta/0.paver`, in full. The file is ten bytes long.
    const TARWISAT_PAVER: [u8; 10] = [0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0xCB, 0x5F, 0x1E, 0xA3];

    /// `Crash Bandicoot - N Sane Trilogy/MemoryConfiguration.igz`.
    const TARWISAT_IGZ: [u8; 16] = [
        0x01, 0x5A, 0x47, 0x49, 0x0A, 0x00, 0x00, 0x00, 0x60, 0x3C, 0x5A, 0xBD, 0x06, 0x00, 0x00,
        0x00,
    ];

    /// `Crash Bandicoot - N Sane Trilogy/archives/4ktextures_crash1.pak`.
    const TARWISAT_IGA: [u8; 16] = [
        0x49, 0x47, 0x41, 0x1A, 0x0B, 0x00, 0x00, 0x00, 0x72, 0x0B, 0x00, 0x00, 0x23, 0x00, 0x00,
        0x00,
    ];

    /// `Crash Bandicoot - N Sane Trilogy/build.xml`, in full.
    const IQLA_ALCHEMY: &[u8] = br#"<Bootstrap>
	<BUILD build="normal" SaveFileGameIdentifier="SAVE" FullTitle="Crash Bandicoot(TM) N. Sane Trilogy"/>
</Bootstrap>
"#;

    /// `DARK SOULS REMASTERED/msg/ENGLISH/item.msgbnd.dcx`, header and both
    /// sub-blocks. All 4 576 `.dcx` files in that install share these 24 bytes.
    const TARWISAT_DCX: [u8; 44] = [
        0x44, 0x43, 0x58, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
        0x24, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x2C, 0x44, 0x43, 0x53, 0x00, 0x00, 0x2F,
        0x80, 0xA0, 0x00, 0x05, 0x48, 0x57, 0x44, 0x43, 0x50, 0x00, 0x44, 0x46, 0x4C, 0x54,
    ];

    /// `ELDEN RING/Game/Data2.bdt` — the same header shape with every value
    /// different, which is why the check resolves the offsets rather than
    /// comparing bytes.
    const TARWISAT_DCX_KRAK: [u8; 44] = [
        0x44, 0x43, 0x58, 0x00, 0x00, 0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
        0x24, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x4C, 0x44, 0x43, 0x53, 0x00, 0x00, 0x05,
        0x3E, 0x02, 0x00, 0x04, 0x1E, 0xF4, 0x44, 0x43, 0x50, 0x00, 0x4B, 0x52, 0x41, 0x4B,
    ];

    /// `DARK SOULS REMASTERED/facegen/FaceGen.fgbnd`.
    const TARWISAT_BND: [u8; 16] = [
        0x42, 0x4E, 0x44, 0x33, 0x30, 0x39, 0x47, 0x31, 0x37, 0x58, 0x35, 0x31, 0x74, 0x00, 0x00,
        0x00,
    ];

    /// `Grand Theft Auto V Enhanced/common.rpf`.
    const TARWISAT_RPF: [u8; 16] = [
        0x37, 0x46, 0x50, 0x52, 0xC1, 0x02, 0x00, 0x00, 0xD0, 0x2F, 0x00, 0x00, 0xFF, 0xFF, 0xEF,
        0x0F,
    ];

    /// `Grand Theft Auto V Enhanced/title.rgl`.
    const TARWISAT_RGL: [u8; 16] = [
        0x52, 0x47, 0x4C, 0x4D, 0x01, 0x00, 0x00, 0x00, 0x10, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    /// `Grand Theft Auto V Enhanced/rpf.cache`.
    const TARWISAT_CACHE: [u8; 16] = [
        0x48, 0x53, 0x48, 0x52, 0x0D, 0x00, 0x86, 0x04, 0x86, 0xFA, 0xA7, 0x44, 0x30, 0xEA, 0x81,
        0x11,
    ];

    /// `AFOP/rogue/sdf/pc/data/sdf.sdftoc`, as far as the studio name inside it.
    const TARWISAT_SDFTOC: [u8; 60] = [
        0x57, 0x45, 0x53, 0x54, 0x29, 0x00, 0x00, 0x00, 0xC9, 0x4E, 0x03, 0x07, 0x32, 0x05, 0x90,
        0x03, 0x00, 0x00, 0x00, 0x00, 0x88, 0x13, 0x00, 0x00, 0xF8, 0x03, 0x00, 0x00, 0x0F, 0x00,
        0x00, 0x00, 0x58, 0x5B, 0x7E, 0x00, 0xEA, 0x07, 0x03, 0x00, 0x0F, 0x00, 0x13, 0x00, 0x1E,
        0x00, 0x28, 0x00, 0x6D, 0x61, 0x73, 0x73, 0x69, 0x76, 0x65, 0x00, 0x52, 0x46, 0x85, 0xC8,
    ];

    /// `AFOP/rogue/sdf/pc/data/sdf-A-0000.sdfdata`. All 1 380 chunks open with
    /// these eight bytes.
    const TARWISAT_SDFDATA: [u8; 12] = [
        0x42, 0x45, 0x52, 0x47, 0x29, 0x00, 0x00, 0x00, 0x8C, 0x0A, 0x00, 0x04,
    ];

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Decodes a hex fixture, dropping anything that is not a hex digit.
    fn min_sitteen(nass: &str) -> Vec<u8> {
        let arqam: Vec<u8> = nass
            .bytes()
            .filter(u8::is_ascii_hexdigit)
            .map(qeemat_raqm)
            .collect();
        arqam
            .chunks_exact(2)
            .filter_map(|zawj| match zawj {
                [aala, adna] => Some((aala << 4) | adna),
                _ => None,
            })
            .collect()
    }

    /// One hex digit's value.
    fn qeemat_raqm(harf: u8) -> u8 {
        match harf {
            b'0'..=b'9' => harf.saturating_sub(b'0'),
            b'a'..=b'f' => harf.saturating_sub(b'a').saturating_add(10),
            _ => harf.saturating_sub(b'A').saturating_add(10),
        }
    }

    /// Writes a file under `jidhr`, creating its parents.
    fn iktub(jidhr: &Path, nisbi: &str, bayt: &[u8]) -> std::io::Result<()> {
        let masar = jidhr.join(nisbi);
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid)?;
        }
        File::create(&masar)?.write_all(bayt)
    }

    /// A header padded out with zeros to a length.
    fn mamdud(tarwisa: &[u8], tul: usize) -> Vec<u8> {
        let mut bayt = tarwisa.to_vec();
        bayt.resize(tul.max(tarwisa.len()), 0);
        bayt
    }

    /// A minimal 64-bit PE image carrying an `.rdata` section and a version
    /// resource.
    ///
    /// The wrapper is built by the reader's own test support and both payloads
    /// are not: what is under test is that the resource tree resolves to the
    /// right block, that the real `String` structures inside it decode, and that
    /// the marker scan finds a byte run at a real offset inside a section — and
    /// a ninety-megabyte executable in the source tree would test the same thing
    /// and cost a thousand times as much to read.
    fn pe_bi_qismayn(rdata: &[u8], mawarid: &[u8]) -> Vec<u8> {
        sawwir(&Sura {
            alat: 0x8664,
            rdata,
            ism_mawarid: b".rsrc",
            hashw: 0,
            kutla: mawarid,
        })
    }

    /// Runs one detector over a directory.
    fn ifhas(fahis: &dyn Fahis, jidhr: &Path) -> HasilatFahs {
        let beea = BeeatTawafuq::Asli;
        let siyaq = SiyaqFahs {
            jidhr,
            tanfidhi: None,
            ism: "",
            nizam: NizamTashghil::Windows,
            beea: &beea,
        };
        fahis
            .ifhas(&siyaq)
            .unwrap_or_else(|_| HasilatFahs::la_shay())
    }

    /// Runs every detector in this module and returns what each one found.
    fn ifhas_kul(jidhr: &Path) -> Vec<(&'static str, HasilatFahs)> {
        jamee()
            .iter()
            .map(|fahis| (fahis.ism(), ifhas(fahis.as_ref(), jidhr)))
            .collect()
    }

    /// The strongest observation whose description contains `ibara`.
    fn wazn_daleel(hasila: &HasilatFahs, ibara: &str) -> Option<u8> {
        hasila
            .dalail
            .iter()
            .filter(|daleel| daleel.wasf.contains(ibara))
            .map(|daleel| daleel.wazn)
            .max()
    }

    // -----------------------------------------------------------------------
    // The six installs
    // -----------------------------------------------------------------------

    /// EA SPORTS FC 26's shape, with the real version resource and the real
    /// table-of-contents header in it.
    fn luba_frostbite(jidhr: &Path) -> std::io::Result<()> {
        iktub(
            jidhr,
            "Engine.Render.Core2.PlatformPcDx12.dll",
            &pe_bi_qismayn(b"", &min_sitteen(MAWARID_FROSTBITE)),
        )?;
        // The sibling with no version resource at all, which is why the
        // detector reads more than one module.
        iktub(
            jidhr,
            "Engine.Render.Core2.PlatformVulkan.dll",
            &pe_bi_qismayn(b"", b""),
        )?;
        iktub(jidhr, "Data/layout.toc", &mamdud(&TARWISAT_TOC, 85_050))?;
        iktub(
            jidhr,
            "Data/Win32/careersba.toc",
            &mamdud(&TARWISAT_TOC, 502_458),
        )?;
        Ok(())
    }

    /// Crimson Desert Enhanced's shape.
    fn luba_blackspace(jidhr: &Path) -> std::io::Result<()> {
        iktub(
            jidhr,
            "bin64/pers.exe",
            &pe_bi_qismayn(b"", &min_sitteen(MAWARID_BLACKSPACE)),
        )?;
        iktub(jidhr, "0000/0.pamt", &mamdud(&TARWISAT_PAMT, 7_124_186))?;
        iktub(jidhr, "0000/0.paz", &mamdud(&TARWISAT_PAZ, 4096))?;
        iktub(jidhr, "0001/0.paz", &mamdud(&TARWISAT_PAZ, 4096))?;
        iktub(jidhr, "meta/0.paver", &TARWISAT_PAVER)?;
        Ok(())
    }

    /// Crash Bandicoot N. Sane Trilogy's shape.
    fn luba_alchemy(jidhr: &Path) -> std::io::Result<()> {
        iktub(
            jidhr,
            "MemoryConfiguration.igz",
            &mamdud(&TARWISAT_IGZ, 5188),
        )?;
        iktub(
            jidhr,
            "archives/4ktextures_crash1.pak",
            &mamdud(&TARWISAT_IGA, 4096),
        )?;
        iktub(jidhr, "build.xml", IQLA_ALCHEMY)?;
        Ok(())
    }

    /// DARK SOULS: REMASTERED's shape — the older generation, flat at the root.
    fn luba_dantelion_qadeema(jidhr: &Path) -> std::io::Result<()> {
        iktub(
            jidhr,
            "DarkSoulsRemastered.exe",
            &pe_bi_qismayn(
                b"N:\\FRPG\\Source\\Dantelion2\\dist\\Include\\dantelion2/Core",
                b"",
            ),
        )?;
        iktub(
            jidhr,
            "msg/ENGLISH/item.msgbnd.dcx",
            &mamdud(&TARWISAT_DCX, 346_275),
        )?;
        iktub(jidhr, "facegen/FaceGen.fgbnd", &mamdud(&TARWISAT_BND, 4096))?;
        for ism in ["param", "paramdef", "mtd", "chr", "menu"] {
            fs::create_dir_all(jidhr.join(ism))?;
        }
        Ok(())
    }

    /// ELDEN RING's shape — the newer generation, one directory down.
    fn luba_dantelion_jadeeda(jidhr: &Path) -> std::io::Result<()> {
        iktub(jidhr, "Game/Data2.bdt", &mamdud(&TARWISAT_DCX_KRAK, 8192))?;
        iktub(
            jidhr,
            "Game/Data2.bhd",
            &mamdud(&[0x80, 0xCC, 0x7A, 0xEE], 4096),
        )?;
        iktub(jidhr, "Game/regulation.bin", &mamdud(&[0x00], 4096))?;
        iktub(
            jidhr,
            "Game/eldenring.exe",
            &pe_bi_qismayn(
                b"W:\\GR\\RootBranch\\Source\\Library\\Dantelion2\\dist",
                b"",
            ),
        )?;
        Ok(())
    }

    /// Grand Theft Auto V Enhanced's shape.
    fn luba_rage(jidhr: &Path) -> std::io::Result<()> {
        iktub(jidhr, "common.rpf", &mamdud(&TARWISAT_RPF, 37_796))?;
        iktub(jidhr, "title.rgl", &mamdud(&TARWISAT_RGL, 2144))?;
        iktub(jidhr, "rpf.cache", &mamdud(&TARWISAT_CACHE, 4096))?;
        iktub(
            jidhr,
            "GTA5_Enhanced.exe",
            &pe_bi_qismayn(b"[RAGE] netKxThrPool %u", b""),
        )?;
        Ok(())
    }

    /// Avatar: Frontiers of Pandora's shape. No executable at all, because the
    /// protector on the real one leaves a reader nothing and this module reads
    /// none.
    fn luba_snowdrop(jidhr: &Path) -> std::io::Result<()> {
        iktub(
            jidhr,
            "rogue/sdf/pc/data/sdf.sdftoc",
            &mamdud(&TARWISAT_SDFTOC, 8192),
        )?;
        iktub(
            jidhr,
            "rogue/sdf/pc/data/sdf-A-0000.sdfdata",
            &mamdud(&TARWISAT_SDFDATA, 4096),
        )?;
        iktub(
            jidhr,
            "rogue/sdf/pc/data/sdf-A-0000.sdfdata.hash",
            &mamdud(&[0x09], 76),
        )?;
        Ok(())
    }

    /// Lays one install out under a temporary directory.
    type BinaLuba = fn(&Path) -> std::io::Result<()>;

    /// One row of the table below: the detector's name, the layout to build,
    /// the family that layout must answer, and the weight it must answer at.
    type SafLuba = (&'static str, BinaLuba, AilatMuharrik, u8);

    /// Every install this module recognises, with the family and the strongest
    /// weight each one must produce.
    const AL_ALAAB: [SafLuba; 7] = [
        (
            "frostbite",
            luba_frostbite,
            AilatMuharrik::Frostbite,
            WAZN_ISM_FROSTBITE,
        ),
        (
            "blackspace",
            luba_blackspace,
            AilatMuharrik::BlackSpace,
            WAZN_ISM_BLACKSPACE,
        ),
        (
            "alchemy",
            luba_alchemy,
            AilatMuharrik::Alchemy,
            WAZN_SIHR_IG,
        ),
        (
            "dantelion",
            luba_dantelion_qadeema,
            AilatMuharrik::Dantelion,
            WAZN_TARWISAT_DCX,
        ),
        (
            "dantelion",
            luba_dantelion_jadeeda,
            AilatMuharrik::Dantelion,
            WAZN_TARWISAT_DCX,
        ),
        ("rage", luba_rage, AilatMuharrik::Rage, WAZN_SIHR_RPF),
        (
            "snowdrop",
            luba_snowdrop,
            AilatMuharrik::Snowdrop,
            WAZN_TARWISAT_SDFTOC,
        ),
    ];

    /// Every shipped shape answers its own engine, at the strength of its own
    /// strongest signature.
    #[test]
    fn kul_luba_tujeeb_bi_muharrikiha() -> std::io::Result<()> {
        for (ism, bina, aila, wazn) in AL_ALAAB {
            let masrah = tempfile::tempdir()?;
            bina(masrah.path())?;
            let hasila = ifhas_kul(masrah.path())
                .into_iter()
                .find(|(fahis, _)| *fahis == ism)
                .map_or_else(HasilatFahs::la_shay, |(_, hasila)| hasila);

            assert_eq!(hasila.aila, Some(aila), "{ism}");
            assert_eq!(hasila.aqwa(), wazn, "{ism}: {:?}", hasila.dalail);
        }
        Ok(())
    }

    /// **The negative control, and the point of the whole module.** No install
    /// answers any detector but its own.
    ///
    /// Written as the full cross product rather than as a spot check, because a
    /// signature that fires on a game it does not describe is the one failure
    /// that costs a user something: it renames their game, changes the tier
    /// sentence they read, and sends the wrong extractor at their files.
    #[test]
    fn la_yujeeb_fahis_ala_luba_ghayrih() -> std::io::Result<()> {
        for (ism, bina, _, _) in AL_ALAAB {
            let masrah = tempfile::tempdir()?;
            bina(masrah.path())?;
            for (fahis, hasila) in ifhas_kul(masrah.path()) {
                if fahis == ism {
                    continue;
                }
                assert!(
                    !hasila.wajad(),
                    "the {fahis} detector answered on the {ism} layout: {:?}",
                    hasila.dalail
                );
            }
        }
        Ok(())
    }

    /// The engines already in the tree answer nothing here either.
    ///
    /// The four shapes are the ones on the machine this was written on: a Unity
    /// game with its player library and a Mono runtime, an Unreal game with its
    /// pak and its shipping binary, a Godot game with its pack, and Capcom's
    /// BIO4 with its data directory and one of its containers.
    #[test]
    fn al_muharrikat_al_maruufa_la_tujeeb() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let jidhr = masrah.path();
        iktub(jidhr, "hollow_knight.exe", b"MZ")?;
        iktub(jidhr, "UnityPlayer.dll", b"MZ")?;
        iktub(jidhr, "hollow_knight_Data/globalgamemanagers", b"\0\0\0\0")?;
        fs::create_dir_all(jidhr.join("MonoBleedingEdge"))?;
        iktub(
            jidhr,
            "Atlas/Content/Paks/Atlas-WindowsNoEditor.pak",
            b"\0\0\0\0",
        )?;
        iktub(jidhr, "Atlas/Binaries/Win64/Atlas.exe", b"MZ")?;
        iktub(jidhr, "Engine/Build/Build.version", b"{}")?;
        iktub(jidhr, "game.pck", b"GDPC")?;
        iktub(jidhr, "BIO4/Font/common_j.fnt.lfs", b"RDLX\xAA\xBA\xEE\xFE")?;
        iktub(jidhr, "Bin32/bio4.exe", b"MZ")?;

        for (fahis, hasila) in ifhas_kul(jidhr) {
            assert!(
                !hasila.wajad(),
                "{fahis} answered on a Unity/Unreal/Godot/BIO4 layout: {:?}",
                hasila.dalail
            );
        }
        Ok(())
    }

    /// An empty directory is no engine, and a missing one is an error rather
    /// than an answer.
    #[test]
    fn faragh_wa_ghiyab() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        for (fahis, hasila) in ifhas_kul(masrah.path()) {
            assert!(!hasila.wajad(), "{fahis} answered on an empty directory");
        }

        let beea = BeeatTawafuq::Asli;
        let mafqud = masrah.path().join("la-shay");
        let siyaq = SiyaqFahs {
            jidhr: &mafqud,
            tanfidhi: None,
            ism: "",
            nizam: NizamTashghil::Windows,
            beea: &beea,
        };
        for fahis in jamee() {
            assert!(
                fahis.ifhas(&siyaq).is_err(),
                "{} accepted a missing root",
                fahis.ism()
            );
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // The layering
    // -----------------------------------------------------------------------

    /// One detector's answer for one install.
    fn wahid(ism: &str, bina: BinaLuba) -> std::io::Result<HasilatFahs> {
        let masrah = tempfile::tempdir()?;
        bina(masrah.path())?;
        Ok(ifhas_kul(masrah.path())
            .into_iter()
            .find(|(fahis, _)| *fahis == ism)
            .map_or_else(HasilatFahs::la_shay, |(_, hasila)| hasila))
    }

    /// Every observation each engine exists to make is present, each at its own
    /// weight.
    ///
    /// Written as an exhaustive list rather than a spot check, because the
    /// layering *is* the design: an engine naming itself outranks a container
    /// tag, a container tag outranks a directory shape, and a studio's name
    /// outranks nothing at all.
    #[test]
    fn kul_daleel_bi_waznihi() -> std::io::Result<()> {
        let jadwal: [(&str, BinaLuba, Vec<(&str, u8)>); 7] = [
            (
                "frostbite",
                luba_frostbite,
                vec![
                    ("`ProductName` as `Frostbite`", WAZN_ISM_FROSTBITE),
                    ("`ProductVersion` as `2.42.5`", WAZN_ISM_FROSTBITE),
                    ("Electronic Arts", WAZN_SHARIKA),
                    ("archive index opening `00 D1 CE 01`", WAZN_SIHR_TOC),
                    ("in the shape this engine reads it", WAZN_BINYA_FROSTBITE),
                ],
            ),
            (
                "blackspace",
                luba_blackspace,
                vec![
                    ("`ProductName` as `BlackSpace version`", WAZN_ISM_BLACKSPACE),
                    ("Pearlabyss Corp", WAZN_SHARIKA),
                    ("metadata table carrying the constant", WAZN_SIHR_PAMT),
                    ("content container opening `PAR `", WAZN_SIHR_PAZ),
                    ("version stamp beside its metadata", WAZN_PAVER),
                    ("four-digit content directories", WAZN_BINYA_BLACKSPACE),
                    ("`26.4.13.1` was read here", 0),
                ],
            ),
            (
                "alchemy",
                luba_alchemy,
                vec![
                    ("not an Unreal pak", WAZN_SIHR_IG),
                    ("this engine's `IGZ` tag", WAZN_SIHR_IG),
                    (
                        "where this engine keeps every archive",
                        WAZN_MUJALLAD_ARSHIF,
                    ),
                    ("the file this engine reads at startup", WAZN_IQLA_ALCHEMY),
                ],
            ),
            (
                "dantelion",
                luba_dantelion_qadeema,
                vec![
                    ("`DCX\\0` compression wrapper", WAZN_TARWISAT_DCX),
                    ("`Dantelion2` appears", WAZN_ALAMAT_DANTELION),
                    ("archive opening `BND3`", WAZN_SIHR_BND),
                    ("this engine's data tree", WAZN_BINYA_DANTELION),
                    ("where this engine keeps every string", WAZN_MAWDI_NUSUS),
                ],
            ),
            (
                "dantelion",
                luba_dantelion_jadeeda,
                vec![
                    ("`DCX\\0` compression wrapper", WAZN_TARWISAT_DCX),
                    ("`Dantelion2` appears", WAZN_ALAMAT_DANTELION),
                    (
                        "archive pairs beside `regulation.bin`",
                        WAZN_BINYA_DANTELION_ARSHIF,
                    ),
                ],
            ),
            (
                "rage",
                luba_rage,
                vec![
                    ("`RPF7` stored little-endian", WAZN_SIHR_RPF),
                    ("`[RAGE]` appears", WAZN_ALAMAT_RAGE),
                    ("the list this engine reads at startup", WAZN_SIHR_RGL),
                    ("the index cache", WAZN_SIHR_CACHE_RPF),
                ],
            ),
            (
                "snowdrop",
                luba_snowdrop,
                vec![
                    (
                        "table of contents: `WEST` at offset zero",
                        WAZN_TARWISAT_SDFTOC,
                    ),
                    ("content chunk opening `BERG`", WAZN_SIHR_SDFDATA),
                ],
            ),
        ];
        for (ism, bina, mutawaqqa) in jadwal {
            let hasila = wahid(ism, bina)?;
            for (ibara, wazn) in mutawaqqa {
                assert_eq!(
                    wazn_daleel(&hasila, ibara),
                    Some(wazn),
                    "{ism}: missing or misweighted: {ibara}"
                );
            }
        }
        Ok(())
    }

    /// The ordering itself, so that a later edit to one weight cannot silently
    /// make a directory name worth as much as a magic number, or a bracketed
    /// log tag worth as much as a version-resource field.
    #[test]
    fn tarteeb_al_awzan() {
        const {
            assert!(WAZN_MAWDI_NUSUS < WAZN_IQLA_ALCHEMY);
            assert!(WAZN_IQLA_ALCHEMY < WAZN_MUJALLAD_ARSHIF);
            assert!(WAZN_MUJALLAD_ARSHIF < WAZN_SHARIKA);
            assert!(WAZN_SHARIKA <= WAZN_PAVER);
            assert!(WAZN_PAVER < WAZN_SIHR_CACHE_RPF);
            assert!(WAZN_SIHR_CACHE_RPF < WAZN_BINYA_BLACKSPACE);
            assert!(WAZN_BINYA_FROSTBITE <= WAZN_BINYA_BLACKSPACE);
            assert!(WAZN_BINYA_BLACKSPACE < WAZN_BINYA_DANTELION);
            assert!(WAZN_BINYA_DANTELION < WAZN_SIHR_PAZ);
            // The one place a shape outranks a container tag, and it is
            // deliberate: `PAR ` is three letters and a space, while a pair of
            // archives named alike beside a rule stamp is a layout nothing else
            // has. A tag is not automatically worth more than a shape — how
            // arbitrary the tag is decides that.
            assert!(WAZN_SIHR_PAZ < WAZN_BINYA_DANTELION_ARSHIF);
            assert!(WAZN_BINYA_DANTELION_ARSHIF < WAZN_SIHR_BND);
            assert!(WAZN_SIHR_BND <= WAZN_SIHR_RGL);
            assert!(WAZN_SIHR_RGL < WAZN_ALAMAT_RAGE);
            assert!(WAZN_ALAMAT_RAGE < WAZN_SIHR_TOC);
            assert!(WAZN_SIHR_TOC <= WAZN_SIHR_PAMT);
            assert!(WAZN_SIHR_PAMT < WAZN_ALAMAT_DANTELION);
            assert!(WAZN_ALAMAT_DANTELION <= WAZN_SIHR_SDFDATA);
            assert!(WAZN_SIHR_SDFDATA < WAZN_ISM_BLACKSPACE);
            assert!(WAZN_ISM_BLACKSPACE <= WAZN_TARWISAT_SDFTOC);
            assert!(WAZN_TARWISAT_SDFTOC < WAZN_SIHR_IG);
            assert!(WAZN_SIHR_IG <= WAZN_TARWISAT_DCX);
            assert!(WAZN_TARWISAT_DCX < WAZN_ISM_FROSTBITE);
        }
    }

    // -----------------------------------------------------------------------
    // Refusals — the extension is never enough
    // -----------------------------------------------------------------------

    /// A file carrying one of these engines' extensions and none of their bytes
    /// buys nothing at all.
    ///
    /// Every one of these is a real collision rather than an invented one:
    /// `.toc` is an ebook's table of contents and a CD cue sheet, `.pak` is
    /// Unreal's and Quake's, `.data` and `.paz` are whatever anybody wants.
    #[test]
    fn imtidad_bila_tarwisa_yurfad() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let jidhr = masrah.path();
        iktub(
            jidhr,
            "Data/layout.toc",
            b"\\contentsline {chapter}{Preface}{1}\n",
        )?;
        iktub(jidhr, "archives/level.pak", b"PACK\x0c\x00\x00\x00")?;
        iktub(jidhr, "map.igz", b"<?xml version=\"1.0\"?>")?;
        iktub(jidhr, "0000/0.pamt", &mamdud(&[0x00; 16], 4096))?;
        iktub(jidhr, "meta/0.paver", b"not ten bytes at all")?;
        iktub(
            jidhr,
            "msg/ENGLISH/item.dcx",
            b"DCX\0\xff\xff\xff\xff\xff\xff\xff\xff",
        )?;
        iktub(
            jidhr,
            "common.rpf",
            b"7FPR\x01\x00\x00\x00\x00\x00\x00\x00dead",
        )?;
        iktub(
            jidhr,
            "rogue/sdf/pc/data/x.sdftoc",
            b"WEST\x29\x00\x00\x00 no studio here",
        )?;
        fs::create_dir_all(jidhr.join("param"))?;

        for (fahis, hasila) in ifhas_kul(jidhr) {
            assert!(
                !hasila.wajad(),
                "{fahis} believed an extension: {:?}",
                hasila.dalail
            );
        }
        Ok(())
    }

    /// The table of contents is found even when it is the last entry in a
    /// directory of thousands.
    ///
    /// The real condition, reproduced rather than described: Avatar's data root
    /// holds 2 763 entries and hands `sdf.sdftoc` back at position 2 760, so a
    /// detector reading a 256-entry index of that directory answers at the
    /// weight of a content chunk and never sees the studio's own name. This
    /// builds three hundred chunks with the table of contents written last and
    /// pins the strongest observation, which is the one the bound was dropping.
    #[test]
    fn sdftoc_yujad_khalfa_hadd_alfahras() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let jidhr = masrah.path();
        for raqm in 0..300_u32 {
            let ism = format!("rogue/sdf/pc/data/sdf-A-{raqm:04}.sdfdata");
            iktub(jidhr, &ism, &mamdud(&TARWISAT_SDFDATA, 64))?;
        }
        iktub(
            jidhr,
            "rogue/sdf/pc/data/sdf.sdftoc",
            &mamdud(&TARWISAT_SDFTOC, 8192),
        )?;
        let hasila = ifhas(&FahisSnowdrop::jadeed(), jidhr);

        assert_eq!(hasila.aila, Some(AilatMuharrik::Snowdrop));
        assert_eq!(
            hasila.aqwa(),
            WAZN_TARWISAT_SDFTOC,
            "the index bound swallowed the table of contents: {:?}",
            hasila.dalail
        );
        Ok(())
    }

    /// A `DCX\0` whose own offsets do not resolve is refused, and one whose do
    /// is accepted in both generations.
    #[test]
    fn dcx_bila_kutlatayn_yurfad() {
        assert_eq!(tarwisat_dcx(&TARWISAT_DCX).as_deref(), Some("DFLT"));
        assert_eq!(tarwisat_dcx(&TARWISAT_DCX_KRAK).as_deref(), Some("KRAK"));

        let mut kadhib = TARWISAT_DCX;
        // The offset at 0x0C, which is supposed to land on `DCP\0`.
        if let Some(bayt) = kadhib.get_mut(15) {
            *bayt = 0x20;
        }
        assert!(
            tarwisat_dcx(&kadhib).is_none(),
            "an offset that misses `DCP` is not a header"
        );
    }

    /// An `RPF7` with an encryption tag nobody ships is refused.
    #[test]
    fn rpf_bi_qina_majhul_yurfad() {
        assert_eq!(tarwisat_rpf(&TARWISAT_RPF), Some((705, 0x0FEF_FFFF)));
        let mut kadhib = TARWISAT_RPF;
        if let Some(bayt) = kadhib.get_mut(12) {
            *bayt = 0x00;
        }
        assert!(tarwisat_rpf(&kadhib).is_none());
        // A zero entry count is not a header either.
        let mut farigh = TARWISAT_RPF;
        for mawdi in 4..8 {
            if let Some(bayt) = farigh.get_mut(mawdi) {
                *bayt = 0x00;
            }
        }
        assert!(tarwisat_rpf(&farigh).is_none());
    }

    /// A `.pamt` whose constant is not at offset eight is refused.
    #[test]
    fn pamt_bila_thabit_yurfad() {
        assert_eq!(tarwisat_pamt(&TARWISAT_PAMT), Some(36));
        let mut kadhib = TARWISAT_PAMT;
        if let Some(bayt) = kadhib.get_mut(8) {
            *bayt = 0x33;
        }
        assert!(tarwisat_pamt(&kadhib).is_none());
    }

    // -----------------------------------------------------------------------
    // The version resource
    // -----------------------------------------------------------------------

    /// A zero-length value reads as absent rather than as the next key.
    ///
    /// The regression this whole reader exists for. `InternalName` in EA's real
    /// resource has `wValueLength` zero, and a reader that steps from the key
    /// straight to the text behind it returns `t(\u{1}LegalCopyright` — a
    /// plausible-looking string that is two struct headers and somebody else's
    /// key.
    #[test]
    fn qeema_faragha_tuqra_ka_ghiyab() {
        let mawarid = min_sitteen(MAWARID_FROSTBITE);
        assert_eq!(
            qeemat_mawrid(&mawarid, "ProductName").as_deref(),
            Some("Frostbite")
        );
        assert_eq!(
            qeemat_mawrid(&mawarid, "ProductVersion").as_deref(),
            Some("2.42.5")
        );
        assert_eq!(
            qeemat_mawrid(&mawarid, "CompanyName").as_deref(),
            Some("Electronic Arts")
        );
        assert_eq!(
            qeemat_mawrid(&mawarid, "InternalName"),
            None,
            "an empty value is absent"
        );

        let pearl = min_sitteen(MAWARID_BLACKSPACE);
        assert_eq!(
            qeemat_mawrid(&pearl, "ProductName").as_deref(),
            Some("BlackSpace version")
        );
        assert_eq!(
            qeemat_mawrid(&pearl, "LegalCopyright").as_deref(),
            Some("Pearlabyss Corp")
        );
        assert_eq!(
            qeemat_mawrid(&pearl, "CompanyName"),
            None,
            "an empty value is absent"
        );
    }

    /// A version is parsed only where the engine published one, and never out of
    /// a build stamp.
    #[test]
    fn al_isdar_yuqra_marratan_wahida() -> std::io::Result<()> {
        let frostbite = wahid("frostbite", luba_frostbite)?;
        let isdar = frostbite.isdar.unwrap_or_else(|| IsdarMuharrik {
            kabir: 0,
            sagheer: 0,
            tasheeh: 0,
            khaam: String::new(),
            mushtaqq: false,
        });
        assert_eq!((isdar.kabir, isdar.sagheer, isdar.tasheeh), (2, 42, 5));
        assert_eq!(isdar.khaam, "2.42.5");
        assert!(
            !isdar.mushtaqq,
            "this one was read off the engine's own module"
        );

        let bila_isdar: [(&str, BinaLuba); 5] = [
            ("blackspace", luba_blackspace),
            ("alchemy", luba_alchemy),
            ("dantelion", luba_dantelion_qadeema),
            ("rage", luba_rage),
            ("snowdrop", luba_snowdrop),
        ];
        for (ism, bina) in bila_isdar {
            assert!(
                wahid(ism, bina)?.isdar.is_none(),
                "{ism} invented a version"
            );
        }

        // And the parser itself refuses anything whose first component is not a
        // number, so a codename can never become a release.
        assert!(hallil_isdar("2.42.5").is_some());
        assert!(hallil_isdar("1.0.0RELEASE_DEV.0").is_none());
        assert!(hallil_isdar("Release-5.3").is_none());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Through resolution and into the report
    // -----------------------------------------------------------------------

    /// The whole path for every one of the six: evidence, resolution, and the
    /// report a player reads.
    ///
    /// Pinned end to end rather than at the detector, because the value of a
    /// newly-named engine is what comes out of `imkaniyat` — and the honest
    /// answer for all six is that naming them changed the *name* and not the
    /// tier. The assertions say exactly that: tier 3, readiness absent, a gap
    /// sentence that names the engine, and no text system claimed.
    #[test]
    fn min_aladilla_ila_altaqreer() -> std::io::Result<()> {
        for (ism, bina, aila, _) in AL_ALAAB {
            let masrah = tempfile::tempdir()?;
            bina(masrah.path())?;

            let mut jami = crate::fahs::JamiHasilat::default();
            for (fahis, hasila) in ifhas_kul(masrah.path()) {
                jami.hasilat.push((fahis, hasila));
            }
            let muharrik = crate::tahdid::hall(&jami);
            assert_eq!(muharrik.aila, aila, "{ism}");
            assert_eq!(
                muharrik.khalfiya,
                taarib_mustalahat::muharrik::KhalfiyaBarmajiya::Majhula,
                "{ism}: compiled C++ has no value in this vocabulary and must not borrow one"
            );

            let taqreer =
                crate::imkaniyat::taqreer(muharrik, &[], "2026-09-06T00:00:00Z".to_owned());
            assert_eq!(
                taqreer.tabaqa,
                taarib_mustalahat::muharrik::Tabaqa::TarjamaFawqiya,
                "{ism}: naming an engine is not the same as reaching into it"
            );
            assert_eq!(
                taqreer.jahiziya,
                taarib_mustalahat::muharrik::JahiziyatTashghil::Ghaiba,
                "{ism}"
            );
            assert!(taqreer.anzimat_qabila.is_empty(), "{ism}");
            assert_eq!(taqreer.isdar_fahs, crate::imkaniyat::ISDAR_FAHS);

            let naqs = taqreer
                .naqs
                .unwrap_or_else(|| taarib_mustalahat::muharrik::Hadd {
                    arabi: String::new(),
                    injilizi: String::new(),
                });
            assert!(
                naqs.injilizi.contains(aila.ism()),
                "{ism}: the gap must name the engine"
            );
            assert!(
                taqreer.sabab_injilizi.contains(aila.ism()),
                "{ism}: the tier's reason must name the engine"
            );
            assert!(
                !taqreer.sabab_injilizi.contains("did not recognise"),
                "{ism}: a named engine must not be described as unrecognised"
            );
        }
        Ok(())
    }
}
