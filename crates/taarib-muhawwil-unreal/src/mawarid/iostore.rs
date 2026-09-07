//! حاوية أيو-ستور — reading UE5's `.utoc` / `.ucas` pair.
//!
//! # This module reads. It does not write.
//!
//! Say it once, at the top, because the absence of a writer here is a decision
//! and not an unfinished corner: **Taarib never produces an IoStore container.**
//!
//! Reinjection ships an additive patch `.pak` mounted at a higher priority than
//! the game's own files, and UE5 still mounts `.pak` files alongside IoStore
//! containers — `FPakPlatformFile` and `FIoDispatcher` coexist in every shipping
//! UE5 build, and the pak layer wins for any path both provide. So a patch that
//! carries the translated `.locres` in a small `.pak` reaches the engine exactly
//! as intended, and writing an IoStore container would buy nothing.
//!
//! It would also cost something. A `.utoc` is a table of file-relative offsets,
//! a perfect-hash index over chunk ids, a compression-block table whose entries
//! are bit-packed into twelve bytes, an optional signature block and a directory
//! index — and the `.ucas` beside it must agree with every one of those numbers,
//! byte for byte. Get one field wrong and the engine mounts the container
//! happily and then fails at the first chunk read, which surfaces to a player as
//! a game that starts and then cannot load a level. A refusal is diagnosable; a
//! container the engine accepts and cannot read is not.
//!
//! The read path exists for one reason: the extractor must **find and read the
//! game's existing `.locres` files**, and in a UE5 title those live inside the
//! IoStore container rather than in a loose `.pak`. Finding them by name is what
//! the directory index is for, and reading them is what everything below it is
//! for.
//!
//! # Why there are two files
//!
//! UE4's `.pak` was one file: an index at a known offset, entries pointing back
//! into the same blob. UE5 split that in two.
//!
//! * The **`.utoc`** is the table of contents. It is small — a few megabytes for
//!   a large game — and this module reads it whole, into one `&[u8]`, and
//!   validates every offset and count in it against that slice's real length
//!   before it sizes a single allocation.
//! * The **`.ucas`** is the content-addressed store: the actual bytes, in
//!   compression blocks, potentially split across several partition files. It is
//!   tens of gigabytes. Nothing here reads it whole. It is reached through
//!   [`QariNitaq`], which reads one bounded range at a time, and the ranges are
//!   derived from a table that has already been proven internally consistent.
//!
//! That asymmetry is the design: the file whose numbers must be trusted is the
//! small one, so it can be validated exhaustively and cheaply; the file that is
//! enormous is only ever addressed by ranges the small one authorised.
//!
//! # The eight `ToC` versions, and what each one changed
//!
//! [`IsdarIoStore`] enumerates every version this build reads. The layout below
//! the header is version-dependent, so the version is checked *before* any other
//! header field is interpreted — a parse that reads fields first and checks the
//! version afterwards has already read fields whose meaning it guessed.
//!
//! **1, `Initial`.** Chunk ids, offsets and lengths, compression blocks,
//! compression method names. No directory index, so files are addressable only
//! by chunk id and not by path.
//!
//! **2, `DirectoryIndex`.** Adds the directory index blob after the signature
//! block, so a container can be searched by name. This is the version that makes
//! `Content/Localization/**/*.locres` findable at all, and therefore the lowest
//! version this adapter can do anything useful with.
//!
//! **3, `PartitionSize`.** The `.ucas` may be split into `_s1`, `_s2` and so on;
//! the header's partition count and partition size become meaningful. Below this
//! version there is exactly one partition of unbounded size, which is what this
//! reader normalises older headers to.
//!
//! **4, `PerfectHash`.** Adds the perfect-hash seed array between the
//! offset/length array and the compression blocks, making lookup by chunk id
//! constant-time. The array is sized by its own header count and must be skipped
//! by exactly that many bytes, or every later array is read at the wrong offset.
//!
//! **5, `PerfectHashWithOverflow`.** Adds a second array, listing the chunk
//! indices the perfect hash could not place. Same consequence for the offsets of
//! everything after it.
//!
//! **6, `OnDemandMetaData`.** Adds an on-demand region to the trailing per-chunk
//! metadata. Nothing this module needs comes after that metadata, so the change
//! only affects how the tail is measured.
//!
//! **7, `RemovedOnDemandMetaData`.** Takes that region back out again.
//!
//! **8, `ReplaceIoChunkHashWithIoHash`.** The per-chunk metadata hash narrows
//! from a 32-byte `FIoChunkHash` to a 20-byte `FIoHash`, so the metadata stride
//! changes from 33 bytes to 21.
//!
//! A version above eight is refused with [`KhataUnreal::IsdarGhayrMadum`]. It is
//! not guessed at: the arrays between the header and the directory index are
//! positional, and a version that inserted one more of them would make every
//! offset after it point into the middle of a neighbouring table. The reader
//! would not fail — it would succeed, and return the wrong bytes.
//!
//! # Compression
//!
//! Each compression block names a method by index into a fixed-width name table.
//!
//! * `None` — the block is stored raw.
//! * `Zlib` — expanded with `flate2`, into a buffer of exactly the declared size.
//! * `Zstd` — expanded with `zstd`, likewise.
//! * `Oodle` — **refused**, with [`KhataUnreal::DaghtMajhul`]. Taarib has no
//!   licence to ship an Oodle decompressor, and this is a legal limit rather than
//!   an engineering one. The intended route for an Oodle container is the game's
//!   own exported decompressor — shipping titles link `oo2core` and export it —
//!   which is the runtime half's business and is handled elsewhere. This module
//!   states the method name it could not expand and stops.
//!
//! # Encryption
//!
//! An encrypted container's compression blocks and directory index are AES-256 in
//! ECB mode, exactly as `.pak` does it. ECB, on independently addressable blocks,
//! is the engine's choice and not this reader's; a container has to be seekable.
//!
//! No key and an encrypted container is [`KhataUnreal::PakMushaffar`]. Taarib
//! does not go looking for keys, and does not read them out of the game's
//! executable. The user supplies the key or the container is not read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aes::Aes256;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, KeyInit};

use crate::khata::{KhataUnreal, hajm_usize, tul_u64};

/// The format name every refusal from this module carries.
const ISM: &str = "IoStore";

/// The sixteen bytes every `.utoc` begins with.
pub const SIHR: [u8; 16] = *b"-==--==--==--==-";

/// How many bytes the `.utoc` header occupies.
pub const HAJM_TARWISA: usize = 144;

/// How many bytes one chunk id occupies.
pub const HAJM_MUARRIF_JUZ: usize = 12;

/// How many bytes one offset-and-length record occupies.
pub const HAJM_IZAHA_WA_TUL: usize = 10;

/// How many bytes one compression-block entry occupies.
pub const HAJM_MADKHAL_KUTLA: usize = 12;

/// How many bytes one perfect-hash seed, or one overflow index, occupies.
pub const HAJM_BADHRA: usize = 4;

/// How many bytes one per-block signature occupies — `FSHAHash`, twenty bytes.
pub const HAJM_TAWQEE_KUTLA: usize = 20;

/// How many bytes one directory entry occupies in the directory index.
pub const HAJM_MADKHAL_DALIL: usize = 16;

/// How many bytes one file entry occupies in the directory index.
pub const HAJM_MADKHAL_MALAF: usize = 12;

/// The AES block size, and therefore the alignment an encrypted read must have.
pub const HAJM_KUTLAT_TASHFEER: usize = 16;

/// The index a directory or file entry uses to mean "there is no such entry".
pub const FAHRAS_GHAYR_SALIH: u32 = u32::MAX;

/// The highest `ToC` version this build reads.
pub const AQSA_ISDAR: u32 = 8;

/// The largest chunk count this build will size an array from.
///
/// Four mebi-entries. The chunk id array and the offset/length array together
/// cost twenty-two bytes an entry, so this ceiling caps them at ninety-two
/// mebibytes — well above any shipping title, and far below the point where a
/// hostile count turns a `.utoc` header into an out-of-memory abort inside a
/// game process. Checked against the declared count before a byte is reserved.
pub const AQSA_AJZA: u32 = 4 * 1024 * 1024;

/// The largest compression-block count this build will size an array from.
///
/// Eight mebi-entries at twelve bytes each is ninety-six mebibytes of table,
/// which at the usual sixty-four kibibyte block size describes half a tebibyte
/// of container. Nothing ships larger.
pub const AQSA_KUTAL: u32 = 8 * 1024 * 1024;

/// The largest number of compression method names a container may declare.
pub const AQSA_ASMA_DAGHT: u32 = 64;

/// The largest fixed width one compression method name may have.
pub const AQSA_TUL_ISM_DAGHT: u32 = 256;

/// The largest number of `.ucas` partitions a container may declare.
pub const AQSA_QITAAT: u32 = 1024;

/// The largest directory index this build will read out of a `.utoc`.
///
/// Two hundred and fifty-six mebibytes of path data. A directory index is a
/// string pool and two arrays of four-byte indices; a real one is single-digit
/// megabytes even for a game with a million files.
pub const AQSA_FAHRAS_DALIL: u32 = 256 * 1024 * 1024;

/// The largest single chunk this build will expand.
///
/// Five hundred and twelve mebibytes, checked against the chunk's *declared*
/// length before the destination is allocated. A `.locres` is measured in
/// megabytes; this ceiling exists for the case where the length field is not
/// describing a `.locres` at all.
pub const AQSA_JUZ: u64 = 512 * 1024 * 1024;

/// The largest uncompressed size one compression block may declare.
///
/// Sixteen mebibytes. The engine's default block size is sixty-four kibibytes
/// and this leaves room for containers built with a larger one, while keeping a
/// single hostile block entry from asking for a gigabyte.
pub const AQSA_KUTLA_KHAAM: u32 = 16 * 1024 * 1024;

/// The largest number of directory entries the directory index may declare.
pub const AQSA_MADAKHIL_DALIL: u32 = 4 * 1024 * 1024;

/// The largest number of file entries the directory index may declare.
pub const AQSA_MALAFAT_DALIL: u32 = 4 * 1024 * 1024;

/// The largest number of strings the directory index's pool may declare.
pub const AQSA_HAWD: u32 = 8 * 1024 * 1024;

/// The largest single string this build will read out of the directory index.
pub const AQSA_TUL_NASS: u32 = 64 * 1024;

/// The largest path this build will assemble while walking the directory tree.
pub const AQSA_TUL_MASAR: usize = 8 * 1024;

/// The largest digest size the signature block may declare.
pub const AQSA_HAJM_BASMA: u32 = 64;

// ---------------------------------------------------------------------------
// Bounded primitive reads
// ---------------------------------------------------------------------------

/// Reads one byte, or [`None`] if it is not there.
fn iqra_u8(bayt: &[u8], izaha: usize) -> Option<u8> {
    bayt.get(izaha).copied()
}

/// Reads four little-endian bytes, or [`None`] if they are not there.
fn iqra_u32(bayt: &[u8], izaha: usize) -> Option<u32> {
    let nihaya = izaha.checked_add(4)?;
    let khana: [u8; 4] = bayt.get(izaha..nihaya)?.try_into().ok()?;
    Some(u32::from_le_bytes(khana))
}

/// Reads four little-endian bytes as a signed index, or [`None`].
fn iqra_i32(bayt: &[u8], izaha: usize) -> Option<i32> {
    let nihaya = izaha.checked_add(4)?;
    let khana: [u8; 4] = bayt.get(izaha..nihaya)?.try_into().ok()?;
    Some(i32::from_le_bytes(khana))
}

/// Reads eight little-endian bytes, or [`None`] if they are not there.
fn iqra_u64(bayt: &[u8], izaha: usize) -> Option<u64> {
    let nihaya = izaha.checked_add(8)?;
    let khana: [u8; 8] = bayt.get(izaha..nihaya)?.try_into().ok()?;
    Some(u64::from_le_bytes(khana))
}

/// Reads a fixed-length run of bytes, or [`None`] if it is not there.
fn iqra_masfufa<const N: usize>(bayt: &[u8], izaha: usize) -> Option<[u8; N]> {
    let nihaya = izaha.checked_add(N)?;
    bayt.get(izaha..nihaya)?.try_into().ok()
}

/// Divides, or [`None`] when the divisor is zero.
///
/// The workspace denies bare integer division, and for a good reason here: every
/// divisor in this module — the compression block size, the partition size —
/// comes out of a header a stranger wrote, and zero is exactly what a hostile
/// header puts there.
const fn qismah(bast: u64, maqam: u64) -> Option<u64> {
    bast.checked_div(maqam)
}

/// Takes a remainder, or [`None`] when the divisor is zero.
const fn baqi(bast: u64, maqam: u64) -> Option<u64> {
    bast.checked_rem(maqam)
}

/// Rounds `qeema` up to the next multiple of `wahda`, or [`None`] on overflow.
fn ila_ala(qeema: u64, wahda: u64) -> Option<u64> {
    let zaid = qeema.checked_add(wahda.checked_sub(1)?)?;
    qismah(zaid, wahda)?.checked_mul(wahda)
}

/// A window of `tul` bytes at `izaha`, refusing anything that runs past the end.
///
/// This is the only way any array in this module is obtained, so "validated
/// against the real file length before it sizes an allocation" is a property of
/// one function rather than a habit spread over twenty call sites.
fn qass<'a>(
    bayt: &'a [u8],
    izaha: usize,
    tul: usize,
    haql: &'static str,
) -> Result<&'a [u8], KhataUnreal> {
    let nihaya = izaha
        .checked_add(tul)
        .ok_or_else(|| KhataUnreal::MalafQaseer {
            haql,
            tul: tul_u64(bayt.len()),
            matlub: u64::MAX,
        })?;
    bayt.get(izaha..nihaya)
        .ok_or_else(|| KhataUnreal::MalafQaseer {
            haql,
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(nihaya),
        })
}

/// The byte span an array of `adad` records of `hajm` bytes occupies.
///
/// Returned as a `usize` only when the product fits one; a count that overflows
/// is refused as an excessive size rather than silently wrapped into a small
/// allocation that a later loop then walks past.
fn mada(adad: u32, hajm: usize, haql: &'static str, saqf: u64) -> Result<usize, KhataUnreal> {
    usize::try_from(adad)
        .ok()
        .and_then(|adad| adad.checked_mul(hajm))
        .ok_or_else(|| KhataUnreal::HajmMufrit {
            haql,
            qeema: u64::from(adad),
            saqf,
        })
}

/// Refuses a declared count above its ceiling, before it is used for anything.
fn saqf_adad(qeema: u32, saqf: u32, haql: &'static str) -> Result<(), KhataUnreal> {
    if qeema > saqf {
        return Err(KhataUnreal::HajmMufrit {
            haql,
            qeema: u64::from(qeema),
            saqf: u64::from(saqf),
        });
    }
    Ok(())
}

/// A refusal naming a field of the container that is inconsistent with itself.
const fn talif(haql: &'static str, qeema: u64, hadd: u64) -> KhataUnreal {
    KhataUnreal::MawridTalif {
        ism: ISM,
        haql,
        qeema,
        hadd,
    }
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// The `.utoc` format version.
///
/// The variants carry the engine's own numbering, because that number is what a
/// container declares and what a bug report quotes. What each one changed is in
/// this module's header, and it is worth reading before touching the parser:
/// versions four and five each insert an array between the offset table and the
/// compression blocks, and skipping one of them by the wrong number of bytes
/// does not fail — it reads the compression block table out of the middle of the
/// seed array and returns plausible garbage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IsdarIoStore {
    /// Version 1, `Initial`. No directory index; chunks are addressable only by
    /// their [`MuarrifJuz`].
    Ibtidai,
    /// Version 2, `DirectoryIndex`. Adds the path tree this module needs to find
    /// a `.locres` by name.
    Dalil,
    /// Version 3, `PartitionSize`. The `.ucas` may be split across partitions.
    HajmQitaa,
    /// Version 4, `PerfectHash`. Adds the perfect-hash seed array.
    BasmaMutqana,
    /// Version 5, `PerfectHashWithOverflow`. Adds the array of chunk indices the
    /// perfect hash could not place.
    BasmaMutqanaBiFaid,
    /// Version 6, `OnDemandMetaData`. Adds an on-demand region to the trailing
    /// per-chunk metadata.
    BayanatTahtTalab,
    /// Version 7, `RemovedOnDemandMetaData`. Removes it again.
    BilaBayanatTahtTalab,
    /// Version 8, `ReplaceIoChunkHashWithIoHash`. The per-chunk metadata hash
    /// narrows from thirty-two bytes to twenty, so the metadata stride changes.
    BasmaIo,
}

impl IsdarIoStore {
    /// The version number as the container declares it.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        match self {
            Self::Ibtidai => 1,
            Self::Dalil => 2,
            Self::HajmQitaa => 3,
            Self::BasmaMutqana => 4,
            Self::BasmaMutqanaBiFaid => 5,
            Self::BayanatTahtTalab => 6,
            Self::BilaBayanatTahtTalab => 7,
            Self::BasmaIo => 8,
        }
    }

    /// The version a declared number names.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::IsdarGhayrMadum`] for anything above [`AQSA_ISDAR`], and
    /// [`KhataUnreal::MawridTalif`] for zero — which is not a future version but
    /// a header that was never written, or was zeroed by something that did not
    /// understand what it was editing.
    pub fn min_raqm(raqm: u8) -> Result<Self, KhataUnreal> {
        match raqm {
            1 => Ok(Self::Ibtidai),
            2 => Ok(Self::Dalil),
            3 => Ok(Self::HajmQitaa),
            4 => Ok(Self::BasmaMutqana),
            5 => Ok(Self::BasmaMutqanaBiFaid),
            6 => Ok(Self::BayanatTahtTalab),
            7 => Ok(Self::BilaBayanatTahtTalab),
            8 => Ok(Self::BasmaIo),
            0 => Err(talif("the ToC version", 0, u64::from(AQSA_ISDAR))),
            akhar => Err(KhataUnreal::IsdarGhayrMadum {
                ism: "IoStore .utoc",
                wujid: u32::from(akhar),
                aqsa: AQSA_ISDAR,
            }),
        }
    }

    /// Whether this version carries a directory index at all.
    #[must_use]
    pub fn yahwi_dalil(self) -> bool {
        self >= Self::Dalil
    }

    /// Whether this version's header describes more than one `.ucas` partition.
    #[must_use]
    pub fn yahwi_qitaat(self) -> bool {
        self >= Self::HajmQitaa
    }

    /// Whether this version carries the perfect-hash seed array.
    #[must_use]
    pub fn yahwi_bidhur(self) -> bool {
        self >= Self::BasmaMutqana
    }

    /// Whether this version carries the perfect-hash overflow array.
    #[must_use]
    pub fn yahwi_faid(self) -> bool {
        self >= Self::BasmaMutqanaBiFaid
    }

    /// The stride of one trailing per-chunk metadata record.
    ///
    /// Thirty-three bytes up to version seven — a thirty-two byte
    /// `FIoChunkHash` and one flags byte — and twenty-one from version eight,
    /// where the hash narrowed to a twenty-byte `FIoHash`. Nothing this module
    /// needs lives after the metadata, so a container whose tail does not match
    /// either stride is still read; see [`JadwalMuhtawayat::bayan`] for why that
    /// leniency is confined to this one region.
    #[must_use]
    pub const fn khatwat_bayan(self) -> usize {
        match self {
            Self::BasmaIo => 21,
            _ => 33,
        }
    }
}

// ---------------------------------------------------------------------------
// Container flags
// ---------------------------------------------------------------------------

/// [`AlamHawiya`]: the container's chunk data is compressed.
pub const ALAM_MADGHUT: u8 = 1 << 0;
/// [`AlamHawiya`]: the container's chunk data and directory index are encrypted.
pub const ALAM_MUSHAFFAR: u8 = 1 << 1;
/// [`AlamHawiya`]: the `.utoc` carries a signature block.
pub const ALAM_MUWAQQA: u8 = 1 << 2;
/// [`AlamHawiya`]: the `.utoc` carries a directory index.
pub const ALAM_MUFAHRAS: u8 = 1 << 3;
/// [`AlamHawiya`]: the container is streamed on demand rather than installed.
pub const ALAM_TAHT_TALAB: u8 = 1 << 4;

/// Every flag bit this build understands.
///
/// A container that sets anything outside this mask is refused rather than
/// masked. The same discipline the patch format applies to its own reserved
/// bits, for the same reason: a bit nobody has defined cannot be known to be
/// harmless, and a future flag that said "the directory index is compressed"
/// would turn a silent mask into a reader that parses a compressed blob as a
/// string table and reports whatever fell out.
pub const ALAM_MAARUFA: u8 =
    ALAM_MADGHUT | ALAM_MUSHAFFAR | ALAM_MUWAQQA | ALAM_MUFAHRAS | ALAM_TAHT_TALAB;

/// The container's flag byte, already checked against [`ALAM_MAARUFA`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlamHawiya(u8);

impl AlamHawiya {
    /// The raw byte.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        self.0
    }

    /// Whether the chunk data is compressed.
    #[must_use]
    pub const fn madghut(self) -> bool {
        self.0 & ALAM_MADGHUT != 0
    }

    /// Whether the chunk data and directory index are encrypted.
    #[must_use]
    pub const fn mushaffar(self) -> bool {
        self.0 & ALAM_MUSHAFFAR != 0
    }

    /// Whether a signature block follows the compression method names.
    #[must_use]
    pub const fn muwaqqa(self) -> bool {
        self.0 & ALAM_MUWAQQA != 0
    }

    /// Whether a directory index follows the signature block.
    #[must_use]
    pub const fn mufahras(self) -> bool {
        self.0 & ALAM_MUFAHRAS != 0
    }

    /// Whether the container is streamed on demand.
    #[must_use]
    pub const fn taht_talab(self) -> bool {
        self.0 & ALAM_TAHT_TALAB != 0
    }
}

// ---------------------------------------------------------------------------
// The header
// ---------------------------------------------------------------------------

/// The `.utoc` header — `FIoStoreTocHeader`.
///
/// ```text
/// TarwisatIoStore — 144 bytes, little-endian
///
///   offset  size  field                type       meaning
///        0    16  sihr                 [u8; 16]   the ASCII bytes "-==--==--==--==-"
///       16     1  isdar                u8         EIoStoreTocVersion, 1..=8
///       17     1  mahjuz0              u8         reserved, written as zero
///       18     2  mahjuz1              u16        reserved, written as zero
///       20     4  hajm_tarwisa         u32        header size, must be 144
///       24     4  adad_ajza            u32        how many chunk entries follow
///       28     4  adad_kutal           u32        how many compression-block entries
///       32     4  hajm_madkhal_kutla   u32        size of one block entry, must be 12
///       36     4  adad_asma_daght      u32        how many compression method names
///       40     4  tul_ism_daght        u32        fixed width of one method name
///       44     4  hajm_kutlat_daght    u32        uncompressed bytes per block
///       48     4  hajm_fahras_dalil    u32        directory index size, in bytes
///       52     4  adad_qitaat          u32        how many .ucas partitions
///       56     8  muarrif_hawiya       u64        FIoContainerId
///       64    16  muarrif_miftah       [u8; 16]   FGuid of the encryption key
///       80     1  alam                 u8         EIoContainerFlags
///       81     1  mahjuz3              u8         reserved, written as zero
///       82     2  mahjuz4              u16        reserved, written as zero
///       84     4  adad_bidhur          u32        perfect-hash seed count (v4+)
///       88     8  hajm_qitaa           u64        bytes per .ucas partition (v3+)
///       96     4  adad_bila_basma      u32        overflow chunk-index count (v5+)
///      100     4  mahjuz7              u32        reserved, written as zero
///      104    40  mahjuz8              [u64; 5]   reserved, written as zero
/// ```
///
/// Sixteen plus one plus one plus two plus nine four-byte fields plus eight plus
/// sixteen plus four plus four plus eight plus four plus four plus forty is one
/// hundred and forty-four exactly, and `hajm_tarwisa` is checked against that
/// number rather than trusted: a header that declares a different size is either
/// a version this build does not read — in which case `isdar` should have said
/// so — or is not a `.utoc` header at all, and every offset computed from it
/// would be measured from the wrong origin.
///
/// The version-conditional fields are **normalised at parse time**, so no code
/// below this struct has to remember which version it is reading:
///
/// * below version 3 there are no partitions, so `adad_qitaat` becomes one and
///   `hajm_qitaa` becomes [`u64::MAX`] — one unbounded partition, which is
///   arithmetically the same thing and removes a special case from the read path;
/// * below version 4 `adad_bidhur` becomes zero;
/// * below version 5 `adad_bila_basma` becomes zero;
/// * below version 2, or with [`ALAM_MUFAHRAS`] clear, `hajm_fahras_dalil`
///   becomes zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TarwisatIoStore {
    /// The format version, checked before any other field is interpreted.
    pub isdar: IsdarIoStore,
    /// The declared header size. Always [`HAJM_TARWISA`] in a header accepted here.
    pub hajm_tarwisa: u32,
    /// How many chunk entries the `ToC` holds, at most [`AQSA_AJZA`].
    pub adad_ajza: u32,
    /// How many compression-block entries the `ToC` holds, at most [`AQSA_KUTAL`].
    pub adad_kutal: u32,
    /// The declared size of one block entry. Always [`HAJM_MADKHAL_KUTLA`].
    pub hajm_madkhal_kutla: u32,
    /// How many compression method names follow the block table.
    pub adad_asma_daght: u32,
    /// The fixed width of one compression method name, in bytes.
    pub tul_ism_daght: u32,
    /// How many uncompressed bytes one compression block expands to, at most.
    pub hajm_kutlat_daght: u32,
    /// How many bytes the directory index blob occupies, after normalisation.
    pub hajm_fahras_dalil: u32,
    /// How many `.ucas` partitions the container is split across, at least one.
    pub adad_qitaat: u32,
    /// The container id the engine mounts this container under.
    pub muarrif_hawiya: u64,
    /// The GUID of the AES key this container was encrypted with, all zero when
    /// it is not encrypted. Taarib reports it and does not resolve it: mapping a
    /// GUID to a key is what a key store does, and this crate has none.
    pub muarrif_miftah: [u8; 16],
    /// The container flags, already checked against [`ALAM_MAARUFA`].
    pub alam: AlamHawiya,
    /// How many perfect-hash seeds follow the offset table, after normalisation.
    pub adad_bidhur: u32,
    /// How many bytes one `.ucas` partition holds, after normalisation.
    pub hajm_qitaa: u64,
    /// How many overflow chunk indices follow the seeds, after normalisation.
    pub adad_bila_basma: u32,
}

impl TarwisatIoStore {
    /// Reads and validates the header, and nothing beyond it.
    ///
    /// The order of the checks is the order they depend on each other in, and is
    /// the same order Taarib's own container format applies to its header:
    ///
    /// 1. **length** — one hundred and forty-four bytes must exist before any
    ///    offset into the header means anything;
    /// 2. **magic** — the cheapest way to reject a file that is not a `.utoc`,
    ///    and what stops every later refusal from being confusing;
    /// 3. **version** — before any *other* field is interpreted, because the
    ///    layout of everything after the header is a property of the version;
    /// 4. **the self-describing sizes** — header size and block-entry size, each
    ///    against the constant this build was written to;
    /// 5. **flags**, then **counts**, each against a ceiling, before any of them
    ///    is allowed to size an allocation.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] when the slice is shorter than the header,
    /// [`KhataUnreal::SihrGhayrMutabaq`] when the magic is absent,
    /// [`KhataUnreal::IsdarGhayrMadum`] for a version above [`AQSA_ISDAR`],
    /// [`KhataUnreal::MawridTalif`] for a self-describing size or a flag bit this
    /// build does not recognise, and [`KhataUnreal::HajmMufrit`] for any count
    /// above its ceiling.
    pub fn min_bayt(bayt: &[u8], masar: &Path) -> Result<Self, KhataUnreal> {
        if bayt.len() < HAJM_TARWISA {
            return Err(KhataUnreal::MalafQaseer {
                haql: "the .utoc header",
                tul: tul_u64(bayt.len()),
                matlub: tul_u64(HAJM_TARWISA),
            });
        }
        let naqis = |haql: &'static str| KhataUnreal::MalafQaseer {
            haql,
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(HAJM_TARWISA),
        };

        let sihr: [u8; 16] = iqra_masfufa(bayt, 0).ok_or_else(|| naqis("the .utoc magic"))?;
        if sihr != SIHR {
            return Err(KhataUnreal::SihrGhayrMutabaq {
                malaf: masar.to_path_buf(),
                ism: ISM,
            });
        }

        let isdar = IsdarIoStore::min_raqm(iqra_u8(bayt, 16).ok_or_else(|| naqis("the version"))?)?;

        let hajm_tarwisa = iqra_u32(bayt, 20).ok_or_else(|| naqis("the header size"))?;
        if hajm_tarwisa != u32::try_from(HAJM_TARWISA).unwrap_or(u32::MAX) {
            return Err(talif(
                "the header size",
                u64::from(hajm_tarwisa),
                tul_u64(HAJM_TARWISA),
            ));
        }

        let adad_ajza = iqra_u32(bayt, 24).ok_or_else(|| naqis("the chunk count"))?;
        let adad_kutal = iqra_u32(bayt, 28).ok_or_else(|| naqis("the block count"))?;
        let hajm_madkhal_kutla = iqra_u32(bayt, 32).ok_or_else(|| naqis("the block entry size"))?;
        let adad_asma_daght = iqra_u32(bayt, 36).ok_or_else(|| naqis("the method name count"))?;
        let tul_ism_daght = iqra_u32(bayt, 40).ok_or_else(|| naqis("the method name length"))?;
        let hajm_kutlat_daght = iqra_u32(bayt, 44).ok_or_else(|| naqis("the block size"))?;
        let hajm_fahras_dalil = iqra_u32(bayt, 48).ok_or_else(|| naqis("the index size"))?;
        let adad_qitaat = iqra_u32(bayt, 52).ok_or_else(|| naqis("the partition count"))?;
        let muarrif_hawiya = iqra_u64(bayt, 56).ok_or_else(|| naqis("the container id"))?;
        let muarrif_miftah: [u8; 16] =
            iqra_masfufa(bayt, 64).ok_or_else(|| naqis("the encryption key GUID"))?;
        let alam_khaam = iqra_u8(bayt, 80).ok_or_else(|| naqis("the container flags"))?;
        let adad_bidhur = iqra_u32(bayt, 84).ok_or_else(|| naqis("the perfect-hash seed count"))?;
        let hajm_qitaa = iqra_u64(bayt, 88).ok_or_else(|| naqis("the partition size"))?;
        let adad_bila_basma = iqra_u32(bayt, 96).ok_or_else(|| naqis("the overflow count"))?;

        // The block entry size is the container telling this build what stride to
        // walk the block table with. It is checked rather than used, because a
        // reader that walked with the declared stride would read a table of
        // thirteen-byte entries as twelve-byte ones and produce offsets that are
        // wrong by a growing amount — plausible near the start, nonsense later.
        if hajm_madkhal_kutla != u32::try_from(HAJM_MADKHAL_KUTLA).unwrap_or(u32::MAX) {
            return Err(talif(
                "the compression block entry size",
                u64::from(hajm_madkhal_kutla),
                tul_u64(HAJM_MADKHAL_KUTLA),
            ));
        }

        if alam_khaam & !ALAM_MAARUFA != 0 {
            let maaruf = u64::from(ALAM_MAARUFA);
            return Err(talif("the container flags", u64::from(alam_khaam), maaruf));
        }
        let alam = AlamHawiya(alam_khaam);

        saqf_adad(adad_ajza, AQSA_AJZA, "the chunk count")?;
        saqf_adad(adad_kutal, AQSA_KUTAL, "the compression block count")?;
        saqf_adad(
            adad_asma_daght,
            AQSA_ASMA_DAGHT,
            "the compression method count",
        )?;
        saqf_adad(
            tul_ism_daght,
            AQSA_TUL_ISM_DAGHT,
            "the compression method name length",
        )?;
        saqf_adad(
            hajm_kutlat_daght,
            AQSA_KUTLA_KHAAM,
            "the compression block size",
        )?;

        // A named method table with a zero-width name is a table of nothing, and
        // every block index into it would resolve to an empty string that no
        // decompressor matches.
        if adad_asma_daght > 0 && tul_ism_daght == 0 {
            let saqf = u64::from(AQSA_TUL_ISM_DAGHT);
            return Err(talif("the compression method name length", 0, saqf));
        }

        // Version normalisation. Everything below this line may read these fields
        // without asking which version produced them.
        let (adad_qitaat, hajm_qitaa) = if isdar.yahwi_qitaat() {
            saqf_adad(adad_qitaat, AQSA_QITAAT, "the partition count")?;
            if adad_qitaat == 0 || hajm_qitaa == 0 {
                let saqf = u64::from(AQSA_QITAAT);
                return Err(talif("the partition count", u64::from(adad_qitaat), saqf));
            }
            (adad_qitaat, hajm_qitaa)
        } else {
            (1, u64::MAX)
        };
        let adad_bidhur = if isdar.yahwi_bidhur() {
            saqf_adad(adad_bidhur, AQSA_AJZA, "the perfect-hash seed count")?;
            adad_bidhur
        } else {
            0
        };
        let adad_bila_basma = if isdar.yahwi_faid() {
            saqf_adad(
                adad_bila_basma,
                AQSA_AJZA,
                "the perfect-hash overflow count",
            )?;
            adad_bila_basma
        } else {
            0
        };
        let hajm_fahras_dalil = if isdar.yahwi_dalil() && alam.mufahras() {
            saqf_adad(
                hajm_fahras_dalil,
                AQSA_FAHRAS_DALIL,
                "the directory index size",
            )?;
            hajm_fahras_dalil
        } else {
            0
        };

        Ok(Self {
            isdar,
            hajm_tarwisa,
            adad_ajza,
            adad_kutal,
            hajm_madkhal_kutla,
            adad_asma_daght,
            tul_ism_daght,
            hajm_kutlat_daght,
            hajm_fahras_dalil,
            adad_qitaat,
            muarrif_hawiya,
            muarrif_miftah,
            alam,
            adad_bidhur,
            hajm_qitaa,
            adad_bila_basma,
        })
    }

    /// Whether the container declares an encryption key GUID that is not all zero.
    #[must_use]
    pub fn lahu_miftah(&self) -> bool {
        self.muarrif_miftah != [0u8; 16]
    }
}

// ---------------------------------------------------------------------------
// Chunk identity
// ---------------------------------------------------------------------------

/// What a chunk holds — `EIoChunkType`.
///
/// Reported rather than acted on. Taarib finds `.locres` by path through the
/// directory index, not by chunk kind, so an unfamiliar kind is carried through
/// as [`NawJuz::Akhar`] rather than refused: the kind byte does not affect where
/// a chunk's bytes are or how they are decoded, and refusing a container because
/// a newer engine added a kind would be refusing a container this build can read
/// perfectly well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NawJuz {
    /// `Invalid` — zero, which no written chunk uses.
    Batil,
    /// `ExportBundleData` — a cooked package's exports.
    BayanatHuzma,
    /// `BulkData`.
    BayanatDafiqa,
    /// `OptionalBulkData`.
    BayanatDafiqaIkhtiyariya,
    /// `MemoryMappedBulkData`.
    BayanatDafiqaMakhruta,
    /// `ScriptObjects`.
    KainatNass,
    /// `ContainerHeader`.
    TarwisatHawiya,
    /// `ExternalFile` — a loose file carried inside the container, which is how
    /// most non-package assets travel and therefore where a `.locres` is found.
    MalafKharji,
    /// `ShaderCodeLibrary`.
    MaktabatShaders,
    /// `ShaderCode`.
    ShaderCode,
    /// `PackageStoreEntry`.
    MadkhalMakhzan,
    /// `OptionalSegmentPackageData`.
    BayanatMaqtaIkhtiyari,
    /// `OptionalSegmentBulkData`.
    BayanatDafiqaMaqtaIkhtiyari,
    /// A kind this build does not name, carried through unchanged.
    Akhar(u8),
}

impl NawJuz {
    /// The kind a declared byte names.
    #[must_use]
    pub const fn min_raqm(raqm: u8) -> Self {
        match raqm {
            0 => Self::Batil,
            1 => Self::BayanatHuzma,
            2 => Self::BayanatDafiqa,
            3 => Self::BayanatDafiqaIkhtiyariya,
            4 => Self::BayanatDafiqaMakhruta,
            5 => Self::KainatNass,
            6 => Self::TarwisatHawiya,
            7 => Self::MalafKharji,
            8 => Self::MaktabatShaders,
            9 => Self::ShaderCode,
            10 => Self::MadkhalMakhzan,
            11 => Self::BayanatMaqtaIkhtiyari,
            12 => Self::BayanatDafiqaMaqtaIkhtiyari,
            akhar => Self::Akhar(akhar),
        }
    }

    /// The byte the container stores.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Batil => 0,
            Self::BayanatHuzma => 1,
            Self::BayanatDafiqa => 2,
            Self::BayanatDafiqaIkhtiyariya => 3,
            Self::BayanatDafiqaMakhruta => 4,
            Self::KainatNass => 5,
            Self::TarwisatHawiya => 6,
            Self::MalafKharji => 7,
            Self::MaktabatShaders => 8,
            Self::ShaderCode => 9,
            Self::MadkhalMakhzan => 10,
            Self::BayanatMaqtaIkhtiyari => 11,
            Self::BayanatDafiqaMaqtaIkhtiyari => 12,
            Self::Akhar(raqm) => raqm,
        }
    }
}

/// A chunk's identity — `FIoChunkId`.
///
/// ```text
/// MuarrifJuz — 12 bytes
///
///   offset  size  field     type      byte order   meaning
///        0     8  muarrif   u64       little       the chunk id proper
///        8     2  fahras    u16       BIG          the chunk index within the id
///       10     1  hashw     u8        —            padding, written as zero
///       11     1  naw       u8        —            EIoChunkType
/// ```
///
/// The two-byte index is **big-endian** while the eight-byte id beside it is
/// little-endian, because the engine writes it through its network-order helper
/// so that a chunk id sorts the same way on either endianness. Reading it
/// little-endian does not fail: it produces a byte-swapped index that matches no
/// chunk, and a lookup that quietly returns nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MuarrifJuz {
    /// The chunk id proper, little-endian.
    pub muarrif: u64,
    /// The index within that id, stored big-endian.
    pub fahras: u16,
    /// What the chunk holds.
    pub naw: NawJuz,
}

impl MuarrifJuz {
    /// Reads one chunk id out of a twelve-byte window.
    ///
    /// Destructured rather than indexed, so the twelve bytes are named once and
    /// the compiler proves the arithmetic reaches every one of them.
    #[must_use]
    pub const fn min_bayt(khana: [u8; 12]) -> Self {
        let [m0, m1, m2, m3, m4, m5, m6, m7, f0, f1, _hashw, naw] = khana;
        let muarrif = u64::from_le_bytes([m0, m1, m2, m3, m4, m5, m6, m7]);
        let fahras = u16::from_be_bytes([f0, f1]);
        Self {
            muarrif,
            fahras,
            naw: NawJuz::min_raqm(naw),
        }
    }

    /// The twelve bytes the container stores, byte order included.
    #[must_use]
    pub fn ila_bayt(self) -> [u8; 12] {
        let mut khana = [0u8; 12];
        let muarrif = self.muarrif.to_le_bytes();
        let fahras = self.fahras.to_be_bytes();
        if let Some(hadaf) = khana.get_mut(0..8) {
            hadaf.copy_from_slice(&muarrif);
        }
        if let Some(hadaf) = khana.get_mut(8..10) {
            hadaf.copy_from_slice(&fahras);
        }
        if let Some(hadaf) = khana.get_mut(11) {
            *hadaf = self.naw.raqm();
        }
        khana
    }
}

/// Where a chunk lives in the container's uncompressed address space —
/// `FIoOffsetAndLength`.
///
/// ```text
/// IzahaWaTul — 10 bytes, both fields BIG-endian
///
///   offset  size  field   width    meaning
///        0     5  izaha   40 bits  offset into the uncompressed address space
///        5     5  tul     40 bits  how many bytes the chunk occupies there
/// ```
///
/// Two things about this record cost more to get wrong than any other field in
/// the format, and neither one announces itself:
///
/// * **The byte order is big-endian**, unlike every other multi-byte field in
///   the `.utoc`. A little-endian read of `00 00 00 10 00` yields 0x1000000000
///   instead of 0x100000 — an offset four million times too large, which fails
///   loudly. But a *small* offset read the wrong way round yields another small
///   offset, and that one does not fail: it lands inside a neighbouring chunk,
///   decompresses cleanly, and returns somebody else's bytes.
/// * **The width is five bytes, not four or eight.** Forty bits caps a container
///   at one tebibyte, which is deliberate. A reader that assumed eight bytes
///   would consume the next record's offset as the high half of this one's
///   length; a reader that assumed four would walk the array at the wrong
///   stride and every entry after the first would be misaligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IzahaWaTul {
    /// The offset into the container's uncompressed address space.
    pub izaha: u64,
    /// How many bytes the chunk occupies there.
    pub tul: u64,
}

impl IzahaWaTul {
    /// Reads one offset-and-length record out of a ten-byte window.
    ///
    /// Each five-byte big-endian field is widened by prefixing three zero bytes
    /// and reading the result as a big-endian `u64`. That is the whole trick, and
    /// it is written this way rather than as a chain of shifts so that the byte
    /// order is stated once, in the call, where it can be read off and checked.
    #[must_use]
    pub const fn min_bayt(khana: [u8; 10]) -> Self {
        let [a0, a1, a2, a3, a4, t0, t1, t2, t3, t4] = khana;
        let izaha = u64::from_be_bytes([0, 0, 0, a0, a1, a2, a3, a4]);
        let tul = u64::from_be_bytes([0, 0, 0, t0, t1, t2, t3, t4]);
        Self { izaha, tul }
    }

    /// One past the last byte of the chunk, or [`None`] when the sum overflows.
    #[must_use]
    pub const fn nihaya(self) -> Option<u64> {
        self.izaha.checked_add(self.tul)
    }
}

// ---------------------------------------------------------------------------
// The compression block table
// ---------------------------------------------------------------------------

/// One compression block — `FIoStoreTocCompressedBlockEntry`.
///
/// ```text
/// MadkhalKutlatDaght — 12 bytes, bit-packed, little-endian within each field
///
///   byte(s)  bits  field           meaning
///     0..=4    40  izaha           byte offset of the block inside the .ucas
///     5..=7    24  hajm_madghut    how many bytes the block occupies stored
///    8..=10    24  hajm_khaam      how many bytes it expands to
///        11     8  fahras_daght    index into the compression method name table
/// ```
///
/// The engine declares this as `uint8 Data[5 + 3 + 3 + 1]` and reads the fields
/// back through overlapping unaligned word loads with shifts and masks — forty
/// bits of offset out of a `u64` masked with `(1 << 40) - 1`, then two
/// twenty-four bit sizes and a byte, pulled out of two overlapping `u32` loads.
/// That is efficient on x86 and undefined on a target that faults on unaligned
/// loads, so this reader does not imitate it. Every field is assembled from named
/// bytes, and the two masks the engine applies are reproduced by construction:
///
/// * **`izaha`** — bytes 0 to 4, little-endian, widened by appending three zero
///   bytes. Appending them *after* the five is what makes the mask unnecessary:
///   the top twenty-four bits are zero because nothing was ever put there. Forty
///   bits addresses one tebibyte, which is the container's hard ceiling.
/// * **`hajm_madghut`** — bytes 5 to 6 to 7, little-endian, widened with one
///   trailing zero byte. Sixteen mebibytes maximum. This is the number of bytes
///   to read from the `.ucas`, and when the container is encrypted it is the
///   *unpadded* count: the read itself is rounded up to the AES block size, and
///   only these bytes are handed to the decompressor.
/// * **`hajm_khaam`** — bytes 8 to 9 to 10, the same way. This is the size the
///   destination buffer is allocated at, so it is checked against
///   [`AQSA_KUTLA_KHAAM`] *before* the allocation and against the decompressor's
///   own output *after* it. A block that declares one size and produces another
///   is [`KhataUnreal::HajmGhayrMutabaq`], not a short buffer nobody noticed.
/// * **`fahras_daght`** — byte 11. Zero means the block is stored raw; any other
///   value is one past the index of a name in the method table, so name `n` is
///   selected by `fahras_daght == n + 1`.
///
/// Getting the packing wrong is the quiet failure again: shift the size fields by
/// one byte and every block still parses, with an offset that is plausible and a
/// length that is not, and the container reads as corrupt rather than as
/// misparsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadkhalKutlatDaght {
    /// Byte offset of the block inside the `.ucas` address space, across all
    /// partitions.
    pub izaha: u64,
    /// How many bytes the block occupies stored, before any AES padding.
    pub hajm_madghut: u32,
    /// How many bytes the block expands to.
    pub hajm_khaam: u32,
    /// Index into the compression method table; zero means stored raw.
    pub fahras_daght: u8,
}

impl MadkhalKutlatDaght {
    /// Unpacks one block entry out of a twelve-byte window.
    #[must_use]
    pub const fn min_bayt(khana: [u8; 12]) -> Self {
        let [z0, z1, z2, z3, z4, m0, m1, m2, k0, k1, k2, daght] = khana;
        // Five little-endian bytes, then three zeros: the 40-bit mask the engine
        // applies is implicit, because the high bytes are never populated.
        let izaha = u64::from_le_bytes([z0, z1, z2, z3, z4, 0, 0, 0]);
        // Three little-endian bytes, then one zero: the 24-bit mask, likewise.
        let hajm_madghut = u32::from_le_bytes([m0, m1, m2, 0]);
        let hajm_khaam = u32::from_le_bytes([k0, k1, k2, 0]);
        Self {
            izaha,
            hajm_madghut,
            hajm_khaam,
            fahras_daght: daght,
        }
    }

    /// One past the last stored byte, or [`None`] when the sum overflows.
    #[must_use]
    pub fn nihaya(&self) -> Option<u64> {
        self.izaha.checked_add(u64::from(self.hajm_madghut))
    }
}

/// How one block was compressed.
///
/// Named methods only. The container carries the method as a string, so a
/// container built with something else names it and this reader reports the name
/// rather than an index nobody can look up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TareeqatDaght {
    /// Stored raw. Method index zero, or the literal name `None`.
    Bila,
    /// Zlib, expanded with `flate2`.
    Zlib,
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
    /// The method a container's name string selects.
    ///
    /// Matched case-insensitively because the engine's own name table is written
    /// from an `FName`, and an `FName` preserves whatever casing the cook used.
    #[must_use]
    pub fn min_ism(ism: &str) -> Self {
        let mahjuz = ism.trim_matches('\0').trim();
        if mahjuz.is_empty() || mahjuz.eq_ignore_ascii_case("none") {
            Self::Bila
        } else if mahjuz.eq_ignore_ascii_case("zlib") {
            Self::Zlib
        } else if mahjuz.eq_ignore_ascii_case("zstd") || mahjuz.eq_ignore_ascii_case("zstandard") {
            Self::Zstd
        } else if mahjuz.eq_ignore_ascii_case("oodle") {
            Self::Oodle
        } else {
            Self::Majhula(mahjuz.to_owned())
        }
    }

    /// The name this method is written under, for a refusal a person can read.
    #[must_use]
    pub fn ism(&self) -> String {
        match self {
            Self::Bila => "None".to_owned(),
            Self::Zlib => "Zlib".to_owned(),
            Self::Zstd => "Zstd".to_owned(),
            Self::Oodle => "Oodle".to_owned(),
            Self::Majhula(ism) => ism.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// The directory index
// ---------------------------------------------------------------------------

/// One directory in the path tree — `FIoDirectoryIndexEntry`.
///
/// ```text
/// MadkhalDalil — 16 bytes, little-endian
///
///   offset  size  field        type   meaning
///        0     4  ism          u32    index into the string pool, or 0xFFFFFFFF
///        4     4  awwal_ibn    u32    first child directory, or 0xFFFFFFFF
///        8     4  shaqiq       u32    next sibling directory, or 0xFFFFFFFF
///       12     4  awwal_malaf  u32    first file in this directory, or 0xFFFFFFFF
/// ```
///
/// A tree stored as first-child / next-sibling links rather than as a child
/// array, which is why walking it needs a cycle guard: nothing in the file stops
/// a sibling pointer from pointing back at an ancestor, and a reader that
/// followed one would walk forever inside a game process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadkhalDalil {
    /// Index into the string pool, or [`FAHRAS_GHAYR_SALIH`] for the root.
    pub ism: u32,
    /// First child directory, or [`FAHRAS_GHAYR_SALIH`].
    pub awwal_ibn: u32,
    /// Next sibling directory, or [`FAHRAS_GHAYR_SALIH`].
    pub shaqiq: u32,
    /// First file in this directory, or [`FAHRAS_GHAYR_SALIH`].
    pub awwal_malaf: u32,
}

/// One file in the path tree — `FIoFileIndexEntry`.
///
/// ```text
/// MadkhalMalaf — 12 bytes, little-endian
///
///   offset  size  field    type   meaning
///        0     4  ism      u32    index into the string pool
///        4     4  talii    u32    next file in the same directory, or 0xFFFFFFFF
///        8     4  bayanat  u32    the chunk index this path resolves to
/// ```
///
/// `bayanat` is the engine's `UserData`, and in a `.utoc` it is the index into
/// the chunk id and offset/length arrays. That single field is the whole reason
/// this module parses the directory index at all: it is the only mapping from
/// `Content/Localization/Game/en/Game.locres` to a range of the `.ucas`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadkhalMalaf {
    /// Index into the string pool.
    pub ism: u32,
    /// Next file in the same directory, or [`FAHRAS_GHAYR_SALIH`].
    pub talii: u32,
    /// The chunk index this path resolves to, already checked against the `ToC`'s
    /// chunk count.
    pub bayanat: u32,
}

/// Reads one UE `FString` and advances the cursor past it.
///
/// The engine's encoding: a signed thirty-two bit count, then that many
/// characters *including* the terminating null. A positive count means one byte
/// per character; a negative count means two, and the magnitude is the number of
/// UTF-16 code units.
///
/// Both cases are bounded by [`AQSA_TUL_NASS`] before the range is taken, and
/// neither is decoded lossily. A path with a replacement character in it is a
/// path that matches nothing, and a reader that produced one would report "no
/// `.locres` in this container" for a container that has one.
fn iqra_nass(bayt: &[u8], mawqi: &mut usize, haql: &'static str) -> Result<String, KhataUnreal> {
    let adad = iqra_i32(bayt, *mawqi).ok_or_else(|| KhataUnreal::MalafQaseer {
        haql,
        tul: tul_u64(bayt.len()),
        matlub: tul_u64(mawqi.saturating_add(4)),
    })?;
    *mawqi = mawqi.saturating_add(4);
    if adad == 0 {
        return Ok(String::new());
    }

    if adad > 0 {
        let adad = u32::try_from(adad).map_err(|_| talif(haql, 0, u64::from(AQSA_TUL_NASS)))?;
        saqf_adad(adad, AQSA_TUL_NASS, haql)?;
        let tul = mada(adad, 1, haql, u64::from(AQSA_TUL_NASS))?;
        let khana = qass(bayt, *mawqi, tul, haql)?;
        *mawqi = mawqi.saturating_add(tul);
        // The trailing null is part of the count and is not part of the string.
        let bila_sifr = khana.split_last().map_or(khana, |(_, bidaya)| bidaya);
        return String::from_utf8(bila_sifr.to_vec())
            .map_err(|_| talif(haql, tul_u64(bila_sifr.len()), u64::from(AQSA_TUL_NASS)));
    }

    let adad = adad
        .checked_neg()
        .and_then(|adad| u32::try_from(adad).ok())
        .ok_or_else(|| talif(haql, u64::from(AQSA_TUL_NASS), u64::from(AQSA_TUL_NASS)))?;
    saqf_adad(adad, AQSA_TUL_NASS, haql)?;
    let tul = mada(adad, 2, haql, u64::from(AQSA_TUL_NASS))?;
    let khana = qass(bayt, *mawqi, tul, haql)?;
    *mawqi = mawqi.saturating_add(tul);
    let wahdat: Vec<u16> = khana
        .chunks_exact(2)
        .filter_map(|zawj| {
            zawj.first_chunk::<2>()
                .map(|zawj| u16::from_le_bytes(*zawj))
        })
        .collect();
    let bila_sifr = wahdat
        .split_last()
        .map_or(wahdat.as_slice(), |(_, bidaya)| bidaya);
    char::decode_utf16(bila_sifr.iter().copied())
        .collect::<Result<String, _>>()
        .map_err(|_| talif(haql, tul_u64(bila_sifr.len()), u64::from(AQSA_TUL_NASS)))
}

/// Reads a `TArray` count and refuses one above its ceiling.
fn iqra_adad(
    bayt: &[u8],
    mawqi: &mut usize,
    haql: &'static str,
    saqf: u32,
) -> Result<u32, KhataUnreal> {
    let adad = iqra_i32(bayt, *mawqi).ok_or_else(|| KhataUnreal::MalafQaseer {
        haql,
        tul: tul_u64(bayt.len()),
        matlub: tul_u64(mawqi.saturating_add(4)),
    })?;
    *mawqi = mawqi.saturating_add(4);
    let adad = u32::try_from(adad).map_err(|_| talif(haql, u64::MAX, u64::from(saqf)))?;
    saqf_adad(adad, saqf, haql)?;
    Ok(adad)
}

/// The path tree — `FIoDirectoryIndexResource`.
///
/// Parsed once at open, into a map from container-relative path to chunk index.
/// The tree form is kept as well, because it is what a diagnostic dump needs and
/// because rebuilding it would mean parsing the blob twice.
#[derive(Debug, Clone)]
pub struct FahrasDalil {
    /// The mount point the engine registers this container under, as written —
    /// typically `../../../ProjectName/`.
    pub nuqtat_wasl: String,
    /// Every directory, in the order the blob stores them.
    pub madakhil: Vec<MadkhalDalil>,
    /// Every file, in the order the blob stores them.
    pub malafat: Vec<MadkhalMalaf>,
    /// The string pool both arrays index into.
    pub hawd: Vec<String>,
    /// Mount-point-relative path to chunk index, built by walking the tree once.
    pub masarat: BTreeMap<String, u32>,
}

impl FahrasDalil {
    /// Parses a decrypted directory index blob.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] when the blob ends inside a field,
    /// [`KhataUnreal::HajmMufrit`] when an array count is above its ceiling, and
    /// [`KhataUnreal::MawridTalif`] for a string that is not valid, an index that
    /// points outside the array it names, a chunk index past the end of the `ToC`,
    /// or a tree walk that visited more nodes than the tree has — which is how a
    /// cycle in the sibling links is caught rather than followed.
    pub fn min_bayt(bayt: &[u8], adad_ajza: u32) -> Result<Self, KhataUnreal> {
        let mut mawqi = 0usize;
        let nuqtat_wasl = iqra_nass(bayt, &mut mawqi, "the directory index mount point")?;

        let adad_madakhil = iqra_adad(
            bayt,
            &mut mawqi,
            "the directory entry count",
            AQSA_MADAKHIL_DALIL,
        )?;
        let mada_madakhil = mada(
            adad_madakhil,
            HAJM_MADKHAL_DALIL,
            "the directory entry array",
            u64::from(AQSA_MADAKHIL_DALIL),
        )?;
        let khana = qass(bayt, mawqi, mada_madakhil, "the directory entry array")?;
        mawqi = mawqi.saturating_add(mada_madakhil);
        let mut madakhil = Vec::with_capacity(khana.len().saturating_div(HAJM_MADKHAL_DALIL));
        for sijill in khana.chunks_exact(HAJM_MADKHAL_DALIL) {
            madakhil.push(MadkhalDalil {
                ism: iqra_u32(sijill, 0).unwrap_or(FAHRAS_GHAYR_SALIH),
                awwal_ibn: iqra_u32(sijill, 4).unwrap_or(FAHRAS_GHAYR_SALIH),
                shaqiq: iqra_u32(sijill, 8).unwrap_or(FAHRAS_GHAYR_SALIH),
                awwal_malaf: iqra_u32(sijill, 12).unwrap_or(FAHRAS_GHAYR_SALIH),
            });
        }

        let adad_malafat = iqra_adad(bayt, &mut mawqi, "the file entry count", AQSA_MALAFAT_DALIL)?;
        let mada_malafat = mada(
            adad_malafat,
            HAJM_MADKHAL_MALAF,
            "the file entry array",
            u64::from(AQSA_MALAFAT_DALIL),
        )?;
        let khana = qass(bayt, mawqi, mada_malafat, "the file entry array")?;
        mawqi = mawqi.saturating_add(mada_malafat);
        let mut malafat = Vec::with_capacity(khana.len().saturating_div(HAJM_MADKHAL_MALAF));
        for sijill in khana.chunks_exact(HAJM_MADKHAL_MALAF) {
            malafat.push(MadkhalMalaf {
                ism: iqra_u32(sijill, 0).unwrap_or(FAHRAS_GHAYR_SALIH),
                talii: iqra_u32(sijill, 4).unwrap_or(FAHRAS_GHAYR_SALIH),
                bayanat: iqra_u32(sijill, 8).unwrap_or(FAHRAS_GHAYR_SALIH),
            });
        }

        let adad_hawd = iqra_adad(bayt, &mut mawqi, "the string pool count", AQSA_HAWD)?;
        // Reserved in a bounded step rather than at the declared count: the count
        // is already under its ceiling, but a container may still declare eight
        // million strings it does not carry, and reserving for all of them before
        // reading the first would let a forty-byte header claim a gibibyte.
        let mut hawd = Vec::new();
        hawd.try_reserve(usize::try_from(adad_hawd.min(4096)).unwrap_or(0))
            .map_err(|_| KhataUnreal::HajmMufrit {
                haql: "the string pool",
                qeema: u64::from(adad_hawd),
                saqf: u64::from(AQSA_HAWD),
            })?;
        for _ in 0..adad_hawd {
            hawd.push(iqra_nass(bayt, &mut mawqi, "a directory index string")?);
        }

        // Every index in the tree is proven in range here, once, so that the walk
        // below is a lookup rather than a second round of bounds checks — and so
        // that a corrupt index is reported as a corrupt index rather than as a
        // path that silently did not appear.
        let adad_hawd_fili = u32::try_from(hawd.len()).unwrap_or(u32::MAX);
        let adad_madakhil_fili = u32::try_from(madakhil.len()).unwrap_or(u32::MAX);
        let adad_malafat_fili = u32::try_from(malafat.len()).unwrap_or(u32::MAX);
        for madkhal in &madakhil {
            tahaqquq_fahras(madkhal.ism, adad_hawd_fili, "a directory name index")?;
            tahaqquq_fahras(
                madkhal.awwal_ibn,
                adad_madakhil_fili,
                "a child directory index",
            )?;
            tahaqquq_fahras(
                madkhal.shaqiq,
                adad_madakhil_fili,
                "a sibling directory index",
            )?;
            tahaqquq_fahras(madkhal.awwal_malaf, adad_malafat_fili, "a first file index")?;
        }
        for madkhal in &malafat {
            tahaqquq_fahras(madkhal.ism, adad_hawd_fili, "a file name index")?;
            tahaqquq_fahras(madkhal.talii, adad_malafat_fili, "a next file index")?;
            if madkhal.bayanat >= adad_ajza {
                return Err(talif(
                    "a file entry's chunk index",
                    u64::from(madkhal.bayanat),
                    u64::from(adad_ajza),
                ));
            }
        }

        let masarat = imshi(&madakhil, &malafat, &hawd)?;
        Ok(Self {
            nuqtat_wasl,
            madakhil,
            malafat,
            hawd,
            masarat,
        })
    }

    /// The chunk index a container-relative path resolves to.
    #[must_use]
    pub fn jid(&self, masar: &str) -> Option<u32> {
        self.masarat.get(masar).copied()
    }

    /// Every path in the container, with the chunk index it resolves to.
    pub fn kull(&self) -> impl Iterator<Item = (&str, u32)> + '_ {
        self.masarat
            .iter()
            .map(|(masar, fahras)| (masar.as_str(), *fahras))
    }
}

/// Refuses an index that is neither the sentinel nor inside its array.
fn tahaqquq_fahras(fahras: u32, adad: u32, haql: &'static str) -> Result<(), KhataUnreal> {
    if fahras != FAHRAS_GHAYR_SALIH && fahras >= adad {
        return Err(talif(haql, u64::from(fahras), u64::from(adad)));
    }
    Ok(())
}

/// The refusal a tree walk that exceeded its step budget produces.
fn dawra(khatawat: usize, mizaniya: usize) -> KhataUnreal {
    talif(
        "the directory tree walk",
        tul_u64(khatawat),
        tul_u64(mizaniya),
    )
}

/// Walks the first-child / next-sibling tree into a flat path map.
///
/// An explicit stack rather than recursion, because the depth is a property of a
/// file a stranger wrote and a recursive walk would answer a deep tree with a
/// stack overflow inside somebody's game — which is not a refusal, it is a crash.
///
/// The iteration budget is the cycle guard. Every directory and every file is
/// visited at most once in a well-formed tree, so a walk that takes more steps
/// than there are nodes has followed a link that points backwards, and is
/// refused rather than followed.
fn imshi(
    madakhil: &[MadkhalDalil],
    malafat: &[MadkhalMalaf],
    hawd: &[String],
) -> Result<BTreeMap<String, u32>, KhataUnreal> {
    let mut masarat = BTreeMap::new();
    if madakhil.is_empty() {
        return Ok(masarat);
    }
    let mizaniya = madakhil
        .len()
        .saturating_add(malafat.len())
        .saturating_mul(2)
        .saturating_add(16);
    let mut khatawat = 0usize;
    let mut makdas: Vec<(u32, String)> = vec![(0, String::new())];

    while let Some((dalil, sabiq)) = makdas.pop() {
        khatawat = khatawat.saturating_add(1);
        if khatawat > mizaniya {
            return Err(dawra(khatawat, mizaniya));
        }
        let Some(madkhal) = usize::try_from(dalil).ok().and_then(|i| madakhil.get(i)) else {
            continue;
        };

        let mut malaf = madkhal.awwal_malaf;
        while malaf != FAHRAS_GHAYR_SALIH {
            khatawat = khatawat.saturating_add(1);
            if khatawat > mizaniya {
                return Err(dawra(khatawat, mizaniya));
            }
            let Some(sijill) = usize::try_from(malaf).ok().and_then(|i| malafat.get(i)) else {
                break;
            };
            let ism = usize::try_from(sijill.ism).ok().and_then(|i| hawd.get(i));
            if let Some(ism) = ism {
                let mut masar = String::with_capacity(sabiq.len().saturating_add(ism.len()));
                masar.push_str(&sabiq);
                masar.push_str(ism);
                if masar.len() <= AQSA_TUL_MASAR {
                    let _ = masarat.insert(masar, sijill.bayanat);
                }
            }
            malaf = sijill.talii;
        }

        let mut ibn = madkhal.awwal_ibn;
        while ibn != FAHRAS_GHAYR_SALIH {
            khatawat = khatawat.saturating_add(1);
            if khatawat > mizaniya {
                return Err(dawra(khatawat, mizaniya));
            }
            let Some(sijill) = usize::try_from(ibn).ok().and_then(|i| madakhil.get(i)) else {
                break;
            };
            if let Some(ism) = usize::try_from(sijill.ism).ok().and_then(|i| hawd.get(i)) {
                let mut masar = String::with_capacity(sabiq.len().saturating_add(ism.len() + 1));
                masar.push_str(&sabiq);
                masar.push_str(ism);
                masar.push('/');
                if masar.len() <= AQSA_TUL_MASAR {
                    makdas.push((ibn, masar));
                }
            } else {
                makdas.push((ibn, sabiq.clone()));
            }
            ibn = sijill.shaqiq;
        }
    }
    Ok(masarat)
}

// ---------------------------------------------------------------------------
// Encryption
// ---------------------------------------------------------------------------

/// Decrypts a buffer in place with AES-256 in ECB mode.
///
/// ECB is the engine's choice, not this reader's, and it follows from what the
/// container has to do: a `.ucas` is seeked into at block boundaries, so a mode
/// that chained blocks would make reading block ten require reading blocks zero
/// through nine. Taarib reads what Unreal wrote.
///
/// The buffer must be a whole number of sixteen-byte blocks. Callers round the
/// read up before decrypting; a trailing partial block is left untouched rather
/// than padded, because inventing padding would turn a truncated read into
/// plausible-looking plaintext.
fn fukk_tashfeer(bayt: &mut [u8], miftah: &[u8; 32], masar: &Path) -> Result<(), KhataUnreal> {
    let shifra = Aes256::new_from_slice(miftah).map_err(|_| KhataUnreal::MiftahGhayrSalih {
        masar: masar.to_path_buf(),
        sabab: "an AES-256 key must be exactly 32 bytes",
    })?;
    for kutla in bayt.chunks_exact_mut(HAJM_KUTLAT_TASHFEER) {
        shifra.decrypt_block(GenericArray::from_mut_slice(kutla));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Decompression
// ---------------------------------------------------------------------------

/// Expands one compression block into a buffer of exactly the size it declared.
///
/// The bound is in two halves, and both are needed:
///
/// * the **declared** size is checked against [`AQSA_KUTLA_KHAAM`] before the
///   destination exists, so a block claiming a gibibyte never reaches the
///   allocator;
/// * the **produced** size is checked against the declaration afterwards, so a
///   frame that expands to less than it promised is refused rather than leaving
///   the tail of the destination as zeros that the caller then reads as data.
///
/// Decompressing into a fixed destination rather than a growing vector is what
/// makes the first half meaningful. A zstd or zlib stream is free to claim one
/// size in its header and expand to another; a fixed destination turns the
/// container's declaration into the ceiling instead of a suggestion.
///
/// # Errors
///
/// [`KhataUnreal::HajmMufrit`] when the declared size is above the ceiling,
/// [`KhataUnreal::DaghtMajhul`] for Oodle and for any method this build does not
/// implement, [`KhataUnreal::FakkFashil`] when the decompressor reports a
/// failure, and [`KhataUnreal::HajmGhayrMutabaq`] when the output size does not
/// match the declaration.
fn fukk_daght(
    tareeqa: &TareeqatDaght,
    makhzun: &[u8],
    hajm_khaam: u32,
) -> Result<Vec<u8>, KhataUnreal> {
    if hajm_khaam > AQSA_KUTLA_KHAAM {
        return Err(KhataUnreal::HajmMufrit {
            haql: "a compression block's uncompressed size",
            qeema: u64::from(hajm_khaam),
            saqf: u64::from(AQSA_KUTLA_KHAAM),
        });
    }
    let siaa = hajm_usize(u64::from(hajm_khaam)).ok_or_else(|| KhataUnreal::HajmMufrit {
        haql: "a compression block's uncompressed size",
        qeema: u64::from(hajm_khaam),
        saqf: u64::from(AQSA_KUTLA_KHAAM),
    })?;

    match tareeqa {
        TareeqatDaght::Bila => {
            // A raw block still has to match its declaration: the stored length
            // and the expanded length are two independent fields, and a block
            // where they disagree is a block the reader and the engine would
            // slice differently.
            if makhzun.len() != siaa {
                return Err(KhataUnreal::HajmGhayrMutabaq {
                    ism: ISM,
                    muallan: u64::from(hajm_khaam),
                    fili: tul_u64(makhzun.len()),
                });
            }
            Ok(makhzun.to_vec())
        },
        TareeqatDaght::Zstd => {
            let mut mafkuk = vec![0u8; siaa];
            let fili = zstd::bulk::decompress_to_buffer(makhzun, &mut mafkuk).map_err(|khata| {
                KhataUnreal::FakkFashil {
                    ism: ISM,
                    tafsil: khata.to_string(),
                }
            })?;
            if fili != siaa {
                return Err(KhataUnreal::HajmGhayrMutabaq {
                    ism: ISM,
                    muallan: u64::from(hajm_khaam),
                    fili: tul_u64(fili),
                });
            }
            Ok(mafkuk)
        },
        TareeqatDaght::Zlib => {
            let mut mafkuk = vec![0u8; siaa];
            let mut jihaz = flate2::Decompress::new(true);
            jihaz
                .decompress(makhzun, &mut mafkuk, flate2::FlushDecompress::Finish)
                .map_err(|khata| KhataUnreal::FakkFashil {
                    ism: ISM,
                    tafsil: khata.to_string(),
                })?;
            let fili = jihaz.total_out();
            if fili != u64::from(hajm_khaam) {
                return Err(KhataUnreal::HajmGhayrMutabaq {
                    ism: ISM,
                    muallan: u64::from(hajm_khaam),
                    fili,
                });
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
// The table of contents
// ---------------------------------------------------------------------------

/// A parsed and validated `.utoc`.
///
/// The `.utoc` is read whole — it is small, and every number in it has to be
/// judged against the real length of the same slice, which is only possible if
/// the whole slice is present. Nothing here holds a borrow: the arrays are owned
/// so that the caller may drop the file bytes and keep the table, which is what
/// the extractor does while it works through the `.ucas`.
///
/// The regions, in the order the format lays them out after the header:
///
/// ```text
///   region                                   present when
///   chunk id array                           always,  adad_ajza * 12
///   offset and length array                  always,  adad_ajza * 10
///   perfect-hash seed array                  v4+,     adad_bidhur * 4
///   chunk indices without a perfect hash     v5+,     adad_bila_basma * 4
///   compression block array                  always,  adad_kutal * 12
///   compression method name table            always,  adad_asma * tul_ism
///   signature block                          ALAM_MUWAQQA
///   directory index blob                     v2+ and ALAM_MUFAHRAS
///   per-chunk metadata                       the remainder
/// ```
#[derive(Debug, Clone)]
pub struct JadwalMuhtawayat {
    tarwisa: TarwisatIoStore,
    ajza: Vec<MuarrifJuz>,
    mawaqi: Vec<IzahaWaTul>,
    bidhur: Vec<i32>,
    bila_basma: Vec<i32>,
    kutal: Vec<MadkhalKutlatDaght>,
    turuq: Vec<TareeqatDaght>,
    dalil: Option<FahrasDalil>,
    bayan: Vec<u8>,
    khatwat_bayan: usize,
}

impl JadwalMuhtawayat {
    /// Reads a whole `.utoc`.
    ///
    /// `miftah` is the container's AES-256 key, needed only when the container
    /// declares [`ALAM_MUSHAFFAR`]. It is demanded up front rather than at the
    /// first encrypted read, because every path forward from an encrypted
    /// container — the directory index, and every chunk in it — needs it, and a
    /// reader that opened successfully and then refused every read would be
    /// reporting the same problem once per file instead of once per container.
    ///
    /// # Errors
    ///
    /// Whatever [`TarwisatIoStore::min_bayt`] refuses, plus
    /// [`KhataUnreal::PakMushaffar`] when the container is encrypted and no key
    /// was supplied, [`KhataUnreal::MiftahGhayrSalih`] when the supplied key does
    /// not produce a directory index that parses,
    /// [`KhataUnreal::MalafQaseer`] when a region runs past the end of the file,
    /// [`KhataUnreal::HajmMufrit`] when a count is above its ceiling, and
    /// [`KhataUnreal::MawridTalif`] for anything inside the directory index that
    /// is not consistent with itself.
    pub fn min_bayt(
        bayt: &[u8],
        masar: &Path,
        miftah: Option<&[u8; 32]>,
    ) -> Result<Self, KhataUnreal> {
        let tarwisa = TarwisatIoStore::min_bayt(bayt, masar)?;
        if tarwisa.alam.mushaffar() && miftah.is_none() {
            return Err(KhataUnreal::PakMushaffar {
                masar: masar.to_path_buf(),
            });
        }

        let mut mawqi = HAJM_TARWISA;

        // 1. The chunk id array.
        let span = mada(
            tarwisa.adad_ajza,
            HAJM_MUARRIF_JUZ,
            "the chunk id array",
            u64::from(AQSA_AJZA),
        )?;
        let khana = qass(bayt, mawqi, span, "the chunk id array")?;
        mawqi = mawqi.saturating_add(span);
        let ajza: Vec<MuarrifJuz> = khana
            .chunks_exact(HAJM_MUARRIF_JUZ)
            .filter_map(|sijill| sijill.first_chunk::<12>().map(|q| MuarrifJuz::min_bayt(*q)))
            .collect();

        // 2. The offset and length array.
        let span = mada(
            tarwisa.adad_ajza,
            HAJM_IZAHA_WA_TUL,
            "the offset and length array",
            u64::from(AQSA_AJZA),
        )?;
        let khana = qass(bayt, mawqi, span, "the offset and length array")?;
        mawqi = mawqi.saturating_add(span);
        let mawaqi: Vec<IzahaWaTul> = khana
            .chunks_exact(HAJM_IZAHA_WA_TUL)
            .filter_map(|sijill| sijill.first_chunk::<10>().map(|q| IzahaWaTul::min_bayt(*q)))
            .collect();

        // 3. The perfect-hash seed array, from version 4. Skipped by exactly its
        //    own declared size: this array is not used for lookup here — the
        //    directory index is — but a byte miscounted here moves every region
        //    after it, and the compression block table would then be read out of
        //    the middle of the seeds.
        let span = mada(
            tarwisa.adad_bidhur,
            HAJM_BADHRA,
            "the perfect-hash seed array",
            u64::from(AQSA_AJZA),
        )?;
        let khana = qass(bayt, mawqi, span, "the perfect-hash seed array")?;
        mawqi = mawqi.saturating_add(span);
        let bidhur: Vec<i32> = khana
            .chunks_exact(HAJM_BADHRA)
            .filter_map(|sijill| sijill.first_chunk::<4>().map(|q| i32::from_le_bytes(*q)))
            .collect();

        // 4. The chunk indices the perfect hash could not place, from version 5.
        let span = mada(
            tarwisa.adad_bila_basma,
            HAJM_BADHRA,
            "the perfect-hash overflow array",
            u64::from(AQSA_AJZA),
        )?;
        let khana = qass(bayt, mawqi, span, "the perfect-hash overflow array")?;
        mawqi = mawqi.saturating_add(span);
        let bila_basma: Vec<i32> = khana
            .chunks_exact(HAJM_BADHRA)
            .filter_map(|sijill| sijill.first_chunk::<4>().map(|q| i32::from_le_bytes(*q)))
            .collect();

        // 5. The compression block table.
        let span = mada(
            tarwisa.adad_kutal,
            HAJM_MADKHAL_KUTLA,
            "the compression block array",
            u64::from(AQSA_KUTAL),
        )?;
        let khana = qass(bayt, mawqi, span, "the compression block array")?;
        mawqi = mawqi.saturating_add(span);
        let kutal: Vec<MadkhalKutlatDaght> = khana
            .chunks_exact(HAJM_MADKHAL_KUTLA)
            .filter_map(|sijill| {
                sijill
                    .first_chunk::<12>()
                    .map(|q| MadkhalKutlatDaght::min_bayt(*q))
            })
            .collect();

        // 6. The compression method name table: fixed-width, null-padded names.
        //    Fixed width rather than length-prefixed, so a name that fills its
        //    field has no terminator and must be trimmed rather than read to a
        //    null that is not there.
        let tul_ism = hajm_usize(u64::from(tarwisa.tul_ism_daght)).unwrap_or(0);
        let span = mada(
            tarwisa.adad_asma_daght,
            tul_ism,
            "the compression method table",
            u64::from(AQSA_ASMA_DAGHT),
        )?;
        let khana = qass(bayt, mawqi, span, "the compression method table")?;
        mawqi = mawqi.saturating_add(span);
        let siaa = usize::try_from(tarwisa.adad_asma_daght.min(AQSA_ASMA_DAGHT)).unwrap_or(0);
        let mut turuq = Vec::with_capacity(siaa);
        if tul_ism > 0 {
            for sijill in khana.chunks_exact(tul_ism) {
                let ism = String::from_utf8_lossy(sijill);
                turuq.push(TareeqatDaght::min_ism(ism.as_ref()));
            }
        }

        // 7. The signature block, when the container is signed. Its size depends
        //    on a digest width the block itself declares, so the width is bounded
        //    before it is used to skip anything.
        if tarwisa.alam.muwaqqa() {
            let hajm_basma = iqra_u32(bayt, mawqi).ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "the signature block",
                tul: tul_u64(bayt.len()),
                matlub: tul_u64(mawqi.saturating_add(4)),
            })?;
            saqf_adad(hajm_basma, AQSA_HAJM_BASMA, "the signature digest size")?;
            if hajm_basma == 0 {
                return Err(talif(
                    "the signature digest size",
                    0,
                    u64::from(AQSA_HAJM_BASMA),
                ));
            }
            mawqi = mawqi.saturating_add(4);
            let tawqee = hajm_usize(u64::from(hajm_basma)).unwrap_or(usize::MAX);
            let ithnan = tawqee
                .checked_mul(2)
                .ok_or_else(|| KhataUnreal::HajmMufrit {
                    haql: "the signature block",
                    qeema: u64::from(hajm_basma),
                    saqf: u64::from(AQSA_HAJM_BASMA),
                })?;
            let _ = qass(bayt, mawqi, ithnan, "the signature block")?;
            mawqi = mawqi.saturating_add(ithnan);
            let span = mada(
                tarwisa.adad_kutal,
                HAJM_TAWQEE_KUTLA,
                "the per-block signature array",
                u64::from(AQSA_KUTAL),
            )?;
            let _ = qass(bayt, mawqi, span, "the per-block signature array")?;
            mawqi = mawqi.saturating_add(span);
        }

        // 8. The directory index, decrypted where the container is encrypted.
        let dalil = if tarwisa.hajm_fahras_dalil > 0 {
            let span = hajm_usize(u64::from(tarwisa.hajm_fahras_dalil)).ok_or_else(|| {
                KhataUnreal::HajmMufrit {
                    haql: "the directory index",
                    qeema: u64::from(tarwisa.hajm_fahras_dalil),
                    saqf: u64::from(AQSA_FAHRAS_DALIL),
                }
            })?;
            let khana = qass(bayt, mawqi, span, "the directory index")?;
            mawqi = mawqi.saturating_add(span);
            let mut mafkuk = khana.to_vec();
            if tarwisa.alam.mushaffar() {
                let miftah = miftah.ok_or_else(|| KhataUnreal::PakMushaffar {
                    masar: masar.to_path_buf(),
                })?;
                fukk_tashfeer(&mut mafkuk, miftah, masar)?;
            }
            // A directory index that does not parse after decryption is almost
            // always the wrong key: the first field is a length-prefixed string,
            // and the odds that random plaintext produces a plausible one are
            // small. It is reported as a key failure only when the container was
            // encrypted, because for a plaintext container the same refusal means
            // exactly what it says.
            match FahrasDalil::min_bayt(&mafkuk, tarwisa.adad_ajza) {
                Ok(dalil) => Some(dalil),
                Err(khata) => {
                    if tarwisa.alam.mushaffar() {
                        return Err(KhataUnreal::MiftahGhayrSalih {
                            masar: masar.to_path_buf(),
                            sabab: "the directory index did not decrypt into a readable path tree",
                        });
                    }
                    return Err(khata);
                },
            }
        } else {
            None
        };

        // 9. The trailing per-chunk metadata. Nothing this module needs follows
        //    it, so a tail that matches neither version's stride is carried as
        //    empty rather than refused — the leniency is confined to a region no
        //    offset is ever computed from, and every earlier region was measured
        //    exactly.
        let khatwat_bayan = tarwisa.isdar.khatwat_bayan();
        let mutawaqqa = mada(
            tarwisa.adad_ajza,
            khatwat_bayan,
            "the per-chunk metadata",
            u64::from(AQSA_AJZA),
        )?;
        let bayan = bayt
            .get(mawqi..)
            .filter(|baqi| baqi.len() == mutawaqqa)
            .map(<[u8]>::to_vec)
            .unwrap_or_default();

        Ok(Self {
            tarwisa,
            ajza,
            mawaqi,
            bidhur,
            bila_basma,
            kutal,
            turuq,
            dalil,
            bayan,
            khatwat_bayan,
        })
    }

    /// The validated header.
    #[must_use]
    pub const fn tarwisa(&self) -> &TarwisatIoStore {
        &self.tarwisa
    }

    /// Every chunk id, in `ToC` order.
    #[must_use]
    pub fn ajza(&self) -> &[MuarrifJuz] {
        &self.ajza
    }

    /// Every chunk's offset and length, in `ToC` order.
    #[must_use]
    pub fn mawaqi(&self) -> &[IzahaWaTul] {
        &self.mawaqi
    }

    /// The perfect-hash seeds, empty below version 4.
    #[must_use]
    pub fn bidhur(&self) -> &[i32] {
        &self.bidhur
    }

    /// The chunk indices the perfect hash could not place, empty below version 5.
    #[must_use]
    pub fn bila_basma(&self) -> &[i32] {
        &self.bila_basma
    }

    /// Every compression block, in `ToC` order.
    #[must_use]
    pub fn kutal(&self) -> &[MadkhalKutlatDaght] {
        &self.kutal
    }

    /// The compression methods the container names, in table order.
    #[must_use]
    pub fn turuq(&self) -> &[TareeqatDaght] {
        &self.turuq
    }

    /// The directory index, when the container carries one.
    ///
    /// [`None`] for a version 1 container, and for any container built without
    /// [`ALAM_MUFAHRAS`]. Such a container is readable by chunk id and not by
    /// path, which for this adapter means the `.locres` inside it cannot be
    /// found by name — the caller reports that rather than guessing at ids.
    #[must_use]
    pub const fn dalil(&self) -> Option<&FahrasDalil> {
        self.dalil.as_ref()
    }

    /// The recorded metadata hash for one chunk, when the tail was present.
    ///
    /// Returned as raw bytes because its construction changes with the version —
    /// a thirty-two byte `FIoChunkHash` up to version seven, a twenty-byte
    /// `FIoHash` from version eight — and a reader that decided which algorithm
    /// produced it would be guessing at exactly the kind of thing this module
    /// refuses to guess at. Compare it with [`Self::tahaqquq_basma`], from a
    /// caller that knows.
    #[must_use]
    pub fn bayan(&self, fahras: usize) -> Option<&[u8]> {
        let bidaya = fahras.checked_mul(self.khatwat_bayan)?;
        let nihaya = bidaya.checked_add(self.khatwat_bayan.checked_sub(1)?)?;
        self.bayan.get(bidaya..nihaya)
    }

    /// Compares a caller-computed digest against the one the `ToC` recorded.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::BasmaGhayrMutabaqa`] when they differ. A chunk whose bytes
    /// do not match the hash the container recorded is a chunk the game files
    /// have been modified under, and reading it as source text would put somebody
    /// else's edit into a translation.
    pub fn tahaqquq_basma(
        &self,
        fahras: usize,
        madkhal: &str,
        mahsuba: &[u8],
    ) -> Result<(), KhataUnreal> {
        let Some(musajjala) = self.bayan(fahras) else {
            return Ok(());
        };
        if musajjala == mahsuba {
            Ok(())
        } else {
            Err(KhataUnreal::BasmaGhayrMutabaqa {
                ism: ISM,
                madkhal: madkhal.to_owned(),
            })
        }
    }

    /// The compression method one block entry selects.
    ///
    /// Index zero is "stored raw" and every other index is one past a name in the
    /// table, which is the engine's convention and not an off-by-one. An index
    /// past the end of the table is reported as an unexpandable method rather
    /// than silently treated as raw: a block that names a method the container
    /// did not declare is a container whose method table was truncated, and
    /// treating it as raw would hand compressed bytes to the caller as text.
    #[must_use]
    pub fn tareeqa(&self, kutla: &MadkhalKutlatDaght) -> TareeqatDaght {
        if kutla.fahras_daght == 0 {
            return TareeqatDaght::Bila;
        }
        usize::from(kutla.fahras_daght)
            .checked_sub(1)
            .and_then(|fahras| self.turuq.get(fahras))
            .cloned()
            .unwrap_or_else(|| {
                TareeqatDaght::Majhula(format!("method index {}", kutla.fahras_daght))
            })
    }
}

// ---------------------------------------------------------------------------
// Reaching the .ucas
// ---------------------------------------------------------------------------

/// A source the `.ucas` can be read from, one bounded range at a time.
///
/// The `.utoc` is handed to this module as a `&[u8]` because it is small enough
/// to hold whole. The `.ucas` is not: a shipping container is tens of gigabytes,
/// and a reader that required it in memory would be unusable in the one process
/// that matters, which is somebody's game. So the blob is behind this trait, and
/// every read through it is a range the table of contents already authorised.
///
/// Implementors are `Debug` because the workspace denies types that are not, and
/// because a container that failed to read should be able to say what it was
/// reading from.
pub trait QariNitaq: std::fmt::Debug {
    /// How many partitions this source can serve.
    fn adad_qitaat(&self) -> u32;

    /// Fills `hadaf` from `izaha` within one partition.
    ///
    /// The destination is supplied by the caller, so the amount of memory a read
    /// takes is decided by code that has already bounded it, not by a length
    /// field inside the container.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] when the underlying source fails, and
    /// [`KhataUnreal::MalafQaseer`] when the range runs past the end of the
    /// partition — which for a `.ucas` means the `.utoc` beside it describes a
    /// file that is not the one on disk.
    fn iqra_nitaq(&self, qitaa: u32, izaha: u64, hadaf: &mut [u8]) -> Result<(), KhataUnreal>;
}

/// The `.ucas` path for one partition of a container.
///
/// Partition zero is the container's own name; partition `n` is that name with
/// `_s{n}` appended before the extension, which is the engine's own scheme.
#[must_use]
pub fn masar_qitaa(masar_utoc: &Path, qitaa: u32) -> PathBuf {
    let mut masar = masar_utoc.to_path_buf();
    if qitaa == 0 {
        let _ = masar.set_extension("ucas");
        return masar;
    }
    let jidhr = masar_utoc
        .file_stem()
        .map_or_else(String::new, |ism| ism.to_string_lossy().into_owned());
    masar.set_file_name(format!("{jidhr}_s{qitaa}.ucas"));
    masar
}

/// A `.ucas` reached through one memory mapping per partition.
///
/// # Safety of the mappings
///
/// A memory map is only as stable as the file behind it: if another process
/// truncates a `.ucas` while this value is alive, touching the mapped pages past
/// the new end is undefined behaviour, and no bounds check in this module can
/// prevent it. Taarib reads a game's own installed files and never writes to
/// them, and this type is safe under that discipline. It is not safe against an
/// adversary who can rewrite the game directory while the extractor runs — who,
/// having write access to the game directory, has already won.
#[derive(Debug)]
pub struct MalafatUcas {
    masarat: Vec<PathBuf>,
    kharait: Vec<memmap2::Mmap>,
}

impl MalafatUcas {
    /// Opens and maps every partition of a container.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] naming the partition that could not be opened
    /// or mapped. A missing `_s1.ucas` beside a `.utoc` that declares two
    /// partitions is a broken installation, and is reported as one rather than
    /// producing a container that reads correctly until the first chunk that
    /// happens to live past the first partition boundary.
    pub fn iftah(masar_utoc: &Path, adad_qitaat: u32) -> Result<Self, KhataUnreal> {
        let mut masarat = Vec::new();
        let mut kharait = Vec::new();
        for qitaa in 0..adad_qitaat.max(1) {
            let masar = masar_qitaa(masar_utoc, qitaa);
            let malaf = std::fs::File::open(&masar).map_err(|sabab| KhataUnreal::KhataMalaf {
                masar: masar.clone(),
                sabab,
            })?;
            // SAFETY: see this type's documentation. The mapping is read-only and
            // the file is one this product opens and never writes.
            let khareeta =
                unsafe { memmap2::Mmap::map(&malaf) }.map_err(|sabab| KhataUnreal::KhataMalaf {
                    masar: masar.clone(),
                    sabab,
                })?;
            masarat.push(masar);
            kharait.push(khareeta);
        }
        Ok(Self { masarat, kharait })
    }

    /// The partition files this source was opened over, in partition order.
    #[must_use]
    pub fn masarat(&self) -> &[PathBuf] {
        &self.masarat
    }
}

impl QariNitaq for MalafatUcas {
    fn adad_qitaat(&self) -> u32 {
        u32::try_from(self.kharait.len()).unwrap_or(u32::MAX)
    }

    fn iqra_nitaq(&self, qitaa: u32, izaha: u64, hadaf: &mut [u8]) -> Result<(), KhataUnreal> {
        let fahras = hajm_usize(u64::from(qitaa)).unwrap_or(usize::MAX);
        let khareeta = self
            .kharait
            .get(fahras)
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a .ucas partition that the .utoc names",
                tul: tul_u64(self.kharait.len()),
                matlub: u64::from(qitaa).saturating_add(1),
            })?;
        let bidaya = hajm_usize(izaha).ok_or_else(|| KhataUnreal::MalafQaseer {
            haql: "a .ucas block offset",
            tul: tul_u64(khareeta.len()),
            matlub: izaha,
        })?;
        let nihaya = bidaya
            .checked_add(hadaf.len())
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a .ucas block",
                tul: tul_u64(khareeta.len()),
                matlub: u64::MAX,
            })?;
        let khana = khareeta
            .get(bidaya..nihaya)
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a .ucas block",
                tul: tul_u64(khareeta.len()),
                matlub: tul_u64(nihaya),
            })?;
        hadaf.copy_from_slice(khana);
        Ok(())
    }
}

/// A `.ucas` already in memory, one slice per partition.
///
/// The in-process form: what a test uses, and what a caller that obtained the
/// blob some other way passes. It exists so that there is exactly one chunk
/// reading implementation rather than one for mapped files and one for buffers —
/// two would be two sets of bounds checks, and the one exercised less often
/// would be the one that was wrong.
#[derive(Debug, Clone, Copy)]
pub struct BaytUcas<'a> {
    qitaat: &'a [&'a [u8]],
}

impl<'a> BaytUcas<'a> {
    /// Wraps one slice per partition, in partition order.
    #[must_use]
    pub const fn jadid(qitaat: &'a [&'a [u8]]) -> Self {
        Self { qitaat }
    }
}

impl QariNitaq for BaytUcas<'_> {
    fn adad_qitaat(&self) -> u32 {
        u32::try_from(self.qitaat.len()).unwrap_or(u32::MAX)
    }

    fn iqra_nitaq(&self, qitaa: u32, izaha: u64, hadaf: &mut [u8]) -> Result<(), KhataUnreal> {
        let fahras = hajm_usize(u64::from(qitaa)).unwrap_or(usize::MAX);
        let khana = self
            .qitaat
            .get(fahras)
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a .ucas partition that the .utoc names",
                tul: tul_u64(self.qitaat.len()),
                matlub: u64::from(qitaa).saturating_add(1),
            })?;
        let bidaya = hajm_usize(izaha).ok_or_else(|| KhataUnreal::MalafQaseer {
            haql: "a .ucas block offset",
            tul: tul_u64(khana.len()),
            matlub: izaha,
        })?;
        let nihaya = bidaya
            .checked_add(hadaf.len())
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a .ucas block",
                tul: tul_u64(khana.len()),
                matlub: u64::MAX,
            })?;
        let nitaq = khana
            .get(bidaya..nihaya)
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a .ucas block",
                tul: tul_u64(khana.len()),
                matlub: tul_u64(nihaya),
            })?;
        hadaf.copy_from_slice(nitaq);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The container
// ---------------------------------------------------------------------------

/// A `.utoc` and the `.ucas` it describes, read together.
///
/// Holding one of these is the proof that the table of contents parsed, that
/// every offset in it lies inside the file it was read from, and that the key —
/// if the container needed one — produced a directory index that reads as a path
/// tree. Nothing here hands out bytes before that is true.
#[derive(Debug)]
pub struct HawiyatIoStore<Q: QariNitaq> {
    masar: PathBuf,
    jadwal: JadwalMuhtawayat,
    masdar: Q,
    miftah: Option<[u8; 32]>,
}

impl HawiyatIoStore<MalafatUcas> {
    /// Opens a container from a `.utoc` path, mapping the `.ucas` beside it.
    ///
    /// **This is the one function in this module that touches the filesystem.**
    /// Everything else works over a `&[u8]` for the table of contents and over a
    /// [`QariNitaq`] for the blob, so that the parsing can be exercised without a
    /// game installed and so that a caller with the bytes already in hand is not
    /// forced to write them to a temporary file first.
    ///
    /// The `.utoc` is read whole, because it is small and because every offset in
    /// it must be judged against the real length of the same buffer. The `.ucas`
    /// is mapped, one mapping per partition, and read by range.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] when the `.utoc` or any `.ucas` partition
    /// cannot be read or mapped, and otherwise whatever
    /// [`JadwalMuhtawayat::min_bayt`] refuses.
    pub fn iftah(masar_utoc: &Path, miftah: Option<[u8; 32]>) -> Result<Self, KhataUnreal> {
        let bayt = std::fs::read(masar_utoc).map_err(|sabab| KhataUnreal::KhataMalaf {
            masar: masar_utoc.to_path_buf(),
            sabab,
        })?;
        let jadwal = JadwalMuhtawayat::min_bayt(&bayt, masar_utoc, miftah.as_ref())?;
        let masdar = MalafatUcas::iftah(masar_utoc, jadwal.tarwisa().adad_qitaat)?;
        Ok(Self {
            masar: masar_utoc.to_path_buf(),
            jadwal,
            masdar,
            miftah,
        })
    }
}

impl<Q: QariNitaq> HawiyatIoStore<Q> {
    /// Pairs an already-parsed table of contents with a source for its blob.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] when the source serves fewer partitions than
    /// the table of contents declares. Refused here rather than at the first read
    /// that crosses a partition boundary, because a container that reads for a
    /// while and then stops is harder to diagnose than one that never opened.
    pub fn jadid(
        masar: PathBuf,
        jadwal: JadwalMuhtawayat,
        masdar: Q,
        miftah: Option<[u8; 32]>,
    ) -> Result<Self, KhataUnreal> {
        let matlub = jadwal.tarwisa().adad_qitaat;
        let mutah = masdar.adad_qitaat();
        if mutah < matlub {
            return Err(talif(
                "the .ucas partition count",
                u64::from(mutah),
                u64::from(matlub),
            ));
        }
        Ok(Self {
            masar,
            jadwal,
            masdar,
            miftah,
        })
    }

    /// The `.utoc` path this container was opened from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// The parsed table of contents.
    #[must_use]
    pub const fn jadwal(&self) -> &JadwalMuhtawayat {
        &self.jadwal
    }

    /// How many chunks the container holds.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.jadwal.mawaqi().len()
    }

    /// Every path in the container, sorted, with the chunk index it resolves to.
    ///
    /// Empty for a container without a directory index — a version 1 container,
    /// or one built without [`ALAM_MUFAHRAS`]. That is not an error here; it is
    /// reported by the absence of paths, and the caller decides whether a
    /// container it cannot search by name is a container it can use.
    pub fn masarat(&self) -> impl Iterator<Item = (&str, u32)> + '_ {
        self.jadwal.dalil().into_iter().flat_map(FahrasDalil::kull)
    }

    /// The chunk index a container-relative path resolves to.
    #[must_use]
    pub fn jid(&self, masar: &str) -> Option<u32> {
        self.jadwal.dalil()?.jid(masar)
    }

    /// Every localization resource in the container.
    ///
    /// The whole reason this module has a read path. A cooked UE5 title keeps its
    /// compiled localization under `Content/Localization/<Target>/<Culture>/`, so
    /// this is a suffix match on `.locres` narrowed by that directory — narrowed,
    /// because a game may ship an unrelated `.locres` in a plugin directory and
    /// the extractor's job is the game's own text.
    ///
    /// Matched case-insensitively: the paths inside a container are whatever the
    /// cook wrote, and a Windows-authored project routinely disagrees with itself
    /// about the casing of `Content`.
    pub fn masarat_locres(&self) -> impl Iterator<Item = (&str, u32)> + '_ {
        self.masarat().filter(|(masar, _)| {
            let khafid = masar.to_ascii_lowercase();
            khafid.ends_with(".locres") && khafid.contains("/localization/")
        })
    }

    /// Every metadata file in the container, beside the resources they describe.
    pub fn masarat_locmeta(&self) -> impl Iterator<Item = (&str, u32)> + '_ {
        self.masarat()
            .filter(|(masar, _)| masar.to_ascii_lowercase().ends_with(".locmeta"))
    }

    /// Reads one chunk by its index in the table of contents.
    ///
    /// The path, in the order it has to happen:
    ///
    /// 1. the chunk's offset and length are read out of the offset table — that
    ///    offset is in the container's **uncompressed** address space, not a file
    ///    offset, which is why what follows is arithmetic rather than a seek;
    /// 2. the length is checked against [`AQSA_JUZ`] before anything is
    ///    allocated;
    /// 3. the range of compression blocks the chunk spans is derived by dividing
    ///    by the header's block size, with a checked division because a header a
    ///    stranger wrote may say zero;
    /// 4. each block is located inside a partition by dividing its `.ucas` offset
    ///    by the partition size, read, decrypted if the container is encrypted,
    ///    and expanded into a buffer of exactly its declared size;
    /// 5. the chunk is sliced out of the concatenated blocks at the offset within
    ///    the first block, because a chunk does not have to start on a block
    ///    boundary.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] when the index, the block range, the block
    /// size or the partition arithmetic is not consistent with the container,
    /// [`KhataUnreal::HajmMufrit`] when the chunk or a block declares more than
    /// this build will allocate, [`KhataUnreal::PakMushaffar`] when the container
    /// is encrypted and no key is held, [`KhataUnreal::DaghtMajhul`] for an Oodle
    /// or unknown method, [`KhataUnreal::FakkFashil`] when a block will not
    /// expand, [`KhataUnreal::HajmGhayrMutabaq`] when one expands to the wrong
    /// size, and [`KhataUnreal::MalafQaseer`] when a block runs past the end of
    /// the partition it was placed in.
    pub fn iqra_juz(&self, fahras: usize) -> Result<Vec<u8>, KhataUnreal> {
        let mawqi = *self.jadwal.mawaqi().get(fahras).ok_or_else(|| {
            talif(
                "a chunk index",
                tul_u64(fahras),
                tul_u64(self.jadwal.mawaqi().len()),
            )
        })?;
        if mawqi.tul > AQSA_JUZ {
            return Err(KhataUnreal::HajmMufrit {
                haql: "a chunk's length",
                qeema: mawqi.tul,
                saqf: AQSA_JUZ,
            });
        }
        let nihaya = mawqi
            .nihaya()
            .ok_or_else(|| talif("a chunk's extent", mawqi.tul, AQSA_JUZ))?;
        if mawqi.tul == 0 {
            return Ok(Vec::new());
        }

        let hajm_kutla = u64::from(self.jadwal.tarwisa().hajm_kutlat_daght);
        let awwal = qismah(mawqi.izaha, hajm_kutla)
            .ok_or_else(|| talif("the compression block size", 0, u64::from(AQSA_KUTLA_KHAAM)))?;
        let dakhil = baqi(mawqi.izaha, hajm_kutla)
            .ok_or_else(|| talif("the compression block size", 0, u64::from(AQSA_KUTLA_KHAAM)))?;
        let akhir = ila_ala(nihaya, hajm_kutla)
            .and_then(|mahdud| qismah(mahdud, hajm_kutla))
            .ok_or_else(|| talif("a chunk's block range", nihaya, u64::MAX))?;

        let awwal = hajm_usize(awwal).ok_or_else(|| talif("a chunk's first block", awwal, 0))?;
        let akhir = hajm_usize(akhir).ok_or_else(|| talif("a chunk's last block", akhir, 0))?;
        let kutal = self.jadwal.kutal().get(awwal..akhir).ok_or_else(|| {
            talif(
                "a chunk's block range",
                tul_u64(akhir),
                tul_u64(self.jadwal.kutal().len()),
            )
        })?;

        let siaa = hajm_usize(mawqi.tul).ok_or(KhataUnreal::HajmMufrit {
            haql: "a chunk's length",
            qeema: mawqi.tul,
            saqf: AQSA_JUZ,
        })?;
        let mut majmu: Vec<u8> = Vec::new();
        majmu
            .try_reserve(siaa)
            .map_err(|_| KhataUnreal::HajmMufrit {
                haql: "a chunk's length",
                qeema: mawqi.tul,
                saqf: AQSA_JUZ,
            })?;

        for kutla in kutal {
            let mut khaam = self.iqra_kutla(kutla)?;
            majmu.append(&mut khaam);
            // The concatenation cannot outrun the chunk by more than one block's
            // worth of leading offset plus one block of trailing slack, and every
            // block was already bounded, so this is a belt on top of a brace —
            // and it is the belt that catches a block table that describes far
            // more bytes than the chunk that indexes it.
            if tul_u64(majmu.len()) > AQSA_JUZ.saturating_add(u64::from(AQSA_KUTLA_KHAAM)) {
                return Err(KhataUnreal::HajmMufrit {
                    haql: "a chunk's expanded blocks",
                    qeema: tul_u64(majmu.len()),
                    saqf: AQSA_JUZ,
                });
            }
        }

        let bidaya =
            hajm_usize(dakhil).ok_or_else(|| talif("a chunk's block offset", dakhil, 0))?;
        let khatam = bidaya
            .checked_add(siaa)
            .ok_or_else(|| talif("a chunk's extent", mawqi.tul, tul_u64(majmu.len())))?;
        majmu
            .get(bidaya..khatam)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| talif("a chunk's extent", tul_u64(khatam), tul_u64(majmu.len())))
    }

    /// Reads one chunk by container-relative path.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] when the container has no directory index or
    /// no such path, and otherwise whatever [`Self::iqra_juz`] refuses.
    pub fn iqra_masar(&self, masar: &str) -> Result<Vec<u8>, KhataUnreal> {
        let fahras = self
            .jid(masar)
            .ok_or_else(|| talif("a path in the directory index", 0, tul_u64(masar.len())))?;
        let fahras = hajm_usize(u64::from(fahras))
            .ok_or_else(|| talif("a chunk index", u64::from(fahras), 0))?;
        self.iqra_juz(fahras)
    }

    /// Reads, decrypts and expands one compression block.
    fn iqra_kutla(&self, kutla: &MadkhalKutlatDaght) -> Result<Vec<u8>, KhataUnreal> {
        let hajm_qitaa = self.jadwal.tarwisa().hajm_qitaa;
        let qitaa = qismah(kutla.izaha, hajm_qitaa)
            .and_then(|qitaa| u32::try_from(qitaa).ok())
            .ok_or_else(|| talif("the partition size", hajm_qitaa, u64::from(AQSA_QITAAT)))?;
        let dakhil = baqi(kutla.izaha, hajm_qitaa)
            .ok_or_else(|| talif("the partition size", hajm_qitaa, u64::from(AQSA_QITAAT)))?;

        if kutla.hajm_madghut > AQSA_KUTLA_KHAAM {
            return Err(KhataUnreal::HajmMufrit {
                haql: "a compression block's stored size",
                qeema: u64::from(kutla.hajm_madghut),
                saqf: u64::from(AQSA_KUTLA_KHAAM),
            });
        }

        // An encrypted block is stored padded to the AES block size, while the
        // table records the unpadded length. Reading the padded span and handing
        // only the recorded length to the decompressor is what keeps the two
        // straight; reading the unpadded span would leave the last cipher block
        // incomplete and decrypt it into noise.
        let mutlub = if self.jadwal.tarwisa().alam.mushaffar() {
            ila_ala(u64::from(kutla.hajm_madghut), tul_u64(HAJM_KUTLAT_TASHFEER)).ok_or_else(
                || {
                    talif(
                        "a compression block's stored size",
                        u64::from(kutla.hajm_madghut),
                        0,
                    )
                },
            )?
        } else {
            u64::from(kutla.hajm_madghut)
        };
        let siaa = hajm_usize(mutlub).ok_or_else(|| KhataUnreal::HajmMufrit {
            haql: "a compression block's stored size",
            qeema: mutlub,
            saqf: u64::from(AQSA_KUTLA_KHAAM),
        })?;

        let mut makhzun = vec![0u8; siaa];
        self.masdar.iqra_nitaq(qitaa, dakhil, &mut makhzun)?;

        if self.jadwal.tarwisa().alam.mushaffar() {
            let miftah = self
                .miftah
                .as_ref()
                .ok_or_else(|| KhataUnreal::PakMushaffar {
                    masar: self.masar.clone(),
                })?;
            fukk_tashfeer(&mut makhzun, miftah, &self.masar)?;
        }

        let hadd = hajm_usize(u64::from(kutla.hajm_madghut)).unwrap_or(0);
        let makhzun = makhzun
            .get(..hadd)
            .ok_or_else(|| KhataUnreal::MalafQaseer {
                haql: "a compression block",
                tul: tul_u64(siaa),
                matlub: u64::from(kutla.hajm_madghut),
            })?;

        let tareeqa = self.jadwal.tareeqa(kutla);
        fukk_daght(&tareeqa, makhzun, kutla.hajm_khaam)
    }
}
