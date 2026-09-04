//! جيم‌ميكر — the `data.win` FORM container, and the offset problem that is the
//! whole difficulty of patching it.
//!
//! One file holds the entire game. `data.win` on Windows, `game.unx` on Linux,
//! `game.ios` and `game.droid` elsewhere: a four-byte `FORM` magic, a total
//! length, and then a flat sequence of chunks, each an eight-byte header —
//! `[4-byte name][u32 payload length]` — followed by its payload. Phase 5's
//! detector already walks that table without reading a payload; see
//! [`taarib_muharrik::dalail::nusus`] for the walk, the container filenames it
//! tries and the `GEN8` layout it reads. This module does not restate any of
//! that. It picks the container up where the detector put it down and turns it
//! into something that can be rewritten.
//!
//! ## The one hard problem
//!
//! **Every string reference in the container is an absolute file offset.**
//!
//! Not an index into a table, not a chunk-relative offset — an absolute byte
//! position in `data.win` pointing at the string's UTF-8 bytes. `GEN8` names the
//! game that way. Every `FONT` entry names itself that way. Every `CODE` entry,
//! every `VARI` and `FUNC` row, every room, object and sprite name is a `u32`
//! holding a position in the file.
//!
//! So the obvious patch — replace an English string with a longer Arabic one —
//! moves every byte after it, and every one of those `u32` fields is now off by
//! the difference. A container written with one of them wrong is a game that
//! does not start, and there is no partial success to salvage: this is why
//! [`crate::khata`]'s posture is refusal, and why nothing here writes into a
//! game's directory until [`MuhawwilGameMaker::tahaqquq_dawra`] has said the
//! bytes it produced are the bytes it meant.
//!
//! ## The rewriting strategy, precisely
//!
//! Three mechanisms, tried in this order. The planner picks the first that
//! applies, records which one it picked in [`WasfIstratijiya`], and refuses when
//! none of them does. They are ordered by how much of the file they disturb, and
//! the first two disturb none of it.
//!
//! ### 1. `Mawdi` — in place, zero delta
//!
//! When the replacement bytes are no longer than the bytes they replace, they
//! are written into the same span and the remainder is padded with `NUL`. Not
//! one offset in the file changes, so every reference — including every
//! reference inside a chunk this build does not parse — is still correct without
//! being touched. This is the only strategy that is safe against chunks nobody
//! has enumerated, and it is therefore the one the planner wants.
//!
//! Its limit is exactly what it sounds like: Arabic is not shorter than English.
//!
//! ### 2. `Ilhaq` — append and repoint, zero delta
//!
//! The observation that makes this format tractable: **a structure addressed by
//! an absolute pointer can live anywhere the pointer can reach.** A string, a
//! `FONT` entry, a glyph record and a `TXTR` blob are all reached through a
//! `u32` that this module knows the position of. So the new bytes are appended
//! in a fresh chunk past the last chunk in the container, and the single `u32`
//! that named the old structure is overwritten with the new absolute offset.
//!
//! Nothing before the appended region moves. The only bytes that change outside
//! it are the pointer fields the plan listed, the `FORM` header's total length,
//! and the new chunk's own eight-byte header. Every unparsed chunk stays valid
//! because none of its bytes and none of its targets moved.
//!
//! This does not work for everything, and the exceptions are named rather than
//! discovered: `STRG`'s own pointer array is read positionally by index, so a
//! string reached *through the pool's index* cannot be relocated this way — only
//! one reached through a pointer field elsewhere can. `TPAG` items are likewise
//! addressed by pointer but their *count* is positional, so an item may be
//! repointed and may not be added.
//!
//! ### 3. `Izaha` — relocate everything past a frontier
//!
//! The last resort, and the one with a precondition this build actually checks.
//! A chunk grows by `delta`; every byte at or after the end of that chunk moves
//! forward by `delta`; every reference whose target is at or after that frontier
//! is rewritten to `target + delta`; the `FORM` length and the grown chunk's
//! length header are increased by `delta`.
//!
//! **The precondition:** every chunk at or after the frontier must be one whose
//! absolute pointers this build enumerates exhaustively — see
//! [`qita_qabila_lilizaha`]. In every generation of this format the layout order
//! puts `STRG` next to last, with only `TXTR` and `AUDO` behind it, and both of
//! those are a count, a pointer array and a run of blobs. That is why relocating
//! a grown string pool is tractable at all and why relocating a grown `FONT`
//! chunk — which sits in front of `TPAG`, `CODE`, `VARI`, `FUNC`, `OBJT` and
//! `ROOM` — is not, and is refused rather than attempted.
//!
//! `delta` is additionally rounded up to [`MUHADHAHA_SAFHA`] so that every
//! structure past the frontier keeps the alignment the runtime's texture uploads
//! depend on. A texture blob that arrives one byte off its 128-byte boundary is
//! a game that renders garbage on some drivers and not others.
//!
//! ### The census that makes all three checkable
//!
//! [`JadwalMaraji`] is built once, at parse time, and holds every 4-byte field
//! in the container that this build believes is an absolute offset, tagged with
//! [`NawMarja`] for what it points at. A field this build did not classify is
//! [`NawMarja::Majhul`] and is never written — its correctness is guaranteed
//! only by the first two strategies moving nothing, which is precisely why they
//! are preferred and why the third has a precondition.
//!
//! ## The font path
//!
//! GameMaker fonts are **baked glyph tables**. A `FONT` entry carries a name, a
//! size, bold and italic flags, a charset byte, an antialiasing byte, a pointer
//! to the `TPAG` item holding its texture region, and a glyph list — and each
//! glyph is a character code, a rectangle on the page, an offset, a shift
//! (advance) and a per-character kerning list. There is no font file to swap and
//! no shaping stage to configure: the container ships pictures of letters and an
//! index from character codes to pictures.
//!
//! Taarib replaces that table with one built from its own rasterized atlas,
//! appends the atlas page to `TXTR`, and repoints the font's `TPAG` item at it.
//! The entry itself is relocated by strategy 2, so the `FONT` chunk's size never
//! changes and nothing in front of `TPAG` moves.
//!
//! ## Why the glyph transport, and why it is not presentation forms
//!
//! The glyph table is indexed **by character code**. Shaped output is indexed
//! **by glyph identifier**. Something has to bridge those, and the bridge is
//! [`taarib_lawha::naql`] — read its module header before this paragraph, since
//! the argument lives there and is not repeated here at length.
//!
//! In one paragraph: the real logical text is shaped once, by real HarfRust, on
//! the real font's `GSUB` and `GPOS`. Each distinct `(font, size, glyph id)` that
//! comes out is assigned a private-use codepoint, the generated glyph table's
//! entry for that codepoint is Taarib's own image of that exact glyph, and the
//! line is emitted as those codepoints in visual order. A presentation-form
//! pipeline maps codepoints to codepoints and must throw the shaping away to do
//! it, losing every ligature, every contextual alternate and every mark
//! attachment the font would have produced. This maps already-shaped glyph
//! identifiers to opaque addresses. Nothing re-derives anything.
//!
//! **The structural guarantee is preserved here and not merely inherited.**
//! Every slot this module looks up is keyed with
//! [`MiftahKhana::min_harf`][taarib_lawha::naql::MiftahKhana::min_harf] from a
//! [`Harf`] — a shaped glyph — and there is no function in this file that takes
//! a `char` and returns a slot. In the other direction there is no inverse: a
//! slot goes into the glyph table and into the sequence handed to the engine,
//! and nowhere else. No path from a character to a slot; no path from a slot
//! back to text.
//!
//! ## What it costs the player, stated before anything is installed
//!
//! Transported text is **no longer text**. It is not searchable, not selectable,
//! not copyable, and not readable by any assistive technology, and it cannot be
//! re-wrapped or concatenated. [`WasfIstratijiya::yukallif`] is true whenever the
//! transport was used and [`WasfIstratijiya::takaleef`] is the sentence a caller
//! is required to show. This is the last rung of the ladder; see
//! [`crate::tabaqa`] for the three and for why guessing between them is refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{ErrorKind, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use taarib_lawha::khareeta::MawdiShakl;
use taarib_lawha::naql::{HawdKhanat, MasdarLawha, MiftahKhana, NamatKhana, TawzeeKhanat};
use taarib_muharrik::dalail::nusus::HadafNusus;
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_saff::natija::Harf;

use crate::hifz::Hafiz;
use crate::khata::{KhataNusus, hajm_usize, tul_u64};
use crate::tabaqa::{Dalil, Mifhas, Rutba, SiyaqTabaqa};
use crate::tarkeeb::{ibn_bila_hala, masar_bila_hala};

// ---------------------------------------------------------------------------
// The format's own numbers
// ---------------------------------------------------------------------------

/// The four bytes every container begins with.
pub const SIHR_FORM: [u8; 4] = *b"FORM";

/// The name this format is called in a refusal a person reads.
pub const SIGHA: &str = "GameMaker FORM";

/// How many bytes a chunk header occupies: four of name, four of length.
pub const HAJM_TARWISAT_QITA: u64 = 8;

/// The lowest bytecode version in `GEN8` this build reads.
///
/// Thirteen is GameMaker Studio 1.4's, which is the oldest runtime still
/// shipping games people buy. Below it the `FONT` entry has no scale fields and
/// the `TPAG` item is a different width, and a reader that guessed at either
/// would produce a glyph table pointing at the wrong rectangles.
pub const ISDAR_ADNA: u8 = 13;

/// The highest bytecode version this build reads.
///
/// Seventeen, which every GameMaker Studio 2 runtime from 2.0 through the 2024
/// releases writes. A container declaring more is refused with
/// [`KhataNusus::IsdarGhayrMadum`] rather than parsed on the assumption that
/// nothing moved — the `FONT` entry gained three fields across the 2.3, 2022.2
/// and 2023.2 releases alone, and each of them shifts the glyph list.
pub const ISDAR_AQSA: u8 = 17;

/// The largest container this build will read into memory.
///
/// Four gibibytes minus one is what a `u32` chunk length can address, and it is
/// also roughly the largest `data.win` that exists. The ceiling is checked
/// against the file's real length before a byte is reserved.
pub const AQSA_HAJM_HAWIYA: u64 = 0xFFFF_FFFF;

/// How many chunks the reader will accept before refusing the table.
///
/// A real container has between fifteen and forty. The same ceiling Phase 5
/// uses, for the same reason: a chunk table whose lengths are nonsense would
/// otherwise be walked forever.
pub const AQSA_QITA: usize = 256;

/// How many entries one directory listing here will look at.
///
/// The probe lists a game's root to find an application bundle and to look for
/// shipped shaping libraries. A game directory with more than four thousand
/// loose files exists, and the listing is bounded rather than trusted for the
/// same reason every other count in this module is.
pub const AQSA_MADAKHIL: usize = 4_096;

/// How many strings the pool may declare.
///
/// A text-heavy game reaches the low tens of thousands. A quarter of a million
/// is far past that and far below what a declared count could ask this process
/// to allocate.
pub const AQSA_NUSUS_HAWD: u32 = 262_144;

/// The longest single string the pool may declare, in bytes.
///
/// Four mebibytes. GameMaker games do embed whole JSON documents and whole
/// shader sources as strings, which is why this is not measured in kilobytes.
pub const AQSA_TUL_NASS: u32 = 4 * 1024 * 1024;

/// How many fonts the `FONT` chunk may declare.
pub const AQSA_KHUTUT: u32 = 4_096;

/// How many glyphs one font's table may declare.
///
/// A CJK font baked into a container reaches several thousand; a transported
/// Arabic table with four sizes reaches a few thousand more. Sixty-five thousand
/// is the ceiling the `u16` character code implies anyway.
pub const AQSA_ASHKAL_KHATT: u32 = 65_536;

/// How many kerning pairs one glyph may declare.
pub const AQSA_AZWAJ_SHAKL: u16 = 4_096;

/// How many texture pages the `TXTR` chunk may declare.
pub const AQSA_SAFAHAT: u32 = 4_096;

/// How many items the `TPAG` chunk may declare.
pub const AQSA_BUNUD_TPAG: u32 = 262_144;

/// The alignment every `TXTR` blob sits on.
///
/// One hundred and twenty-eight bytes. The runtime uploads a page straight from
/// the mapped file on some backends, and a blob that arrives off its boundary
/// renders garbage on some drivers and correctly on others — which is the worst
/// possible failure, because it reproduces on one machine in five.
pub const MUHADHAHA_SAFHA: u64 = 128;

/// The alignment a chunk payload sits on.
pub const MUHADHAHA_QITA: u64 = 16;

/// The name of the chunk Taarib appends its own bytes in.
///
/// A chunk name the runtime does not know, holding relocated font entries,
/// glyph tables and string bodies. It is placed after the last chunk in the
/// container so that nothing before it moves; see this module's header for the
/// append strategy and for what it costs.
pub const ISM_QITA_ILHAQ: [u8; 4] = *b"TARB";

/// The chunk that carries the container's general information.
pub const ISM_GEN8: [u8; 4] = *b"GEN8";
/// The chunk that carries the string pool.
pub const ISM_STRG: [u8; 4] = *b"STRG";
/// The chunk that carries the font entries.
pub const ISM_FONT: [u8; 4] = *b"FONT";
/// The chunk that carries the texture pages.
pub const ISM_TXTR: [u8; 4] = *b"TXTR";
/// The chunk that carries the texture page items.
pub const ISM_TPAG: [u8; 4] = *b"TPAG";
/// The chunk that carries the bytecode.
pub const ISM_CODE: [u8; 4] = *b"CODE";
/// The chunk that carries the variable table.
pub const ISM_VARI: [u8; 4] = *b"VARI";
/// The chunk that carries the function table.
pub const ISM_FUNC: [u8; 4] = *b"FUNC";
/// The chunk that carries the audio blobs.
pub const ISM_AUDO: [u8; 4] = *b"AUDO";
/// The chunk that carries the extension table.
pub const ISM_EXTN: [u8; 4] = *b"EXTN";

/// Whether a chunk is one whose absolute pointers this build enumerates.
///
/// The precondition of the relocating strategy, and the reason it is safe when
/// it applies. `STRG`, `TXTR` and `AUDO` are each a count, a pointer array and a
/// run of blobs, and this module knows where every pointer in them is. Anything
/// else at or after a relocation frontier means there are `u32` fields in the
/// moved region that nobody has classified, and moving the region would leave
/// them pointing four kilobytes short of where they meant to.
///
/// Deliberately a function over a name rather than a set a caller can extend: a
/// list somebody appends to in order to make one game work is a list that turns
/// this precondition into a formality.
#[must_use]
pub const fn qita_qabila_lilizaha(ism: [u8; 4]) -> bool {
    matches!(&ism, b"STRG" | b"TXTR" | b"AUDO" | b"TARB")
}

// ---------------------------------------------------------------------------
// Bounded reading and writing
//
// Every field in this module is read through one of these. None of them can
// read past the end of the buffer and none of them can panic: an offset that
// leaves the container yields `None`, and the caller turns that into a named
// refusal rather than into an index that would have been out of range.
// ---------------------------------------------------------------------------

/// A little-endian `u16` at a byte offset, bounds-checked.
fn iqra_u16(bayt: &[u8], izaha: usize) -> Option<u16> {
    let nihaya = izaha.checked_add(2)?;
    bayt.get(izaha..nihaya).and_then(|juz| <[u8; 2]>::try_from(juz).ok()).map(u16::from_le_bytes)
}

/// A little-endian `u32` at a byte offset, bounds-checked.
fn iqra_u32(bayt: &[u8], izaha: usize) -> Option<u32> {
    let nihaya = izaha.checked_add(4)?;
    bayt.get(izaha..nihaya).and_then(|juz| <[u8; 4]>::try_from(juz).ok()).map(u32::from_le_bytes)
}

/// A signed little-endian `i16` at a byte offset, bounds-checked.
fn iqra_i16(bayt: &[u8], izaha: usize) -> Option<i16> {
    iqra_u16(bayt, izaha).map(|khaam| i16::from_le_bytes(khaam.to_le_bytes()))
}

/// A four-byte name at a byte offset, bounds-checked.
fn iqra_ism(bayt: &[u8], izaha: usize) -> Option<[u8; 4]> {
    let nihaya = izaha.checked_add(4)?;
    bayt.get(izaha..nihaya).and_then(|juz| <[u8; 4]>::try_from(juz).ok())
}

/// Writes a little-endian `u32` at a byte offset, or reports that it could not.
///
/// Returns `false` rather than writing a short field. A partial write into a
/// pointer array is a container that points somewhere nobody chose, which is
/// worse than a refusal by exactly the margin between a game that starts and a
/// game that does not.
fn uktub_u32(bayt: &mut [u8], izaha: usize, qeema: u32) -> bool {
    let Some(nihaya) = izaha.checked_add(4) else {
        return false;
    };
    match bayt.get_mut(izaha..nihaya) {
        Some(khana) => {
            khana.copy_from_slice(&qeema.to_le_bytes());
            true
        }
        None => false,
    }
}

/// Writes a little-endian `u16` at a byte offset, or reports that it could not.
fn uktub_u16(bayt: &mut [u8], izaha: usize, qeema: u16) -> bool {
    let Some(nihaya) = izaha.checked_add(2) else {
        return false;
    };
    match bayt.get_mut(izaha..nihaya) {
        Some(khana) => {
            khana.copy_from_slice(&qeema.to_le_bytes());
            true
        }
        None => false,
    }
}

/// Rounds a length up to a boundary, or [`None`] on overflow.
///
/// `checked_rem` and `checked_sub` rather than `%` and `-`: the workspace denies
/// bare integer division, and a modulus written with the operator would be the
/// one place in this file where a zero divisor is a panic instead of an error.
fn muhadhah(qeema: u64, hadd: u64) -> Option<u64> {
    let baqi = qeema.checked_rem(hadd)?;
    if baqi == 0 {
        return Some(qeema);
    }
    qeema.checked_add(hadd.checked_sub(baqi)?)
}

/// A `u32` from a `u64` offset, or a named refusal when the container has grown
/// past what its own pointer width can address.
fn izaha_u32(qeema: u64) -> Result<u32, KhataNusus> {
    u32::try_from(qeema).map_err(|_| KhataNusus::HajmMufrit {
        haql: "an absolute container offset",
        qeema,
        saqf: AQSA_HAJM_HAWIYA,
    })
}

/// A finite `f32` rounded to a whole pixel, or [`None`].
///
/// A float-to-integer `as` is `cast_possible_truncation`, `cast_possible_wrap`
/// and `cast_sign_loss` at once, and it saturates silently — a coordinate that
/// came out of a corrupt layout as `1e30` would become `i32::MAX` and be baked
/// into a glyph table rather than refused. Rust offers no `TryFrom<f32>`, so the
/// conversion is done on the bits: `round` first, which makes the value
/// integral, after which a normal `f32` is exactly
/// `(2^23 + mantissa) * 2^(exponent - 150)` and the shift discards only zeros.
fn sahih_min_ashri(qeema: f32) -> Option<i32> {
    if !qeema.is_finite() {
        return None;
    }
    let bitat = qeema.round().to_bits();
    let salib = (bitat & 0x8000_0000) != 0;
    let uss = (bitat >> 23) & 0xFF;
    let kasr = bitat & 0x007F_FFFF;

    // A biased exponent of zero is a zero or a subnormal, and every subnormal is
    // far below half a pixel, so `round` has already made it zero.
    if uss == 0 {
        return Some(0);
    }
    let asas = u64::from(kasr) | (1_u64 << 23);
    let izaha = i32::try_from(uss).ok()?.checked_sub(150)?;
    let mutlaq: u64 = if izaha >= 0 {
        let khatawat = u32::try_from(izaha).ok()?;
        // The mantissa is below 2^24, so a shift under forty cannot overflow a
        // `u64`; at or above it the value is already past any pixel coordinate.
        if khatawat >= 40 {
            return None;
        }
        asas.checked_shl(khatawat)?
    } else {
        let khatawat = u32::try_from(izaha.checked_neg()?).ok()?;
        if khatawat >= 64 {
            return Some(0);
        }
        asas.checked_shr(khatawat)?
    };
    let mahdud = i64::try_from(mutlaq).ok()?;
    i32::try_from(if salib { mahdud.checked_neg()? } else { mahdud }).ok()
}

// ---------------------------------------------------------------------------
// Which generation of the format this is
// ---------------------------------------------------------------------------

/// Which runtime generation wrote a container.
///
/// Derived from the `GEN8` bytecode version and nothing else. The runtime
/// version string beside it is marketing — a 2022.11 export and a 2.3.7 export
/// can write the same chunk layout — and the bytecode number is the field the
/// layout actually depends on. Reading it first, before any other chunk, is the
/// same discipline `taarib-ruqaa`'s header applies: a parse that reads fields
/// and checks the version afterwards has already read fields whose meaning it
/// guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JeelHawiya {
    /// Bytecode 13 and 14: GameMaker Studio 1.4.
    ///
    /// `FONT` entries stop after the scale pair, `TPAG` items are twenty-two
    /// bytes, and `GEN8` has no debugger port.
    Studio1,
    /// Bytecode 15 and 16: GameMaker Studio 2.0 through 2.2.
    ///
    /// `GEN8` gains the debugger port; the font entry is unchanged.
    Studio2,
    /// Bytecode 17: GameMaker Studio 2.3 and every release after it.
    ///
    /// The `FONT` entry gains an ascender offset, and later point releases add
    /// an ascender, an SDF spread and a line height. Those are read as optional
    /// trailing fields against the entry's own extent rather than against a
    /// release number this build cannot see.
    Studio2Muhaddath,
}

impl JeelHawiya {
    /// The generation a bytecode version belongs to.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::IsdarGhayrMadum`] outside [`ISDAR_ADNA`]..=[`ISDAR_AQSA`].
    /// Refused rather than read on the assumption that nothing moved: three
    /// separate fields were inserted into the `FONT` entry across the 2.3,
    /// 2022.2 and 2023.2 releases, and each of them shifts the glyph list that
    /// this module rebuilds.
    pub fn min_bytecode(bytecode: u8) -> Result<Self, KhataNusus> {
        match bytecode {
            13 | 14 => Ok(Self::Studio1),
            15 | 16 => Ok(Self::Studio2),
            17 => Ok(Self::Studio2Muhaddath),
            _ => Err(KhataNusus::IsdarGhayrMadum {
                sigha: SIGHA,
                wujid: u32::from(bytecode),
                adna: u32::from(ISDAR_ADNA),
                aqsa: u32::from(ISDAR_AQSA),
            }),
        }
    }

    /// The generation's name, as a log line and the capability report write it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Studio1 => "GameMaker Studio 1.4",
            Self::Studio2 => "GameMaker Studio 2.0-2.2",
            Self::Studio2Muhaddath => "GameMaker Studio 2.3 or newer",
        }
    }

    /// Whether `GEN8` carries a debugger port after the Steam application id.
    #[must_use]
    pub const fn lahu_manfath_tanqih(self) -> bool {
        !matches!(self, Self::Studio1)
    }

    /// Whether a `FONT` entry carries an ascender offset after its scale pair.
    #[must_use]
    pub const fn lahu_izahat_suud(self) -> bool {
        matches!(self, Self::Studio2Muhaddath)
    }
}

// ---------------------------------------------------------------------------
// The chunk table
// ---------------------------------------------------------------------------

/// One chunk of the container.
///
/// ```text
/// FORM header — 8 bytes at file offset 0
///
///   offset  size  field   type      meaning
///        0     4  sihr    [u8; 4]   the four bytes "FORM"
///        4     4  tul     u32       payload length, not counting these 8 bytes
///
/// Chunk header — 8 bytes, repeated until the FORM payload is consumed
///
///   offset  size  field   type      meaning
///        0     4  ism     [u8; 4]   the chunk's four-character name
///        4     4  tul     u32       payload length, not counting these 8 bytes
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QitaHawiya {
    /// The four-character name.
    pub ism: [u8; 4],
    /// Where this chunk's eight-byte header starts in the file.
    pub tarwisa: u64,
    /// Where its payload starts in the file.
    pub izaha: u64,
    /// How long the payload is.
    pub tul: u64,
}

impl QitaHawiya {
    /// The name as text, for a message a person reads.
    ///
    /// Lossy on purpose: a chunk whose name is not ASCII has already been
    /// refused by the walk, so this only ever renders four printable bytes, and
    /// a refusal is not the place to introduce a second way to fail.
    #[must_use]
    pub fn ism_nass(&self) -> String {
        String::from_utf8_lossy(&self.ism).into_owned()
    }

    /// The file offset one byte past this chunk's payload.
    #[must_use]
    pub const fn nihaya(&self) -> u64 {
        self.izaha.saturating_add(self.tul)
    }

    /// Whether an absolute file offset falls inside this chunk's payload.
    #[must_use]
    pub const fn yahwi(&self, izaha: u64) -> bool {
        izaha >= self.izaha && izaha < self.nihaya()
    }
}

/// Walks the chunk table of a container already in memory.
///
/// The walk checks each step against the buffer's real length rather than
/// against the length the container claims for itself, because the two disagree
/// on real disks: a `data.win` truncated by a failed download and a `data.win`
/// a modding tool rebuilt without fixing the `FORM` size are both things people
/// have. A name that is not four printable ASCII bytes ends the walk, because
/// past that point the reader is no longer reading chunk headers.
///
/// # Errors
///
/// [`KhataNusus::MalafQaseer`] when the buffer does not hold the eight-byte
/// `FORM` header, [`KhataNusus::SihrGhayrMutabaq`] when those bytes are not
/// `FORM`, and [`KhataNusus::HawiyaTalifa`] when a chunk length runs past the
/// end of the `FORM` payload or the table exceeds [`AQSA_QITA`] entries.
pub fn jadwal_qita(bayt: &[u8], masar: &Path) -> Result<Vec<QitaHawiya>, KhataNusus> {
    let tul_kulli = tul_u64(bayt.len());
    if tul_kulli < HAJM_TARWISAT_QITA {
        return Err(KhataNusus::MalafQaseer {
            haql: "the FORM header",
            tul: tul_kulli,
            matlub: HAJM_TARWISAT_QITA,
        });
    }
    let sihr = iqra_ism(bayt, 0).ok_or(KhataNusus::MalafQaseer {
        haql: "the FORM magic",
        tul: tul_kulli,
        matlub: HAJM_TARWISAT_QITA,
    })?;
    if sihr != SIHR_FORM {
        return Err(KhataNusus::SihrGhayrMutabaq { masar: masar.to_path_buf(), sigha: SIGHA });
    }

    let mualan = u64::from(iqra_u32(bayt, 4).ok_or(KhataNusus::MalafQaseer {
        haql: "the FORM payload length",
        tul: tul_kulli,
        matlub: HAJM_TARWISAT_QITA,
    })?);
    let nihaya = HAJM_TARWISAT_QITA.saturating_add(mualan);
    if nihaya > tul_kulli {
        return Err(KhataNusus::MalafQaseer {
            haql: "the FORM payload the header declares",
            tul: tul_kulli,
            matlub: nihaya,
        });
    }

    let mut jadwal: Vec<QitaHawiya> = Vec::new();
    let mut mawqi = HAJM_TARWISAT_QITA;
    while mawqi < nihaya {
        if jadwal.len() >= AQSA_QITA {
            return Err(KhataNusus::HajmMufrit {
                haql: "chunks in the FORM table",
                qeema: tul_u64(jadwal.len()).saturating_add(1),
                saqf: tul_u64(AQSA_QITA),
            });
        }
        let ras = hajm_usize(mawqi).ok_or(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "a chunk header offset",
            qeema: mawqi,
            hadd: tul_kulli,
        })?;
        let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql,
            qeema,
            hadd: nihaya,
        };
        let ism = iqra_ism(bayt, ras).ok_or_else(|| talif("a chunk name", mawqi))?;
        if !ism.iter().all(u8::is_ascii_graphic) {
            return Err(talif("a chunk name that is not four printable bytes", mawqi));
        }
        let tul = u64::from(
            iqra_u32(bayt, ras.saturating_add(4)).ok_or_else(|| talif("a chunk length", mawqi))?,
        );
        let izaha = mawqi.checked_add(HAJM_TARWISAT_QITA).ok_or_else(|| talif("a chunk", mawqi))?;
        let baad = izaha.checked_add(tul).ok_or_else(|| talif("a chunk length", tul))?;
        if baad > nihaya {
            return Err(talif("a chunk running past the FORM payload", baad));
        }
        jadwal.push(QitaHawiya { ism, tarwisa: mawqi, izaha, tul });
        mawqi = baad;
    }
    Ok(jadwal)
}

/// The first chunk with a given name, if the table holds one.
#[must_use]
pub fn qita_bi_ism(jadwal: &[QitaHawiya], ism: [u8; 4]) -> Option<QitaHawiya> {
    jadwal.iter().copied().find(|qita| qita.ism == ism)
}

// ---------------------------------------------------------------------------
// GEN8 — read first, because every later chunk's layout depends on it
// ---------------------------------------------------------------------------

/// What `GEN8` says about the container.
///
/// ```text
/// GEN8 payload — little-endian
///
///   offset  size  field           meaning
///        0     1  munaqqih        debugger disabled
///        1     1  bytecode        the bytecode version: the layout selector
///        2     2  hashw           padding
///        4     4  ptr             STRG pointer: the project filename
///        8     4  ptr             STRG pointer: the configuration name
///       12     4  akhir_kain      last object id
///       16     4  akhir_balata    last tile id
///       20     4  muarrif         game id
///       24    16  guid            legacy DirectPlay GUID, zeroed since 2012
///       40     4  ptr             STRG pointer: the game name
///       44     4  isdar_kabir     runtime major
///       48     4  isdar_sagheer   runtime minor
///       52     4  isdar_tasheeh   runtime release
///       56     4  isdar_bina      runtime build
///       60     4  ard             default window width
///       64     4  irtifa          default window height
///       68     4  alam            the info flags
///       72     4  crc             licence CRC32
///       76    16  md5             licence MD5
///       92     8  waqt            build timestamp, Unix seconds
///      100     4  ptr             STRG pointer: the display name
///      104     8  ahdaf           active targets
///      112     8  tasneef         function classifications
///      120     4  steam           Steam application id
///      124     4  manfath         debugger port, bytecode 14 and above
/// ```
///
/// The bytecode version at offset 1 is read before anything else in the
/// container is interpreted, and a version outside this build's band ends the
/// parse there. Phase 5's detector reads the same structure to identify the
/// engine; this reads it to decide how to write the container back, which is why
/// it keeps the flags and the pointer offsets that the detector discards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BayanGen8 {
    /// The bytecode version, which is what decides every other chunk's layout.
    pub bytecode: u8,
    /// The generation that version belongs to.
    pub jeel: JeelHawiya,
    /// Whether the container shipped with the debugger switched off.
    pub munaqqih_muattal: bool,
    /// The project's game id.
    pub muarrif: u32,
    /// Runtime major, minor, release and build, as `GEN8` records them.
    pub isdar: (u32, u32, u32, u32),
    /// The default window size.
    pub nafidha: (u32, u32),
    /// The info flags at offset 68, kept verbatim.
    ///
    /// Not interpreted here. They control fullscreen, interpolation, sync and a
    /// dozen other runtime behaviours, and a patcher that rewrote one would be
    /// changing how the game plays in order to change how it reads.
    pub alam: u32,
    /// Absolute file offset of the `u32` holding the game name's pointer.
    pub mawqi_ism: u64,
    /// Absolute file offset of the `u32` holding the display name's pointer,
    /// when the chunk reaches that far.
    pub mawqi_ard: Option<u64>,
}

/// Reads `GEN8`, version first.
///
/// # Errors
///
/// [`KhataNusus::MalafQaseer`] when the chunk stops short of the fields the
/// layout above requires, and [`KhataNusus::IsdarGhayrMadum`] when the bytecode
/// version is outside [`ISDAR_ADNA`]..=[`ISDAR_AQSA`].
pub fn iqra_gen8(bayt: &[u8], qita: &QitaHawiya) -> Result<BayanGen8, KhataNusus> {
    let bidaya = hajm_usize(qita.izaha).ok_or_else(|| KhataNusus::HawiyaTalifa {
        sigha: SIGHA,
        haql: "the GEN8 payload offset",
        qeema: qita.izaha,
        hadd: tul_u64(bayt.len()),
    })?;
    let hadd = hajm_usize(qita.nihaya()).unwrap_or(bayt.len()).min(bayt.len());
    let juz = bayt.get(bidaya..hadd).ok_or_else(|| KhataNusus::MalafQaseer {
        haql: "the GEN8 payload",
        tul: tul_u64(bayt.len()),
        matlub: qita.nihaya(),
    })?;
    let qaseer = |haql: &'static str, matlub: u64| KhataNusus::MalafQaseer {
        haql,
        tul: tul_u64(juz.len()),
        matlub,
    };

    // The version, before any other field is read. Every offset below means what
    // it means only because this number said so.
    let bytecode = *juz.get(1).ok_or_else(|| qaseer("the GEN8 bytecode version", 2))?;
    let jeel = JeelHawiya::min_bytecode(bytecode)?;

    let munaqqih_muattal = *juz.first().ok_or_else(|| qaseer("the GEN8 debugger flag", 1))? != 0;
    let muarrif = iqra_u32(juz, 20).ok_or_else(|| qaseer("the GEN8 game id", 24))?;
    let isdar = (
        iqra_u32(juz, 44).ok_or_else(|| qaseer("the GEN8 runtime major", 48))?,
        iqra_u32(juz, 48).ok_or_else(|| qaseer("the GEN8 runtime minor", 52))?,
        iqra_u32(juz, 52).ok_or_else(|| qaseer("the GEN8 runtime release", 56))?,
        iqra_u32(juz, 56).ok_or_else(|| qaseer("the GEN8 runtime build", 60))?,
    );
    let nafidha = (
        iqra_u32(juz, 60).ok_or_else(|| qaseer("the GEN8 window width", 64))?,
        iqra_u32(juz, 64).ok_or_else(|| qaseer("the GEN8 window height", 68))?,
    );
    let alam = iqra_u32(juz, 68).ok_or_else(|| qaseer("the GEN8 info flags", 72))?;
    if iqra_u32(juz, 40).is_none() {
        return Err(qaseer("the GEN8 game name pointer", 44));
    }
    let mawqi_ism = qita.izaha.saturating_add(40);
    let mawqi_ard = iqra_u32(juz, 100).map(|_| qita.izaha.saturating_add(100));

    Ok(BayanGen8 {
        bytecode,
        jeel,
        munaqqih_muattal,
        muarrif,
        isdar,
        nafidha,
        alam,
        mawqi_ism,
        mawqi_ard,
    })
}

// ---------------------------------------------------------------------------
// STRG — the string pool, and the reason this format is hard
// ---------------------------------------------------------------------------

/// One string in the pool.
///
/// ```text
/// STRG payload
///
///   offset       size      field       meaning
///        0          4      adad        how many strings follow
///        4  4 * adad      mu_ashir[]   absolute file offsets, one per string
///      ...                 the bodies, each laid out as:
///
///   offset       size      field       meaning
///        0          4      tul         the body's length in bytes, no NUL
///        4        tul      bayt        the UTF-8 bytes
///    4+tul          1      sifr        a terminating NUL
/// ```
///
/// **A pointer names the bytes, not the record.** The length sits four bytes
/// *behind* the pointer, and the NUL sits one byte past the bytes it counts.
/// That is what lets the runtime hand a pointer straight to C code expecting a
/// NUL-terminated string while still knowing the length in O(1) — and it is why
/// [`MadkhalHawd::mawqi_sijil`] and [`MadkhalHawd::izaha`] are four apart and
/// both are kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalHawd {
    /// Absolute file offset of the `u32` slot in the pool's pointer array that
    /// names this string.
    pub mawqi_mu_ashir: u64,
    /// Absolute file offset of the record: the `u32` length.
    pub mawqi_sijil: u64,
    /// Absolute file offset of the body: what every pointer in the container
    /// holds for this string.
    pub izaha: u64,
    /// The decoded text.
    pub nass: String,
}

impl MadkhalHawd {
    /// How many bytes the whole record occupies: length, body, NUL.
    #[must_use]
    pub fn hajm_sijil(&self) -> u64 {
        tul_u64(self.nass.len()).saturating_add(5)
    }
}

/// The parsed string pool.
///
/// Holds the entries in pool order — which is the order the pointer array names
/// them and therefore the order every index in the container means — plus an
/// index from an absolute file offset back to the entry it addresses. That
/// second map is what turns an arbitrary `u32` found somewhere in the container
/// into "this is a reference to string 4,192", which is the first half of the
/// census the rewriting strategy depends on.
#[derive(Debug, Clone)]
pub struct HawdNusus {
    qita: QitaHawiya,
    madakhil: Vec<MadkhalHawd>,
    bi_izaha: BTreeMap<u64, u32>,
}

impl HawdNusus {
    /// Where the pool's chunk sits.
    #[must_use]
    pub const fn qita(&self) -> QitaHawiya {
        self.qita
    }

    /// How many strings the pool holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// Every entry, in pool order.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalHawd] {
        &self.madakhil
    }

    /// One entry by pool index.
    #[must_use]
    pub fn madkhal(&self, fahras: u32) -> Option<&MadkhalHawd> {
        self.madakhil.get(usize::try_from(fahras).ok()?)
    }

    /// The pool index a file offset addresses, if it addresses one at all.
    ///
    /// The exact-match requirement is deliberate. A `u32` that lands one byte
    /// inside a string body is not a reference to that string — it is a number
    /// that happens to fall in the pool's range, and treating it as a reference
    /// would rewrite an integer somewhere in the bytecode into a pointer.
    #[must_use]
    pub fn fahras_bi_izaha(&self, izaha: u64) -> Option<u32> {
        self.bi_izaha.get(&izaha).copied()
    }

    /// Reads the pool.
    ///
    /// Every length is checked against the container before a byte is reserved,
    /// and every body is required to be valid UTF-8. It is refused rather than
    /// replaced: a lossy decode would put replacement characters into a game's
    /// dialogue and then write them back as if they were the original.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the declared string count exceeds
    /// [`AQSA_NUSUS_HAWD`] or a declared length exceeds [`AQSA_TUL_NASS`],
    /// [`KhataNusus::MalafQaseer`] when the chunk stops short of the pointer
    /// array, [`KhataNusus::HawiyaTalifa`] when a pointer leaves the container or
    /// a body runs past its end, and [`KhataNusus::NassGhayrSalih`] naming the
    /// first invalid byte when a body is not UTF-8.
    pub fn iqra(bayt: &[u8], qita: &QitaHawiya) -> Result<Self, KhataNusus> {
        let tul_kulli = tul_u64(bayt.len());
        let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql,
            qeema,
            hadd: tul_kulli,
        };
        let ras = hajm_usize(qita.izaha)
            .ok_or_else(|| talif("the STRG payload offset", qita.izaha))?;
        let adad = iqra_u32(bayt, ras).ok_or_else(|| KhataNusus::MalafQaseer {
            haql: "the STRG string count",
            tul: tul_u64(bayt.len()),
            matlub: qita.izaha.saturating_add(4),
        })?;
        if adad > AQSA_NUSUS_HAWD {
            return Err(KhataNusus::HajmMufrit {
                haql: "strings in the STRG pool",
                qeema: u64::from(adad),
                saqf: u64::from(AQSA_NUSUS_HAWD),
            });
        }
        let hajm_masfufa = u64::from(adad).saturating_mul(4);
        let nihayat_masfufa = qita.izaha.saturating_add(4).saturating_add(hajm_masfufa);
        if nihayat_masfufa > qita.nihaya() {
            return Err(KhataNusus::MalafQaseer {
                haql: "the STRG pointer array the chunk declares",
                tul: qita.tul,
                matlub: nihayat_masfufa.saturating_sub(qita.izaha),
            });
        }

        let mut madakhil: Vec<MadkhalHawd> = Vec::with_capacity(
            usize::try_from(adad).unwrap_or_default().min(bayt.len()),
        );
        let mut bi_izaha: BTreeMap<u64, u32> = BTreeMap::new();
        for fahras in 0..adad {
            let mawqi_mu_ashir = qita
                .izaha
                .checked_add(4)
                .and_then(|asas| asas.checked_add(u64::from(fahras).saturating_mul(4)))
                .ok_or_else(|| talif("a STRG pointer slot", u64::from(fahras)))?;
            let khana = hajm_usize(mawqi_mu_ashir)
                .ok_or_else(|| talif("a STRG pointer slot", mawqi_mu_ashir))?;
            let izaha = u64::from(
                iqra_u32(bayt, khana).ok_or_else(|| talif("a STRG pointer", mawqi_mu_ashir))?,
            );
            let madkhal = Self::iqra_sijil(bayt, izaha, fahras)?;
            let _ = bi_izaha.insert(izaha, fahras);
            madakhil.push(MadkhalHawd { mawqi_mu_ashir, ..madkhal });
        }
        Ok(Self { qita: *qita, madakhil, bi_izaha })
    }

    /// Reads one record: the length behind the pointer, the body, the NUL.
    fn iqra_sijil(bayt: &[u8], izaha: u64, fahras: u32) -> Result<MadkhalHawd, KhataNusus> {
        let tul_kulli = tul_u64(bayt.len());
        let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql,
            qeema,
            hadd: tul_kulli,
        };
        let mawqi_sijil =
            izaha.checked_sub(4).ok_or_else(|| talif("a STRG pointer below the file", izaha))?;
        let ras = hajm_usize(mawqi_sijil).ok_or_else(|| talif("a STRG record", mawqi_sijil))?;
        let tul = iqra_u32(bayt, ras).ok_or_else(|| talif("a STRG record length", mawqi_sijil))?;
        if tul > AQSA_TUL_NASS {
            return Err(KhataNusus::HajmMufrit {
                haql: "the length of one STRG string",
                qeema: u64::from(tul),
                saqf: u64::from(AQSA_TUL_NASS),
            });
        }
        let bidaya = hajm_usize(izaha).ok_or_else(|| talif("a STRG body", izaha))?;
        let nihaya = bidaya
            .checked_add(usize::try_from(tul).unwrap_or(usize::MAX))
            .ok_or_else(|| talif("a STRG body length", u64::from(tul)))?;
        let jism = bayt
            .get(bidaya..nihaya)
            .ok_or_else(|| talif("a STRG body running past the file", tul_u64(nihaya)))?;
        let nass = match std::str::from_utf8(jism) {
            Ok(nass) => nass.to_owned(),
            Err(khata) => {
                return Err(KhataNusus::NassGhayrSalih {
                    malaf: format!("STRG string {fahras}"),
                    tarmiz: "UTF-8",
                    mawqi: izaha.saturating_add(tul_u64(khata.valid_up_to())),
                });
            }
        };
        Ok(MadkhalHawd { mawqi_mu_ashir: 0, mawqi_sijil, izaha, nass })
    }
}

// ---------------------------------------------------------------------------
// The reference census
// ---------------------------------------------------------------------------

/// What one absolute offset in the container points at.
///
/// The census tags every `u32` it believes is a pointer, and the tag decides
/// what happens to it when something moves. A field this module did not
/// classify is [`NawMarja::Majhul`] and is **never written**: its correctness is
/// guaranteed only by the zero-delta strategies moving nothing, which is exactly
/// why they are preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NawMarja {
    /// A string body, by pool index.
    Nass(u32),
    /// A `FONT` entry, by font index.
    MadkhalKhatt(u32),
    /// A glyph record inside a font's table.
    ShaklKhatt(u32, u32),
    /// A `TPAG` item, by item index.
    Band(u32),
    /// A `TXTR` blob, by page index.
    Safha(u32),
    /// An `AUDO` blob, by sound index.
    Sawt(u32),
    /// A field that looks like a pointer and has not been classified.
    Majhul,
}

impl NawMarja {
    /// Whether the census understands this reference well enough to rewrite it.
    #[must_use]
    pub const fn maaruf(self) -> bool {
        !matches!(self, Self::Majhul)
    }

    /// A short name for a refusal a person reads.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Nass(_) => "a string body",
            Self::MadkhalKhatt(_) => "a font entry",
            Self::ShaklKhatt(_, _) => "a glyph record",
            Self::Band(_) => "a texture page item",
            Self::Safha(_) => "a texture blob",
            Self::Sawt(_) => "an audio blob",
            Self::Majhul => "an unclassified pointer",
        }
    }
}

/// One reference site: a `u32` field, where it is, and what it points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MarjaIzaha {
    /// Absolute file offset of the four-byte field itself.
    pub mawqi: u64,
    /// The value it currently holds: an absolute file offset.
    pub hadaf: u64,
    /// What that value points at.
    pub naw: NawMarja,
}

/// Every reference this build found, and which chunk each one lives in.
///
/// Built once, at parse time, and consulted by every rewriting strategy. Two
/// facts come out of it and both are load-bearing:
///
/// * **for a given target, every field that names it** — so a structure can be
///   relocated by rewriting a known, finite set of `u32`s;
/// * **which chunks were fully enumerated** — so the planner can tell the
///   difference between "no reference into this region" and "this region was
///   never examined", which are the same observation and opposite conclusions.
#[derive(Debug, Clone, Default)]
pub struct JadwalMaraji {
    maraji: Vec<MarjaIzaha>,
    qita_mafhuma: BTreeSet<[u8; 4]>,
}

impl JadwalMaraji {
    /// An empty census.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { maraji: Vec::new(), qita_mafhuma: BTreeSet::new() }
    }

    /// Records one reference.
    pub fn sajjil(&mut self, marja: MarjaIzaha) {
        self.maraji.push(marja);
    }

    /// Records that a chunk was enumerated exhaustively.
    ///
    /// Only a parser that reached the end of a chunk and accounted for every
    /// byte of it may call this. A chunk whose parse gave up halfway is not
    /// enumerated, and saying it was would turn the relocation precondition into
    /// a formality.
    pub fn sajjil_qita(&mut self, ism: [u8; 4]) {
        let _ = self.qita_mafhuma.insert(ism);
    }

    /// Whether a chunk was enumerated exhaustively.
    #[must_use]
    pub fn mafhuma(&self, ism: [u8; 4]) -> bool {
        self.qita_mafhuma.contains(&ism)
    }

    /// Every reference, in the order they were found.
    #[must_use]
    pub fn maraji(&self) -> &[MarjaIzaha] {
        &self.maraji
    }

    /// How many references were recorded.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.maraji.len()
    }

    /// Every reference to one string, by pool index.
    pub fn maraji_nass(&self, fahras: u32) -> impl Iterator<Item = MarjaIzaha> + '_ {
        self.maraji.iter().copied().filter(move |marja| marja.naw == NawMarja::Nass(fahras))
    }

    /// Every reference whose target is at or after a file offset.
    ///
    /// The relocation planner's query: these are the fields that would be wrong
    /// if everything from `hadd` onwards moved.
    pub fn maraji_baad(&self, hadd: u64) -> impl Iterator<Item = MarjaIzaha> + '_ {
        self.maraji.iter().copied().filter(move |marja| marja.hadaf >= hadd)
    }
}

/// Reads a run of contiguous fixed-width entries whose first field is a string
/// pointer, and verifies the stride against the chunk's own extent.
///
/// `VARI` and `FUNC` have no pointer array — their entries sit end to end and
/// the width depends on the generation. Rather than trust a width, this reads
/// entries at the declared stride, requires every first field to be an *exact*
/// pool body offset, and requires the run to land exactly on `nihaya`. Both
/// checks together are what let the caller mark the chunk enumerated: a stride
/// that is wrong by four bytes fails the pool lookup within two entries, and one
/// that is wrong by a multiple of the entry size fails the extent check.
///
/// Returns [`None`] when either check fails, which the caller reads as "this
/// chunk was not enumerated" rather than as "this chunk holds no references".
fn maraji_mutatabia(
    bayt: &[u8],
    bidaya: u64,
    nihaya: u64,
    khutwa: u64,
    hawd: &HawdNusus,
) -> Option<Vec<MarjaIzaha>> {
    if khutwa == 0 || nihaya < bidaya {
        return None;
    }
    let madi = nihaya.checked_sub(bidaya)?;
    if madi.checked_rem(khutwa)? != 0 {
        return None;
    }
    let mut maraji: Vec<MarjaIzaha> = Vec::new();
    let mut mawqi = bidaya;
    while mawqi < nihaya {
        let ras = hajm_usize(mawqi)?;
        let hadaf = u64::from(iqra_u32(bayt, ras)?);
        let fahras = hawd.fahras_bi_izaha(hadaf)?;
        maraji.push(MarjaIzaha { mawqi, hadaf, naw: NawMarja::Nass(fahras) });
        mawqi = mawqi.checked_add(khutwa)?;
    }
    Some(maraji)
}

/// The width of one `VARI` entry in a generation, and how many bytes of header
/// precede the run.
///
/// ```text
/// VARI payload, bytecode 15 and above
///
///   offset  size  meaning
///        0     4  variable count, first form
///        4     4  variable count, second form
///        8     4  the highest local-variable count of any script
///       12   ...  the entries
///
/// One entry, bytecode 15 and above — 20 bytes
///
///   offset  size  meaning
///        0     4  STRG pointer: the variable's name
///        4     4  instance type
///        8     4  variable id
///       12     4  how many times the bytecode references it
///       16     4  the address of the first of those references
///
/// One entry, bytecode 13 and 14 — 12 bytes
///
///   offset  size  meaning
///        0     4  STRG pointer: the variable's name
///        4     4  how many times the bytecode references it
///        8     4  the address of the first of those references
/// ```
const fn shakl_vari(jeel: JeelHawiya) -> (u64, u64) {
    match jeel {
        JeelHawiya::Studio1 => (0, 12),
        JeelHawiya::Studio2 | JeelHawiya::Studio2Muhaddath => (12, 20),
    }
}

// ---------------------------------------------------------------------------
// TPAG — the rectangle a sprite or a font occupies on a page
// ---------------------------------------------------------------------------

/// How many bytes one `TPAG` item occupies, in every generation.
pub const HAJM_BAND_TPAG: u64 = 22;

/// One texture page item.
///
/// ```text
/// TPAG item — 22 bytes, little-endian, unchanged since Studio 1.4
///
///   offset  size  field         meaning
///        0     2  masdar_s      x of the region on the page
///        2     2  masdar_a      y of the region on the page
///        4     2  masdar_ard    width of the region
///        6     2  masdar_irtifa height of the region
///        8     2  hadaf_s       x the region is drawn at inside the sprite
///       10     2  hadaf_a       y it is drawn at
///       12     2  hadaf_ard     width it is drawn at
///       14     2  hadaf_irtifa  height it is drawn at
///       16     2  itar_ard      the untrimmed width the sprite declares
///       18     2  itar_irtifa   the untrimmed height
///       20     2  safha         index of the texture page, not a pointer
/// ```
///
/// The last field is an **index**, not an offset, which is the one piece of luck
/// in this format: appending a texture page does not invalidate a single `TPAG`
/// item, because no item names a page by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BandTpag {
    /// Absolute file offset of the item.
    pub mawqi: u64,
    /// The region on the page.
    pub masdar: MustatilSafha,
    /// Where the region is drawn inside the sprite's own frame.
    pub hadaf: MustatilSafha,
    /// The untrimmed frame the sprite declares.
    pub itar: (u16, u16),
    /// Which texture page, by index.
    pub safha: u16,
}

/// A rectangle in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord, Hash)]
pub struct MustatilSafha {
    /// Left edge.
    pub s: u16,
    /// Top edge.
    pub a: u16,
    /// Width.
    pub ard: u16,
    /// Height.
    pub irtifa: u16,
}

impl BandTpag {
    /// Reads one item at an absolute offset.
    fn iqra(bayt: &[u8], mawqi: u64) -> Option<Self> {
        let ras = hajm_usize(mawqi)?;
        let haql = |izaha: usize| iqra_u16(bayt, ras.checked_add(izaha)?);
        Some(Self {
            mawqi,
            masdar: MustatilSafha {
                s: haql(0)?,
                a: haql(2)?,
                ard: haql(4)?,
                irtifa: haql(6)?,
            },
            hadaf: MustatilSafha {
                s: haql(8)?,
                a: haql(10)?,
                ard: haql(12)?,
                irtifa: haql(14)?,
            },
            itar: (haql(16)?, haql(18)?),
            safha: haql(20)?,
        })
    }

    /// Overwrites this item in place so that it names a whole new page.
    ///
    /// The one edit the font path needs and the only one that is free: a `TPAG`
    /// item is a fixed twenty-two bytes and names its page by index, so
    /// repointing a font at an appended atlas page changes twenty-two bytes and
    /// moves nothing. Adding an item would be a different matter — the chunk's
    /// count is positional and `TPAG` sits in front of `CODE`, `VARI` and `FUNC`
    /// — which is why this module repoints and never appends here.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HawiyaTalifa`] when the item's twenty-two bytes are not all
    /// inside the buffer.
    pub fn ustub(
        &self,
        bayt: &mut [u8],
        safha: u16,
        mustatil: MustatilSafha,
    ) -> Result<(), KhataNusus> {
        let tul = tul_u64(bayt.len());
        let ras = hajm_usize(self.mawqi).ok_or(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "a TPAG item offset",
            qeema: self.mawqi,
            hadd: tul,
        })?;
        let talif = KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "a TPAG item running past the container",
            qeema: self.mawqi.saturating_add(HAJM_BAND_TPAG),
            hadd: tul,
        };
        let khaam = bayt_band(safha, mustatil);
        let mut salim = true;
        for (fahras, juz) in khaam.chunks_exact(2).enumerate() {
            let Some(mawdi) = ras.checked_add(fahras.saturating_mul(2)) else {
                salim = false;
                break;
            };
            let Some(qeema) = iqra_u16(juz, 0) else {
                salim = false;
                break;
            };
            if !uktub_u16(bayt, mawdi, qeema) {
                salim = false;
                break;
            }
        }
        if salim { Ok(()) } else { Err(talif) }
    }
}

/// The twenty-two bytes of one `TPAG` item covering a whole page.
///
/// One spelling, used by the in-place writer and by the patcher's planner, so
/// the two cannot drift into describing the same rectangle differently.
fn bayt_band(safha: u16, mustatil: MustatilSafha) -> Vec<u8> {
    let mut khaam: Vec<u8> = Vec::with_capacity(22);
    for qeema in [
        mustatil.s,
        mustatil.a,
        mustatil.ard,
        mustatil.irtifa,
        0,
        0,
        mustatil.ard,
        mustatil.irtifa,
        mustatil.ard,
        mustatil.irtifa,
        safha,
    ] {
        khaam.extend_from_slice(&qeema.to_le_bytes());
    }
    khaam
}

/// Reads the `TPAG` chunk: a count, a pointer array, then the items.
///
/// # Errors
///
/// [`KhataNusus::HajmMufrit`] above [`AQSA_BUNUD_TPAG`], and
/// [`KhataNusus::HawiyaTalifa`] when the count, the array or an item leaves the
/// chunk.
pub fn iqra_tpag(bayt: &[u8], qita: &QitaHawiya) -> Result<Vec<BandTpag>, KhataNusus> {
    let tul = tul_u64(bayt.len());
    let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
        sigha: SIGHA,
        haql,
        qeema,
        hadd: tul,
    };
    let ras = hajm_usize(qita.izaha).ok_or_else(|| talif("the TPAG payload", qita.izaha))?;
    let adad = iqra_u32(bayt, ras).ok_or_else(|| talif("the TPAG item count", qita.izaha))?;
    if adad > AQSA_BUNUD_TPAG {
        return Err(KhataNusus::HajmMufrit {
            haql: "items in the TPAG chunk",
            qeema: u64::from(adad),
            saqf: u64::from(AQSA_BUNUD_TPAG),
        });
    }
    let mut bunud: Vec<BandTpag> = Vec::with_capacity(usize::try_from(adad).unwrap_or_default());
    for fahras in 0..adad {
        let khana = ras
            .checked_add(4)
            .and_then(|asas| asas.checked_add(usize::try_from(fahras).ok()?.checked_mul(4)?))
            .ok_or_else(|| talif("a TPAG pointer slot", u64::from(fahras)))?;
        let mawqi = u64::from(
            iqra_u32(bayt, khana).ok_or_else(|| talif("a TPAG pointer", tul_u64(khana)))?,
        );
        if !qita.yahwi(mawqi) || mawqi.saturating_add(HAJM_BAND_TPAG) > qita.nihaya() {
            return Err(talif("a TPAG item outside its own chunk", mawqi));
        }
        bunud.push(BandTpag::iqra(bayt, mawqi).ok_or_else(|| talif("a TPAG item", mawqi))?);
    }
    Ok(bunud)
}

// ---------------------------------------------------------------------------
// TXTR — the texture pages
// ---------------------------------------------------------------------------

/// The eight bytes every PNG begins with.
pub const SIHR_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The four bytes a QOI page begins with, which GameMaker 2022.5 and later
/// write instead of PNG for some pages.
///
/// GameMaker writes the QOI magic reversed — `fioq` rather than the `qoif` the
/// format defines — so both spellings are accepted. A reader that knew only one
/// of them would decide a perfectly good page entry had no blob pointer, and
/// refuse a container it could have patched.
pub const SIHR_QOI: [u8; 4] = *b"fioq";

/// The QOI magic as the format itself defines it.
pub const SIHR_QOI_QIYASI: [u8; 4] = *b"qoif";

/// The four bytes that begin a BZip2-compressed QOI page.
///
/// The other encoding the 2022.5 and later runtimes write. The bytes are a
/// marker rather than an image signature, and a page carrying one is still a
/// page: it is recognised so that its entry's blob pointer can be found, and its
/// contents are never decoded here.
pub const SIHR_QOI_MADGHUT: [u8; 4] = [0x2F, 0x0A, 0x0A, 0x0A];

/// One texture page.
///
/// ```text
/// TXTR payload
///
///   offset       size      field        meaning
///        0          4      adad         how many pages
///        4  4 * adad       mu_ashir[]   absolute offsets of the page entries
///      ...                 the entries, whose width changed four times
///      ...                 the blobs, each aligned to MUHADHAHA_SAFHA
/// ```
///
/// **The entry's width is not assumed.** Studio 1.4 wrote two `u32`s, Studio 2.0
/// wrote three, and the 2022 and 2023 releases added a declared blob length and
/// a texture-block descriptor. A reader that picked a width from a version
/// number would read somebody else's field as a pointer on the first release it
/// had not been told about.
///
/// So the blob pointer is found by **verification instead**: inside the entry's
/// own extent, exactly one four-byte-aligned `u32` may point at a
/// [`MUHADHAHA_SAFHA`]-aligned position inside the `TXTR` chunk whose first
/// bytes are a PNG or QOI signature. Zero candidates or more than one is a
/// refusal, not a guess. That test is independent of the layout and gets
/// stronger, not weaker, as the format gains fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafhaTxtr {
    /// Absolute file offset of the page entry.
    pub mawqi_madkhal: u64,
    /// Absolute file offset of the `u32` field holding the blob pointer.
    pub mawqi_mu_ashir: u64,
    /// Absolute file offset of the blob itself.
    pub izaha: u64,
    /// How many bytes the blob occupies, up to the next page or the chunk's end.
    pub tul: u64,
}

/// Reads the `TXTR` chunk.
///
/// # Errors
///
/// [`KhataNusus::HajmMufrit`] above [`AQSA_SAFAHAT`], and
/// [`KhataNusus::HawiyaTalifa`] when the count or the array leaves the chunk, or
/// when a page entry does not have exactly one field that could be its blob
/// pointer.
pub fn iqra_txtr(bayt: &[u8], qita: &QitaHawiya) -> Result<Vec<SafhaTxtr>, KhataNusus> {
    let tul = tul_u64(bayt.len());
    let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
        sigha: SIGHA,
        haql,
        qeema,
        hadd: tul,
    };
    let ras = hajm_usize(qita.izaha).ok_or_else(|| talif("the TXTR payload", qita.izaha))?;
    let adad = iqra_u32(bayt, ras).ok_or_else(|| talif("the TXTR page count", qita.izaha))?;
    if adad > AQSA_SAFAHAT {
        return Err(KhataNusus::HajmMufrit {
            haql: "pages in the TXTR chunk",
            qeema: u64::from(adad),
            saqf: u64::from(AQSA_SAFAHAT),
        });
    }

    let mut madakhil: Vec<u64> = Vec::with_capacity(usize::try_from(adad).unwrap_or_default());
    for fahras in 0..adad {
        let khana = ras
            .checked_add(4)
            .and_then(|asas| asas.checked_add(usize::try_from(fahras).ok()?.checked_mul(4)?))
            .ok_or_else(|| talif("a TXTR pointer slot", u64::from(fahras)))?;
        let mawqi = u64::from(
            iqra_u32(bayt, khana).ok_or_else(|| talif("a TXTR pointer", tul_u64(khana)))?,
        );
        if !qita.yahwi(mawqi) {
            return Err(talif("a TXTR page entry outside its own chunk", mawqi));
        }
        madakhil.push(mawqi);
    }

    // An entry's extent is the distance to the next entry, and for the last one
    // it is the distance to the first blob — which is not known yet, so the last
    // entry is bounded by the chunk instead. Both bounds are upper bounds, and
    // the signature test is what actually decides.
    let mut safahat: Vec<SafhaTxtr> = Vec::with_capacity(madakhil.len());
    for (fahras, mawqi) in madakhil.iter().enumerate() {
        let hadd =
            madakhil.get(fahras.saturating_add(1)).copied().unwrap_or_else(|| qita.nihaya());
        let (mawqi_mu_ashir, izaha) = mu_ashir_safha(bayt, qita, *mawqi, hadd.max(*mawqi))
            .ok_or_else(|| talif("a TXTR page entry with no identifiable blob pointer", *mawqi))?;
        safahat.push(SafhaTxtr { mawqi_madkhal: *mawqi, mawqi_mu_ashir, izaha, tul: 0 });
    }

    // A blob runs to the next blob, and the last runs to the chunk's end. The
    // blobs are not in entry order in every container, so the ends are computed
    // from a sorted copy rather than from the order the entries were read in.
    let mut bidayat: Vec<u64> = safahat.iter().map(|safha| safha.izaha).collect();
    bidayat.sort_unstable();
    for safha in &mut safahat {
        let baad = bidayat.iter().copied().find(|bidaya| *bidaya > safha.izaha);
        safha.tul = baad.unwrap_or_else(|| qita.nihaya()).saturating_sub(safha.izaha);
    }
    Ok(safahat)
}

/// Finds the one field of a page entry that can be its blob pointer.
///
/// Returns the field's offset and the blob's offset, or [`None`] when there is
/// not exactly one candidate. See [`SafhaTxtr`] for why this is a search rather
/// than a layout constant.
fn mu_ashir_safha(bayt: &[u8], qita: &QitaHawiya, bidaya: u64, hadd: u64) -> Option<(u64, u64)> {
    let mut wahid: Option<(u64, u64)> = None;
    let mut mawqi = bidaya;
    while mawqi.checked_add(4)? <= hadd {
        let ras = hajm_usize(mawqi)?;
        let hadaf = u64::from(iqra_u32(bayt, ras)?);
        let mahdhi = qita.yahwi(hadaf) && hadaf.checked_rem(MUHADHAHA_SAFHA)? == 0;
        if mahdhi && safha_masmuha(bayt, hadaf) {
            if wahid.is_some() {
                return None;
            }
            wahid = Some((mawqi, hadaf));
        }
        mawqi = mawqi.checked_add(4)?;
    }
    wahid
}

/// Whether the bytes at an offset begin an image this format stores as a page.
fn safha_masmuha(bayt: &[u8], izaha: u64) -> bool {
    let Some(ras) = hajm_usize(izaha) else {
        return false;
    };
    let Some(muqaddima) = bayt.get(ras..ras.saturating_add(8)) else {
        return false;
    };
    muqaddima == SIHR_PNG
        || muqaddima.starts_with(&SIHR_QOI)
        || muqaddima.starts_with(&SIHR_QOI_QIYASI)
        || muqaddima.starts_with(&SIHR_QOI_MADGHUT)
}

// ---------------------------------------------------------------------------
// FONT — the baked glyph table
// ---------------------------------------------------------------------------

/// One entry of a glyph's kerning list.
///
/// ```text
/// Kerning pair — 4 bytes
///
///   offset  size  field     meaning
///        0     2  akhar     the character code this pair applies against
///        2     2  tashih    pixels added to the first glyph's shift
/// ```
///
/// Both fields are signed sixteen-bit. The correction is **added** to the first
/// glyph's advance, which is the opposite sign convention from the one Godot's
/// `.fnt` importer uses; nothing here converts between them because nothing here
/// writes a `.fnt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZawjTaqaddumShakl {
    /// The character code the pair applies against.
    pub akhar: u16,
    /// Pixels added to the first glyph's advance.
    pub tashih: i16,
}

/// One glyph of a font's baked table.
///
/// ```text
/// Glyph record — 16 bytes plus 4 per kerning pair
///
///   offset  size  field       meaning
///        0     2  harf        the character code this glyph draws
///        2     2  s           x of the glyph's rectangle on the page
///        4     2  a           y of the rectangle
///        6     2  ard         width of the rectangle
///        8     2  irtifa      height of the rectangle
///       10     2  izaha       signed: pixels right of the pen the image starts
///       12     2  taqaddum    signed: pixels the pen moves after this glyph
///       14     2  adad_azwaj  how many kerning pairs follow
///       16   4*k  azwaj       the pairs
/// ```
///
/// **The index is a character code.** That single fact is the whole reason the
/// glyph transport exists: shaped output is indexed by glyph identifier and this
/// table is indexed by character, so a shaped run cannot address it without a
/// naming layer. See the module header, and [`taarib_lawha::naql`] for the
/// argument that the naming layer is not a presentation-form pipeline.
///
/// The rectangle is relative to the **page**, not to the font's `TPAG` item. A
/// generated table therefore has to know where its atlas page sits before it can
/// be written, which is why the font path appends the page first and rebuilds
/// the table second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaklKhatt {
    /// The character code this glyph draws.
    pub harf: u16,
    /// The glyph's rectangle on its texture page.
    pub mustatil: MustatilSafha,
    /// Pixels right of the pen at which the image starts.
    pub izaha: i16,
    /// Pixels the pen advances after drawing this glyph.
    pub taqaddum: i16,
    /// The kerning list.
    pub azwaj: Vec<ZawjTaqaddumShakl>,
}

impl ShaklKhatt {
    /// How many bytes this record occupies when written.
    #[must_use]
    pub fn hajm(&self) -> u64 {
        tul_u64(self.azwaj.len()).saturating_mul(4).saturating_add(16)
    }

    /// Serializes the record into the byte order the container uses.
    #[must_use]
    pub fn bayt(&self) -> Vec<u8> {
        let mut khaam = Vec::with_capacity(usize::try_from(self.hajm()).unwrap_or(16));
        khaam.extend_from_slice(&self.harf.to_le_bytes());
        khaam.extend_from_slice(&self.mustatil.s.to_le_bytes());
        khaam.extend_from_slice(&self.mustatil.a.to_le_bytes());
        khaam.extend_from_slice(&self.mustatil.ard.to_le_bytes());
        khaam.extend_from_slice(&self.mustatil.irtifa.to_le_bytes());
        khaam.extend_from_slice(&self.izaha.to_le_bytes());
        khaam.extend_from_slice(&self.taqaddum.to_le_bytes());
        khaam.extend_from_slice(&u16::try_from(self.azwaj.len()).unwrap_or(u16::MAX).to_le_bytes());
        for zawj in &self.azwaj {
            khaam.extend_from_slice(&zawj.akhar.to_le_bytes());
            khaam.extend_from_slice(&zawj.tashih.to_le_bytes());
        }
        khaam
    }

    /// Reads one record at an absolute offset.
    fn iqra(bayt: &[u8], mawqi: u64) -> Option<Self> {
        let ras = hajm_usize(mawqi)?;
        let haql = |izaha: usize| iqra_u16(bayt, ras.checked_add(izaha)?);
        let musharraf = |izaha: usize| iqra_i16(bayt, ras.checked_add(izaha)?);
        let adad = haql(14)?;
        if adad > AQSA_AZWAJ_SHAKL {
            return None;
        }
        let mut azwaj: Vec<ZawjTaqaddumShakl> = Vec::with_capacity(usize::from(adad));
        for fahras in 0..adad {
            let asas = ras.checked_add(16)?.checked_add(usize::from(fahras).checked_mul(4)?)?;
            azwaj.push(ZawjTaqaddumShakl {
                akhar: iqra_u16(bayt, asas)?,
                tashih: iqra_i16(bayt, asas.checked_add(2)?)?,
            });
        }
        Some(Self {
            harf: haql(0)?,
            mustatil: MustatilSafha {
                s: haql(2)?,
                a: haql(4)?,
                ard: haql(6)?,
                irtifa: haql(8)?,
            },
            izaha: musharraf(10)?,
            taqaddum: musharraf(12)?,
            azwaj,
        })
    }
}

/// One font resource, as the container carries it.
///
/// ```text
/// FONT payload
///
///   offset       size      field        meaning
///        0          4      adad         how many fonts
///        4  4 * adad       mu_ashir[]   absolute offsets of the font entries
///      ...                 the entries
///      ...                 a trailing ramp of padding GameMaker always writes
///
/// One font entry
///
///   offset  size  field         meaning
///        0     4  ptr           STRG pointer: the resource name
///        4     4  ptr           STRG pointer: the display font name
///        8     4  hajm          em size: an integer through 2.2, an f32 after
///       12     4  areed         bold
///       16     4  ma_il         italic
///       20     2  bidayat_nitaq the first character code the font was baked for
///       22     1  tarmiz        the charset byte
///       23     1  tanaim        the antialiasing level
///       24     4  nihayat_nitaq the last character code
///       28     4  ptr           TPAG pointer: the item this font draws from
///       32     4  miqyas_s      horizontal scale, f32
///       36     4  miqyas_a      vertical scale, f32
///      ...            optional trailing fields added in 2.3, 2022.2, 2023.2
///        n     4  adad_ashkal   how many glyphs
///      n+4  4 * g  mu_ashir[]   absolute offsets of the glyph records
///      ...                 the glyph records
/// ```
///
/// **The glyph count's position is found, not assumed.** Three separate fields
/// were inserted between the scale pair and the glyph count across the 2.3,
/// 2022.2 and 2023.2 releases, and the bytecode version does not distinguish
/// them — all three write bytecode 17. So the reader scans forward from the
/// scale pair for the first `u32` whose value is a plausible glyph count *and*
/// which is followed by exactly that many pointers, all inside the chunk, whose
/// first one lands exactly one byte past the pointer array. That conjunction is
/// what GameMaker's own serializer guarantees and what a wrong position cannot
/// satisfy by accident.
#[derive(Debug, Clone)]
pub struct KhattGameMaker {
    /// Absolute file offset of the entry.
    pub mawqi: u64,
    /// Absolute file offset of the `u32` in the `FONT` pointer array that names
    /// this entry.
    pub mawqi_mu_ashir: u64,
    /// Absolute offset of the entry's `u32` name pointer.
    pub mawqi_ism: u64,
    /// The resource name, resolved through the pool.
    pub ism: String,
    /// The display font name, resolved through the pool.
    pub ism_ard: String,
    /// The em size as the container stores it, uninterpreted.
    pub hajm: u32,
    /// Whether the font is bold.
    pub areed: bool,
    /// Whether it is italic.
    pub ma_il: bool,
    /// The charset byte.
    pub tarmiz: u8,
    /// The antialiasing level.
    pub tanaim: u8,
    /// The character range the table was baked for.
    pub nitaq: (u16, u32),
    /// Absolute offset of the `u32` holding the `TPAG` pointer.
    pub mawqi_band: u64,
    /// The `TPAG` item this font draws from.
    pub band: u64,
    /// Absolute offset of the `u32` glyph count.
    pub mawqi_adad_ashkal: u64,
    /// The glyph table, in the order the pointer array names it.
    pub ashkal: Vec<ShaklKhatt>,
    /// How many bytes the entry occupies, from `mawqi` to the last glyph record.
    pub hajm_madkhal: u64,
}

/// Where a font entry's fixed fields end and its optional ones begin.
///
/// Forty bytes: the two string pointers, the size, bold, italic, the range and
/// charset word, the range end, the `TPAG` pointer and the scale pair.
const HAJM_THABIT_KHATT: u64 = 40;

/// How far past the fixed fields the glyph count is looked for.
///
/// Six four-byte steps, which covers every optional field the format has gained
/// since Studio 1.4 with three to spare. Bounded rather than open-ended: an
/// unbounded scan would eventually find a plausible-looking count inside the
/// glyph records themselves and rebuild a table from the middle of one.
const AQSA_KHUTUWAT_ADAD_ASHKAL: u64 = 6;

/// Reads the `FONT` chunk.
///
/// # Errors
///
/// [`KhataNusus::HajmMufrit`] above [`AQSA_KHUTUT`] or [`AQSA_ASHKAL_KHATT`],
/// [`KhataNusus::HawiyaTalifa`] when the count, the array or an entry leaves the
/// chunk, and [`KhataNusus::JadwalAshkalMarfud`] when an entry's glyph count
/// could not be located — which is a rung declining rather than a corrupt file,
/// because it means this build does not know the layout of a newer release and
/// the caller should descend rather than write a table into the wrong offsets.
pub fn iqra_font(
    bayt: &[u8],
    qita: &QitaHawiya,
    hawd: &HawdNusus,
) -> Result<Vec<KhattGameMaker>, KhataNusus> {
    let tul = tul_u64(bayt.len());
    let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
        sigha: SIGHA,
        haql,
        qeema,
        hadd: tul,
    };
    let ras = hajm_usize(qita.izaha).ok_or_else(|| talif("the FONT payload", qita.izaha))?;
    let adad = iqra_u32(bayt, ras).ok_or_else(|| talif("the FONT count", qita.izaha))?;
    if adad > AQSA_KHUTUT {
        return Err(KhataNusus::HajmMufrit {
            haql: "fonts in the FONT chunk",
            qeema: u64::from(adad),
            saqf: u64::from(AQSA_KHUTUT),
        });
    }

    let mut khutut: Vec<KhattGameMaker> = Vec::with_capacity(
        usize::try_from(adad).unwrap_or_default(),
    );
    for fahras in 0..adad {
        let khana = ras
            .checked_add(4)
            .and_then(|asas| asas.checked_add(usize::try_from(fahras).ok()?.checked_mul(4)?))
            .ok_or_else(|| talif("a FONT pointer slot", u64::from(fahras)))?;
        let mawqi = u64::from(
            iqra_u32(bayt, khana).ok_or_else(|| talif("a FONT pointer", tul_u64(khana)))?,
        );
        if !qita.yahwi(mawqi) {
            return Err(talif("a FONT entry outside its own chunk", mawqi));
        }
        khutut.push(iqra_madkhal_khatt(bayt, qita, hawd, mawqi, tul_u64(khana), fahras)?);
    }
    Ok(khutut)
}

/// Reads one font entry, locating its glyph count by verification.
fn iqra_madkhal_khatt(
    bayt: &[u8],
    qita: &QitaHawiya,
    hawd: &HawdNusus,
    mawqi: u64,
    mawqi_mu_ashir: u64,
    fahras: u32,
) -> Result<KhattGameMaker, KhataNusus> {
    let tul = tul_u64(bayt.len());
    let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
        sigha: SIGHA,
        haql,
        qeema,
        hadd: tul,
    };
    let ras = hajm_usize(mawqi).ok_or_else(|| talif("a FONT entry", mawqi))?;
    let haql32 = |izaha: usize| -> Result<u32, KhataNusus> {
        ras.checked_add(izaha)
            .and_then(|mawdi| iqra_u32(bayt, mawdi))
            .ok_or_else(|| talif("a FONT entry field", mawqi))
    };
    let nass = |mu_ashir: u32| -> String {
        hawd.fahras_bi_izaha(u64::from(mu_ashir))
            .and_then(|fahras| hawd.madkhal(fahras))
            .map_or_else(String::new, |madkhal| madkhal.nass.clone())
    };

    let ism = nass(haql32(0)?);
    let ism_ard = nass(haql32(4)?);
    let hajm = haql32(8)?;
    let areed = haql32(12)? != 0;
    let ma_il = haql32(16)? != 0;
    let bidayat_nitaq = ras
        .checked_add(20)
        .and_then(|mawdi| iqra_u16(bayt, mawdi))
        .ok_or_else(|| talif("a FONT range start", mawqi))?;
    let tarmiz = *bayt
        .get(ras.saturating_add(22))
        .ok_or_else(|| talif("a FONT charset byte", mawqi))?;
    let tanaim = *bayt
        .get(ras.saturating_add(23))
        .ok_or_else(|| talif("a FONT antialiasing byte", mawqi))?;
    let nihayat_nitaq = haql32(24)?;
    let band = u64::from(haql32(28)?);

    let (mawqi_adad_ashkal, ashkal, hajm_madkhal) =
        jadwal_ashkal(bayt, qita, mawqi, fahras, &ism)?;

    Ok(KhattGameMaker {
        mawqi,
        mawqi_mu_ashir,
        mawqi_ism: mawqi,
        ism,
        ism_ard,
        hajm,
        areed,
        ma_il,
        tarmiz,
        tanaim,
        nitaq: (bidayat_nitaq, nihayat_nitaq),
        mawqi_band: mawqi.saturating_add(28),
        band,
        mawqi_adad_ashkal,
        ashkal,
        hajm_madkhal,
    })
}

/// Locates and reads a font entry's glyph table.
///
/// The search is the one described on [`KhattGameMaker`]: step forward four
/// bytes at a time from the fixed fields, and accept the first position whose
/// `u32` is a plausible count *and* whose following pointers are all inside the
/// chunk, strictly increasing, with the first landing exactly past the array.
fn jadwal_ashkal(
    bayt: &[u8],
    qita: &QitaHawiya,
    mawqi: u64,
    fahras: u32,
    ism: &str,
) -> Result<(u64, Vec<ShaklKhatt>, u64), KhataNusus> {
    let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
    for khutwa in 0..=AQSA_KHUTUWAT_ADAD_ASHKAL {
        let Some(mawdi) = mawqi
            .checked_add(HAJM_THABIT_KHATT)
            .and_then(|asas| asas.checked_add(khutwa.checked_mul(4)?))
        else {
            break;
        };
        let Some(ras) = hajm_usize(mawdi) else { break };
        let Some(adad) = iqra_u32(bayt, ras) else { break };
        if adad == 0 || adad > AQSA_ASHKAL_KHATT {
            continue;
        }
        let Some(bidayat_masfufa) = mawdi.checked_add(4) else { break };
        let Some(nihayat_masfufa) =
            bidayat_masfufa.checked_add(u64::from(adad).saturating_mul(4))
        else {
            continue;
        };
        if nihayat_masfufa > qita.nihaya() {
            continue;
        }
        let Some(mu_ashirat) = masfufat_ashkal(bayt, qita, bidayat_masfufa, adad) else {
            continue;
        };
        if mu_ashirat.first().copied() != Some(nihayat_masfufa) {
            continue;
        }
        let mut ashkal: Vec<ShaklKhatt> = Vec::with_capacity(mu_ashirat.len());
        let mut nihaya = nihayat_masfufa;
        for mu_ashir in &mu_ashirat {
            let shakl = ShaklKhatt::iqra(bayt, *mu_ashir).ok_or_else(|| {
                marfud(format!(
                    "font {fahras} ({ism}) has a glyph record at {mu_ashir} that runs past \
                     the FONT chunk"
                ))
            })?;
            nihaya = mu_ashir.saturating_add(shakl.hajm()).max(nihaya);
            ashkal.push(shakl);
        }
        if nihaya > qita.nihaya() {
            continue;
        }
        return Ok((mawdi, ashkal, nihaya.saturating_sub(mawqi)));
    }
    Err(marfud(format!(
        "font {fahras} ({ism}) has no glyph count in the {AQSA_KHUTUWAT_ADAD_ASHKAL} words \
         after its fixed fields, which means this build does not know the entry layout of \
         the runtime that wrote this container"
    )))
}

/// The glyph pointer array, when every entry is inside the chunk and the array
/// is strictly increasing.
fn masfufat_ashkal(bayt: &[u8], qita: &QitaHawiya, bidaya: u64, adad: u32) -> Option<Vec<u64>> {
    let mut mu_ashirat: Vec<u64> = Vec::with_capacity(usize::try_from(adad).ok()?);
    let mut sabiq = 0_u64;
    for fahras in 0..adad {
        let mawdi = bidaya.checked_add(u64::from(fahras).checked_mul(4)?)?;
        let hadaf = u64::from(iqra_u32(bayt, hajm_usize(mawdi)?)?);
        if !qita.yahwi(hadaf) || hadaf <= sabiq {
            return None;
        }
        sabiq = hadaf;
        mu_ashirat.push(hadaf);
    }
    Some(mu_ashirat)
}

// ---------------------------------------------------------------------------
// The addressable model
// ---------------------------------------------------------------------------

/// A parsed container: the bytes, the chunk table, and everything addressable.
///
/// The bytes are kept whole and unmodified. Every chunk this build does not
/// parse is therefore preserved **verbatim** for free — there is no
/// re-serialization step that could round-trip it wrongly, because there is no
/// re-serialization step at all. A rewrite is a set of byte edits applied to a
/// copy of this buffer plus, at most, an appended region; anything not named by
/// an edit is the original byte.
///
/// That is a deliberate inversion of how a container writer is usually built,
/// and it is the property [`MuhawwilGameMaker::tahaqquq_dawra`] verifies rather
/// than assumes.
#[derive(Debug, Clone)]
pub struct HawiyatGameMaker {
    bayt: Vec<u8>,
    jadwal: Vec<QitaHawiya>,
    gen8: BayanGen8,
    hawd: HawdNusus,
    khutut: Vec<KhattGameMaker>,
    bunud: Vec<BandTpag>,
    safahat: Vec<SafhaTxtr>,
    maraji: JadwalMaraji,
}

impl HawiyatGameMaker {
    /// Parses a container already in memory.
    ///
    /// The order of the steps is the order their dependencies run in, and not a
    /// more convenient one: the chunk table before any chunk, `GEN8`'s bytecode
    /// version before any chunk whose layout it selects, the string pool before
    /// anything that resolves a name through it, and the census last, because
    /// the census is the only step that needs all of the others.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the buffer exceeds [`AQSA_HAJM_HAWIYA`],
    /// [`KhataNusus::SihrGhayrMutabaq`] when it does not begin with `FORM`,
    /// [`KhataNusus::IsdarGhayrMadum`] for a bytecode version outside this
    /// build's band, [`KhataNusus::HawiyaTalifa`] when `GEN8` or `STRG` is
    /// missing or a chunk's contents leave the container, and whatever the
    /// per-chunk readers above report.
    pub fn min_bayt(bayt: Vec<u8>, masar: &Path) -> Result<Self, KhataNusus> {
        let tul = tul_u64(bayt.len());
        if tul > AQSA_HAJM_HAWIYA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the container's length",
                qeema: tul,
                saqf: AQSA_HAJM_HAWIYA,
            });
        }
        let jadwal = jadwal_qita(&bayt, masar)?;
        let mafqud = |haql: &'static str| KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql,
            qeema: 0,
            hadd: tul_u64(jadwal.len()),
        };

        let qita_gen8 = qita_bi_ism(&jadwal, ISM_GEN8).ok_or_else(|| mafqud("a GEN8 chunk"))?;
        let gen8 = iqra_gen8(&bayt, &qita_gen8)?;

        let qita_strg = qita_bi_ism(&jadwal, ISM_STRG).ok_or_else(|| mafqud("a STRG chunk"))?;
        let hawd = HawdNusus::iqra(&bayt, &qita_strg)?;

        let bunud = match qita_bi_ism(&jadwal, ISM_TPAG) {
            Some(qita) => iqra_tpag(&bayt, &qita)?,
            None => Vec::new(),
        };
        let safahat = match qita_bi_ism(&jadwal, ISM_TXTR) {
            Some(qita) => iqra_txtr(&bayt, &qita)?,
            None => Vec::new(),
        };
        let khutut = match qita_bi_ism(&jadwal, ISM_FONT) {
            Some(qita) => iqra_font(&bayt, &qita, &hawd)?,
            None => Vec::new(),
        };

        let maraji = ihsa_maraji(&bayt, &jadwal, gen8, &hawd, &khutut, &bunud, &safahat);
        Ok(Self { bayt, jadwal, gen8, hawd, khutut, bunud, safahat, maraji })
    }

    /// The container's bytes, exactly as they were read.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }

    /// The chunk table.
    #[must_use]
    pub fn jadwal(&self) -> &[QitaHawiya] {
        &self.jadwal
    }

    /// What `GEN8` said.
    #[must_use]
    pub const fn gen8(&self) -> BayanGen8 {
        self.gen8
    }

    /// Which generation wrote this container.
    #[must_use]
    pub const fn jeel(&self) -> JeelHawiya {
        self.gen8.jeel
    }

    /// The string pool.
    #[must_use]
    pub const fn hawd(&self) -> &HawdNusus {
        &self.hawd
    }

    /// The font entries.
    #[must_use]
    pub fn khutut(&self) -> &[KhattGameMaker] {
        &self.khutut
    }

    /// The texture page items.
    #[must_use]
    pub fn bunud(&self) -> &[BandTpag] {
        &self.bunud
    }

    /// The texture pages.
    #[must_use]
    pub fn safahat(&self) -> &[SafhaTxtr] {
        &self.safahat
    }

    /// The reference census.
    #[must_use]
    pub const fn maraji(&self) -> &JadwalMaraji {
        &self.maraji
    }

    /// The game's name, as `GEN8` points at it.
    #[must_use]
    pub fn ism_luba(&self) -> Option<&str> {
        let mu_ashir = hajm_usize(self.gen8.mawqi_ism).and_then(|ras| iqra_u32(&self.bayt, ras))?;
        let fahras = self.hawd.fahras_bi_izaha(u64::from(mu_ashir))?;
        self.hawd.madkhal(fahras).map(|madkhal| madkhal.nass.as_str())
    }

    /// The `TPAG` item a font draws from, by the pointer the entry holds.
    #[must_use]
    pub fn band_khatt(&self, khatt: &KhattGameMaker) -> Option<BandTpag> {
        self.bunud.iter().copied().find(|band| band.mawqi == khatt.band)
    }

    /// The last chunk in the table, which is where an appended region goes.
    #[must_use]
    pub fn qita_akhira(&self) -> Option<QitaHawiya> {
        self.jadwal.last().copied()
    }
}

/// Builds the reference census over an already-parsed container.
///
/// Every chunk this walks is marked enumerated only when the walk accounted for
/// all of it. `CODE`, `VARI` and `FUNC` are the interesting cases: each holds one
/// string pointer per entry and none of them is marked enumerated unless the
/// stride the walk used lands exactly on the chunk's end *and* every first field
/// it read was an exact pool body offset. A stride that is wrong by four bytes
/// fails the pool lookup within two entries; one wrong by a whole entry fails the
/// extent check. Neither can pass by accident, which is what makes the
/// relocation precondition mean something.
fn ihsa_maraji(
    bayt: &[u8],
    jadwal: &[QitaHawiya],
    gen8: BayanGen8,
    hawd: &HawdNusus,
    khutut: &[KhattGameMaker],
    bunud: &[BandTpag],
    safahat: &[SafhaTxtr],
) -> JadwalMaraji {
    let mut sijil = JadwalMaraji::jadeed();
    let nass_marja = |sijil: &mut JadwalMaraji, mawqi: u64| {
        let Some(hadaf) = hajm_usize(mawqi).and_then(|ras| iqra_u32(bayt, ras)).map(u64::from)
        else {
            return;
        };
        let naw = hawd.fahras_bi_izaha(hadaf).map_or(NawMarja::Majhul, NawMarja::Nass);
        sijil.sajjil(MarjaIzaha { mawqi, hadaf, naw });
    };

    // GEN8's string pointers: the project filename and configuration name at the
    // head of the chunk, then the two names the parse already located.
    if let Some(qita) = qita_bi_ism(jadwal, ISM_GEN8) {
        for izaha in [4_u64, 8] {
            if izaha.saturating_add(4) <= qita.tul {
                nass_marja(&mut sijil, qita.izaha.saturating_add(izaha));
            }
        }
    }
    nass_marja(&mut sijil, gen8.mawqi_ism);
    if let Some(mawqi) = gen8.mawqi_ard {
        nass_marja(&mut sijil, mawqi);
    }
    sijil.sajjil_qita(ISM_GEN8);

    // The pool's own pointer array. Every entry is a string reference by
    // construction, so this is the one walk that cannot fail.
    for (fahras, madkhal) in hawd.madakhil().iter().enumerate() {
        let Ok(raqm) = u32::try_from(fahras) else { break };
        sijil.sajjil(MarjaIzaha {
            mawqi: madkhal.mawqi_mu_ashir,
            hadaf: madkhal.izaha,
            naw: NawMarja::Nass(raqm),
        });
    }
    sijil.sajjil_qita(ISM_STRG);

    // TPAG items are named by the chunk's own array and name nothing themselves.
    for (fahras, band) in bunud.iter().enumerate() {
        let Ok(raqm) = u32::try_from(fahras) else { break };
        let mawqi = qita_bi_ism(jadwal, ISM_TPAG).map(|qita| {
            qita.izaha.saturating_add(4).saturating_add(tul_u64(fahras).saturating_mul(4))
        });
        if let Some(mawqi) = mawqi {
            sijil.sajjil(MarjaIzaha { mawqi, hadaf: band.mawqi, naw: NawMarja::Band(raqm) });
        }
    }
    if qita_bi_ism(jadwal, ISM_TPAG).is_some() {
        sijil.sajjil_qita(ISM_TPAG);
    }

    // TXTR: the entry pointer and, inside each entry, the verified blob pointer.
    for (fahras, safha) in safahat.iter().enumerate() {
        let Ok(raqm) = u32::try_from(fahras) else { break };
        if let Some(qita) = qita_bi_ism(jadwal, ISM_TXTR) {
            sijil.sajjil(MarjaIzaha {
                mawqi: qita
                    .izaha
                    .saturating_add(4)
                    .saturating_add(tul_u64(fahras).saturating_mul(4)),
                hadaf: safha.mawqi_madkhal,
                naw: NawMarja::Safha(raqm),
            });
        }
        sijil.sajjil(MarjaIzaha {
            mawqi: safha.mawqi_mu_ashir,
            hadaf: safha.izaha,
            naw: NawMarja::Safha(raqm),
        });
    }
    if qita_bi_ism(jadwal, ISM_TXTR).is_some() {
        sijil.sajjil_qita(ISM_TXTR);
    }

    // FONT: the entry pointer, the two names, the TPAG pointer, the glyph array.
    for (fahras, khatt) in khutut.iter().enumerate() {
        let Ok(raqm) = u32::try_from(fahras) else { break };
        sijil.sajjil(MarjaIzaha {
            mawqi: khatt.mawqi_mu_ashir,
            hadaf: khatt.mawqi,
            naw: NawMarja::MadkhalKhatt(raqm),
        });
        nass_marja(&mut sijil, khatt.mawqi_ism);
        nass_marja(&mut sijil, khatt.mawqi_ism.saturating_add(4));
        sijil.sajjil(MarjaIzaha {
            mawqi: khatt.mawqi_band,
            hadaf: khatt.band,
            naw: NawMarja::Band(raqm),
        });
        for shakl in 0..u32::try_from(khatt.ashkal.len()).unwrap_or(u32::MAX) {
            let mawqi = khatt
                .mawqi_adad_ashkal
                .saturating_add(4)
                .saturating_add(u64::from(shakl).saturating_mul(4));
            let Some(hadaf) = hajm_usize(mawqi).and_then(|ras| iqra_u32(bayt, ras)).map(u64::from)
            else {
                continue;
            };
            sijil.sajjil(MarjaIzaha { mawqi, hadaf, naw: NawMarja::ShaklKhatt(raqm, shakl) });
        }
    }
    if qita_bi_ism(jadwal, ISM_FONT).is_some() {
        sijil.sajjil_qita(ISM_FONT);
    }

    ihsa_asmaa_qita(bayt, jadwal, hawd, &mut sijil);
    sijil
}

/// Walks the three chunks whose entries each carry one string pointer.
///
/// `CODE` is named by its own pointer array, so its walk is exact. `VARI` and
/// `FUNC` are contiguous runs whose entry width changed with the generation, so
/// each is tried against the strides this build knows and accepted only when the
/// run verifies — see [`maraji_mutatabia`].
fn ihsa_asmaa_qita(
    bayt: &[u8],
    jadwal: &[QitaHawiya],
    hawd: &HawdNusus,
    sijil: &mut JadwalMaraji,
) {
    if let Some(qita) = qita_bi_ism(jadwal, ISM_CODE)
        && let Some(maraji) = maraji_masfufa(bayt, &qita, hawd)
    {
        for marja in maraji {
            sijil.sajjil(marja);
        }
        sijil.sajjil_qita(ISM_CODE);
    }

    // VARI: an optional header, then fixed-width entries to the chunk's end.
    if let Some(qita) = qita_bi_ism(jadwal, ISM_VARI) {
        for jeel in [JeelHawiya::Studio2, JeelHawiya::Studio1] {
            let (tarwisa, khutwa) = shakl_vari(jeel);
            let bidaya = qita.izaha.saturating_add(tarwisa);
            if let Some(maraji) = maraji_mutatabia(bayt, bidaya, qita.nihaya(), khutwa, hawd) {
                for marja in maraji {
                    sijil.sajjil(marja);
                }
                sijil.sajjil_qita(ISM_VARI);
                break;
            }
        }
    }

    // FUNC: twelve-byte entries in every generation, but preceded by a count in
    // some and by nothing in others, and followed by a locals table in the newer
    // ones. Only the shape that verifies end to end is accepted.
    if let Some(qita) = qita_bi_ism(jadwal, ISM_FUNC) {
        for tarwisa in [0_u64, 4] {
            let bidaya = qita.izaha.saturating_add(tarwisa);
            if let Some(maraji) = maraji_mutatabia(bayt, bidaya, qita.nihaya(), 12, hawd) {
                for marja in maraji {
                    sijil.sajjil(marja);
                }
                sijil.sajjil_qita(ISM_FUNC);
                break;
            }
        }
    }

    // AUDO: a count and a pointer array into its own chunk, and nothing else
    // that is an offset. It carries no string references at all, and it is
    // walked anyway — because it sits behind STRG and TXTR, so every one of its
    // blob pointers is a field that a growing pool would otherwise silently
    // invalidate.
    if let Some(qita) = qita_bi_ism(jadwal, ISM_AUDO)
        && let Some(maraji) = maraji_kutal(bayt, &qita)
    {
        for marja in maraji {
            sijil.sajjil(marja);
        }
        sijil.sajjil_qita(ISM_AUDO);
    }
}

/// The pointer array of a chunk that is a count, an array and a run of blobs.
///
/// `AUDO`'s shape. Returns [`None`] when the count or any pointer leaves the
/// chunk, which the caller reads as "not enumerated".
fn maraji_kutal(bayt: &[u8], qita: &QitaHawiya) -> Option<Vec<MarjaIzaha>> {
    let adad = iqra_u32(bayt, hajm_usize(qita.izaha)?)?;
    if u64::from(adad).checked_mul(4)?.checked_add(4)? > qita.tul {
        return None;
    }
    let mut maraji: Vec<MarjaIzaha> = Vec::with_capacity(usize::try_from(adad).ok()?);
    for fahras in 0..adad {
        let mawqi =
            qita.izaha.checked_add(4)?.checked_add(u64::from(fahras).checked_mul(4)?)?;
        let hadaf = u64::from(iqra_u32(bayt, hajm_usize(mawqi)?)?);
        if !qita.yahwi(hadaf) {
            return None;
        }
        maraji.push(MarjaIzaha { mawqi, hadaf, naw: NawMarja::Sawt(fahras) });
    }
    Some(maraji)
}

/// The name pointers of a chunk laid out as a count, a pointer array and
/// entries whose first field is a string pointer.
///
/// `CODE`'s shape, and the only one of the three that needs no stride guess: the
/// array names every entry outright. Returns [`None`] when any pointer leaves the
/// chunk or any first field is not an exact pool body offset, because either
/// means the chunk is not the shape this build believes and marking it
/// enumerated would be a lie the relocation planner acts on.
fn maraji_masfufa(bayt: &[u8], qita: &QitaHawiya, hawd: &HawdNusus) -> Option<Vec<MarjaIzaha>> {
    let ras = hajm_usize(qita.izaha)?;
    let adad = iqra_u32(bayt, ras)?;
    if u64::from(adad).checked_mul(4)?.checked_add(4)? > qita.tul {
        return None;
    }
    let mut maraji: Vec<MarjaIzaha> = Vec::with_capacity(usize::try_from(adad).ok()?);
    for fahras in 0..adad {
        let khana = qita
            .izaha
            .checked_add(4)?
            .checked_add(u64::from(fahras).checked_mul(4)?)?;
        let madkhal = u64::from(iqra_u32(bayt, hajm_usize(khana)?)?);
        if !qita.yahwi(madkhal) {
            return None;
        }
        let hadaf = u64::from(iqra_u32(bayt, hajm_usize(madkhal)?)?);
        let nass = hawd.fahras_bi_izaha(hadaf)?;
        maraji.push(MarjaIzaha { mawqi: madkhal, hadaf, naw: NawMarja::Nass(nass) });
    }
    Some(maraji)
}

// ---------------------------------------------------------------------------
// The rewriting plan
// ---------------------------------------------------------------------------

/// Which of the three mechanisms a change used.
///
/// Recorded per change and reported in [`WasfIstratijiya`], because the three
/// have different failure modes and a maintainer looking at a game that broke
/// needs to know which one was in play. See this module's header for what each
/// one does and what it costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IstratijiyaIzaha {
    /// Written into the bytes it replaced. Nothing moved.
    Mawdi,
    /// Written into an appended region, with the pointer that named the old
    /// structure repointed at it. Nothing before the region moved.
    Ilhaq,
    /// Everything past a frontier moved by a fixed delta.
    Izaha,
}

impl IstratijiyaIzaha {
    /// The name a log line and the capability report use.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mawdi => "in place",
            Self::Ilhaq => "appended and repointed",
            Self::Izaha => "relocated",
        }
    }
}

/// One planned in-place overwrite, in the original container's coordinates.
///
/// `qadeem` is not documentation. It is checked immediately before the write,
/// and a mismatch is a refusal: a plan built against one parse and applied to a
/// different buffer would otherwise overwrite whatever happened to be there. The
/// only cost is one comparison per edit, against a patcher that silently writes
/// a pointer into the middle of a sprite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TahreerBayt {
    /// Where the edit goes, in the original container's coordinates.
    pub mawqi: u64,
    /// What must be there before the write.
    pub qadeem: Vec<u8>,
    /// What to write.
    pub jadeed: Vec<u8>,
    /// What the edit is for, for the diagnostics bundle.
    pub sabab: &'static str,
}

/// One planned insertion: bytes that go **before** an original offset.
///
/// Every insertion's length is rounded up to [`MUHADHAHA_SAFHA`] with `NUL`
/// padding. That is not tidiness: `TXTR` blobs sit on that boundary and the
/// runtime uploads some of them straight from the mapped file, so an insertion
/// of an odd length anywhere in front of `TXTR` would move every page off its
/// alignment and produce a game that renders correctly on four machines out of
/// five. Padding every insertion to the coarsest alignment in the container
/// makes that impossible rather than merely unlikely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdrajBayt {
    /// The original offset the bytes go before.
    pub mawqi: u64,
    /// The bytes, already padded to [`MUHADHAHA_SAFHA`].
    pub bayt: Vec<u8>,
    /// Which chunk grows by this insertion, so its length header can be fixed.
    pub qita: [u8; 4],
    /// What the block holds.
    ///
    /// Tagged rather than identified by position, because several blocks can
    /// share an insertion offset — the last chunk in a container is sometimes
    /// `TXTR` itself — and a writer that told them apart by their order in a
    /// sorted list would resolve one of them to the other's address the first
    /// time a container arrived in that shape.
    pub naw: NawIdraj,
}

/// What an inserted block holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NawIdraj {
    /// String records appended to the end of the pool.
    Hawd,
    /// New pointer slots for the texture page array.
    MasfufatSafahat,
    /// New texture page entries.
    MadakhilSafahat,
    /// New texture blobs.
    KutalSafahat,
    /// The appended region: rebuilt font entries and their glyph tables.
    Ilhaq,
}

/// The map from an offset in the original container to its offset in the output.
///
/// Built from the insertion list once, and then consulted for every reference in
/// the census. Two queries, and the difference between them is the whole
/// subtlety of an insertion:
///
/// * [`KhareetatIzaha::izaha_jadida`] maps an **original byte**. An insertion at
///   `m` sits between the original bytes `m-1` and `m`, so a byte at or after
///   `m` moves and a byte before it does not.
/// * [`KhareetatIzaha::bidayat_idraj`] gives where the **inserted block itself**
///   begins, which is before the byte that used to be at `m` and therefore is
///   *not* what the first query returns.
///
/// Confusing the two puts every new string one block-length past itself, which
/// is a game that starts, runs, and draws the wrong text — the failure that is
/// hardest to attribute and therefore the one worth a paragraph.
#[derive(Debug, Clone, Default)]
pub struct KhareetatIzaha {
    nuqat: Vec<(u64, u64)>,
}

impl KhareetatIzaha {
    /// Builds the map from a list of insertions.
    ///
    /// The list is sorted by offset; insertions at the same offset keep the
    /// order they were planned in, because that is the order they are written in
    /// and the order [`KhareetatIzaha::bidayat_idraj`] accounts for.
    #[must_use]
    pub fn jadeed(idrajat: &[IdrajBayt]) -> Self {
        let mut nuqat: Vec<(u64, u64)> = idrajat
            .iter()
            .map(|idraj| (idraj.mawqi, tul_u64(idraj.bayt.len())))
            .collect();
        nuqat.sort_by_key(|(mawqi, _)| *mawqi);
        Self { nuqat }
    }

    /// The total number of bytes inserted.
    #[must_use]
    pub fn majmu(&self) -> u64 {
        self.nuqat.iter().fold(0_u64, |kull, (_, tul)| kull.saturating_add(*tul))
    }

    /// Where an original byte ends up in the output.
    #[must_use]
    pub fn izaha_jadida(&self, qadeem: u64) -> u64 {
        self.nuqat
            .iter()
            .filter(|(mawqi, _)| *mawqi <= qadeem)
            .fold(qadeem, |mawdi, (_, tul)| mawdi.saturating_add(*tul))
    }

    /// Where the `tartib`-th block inserted at `mawqi` begins in the output.
    #[must_use]
    pub fn bidayat_idraj(&self, mawqi: u64, tartib: usize) -> u64 {
        let qabl = self
            .nuqat
            .iter()
            .filter(|(mawdi, _)| *mawdi < mawqi)
            .fold(0_u64, |kull, (_, tul)| kull.saturating_add(*tul));
        let dakhil = self
            .nuqat
            .iter()
            .filter(|(mawdi, _)| *mawdi == mawqi)
            .take(tartib)
            .fold(0_u64, |kull, (_, tul)| kull.saturating_add(*tul));
        mawqi.saturating_add(qabl).saturating_add(dakhil)
    }

    /// Whether anything moves at all.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.nuqat.is_empty()
    }

    /// The lowest offset at which anything moves, if anything does.
    ///
    /// The relocation frontier: every chunk at or after this offset has to be
    /// one whose pointers the census enumerated, or the plan is refused.
    #[must_use]
    pub fn hadd_haraka(&self) -> Option<u64> {
        self.nuqat.first().map(|(mawqi, _)| *mawqi)
    }
}

/// Pads a block to the container's coarsest alignment with `NUL`.
fn hashw_muhadhah(mut bayt: Vec<u8>) -> Vec<u8> {
    let tul = tul_u64(bayt.len());
    let matlub = muhadhah(tul, MUHADHAHA_SAFHA).unwrap_or(tul);
    if let Some(hashw) = matlub.checked_sub(tul_u64(bayt.len()))
        && let Some(adad) = hajm_usize(hashw)
    {
        bayt.resize(bayt.len().saturating_add(adad), 0);
    }
    bayt
}

// ---------------------------------------------------------------------------
// The patcher
// ---------------------------------------------------------------------------

/// Rewrites a `data.win`.
///
/// Nothing is written into the container while a plan is being built. Every
/// change is recorded as an overwrite or an insertion in the *original*
/// container's coordinates, and [`MuhawwilGameMaker::ukhruj`] applies the whole
/// plan once. That inversion is what makes "byte-faithful for everything
/// untouched" a property of the design rather than a hope: the output is the
/// original buffer with exactly the planned edits spliced into it, so a byte
/// nobody named cannot change, and [`MuhawwilGameMaker::tahaqquq_dawra`] proves
/// it for the produced bytes rather than assuming it.
#[derive(Debug, Clone)]
pub struct MuhawwilGameMaker {
    hawiya: HawiyatGameMaker,
    hawd_mulhaq: Vec<u8>,
    mawadi_nusus: BTreeMap<u32, u64>,
    safahat_mulhaqa: Vec<Vec<u8>>,
    mulhaq_akhir: Vec<u8>,
    maraji_muajjala: Vec<MarjaMuajjal>,
    tahrirat: Vec<TahreerBayt>,
    istratijiyat: BTreeSet<IstratijiyaIzaha>,
    mawdi_ilhaq: MawdiIlhaq,
    naqala: bool,
    adad_nusus: u32,
    adad_ashkal: u32,
}

impl MuhawwilGameMaker {
    /// A patcher over a parsed container.
    #[must_use]
    pub const fn jadeed(hawiya: HawiyatGameMaker) -> Self {
        Self {
            hawiya,
            hawd_mulhaq: Vec::new(),
            mawadi_nusus: BTreeMap::new(),
            safahat_mulhaqa: Vec::new(),
            mulhaq_akhir: Vec::new(),
            maraji_muajjala: Vec::new(),
            tahrirat: Vec::new(),
            istratijiyat: BTreeSet::new(),
            mawdi_ilhaq: MawdiIlhaq::TadhyeelAkhir,
            naqala: false,
            adad_nusus: 0,
            adad_ashkal: 0,
        }
    }

    /// Chooses where the appended region goes.
    ///
    /// Both options are described on [`MawdiIlhaq`], with what each one assumes
    /// about the runtime. The default is the one that introduces no new chunk
    /// name.
    pub const fn bi_mawdi_ilhaq(&mut self, mawdi: MawdiIlhaq) -> &mut Self {
        self.mawdi_ilhaq = mawdi;
        self
    }

    /// Where the appended region goes.
    #[must_use]
    pub const fn mawdi_ilhaq(&self) -> MawdiIlhaq {
        self.mawdi_ilhaq
    }

    /// The container this patcher was built over.
    #[must_use]
    pub const fn hawiya(&self) -> &HawiyatGameMaker {
        &self.hawiya
    }

    /// Every in-place overwrite the plan holds so far.
    #[must_use]
    pub fn tahrirat(&self) -> &[TahreerBayt] {
        &self.tahrirat
    }

    /// Which strategies the plan has used.
    pub fn istratijiyat(&self) -> impl Iterator<Item = IstratijiyaIzaha> + '_ {
        self.istratijiyat.iter().copied()
    }

    /// Records an overwrite, capturing the bytes it is replacing as its guard.
    fn sajjil_tahreer(
        &mut self,
        mawqi: u64,
        jadeed: Vec<u8>,
        sabab: &'static str,
    ) -> Result<(), KhataNusus> {
        let tul = tul_u64(self.hawiya.bayt.len());
        let ras = hajm_usize(mawqi).ok_or(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "an edit offset",
            qeema: mawqi,
            hadd: tul,
        })?;
        let nihaya = ras.checked_add(jadeed.len()).ok_or(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "an edit extent",
            qeema: mawqi,
            hadd: tul,
        })?;
        let qadeem = self
            .hawiya
            .bayt
            .get(ras..nihaya)
            .ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "an edit running past the container",
                qeema: tul_u64(nihaya),
                hadd: tul,
            })?
            .to_vec();
        self.tahrirat.push(TahreerBayt { mawqi, qadeem, jadeed, sabab });
        Ok(())
    }

    /// Replaces one string in the pool.
    ///
    /// Two paths, and which one is taken is decided by arithmetic rather than by
    /// preference:
    ///
    /// * When the replacement's record fits the bytes the old record occupied,
    ///   it is written there and the remainder is `NUL`-filled. Nothing moves;
    ///   not one pointer in the container is touched, including the pointers in
    ///   chunks this build never parsed.
    /// * Otherwise the new record is appended to the end of the `STRG` chunk and
    ///   **every** reference to the old body is repointed — the pool's own array
    ///   slot and every site the census found. `STRG` grows, so everything behind
    ///   it moves, and [`MuhawwilGameMaker::ukhruj`] checks the relocation
    ///   precondition before a byte is produced.
    ///
    /// The pool's **order is never changed**: no string is inserted, removed or
    /// swapped, only rewritten. That is a hard requirement rather than a
    /// convenience, because the bytecode in `CODE` refers to strings by pool
    /// index and not by offset, and reordering the pool would silently rewrite
    /// every literal in the game's script to a different literal.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HawiyaTalifa`] when `fahras` is not a pool index or a
    /// reference site leaves the container, and [`KhataNusus::HajmMufrit`] when
    /// the replacement is longer than [`AQSA_TUL_NASS`].
    pub fn badil_nass(&mut self, fahras: u32, nass: &str) -> Result<(), KhataNusus> {
        let tul_jadeed = tul_u64(nass.len());
        if tul_jadeed > u64::from(AQSA_TUL_NASS) {
            return Err(KhataNusus::HajmMufrit {
                haql: "a replacement string",
                qeema: tul_jadeed,
                saqf: u64::from(AQSA_TUL_NASS),
            });
        }
        let madkhal =
            self.hawiya.hawd.madkhal(fahras).cloned().ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "a pool index this container does not have",
                qeema: u64::from(fahras),
                hadd: tul_u64(self.hawiya.hawd.adad()),
            })?;

        let sijil = sijil_nass(nass);
        if tul_u64(sijil.len()) <= madkhal.hajm_sijil() {
            let mut kutla = sijil;
            let hashw = hajm_usize(madkhal.hajm_sijil().saturating_sub(tul_u64(kutla.len())))
                .unwrap_or_default();
            kutla.resize(kutla.len().saturating_add(hashw), 0);
            self.sajjil_tahreer(madkhal.mawqi_sijil, kutla, "a string rewritten in place")?;
            let _ = self.istratijiyat.insert(IstratijiyaIzaha::Mawdi);
            self.adad_nusus = self.adad_nusus.saturating_add(1);
            return Ok(());
        }

        // The record does not fit. It is appended and every reference to it is
        // repointed; the offsets are resolved at output time, when the map from
        // original offsets to output offsets exists.
        let izaha_dakhili = tul_u64(self.hawd_mulhaq.len()).saturating_add(4);
        self.hawd_mulhaq.extend_from_slice(&sijil);
        let _ = self.mawadi_nusus.insert(fahras, izaha_dakhili);
        let _ = self.istratijiyat.insert(IstratijiyaIzaha::Ilhaq);
        self.adad_nusus = self.adad_nusus.saturating_add(1);
        Ok(())
    }

    /// How many strings the plan replaces.
    #[must_use]
    pub const fn adad_nusus(&self) -> u32 {
        self.adad_nusus
    }
}

/// Serializes one pool record: the length, the bytes, the terminating `NUL`.
fn sijil_nass(nass: &str) -> Vec<u8> {
    let mut khaam = Vec::with_capacity(nass.len().saturating_add(5));
    khaam.extend_from_slice(&u32::try_from(nass.len()).unwrap_or(u32::MAX).to_le_bytes());
    khaam.extend_from_slice(nass.as_bytes());
    khaam.push(0);
    khaam
}

/// Where an appended region goes.
///
/// Both options put the bytes past every existing structure, so neither moves
/// anything. They differ only in how the container's own framing describes them,
/// and each has a cost this module states rather than hides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum MawdiIlhaq {
    /// Extend the payload of the last chunk in the table.
    ///
    /// The default, and the safer of the two. The last chunk in every container
    /// this format produces is a count, a pointer array and a run of blobs, and
    /// a reader that walks it by its own count never reaches trailing bytes. No
    /// new chunk name is introduced, so no dispatch on chunk names is involved
    /// at all.
    ///
    /// **The cost:** it assumes the last chunk tolerates bytes past its declared
    /// contents. Every generation's last chunk does, and this build checks that
    /// the chunk is one whose extent it enumerated before choosing this.
    #[default]
    TadhyeelAkhir,
    /// Add a chunk named [`ISM_QITA_ILHAQ`] after the last one.
    ///
    /// **The cost:** it assumes the runtime skips a chunk name it does not know.
    /// The format is chunked precisely so that it can, and every runtime this
    /// product has been pointed at does — but it is an assumption about code
    /// nobody here can read, and it is why this is not the default.
    QitaJadida,
}

impl MawdiIlhaq {
    /// The name a log line and the capability report use.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::TadhyeelAkhir => "extending the last chunk",
            Self::QitaJadida => "a new TARB chunk",
        }
    }
}

/// The vertical metrics a generated GameMaker font needs.
///
/// GameMaker's glyph record has a horizontal offset and an advance and **no
/// vertical offset at all** — see [`ShaklKhatt`]. A glyph is drawn with the top
/// of its rectangle at the top of the line box, so every glyph's vertical
/// placement has to be baked into the page as transparent rows above and below
/// the ink.
///
/// That is a constraint on the atlas rather than on this module, and it is
/// checked here rather than assumed: the page handed to
/// [`MuhawwilGameMaker::istabdil_khatt`] must be packed in cells `irtifa` tall
/// with the baseline `suud` rows down, and every glyph whose atlas rectangle
/// cannot be expanded to that cell without leaving the page is refused by name.
/// A glyph placed by a rectangle that was silently clamped would draw a letter
/// half a line high, which reads to a player as a broken font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QiyasatKhattNaql {
    /// How far the baseline sits below the top of the line box, in pixels.
    pub suud: u16,
    /// The line box's height, in pixels.
    pub irtifa: u16,
}

impl QiyasatKhattNaql {
    /// Builds the metrics.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhattMarfud`] when the line height is zero or smaller than
    /// the ascent, which would make every line of the game's text overlap the
    /// one above it.
    pub fn jadeed(suud: u16, irtifa: u16) -> Result<Self, KhataNusus> {
        if irtifa == 0 || irtifa < suud {
            return Err(KhataNusus::KhattMarfud {
                sabab: format!(
                    "the generated font's line height is {irtifa} and its ascent is {suud}; \
                     every line would overlap the one above it"
                ),
            });
        }
        Ok(Self { suud, irtifa })
    }
}

impl MuhawwilGameMaker {
    /// Appends a texture page and returns the index that will name it.
    ///
    /// The page grows `TXTR` in three places — the pointer array, the entry run
    /// and the blob run — so this is the one operation in the module that uses
    /// the relocating strategy. It is tractable because `TXTR` is next to last in
    /// every generation's layout: the only chunk behind it is `AUDO`, whose
    /// pointers the census enumerated. [`MuhawwilGameMaker::ukhruj`] rechecks
    /// that before producing a byte.
    ///
    /// The new page entry is a **copy of the last existing entry** with its blob
    /// pointer and, when one is present, its declared blob length rewritten. The
    /// entry's layout changed four times across the format's history and is not
    /// reproduced from a version number here; copying an entry the container
    /// itself wrote is the only way to get a field this build has never heard of
    /// right.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::JadwalAshkalMarfud`] when the container has no `TXTR` chunk
    /// or no existing page to copy an entry from, when the blob does not begin
    /// with a PNG or QOI signature, or when the copied entry holds more than one
    /// field that could be a declared length. [`KhataNusus::HajmMufrit`] when the
    /// page count would exceed [`AQSA_SAFAHAT`].
    pub fn alhiq_safha(&mut self, kutla: &[u8]) -> Result<u16, KhataNusus> {
        let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
        if !(kutla.starts_with(&SIHR_PNG)
            || kutla.starts_with(&SIHR_QOI)
            || kutla.starts_with(&SIHR_QOI_QIYASI))
        {
            return Err(marfud(
                "the atlas page is not a PNG or a QOI image, which are the only two \
                 encodings a TXTR blob carries"
                    .to_owned(),
            ));
        }
        if self.hawiya.safahat.is_empty() {
            return Err(marfud(
                "this container has no texture page to copy an entry layout from, so a new \
                 page cannot be described in a shape its runtime reads"
                    .to_owned(),
            ));
        }
        let mawjud = tul_u64(self.hawiya.safahat.len());
        let jadeed = mawjud.saturating_add(tul_u64(self.safahat_mulhaqa.len()));
        if jadeed >= u64::from(AQSA_SAFAHAT) {
            return Err(KhataNusus::HajmMufrit {
                haql: "pages in the TXTR chunk after appending",
                qeema: jadeed.saturating_add(1),
                saqf: u64::from(AQSA_SAFAHAT),
            });
        }
        let fahras = u16::try_from(jadeed)
            .map_err(|_| marfud("more texture pages than a page index can name".to_owned()))?;
        self.safahat_mulhaqa.push(kutla.to_vec());
        let _ = self.istratijiyat.insert(IstratijiyaIzaha::Izaha);
        Ok(fahras)
    }

    /// Replaces a font's glyph table with one built from the glyph transport.
    ///
    /// The steps, in the order they have to run:
    ///
    /// 1. every slot in the request's assignment is resolved against the atlas,
    ///    and its cell in the page is computed from the request's metrics — the
    ///    expansion described on [`QiyasatKhattNaql`], refused rather than
    ///    clamped when it will not fit;
    /// 2. the glyph records are built in ascending slot order, so two builds of
    ///    one patch produce byte-identical output;
    /// 3. the whole font entry — its fixed fields copied from the original, its
    ///    glyph pointer array and its glyph records — is appended, and the
    ///    `FONT` chunk's pointer array slot is repointed at it, so the `FONT`
    ///    chunk's own size never changes and nothing in front of `TPAG`, `CODE`,
    ///    `VARI` or `FUNC` moves;
    /// 4. the font's `TPAG` item is repointed at the appended page, covering all
    ///    of it.
    ///
    /// **No character reaches this function.** The only key it can build is
    /// [`MiftahKhana`], which has a font index, a size and a glyph identifier and
    /// no field for a codepoint — so there is no path here from a character to a
    /// slot, and the slots it writes go into the glyph table and nowhere else.
    /// That is the transport's invariant, preserved rather than restated.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::JadwalAshkalMarfud`] when the font index is not in the
    /// container, when the atlas does not hold a slot's glyph, when a glyph's
    /// cell will not fit the page, when a metric does not fit the sixteen-bit
    /// field the record gives it, or when the table would exceed
    /// [`AQSA_ASHKAL_KHATT`] entries. [`KhataNusus::KhattMarfud`] when the font
    /// has no `TPAG` item to repoint.
    pub fn istabdil_khatt(&mut self, talab: &TalabKhatt<'_>) -> Result<u32, KhataNusus> {
        let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
        let fahras_khatt = talab.khatt;
        let khatt = self
            .hawiya
            .khutut
            .get(usize::try_from(fahras_khatt).unwrap_or(usize::MAX))
            .cloned()
            .ok_or_else(|| {
                marfud(format!("this container has no font {fahras_khatt} to replace"))
            })?;
        let (ard_safha, irtifa_safha) = talab
            .lawha
            .qiyas_safha(talab.safha_lawha)
            .ok_or_else(|| marfud(format!("the atlas has no page {}", talab.safha_lawha)))?;

        let ashkal = ibni_ashkal(talab, (ard_safha, irtifa_safha))?;
        let adad = u32::try_from(ashkal.len())
            .map_err(|_| marfud("more glyphs than a glyph count can name".to_owned()))?;
        if adad > AQSA_ASHKAL_KHATT {
            return Err(KhataNusus::HajmMufrit {
                haql: "glyphs in a generated font table",
                qeema: u64::from(adad),
                saqf: u64::from(AQSA_ASHKAL_KHATT),
            });
        }

        let band = self.hawiya.band_khatt(&khatt).ok_or_else(|| KhataNusus::KhattMarfud {
            sabab: format!(
                "font {fahras_khatt} ({}) points at {} for its texture region and no TPAG \
                 item sits there, so there is nothing to repoint at the new page",
                khatt.ism, khatt.band
            ),
        })?;
        let mustatil = MustatilSafha { s: 0, a: 0, ard: ard_safha, irtifa: irtifa_safha };
        self.sajjil_tahreer_band(band, talab.safha_hawiya, mustatil)?;

        self.alhiq_madkhal_khatt(&khatt, &ashkal)?;
        self.naqala = true;
        self.adad_ashkal = self.adad_ashkal.saturating_add(adad);
        let _ = self.istratijiyat.insert(IstratijiyaIzaha::Ilhaq);
        Ok(adad)
    }

    /// How many glyphs the plan writes into generated tables.
    #[must_use]
    pub const fn adad_ashkal(&self) -> u32 {
        self.adad_ashkal
    }

    /// Whether the plan used the glyph transport, and therefore costs the player
    /// selectable, searchable text.
    #[must_use]
    pub const fn naqala(&self) -> bool {
        self.naqala
    }
}

/// What [`MuhawwilGameMaker::istabdil_khatt`] needs in order to rebuild a font.
///
/// A request rather than seven parameters, so that the per-pair advance
/// corrections can be supplied by a caller that has them and omitted by one that
/// does not, without either shape being the odd one out.
#[derive(Clone, Copy)]
pub struct TalabKhatt<'a> {
    /// Which font in the container to replace, by index into
    /// [`HawiyatGameMaker::khutut`].
    pub khatt: u32,
    /// The frozen slot assignment: every glyph the patch transports, with the
    /// private-use codepoint that addresses it.
    pub tawzee: &'a TawzeeKhanat,
    /// Where the glyph images are.
    ///
    /// The atlas's rasterization mode is not asked for and is not part of this
    /// request: an atlas is compiled in one mode, so the source already knows
    /// which images it holds, and a second statement of it here would be a
    /// second answer to a settled question.
    pub lawha: &'a dyn MasdarLawha,
    /// Which page of the **atlas** the glyphs are on.
    ///
    /// Deliberately separate from [`TalabKhatt::safha_hawiya`]. The two are
    /// different index spaces — one numbers the pages `taarib-lawha` packed, the
    /// other numbers the pages this container carries — and they coincide only
    /// by accident. A single field would work on the first game anybody tried
    /// and put the font on somebody else's texture on the second.
    pub safha_lawha: u16,
    /// Which page of the **container** the font's `TPAG` item should name, as
    /// [`MuhawwilGameMaker::alhiq_safha`] numbered it.
    pub safha_hawiya: u16,
    /// The vertical metrics the generated font declares.
    pub qiyasat: QiyasatKhattNaql,
    /// Per-ordered-pair advance corrections, in pixels added to the first
    /// slot's advance.
    ///
    /// This is where the shaper's `GPOS` adjustments are carried, and it is a
    /// slice rather than a computed table because the pair data belongs to the
    /// transport that laid the lines out, not to the container that draws them.
    /// An empty slice is a font whose advances are each slot's own, which is
    /// correct whenever the layout produced no per-pair adjustment.
    pub azwaj: &'a [(char, char, i16)],
}

impl fmt::Debug for TalabKhatt<'_> {
    /// Prints the request's numbers and never a slot's characters.
    ///
    /// Invariant 3 of the transport, honoured at this boundary too: a derived
    /// `Debug` would put the pair list's private-use codepoints into any log
    /// line that formatted a request, and a run of them in a bug report is
    /// exactly the thing somebody copies out and mistakes for Arabic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TalabKhatt")
            .field("khatt", &self.khatt)
            .field("safha_lawha", &self.safha_lawha)
            .field("safha_hawiya", &self.safha_hawiya)
            .field("qiyasat", &self.qiyasat)
            .field("adad_khanat", &self.tawzee.adad())
            .field("adad_azwaj", &self.azwaj.len())
            .finish()
    }
}

/// Builds the glyph table from a frozen slot assignment.
///
/// In ascending slot order, which is ascending codepoint order, which is the
/// order [`TawzeeKhanat::tawzee`] yields — so two builds of one patch produce
/// byte-identical records and a diff between them describes a real change.
fn ibni_ashkal(
    talab: &TalabKhatt<'_>,
    qiyas_safha: (u16, u16),
) -> Result<Vec<ShaklKhatt>, KhataNusus> {
    let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
    let (_, irtifa_safha) = qiyas_safha;
    let mut ashkal: Vec<ShaklKhatt> = Vec::with_capacity(talab.tawzee.adad());

    for (miftah, khana) in talab.tawzee.tawzee() {
        // GameMaker's glyph record names its character with a `u16`. That is the
        // sixteen-bit character type the transport's header warns about, so an
        // astral slot is not narrowly addressable here — it is not addressable
        // at all, and a table built from one would draw nothing for every glyph
        // in it. Refused by name rather than truncated.
        let harf = u16::try_from(u32::from(khana)).map_err(|_| {
            marfud(format!(
                "slot U+{:04X} for {miftah} is above U+FFFF and a GameMaker glyph names its \
                 character with a sixteen-bit field; this patch's transport has to be built \
                 in the basic-plane slot mode",
                u32::from(khana)
            ))
        })?;
        let mawdi = talab
            .lawha
            .mawdi(miftah)
            .ok_or_else(|| marfud(format!("the atlas does not hold {miftah}")))?;
        if mawdi.safha != talab.safha_lawha {
            return Err(marfud(format!(
                "{miftah} is on atlas page {} and this font was given atlas page {}",
                mawdi.safha, talab.safha_lawha
            )));
        }
        ashkal.push(shakl_min_mawdi(miftah, harf, mawdi, talab.qiyasat, irtifa_safha)?);
    }

    adkhil_azwaj(&mut ashkal, talab.azwaj);
    Ok(ashkal)
}

/// Turns one atlas position into one glyph record.
///
/// The cell expansion is the whole of the vertical placement: GameMaker draws a
/// glyph with the top of its rectangle at the top of the line box, so the
/// rectangle has to start `suud - izaha_a` rows above the ink and run the full
/// line height. Both edges are checked against the page, and a cell that would
/// leave it is refused by name — a clamped rectangle draws a letter cut in half,
/// which reads to a player as a corrupt font rather than as a packing bug.
fn shakl_min_mawdi(
    miftah: MiftahKhana,
    harf: u16,
    mawdi: MawdiShakl,
    qiyasat: QiyasatKhattNaql,
    irtifa_safha: u16,
) -> Result<ShaklKhatt, KhataNusus> {
    let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
    let fajwa = i32::from(qiyasat.suud).saturating_sub(i32::from(mawdi.izaha_a));
    let ala = i32::from(mawdi.a).checked_sub(fajwa).ok_or_else(|| {
        marfud(format!("{miftah} has a cell top that does not fit a signed offset"))
    })?;
    if ala < 0 {
        return Err(marfud(format!(
            "{miftah} sits {} row(s) from the top of atlas page {} and its line cell needs \
             {fajwa}; the page was not packed in cells of the font's line height",
            mawdi.a, mawdi.safha
        )));
    }
    let a = u16::try_from(ala)
        .map_err(|_| marfud(format!("{miftah} has a cell top outside a page coordinate")))?;
    let asfal = ala.saturating_add(i32::from(qiyasat.irtifa));
    if asfal > i32::from(irtifa_safha) {
        return Err(marfud(format!(
            "{miftah} needs a cell running to row {asfal} of a page {irtifa_safha} rows tall"
        )));
    }
    let taqaddum = sahih_min_ashri(mawdi.taqaddum)
        .and_then(|qeema| i16::try_from(qeema).ok())
        .ok_or_else(|| {
            marfud(format!("{miftah} has an advance that does not fit a sixteen-bit field"))
        })?;
    Ok(ShaklKhatt {
        harf,
        mustatil: MustatilSafha { s: mawdi.s, a, ard: mawdi.ard, irtifa: qiyasat.irtifa },
        izaha: mawdi.izaha_s,
        taqaddum,
        azwaj: Vec::new(),
    })
}

/// Attaches the per-pair advance corrections to the records they belong to.
///
/// A pair whose first slot is not in the table is dropped rather than refused:
/// the transport's pair list is derived from the lines it laid out, and a line
/// that used a font this call is not rebuilding contributes pairs that are
/// simply not this table's business. The lists are sorted so that two builds of
/// one patch produce identical bytes whatever order the caller supplied.
fn adkhil_azwaj(ashkal: &mut [ShaklKhatt], azwaj: &[(char, char, i16)]) {
    for (awwal, thani, tashih) in azwaj.iter().copied() {
        let (Ok(min), Ok(ila)) =
            (u16::try_from(u32::from(awwal)), u16::try_from(u32::from(thani)))
        else {
            continue;
        };
        if let Some(shakl) = ashkal.iter_mut().find(|shakl| shakl.harf == min)
            && shakl.azwaj.len() < usize::from(AQSA_AZWAJ_SHAKL)
        {
            shakl.azwaj.push(ZawjTaqaddumShakl { akhar: ila, tashih });
        }
    }
    for shakl in ashkal.iter_mut() {
        shakl.azwaj.sort_unstable();
        shakl.azwaj.dedup_by_key(|zawj| zawj.akhar);
    }
}

/// A pointer whose value is only known once the appended region has an address.
///
/// The appended region's position depends on how much the pool and the texture
/// chunk grew, and those are not final until the plan is. So a pointer into the
/// region is planned as "the region's base plus this many bytes" and resolved in
/// one pass at output, rather than guessed at and corrected — a correction pass
/// over pointers is the shape of bug this whole module is built to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarjaMuajjal {
    /// Where the four-byte field is. Absolute in the original container for an
    /// external site, or an offset within the appended region for an internal
    /// one.
    mawqi: u64,
    /// How far into the appended region the target sits.
    dakhili: u64,
    /// Whether `mawqi` is inside the appended region.
    juwwani: bool,
    /// What the pointer is for.
    sabab: &'static str,
}

impl MuhawwilGameMaker {
    /// Plans the twenty-two byte overwrite that repoints a font's `TPAG` item.
    fn sajjil_tahreer_band(
        &mut self,
        band: BandTpag,
        safha: u16,
        mustatil: MustatilSafha,
    ) -> Result<(), KhataNusus> {
        let khaam = bayt_band(safha, mustatil);
        self.sajjil_tahreer(band.mawqi, khaam, "a font's texture page item repointed")
    }

    /// Appends a rebuilt font entry and plans the pointer that will name it.
    ///
    /// The entry's fixed fields are copied byte for byte from the original, so
    /// every field this build has never heard of — the ascender, the SDF spread,
    /// the line height the later releases added — survives unexamined. Only the
    /// glyph count, the glyph pointer array and the glyph records are new.
    fn alhiq_madkhal_khatt(
        &mut self,
        khatt: &KhattGameMaker,
        ashkal: &[ShaklKhatt],
    ) -> Result<(), KhataNusus> {
        let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
        let ras = hajm_usize(khatt.mawqi)
            .ok_or_else(|| marfud("a font entry outside this container".to_owned()))?;
        let hadd = hajm_usize(khatt.mawqi_adad_ashkal)
            .ok_or_else(|| marfud("a font glyph count outside this container".to_owned()))?;
        let thabit = self
            .hawiya
            .bayt
            .get(ras..hadd)
            .ok_or_else(|| marfud("a font entry running past this container".to_owned()))?
            .to_vec();

        let asas = tul_u64(self.mulhaq_akhir.len());
        self.mulhaq_akhir.extend_from_slice(&thabit);
        let adad = u32::try_from(ashkal.len())
            .map_err(|_| marfud("more glyphs than a glyph count can name".to_owned()))?;
        self.mulhaq_akhir.extend_from_slice(&adad.to_le_bytes());

        // The pointer array, written as zeros and planned as deferred pointers;
        // the records follow it immediately, which is the adjacency the reader's
        // own glyph-count search relies on.
        let bidayat_masfufa = tul_u64(self.mulhaq_akhir.len());
        self.mulhaq_akhir.resize(
            self.mulhaq_akhir.len().saturating_add(ashkal.len().saturating_mul(4)),
            0,
        );
        let mut mawdi = tul_u64(self.mulhaq_akhir.len());
        for (fahras, shakl) in ashkal.iter().enumerate() {
            self.maraji_muajjala.push(MarjaMuajjal {
                mawqi: bidayat_masfufa.saturating_add(tul_u64(fahras).saturating_mul(4)),
                dakhili: mawdi,
                juwwani: true,
                sabab: "a generated glyph record",
            });
            let khaam = shakl.bayt();
            mawdi = mawdi.saturating_add(tul_u64(khaam.len()));
            self.mulhaq_akhir.extend_from_slice(&khaam);
        }

        self.maraji_muajjala.push(MarjaMuajjal {
            mawqi: khatt.mawqi_mu_ashir,
            dakhili: asas,
            juwwani: false,
            sabab: "a rebuilt font entry",
        });
        Ok(())
    }
}

/// Where the new texture-page bytes go and what shape a new entry takes.
///
/// Computed from the container rather than from a version number; see
/// [`SafhaTxtr`] for why the entry's layout is copied instead of constructed.
#[derive(Debug, Clone)]
struct KhutatTxtr {
    /// The chunk itself.
    qita: QitaHawiya,
    /// One past the last pointer slot in the array.
    nihayat_masfufa: u64,
    /// Where the entries end, which is where the first blob begins.
    nihayat_madakhil: u64,
    /// A copy of the last existing entry, to be adapted per new page.
    qalib: Vec<u8>,
    /// Offset within the copy of the field holding the blob pointer.
    izahat_mu_ashir: u64,
    /// Offset within the copy of a field that declares the blob's length, when
    /// the entry carries one.
    izahat_tul: Option<u64>,
}

impl MuhawwilGameMaker {
    /// Works out how to describe an appended texture page.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::JadwalAshkalMarfud`] when there is no `TXTR` chunk, no
    /// entry to copy, or the copy holds more than one field that could be a
    /// declared blob length — in which case updating the right one would be a
    /// coin flip, and a page whose declared length is wrong is a page the
    /// runtime uploads the wrong number of bytes from.
    fn khutat_txtr(&self) -> Result<KhutatTxtr, KhataNusus> {
        let marfud = |sabab: String| KhataNusus::JadwalAshkalMarfud { sabab };
        let qita = qita_bi_ism(&self.hawiya.jadwal, ISM_TXTR)
            .ok_or_else(|| marfud("this container has no TXTR chunk".to_owned()))?;
        let akhir = self
            .hawiya
            .safahat
            .last()
            .copied()
            .ok_or_else(|| marfud("this container has no texture page to copy".to_owned()))?;
        let adad = tul_u64(self.hawiya.safahat.len());
        let nihayat_masfufa = qita.izaha.saturating_add(4).saturating_add(adad.saturating_mul(4));
        let nihayat_madakhil = self
            .hawiya
            .safahat
            .iter()
            .map(|safha| safha.izaha)
            .min()
            .unwrap_or_else(|| qita.nihaya());

        // The copied entry runs from the last entry's start to whichever comes
        // first: the blob region, or the end of the chunk.
        let bidaya = hajm_usize(akhir.mawqi_madkhal)
            .ok_or_else(|| marfud("a texture entry outside this container".to_owned()))?;
        let hadd = hajm_usize(nihayat_madakhil.max(akhir.mawqi_madkhal))
            .ok_or_else(|| marfud("a texture entry outside this container".to_owned()))?;
        let qalib = self
            .hawiya
            .bayt
            .get(bidaya..hadd)
            .ok_or_else(|| marfud("a texture entry running past this container".to_owned()))?
            .to_vec();
        let izahat_mu_ashir = akhir.mawqi_mu_ashir.checked_sub(akhir.mawqi_madkhal).ok_or_else(
            || marfud("a texture entry whose blob pointer sits before it".to_owned()),
        )?;

        // A field declaring the source blob's length has to be updated with it.
        // Two candidates means guessing which, and a page whose declared length
        // is wrong is a page the runtime uploads the wrong bytes from.
        let mut izahat_tul: Option<u64> = None;
        let mut mawdi = 0_usize;
        while mawdi.saturating_add(4) <= qalib.len() {
            let raqm = u64::from(iqra_u32(&qalib, mawdi).unwrap_or_default());
            if tul_u64(mawdi) != izahat_mu_ashir && raqm == akhir.tul && raqm != 0 {
                if izahat_tul.is_some() {
                    return Err(marfud(
                        "this container's texture entries hold more than one field equal to \
                         the blob's length, so which one declares it cannot be told apart"
                            .to_owned(),
                    ));
                }
                izahat_tul = Some(tul_u64(mawdi));
            }
            mawdi = mawdi.saturating_add(4);
        }

        Ok(KhutatTxtr {
            qita,
            nihayat_masfufa,
            nihayat_madakhil,
            qalib,
            izahat_mu_ashir,
            izahat_tul,
        })
    }

    /// Builds the insertion list the plan implies.
    fn idrajat(&self) -> Result<Vec<IdrajBayt>, KhataNusus> {
        let mut idrajat: Vec<IdrajBayt> = Vec::new();
        if !self.hawd_mulhaq.is_empty() {
            idrajat.push(IdrajBayt {
                mawqi: self.hawiya.hawd.qita().nihaya(),
                bayt: hashw_muhadhah(self.hawd_mulhaq.clone()),
                qita: ISM_STRG,
                naw: NawIdraj::Hawd,
            });
        }
        if !self.safahat_mulhaqa.is_empty() {
            let khutat = self.khutat_txtr()?;
            let adad = tul_u64(self.safahat_mulhaqa.len());
            idrajat.push(IdrajBayt {
                mawqi: khutat.nihayat_masfufa,
                bayt: hashw_muhadhah(vec![0_u8; usize::try_from(adad.saturating_mul(4))
                    .unwrap_or_default()]),
                qita: ISM_TXTR,
                naw: NawIdraj::MasfufatSafahat,
            });
            let mut madakhil: Vec<u8> = Vec::new();
            for _ in 0..self.safahat_mulhaqa.len() {
                madakhil.extend_from_slice(&khutat.qalib);
            }
            idrajat.push(IdrajBayt {
                mawqi: khutat.nihayat_madakhil,
                bayt: hashw_muhadhah(madakhil),
                qita: ISM_TXTR,
                naw: NawIdraj::MadakhilSafahat,
            });
            let mut kutal: Vec<u8> = Vec::new();
            for kutla in &self.safahat_mulhaqa {
                kutal.extend_from_slice(&hashw_muhadhah(kutla.clone()));
            }
            idrajat.push(IdrajBayt {
                mawqi: khutat.qita.nihaya(),
                bayt: kutal,
                qita: ISM_TXTR,
                naw: NawIdraj::KutalSafahat,
            });
        }
        if !self.mulhaq_akhir.is_empty() {
            let akhira = self.hawiya.qita_akhira().ok_or(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "a container with no chunks to append behind",
                qeema: 0,
                hadd: 0,
            })?;
            let mut kutla: Vec<u8> = Vec::new();
            if matches!(self.mawdi_ilhaq, MawdiIlhaq::QitaJadida) {
                kutla.extend_from_slice(&ISM_QITA_ILHAQ);
                let tul = u32::try_from(hashw_muhadhah(self.mulhaq_akhir.clone()).len())
                    .map_err(|_| KhataNusus::HajmMufrit {
                        haql: "the appended region",
                        qeema: tul_u64(self.mulhaq_akhir.len()),
                        saqf: AQSA_HAJM_HAWIYA,
                    })?;
                kutla.extend_from_slice(&tul.to_le_bytes());
            }
            kutla.extend_from_slice(&hashw_muhadhah(self.mulhaq_akhir.clone()));
            // A new chunk grows the FORM payload and no existing chunk, so the
            // growth is attributed to a name the chunk table does not hold and
            // the length fixer leaves every real chunk header alone.
            let qita = match self.mawdi_ilhaq {
                MawdiIlhaq::TadhyeelAkhir => akhira.ism,
                MawdiIlhaq::QitaJadida => ISM_QITA_ILHAQ,
            };
            idrajat.push(IdrajBayt {
                mawqi: akhira.nihaya(),
                bayt: kutla,
                qita,
                naw: NawIdraj::Ilhaq,
            });
        }
        Ok(idrajat)
    }
}

impl MuhawwilGameMaker {
    /// Produces the patched container.
    ///
    /// One pass, in a fixed order, and nothing is written into a game's
    /// directory by this function — it returns bytes, which is what makes it
    /// usable for diffing and diagnostics as well as for installing. The
    /// supported way to install is [`MuhawwilGameMaker::uktub`], which runs this
    /// and then hands the result to the reversibility guard. The order is:
    ///
    /// 1. work out the insertions the plan implies and build the offset map;
    /// 2. check the relocation precondition, before a byte is produced;
    /// 3. splice the insertions into a copy of the original;
    /// 4. apply the in-place overwrites, each guarded by the bytes it expects;
    /// 5. rewrite every reference the census classified;
    /// 6. wire the appended texture pages and the appended region's pointers;
    /// 7. fix the chunk length headers and the `FORM` length;
    /// 8. verify that every untouched region is byte-identical.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the output would exceed
    /// [`AQSA_HAJM_HAWIYA`], [`KhataNusus::HawiyaTalifa`] when an edit or a
    /// reference does not fit the buffer, [`KhataNusus::JadwalAshkalMarfud`]
    /// from the texture planning, and [`KhataNusus::DawraGhayrMutabaqa`] when
    /// the relocation precondition does not hold or the verification pass finds
    /// a byte that changed and should not have.
    pub fn ukhruj(&self) -> Result<Vec<u8>, KhataNusus> {
        let idrajat = self.idrajat()?;
        let khareeta = KhareetatIzaha::jadeed(&idrajat);
        self.tahaqquq_shart(&khareeta)?;
        let mut mukhraj = self.ilsaq(&idrajat)?;
        self.atbiq_tahrirat(&mut mukhraj, &khareeta)?;
        self.asleh_maraji(&mut mukhraj, &khareeta)?;
        self.asleh_txtr(&mut mukhraj, &khareeta)?;
        self.asleh_muajjala(&mut mukhraj, &khareeta)?;
        self.asleh_atwal(&mut mukhraj, &idrajat, &khareeta)?;
        self.tahaqquq_dawra(&mukhraj)?;
        Ok(mukhraj)
    }

    /// Produces the patched container and writes it through the guard.
    ///
    /// [`MuhawwilGameMaker::ukhruj`] deliberately returns bytes and writes
    /// nowhere, which is right for a function that also serves diffing and
    /// diagnostics — but leaving the write to the caller is how a crate ends up
    /// with a sixth atomic-write implementation and a `data.win` modified with
    /// no recorded original. This is the one supported way to install: the
    /// container is produced and fully verified in memory first, and only then
    /// is [`Hafiz::iktub`] given the bytes, which preserves the game's own
    /// `data.win` and its fingerprint before overwriting it.
    ///
    /// A `data.win` *is* the game — code, strings, textures and room layouts in
    /// one file — so there is no partial recovery from a bad write. Either the
    /// original comes back byte for byte or the player redownloads the game.
    ///
    /// # Errors
    ///
    /// Everything [`MuhawwilGameMaker::ukhruj`] refuses, plus
    /// [`KhataNusus::NuskhaMafquda`] when the original cannot be preserved and
    /// [`KhataNusus::KhataMalaf`] when the write itself fails.
    pub fn uktub(&self, hafiz: &mut dyn Hafiz, hadaf: &Path) -> Result<(), KhataNusus> {
        let mukhraj = self.ukhruj()?;
        hafiz.iktub(hadaf, &mukhraj)
    }

    /// Checks that nothing moves that this build cannot account for.
    ///
    /// Two conditions, and both have to hold:
    ///
    /// * every chunk with a byte at or after the frontier is one whose absolute
    ///   pointers the census enumerated exhaustively — not merely one whose name
    ///   appears in [`qita_qabila_lilizaha`], because a chunk that *could* be
    ///   enumerated and was not is a chunk full of unrewritten pointers;
    /// * no unclassified pointer points at or past the frontier, because such a
    ///   field would be left naming bytes that have moved.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::DawraGhayrMutabaqa`] naming how many chunks or fields could
    /// not be accounted for. That variant rather than a new one on purpose: the
    /// honest statement is that this build cannot reproduce the container
    /// faithfully, which is the same statement the verification pass makes.
    fn tahaqquq_shart(&self, khareeta: &KhareetatIzaha) -> Result<(), KhataNusus> {
        let Some(hadd) = khareeta.hadd_haraka() else {
            return Ok(());
        };
        let mut majhula: u64 = 0;
        for qita in &self.hawiya.jadwal {
            if qita.nihaya() <= hadd {
                continue;
            }
            if !qita_qabila_lilizaha(qita.ism) || !self.hawiya.maraji.mafhuma(qita.ism) {
                majhula = majhula.saturating_add(1);
            }
        }
        let mutalliqa = self
            .hawiya
            .maraji
            .maraji_baad(hadd)
            .filter(|marja| !marja.naw.maaruf())
            .count();
        let kull = majhula.saturating_add(tul_u64(mutalliqa));
        if kull > 0 {
            return Err(KhataNusus::DawraGhayrMutabaqa { sigha: SIGHA, adad: kull });
        }
        Ok(())
    }

    /// Splices the insertions into a copy of the original.
    fn ilsaq(&self, idrajat: &[IdrajBayt]) -> Result<Vec<u8>, KhataNusus> {
        let asl = &self.hawiya.bayt;
        let zaid = idrajat.iter().fold(0_u64, |kull, idraj| {
            kull.saturating_add(tul_u64(idraj.bayt.len()))
        });
        let hajm = tul_u64(asl.len()).saturating_add(zaid);
        if hajm > AQSA_HAJM_HAWIYA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the patched container's length",
                qeema: hajm,
                saqf: AQSA_HAJM_HAWIYA,
            });
        }
        let mut murattaba: Vec<&IdrajBayt> = idrajat.iter().collect();
        murattaba.sort_by_key(|idraj| idraj.mawqi);

        let mut mukhraj: Vec<u8> = Vec::with_capacity(hajm_usize(hajm).unwrap_or(asl.len()));
        let mut mawqi = 0_usize;
        for idraj in murattaba {
            let hadd = hajm_usize(idraj.mawqi).unwrap_or(asl.len()).min(asl.len()).max(mawqi);
            let juz = asl.get(mawqi..hadd).ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "an insertion point outside the container",
                qeema: idraj.mawqi,
                hadd: tul_u64(asl.len()),
            })?;
            mukhraj.extend_from_slice(juz);
            mukhraj.extend_from_slice(&idraj.bayt);
            mawqi = hadd;
        }
        mukhraj.extend_from_slice(asl.get(mawqi..).unwrap_or_default());
        Ok(mukhraj)
    }

    /// Applies the in-place overwrites, each guarded by the bytes it expects.
    fn atbiq_tahrirat(
        &self,
        mukhraj: &mut [u8],
        khareeta: &KhareetatIzaha,
    ) -> Result<(), KhataNusus> {
        for tahreer in &self.tahrirat {
            let ras = hajm_usize(tahreer.mawqi).ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "an edit offset",
                qeema: tahreer.mawqi,
                hadd: tul_u64(self.hawiya.bayt.len()),
            })?;
            let hali = self
                .hawiya
                .bayt
                .get(ras..ras.saturating_add(tahreer.qadeem.len()))
                .unwrap_or_default();
            if hali != tahreer.qadeem.as_slice() {
                // The plan was built against a different parse of this file. One
                // comparison stands between that and a pointer written into the
                // middle of a sprite.
                return Err(KhataNusus::DawraGhayrMutabaqa {
                    sigha: SIGHA,
                    adad: tul_u64(tahreer.qadeem.len()),
                });
            }
            let jadeed = khareeta.izaha_jadida(tahreer.mawqi);
            let bidaya = hajm_usize(jadeed).ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "a mapped edit offset",
                qeema: jadeed,
                hadd: tul_u64(mukhraj.len()),
            })?;
            // Read before the mutable borrow: `ok_or`'s argument is evaluated
            // eagerly, so the length read would overlap with `get_mut`.
            let tul_mukhraj = tul_u64(mukhraj.len());
            let khana = mukhraj
                .get_mut(bidaya..bidaya.saturating_add(tahreer.jadeed.len()))
                .ok_or(KhataNusus::HawiyaTalifa {
                    sigha: SIGHA,
                    haql: "an edit running past the patched container",
                    qeema: jadeed,
                    hadd: tul_mukhraj,
                })?;
            khana.copy_from_slice(&tahreer.jadeed);
        }
        Ok(())
    }
}

/// Where each inserted block begins in the output.
///
/// The blocks are placed in ascending offset, ties broken by the order they were
/// planned in — which is the order [`MuhawwilGameMaker::ilsaq`] writes them, and
/// therefore the order that has to be used to resolve a pointer into one of them.
fn mawadi_idrajat(idrajat: &[IdrajBayt]) -> BTreeMap<NawIdraj, u64> {
    let mut tartib: Vec<usize> = (0..idrajat.len()).collect();
    tartib.sort_by_key(|fahras| idrajat.get(*fahras).map_or(u64::MAX, |idraj| idraj.mawqi));
    let mut mawadi: BTreeMap<NawIdraj, u64> = BTreeMap::new();
    let mut zaid = 0_u64;
    for fahras in tartib {
        let Some(idraj) = idrajat.get(fahras) else { continue };
        let _ = mawadi.insert(idraj.naw, idraj.mawqi.saturating_add(zaid));
        zaid = zaid.saturating_add(tul_u64(idraj.bayt.len()));
    }
    mawadi
}

impl MuhawwilGameMaker {
    /// Rewrites every reference the census classified.
    ///
    /// A string whose record was appended gets its new address; everything else
    /// gets its old target's new position. Both are written even when they did
    /// not change, because a field written with the value it already held is a
    /// no-op and a field skipped by a stale condition is a dangling pointer.
    fn asleh_maraji(
        &self,
        mukhraj: &mut [u8],
        khareeta: &KhareetatIzaha,
    ) -> Result<(), KhataNusus> {
        let idrajat = self.idrajat()?;
        let bidayat_hawd =
            mawadi_idrajat(&idrajat).get(&NawIdraj::Hawd).copied().unwrap_or_default();
        for marja in self.hawiya.maraji.maraji() {
            if !marja.naw.maaruf() {
                continue;
            }
            let hadaf = match marja.naw {
                NawMarja::Nass(fahras) => match self.mawadi_nusus.get(&fahras) {
                    Some(dakhili) => bidayat_hawd.saturating_add(*dakhili),
                    None => khareeta.izaha_jadida(marja.hadaf),
                },
                _ => khareeta.izaha_jadida(marja.hadaf),
            };
            let mawqi = khareeta.izaha_jadida(marja.mawqi);
            uktub_marja(mukhraj, mawqi, hadaf)?;
        }
        Ok(())
    }

    /// Wires the appended texture pages: the count, the new pointer slots, the
    /// new entries and their blob pointers.
    fn asleh_txtr(
        &self,
        mukhraj: &mut [u8],
        khareeta: &KhareetatIzaha,
    ) -> Result<(), KhataNusus> {
        if self.safahat_mulhaqa.is_empty() {
            return Ok(());
        }
        let khutat = self.khutat_txtr()?;
        let idrajat = self.idrajat()?;
        let mawadi = mawadi_idrajat(&idrajat);
        let talif = |haql: &'static str, qeema: u64| KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql,
            qeema,
            hadd: tul_u64(mukhraj.len()),
        };
        let bidayat_masfufa = mawadi
            .get(&NawIdraj::MasfufatSafahat)
            .copied()
            .ok_or_else(|| talif("the appended TXTR pointer slots", 0))?;
        let bidayat_madakhil = mawadi
            .get(&NawIdraj::MadakhilSafahat)
            .copied()
            .ok_or_else(|| talif("the appended TXTR entries", 0))?;
        let bidayat_kutal = mawadi
            .get(&NawIdraj::KutalSafahat)
            .copied()
            .ok_or_else(|| talif("the appended TXTR blobs", 0))?;

        let mawjud = tul_u64(self.hawiya.safahat.len());
        let jadeed = mawjud.saturating_add(tul_u64(self.safahat_mulhaqa.len()));
        let mawqi_adad = khareeta.izaha_jadida(khutat.qita.izaha);
        uktub_marja(mukhraj, mawqi_adad, jadeed)?;

        let ittisa = tul_u64(khutat.qalib.len());
        let mut mawdi_kutla = bidayat_kutal;
        for (fahras, kutla) in self.safahat_mulhaqa.iter().enumerate() {
            let raqm = tul_u64(fahras);
            let madkhal = bidayat_madakhil.saturating_add(raqm.saturating_mul(ittisa));
            uktub_marja(
                mukhraj,
                bidayat_masfufa.saturating_add(raqm.saturating_mul(4)),
                madkhal,
            )?;
            uktub_marja(mukhraj, madkhal.saturating_add(khutat.izahat_mu_ashir), mawdi_kutla)?;
            if let Some(izahat_tul) = khutat.izahat_tul {
                uktub_marja(
                    mukhraj,
                    madkhal.saturating_add(izahat_tul),
                    tul_u64(kutla.len()),
                )?;
            }
            mawdi_kutla = mawdi_kutla
                .saturating_add(tul_u64(hashw_muhadhah(kutla.clone()).len()));
        }
        Ok(())
    }

    /// Resolves every pointer into the appended region.
    fn asleh_muajjala(
        &self,
        mukhraj: &mut [u8],
        khareeta: &KhareetatIzaha,
    ) -> Result<(), KhataNusus> {
        if self.maraji_muajjala.is_empty() {
            return Ok(());
        }
        let idrajat = self.idrajat()?;
        let mawadi = mawadi_idrajat(&idrajat);
        let bidaya =
            mawadi.get(&NawIdraj::Ilhaq).copied().ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "the appended region",
                qeema: 0,
                hadd: tul_u64(mukhraj.len()),
            })?;
        let asas = match self.mawdi_ilhaq {
            MawdiIlhaq::TadhyeelAkhir => bidaya,
            MawdiIlhaq::QitaJadida => bidaya.saturating_add(HAJM_TARWISAT_QITA),
        };
        for marja in &self.maraji_muajjala {
            let mawqi = if marja.juwwani {
                asas.saturating_add(marja.mawqi)
            } else {
                khareeta.izaha_jadida(marja.mawqi)
            };
            uktub_marja(mukhraj, mawqi, asas.saturating_add(marja.dakhili)).map_err(
                |khata| match khata {
                    // The generic message says a reference site is out of range;
                    // this says which reference, which is what a person reading
                    // the refusal actually needs.
                    KhataNusus::HawiyaTalifa { sigha, qeema, hadd, .. } => {
                        KhataNusus::HawiyaTalifa { sigha, haql: marja.sabab, qeema, hadd }
                    }
                    akhar => akhar,
                },
            )?;
        }
        Ok(())
    }
}

/// Writes one absolute offset into the output, or refuses.
fn uktub_marja(mukhraj: &mut [u8], mawqi: u64, hadaf: u64) -> Result<(), KhataNusus> {
    let qeema = izaha_u32(hadaf)?;
    let ras = hajm_usize(mawqi).ok_or_else(|| KhataNusus::HawiyaTalifa {
        sigha: SIGHA,
        haql: "a reference site outside the patched container",
        qeema: mawqi,
        hadd: tul_u64(mukhraj.len()),
    })?;
    if uktub_u32(mukhraj, ras, qeema) {
        Ok(())
    } else {
        Err(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "a reference site running past the patched container",
            qeema: mawqi,
            hadd: tul_u64(mukhraj.len()),
        })
    }
}

impl MuhawwilGameMaker {
    /// Fixes the length header of every chunk that grew, and the `FORM` length.
    fn asleh_atwal(
        &self,
        mukhraj: &mut [u8],
        idrajat: &[IdrajBayt],
        khareeta: &KhareetatIzaha,
    ) -> Result<(), KhataNusus> {
        for qita in &self.hawiya.jadwal {
            let zaid = idrajat
                .iter()
                .filter(|idraj| idraj.qita == qita.ism)
                .fold(0_u64, |kull, idraj| kull.saturating_add(tul_u64(idraj.bayt.len())));
            if zaid == 0 {
                continue;
            }
            let mawqi = khareeta.izaha_jadida(qita.tarwisa).saturating_add(4);
            let ras = hajm_usize(mawqi).ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "a chunk length header",
                qeema: mawqi,
                hadd: tul_u64(mukhraj.len()),
            })?;
            let tul = izaha_u32(qita.tul.saturating_add(zaid))?;
            if !uktub_u32(mukhraj, ras, tul) {
                return Err(KhataNusus::HawiyaTalifa {
                    sigha: SIGHA,
                    haql: "a chunk length header past the patched container",
                    qeema: mawqi,
                    hadd: tul_u64(mukhraj.len()),
                });
            }
        }
        let hajm = izaha_u32(tul_u64(mukhraj.len()).saturating_sub(HAJM_TARWISAT_QITA))?;
        if !uktub_u32(mukhraj, 4, hajm) {
            return Err(KhataNusus::MalafQaseer {
                haql: "the FORM header of the patched container",
                tul: tul_u64(mukhraj.len()),
                matlub: HAJM_TARWISAT_QITA,
            });
        }
        Ok(())
    }

    /// Every region of the original this plan is allowed to have changed, in the
    /// original container's coordinates.
    ///
    /// Recomputed from the plan rather than collected while writing. The two
    /// would almost always agree, and the one time they did not would be a
    /// writer that touched a byte it had not recorded — which is precisely the
    /// failure the verification exists to catch, and which a self-reported list
    /// would hide.
    fn manatiq_masmuha(&self) -> Vec<(u64, u64)> {
        let mut manatiq: Vec<(u64, u64)> = vec![(4, 8)];
        for tahreer in &self.tahrirat {
            manatiq.push((
                tahreer.mawqi,
                tahreer.mawqi.saturating_add(tul_u64(tahreer.qadeem.len())),
            ));
        }
        for marja in self.hawiya.maraji.maraji() {
            if marja.naw.maaruf() {
                manatiq.push((marja.mawqi, marja.mawqi.saturating_add(4)));
            }
        }
        for marja in &self.maraji_muajjala {
            if !marja.juwwani {
                manatiq.push((marja.mawqi, marja.mawqi.saturating_add(4)));
            }
        }
        for qita in &self.hawiya.jadwal {
            manatiq.push((qita.tarwisa.saturating_add(4), qita.tarwisa.saturating_add(8)));
            if qita.ism == ISM_TXTR && !self.safahat_mulhaqa.is_empty() {
                manatiq.push((qita.izaha, qita.izaha.saturating_add(4)));
            }
        }
        manatiq.sort_unstable();
        let mut madmuma: Vec<(u64, u64)> = Vec::with_capacity(manatiq.len());
        for (bidaya, nihaya) in manatiq {
            if madmuma.last().is_some_and(|akhir| bidaya <= akhir.1) {
                if let Some(akhir) = madmuma.last_mut() {
                    akhir.1 = akhir.1.max(nihaya);
                }
            } else {
                madmuma.push((bidaya, nihaya));
            }
        }
        madmuma
    }

    /// Proves that every byte nobody named is the byte that was there before.
    ///
    /// The check this whole module is arranged around. The output is the
    /// original with insertions spliced in and a listed set of fields
    /// overwritten, so every original byte outside that set must appear
    /// unchanged at the position the offset map says it moved to. Anything else
    /// means the writer touched something it did not record, and a container
    /// that has been changed in a way nobody can name is a container that does
    /// not reach a player's game directory.
    ///
    /// Recomputes the plan rather than trusting the write pass's own account of
    /// what it did: the set of regions a plan is allowed to have changed is
    /// derived from the plan itself, so a writer that touched a byte it never
    /// recorded is caught rather than believed.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::DawraGhayrMutabaqa`] with the number of differing bytes,
    /// and [`KhataNusus::HajmMufrit`] when the produced length is not the length
    /// the plan implies.
    pub fn tahaqquq_dawra(&self, mukhraj: &[u8]) -> Result<(), KhataNusus> {
        let idrajat = self.idrajat()?;
        let khareeta = KhareetatIzaha::jadeed(&idrajat);
        let asl = &self.hawiya.bayt;
        let mutawaqqa = tul_u64(asl.len()).saturating_add(khareeta.majmu());
        if tul_u64(mukhraj.len()) != mutawaqqa {
            return Err(KhataNusus::HajmMufrit {
                haql: "the patched container's length against the plan",
                qeema: tul_u64(mukhraj.len()),
                saqf: mutawaqqa,
            });
        }

        let manatiq = self.manatiq_masmuha();
        let mut hudud: BTreeSet<u64> = BTreeSet::new();
        let _ = hudud.insert(0);
        let _ = hudud.insert(tul_u64(asl.len()));
        for idraj in &idrajat {
            let _ = hudud.insert(idraj.mawqi.min(tul_u64(asl.len())));
        }
        for (bidaya, nihaya) in &manatiq {
            let _ = hudud.insert((*bidaya).min(tul_u64(asl.len())));
            let _ = hudud.insert((*nihaya).min(tul_u64(asl.len())));
        }

        let mut mukhtalif: u64 = 0;
        let nuqat: Vec<u64> = hudud.into_iter().collect();
        for zawj in nuqat.windows(2) {
            let (Some(bidaya), Some(nihaya)) = (zawj.first().copied(), zawj.get(1).copied())
            else {
                continue;
            };
            if nihaya <= bidaya {
                continue;
            }
            if manatiq.iter().any(|(min, ila)| *min <= bidaya && bidaya < *ila) {
                continue;
            }
            mukhtalif = mukhtalif.saturating_add(farq_qita(
                asl,
                mukhraj,
                bidaya,
                nihaya,
                khareeta.izaha_jadida(bidaya),
            ));
        }
        if mukhtalif > 0 {
            return Err(KhataNusus::DawraGhayrMutabaqa { sigha: SIGHA, adad: mukhtalif });
        }
        Ok(())
    }
}

/// How many bytes of one original span differ from where they should have moved.
fn farq_qita(asl: &[u8], mukhraj: &[u8], bidaya: u64, nihaya: u64, jadeed: u64) -> u64 {
    let (Some(min), Some(ila), Some(hadaf)) =
        (hajm_usize(bidaya), hajm_usize(nihaya), hajm_usize(jadeed))
    else {
        return nihaya.saturating_sub(bidaya);
    };
    let Some(qadeem) = asl.get(min..ila) else {
        return nihaya.saturating_sub(bidaya);
    };
    let Some(hali) = mukhraj.get(hadaf..hadaf.saturating_add(qadeem.len())) else {
        return tul_u64(qadeem.len());
    };
    if qadeem == hali {
        return 0;
    }
    tul_u64(qadeem.iter().zip(hali.iter()).filter(|(awwal, thani)| awwal != thani).count())
}

// ---------------------------------------------------------------------------
// The transport seam
// ---------------------------------------------------------------------------

/// Records the glyphs one shaped line needs, so the pool can be frozen.
///
/// The only thing this function can key on is a [`Harf`], which carries a font
/// index and a glyph identifier and — deliberately, see its own documentation —
/// no codepoint. There is therefore no way to write a version of this that
/// registers a slot *for a character*, which is the transport's first invariant
/// holding at this boundary rather than being restated at it.
pub fn sajjil_ashkal(hawd: &mut HawdKhanat, hajm_rubi: u16, huruf: &[Harf]) {
    for harf in huruf {
        hawd.sajjil(MiftahKhana::min_harf(harf, hajm_rubi));
    }
}

/// Emits one shaped line as the sequence of slots that draws it.
///
/// **The result is not text.** It is a sequence of addresses into the glyph
/// table generated in the same build, in visual order, and it is meaningful to
/// nothing else. It is not searchable, not selectable, not copyable and not
/// readable by any assistive technology, and it cannot be re-wrapped,
/// re-shaped or concatenated with another one and stay correct. The `String`
/// return type is what the container's string pool is filled from and is not a
/// claim about what the value is.
///
/// The glyphs arrive from `saff` already in visual order, and the order is not
/// touched here: reordering them would be this module deciding something the
/// layout already decided.
///
/// # Errors
///
/// [`KhataNusus::JadwalAshkalMarfud`] naming the glyph when the assignment does
/// not hold one of the line's glyphs, which means the pool was frozen before
/// this line was registered — the one ordering mistake this seam can make, and
/// the one it refuses rather than silently dropping a letter over.
pub fn nass_manqul(
    tawzee: &TawzeeKhanat,
    hajm_rubi: u16,
    huruf: &[Harf],
) -> Result<String, KhataNusus> {
    let mut khanat = String::with_capacity(huruf.len());
    for harf in huruf {
        let miftah = MiftahKhana::min_harf(harf, hajm_rubi);
        let khana = tawzee.khana(miftah).ok_or_else(|| KhataNusus::JadwalAshkalMarfud {
            sabab: format!(
                "the frozen slot assignment does not hold {miftah}, so this line was laid \
                 out after the glyph pool was frozen"
            ),
        })?;
        khanat.push(khana);
    }
    Ok(khanat)
}

/// Which slot mode a GameMaker container can address.
///
/// The basic-plane one, always. A GameMaker glyph names its character with a
/// sixteen-bit field ([`ShaklKhatt`]), so a slot above U+FFFF cannot appear in a
/// glyph table at all — this is the same sixteen-bit character type the
/// transport's own header warns about, arrived at from the container's side.
///
/// Six thousand four hundred slots is the ceiling that follows, and it is a real
/// one: a patch with four sizes and a full Arabic repertoire can approach it.
/// The transport refuses rather than truncates when it is reached, which is the
/// correct behaviour — a glyph table cut short does not draw fewer letters, it
/// draws the wrong ones.
#[must_use]
pub const fn namat_khanat_gamemaker() -> NamatKhana {
    NamatKhana::Asasi
}

// ---------------------------------------------------------------------------
// draw_text alignment
// ---------------------------------------------------------------------------

/// GameMaker's horizontal alignment constants, as `draw_set_halign` takes them.
///
/// ```text
/// fa_left   0
/// fa_center 1
/// fa_right  2
/// ```
///
/// A patched game needs these flipped wherever the original chose an edge rather
/// than the centre: an interface laid out against the left margin in English
/// belongs against the right margin in Arabic, and a transported line drawn with
/// `fa_left` sits at the wrong end of its box no matter how correctly it was
/// shaped. The centre is its own mirror and is never touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MuhadhahaGameMaker {
    /// `fa_left`.
    Yasar,
    /// `fa_center`.
    Wasat,
    /// `fa_right`.
    Yameen,
}

impl MuhadhahaGameMaker {
    /// The constant's numeric value.
    #[must_use]
    pub const fn qeema(self) -> i16 {
        match self {
            Self::Yasar => 0,
            Self::Wasat => 1,
            Self::Yameen => 2,
        }
    }

    /// The alignment a value names, or [`None`] when it names none of them.
    #[must_use]
    pub const fn min_qeema(qeema: i16) -> Option<Self> {
        match qeema {
            0 => Some(Self::Yasar),
            1 => Some(Self::Wasat),
            2 => Some(Self::Yameen),
            _ => None,
        }
    }

    /// The right-to-left mirror of this alignment.
    #[must_use]
    pub const fn maqluba(self) -> Self {
        match self {
            Self::Yasar => Self::Yameen,
            Self::Wasat => Self::Wasat,
            Self::Yameen => Self::Yasar,
        }
    }

    /// The name a log line uses.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Yasar => "fa_left",
            Self::Wasat => "fa_center",
            Self::Yameen => "fa_right",
        }
    }
}

/// One place an alignment constant is pushed, and what it should become.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MawdiMuhadhaha {
    /// Absolute file offset of the four-byte instruction word.
    pub mawqi: u64,
    /// The alignment currently pushed there.
    pub qadeem: MuhadhahaGameMaker,
    /// The alignment it should push instead.
    pub jadeed: MuhadhahaGameMaker,
}

/// The opcode byte of the instruction that pushes a small signed integer.
///
/// ```text
/// pushi.e — 4 bytes, little-endian
///
///   offset  size  field    meaning
///        0     2  qeema    the immediate, signed
///        2     1  naw      the operand type: 0x0F, the sixteen-bit integer
///        3     1  amaliya  the opcode: 0x84
/// ```
pub const AMALIYAT_DAF: u8 = 0x84;

/// The operand type byte of a sixteen-bit integer immediate.
pub const NAW_SAHIH_QASEER: u8 = 0x0F;

/// The opcode byte of a script or function call.
pub const AMALIYAT_NIDAA: u8 = 0xD9;

/// Finds candidate alignment pushes in the bytecode.
///
/// A candidate is a `pushi.e` of `fa_left` or `fa_right` immediately followed by
/// a call taking one argument — the exact shape `draw_set_halign(fa_left)`
/// compiles to.
///
/// **These are candidates and this function says so in its name and here.** It
/// does not establish that the call is `draw_set_halign`: a bytecode function
/// reference is the head of an occurrence chain rather than an index, walking it
/// means reproducing the linker's own bookkeeping, and this build does not. So
/// the same eight bytes also describe `draw_set_valign(fa_top)` and any other
/// one-argument call preceded by a small constant.
///
/// The safety therefore does not come from the scan. It comes from
/// [`MuhawwilGameMaker::sahhih_muhadhaha`], which writes only the sites it is
/// handed and guards each one against the bytes it expects — so a caller reviews
/// this list, or supplies its own, and nothing is rewritten on the strength of a
/// pattern match alone.
#[must_use]
pub fn mawadi_muhadhaha_murashaha(hawiya: &HawiyatGameMaker) -> Vec<MawdiMuhadhaha> {
    let mut mawadi: Vec<MawdiMuhadhaha> = Vec::new();
    let Some(qita) = qita_bi_ism(hawiya.jadwal(), ISM_CODE) else {
        return mawadi;
    };
    let (Some(bidaya), Some(nihaya)) = (hajm_usize(qita.izaha), hajm_usize(qita.nihaya())) else {
        return mawadi;
    };
    let bayt = hawiya.bayt();
    let mut mawdi = bidaya;
    while mawdi.saturating_add(8) <= nihaya.min(bayt.len()) {
        let daf = bayt.get(mawdi..mawdi.saturating_add(4)).unwrap_or_default();
        let nidaa = bayt.get(mawdi.saturating_add(4)..mawdi.saturating_add(8)).unwrap_or_default();
        let shakl_daf = daf.get(2) == Some(&NAW_SAHIH_QASEER) && daf.get(3) == Some(&AMALIYAT_DAF);
        let shakl_nidaa = nidaa.get(3) == Some(&AMALIYAT_NIDAA)
            && iqra_i16(nidaa, 0) == Some(1);
        if shakl_daf
            && shakl_nidaa
            && let Some(qadeem) = iqra_i16(daf, 0).and_then(MuhadhahaGameMaker::min_qeema)
            && !matches!(qadeem, MuhadhahaGameMaker::Wasat)
        {
            mawadi.push(MawdiMuhadhaha {
                mawqi: tul_u64(mawdi),
                qadeem,
                jadeed: qadeem.maqluba(),
            });
        }
        mawdi = mawdi.saturating_add(4);
    }
    mawadi
}

impl MuhawwilGameMaker {
    /// Plans the alignment corrections for a list of sites.
    ///
    /// Each site is written as a whole four-byte `pushi.e` word rather than as a
    /// two-byte immediate, so the guard covers the opcode and the operand type
    /// as well as the value. A site whose bytes are not the `pushi.e` of the
    /// alignment the plan expects is refused: the plan was built against a
    /// different reading of this file, and writing an immediate into whatever is
    /// actually there would corrupt an instruction rather than change a
    /// constant.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HawiyaTalifa`] when a site is outside the container or does
    /// not hold the instruction the plan expects.
    pub fn sahhih_muhadhaha(&mut self, mawadi: &[MawdiMuhadhaha]) -> Result<u32, KhataNusus> {
        let mut adad: u32 = 0;
        for mawdi in mawadi {
            let ras = hajm_usize(mawdi.mawqi).ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "an alignment site outside the container",
                qeema: mawdi.mawqi,
                hadd: tul_u64(self.hawiya.bayt.len()),
            })?;
            let hali = self.hawiya.bayt.get(ras..ras.saturating_add(4)).unwrap_or_default();
            let mutawaqqa = kalimat_daf(mawdi.qadeem);
            if hali != mutawaqqa.as_slice() {
                return Err(KhataNusus::HawiyaTalifa {
                    sigha: SIGHA,
                    haql: "an alignment site that does not hold the push it was recorded with",
                    qeema: mawdi.mawqi,
                    hadd: tul_u64(self.hawiya.bayt.len()),
                });
            }
            self.sajjil_tahreer(
                mawdi.mawqi,
                kalimat_daf(mawdi.jadeed),
                "a draw_text alignment constant mirrored",
            )?;
            let _ = self.istratijiyat.insert(IstratijiyaIzaha::Mawdi);
            adad = adad.saturating_add(1);
        }
        Ok(adad)
    }
}

/// The four bytes of a `pushi.e` of one alignment constant.
fn kalimat_daf(muhadhaha: MuhadhahaGameMaker) -> Vec<u8> {
    let mut kalima = Vec::with_capacity(4);
    kalima.extend_from_slice(&muhadhaha.qeema().to_le_bytes());
    kalima.push(NAW_SAHIH_QASEER);
    kalima.push(AMALIYAT_DAF);
    kalima
}

// ---------------------------------------------------------------------------
// The strategy descriptor
// ---------------------------------------------------------------------------

/// What a patch did to a container, and what it costs the player.
///
/// Built after the plan and before anything is installed, because the sentence
/// in [`WasfIstratijiya::takaleef`] has to reach a person while they can still
/// decline. A patch that transported its text is a patch whose text has stopped
/// being text, and a product that discovered that for the user after the fact
/// would have taken something from them without asking.
#[derive(Debug, Clone)]
pub struct WasfIstratijiya {
    /// Which runtime generation the container is.
    pub jeel: JeelHawiya,
    /// The bytecode version it declares.
    pub bytecode: u8,
    /// Which rung the tier probe put this game on.
    pub rutba: Rutba,
    /// Which rewriting strategies the plan used.
    pub istratijiyat: Vec<IstratijiyaIzaha>,
    /// Where the appended region went.
    pub mawdi_ilhaq: MawdiIlhaq,
    /// How many strings were replaced.
    pub adad_nusus: u32,
    /// How many glyphs were written into generated tables.
    pub adad_ashkal: u32,
    /// How many texture pages were appended.
    pub adad_safahat: u16,
    /// How many bytes the container grew.
    pub zaid: u64,
    /// Whether the glyph transport was used.
    pub naqala: bool,
}

impl WasfIstratijiya {
    /// Describes a plan.
    ///
    /// # Errors
    ///
    /// Whatever the insertion planning reports — the descriptor states how many
    /// bytes the container grows, and that number is only knowable once the
    /// insertions are known.
    pub fn min_muhawwil(
        muhawwil: &MuhawwilGameMaker,
        rutba: Rutba,
    ) -> Result<Self, KhataNusus> {
        let idrajat = muhawwil.idrajat()?;
        let zaid = idrajat
            .iter()
            .fold(0_u64, |kull, idraj| kull.saturating_add(tul_u64(idraj.bayt.len())));
        Ok(Self {
            jeel: muhawwil.hawiya.jeel(),
            bytecode: muhawwil.hawiya.gen8().bytecode,
            rutba,
            istratijiyat: muhawwil.istratijiyat().collect(),
            mawdi_ilhaq: muhawwil.mawdi_ilhaq(),
            adad_nusus: muhawwil.adad_nusus(),
            adad_ashkal: muhawwil.adad_ashkal(),
            adad_safahat: u16::try_from(muhawwil.safahat_mulhaqa.len()).unwrap_or(u16::MAX),
            zaid,
            naqala: muhawwil.naqala(),
        })
    }

    /// Whether this patch takes a capability away from the player.
    ///
    /// True exactly when the glyph transport was used, which is the same
    /// condition as [`Rutba::yukallif`] and is checked from the plan rather than
    /// inferred from the rung: a rung-three verdict that ended up not
    /// transporting anything costs nothing, and saying otherwise would be a
    /// warning people learn to ignore.
    #[must_use]
    pub const fn yukallif(&self) -> bool {
        self.naqala
    }

    /// What the player must be told before this patch is installed.
    #[must_use]
    pub const fn takaleef(&self) -> &'static str {
        if self.naqala {
            "This game's engine draws text from a baked table of glyph pictures and cannot \
             shape Arabic, so Taarib draws the text itself. It will look correct, and inside \
             the game it will NOT be selectable, searchable or copyable, and no screen reader \
             will be able to read it. Nothing outside the game is affected, and uninstalling \
             restores the original file byte for byte."
        } else {
            "Taarib replaced this game's text and changed nothing about how the game draws \
             it, so the text behaves exactly like the game's own."
        }
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub const fn takaleef_arabi(&self) -> &'static str {
        if self.naqala {
            "محرّك هذه اللعبة يرسم النصّ من جدول صور جاهزة ولا يستطيع تشكيل العربية، فيرسم \
             تعريب النصّ بنفسه. سيظهر صحيحًا، ولن يكون داخل اللعبة قابلًا للتحديد أو البحث \
             أو النسخ، ولن يقرأه قارئ الشاشة. لا يتأثّر شيء خارج اللعبة، وإزالة الترقيع \
             تُعيد الملف الأصلي بايتًا ببايت."
        } else {
            "استبدل تعريب نصوص هذه اللعبة ولم يغيّر شيئًا في طريقة رسمها، فيبقى النصّ نصًّا \
             حقيقيًا كما كان."
        }
    }

    /// The descriptor as lines for the log and the diagnostics bundle.
    #[must_use]
    pub fn sutur(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(6);
        sutur.push(format!(
            "GameMaker: {} (bytecode {}), rung {} ({})",
            self.jeel.ism(),
            self.bytecode,
            self.rutba as u8,
            self.rutba.ism()
        ));
        let asmaa: Vec<&str> =
            self.istratijiyat.iter().map(|istratijiya| istratijiya.ism()).collect();
        sutur.push(format!(
            "  rewriting: {}; appended region: {}",
            if asmaa.is_empty() { "nothing changed".to_owned() } else { asmaa.join(", ") },
            self.mawdi_ilhaq.ism()
        ));
        sutur.push(format!(
            "  {} string(s) replaced, {} glyph(s) baked, {} texture page(s) appended, \
             container grew by {} byte(s)",
            self.adad_nusus, self.adad_ashkal, self.adad_safahat, self.zaid
        ));
        sutur.push(format!("  {}", self.takaleef()));
        sutur
    }
}

// ---------------------------------------------------------------------------
// The tier probe
// ---------------------------------------------------------------------------

/// The container filenames, in the order they are worth trying.
///
/// The same list Phase 5's detector uses, kept here because a probe that asked
/// the detector for it would be asking a crate that does not export it, and
/// because the two lists changing independently is a bug either way round.
pub const HAWIYAT: [&str; 9] = [
    "data.win",
    "Data.win",
    "game.unx",
    "game.ios",
    "game.droid",
    "assets/game.unx",
    "assets/data.win",
    "assets/game.droid",
    "assets/game.ios",
];

/// Shared libraries whose presence beside a container means the game ships a
/// text stack the engine itself does not have.
///
/// A GameMaker game does not link any of these. One in a game's directory is an
/// extension, and an extension that carries `HarfBuzz` or `FriBidi` is a game that
/// may already shape and reorder Arabic correctly — which would make a takeover
/// a regression rather than a fix.
pub const MAKTABAT_TASHKEEL: [&str; 6] =
    ["harfbuzz", "fribidi", "icuuc", "icudt", "freetype", "raqm"];

/// Names that appear in a container's string pool when the game ships a text
/// extension.
///
/// Searched for in the pool because that is where an extension's own name, its
/// library filename and every function it exports are stored. Scribble is
/// GameMaker's most widely used text renderer and is named explicitly: it does
/// its own glyph layout, and a game using it is a game where taking the font
/// resource over changes nothing about how text is placed.
pub const ASMAA_IMTIDAD_NASS: [&str; 7] =
    ["harfbuzz", "fribidi", "scribble", "bidi", "shaper", "text_extension", "arabic"];

/// How many bytes of the string pool the probe scans.
///
/// Eight mebibytes. An extension's names sit in the pool wherever the compiler
/// put them, so the scan cannot start at a known offset — but a pool larger than
/// this belongs to a game whose script is a novel, and the names are in the
/// first pages of it either way.
pub const AQSA_MASH_HAWD: u64 = 8 * 1024 * 1024;

/// The GameMaker tier probe.
///
/// **The expected verdict is rung three, and it is still measured rather than
/// asserted.** GameMaker fonts are baked glyph tables pointing into texture
/// pages: there is no font file to swap, no shaping stage to configure, and the
/// glyph table's index is a character code. That is about as close to a
/// conclusive argument for a takeover as this crate has.
///
/// It is still not asserted, for one specific reason. A GameMaker game can ship
/// an extension — Scribble, or a native library binding `HarfBuzz` — that does its
/// own text layout, and for such a game taking the font resource over would
/// replace a working implementation with a newer one and lose whatever the
/// extension was doing. So the probe looks for that, weights it heavily, and
/// when both readings are strong lets [`crate::tabaqa::hukm`] refuse rather than
/// pick. A contradiction here is a game somebody has to look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MifhasGameMaker;

impl MifhasGameMaker {
    /// The probe.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// Finds the container under a game root.
    ///
    /// Every candidate is resolved through [`crate::tarkeeb::masar_bila_hala`]
    /// rather than joined literally. A depot that ships `Assets/game.unx` where
    /// this list says `assets/` runs correctly for the player — Wine's view of
    /// the filesystem is case-insensitive — and a literal join answers "no
    /// container", which [`crate::ayn_hadaf`] reads as "no adapter applies" and
    /// the installer reports as a success that translated nothing. The exact
    /// spelling is still tried first for each component, so a consistent depot
    /// costs no directory read at all.
    #[must_use]
    pub fn hawiya(jidhr: &Path) -> Option<PathBuf> {
        for nisbi in HAWIYAT {
            let murashah = masar_bila_hala(jidhr, nisbi);
            if murashah.is_file() {
                return Some(murashah);
            }
        }
        // A macOS application bundle keeps the container under Contents.
        let qaima = fs::read_dir(jidhr).ok()?;
        for madkhal in qaima.take(AQSA_MADAKHIL).flatten() {
            let ism = madkhal.file_name();
            let hazma = Path::new(&ism)
                .extension()
                .is_some_and(|lahiqa| lahiqa.eq_ignore_ascii_case("app"));
            if !hazma {
                continue;
            }
            // Resolved once per bundle rather than once per candidate: the three
            // names below live in the same directory.
            let mawarid = masar_bila_hala(&madkhal.path(), "Contents/Resources");
            for lahiqa in ["game.ios", "game.unx", "data.win"] {
                let murashah = ibn_bila_hala(&mawarid, lahiqa);
                if murashah.as_ref().is_some_and(|masar| masar.is_file()) {
                    return murashah;
                }
            }
        }
        None
    }
}

impl Mifhas for MifhasGameMaker {
    fn hadaf(&self) -> HadafNusus {
        HadafNusus::GameMaker
    }

    fn yantabiq(&self, siyaq: &SiyaqTabaqa<'_>) -> bool {
        if matches!(siyaq.aila, AilatMuharrik::GameMaker) {
            return true;
        }
        if Self::hawiya(siyaq.jidhr).is_some() {
            return true;
        }
        siyaq
            .tanfidhi
            .and_then(Path::parent)
            .is_some_and(|mujallad| Self::hawiya(mujallad).is_some())
    }

    fn adilla(&self, siyaq: &SiyaqTabaqa<'_>) -> Result<Vec<Dalil>, KhataNusus> {
        let mut adilla: Vec<Dalil> = Vec::new();
        let Some(masar) = Self::hawiya(siyaq.jidhr)
            .or_else(|| siyaq.tanfidhi.and_then(Path::parent).and_then(Self::hawiya))
        else {
            adilla.push(Dalil::siyaq(
                "nusus:gamemaker",
                "no data.win, game.unx, game.ios or game.droid is present under this game \
                 root, so there is no container to read a verdict out of",
            ));
            return Ok(adilla);
        };
        let ism = masar.display().to_string();

        let bayanat = fs::metadata(&masar)
            .map_err(|sabab| KhataNusus::KhataMalaf { masar: masar.clone(), sabab })?;
        let tul_malaf = bayanat.len();
        let mut malaf = File::open(&masar)
            .map_err(|sabab| KhataNusus::KhataMalaf { masar: masar.clone(), sabab })?;

        let Some(jadwal) = jadwal_qita_musalsal(&mut malaf, tul_malaf) else {
            adilla.push(Dalil::siyaq(
                ism.clone(),
                "the file is present and does not open with a FORM header, so it is not a \
                 GameMaker container",
            ));
            return Ok(adilla);
        };
        let asmaa: Vec<String> = jadwal.iter().map(QitaHawiya::ism_nass).collect();
        adilla.push(Dalil::siyaq(
            ism.clone(),
            format!("a FORM container of {tul_malaf} bytes holding: {}", asmaa.join(", ")),
        ));

        adilla_isdar(&mut malaf, tul_malaf, &jadwal, &ism, &mut adilla);
        adilla_khutut(&jadwal, &ism, &mut adilla);
        adilla_imtidad(&mut malaf, tul_malaf, &jadwal, &ism, &mut adilla);
        adilla_maktabat(siyaq.jidhr, &masar, &mut adilla);
        Ok(adilla)
    }
}

/// Reads a bounded run of bytes at an offset, after checking it is in the file.
///
/// The probe never reads a container whole: a `data.win` is routinely gigabytes,
/// and every question the probe asks is answered by a few dozen bytes of chunk
/// header plus one bounded scan. A short read is a short read rather than a
/// reason to invent bytes — a game's data file can be replaced by an updater
/// between the `stat` and the `read`.
fn iqra_bi_izaha(malaf: &mut File, tul_malaf: u64, izaha: u64, tul: usize) -> Option<Vec<u8>> {
    let nihaya = izaha.checked_add(u64::try_from(tul).ok()?)?;
    if nihaya > tul_malaf {
        return None;
    }
    let _ = malaf.seek(SeekFrom::Start(izaha)).ok()?;
    let mut bayt = vec![0_u8; tul];
    let mut mala = 0_usize;
    while mala < tul {
        match malaf.read(bayt.get_mut(mala..)?) {
            Ok(0) => break,
            Ok(adad) => mala = mala.checked_add(adad)?,
            Err(khata) if khata.kind() == ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    bayt.truncate(mala);
    Some(bayt)
}

/// Walks the chunk table by seeking, without reading a payload.
///
/// The probe's version of [`jadwal_qita`]: same walk, same bounds, and no
/// allocation proportional to the file. A truncated or nonsense table ends the
/// walk and the chunks reached so far are still evidence.
fn jadwal_qita_musalsal(malaf: &mut File, tul_malaf: u64) -> Option<Vec<QitaHawiya>> {
    let tarwisa = iqra_bi_izaha(malaf, tul_malaf, 0, 8)?;
    if !tarwisa.starts_with(&SIHR_FORM) {
        return None;
    }
    let mualan = u64::from(iqra_u32(&tarwisa, 4)?);
    let nihaya = HAJM_TARWISAT_QITA.saturating_add(mualan).min(tul_malaf);
    let mut jadwal: Vec<QitaHawiya> = Vec::new();
    let mut mawqi = HAJM_TARWISAT_QITA;
    while mawqi < nihaya && jadwal.len() < AQSA_QITA {
        let Some(ras) = iqra_bi_izaha(malaf, tul_malaf, mawqi, 8) else { break };
        let (Some(ism), Some(tul)) = (iqra_ism(&ras, 0), iqra_u32(&ras, 4).map(u64::from)) else {
            break;
        };
        if !ism.iter().all(u8::is_ascii_graphic) {
            break;
        }
        let Some(izaha) = mawqi.checked_add(HAJM_TARWISAT_QITA) else { break };
        let Some(baad) = izaha.checked_add(tul) else { break };
        if baad > nihaya {
            break;
        }
        jadwal.push(QitaHawiya { ism, tarwisa: mawqi, izaha, tul });
        mawqi = baad;
    }
    Some(jadwal)
}

/// Records what `GEN8` says, and says explicitly that it decides nothing.
///
/// The version is the single most tempting thing in this container to draw a
/// conclusion from and the single least informative. Every GameMaker runtime
/// from 1.4 to the 2024 releases draws text the same way — a baked glyph table
/// and a texture page — so a version number distinguishes chunk layouts and
/// nothing else. It is recorded as context, at weight zero, pointing nowhere.
fn adilla_isdar(
    malaf: &mut File,
    tul_malaf: u64,
    jadwal: &[QitaHawiya],
    ism: &str,
    adilla: &mut Vec<Dalil>,
) {
    let Some(qita) = qita_bi_ism(jadwal, ISM_GEN8) else {
        adilla.push(Dalil::siyaq(
            ism.to_owned(),
            "the container has no GEN8 chunk, so it is an IFF file of some other kind rather \
             than a GameMaker game",
        ));
        return;
    };
    let hadd = usize::try_from(qita.tul.min(128)).unwrap_or(128);
    let Some(bayt) = iqra_bi_izaha(malaf, tul_malaf, qita.izaha, hadd) else {
        adilla.push(Dalil::siyaq(
            ism.to_owned(),
            "the GEN8 chunk could not be read, so the container's generation is unknown",
        ));
        return;
    };
    let Some(bytecode) = bayt.get(1).copied() else {
        adilla.push(Dalil::siyaq(ism.to_owned(), "the GEN8 chunk is shorter than its version"));
        return;
    };
    let jeel = JeelHawiya::min_bytecode(bytecode)
        .map_or_else(|_| "a generation this build does not read".to_owned(), |jeel| {
            jeel.ism().to_owned()
        });
    let isdar = (44..=56)
        .step_by(4)
        .filter_map(|izaha| iqra_u32(&bayt, izaha))
        .map(|juz| juz.to_string())
        .collect::<Vec<String>>()
        .join(".");
    adilla.push(Dalil::siyaq(
        ism.to_owned(),
        format!(
            "GEN8 declares bytecode {bytecode} ({jeel}) and runtime {isdar}; that selects \
             which chunk layouts this build reads and says nothing about shaping, because \
             every GameMaker runtime draws text from a baked glyph table"
        ),
    ));
}

/// The evidence that actually decides the rung.
fn adilla_khutut(jadwal: &[QitaHawiya], ism: &str, adilla: &mut Vec<Dalil>) {
    match qita_bi_ism(jadwal, ISM_FONT) {
        Some(qita) => adilla.push(Dalil::jadeed(
            ism.to_owned(),
            format!(
                "a FONT chunk of {} bytes is present: this game's text is drawn from baked \
                 glyph tables indexed by character code and pointing into texture pages. \
                 There is no font file to substitute and no shaping stage to configure, so \
                 an Arabic string handed to draw_text is drawn one unjoined picture at a time",
                qita.tul
            ),
            Some(Rutba::Istila),
            85,
        )),
        None => adilla.push(Dalil::siyaq(
            ism.to_owned(),
            "there is no FONT chunk, so this game's text is not drawn from GameMaker font \
             resources at all — it comes from sprites, from an extension, or from a runtime \
             this container does not describe, and which of those it is has to be found \
             before any rung can be chosen",
        )),
    }
    if qita_bi_ism(jadwal, ISM_STRG).is_none() {
        adilla.push(Dalil::siyaq(
            ism.to_owned(),
            "there is no STRG chunk, so there is no string pool to translate into",
        ));
    }
}

/// Whether a byte run contains an ASCII needle, case-insensitively.
///
/// A byte search rather than a string one because the pool is scanned in bounded
/// slices and a slice can cut a multi-byte character in half; decoding it would
/// either fail on that boundary or silently substitute a replacement character,
/// and neither is worth doing to look for seven ASCII names.
fn yahwi_ascii(kawm: &[u8], ibra: &str) -> bool {
    let matlub = ibra.as_bytes();
    if matlub.is_empty() || kawm.len() < matlub.len() {
        return false;
    }
    kawm.windows(matlub.len()).any(|nafidha| {
        nafidha
            .iter()
            .zip(matlub.iter())
            .all(|(wahid, hadaf)| wahid.eq_ignore_ascii_case(hadaf))
    })
}

/// Looks for a text extension, which is the one thing that could put this game
/// on a rung above the takeover.
///
/// The pool is where an extension's own name, its library filename and every
/// function it exports are stored, so that is what is scanned. The names split
/// into two strengths and they are weighted differently on purpose:
///
/// * `harfbuzz`, `fribidi` and `scribble` are near-conclusive. A GameMaker game
///   does not contain those strings by accident, and each of them is a text
///   stack that lays out its own glyphs — taking the font resource over would
///   replace something that works.
/// * `bidi`, `shaper`, `arabic` and `text_extension` are suggestive and no more.
///   A game with an Arabic language option in its menu has the word `arabic` in
///   its pool and no shaping whatsoever, which is exactly the game this product
///   was built for.
///
/// The strong reading scores seventy against the font chunk's eighty-five, which
/// is inside [`crate::tabaqa::FARQ_HASIM`] — so a game that ships both a `FONT`
/// chunk and `HarfBuzz` produces a refusal rather than a verdict. That is the
/// intended outcome: those two readings disagree about whether the engine is
/// already correct, and choosing between them by tiebreak would be choosing
/// arbitrarily between shipping unjoined Arabic and taking over a working
/// implementation.
fn adilla_imtidad(
    malaf: &mut File,
    tul_malaf: u64,
    jadwal: &[QitaHawiya],
    ism: &str,
    adilla: &mut Vec<Dalil>,
) {
    if let Some(qita) = qita_bi_ism(jadwal, ISM_EXTN) {
        adilla.push(Dalil::siyaq(
            ism.to_owned(),
            format!(
                "an EXTN chunk of {} bytes is present, so this game loads at least one \
                 native extension",
                qita.tul
            ),
        ));
    }
    let Some(qita) = qita_bi_ism(jadwal, ISM_STRG) else {
        return;
    };
    let hadd = usize::try_from(qita.tul.min(AQSA_MASH_HAWD)).unwrap_or_default();
    let Some(kawm) = iqra_bi_izaha(malaf, tul_malaf, qita.izaha, hadd) else {
        return;
    };

    let qawiya: Vec<&str> = ["harfbuzz", "fribidi", "scribble"]
        .into_iter()
        .filter(|ibra| yahwi_ascii(&kawm, ibra))
        .collect();
    if !qawiya.is_empty() {
        adilla.push(Dalil::jadeed(
            ism.to_owned(),
            format!(
                "the string pool names {}, which is a text stack that lays out its own \
                 glyphs; if this game draws its text through it, the engine may already \
                 shape and reorder Arabic and a takeover would replace something that works",
                qawiya.join(", ")
            ),
            Some(Rutba::Tasheeh),
            70,
        ));
    }
    let daeefa: Vec<&str> = ASMAA_IMTIDAD_NASS
        .into_iter()
        .filter(|ibra| !qawiya.contains(ibra) && yahwi_ascii(&kawm, ibra))
        .collect();
    if !daeefa.is_empty() {
        adilla.push(Dalil::jadeed(
            ism.to_owned(),
            format!(
                "the string pool contains {}, which is suggestive and no more: a game with \
                 an Arabic language option in its menu carries the same words and shapes \
                 nothing",
                daeefa.join(", ")
            ),
            Some(Rutba::Tasheeh),
            30,
        ));
    }
    if yahwi_ascii(&kawm, "draw_set_halign") {
        adilla.push(Dalil::siyaq(
            ism.to_owned(),
            "the game calls draw_set_halign, so it sets text alignment explicitly and the \
             patch has to mirror those constants rather than assume a default",
        ));
    }
}

/// Looks for a shaping library shipped beside the container.
///
/// A GameMaker runtime links none of these. One in the game's directory is an
/// extension's dependency, and it is weaker evidence than a name in the pool
/// because a library can be shipped and never loaded — a leftover from a build
/// that tried something is a real thing to find in a game directory.
fn adilla_maktabat(jidhr: &Path, masar: &Path, adilla: &mut Vec<Dalil>) {
    let mut wujidat: Vec<String> = Vec::new();
    for mujallad in [Some(jidhr), masar.parent()].into_iter().flatten() {
        let Ok(qaima) = fs::read_dir(mujallad) else {
            continue;
        };
        for madkhal in qaima.take(AQSA_MADAKHIL).flatten() {
            let ism = madkhal.file_name().to_string_lossy().to_ascii_lowercase();
            // `.so.N` is a versioned soname, which `Path::extension` reads as `N`.
            let maktaba = Path::new(&ism).extension().is_some_and(|lahiqa| {
                ["dll", "so", "dylib"].iter().any(|naw| lahiqa.eq_ignore_ascii_case(naw))
            }) || ism.contains(".so.");
            if maktaba && MAKTABAT_TASHKEEL.iter().any(|ibra| ism.contains(ibra)) {
                wujidat.push(ism);
            }
        }
    }
    wujidat.sort_unstable();
    wujidat.dedup();
    if wujidat.is_empty() {
        return;
    }
    adilla.push(Dalil::jadeed(
        "nusus:gamemaker",
        format!(
            "shaping libraries are shipped beside the container: {}. A GameMaker runtime \
             links none of them, so an extension brought them — though a library can be \
             shipped and never loaded, which is why this is weighed below what the string \
             pool says",
            wujidat.join(", ")
        ),
        Some(Rutba::Tasheeh),
        55,
    ));
}


