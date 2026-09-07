//! بيو٤ — Capcom's `BIO4` codebase, read out of the containers it ships and the
//! name its own executable carries.
//!
//! This is the engine Capcom wrote for Resident Evil 4 on the GameCube in 2005
//! and carried forward into the game's later ports. It has no published name and
//! no published version, so both are taken from the binary itself: the Windows
//! version resource of `bio4.exe` gives `InternalName` as `BIO4`, and that is
//! also the name of the directory the game reads its data out of.
//!
//! ## What this engine is not
//!
//! **It is not MT Framework.** That belief is common, it is repeated in
//! wiki articles and in modding threads, and every file on disk contradicts it.
//! The claim was checked against the shipped Ultimate HD Edition build rather
//! than argued:
//!
//! | checked | found |
//! | --- | --- |
//! | `MT Framework` / `MTFramework` in `bio4.exe` | zero occurrences |
//! | `UnrealEngine` / `Epic Games` in `bio4.exe` | zero occurrences |
//! | `BIO4` in `bio4.exe` | nineteen, every one of them a data path or a path prefix |
//! | textures | 616 `.tpl`, the GameCube texture format, magic `0x12345678` |
//! | MT Framework's `.tex` inside `ARC\0` | none |
//! | the eight files that *are* named `.arc` | `0x55AA382D`, Nintendo's U8 archive |
//! | compression | 5 915 `.lfs`, magic `RDLX` |
//!
//! The eight `.arc` files are the trap. MT Framework's archive extension is
//! `.arc` too, and a detector that stopped at the extension would file this game
//! under the wrong engine and send an extractor at it that cannot read one byte.
//! They are Wii Home-Button assets in Nintendo's U8 format, and this module
//! reads their magic and records what it found precisely so that the next reader
//! does not have to take the claim on trust.
//!
//! ## The four container headers this module reads
//!
//! Each is stated in full, because none of them is documented by Capcom and a
//! maintainer looking at a wrong answer has to be able to check the layout
//! against the file rather than against this code. Every number below was
//! verified across every file of that kind in the shipped game, and the count is
//! given so that a claim resting on one sample is visible as one.
//!
//! ### `.lfs` — the compression container. 5 915 of 5 915 agree.
//!
//! ```text
//! offset  size  field       notes
//! 0x00    4     magic       `RDLX`
//! 0x04    4     sentinel    u32 LE, 0xFEEEBAAA
//! 0x08    4     unknown     u32 LE — larger than the field below in 5 802 files
//!                           of 5 915 and smaller in 113, so it is *not* simply a
//!                           decompressed size and is not read here
//! 0x0C    4     payload     u32 LE, always strictly less than the file's length
//! ```
//!
//! The eight bytes at offset zero are the whole strength of this signature: an
//! arbitrary four-character tag followed by an arbitrary 32-bit sentinel is not
//! something another format lands on. The size at 0x0C is checked against the
//! real file length and nothing more is claimed about it — `file length minus
//! payload` runs from 36 to 61 530 bytes across the set, which is a range, not a
//! header size, and inventing a meaning for it would be inventing a fact.
//!
//! ### `.tpl` — the GameCube texture container. 616 of 616 agree.
//!
//! ```text
//! offset  size  field         notes
//! 0x00    4     magic         0x12345678, in *either* byte order — see below
//! 0x04    4     image count   u32 LE, observed 1, 2 and 3
//! 0x08    4     table offset  u32 LE, always 12
//! 0x0C    4     table end     u32 LE, always 12 + count * 8
//! ```
//!
//! 506 files store the magic as the bytes `12 34 56 78` — the constant written
//! big-endian, which is how a PowerPC GameCube wrote it — and 110 store it
//! byte-swapped as `78 56 34 12`. The fields *behind* the magic are
//! little-endian in both, which is the port's own fingerprint: the tool that
//! converted the assets swapped some files' magic and not others', and left the
//! bodies alone. Both orders are accepted.
//!
//! `0x12345678` on its own is a placeholder constant that appears all over the
//! place, which is exactly why the two structural fields behind it are required
//! rather than optional. A `.tpl` file that is a Smarty template does not have
//! twelve at offset eight.
//!
//! ### `.dct` — the text dictionary. 8 of 8 agree.
//!
//! ```text
//! offset  size  field       notes
//! 0x00    4     magic       `DICT`
//! 0x04    4     version     u32 LE, 0x1000 — the shipped loader compares it for
//!                           exact equality and refuses anything else
//! 0x08    4     seed        u32 LE, the CRC seed the game hashes key names with
//! 0x0C    4     buckets     u32 LE, a *self-relative* pointer to the bucket array
//! 0x10    4     count       u32 LE, how many buckets
//! ```
//!
//! The pointer is self-relative with a one-byte bias: the array is at
//! `0x0C + value + 1`. In every shipped file the value is 7, which puts the array
//! at 0x14, immediately behind the header. That is why the header field at 0x0C
//! reads as the constant `7` rather than as a pointer, and reading it as an
//! opaque constant is the single easiest way to get this format wrong.
//!
//! The check here is the one the game's own loader makes and no more: the magic,
//! the version compared exactly, and a bucket array that starts behind the
//! header and ends inside the file. `taarib-istikhraj`'s `qamus` module owns
//! this format properly — the buckets, the CRC-32 over key names, and a
//! byte-identical rebuild — and it is the place to read about it. This module
//! reads twenty bytes and stops, because a detector's job is to recognise the
//! file and not to open it.
//!
//! The strings are **not** uniformly UTF-8, so no encoding is asserted and none
//! is checked. Measured per string rather than over the whole file: the five
//! Latin-script dictionaries and `JAPANESE_WIN32.dct` decode cleanly in all 403
//! of their entries, and the two Chinese files do not — 7 strings in
//! `CHINESE_S_WIN32.dct` and 30 in `CHINESE_T_WIN32.dct` are Big5, which is what
//! `taarib-istikhraj`'s `qamus` module reports and refuses to guess at.
//!
//! ### `.udas` and `.snd` — a second container. 7 files, in both byte orders.
//!
//! A single u32 magic, `0x20BEB6CA`, stored big-endian in five files and
//! little-endian in two — the same split the `.tpl` magic shows. Nothing behind
//! it is understood, so nothing behind it is read, and this signature is
//! weighted below the three above because seven files is a small sample and the
//! module says so rather than pretending otherwise.
//!
//! ## What is known about the version, and what is not
//!
//! Capcom never versioned this engine. What the binary carries is a *file*
//! version, `1.0.0RELEASE_DEV.0`, which is the same string in the shipped
//! release as in a development build and says nothing about the engine. It is
//! recorded as evidence and is **never** parsed into an engine version, because
//! `1.0.0` presented as an engine version is a confident wrong answer and the
//! absence of one is not.
//!
//! What is readable is the year. `LegalCopyright` in the same resource reads
//! `©CAPCOM CO., LTD. 2005, 2014 ALL RIGHTS RESERVED.`, and the last year in it
//! is the year that build shipped — 2005 for the GameCube original the codebase
//! comes from, 2014 for the Ultimate HD Edition on disk here. That is the only
//! statement in the whole install that separates one generation of this codebase
//! from another, so it is reported as the version with
//! [`IsdarMuharrik::mushtaqq`] set: a year is not a release number, and
//! `crate::tahdid` must never let it outrank a version some other detector
//! actually read.
//!
//! ## What this module deliberately does not read
//!
//! The executable imports `d3d9.dll`, `d3dx9_43.dll`, `dinput8.dll`,
//! `xinput1_3.dll` and `x3daudio1_7.dll`, and it is a 32-bit i386 image. The
//! imports belong to [`crate::dalail::thunai`], which owns that evidence source
//! for every engine, and duplicating them here would be this module doing
//! another module's job. One consequence is worth stating plainly rather than
//! leaving to be discovered: [`taarib_mustalahat::muharrik::WajihaRusum`] has no
//! Direct3D 9 value, so a game on this engine reports **no** graphics API at
//! all. That is a gap in the vocabulary rather than a gap in the reading, and
//! closing it is a change to `taarib-mustalahat` that this work did not make.
//!
//! ## The character table is in the executable, not in the data
//!
//! Worth writing down here because this is the module a reader reaches for when
//! asking what is known about `BIO4`, and because it was for a long time the one
//! open question: **which code point selects which glyph cell is decided by a
//! table inside `bio4.exe`.** One flat array of `u32` code points per language,
//! whose index *is* the cell — 260 entries at `0x00C0_CE18` for the five Latin
//! scripts, and three larger pairs for Japanese and the two Chinese builds. The
//! routine that builds a `std::map` out of it is at `0x006A_7F50`; the `.dct`
//! string reaches it through a UTF-8 decoder at `0x006A_85A0` that also expands
//! `^917550^` into U+E002E. Every code point in all eight shipped dictionaries
//! resolves through those tables.
//!
//! `taarib-muhawwil-bio4`'s `kharita` module owns this, reproduces the Latin array
//! and records the file offsets of the two immediates that would point the game at
//! a longer one. This module does not read the executable for it: a detector's job
//! is to recognise the engine, and disassembling a nine-megabyte image to confirm
//! something already written down would be a scan nobody asked for.
//!
//! ## What a miss means
//!
//! Nothing. The scan is bounded in depth, in entries per directory, in entries
//! overall and in how many files of each kind it opens, so a container that
//! exists past those bounds was not looked at rather than found absent.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::muharrik::{AilatMuharrik, IsdarMuharrik, NawDaleel};
use taarib_usus::khata::Natija;

use crate::dalail::imtidad;
// Aliased on the way in because this module already has a `tanfidhi` of its
// own: the function that resolves which binary to read.
use crate::dalail::tanfidhi::{
    BayanTanfidhi, bayan as bayan_tanfidhi, iqra_nafidha, raqm32_sagheer,
};
use crate::fahs::{AQSA_UMQ, Fahis, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

// ---------------------------------------------------------------------------
// The names and constants the engine itself defines
// ---------------------------------------------------------------------------

/// The project name Capcom's own binary declares, in the case it declares it.
///
/// Read out of the executable's version resource as `InternalName`, and also the
/// name of the directory beside the executable that every data path in the
/// binary is written relative to — `BIO4/text/`, `BIO4\snd\bio4bgm.xwb`,
/// `BIO4/uvdata.t3d`. The two agreeing is why this is a naming rule and not a
/// guess.
pub const ISM_DAKHILI: &str = "BIO4";

/// The engine's data directory, folded to lower case for the index lookup.
const MUJALLAD_BAYANAT: &str = "bio4";

/// The directory the 32-bit executable lives in, folded to lower case.
///
/// Named for the architecture rather than for the game, and on its own it means
/// nothing: plenty of things ship a `Bin32`. It is recorded only once the data
/// directory has already been found.
const MUJALLAD_TANFIDH: &str = "bin32";

/// The `.lfs` compression container's four-character tag.
const SIHR_LFS: [u8; 4] = *b"RDLX";

/// The 32-bit sentinel behind that tag. Little-endian, so the bytes on disk are
/// `AA BA EE FE`.
const SHAHID_LFS: u32 = 0xFEEE_BAAA;

/// The text dictionary's four-character tag.
const SIHR_DICT: [u8; 4] = *b"DICT";

/// The only dictionary version the game's own loader accepts.
///
/// It compares this field for exact equality, so a reader that accepted more
/// would be recognising files the game itself refuses.
const ISDAR_DICT: u32 = 0x1000;

/// Where a `.dct` header keeps the self-relative pointer to its bucket array.
const MAWQI_MUASHIR_DICT: u32 = 12;

/// The GameCube texture container's magic, as a value rather than as bytes,
/// because the shipped files store it in both byte orders.
const SIHR_TPL: u32 = 0x1234_5678;

/// Where a `.tpl` header says its image table starts. Twelve in all 616 files.
const IZAHAT_JADWAL_TPL: u32 = 12;

/// One `.tpl` image-table record: an image header offset and a palette header
/// offset, four bytes each.
const TUL_QAYD_TPL: u32 = 8;

/// The most images a `.tpl` header may claim before the file is refused.
///
/// The shipped files hold one, two or three. Sixty-four is far above anything
/// observed and low enough that `count * 8` cannot be made to overflow or to
/// describe a table larger than any real header.
const AQSA_SUWAR_TPL: u32 = 64;

/// The `.udas` and `.snd` container magic, in the byte order the GameCube wrote
/// it. The port ships files in both orders and both are accepted.
const SIHR_UDAS: u32 = 0x20BE_B6CA;

/// Nintendo's U8 archive magic.
///
/// Present here only so the module can say what the eight `.arc` files are and,
/// by saying it, say what they are not. Nothing in this crate reads a U8
/// archive.
const SIHR_U8: u32 = 0x55AA_382D;

/// The fixed part of a `.dct` header, and therefore the earliest position its
/// bucket array can start at.
const TUL_TARWISAT_DICT: u32 = 20;

/// One `.dct` bucket: a key hash and a self-relative pointer to a string.
const TUL_QAYD_DICT: u32 = 8;

/// The most buckets a `.dct` header may claim.
///
/// The shipped files claim 403. A million is far above any plausible dictionary
/// and keeps `count * 8` inside a `u32` by construction.
const AQSA_QUYUD_DICT: u32 = 1_000_000;

// ---------------------------------------------------------------------------
// Bounds. Every one is a ceiling, never a target.
// ---------------------------------------------------------------------------

/// Bytes taken from the front of a container file.
///
/// The longest header this module reads is sixteen bytes. Sixty-four leaves room
/// to look at the file in a hex dump beside the layouts documented above without
/// reading anything that could be called content.
const HAJM_TARWISA_HAWIYA: usize = 64;

/// How many entries of any one directory the scan will look at.
///
/// Two hundred and fifty-six, and it is the bound that makes the scan
/// order-independent rather than the one that makes it fast. This game keeps
/// 2 246 files in `BIO4/ImagePack` and 2 515 in `BIO4/ImagePackHD`; without a
/// per-directory ceiling a single one of those could consume the whole budget
/// before `BIO4/text` was ever listed, and which of them came first would be
/// whatever order the filesystem happened to hand back.
const AQSA_MADAKHIL_MUJALLAD: usize = 256;

/// How many entries the whole scan will look at.
///
/// Four thousand and ninety-six, and the cost of it was measured rather than
/// guessed at: against the real install on a Windows drive seen through WSL —
/// the slowest directory listing this code will ever meet — the whole detector
/// takes about 1.5 seconds for a game that passes the gate and about ten
/// milliseconds for one that does not. Halving the budget takes it to a third of
/// a second and drops the two weakest observations, because the game keeps its
/// `.arc` archives three levels down behind a hundred and fifteen sibling
/// directories of room data.
///
/// The budget is kept and the second is spent. This runs once per game and its
/// answer is stored; the observations it buys are the ones that say what the
/// eight `.arc` files are, which is the single question a reader of this report
/// is most likely to get wrong on their own.
const AQSA_MADAKHIL_MASH: usize = 4096;

/// How many files of each kind the scan will keep, and therefore open.
///
/// Three. One would make every signature rest on whichever file the filesystem
/// listed first; a hundred would read a hundred headers to learn what the third
/// already said.
const HISSAT_SINF: usize = 3;

// ---------------------------------------------------------------------------
// Weights.
//
// The scale is the one documented on `HasilatFahs::sajjil` and on
// `crate::tahdid`: a file only one engine ever ships is 90 and up, a file
// several engines share is 40 to 60, a name that merely suggests something is
// below 30. Named here rather than written inline so the reasoning is
// reviewable in one place.
// ---------------------------------------------------------------------------

/// A `.lfs` file opening with `RDLX` and the sentinel behind it.
///
/// The strongest observation this detector can make. Eight bytes of arbitrary
/// constant at offset zero — a four-character tag nobody else uses followed by a
/// 32-bit value nobody else picked — with a size field behind them that fits the
/// real file. It is 96 rather than 100 because a detector is not permitted to
/// conclude, and because a file can be copied somewhere by something other than
/// the engine that wrote it.
const WAZN_SIHR_LFS: u8 = 96;

/// A `.dct` file whose header is `DICT` and whose entry table fits in front of
/// its strings.
///
/// Near-conclusive for the same reason the Unity container reader is: two
/// independent things had to agree, a four-character tag and a self-consistent
/// size relationship between three of its fields. Below the `.lfs` signature
/// because `DICT` is an English word and `RDLX` is not.
const WAZN_TARWISAT_DICT: u8 = 94;

/// The executable's version resource giving `InternalName` as `BIO4`.
///
/// The engine naming itself, in a structure Microsoft's resource compiler wrote
/// and Capcom filled in. Conclusive about which codebase this is and silent
/// about everything else.
const WAZN_ISM_DAKHILI: u8 = 90;

/// A `.tpl` file whose magic is `0x12345678` **and** whose two table fields
/// agree with its image count.
///
/// Deliberately below the two above. The magic alone is a placeholder constant
/// that turns up in test fixtures and in tutorials, so it is the structure that
/// carries this observation, and an observation whose strength comes from
/// structure rather than from an arbitrary constant is worth a little less.
const WAZN_TARWISAT_TPL: u8 = 90;

/// A `.udas` or `.snd` file opening with `0x20BEB6CA`.
///
/// An arbitrary 32-bit constant, which would earn more, held down to 85 because
/// there are seven such files in the whole install and nothing behind the magic
/// is understood. Seven samples is a small sample.
const WAZN_SIHR_UDAS: u8 = 85;

/// The engine's data directory, holding the interior shape it always holds.
///
/// `BIO4/` with `text/` and `Etc/` inside it. Strong shape evidence and only
/// shape evidence: no byte of any file has been read at this point.
const WAZN_BINYA_KAMILA: u8 = 78;

/// The version resource's copyright naming Capcom.
///
/// Corroboration only — it never names the family. Capcom published Resident
/// Evil 5 on MT Framework and Resident Evil 2 on the RE Engine, and both carry
/// the same company in the same field, so this says who made the game and not
/// what it was made with.
const WAZN_HUQUQ_CAPCOM: u8 = 70;

/// A directory named `BIO4` at the game's root, with nothing recognised inside
/// it.
///
/// A name, and a far more specific one than `Data` — the bottom band of the
/// scale is for names that merely suggest something, and this one names the
/// project. It is still only a name, and a mod folder somebody called `BIO4`
/// would produce it, so it sits at the floor of the band above rather than in it.
const WAZN_ISM_MUJALLAD: u8 = 62;

/// A `.arc` file that is a Nintendo U8 archive.
///
/// Recorded as corroboration and never allowed to name the family on its own: a
/// U8 archive says the assets came off a Nintendo console, which is true of this
/// lineage and of a great many other things. Its real job is to be in the
/// evidence trail when somebody asks why this is not MT Framework.
const WAZN_ARSHIF_U8: u8 = 55;

/// The executable the layout rule names, inside `Bin32/` beside the data
/// directory.
///
/// Recorded only once the data directory has been found, and never allowed to
/// name the family: on its own it is an executable inside a directory named
/// after an architecture, which is a shape half the games ever shipped have.
const WAZN_MUJALLAD_TANFIDH: u8 = 40;

/// The image-pack directories the HD release ships.
///
/// `ImagePack` and `ImagePackHD`. Generic names, recorded for the reader rather
/// than for the arithmetic.
const WAZN_HUZAM_SUWAR: u8 = 30;

// ---------------------------------------------------------------------------
// The detector
// ---------------------------------------------------------------------------

/// Detection of Capcom's `BIO4` codebase.
///
/// Holds no state; every listing and every read lives inside one call to
/// [`Fahis::ifhas`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FahisBio4;

impl FahisBio4 {
    /// Builds the detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Fahis for FahisBio4 {
    fn ism(&self) -> &'static str {
        "bio4"
    }

    /// # Errors
    ///
    /// Only when the game's root directory is not there. A root that will not
    /// list, a container that will not open and a header that will not parse are
    /// all the absence of evidence, which is [`HasilatFahs::la_shay`].
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        if !siyaq.jidhr.exists() {
            return Err(KhataMuharrik::JidhrMafqud { jidhr: siyaq.jidhr.to_path_buf() }.into());
        }

        let mut hasila = HasilatFahs::la_shay();
        let jidhr = fahras_mujallad(siyaq.jidhr);

        // The gate. One listing of the game root decides whether anything else
        // in this module runs, which is what keeps a detector for one engine
        // from costing a directory walk on every game in somebody's library.
        let bayanat = jidhr.mujallad(MUJALLAD_BAYANAT).map(ToOwned::to_owned);
        if bayanat.is_none() && !fihi_hawiya(siyaq, &jidhr) {
            return Ok(hasila);
        }

        let mash = masah(siyaq, bayanat.as_deref());
        hawiyat(&mash, &mut hasila);

        if let Some(nisbi) = bayanat.as_deref() {
            binya(siyaq, nisbi, &mut hasila);
        }
        tanfidhi(siyaq, &jidhr, bayanat.as_deref(), &mut hasila);

        Ok(hasila)
    }
}

// ---------------------------------------------------------------------------
// The bounded scan
// ---------------------------------------------------------------------------

/// One file the scan kept, and where it is.
#[derive(Debug)]
struct Murashah {
    /// Path relative to the game root, as evidence records it.
    nisbi: String,
    /// Absolute path, for the header read.
    masar: PathBuf,
}

/// The file extensions this module knows how to open.
///
/// `udas` and `snd` are one kind under two names: both open with the same magic
/// and neither is understood past it.
#[derive(Debug, Default)]
struct Hasad {
    /// Compression containers.
    lfs: Vec<Murashah>,
    /// Text dictionaries.
    dct: Vec<Murashah>,
    /// GameCube textures.
    tpl: Vec<Murashah>,
    /// The second container, under either of its two extensions.
    udas: Vec<Murashah>,
    /// Files named `.arc`, kept so the module can say what they actually are.
    arc: Vec<Murashah>,
}

impl Hasad {
    /// Whether every kind has as many candidates as it will use.
    fn iktafa(&self) -> bool {
        [&self.lfs, &self.dct, &self.tpl, &self.udas, &self.arc]
            .iter()
            .all(|sinf| sinf.len() >= HISSAT_SINF)
    }

    /// Files a file name of this kind, when that kind still has room.
    fn daa(&mut self, ism: &str, nisbi: String, masar: PathBuf) {
        let hadaf = if imtidad(ism, "lfs") {
            &mut self.lfs
        } else if imtidad(ism, "dct") {
            &mut self.dct
        } else if imtidad(ism, "tpl") {
            &mut self.tpl
        } else if imtidad(ism, "udas") || imtidad(ism, "snd") {
            &mut self.udas
        } else if imtidad(ism, "arc") {
            &mut self.arc
        } else {
            return;
        };
        if hadaf.len() < HISSAT_SINF {
            hadaf.push(Murashah { nisbi, masar });
        }
    }
}

/// Whether the game's root itself holds a container this module recognises.
///
/// The second half of the gate, and it reads headers rather than trusting
/// extensions. `.tpl` is a template extension in half the world's build systems,
/// and a game shipping one at its root would otherwise buy a four-thousand-entry
/// directory walk for a file that is not a texture. Bounded by the root listing
/// and by [`HISSAT_SINF`] small reads per kind.
///
/// It runs the same header readers the detector runs, so the gate and the answer
/// cannot disagree: whatever this accepts is exactly what the module would have
/// recorded, and whatever it refuses is a file the module had nothing to say
/// about anyway.
fn fihi_hawiya(siyaq: &SiyaqFahs<'_>, jidhr: &Fahras) -> bool {
    let mut hasad = Hasad::default();
    for (_, haqiqi, mujallad) in &jidhr.madakhil {
        if *mujallad {
            continue;
        }
        let Ok(masar) = siyaq.dakhil(haqiqi) else { continue };
        hasad.daa(haqiqi, haqiqi.clone(), masar);
    }
    let mut hasila = HasilatFahs::la_shay();
    hawiyat(&hasad, &mut hasila);
    hasila.wajad()
}

/// Walks the game for container files, breadth first and bounded four ways.
///
/// Breadth first rather than depth first, and with a ceiling on each directory
/// as well as on the walk overall, because this game's asset directories hold
/// thousands of files each and a depth-first walk that entered one of them first
/// would spend its whole budget there. Every sibling directory gets its turn
/// under this arrangement whatever order the filesystem lists them in, which is
/// what makes the answer the same on two machines.
///
/// The walk starts at the engine's data directory when there is one and at the
/// game root when there is not, so that a flatter layout than the one shipped
/// here is still reachable.
fn masah(siyaq: &SiyaqFahs<'_>, bayanat: Option<&str>) -> Hasad {
    let mut hasad = Hasad::default();
    let mut tabur: VecDeque<(Option<String>, usize)> = VecDeque::new();
    let bidaya = bayanat.map(ToOwned::to_owned);
    let umq_bidaya = usize::from(bidaya.is_some());
    tabur.push_back((bidaya, umq_bidaya));

    let mut zurr = 0_usize;
    while let Some((nisbi, umq)) = tabur.pop_front() {
        if zurr >= AQSA_MADAKHIL_MASH || hasad.iktafa() {
            break;
        }
        let masar = match nisbi.as_deref() {
            Some(nisbi) => {
                let Ok(masar) = siyaq.dakhil(nisbi) else { continue };
                masar
            }
            None => siyaq.jidhr.to_path_buf(),
        };
        let Ok(madakhil) = fs::read_dir(&masar) else { continue };

        for (adad, madkhal) in madakhil.enumerate() {
            if adad >= AQSA_MADAKHIL_MUJALLAD || zurr >= AQSA_MADAKHIL_MASH {
                break;
            }
            zurr = zurr.saturating_add(1);
            let Ok(madkhal) = madkhal else { continue };
            let khaam = madkhal.file_name();
            let Some(ism) = khaam.to_str() else { continue };
            let dakhili = match nisbi.as_deref() {
                Some(sabiq) => format!("{sabiq}/{ism}"),
                None => ism.to_owned(),
            };

            if huwa_mujallad(&madkhal) {
                if umq < AQSA_UMQ {
                    tabur.push_back((Some(dakhili), umq.saturating_add(1)));
                }
                continue;
            }
            let Ok(kamil) = siyaq.dakhil(&dakhili) else { continue };
            hasad.daa(ism, dakhili, kamil);
        }
    }
    hasad
}

/// Whether a directory entry is a directory, following a symbolic link to
/// whatever it points at.
///
/// Classified by what it points at rather than by being a link, which matches
/// [`SiyaqFahs::yujad`]. Containment is not weakened by that: every path built
/// from a name found here goes back through [`SiyaqFahs::dakhil`].
fn huwa_mujallad(madkhal: &fs::DirEntry) -> bool {
    match madkhal.file_type() {
        Ok(naw) if naw.is_symlink() => fs::metadata(madkhal.path()).is_ok_and(|wasf| wasf.is_dir()),
        Ok(naw) => naw.is_dir(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// The container headers
// ---------------------------------------------------------------------------

/// Opens the headers the scan collected and records what each one was.
///
/// Every candidate of a kind is tried until one parses, and a candidate that
/// does not parse does not disqualify the kind: a file carrying the extension
/// and not the header is exactly what an unrelated `.tpl` template is, and
/// stepping over it is the whole point of reading the header at all.
fn hawiyat(mash: &Hasad, hasila: &mut HasilatFahs) {
    lfs(&mash.lfs, hasila);
    dict(&mash.dct, hasila);
    tpl(&mash.tpl, hasila);
    udas(&mash.udas, hasila);
    arshif(&mash.arc, hasila);
}

/// The `RDLX` compression container.
fn lfs(murashahun: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(tul) = tul_malaf(&murashah.masar) else { continue };
        let Some(hajm) = tarwisat_lfs(&nafidha, tul) else { continue };
        hasila.sajjil_aila(
            AilatMuharrik::Bio4,
            NawDaleel::TarwisatHawiya,
            format!(
                "`RDLX` compression container with the sentinel {SHAHID_LFS:#010X} behind it, \
                 declaring {hajm} bytes of payload inside a file of {tul}"
            ),
            Some(murashah.nisbi.clone()),
            WAZN_SIHR_LFS,
        );
        return;
    }
}

/// The `DICT` text dictionary.
fn dict(murashahun: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(tul) = tul_malaf(&murashah.masar) else { continue };
        let Some(tarwisa) = tarwisat_dict(&nafidha, tul) else { continue };
        hasila.sajjil_aila(
            AilatMuharrik::Bio4,
            NawDaleel::TarwisatHawiya,
            format!(
                "`DICT` text dictionary, version {ISDAR_DICT:#06X}: {} buckets of eight bytes \
                 from {} to {}, hashed with the seed {:#010X}. Only the header was read; the \
                 buckets and the strings behind them belong to the dictionary reader.",
                tarwisa.adad, tarwisa.asas_jadwal, tarwisa.nihayat_jadwal, tarwisa.bidhra
            ),
            Some(murashah.nisbi.clone()),
            WAZN_TARWISAT_DICT,
        );
        return;
    }
}

/// The GameCube texture container.
fn tpl(murashahun: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(tarwisa) = tarwisat_tpl(&nafidha) else { continue };
        let tarteeb = if tarwisa.kabir {
            "big-endian, as the GameCube wrote it"
        } else {
            "byte-swapped to little-endian by the port's asset conversion"
        };
        hasila.sajjil_aila(
            AilatMuharrik::Bio4,
            NawDaleel::TarwisatHawiya,
            format!(
                "`.tpl` GameCube texture container, magic {SIHR_TPL:#010X} written {tarteeb}, \
                 holding {} image{} with the image table at {IZAHAT_JADWAL_TPL} and ending at \
                 {}, which is what the count requires",
                tarwisa.adad,
                if tarwisa.adad == 1 { "" } else { "s" },
                tarwisa.nihayat_jadwal
            ),
            Some(murashah.nisbi.clone()),
            WAZN_TARWISAT_TPL,
        );
        return;
    }
}

/// The second container, under `.udas` or `.snd`.
fn udas(murashahun: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(kabir) = sihr_fi_ittijahayn(&nafidha, SIHR_UDAS) else { continue };
        let tarteeb = if kabir { "big-endian" } else { "little-endian" };
        hasila.sajjil_aila(
            AilatMuharrik::Bio4,
            NawDaleel::TarwisatHawiya,
            format!(
                "container magic {SIHR_UDAS:#010X}, written {tarteeb}. Nothing behind it is \
                 understood and nothing behind it was read. It is weighted below this \
                 module's three other signatures because it was established from seven files \
                 rather than from hundreds or thousands, and a small sample is a weaker claim \
                 whatever the magic looks like."
            ),
            Some(murashah.nisbi.clone()),
            WAZN_SIHR_UDAS,
        );
        return;
    }
}

/// The `.arc` files, and what they actually are.
///
/// The one observation here that exists to refute something rather than to
/// establish it. `.arc` is also MT Framework's archive extension, and a reader
/// who meets eight of them in a Capcom game will reach for that conclusion; the
/// magic says Nintendo's U8 instead, and the evidence trail should say so
/// without anybody having to open a hex editor.
fn arshif(murashahun: &[Murashah], hasila: &mut HasilatFahs) {
    for murashah in murashahun {
        let Some(nafidha) = iqra_nafidha(&murashah.masar, 0, HAJM_TARWISA_HAWIYA) else {
            continue;
        };
        let Some(sihr) = raqm32_kabir(&nafidha, 0) else { continue };
        if sihr != SIHR_U8 {
            continue;
        }
        hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            format!(
                "a file named `.arc` that opens with {SIHR_U8:#010X} — Nintendo's U8 archive, \
                 not MT Framework's `ARC\\0`. The extension is shared and the format is not, \
                 which is the single thing most likely to be mistaken about this engine."
            ),
            Some(murashah.nisbi.clone()),
            WAZN_ARSHIF_U8,
        );
        return;
    }
}

/// Whether a `.lfs` header is one, and how much payload it declares.
///
/// Three conditions, and the third is the one that stops an arbitrary file from
/// passing: the declared payload has to fit inside the file that carries it. The
/// word at 0x08 is not read, because across the shipped set it is larger than
/// the payload in 5 802 files and smaller in 113, so whatever it is, it is not
/// the decompressed size a reader would assume.
fn tarwisat_lfs(nafidha: &[u8], tul: u64) -> Option<u64> {
    if !nafidha.starts_with(&SIHR_LFS) {
        return None;
    }
    if raqm32_sagheer(nafidha, 4)? != SHAHID_LFS {
        return None;
    }
    let hajm = u64::from(raqm32_sagheer(nafidha, 12)?);
    (hajm > 0 && hajm < tul).then_some(hajm)
}

/// A `.dct` header, once its own fields have been checked against each other.
#[derive(Debug)]
struct TarwisatDict {
    /// The CRC seed the game hashes key names with, recorded rather than used.
    bidhra: u32,
    /// How many buckets the header claims.
    adad: u32,
    /// Where the bucket array starts, once the self-relative pointer at 0x0C
    /// has been resolved.
    asas_jadwal: u32,
    /// Where it ends, which has to be inside the file.
    nihayat_jadwal: u32,
}

/// Reads a `.dct` header and refuses one whose fields contradict each other.
///
/// The version is compared for exact equality because the game's own loader
/// does, and the bucket pointer is resolved the way the game resolves it —
/// self-relative from its own field, with a one-byte bias — so that the check
/// is the loader's check and not a paraphrase of it. Nothing is asserted about
/// the buckets themselves, about the key hashes, or about the strings'
/// encoding: `taarib-istikhraj`'s `qamus` module owns all three, and a second
/// opinion from a detector would be a second place to keep them right.
fn tarwisat_dict(nafidha: &[u8], tul: u64) -> Option<TarwisatDict> {
    if !nafidha.starts_with(&SIHR_DICT) {
        return None;
    }
    if raqm32_sagheer(nafidha, 4)? != ISDAR_DICT {
        return None;
    }
    let bidhra = raqm32_sagheer(nafidha, 8)?;
    let muashir = raqm32_sagheer(nafidha, 12)?;
    let adad = raqm32_sagheer(nafidha, 16)?;
    if adad == 0 || adad > AQSA_QUYUD_DICT {
        return None;
    }
    let asas_jadwal = MAWQI_MUASHIR_DICT.checked_add(muashir)?.checked_add(1)?;
    if asas_jadwal < TUL_TARWISAT_DICT {
        return None;
    }
    let nihayat_jadwal = asas_jadwal.checked_add(adad.checked_mul(TUL_QAYD_DICT)?)?;
    if u64::from(nihayat_jadwal) > tul {
        return None;
    }
    Some(TarwisatDict { bidhra, adad, asas_jadwal, nihayat_jadwal })
}

/// A `.tpl` header, once its table fields have been checked against its count.
#[derive(Debug)]
struct TarwisatTpl {
    /// How many images the header claims.
    adad: u32,
    /// Where the image table ends, which the count decides.
    nihayat_jadwal: u32,
    /// Whether the magic was stored big-endian.
    kabir: bool,
}

/// Reads a `.tpl` header.
///
/// The magic alone would be worthless — `0x12345678` is the constant everybody
/// reaches for when they need one — so the two fields behind it carry the check:
/// the image table starts at twelve and ends exactly where the image count puts
/// it. The fields are little-endian whichever way the magic was written, which
/// is the port's own quirk and is documented at the top of this module.
fn tarwisat_tpl(nafidha: &[u8]) -> Option<TarwisatTpl> {
    let kabir = sihr_fi_ittijahayn(nafidha, SIHR_TPL)?;
    let adad = raqm32_sagheer(nafidha, 4)?;
    if adad == 0 || adad > AQSA_SUWAR_TPL {
        return None;
    }
    if raqm32_sagheer(nafidha, 8)? != IZAHAT_JADWAL_TPL {
        return None;
    }
    let nihayat_jadwal = IZAHAT_JADWAL_TPL.checked_add(adad.checked_mul(TUL_QAYD_TPL)?)?;
    if raqm32_sagheer(nafidha, 12)? != nihayat_jadwal {
        return None;
    }
    Some(TarwisatTpl { adad, nihayat_jadwal, kabir })
}

/// Whether the first four bytes are `matlub` in either byte order, and which.
///
/// `Some(true)` for big-endian, `Some(false)` for little-endian, `None` for
/// neither. Both are accepted because this port ships both: the conversion that
/// produced the Windows assets swapped some files' magic and left others alone,
/// and a reader that took one side would miss between a sixth and five sixths of
/// them depending on which side it took.
fn sihr_fi_ittijahayn(nafidha: &[u8], matlub: u32) -> Option<bool> {
    if raqm32_kabir(nafidha, 0)? == matlub {
        return Some(true);
    }
    (raqm32_sagheer(nafidha, 0)? == matlub).then_some(false)
}

// ---------------------------------------------------------------------------
// The directory shape
// ---------------------------------------------------------------------------

/// The subdirectories the engine's data directory holds, folded to lower case.
///
/// `text` and `Etc` are the two that decide the weight: the first is where the
/// dictionaries live and the second is where the shared containers do, and a
/// directory called `BIO4` with both of them inside it is this engine's layout
/// rather than a folder somebody named after the game.
const MUJALLADAT_BAYANAT: &[&str] = &["text", "etc", "font", "imagepack", "imagepackhd"];

/// Records the engine's data directory and what is inside it.
fn binya(siyaq: &SiyaqFahs<'_>, nisbi: &str, hasila: &mut HasilatFahs) {
    let Ok(masar) = siyaq.dakhil(nisbi) else { return };
    let dakhil = fahras_mujallad(&masar);

    let mawjuda: Vec<&str> =
        MUJALLADAT_BAYANAT.iter().filter_map(|ism| dakhil.mujallad(ism)).collect();
    let marahil = dakhil.marahil();
    let kamila = dakhil.mujallad("text").is_some() && dakhil.mujallad("etc").is_some();

    let wasf = if kamila {
        let marahil = if marahil == 0 {
            String::new()
        } else {
            let jam = if marahil == 1 { "y" } else { "ies" };
            format!(", beside {marahil} stage director{jam}")
        };
        format!(
            "engine data directory `{nisbi}` in the shape this codebase reads it: {} inside \
             it{marahil}. Every data path in the executable is written relative to this \
             directory.",
            mawjuda.join(", ")
        )
    } else {
        format!(
            "a directory named `{nisbi}` at the game's root, which is this engine's project \
             name — and nothing recognised inside it, so this is a name and not a layout"
        )
    };
    hasila.sajjil_aila(
        AilatMuharrik::Bio4,
        NawDaleel::BinyatMujallad,
        wasf,
        Some(nisbi.to_owned()),
        if kamila { WAZN_BINYA_KAMILA } else { WAZN_ISM_MUJALLAD },
    );

    for ism in ["imagepack", "imagepackhd"] {
        let Some(haqiqi) = dakhil.mujallad(ism) else { continue };
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!("`{haqiqi}`, where this engine keeps its packed screen images"),
            Some(format!("{nisbi}/{haqiqi}")),
            WAZN_HUZAM_SUWAR,
        );
    }
}

// ---------------------------------------------------------------------------
// The executable
// ---------------------------------------------------------------------------

/// Resolves the game's executable from the layout rule, and reads what its
/// version resource says about itself.
///
/// The rule is the engine's own and it is why an executable may be *named* here
/// rather than guessed at: the data directory is the project's internal name,
/// and the binary is that same name with `.exe` on it, inside `Bin32` beside the
/// data directory. Where discovery already named an executable and the rule
/// names none, discovery's is read and nothing is claimed about it.
fn tanfidhi(
    siyaq: &SiyaqFahs<'_>,
    jidhr: &Fahras,
    bayanat: Option<&str>,
    hasila: &mut HasilatFahs,
) {
    let mathbut = tanfidhi_min_binya(siyaq, jidhr, bayanat);
    if let Some((nisbi, masar)) = mathbut.as_ref() {
        hasila.tanfidhi = Some(masar.clone());
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "`{nisbi}`, named by the data directory beside it: this engine's binary is \
                 its project name with `.exe` on it, in the 32-bit binary directory beside \
                 the data"
            ),
            Some(nisbi.clone()),
            WAZN_MUJALLAD_TANFIDH,
        );
    }

    let maqru = mathbut.or_else(|| {
        let masar = siyaq.tanfidhi?;
        let nisbi = masar
            .strip_prefix(siyaq.jidhr)
            .ok()
            .and_then(Path::to_str)
            .map(|nass| nass.replace('\\', "/"))?;
        Some((nisbi, masar.to_path_buf()))
    });
    let Some((nisbi, masar)) = maqru else { return };
    let Some(bayan) = bayan_tanfidhi(&masar) else { return };

    if let Some(mimariya) = bayan.mimariya {
        hasila.mimariya = Some(mimariya);
    }
    // The order is the rule. The internal name is what says this binary *is*
    // this engine; the copyright is what says who wrote it. A year taken from
    // the second without the first would be a year off some other Capcom title's
    // binary reported as this engine's build.
    let huwa = ism_dakhili(&bayan, &nisbi, hasila);
    huquq(&bayan, &nisbi, huwa, hasila);
    isdar_malaf(&bayan, &nisbi, hasila);
}

/// The executable the layout rule proves, when both halves of it are there.
fn tanfidhi_min_binya(
    siyaq: &SiyaqFahs<'_>,
    jidhr: &Fahras,
    bayanat: Option<&str>,
) -> Option<(String, PathBuf)> {
    let mujallad = bayanat?;
    let ism = format!("{}.exe", mujallad.to_ascii_lowercase());
    if let Some(haqiqi) = jidhr.mujallad(MUJALLAD_TANFIDH) {
        let masar = siyaq.dakhil(haqiqi).ok()?;
        if let Some(mawjud) = fahras_mujallad(&masar).malaf(&ism) {
            // Both halves spelled as the filesystem spells them, not as this
            // module asked for them: the lookup is case-insensitive and the
            // evidence line has to name a path a reader can go and look at.
            let nisbi = format!("{haqiqi}/{mawjud}");
            return siyaq.dakhil(&nisbi).ok().map(|kamil| (nisbi, kamil));
        }
    }
    let mawjud = jidhr.malaf(&ism)?.to_owned();
    siyaq.dakhil(&mawjud).ok().map(|kamil| (mawjud, kamil))
}

/// Records the internal name, which is the engine naming itself.
///
/// Returns whether the binary named this engine, because the copyright behind it
/// only means something about *this* engine once it has.
fn ism_dakhili(bayan: &BayanTanfidhi, nisbi: &str, hasila: &mut HasilatFahs) -> bool {
    let Some(ism) = bayan.ism_dakhili.as_deref() else { return false };
    if !ism.eq_ignore_ascii_case(ISM_DAKHILI) {
        return false;
    }
    let dhayl = bayan
        .sharika
        .as_deref()
        .map_or_else(String::new, |sharika| format!(", by `{sharika}`"));
    hasila.sajjil_aila(
        AilatMuharrik::Bio4,
        NawDaleel::BayanatMudmaja,
        format!(
            "the executable's version resource gives `InternalName` as `{ism}`{dhayl} — the \
             project name this codebase was built under, and the name of the data directory \
             every path in the binary is written relative to"
        ),
        Some(nisbi.to_owned()),
        WAZN_ISM_DAKHILI,
    );
    true
}

/// Records the copyright line and takes the build's year out of it.
///
/// Corroboration and never an identification, however specific it looks. Every
/// Capcom title carries a Capcom copyright, including the MT Framework and RE
/// Engine ones this module exists to be told apart from, so a sentence naming
/// the company says who published the game and nothing whatever about which
/// engine it runs on.
///
/// `huwa` is whether the binary's internal name said this engine. The year is
/// only taken when it did, because "the year on this build of this engine" is a
/// claim about this engine, and reading it off a binary that never said it was
/// one would be exactly the confident wrong answer the module refuses elsewhere.
fn huquq(bayan: &BayanTanfidhi, nisbi: &str, huwa: bool, hasila: &mut HasilatFahs) {
    let Some(huquq) = bayan.huquq.as_deref() else { return };
    if !huquq.to_ascii_uppercase().contains("CAPCOM") {
        return;
    }
    hasila.sajjil(
        NawDaleel::BayanatMudmaja,
        format!(
            "the executable's version resource carries the copyright `{huquq}`, which names \
             the publisher and not the engine"
        ),
        Some(nisbi.to_owned()),
        WAZN_HUQUQ_CAPCOM,
    );

    if !huwa {
        return;
    }
    let Some(sana) = sanat_akhira(huquq) else { return };
    hasila.isdar = Some(IsdarMuharrik {
        kabir: sana,
        sagheer: 0,
        tasheeh: 0,
        khaam: sana.to_string(),
        mushtaqq: true,
    });
    hasila.sajjil(
        NawDaleel::BayanatMudmaja,
        format!(
            "the last year in that copyright is {sana}, and it is reported as this build's \
             version because it is the only thing in the whole install that separates one \
             generation of this codebase from another. Capcom published no version for this \
             engine, so {sana} is a year and not a release number, and it is marked derived so \
             that it can never outrank a version some other detector actually read."
        ),
        Some(nisbi.to_owned()),
        WAZN_HUQUQ_CAPCOM,
    );
}

/// Records the file version, and refuses to read an engine version out of it.
fn isdar_malaf(bayan: &BayanTanfidhi, nisbi: &str, hasila: &mut HasilatFahs) {
    let Some(khaam) = bayan.isdar_malaf.as_deref() else { return };
    hasila.sajjil(
        NawDaleel::BayanatMudmaja,
        format!(
            "the executable's `FileVersion` is `{khaam}`. It is recorded and it is \
             deliberately not parsed into an engine version: it is the game binary's own \
             version field, this engine has no version numbering of its own, and three \
             numbers presented as one would be a confident wrong answer where the honest \
             result is that there is none."
        ),
        Some(nisbi.to_owned()),
        0,
    );
}

/// The last four-digit year in a line of text.
///
/// Four digits between 1990 and 2100, so that a version fragment or a product
/// code cannot be read as a year. The *last* one because a copyright that names
/// two years names the original and the build, in that order.
fn sanat_akhira(nass: &str) -> Option<u16> {
    let mut akhira: Option<u16> = None;
    for juz in nass.split(|harf: char| !harf.is_ascii_digit()) {
        if juz.len() == 4
            && let Ok(sana) = juz.parse::<u16>()
            && (1990..=2100).contains(&sana)
        {
            akhira = Some(sana);
        }
    }
    akhira
}

// ---------------------------------------------------------------------------
// Directories
// ---------------------------------------------------------------------------

/// One directory's entries, indexed for case-insensitive lookup.
///
/// The same shape [`crate::dalail::unity`] keeps, and for the same reason: these
/// names come off a Windows filesystem and are asked for by a module that spells
/// them however it likes, so `BIO4`, `Bio4` and `bio4` have to be one directory
/// rather than three.
#[derive(Debug, Default)]
struct Fahras {
    /// `(lowered name, name as the filesystem spells it, is a directory)`.
    madakhil: Vec<(String, String, bool)>,
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

    /// How many `St<digit>` stage directories are here.
    ///
    /// The shipped game has eight, `St0` through `St7`, and they are the engine's
    /// own division of its world data. Counted rather than named because the
    /// count is what a reader can check and the names carry nothing more.
    fn marahil(&self) -> usize {
        self.madakhil
            .iter()
            .filter(|(khafid, _, mujallad)| {
                *mujallad
                    && khafid.len() == 3
                    && khafid.starts_with("st")
                    && khafid.ends_with(|harf: char| harf.is_ascii_digit())
            })
            .count()
    }

}

/// Lists one directory, bounded, and returns an empty index when it will not
/// list.
///
/// An unreadable directory is the absence of evidence rather than a failure:
/// [`Fahis::ifhas`] is only allowed to fail for a root that is not there.
fn fahras_mujallad(masar: &Path) -> Fahras {
    let Ok(madakhil) = fs::read_dir(masar) else { return Fahras::default() };
    let mut fahras = Fahras::default();
    for (adad, madkhal) in madakhil.enumerate() {
        if adad >= AQSA_MADAKHIL_MUJALLAD {
            break;
        }
        let Ok(madkhal) = madkhal else { continue };
        let khaam = madkhal.file_name();
        let Some(ism) = khaam.to_str() else { continue };
        let mujallad = huwa_mujallad(&madkhal);
        fahras.madakhil.push((ism.to_ascii_lowercase(), ism.to_owned(), mujallad));
    }
    fahras
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

#[cfg(test)]
mod ikhtibarat {
    use std::fs::File;
    use std::io::Write as _;

    use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

    use super::*;
    use crate::dalail::tanfidhi::suwar::{Sura, sawwir};
    use crate::tahdid;

    // -----------------------------------------------------------------------
    // Fixtures.
    //
    // Every header below is the real bytes off the shipped Ultimate HD Edition
    // install, copied verbatim, with the file padded out to the real file's
    // length behind it. The padding is zeros and is never read: the detector
    // opens sixty-four bytes and checks the length, so what a fixture has to
    // reproduce is the header and the size, and inventing a body would be
    // inventing evidence.
    // -----------------------------------------------------------------------

    /// `BIO4/Font/common_j.fnt.lfs`, and its real length.
    const TARWISAT_LFS: [u8; 16] = [
        0x52, 0x44, 0x4C, 0x58, 0xAA, 0xBA, 0xEE, 0xFE, 0x70, 0x08, 0x00, 0x00, 0xA8, 0x02,
        0x00, 0x00,
    ];
    /// The length of that file on disk.
    const TUL_LFS: usize = 716;

    /// `BIO4/Etc/moji8.tpl` — the magic stored big-endian, one image.
    const TARWISAT_TPL_KABIR: [u8; 16] = [
        0x12, 0x34, 0x56, 0x78, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x14, 0x00,
        0x00, 0x00,
    ];

    /// `BIO4/SS/chs/f00a.tpl` — the same header with the magic byte-swapped.
    const TARWISAT_TPL_SAGHEER: [u8; 16] = [
        0x78, 0x56, 0x34, 0x12, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x14, 0x00,
        0x00, 0x00,
    ];
    /// The length of both of those files on disk.
    const TUL_TPL: usize = 64;

    /// `BIO4/text/ENGLISH_WIN32.dct` — magic, version, seed, bucket pointer and
    /// bucket count.
    const TARWISAT_DICT: [u8; 20] = [
        0x44, 0x49, 0x43, 0x54, 0x00, 0x10, 0x00, 0x00, 0x9F, 0x7D, 0xD5, 0x55, 0x07, 0x00,
        0x00, 0x00, 0x93, 0x01, 0x00, 0x00,
    ];
    /// The length of that file on disk.
    const TUL_DICT: usize = 10_824;

    /// `BIO4/Etc/core.udas` — the magic stored big-endian.
    const TARWISAT_UDAS: [u8; 4] = [0x20, 0xBE, 0xB6, 0xCA];

    /// `BIO4/Etc/movie_chs.udas` — the same magic byte-swapped.
    const TARWISAT_UDAS_SAGHEER: [u8; 4] = [0xCA, 0xB6, 0xBE, 0x20];

    /// `BIO4/iww/HomeButton2/homeBtn.arc` — Nintendo's U8 magic.
    const TARWISAT_ARC: [u8; 4] = [0x55, 0xAA, 0x38, 0x2D];

    /// The whole `VS_VERSION_INFO` resource of `Bin32/bio4.exe`, verbatim.
    ///
    /// 872 bytes lifted out of the shipped executable at offset 8 703 480, which
    /// is where that image's own resource tree says the block starts and for
    /// exactly the length its leaf declares. It is here rather than
    /// reconstructed because the one thing a parser of this structure can get
    /// wrong is the padding between a key and its value, and the real block
    /// carries both cases: `InternalName` needs none and `CompanyName` needs one
    /// zero code unit.
    const MAWARID: &str = "\
        680334000000560053005F00560045005200530049004F004E005F0049004E0046004F0000000000\
        BD04EFFE00000100000001000000000000000100000000003F000000000000000400040001000000\
        000000000000000000000000C6020000010053007400720069006E006700460069006C0065004900\
        6E0066006F000000A202000001003000300030003000300034006200300000004600130001004300\
        6F006D00700061006E0079004E0061006D0065000000000043004100500043004F004D0020005500\
        2E0053002E0041002C00200049004E0043002E000000000064001E000100460069006C0065004400\
        650073006300720069007000740069006F006E00000000005200650073006900640065006E007400\
        20004500760069006C002000340020002F002000420069006F00680061007A006100720064002000\
        34000000460013000100460069006C006500560065007200730069006F006E000000000031002E00\
        30002E003000520045004C0045004100530045005F004400450056002E003000000000002A000500\
        010049006E007400650072006E0061006C004E0061006D0065000000420049004F00340000000000\
        8800320001004C006500670061006C0043006F0070007900720069006700680074000000A9004300\
        4100500043004F004D00200043004F002E002C0020004C00540044002E0020003200300030003500\
        2C0020003200300031003400200041004C004C002000520049004700480054005300200052004500\
        5300450052005600450044002E0000003A00090001004F0072006900670069006E0061006C004600\
        69006C0065006E0061006D0065000000620069006F0034002E00650078006500000000005C001E00\
        0100500072006F0064007500630074004E0061006D00650000000000520065007300690064006500\
        6E00740020004500760069006C002000340020002F002000420069006F00680061007A0061007200\
        64002000340000004A0013000100500072006F006400750063007400560065007200730069006F00\
        6E00000031002E0030002E003000520045004C0045004100530045005F004400450056002E003000\
        00000000440000000100560061007200460069006C00650049006E0066006F000000000024000400\
        00005400720061006E0073006C006100740069006F006E00000000000000B004";

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Decodes a hex fixture, dropping anything that is not a hex digit.
    fn min_sitteen(nass: &str) -> Vec<u8> {
        let arqam: Vec<u8> =
            nass.bytes().filter(u8::is_ascii_hexdigit).map(qeemat_raqm).collect();
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

    /// A header padded out to the real file's length with zeros.
    fn mamdud(tarwisa: &[u8], tul: usize) -> Vec<u8> {
        let mut bayt = tarwisa.to_vec();
        bayt.resize(tul.max(tarwisa.len()), 0);
        bayt
    }

    /// A minimal 32-bit PE image carrying `mawarid` as its version resource.
    ///
    /// The wrapper is built by the reader's own test support and the resource
    /// bytes inside it are not: what is under test is that the resource tree is
    /// walked correctly and that the real `String` structures at the end of it
    /// decode, and a nine-megabyte executable in the source tree would test the
    /// same thing and cost a hundred times as much to read.
    fn pe_bi_mawarid(mawarid: &[u8]) -> Vec<u8> {
        sawwir(&Sura {
            alat: 0x014C,
            rdata: b"",
            ism_mawarid: b".rsrc",
            hashw: 0,
            kutla: mawarid,
        })
    }

    /// Lays out the shipped install's shape, with every real header in it.
    fn luba_kamila(jidhr: &Path) -> std::io::Result<()> {
        iktub(jidhr, "Bin32/bio4.exe", &pe_bi_mawarid(&min_sitteen(MAWARID)))?;
        iktub(jidhr, "BIO4/Font/common_j.fnt.lfs", &mamdud(&TARWISAT_LFS, TUL_LFS))?;
        iktub(jidhr, "BIO4/text/ENGLISH_WIN32.dct", &mamdud(&TARWISAT_DICT, TUL_DICT))?;
        iktub(jidhr, "BIO4/Etc/moji8.tpl", &mamdud(&TARWISAT_TPL_KABIR, TUL_TPL))?;
        iktub(jidhr, "BIO4/Etc/core.udas", &mamdud(&TARWISAT_UDAS, 4096))?;
        iktub(jidhr, "BIO4/iww/HomeButton2/homeBtn.arc", &mamdud(&TARWISAT_ARC, 4096))?;
        fs::create_dir_all(jidhr.join("BIO4/St0"))?;
        fs::create_dir_all(jidhr.join("BIO4/ImagePack"))?;
        fs::create_dir_all(jidhr.join("BIO4/ImagePackHD"))?;
        Ok(())
    }

    /// Runs the detector over a directory.
    fn ifhas(jidhr: &Path) -> HasilatFahs {
        let beea = BeeatTawafuq::Asli;
        let siyaq = SiyaqFahs {
            jidhr,
            tanfidhi: None,
            ism: "Resident Evil 4",
            nizam: NizamTashghil::Windows,
            beea: &beea,
        };
        FahisBio4::jadeed().ifhas(&siyaq).unwrap_or_else(|_| HasilatFahs::la_shay())
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
    // The whole install
    // -----------------------------------------------------------------------

    /// The shipped shape, with every container header in it, answers this
    /// family at the strength of its strongest signature and reads the year out
    /// of the executable.
    ///
    /// The numbers here are the ones the real install produced: family `Bio4`,
    /// the `RDLX` signature at 96, and a derived version of 2014 from the
    /// copyright rather than `1.0.0` from the file version.
    #[test]
    fn luba_haqiqiya_tujeeb_bi_bio4() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        luba_kamila(masrah.path())?;
        let hasila = ifhas(masrah.path());

        assert_eq!(hasila.aila, Some(AilatMuharrik::Bio4));
        assert_eq!(hasila.aqwa(), WAZN_SIHR_LFS, "the RDLX header is the strongest signal");
        assert_eq!(hasila.mimariya, Some(Mimariya::X86));

        let isdar = hasila.isdar.unwrap_or_else(|| IsdarMuharrik {
            kabir: 0,
            sagheer: 0,
            tasheeh: 0,
            khaam: String::new(),
            mushtaqq: false,
        });
        assert_eq!(isdar.kabir, 2014, "the last year in the copyright is the build's year");
        assert_eq!(isdar.khaam, "2014");
        assert!(isdar.mushtaqq, "a year is derived, and must never outrank a version read");
        Ok(())
    }

    /// Every one of the six observations the module exists to make is present,
    /// each at its own weight.
    ///
    /// Written as an exhaustive list rather than a spot check, because the
    /// layering *is* the design: a signature at 96, a dictionary at 94, the
    /// engine naming itself at 90, a texture header at 90, a second container at
    /// 85, the directory shape at 78, and the archive note that says what this
    /// engine is not.
    #[test]
    fn kul_daleel_bi_waznihi() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        luba_kamila(masrah.path())?;
        let hasila = ifhas(masrah.path());

        for (ibara, wazn) in [
            ("`RDLX` compression container", WAZN_SIHR_LFS),
            ("`DICT` text dictionary", WAZN_TARWISAT_DICT),
            ("`InternalName` as `BIO4`", WAZN_ISM_DAKHILI),
            ("GameCube texture container", WAZN_TARWISAT_TPL),
            ("container magic 0x20BEB6CA", WAZN_SIHR_UDAS),
            ("engine data directory", WAZN_BINYA_KAMILA),
            ("Nintendo's U8 archive", WAZN_ARSHIF_U8),
        ] {
            assert_eq!(wazn_daleel(&hasila, ibara), Some(wazn), "missing or misweighted: {ibara}");
        }
        Ok(())
    }

    /// The file version is recorded and is never turned into an engine version.
    ///
    /// `1.0.0RELEASE_DEV.0` parses perfectly well as `1.0.0`, which is exactly
    /// why this is pinned: the failure it guards against is a later reader
    /// deciding the string is a version after all.
    #[test]
    fn isdar_almalaf_yusajjal_wala_yufassar() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        luba_kamila(masrah.path())?;
        let hasila = ifhas(masrah.path());

        assert_eq!(wazn_daleel(&hasila, "1.0.0RELEASE_DEV.0"), Some(0));
        assert_eq!(hasila.isdar.as_ref().map(|isdar| isdar.kabir), Some(2014));
        Ok(())
    }

    /// Replaces the first occurrence of `sabiq` with an equally long `badil`.
    fn ibdal(kawm: &[u8], sabiq: &[u8], badil: &[u8]) -> Vec<u8> {
        let Some(mawqi) = kawm.windows(sabiq.len()).position(|nafidha| nafidha == sabiq) else {
            return kawm.to_vec();
        };
        let mut kharij = kawm.get(..mawqi).unwrap_or_default().to_vec();
        kharij.extend_from_slice(badil);
        kharij.extend_from_slice(kawm.get(mawqi.saturating_add(sabiq.len())..).unwrap_or_default());
        kharij
    }

    /// A Capcom copyright is not an identification and never yields a year on
    /// its own.
    ///
    /// Capcom shipped Resident Evil 5 on MT Framework and Resident Evil 2 on the
    /// RE Engine, and every one of those binaries carries the same company in
    /// the same field. So the fixture here is the real resource block with the
    /// internal name changed and nothing else: the copyright is still recorded,
    /// the family is not claimed from it, and no year is taken.
    #[test]
    fn huquq_capcom_wahdaha_la_tusammi_wala_tuarrikh() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let ism_akhar: Vec<u8> = "BIO4".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let badil: Vec<u8> = "MT00".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mawarid = ibdal(&min_sitteen(MAWARID), &ism_akhar, &badil);
        iktub(masrah.path(), "Bin32/bio4.exe", &pe_bi_mawarid(&mawarid))?;
        fs::create_dir_all(masrah.path().join("BIO4/text"))?;
        fs::create_dir_all(masrah.path().join("BIO4/Etc"))?;
        let hasila = ifhas(masrah.path());

        assert!(wazn_daleel(&hasila, "`InternalName` as `BIO4`").is_none());
        assert_eq!(wazn_daleel(&hasila, "names the publisher and not the engine"), Some(70));
        assert!(hasila.isdar.is_none(), "a year was taken off a binary that never named itself");
        Ok(())
    }

    /// The detector names the executable the layout rule proves, so that the
    /// probe's second pass hands it to the binary reader.
    #[test]
    fn yusammi_altanfidhi_min_alqaida() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        luba_kamila(masrah.path())?;
        let hasila = ifhas(masrah.path());

        let tanfidhi = hasila.tanfidhi.unwrap_or_default();
        assert!(tanfidhi.ends_with("Bin32/bio4.exe"), "{}", tanfidhi.display());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Degradation
    // -----------------------------------------------------------------------

    /// A directory named after the project and nothing recognised inside it is
    /// a name, and the weight says so.
    #[test]
    fn alism_wahdahu_yanzil_ila_wazn_alism() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        fs::create_dir_all(masrah.path().join("BIO4/somethingelse"))?;
        let hasila = ifhas(masrah.path());

        assert_eq!(hasila.aila, Some(AilatMuharrik::Bio4));
        assert_eq!(hasila.aqwa(), WAZN_ISM_MUJALLAD);
        assert!(hasila.isdar.is_none(), "no executable, so no year and no version");
        Ok(())
    }

    /// The interior shape without a readable container is stronger than the
    /// name and weaker than any magic.
    #[test]
    fn albinya_bila_hawiya_bayna_alwaznayn() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        fs::create_dir_all(masrah.path().join("BIO4/text"))?;
        fs::create_dir_all(masrah.path().join("BIO4/Etc"))?;
        let hasila = ifhas(masrah.path());

        assert_eq!(hasila.aqwa(), WAZN_BINYA_KAMILA);
        // The ordering itself, so that a later edit to one weight cannot
        // silently make a directory name worth as much as a magic number.
        const {
            assert!(WAZN_ISM_MUJALLAD < WAZN_BINYA_KAMILA);
            assert!(WAZN_BINYA_KAMILA < WAZN_TARWISAT_TPL);
            assert!(WAZN_TARWISAT_TPL < WAZN_TARWISAT_DICT);
            assert!(WAZN_TARWISAT_DICT < WAZN_SIHR_LFS);
        }
        Ok(())
    }

    /// A container header on its own, with no directory named `BIO4` anywhere,
    /// still identifies the family — the gate lets a flatter layout through.
    #[test]
    fn hawiya_wahdaha_takfi() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        iktub(masrah.path(), "data.lfs", &mamdud(&TARWISAT_LFS, TUL_LFS))?;
        let hasila = ifhas(masrah.path());

        assert_eq!(hasila.aila, Some(AilatMuharrik::Bio4));
        assert_eq!(hasila.aqwa(), WAZN_SIHR_LFS);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Refusals — the extension is never enough
    // -----------------------------------------------------------------------

    /// A `.tpl` that is a template rather than a texture.
    ///
    /// The magic `0x12345678` is a placeholder constant, so the guard is the
    /// two table fields behind it. Here they are wrong, and the file is stepped
    /// over rather than believed.
    #[test]
    fn tpl_bila_bunya_yurfad() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let mut kadhib = TARWISAT_TPL_KABIR;
        if let Some(bayt) = kadhib.get_mut(8) {
            *bayt = 0xFF;
        }
        iktub(masrah.path(), "BIO4/Etc/qalib.tpl", &mamdud(&kadhib, TUL_TPL))?;
        fs::create_dir_all(masrah.path().join("BIO4/text"))?;
        fs::create_dir_all(masrah.path().join("BIO4/Etc"))?;
        let hasila = ifhas(masrah.path());

        assert_eq!(hasila.aqwa(), WAZN_BINYA_KAMILA, "a bad TPL must not answer at 90");
        assert!(wazn_daleel(&hasila, "GameCube texture container").is_none());
        Ok(())
    }

    /// A dictionary whose version is not the one the game's loader accepts.
    #[test]
    fn dict_bi_isdar_akhar_yurfad() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let mut kadhib = TARWISAT_DICT;
        if let Some(bayt) = kadhib.get_mut(4) {
            *bayt = 0x01;
        }
        iktub(masrah.path(), "BIO4/text/x.dct", &mamdud(&kadhib, TUL_DICT))?;
        let hasila = ifhas(masrah.path());

        assert!(wazn_daleel(&hasila, "`DICT` text dictionary").is_none());
        Ok(())
    }

    /// A dictionary whose bucket array runs off the end of the file.
    #[test]
    fn dict_bi_jadwal_kharij_almalaf_yurfad() {
        let mut kadhib = TARWISAT_DICT;
        // 403 buckets need 3 244 bytes behind a 20-byte header.
        assert!(tarwisat_dict(&kadhib, u64::try_from(TUL_DICT).unwrap_or(0)).is_some());
        assert!(tarwisat_dict(&kadhib, 3_000).is_none());
        for mawdi in 16..20 {
            if let Some(bayt) = kadhib.get_mut(mawdi) {
                *bayt = 0x00;
            }
        }
        assert!(tarwisat_dict(&kadhib, 10_824).is_none(), "a zero bucket count is not a header");
    }

    /// An `.lfs` with the right tag and the wrong sentinel.
    ///
    /// The pair is the signature; either half alone is four bytes of coincidence.
    #[test]
    fn lfs_bila_shahid_yurfad() {
        assert!(tarwisat_lfs(&TARWISAT_LFS, u64::try_from(TUL_LFS).unwrap_or(0)).is_some());
        let mut kadhib = TARWISAT_LFS;
        if let Some(bayt) = kadhib.get_mut(4) {
            *bayt = 0x00;
        }
        assert!(tarwisat_lfs(&kadhib, 716).is_none());
        // A payload larger than the file that carries it is not a header either.
        assert!(tarwisat_lfs(&TARWISAT_LFS, 8).is_none());
    }

    /// Both byte orders of the two swapped magics are accepted, and a third
    /// value is not.
    #[test]
    fn alsihr_yuqbal_fi_alittijahayn() {
        assert_eq!(sihr_fi_ittijahayn(&TARWISAT_TPL_KABIR, SIHR_TPL), Some(true));
        assert_eq!(sihr_fi_ittijahayn(&TARWISAT_TPL_SAGHEER, SIHR_TPL), Some(false));
        assert_eq!(sihr_fi_ittijahayn(&TARWISAT_UDAS, SIHR_UDAS), Some(true));
        assert_eq!(sihr_fi_ittijahayn(&TARWISAT_UDAS_SAGHEER, SIHR_UDAS), Some(false));
        assert_eq!(sihr_fi_ittijahayn(&[0, 1, 2, 3], SIHR_TPL), None);
        assert!(tarwisat_tpl(&TARWISAT_TPL_SAGHEER).is_some());
    }

    /// The year comes off the end of a copyright line and refuses anything that
    /// is not a plausible year.
    #[test]
    fn sanat_alhuquq_hiya_alakhira() {
        assert_eq!(
            sanat_akhira("\u{a9}CAPCOM CO., LTD. 2005, 2014 ALL RIGHTS RESERVED."),
            Some(2014)
        );
        assert_eq!(sanat_akhira("\u{a9}CAPCOM CO., LTD. 2005"), Some(2005));
        assert_eq!(sanat_akhira("1.0.0RELEASE_DEV.0"), None);
        assert_eq!(sanat_akhira("build 12345"), None);
        assert_eq!(sanat_akhira("no digits here"), None);
    }

    // -----------------------------------------------------------------------
    // The engines in the same library
    // -----------------------------------------------------------------------

    /// A Unity game answers nothing at all — not a weak claim, nothing.
    ///
    /// The shape here is the one on the machine this was written on: a
    /// `*_Data` directory, the player library beside it and a Mono runtime.
    #[test]
    fn luba_unity_la_tujeeb() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        iktub(masrah.path(), "hollow_knight.exe", b"MZ")?;
        iktub(masrah.path(), "UnityPlayer.dll", b"MZ")?;
        iktub(masrah.path(), "hollow_knight_Data/globalgamemanagers", b"\0\0\0\0")?;
        iktub(masrah.path(), "hollow_knight_Data/Managed/Assembly-CSharp.dll", b"MZ")?;
        fs::create_dir_all(masrah.path().join("MonoBleedingEdge"))?;
        let hasila = ifhas(masrah.path());

        assert!(!hasila.wajad(), "a Unity game produced BIO4 evidence: {:?}", hasila.dalail);
        assert_eq!(hasila.aila, None);
        Ok(())
    }

    /// An Unreal game answers nothing either.
    #[test]
    fn luba_unreal_la_tujeeb() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        iktub(masrah.path(), "Atlas/Content/Paks/Atlas-WindowsNoEditor.pak", b"\0\0\0\0")?;
        iktub(masrah.path(), "Atlas/Binaries/Win64/LittleNightmares.exe", b"MZ")?;
        iktub(masrah.path(), "Engine/Build/Build.version", b"{}")?;
        let hasila = ifhas(masrah.path());

        assert!(!hasila.wajad(), "an Unreal game produced BIO4 evidence: {:?}", hasila.dalail);
        Ok(())
    }

    /// A game whose root holds a file called `.tpl` that is a template rather
    /// than a texture is refused at the gate, not after a directory walk.
    ///
    /// The gate runs the same header readers the detector runs, so a file whose
    /// extension is one of this engine's and whose bytes are not buys nothing at
    /// all — which is the difference between a detector that costs ten
    /// milliseconds on somebody's whole library and one that does not.
    #[test]
    fn imtidad_mustaar_la_yaftah_albawwaba() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        iktub(masrah.path(), "layout.tpl", b"<html>{$title}</html>")?;
        iktub(masrah.path(), "strings.dct", b"# a spell-checker dictionary\nhello\n")?;
        fs::create_dir_all(masrah.path().join("Content"))?;
        let hasila = ifhas(masrah.path());

        assert!(!hasila.wajad(), "a borrowed extension opened the gate: {:?}", hasila.dalail);
        Ok(())
    }

    /// An empty directory is not this engine, and a missing one is an error
    /// rather than an answer.
    #[test]
    fn faragh_wa_ghiyab() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        assert!(!ifhas(masrah.path()).wajad());

        let beea = BeeatTawafuq::Asli;
        let mafqud = masrah.path().join("la-shay");
        let siyaq = SiyaqFahs {
            jidhr: &mafqud,
            tanfidhi: None,
            ism: "",
            nizam: NizamTashghil::Windows,
            beea: &beea,
        };
        assert!(FahisBio4::jadeed().ifhas(&siyaq).is_err());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Through resolution and into the report
    // -----------------------------------------------------------------------

    /// The whole path: evidence, resolution, and the report a player reads.
    ///
    /// Pinned end to end rather than at the detector, because the value of a new
    /// engine family is what comes out of `imkaniyat`, and the two numbers that
    /// decide what a user is shown are the tier and the readiness verdict.
    #[test]
    fn min_aladilla_ila_altaqreer() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        luba_kamila(masrah.path())?;

        let mut jami = crate::fahs::JamiHasilat::default();
        jami.hasilat.push(("bio4", ifhas(masrah.path())));
        let muharrik = tahdid::hall(&jami);

        assert_eq!(muharrik.aila, AilatMuharrik::Bio4);
        assert_eq!(muharrik.aila.ism(), "Capcom BIO4");
        assert!(muharrik.thiqa >= WAZN_SIHR_LFS, "confidence {} below the base", muharrik.thiqa);
        assert_eq!(muharrik.mimariya, Mimariya::X86);
        assert_eq!(
            muharrik.khalfiya,
            taarib_mustalahat::muharrik::KhalfiyaBarmajiya::Majhula,
            "compiled C++ has no value in this vocabulary and must not borrow one"
        );

        let taqreer = crate::imkaniyat::taqreer(muharrik, &[], "2026-09-05T00:00:00Z".to_owned());
        assert_eq!(taqreer.tabaqa, taarib_mustalahat::muharrik::Tabaqa::Kamil);
        assert_eq!(
            taqreer.jahiziya,
            taarib_mustalahat::muharrik::JahiziyatTashghil::Ghaiba,
            "nothing in this build puts Arabic on this engine's screen"
        );
        assert!(taqreer.naqs.is_some(), "an unfinished tier names what is missing");
        assert!(taqreer.anzimat_qabila.is_empty(), "nothing is taken over while nothing runs");
        assert_eq!(taqreer.isdar_fahs, crate::imkaniyat::ISDAR_FAHS);
        Ok(())
    }
}
