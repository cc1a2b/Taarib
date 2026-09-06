//! The script-engine write, dispatched once, from inside the install pipeline.
//!
//! Four of the engines Taarib supports are patched as *data*: RPG Maker MV and
//! MZ through `data/*.json`, Ren'Py through generated `game/tl/arabic/*.rpy`,
//! GameMaker through the string pool of `data.win`, Electron through a payload
//! placed inside `app.asar`. `taarib-muhawwil-nusus` has held the readers and
//! the writers for all four since Phase 10 — and nothing in this crate ever
//! called the writers. A package for one of those games installed a plugin, a
//! Python module and a `.ruqaa` beside a game whose own files still held the
//! developer's English.
//!
//! This module is the missing caller, and it is deliberately thin. It knows
//! four things the engine crate does not:
//!
//! 0. **Whether this game may be written to at all.** [`IdhnNusus`] is the
//!    answer, and it is an argument rather than something worked out here. The
//!    write used to run unconditionally, from
//!    [`crate::masar_tathbeet::thabbit`], re-deriving the engine from the
//!    filesystem — so a tier-3 game, whose report had just told the player
//!    «the game is not modified at all», had its own shipped text replaced,
//!    and so did a game the safety layer had refused outright. Neither of those
//!    facts is visible in a game directory, which is exactly why they have to
//!    arrive as a value.
//!
//! 1. **Where the Arabic is.** A `.ruqaa` keys its string table on the first
//!    eight bytes of BLAKE3 over the *source* text, and stores only the
//!    translation. [`MutarjimRuqaa`] turns that into the one question a patcher
//!    asks — "what is the Arabic for this string?" — so the engine crate never
//!    learns the container's shape.
//! 2. **How a game file may be touched.** The engine crate writes through its
//!    own `Hafiz` guard; this crate writes through [`Muthabbit`]. The two are the
//!    same contract stated twice, and [`HafizMuthabbit`] is the adapter, written
//!    so that the *real* refusal survives the crossing rather than being
//!    flattened into the other crate's vocabulary and back.
//! 3. **Where the build artifacts are.** The Electron adapter embeds a compiled
//!    renderer runtime that no Rust code produces. It is staged into the
//!    component store as `mulhaq/electron/taarib.js`, and this module is the
//!    only thing that reads it, because the component store's layout is this
//!    crate's business and not the engine crate's. The Ren'Py adapter needs a
//!    font *name* for the same reason: the face is a component the installer
//!    deploys, and only this crate knows where a component lives or what it is
//!    called. That name arrives in the permit, chosen by
//!    [`crate::tarkib::khutta`] out of the very listing the deployment will
//!    place — so the file the generated `.rpy` names and the file that lands in
//!    `game/` cannot be two different files, by construction rather than by two
//!    functions agreeing.
//!
//! ## Why it runs where it runs
//!
//! [`crate::tarkib::nashr_bi_khutta`] calls it first, before the framework
//! deployment step and before the additive layer. That ordering is not
//! incidental. RPG Maker's extraction records carry byte offsets into
//! `js/plugins.js`, and deploying the adapter appends Taarib's registration to
//! that file; splicing against offsets measured before the append would be
//! splicing against a file whose length has moved. The engine crate documents
//! the same ordering for the same reason, and honouring it here is what keeps
//! the two halves of an RPG Maker install from tripping over each other.
//!
//! It runs from *inside* the deployment step rather than beside it because that
//! is the step holding the plan. A write that ran beside it, as this one used
//! to, is a write with no plan to consult — and every question this module must
//! not answer for itself lives in that plan.
//!
//! The ordering is also why the Ren'Py font arrives as a name and not as a
//! file: at the moment this runs, nothing has been deployed and the face is
//! still in the component store. The name comes from the plan, which chose it
//! from the very listing the deployment will place a step later — so what the
//! generated `.rpy` points at and what lands in `game/` cannot be two different
//! files.
//!
//! ## What it does not do
//!
//! It does not *decide* whether a game may be patched — it is handed the
//! decision — and it does not deploy the runtime adapters, which is `tarkib`'s
//! table. A game on none of the four engines gets [`None`] and no writes, which
//! is the answer for every Unity, Unreal and Godot game the installer will ever
//! see.

use std::io;
use std::path::{Path, PathBuf};

use taarib_muhawwil_nusus::khata::KhataNusus;
use taarib_muhawwil_nusus::tarkeeb::{Mawarid, Mutarjim, TaqreerTarkeeb};
use taarib_muhawwil_nusus::{Hafiz, tarkeeb};
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_ruqaa::aqsam::NawQism;
use taarib_ruqaa::qari::{self, JadwalNusus, MalafRuqaa};
use taarib_usus::khata::Tafsir as _;
use taarib_usus::masarat::Masarat;

use crate::bayan::Muthabbit;
use crate::khata::{KhataTathbeet, NatijatTathbeet};
use crate::tarkib::{KhuttatTarkib, QararTabaqa};

/// The component store path of the compiled Electron renderer runtime.
///
/// Written by `esbuild` over `adapters-script/electron/taarib.ts` and staged by
/// `taarib-tajmee` under row I1. Named here because this is the only place that
/// reads it, and spelled `/`-separated because that is how every other component
/// name in this crate is spelled.
pub const MUKAWWIN_TASHGHIL_GHILAF: &str = "mulhaq/electron/taarib.js";

/// The largest adapter runtime this build will embed into a game's archive.
///
/// Eight mebibytes. The Electron runtime is one bundled JavaScript file and a
/// real one is under a megabyte; the ceiling exists because the bytes are read
/// into memory and then written into an archive, and a component store somebody
/// has put something else into should be a refusal rather than an exhausted
/// machine.
pub const AQSA_TASHGHIL: u64 = 8 * 1024 * 1024;

/// What authorises one script-engine write, and configures it.
///
/// Two forms, and the difference between them is whether the caller holds a
/// deployment plan. An installer that deploys does — and hands the plan, so the
/// tier and the Ren'Py face are the plan's answers rather than second opinions.
/// An installer that deploys nothing, such as `taarib-tilqai`'s automatic
/// pipeline, has no component store to plan against and holds only the
/// decision; it registers no face, because a name registered for a file no step
/// will place is a game pointed at a font that is not there.
///
/// Both forms carry a [`QararTabaqa`], and that is the point: there is no third
/// form, and no way to reach [`raqqi_nusus`] without one.
#[derive(Debug, Clone, Copy)]
pub struct IdhnNusus<'a> {
    qarar: QararTabaqa,
    khatt_renpy: Option<&'a str>,
}

impl<'a> IdhnNusus<'a> {
    /// The permit a deployment plan carries.
    #[must_use]
    pub fn min_khutta(mukhattat: &'a KhuttatTarkib) -> Self {
        Self { qarar: mukhattat.qarar(), khatt_renpy: mukhattat.khatt_renpy.as_deref() }
    }

    /// The permit an install that deploys nothing carries.
    #[must_use]
    pub const fn min_qarar(qarar: QararTabaqa) -> Self {
        Self { qarar, khatt_renpy: None }
    }

    /// The decision itself.
    #[must_use]
    pub const fn qarar(&self) -> QararTabaqa {
        self.qarar
    }
}

/// The handle one deployment step writes through.
///
/// It holds the recorder every write already went through and the package whose
/// translations the script-engine write places — so that write happens *inside*
/// the step that holds the plan, under the plan's tier, instead of
/// unconditionally beside it.
///
/// [`crate::masar_tathbeet::thabbit`] builds one, hands it to the deployment
/// step, and reads back what the script-engine write did. A step that never
/// calls [`Nashir::raqqi`] has declared no tier decision, and `thabbit` then
/// places no package content either: an install that never established what it
/// is allowed to do to a game writes nothing into it.
pub struct Nashir<'a> {
    muthabbit: &'a mut dyn Muthabbit,
    ruqaa: &'a MalafRuqaa,
    jidhr_luba: &'a Path,
    qarar: Option<QararTabaqa>,
    nusus: Option<TaqreerTarkeeb>,
}

// Written out rather than derived: a recorder and a mapped package are not
// values anybody wants printed, and what a reader of a log line needs is which
// game this is about and whether the text write has run.
impl std::fmt::Debug for Nashir<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Nashir")
            .field("jidhr_luba", &self.jidhr_luba)
            .field("qarar", &self.qarar)
            .finish_non_exhaustive()
    }
}

impl<'a> Nashir<'a> {
    /// Wraps a recorder and a package for the duration of one deployment.
    pub fn jadeed(
        muthabbit: &'a mut dyn Muthabbit,
        ruqaa: &'a MalafRuqaa,
        jidhr_luba: &'a Path,
    ) -> Self {
        Self { muthabbit, ruqaa, jidhr_luba, qarar: None, nusus: None }
    }

    /// The recorder, for the deployment's own writes.
    pub fn muthabbit(&mut self) -> &mut dyn Muthabbit {
        self.muthabbit
    }

    /// Writes the package's translations into the game's own engine data, under
    /// the permit it is given.
    ///
    /// Runs at most once. A second call is a no-op rather than a second pass:
    /// the first already placed everything the package carries, and re-running
    /// an RPG Maker splice against a file whose length has moved is the failure
    /// this module's ordering exists to avoid.
    ///
    /// # Errors
    ///
    /// Whatever [`raqqi_nusus`] raises.
    pub fn raqqi(
        &mut self,
        idhn: IdhnNusus<'_>,
        mukawwinat: Option<&Path>,
    ) -> NatijatTathbeet<()> {
        if self.qarar.is_some() {
            return Ok(());
        }
        self.qarar = Some(idhn.qarar());
        self.nusus =
            raqqi_nusus(idhn, self.jidhr_luba, self.ruqaa, mukawwinat, self.muthabbit)?;
        Ok(())
    }

    /// The decision the deployment step declared, when it declared one.
    #[must_use]
    pub const fn qarar(&self) -> Option<QararTabaqa> {
        self.qarar
    }

    /// What the script-engine write did, consuming the handle.
    #[must_use]
    pub fn nusus(self) -> Option<TaqreerTarkeeb> {
        self.nusus
    }
}

/// The string table of one package, as a source-string lookup.
///
/// The container stores a 64-bit BLAKE3 prefix of each source string and the
/// Arabic it maps to — never the source itself, which is why this is a hash and
/// a binary search rather than a map. [`qari::miftah_min_nass`] is the only
/// definition of that key in the product and is used here rather than restated.
#[derive(Debug)]
pub struct MutarjimRuqaa<'a> {
    jadwal: JadwalNusus<'a>,
}

impl<'a> MutarjimRuqaa<'a> {
    /// Wraps an already-read string table.
    #[must_use]
    pub const fn jadeed(jadwal: JadwalNusus<'a>) -> Self {
        Self { jadwal }
    }

    /// How many strings the patch carries.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.jadwal.adad()
    }
}

impl Mutarjim for MutarjimRuqaa<'_> {
    fn tarjim(&self, asl: &str) -> Option<&str> {
        self.jadwal.tarjama(qari::miftah_min_nass(asl))
    }
}

/// The engine crate's write guard, backed by this crate's recorder.
///
/// Both crates enforce the same rule — nothing is written into a game until its
/// original is preserved and the manifest line naming that backup is on the
/// device — and each states it with its own trait. This is the adapter, and the
/// only interesting thing about it is [`HafizMuthabbit::khata`].
///
/// A refusal raised by [`Muthabbit`] is a [`KhataTathbeet`], carrying a
/// permanent error code, the path, and the remedy the person in front of the
/// screen needs. Crossing into [`Hafiz`] means expressing it as a
/// [`KhataNusus`], which is a different vocabulary with different codes. Rather
/// than lose the original and hand a user a code that names the wrong crate, the
/// real refusal is kept here and put back by [`raqqi_nusus`] when the call
/// unwinds. The [`KhataNusus`] that travels through the engine crate exists only
/// so the adapters' `?` operators have something to carry.
pub struct HafizMuthabbit<'a> {
    muthabbit: &'a mut dyn Muthabbit,
    khata: Option<KhataTathbeet>,
}

// Written out rather than derived, because `Muthabbit` is a trait object and a
// recorder is not a value anybody wants printed. What a reader of a log line
// needs from this type is whether a refusal is being carried, which is the one
// thing it holds that is its own.
impl std::fmt::Debug for HafizMuthabbit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HafizMuthabbit").field("khata", &self.khata).finish_non_exhaustive()
    }
}

impl<'a> HafizMuthabbit<'a> {
    /// Wraps a recorder for the duration of one script-engine write.
    pub fn jadeed(muthabbit: &'a mut dyn Muthabbit) -> Self {
        Self { muthabbit, khata: None }
    }

    /// The real refusal, when one was raised while crossing.
    #[must_use]
    pub const fn khata(&mut self) -> Option<KhataTathbeet> {
        self.khata.take()
    }

    /// Records a refusal and restates it in the engine crate's vocabulary.
    fn abur<T>(&mut self, natija: NatijatTathbeet<T>, masar: &Path) -> Result<T, KhataNusus> {
        natija.map_err(|khata| {
            let sabab = khata.injilizi();
            self.khata = Some(khata);
            KhataNusus::KhataMalaf { masar: masar.to_path_buf(), sabab: io::Error::other(sabab) }
        })
    }
}

impl Hafiz for HafizMuthabbit<'_> {
    fn iktub(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus> {
        let natija = self.muthabbit.iktub(masar, bayt);
        self.abur(natija, masar)
    }

    fn ansha(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus> {
        let natija = self.muthabbit.ansha(masar, bayt);
        self.abur(natija, masar)
    }

    fn ansha_mujallad(&mut self, masar: &Path) -> Result<(), KhataNusus> {
        let natija = self.muthabbit.ansha_mujallad(masar);
        self.abur(natija, masar)
    }

    fn ihdhif(&mut self, masar: &Path) -> Result<bool, KhataNusus> {
        let natija = self.muthabbit.ihdhif(masar);
        self.abur(natija, masar)
    }

    fn ihdhif_mujallad(&mut self, masar: &Path) -> Result<bool, KhataNusus> {
        // The recorder has no directory removal: an install never removes one,
        // and the restore path owns the uninstall. Reporting "there was no
        // directory of mine to remove" is the truthful answer for an install and
        // is what every additive adapter here does with the result anyway.
        let _ = masar;
        Ok(false)
    }
}

/// Writes a package's translations into a game's own engine data, under a
/// permit.
///
/// Returns [`None`] when the permit forbids modifying this game at all, when the
/// game is not on one of the four script engines, or when the package carries no
/// string table — a font-only patch is a real thing and is not a failure.
///
/// The permit is checked before the engine is identified, and deliberately: the
/// engine is read off the game's own directory, and a tier and a safety refusal
/// are not things a directory can be asked about. Checking the other way round
/// is what this defect *was*.
///
/// `mukawwinat` is the component store root. It is optional because an
/// installer that cannot resolve the store should still patch the engines that
/// need nothing from it, and because the two that do read it fail differently:
/// [`None`] makes the Electron adapter decline with a message naming the
/// component, rather than writing a translation table into somebody's
/// `app.asar` with no runtime to read it, while for Ren'Py it means no font is
/// registered — a translated, correctly-shaped game rendered in whatever face
/// it already ships, which is the difference between a Ren'Py build whose GUI
/// font covers Arabic and one whose font does not.
///
/// # Errors
///
/// [`KhataTathbeet::NususMarfuda`] when the package's string table cannot be
/// read or the adapter refused; whatever the recorder raises when an original
/// cannot be preserved or a replacement cannot be written, with its own code and
/// path intact.
pub fn raqqi_nusus(
    idhn: IdhnNusus<'_>,
    jidhr_luba: &Path,
    ruqaa: &MalafRuqaa,
    mukawwinat: Option<&Path>,
    muthabbit: &mut dyn Muthabbit,
) -> NatijatTathbeet<Option<TaqreerTarkeeb>> {
    // Before the engine, before the package, before anything is opened. A tier-3
    // game is one whose own text is never replaced — the Arabic is drawn over it
    // from a second process — and this module writes nothing but replacements of
    // a game's own text.
    if !idhn.qarar().tughayyar_al_luba() {
        return Ok(None);
    }

    // Cheapest question next: most games are Unity, and identifying one costs
    // four metadata queries and opens nothing. Reading the package's string
    // table before knowing whether anything will use it would decompress a
    // section for every install of every engine. The family is kept rather than
    // the target because the two are one to one in `ayn_hadaf` and the family
    // is the half this crate can name without depending on the probe.
    let Some((_, aila)) = tarkeeb::ayn_hadaf(jidhr_luba) else {
        return Ok(None);
    };

    let mafateeh = ruqaa.ruqaa().map_err(|khata| marfud(jidhr_luba, &khata.to_string()))?;
    if !mafateeh.yahwi(NawQism::Nusus) {
        return Ok(None);
    }
    let qism = mafateeh
        .qism(NawQism::Nusus)
        .map_err(|khata| marfud(jidhr_luba, &khata.to_string()))?;
    // The section's own bytes, never the whole file: every table reader here
    // parses a preamble at the start of what it is handed, and handed the
    // package it would read the file header as one.
    let jadwal = qari::nusus(qism.bayt()).map_err(|khata| marfud(jidhr_luba, &khata.to_string()))?;
    if jadwal.khali() {
        return Ok(None);
    }

    let tashghil = match mukawwinat.and_then(iqra_tashghil) {
        Some(natija) => Some(natija?),
        None => None,
    };
    // The face is placed by the component store's own deployment step and named
    // by the plan that step executes, so the name arrives in the permit rather
    // than being read out of the store a second time here. The engine is still
    // checked, because a permit built for a Ren'Py game is the only one that
    // carries a face and an adapter for another engine must not be handed one.
    let khatt = match aila {
        AilatMuharrik::Renpy => idhn.khatt_renpy,
        _ => None,
    };
    let mawarid = Mawarid { tashghil_ghilaf: tashghil.as_deref(), khatt_renpy: khatt };

    let mutarjim = MutarjimRuqaa::jadeed(jadwal);
    let mut hafiz = HafizMuthabbit::jadeed(muthabbit);
    match tarkeeb::rakkib_luba(jidhr_luba, &mutarjim, &mawarid, &mut hafiz) {
        Ok(taqreer) => Ok(taqreer),
        // The recorder's own refusal outranks the restatement of it: it carries
        // the permanent code, the path and the remedy.
        Err(khata) => Err(hafiz.khata().unwrap_or_else(|| marfud(jidhr_luba, &khata.injilizi()))),
    }
}

/// Reads the compiled Electron renderer runtime out of the component store.
///
/// [`None`] when the store does not carry it, which is a component that was
/// never built rather than a failure to read one: `esbuild` produces it from
/// `adapters-script/electron/taarib.ts` and a bundle staged without that step
/// simply has no Electron support. Saying so through the adapter's own refusal
/// is more use than a missing-file error naming a path nobody recognises.
///
/// # Errors
///
/// [`KhataTathbeet::HajmMufrit`] when the file is above [`AQSA_TASHGHIL`],
/// checked against the size the filesystem declares before a byte is read, and
/// [`KhataTathbeet::KhataMalaf`] when it is there and cannot be read or is not
/// UTF-8 — a runtime that is not text is not the file this names.
fn iqra_tashghil(jidhr_makhzan: &Path) -> Option<NatijatTathbeet<String>> {
    let mut masar = jidhr_makhzan.to_path_buf();
    for juz in MUKAWWIN_TASHGHIL_GHILAF.split('/') {
        masar.push(juz);
    }
    let bayanat = std::fs::metadata(&masar).ok()?;
    if !bayanat.is_file() {
        return None;
    }
    if bayanat.len() > AQSA_TASHGHIL {
        return Some(Err(KhataTathbeet::HajmMufrit {
            haql: "the Electron renderer runtime",
            qeema: bayanat.len(),
            saqf: AQSA_TASHGHIL,
        }));
    }
    Some(std::fs::read_to_string(&masar).map_err(|sabab| KhataTathbeet::KhataMalaf {
        masar,
        amal: "reading the compiled Electron renderer runtime from the component store",
        sabab,
    }))
}

/// The component store this machine resolves to, when it resolves to one.
///
/// [`crate::masar_tathbeet::thabbit`] is handed a game, a package and a backup
/// directory and nothing about where Taarib keeps its own files, so the one
/// adapter that needs a build artifact would otherwise have no way to reach it
/// without a parameter every caller of the install pipeline would have to
/// thread through. [`Masarat`] is the product's single answer to "where are
/// Taarib's directories", overrides and portable marker included, and asking it
/// here is asking the same question `tarkib`'s callers already ask rather than
/// forming a second opinion about the answer.
///
/// [`None`] on a machine with no home directory to derive from — a service
/// account, a stripped container — where the three adapters that need nothing
/// from the store still run.
#[must_use]
pub fn makhzan_mukawwinat() -> Option<PathBuf> {
    Masarat::iktashif().ok().map(|masarat| masarat.mukawwinat())
}

/// The refusal this module raises when the write could not be attempted.
fn marfud(jidhr: &Path, sabab: &str) -> KhataTathbeet {
    KhataTathbeet::NususMarfuda { masar: jidhr.to_path_buf(), sabab: sabab.to_owned() }
}
