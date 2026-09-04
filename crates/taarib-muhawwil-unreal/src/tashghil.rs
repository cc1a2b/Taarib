//! التشغيل — switching Slate's real text path on, and keeping it on.
//!
//! Slate ships `HarfBuzz` and ICU and does not use them by default. Its shaping
//! method is a console variable, and the cheap path — kerning only, no shaping
//! — is what a shipped game runs unless something says otherwise. Arabic drawn
//! through the kerning-only path is a run of isolated letter forms in logical
//! order: unjoined, and read left to right. That is the exact failure this
//! product exists to prevent, and flipping one console variable is the single
//! most consequential correction in the whole adapter. Everything else here —
//! the bidirectional detection, the watchdog, the ini writer — exists to make
//! sure that one variable is set, stays set, and can be unset again.
//!
//! ## The ladder, cheapest first
//!
//! Most of this does not need to be inside the game's process at all. Unreal
//! reads its console variables out of ini files during engine start, long
//! before the first font measure, and that route is additive, reversible,
//! survives a game update that replaces binaries, and never looks to an
//! anti-tamper system like something attaching to a process. `taarib-haqn` puts
//! it plainly — loading by override beats loading by force — and the same
//! reasoning applies one level down, to settings.
//!
//! 1. **Configuration.** `[SystemSettings]` in the game's `Engine.ini` is the
//!    documented Unreal mechanism for setting a console variable at startup:
//!    every `CVarName=Value` under that section is applied while the engine is
//!    initialising. Nothing is injected, nothing is hooked, and undoing it is
//!    deleting the lines this module marked.
//! 2. **Command line.** `-ini:Engine:[SystemSettings]:Key=Value` and
//!    `-dpcvars=` carry the same settings as launch options, for the games that
//!    regenerate their ini from a template on every start and would erase rung
//!    one before it could apply.
//! 3. **Injection.** A live console-variable write through [`Musaddir`], plus
//!    the re-assert watchdog, for the games that apply their own saved settings
//!    *after* engine start and overwrite whatever the ini said.
//!
//! Each rung is attempted **and then verified by reading the value back**, and
//! the rung recorded in [`SijillTashghil`] is the one whose effect was
//! confirmed, not merely the one that was attempted. Phase 7's resolver records
//! which rung fired for the same reason: a takeover that works is not the same
//! as a takeover that works for the right reason, and the difference is the
//! whole of the next bug report.
//!
//! ## The seam to `slate.rs`
//!
//! [`Musaddir`] is this module's only view of the live process. `slate.rs` is
//! expected to implement it over Slate's console manager and internationalisation
//! singleton; nothing in this file knows how that is done. Two methods are
//! required — set a console variable, read one back — and the rest carry
//! default bodies that refuse with [`KhataUnreal::SlateGhayrMawjud`], so an
//! implementation that has only reached the console manager still compiles and
//! still drives rungs one and two, which are the preferred rungs anyway.
//!
//! ## Writing into somebody's `Engine.ini`
//!
//! A half-written `Engine.ini` is a game that will not start, so the write is:
//! read the whole file, change only the keys Taarib owns, keep every other
//! line, comment, blank line and ordering byte-for-byte, and hand the result to
//! [`taarib_usus::masarat::kitaba_dharra_nass`], which writes a temporary file
//! in the same directory, flushes it to the device, and renames it into place.
//! An interruption anywhere leaves the original intact.
//!
//! Taarib's lines sit between [`ALAMAT_BIDAYA`] and [`ALAMAT_NIHAYA`], and a
//! game key that had to be displaced is commented out in place with
//! [`ALAMAT_MUATTAL`] rather than deleted. That makes the change findable and
//! makes [`MalafIni::tarajua`] an exact inverse: the block goes, the displaced
//! lines come back where they were, and the file is the file the game shipped.

use std::path::{Path, PathBuf};
use std::time::Duration;

use taarib_usus::masarat;

use crate::khata::{KhataUnreal, tul_u64};

// ---------------------------------------------------------------------------
// What is being set
// ---------------------------------------------------------------------------

/// Slate's shaping-method console variable.
///
/// `0` auto, `1` kerning only, `2` full shaping. Auto asks Slate to guess per
/// run and is not enough: a game that names a shaping method in its own text
/// style never reaches the guess, and the guess is bypassed entirely on the
/// cached-measurement path. Full shaping is set explicitly so the question does
/// not arise.
pub const MUTAGHAYYIR_SHAKL: &str = "Slate.DefaultTextShapingMethod";

/// The short name some engine versions and some game builds expose instead.
///
/// Written alongside the canonical name. An unrecognised key under
/// `[SystemSettings]` is inert — Unreal creates an unregistered console variable
/// for it and nothing reads it — so writing both costs one line and removes a
/// version-matrix question from the install path.
pub const MUTAGHAYYIR_SHAKL_BADIL: &str = "Slate.ShapingMethod";

/// Slate's text-flow console variable — its bidirectional detection.
///
/// `0` auto (detect direction from the text), `1` left to right, `2` right to
/// left. Auto is the detection Slate supports and does not always have on,
/// because a game that hard-codes left to right in its widget styles never asks
/// for it. Forcing `2` here would be wrong rather than aggressive: it would
/// reverse the Latin runs inside Arabic sentences — version numbers, player
/// names, key bindings. Correcting a game that hard-codes a direction is
/// `qiyas_unreal`'s job, not this one's.
pub const MUTAGHAYYIR_ITTIJAH: &str = "Slate.DefaultTextFlowDirection";

/// `ETextShapingMethod::FullShaping`.
pub const SHAKL_KAMIL: &str = "2";

/// `ETextFlowDirection::Auto` — detection on.
pub const ITTIJAH_TILQAI: &str = "0";

/// The ini section Unreal applies as console variables during engine start.
pub const QISM_NIZAM: &str = "SystemSettings";

/// Opens the block of lines Taarib owns.
pub const ALAMAT_BIDAYA: &str = "; taarib:bidaya";

/// Closes the block of lines Taarib owns.
pub const ALAMAT_NIHAYA: &str = "; taarib:nihaya";

/// Prefixes a game line Taarib had to displace, so undo can restore it exactly.
pub const ALAMAT_MUATTAL: &str = "; taarib:muattal ";

/// The largest ini this module will read.
///
/// An `Engine.ini` is kilobytes. Four megabytes is far past anything a real
/// game writes and is checked against the file's declared length before a byte
/// is read, because the file being parsed lives in a directory a game and its
/// launcher both write to.
pub const AQSA_HAJM_INI: u64 = 4 * 1024 * 1024;

// ---------------------------------------------------------------------------
// The watchdog's bound
// ---------------------------------------------------------------------------

/// How long the watchdog waits between checks.
pub const FASIL_HARIS: Duration = Duration::from_secs(5);

/// How many checks the watchdog performs after a successful assert.
///
/// Twelve at five seconds is one minute. Games that reset console variables do
/// it while applying their own saved configuration, which happens during engine
/// start and again the first time the settings screen is constructed — both
/// inside the first minute of a session in practice. A game that reset shaping
/// at minute forty would be doing it in response to something the player did,
/// and a poll cannot fix that; a hook can, and that is `qiyas_unreal`'s
/// territory.
pub const HADD_NABADAT: u32 = 12;

/// How many extra checks a re-assert buys.
///
/// A game that has overwritten the setting once is a game likely to do it
/// again, so the window extends rather than expiring on schedule.
pub const TAMDEED_NABADAT: u32 = 12;

/// The absolute ceiling on checks, however many re-asserts fire.
///
/// Sixty checks is five minutes and then the thread ends. This bound is the
/// point: a watchdog that polls forever inside somebody's game is a thread that
/// never sleeps for the life of the process, shows up in their profile, and is
/// its own bug. Five minutes of five-second wakeups is unmeasurable; an hour of
/// them is a support ticket.
pub const HADD_MUTLAQ_NABADAT: u32 = 60;

// ---------------------------------------------------------------------------
// The seam to slate.rs
// ---------------------------------------------------------------------------

/// The live process, as this module needs to see it.
///
/// Implemented by `slate.rs` over Slate's console manager and, where it can
/// reach them, the internationalisation and font singletons. Declared here
/// rather than there so that neither file waits on the other, and kept
/// deliberately small: everything below the first two methods has a default
/// body that refuses, because a partial implementation must still let rungs one
/// and two run.
pub trait Musaddir: Send + Sync {
    /// Whether the implementation has a usable handle on the engine right now.
    ///
    /// Checked before the injected rung is attempted, so that a module loaded
    /// into a process whose Slate has not initialised yet reports the ini rung
    /// honestly instead of reporting a failure.
    fn muhayya(&self) -> bool;

    /// Sets a console variable in the running process.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the console manager cannot be
    /// reached, and [`KhataUnreal::TashghilFashil`] when it was reached and
    /// refused the write.
    fn daa(&self, miftah: &str, qeema: &str) -> Result<(), KhataUnreal>;

    /// Reads a console variable back.
    ///
    /// The read-back is what separates "was written" from "took effect", and
    /// every rung in this module is judged on it.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the console manager cannot be
    /// reached or does not know the variable.
    fn iqra(&self, miftah: &str) -> Result<String, KhataUnreal>;

    /// Makes a culture active in the running process.
    ///
    /// Used by [`crate::alam`] on its third rung. The default refuses, so a
    /// `slate.rs` that has bound only the console manager is still a usable
    /// [`Musaddir`].
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the implementation does not reach
    /// the internationalisation singleton.
    fn thabbit_thaqafa(&self, wasm: &str) -> Result<(), KhataUnreal> {
        let _ = wasm;
        Err(KhataUnreal::SlateGhayrMawjud {
            sabab: "this Slate binding does not reach the internationalisation singleton"
                .to_owned(),
        })
    }

    /// The culture the process considers active.
    ///
    /// # Errors
    ///
    /// As [`Musaddir::thabbit_thaqafa`].
    fn thaqafa_haliya(&self) -> Result<String, KhataUnreal> {
        Err(KhataUnreal::SlateGhayrMawjud {
            sabab: "this Slate binding does not reach the internationalisation singleton"
                .to_owned(),
        })
    }

    /// Registers one sub-font's bytes with Slate's font system.
    ///
    /// The bytes are always the patch's own. `nitaqat` are inclusive codepoint
    /// ranges and `thaqafat` the culture tags the sub-font claims.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the font system cannot be
    /// reached, and [`KhataUnreal::KhattMarfud`] when it was reached and
    /// rejected the bytes.
    fn sajjil_farai(
        &self,
        ism: &str,
        bayt: &[u8],
        nitaqat: &[(u32, u32)],
        thaqafat: &[&str],
    ) -> Result<(), KhataUnreal> {
        let _ = (ism, bayt, nitaqat, thaqafat);
        Err(KhataUnreal::SlateGhayrMawjud {
            sabab: "this Slate binding does not reach the font system".to_owned(),
        })
    }

    /// Points a named Slate style at a composite font built from registered
    /// sub-fonts.
    ///
    /// # Errors
    ///
    /// As [`Musaddir::sajjil_farai`], plus [`KhataUnreal::KhattMarfud`] when the
    /// style name is not one this game's style set defines.
    fn atbiq_murakkab(
        &self,
        uslub: &str,
        asasi: &str,
        farai: &[&str],
    ) -> Result<(), KhataUnreal> {
        let _ = (uslub, asasi, farai);
        Err(KhataUnreal::SlateGhayrMawjud {
            sabab: "this Slate binding does not reach the font system".to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Rungs, and the record of which one fired
// ---------------------------------------------------------------------------

/// One rung of the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rutba {
    /// A write into the game's own ini files. Nothing is injected.
    Idadat,
    /// Launch options, for games that regenerate their ini.
    SatrAwamir,
    /// A live write inside the process, for games that overwrite it later.
    Haqn,
}

impl Rutba {
    /// A stable short name for logs and diagnostics bundles.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Idadat => "idadat",
            Self::SatrAwamir => "satr-awamir",
            Self::Haqn => "haqn",
        }
    }

    /// The rung's ordinal, one-based, as the ladder is documented.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Idadat => 1,
            Self::SatrAwamir => 2,
            Self::Haqn => 3,
        }
    }

    /// Whether this rung puts code inside the game's process.
    #[must_use]
    pub const fn muqtahim(self) -> bool {
        matches!(self, Self::Haqn)
    }
}

/// What one rung did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatijatRutba {
    /// Which rung.
    pub rutba: Rutba,
    /// Whether the rung's effect was confirmed by reading the value back.
    ///
    /// A rung that could not be verified is not a rung that failed: the ini
    /// route cannot be verified at all until the game next starts.
    pub muakkada: bool,
    /// One sentence naming what happened, for the log and the bundle.
    pub mulahaza: String,
}

/// The record of the whole walk.
///
/// Kept because the answer "Arabic joined correctly" is not the same answer as
/// "Arabic joined correctly because the ini took", and only the second one
/// tells the next maintainer whether the injected rung is still needed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SijillTashghil {
    /// Every rung attempted, in the order attempted.
    pub rutab: Vec<NatijatRutba>,
    /// The rung whose effect was confirmed, if any.
    pub nafidha: Option<Rutba>,
}

impl SijillTashghil {
    /// Records a rung.
    pub fn sajjil(&mut self, rutba: Rutba, muakkada: bool, mulahaza: impl Into<String>) {
        if muakkada && self.nafidha.is_none() {
            self.nafidha = Some(rutba);
        }
        self.rutab.push(NatijatRutba { rutba, muakkada, mulahaza: mulahaza.into() });
    }

    /// Whether any rung was confirmed.
    #[must_use]
    pub const fn muakkad(&self) -> bool {
        self.nafidha.is_some()
    }

    /// Whether any rung ran at all, confirmed or not.
    #[must_use]
    pub const fn juribat(&self) -> bool {
        !self.rutab.is_empty()
    }

    /// Every rung's note joined into one sentence, for an error's `sabab`.
    #[must_use]
    pub fn sabab(&self) -> String {
        if self.rutab.is_empty() {
            return "no rung was attempted".to_owned();
        }
        self.rutab
            .iter()
            .map(|natija| {
                let rutba = natija.rutba;
                format!("rung {} ({}): {}", rutba.raqm(), rutba.ism(), natija.mulahaza)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

// ---------------------------------------------------------------------------
// One ini entry
// ---------------------------------------------------------------------------

/// A single `Key=Value` Taarib writes into an ini section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalIni {
    /// The left-hand side, including any Unreal array prefix (`+`, `-`, `.`).
    pub miftah: String,
    /// The right-hand side, verbatim.
    pub qeema: String,
}

impl MadkhalIni {
    /// Builds an entry.
    #[must_use]
    pub fn jadeed(miftah: impl Into<String>, qeema: impl Into<String>) -> Self {
        Self { miftah: miftah.into(), qeema: qeema.into() }
    }

    /// The line as it is written.
    #[must_use]
    pub fn satr(&self) -> String {
        format!("{}={}", self.miftah, self.qeema)
    }

    /// Whether this entry adds to a list rather than replacing a scalar.
    ///
    /// Unreal's `+Key=` appends and each occurrence is a distinct value, so an
    /// existing `+Key=` line must never be displaced the way a scalar is —
    /// commenting one out would silently drop a value the game needs.
    #[must_use]
    pub fn jamii(&self) -> bool {
        self.miftah.starts_with(['+', '-', '.', '!'])
    }
}

// ---------------------------------------------------------------------------
// The ini file
// ---------------------------------------------------------------------------

/// An ini held as its own lines, so that writing it back changes only what
/// Taarib changed.
///
/// Comments, blank lines, ordering, the byte order mark and the line ending the
/// file already used are all preserved. A game's `Engine.ini` is frequently
/// hand-edited by its own players; rewriting it into a normalised form would
/// destroy work that is not Taarib's to destroy, and would make every undo a
/// guess.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MalafIni {
    satur: Vec<String>,
    bom: bool,
    crlf: bool,
    nihaya: bool,
}

impl MalafIni {
    /// Reads an ini, treating a missing file as an empty one.
    ///
    /// A missing `Engine.ini` is the ordinary case before a game's first run,
    /// and creating it is exactly what the engine would do.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] when the file exists and cannot be read,
    /// [`KhataUnreal::HajmMufrit`] when it is larger than [`AQSA_HAJM_INI`], and
    /// [`KhataUnreal::IdadatMarfuda`] when it is UTF-16 or is not valid UTF-8.
    /// The last is a refusal rather than a lossy decode: an ini rewritten
    /// through a replacement character is an ini with a corrupted game key in
    /// it, and the caller's next rung is a better outcome than that.
    pub fn iqra(masar: &Path) -> Result<Self, KhataUnreal> {
        let bayt = match std::fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(sabab) => {
                return Err(KhataUnreal::KhataMalaf { masar: masar.to_path_buf(), sabab });
            }
        };

        let tul = tul_u64(bayt.len());
        if tul > AQSA_HAJM_INI {
            return Err(KhataUnreal::HajmMufrit {
                haql: "ini file length",
                qeema: tul,
                saqf: AQSA_HAJM_INI,
            });
        }

        if matches!(bayt.first_chunk::<2>(), Some(&[0xFF, 0xFE] | &[0xFE, 0xFF])) {
            return Err(KhataUnreal::IdadatMarfuda {
                masar: masar.to_path_buf(),
                sabab: "the file is UTF-16 and Taarib will not re-encode a game's \
                        configuration"
                    .to_owned(),
            });
        }

        let bom = bayt.first_chunk::<3>() == Some(&[0xEF, 0xBB, 0xBF]);
        let jism = if bom { bayt.get(3..).unwrap_or(&[]) } else { bayt.as_slice() };
        let nass = core::str::from_utf8(jism).map_err(|khata| KhataUnreal::IdadatMarfuda {
            masar: masar.to_path_buf(),
            sabab: format!("not valid UTF-8 at byte {}", khata.valid_up_to()),
        })?;

        let crlf = nass.contains("\r\n");
        let nihaya = nass.ends_with('\n');
        let mut satur: Vec<String> =
            nass.split('\n').map(|satr| satr.trim_end_matches('\r').to_owned()).collect();
        if nihaya {
            let _ = satur.pop();
        }
        if satur.len() == 1 && satur.first().is_some_and(String::is_empty) {
            satur.clear();
        }

        Ok(Self { satur, bom, crlf, nihaya })
    }

    /// The lines, as they stand.
    #[must_use]
    pub fn satur(&self) -> &[String] {
        &self.satur
    }

    /// Whether Taarib's block is present.
    #[must_use]
    pub fn maalum(&self) -> bool {
        self.satur.iter().any(|satr| satr.trim() == ALAMAT_BIDAYA)
    }

    /// Removes every Taarib block in the file and restores every line they
    /// displaced.
    ///
    /// The exact inverse of every [`MalafIni::damj`] that has run: the marked
    /// blocks go, the commented-out originals lose their prefix and stay where
    /// they were, and a file that Taarib never touched is left alone. Running it
    /// twice is the same as running it once. This is what uninstall wants.
    pub fn tarajua(&mut self) {
        self.tarajua_qism(None);
    }

    /// The same undo, restricted to one section.
    ///
    /// [`MalafIni::damj`] needs this and not the whole-file form. One
    /// `Engine.ini` carries the shaping block under `[SystemSettings]` and the
    /// culture block under `[Internationalization]`, written by two different
    /// modules in two separate calls; an unscoped undo at the head of the second
    /// call would silently delete the first call's work, and the symptom would
    /// be a game whose Arabic is selected and unjoined.
    fn tarajua_qism(&mut self, qism: Option<&str>) {
        let mut natija: Vec<String> = Vec::with_capacity(self.satur.len());
        let mut dakhil_kutla = false;
        // A key before the first header belongs to the implicit root section,
        // which is never a section Taarib writes into.
        let mut dakhil_qism = qism.is_none();

        for satr in &self.satur {
            if let Some(ism) = ism_qism(satr) {
                dakhil_qism = qism.is_none_or(|matlub| ism.eq_ignore_ascii_case(matlub));
                dakhil_kutla = false;
                natija.push(satr.clone());
                continue;
            }
            if !dakhil_qism {
                natija.push(satr.clone());
                continue;
            }
            let mahdhuf = satr.trim();
            if mahdhuf == ALAMAT_BIDAYA {
                dakhil_kutla = true;
                continue;
            }
            if mahdhuf == ALAMAT_NIHAYA {
                dakhil_kutla = false;
                continue;
            }
            if dakhil_kutla {
                continue;
            }
            match satr.strip_prefix(ALAMAT_MUATTAL) {
                Some(asli) => natija.push(asli.to_owned()),
                None => natija.push(satr.clone()),
            }
        }
        self.satur = natija;
    }

    /// Writes Taarib's entries into one section, replacing any previous block
    /// **in that section** and leaving Taarib's blocks in other sections alone.
    ///
    /// Idempotent by construction: this section's previous block is undone
    /// first, so an installer that runs three times produces the file the first
    /// run produced.
    pub fn damj(&mut self, qism: &str, madakhil: &[MadkhalIni]) {
        if madakhil.is_empty() {
            return;
        }
        // Keys a sibling module already put in this section's block survive.
        // `alam` and `khatt` both write under `[Internationalization]` in two
        // separate calls, and a block that replaced rather than composed would
        // make the second call delete the first one's key.
        let mahfuza = self.kutla_qaima(qism, madakhil);
        self.tarajua_qism(Some(qism));

        let (bidayat_qism, nihayat_qism) =
            self.hudud(qism).unwrap_or_else(|| self.adif_qism(qism));

        // Displace only scalar keys, and only inside this section. An existing
        // `+Key=` line is one value of a list and commenting it out would drop
        // it; a scalar left in place would win over Taarib's line, because
        // Unreal's config reader answers a lookup with the first match.
        for fahras in bidayat_qism..nihayat_qism {
            let Some(satr) = self.satur.get(fahras) else { continue };
            let Some(miftah) = miftah_satr(satr) else { continue };
            let mudakhal = madakhil
                .iter()
                .any(|madkhal| !madkhal.jamii() && madkhal.miftah.eq_ignore_ascii_case(miftah));
            if mudakhal {
                let muattal = format!("{ALAMAT_MUATTAL}{satr}");
                if let Some(makan) = self.satur.get_mut(fahras) {
                    *makan = muattal;
                }
            }
        }

        let mawjuda: Vec<String> = self
            .satur
            .get(bidayat_qism..nihayat_qism)
            .unwrap_or(&[])
            .iter()
            .map(|satr| satr.trim().to_owned())
            .collect();

        let mut kutla: Vec<String> = Vec::with_capacity(madakhil.len() + mahfuza.len() + 2);
        kutla.push(ALAMAT_BIDAYA.to_owned());
        kutla.extend(mahfuza.iter().cloned());
        for madkhal in madakhil {
            let satr = madkhal.satr();
            // An additive key whose exact line is already there needs no second
            // copy: Unreal would load the localization path twice.
            let mukarrar = mawjuda.iter().any(|mawjud| mawjud == &satr)
                || mahfuza.iter().any(|mawjud| mawjud == &satr);
            if madkhal.jamii() && mukarrar {
                continue;
            }
            kutla.push(satr);
        }
        kutla.push(ALAMAT_NIHAYA.to_owned());

        let _ = self.satur.splice(nihayat_qism..nihayat_qism, kutla);
    }

    /// Writes the file back atomically.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::IdadatMarfuda`] naming the path and the underlying reason,
    /// which is what pushes the caller onto the next rung rather than stopping
    /// it.
    pub fn aktub(&self, masar: &Path) -> Result<(), KhataUnreal> {
        let fasil = if self.crlf { "\r\n" } else { "\n" };
        let mut nass = String::new();
        if self.bom {
            nass.push('\u{feff}');
        }
        for (fahras, satr) in self.satur.iter().enumerate() {
            if fahras > 0 {
                nass.push_str(fasil);
            }
            nass.push_str(satr);
        }
        if !self.satur.is_empty() && self.nihaya {
            nass.push_str(fasil);
        }
        masarat::kitaba_dharra_nass(masar, &nass).map_err(|khata| KhataUnreal::IdadatMarfuda {
            masar: masar.to_path_buf(),
            sabab: khata.li_sijill(),
        })
    }

    /// The lines of an existing Taarib block in one section that the incoming
    /// entries do not claim.
    ///
    /// Ownership is by key: an entry Taarib is about to write replaces the line
    /// carrying the same key, and every other line in the block belongs to a
    /// sibling module and is carried through unchanged. That is what makes
    /// `alam` and `khatt` — which write different keys into the same
    /// `[Internationalization]` block — compose instead of overwrite, while a
    /// second run of either still replaces its own lines rather than
    /// accumulating them.
    fn kutla_qaima(&self, qism: &str, madakhil: &[MadkhalIni]) -> Vec<String> {
        let Some((bidaya, nihaya)) = self.hudud(qism) else { return Vec::new() };
        let mut mahfuza = Vec::new();
        let mut dakhil = false;
        for satr in self.satur.get(bidaya..nihaya).unwrap_or(&[]) {
            let mahdhuf = satr.trim();
            if mahdhuf == ALAMAT_BIDAYA {
                dakhil = true;
                continue;
            }
            if mahdhuf == ALAMAT_NIHAYA {
                dakhil = false;
                continue;
            }
            if !dakhil {
                continue;
            }
            let Some(miftah) = miftah_satr(satr) else { continue };
            let mutalab =
                madakhil.iter().any(|madkhal| madkhal.miftah.eq_ignore_ascii_case(miftah));
            if !mutalab {
                mahfuza.push(mahdhuf.to_owned());
            }
        }
        mahfuza
    }

    /// The half-open line range of a section's body, header excluded.
    ///
    /// Section names are matched case-insensitively, as Unreal's own config
    /// reader matches them. Matching exactly would append a second
    /// `[SystemSettings]` to a file that spells it `[systemsettings]`, and the
    /// engine would read the first one.
    fn hudud(&self, qism: &str) -> Option<(usize, usize)> {
        let bidaya = self
            .satur
            .iter()
            .position(|satr| ism_qism(satr).is_some_and(|ism| ism.eq_ignore_ascii_case(qism)))?
            + 1;
        let nihaya = self
            .satur
            .iter()
            .skip(bidaya)
            .position(|satr| ism_qism(satr).is_some())
            .map_or(self.satur.len(), |izaha| bidaya + izaha);
        Some((bidaya, nihaya))
    }

    /// Appends a section header at the end and returns its empty body range.
    fn adif_qism(&mut self, qism: &str) -> (usize, usize) {
        if self.satur.last().is_some_and(|satr| !satr.trim().is_empty()) {
            self.satur.push(String::new());
        }
        self.satur.push(format!("[{qism}]"));
        self.nihaya = true;
        let nihaya = self.satur.len();
        (nihaya, nihaya)
    }
}

/// The section a line opens, if it opens one.
fn ism_qism(satr: &str) -> Option<&str> {
    let mahdhuf = satr.trim();
    mahdhuf.strip_prefix('[').and_then(|baqi| baqi.strip_suffix(']'))
}

/// The left-hand side of an assignment, ignoring comments and headers.
fn miftah_satr(satr: &str) -> Option<&str> {
    let mahdhuf = satr.trim();
    if mahdhuf.is_empty() || mahdhuf.starts_with([';', '#', '[']) {
        return None;
    }
    mahdhuf.split_once('=').map(|(yasar, _)| yasar.trim())
}

/// Writes one section's worth of Taarib entries into an ini on disk.
///
/// The one function [`crate::alam`] and [`crate::khatt`] use, so that neither of
/// them opens a file itself and neither of them can be pointed at anything but
/// an ini.
///
/// # Errors
///
/// Whatever [`MalafIni::iqra`] or [`MalafIni::aktub`] refuses.
pub fn aktub_madakhil(
    masar: &Path,
    qism: &str,
    madakhil: &[MadkhalIni],
) -> Result<(), KhataUnreal> {
    let mut malaf = MalafIni::iqra(masar)?;
    malaf.damj(qism, madakhil);
    malaf.aktub(masar)
}

/// Removes everything Taarib wrote into an ini and restores what it displaced.
///
/// # Errors
///
/// Whatever [`MalafIni::iqra`] or [`MalafIni::aktub`] refuses. A file that is
/// missing, or that carries no marker, is success with no write.
pub fn tarajua_madakhil(masar: &Path) -> Result<(), KhataUnreal> {
    let mut malaf = MalafIni::iqra(masar)?;
    if !malaf.maalum() {
        return Ok(());
    }
    malaf.tarajua();
    malaf.aktub(masar)
}

// ---------------------------------------------------------------------------
// The correction itself
// ---------------------------------------------------------------------------

/// Forcing full shaping on, down the ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tashghil {
    ini: PathBuf,
    shakl: bool,
    ittijah: bool,
}

impl Tashghil {
    /// Builds the correction against one game's `Engine.ini`.
    ///
    /// Both corrections are on: forcing shaping is the point of the module, and
    /// turning detection on costs nothing on a game whose text is Latin.
    #[must_use]
    pub fn jadeed(ini: impl Into<PathBuf>) -> Self {
        Self { ini: ini.into(), shakl: true, ittijah: true }
    }

    /// Turns the shaping correction off, for a patch that only needs detection.
    #[must_use]
    pub const fn bi_shakl(mut self, shakl: bool) -> Self {
        self.shakl = shakl;
        self
    }

    /// Turns the bidirectional detection correction off.
    #[must_use]
    pub const fn bi_ittijah(mut self, ittijah: bool) -> Self {
        self.ittijah = ittijah;
        self
    }

    /// The ini this correction writes into.
    #[must_use]
    pub fn ini(&self) -> &Path {
        &self.ini
    }

    /// The console variables this correction owns, in write order.
    #[must_use]
    pub fn mutaghayyirat(&self) -> Vec<(&'static str, &'static str)> {
        let mut kull = Vec::with_capacity(3);
        if self.shakl {
            kull.push((MUTAGHAYYIR_SHAKL, SHAKL_KAMIL));
            kull.push((MUTAGHAYYIR_SHAKL_BADIL, SHAKL_KAMIL));
        }
        if self.ittijah {
            kull.push((MUTAGHAYYIR_ITTIJAH, ITTIJAH_TILQAI));
        }
        kull
    }

    /// The same variables as ini entries.
    #[must_use]
    pub fn madakhil(&self) -> Vec<MadkhalIni> {
        self.mutaghayyirat()
            .into_iter()
            .map(|(miftah, qeema)| MadkhalIni::jadeed(miftah, qeema))
            .collect()
    }

    /// The assertions the watchdog holds for this correction.
    ///
    /// The alias key is deliberately not asserted: on an engine that does not
    /// register it, reading it back fails forever and the watchdog would report
    /// a reset on every tick of a game that is behaving perfectly.
    #[must_use]
    pub fn tawkeedat(&self) -> Vec<Tawkeed> {
        let mut kull = Vec::with_capacity(2);
        if self.shakl {
            kull.push(Tawkeed::mutaghayyir(MUTAGHAYYIR_SHAKL, SHAKL_KAMIL));
        }
        if self.ittijah {
            kull.push(Tawkeed::mutaghayyir(MUTAGHAYYIR_ITTIJAH, ITTIJAH_TILQAI));
        }
        kull
    }

    /// Rung one: write the console variables into `[SystemSettings]`.
    ///
    /// # Errors
    ///
    /// Whatever [`aktub_madakhil`] refuses, unchanged, so the caller can name
    /// the path in its own message.
    pub fn rutbat_idadat(&self) -> Result<(), KhataUnreal> {
        aktub_madakhil(&self.ini, QISM_NIZAM, &self.madakhil())
    }

    /// Rung two: the launch options that carry the same settings.
    ///
    /// Two forms, because games disagree about which they honour: an `-ini:`
    /// override per key, which the config system applies as if the line were in
    /// the file, and one aggregate `-dpcvars=`, which the console manager
    /// applies during start.
    ///
    /// `-execcmds=` is deliberately absent. It runs after the engine is up, by
    /// which time the first font measure has already been made with kerning
    /// only and its result is in Slate's measure cache — so the menu the player
    /// is looking at stays unjoined until something invalidates it.
    #[must_use]
    pub fn rutbat_satr(&self) -> Vec<String> {
        let mutaghayyirat = self.mutaghayyirat();
        if mutaghayyirat.is_empty() {
            return Vec::new();
        }
        let mut khiyarat: Vec<String> = mutaghayyirat
            .iter()
            .map(|(miftah, qeema)| format!("-ini:Engine:[{QISM_NIZAM}]:{miftah}={qeema}"))
            .collect();
        let majmua = mutaghayyirat
            .iter()
            .map(|(miftah, qeema)| format!("{miftah}={qeema}"))
            .collect::<Vec<_>>()
            .join(",");
        khiyarat.push(format!("-dpcvars=\"{majmua}\""));
        khiyarat
    }

    /// Rung three: write the console variables in the live process.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the binding has no handle on the
    /// engine, and otherwise whatever [`Musaddir::daa`] refuses.
    pub fn rutbat_haqn(&self, musaddir: &dyn Musaddir) -> Result<(), KhataUnreal> {
        if !musaddir.muhayya() {
            return Err(KhataUnreal::SlateGhayrMawjud {
                sabab: "the Slate binding reports no handle on this process yet".to_owned(),
            });
        }
        for (miftah, qeema) in self.mutaghayyirat() {
            // The alias is best-effort: an engine that does not register it
            // refuses the write, and that refusal says nothing about whether
            // shaping is on.
            let natija = musaddir.daa(miftah, qeema);
            if miftah == MUTAGHAYYIR_SHAKL_BADIL {
                if let Err(khata) = natija {
                    tracing::debug!(
                        miftah,
                        sabab = %khata,
                        "the short shaping-method name is not registered in this build"
                    );
                }
                continue;
            }
            natija?;
        }
        Ok(())
    }

    /// Reads the corrections back out of the live process.
    ///
    /// # Errors
    ///
    /// Whatever [`Musaddir::iqra`] refuses.
    pub fn tahaqquq(&self, musaddir: &dyn Musaddir) -> Result<bool, KhataUnreal> {
        for tawkeed in self.tawkeedat() {
            if !tawkeed.muhaqqaq(musaddir)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Walks the ladder and reports which rung actually took.
    ///
    /// `musaddir` is [`None`] for the offline installer, which has no process to
    /// look at: it writes the ini, records rung one as unverified, and returns.
    /// Inside the game it is [`Some`], the ini is still written first — so the
    /// next launch needs no injection at all — and the read-back decides whether
    /// rungs two and three are needed.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::TashghilFashil`] naming every route that was tried, and
    /// only when no rung took and no rung is even pending. This is the one
    /// failure in the module that is worth raising, because Arabic drawn without
    /// it is unjoined.
    pub fn shaghghil(
        &self,
        musaddir: Option<&dyn Musaddir>,
    ) -> Result<SijillTashghil, KhataUnreal> {
        let mut sijill = SijillTashghil::default();

        let idadat_najahat = match self.rutbat_idadat() {
            Ok(()) => {
                sijill.sajjil(
                    Rutba::Idadat,
                    false,
                    format!("written into [{QISM_NIZAM}]; applies at the next launch"),
                );
                true
            }
            Err(khata) => {
                sijill.sajjil(Rutba::Idadat, false, khata.to_string());
                false
            }
        };

        let khiyarat = self.rutbat_satr();
        if khiyarat.is_empty() {
            sijill.sajjil(Rutba::SatrAwamir, false, "nothing to force".to_owned());
        } else {
            sijill.sajjil(
                Rutba::SatrAwamir,
                false,
                format!("{} launch options offered to the installer", khiyarat.len()),
            );
        }

        let Some(musaddir) = musaddir else {
            if idadat_najahat {
                return Ok(sijill);
            }
            return Err(KhataUnreal::TashghilFashil { sabab: sijill.sabab() });
        };

        // Verify before injecting. A game whose ini already took needs nothing
        // written into its memory, and not writing is the whole point of the
        // ladder.
        match self.tahaqquq(musaddir) {
            Ok(true) => {
                sijill.rutab.clear();
                sijill.sajjil(
                    Rutba::Idadat,
                    true,
                    "the process already reports full shaping; nothing was injected".to_owned(),
                );
                return Ok(sijill);
            }
            Ok(false) => {}
            Err(khata) => {
                tracing::debug!(sabab = %khata, "the shaping state could not be read back");
            }
        }

        match self.rutbat_haqn(musaddir) {
            Ok(()) => {
                let muakkada = self.tahaqquq(musaddir).unwrap_or(false);
                sijill.sajjil(
                    Rutba::Haqn,
                    muakkada,
                    if muakkada {
                        "set in the live process and confirmed by read-back".to_owned()
                    } else {
                        "set in the live process but the read-back did not confirm it".to_owned()
                    },
                );
            }
            Err(khata) => sijill.sajjil(Rutba::Haqn, false, khata.to_string()),
        }

        if sijill.muakkad() || idadat_najahat {
            Ok(sijill)
        } else {
            Err(KhataUnreal::TashghilFashil { sabab: sijill.sabab() })
        }
    }

    /// Removes everything Taarib wrote into this ini.
    ///
    /// Every Taarib block in the file, not only this module's: one ini carries
    /// the shaping keys and the culture keys, uninstall wants both gone, and a
    /// per-module undo would leave whichever module the caller forgot.
    ///
    /// # Errors
    ///
    /// Whatever [`tarajua_madakhil`] refuses.
    pub fn tarajua(&self) -> Result<(), KhataUnreal> {
        tarajua_madakhil(&self.ini)
    }
}

// ---------------------------------------------------------------------------
// The re-assert watchdog
// ---------------------------------------------------------------------------

/// One thing that must still be true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tawkeed {
    /// A console variable that must still hold a value.
    Mutaghayyir {
        /// The variable's name.
        miftah: String,
        /// The value it must report.
        qeema: String,
    },
    /// The culture that must still be active. Asserted for [`crate::alam`].
    Thaqafa {
        /// The culture tag.
        wasm: String,
    },
}

impl Tawkeed {
    /// A console-variable assertion.
    #[must_use]
    pub fn mutaghayyir(miftah: impl Into<String>, qeema: impl Into<String>) -> Self {
        Self::Mutaghayyir { miftah: miftah.into(), qeema: qeema.into() }
    }

    /// A culture assertion.
    #[must_use]
    pub fn thaqafa(wasm: impl Into<String>) -> Self {
        Self::Thaqafa { wasm: wasm.into() }
    }

    /// A stable name for logs.
    #[must_use]
    pub fn ism(&self) -> &str {
        match self {
            Self::Mutaghayyir { miftah, .. } => miftah,
            Self::Thaqafa { .. } => "culture",
        }
    }

    /// Whether the process still reports what this assertion requires.
    ///
    /// # Errors
    ///
    /// Whatever [`Musaddir::iqra`] or [`Musaddir::thaqafa_haliya`] refuses.
    pub fn muhaqqaq(&self, musaddir: &dyn Musaddir) -> Result<bool, KhataUnreal> {
        match self {
            Self::Mutaghayyir { miftah, qeema } => {
                Ok(musaddir.iqra(miftah)?.trim() == qeema.as_str())
            }
            Self::Thaqafa { wasm } => {
                // `ar-SA` satisfies an assertion of `ar`: Unreal's fallback
                // chain reaches the patch's resources from either, and a game
                // that answered `ar-SA` to a request for `ar` has done what was
                // asked. Re-asserting on that would fight the engine.
                let hali = musaddir.thaqafa_haliya()?;
                let asas = format!("{}-", wasm.to_ascii_lowercase());
                Ok(hali.eq_ignore_ascii_case(wasm)
                    || hali.to_ascii_lowercase().starts_with(&asas))
            }
        }
    }

    /// Re-applies this assertion.
    ///
    /// # Errors
    ///
    /// Whatever [`Musaddir::daa`] or [`Musaddir::thabbit_thaqafa`] refuses.
    pub fn akkid(&self, musaddir: &dyn Musaddir) -> Result<(), KhataUnreal> {
        match self {
            Self::Mutaghayyir { miftah, qeema } => musaddir.daa(miftah, qeema),
            Self::Thaqafa { wasm } => musaddir.thabbit_thaqafa(wasm),
        }
    }
}

/// What one tick of the watchdog found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NabdHaris {
    /// Everything still held. Nothing was written.
    Salim,
    /// Something had been reset and was re-asserted.
    Uid,
    /// The bound is spent; the watchdog has stopped.
    Intaha,
}

/// The bounded re-assert watchdog.
///
/// It exists because a game that applies its own saved configuration after
/// engine start silently undoes a set-at-launch, and the symptom is Arabic that
/// joins on the loading screen and stops joining in the main menu — which reads
/// as a font bug and costs a maintainer a day.
///
/// It is bounded because the alternative is worse. See [`HADD_NABADAT`],
/// [`TAMDEED_NABADAT`] and [`HADD_MUTLAQ_NABADAT`] for the numbers and the
/// reasoning behind each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarisTashghil {
    tawkeedat: Vec<Tawkeed>,
    mutabaqqi: u32,
    masruf: u32,
    fasil: Duration,
}

impl HarisTashghil {
    /// Builds a watchdog over a set of assertions.
    #[must_use]
    pub const fn jadeed(tawkeedat: Vec<Tawkeed>) -> Self {
        Self {
            tawkeedat,
            mutabaqqi: HADD_NABADAT,
            masruf: 0,
            fasil: FASIL_HARIS,
        }
    }

    /// Adds an assertion — how [`crate::alam`] joins this watchdog instead of
    /// starting a second one. Two polling threads inside one game is one thread
    /// too many.
    pub fn adif(&mut self, tawkeed: Tawkeed) {
        self.tawkeedat.push(tawkeed);
    }

    /// Overrides the interval between checks.
    #[must_use]
    pub const fn bi_fasil(mut self, fasil: Duration) -> Self {
        self.fasil = fasil;
        self
    }

    /// How many checks remain.
    #[must_use]
    pub const fn mutabaqqi(&self) -> u32 {
        self.mutabaqqi
    }

    /// How many checks have been spent.
    #[must_use]
    pub const fn masruf(&self) -> u32 {
        self.masruf
    }

    /// The assertions being held.
    #[must_use]
    pub fn tawkeedat(&self) -> &[Tawkeed] {
        &self.tawkeedat
    }

    /// One check.
    ///
    /// Reads every assertion back, re-applies the ones that no longer hold, and
    /// logs each re-assert with the name of what was reset — because the fact
    /// that a game resets this variable is the single most useful thing in that
    /// game's diagnostics bundle.
    ///
    /// Nothing here is fallible to the caller. A watchdog that propagated a read
    /// failure would turn a game whose Slate binding went away — because a level
    /// transition tore down a subsystem, say — into a reported error every five
    /// seconds. Failures are logged and the tick is spent.
    pub fn nabd(&mut self, musaddir: &dyn Musaddir) -> NabdHaris {
        if self.mutabaqqi == 0 || self.tawkeedat.is_empty() {
            return NabdHaris::Intaha;
        }
        self.mutabaqqi = self.mutabaqqi.saturating_sub(1);
        self.masruf = self.masruf.saturating_add(1);

        let mut uid = false;
        for tawkeed in &self.tawkeedat {
            match tawkeed.muhaqqaq(musaddir) {
                Ok(true) => {}
                Ok(false) => {
                    match tawkeed.akkid(musaddir) {
                        Ok(()) => {
                            uid = true;
                            tracing::warn!(
                                tawkeed = tawkeed.ism(),
                                nabda = self.masruf,
                                "the game reset a Taarib setting; it has been re-asserted"
                            );
                        }
                        Err(khata) => tracing::warn!(
                            tawkeed = tawkeed.ism(),
                            sabab = %khata,
                            "the game reset a Taarib setting and it could not be re-asserted"
                        ),
                    }
                }
                Err(khata) => tracing::debug!(
                    tawkeed = tawkeed.ism(),
                    sabab = %khata,
                    "a Taarib setting could not be read back on this tick"
                ),
            }
        }

        if uid {
            let saqf = HADD_MUTLAQ_NABADAT.saturating_sub(self.masruf);
            self.mutabaqqi = self.mutabaqqi.saturating_add(TAMDEED_NABADAT).min(saqf);
            NabdHaris::Uid
        } else {
            NabdHaris::Salim
        }
    }

    /// Runs the watchdog to exhaustion on the calling thread, sleeping between
    /// checks, and returns how many re-asserts fired.
    ///
    /// Meant to be given its own thread by the module's entry point. It ends on
    /// its own; nothing has to remember to stop it.
    pub fn raqib(&mut self, musaddir: &dyn Musaddir) -> u32 {
        let mut marrat = 0_u32;
        loop {
            std::thread::sleep(self.fasil);
            match self.nabd(musaddir) {
                NabdHaris::Intaha => break,
                NabdHaris::Uid => marrat = marrat.saturating_add(1),
                NabdHaris::Salim => {}
            }
        }
        tracing::info!(
            marrat,
            nabadat = self.masruf,
            "the shaping watchdog has finished its bounded run"
        );
        marrat
    }
}
