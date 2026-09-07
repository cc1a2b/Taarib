//! اللوكريس — `.locres`, the compiled resource a shipped game actually loads.
//!
//! A `.locres` is one culture's translated text, compiled by Unreal's own
//! localization commandlet out of the `.manifest` and `.archive` files that
//! never ship. It is a two-level table — namespace, then key — where every entry
//! carries the hash of the source string it was translated from, and, from
//! version 1 onward, an index into a string array at the end of the file rather
//! than the string itself.
//!
//! This module reads and writes all four shapes the format has had.
//!
//! ## The four versions, and what each one added
//!
//! ```text
//! MawridLocres — the header, little-endian
//!
//!   offset  size  field           type      in
//!        0    16  sihr            [u8;16]   v1 v2 v3   the magic GUID; ABSENT in legacy
//!       16     1  isdar           u8        v1 v2 v3   1, 2 or 3
//!       17     8  izahat_hawd     i64       v1 v2 v3   byte offset of the string array
//!       25     4  adad_madakhil   u32          v2 v3   every entry, over every namespace
//!       29     4  adad_fadaat     u32       v1 v2 v3   how many namespaces follow
//!
//!   A legacy file has no header at all: byte 0 is adad_fadaat, as a u32.
//! ```
//!
//! ```text
//! FadaaLocres — one namespace, repeated adad_fadaat times
//!
//!   offset  size  field           type      in
//!        0     4  basma           u32          v2 v3   hash of the namespace
//!        -   var  ism             FString   v1 v2 v3   the namespace itself
//!        -     4  adad            u32       v1 v2 v3   how many entries follow
//!
//! MadkhalLocres — one entry, repeated adad times
//!
//!        0     4  basma           u32          v2 v3   hash of the key
//!        -   var  miftah          FString   v1 v2 v3   the key
//!        -     4  basmat_asl      u32       v1 v2 v3   CRC of the source string
//!        -   var  tarjama         FString   legacy     the translation, inline
//!        -     4  fahras_nass     i32       v1 v2 v3   index into the string array
//!
//! HawdNusus — the string array, at izahat_hawd; v1, v2 and v3 only
//!
//!        0     4  adad            i32       v1 v2 v3   how many strings
//!        -   var  nass            FString   v1 v2 v3   one string
//!        -     4  adad_marja      i32          v2 v3   how many entries reference it
//! ```
//!
//! * **Legacy** ([`IsdarLocres::Qadim`]) has no magic and no version byte. It is
//!   detected by the *absence* of the magic GUID, which is the only way it can
//!   be detected: the file begins directly with a namespace count, and a count
//!   is not a value any prefix check can distinguish from data. Every entry
//!   carries its translation inline, so the same string translated five hundred
//!   times is stored five hundred times.
//! * **Compact** ([`IsdarLocres::Mudmaj`], version 1) added the magic, the
//!   version byte, and the string array. Entries stopped carrying strings and
//!   started carrying indices into it, so a repeated translation is stored once.
//! * **Optimized** ([`IsdarLocres::Muhassan`], version 2) added a hash in front
//!   of every namespace and every key, a total entry count in the header, and a
//!   reference count beside every string in the array. The hashes are what the
//!   runtime looks entries up by; the counts let it size its tables in one pass
//!   instead of growing them.
//! * **`OptimizedCityHash`** ([`IsdarLocres::MuhassanMadina`], version 3) is
//!   byte-for-byte the same layout as version 2 and differs only in how those
//!   hashes are computed: `CityHash64` over the UTF-16 form of the lowercased key,
//!   instead of the CRC-32 version 2 used. The layout being identical is exactly
//!   why the version byte has to be honoured — a reader that treated 3 as 2
//!   would parse the whole file correctly and compute every hash wrongly.
//!
//! ## Why the hashes are the dangerous part
//!
//! At run time the engine does not search a `.locres` by string. It looks an
//! entry up by the hash of its namespace and key, and only compares the strings
//! on a hash match. A file whose hashes were recomputed by a function that
//! disagrees with Unreal's — one bit of endianness, one difference in how the
//! key was lowercased, `CityHash` where the version says CRC — loads without a
//! single error, reports the right number of entries, and then resolves nothing.
//! Every piece of text in the game falls back to its source string, and the
//! symptom is "the translation did not apply", with no message anywhere saying
//! why.
//!
//! This module answers that in two ways. **Hashes read from a file are carried
//! and written back unchanged**, so a round-trip cannot introduce the failure at
//! all. And when a hash genuinely must be computed — for an entry Taarib adds —
//! it is computed by [`basmat_miftah`], which dispatches on the file's own
//! version to [`basmat_crc`] or to [`basmat_madina_miftah`], the latter built on
//! this module's own implementation of `CityHash64` over precisely the bytes
//! Unreal hashes.
//!
//! ## What the hash is fed, and how it is narrowed
//!
//! Three rules, and every one of them was established by computing candidate
//! hashes over the keys in shipped `.locres` files and comparing them against
//! the hashes those files stored — not by reading the engine's source and hoping.
//! The evidence is in `tests/basmat.rs`: 114,918 stored namespace and key hashes
//! from four titles across three engine versions, all reproduced exactly.
//!
//! * **The string is hashed as it stands.** It is *not* lowercased. `"Ok"` and
//!   `"OK"` are two keys with two hashes in a real file, which is the single
//!   observation that settles it, and it is the observation this module used to
//!   get wrong — it lowercased first, so every hash it computed for an added
//!   entry was the hash of a key the engine would never look up.
//! * **The bytes are UTF-16 little-endian code units, with no terminator and no
//!   byte-order mark.** UTF-16 because the engine hashes its own in-memory
//!   `TCHAR` buffer, and little endian by explicit conversion rather than by
//!   reinterpreting memory, because a `.locres` compiled on one machine has to
//!   resolve on every other one.
//! * **The 64-bit result is folded, not truncated.** The stored field is 32 bits
//!   and the engine narrows it through `GetTypeHash(uint64)`, which is the low
//!   word plus twenty-three times the high word — [`tayy_basma`]. Taking the low
//!   half instead is a different number for every key but the ones whose high
//!   word is zero.
//!
//! The empty string hashes to **zero** in both hashed versions. For version 2
//! that falls out of the CRC's own pre- and post-inversion; for version 3 it is
//! a case the engine states separately, and [`basmat_madina_miftah`] states it
//! separately too. It is not a corner: the unnamed namespace is the commonest
//! namespace in a shipped game, and every entry in it sits under a zero hash.
//!
//! ## The round-trip
//!
//! Read a `.locres` and write it straight back and the bytes are identical, for
//! versions 1, 2 and 3 and for legacy. Two regions make that true rather than
//! nearly true: `fajwa`, anything between the end of the namespace table and the
//! start of the string array, and `dhayl`, anything after the last string. Both
//! are empty in every file Unreal writes and both are preserved verbatim,
//! because a region this build does not interpret is a region it must not
//! invent.
//!
//! One layout is refused rather than round-tripped: a file whose string array
//! begins *before* its namespace table ends. Unreal writes the array last, this
//! module writes the array last, and a file with the other order could be read
//! but could not be written back unchanged. Refusing it names the field;
//! rewriting it silently would be the one thing the contract exists to stop.

use std::path::Path;

use super::{
    Katib, Mawrid, NassMukhazzan, Qari, adad_musir, iqra_malaf, tahaqquq_adad, uktub_malaf,
};
use crate::khata::{KhataUnreal, tul_u64};

/// The sixteen bytes every non-legacy `.locres` begins with.
///
/// A GUID, written in Unreal's own byte order rather than a canonical GUID text
/// form, so it is compared as bytes and never parsed.
pub const SIHR: [u8; 16] = [
    0x0E, 0x14, 0x74, 0x75, 0x67, 0x4A, 0x03, 0xFC, 0x4A, 0x15, 0x90, 0x9D, 0xC3, 0x37, 0x7F, 0x1B,
];

/// The format's name in every refusal this module raises.
const ISM: &str = ".locres";

/// The highest version byte this build reads.
pub const AQSA_ISDAR: u32 = 3;

/// The largest number of namespaces one file may declare.
///
/// A million. The largest shipped `.locres` this product has been pointed at has
/// a few thousand namespaces — one per localization target plus one per asset
/// that declares its own — so the ceiling is three orders of magnitude clear of
/// anything real, and still small enough that the reservation it permits is
/// bounded by the second half of [`tahaqquq_adad`]'s check rather than by hope.
pub const AQSA_FADAAT: u64 = 1 << 20;

/// The largest number of entries one namespace may declare.
pub const AQSA_MADAKHIL_FADAA: u64 = 1 << 22;

/// The largest number of entries one file may declare, over every namespace.
///
/// Sixteen million. A large role-playing game compiles about a hundred thousand.
pub const AQSA_MADAKHIL: u64 = 1 << 24;

/// The largest number of strings the string array may declare.
pub const AQSA_NUSUS: u64 = 1 << 24;

/// The fewest bytes one namespace record can occupy, in versions 2 and 3.
const AQALL_FADAA: u64 = 12;

/// The fewest bytes one namespace record can occupy, in legacy and version 1.
const AQALL_FADAA_BILA_BASMA: u64 = 8;

/// The fewest bytes one entry can occupy, in versions 2 and 3.
const AQALL_MADKHAL: u64 = 16;

/// The fewest bytes one entry can occupy, in legacy and version 1.
const AQALL_MADKHAL_BILA_BASMA: u64 = 12;

/// The fewest bytes one string-array element can occupy, in versions 2 and 3.
const AQALL_NASS_HAWD: u64 = 8;

/// The fewest bytes one string-array element can occupy, in version 1.
const AQALL_NASS_HAWD_BILA_MARJA: u64 = 4;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Which of the four shapes a file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IsdarLocres {
    /// No magic, no version byte, no string array; every translation inline.
    Qadim,
    /// Version 1: magic, version byte, and the string array.
    Mudmaj,
    /// Version 2: adds the CRC-32 namespace and key hashes, the header's entry
    /// count, and the per-string reference count.
    Muhassan,
    /// Version 3: version 2's layout with `CityHash64` key hashes.
    MuhassanMadina,
}

impl IsdarLocres {
    /// The byte the file carries, or [`None`] for legacy, which carries none.
    #[must_use]
    pub const fn raqm(self) -> Option<u8> {
        match self {
            Self::Qadim => None,
            Self::Mudmaj => Some(1),
            Self::Muhassan => Some(2),
            Self::MuhassanMadina => Some(3),
        }
    }

    /// The version for a byte read after the magic, or [`None`] for one this
    /// build does not read.
    #[must_use]
    pub const fn min_raqm(raqm: u8) -> Option<Self> {
        match raqm {
            1 => Some(Self::Mudmaj),
            2 => Some(Self::Muhassan),
            3 => Some(Self::MuhassanMadina),
            _ => None,
        }
    }

    /// Whether entries reference the string array instead of carrying their
    /// translation inline.
    #[must_use]
    pub const fn yahwi_hawd(self) -> bool {
        !matches!(self, Self::Qadim)
    }

    /// Whether namespaces and keys carry a hash, and strings a reference count.
    #[must_use]
    pub const fn yahwi_basmat(self) -> bool {
        matches!(self, Self::Muhassan | Self::MuhassanMadina)
    }
}

// ---------------------------------------------------------------------------
// The hashes
// ---------------------------------------------------------------------------

/// The CRC-32 of a string, over its UTF-16 code units widened to 32 bits.
///
/// This is `FCrc::StrCrc32` exactly: the standard reflected CRC-32, pre- and
/// post-inverted, fed four bytes per character because Unreal's own loop shifts
/// each `TCHAR` down a byte at a time four times regardless of how wide a
/// `TCHAR` is. For a UTF-16 code unit that means two significant bytes followed
/// by two zeros, and reproducing the two zeros is not pedantry — dropping them
/// changes every hash in the file.
///
/// Computed bitwise rather than from a lookup table. The table is a kilobyte of
/// static data transcribed from a generator, and the only caller is the path
/// that authors new entries; the bitwise form is the definition itself, so there
/// is nothing to transcribe wrongly.
#[must_use]
pub fn basmat_crc(nass: &str) -> u32 {
    let mut basma = u32::MAX;
    for wahda in nass.encode_utf16() {
        let mut harf = u32::from(wahda);
        let mut marra = 0u8;
        while marra < 4 {
            basma = crc_bayt(basma, u8::try_from(harf & 0xFF).unwrap_or(0));
            harf >>= 8;
            marra = marra.saturating_add(1);
        }
    }
    !basma
}

/// One byte through the reflected CRC-32 polynomial.
fn crc_bayt(basma: u32, wahda: u8) -> u32 {
    let mut basma = basma ^ u32::from(wahda);
    let mut bit = 0u8;
    while bit < 8 {
        basma = if basma & 1 == 0 {
            basma >> 1
        } else {
            (basma >> 1) ^ 0xEDB8_8320
        };
        bit = bit.saturating_add(1);
    }
    basma
}

/// The hash of a source string, which is the same in every version.
///
/// Version 3 changed how *keys* are hashed and left this one alone: it is not a
/// lookup key, it is the fingerprint of the English text a translation was made
/// from, and the engine compares it to decide whether a translation has gone
/// stale. It is not lowercased, because case is part of the source text.
///
/// Checked against real bytes: for every entry of a shipped Arabic `.locres`
/// whose source string is recoverable from the same game's native-culture
/// `.locres`, this reproduces the stored fingerprint exactly. See
/// `tests/basmat.rs`.
#[must_use]
pub fn basmat_asl(nass: &str) -> u32 {
    basmat_crc(nass)
}

/// The namespace or key hash a given version wants, or [`None`] for a version
/// that stores no hashes.
///
/// The string is hashed as it stands. Nothing is lowercased anywhere on this
/// path — see this module's header for the observation that settles it and for
/// what the lowercasing this function used to do cost.
#[must_use]
pub fn basmat_miftah(isdar: IsdarLocres, nass: &str) -> Option<u32> {
    match isdar {
        IsdarLocres::Qadim | IsdarLocres::Mudmaj => None,
        IsdarLocres::Muhassan => Some(basmat_crc(nass)),
        IsdarLocres::MuhassanMadina => Some(basmat_madina_miftah(nass)),
    }
}

/// Version 3's namespace and key hash: `FTextKey`'s own, in full.
///
/// Zero for the empty string, which the engine answers without hashing
/// anything — a default-constructed `FTextKey` carries a zero hash and an empty
/// string, and that pair is what the unnamed namespace serializes as. Otherwise
/// [`basmat_madina_nass`] narrowed through [`tayy_basma`].
#[must_use]
pub fn basmat_madina_miftah(nass: &str) -> u32 {
    if nass.is_empty() {
        return 0;
    }
    tayy_basma(basmat_madina_nass(nass))
}

/// Unreal's `GetTypeHash(uint64)`: the low word plus twenty-three times the high
/// one, wrapping.
///
/// This is how a 64-bit hash becomes the 32-bit field a `.locres` stores, and it
/// is not interchangeable with taking the low half — see this module's header.
/// Written with `wrapping_*` because the engine's arithmetic is `uint32` and
/// wraps, and a build with overflow checks on must produce the same number as
/// one without.
#[must_use]
pub fn tayy_basma(kamil: u64) -> u32 {
    let adna = u32::try_from(kamil & 0xFFFF_FFFF).unwrap_or(0);
    let aala = u32::try_from(kamil >> 32).unwrap_or(0);
    adna.wrapping_add(aala.wrapping_mul(23))
}

/// `CityHash64` of a string's UTF-16LE code units, as the engine hashes them.
///
/// The conversion is explicit — `encode_utf16` then `to_le_bytes` — rather than
/// a reinterpretation of a `TCHAR` buffer, because the value is written into a
/// file that has to resolve on every platform the game ships to and a
/// reinterpretation would encode the build machine's endianness into the game's
/// text. No terminator is included, no byte-order mark is prepended, and the
/// string is not case-folded: the engine hashes the characters and nothing else.
#[must_use]
pub fn basmat_madina_nass(nass: &str) -> u64 {
    let mut bayt = Vec::with_capacity(nass.len().saturating_mul(2));
    for wahda in nass.encode_utf16() {
        bayt.extend_from_slice(&wahda.to_le_bytes());
    }
    basmat_madina(&bayt)
}

/// The first constant of `CityHash`.
const K0: u64 = 0xc3a5_c85c_97cb_3127;
/// The second constant of `CityHash`.
const K1: u64 = 0xb492_b66f_be98_f273;
/// The third constant of `CityHash`.
const K2: u64 = 0x9ae1_6a3b_2f90_404f;
/// The multiplier `CityHash`'s 128-to-64 fold uses.
const KMUL: u64 = 0x9ddf_ea08_eb38_2d69;

/// `CityHash64` over arbitrary bytes.
///
/// A faithful transcription of Google's `CityHash` 1.1 `CityHash64`, which is the
/// version Unreal vendors. It is implemented here rather than taken from a crate
/// for the reason this whole module exists: the value ends up in a file the
/// engine looks entries up by, so the implementation has to be one this
/// repository can read, and a dependency that silently moved to `CityHash` 1.0 or
/// to `CityHashCrc` would change every hash Taarib writes.
///
/// All loads are little-endian and bounds-checked; a load past the end reads
/// zero, which cannot happen because every caller below has already sized its
/// slice, and is still written that way so no arithmetic here can index out of
/// range.
#[must_use]
#[expect(
    clippy::many_single_char_names,
    reason = "x, y, z, v and w are CityHash64's own names for its state words, and this \
              transcription is checked against Google's source line by line"
)]
pub fn basmat_madina(bayt: &[u8]) -> u64 {
    let tul = bayt.len();
    if tul <= 32 {
        return if tul <= 16 {
            madina_0_ila_16(bayt)
        } else {
            madina_17_ila_32(bayt)
        };
    }
    if tul <= 64 {
        return madina_33_ila_64(bayt);
    }

    let tul64 = tul_u64(tul);
    let mut x = jalb64(bayt, tul.saturating_sub(40));
    let mut y =
        jalb64(bayt, tul.saturating_sub(16)).wrapping_add(jalb64(bayt, tul.saturating_sub(56)));
    let mut z = madina_16(
        jalb64(bayt, tul.saturating_sub(48)).wrapping_add(tul64),
        jalb64(bayt, tul.saturating_sub(24)),
    );
    let mut v = daeef_min_bayt(bayt, tul.saturating_sub(64), tul64, z);
    let mut w = daeef_min_bayt(bayt, tul.saturating_sub(32), y.wrapping_add(K1), x);
    x = x.wrapping_mul(K1).wrapping_add(jalb64(bayt, 0));

    // The tail was folded in above; the loop now walks whole 64-byte blocks from
    // the front, which is why the length is rounded down to a multiple of 64.
    let mut baqi = tul.saturating_sub(1) & !63usize;
    let mut mawqi = 0usize;
    while baqi != 0 {
        x = daur(
            x.wrapping_add(y)
                .wrapping_add(v.0)
                .wrapping_add(jalb64(bayt, mawqi.saturating_add(8))),
            37,
        )
        .wrapping_mul(K1);
        y = daur(
            y.wrapping_add(v.1)
                .wrapping_add(jalb64(bayt, mawqi.saturating_add(48))),
            42,
        )
        .wrapping_mul(K1);
        x ^= w.1;
        y = y
            .wrapping_add(v.0)
            .wrapping_add(jalb64(bayt, mawqi.saturating_add(40)));
        z = daur(z.wrapping_add(w.0), 33).wrapping_mul(K1);
        v = daeef_min_bayt(bayt, mawqi, v.1.wrapping_mul(K1), x.wrapping_add(w.0));
        w = daeef_min_bayt(
            bayt,
            mawqi.saturating_add(32),
            z.wrapping_add(w.1),
            y.wrapping_add(jalb64(bayt, mawqi.saturating_add(16))),
        );
        core::mem::swap(&mut z, &mut x);
        mawqi = mawqi.saturating_add(64);
        baqi = baqi.saturating_sub(64);
    }
    madina_16(
        madina_16(v.0, w.0)
            .wrapping_add(khalt(y).wrapping_mul(K1))
            .wrapping_add(z),
        madina_16(v.1, w.1).wrapping_add(x),
    )
}

/// Eight little-endian bytes at an offset, or zero past the end.
fn jalb64(bayt: &[u8], izaha: usize) -> u64 {
    bayt.get(izaha..)
        .and_then(<[u8]>::first_chunk::<8>)
        .map_or(0, |khana| u64::from_le_bytes(*khana))
}

/// Four little-endian bytes at an offset, or zero past the end.
fn jalb32(bayt: &[u8], izaha: usize) -> u32 {
    bayt.get(izaha..)
        .and_then(<[u8]>::first_chunk::<4>)
        .map_or(0, |khana| u32::from_le_bytes(*khana))
}

/// `CityHash`'s rotate.
const fn daur(qeema: u64, miqdar: u32) -> u64 {
    qeema.rotate_right(miqdar)
}

/// `CityHash`'s `ShiftMix`.
const fn khalt(qeema: u64) -> u64 {
    qeema ^ (qeema >> 47)
}

/// `CityHash`'s 128-to-64 fold with the standard multiplier.
const fn madina_16(awwal: u64, thani: u64) -> u64 {
    madina_16_bi(awwal, thani, KMUL)
}

/// `CityHash`'s 128-to-64 fold with a length-derived multiplier.
const fn madina_16_bi(awwal: u64, thani: u64, mudaaf: u64) -> u64 {
    let mut a = (awwal ^ thani).wrapping_mul(mudaaf);
    a ^= a >> 47;
    let mut b = (thani ^ a).wrapping_mul(mudaaf);
    b ^= b >> 47;
    b.wrapping_mul(mudaaf)
}

/// `CityHash` for zero to sixteen bytes.
#[expect(
    clippy::many_single_char_names,
    reason = "a, b, c, d, y and z are CityHash64's own names for these intermediates, and \
              this transcription is checked against Google's source line by line"
)]
fn madina_0_ila_16(bayt: &[u8]) -> u64 {
    let tul = bayt.len();
    let tul64 = tul_u64(tul);
    if tul >= 8 {
        let mudaaf = K2.wrapping_add(tul64.wrapping_mul(2));
        let a = jalb64(bayt, 0).wrapping_add(K2);
        let b = jalb64(bayt, tul.saturating_sub(8));
        let c = daur(b, 37).wrapping_mul(mudaaf).wrapping_add(a);
        let d = daur(a, 25).wrapping_add(b).wrapping_mul(mudaaf);
        return madina_16_bi(c, d, mudaaf);
    }
    if tul >= 4 {
        let mudaaf = K2.wrapping_add(tul64.wrapping_mul(2));
        let a = u64::from(jalb32(bayt, 0));
        let b = u64::from(jalb32(bayt, tul.saturating_sub(4)));
        return madina_16_bi(tul64.wrapping_add(a << 3), b, mudaaf);
    }
    if tul > 0 {
        let a = bayt.first().copied().unwrap_or(0);
        let b = bayt.get(tul >> 1).copied().unwrap_or(0);
        let c = bayt.get(tul.saturating_sub(1)).copied().unwrap_or(0);
        let y = u32::from(a).wrapping_add(u32::from(b) << 8);
        let z = u32::try_from(tul)
            .unwrap_or(u32::MAX)
            .wrapping_add(u32::from(c) << 2);
        let makhlut = u64::from(y).wrapping_mul(K2) ^ u64::from(z).wrapping_mul(K0);
        return khalt(makhlut).wrapping_mul(K2);
    }
    K2
}

/// `CityHash` for seventeen to thirty-two bytes.
fn madina_17_ila_32(bayt: &[u8]) -> u64 {
    let tul = bayt.len();
    let mudaaf = K2.wrapping_add(tul_u64(tul).wrapping_mul(2));
    let a = jalb64(bayt, 0).wrapping_mul(K1);
    let b = jalb64(bayt, 8);
    let c = jalb64(bayt, tul.saturating_sub(8)).wrapping_mul(mudaaf);
    let d = jalb64(bayt, tul.saturating_sub(16)).wrapping_mul(K2);
    madina_16_bi(
        daur(a.wrapping_add(b), 43)
            .wrapping_add(daur(c, 30))
            .wrapping_add(d),
        a.wrapping_add(daur(b.wrapping_add(K2), 18)).wrapping_add(c),
        mudaaf,
    )
}

/// `CityHash` for thirty-three to sixty-four bytes.
#[expect(
    clippy::many_single_char_names,
    reason = "a through h, then u, v, w, x, y and z, are CityHash64's own names for these \
              intermediates, and this transcription is checked against Google's source line \
              by line"
)]
fn madina_33_ila_64(bayt: &[u8]) -> u64 {
    let tul = bayt.len();
    let mudaaf = K2.wrapping_add(tul_u64(tul).wrapping_mul(2));
    let a = jalb64(bayt, 0).wrapping_mul(K2);
    let b = jalb64(bayt, 8);
    let c = jalb64(bayt, tul.saturating_sub(24));
    let d = jalb64(bayt, tul.saturating_sub(32));
    let e = jalb64(bayt, 16).wrapping_mul(K2);
    let f = jalb64(bayt, 24).wrapping_mul(9);
    let g = jalb64(bayt, tul.saturating_sub(8));
    let h = jalb64(bayt, tul.saturating_sub(16)).wrapping_mul(mudaaf);
    let u = daur(a.wrapping_add(g), 43).wrapping_add(daur(b, 30).wrapping_add(c).wrapping_mul(9));
    let v = (a.wrapping_add(g) ^ d).wrapping_add(f).wrapping_add(1);
    let w = u
        .wrapping_add(v)
        .wrapping_mul(mudaaf)
        .swap_bytes()
        .wrapping_add(h);
    let x = daur(e.wrapping_add(f), 42).wrapping_add(c);
    let y = v
        .wrapping_add(w)
        .wrapping_mul(mudaaf)
        .swap_bytes()
        .wrapping_add(g)
        .wrapping_mul(mudaaf);
    let z = e.wrapping_add(f).wrapping_add(c);
    let a2 = x
        .wrapping_add(z)
        .wrapping_mul(mudaaf)
        .wrapping_add(y)
        .swap_bytes()
        .wrapping_add(b);
    let b2 = khalt(
        z.wrapping_add(a2)
            .wrapping_mul(mudaaf)
            .wrapping_add(d)
            .wrapping_add(h),
    )
    .wrapping_mul(mudaaf);
    b2.wrapping_add(x)
}

/// `CityHash`'s `WeakHashLen32WithSeeds`, over six words.
#[expect(
    clippy::many_single_char_names,
    reason = "the parameter names are `WeakHashLen32WithSeeds`'s own, and renaming them would \
              hide that this is a transcription of it"
)]
const fn daeef(w: u64, x: u64, y: u64, z: u64, a: u64, b: u64) -> (u64, u64) {
    let mut a = a.wrapping_add(w);
    let mut b = daur(b.wrapping_add(a).wrapping_add(z), 21);
    let c = a;
    a = a.wrapping_add(x);
    a = a.wrapping_add(y);
    b = b.wrapping_add(daur(a, 44));
    (a.wrapping_add(z), b.wrapping_add(c))
}

/// `CityHash`'s `WeakHashLen32WithSeeds`, over thirty-two bytes at an offset.
fn daeef_min_bayt(bayt: &[u8], izaha: usize, a: u64, b: u64) -> (u64, u64) {
    daeef(
        jalb64(bayt, izaha),
        jalb64(bayt, izaha.saturating_add(8)),
        jalb64(bayt, izaha.saturating_add(16)),
        jalb64(bayt, izaha.saturating_add(24)),
        a,
        b,
    )
}

// ---------------------------------------------------------------------------
// The records
// ---------------------------------------------------------------------------

/// Where an entry's translation lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarjaTarjama {
    /// Legacy: the translation is stored inside the entry.
    Mudmaj(NassMukhazzan),
    /// Versions 1, 2 and 3: an index into the string array.
    Fahras(u32),
}

/// One string in the array, with the reference count stored beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NassHawd {
    nass: NassMukhazzan,
    adad_marja: Option<i32>,
    marja_fili: u32,
}

impl NassHawd {
    /// The string.
    #[must_use]
    pub const fn nass(&self) -> &NassMukhazzan {
        &self.nass
    }

    /// The reference count the file stored, or [`None`] in version 1, which
    /// stores none.
    ///
    /// Carried rather than trusted: nothing in this module reads it to decide
    /// anything, because it is a number in a file. What editing consults is the
    /// count taken from the tables in memory.
    #[must_use]
    pub const fn adad_marja(&self) -> Option<i32> {
        self.adad_marja
    }

    /// How many entries in this resource actually point at this string, counted
    /// from the tables rather than read from the file.
    #[must_use]
    pub const fn marja_fili(&self) -> u32 {
        self.marja_fili
    }
}

/// One entry: a key, the fingerprint of its source, and its translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalLocres {
    miftah: NassMukhazzan,
    basma: Option<u32>,
    basmat_asl: u32,
    tarjama: MarjaTarjama,
}

impl MadkhalLocres {
    /// The key.
    #[must_use]
    pub const fn miftah(&self) -> &NassMukhazzan {
        &self.miftah
    }

    /// The key's hash as the file stored it, or [`None`] in legacy and version
    /// 1.
    #[must_use]
    pub const fn basma(&self) -> Option<u32> {
        self.basma
    }

    /// The CRC of the source string this was translated from.
    #[must_use]
    pub const fn basmat_asl(&self) -> u32 {
        self.basmat_asl
    }

    /// Where the translation lives.
    #[must_use]
    pub const fn tarjama(&self) -> &MarjaTarjama {
        &self.tarjama
    }
}

/// One namespace and the entries under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FadaaLocres {
    ism: NassMukhazzan,
    basma: Option<u32>,
    madakhil: Vec<MadkhalLocres>,
}

impl FadaaLocres {
    /// The namespace.
    #[must_use]
    pub const fn ism(&self) -> &NassMukhazzan {
        &self.ism
    }

    /// The namespace's hash as the file stored it, or [`None`] in legacy and
    /// version 1.
    #[must_use]
    pub const fn basma(&self) -> Option<u32> {
        self.basma
    }

    /// The entries, in the order the file lists them.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalLocres] {
        &self.madakhil
    }
}

/// A whole `.locres`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawridLocres {
    isdar: IsdarLocres,
    adad_madakhil: Option<u32>,
    fadaat: Vec<FadaaLocres>,
    hawd: Vec<NassHawd>,
    fajwa: Vec<u8>,
    dhayl: Vec<u8>,
}

impl Mawrid for MawridLocres {
    const ISM: &'static str = ISM;

    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataUnreal> {
        let mut qari = Qari::jadeed(ISM, bayt);
        let isdar = iqra_isdar(&mut qari)?;

        // The string array's offset is read before the namespace table because
        // the header holds it there, and is judged against the table's real end
        // afterwards — a forward reference cannot be checked until the thing it
        // points past has been read.
        let izahat_hawd = if isdar.yahwi_hawd() {
            let izaha = qari.iqra_i64("izahat_hawd")?;
            Some(u64::try_from(izaha).map_err(|_| KhataUnreal::MawridTalif {
                ism: ISM,
                haql: "izahat_hawd",
                qeema: izaha.unsigned_abs(),
                hadd: tul_u64(bayt.len()),
            })?)
        } else {
            None
        };

        let adad_madakhil = if isdar.yahwi_basmat() {
            Some(qari.iqra_u32("adad_madakhil")?)
        } else {
            None
        };

        let muallan = u64::from(qari.iqra_u32("adad_fadaat")?);
        let aqall = if isdar.yahwi_basmat() {
            AQALL_FADAA
        } else {
            AQALL_FADAA_BILA_BASMA
        };
        let adad = tahaqquq_adad(ISM, "adad_fadaat", muallan, AQSA_FADAAT, aqall, qari.baqi())?;

        let mut fadaat = Vec::with_capacity(adad);
        let mut majmu = 0u64;
        for _ in 0..adad {
            fadaat.push(iqra_fadaa(&mut qari, isdar, &mut majmu)?);
        }

        let nihayat_jadwal = tul_u64(qari.mawqi());
        let (hawd, fajwa) = match izahat_hawd {
            None => (Vec::new(), Vec::new()),
            Some(izaha) => {
                if izaha < nihayat_jadwal {
                    return Err(KhataUnreal::MawridTalif {
                        ism: ISM,
                        haql: "izahat_hawd, which may not precede the namespace table's end",
                        qeema: izaha,
                        hadd: nihayat_jadwal,
                    });
                }
                // Reading the gap rather than seeking over it is what preserves
                // it, and it also proves the offset is inside the file without a
                // second bounds check written somewhere else.
                let mada = izaha.saturating_sub(nihayat_jadwal);
                let fajwa = qari
                    .iqra_bayt("the gap before the string array", mada)?
                    .to_vec();
                (iqra_hawd(&mut qari, isdar)?, fajwa)
            },
        };

        let dhayl = qari.baqiya().to_vec();
        let mut mawrid = Self {
            isdar,
            adad_madakhil,
            fadaat,
            hawd,
            fajwa,
            dhayl,
        };
        mawrid.tahaqquq_faharis()?;
        Ok(mawrid)
    }

    fn ila_bayt(&self) -> Result<Vec<u8>, KhataUnreal> {
        let mut katib = Katib::bi_siaa(self.siaa_mutawaqqaa());
        let mut izahat_haql = 0usize;
        if let Some(raqm) = self.isdar.raqm() {
            katib.uktub_bayt(&SIHR);
            katib.uktub_u8(raqm);
            // Written as zero and filled in below, once the namespace table has
            // been emitted and the array's real offset exists.
            izahat_haql = katib.mawqi();
            katib.uktub_i64(0);
        }
        if self.isdar.yahwi_basmat() {
            katib.uktub_u32(self.adad_madakhil.unwrap_or(0));
        }
        katib.uktub_u32(adad_khana("adad_fadaat", self.fadaat.len(), AQSA_FADAAT)?);

        for fadaa in &self.fadaat {
            if self.isdar.yahwi_basmat() {
                katib.uktub_u32(fadaa.basma.unwrap_or(0));
            }
            katib.uktub_nass(ISM, "a namespace", &fadaa.ism)?;
            katib.uktub_u32(adad_khana(
                "a namespace's entry count",
                fadaa.madakhil.len(),
                AQSA_MADAKHIL_FADAA,
            )?);
            for madkhal in &fadaa.madakhil {
                if self.isdar.yahwi_basmat() {
                    katib.uktub_u32(madkhal.basma.unwrap_or(0));
                }
                katib.uktub_nass(ISM, "an entry's key", &madkhal.miftah)?;
                katib.uktub_u32(madkhal.basmat_asl);
                match &madkhal.tarjama {
                    MarjaTarjama::Mudmaj(nass) => {
                        katib.uktub_nass(ISM, "an entry's translation", nass)?;
                    },
                    MarjaTarjama::Fahras(fahras) => {
                        katib.uktub_i32(i32::try_from(*fahras).map_err(|_| {
                            KhataUnreal::HajmMufrit {
                                haql: "an entry's string index",
                                qeema: u64::from(*fahras),
                                saqf: AQSA_NUSUS,
                            }
                        })?);
                    },
                }
            }
        }

        if self.isdar.yahwi_hawd() {
            katib.uktub_bayt(&self.fajwa);
            let izaha = i64::try_from(katib.mawqi()).map_err(|_| KhataUnreal::HajmMufrit {
                haql: "izahat_hawd",
                qeema: tul_u64(katib.mawqi()),
                saqf: u64::from(u32::MAX),
            })?;
            katib.uktub_i64_fi(ISM, "izahat_hawd", izahat_haql, izaha)?;
            let adad = adad_khana("the string array's count", self.hawd.len(), AQSA_NUSUS)?;
            katib.uktub_i32(i32::try_from(adad).map_err(|_| KhataUnreal::HajmMufrit {
                haql: "the string array's count",
                qeema: u64::from(adad),
                saqf: AQSA_NUSUS,
            })?);
            for nass in &self.hawd {
                katib.uktub_nass(ISM, "a string in the array", &nass.nass)?;
                if self.isdar.yahwi_basmat() {
                    katib.uktub_i32(nass.adad_marja.unwrap_or(0));
                }
            }
        }
        katib.uktub_bayt(&self.dhayl);
        Ok(katib.ila_vec())
    }
}

impl MawridLocres {
    /// An empty resource of a given version, for a file Taarib authors.
    #[must_use]
    pub const fn jadeed(isdar: IsdarLocres) -> Self {
        Self {
            isdar,
            adad_madakhil: if isdar.yahwi_basmat() { Some(0) } else { None },
            fadaat: Vec::new(),
            hawd: Vec::new(),
            fajwa: Vec::new(),
            dhayl: Vec::new(),
        }
    }

    /// Reads a `.locres` from a path.
    ///
    /// **The convenience function.** Everything else in this type works over
    /// `&[u8]`, because a `.locres` is found inside a `.pak` at least as often
    /// as it is found loose.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] naming the path, [`KhataUnreal::HajmMufrit`]
    /// when the file is above [`super::AQSA_MALAF`], and otherwise whatever
    /// [`Mawrid::min_bayt`] refuses.
    pub fn min_malaf(masar: &Path) -> Result<Self, KhataUnreal> {
        Self::min_bayt(&iqra_malaf(masar)?)
    }

    /// Writes this resource to a path.
    ///
    /// # Errors
    ///
    /// Whatever [`Mawrid::ila_bayt`] refuses, or [`KhataUnreal::KhataMalaf`]
    /// naming the path.
    pub fn ila_malaf(&self, masar: &Path) -> Result<(), KhataUnreal> {
        uktub_malaf(masar, &self.ila_bayt()?)
    }

    /// Which version this file is, and therefore which shape it writes back as.
    #[must_use]
    pub const fn isdar(&self) -> IsdarLocres {
        self.isdar
    }

    /// The namespaces, in file order.
    #[must_use]
    pub fn fadaat(&self) -> &[FadaaLocres] {
        &self.fadaat
    }

    /// The string array, in file order. Empty in legacy files, which have none.
    #[must_use]
    pub fn hawd(&self) -> &[NassHawd] {
        &self.hawd
    }

    /// The bytes between the namespace table and the string array, preserved
    /// verbatim. Empty in every file Unreal writes.
    #[must_use]
    pub fn fajwa(&self) -> &[u8] {
        &self.fajwa
    }

    /// The bytes after the last string, preserved verbatim.
    #[must_use]
    pub fn dhayl(&self) -> &[u8] {
        &self.dhayl
    }

    /// The header's entry count as the file stated it, or [`None`] in legacy and
    /// version 1, which have no such field.
    ///
    /// Carried rather than believed: [`MawridLocres::adad_madakhil`] counts the
    /// tables instead.
    #[must_use]
    pub const fn adad_madakhil_muallan(&self) -> Option<u32> {
        self.adad_madakhil
    }

    /// How many entries this resource actually holds.
    #[must_use]
    pub fn adad_madakhil(&self) -> u64 {
        self.fadaat
            .iter()
            .map(|fadaa| tul_u64(fadaa.madakhil.len()))
            .sum()
    }

    /// Every entry, with the namespace it sits under.
    pub fn madakhil(&self) -> impl Iterator<Item = (&FadaaLocres, &MadkhalLocres)> + '_ {
        self.fadaat
            .iter()
            .flat_map(|fadaa| fadaa.madakhil.iter().map(move |madkhal| (fadaa, madkhal)))
    }

    /// The translation an entry carries, wherever it is stored.
    #[must_use]
    // The two arms borrow from different places — an inline string from the
    // entry, a pooled one from the container — so the answer lives only as
    // long as both do.
    pub fn nass_madkhal<'a>(&'a self, madkhal: &'a MadkhalLocres) -> Option<&'a str> {
        match &madkhal.tarjama {
            MarjaTarjama::Mudmaj(nass) => Some(nass.nass()),
            MarjaTarjama::Fahras(fahras) => {
                let khana = usize::try_from(*fahras).ok()?;
                Some(self.hawd.get(khana)?.nass.nass())
            },
        }
    }

    /// One entry, by namespace and key.
    #[must_use]
    pub fn jid(&self, fadaa: &str, miftah: &str) -> Option<&MadkhalLocres> {
        let (mawqi_fadaa, mawqi_madkhal) = self.mawqi_madkhal(fadaa, miftah)?;
        self.fadaat.get(mawqi_fadaa)?.madakhil.get(mawqi_madkhal)
    }

    /// The translation for a namespace and key.
    #[must_use]
    pub fn tarjama(&self, fadaa: &str, miftah: &str) -> Option<&str> {
        self.nass_madkhal(self.jid(fadaa, miftah)?)
    }

    /// Replaces one entry's translation, leaving every other byte alone.
    ///
    /// Returns whether the entry was there. Nothing is created: a key a game
    /// does not have is not a key this call invents, because an entry Unreal
    /// never compiled is an entry nothing looks up.
    ///
    /// A string in the array may be referenced by several entries — that is what
    /// the array is *for* — so an edit to a shared string appends a new one and
    /// repoints this entry at it, rather than rewriting text five hundred other
    /// entries are also using. An unshared string is edited in place, so a
    /// wholesale retranslation does not double the array.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::HajmMufrit`] when appending would take the string array
    /// past [`AQSA_NUSUS`], and [`KhataUnreal::MawridTalif`] when an entry's
    /// stored index does not fit a `usize` on this target.
    pub fn istabdil(&mut self, fadaa: &str, miftah: &str, nass: &str) -> Result<bool, KhataUnreal> {
        let Some((mawqi_fadaa, mawqi_madkhal)) = self.mawqi_madkhal(fadaa, miftah) else {
            return Ok(false);
        };
        let marja = self
            .fadaat
            .get(mawqi_fadaa)
            .and_then(|fadaa| fadaa.madakhil.get(mawqi_madkhal))
            .map(|madkhal| madkhal.tarjama.clone());
        let Some(marja) = marja else { return Ok(false) };

        let jadeed = match marja {
            MarjaTarjama::Mudmaj(_) => MarjaTarjama::Mudmaj(NassMukhazzan::jadeed(nass)),
            MarjaTarjama::Fahras(fahras) => {
                let khana = usize::try_from(fahras).map_err(|_| KhataUnreal::MawridTalif {
                    ism: ISM,
                    haql: "an entry's string index",
                    qeema: u64::from(fahras),
                    hadd: tul_u64(self.hawd.len()),
                })?;
                let mushtarak = self.hawd.get(khana).is_some_and(|hawd| hawd.marja_fili > 1);
                if mushtarak {
                    let makan = self.adif_nass_hawd(nass)?;
                    if let Some(qadim) = self.hawd.get_mut(khana) {
                        qadim.marja_fili = qadim.marja_fili.saturating_sub(1);
                    }
                    MarjaTarjama::Fahras(makan)
                } else {
                    if let Some(qadim) = self.hawd.get_mut(khana) {
                        qadim.nass.ghayyir(nass);
                    }
                    MarjaTarjama::Fahras(fahras)
                }
            },
        };

        if let Some(madkhal) = self
            .fadaat
            .get_mut(mawqi_fadaa)
            .and_then(|fadaa| fadaa.madakhil.get_mut(mawqi_madkhal))
        {
            madkhal.tarjama = jadeed;
        }
        Ok(true)
    }

    /// Adds an entry, or replaces the translation of one already there.
    ///
    /// This is the call that computes hashes, and the only one: the namespace
    /// and key hashes come from [`basmat_miftah`] for this file's own version,
    /// and the source fingerprint from [`basmat_asl`]. A file that has had this
    /// called on it no longer round-trips byte-identically, which is correct —
    /// it is no longer the file that was read.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::HajmMufrit`] when the resource would exceed
    /// [`AQSA_FADAAT`], [`AQSA_MADAKHIL_FADAA`] or [`AQSA_NUSUS`], and whatever
    /// [`MawridLocres::istabdil`] refuses when the entry already exists.
    pub fn adif(
        &mut self,
        fadaa: &str,
        miftah: &str,
        asl: &str,
        tarjama: &str,
    ) -> Result<(), KhataUnreal> {
        if let Some((mawqi_fadaa, mawqi_madkhal)) = self.mawqi_madkhal(fadaa, miftah) {
            self.istabdil(fadaa, miftah, tarjama)?;
            if let Some(madkhal) = self
                .fadaat
                .get_mut(mawqi_fadaa)
                .and_then(|fadaa| fadaa.madakhil.get_mut(mawqi_madkhal))
            {
                madkhal.basmat_asl = basmat_asl(asl);
            }
            return Ok(());
        }

        let marja = if self.isdar.yahwi_hawd() {
            MarjaTarjama::Fahras(self.adif_nass_hawd(tarjama)?)
        } else {
            MarjaTarjama::Mudmaj(NassMukhazzan::jadeed(tarjama))
        };
        let madkhal = MadkhalLocres {
            miftah: NassMukhazzan::jadeed(miftah),
            basma: basmat_miftah(self.isdar, miftah),
            basmat_asl: basmat_asl(asl),
            tarjama: marja,
        };

        let mawqi_fadaa = if let Some(mawqi) = self.mawqi_fadaa(fadaa) {
            mawqi
        } else {
            if tul_u64(self.fadaat.len()) >= AQSA_FADAAT {
                return Err(KhataUnreal::HajmMufrit {
                    haql: "adad_fadaat",
                    qeema: tul_u64(self.fadaat.len()).saturating_add(1),
                    saqf: AQSA_FADAAT,
                });
            }
            self.fadaat.push(FadaaLocres {
                ism: NassMukhazzan::jadeed(fadaa),
                basma: basmat_miftah(self.isdar, fadaa),
                madakhil: Vec::new(),
            });
            self.fadaat.len().saturating_sub(1)
        };

        let Some(hadaf) = self.fadaat.get_mut(mawqi_fadaa) else {
            return Ok(());
        };
        if tul_u64(hadaf.madakhil.len()) >= AQSA_MADAKHIL_FADAA {
            return Err(KhataUnreal::HajmMufrit {
                haql: "a namespace's entry count",
                qeema: tul_u64(hadaf.madakhil.len()).saturating_add(1),
                saqf: AQSA_MADAKHIL_FADAA,
            });
        }
        hadaf.madakhil.push(madkhal);
        if let Some(adad) = self.adad_madakhil.as_mut() {
            *adad = adad.saturating_add(1);
        }
        Ok(())
    }

    /// Recomputes the two counts the file caches: the header's entry count and
    /// every string's reference count.
    ///
    /// Not called automatically on write, because writing must reproduce the
    /// file that was read even when its cached counts disagree with its tables —
    /// a disagreement Taarib did not introduce is a disagreement Taarib does not
    /// silently repair. Call it after building a resource from scratch.
    pub fn ahsi_ihsaat(&mut self) {
        if self.isdar.yahwi_basmat() {
            let majmu = self.adad_madakhil();
            self.adad_madakhil = Some(u32::try_from(majmu).unwrap_or(u32::MAX));
        }
        let yahwi = self.isdar.yahwi_basmat();
        for nass in &mut self.hawd {
            if yahwi {
                nass.adad_marja = Some(i32::try_from(nass.marja_fili).unwrap_or(i32::MAX));
            }
        }
    }

    /// Recomputes every namespace and key hash for this file's version.
    ///
    /// **This changes bytes**, and is therefore never called on the read path.
    /// It exists for a resource Taarib built itself, and for the one diagnostic
    /// worth having: comparing the result against what the file stored says
    /// whether this build's hash functions agree with the compiler that produced
    /// the game — which is the difference between a patch that resolves and a
    /// patch that loads and shows nothing.
    pub fn ahsi_basmat(&mut self) {
        let isdar = self.isdar;
        for fadaa in &mut self.fadaat {
            fadaa.basma = basmat_miftah(isdar, fadaa.ism.nass());
            for madkhal in &mut fadaa.madakhil {
                madkhal.basma = basmat_miftah(isdar, madkhal.miftah.nass());
            }
        }
    }

    /// Whether every stored hash matches what this build computes for it.
    ///
    /// [`None`] for a version that stores no hashes. `Some(false)` means a patch
    /// written from this resource would load and resolve nothing, and is worth
    /// reporting before the patch is built rather than after a player installs
    /// it.
    #[must_use]
    pub fn basmat_mutabaqa(&self) -> Option<bool> {
        if !self.isdar.yahwi_basmat() {
            return None;
        }
        let mutabaq = self.fadaat.iter().all(|fadaa| {
            fadaa.basma == basmat_miftah(self.isdar, fadaa.ism.nass())
                && fadaa.madakhil.iter().all(|madkhal| {
                    madkhal.basma == basmat_miftah(self.isdar, madkhal.miftah.nass())
                })
        });
        Some(mutabaq)
    }

    /// Appends a string to the array and returns its index.
    fn adif_nass_hawd(&mut self, nass: &str) -> Result<u32, KhataUnreal> {
        let makan = tul_u64(self.hawd.len());
        if makan >= AQSA_NUSUS {
            return Err(KhataUnreal::HajmMufrit {
                haql: "the string array's count",
                qeema: makan.saturating_add(1),
                saqf: AQSA_NUSUS,
            });
        }
        self.hawd.push(NassHawd {
            nass: NassMukhazzan::jadeed(nass),
            adad_marja: if self.isdar.yahwi_basmat() {
                Some(1)
            } else {
                None
            },
            marja_fili: 1,
        });
        u32::try_from(makan).map_err(|_| KhataUnreal::HajmMufrit {
            haql: "the string array's count",
            qeema: makan,
            saqf: AQSA_NUSUS,
        })
    }

    /// Where a namespace sits, by name.
    fn mawqi_fadaa(&self, fadaa: &str) -> Option<usize> {
        self.fadaat
            .iter()
            .position(|mawjud| mawjud.ism.nass() == fadaa)
    }

    /// Where an entry sits, by namespace and key.
    fn mawqi_madkhal(&self, fadaa: &str, miftah: &str) -> Option<(usize, usize)> {
        let mawqi_fadaa = self.mawqi_fadaa(fadaa)?;
        let mawqi_madkhal = self
            .fadaat
            .get(mawqi_fadaa)?
            .madakhil
            .iter()
            .position(|mawjud| mawjud.miftah.nass() == miftah)?;
        Some((mawqi_fadaa, mawqi_madkhal))
    }

    /// A rough size for the output buffer, so writing a large resource is not a
    /// sequence of reallocations.
    fn siaa_mutawaqqaa(&self) -> usize {
        let madakhil = usize::try_from(self.adad_madakhil()).unwrap_or(0);
        madakhil
            .saturating_mul(48)
            .saturating_add(self.hawd.len().saturating_mul(48))
            .max(64)
    }

    /// Proves every entry's string index is inside the array, and counts how
    /// many entries point at each string.
    ///
    /// Done once, here, rather than at every lookup: an index proven in range at
    /// load is an index a caller can resolve without a second opinion, and an
    /// index out of range is a corrupt file rather than a missing translation.
    fn tahaqquq_faharis(&mut self) -> Result<(), KhataUnreal> {
        let hadd = tul_u64(self.hawd.len());
        let mut adaad = vec![0u32; self.hawd.len()];
        for fadaa in &self.fadaat {
            for madkhal in &fadaa.madakhil {
                let MarjaTarjama::Fahras(fahras) = &madkhal.tarjama else {
                    continue;
                };
                let fahras = *fahras;
                let khana = usize::try_from(fahras)
                    .ok()
                    .filter(|khana| *khana < self.hawd.len())
                    .ok_or_else(|| KhataUnreal::MawridTalif {
                        ism: ISM,
                        haql: "an entry's string index",
                        qeema: u64::from(fahras),
                        hadd,
                    })?;
                if let Some(adad) = adaad.get_mut(khana) {
                    *adad = adad.saturating_add(1);
                }
            }
        }
        for (nass, adad) in self.hawd.iter_mut().zip(adaad) {
            nass.marja_fili = adad;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reading, piece by piece
// ---------------------------------------------------------------------------

/// Decides the version, consuming the header bytes that carry it.
///
/// The absence of the magic is the legacy signal and the only one available: a
/// legacy file begins with a namespace count, and no count can be told apart
/// from data by inspection. So the check is exact — sixteen bytes equal to
/// [`SIHR`] or not — and a file that is neither a legacy `.locres` nor any other
/// format fails later, at the first field that does not fit, rather than here.
fn iqra_isdar(qari: &mut Qari<'_>) -> Result<IsdarLocres, KhataUnreal> {
    if qari.kull().get(..SIHR.len()) != Some(SIHR.as_slice()) {
        return Ok(IsdarLocres::Qadim);
    }
    let _sihr: [u8; 16] = qari.iqra_masfufa("the magic")?;
    let raqm = qari.iqra_u8("the version byte")?;
    if u32::from(raqm) > AQSA_ISDAR {
        return Err(KhataUnreal::IsdarGhayrMadum {
            ism: ISM,
            wujid: u32::from(raqm),
            aqsa: AQSA_ISDAR,
        });
    }
    IsdarLocres::min_raqm(raqm).ok_or_else(|| KhataUnreal::MawridTalif {
        ism: ISM,
        haql: "the version byte, which is 1, 2 or 3 whenever the magic is present",
        qeema: u64::from(raqm),
        hadd: u64::from(AQSA_ISDAR).saturating_add(1),
    })
}

/// One namespace and every entry under it.
fn iqra_fadaa(
    qari: &mut Qari<'_>,
    isdar: IsdarLocres,
    majmu: &mut u64,
) -> Result<FadaaLocres, KhataUnreal> {
    let basma = if isdar.yahwi_basmat() {
        Some(qari.iqra_u32("a namespace hash")?)
    } else {
        None
    };
    let ism = qari.iqra_nass("a namespace")?;
    let muallan = u64::from(qari.iqra_u32("a namespace's entry count")?);
    let aqall = if isdar.yahwi_basmat() {
        AQALL_MADKHAL
    } else {
        AQALL_MADKHAL_BILA_BASMA
    };
    let adad = tahaqquq_adad(
        ISM,
        "a namespace's entry count",
        muallan,
        AQSA_MADAKHIL_FADAA,
        aqall,
        qari.baqi(),
    )?;

    // The per-namespace ceiling alone would let a thousand namespaces of four
    // million entries through. The running total is what stops that.
    *majmu = majmu.saturating_add(muallan);
    if *majmu > AQSA_MADAKHIL {
        return Err(KhataUnreal::HajmMufrit {
            haql: "the entries over every namespace",
            qeema: *majmu,
            saqf: AQSA_MADAKHIL,
        });
    }

    let mut madakhil = Vec::with_capacity(adad);
    for _ in 0..adad {
        madakhil.push(iqra_madkhal(qari, isdar)?);
    }
    Ok(FadaaLocres {
        ism,
        basma,
        madakhil,
    })
}

/// One entry.
fn iqra_madkhal(qari: &mut Qari<'_>, isdar: IsdarLocres) -> Result<MadkhalLocres, KhataUnreal> {
    let basma = if isdar.yahwi_basmat() {
        Some(qari.iqra_u32("an entry's key hash")?)
    } else {
        None
    };
    let miftah = qari.iqra_nass("an entry's key")?;
    let basmat_asl = qari.iqra_u32("an entry's source hash")?;
    let tarjama = if isdar.yahwi_hawd() {
        let muallan = qari.iqra_i32("an entry's string index")?;
        MarjaTarjama::Fahras(
            u32::try_from(muallan).map_err(|_| KhataUnreal::MawridTalif {
                ism: ISM,
                haql: "an entry's string index",
                qeema: u64::from(muallan.unsigned_abs()),
                hadd: AQSA_NUSUS,
            })?,
        )
    } else {
        MarjaTarjama::Mudmaj(qari.iqra_nass("an entry's translation")?)
    };
    Ok(MadkhalLocres {
        miftah,
        basma,
        basmat_asl,
        tarjama,
    })
}

/// The string array, with its reference counts where the version has them.
fn iqra_hawd(qari: &mut Qari<'_>, isdar: IsdarLocres) -> Result<Vec<NassHawd>, KhataUnreal> {
    let muallan = qari.iqra_i32("the string array's count")?;
    let adad_muallan = adad_musir(ISM, "the string array's count", muallan)?;
    let aqall = if isdar.yahwi_basmat() {
        AQALL_NASS_HAWD
    } else {
        AQALL_NASS_HAWD_BILA_MARJA
    };
    let adad = tahaqquq_adad(
        ISM,
        "the string array's count",
        adad_muallan,
        AQSA_NUSUS,
        aqall,
        qari.baqi(),
    )?;
    let mut hawd = Vec::with_capacity(adad);
    for _ in 0..adad {
        let nass = qari.iqra_nass("a string in the array")?;
        let adad_marja = if isdar.yahwi_basmat() {
            Some(qari.iqra_i32("a string's reference count")?)
        } else {
            None
        };
        hawd.push(NassHawd {
            nass,
            adad_marja,
            marja_fili: 0,
        });
    }
    Ok(hawd)
}

/// A count as the `u32` the format writes, refused when it does not fit.
fn adad_khana(haql: &'static str, adad: usize, saqf: u64) -> Result<u32, KhataUnreal> {
    u32::try_from(adad).map_err(|_| KhataUnreal::HajmMufrit {
        haql,
        qeema: tul_u64(adad),
        saqf,
    })
}
