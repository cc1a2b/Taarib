//! البداية — what `taarib-mudkhal` calls once this payload is inside the game.
//!
//! `docs/bidaya.md` is the contract; this file is the Unreal payload's half of
//! it. The loader maps this module beside the game's executable, resolves
//! `taarib_bidaya`, and calls it once, from its own thread, before the game has
//! finished starting. Everything the runtime half of this adapter does begins
//! here, and the failure this file exists to prevent is the one the contract
//! names: a module that loads, reports success, and leaves the game in English.
//!
//! ## The order, and what each step means here
//!
//! 1. **Where am I.** [`taarib_haqn::mawqi::mujallad_nafsi`], resolved from an
//!    address inside this module. Never `current_exe` for this question: the
//!    executable belongs to the game, and the working directory belongs to
//!    whichever launcher started it.
//! 2. **What am I inside.** Unreal links the whole engine into the game's own
//!    executable, so the module this payload exists to take over *is* the
//!    executable — a module that is never absent. So the question "is this that
//!    kind of game" cannot be answered by a handle being null, and is answered
//!    instead by the identification this crate already owns: [`afhas`] over the
//!    game root, which reads the container footers and the build layout. The
//!    executable's own module handle is still asked for through
//!    [`taarib_haqn::mawqi::qaidat_wahda`] and recorded, because on Windows it
//!    is the authoritative answer to "which image am I loaded beside" and
//!    because an address in the log is what makes a crash report legible. It is
//!    not by itself grounds to decline: on Unix a by-name module lookup is a
//!    `dlopen` with `RTLD_NOLOAD`, which answers for shared objects and not for
//!    the program image, and declining every Linux Unreal game on the strength
//!    of that would be the exact defect this contract closes.
//! 3. **The patch.** `<own dir>/*.ruqaa`, opened through
//!    [`taarib_ruqaa::qari::MalafRuqaa`], which maps it and validates its
//!    framing and content hash before a byte of the body is read. No patch is a
//!    logged decline: nothing to apply, and the game is left alone.
//! 4. **The disclosure** is tier 3's, and this is not tier 3. `Iqrar::baad_ard`
//!    and `sidq::mahfuz_salih` govern the overlay, which draws over a game it
//!    did not write; an engine adapter that switches Slate's own shaping method
//!    on displays nothing of its own and discloses nothing. This step is
//!    skipped by construction rather than routed around, and there is no code
//!    here that could route around it.
//! 5. **Resolve what the adapter needs.** The console binding through
//!    [`crate::wasl::wasl`] and the Slate binding through [`crate::wasl::slate`]
//!    — the two resolvers this crate already owns, which is what "how they are
//!    meant to be found" means. Both write nothing.
//! 6. **Hand over** to [`Tashghil::shaghghil`], the adapter's own
//!    initialisation, and to the bounded watchdog beside it.
//!
//! ## Why nothing is written into game memory
//!
//! [`taarib_haqn::Masar`] installs a detour at an address and
//! [`taarib_haqn::KhatfJadwal`] replaces a vtable slot; between them they are
//! the only things allowed to write into a game. This bootstrap constructs
//! neither, and the reason is a property of the adapter rather than an omission
//! here.
//!
//! - The three measurement and layout corrections live in [`crate::qiyas`], and
//!   [`crate::qiyas::AhdafQiyas`] is *supplied* rather than discovered. Nothing
//!   in this workspace supplies it. Resolving `FTextLayout::SetJustification`
//!   would mean either a byte-pattern database, which [`crate::slate`] refuses
//!   by name because a pattern nobody has verified eventually matches the wrong
//!   function, or a hand-written MSVC decoration, which is not reconstructible
//!   from a class and function name and would fail to resolve while looking
//!   exactly like a build that exports nothing.
//! - The console rung needs [`crate::wasl::FaharisAwamir`], and constructing one
//!   is an assertion that the caller verified this build's vtable slots. A
//!   bootstrap inside a shipped game has not verified them and cannot: the only
//!   way to check a slot index is to call it, and calling the wrong slot with
//!   the wrong signature corrupts the process it was trying to help. So [`None`]
//!   is passed, which is the case [`crate::wasl::WaslAwamir::bila_jadwal`]
//!   exists for — the manager is found and reported and every write through it
//!   is refused by name.
//!
//! What is left is the rung that never needed to be inside the process at all:
//! `[SystemSettings]` in the game's own `Engine.ini`, which Unreal applies while
//! the engine is initialising. Writing it from in here is worth doing even
//! though an installer with the game shut down is the better place for it. The
//! write is atomic, marked and reversible; it is idempotent, so it costs nothing
//! when the installer already made it; and on a build whose console slots nobody
//! has verified it is the only thing that makes the *next* launch shape Arabic
//! correctly, with no injection at all.
//!
//! ## Refusal discipline
//!
//! Every step that can decline writes one line to `<own dir>/`[`ISM_SIJILL`],
//! capped at [`AQSA_SIJILL`] bytes and truncated before the append that would
//! exceed it. A payload inside a game has no console and no channel to Studio,
//! and a game launched a thousand times must not fill a disk. The four states
//! of the contract's table are [`HalatBidaya`] and are never collapsed into each
//! other; evidence that is not an outcome is written as a `note` line, which is
//! a fifth prefix and not a fifth state.
//!
//! Nothing here may unwind. The whole body runs inside
//! [`std::panic::catch_unwind`], and so does the handler that reports a caught
//! panic, because the log write is itself a thing that can fail inside somebody
//! else's process.

use std::ffi::OsStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use taarib_haqn::mawqi::{mujallad_nafsi, qaidat_wahda};
use taarib_ruqaa::IMTIDAD;
use taarib_ruqaa::qari::MalafRuqaa;

use crate::isdar::{Bina, Tabaa, afhas};
use crate::qiyas::HadafKhatf;
use crate::slate::{DiqqatMuttajih, IsdarSlate, MasdarWasl, Unwan, WaslSlate};
use crate::tashghil::{HarisTashghil, Musaddir, Tashghil};
use crate::wasl;

// ---------------------------------------------------------------------------
// The log
// ---------------------------------------------------------------------------

/// The file every refusal, decline, failure and hand-over is written to.
///
/// Beside this module, which for a split component is the game's own `taarib/`
/// directory and for a single component is the directory holding the game's
/// executable. Named by the contract rather than chosen here, so that a
/// maintainer reading a bug report finds the same file for all three payloads.
pub const ISM_SIJILL: &str = "taarib.sijill";

/// The cap on that log, past which it is truncated before the next append.
///
/// A quarter of a megabyte is a few thousand launches' worth of lines. The same
/// number as `taarib-mudkhal`'s own cap, deliberately: two logs in one directory
/// that disagreed about how large a log may be would be a directory whose size
/// nobody can predict.
pub const AQSA_SIJILL: u64 = 262_144;

/// The name this payload writes into every log line.
pub const ISM_HAMULA: &str = "taarib-muhawwil-unreal";

/// What one line of the log reports.
///
/// The four states of the contract's table, which are distinct and must not be
/// collapsed: a game that is not Unreal has *declined*, a game whose patch is
/// missing has *declined*, a game whose `Engine.ini` cannot be found has
/// *refused* — something a person can fix — and a hook or a read that failed
/// unexpectedly has *failed*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HalatBidaya {
    /// This is not that kind of game, or no patch is installed. The game is
    /// untouched and nothing is wrong.
    Imtina,
    /// Something is wrong that the user could fix. The game is untouched.
    Rafd,
    /// A read or an install failed unexpectedly. Anything installed before the
    /// failure is removed before returning.
    Ikhfaq,
    /// The adapter's own initialisation ran and reported what took.
    Intilaq,
}

impl HalatBidaya {
    /// The word the log line carries, which is the word the contract uses.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Imtina => "declined",
            Self::Rafd => "refused",
            Self::Ikhfaq => "failed",
            Self::Intilaq => "started",
        }
    }

    /// Whether this state is worth a warning rather than an informational
    /// record, for the process's log subscriber if it happens to have one.
    #[must_use]
    pub const fn munabbiha(self) -> bool {
        matches!(self, Self::Rafd | Self::Ikhfaq)
    }
}

/// Appends one line, truncating the file first when it is over the cap.
///
/// Every failure is swallowed. This runs inside somebody's game, and a payload
/// that could not write its own log has no business interrupting a launch to
/// say so — nor any surface on which to say it.
fn uktub(mujallad: &Path, satr: &str) {
    use std::io::Write as _;

    let masar = mujallad.join(ISM_SIJILL);
    if std::fs::metadata(&masar).is_ok_and(|bayan| bayan.len() > AQSA_SIJILL) {
        let _ = std::fs::remove_file(&masar);
    }
    let Ok(mut malaf) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&masar)
    else {
        return;
    };
    let _ = writeln!(malaf, "{satr}");
}

/// Seconds since the Unix epoch, or zero when the clock cannot be read.
///
/// A timestamp rather than nothing, because the log is append-only across
/// launches and two launches whose lines cannot be told apart are two launches
/// nobody can diagnose. Zero rather than a refusal: a clock that will not answer
/// is not a reason to lose the line.
fn waqt() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |mudda| mudda.as_secs())
}

/// Records one outcome, by state and by the name of the step that produced it.
fn sajjil(mujallad: &Path, hala: HalatBidaya, khatwa: &str, tafsil: &str) {
    if hala.munabbiha() {
        tracing::warn!(
            hala = hala.ism(),
            khatwa,
            tafsil,
            "the Unreal payload's bootstrap"
        );
    } else {
        tracing::info!(
            hala = hala.ism(),
            khatwa,
            tafsil,
            "the Unreal payload's bootstrap"
        );
    }
    uktub(
        mujallad,
        &format!("{} {ISM_HAMULA} {} {khatwa}: {tafsil}", waqt(), hala.ism()),
    );
}

/// Records a fact that is not an outcome — what was found, where, and at what
/// address.
///
/// A separate prefix so that grepping the log for one of the four states
/// returns outcomes only. Evidence that had to be spelled as a state would
/// force every fact to pretend to be a decision.
fn athar(mujallad: &Path, khatwa: &str, tafsil: &str) {
    tracing::debug!(khatwa, tafsil, "the Unreal payload's bootstrap");
    uktub(
        mujallad,
        &format!("{} {ISM_HAMULA} note {khatwa}: {tafsil}", waqt()),
    );
}

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

/// How far up from this module's own directory the game root is looked for.
///
/// The payload sits either beside the game's executable —
/// `<root>/<Project>/Binaries/<platform>/`, four levels down — or in the game's
/// own `taarib/` directory, one level down. Eight is both of those with room to
/// spare, and it is a bound rather than a walk to the filesystem root because a
/// payload that examined `/` would be reading directories that are none of its
/// business.
pub const AQSA_ASLAF: usize = 8;

/// How many directory entries are examined in any one directory.
///
/// A game directory holds tens of files. A directory holding more than this is
/// one whose shape this payload has not met, and walking all of it at launch
/// would be startup cost paid for nothing.
pub const AQSA_MADAKHIL: usize = 4096;

/// How many patches beside this module are considered.
///
/// An installation places one. More than one means an old patch was left
/// behind, which is worth reporting and is not worth walking.
pub const AQSA_RUQAAT: usize = 8;

/// How many lines of the build report are copied into the log.
///
/// [`Bina::athar`] records one line per container it read, up to sixteen. All
/// sixteen on every launch would be most of the log; the first few are the ones
/// that identify the build.
pub const AQSA_ATHAR: usize = 8;

// ---------------------------------------------------------------------------
// The entry point
// ---------------------------------------------------------------------------

/// Whether the bootstrap has already run in this process.
///
/// The loader calls the entry once. This guard is for the case it does not:
/// two payload directories, a proxy that was loaded twice, a game that reloads
/// its plugins. Running twice would start a second watchdog thread and write a
/// second block into the game's configuration, and neither is undone by the
/// first one having succeeded.
static MARRA: AtomicBool = AtomicBool::new(false);

/// The symbol `taarib-mudkhal` resolves and calls once, on Windows.
///
/// `extern "system"` because that is what the loader transmutes to on this
/// platform; on every Windows target in the matrix it is the same convention as
/// `extern "C"`, and spelling it the way the caller spells it is what keeps that
/// true if a target is ever added where it is not.
///
/// It takes nothing, returns nothing, and cannot unwind: the body is wrapped in
/// [`std::panic::catch_unwind`], because this workspace builds with
/// `panic = "unwind"` so that a fault inside somebody's game is caught at the
/// ABI edge and reported rather than aborting their process.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn taarib_bidaya() {
    ihmi();
}

/// The symbol `taarib-mudkhal` resolves and calls once, everywhere else.
///
/// See the Windows arm; the two differ only in the convention the loader
/// transmutes to.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn taarib_bidaya() {
    ihmi();
}

/// Runs the bootstrap with the unwind boundary closed.
///
/// Two nested `catch_unwind`s and not one. The inner one guards the work; the
/// outer one guards the *report*, because reporting means resolving this
/// module's directory and writing a file, and a panic there would unwind out of
/// the panic handler and into the game.
fn ihmi() {
    let Err(dhuar) = catch_unwind(AssertUnwindSafe(ibda_mahmi)) else {
        return;
    };
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Some(mujallad) = mujallad_nafsi() else {
            return;
        };
        sajjil(
            &mujallad,
            HalatBidaya::Ikhfaq,
            "bidaya",
            &format!(
                "the bootstrap panicked and the game was left untouched: {}",
                wasf_dhuar(&*dhuar)
            ),
        );
    }));
}

/// A panic payload as a sentence, without assuming what it is.
fn wasf_dhuar(dhuar: &(dyn core::any::Any + Send)) -> &str {
    dhuar
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| dhuar.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("the panic payload was neither a string nor a &str")
}

/// The bootstrap, from the one question that has to be answered before there is
/// anywhere to report to.
///
/// A module whose own directory cannot be resolved returns in silence. That is
/// the one path in this file with no log line, and it cannot be otherwise: the
/// log lives in the directory that could not be resolved.
fn ibda_mahmi() {
    let Some(mujallad) = mujallad_nafsi() else {
        return;
    };

    if MARRA.swap(true, Ordering::AcqRel) {
        sajjil(
            &mujallad,
            HalatBidaya::Imtina,
            "bidaya",
            "taarib_bidaya has already run in this process; the second call did nothing, \
             because running twice would start a second watchdog and write a second \
             configuration block",
        );
        return;
    }

    let (hala, tafsil) = ibda(&mujallad);
    sajjil(&mujallad, hala, "bidaya", &tafsil);
}

// ---------------------------------------------------------------------------
// The body
// ---------------------------------------------------------------------------

/// The ordered body of the contract, returning the state of the whole run.
///
/// Every step logs its own outcome by name as it happens; the pair returned
/// here is the closing line, so a reader who greps for one launch's last
/// `bidaya` line learns what the payload did without reading the rest.
fn ibda(mujallad: &Path) -> (HalatBidaya, String) {
    athar(
        mujallad,
        "mawqi",
        &format!("loaded from {}", mujallad.display()),
    );

    // 2 — what am I inside.
    sajjil_wahda(mujallad);
    let bina = match jid_bina(mujallad) {
        Ok(bina) => bina,
        Err(sabab) => {
            sajjil(mujallad, HalatBidaya::Imtina, "bina", &sabab);
            return (
                HalatBidaya::Imtina,
                "this process is not an Unreal game, so the adapter declined and the game \
                 was left untouched"
                    .to_owned(),
            );
        },
    };
    athar(
        mujallad,
        "bina",
        &format!("{} at {}", bina.wasf(), bina.jidhr.display()),
    );
    for satr in bina.athar.iter().take(AQSA_ATHAR) {
        athar(mujallad, "bina", satr);
    }

    // 3 — the patch.
    let ruqaat = ruqaat(mujallad);
    if ruqaat.is_empty() {
        let tafsil = format!(
            "no *.{IMTIDAD} patch is installed beside this module, so there is nothing to \
             apply and nothing was touched"
        );
        sajjil(mujallad, HalatBidaya::Imtina, "ruqaa", &tafsil);
        return (HalatBidaya::Imtina, tafsil);
    }
    let Some(malaf) = iftah_ruqaa(mujallad, &ruqaat) else {
        let tafsil = format!(
            "{} patch file(s) are installed beside this module and none of them validated; \
             reinstalling the patch is what fixes it",
            ruqaat.len()
        );
        sajjil(mujallad, HalatBidaya::Rafd, "ruqaa", &tafsil);
        return (HalatBidaya::Rafd, tafsil);
    };
    // The mapping has done what it was opened for: the patch is present, framed
    // correctly, and is the bytes its own hash names. Nothing installed below
    // reads it on a frame path, so the address space goes back to the game here
    // rather than being held for the rest of the bootstrap.
    drop(malaf);

    // 4 — the disclosure is tier 3's, and this is tier 1. Nothing to do, and
    //     nothing here that could route around it.

    // 5 — resolve what the adapter needs, through the crate's own resolvers.
    let musaddir = wasl::wasl(None);
    sajjil_awamir(mujallad, musaddir);
    sajjil_slate(mujallad, &bina);
    sajjil_qiyas(mujallad);

    // 6 — hand over.
    slim_lil_muhawwil(mujallad, &bina, musaddir)
}

/// Records the module this payload exists to take over.
///
/// Unreal links the engine into the game's executable, so this is the
/// executable's own image. The lookup is [`qaidat_wahda`] — the contract's call
/// — and its answer is evidence rather than a decision, for the reason set out
/// in this module's header.
fn sajjil_wahda(mujallad: &Path) {
    let Some(ism) = ism_tanfidhi() else {
        athar(
            mujallad,
            "wahda",
            "the running executable's name could not be read, so the primary image was not \
             looked up by name; the build report below is what identifies this process",
        );
        return;
    };
    match qaidat_wahda(&ism).and_then(|qaida| Unwan::min_muashir(qaida.cast_const())) {
        Some(qaida) => athar(
            mujallad,
            "wahda",
            &format!(
                "{ism} is mapped at {:#x} and is the module Unreal linked the engine \
                      into",
                qaida.raqm()
            ),
        ),
        None => athar(
            mujallad,
            "wahda",
            &format!(
                "the loader would not name {ism} as a module, which is the ordinary answer \
                 on Unix — a by-name lookup there resolves shared objects and not the \
                 program image — so the build report below is what identifies this process"
            ),
        ),
    }
}

/// The running executable's file name.
fn ism_tanfidhi() -> Option<String> {
    let masar = std::env::current_exe().ok()?;
    let ism = masar.file_name()?.to_str()?;
    Some(ism.to_owned())
}

// ---------------------------------------------------------------------------
// Step 2 — the build
// ---------------------------------------------------------------------------

/// Finds the game root above this module and identifies the build in it.
///
/// The walk is upwards from this module's own directory because that is the one
/// location a payload knows for certain, and it stops at the first ancestor
/// that [`afhas`] recognises — which is the first one holding a
/// `<Project>/Content` or `<Project>/Content/Paks`, since that is the shape
/// [`afhas`] identifies a build by. A directory that is not recognised is not
/// an error and not a reason to stop: `<Project>/Binaries/Win64` is three such
/// directories below the root, and every one of them has to be walked past.
///
/// # Errors
///
/// The list of directories that were examined, as one sentence, when none of
/// them was an Unreal build. The caller turns that into the decline.
fn jid_bina(mujallad: &Path) -> Result<Bina, String> {
    let mut jurribat: Vec<String> = Vec::with_capacity(AQSA_ASLAF);
    let mut hali = Some(mujallad);

    for _ in 0..AQSA_ASLAF {
        let Some(marshah) = hali else { break };
        match afhas(marshah) {
            Ok(bina) if bina.tabaa != Tabaa::Majhul => return Ok(bina),
            Ok(_) => jurribat.push(format!("{}: no Unreal content layout", marshah.display())),
            Err(khata) => jurribat.push(format!("{}: {khata}", marshah.display())),
        }
        hali = marshah.parent();
    }

    Err(format!(
        "no Unreal build was found at or above this module, so this is not a game this \
         payload has anything to do ({})",
        jurribat.join("; ")
    ))
}

// ---------------------------------------------------------------------------
// Step 3 — the patch
// ---------------------------------------------------------------------------

/// The patches installed beside this module, in a stable order.
fn ruqaat(mujallad: &Path) -> Vec<PathBuf> {
    let Ok(madakhil) = std::fs::read_dir(mujallad) else {
        return Vec::new();
    };

    let mut kull: Vec<PathBuf> = Vec::new();
    for madkhal in madakhil.flatten().take(AQSA_MADAKHIL) {
        if kull.len() >= AQSA_RUQAAT {
            break;
        }
        let masar = madkhal.path();
        if !masar.is_file() {
            continue;
        }
        let imtidad = masar
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default();
        if imtidad.eq_ignore_ascii_case(IMTIDAD) {
            kull.push(masar);
        }
    }
    kull.sort();
    kull
}

/// Opens the first patch that validates, reporting every one that does not.
///
/// Validation is [`taarib_ruqaa::qari::Ruqaa::iftah`]'s: the header, the section
/// table, and the content hash, before any record is cast. A patch that fails it
/// is a patch this payload will not read the body of, and saying which one
/// failed and why is the difference between a fixable install and a mystery.
///
/// The mapping is returned rather than dropped here so that its lifetime is the
/// caller's decision — the caller gives it back as soon as the patch has been
/// accounted for. Holding a mapping open for the life of a game in order to read
/// nothing from it would be address space taken from the game for no purpose,
/// and a 32-bit build has little enough of it.
fn iftah_ruqaa(mujallad: &Path, ruqaat: &[PathBuf]) -> Option<MalafRuqaa> {
    let mut maftuh: Option<MalafRuqaa> = None;

    for masar in ruqaat {
        let ism = masar
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("the patch");
        match MalafRuqaa::iftah(masar) {
            Ok(malaf) => {
                if maftuh.is_some() {
                    athar(
                        mujallad,
                        "ruqaa",
                        &format!(
                            "{ism} is a second installed patch and was not read; one \
                             installation places one patch, and an extra one is usually an \
                             old file left behind"
                        ),
                    );
                    continue;
                }
                athar(mujallad, "ruqaa", &wasf_ruqaa(ism, &malaf));
                maftuh = Some(malaf);
            },
            Err(khata) => sajjil(
                mujallad,
                HalatBidaya::Rafd,
                "ruqaa",
                &format!("{ism} did not validate: {khata}"),
            ),
        }
    }
    maftuh
}

/// One patch as a log line: what it carries and which bytes it is.
fn wasf_ruqaa(ism: &str, malaf: &MalafRuqaa) -> String {
    match malaf.ruqaa() {
        Ok(ruqaa) => format!(
            "{ism} validated: {} section(s), {} byte(s), content {}",
            ruqaa.jard().len(),
            ruqaa.tarwisa().hajm_kulli,
            sittasi_mukhtasar(ruqaa.basma())
        ),
        // Unreachable in practice — the same bytes validated when the file was
        // opened — and reported rather than asserted, because the bytes are a
        // mapping of a file somebody else can reach.
        Err(khata) => format!("{ism} validated at open and would not re-read: {khata}"),
    }
}

/// The first eight bytes of a hash, in lower-case hexadecimal.
///
/// Enough to tell two patches apart in a log and short enough not to fill the
/// line. The whole hash belongs in a diagnostics bundle, not in a file that is
/// appended to on every launch.
fn sittasi_mukhtasar(basma: &[u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut nass = String::with_capacity(16);
    for wahda in basma.iter().take(8) {
        // Writing into the buffer rather than formatting a fresh `String` per
        // byte: this runs on every launch inside somebody's game.
        let _ = write!(nass, "{wahda:02x}");
    }
    nass
}

// ---------------------------------------------------------------------------
// Step 5 — what resolved, and what did not
// ---------------------------------------------------------------------------

/// Records the console binding and why it will not be written through.
///
/// [`crate::wasl::wasl`] is given [`None`]: the slot indices of
/// `IConsoleManager::FindConsoleVariable` and `IConsoleVariable::Set` are a
/// property of one engine build, and [`crate::wasl::FaharisAwamir`] is a
/// caller's assertion that it verified them. This bootstrap has not, and cannot
/// from inside a shipped game — the only way to test a slot is to call it, and
/// calling the wrong one with the wrong signature is a crash in a player's game
/// with Taarib's name on it. So the manager is bound for reporting, every write
/// through it is refused by name, and the ladder falls back to the rung that
/// never needed to be in the process at all.
fn sajjil_awamir(mujallad: &Path, musaddir: &wasl::WaslAwamir) {
    if musaddir.wujid() {
        sajjil(
            mujallad,
            HalatBidaya::Imtina,
            "awamir",
            "IConsoleManager::Get resolved in this build, and the console rung was still not \
             taken: writing through a vtable slot whose index nobody verified against this \
             build is how an overlay calls an unrelated engine function. Nothing is wrong \
             and there is nothing for anyone to fix; the configuration rung carries the \
             correction instead",
        );
    } else {
        sajjil(
            mujallad,
            HalatBidaya::Imtina,
            "awamir",
            &format!(
                "the console manager is not reachable in this build, which is the ordinary \
                 case for a shipping monolithic game: {}",
                musaddir.sabab()
            ),
        );
    }
}

/// Records whether Slate itself is reachable, and at which return width.
///
/// Diagnostic and nothing more: no correction is installed on it, and the
/// binding writes nothing. It is still worth doing on every launch, because
/// "Slate resolved as `_ZN17FSlateApplication3GetEv` in the primary image" and
/// "none of six spellings resolved in any of six modules" are the two sentences
/// that decide what a maintainer does next, and neither can be obtained with the
/// game shut down.
fn sajjil_slate(mujallad: &Path, bina: &Bina) {
    let Some(isdar) = bina.isdar.as_ref() else {
        sajjil(
            mujallad,
            HalatBidaya::Imtina,
            "slate",
            &format!(
                "the engine version could not be narrowed to one release, so the width of \
                 Slate's measurement return value is unknown and was not guessed at ({})",
                bina.wasf()
            ),
        );
        return;
    };

    let isdar = IsdarSlate::jadeed(u32::from(isdar.kabir), u32::from(isdar.sagheer));
    match wasl::slate(isdar) {
        Ok(mawsul) => athar(mujallad, "slate", &wasf_slate(&mawsul)),
        Err(khata) => {
            sajjil(mujallad, HalatBidaya::Imtina, "slate", &khata.to_string());
        },
    }
}

/// One Slate binding as a log line: where it came from and what it resolved as.
fn wasf_slate(mawsul: &WaslSlate) -> String {
    let masdar = match mawsul.masdar() {
        MasdarWasl::WahdaRaisiya => "the process's primary image",
        MasdarWasl::WahdaMustaqilla => "a separate Slate module",
    };
    let diqqa = match mawsul.diqqa() {
        DiqqatMuttajih::Mufrada => "FVector2f, two f32",
        DiqqatMuttajih::Mudaafa => "FVector2d, two f64",
    };
    format!(
        "Slate is reachable: {} resolved in {masdar} at {:#x}, and this engine returns a \
         measurement as {diqqa}",
        mawsul.ramz(),
        mawsul.tatbeeq().raqm()
    )
}

/// Records each measurement and layout correction that is not installed, by the
/// name of the function it would have been installed on.
///
/// Four lines on every launch, and they earn their place. The corrections in
/// [`crate::qiyas`] take their addresses from a caller; nothing in this
/// workspace supplies them, and this file will not invent one — a detour at a
/// guessed address is a crash in a player's game, and a byte-pattern database
/// nobody has verified against this build is the same thing with a longer fuse.
/// So [`taarib_haqn::Masar`] is never constructed here, no vtable slot is
/// replaced, and nothing at all is written into the game's memory. Saying so by
/// name is what stops the next reader concluding that the corrections were
/// installed and silently did nothing.
fn sajjil_qiyas(mujallad: &Path) {
    const AHDAF: [HadafKhatf; 4] = [
        HadafKhatf::Ittijah,
        HadafKhatf::Muhadhaha,
        HadafKhatf::Laff,
        HadafKhatf::Mujassam,
    ];

    for hadaf in AHDAF {
        sajjil(
            mujallad,
            HalatBidaya::Imtina,
            "qiyas",
            &format!("{}: {}", hadaf.ism(), hadaf.tafsil_ghiyab()),
        );
    }
    athar(
        mujallad,
        "qiyas",
        "no detour and no vtable slot was installed, so this process is byte-identical to \
         the one this payload entered",
    );
}

// ---------------------------------------------------------------------------
// Step 6 — the hand-over
// ---------------------------------------------------------------------------

/// Hands control to [`Tashghil::shaghghil`] and, when there is anything to
/// hold, to the watchdog beside it.
///
/// The ini is found rather than derived. `<Project>/Saved/Config/<platform>/`
/// is spelled `Windows` on Unreal 5 and `WindowsNoEditor` on Unreal 4, and a
/// payload that wrote its correction into a path it had guessed would leave a
/// file the engine never reads in a directory the player never asked for. So
/// only a file that is already there is written to, and a game that has not yet
/// produced one is a named refusal: the next launch, after the engine has
/// written its own configuration once, finds it.
fn slim_lil_muhawwil(
    mujallad: &Path,
    bina: &Bina,
    musaddir: &'static wasl::WaslAwamir,
) -> (HalatBidaya, String) {
    let Some(ini) = jid_ini(&bina.jidhr) else {
        let tafsil = format!(
            "no Engine.ini was found under {}, so the shaping correction had nowhere to be \
             written; Unreal writes <Project>/Saved/Config/<platform>/Engine.ini during its \
             first run, and this payload will not invent that path inside a running game",
            bina.jidhr.display()
        );
        sajjil(mujallad, HalatBidaya::Rafd, "tashghil", &tafsil);
        return (HalatBidaya::Rafd, tafsil);
    };
    athar(
        mujallad,
        "tashghil",
        &format!("the game's configuration is {}", ini.display()),
    );

    let tashghil = Tashghil::jadeed(ini);
    let sijill = match tashghil.shaghghil(Some(musaddir)) {
        Ok(sijill) => sijill,
        Err(khata) => {
            let tafsil = khata.to_string();
            sajjil(mujallad, HalatBidaya::Ikhfaq, "tashghil", &tafsil);
            return (HalatBidaya::Ikhfaq, tafsil);
        },
    };

    for natija in &sijill.rutab {
        athar(
            mujallad,
            "tashghil",
            &format!(
                "rung {} ({}) {}: {}",
                natija.rutba.raqm(),
                natija.rutba.ism(),
                if natija.muakkada {
                    "confirmed"
                } else {
                    "unconfirmed"
                },
                natija.mulahaza
            ),
        );
    }

    let tafsil = match sijill.nafidha {
        Some(rutba) => format!(
            "full shaping is on and confirmed through rung {} ({})",
            rutba.raqm(),
            rutba.ism()
        ),
        None => "full shaping is written into the game's configuration and applies at the \
                 next launch; no rung could be confirmed in this process"
            .to_owned(),
    };
    sajjil(mujallad, HalatBidaya::Intilaq, "tashghil", &tafsil);

    ibda_haris(mujallad, &tashghil, musaddir);
    (HalatBidaya::Intilaq, tafsil)
}

/// Starts the bounded re-assert watchdog, when there is a binding it can read
/// through.
///
/// Gated on [`Musaddir::muhayya`] and not started otherwise. A watchdog whose
/// every read fails is a thread that wakes sixty times inside somebody's game to
/// learn nothing, shows up in their profile, and is its own bug — and
/// [`HarisTashghil`] is deliberately bounded for the same reason.
///
/// The thread is detached. `taarib-mudkhal` never unloads a payload it has
/// loaded, so the code the thread runs in stays mapped; a loader that did unload
/// would have to join it first, and there is no such loader in this product.
fn ibda_haris(mujallad: &Path, tashghil: &Tashghil, musaddir: &'static wasl::WaslAwamir) {
    if !musaddir.muhayya() {
        athar(
            mujallad,
            "haris",
            "the watchdog was not started: this binding cannot read a console variable back, \
             so every tick would report a failure about a game that is behaving correctly",
        );
        return;
    }

    let tawkeedat = tashghil.tawkeedat();
    if tawkeedat.is_empty() {
        athar(
            mujallad,
            "haris",
            "the watchdog was not started: nothing to hold",
        );
        return;
    }

    let adad = tawkeedat.len();
    let mut haris = HarisTashghil::jadeed(tawkeedat);
    let khayt = std::thread::Builder::new()
        .name("taarib-haris".to_owned())
        .spawn(move || {
            let _ = haris.raqib(musaddir);
        });

    match khayt {
        Ok(_) => athar(
            mujallad,
            "haris",
            &format!("the watchdog is holding {adad} assertion(s) on its own thread"),
        ),
        Err(sabab) => sajjil(
            mujallad,
            HalatBidaya::Ikhfaq,
            "haris",
            &format!(
                "the watchdog thread could not be started, so a game that resets the shaping \
                 method will not be corrected in this session: {sabab}"
            ),
        ),
    }
}

/// The game's own `Engine.ini`, found by shape and never derived.
///
/// `<root>/<Project>/Saved/Config/<platform>/Engine.ini`. The project directory
/// is not known in advance and the platform directory is spelled differently
/// across engine versions, so both are found by looking rather than by naming —
/// the same reason [`crate::isdar`] finds the container directory by shape. The
/// two segments that *are* named — `Saved/Config` and `Engine.ini` — go through
/// [`crate::isdar::masar_bila_hala`], because a game running under Wine writes
/// them through a case-insensitive filesystem and this payload also runs where
/// that filesystem is not.
///
/// When more than one exists — a game whose engine was upgraded keeps both
/// `Windows` and `WindowsNoEditor` — the most recently written one wins, because
/// that is the one the engine in use is writing. Ties fall back to path order so
/// that two launches of one game never disagree.
fn jid_ini(jidhr: &Path) -> Option<PathBuf> {
    let mut murashahat: Vec<(SystemTime, PathBuf)> = Vec::new();

    for mashru in mujalladat(jidhr) {
        if mashru
            .file_name()
            .is_some_and(|ism| ism.eq_ignore_ascii_case("Engine"))
        {
            continue;
        }
        let idadat = crate::isdar::masar_bila_hala(&mashru, "Saved/Config");
        for manassa in mujalladat(&idadat) {
            let ini = crate::isdar::masar_bila_hala(&manassa, "Engine.ini");
            let Ok(bayan) = std::fs::metadata(&ini) else {
                continue;
            };
            if !bayan.is_file() {
                continue;
            }
            murashahat.push((bayan.modified().unwrap_or(UNIX_EPOCH), ini));
        }
    }

    murashahat.sort_by(|(awwal, masar_awwal), (thani, masar_thani)| {
        thani.cmp(awwal).then_with(|| masar_awwal.cmp(masar_thani))
    });
    murashahat.into_iter().next().map(|(_, masar)| masar)
}

/// The subdirectories of one directory, in a stable order.
///
/// Sorted, because a game's configuration must not depend on the order the file
/// system happened to return entries in, and empty for a directory that cannot
/// be listed — which for `Saved/Config` is the ordinary answer before a game's
/// first run.
fn mujalladat(jidhr: &Path) -> Vec<PathBuf> {
    let Ok(madakhil) = std::fs::read_dir(jidhr) else {
        return Vec::new();
    };

    let mut kull: Vec<PathBuf> = Vec::new();
    for madkhal in madakhil.flatten().take(AQSA_MADAKHIL) {
        let masar = madkhal.path();
        if masar.is_dir() {
            kull.push(masar);
        }
    }
    kull.sort();
    kull
}
