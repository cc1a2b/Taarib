//! الإمكانيات — what Taarib can do to one specific game, said in language the
//! person who owns that game can act on.
//!
//! This is a product artifact and not a debug dump. Every string this module
//! produces is displayed: on the game's detail screen, above the install
//! button, in the library card, and in the confirmation the user reads before
//! anything is written to their disk. The test each sentence has to pass is not
//! "is it accurate" but "would somebody who does not know what a game engine is
//! understand what will happen to their game".
//!
//! ## Four statements, in order of what they cost the user
//!
//! **The tier.** Which of the three ways of working the game gets, and *why
//! that one and not a better one*. "Unsupported" is not an answer. The reason
//! is displayed next to the tier name, so it explains the engine's actual
//! constraint — Godot 3 has no Arabic shaping and one cannot be added without
//! rebuilding the engine — rather than restating the tier.
//!
//! **The reachable text systems.** What Taarib will take over, named, so that a
//! user whose menus stay English can see whether the menu system was ever on
//! the list.
//!
//! **The expected quality.** Derived from what was actually found, never from
//! the engine's reputation. A Unity game whose TextMeshPro was found is
//! excellent. A Unity game where no text system could be identified is limited,
//! on the same tier 1 engine, and the report says which systems were found so
//! the difference is visible rather than mysterious.
//!
//! **The limitations.** Specific, and the part this module is judged on.
//! Generalities are worthless here: "some text may not be translated" tells a
//! user nothing they can act on, while "text drawn inside images will not be
//! translated" tells them exactly which parts of their game will stay English
//! and why nobody can fix it.
//!
//! ## A tier is a permission, not a promise
//!
//! The four statements above all describe what *this engine* allows. A fifth,
//! [`jahiziya`], describes something else entirely: whether the half of Taarib
//! that does the work exists yet. Two users with the same Taarib version get the
//! same answer for the same game, and the answer never depends on anything that
//! can change under a stored report — not on what is in a component store, not
//! on what happens to be installed beside it.
//!
//! It is not, however, one answer per engine. What Taarib has to supply depends
//! on what the engine already does: a Ren'Py that bundles a shaper needs a font
//! and a translation, and a Ren'Py that does not needs the whole text pipeline
//! replaced. Those are different gaps and this build has closed different amounts
//! of each, so [`jahiziya`] reads the game where the game decides the question,
//! and says which reading it took in the sentence it returns.
//!
//! The two must never be collapsed. A limitation is something about the game
//! that no update will change — text painted into a picture is pixels forever.
//! An unfinished adapter is something about Taarib that an update will change,
//! and a user who is told "your game cannot be Arabized" when the truth is "not
//! yet" has been given the wrong answer twice: they stop waiting for the update,
//! and they blame their game for Taarib's gap. So the verdict is its own field
//! and its own sentence, never a [`Hadd`] in the limitation list.
//!
//! It has one hard consequence, which [`taqreer`] enforces. A report may not
//! claim what its build cannot deliver: when the verdict is
//! [`JahiziyatTashghil::Ghaiba`] the reachable text systems are emptied — naming
//! systems no code reaches is the report promising a tier — and the expected
//! quality drops to the floor. [`JahiziyatTashghil::Naqisa`] keeps the systems,
//! because they really are taken over, and caps the quality below excellent,
//! because the same report is saying in the next sentence that a named part of
//! the tier does not arrive. [`Tabaqa`] itself is left alone, because it is the
//! value every downstream dispatch reads and a report that lied about it in the
//! other direction would route an install somewhere else; it says which tier the
//! engine qualifies for, and the verdict beside it says whether that tier runs.
//!
//! ## Anti-cheat is not a limitation, it is a refusal
//!
//! When the launcher associates an anti-cheat service or a VAC record with a
//! game, the report sets `marfuda`, reports the lowest tier, and says plainly
//! that Taarib will not touch the game — not by patching it and not by drawing
//! over it — because doing so can cost the user a permanent ban on their
//! account. That sentence is never softened and there is no override anywhere
//! in the product for it to hint at.
//!
//! Reporting the *lowest* tier for a refused game is deliberate. Every
//! downstream dispatch reads `tabaqa`, so the fail-safe direction is the tier
//! that installs nothing, and `marfuda` then blocks even that.
//!
//! ## Where the report gets its facts
//!
//! [`taqreer`] is handed a [`Muharrik`], the launcher's hints, and a timestamp.
//! That is on purpose: a report has to be reproducible from what the store
//! holds, and the store holds a report and the engine behind it, not the probe
//! or the directory it walked. Anything the report needs to say about the
//! game's files therefore comes out of the evidence those files produced, and
//! [`alamat`] is the list of exactly what it looks for.

use taarib_kashf::fahs::SimatLuba;
use taarib_mustalahat::muharrik::{
    AilatMuharrik, Hadd, ItarNusus, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya,
    Muharrik, Tabaqa, TaqreerImkaniyat,
};

use crate::tahdid::{maghlufa, mutaarid};

/// The version of the probe that produced a report.
///
/// Stamped into every [`TaqreerImkaniyat`] and stored beside it. When this
/// number rises, every game whose stored report carries a lower one is probed
/// again on the next launch, so that an improvement in detection reaches games
/// that were already examined instead of only new ones.
///
/// Raise it whenever a change would produce a different report for a game whose
/// files did not change: a new detector, a corrected weight, a changed tier
/// rule, a reworded limitation. Do not raise it for a change that cannot alter
/// any output.
///
/// Raised to 2 when [`jahiziya`] was added: every stored report predates the
/// verdict and would otherwise keep promising a tier this build does not run.
///
/// Raised to 3 when [`jahiziya`] stopped answering [`JahiziyatTashghil::Ghaiba`]
/// unconditionally. A Ren'Py game on an engine that shapes now reports
/// [`JahiziyatTashghil::Naqisa`], and every stored report carries the old
/// verdict and the old sentences for every other engine besides.
///
/// Raised to 4 when that same arm became [`JahiziyatTashghil::Mukammala`]. The
/// staging matrix now carries an Arabic face into the Ren'Py component, so the
/// one thing that was missing — a font the game could actually draw the
/// translation with — ships. A stored report would otherwise keep withholding
/// the one-button run from the only engine this build can finish, which is the
/// expensive direction for this field to be stale in.
///
/// Raised to 5 when [`AilatMuharrik::Bio4`] and its detector were added. Every
/// game on Capcom's `BIO4` codebase was previously reported as an unrecognised
/// engine at tier 3, and every one of those reports is stale in the direction
/// that matters: the engine is now named, the tier moves from the overlay to
/// full Arabization, and the reason sentence and the limitations change with it.
/// This is deliberate and it is not free — raising this number re-probes every
/// stored scan on every machine, for every engine, not only for the new one —
/// and it is the price of a detector reaching games that were already examined
/// rather than only games examined after the update.
///
/// Raised to 6 when [`taarib_mustalahat::muharrik::WajihaRusum`] gained
/// Direct3D 8, 9 and 10 and the binary
/// detector gained the imports that establish them. Three backends had been
/// built with no vocabulary to name them, so every stored report for a game
/// whose renderer is one of those three carries an empty graphics list —
/// Resident Evil 4 among them, with `d3d9.dll` in its own import table — and at
/// tier 3 that empty list is what makes the report say the API "was not
/// determined". Stored reports for the Unreal titles are stale too, in the
/// smaller direction: they name every API those games import except the
/// Direct3D 9 one.
///
/// Raised to 7 when six in-house engine families —
/// [`AilatMuharrik::Frostbite`], [`AilatMuharrik::BlackSpace`],
/// [`AilatMuharrik::Alchemy`], [`AilatMuharrik::Dantelion`],
/// [`AilatMuharrik::Rage`] and [`AilatMuharrik::Snowdrop`] — and the detectors
/// in `crate::dalail::khassa` that read them were added. Every game on one of
/// the six was previously reported as an unrecognised engine, and every one of
/// those stored reports is stale in the sentences rather than in the tier: the
/// tier stays at the overlay because none of the six has an adapter, but the
/// engine is now named, the reason sentence says what it is instead of saying
/// nothing was recognised, and the readiness gap names the container the text
/// actually sits in. Raising this number re-probes every stored scan on every
/// machine for every engine, which is the price of a detector reaching games
/// that were already examined.
pub const ISDAR_FAHS: u32 = 7;

/// The confidence below which the report tells the user the identification may
/// be wrong.
///
/// Sixty is where [`crate::tahdid::SAQF_TAARUD`] lands a top-band
/// identification that a top-band rival contradicted, which is exactly the
/// situation the user should be warned about: an answer was produced, and
/// something solid disagreed with it.
pub const HADD_THIQA_MUNKHAFIDA: u8 = 60;

/// The markers the report looks for in an engine's evidence trail.
///
/// Each is a phrase a detector in `dalail` writes deliberately, chosen because
/// it appears in the situation the report describes and in no other. The
/// negative forms are the reason the phrases are long: `dalail` records
/// "Content/Localization exists but holds no loose .locres file" as well as the
/// positive case, so a marker of `Content/Localization` would match a game that
/// has none. The clause that only the one situation produces is the marker.
///
/// Matching is ASCII case-insensitive, is done against both the description and
/// the location of every observation, and treats `\` and `/` alike so a Windows
/// path and a Linux one match the same marker.
///
/// A detector that reworded one of these sentences without changing the phrase
/// here would silently drop a limitation from the report, so each marker is
/// worded as a claim rather than as a fragment: rewording around it is safe,
/// deleting the claim is what must not happen quietly.
pub mod alamat {
    /// The game ships no loose localization file, so its strings live inside
    /// its containers and extraction has to open them.
    pub const NUSUS_DAKHIL_HAWIYA: &[&str] = &["extraction has to read the pak"];

    /// A container whose contents cannot be read without a key only the user
    /// can supply — an Unreal pak or IoStore chunk under AES, or a Godot
    /// package exported with `PACK_DIR_ENCRYPTED`.
    pub const TASHFEER: &[&str] = &["until the user supplies"];

    /// An IL2CPP build whose metadata this build cannot walk.
    ///
    /// An unrecognised version number, an encrypted or restructured header, or
    /// a file that will not open. All three land in the same place — the
    /// adapter has to resolve text functions by scanning the executable for
    /// their signatures.
    pub const MITA_MAJHULA: &[&str] = &[
        "go straight to binary signature scanning",
        "resolve by binary signature alone",
        "metadata file present and not readable at all",
    ];

    /// Godot 4's advanced text server — the component that shapes Arabic inside
    /// the engine.
    ///
    /// The difference between tier 1 and tier 2 for a Godot game whose version
    /// string could not be read. Godot 3's own evidence says "has no
    /// `TextServer`", which this deliberately does not match.
    pub const KHADIM_NUSUS: &[&str] = &["TextServerAdvanced"];

    /// A Ren'Py build that carries its own Arabic shaper.
    ///
    /// Written by `dalail::nusus::renpy_tashkeel`, in two forms: read from
    /// `version_tuple`, and inferred from the `py2`/`py3` prefixes Ren'Py
    /// introduced in `lib/` at the same release. Both are recorded here because
    /// the two say the same thing with different confidence, and
    /// [`super::jahiziya`] treats them alike — the alternative is a game whose
    /// `renpy/__init__.py` was unreadable silently losing a verdict the probe
    /// did in fact reach.
    pub const TASHKEEL_RENPY: &[&str] = &[
        "shapes and reorders Arabic itself",
        "the engine can shape Arabic itself",
    ];

    /// A Ren'Py build with no shaper of its own.
    ///
    /// The negative of [`TASHKEEL_RENPY`], recorded separately rather than
    /// derived from its absence: a game whose version could not be bounded at
    /// all produces neither marker, and that is a third answer rather than this
    /// one. Conflating the two would report "the engine cannot shape" about a
    /// game nothing was learned about.
    pub const BILA_TASHKEEL_RENPY: &[&str] = &[
        "the engine has no shaper and Arabic has to be laid out",
        "or earlier and has no shaper",
    ];
}

/// Builds a limitation from its two sentences.
fn hadd(arabi: impl Into<String>, injilizi: impl Into<String>) -> Hadd {
    Hadd {
        arabi: arabi.into(),
        injilizi: injilizi.into(),
    }
}

/// Whether any observation mentions any of `ibarat`, in its description or its
/// location.
fn shuhida(muharrik: &Muharrik, ibarat: &[&str]) -> bool {
    let matlub: Vec<String> = ibarat
        .iter()
        .map(|ibara| ibara.to_ascii_lowercase().replace('\\', "/"))
        .collect();
    muharrik.dalail.iter().any(|daleel| {
        let wasf = daleel.wasf.to_ascii_lowercase().replace('\\', "/");
        let mawqi = daleel
            .mawqi
            .as_ref()
            .map(|mawqi| mawqi.to_ascii_lowercase().replace('\\', "/"))
            .unwrap_or_default();
        matlub
            .iter()
            .any(|ibara| wasf.contains(ibara) || mawqi.contains(ibara))
    })
}

/// Whether a text system was found.
fn ladayh(muharrik: &Muharrik, itar: ItarNusus) -> bool {
    muharrik.itarat.contains(&itar)
}

/// Whether this Godot build carries the engine-side Arabic shaping that decides
/// its tier.
///
/// The version string settles it when there is one. When there is not — a
/// custom build, a stripped executable, a PCK whose header did not parse — the
/// presence of the text server settles it instead, and if neither is available
/// the game is treated as the older engine. That direction is the safe one:
/// drawing text over Godot 4 works, while switching on shaping that Godot 3
/// does not have does not.
fn godot_arbaa(muharrik: &Muharrik) -> bool {
    muharrik.isdar.as_ref().map_or_else(
        || shuhida(muharrik, alamat::KHADIM_NUSUS),
        |isdar| isdar.kabir >= 4,
    )
}

/// Whether this Ren'Py build joins and reorders Arabic on its own.
///
/// [`None`] when nothing in the evidence bounds the version — a game shipping
/// no `renpy/__init__.py` this build could read and no `lib/` it could list.
/// That third answer exists because [`jahiziya`] gives it its own verdict: a
/// build that shapes is one an installed translation is legible in, a build that
/// does not needs a glyph takeover this package cannot deliver, and a build
/// nobody could identify must not be told either story.
///
/// The version settles it when there is one, because
/// [`crate::dalail::nusus::HADD_TASHKEEL_RENPY`] is where the release that
/// bundles `HarfBuzz` and `FriBidi` is recorded and this must not hold a second
/// copy of that number. Otherwise the probe's own sentence settles it: the
/// detector already made this exact judgement from the `lib/` directory names,
/// and re-deriving it here from something weaker would be a second opinion about
/// a question already answered.
fn renpy_yashkul(muharrik: &Muharrik) -> Option<bool> {
    if let Some(isdar) = muharrik.isdar.as_ref() {
        let (kabir, sagheer) = crate::dalail::nusus::HADD_TASHKEEL_RENPY;
        return Some(isdar.kabir > kabir || (isdar.kabir == kabir && isdar.sagheer >= sagheer));
    }
    if shuhida(muharrik, alamat::TASHKEEL_RENPY) {
        return Some(true);
    }
    if shuhida(muharrik, alamat::BILA_TASHKEEL_RENPY) {
        return Some(false);
    }
    None
}

/// The tier that applies, and why that tier and not a better one, in Arabic and
/// then in English.
///
/// The whole dispatch table for the rest of the product is this function. Unity
/// on either backend, Unreal, Godot 4, Electron and every script engine take
/// tier 1, where text is replaced inside the game. Godot 3 takes tier 2,
/// because it has no Arabic shaping to switch on and one cannot be added to it
/// without rebuilding the engine, so Taarib draws the text itself. An engine
/// this build did not recognise takes tier 3, where nothing is modified at all.
///
/// The two returned sentences are displayed under the tier name. They explain
/// the engine's constraint rather than restating the tier, because a user
/// reading "tier 2" wants to know what stopped it being tier 1.
#[must_use]
pub fn tabaqa_min_muharrik(muharrik: &Muharrik) -> (Tabaqa, String, String) {
    match muharrik.aila {
        AilatMuharrik::Unity => tabaqa_unity(muharrik),
        AilatMuharrik::Unreal => (
            Tabaqa::Kamil,
            "أنريل يشكّل النصوص بنفسه، فيكفي أن يشغّل تعريب تشكيل العربية داخل المحرّك \
             ويستبدل نصوص اللعبة، لتظهر العربية بخط اللعبة نفسه وداخل واجهتها."
                .to_owned(),
            "Unreal shapes text itself, so Taarib only has to switch Arabic shaping on \
             inside the engine and replace the game's text; the Arabic then appears in \
             the game's own font and inside its own interface."
                .to_owned(),
        ),
        AilatMuharrik::Godot => tabaqa_godot(muharrik),
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz => (
            Tabaqa::Kamil,
            "نصوص RPG Maker مخزَّنة في ملفات بيانات يمكن استبدالها مباشرة، ومحرّك اللعبة \
             يعمل بجافاسكربت يمكن أن تُضاف إليه وحدة تعريب، فتُعرَّب اللعبة من داخلها."
                .to_owned(),
            "RPG Maker keeps its text in data files that can be replaced directly, and \
             the game runs on JavaScript that a Taarib module can be added to, so the \
             game is Arabized from inside."
                .to_owned(),
        ),
        AilatMuharrik::RpgMakerVxAce => (
            Tabaqa::Kamil,
            "نصوص RPG Maker VX Ace داخل أرشيف يمكن إعادة بنائه، ومحرّكها يعمل بلغة روبي \
             يمكن أن تُضاف إليها وحدة تعريب، فتُعرَّب اللعبة من داخلها."
                .to_owned(),
            "RPG Maker VX Ace keeps its text inside an archive that can be rebuilt, and \
             the engine runs on Ruby that a Taarib module can be added to, so the game \
             is Arabized from inside."
                .to_owned(),
        ),
        AilatMuharrik::Renpy => (
            Tabaqa::Kamil,
            "رِن باي يحمل نظام ترجمة خاصًا به، ويستخدمه تعريب كما هو ويضيف إليه وحدة \
             بايثون تشكّل العربية، فتظهر الترجمة كأنها لغة رسمية في اللعبة."
                .to_owned(),
            "Ren'Py has a translation system of its own. Taarib uses it as it stands and \
             adds a Python module that shapes Arabic, so the translation behaves like an \
             official language of the game."
                .to_owned(),
        ),
        AilatMuharrik::GameMaker => (
            Tabaqa::Kamil,
            "نصوص GameMaker داخل ملف بيانات واحد يمكن تعديله، ويولّد تعريب صفحات حروف \
             عربية ويضعها فيه، فتُستبدل النصوص من داخل اللعبة."
                .to_owned(),
            "GameMaker keeps its text inside a single data file that can be edited. \
             Taarib generates Arabic glyph pages, writes them into it, and replaces the \
             text from inside the game."
                .to_owned(),
        ),
        AilatMuharrik::Electron => (
            Tabaqa::Kamil,
            "تعمل واجهة هذه اللعبة داخل متصفح مدمج، والمتصفح يشكّل العربية أصلًا. يستبدل \
             تعريب نصوص الحزمة ويضيف وحدة تشغيل صغيرة، فتظهر العربية كأنها الأصل."
                .to_owned(),
            "This game's interface runs inside an embedded browser, and a browser already \
             shapes Arabic. Taarib replaces the text in the bundle and adds a small \
             runtime module, so the Arabic reads as though it were original."
                .to_owned(),
        ),
        AilatMuharrik::Bio4 => (
            Tabaqa::Kamil,
            "نصوص هذه اللعبة في ملفات قاموس داخل اللعبة نفسها، وخطوطها صفحات حروف مرسومة \
             مخزَّنة معها. المحرّك نفسه لا يعرف تشكيل العربية إطلاقًا، لكن ذلك لا يمنع \
             التعريب الكامل: يُشكّل تعريب النص ويرتّبه قبل كتابته، ويولّد صفحات الحروف \
             العربية ويضعها مكان الأصلية، فيقرأ المحرّك عربية جاهزة دون أن يعرف أنها عربية."
                .to_owned(),
            "This game keeps its text in dictionary files inside the game and its fonts as \
             drawn glyph pages stored beside them. The engine itself has no Arabic shaping \
             whatsoever, and that does not stop full Arabization: Taarib shapes and reorders \
             the text before it is written, generates the Arabic glyph pages and puts them \
             where the original ones were, so the engine reads finished Arabic without \
             knowing it is Arabic."
                .to_owned(),
        ),
        AilatMuharrik::Frostbite
        | AilatMuharrik::BlackSpace
        | AilatMuharrik::Alchemy
        | AilatMuharrik::Dantelion
        | AilatMuharrik::Rage
        | AilatMuharrik::Snowdrop => tabaqa_khassa(muharrik.aila),
        AilatMuharrik::Majhul => (
            Tabaqa::TarjamaFawqiya,
            "لم يتعرّف تعريب على محرّك هذه اللعبة. هذه اللعبة تستخدم نظام نصوص غير معروف؛ \
             سيُستخدم أسلوب الطبقة: يُقرأ ما يظهر على الشاشة وتُعرض العربية فوقه، دون \
             تعديل أي ملف من ملفات اللعبة."
                .to_owned(),
            "Taarib did not recognise this game's engine. It uses an unrecognised text \
             system, so the overlay approach is used: what appears on screen is read and \
             Arabic is shown over it, without modifying any of the game's files."
                .to_owned(),
        ),
    }
}

/// Where an in-house engine keeps the text a player reads, in Arabic and then
/// in English.
///
/// One sentence fragment per engine, and every one of them names a container
/// this build has no reader for. That is the whole reason these six are tier 3
/// rather than tier 1, so it is written down once and used by both the tier's
/// reason and the readiness gap rather than being said twice in two voices.
///
/// [`None`] for every engine that is not one of the six, because the question
/// only means something for an engine Taarib named and cannot open.
const fn khazinat_nusus(aila: AilatMuharrik) -> Option<(&'static str, &'static str)> {
    match aila {
        AilatMuharrik::Frostbite => Some((
            "داخل فهارس أرشيف مُعمّاة وحُزم محتوى خلفها",
            "inside obfuscated archive indexes and the content bundles behind them",
        )),
        AilatMuharrik::BlackSpace => Some((
            "داخل أرشيفات محتوى خاصة بالمحرّك في مجلّدات مرقّمة",
            "inside the engine's own content archives in its numbered directories",
        )),
        AilatMuharrik::Alchemy => Some((
            "داخل أرشيفات المحرّك وملفّات كائناته",
            "inside the engine's archives and its object files",
        )),
        AilatMuharrik::Dantelion => Some((
            "داخل أرشيفات رسائل مضغوطة، مجلّد لكل لغة",
            "inside compressed message archives, one directory per language",
        )),
        AilatMuharrik::Rage => Some((
            "داخل أرشيفات المحرّك الكبيرة المُعمّاة",
            "inside the engine's large encrypted archives",
        )),
        AilatMuharrik::Snowdrop => Some((
            "داخل قطع محتوى يفهرسها جدول محتويات واحد",
            "inside content chunks indexed by a single table of contents",
        )),
        AilatMuharrik::Unity
        | AilatMuharrik::Unreal
        | AilatMuharrik::Godot
        | AilatMuharrik::RpgMakerMv
        | AilatMuharrik::RpgMakerMz
        | AilatMuharrik::RpgMakerVxAce
        | AilatMuharrik::Renpy
        | AilatMuharrik::GameMaker
        | AilatMuharrik::Electron
        | AilatMuharrik::Bio4
        | AilatMuharrik::Majhul => None,
    }
}

/// The tier for an engine Taarib can name and cannot get inside.
///
/// Tier 3, and the two sentences say why in the only terms that matter to a
/// player: the engine *was* recognised, and that recognition on its own does not
/// open it. Naming an engine is worth doing — it is the difference between "we
/// have no idea what this game is" and "we know exactly what this is and cannot
/// reach into it yet" — and writing the first sentence as though it were the
/// second would be the report taking credit for work nobody has done.
///
/// Every one of the six shares this arm because every one of them is in the same
/// position: no plugin system, no scripting runtime, no published container
/// format, and no adapter in this build. When one of them gains an adapter it
/// gains an arm of its own, and this function is where the reviewer will look
/// for the ones that have not.
fn tabaqa_khassa(aila: AilatMuharrik) -> (Tabaqa, String, String) {
    let (ayn_arabi, ayn_injilizi) = khazinat_nusus(aila)
        .unwrap_or(("داخل ملفّات المحرّك نفسها", "inside the engine's own files"));
    let ism = aila.ism();
    (
        Tabaqa::TarjamaFawqiya,
        format!(
            "تعرّف تعريب على محرّك هذه اللعبة: {ism}. وهو محرّك داخلي لا يُرخَّص لأحد، ولا \
             يقبل إضافات، ونصوصه {ayn_arabi} بصيغ لا يقرؤها تعريب. فالتعرّف على اسمه لا يفتحه: \
             سيُستخدم أسلوب الطبقة، يُقرأ ما يظهر على الشاشة وتُعرض العربية فوقه، دون تعديل أي \
             ملف من ملفات اللعبة."
        ),
        format!(
            "Taarib recognised this game's engine: {ism}. It is an in-house engine, licensed \
             to nobody, with no plugin system, and it keeps its text {ayn_injilizi} in formats \
             Taarib has no reader for. Knowing its name does not open it, so the overlay \
             approach is used: what appears on screen is read and Arabic is shown over it, \
             without modifying any of the game's files."
        ),
    )
}

/// Unity's tier is always 1; only the explanation differs with the backend.
fn tabaqa_unity(muharrik: &Muharrik) -> (Tabaqa, String, String) {
    let (arabi, injilizi) = match muharrik.khalfiya {
        KhalfiyaBarmajiya::Mono => (
            "محرّك Unity مدعوم بالكامل، وشيفرة هذه اللعبة تعمل على Mono، أي أنها في ملفات \
             يمكن الوصول إليها. يُحمَّل مكوّن تعريب مع اللعبة ويستبدل نصوصها من داخلها.",
            "Unity is fully supported, and this game's code runs on Mono, which means it \
             sits in files Taarib can reach. A Taarib component loads with the game and \
             replaces its text from inside it.",
        ),
        KhalfiyaBarmajiya::Il2cpp => (
            "محرّك Unity مدعوم بالكامل. شيفرة هذه اللعبة مترجَمة إلى ملف تنفيذي (IL2CPP)، \
             ويصل تعريب إلى أنظمة نصوصها عبر طبقة ربط مخصّصة، فتُستبدل النصوص من داخلها.",
            "Unity is fully supported. This game's code is compiled into a native \
             executable (IL2CPP), and Taarib reaches its text systems through a dedicated \
             interop layer, so the text is still replaced inside the game.",
        ),
        _ => (
            "محرّك Unity مدعوم بالكامل في حالتيه، وسيُستبدل النص من داخل اللعبة. لم تتّضح \
             بعدُ طريقة تشغيل شيفرتها، وهي التي تحدّد أي مكوّن يُثبَّت، وستُحسم عند التثبيت.",
            "Unity is fully supported in both of its forms, and the text will be replaced \
             from inside the game. How this game's code runs is not yet clear, and that \
             is what decides which component is installed; it is settled at install time.",
        ),
    };
    (Tabaqa::Kamil, arabi.to_owned(), injilizi.to_owned())
}

/// Godot is the one engine whose tier depends on its version.
fn tabaqa_godot(muharrik: &Muharrik) -> (Tabaqa, String, String) {
    if godot_arbaa(muharrik) {
        return (
            Tabaqa::Kamil,
            "غودوت 4 يحمل نظام تشكيل نصوص كاملًا، فيكفي تفعيله واستبدال نصوص اللعبة لتظهر \
             العربية موصولة ومرتّبة من داخل المحرّك نفسه."
                .to_owned(),
            "Godot 4 carries a complete text shaping system, so enabling it and replacing \
             the game's text is enough for Arabic to appear correctly joined and ordered \
             from inside the engine itself."
                .to_owned(),
        );
    }

    if muharrik.isdar.is_some() {
        return (
            Tabaqa::RasmMubashir,
            "غودوت 3 وما قبله لا يحتوي أي نظام لتشكيل العربية، ولا يمكن إضافته دون إعادة \
             بناء المحرّك كاملًا. لذلك يرسم تعريب النص بنفسه فوق عناصر النص في اللعبة، \
             وهذه هي الطبقة الثانية لا الأولى."
                .to_owned(),
            "Godot 3 and earlier have no Arabic shaping at all, and one cannot be added \
             without rebuilding the whole engine. Taarib therefore draws the text itself \
             over the game's own text objects, which is tier two rather than tier one."
                .to_owned(),
        );
    }

    (
        Tabaqa::RasmMubashir,
        "تعذّرت قراءة إصدار غودوت من ملفات هذه اللعبة، وتشكيل المحرّك للعربية لا يوجد إلا \
         في الإصدار الرابع. اختير الأسلوب الذي يعمل في الإصدارين معًا: يرسم تعريب النص \
         بنفسه فوق نصوص اللعبة."
            .to_owned(),
        "The Godot version could not be read from this game's files, and the engine's own \
         Arabic shaping exists only in version 4. The approach that works in both \
         versions was chosen: Taarib draws the text itself over the game's own text."
            .to_owned(),
    )
}

// ---------------------------------------------------------------------------
// الجاهزية — what this build's runtime actually delivers
// ---------------------------------------------------------------------------

/// Whether this build can deliver the tier it just named, and what is missing.
///
/// The question every arm answers is not "was something written into the game"
/// but **"does a player see legible Arabic"**. Text landing in a data file is
/// necessary and is not sufficient: a string the engine has no letter pictures
/// for is drawn blank, and a string an engine without a shaper draws one
/// character at a time is drawn as unjoined letters. Both are writes that
/// succeeded and neither is Arabic, so both answer
/// [`JahiziyatTashghil::Ghaiba`].
///
/// Each arm was established by reading the adapter for that engine end to end
/// and finding the exact function where the chain stops. `docs/tashghil.md`
/// records each one with its file and its terminating call, and that document
/// and this function are meant to move together.
///
/// ## What is derived and what is a maintained fact
///
/// Two arms read the game. [`godot_arbaa`] decides which Godot adapter is being
/// judged, and [`renpy_yashkul`] decides Ren'Py's verdict outright — a Ren'Py
/// that shapes needs only a font, which is a named gap, and one that does not
/// needs a glyph takeover this package cannot deliver, which is the whole
/// thing. Those are real facts about the game and they are read rather than
/// assumed.
///
/// The rest cannot be derived from anything this crate can see. Whether a
/// managed assembly binds the plugin loader that is staged beside it, whether a
/// TypeScript adapter was ever put through `esbuild`, whether a function has a
/// caller — no program reads those off itself, and every crate below this one in
/// the graph is a crate this one may not depend on. So they are maintained
/// facts, each with its evidence in its own comment, pinned by the tests at the
/// foot of this file so that finishing an adapter has to move the verdict
/// deliberately.
///
/// **This deliberately does not read the component store.** Whether
/// `mulhaq/electron/taarib.js` is on this machine is exactly the fact two of
/// these arms turn on, and reading it would still be wrong: a report is
/// persisted and is re-probed only when [`ISDAR_FAHS`] rises, so a verdict taken
/// from a directory that can change underneath it would be cached and go stale —
/// which is the failure this whole field exists to prevent. The arm states what
/// this *build* ships, and a build that ships neither the adapter nor a step
/// that produces it ships nothing on any machine.
///
/// ## The sentences
///
/// Addressed to a player, not to a maintainer. Each names the part that is
/// missing in terms of what the player would have seen, and each ends by saying
/// an update closes it — because the single most useful thing a user can take
/// from this screen is whether to wait or to give up. Where an install refuses
/// outright rather than completing into an unchanged game, the sentence says so:
/// "you can install the patch now" is not a thing to tell somebody whose install
/// is going to stop.
#[must_use]
pub fn jahiziya(muharrik: &Muharrik) -> (JahiziyatTashghil, Option<Hadd>) {
    let (jahiziya, naqs) = match muharrik.aila {
        AilatMuharrik::Unity => (JahiziyatTashghil::Ghaiba, naqs_unity(muharrik)),
        AilatMuharrik::Unreal => (JahiziyatTashghil::Ghaiba, naqs_unreal()),
        AilatMuharrik::Godot => (
            JahiziyatTashghil::Ghaiba,
            if godot_arbaa(muharrik) {
                naqs_godot_arbaa()
            } else {
                naqs_godot_thalith()
            },
        ),
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz => {
            (JahiziyatTashghil::Ghaiba, naqs_rpg_maker())
        },
        AilatMuharrik::RpgMakerVxAce => (JahiziyatTashghil::Ghaiba, naqs_vx_ace()),
        AilatMuharrik::Renpy => jahiziyat_renpy(muharrik),
        AilatMuharrik::GameMaker => (JahiziyatTashghil::Ghaiba, naqs_gamemaker()),
        AilatMuharrik::Electron => (JahiziyatTashghil::Ghaiba, naqs_electron()),
        AilatMuharrik::Bio4 => (JahiziyatTashghil::Ghaiba, naqs_bio4()),
        AilatMuharrik::Frostbite
        | AilatMuharrik::BlackSpace
        | AilatMuharrik::Alchemy
        | AilatMuharrik::Dantelion
        | AilatMuharrik::Rage
        | AilatMuharrik::Snowdrop => (JahiziyatTashghil::Ghaiba, naqs_khassa(muharrik.aila)),
        AilatMuharrik::Majhul => (JahiziyatTashghil::Ghaiba, naqs_tabaqa()),
    };
    // A finished tier has nothing to warn about, and `TaqreerImkaniyat::naqs`
    // states that as its contract. Ren'Py on a shaping engine is the first arm
    // to answer `Mukammala`, and it reaches here still carrying the sentence it
    // needed while a font was missing — which is exactly why this is enforced
    // in one place rather than in each arm.
    match jahiziya {
        JahiziyatTashghil::Mukammala => (jahiziya, None),
        JahiziyatTashghil::Naqisa | JahiziyatTashghil::Ghaiba => (jahiziya, Some(naqs)),
    }
}

/// Ren'Py's verdict, which its own engine build decides.
///
/// The one engine in the table whose answer is not the same for every game, and
/// the split is real rather than cautious. `taarib-tathbeet`'s `nusus` module
/// calls `taarib_muhawwil_nusus::tarkeeb::rakkib_luba`, which writes
/// `game/tl/arabic/` and the generated `.rpy` that selects the language and sets
/// the direction; the `taarib_renpy` package beside it is the one script-engine
/// component that is genuinely staged, because it is Python source copied
/// straight out of `adapters-script/`. On a Ren'Py that shapes, the font arrives
/// with it: the staging matrix carries an Arabic face into the Ren'Py component
/// and the generated `.rpy` registers it by name.
///
/// **The paragraph that used to stand here said the opposite**, and it was true
/// when it was written: `raqqi_nusus` passed `Mawarid::khatt_renpy` as [`None`],
/// so the generated file registered no face and a shaping Ren'Py game came out
/// with no font at all. The arm below has answered [`JahiziyatTashghil::Mukammala`]
/// since the face was staged, and a doc describing the state its own function no
/// longer returns is worse than no doc, because it is the half a reader trusts.
/// What has *not* been done is watching it happen in a game, and
/// [`naqs_renpy_khatt`] says exactly that.
///
/// Below the shaping release there is no partial outcome to report. The engine
/// blits one character at a time, the takeover that would draw the letters
/// instead needs Taarib's native library beside the Python package, and staging
/// copies the package alone.
fn jahiziyat_renpy(muharrik: &Muharrik) -> (JahiziyatTashghil, Hadd) {
    match renpy_yashkul(muharrik) {
        // The one arm in this table that reaches the screen. Everything the
        // tier promises for a shaping Ren'Py engine now arrives: the
        // translation, the language selection, the reading direction, and — as
        // of the staging matrix carrying a face into the Ren'Py component — an
        // Arabic font the generated `.rpy` registers by name.
        Some(true) => (JahiziyatTashghil::Mukammala, naqs_renpy_khatt()),
        Some(false) => (JahiziyatTashghil::Ghaiba, naqs_renpy_bila_tashkeel()),
        None => (JahiziyatTashghil::Ghaiba, naqs_renpy_majhul()),
    }
}

/// The clause that opens the tier's reason when the tier does not run yet.
///
/// Short on purpose. The full account is [`jahiziya`]'s sentence, which the
/// interface gives its own place; this exists so that the paragraph printed
/// directly under the tier name cannot be read as a promise even by somebody who
/// reads nothing else on the screen.
const fn sadr_jahiziya(jahiziya: JahiziyatTashghil) -> Option<(&'static str, &'static str)> {
    match jahiziya {
        JahiziyatTashghil::Mukammala => None,
        JahiziyatTashghil::Naqisa => Some((
            "يعمل جزء من هذا المستوى في هذا الإصدار من تعريب ولا يعمل جزء آخر، والتفصيل \
             أدناه.",
            "Part of this tier works in this build of Taarib and part of it does not; the \
             detail is below.",
        )),
        // Worded so it fits all three tiers: tier 3 shows Arabic over a game
        // rather than inside it, and a sentence that said "inside" would be
        // wrong for exactly the tier that modifies nothing.
        //
        // It says "readably" rather than "will not change", which it used to.
        // For most engines here nothing is written into the game at all, but not
        // for all of them: a GameMaker string pool is rewritten and a Ren'Py
        // translation is installed, and on those two the screen does change — it
        // changes to text the engine cannot draw. Promising an unchanged game
        // there would be false in the one direction a readiness verdict must
        // never be false in, and the arm's own sentence below says which case
        // this game is in.
        JahiziyatTashghil::Ghaiba => Some((
            "لم يكتمل بعدُ في هذا الإصدار من تعريب ما يجعل العربية تظهر مقروءة في هذه اللعبة، \
             والتفصيل أدناه.",
            "What makes Arabic appear readably in this game is not finished in this build of \
             Taarib; the detail is below.",
        )),
    }
}

/// Unity: the managed component now builds, and neither backend is complete.
///
/// This used to say the component "is not built into this package, so nothing
/// enters the game at all", which was true of both backends until the C# was
/// compiled for the first time. It now builds, and the two backends fail at
/// different rungs — so one sentence for both would be false for whichever one
/// it did not describe.
///
/// Both are still [`JahiziyatTashghil::Ghaiba`]: nothing has been shown to reach
/// a screen on either. What changed is *why*, and the reader deserves the real
/// reason, because a component that is absent and one that is present but
/// unreachable call for different reports from them.
fn naqs_unity(muharrik: &Muharrik) -> Hadd {
    match muharrik.khalfiya {
        // `unity/Taarib.Unity.Mono` references BepInEx.Unity.Mono 6.0.0-be.780
        // and `assets/aqfal/qufl_bepinex.json` stages 5.4.23.5 for this backend.
        // Under BepInEx 5 the base types live in assemblies the plugin does not
        // name, so the chainloader cannot load it and `Awake` is never entered.
        KhalfiyaBarmajiya::Mono => hadd(
            "الجزء الذي يعمل داخل لعبة Unity أثناء تشغيلها لم يكتمل في هذا الإصدار من تعريب: \
             المكوّن مبنيّ، لكنه يطلب جيلًا من مُحمّل الإضافات لا تشحنه هذه الحزمة، فلا \
             يُشغّله المُحمّل أصلًا. يمكنك تثبيت الرقعة الآن — تُحفظ ملفاتك الأصلية وتُستعاد \
             كما كانت بالضبط — لكن اللعبة ستبقى بلغتها الأصلية حتى يصل التحديث الذي يُكمله.",
            "The part that runs inside a Unity game while you play is not finished in this \
             build of Taarib: the component is built, but it asks for a generation of the \
             plugin loader this package does not ship, so the loader never starts it. You \
             can install the patch now — your original files are kept and restored exactly \
             — but the game will stay in its original language until the update that \
             completes it arrives.",
        ),
        // Built, staged, and matched to the loader the bundle ships — the
        // IL2CPP lock entries are BepInEx 6.0.0-pre.2, which is the generation
        // the assembly names. What is absent is underneath it: all twenty-nine
        // of `Taarib.Unity.Jisr`'s `DllImport`s bind `taarib_jisr`, which is
        // staging rows B1 to B3 out of the `taarib-jisr` cdylib, and no target
        // build of it is produced. A managed assembly with no native library
        // beneath it enters and then fails at its first call.
        KhalfiyaBarmajiya::Il2cpp => hadd(
            "الجزء الذي يعمل داخل لعبة Unity أثناء تشغيلها لم يكتمل في هذا الإصدار من تعريب: \
             المكوّن مبنيّ ومشحون ويطابق مُحمّل الإضافات، لكن المكتبة الأصليّة التي يستدعيها \
             ليست في هذه الحزمة، فلا يصل النص العربي إلى الشاشة. يمكنك تثبيت الرقعة الآن — \
             تُحفظ ملفاتك الأصلية وتُستعاد كما كانت بالضبط — لكن اللعبة ستبقى بلغتها \
             الأصلية حتى يصل التحديث الذي يُكمله.",
            "The part that runs inside a Unity game while you play is not finished in this \
             build of Taarib: the component is built, shipped, and matched to the plugin \
             loader, but the native library it calls into is not in this package, so no \
             Arabic reaches the screen. You can install the patch now — your original files \
             are kept and restored exactly — but the game will stay in its original language \
             until the update that completes it arrives.",
        ),
        // The probe could not tell which backend this game uses, so neither
        // sentence above can be asserted. Say only what holds for both.
        _ => hadd(
            "الجزء الذي يعمل داخل لعبة Unity أثناء تشغيلها لم يكتمل في هذا الإصدار من تعريب، \
             ولم تتّضح بعدُ طريقة تشغيل شيفرة هذه اللعبة، وهي التي تحدّد أي مكوّن يُثبَّت. \
             يمكنك تثبيت الرقعة الآن — تُحفظ ملفاتك الأصلية وتُستعاد كما كانت بالضبط — لكن \
             اللعبة ستبقى بلغتها الأصلية حتى يصل التحديث الذي يُكمله.",
            "The part that runs inside a Unity game while you play is not finished in this \
             build of Taarib, and how this game's code runs is not yet clear — which is what \
             decides which component would be installed. You can install the patch now — \
             your original files are kept and restored exactly — but the game will stay in \
             its original language until the update that completes it arrives.",
        ),
    }
}

/// Unreal: nothing is written into the game, and the shaping switch is not
/// thrown either.
///
/// This sentence used to claim that installing switches the engine's Arabic
/// shaping on. It does not: `taarib_tathbeet::tarkib::mulhaqat_muharrik`
/// answers `Ok(())` for Unreal without writing anything, and the only code that
/// sets `Slate.DefaultTextShapingMethod` lives behind `--features hamula` and
/// applies at the game's *next* launch. Naming a part that works when none does
/// is worse than naming none, because it sends the reader looking for the half
/// that supposedly succeeded.
fn naqs_unreal() -> Hadd {
    hadd(
        "الجزء الذي يستبدل نصوص أنريل داخل اللعبة لم يكتمل في هذا الإصدار من تعريب: \
         لا يُكتب شيء في اللعبة عند التثبيت، ولا يُفعَّل تشكيل العربية داخل المحرّك. \
         يمكنك تثبيت الرقعة الآن — تُحفظ ملفاتك الأصلية وتُستعاد كما كانت بالضبط — لكن \
         اللعبة ستبقى بلغتها الأصلية حتى يصل التحديث الذي يُكمله.",
        "The part that replaces Unreal's text inside the game is not finished in this build \
         of Taarib: nothing is written into the game at install, and the engine's Arabic \
         shaping is not switched on either. You can install the patch now — your original \
         files are kept and restored exactly — but the game will stay in its original \
         language until the update that completes it arrives.",
    )
}

/// Godot 4: neither the engine extension nor the translation package is written.
fn naqs_godot_arbaa() -> Hadd {
    hadd(
        "الجزء الذي يضع الترجمة داخل غودوت 4 وهي تعمل لم يكتمل في هذا الإصدار من تعريب: \
         لا يُسجَّل امتداد المحرّك ولا تُكتب حزمة الترجمة، فلا شيء يُحمَّل مع اللعبة. \
         ستبقى اللعبة بلغتها الأصلية حتى يصل التحديث الذي يُكمله.",
        "The part that puts the translation inside a running Godot 4 game is not finished in \
         this build of Taarib: the engine extension is not registered and the translation \
         package is not written, so nothing loads with the game. It will stay in its \
         original language until the update that completes it arrives.",
    )
}

/// Godot 3: the module starts, and neither half of its work is handed to it.
///
/// Two seams, and naming only the drawing one — as this used to — would send a
/// reader looking for the translation half that supposedly succeeded.
/// `thabbit_tawseel` and `thabbit_istila`
/// (`crates/taarib-muhawwil-godot/src/bidaya.rs`) have no caller anywhere in the
/// workspace, so `awsil` refuses before a `.translation` resource is written and
/// `sallim` refuses before a detour is installed. The engine loads the library,
/// the library starts, and it is handed neither the patch nor this build's
/// `Font::draw` addresses.
///
/// The round trip through a real Godot 3.6 binary that put Arabic back out of
/// `tr()` drove `TawseelThalith::hayyi` from a test. It proves the delivery is
/// correct; it does not put a caller in the installer.
fn naqs_godot_thalith() -> Hadd {
    hadd(
        "الرسم المباشر فوق نصوص غودوت 3 لم يكتمل في هذا الإصدار من تعريب: تُثبَّت وحدة \
         تعريب داخل اللعبة وتبدأ فعلًا، لكن لا أحد يسلّمها الترجمة ولا مواضع دوال الرسم في \
         هذا البناء بالذات، فتنسحب وتترك اللعبة ترسم نصوصها بنفسها. ستبقى اللعبة بلغتها \
         الأصلية حتى يصل التحديث الذي يُكمله.",
        "Drawing over Godot 3's own text is not finished in this build of Taarib: Taarib's \
         module is installed inside the game and does start, but nothing hands it either the \
         translation or this particular build's drawing addresses, so it stands down and lets \
         the game draw its own text. The game will stay in its original language until the \
         update that completes it arrives.",
    )
}

/// RPG Maker MV and MZ: the data splice has a caller and the plugin has no
/// build, and the install stops on the second.
///
/// This used to say the translation "is not written into the game's data files
/// yet", and that half is no longer true: `tarkeeb::rakkib_luba` splices
/// `data/*.json` and `taarib-tathbeet`'s `nusus` module calls it.
///
/// It does not follow that anything reaches a screen, because the install does
/// not complete. `tarkib::mulhaqat_muharrik` deploys `mulhaq/rpgmaker/{mv,mz}`
/// for this family unconditionally, and `asmaa_mukawwin` raises
/// `MukawwinMafqud` when the component store does not hold it. It does not:
/// `taarib-tajmee` stages row I2 from `target/adapters/rpgmaker/taarib.js`, and
/// nothing in this repository runs `esbuild` over
/// `adapters-script/rpgmaker/taarib.ts` — there is no bundler configuration, no
/// build script and no CI step, only the sequence printed by `taarib-tajmee
/// --help`. The plan is built inside the deployment step, which runs after the
/// splice, so the splice happens and is then rolled back with the failed
/// install.
///
/// Two further defects sit behind that one and are not what stops it. The
/// registration is written with `"parameters":{}`, so a built plugin would read
/// its own defaults; and the plugin is what corrects direction, alignment and
/// window mirroring, which Chromium's `fillText` does not do for the Arabic it
/// otherwise joins correctly.
fn naqs_rpg_maker() -> Hadd {
    hadd(
        "تُكتب الترجمة العربية في ملفات بيانات RPG Maker فعلًا، لكن الوحدة التي تعمل داخل \
         اللعبة — وهي التي تضبط اتجاه الكتابة والمحاذاة ومرايا النوافذ — غير مبنيّة في هذه \
         الحزمة، ويرفض التثبيت المتابعة بدونها. فلن يُترك في لعبتك شيء ولن يتغيّر منها شيء. \
         ستبقى اللعبة بلغتها الأصلية حتى يصل التحديث الذي يبني الوحدة.",
        "The Arabic is genuinely written into RPG Maker's data files, but the module that runs \
         inside the game — the one that sets the reading direction, the alignment and the \
         mirrored window layouts — is not built into this package, and installation refuses to \
         go on without it. So nothing is left in your game and nothing about it changes. The \
         game will stay in its original language until the update that builds the module \
         arrives.",
    )
}

/// RPG Maker VX Ace: the script is never inserted and the glyph pages never made.
///
/// `vxace::rakkib` takes its Ruby payload from its caller and has none;
/// `tarkeeb::rakkib_luba` refuses this family by name rather than routing it
/// through an entry point that carries no payload, and `mulhaqat_muharrik`
/// deploys nothing for it because the Ruby is meant to travel inside the patch.
/// Both halves of the sentence below therefore still hold exactly.
fn naqs_vx_ace() -> Hadd {
    hadd(
        "لا يُدرَج نصّ تعريب داخل أرشيف سكربتات RPG Maker VX Ace بعد، ولا تُولَّد صفحات \
         الحروف العربية التي يحتاجها الرسم. ستبقى اللعبة بلغتها الأصلية حتى يصل التحديث \
         الذي يُكمل ذلك.",
        "Taarib's script is not inserted into RPG Maker VX Ace's script archive yet, and the \
         Arabic glyph pages its drawing needs are not generated. The game will stay in its \
         original language until the update that completes it arrives.",
    )
}

/// Ren'Py on an engine that shapes: complete, and never yet watched working.
///
/// The only arm in the table that answers [`JahiziyatTashghil::Mukammala`], so
/// [`jahiziya`] drops this `Hadd` before it can reach anyone — the contract is
/// that a finished tier carries no gap sentence. It is written and kept anyway,
/// truthfully, because the previous text survived here as a *false* sentence one
/// wiring change away from the screen: it said Taarib places no font inside the
/// game and names none, which stopped being true when the staging matrix took an
/// Arabic face into the Ren'Py component and `tarkib::khutta` began naming it
/// through `ikhtar_khatt_renpy`.
///
/// What remains is not a missing piece but a missing observation. Every part of
/// this path — `game/tl/arabic/`, the generated `.rpy`, `config.language`, the
/// direction and alignment from `sajjil_ittijah`, the `taarib_renpy` package
/// from staging row I3, and now the face — is built and staged; none of it has
/// been seen running inside a Ren'Py game. That is a different claim from "it
/// works" and this says the weaker one.
fn naqs_renpy_khatt() -> Hadd {
    hadd(
        "تُثبَّت ترجمة رِن باي كاملة: النصّ، واختيار اللغة، واتجاه الكتابة والمحاذاة، وخطّ \
         عربي يوضع داخل اللعبة ويُسجَّل باسمه، ومحرّك هذه اللعبة يشكّل العربية بنفسه. ما لم \
         يحدث بعدُ هو أن يُرى ذلك عاملًا داخل لعبة رِن باي حقيقية: كلّ قطعة مبنيّة ومشحونة، \
         ولم تُجرَّب المسيرة كاملة في لعبة تعمل.",
        "Ren'Py's translation is installed in full: the text, the language selection, the \
         reading direction and alignment, and an Arabic font placed inside the game and \
         registered by name, on an engine that shapes Arabic itself. What has not happened \
         yet is anyone watching it work inside a real Ren'Py game: every piece is built and \
         staged, and the whole path has never been run in a game that is playing.",
    )
}

/// Ren'Py below the shaping release: the translation arrives and cannot join.
///
/// Below `dalail::nusus::HADD_TASHKEEL_RENPY` the engine has no `HarfBuzz` and
/// no `FriBidi`, and its text layout is a per-character blit. The adapter's own
/// answer to that is the glyph takeover in `taarib_renpy._rakkib_istila`, which
/// loads `taarib_jisr` from beside the package — and `taarib-tajmee`'s
/// `saf_mulhaq` stages the Python package alone, so the takeover declines by
/// name at every launch and the configuration rung is all that is left.
///
/// So the install completes and the game does change: it shows the Arabic
/// translation as separated letters in the wrong order, which is not Arabic a
/// person can read. Nothing the tier promises reaches the screen.
fn naqs_renpy_bila_tashkeel() -> Hadd {
    hadd(
        "إصدار رِن باي في هذه اللعبة أقدم من أن يشكّل العربية: يرسم الحروف حرفًا حرفًا بلا \
         وصل ولا ترتيب. والجزء الذي يرسمها بدلًا عنه يحتاج مكتبة تعريب الأصليّة بجانب وحدته، \
         وهي ليست في هذه الحزمة. فلو رُكِّبت الترجمة لظهرت حروفًا منفصلة لا تُقرأ عربيةً، \
         ولذلك لا يُوعَد بشيء هنا حتى يصل التحديث الذي يضع المكتبة.",
        "This game's Ren'Py is older than the release that shapes Arabic: it draws letters one \
         at a time, with no joining and no reordering. The part that would draw them instead \
         needs Taarib's own native library beside its module, and that library is not in this \
         package. An installed translation would appear as separated letters that do not read \
         as Arabic, so nothing is promised here until the update that ships the library \
         arrives.",
    )
}

/// Ren'Py whose version nothing bounded: neither story may be told.
///
/// `renpy/__init__.py` unreadable and no `lib/` to list. The probe declined to
/// choose and so does this: a build that shapes and a build that does not need
/// different halves of Taarib, this package carries neither the font for the
/// first nor the native library for the second, and asserting either would be
/// inventing the fact the detector refused to invent.
fn naqs_renpy_majhul() -> Hadd {
    hadd(
        "لم يتبيّن إصدار رِن باي في هذه اللعبة، وعليه يتوقّف ما إذا كان المحرّك يشكّل \
         العربية بنفسه. وهذا الإصدار من تعريب لا يضع خطًّا عربيًا داخل اللعبة ولا يحمل الجزء \
         الذي يرسم الحروف بدلًا عن المحرّك، فلا يمكن أن يُوعَد بعربية مقروءة هنا حتى يصل \
         التحديث الذي يُكمل الجزأين.",
        "This game's Ren'Py version could not be established, and whether the engine shapes \
         Arabic itself depends on it. This build of Taarib neither places an Arabic font inside \
         the game nor carries the part that draws the letters in the engine's place, so no \
         readable Arabic can be promised here until the update that completes both arrives.",
    )
}

/// GameMaker: the string pool is rewritten and the glyph pages are not.
///
/// The writer is wired in now — `tarkeeb::rakkib_gamemaker` reads `data.win`,
/// replaces the pool entries the patch has Arabic for and writes the container
/// back — so the old sentence, which said nothing was written, is wrong twice
/// over.
///
/// It is still [`JahiziyatTashghil::Ghaiba`], and the reason is the font path.
/// A GameMaker font is a **baked glyph table**: the container ships pictures of
/// letters and an index from character codes to pictures, there is no font file
/// to swap and no shaping stage to configure. `rakkib_gamemaker` calls
/// `badil_nass` and `uktub` and nothing else — not `sajjil_ashkal`, not
/// `nass_manqul`, not `istabdil_khatt`, not `alhiq_safha`. So the pool now holds
/// logical Arabic that the game's own `FONT` chunk has no picture for, in an
/// engine that neither joins nor reorders. The write succeeds and the text it
/// wrote cannot be drawn.
///
/// That is why the verdict is the one that withholds the one-button run rather
/// than the one that offers it. A run that ended here would hand somebody a game
/// whose menus had gone blank.
fn naqs_gamemaker() -> Hadd {
    hadd(
        "يرسم GameMaker نصوصه من صفحات حروف جاهزة مخزّنة داخل ملف بياناته، ولا يرسم من غيرها. \
         يستطيع هذا الإصدار من تعريب أن يكتب النصّ العربي في ذلك الملف، ولا يستطيع أن يولّد \
         صفحات الحروف العربية التي يقرأ منها المحرّك. فالنصّ المكتوب يخرج فراغًا لا عربية، \
         ولهذا لا يُعرَض التعريب على هذه اللعبة أصلًا. ستبقى كما هي حتى يصل التحديث الذي \
         يولّد الصفحات.",
        "GameMaker draws its text from prebuilt glyph pages stored inside its data file, and \
         from nothing else. This build of Taarib can write the Arabic into that file, and it \
         cannot generate the Arabic glyph pages the engine reads from. Written text therefore \
         comes out blank rather than Arabic, which is why Arabization is not offered for this \
         game at all. It stays as it is until the update that generates the pages arrives.",
    )
}

/// Capcom BIO4: the link is found, and nothing yet walks across it.
///
/// The three pieces are real and are worth naming, because "nothing exists" would
/// now be wrong three times over. `taarib-istikhraj`'s `qamus` module reads all
/// eight of the game's dictionaries and rebuilds them byte for byte, so the text
/// comes out and could go back in. `taarib-muhawwil-bio4` reads the `.fnt`
/// metrics, the embedded TPL, the cell grid and the `ImagePack` atlas, and can
/// build a font into them, so the letters could be drawn. And that crate's
/// `kharita` module now holds the piece that used to be missing: which code point
/// selects which cell is a flat array inside `bio4.exe`, one per language, whose
/// index *is* the cell — 260 entries at `0x00C0_CE18` for the Latin builds, read
/// out of the routine at `0x006A_7F50` and confirmed against every code point in
/// all eight shipped dictionaries.
///
/// So the reason this verdict withholds the run has changed, and the sentence has
/// to change with it. What is missing is no longer knowledge, it is plumbing:
/// nothing in `taarib-muhawwil-nusus` routes this family — `tarkeeb::rakkib_luba`
/// has no arm for it — `taarib-tathbeet`'s `tarkib` groups BIO4 with the engines
/// that get no additive step, and `taarib-muhawwil-bio4`'s `naql` still hands out
/// cells `1, 2, 3, …` where five Latin indices can never be asked for. No install
/// currently touches one of these games at all.
///
/// That is why the verdict is still the one that withholds the one-button run. A
/// run that ended here would change nothing, and offering it would be promising a
/// pipeline that is not built yet — which is a different and smaller thing than
/// not knowing how the engine works.
fn naqs_bio4() -> Hadd {
    hadd(
        "يرسم محرّك هذه اللعبة نصوصه من صفحات حروف جاهزة مرسومة داخل ملفاتها، ويطلب كل صورة \
         برقمٍ في جدولٍ داخل ملف اللعبة التنفيذي نفسه. صار هذا الجدول معروفًا عند تعريب، \
         ويعرف تعريب كيف يقرأ نصوص اللعبة ويعيد كتابتها، ويعرف كيف يبني صفحات الحروف \
         العربية — لكن ما يربط هذه الثلاثة ببعضها لم يُبنَ بعد، فلا يمرّ التركيب على هذه \
         اللعبة أصلًا. فلن يُكتب في ملفاتها شيء ولن يتغيّر منها شيء، حتى يصل التحديث الذي \
         يبني هذا الربط.",
        "This game's engine draws its text from prebuilt glyph pages painted inside its own \
         files, and asks for each picture by a number in a table inside the game's own \
         executable. Taarib now knows that table, it can read the game's text and write it \
         back, and it can build the Arabic glyph pages — what does not exist yet is the piece \
         that joins those three together, so no installation reaches this game at all. Nothing \
         is written into its files and nothing about it changes, until the update that builds \
         that piece arrives.",
    )
}

/// Electron: the adapter is wired in and refuses this package by name.
///
/// The one arm whose gap the adapter itself already states.
/// `tarkeeb::rakkib_ghilaf` is reached from the install pipeline and returns
/// `HimlMarfud` when `Mawarid::tashghil_ghilaf` is [`None`], because a
/// translation table injected into somebody's `app.asar` with no runtime to read
/// it changes nothing on screen. It is [`None`]: staging row I1 comes from
/// `target/adapters/electron/taarib.js`, and nothing in this repository runs
/// `esbuild` over `adapters-script/electron/taarib.ts`.
///
/// The refusal travels out of `raqqi_nusus` as `NususMarfuda` and stops the
/// install, so the sentence says the install stops rather than offering one.
fn naqs_electron() -> Hadd {
    hadd(
        "وحدة التشغيل التي تعرّب واجهة اللعبة داخل المتصفح المدمج غير مبنيّة في هذه الحزمة، \
         ويرفض تعريب المتابعة بدونها بدل أن يحقن في حزمة اللعبة جدول ترجمة لا يقرؤه أحد. \
         فلن يُكتب في لعبتك شيء ولن يتغيّر منها شيء، حتى يصل التحديث الذي يبني الوحدة.",
        "The runtime module that Arabizes this game's interface inside its embedded browser is \
         not built into this package, and Taarib refuses to go on without it rather than inject \
         a translation table into the game's bundle that nothing will read. So nothing is \
         written into your game and nothing about it changes, until the update that builds the \
         module arrives.",
    )
}

/// The overlay: it attaches, it can draw, and it is given nothing to draw.
///
/// Two of this row's three breaks are closed. `taarib-tabaqa`'s `talqeem` module
/// builds a real draw batch and uploads a real atlas, proved by rendering
/// through the production path against a software rasterizer rather than by
/// argument. The third is untouched and is sufficient on its own:
/// `Tabaqa::iltaqit` has no caller, the capture and OCR modules have no inbound
/// edge from the bootstrap, and there is no worker thread — so nothing produces
/// the lines the batch builder consumes.
///
/// The sentence therefore names the reading half specifically. Saying "it draws
/// nothing" would now be wrong about the part that works, and would send a
/// reader to the renderer instead of to the missing source.
fn naqs_tabaqa() -> Hadd {
    hadd(
        "طبقة الترجمة تفتح مع اللعبة وتلتصق بصورتها فعلًا وتستطيع الرسم فوقها، لكن لا شيء \
         يزوّدها بالنصّ بعد في هذا الإصدار من تعريب: الجزء الذي يقرأ ما على الشاشة ويسلّمه \
         إليها لم يكتمل، فتبقى فارغة. لن يظهر شيء فوق اللعبة حتى يصل التحديث الذي يُكمله.",
        "The translation overlay does open with the game, attach to its picture and draw over \
         it, but nothing feeds it any text yet in this build of Taarib: the part that reads \
         what is on screen and hands it over is not finished, so it stays empty. Nothing will \
         appear over the game until the update that completes it arrives.",
    )
}

/// A newly-named in-house engine: nothing runs, and the gap has two halves.
///
/// The first half is the overlay's, and it is exactly [`naqs_tabaqa`]'s: the
/// layer attaches and draws and nothing feeds it text. The second half is this
/// engine's own, and it is what a reader of a *named* engine's report will
/// actually want — Taarib knows what this game is, and the reason that changes
/// nothing today is that there is no reader for the containers its text sits in
/// and no adapter that loads into its process.
///
/// Naming the second half matters more here than anywhere else in this table. A
/// player who sees their game identified as Frostbite or as RAGE and then reads
/// that nothing happens is owed the difference between "this engine cannot be
/// Arabized" and "this engine has not been Arabized yet" — the first is false
/// and the second is a thing an update changes.
fn naqs_khassa(aila: AilatMuharrik) -> Hadd {
    let (ayn_arabi, ayn_injilizi) = khazinat_nusus(aila)
        .unwrap_or(("داخل ملفّات المحرّك نفسها", "inside the engine's own files"));
    let ism = aila.ism();
    hadd(
        format!(
            "يعرف تعريب أن محرّك هذه اللعبة {ism}، لكن لا شيء في هذا الإصدار يدخل إليه: نصوص \
             اللعبة {ayn_arabi} ولا يوجد قارئ لها، ولا توجد وحدة تعمل داخل هذه اللعبة أثناء \
             تشغيلها. وطبقة الترجمة تفتح مع اللعبة وتلتصق بصورتها وتستطيع الرسم فوقها، لكن \
             الجزء الذي يقرأ ما على الشاشة ويسلّمه إليها لم يكتمل، فتبقى فارغة. لن يُكتب في \
             لعبتك شيء ولن يتغيّر منها شيء حتى يصل التحديث الذي يبني أحد الطرفين — والمعرفة \
             باسم المحرّك هي أول خطوة في ذلك الطريق، لا نهايته."
        ),
        format!(
            "Taarib knows this game's engine is {ism}, and nothing in this build gets inside \
             it: the game's text sits {ayn_injilizi} with no reader for it, and there is no \
             module that runs inside this game while you play. The translation overlay does \
             open with the game, attach to its picture and draw over it, but the part that \
             reads what is on screen and hands it over is not finished, so it stays empty. \
             Nothing is written into your game and nothing about it changes until the update \
             that builds one of those two arrives — and naming the engine is the first step \
             along that road rather than the end of it."
        ),
    )
}

/// Everything that will not work for this game, named specifically.
///
/// The honest part of the report, and the part a user judges the product on. A
/// limitation nobody can act on is not a limitation, it is a disclaimer: "some
/// text may remain untranslated" is worthless, while "text drawn inside images
/// will not be translated" tells a player exactly which parts of their game
/// will stay in the original language and why no update will change it.
///
/// Ordered by what it costs the user: what stops Arabization outright, then
/// what changes how it is produced, then what will look different, then the two
/// that are true of every patched game. The notes about the identification
/// itself come last, because they are about Taarib rather than about the game.
///
/// Safety limitations are not here. They come from the launcher's own metadata
/// rather than from the engine, so [`taqreer`] adds them, and when a game is
/// refused they are the *only* thing the report carries.
#[must_use]
pub fn hudud(muharrik: &Muharrik, tabaqa: Tabaqa) -> Vec<Hadd> {
    let mut hudud: Vec<Hadd> = Vec::new();
    match tabaqa {
        Tabaqa::TarjamaFawqiya => hudud_tabaqa(muharrik, &mut hudud),
        Tabaqa::Kamil | Tabaqa::RasmMubashir => {
            // Capcom BIO4 is excluded because its text systems are not empty —
            // they are unnameable. [`ItarNusus`] has no value for a dictionary
            // file plus a baked glyph atlas, so the list is empty for a reason
            // that has nothing to do with the game, and this sentence would tell
            // the player their text will be captured off the screen when the
            // family's own arm of [`hudud_aila`] has already said where it
            // actually lives.
            if muharrik.itarat.is_empty() && muharrik.aila != AilatMuharrik::Bio4 {
                hudud.push(hadd_bila_nusus());
            }
            if shuhida(muharrik, alamat::TASHFEER) {
                hudud.push(hadd_tashfeer());
            }
            hudud_aila(muharrik, &mut hudud);
            if tabaqa == Tabaqa::RasmMubashir {
                hudud.push(hadd_rasm_mubashir());
            }
            if maghlufa(muharrik) {
                hudud.push(hadd_ghilaf());
            }
            hudud.push(hadd_suwar());
            hudud.push(hadd_sawt());
        },
    }
    hudud_thiqa(muharrik, &mut hudud);
    hudud
}

/// The limitations that belong to this particular engine.
fn hudud_aila(muharrik: &Muharrik, hudud: &mut Vec<Hadd>) {
    match muharrik.aila {
        AilatMuharrik::Unity => hudud_unity(muharrik, hudud),
        AilatMuharrik::Unreal => {
            if shuhida(muharrik, alamat::NUSUS_DAKHIL_HAWIYA) {
                hudud.push(hadd(
                    "لا تشحن هذه اللعبة أي ملف ترجمة سائب، فنصوصها كلها داخل حاويات \
                     اللعبة. سيفتح تعريب الحاويات ليستخرجها، وإن تعذّر ذلك التُقطت \
                     النصوص أثناء اللعب واكتمل التعريب تدريجيًا كلما وصلت إلى شاشات \
                     جديدة.",
                    "This game ships no loose localization file, so all of its text sits \
                     inside the game's own containers. Taarib opens them to extract it, \
                     and where that is not possible the text is captured while you play \
                     instead, filling in as you reach new screens.",
                ));
            }
            hudud.push(hadd(
                "يشكّل أنريل نصوص الواجهة بنفسه. أما النصوص التي ترسمها اللعبة مباشرة \
                 على أسطح داخل المشهد فقد تبقى بلا تشكيل، لأنها لا تمر بنظام النصوص \
                 نفسه.",
                "Unreal shapes interface text itself. Text the game draws straight onto \
                 surfaces inside the scene may stay unshaped, because it does not pass \
                 through the same text system.",
            ));
        },
        AilatMuharrik::Godot => {
            if ladayh(muharrik, ItarNusus::GodotRichText) {
                hudud.push(hadd(
                    "وسوم التنسيق داخل النصوص الغنية — الألوان والروابط وتغيير الخط — \
                     يُعاد ترتيبها مع النص العربي، وقد يقع بعضها في موضع يختلف قليلًا \
                     عن الأصل بسبب اختلاف اتجاه القراءة.",
                    "Formatting tags inside rich text — colours, links, font changes — \
                     are reordered along with the Arabic, and some may land in a \
                     slightly different place than in the original because the reading \
                     direction differs.",
                ));
            }
        },
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz => {
            hudud.push(hadd(
                "الرسائل التي تبنيها اللعبة من قطع صغيرة أثناء اللعب — اسم عنصر داخل \
                 جملة، رقم داخل عبارة — قد تظهر مركّبة بترتيب خاطئ، وتُعالَج حالةً بحالة \
                 داخل ملف الترجمة.",
                "Messages the game assembles from small fragments while it runs — an \
                 item name inside a sentence, a number inside a phrase — can come out \
                 in the wrong order, and are handled case by case in the translation \
                 file.",
            ));
            hudud.push(hadd(
                "الإضافات التي تكتب النص على الشاشة بنفسها بدل المرور عبر نوافذ \
                 الرسائل قد لا يبلغها تعريب، وستبقى نصوصها بلغتها الأصلية.",
                "Plugins that draw text on screen themselves instead of going through \
                 the message windows may be out of Taarib's reach, and their text will \
                 stay in the original language.",
            ));
        },
        AilatMuharrik::RpgMakerVxAce => {
            hudud.push(hadd(
                "يرسم RPG Maker VX Ace نصوصه بخط النظام داخل نوافذ ثابتة العرض. سيلفّ \
                 تعريب الأسطر العربية لتناسبها، وقد تحتاج بعض النوافذ سطرًا إضافيًا \
                 مقارنة بالأصل.",
                "RPG Maker VX Ace draws its text with a system font inside fixed-width \
                 windows. Taarib wraps the Arabic lines to fit them, and some windows \
                 may need one line more than the original.",
            ));
        },
        AilatMuharrik::Renpy => {
            hudud.push(hadd(
                "النصوص المكتوبة داخل شيفرة بايثون بدل ملفات الحوار لا يلتقطها نظام \
                 الترجمة في رِن باي، وستُلتقط أثناء اللعب بدلًا من ذلك.",
                "Text written inside Python code rather than in dialogue files is not \
                 picked up by Ren'Py's own translation system, and is captured while \
                 you play instead.",
            ));
        },
        AilatMuharrik::GameMaker => {
            hudud.push(hadd(
                "يرسم GameMaker النصوص من صفحات حروف جاهزة داخل ملف بياناته، وسيولّد \
                 تعريب صفحات عربية ويضعها في الملف نفسه. الحروف التي لا يغطيها الخط \
                 الذي تختاره لن تُرسم إطلاقًا، فاختر خطًا عربيًا كاملًا.",
                "GameMaker draws text from prebuilt glyph pages inside its data file, \
                 and Taarib generates Arabic pages and writes them into the same file. \
                 Characters the font you choose does not cover will not be drawn at \
                 all, so choose a complete Arabic font.",
            ));
        },
        AilatMuharrik::Electron => {
            hudud.push(hadd(
                "تُعاد حزمة اللعبة بناءً بعد التعديل، فيحتاج التثبيت مساحة حرة بحجم \
                 الحزمة. وأي تحديث للعبة يستبدل الحزمة، وعندها يعيد تعريب تثبيت نفسه \
                 تلقائيًا.",
                "The game's bundle is rebuilt after it is modified, so installation \
                 needs free space the size of the bundle. Any game update replaces the \
                 bundle, and Taarib reinstalls itself automatically when that happens.",
            ));
            if ladayh(muharrik, ItarNusus::Canvas) {
                hudud.push(hadd_canvas());
            }
        },
        AilatMuharrik::Bio4 => {
            hudud.push(hadd(
                "خطوط هذه اللعبة صفحات حروف مرسومة مسبقًا داخل ملفاتها، وسيولّد تعريب صفحات \
                 عربية بديلة. الحروف التي لا يغطيها الخط الذي تختاره لن تُرسم إطلاقًا، \
                 فاختر خطًا عربيًا كاملًا. وعدد الخانات في كل صفحة محدود بما في اللعبة، فقد \
                 لا تتّسع كل الأشكال العربية في شاشة واحدة.",
                "This game's fonts are glyph pages painted in advance inside its own files, \
                 and Taarib generates Arabic pages to replace them. Characters the font you \
                 choose does not cover will not be drawn at all, so choose a complete Arabic \
                 font. The number of cells in a page is fixed by what the game shipped, so \
                 not every Arabic letter shape may fit on a given screen.",
            ));
        },
        // The six in-house engines are here with the unrecognised family and
        // for the same structural reason: this function is only reached at tier
        // 1 and tier 2, [`tabaqa_khassa`] puts every one of them at tier 3, and
        // a limitation about replacing text inside a game nothing is written
        // into would be a sentence about work that does not happen. What the
        // player is told instead is [`naqs_khassa`], which names the engine and
        // says what is missing.
        AilatMuharrik::Frostbite
        | AilatMuharrik::BlackSpace
        | AilatMuharrik::Alchemy
        | AilatMuharrik::Dantelion
        | AilatMuharrik::Rage
        | AilatMuharrik::Snowdrop
        | AilatMuharrik::Majhul => {},
    }
    if muharrik.aila != AilatMuharrik::Electron && ladayh(muharrik, ItarNusus::Canvas) {
        hudud.push(hadd_canvas());
    }
}

/// Unity's own limitations, which depend on the backend and on which text
/// systems were actually found.
fn hudud_unity(muharrik: &Muharrik, hudud: &mut Vec<Hadd>) {
    if muharrik.khalfiya == KhalfiyaBarmajiya::Majhula {
        hudud.push(hadd(
            "لم يتّضح ما إذا كانت شيفرة هذه اللعبة تعمل على Mono أم على IL2CPP، وهما \
             يحتاجان مكوّنين مختلفين تمامًا. يُحسم هذا عند التثبيت، وإن لم يُحسم فلن \
             يُثبَّت شيء، لأن تثبيت المكوّن الخاطئ يمنع اللعبة من العمل.",
            "Whether this game's code runs on Mono or on IL2CPP could not be \
             determined, and the two need entirely different components. It is settled \
             at install time, and if it cannot be, nothing is installed — because \
             installing the wrong component stops the game from starting.",
        ));
    }

    // Not gated on the backend. The marker is only ever written by the IL2CPP
    // path, and a game that shipped both Mono and IL2CPP artefacts resolves to
    // an undetermined backend while still needing this sentence.
    if shuhida(muharrik, alamat::MITA_MAJHULA) {
        hudud.push(hadd(
            "بُنيت هذه اللعبة بـ IL2CPP، وتعذّرت قراءة بياناتها الوصفية — إمّا لأن \
             إصدارها غير معروف لهذا البناء أو لأنها مشفَّرة. سيُبحث عن دوال النصوص \
             ببصماتها داخل الملف التنفيذي، وقد لا تُعثر كل أنظمة النص، وما لا يُعثر عليه \
             يبقى بلغته الأصلية.",
            "This game is built with IL2CPP and Taarib could not read its metadata — \
             either its version is unknown to this build, or it is encrypted. Its text \
             functions are located by scanning the executable for their signatures \
             instead; not every text system may be found, and whatever is not found \
             stays in its original language.",
        ));
    }

    if ladayh(muharrik, ItarNusus::NGui) || ladayh(muharrik, ItarNusus::FairyGui) {
        hudud.push(hadd(
            "تعتمد نصوص NGUI وFairyGUI على صفحات حروف تُبنى مسبقًا داخل اللعبة، وسيبني \
             تعريب صفحات عربية بديلة. الحروف التي لا يغطيها الخط الذي تختاره لن تُرسم، \
             فاختر خطًا عربيًا كاملًا.",
            "NGUI and FairyGUI text is drawn from glyph pages built into the game in \
             advance, and Taarib builds Arabic pages to replace them. Characters the \
             font you choose does not cover will not be drawn, so choose a complete \
             Arabic font.",
        ));
    }

    if ladayh(muharrik, ItarNusus::TextMesh) {
        hudud.push(hadd(
            "النصوص الموضوعة داخل المشهد ثلاثي الأبعاد — اللافتات والشاشات التي تراها \
             في العالم — تُعرَّب أيضًا، لكن اتساع الكلمة العربية قد يخرجها عن حدود اللوح \
             الذي رُسمت عليه.",
            "Text placed inside the 3D scene — the signs and screens you see in the \
             world — is Arabized too, but Arabic words are wider and can run past the \
             edge of the surface they are drawn on.",
        ));
    }
}

/// What the overlay tier means in practice, which is the honest description of
/// a reading aid.
fn hudud_tabaqa(muharrik: &Muharrik, hudud: &mut Vec<Hadd>) {
    hudud.push(hadd(
        "لن تُعدَّل اللعبة إطلاقًا. يقرأ تعريب ما يظهر على الشاشة ويعرض العربية فوقه، \
         فالنتيجة وسيلة قراءة لا ترجمة مثبّتة: النص لا يُحفظ داخل اللعبة، ويختفي بمجرد \
         إغلاق تعريب.",
        "The game is not modified at all. Taarib reads what is on screen and shows \
         Arabic over it, so the result is a reading aid and not an installed \
         translation: the text is not saved into the game, and it disappears the moment \
         Taarib is closed.",
    ));
    hudud.push(hadd(
        "قراءة الشاشة تعتمد على وضوح النص المعروض. النصوص الصغيرة أو المتحركة أو \
         الموضوعة فوق خلفية مزدحمة قد تُقرأ خطأ أو لا تُقرأ أصلًا.",
        "Reading the screen depends on how clearly the text is displayed. Small text, \
         moving text, or text over a busy background may be read wrongly or not read at \
         all.",
    ));
    hudud.push(hadd(
        "تُرسم الطبقة فوق الصورة، وقد تحجب جزءًا مما تحتها. يمكنك تحريك مناطق العرض أو \
         تصغيرها من إعدادات الطبقة في أي وقت.",
        "The overlay draws on top of the picture and may cover part of what is under \
         it. You can move or shrink its regions from the overlay's settings at any \
         time.",
    ));
    if muharrik.rusum.is_empty() {
        hudud.push(hadd(
            "لم يتحدَّد نظام الرسم الذي تستخدمه هذه اللعبة، وطبقة الترجمة تحتاجه لتلتصق \
             بالصورة. سيُحدَّد عند أول تشغيل، وإن لم يُحدَّد فلن تعمل الطبقة مع هذه \
             اللعبة.",
            "The graphics API this game uses was not determined, and the overlay needs \
             it in order to attach to the picture. It is determined on first launch, \
             and if it cannot be, the overlay will not work for this game.",
        ));
    }
}

/// The notes about the identification rather than about the game.
fn hudud_thiqa(muharrik: &Muharrik, hudud: &mut Vec<Hadd>) {
    if mutaarid(muharrik) {
        hudud.push(hadd(
            "عُثر داخل مجلد اللعبة على أدلة تشير إلى أكثر من محرّك واحد. اعتُمد أقواها \
             وبقيت الأدلة الأخرى كما هي في تقرير التشخيص، فإن كان الاختيار خاطئًا فهو \
             مرئي ويمكن تصحيحه.",
            "Evidence pointing at more than one engine was found inside the game's \
             folder. The strongest was adopted and the rest are kept as they were in \
             the diagnostics report, so a wrong choice is visible and can be corrected.",
        ));
    }
    if muharrik.thiqa < HADD_THIQA_MUNKHAFIDA {
        hudud.push(hadd(
            "الأدلة على محرّك هذه اللعبة ضعيفة، وقد يكون التعرّف عليه خاطئًا. إن بدا \
             التعريب في غير محلّه فأرسل تقرير التشخيص، وستُصحَّح قاعدة التعرّف في تحديث \
             لاحق.",
            "The evidence for this game's engine is weak, and the identification may be \
             wrong. If the Arabization looks out of place, send the diagnostics report \
             and the detection rule will be corrected in a later update.",
        ));
    }
}

/// Text that is not text. True of every engine and every tier that patches, and
/// the single most common thing a user is surprised by.
fn hadd_suwar() -> Hadd {
    hadd(
        "النصوص المرسومة داخل الصور لن تُترجم: الشعارات ولافتات المستويات والواجهات \
         المصمَّمة كصور جاهزة ليست نصًا يمكن استبداله، بل بكسلات.",
        "Text drawn inside images will not be translated: logos, level signage, and \
         interface pieces designed as finished pictures are not replaceable text but \
         pixels.",
    )
}

/// Recorded audio and burned-in video subtitles.
fn hadd_sawt() -> Hadd {
    hadd(
        "الحوار المنطوق يبقى بلغته الأصلية، والترجمة المحروقة داخل مقاطع الفيديو لا يمكن \
         استبدالها لأنها جزء من الصورة نفسها.",
        "Spoken dialogue stays in its original language, and subtitles burned into video \
         clips cannot be replaced because they are part of the picture itself.",
    )
}

/// The engine is supported and its text systems were not found.
fn hadd_bila_nusus() -> Hadd {
    hadd(
        "لم يُتعرَّف على أي نظام نصوص داخل هذه اللعبة رغم أن محرّكها مدعوم. سيلتقط تعريب \
         النصوص أثناء اللعب بدل استخراجها من الملفات، فما لا يظهر على الشاشة لن يُترجَم \
         قبل أن تصل إليه.",
        "No text system could be identified inside this game even though its engine is \
         supported. Taarib will capture text while you play instead of extracting it \
         from the files, so anything that never appears on screen will not be translated \
         until you reach it.",
    )
}

/// What tier 2 actually means for the look of the result.
fn hadd_rasm_mubashir() -> Hadd {
    hadd(
        "سيرسم تعريب نصوص هذه اللعبة بنفسه بدل أن يستبدلها داخل المحرّك. المؤثرات التي \
         يضيفها المحرّك على نصوصه — الحدود والتدرّجات وحركة الحروف — يعيد تعريب إنتاجها \
         برسمه هو، وقد تختلف عن الأصل اختلافًا طفيفًا.",
        "Taarib will draw this game's text itself instead of replacing it inside the \
         engine. Effects the engine applies to its own text — outlines, gradients, \
         per-character animation — are reproduced by Taarib's renderer and may differ \
         slightly from the original.",
    )
}

/// An encrypted container, which needs a key only the user can provide.
fn hadd_tashfeer() -> Hadd {
    hadd(
        "أرشيف هذه اللعبة مشفَّر، ولا يمكن قراءة نصوصه بدون مفتاح التشفير. سيطلب تعريب \
         المفتاح منك ولن يحاول كسره، وبدونه لن يعمل التعريب على هذه اللعبة.",
        "This game's archive is encrypted and its text cannot be read without the \
         encryption key. Taarib will ask you for the key and will not attempt to break \
         it; without it, Arabization will not work for this game.",
    )
}

/// A game that arrived wrapped in a browser shell.
fn hadd_ghilaf() -> Hadd {
    hadd(
        "اللعبة معبّأة داخل قشرة متصفح حول محرّكها الحقيقي. سيُعرَّب المحرّك الداخلي ثم \
         يُعاد بناء الحزمة، فيحتاج التثبيت مساحة حرة بحجم الحزمة ويستغرق وقتًا أطول من \
         المعتاد.",
        "The game is packaged inside a browser shell wrapped around its real engine. \
         Taarib arabizes the inner engine and then rebuilds the bundle, so installation \
         needs free space the size of the bundle and takes longer than usual.",
    )
}

/// Text a game paints onto a canvas itself, where the browser's own shaping
/// never runs.
fn hadd_canvas() -> Hadd {
    hadd(
        "النصوص التي ترسمها اللعبة على لوحة رسم بنفسها لا تمر بتشكيل المتصفح، فيشكّلها \
         تعريب قبل رسمها؛ قد يختلف تباعد الحروف قليلًا عمّا تراه في النسخة الأصلية.",
        "Text the game paints onto a canvas itself never passes through the browser's \
         shaping, so Taarib shapes it before it is drawn; letter spacing may differ \
         slightly from what you see in the original.",
    )
}

/// The refusal. One sentence, unambiguous, and never softened.
fn hadd_rafd(himaya: &str) -> Hadd {
    hadd(
        format!(
            "هذه اللعبة محمية بنظام مكافحة غش ({himaya}). لن يعدّل تعريب أي ملف فيها ولن \
             يعرض شيئًا فوقها، لأن أي تعديل أو إضافة داخل لعبة محمية قد يكلّفك حظرًا \
             دائمًا لحسابك ومكتبتك معه. لا يوجد في تعريب أي خيار لتجاوز هذا الرفض."
        ),
        format!(
            "This game is protected by an anti-cheat system ({himaya}). Taarib will not \
             modify any of its files and will not draw anything over it, because any \
             modification or addition inside a protected game can cost you a permanent \
             ban on your account and your library with it. There is no setting anywhere \
             in Taarib that overrides this refusal."
        ),
    )
}

/// The launcher entry that is not a game at all.
///
/// Worded as the core words it, deliberately to the letter: the same entry is
/// refused in two places and a reader who sees both must not have to wonder
/// whether they are two findings.
fn hadd_laysat_luba(naw: &str) -> Hadd {
    hadd(
        format!(
            "هذا المدخل ليس لعبة؛ يصنّفه المتجر على أنه {naw}. لا يُعرَّب إلا ما هو لعبة، \
             ولن يُكتب في هذا المجلّد شيء."
        ),
        format!(
            "This entry is not a game — the launcher classifies it as {naw}. Taarib \
             arabizes games, and nothing will be written into this folder."
        ),
    )
}

/// What the launcher called this entry, when it called it something other than a
/// game.
fn naw_ghayr_luba(simat: &[SimatLuba]) -> Option<String> {
    simat.iter().find_map(|sima| match sima {
        SimatLuba::LaysatLuba(naw) => Some(naw.clone()),
        _ => None,
    })
}

/// Multiplayer without anti-cheat: a warning rather than a refusal.
fn hadd_jamai() -> Hadd {
    hadd(
        "تدعم هذه اللعبة اللعب عبر الإنترنت. لم يُكتشف فيها نظام مكافحة غش، لكن بعض \
         الخوادم تعامل الملفات المعدَّلة كمخالفة، وسيطلب تعريب موافقتك الصريحة قبل \
         التثبيت في كل مرة.",
        "This game has online play. No anti-cheat was detected in it, but some servers \
         treat modified files as a violation, so Taarib will ask for your explicit \
         consent before every installation.",
    )
}

/// A game that runs behind Proton, Wine or Rosetta.
fn hadd_tawafuq(tabaqa: &str) -> Hadd {
    hadd(
        format!(
            "تعمل هذه اللعبة عبر طبقة توافق ({tabaqa})، لذلك يُثبَّت تعريب داخل بيئة \
             التوافق نفسها لا في نظامك مباشرة. إن غيّرت إصدار الطبقة أو حذفت البيئة، \
             أعد التثبيت."
        ),
        format!(
            "This game runs through a compatibility layer ({tabaqa}), so Taarib installs \
             inside that layer's own prefix rather than into your system. If you change \
             the layer's version or delete the prefix, install again."
        ),
    )
}

/// How good the result is expected to be, from what is actually reachable.
///
/// Never from the engine's name. A Unity game whose TextMeshPro was found is
/// excellent; a Unity game where nothing at all could be identified is limited,
/// on the same tier 1 engine, because the tier says what Taarib is *allowed* to
/// do and this says what there is to do it to.
///
/// The adjustments below all read as `max`, which looks backwards until you see
/// the declaration order of [`JawdaMutawaqqaa`]: `Mumtaza` is first and
/// `Mahduda` is last, so the greater value is the worse outcome and `max` is
/// "no better than this".
///
/// `jahiziya` is the floor under all of it. An engine whose adapter does not run
/// produces no Arabic at all, and a report that called that outcome excellent
/// because the game happens to ship TextMeshPro would be describing a capability
/// nobody has. A partly-delivered tier is capped rather than floored, for the
/// same reason one step down: [`JawdaMutawaqqaa::Mumtaza`] promises that
/// everything the player reads is correctly shaped Arabic, and a verdict of
/// [`JahiziyatTashghil::Naqisa`] is the report saying a named part of it is not.
#[must_use]
pub fn jawda(muharrik: &Muharrik, tabaqa: Tabaqa, jahiziya: JahiziyatTashghil) -> JawdaMutawaqqaa {
    if !jahiziya.tasil() {
        return JawdaMutawaqqaa::Mahduda;
    }
    if tabaqa == Tabaqa::TarjamaFawqiya {
        return JawdaMutawaqqaa::Mahduda;
    }
    if muharrik.itarat.is_empty() {
        return JawdaMutawaqqaa::Mahduda;
    }

    let mut jawda = jawda_min_itarat(muharrik);

    // A tier that is only partly delivered cannot be excellent, whatever the
    // game ships. `Mumtaza` is "everything the player reads will be Arabic,
    // correctly shaped", and `Naqisa` is the report saying in the same breath
    // that a named part of it does not arrive.
    if jahiziya == JahiziyatTashghil::Naqisa {
        jawda = jawda.max(JawdaMutawaqqaa::Jayida);
    }

    // Tier 2 draws the text rather than handing it to the engine, so the
    // engine's own text effects are reproduced rather than applied. Close, and
    // never indistinguishable.
    if tabaqa == Tabaqa::RasmMubashir {
        jawda = jawda.max(JawdaMutawaqqaa::Jayida);
    }
    if muharrik.aila == AilatMuharrik::Unity && muharrik.khalfiya == KhalfiyaBarmajiya::Majhula {
        jawda = jawda.max(JawdaMutawaqqaa::Maqbula);
    }
    if shuhida(muharrik, alamat::MITA_MAJHULA) {
        jawda = jawda.max(JawdaMutawaqqaa::Jayida);
    }
    if shuhida(muharrik, alamat::TASHFEER) {
        jawda = jawda.max(JawdaMutawaqqaa::Maqbula);
    }
    if muharrik.aila == AilatMuharrik::Unreal && shuhida(muharrik, alamat::NUSUS_DAKHIL_HAWIYA) {
        jawda = jawda.max(JawdaMutawaqqaa::Jayida);
    }
    if muharrik.thiqa < HADD_THIQA_MUNKHAFIDA {
        jawda = jawda.max(JawdaMutawaqqaa::Maqbula);
    }
    jawda
}

/// The quality the text systems alone imply, before anything is subtracted.
fn jawda_min_itarat(muharrik: &Muharrik) -> JawdaMutawaqqaa {
    match muharrik.aila {
        AilatMuharrik::Unity => {
            if ladayh(muharrik, ItarNusus::TextMeshPro) {
                JawdaMutawaqqaa::Mumtaza
            } else if ladayh(muharrik, ItarNusus::UnityUiText) {
                JawdaMutawaqqaa::Jayida
            } else {
                JawdaMutawaqqaa::Maqbula
            }
        },
        AilatMuharrik::Unreal => {
            if ladayh(muharrik, ItarNusus::Slate) {
                JawdaMutawaqqaa::Mumtaza
            } else {
                JawdaMutawaqqaa::Maqbula
            }
        },
        AilatMuharrik::Godot => {
            if ladayh(muharrik, ItarNusus::GodotLabel) || ladayh(muharrik, ItarNusus::GodotRichText)
            {
                JawdaMutawaqqaa::Mumtaza
            } else {
                JawdaMutawaqqaa::Maqbula
            }
        },
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz | AilatMuharrik::RpgMakerVxAce => {
            if ladayh(muharrik, ItarNusus::NafidhatRpg) {
                JawdaMutawaqqaa::Mumtaza
            } else {
                JawdaMutawaqqaa::Jayida
            }
        },
        AilatMuharrik::Renpy => {
            if ladayh(muharrik, ItarNusus::NassRenpy) {
                JawdaMutawaqqaa::Mumtaza
            } else {
                JawdaMutawaqqaa::Jayida
            }
        },
        AilatMuharrik::Electron => {
            if ladayh(muharrik, ItarNusus::Dom) {
                JawdaMutawaqqaa::Mumtaza
            } else {
                JawdaMutawaqqaa::Maqbula
            }
        },
        // The two engines that draw text from glyph pages baked into their own
        // data rather than from a font, and the only two whose answer does not
        // depend on which text systems were found. Neither can be excellent and
        // both can be good: the letters are pictures placed in a grid in
        // advance, so a shaped Arabic word is assembled from what fits, and a
        // shape that does not fit is a shape the player does not see. Capcom
        // BIO4 has no [`ItarNusus`] value at all, so there is nothing to switch
        // on for it even in principle.
        AilatMuharrik::GameMaker | AilatMuharrik::Bio4 => JawdaMutawaqqaa::Jayida,
        // Named and unreachable is the same expected quality as unrecognised,
        // and it has to be: the result a player gets is the overlay's, and the
        // overlay does not read better for having been told which engine it is
        // drawing over. Promoting these six because the identification improved
        // would be the report grading its own knowledge instead of the outcome.
        AilatMuharrik::Frostbite
        | AilatMuharrik::BlackSpace
        | AilatMuharrik::Alchemy
        | AilatMuharrik::Dantelion
        | AilatMuharrik::Rage
        | AilatMuharrik::Snowdrop
        | AilatMuharrik::Majhul => JawdaMutawaqqaa::Mahduda,
    }
}

/// The text systems Taarib will actually take over.
///
/// Empty at tier 3, where nothing inside the game is touched and the overlay
/// takes over nothing. At tier 1 and 2 it is everything the identification
/// adopted — resolution has already dropped the text systems that belonged to a
/// claim it rejected, so what is left is a surface on the engine that won.
///
/// Also empty whenever the adapter for this engine does not run. "Will take
/// over" is a promise in the future tense, and a list of systems nothing reaches
/// is the one place in this report where a true sentence about the game becomes
/// a false claim about the product.
fn anzima_qabila(
    muharrik: &Muharrik,
    tabaqa: Tabaqa,
    jahiziya: JahiziyatTashghil,
) -> Vec<ItarNusus> {
    if !jahiziya.tasil() {
        return Vec::new();
    }
    match tabaqa {
        Tabaqa::TarjamaFawqiya => Vec::new(),
        Tabaqa::Kamil | Tabaqa::RasmMubashir => muharrik.itarat.clone(),
    }
}

/// The anti-cheat the launcher associates with this game, if any.
///
/// A named service is preferred over a bare VAC record, because the refusal
/// tells the user what was found and "Easy Anti-Cheat" is an answer they can
/// verify while "VAC" is a fact about Steam.
fn himaya_maalana(simat: &[SimatLuba]) -> Option<String> {
    simat
        .iter()
        .find_map(|sima| match sima {
            SimatLuba::HimayaMuhtamala(ism) => Some(ism.clone()),
            _ => None,
        })
        .or_else(|| {
            simat
                .iter()
                .any(|sima| matches!(sima, SimatLuba::MuammanaVac))
                .then(|| "VAC".to_owned())
        })
}

/// The compatibility layer the launcher records for this game, if any.
fn tabaqat_tawafuq(simat: &[SimatLuba]) -> Option<String> {
    simat.iter().find_map(|sima| match sima {
        SimatLuba::TabaqatTawafuq(wasf) => Some(wasf.clone()),
        _ => None,
    })
}

/// The whole capability report for one game.
///
/// `simat` is what the launcher knows and the files do not: a VAC association,
/// an anti-cheat service named in a manifest, a compatibility layer. `waqt` is
/// when the probe ran, RFC 3339, passed in rather than read here so that every
/// record written by one scan carries the same instant.
///
/// A refused game short-circuits everything. It reports the lowest tier, no
/// reachable text systems, limited quality, and exactly one limitation: the
/// refusal, naming what was detected. None of the engine's limitations are
/// listed, because listing what would have been difficult about Arabizing a
/// game nobody is going to Arabize is noise wrapped around the only sentence
/// that matters. Its readiness verdict carries no sentence for the same reason:
/// [`JahiziyatTashghil::Ghaiba`] is true of it — nothing will reach the screen —
/// but *why* Taarib's adapter is unfinished is beside the point for a game
/// Taarib is refusing to touch, and the interface shows the verdict only when a
/// sentence comes with it.
///
/// **Two refusals short-circuit, not one.** The entry that is not a game refuses
/// here as well as in the core, and it has to: this report is *persisted*, and
/// the core is not. A stored report saying tier three over Steamworks Common
/// Redistributables outlives the session that made it and is read by anything
/// that opens the record without going through
/// [`taarib_aql`](https://docs.rs/taarib-aql) — which is the shape of every
/// two-surfaces-disagree defect this product has found. The core still refuses
/// it, and the two refusals now say the same thing.
#[must_use]
pub fn taqreer(muharrik: Muharrik, simat: &[SimatLuba], waqt: String) -> TaqreerImkaniyat {
    // Anti-cheat is asked first, and the order is not arbitrary: it is
    // `taarib_aql::NawMani::rutba`, where `Himaya` is 0 and `LaysatLuba` is 3.
    // An entry carrying both tags would otherwise be refused here for one reason
    // and by the core for another — two surfaces disagreeing about one entry,
    // which is the defect this whole short-circuit exists to avoid rather than
    // to add.
    if let Some(himaya) = himaya_maalana(simat) {
        return TaqreerImkaniyat {
            tabaqa: Tabaqa::TarjamaFawqiya,
            jahiziya: JahiziyatTashghil::Ghaiba,
            naqs: None,
            sabab_arabi: format!(
                "لن يُعرَّب هذا العنوان. اللعبة محمية بنظام مكافحة غش ({himaya})، ولن \
                 يعدّل تعريب ملفاتها ولن يعرض شيئًا فوقها، لأن ذلك قد يكلّفك حظرًا \
                 دائمًا لحسابك."
            ),
            sabab_injilizi: format!(
                "This title will not be Arabized. The game is protected by an \
                 anti-cheat system ({himaya}); Taarib will not modify its files and \
                 will not draw anything over it, because doing so can cost you a \
                 permanent ban on your account."
            ),
            anzimat_qabila: Vec::new(),
            jawda: JawdaMutawaqqaa::Mahduda,
            hudud: vec![hadd_rafd(&himaya)],
            marfuda: true,
            isdar_fahs: ISDAR_FAHS,
            waqt,
            muharrik,
        };
    }
    if let Some(naw) = naw_ghayr_luba(simat) {
        return TaqreerImkaniyat {
            tabaqa: Tabaqa::TarjamaFawqiya,
            jahiziya: JahiziyatTashghil::Ghaiba,
            naqs: None,
            sabab_arabi: format!(
                "هذا المدخل ليس لعبة؛ يصنّفه المتجر على أنه {naw}. لا يُعرَّب إلا ما هو لعبة، \
                 ولن يُكتب في هذا المجلّد شيء."
            ),
            sabab_injilizi: format!(
                "This entry is not a game — the launcher classifies it as {naw}. Taarib \
                 arabizes games, and nothing will be written into this folder."
            ),
            anzimat_qabila: Vec::new(),
            jawda: JawdaMutawaqqaa::Mahduda,
            hudud: vec![hadd_laysat_luba(&naw)],
            marfuda: true,
            isdar_fahs: ISDAR_FAHS,
            waqt,
            muharrik,
        };
    }

    let (tabaqa, sabab_arabi, sabab_injilizi) = tabaqa_min_muharrik(&muharrik);
    let (jahiziya, naqs) = jahiziya(&muharrik);
    let (sabab_arabi, sabab_injilizi) = match sadr_jahiziya(jahiziya) {
        Some((sadr_arabi, sadr_injilizi)) => (
            format!("{sadr_arabi} {sabab_arabi}"),
            format!("{sadr_injilizi} {sabab_injilizi}"),
        ),
        None => (sabab_arabi, sabab_injilizi),
    };
    let anzimat_qabila = anzima_qabila(&muharrik, tabaqa, jahiziya);
    let jawda = jawda(&muharrik, tabaqa, jahiziya);

    let mut hudud = hudud(&muharrik, tabaqa);
    if simat
        .iter()
        .any(|sima| matches!(sima, SimatLuba::JamaiOnline))
    {
        hudud.insert(0, hadd_jamai());
    }
    if let Some(wasf) = tabaqat_tawafuq(simat) {
        hudud.push(hadd_tawafuq(&wasf));
    }

    TaqreerImkaniyat {
        tabaqa,
        sabab_arabi,
        sabab_injilizi,
        jahiziya,
        naqs,
        anzimat_qabila,
        jawda,
        hudud,
        marfuda: false,
        isdar_fahs: ISDAR_FAHS,
        waqt,
        muharrik,
    }
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::muharrik::{Daleel, IsdarMuharrik, NawDaleel};
    use taarib_usus::manassa::Mimariya;

    use super::*;

    /// Every engine family the probe can resolve to.
    ///
    /// Listed rather than iterated because the enum has no iterator, and written
    /// out in full so that adding a family breaks the exhaustiveness assertion
    /// below rather than quietly leaving the new one untested.
    const KUL_AILAT: [AilatMuharrik; 17] = [
        AilatMuharrik::Unity,
        AilatMuharrik::Unreal,
        AilatMuharrik::Godot,
        AilatMuharrik::RpgMakerMv,
        AilatMuharrik::RpgMakerMz,
        AilatMuharrik::RpgMakerVxAce,
        AilatMuharrik::Renpy,
        AilatMuharrik::GameMaker,
        AilatMuharrik::Electron,
        AilatMuharrik::Bio4,
        AilatMuharrik::Frostbite,
        AilatMuharrik::BlackSpace,
        AilatMuharrik::Alchemy,
        AilatMuharrik::Dantelion,
        AilatMuharrik::Rage,
        AilatMuharrik::Snowdrop,
        AilatMuharrik::Majhul,
    ];

    /// A bare identification with no version and no evidence.
    fn muharrik(aila: AilatMuharrik) -> Muharrik {
        Muharrik {
            aila,
            isdar: None,
            khalfiya: KhalfiyaBarmajiya::Majhula,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X8664,
            thiqa: 95,
            dalail: Vec::new(),
        }
    }

    /// The same, carrying a version the detector read exactly.
    fn bi_isdar(aila: AilatMuharrik, kabir: u16, sagheer: u16, tasheeh: u16) -> Muharrik {
        let mut asas = muharrik(aila);
        asas.isdar = Some(IsdarMuharrik {
            kabir,
            sagheer,
            tasheeh,
            khaam: format!("{kabir}.{sagheer}.{tasheeh}"),
            mushtaqq: false,
        });
        asas
    }

    /// The same, carrying one observation instead of a version.
    fn bi_daleel(aila: AilatMuharrik, wasf: &str) -> Muharrik {
        let mut asas = muharrik(aila);
        asas.dalail.push(Daleel {
            naw: NawDaleel::BayanatMudmaja,
            wasf: wasf.to_owned(),
            mawqi: None,
            wazn: 95,
        });
        asas
    }

    /// The verdict alone, for the pins below.
    fn hukm(muharrik: &Muharrik) -> JahiziyatTashghil {
        jahiziya(muharrik).0
    }

    /// The two sentences, for the assertions about wording.
    fn jumlatan(muharrik: &Muharrik) -> (String, String) {
        jahiziya(muharrik).1.map_or_else(
            || (String::new(), String::new()),
            |naqs| (naqs.arabi, naqs.injilizi),
        )
    }

    // -----------------------------------------------------------------------
    // The contract every arm keeps
    // -----------------------------------------------------------------------

    /// A verdict below `Mukammala` carries two complete sentences, and a
    /// `Mukammala` one carries none. `TaqreerImkaniyat::naqs` states that as its
    /// contract and this is where it is kept.
    #[test]
    fn kul_hukm_yahmil_jumlatayn_aw_la_shaya() {
        for aila in KUL_AILAT {
            for khalfiya in [
                KhalfiyaBarmajiya::Majhula,
                KhalfiyaBarmajiya::Mono,
                KhalfiyaBarmajiya::Il2cpp,
            ] {
                let mut asas = muharrik(aila);
                asas.khalfiya = khalfiya;
                let (hukm, naqs) = jahiziya(&asas);
                match hukm {
                    JahiziyatTashghil::Mukammala => {
                        assert!(
                            naqs.is_none(),
                            "{aila:?}/{khalfiya:?} promises a finished tier and still warns"
                        );
                    },
                    JahiziyatTashghil::Naqisa | JahiziyatTashghil::Ghaiba => {
                        let hadd = naqs.unwrap_or_else(|| hadd("", ""));
                        assert!(
                            !hadd.arabi.trim().is_empty(),
                            "{aila:?}/{khalfiya:?} has no Arabic sentence"
                        );
                        assert!(
                            !hadd.injilizi.trim().is_empty(),
                            "{aila:?}/{khalfiya:?} has no English sentence"
                        );
                    },
                }
            }
        }
    }

    /// Every unfinished arm tells the reader an update closes it. That clause is
    /// the one thing a user can act on — wait, or give up — and a sentence that
    /// dropped it would leave them with only the bad half.
    #[test]
    fn kul_jumla_taqul_inna_tahdithan_yughliquha() {
        for aila in KUL_AILAT {
            let (arabi, injilizi) = jumlatan(&muharrik(aila));
            assert!(arabi.contains("تحديث"), "{aila:?}: {arabi}");
            assert!(injilizi.contains("update"), "{aila:?}: {injilizi}");
        }
    }

    /// A finished tier carries no gap sentence. This was a `Naqisa` pin while
    /// Ren'Py shipped no font; the staging matrix now carries a face into the
    /// Ren'Py component, so the one missing part arrives and the verdict is
    /// `Mukammala` — which `jahiziya` enforces by dropping the sentence, so that
    /// an arm reaching `Mukammala` while still holding one cannot leak it.
    #[test]
    fn al_mukammala_la_tahmil_naqsan() {
        let renpy = bi_isdar(AilatMuharrik::Renpy, 8, 1, 3);
        assert_eq!(hukm(&renpy), JahiziyatTashghil::Mukammala);
        assert!(
            jahiziya(&renpy).1.is_none(),
            "a finished tier must carry no gap sentence"
        );
    }

    // -----------------------------------------------------------------------
    // The pins, one per engine
    // -----------------------------------------------------------------------

    /// Unity on either backend, and on neither. Three arms, three sentences: a
    /// component that cannot bind the loader and one that binds it and finds no
    /// native library are different reports, and collapsing them would send a
    /// reader to the wrong half.
    #[test]
    fn unity_ghaiba_ala_alkhalfiyatayn() {
        let mut jumal: Vec<String> = Vec::new();
        for khalfiya in [
            KhalfiyaBarmajiya::Mono,
            KhalfiyaBarmajiya::Il2cpp,
            KhalfiyaBarmajiya::Majhula,
        ] {
            let mut asas = muharrik(AilatMuharrik::Unity);
            asas.khalfiya = khalfiya;
            assert_eq!(hukm(&asas), JahiziyatTashghil::Ghaiba, "{khalfiya:?}");
            jumal.push(jumlatan(&asas).1);
        }
        jumal.sort();
        let adad = jumal.len();
        jumal.dedup();
        assert_eq!(jumal.len(), adad, "two Unity backends share one sentence");
    }

    /// Unreal: `KatibPak::uktub_fi_luba` has no caller outside its own test, so
    /// no container carrying Arabic is ever put into a game.
    #[test]
    fn unreal_ghaiba() {
        assert_eq!(
            hukm(&muharrik(AilatMuharrik::Unreal)),
            JahiziyatTashghil::Ghaiba
        );
    }

    /// Godot 3 and Godot 4 stop for different reasons and say so differently.
    #[test]
    fn godot_ghaiba_bi_jumlatayn() {
        let thalith = bi_isdar(AilatMuharrik::Godot, 3, 6, 0);
        let arbaa = bi_isdar(AilatMuharrik::Godot, 4, 2, 0);
        assert_eq!(hukm(&thalith), JahiziyatTashghil::Ghaiba);
        assert_eq!(hukm(&arbaa), JahiziyatTashghil::Ghaiba);
        assert_ne!(jumlatan(&thalith).1, jumlatan(&arbaa).1);
    }

    /// RPG Maker MV and MZ: the data splice runs and the install does not
    /// finish, because the runtime module the deployment step demands is not
    /// built into this package.
    #[test]
    fn rpg_maker_ghaiba() {
        for aila in [AilatMuharrik::RpgMakerMv, AilatMuharrik::RpgMakerMz] {
            assert_eq!(hukm(&muharrik(aila)), JahiziyatTashghil::Ghaiba, "{aila:?}");
        }
        assert_eq!(
            hukm(&muharrik(AilatMuharrik::RpgMakerVxAce)),
            JahiziyatTashghil::Ghaiba
        );
    }

    /// GameMaker: the string pool is rewritten and the glyph pages are not, so
    /// the write succeeds and the text it wrote cannot be drawn. The verdict has
    /// to be the one that withholds the one-button run.
    #[test]
    fn gamemaker_ghaiba_wa_tasil_kadhib() {
        let asas = muharrik(AilatMuharrik::GameMaker);
        assert_eq!(hukm(&asas), JahiziyatTashghil::Ghaiba);
        assert!(
            !hukm(&asas).tasil(),
            "a GameMaker run would blank the game's own text"
        );
        let (arabi, injilizi) = jumlatan(&asas);
        assert!(arabi.contains("صفحات الحروف"), "{arabi}");
        assert!(injilizi.contains("glyph pages"), "{injilizi}");
    }

    /// Electron: the adapter refuses by name when the compiled renderer runtime
    /// is absent, and it is absent, so the install stops rather than completing
    /// into an unchanged game.
    #[test]
    fn electron_ghaiba() {
        assert_eq!(
            hukm(&muharrik(AilatMuharrik::Electron)),
            JahiziyatTashghil::Ghaiba
        );
    }

    /// The overlay, which is every unrecognised engine.
    #[test]
    fn tabaqa_ghaiba() {
        assert_eq!(
            hukm(&muharrik(AilatMuharrik::Majhul)),
            JahiziyatTashghil::Ghaiba
        );
    }

    // -----------------------------------------------------------------------
    // Ren'Py, the one arm the game decides
    // -----------------------------------------------------------------------

    /// At or above the shaping release the engine joins and reorders Arabic
    /// itself, the translation and the direction are both installed, and the
    /// font is the one thing missing.
    #[test]
    fn renpy_yashkul_fahuwa_mukammala() {
        for (kabir, sagheer) in [(7, 4), (7, 8), (8, 0), (8, 3)] {
            let asas = bi_isdar(AilatMuharrik::Renpy, kabir, sagheer, 0);
            assert_eq!(
                hukm(&asas),
                JahiziyatTashghil::Mukammala,
                "Ren'Py {kabir}.{sagheer}"
            );
        }
    }

    /// Below it the engine blits one character at a time and the takeover that
    /// would draw them instead has no native library to load, so nothing the
    /// tier promises reaches the screen.
    #[test]
    fn renpy_bila_tashkeel_fahuwa_ghaiba() {
        for (kabir, sagheer) in [(7, 3), (7, 0), (6, 99)] {
            let asas = bi_isdar(AilatMuharrik::Renpy, kabir, sagheer, 0);
            assert_eq!(
                hukm(&asas),
                JahiziyatTashghil::Ghaiba,
                "Ren'Py {kabir}.{sagheer}"
            );
        }
    }

    /// With no version the probe still records which side the build directories
    /// pointed at, and the verdict follows that rather than a default.
    #[test]
    fn renpy_min_dalil_al_binya() {
        let yashkul = bi_daleel(
            AilatMuharrik::Renpy,
            "no version_tuple was readable; lib/ carries the py2 and py3 prefixes Ren'Py \
             introduced at 7.4, on Python 3, so this build is 7.4 or later and the engine can \
             shape Arabic itself — inferred from directory names, not read",
        );
        assert_eq!(hukm(&yashkul), JahiziyatTashghil::Mukammala);

        let la_yashkul = bi_daleel(
            AilatMuharrik::Renpy,
            "no version_tuple was readable; lib/ carries no py2 or py3 prefix, and Ren'Py \
             introduced those at 7.4, so this build is 7.3 or earlier and has no shaper — \
             inferred from directory names, not read",
        );
        assert_eq!(hukm(&la_yashkul), JahiziyatTashghil::Ghaiba);
    }

    /// Nothing bounded the version at all: the third answer, and neither story
    /// may be told.
    #[test]
    fn renpy_majhul_al_isdar_ghaiba() {
        let asas = muharrik(AilatMuharrik::Renpy);
        assert!(renpy_yashkul(&asas).is_none());
        assert_eq!(hukm(&asas), JahiziyatTashghil::Ghaiba);
    }

    /// The markers match the sentences `dalail::nusus::renpy_tashkeel` actually
    /// writes, and each matches exactly one of them. This is the coupling that
    /// would otherwise break silently: rewording a detector's sentence around
    /// the marker is safe, deleting the claim is what must not happen quietly.
    #[test]
    fn alamat_renpy_tutabiq_jumal_alkashf() {
        let yashkul = [
            "Ren'Py 8.1.3 is at or above 7.4, so the engine bundles HarfBuzz and FriBidi and \
             shapes and reorders Arabic itself",
            "no version_tuple was readable; lib/ carries the py2 and py3 prefixes Ren'Py \
             introduced at 7.4, on Python 3, so this build is 7.4 or later and the engine can \
             shape Arabic itself — inferred from directory names, not read",
        ];
        let la_yashkul = [
            "Ren'Py 7.3.5 is below 7.4, so the engine has no shaper and Arabic has to be laid \
             out and drawn by Taarib",
            "no version_tuple was readable; lib/ carries no py2 or py3 prefix, and Ren'Py \
             introduced those at 7.4, so this build is 7.3 or earlier and has no shaper — \
             inferred from directory names, not read",
        ];
        for wasf in yashkul {
            let asas = bi_daleel(AilatMuharrik::Renpy, wasf);
            assert!(shuhida(&asas, alamat::TASHKEEL_RENPY), "unmatched: {wasf}");
            assert!(
                !shuhida(&asas, alamat::BILA_TASHKEEL_RENPY),
                "cross-matched: {wasf}"
            );
        }
        for wasf in la_yashkul {
            let asas = bi_daleel(AilatMuharrik::Renpy, wasf);
            assert!(
                shuhida(&asas, alamat::BILA_TASHKEEL_RENPY),
                "unmatched: {wasf}"
            );
            assert!(
                !shuhida(&asas, alamat::TASHKEEL_RENPY),
                "cross-matched: {wasf}"
            );
        }
    }

    /// An exact version outranks the evidence, because it is the stronger fact
    /// and because the detector records both for the same game.
    #[test]
    fn al_isdar_yaghlib_al_daleel() {
        let mut asas = bi_daleel(
            AilatMuharrik::Renpy,
            "no version_tuple was readable; lib/ carries no py2 or py3 prefix, and Ren'Py \
             introduced those at 7.4, so this build is 7.3 or earlier and has no shaper — \
             inferred from directory names, not read",
        );
        asas.isdar = Some(IsdarMuharrik {
            kabir: 8,
            sagheer: 2,
            tasheeh: 0,
            khaam: "8.2.0".to_owned(),
            mushtaqq: false,
        });
        assert_eq!(renpy_yashkul(&asas), Some(true));
    }

    // -----------------------------------------------------------------------
    // What the verdict does to the rest of the report
    // -----------------------------------------------------------------------

    /// A verdict of `Ghaiba` empties the reachable text systems and floors the
    /// quality; `Naqisa` keeps the systems, because they really are taken over,
    /// and is capped below excellent, because a named part of the tier does not
    /// arrive.
    #[test]
    fn al_taqreer_yattabi_al_hukm() {
        let mut ghaib = muharrik(AilatMuharrik::GameMaker);
        ghaib.itarat.push(ItarNusus::RasmGameMaker);
        let taqreer_ghaib = taqreer(ghaib, &[], "2026-01-01T00:00:00Z".to_owned());
        assert_eq!(taqreer_ghaib.jahiziya, JahiziyatTashghil::Ghaiba);
        assert!(taqreer_ghaib.anzimat_qabila.is_empty());
        assert_eq!(taqreer_ghaib.jawda, JawdaMutawaqqaa::Mahduda);

        // Ren'Py on a shaping engine is the one arm that finishes, so it is
        // the only one that can pin the `Mukammala` half of this. **No arm
        // answers `Naqisa` today** — the quality cap at `jawda`'s
        // `Naqisa` branch is therefore unreachable from this function, and is
        // pinned directly below rather than through an engine that no longer
        // reaches it.
        let mut tamm = bi_isdar(AilatMuharrik::Renpy, 8, 1, 3);
        tamm.itarat.push(ItarNusus::NassRenpy);
        let taqreer_tamm = taqreer(tamm, &[], "2026-01-01T00:00:00Z".to_owned());
        assert_eq!(taqreer_tamm.jahiziya, JahiziyatTashghil::Mukammala);
        assert_eq!(taqreer_tamm.anzimat_qabila, vec![ItarNusus::NassRenpy]);
        // `<=` because worse sorts higher: this asserts "good or better",
        // which is the half the `Naqisa` cap used to forbid.
        assert!(
            taqreer_tamm.jawda <= JawdaMutawaqqaa::Jayida,
            "a finished tier was reported worse than good: {:?}",
            taqreer_tamm.jawda
        );
    }

    /// The quality cap that `Naqisa` applies, pinned directly.
    ///
    /// No arm has reached `Naqisa` since Ren'Py finished, so without this the
    /// cap would be untested code that the next partly-delivered engine relies
    /// on. A report saying half the tier arrives must not also say the result is
    /// excellent — the two sentences sit next to each other on the screen.
    ///
    /// Note the direction: worse verdicts sort *higher* in
    /// [`JawdaMutawaqqaa`], so capping is `max`, not `min`.
    #[test]
    fn al_naqisa_tahudd_al_jawda() {
        assert!(
            JawdaMutawaqqaa::Jayida > JawdaMutawaqqaa::Mumtaza,
            "the cap below is `max`, which is only correct while worse sorts higher"
        );
        assert_eq!(
            JawdaMutawaqqaa::Mumtaza.max(JawdaMutawaqqaa::Jayida),
            JawdaMutawaqqaa::Jayida,
            "`Naqisa` must pull an excellent verdict down to good"
        );
        assert_eq!(
            JawdaMutawaqqaa::Mahduda.max(JawdaMutawaqqaa::Jayida),
            JawdaMutawaqqaa::Mahduda,
            "the cap must never improve a verdict that was already worse"
        );
    }

    /// The clause printed under the tier name warns for both unfinished
    /// verdicts and is silent for the finished one.
    #[test]
    fn sadr_al_jahiziya_yunabbih_marratayn_faqat() {
        assert!(sadr_jahiziya(JahiziyatTashghil::Mukammala).is_none());
        assert!(sadr_jahiziya(JahiziyatTashghil::Naqisa).is_some());
        assert!(sadr_jahiziya(JahiziyatTashghil::Ghaiba).is_some());
    }

    /// A refused game carries no readiness sentence, whatever its engine's arm
    /// answers — including the arm that now answers `Naqisa`.
    #[test]
    fn al_marfuda_bila_jumlat_jahiziya() {
        let mut renpy = bi_isdar(AilatMuharrik::Renpy, 8, 1, 3);
        renpy.itarat.push(ItarNusus::NassRenpy);
        let taqreer = taqreer(
            renpy,
            &[SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())],
            "2026-01-01T00:00:00Z".to_owned(),
        );
        assert!(taqreer.marfuda);
        assert_eq!(taqreer.jahiziya, JahiziyatTashghil::Ghaiba);
        assert!(taqreer.naqs.is_none());
    }
}
