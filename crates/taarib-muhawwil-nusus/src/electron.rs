//! غلاف — Electron and NW.js: read the `asar`, put the payload inside it, and
//! then get out of the way.
//!
//! This is the shortest adapter in the crate, and the reason is worth stating
//! before any code: **the renderer is Chromium.** Chromium shapes Arabic with
//! `HarfBuzz` and runs the full Unicode bidirectional algorithm on every text
//! node it lays out, and it has been doing both, against every font on the web,
//! for longer than this project has existed. Nothing here can improve on it.
//! So the whole job for an ordinary Electron game is three things — the font,
//! the direction, the text — and this module deliberately does not build the
//! machinery it would need for anything more.
//!
//! Rung one is therefore the expected verdict, and [`MifhasGhilaf`] exists to
//! *check* that rather than to assert it, because there is one real exception.
//! A game that paints its dialogue onto a `<canvas>` with
//! `CanvasRenderingContext2D.fillText` still gets shaping — canvas text goes
//! through the same shaper the layout engine uses — but a game that draws each
//! character as a separate sprite, or blits a per-character bitmap font, or
//! ships its own layout library, gets none of it. Those games are rung three,
//! and finding one is surprising enough that the probe records the evidence
//! that led there rather than just the answer.
//!
//! ## The asar archive, byte for byte
//!
//! An `app.asar` is a Chromium `Pickle` holding a JSON directory, followed by
//! every packed file's bytes concatenated together.
//!
//! ```text
//! app.asar
//!
//!   offset  size          field            meaning
//!        0     4          hajm_itar        u32 LE, always 4: the size pickle's
//!                                          payload is one u32
//!        4     4          tul_tarwisa      u32 LE: bytes in the header pickle,
//!                                          = 8 + align4(tul_json)
//!        8     4          hajm_hamula      u32 LE: the header pickle's payload,
//!                                          = 4 + align4(tul_json)
//!       12     4          tul_json         u32 LE: bytes of JSON, unpadded
//!       16     tul_json   shajara          the directory tree, UTF-8 JSON
//!       16+n   hashw      hashw            zero bytes up to a 4-byte boundary
//!   16+align4(tul_json)   muhtawa          every packed file's bytes, in the
//!                                          order the packer wrote them
//! ```
//!
//! Two details in that table are where readers go wrong.
//!
//! **The padding rule applies to the header and to nothing else.** `writeString`
//! in Chromium's pickle rounds the string up to a four-byte boundary, which is
//! why `tul_tarwisa` is `8 + align4(tul_json)` and why the body region starts at
//! `16 + align4(tul_json)` rather than `16 + tul_json`. File bodies inside
//! `muhtawa` are **not** padded and **not** aligned: entry *n+1* begins exactly
//! where entry *n* ended, in packing order — which is not the order the JSON
//! lists them in, and must never be assumed to be.
//!
//! **`offset` is a string.** This is the single most misparsed field in the
//! format. The directory says `{"size":1234,"offset":"5678"}` — `size` is a JSON
//! number and `offset` is a JSON *string* of decimal digits, because the format
//! predates `BigInt` and an archive can exceed the 2^53 a JSON number holds
//! exactly. A reader that calls `as_u64()` on it gets `None`, concludes the
//! entry is a directory, and silently drops every file in the archive. This
//! module parses it as text and refuses anything that is not decimal digits.
//!
//! ## Entries that are not packed bytes
//!
//! * `"unpacked": true` — the file lives outside the archive, in an
//!   `app.asar.unpacked/` directory beside it, at the same relative path. Such
//!   an entry carries no `offset`. Native modules (`.node`) and anything the
//!   application `fs.realpath`s are packed this way. These files are **left
//!   entirely alone**: this module records where they are and never reads,
//!   rewrites or repacks them, because a `.node` moved into the archive is a
//!   native module the loader can no longer `dlopen`.
//! * `"link": "other/path"` — a symbolic link inside the archive.
//! * `"executable": true` — a packed helper binary; preserved as declared.
//! * `"integrity": {...}` — Electron 16 and later can hash the archive and check
//!   it against a value fused into the executable. That value is outside the
//!   archive and cannot be rewritten from here, so an archive carrying integrity
//!   blocks is refused for repacking with [`KhataNusus::HimlMarfud`], naming the
//!   unpacked `resources/app/` route as the alternative. Silently repacking one
//!   produces a game that will not launch.
//!
//! ## What is guaranteed byte-exact, and what is not
//!
//! The body region of the original archive is **never rewritten**. New and
//! replaced files are appended past its end and only their own directory entries
//! change; every other entry keeps the offset and the bytes it had. Before
//! writing, the rebuilt archive is re-parsed *out of the buffer that is about to
//! be written* and every untouched entry is compared against the original byte
//! for byte; a difference is [`KhataNusus::DawraGhayrMutabaqa`] with the count,
//! raised at a moment when nothing has been written at all.
//!
//! The *header* is regenerated, necessarily — it carries the offsets that just
//! changed — so this module does not claim a byte-identical header and does not
//! pretend to. Reversibility comes from the untouched original being preserved
//! before the first write, which is [`crate::hifz`]'s mechanism: this module
//! opens no file for writing, and [`HawiyatAsar::uktub`] takes the guard it
//! writes through as an argument rather than constructing one.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use taarib_muharrik::dalail::nusus::HadafNusus;

use crate::hifz::Hafiz;
use crate::khata::{KhataNusus, hajm_usize, tul_u64};
use crate::tabaqa::{Dalil, Mifhas, Rutba, SiyaqTabaqa};
use crate::tarkeeb::masar_bila_hala;

/// The format's name, as it appears in every refusal this module raises.
pub const SIGHA: &str = "asar";

/// Bytes of fixed framing before the directory JSON begins.
pub const HAJM_ITAR: usize = 16;

/// The alignment the header pickle's string field is padded to.
///
/// Four, from Chromium's `Pickle::WriteString`. It applies to the header and to
/// nothing else — see this module's header.
pub const MUHADHAT_TARWISA: u64 = 4;

/// The largest directory header this module will read.
///
/// Eight mebibytes, the same ceiling Phase 5's probe uses, so that a game
/// refused there is refused here for the same stated reason rather than for a
/// different one. A fifty-thousand-file application has a header of about two.
/// The length is read out of the file being patched, which makes it untrusted
/// input in the ordinary sense: a game directory is not a trusted source.
pub const AQSA_TARWISA: u64 = 8 * 1024 * 1024;

/// The largest archive this module will load into memory.
///
/// Five hundred and twelve mebibytes. Repacking needs the body region resident,
/// and the ceiling is checked against the file's real length before a byte is
/// reserved. Real game archives are tens of megabytes; an `app.asar` past this
/// is one whose assets should have been unpacked, and refusing is better than
/// exhausting the address space of a 32-bit build.
pub const AQSA_HAWIYA: u64 = 512 * 1024 * 1024;

/// The most directory entries this module will walk.
pub const AQSA_MADAKHIL: usize = 200_000;

/// The deepest directory nesting this module will follow.
///
/// A tree is data from an untrusted file, and a recursive walk over one needs a
/// bound that is not the process stack.
pub const AQSA_UMQ: usize = 64;

/// The most bytes read from any one script while gathering evidence.
pub const AQSA_NASS_BARMAJI: usize = 4 * 1024 * 1024;

/// The most bytes of script scanned in total, across all files, per probe.
pub const AQSA_MASH_KULLI: usize = 24 * 1024 * 1024;

/// The longest extractable string, in characters.
///
/// Past this it is a licence text, a base64 blob or a minified bundle in a
/// string literal, and none of those is dialogue.
pub const AQSA_NASS: usize = 4096;

/// The shortest extractable string, in characters.
///
/// Two, so that "OK" and "No" survive. One-character strings are punctuation,
/// separators and CSS units far more often than they are text.
pub const ADNA_NASS: usize = 2;

/// The directory inside the archive that holds everything Taarib adds.
///
/// One directory, one name, so that an uninstall is a directory removal and a
/// second install cannot leave an orphan from the first.
pub const MUJALLAD_HIML: &str = "taarib";

/// Rounds a length up to the header's four-byte boundary.
///
/// Returns [`None`] on overflow rather than wrapping, because the value being
/// rounded came out of the file and a wrapped length is an offset that lands
/// inside the header it was supposed to skip.
#[must_use]
pub const fn muhadhah(tul: u64) -> Option<u64> {
    match tul.checked_add(MUHADHAT_TARWISA - 1) {
        Some(marfu) => Some(marfu & !(MUHADHAT_TARWISA - 1)),
        None => None,
    }
}

/// Reads four little-endian bytes at an offset, bounds-checked.
fn iqra_u32(bayt: &[u8], izaha: usize) -> Option<u32> {
    let nihaya = izaha.checked_add(4)?;
    bayt.get(izaha..nihaya).and_then(|juz| <[u8; 4]>::try_from(juz).ok()).map(u32::from_le_bytes)
}

/// The sixteen bytes of pickle framing at the start of an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TarwisatAsar {
    /// Bytes in the header pickle: `8 + align4(tul_json)`.
    pub tul_tarwisa: u64,
    /// Bytes of directory JSON, unpadded.
    pub tul_json: u64,
    /// Where the packed bodies begin: `8 + tul_tarwisa`.
    pub bidayat_muhtawa: u64,
}

impl TarwisatAsar {
    /// Reads and validates the framing, and nothing beyond it.
    ///
    /// The checks run in this order because each one makes the next meaningful:
    /// enough bytes to read the framing at all; the constant 4 that identifies
    /// a pickle-framed file; the declared header size against this build's
    /// ceiling *before* anything is allocated for it; and finally the JSON
    /// length against the header size and the header against the real file.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::MalafQaseer`] when the file is shorter than the framing or
    /// than what the framing declares, [`KhataNusus::SihrGhayrMutabaq`] when the
    /// leading constant is not 4, [`KhataNusus::HajmMufrit`] when the header
    /// exceeds [`AQSA_TARWISA`], and [`KhataNusus::HawiyaTalifa`] when the JSON
    /// length does not fit inside the header the file declared.
    pub fn min_bayt(masar: &Path, bayt: &[u8]) -> Result<Self, KhataNusus> {
        let tul_malaf = tul_u64(bayt.len());
        if bayt.len() < HAJM_ITAR {
            return Err(KhataNusus::MalafQaseer {
                haql: "the asar pickle framing",
                tul: tul_malaf,
                matlub: tul_u64(HAJM_ITAR),
            });
        }
        let qaseer = |haql: &'static str| KhataNusus::MalafQaseer {
            haql,
            tul: tul_malaf,
            matlub: tul_u64(HAJM_ITAR),
        };

        if iqra_u32(bayt, 0).ok_or_else(|| qaseer("the size pickle"))? != 4 {
            return Err(KhataNusus::SihrGhayrMutabaq { masar: masar.to_path_buf(), sigha: SIGHA });
        }
        let tul_tarwisa = u64::from(iqra_u32(bayt, 4).ok_or_else(|| qaseer("the header size"))?);
        if tul_tarwisa > AQSA_TARWISA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the asar directory header",
                qeema: tul_tarwisa,
                saqf: AQSA_TARWISA,
            });
        }
        let tul_json = u64::from(iqra_u32(bayt, 12).ok_or_else(|| qaseer("the JSON length"))?);
        if tul_json == 0 {
            return Err(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "the declared JSON length",
                qeema: 0,
                hadd: tul_tarwisa,
            });
        }
        // The JSON plus its own four-byte length field must fit inside the
        // header pickle. Without this, a header claiming a short pickle and a
        // long string would have the reader walk out of the framing and read
        // packed file bytes as directory JSON.
        let matlub = tul_json.saturating_add(4);
        if matlub > tul_tarwisa {
            return Err(KhataNusus::HawiyaTalifa {
                sigha: SIGHA,
                haql: "the declared JSON length",
                qeema: tul_json,
                hadd: tul_tarwisa.saturating_sub(4),
            });
        }
        let bidayat_muhtawa = tul_tarwisa.checked_add(8).ok_or(KhataNusus::HawiyaTalifa {
            sigha: SIGHA,
            haql: "the header size (adding the framing overflows)",
            qeema: tul_tarwisa,
            hadd: AQSA_TARWISA,
        })?;
        if bidayat_muhtawa > tul_malaf {
            return Err(KhataNusus::MalafQaseer {
                haql: "the asar directory header",
                tul: tul_malaf,
                matlub: bidayat_muhtawa,
            });
        }
        Ok(Self { tul_tarwisa, tul_json, bidayat_muhtawa })
    }

    /// Builds the framing for a directory JSON of a given length.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the header this would produce is past
    /// [`AQSA_TARWISA`], which is also the point at which the u32 fields in the
    /// framing would stop being able to express it.
    pub fn li_tul(tul_json: u64) -> Result<Self, KhataNusus> {
        let mahshu = muhadhah(tul_json).ok_or(KhataNusus::HajmMufrit {
            haql: "the rebuilt asar directory header",
            qeema: tul_json,
            saqf: AQSA_TARWISA,
        })?;
        let tul_tarwisa = mahshu.checked_add(8).ok_or(KhataNusus::HajmMufrit {
            haql: "the rebuilt asar directory header",
            qeema: mahshu,
            saqf: AQSA_TARWISA,
        })?;
        if tul_tarwisa > AQSA_TARWISA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the rebuilt asar directory header",
                qeema: tul_tarwisa,
                saqf: AQSA_TARWISA,
            });
        }
        Ok(Self {
            tul_tarwisa,
            tul_json,
            bidayat_muhtawa: tul_tarwisa.saturating_add(8),
        })
    }

    /// The sixteen framing bytes, ready to be written before the JSON.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when a field does not fit the u32 the format
    /// gives it, which [`TarwisatAsar::li_tul`] has already ruled out and which
    /// is re-checked here rather than cast.
    pub fn ila_bayt(&self) -> Result<[u8; HAJM_ITAR], KhataNusus> {
        let mufrit = |qeema: u64| KhataNusus::HajmMufrit {
            haql: "an asar framing field",
            qeema,
            saqf: u64::from(u32::MAX),
        };
        let tul_tarwisa = u32::try_from(self.tul_tarwisa).map_err(|_| mufrit(self.tul_tarwisa))?;
        let tul_json = u32::try_from(self.tul_json).map_err(|_| mufrit(self.tul_json))?;
        let hajm_hamula = tul_tarwisa.checked_sub(4).ok_or_else(|| mufrit(self.tul_tarwisa))?;
        let mut itar = [0_u8; HAJM_ITAR];
        let mut daa = |izaha: usize, qeema: u32| {
            if let Some(khana) = itar.get_mut(izaha..izaha.saturating_add(4)) {
                khana.copy_from_slice(&qeema.to_le_bytes());
            }
        };
        daa(0, 4);
        daa(4, tul_tarwisa);
        daa(8, hajm_hamula);
        daa(12, tul_json);
        Ok(itar)
    }
}

/// One entry from the directory tree, flattened out of the nesting.
///
/// The tree is nested and every consumer of it wants a path, so the walk is done
/// once, here, and the nesting is kept only in [`HawiyatAsar::shajara`] where the
/// rewriter needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalAsar {
    /// The path inside the archive, forward slashes, no leading slash.
    pub masar: String,
    /// The file's length in bytes, as the directory declares it.
    pub hajm: u64,
    /// Where the bytes start, relative to [`TarwisatAsar::bidayat_muhtawa`].
    ///
    /// [`None`] for an entry that has no packed bytes: an unpacked file or a
    /// link. Parsed from the JSON *string* the format stores, never from a
    /// number — see this module's header.
    pub izaha: Option<u64>,
    /// The `executable` flag, preserved as found.
    pub tanfidhi: bool,
    /// The `unpacked` flag: the bytes live in `app.asar.unpacked/` beside the
    /// archive and this module never touches them.
    pub ghayr_mahzum: bool,
    /// The `link` target, when the entry is a symbolic link inside the archive.
    pub wasla: Option<String>,
    /// Whether the entry carries an `integrity` block.
    pub salama: bool,
}

impl MadkhalAsar {
    /// Whether this entry's bytes are inside the archive.
    ///
    /// False for unpacked files and links, which is the question every caller
    /// that is about to read bytes actually wants answered.
    #[must_use]
    pub const fn mahzum(&self) -> bool {
        self.izaha.is_some() && !self.ghayr_mahzum && self.wasla.is_none()
    }

    /// The lowercase extension, or an empty string when there is none.
    #[must_use]
    pub fn imtidad(&self) -> String {
        match self.masar.rsplit_once('.') {
            Some((_, lahiqa)) if !lahiqa.contains('/') => lahiqa.to_ascii_lowercase(),
            _ => String::new(),
        }
    }
}

/// Reads the `offset` field, which the format stores as decimal text.
///
/// Refuses anything that is not entirely ASCII digits rather than taking a
/// prefix. `"12 "` and `"0x1F"` are corruption or another tool's output, and a
/// lenient parse of either produces an offset that points at the wrong file's
/// bytes — which is a patch that silently swaps two files' contents.
fn izahat_madkhal(qeema: &Value) -> Option<u64> {
    let nass = qeema.as_str()?;
    if nass.is_empty() || !nass.bytes().all(|bayt| bayt.is_ascii_digit()) {
        return None;
    }
    nass.parse::<u64>().ok()
}

/// Joins a directory path onto a child name.
fn taht(asas: &str, ism: &str) -> String {
    if asas.is_empty() { ism.to_owned() } else { format!("{asas}/{ism}") }
}

/// The whole application archive, resident and ready to be rewritten.
///
/// Holds the original bytes rather than a file handle, because a repack reads
/// every packed body it preserves and a handle would mean thousands of seeks
/// over a file that is already smaller than a single texture page.
#[derive(Debug)]
pub struct HawiyatAsar {
    /// Where the archive was read from.
    masar: PathBuf,
    /// The framing as read.
    tarwisa: TarwisatAsar,
    /// The directory tree, parsed, with unknown members preserved.
    shajara: Value,
    /// Every entry, flattened, in the tree's own order.
    madakhil: Vec<MadkhalAsar>,
    /// The original file, whole.
    bayt: Vec<u8>,
    /// Files added or replaced by this session, in the order they were added.
    idafat: Vec<(String, Vec<u8>)>,
    /// Whether any entry declared an `integrity` block.
    salama: bool,
}

impl HawiyatAsar {
    /// Reads an archive from disk.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when the file cannot be read,
    /// [`KhataNusus::HajmMufrit`] when it is larger than [`AQSA_HAWIYA`], and
    /// whatever [`HawiyatAsar::min_bayt`] refuses.
    pub fn min_masar(masar: &Path) -> Result<Self, KhataNusus> {
        let bayanat = fs::metadata(masar)
            .map_err(|sabab| KhataNusus::KhataMalaf { masar: masar.to_path_buf(), sabab })?;
        if bayanat.len() > AQSA_HAWIYA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the asar archive",
                qeema: bayanat.len(),
                saqf: AQSA_HAWIYA,
            });
        }
        let bayt = fs::read(masar)
            .map_err(|sabab| KhataNusus::KhataMalaf { masar: masar.to_path_buf(), sabab })?;
        Self::min_bayt(masar.to_path_buf(), bayt)
    }

    /// Parses an archive already in memory.
    ///
    /// # Errors
    ///
    /// Whatever [`TarwisatAsar::min_bayt`] refuses;
    /// [`KhataNusus::NassGhayrSalih`] when the directory is not valid UTF-8;
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when it is not JSON or not the shape
    /// the format defines; [`KhataNusus::HajmMufrit`] when it declares more than
    /// [`AQSA_MADAKHIL`] entries or nests past [`AQSA_UMQ`]; and
    /// [`KhataNusus::HawiyaTalifa`] when an entry's bytes are not inside the
    /// file.
    pub fn min_bayt(masar: PathBuf, bayt: Vec<u8>) -> Result<Self, KhataNusus> {
        let tarwisa = TarwisatAsar::min_bayt(&masar, &bayt)?;
        let tul_json = hajm_usize(tarwisa.tul_json).ok_or(KhataNusus::HajmMufrit {
            haql: "the asar directory header",
            qeema: tarwisa.tul_json,
            saqf: AQSA_TARWISA,
        })?;
        let nihaya = HAJM_ITAR.checked_add(tul_json).ok_or(KhataNusus::HajmMufrit {
            haql: "the asar directory header",
            qeema: tarwisa.tul_json,
            saqf: AQSA_TARWISA,
        })?;
        let khaam = bayt.get(HAJM_ITAR..nihaya).ok_or_else(|| KhataNusus::MalafQaseer {
            haql: "the asar directory JSON",
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(nihaya),
        })?;
        let ism = masar.display().to_string();
        let nass = std::str::from_utf8(khaam).map_err(|khata| KhataNusus::NassGhayrSalih {
            malaf: ism.clone(),
            tarmiz: "UTF-8",
            mawqi: tul_u64(khata.valid_up_to()),
        })?;
        let shajara: Value = serde_json::from_str(nass).map_err(|khata| {
            KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism.clone(),
                haql: format!("the directory is not valid JSON: {khata}"),
            }
        })?;

        let mut madakhil = Vec::new();
        let mut salama = false;
        let judhur = shajara.get("files").and_then(Value::as_object).ok_or_else(|| {
            KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism.clone(),
                haql: "files (the directory has no root file map)".to_owned(),
            }
        })?;
        imshi(judhur, "", 0, &ism, &mut madakhil, &mut salama)?;

        let tul_malaf = tul_u64(bayt.len());
        let mutah = tul_malaf.saturating_sub(tarwisa.bidayat_muhtawa);
        for madkhal in &madakhil {
            let Some(izaha) = madkhal.izaha else { continue };
            if madkhal.ghayr_mahzum {
                continue;
            }
            let nihayat_madkhal =
                izaha.checked_add(madkhal.hajm).ok_or(KhataNusus::HawiyaTalifa {
                    sigha: SIGHA,
                    haql: "an entry's offset plus its size overflows",
                    qeema: izaha,
                    hadd: mutah,
                })?;
            if nihayat_madkhal > mutah {
                return Err(KhataNusus::HawiyaTalifa {
                    sigha: SIGHA,
                    haql: "an entry ends past the last packed byte",
                    qeema: nihayat_madkhal,
                    hadd: mutah,
                });
            }
        }

        Ok(Self { masar, tarwisa, shajara, madakhil, bayt, idafat: Vec::new(), salama })
    }

    /// Where this archive was read from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// The framing as read.
    #[must_use]
    pub const fn tarwisa(&self) -> TarwisatAsar {
        self.tarwisa
    }

    /// Every entry, flattened, in the directory's own order.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalAsar] {
        &self.madakhil
    }

    /// Whether any entry declares an `integrity` block.
    ///
    /// True means the application was built with Electron's embedded asar
    /// integrity checking, whose expected hash is fused into the executable and
    /// is not reachable from here. [`HawiyatAsar::uktub`] refuses in that case.
    #[must_use]
    pub const fn yatahaqqaq(&self) -> bool {
        self.salama
    }

    /// One entry by path.
    #[must_use]
    pub fn madkhal(&self, masar: &str) -> Option<&MadkhalAsar> {
        self.madakhil.iter().find(|madkhal| madkhal.masar == masar)
    }

    /// A packed file's bytes.
    ///
    /// [`None`] for a path that is not in the archive, and for one whose bytes
    /// are not in the archive — an unpacked file or a link — which the caller
    /// must distinguish with [`MadkhalAsar::mahzum`] rather than by treating an
    /// absent body as an empty file.
    #[must_use]
    pub fn muhtawa(&self, masar: &str) -> Option<&[u8]> {
        if let Some((_, jadeed)) =
            self.idafat.iter().rev().find(|(ism, _)| ism.as_str() == masar)
        {
            return Some(jadeed);
        }
        let madkhal = self.madkhal(masar)?;
        if !madkhal.mahzum() {
            return None;
        }
        self.qita(madkhal.izaha?, madkhal.hajm)
    }

    /// The bytes of a range inside the original body region.
    fn qita(&self, izaha: u64, hajm: u64) -> Option<&[u8]> {
        let bidaya = hajm_usize(self.tarwisa.bidayat_muhtawa.checked_add(izaha)?)?;
        let nihaya = bidaya.checked_add(hajm_usize(hajm)?)?;
        self.bayt.get(bidaya..nihaya)
    }

    /// Where an unpacked file actually lives, given the archive's own path.
    ///
    /// `resources/app.asar` plus `js/native.node` is
    /// `resources/app.asar.unpacked/js/native.node`. Returns [`None`] for an
    /// entry that is packed, because asking this about a packed file is a
    /// caller that has confused the two.
    #[must_use]
    pub fn masar_ghayr_mahzum(&self, masar: &str) -> Option<PathBuf> {
        let madkhal = self.madkhal(masar)?;
        if !madkhal.ghayr_mahzum {
            return None;
        }
        let ism = self.masar.file_name()?.to_str()?;
        let mujallad = self.masar.parent()?.join(format!("{ism}.unpacked"));
        Some(madkhal.masar.split('/').fold(mujallad, |masar, juz| masar.join(juz)))
    }
}

/// Walks one directory level of the tree, depth-first, in key order.
///
/// Iterative recursion is avoided by bounding the depth instead: `AQSA_UMQ` is
/// checked before descending, so a directory tree that nests itself cannot walk
/// the process stack into the ground.
fn imshi(
    mustawa: &Map<String, Value>,
    asas: &str,
    umq: usize,
    ism: &str,
    madakhil: &mut Vec<MadkhalAsar>,
    salama: &mut bool,
) -> Result<(), KhataNusus> {
    if umq > AQSA_UMQ {
        return Err(KhataNusus::HajmMufrit {
            haql: "the asar directory nesting depth",
            qeema: tul_u64(umq),
            saqf: tul_u64(AQSA_UMQ),
        });
    }
    for (juz, uqda) in mustawa {
        if madakhil.len() >= AQSA_MADAKHIL {
            return Err(KhataNusus::HajmMufrit {
                haql: "the asar directory entry count",
                qeema: tul_u64(madakhil.len()),
                saqf: tul_u64(AQSA_MADAKHIL),
            });
        }
        // A path component that escapes the archive is refused rather than
        // normalised: an entry named `../../etc` is a packer that was fed a
        // hostile path, and unpacking it would write outside the game.
        if juz.is_empty() || juz == "." || juz == ".." || juz.contains(['/', '\\']) {
            return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism.to_owned(),
                haql: format!("an entry name is not a single path component: {juz}"),
            });
        }
        let masar = taht(asas, juz);
        if let Some(atfal) = uqda.get("files").and_then(Value::as_object) {
            imshi(atfal, &masar, umq.saturating_add(1), ism, madakhil, salama)?;
            continue;
        }
        let ghayr_mahzum = uqda.get("unpacked").and_then(Value::as_bool).unwrap_or(false);
        let wasla = uqda.get("link").and_then(Value::as_str).map(str::to_owned);
        let izaha = uqda.get("offset").and_then(izahat_madkhal);
        let hajm = uqda.get("size").and_then(Value::as_u64).unwrap_or(0);
        if uqda.get("size").is_some_and(|qeema| qeema.as_u64().is_none()) {
            return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism.to_owned(),
                haql: format!("{masar}: size is not a non-negative integer"),
            });
        }
        // An entry with neither an offset, nor the unpacked flag, nor a link
        // target is a file whose bytes are nowhere. That is corruption, and the
        // alternative to refusing is a repack that writes an empty file over
        // something the game needs.
        if izaha.is_none() && !ghayr_mahzum && wasla.is_none() {
            let sabab = uqda.get("offset").map_or_else(
                || format!("{masar}: no offset, no unpacked flag and no link target"),
                |_| format!("{masar}: offset is not a decimal string, which the format requires"),
            );
            return Err(KhataNusus::BunyaGhayrMutawaqqaa { malaf: ism.to_owned(), haql: sabab });
        }
        if uqda.get("integrity").is_some() {
            *salama = true;
        }
        madakhil.push(MadkhalAsar {
            masar,
            hajm,
            izaha,
            tanfidhi: uqda.get("executable").and_then(Value::as_bool).unwrap_or(false),
            ghayr_mahzum,
            wasla,
            salama: uqda.get("integrity").is_some(),
        });
    }
    Ok(())
}

impl HawiyatAsar {
    /// Adds a file to the archive, or replaces one that is already there.
    ///
    /// The bytes are queued rather than written: nothing in the original body
    /// region moves, and the new content is appended past its end when
    /// [`HawiyatAsar::ila_bayt`] runs. That is what makes every untouched entry
    /// byte-identical after a repack, and it is why replacing a file leaves its
    /// old bytes in place as dead space — a few kilobytes of waste bought in
    /// exchange for a round-trip that can be proved rather than hoped for.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the path is not a usable
    /// archive path or when it names something the directory already holds as a
    /// directory, and [`KhataNusus::HajmMufrit`] when the archive would grow
    /// past [`AQSA_HAWIYA`].
    pub fn daa(&mut self, masar: &str, muhtawa: Vec<u8>) -> Result<(), KhataNusus> {
        let ism = self.masar.display().to_string();
        let ajzaa: Vec<&str> = masar.split('/').collect();
        if masar.is_empty()
            || ajzaa.iter().any(|juz| juz.is_empty() || *juz == "." || *juz == ".."
                || juz.contains('\\'))
        {
            return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism,
                haql: format!("{masar} is not a usable path inside an asar"),
            });
        }
        if ajzaa.len() > AQSA_UMQ {
            return Err(KhataNusus::HajmMufrit {
                haql: "the payload path's nesting depth",
                qeema: tul_u64(ajzaa.len()),
                saqf: tul_u64(AQSA_UMQ),
            });
        }
        let bidaya = format!("{masar}/");
        if self.madakhil.iter().any(|madkhal| madkhal.masar.starts_with(&bidaya)) {
            return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism,
                haql: format!("{masar} is a directory in this archive and cannot become a file"),
            });
        }
        if let Some(madkhal) = self.madkhal(masar)
            && (madkhal.ghayr_mahzum || madkhal.wasla.is_some())
        {
            // An unpacked file lives outside the archive and a link points at
            // something else. Turning either into packed bytes would move a
            // native module the loader has to `dlopen` from disk, or silently
            // break a link the application resolves at runtime.
            return Err(KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: ism,
                haql: format!("{masar} is unpacked or a link and is not rewritten from here"),
            });
        }
        let namaa = self
            .idafat
            .iter()
            .try_fold(tul_u64(self.bayt.len()), |majmu, (_, bayt)| {
                majmu.checked_add(tul_u64(bayt.len()))
            })
            .and_then(|majmu| majmu.checked_add(tul_u64(muhtawa.len())))
            .ok_or(KhataNusus::HajmMufrit {
                haql: "the repacked asar archive",
                qeema: u64::MAX,
                saqf: AQSA_HAWIYA,
            })?;
        if namaa > AQSA_HAWIYA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the repacked asar archive",
                qeema: namaa,
                saqf: AQSA_HAWIYA,
            });
        }
        self.idafat.retain(|(sabiq, _)| sabiq.as_str() != masar);
        self.idafat.push((masar.to_owned(), muhtawa));
        Ok(())
    }

    /// The whole rebuilt archive as bytes.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the rebuilt directory header is past
    /// [`AQSA_TARWISA`] or an offset stops fitting the format's fields, and
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the tree cannot be rewritten
    /// because a path component collides with a file.
    pub fn ila_bayt(&self) -> Result<Vec<u8>, KhataNusus> {
        let bidaya = hajm_usize(self.tarwisa.bidayat_muhtawa).ok_or(KhataNusus::HajmMufrit {
            haql: "the asar body offset",
            qeema: self.tarwisa.bidayat_muhtawa,
            saqf: AQSA_HAWIYA,
        })?;
        let asli = self.bayt.get(bidaya..).unwrap_or_default();
        let mut shajara = self.shajara.clone();
        let mut muhtawa = Vec::with_capacity(asli.len().saturating_add(64 * 1024));
        muhtawa.extend_from_slice(asli);

        for (masar, bayt) in &self.idafat {
            let izaha = tul_u64(muhtawa.len());
            let mut uqda = Map::new();
            let _ = uqda.insert("size".to_owned(), Value::from(tul_u64(bayt.len())));
            let _ = uqda.insert("offset".to_owned(), Value::String(izaha.to_string()));
            let ajzaa: Vec<&str> = masar.split('/').collect();
            thabbit(&mut shajara, &ajzaa, Value::Object(uqda), &self.masar)?;
            muhtawa.extend_from_slice(bayt);
        }

        let json = serde_json::to_vec(&shajara).map_err(|khata| {
            KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: self.masar.display().to_string(),
                haql: format!("the rebuilt directory could not be serialized: {khata}"),
            }
        })?;
        let tarwisa = TarwisatAsar::li_tul(tul_u64(json.len()))?;
        let itar = tarwisa.ila_bayt()?;
        // `tul_tarwisa` is `8 + align4(tul_json)`, so what the pickle pads with
        // is exactly `align4(tul_json) - tul_json` zero bytes.
        let hashw = hajm_usize(
            tarwisa.tul_tarwisa.saturating_sub(8).saturating_sub(tul_u64(json.len())),
        )
        .unwrap_or(0);

        let mut khaam = Vec::with_capacity(
            HAJM_ITAR
                .saturating_add(json.len())
                .saturating_add(hashw)
                .saturating_add(muhtawa.len()),
        );
        khaam.extend_from_slice(&itar);
        khaam.extend_from_slice(&json);
        // The pickle pads its string field with zeros, not with whatever was in
        // the buffer. A packer that leaves uninitialized bytes here produces an
        // archive that hashes differently on every build.
        khaam.resize(khaam.len().saturating_add(hashw), 0);
        khaam.extend_from_slice(&muhtawa);
        Ok(khaam)
    }

    /// Writes the rebuilt archive through the guard, after proving the round
    /// trip.
    ///
    /// The order is deliberate and is the whole safety argument: the archive is
    /// serialized into memory, *re-parsed from those very bytes*, and every
    /// entry this session did not touch is compared against the original byte
    /// for byte — all before anything reaches the disk. Only then are the bytes
    /// handed to [`Hafiz::iktub`], which preserves the game's own `app.asar`
    /// and its fingerprint and writes atomically. A mismatch anywhere means
    /// nothing is written at all, and an interruption during the write leaves
    /// either the whole old archive or the whole new one.
    ///
    /// The earlier shape proved the round trip by writing a temporary file and
    /// reading it back. Doing it in memory is strictly stronger — the check now
    /// happens before the filesystem is involved — and it removes a second
    /// atomic-write implementation from a crate that should have exactly one.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HimlMarfud`] when the archive carries Electron's embedded
    /// integrity blocks, whose expected hash is fused into the executable and
    /// cannot be rewritten from here; [`KhataNusus::NuskhaMafquda`] when the
    /// original cannot be preserved; [`KhataNusus::KhataMalaf`] when the write
    /// fails; [`KhataNusus::DawraGhayrMutabaqa`] when the re-parsed archive
    /// differs from the original in a region nothing was supposed to change;
    /// and whatever [`HawiyatAsar::ila_bayt`] refuses.
    pub fn uktub(&self, hafiz: &mut dyn Hafiz, hadaf: &Path) -> Result<(), KhataNusus> {
        if self.salama {
            return Err(KhataNusus::HimlMarfud {
                alia: "asar repack",
                sabab: format!(
                    "{} carries per-file integrity blocks, so this application was built \
                     with Electron's embedded asar validation and the expected header hash \
                     is fused into the executable rather than stored in the archive. \
                     Repacking it would produce a game that refuses to launch. The route \
                     that remains is an unpacked resources/app/ directory.",
                    self.masar.display()
                ),
            });
        }
        let khaam = self.ila_bayt()?;

        // Parsed back out of the buffer that is about to be written, not out of
        // a file. The two are the same bytes, and doing it here means a rebuild
        // this module cannot re-read is a refusal that touched nothing.
        let jadeed = Self::min_bayt(hadaf.to_path_buf(), khaam.clone())?;
        self.dawra(&jadeed)?;

        hafiz.iktub(hadaf, &khaam)
    }

    /// Compares a rebuilt archive against this one, entry by entry.
    ///
    /// The only honest check available for a container this module reconstructs
    /// rather than edits in place. It counts differing bytes instead of stopping
    /// at the first, because "one byte differs" and "four megabytes differ" are
    /// two very different bugs and the refusal should say which one happened.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::DawraGhayrMutabaqa`] when an untouched entry's bytes
    /// changed, when an entry disappeared, or when a file this session added did
    /// not survive the write.
    pub fn dawra(&self, jadeed: &Self) -> Result<(), KhataNusus> {
        let mudafa: BTreeSet<&str> =
            self.idafat.iter().map(|(masar, _)| masar.as_str()).collect();
        let mut farq: u64 = 0;

        for madkhal in &self.madakhil {
            if mudafa.contains(madkhal.masar.as_str()) {
                continue;
            }
            let Some(qadeem) = self.muhtawa(&madkhal.masar) else {
                // Not packed: an unpacked file or a link. Its bytes were never
                // in the archive and must still not be, which is checked by
                // the entry surviving rather than by comparing bodies.
                if jadeed.madkhal(&madkhal.masar).is_none() {
                    farq = farq.saturating_add(1);
                }
                continue;
            };
            match jadeed.muhtawa(&madkhal.masar) {
                Some(hadeeth) if hadeeth.len() == qadeem.len() => {
                    let ikhtilaf = qadeem
                        .iter()
                        .zip(hadeeth.iter())
                        .filter(|(awwal, thani)| awwal != thani)
                        .count();
                    farq = farq.saturating_add(tul_u64(ikhtilaf));
                }
                Some(hadeeth) => {
                    farq = farq
                        .saturating_add(tul_u64(hadeeth.len().abs_diff(qadeem.len())))
                        .saturating_add(tul_u64(qadeem.len().min(hadeeth.len())));
                }
                None => farq = farq.saturating_add(tul_u64(qadeem.len())),
            }
        }

        for (masar, bayt) in &self.idafat {
            match jadeed.muhtawa(masar) {
                Some(maktub) if maktub == bayt.as_slice() => {}
                Some(maktub) => {
                    farq = farq.saturating_add(tul_u64(maktub.len().abs_diff(bayt.len()).max(1)));
                }
                None => farq = farq.saturating_add(tul_u64(bayt.len())),
            }
        }

        if farq == 0 {
            Ok(())
        } else {
            Err(KhataNusus::DawraGhayrMutabaqa { sigha: SIGHA, adad: farq })
        }
    }
}

/// Sets a node in the directory tree, creating the directories above it.
///
/// Works on the nested `Value` rather than on the flattened list because the
/// nesting is what gets serialized, and because every member this module does
/// not understand — `integrity`, anything a future Electron adds — survives
/// untouched by being left where it was found.
/// Recursive rather than iterative, and deliberately: descending a `&mut Value`
/// with a loop means reassigning a reference to something reborrowed from
/// itself, which is the one shape of tree walk the borrow checker is entitled to
/// reject. Recursion says the same thing in a way that is obviously sound, and
/// the depth is the path's component count — bounded by [`AQSA_UMQ`] at the
/// caller — rather than by anything in the archive.
fn thabbit(
    shajara: &mut Value,
    ajzaa: &[&str],
    uqda: Value,
    ism: &Path,
) -> Result<(), KhataNusus> {
    let talif = |haql: String| KhataNusus::BunyaGhayrMutawaqqaa {
        malaf: ism.display().to_string(),
        haql,
    };
    let Some((awwal, baqi)) = ajzaa.split_first() else {
        return Err(talif("a payload path has no components".to_owned()));
    };
    let mustawa = shajara
        .get_mut("files")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| talif(format!("{awwal}: its parent is not a directory")))?;
    if baqi.is_empty() {
        let _ = mustawa.insert((*awwal).to_owned(), uqda);
        return Ok(());
    }
    let farah = mustawa.entry((*awwal).to_owned()).or_insert_with(|| {
        let mut khali = Map::new();
        let _ = khali.insert("files".to_owned(), Value::Object(Map::new()));
        Value::Object(khali)
    });
    if farah.get("files").is_none() {
        return Err(talif(format!("{awwal} is a file, not a directory")));
    }
    thabbit(farah, baqi, uqda, ism)
}

// ---------------------------------------------------------------------------
// String extraction
//
// Two sources, and they are not equally trustworthy, which is the honest thing
// to say about this whole section.
//
// A **resource file** — `locales/en.json`, `i18n/en-US.json`, `lang/*.json` —
// is a map the application itself looks strings up in. Every string value in it
// is text meant for a person, its key is a stable identity the application
// already relies on, and extraction from it is exact. Nothing is guessed.
//
// A **script** is not that. A game that writes its dialogue inline in
// JavaScript is a game whose text is mixed in with selectors, class names,
// event names, URLs and format specifiers, and telling those apart needs to
// know what the surrounding code does with each literal. A JavaScript parser is
// out of scope for this crate and a type-aware one is out of scope for any
// static tool, so what runs here is a **lexer plus a filter**: find every
// string and template literal the language defines, then drop the ones that
// look like machinery.
//
// What that will get wrong is written out in `nusus_min_js`, in specific terms,
// because a heuristic whose failures are not documented is a heuristic that
// gets trusted as if it were a parser.
// ---------------------------------------------------------------------------

/// Where an extracted string came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NawNass {
    /// A value in a resource file the application looks strings up in.
    ///
    /// Exact: the key is the application's own, and every string value in such
    /// a file is text by construction.
    Mawrid,
    /// A literal found inside a script.
    ///
    /// Heuristic. See [`nusus_min_js`] for what that means and what it misses.
    Barmaji,
}

impl NawNass {
    /// The name this source carries in a report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mawrid => "resource",
            Self::Barmaji => "script",
        }
    }

    /// Whether strings from this source are exact rather than inferred.
    #[must_use]
    pub const fn daqiq(self) -> bool {
        matches!(self, Self::Mawrid)
    }
}

/// One string found in the application, with a stable identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NassMustakhraj {
    /// A stable identity: BLAKE3 over source, key and text, first sixteen bytes
    /// as hexadecimal.
    ///
    /// Stable across rebuilds of the same game because it depends on nothing
    /// about the order files were walked in, and it *changes* when the source
    /// text changes — which is correct, because a changed source string is a
    /// string that needs looking at again rather than one whose old translation
    /// should be re-applied silently.
    pub huwiya: String,
    /// The archive path the string was found in.
    pub masdar: String,
    /// Where inside that file: a JSON pointer for a resource, `js:<n>` for the
    /// n-th literal in a script.
    pub miftah: String,
    /// The string itself.
    pub nass: String,
    /// Which of the two sources this came from.
    pub naw: NawNass,
}

/// The stable identity of one extracted string.
///
/// Sixteen bytes of BLAKE3 as hexadecimal: short enough to read in a diff, and
/// far past any collision a game's string count can reach.
#[must_use]
pub fn huwiyat_nass(masdar: &str, miftah: &str, nass: &str) -> String {
    let mut hashib = blake3::Hasher::new();
    hashib.update(masdar.as_bytes());
    hashib.update(&[0]);
    hashib.update(miftah.as_bytes());
    hashib.update(&[0]);
    hashib.update(nass.as_bytes());
    let basma = hashib.finalize();
    basma.to_hex().get(..32).unwrap_or_default().to_owned()
}

/// Whether a string is plausibly text a player reads.
///
/// The rejections are listed here rather than scattered through the scanner so
/// that the answer to "why was my dialogue line dropped" is one function long:
///
/// * shorter than [`ADNA_NASS`] or longer than [`AQSA_NASS`] characters;
/// * no alphabetic character anywhere — punctuation, numbers, separators;
/// * a path or a URL: a leading `/` or `./`, a backslash, or `://`;
/// * a bare filename with an asset extension;
/// * `SCREAMING_SNAKE_CASE`, which is a constant name and not a sentence;
/// * a lone CSS declaration or selector: a `{`/`}` pair, a trailing `;` with a
///   `:` before it, or a leading `#`/`.` followed by an identifier;
/// * a colour, a number with a unit, or a MIME type;
/// * a long unbroken run with no spaces and mixed case and digits, which is a
///   hash, a base64 blob or a minified identifier.
#[must_use]
pub fn yasluh_lil_tarjama(nass: &str) -> bool {
    let munaqqa = nass.trim();
    let adad = munaqqa.chars().count();
    if !(ADNA_NASS..=AQSA_NASS).contains(&adad) {
        return false;
    }
    if !munaqqa.chars().any(char::is_alphabetic) {
        return false;
    }
    if munaqqa.contains("://") || munaqqa.contains('\\') || munaqqa.starts_with('/')
        || munaqqa.starts_with("./") || munaqqa.starts_with("../")
    {
        return false;
    }
    if munaqqa.contains('{') && munaqqa.contains('}') && munaqqa.contains(':') {
        return false;
    }
    if munaqqa.ends_with(';') && munaqqa.contains(':') && !munaqqa.contains(' ') {
        return false;
    }
    if munaqqa.starts_with('#') || munaqqa.starts_with('.') {
        let baqi = munaqqa.get(1..).unwrap_or_default();
        if !baqi.is_empty()
            && baqi.chars().all(|harf| harf.is_ascii_alphanumeric() || harf == '-' || harf == '_')
        {
            return false;
        }
    }
    let munkhafid = munaqqa.to_ascii_lowercase();
    if !munaqqa.contains(' ')
        && IMTIDADAT_ASUL.iter().any(|lahiqa| munkhafid.ends_with(lahiqa))
    {
        return false;
    }
    if munaqqa.contains('/') && !munaqqa.contains(' ') {
        return false;
    }
    // A constant name: capitals, digits and underscores only, with at least one
    // underscore so that "OK" and "HP" survive.
    if munaqqa.contains('_')
        && munaqqa
            .chars()
            .all(|harf| harf.is_ascii_uppercase() || harf.is_ascii_digit() || harf == '_')
    {
        return false;
    }
    // An unbroken run of mixed case and digits is a hash or a minified name.
    if !munaqqa.contains(' ')
        && adad > 16
        && munaqqa.chars().any(|harf| harf.is_ascii_digit())
        && munaqqa.chars().any(char::is_uppercase)
        && munaqqa.chars().any(char::is_lowercase)
        && munaqqa.chars().all(|harf| harf.is_ascii_alphanumeric() || harf == '+' || harf == '=')
    {
        return false;
    }
    true
}

/// Extensions that make a bare token an asset reference rather than a sentence.
const IMTIDADAT_ASUL: [&str; 20] = [
    ".png", ".jpg", ".jpeg", ".webp", ".gif", ".svg", ".bmp", ".ogg", ".mp3", ".wav", ".m4a",
    ".mp4", ".webm", ".json", ".js", ".mjs", ".cjs", ".css", ".html", ".ttf",
];

/// Escapes one JSON Pointer token per RFC 6901.
fn ramz_muashir(juz: &str) -> String {
    juz.replace('~', "~0").replace('/', "~1")
}

/// Every translatable string in a JSON document, keyed by JSON Pointer.
///
/// Exact rather than heuristic in one specific sense: it makes no judgement
/// about *where* in the document a string sits. It walks the whole tree and
/// applies [`yasluh_lil_tarjama`] to every string value, so a locale file's
/// values come through and its keys do not, because keys are not values.
#[must_use]
pub fn nusus_min_json(masdar: &str, qeema: &Value) -> Vec<NassMustakhraj> {
    let mut hasad = Vec::new();
    let mut mukaddas: Vec<(String, &Value)> = vec![(String::new(), qeema)];
    while let Some((muashir, uqda)) = mukaddas.pop() {
        match uqda {
            Value::String(nass) => {
                if yasluh_lil_tarjama(nass) {
                    let miftah = if muashir.is_empty() { "/".to_owned() } else { muashir };
                    hasad.push(NassMustakhraj {
                        huwiya: huwiyat_nass(masdar, &miftah, nass),
                        masdar: masdar.to_owned(),
                        miftah,
                        nass: nass.clone(),
                        naw: NawNass::Mawrid,
                    });
                }
            }
            Value::Array(qeem) => {
                for (fahras, ibn) in qeem.iter().enumerate().rev() {
                    mukaddas.push((format!("{muashir}/{fahras}"), ibn));
                }
            }
            Value::Object(kain) => {
                for (ism, ibn) in kain.iter().rev() {
                    mukaddas.push((format!("{muashir}/{}", ramz_muashir(ism)), ibn));
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
        if hasad.len() >= AQSA_MADAKHIL {
            break;
        }
    }
    hasad
}

/// Where the scanner is inside a script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HalatMash {
    /// Ordinary code.
    Shifra,
    /// After `//`, until the end of the line.
    TaliqSatr,
    /// After `/*`, until `*/`.
    TaliqKutla,
    /// Inside `'...'`.
    NassMufrad,
    /// Inside `"..."`.
    NassMuzdawaj,
    /// Inside a template literal.
    Qalab,
    /// Inside a regular expression literal.
    TaabirNamati,
}

/// Decodes one escape sequence, returning what it produced and how many source
/// characters it consumed after the backslash.
fn fukk_hurub(huruf: &[char], badi: usize) -> (Option<char>, usize) {
    let Some(&harf) = huruf.get(badi) else { return (None, 0) };
    match harf {
        'n' => (Some('\n'), 1),
        't' => (Some('\t'), 1),
        'r' => (Some('\r'), 1),
        'b' => (Some('\u{8}'), 1),
        'f' => (Some('\u{c}'), 1),
        'v' => (Some('\u{b}'), 1),
        '0' if !huruf.get(badi.saturating_add(1)).is_some_and(char::is_ascii_digit) => {
            (Some('\0'), 1)
        }
        // A backslash before a newline is a line continuation: it produces
        // nothing at all, which is different from producing a newline.
        '\n' => (None, 1),
        '\r' => (None, if huruf.get(badi.saturating_add(1)) == Some(&'\n') { 2 } else { 1 }),
        'x' => {
            let raqmi: String = huruf.iter().skip(badi.saturating_add(1)).take(2).collect();
            match u32::from_str_radix(&raqmi, 16).ok().and_then(char::from_u32) {
                Some(qeema) if raqmi.len() == 2 => (Some(qeema), 3),
                _ => (Some('x'), 1),
            }
        }
        'u' => {
            if huruf.get(badi.saturating_add(1)) == Some(&'{') {
                let raqmi: String = huruf
                    .iter()
                    .skip(badi.saturating_add(2))
                    .take_while(|harf| **harf != '}')
                    .collect();
                let tul = raqmi.chars().count().saturating_add(3);
                match u32::from_str_radix(&raqmi, 16).ok().and_then(char::from_u32) {
                    Some(qeema) => (Some(qeema), tul),
                    None => (Some('u'), 1),
                }
            } else {
                let raqmi: String = huruf.iter().skip(badi.saturating_add(1)).take(4).collect();
                match u32::from_str_radix(&raqmi, 16).ok().and_then(char::from_u32) {
                    Some(qeema) if raqmi.chars().count() == 4 => (Some(qeema), 5),
                    // A lone surrogate half is real in JavaScript source and is
                    // not a `char`. It is dropped rather than replaced, because
                    // a replacement character in a translation source is a
                    // string a translator will faithfully translate as garbage.
                    _ => (None, 5),
                }
            }
        }
        _ => (Some(harf), 1),
    }
}

/// Every string and plain template literal in a script, with the ordinal of
/// each.
///
/// ## What this is
///
/// A JavaScript *lexer*, not a parser. It tracks the five things the grammar
/// needs to find a literal's boundaries — line comments, block comments, the
/// three quote characters, escape sequences, and regular expression literals —
/// and it decodes the escapes so that a `\n` in the source becomes a newline in
/// the extracted string rather than two characters. Every literal it finds is
/// then put through [`yasluh_lil_tarjama`].
///
/// ## What it will miss, specifically
///
/// * **Interpolated templates.** A `` `Hello ${name}` `` is skipped entirely. It
///   is not a translatable unit: the page never contains that text, it contains
///   whatever the interpolation produced, so a translation keyed on it could
///   never match anything at runtime.
/// * **Concatenation.** `"You have " + n + " potions"` yields two fragments,
///   both of which look like text and neither of which is the sentence. They
///   pass the filter and appear as two entries, which a translator has to
///   recognise for what they are.
/// * **Text the filter throws away.** [`yasluh_lil_tarjama`] rejects on shape,
///   and shape is not meaning. A single-word menu label spelled like a constant,
///   a line that is genuinely a path, an item name that ends in `.js` — all
///   dropped.
/// * **Machinery the filter keeps.** The reverse is more common: event names,
///   ARIA labels, class lists, English error messages meant for a developer's
///   console, and log lines all read as text, and all of them come through.
/// * **Regular expressions.** Whether `/` opens a regex or divides is decided
///   from the previous significant character, which is what syntax highlighters
///   do and is not always right. A misjudged `/` can make the scanner read a
///   division as a regex body and miss literals until the next newline.
/// * **Anything not in source form.** A minified bundle still lexes, and its
///   literals are still found; a bundle with strings in a lookup array indexed
///   by number lexes fine and produces a flat list with no clue which string
///   goes where. Strings built at runtime, loaded from a `.pak`, decompressed,
///   or fetched are not here at all.
///
/// None of that is fixable without knowing what the code *does* with each
/// literal, which needs a type-aware analysis of somebody else's minified
/// bundle. The right answer for a game whose text is inline in JavaScript is a
/// translator reading the list and deleting what is not dialogue — so the list
/// is produced with its provenance attached ([`NawNass::Barmaji`]) and is never
/// presented as complete.
#[must_use]
pub fn nusus_min_js(masdar: &str, nass: &str) -> Vec<NassMustakhraj> {
    let huruf: Vec<char> = nass.chars().collect();
    let mut hasad: Vec<NassMustakhraj> = Vec::new();
    let mut halat = HalatMash::Shifra;
    let mut yasmah = true;
    let mut mukaddas: Vec<usize> = Vec::new();
    let mut umq: usize = 0;
    let mut jari = String::new();
    let mut fiha_istibdal = false;
    let mut tarteeb: usize = 0;
    let mut mawdi: usize = 0;

    let mut sajjil = |jari: &str, tarteeb: &mut usize| {
        let miftah = format!("js:{tarteeb}");
        *tarteeb = tarteeb.saturating_add(1);
        if !yasluh_lil_tarjama(jari) || hasad.len() >= AQSA_MADAKHIL {
            return;
        }
        hasad.push(NassMustakhraj {
            huwiya: huwiyat_nass(masdar, &miftah, jari),
            masdar: masdar.to_owned(),
            miftah,
            nass: jari.to_owned(),
            naw: NawNass::Barmaji,
        });
    };

    while let Some(&harf) = huruf.get(mawdi) {
        let baad = huruf.get(mawdi.saturating_add(1)).copied();
        match halat {
            HalatMash::Shifra => match harf {
                '/' if baad == Some('/') => {
                    halat = HalatMash::TaliqSatr;
                    mawdi = mawdi.saturating_add(2);
                    continue;
                }
                '/' if baad == Some('*') => {
                    halat = HalatMash::TaliqKutla;
                    mawdi = mawdi.saturating_add(2);
                    continue;
                }
                '/' if yasmah => halat = HalatMash::TaabirNamati,
                '\'' => {
                    halat = HalatMash::NassMufrad;
                    jari.clear();
                }
                '"' => {
                    halat = HalatMash::NassMuzdawaj;
                    jari.clear();
                }
                '`' => {
                    halat = HalatMash::Qalab;
                    jari.clear();
                    fiha_istibdal = false;
                }
                '{' => {
                    umq = umq.saturating_add(1);
                    yasmah = true;
                }
                '}' => {
                    if mukaddas.last() == Some(&umq) {
                        let _ = mukaddas.pop();
                        halat = HalatMash::Qalab;
                    } else {
                        umq = umq.saturating_sub(1);
                        yasmah = true;
                    }
                }
                _ => {
                    yasmah = !(harf.is_alphanumeric()
                        || harf == '_'
                        || harf == '$'
                        || harf == ')'
                        || harf == ']');
                }
            },
            HalatMash::TaliqSatr => {
                if harf == '\n' {
                    halat = HalatMash::Shifra;
                }
            }
            HalatMash::TaliqKutla => {
                if harf == '*' && baad == Some('/') {
                    halat = HalatMash::Shifra;
                    mawdi = mawdi.saturating_add(2);
                    continue;
                }
            }
            HalatMash::NassMufrad | HalatMash::NassMuzdawaj => {
                let mughliq = if halat == HalatMash::NassMufrad { '\'' } else { '"' };
                if harf == '\\' {
                    let (natij, khutwa) = fukk_hurub(&huruf, mawdi.saturating_add(1));
                    if let Some(qeema) = natij {
                        jari.push(qeema);
                    }
                    mawdi = mawdi.saturating_add(khutwa.saturating_add(1));
                    continue;
                }
                if harf == mughliq {
                    sajjil(&jari, &mut tarteeb);
                    jari.clear();
                    halat = HalatMash::Shifra;
                    yasmah = false;
                } else if harf == '\n' {
                    // An unterminated quote is a mis-lex, not a string. Drop it
                    // and resynchronise on the next line rather than swallowing
                    // the rest of the file.
                    jari.clear();
                    halat = HalatMash::Shifra;
                    yasmah = true;
                } else {
                    jari.push(harf);
                }
            }
            HalatMash::Qalab => {
                if harf == '\\' {
                    let (natij, khutwa) = fukk_hurub(&huruf, mawdi.saturating_add(1));
                    if let Some(qeema) = natij {
                        jari.push(qeema);
                    }
                    mawdi = mawdi.saturating_add(khutwa.saturating_add(1));
                    continue;
                }
                if harf == '$' && baad == Some('{') {
                    // The brace depth is recorded as it stands, not incremented:
                    // the substitution's own closing brace arrives at exactly
                    // this depth, and every `{` nested inside it raises and
                    // lowers the depth around that. Recording depth+1 here is
                    // the mistake that makes a template with an object literal
                    // in it swallow the rest of the file.
                    fiha_istibdal = true;
                    mukaddas.push(umq);
                    halat = HalatMash::Shifra;
                    yasmah = true;
                    mawdi = mawdi.saturating_add(2);
                    continue;
                }
                if harf == '`' {
                    if !fiha_istibdal {
                        sajjil(&jari, &mut tarteeb);
                    }
                    jari.clear();
                    halat = HalatMash::Shifra;
                    yasmah = false;
                } else {
                    jari.push(harf);
                }
            }
            HalatMash::TaabirNamati => {
                if harf == '\\' {
                    mawdi = mawdi.saturating_add(2);
                    continue;
                }
                if harf == '/' {
                    halat = HalatMash::Shifra;
                    yasmah = false;
                } else if harf == '\n' {
                    halat = HalatMash::Shifra;
                    yasmah = true;
                }
            }
        }
        mawdi = mawdi.saturating_add(1);
    }
    hasad
}

/// What one extraction pass produced, including what it did not read.
///
/// The skipped list is not decoration. An extraction that quietly ignored the
/// one file holding the game's dialogue looks exactly like an extraction of a
/// game with no dialogue, and the difference has to be visible before a
/// translator spends a week on the wrong list.
#[derive(Debug, Clone, Default)]
pub struct HasadNusus {
    /// Every string found, in archive order.
    pub nusus: Vec<NassMustakhraj>,
    /// How many files were read.
    pub maqrua: usize,
    /// Files that were not read, each with the reason.
    pub matruka: Vec<(String, String)>,
}

impl HasadNusus {
    /// How many strings came from a resource file rather than from a script.
    #[must_use]
    pub fn adad_mawarid(&self) -> usize {
        self.nusus.iter().filter(|nass| nass.naw.daqiq()).count()
    }

    /// The lines this pass contributes to a log or a diagnostics bundle.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "{} string(s) from {} file(s): {} exact from resources, {} heuristic from scripts",
            self.nusus.len(),
            self.maqrua,
            self.adad_mawarid(),
            self.nusus.len().saturating_sub(self.adad_mawarid())
        )];
        for (masar, sabab) in &self.matruka {
            sutur.push(format!("  skipped {masar}: {sabab}"));
        }
        sutur
    }
}

/// File names that are the application's own bookkeeping rather than its text.
const MALAFAT_IDARIYA: [&str; 4] =
    ["package.json", "package-lock.json", "yarn.lock", "npm-shrinkwrap.json"];

/// Whether an archive path is one this module reads for strings.
fn yumash(madkhal: &MadkhalAsar) -> bool {
    // The trailing slash matters: without it a game file called `taaribat.js`
    // would be mistaken for one of Taarib's own and silently skipped.
    if !madkhal.mahzum() || madkhal.masar.starts_with(&format!("{MUJALLAD_HIML}/")) {
        return false;
    }
    if madkhal.masar.contains("node_modules/") {
        return false;
    }
    let ism = madkhal.masar.rsplit('/').next().unwrap_or(&madkhal.masar);
    if MALAFAT_IDARIYA.contains(&ism) {
        return false;
    }
    matches!(madkhal.imtidad().as_str(), "json" | "js" | "mjs" | "cjs")
}

/// Every translatable string in an application, from its resources and its
/// scripts.
///
/// Resource files come first in the result and are marked [`NawNass::Mawrid`];
/// script literals follow and are marked [`NawNass::Barmaji`]. The distinction
/// is carried all the way to the translator, because the two lists have
/// completely different error rates and presenting them as one would hide that.
///
/// `node_modules/` is skipped entirely: it is somebody else's English, it is
/// enormous, and translating a dependency's console warnings has never helped a
/// player.
///
/// # Errors
///
/// [`KhataNusus::NassGhayrSalih`] when a file this module reads is not valid
/// UTF-8 — refused rather than decoded lossily, because a replacement character
/// in a translation source is a character a translator will faithfully carry
/// into the translation. [`KhataNusus::HajmMufrit`] when a single script is
/// larger than [`AQSA_NASS_BARMAJI`].
pub fn istakhrij(hawiya: &HawiyatAsar) -> Result<HasadNusus, KhataNusus> {
    let mut hasad = HasadNusus::default();
    let mut mash: usize = 0;

    for marra in [true, false] {
        for madkhal in hawiya.madakhil() {
            let json = madkhal.imtidad() == "json";
            if json != marra || !yumash(madkhal) {
                continue;
            }
            let Some(bayt) = hawiya.muhtawa(&madkhal.masar) else {
                continue;
            };
            if bayt.len() > AQSA_NASS_BARMAJI {
                return Err(KhataNusus::HajmMufrit {
                    haql: "a script inside the asar",
                    qeema: tul_u64(bayt.len()),
                    saqf: tul_u64(AQSA_NASS_BARMAJI),
                });
            }
            if mash.saturating_add(bayt.len()) > AQSA_MASH_KULLI {
                hasad.matruka.push((
                    madkhal.masar.clone(),
                    format!(
                        "the {AQSA_MASH_KULLI}-byte scan budget was already spent on earlier \
                         files"
                    ),
                ));
                continue;
            }
            let munaqqa = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bayt);
            let nass =
                std::str::from_utf8(munaqqa).map_err(|khata| KhataNusus::NassGhayrSalih {
                    malaf: madkhal.masar.clone(),
                    tarmiz: "UTF-8",
                    mawqi: tul_u64(khata.valid_up_to()),
                })?;
            mash = mash.saturating_add(bayt.len());
            hasad.maqrua = hasad.maqrua.saturating_add(1);

            if json {
                match serde_json::from_str::<Value>(nass) {
                    Ok(qeema) => hasad.nusus.extend(nusus_min_json(&madkhal.masar, &qeema)),
                    Err(khata) => hasad.matruka.push((
                        madkhal.masar.clone(),
                        format!("named .json and does not parse as JSON: {khata}"),
                    )),
                }
            } else {
                hasad.nusus.extend(nusus_min_js(&madkhal.masar, nass));
            }
        }
    }
    Ok(hasad)
}

// ---------------------------------------------------------------------------
// Delivery
// ---------------------------------------------------------------------------

/// The base64 alphabet, RFC 4648.
const HURUF_SITTASI: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes bytes as base64, for embedding a font in a data URL.
///
/// Written here rather than pulled in as a dependency because it is twenty
/// lines and this crate's dependency list is deliberately short. No division:
/// the whole transform is shifts and masks over three-byte groups.
#[must_use]
pub fn sittasi_arbaa(bayt: &[u8]) -> String {
    let taqdeer = bayt.len().saturating_add(bayt.len() >> 1).saturating_add(4);
    let mut nass = String::with_capacity(taqdeer);
    let baqi_tul = bayt.chunks_exact(3).remainder().len();
    {
        let mut harf = |sitta: u32| {
            let fahras = usize::try_from(sitta & 63).unwrap_or(0);
            nass.push(char::from(HURUF_SITTASI.get(fahras).copied().unwrap_or(b'A')));
        };
        let mut kutal = bayt.chunks_exact(3);
        for juz in kutal.by_ref() {
            let qeema = (u32::from(juz.first().copied().unwrap_or(0)) << 16)
                | (u32::from(juz.get(1).copied().unwrap_or(0)) << 8)
                | u32::from(juz.get(2).copied().unwrap_or(0));
            harf(qeema >> 18);
            harf(qeema >> 12);
            harf(qeema >> 6);
            harf(qeema);
        }
        let baqi = kutal.remainder();
        if baqi_tul == 1 {
            let qeema = u32::from(baqi.first().copied().unwrap_or(0)) << 16;
            harf(qeema >> 18);
            harf(qeema >> 12);
        } else if baqi_tul == 2 {
            let qeema = (u32::from(baqi.first().copied().unwrap_or(0)) << 16)
                | (u32::from(baqi.get(1).copied().unwrap_or(0)) << 8);
            harf(qeema >> 18);
            harf(qeema >> 12);
            harf(qeema >> 6);
        }
    }
    if baqi_tul == 1 {
        nass.push_str("==");
    } else if baqi_tul == 2 {
        nass.push('=');
    }
    nass
}

/// The Arabic font the patch ships, ready to become a data URL.
#[derive(Debug, Clone)]
pub struct KhattMudmaj {
    /// The family name the runtime registers it under.
    pub ism: String,
    /// The MIME type for the data URL: `font/ttf`, `font/woff2`, `font/otf`.
    pub naw: String,
    /// The font file, whole.
    pub bayt: Vec<u8>,
}

impl KhattMudmaj {
    /// The font as a `data:` URL.
    ///
    /// A data URL and never a file URL or an HTTPS one, and the reason is a
    /// behavioural guarantee rather than a convenience: a patch that made the
    /// game fetch something would have changed the game's network behaviour, and
    /// "installing the Arabic patch made my game talk to a server" is not a
    /// sentence this project will ever have to answer for. It also means the
    /// font works with the file:// origin, offline, and behind a content
    /// security policy that forbids remote font sources.
    #[must_use]
    pub fn rabt(&self) -> String {
        format!("data:{};base64,{}", self.naw, sittasi_arbaa(&self.bayt))
    }
}

/// Everything the runtime needs, assembled on the Rust side.
#[derive(Debug, Clone)]
pub struct HimlGhilaf {
    /// The rung the probe settled on. Gates the canvas takeover in the runtime.
    pub rutba: Rutba,
    /// Source text to Arabic, keyed by the exact source string.
    ///
    /// Keyed by text and not by [`NassMustakhraj::huwiya`] because the runtime
    /// matches what it finds in the DOM, and what it finds in the DOM is text.
    /// The identity is the translator's key; this is the runtime's.
    pub jadwal: BTreeMap<String, String>,
    /// The bundled font, when the patch ships one.
    pub khatt: Option<KhattMudmaj>,
    /// The compiled renderer runtime — `adapters-script/electron/taarib.ts`
    /// after the build has run over it.
    ///
    /// This crate never compiles TypeScript. It embeds what the build produced,
    /// verbatim, and the file it produces is readable JavaScript rather than a
    /// generated blob.
    pub tashghil: String,
}

impl HimlGhilaf {
    /// The payload as the JSON the runtime reads.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HimlMarfud`] when the payload cannot be serialized, which
    /// in practice means a string table larger than the process can hold.
    pub fn hamula(&self) -> Result<Vec<u8>, KhataNusus> {
        let mut jadwal = Map::new();
        for (asl, tarjama) in &self.jadwal {
            let _ = jadwal.insert(asl.clone(), Value::String(tarjama.clone()));
        }
        let mut jidhr = Map::new();
        let _ = jidhr.insert("isdar".to_owned(), Value::from(1_u64));
        let _ = jidhr.insert("rutba".to_owned(), Value::from(u64::from(self.rutba as u8)));
        let _ = jidhr.insert("ittijah".to_owned(), Value::String("rtl".to_owned()));
        let _ = jidhr.insert("jadwal".to_owned(), Value::Object(jadwal));
        if let Some(khatt) = self.khatt.as_ref() {
            let mut kutla = Map::new();
            let _ = kutla.insert("ism".to_owned(), Value::String(khatt.ism.clone()));
            let _ = kutla.insert("rabt".to_owned(), Value::String(khatt.rabt()));
            let _ = jidhr.insert("khatt".to_owned(), Value::Object(kutla));
        }
        serde_json::to_vec(&Value::Object(jidhr)).map_err(|khata| KhataNusus::HimlMarfud {
            alia: "asar payload",
            sabab: format!("the runtime payload could not be serialized: {khata}"),
        })
    }
}

/// The Electron main-process shim.
///
/// It registers the preload through Electron's own mechanisms and then loads the
/// application's original entry point. It never replaces a preload the
/// application already set: `registerPreloadScript` and `setPreloads` both add
/// to a list, and the `BrowserWindow` fallback only fills in a `preload` that
/// was empty. A patch that displaced the game's own preload would break the
/// game's own IPC.
const TAMHID: &str = r#"'use strict';
// تمهيد — Taarib's Electron entry shim.
//
// package.json's "main" points here; the application's original entry point is
// recorded below and required at the end of this file, unchanged. Everything
// between is one job: make sure every renderer gets Taarib's preload, without
// removing anything the application set up for itself.
const path = require('node:path');
const ASL = __TAARIB_ASL__;
const TAWTIA = path.join(__dirname, 'tawtia.js');

function sajjil(ses) {
  if (!ses) return;
  try {
    // Electron 35 and later. `type: "frame"` runs it in every frame, which is
    // what a game with an iframe-based UI needs.
    if (typeof ses.registerPreloadScript === 'function') {
      ses.registerPreloadScript({ type: 'frame', filePath: TAWTIA });
      return;
    }
  } catch (khata) {
    console.warn('[taarib] registerPreloadScript refused:', khata);
  }
  try {
    if (typeof ses.setPreloads === 'function') {
      const sabiq = typeof ses.getPreloads === 'function' ? ses.getPreloads() : [];
      if (!sabiq.includes(TAWTIA)) ses.setPreloads(sabiq.concat([TAWTIA]));
    }
  } catch (khata) {
    console.warn('[taarib] setPreloads refused:', khata);
  }
}

try {
  const electron = require('electron');
  const app = electron.app;
  const BrowserWindow = electron.BrowserWindow;

  if (app && typeof app.on === 'function') {
    app.on('session-created', sajjil);
    const jahiz = () => {
      try {
        sajjil(electron.session && electron.session.defaultSession);
      } catch (khata) {
        console.warn('[taarib] the default session was not reachable:', khata);
      }
    };
    if (app.isReady && app.isReady()) jahiz();
    else app.on('ready', jahiz);
  }

  // Last resort, and only for a window the application left without a preload:
  // some applications build a session per window and never touch the default.
  if (typeof BrowserWindow === 'function') {
    const Asli = BrowserWindow;
    const Badeel = function TaaribBrowserWindow(khiyarat) {
      const idad = khiyarat || {};
      const web = idad.webPreferences || (idad.webPreferences = {});
      if (!web.preload) web.preload = TAWTIA;
      return Reflect.construct(Asli, [idad], new.target || Badeel);
    };
    Badeel.prototype = Asli.prototype;
    Object.setPrototypeOf(Badeel, Asli);
    try {
      Object.defineProperty(electron, 'BrowserWindow', {
        value: Badeel,
        configurable: true,
        enumerable: true,
        writable: true,
      });
    } catch (khata) {
      console.warn('[taarib] BrowserWindow could not be wrapped:', khata);
    }
  }
} catch (khata) {
  // Not Electron, or an Electron whose main module is not reachable from here.
  // The application still has to start, so this is a warning and not a throw.
  console.warn('[taarib] the Electron main module was not reachable:', khata);
}

module.exports = require(path.join(__dirname, '..', ASL));
"#;

/// The bootstrap that runs inside the renderer.
///
/// Under Electron this is a preload script; under NW.js it is the manifest's
/// `inject-js-start`. Both run before the page's own scripts, which is the whole
/// requirement — the runtime has to be installed before the game draws its first
/// frame or the first frame is unshaped.
///
/// It reads the payload as **data** and requires the runtime as **code**, and
/// the two never swap roles: `JSON.parse` for the string table, `require` for
/// the runtime, and — on the rung-three path where the hook has to live in the
/// page's own world — a `<script src=...>` pointing at a real file rather than a
/// script whose body was assembled from a string.
const TAWTIA: &str = r"'use strict';
// توطئة — Taarib's renderer bootstrap.
//
// Runs before the page's own scripts. Two loads and nothing else: the payload
// is parsed as JSON, the runtime is required as a module. No patch content ever
// reaches a code path.
const fs = require('node:fs');
const path = require('node:path');

const JIDHR = __dirname;
let hamula = null;
try {
  hamula = JSON.parse(fs.readFileSync(path.join(JIDHR, 'hamula.json'), 'utf8'));
} catch (khata) {
  console.warn('[taarib] the payload could not be read:', khata);
}

if (hamula) {
  try {
    require(path.join(JIDHR, 'taarib.js')).rakkib(hamula, 'maazul');
  } catch (khata) {
    console.warn('[taarib] the runtime declined to install:', khata);
  }

  // Rung three only. The canvas hook has to be installed on the page's own
  // CanvasRenderingContext2D, and a preload's isolated world has a different
  // one, so the runtime is loaded a second time by URL into the page's world.
  // By URL and never as a string body: if a content security policy refuses
  // it, this branch declines and says so rather than reaching for eval.
  if (hamula.rutba === 3) {
    const rakkibFiSafha = () => {
      const jidhrSafha = document.documentElement || document.head || document.body;
      if (!jidhrSafha) return false;
      const bayanat = document.createElement('script');
      bayanat.type = 'application/json';
      bayanat.id = 'taarib-hamula';
      // textContent, never innerHTML: the payload is data and is inert here.
      bayanat.textContent = JSON.stringify(hamula);
      jidhrSafha.appendChild(bayanat);

      const barnamaj = document.createElement('script');
      barnamaj.src = 'file://' + path.join(JIDHR, 'taarib.js').replace(/\\/g, '/');
      barnamaj.dataset.taaribNitaq = 'safha';
      barnamaj.onerror = () => {
        console.warn(
          '[taarib] the page refused to load the runtime, so canvas text is left to the ' +
            'game. Nothing was evaluated from a string in its place.'
        );
      };
      jidhrSafha.appendChild(barnamaj);
      return true;
    };
    if (!rakkibFiSafha()) {
      const muraqib = new MutationObserver(() => {
        if (rakkibFiSafha()) muraqib.disconnect();
      });
      muraqib.observe(document, { childList: true, subtree: true });
    }
  }
}
";

/// What an install did, named file by file.
#[derive(Debug, Clone)]
pub struct TaqreerTarkeeb {
    /// Which of the application's own entry mechanisms was used.
    pub alia: &'static str,
    /// The application's original entry point, recorded so an uninstall can put
    /// it back and so a second install does not chain onto the first.
    pub asl: String,
    /// Every archive path this install wrote.
    pub malafat: Vec<String>,
}

/// The manifest member that remembers the application's own entry point.
const HAQL_ASL: &str = "taaribAsl";

/// Installs the payload into an archive, through the application's own entry
/// mechanism.
///
/// Two mechanisms, chosen from what the manifest already says rather than from a
/// guess about which shell this is:
///
/// * **`main` names a script** — Electron. `main` is repointed at
///   `taarib/tamhid.js`, which registers the preload on every session and then
///   requires the original entry unchanged.
/// * **`main` names an HTML document** — NW.js, where `main` is a page and not a
///   module. `main` is left exactly as it is and `inject-js-start` is set to the
///   bootstrap, which NW.js runs before the page's own scripts.
///
/// Re-installing is idempotent: the original entry is remembered in
/// [`HAQL_ASL`], so a second install repoints at the same original rather than
/// chaining a shim onto a shim.
///
/// # Errors
///
/// [`KhataNusus::HimlMarfud`] when `package.json` is absent, unparseable, not an
/// object, or names an entry this module cannot register behind — a rung
/// declining, so the caller can fall back to an unpacked `resources/app/`.
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] or [`KhataNusus::HajmMufrit`] from
/// [`HawiyatAsar::daa`] when a payload path collides with the application's own
/// tree.
pub fn rakkib(
    hawiya: &mut HawiyatAsar,
    himl: &HimlGhilaf,
) -> Result<TaqreerTarkeeb, KhataNusus> {
    let marfud = |sabab: String| KhataNusus::HimlMarfud { alia: "asar entry point", sabab };

    let khaam = hawiya
        .muhtawa("package.json")
        .ok_or_else(|| marfud("the archive has no packed package.json".to_owned()))?
        .to_vec();
    let munaqqa = khaam.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&khaam);
    let mut huzma: Value = serde_json::from_slice(munaqqa)
        .map_err(|khata| marfud(format!("package.json does not parse as JSON: {khata}")))?;
    let kain = huzma
        .as_object_mut()
        .ok_or_else(|| marfud("package.json is not a JSON object".to_owned()))?;

    // A previous install recorded the real entry point. Use it, so a reinstall
    // repoints at the application rather than at the last shim.
    let asl = match kain.get(HAQL_ASL).and_then(Value::as_str) {
        Some(sabiq) => sabiq.to_owned(),
        None => kain.get("main").and_then(Value::as_str).unwrap_or("index.js").to_owned(),
    };
    if asl.trim().is_empty() {
        return Err(marfud("package.json names an empty entry point".to_owned()));
    }
    let safha = asl.rsplit_once('.').is_some_and(|(_, imtidad)| {
        imtidad.eq_ignore_ascii_case("html") || imtidad.eq_ignore_ascii_case("htm")
    });

    let mut malafat = Vec::new();
    let mut uktub = |hawiya: &mut HawiyatAsar,
                     ism: &str,
                     bayt: Vec<u8>|
     -> Result<(), KhataNusus> {
        let masar = format!("{MUJALLAD_HIML}/{ism}");
        hawiya.daa(&masar, bayt)?;
        malafat.push(masar);
        Ok(())
    };
    uktub(hawiya, "hamula.json", himl.hamula()?)?;
    uktub(hawiya, "taarib.js", himl.tashghil.clone().into_bytes())?;
    uktub(hawiya, "tawtia.js", TAWTIA.as_bytes().to_vec())?;

    let alia = if safha {
        // NW.js: `main` is a page. Repointing it would break the application;
        // `inject-js-start` is the shell's own hook for exactly this.
        let _ = kain.insert(
            "inject-js-start".to_owned(),
            Value::String(format!("{MUJALLAD_HIML}/tawtia.js")),
        );
        "inject-js-start"
    } else {
        let tamhid = TAMHID.replace(
            "__TAARIB_ASL__",
            &serde_json::to_string(&asl)
                .map_err(|khata| marfud(format!("the entry point could not be quoted: {khata}")))?,
        );
        uktub(hawiya, "tamhid.js", tamhid.into_bytes())?;
        let _ = kain
            .insert("main".to_owned(), Value::String(format!("{MUJALLAD_HIML}/tamhid.js")));
        "main"
    };
    let _ = kain.insert(HAQL_ASL.to_owned(), Value::String(asl.clone()));

    let mutabbaa = serde_json::to_vec_pretty(&huzma)
        .map_err(|khata| marfud(format!("package.json could not be rewritten: {khata}")))?;
    hawiya.daa("package.json", mutabbaa)?;
    malafat.push("package.json".to_owned());

    Ok(TaqreerTarkeeb { alia, asl, malafat })
}

// ---------------------------------------------------------------------------
// The tier probe
// ---------------------------------------------------------------------------

/// A marker looked for in the application's own scripts.
///
/// `(pattern, what finding it means, which rung it points at, weight)`.
type Alama = (&'static str, &'static str, Option<Rutba>, u8);

/// Markers that mean the browser's own text stack never sees the game's text.
///
/// Each of these is a way of putting characters on screen that bypasses layout
/// and therefore bypasses `HarfBuzz`: a bitmap font blitted glyph by glyph, a
/// signed-distance-field text mesh, or a layout library the game bundled because
/// it decided not to use the browser's.
const ALAMAT_ISTILA: [Alama; 12] = [
    (
        "BitmapText",
        "a bitmap-font text object, which blits one pre-rendered image per character and \
         never asks the shaper what the characters should look like together",
        Some(Rutba::Istila),
        78,
    ),
    (
        "bitmapFont",
        "a bitmap font resource, whose glyphs are indexed by codepoint rather than by shaped \
         glyph identifier",
        Some(Rutba::Istila),
        70,
    ),
    (
        "createTextureFromChar",
        "a per-character texture cache, which is per-character rendering by definition",
        Some(Rutba::Istila),
        76,
    ),
    (
        "troika-three-text",
        "a WebGL text library that lays out and rasterizes on its own",
        Some(Rutba::Istila),
        80,
    ),
    (
        "three-bmfont-text",
        "a WebGL bitmap-font text library",
        Some(Rutba::Istila),
        80,
    ),
    (
        "opentype.js",
        "a bundled font parser, which a game only ships when it is positioning glyphs itself",
        Some(Rutba::Istila),
        82,
    ),
    (
        "harfbuzzjs",
        "a bundled shaper: the game shapes text itself instead of letting the browser do it",
        Some(Rutba::Istila),
        85,
    ),
    (
        "arabic-reshaper",
        "a presentation-form reshaper, which is the wrong answer to Arabic and means the \
         game is not using the browser's shaping at all",
        Some(Rutba::Istila),
        85,
    ),
    (
        "ArabicReshaper",
        "a presentation-form reshaper under its other spelling",
        Some(Rutba::Istila),
        85,
    ),
    (
        "unicode-bidirectional",
        "a bundled bidirectional algorithm, which means the game reorders text itself",
        Some(Rutba::Istila),
        80,
    ),
    (
        "bidi-js",
        "a bundled bidirectional algorithm under its other package name",
        Some(Rutba::Istila),
        80,
    ),
    (
        "msdf",
        "multi-channel distance-field text, which is glyph-image rendering on the GPU",
        Some(Rutba::Istila),
        66,
    ),
];

/// Markers that mean the game's text is DOM text.
///
/// DOM text is laid out by Blink, shaped by `HarfBuzz` and reordered by the full
/// bidirectional algorithm. Finding these is finding rung one.
///
/// The weights are small on purpose. `textContent` and `querySelector` appear in
/// every bundle ever minified, including bundles belonging to games that draw
/// every character as a sprite, so they are worth a nudge and not a verdict.
/// Together they cannot outweigh a single conclusive takeover marker, which is
/// the arithmetic that keeps a Phaser game with `BitmapText` from being read as
/// a React application because both of them call `querySelector`.
const ALAMAT_IDAD: [Alama; 6] = [
    (
        "createTextNode",
        "text is put into the document as real text nodes, which Blink shapes and reorders",
        Some(Rutba::Idad),
        18,
    ),
    (
        "textContent",
        "text is assigned to DOM nodes, which is the shaped-and-reordered path",
        Some(Rutba::Idad),
        12,
    ),
    ("innerText", "text is assigned to DOM nodes", Some(Rutba::Idad), 8),
    (
        "react-dom",
        "a DOM-rendering framework, so the interface is document text",
        Some(Rutba::Idad),
        22,
    ),
    ("createApp", "a DOM-rendering framework's entry point", Some(Rutba::Idad), 10),
    ("querySelector", "the document is the interface", Some(Rutba::Idad), 6),
];

/// Markers that are context and are deliberately not evidence.
///
/// `fillText` is the important one. A game that paints its dialogue onto a
/// canvas with `fillText` *is still shaped*: canvas text goes through the same
/// shaper the layout engine uses, joins its letters, applies its ligatures and
/// resolves the bidirectional levels of the string it is given. It loses
/// selection and accessibility, which the canvas cost the game and not the
/// patch. So finding `fillText` is not a reason to take the engine over, and
/// recording it as though it were is exactly the mistake this module was
/// written to avoid.
const ALAMAT_SIYAQ: [Alama; 3] = [
    (
        "fillText",
        "canvas text drawing, which still goes through the browser's shaper and is therefore \
         not a reason to take anything over",
        Some(Rutba::Idad),
        12,
    ),
    ("measureText", "canvas text measurement", None, 0),
    ("direction", "an explicit text direction is set somewhere in the application", None, 0),
];

/// Extensions the probe reads while gathering evidence.
const IMTIDADAT_MASH: [&str; 5] = ["js", "mjs", "cjs", "html", "htm"];

/// What one sweep of an application's scripts saw.
#[derive(Debug, Default)]
struct MasahGhilaf {
    /// Every marker that matched, with how many files it matched in.
    isabat: BTreeMap<&'static str, usize>,
    /// How many files were read.
    malafat: usize,
    /// How many bytes were read.
    bayt: usize,
    /// Whether the sweep stopped at a budget rather than at the end.
    mabtur: bool,
    /// Files that draw with `drawImage` and index the string a character at a
    /// time, and never call `fillText`. That combination is per-character
    /// blitting and is the one shape the marker list alone would not catch.
    blit_bi_harf: usize,
}

impl MasahGhilaf {
    /// Folds one file's text into the sweep.
    fn ibla(&mut self, nass: &str) {
        self.malafat = self.malafat.saturating_add(1);
        self.bayt = self.bayt.saturating_add(nass.len());
        for (namat, _, _, _) in
            ALAMAT_ISTILA.iter().chain(ALAMAT_IDAD.iter()).chain(ALAMAT_SIYAQ.iter())
        {
            if nass.contains(*namat) {
                let adad = self.isabat.entry(*namat).or_insert(0);
                *adad = adad.saturating_add(1);
            }
        }
        let bi_harf = nass.contains("charCodeAt")
            || nass.contains("codePointAt")
            || nass.contains("charAt");
        if bi_harf && nass.contains("drawImage") && !nass.contains("fillText") {
            self.blit_bi_harf = self.blit_bi_harf.saturating_add(1);
        }
    }

    /// Whether there is room in the budget for more.
    const fn yattasi(&self) -> bool {
        self.bayt < AQSA_MASH_KULLI && self.malafat < AQSA_MADAKHIL
    }
}

/// Where an application's own files were found.
const ARSHIFA: [&str; 2] = ["resources/app.asar", "Contents/Resources/app.asar"];

/// Where an unpacked application's files were found.
const MUJALLADAT: [&str; 2] = ["resources/app", "Contents/Resources/app"];

/// Finds the packed application archive under a game root.
///
/// Both layouts a shell ships in: `resources/` on Windows and Linux, and the
/// same directory inside `Contents/` on macOS. The order is the order
/// [`MifhasGhilaf`] sweeps them in, and it is the same list, because two
/// answers to "where is this application's archive" is one answer too many.
///
/// Returns [`None`] for an application shipped unpacked as `resources/app/`.
/// That is a different install route — the files are already on disk and are
/// patched as files — and reporting it as "no archive" is what lets a caller
/// tell the two apart instead of failing to open a directory as an archive.
///
/// Both layouts are resolved through [`crate::tarkeeb::masar_bila_hala`], not
/// joined literally. `Resources/` is the spelling Electron itself uses on macOS
/// and the one a repacker reaches for on any platform, and a Linux `Path::join`
/// asking for `resources/` beside it finds nothing — which [`crate::ayn_hadaf`]
/// reads as "not an Electron game" and the installer reports as a success
/// having translated nothing.
#[must_use]
pub fn hawiya(jidhr: &Path) -> Option<PathBuf> {
    ARSHIFA
        .iter()
        .map(|nisbi| masar_bila_hala(jidhr, nisbi))
        .find(|murashah| murashah.is_file())
}

/// The shaping-tier probe for Chromium-shelled games.
///
/// Its answer is [`Rutba::Idad`] for very nearly every game it will ever see,
/// and that is not laziness — it is what the renderer being Chromium means. The
/// probe exists to catch the exception, and it looks for the exception in the
/// only place the exception is visible: the shipped scripts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MifhasGhilaf;

impl MifhasGhilaf {
    /// A probe. Holds nothing, so constructing one costs nothing.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// Sweeps an archive's scripts.
    fn ifhas_hawiya(masar: &Path, masah: &mut MasahGhilaf) -> Option<String> {
        let hawiya = HawiyatAsar::min_masar(masar).ok()?;
        for madkhal in hawiya.madakhil() {
            if !masah.yattasi() {
                masah.mabtur = true;
                break;
            }
            if !madkhal.mahzum()
                || madkhal.masar.contains("node_modules/")
                || !IMTIDADAT_MASH.contains(&madkhal.imtidad().as_str())
            {
                continue;
            }
            let Some(bayt) = hawiya.muhtawa(&madkhal.masar) else { continue };
            let mahdud = bayt.get(..bayt.len().min(AQSA_NASS_BARMAJI)).unwrap_or(bayt);
            masah.ibla(&String::from_utf8_lossy(mahdud));
        }
        Some(format!("{} entries", hawiya.madakhil().len()))
    }

    /// Sweeps an unpacked application directory.
    fn ifhas_mujallad(masar: &Path, masah: &mut MasahGhilaf) {
        for madkhal in walkdir::WalkDir::new(masar)
            .max_depth(AQSA_UMQ)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !masah.yattasi() {
                masah.mabtur = true;
                break;
            }
            if !madkhal.file_type().is_file() {
                continue;
            }
            let masar_madkhal = madkhal.path();
            if masar_madkhal.components().any(|juz| juz.as_os_str() == "node_modules") {
                continue;
            }
            let imtidad = masar_madkhal
                .extension()
                .and_then(|juz| juz.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if !IMTIDADAT_MASH.contains(&imtidad.as_str()) {
                continue;
            }
            let Ok(bayt) = fs::read(masar_madkhal) else { continue };
            let mahdud = bayt.get(..bayt.len().min(AQSA_NASS_BARMAJI)).unwrap_or(&bayt);
            masah.ibla(&String::from_utf8_lossy(mahdud));
        }
    }
}

impl Mifhas for MifhasGhilaf {
    fn hadaf(&self) -> HadafNusus {
        HadafNusus::Ghilaf
    }

    fn yantabiq(&self, siyaq: &SiyaqTabaqa<'_>) -> bool {
        if siyaq.aila == taarib_mustalahat::muharrik::AilatMuharrik::Electron {
            return true;
        }
        ARSHIFA
            .iter()
            .chain(MUJALLADAT.iter())
            .any(|nisbi| masar_bila_hala(siyaq.jidhr, nisbi).exists())
    }

    fn adilla(&self, siyaq: &SiyaqTabaqa<'_>) -> Result<Vec<Dalil>, KhataNusus> {
        let mut adilla = vec![
            Dalil::jadeed(
                "nusus:ghilaf",
                "the renderer is Chromium, which shapes Arabic with HarfBuzz and applies the \
                 full Unicode bidirectional algorithm to everything it lays out",
                Some(Rutba::Idad),
                55,
            ),
            Dalil::siyaq(
                "nusus:ghilaf",
                "rung one is the expected verdict for this engine. Rung three would mean the \
                 game bypassed the browser's own text stack, which is unusual enough that the \
                 evidence below is what a reviewer should read before acting on it",
            ),
        ];

        let mut masah = MasahGhilaf::default();
        let mut masdar = String::new();
        for nisbi in ARSHIFA {
            let masar = masar_bila_hala(siyaq.jidhr, nisbi);
            if !masar.is_file() {
                continue;
            }
            match Self::ifhas_hawiya(&masar, &mut masah) {
                Some(wasf) => masdar = format!("{nisbi} ({wasf})"),
                None => adilla.push(Dalil::siyaq(
                    nisbi,
                    "an application archive is present and could not be read, so nothing was \
                     concluded from its contents",
                )),
            }
            break;
        }
        if masdar.is_empty() {
            for nisbi in MUJALLADAT {
                let masar = masar_bila_hala(siyaq.jidhr, nisbi);
                if !masar.is_dir() {
                    continue;
                }
                Self::ifhas_mujallad(&masar, &mut masah);
                masdar = format!("{nisbi}/");
                break;
            }
        }

        if masah.malafat == 0 {
            adilla.push(Dalil::siyaq(
                "nusus:ghilaf",
                "no application script could be read, so the verdict rests on the shell alone",
            ));
            return Ok(adilla);
        }
        adilla.push(Dalil::siyaq(
            if masdar.is_empty() { "nusus:ghilaf" } else { masdar.as_str() },
            format!(
                "{} script(s) totalling {} byte(s) were read{}",
                masah.malafat,
                masah.bayt,
                if masah.mabtur { ", and the sweep stopped at its budget" } else { "" }
            ),
        ));

        for (namat, wasf, yushir, thiqa) in
            ALAMAT_ISTILA.iter().chain(ALAMAT_IDAD.iter()).chain(ALAMAT_SIYAQ.iter())
        {
            let Some(adad) = masah.isabat.get(namat) else { continue };
            adilla.push(Dalil::jadeed(
                format!("script marker {namat}"),
                format!("found in {adad} file(s): {wasf}"),
                *yushir,
                *thiqa,
            ));
        }
        if masah.blit_bi_harf > 0 {
            adilla.push(Dalil::jadeed(
                "script shape",
                format!(
                    "{} file(s) walk a string a character at a time and draw with drawImage \
                     while never calling fillText, which is per-character blitting: the \
                     browser's shaper is not involved and Arabic letters would not join",
                    masah.blit_bi_harf
                ),
                Some(Rutba::Istila),
                80,
            ));
        }
        Ok(adilla)
    }
}









