//! الحاوية — Unreal's `.pak`, read from its last bytes forward.
//!
//! A `.pak` is a concatenation of file payloads with an index bolted onto the
//! end and a footer after that. Nothing at the front of the file identifies it:
//! byte zero of a shipping pak is the first byte of somebody's texture. So the
//! reader starts at the end, and everything else follows from where it lands.
//!
//! This module is the **full container reader and a small container writer**.
//! `taarib_muharrik::dalail::unreal::DhaylPak` is the cheap detection probe —
//! one seek, one 512-byte read, no index, no decryption — and the two are not
//! duplicates of each other: the probe answers "is this an Unreal game and
//! roughly which engine", this answers "give me the bytes of
//! `Content/Localization/Game/en/Game.locres`". The engine-version mapping is
//! not repeated here at all. [`TadhyeelPak::mada`] builds a `DhaylPak` and asks
//! it, so there is exactly one table saying what pak version 8 with four
//! compression slots implies, and it lives in Phase 5.
//!
//! ## The footer, and why finding it is a search
//!
//! `FPakInfo` is serialized at the very end of the file, in this order:
//!
//! ```text
//! FPakInfo — 44, 45, 61, 189, 221 or 222 bytes, little-endian, at EOF
//!
//!   size  field                type       present when
//!     16  muarrif_miftah       FGuid      version >= 7
//!      1  fahras_mushaffar     u8         version >= 4
//!      4  sihr = 0x5A6F12E1    u32        always
//!      4  isdar                u32        always
//!      8  izahat_fahras        u64        always
//!      8  hajm_fahras          u64        always
//!     20  basmat_fahras        SHA-1      always
//!      1  mujammad             u8         version == 9 only
//!    128  asma_daght           4 x 32     version 8, first shape
//!    160  asma_daght           5 x 32     version >= 8, second shape
//! ```
//!
//! Add those up and the footer is 44 bytes for versions 1 through 3, 45 for
//! versions 4 through 6, 61 for version 7, 189 for the four-slot shape of
//! version 8, 221 for the five-slot shape of version 8 and for versions 10 and
//! 11, and 222 for version 9 — which is the only version that writes
//! `bIndexIsFrozen`.
//!
//! **`bEncryptedIndex` arrived with the feature it describes, at version 4.**
//! Versions 1 to 3 have no such field: `FPakInfo` did not carry one and
//! `UnrealPak` did not write one, so their footer is forty-four bytes and the
//! byte in front of the magic is the last byte of the index. Reading it as a
//! flag is wrong twice over — it is a field that does not exist, and it makes
//! this reader report a footer one byte longer than the file has. Both readings
//! put the magic at the same distance from EOF, so the search cannot tell them
//! apart by where the magic lands; only the version separates them, which is why
//! the two sizes carry disjoint version lists in [`AHJAM_TADHYEEL`] and why
//! either order of trying them gives the same answer. The corroboration is
//! arithmetic on a real container: a version 3 pak's `izahat_fahras +
//! hajm_fahras + 44` is exactly its length on disk.
//!
//! **The version is inside the footer and the footer's size depends on the
//! version.** That is the circularity, and there is no way out of it that reads
//! only one place: to know where the magic is you must know the version, and to
//! know the version you must first find the magic. So the reader searches. It
//! takes the last [`HAJM_DHAYL`] bytes once, tries each known footer size
//! largest-first, and accepts the first candidate whose magic is where that size
//! puts it *and* whose version is one that size is legal for. Both halves are
//! required: `0x5A6F12E1` is four bytes and a large enough tail will eventually
//! contain any four bytes you ask it for, so a magic that is not corroborated by
//! a plausible version and an index that fits inside the file is a coincidence,
//! not a footer.
//!
//! Largest-first matters. A version 9 footer is a version 8 five-slot footer
//! plus one byte, and a 221-byte read of a 222-byte footer lands one byte off,
//! where it finds nothing — but the reverse mistake, accepting the shorter shape
//! for the longer file, would misread the compression name table by one byte and
//! then expand every block with the wrong method. Trying the longest candidates
//! first means the ambiguous pair is resolved in favour of the shape that
//! actually carries the extra field.
//!
//! ## The eleven versions, and what each one changed
//!
//! | version | name | what it changed for a reader |
//! | --- | --- | --- |
//! | 1 | `Initial` | the original: every entry carries an `int64` timestamp |
//! | 2 | `NoTimestamps` | the timestamp is gone; entries shrink by eight bytes |
//! | 3 | `CompressionEncryption` | entries gain `bEncrypted`, a block size, a block list |
//! | 4 | `IndexEncryption` | the index may be AES-encrypted; the footer byte now means something |
//! | 5 | `RelativeChunkOffsets` | block offsets are relative to the entry, not to the file |
//! | 6 | `DeleteRecords` | an entry may say "this path is *removed*", for patch paks |
//! | 7 | `EncryptionKeyGuid` | the footer names which key was used, before the flag byte |
//! | 8 | `FNameBasedCompressionMethod` | methods are names in a footer table; 4 slots, then 5 |
//! | 9 | `FrozenIndex` | one extra footer byte for a memory-image index this build refuses |
//! | 10 | `PathHashIndex` | the flat index becomes a path-hash index plus a directory index |
//! | 11 | `Fnv64BugFix` | the same shapes, with the path hash corrected to Fnv64 |
//!
//! Version 9's frozen index is a serialized memory image of the engine's own
//! containers, laid out for the target platform's pointer size and alignment. It
//! is not a format; it is a memory dump. Reading it would mean reproducing one
//! specific engine build's `TMap` layout, and getting it subtly wrong produces
//! entries rather than an error. Version 9 with the frozen flag set is refused
//! by name; version 9 without it reads as an ordinary flat index, which is what
//! `UnrealPak` writes unless it is asked otherwise.
//!
//! ## Two index shapes
//!
//! **Versions 1 through 9** carry one flat index: a mount point, a count, and
//! then that many (path, entry) pairs, each entry serialized in full.
//!
//! **Versions 10 and 11** replace it with three regions. The primary index at
//! `izahat_fahras` holds the mount point, the count, a path-hash seed, and then
//! the offset, size and SHA-1 of two secondary regions plus a blob of
//! *encoded* entries and a tail of full ones. The **path-hash index** maps a
//! 64-bit hash of the lowercased path to a location in that blob, and is what
//! the engine uses at runtime. The **full-directory index** maps directory and
//! file names to the same locations, and is what makes a pak enumerable — a
//! shipping build may omit it, and a pak without it can be read by hash but
//! cannot be listed. Each of the three is independently compressed by nothing
//! and independently *encrypted*, so a reader that decrypted the primary index
//! and then read the other two as plaintext gets two regions of noise and no
//! error. This module decrypts each region separately, as the engine does.
//!
//! A location is an `int32` with three readings: zero or above is a byte offset
//! into the encoded blob, below zero and not `i32::MIN` is `-(n + 1)` into the
//! array of full entries, and `i32::MIN` means no entry at all.
//!
//! ## Entries are stored twice, with different offset bases
//!
//! This is the single most consequential thing about the format, and the one a
//! reader gets wrong silently.
//!
//! Every file's entry header appears **in the index** and again **immediately
//! before the file's own bytes**. The two copies describe the same file and do
//! not hold the same numbers:
//!
//! ```text
//!   izaha ──►  ┌──────────────────────────┐
//!              │ entry header (copy two)  │  Offset written as 0
//!              │  ...                     │  block offsets relative to here
//!              ├──────────────────────────┤
//!              │ payload bytes            │  data starts at izaha + tul_musalsal
//!              └──────────────────────────┘
//! ```
//!
//! * The index copy's `izaha` is an absolute file offset, and it points at the
//!   *second copy*, not at the payload. The payload begins
//!   [`MadkhalPak::tul_musalsal`] bytes later.
//! * The second copy writes `izaha` as zero, because it is describing itself.
//! * From version 5 the compression-block offsets in both copies are relative to
//!   the entry header — the value written already includes the header's own
//!   length — and before version 5 they are absolute file offsets.
//! * Versions 10 and 11 add a third form, the *encoded* entry, which stores no
//!   block offsets at all. It stores block lengths, and they accumulate from the
//!   **payload start**, not from the entry header: a third base, in the shape
//!   that a shipping UE 4.27 game actually uses. See [`madkhal_min_murammaz`].
//!
//! A reader that seeks to the index copy's `izaha` and reads `hajm_makhzun`
//! bytes gets the entry header plus the first part of the payload, decompresses
//! garbage, and reports a decompression failure — if it is lucky. A reader that
//! treats a version 5 block offset as absolute reads from near the start of the
//! file, which for a fifty gigabyte pak is somebody else's texture that
//! decompresses cleanly. This module resolves both copies to absolute file
//! offsets once, in [`MadkhalPak`], and nothing downstream sees a relative
//! number.
//!
//! ## AES-256 in ECB, and what a wrong key looks like
//!
//! Unreal encrypts pak regions with **AES-256 in ECB mode** — not CBC, no IV, no
//! chaining. Sixteen-byte blocks, each independent. Every encrypted region is
//! padded up to a multiple of sixteen before encryption, so the ciphertext is
//! longer than the plaintext and the plaintext must be **truncated back to the
//! declared size** afterwards. Skipping that truncation leaves up to fifteen
//! bytes of padding on the end of an index, which shifts nothing and breaks
//! nothing until the last entry, and then produces a refusal that names the
//! wrong field.
//!
//! No key and an encrypted index is [`KhataUnreal::PakMushaffar`]. Taarib does
//! not look for the key. The key ships inside the game's own executable and is
//! recoverable, and a player extracting their own game's text is doing nothing
//! this product needs to be shy about — but a tool that recovered keys would be
//! a tool with a second purpose, and this one has exactly one. The user supplies
//! it or the container is not read.
//!
//! A key that is supplied and does not work is
//! [`KhataUnreal::MiftahGhayrSalih`], and the test is worth being precise about,
//! because **there is no honest test for "is this the right key" other than
//! decrypting and looking**. ECB has no authentication tag; every 32-byte key
//! "works" in the sense that it produces output. The recorded SHA-1 cannot
//! settle it either, for the reason in the next section. So the test is
//! self-consistency of the result: the mount point must be a well-formed
//! `FString` with its NUL terminator, the entry count must be non-negative, and
//! the bytes those two imply must fit inside the region that was decrypted. A
//! wrong key fails that within the first eight bytes with overwhelming
//! probability, and a right key passes it always. It is a probabilistic test
//! stated as one, rather than a certainty this format cannot provide.
//!
//! ## The recorded SHA-1, and both places it might have been taken
//!
//! The footer records a SHA-1 of the index. Engine versions disagree about
//! whether it was taken over the index as *stored* or over the index as
//! *decrypted*, and the disagreement is invisible on the unencrypted paks that
//! most tools are tested against, where the two are the same bytes. A reader
//! that picked one reading would refuse a large fraction of shipping encrypted
//! paks and tell the user their key was wrong.
//!
//! So [`HawiyatPak`] accepts a hash that matches *either* reading and refuses
//! one that matches neither with [`KhataUnreal::BasmaGhayrMutabaqa`]. Matching
//! either means the writer recorded it; matching neither means the bytes changed
//! after it was written. An all-zero recorded hash is treated as "not recorded"
//! and skipped, because a hash of zero is not the SHA-1 of anything and older
//! writers left the field unfilled.
//!
//! ## Compression
//!
//! `Zlib` and `Gzip` go through `flate2`, `Zstd` through `zstd`, `None` is
//! copied. **`Oodle` is refused** with [`KhataUnreal::DaghtMajhul`]: Taarib has
//! no licence to ship an Oodle decompressor. The intended route is the game's
//! own — every Oodle-compressed game links a decompressor and Unreal exports it
//! — and calling into it belongs to the runtime half of this adapter, not to a
//! module that parses files offline. Below version 8 the method is an
//! `ECompressionFlags` bitfield rather than a name; the bias bits are masked off
//! and the low bits mapped, so a version 3 pak and a version 11 pak produce the
//! same [`TareeqatDaght`].
//!
//! ## What this module writes
//!
//! [`KatibPak`] builds one small, uncompressed, unencrypted container holding
//! the files the patch replaces — the `.locres` for Arabic, the `.locmeta` that
//! lists it, and nothing else. It writes **[`ISDAR_KITABA`], pak version 5**,
//! and the reasoning is the whole justification for the writer being this small:
//!
//! * Version 3 is the first that can state "this entry is not compressed and not
//!   encrypted" in the fields it actually carries, rather than by the absence of
//!   fields a reader has to infer from the version.
//! * Version 5 is the first where block offsets are relative to the entry, which
//!   is the convention every later version keeps. An entry this module writes
//!   therefore means the same thing to a 4.17 reader and to a 5.5 reader.
//! * Everything above 5 adds structures a patch does not need and cannot get
//!   wrong for free: a key GUID (7), a compression-name table (8), a frozen
//!   index (9), a path-hash index (10, 11). A path-hash index with one wrong
//!   hash produces a pak that mounts and then resolves nothing, which is the
//!   worst failure available here because it looks like success.
//! * Every engine from 4.17 through UE5 loads a version 5 index through its
//!   legacy path, which it must keep for the patch paks its own users shipped.
//!
//! A game whose engine predates 4.17 declares a lower version in its own
//! containers, and an engine refuses a container newer than it can read — a
//! 4.13 title reads version 3 and would leave a version 5 patch unmounted with
//! one line in its log. So [`KatibPak::bi_isdar`] follows the game down:
//! [`IsdarPak::lil_kitaba`] takes the game's own version, caps it at
//! [`ISDAR_KITABA`], and refuses below 3, where entries cannot even say "not
//! compressed, not encrypted". Between 3 and 5 the entry bytes this writer
//! emits are identical; only the footer's flag byte, which arrived at 4, comes
//! and goes.
//!
//! ## `_P`, and why the patch is one deletable file
//!
//! Unreal mounts `.pak` files from a directory in **name order**, and gives a
//! container whose name ends in `_P` a higher mount priority than one that does
//! not. A later mount wins for any path both containers hold. So a patch named
//! [`crate::ISM_HAWIYA`] — `zzz_taarib_P.pak` — sorts after every container a
//! game ships, carries the `_P` suffix that raises it above them, and overrides
//! the paths it holds without a byte of the game's own containers being touched.
//!
//! Both halves of the name do work. `zzz` wins the ordering among `_P` files, so
//! the patch also sits above any other mod the player installed. `_P` wins the
//! priority class, so it sits above the game's own content even if the game's
//! own containers sort later. Installing is writing one file into
//! [`crate::DALIL_HAWIYA`]; uninstalling is deleting it; and there is no third
//! state in which a game is half-patched.
//!
//! ## Bounds
//!
//! Every count and every length in a pak is a number a stranger wrote that
//! decides how much memory this process is about to reserve, and this process is
//! sometimes the game itself. Each one is judged against a named ceiling and
//! against the bytes actually present before it reaches an allocator. The
//! payload is memory-mapped rather than read, because a shipping pak routinely
//! passes fifty gigabytes and the reader touches a few kilobytes of it.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use aes::Aes256;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, KeyInit};
use taarib_muharrik::dalail::unreal::{DhaylPak, HAJM_DHAYL_PAK, MadaIsdar};

use super::{Katib, NassMukhazzan, Qari, adad_musir, tahaqquq_adad};
use crate::khata::{KhataUnreal, hajm_usize, tul_u64};

/// The format name every refusal from this module carries.
const ISM: &str = "pak";

/// `FPakInfo::PakFile_Magic`, little-endian in the file.
pub const SIHR: u32 = 0x5A6F_12E1;

/// The highest container version this build reads.
pub const AQSA_ISDAR: u32 = 11;

/// The highest container version [`KatibPak`] writes, and its default.
///
/// Five. The justification is in this module's header, and it is short: version
/// 5 is the lowest version whose entries mean the same thing to every engine
/// from 4.17 through UE5, and every version above it adds a structure a patch
/// does not need and could get wrong. A game whose own containers are older
/// gets its own version instead — see [`IsdarPak::lil_kitaba`].
pub const ISDAR_KITABA: u32 = 5;

/// The lowest container version [`KatibPak`] writes.
///
/// Three is where an entry first carries its encryption flag and block size,
/// which is what lets a writer state "raw, unencrypted" in fields the engine
/// reads rather than in fields it infers from the version.
pub const ADNA_ISDAR_KITABA: u32 = 3;

/// How many bytes at the end of a `.pak` the footer search reads.
///
/// The same 512 bytes Phase 5's detection probe reads, named here so that the
/// two never drift apart: a footer the probe can find and this reader cannot
/// would be a game reported as readable and then refused.
pub const HAJM_DHAYL: usize = HAJM_DHAYL_PAK;

/// The AES block size, in bytes. ECB, so this is also the padding unit.
pub const HAJM_KUTLAT_TASHFEER: usize = 16;

/// The length of an AES-256 key, in bytes.
pub const HAJM_MIFTAH: usize = 32;

/// The length of a SHA-1 digest, in bytes.
pub const HAJM_BASMA: usize = 20;

/// The width of one compression-method name slot in the footer, from version 8.
///
/// `FName::StringBufferSize`. The name is NUL-padded into the slot rather than
/// length-prefixed, so an unused slot is thirty-two zero bytes.
pub const HAJM_ISM_DAGHT: usize = 32;

/// How many compression-method slots version 8's first footer shape reserves.
///
/// Four, which is 4.22 and nothing else. The slot count is the only thing that
/// separates 4.22 from 4.23 in a container, and Phase 5's version mapping keys
/// off it.
pub const KHANAT_DAGHT_AWWALIYA: usize = 4;

/// How many slots every footer from 4.23 onward reserves.
pub const KHANAT_DAGHT_MUWASSAA: usize = 5;

/// The known footer sizes, each with the versions that size is legal for.
///
/// Ordered **largest first**, which is the whole reason this is an ordered array
/// and not a map: version 9's footer is version 8's five-slot footer plus one
/// byte, and trying the shorter shape first on a version 9 file would find its
/// magic one byte early only if the byte before it happened to match — rare, but
/// the failure it produces is a compression-name table read one byte off, which
/// is silent. Longest first resolves the pair toward the shape that carries the
/// extra field.
///
/// The 44/45 pair is not that kind of ambiguity and does not depend on the
/// order. Both shapes put the magic at the same byte of the file — 45 is 44 with
/// one more field *before* the magic — so both candidates find it and the
/// version alone decides, which is why their version lists share nothing.
///
/// The sizes are the sums from this module's header: 44 = 4 + 4 + 8 + 8 + 20,
/// plus 1 for the encrypted-index flag from version 4, plus 16 for the key GUID
/// from version 7, plus 32 per compression slot from version 8, plus 1 for the
/// frozen flag at version 9 exactly.
const AHJAM_TADHYEEL: [(u64, &[u32]); 6] = [
    (222, &[9]),
    (221, &[8, 10, 11]),
    (189, &[8]),
    (61, &[7]),
    (45, &[4, 5, 6]),
    (44, &[1, 2, 3]),
];

/// The largest index this build will hold in memory at once.
///
/// Two hundred and fifty-six mebibytes. A pak index is a mount point, a count
/// and one record per file; a game with two million packaged files has an index
/// in the low hundreds of megabytes, and nothing real is above that. Checked
/// against the *declared* size in the footer, before the bytes are taken.
pub const AQSA_HAJM_FAHRAS: u64 = 256 * 1024 * 1024;

/// The largest number of entries an index may declare.
pub const AQSA_MADAKHIL: u64 = 4 * 1024 * 1024;

/// The largest number of compression blocks one entry may declare.
///
/// A million blocks at the engine's usual 64 KiB block size is a sixty-four
/// gigabyte file, which is larger than any single asset in any shipping game.
pub const AQSA_KUTAL: u64 = 1024 * 1024;

/// The largest uncompressed size one entry may declare.
///
/// Half a gibibyte, matching [`super::AQSA_MALAF`]. This module exists to reach
/// localization resources, and the largest `.locres` any game ships is
/// single-digit megabytes. The ceiling is what stops a corrupt or hostile size
/// field from reserving a game process's entire address space.
pub const AQSA_HAJM_KHAAM: u64 = 512 * 1024 * 1024;

/// The largest expanded size one compression block may declare.
pub const AQSA_KUTLA_KHAAM: u64 = 16 * 1024 * 1024;

/// The largest mount point or entry path this build accepts, in bytes.
pub const AQSA_TUL_MASAR: u64 = 8 * 1024;

/// The largest number of directories a full-directory index may declare.
pub const AQSA_DALAIL: u64 = 1024 * 1024;

/// The smallest number of bytes a flat-index record can occupy.
///
/// Four for the path's own length field, plus the forty-four bytes every entry
/// carries in every version. Used to refuse an entry count whose records could
/// not fit in the index that declares them, before the first record is parsed.
const AQALL_MADKHAL: u64 = 48;

/// `FPakEntryLocation::Invalid`.
///
/// Not zero: zero is a perfectly good offset into the encoded-entry blob, and a
/// reader that treated it as absent would drop the first file in the container.
const MAWQI_GHAYR_SALIH: i32 = i32::MIN;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// The container format version, with a predicate for every change it made.
///
/// A newtype over the engine's own number rather than an enum, because the
/// number is what the file declares and what a bug report quotes, and because a
/// build that met version 12 should be able to say "twelve" rather than lose it
/// on the way to a variant. Every question the parser asks about the version is
/// a method here, so a reader never compares a raw integer and no one has to
/// remember that relative chunk offsets arrived at five.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IsdarPak(u32);

impl IsdarPak {
    /// The version this container declares.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        self.0
    }

    /// Accepts a declared version, refusing zero and anything above
    /// [`AQSA_ISDAR`].
    ///
    /// Refused rather than parsed optimistically. A version above this build's
    /// is a format whose footer size this reader does not know, so it would have
    /// been found by the search only by coincidence, and every offset after it
    /// would be a guess.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::IsdarGhayrMadum`] for a version above [`AQSA_ISDAR`], and
    /// [`KhataUnreal::MawridTalif`] for version zero, which no engine wrote.
    pub fn min_raqm(raqm: u32) -> Result<Self, KhataUnreal> {
        if raqm == 0 {
            return Err(talif("the container version", 0, u64::from(AQSA_ISDAR)));
        }
        if raqm > AQSA_ISDAR {
            return Err(KhataUnreal::IsdarGhayrMadum {
                ism: ISM,
                wujid: raqm,
                aqsa: AQSA_ISDAR,
            });
        }
        Ok(Self(raqm))
    }

    /// The version [`KatibPak`] writes for a game whose containers declare
    /// this one.
    ///
    /// The game's own version, capped at [`ISDAR_KITABA`]: an engine reads
    /// every version up to its own and none above it, so a patch written at the
    /// game's version is always mountable and a patch written above it never
    /// is. Version 5 stays the ceiling for the reasons in this module's header.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] below [`ADNA_ISDAR_KITABA`]: a version 1 or
    /// 2 entry has no encryption flag and no block size, and this writer does
    /// not produce a layout it cannot state those two facts in.
    pub fn lil_kitaba(self) -> Result<Self, KhataUnreal> {
        if self.0 < ADNA_ISDAR_KITABA {
            return Err(talif(
                "the container version, below the lowest this writer can express",
                u64::from(self.0),
                u64::from(ADNA_ISDAR_KITABA),
            ));
        }
        Ok(Self(if self.0 > ISDAR_KITABA {
            ISDAR_KITABA
        } else {
            self.0
        }))
    }

    /// Version 2. Entries no longer carry an `int64` timestamp.
    #[must_use]
    pub const fn bila_tawaqit(self) -> bool {
        self.0 >= 2
    }

    /// Version 3. Entries carry an encrypted flag, a block size, and — when the
    /// entry is compressed — a list of compression blocks.
    #[must_use]
    pub const fn daght_wa_tashfeer(self) -> bool {
        self.0 >= 3
    }

    /// Version 4. The index itself may be encrypted, so the footer's flag byte
    /// means something. Below 4 the byte is written and ignored.
    #[must_use]
    pub const fn tashfeer_fahras(self) -> bool {
        self.0 >= 4
    }

    /// Version 5. Compression-block offsets are relative to the entry header
    /// rather than to the start of the file. See this module's header for what
    /// reading them with the wrong base does.
    #[must_use]
    pub const fn izahat_nisbiya(self) -> bool {
        self.0 >= 5
    }

    /// Version 6. An entry may be a delete record: a path this container removes
    /// from every container mounted below it, carrying no payload.
    #[must_use]
    pub const fn sijillat_hadhf(self) -> bool {
        self.0 >= 6
    }

    /// Version 7. The footer names the encryption key by GUID, sixteen bytes
    /// before the flag byte — which is what makes a version 7 footer sixteen
    /// bytes longer than a version 6 one at the same magic distance.
    #[must_use]
    pub const fn muarrif_miftah(self) -> bool {
        self.0 >= 7
    }

    /// Version 8. Compression methods are names in a footer table, indexed from
    /// one, instead of an `ECompressionFlags` bitfield in each entry.
    #[must_use]
    pub const fn jadwal_asma_daght(self) -> bool {
        self.0 >= 8
    }

    /// Version 9, and version 9 alone. One extra footer byte says whether the
    /// index is a frozen memory image. Not `>= 9`: version 10 dropped the byte
    /// again, and a reader that kept the inequality would skip a byte that is
    /// not there and misread the whole compression-name table.
    #[must_use]
    pub const fn alam_tajmid(self) -> bool {
        self.0 == 9
    }

    /// Version 10. The flat index is replaced by a primary index that points at
    /// a path-hash index and a full-directory index.
    #[must_use]
    pub const fn fahras_basmat_masar(self) -> bool {
        self.0 >= 10
    }

    /// Version 11. The path hash is Fnv64 over the lowercased path in UTF-16,
    /// corrected from version 10's implementation.
    ///
    /// It changes nothing for a reader that walks the full-directory index,
    /// which is how this module enumerates a container, and everything for one
    /// that looks a path up by hash. That is why [`FahrasPak`] resolves through
    /// names and treats the path-hash index as corroboration.
    #[must_use]
    pub const fn basmat_fnv64(self) -> bool {
        self.0 >= 11
    }

    /// How many compression-method name slots a footer of `hajm` bytes reserves
    /// for this version.
    ///
    /// Zero below version 8. From version 8 it is whatever the footer size says,
    /// because the slot count is not stored anywhere — it is the difference
    /// between two footer sizes, and that is exactly why the four-slot shape and
    /// the five-slot shape are two separate entries in [`AHJAM_TADHYEEL`].
    #[must_use]
    pub const fn khanat_daght(self, hajm: u64) -> usize {
        if !self.jadwal_asma_daght() {
            return 0;
        }
        match hajm {
            189 => KHANAT_DAGHT_AWWALIYA,
            _ => KHANAT_DAGHT_MUWASSAA,
        }
    }
}

// ---------------------------------------------------------------------------
// Refusals and bounded arithmetic
// ---------------------------------------------------------------------------

/// A refusal naming a field of the container that is inconsistent with itself.
const fn talif(haql: &'static str, qeema: u64, hadd: u64) -> KhataUnreal {
    KhataUnreal::MawridTalif {
        ism: ISM,
        haql,
        qeema,
        hadd,
    }
}

/// A refusal naming a declared size above its ceiling.
const fn mufrit(haql: &'static str, qeema: u64, saqf: u64) -> KhataUnreal {
    KhataUnreal::HajmMufrit { haql, qeema, saqf }
}

/// A refusal naming a field the file ended before.
const fn qaseer(haql: &'static str, tul: u64, matlub: u64) -> KhataUnreal {
    KhataUnreal::MalafQaseer { haql, tul, matlub }
}

/// Rounds `qeema` up to the next multiple of `wahda`, or [`None`] on overflow.
///
/// Used for exactly one thing: the size of an AES-encrypted region, which is the
/// plaintext size rounded up to sixteen. `wahda` is a constant at every call
/// site, so the division cannot be by zero, and it is still written with
/// `checked_div` because the workspace denies bare integer division and an
/// exception made here is an exception someone copies.
fn ila_ala(qeema: u64, wahda: u64) -> Option<u64> {
    let zaid = qeema.checked_add(wahda.checked_sub(1)?)?;
    zaid.checked_div(wahda)?.checked_mul(wahda)
}

/// A window of `tul` bytes at `izaha`, refusing anything that runs past the end.
///
/// The only way a sub-slice of a container is obtained in this module, so
/// "bounded against what is actually present" is a property of one function
/// rather than a habit spread over forty call sites.
fn qass<'a>(
    bayt: &'a [u8],
    izaha: u64,
    tul: u64,
    haql: &'static str,
) -> Result<&'a [u8], KhataUnreal> {
    let hadd = tul_u64(bayt.len());
    let nihaya = izaha
        .checked_add(tul)
        .ok_or_else(|| qaseer(haql, hadd, u64::MAX))?;
    let bidaya = hajm_usize(izaha).ok_or_else(|| qaseer(haql, hadd, nihaya))?;
    let tarf = hajm_usize(nihaya).ok_or_else(|| qaseer(haql, hadd, nihaya))?;
    bayt.get(bidaya..tarf)
        .ok_or_else(|| qaseer(haql, hadd, nihaya))
}

// ---------------------------------------------------------------------------
// Compression
// ---------------------------------------------------------------------------

/// `ECompressionFlags::COMPRESS_ZLIB`, in the pre-version-8 entry field.
const DAGHT_QADIM_ZLIB: u32 = 0x01;
/// `ECompressionFlags::COMPRESS_GZIP`.
const DAGHT_QADIM_GZIP: u32 = 0x02;
/// `ECompressionFlags::COMPRESS_Custom`, which in every shipping game means
/// Oodle.
const DAGHT_QADIM_KHASS: u32 = 0x04;
/// The bits of the legacy field that select a method rather than a tuning hint.
///
/// `COMPRESS_BiasMemory` and `COMPRESS_BiasSpeed` live above these and say how
/// the *compressor* was tuned. A reader that failed to mask them off would meet
/// `0x11` — zlib, tuned for memory — and report an unknown method for a stream
/// it can expand perfectly.
const QINA_DAGHT_QADIM: u32 = 0x0F;

/// How one entry, or one block of it, was compressed.
///
/// Named methods only. From version 8 the container carries the method as a
/// name, so a container built with something else names it and this reader
/// reports the name rather than an index nobody can look up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TareeqatDaght {
    /// Stored raw. Method index zero, or the literal name `None`.
    Bila,
    /// Zlib, expanded with `flate2`.
    Zlib,
    /// Gzip, expanded with `flate2` reading a gzip wrapper rather than a zlib
    /// one. The two differ only in the header, and handing a gzip stream to a
    /// zlib decoder fails on the first byte rather than quietly — which is the
    /// good case, and still a refusal a user would not understand.
    Gzip,
    /// Zstandard, expanded with `zstd`.
    Zstd,
    /// Oodle. **Refused**: Taarib has no licence to ship an Oodle decompressor,
    /// and the intended route is the game's own exported one, which belongs to
    /// the runtime half of this adapter and not to this module.
    Oodle,
    /// A method this build cannot expand, carrying the name the container gave.
    Majhula(String),
}

impl TareeqatDaght {
    /// The method a version 8-and-later name selects.
    ///
    /// Matched case-insensitively, and after trimming the NUL padding the
    /// footer's fixed-width slots carry: the name is written from an `FName`,
    /// which preserves whatever casing the cook used, into a thirty-two byte
    /// slot that is zero-filled to the end.
    #[must_use]
    pub fn min_ism(ism: &str) -> Self {
        let mahjuz = ism.trim_matches('\0').trim();
        if mahjuz.is_empty() || mahjuz.eq_ignore_ascii_case("none") {
            Self::Bila
        } else if mahjuz.eq_ignore_ascii_case("zlib") {
            Self::Zlib
        } else if mahjuz.eq_ignore_ascii_case("gzip") {
            Self::Gzip
        } else if mahjuz.eq_ignore_ascii_case("zstd") || mahjuz.eq_ignore_ascii_case("zstandard") {
            Self::Zstd
        } else if mahjuz.eq_ignore_ascii_case("oodle") {
            Self::Oodle
        } else {
            Self::Majhula(mahjuz.to_owned())
        }
    }

    /// The method a pre-version-8 `ECompressionFlags` field selects.
    ///
    /// The tuning bias bits are masked off first; see [`QINA_DAGHT_QADIM`].
    #[must_use]
    pub fn min_alam(alam: u32) -> Self {
        match alam & QINA_DAGHT_QADIM {
            0 => Self::Bila,
            DAGHT_QADIM_ZLIB => Self::Zlib,
            DAGHT_QADIM_GZIP => Self::Gzip,
            DAGHT_QADIM_KHASS => Self::Oodle,
            akhar => Self::Majhula(format!("ECompressionFlags 0x{akhar:02X}")),
        }
    }

    /// The name this method is written under, for a refusal a person can read.
    #[must_use]
    pub fn ism(&self) -> String {
        match self {
            Self::Bila => "None".to_owned(),
            Self::Zlib => "Zlib".to_owned(),
            Self::Gzip => "Gzip".to_owned(),
            Self::Zstd => "Zstd".to_owned(),
            Self::Oodle => "Oodle".to_owned(),
            Self::Majhula(ism) => ism.clone(),
        }
    }

    /// Whether this method leaves the bytes alone.
    #[must_use]
    pub const fn khaam(&self) -> bool {
        matches!(self, Self::Bila)
    }
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

/// One compression block, resolved to absolute file offsets.
///
/// The file stores these relative to the entry header from version 5 and
/// absolute before it. Both are turned into absolute offsets by
/// [`MadkhalPak::min_qari`] the moment they are read, so no code below this
/// point has to know which base the container used — which is the point, because
/// getting that base wrong does not fail, it reads the wrong bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KutlatDaght {
    /// The first byte of the stored block, from the start of the file.
    pub bidaya: u64,
    /// One past its last byte.
    pub nihaya: u64,
}

impl KutlatDaght {
    /// How many bytes the block occupies as stored, or [`None`] if the pair is
    /// inverted — which is a corrupt entry and not an empty block.
    #[must_use]
    pub const fn tul(self) -> Option<u64> {
        self.nihaya.checked_sub(self.bidaya)
    }
}

/// One file's record, as the index describes it.
///
/// Read from the index copy, which is the copy whose `izaha` is absolute. See
/// this module's header for the second copy and why the two differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalPak {
    /// The absolute offset of this entry's **second copy**, which sits
    /// immediately before the payload. The payload itself begins
    /// [`Self::tul_musalsal`] bytes later.
    pub izaha: u64,
    /// How many bytes the payload occupies as stored.
    pub hajm_makhzun: u64,
    /// How many bytes it becomes once expanded. Equal to [`Self::hajm_makhzun`]
    /// when the entry is stored raw.
    pub hajm_khaam: u64,
    /// How it was compressed, already resolved from either the version 8 name
    /// table or the legacy flags field.
    pub daght: TareeqatDaght,
    /// The raw method selector the entry declared, kept for diagnostics: a
    /// refusal that says "method index 6" is one a maintainer can check against
    /// the container's own table.
    pub fahras_daght: u32,
    /// The SHA-1 the container recorded over the *uncompressed* payload. All
    /// zeroes when the writer did not record one.
    pub basma: [u8; HAJM_BASMA],
    /// The compression blocks, in absolute file offsets. Empty when the entry is
    /// stored raw.
    pub kutal: Vec<KutlatDaght>,
    /// Whether the payload is AES-encrypted.
    pub mushaffar: bool,
    /// The uncompressed size of one compression block.
    pub hajm_kutla: u32,
    /// Whether this is a delete record: a path this container removes from every
    /// container mounted below it, carrying no payload. Version 6 and later.
    pub mahdhuf: bool,
    /// The container version this entry was read under, which decides how many
    /// bytes its second copy occupies.
    pub isdar: IsdarPak,
}

impl MadkhalPak {
    /// Reads one full entry.
    ///
    /// `jadwal` is the footer's compression-method name table, empty below
    /// version 8. `asas` is the base a version 5-and-later relative block offset
    /// is measured from: [`None`] for an index copy, whose own offset field is
    /// that base, and `Some(position)` for the copy stored before the data,
    /// whose offset field is written as zero and whose real base is where it
    /// sits. Passing the wrong one is the mistake this module's header describes
    /// — it does not fail, it reads elsewhere in the file.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] when a field runs past the end of the index,
    /// [`KhataUnreal::MawridTalif`] for a negative offset or size, a block
    /// count below zero, or a block pair that runs backwards, and
    /// [`KhataUnreal::HajmMufrit`] for a size or block count above its ceiling.
    pub fn min_qari(
        qari: &mut Qari<'_>,
        isdar: IsdarPak,
        jadwal: &[TareeqatDaght],
        asas: Option<u64>,
    ) -> Result<Self, KhataUnreal> {
        let izaha_khaam = qari.iqra_i64("an entry's offset")?;
        let mahdhuf = isdar.sijillat_hadhf() && izaha_khaam == -1;
        let muallan = if mahdhuf {
            0
        } else {
            tul_musir(izaha_khaam, "an entry's offset")?
        };
        // The second copy writes its own offset as zero, so the caller's
        // position replaces it. Without this the entry would claim to live at
        // byte zero of the container, which is the first byte of somebody's
        // texture and reads cleanly.
        let izaha = asas.unwrap_or(muallan);
        let hajm_makhzun = tul_musir(qari.iqra_i64("an entry's stored size")?, "an entry's size")?;
        let hajm_khaam = tul_musir(
            qari.iqra_i64("an entry's expanded size")?,
            "an entry's expanded size",
        )?;
        if hajm_khaam > AQSA_HAJM_KHAAM {
            return Err(mufrit(
                "an entry's expanded size",
                hajm_khaam,
                AQSA_HAJM_KHAAM,
            ));
        }

        let fahras_daght = qari.iqra_u32("an entry's compression method")?;
        let daght = if isdar.jadwal_asma_daght() {
            tareeqa_min_jadwal(fahras_daght, jadwal)
        } else {
            TareeqatDaght::min_alam(fahras_daght)
        };
        if !isdar.bila_tawaqit() {
            // Version 1 only. The timestamp is read and dropped: it is not part
            // of any decision this module makes, and preserving it would imply a
            // round-trip this module does not offer for containers it did not
            // write.
            let _ = qari.iqra_i64("an entry's timestamp")?;
        }
        let basma: [u8; HAJM_BASMA] = qari.iqra_masfufa("an entry's hash")?;

        let (kutal, mushaffar, hajm_kutla) = if isdar.daght_wa_tashfeer() {
            let kutal = if daght.khaam() {
                Vec::new()
            } else {
                kutal_min_qari(qari, isdar, izaha)?
            };
            let mushaffar = qari.iqra_u8("an entry's encryption flag")? != 0;
            (
                kutal,
                mushaffar,
                qari.iqra_u32("an entry's compression block size")?,
            )
        } else {
            (Vec::new(), false, 0u32)
        };

        Ok(Self {
            izaha,
            hajm_makhzun,
            hajm_khaam,
            daght,
            fahras_daght,
            basma,
            kutal,
            mushaffar,
            hajm_kutla,
            mahdhuf,
            isdar,
        })
    }

    /// How many bytes this entry occupies when serialized under `isdar`.
    ///
    /// This is `FPakEntry::GetSerializedSize`, and it is load-bearing twice: it
    /// is the distance from the entry's own offset to the first byte of the
    /// payload, and it is the base a version 5-and-later relative block offset
    /// is measured from. A reader that computed it with the wrong version — one
    /// timestamp, one block list — lands inside the payload and expands noise.
    #[must_use]
    pub fn tul_musalsal(&self) -> u64 {
        tul_musalsal_li(self.isdar, !self.daght.khaam(), tul_u64(self.kutal.len()))
    }

    /// The absolute offset of the first payload byte, or [`None`] on overflow.
    #[must_use]
    pub fn bidayat_bayanat(&self) -> Option<u64> {
        self.izaha.checked_add(self.tul_musalsal())
    }

    /// Whether the container recorded a hash for this entry.
    ///
    /// All zeroes is not a SHA-1 of anything, and older writers left the field
    /// unfilled. Treating it as a hash would refuse every entry in those
    /// containers with a message about tampering.
    #[must_use]
    pub fn lahu_basma(&self) -> bool {
        self.basma.iter().any(|bayt| *bayt != 0)
    }
}

/// `FPakEntry::GetSerializedSize`, as a function of the version and the shape
/// of the entry rather than of an entry value.
///
/// Kept separate from [`MadkhalPak::tul_musalsal`] because the encoded-entry
/// decoder needs the answer *before* it has an entry to ask: version 10 and 11
/// store a single-block entry's absolute block bounds as "the entry's offset
/// plus its own serialized size", so the size has to be computed from the block
/// count alone.
///
/// The forty-four bytes are the offset, the stored size, the expanded size and
/// the twenty-byte hash, which every version writes. Everything after that is a
/// version's worth of history.
const fn tul_musalsal_li(isdar: IsdarPak, madghut: bool, adad_kutal: u64) -> u64 {
    let mut tul: u64 = 44;
    tul = tul.saturating_add(4);
    if !isdar.bila_tawaqit() {
        tul = tul.saturating_add(8);
    }
    if isdar.daght_wa_tashfeer() {
        tul = tul.saturating_add(5);
        if madghut {
            tul = tul
                .saturating_add(adad_kutal.saturating_mul(16))
                .saturating_add(4);
        }
    }
    tul
}

/// A signed length or offset as a `u64`, refusing a negative one.
///
/// Unreal writes every offset and size in a pak as `int64`. A negative one is a
/// corrupt field, not a very large unsigned one, and the distinction is what
/// keeps a sign error from becoming a nine-exabyte read.
fn tul_musir(qeema: i64, haql: &'static str) -> Result<u64, KhataUnreal> {
    u64::try_from(qeema).map_err(|_| talif(haql, qeema.unsigned_abs(), i64::MAX.unsigned_abs()))
}

/// The compression method a version 8-and-later entry's index selects.
///
/// Index zero is "stored raw" and every other index is one past a name in the
/// footer's table, which is the engine's convention and not an off-by-one. An
/// index past the end of the table is reported as an unexpandable method rather
/// than silently treated as raw: an entry naming a method the footer did not
/// declare is a container whose table was truncated, and calling it raw would
/// hand compressed bytes to a caller as text.
fn tareeqa_min_jadwal(fahras: u32, jadwal: &[TareeqatDaght]) -> TareeqatDaght {
    if fahras == 0 {
        return TareeqatDaght::Bila;
    }
    hajm_usize(u64::from(fahras))
        .and_then(|fahras| fahras.checked_sub(1))
        .and_then(|fahras| jadwal.get(fahras))
        .cloned()
        .unwrap_or_else(|| TareeqatDaght::Majhula(format!("method index {fahras}")))
}

/// Reads one entry's compression-block list, resolving it to absolute offsets.
///
/// `asas` is added to every offset when the container writes them relative,
/// which is version 5 and later. The pair is also checked for order: a block
/// whose end precedes its start is refused here rather than producing a
/// subtraction that wraps into a length of eighteen quintillion.
fn kutal_min_qari(
    qari: &mut Qari<'_>,
    isdar: IsdarPak,
    asas: u64,
) -> Result<Vec<KutlatDaght>, KhataUnreal> {
    let haql = "an entry's compression block count";
    let adad = adad_musir(ISM, haql, qari.iqra_i32(haql)?)?;
    let adad = tahaqquq_adad(ISM, haql, adad, AQSA_KUTAL, 16, qari.baqi())?;
    let mut kutal = Vec::with_capacity(adad);
    let izaha = if isdar.izahat_nisbiya() { asas } else { 0 };
    for _ in 0..adad {
        let bidaya = tul_musir(
            qari.iqra_i64("a compression block's start")?,
            "a block's start",
        )?;
        let nihaya = tul_musir(qari.iqra_i64("a compression block's end")?, "a block's end")?;
        let bidaya = bidaya
            .checked_add(izaha)
            .ok_or_else(|| talif("a compression block's start", bidaya, u64::MAX))?;
        let nihaya = nihaya
            .checked_add(izaha)
            .ok_or_else(|| talif("a compression block's end", nihaya, u64::MAX))?;
        if nihaya < bidaya {
            return Err(talif("a compression block's end", nihaya, bidaya));
        }
        kutal.push(KutlatDaght { bidaya, nihaya });
    }
    Ok(kutal)
}

// ---------------------------------------------------------------------------
// SHA-1
//
// Implemented here rather than pulled in, and the reason is proportion. This is
// the only place in the product that needs SHA-1: the pak format records one
// over its index and one over each entry, and Unreal chose it long before it was
// broken. Everything Taarib itself authenticates uses BLAKE3 and Ed25519, in
// `taarib-ruqaa` and `taarib-khatm`.
//
// So the choice was a dependency whose only job is to read somebody else's
// legacy field, or eighty lines. Eighty lines that never change, against a crate
// in the supply chain of a binary that gets injected into a game process.
//
// SHA-1 here is an integrity check against a container that was rewritten or
// truncated, and nothing else. It is not a security boundary and this module
// does not treat it as one: a `.pak` is not signed, the hash sits in the same
// file as the bytes it covers, and anyone who can change one can change the
// other. What it catches is the honest case — a partial download, a failed
// patch, a mod manager that rewrote an entry — which is the case that actually
// happens.
// ---------------------------------------------------------------------------

/// How many bytes SHA-1 consumes at a time.
const HAJM_KUTLAT_BASMA: usize = 64;

/// A streaming SHA-1, exactly as FIPS 180-4 defines it.
///
/// Streaming rather than one-shot because the index of a large pak is hundreds
/// of megabytes and the entries are scattered through fifty gigabytes; hashing
/// either through one contiguous buffer would mean holding it.
#[derive(Debug, Clone)]
pub struct Basma {
    halat: [u32; 5],
    tul: u64,
    hawd: [u8; HAJM_KUTLAT_BASMA],
    mumtali: usize,
}

impl Default for Basma {
    fn default() -> Self {
        Self::jadeeda()
    }
}

impl Basma {
    /// A fresh digest, at FIPS 180-4's initial state.
    #[must_use]
    pub const fn jadeeda() -> Self {
        Self {
            halat: [
                0x6745_2301,
                0xEFCD_AB89,
                0x98BA_DCFE,
                0x1032_5476,
                0xC3D2_E1F0,
            ],
            tul: 0,
            hawd: [0u8; HAJM_KUTLAT_BASMA],
            mumtali: 0,
        }
    }

    /// Feeds more bytes in.
    pub fn hadhi(&mut self, bayt: &[u8]) {
        self.tul = self.tul.wrapping_add(tul_u64(bayt.len()).wrapping_mul(8));
        self.dafa(bayt);
    }

    /// Buffers bytes without counting them, which is what the padding needs.
    ///
    /// The length in the final block is the length of the *message*, so the
    /// padding this writes must not be added to it. Two functions rather than a
    /// flag, because a flag is a thing a later edit forgets to set.
    fn dafa(&mut self, bayt: &[u8]) {
        let mut baqi = bayt;
        while !baqi.is_empty() {
            let masaha = HAJM_KUTLAT_BASMA.saturating_sub(self.mumtali);
            let akhdh = masaha.min(baqi.len());
            let nihaya = self.mumtali.saturating_add(akhdh);
            let (awwal, thani) = baqi.split_at(akhdh);
            if let Some(khana) = self.hawd.get_mut(self.mumtali..nihaya) {
                khana.copy_from_slice(awwal);
            }
            self.mumtali = nihaya;
            baqi = thani;
            if self.mumtali == HAJM_KUTLAT_BASMA {
                let kutla = self.hawd;
                self.idghim(&kutla);
                self.mumtali = 0;
            }
        }
    }

    /// Finishes and returns the twenty-byte digest.
    #[must_use]
    pub fn anhi(mut self) -> [u8; HAJM_BASMA] {
        let tul = self.tul;
        self.dafa(&[0x80]);
        while self.mumtali != 56 {
            self.dafa(&[0]);
        }
        self.dafa(&tul.to_be_bytes());
        let mut natija = [0u8; HAJM_BASMA];
        for (kalima, khana) in self.halat.iter().zip(natija.chunks_exact_mut(4)) {
            khana.copy_from_slice(&kalima.to_be_bytes());
        }
        natija
    }

    /// The compression function, over one sixty-four byte block.
    fn idghim(&mut self, kutla: &[u8; HAJM_KUTLAT_BASMA]) {
        let mut jadwal = [0u32; 80];
        for (khana, makan) in kutla.chunks_exact(4).zip(jadwal.iter_mut()) {
            let arba: [u8; 4] = khana.try_into().unwrap_or([0u8; 4]);
            *makan = u32::from_be_bytes(arba);
        }
        for fahras in 16_usize..80 {
            let khilat = jadwal.get(fahras.saturating_sub(3)).copied().unwrap_or(0)
                ^ jadwal.get(fahras.saturating_sub(8)).copied().unwrap_or(0)
                ^ jadwal.get(fahras.saturating_sub(14)).copied().unwrap_or(0)
                ^ jadwal.get(fahras.saturating_sub(16)).copied().unwrap_or(0);
            if let Some(makan) = jadwal.get_mut(fahras) {
                *makan = khilat.rotate_left(1);
            }
        }

        let [mut alif, mut ba, mut jim, mut dal, mut ha] = self.halat;
        for fahras in 0..80usize {
            let (dala, thabit) = match fahras {
                0..=19 => ((ba & jim) | ((!ba) & dal), 0x5A82_7999u32),
                20..=39 => (ba ^ jim ^ dal, 0x6ED9_EBA1),
                40..=59 => ((ba & jim) | (ba & dal) | (jim & dal), 0x8F1B_BCDC),
                _ => (ba ^ jim ^ dal, 0xCA62_C1D6),
            };
            let waqti = alif
                .rotate_left(5)
                .wrapping_add(dala)
                .wrapping_add(ha)
                .wrapping_add(thabit)
                .wrapping_add(jadwal.get(fahras).copied().unwrap_or(0));
            ha = dal;
            dal = jim;
            jim = ba.rotate_left(30);
            ba = alif;
            alif = waqti;
        }

        let [awwal, thani, thalith, rabi, khamis] = self.halat;
        self.halat = [
            awwal.wrapping_add(alif),
            thani.wrapping_add(ba),
            thalith.wrapping_add(jim),
            rabi.wrapping_add(dal),
            khamis.wrapping_add(ha),
        ];
    }
}

/// The SHA-1 of one buffer.
#[must_use]
pub fn basma_bayt(bayt: &[u8]) -> [u8; HAJM_BASMA] {
    let mut basma = Basma::jadeeda();
    basma.hadhi(bayt);
    basma.anhi()
}

// ---------------------------------------------------------------------------
// Decryption
// ---------------------------------------------------------------------------

/// Decrypts a region in place with AES-256 in ECB mode.
///
/// **ECB, not CBC.** There is no initialisation vector and no chaining: each
/// sixteen-byte block is decrypted on its own. That is Unreal's choice, not
/// this module's, and it is why a container's encryption is a lock against
/// casual inspection rather than a cryptographic guarantee — identical
/// plaintext blocks produce identical ciphertext blocks, which is visible in a
/// hex dump of any pak with a run of zeroes in it.
///
/// A trailing partial block is left untouched rather than padded. Inventing
/// padding would turn a truncated read into plausible-looking plaintext, which
/// is the one outcome worse than a refusal.
///
/// # Errors
///
/// [`KhataUnreal::MiftahGhayrSalih`] when the key is not thirty-two bytes.
fn fukk_tashfeer(
    bayt: &mut [u8],
    miftah: &[u8; HAJM_MIFTAH],
    masar: &Path,
) -> Result<(), KhataUnreal> {
    let shifra = Aes256::new_from_slice(miftah).map_err(|_| KhataUnreal::MiftahGhayrSalih {
        masar: masar.to_path_buf(),
        sabab: "an AES-256 key must be exactly 32 bytes",
    })?;
    for kutla in bayt.chunks_exact_mut(HAJM_KUTLAT_TASHFEER) {
        shifra.decrypt_block(GenericArray::from_mut_slice(kutla));
    }
    Ok(())
}

/// Takes one region of a container, decrypting it when it is encrypted.
///
/// The two-step that this module's header describes, in one place: an encrypted
/// region occupies `tul` rounded up to sixteen bytes on disk, all of it is
/// decrypted, and the result is then **truncated back to `tul`**. A caller that
/// kept the padding would carry up to fifteen trailing bytes into a parser that
/// has no field for them.
///
/// # Errors
///
/// [`KhataUnreal::HajmMufrit`] when the declared length is above `saqf`,
/// [`KhataUnreal::MalafQaseer`] when the region is not inside the container, and
/// [`KhataUnreal::PakMushaffar`] when it is encrypted and no key was supplied.
fn mintaqa(
    bayt: &[u8],
    izaha: u64,
    tul: u64,
    haql: &'static str,
    saqf: u64,
    tashfeer: Tashfeer<'_>,
) -> Result<Vec<u8>, KhataUnreal> {
    if tul > saqf {
        return Err(mufrit(haql, tul, saqf));
    }
    if !tashfeer.mushaffara {
        return Ok(qass(bayt, izaha, tul, haql)?.to_vec());
    }
    let Some(miftah) = tashfeer.miftah else {
        return Err(KhataUnreal::PakMushaffar {
            masar: tashfeer.masar.to_path_buf(),
        });
    };
    let muhadhah =
        ila_ala(tul, tul_u64(HAJM_KUTLAT_TASHFEER)).ok_or_else(|| mufrit(haql, tul, saqf))?;
    let mut khana = qass(bayt, izaha, muhadhah, haql)?.to_vec();
    fukk_tashfeer(&mut khana, miftah, tashfeer.masar)?;
    khana.truncate(hajm_usize(tul).unwrap_or(usize::MAX));
    Ok(khana)
}

/// What one region needs to know about encryption, in one value.
///
/// Three things travel together everywhere in this module — is the region
/// encrypted, is there a key, and which container is being read — and passing
/// them as three parameters through five call layers is how one of them ends up
/// out of step with the other two.
#[derive(Debug, Clone, Copy)]
struct Tashfeer<'a> {
    /// The AES-256 key, when the caller supplied one.
    miftah: Option<&'a [u8; HAJM_MIFTAH]>,
    /// Whether the region this describes is encrypted.
    mushaffara: bool,
    /// The container, for a refusal a user can act on.
    masar: &'a Path,
}

// ---------------------------------------------------------------------------
// Decompression
// ---------------------------------------------------------------------------

/// Expands one block into a buffer of exactly the size it declared.
///
/// The bound is in two halves, and both are needed:
///
/// * the **declared** size is checked against its ceiling before the destination
///   exists, so a block claiming a gibibyte never reaches the allocator;
/// * the **produced** size is checked against the declaration afterwards, so a
///   frame that expands to less than it promised is refused rather than leaving
///   the tail of the destination as zeros the caller then reads as data.
///
/// Expanding into a fixed destination rather than a growing vector is what makes
/// the first half mean anything. A zlib or zstd stream is free to claim one size
/// in its header and expand to another; a fixed destination turns the
/// container's declaration into a ceiling instead of a suggestion.
///
/// # Errors
///
/// [`KhataUnreal::HajmMufrit`] when the declared size is above the ceiling,
/// [`KhataUnreal::DaghtMajhul`] for Oodle and for any method this build does not
/// implement, [`KhataUnreal::FakkFashil`] when the decompressor reports a
/// failure, and [`KhataUnreal::HajmGhayrMutabaq`] when the output does not match
/// the declaration.
fn fukk_daght(
    tareeqa: &TareeqatDaght,
    makhzun: &[u8],
    hajm_khaam: u64,
) -> Result<Vec<u8>, KhataUnreal> {
    let haql = "a compression block's expanded size";
    if hajm_khaam > AQSA_KUTLA_KHAAM {
        return Err(mufrit(haql, hajm_khaam, AQSA_KUTLA_KHAAM));
    }
    let siaa = hajm_usize(hajm_khaam).ok_or_else(|| mufrit(haql, hajm_khaam, AQSA_KUTLA_KHAAM))?;
    let ghayr_mutabaq = |fili: u64| KhataUnreal::HajmGhayrMutabaq {
        ism: ISM,
        muallan: hajm_khaam,
        fili,
    };
    let fashil = |tafsil: String| KhataUnreal::FakkFashil { ism: ISM, tafsil };

    match tareeqa {
        TareeqatDaght::Bila => {
            // A raw block still has to match its declaration. The stored length
            // and the expanded length are two independent fields, and a block
            // where they disagree is a block this reader and the engine would
            // slice differently.
            if makhzun.len() != siaa {
                return Err(ghayr_mutabaq(tul_u64(makhzun.len())));
            }
            Ok(makhzun.to_vec())
        },
        TareeqatDaght::Zlib => {
            let mut mafkuk = vec![0u8; siaa];
            let mut jihaz = flate2::Decompress::new(true);
            jihaz
                .decompress(makhzun, &mut mafkuk, flate2::FlushDecompress::Finish)
                .map_err(|khata| fashil(khata.to_string()))?;
            let fili = jihaz.total_out();
            if fili != hajm_khaam {
                return Err(ghayr_mutabaq(fili));
            }
            Ok(mafkuk)
        },
        TareeqatDaght::Gzip => {
            // `flate2::Decompress::new_gzip` exists only behind the zlib-C
            // backend, and this workspace builds the pure-Rust one. The reader
            // form is available on every backend, and `take` keeps the ceiling:
            // it will hand over at most one byte more than the block declared,
            // which is exactly enough to notice that it over-expanded and
            // nowhere near enough to matter if it does.
            let mut mafkuk = Vec::with_capacity(siaa);
            let hadd = hajm_khaam.saturating_add(1);
            let mut jihaz = flate2::read::GzDecoder::new(makhzun).take(hadd);
            jihaz
                .read_to_end(&mut mafkuk)
                .map_err(|khata| fashil(khata.to_string()))?;
            if mafkuk.len() != siaa {
                return Err(ghayr_mutabaq(tul_u64(mafkuk.len())));
            }
            Ok(mafkuk)
        },
        TareeqatDaght::Zstd => {
            let mut mafkuk = vec![0u8; siaa];
            let fili = zstd::bulk::decompress_to_buffer(makhzun, &mut mafkuk)
                .map_err(|khata| fashil(khata.to_string()))?;
            if fili != siaa {
                return Err(ghayr_mutabaq(tul_u64(fili)));
            }
            Ok(mafkuk)
        },
        TareeqatDaght::Oodle | TareeqatDaght::Majhula(_) => Err(KhataUnreal::DaghtMajhul {
            ism: ISM,
            naw: tareeqa.ism(),
        }),
    }
}

// ---------------------------------------------------------------------------
// The footer
// ---------------------------------------------------------------------------

/// Which optional fields a footer of `hajm` bytes carries.
///
/// Three independent yes-or-no answers and a count, as a named value rather
/// than as a tuple, because a tuple of three booleans is a thing a call site
/// destructures in the wrong order exactly once.
#[derive(Debug, Clone, Copy)]
struct ShaklTadhyeel {
    /// Whether sixteen bytes of `EncryptionKeyGuid` open the footer. Version 7
    /// and later.
    muarrif_miftah: bool,
    /// Whether the `bEncryptedIndex` byte sits between the GUID and the magic.
    /// Version 4 and later; versions 1 to 3 have neither the field nor the
    /// feature, and their footer is one byte shorter for it.
    alam_tashfeer: bool,
    /// Whether the `bIndexIsFrozen` byte follows the index hash. Version 9, and
    /// version 9 alone.
    alam_tajmid: bool,
    /// How many thirty-two byte compression-method name slots end the footer.
    khanat: usize,
}

impl ShaklTadhyeel {
    /// How far into the footer the magic sits: everything this shape writes in
    /// front of it, and nothing else.
    ///
    /// This is the number the search is built on, so it is derived from the
    /// shape rather than written beside it — a literal here and a field list
    /// there is how a footer comes to be read one byte off.
    const fn izahat_sihr(self) -> usize {
        let mut izaha: usize = 0;
        if self.muarrif_miftah {
            izaha = izaha.saturating_add(16);
        }
        if self.alam_tashfeer {
            izaha = izaha.saturating_add(1);
        }
        izaha
    }
}

/// The shape a footer of `hajm` bytes has.
///
/// Derived from the size rather than stored beside it, so that
/// [`AHJAM_TADHYEEL`] and this function cannot disagree: every entry in that
/// table satisfies `44 + 16*guid + 1*flag + 1*frozen + 32*slots == hajm`, and
/// this is the same equation solved the other way.
const fn shakl_tadhyeel(hajm: u64) -> ShaklTadhyeel {
    match hajm {
        44 => ShaklTadhyeel {
            muarrif_miftah: false,
            alam_tashfeer: false,
            alam_tajmid: false,
            khanat: 0,
        },
        45 => ShaklTadhyeel {
            muarrif_miftah: false,
            alam_tashfeer: true,
            alam_tajmid: false,
            khanat: 0,
        },
        61 => ShaklTadhyeel {
            muarrif_miftah: true,
            alam_tashfeer: true,
            alam_tajmid: false,
            khanat: 0,
        },
        189 => ShaklTadhyeel {
            muarrif_miftah: true,
            alam_tashfeer: true,
            alam_tajmid: false,
            khanat: KHANAT_DAGHT_AWWALIYA,
        },
        222 => ShaklTadhyeel {
            muarrif_miftah: true,
            alam_tashfeer: true,
            alam_tajmid: true,
            khanat: KHANAT_DAGHT_MUWASSAA,
        },
        // 221, and the arm a size outside the table falls into — which
        // [`AHJAM_TADHYEEL`] never produces, and which is the widest shape so
        // that a wrong guess reads past the field rather than short of it.
        _ => ShaklTadhyeel {
            muarrif_miftah: true,
            alam_tashfeer: true,
            alam_tajmid: false,
            khanat: KHANAT_DAGHT_MUWASSAA,
        },
    }
}

/// `FPakInfo`, read from the last bytes of the container.
///
/// Everything a reader needs before it touches the index, and nothing more: the
/// index is not read here, no entry is walked, and nothing is decrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TadhyeelPak {
    /// The container format version.
    pub isdar: IsdarPak,
    /// Where the index starts, already checked to lie inside the file.
    pub izahat_fahras: u64,
    /// How long the index is, already checked to fit inside the file.
    pub hajm_fahras: u64,
    /// The SHA-1 the writer recorded over the index. All zeroes when it did not
    /// record one; see [`Self::lahu_basma`].
    pub basmat_fahras: [u8; HAJM_BASMA],
    /// Whether the index is AES-encrypted. Always false below version 4, which
    /// wrote the byte and had no such feature.
    pub fahras_mushaffar: bool,
    /// The encryption key's GUID, sixteen bytes as the engine wrote them, when
    /// the container names one and it is not all zeroes. A zero GUID means the
    /// project's default key rather than a named one. Version 7 and later.
    pub muarrif_miftah: Option<[u8; 16]>,
    /// Whether version 9 marked its index as a frozen memory image. Refused by
    /// [`FahrasPak`]; see this module's header for why it is not parsed.
    pub mujammad: bool,
    /// The compression-method name table, empty below version 8.
    pub asma_daght: Vec<TareeqatDaght>,
    /// How many bytes the footer occupies, which is what identified its shape.
    pub hajm_tadhyeel: u64,
    /// The container's real length on disk.
    pub hajm_malaf: u64,
}

impl TadhyeelPak {
    /// Finds and reads the footer in the last bytes of a container.
    ///
    /// `dhayl` is the tail of the file — up to [`HAJM_DHAYL`] bytes ending at
    /// EOF — and `hajm_malaf` is the file's real length, which is what every
    /// offset in the footer is judged against.
    ///
    /// The search is described in this module's header. In short: the footer's
    /// size depends on the version, the version is inside the footer, so each
    /// known size is tried largest-first and the first one whose magic lands
    /// where that size puts it *and* whose version is legal for that size is the
    /// answer.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SihrGhayrMutabaq`] when no candidate holds the magic,
    /// [`KhataUnreal::IsdarGhayrMadum`] when one does but declares a version
    /// above [`AQSA_ISDAR`], [`KhataUnreal::MawridTalif`] when the version
    /// contradicts the footer size or the index does not lie inside the file,
    /// and [`KhataUnreal::MalafQaseer`] when the tail is shorter than the
    /// candidate it is being read as.
    pub fn min_dhayl(dhayl: &[u8], hajm_malaf: u64, masar: &Path) -> Result<Self, KhataUnreal> {
        for (hajm, nusakh) in AHJAM_TADHYEEL {
            let Some(khana) = Self::murashshah(dhayl, hajm) else {
                continue;
            };
            let Some(isdar) = Self::isdar_murashshah(khana, hajm) else {
                continue;
            };
            if !nusakh.contains(&isdar) {
                continue;
            }
            return Self::min_khana(khana, hajm, hajm_malaf);
        }

        // Nothing agreed. Before reporting "not a pak", look again for a magic
        // at any known size and report what it actually said, because "version
        // 12 is above the 11 this build reads" is a message a user can act on
        // and "this is not a pak" is one that would send them looking for a
        // corrupt download they do not have.
        for (hajm, _) in AHJAM_TADHYEEL {
            let Some(khana) = Self::murashshah(dhayl, hajm) else {
                continue;
            };
            let Some(isdar) = Self::isdar_murashshah(khana, hajm) else {
                continue;
            };
            if isdar > AQSA_ISDAR {
                return Err(KhataUnreal::IsdarGhayrMadum {
                    ism: ISM,
                    wujid: isdar,
                    aqsa: AQSA_ISDAR,
                });
            }
            return Err(talif(
                "the container version, which does not match the footer size it was found at",
                u64::from(isdar),
                u64::from(AQSA_ISDAR),
            ));
        }
        Err(KhataUnreal::SihrGhayrMutabaq {
            malaf: masar.to_path_buf(),
            ism: ISM,
        })
    }

    /// The last `hajm` bytes of the tail, if the magic is where a footer of that
    /// size would put it.
    fn murashshah(dhayl: &[u8], hajm: u64) -> Option<&[u8]> {
        let mada = hajm_usize(hajm)?;
        let bidaya = dhayl.len().checked_sub(mada)?;
        let khana = dhayl.get(bidaya..)?;
        let izaha = shakl_tadhyeel(hajm).izahat_sihr();
        let arba: [u8; 4] = khana.get(izaha..izaha.checked_add(4)?)?.try_into().ok()?;
        (u32::from_le_bytes(arba) == SIHR).then_some(khana)
    }

    /// The version a candidate declares, read from the four bytes after the
    /// magic.
    fn isdar_murashshah(khana: &[u8], hajm: u64) -> Option<u32> {
        let izaha = shakl_tadhyeel(hajm).izahat_sihr().checked_add(4)?;
        let arba: [u8; 4] = khana.get(izaha..izaha.checked_add(4)?)?.try_into().ok()?;
        Some(u32::from_le_bytes(arba))
    }

    /// Parses a candidate this reader has already committed to.
    fn min_khana(khana: &[u8], hajm: u64, hajm_malaf: u64) -> Result<Self, KhataUnreal> {
        let shakl = shakl_tadhyeel(hajm);
        let mut qari = Qari::jadeed(ISM, khana);
        let muarrif_miftah = if shakl.muarrif_miftah {
            let khaam: [u8; 16] = qari.iqra_masfufa("the footer's encryption key GUID")?;
            khaam.iter().any(|bayt| *bayt != 0).then_some(khaam)
        } else {
            None
        };
        // Versions 1 to 3 have no such byte, and the byte in front of their
        // magic belongs to the index. Reading it would be reading a field that
        // does not exist; treating its absence as "not encrypted" is what the
        // format itself says, since the feature arrived with the field.
        let alam = if shakl.alam_tashfeer {
            qari.iqra_u8("the footer's index encryption flag")?
        } else {
            0
        };
        let sihr = qari.iqra_u32("the footer's magic")?;
        if sihr != SIHR {
            return Err(talif(
                "the footer's magic",
                u64::from(sihr),
                u64::from(SIHR),
            ));
        }
        let isdar = IsdarPak::min_raqm(qari.iqra_u32("the footer's version")?)?;
        let izahat_fahras = tul_musir(qari.iqra_i64("the index offset")?, "the index offset")?;
        let hajm_fahras = tul_musir(qari.iqra_i64("the index size")?, "the index size")?;
        let basmat_fahras: [u8; HAJM_BASMA] = qari.iqra_masfufa("the index hash")?;
        // The footer size said which optional bytes are there; the version has
        // to agree, because the size is only ever a consequence of the version.
        // They cannot disagree by the time this runs — the search already
        // matched the size against the version — and the checks are here anyway,
        // because the cost of either being wrong is every field after it read at
        // the wrong offset, silently.
        if shakl.alam_tashfeer != isdar.tashfeer_fahras() {
            return Err(talif(
                "the encrypted-index flag, which no version below 4 writes",
                u64::from(isdar.raqm()),
                4,
            ));
        }
        if shakl.alam_tajmid != isdar.alam_tajmid() {
            return Err(talif(
                "the frozen-index flag, which only version 9 writes",
                u64::from(isdar.raqm()),
                9,
            ));
        }
        let mujammad = shakl.alam_tajmid && qari.iqra_u8("the frozen-index flag")? != 0;

        let mut asma_daght = Vec::with_capacity(shakl.khanat);
        for _ in 0..shakl.khanat {
            let slot = qari.iqra_bayt("a compression method name", tul_u64(HAJM_ISM_DAGHT))?;
            let ism: String = slot
                .iter()
                .take_while(|bayt| **bayt != 0)
                .map(|bayt| char::from(*bayt))
                .collect();
            asma_daght.push(TareeqatDaght::min_ism(&ism));
        }

        // The index has to be inside the file before anything is read from it.
        // A footer that says "index at 2^40" in a four megabyte pak is the exact
        // shape of a truncated download, and the refusal must name the field.
        let nihaya = izahat_fahras
            .checked_add(hajm_fahras)
            .ok_or_else(|| talif("the index size", hajm_fahras, hajm_malaf))?;
        if nihaya > hajm_malaf {
            return Err(talif("the end of the index", nihaya, hajm_malaf));
        }

        Ok(Self {
            isdar,
            izahat_fahras,
            hajm_fahras,
            basmat_fahras,
            fahras_mushaffar: isdar.tashfeer_fahras() && alam != 0,
            muarrif_miftah,
            mujammad,
            asma_daght,
            hajm_tadhyeel: hajm,
            hajm_malaf,
        })
    }

    /// Whether the container recorded a hash over its index.
    #[must_use]
    pub fn lahu_basma(&self) -> bool {
        self.basmat_fahras.iter().any(|bayt| *bayt != 0)
    }

    /// Whether the index cannot be read without a key the user supplies.
    #[must_use]
    pub const fn yahtaj_miftah(&self) -> bool {
        self.fahras_mushaffar
    }

    /// The key GUID as Unreal prints an `FGuid`: four 32-bit words, thirty-two
    /// uppercase hex digits, no separators.
    ///
    /// That is the form a user sees in their project's crypto settings, so it is
    /// the form Taarib asks them for and the form it prints back.
    #[must_use]
    pub fn muarrif_miftah_nass(&self) -> Option<String> {
        use std::fmt::Write as _;

        let khaam = self.muarrif_miftah?;
        let mut nass = String::with_capacity(32);
        for kalima in khaam.chunks_exact(4) {
            let arba: [u8; 4] = kalima.try_into().unwrap_or([0u8; 4]);
            let _ = write!(nass, "{:08X}", u32::from_le_bytes(arba));
        }
        Some(nass)
    }

    /// The engine version range this container's format implies.
    ///
    /// **Answered by Phase 5, not here.** The pak-version-to-engine-version
    /// table is inferred rather than published, and two copies of an inference
    /// eventually disagree — so this builds the detector's own footer value and
    /// asks it, which is also why `taarib-muharrik` is a dependency of this
    /// crate.
    #[must_use]
    pub fn mada(&self) -> MadaIsdar {
        DhaylPak {
            nuskha: self.isdar.raqm(),
            mawdi_fahras: self.izahat_fahras,
            hajm_fahras: self.hajm_fahras,
            mushaffar: self.fahras_mushaffar,
            muarrif_miftah: self.muarrif_miftah_nass(),
            khanat_dagt: self.asma_daght.len(),
            hajm_malaf: self.hajm_malaf,
            izahat_tawqi: self.hajm_tadhyeel,
            bil_mash: false,
        }
        .mada()
    }
}

// ---------------------------------------------------------------------------
// The index
// ---------------------------------------------------------------------------

/// A container's index, in whichever of the two shapes it was written.
///
/// Both shapes reduce to the same thing here: a mount point and a map from path
/// to entry. What differs is how much a container is willing to tell you. A flat
/// index always names every file. A version 10 or 11 index names them only if it
/// carries a full-directory index, which a shipping build may have pruned to
/// save memory — the engine can still resolve a path it already knows by hash,
/// but nothing can enumerate it. [`Self::dalil_kamil`] says which case this is,
/// so a caller that found no `.locres` can tell "this game has none" from "this
/// container will not say".
#[derive(Debug, Clone, Default)]
pub struct FahrasPak {
    nuqtat_wasl: String,
    madakhil: BTreeMap<String, MadkhalPak>,
    adad_muallan: u64,
    dalil_kamil: bool,
}

impl FahrasPak {
    /// Reads the index a footer points at, decrypting each region it names.
    ///
    /// `bayt` is the whole container. Every region is taken from it by offset
    /// and length, checked against its real size first, so a footer that points
    /// outside the file produces a refusal rather than a read.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::PakMushaffar`] when the index is encrypted and `miftah` is
    /// [`None`], [`KhataUnreal::MiftahGhayrSalih`] when a key was supplied and
    /// the result is not an index, [`KhataUnreal::BasmaGhayrMutabaqa`] when the
    /// recorded hash matches neither the stored nor the decrypted bytes,
    /// [`KhataUnreal::MalafQaseer`] when a region runs past the end,
    /// [`KhataUnreal::HajmMufrit`] for a size or count above its ceiling, and
    /// [`KhataUnreal::MawridTalif`] for a frozen index or a field inconsistent
    /// with the container.
    pub fn min_bayt(
        bayt: &[u8],
        tadhyeel: &TadhyeelPak,
        miftah: Option<&[u8; HAJM_MIFTAH]>,
        masar: &Path,
    ) -> Result<Self, KhataUnreal> {
        // Version 9's frozen index is a memory image, not a serialization. See
        // this module's header: reading it would mean reproducing one engine
        // build's container layout, and a subtly wrong reproduction produces
        // entries rather than an error.
        if tadhyeel.mujammad {
            return Err(talif(
                "the index, which this container froze as a memory image rather than serializing",
                1,
                0,
            ));
        }

        let tashfeer = Tashfeer {
            miftah,
            mushaffara: tadhyeel.fahras_mushaffar,
            masar,
        };
        let khana = mintaqa(
            bayt,
            tadhyeel.izahat_fahras,
            tadhyeel.hajm_fahras,
            "the index",
            AQSA_HAJM_FAHRAS,
            tashfeer,
        )?;
        if tadhyeel.fahras_mushaffar {
            tahaqquq_miftah(&khana, masar)?;
        }
        tahaqquq_basma(
            bayt,
            tadhyeel.izahat_fahras,
            tadhyeel.hajm_fahras,
            &khana,
            tadhyeel.lahu_basma().then_some(tadhyeel.basmat_fahras),
            "the index",
        )?;

        if tadhyeel.isdar.fahras_basmat_masar() {
            Self::min_fahras_masarat(bayt, &khana, tadhyeel, tashfeer)
        } else {
            Self::min_fahras_musattah(&khana, tadhyeel)
        }
    }

    /// The flat index of versions 1 through 9.
    ///
    /// ```text
    /// FahrasPak — the flat shape, little-endian
    ///
    ///   FString  nuqtat_wasl   the mount point every path is relative to
    ///   i32      adad          how many records follow
    ///   repeat adad:
    ///     FString      masar   the path, relative to the mount point
    ///     FPakEntry    madkhal the full entry, index copy
    /// ```
    fn min_fahras_musattah(khana: &[u8], tadhyeel: &TadhyeelPak) -> Result<Self, KhataUnreal> {
        let mut qari = Qari::jadeed(ISM, khana);
        let nuqtat_wasl = qari.iqra_nass("the index mount point")?;
        let haql = "the index entry count";
        let adad = adad_musir(ISM, haql, qari.iqra_i32(haql)?)?;
        let mahjuz = tahaqquq_adad(ISM, haql, adad, AQSA_MADAKHIL, AQALL_MADKHAL, qari.baqi())?;

        let mut madakhil = BTreeMap::new();
        for _ in 0..mahjuz {
            let masar = qari.iqra_nass("an index entry's path")?;
            let madkhal =
                MadkhalPak::min_qari(&mut qari, tadhyeel.isdar, &tadhyeel.asma_daght, None)?;
            let _ = madakhil.insert(masar_muwahhad(masar.nass()), madkhal);
        }

        Ok(Self {
            nuqtat_wasl: nuqtat_wasl.nass().to_owned(),
            madakhil,
            adad_muallan: adad,
            dalil_kamil: true,
        })
    }

    /// The path-hash index of versions 10 and 11.
    ///
    /// ```text
    /// the primary index — little-endian
    ///
    ///   FString      nuqtat_wasl    the mount point
    ///   i32          adad           how many entries the container holds
    ///   u64          badhra         the seed the path hash was salted with
    ///   i32          present        whether a path-hash index follows
    ///   i64 i64 [20] izaha tul basma  where it is, when present
    ///   i32          present        whether a full-directory index follows
    ///   i64 i64 [20] izaha tul basma  where it is, when present
    ///   i32 + bytes  murammaz       the encoded-entry blob
    ///   i32 + n      kamila         the entries that would not fit the encoding
    /// ```
    ///
    /// The two secondary regions live outside the primary index, at their own
    /// offsets, and each is **separately encrypted with the same key**. A reader
    /// that decrypted the primary index and then read the other two where they
    /// sit gets two regions of noise and no error at all, because nothing in
    /// them has a magic. That is why each goes back through [`mintaqa`] rather
    /// than being sliced out of an already-decrypted buffer.
    fn min_fahras_masarat(
        bayt: &[u8],
        khana: &[u8],
        tadhyeel: &TadhyeelPak,
        tashfeer: Tashfeer<'_>,
    ) -> Result<Self, KhataUnreal> {
        let mut qari = Qari::jadeed(ISM, khana);
        let nuqtat_wasl = qari.iqra_nass("the index mount point")?;
        let haql = "the index entry count";
        let adad = adad_musir(ISM, haql, qari.iqra_i32(haql)?)?;
        if adad > AQSA_MADAKHIL {
            return Err(mufrit(haql, adad, AQSA_MADAKHIL));
        }
        // The seed salts the path hash. This reader resolves through names and
        // never recomputes a hash, so it is read to advance the cursor and to
        // prove the field was there, and then deliberately dropped.
        let _ = qari.iqra_i64("the path hash seed")?;

        let mawqi_basmat = mintaqa_thanawiya(&mut qari, "the path-hash index")?;
        let mawqi_dalil = mintaqa_thanawiya(&mut qari, "the full-directory index")?;

        let haql_ramz = "the encoded entry blob";
        let tul_ramz = adad_musir(ISM, haql_ramz, qari.iqra_i32(haql_ramz)?)?;
        let _ = tahaqquq_adad(ISM, haql_ramz, tul_ramz, AQSA_HAJM_FAHRAS, 1, qari.baqi())?;
        let murammaz = qari.iqra_bayt(haql_ramz, tul_ramz)?.to_vec();

        let haql_kamil = "the index's unencoded entry count";
        let adad_kamil = adad_musir(ISM, haql_kamil, qari.iqra_i32(haql_kamil)?)?;
        let adad_kamil =
            tahaqquq_adad(ISM, haql_kamil, adad_kamil, AQSA_MADAKHIL, 44, qari.baqi())?;
        let mut kamila = Vec::with_capacity(adad_kamil);
        for _ in 0..adad_kamil {
            kamila.push(MadkhalPak::min_qari(
                &mut qari,
                tadhyeel.isdar,
                &tadhyeel.asma_daght,
                None,
            )?);
        }

        // Read and check the path-hash index even though nothing here resolves
        // through it. It is the region a container is most likely to still have
        // when the directory index was pruned, so a container whose path-hash
        // index is corrupt is a container this reader should refuse rather than
        // report as merely unnameable.
        if let Some((izaha, tul, basma)) = mawqi_basmat {
            let haql = "the path-hash index";
            let mafkuk = mintaqa(bayt, izaha, tul, haql, AQSA_HAJM_FAHRAS, tashfeer)?;
            tahaqquq_basma(bayt, izaha, tul, &mafkuk, basma, haql)?;
            let mut basmat = Qari::jadeed(ISM, &mafkuk);
            let haql = "the path-hash index's pair count";
            let adad = adad_musir(ISM, haql, basmat.iqra_i32(haql)?)?;
            let _ = tahaqquq_adad(ISM, haql, adad, AQSA_MADAKHIL, 12, basmat.baqi())?;
        }

        let (madakhil, dalil_kamil) = if let Some((izaha, tul, basma)) = mawqi_dalil {
            let haql = "the full-directory index";
            let mafkuk = mintaqa(bayt, izaha, tul, haql, AQSA_HAJM_FAHRAS, tashfeer)?;
            tahaqquq_basma(bayt, izaha, tul, &mafkuk, basma, haql)?;
            (
                Self::min_dalil_kamil(&mafkuk, &murammaz, &kamila, tadhyeel)?,
                true,
            )
        } else {
            (BTreeMap::new(), false)
        };

        Ok(Self {
            nuqtat_wasl: nuqtat_wasl.nass().to_owned(),
            madakhil,
            adad_muallan: adad,
            dalil_kamil,
        })
    }

    /// The full-directory index: directories, their files, and where each
    /// file's entry lives.
    ///
    /// ```text
    /// the full-directory index — little-endian
    ///
    ///   i32                  how many directories follow
    ///   repeat:
    ///     FString  dalil     the directory, with its trailing separator
    ///     i32                how many files in it
    ///     repeat:
    ///       FString  ism     the file name alone
    ///       i32      mawqi   where its entry is; see MAWQI_GHAYR_SALIH
    /// ```
    fn min_dalil_kamil(
        khana: &[u8],
        murammaz: &[u8],
        kamila: &[MadkhalPak],
        tadhyeel: &TadhyeelPak,
    ) -> Result<BTreeMap<String, MadkhalPak>, KhataUnreal> {
        let mut qari = Qari::jadeed(ISM, khana);
        let haql = "the full-directory index's directory count";
        let adad = adad_musir(ISM, haql, qari.iqra_i32(haql)?)?;
        let adad = tahaqquq_adad(ISM, haql, adad, AQSA_DALAIL, 8, qari.baqi())?;

        let mut madakhil = BTreeMap::new();
        for _ in 0..adad {
            let dalil = qari.iqra_nass("a directory's name")?;
            let haql_malafat = "a directory's file count";
            let adad_malafat = adad_musir(ISM, haql_malafat, qari.iqra_i32(haql_malafat)?)?;
            let adad_malafat = tahaqquq_adad(
                ISM,
                haql_malafat,
                adad_malafat,
                AQSA_MADAKHIL,
                8,
                qari.baqi(),
            )?;
            for _ in 0..adad_malafat {
                let ism = qari.iqra_nass("a file's name")?;
                let mawqi = qari.iqra_i32("a file's entry location")?;
                let Some(madkhal) = hall_mawqi(mawqi, murammaz, kamila, tadhyeel)? else {
                    continue;
                };
                let masar = masar_muwahhad(&format!("{}{}", dalil.nass(), ism.nass()));
                if tul_u64(masar.len()) > AQSA_TUL_MASAR {
                    return Err(mufrit(
                        "an entry's path",
                        tul_u64(masar.len()),
                        AQSA_TUL_MASAR,
                    ));
                }
                let _ = madakhil.insert(masar, madkhal);
            }
        }
        Ok(madakhil)
    }

    /// The mount point every path in this index is relative to.
    ///
    /// Typically `../../../`, which is Unreal's way of saying "the directory
    /// above `Engine` and the project" — the paths inside are then
    /// `<Project>/Content/...`. It is carried rather than folded into the keys
    /// because a patch container has to be written with the *same* mount point
    /// as the container it overrides, or the engine resolves its paths somewhere
    /// else and the override silently does nothing.
    #[must_use]
    pub fn nuqtat_wasl(&self) -> &str {
        &self.nuqtat_wasl
    }

    /// How many entries the container declared, which is not always how many
    /// this index can name — see [`Self::dalil_kamil`].
    #[must_use]
    pub const fn adad_muallan(&self) -> u64 {
        self.adad_muallan
    }

    /// Whether every declared entry could be named.
    ///
    /// False only for a version 10 or 11 container whose full-directory index
    /// was pruned at cook time.
    #[must_use]
    pub const fn dalil_kamil(&self) -> bool {
        self.dalil_kamil
    }

    /// Every path this index can name, with its entry, in sorted order.
    pub fn madakhil(&self) -> impl Iterator<Item = (&str, &MadkhalPak)> + '_ {
        self.madakhil
            .iter()
            .map(|(masar, madkhal)| (masar.as_str(), madkhal))
    }

    /// How many entries this index can name.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// Finds one entry by path.
    ///
    /// Exact first, then case-insensitively over normalized separators. The
    /// second pass exists because a pak cooked on Windows may hold backslashes
    /// and a caller composing a path from Unreal's own conventions will not, and
    /// a lookup that failed over a separator would report a game as having no
    /// localization at all.
    #[must_use]
    pub fn jid(&self, masar: &str) -> Option<&MadkhalPak> {
        let matlub = masar_muwahhad(masar);
        if let Some(madkhal) = self.madakhil.get(&matlub) {
            return Some(madkhal);
        }
        self.madakhil
            .iter()
            .find(|(mawjud, _)| mawjud.eq_ignore_ascii_case(&matlub))
            .map(|(_, madkhal)| madkhal)
    }
}

/// One path, with separators normalized and any leading slash removed.
///
/// Not lowercased: the keys keep the casing the container used, because that is
/// what a diagnostic has to print for a user to find the file on disk. Case
/// folding happens only in [`FahrasPak::jid`]'s second pass.
fn masar_muwahhad(masar: &str) -> String {
    masar.replace('\\', "/").trim_start_matches('/').to_owned()
}

/// Decides whether a decrypted region is an index at all.
///
/// **This is the only test for "is this the right key" that AES-256 in ECB
/// permits.** There is no authentication tag, so every thirty-two byte key
/// decrypts every ciphertext into something; the question is whether the
/// something is structured. Three cheap facts settle it: the region must open
/// with a well-formed `FString` — a length whose sign and magnitude are
/// plausible and whose last unit is the NUL the format includes in the count —
/// the entry count after it must be non-negative, and it must be below the
/// ceiling this build will reserve for.
///
/// A wrong key produces sixteen bytes of uniformly distributed noise, which
/// fails the first of those with probability very close to one. A right key
/// passes all three always. It is a probabilistic test, and it is stated as one
/// here rather than presented as a proof the format cannot supply.
///
/// # Errors
///
/// [`KhataUnreal::MiftahGhayrSalih`] naming which of the three failed.
fn tahaqquq_miftah(khana: &[u8], masar: &Path) -> Result<(), KhataUnreal> {
    let ghayr = |sabab: &'static str| KhataUnreal::MiftahGhayrSalih {
        masar: masar.to_path_buf(),
        sabab,
    };
    let mut qari = Qari::jadeed(ISM, khana);
    let wasl = qari
        .iqra_nass("the index mount point")
        .map_err(|_| ghayr("the decrypted index does not begin with a well-formed mount point"))?;
    if tul_u64(wasl.nass().len()) > AQSA_TUL_MASAR {
        return Err(ghayr(
            "the decrypted index's mount point is implausibly long",
        ));
    }
    let adad = qari
        .iqra_i32("the index entry count")
        .map_err(|_| ghayr("the decrypted index ends before its entry count"))?;
    if adad < 0 {
        return Err(ghayr("the decrypted index declares a negative entry count"));
    }
    if u64::from(adad.unsigned_abs()) > AQSA_MADAKHIL {
        return Err(ghayr(
            "the decrypted index declares more entries than any game ships",
        ));
    }
    Ok(())
}

/// Checks a region against the SHA-1 the container recorded for it.
///
/// Accepts a hash that matches the region **as decrypted** or **as stored**, for
/// the reason in this module's header: engine versions disagree about which of
/// the two the writer hashed, the disagreement is invisible on unencrypted paks,
/// and a reader that insisted on one reading would tell a large fraction of
/// users with correct keys that their key was wrong.
///
/// The stored form is tried at both the declared length and the length rounded
/// up to the AES block, because a writer that hashed what it wrote hashed the
/// padding too.
///
/// # Errors
///
/// [`KhataUnreal::BasmaGhayrMutabaqa`] naming the region when no reading
/// matches, which means the bytes changed after the hash was recorded.
fn tahaqquq_basma(
    bayt: &[u8],
    izaha: u64,
    tul: u64,
    mafkuk: &[u8],
    musajjala: Option<[u8; HAJM_BASMA]>,
    madkhal: &'static str,
) -> Result<(), KhataUnreal> {
    let Some(musajjala) = musajjala else {
        return Ok(());
    };
    if basma_bayt(mafkuk) == musajjala {
        return Ok(());
    }
    let muhadhah = ila_ala(tul, tul_u64(HAJM_KUTLAT_TASHFEER)).unwrap_or(tul);
    for tul_makhzun in [tul, muhadhah] {
        if let Ok(khana) = qass(bayt, izaha, tul_makhzun, madkhal)
            && basma_bayt(khana) == musajjala
        {
            return Ok(());
        }
    }
    Err(KhataUnreal::BasmaGhayrMutabaqa {
        ism: ISM,
        madkhal: madkhal.to_owned(),
    })
}

/// One entry decoded from a version 10 or 11 encoded-entry blob.
///
/// The blob packs an entry into as few bytes as it can, and the packing is a
/// single `u32` of flags followed by only the fields those flags say are
/// present:
///
/// ```text
/// alam — u32, little-endian
///
///   bit 31      the offset that follows is 32-bit, not 64
///   bit 30      the expanded size that follows is 32-bit, not 64
///   bit 29      the stored size that follows is 32-bit, not 64
///   bits 28-23  the compression method index, 0 for stored raw
///   bit 22      the payload is encrypted
///   bits 21-6   how many compression blocks follow, as u32 sizes
///   bits 5-0    the compression block size, in units of 2048 bytes
/// ```
///
/// Two shapes of block list come out of it, and they differ only in what is
/// stored — never in where the first block begins:
///
/// * exactly one block and not encrypted — no sizes are stored at all, and the
///   single block spans from the entry's payload start for its stored size;
/// * anything else — one `u32` size per block and nothing else, so the offsets
///   are accumulated from the same payload start, aligned up to sixteen between
///   blocks when the entry is encrypted.
///
/// **The payload start is the base for both**, and it is the entry's offset plus
/// the serialized length of the entry header stored at that offset — this is
/// `Entry.Offset + Entry.GetSerializedSize(Version)` in the engine's own
/// `DecodePakEntry`. Never the entry's offset on its own: that addresses the
/// header, whose first bytes are a length field and not a compressed stream.
/// The header's length is a function of the version and of the entry's own
/// shape and runs from 73 to 457 bytes in a shipping container, so it is
/// computed by [`tul_musalsal_li`] every time and never assumed.
///
/// This base is *not* the one the flat index uses. There each block offset is
/// written out relative to the entry header and therefore already carries the
/// header's length, so [`kutal_min_qari`] adds the entry offset and stops. Here
/// nothing but block lengths is stored, so the header's length has to be added
/// back. Getting it wrong does not fail: it reads the header and the first bytes
/// of the payload as a compressed stream, which refuses with
/// [`KhataUnreal::FakkFashil`] — a message that tells a user with an intact game
/// that their container is damaged.
///
/// Both shapes are resolved to absolute offsets here, so a caller never meets
/// the difference.
///
/// # Errors
///
/// [`KhataUnreal::MalafQaseer`] when the blob ends inside the record,
/// [`KhataUnreal::MawridTalif`] for a negative size or an offset outside the
/// container, and [`KhataUnreal::HajmMufrit`] for a size above its ceiling.
fn madkhal_min_murammaz(
    murammaz: &[u8],
    mawqi: u64,
    isdar: IsdarPak,
    jadwal: &[TareeqatDaght],
) -> Result<MadkhalPak, KhataUnreal> {
    let mut qari = Qari::jadeed(ISM, murammaz);
    qari.iqfiz("an encoded entry's location", mawqi)?;
    let alam = qari.iqra_u32("an encoded entry's flags")?;

    let fahras_daght = (alam >> 23) & 0x3F;
    let izaha = qeema_murammaza(
        &mut qari,
        alam & (1 << 31) != 0,
        "an encoded entry's offset",
    )?;
    let hajm_khaam = qeema_murammaza(
        &mut qari,
        alam & (1 << 30) != 0,
        "an encoded entry's expanded size",
    )?;
    if hajm_khaam > AQSA_HAJM_KHAAM {
        return Err(mufrit(
            "an encoded entry's expanded size",
            hajm_khaam,
            AQSA_HAJM_KHAAM,
        ));
    }
    let hajm_makhzun = if fahras_daght == 0 {
        hajm_khaam
    } else {
        qeema_murammaza(
            &mut qari,
            alam & (1 << 29) != 0,
            "an encoded entry's stored size",
        )?
    };
    let mushaffar = alam & (1 << 22) != 0;
    let adad_kutal = u64::from((alam >> 6) & 0xFFFF);
    let daght = tareeqa_min_jadwal(fahras_daght, jadwal);
    let hajm_kutla = if adad_kutal == 0 {
        0
    } else if hajm_khaam < 65536 {
        u32::try_from(hajm_khaam).unwrap_or(u32::MAX)
    } else {
        (alam & 0x3F) << 11
    };

    let bidayat_bayanat = izaha
        .checked_add(tul_musalsal_li(isdar, fahras_daght != 0, adad_kutal))
        .ok_or_else(|| talif("an encoded entry's offset", izaha, u64::MAX))?;
    let kutal = if adad_kutal == 1 && !mushaffar {
        let nihaya = bidayat_bayanat
            .checked_add(hajm_makhzun)
            .ok_or_else(|| talif("an encoded entry's stored size", hajm_makhzun, u64::MAX))?;
        vec![KutlatDaght {
            bidaya: bidayat_bayanat,
            nihaya,
        }]
    } else {
        kutal_murammaza(&mut qari, adad_kutal, bidayat_bayanat, mushaffar)?
    };

    Ok(MadkhalPak {
        izaha,
        hajm_makhzun,
        hajm_khaam,
        daght,
        fahras_daght,
        // An encoded entry has no room for a hash, so there is none to check.
        // Saying so with zeroes is the same convention the flat index uses for a
        // writer that did not record one.
        basma: [0u8; HAJM_BASMA],
        kutal,
        mushaffar,
        hajm_kutla,
        mahdhuf: false,
        isdar,
    })
}

/// One value from an encoded entry, four bytes wide or eight as the flags say.
fn qeema_murammaza(
    qari: &mut Qari<'_>,
    daiq: bool,
    haql: &'static str,
) -> Result<u64, KhataUnreal> {
    if daiq {
        Ok(u64::from(qari.iqra_u32(haql)?))
    } else {
        tul_musir(qari.iqra_i64(haql)?, haql)
    }
}

/// The accumulated block list of an encoded entry, resolved to absolute offsets.
///
/// `bidayat_bayanat` is the entry's **payload start** — its offset plus the
/// serialized length of the header stored there — and never the entry's offset
/// itself. See [`madkhal_min_murammaz`] for why the two are different bases and
/// what using the wrong one produces.
///
/// The running position stays relative and the base is added per block rather
/// than the base being the accumulator's starting value, because the alignment
/// below is Unreal's `Align(BlockSize, AESBlockSize)` on the block's *length*.
/// Accumulating from zero keeps the running position a multiple of sixteen, so
/// aligning the position and aligning each length are the same arithmetic; seed
/// it with a base that is not itself aligned and they stop being.
fn kutal_murammaza(
    qari: &mut Qari<'_>,
    adad: u64,
    bidayat_bayanat: u64,
    mushaffar: bool,
) -> Result<Vec<KutlatDaght>, KhataUnreal> {
    let haql = "an encoded entry's compression block count";
    let adad = tahaqquq_adad(ISM, haql, adad, AQSA_KUTAL, 4, qari.baqi())?;
    let mut kutal = Vec::with_capacity(adad);
    let mut mawqi: u64 = 0;
    for _ in 0..adad {
        let tul = u64::from(qari.iqra_u32("an encoded compression block's size")?);
        let nihaya = mawqi
            .checked_add(tul)
            .ok_or_else(|| talif("an encoded compression block's size", tul, u64::MAX))?;
        let bidaya = mawqi
            .checked_add(bidayat_bayanat)
            .ok_or_else(|| talif("an encoded compression block's start", mawqi, u64::MAX))?;
        let mutlaqa = nihaya
            .checked_add(bidayat_bayanat)
            .ok_or_else(|| talif("an encoded compression block's end", nihaya, u64::MAX))?;
        kutal.push(KutlatDaght {
            bidaya,
            nihaya: mutlaqa,
        });
        // Encrypted blocks are each padded to the AES block, so the next one
        // starts on a sixteen-byte boundary. A reader that packed them tightly
        // would be up to fifteen bytes early from the second block onward.
        mawqi = if mushaffar {
            ila_ala(nihaya, tul_u64(HAJM_KUTLAT_TASHFEER))
                .ok_or_else(|| talif("an encoded compression block's end", nihaya, u64::MAX))?
        } else {
            nihaya
        };
    }
    Ok(kutal)
}

/// Reads the "is there one, and where" triple a primary index writes for each
/// of its two secondary regions.
///
/// Returns [`None`] when the container says the region is absent, which is a
/// legitimate state and not a defect: a cook may prune the full-directory index,
/// and a container with no compression has nothing to say in either.
fn mintaqa_thanawiya(
    qari: &mut Qari<'_>,
    haql: &'static str,
) -> Result<Option<(u64, u64, Option<[u8; HAJM_BASMA]>)>, KhataUnreal> {
    // Serialized as an `int32`, because that is how `FArchive` writes a `bool`.
    // Reading it as one byte would leave three bytes in the stream and shift
    // every field after it.
    if qari.iqra_i32(haql)? == 0 {
        return Ok(None);
    }
    let izaha = tul_musir(qari.iqra_i64(haql)?, haql)?;
    let tul = tul_musir(qari.iqra_i64(haql)?, haql)?;
    let basma: [u8; HAJM_BASMA] = qari.iqra_masfufa(haql)?;
    let basma = basma.iter().any(|bayt| *bayt != 0).then_some(basma);
    Ok(Some((izaha, tul, basma)))
}

/// Resolves one `FPakEntryLocation` to the entry it names.
///
/// Three readings, and the zero case is the one worth naming: a location of zero
/// is a perfectly good byte offset into the encoded blob and means the *first*
/// entry, so [`MAWQI_GHAYR_SALIH`] is `i32::MIN` rather than zero and a reader
/// that used zero as its sentinel would drop a file.
///
/// # Errors
///
/// [`KhataUnreal::MawridTalif`] when a negative location names an entry past the
/// end of the unencoded array, and whatever
/// [`madkhal_min_murammaz`] raises for a blob offset that is not a record.
fn hall_mawqi(
    mawqi: i32,
    murammaz: &[u8],
    kamila: &[MadkhalPak],
    tadhyeel: &TadhyeelPak,
) -> Result<Option<MadkhalPak>, KhataUnreal> {
    if mawqi == MAWQI_GHAYR_SALIH {
        return Ok(None);
    }
    if mawqi >= 0 {
        let izaha = u64::try_from(mawqi).unwrap_or(0);
        let madkhal = madkhal_min_murammaz(murammaz, izaha, tadhyeel.isdar, &tadhyeel.asma_daght)?;
        return Ok(Some(madkhal));
    }
    // The encoding is -(n + 1), so n is -(location + 1). `i32::MIN` is already
    // out of the way above, which is what makes both operations safe.
    let fahras = mawqi.saturating_add(1).saturating_neg();
    let hadd = tul_u64(kamila.len());
    let fahras = usize::try_from(fahras).map_err(|_| {
        talif(
            "a file's entry location",
            u64::from(mawqi.unsigned_abs()),
            hadd,
        )
    })?;
    kamila
        .get(fahras)
        .cloned()
        .map(Some)
        .ok_or_else(|| talif("a file's entry location", tul_u64(fahras), hadd))
}

// ---------------------------------------------------------------------------
// The container
// ---------------------------------------------------------------------------

/// Where a container's bytes come from.
///
/// Two forms so that the reader is one implementation rather than two. A pak on
/// disk is mapped, because a shipping container routinely passes fifty
/// gigabytes and this reader touches a few kilobytes of it; a pak already in
/// memory is used as it is, which is what a test does and what a caller that
/// obtained the bytes some other way passes.
#[derive(Debug)]
enum MasdarPak {
    /// A memory-mapped file.
    ///
    /// A mapping is only as stable as the file behind it: if another process
    /// truncates the pak while this value is alive, touching the mapped pages
    /// past the new end is undefined behaviour, and no bounds check in this
    /// module can prevent it. Taarib reads a game's own installed files and
    /// never writes to them, and this is safe under that discipline. It is not
    /// safe against an adversary who can rewrite the game directory while the
    /// extractor runs — who, having write access to the game directory, has
    /// already won.
    Khareeta(memmap2::Mmap),
    /// Bytes already held.
    Dhakira(Vec<u8>),
}

impl MasdarPak {
    /// The container's bytes.
    fn bayt(&self) -> &[u8] {
        match self {
            Self::Khareeta(khareeta) => khareeta,
            Self::Dhakira(bayt) => bayt,
        }
    }
}

/// A parsed `.pak`, with its index resolved and its payloads still on disk.
///
/// Construction reads the footer and the whole index and nothing else. A
/// payload is read, decrypted and expanded only when it is asked for, which is
/// what makes opening a fifty gigabyte container cost the same as opening a four
/// megabyte one.
#[derive(Debug)]
pub struct HawiyatPak {
    masar: PathBuf,
    masdar: MasdarPak,
    tadhyeel: TadhyeelPak,
    fahras: FahrasPak,
    miftah: Option<[u8; HAJM_MIFTAH]>,
}

impl HawiyatPak {
    /// Opens and maps a container on disk, then reads its footer and index.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] when the file cannot be opened or mapped, and
    /// everything [`FahrasPak::min_bayt`] and [`TadhyeelPak::min_dhayl`] raise.
    pub fn iftah(masar: &Path, miftah: Option<[u8; HAJM_MIFTAH]>) -> Result<Self, KhataUnreal> {
        let malaf = std::fs::File::open(masar).map_err(|sabab| KhataUnreal::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })?;
        // SAFETY: see `MasdarPak::Khareeta`. The mapping is read-only and the
        // file is one this product opens and never writes.
        let khareeta =
            unsafe { memmap2::Mmap::map(&malaf) }.map_err(|sabab| KhataUnreal::KhataMalaf {
                masar: masar.to_path_buf(),
                sabab,
            })?;
        Self::jadeed(MasdarPak::Khareeta(khareeta), masar, miftah)
    }

    /// Reads a container already held in memory.
    ///
    /// # Errors
    ///
    /// Everything [`FahrasPak::min_bayt`] and [`TadhyeelPak::min_dhayl`] raise.
    /// `masar` is used only to name the container in a refusal, so a caller with
    /// no path may pass any name it would like the user to see.
    pub fn min_bayt(
        bayt: Vec<u8>,
        masar: &Path,
        miftah: Option<[u8; HAJM_MIFTAH]>,
    ) -> Result<Self, KhataUnreal> {
        Self::jadeed(MasdarPak::Dhakira(bayt), masar, miftah)
    }

    /// The one construction path, whichever source the bytes came from.
    fn jadeed(
        masdar: MasdarPak,
        masar: &Path,
        miftah: Option<[u8; HAJM_MIFTAH]>,
    ) -> Result<Self, KhataUnreal> {
        let bayt = masdar.bayt();
        let hajm_malaf = tul_u64(bayt.len());
        let bidaya = bayt.len().saturating_sub(HAJM_DHAYL);
        let dhayl = bayt
            .get(bidaya..)
            .ok_or_else(|| qaseer("the footer", hajm_malaf, tul_u64(HAJM_DHAYL)))?;
        let tadhyeel = TadhyeelPak::min_dhayl(dhayl, hajm_malaf, masar)?;
        // An encrypted index with no key is the one refusal in this module the
        // user can resolve, and it is raised before anything is read rather than
        // after a decryption that produced noise.
        if tadhyeel.yahtaj_miftah() && miftah.is_none() {
            return Err(KhataUnreal::PakMushaffar {
                masar: masar.to_path_buf(),
            });
        }
        let fahras = FahrasPak::min_bayt(bayt, &tadhyeel, miftah.as_ref(), masar)?;
        Ok(Self {
            masar: masar.to_path_buf(),
            masdar,
            tadhyeel,
            fahras,
            miftah,
        })
    }

    /// The container this was read from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// The footer.
    #[must_use]
    pub const fn tadhyeel(&self) -> &TadhyeelPak {
        &self.tadhyeel
    }

    /// The index.
    #[must_use]
    pub const fn fahras(&self) -> &FahrasPak {
        &self.fahras
    }

    /// How many entries this container can name.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.fahras.adad()
    }

    /// Every path this container can name, in sorted order.
    pub fn masarat(&self) -> impl Iterator<Item = &str> + '_ {
        self.fahras.madakhil().map(|(masar, _)| masar)
    }

    /// Finds one entry by path.
    #[must_use]
    pub fn jid(&self, masar: &str) -> Option<&MadkhalPak> {
        self.fahras.jid(masar)
    }

    /// Every `.locres` in the container.
    ///
    /// The reason this module exists: a game's compiled translations, one file
    /// per culture, which Phase 8 reads to learn what the game currently says
    /// and writes back beside as Arabic.
    pub fn masarat_locres(&self) -> impl Iterator<Item = &str> + '_ {
        self.masarat().filter(|masar| lahiqa_hiya(masar, ".locres"))
    }

    /// Every `.locmeta` in the container, which names the native culture and
    /// lists the compiled ones.
    pub fn masarat_locmeta(&self) -> impl Iterator<Item = &str> + '_ {
        self.masarat()
            .filter(|masar| lahiqa_hiya(masar, ".locmeta"))
    }

    /// Reads one file out of the container, by path.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] when the index does not name the path — this
    /// module has no "not found" of its own, and an absent path in a container
    /// somebody said held it is a claim about the container that turned out to
    /// be false — plus everything [`Self::iqra_madkhal`] raises.
    pub fn iqra_masar(&self, masar: &str) -> Result<Vec<u8>, KhataUnreal> {
        let madkhal = self.jid(masar).ok_or_else(|| {
            talif(
                "a path this container's index does not name",
                0,
                tul_u64(self.adad()),
            )
        })?;
        self.iqra_madkhal(madkhal, masar)
    }

    /// Reads one entry's payload: located, decrypted, expanded and checked.
    ///
    /// `ism` names the entry in any refusal, and is the path a user would
    /// recognise rather than an index into a table they cannot see.
    ///
    /// The order is deliberate. The **second copy of the entry header** is read
    /// and compared against the index copy before a byte of payload is touched,
    /// because that comparison is the one cheap test that catches the offset-base
    /// mistake this module's header describes: if the payload started somewhere
    /// other than where this reader thinks, the header it finds at the entry's
    /// own offset will not describe the same file, and the block bounds it
    /// states outright will not be the ones this reader derived.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::PakMushaffar`] when the payload is encrypted and no key
    /// was supplied, [`KhataUnreal::MawridTalif`] for a delete record, for a
    /// second copy that contradicts the index, or for an offset outside the
    /// container, [`KhataUnreal::DaghtMajhul`] for Oodle and any method this
    /// build cannot expand, [`KhataUnreal::FakkFashil`] when a block will not
    /// expand, [`KhataUnreal::HajmGhayrMutabaq`] when the result is not the size
    /// the entry declared, and [`KhataUnreal::BasmaGhayrMutabaqa`] when it does
    /// not match the recorded hash.
    pub fn iqra_madkhal(&self, madkhal: &MadkhalPak, ism: &str) -> Result<Vec<u8>, KhataUnreal> {
        if madkhal.mahdhuf {
            return Err(talif(
                "a delete record, which names a path this container removes and holds no bytes",
                0,
                0,
            ));
        }
        if madkhal.mushaffar && self.miftah.is_none() {
            return Err(KhataUnreal::PakMushaffar {
                masar: self.masar.clone(),
            });
        }
        if madkhal.hajm_khaam > AQSA_HAJM_KHAAM {
            return Err(mufrit(
                "an entry's expanded size",
                madkhal.hajm_khaam,
                AQSA_HAJM_KHAAM,
            ));
        }

        self.tahaqquq_nuskha_thaniya(madkhal)?;
        let muhtawa = if madkhal.daght.khaam() {
            self.iqra_khaam(madkhal)?
        } else {
            self.iqra_madghut(madkhal)?
        };
        let fili = tul_u64(muhtawa.len());
        if fili != madkhal.hajm_khaam {
            return Err(KhataUnreal::HajmGhayrMutabaq {
                ism: ISM,
                muallan: madkhal.hajm_khaam,
                fili,
            });
        }
        self.tahaqquq_basmat_madkhal(madkhal, ism, &muhtawa)?;
        Ok(muhtawa)
    }

    /// Reads an entry stored raw, decrypting it when it is encrypted.
    fn iqra_khaam(&self, madkhal: &MadkhalPak) -> Result<Vec<u8>, KhataUnreal> {
        // Two independent fields describing one length. A raw entry where they
        // disagree is one this reader and the engine would slice differently,
        // and the difference is invisible in the bytes.
        if madkhal.hajm_makhzun != madkhal.hajm_khaam {
            return Err(KhataUnreal::HajmGhayrMutabaq {
                ism: ISM,
                muallan: madkhal.hajm_khaam,
                fili: madkhal.hajm_makhzun,
            });
        }
        let bidaya = madkhal
            .bidayat_bayanat()
            .ok_or_else(|| talif("an entry's offset", madkhal.izaha, self.tadhyeel.hajm_malaf))?;
        mintaqa(
            self.masdar.bayt(),
            bidaya,
            madkhal.hajm_makhzun,
            "an entry's payload",
            AQSA_HAJM_KHAAM,
            self.tashfeer(madkhal),
        )
    }

    /// What one entry's payload needs to know about encryption.
    ///
    /// Per entry rather than per container: version 3 put an encryption flag on
    /// every entry, so a container whose index is plaintext may still hold
    /// encrypted files, and a reader that decided once at the top would hand
    /// ciphertext to the decompressor.
    fn tashfeer(&self, madkhal: &MadkhalPak) -> Tashfeer<'_> {
        Tashfeer {
            miftah: self.miftah.as_ref(),
            mushaffara: madkhal.mushaffar,
            masar: self.masar.as_path(),
        }
    }

    /// Reads a compressed entry, one block at a time.
    ///
    /// Each block expands to the entry's block size, except the last, which
    /// expands to whatever is left. Passing the block size for the last block
    /// too would refuse every compressed file whose size is not an exact
    /// multiple of it, which is almost all of them.
    fn iqra_madghut(&self, madkhal: &MadkhalPak) -> Result<Vec<u8>, KhataUnreal> {
        if madkhal.kutal.is_empty() {
            return Err(talif("a compressed entry's block count", 0, 1));
        }
        let hajm_kutla = u64::from(madkhal.hajm_kutla);
        if hajm_kutla == 0 && madkhal.kutal.len() > 1 {
            return Err(talif(
                "a compressed entry's block size",
                0,
                madkhal.hajm_khaam,
            ));
        }
        let haql = "an entry's expanded size";
        let siaa = hajm_usize(madkhal.hajm_khaam)
            .ok_or_else(|| mufrit(haql, madkhal.hajm_khaam, AQSA_HAJM_KHAAM))?;

        let mut muhtawa = Vec::with_capacity(siaa);
        for kutla in &madkhal.kutal {
            let tul = kutla
                .tul()
                .ok_or_else(|| talif("a compression block's end", kutla.nihaya, kutla.bidaya))?;
            let makhzun = mintaqa(
                self.masdar.bayt(),
                kutla.bidaya,
                tul,
                "a compression block",
                AQSA_KUTLA_KHAAM,
                self.tashfeer(madkhal),
            )?;
            let baqi = madkhal.hajm_khaam.saturating_sub(tul_u64(muhtawa.len()));
            let matlub = if hajm_kutla == 0 {
                baqi
            } else {
                hajm_kutla.min(baqi)
            };
            let mafkuk = fukk_daght(&madkhal.daght, &makhzun, matlub)?;
            muhtawa.extend_from_slice(&mafkuk);
        }
        Ok(muhtawa)
    }

    /// Reads the entry header stored immediately before the payload and holds
    /// it against the copy the index gave.
    ///
    /// The two are written by the same tool from the same values and differ only
    /// in their offset base, so a disagreement in any field means this reader is
    /// not looking where it thinks it is. Refused rather than preferred one way
    /// or the other: there is no principled reason to trust either copy over the
    /// other, and a container whose two copies disagree is a container something
    /// rewrote.
    ///
    /// **The block bounds are compared, not only the sizes.** The sizes are the
    /// same numbers whatever base a reader resolved the blocks against, so
    /// comparing them alone cannot notice a wrong base — which is precisely the
    /// mistake this module's header spends the most words on, and precisely the
    /// one that shipped. The bounds can: the second copy states each block's
    /// position outright, the index copy or the encoded record derives it, and
    /// the two are the same absolute bytes on a container nobody has touched.
    fn tahaqquq_nuskha_thaniya(&self, madkhal: &MadkhalPak) -> Result<(), KhataUnreal> {
        let khana = qass(
            self.masdar.bayt(),
            madkhal.izaha,
            madkhal.tul_musalsal(),
            "an entry's second copy",
        )?;
        let mut qari = Qari::jadeed(ISM, khana);
        let thaniya = MadkhalPak::min_qari(
            &mut qari,
            madkhal.isdar,
            &self.tadhyeel.asma_daght,
            Some(madkhal.izaha),
        )?;
        if thaniya.hajm_makhzun != madkhal.hajm_makhzun {
            return Err(talif(
                "the stored size in the entry's second copy",
                thaniya.hajm_makhzun,
                madkhal.hajm_makhzun,
            ));
        }
        if thaniya.hajm_khaam != madkhal.hajm_khaam {
            return Err(talif(
                "the expanded size in the entry's second copy",
                thaniya.hajm_khaam,
                madkhal.hajm_khaam,
            ));
        }
        if thaniya.fahras_daght != madkhal.fahras_daght {
            return Err(talif(
                "the compression method in the entry's second copy",
                u64::from(thaniya.fahras_daght),
                u64::from(madkhal.fahras_daght),
            ));
        }
        if thaniya.kutal.len() != madkhal.kutal.len() {
            return Err(talif(
                "the compression block count in the entry's second copy",
                tul_u64(thaniya.kutal.len()),
                tul_u64(madkhal.kutal.len()),
            ));
        }
        for (thaniya, awwal) in thaniya.kutal.iter().zip(madkhal.kutal.iter()) {
            if thaniya.bidaya != awwal.bidaya {
                return Err(talif(
                    "a compression block's start in the entry's second copy",
                    thaniya.bidaya,
                    awwal.bidaya,
                ));
            }
            if thaniya.nihaya != awwal.nihaya {
                return Err(talif(
                    "a compression block's end in the entry's second copy",
                    thaniya.nihaya,
                    awwal.nihaya,
                ));
            }
        }
        Ok(())
    }

    /// Checks a payload against the SHA-1 the container recorded for it.
    ///
    /// Both readings again, for the same reason the index hash needs both: some
    /// engine versions hash a file's original bytes and some hash what they
    /// wrote, and on an uncompressed entry the two are the same buffer so the
    /// difference never shows up in testing. The stored form is streamed block
    /// by block rather than assembled, so checking it costs no more memory than
    /// reading it did.
    fn tahaqquq_basmat_madkhal(
        &self,
        madkhal: &MadkhalPak,
        ism: &str,
        muhtawa: &[u8],
    ) -> Result<(), KhataUnreal> {
        if !madkhal.lahu_basma() {
            return Ok(());
        }
        if basma_bayt(muhtawa) == madkhal.basma {
            return Ok(());
        }
        let bayt = self.masdar.bayt();
        let mut basma = Basma::jadeeda();
        if madkhal.kutal.is_empty() {
            let bidaya = madkhal.bidayat_bayanat().unwrap_or(madkhal.izaha);
            if let Ok(khana) = qass(bayt, bidaya, madkhal.hajm_makhzun, "an entry's payload") {
                basma.hadhi(khana);
            }
        } else {
            for kutla in &madkhal.kutal {
                let tul = kutla.tul().unwrap_or(0);
                if let Ok(khana) = qass(bayt, kutla.bidaya, tul, "a compression block") {
                    basma.hadhi(khana);
                }
            }
        }
        if basma.anhi() == madkhal.basma {
            return Ok(());
        }
        Err(KhataUnreal::BasmaGhayrMutabaqa {
            ism: ISM,
            madkhal: ism.to_owned(),
        })
    }
}

/// Whether a path ends in an extension, ignoring case.
///
/// Case-insensitively because a container cooked on Windows carries whatever
/// casing the content browser had, and a game that shipped `Game.LocRes` is a
/// game this reader must not report as having no translations.
fn lahiqa_hiya(masar: &str, lahiqa: &str) -> bool {
    masar.len() >= lahiqa.len()
        && masar
            .get(masar.len().saturating_sub(lahiqa.len())..)
            .is_some_and(|dhayl| dhayl.eq_ignore_ascii_case(lahiqa))
}

// ---------------------------------------------------------------------------
// The writer
// ---------------------------------------------------------------------------

/// The mount point [`KatibPak`] writes unless it is told another one.
///
/// `../../../` is what `UnrealPak` writes for a container staged from the
/// project root, and it is what the game's own containers carry, so paths inside
/// the patch resolve against the same base the paths they override resolved
/// against. A patch with a different mount point mounts cleanly and then
/// overrides nothing, which is the failure mode this constant exists to avoid.
pub const NUQTAT_WASL_IFTIRADIYA: &str = "../../../";

/// Whether a container's name puts it in Unreal's higher mount-priority class.
///
/// The engine gives a `_P`-suffixed `.pak` priority over one without, and orders
/// containers within a class by name. Taarib's own container is named for both
/// halves of that rule — see [`crate::ISM_HAWIYA`] — and this is the predicate a
/// caller uses to check that a name it composed still satisfies it.
#[must_use]
pub fn ism_dhu_awlawiya(ism: &str) -> bool {
    Path::new(ism)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|jidhr| jidhr.ends_with("_P"))
}

/// Builds a small, uncompressed, unencrypted container.
///
/// This is the whole write path, and it is deliberately the smallest thing that
/// can be correct: the patch holds the handful of files Taarib replaces — an
/// Arabic `.locres`, the `.locmeta` that lists it — and there is no reason for
/// it to be compressed, encrypted, chunked, or indexed by hash. Every one of
/// those would be a structure this module could get wrong in a way that mounts
/// and then misbehaves.
///
/// It writes pak version [`ISDAR_KITABA`] unless [`Self::bi_isdar`] lowers it
/// to the game's own; the justification is in this module's header.
#[derive(Debug, Clone)]
pub struct KatibPak {
    nuqtat_wasl: String,
    isdar: IsdarPak,
    malafat: BTreeMap<String, Vec<u8>>,
}

impl Default for KatibPak {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl KatibPak {
    /// An empty container with the default mount point.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::bi_nuqtat_wasl(NUQTAT_WASL_IFTIRADIYA)
    }

    /// An empty container with a mount point the caller chose.
    ///
    /// The right value is whatever the container being overridden used, which
    /// [`FahrasPak::nuqtat_wasl`] reports. A trailing separator is appended when
    /// it is missing, because the engine's own loader does the same and a mount
    /// point without one joins its paths wrongly.
    #[must_use]
    pub fn bi_nuqtat_wasl(nuqtat_wasl: &str) -> Self {
        let mut wasl = nuqtat_wasl.replace('\\', "/");
        if !wasl.ends_with('/') {
            wasl.push('/');
        }
        Self {
            nuqtat_wasl: wasl,
            isdar: IsdarPak(ISDAR_KITABA),
            malafat: BTreeMap::new(),
        }
    }

    /// Writes the container at the version a game's own containers declare.
    ///
    /// The value to pass is the game's, straight from [`TadhyeelPak::isdar`];
    /// [`IsdarPak::lil_kitaba`] does the capping and the refusing, so a caller
    /// never chooses a number itself.
    ///
    /// # Errors
    ///
    /// Whatever [`IsdarPak::lil_kitaba`] refuses.
    pub fn bi_isdar(mut self, isdar_luba: IsdarPak) -> Result<Self, KhataUnreal> {
        self.isdar = isdar_luba.lil_kitaba()?;
        Ok(self)
    }

    /// The version this container will declare.
    #[must_use]
    pub const fn isdar(&self) -> IsdarPak {
        self.isdar
    }

    /// Adds one file, replacing any file already at that path.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] for an empty path, and
    /// [`KhataUnreal::HajmMufrit`] for a path above [`AQSA_TUL_MASAR`], a
    /// payload above [`AQSA_HAJM_KHAAM`], or more files than
    /// [`AQSA_MADAKHIL`] — the same ceilings the reader enforces, applied on the
    /// way out so that this module cannot write a container it would refuse to
    /// read.
    pub fn daa(&mut self, masar: &str, muhtawa: Vec<u8>) -> Result<(), KhataUnreal> {
        let masar = masar_muwahhad(masar);
        if masar.is_empty() {
            return Err(talif("an entry's path, which is empty", 0, 1));
        }
        let tul = tul_u64(masar.len());
        if tul > AQSA_TUL_MASAR {
            return Err(mufrit("an entry's path", tul, AQSA_TUL_MASAR));
        }
        let hajm = tul_u64(muhtawa.len());
        if hajm > AQSA_HAJM_KHAAM {
            return Err(mufrit("an entry's payload", hajm, AQSA_HAJM_KHAAM));
        }
        if tul_u64(self.malafat.len()) >= AQSA_MADAKHIL {
            return Err(mufrit(
                "the container's entry count",
                AQSA_MADAKHIL,
                AQSA_MADAKHIL,
            ));
        }
        let _ = self.malafat.insert(masar, muhtawa);
        Ok(())
    }

    /// The mount point this container will declare.
    #[must_use]
    pub fn nuqtat_wasl(&self) -> &str {
        &self.nuqtat_wasl
    }

    /// How many files it holds.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.malafat.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.malafat.is_empty()
    }

    /// Every path it holds, in the order it will store them.
    pub fn masarat(&self) -> impl Iterator<Item = &str> + '_ {
        self.malafat.keys().map(String::as_str)
    }

    /// Serializes the whole container.
    ///
    /// ```text
    /// the container this writes — version 3, 4 or 5, little-endian
    ///
    ///   for each file, in sorted path order:
    ///     53 bytes  the entry header, with its offset written as zero
    ///     n  bytes  the payload, stored raw
    ///   the index:
    ///     FString   the mount point
    ///     i32       how many entries follow
    ///     for each: FString path, then the same 53-byte header with the
    ///               absolute offset of that file's own header
    ///   the footer, 44 bytes at version 3 and 45 from version 4:
    ///     u8        the index encryption flag, zero — version 4 and later
    ///     u32       the magic
    ///     u32       the version
    ///     i64 i64   where the index is and how long
    ///     [u8; 20]  its SHA-1
    /// ```
    ///
    /// Paths are written in sorted order because a `BTreeMap` holds them that
    /// way, and that makes the output a deterministic function of its input: the
    /// same set of files produces the same bytes on every machine, which is what
    /// lets a build be reproduced and a patch be compared against the one a user
    /// actually has.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::HajmMufrit`] when the container as a whole, or one path in
    /// it, is larger than the format's signed fields can express — which for a
    /// patch holding a few localization resources cannot happen, and is checked
    /// because "cannot happen" is not a thing a serializer should assume about
    /// input it did not choose.
    pub fn ila_bayt(&self) -> Result<Vec<u8>, KhataUnreal> {
        let isdar = self.isdar;
        let mut katib = Katib::jadeed();
        let mut mawaqi: Vec<(&str, u64, u64, [u8; HAJM_BASMA])> =
            Vec::with_capacity(self.malafat.len());

        for (masar, muhtawa) in &self.malafat {
            let izaha = tul_u64(katib.mawqi());
            let hajm = tul_u64(muhtawa.len());
            let basma = basma_bayt(muhtawa);
            // Copy two, whose offset field is zero because it describes itself.
            uktub_madkhal(&mut katib, 0, hajm, &basma)?;
            katib.uktub_bayt(muhtawa);
            mawaqi.push((masar.as_str(), izaha, hajm, basma));
        }

        let izahat_fahras = tul_u64(katib.mawqi());
        let adad = i32::try_from(mawaqi.len()).map_err(|_| {
            mufrit(
                "the container's entry count",
                tul_u64(mawaqi.len()),
                AQSA_MADAKHIL,
            )
        })?;
        let mut fahras = Katib::bi_siaa(mawaqi.len().saturating_mul(96));
        fahras.uktub_nass(
            ISM,
            "the mount point",
            &NassMukhazzan::jadeed(&self.nuqtat_wasl),
        )?;
        fahras.uktub_i32(adad);
        for (masar, izaha, hajm, basma) in &mawaqi {
            fahras.uktub_nass(ISM, "an entry's path", &NassMukhazzan::jadeed(masar))?;
            // The index copy, whose offset is absolute and points at copy two.
            uktub_madkhal(&mut fahras, *izaha, *hajm, basma)?;
        }
        let bayt_fahras = fahras.ila_vec();
        let hajm_fahras = tul_u64(bayt_fahras.len());
        let basmat_fahras = basma_bayt(&bayt_fahras);
        katib.uktub_bayt(&bayt_fahras);

        // The flag byte exists from version 4; a version 3 reader seeks
        // forty-four bytes from the end and would find the magic one byte late.
        if isdar.tashfeer_fahras() {
            katib.uktub_u8(0);
        }
        katib.uktub_u32(SIHR);
        katib.uktub_u32(isdar.raqm());
        katib.uktub_i64(musir(izahat_fahras, "the index offset")?);
        katib.uktub_i64(musir(hajm_fahras, "the index size")?);
        katib.uktub_bayt(&basmat_fahras);
        Ok(katib.ila_vec())
    }

    /// Writes the container to a path.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] naming the path, and everything
    /// [`Self::ila_bayt`] raises.
    pub fn uktub(&self, masar: &Path) -> Result<(), KhataUnreal> {
        let bayt = self.ila_bayt()?;
        std::fs::write(masar, bayt).map_err(|sabab| KhataUnreal::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })
    }

    /// Writes the container where a packaged game will mount it, and returns
    /// where it went.
    ///
    /// `mujallad_mashru` is the project directory — `<root>/<Project>` — which
    /// Phase 5 reports through `FahisUnreal::mashru`. The file lands at
    /// [`crate::DALIL_HAWIYA`] under it, named [`crate::ISM_HAWIYA`], beside the
    /// game's own containers rather than in a mod folder: a mod folder is a
    /// convention some games have and most do not, and the containers directory
    /// is where the engine already looks.
    ///
    /// Nothing the game shipped is touched. Uninstalling is deleting the one
    /// file this returns.
    ///
    /// The containers directory is resolved through
    /// [`crate::isdar::masar_bila_hala`] rather than joined literally. A depot
    /// that ships `content/paks` runs correctly for a player on Wine or Proton,
    /// and a literal join would create a second `Content/Paks` beside it, put
    /// the patch container in it, and report an install the engine never mounts.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] when the containers directory cannot be
    /// created or the file cannot be written, and everything [`Self::ila_bayt`]
    /// raises.
    pub fn uktub_fi_luba(&self, mujallad_mashru: &Path) -> Result<PathBuf, KhataUnreal> {
        let mujallad = crate::isdar::masar_bila_hala(mujallad_mashru, crate::DALIL_HAWIYA);
        std::fs::create_dir_all(&mujallad).map_err(|sabab| KhataUnreal::KhataMalaf {
            masar: mujallad.clone(),
            sabab,
        })?;
        // The file name itself is Taarib's own and is written as spelled: the
        // `_P` suffix and the leading `zzz` are what give it mount priority, and
        // adopting some other casing found on disk would be adopting a name a
        // previous tool chose.
        let masar = mujallad.join(crate::ISM_HAWIYA);
        self.uktub(&masar)?;
        Ok(masar)
    }
}

/// Writes one version 5 entry header: uncompressed, unencrypted, fifty-three
/// bytes.
///
/// The same bytes for both copies; only `izaha` differs, which is the entire
/// difference this module's header spends so long on.
fn uktub_madkhal(
    katib: &mut Katib,
    izaha: u64,
    hajm: u64,
    basma: &[u8; HAJM_BASMA],
) -> Result<(), KhataUnreal> {
    katib.uktub_i64(musir(izaha, "an entry's offset")?);
    katib.uktub_i64(musir(hajm, "an entry's stored size")?);
    katib.uktub_i64(musir(hajm, "an entry's expanded size")?);
    // Version 5 predates the compression-method name table, so this is the
    // legacy `ECompressionFlags` field and zero is `COMPRESS_None`.
    katib.uktub_i32(0);
    katib.uktub_bayt(basma);
    // The encryption flag and the compression block size, both of which version
    // 3 added and both of which are zero for an entry stored raw. They are
    // written rather than skipped: the field is unconditional from version 3,
    // and omitting it would shift every byte after it.
    katib.uktub_u8(0);
    katib.uktub_u32(0);
    Ok(())
}

/// A length as the `i64` the format writes, refusing one that does not fit.
fn musir(qeema: u64, haql: &'static str) -> Result<i64, KhataUnreal> {
    i64::try_from(qeema).map_err(|_| mufrit(haql, qeema, i64::MAX.unsigned_abs()))
}
