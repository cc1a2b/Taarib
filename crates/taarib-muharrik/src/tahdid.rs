//! التحديد — one answer out of every detector's observations, with the evidence
//! kept and the disagreements named rather than averaged away.
//!
//! [`crate::fahs`] guarantees that no detector concludes anything: each one
//! reports what it saw and stops. This module is where a conclusion is drawn,
//! and it is the only place in the crate allowed to hold an opinion. It answers
//! three questions — which engine family the game belongs to ([`hall`]), how
//! much that answer is worth ([`thiqa`]), and what contradicted it
//! ([`taarud`]).
//!
//! ## The weight scale
//!
//! Every observation carries a weight from 0 to 100, recorded by the detector
//! through [`HasilatFahs::sajjil`](crate::fahs::HasilatFahs::sajjil). The scale
//! is not a feeling about how sure a detector is. It is a claim about how many
//! engines could have produced the thing that was seen:
//!
//! - **90–100** — only one engine ever ships this. `GameAssembly.dll`, a
//!   `UnityPlayer` import, `il2cpp_data/Metadata/global-metadata.dat`,
//!   `renpy/__init__.py`, a GameMaker `GEN8` chunk, a Godot PCK magic.
//! - **60–89** — characteristic of one engine, and occasionally shipped beside
//!   another. `Engine/Binaries/`, `resources/app.asar`, a `.pak` footer.
//! - **40–59** — several engines ship it. `package.json`, a `d3d11` import, a
//!   `mono-2.0-bdwgc` import.
//! - **30–39** — consistent with this engine and with three others. A `www/`
//!   directory; a `lib/` directory holding a Python runtime.
//! - **0–29** — a name that merely suggests something. An executable called
//!   `nw.exe`; a directory called `Data`.
//!
//! [`HADD_ADNA_LIL_TABANNI`] sits at the top of that last band. A suggestion
//! promoted to a conclusion sends an adapter into a game it cannot handle, and
//! that failure happens inside the player's game rather than in a log somebody
//! reads afterwards.
//!
//! ## The confidence formula
//!
//! ```text
//! thiqa = clamp(asas + taadud - uqubat, 0, SAQF_THIQA)
//! ```
//!
//! **`asas`** is the strongest single observation supporting the adopted
//! family: the floor of belief. One 95-weight marker is very nearly the whole
//! answer on its own, and the formula says so rather than diluting it with an
//! average over weaker observations that agree with it.
//!
//! **`taadud`** is corroboration, counted per *detector* and never per
//! observation. Four sightings of the same `*_Data` directory by one detector
//! are one fact; the same family named by the directory walker, the binary
//! reader and the container reader is three independent facts. Every supporting
//! detector after the strongest contributes by its own band —
//! [`musahamat_shahid`] — capped at [`SAQF_TAADUD`].
//!
//! So a single 95-weight observation resolves to 95, while three 90-weight
//! observations from three different detectors resolve to 98. That ordering is
//! deliberate: independence is worth more than a few points of strength,
//! because the failure mode of one detector is a bug in one detector, while
//! three detectors agreeing is a game that really is what they say it is.
//!
//! **`uqubat`** is contradiction. Every *other* family any detector named costs
//! points by its own band — [`thaman_munafis`] — capped at [`SAQF_TAARUD`]. A
//! rival at the top of the scale takes a quarter of the range, so two strong
//! contradictory claims land in the low seventies: low enough that the
//! interface shows the doubt, high enough that an adapter still runs.
//!
//! The penalty is never charged for a packaging shell. Electron around a Godot
//! build is not two claims about one engine; it is one claim about the engine
//! and one claim about the box it arrived in, and both of them are true.
//!
//! ## An unknown engine is an answer
//!
//! [`AilatMuharrik::Majhul`] uses the same formula with a different `asas`.
//! Instead of the strongest supporting observation, which does not exist, the
//! base is *how much was examined*. A directory holding one executable and
//! nothing else is unknown at low confidence — almost nothing was looked at, so
//! almost nothing is known. A directory where an executable was parsed, a
//! graphics API was identified, several detectors ran and reported, and not one
//! engine marker turned up anywhere, is unknown at much higher confidence: the
//! sweep was wide and it came back empty, and that is a real result.
//!
//! It is capped at [`SAQF_MAJHUL`] and never reaches certainty, because
//! "unknown" is a statement about this build of the probe rather than about the
//! game. `isdar_fahs` exists precisely so a better probe re-examines it.
//!
//! ## Where a version came from outranks who read it
//!
//! The weight scale above answers "how many engines could have produced this",
//! which is the right question for *which engine* and the wrong one for *which
//! version*. Two detectors can be equally certain the game is Unity and read
//! two different version strings out of it, and the one to believe is not the
//! one whose strongest observation scored higher.
//!
//! [`isdar_min`] ranks the readings in three tiers, and the order of the tiers
//! is the rule:
//!
//! 1. **An exact version out of a record the engine wrote** — a version string
//!    at the offset a format puts it at. The build that made the file is the
//!    build the string names.
//! 2. **An exact version swept out of a binary.** A version-shaped run of bytes
//!    matches whatever is on disk, and what is on disk in a shipped engine
//!    runtime includes constants its vendor compiled in years earlier. This is
//!    why a Unity 6 game and a Unity 2022 game both reported themselves as an
//!    alpha of Unity 2018: `2018.3.0a1` is a floor constant in the serializer's
//!    error-message table of every player since 2018, and a sweep with no
//!    anchor meets it before it meets the build's own stamp.
//! 3. **A range derived from a container's format version.** Never a build, and
//!    at its widest several releases. `dalail::unreal` maps a `.pak` or `.utoc`
//!    format version onto an engine range and sets `IsdarMuharrik::mushtaqq` on
//!    what it returns. The rule this module enforces is *a derived range must
//!    never outrank an exact version, whatever the weight of the observation
//!    that carried it*, and this is the only place that can enforce it, because
//!    this is the only place that sees two detectors' answers at once.
//!
//! Tiers 1 and 2 are separated by [`hujjiyat_masdar`], stated over
//! [`NawDaleel`] so it holds for every detector and every engine rather than
//! for the pair that exposed it. Tier 3 is separated by [`mushtaqq`], and it is
//! ordered ahead of the other two: a pak footer is a container header, which
//! makes it a record by [`NawDaleel`], and no source kind can tell a range
//! *inferred* from a container apart from a version *read* out of one. Only the
//! code that did the inferring knows, so it says so on the value it returns.
//!
//! Detector strength breaks ties inside a tier and never across one, so
//! registration order can no longer decide a version.

use std::cmp::Reverse;

use taarib_mustalahat::muharrik::{
    AilatMuharrik, Daleel, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, Muharrik, NawDaleel,
    WajihaRusum,
};
use taarib_usus::manassa::Mimariya;

use crate::fahs::{HasilatFahs, JamiHasilat};

/// The weight below which a claim is not adopted, whatever else is true.
///
/// The bottom band of the scale is "a name that merely suggests something", and
/// a suggestion is not an identification. A game whose only evidence is a
/// directory called `Data` resolves to [`AilatMuharrik::Majhul`] with the
/// suggestion kept in the evidence, which is honest, instead of resolving to
/// Unity and sending a BepInEx payload into something that is not Unity.
pub const HADD_ADNA_LIL_TABANNI: u8 = 30;

/// The most corroboration can add, however many detectors agree.
///
/// Twelve points is the distance between "one strong marker" and "as sure as
/// this probe gets". Beyond three agreeing detectors the additional agreement
/// is nearly free — detectors that look at the same directory tend to succeed
/// together — so the cap stops a game with many small markers from outranking a
/// game with one conclusive one by sheer count.
pub const SAQF_TAADUD: u16 = 12;

/// The most contradiction can take away.
///
/// Thirty-five points takes a 95-weight identification down to 60, which is the
/// bottom of the range where the interface still presents an answer. Past that
/// the honest report is not "this engine, barely" but a conflict, and
/// [`taarud`] is what says so.
pub const SAQF_TAARUD: u16 = 35;

/// The highest confidence any identification is allowed to reach.
///
/// Not 100, ever. A hundred would mean no future probe could change this
/// answer, and the whole point of stamping a report with `isdar_fahs` is that a
/// later probe can and does.
pub const SAQF_THIQA: u16 = 99;

/// Where confidence in "unknown" starts, before anything was examined.
pub const ASAS_MAJHUL: u16 = 15;

/// Added to an unknown result when some detector actually parsed an executable.
///
/// Reading a binary header is the difference between a directory nobody could
/// make sense of and a directory containing a program that imports nothing any
/// engine imports.
pub const ZIYADAT_TANFIDHI: u16 = 10;

/// Added to an unknown result when a graphics API was identified.
///
/// Worth more than the executable on its own: it means the binary was read
/// deeply enough to enumerate what it links against, and that is exactly the
/// place a Unity, Unreal or Godot marker would have appeared if there were one.
pub const ZIYADAT_RUSUM: u16 = 15;

/// Added to an unknown result per observation of any kind, up to
/// [`SAQF_ITTISA`].
pub const ZIYADAT_ITTISA: u16 = 2;

/// The most breadth of evidence can add to an unknown result.
pub const SAQF_ITTISA: u16 = 20;

/// Added to an unknown result per detector that reported anything at all.
pub const ZIYADAT_FAHIS: u16 = 5;

/// The most detector participation can add to an unknown result.
pub const SAQF_FUHUS: u16 = 25;

/// The highest confidence "unknown" is allowed to reach.
pub const SAQF_MAJHUL: u16 = 85;

/// How strong a rival claim has to be before the conflict is read as a second
/// engine's files sitting beside the game's, rather than as a misreading.
///
/// Seventy is the floor of the "characteristic of one engine" band. Below it a
/// rival is a stray file; at or above it, a whole engine layout is present and
/// the question becomes which of the two the player actually launches.
pub const HADD_QUWWAT_MUNAFIS: u8 = 70;

/// The prefix on the note resolution writes when it kept a packaging shell.
///
/// Resolution's conclusions have to survive into [`Muharrik`] and back out of
/// the store, because the capability report is built from a `Muharrik` and
/// nothing else. A note in the evidence trail is the only channel that does
/// that, so the note has a documented, stable prefix and
/// [`maghlufa`] reads it.
pub const WASF_GHILAF: &str = "packaging shell retained: ";

/// The prefix on every note resolution writes about a claim it did not adopt.
///
/// Same channel, same reason as [`WASF_GHILAF`]: the losing evidence is kept
/// verbatim in `dalail`, and this note is what tells a later reader that the
/// evidence lost rather than that it was never considered.
pub const WASF_TAARUD: &str = "not adopted: ";

/// The prefix on the note resolution writes about a claim that was too weak to
/// be adopted at all.
///
/// Distinct from [`WASF_TAARUD`] on purpose. "Unreal lost to Unity" and "a
/// suggestion of Unity was seen and nothing was concluded from it" are
/// different facts, they produce different sentences in the capability report,
/// and one prefix for both would make the report claim a conflict that never
/// happened.
pub const WASF_DUNA_HADD: &str = "below the adoption floor: ";

/// What one corroborating detector adds, by the band its strongest observation
/// falls in.
///
/// Banded rather than proportional so that the number is explainable: it comes
/// from the same five bands the weight scale is written in, and a maintainer
/// reading a confidence value can name which band produced each point of it.
#[must_use]
pub const fn musahamat_shahid(wazn: u8) -> u16 {
    match wazn {
        90..=u8::MAX => 4,
        70..=89 => 3,
        40..=69 => 2,
        30..=39 => 1,
        _ => 0,
    }
}

/// What one rival family costs, by the band its strongest claim falls in.
///
/// A 90-band rival costs 25 — a quarter of the whole scale — because a file
/// only one engine ever ships was found for an engine we are not choosing, and
/// that is either a bundled tool or a mistake, and both deserve visible doubt.
/// A rival in the bottom band still costs 2, so that a game with a scattering
/// of weak cross-engine hints never reports as certain.
#[must_use]
pub const fn thaman_munafis(wazn: u8) -> u16 {
    match wazn {
        90..=u8::MAX => 25,
        70..=89 => 18,
        40..=69 => 10,
        30..=39 => 5,
        _ => 2,
    }
}

/// How much a version reading is worth by *where it was read*.
///
/// Two bands, and the line between them is whether the engine itself wrote the
/// thing that was read:
///
/// - **1 — a record.** [`NawDaleel::BayanatMudmaja`] is a version string or
///   structure the engine embedded in the game's own data;
///   [`NawDaleel::TarwisatHawiya`] is the header of a container the engine
///   wrote. Both are read at the offset the format puts them at, so the build
///   that produced the file is the build the string names.
/// - **0 — circumstance.** [`NawDaleel::TawqiThunai`] is a byte pattern found
///   by sweeping a binary: it matches whatever happens to be on disk, including
///   a constant the engine's vendor compiled into its runtime years before this
///   game existed. [`NawDaleel::BinyatMujallad`] is a directory or file name,
///   which dates a build only by implication.
///
/// Deliberately two bands and not four. The distinction that decides a wrong
/// answer is record against sweep; ranking the other pair against each other
/// would be inventing an ordering no observed failure asks for.
#[must_use]
pub const fn hujjiyat_masdar(naw: NawDaleel) -> u8 {
    match naw {
        NawDaleel::BayanatMudmaja | NawDaleel::TarwisatHawiya => 1,
        NawDaleel::TawqiThunai | NawDaleel::BinyatMujallad => 0,
    }
}

/// Whether a version is an inference over a range rather than a build's own
/// statement about itself.
///
/// Read from the flag the detector that made the inference set. A derived range
/// loses to every exact version, in either band and at any weight:
/// `UE 4.3-4.15` spans thirteen releases, and an adapter told the game is one
/// of thirteen is an adapter told nothing it can gate a decision on.
///
/// This used to read the word "derived" out of the raw string, because the flag
/// did not exist. That worked and was still wrong: the raw string is prose a
/// user reads, and rewording one sentence would have silently un-demoted every
/// range in the product with nothing failing to say so.
#[must_use]
pub const fn mushtaqq(isdar: &IsdarMuharrik) -> bool {
    isdar.mushtaqq
}

/// Two or more detectors named different engines, and what was made of it.
///
/// Kept as a value rather than folded into a confidence number, because the
/// three situations that produce it are genuinely different and averaging them
/// turns all three into one confident wrong answer:
///
/// 1. **A packaging shell.** Electron or `NW.js` around a real engine. Both
///    readings are correct at once, and installation needs both of them: the
///    inner engine says what to patch, the shell says how to unpack and repack
///    it.
/// 2. **A bundled second engine.** A Unity launcher shipped beside an Unreal
///    game, an installer left in place, a toolchain directory nobody cleaned
///    out. Two complete layouts, one game.
/// 3. **A misidentification.** One of the two detectors is wrong, and this
///    build cannot tell which. Confidence drops and the losing evidence stays
///    where a maintainer can read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaarudDalail {
    /// The adopted family first, then every claim that was not adopted, each
    /// with the strongest weight behind it. When nothing cleared
    /// [`HADD_ADNA_LIL_TABANNI`], the list is simply in weight order and the
    /// answer was [`AilatMuharrik::Majhul`].
    pub ailat: Vec<(AilatMuharrik, u8)>,
    /// Which of the three situations this is and how it was resolved, in
    /// English, for the diagnostics bundle. The sentence a *user* reads is
    /// written by [`crate::imkaniyat`] in both languages; this one is written
    /// for whoever has to fix a wrong identification.
    pub sabab: String,
}

/// Confidence that `aila` is this game's engine, on the same 0-100 scale the
/// interface shows.
///
/// Defined for every family, not only the one [`hall`] adopted, so that the
/// diagnostics screen can show what the runners-up were worth. A family no
/// detector named scores 0; [`AilatMuharrik::Majhul`] is scored by how much was
/// examined rather than by what was found. The formula and every constant in it
/// are documented at the top of this module.
#[must_use]
pub fn thiqa(jami: &JamiHasilat, aila: AilatMuharrik) -> u8 {
    let ghilaf = ghilaf_hawla_dakhil(jami);
    let daimun = daimu_aila(jami, aila);

    let (asas, taadud) = if aila == AilatMuharrik::Majhul {
        (asas_majhul(jami), 0)
    } else {
        let Some(aqwa) = daimun.first() else { return 0 };
        let musanidun: u16 =
            daimun.iter().skip(1).map(|wazn| musahamat_shahid(*wazn)).sum::<u16>();
        (u16::from(*aqwa), musanidun.min(SAQF_TAADUD))
    };

    // A shell is not a rival to the engine it wraps, so it is charged to
    // nothing when the inner engine is the family being scored.
    let yatajahal_ghilaf = aila != AilatMuharrik::Electron && ghilaf.is_some();
    let uqubat: u16 = jami
        .ailat()
        .into_iter()
        .filter(|(munafis, _)| *munafis != aila && *munafis != AilatMuharrik::Majhul)
        .filter(|(munafis, _)| !(yatajahal_ghilaf && *munafis == AilatMuharrik::Electron))
        .map(|(_, wazn)| thaman_munafis(wazn))
        .sum::<u16>()
        .min(SAQF_TAARUD);

    let saqf = if aila == AilatMuharrik::Majhul { SAQF_MAJHUL } else { SAQF_THIQA };
    let qeema = asas.saturating_add(taadud).saturating_sub(uqubat).min(saqf);
    u8::try_from(qeema).unwrap_or(100)
}

/// Reports a disagreement between detectors, or `None` when there was none.
///
/// `None` is the ordinary answer. One family named, or no family named at all,
/// is not a conflict — an empty result is a game this build does not recognise,
/// and that is [`AilatMuharrik::Majhul`]'s job rather than this function's.
#[must_use]
pub fn taarud(jami: &JamiHasilat) -> Option<TaarudDalail> {
    let murashshahun = murashshahun(jami);
    if murashshahun.len() < 2 {
        return None;
    }

    let ghilaf = ghilaf_hawla_dakhil(jami);
    let faiz = ikhtiyar(jami, &murashshahun, ghilaf);

    let mut ailat: Vec<(AilatMuharrik, u8)> = Vec::with_capacity(murashshahun.len());
    let mudkhal_faiz =
        faiz.and_then(|mukhtar| murashshahun.iter().find(|(aila, _)| *aila == mukhtar));
    if let Some(mudkhal) = mudkhal_faiz {
        ailat.push(*mudkhal);
    }
    ailat.extend(murashshahun.iter().filter(|(aila, _)| Some(*aila) != faiz).copied());

    Some(TaarudDalail { sabab: sabab_taarud(jami, &murashshahun, faiz, ghilaf), ailat })
}

/// Whether this identification was made over a packaging shell that resolution
/// kept.
///
/// True for a Godot or RPG Maker game inside Electron or `NW.js`, false for a
/// game that *is* an Electron application. The capability report needs the
/// distinction because a wrapped game is repacked on installation and an
/// unwrapped one is not.
#[must_use]
pub fn maghlufa(muharrik: &Muharrik) -> bool {
    muharrik.dalail.iter().any(|daleel| daleel.wasf.starts_with(WASF_GHILAF))
}

/// Whether resolution had to choose between claims about different engines.
///
/// Read from the retained evidence rather than recomputed, so it stays true of
/// a report loaded back out of the store months later.
#[must_use]
pub fn mutaarid(muharrik: &Muharrik) -> bool {
    muharrik.dalail.iter().any(|daleel| daleel.wasf.starts_with(WASF_TAARUD))
}

/// Every family a detector named, strongest first, with `Majhul` removed.
///
/// `Majhul` is what resolution *concludes*; a detector that named it would be
/// concluding, which [`crate::fahs`] forbids, so it is dropped here rather than
/// allowed to compete.
fn murashshahun(jami: &JamiHasilat) -> Vec<(AilatMuharrik, u8)> {
    jami.ailat().into_iter().filter(|(aila, _)| *aila != AilatMuharrik::Majhul).collect()
}

/// The strongest observation of each detector that named `aila`, strongest
/// first.
///
/// One entry per detector, which is what makes corroboration a count of
/// independent sources rather than a count of files.
fn daimu_aila(jami: &JamiHasilat, aila: AilatMuharrik) -> Vec<u8> {
    let mut awzan: Vec<u8> = jami
        .hasilat
        .iter()
        .filter(|(_, hasila)| hasila.aila == Some(aila))
        .map(|(_, hasila)| hasila.aqwa())
        .collect();
    awzan.sort_unstable_by(|awwal, thani| thani.cmp(awwal));
    awzan
}

/// The total weight of everything the detectors that named `aila` observed.
///
/// The volume measure, used only to separate a game from a tool shipped beside
/// it: a bundled launcher leaves two or three markers, and the game it was
/// bundled with leaves a directory tree of them.
fn hajm_daim(jami: &JamiHasilat, aila: AilatMuharrik) -> u32 {
    jami.hasilat
        .iter()
        .filter(|(_, hasila)| hasila.aila == Some(aila))
        .flat_map(|(_, hasila)| hasila.dalail.iter())
        .map(|daleel| u32::from(daleel.wazn))
        .sum()
}

/// How much was examined, for a game no detector could name.
fn asas_majhul(jami: &JamiHasilat) -> u16 {
    let mut asas = ASAS_MAJHUL;
    if jami.hasilat.iter().any(|(_, hasila)| hasila.mimariya.is_some()) {
        asas = asas.saturating_add(ZIYADAT_TANFIDHI);
    }
    if jami.rusum().iter().any(|wajiha| *wajiha != WajihaRusum::Majhula) {
        asas = asas.saturating_add(ZIYADAT_RUSUM);
    }

    let adad = u16::try_from(jami.dalail().len()).unwrap_or(u16::MAX);
    asas = asas.saturating_add(adad.saturating_mul(ZIYADAT_ITTISA).min(SAQF_ITTISA));

    let fuhus = u16::try_from(jami.hasilat.len()).unwrap_or(u16::MAX);
    asas.saturating_add(fuhus.saturating_mul(ZIYADAT_FAHIS).min(SAQF_FUHUS))
}

/// The shell's strongest weight, when a browser shell was found wrapping an
/// engine that clears [`HADD_ADNA_LIL_TABANNI`].
///
/// `None` for a game that genuinely *is* an Electron application, and `None`
/// when the only other claim is a stray suggestion — a weak marker must not be
/// able to demote a real Electron game to a wrapper for something that is not
/// there.
fn ghilaf_hawla_dakhil(jami: &JamiHasilat) -> Option<u8> {
    let murashshahun = murashshahun(jami);
    let (_, wazn) = murashshahun.iter().find(|(aila, _)| *aila == AilatMuharrik::Electron)?;
    let lahu_dakhil = murashshahun
        .iter()
        .any(|(aila, quwwa)| {
            *aila != AilatMuharrik::Electron && *quwwa >= HADD_ADNA_LIL_TABANNI
        });
    lahu_dakhil.then_some(*wazn)
}

/// Declaration order, used as the last tie-break so that identical evidence
/// always produces an identical answer.
///
/// A resolution that depended on the order detectors happened to run in would
/// be a resolution that changed when somebody reordered a list.
const fn rutba(aila: AilatMuharrik) -> u8 {
    match aila {
        AilatMuharrik::Unity => 0,
        AilatMuharrik::Unreal => 1,
        AilatMuharrik::Godot => 2,
        AilatMuharrik::RpgMakerMv => 3,
        AilatMuharrik::RpgMakerMz => 4,
        AilatMuharrik::RpgMakerVxAce => 5,
        AilatMuharrik::Renpy => 6,
        AilatMuharrik::GameMaker => 7,
        AilatMuharrik::Electron => 8,
        AilatMuharrik::Bio4 => 9,
        AilatMuharrik::Majhul => 10,
    }
}

/// Picks the family, or `None` when nothing cleared the adoption floor.
///
/// The order of the tests is the rule, and it is the rule because each test
/// answers a question the next one cannot:
///
/// 1. **A shell never wins over what it wraps.** `resources/app.asar` is a
///    90-band marker and would beat almost any inner engine on weight alone,
///    which is exactly why this is decided before weights are compared.
/// 2. **Strongest single observation.** A file only one engine ships outranks
///    any quantity of files several engines share.
/// 3. **Independent detectors.** On equal strength, the family three detectors
///    reached from three different directions is the game.
/// 4. **Total supporting weight.** This is the bundled-tool test: a launcher
///    shipped beside a game leaves two or three markers, and the game leaves a
///    tree of them.
/// 5. **Declaration order**, so the answer is deterministic.
fn ikhtiyar(
    jami: &JamiHasilat,
    murashshahun: &[(AilatMuharrik, u8)],
    ghilaf: Option<u8>,
) -> Option<AilatMuharrik> {
    murashshahun
        .iter()
        .filter(|(_, wazn)| *wazn >= HADD_ADNA_LIL_TABANNI)
        .filter(|(aila, _)| !(ghilaf.is_some() && *aila == AilatMuharrik::Electron))
        .max_by_key(|(aila, wazn)| {
            (
                *wazn,
                daimu_aila(jami, *aila).len(),
                hajm_daim(jami, *aila),
                Reverse(rutba(*aila)),
            )
        })
        .map(|(aila, _)| *aila)
}

/// Names which of the three situations a disagreement is, and how it was
/// settled. English, for the diagnostics bundle.
fn sabab_taarud(
    jami: &JamiHasilat,
    murashshahun: &[(AilatMuharrik, u8)],
    faiz: Option<AilatMuharrik>,
    ghilaf: Option<u8>,
) -> String {
    let Some(mukhtar) = faiz else {
        let aqwa = murashshahun.iter().map(|(_, wazn)| *wazn).max().unwrap_or(0);
        return format!(
            "nothing adopted: the strongest claim for any engine is {aqwa}, below the \
             floor of {HADD_ADNA_LIL_TABANNI} at which a claim becomes a conclusion. The \
             game is reported as unknown and every claim is kept as evidence."
        );
    };

    if let Some(wazn_ghilaf) = ghilaf {
        return format!(
            "packaging shell: a browser shell was found at weight {wazn_ghilaf} together \
             with {}. The inner engine is the engine and the shell is how the game is \
             packaged; both are kept, because installation needs the shell to unpack and \
             repack and the inner engine to patch.",
            mukhtar.ism()
        );
    }

    let khasir = murashshahun.iter().find(|(aila, _)| *aila != mukhtar);
    let Some((aila_khasir, wazn_khasir)) = khasir.copied() else {
        return format!("{} was the only claim left standing.", mukhtar.ism());
    };

    let hajm_faiz = hajm_daim(jami, mukhtar);
    let hajm_khasir = hajm_daim(jami, aila_khasir);
    let wazn_faiz = murashshahun
        .iter()
        .find(|(aila, _)| *aila == mukhtar)
        .map_or(0, |(_, wazn)| *wazn);

    if wazn_khasir >= HADD_QUWWAT_MUNAFIS
        && hajm_faiz.saturating_mul(2) >= hajm_khasir.saturating_mul(3)
    {
        return format!(
            "bundled second engine: {} is present with strong markers (weight \
             {wazn_khasir}) beside {}, which carries {hajm_faiz} points of supporting \
             evidence against {hajm_khasir}. The smaller layout is read as a tool or \
             launcher shipped with the game, not as the game.",
            aila_khasir.ism(),
            mukhtar.ism()
        );
    }

    format!(
        "disagreement: {} at weight {wazn_faiz} and {} at weight {wazn_khasir} are \
         comparable, and this build cannot tell which detector is wrong. The stronger was \
         adopted, confidence was reduced by the strength of the claim that lost, and the \
         losing evidence is kept verbatim.",
        mukhtar.ism(),
        aila_khasir.ism()
    )
}

/// Combines every detector's observations into one identification.
///
/// Infallible by construction. There is no input for which this cannot answer,
/// because [`AilatMuharrik::Majhul`] is a real answer and not a failure: a game
/// nobody recognised gets an identification that says so, with a confidence
/// value describing how thoroughly it was looked at, and every observation kept
/// underneath it.
///
/// What survives into the result, and from where:
///
/// - **Version and backend** come only from detectors that named the adopted
///   family. A shell's own version — Electron 28, `NW.js` 0.83 — must never be
///   reported as the game engine's version, and restricting the source is what
///   guarantees it cannot be. Among those detectors the version is chosen by
///   where it was read rather than by who read it: see [`isdar_min`] and
///   [`hujjiyat_masdar`].
/// - **Text systems** come from the detectors that named the adopted family,
///   from detectors that named no family at all, and from a shell that was
///   kept. They do not come from a rejected rival: a rival's text systems
///   belong to the engine that was rejected.
/// - **Graphics APIs** come from everywhere, because a linked graphics library
///   is a fact about the binary and belongs to no engine's claim.
#[must_use]
pub fn hall(jami: &JamiHasilat) -> Muharrik {
    let murashshahun = murashshahun(jami);
    let ghilaf = ghilaf_hawla_dakhil(jami);
    let mukhtar = ikhtiyar(jami, &murashshahun, ghilaf);
    let aila = mukhtar.unwrap_or(AilatMuharrik::Majhul);

    let mut daimun: Vec<&HasilatFahs> = jami
        .hasilat
        .iter()
        .filter(|(_, hasila)| hasila.aila == Some(aila))
        .map(|(_, hasila)| hasila)
        .collect();
    daimun.sort_by_key(|hasila| Reverse(hasila.aqwa()));

    let hiyad: Vec<&HasilatFahs> = jami
        .hasilat
        .iter()
        .filter(|(_, hasila)| hasila.aila.is_none())
        .map(|(_, hasila)| hasila)
        .collect();

    let ghilafiyun: Vec<&HasilatFahs> = if ghilaf.is_some() {
        jami.hasilat
            .iter()
            .filter(|(_, hasila)| hasila.aila == Some(AilatMuharrik::Electron))
            .map(|(_, hasila)| hasila)
            .collect()
    } else {
        Vec::new()
    };

    Muharrik {
        aila,
        isdar: isdar_min(&daimun),
        khalfiya: khalfiya_min(aila, &daimun, &hiyad),
        itarat: itarat_min(&daimun, &hiyad, &ghilafiyun),
        rusum: jami.rusum().into_iter().filter(|wajiha| *wajiha != WajihaRusum::Majhula).collect(),
        mimariya: mimariya_min(&daimun, jami),
        thiqa: thiqa(jami, aila),
        dalail: dalail_min(jami, aila, mukhtar, ghilaf, &murashshahun),
    }
}

/// The engine version, from the most authoritative reading among the detectors
/// that supported the adopted family.
///
/// Exactness first — an inferred range loses to every reading — then
/// [`hujjiyat_masdar`], then the detector's own strength. The three tiers and
/// why they are in that order are documented at the top of this module.
///
/// Taking the first detector that read anything, which is what this did, made
/// the answer depend on which of two equally strong detectors was registered
/// first: a byte sweep that happened to run first outranked the engine's own
/// header, and a range interpolated from a pak footer outranked the exact tag
/// the build wrote into its executable.
///
/// Ties keep the earlier candidate, and `daimun` arrives strongest-first, so
/// identical evidence still produces an identical answer.
fn isdar_min(daimun: &[&HasilatFahs]) -> Option<IsdarMuharrik> {
    let mut afdal: Option<((bool, u8, u8), &IsdarMuharrik)> = None;
    for hasila in daimun {
        let Some(isdar) = hasila.isdar.as_ref() else { continue };
        let miftah = (!mushtaqq(isdar), hujjiyat_isdar(hasila, isdar), hasila.aqwa());
        if afdal.is_none_or(|(sabiq, _)| miftah > sabiq) {
            afdal = Some((miftah, isdar));
        }
    }
    afdal.map(|(_, isdar)| isdar.clone())
}

/// Where one detector's version reading came from, at its best.
///
/// A detector records the version and its evidence as separate fields, so the
/// link between them is the raw string itself: an observation that reports a
/// version quotes it, which is what makes this a rule about evidence rather
/// than a list of detector names.
///
/// Where nothing quotes it — a detector that assembled the raw string out of
/// numbers it read rather than copying it off disk — the answer is the best
/// source that detector used anywhere. That is a ceiling on where the version
/// could have come from, so it never credits a detector with a kind of evidence
/// it did not produce, and a detector whose every observation is a byte sweep
/// stays in the lower band under either path.
fn hujjiyat_isdar(hasila: &HasilatFahs, isdar: &IsdarMuharrik) -> u8 {
    hasila
        .dalail
        .iter()
        .filter(|daleel| daleel.wasf.contains(&isdar.khaam))
        .map(|daleel| hujjiyat_masdar(daleel.naw))
        .max()
        .or_else(|| hasila.dalail.iter().map(|daleel| hujjiyat_masdar(daleel.naw)).max())
        .unwrap_or(0)
}

/// How the game's code runs.
///
/// Three sources in order. A supporting detector's reading wins outright. A
/// detector that named no family is trusted only when what it read is possible
/// for the adopted engine, so that a stray `mono-2.0-bdwgc` import cannot make
/// an Unreal game report a Mono backend. Failing both, engines whose backend is
/// fixed by construction — Ren'Py is Python, VX Ace is Ruby — get theirs from
/// the engine itself.
///
/// Unity and Godot are deliberately excluded from that last step. Mono against
/// IL2CPP and `GDScript` against C# are real forks in the road that decide which
/// payload is installed, and guessing one of them would install the wrong
/// component into somebody's game.
fn khalfiya_min(
    aila: AilatMuharrik,
    daimun: &[&HasilatFahs],
    hiyad: &[&HasilatFahs],
) -> KhalfiyaBarmajiya {
    daimun
        .iter()
        .find_map(|hasila| hasila.khalfiya)
        .or_else(|| {
            hiyad
                .iter()
                .find_map(|hasila| hasila.khalfiya)
                .filter(|khalfiya| khalfiya_tunasib(aila, *khalfiya))
        })
        .or_else(|| khalfiya_bunyawiya(aila))
        .unwrap_or(KhalfiyaBarmajiya::Majhula)
}

/// Whether an engine could possibly run its code this way.
const fn khalfiya_tunasib(aila: AilatMuharrik, khalfiya: KhalfiyaBarmajiya) -> bool {
    match aila {
        AilatMuharrik::Unity => {
            matches!(khalfiya, KhalfiyaBarmajiya::Mono | KhalfiyaBarmajiya::Il2cpp)
        }
        AilatMuharrik::Unreal => matches!(khalfiya, KhalfiyaBarmajiya::UnrealNative),
        AilatMuharrik::Godot => matches!(
            khalfiya,
            KhalfiyaBarmajiya::GdScript
                | KhalfiyaBarmajiya::GodotCSharp
                | KhalfiyaBarmajiya::GodotNative
        ),
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz | AilatMuharrik::Electron => {
            matches!(khalfiya, KhalfiyaBarmajiya::JavaScript)
        }
        AilatMuharrik::RpgMakerVxAce => matches!(khalfiya, KhalfiyaBarmajiya::Ruby),
        AilatMuharrik::Renpy => matches!(khalfiya, KhalfiyaBarmajiya::Python),
        AilatMuharrik::GameMaker => matches!(khalfiya, KhalfiyaBarmajiya::GameMakerVm),
        // Nothing, and that is the honest answer rather than an oversight. This
        // engine's game code is compiled C++ with no scripting runtime under it
        // at all, and [`KhalfiyaBarmajiya`] has no value for that: `UnrealNative`
        // names Unreal's C++ specifically and would be a false statement here.
        // So no neutral detector's reading is accepted for it, which is also
        // what stops a stray `mono-2.0-bdwgc` in a neighbouring library from
        // giving a 2005 Capcom binary a Mono backend.
        AilatMuharrik::Bio4 => false,
        AilatMuharrik::Majhul => true,
    }
}

/// The backend an engine has by construction, where there is only one.
const fn khalfiya_bunyawiya(aila: AilatMuharrik) -> Option<KhalfiyaBarmajiya> {
    match aila {
        AilatMuharrik::Unreal => Some(KhalfiyaBarmajiya::UnrealNative),
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz | AilatMuharrik::Electron => {
            Some(KhalfiyaBarmajiya::JavaScript)
        }
        AilatMuharrik::RpgMakerVxAce => Some(KhalfiyaBarmajiya::Ruby),
        AilatMuharrik::Renpy => Some(KhalfiyaBarmajiya::Python),
        AilatMuharrik::GameMaker => Some(KhalfiyaBarmajiya::GameMakerVm),
        // `Bio4` is here for the same reason as the two beside it and a
        // different one. Unity and Godot are excluded because their fork in the
        // road decides which payload is installed; this one is excluded because
        // there is no value to name — see [`khalfiya_tunasib`] — so it resolves
        // to `Majhula`, which is what "compiled C++, no scripting runtime" has
        // to look like in a vocabulary that has no word for it yet.
        AilatMuharrik::Unity
        | AilatMuharrik::Godot
        | AilatMuharrik::Bio4
        | AilatMuharrik::Majhul => None,
    }
}

/// Every text system the adopted answer is entitled to claim, in the order it
/// was observed.
fn itarat_min(
    daimun: &[&HasilatFahs],
    hiyad: &[&HasilatFahs],
    ghilafiyun: &[&HasilatFahs],
) -> Vec<ItarNusus> {
    let mut itarat: Vec<ItarNusus> = Vec::new();
    for hasila in daimun.iter().chain(hiyad).chain(ghilafiyun) {
        for itar in &hasila.itarat {
            if !itarat.contains(itar) {
                itarat.push(*itar);
            }
        }
    }
    itarat
}

/// The architecture of the game's executable.
///
/// A supporting detector first, then any detector that read a binary at all,
/// and only then the host's own architecture. That last fallback is a guess and
/// is documented as one: it is used solely to pick which payload to install,
/// and picking wrong fails loudly at injection rather than quietly at runtime.
fn mimariya_min(daimun: &[&HasilatFahs], jami: &JamiHasilat) -> Mimariya {
    daimun
        .iter()
        .find_map(|hasila| hasila.mimariya)
        .or_else(|| jami.hasilat.iter().find_map(|(_, hasila)| hasila.mimariya))
        .unwrap_or_else(Mimariya::hali)
}

/// Every observation, plus resolution's own notes, strongest first.
///
/// The notes are the only way a decision made here reaches
/// [`crate::imkaniyat`], which is handed a [`Muharrik`] and nothing else — and
/// the only way it survives a round trip through the store, which persists a
/// report and not the probe that produced it.
fn dalail_min(
    jami: &JamiHasilat,
    aila: AilatMuharrik,
    mukhtar: Option<AilatMuharrik>,
    ghilaf: Option<u8>,
    murashshahun: &[(AilatMuharrik, u8)],
) -> Vec<Daleel> {
    let mut dalail: Vec<Daleel> = jami.dalail().into_iter().cloned().collect();

    if let Some(wazn) = ghilaf {
        dalail.push(Daleel {
            naw: NawDaleel::BinyatMujallad,
            wasf: format!(
                "{WASF_GHILAF}a browser shell packages this game around {}. The inner \
                 engine is the engine; the shell is how it was delivered, and installing \
                 anything needs both of them.",
                aila.ism()
            ),
            mawqi: None,
            wazn,
        });
    }

    for (aila_akhar, wazn) in murashshahun {
        if Some(*aila_akhar) == mukhtar {
            continue;
        }
        if ghilaf.is_some() && *aila_akhar == AilatMuharrik::Electron {
            continue;
        }
        let wasf = if mukhtar.is_some() {
            format!(
                "{WASF_TAARUD}{} was named at weight {wazn} and lost to {}. Its own \
                 evidence is in this list, recorded exactly as it was observed.",
                aila_akhar.ism(),
                aila.ism()
            )
        } else {
            format!(
                "{WASF_DUNA_HADD}{} was named at weight {wazn}, under the floor of \
                 {HADD_ADNA_LIL_TABANNI}. A suggestion this weak is not an \
                 identification, so the game is reported as unknown.",
                aila_akhar.ism()
            )
        };
        dalail.push(Daleel { naw: NawDaleel::BinyatMujallad, wasf, mawqi: None, wazn: *wazn });
    }

    dalail.sort_by_key(|daleel| Reverse(daleel.wazn));
    dalail
}
