//! الترجمة — Godot's compiled `.translation`, in both of its forms.
//!
//! A Godot project's translations are compiled from `.po` or `.csv` into a
//! resource the engine loads like any other, and there are two classes it can
//! be. `Translation` is a message list: source strings and the strings shown in
//! their place. `OptimizedTranslation` is the same data as a closed hash table,
//! generated from a `Translation` at export time, and it is what a released game
//! usually ships because it resolves a lookup without building a dictionary at
//! load.
//!
//! Both arrive wrapped in Godot's binary resource container, so this module is
//! two formats stacked: the wrapper, which is generic and shared with every
//! other `.res` in the engine, and the property payload, which is one of the two
//! translation shapes.
//!
//! ## The wrapper
//!
//! ```text
//! RSRC header — little-endian
//!
//!   offset  size  field        type      meaning
//!        0     4  sihr         [u8; 4]   "RSRC", or "RSCC" for the compressed form
//!        4     4  kabir        u32       big-endian flag
//!        8     4  haqiqi64     u32       64-bit floats flag
//!       12     4  muharrik_r   u32       the engine's major version
//!       16     4  muharrik_s   u32       the engine's minor version
//!       20     4  isdar        u32       the resource format version
//!       24   ...  naw          nass      the resource's class name
//!      ...     8  izahat_wasf  u64       offset to import metadata; zero in practice
//!      ...    56  ...          ...       fourteen words, read below
//!      ...     4  adad_hawd    u32       how many strings the string map holds
//!      ...   ...  hawd         nass[]    the string map
//!      ...     4  adad_kharij  u32       how many external resources
//!      ...   ...  kharijiya    ...       type, optional uid, path — each
//!      ...     4  adad_dakhil  u32       how many internal resources
//!      ...   ...  dakhiliya    ...       path and u64 offset — each
//!      ...   ...  ...          ...       each internal resource, at its offset
//!  tul - 4     4  sihr         [u8; 4]   "RSRC" again, closing the file
//! ```
//!
//! The fifty-six bytes after `izahat_wasf` are the part that catches readers,
//! because they mean two different things and the file does not flag which:
//!
//! ```text
//! format <= 3 (Godot 3)   fourteen reserved u32, all zero
//!
//! format >= 4 (Godot 4)   alam     u32       the ALAM_* bits
//!                         muarrif  u64       the resource's UID
//!                         [sanf    nass      only when ALAM_SANF is set]
//!                         eleven reserved u32
//! ```
//!
//! Four bytes of flags plus eight of UID is twelve, which is three words, and
//! fourteen minus three is eleven — so Godot 4 did not grow the header, it carved
//! two fields out of the reserved run Godot 3 left. Absent a script class the two
//! layouts occupy exactly the same bytes, which is why a reader that ignores the
//! format version reads a Godot 4 resource without complaint and then finds the
//! string table three words early.
//!
//! ```text
//! nass — one string inside a binary resource
//!
//!   offset  size  field  type    meaning
//!        0     4  tul    u32     how many bytes follow, NUL TERMINATOR INCLUDED
//!        4   ...  bayt   [u8]    tul bytes of UTF-8, the last of which is zero
//! ```
//!
//! A length of zero is the empty string and nothing follows it — there is no
//! terminator to write, because the count that would have held it is the count
//! that is zero. Every other length includes the terminator, so a five means a
//! four byte string.
//!
//! ## `RSCC`, the compressed variant
//!
//! When the exporter compresses a resource the four magic bytes are `RSCC` and
//! what follows is not a resource but a block-compressed stream whose expansion
//! begins with `RSRC`:
//!
//! ```text
//! RSCC header — little-endian
//!
//!   offset  size  field       type      meaning
//!        0     4  sihr        [u8; 4]   "RSCC"
//!        4     4  naw_daght   u32       0 FastLZ, 1 Deflate, 2 Zstd, 3 Gzip, 4 Brotli
//!        8     4  hajm_kutla  u32       how many bytes each block expands to
//!       12     4  majmu       u32       the whole expanded length
//!       16   ...  ahjam       u32[]     one stored size per block
//!      ...   ...  kutal       [u8]      the blocks, back to back
//! ```
//!
//! There are `(majmu / hajm_kutla) + 1` blocks — always one more than the
//! division gives, so a stream whose length is an exact multiple of the block
//! size ends with an empty block rather than with no room for the remainder.
//! Every block expands to `hajm_kutla` bytes except the last, which expands to
//! whatever is left.
//!
//! This build expands Deflate, Zstd and Gzip, which is what the engine's default
//! (Zstd) and every setting a project is likely to have chosen produce. `FastLZ`
//! and Brotli are refused by name with [`KhataGodot::DaghtMajhul`] rather than
//! guessed at.
//!
//! Reading an `RSCC` and writing it back produces an `RSRC` whose expansion is
//! byte-identical, not an `RSCC` that is byte-identical. That is not a shortcut:
//! a compressor's output is a function of its input *and* its version, its level
//! and its window, so no reader can promise to reproduce a stream it did not
//! produce. What is promised is that the resource inside is unchanged, and
//! [`GhilafMawrid::daght`] reports which mode it arrived under.
//!
//! ## `Translation`: the message list
//!
//! The plain class stores one property, `messages`, and the shape of it changed
//! between engine generations. Godot 3 stores a `PoolStringArray` of alternating
//! source and translation; Godot 4 stores a `Dictionary` keyed by source. The two
//! are the same information and neither can be inferred from the other's
//! presence, so [`Tarjama`] records which one it read in [`ShaklRasail`] and
//! writes the same one back — the same reason the sibling adapter treats a
//! string's encoding as part of its value.
//!
//! ## `OptimizedTranslation`: three parallel arrays
//!
//! The optimized class stores no messages at all. It stores three arrays:
//!
//! * `hash_table`, a `PackedInt32Array` whose length is a prime and whose entries
//!   are either `-1` for an empty slot or an index into the next array;
//! * `bucket_table`, a `PackedInt32Array` holding variable-length buckets;
//! * `strings`, a `PackedByteArray` holding every translated string's UTF-8, back
//!   to back with no separators.
//!
//! ```text
//! One bucket, as words inside bucket_table
//!
//!   word  field  meaning
//!      0  adad   how many elements this bucket holds
//!      1  daala  the second-round hash seed chosen for this bucket
//!    2+4i  key         hash(daala, source) for element i
//!    3+4i  izaha       where element i's translation starts in `strings`
//!    4+4i  tul_madghut how many bytes it occupies there
//!    5+4i  tul_khaam   how many bytes it expands to
//! ```
//!
//! A lookup is: hash the source with seed zero, take that modulo the hash
//! table's length, read the bucket index there, refuse a `-1`, re-hash the source
//! with *that bucket's* seed, and scan the bucket's elements for a matching key.
//! The two-round scheme is what makes the table perfect: at generation the seed
//! is searched upward from one until no two sources in the same bucket collide,
//! so within a bucket the second hash is unique and the scan is a formality.
//!
//! ### The hash, exactly
//!
//! ```text
//! basma(daala, nass):
//!     if daala == 0 { daala = 0x0100_0193 }
//!     for each UTF-8 byte b of nass:
//!         daala = (daala * 0x0100_0193) ^ b        // b widened SIGNED
//!     daala
//! ```
//!
//! It is FNV's 32-bit prime in FNV-1's multiply-then-xor order, seeded with the
//! prime rather than with FNV's offset basis, and the bytes are widened
//! **signed** — the engine's loop is `d = (d * 0x1000193) ^ uint32_t(*p_str)`
//! over a `const char *`, and `char` is signed on every target this product
//! ships to, so a byte of 0xD8 contributes `0xFFFF_FFD8` and not `0xD8`. See
//! [`basma_godot`] for the measurement that settled it. Every one of those four
//! details is load bearing, and the failure they prevent is the quietest one in
//! this whole crate:
//! a table whose hashes were computed differently from the engine's is a
//! perfectly well-formed resource that the engine loads without a single warning
//! and then finds nothing in. Every lookup misses, every string falls through to
//! its source text, and the game runs in its original language with a translation
//! installed. Nothing is corrupt, nothing errors, and there is no message
//! anywhere saying why.
//!
//! ### An optimized translation cannot be enumerated
//!
//! `strings` holds the translations. The source strings are **not stored** — only
//! their hashes are, and a hash is not reversible. So an `OptimizedTranslation`
//! answers "what is the translation of this exact string" and cannot answer "what
//! strings are in here". Extraction has to get its source text from a plain
//! `Translation`, or from the game's own scenes and scripts; this module can
//! confirm a string is present and produce its translation, and that is the whole
//! of what the format permits.
//!
//! ### What is refused
//!
//! Godot compresses an optimized translation's strings with SMAZ, a codebook
//! compressor for short English text, and stores a string uncompressed whenever
//! the compressed form is not smaller. An entry whose `tul_madghut` differs from
//! its `tul_khaam` is therefore SMAZ-compressed, and this build refuses it with
//! [`KhataGodot::DaghtMajhul`] naming [`DAGHT_SMAZ`] rather than decompress it
//! against a two hundred and fifty-four entry codebook transcribed by hand. A
//! single wrong entry in that table would produce plausible text that is not the
//! text in the file, which is precisely the class of failure this crate refuses
//! to risk.
//!
//! Writing is unaffected: [`TarjamaMurakkaza::min_tarjama`] stores every string
//! uncompressed, which is what the engine's own generator does whenever
//! compression does not help — and it never helps here, because SMAZ's codebook
//! is English digrams and words, and Arabic UTF-8 is bytes above 0x7F that SMAZ
//! can only emit as literal escapes. Compressing Arabic with it makes the pool
//! larger.
//!
//! ## The subset of the wrapper this module handles
//!
//! Handled: the `RSRC` and `RSCC` magics; format versions up to
//! [`ISDAR_SIGHA_AQSA`]; the class name; the flags, UID and script class of
//! format 4 and above; the reserved run of either generation; the string map;
//! external and internal resource tables; and, inside the one internal resource,
//! a property list whose values are drawn from [`Qeema`].
//!
//! Refused, by name: a non-zero big-endian flag, because every field in this
//! module is read little-endian and honouring the flag would mean a second
//! reader for a file Godot does not export; a non-zero 64-bit-float flag, for the
//! same reason; a format version above what this build knows; a resource holding
//! other than exactly one internal resource, which a `.translation` never does;
//! and any property whose variant tag is outside [`Qeema`], which is every tag a
//! translation cannot contain. This is a translation reader, not a general Godot
//! resource reader, and it says so by refusing rather than by skipping.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::khata::{KhataGodot, hajm_usize, tul_u64};

use super::{
    AQSA_TUL_HAQL, Katib, Mawrid, Qari, hashw_muhadhah, iqra_malaf, tahaqquq_adad, tahaqquq_tul,
    uktub_malaf,
};

/// The format's name, in every refusal this module raises.
pub const ISM: &str = "Godot translation resource";

/// The four bytes an uncompressed binary resource begins and ends with.
pub const SIHR_RSRC: [u8; 4] = *b"RSRC";

/// The four bytes a block-compressed binary resource begins with.
pub const SIHR_RSCC: [u8; 4] = *b"RSCC";

/// The highest resource format version this build reads.
///
/// Six, which is Godot 4's current number. Godot 3 wrote three.
pub const ISDAR_SIGHA_AQSA: u32 = 6;

/// The first format version that carries flags, a UID and a script class.
pub const ISDAR_SIGHA_BI_ALAM: u32 = 4;

/// The resource format version Godot 3 writes, and the highest it reads.
///
/// `FORMAT_VERSION = 3` in Godot 3.6's `core/io/resource_format_binary.cpp`, and
/// its loader refuses outright — `ERR_FAIL_MSG`, no resource, no fallback — when
/// either the format version or the recorded engine major version is above its
/// own. So a resource written for a Godot 3 game with anything but this is not a
/// resource that loads slightly wrong; it is a file the engine will not open.
pub const ISDAR_SIGHA_THALITH: u32 = 3;

/// How many reserved words a pre-flags resource carries after the metadata
/// offset.
pub const ADAD_MAHJUZ_QADEEM: usize = 14;

/// How many reserved words a resource with flags carries — three fewer, because
/// the flags word and the UID were carved out of the same fifty-six bytes.
pub const ADAD_MAHJUZ_HADITH: usize = 11;

/// [`GhilafMawrid::alam`]: scene node identifiers are names rather than numbers.
pub const ALAM_MAARIF_MASHHAD: u32 = 1 << 0;

/// [`GhilafMawrid::alam`]: external resources carry a UID as well as a path.
pub const ALAM_MUARRIFAT: u32 = 1 << 1;

/// [`GhilafMawrid::alam`]: the engine that wrote this was built with 64-bit
/// reals, which changes the width of every float in the file.
pub const ALAM_HAQIQI_MUDAAF: u32 = 1 << 2;

/// [`GhilafMawrid::alam`]: a script class name follows the UID.
pub const ALAM_SANF: u32 = 1 << 3;

/// The variant tag for the null value.
pub const WASF_FARAGH: u32 = 1;
/// The variant tag for a boolean.
pub const WASF_MANTIQI: u32 = 2;
/// The variant tag for a 32-bit signed integer.
pub const WASF_SAHIH: u32 = 3;
/// The variant tag for a string.
pub const WASF_NASS: u32 = 5;
/// The variant tag for a dictionary.
pub const WASF_QAMUS: u32 = 26;
/// The variant tag for an untyped array.
pub const WASF_MASFUFA: u32 = 30;
/// The variant tag for a packed byte array.
pub const WASF_BAYT: u32 = 31;
/// The variant tag for a packed 32-bit integer array.
pub const WASF_SAHIHAT: u32 = 32;
/// The variant tag for a packed string array.
pub const WASF_NUSUS: u32 = 34;
/// The variant tag for a 64-bit signed integer.
pub const WASF_SAHIH_TAWIL: u32 = 40;
/// The variant tag for a `StringName`, which serializes exactly like a string
/// and is a different type to the engine.
pub const WASF_ISM_NASS: u32 = 44;

/// The high bit of a container's length word, meaning the value was shared.
///
/// Preserved rather than dropped: it is part of how the engine reconstructs
/// object identity, and a dictionary that comes back unshared is a different
/// value even when its contents match.
pub const ALAM_MUSHARAK: u32 = 0x8000_0000;

/// The high bit of a property-name word, meaning the name follows inline instead
/// of indexing the string map.
pub const ALAM_ISM_MUDMAJ: u32 = 0x8000_0000;

/// The boundary a packed byte array is padded up to.
pub const MUHADHAT_BAYT: usize = 4;

/// Compression mode 0: `FastLZ`. Not expanded by this build.
pub const DAGHT_FASTLZ: u32 = 0;
/// Compression mode 1: raw Deflate, no zlib or gzip framing.
pub const DAGHT_DEFLATE: u32 = 1;
/// Compression mode 2: Zstandard, one frame per block. The engine's default.
pub const DAGHT_ZSTD: u32 = 2;
/// Compression mode 3: gzip.
pub const DAGHT_GZIP: u32 = 3;
/// Compression mode 4: Brotli. Not expanded by this build.
pub const DAGHT_BROTLI: u32 = 4;

/// Taarib's own number for the SMAZ codebook an optimized translation's strings
/// may be compressed with.
///
/// The format carries no mode field there — an element whose stored length
/// differs from its expanded length *is* the flag — so this number exists only so
/// that [`KhataGodot::DaghtMajhul`] can name the thing it is refusing rather than
/// report a mode of zero, which would read as `FastLZ`.
pub const DAGHT_SMAZ: u32 = 100;

/// The class name of a plain message list.
pub const NAW_BASITA: &str = "Translation";

/// The class name of the hash table form in Godot 4.
pub const NAW_MURAKKAZA: &str = "OptimizedTranslation";

/// The class name of the hash table form in Godot 3, which is the same format
/// under an older name.
pub const NAW_MURAKKAZA_QADEEM: &str = "PHashTranslation";

/// The property holding a plain translation's messages.
pub const KHASIYAT_RASAIL: &str = "messages";

/// The property holding the locale both classes are registered under.
pub const KHASIYAT_THAQAFA: &str = "locale";

/// The property holding an optimized translation's top-level table.
pub const KHASIYAT_JADWAL: &str = "hash_table";

/// The property holding an optimized translation's buckets.
pub const KHASIYAT_DILAA: &str = "bucket_table";

/// The property holding an optimized translation's string pool.
pub const KHASIYAT_HAWD: &str = "strings";

/// The internal resource's own path in a file holding exactly one resource.
///
/// `local://1`, not `local://0`: the engine's saver numbers the resources it is
/// about to write from one, and a file Godot 3.6 wrote for a single
/// `Translation` was observed to carry exactly this.
pub const MASAR_DAKHILI: &str = "local://1";

/// The FNV 32-bit prime, which is both the multiplier and the seed of Godot's
/// translation hash.
pub const THABIT_BASMA: u32 = 0x0100_0193;

/// How many words a bucket's fixed head occupies: its size and its seed.
pub const TUL_RAAS_DALU: usize = 2;

/// How many words each bucket element occupies: key, offset, stored length,
/// expanded length.
pub const TUL_UNSUR_DALU: usize = 4;

/// The largest number of messages one translation may hold.
///
/// Two million. The largest localized game scripts are in the low hundreds of
/// thousands of strings.
pub const AQSA_RASAIL: u64 = 2_000_000;

/// The largest number of entries any table inside the wrapper may declare.
pub const AQSA_MAWARID: u64 = 1_000_000;

/// The largest number of properties one resource record may declare.
pub const AQSA_KHASAIS: u64 = 65_536;

/// How deeply a variant may nest before this reader stops descending.
///
/// Eight. A translation's properties are flat or one level deep; a file that
/// nests further is either not a translation or is trying to exhaust this
/// process's stack, and the two are refused the same way.
pub const AQSA_UMQ: u32 = 8;

/// The largest number of blocks a compressed resource may declare.
pub const AQSA_KUTAL: u64 = 262_144;

/// The largest expansion this build will produce from a compressed resource.
pub const AQSA_TAWSEE: u64 = 512 * 1024 * 1024;

/// The largest number of words any packed integer array may declare.
///
/// Sixteen million, which is sixty-four mebibytes of table. An optimized
/// translation of [`AQSA_RASAIL`] messages needs a bucket table of about ten
/// million words, so this is the smallest ceiling that does not refuse a
/// translation the rest of this module would accept.
pub const AQSA_KALIMAT: u64 = 16_000_000;

/// The shape refusal, in one spelling for the whole module.
const fn talifa(haql: &'static str) -> KhataGodot {
    KhataGodot::TarjamaTalifa { haql }
}

/// Which engine generation a resource is being written **for**.
///
/// Reading needs no such argument — the file states its own format version, and
/// [`MawridTarjama`]'s reader branches on it. Writing does, and there is no
/// default that could be right: the two generations disagree about the format
/// version, the size of the reserved run, the class name of the hash-table form,
/// the type of the `messages` property and which variant tags exist at all.
/// Every one of those disagreements is silent in one direction and fatal in the
/// other, so this is an argument rather than a field with a value picked for the
/// caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JeelMawrid {
    /// Godot 3. Format 3, fourteen reserved words, `PHashTranslation`,
    /// `messages` as a `PoolStringArray`, and no variant tag above 41.
    Thalith,
    /// Godot 4. Format 4 and up, eleven reserved words plus flags and a UID,
    /// `OptimizedTranslation`, `messages` as a `Dictionary` of `StringName`.
    Rabi,
}

impl JeelMawrid {
    /// The resource format version this generation writes.
    #[must_use]
    pub const fn isdar_sigha(self) -> u32 {
        match self {
            Self::Thalith => ISDAR_SIGHA_THALITH,
            Self::Rabi => ISDAR_SIGHA_BI_ALAM,
        }
    }

    /// Whether this generation's wrapper carries flags, a UID and a script class.
    #[must_use]
    pub const fn bi_alam(self) -> bool {
        matches!(self, Self::Rabi)
    }

    /// How many reserved words follow the metadata offset.
    #[must_use]
    pub const fn adad_mahjuz(self) -> usize {
        if self.bi_alam() { ADAD_MAHJUZ_HADITH } else { ADAD_MAHJUZ_QADEEM }
    }

    /// The class name of the hash-table form.
    ///
    /// Godot 3 registers it as `PHashTranslation` and has no class called
    /// `OptimizedTranslation`; the engine instantiates a resource by the class
    /// name on its internal record, so writing Godot 4's name for a Godot 3 game
    /// produces a file whose only symptom is a missing class at load.
    #[must_use]
    pub const fn naw_murakkaza(self) -> &'static str {
        match self {
            Self::Thalith => NAW_MURAKKAZA_QADEEM,
            Self::Rabi => NAW_MURAKKAZA,
        }
    }

    /// The shape `Translation::messages` takes on this generation.
    ///
    /// Not a preference. Godot 3 binds the property through
    /// `_set_messages(const PoolVector<String> &)` and Godot 4 through
    /// `_set_messages(const Dictionary &)`; handing either the other's value is
    /// a failed `Variant` conversion, which leaves the property unset and the
    /// translation empty without one line of complaint.
    #[must_use]
    pub const fn shakl_rasail(self) -> ShaklRasail {
        match self {
            Self::Thalith => ShaklRasail::Masfufa,
            Self::Rabi => ShaklRasail::Qamus,
        }
    }

    /// Whether this generation's binary serializer knows a variant tag.
    ///
    /// Godot 3's tag list ends at `VARIANT_DOUBLE = 41`. Godot 4 added
    /// `StringName` at 44, which is the one a translation can actually reach:
    /// Godot 4 stores a message dictionary's keys and values as `StringName`,
    /// and a Godot 3 loader handed tag 44 falls through its `switch` and returns
    /// `ERR_FILE_CORRUPT` for the whole resource.
    #[must_use]
    pub const fn yaqra_wasf(self, wasf: u32) -> bool {
        match self {
            Self::Rabi => true,
            Self::Thalith => wasf <= WASF_DOUBLE_THALITH,
        }
    }

    /// A stable short name, for a refusal that has to say which was asked for.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Thalith => "Godot 3",
            Self::Rabi => "Godot 4",
        }
    }
}

/// The highest variant tag Godot 3's binary serializer defines.
///
/// `VARIANT_DOUBLE = 41`. Named so that [`JeelMawrid::yaqra_wasf`] compares
/// against the engine's own last tag rather than against a number in a
/// condition.
pub const WASF_DOUBLE_THALITH: u32 = 41;

/// Reads one length-prefixed string.
///
/// The stored length counts the NUL terminator, so a five is a four byte string.
/// A zero is the empty string with nothing following it — a form the engine's own
/// writer never produces, since it writes a one and a lone terminator, but one
/// that costs nothing to accept and would otherwise read as a truncated field.
fn iqra_nass(qari: &mut Qari<'_>, haql: &'static str) -> Result<String, KhataGodot> {
    let tul = u64::from(qari.iqra_u32(haql)?);
    nass_min_tul(qari, haql, tul)
}

/// Reads `tul` bytes as one NUL-terminated string, the terminator counted.
///
/// Split out because a property name written inline carries its length in the
/// same word as its inline flag, so the length is already in hand and there is no
/// prefix left to read.
fn nass_min_tul(qari: &mut Qari<'_>, haql: &'static str, tul: u64) -> Result<String, KhataGodot> {
    if tul == 0 {
        return Ok(String::new());
    }
    tahaqquq_tul(haql, tul)?;
    let khaam = qari.iqra_bayt(haql, tul)?;
    let (akhir, jism) = khaam.split_last().unwrap_or((&0, &[]));
    if *akhir != 0 {
        // Trimming instead would mean a file that lies about its own lengths
        // still reads, with every field after it shifted by one byte and nothing
        // to say so.
        return Err(talifa("a string whose declared length does not end in a NUL"));
    }
    qari.nass(jism)
}

/// Writes one length-prefixed string, the way the engine writes it: the UTF-8
/// bytes, a terminator, and a length counting both.
fn uktub_nass(katib: &mut Katib, haql: &'static str, nass: &str) -> Result<(), KhataGodot> {
    let khaam = nass.as_bytes();
    let tul = tul_u64(khaam.len()).saturating_add(1);
    if tul > AQSA_TUL_HAQL {
        return Err(KhataGodot::HajmMufrit { haql, qeema: tul, saqf: AQSA_TUL_HAQL });
    }
    let muallan = u32::try_from(tul).map_err(|_| KhataGodot::HajmMufrit {
        haql,
        qeema: tul,
        saqf: AQSA_TUL_HAQL,
    })?;
    katib.uktub_u32(muallan);
    katib.uktub_bayt(khaam);
    katib.uktub_u8(0);
    Ok(())
}

/// One property value, in the subset a translation resource can hold.
///
/// Deliberately not every Godot variant. A `.translation` carries strings, a
/// dictionary or a string array of messages, and three packed arrays; a file
/// whose properties are transforms and object references is not a translation,
/// and a reader that skipped what it did not recognise would have to guess how
/// many bytes to skip. Every tag outside this set is refused by number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Qeema {
    /// The null value.
    Faragh,
    /// A boolean, stored as a full word.
    Mantiqi(bool),
    /// A 32-bit signed integer.
    Sahih(i32),
    /// A 64-bit signed integer, which is a different tag and not a wider
    /// encoding of the same one.
    SahihTawil(i64),
    /// A string.
    Nass(String),
    /// A `StringName`. Serialized exactly like [`Qeema::Nass`] and a different
    /// type to the engine, which is why it is a separate variant rather than a
    /// flag: Godot 4 stores a translation's messages as `StringName` keys and
    /// values, and writing them back as plain strings would produce a dictionary
    /// the engine populates with the wrong key type.
    IsmNass(String),
    /// A packed byte array, padded out to a four byte boundary in the file.
    Bayt(Vec<u8>),
    /// A packed 32-bit integer array.
    Sahihat(Vec<i32>),
    /// A packed string array.
    Nusus(Vec<String>),
    /// A dictionary. The pairs keep the order the file stored them in, because
    /// Godot's `Dictionary` is ordered and rewriting it in some other order would
    /// change the file without changing its meaning.
    Qamus {
        /// The high bit of the length word: whether the value was shared.
        musharak: bool,
        /// The pairs, in file order.
        azwaj: Vec<(Self, Self)>,
    },
    /// An untyped array.
    Masfufa {
        /// The high bit of the length word: whether the value was shared.
        musharak: bool,
        /// The elements, in file order.
        anasir: Vec<Self>,
    },
}

impl Qeema {
    /// The variant tag this value serializes with.
    #[must_use]
    pub const fn wasf(&self) -> u32 {
        match self {
            Self::Faragh => WASF_FARAGH,
            Self::Mantiqi(_) => WASF_MANTIQI,
            Self::Sahih(_) => WASF_SAHIH,
            Self::SahihTawil(_) => WASF_SAHIH_TAWIL,
            Self::Nass(_) => WASF_NASS,
            Self::IsmNass(_) => WASF_ISM_NASS,
            Self::Bayt(_) => WASF_BAYT,
            Self::Sahihat(_) => WASF_SAHIHAT,
            Self::Nusus(_) => WASF_NUSUS,
            Self::Qamus { .. } => WASF_QAMUS,
            Self::Masfufa { .. } => WASF_MASFUFA,
        }
    }

    /// The text of a string or a `StringName`, if this is one.
    #[must_use]
    pub fn nass(&self) -> Option<&str> {
        match self {
            Self::Nass(nass) | Self::IsmNass(nass) => Some(nass),
            _ => None,
        }
    }

    /// The words of a packed integer array, if this is one.
    #[must_use]
    pub fn sahihat(&self) -> Option<&[i32]> {
        match self {
            Self::Sahihat(kalimat) => Some(kalimat),
            _ => None,
        }
    }

    /// The bytes of a packed byte array, if this is one.
    #[must_use]
    pub fn bayt(&self) -> Option<&[u8]> {
        match self {
            Self::Bayt(khaam) => Some(khaam),
            _ => None,
        }
    }
}

impl Qeema {
    /// Reads one variant, refusing a tag outside this subset.
    ///
    /// `umq` is how deep the reader already is. A translation's values are flat
    /// or one level deep; the limit exists so that a file describing a dictionary
    /// inside a dictionary inside an array forever is refused rather than
    /// exhausting the stack of whatever process is reading it — which may be a
    /// game.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::TarjamaTalifa`] for a tag this build does not read or for
    /// nesting past [`AQSA_UMQ`], [`KhataGodot::MalafQaseer`] when a field runs
    /// past the end, [`KhataGodot::HajmMufrit`] for a length above its ceiling,
    /// [`KhataGodot::HawiyaTalifa`] for a count the file cannot hold, and
    /// [`KhataGodot::NassGhayrSalih`] for text that is not valid UTF-8.
    pub fn iqra(qari: &mut Qari<'_>, umq: u32) -> Result<Self, KhataGodot> {
        if umq > AQSA_UMQ {
            return Err(talifa("a property nested deeper than this reader descends"));
        }
        let dakhil = umq.saturating_add(1);
        let wasf = qari.iqra_u32("a property's variant tag")?;
        match wasf {
            WASF_FARAGH => Ok(Self::Faragh),
            WASF_MANTIQI => Ok(Self::Mantiqi(qari.iqra_u32("a boolean")? != 0)),
            WASF_SAHIH => Ok(Self::Sahih(qari.iqra_i32("a 32-bit integer")?)),
            WASF_SAHIH_TAWIL => {
                let khana = qari.iqra_masfufa::<8>("a 64-bit integer")?;
                Ok(Self::SahihTawil(i64::from_le_bytes(khana)))
            }
            WASF_NASS => Ok(Self::Nass(iqra_nass(qari, "a string property")?)),
            WASF_ISM_NASS => Ok(Self::IsmNass(iqra_nass(qari, "a string name property")?)),
            WASF_BAYT => {
                let haql = "a packed byte array";
                let tul = u64::from(qari.iqra_u32("a packed byte array's length")?);
                tahaqquq_tul(haql, tul)?;
                let khaam = qari.iqra_bayt(haql, tul)?.to_vec();
                // The array is padded to a four byte boundary and the padding is
                // not part of the length, so a reader that stops at the length
                // starts the next property up to three bytes early.
                let hashw = hashw_muhadhah(khaam.len(), MUHADHAT_BAYT);
                qari.takhatta("a packed byte array's padding", tul_u64(hashw))?;
                Ok(Self::Bayt(khaam))
            }
            WASF_SAHIHAT => {
                let haql = "a packed integer array's length";
                let adad = u64::from(qari.iqra_u32(haql)?);
                let matlub =
                    tahaqquq_adad(ISM, haql, adad, AQSA_KALIMAT, 4, qari.baqi())?;
                let mut kalimat = Vec::with_capacity(matlub);
                for _ in 0..matlub {
                    kalimat.push(qari.iqra_i32("a packed integer array's element")?);
                }
                Ok(Self::Sahihat(kalimat))
            }
            WASF_NUSUS => {
                let haql = "a packed string array's length";
                let adad = u64::from(qari.iqra_u32(haql)?);
                let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_RASAIL, 4, qari.baqi())?;
                let mut nusus = Vec::with_capacity(matlub);
                for _ in 0..matlub {
                    nusus.push(iqra_nass(qari, "a packed string array's element")?);
                }
                Ok(Self::Nusus(nusus))
            }
            WASF_QAMUS => {
                let haql = "a dictionary's length";
                let raas = qari.iqra_u32(haql)?;
                let musharak = raas & ALAM_MUSHARAK != 0;
                let adad = u64::from(raas & !ALAM_MUSHARAK);
                // Eight is the least a pair can occupy: two variant tags.
                let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_RASAIL, 8, qari.baqi())?;
                let mut azwaj = Vec::with_capacity(matlub);
                for _ in 0..matlub {
                    let miftah = Self::iqra(qari, dakhil)?;
                    let qeema = Self::iqra(qari, dakhil)?;
                    azwaj.push((miftah, qeema));
                }
                Ok(Self::Qamus { musharak, azwaj })
            }
            WASF_MASFUFA => {
                let haql = "an array's length";
                let raas = qari.iqra_u32(haql)?;
                let musharak = raas & ALAM_MUSHARAK != 0;
                let adad = u64::from(raas & !ALAM_MUSHARAK);
                let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_RASAIL, 4, qari.baqi())?;
                let mut anasir = Vec::with_capacity(matlub);
                for _ in 0..matlub {
                    anasir.push(Self::iqra(qari, dakhil)?);
                }
                Ok(Self::Masfufa { musharak, anasir })
            }
            _ => Err(talifa("a property whose variant tag is outside what a translation holds")),
        }
    }

    /// Writes one variant back in the form it was read.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when a length is larger than the `u32` the
    /// format allots it, which is also the only way this can fail.
    pub fn uktub(&self, katib: &mut Katib) -> Result<(), KhataGodot> {
        katib.uktub_u32(self.wasf());
        match self {
            Self::Faragh => {}
            Self::Mantiqi(qeema) => katib.uktub_u32(u32::from(*qeema)),
            Self::Sahih(qeema) => katib.uktub_i32(*qeema),
            Self::SahihTawil(qeema) => katib.uktub_bayt(&qeema.to_le_bytes()),
            Self::Nass(nass) => uktub_nass(katib, "a string property", nass)?,
            Self::IsmNass(nass) => uktub_nass(katib, "a string name property", nass)?,
            Self::Bayt(khaam) => {
                katib.uktub_u32(tul_u32("a packed byte array", khaam.len())?);
                katib.uktub_bayt(khaam);
                katib.uktub_asfar(hashw_muhadhah(khaam.len(), MUHADHAT_BAYT));
            }
            Self::Sahihat(kalimat) => {
                katib.uktub_u32(tul_u32("a packed integer array", kalimat.len())?);
                for kalima in kalimat {
                    katib.uktub_i32(*kalima);
                }
            }
            Self::Nusus(nusus) => {
                katib.uktub_u32(tul_u32("a packed string array", nusus.len())?);
                for nass in nusus {
                    uktub_nass(katib, "a packed string array's element", nass)?;
                }
            }
            Self::Qamus { musharak, azwaj } => {
                let adad = tul_u32("a dictionary", azwaj.len())?;
                katib.uktub_u32(if *musharak { adad | ALAM_MUSHARAK } else { adad });
                for (miftah, qeema) in azwaj {
                    miftah.uktub(katib)?;
                    qeema.uktub(katib)?;
                }
            }
            Self::Masfufa { musharak, anasir } => {
                let adad = tul_u32("an array", anasir.len())?;
                katib.uktub_u32(if *musharak { adad | ALAM_MUSHARAK } else { adad });
                for unsur in anasir {
                    unsur.uktub(katib)?;
                }
            }
        }
        Ok(())
    }
}

/// A container length as the `u32` the format stores, refusing one that does not
/// fit.
///
/// The high bit is reserved for the shared flag on dictionaries and arrays, so
/// the real ceiling is [`AQSA_KALIMAT`] rather than `u32::MAX` — a length that
/// set the top bit would come back as a shared container half its size.
fn tul_u32(haql: &'static str, tul: usize) -> Result<u32, KhataGodot> {
    let qeema = tul_u64(tul);
    if qeema > AQSA_KALIMAT {
        return Err(KhataGodot::HajmMufrit { haql, qeema, saqf: AQSA_KALIMAT });
    }
    u32::try_from(tul).map_err(|_| KhataGodot::HajmMufrit { haql, qeema, saqf: AQSA_KALIMAT })
}

/// Expands a `RSCC` stream, returning the mode it used and the resource inside.
///
/// # Errors
///
/// [`KhataGodot::TarjamaTalifa`] for a zero block size, which would make the
/// block count meaningless; [`KhataGodot::DaghtMajhul`] for `FastLZ` or Brotli,
/// which this build does not expand; [`KhataGodot::HajmMufrit`] when the declared
/// expansion is above [`AQSA_TAWSEE`] or the block count above [`AQSA_KUTAL`];
/// [`KhataGodot::FakkFashil`] when a block does not decompress; and
/// [`KhataGodot::HajmGhayrMutabaq`] when a block expands to a size other than the
/// one the framing implies.
pub fn fukk_daght(bayt: &[u8]) -> Result<(u32, Vec<u8>), KhataGodot> {
    let mut qari = Qari::jadeed(ISM, bayt);
    let _ = qari.iqra_masfufa::<4>("the compressed resource magic")?;
    let naw = qari.iqra_u32("the compression mode")?;
    let hajm_kutla = u64::from(qari.iqra_u32("the compressed resource's block size")?);
    if hajm_kutla == 0 {
        return Err(talifa("a compressed resource whose block size is zero"));
    }
    let majmu = u64::from(qari.iqra_u32("the compressed resource's expanded length")?);
    if majmu > AQSA_TAWSEE {
        return Err(KhataGodot::HajmMufrit {
            haql: "the compressed resource's expanded length",
            qeema: majmu,
            saqf: AQSA_TAWSEE,
        });
    }
    // Always one more block than the division gives, so a stream whose length is
    // an exact multiple of the block size ends with an empty block rather than
    // with nowhere to put the remainder. `checked_div` because the block size
    // came out of the file, even though it was just refused at zero.
    let adad_kutal = majmu
        .checked_div(hajm_kutla)
        .and_then(|adad| adad.checked_add(1))
        .ok_or_else(|| talifa("a compressed resource whose block count cannot be computed"))?;
    let matlub = tahaqquq_adad(
        ISM,
        "the compressed resource's block count",
        adad_kutal,
        AQSA_KUTAL,
        4,
        qari.baqi(),
    )?;
    let mut ahjam = Vec::with_capacity(matlub);
    for _ in 0..matlub {
        ahjam.push(u64::from(qari.iqra_u32("a compressed block's stored size")?));
    }
    let mut khaam = Vec::with_capacity(hajm_usize(majmu).unwrap_or(0));
    for hajm in ahjam {
        let matlub = hajm_kutla.min(majmu.saturating_sub(tul_u64(khaam.len())));
        let shifra = qari.iqra_bayt("a compressed block", hajm)?;
        let kutla = fukk_kutla(naw, shifra, matlub)?;
        if tul_u64(kutla.len()) != matlub {
            return Err(KhataGodot::HajmGhayrMutabaq {
                muallan: matlub,
                fili: tul_u64(kutla.len()),
            });
        }
        khaam.extend_from_slice(&kutla);
    }
    if tul_u64(khaam.len()) != majmu {
        return Err(KhataGodot::HajmGhayrMutabaq { muallan: majmu, fili: tul_u64(khaam.len()) });
    }
    Ok((naw, khaam))
}

/// Expands one block by the mode the stream declared.
fn fukk_kutla(naw: u32, shifra: &[u8], matlub: u64) -> Result<Vec<u8>, KhataGodot> {
    if matlub == 0 {
        return Ok(Vec::new());
    }
    let siaa = hajm_usize(matlub).unwrap_or(0);
    match naw {
        DAGHT_DEFLATE => {
            let mut khaam = Vec::with_capacity(siaa);
            // Raw deflate: the engine compresses with a negative window size, so
            // there is no zlib header and a zlib reader would refuse the first
            // byte.
            flate2::read::DeflateDecoder::new(shifra)
                .take(matlub)
                .read_to_end(&mut khaam)
                .map_err(|sabab| KhataGodot::FakkFashil { tafsil: sabab.to_string() })?;
            Ok(khaam)
        }
        DAGHT_GZIP => {
            let mut khaam = Vec::with_capacity(siaa);
            flate2::read::GzDecoder::new(shifra)
                .take(matlub)
                .read_to_end(&mut khaam)
                .map_err(|sabab| KhataGodot::FakkFashil { tafsil: sabab.to_string() })?;
            Ok(khaam)
        }
        DAGHT_ZSTD => zstd::stream::decode_all(shifra)
            .map_err(|sabab| KhataGodot::FakkFashil { tafsil: sabab.to_string() }),
        // [`DAGHT_FASTLZ`], [`DAGHT_BROTLI`], and anything the engine adds later.
        // Named by number rather than guessed at: an entry expanded with the
        // wrong algorithm is not a decode failure, it is plausible-looking bytes.
        _ => Err(KhataGodot::DaghtMajhul { naw }),
    }
}

/// One external resource the wrapper references.
///
/// A `.translation` has none — it is a leaf resource with no dependencies — but
/// the table is part of the wrapper and is read and written back so that the
/// round trip does not depend on that being true of every file this reader sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawridKhariji {
    /// The referenced resource's class name.
    pub naw: String,
    /// Its UID, present only when the wrapper's flags say the table carries one.
    pub muarrif: Option<u64>,
    /// Its `res://` path.
    pub masar: String,
}

/// Everything in a binary resource before its first property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhilafMawrid {
    /// The engine's major and minor version, as the exporter recorded them.
    pub muharrik: (u32, u32),
    /// The resource format version. Godot 3 wrote 3; Godot 4 writes 4 and above.
    pub isdar: u32,
    /// The resource's class name: [`NAW_BASITA`], [`NAW_MURAKKAZA`] or
    /// [`NAW_MURAKKAZA_QADEEM`] for the files this module accepts.
    pub naw: String,
    /// The offset of the import metadata block. Zero in every exported resource.
    pub izahat_wasf: u64,
    /// The `ALAM_*` bits, zero before format 4.
    pub alam: u32,
    /// The resource's UID, zero before format 4.
    pub muarrif: u64,
    /// The script class name, present only when [`ALAM_SANF`] is set.
    pub sanf: Option<String>,
    /// The reserved words, verbatim: fourteen before format 4, eleven after.
    pub mahjuz: Vec<u32>,
    /// The string map every property name and some values index into.
    pub hawd: Vec<String>,
    /// The external resource table.
    pub kharijiya: Vec<MawridKhariji>,
    /// The compression mode this resource arrived under, or [`None`] when it was
    /// stored plain. Reported, not reproduced — see this module's header.
    pub daght: Option<u32>,
}

impl GhilafMawrid {
    /// Whether this format version carries flags, a UID and a script class.
    #[must_use]
    pub const fn bi_alam(&self) -> bool {
        self.isdar >= ISDAR_SIGHA_BI_ALAM
    }

    /// Whether the external resource table carries UIDs.
    #[must_use]
    pub const fn bi_muarrifat(&self) -> bool {
        self.alam & ALAM_MUARRIFAT != 0
    }

    /// How many reserved words this version writes.
    #[must_use]
    pub const fn adad_mahjuz(&self) -> usize {
        if self.bi_alam() { ADAD_MAHJUZ_HADITH } else { ADAD_MAHJUZ_QADEEM }
    }

    /// A minimal wrapper for a resource Taarib generates from nothing, for one
    /// engine generation.
    ///
    /// No flags either way: no named scene identifiers, no UID, no doubles and
    /// no script class. That is the smallest wrapper each generation accepts,
    /// and every field it leaves out is a field a patch has no business
    /// inventing a value for. What the generation decides is the format version
    /// and, with it, the size of the reserved run — twelve of those bytes are a
    /// flags word and a UID on Godot 4 and are three more reserved words on
    /// Godot 3, and the file says which only through the version.
    #[must_use]
    pub fn li_jeel(naw: &str, muharrik: (u32, u32), jeel: JeelMawrid) -> Self {
        Self {
            muharrik,
            isdar: jeel.isdar_sigha(),
            naw: naw.to_owned(),
            izahat_wasf: 0,
            alam: 0,
            muarrif: 0,
            sanf: None,
            mahjuz: vec![0u32; jeel.adad_mahjuz()],
            hawd: Vec::new(),
            kharijiya: Vec::new(),
            daght: None,
        }
    }

    /// A minimal Godot 4 wrapper — [`GhilafMawrid::li_jeel`] with
    /// [`JeelMawrid::Rabi`].
    #[must_use]
    pub fn jadeed(naw: &str, muharrik: (u32, u32)) -> Self {
        Self::li_jeel(naw, muharrik, JeelMawrid::Rabi)
    }

    /// Which generation this wrapper is shaped for, taken from its own format
    /// version.
    #[must_use]
    pub const fn jeel(&self) -> JeelMawrid {
        if self.bi_alam() { JeelMawrid::Rabi } else { JeelMawrid::Thalith }
    }
}

/// One property of a resource: its name, its value, and how the name was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Khasiya {
    /// The property's name.
    pub ism: String,
    /// Its value.
    pub qeema: Qeema,
    /// The string map index the name was stored as, or [`None`] when it was
    /// written inline. Preserved because both spellings are legal and a rewrite
    /// that chose the other one would move every byte after it.
    fahras_ism: Option<u32>,
}

impl Khasiya {
    /// A property whose name will be written inline.
    #[must_use]
    pub fn jadeeda(ism: &str, qeema: Qeema) -> Self {
        Self { ism: ism.to_owned(), qeema, fahras_ism: None }
    }

    /// The string map index the name was stored as, if it was stored as one.
    #[must_use]
    pub const fn fahras_ism(&self) -> Option<u32> {
        self.fahras_ism
    }
}

/// A binary resource holding exactly one translation.
///
/// This is the type the round-trip contract applies to: read it, write it back
/// unedited, and the bytes match. [`MawridTarjama::ghayyir`] replaces one
/// property's value and leaves the property order, the string map, the reserved
/// words and every other property exactly as the file had them, which is what
/// makes reinjection a change to the strings and provably nothing else.
///
/// [`Tarjama`] and [`TarjamaMurakkaza`] are views onto it: they read the
/// properties out into a shape a translator can work with, and build a fresh
/// minimal resource on the way back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawridTarjama {
    ghilaf: GhilafMawrid,
    masar_dakhili: String,
    naw_dakhili: String,
    khasais: Vec<Khasiya>,
    khatima: bool,
}

impl MawridTarjama {
    /// The wrapper.
    #[must_use]
    pub const fn ghilaf(&self) -> &GhilafMawrid {
        &self.ghilaf
    }

    /// The class name on the resource record, which is the one the engine
    /// instantiates.
    #[must_use]
    pub fn naw(&self) -> &str {
        &self.naw_dakhili
    }

    /// The internal resource's own path, normally `local://` and an index.
    #[must_use]
    pub fn masar_dakhili(&self) -> &str {
        &self.masar_dakhili
    }

    /// Every property, in the order the file stored them.
    #[must_use]
    pub fn khasais(&self) -> &[Khasiya] {
        &self.khasais
    }

    /// One property's value by name.
    #[must_use]
    pub fn khasiya(&self, ism: &str) -> Option<&Qeema> {
        self.khasais.iter().find(|khasiya| khasiya.ism == ism).map(|khasiya| &khasiya.qeema)
    }

    /// Replaces one property's value in place, leaving everything else alone.
    ///
    /// Returns whether the property was there. A property that is not present is
    /// not added, because the set of properties a resource carries is decided by
    /// its class and inventing one would produce a file the engine reads and then
    /// ignores a field of.
    pub fn ghayyir(&mut self, ism: &str, qeema: Qeema) -> bool {
        for khasiya in &mut self.khasais {
            if khasiya.ism == ism {
                khasiya.qeema = qeema;
                return true;
            }
        }
        false
    }

    /// Whether the class name is one of the two translation classes.
    #[must_use]
    pub fn tarjama(&self) -> bool {
        matches!(self.naw_dakhili.as_str(), NAW_BASITA | NAW_MURAKKAZA | NAW_MURAKKAZA_QADEEM)
    }

    /// Whether the class is one of the hash table forms.
    #[must_use]
    pub fn murakkaza(&self) -> bool {
        matches!(self.naw_dakhili.as_str(), NAW_MURAKKAZA | NAW_MURAKKAZA_QADEEM)
    }

    /// Builds a resource around a class name and a property list, for one
    /// engine generation.
    ///
    /// The internal resource path is `local://1`, which is what Godot's own
    /// saver writes for a file holding a single unnamed resource: the saver
    /// numbers the resources it is about to write from one, and the value ends
    /// up as the sub-path the loader names the resource by.
    #[must_use]
    pub fn li_jeel(
        naw: &str,
        muharrik: (u32, u32),
        jeel: JeelMawrid,
        khasais: Vec<Khasiya>,
    ) -> Self {
        Self {
            ghilaf: GhilafMawrid::li_jeel(naw, muharrik, jeel),
            masar_dakhili: MASAR_DAKHILI.to_owned(),
            naw_dakhili: naw.to_owned(),
            khasais,
            khatima: true,
        }
    }

    /// Builds a Godot 4 resource — [`MawridTarjama::li_jeel`] with
    /// [`JeelMawrid::Rabi`].
    #[must_use]
    pub fn min_khasais(naw: &str, muharrik: (u32, u32), khasais: Vec<Khasiya>) -> Self {
        Self::li_jeel(naw, muharrik, JeelMawrid::Rabi, khasais)
    }

    /// Reads a resource from a file.
    ///
    /// # Errors
    ///
    /// Everything [`super::iqra_malaf`] and [`Mawrid::min_bayt`] raise, with the
    /// path filled into the refusals that could not name it.
    pub fn min_malaf(masar: &Path) -> Result<Self, KhataGodot> {
        let bayt = iqra_malaf(masar)?;
        Self::min_bayt(&bayt).map_err(|khata| super::sammi_masar(khata, masar))
    }

    /// Writes the resource to a file.
    ///
    /// # Errors
    ///
    /// Everything [`Mawrid::ila_bayt`] and [`super::uktub_malaf`] raise.
    pub fn ila_malaf(&self, masar: &Path) -> Result<(), KhataGodot> {
        uktub_malaf(masar, &self.ila_bayt()?)
    }

    /// Reads one property name, which is either a string map index or an inline
    /// string flagged in the same word.
    fn ism_khasiya(
        qari: &mut Qari<'_>,
        hawd: &[String],
    ) -> Result<(String, Option<u32>), KhataGodot> {
        let raas = qari.iqra_u32("a property name")?;
        if raas & ALAM_ISM_MUDMAJ != 0 {
            let tul = u64::from(raas & !ALAM_ISM_MUDMAJ);
            return Ok((nass_min_tul(qari, "an inline property name", tul)?, None));
        }
        let fahras = hajm_usize(u64::from(raas)).unwrap_or(usize::MAX);
        let hadd = tul_u64(hawd.len());
        let ism = hawd.get(fahras).cloned().ok_or_else(|| {
            qari.talif("a property name outside the string map", u64::from(raas), hadd)
        })?;
        Ok((ism, Some(raas)))
    }
}

impl Mawrid for MawridTarjama {
    const ISM: &'static str = ISM;

    /// Reads a whole `.translation`, expanding an `RSCC` wrapper first if there
    /// is one.
    ///
    /// The order of the checks matters in one place above all: the **format
    /// version** is read and bounded before the fifty-six bytes after the
    /// metadata offset are interpreted, because those bytes are fourteen reserved
    /// words in one generation and flags plus a UID plus eleven reserved words in
    /// the other, and nothing else in the file distinguishes them.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::SihrGhayrMutabaq`] with an empty path when the magic is
    /// neither `RSRC` nor `RSCC`, or when an `RSCC` expands to something that is
    /// not a resource; [`KhataGodot::IsdarGhayrMadum`] above
    /// [`ISDAR_SIGHA_AQSA`]; [`KhataGodot::TarjamaTalifa`] for a big-endian or
    /// double-precision file, for a resource that does not hold exactly one
    /// internal resource, and for a variant tag outside [`Qeema`]; plus
    /// everything [`fukk_daght`], [`Qeema::iqra`] and [`super::tahaqquq_adad`]
    /// raise.
    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot> {
        if bayt.get(..SIHR_RSCC.len()) == Some(SIHR_RSCC.as_slice()) {
            let (naw, khaam) = fukk_daght(bayt)?;
            // Checked rather than recursed into blindly: a stream that expands to
            // another compressed stream would otherwise recurse until the stack
            // ran out, and no engine writes one.
            if khaam.get(..SIHR_RSRC.len()) != Some(SIHR_RSRC.as_slice()) {
                return Err(KhataGodot::SihrGhayrMutabaq { masar: PathBuf::new() });
            }
            let mut dakhil = Self::min_bayt(&khaam)?;
            dakhil.ghilaf.daght = Some(naw);
            return Ok(dakhil);
        }
        let mut qari = Qari::jadeed(ISM, bayt);
        if qari.iqra_masfufa::<4>("the resource magic")? != SIHR_RSRC {
            return Err(KhataGodot::SihrGhayrMutabaq { masar: PathBuf::new() });
        }
        if qari.iqra_u32("the endianness flag")? != 0 {
            return Err(talifa("a big-endian resource, which this build does not read"));
        }
        if qari.iqra_u32("the double-precision flag")? != 0 {
            return Err(talifa("a resource whose reals are 64-bit, which widens every float"));
        }
        let muharrik = (
            qari.iqra_u32("the engine major version")?,
            qari.iqra_u32("the engine minor version")?,
        );
        let isdar = qari.iqra_u32("the resource format version")?;
        if isdar > ISDAR_SIGHA_AQSA {
            return Err(KhataGodot::IsdarGhayrMadum { wujid: isdar, aqsa: ISDAR_SIGHA_AQSA });
        }
        let naw = iqra_nass(&mut qari, "the resource class name")?;
        let izahat_wasf = qari.iqra_u64("the import metadata offset")?;

        let bi_alam = isdar >= ISDAR_SIGHA_BI_ALAM;
        let (alam, muarrif, sanf) = if bi_alam {
            let alam = qari.iqra_u32("the format flags")?;
            let muarrif = qari.iqra_u64("the resource identifier")?;
            let sanf = if alam & ALAM_SANF != 0 {
                Some(iqra_nass(&mut qari, "the script class name")?)
            } else {
                None
            };
            (alam, muarrif, sanf)
        } else {
            (0, 0, None)
        };
        let adad_mahjuz = if bi_alam { ADAD_MAHJUZ_HADITH } else { ADAD_MAHJUZ_QADEEM };
        let mut mahjuz = Vec::with_capacity(adad_mahjuz);
        for _ in 0..adad_mahjuz {
            mahjuz.push(qari.iqra_u32("a reserved resource word")?);
        }

        let haql = "the string map size";
        let adad = u64::from(qari.iqra_u32(haql)?);
        let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_MAWARID, 4, qari.baqi())?;
        let mut hawd = Vec::with_capacity(matlub);
        for _ in 0..matlub {
            hawd.push(iqra_nass(&mut qari, "a string map entry")?);
        }

        let haql = "the external resource count";
        let adad = u64::from(qari.iqra_u32(haql)?);
        let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_MAWARID, 8, qari.baqi())?;
        let mut kharijiya = Vec::with_capacity(matlub);
        for _ in 0..matlub {
            let naw_kharij = iqra_nass(&mut qari, "an external resource's class")?;
            let muarrif_kharij = if alam & ALAM_MUARRIFAT != 0 {
                Some(qari.iqra_u64("an external resource's identifier")?)
            } else {
                None
            };
            let masar_kharij = iqra_nass(&mut qari, "an external resource's path")?;
            kharijiya.push(MawridKhariji {
                naw: naw_kharij,
                muarrif: muarrif_kharij,
                masar: masar_kharij,
            });
        }

        let haql = "the internal resource count";
        let adad = u64::from(qari.iqra_u32(haql)?);
        let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_MAWARID, 12, qari.baqi())?;
        if matlub != 1 {
            // A `.translation` is one resource in one file. Anything else is a
            // scene or a packed bundle, and rewriting one property of it through
            // a reader built for translations is how a game loses an asset.
            return Err(talifa("a resource file that does not hold exactly one resource"));
        }
        let masar_dakhili = iqra_nass(&mut qari, "the internal resource's path")?;
        let izaha = qari.iqra_u64("the internal resource's offset")?;

        let ghilaf = GhilafMawrid {
            muharrik,
            isdar,
            naw,
            izahat_wasf,
            alam,
            muarrif,
            sanf,
            mahjuz,
            hawd,
            kharijiya,
            daght: None,
        };

        qari.iqfiz("the internal resource's offset", izaha)?;
        let naw_dakhili = iqra_nass(&mut qari, "the internal resource's class name")?;
        let haql = "the property count";
        let adad = u64::from(qari.iqra_u32(haql)?);
        let matlub = tahaqquq_adad(ISM, haql, adad, AQSA_KHASAIS, 8, qari.baqi())?;
        let mut khasais = Vec::with_capacity(matlub);
        for _ in 0..matlub {
            let (ism, fahras_ism) = Self::ism_khasiya(&mut qari, &ghilaf.hawd)?;
            let qeema = Qeema::iqra(&mut qari, 0)?;
            khasais.push(Khasiya { ism, qeema, fahras_ism });
        }

        let khatima = bayt
            .len()
            .checked_sub(SIHR_RSRC.len())
            .and_then(|akhir| bayt.get(akhir..))
            == Some(SIHR_RSRC.as_slice());
        Ok(Self { ghilaf, masar_dakhili, naw_dakhili, khasais, khatima })
    }

    /// Writes the resource back out.
    ///
    /// Byte-identical to what [`Mawrid::min_bayt`] read when nothing was edited,
    /// with one stated exception: an `RSCC` file comes back as an `RSRC` whose
    /// content matches, because no reader can reproduce a compressor's output.
    ///
    /// The internal resource's offset is the one place a value is recomputed
    /// rather than carried, and it has to be: an edit that lengthens a property
    /// does not move the record, but an edit to the string map would, and a
    /// carried offset would then point into the middle of a string. The engine's
    /// own saver writes the record immediately after the tables, so the
    /// recomputed offset equals the stored one for every file it produced.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when a table or a string is longer than its
    /// length field allows, and [`KhataGodot::TarjamaTalifa`] when a property name
    /// claims a string map index the map does not hold, or when a property holds
    /// a variant tag the generation this wrapper declares has no case for — see
    /// [`tahaqquq_awsaf`].
    fn ila_bayt(&self) -> Result<Vec<u8>, KhataGodot> {
        let mut katib = Katib::bi_siaa(0);
        let ghilaf = &self.ghilaf;
        let jeel = ghilaf.jeel();
        for khasiya in &self.khasais {
            tahaqquq_awsaf(jeel, &khasiya.qeema, 0)?;
        }
        katib.uktub_bayt(&SIHR_RSRC);
        katib.uktub_u32(0);
        katib.uktub_u32(0);
        katib.uktub_u32(ghilaf.muharrik.0);
        katib.uktub_u32(ghilaf.muharrik.1);
        katib.uktub_u32(ghilaf.isdar);
        uktub_nass(&mut katib, "the resource class name", &ghilaf.naw)?;
        katib.uktub_u64(ghilaf.izahat_wasf);
        if ghilaf.bi_alam() {
            katib.uktub_u32(ghilaf.alam);
            katib.uktub_u64(ghilaf.muarrif);
            if let Some(sanf) = &ghilaf.sanf {
                uktub_nass(&mut katib, "the script class name", sanf)?;
            }
        }
        for kalima in &ghilaf.mahjuz {
            katib.uktub_u32(*kalima);
        }
        katib.uktub_u32(tul_u32("the string map size", ghilaf.hawd.len())?);
        for nass in &ghilaf.hawd {
            uktub_nass(&mut katib, "a string map entry", nass)?;
        }
        katib.uktub_u32(tul_u32("the external resource count", ghilaf.kharijiya.len())?);
        for kharij in &ghilaf.kharijiya {
            uktub_nass(&mut katib, "an external resource's class", &kharij.naw)?;
            if let Some(muarrif) = kharij.muarrif {
                katib.uktub_u64(muarrif);
            }
            uktub_nass(&mut katib, "an external resource's path", &kharij.masar)?;
        }
        katib.uktub_u32(1);
        uktub_nass(&mut katib, "the internal resource's path", &self.masar_dakhili)?;
        // Written as a placeholder and filled in once the tables are behind us,
        // because the offset is the length of everything above it.
        let makan = katib.mawqi();
        katib.uktub_u64(0);
        let izaha = tul_u64(katib.mawqi());
        katib.uktub_u64_fi(ISM, "the internal resource's offset", makan, izaha)?;

        uktub_nass(&mut katib, "the internal resource's class name", &self.naw_dakhili)?;
        katib.uktub_u32(tul_u32("the property count", self.khasais.len())?);
        for khasiya in &self.khasais {
            if let Some(fahras) = khasiya.fahras_ism {
                let mawjud = hajm_usize(u64::from(fahras))
                    .and_then(|fahras| ghilaf.hawd.get(fahras));
                if mawjud != Some(&khasiya.ism) {
                    return Err(talifa(
                        "a property name whose string map index does not hold it",
                    ));
                }
                katib.uktub_u32(fahras);
            } else {
                let tul = tul_u64(khasiya.ism.len()).saturating_add(1);
                let muallan = tul_u32("an inline property name", hajm_usize(tul).unwrap_or(0))?;
                katib.uktub_u32(muallan | ALAM_ISM_MUDMAJ);
                katib.uktub_bayt(khasiya.ism.as_bytes());
                katib.uktub_u8(0);
            }
            khasiya.qeema.uktub(&mut katib)?;
        }
        if self.khatima {
            katib.uktub_bayt(&SIHR_RSRC);
        }
        Ok(katib.ila_vec())
    }
}

/// Refuses a value carrying a variant tag the target generation cannot read.
///
/// Godot 3's binary loader is a `switch` over its own tag list, and its default
/// arm is `ERR_FILE_CORRUPT` for the whole resource — so one Godot 4 tag
/// anywhere inside a property costs the entire translation, not that property.
/// The check is here, at the one place a resource becomes bytes, rather than at
/// each of the constructors that could produce such a value: a value assembled
/// correctly and written for the wrong engine is the same file as a value
/// assembled wrongly, and only the writer knows which engine was asked for.
///
/// # Errors
///
/// [`KhataGodot::TarjamaTalifa`] naming the situation, and for nesting past
/// [`AQSA_UMQ`] — the same bound the reader descends under, so a value this
/// cannot finish walking is refused rather than written unchecked.
pub fn tahaqquq_awsaf(jeel: JeelMawrid, qeema: &Qeema, umq: u32) -> Result<(), KhataGodot> {
    if umq > AQSA_UMQ {
        return Err(talifa("a property nested deeper than this writer descends"));
    }
    if !jeel.yaqra_wasf(qeema.wasf()) {
        return Err(talifa(
            "a property whose variant tag the engine generation this resource is written for \
             has no case for, which its loader answers by refusing the whole resource",
        ));
    }
    let dakhil = umq.saturating_add(1);
    match qeema {
        Qeema::Qamus { azwaj, .. } => {
            for (miftah, dakhili) in azwaj {
                tahaqquq_awsaf(jeel, miftah, dakhil)?;
                tahaqquq_awsaf(jeel, dakhili, dakhil)?;
            }
        }
        Qeema::Masfufa { anasir, .. } => {
            for unsur in anasir {
                tahaqquq_awsaf(jeel, unsur, dakhil)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// One UTF-8 byte as the engine's hash loop widens it.
///
/// The engine walks a `const char *` and writes `uint32_t(*p_str)`, so the
/// widening is the target compiler's `char` signedness. See [`basma_godot`].
fn wahda_basma(bayt: u8) -> u32 {
    // `i8` then `i32` is the sign extension spelled without a lint-restricted
    // cast; the last step reinterprets the bits, which is what the C conversion
    // to `uint32_t` does.
    let muwaqqa = i32::from(i8::from_ne_bytes([bayt]));
    u32::from_ne_bytes(muwaqqa.to_ne_bytes())
}

/// Godot's translation hash, exactly as the engine computes it.
///
/// ```text
/// basma(daala, nass):
///     if daala == 0 { daala = 0x0100_0193 }
///     for each UTF-8 byte b of nass:
///         daala = (daala * 0x0100_0193) ^ (uint32_t)(char)b
///     daala
/// ```
///
/// Four details are load bearing and each of them is a way to be silently wrong:
///
/// * the multiplier is FNV's 32-bit prime, and so is the seed — this is not FNV's
///   offset basis, which is what a reader reaching for a known constant would
///   use;
/// * the multiply comes **before** the exclusive-or, which is FNV-1's order and
///   not FNV-1a's;
/// * the bytes are the string's **UTF-8**, not its code points;
/// * each byte is widened **signed**. The engine's line is
///   `d = (d * 0x1000193) ^ uint32_t(*p_str)` with `p_str` a `const char *`, so
///   the widening is whatever `char` means to the compiler that built the
///   engine, and that is signed on x86-64 Windows, x86-64 Linux and both Apple
///   targets — every platform Taarib ships for. A byte of 0xD8 therefore
///   contributes `0xFFFF_FFD8`, not `0x0000_00D8`.
///
/// The last of those was measured rather than reasoned about. A
/// `PHashTranslation` written by Godot 3.6 itself, holding the source strings
/// `Hello`, `New Game`, `Café — “quoted”` and `日本語`, was walked with both
/// widenings: the two agree on the pure-ASCII pair — every byte is below 0x80,
/// so there is nothing to sign-extend — and only the signed one finds the two
/// with bytes above 0x7F. The unsigned reading lands on an empty slot and
/// reports the string as untranslated. `tests/thalith.rs` keeps that file and
/// re-runs the comparison.
///
/// Where this is *not* the engine's arithmetic is a Linux ARM or Android build,
/// whose ABI makes `char` unsigned. Godot has the same divergence against
/// itself there — a table an editor generated on a desktop cannot be searched
/// for a non-ASCII source by an engine built for those targets — and this
/// function follows the desktop, because that is where this product's games run.
/// Sources that are pure ASCII, which is nearly all of them, are unaffected
/// either way.
///
/// A table built with any of those wrong is a well-formed resource that the
/// engine loads without one warning and then finds nothing in. Every lookup
/// misses, every string falls back to its source text, and the game runs
/// untranslated with a translation installed.
///
/// Multiplication wraps by design: the engine's `uint32_t` arithmetic wraps, and
/// a checked or saturating version would agree with it for a while and then stop.
#[must_use]
pub fn basma_godot(daala: u32, nass: &str) -> u32 {
    let mut halat = if daala == 0 { THABIT_BASMA } else { daala };
    for wahid in nass.as_bytes() {
        halat = halat.wrapping_mul(THABIT_BASMA) ^ wahda_basma(*wahid);
    }
    halat
}

/// The primes Godot sizes a hash table from, in the engine's own order.
const AWWALIYAT: [u32; 29] = [
    5, 13, 23, 47, 97, 193, 389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317, 196_613,
    393_241, 786_433, 1_572_869, 3_145_739, 6_291_469, 12_582_917, 25_165_843, 50_331_653,
    100_663_319, 201_326_611, 402_653_189, 805_306_457, 1_610_612_741,
];

/// The first prime in the engine's table strictly greater than `adad`.
///
/// The table is the engine's, not an arbitrary prime sequence, because the hash
/// table's length is the modulus every lookup uses: a table sized from a
/// different prime puts every string in a different bucket, and the engine's
/// lookup would find an empty slot where this build put an entry.
///
/// Saturates at the largest prime in the table rather than returning zero the way
/// the engine does, because zero would become a modulus. Nothing reaches it:
/// [`AQSA_RASAIL`] is three orders of magnitude below.
#[must_use]
pub fn awwal_akbar(adad: u32) -> u32 {
    for awwal in AWWALIYAT {
        if awwal > adad {
            return awwal;
        }
    }
    AWWALIYAT.last().copied().unwrap_or(5)
}

/// Which shape a plain translation's `messages` property was stored in.
///
/// Both spellings hold the same information, neither can be derived from the
/// other's absence, and writing back the wrong one produces a resource the engine
/// reads into an empty message map — so the shape is part of the value, exactly
/// as a string's encoding is in the sibling adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShaklRasail {
    /// Godot 4: a `Dictionary` keyed by source `StringName`.
    #[default]
    Qamus,
    /// Godot 3: a flat `PoolStringArray` of alternating source and translation.
    Masfufa,
}

/// A plain `Translation`: a list of source strings and what to show instead.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tarjama {
    thaqafa: String,
    rasail: Vec<(String, String)>,
    shakl: ShaklRasail,
}

impl Tarjama {
    /// An empty translation for a locale, in the given shape.
    #[must_use]
    pub fn jadeeda(thaqafa: &str, shakl: ShaklRasail) -> Self {
        Self { thaqafa: thaqafa.to_owned(), rasail: Vec::new(), shakl }
    }

    /// The locale this translation registers itself under.
    #[must_use]
    pub fn thaqafa(&self) -> &str {
        &self.thaqafa
    }

    /// Sets the locale.
    pub fn dhaa_thaqafa(&mut self, thaqafa: &str) {
        thaqafa.clone_into(&mut self.thaqafa);
    }

    /// The shape the messages were stored in, and will be written back in.
    #[must_use]
    pub const fn shakl(&self) -> ShaklRasail {
        self.shakl
    }

    /// Every message, in file order.
    #[must_use]
    pub fn rasail(&self) -> &[(String, String)] {
        &self.rasail
    }

    /// How many messages there are.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.rasail.len()
    }

    /// Whether there are no messages.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.rasail.is_empty()
    }

    /// Adds a message, replacing any existing one for the same source.
    ///
    /// Replacing rather than appending is the engine's own behaviour: both
    /// storage shapes become one map, and a duplicate source would silently keep
    /// whichever copy the map happened to insert last.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when the translation already holds
    /// [`AQSA_RASAIL`] messages.
    pub fn daa(&mut self, masdar: &str, hadaf: &str) -> Result<(), KhataGodot> {
        for (mawjud, qeema) in &mut self.rasail {
            if mawjud.as_str() == masdar {
                hadaf.clone_into(qeema);
                return Ok(());
            }
        }
        if tul_u64(self.rasail.len()) >= AQSA_RASAIL {
            return Err(KhataGodot::HajmMufrit {
                haql: "the translation's message count",
                qeema: tul_u64(self.rasail.len()).saturating_add(1),
                saqf: AQSA_RASAIL,
            });
        }
        self.rasail.push((masdar.to_owned(), hadaf.to_owned()));
        Ok(())
    }

    /// The translation of an exact source string.
    #[must_use]
    pub fn ibhath(&self, masdar: &str) -> Option<&str> {
        self.rasail
            .iter()
            .find(|(mawjud, _)| mawjud.as_str() == masdar)
            .map(|(_, hadaf)| hadaf.as_str())
    }

    /// Reads a plain translation out of a resource.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::TarjamaTalifa`] when the resource is not a `Translation`,
    /// when it has no `messages` property, when that property is neither a
    /// dictionary nor a string array, when a dictionary key or value is not a
    /// string, or when a flat array has an odd length — which would leave one
    /// source with no translation and shift every pair after it.
    pub fn min_mawrid(mawrid: &MawridTarjama) -> Result<Self, KhataGodot> {
        if mawrid.naw_dakhili != NAW_BASITA {
            return Err(talifa("a resource that is not a plain Translation"));
        }
        let thaqafa = mawrid
            .khasiya(KHASIYAT_THAQAFA)
            .and_then(Qeema::nass)
            .unwrap_or_default()
            .to_owned();
        let khaam = mawrid
            .khasiya(KHASIYAT_RASAIL)
            .ok_or_else(|| talifa("a Translation with no messages property"))?;
        let (rasail, shakl) = match khaam {
            Qeema::Qamus { azwaj, .. } => {
                let mut rasail = Vec::with_capacity(azwaj.len());
                for (miftah, qeema) in azwaj {
                    let masdar = miftah
                        .nass()
                        .ok_or_else(|| talifa("a message key that is not a string"))?;
                    let hadaf = qeema
                        .nass()
                        .ok_or_else(|| talifa("a message value that is not a string"))?;
                    rasail.push((masdar.to_owned(), hadaf.to_owned()));
                }
                (rasail, ShaklRasail::Qamus)
            }
            Qeema::Nusus(nusus) => {
                if nusus.len() & 1 != 0 {
                    return Err(talifa("a flat message array with an odd number of entries"));
                }
                let mut rasail = Vec::with_capacity(nusus.len());
                for zawj in nusus.chunks_exact(2) {
                    let masdar = zawj.first().cloned().unwrap_or_default();
                    let hadaf = zawj.get(1).cloned().unwrap_or_default();
                    rasail.push((masdar, hadaf));
                }
                (rasail, ShaklRasail::Masfufa)
            }
            _ => {
                return Err(talifa("a messages property that is neither a dictionary nor an array"));
            }
        };
        Ok(Self { thaqafa, rasail, shakl })
    }

    /// The `messages` property in one generation's shape.
    ///
    /// The shape comes from the generation being written for and **not** from
    /// [`Tarjama::shakl`], which records what the file this was read from
    /// happened to hold. The two are the same for a resource that is going back
    /// to the engine it came from, and they are not the same for a translation
    /// lifted out of a Godot 4 game and written for a Godot 3 one — where
    /// carrying the shape across would produce a `Dictionary` that Godot 3's
    /// `_set_messages(const PoolVector<String> &)` silently fails to convert.
    fn qeemat_rasail(&self, jeel: JeelMawrid) -> Qeema {
        match jeel.shakl_rasail() {
            ShaklRasail::Qamus => Qeema::Qamus {
                musharak: false,
                azwaj: self
                    .rasail
                    .iter()
                    .map(|(masdar, hadaf)| {
                        (Qeema::IsmNass(masdar.clone()), Qeema::IsmNass(hadaf.clone()))
                    })
                    .collect(),
            },
            ShaklRasail::Masfufa => {
                let mut nusus = Vec::with_capacity(self.rasail.len().saturating_mul(2));
                for (masdar, hadaf) in &self.rasail {
                    nusus.push(masdar.clone());
                    nusus.push(hadaf.clone());
                }
                Qeema::Nusus(nusus)
            }
        }
    }

    /// Builds a fresh resource holding this translation, for one generation.
    ///
    /// The property order is the engine's — `messages` then `locale`, which is
    /// the order `Translation` registers them in and the order Godot 3.6's own
    /// saver was observed to write — and both names go through the string map
    /// rather than being written inline, because that is what the engine's own
    /// saver does.
    #[must_use]
    pub fn ila_mawrid(&self, jeel: JeelMawrid, muharrik: (u32, u32)) -> MawridTarjama {
        let mut mawrid = MawridTarjama::li_jeel(NAW_BASITA, muharrik, jeel, Vec::new());
        mawrid.ghilaf.hawd =
            vec![KHASIYAT_RASAIL.to_owned(), KHASIYAT_THAQAFA.to_owned()];
        mawrid.khasais = vec![
            Khasiya {
                ism: KHASIYAT_RASAIL.to_owned(),
                qeema: self.qeemat_rasail(jeel),
                fahras_ism: Some(0),
            },
            Khasiya {
                ism: KHASIYAT_THAQAFA.to_owned(),
                qeema: Qeema::Nass(self.thaqafa.clone()),
                fahras_ism: Some(1),
            },
        ];
        mawrid
    }

    /// Reads a plain translation from a whole `.translation`.
    ///
    /// # Errors
    ///
    /// Everything [`MawridTarjama`]'s reader and [`Tarjama::min_mawrid`] raise.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot> {
        Self::min_mawrid(&MawridTarjama::min_bayt(bayt)?)
    }

    /// Writes a fresh `.translation` holding this translation.
    ///
    /// Not a byte-identical round trip and not meant to be: the resource is built
    /// from scratch, so a file read through [`Tarjama::min_bayt`] and written back
    /// through here keeps every message and drops whatever else the original
    /// wrapper carried. Reinjection into a file a game shipped goes through
    /// [`MawridTarjama::ghayyir`] instead, which preserves it.
    ///
    /// # Errors
    ///
    /// Everything [`Mawrid::ila_bayt`] raises.
    pub fn ila_bayt(&self, jeel: JeelMawrid, muharrik: (u32, u32)) -> Result<Vec<u8>, KhataGodot> {
        self.ila_mawrid(jeel, muharrik).ila_bayt()
    }

    /// Writes a fresh `.translation` to a file.
    ///
    /// # Errors
    ///
    /// Everything [`MawridTarjama::ila_malaf`] raises.
    pub fn ila_malaf(
        &self,
        masar: &Path,
        jeel: JeelMawrid,
        muharrik: (u32, u32),
    ) -> Result<(), KhataGodot> {
        self.ila_mawrid(jeel, muharrik).ila_malaf(masar)
    }
}

/// How many seeds the bucket search tries before giving up.
///
/// A million. The search normally succeeds on the first or second seed; it can
/// never succeed when two sources in one bucket are the *same string*, because no
/// seed separates a value from itself. The bound is what turns that into a named
/// refusal instead of a hang.
const AQSA_MUHAWALAT: u32 = 1_000_000;

/// A word of a packed integer array read as unsigned, with no lossy cast.
const fn kalima_u32(qeema: i32) -> u32 {
    u32::from_le_bytes(qeema.to_le_bytes())
}

/// An unsigned word stored back into a packed integer array, with no lossy cast.
///
/// The keys in a bucket are hashes, so about half of them are above `i32::MAX`
/// and every one of those is a negative word in the file. That is what the engine
/// writes and what it compares against.
const fn kalima_i32(qeema: u32) -> i32 {
    i32::from_le_bytes(qeema.to_le_bytes())
}

/// An `OptimizedTranslation`: the same messages as a perfect hash table.
///
/// Three parallel arrays and nothing else. See this module's header for the
/// layout, the lookup, and the two properties that matter most: the hash must be
/// the engine's exactly, and the table cannot be enumerated because the source
/// strings are not in it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TarjamaMurakkaza {
    thaqafa: String,
    jadwal: Vec<i32>,
    dilaa: Vec<i32>,
    hawd: Vec<u8>,
}

impl TarjamaMurakkaza {
    /// The locale this translation registers itself under.
    #[must_use]
    pub fn thaqafa(&self) -> &str {
        &self.thaqafa
    }

    /// The top-level table: one word per slot, `-1` where the slot is empty.
    #[must_use]
    pub fn jadwal(&self) -> &[i32] {
        &self.jadwal
    }

    /// The bucket array.
    #[must_use]
    pub fn dilaa(&self) -> &[i32] {
        &self.dilaa
    }

    /// The string pool.
    #[must_use]
    pub fn hawd(&self) -> &[u8] {
        &self.hawd
    }

    /// One word of the bucket array.
    fn kalima(&self, fahras: usize, haql: &'static str) -> Result<u32, KhataGodot> {
        self.dilaa.get(fahras).copied().map(kalima_u32).ok_or_else(|| KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql,
            qeema: tul_u64(fahras),
            hadd: tul_u64(self.dilaa.len()),
        })
    }

    /// Looks a source string up, exactly the way the engine does.
    ///
    /// Hash with seed zero, take the remainder against the table length, read the
    /// bucket index, re-hash with *that bucket's* seed, and scan its elements for
    /// the key. A miss is [`None`] and not an error: an untranslated string is the
    /// ordinary case, and the engine falls back to the source text for it.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when a bucket index, an element or a pool
    /// offset points outside the array that holds it;
    /// [`KhataGodot::DaghtMajhul`] naming [`DAGHT_SMAZ`] when the entry is
    /// compressed, which this build does not expand — see this module's header;
    /// and [`KhataGodot::NassGhayrSalih`] when the pooled bytes are not UTF-8.
    pub fn ibhath(&self, masdar: &str) -> Result<Option<String>, KhataGodot> {
        let hajm = self.jadwal.len();
        if hajm == 0 {
            return Ok(None);
        }
        let khana = hajm_usize(u64::from(basma_godot(0, masdar)))
            .and_then(|basma| basma.checked_rem(hajm))
            .ok_or_else(|| KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "an optimized translation's table length",
                qeema: tul_u64(hajm),
                hadd: tul_u64(usize::MAX),
            })?;
        let raas = self.jadwal.get(khana).copied().unwrap_or(-1);
        if raas == -1 {
            return Ok(None);
        }
        let dalu = usize::try_from(raas).map_err(|_| KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql: "a bucket index that is negative and is not the empty sentinel",
            qeema: u64::from(kalima_u32(raas)),
            hadd: tul_u64(self.dilaa.len()),
        })?;
        let adad = self.kalima(dalu, "a bucket's element count")?;
        let daala = self.kalima(dalu.saturating_add(1), "a bucket's seed")?;
        let basma = basma_godot(daala, masdar);
        let matlub = hajm_usize(u64::from(adad)).unwrap_or(usize::MAX);
        for fahras in 0..matlub {
            let asas = dalu
                .checked_add(TUL_RAAS_DALU)
                .and_then(|raas| {
                    fahras.checked_mul(TUL_UNSUR_DALU).and_then(|zaha| raas.checked_add(zaha))
                })
                .ok_or_else(|| KhataGodot::HawiyaTalifa {
                    ism: ISM,
                    haql: "a bucket element index that overflows",
                    qeema: tul_u64(fahras),
                    hadd: tul_u64(self.dilaa.len()),
                })?;
            if self.kalima(asas, "a bucket element's key")? != basma {
                continue;
            }
            let izaha = self.kalima(asas.saturating_add(1), "a bucket element's pool offset")?;
            let madghut = self.kalima(asas.saturating_add(2), "a bucket element's stored size")?;
            let khaam = self.kalima(asas.saturating_add(3), "a bucket element's expanded size")?;
            if madghut != khaam {
                return Err(KhataGodot::DaghtMajhul { naw: DAGHT_SMAZ });
            }
            return self.nass_min_hawd(izaha, khaam, fahras).map(Some);
        }
        Ok(None)
    }

    /// Decodes one pooled string.
    fn nass_min_hawd(&self, izaha: u32, tul: u32, fahras: usize) -> Result<String, KhataGodot> {
        let bidaya = hajm_usize(u64::from(izaha)).unwrap_or(usize::MAX);
        let nihaya = bidaya.checked_add(hajm_usize(u64::from(tul)).unwrap_or(usize::MAX));
        let khaam = nihaya
            .and_then(|nihaya| self.hawd.get(bidaya..nihaya))
            .ok_or_else(|| KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "a pooled string that reaches past the end of the pool",
                qeema: u64::from(izaha).saturating_add(u64::from(tul)),
                hadd: tul_u64(self.hawd.len()),
            })?;
        // One trailing NUL is dropped if there is one. The engine's generator
        // measures the string with a length that has, in some versions, counted
        // the terminator; dropping it here is correct under either convention and
        // no translation legitimately ends in U+0000.
        let jism = match khaam.split_last() {
            Some((0, jism)) => jism,
            _ => khaam,
        };
        match std::str::from_utf8(jism) {
            Ok(nass) => Ok(nass.to_owned()),
            Err(khata) => Err(KhataGodot::NassGhayrSalih {
                fahras: u32::try_from(fahras).unwrap_or(u32::MAX),
                mawqi: u32::try_from(khata.valid_up_to()).unwrap_or(u32::MAX),
            }),
        }
    }
}

impl TarjamaMurakkaza {
    /// Builds the hash table from a plain translation, the way the engine's own
    /// generator does.
    ///
    /// Sources go into buckets by `basma_godot(0, source) % prime`, where the
    /// prime is [`awwal_akbar`] of the message count — the engine's table, not any
    /// prime, because the table length is the modulus every later lookup uses.
    /// Each non-empty bucket then gets a seed, searched upward from one until no
    /// two of its sources hash alike under it, and that seed's hashes become the
    /// bucket's keys. This is what makes the table perfect: within a bucket the
    /// keys are unique by construction, so a lookup either matches exactly or the
    /// string is not there.
    ///
    /// Every string is stored **uncompressed** — stored length equal to expanded
    /// length, which is the engine's own flag for "not compressed". The engine
    /// compresses with SMAZ and keeps the result only when it is smaller, and for
    /// Arabic it never is: SMAZ's codebook is English words and digrams, so every
    /// byte above 0x7F becomes a literal escape and the pool grows. Writing them
    /// plain is therefore not a simplification, it is the same decision the
    /// engine's generator reaches with this input.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when the message count, the string pool or the
    /// bucket table exceeds its ceiling, and [`KhataGodot::TarjamaTalifa`] when a
    /// bucket holds two identical source strings, which no seed can separate.
    pub fn min_tarjama(asl: &Tarjama) -> Result<Self, KhataGodot> {
        let adad = u32::try_from(asl.rasail().len()).map_err(|_| KhataGodot::HajmMufrit {
            haql: "the translation's message count",
            qeema: tul_u64(asl.rasail().len()),
            saqf: AQSA_RASAIL,
        })?;
        let hajm = hajm_usize(u64::from(awwal_akbar(adad))).ok_or_else(|| KhataGodot::HajmMufrit {
            haql: "the optimized translation's table length",
            qeema: u64::from(awwal_akbar(adad)),
            saqf: AQSA_KALIMAT,
        })?;

        let mut hawd: Vec<u8> = Vec::new();
        let mut mawaqi: Vec<(u32, u32)> = Vec::with_capacity(asl.rasail().len());
        for (_, hadaf) in asl.rasail() {
            let izaha = tul_u32("the string pool", hawd.len())?;
            // The terminator is stored and counted. The engine's generator
            // measures each string with `CharString::size()`, which includes the
            // NUL, and pools the same bytes — so a resource written without it
            // has a different `strings` array and different lengths from one the
            // engine would have written for the same messages. It reads back
            // correctly either way, because the uncompressed path is given an
            // explicit length; matching is what makes a generated table
            // comparable against an engine-written one.
            let tul = tul_u32("a pooled string", hadaf.len().saturating_add(1))?;
            hawd.extend_from_slice(hadaf.as_bytes());
            hawd.push(0);
            mawaqi.push((izaha, tul));
        }
        if tul_u64(hawd.len()) > AQSA_TUL_HAQL {
            return Err(KhataGodot::HajmMufrit {
                haql: "the string pool",
                qeema: tul_u64(hawd.len()),
                saqf: AQSA_TUL_HAQL,
            });
        }

        let mut tawzee: Vec<Vec<usize>> = vec![Vec::new(); hajm];
        for (fahras, (masdar, _)) in asl.rasail().iter().enumerate() {
            let khana = hajm_usize(u64::from(basma_godot(0, masdar)))
                .and_then(|basma| basma.checked_rem(hajm))
                .ok_or_else(|| KhataGodot::HajmMufrit {
                    haql: "the optimized translation's table length",
                    qeema: tul_u64(hajm),
                    saqf: AQSA_KALIMAT,
                })?;
            if let Some(dalu) = tawzee.get_mut(khana) {
                dalu.push(fahras);
            }
        }

        let mut jadwal = vec![-1i32; hajm];
        let mut dilaa: Vec<i32> = Vec::new();
        for (khana, unsur) in tawzee.iter().enumerate() {
            if unsur.is_empty() {
                continue;
            }
            let daala = Self::ijad_daala(asl, unsur)?;
            let mawqi = tul_u32("the bucket table", dilaa.len())?;
            if let Some(makan) = jadwal.get_mut(khana) {
                *makan = kalima_i32(mawqi);
            }
            dilaa.push(kalima_i32(tul_u32("a bucket's element count", unsur.len())?));
            dilaa.push(kalima_i32(daala));
            // The engine collects a bucket's elements in a `Map<uint32_t, int>`
            // keyed by the second-round hash and then walks it front to back, so
            // its buckets come out sorted ascending by key. Sorting here is what
            // makes a generated bucket the same run of words the engine would
            // have produced; the lookup is a scan and would find them in any
            // order, so this is about being comparable rather than about being
            // correct. `ijad_daala` has already established that the keys within
            // one bucket are distinct, so the order is total.
            let mut anasir: Vec<(u32, u32, u32)> = Vec::with_capacity(unsur.len());
            for fahras in unsur {
                let masdar = asl.rasail().get(*fahras).map_or("", |(masdar, _)| masdar.as_str());
                let (izaha, tul) = mawaqi.get(*fahras).copied().unwrap_or((0, 0));
                anasir.push((basma_godot(daala, masdar), izaha, tul));
            }
            anasir.sort_unstable_by_key(|(miftah, _, _)| *miftah);
            for (miftah, izaha, tul) in anasir {
                dilaa.push(kalima_i32(miftah));
                dilaa.push(kalima_i32(izaha));
                // Stored length equal to expanded length is the format's way of
                // saying the string was not compressed.
                dilaa.push(kalima_i32(tul));
                dilaa.push(kalima_i32(tul));
            }
            if tul_u64(dilaa.len()) > AQSA_KALIMAT {
                return Err(KhataGodot::HajmMufrit {
                    haql: "the bucket table",
                    qeema: tul_u64(dilaa.len()),
                    saqf: AQSA_KALIMAT,
                });
            }
        }
        Ok(Self { thaqafa: asl.thaqafa().to_owned(), jadwal, dilaa, hawd })
    }

    /// Finds the smallest seed that separates every source in one bucket.
    fn ijad_daala(asl: &Tarjama, unsur: &[usize]) -> Result<u32, KhataGodot> {
        let mut daala: u32 = 1;
        let mut basmat: Vec<u32> = Vec::with_capacity(unsur.len());
        while daala <= AQSA_MUHAWALAT {
            basmat.clear();
            let mut salih = true;
            for fahras in unsur {
                let masdar = asl.rasail().get(*fahras).map_or("", |(masdar, _)| masdar.as_str());
                let basma = basma_godot(daala, masdar);
                if basmat.contains(&basma) {
                    salih = false;
                    break;
                }
                basmat.push(basma);
            }
            if salih {
                return Ok(daala);
            }
            daala = daala.saturating_add(1);
        }
        // No seed separates a string from itself, so this is not a search that
        // needed longer — it is a message list holding one source twice.
        Err(talifa("two identical source strings in one bucket, which no seed separates"))
    }

    /// Reads an optimized translation out of a resource.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::TarjamaTalifa`] when the resource is not an
    /// `OptimizedTranslation` or `PHashTranslation`, or when one of its three
    /// arrays is missing or is not the type the format gives it.
    pub fn min_mawrid(mawrid: &MawridTarjama) -> Result<Self, KhataGodot> {
        if !mawrid.murakkaza() {
            return Err(talifa("a resource that is not an OptimizedTranslation"));
        }
        let thaqafa = mawrid
            .khasiya(KHASIYAT_THAQAFA)
            .and_then(Qeema::nass)
            .unwrap_or_default()
            .to_owned();
        let jadwal = mawrid
            .khasiya(KHASIYAT_JADWAL)
            .and_then(Qeema::sahihat)
            .ok_or_else(|| talifa("an optimized translation with no hash table"))?
            .to_vec();
        let dilaa = mawrid
            .khasiya(KHASIYAT_DILAA)
            .and_then(Qeema::sahihat)
            .ok_or_else(|| talifa("an optimized translation with no bucket table"))?
            .to_vec();
        let hawd = mawrid
            .khasiya(KHASIYAT_HAWD)
            .and_then(Qeema::bayt)
            .ok_or_else(|| talifa("an optimized translation with no string pool"))?
            .to_vec();
        Ok(Self { thaqafa, jadwal, dilaa, hawd })
    }

    /// Builds a fresh resource holding this table, for one generation.
    ///
    /// Four properties: the locale and the three arrays, in that order, which is
    /// the order Godot 3.6's own saver was observed to write them in. The
    /// `messages` property `Translation` defines is deliberately absent — an
    /// optimized translation's message map is empty by construction, and the
    /// engine's saver leaves it out of such a file too.
    ///
    /// The class name comes from the generation: Godot 3 knows this class as
    /// `PHashTranslation` and has never had one called `OptimizedTranslation`,
    /// so the name is not a spelling preference — it is which class the engine
    /// instantiates, and a name it does not know is a resource that does not
    /// load.
    #[must_use]
    pub fn ila_mawrid(&self, jeel: JeelMawrid, muharrik: (u32, u32)) -> MawridTarjama {
        let naw = jeel.naw_murakkaza();
        let mut mawrid = MawridTarjama::li_jeel(naw, muharrik, jeel, Vec::new());
        mawrid.ghilaf.hawd = vec![
            KHASIYAT_THAQAFA.to_owned(),
            KHASIYAT_JADWAL.to_owned(),
            KHASIYAT_DILAA.to_owned(),
            KHASIYAT_HAWD.to_owned(),
        ];
        mawrid.khasais = vec![
            Khasiya {
                ism: KHASIYAT_THAQAFA.to_owned(),
                qeema: Qeema::Nass(self.thaqafa.clone()),
                fahras_ism: Some(0),
            },
            Khasiya {
                ism: KHASIYAT_JADWAL.to_owned(),
                qeema: Qeema::Sahihat(self.jadwal.clone()),
                fahras_ism: Some(1),
            },
            Khasiya {
                ism: KHASIYAT_DILAA.to_owned(),
                qeema: Qeema::Sahihat(self.dilaa.clone()),
                fahras_ism: Some(2),
            },
            Khasiya {
                ism: KHASIYAT_HAWD.to_owned(),
                qeema: Qeema::Bayt(self.hawd.clone()),
                fahras_ism: Some(3),
            },
        ];
        mawrid
    }

    /// Reads an optimized translation from a whole `.translation`.
    ///
    /// # Errors
    ///
    /// Everything [`MawridTarjama`]'s reader and
    /// [`TarjamaMurakkaza::min_mawrid`] raise.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot> {
        Self::min_mawrid(&MawridTarjama::min_bayt(bayt)?)
    }

    /// Writes a fresh `.translation` holding this table.
    ///
    /// # Errors
    ///
    /// Everything [`Mawrid::ila_bayt`] raises.
    pub fn ila_bayt(&self, jeel: JeelMawrid, muharrik: (u32, u32)) -> Result<Vec<u8>, KhataGodot> {
        self.ila_mawrid(jeel, muharrik).ila_bayt()
    }

    /// Reads an optimized translation from a file.
    ///
    /// # Errors
    ///
    /// Everything [`MawridTarjama::min_malaf`] and
    /// [`TarjamaMurakkaza::min_mawrid`] raise.
    pub fn min_malaf(masar: &Path) -> Result<Self, KhataGodot> {
        Self::min_mawrid(&MawridTarjama::min_malaf(masar)?)
    }

    /// Writes a fresh `.translation` to a file.
    ///
    /// # Errors
    ///
    /// Everything [`MawridTarjama::ila_malaf`] raises.
    pub fn ila_malaf(
        &self,
        masar: &Path,
        jeel: JeelMawrid,
        muharrik: (u32, u32),
    ) -> Result<(), KhataGodot> {
        self.ila_mawrid(jeel, muharrik).ila_malaf(masar)
    }
}
