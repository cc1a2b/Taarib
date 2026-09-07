//! The `.locres` hashes, pinned to bytes read out of shipped games.
//!
//! A `.locres` is looked up by hash at run time and the strings are only
//! compared on a hash match, so a file whose hashes disagree with the engine's
//! loads without an error, reports the right number of entries, and resolves
//! nothing. There is no way to notice that from inside this repository except by
//! computing a hash and comparing it against one a real game's own localization
//! commandlet wrote — so that is what these tests do.
//!
//! # Where the numbers came from
//!
//! Every expected value below was read out of a shipped container with
//! `taarib_muhawwil_unreal::mawarid::pak`, on 2026-09-05, from a Steam library
//! at `F:\SteamLibrary`:
//!
//! | game | engine | container | resource |
//! | --- | --- | --- | --- |
//! | Tangles | UE 5.3 | pak v11 | `Engine/Content/Localization/Engine/en/Engine.locres`, `.locres` v3 |
//! | Little Nightmares Enhanced Edition | UE 4.27 | pak v11 | `Atlas/Content/Localization/Game/{ar,en,ja}/Game.locres`, `.locres` v3 |
//! | MECCHA CHAMELEON | UE 5.5 | pak v11 | `Chameleon/Content/Localization/**`, `.locres` v3 |
//! | Little Nightmares | UE 4.13 | pak v3 | `Atlas/Content/Localization/Game/*/Game.locres`, legacy `.locres` |
//!
//! Across those four titles the candidate implemented here reproduced **114,918
//! of 114,918** stored namespace and key hashes, and the source fingerprint
//! reproduced 4,000 of 4,000 where the native-culture string was recoverable.
//! The pairs pinned below are the short ones, chosen so that a reader can find
//! them in a hex editor: the file stores the hash immediately in front of the
//! `FString` it belongs to.
//!
//! Nothing here reads a game. The games established the numbers; the numbers
//! are the test, so it runs on a build machine that has never seen a game.
//!
//! # The three things a future change must not break
//!
//! * The string is hashed **as it stands**. `"Ok"` and `"OK"` are two keys with
//!   two different hashes in one real file, which is the observation that rules
//!   out case folding. An earlier implementation lowercased, which made every
//!   hash it computed the hash of a key the engine never looks up.
//! * The 64-bit `CityHash64` is **folded** through `GetTypeHash(uint64)`, not
//!   truncated to its low word.
//! * The empty string hashes to **zero** in both hashed versions, which matters
//!   because the unnamed namespace is the commonest namespace a game ships.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; refusing to panic here would mean a test \
              that cannot fail"
)]

use taarib_muhawwil_unreal::mawarid::locres::{
    FadaaLocres, IsdarLocres, MawridLocres, basmat_asl, basmat_crc, basmat_madina,
    basmat_madina_miftah, basmat_madina_nass, basmat_miftah, tayy_basma,
};
use taarib_muhawwil_unreal::mawarid::{Mawrid, TarmizNass};

/// Keys and the hash a shipped version 3 `.locres` stores in front of each.
///
/// From `Engine/Content/Localization/Engine/en/Engine.locres` inside Tangles'
/// `Tangles-Windows.pak`, except `"DEMO"`-style entries, which are from Little
/// Nightmares Enhanced Edition. `"Ok"` and `"OK"` appear in the same file with
/// different values, which is the pair that settles the case question.
const MIFATIH_ISDAR_THALITH: [(&str, u32); 12] = [
    ("Ok", 0xbabb_b05c),
    ("OK", 0x0429_d966),
    ("No", 0xb4ea_288f),
    ("In", 0x14dd_b623),
    ("Out", 0xaf81_951b),
    ("Any", 0x660c_83ba),
    ("One", 0xf3a1_795f),
    ("Two", 0x8715_3054),
    ("Ten", 0xcc94_8950),
    ("Yes", 0xc029_d3fe),
    ("Back", 0x25ea_2e6c),
    ("Close", 0xd71e_cdc4),
];

/// Namespaces and their stored hashes, from the same file. A namespace is
/// hashed by the same function as a key; these are here because a reader who
/// suspects the two diverge should be able to see that they do not.
const FADAAT_ISDAR_THALITH: [(&str, u32); 6] = [
    ("Actor", 0x11b7_e909),
    ("Adjust", 0x657e_114c),
    ("Anon", 0x059c_9255),
    ("ARKit", 0xffb5_5dcb),
    ("Audio", 0x835f_8c42),
    ("CCR", 0x82b1_9e81),
];

#[test]
fn version_three_reproduces_shipped_key_hashes() {
    for (miftah, mutawaqqa) in MIFATIH_ISDAR_THALITH {
        let hasil = basmat_miftah(IsdarLocres::MuhassanMadina, miftah);
        assert_eq!(
            hasil,
            Some(mutawaqqa),
            "key {miftah:?}: a shipped .locres stores {mutawaqqa:#010x} and this build \
             computes {hasil:#010x?}"
        );
    }
}

#[test]
fn version_three_reproduces_shipped_namespace_hashes() {
    for (fadaa, mutawaqqa) in FADAAT_ISDAR_THALITH {
        assert_eq!(
            basmat_miftah(IsdarLocres::MuhassanMadina, fadaa),
            Some(mutawaqqa),
            "namespace {fadaa:?}"
        );
    }
}

#[test]
fn case_is_part_of_the_key() {
    // Both are in one shipped file, under one namespace, with two hashes.
    assert_ne!(basmat_madina_miftah("Ok"), basmat_madina_miftah("OK"));
    assert_ne!(basmat_madina_miftah("Ok"), basmat_madina_miftah("ok"));
    assert_ne!(basmat_crc("Back"), basmat_crc("back"));
}

#[test]
fn the_empty_string_hashes_to_zero_in_both_hashed_versions() {
    assert_eq!(basmat_miftah(IsdarLocres::MuhassanMadina, ""), Some(0));
    assert_eq!(basmat_miftah(IsdarLocres::Muhassan, ""), Some(0));
    // And the fold of CityHash64 over no bytes is emphatically not zero, which
    // is why the empty case is stated rather than left to fall out.
    assert_ne!(tayy_basma(basmat_madina(&[])), 0);
}

#[test]
fn the_fold_is_not_a_truncation() {
    // A key long enough that CityHash64's high word is non-zero: the two
    // narrowings then disagree, and the file agrees with the fold.
    let kamil = basmat_madina_nass("ToggleInventory");
    let matwi = tayy_basma(kamil);
    let maqtu = u32::try_from(kamil & 0xFFFF_FFFF).unwrap_or(0);
    assert_ne!(matwi, maqtu);
    assert_eq!(
        matwi, 0x8f71_6c33,
        "the stored hash for \"ToggleInventory\""
    );
}

#[test]
fn get_type_hash_is_the_engines_arithmetic() {
    // Low word plus twenty-three times the high word, wrapping.
    assert_eq!(tayy_basma(0), 0);
    assert_eq!(tayy_basma(1), 1);
    assert_eq!(tayy_basma(1 << 32), 23);
    assert_eq!(
        tayy_basma(0xFFFF_FFFF_0000_0000),
        0xFFFF_FFFFu32.wrapping_mul(23)
    );
}

#[test]
fn versions_without_hashes_answer_none() {
    assert_eq!(basmat_miftah(IsdarLocres::Qadim, "Back"), None);
    assert_eq!(basmat_miftah(IsdarLocres::Mudmaj, "Back"), None);
}

#[test]
fn the_source_fingerprint_is_the_engines_crc() {
    // `FCrc::StrCrc32` over UTF-16 code units, four bytes fed per character.
    // Both values are the `basmat_asl` a shipped Tangles entry stores beside a
    // key whose English source string is the same word.
    assert_eq!(basmat_asl("Back"), 0x44d1_6ff1);
    assert_eq!(basmat_asl("Close"), 0x7f06_4a25);
    // Version 2 hashes its keys with the same function, un-folded and
    // un-lowercased.
    assert_eq!(
        basmat_miftah(IsdarLocres::Muhassan, "Back"),
        Some(0x44d1_6ff1)
    );
}

#[test]
fn crc_feeds_four_bytes_per_code_unit() {
    // Two significant bytes and two zeros per UTF-16 unit. Dropping the zeros
    // changes every hash in the file, so the property is pinned here against a
    // longhand CRC rather than left to the implementation to agree with itself.
    let mut yadawi = u32::MAX;
    for bayt in [0x41u8, 0x00, 0x00, 0x00] {
        yadawi = bi_khana(bayt, yadawi);
    }
    assert_eq!(basmat_crc("A"), !yadawi);
}

/// One byte through the reflected CRC-32 polynomial, written out longhand so
/// the test does not simply call the function it is checking.
fn bi_khana(bayt: u8, basma: u32) -> u32 {
    let mut hali = basma ^ u32::from(bayt);
    for _ in 0..8 {
        hali = if hali & 1 == 0 {
            hali >> 1
        } else {
            (hali >> 1) ^ 0xEDB8_8320
        };
    }
    hali
}

/// A resource built from nothing, edited, written, and read back — the write
/// half of the round-trip contract, without a game on the machine.
#[test]
fn an_authored_resource_carries_hashes_the_engine_would_look_up() {
    let mut mawrid = MawridLocres::jadeed(IsdarLocres::MuhassanMadina);
    assert!(mawrid.adif("", "Back", "Back", "رجوع").is_ok());
    assert!(mawrid.adif("Input_UI", "Close", "Close", "إغلاق").is_ok());
    mawrid.ahsi_ihsaat();

    assert_eq!(mawrid.basmat_mutabaqa(), Some(true));

    let Ok(bayt) = mawrid.ila_bayt() else {
        panic!("the resource would not serialize")
    };
    let Ok(thani) = MawridLocres::min_bayt(&bayt) else {
        panic!("a resource this build wrote would not read back")
    };
    assert_eq!(
        thani, mawrid,
        "a resource this build wrote must read back as itself"
    );
    assert_eq!(thani.ila_bayt().ok().as_deref(), Some(bayt.as_slice()));

    // The stored hashes are the engine's, not this build's private opinion.
    let Some(madkhal) = thani.jid("", "Back") else {
        panic!("the entry is not there")
    };
    assert_eq!(madkhal.basma(), Some(0x25ea_2e6c));
    assert_eq!(madkhal.basmat_asl(), 0x44d1_6ff1);
    assert_eq!(thani.tarjama("", "Back"), Some("رجوع"));
    // The unnamed namespace: hashed to zero, as every shipped file spells it.
    assert_eq!(
        thani.fadaat().first().map(FadaaLocres::basma),
        Some(Some(0))
    );

    // Arabic goes in as ordinary Unicode and is stored UTF-16, which is the
    // encoding Unreal picks for anything that is not pure ASCII.
    let Some(nass) = thani.hawd().first() else {
        panic!("the string array is empty")
    };
    assert_eq!(nass.nass().tarmiz(), TarmizNass::Utf16);
    assert_eq!(nass.nass().nass(), "رجوع");
}
