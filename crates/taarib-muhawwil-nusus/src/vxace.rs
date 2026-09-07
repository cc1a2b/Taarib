//! في إكس أيس — RPG Maker VX Ace: one obfuscated archive, a directory of Ruby
//! `Marshal` streams, and a script list that is itself a data structure.
//!
//! Every other engine in this crate stores its content as text or as JSON. VX
//! Ace stores it as **serialized Ruby objects**, which changes the shape of the
//! whole problem: there is no field to edit in place, no offset to patch, and no
//! way to change one string without re-serializing the graph it lives in. That
//! is why the largest part of this module is a `Marshal` reader and writer, and
//! why the two are held to a round-trip check before anything is written.
//!
//! ## The link table, which is the trap
//!
//! `Marshal` back-references are positional. Every object the writer emits —
//! every string, array, hash and object, but not a `Fixnum`, `nil`, `true`,
//! `false` or `Symbol` — is appended to a table as it is written, and a repeat
//! of that same object is emitted as `@<index>` into that table. Symbols get a
//! second, separate table addressed by `;<index>`.
//!
//! The consequence is the one that breaks naive patchers: **inserting or
//! resizing an object renumbers every later reference.** A patcher that found a
//! string's bytes in the stream and replaced them with longer ones would leave
//! every `@` after it pointing one object short of where it meant to, and the
//! result loads without error and hands the game the wrong object.
//!
//! ### The strategy here
//!
//! **Do not renumber. Do not carry indices at all.**
//!
//! The reader resolves every `@` and `;` into a handle in an arena
//! ([`Muashir`]), so the in-memory model has no indices in it — only identity.
//! Two occurrences that were one object in the stream are one node afterwards,
//! and a cyclic graph is representable rather than fatal.
//!
//! The writer then re-derives every index from the emission order it actually
//! produces, keeping its own identity map from node to freshly assigned index.
//! Because the indices are a function of the output rather than an input, there
//! is nothing to renumber: a string that grew by four bytes changes no index,
//! and an object inserted in the middle simply takes the next number and pushes
//! nothing.
//!
//! Two obligations follow, and both are enforced rather than promised:
//!
//! 1. **Identity must survive.** If the reader collapsed two references into
//!    two nodes, the writer would emit two objects where the engine expects
//!    one, and a game that mutates a shared `RPG::Item` would see the mutation
//!    on one of them. The arena is what makes that structural.
//! 2. **An unedited round trip must be byte-identical.** [`dawra_mutabaqa`]
//!    re-serializes what was just read and compares. It is the only honest
//!    check available for a format this crate reconstructs rather than edits,
//!    and a mismatch raises [`KhataNusus::DawraGhayrMutabaqa`] *before* the
//!    game's directory is touched. A difference means the reader dropped
//!    something it did not understand, and refusing is right.
//!
//! ## The archive
//!
//! `Game.rgss3a` is RGSSAD version 3. Its file table is obfuscated with a key
//! derived from a seed in its own header, and each entry's data is enciphered
//! with a per-entry key that advances. Both are implemented in full below; see
//! [`Hawiya`] for the derivation and the rotation, and note that the two differ
//! — the table key is fixed and the data key rotates, which is exactly the
//! change version 3 made from version 1 and the reason a reader ported from an
//! `.rgssad` produces plausible-looking garbage rather than failing.
//!
//! ## Delivery
//!
//! One Ruby script appended to `Scripts.rvdata2`, and nothing else. Additive,
//! removable by deleting that one entry, and it carries no evaluated content:
//! the script reads its settings from a data file and never calls `eval` on
//! anything the patch supplies.
//!
//! ## The rung comes first
//!
//! VX Ace draws through a platform bitmap API, and that API is not the same
//! everywhere the game runs. The official RGSS3 runtime reaches GDI, which does
//! not shape; an mkxp-z port of the same game reaches `FreeType` through `SDL_ttf`
//! and does. So this module takes nothing over until [`MifhasVxAce`] says which
//! runtime will actually load, and the evidence for that is the shipped files
//! rather than the engine's reputation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use taarib_muharrik::dalail::nusus::HadafNusus;

use crate::hifz::Hafiz;
use crate::khata::{KhataNusus, hajm_usize, tul_u64};
use crate::tabaqa::{Dalil, Mifhas, Rutba, SiyaqTabaqa};
use crate::tarkeeb::masar_bila_hala;

// ---------------------------------------------------------------------------
// Ceilings
// ---------------------------------------------------------------------------

/// How many entries the archive's file table may declare.
///
/// A large VX Ace game packs a few thousand assets. Sixty-four thousand is far
/// past that and small enough that a corrupt table cannot be walked forever.
pub const AQSA_MADAKHIL: usize = 65_536;

/// The largest single member this reader will extract from the archive.
pub const AQSA_UDW: u64 = 512 * 1024 * 1024;

/// The largest `Marshal` stream this reader will decode.
pub const AQSA_SILSILA: u64 = 256 * 1024 * 1024;

/// The largest script source this reader will expand from `Scripts.rvdata2`.
pub const AQSA_NASS_RUBY: u64 = 16 * 1024 * 1024;

/// How deep a `Marshal` stream may nest before the reader refuses.
///
/// A VX Ace map with a hundred events nests about a dozen levels. Two hundred
/// is far past that and shallow enough that the reader cannot exhaust the
/// native stack on a hostile stream.
pub const AQSA_UMQ: u32 = 200;

/// How many objects one `Marshal` stream may create.
pub const AQSA_KAINAT: usize = 8_000_000;

/// The archive Phase 10 reads and writes.
pub const ISM_HAWIYA: &str = "Game.rgss3a";

/// The name this format uses in a refusal.
const SIGHA_RGSS: &str = "RGSSAD v3";

/// The name the serializer uses in a refusal.
const SIGHA_SILSILA: &str = "Ruby Marshal 4.8";

/// The bytes an RGSSAD archive of any generation begins with.
pub const SIHR: [u8; 7] = *b"RGSSAD\0";

/// The only archive version this module reads or writes.
///
/// Versions one and two are RPG Maker XP and VX, whose runtimes, script APIs
/// and data formats are different engines wearing a similar name. Taarib has no
/// adapter for either, and reading their archives here would produce content
/// nothing downstream could use.
pub const ISDAR_HAWIYA: u8 = 3;

/// The two bytes every `Marshal` stream this engine writes begins with.
pub const ISDAR_SILSILA: [u8; 2] = [4, 8];

// ---------------------------------------------------------------------------
// The tier probe
//
// The question is not "does RPG Maker VX Ace shape Arabic". It is "which text
// stack will this copy of this game actually reach", and the answer is a
// property of the files beside it rather than of the product name. A stock
// export loads RGSS3 and draws through the platform's plain text call, which
// maps characters to glyphs through the font's cmap and applies no OpenType
// layout: Arabic comes out in isolated forms, unjoined. The same project
// shipped on an mkxp-z runtime draws through FreeType, and whether *that*
// joins depends on whether the port shipped HarfBuzz.
//
// So the probe establishes the runtime first and reports the other one as
// context. That is not a tiebreak dressed up as evidence: a directory holding
// both runtimes is a directory where the launcher decides, and the launcher is
// what the probe reads.
// ---------------------------------------------------------------------------

/// Files that mean the game runs on an mkxp or mkxp-z runtime.
const ALAMAT_MKXP: [&str; 6] = [
    "mkxp.json",
    "mkxp-z.json",
    "mkxp.conf",
    "mkxp-z.exe",
    "libmkxp-z.so",
    "mkxp-z",
];

/// The RGSS3 runtime libraries the official player ships.
const ALAMAT_RGSS3: [&str; 4] = ["rgss300.dll", "rgss301.dll", "rgss302.dll", "rgss3.dll"];

/// Substrings that identify a shipped `HarfBuzz` shared library.
const QITA_HARFBUZZ: [&str; 2] = ["libharfbuzz", "harfbuzz."];

/// Substrings that identify a shipped `FreeType` shared library, which mkxp-z
/// links and the official runtime does not.
const QITA_FREETYPE: [&str; 3] = ["libfreetype", "freetype.", "sdl2_ttf"];

/// How many entries the probe will look at in any one directory.
const AQSA_MADAKHIL_MUJALLAD: usize = 4_096;

/// Which text stack a game will actually load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Muharrik {
    /// The official RGSS3 runtime.
    Rasmi,
    /// An mkxp or mkxp-z reimplementation.
    Badeel,
    /// Neither could be identified from the shipped files.
    Majhul,
}

/// The RPG Maker VX Ace tier probe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MifhasVxAce;

impl MifhasVxAce {
    /// A probe for VX Ace.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

/// One directory's file names, lowercased, bounded, and never recursive.
fn asmaa_mujallad(masar: &Path) -> Vec<String> {
    let Ok(qaima) = fs::read_dir(masar) else {
        return Vec::new();
    };
    qaima
        .take(AQSA_MADAKHIL_MUJALLAD)
        .flatten()
        .filter_map(|madkhal| {
            let ism = madkhal.file_name().into_string().ok()?;
            madkhal
                .file_type()
                .is_ok_and(|naw| naw.is_file())
                .then(|| ism.to_lowercase())
        })
        .collect()
}

/// Reads at most `hadd` bytes from the start of a file.
fn iqra_muhaddad(masar: &Path, hadd: u64) -> Option<Vec<u8>> {
    let malaf = fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(hadd).read_to_end(&mut bayt).ok()?;
    Some(bayt)
}

/// One key out of `Game.ini`, matched case-insensitively.
///
/// Not an INI parser, deliberately: the file has one section and four keys, is
/// written by a Japanese editor in whatever the system code page was, and a
/// strict parser would reject files the engine itself reads happily.
fn qeemat_ini(nass: &str, miftah: &str) -> Option<String> {
    for satr in nass.lines() {
        let munaqqa = satr.trim();
        if munaqqa.starts_with(';') || munaqqa.starts_with('#') || munaqqa.starts_with('[') {
            continue;
        }
        let (ism, qeema) = munaqqa.split_once('=')?;
        if ism.trim().eq_ignore_ascii_case(miftah) {
            let munaqqah = qeema.trim().trim_matches('"').trim();
            if !munaqqah.is_empty() {
                return Some(munaqqah.to_owned());
            }
        }
    }
    None
}

/// The directories the probe will look in for a runtime file.
///
/// Two names rather than the four spellings this used to list: the resolver
/// answers for every casing a depot might carry, including the ones nobody
/// thought to enumerate.
fn amakin(jidhr: &Path) -> Vec<PathBuf> {
    let mut qaima = vec![jidhr.to_path_buf()];
    for far in ["System", "lib"] {
        let masar = masar_bila_hala(jidhr, far);
        if masar.is_dir() {
            qaima.push(masar);
        }
    }
    qaima
}

/// The first file in any of `amakin` whose lowercased name matches.
fn ibhath(amakin: &[PathBuf], jidhr: &Path, mutabiq: impl Fn(&str) -> bool) -> Option<String> {
    for makan in amakin {
        for ism in asmaa_mujallad(makan) {
            if mutabiq(&ism) {
                let nisbi = makan.strip_prefix(jidhr).unwrap_or(makan).to_string_lossy();
                return Some(if nisbi.is_empty() {
                    ism
                } else {
                    format!("{}/{ism}", nisbi.replace('\\', "/"))
                });
            }
        }
    }
    None
}

impl Mifhas for MifhasVxAce {
    fn hadaf(&self) -> HadafNusus {
        HadafNusus::VxAce
    }

    fn yantabiq(&self, siyaq: &SiyaqTabaqa<'_>) -> bool {
        // The same gate Phase 5 uses: the version 3 archive, or a `Data`
        // directory holding this engine's own serialization extension. A
        // `Game.ini` alone is not enough — every RPG Maker generation has one.
        masar_bila_hala(siyaq.jidhr, ISM_HAWIYA).is_file()
            || masar_bila_hala(siyaq.jidhr, "Data/System.rvdata2").is_file()
            || masar_bila_hala(siyaq.jidhr, "Data/Scripts.rvdata2").is_file()
    }

    fn adilla(&self, siyaq: &SiyaqTabaqa<'_>) -> Result<Vec<Dalil>, KhataNusus> {
        let jidhr = siyaq.jidhr;
        let mut adilla: Vec<Dalil> = Vec::new();
        let makan = amakin(jidhr);

        let mkxp = ibhath(&makan, jidhr, |ism| ALAMAT_MKXP.contains(&ism));
        let rasmi = ibhath(&makan, jidhr, |ism| ALAMAT_RGSS3.contains(&ism));
        let maktaba = ibhath(&makan, jidhr, |ism| {
            QITA_HARFBUZZ.iter().any(|juz| ism.contains(juz))
        });
        let freetype = ibhath(&makan, jidhr, |ism| {
            QITA_FREETYPE.iter().any(|juz| ism.contains(juz))
        });

        let muharrik = match (mkxp.as_ref(), rasmi.as_ref()) {
            (Some(_), _) => Muharrik::Badeel,
            (None, Some(_)) => Muharrik::Rasmi,
            (None, None) => Muharrik::Majhul,
        };

        match muharrik {
            Muharrik::Badeel => {
                let ism = mkxp.unwrap_or_else(|| "an mkxp runtime".to_owned());
                match maktaba.as_ref() {
                    Some(hb) => adilla.push(Dalil::jadeed(
                        ism,
                        format!(
                            "this game runs on an mkxp runtime and ships {hb}, so its text \
                             goes through FreeType with HarfBuzz and joins; what it does not \
                             do is lay the paragraph out right to left"
                        ),
                        Some(Rutba::Tasheeh),
                        WAZN_MKXP_MUSHAKKIL,
                    )),
                    None => adilla.push(Dalil::jadeed(
                        ism,
                        format!(
                            "this game runs on an mkxp runtime and ships no HarfBuzz library \
                             ({}), so its FreeType path positions one glyph per character \
                             with no joining",
                            freetype.unwrap_or_else(|| "no FreeType either".to_owned())
                        ),
                        Some(Rutba::Istila),
                        WAZN_MKXP_BILA,
                    )),
                }
                if let Some(ism) = rasmi.as_ref() {
                    adilla.push(Dalil::siyaq(
                        ism.clone(),
                        "the official RGSS3 runtime also ships here; the mkxp launcher is \
                         what loads, so the RGSS3 library is not what draws the text",
                    ));
                }
            },
            Muharrik::Rasmi => {
                let ism = rasmi.unwrap_or_else(|| "RGSS3".to_owned());
                adilla.push(Dalil::jadeed(
                    ism,
                    "the official RGSS3 runtime draws through the platform's plain text \
                     call, which maps characters to glyphs through the font's cmap and \
                     applies no OpenType layout: Arabic comes out in isolated forms",
                    Some(Rutba::Istila),
                    WAZN_RGSS3,
                ));
            },
            Muharrik::Majhul => adilla.push(Dalil::siyaq(
                "runtime",
                "no runtime library was found beside the game, so which text stack will \
                 load could not be established from the shipped files",
            )),
        }

        adilla.extend(adillat_ini(jidhr, muharrik));
        adilla.extend(adillat_bayanat(jidhr));
        if let Some(tanfidhi) = siyaq.tanfidhi {
            adilla.push(Dalil::siyaq(
                "executable",
                tanfidhi
                    .strip_prefix(jidhr)
                    .unwrap_or(tanfidhi)
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        adilla.push(Dalil::siyaq("engine family", siyaq.aila.ism()));
        Ok(adilla)
    }
}

/// What `Game.ini` says, which is the runtime's own statement of what it loads.
///
/// Worth its own weight only when the official player is what will run: on an
/// mkxp port the `Library=` line is left in place and ignored, so believing it
/// there would be believing a line nothing reads.
fn adillat_ini(jidhr: &Path, muharrik: Muharrik) -> Vec<Dalil> {
    let mut adilla = Vec::new();
    let masar_ini = masar_bila_hala(jidhr, "Game.ini");
    let Some(bayt) = masar_ini
        .is_file()
        .then(|| iqra_muhaddad(&masar_ini, 256 * 1024))
        .flatten()
    else {
        return adilla;
    };
    let nass = String::from_utf8_lossy(&bayt);
    if let Some(maktaba) = qeemat_ini(&nass, "Library") {
        let ism = maktaba
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(&maktaba)
            .to_owned();
        let kabira = ism.to_ascii_uppercase();
        if kabira.starts_with("RGSS3") && muharrik == Muharrik::Rasmi {
            adilla.push(Dalil::jadeed(
                "Game.ini",
                format!("Library={ism}, which is the RGSS3 runtime this game loads"),
                Some(Rutba::Istila),
                WAZN_INI,
            ));
        } else {
            adilla.push(Dalil::siyaq("Game.ini", format!("Library={ism}")));
        }
    }
    if let Some(nusus) = qeemat_ini(&nass, "Scripts") {
        adilla.push(Dalil::siyaq("Game.ini", format!("Scripts={nusus}")));
    }
    adilla
}

/// What the game's own data directory looks like. Context, never a verdict.
fn adillat_bayanat(jidhr: &Path) -> Vec<Dalil> {
    let mut adilla = Vec::new();
    if masar_bila_hala(jidhr, ISM_HAWIYA).is_file() {
        adilla.push(Dalil::siyaq(
            ISM_HAWIYA,
            "the game's assets are inside the version 3 archive rather than loose",
        ));
    }
    let bayanat = masar_bila_hala(jidhr, "Data");
    let adad = asmaa_mujallad(&bayanat)
        .iter()
        .filter(|ism| ism.ends_with(".rvdata2"))
        .count();
    if adad > 0 {
        adilla.push(Dalil::siyaq("Data", format!("{adad} loose .rvdata2 files")));
    }
    adilla
}

/// Weight of an mkxp runtime that also ships a shaper.
///
/// High, because the two facts together are near-conclusive: the runtime says
/// which text stack loads and the library says whether that stack can join.
const WAZN_MKXP_MUSHAKKIL: u8 = 85;

/// Weight of an mkxp runtime with no shaper beside it.
const WAZN_MKXP_BILA: u8 = 80;

/// Weight of the official RGSS3 runtime.
///
/// Conclusive about the drawing path, because that runtime has exactly one and
/// it has not changed since 2011.
const WAZN_RGSS3: u8 = 75;

/// Weight of `Game.ini` naming the RGSS3 library, on top of finding it.
const WAZN_INI: u8 = 30;

// ---------------------------------------------------------------------------
// Game.rgss3a — the RGSSAD version 3 archive
//
//   offset  size  meaning
//        0     6  the ASCII bytes RGSSAD
//        6     1  a zero byte
//        7     1  the archive version: 3
//        8     4  the key seed, little-endian
//       12   ...  the file table, then the file data
//
// Two different key schedules live in this format and confusing them is the
// classic way to get it wrong.
//
// **The table key is derived once and never advances.** It is
// `badhra * 9 + 3`, computed with wrapping arithmetic over 32 bits, and every
// field of every table entry is XORed with that one value. Version 1 advanced
// its key after each field; version 3 stopped, which is what makes the table
// randomly addressable, and a reader ported from an `.rgssad` walks it happily
// and produces enormous offsets rather than an error.
//
// **The data key rotates.** Each entry carries its own starting key in the
// table, and the member's bytes are enciphered four at a time as little-endian
// words: the word is XORed with the current key, and the key then becomes
// `miftah * 7 + 3`. A trailing group shorter than four bytes uses the low bytes
// of the key it reached.
//
// Each table entry is:
//
//   u32 offset       ^ table key
//   u32 size         ^ table key
//   u32 entry key    ^ table key
//   u32 name length  ^ table key
//   name bytes, byte i XORed with byte (i and 3) of the table key
//
// and the table ends at the first entry whose decoded offset is zero.
//
// None of this is encryption and this module does not treat it as such: the
// seed that produces the key is in the header of the same file. It is
// obfuscation, the game publishes its own key, and Taarib reads the key the
// game publishes and derives none that it does not.
// ---------------------------------------------------------------------------

/// The table key an archive derives from its seed.
///
/// Wrapping rather than checked: the multiply overflows 32 bits for most seeds
/// and the overflow is the specification. A checked version of this function
/// would refuse almost every real archive.
#[must_use]
pub const fn miftah_min_badhra(badhra: u32) -> u32 {
    badhra.wrapping_mul(9).wrapping_add(3)
}

/// Advances a data key by one four-byte group.
///
/// The other half of the format's arithmetic, and deliberately a separate
/// function from [`miftah_min_badhra`]: the two constants differ, they apply to
/// different things, and writing them as one parametrised helper would make the
/// next reader assume they are the same schedule.
#[must_use]
pub const fn miftah_taali(miftah: u32) -> u32 {
    miftah.wrapping_mul(7).wrapping_add(3)
}

/// Enciphers or deciphers a member's bytes in place.
///
/// The same function does both, because the cipher is a XOR against a key
/// stream that depends only on the starting key: applying it twice restores the
/// input, which is also the cheapest possible self-test and is what
/// [`dawra_hawiya`] uses.
pub fn ashfir(bayt: &mut [u8], miftah_bidaya: u32) {
    let mut miftah = miftah_bidaya;
    for majmua in bayt.chunks_mut(4) {
        let mafatih = miftah.to_le_bytes();
        for (khana, wahid) in majmua.iter_mut().enumerate() {
            if let Some(qina) = mafatih.get(khana) {
                *wahid ^= *qina;
            }
        }
        miftah = miftah_taali(miftah);
    }
}

/// One member of the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalRgss {
    /// The member's path inside the archive, exactly as the table spells it —
    /// backslashes and all, because that is what the engine's own `load_data`
    /// looks up and normalising it would break the lookup.
    pub masar: String,
    /// Where its enciphered bytes begin.
    pub izaha: u64,
    /// How many bytes it is.
    pub tul: u64,
    /// The starting key for its data.
    pub miftah: u32,
}

/// An opened archive: its header, its file table, and nothing else.
#[derive(Debug, Clone)]
pub struct Hawiya {
    /// Where the archive is.
    pub masar: PathBuf,
    /// The seed from its header.
    pub badhra: u32,
    /// The table key derived from that seed.
    pub miftah: u32,
    /// Where the file table ended, which is where the data begins.
    pub nihayat_fahras: u64,
    madakhil: Vec<MadkhalRgss>,
}

/// An I/O failure, carrying the path that produced it.
fn khata_malaf(masar: &Path, sabab: std::io::Error) -> KhataNusus {
    KhataNusus::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    }
}

/// A field in a container that points outside it.
const fn hawiya_talifa(haql: &'static str, qeema: u64, hadd: u64) -> KhataNusus {
    KhataNusus::HawiyaTalifa {
        sigha: SIGHA_RGSS,
        haql,
        qeema,
        hadd,
    }
}

/// A little-endian `u32` at an offset, bounds-checked.
fn u32_le(bayt: &[u8], izaha: usize) -> Option<u32> {
    let nihaya = izaha.checked_add(4)?;
    bayt.get(izaha..nihaya)
        .and_then(|juz| <[u8; 4]>::try_from(juz).ok())
        .map(u32::from_le_bytes)
}

impl Hawiya {
    /// Every member, in the order the table listed them.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalRgss] {
        &self.madakhil
    }

    /// One member by path, matched case-insensitively and slash-insensitively.
    ///
    /// The archive stores `Data\System.rvdata2`; a caller naturally writes
    /// `Data/System.rvdata2`, and the engine itself is not case sensitive about
    /// either. Matching loosely here is what keeps every call site from
    /// carrying its own normalisation.
    #[must_use]
    pub fn madkhal(&self, masar: &str) -> Option<&MadkhalRgss> {
        let matlub = masar.replace('\\', "/").to_ascii_lowercase();
        self.madakhil
            .iter()
            .find(|madkhal| madkhal.masar.replace('\\', "/").to_ascii_lowercase() == matlub)
    }

    /// Reads the header and the file table, and no member's bytes.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when the file cannot be read,
    /// [`KhataNusus::SihrGhayrMutabaq`] when it does not carry the RGSSAD
    /// signature, [`KhataNusus::IsdarGhayrMadum`] for versions one and two,
    /// [`KhataNusus::MalafQaseer`] when the table runs past the file,
    /// [`KhataNusus::HajmMufrit`] when it declares more than
    /// [`AQSA_MADAKHIL`] entries or a name longer than a path can be, and
    /// [`KhataNusus::HawiyaTalifa`] when an entry points outside the file.
    pub fn iqra(masar: &Path) -> Result<Self, KhataNusus> {
        let bayt = fs::read(masar).map_err(|sabab| khata_malaf(masar, sabab))?;
        let tul_malaf = tul_u64(bayt.len());
        if !bayt.starts_with(&SIHR) {
            return Err(KhataNusus::SihrGhayrMutabaq {
                masar: masar.to_path_buf(),
                sigha: SIGHA_RGSS,
            });
        }
        let isdar = *bayt.get(7).ok_or(KhataNusus::MalafQaseer {
            haql: "the archive version byte",
            tul: tul_malaf,
            matlub: 8,
        })?;
        if isdar != ISDAR_HAWIYA {
            return Err(KhataNusus::IsdarGhayrMadum {
                sigha: SIGHA_RGSS,
                wujid: u32::from(isdar),
                adna: u32::from(ISDAR_HAWIYA),
                aqsa: u32::from(ISDAR_HAWIYA),
            });
        }
        let badhra = u32_le(&bayt, 8).ok_or(KhataNusus::MalafQaseer {
            haql: "the archive key seed",
            tul: tul_malaf,
            matlub: 12,
        })?;
        let miftah = miftah_min_badhra(badhra);

        let (madakhil, nihayat_fahras) = Self::iqra_fahras(&bayt, miftah, tul_malaf)?;
        Ok(Self {
            masar: masar.to_path_buf(),
            badhra,
            miftah,
            nihayat_fahras,
            madakhil,
        })
    }

    /// Walks the obfuscated file table.
    fn iqra_fahras(
        bayt: &[u8],
        miftah: u32,
        tul_malaf: u64,
    ) -> Result<(Vec<MadkhalRgss>, u64), KhataNusus> {
        let mut mawqi = 12_usize;
        let mut madakhil: Vec<MadkhalRgss> = Vec::new();
        let mafatih = miftah.to_le_bytes();

        loop {
            let qaseer = |haql: &'static str, matlub: u64| KhataNusus::MalafQaseer {
                haql,
                tul: tul_malaf,
                matlub,
            };
            let izaha = u32_le(bayt, mawqi)
                .map(|khaam| khaam ^ miftah)
                .ok_or_else(|| {
                    qaseer("a table entry's offset", tul_u64(mawqi.saturating_add(4)))
                })?;
            if izaha == 0 {
                mawqi = mawqi.saturating_add(4);
                break;
            }
            let tul = u32_le(bayt, mawqi.saturating_add(4))
                .map(|khaam| khaam ^ miftah)
                .ok_or_else(|| qaseer("a table entry's size", tul_u64(mawqi.saturating_add(8))))?;
            let miftah_madkhal = u32_le(bayt, mawqi.saturating_add(8))
                .map(|khaam| khaam ^ miftah)
                .ok_or_else(|| qaseer("a table entry's key", tul_u64(mawqi.saturating_add(12))))?;
            let tul_ism = u32_le(bayt, mawqi.saturating_add(12))
                .map(|khaam| khaam ^ miftah)
                .ok_or_else(|| {
                    qaseer(
                        "a table entry's name length",
                        tul_u64(mawqi.saturating_add(16)),
                    )
                })?;
            if tul_ism > AQSA_TUL_ISM {
                return Err(KhataNusus::HajmMufrit {
                    haql: "a member's name length",
                    qeema: u64::from(tul_ism),
                    saqf: u64::from(AQSA_TUL_ISM),
                });
            }
            let bidayat_ism = mawqi.saturating_add(16);
            let tul_ism_usize = hajm_usize(u64::from(tul_ism)).unwrap_or(usize::MAX);
            let nihayat_ism = bidayat_ism.saturating_add(tul_ism_usize);
            let khaam_ism = bayt
                .get(bidayat_ism..nihayat_ism)
                .ok_or_else(|| qaseer("a table entry's name", tul_u64(nihayat_ism)))?;

            // The name is XORed byte by byte against the *cycling* bytes of the
            // one table key — not against an advancing stream. Getting this
            // wrong yields readable-looking mojibake for the first four
            // characters of every path, which is a bug that survives review.
            let mut ism = Vec::with_capacity(khaam_ism.len());
            for (khana, wahid) in khaam_ism.iter().enumerate() {
                let qina = mafatih.get(khana & 3).copied().unwrap_or(0);
                ism.push(*wahid ^ qina);
            }
            let masar = String::from_utf8(ism).map_err(|khata| KhataNusus::NassGhayrSalih {
                malaf: ISM_HAWIYA.to_owned(),
                tarmiz: "UTF-8",
                mawqi: tul_u64(bidayat_ism.saturating_add(khata.utf8_error().valid_up_to())),
            })?;

            let nihaya = u64::from(izaha)
                .checked_add(u64::from(tul))
                .ok_or_else(|| hawiya_talifa("a member's end", u64::from(izaha), tul_malaf))?;
            if nihaya > tul_malaf {
                return Err(hawiya_talifa("a member's end", nihaya, tul_malaf));
            }

            madakhil.push(MadkhalRgss {
                masar,
                izaha: u64::from(izaha),
                tul: u64::from(tul),
                miftah: miftah_madkhal,
            });
            if madakhil.len() > AQSA_MADAKHIL {
                return Err(KhataNusus::HajmMufrit {
                    haql: "the archive's file table",
                    qeema: tul_u64(madakhil.len()),
                    saqf: tul_u64(AQSA_MADAKHIL),
                });
            }
            mawqi = nihayat_ism;
        }
        Ok((madakhil, tul_u64(mawqi)))
    }

    /// Reads one member and deciphers it.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when the archive cannot be read,
    /// [`KhataNusus::HajmMufrit`] when the member declares more than
    /// [`AQSA_UDW`], and [`KhataNusus::HawiyaTalifa`] when its range leaves the
    /// file — checked against the file's real length here rather than trusted
    /// from the table, because the table came out of the same untrusted file.
    pub fn istakhrij(&self, madkhal: &MadkhalRgss) -> Result<Vec<u8>, KhataNusus> {
        use std::io::{Seek as _, SeekFrom};

        if madkhal.tul > AQSA_UDW {
            return Err(KhataNusus::HajmMufrit {
                haql: "an archive member",
                qeema: madkhal.tul,
                saqf: AQSA_UDW,
            });
        }
        let mut malaf =
            fs::File::open(&self.masar).map_err(|sabab| khata_malaf(&self.masar, sabab))?;
        let tul_malaf = malaf
            .metadata()
            .map_err(|sabab| khata_malaf(&self.masar, sabab))?
            .len();
        let nihaya = madkhal
            .izaha
            .checked_add(madkhal.tul)
            .ok_or_else(|| hawiya_talifa("a member's end", madkhal.izaha, tul_malaf))?;
        if nihaya > tul_malaf {
            return Err(hawiya_talifa("a member's end", nihaya, tul_malaf));
        }
        let _ = malaf
            .seek(SeekFrom::Start(madkhal.izaha))
            .map_err(|sabab| khata_malaf(&self.masar, sabab))?;
        let mut bayt = vec![0_u8; hajm_usize(madkhal.tul).unwrap_or(0)];
        malaf
            .read_exact(&mut bayt)
            .map_err(|sabab| khata_malaf(&self.masar, sabab))?;
        ashfir(&mut bayt, madkhal.miftah);
        Ok(bayt)
    }
}

/// The longest member name this reader accepts.
///
/// Four kilobytes, which is above every filesystem's own path limit and far
/// below the point where a corrupt length field could reserve anything
/// interesting.
pub const AQSA_TUL_ISM: u32 = 4096;

/// Where one member of a rebuilt archive comes from.
///
/// A rebuild names every member, and almost all of them are unchanged. Holding
/// the bytes of an unchanged member would mean holding the whole archive in
/// memory to replace four files in it, so an unchanged member is named by its
/// entry in the source archive and copied through without ever being fully
/// resident.
#[derive(Debug, Clone)]
pub enum MasdarUdw {
    /// Bytes the caller already has: a rewritten data file, a new script list.
    Dhakira(Vec<u8>),
    /// A member of the archive being rebuilt, copied through.
    MinHawiya(MadkhalRgss),
}

impl MasdarUdw {
    /// How many bytes this member will occupy.
    #[must_use]
    pub fn tul(&self) -> u64 {
        match self {
            Self::Dhakira(bayt) => tul_u64(bayt.len()),
            Self::MinHawiya(madkhal) => madkhal.tul,
        }
    }
}

/// The per-entry data key a rebuild assigns to a member.
///
/// Derived from the member's own name rather than drawn from a random source,
/// so rebuilding the same archive twice produces byte-identical output and a
/// patch can be diffed. Nothing about it is secret and nothing pretends
/// otherwise: the archive publishes the table key in its own header, and a
/// per-entry key that a reader can recover from the table it just read is
/// obfuscation whichever way it was chosen.
///
/// Forced non-zero for one practical reason: a zero key makes the cipher a
/// no-op for its first four bytes, and a member whose key happened to be zero
/// is a member where a broken decipherer still appears to work.
#[must_use]
pub fn miftah_madkhal(masar: &str) -> u32 {
    let khulasa = blake3::hash(masar.as_bytes());
    let mut arbaa = [0_u8; 4];
    if let Some(juz) = khulasa.as_bytes().get(..4) {
        arbaa.copy_from_slice(juz);
    }
    u32::from_le_bytes(arbaa) | 1
}

/// Writes a complete archive: header, obfuscated table, enciphered members.
///
/// The table is emitted before any member, so every offset has to be known
/// before a byte of data is written — which is why this takes the whole member
/// list rather than being an appending writer. The table's own size is
/// `12 + sum(16 + name length) + 4`, and the arithmetic for it is done in `u64`
/// with checked addition, because a member list assembled from a corrupt source
/// archive is exactly the input that would overflow it.
///
/// The output goes through [`Hafiz::iktub`], which preserves the original
/// `Game.rgss3a` and its fingerprint before a byte is written and then writes
/// atomically, so an interrupted write never leaves a truncated archive — a file
/// that *is* the game, and a half-written one is a game that will not start.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] for any failed read or write,
/// [`KhataNusus::NuskhaMafquda`] when the original cannot be preserved,
/// [`KhataNusus::HajmMufrit`] when the member list or a member exceeds this
/// build's ceilings, and [`KhataNusus::HawiyaTalifa`] when the table's size
/// arithmetic overflows.
pub fn uktub_hawiya(
    hafiz: &mut dyn Hafiz,
    hadaf: &Path,
    masdar: Option<&Hawiya>,
    aada: &[(String, MasdarUdw)],
    badhra: u32,
) -> Result<(), KhataNusus> {
    if aada.len() > AQSA_MADAKHIL {
        return Err(KhataNusus::HajmMufrit {
            haql: "the archive's file table",
            qeema: tul_u64(aada.len()),
            saqf: tul_u64(AQSA_MADAKHIL),
        });
    }
    let miftah = miftah_min_badhra(badhra);

    // Phase one: the table's size, hence every member's offset.
    let mut tul_fahras: u64 = 12;
    for (masar_udw, _) in aada {
        let tul_ism = tul_u64(masar_udw.len());
        if tul_ism > u64::from(AQSA_TUL_ISM) {
            return Err(KhataNusus::HajmMufrit {
                haql: "a member's name length",
                qeema: tul_ism,
                saqf: u64::from(AQSA_TUL_ISM),
            });
        }
        tul_fahras = tul_fahras
            .checked_add(16)
            .and_then(|majmu| majmu.checked_add(tul_ism))
            .ok_or_else(|| hawiya_talifa("the file table's size", tul_fahras, u64::MAX))?;
    }
    tul_fahras = tul_fahras
        .checked_add(4)
        .ok_or_else(|| hawiya_talifa("the file table's terminator", tul_fahras, u64::MAX))?;

    // Phase two: the table itself, with every offset now known.
    let mut fahras: Vec<u8> = Vec::with_capacity(hajm_usize(tul_fahras).unwrap_or(0));
    fahras.extend_from_slice(&SIHR);
    fahras.push(ISDAR_HAWIYA);
    fahras.extend_from_slice(&badhra.to_le_bytes());
    let mafatih = miftah.to_le_bytes();
    let mut mawqi = tul_fahras;
    let mut mafatih_udw: Vec<u32> = Vec::with_capacity(aada.len());

    for (masar_udw, mansha) in aada {
        let tul = mansha.tul();
        if tul > AQSA_UDW {
            return Err(KhataNusus::HajmMufrit {
                haql: "an archive member",
                qeema: tul,
                saqf: AQSA_UDW,
            });
        }
        let izaha = u32::try_from(mawqi)
            .map_err(|_| hawiya_talifa("a member's offset", mawqi, u64::from(u32::MAX)))?;
        let tul32 = u32::try_from(tul)
            .map_err(|_| hawiya_talifa("a member's size", tul, u64::from(u32::MAX)))?;
        let miftah_udw = miftah_madkhal(masar_udw);
        mafatih_udw.push(miftah_udw);

        fahras.extend_from_slice(&(izaha ^ miftah).to_le_bytes());
        fahras.extend_from_slice(&(tul32 ^ miftah).to_le_bytes());
        fahras.extend_from_slice(&(miftah_udw ^ miftah).to_le_bytes());
        let tul_ism = u32::try_from(masar_udw.len()).unwrap_or(AQSA_TUL_ISM);
        fahras.extend_from_slice(&(tul_ism ^ miftah).to_le_bytes());
        for (khana, wahid) in masar_udw.as_bytes().iter().enumerate() {
            let qina = mafatih.get(khana & 3).copied().unwrap_or(0);
            fahras.push(*wahid ^ qina);
        }
        mawqi = mawqi
            .checked_add(tul)
            .ok_or_else(|| hawiya_talifa("the archive's total size", mawqi, u64::MAX))?;
    }
    // The terminator is a zero offset, and it is obfuscated like every other
    // field — so the bytes written are the table key itself, which is what zero
    // XORed with it comes to. Four raw zero bytes would decode to the key
    // rather than to zero, and the reader would walk off the end of the table.
    fahras.extend_from_slice(&miftah.to_le_bytes());

    // The archive is assembled whole and handed to the guard in one call. The
    // earlier shape streamed members into a temporary beside the target and
    // renamed over it, which is the same durability but a *second* atomic-write
    // implementation — and a write the reversibility guard never saw. `mawqi`
    // is the finished length, already overflow-checked above, so the buffer is
    // sized once rather than grown member by member.
    let mut makhraj: Vec<u8> = Vec::with_capacity(hajm_usize(mawqi).unwrap_or(fahras.len()));
    makhraj.extend_from_slice(&fahras);
    for ((masar_udw, mansha), miftah_udw) in aada.iter().zip(mafatih_udw.iter()) {
        let mut bayt = match mansha {
            MasdarUdw::Dhakira(mahfuz) => mahfuz.clone(),
            MasdarUdw::MinHawiya(madkhal) => {
                let Some(hawiya) = masdar else {
                    return Err(KhataNusus::HimlMarfud {
                        alia: "the VX Ace archive rebuild",
                        sabab: format!(
                            "{masar_udw} was to be copied from the source archive and no \
                             source archive was supplied"
                        ),
                    });
                };
                hawiya.istakhrij(madkhal)?
            },
        };
        ashfir(&mut bayt, *miftah_udw);
        makhraj.extend_from_slice(&bayt);
    }

    hafiz.iktub(hadaf, &makhraj)
}

/// Re-reads a written archive and confirms it holds what it was told to.
///
/// Names and lengths, not contents: the contents were enciphered on the way out
/// and deciphering all of them again would double the cost of every patch for a
/// check the cipher's own structure already gives — XOR against a key stream is
/// its own inverse, so a member that enciphers and deciphers to different bytes
/// is a bug in `ashfir` rather than in the archive. What this catches is the
/// failure that actually happens: a table whose offsets, key or terminator were
/// computed wrong, which produces an archive the engine mounts and then cannot
/// read a file out of.
///
/// # Errors
///
/// [`KhataNusus::DawraGhayrMutabaqa`] naming how many entries disagree, plus
/// whatever [`Hawiya::iqra`] refuses about the file just written.
pub fn dawra_hawiya(hadaf: &Path, mutawaqqa: &[(String, MasdarUdw)]) -> Result<(), KhataNusus> {
    let hawiya = Hawiya::iqra(hadaf)?;
    let mut mukhtalif: u64 = 0;
    if hawiya.madakhil().len() != mutawaqqa.len() {
        mukhtalif =
            mukhtalif.saturating_add(tul_u64(hawiya.madakhil().len().abs_diff(mutawaqqa.len())));
    }
    for ((masar_udw, mansha), madkhal) in mutawaqqa.iter().zip(hawiya.madakhil().iter()) {
        if &madkhal.masar != masar_udw || madkhal.tul != mansha.tul() {
            mukhtalif = mukhtalif.saturating_add(1);
        }
    }
    if mukhtalif > 0 {
        return Err(KhataNusus::DawraGhayrMutabaqa {
            sigha: SIGHA_RGSS,
            adad: mukhtalif,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Ruby Marshal 4.8
//
// The type byte, then the payload. Every type is listed here with what this
// module does about it, because "we implement Marshal" is a claim nobody should
// take on trust:
//
//   0  nil                     read, written
//   T  true                    read, written
//   F  false                   read, written
//   i  Fixnum                  read, written, in Ruby's own variable-length form
//   l  Bignum                  read, written, payload kept verbatim
//   f  Float                   read, written, payload kept verbatim as text
//   :  Symbol                  read, written, with its own table
//   ;  Symbol back-reference   resolved to a handle
//   "  String                  read, written, bytes plus declared encoding
//   [  Array                   read, written
//   {  Hash                    read, written
//   }  Hash with a default     read, written
//   o  Object                  read, written, class symbol plus ivars
//   u  _dump user object       read, written, payload kept verbatim
//   U  marshal_dump object     read, written
//   S  Struct                  read, written
//   c  Class                   read, written
//   m  Module                  read, written
//   I  instance-variable wrap  read, written, attached to the value it wraps
//   e  extended by a module    read, written, attached to the value it wraps
//   C  subclass of a builtin   read, written, attached to the value it wraps
//   /  Regexp                  read, written
//   @  object back-reference   resolved to a handle
//   d  Data                    REFUSED
//   M  old-style Module        REFUSED
//
// `d` and `M` are refused rather than skipped. Both name a class this process
// would have to know in order to reconstruct anything, neither appears in a VX
// Ace project, and a reader that skipped them would produce a stream that no
// longer round-trips — which the round-trip check would then refuse anyway,
// several seconds later and with a worse message.
//
// ## Where the link table went
//
// Nowhere the caller can see it, and that is the point. Both of Ruby's tables —
// the object table `@` indexes and the symbol table `;` indexes — are resolved
// during reading into handles into one arena. There is no index in the model,
// so there is no index to renumber when a string changes length. The writer
// re-derives both tables from the order it actually emits, which is the only
// order that can possibly be right.
// ---------------------------------------------------------------------------

/// A handle into a [`Silsila`]'s arena.
///
/// Identity, not position. Two references that were one object in the stream
/// are one handle afterwards, which is what makes an edit to a shared object
/// land on every reference to it — and what makes a cyclic graph, which a VX Ace
/// map's event list genuinely is, representable at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Muashir(u32);

impl Muashir {
    /// The handle as an index, for a caller keeping its own side table.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        self.0
    }
}

/// What a string declared about its own encoding.
///
/// Kept rather than resolved to a Rust `String`, because re-emitting the
/// declaration byte for byte is what makes an unedited round trip identical,
/// and because a string that is *not* valid in its declared encoding must be
/// refused rather than lossily converted — replacement characters in a game's
/// dialogue would read as a translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tarmiz {
    /// No encoding instance variable: Ruby reads it as ASCII-8BIT, so the bytes
    /// are binary and are never decoded here.
    Thunai,
    /// `:E => true`, which is UTF-8.
    Utf8,
    /// `:E => false`, which is US-ASCII.
    Ascii,
    /// `:encoding => "..."`, naming something else.
    Musamma(String),
}

/// One value a `Marshal` stream describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Uqda {
    /// `nil`.
    Faragh,
    /// `true` or `false`.
    Sawab(bool),
    /// A `Fixnum`.
    Sahih(i64),
    /// A `Bignum`, kept as the stream spelled it: the sign byte and the
    /// little-endian sixteen-bit words that follow.
    Kabir {
        /// The sign character, `+` or `-`.
        ishara: u8,
        /// The words, verbatim.
        adad: Vec<u8>,
    },
    /// A `Float`, kept as the stream's own text so it re-emits identically.
    Ashari(Vec<u8>),
    /// A `Symbol`.
    Ramz(Vec<u8>),
    /// A `String`: its bytes and what it said about them.
    Nass {
        /// The bytes, exactly as stored.
        bayt: Vec<u8>,
        /// What the instance variables declared about them.
        tarmiz: Tarmiz,
    },
    /// An `Array`.
    Masfufa(Vec<Muashir>),
    /// A `Hash`, in the order the stream wrote its pairs, with its default.
    Kharita {
        /// The pairs.
        azwaj: Vec<(Muashir, Muashir)>,
        /// The default value, for the `}` form.
        tilqai: Option<Muashir>,
    },
    /// A plain object: a class name and its instance variables.
    Kaain {
        /// The class symbol.
        sanf: Muashir,
        /// The instance variables, in order.
        sifat: Vec<(Muashir, Muashir)>,
    },
    /// An object whose class defines `_dump`: `Table`, `Color`, `Tone`.
    ///
    /// The payload is opaque and is kept verbatim. Reconstructing a `Table`
    /// would mean implementing RPG Maker's own binary layout for a structure
    /// that holds no text, and re-emitting the bytes is both correct and the
    /// only thing that round-trips.
    Marfu {
        /// The class symbol.
        sanf: Muashir,
        /// The payload, verbatim.
        himl: Vec<u8>,
    },
    /// An object whose class defines `marshal_dump`.
    MarfuQeema {
        /// The class symbol.
        sanf: Muashir,
        /// The value its `marshal_dump` produced.
        qeema: Muashir,
    },
    /// A `Struct`.
    Bunya {
        /// The struct's symbol.
        sanf: Muashir,
        /// Its members, in order.
        sifat: Vec<(Muashir, Muashir)>,
    },
    /// A `Class` by name.
    Sanf(Vec<u8>),
    /// A `Module` by name.
    Wahda(Vec<u8>),
    /// A `Regexp`: its source and its option bits.
    Namat {
        /// The pattern's bytes.
        bayt: Vec<u8>,
        /// The option byte.
        khiyarat: u8,
    },
}

/// A decoded `Marshal` stream: an arena, three decoration tables, and a root.
///
/// The decorations are kept beside the arena rather than inside [`Uqda`]
/// because Ruby applies them to a value that has *already* been entered in the
/// link table — the `I`, `e` and `C` prefixes do not create objects of their
/// own. Modelling them as wrappers would put an extra node in the arena and the
/// writer would then emit an extra link-table entry, which renumbers everything
/// after it: precisely the failure this whole design exists to make impossible.
#[derive(Debug, Clone)]
pub struct Silsila {
    uqad: Vec<Uqda>,
    sifat: BTreeMap<u32, Vec<(Muashir, Muashir)>>,
    tamdid: BTreeMap<u32, Vec<Muashir>>,
    sanf_asli: BTreeMap<u32, Muashir>,
    jidhr: Muashir,
}

impl Silsila {
    /// The value the stream produced.
    #[must_use]
    pub const fn jidhr(&self) -> Muashir {
        self.jidhr
    }

    /// How many values the stream created.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.uqad.len()
    }

    /// One value by handle, or [`None`] for a handle from another stream.
    #[must_use]
    pub fn uqda(&self, muashir: Muashir) -> Option<&Uqda> {
        self.uqad.get(usize::try_from(muashir.0).ok()?)
    }

    /// Every handle in the arena, in creation order.
    ///
    /// Creation order is emission order, so walking this visits every value
    /// exactly once whatever the shape of the graph — which matters, because a
    /// VX Ace map refers to itself and a recursive walk over it does not stop.
    pub fn kul(&self) -> impl Iterator<Item = (Muashir, &Uqda)> {
        self.uqad
            .iter()
            .enumerate()
            .filter_map(|(fahras, uqda)| Some((Muashir(u32::try_from(fahras).ok()?), uqda)))
    }

    /// A symbol's name, when the handle is a symbol.
    #[must_use]
    pub fn ramz(&self, muashir: Muashir) -> Option<&str> {
        match self.uqda(muashir)? {
            Uqda::Ramz(bayt) => core::str::from_utf8(bayt).ok(),
            _ => None,
        }
    }

    /// A string's text, when the handle is a string that declared a text
    /// encoding this build reads.
    ///
    /// [`None`] for a binary string, which is not a failure: `Table` payloads
    /// and tileset flags are stored as binary strings and there is nothing in
    /// them to translate.
    #[must_use]
    pub fn nass(&self, muashir: Muashir) -> Option<&str> {
        match self.uqda(muashir)? {
            Uqda::Nass { bayt, tarmiz } => match tarmiz {
                Tarmiz::Utf8 | Tarmiz::Ascii => core::str::from_utf8(bayt).ok(),
                Tarmiz::Musamma(ism) if ism.eq_ignore_ascii_case("UTF-8") => {
                    core::str::from_utf8(bayt).ok()
                },
                _ => None,
            },
            _ => None,
        }
    }

    /// An integer, when the handle is one.
    #[must_use]
    pub fn sahih(&self, muashir: Muashir) -> Option<i64> {
        match self.uqda(muashir)? {
            Uqda::Sahih(qeema) => Some(*qeema),
            _ => None,
        }
    }

    /// An array's members.
    #[must_use]
    pub fn anasir(&self, muashir: Muashir) -> Option<&[Muashir]> {
        match self.uqda(muashir)? {
            Uqda::Masfufa(anasir) => Some(anasir.as_slice()),
            _ => None,
        }
    }

    /// An object's or struct's instance variable by name, including the `@`.
    #[must_use]
    pub fn sifa(&self, muashir: Muashir, ism: &str) -> Option<Muashir> {
        let (Uqda::Kaain { sifat, .. } | Uqda::Bunya { sifat, .. }) = self.uqda(muashir)? else {
            return None;
        };
        sifat
            .iter()
            .find(|(ramz, _)| self.ramz(*ramz) == Some(ism))
            .map(|(_, qeema)| *qeema)
    }

    /// A hash's value for a symbol key.
    #[must_use]
    pub fn min_kharita(&self, muashir: Muashir, ism: &str) -> Option<Muashir> {
        let Uqda::Kharita { azwaj, .. } = self.uqda(muashir)? else {
            return None;
        };
        azwaj
            .iter()
            .find(|(miftah, _)| self.ramz(*miftah) == Some(ism))
            .map(|(_, qeema)| *qeema)
    }

    /// An object's class name, when the handle is an object.
    #[must_use]
    pub fn sanf(&self, muashir: Muashir) -> Option<&str> {
        let sanf = match self.uqda(muashir)? {
            Uqda::Kaain { sanf, .. }
            | Uqda::Marfu { sanf, .. }
            | Uqda::MarfuQeema { sanf, .. }
            | Uqda::Bunya { sanf, .. } => *sanf,
            _ => return None,
        };
        self.ramz(sanf)
    }

    /// Replaces a string's bytes, keeping its declared encoding.
    ///
    /// The only mutation this type offers, and deliberately the only one. A
    /// translation replaces text and nothing else; an API that could replace a
    /// node with a different *kind* of node would be an API through which a
    /// patch could change a map's event list into a number.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the handle is not a string, or
    /// is not one from this stream.
    pub fn baddil_nass(&mut self, muashir: Muashir, jadeed: &str) -> Result<(), KhataNusus> {
        let khana = usize::try_from(muashir.0)
            .ok()
            .and_then(|fahras| self.uqad.get_mut(fahras))
            .ok_or_else(|| bunya_ghayr("a handle from another stream reached an edit"))?;
        let Uqda::Nass { bayt, tarmiz } = khana else {
            return Err(bunya_ghayr("an edit targeted a value that is not a string"));
        };
        // A translated string is UTF-8 whatever the original declared. Ruby
        // reads a string with no encoding instance variable as ASCII-8BIT and
        // would then compare it byte-wise against a UTF-8 literal in a script,
        // so the declaration is upgraded rather than preserved — the one place
        // this writer deliberately does not reproduce what it read, and the
        // round-trip check is run before the edit rather than after it for
        // exactly that reason.
        *bayt = jadeed.as_bytes().to_vec();
        *tarmiz = Tarmiz::Utf8;
        Ok(())
    }
}

/// The refusal a stream that is not the shape this engine writes produces.
fn bunya_ghayr(sabab: &str) -> KhataNusus {
    KhataNusus::BunyaGhayrMutawaqqaa {
        malaf: SIGHA_SILSILA.to_owned(),
        haql: sabab.to_owned(),
    }
}

/// A bounds-checked cursor over a `Marshal` stream.
#[derive(Debug)]
struct Qari<'a> {
    bayt: &'a [u8],
    mawqi: usize,
}

impl<'a> Qari<'a> {
    /// A cursor at the start of a stream.
    const fn jadeed(bayt: &'a [u8]) -> Self {
        Self { bayt, mawqi: 0 }
    }

    /// The refusal a short read produces.
    fn qaseer(&self, haql: &'static str, matlub: u64) -> KhataNusus {
        KhataNusus::MalafQaseer {
            haql,
            tul: tul_u64(self.bayt.len().saturating_sub(self.mawqi)),
            matlub,
        }
    }

    /// One byte.
    fn wahid(&mut self, haql: &'static str) -> Result<u8, KhataNusus> {
        let Some(qeema) = self.bayt.get(self.mawqi).copied() else {
            return Err(self.qaseer(haql, 1));
        };
        self.mawqi = self.mawqi.saturating_add(1);
        Ok(qeema)
    }

    /// A run of bytes, borrowed from the stream rather than from the cursor.
    fn nitaq(&mut self, haql: &'static str, tul: usize) -> Result<&'a [u8], KhataNusus> {
        let masdar: &'a [u8] = self.bayt;
        let Some(nihaya) = self.mawqi.checked_add(tul) else {
            return Err(self.qaseer(haql, tul_u64(tul)));
        };
        let Some(juz) = masdar.get(self.mawqi..nihaya) else {
            return Err(self.qaseer(haql, tul_u64(tul)));
        };
        self.mawqi = nihaya;
        Ok(juz)
    }

    /// Ruby's own variable-length integer, which is `r_long` in `marshal.c`.
    ///
    /// Five cases, and every one of them is load-bearing:
    ///
    /// * a zero byte is zero;
    /// * a byte in `5..=127` is itself minus five, which is how Ruby packs the
    ///   small non-negative numbers that make up almost every length field;
    /// * a byte in `-128..=-5` is itself plus five, the same trick for small
    ///   negatives;
    /// * a byte in `1..=4` counts the little-endian bytes of a non-negative
    ///   number that follow;
    /// * a byte in `-4..=-1` counts the bytes of a *negative* number, which is
    ///   sign-extended from all-ones rather than zero-filled — and getting that
    ///   one wrong turns a small negative into a number near four billion, which
    ///   is how an event's coordinate becomes an allocation request.
    fn raqm(&mut self, haql: &'static str) -> Result<i64, KhataNusus> {
        let awwal = self.wahid(haql)?;
        let ishara = i64::from(i8::from_le_bytes([awwal]));
        if ishara == 0 {
            return Ok(0);
        }
        if (5..128).contains(&ishara) {
            return Ok(ishara.saturating_sub(5));
        }
        if (-128..-4).contains(&ishara) {
            return Ok(ishara.saturating_add(5));
        }
        let salib = ishara < 0;
        let adad = usize::try_from(ishara.unsigned_abs()).unwrap_or(0);
        if adad > 8 {
            return Err(bunya_ghayr(
                "an integer declares more bytes than one can hold",
            ));
        }
        let juz = self.nitaq(haql, adad)?;
        let mut qeema: i64 = if salib { -1 } else { 0 };
        for (rutba, wahid) in juz.iter().enumerate() {
            let izaha = u32::try_from(rutba).unwrap_or(0).saturating_mul(8).min(56);
            let qina = i64::from(*wahid) << izaha;
            let qina3 = 0xFF_i64 << izaha;
            qeema &= !qina3;
            qeema |= qina;
        }
        Ok(qeema)
    }

    /// A length field, refused before it is used to reserve anything.
    fn tul(&mut self, haql: &'static str, saqf: u64) -> Result<usize, KhataNusus> {
        let khaam = self.raqm(haql)?;
        let qeema = u64::try_from(khaam).map_err(|_| bunya_ghayr("a length field is negative"))?;
        if qeema > saqf {
            return Err(KhataNusus::HajmMufrit { haql, qeema, saqf });
        }
        hajm_usize(qeema).ok_or(KhataNusus::HajmMufrit { haql, qeema, saqf })
    }
}

/// The reader's mutable state: the arena and both of Ruby's tables.
#[derive(Debug)]
struct AlatQiraa {
    uqad: Vec<Uqda>,
    sifat: BTreeMap<u32, Vec<(Muashir, Muashir)>>,
    tamdid: BTreeMap<u32, Vec<Muashir>>,
    sanf_asli: BTreeMap<u32, Muashir>,
    wasla: Vec<Muashir>,
    rumuz: Vec<Muashir>,
}

impl AlatQiraa {
    /// An empty reader.
    const fn jadeed() -> Self {
        Self {
            uqad: Vec::new(),
            sifat: BTreeMap::new(),
            tamdid: BTreeMap::new(),
            sanf_asli: BTreeMap::new(),
            wasla: Vec::new(),
            rumuz: Vec::new(),
        }
    }

    /// Interns a value and returns its handle.
    fn adhif(&mut self, uqda: Uqda) -> Result<Muashir, KhataNusus> {
        if self.uqad.len() >= AQSA_KAINAT {
            return Err(KhataNusus::HajmMufrit {
                haql: "the objects a Marshal stream creates",
                qeema: tul_u64(self.uqad.len().saturating_add(1)),
                saqf: tul_u64(AQSA_KAINAT),
            });
        }
        let raqm = u32::try_from(self.uqad.len()).map_err(|_| KhataNusus::HajmMufrit {
            haql: "the objects a Marshal stream creates",
            qeema: tul_u64(self.uqad.len()),
            saqf: u64::from(u32::MAX),
        })?;
        self.uqad.push(uqda);
        Ok(Muashir(raqm))
    }

    /// Interns a value and enters it in the object link table, in that order.
    ///
    /// The order is the specification, not a convenience: Ruby enters a
    /// container before it reads the container's members, which is what lets an
    /// object contain a reference to itself. Entering afterwards would shift
    /// every index inside it by one.
    fn adhif_marja(&mut self, uqda: Uqda) -> Result<Muashir, KhataNusus> {
        let muashir = self.adhif(uqda)?;
        self.wasla.push(muashir);
        Ok(muashir)
    }

    /// Replaces a value in place, for a container filled after it was entered.
    fn haddith(&mut self, muashir: Muashir, uqda: Uqda) -> Result<(), KhataNusus> {
        let khana = usize::try_from(muashir.0)
            .ok()
            .and_then(|fahras| self.uqad.get_mut(fahras))
            .ok_or_else(|| bunya_ghayr("a container was filled through a stale handle"))?;
        *khana = uqda;
        Ok(())
    }
}

/// Decodes a `Marshal` stream into an arena with every back-reference resolved.
///
/// # Errors
///
/// [`KhataNusus::SihrGhayrMutabaq`] when the stream does not open with the 4.8
/// version pair, [`KhataNusus::MalafQaseer`] when it ends inside a value,
/// [`KhataNusus::HajmMufrit`] when it declares a length, a depth or an object
/// count above this build's ceilings, [`KhataNusus::HawiyaTalifa`] when a
/// back-reference names an index that was never entered, and
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] for a type byte this reader refuses.
pub fn iqra_silsila(bayt: &[u8]) -> Result<Silsila, KhataNusus> {
    if tul_u64(bayt.len()) > AQSA_SILSILA {
        return Err(KhataNusus::HajmMufrit {
            haql: "a Marshal stream",
            qeema: tul_u64(bayt.len()),
            saqf: AQSA_SILSILA,
        });
    }
    if !bayt.starts_with(&ISDAR_SILSILA) {
        return Err(KhataNusus::SihrGhayrMutabaq {
            masar: PathBuf::from(SIGHA_SILSILA),
            sigha: "Ruby Marshal",
        });
    }
    let mut qari = Qari::jadeed(bayt);
    let _ = qari.nitaq("the Marshal version pair", 2)?;
    let mut ala = AlatQiraa::jadeed();
    let jidhr = iqra_qeema(&mut ala, &mut qari, 0)?;
    Ok(Silsila {
        uqad: ala.uqad,
        sifat: ala.sifat,
        tamdid: ala.tamdid,
        sanf_asli: ala.sanf_asli,
        jidhr,
    })
}

/// Reads one value, resolving both of Ruby's tables into handles.
fn iqra_qeema(ala: &mut AlatQiraa, qari: &mut Qari<'_>, umq: u32) -> Result<Muashir, KhataNusus> {
    if umq > AQSA_UMQ {
        return Err(KhataNusus::HajmMufrit {
            haql: "Marshal nesting depth",
            qeema: u64::from(umq),
            saqf: u64::from(AQSA_UMQ),
        });
    }
    let naw = qari.wahid("a Marshal type byte")?;
    let taht = umq.saturating_add(1);
    match naw {
        b'0' => ala.adhif(Uqda::Faragh),
        b'T' => ala.adhif(Uqda::Sawab(true)),
        b'F' => ala.adhif(Uqda::Sawab(false)),
        b'i' => {
            let qeema = qari.raqm("a Fixnum")?;
            ala.adhif(Uqda::Sahih(qeema))
        },

        // `@` and `;` are the two link tables, and both resolve to the handle
        // that was entered rather than to a copy of it. That is the whole
        // renumbering problem solved at the point it enters the model.
        b'@' => {
            let fahras = qari.raqm("an object back-reference")?;
            let khana = usize::try_from(fahras)
                .map_err(|_| bunya_ghayr("an object back-reference is negative"))?;
            ala.wasla
                .get(khana)
                .copied()
                .ok_or_else(|| KhataNusus::HawiyaTalifa {
                    sigha: SIGHA_SILSILA,
                    haql: "an object back-reference",
                    qeema: tul_u64(khana),
                    hadd: tul_u64(ala.wasla.len()),
                })
        },
        b';' => {
            let fahras = qari.raqm("a symbol back-reference")?;
            let khana = usize::try_from(fahras)
                .map_err(|_| bunya_ghayr("a symbol back-reference is negative"))?;
            ala.rumuz
                .get(khana)
                .copied()
                .ok_or_else(|| KhataNusus::HawiyaTalifa {
                    sigha: SIGHA_SILSILA,
                    haql: "a symbol back-reference",
                    qeema: tul_u64(khana),
                    hadd: tul_u64(ala.rumuz.len()),
                })
        },
        b':' => {
            let tul = qari.tul("a symbol's length", u64::from(AQSA_TUL_ISM))?;
            let ism = qari.nitaq("a symbol", tul)?.to_vec();
            let muashir = ala.adhif(Uqda::Ramz(ism))?;
            ala.rumuz.push(muashir);
            Ok(muashir)
        },

        b'"' => {
            let tul = qari.tul("a string's length", AQSA_SILSILA)?;
            let jism = qari.nitaq("a string", tul)?.to_vec();
            ala.adhif_marja(Uqda::Nass {
                bayt: jism,
                tarmiz: Tarmiz::Thunai,
            })
        },
        b'f' => {
            let tul = qari.tul("a float's length", 256)?;
            let jism = qari.nitaq("a float", tul)?.to_vec();
            ala.adhif_marja(Uqda::Ashari(jism))
        },
        b'l' => {
            let ishara = qari.wahid("a Bignum's sign")?;
            let kalimat = qari.tul("a Bignum's word count", AQSA_SILSILA)?;
            let jism = qari.nitaq("a Bignum", kalimat.saturating_mul(2))?.to_vec();
            ala.adhif_marja(Uqda::Kabir { ishara, adad: jism })
        },
        b'/' => {
            let tul = qari.tul("a Regexp's length", AQSA_SILSILA)?;
            let jism = qari.nitaq("a Regexp", tul)?.to_vec();
            let khiyarat = qari.wahid("a Regexp's options")?;
            ala.adhif_marja(Uqda::Namat {
                bayt: jism,
                khiyarat,
            })
        },
        b'c' | b'm' => {
            let tul = qari.tul("a class name's length", u64::from(AQSA_TUL_ISM))?;
            let ism = qari.nitaq("a class name", tul)?.to_vec();
            ala.adhif_marja(if naw == b'c' {
                Uqda::Sanf(ism)
            } else {
                Uqda::Wahda(ism)
            })
        },

        b'[' => {
            let adad = qari.tul("an array's length", tul_u64(AQSA_KAINAT))?;
            let muashir = ala.adhif_marja(Uqda::Masfufa(Vec::new()))?;
            let mut anasir = Vec::with_capacity(adad.min(1024));
            for _ in 0..adad {
                anasir.push(iqra_qeema(ala, qari, taht)?);
            }
            ala.haddith(muashir, Uqda::Masfufa(anasir))?;
            Ok(muashir)
        },
        b'{' | b'}' => {
            let adad = qari.tul("a hash's length", tul_u64(AQSA_KAINAT))?;
            let muashir = ala.adhif_marja(Uqda::Kharita {
                azwaj: Vec::new(),
                tilqai: None,
            })?;
            let mut azwaj = Vec::with_capacity(adad.min(1024));
            for _ in 0..adad {
                let miftah = iqra_qeema(ala, qari, taht)?;
                let qeema = iqra_qeema(ala, qari, taht)?;
                azwaj.push((miftah, qeema));
            }
            let tilqai = if naw == b'}' {
                Some(iqra_qeema(ala, qari, taht)?)
            } else {
                None
            };
            ala.haddith(muashir, Uqda::Kharita { azwaj, tilqai })?;
            Ok(muashir)
        },
        b'o' | b'S' => {
            let sanf = iqra_qeema(ala, qari, taht)?;
            let adad = qari.tul("an object's field count", tul_u64(AQSA_KAINAT))?;
            let hayakil = if naw == b'o' {
                Uqda::Kaain {
                    sanf,
                    sifat: Vec::new(),
                }
            } else {
                Uqda::Bunya {
                    sanf,
                    sifat: Vec::new(),
                }
            };
            let muashir = ala.adhif_marja(hayakil)?;
            let mut sifat = Vec::with_capacity(adad.min(256));
            for _ in 0..adad {
                let ism = iqra_qeema(ala, qari, taht)?;
                let qeema = iqra_qeema(ala, qari, taht)?;
                sifat.push((ism, qeema));
            }
            ala.haddith(
                muashir,
                if naw == b'o' {
                    Uqda::Kaain { sanf, sifat }
                } else {
                    Uqda::Bunya { sanf, sifat }
                },
            )?;
            Ok(muashir)
        },
        b'u' => {
            let sanf = iqra_qeema(ala, qari, taht)?;
            let tul = qari.tul("a dumped object's payload", AQSA_SILSILA)?;
            let himl = qari.nitaq("a dumped object's payload", tul)?.to_vec();
            ala.adhif_marja(Uqda::Marfu { sanf, himl })
        },
        b'U' => {
            let sanf = iqra_qeema(ala, qari, taht)?;
            let muashir = ala.adhif_marja(Uqda::MarfuQeema {
                sanf,
                qeema: Muashir(0),
            })?;
            let qeema = iqra_qeema(ala, qari, taht)?;
            ala.haddith(muashir, Uqda::MarfuQeema { sanf, qeema })?;
            Ok(muashir)
        },

        // The three decorations. None of them creates an object of its own, so
        // none of them enters the link table — the value they wrap does.
        b'I' => {
            let jism = iqra_qeema(ala, qari, taht)?;
            let adad = qari.tul("an instance-variable count", tul_u64(AQSA_KAINAT))?;
            let mut sifat = Vec::with_capacity(adad.min(64));
            for _ in 0..adad {
                let ism = iqra_qeema(ala, qari, taht)?;
                let qeema = iqra_qeema(ala, qari, taht)?;
                sifat.push((ism, qeema));
            }
            imtass_tarmiz(ala, jism, sifat);
            Ok(jism)
        },
        b'e' => {
            let wahda = iqra_qeema(ala, qari, taht)?;
            let jism = iqra_qeema(ala, qari, taht)?;
            ala.tamdid.entry(jism.raqm()).or_default().push(wahda);
            Ok(jism)
        },
        b'C' => {
            let sanf = iqra_qeema(ala, qari, taht)?;
            let jism = iqra_qeema(ala, qari, taht)?;
            let _ = ala.sanf_asli.insert(jism.raqm(), sanf);
            Ok(jism)
        },

        b'd' => Err(bunya_ghayr(
            "a Data value: it names a C extension's own layout, no VX Ace project \
             contains one, and reconstructing it would mean guessing at bytes",
        )),
        b'M' => Err(bunya_ghayr(
            "an old-style Module value, which Ruby itself has not written since 1.8",
        )),
        _ => Err(bunya_ghayr(&format!(
            "an unknown Marshal type byte {naw:#04x}"
        ))),
    }
}

/// Folds a string's encoding instance variables into its own declaration, and
/// keeps everything else.
///
/// `:E => true` and `:E => false` are how Ruby spells UTF-8 and US-ASCII, and
/// `:encoding => "..."` is how it spells everything else. They are moved onto
/// the node rather than left in the side table for one reason: the writer emits
/// the declaration from the node, so leaving them in both places would emit
/// them twice and the round trip would not match.
fn imtass_tarmiz(ala: &mut AlatQiraa, jism: Muashir, sifat: Vec<(Muashir, Muashir)>) {
    let huwa_nass = matches!(
        ala.uqad.get(usize::try_from(jism.0).unwrap_or(usize::MAX)),
        Some(Uqda::Nass { .. })
    );
    if !huwa_nass {
        if !sifat.is_empty() {
            let _ = ala.sifat.insert(jism.raqm(), sifat);
        }
        return;
    }

    let mut baqi: Vec<(Muashir, Muashir)> = Vec::new();
    let mut tarmiz = Tarmiz::Thunai;
    for (ism, qeema) in sifat {
        let ramz = match ala.uqad.get(usize::try_from(ism.0).unwrap_or(usize::MAX)) {
            Some(Uqda::Ramz(bayt)) => core::str::from_utf8(bayt).unwrap_or_default().to_owned(),
            _ => String::new(),
        };
        let mahmul = ala
            .uqad
            .get(usize::try_from(qeema.0).unwrap_or(usize::MAX))
            .cloned();
        match (ramz.as_str(), mahmul) {
            ("E", Some(Uqda::Sawab(true))) => tarmiz = Tarmiz::Utf8,
            ("E", Some(Uqda::Sawab(false))) => tarmiz = Tarmiz::Ascii,
            ("encoding", Some(Uqda::Nass { bayt, .. })) => {
                tarmiz = Tarmiz::Musamma(String::from_utf8_lossy(&bayt).into_owned());
            },
            _ => baqi.push((ism, qeema)),
        }
    }
    if let Some(Uqda::Nass { tarmiz: hadaf, .. }) = ala
        .uqad
        .get_mut(usize::try_from(jism.0).unwrap_or(usize::MAX))
    {
        *hadaf = tarmiz;
    }
    if !baqi.is_empty() {
        let _ = ala.sifat.insert(jism.raqm(), baqi);
    }
}

/// The writer's state: the two tables, re-derived from what is emitted.
///
/// The object table is keyed by handle, so identity decides what becomes a
/// back-reference. The symbol table is keyed by the symbol's *bytes*, because
/// Ruby interns symbols and a stream that spelled the same symbol twice would
/// otherwise emit it twice and disagree with the engine's own writer.
#[derive(Debug, Default)]
struct AlatKitaba {
    wasla: BTreeMap<u32, i64>,
    rumuz: BTreeMap<Vec<u8>, i64>,
    /// How many objects have been entered, including the ones with no handle.
    ///
    /// Not `wasla.len()`, and the difference is the whole reason this field
    /// exists: an encoding declaration's value is a `String`, which Ruby enters
    /// in the object table like any other, and this writer synthesizes it from
    /// [`Tarmiz`] rather than from a node. Counting only the handles would give
    /// the next real object an index one too low and every back-reference after
    /// it would name the wrong object.
    adad_wasla: i64,
}

impl AlatKitaba {
    /// Assigns the next object-table index to a handle.
    fn sajjil(&mut self, muashir: Muashir) -> i64 {
        let fahras = self.adad_wasla;
        self.adad_wasla = self.adad_wasla.saturating_add(1);
        let _ = self.wasla.insert(muashir.raqm(), fahras);
        fahras
    }

    /// Consumes an index for an object this writer synthesized.
    const fn sajjil_bila_maqbad(&mut self) {
        self.adad_wasla = self.adad_wasla.saturating_add(1);
    }
}

/// Writes Ruby's variable-length integer, which is `w_long` in `marshal.c`.
///
/// The inverse of [`Qari::raqm`], case for case. The long form emits a count
/// byte — positive for a non-negative value, negative for a negative one —
/// followed by that many little-endian bytes, and it stops as soon as the
/// remaining bits are all zero for a positive number or all one for a negative
/// one. That "all one" stop is what makes `-300` two bytes rather than eight.
fn uktub_raqm(kharj: &mut Vec<u8>, qeema: i64) {
    if qeema == 0 {
        kharj.push(0);
        return;
    }
    if (1..123).contains(&qeema)
        && let Ok(sagheer) = u8::try_from(qeema.saturating_add(5))
    {
        kharj.push(sagheer);
        return;
    }
    if (-123..0).contains(&qeema)
        && let Ok(sagheer) = i8::try_from(qeema.saturating_sub(5))
    {
        kharj.extend_from_slice(&sagheer.to_le_bytes());
        return;
    }
    let mut jism: Vec<u8> = Vec::with_capacity(8);
    let mut baqi = qeema;
    for _ in 0..8 {
        jism.push(u8::try_from(baqi & 0xFF).unwrap_or(0));
        baqi >>= 8;
        if baqi == 0 || baqi == -1 {
            break;
        }
    }
    let adad = i64::try_from(jism.len()).unwrap_or(8);
    let adad = if qeema < 0 { -adad } else { adad };
    kharj.extend_from_slice(&i8::try_from(adad).unwrap_or(0).to_le_bytes());
    kharj.extend_from_slice(&jism);
}

/// Writes a length-prefixed byte run.
fn uktub_bayt(kharj: &mut Vec<u8>, jism: &[u8]) {
    uktub_raqm(kharj, i64::try_from(jism.len()).unwrap_or(i64::MAX));
    kharj.extend_from_slice(jism);
}

/// Writes a symbol, emitting a back-reference for one already written.
fn uktub_ramz(kharj: &mut Vec<u8>, ism: &[u8], katib: &mut AlatKitaba) {
    if let Some(fahras) = katib.rumuz.get(ism).copied() {
        kharj.push(b';');
        uktub_raqm(kharj, fahras);
        return;
    }
    let fahras = i64::try_from(katib.rumuz.len()).unwrap_or(i64::MAX);
    let _ = katib.rumuz.insert(ism.to_vec(), fahras);
    kharj.push(b':');
    uktub_bayt(kharj, ism);
}

/// Serializes an arena back into a `Marshal` stream.
///
/// Both tables are re-derived from the emission order this function produces,
/// which is what makes an edited stream correct: there is no index carried over
/// from the input, so nothing has to be renumbered when a string changes length
/// or an object is inserted. The cost is that an unedited round trip has to be
/// *checked* rather than assumed, and [`dawra_mutabaqa`] is that check.
///
/// # Errors
///
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] when a handle is not from this stream,
/// or the graph nests deeper than [`AQSA_UMQ`].
pub fn uktub_silsila(silsila: &Silsila) -> Result<Vec<u8>, KhataNusus> {
    let mut kharj: Vec<u8> = Vec::with_capacity(silsila.adad().saturating_mul(8));
    kharj.extend_from_slice(&ISDAR_SILSILA);
    let mut katib = AlatKitaba::default();
    uktub_qeema(silsila, silsila.jidhr(), &mut kharj, &mut katib, 0)?;
    Ok(kharj)
}

/// The instance variables a node re-emits, encoding included.
///
/// Returned as bytes-and-handle pairs rather than as handles throughout,
/// because the encoding declaration is synthesized from [`Tarmiz`] rather than
/// held in the arena: folding it into the node on read is what stops it being
/// emitted twice, and rebuilding it here is the other half of that trade.
fn sifat_lil_kitaba(
    silsila: &Silsila,
    muashir: Muashir,
) -> (Option<&Tarmiz>, &[(Muashir, Muashir)]) {
    let tarmiz = match silsila.uqda(muashir) {
        Some(Uqda::Nass { tarmiz, .. }) if *tarmiz != Tarmiz::Thunai => Some(tarmiz),
        _ => None,
    };
    let sifat = silsila
        .sifat
        .get(&muashir.raqm())
        .map_or::<&[(Muashir, Muashir)], _>(&[], Vec::as_slice);
    (tarmiz, sifat)
}

/// Writes one value, with its decorations, its body and its instance
/// variables, in Ruby's own order.
///
/// The order is `I`, then each `e`, then `C`, then the body, then the instance
/// variables — and it is stated here rather than derived, because it is the one
/// part of this writer that cannot be checked against the reader by inspection.
/// [`dawra_mutabaqa`] is what actually establishes it, on the game's own files,
/// before anything is written.
fn uktub_qeema(
    silsila: &Silsila,
    muashir: Muashir,
    kharj: &mut Vec<u8>,
    katib: &mut AlatKitaba,
    umq: u32,
) -> Result<(), KhataNusus> {
    if umq > AQSA_UMQ {
        return Err(bunya_ghayr(
            "a value nests deeper than this writer will emit",
        ));
    }
    let uqda = silsila
        .uqda(muashir)
        .ok_or_else(|| bunya_ghayr("a handle from another stream reached the writer"))?;
    let taht = umq.saturating_add(1);

    // Symbols and immediates never enter the object table, so they are handled
    // before the back-reference check rather than inside it.
    match uqda {
        Uqda::Ramz(ism) => {
            uktub_ramz(kharj, ism, katib);
            return Ok(());
        },
        Uqda::Faragh => {
            kharj.push(b'0');
            return Ok(());
        },
        Uqda::Sawab(qeema) => {
            kharj.push(if *qeema { b'T' } else { b'F' });
            return Ok(());
        },
        Uqda::Sahih(qeema) => {
            kharj.push(b'i');
            uktub_raqm(kharj, *qeema);
            return Ok(());
        },
        _ => {},
    }

    if let Some(fahras) = katib.wasla.get(&muashir.raqm()).copied() {
        kharj.push(b'@');
        uktub_raqm(kharj, fahras);
        return Ok(());
    }

    let (tarmiz, sifat) = sifat_lil_kitaba(silsila, muashir);
    let adad_sifat = sifat.len().saturating_add(usize::from(tarmiz.is_some()));
    if adad_sifat > 0 {
        kharj.push(b'I');
    }
    if let Some(wahdat) = silsila.tamdid.get(&muashir.raqm()) {
        for wahda in wahdat {
            kharj.push(b'e');
            uktub_qeema(silsila, *wahda, kharj, katib, taht)?;
        }
    }
    if let Some(sanf) = silsila.sanf_asli.get(&muashir.raqm()).copied() {
        kharj.push(b'C');
        uktub_qeema(silsila, sanf, kharj, katib, taht)?;
    }
    let _ = katib.sajjil(muashir);

    match uqda {
        Uqda::Nass { bayt, .. } => {
            kharj.push(b'"');
            uktub_bayt(kharj, bayt);
        },
        Uqda::Ashari(bayt) => {
            kharj.push(b'f');
            uktub_bayt(kharj, bayt);
        },
        Uqda::Kabir { ishara, adad } => {
            kharj.push(b'l');
            kharj.push(*ishara);
            // The count is in sixteen-bit words, which is why this is a shift
            // and not a division: the lint that forbids bare integer division
            // is forbidding the case where the divisor could be zero, and two
            // never is — but a shift also says out loud that the unit is words.
            uktub_raqm(kharj, i64::try_from(adad.len() >> 1).unwrap_or(0));
            kharj.extend_from_slice(adad);
        },
        Uqda::Namat { bayt, khiyarat } => {
            kharj.push(b'/');
            uktub_bayt(kharj, bayt);
            kharj.push(*khiyarat);
        },
        Uqda::Sanf(ism) => {
            kharj.push(b'c');
            uktub_bayt(kharj, ism);
        },
        Uqda::Wahda(ism) => {
            kharj.push(b'm');
            uktub_bayt(kharj, ism);
        },
        Uqda::Masfufa(anasir) => {
            kharj.push(b'[');
            uktub_raqm(kharj, i64::try_from(anasir.len()).unwrap_or(i64::MAX));
            for udw in anasir {
                uktub_qeema(silsila, *udw, kharj, katib, taht)?;
            }
        },
        Uqda::Kharita { azwaj, tilqai } => {
            kharj.push(if tilqai.is_some() { b'}' } else { b'{' });
            uktub_raqm(kharj, i64::try_from(azwaj.len()).unwrap_or(i64::MAX));
            for (miftah, qeema) in azwaj {
                uktub_qeema(silsila, *miftah, kharj, katib, taht)?;
                uktub_qeema(silsila, *qeema, kharj, katib, taht)?;
            }
            if let Some(asl) = tilqai {
                uktub_qeema(silsila, *asl, kharj, katib, taht)?;
            }
        },
        Uqda::Kaain { sanf, sifat: huqul } | Uqda::Bunya { sanf, sifat: huqul } => {
            kharj.push(if matches!(uqda, Uqda::Kaain { .. }) {
                b'o'
            } else {
                b'S'
            });
            uktub_qeema(silsila, *sanf, kharj, katib, taht)?;
            uktub_raqm(kharj, i64::try_from(huqul.len()).unwrap_or(i64::MAX));
            for (ism, qeema) in huqul {
                uktub_qeema(silsila, *ism, kharj, katib, taht)?;
                uktub_qeema(silsila, *qeema, kharj, katib, taht)?;
            }
        },
        Uqda::Marfu { sanf, himl } => {
            kharj.push(b'u');
            uktub_qeema(silsila, *sanf, kharj, katib, taht)?;
            uktub_bayt(kharj, himl);
        },
        Uqda::MarfuQeema { sanf, qeema } => {
            kharj.push(b'U');
            uktub_qeema(silsila, *sanf, kharj, katib, taht)?;
            uktub_qeema(silsila, *qeema, kharj, katib, taht)?;
        },
        // Handled above, before the object table was consulted.
        Uqda::Ramz(_) | Uqda::Faragh | Uqda::Sawab(_) | Uqda::Sahih(_) => {},
    }

    if adad_sifat > 0 {
        uktub_raqm(kharj, i64::try_from(adad_sifat).unwrap_or(i64::MAX));
        match tarmiz {
            Some(Tarmiz::Utf8) => {
                uktub_ramz(kharj, b"E", katib);
                kharj.push(b'T');
            },
            Some(Tarmiz::Ascii) => {
                uktub_ramz(kharj, b"E", katib);
                kharj.push(b'F');
            },
            Some(Tarmiz::Musamma(ism)) => {
                uktub_ramz(kharj, b"encoding", katib);
                // The name is a String, and Ruby enters it in the object table
                // like any other. This writer has no handle for it, so the
                // index it consumes is accounted for explicitly.
                katib.sajjil_bila_maqbad();
                kharj.push(b'"');
                uktub_bayt(kharj, ism.as_bytes());
            },
            Some(Tarmiz::Thunai) | None => {},
        }
        for (ism, qeema) in sifat {
            uktub_qeema(silsila, *ism, kharj, katib, taht)?;
            uktub_qeema(silsila, *qeema, kharj, katib, taht)?;
        }
    }
    Ok(())
}

/// Confirms that reading and re-writing a stream reproduces it exactly.
///
/// The whole justification for the arena design rests on this. A reader that
/// silently dropped a type byte, or a writer that emitted a decoration in the
/// wrong order, would produce a stream Ruby still loads — into objects that are
/// subtly not the ones the game shipped. Comparing the bytes is the only check
/// that catches that, and it is run *before* any edit, on the file as it was
/// found, so a mismatch means "this build does not understand this file" rather
/// than "the translation broke it".
///
/// # Errors
///
/// [`KhataNusus::DawraGhayrMutabaqa`] naming how many bytes differ, and
/// whatever [`uktub_silsila`] refuses.
pub fn dawra_mutabaqa(asli: &[u8], silsila: &Silsila) -> Result<(), KhataNusus> {
    let muaad = uktub_silsila(silsila)?;
    if muaad == asli {
        return Ok(());
    }
    let mushtarak = asli.len().min(muaad.len());
    let mut mukhtalif = tul_u64(asli.len().abs_diff(muaad.len()));
    for khana in 0..mushtarak {
        if asli.get(khana) != muaad.get(khana) {
            mukhtalif = mukhtalif.saturating_add(1);
        }
    }
    Err(KhataNusus::DawraGhayrMutabaqa {
        sigha: SIGHA_SILSILA,
        adad: mukhtalif,
    })
}

// ---------------------------------------------------------------------------
// Scripts.rvdata2
//
// An `Array` of three-element `Array`s: an integer identifier the editor uses,
// a display name, and the script's source deflated with `Zlib::Deflate`. The
// engine inflates each in turn and evaluates it at boot, in list order, which
// is what makes appending one entry a complete and reversible delivery
// mechanism — and what makes the *position* of that entry matter, since a
// script that overrides `Bitmap#draw_text` has to load after everything that
// defines it.
// ---------------------------------------------------------------------------

/// One entry of the script list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalNass {
    /// The editor's own identifier for the entry.
    pub muarrif: i64,
    /// The name shown in the editor's script list.
    pub ism: String,
    /// The inflated source.
    pub masdar: String,
    /// Where the entry sits in the list.
    pub tarteeb: usize,
}

/// Reads the script list, inflating every entry.
///
/// # Errors
///
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the stream is not an array of
/// three-element arrays, [`KhataNusus::FakkFashil`] when an entry will not
/// inflate, [`KhataNusus::HajmMufrit`] when one expands past
/// [`AQSA_NASS_RUBY`], and [`KhataNusus::NassGhayrSalih`] when an inflated
/// script is not valid UTF-8 — refused rather than lossily converted, because
/// a replacement character inside a Ruby source file is a syntax error the
/// player meets as a crash on the title screen.
pub fn iqra_qaimat_nusus(silsila: &Silsila) -> Result<Vec<MadkhalNass>, KhataNusus> {
    let Some(banud) = silsila.anasir(silsila.jidhr()) else {
        return Err(bunya_ghayr("Scripts.rvdata2 is not an array"));
    };
    let mut qaima = Vec::with_capacity(banud.len());
    for (tarteeb, band) in banud.iter().enumerate() {
        let Some(huqul) = silsila.anasir(*band) else {
            return Err(bunya_ghayr("a script list entry is not an array"));
        };
        let muarrif = huqul
            .first()
            .and_then(|udw| silsila.sahih(*udw))
            .unwrap_or(0);
        let ism = huqul
            .get(1)
            .and_then(|udw| match silsila.uqda(*udw) {
                Some(Uqda::Nass { bayt, .. }) => Some(String::from_utf8_lossy(bayt).into_owned()),
                _ => None,
            })
            .unwrap_or_default();
        let madghut = match huqul.get(2).and_then(|udw| silsila.uqda(*udw)) {
            Some(Uqda::Nass { bayt, .. }) => bayt.clone(),
            _ => {
                return Err(bunya_ghayr(
                    "a script list entry carries no deflated source",
                ));
            },
        };
        let khaam = fukk_zlib(madghut.as_slice(), AQSA_NASS_RUBY)?;
        let masdar = String::from_utf8(khaam).map_err(|khata| KhataNusus::NassGhayrSalih {
            malaf: "Scripts.rvdata2".to_owned(),
            tarmiz: "UTF-8",
            mawqi: tul_u64(khata.utf8_error().valid_up_to()),
        })?;
        qaima.push(MadkhalNass {
            muarrif,
            ism,
            masdar,
            tarteeb,
        });
    }
    Ok(qaima)
}

/// Expands a zlib stream, bounded on both the input and the output.
fn fukk_zlib(masdar: impl std::io::Read, saqf: u64) -> Result<Vec<u8>, KhataNusus> {
    let fakk = flate2::read::ZlibDecoder::new(masdar.take(saqf));
    let mut khaam = Vec::new();
    let adad = fakk
        .take(saqf.saturating_add(1))
        .read_to_end(&mut khaam)
        .map_err(|sabab| KhataNusus::FakkFashil {
            sigha: "Scripts.rvdata2",
            tafsil: sabab.to_string(),
        })?;
    if tul_u64(adad) > saqf {
        return Err(KhataNusus::HajmMufrit {
            haql: "an inflated script",
            qeema: tul_u64(adad),
            saqf,
        });
    }
    Ok(khaam)
}

/// Deflates a script the way `Zlib::Deflate.deflate` does.
fn udghut_zlib(khaam: &[u8]) -> Result<Vec<u8>, KhataNusus> {
    use std::io::Write as _;

    let mut daght = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    daght
        .write_all(khaam)
        .map_err(|sabab| KhataNusus::FakkFashil {
            sigha: "Scripts.rvdata2",
            tafsil: sabab.to_string(),
        })?;
    daght.finish().map_err(|sabab| KhataNusus::FakkFashil {
        sigha: "Scripts.rvdata2",
        tafsil: sabab.to_string(),
    })
}

impl Silsila {
    /// Interns a value, for the one caller that builds rather than reads.
    fn adhif(&mut self, uqda: Uqda) -> Result<Muashir, KhataNusus> {
        if self.uqad.len() >= AQSA_KAINAT {
            return Err(KhataNusus::HajmMufrit {
                haql: "the objects a Marshal stream creates",
                qeema: tul_u64(self.uqad.len().saturating_add(1)),
                saqf: tul_u64(AQSA_KAINAT),
            });
        }
        let raqm = u32::try_from(self.uqad.len()).map_err(|_| KhataNusus::HajmMufrit {
            haql: "the objects a Marshal stream creates",
            qeema: tul_u64(self.uqad.len()),
            saqf: u64::from(u32::MAX),
        })?;
        self.uqad.push(uqda);
        Ok(Muashir(raqm))
    }

    /// Appends one entry to the script list.
    ///
    /// Appended rather than inserted, deliberately. The engine evaluates the
    /// list in order, and Taarib's script replaces methods that the game's own
    /// scripts — and every third-party script it ships — may also replace. Going
    /// last is the only position from which an override reliably wins, and it is
    /// also the only position whose removal cannot renumber anything a
    /// translator or a save file refers to.
    ///
    /// The identifier is derived from the entry's name so that reinstalling
    /// produces the same list twice, and is forced away from the range the
    /// editor generates so it cannot collide with a real script.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the stream is not a script
    /// list, [`KhataNusus::FakkFashil`] when the source will not deflate, and
    /// [`KhataNusus::HajmMufrit`] when the arena is full.
    pub fn arkib_nass_ruby(&mut self, ism: &str, masdar: &str) -> Result<Muashir, KhataNusus> {
        if self.anasir(self.jidhr()).is_none() {
            return Err(bunya_ghayr("Scripts.rvdata2 is not an array"));
        }
        // The name's encoding is copied from an entry the game itself wrote,
        // so the new row looks exactly like its neighbours to the editor. A
        // guess here is visible: VX Ace shows a script whose name is tagged
        // with the wrong encoding as mojibake in its own list.
        let tarmiz_ism = self.tarmiz_ism_awwal();
        let madghut = udghut_zlib(masdar.as_bytes())?;
        let muarrif = self.adhif(Uqda::Sahih(muarrif_nass(ism)))?;
        let ism_uqda = self.adhif(Uqda::Nass {
            bayt: ism.as_bytes().to_vec(),
            tarmiz: tarmiz_ism,
        })?;
        // The deflated source is binary and carries no encoding declaration,
        // exactly as the engine writes it: `Zlib::Inflate.inflate` takes bytes.
        let masdar_uqda = self.adhif(Uqda::Nass {
            bayt: madghut,
            tarmiz: Tarmiz::Thunai,
        })?;
        let saf = self.adhif(Uqda::Masfufa(vec![muarrif, ism_uqda, masdar_uqda]))?;

        let jidhr = self.jidhr();
        let khana = usize::try_from(jidhr.raqm())
            .ok()
            .and_then(|fahras| self.uqad.get_mut(fahras))
            .ok_or_else(|| bunya_ghayr("the script list's own handle is stale"))?;
        if let Uqda::Masfufa(banud) = khana {
            banud.push(saf);
        }
        Ok(saf)
    }

    /// Removes every entry whose name matches, and reports how many went.
    ///
    /// Matching on the name rather than on the identifier is what makes the
    /// uninstall safe against a game whose own scripts were renumbered by the
    /// editor between install and removal.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the stream is not a script
    /// list.
    pub fn izal_nass_ruby(&mut self, ism: &str) -> Result<usize, KhataNusus> {
        let Some(banud) = self.anasir(self.jidhr()) else {
            return Err(bunya_ghayr("Scripts.rvdata2 is not an array"));
        };
        let baqi: Vec<Muashir> = banud
            .iter()
            .copied()
            .filter(|band| {
                self.anasir(*band)
                    .and_then(|huqul| huqul.get(1).copied())
                    .and_then(|udw| match self.uqda(udw) {
                        Some(Uqda::Nass { bayt, .. }) => Some(bayt.as_slice() != ism.as_bytes()),
                        _ => None,
                    })
                    .unwrap_or(true)
            })
            .collect();
        let muzal = banud.len().saturating_sub(baqi.len());
        let jidhr = self.jidhr();
        let khana = usize::try_from(jidhr.raqm())
            .ok()
            .and_then(|fahras| self.uqad.get_mut(fahras))
            .ok_or_else(|| bunya_ghayr("the script list's own handle is stale"))?;
        *khana = Uqda::Masfufa(baqi);
        Ok(muzal)
    }

    /// The encoding declaration the game's own script names use.
    fn tarmiz_ism_awwal(&self) -> Tarmiz {
        self.anasir(self.jidhr())
            .and_then(|banud| banud.first().copied())
            .and_then(|band| self.anasir(band).and_then(|huqul| huqul.get(1).copied()))
            .and_then(|udw| match self.uqda(udw) {
                Some(Uqda::Nass { tarmiz, .. }) => Some(tarmiz.clone()),
                _ => None,
            })
            .unwrap_or(Tarmiz::Thunai)
    }
}

/// The script-list identifier for a name, deterministic and out of the
/// editor's own range.
///
/// RPG Maker's editor allocates small positive identifiers. Forcing the high
/// bit off and the value above sixteen million keeps this out of that range
/// without ever being negative, which the editor displays as a corrupt row.
#[must_use]
pub fn muarrif_nass(ism: &str) -> i64 {
    let khulasa = blake3::hash(ism.as_bytes());
    let mut arbaa = [0_u8; 4];
    if let Some(juz) = khulasa.as_bytes().get(..4) {
        arbaa.copy_from_slice(juz);
    }
    i64::from(u32::from_le_bytes(arbaa) & 0x00FF_FFFF).saturating_add(0x0100_0000)
}

// ---------------------------------------------------------------------------
// Locating the text
//
// The same database and event structures as RPG Maker MV and MZ, because they
// are the same product: `Actors`, `Classes`, `Skills`, `Items`, `Weapons`,
// `Armors`, `Enemies`, `States`, `Troops`, `CommonEvents`, every `MapXXX`, and
// `System`. What differs is only the serialization — an `RPG::Actor` here is a
// `Marshal` object with `@name` rather than a JSON row with `"name"`.
//
// Two categories are deliberately left alone:
//
// * **`@note`.** VX Ace's note field is where script authors put configuration
//   — `<hp_gain: 30>`, `<element: fire>` — and it is parsed by those scripts
//   with literal string matching. Translating it silently breaks whichever
//   feature reads it, and the player has no way to connect the two.
// * **Event codes 355 and 655.** Those are `Script` commands: their parameter
//   is Ruby source, not text. Translating source is how a patch turns a working
//   game into one that raises a `NameError` on an unrelated map.
//
// Comments (108 and 408) are skipped for the same reason as `@note`, and switch
// and variable names because the player never sees either.
// ---------------------------------------------------------------------------

/// Instance variables holding one player-visible string, by class.
const HUQUL_NASS: [(&str, &[&str]); 12] = [
    ("RPG::Actor", &["@name", "@nickname", "@description"]),
    ("RPG::Class", &["@name"]),
    (
        "RPG::Skill",
        &["@name", "@description", "@message1", "@message2"],
    ),
    ("RPG::Item", &["@name", "@description"]),
    ("RPG::Weapon", &["@name", "@description"]),
    ("RPG::Armor", &["@name", "@description"]),
    ("RPG::Enemy", &["@name"]),
    (
        "RPG::State",
        &["@name", "@message1", "@message2", "@message3", "@message4"],
    ),
    ("RPG::Troop", &["@name"]),
    ("RPG::CommonEvent", &["@name"]),
    ("RPG::Map", &["@display_name"]),
    ("RPG::Event", &["@name"]),
];

/// Instance variables holding an array of player-visible strings, by class.
const HUQUL_QAIMA: [(&str, &[&str]); 2] = [
    (
        "RPG::System",
        &["@elements", "@skill_types", "@weapon_types", "@armor_types"],
    ),
    (
        "RPG::System::Terms",
        &["@basic", "@params", "@etypes", "@commands"],
    ),
];

/// Instance variables of `RPG::System` holding one string.
const HUQUL_NIZAM: [&str; 2] = ["@game_title", "@currency_unit"];

/// Event command codes whose first parameter is a line of player-visible text.
const RUMUZ_SATR: [i64; 2] = [401, 405];

/// Event command codes whose second parameter is player-visible text.
const RUMUZ_THANI: [i64; 3] = [320, 324, 325];

/// The event command code whose first parameter is an array of choices.
const RAMZ_IKHTIYARAT: i64 = 102;

/// The event command code whose second parameter is one chosen branch's label.
const RAMZ_FAR: i64 = 402;

/// One translatable string, addressed by the handle that holds it.
///
/// A handle rather than a path, because the arena already guarantees identity:
/// replacing the string this names replaces it everywhere the stream referred
/// to it, which is what the engine's own semantics require. The other fields
/// are context for the translator and for the diagnostics bundle, not an
/// address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MustalahVxAce {
    /// Where the string lives.
    pub muashir: Muashir,
    /// Which file it came from.
    pub malaf: String,
    /// The class that held it.
    pub sanf: String,
    /// The instance variable or parameter that held it.
    pub haql: String,
    /// The event command code, when it came from one.
    pub ramz: Option<i64>,
    /// The text as the game has it.
    pub nass: String,
}

/// Whether a string is worth offering to a translator at all.
fn yustahaqq(nass: &str) -> bool {
    nass.chars()
        .any(|harf| !harf.is_whitespace() && !harf.is_ascii_punctuation())
}

/// Collects every translatable string in one decoded data file.
///
/// Walks the arena in creation order rather than descending the object graph,
/// because a VX Ace map's events refer back to the map and a recursive walk
/// over one does not terminate. Deduplicated by handle: a string the stream
/// shared between two rows is one string, and offering it twice would invite
/// two different translations of one object.
#[must_use]
pub fn iltiqat_nusus(silsila: &Silsila, malaf: &str) -> Vec<MustalahVxAce> {
    let mut mustalahat: Vec<MustalahVxAce> = Vec::new();
    let mut mashhud: BTreeSet<u32> = BTreeSet::new();
    let mut adhif = |muashir: Muashir, sanf: &str, haql: String, ramz: Option<i64>| {
        let Some(nass) = silsila.nass(muashir) else {
            return;
        };
        if !yustahaqq(nass) || !mashhud.insert(muashir.raqm()) {
            return;
        }
        mustalahat.push(MustalahVxAce {
            muashir,
            malaf: malaf.to_owned(),
            sanf: sanf.to_owned(),
            haql,
            ramz,
            nass: nass.to_owned(),
        });
    };

    for (muashir, uqda) in silsila.kul() {
        let Uqda::Kaain { .. } = uqda else {
            continue;
        };
        let Some(sanf) = silsila.sanf(muashir) else {
            continue;
        };

        if sanf == "RPG::EventCommand" {
            iltiqat_amr(silsila, muashir, &mut adhif);
            continue;
        }
        for (matlub, huqul) in HUQUL_NASS {
            if sanf != matlub {
                continue;
            }
            for haql in huqul {
                if let Some(qeema) = silsila.sifa(muashir, haql) {
                    adhif(qeema, sanf, (*haql).to_owned(), None);
                }
            }
        }
        if sanf == "RPG::System" {
            for haql in HUQUL_NIZAM {
                if let Some(qeema) = silsila.sifa(muashir, haql) {
                    adhif(qeema, sanf, haql.to_owned(), None);
                }
            }
        }
        for (matlub, huqul) in HUQUL_QAIMA {
            if sanf != matlub {
                continue;
            }
            for haql in huqul {
                let Some(saf) = silsila.sifa(muashir, haql) else {
                    continue;
                };
                let Some(anasir) = silsila.anasir(saf) else {
                    continue;
                };
                for (khana, udw) in anasir.iter().enumerate() {
                    adhif(*udw, sanf, format!("{haql}[{khana}]"), None);
                }
            }
        }
    }
    mustalahat
}

/// Collects the player-visible parameters of one `RPG::EventCommand`.
fn iltiqat_amr(
    silsila: &Silsila,
    muashir: Muashir,
    adhif: &mut impl FnMut(Muashir, &str, String, Option<i64>),
) {
    let Some(ramz) = silsila
        .sifa(muashir, "@code")
        .and_then(|udw| silsila.sahih(udw))
    else {
        return;
    };
    let Some(muamalat) = silsila
        .sifa(muashir, "@parameters")
        .and_then(|udw| silsila.anasir(udw))
    else {
        return;
    };
    let sanf = "RPG::EventCommand";

    if RUMUZ_SATR.contains(&ramz)
        && let Some(udw) = muamalat.first()
    {
        adhif(*udw, sanf, "parameters[0]".to_owned(), Some(ramz));
        return;
    }
    if RUMUZ_THANI.contains(&ramz)
        && let Some(udw) = muamalat.get(1)
    {
        adhif(*udw, sanf, "parameters[1]".to_owned(), Some(ramz));
        return;
    }
    if ramz == RAMZ_FAR
        && let Some(udw) = muamalat.get(1)
    {
        adhif(*udw, sanf, "parameters[1]".to_owned(), Some(ramz));
        return;
    }
    if ramz == RAMZ_IKHTIYARAT
        && let Some(saf) = muamalat.first()
        && let Some(anasir) = silsila.anasir(*saf)
    {
        for (khana, udw) in anasir.iter().enumerate() {
            adhif(*udw, sanf, format!("parameters[0][{khana}]"), Some(ramz));
        }
    }
}

// ---------------------------------------------------------------------------
// Delivery
// ---------------------------------------------------------------------------

/// The name the injected script carries in the editor's script list.
///
/// Also the key the uninstall matches on, which is why it is a constant rather
/// than a parameter: an install and an uninstall that disagreed about the name
/// would leave a script nothing can remove.
pub const ISM_NASS_TAARIB: &str = "Taarib";

/// Where the in-engine Ruby reads its settings from, relative to the game root.
///
/// A `key=value` text file, and that is not a stylistic choice. RGSS3 embeds
/// Ruby 1.9.2 with almost none of the standard library and no JSON parser at
/// all, so a settings file the injected script can read without `eval` has to
/// be something twenty lines of Ruby can parse. `eval` on patch content is
/// exactly what this crate does not do.
pub const MALAF_IDAD: &str = "Taarib/idad.txt";

/// Where the atlas the takeover blits from lives, relative to the game root.
///
/// The atlas is a bitmap the patch ships and the Ruby side loads with
/// `Bitmap.new`. It is *not* read from the native library's runtime atlas: RGSS3
/// can only get pixels into a `Bitmap` one `set_pixel` call at a time, and a
/// two-thousand-pixel-square page is four million calls. Rasterizing at patch
/// time and blitting at run time is the only arrangement this engine can
/// actually sustain.
pub const MALAF_LAWHA: &str = "Taarib/lawha.png";

/// The glyph rectangle table that goes with the atlas.
pub const MALAF_JADWAL: &str = "Taarib/lawha.tbl";

/// What the patch decided, as the injected Ruby reads it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdadVxAce {
    /// The rung, as its stable number.
    pub rutba: u8,
    /// How sure the probe was, 0..=100.
    pub thiqa: u8,
    /// The font the patch installed, relative to the game root.
    pub khatt: String,
    /// The size the interface draws at.
    pub hajm: u32,
    /// Where the native library lives, relative to the game root.
    pub dalil_jisr: String,
    /// Every line the probe produced, so the game's own log can explain itself.
    pub athar: Vec<String>,
}

impl IdadVxAce {
    /// Settings for one game.
    #[must_use]
    pub fn jadeed(rutba: Rutba, thiqa: u8, khatt: impl Into<String>) -> Self {
        Self {
            rutba: rutba as u8,
            thiqa,
            khatt: khatt.into(),
            hajm: 24,
            dalil_jisr: "Taarib".to_owned(),
            athar: Vec::new(),
        }
    }

    /// The settings file's own text.
    ///
    /// One `key=value` per line, values with every control character stripped.
    /// The stripping is not cosmetic: a newline inside a value would end the
    /// line early and the next fragment would be read as a key, so a font path
    /// containing one could silently change a different setting.
    #[must_use]
    pub fn nass(&self) -> String {
        let munaqqa = |qeema: &str| -> String {
            qeema
                .chars()
                .filter(|harf| !harf.is_control() && *harf != '=')
                .collect()
        };
        let mut nass = String::with_capacity(256);
        nass.push_str("# taarib:generated — settings for the injected VX Ace script.\n");
        nass.push_str("# Deleting the Taarib script entry removes the patch completely.\n");
        let _ = writeln!(nass, "rutba={}", self.rutba);
        let _ = writeln!(nass, "thiqa={}", self.thiqa);
        let _ = writeln!(nass, "khatt={}", munaqqa(&self.khatt));
        let _ = writeln!(nass, "hajm={}", self.hajm);
        let _ = writeln!(nass, "dalil_jisr={}", munaqqa(&self.dalil_jisr));
        let _ = writeln!(nass, "lawha={MALAF_LAWHA}");
        let _ = writeln!(nass, "jadwal={MALAF_JADWAL}");
        for (raqm, satr) in self.athar.iter().enumerate() {
            let _ = writeln!(nass, "athar{raqm}={}", munaqqa(satr));
        }
        nass
    }
}

/// Where a game keeps its script list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MawqiNusus {
    /// Loose in `Data/Scripts.rvdata2`, which is what an undeployed project
    /// looks like and what most itch.io uploads are.
    Malaf(PathBuf),
    /// Inside `Game.rgss3a`, which is what a deployed game looks like.
    Hawiya {
        /// Where the archive is.
        masar: PathBuf,
        /// The member's path inside it.
        udw: String,
    },
}

/// Finds the script list, loose or archived.
///
/// The loose file wins when both exist, and that is the engine's own rule
/// rather than a preference: RGSS3 mounts the archive and then lets a real file
/// on disk shadow a member of the same name, so a project with both runs the
/// loose one. A patcher that chose the archive there would produce a patch that
/// installs successfully and has no effect.
#[must_use]
pub fn ayn_nusus(jidhr: &Path) -> Option<MawqiNusus> {
    let masar = masar_bila_hala(jidhr, "Data/Scripts.rvdata2");
    if masar.is_file() {
        return Some(MawqiNusus::Malaf(masar));
    }
    let masar = masar_bila_hala(jidhr, ISM_HAWIYA);
    if masar.is_file() {
        return Some(MawqiNusus::Hawiya {
            masar,
            udw: "Data/Scripts.rvdata2".to_owned(),
        });
    }
    None
}

/// Writes the settings file, which this patch owns outright.
///
/// [`Hafiz::ansha`] rather than [`Hafiz::iktub`] because `MALAF_IDAD` is a file
/// Taarib brings into existence: it has no original to preserve, and undoing it
/// is a delete rather than a restore. Recording it as a modification would arm
/// the uninstaller to write an empty "original" over it; recording it as an
/// addition is what makes [`izal`] able to remove it and nothing else.
fn uktub_idad(hafiz: &mut dyn Hafiz, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus> {
    if let Some(walid) = masar.parent() {
        hafiz.ansha_mujallad(walid)?;
    }
    hafiz.ansha(masar, bayt)
}

/// Installs the Ruby script and the settings file.
///
/// The order is fixed and it matters:
///
/// 1. read the script list, from wherever it actually is;
/// 2. **check the round trip before touching anything** — if this build cannot
///    reproduce the file it just read, it stops here, and the game is exactly as
///    it was;
/// 3. append the entry;
/// 4. write the settings file;
/// 5. write the script list back, atomically, rebuilding the archive when the
///    list came out of one.
///
/// Step two is the whole reversibility argument in one line. Everything after
/// it can be undone by [`izal`]; nothing before it changed anything.
///
/// This function does not take a backup *itself*, and it does not need to: it
/// holds a [`Hafiz`], and every write below goes through it, so the original is
/// preserved and its manifest line flushed before any byte is modified. A
/// patcher that took its own backup would be a second, unregistered copy that
/// nothing knows how to restore from — which is precisely why the guard is
/// passed in rather than constructed here.
///
/// # Errors
///
/// [`KhataNusus::HimlMarfud`] when no script list could be found — a rung
/// declining rather than a failure, so the caller can descend —
/// [`KhataNusus::DawraGhayrMutabaqa`] when the round trip does not match,
/// [`KhataNusus::NuskhaMafquda`] when an original cannot be preserved or the
/// settings file's path is already occupied, and whatever the archive,
/// `Marshal` and file layers refuse.
pub fn rakkib(
    hafiz: &mut dyn Hafiz,
    jidhr: &Path,
    masdar_ruby: &str,
    idad: &IdadVxAce,
) -> Result<Vec<PathBuf>, KhataNusus> {
    let Some(mawqi) = ayn_nusus(jidhr) else {
        return Err(KhataNusus::HimlMarfud {
            alia: "the VX Ace script list",
            sabab: format!(
                "neither Data/Scripts.rvdata2 nor {ISM_HAWIYA} is present under {}",
                jidhr.display()
            ),
        });
    };

    let mut maktub: Vec<PathBuf> = Vec::new();
    let masar_idad = masar_bila_hala(jidhr, MALAF_IDAD);

    match mawqi {
        MawqiNusus::Malaf(masar) => {
            let asli = fs::read(&masar).map_err(|sabab| khata_malaf(&masar, sabab))?;
            let mut silsila = iqra_silsila(&asli)?;
            dawra_mutabaqa(&asli, &silsila)?;
            let _ = silsila.izal_nass_ruby(ISM_NASS_TAARIB)?;
            let _ = silsila.arkib_nass_ruby(ISM_NASS_TAARIB, masdar_ruby)?;
            uktub_idad(hafiz, &masar_idad, idad.nass().as_bytes())?;
            maktub.push(masar_idad);
            hafiz.iktub(&masar, &uktub_silsila(&silsila)?)?;
            maktub.push(masar);
        },
        MawqiNusus::Hawiya { masar, udw } => {
            let hawiya = Hawiya::iqra(&masar)?;
            let madkhal = hawiya
                .madkhal(&udw)
                .cloned()
                .ok_or_else(|| KhataNusus::HimlMarfud {
                    alia: "the VX Ace script list",
                    sabab: format!("{ISM_HAWIYA} carries no {udw}"),
                })?;
            let asli = hawiya.istakhrij(&madkhal)?;
            let mut silsila = iqra_silsila(&asli)?;
            dawra_mutabaqa(&asli, &silsila)?;
            let _ = silsila.izal_nass_ruby(ISM_NASS_TAARIB)?;
            let _ = silsila.arkib_nass_ruby(ISM_NASS_TAARIB, masdar_ruby)?;
            let jadeed = uktub_silsila(&silsila)?;

            // Every other member is named rather than held: a rebuild that
            // loaded the whole archive to replace one file in it would need as
            // much memory as the game is large.
            let mut aada: Vec<(String, MasdarUdw)> = Vec::with_capacity(hawiya.madakhil().len());
            for sabiq in hawiya.madakhil() {
                if sabiq.masar == madkhal.masar {
                    aada.push((sabiq.masar.clone(), MasdarUdw::Dhakira(jadeed.clone())));
                } else {
                    aada.push((sabiq.masar.clone(), MasdarUdw::MinHawiya(sabiq.clone())));
                }
            }
            uktub_idad(hafiz, &masar_idad, idad.nass().as_bytes())?;
            maktub.push(masar_idad);
            uktub_hawiya(hafiz, &masar, Some(&hawiya), &aada, hawiya.badhra)?;
            dawra_hawiya(&masar, &aada)?;
            maktub.push(masar);
        },
    }
    Ok(maktub)
}

/// Removes the injected script and the settings file, and nothing else.
///
/// Reports what it removed rather than a count, so the caller can record
/// exactly which files changed. A game whose script list does not contain a
/// Taarib entry is not an error: uninstalling twice succeeds and does nothing
/// the second time, which is what an uninstall run after a failed install has
/// to do.
///
/// # Errors
///
/// [`KhataNusus::HimlMarfud`] when no script list could be found, and whatever
/// the archive, `Marshal` and file layers refuse.
pub fn izal(hafiz: &mut dyn Hafiz, jidhr: &Path) -> Result<Vec<PathBuf>, KhataNusus> {
    let mut muzala: Vec<PathBuf> = Vec::new();
    let masar_idad = masar_bila_hala(jidhr, MALAF_IDAD);
    if hafiz.ihdhif(&masar_idad)? {
        muzala.push(masar_idad);
    }

    let Some(mawqi) = ayn_nusus(jidhr) else {
        return Err(KhataNusus::HimlMarfud {
            alia: "the VX Ace script list",
            sabab: format!("no script list is present under {}", jidhr.display()),
        });
    };
    match mawqi {
        MawqiNusus::Malaf(masar) => {
            let asli = fs::read(&masar).map_err(|sabab| khata_malaf(&masar, sabab))?;
            let mut silsila = iqra_silsila(&asli)?;
            if silsila.izal_nass_ruby(ISM_NASS_TAARIB)? == 0 {
                return Ok(muzala);
            }
            hafiz.iktub(&masar, &uktub_silsila(&silsila)?)?;
            muzala.push(masar);
        },
        MawqiNusus::Hawiya { masar, udw } => {
            let hawiya = Hawiya::iqra(&masar)?;
            let Some(madkhal) = hawiya.madkhal(&udw).cloned() else {
                return Ok(muzala);
            };
            let asli = hawiya.istakhrij(&madkhal)?;
            let mut silsila = iqra_silsila(&asli)?;
            if silsila.izal_nass_ruby(ISM_NASS_TAARIB)? == 0 {
                return Ok(muzala);
            }
            let jadeed = uktub_silsila(&silsila)?;
            let mut aada: Vec<(String, MasdarUdw)> = Vec::with_capacity(hawiya.madakhil().len());
            for sabiq in hawiya.madakhil() {
                if sabiq.masar == madkhal.masar {
                    aada.push((sabiq.masar.clone(), MasdarUdw::Dhakira(jadeed.clone())));
                } else {
                    aada.push((sabiq.masar.clone(), MasdarUdw::MinHawiya(sabiq.clone())));
                }
            }
            uktub_hawiya(hafiz, &masar, Some(&hawiya), &aada, hawiya.badhra)?;
            dawra_hawiya(&masar, &aada)?;
            muzala.push(masar);
        },
    }
    Ok(muzala)
}

/// Rewrites one data file's translatable strings in place.
///
/// `tarjamat` maps a handle to its Arabic. Handles come from
/// [`iltiqat_nusus`] over *this* stream: a handle from another one is refused
/// rather than applied, because arena indices are only meaningful inside the
/// stream that issued them and applying one across streams would rewrite an
/// arbitrary object.
///
/// The round trip is checked against the untouched original before any
/// replacement is made, so a file this build cannot reproduce is left exactly
/// as it was found.
///
/// # Errors
///
/// [`KhataNusus::DawraGhayrMutabaqa`] when the untouched round trip does not
/// match, [`KhataNusus::BunyaGhayrMutawaqqaa`] when a handle is not a string in
/// this stream, and [`KhataNusus::KhataMalaf`] for any failed read or write.
pub fn ahill_nusus(
    hafiz: &mut dyn Hafiz,
    masar: &Path,
    tarjamat: &BTreeMap<u32, String>,
) -> Result<usize, KhataNusus> {
    let asli = fs::read(masar).map_err(|sabab| khata_malaf(masar, sabab))?;
    let mut silsila = iqra_silsila(&asli)?;
    dawra_mutabaqa(&asli, &silsila)?;

    let mut mubaddal = 0_usize;
    for (raqm, jadeed) in tarjamat {
        silsila.baddil_nass(Muashir(*raqm), jadeed)?;
        mubaddal = mubaddal.saturating_add(1);
    }
    if mubaddal == 0 {
        return Ok(0);
    }
    hafiz.iktub(masar, &uktub_silsila(&silsila)?)?;
    Ok(mubaddal)
}
