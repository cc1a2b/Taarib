//! آر بي جي ميكر — MV and MZ, patched as the JSON and JavaScript they are.
//!
//! ## Read the rung before reading anything else
//!
//! The reflex about RPG Maker is that it cannot shape Arabic. That reflex is
//! half wrong, and acting on the wrong half breaks a game that was already
//! working.
//!
//! MV runs on NW.js and MZ runs on NW.js or a plain browser. Both draw through
//! `Bitmap.prototype.drawText`, which is a thin wrapper over the canvas
//! `fillText`. Canvas `fillText` in Chromium goes through the same `HarfBuzz` the
//! layout engine uses, so `drawText("مرحبا")` **joins correctly today, with no
//! patch at all**. Every command window, every item name, every menu label in
//! every RPG Maker game already shapes.
//!
//! What does not shape is the message window. `Window_Base.processCharacter`
//! pulls one character off `textState` per call and hands that single character
//! to `drawText`, because that is how the typewriter reveal works. A shaper
//! given one character at a time has no neighbours to join to, so the dialogue
//! — the overwhelming majority of a role-playing game's words — comes out as
//! twenty-eight disconnected letterforms while the menus beside it are perfect.
//!
//! So the rung is genuinely a question, it is genuinely per-game, and
//! [`MifhasRpgMaker`] answers it from the shipped files rather than from the
//! engine's name: which Chromium generation the shell carries, whether the core
//! script's own text chain is the stock per-character one, and whether an
//! enabled plugin has already replaced that chain. A game whose message chain
//! has been replaced by a plugin that draws whole strings needs direction and
//! alignment corrected and nothing else — [`Rutba::Tasheeh`] — and taking it
//! over would be a regression. A stock game needs the takeover for its message
//! window. Neither answer is hard-coded here; [`hukm`] weighs the evidence and
//! refuses when it is too close to call.
//!
//! ## Byte-faithful round-trip, and why it is not a `serde_json` round-trip
//!
//! The engine's data files are written by the editor and read by the editor
//! again. Their key order is the editor's, their number formatting is
//! JavaScript's, and their whitespace is `JSON.stringify` with no indent. A
//! `serde_json::from_str` followed by a `to_string` preserves none of that
//! reliably — key order only with `preserve_order`, number spelling not at all
//! (`1e+21` becomes `1e21`, `-0` becomes `0`), and escape choice not at all
//! (`é` becomes the raw character).
//!
//! So this module **never re-serializes a game file**. [`imsah_nusus`] walks
//! the raw text and records the byte span of every string literal; [`khayyit`]
//! splices replacements into those spans and copies every other byte through
//! untouched. Everything not translated is therefore byte-identical by
//! construction rather than by luck, including the byte-order mark, the trailing
//! newline, and whatever the editor did with `\/`. [`tahaqquq_dawra`] proves it
//! per file by rebuilding the original from the patched bytes and comparing, and
//! raises [`KhataNusus::DawraGhayrMutabaqa`] with the count of differing bytes
//! before anything reaches the game's directory.
//!
//! ## The asset "encryption" needs no cipher
//!
//! MV and MZ optionally obfuscate `img/` and `audio/`. The scheme is a sixteen
//! byte header followed by the file's own first sixteen bytes `XORed` with a
//! sixteen byte key that `System.json` publishes in plain hexadecimal. It is not
//! AES, not any block cipher, and this module adds no cryptographic dependency
//! to do it — see [`fukk_tashfeer`]. When `System.json` says the game is not
//! obfuscated, [`TashfeerMawarid::mushaffar`] is false and nothing is touched.
//!
//! ## Every write goes through the guard
//!
//! Nothing in this module calls `std::fs::write`. Writes go through [`Hafiz`],
//! which preserves the original byte-exact before the first modification and
//! writes the replacement atomically. See that trait's own note about where it
//! is expected to live.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde_json::Value;
use taarib_muharrik::dalail::nusus::HadafNusus;
use taarib_mustalahat::muharrik::AilatMuharrik;

use crate::hifz::Hafiz;
use crate::khata::{KhataNusus, tul_u64};
use crate::tabaqa::{Dalil, Hukm, Mifhas, Rutba, SiyaqTabaqa, hukm};
use crate::tarkeeb::masar_bila_hala;

/// The format name every container error in this module reports.
const SIGHA: &str = "RPG Maker JSON";

/// The most bytes this module will read from one of the engine's data files.
///
/// A `Map001.json` on a large commercial project runs to a few megabytes; the
/// largest thing here is a `CommonEvents.json` on a project with a thousand
/// common events, which is still well under twenty. Sixty-four is far above
/// anything real and far below anything that would let a corrupt or hostile
/// file decide how much memory this process takes.
pub const AQSA_MALAF: u64 = 64 * 1024 * 1024;

/// The most string literals one file may contribute before the walk stops.
///
/// A map with three thousand events reaches perhaps four hundred thousand
/// literals. Two million is the point past which the file is not a map.
pub const AQSA_NUSUS: usize = 2_000_000;

/// How deep the JSON scanner will nest before refusing.
///
/// The engine's own files nest about eight deep — map, events, pages, list,
/// parameters, choice array. Sixty-four leaves room for anything an editor
/// plugin produces and stops a file crafted to recurse this scanner forever.
pub const AQSA_UMQ: usize = 64;

/// The fixed header an obfuscated MV or MZ asset begins with.
///
/// `RPGMV` in ASCII, then zeros, then the scheme's own version triple. MZ kept
/// the same header rather than minting a new one, which is why one constant
/// serves both.
pub const TARWISAT_TASHFEER: [u8; 16] = [
    0x52, 0x50, 0x47, 0x4D, 0x56, 0x00, 0x00, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// How many bytes of the payload the key is applied to.
///
/// Sixteen, once, at the front. The rest of the file is stored verbatim, which
/// is why a "decryption key" recovered from `System.json` is a masking key and
/// this module says so rather than calling it encryption.
pub const TUL_QINAA: usize = 16;

/// Which product wrote the project.
///
/// Not a cosmetic distinction. MV ships an ES5 runtime on a Chromium from 2016
/// and registers plugins from `js/plugins.js` loaded as a classic script; MZ
/// ships a modern runtime and a different core script set. Sending one's payload
/// at the other produces a game that will not boot, so this is read from disk
/// and never inferred from a version number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsdarRpg {
    /// RPG Maker MV — `js/rpg_*.js`.
    Mv,
    /// RPG Maker MZ — `js/rmmz_*.js`.
    Mz,
}

impl IsdarRpg {
    /// The product's name, as a log line and the capability report write it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mv => "RPG Maker MV",
            Self::Mz => "RPG Maker MZ",
        }
    }

    /// The engine family this product belongs to, for the probe's verdict.
    #[must_use]
    pub const fn aila(self) -> AilatMuharrik {
        match self {
            Self::Mv => AilatMuharrik::RpgMakerMv,
            Self::Mz => AilatMuharrik::RpgMakerMz,
        }
    }

    /// The core script that holds the window text chain, relative to the
    /// project base.
    ///
    /// The one file worth reading to answer the rung question: it defines
    /// `Window_Base.prototype.processCharacter` and its neighbours, which is
    /// where the per-character drawing that defeats the browser's shaping
    /// lives.
    #[must_use]
    pub const fn nawat_nawafidh(self) -> &'static str {
        match self {
            Self::Mv => "js/rpg_windows.js",
            Self::Mz => "js/rmmz_windows.js",
        }
    }

    /// The core script that defines `Bitmap.prototype.drawText`.
    #[must_use]
    pub const fn nawat_asas(self) -> &'static str {
        match self {
            Self::Mv => "js/rpg_core.js",
            Self::Mz => "js/rmmz_core.js",
        }
    }
}

impl fmt::Display for IsdarRpg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.ism())
    }
}

/// Where an RPG Maker project can sit relative to the directory a user pointed
/// at.
///
/// `www/` is MV's deployment layout and what most MV games on a store look
/// like. The empty prefix is MZ's deployment layout and is also what an
/// undeployed MV *project folder* looks like, which is what a great many itch.io
/// uploads are. The two `resources/app` entries are the same project repackaged
/// into an Electron shell.
const QAWAID: [&str; 4] = ["", "www", "resources/app", "resources/app/www"];

/// A located project: which product, where its directories are, and whether it
/// was deployed.
#[derive(Debug, Clone)]
pub struct BunyatMashru {
    jidhr: PathBuf,
    asas: String,
    isdar: IsdarRpg,
    manshur: bool,
}

impl BunyatMashru {
    /// Finds the project under `jidhr` and decides which product wrote it.
    ///
    /// The product is read from which core script set is on disk, and confirmed
    /// against the runtime's own `Utils.RPGMAKER_NAME` where the script can be
    /// read. It is never inferred from a directory name: a repack renames
    /// directories all the time and cannot rewrite what the runtime declares
    /// about itself without breaking every plugin that reads it.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when none of the four layouts holds
    /// a core script set — which is the honest answer for a directory that is
    /// not an RPG Maker project, and is distinct from a project this build
    /// cannot read.
    pub fn iktashif(jidhr: &Path) -> Result<Self, KhataNusus> {
        let mut afdal: Option<Self> = None;
        for asas in QAWAID {
            let Some(isdar) = isdar_taht(jidhr, asas) else {
                continue;
            };
            let manshur = !yujad_malaf_mashru(&masar_asas(jidhr, asas));
            let murashah = Self {
                jidhr: jidhr.to_path_buf(),
                asas: asas.to_owned(),
                isdar,
                manshur,
            };
            // A deeper prefix wins: a project inside `resources/app/www` is a
            // repack whose outer directory also happens to hold a `js`, and the
            // inner one is the project the runtime actually loads.
            if afdal
                .as_ref()
                .is_none_or(|sabiq| murashah.asas.len() > sabiq.asas.len())
            {
                afdal = Some(murashah);
            }
        }
        afdal.ok_or_else(|| KhataNusus::BunyaGhayrMutawaqqaa {
            malaf: jidhr.display().to_string(),
            haql: "js/rpg_core.js or js/rmmz_core.js under any known project layout".to_owned(),
        })
    }

    /// The directory the user pointed at.
    #[must_use]
    pub fn jidhr(&self) -> &Path {
        &self.jidhr
    }

    /// The project prefix inside it — empty, `www`, or one of the repack
    /// layouts.
    #[must_use]
    pub fn asas(&self) -> &str {
        &self.asas
    }

    /// Which product wrote the project.
    #[must_use]
    pub const fn isdar(&self) -> IsdarRpg {
        self.isdar
    }

    /// Whether this is a deployed game rather than an editor project folder.
    ///
    /// Decided by the absence of a `.rpgproject` or `.rmmzproject` file, which
    /// the editor writes and deployment strips. It matters because an undeployed
    /// project is one the developer still opens in the editor, and a patch
    /// installed into one has to survive the editor rewriting every data file.
    #[must_use]
    pub const fn manshur(&self) -> bool {
        self.manshur
    }

    /// The absolute path of a project-relative path such as `data/System.json`.
    ///
    /// Every read and every write this module performs goes through here, so the
    /// case fold belongs here and nowhere else. A deployment that spells the
    /// directory `Data/` or `WWW/` — which Wine hides from the player entirely —
    /// would otherwise make [`istakhrij`] fail to list a directory that is right
    /// there, and [`rakkib`] write a `data/Map001.json` the engine never opens.
    ///
    /// The fold stops at the first component that is not on disk, so a file this
    /// module is about to *create* keeps the spelling asked for: `js/plugins/`
    /// resolves to the real `js/` and creates `plugins/` under it rather than
    /// beside it.
    #[must_use]
    pub fn malaf(&self, nisbi: &str) -> PathBuf {
        masar_bila_hala(&masar_asas(&self.jidhr, &self.asas), nisbi)
    }

    /// The `data/` directory, which holds every file this module extracts from.
    #[must_use]
    pub fn bayanat(&self) -> PathBuf {
        self.malaf("data")
    }

    /// The `js/` directory, which holds the core scripts and `plugins.js`.
    #[must_use]
    pub fn nusus(&self) -> PathBuf {
        self.malaf("js")
    }

    /// The `img/` directory, which asset obfuscation covers.
    #[must_use]
    pub fn suwar(&self) -> PathBuf {
        self.malaf("img")
    }

    /// The `audio/` directory, which asset obfuscation also covers.
    #[must_use]
    pub fn sawt(&self) -> PathBuf {
        self.malaf("audio")
    }
}

/// The absolute path of a project prefix.
///
/// `www` and `resources/app` are the exporter's spelling, not a guarantee: an
/// itch.io upload repacked on Windows routinely carries `WWW/`, and the four
/// layouts in [`QAWAID`] are the only thing standing between a game and "no RPG
/// Maker project here".
fn masar_asas(jidhr: &Path, asas: &str) -> PathBuf {
    masar_bila_hala(jidhr, asas)
}

/// Which product's core scripts are present under a prefix, if either is.
///
/// MZ is tested first: a project upgraded in place carries both script sets and
/// the runtime loads MZ's.
fn isdar_taht(jidhr: &Path, asas: &str) -> Option<IsdarRpg> {
    let asl = masar_asas(jidhr, asas);
    let mz = masar_bila_hala(&asl, "js/rmmz_core.js");
    let mv = masar_bila_hala(&asl, "js/rpg_core.js");
    if mz.is_file() {
        return Some(IsdarRpg::Mz);
    }
    if mv.is_file() {
        return Some(IsdarRpg::Mv);
    }
    None
}

/// Whether the editor's own project file sits beside the project.
fn yujad_malaf_mashru(asl: &Path) -> bool {
    let Ok(qaima) = fs::read_dir(asl) else {
        return false;
    };
    qaima.take(4_096).flatten().any(|madkhal| {
        let ism = madkhal.file_name().to_string_lossy().to_ascii_lowercase();
        ism.ends_with(".rpgproject") || ism.ends_with(".rmmzproject")
    })
}

// ---------------------------------------------------------------------------
// Bounded reading
// ---------------------------------------------------------------------------

/// Reads a game file whole, after checking its declared size against a ceiling.
///
/// The check is against the *metadata* length, before a byte is reserved: a
/// game directory is somebody else's data, and a length field there is a number
/// that decides how much memory this process is about to take.
///
/// # Errors
///
/// [`KhataNusus::HajmMufrit`] when the file is larger than `saqf`, and
/// [`KhataNusus::KhataMalaf`] when it cannot be opened or read.
pub fn iqra_malaf(masar: &Path, saqf: u64) -> Result<Vec<u8>, KhataNusus> {
    let bayanat = fs::metadata(masar).map_err(|sabab| KhataNusus::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    if bayanat.len() > saqf {
        return Err(KhataNusus::HajmMufrit {
            haql: "file length",
            qeema: bayanat.len(),
            saqf,
        });
    }
    fs::read(masar).map_err(|sabab| KhataNusus::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

/// Decodes a game file's bytes as UTF-8, tolerating a byte-order mark.
///
/// Returns the text and whether a mark was stripped, because the mark is part
/// of the file's bytes and has to be put back when the file is rewritten. The
/// editor writes these files without one and hand-editing on Windows adds one
/// often enough that dropping it silently would change a file this module
/// promises not to change.
///
/// # Errors
///
/// [`KhataNusus::NassGhayrSalih`] naming the first byte that is not valid
/// UTF-8. Refused rather than replaced: a lossy decode would put replacement
/// characters into a game's dialogue and hand them to a translator as text.
pub fn nass_min_bayt(bayt: &[u8], malaf: &str) -> Result<(String, bool), KhataNusus> {
    let (jism, alama) = match bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(baqi) => (baqi, true),
        None => (bayt, false),
    };
    match std::str::from_utf8(jism) {
        Ok(nass) => Ok((nass.to_owned(), alama)),
        Err(khata) => Err(KhataNusus::NassGhayrSalih {
            malaf: malaf.to_owned(),
            tarmiz: "UTF-8",
            mawqi: tul_u64(khata.valid_up_to()),
        }),
    }
}

// ---------------------------------------------------------------------------
// The raw JSON scanner
//
// Not a parser: nothing is built, nothing is re-serialized, and no value is
// converted. It walks the document's bytes once and records where every string
// literal is, so a later splice can replace those exact ranges and copy every
// other byte through. That is what makes the round-trip byte-faithful by
// construction rather than by hoping a serializer agrees with JavaScript about
// how to spell a number.
// ---------------------------------------------------------------------------

/// One step of a path into a JSON document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JuzMasar {
    /// An object member, by its key.
    Miftah(String),
    /// An array element, by its position.
    Fahras(usize),
}

impl fmt::Display for JuzMasar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Miftah(ism) => f.write_str(ism),
            Self::Fahras(fahras) => write!(f, "{fahras}"),
        }
    }
}

/// Renders a path the way every identity and every diagnostic in this module
/// writes it: slash-separated, rooted, and stable across runs.
#[must_use]
pub fn masar_nassi(masar: &[JuzMasar]) -> String {
    let mut makhtut = String::with_capacity(masar.len().saturating_mul(8));
    for juz in masar {
        makhtut.push('/');
        match juz {
            JuzMasar::Miftah(ism) => makhtut.push_str(ism),
            JuzMasar::Fahras(fahras) => makhtut.push_str(&fahras.to_string()),
        }
    }
    makhtut
}

/// One string literal, where it sits in the raw text and what it decodes to.
///
/// `bidaya` and `nihaya` bracket the literal *including* its quotation marks,
/// which is exactly the range a replacement literal takes the place of.
#[derive(Debug, Clone)]
pub struct MawqiNass {
    /// The path to this literal, from the document root.
    pub masar: Vec<JuzMasar>,
    /// Byte offset of the opening quotation mark.
    pub bidaya: usize,
    /// Byte offset one past the closing quotation mark.
    pub nihaya: usize,
    /// The literal's decoded value.
    pub qeema: String,
}

/// Records where every string *value* in a JSON document lives.
///
/// Object keys are walked into the path and deliberately not recorded: a key is
/// never translatable, and recording every one of them on a map file with three
/// hundred thousand literals would double the walk's memory for nothing.
///
/// # Errors
///
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] for a document that is not well-formed
/// JSON or nests past [`AQSA_UMQ`], [`KhataNusus::MalafQaseer`] for one that
/// ends inside a value, [`KhataNusus::HajmMufrit`] when it holds more than
/// [`AQSA_NUSUS`] string values, and [`KhataNusus::NassGhayrSalih`] for a
/// literal carrying an unpaired surrogate, which no Rust string can hold and
/// which is refused rather than replaced.
pub fn imsah_nusus(khaam: &str, malaf: &str) -> Result<Vec<MawqiNass>, KhataNusus> {
    let mut masih = Masih {
        bayt: khaam.as_bytes(),
        i: 0,
        malaf,
        masar: Vec::new(),
        mawaqi: Vec::new(),
    };
    masih.wathiqa()?;
    Ok(masih.mawaqi)
}

/// The scanner's working state.
struct Masih<'a> {
    bayt: &'a [u8],
    i: usize,
    malaf: &'a str,
    masar: Vec<JuzMasar>,
    mawaqi: Vec<MawqiNass>,
}

impl Masih<'_> {
    /// The byte at the cursor.
    fn hali(&self) -> Option<u8> {
        self.bayt.get(self.i).copied()
    }

    /// Advances the cursor by one byte.
    const fn taqaddam(&mut self) {
        self.i = self.i.saturating_add(1);
    }

    /// Skips the whitespace JSON allows between tokens.
    fn faragh(&mut self) {
        while matches!(self.hali(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.taqaddam();
        }
    }

    /// A malformed-document error naming what was expected where.
    fn bunya(&self, tawaqqu: &str) -> KhataNusus {
        KhataNusus::BunyaGhayrMutawaqqaa {
            malaf: self.malaf.to_owned(),
            haql: format!("byte {}: expected {tawaqqu}", self.i),
        }
    }

    /// The whole document: one value, then nothing but whitespace.
    fn wathiqa(&mut self) -> Result<(), KhataNusus> {
        self.qeem()?;
        self.faragh();
        if self.i < self.bayt.len() {
            return Err(self.bunya("end of document"));
        }
        Ok(())
    }
}

impl Masih<'_> {
    /// Walks one value and everything under it, iteratively.
    ///
    /// Iteratively and not recursively on purpose: the depth of this walk is
    /// decided by a file in somebody's game directory, and a recursive walker
    /// would answer a crafted file with a stack overflow, which is an abort
    /// this workspace's panic policy cannot catch.
    fn qeem(&mut self) -> Result<(), KhataNusus> {
        // One entry per open container: true for an object, false for an array.
        let mut kadas: Vec<bool> = Vec::new();
        loop {
            self.faragh();
            match self.hali() {
                None => return Err(self.naqis("a value")),
                Some(b'{') => {
                    self.iftah(&mut kadas, true)?;
                    self.faragh();
                    if self.hali() == Some(b'}') {
                        self.taqaddam();
                        self.aghliq(&mut kadas);
                    } else {
                        self.miftah()?;
                        continue;
                    }
                },
                Some(b'[') => {
                    self.iftah(&mut kadas, false)?;
                    self.faragh();
                    if self.hali() == Some(b']') {
                        self.taqaddam();
                        self.aghliq(&mut kadas);
                    } else {
                        continue;
                    }
                },
                Some(b'"') => {
                    let (qeema, bidaya, nihaya) = self.nass_harfi()?;
                    if self.mawaqi.len() >= AQSA_NUSUS {
                        return Err(KhataNusus::HajmMufrit {
                            haql: "string literals in one file",
                            qeema: tul_u64(self.mawaqi.len().saturating_add(1)),
                            saqf: tul_u64(AQSA_NUSUS),
                        });
                    }
                    self.mawaqi.push(MawqiNass {
                        masar: self.masar.clone(),
                        bidaya,
                        nihaya,
                        qeema,
                    });
                },
                Some(_) => self.qeema_basita()?,
            }

            // The value is complete. Close every container it finished, and
            // step to the next sibling of the first one it did not.
            loop {
                if kadas.is_empty() {
                    return Ok(());
                }
                self.faragh();
                let kaen = kadas.last().copied().unwrap_or(false);
                match self.hali() {
                    Some(b',') => {
                        self.taqaddam();
                        if kaen {
                            self.miftah()?;
                        } else {
                            self.tali_fahras();
                        }
                        break;
                    },
                    Some(b'}') if kaen => {
                        self.taqaddam();
                        self.aghliq(&mut kadas);
                    },
                    Some(b']') if !kaen => {
                        self.taqaddam();
                        self.aghliq(&mut kadas);
                    },
                    Some(_) => return Err(self.bunya("a comma or a closing bracket")),
                    None => return Err(self.naqis("a closing bracket")),
                }
            }
        }
    }

    /// Enters a container, pushing its path step and checking the depth bound.
    fn iftah(&mut self, kadas: &mut Vec<bool>, kaen: bool) -> Result<(), KhataNusus> {
        if kadas.len() >= AQSA_UMQ {
            return Err(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "nesting depth",
                qeema: tul_u64(kadas.len().saturating_add(1)),
                hadd: tul_u64(AQSA_UMQ),
            });
        }
        self.taqaddam();
        kadas.push(kaen);
        self.masar.push(if kaen {
            JuzMasar::Miftah(String::new())
        } else {
            JuzMasar::Fahras(0)
        });
        Ok(())
    }

    /// Leaves a container, popping its path step.
    fn aghliq(&mut self, kadas: &mut Vec<bool>) {
        let _ = kadas.pop();
        let _ = self.masar.pop();
    }

    /// Moves the innermost array's path step to the next element.
    fn tali_fahras(&mut self) {
        if let Some(JuzMasar::Fahras(fahras)) = self.masar.last_mut() {
            *fahras = fahras.saturating_add(1);
        }
    }

    /// Reads an object key and the colon after it, and installs the key as the
    /// innermost path step.
    fn miftah(&mut self) -> Result<(), KhataNusus> {
        self.faragh();
        if self.hali() != Some(b'"') {
            return Err(self.bunya("an object key"));
        }
        let (qeema, _, _) = self.nass_harfi()?;
        if let Some(juz) = self.masar.last_mut() {
            *juz = JuzMasar::Miftah(qeema);
        }
        self.faragh();
        if self.hali() != Some(b':') {
            return Err(self.bunya("a colon after an object key"));
        }
        self.taqaddam();
        Ok(())
    }

    /// A truncation error naming what the document ended before.
    fn naqis(&self, haql: &'static str) -> KhataNusus {
        KhataNusus::MalafQaseer {
            haql,
            tul: tul_u64(self.bayt.len()),
            matlub: tul_u64(self.i.saturating_add(1)),
        }
    }

    /// Skips a number, `true`, `false` or `null` without interpreting it.
    ///
    /// Deliberately not parsed. The engine's numbers are JavaScript's, this
    /// module never rewrites one, and converting a number here would be
    /// inventing a second spelling of a value the file already spells.
    fn qeema_basita(&mut self) -> Result<(), KhataNusus> {
        let bida = self.i;
        while matches!(
            self.hali(),
            Some(
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'
                    ..=b'9' | b't' | b'r' | b'u' | b'f' | b'a' | b'l' | b's' | b'n'
            )
        ) {
            self.taqaddam();
        }
        if self.i == bida {
            return Err(self.bunya("a number, true, false or null"));
        }
        Ok(())
    }
}

impl Masih<'_> {
    /// Reads one string literal, returning its value and its byte range.
    ///
    /// The cursor is on the opening quotation mark and ends one past the
    /// closing one, so the returned range is exactly what a replacement literal
    /// substitutes for. The path is deliberately not attached here: an object
    /// key goes through this function too, and cloning the path for every key on
    /// a map file with three hundred thousand literals would double the walk's
    /// allocations for a record nothing can translate.
    fn nass_harfi(&mut self) -> Result<(String, usize, usize), KhataNusus> {
        let bidaya = self.i;
        self.taqaddam();
        let mut mabni: Vec<u8> = Vec::new();
        loop {
            let Some(harf) = self.hali() else {
                return Err(self.naqis("the end of a string literal"));
            };
            match harf {
                b'"' => {
                    self.taqaddam();
                    break;
                },
                b'\\' => {
                    self.taqaddam();
                    self.mahrab(&mut mabni)?;
                },
                0x00..=0x1F => return Err(self.bunya("an escaped control character")),
                _ => {
                    mabni.push(harf);
                    self.taqaddam();
                },
            }
        }
        let qeema = String::from_utf8(mabni).map_err(|khata| KhataNusus::NassGhayrSalih {
            malaf: self.malaf.to_owned(),
            tarmiz: "UTF-8",
            mawqi: tul_u64(bidaya.saturating_add(khata.utf8_error().valid_up_to())),
        })?;
        Ok((qeema, bidaya, self.i))
    }

    /// Decodes one escape sequence, cursor just past the backslash.
    fn mahrab(&mut self, mabni: &mut Vec<u8>) -> Result<(), KhataNusus> {
        let Some(ramz) = self.hali() else {
            return Err(self.naqis("an escape sequence"));
        };
        let basit = match ramz {
            b'"' => Some(b'"'),
            b'\\' => Some(b'\\'),
            b'/' => Some(b'/'),
            b'b' => Some(0x08),
            b'f' => Some(0x0C),
            b'n' => Some(b'\n'),
            b'r' => Some(b'\r'),
            b't' => Some(b'\t'),
            _ => None,
        };
        if let Some(bayt) = basit {
            mabni.push(bayt);
            self.taqaddam();
            return Ok(());
        }
        if ramz != b'u' {
            return Err(self.bunya("a JSON escape sequence"));
        }
        self.taqaddam();
        let awwal = self.arbaa_sitta()?;
        let naqta = if (0xD800..=0xDBFF).contains(&awwal) {
            // A high surrogate must be followed by its low half. RPG Maker's
            // own files never carry a lone one; a file that does is refused
            // rather than repaired, because no Rust string can hold it and
            // substituting U+FFFD would hand a translator a corrupted string
            // that reads as text.
            if self.hali() != Some(b'\\') {
                return Err(self.mufrad());
            }
            self.taqaddam();
            if self.hali() != Some(b'u') {
                return Err(self.mufrad());
            }
            self.taqaddam();
            let thani = self.arbaa_sitta()?;
            if !(0xDC00..=0xDFFF).contains(&thani) {
                return Err(self.mufrad());
            }
            let alawi = awwal.saturating_sub(0xD800) << 10;
            let adna = thani.saturating_sub(0xDC00);
            0x1_0000_u32.saturating_add(alawi).saturating_add(adna)
        } else if (0xDC00..=0xDFFF).contains(&awwal) {
            return Err(self.mufrad());
        } else {
            awwal
        };
        let Some(harf) = char::from_u32(naqta) else {
            return Err(self.mufrad());
        };
        let mut khazina = [0_u8; 4];
        mabni.extend_from_slice(harf.encode_utf8(&mut khazina).as_bytes());
        Ok(())
    }

    /// The refusal for a code unit that is not a character on its own.
    fn mufrad(&self) -> KhataNusus {
        KhataNusus::NassGhayrSalih {
            malaf: self.malaf.to_owned(),
            tarmiz: "UTF-16 surrogate pair",
            mawqi: tul_u64(self.i),
        }
    }

    /// Reads exactly four hexadecimal digits as a code unit.
    fn arbaa_sitta(&mut self) -> Result<u32, KhataNusus> {
        let nihaya = self.i.saturating_add(4);
        let Some(arqam) = self.bayt.get(self.i..nihaya) else {
            return Err(self.naqis("four hexadecimal digits"));
        };
        let nass = std::str::from_utf8(arqam).map_err(|_| self.bunya("four hexadecimal digits"))?;
        let qeema =
            u32::from_str_radix(nass, 16).map_err(|_| self.bunya("four hexadecimal digits"))?;
        self.i = nihaya;
        Ok(qeema)
    }
}

/// Encodes a string as a JSON literal, quotation marks included.
///
/// Only what JSON requires is escaped: the quotation mark, the backslash and
/// the C0 controls. Everything else — Arabic, the bidirectional marks, the
/// object replacement character — is written as raw UTF-8, which is what
/// `JSON.stringify` writes and what the engine's `JSON.parse` reads back.
///
/// This is used for replacement literals only. An untranslated literal is never
/// re-encoded; its original bytes are copied through, whatever the editor chose
/// to escape in them.
#[must_use]
pub fn iqtibas_json(nass: &str) -> String {
    use std::fmt::Write as _;

    let mut makhtut = String::with_capacity(nass.len().saturating_add(2));
    makhtut.push('"');
    for harf in nass.chars() {
        match harf {
            '"' => makhtut.push_str("\\\""),
            '\\' => makhtut.push_str("\\\\"),
            '\n' => makhtut.push_str("\\n"),
            '\r' => makhtut.push_str("\\r"),
            '\t' => makhtut.push_str("\\t"),
            '\u{08}' => makhtut.push_str("\\b"),
            '\u{0C}' => makhtut.push_str("\\f"),
            _ if u32::from(harf) < 0x20 => {
                let _ = write!(makhtut, "\\u{:04x}", u32::from(harf));
            },
            _ => makhtut.push(harf),
        }
    }
    makhtut.push('"');
    makhtut
}

// ---------------------------------------------------------------------------
// Splicing, and proving the splice
// ---------------------------------------------------------------------------

/// One replacement: a byte range of the original and the literal to put there.
#[derive(Debug, Clone)]
pub struct Tabdil {
    /// Byte offset of the opening quotation mark in the original text.
    pub bidaya: usize,
    /// Byte offset one past the closing quotation mark.
    pub nihaya: usize,
    /// The replacement, already a complete JSON literal with its quotes.
    pub badeel: String,
}

/// Splices replacements into a document, copying every other byte untouched.
///
/// The order of `tabdilat` does not matter and their ranges must not overlap.
/// What comes back differs from `khaam` in exactly the ranges named and nowhere
/// else — not in key order, not in number spelling, not in whitespace, not in
/// which characters the editor chose to escape.
///
/// # Errors
///
/// [`KhataNusus::HawiyaTalifa`] when a range runs past the end of the document,
/// does not start where it ends, or overlaps another. All three mean an
/// extraction record and the file it came from have drifted apart, and splicing
/// on that basis would write a data file the engine cannot parse.
pub fn khayyit(khaam: &str, tabdilat: &[Tabdil]) -> Result<String, KhataNusus> {
    let mut tarteeb: Vec<&Tabdil> = tabdilat.iter().collect();
    tarteeb.sort_by_key(|tabdil| tabdil.bidaya);

    let mut makhtut = String::with_capacity(khaam.len());
    let mut sabiq = 0_usize;
    for tabdil in tarteeb {
        if tabdil.bidaya < sabiq || tabdil.nihaya < tabdil.bidaya {
            return Err(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "replacement range",
                qeema: tul_u64(tabdil.bidaya),
                hadd: tul_u64(sabiq),
            });
        }
        let Some(bayn) = khaam.get(sabiq..tabdil.bidaya) else {
            return Err(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "replacement start",
                qeema: tul_u64(tabdil.bidaya),
                hadd: tul_u64(khaam.len()),
            });
        };
        if khaam.get(tabdil.bidaya..tabdil.nihaya).is_none() {
            return Err(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "replacement end",
                qeema: tul_u64(tabdil.nihaya),
                hadd: tul_u64(khaam.len()),
            });
        }
        makhtut.push_str(bayn);
        makhtut.push_str(&tabdil.badeel);
        sabiq = tabdil.nihaya;
    }
    let Some(dhayl) = khaam.get(sabiq..) else {
        return Err(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "replacement end",
            qeema: tul_u64(sabiq),
            hadd: tul_u64(khaam.len()),
        });
    };
    makhtut.push_str(dhayl);
    Ok(makhtut)
}

/// Proves that the splice touched only what it was asked to touch.
///
/// Rebuilds the original from the patched text by putting every original
/// literal back at its shifted position, and compares the result with the
/// original byte for byte. A mismatch means the offset arithmetic is wrong
/// somewhere, which is the one failure mode this design can still have and the
/// one that would corrupt a data file quietly.
///
/// Run before anything reaches the game's directory. It costs one more pass
/// over a file that is already in memory, and it is the difference between
/// "should round-trip" and "does".
///
/// # Errors
///
/// [`KhataNusus::DawraGhayrMutabaqa`] carrying how many bytes differ.
pub fn tahaqquq_dawra(asl: &str, jadeed: &str, tabdilat: &[Tabdil]) -> Result<(), KhataNusus> {
    let mut tarteeb: Vec<&Tabdil> = tabdilat.iter().collect();
    tarteeb.sort_by_key(|tabdil| tabdil.bidaya);

    let mut muaad = String::with_capacity(asl.len());
    let mut sabiq_asl = 0_usize;
    let mut sabiq_jadeed = 0_usize;
    for tabdil in tarteeb {
        let Some(bayn) = jadeed.get(sabiq_jadeed..) else {
            break;
        };
        let tul_bayn = tabdil.bidaya.saturating_sub(sabiq_asl);
        let Some(nusukh) = bayn.get(..tul_bayn) else {
            break;
        };
        muaad.push_str(nusukh);
        if let Some(asli) = asl.get(tabdil.bidaya..tabdil.nihaya) {
            muaad.push_str(asli);
        }
        sabiq_asl = tabdil.nihaya;
        sabiq_jadeed = sabiq_jadeed
            .saturating_add(tul_bayn)
            .saturating_add(tabdil.badeel.len());
    }
    if let Some(dhayl) = jadeed.get(sabiq_jadeed..) {
        muaad.push_str(dhayl);
    }

    if muaad == asl {
        return Ok(());
    }
    let mukhtalif = farq_bayt(asl.as_bytes(), muaad.as_bytes());
    Err(KhataNusus::DawraGhayrMutabaqa {
        sigha: SIGHA,
        adad: mukhtalif,
    })
}

/// How many byte positions two buffers disagree on, counting a length
/// difference as a disagreement per missing byte.
fn farq_bayt(awwal: &[u8], thani: &[u8]) -> u64 {
    let mushtarak = awwal.len().min(thani.len());
    let mut adad = tul_u64(awwal.len().max(thani.len()).saturating_sub(mushtarak));
    for khana in 0..mushtarak {
        if awwal.get(khana) != thani.get(khana) {
            adad = adad.saturating_add(1);
        }
    }
    adad
}

// ---------------------------------------------------------------------------
// The escape grammar
//
// This scanner is an index over the raw string, not a second shaper-facing
// extractor. The extractor that feeds the bidirectional algorithm at run time
// is `taarib_saff::nasq` with `LahjatNasq::RpgMaker`, and it stays the only
// one: a second Arabic-facing markup parser would be a second set of bugs.
// What this produces is the patch-time index — the identity of a line with its
// markup removed, the span records the compiler records, and the reconstruction
// that puts the markup back after a translator has seen only words.
//
// Two deliberate differences from the run-time dialect, both reported rather
// than hidden:
//
//   * This scanner is lenient. `nasq` refuses `\V` with no `[n]` after it,
//     which is correct at layout time. At patch time the engine itself draws a
//     literal backslash there, and refusing to extract a whole game over one
//     malformed code in one line would be the wrong trade.
//   * This scanner knows `\{`, `\}` and `\$`, which the run-time dialect
//     currently leaves as literal text. They are recorded so the size changes
//     and the gold window survive a round trip through a translator.
// ---------------------------------------------------------------------------

/// What one RPG Maker escape code does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NawHarf {
    /// `\\` — one printable backslash. The only code that leaves a character
    /// behind in the clean text.
    Mail,
    /// `\C[n]` — the text colour changes to palette entry `n` from here on.
    Lawn,
    /// `\I[n]` — icon `n` is drawn inline, occupying width and drawing no
    /// glyph.
    Ayqouna,
    /// `\V[n]` — the value of game variable `n` is substituted at draw time.
    Mutaghayyir,
    /// `\N[n]` — actor `n`'s name is substituted at draw time.
    IsmBatal,
    /// `\P[n]` — party member `n`'s name is substituted at draw time.
    IsmRafiq,
    /// `\G` — the currency unit.
    Umla,
    /// `\$` — opens the gold window.
    Nuqud,
    /// `\{` — one step larger from here on.
    Takbir,
    /// `\}` — one step smaller from here on.
    Tasgheer,
    /// `\.` — a quarter-second pause.
    Waqfa,
    /// `\|` — a one-second pause.
    Intizar,
    /// `\!` — wait for the confirm button.
    Iqaf,
    /// `\>` — draw the rest of the line at once.
    Fawri,
    /// `\<` — cancel drawing the rest of the line at once.
    Tadrijee,
    /// `\^` — do not wait at the end of the message.
    BilaIntizar,
}

impl NawHarf {
    /// The code's name, as the extraction record and the diagnostics write it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mail => "backslash",
            Self::Lawn => "colour",
            Self::Ayqouna => "icon",
            Self::Mutaghayyir => "variable",
            Self::IsmBatal => "actor name",
            Self::IsmRafiq => "party member name",
            Self::Umla => "currency",
            Self::Nuqud => "gold window",
            Self::Takbir => "size up",
            Self::Tasgheer => "size down",
            Self::Waqfa => "quarter-second pause",
            Self::Intizar => "one-second pause",
            Self::Iqaf => "wait for input",
            Self::Fawri => "instant",
            Self::Tadrijee => "not instant",
            Self::BilaIntizar => "no trailing wait",
        }
    }

    /// Whether the code leaves a character in the clean text.
    ///
    /// True for `\\` alone. Everything else is lifted out entirely, which is
    /// the whole point: a backslash and a bracket must never reach the shaper
    /// as literal characters, because the bidirectional algorithm resolves them
    /// as neutrals and a neutral in the middle of an Arabic run reorders the
    /// text around it.
    #[must_use]
    pub const fn yastabdil(self) -> bool {
        matches!(self, Self::Mail)
    }

    /// Whether the code opens a span that runs until another code closes it,
    /// rather than standing at one point.
    ///
    /// Colour and size do; icons, pauses and substitutions do not. The
    /// distinction is what lets a coloured word keep its colour after
    /// reordering has moved it to the other end of the line.
    #[must_use]
    pub const fn mumtadd(self) -> bool {
        matches!(
            self,
            Self::Lawn | Self::Takbir | Self::Tasgheer | Self::Fawri | Self::Tadrijee
        )
    }

    /// Which family of extent this code belongs to, for closing a span.
    ///
    /// Colour closes colour, a size step closes a size step, and instant closes
    /// instant. A colour change does not end a size change, which is why one
    /// list of "the next markup code" would get both wrong.
    const fn ailat_imtidad(self) -> Option<u8> {
        match self {
            Self::Lawn => Some(0),
            Self::Takbir | Self::Tasgheer => Some(1),
            Self::Fawri | Self::Tadrijee => Some(2),
            _ => None,
        }
    }
}

/// One escape code, lifted out of a string and recorded against the clean text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NitaqHurub {
    /// What the code does.
    pub naw: NawHarf,
    /// The code exactly as it was written, so reconstruction is a copy rather
    /// than a re-spelling.
    pub khaam: String,
    /// Byte offset into the clean text where the code stood.
    pub mawqi: usize,
    /// How many bytes of clean text this span governs. Zero for a code that
    /// stands at a point, one for `\\`, and the distance to the next code of
    /// its family for a colour or size change.
    pub tul: usize,
    /// The bracketed number exactly as written, for the codes that take one.
    pub tarteeb: Option<u32>,
}

/// A string split into the text a translator sees and the markup they do not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NassMuqassam {
    /// The text with every escape code lifted out.
    pub naqi: String,
    /// Where the codes were, in the order they appeared.
    pub nitaqat: Vec<NitaqHurub>,
}

impl NassMuqassam {
    /// Whether the string carried no markup at all.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.nitaqat.is_empty()
    }

    /// Rebuilds the original string from the clean text and the records.
    ///
    /// Byte for byte, for every input [`iqsim_hurub`] produced. This is the
    /// half of the contract the translation pipeline rests on: markup is lifted
    /// out before a translator or a machine translation provider ever sees the
    /// string, so neither can damage a code they were never shown, and it is
    /// put back mechanically rather than by pattern-matching the result.
    #[must_use]
    pub fn aid_binaa(&self) -> String {
        let mut khaam = String::with_capacity(self.naqi.len().saturating_add(16));
        let mut sabiq = 0_usize;
        for nitaq in &self.nitaqat {
            if let Some(bayn) = self.naqi.get(sabiq..nitaq.mawqi) {
                khaam.push_str(bayn);
                sabiq = nitaq.mawqi;
            }
            khaam.push_str(&nitaq.khaam);
            if nitaq.naw.yastabdil() {
                sabiq = sabiq.saturating_add(nitaq.tul);
            }
        }
        if let Some(dhayl) = self.naqi.get(sabiq..) {
            khaam.push_str(dhayl);
        }
        khaam
    }
}

/// Lifts every escape code out of a string.
///
/// The complete grammar, on the *decoded* string — the JSON reader has already
/// turned `\\n` into a newline, so a backslash here is a real backslash:
///
/// | written | meaning | leaves | extent |
/// | --- | --- | --- | --- |
/// | `\\` | one printable backslash | `\` | one byte |
/// | `\C[n]` `\c[n]` | colour becomes palette entry `n` | – | to the next colour code |
/// | `\I[n]` `\i[n]` | icon `n` inline | – | a point |
/// | `\V[n]` `\v[n]` | game variable `n` | – | a point |
/// | `\N[n]` `\n[n]` | actor `n`'s name | – | a point |
/// | `\P[n]` `\p[n]` | party member `n`'s name | – | a point |
/// | `\G` `\g` | the currency unit | – | a point |
/// | `\$` | open the gold window | – | a point |
/// | `\{` | one size step up | – | to the next size code |
/// | `\}` | one size step down | – | to the next size code |
/// | `\.` | quarter-second pause | – | a point |
/// | `\|` | one-second pause | – | a point |
/// | `\!` | wait for input | – | a point |
/// | `\>` | rest of the line at once | – | to `\<` |
/// | `\<` | cancel that | – | to `\>` |
/// | `\^` | no wait at the end | – | a point |
///
/// A backslash followed by anything else — including one of the bracketed
/// codes with its bracket missing — is ordinary text and draws as a backslash,
/// which is exactly what the engine does with it.
#[must_use]
pub fn iqsim_hurub(khaam: &str) -> NassMuqassam {
    let mut naqi = String::with_capacity(khaam.len());
    let mut nitaqat: Vec<NitaqHurub> = Vec::new();
    let mut i = 0_usize;

    while let Some(baqi) = khaam.get(i..) {
        let Some(harf) = baqi.chars().next() else {
            break;
        };
        if harf != '\\' {
            naqi.push(harf);
            i = i.saturating_add(harf.len_utf8());
            continue;
        }
        let bad = khaam.get(i.saturating_add(1)..).unwrap_or_default();
        let Some(ramz) = bad.chars().next() else {
            naqi.push('\\');
            i = i.saturating_add(1);
            continue;
        };
        let muqawwas = |naw: NawHarf| -> Option<(NawHarf, usize, Option<u32>)> {
            let baad_ramz = bad.get(ramz.len_utf8()..).unwrap_or_default();
            let (qeema, akal) = arqam_muqawwasa(baad_ramz)?;
            Some((
                naw,
                1_usize.saturating_add(ramz.len_utf8()).saturating_add(akal),
                Some(qeema),
            ))
        };
        let mufrad = |naw: NawHarf| -> Option<(NawHarf, usize, Option<u32>)> {
            Some((naw, 1_usize.saturating_add(ramz.len_utf8()), None))
        };
        let qiraa = match ramz {
            '\\' => mufrad(NawHarf::Mail),
            'C' | 'c' => muqawwas(NawHarf::Lawn),
            'I' | 'i' => muqawwas(NawHarf::Ayqouna),
            'V' | 'v' => muqawwas(NawHarf::Mutaghayyir),
            'N' | 'n' => muqawwas(NawHarf::IsmBatal),
            'P' | 'p' => muqawwas(NawHarf::IsmRafiq),
            'G' | 'g' => mufrad(NawHarf::Umla),
            '$' => mufrad(NawHarf::Nuqud),
            '{' => mufrad(NawHarf::Takbir),
            '}' => mufrad(NawHarf::Tasgheer),
            '.' => mufrad(NawHarf::Waqfa),
            '|' => mufrad(NawHarf::Intizar),
            '!' => mufrad(NawHarf::Iqaf),
            '>' => mufrad(NawHarf::Fawri),
            '<' => mufrad(NawHarf::Tadrijee),
            '^' => mufrad(NawHarf::BilaIntizar),
            _ => None,
        };
        let Some((naw, tul_ramz, tarteeb)) = qiraa else {
            naqi.push('\\');
            i = i.saturating_add(1);
            continue;
        };
        let nihaya = i.saturating_add(tul_ramz);
        let Some(makhtut) = khaam.get(i..nihaya) else {
            naqi.push('\\');
            i = i.saturating_add(1);
            continue;
        };
        nitaqat.push(NitaqHurub {
            naw,
            khaam: makhtut.to_owned(),
            mawqi: naqi.len(),
            tul: usize::from(naw.yastabdil()),
            tarteeb,
        });
        if naw.yastabdil() {
            naqi.push('\\');
        }
        i = nihaya;
    }

    imtidad(&mut nitaqat, naqi.len());
    NassMuqassam { naqi, nitaqat }
}

/// Reads `[digits]` at the start of a string, returning the number and how many
/// bytes it consumed including both brackets.
///
/// Nine digits at most, which is every index any RPG Maker database can hold
/// and one short of what would not fit a `u32`. A longer run is not a code.
fn arqam_muqawwasa(nass: &str) -> Option<(u32, usize)> {
    let mut huruf = nass.chars();
    if huruf.next()? != '[' {
        return None;
    }
    let mut arqam = String::new();
    let mut akal = 1_usize;
    for harf in huruf {
        akal = akal.saturating_add(harf.len_utf8());
        if harf == ']' {
            if arqam.is_empty() {
                return None;
            }
            return arqam.parse::<u32>().ok().map(|qeema| (qeema, akal));
        }
        if !harf.is_ascii_digit() || arqam.len() >= 9 {
            return None;
        }
        arqam.push(harf);
    }
    None
}

/// Gives every extent-carrying code the run of clean text it governs.
///
/// A colour change runs until the next colour change, a size step until the
/// next size step, and instant drawing until it is cancelled. Each family is
/// closed only by its own, because a colour change in the middle of an enlarged
/// run does not end the enlargement.
fn imtidad(nitaqat: &mut [NitaqHurub], tul_naqi: usize) {
    let mawaqi: Vec<(usize, Option<u8>)> = nitaqat
        .iter()
        .map(|nitaq| (nitaq.mawqi, nitaq.naw.ailat_imtidad()))
        .collect();
    for (khana, nitaq) in nitaqat.iter_mut().enumerate() {
        let Some(aila) = nitaq.naw.ailat_imtidad() else {
            continue;
        };
        let baad = khana.saturating_add(1);
        let nihaya = mawaqi
            .iter()
            .skip(baad)
            .find(|(_, ukhra)| *ukhra == Some(aila))
            .map_or(tul_naqi, |(mawqi, _)| *mawqi);
        nitaq.tul = nihaya.saturating_sub(nitaq.mawqi);
    }
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// Where a translatable string came from.
///
/// Kept on every record because it decides how the string is presented for
/// translation and how much room it has: a command label lives in a fixed
/// button and a dialogue line wraps inside a message window, and a translator
/// shown both as "text" will overflow one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NawMadkhal {
    /// The speaker's name on a message header — command 101, MZ only.
    IsmMutakallim,
    /// One line of a message — command 401.
    SatrNass,
    /// One entry of a choice list — command 102, and its echo on 402.
    Ikhtiyar,
    /// One line of scrolling text — command 405.
    NassMutamarrir,
    /// A map's display name.
    IsmKhareeta,
    /// A field of a database row.
    HaqlQaeda,
    /// A term from `System.json`.
    MustalahNizam,
    /// A plugin parameter that carries user-facing text.
    BarametrMulhaq,
}

impl NawMadkhal {
    /// The kind's stable name, as the patch and the diagnostics write it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::IsmMutakallim => "speaker",
            Self::SatrNass => "message",
            Self::Ikhtiyar => "choice",
            Self::NassMutamarrir => "scrolling",
            Self::IsmKhareeta => "map name",
            Self::HaqlQaeda => "database",
            Self::MustalahNizam => "term",
            Self::BarametrMulhaq => "plugin parameter",
        }
    }
}

/// One translatable string, with its identity and its markup lifted out.
#[derive(Debug, Clone)]
pub struct MadkhalNusus {
    /// The stable identity — see [`huwiya`].
    pub huwiya: String,
    /// The file, project-relative, with forward slashes.
    pub malaf: String,
    /// The path inside that file, as [`masar_nassi`] writes it.
    pub masar: String,
    /// What kind of string it is.
    pub naw: NawMadkhal,
    /// The string exactly as the file stores it, markup included.
    pub khaam: String,
    /// The same string with every escape code lifted out — what a translator
    /// is shown and what the identity is computed over.
    pub naqi: String,
    /// Where the escape codes were.
    pub nitaqat: Vec<NitaqHurub>,
    /// Byte offset of the opening quotation mark of the literal.
    pub bidaya: usize,
    /// Byte offset one past its closing quotation mark.
    pub nihaya: usize,
}

/// Everything one project offers for translation.
#[derive(Debug, Clone, Default)]
pub struct SijillNusus {
    /// Every string found, in the order the files were walked.
    pub madakhil: Vec<MadkhalNusus>,
    /// Every file that contributed one, project-relative.
    pub malaffat: BTreeSet<String>,
}

impl SijillNusus {
    /// The records that came from one file, in document order.
    #[must_use]
    pub fn li_malaf(&self, malaf: &str) -> Vec<&MadkhalNusus> {
        let mut mukhtara: Vec<&MadkhalNusus> = self
            .madakhil
            .iter()
            .filter(|madkhal| madkhal.malaf == malaf)
            .collect();
        mukhtara.sort_by_key(|madkhal| madkhal.bidaya);
        mukhtara
    }

    /// How many records of each kind were found, for the capability report.
    #[must_use]
    pub fn ihsaa(&self) -> BTreeMap<NawMadkhal, usize> {
        let mut adad: BTreeMap<NawMadkhal, usize> = BTreeMap::new();
        for madkhal in &self.madakhil {
            let khana = adad.entry(madkhal.naw).or_insert(0_usize);
            *khana = khana.saturating_add(1);
        }
        adad
    }
}

/// The identity a translation is keyed on.
///
/// BLAKE3 over the file, the path inside it, and the *clean* text — markup
/// excluded on purpose. Recolouring a line, moving an icon inside it, or
/// changing how many quarter-second pauses it carries leaves the identity
/// alone, so a translation survives the developer editing the presentation of a
/// line they did not rewrite. Moving the line to a different event does change
/// it, because a line that moved is a line whose context moved with it.
#[must_use]
pub fn huwiya(malaf: &str, masar: &str, naqi: &str) -> String {
    use std::fmt::Write as _;

    let mut hashib = blake3::Hasher::new();
    hashib.update(malaf.as_bytes());
    hashib.update(&[0]);
    hashib.update(masar.as_bytes());
    hashib.update(&[0]);
    hashib.update(naqi.as_bytes());
    let basma = hashib.finalize();
    let bayt = basma.as_bytes();
    let mut makhtut = String::with_capacity(32);
    for khana in 0..16_usize {
        if let Some(qeema) = bayt.get(khana) {
            let _ = write!(makhtut, "{qeema:02x}");
        }
    }
    makhtut
}

/// The database files whose rows carry user-facing text, and which fields of a
/// row those are.
///
/// `note` is absent from every one of these lists, deliberately. A note is not
/// documentation — it is the parameter channel plugins read, full of
/// `<TextColor:3>` and `<Passive: atk + 20>`, and translating one breaks the
/// plugin that parses it. Notes that genuinely hold player-facing text are a
/// per-plugin convention and belong to whoever wrote that plugin, not here.
static QAWAID_BAYANAT: [(&str, &[&str]); 8] = [
    ("Actors.json", &["name", "nickname", "profile"]),
    ("Classes.json", &["name"]),
    (
        "Skills.json",
        &["name", "description", "message1", "message2"],
    ),
    ("Items.json", &["name", "description"]),
    ("Weapons.json", &["name", "description"]),
    ("Armors.json", &["name", "description"]),
    ("Enemies.json", &["name"]),
    (
        "States.json",
        &["name", "message1", "message2", "message3", "message4"],
    ),
];

/// The `System.json` arrays whose entries name things the player reads.
///
/// `switches` and `variables` are absent: they are the editor's own names for
/// its bookkeeping slots and are never drawn on a screen.
const MASFUFAT_NIZAM: [&str; 5] = [
    "armorTypes",
    "elements",
    "equipTypes",
    "skillTypes",
    "weaponTypes",
];

/// Reads every translatable string out of a project.
///
/// Walks `data/` for the map files, the event lists and the database, reads
/// `System.json`'s term tree, and reads the user-facing parameters out of
/// `js/plugins.js`. Every record carries the byte range of the literal it came
/// from, which is what makes the later write a splice rather than a
/// re-serialization.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] when `data/` or one of its files cannot be read,
/// [`KhataNusus::HajmMufrit`] for a file past [`AQSA_MALAF`],
/// [`KhataNusus::NassGhayrSalih`] for one that is not UTF-8, and
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] for one whose JSON this build cannot
/// walk. A data file that will not parse is refused rather than skipped: a
/// silently skipped map is a map whose dialogue never reaches a translator and
/// nobody finds out until a player does.
pub fn istakhrij(bunya: &BunyatMashru) -> Result<SijillNusus, KhataNusus> {
    let mut sijill = SijillNusus::default();
    let bayanat = bunya.bayanat();
    let qaima = fs::read_dir(&bayanat).map_err(|sabab| KhataNusus::KhataMalaf {
        masar: bayanat.clone(),
        sabab,
    })?;

    let mut asmaa: Vec<String> = Vec::new();
    for madkhal in qaima.take(AQSA_MADAKHIL).flatten() {
        if !madkhal.file_type().is_ok_and(|naw| naw.is_file()) {
            continue;
        }
        if let Some(ism) = madkhal.file_name().to_str() {
            asmaa.push(ism.to_owned());
        }
    }
    asmaa.sort();

    // The leaf names are folded, for the same reason [`tarkeeb::masar_bila_hala`]
    // folds the directories above them: a repack that lower-cased `data/` did
    // not stop at the directory, and this is the other half of the same walk. A
    // `commonevents.json` that falls through here is a file holding every common
    // event's dialogue that no translator is ever shown, and the install then
    // reports success having translated none of it — the silent skip this
    // function's own contract refuses.
    //
    // `nisbi` is still built from the entry's real spelling, so `malaf` opens the
    // file the directory actually offered rather than the one this table names.
    for ism in &asmaa {
        let nisbi = format!("data/{ism}");
        if ism_khareeta(ism) {
            istakhrij_malaf(bunya, &nisbi, &mut sijill, Sinf::Khareeta)?;
        } else if ism.eq_ignore_ascii_case("CommonEvents.json") {
            istakhrij_malaf(bunya, &nisbi, &mut sijill, Sinf::AhdathAmma)?;
        } else if ism.eq_ignore_ascii_case("Troops.json") {
            istakhrij_malaf(bunya, &nisbi, &mut sijill, Sinf::Firaq)?;
        } else if ism.eq_ignore_ascii_case("System.json") {
            istakhrij_malaf(bunya, &nisbi, &mut sijill, Sinf::Nizam)?;
        } else if let Some(huqul) = QAWAID_BAYANAT
            .iter()
            .find(|(marja, _)| marja.eq_ignore_ascii_case(ism))
            .map(|(_, huqul)| *huqul)
        {
            istakhrij_malaf(bunya, &nisbi, &mut sijill, Sinf::Qaeda(huqul))?;
        }
    }

    istakhrij_mulhaqat(bunya, &mut sijill)?;
    Ok(sijill)
}

/// How many entries one directory listing here will look at.
const AQSA_MADAKHIL: usize = 16_384;

/// Which walker a data file needs.
#[derive(Debug, Clone, Copy)]
enum Sinf {
    /// `MapNNN.json` — a display name and every event's command list.
    Khareeta,
    /// `CommonEvents.json` — one command list per row.
    AhdathAmma,
    /// `Troops.json` — one command list per page of each row.
    Firaq,
    /// `System.json` — the term tree and the type-name arrays.
    Nizam,
    /// A database table and the fields of a row that carry text.
    Qaeda(&'static [&'static str]),
}

/// Whether a filename is one of the numbered map files.
///
/// `MapInfos.json` is deliberately not one: it holds the editor's tree of map
/// names, which the player never sees, and translating it would put Arabic into
/// the developer's own project browser and nowhere else. The digit test is what
/// excludes it, and it survives the fold below — `mapinfos` is no more digits
/// than `MapInfos` is.
///
/// Folded, because `strip_prefix` and `strip_suffix` compare bytes and the
/// repacks this crate exists to survive lower-case whole trees at a time.
fn ism_khareeta(ism: &str) -> bool {
    let saghir = ism.to_ascii_lowercase();
    let Some(baqi) = saghir.strip_prefix("map") else {
        return false;
    };
    let Some(raqm) = baqi.strip_suffix(".json") else {
        return false;
    };
    !raqm.is_empty() && raqm.bytes().all(|bayt| bayt.is_ascii_digit())
}

/// Reads one data file and adds whatever it offers.
fn istakhrij_malaf(
    bunya: &BunyatMashru,
    nisbi: &str,
    sijill: &mut SijillNusus,
    sinf: Sinf,
) -> Result<(), KhataNusus> {
    let masar = bunya.malaf(nisbi);
    let bayt = iqra_malaf(&masar, AQSA_MALAF)?;
    let (khaam, _) = nass_min_bayt(&bayt, nisbi)?;
    let fahras = fahras_mawaqi(imsah_nusus(&khaam, nisbi)?);
    let jidhr: Value =
        serde_json::from_str(&khaam).map_err(|khata| KhataNusus::BunyaGhayrMutawaqqaa {
            malaf: nisbi.to_owned(),
            haql: format!("the document does not parse: {khata}"),
        })?;

    let mut jami = Jami {
        sijill: &mut *sijill,
        fahras,
        malaf: nisbi,
        izaha: 0,
    };
    match sinf {
        Sinf::Khareeta => jami.khareeta(&jidhr),
        Sinf::AhdathAmma => jami.ahdath_amma(&jidhr),
        Sinf::Firaq => jami.firaq(&jidhr),
        Sinf::Nizam => jami.nizam(&jidhr),
        Sinf::Qaeda(huqul) => jami.qaeda(&jidhr, huqul),
    }
    if jami
        .sijill
        .madakhil
        .iter()
        .any(|madkhal| madkhal.malaf == nisbi)
    {
        let _ = sijill.malaffat.insert(nisbi.to_owned());
    }
    Ok(())
}

/// Indexes a scan by path, so a semantic walk can find a literal's bytes.
fn fahras_mawaqi(mawaqi: Vec<MawqiNass>) -> BTreeMap<String, MawqiNass> {
    let mut fahras = BTreeMap::new();
    for mawqi in mawaqi {
        let miftah = masar_nassi(&mawqi.masar);
        let _ = fahras.insert(miftah, mawqi);
    }
    fahras
}

/// The bridge between the semantic walk and the byte index.
struct Jami<'a> {
    sijill: &'a mut SijillNusus,
    fahras: BTreeMap<String, MawqiNass>,
    malaf: &'a str,
    izaha: usize,
}

impl Jami<'_> {
    /// Records the literal at `masar`, if there is one and it holds text.
    ///
    /// A string that is empty or nothing but whitespace is skipped: an empty
    /// message line is a blank line in a window, and offering it for
    /// translation wastes a translator's attention on a string that has none.
    fn daa(&mut self, masar: &str, naw: NawMadkhal) {
        let Some(mawqi) = self.fahras.get(masar) else {
            return;
        };
        if mawqi.qeema.trim().is_empty() {
            return;
        }
        let muqassam = iqsim_hurub(&mawqi.qeema);
        if muqassam.naqi.trim().is_empty() {
            return;
        }
        let huwiyat = huwiya(self.malaf, masar, &muqassam.naqi);
        self.sijill.madakhil.push(MadkhalNusus {
            huwiya: huwiyat,
            malaf: self.malaf.to_owned(),
            masar: masar.to_owned(),
            naw,
            khaam: mawqi.qeema.clone(),
            naqi: muqassam.naqi,
            nitaqat: muqassam.nitaqat,
            bidaya: mawqi.bidaya.saturating_add(self.izaha),
            nihaya: mawqi.nihaya.saturating_add(self.izaha),
        });
    }
}

impl Jami<'_> {
    /// One event command list.
    ///
    /// The four codes that hold text a player reads, plus the speaker name MZ
    /// added as a fifth parameter of the message header. Command 402 is
    /// included because the editor writes the chosen branch's label a second
    /// time there: it is never drawn, and leaving it in the source language
    /// turns the developer's own event editor into a bilingual mess the next
    /// time they open it.
    fn qaima_awamir(&mut self, qaima: &Value, asas: &str) {
        let Some(awamir) = qaima.as_array() else {
            return;
        };
        for (khana, amr) in awamir.iter().enumerate() {
            let Some(ramz) = amr.get("code").and_then(Value::as_i64) else {
                continue;
            };
            let barametr = amr.get("parameters");
            let asl = format!("{asas}/{khana}/parameters");
            match ramz {
                101
                    // MZ writes the speaker's name into the fifth parameter;
                    // MV has only four and carries its name boxes inside the
                    // 401 lines, where this walker already sees them.
                    if barametr.and_then(|qeem| qeem.get(4)).is_some_and(Value::is_string) => {
                        self.daa(&format!("{asl}/4"), NawMadkhal::IsmMutakallim);
                    }
                401 => self.daa(&format!("{asl}/0"), NawMadkhal::SatrNass),
                405 => self.daa(&format!("{asl}/0"), NawMadkhal::NassMutamarrir),
                402 => self.daa(&format!("{asl}/1"), NawMadkhal::Ikhtiyar),
                102 => {
                    let ikhtiyarat =
                        barametr.and_then(|qeem| qeem.get(0)).and_then(Value::as_array);
                    if let Some(qaima_ikhtiyar) = ikhtiyarat {
                        for (mawdi, _) in qaima_ikhtiyar.iter().enumerate() {
                            self.daa(&format!("{asl}/0/{mawdi}"), NawMadkhal::Ikhtiyar);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// One map: its display name and every event page's command list.
    fn khareeta(&mut self, jidhr: &Value) {
        self.daa("/displayName", NawMadkhal::IsmKhareeta);
        let Some(ahdath) = jidhr.get("events").and_then(Value::as_array) else {
            return;
        };
        for (khana, hadath) in ahdath.iter().enumerate() {
            let Some(safahat) = hadath.get("pages").and_then(Value::as_array) else {
                continue;
            };
            for (safha, warqa) in safahat.iter().enumerate() {
                let Some(qaima) = warqa.get("list") else {
                    continue;
                };
                let asas = format!("/events/{khana}/pages/{safha}/list");
                self.qaima_awamir(qaima, &asas);
            }
        }
    }

    /// `CommonEvents.json`: an array whose first entry is null.
    fn ahdath_amma(&mut self, jidhr: &Value) {
        let Some(sufuf) = jidhr.as_array() else {
            return;
        };
        for (khana, saf) in sufuf.iter().enumerate() {
            let Some(qaima) = saf.get("list") else {
                continue;
            };
            self.qaima_awamir(qaima, &format!("/{khana}/list"));
        }
    }

    /// `Troops.json`: one command list per page of each troop.
    fn firaq(&mut self, jidhr: &Value) {
        let Some(sufuf) = jidhr.as_array() else {
            return;
        };
        for (khana, saf) in sufuf.iter().enumerate() {
            let Some(safahat) = saf.get("pages").and_then(Value::as_array) else {
                continue;
            };
            for (safha, warqa) in safahat.iter().enumerate() {
                let Some(qaima) = warqa.get("list") else {
                    continue;
                };
                self.qaima_awamir(qaima, &format!("/{khana}/pages/{safha}/list"));
            }
        }
    }

    /// A database table: the named fields of every row that has them.
    fn qaeda(&mut self, jidhr: &Value, huqul: &[&str]) {
        let Some(sufuf) = jidhr.as_array() else {
            return;
        };
        for (khana, saf) in sufuf.iter().enumerate() {
            for haql in huqul {
                if saf.get(*haql).is_some_and(Value::is_string) {
                    self.daa(&format!("/{khana}/{haql}"), NawMadkhal::HaqlQaeda);
                }
            }
        }
    }

    /// `System.json`: the title, the currency unit, the whole `terms` tree, and
    /// the type-name arrays.
    fn nizam(&mut self, jidhr: &Value) {
        self.daa("/gameTitle", NawMadkhal::MustalahNizam);
        self.daa("/currencyUnit", NawMadkhal::MustalahNizam);

        for ism in MASFUFAT_NIZAM {
            let Some(masfufa) = jidhr.get(ism).and_then(Value::as_array) else {
                continue;
            };
            for (khana, _) in masfufa.iter().enumerate() {
                self.daa(&format!("/{ism}/{khana}"), NawMadkhal::MustalahNizam);
            }
        }

        let Some(mustalahat) = jidhr.get("terms") else {
            return;
        };
        for ism in ["basic", "commands", "params"] {
            let Some(masfufa) = mustalahat.get(ism).and_then(Value::as_array) else {
                continue;
            };
            for (khana, _) in masfufa.iter().enumerate() {
                self.daa(&format!("/terms/{ism}/{khana}"), NawMadkhal::MustalahNizam);
            }
        }
        // `messages` is an object, not an array, and its values carry the
        // engine's own `%1`/`%2` substitutions. Those are the host language's
        // placeholders rather than RPG Maker markup, so they stay inside the
        // string and reach the translator, who has to be able to move them.
        if let Some(rasail) = mustalahat.get("messages").and_then(Value::as_object) {
            for (miftah, qeema) in rasail {
                if qeema.is_string() {
                    self.daa(
                        &format!("/terms/messages/{miftah}"),
                        NawMadkhal::MustalahNizam,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `js/plugins.js`
//
// Not a JSON document: it is a JavaScript file that assigns one array literal
// to a global. The array itself is JSON, so it is located textually and then
// handled exactly like every other file here — scanned for byte ranges, walked
// for meaning, spliced rather than rewritten. Nothing outside the array is ever
// touched, which matters because developers put licence headers, feature
// switches and `//` comments in this file all the time.
// ---------------------------------------------------------------------------

/// The byte range of the `$plugins` array literal inside `js/plugins.js`,
/// brackets included.
///
/// # Errors
///
/// [`KhataNusus::HimlMarfud`] when the assignment or its array cannot be found,
/// which is a rung declining rather than a failure: the caller can still
/// install everything else and report that the plugin list was left alone.
pub fn mawqi_mulhaqat(khaam: &str) -> Result<(usize, usize), KhataNusus> {
    let marfud = |sabab: &str| KhataNusus::HimlMarfud {
        alia: "js/plugins.js",
        sabab: sabab.to_owned(),
    };
    let Some(mawqi) = khaam.find("$plugins") else {
        return Err(marfud("the file assigns nothing to $plugins"));
    };
    let baqi = khaam.get(mawqi..).unwrap_or_default();
    let Some(fath) = baqi.find('[') else {
        return Err(marfud("no array literal follows the $plugins assignment"));
    };
    let bidaya = mawqi.saturating_add(fath);
    let nihaya = nihayat_masfufa(khaam, bidaya)
        .ok_or_else(|| marfud("the $plugins array literal is not closed"))?;
    Ok((bidaya, nihaya))
}

/// Finds the bracket that closes the array literal starting at `bidaya`.
///
/// String-aware, because a plugin description containing a `]` is ordinary and
/// a naive `rfind` would either stop inside one or run past the array into the
/// rest of the file.
fn nihayat_masfufa(khaam: &str, bidaya: usize) -> Option<usize> {
    let bayt = khaam.as_bytes();
    let mut umq = 0_u32;
    let mut dakhil_nass = false;
    let mut mahrub = false;
    let mut i = bidaya;
    while let Some(&harf) = bayt.get(i) {
        if dakhil_nass {
            if mahrub {
                mahrub = false;
            } else if harf == b'\\' {
                mahrub = true;
            } else if harf == b'"' {
                dakhil_nass = false;
            }
        } else {
            match harf {
                b'"' => dakhil_nass = true,
                b'[' | b'{' => umq = umq.saturating_add(1),
                b']' | b'}' => {
                    umq = umq.saturating_sub(1);
                    if umq == 0 {
                        return Some(i.saturating_add(1));
                    }
                },
                _ => {},
            }
        }
        i = i.saturating_add(1);
    }
    None
}

/// The parameter key fragments that mark a value as something a player reads.
///
/// A single English word — `Attack`, `Escape` — is indistinguishable from a
/// switch value by inspection, so the key decides. Everything here is matched
/// case-insensitively as a substring.
const ALFAZ_NASS: [&str; 14] = [
    "text", "name", "label", "message", "desc", "title", "command", "term", "caption", "hint",
    "help", "word", "format", "prompt",
];

/// File extensions a parameter value ending in one is an asset reference, not
/// text.
const LAWAHIQ_MAWARID: [&str; 11] = [
    ".png", ".jpg", ".ogg", ".m4a", ".wav", ".mp3", ".webm", ".json", ".js", ".ttf", ".woff",
];

/// Whether a plugin parameter carries text a player reads.
///
/// The complete rule, in order. A value is **rejected** when it is blank or
/// longer than five hundred and twelve characters; when it parses as a number;
/// when it is `true`, `false` or `null` in any case; when it begins with `[` or
/// `{`, which in MZ means a nested JSON document this module will not recurse
/// into because recursing would mean re-serializing it; when it contains a
/// slash or a backslash and no space, which is a path; or when it ends in one
/// of [`LAWAHIQ_MAWARID`].
///
/// A surviving value is **accepted** when it contains whitespace, or a
/// non-ASCII character, or an RPG Maker escape code, or when its key contains
/// one of [`ALFAZ_NASS`]. Everything else is left alone, because a bare token
/// like `left` or `#ffffff` under a key like `align` is a setting, and offering
/// it for translation invites somebody to translate it and break the plugin.
#[must_use]
pub fn nass_wajih(miftah: &str, qeema: &str) -> bool {
    let munaqqa = qeema.trim();
    if munaqqa.is_empty() || munaqqa.chars().count() > 512 {
        return false;
    }
    if munaqqa.parse::<f64>().is_ok() {
        return false;
    }
    if matches!(
        munaqqa.to_ascii_lowercase().as_str(),
        "true" | "false" | "null"
    ) {
        return false;
    }
    if munaqqa.starts_with('[') || munaqqa.starts_with('{') {
        return false;
    }
    let bila_faragh = !munaqqa.chars().any(char::is_whitespace);
    if bila_faragh && (munaqqa.contains('/') || munaqqa.contains('\\')) {
        return false;
    }
    let munkhafid = munaqqa.to_ascii_lowercase();
    if LAWAHIQ_MAWARID
        .iter()
        .any(|lahiqa| munkhafid.ends_with(lahiqa))
    {
        return false;
    }
    if !bila_faragh || !munaqqa.is_ascii() {
        return true;
    }
    if munaqqa.contains("\\C[") || munaqqa.contains("\\I[") || munaqqa.contains("\\V[") {
        return true;
    }
    let miftah_munkhafid = miftah.to_ascii_lowercase();
    ALFAZ_NASS
        .iter()
        .any(|lafz| miftah_munkhafid.contains(lafz))
}

/// Reads the user-facing plugin parameters out of `js/plugins.js`.
///
/// A project with no `plugins.js` — which happens when a repack strips it —
/// contributes nothing and is not an error.
///
/// # Errors
///
/// [`KhataNusus::HajmMufrit`], [`KhataNusus::NassGhayrSalih`] and
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] as the data files raise them. A
/// `$plugins` array that is valid JavaScript and not valid JSON is refused
/// here, because a splice into a literal this module could not read would be a
/// splice at a guessed offset.
pub fn istakhrij_mulhaqat(
    bunya: &BunyatMashru,
    sijill: &mut SijillNusus,
) -> Result<(), KhataNusus> {
    let nisbi = "js/plugins.js";
    let masar = bunya.malaf(nisbi);
    if !masar.is_file() {
        return Ok(());
    }
    let bayt = iqra_malaf(&masar, AQSA_MALAF)?;
    let (khaam, _) = nass_min_bayt(&bayt, nisbi)?;
    let Ok((bidaya, nihaya)) = mawqi_mulhaqat(&khaam) else {
        return Ok(());
    };
    let Some(masfufa) = khaam.get(bidaya..nihaya) else {
        return Ok(());
    };
    let jidhr: Value =
        serde_json::from_str(masfufa).map_err(|khata| KhataNusus::BunyaGhayrMutawaqqaa {
            malaf: nisbi.to_owned(),
            haql: format!("the $plugins array is not valid JSON: {khata}"),
        })?;
    let fahras = fahras_mawaqi(imsah_nusus(masfufa, nisbi)?);
    let mut jami = Jami {
        sijill: &mut *sijill,
        fahras,
        malaf: nisbi,
        izaha: bidaya,
    };

    if let Some(madakhil) = jidhr.as_array() {
        for (khana, madkhal) in madakhil.iter().enumerate() {
            let Some(barametr) = madkhal.get("parameters").and_then(Value::as_object) else {
                continue;
            };
            for (miftah, qeema) in barametr {
                if qeema.as_str().is_some_and(|nass| nass_wajih(miftah, nass)) {
                    let masar_dakhili = format!("/{khana}/parameters/{miftah}");
                    jami.daa(&masar_dakhili, NawMadkhal::BarametrMulhaq);
                }
            }
        }
    }
    if sijill.madakhil.iter().any(|madkhal| madkhal.malaf == nisbi) {
        let _ = sijill.malaffat.insert(nisbi.to_owned());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Asset obfuscation
//
// Called encryption by the editor and by everyone who has written about it, and
// it is not one. There is no cipher here and this module adds no cryptographic
// dependency: `System.json` publishes a sixteen byte key in plain hexadecimal,
// the file gets a sixteen byte header, and the file's own first sixteen bytes
// are XORed with that key. Everything after byte thirty-two is stored verbatim.
// Naming it honestly matters, because a reader who believes a cipher is
// involved will look for one and conclude this code is wrong.
// ---------------------------------------------------------------------------

/// Whether a project obfuscates its assets, and with what.
#[derive(Debug, Clone, Copy, Default)]
pub struct TashfeerMawarid {
    /// The masking key, when the project publishes one.
    pub miftah: Option<[u8; 16]>,
    /// Whether `img/` is obfuscated.
    pub suwar: bool,
    /// Whether `audio/` is obfuscated.
    pub aswat: bool,
}

impl TashfeerMawarid {
    /// Whether anything at all is obfuscated.
    ///
    /// False is the ordinary answer and means the patcher touches no asset
    /// bytes. Reporting that plainly is the point: a patcher that pretended to
    /// decrypt an unobfuscated file would be doing work that cannot be
    /// verified and hiding a wrong answer inside it.
    #[must_use]
    pub const fn mushaffar(&self) -> bool {
        self.suwar || self.aswat
    }
}

/// Reads the obfuscation state out of `data/System.json`.
///
/// # Errors
///
/// [`KhataNusus::MiftahMafqud`] when the project declares obfuscated assets and
/// no usable `encryptionKey` is present — which for this engine means the key
/// recovery failed, not that a key was withheld: Taarib reads the key the game
/// itself publishes and never tries to derive one it does not.
/// [`KhataNusus::KhataMalaf`], [`KhataNusus::HajmMufrit`],
/// [`KhataNusus::NassGhayrSalih`] and [`KhataNusus::BunyaGhayrMutawaqqaa`] when
/// `System.json` cannot be read or parsed.
pub fn iqra_tashfeer(bunya: &BunyatMashru) -> Result<TashfeerMawarid, KhataNusus> {
    let nisbi = "data/System.json";
    let masar = bunya.malaf(nisbi);
    let bayt = iqra_malaf(&masar, AQSA_MALAF)?;
    let (khaam, _) = nass_min_bayt(&bayt, nisbi)?;
    let jidhr: Value =
        serde_json::from_str(&khaam).map_err(|khata| KhataNusus::BunyaGhayrMutawaqqaa {
            malaf: nisbi.to_owned(),
            haql: format!("the document does not parse: {khata}"),
        })?;

    let suwar = jidhr
        .get("hasEncryptedImages")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let aswat = jidhr
        .get("hasEncryptedAudio")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let miftah = jidhr
        .get("encryptionKey")
        .and_then(Value::as_str)
        .and_then(miftah_min_nass);

    if (suwar || aswat) && miftah.is_none() {
        return Err(KhataNusus::MiftahMafqud {
            masar,
            sabab: "System.json declares obfuscated assets and its encryptionKey is absent or \
                    is not thirty-two hexadecimal digits"
                .to_owned(),
        });
    }
    Ok(TashfeerMawarid {
        miftah,
        suwar,
        aswat,
    })
}

/// Reads the thirty-two hexadecimal digits `System.json` publishes as sixteen
/// bytes.
///
/// Anything else — the wrong length, a non-hexadecimal digit, a key the editor
/// left as an empty string — is no key rather than a partial one.
#[must_use]
pub fn miftah_min_nass(sittasi: &str) -> Option<[u8; 16]> {
    let munaqqa = sittasi.trim();
    if munaqqa.len() != 32 || !munaqqa.is_ascii() {
        return None;
    }
    let mut miftah = [0_u8; 16];
    for (khana, zawj) in munaqqa.as_bytes().chunks_exact(2).enumerate() {
        let nass = std::str::from_utf8(zawj).ok()?;
        let qeema = u8::from_str_radix(nass, 16).ok()?;
        *miftah.get_mut(khana)? = qeema;
    }
    Some(miftah)
}

/// The signatures a de-obfuscated MV or MZ asset can begin with.
///
/// Used to prove the recovered key actually decodes the file rather than
/// assuming it. A wrong key produces sixteen bytes of noise followed by a
/// perfectly intact PNG body, which an image loader reports as a corrupt file
/// somewhere deep in a game session; catching it here names the real cause.
const TAWAQI_MAWARID: [&[u8]; 5] = [
    &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
    b"OggS",
    b"RIFF",
    b"ID3",
    &[0xFF, 0xFB],
];

/// Removes the obfuscation from one asset.
///
/// # Errors
///
/// [`KhataNusus::MalafQaseer`] when the file is shorter than the sixteen byte
/// header it claims, and [`KhataNusus::MiftahGhayrSalih`] when the header is
/// not the one this scheme writes or when the recovered key does not produce a
/// file that starts like any format the scheme covers.
pub fn fukk_tashfeer(bayt: &[u8], miftah: [u8; 16], masar: &Path) -> Result<Vec<u8>, KhataNusus> {
    let Some(jism) = bayt.get(TUL_QINAA..) else {
        return Err(KhataNusus::MalafQaseer {
            haql: "the obfuscation header",
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(TUL_QINAA),
        });
    };
    if bayt.get(..5) != TARWISAT_TASHFEER.get(..5) {
        return Err(KhataNusus::MiftahGhayrSalih {
            masar: masar.to_path_buf(),
            sabab: "the file does not begin with the RPGMV obfuscation header, so the key \
                    System.json publishes does not apply to it",
        });
    }
    let mut maftuh = jism.to_vec();
    for (khana, bayt_hali) in maftuh.iter_mut().enumerate().take(TUL_QINAA) {
        if let Some(qinaa) = miftah.get(khana) {
            *bayt_hali ^= *qinaa;
        }
    }
    let maruf = TAWAQI_MAWARID
        .iter()
        .any(|tawqee| maftuh.starts_with(tawqee))
        || maftuh.get(4..8) == Some(b"ftyp");
    if !maruf {
        return Err(KhataNusus::MiftahGhayrSalih {
            masar: masar.to_path_buf(),
            sabab: "the recovered key produced bytes that begin like no image or audio format \
                    this scheme covers",
        });
    }
    Ok(maftuh)
}

/// Puts the obfuscation back, so a patched asset loads through the game's own
/// loader unchanged.
///
/// The inverse of [`fukk_tashfeer`] exactly: the same header, the same sixteen
/// masked bytes, the same verbatim tail. A patched game whose assets were
/// obfuscated stays obfuscated, because turning it off would mean rewriting
/// `System.json`'s flags and every asset in the game to match.
#[must_use]
pub fn shaffir(bayt: &[u8], miftah: [u8; 16]) -> Vec<u8> {
    let mut mabni = Vec::with_capacity(bayt.len().saturating_add(TUL_QINAA));
    mabni.extend_from_slice(&TARWISAT_TASHFEER);
    mabni.extend_from_slice(bayt);
    for (khana, bayt_hali) in mabni.iter_mut().skip(TUL_QINAA).enumerate().take(TUL_QINAA) {
        if let Some(qinaa) = miftah.get(khana) {
            *bayt_hali ^= *qinaa;
        }
    }
    mabni
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

// Every write in this module goes through `Hafiz`, the crate's single
// reversibility guard. Nothing here calls `std::fs::write`, `File::create` or
// `fs::remove_file`. That is not a style preference. This crate modifies files
// that *are* the game, and a write that bypassed the guard would be a
// modification with no recorded original — a patch that can install and cannot
// uninstall, which is a patch that has taken somebody's game hostage.

/// Translations, keyed by [`MadkhalNusus::huwiya`].
///
/// The value is the **raw** replacement — markup included, exactly as it should
/// appear in the game's data file. Reinsertion of escape codes into a
/// translated line is the compiler's job, not this module's: a code's position
/// inside a sentence is a translation decision, and transplanting byte offsets
/// from one language onto another would put an icon in the middle of a word.
pub type Tarjamat = BTreeMap<String, String>;

/// What one install did.
#[derive(Debug, Clone, Default)]
pub struct TaqreerTarkeeb {
    /// How many game files were rewritten.
    pub malaffat: usize,
    /// How many strings were replaced.
    pub nusus: usize,
    /// How many extracted strings had no translation and were left alone.
    pub matruka: usize,
}

/// The escape codes whose loss in translation is a defect rather than a choice.
///
/// An icon, a variable, an actor's name and the currency unit each stand for
/// something the engine substitutes at draw time. A translation that dropped
/// one silently loses information the player was meant to see. Colour changes
/// and pauses are not on this list: a translator moving or removing them is
/// making a legitimate typographic decision about a different language.
const HURUB_LAZIMA: [NawHarf; 5] = [
    NawHarf::Ayqouna,
    NawHarf::Mutaghayyir,
    NawHarf::IsmBatal,
    NawHarf::IsmRafiq,
    NawHarf::Umla,
];

/// Checks that a translation kept every code that carries information.
///
/// # Errors
///
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] naming the identity and the code that
/// went missing or appeared. Refused rather than repaired: putting a `\V[7]`
/// back at a guessed position would be this module deciding where a variable
/// belongs in a sentence it cannot read.
pub fn tahaqquq_hurub(madkhal: &MadkhalNusus, badeel: &str) -> Result<(), KhataNusus> {
    let jadeed = iqsim_hurub(badeel);
    for naw in HURUB_LAZIMA {
        let asl = madkhal
            .nitaqat
            .iter()
            .filter(|nitaq| nitaq.naw == naw)
            .count();
        let baad = jadeed
            .nitaqat
            .iter()
            .filter(|nitaq| nitaq.naw == naw)
            .count();
        if asl != baad {
            return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: madkhal.malaf.clone(),
                haql: format!(
                    "{}: the source carries {asl} {} code(s) and the translation carries {baad}",
                    madkhal.masar,
                    naw.ism()
                ),
            });
        }
    }
    Ok(())
}

/// Writes every translation into the project's own data files.
///
/// One splice per file, one round-trip proof per file, and one write through
/// the guard per file. A file whose proof fails is not written and neither is
/// any file after it, because a half-applied patch across a data directory is
/// worse than none.
///
/// # Errors
///
/// [`KhataNusus::DawraGhayrMutabaqa`] when a splice changed a byte it was not
/// asked to, [`KhataNusus::BunyaGhayrMutawaqqaa`] when a translation lost a
/// code that carries information or a file has drifted from its extraction
/// record, and the reading and writing errors those operations raise.
pub fn rakkib(
    bunya: &BunyatMashru,
    sijill: &SijillNusus,
    tarjamat: &Tarjamat,
    hafiz: &mut dyn Hafiz,
) -> Result<TaqreerTarkeeb, KhataNusus> {
    let mut taqreer = TaqreerTarkeeb::default();
    for nisbi in &sijill.malaffat {
        let masar = bunya.malaf(nisbi);
        let bayt = iqra_malaf(&masar, AQSA_MALAF)?;
        let (khaam, alama) = nass_min_bayt(&bayt, nisbi)?;

        let mut tabdilat: Vec<Tabdil> = Vec::new();
        for madkhal in sijill.li_malaf(nisbi) {
            let Some(badeel) = tarjamat.get(&madkhal.huwiya) else {
                taqreer.matruka = taqreer.matruka.saturating_add(1);
                continue;
            };
            // The extraction record carries the literal it was taken from. If
            // the file has changed under the patch — an updater, an editor
            // save — the bytes at that offset are no longer that literal, and
            // splicing there would write into the middle of something else.
            //
            // Compared by decoded value rather than by literal, because the
            // editor's escaping choices are its own — `\/` and `é` are
            // both ordinary in these files — and a literal comparison would
            // reject a file that has not changed at all.
            let asli = khaam
                .get(madkhal.bidaya..madkhal.nihaya)
                .unwrap_or_default();
            let mutabiq = imsah_nusus(asli, nisbi)
                .ok()
                .and_then(|mawaqi| mawaqi.into_iter().next())
                .is_some_and(|mawqi| mawqi.qeema == madkhal.khaam);
            if !mutabiq {
                return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                    malaf: nisbi.clone(),
                    haql: format!(
                        "{}: the file no longer holds the string this patch was built against",
                        madkhal.masar
                    ),
                });
            }
            tahaqquq_hurub(madkhal, badeel)?;
            tabdilat.push(Tabdil {
                bidaya: madkhal.bidaya,
                nihaya: madkhal.nihaya,
                badeel: iqtibas_json(badeel),
            });
            taqreer.nusus = taqreer.nusus.saturating_add(1);
        }
        if tabdilat.is_empty() {
            continue;
        }

        let jadeed = khayyit(&khaam, &tabdilat)?;
        tahaqquq_dawra(&khaam, &jadeed, &tabdilat)?;

        let mut makhraj: Vec<u8> = Vec::with_capacity(jadeed.len().saturating_add(3));
        if alama {
            makhraj.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        makhraj.extend_from_slice(jadeed.as_bytes());
        hafiz.iktub(&masar, &makhraj)?;
        taqreer.malaffat = taqreer.malaffat.saturating_add(1);
    }
    Ok(taqreer)
}

// ---------------------------------------------------------------------------
// Delivery
// ---------------------------------------------------------------------------

/// The plugin's name in the engine's plugin list, and its file stem.
///
/// Lowercase because the file is `js/plugins/taarib.js` and RPG Maker matches
/// the two by string equality on case-sensitive filesystems.
pub const ISM_MULHAQ: &str = "taarib";

/// Where the plugin's own files live, relative to the project.
pub const MUJALLAD_MULHAQ: &str = "js/plugins/taarib";

/// What the patcher tells the plugin about the game it was installed into.
///
/// Every value is recorded here at patch time and read by the plugin at
/// startup. The plugin decides nothing for itself — in particular it does not
/// re-run the tier probe, because the probe reads files that are no longer
/// there in the same state once a patch has been installed, and two answers to
/// one question is one answer too many.
#[derive(Debug, Clone)]
pub struct IdadatMulhaq {
    /// The rung the probe established. The plugin branches on this and on
    /// nothing else.
    pub rutba: Rutba,
    /// The font file's name inside [`MUJALLAD_MULHAQ`].
    pub khatt: String,
    /// The patch data file's name inside [`MUJALLAD_MULHAQ`].
    pub bayanat: String,
    /// The language discriminant, as `taarib-wasm`'s `LughaNass` numbers it.
    pub lugha: u32,
    /// The digit policy, as `SiyasatArqam` numbers it.
    pub arqam: u32,
    /// The diacritic policy, as `SiyasatTashkeel` numbers it.
    pub tashkeel: u32,
    /// The justification mode, as `NamatDabt` numbers it.
    pub dabt: u32,
    /// Whether window layouts are mirrored right to left.
    pub nawafidh_yameen: bool,
}

impl IdadatMulhaq {
    /// The settings as the `parameters` object of a plugin list entry.
    ///
    /// Every value is a string, because that is the only type RPG Maker's
    /// plugin manager stores and the only type its `PluginManager.parameters`
    /// hands back. The plugin parses them itself, from a fixed vocabulary,
    /// never by evaluating them.
    #[must_use]
    pub fn barametr(&self) -> String {
        let mut makhtut = String::from("{");
        let mut zawj = |ism: &str, qeema: &str, awwal: bool| {
            if !awwal {
                makhtut.push(',');
            }
            makhtut.push_str(&iqtibas_json(ism));
            makhtut.push(':');
            makhtut.push_str(&iqtibas_json(qeema));
        };
        zawj("rutba", &(self.rutba as u8).to_string(), true);
        zawj("khatt", &self.khatt, false);
        zawj("bayanat", &self.bayanat, false);
        zawj("lugha", &self.lugha.to_string(), false);
        zawj("arqam", &self.arqam.to_string(), false);
        zawj("tashkeel", &self.tashkeel.to_string(), false);
        zawj("dabt", &self.dabt.to_string(), false);
        zawj(
            "nawafidhYameen",
            if self.nawafidh_yameen {
                "true"
            } else {
                "false"
            },
            false,
        );
        makhtut.push('}');
        makhtut
    }
}

/// The files an install places into the project.
#[derive(Debug, Clone, Default)]
pub struct HimlMulhaq {
    /// The compiled plugin, written to `js/plugins/taarib.js`.
    pub mulhaq: Vec<u8>,
    /// Everything else, by name inside [`MUJALLAD_MULHAQ`] — the font, the
    /// patch data, and on the takeover rung the WebAssembly core and its glue.
    pub mawarid: Vec<(String, Vec<u8>)>,
}

/// Places the plugin's files and registers it in the engine's own plugin list.
///
/// Additive in both halves: a file that did not exist and one entry appended to
/// an array. Uninstalling is deleting the file and the entry, which is why the
/// entry is appended rather than merged into anything and why nothing else in
/// `plugins.js` is rewritten.
///
/// Appended *last*, deliberately. The plugin overrides the window text chain,
/// and a plugin that loaded before another plugin that overrides the same chain
/// would have its override replaced.
///
/// Run this **after** [`rakkib`], never before. Extraction records carry byte
/// offsets into `js/plugins.js`, and appending an entry moves nothing before
/// the closing bracket but does move the file's length; running the two in the
/// other order would leave [`rakkib`] checking offsets against a file it did
/// not measure. It would notice and refuse, which is the safe failure and still
/// a wasted install.
///
/// # Errors
///
/// [`KhataNusus::HimlMarfud`] when a payload name is not a plain relative name,
/// when `plugins.js` has no readable `$plugins` array, or when an entry named
/// [`ISM_MULHAQ`] is already registered — all three are rungs declining, and
/// the caller can descend or ask the player to uninstall the previous patch.
/// Plus whatever reading `plugins.js` and writing through the guard raise.
pub fn rakkib_mulhaq(
    bunya: &BunyatMashru,
    himl: &HimlMulhaq,
    idadat: &IdadatMulhaq,
    hafiz: &mut dyn Hafiz,
) -> Result<(), KhataNusus> {
    for (ism, _) in &himl.mawarid {
        if !ism_amin(ism) {
            return Err(KhataNusus::HimlMarfud {
                alia: "js/plugins/taarib",
                sabab: format!("{ism} is not a plain relative file name"),
            });
        }
    }

    hafiz.ansha(&bunya.malaf("js/plugins/taarib.js"), &himl.mulhaq)?;
    for (ism, bayt) in &himl.mawarid {
        hafiz.ansha(&bunya.malaf(&format!("{MUJALLAD_MULHAQ}/{ism}")), bayt)?;
    }
    sajjil_mulhaq(bunya, idadat, hafiz)
}

/// Whether a payload name is a plain relative name this module will write.
///
/// No absolute path, no drive letter, no backslash, no `..`, no leading dot.
/// The payload comes out of a patch file a stranger produced, and a name is the
/// one field in it that decides where bytes land on somebody's disk.
fn ism_amin(ism: &str) -> bool {
    if ism.is_empty() || ism.len() > 128 {
        return false;
    }
    if ism.starts_with('/') || ism.starts_with('.') || ism.contains('\\') || ism.contains(':') {
        return false;
    }
    ism.split('/')
        .all(|juz| !juz.is_empty() && juz != "." && juz != "..")
}

/// Appends the plugin's entry to `$plugins`, leaving every other byte alone.
///
/// # Errors
///
/// As [`rakkib_mulhaq`] documents.
pub fn sajjil_mulhaq(
    bunya: &BunyatMashru,
    idadat: &IdadatMulhaq,
    hafiz: &mut dyn Hafiz,
) -> Result<(), KhataNusus> {
    let nisbi = "js/plugins.js";
    let masar = bunya.malaf(nisbi);
    let bayt = iqra_malaf(&masar, AQSA_MALAF)?;
    let (khaam, alama) = nass_min_bayt(&bayt, nisbi)?;
    let (bidaya, nihaya) = mawqi_mulhaqat(&khaam)?;
    let Some(masfufa) = khaam.get(bidaya..nihaya) else {
        return Err(KhataNusus::HimlMarfud {
            alia: nisbi,
            sabab: "the $plugins array is not on a character boundary".to_owned(),
        });
    };
    let madakhil: Value =
        serde_json::from_str(masfufa).map_err(|khata| KhataNusus::HimlMarfud {
            alia: nisbi,
            sabab: format!("the $plugins array is not valid JSON: {khata}"),
        })?;
    let musajjal = madakhil.as_array().is_some_and(|qaima| {
        qaima
            .iter()
            .any(|madkhal| madkhal.get("name").and_then(Value::as_str) == Some(ISM_MULHAQ))
    });
    if musajjal {
        return Err(KhataNusus::HimlMarfud {
            alia: nisbi,
            sabab: format!(
                "an entry named {ISM_MULHAQ} is already registered; uninstall the previous \
                 patch before installing this one"
            ),
        });
    }

    // The insertion point is just before the closing bracket. The comma is
    // needed only when the array already holds something.
    let mughliq = nihaya.saturating_sub(1);
    let sabiq = khaam.get(bidaya..mughliq).unwrap_or_default();
    let khali = sabiq.trim_end().ends_with('[');
    let fasila = if khali { "" } else { "," };
    let madkhal = format!(
        "{fasila}\n{{\"name\":{},\"status\":true,\"description\":{},\"parameters\":{}}}\n",
        iqtibas_json(ISM_MULHAQ),
        iqtibas_json(WASF_MULHAQ),
        idadat.barametr()
    );

    let mut makhraj: Vec<u8> = Vec::with_capacity(bayt.len().saturating_add(madkhal.len()));
    if alama {
        makhraj.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    let qabl = khaam.get(..mughliq).unwrap_or_default();
    let baad = khaam.get(mughliq..).unwrap_or_default();
    makhraj.extend_from_slice(qabl.as_bytes());
    makhraj.extend_from_slice(madkhal.as_bytes());
    makhraj.extend_from_slice(baad.as_bytes());
    hafiz.iktub(&masar, &makhraj)
}

/// What the plugin manager shows beside the entry.
const WASF_MULHAQ: &str = "تعريب — Arabic text for RPG Maker MV and MZ. Installed by Taarib; remove this entry \
     and js/plugins/taarib.js to uninstall.";

// ---------------------------------------------------------------------------
// The tier probe
// ---------------------------------------------------------------------------

/// The most bytes read from one JavaScript file while probing.
const AQSA_BARMAJI: u64 = 8 * 1024 * 1024;

/// How many enabled plugins are opened while probing.
///
/// A project with sixty plugins is ordinary and one with three hundred is a
/// project somebody has lost control of. Reading past this many would make the
/// probe slower than the patch.
const AQSA_MULHAQAT: usize = 128;

/// The RPG Maker MV and MZ tier probe.
///
/// Answers one question from the shipped files: does what this game actually
/// runs put whole strings in front of the canvas, or one character at a time?
/// Everything else here serves that question.
#[derive(Debug, Clone, Copy, Default)]
pub struct MifhasRpgMaker;

impl Mifhas for MifhasRpgMaker {
    fn hadaf(&self) -> HadafNusus {
        HadafNusus::RpgMakerJs
    }

    fn yantabiq(&self, siyaq: &SiyaqTabaqa<'_>) -> bool {
        if matches!(
            siyaq.aila,
            AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz
        ) {
            return true;
        }
        // Four metadata queries at most, and no file is opened. Cheap enough to
        // run on every game, which is what the trait requires.
        QAWAID
            .iter()
            .any(|asas| isdar_taht(siyaq.jidhr, asas).is_some())
    }

    fn adilla(&self, siyaq: &SiyaqTabaqa<'_>) -> Result<Vec<Dalil>, KhataNusus> {
        let bunya = BunyatMashru::iktashif(siyaq.jidhr)?;
        let mut adilla: Vec<Dalil> = Vec::new();

        adilla.push(Dalil::siyaq(
            "nusus:rpgmaker/layout",
            format!(
                "{}, {}, project at {}",
                bunya.isdar().ism(),
                if bunya.manshur() {
                    "deployed"
                } else {
                    "an editor project folder"
                },
                if bunya.asas().is_empty() {
                    "the game root"
                } else {
                    bunya.asas()
                }
            ),
        ));

        jeel_chromium(siyaq.jidhr, &mut adilla);
        let maqtua = mulhaqat_tatajawaz(&bunya, &mut adilla);
        silsilat_nass(&bunya, maqtua, &mut adilla);
        khatt_muallan(&bunya, &mut adilla);
        Ok(adilla)
    }
}

/// Gathers the evidence and weighs it into a verdict.
///
/// # Errors
///
/// [`KhataNusus::TabaqaMajhula`] when nothing observed pointed at a rung or the
/// two leading readings are too close to act on — which for this engine is the
/// real answer for a game whose core scripts were bundled into one minified
/// file and whose message chain is replaced by a plugin nobody can read.
/// Plus whatever [`Mifhas::adilla`] raises.
pub fn tabaqat(siyaq: &SiyaqTabaqa<'_>) -> Result<Hukm, KhataNusus> {
    let mifhas = MifhasRpgMaker;
    let adilla = mifhas.adilla(siyaq)?;
    hukm(siyaq, HadafNusus::RpgMakerJs, adilla)
}

/// Reads the Chromium generation off the shell's own runtime files.
///
/// Not an NW.js version and not read as one. These are Chromium's own history,
/// which every shell inherits: `natives_blob.bin` was removed in Chromium 74
/// and the `swiftshader/` directory became a single `vk_swiftshader` library
/// around Chromium 90. Every generation in that sequence shapes Arabic on a
/// canvas — `HarfBuzz` has been in Chromium's font stack far longer than any of
/// them — so this evidence points at corrected shaping rather than at takeover,
/// with the weight falling as the generation gets older and its handling of
/// mixed-direction runs less trustworthy.
///
/// The names are case-folded like every other lookup into a game directory here.
/// A miss costs an evidence sentence rather than an install, but a probe that
/// reported "no shell runtime files are present" for a repack that capitalised
/// `Swiftshader/` would be a probe stating something untrue about a directory it
/// had just read, and that is worse than the five extra listings it avoids.
fn jeel_chromium(jidhr: &Path, adilla: &mut Vec<Dalil>) {
    const HADEETHA: [&str; 3] = [
        "vk_swiftshader.dll",
        "vk_swiftshader_icd.json",
        "libvk_swiftshader.so",
    ];
    if let Some(ism) = HADEETHA
        .iter()
        .find(|ism| masar_bila_hala(jidhr, ism).is_file())
    {
        adilla.push(Dalil::jadeed(
            format!("nusus:rpgmaker/{ism}"),
            "the shell is Chromium 90 or newer, whose canvas fillText shapes Arabic and \
             resolves mixed-direction runs",
            Some(Rutba::Tasheeh),
            30,
        ));
        return;
    }
    if masar_bila_hala(jidhr, "natives_blob.bin").is_file() {
        adilla.push(Dalil::jadeed(
            "nusus:rpgmaker/natives_blob.bin",
            "the shell is Chromium 73 or older, which shapes Arabic on a canvas and is less \
             dependable on mixed-direction runs",
            Some(Rutba::Tasheeh),
            25,
        ));
        return;
    }
    if masar_bila_hala(jidhr, "swiftshader").is_dir() {
        adilla.push(Dalil::jadeed(
            "nusus:rpgmaker/swiftshader",
            "the shell is Chromium 74 to 89, whose canvas fillText shapes Arabic",
            Some(Rutba::Tasheeh),
            28,
        ));
        return;
    }
    adilla.push(Dalil::jadeed(
        "nusus:rpgmaker/shell",
        "no shell runtime files are present, so the game is opened by whatever browser the \
         player has, every one of which shapes Arabic on a canvas",
        Some(Rutba::Tasheeh),
        18,
    ));
}

/// Reads at most `hadd` bytes of a file as text, for probing only.
///
/// A file that is missing, denied or not UTF-8 is the absence of evidence
/// rather than an error: the probe records what it did see and lets [`hukm`]
/// refuse if that turns out to be nothing.
fn iqra_muhaddad(masar: &Path, hadd: u64) -> Option<String> {
    let malaf = fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(hadd).read_to_end(&mut bayt).ok()?;
    let jism = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bayt);
    String::from_utf8(jism.to_vec()).ok()
}

/// Looks at what the enabled plugins do to the text chain.
///
/// Returns whether the chain itself has been replaced. When it has, the core
/// script's own chain is no longer what runs, and the evidence drawn from it is
/// recorded at a reduced weight rather than thrown away — the stock chain is
/// still what a plugin's override usually calls through to, and pretending to
/// know nothing would be as wrong as pretending to know everything.
///
/// A plugin that wraps `drawTextEx` is deliberately not counted. Wrapping
/// `drawTextEx` to prepend a name box or a portrait is one of the commonest
/// things a plugin does and leaves the drawing chain underneath it untouched.
fn mulhaqat_tatajawaz(bunya: &BunyatMashru, adilla: &mut Vec<Dalil>) -> bool {
    const SILSILA: [&str; 3] = [
        "Window_Base.prototype.processCharacter",
        "Window_Base.prototype.processNormalCharacter",
        "Window_Base.prototype.flushTextState",
    ];
    let Some(khaam) = iqra_muhaddad(&bunya.malaf("js/plugins.js"), AQSA_BARMAJI) else {
        adilla.push(Dalil::siyaq(
            "nusus:rpgmaker/js/plugins.js",
            "the plugin list could not be read, so what the plugins do to the text chain is \
             unknown",
        ));
        return false;
    };
    let asmaa = asmaa_mufaala(&khaam);
    adilla.push(Dalil::siyaq(
        "nusus:rpgmaker/js/plugins.js",
        format!("{} enabled plugin(s) are registered", asmaa.len()),
    ));

    let mut maqtua: Vec<String> = Vec::new();
    let mut nuqta: Vec<String> = Vec::new();
    for ism in asmaa.iter().take(AQSA_MULHAQAT) {
        let masar = bunya.malaf(&format!("js/plugins/{ism}.js"));
        let Some(nass) = iqra_muhaddad(&masar, AQSA_BARMAJI) else {
            continue;
        };
        if SILSILA.iter().any(|alama| nass.contains(alama)) {
            maqtua.push(ism.clone());
        }
        if nass.contains("Bitmap.prototype.drawText") {
            nuqta.push(ism.clone());
        }
    }

    if !nuqta.is_empty() {
        adilla.push(Dalil::jadeed(
            "nusus:rpgmaker/plugins",
            format!(
                "an enabled plugin replaces Bitmap.prototype.drawText, so what reaches the \
                 canvas is no longer the engine's own call: {}",
                nuqta.join(", ")
            ),
            Some(Rutba::Istila),
            55,
        ));
    }
    if maqtua.is_empty() {
        return false;
    }
    adilla.push(Dalil::siyaq(
        "nusus:rpgmaker/plugins",
        format!(
            "an enabled plugin replaces the Window_Base text chain, so the core script's own \
             chain is not what runs and the evidence from it is weighted down: {}",
            maqtua.join(", ")
        ),
    ));
    true
}

/// The names of the enabled entries in a `plugins.js`.
///
/// Falls back to counting `"name"` members when the array is valid JavaScript
/// and not valid JSON, which a hand-edited `plugins.js` frequently is and which
/// the game itself runs perfectly well, because the engine evaluates this file
/// rather than parsing it.
fn asmaa_mufaala(khaam: &str) -> Vec<String> {
    let Ok((bidaya, nihaya)) = mawqi_mulhaqat(khaam) else {
        return Vec::new();
    };
    let Some(masfufa) = khaam.get(bidaya..nihaya) else {
        return Vec::new();
    };
    let Ok(Value::Array(qeem)) = serde_json::from_str::<Value>(masfufa) else {
        return Vec::new();
    };
    qeem.iter()
        .filter(|madkhal| madkhal.get("status").and_then(Value::as_bool) == Some(true))
        .filter_map(|madkhal| madkhal.get("name").and_then(Value::as_str))
        .filter(|ism| ism_amin(&format!("{ism}.js")))
        .map(str::to_owned)
        .collect()
}

/// Reads the shipped core scripts' own text chain.
///
/// This is the decisive observation and the reason this probe exists. MV's
/// `Window_Base.prototype.processNormalCharacter` takes one character off the
/// text state and hands that single character to `drawText`, which is why a
/// stock MV message window cannot join Arabic however good the browser
/// underneath it is. MZ's chain buffers characters and flushes whole runs
/// through `flushTextState`, so the same browser joins them correctly, and its
/// text state even carries a direction flag. Both facts are read off the file
/// rather than assumed from the product name, because a game that replaced its
/// core script is a game where the product name has stopped being evidence.
fn silsilat_nass(bunya: &BunyatMashru, maqtua: bool, adilla: &mut Vec<Dalil>) {
    let wazn = |kamil: u8| if maqtua { 40 } else { kamil };
    let nisbi = bunya.isdar().nawat_nawafidh();
    match iqra_muhaddad(&bunya.malaf(nisbi), AQSA_BARMAJI) {
        Some(nass) => {
            if nass.contains("flushTextState") {
                adilla.push(Dalil::jadeed(
                    format!("nusus:rpgmaker/{nisbi}"),
                    "the window text chain buffers characters and flushes whole runs through \
                     one drawText, so the canvas shapes what it is given",
                    Some(Rutba::Tasheeh),
                    wazn(80),
                ));
            } else if nass.contains("processNormalCharacter") {
                adilla.push(Dalil::jadeed(
                    format!("nusus:rpgmaker/{nisbi}"),
                    "the window text chain draws one character per call, which leaves the \
                     shaper no neighbours to join to however capable the browser is",
                    Some(Rutba::Istila),
                    wazn(85),
                ));
            } else {
                adilla.push(Dalil::siyaq(
                    format!("nusus:rpgmaker/{nisbi}"),
                    "the core script is present and its text chain could not be identified; it \
                     may have been bundled, minified or replaced",
                ));
            }
            if nass.contains("createTextBuffer") {
                adilla.push(Dalil::jadeed(
                    format!("nusus:rpgmaker/{nisbi}"),
                    "the text state carries a right-to-left flag, so the engine already knows \
                     the concept and needs its direction and alignment corrected rather than \
                     its drawing replaced",
                    Some(Rutba::Tasheeh),
                    25,
                ));
            }
        },
        None => adilla.push(Dalil::siyaq(
            format!("nusus:rpgmaker/{nisbi}"),
            "the window core script could not be read, so the engine's own text chain is \
             unknown",
        )),
    }

    let asas = bunya.isdar().nawat_asas();
    if let Some(nass) = iqra_muhaddad(&bunya.malaf(asas), AQSA_BARMAJI)
        && nass.contains("Bitmap.prototype.drawText")
        && nass.contains("fillText")
    {
        adilla.push(Dalil::jadeed(
            format!("nusus:rpgmaker/{asas}"),
            "Bitmap.prototype.drawText is still the canvas fillText, so every menu, command \
             and item name in this game already shapes",
            Some(Rutba::Tasheeh),
            20,
        ));
    }
}

/// Records the font the runtime loads, as context.
///
/// Never evidence for a rung. A game that bundles a font with no Arabic
/// coverage still shapes or fails to shape for reasons that have nothing to do
/// with which font is named; what this changes is whether the patch has to
/// install its own font, which every rung does anyway.
fn khatt_muallan(bunya: &BunyatMashru, adilla: &mut Vec<Dalil>) {
    let Some(khaam) = iqra_muhaddad(&bunya.malaf("data/System.json"), AQSA_BARMAJI) else {
        return;
    };
    let Ok(jidhr) = serde_json::from_str::<Value>(&khaam) else {
        return;
    };
    let mutaqaddim = jidhr.get("advanced");
    let ism = mutaqaddim
        .and_then(|kutla| kutla.get("mainFontFilename"))
        .and_then(Value::as_str);
    let hajm = mutaqaddim
        .and_then(|kutla| kutla.get("fontSize"))
        .and_then(Value::as_i64);
    if let Some(malaf) = ism {
        adilla.push(Dalil::siyaq(
            "nusus:rpgmaker/data/System.json",
            format!(
                "the runtime loads {malaf} at size {}",
                hajm.unwrap_or_default()
            ),
        ));
    }
    let suwar = jidhr
        .get("hasEncryptedImages")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let aswat = jidhr
        .get("hasEncryptedAudio")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if suwar || aswat {
        adilla.push(Dalil::siyaq(
            "nusus:rpgmaker/data/System.json",
            format!(
                "assets are obfuscated (images: {suwar}, audio: {aswat}) and the masking key \
                 is published in System.json"
            ),
        ));
    }
}
