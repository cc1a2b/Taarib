//! غودوت — Godot, and the one question this detector exists to answer.
//!
//! # Godot 3 and Godot 4 are two engines wearing one name
//!
//! Everything else in this file is secondary to that sentence. Taarib's Godot
//! adapter (Phase 9) is not one adapter with a version flag; it is two entirely
//! separate strategies, and the version decides which one runs:
//!
//! - **Godot 4** ships `TextServerAdvanced`, a real text server built on
//!   `HarfBuzz` and ICU. It shapes Arabic, applies the bidirectional algorithm,
//!   positions marks, and does it correctly. Taarib therefore *does not* take
//!   the text over. It registers its bundled Arabic font as a `FontFile` created
//!   from patch bytes, sets the text and base direction on the `Control` nodes
//!   the patch names, loads a generated `Translation` resource for `ar`, and
//!   lets the engine render. The game's own theme, effects, animations and
//!   materials keep working, because the text is still the engine's text.
//! - **Godot 3** has no text server at all. It draws through `Font::draw_char`,
//!   one codepoint at a time, with no shaping, no joining, no bidi and no mark
//!   attachment. Taarib therefore takes the text over completely: it hooks the
//!   drawing path, intercepts every string, lays it out through `jisr`, and
//!   draws glyphs from its own atlas through
//!   `VisualServer::canvas_item_add_texture_rect_region` — an API that has not
//!   changed across all of 3.x.
//!
//! **Getting this backwards fails in two different, equally visible ways.**
//! Treat a Godot 3 game as Godot 4 and Taarib hands Arabic strings to an engine
//! that cannot shape them: the player gets twenty-eight disconnected isolated
//! letterforms in left-to-right order, which is not bad Arabic, it is not Arabic.
//! Treat a Godot 4 game as Godot 3 and Taarib tears out a text pipeline that was
//! already correct, replacing engine-native text with drawn glyphs, losing the
//! theme, the outlines, the per-character animations and the `RichTextLabel`
//! effects for no gain at all — a redundant takeover that is strictly worse than
//! doing nothing.
//!
//! So the version is not a detail in the capability report. It is the dispatch.
//! Every discriminator below is ranked by how hard it is to get wrong.
//!
//! ## The discriminators, strongest first
//!
//! | evidence | says | how firm |
//! | --- | --- | --- |
//! | PCK header engine major field | 3 or 4 directly | decisive |
//! | PCK format version 1 vs 2+ | 1 is Godot 3, 2+ is Godot 4 | decisive |
//! | `project.godot` `config_version=4` / `=5` | 4 is Godot 3, 5 is Godot 4 | decisive |
//! | `config/features=("4.3", ...)` | the minor version too | decisive |
//! | a `.gdextension` file | Godot 4: `GDExtension` did not exist before it | strong |
//! | a `.gdnlib` / `.gdns` file | Godot 3: `GDNative` was replaced in 4 | strong |
//! | `data_*/Mono/` | Godot 3 with C#, which shipped a Mono runtime | strong |
//! | `GodotSharp.dll` / `*.runtimeconfig.json` | Godot 4 with C# on .NET | strong |
//!
//! The header fields and the format version are read from the same 100 bytes and
//! are checked against each other. When they disagree — a format 2 package
//! claiming engine major 3 — the engine major wins and the disagreement is
//! recorded as evidence rather than smoothed over, because that combination is
//! either a repacking tool that wrote the wrong constant or a build this
//! detector has never seen, and both of those are things a maintainer needs to
//! be told.
//!
//! ## Division of labour with the other detectors
//!
//! `dalail::binya` walks the game once for every engine's directory shape and
//! sees the Godot markers a walk can see: a `.pck` beside an executable named
//! after it, `project.godot`, a `.console.exe`, a `data_*` directory. It
//! deliberately claims **no version**, because the shape is identical for both
//! generations and guessing it would be guessing at the answer the user is
//! shown. `dalail::thunai` reads the executable for module and string
//! signatures.
//!
//! This detector is the one that opens the package. Several of the shapes above
//! are observed here as well, which costs a duplicated line in the evidence
//! trail and buys independence: [`crate::tahdid`] combines by strongest weight
//! per family rather than by summing, so nothing is inflated by being seen
//! twice, and this file does not silently lose evidence when another file
//! changes.
//!
//! ## The PCK header, exactly as it is written
//!
//! Both formats begin with the same five fields and diverge after them. All
//! little-endian.
//!
//! **Format 1 — Godot 3**
//!
//! | offset | field | bytes |
//! | --- | --- | --- |
//! | 0 | magic `GDPC` (`0x43504447` as a `u32`) | 4 |
//! | 4 | pack format version (1) | 4 |
//! | 8 | engine major | 4 |
//! | 12 | engine minor | 4 |
//! | 16 | engine patch | 4 |
//! | 20 | reserved, sixteen `u32` | 64 |
//! | 84 | file count | 4 |
//!
//! **Format 2 — Godot 4**
//!
//! | offset | field | bytes |
//! | --- | --- | --- |
//! | 0 | magic `GDPC` | 4 |
//! | 4 | pack format version (2, and 3 from the 4.4 line) | 4 |
//! | 8 | engine major | 4 |
//! | 12 | engine minor | 4 |
//! | 16 | engine patch | 4 |
//! | 20 | pack flags | 4 |
//! | 24 | file base offset | 8 |
//! | 32 | reserved, sixteen `u32` | 64 |
//! | 96 | file count | 4 |
//!
//! Pack flags is a bitfield whose bit 0 is `PACK_DIR_ENCRYPTED`. Bit 1 has been
//! observed carrying a relative-file-base marker on the 4.4 line; this reader
//! records the raw value and interprets only bit 0, so a flag added later
//! changes nothing here.
//!
//! **The `4.4` caveat.** The Godot 4 line has been observed writing pack format
//! 3 as well as 2. This reader never keys on the exact number: it treats format
//! 1 as the Godot 3 line, format 2 and above as the Godot 4 line, and confirms
//! both against the engine major field that sits four bytes later. A format
//! number invented after this build therefore lands in the right place.
//!
//! ## The embedded package, and the arithmetic that finds it
//!
//! Godot can append the whole PCK to the end of the executable, which is the
//! default for a single-file export. The engine finds it by reading from the end
//! of its own binary, and this detector does exactly what the engine does:
//!
//! ```text
//!   [ ...executable... ][ PCK: GDPC ... ][ pad ][ u64 size ][ u32 GDPC ]  EOF
//!                       ^                                   ^
//!                       |                                   the last 4 bytes
//!                       starts at  len - 12 - size
//! ```
//!
//! - The **last four bytes** are the magic again. No magic there, no embedded
//!   package, and the check costs one twelve-byte read.
//! - The **eight bytes before it** are a `u64` size, counted from the first byte
//!   of the embedded PCK up to but not including the size field itself. The
//!   alignment padding Godot inserts before the size is inside that count.
//! - The package therefore starts at `file_len - 12 - size`, and the byte there
//!   must be the *leading* `GDPC`. This reader checks that the subtraction does
//!   not underflow before it seeks, and checks the leading magic after it does,
//!   so a binary whose last twelve bytes happen to end in `GDPC` produces the
//!   absence of evidence rather than a wild seek.
//!
//! The same trailer is written by Godot 3 and Godot 4, so an embedded package is
//! read for its format version exactly like a loose one.
//!
//! ## Encryption
//!
//! Format 2 carries `PACK_DIR_ENCRYPTED` in its flags word, and this reader
//! reports it: an encrypted directory means the package cannot be listed, let
//! alone translated, until the user supplies the key the game was exported with.
//!
//! Format 1 has no flags word. Godot 3 encrypts *entries* — the script
//! encryption key applies to the compiled `.gdc` files inside the package — and
//! an entry's flags live in the index, which this detector does not walk. So for
//! a Godot 3 package the honest answer is **unknown**, not "not encrypted", and
//! that is what [`TarwisatPck::mushaffara`] returns.
//!
//! ## The three scripting backends, and how they are told apart
//!
//! | backend | evidence | note |
//! | --- | --- | --- |
//! | `GodotCSharp` | `data_*/Mono/` in 3.x, `GodotSharp.dll` in 4.x | positive |
//! | `GodotNative` | no package anywhere: not loose, not embedded | residue |
//! | `GdScript` | a package, and no .NET marker beside it | inferred from absence |
//!
//! The names are [`KhalfiyaBarmajiya::GodotCSharp`], [`KhalfiyaBarmajiya::GodotNative`]
//! and [`KhalfiyaBarmajiya::GdScript`].
//!
//! Two of those are honest inferences rather than observations, and are labelled
//! as such in the evidence trail. `GDScript` compiles into the package and leaves
//! no file outside it, so *nothing on disk positively says "this game uses
//! `GDScript`"* — the absence of a runtime is the whole of the evidence. And a
//! custom engine build with the project compiled in is diagnosed only after the
//! embedded-package check has failed, because "no `.pck` beside the executable"
//! is far more often an embedded package than a native build.
//!
//! ## Text systems
//!
//! [`ItarNusus::GodotLabel`] is asserted whenever Godot is confirmed, and the
//! basis is stated rather than assumed: `Label` is the engine's base text
//! control, it is compiled into every export template, and a Godot game with an
//! interface has one. What this detector cannot see is *which* nodes a game
//! actually uses, because the scene tree lives inside the package and opening it
//! is Phase 12's job, not a probe's.
//!
//! [`ItarNusus::GodotRichText`] is **not** asserted on that basis. It is
//! asserted only where a `RichTextLabel` was actually seen — in a loose `.tscn`
//! or `.tres` in a game shipped with unpacked resources — because a rich text
//! label brings `BBCode`, and `BBCode` brings a markup dialect `nasq` has to parse
//! before any of it reaches the shaper. Promising that surface without evidence
//! would put a parser in the patch pipeline for a game that has no use for one.
//!
//! ## Weights
//!
//! | observation | weight | why |
//! | --- | --- | --- |
//! | validated `GDPC` header | 95 | near-conclusive: magic, format and engine fields agree |
//! | validated embedded package | 95 | same header, reached through the trailer |
//! | `project.godot` with a `config_version` | 85 | Godot's own project file, and nobody else's |
//! | `.tscn`/`.tres` with a `gd_scene`/`gd_resource` head | 80 | Godot's own resource format |
//! | a `.gdextension` file | 80 | exists only in Godot 4 |
//! | a `.gdnlib`/`.gdns` file | 75 | exists only in Godot 3 |
//! | `data_*/Mono` or `GodotSharp.dll` | 70 | the C# export layout |
//! | `<Game>.console.exe` | 45 | Godot 4's console wrapper; other tools ship look-alikes |
//! | `.godot/` or `.import/` directory | 40 | a project cache, sometimes shipped by accident |
//! | `.pck` extension with no `GDPC` | 25 | the extension is used by several unrelated products |
//! | `_sc_` self-contained marker | 20 | one empty file, and easy to imitate |
//! | derived detail (encryption, backend) | 30 | already implied by the header it came from |
//! | `RichTextLabel` seen in a loose scene | 55 | a real observation of a real node type |

use std::cmp::Reverse;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use taarib_mustalahat::muharrik::{
    AilatMuharrik, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel,
};
use taarib_usus::khata::Natija;

use crate::dalail::imtidad;
use crate::fahs::{AQSA_MADAKHIL, AQSA_UMQ, Fahis, HAJM_TARWISA, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

/// `PACK_HEADER_MAGIC`, which Godot stores as a little-endian `u32` and which
/// therefore reads as the four ASCII bytes `GDPC` in file order — at the start
/// of a loose package and again in the last four bytes of an embedded one.
const TAWQI_PCK: &[u8; 4] = b"GDPC";

/// Where the file count sits in a format 1 header, and therefore how long that
/// header is.
const IZAHAT_ADAD_V1: usize = 84;

/// Where the file count sits in a format 2 header.
const IZAHAT_ADAD_V2: usize = 96;

/// How many bytes are read from the start of a package.
///
/// Enough for the longer of the two headers plus its file count, and nothing
/// beyond it. The index that follows can describe a hundred thousand files and
/// is none of this detector's business.
pub const HAJM_RASS_PCK: usize = 128;

/// The embedded-package trailer: a `u64` size and a `u32` magic.
pub const HAJM_DHAYL_MUDMAJ: usize = 12;

/// `PACK_DIR_ENCRYPTED` in a format 2 header's flags word.
const ALAM_FAHRAS_MUSHAFFAR: u32 = 1 << 0;

/// A file count above this is not a file count; it is a header that was read at
/// the wrong offset or a package that is truncated.
///
/// Four million entries is far past anything a shipped game carries, and small
/// enough that a garbage `u32` fails it almost always.
const AQSA_ADAD_MALAFFAT: u32 = 4_000_000;

/// How many `.pck` candidates are opened before the search stops.
const AQSA_HUZAM: usize = 8;

/// How many executables are checked for an embedded package.
const AQSA_TANFIDHIYAT: usize = 8;

/// How many loose scene or resource files are read looking for node types.
const AQSA_MASHAHID: usize = 16;

/// The executable extensions worth checking for an embedded package.
///
/// The empty string covers Linux and macOS binaries, which have no extension at
/// all; `.x86_64` and `.x86` cover Godot's own Linux export-template naming.
const IMTIDADAT_TANFIDH: [&str; 5] = ["exe", "x86_64", "x86", "arm64", "bin"];

/// Extensions of Godot's own text and binary resource files.
const IMTIDADAT_MAWRID: [&str; 4] = ["tscn", "tres", "scn", "res"];

/// The needle that identifies a `RichTextLabel` inside a scene or resource,
/// text or binary: both formats store the node's type name as a plain ASCII
/// string, so one byte search covers both.
const IBRAT_NASS_GHANI: &[u8] = b"RichTextLabel";

/// The same for a plain `Label`.
const IBRAT_LABEL: &[u8] = b"Label";

// ---------------------------------------------------------------------------
// Bounded reading
//
// A `.pck` is a shipping container and can be tens of gigabytes; an executable
// with a package embedded in it is exactly as large. Nothing here reads a whole
// file. The header read is 128 bytes from the start, the trailer read is 12
// bytes from the end, and the one seek that goes anywhere else is validated
// against the file's real length first — an embedded size claiming the package
// starts before byte zero has to produce the absence of evidence, not a seek.
// ---------------------------------------------------------------------------

/// Reads up to `hajm` bytes from the start of a file, with the file's length.
fn iqra_rass(masar: &Path, hajm: usize) -> Option<(Vec<u8>, u64)> {
    let mut malaf = File::open(masar).ok()?;
    let tul = malaf.metadata().ok()?.len();
    let miqdar = usize::try_from(tul.min(u64::try_from(hajm).ok()?)).ok()?;
    let mut bayt = vec![0_u8; miqdar];
    malaf.read_exact(&mut bayt).ok()?;
    Some((bayt, tul))
}

/// Reads up to `hajm` bytes starting at `mawdi`, refusing an offset that is not
/// inside the file and clamping a read that would run past its end.
///
/// The clamp matters for an embedded package that sits close to the end of its
/// executable: the header is there, the 128 bytes this reader would like are
/// not, and reading what exists is right where refusing would lose the package.
fn iqra_min(masar: &Path, mawdi: u64, hajm: usize) -> Option<Vec<u8>> {
    let mut malaf = File::open(masar).ok()?;
    let tul = malaf.metadata().ok()?.len();
    if mawdi >= tul {
        return None;
    }
    let mutah = tul.checked_sub(mawdi)?;
    let miqdar = usize::try_from(mutah.min(u64::try_from(hajm).ok()?)).ok()?;
    let _ = malaf.seek(SeekFrom::Start(mawdi)).ok()?;
    let mut bayt = vec![0_u8; miqdar];
    malaf.read_exact(&mut bayt).ok()?;
    Some(bayt)
}

/// Reads the last `hajm` bytes of a file, with the file's length.
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

/// Whether a byte slice contains a needle. Linear, and over at most
/// [`HAJM_TARWISA`] bytes.
fn yahtawi(kawm: &[u8], ibra: &[u8]) -> bool {
    if ibra.is_empty() || kawm.len() < ibra.len() {
        return false;
    }
    kawm.windows(ibra.len()).any(|nafidha| nafidha == ibra)
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

/// The entry budget for one probe, shared across every directory opened.
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
    /// A directory that will not open yields nothing, which is the right
    /// reading of a permission failure. Symbolic links are skipped rather than
    /// followed, so a link inside the game cannot walk the probe out of it.
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

    /// Every entry from the root down to [`AQSA_UMQ`] levels, breadth first.
    ///
    /// Breadth first on purpose: Godot puts its markers next to the executable,
    /// so the shallow entries are the ones worth the budget, and a game that
    /// exhausts the budget in a deep asset tree still had its root read first.
    fn shajara(&mut self, jidhr: &Path) -> Vec<Madkhal> {
        let mut kul = Vec::new();
        let mut tabaqa = vec![jidhr.to_path_buf()];
        for _ in 0..AQSA_UMQ {
            let mut talia = Vec::new();
            for masar in &tabaqa {
                for madkhal in self.madakhil(masar) {
                    if madkhal.mujallad {
                        talia.push(madkhal.masar.clone());
                    }
                    kul.push(madkhal);
                }
            }
            if talia.is_empty() {
                break;
            }
            tabaqa = talia;
        }
        kul
    }
}

/// Whether a path's extension matches, ignoring case.
fn imtidad_huwa(masar: &Path, imtidad: &str) -> bool {
    masar
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|mawjud| mawjud.eq_ignore_ascii_case(imtidad))
}

/// A path relative to the game root, as the evidence trail records it.
fn nisbi(jidhr: &Path, masar: &Path) -> Option<String> {
    masar
        .strip_prefix(jidhr)
        .ok()
        .map(|nisbi| nisbi.display().to_string())
}

// ---------------------------------------------------------------------------
// Which Godot
// ---------------------------------------------------------------------------

/// Which generation of Godot a game was built with.
///
/// This is the value Phase 9 dispatches on, and the reason this module exists.
/// It is not a version label: it names two different adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JeelGodot {
    /// Godot 3. No text server, no shaping, no bidirectional algorithm.
    ///
    /// Tier 2: Taarib hooks the drawing path, shapes through `jisr`, and draws
    /// its own glyphs from its own atlas. The engine never sees Arabic text,
    /// because it could not do anything correct with it if it did.
    Thalith,
    /// Godot 4. `TextServerAdvanced` shapes, applies bidi and positions marks.
    ///
    /// Tier 1: Taarib registers a font, sets direction on the controls the patch
    /// names, and translates through the engine's own translation system. The
    /// engine renders, so every theme, effect and animation the game already
    /// applies to its text keeps applying.
    Rabi,
}

impl JeelGodot {
    /// The generation's name as the report writes it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Thalith => "Godot 3",
            Self::Rabi => "Godot 4",
        }
    }

    /// Whether the engine shapes text itself.
    ///
    /// The single fact Phase 9 branches on. True means hand the engine a font
    /// and get out of the way; false means take the text over entirely.
    #[must_use]
    pub const fn yushakkil(self) -> bool {
        matches!(self, Self::Rabi)
    }

    /// One sentence naming what the adapter does, for the evidence trail.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Thalith => {
                "Godot 3 has no TextServer: no shaping, no bidi, no mark attachment. Taarib \
                 takes the text over and draws the glyphs itself"
            },
            Self::Rabi => {
                "Godot 4 ships TextServerAdvanced, which shapes and applies the bidirectional \
                 algorithm. Taarib registers a font and translates through the engine"
            },
        }
    }
}

// ---------------------------------------------------------------------------
// The PCK header
// ---------------------------------------------------------------------------

/// A Godot package header, read either from the start of a `.pck` or from an
/// embedded package inside an executable.
///
/// The index that follows is not read. Phase 9 reads it, writes it, and mounts
/// a patch package over it; a probe only has to know what the package is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TarwisatPck {
    /// The pack format version: 1 for the Godot 3 line, 2 and above for the
    /// Godot 4 line.
    pub sigha: u32,
    /// Engine major version, written by the exporter that produced the package.
    pub kabir: u32,
    /// Engine minor version.
    pub sagheer: u32,
    /// Engine patch version. Frequently zero even for a 3.5.2 build, because
    /// several 3.x exporters wrote a zero here; it is reported as found and
    /// never used to tell two builds apart.
    pub tasheeh: u32,
    /// The flags word, format 2 and above only.
    pub aalam: Option<u32>,
    /// The file-base offset, format 2 and above only.
    pub asas: Option<u64>,
    /// How many entries the index declares.
    pub adad_malaffat: u32,
    /// Where the package starts inside the file it was found in. Zero for a
    /// loose `.pck`.
    pub bidaya: u64,
    /// Whether the package was appended to an executable rather than shipped as
    /// its own file.
    pub mudmaja: bool,
    /// The length of the file the package was found in.
    pub hajm_malaf: u64,
}

impl TarwisatPck {
    /// Reads a package header from a file, loose first and embedded second.
    ///
    /// A `.pck` is tried at offset zero; anything else — an executable, or a
    /// `.pck` whose first bytes are not `GDPC` — falls through to the trailer at
    /// the end of the file. A file that yields neither returns [`None`], which
    /// is the absence of evidence and never an error.
    #[must_use]
    pub fn iqra(masar: &Path) -> Option<Self> {
        Self::min_rass(masar).or_else(|| Self::min_mudmaj(masar))
    }

    /// Reads a package that begins at the first byte of the file.
    #[must_use]
    pub fn min_rass(masar: &Path) -> Option<Self> {
        let (rass, tul) = iqra_rass(masar, HAJM_RASS_PCK)?;
        Self::tahlil(&rass, 0, false, tul)
    }

    /// Reads a package appended to the end of an executable.
    ///
    /// The arithmetic is the engine's own, and every step of it is checked:
    /// the trailing magic before the size is read, the size before the
    /// subtraction, and the subtraction before the seek. A twelve-byte tail that
    /// happens to end in `GDPC` therefore costs one failed header parse and
    /// nothing else.
    #[must_use]
    pub fn min_mudmaj(masar: &Path) -> Option<Self> {
        let (dhayl, tul) = iqra_dhayl(masar, HAJM_DHAYL_MUDMAJ)?;
        if dhayl.len() < HAJM_DHAYL_MUDMAJ || !dhayl.ends_with(TAWQI_PCK) {
            return None;
        }
        let hajm = u64_min(&dhayl, 0)?;
        // The package plus its own trailer cannot be longer than the file that
        // holds it, and a zero-length package is not one.
        if hajm == 0 || hajm.checked_add(u64::try_from(HAJM_DHAYL_MUDMAJ).ok()?)? > tul {
            return None;
        }
        let bidaya = tul
            .checked_sub(u64::try_from(HAJM_DHAYL_MUDMAJ).ok()?)?
            .checked_sub(hajm)?;
        let rass = iqra_min(masar, bidaya, HAJM_RASS_PCK)?;
        Self::tahlil(&rass, bidaya, true, tul)
    }

    /// Parses a header out of bytes that start at the package's first byte.
    ///
    /// Every field is range-checked before the header is accepted. The magic
    /// alone is four bytes and four bytes occur by accident; the magic plus a
    /// plausible format version plus a plausible engine version plus a file
    /// count that is not absurd do not.
    fn tahlil(bayt: &[u8], bidaya: u64, mudmaja: bool, tul: u64) -> Option<Self> {
        if !bayt.starts_with(TAWQI_PCK) {
            return None;
        }
        let sigha = u32_min(bayt, 4)?;
        let kabir = u32_min(bayt, 8)?;
        let sagheer = u32_min(bayt, 12)?;
        let tasheeh = u32_min(bayt, 16)?;
        // Godot's major version has been a single digit since 2014 and its
        // minor has never reached 100. A header that says otherwise was read at
        // the wrong offset.
        if kabir == 0 || kabir > 9 || sagheer > 99 || tasheeh > 99 || sigha > 16 {
            return None;
        }

        let (aalam, asas, izahat_adad) = if sigha >= 2 {
            (
                Some(u32_min(bayt, 20)?),
                Some(u64_min(bayt, 24)?),
                IZAHAT_ADAD_V2,
            )
        } else {
            (None, None, IZAHAT_ADAD_V1)
        };
        // The file base is an offset into the file for an embedded package and
        // is relative to the package start on the 4.4 line. Either reading has
        // to land inside the file.
        if let Some(asas) = asas
            && asas > tul
            && bidaya.checked_add(asas).is_none_or(|nihaya| nihaya > tul)
        {
            return None;
        }

        let adad_malaffat = u32_min(bayt, izahat_adad)?;
        if adad_malaffat > AQSA_ADAD_MALAFFAT {
            return None;
        }

        Some(Self {
            sigha,
            kabir,
            sagheer,
            tasheeh,
            aalam,
            asas,
            adad_malaffat,
            bidaya,
            mudmaja,
            hajm_malaf: tul,
        })
    }

    /// Which generation of Godot wrote this package.
    ///
    /// The engine major field decides, because it is the engine's own statement
    /// about itself. A major of 2 is Godot 2, which is outside the coverage
    /// matrix, and returns [`None`] rather than being rounded up to something
    /// Taarib supports. A major above 4 is treated as the Godot 4 line — an
    /// assumption, stated here as one: an engine released after this build will
    /// have a text server, because removing one is not a thing engines do.
    #[must_use]
    pub const fn jeel(&self) -> Option<JeelGodot> {
        match self.kabir {
            3 => Some(JeelGodot::Thalith),
            4..=9 => Some(JeelGodot::Rabi),
            _ => None,
        }
    }

    /// Which generation the *format version* alone would say, which is what
    /// gets compared against [`TarwisatPck::jeel`].
    #[must_use]
    pub const fn jeel_min_sigha(&self) -> Option<JeelGodot> {
        match self.sigha {
            1 => Some(JeelGodot::Thalith),
            2..=16 => Some(JeelGodot::Rabi),
            _ => None,
        }
    }

    /// Whether the format version and the engine major disagree.
    ///
    /// Reported, never resolved silently. A format 2 package claiming engine
    /// major 3 is either a repacking tool that wrote the wrong constant or a
    /// build nobody here has seen, and both need saying out loud.
    #[must_use]
    pub fn tanaqud(&self) -> bool {
        match (self.jeel(), self.jeel_min_sigha()) {
            (Some(min_kabir), Some(min_sigha)) => min_kabir != min_sigha,
            _ => false,
        }
    }

    /// Whether the package's directory is encrypted.
    ///
    /// [`None`] is a real answer and the only honest one for a format 1
    /// package: Godot 3 has no flags word in its header and encrypts individual
    /// entries instead, and the entry flags live in an index this detector does
    /// not walk. "Unknown" and "not encrypted" are different promises.
    #[must_use]
    pub fn mushaffara(&self) -> Option<bool> {
        self.aalam.map(|aalam| aalam & ALAM_FAHRAS_MUSHAFFAR != 0)
    }

    /// The engine version, exactly as the exporter wrote it into the package.
    ///
    /// Unlike Unreal's containers this is a point value rather than a range,
    /// because Godot writes its own major, minor and patch into every package
    /// it produces.
    #[must_use]
    pub fn isdar(&self) -> Option<IsdarMuharrik> {
        let kabir = u16::try_from(self.kabir).ok()?;
        let sagheer = u16::try_from(self.sagheer).ok()?;
        let tasheeh = u16::try_from(self.tasheeh).ok()?;
        let khaam = format!("{kabir}.{sagheer}.{tasheeh} (PCK format {})", self.sigha);
        Some(IsdarMuharrik {
            kabir,
            sagheer,
            tasheeh,
            khaam,
            mushtaqq: false,
        })
    }
}

// ---------------------------------------------------------------------------
// The exported layout
//
// A Godot export is small and flat, which is why the survey below is one
// breadth-first walk and a handful of targeted joins:
//
//   <root>/Game.exe                     the engine, with or without the package
//   <root>/Game.pck                     the package, when it was not embedded
//   <root>/Game.console.exe             Godot 4's console wrapper
//   <root>/data_Game_windows_x86_64/    C#: assemblies (4.x) or Mono/ (3.x)
//   <root>/*.gdextension                Godot 4 native extensions
//   <root>/*.gdnlib, *.gdns             Godot 3 GDNative
//   <root>/Game.app/Contents/MacOS/Game
//   <root>/Game.app/Contents/Resources/Game.pck
//
// The macOS bundle is the one shape the depth limit would miss — the package
// sits four levels down — so it is reached by naming the path rather than by
// walking to it.
// ---------------------------------------------------------------------------

/// Everything the directory layout yielded, before any of it becomes evidence.
#[derive(Debug, Default)]
struct BinyaGodot {
    /// `.pck` candidates, capped at [`AQSA_HUZAM`].
    huzam: Vec<PathBuf>,
    /// Executables that might carry an embedded package, capped at
    /// [`AQSA_TANFIDHIYAT`].
    tanfidhiyat: Vec<PathBuf>,
    /// A `project.godot`, when the game shipped its resources loose.
    mashru: Option<PathBuf>,
    /// `.gdextension` files, which exist only in Godot 4.
    gdextension: Vec<String>,
    /// `.gdnlib` and `.gdns` files, which exist only in Godot 3.
    gdnative: Vec<String>,
    /// A `data_*/Mono` directory: Godot 3 with C#.
    mono: Option<PathBuf>,
    /// A .NET assembly layout: Godot 4 with C#.
    dotnet: Option<PathBuf>,
    /// A `*.console.exe` wrapper.
    console: Option<String>,
    /// A `.godot` or `.import` cache directory left in the export.
    makhbaa: Option<String>,
    /// A `_sc_` or `._sc_` self-contained-mode marker.
    sc: bool,
    /// Loose scene and resource files, capped at [`AQSA_MASHAHID`].
    mawarid: Vec<PathBuf>,
}

/// Whether a directory entry is a plausible game executable.
///
/// Windows and Godot's Linux export templates carry an extension; a plain Linux
/// or macOS binary carries none, and an extension-less file at the top of a game
/// directory is nearly always the game.
fn yashbah_tanfidh(madkhal: &Madkhal) -> bool {
    if madkhal.mujallad {
        return false;
    }
    match madkhal.masar.extension().and_then(std::ffi::OsStr::to_str) {
        Some(imtidad) => IMTIDADAT_TANFIDH
            .iter()
            .any(|maruf| maruf.eq_ignore_ascii_case(imtidad)),
        None => true,
    }
}

/// Walks the export once and reports what it found.
fn tafahhus_binya(siyaq: &SiyaqFahs<'_>, mizaniya: &mut Mizaniya) -> BinyaGodot {
    let mut binya = BinyaGodot::default();
    let jidhr = siyaq.jidhr;

    // The executable discovery already named goes first: it is the one the
    // launcher actually starts, so it is the one most likely to carry the
    // package.
    if let Some(tanfidhi) = siyaq.tanfidhi
        && tanfidhi.is_file()
    {
        binya.tanfidhiyat.push(tanfidhi.to_path_buf());
    }

    let judhur = mizaniya.madakhil(jidhr);
    for madkhal in &judhur {
        if binya.tanfidhiyat.len() < AQSA_TANFIDHIYAT
            && yashbah_tanfidh(madkhal)
            && !binya.tanfidhiyat.contains(&madkhal.masar)
        {
            binya.tanfidhiyat.push(madkhal.masar.clone());
        }
        if madkhal.mujallad && imtidad(&madkhal.ism, "app") {
            hazmat_mac(&madkhal.masar, mizaniya, &mut binya);
        }
        if madkhal.mujallad && madkhal.ism.starts_with("data_") {
            tasnif_dotnet(&madkhal.masar, mizaniya, &mut binya);
        }
    }

    for madkhal in mizaniya.shajara(jidhr) {
        tasnif_madkhal(&madkhal, &mut binya);
    }
    binya
}

/// Classifies one entry from the walk.
fn tasnif_madkhal(madkhal: &Madkhal, binya: &mut BinyaGodot) {
    if madkhal.mujallad {
        if binya.makhbaa.is_none() && (madkhal.ism == ".godot" || madkhal.ism == ".import") {
            binya.makhbaa = Some(madkhal.ism.clone());
        }
        if binya.dotnet.is_none() && madkhal.ism == "GodotSharp" {
            binya.dotnet = Some(madkhal.masar.clone());
        }
        return;
    }

    if madkhal.ism == "project.godot" {
        if binya.mashru.is_none() {
            binya.mashru = Some(madkhal.masar.clone());
        }
        return;
    }
    if madkhal.ism == "_sc_" || madkhal.ism == "._sc_" {
        binya.sc = true;
        return;
    }
    if madkhal.ism.ends_with(".console.exe") {
        if binya.console.is_none() {
            binya.console = Some(madkhal.ism.clone());
        }
        return;
    }
    if binya.dotnet.is_none()
        && (madkhal.ism.eq_ignore_ascii_case("GodotSharp.dll")
            || madkhal.ism.ends_with(".runtimeconfig.json"))
    {
        binya.dotnet = Some(madkhal.masar.clone());
        return;
    }
    if imtidad_huwa(&madkhal.masar, "pck") {
        if binya.huzam.len() < AQSA_HUZAM && !binya.huzam.contains(&madkhal.masar) {
            binya.huzam.push(madkhal.masar.clone());
        }
        return;
    }
    if imtidad_huwa(&madkhal.masar, "gdextension") {
        binya.gdextension.push(madkhal.ism.clone());
        return;
    }
    if imtidad_huwa(&madkhal.masar, "gdnlib") || imtidad_huwa(&madkhal.masar, "gdns") {
        binya.gdnative.push(madkhal.ism.clone());
        return;
    }
    if binya.mawarid.len() < AQSA_MASHAHID
        && IMTIDADAT_MAWRID
            .iter()
            .any(|imtidad| imtidad_huwa(&madkhal.masar, imtidad))
    {
        binya.mawarid.push(madkhal.masar.clone());
    }
}

/// Looks inside a `data_*` directory for the two C# layouts.
///
/// Godot 3 exported a Mono runtime into `data_<name>_<platform>/Mono/`; Godot 4
/// exports .NET assemblies straight into `data_<name>_<platform>_<arch>/`. The
/// two are as good a version discriminator as they are a backend one.
fn tasnif_dotnet(mujallad: &Path, mizaniya: &mut Mizaniya, binya: &mut BinyaGodot) {
    for madkhal in mizaniya.madakhil(mujallad) {
        if madkhal.mujallad {
            if madkhal.ism == "Mono" && binya.mono.is_none() {
                binya.mono = Some(madkhal.masar.clone());
            }
            if madkhal.ism == "GodotSharp" && binya.dotnet.is_none() {
                binya.dotnet = Some(madkhal.masar.clone());
            }
            continue;
        }
        if binya.dotnet.is_none()
            && (madkhal.ism.eq_ignore_ascii_case("GodotSharp.dll")
                || madkhal.ism.ends_with(".runtimeconfig.json"))
        {
            binya.dotnet = Some(madkhal.masar.clone());
        }
    }
}

/// Reaches into a macOS `.app` bundle for the two things that matter, by naming
/// the paths rather than walking to them.
fn hazmat_mac(hazma: &Path, mizaniya: &mut Mizaniya, binya: &mut BinyaGodot) {
    let muhtawayat = hazma.join("Contents");
    for madkhal in mizaniya.madakhil(&muhtawayat.join("Resources")) {
        if !madkhal.mujallad
            && imtidad_huwa(&madkhal.masar, "pck")
            && binya.huzam.len() < AQSA_HUZAM
        {
            binya.huzam.push(madkhal.masar);
        }
    }
    for madkhal in mizaniya.madakhil(&muhtawayat.join("MacOS")) {
        if !madkhal.mujallad && binya.tanfidhiyat.len() < AQSA_TANFIDHIYAT {
            binya.tanfidhiyat.push(madkhal.masar);
        }
    }
}

// ---------------------------------------------------------------------------
// project.godot
// ---------------------------------------------------------------------------

/// What a `project.godot` said about itself.
///
/// Only two lines out of it are read, and both are decisive:
/// `config_version=4` is the Godot 3 line and `=5` is the Godot 4 line, while
/// `config/features` carries the minor version as a literal string the exporter
/// wrote — `"4.3"`, `"3.5"` — which is a better version than any container can
/// give.
#[derive(Debug, Default, Clone)]
struct BayanMashru {
    /// The `config_version` value.
    sigha: Option<u32>,
    /// The first `major.minor` string found in `config/features`.
    isdar: Option<(u16, u16)>,
}

/// Reads the head of a `project.godot`.
///
/// Bounded to [`HAJM_TARWISA`] bytes and parsed as lossy UTF-8: the file is a
/// small INI-shaped text file whose first section carries everything wanted
/// here, and a project that put a megabyte of comments above it gets read as far
/// as the limit and no further.
fn iqra_mashru(masar: &Path) -> Option<BayanMashru> {
    let (bayt, _) = iqra_rass(masar, HAJM_TARWISA)?;
    let nass = String::from_utf8_lossy(&bayt);
    let mut bayan = BayanMashru::default();
    for satr in nass.lines() {
        let satr = satr.trim();
        if let Some(qeema) = satr.strip_prefix("config_version=") {
            bayan.sigha = qeema.trim().parse::<u32>().ok();
        } else if satr.starts_with("config/features=") && bayan.isdar.is_none() {
            bayan.isdar = isdar_min_simat(satr);
        }
    }
    if bayan.sigha.is_none() && bayan.isdar.is_none() {
        None
    } else {
        Some(bayan)
    }
}

/// Pulls the first `major.minor` out of a `config/features` line.
///
/// The line is `config/features=PackedStringArray("4.3", "Forward Plus")` in
/// Godot 4 and `config/features=PoolStringArray( "3.5" )` in Godot 3, so the
/// parse is the same for both: take quoted tokens, keep the first that looks
/// like a version.
fn isdar_min_simat(satr: &str) -> Option<(u16, u16)> {
    for juz in satr.split('"').skip(1).step_by(2) {
        let mut aqsam = juz.split('.');
        let Some(kabir) = aqsam
            .next()
            .and_then(|raqm| raqm.trim().parse::<u16>().ok())
        else {
            continue;
        };
        let Some(sagheer) = aqsam
            .next()
            .and_then(|raqm| raqm.trim().parse::<u16>().ok())
        else {
            continue;
        };
        if (2..=9).contains(&kabir) && sagheer <= 99 {
            return Some((kabir, sagheer));
        }
    }
    None
}

/// Which generation a `config_version` names.
///
/// Godot 3 writes 4 and Godot 4 writes 5. The off-by-one is Godot's, not a typo
/// here: the project file's own schema version has never matched the engine's.
const fn jeel_min_sigha_mashru(sigha: u32) -> Option<JeelGodot> {
    match sigha {
        4 => Some(JeelGodot::Thalith),
        5..=16 => Some(JeelGodot::Rabi),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The detector
// ---------------------------------------------------------------------------

/// The Godot detector.
///
/// Holds no state and is safe to run against anything. Its one job that cannot
/// be got wrong is telling Godot 3 from Godot 4, and every path through it
/// either produces that answer with the evidence behind it or produces no answer
/// at all.
#[derive(Debug, Default, Clone, Copy)]
pub struct FahisGodot;

impl FahisGodot {
    /// A detector.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// Finds the game's package and reads its header.
    ///
    /// Phase 9 calls this to know what it is mounting over: the path is where
    /// the original package lives, and [`TarwisatPck::mudmaja`] says whether it
    /// is a file of its own or bytes at the end of the executable — which
    /// decides nothing about the patch, since
    /// `ProjectSettings.load_resource_pack` mounts a separate file either way,
    /// but decides everything about reading the original.
    #[must_use]
    pub fn hazma(jidhr: &Path, tanfidhi: Option<&Path>) -> Option<(PathBuf, TarwisatPck)> {
        let siyaq_masar = tanfidhi.filter(|masar| masar.is_file());
        let mut mizaniya = Mizaniya::jadeeda();
        let mut binya = BinyaGodot::default();
        if let Some(masar) = siyaq_masar {
            binya.tanfidhiyat.push(masar.to_path_buf());
        }
        for madkhal in mizaniya.shajara(jidhr) {
            tasnif_madkhal(&madkhal, &mut binya);
            if binya.tanfidhiyat.len() < AQSA_TANFIDHIYAT && yashbah_tanfidh(&madkhal) {
                binya.tanfidhiyat.push(madkhal.masar);
            }
        }
        for masar in &binya.huzam {
            if let Some(tarwisa) = TarwisatPck::min_rass(masar) {
                return Some((masar.clone(), tarwisa));
            }
        }
        for masar in &binya.tanfidhiyat {
            if let Some(tarwisa) = TarwisatPck::min_mudmaj(masar) {
                return Some((masar.clone(), tarwisa));
            }
        }
        None
    }
}

impl Fahis for FahisGodot {
    fn ism(&self) -> &'static str {
        "godot"
    }

    /// # Errors
    ///
    /// Returns [`KhataMuharrik::JidhrMafqud`] when the game directory is not
    /// there. Nothing else is an error: a `.pck` that will not parse, a trailer
    /// that leads nowhere, a `project.godot` in an encoding this build cannot
    /// read — each of those is the absence of evidence, and what was seen is
    /// still reported.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        if !siyaq.jidhr.is_dir() {
            return Err(KhataMuharrik::JidhrMafqud {
                jidhr: siyaq.jidhr.to_path_buf(),
            }
            .into());
        }

        let mut hasila = HasilatFahs::la_shay();
        let mut mizaniya = Mizaniya::jadeeda();
        let binya = tafahhus_binya(siyaq, &mut mizaniya);

        let hazma = qira_huzam(siyaq.jidhr, &binya, &mut hasila);
        if let Some((masar, _)) = hazma.as_ref()
            && binya.huzam.iter().all(|mawjud| mawjud != masar)
        {
            // The package was embedded, so the file it came out of is the game's
            // real executable — worth reporting when discovery named something
            // else, such as a store's own launcher shim.
            hasila.tanfidhi = Some(masar.clone());
        }

        let bayan = binya.mashru.as_deref().and_then(iqra_mashru);
        let shawahid = dalail_binya(siyaq.jidhr, &binya, bayan.as_ref(), &mut hasila);

        let tarwisa = hazma.as_ref().map(|(_, tarwisa)| tarwisa);
        let jeel = hasm_jeel(&binya, tarwisa, bayan.as_ref(), &mut hasila);
        istinbat_isdar(tarwisa, bayan.as_ref(), &mut hasila);

        // The backend and the text systems are named only once the family is
        // settled. Naming them off weak evidence would put Godot's text systems
        // into the capability report of a game that merely shipped a file with
        // a familiar extension.
        if hasila.aila == Some(AilatMuharrik::Godot) {
            hasila.khalfiya = Some(khalfiya(&binya, hazma.is_some(), &mut hasila));
            anzimat_nusus(jeel, shawahid, &mut hasila);
        }

        Ok(hasila)
    }
}

/// Reads package headers: loose files first, then the trailer of every
/// executable candidate.
///
/// The order is deliberate and is the same order the engine itself uses. It also
/// keeps the diagnosis right: "no `.pck` beside the executable" is far more
/// often an embedded package than a native build, and calling it a native build
/// before checking would send Phase 9 down the hook path for a game that would
/// have taken a resource pack happily.
fn qira_huzam(
    jidhr: &Path,
    binya: &BinyaGodot,
    hasila: &mut HasilatFahs,
) -> Option<(PathBuf, TarwisatPck)> {
    for masar in binya.huzam.iter().take(AQSA_HUZAM) {
        let Some(tarwisa) = TarwisatPck::min_rass(masar) else {
            continue;
        };
        sajjil_hazma(jidhr, masar, &tarwisa, hasila);
        return Some((masar.clone(), tarwisa));
    }

    if !binya.huzam.is_empty() {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "{} file(s) with a .pck extension, none beginning with GDPC; the extension is \
                 used by several unrelated products and on its own says almost nothing",
                binya.huzam.len()
            ),
            None,
            25,
        );
    }

    for masar in binya.tanfidhiyat.iter().take(AQSA_TANFIDHIYAT) {
        let Some(tarwisa) = TarwisatPck::min_mudmaj(masar) else {
            continue;
        };
        sajjil_hazma(jidhr, masar, &tarwisa, hasila);
        return Some((masar.clone(), tarwisa));
    }
    None
}

/// Records one validated package header.
fn sajjil_hazma(jidhr: &Path, masar: &Path, tarwisa: &TarwisatPck, hasila: &mut HasilatFahs) {
    let mawqi = nisbi(jidhr, masar);
    hasila.sajjil_aila(
        AilatMuharrik::Godot,
        NawDaleel::TarwisatHawiya,
        format!(
            "GDPC package header{}: pack format {}, engine {}.{}.{}, {} entries",
            if tarwisa.mudmaja {
                format!(
                    ", embedded in the executable and reached through its 12-byte trailer at \
                     offset {}",
                    tarwisa.bidaya
                )
            } else {
                String::new()
            },
            tarwisa.sigha,
            tarwisa.kabir,
            tarwisa.sagheer,
            tarwisa.tasheeh,
            tarwisa.adad_malaffat
        ),
        mawqi.clone(),
        95,
    );

    if tarwisa.tanaqud() {
        hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            format!(
                "the package's pack format ({}) and its engine major ({}) disagree about which \
                 Godot wrote it; the engine major is taken as authoritative and the conflict is \
                 recorded rather than smoothed over",
                tarwisa.sigha, tarwisa.kabir
            ),
            mawqi.clone(),
            35,
        );
    }

    match tarwisa.mushaffara() {
        Some(true) => hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            "the package directory is encrypted (PACK_DIR_ENCRYPTED); nothing inside it can be \
             listed or translated until the user supplies the export key",
            mawqi,
            30,
        ),
        Some(false) => {},
        None => hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            "pack format 1 has no flags word, so whether this package's entries are encrypted \
             is unknown rather than known to be no; Godot 3 encrypts entries individually and \
             those flags live in the index",
            mawqi,
            10,
        ),
    }
}

/// What the loose resource files, if there were any, showed.
#[derive(Debug, Default, Clone, Copy)]
struct ShawahidNusus {
    /// A plain `Label` was seen by name.
    label: bool,
    /// A `RichTextLabel` was seen by name.
    ghani: bool,
    /// How many resource files were opened.
    adad: usize,
}

/// Records everything the layout said, and reads any loose scenes for node
/// types.
fn dalail_binya(
    jidhr: &Path,
    binya: &BinyaGodot,
    bayan: Option<&BayanMashru>,
    hasila: &mut HasilatFahs,
) -> ShawahidNusus {
    if let Some(masar) = binya.mashru.as_ref()
        && let Some(bayan) = bayan
    {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BayanatMudmaja,
            format!(
                "project.godot with config_version={} and features {}",
                bayan
                    .sigha
                    .map_or_else(|| "absent".to_owned(), |sigha| sigha.to_string()),
                bayan.isdar.map_or_else(
                    || "absent".to_owned(),
                    |(kabir, sagheer)| format!("naming {kabir}.{sagheer}")
                )
            ),
            nisbi(jidhr, masar),
            85,
        );
    }
    if !binya.gdextension.is_empty() {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            format!(
                "{} .gdextension file(s): {}. GDExtension is Godot 4's native interface and did \
                 not exist in Godot 3",
                binya.gdextension.len(),
                binya.gdextension.join(", ")
            ),
            None,
            80,
        );
    }
    if !binya.gdnative.is_empty() {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            format!(
                "{} GDNative file(s): {}. GDNative is Godot 3's native interface and was \
                 replaced by GDExtension in Godot 4",
                binya.gdnative.len(),
                binya.gdnative.join(", ")
            ),
            None,
            75,
        );
    }
    if let Some(masar) = binya.mono.as_ref() {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            "a data_*/Mono directory: the runtime Godot 3 shipped for C# projects",
            nisbi(jidhr, masar),
            70,
        );
    }
    if let Some(masar) = binya.dotnet.as_ref() {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            "a .NET assembly layout (GodotSharp / runtimeconfig): Godot 4's C# export",
            nisbi(jidhr, masar),
            70,
        );
    }
    if let Some(ism) = binya.console.as_ref() {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            format!("{ism}: the console wrapper Godot 4 generates beside the game executable"),
            None,
            45,
        );
    }
    if let Some(ism) = binya.makhbaa.as_ref() {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            format!(
                "a {ism} cache directory, which belongs to a Godot project rather than to an \
                 export and was most likely shipped by accident"
            ),
            None,
            40,
        );
    }
    if binya.sc {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BinyatMujallad,
            "a _sc_ marker, which puts a Godot binary into self-contained mode",
            None,
            20,
        );
    }
    masah_mawarid(jidhr, binya, hasila)
}

/// Opens up to [`AQSA_MASHAHID`] loose scene or resource files and looks for
/// node type names.
///
/// Both of Godot's resource formats store a node's type as a plain ASCII string
/// — the text form writes `type="RichTextLabel"`, the binary form writes the
/// same characters into its string table — so one byte search covers both, and
/// neither format has to be parsed to answer the only question being asked.
fn masah_mawarid(jidhr: &Path, binya: &BinyaGodot, hasila: &mut HasilatFahs) -> ShawahidNusus {
    let mut shawahid = ShawahidNusus::default();
    let mut mawrid_maruf: Option<String> = None;
    for masar in binya.mawarid.iter().take(AQSA_MASHAHID) {
        let Some((bayt, _)) = iqra_rass(masar, HAJM_TARWISA) else {
            continue;
        };
        shawahid.adad = shawahid.adad.saturating_add(1);
        if mawrid_maruf.is_none()
            && (yahtawi(&bayt, b"[gd_scene") || yahtawi(&bayt, b"[gd_resource"))
        {
            mawrid_maruf = nisbi(jidhr, masar);
        }
        if yahtawi(&bayt, IBRAT_NASS_GHANI) {
            shawahid.ghani = true;
        } else if yahtawi(&bayt, IBRAT_LABEL) {
            shawahid.label = true;
        }
    }
    if let Some(mawqi) = mawrid_maruf {
        hasila.sajjil_aila(
            AilatMuharrik::Godot,
            NawDaleel::BayanatMudmaja,
            "a loose resource file carrying Godot's own gd_scene/gd_resource header",
            Some(mawqi),
            80,
        );
    }
    shawahid
}

/// Which major version a bare number names.
const fn jeel_min_kabir(kabir: u16) -> Option<JeelGodot> {
    match kabir {
        3 => Some(JeelGodot::Thalith),
        4..=9 => Some(JeelGodot::Rabi),
        _ => None,
    }
}

/// Decides Godot 3 against Godot 4, and says what decided it.
///
/// Every source that has an opinion contributes a candidate with the weight its
/// evidence deserves; the heaviest wins. When two sources of comparable weight
/// disagree — a Godot 3 package next to a `.gdextension`, which is what a game
/// shipping two engines' leftovers looks like — the disagreement is recorded as
/// its own piece of evidence, because that is a game somebody has to look at by
/// hand rather than one to guess about.
fn hasm_jeel(
    binya: &BinyaGodot,
    tarwisa: Option<&TarwisatPck>,
    bayan: Option<&BayanMashru>,
    hasila: &mut HasilatFahs,
) -> Option<JeelGodot> {
    let mut murashahat: Vec<(JeelGodot, NawDaleel, String, u8)> = Vec::new();
    if let Some(tarwisa) = tarwisa
        && let Some(jeel) = tarwisa.jeel()
    {
        murashahat.push((
            jeel,
            NawDaleel::TarwisatHawiya,
            format!(
                "the package header's engine major field is {}",
                tarwisa.kabir
            ),
            90,
        ));
    }
    if let Some(bayan) = bayan {
        if let Some(sigha) = bayan.sigha
            && let Some(jeel) = jeel_min_sigha_mashru(sigha)
        {
            murashahat.push((
                jeel,
                NawDaleel::BayanatMudmaja,
                format!("project.godot declares config_version={sigha}"),
                85,
            ));
        }
        if let Some((kabir, sagheer)) = bayan.isdar
            && let Some(jeel) = jeel_min_kabir(kabir)
        {
            murashahat.push((
                jeel,
                NawDaleel::BayanatMudmaja,
                format!("project.godot's config/features names {kabir}.{sagheer}"),
                85,
            ));
        }
    }
    if !binya.gdextension.is_empty() {
        murashahat.push((
            JeelGodot::Rabi,
            NawDaleel::BinyatMujallad,
            "a .gdextension file, which exists only in Godot 4".to_owned(),
            80,
        ));
    }
    if !binya.gdnative.is_empty() {
        murashahat.push((
            JeelGodot::Thalith,
            NawDaleel::BinyatMujallad,
            "a GDNative library definition, which exists only in Godot 3".to_owned(),
            75,
        ));
    }
    if binya.mono.is_some() {
        murashahat.push((
            JeelGodot::Thalith,
            NawDaleel::BinyatMujallad,
            "a Mono runtime directory, which is how Godot 3 shipped C#".to_owned(),
            70,
        ));
    }
    if binya.dotnet.is_some() {
        murashahat.push((
            JeelGodot::Rabi,
            NawDaleel::BinyatMujallad,
            "a .NET assembly layout, which is how Godot 4 ships C#".to_owned(),
            70,
        ));
    }
    if binya.console.is_some() {
        murashahat.push((
            JeelGodot::Rabi,
            NawDaleel::BinyatMujallad,
            "a .console.exe wrapper, which Godot 4 generates and Godot 3 did not".to_owned(),
            45,
        ));
    }

    murashahat.sort_by_key(|murashah| Reverse(murashah.3));
    let (jeel, naw, sabab, wazn) = murashahat.first()?.clone();
    if let Some((mukhalif, _, sabab_mukhalif, wazn_mukhalif)) =
        murashahat.iter().find(|murashah| murashah.0 != jeel)
    {
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            format!(
                "sources disagree about the generation: {sabab} says {} at weight {wazn}, while \
                 {sabab_mukhalif} says {} at weight {wazn_mukhalif}. The heavier source is taken \
                 and the conflict is reported, not averaged",
                jeel.ism(),
                mukhalif.ism()
            ),
            None,
            40,
        );
    }
    hasila.sajjil(
        naw,
        format!("{sabab}, so this is {}. {}", jeel.ism(), jeel.wasf()),
        None,
        wazn,
    );
    Some(jeel)
}

/// Sets the version from the package, or from `project.godot` when there is no
/// package to read.
fn istinbat_isdar(
    tarwisa: Option<&TarwisatPck>,
    bayan: Option<&BayanMashru>,
    hasila: &mut HasilatFahs,
) {
    if let Some(tarwisa) = tarwisa
        && let Some(isdar) = tarwisa.isdar()
    {
        hasila.isdar = Some(isdar);
        return;
    }
    if let Some((kabir, sagheer)) = bayan.and_then(|bayan| bayan.isdar) {
        hasila.isdar = Some(IsdarMuharrik {
            kabir,
            sagheer,
            tasheeh: 0,
            khaam: format!("{kabir}.{sagheer} (project.godot config/features)"),
            mushtaqq: false,
        });
    }
}

/// Decides the scripting backend and records how honest that decision is.
fn khalfiya(
    binya: &BinyaGodot,
    ladayha_hazma: bool,
    hasila: &mut HasilatFahs,
) -> KhalfiyaBarmajiya {
    if binya.dotnet.is_some() || binya.mono.is_some() {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            if binya.mono.is_some() {
                "the project ships a Mono runtime, so its scripts are C# on Godot 3's Mono build"
            } else {
                "the project ships .NET assemblies, so its scripts are C# on Godot 4's .NET build"
            },
            None,
            30,
        );
        return KhalfiyaBarmajiya::GodotCSharp;
    }
    if !ladayha_hazma {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "no package was found: not a loose .pck, and no embedded package at the end of any \
             executable here. What is left is a custom engine build with the project compiled \
             in, which cannot load a resource pack and has to be hooked instead",
            None,
            30,
        );
        return KhalfiyaBarmajiya::GodotNative;
    }
    hasila.sajjil(
        NawDaleel::BinyatMujallad,
        "no .NET runtime and no native project build, so the scripts are GDScript. This is an \
         inference from absence rather than an observation: GDScript compiles into the package \
         and leaves nothing outside it to see",
        None,
        20,
    );
    KhalfiyaBarmajiya::GdScript
}

/// Names the text systems, each with the evidence that supports it.
fn anzimat_nusus(jeel: Option<JeelGodot>, shawahid: ShawahidNusus, hasila: &mut HasilatFahs) {
    hasila.daa_itar(ItarNusus::GodotLabel);
    hasila.sajjil(
        NawDaleel::BinyatMujallad,
        match jeel {
            Some(jeel) => format!(
                "Label is {}'s base text control and is compiled into every export template, so \
                 it is reachable here. Which nodes the game actually uses lives inside the \
                 package, which a probe does not open",
                jeel.ism()
            ),
            None => "Label is Godot's base text control and is compiled into every export \
                     template, so it is reachable here"
                .to_owned(),
        },
        None,
        20,
    );

    if shawahid.ghani {
        hasila.daa_itar(ItarNusus::GodotRichText);
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            "a RichTextLabel was found by name in a loose resource file, so the game has BBCode \
             text and the patch pipeline needs the markup dialect that comes with it",
            None,
            55,
        );
        return;
    }
    hasila.sajjil(
        NawDaleel::BinyatMujallad,
        if shawahid.adad == 0 {
            "no loose scene or resource file was readable, so no RichTextLabel could be observed \
             and none is claimed; Phase 12 finds the real node types when it opens the package"
                .to_owned()
        } else {
            format!(
                "{} loose resource file(s) were read and none named a RichTextLabel{}; the \
                 rich-text surface is therefore not claimed",
                shawahid.adad,
                if shawahid.label {
                    ", though a plain Label was named"
                } else {
                    ""
                }
            )
        },
        None,
        5,
    );
}
