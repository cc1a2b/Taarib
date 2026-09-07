//! التركيب — the one place a script-engine patch is written into a game.
//!
//! Every other module here can *read* a game and can *write* one, and until this
//! module existed nothing joined the two. `tawjih` answers "which adapter and
//! which rung", `hifz` answers "how may a file be touched", and the four engine
//! modules answer "what does the patched file look like" — but the sentence
//! "here is a game and here are its translations, apply them" had no home, so
//! the installer never said it. The result was a package that placed a plugin,
//! a Python module and a `.ruqaa` beside a game whose own data files still held
//! the developer's English.
//!
//! ## What this module is, and what it deliberately is not
//!
//! It is a dispatcher. It identifies which of the four adapters applies to a
//! directory, asks that adapter for the strings the game actually holds, looks
//! each one up, and hands the adapter back the replacements through the write
//! entry point the adapter already had. It contains no format knowledge: not one
//! byte offset, not one chunk name, not one escape code. Everything it calls was
//! already written and already correct; what was missing was the caller.
//!
//! It is also not the runtime side. Three of these four engines also load a
//! Taarib adapter at run time — `js/plugins/taarib.js`, `game/taarib_renpy/`,
//! the injected renderer — and those are deployed by the installer's own
//! component store. This module writes *text into files the engine reads on its
//! own*, which is the half that works on every one of the four whether or not
//! the runtime half loaded.
//!
//! ## The lookup key, stated once
//!
//! [`Mutarjim::tarjim`] is asked for a source string **exactly as the game's own
//! file holds it**. The compiler keys its table on the *clean* source — markup
//! and placeholders lifted into spans rather than left inside the string — so
//! for a string carrying no markup the two spellings are one string and the
//! lookup hits directly. Where an adapter has computed a clean form of its own,
//! this module offers that as a second key: RPG Maker's reader produces
//! [`rpgmaker::MadkhalNusus::naqi`] beside the raw literal, and both are tried,
//! raw first. No other adapter here has a clean form, so no other one guesses at
//! producing one.
//!
//! ## Why a translation is skipped rather than forced
//!
//! A translation that has lost an escape code the engine substitutes at draw
//! time — an icon, a variable, an actor's name, the currency unit — is a
//! translation that has lost information the player was meant to see. RPG Maker
//! states which codes those are and [`rpgmaker::tahaqquq_hurub`] refuses one
//! that dropped any of them. This module runs that check *before* the write
//! rather than letting the writer raise: one line with a missing `\V[7]` is a
//! line to leave in English and count, not a reason to abandon an install that
//! is otherwise correct. [`TaqreerTarkeeb::matruka`] carries the count and
//! [`TaqreerTarkeeb::mulahazat`] says why.
//!
//! ## The order this runs in
//!
//! Before the installer deploys its adapter files, never after, and RPG Maker is
//! why. Its extraction records carry byte offsets into `js/plugins.js`, and
//! registering a plugin appends an entry to that file. Splicing against offsets
//! measured before that append would be splicing against a file that has moved;
//! [`rpgmaker::rakkib`] would notice and refuse, which is the safe failure and
//! still a wasted install. `rpgmaker::rakkib_mulhaq` documents the same ordering
//! for the same reason.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_muharrik::dalail::nusus::HadafNusus;
use taarib_mustalahat::muharrik::AilatMuharrik;

use crate::hifz::Hafiz;
use crate::khata::KhataNusus;
use crate::tabaqa::{Mifhas, Rutba, SiyaqTabaqa, hukm};
use crate::{electron, gamemaker, renpy, rpgmaker};

/// A source string's Arabic, when the patch carries one.
///
/// A trait rather than a map so that this crate never learns the shape of a
/// `.ruqaa`: the installer owns the container format and hands this module a
/// lookup, which is the whole of what a patcher needs from a patch. It also
/// means a round-trip proof can drive the same code path from a plain map.
pub trait Mutarjim {
    /// The Arabic for one source string, or [`None`] when the patch has none.
    ///
    /// Asked with the string as the game's own file holds it. See this module's
    /// header for what that means when the string carries markup.
    fn tarjim(&self, asl: &str) -> Option<&str>;
}

impl Mutarjim for BTreeMap<String, String> {
    fn tarjim(&self, asl: &str) -> Option<&str> {
        self.get(asl).map(String::as_str)
    }
}

impl<T: Mutarjim + ?Sized> Mutarjim for &T {
    fn tarjim(&self, asl: &str) -> Option<&str> {
        (**self).tarjim(asl)
    }
}

/// The build artifacts an adapter needs and this crate does not produce.
///
/// Two of the four engines take a payload that was compiled by something other
/// than `cargo` — the Electron renderer runtime is TypeScript that `esbuild`
/// turned into JavaScript — and one takes a font file name that the installer's
/// component store decides. Passing them in keeps this crate free of both: it
/// embeds what the build produced and reads nothing from a component store it
/// has no business knowing the layout of.
#[derive(Debug, Default, Clone, Copy)]
pub struct Mawarid<'a> {
    /// The compiled Electron renderer runtime — `adapters-script/electron/taarib.ts`
    /// after the build has run over it.
    ///
    /// [`None`] means the Electron adapter declines: injecting a payload with no
    /// runtime to read it writes a file into somebody's `app.asar` and changes
    /// nothing about what they see.
    pub tashghil_ghilaf: Option<&'a str>,

    /// The Arabic font the Ren'Py patch installed, relative to `game/`.
    ///
    /// [`None`] leaves the generated file's font registration out entirely
    /// rather than registering an empty name — Ren'Py's `sajjil_khatt` assigns
    /// whatever it is given to every `gui` font variable, so an empty name is
    /// not a no-op, it is a game with no font.
    pub khatt_renpy: Option<&'a str>,
}

/// What one script-engine write did.
#[derive(Debug, Clone)]
pub struct TaqreerTarkeeb {
    /// Which adapter ran.
    pub hadaf: HadafNusus,
    /// The engine family the locator established from the game's own files.
    pub aila: AilatMuharrik,
    /// The rung the probe settled on, when it settled on one.
    ///
    /// [`None`] means the shaping evidence was contradictory or absent and the
    /// probe refused to choose. That stops the two adapters whose payload
    /// *states* a rung and does not stop the two whose write is a data splice —
    /// see [`rakkib_luba`] for why those are different questions.
    pub rutba: Option<Rutba>,
    /// How sure the probe was, 0..=100, and zero when it refused.
    pub thiqa: u8,
    /// How many of the game's files were rewritten or added.
    pub malaffat: usize,
    /// How many strings were replaced.
    pub nusus: usize,
    /// How many source strings the patch had no translation for, or had one
    /// this module refused to write.
    pub matruka: usize,
    /// Every path written, for the install report.
    pub masarat: Vec<PathBuf>,
    /// Anything a person reading the report needs to know about what was left
    /// undone, in this module's own words.
    pub mulahazat: Vec<String>,
}

impl TaqreerTarkeeb {
    /// An empty report for one adapter's verdict.
    const fn min_hukm(
        hadaf: HadafNusus,
        aila: AilatMuharrik,
        rutba: Option<Rutba>,
        thiqa: u8,
    ) -> Self {
        Self {
            hadaf,
            aila,
            rutba,
            thiqa,
            malaffat: 0,
            nusus: 0,
            matruka: 0,
            masarat: Vec::new(),
            mulahazat: Vec::new(),
        }
    }

    /// The lines this write contributes to the install report.
    #[must_use]
    pub fn sutur(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "{} at {} (confidence {}): {} string(s) written into {} file(s), {} left as they \
             were",
            self.hadaf.ism(),
            self.rutba.map_or("an undetermined rung", Rutba::ism),
            self.thiqa,
            self.nusus,
            self.malaffat,
            self.matruka
        )];
        for masar in &self.masarat {
            sutur.push(format!("  wrote {}", masar.display()));
        }
        for mulahaza in &self.mulahazat {
            sutur.push(format!("  note: {mulahaza}"));
        }
        sutur
    }
}

/// Resolves one child name against a directory, ignoring letter case.
///
/// A Windows game running under Wine or Proton sees a case-insensitive
/// filesystem, so a depot that ships `Game/` where the engine's own convention
/// is `game/` runs correctly and is invisible to a literal `Path::join` on
/// Linux. The consequence here is not a missed detection but a silent one:
/// [`ayn_hadaf`] answers [`None`], the dispatcher treats that as "no adapter
/// applies", and the install reports success having translated nothing —
/// exactly the outcome the pipeline is not allowed to produce. Steam depots are
/// usually consistent; Bottles, Lutris and Heroic installs are much less so.
///
/// The exact name is tried first, so the common case costs no directory read.
pub(crate) fn ibn_bila_hala(jidhr: &Path, ism: &str) -> Option<PathBuf> {
    let mubashir = jidhr.join(ism);
    if mubashir.exists() {
        return Some(mubashir);
    }
    // Bounded like every other directory walk in this crate: a game root is
    // somebody else's data, and an unbounded read there is a promise about
    // memory this code cannot keep.
    fs::read_dir(jidhr)
        .ok()?
        .take(4_096)
        .flatten()
        .find(|madkhal| {
            madkhal
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(ism)
        })
        .map(|madkhal| madkhal.path())
}

/// Resolves a whole relative path against a directory, ignoring letter case.
///
/// [`ibn_bila_hala`] applied one component at a time, with the rule a single
/// fold does not have: **once a component is missing, every component after it
/// is passed through verbatim** rather than searched for. That is what makes one
/// helper serve both directions. Reading `resources/app.asar` under a depot that
/// spells it `Resources/` finds the file; writing `game/tl/arabic/…` under a
/// depot that spells it `Game/` lands inside the directory the engine already
/// loads, and creates `tl/arabic/` under it with the spelling the caller asked
/// for rather than inventing a second tree beside the game's own.
///
/// That second case is the one worth stating plainly, because it is worse than a
/// missed detection: a patch written into a freshly created `game/` beside an
/// existing `Game/` is a patch the engine never reads, and — under Wine, where
/// an exact match wins a case-insensitive lookup — a directory that can shadow
/// the game's own content.
///
/// `nisbi` is a crate constant such as `game/tl/arabic/taarib_idad.rpy`, split on
/// both separators because these constants are read on Windows too.
///
/// This lives here rather than in a utility module because [`ibn_bila_hala`] is
/// here: one resolver per crate, so that the bound and the fast path cannot
/// drift between two copies of the same walk.
pub(crate) fn masar_bila_hala(jidhr: &Path, nisbi: &str) -> PathBuf {
    let mut mabni = jidhr.to_path_buf();
    let mut mafqud = false;
    for juz in nisbi
        .split(['/', '\\'])
        .filter(|juz| !juz.is_empty() && *juz != ".")
    {
        if mafqud {
            mabni.push(juz);
            continue;
        }
        if let Some(mawjud) = ibn_bila_hala(&mabni, juz) {
            mabni = mawjud;
        } else {
            mafqud = true;
            mabni.push(juz);
        }
    }
    mabni
}

/// Which adapter applies to a directory, and the engine family that implies.
///
/// Identified from the game's own files rather than taken from a caller,
/// because the install pipeline reaches this point holding a game root and a
/// package and nothing about the engine — and because every one of these four
/// locators is the same one the corresponding probe already uses, so there is no
/// second opinion being formed here.
///
/// **An inner engine beats the wrapper**, which is [`crate::tawjih`]'s rule
/// restated in the one other place it has to hold: an Electron shell around an
/// RPG Maker MV game is patched as MV, because a shell upgrade replaces
/// `app.asar` wholesale and leaves `data/` alone, and the reverse is not true.
/// The order of the checks below *is* that rule.
///
/// Returns [`None`] for a directory none of the four adapters can act on, which
/// is the ordinary answer for a Unity or Unreal game and is not a failure.
#[must_use]
pub fn ayn_hadaf(jidhr: &Path) -> Option<(HadafNusus, AilatMuharrik)> {
    if let Ok(bunya) = rpgmaker::BunyatMashru::iktashif(jidhr) {
        return Some((HadafNusus::RpgMakerJs, bunya.isdar().aila()));
    }
    // The leaf names are folded as well as the directories. A depot that
    // upper-cased `Game/` is a depot that was repacked by something with a
    // case-insensitive view of the tree, and there is no reason to trust it to
    // have left `script.rpyc` alone while it changed the directory above it.
    let renpy = ibn_bila_hala(jidhr, "renpy");
    if renpy.as_ref().is_some_and(|masar| masar.is_dir())
        || masar_bila_hala(jidhr, "renpy/__init__.py").is_file()
        || masar_bila_hala(jidhr, "game/script.rpyc").is_file()
    {
        return Some((HadafNusus::RenPy, AilatMuharrik::Renpy));
    }
    if gamemaker::MifhasGameMaker::hawiya(jidhr).is_some() {
        return Some((HadafNusus::GameMaker, AilatMuharrik::GameMaker));
    }
    if electron::hawiya(jidhr).is_some() {
        return Some((HadafNusus::Ghilaf, AilatMuharrik::Electron));
    }
    None
}

/// Writes a patch's translations into one game, through its own engine's data.
///
/// Returns [`None`] when no adapter in this crate applies to the directory,
/// which is not a failure: most games are not built on one of these engines, and
/// an installer calling this unconditionally is the point.
///
/// The rung is established here, once, by the same probe [`crate::tawjih`] runs,
/// and is *not* re-derived by anything downstream.
///
/// **A probe that refuses does not necessarily stop the write**, and the split
/// is deliberate. The rung is a statement about *shaping*: whether the engine
/// joins Arabic letters and resolves direction on its own. Two of these four
/// adapters write a settings file that tells their runtime side which rung to
/// behave as — Ren'Py's `idad.json` and the Electron payload — and a guessed
/// rung there is either an engine taken over that was already correct or Arabic
/// that does not join, so those two refuse when the probe does. The other two
/// write translated text into a data file and never consult the rung at all;
/// refusing to translate an RPG Maker map because the shaping evidence about its
/// core scripts is ambiguous would be answering a question nobody asked. Those
/// two proceed and [`TaqreerTarkeeb::rutba`] is [`None`], with the probe's own
/// sentence in [`TaqreerTarkeeb::mulahazat`].
///
/// # Errors
///
/// [`KhataNusus::TabaqaMajhula`] when the probe cannot establish a rung *and*
/// the adapter's payload states one, and whatever the adapter that ran refuses:
/// a container that will not round-trip, a data file that has drifted from the
/// extraction it was measured against, an original that cannot be preserved.
/// Nothing partial is left behind by a refusal — each adapter verifies its own
/// output in memory before the guard is handed a byte.
pub fn rakkib_luba(
    jidhr: &Path,
    mutarjim: &dyn Mutarjim,
    mawarid: &Mawarid<'_>,
    hafiz: &mut dyn Hafiz,
) -> Result<Option<TaqreerTarkeeb>, KhataNusus> {
    let Some((hadaf, aila)) = ayn_hadaf(jidhr) else {
        return Ok(None);
    };
    let siyaq = SiyaqTabaqa {
        jidhr,
        tanfidhi: None,
        aila,
    };
    let mifhas: &dyn Mifhas = match hadaf {
        HadafNusus::RpgMakerJs => &rpgmaker::MifhasRpgMaker,
        HadafNusus::RenPy => &renpy::MifhasRenPy,
        HadafNusus::GameMaker => &gamemaker::MifhasGameMaker,
        HadafNusus::Ghilaf => &electron::MifhasGhilaf,
        // `ayn_hadaf` never returns it: VX Ace is patched through its own
        // archive rewriter, which takes a Ruby payload this dispatcher has no
        // way to supply, and saying so is better than a route that half works.
        HadafNusus::VxAce => {
            return Err(KhataNusus::HimlMarfud {
                alia: "vx ace",
                sabab: "VX Ace is not dispatched from here; its script archive is rewritten \
                        through vxace::rakkib with a Ruby payload this entry point does not \
                        carry"
                    .to_owned(),
            });
        },
    };

    let (natija, imtina) = match hukm(&siyaq, hadaf, mifhas.adilla(&siyaq)?) {
        Ok(natija) => (Some(natija), None),
        Err(KhataNusus::TabaqaMajhula { sabab }) => (None, Some(sabab)),
        Err(khata) => return Err(khata),
    };
    let mut taqreer = TaqreerTarkeeb::min_hukm(
        hadaf,
        aila,
        natija.as_ref().map(|natija| natija.rutba),
        natija.as_ref().map_or(0, |natija| natija.thiqa),
    );
    if let Some(sabab) = imtina.clone() {
        taqreer.mulahazat.push(format!(
            "the shaping tier could not be established, so nothing here states one: {sabab}"
        ));
    }
    let matlub = || {
        natija.as_ref().ok_or_else(|| KhataNusus::TabaqaMajhula {
            sabab: imtina.clone().unwrap_or_default(),
        })
    };

    match hadaf {
        HadafNusus::RpgMakerJs => rakkib_rpgmaker(jidhr, mutarjim, hafiz, &mut taqreer)?,
        HadafNusus::RenPy => {
            rakkib_renpy(jidhr, mutarjim, mawarid, matlub()?, hafiz, &mut taqreer)?;
        },
        HadafNusus::GameMaker => rakkib_gamemaker(jidhr, mutarjim, hafiz, &mut taqreer)?,
        HadafNusus::Ghilaf => {
            rakkib_ghilaf(jidhr, mutarjim, mawarid, matlub()?, hafiz, &mut taqreer)?;
        },
        HadafNusus::VxAce => {},
    }
    Ok(Some(taqreer))
}

// ---------------------------------------------------------------------------
// RPG Maker MV and MZ
// ---------------------------------------------------------------------------

/// Splices the translations into `data/*.json` and the plugin parameters.
///
/// The extraction runs *here*, against the files as they are on disk at this
/// moment, rather than being carried from an earlier pass. That is not caution
/// for its own sake: every record carries the byte range of the literal it came
/// from, and a record measured against a file a launcher has since updated names
/// a range that is no longer that literal. Measuring and splicing in one breath
/// is what makes the offsets true.
fn rakkib_rpgmaker(
    jidhr: &Path,
    mutarjim: &dyn Mutarjim,
    hafiz: &mut dyn Hafiz,
    taqreer: &mut TaqreerTarkeeb,
) -> Result<(), KhataNusus> {
    let bunya = rpgmaker::BunyatMashru::iktashif(jidhr)?;
    let sijill = rpgmaker::istakhrij(&bunya)?;

    let mut tarjamat = rpgmaker::Tarjamat::new();
    let mut hurub_naqisa: usize = 0;
    for madkhal in &sijill.madakhil {
        // Raw first, clean second. A game's literal is what the compiler saw
        // when the string carries no markup, and the reader's own clean form is
        // what it saw when it does; trying the raw spelling first means a
        // translation keyed on a line that genuinely contains a backslash is
        // never shadowed by the lifted form of a different line.
        let Some(badeel) = mutarjim
            .tarjim(&madkhal.khaam)
            .or_else(|| mutarjim.tarjim(&madkhal.naqi))
        else {
            continue;
        };
        if rpgmaker::tahaqquq_hurub(madkhal, badeel).is_err() {
            hurub_naqisa = hurub_naqisa.saturating_add(1);
            continue;
        }
        let _ = tarjamat.insert(madkhal.huwiya.clone(), badeel.to_owned());
    }

    if hurub_naqisa > 0 {
        taqreer.mulahazat.push(format!(
            "{hurub_naqisa} translation(s) were left in the original language because they no \
             longer carry an escape code the engine substitutes at draw time — an icon, a \
             variable, an actor's name or the currency unit. Writing one of those would have \
             silently removed something the player was meant to see."
        ));
    }

    let natija = rpgmaker::rakkib(&bunya, &sijill, &tarjamat, hafiz)?;
    taqreer.malaffat = natija.malaffat;
    taqreer.nusus = natija.nusus;
    // The writer's own count, not this function's. Both walk the same records
    // and a string skipped above is a string the writer then finds no entry
    // for, so the two agree — and when they ever stop agreeing, the number the
    // report should carry is the one taken at the moment of writing.
    taqreer.matruka = natija.matruka;
    Ok(())
}

// ---------------------------------------------------------------------------
// Ren'Py
// ---------------------------------------------------------------------------

/// How deep the walk for `.rpy` and `.rpyc` sources goes under `game/`.
const UMQ_RENPY: usize = 16;

/// How many files that walk will look at.
const AQSA_MALAFAT_RENPY: usize = 64_000;

/// The largest Ren'Py source file this module will read.
const AQSA_MALAF_RENPY: u64 = 32 * 1024 * 1024;

/// Where Ren'Py keeps every translation, its own and Taarib's.
///
/// One string rather than [`renpy::MUJALLAD_TARJAMA`], which names the Arabic
/// one specifically: the rule below is about *all* translations, including the
/// game's own Japanese, and narrowing it to Arabic would let a game's shipped
/// French be read as English.
const MUJALLAD_TARAJIM: &str = "game/tl/";

/// Whether a game-relative source path is inside [`MUJALLAD_TARAJIM`].
///
/// Compared without regard to case because [`masarat_renpy`] reports the
/// directory as the depot actually spells it — a game shipping `Game/` produces
/// `Game/tl/…`, and a literal prefix test would answer "not a translation" for
/// every file under it. The exclusion below is load-bearing, so failing it open
/// is not a cosmetic miss: it would offer the game's own French, or Taarib's own
/// Arabic from a previous install, as a source string to translate from.
fn taht_tarajim(masdar: &str) -> bool {
    masdar
        .as_bytes()
        .get(..MUJALLAD_TARAJIM.len())
        .is_some_and(|badia| badia.eq_ignore_ascii_case(MUJALLAD_TARAJIM.as_bytes()))
}

/// Generates `game/tl/arabic/` and the settings the in-engine package reads.
///
/// Additive in every one of its writes: Ren'Py compiles anything under
/// `game/tl/<language>/` by itself, with no registration step of any kind, so
/// the whole patch is files the engine has never heard of. Uninstalling is a
/// delete and a game update that rewrites what it ships leaves these alone.
fn rakkib_renpy(
    jidhr: &Path,
    mutarjim: &dyn Mutarjim,
    mawarid: &Mawarid<'_>,
    natija: &crate::tabaqa::Hukm,
    hafiz: &mut dyn Hafiz,
    taqreer: &mut TaqreerTarkeeb,
) -> Result<(), KhataNusus> {
    let (sijillat, mutakhatta) = iltiqat_renpy(jidhr);

    let masar = renpy::Masar::min_rutba(natija.rutba);
    let mut idad = renpy::IdadRenPy::jadeed(
        masar,
        natija.rutba,
        natija.thiqa,
        mawarid.khatt_renpy.unwrap_or_default(),
    );
    idad.athar = natija.taqreer();
    if mawarid.khatt_renpy.is_none() {
        taqreer.mulahazat.push(
            "no Arabic font was supplied with this patch, so the generated file registers the \
             direction and the translation and leaves every font variable the game set alone. \
             Ren'Py assigns whatever name it is given to every gui font variable, and an empty \
             one is not a no-op."
                .to_owned(),
        );
    }

    if mutakhatta > 0 {
        taqreer.mulahazat.push(format!(
            "{mutakhatta} Ren'Py file(s) could not be read or walked and contributed nothing. \
             A compiled script this build cannot open is skipped rather than raised: refusing \
             the whole install over one of them would leave the game entirely untranslated."
        ));
    }

    taqreer
        .masarat
        .extend(renpy::iktub_idad(hafiz, jidhr, &idad)?);
    let (masar_mustalahat, adad_mustalahat, matruka_mustalahat) =
        renpy::iktub_mustalahat(hafiz, jidhr, &sijillat, mutarjim)?;
    let (masar_hiwar, adad_hiwar, matruka_hiwar) =
        renpy::iktub_hiwar(hafiz, jidhr, &sijillat, mutarjim)?;
    taqreer.masarat.push(masar_mustalahat);
    taqreer.masarat.push(masar_hiwar);

    taqreer.malaffat = taqreer.masarat.len();
    taqreer.nusus = adad_mustalahat.saturating_add(adad_hiwar);
    taqreer.matruka = taqreer
        .matruka
        .saturating_add(matruka_mustalahat.saturating_add(matruka_hiwar));
    Ok(())
}

/// Every translatable statement a Ren'Py game exposes on disk.
///
/// Two passes over `game/`, and the split is what makes the dialogue mechanism
/// usable at all. Ren'Py's statement identifier is a hash of the statement's
/// regenerated source and only the engine can compute one, so this crate never
/// invents one — it reads them out of a translation the game already ships,
/// where they are exact. Those translations live in `game/tl/<language>/`, and
/// the statements they identify live in `game/*.rpy`: a **different file**. A
/// pass that recovered identifiers per file would therefore recover every one of
/// them and attach none, which is silently an install with no dialogue in it.
///
/// So the first pass reads identifiers from every source in the tree, and the
/// second attaches them to the statements by source text. A file that cannot be
/// read is skipped and counted rather than raised — a Ren'Py game routinely
/// ships a `.rpyc` this build cannot walk, and refusing the whole install over
/// one of them would mean patching nothing at all.
///
/// Returns the statements and how many files were skipped.
#[must_use]
pub fn iltiqat_renpy(jidhr: &Path) -> (Vec<renpy::Sijill>, usize) {
    let mut sijillat: Vec<renpy::Sijill> = Vec::new();
    let mut mutakhatta: usize = 0;
    let mut bi_asl: BTreeMap<String, String> = BTreeMap::new();
    let masarat = masarat_renpy(jidhr);

    // Read once. Every source is needed by both passes, and reading a visual
    // novel's script tree twice is the kind of thing that turns a two-second
    // install into a twenty-second one.
    let mut masadir: Vec<(&str, String)> = Vec::new();
    let mut asmaa_rpy: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (masar, masdar, imtidad) in &masarat {
        if imtidad != "rpy" {
            continue;
        }
        let _ = asmaa_rpy.insert(masdar.as_str());
        match iqra_muhaddad(masar, AQSA_MALAF_RENPY).map(String::from_utf8) {
            Some(Ok(nass)) => masadir.push((masdar.as_str(), nass)),
            _ => mutakhatta = mutakhatta.saturating_add(1),
        }
    }

    // Pass one: every identifier the game's own translations carry, from the
    // whole tree, because the block that names one is never in the file that
    // holds the statement it names.
    for (_, nass) in &masadir {
        for (muarrif, asl) in renpy::muarrifat_min_tarjama(nass) {
            let _ = bi_asl.entry(asl).or_insert(muarrif);
        }
    }

    // Pass two: the statements themselves, with the identifiers attached.
    //
    // `game/tl/` is excluded, and that exclusion is load-bearing rather than
    // tidy. Everything under it is a *translation*, and the scanner is
    // line-oriented: it skips the `translate <language> <id>:` line and then
    // reads the indented statement under it like any other say statement. So a
    // game shipping a French translation would offer its French as English to
    // translate from, and — worse — a second pass over a game Taarib has already
    // patched would offer Taarib's own Arabic as a source string and key a patch
    // on it. Ren'Py's own meaning for the directory is exactly this rule.
    for (masdar, nass) in &masadir {
        if taht_tarajim(masdar) {
            continue;
        }
        for mut sijill in renpy::iltiqat_min_rpy(nass, masdar) {
            if sijill.muarrif.is_none() {
                sijill.muarrif = bi_asl.get(&sijill.asl).cloned();
            }
            sijillat.push(sijill);
        }
    }

    // Pass three: the compiled scripts, for a game that ships no source. Only
    // when the source is not beside it — the two hold the same statements, and
    // the source is the one that carries a line number a translator can be
    // pointed at.
    for (masar, masdar, imtidad) in &masarat {
        if imtidad != "rpyc" || taht_tarajim(masdar) {
            continue;
        }
        if asmaa_rpy.contains(masdar.strip_suffix('c').unwrap_or(masdar)) {
            continue;
        }
        let Some(bayt) = iqra_muhaddad(masar, AQSA_MALAF_RENPY) else {
            mutakhatta = mutakhatta.saturating_add(1);
            continue;
        };
        match renpy::iltiqat_min_rpyc(&bayt, masdar) {
            Ok(mustakhraj) => sijillat.extend(mustakhraj),
            Err(_) => mutakhatta = mutakhatta.saturating_add(1),
        }
    }
    (sijillat, mutakhatta)
}

/// Every `.rpy` and `.rpyc` under `game/`, as (absolute, game-relative,
/// extension).
///
/// The walk root is resolved through [`masar_bila_hala`]: a depot spelling it
/// `Game/` would otherwise be walked as a directory that does not exist, the
/// walk would yield nothing, and the install would write a translation of the
/// empty set and report success.
fn masarat_renpy(jidhr: &Path) -> Vec<(PathBuf, String, String)> {
    let mut masarat = Vec::new();
    for madkhal in walkdir::WalkDir::new(masar_bila_hala(jidhr, "game"))
        .max_depth(UMQ_RENPY)
        .follow_links(false)
        .into_iter()
        .take(AQSA_MALAFAT_RENPY)
        .filter_map(Result::ok)
    {
        if !madkhal.file_type().is_file() {
            continue;
        }
        let masar = madkhal.path();
        let imtidad = masar
            .extension()
            .and_then(|juz| juz.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if imtidad != "rpy" && imtidad != "rpyc" {
            continue;
        }
        let masdar = masar
            .strip_prefix(jidhr)
            .unwrap_or(masar)
            .to_string_lossy()
            .replace('\\', "/");
        masarat.push((masar.to_path_buf(), masdar, imtidad));
    }
    masarat.sort();
    masarat
}

/// Reads a file, refusing one above a ceiling before a byte is reserved.
fn iqra_muhaddad(masar: &Path, saqf: u64) -> Option<Vec<u8>> {
    let bayanat = fs::metadata(masar).ok()?;
    if !bayanat.is_file() || bayanat.len() > saqf {
        return None;
    }
    fs::read(masar).ok()
}

// ---------------------------------------------------------------------------
// GameMaker
// ---------------------------------------------------------------------------

/// Rewrites the string pool of a `data.win` and writes the container back.
///
/// Every replacement goes through [`gamemaker::MuhawwilGameMaker::badil_nass`],
/// which either rewrites a record in place or appends it and repoints every
/// reference the census found; the container is produced and fully verified in
/// memory by [`gamemaker::MuhawwilGameMaker::uktub`] before the guard is handed
/// a byte. A `data.win` *is* the game, so there is no partial success here to
/// salvage and none is attempted.
fn rakkib_gamemaker(
    jidhr: &Path,
    mutarjim: &dyn Mutarjim,
    hafiz: &mut dyn Hafiz,
    taqreer: &mut TaqreerTarkeeb,
) -> Result<(), KhataNusus> {
    let Some(masar) = gamemaker::MifhasGameMaker::hawiya(jidhr) else {
        return Ok(());
    };
    let bayt = fs::read(&masar).map_err(|sabab| KhataNusus::KhataMalaf {
        masar: masar.clone(),
        sabab,
    })?;
    let hawiya = gamemaker::HawiyatGameMaker::min_bayt(bayt, &masar)?;

    // Collected before the converter borrows the container: the pool is read
    // through `hawiya` and `MuhawwilGameMaker::jadeed` takes it by value.
    let mut badail: Vec<(u32, String)> = Vec::new();
    for (fahras, madkhal) in hawiya.hawd().madakhil().iter().enumerate() {
        let Ok(fahras) = u32::try_from(fahras) else {
            break;
        };
        match mutarjim.tarjim(&madkhal.nass) {
            Some(badeel) => badail.push((fahras, badeel.to_owned())),
            None => taqreer.matruka = taqreer.matruka.saturating_add(1),
        }
    }
    if badail.is_empty() {
        taqreer.mulahazat.push(
            "this patch carries no translation for any string in the container's pool, so it \
             was left exactly as it shipped"
                .to_owned(),
        );
        return Ok(());
    }

    let mut muhawwil = gamemaker::MuhawwilGameMaker::jadeed(hawiya);
    for (fahras, badeel) in &badail {
        muhawwil.badil_nass(*fahras, badeel)?;
    }
    muhawwil.uktub(hafiz, &masar)?;

    taqreer.malaffat = 1;
    taqreer.nusus = badail.len();
    taqreer.masarat.push(masar);
    Ok(())
}

// ---------------------------------------------------------------------------
// Electron and NW.js
// ---------------------------------------------------------------------------

/// Injects the translation table and the renderer runtime into `app.asar`.
///
/// The Electron half is the one adapter here whose delivery is *not* a rewrite
/// of the game's own text: Chromium already shapes Arabic correctly, so the
/// patch supplies a table the injected runtime matches DOM text against. That
/// makes the runtime load-bearing — a table with nothing to read it changes
/// nothing on screen — and the absence of the compiled runtime is therefore a
/// refusal rather than a partial write.
fn rakkib_ghilaf(
    jidhr: &Path,
    mutarjim: &dyn Mutarjim,
    mawarid: &Mawarid<'_>,
    natija: &crate::tabaqa::Hukm,
    hafiz: &mut dyn Hafiz,
    taqreer: &mut TaqreerTarkeeb,
) -> Result<(), KhataNusus> {
    let Some(masar) = electron::hawiya(jidhr) else {
        return Ok(());
    };
    let Some(tashghil) = mawarid.tashghil_ghilaf else {
        return Err(KhataNusus::HimlMarfud {
            alia: "asar runtime",
            sabab: "the compiled Electron renderer runtime was not supplied. It is built from \
                    adapters-script/electron/taarib.ts and staged into the component store as \
                    mulhaq/electron/taarib.js; without it the payload would be a translation \
                    table with nothing to read it, which changes nothing on screen."
                .to_owned(),
        });
    };

    let mut hawiya = electron::HawiyatAsar::min_masar(&masar)?;
    let hasad = electron::istakhrij(&hawiya)?;

    let mut jadwal: BTreeMap<String, String> = BTreeMap::new();
    for nass in &hasad.nusus {
        match mutarjim.tarjim(&nass.nass) {
            Some(badeel) => {
                let _ = jadwal.insert(nass.nass.clone(), badeel.to_owned());
            },
            None => taqreer.matruka = taqreer.matruka.saturating_add(1),
        }
    }
    for (masdar, sabab) in &hasad.matruka {
        taqreer
            .mulahazat
            .push(format!("{masdar} was not read for strings: {sabab}"));
    }

    let adad = jadwal.len();
    // No font block. The payload carries a face inline as a base64 data URL and
    // this entry point is handed no font file, so nothing is registered rather
    // than a name pointing at bytes that are not there. Chromium shapes Arabic
    // with whatever face the application already resolves, which is why the
    // Electron adapter is the one of the four that reads correctly without one.
    let himl = electron::HimlGhilaf {
        rutba: natija.rutba,
        jadwal,
        khatt: None,
        tashghil: tashghil.to_owned(),
    };

    let natija_tarkeeb = electron::rakkib(&mut hawiya, &himl)?;
    hawiya.uktub(hafiz, &masar)?;

    taqreer.malaffat = 1;
    taqreer.nusus = adad;
    taqreer.masarat.push(masar);
    taqreer.mulahazat.push(format!(
        "registered through the application's own {} entry, whose original value {} is \
         recorded in package.json so an uninstall puts it back",
        natija_tarkeeb.alia, natija_tarkeeb.asl
    ));
    Ok(())
}
