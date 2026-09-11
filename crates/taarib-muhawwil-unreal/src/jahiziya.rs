//! الجاهزية — what this build of the Unreal adapter can actually do.
//!
//! Every other module in this crate says what it *would* do to a game. This one
//! says what happens, in this package, today. It exists because the answer is
//! read by something that acts on it: a gate that refuses the one-button run for
//! an engine that cannot be patched asks here, and a hopeful answer here becomes
//! a user who waits for a translation that is never going to appear.
//!
//! ## The shape of the answer
//!
//! Six capabilities, [`Qudra`], each in one of three states, [`HalatQudra`].
//! The distinction the states draw is the one that matters to a reader:
//!
//! * [`HalatQudra::Amila`] — the code is here, it is correct, and something in
//!   this package drives it. It runs.
//! * [`HalatQudra::Mahjuba`] — the code is here and it is correct, and **nothing
//!   in this package calls it**. It never runs against a real game. This is not
//!   a bug in the module it names; it is a missing edge somewhere else, and
//!   saying so is more useful than either "works" or "missing".
//! * [`HalatQudra::Ghaiba`] — it is not available in this build at all, either
//!   because a feature that carries it was not compiled or because it needs an
//!   input nothing in this repository produces.
//!
//! [`jahiziya`] reduces the six to the one verdict
//! [`taarib_mustalahat::muharrik::JahiziyatTashghil`] spells, by a single blunt
//! rule: **Arabic reaches a player's screen only if the translated container is
//! written into the game.** Everything else here improves what the player sees
//! once it is there, and none of it substitutes for it.
//!
//! ## Why this is a hand-maintained table and what stops it drifting
//!
//! One axis is mechanical: [`hamula_mabniya`] is `cfg!(feature = "hamula")`, so
//! the state of [`Qudra::Tashghil`] cannot claim a bootstrap this build does not
//! export. The rest are facts about which functions have callers, and no program
//! can read that off itself — they were established by grepping `crates/`,
//! `apps/`, `unity/` and `adapters-script/` for each entry point, the same way
//! `docs/tashghil.md` establishes its own table, and they must be re-established
//! whenever a caller is added.
//!
//! When one of them changes: change the arm here, change the row in
//! `docs/tashghil.md`, and change the Unreal arm of `taarib-muharrik`'s
//! `imkaniyat::jahiziya` — which is the table the rest of the product reads and
//! which cannot call into this crate, because this crate depends on it and not
//! the other way round. Three places, one commit. A readiness value that
//! disagrees with the code it describes is worse than no readiness value: it is
//! a claim with a function name attached, which is the shape of a thing people
//! stop checking.
//!
//! ## What is true as this is written, and exactly how far it was checked
//!
//! Reading works and is driven — `taarib-istikhraj`'s Unreal extractor and the
//! Studio's game view both go through [`crate::mawarid`]. Writing the patch
//! container works and is driven: `taarib_tathbeet::tarkib`'s Unreal arm plans
//! it and its deployment step builds it through [`crate::hawiya::ibni`] and
//! writes it through the install recorder. The face travels in that container
//! and is named through the engine's own localized fallback-font string on the
//! engines that read one. The three in-process corrections are unreached and
//! the measurement corrections are unreachable. So the verdict is
//! [`JahiziyatTashghil::Naqisa`]: the container that carries the Arabic reaches
//! the game, and no part of the path has yet been watched putting a glyph on a
//! screen.
//!
//! "Works" above means these five rungs, and stops there:
//!
//! 1. **Constants checked against real bytes.** The `.pak`, `.locres`,
//!    `.locmeta` and `.utoc` magics and the package tag were compared against
//!    shipped containers rather than against memory.
//! 2. **Containers read.** `.pak` version 3 and version 11, IoStore `ToC`
//!    version 6 and version 8, and `.locres` in its legacy, version 1 and 3
//!    shapes, all from games on the machine this was written on. Every one of
//!    them wrote back byte-identically. Version 2 of `.locres` and pak versions
//!    other than 3 and 11 are implemented and were **not** met on a real game.
//! 3. **A container written.** [`crate::mawarid::pak::KatibPak`] built an
//!    additive patch `.pak` holding an edited Arabic `.locres` and a `.locmeta`
//!    naming `ar`, and this crate's own reader read it back and resolved both.
//! 4. **A container written into a game.** The installer's Unreal arm placed
//!    [`crate::hawiya`]'s container beside a shipped 4.13 title's own, built
//!    from that title's containers, and the product's reader read it back.
//! 5. **A real game drawing it: not reached.** No engine has been observed
//!    mounting a container this build wrote. That rung needs a Windows machine
//!    running the game, and nothing below it substitutes for it.

use taarib_mustalahat::muharrik::{Hadd, JahiziyatTashghil};

/// One thing this adapter would have to do for Arabic to reach the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Qudra {
    /// Read the game's containers and its localization resources.
    Qiraa,
    /// Author the additive patch container the engine mounts.
    Kitaba,
    /// Force Slate's full shaping path on.
    Tashghil,
    /// Register the patch's own font with Slate's font system.
    Khatt,
    /// Register Arabic as a culture and make it the active one.
    Alam,
    /// Correct flow direction, alignment resolution and wrapping.
    Qiyas,
}

/// Every capability, in the order a maintainer reads them: offline first,
/// because that is the order they run in.
pub const QUDRAT: [Qudra; 6] = [
    Qudra::Qiraa,
    Qudra::Kitaba,
    Qudra::Tashghil,
    Qudra::Khatt,
    Qudra::Alam,
    Qudra::Qiyas,
];

impl Qudra {
    /// A stable machine name, for logs and the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Qiraa => "qiraa",
            Self::Kitaba => "kitaba",
            Self::Tashghil => "tashghil",
            Self::Khatt => "khatt",
            Self::Alam => "alam",
            Self::Qiyas => "qiyas",
        }
    }

    /// The module that owns it, so a reader can go and look.
    #[must_use]
    pub const fn wahda(self) -> &'static str {
        match self {
            Self::Qiraa | Self::Kitaba => "mawarid",
            Self::Tashghil => "tashghil",
            Self::Khatt => "khatt",
            Self::Alam => "alam",
            Self::Qiyas => "qiyas",
        }
    }
}

/// How far one capability actually goes in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalatQudra {
    /// Implemented, correct, and driven by something in this package.
    Amila,
    /// Implemented and correct, and called by nothing, so it never runs.
    Mahjuba,
    /// Not available in this build at all.
    Ghaiba,
}

impl HalatQudra {
    /// A stable machine name.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Amila => "amila",
            Self::Mahjuba => "mahjuba",
            Self::Ghaiba => "ghaiba",
        }
    }

    /// Whether this capability runs at all.
    #[must_use]
    pub const fn tajri(self) -> bool {
        matches!(self, Self::Amila)
    }
}

/// One capability, its state, and the sentence a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BayanQudra {
    /// Which capability.
    pub qudra: Qudra,
    /// How far it goes.
    pub hala: HalatQudra,
    /// What that means, in Arabic, as a complete sentence.
    pub arabi: &'static str,
    /// The same sentence in English.
    pub injilizi: &'static str,
}

/// Whether this build carries the in-process bootstrap.
///
/// The one mechanical fact in this module. Without the `hamula` feature this
/// crate's cdylib exports no `taarib_bidaya`, `taarib-mudkhal` resolves nothing
/// in it, and no line of the runtime half is ever entered — so a readiness
/// answer that depended on prose alone could claim a payload that is not in the
/// file. This cannot.
#[must_use]
pub const fn hamula_mabniya() -> bool {
    cfg!(feature = "hamula")
}

/// How far one capability goes in this build.
#[must_use]
#[expect(
    clippy::match_same_arms,
    reason = "each arm is a separately maintained fact with its own evidence, and merging \
              two that happen to answer alike today would make a change to one silently \
              change the other"
)]
pub const fn hala(qudra: Qudra) -> HalatQudra {
    match qudra {
        // `taarib-istikhraj`'s Unreal extractor and the Studio's game view both
        // drive `mawarid`, and both were run against shipped titles.
        Qudra::Qiraa => HalatQudra::Amila,
        // `hawiya::ibni` builds the container and `taarib_tathbeet::tarkib`'s
        // Unreal deployment arm writes it into the game.
        Qudra::Kitaba => HalatQudra::Amila,
        // `Tashghil::shaghghil` is called from `bidaya`, and from nowhere else.
        // With no bootstrap there is no offline caller either: the installer
        // writes the patch and never touches `Engine.ini`.
        Qudra::Tashghil => {
            if hamula_mabniya() {
                HalatQudra::Amila
            } else {
                HalatQudra::Ghaiba
            }
        },
        // The face travels in the container and is named through the engine's
        // own localized fallback-font string, which `hawiya::ibni` writes on
        // every engine whose resources carry it. `Khatt::sajjil`, the
        // in-process registration, still has no caller.
        Qudra::Khatt => HalatQudra::Amila,
        // `Alam::faal` has no caller anywhere. The container adds an `ar`
        // culture and overrides every shipped one, so which culture is active
        // stops mattering, and nothing activates one.
        Qudra::Alam => HalatQudra::Mahjuba,
        // `TasheehQiyas::rakkib` needs an `AhdafQiyas`, which is constructed
        // nowhere, and this crate refuses to invent an address. Unreachable
        // rather than merely unreached.
        Qudra::Qiyas => HalatQudra::Ghaiba,
    }
}

/// One capability's state with the sentences that explain it.
#[must_use]
pub const fn bayan(qudra: Qudra) -> BayanQudra {
    let hala = hala(qudra);
    let (arabi, injilizi) = match qudra {
        Qudra::Qiraa => (
            "قراءة حاويات اللعبة ومواردها تعمل، وقد جُرِّبت على ألعاب مشحونة: حاويات pak \
             وIoStore، وملفات locres وlocmeta، وجداول النصوص. تُرفض الكتل المضغوطة بـ \
             Oodle والحاويات المشفَّرة بلا مفتاح، ويُسمَّى السبب.",
            "Reading the game's containers and resources works and has been run against \
             shipped games: pak and IoStore containers, .locres, .locmeta and string tables. \
             Oodle-compressed blocks and encrypted containers with no key are refused by \
             name.",
        ),
        Qudra::Kitaba => (
            "كتابة حاوية الرقعة الإضافية تعمل ويستدعيها المثبِّت: تُبنى من حاويات اللعبة \
             نفسها، تُستبدل فيها النصوص المترجمة في كل ثقافة تشحنها اللعبة، وتُضاف ثقافة \
             عربية بجانبها، ثم تُكتب بجانب حاويات اللعبة حيث يركّبها المحرّك باسمها.",
            "Writing the additive patch container works and the installer calls it: it is \
             built from the game's own containers, the translated strings are replaced in \
             every culture the game ships, an Arabic culture is added beside them, and it is \
             written beside the game's containers where the engine mounts it by name.",
        ),
        Qudra::Tashghil => {
            if hamula_mabniya() {
                (
                    "تشغيل التشكيل الكامل: يكتبه محمّل الوحدة داخل اللعبة في ملف \
                     Engine.ini، ويسري من التشغيل التالي.",
                    "Forcing full shaping: the in-process bootstrap writes it into the game's \
                     Engine.ini, and it applies from the next launch.",
                )
            } else {
                (
                    "تشغيل التشكيل الكامل غير موجود في هذه الحزمة: خاصية hamula لم تُبْنَ، \
                     فلا تُصدَّر نقطة البدء، ولا يكتب المثبِّت شيئًا في إعدادات اللعبة.",
                    "Forcing full shaping is absent from this build: the `hamula` feature was \
                     not compiled, so no bootstrap symbol is exported, and the installer \
                     writes nothing into the game's configuration.",
                )
            }
        },
        Qudra::Khatt => (
            "خطّ الرقعة يسافر داخل الحاوية ويُسمَّى خطَّ الاحتياط في Slate عبر نصّ المحرّك \
             المترجم نفسه، وذلك على المحرّكات التي تقرأ هذا النصّ (أنريل 4.13 حتى نحو 4.19)؛ \
             المحرّكات الأحدث تحمل خطًّا عربيًّا في احتياطها المركّب ولا تحتاج إليه. أمّا \
             التسجيل داخل العملية فلا يستدعيه شيء.",
            "The patch's face travels in the container and is named as Slate's fallback face \
             through the engine's own localized string, on the engines that read one \
             (Unreal 4.13 to about 4.19); later engines carry an Arabic face in their \
             fallback composite and need none. The in-process registration has no caller.",
        ),
        Qudra::Alam => (
            "تُضاف العربية ثقافةً داخل الحاوية وتُستبدل النصوص في كل ثقافة تشحنها اللعبة، \
             فلا يعود مهمًّا أيّ ثقافة نشطة؛ أمّا تفعيلها داخل العملية فلا يستدعيه شيء.",
            "Arabic is added as a culture inside the container and the strings are replaced \
             in every culture the game ships, so which culture is active stops mattering; \
             activating one in the process has no caller.",
        ),
        Qudra::Qiyas => (
            "تصحيحات الاتجاه والمحاذاة واللفّ لا تعمل في أي بناء: تحتاج عناوين دوال \
             محقَّقة لبناء المحرّك بعينه، ولا شيء في هذا المستودع ينتجها، وهذه الوحدة لا \
             تخمّن عنوانًا لأن خطّافًا في العنوان الخطأ يُعطِّل لعبة اللاعب.",
            "The direction, alignment and wrapping corrections run in no build: they need \
             verified function addresses for one specific engine build, nothing in this \
             repository produces them, and this crate will not guess an address, because a \
             detour at the wrong one breaks a player's game.",
        ),
    };
    BayanQudra {
        qudra,
        hala,
        arabi,
        injilizi,
    }
}

/// Every capability, in [`QUDRAT`] order.
#[must_use]
pub fn taqreer() -> [BayanQudra; QUDRAT.len()] {
    QUDRAT.map(bayan)
}

/// Whether anything this adapter does reaches a player's screen, and how much.
///
/// The rule is one line and is deliberately blunt: a translation reaches the
/// screen only when the container carrying it is written into the game, so
/// [`Qudra::Kitaba`] is a floor under everything else. An adapter that forced
/// full shaping, registered a font, activated Arabic and corrected every
/// measurement, and never installed a translated `.locres`, would show the
/// player a game in its original language with beautifully shaped English in it.
#[must_use]
pub fn jahiziya() -> JahiziyatTashghil {
    if !hala(Qudra::Kitaba).tajri() {
        return JahiziyatTashghil::Ghaiba;
    }
    if QUDRAT.iter().all(|qudra| hala(*qudra).tajri()) {
        JahiziyatTashghil::Mukammala
    } else {
        JahiziyatTashghil::Naqisa
    }
}

/// What is unfinished, as a sentence addressed to a player.
///
/// [`None`] when [`jahiziya`] is [`JahiziyatTashghil::Mukammala`], because there
/// is then nothing to warn about. The sentence names what the player would have
/// seen and says an update closes it, because the one thing a user can act on is
/// whether to wait.
#[must_use]
pub fn naqs() -> Option<Hadd> {
    if jahiziya() == JahiziyatTashghil::Mukammala {
        return None;
    }
    Some(Hadd {
        arabi: "يكتب هذا الإصدار حاوية الرقعة بجانب حاويات اللعبة: نصوص اللعبة المترجمة \
                في كل ثقافة تشحنها، وثقافة عربية بجانبها، والخطّ حيث يقرأ المحرّك اسمه. \
                يركّبها المحرّك باسمها ويرسم منها بنفسه، ولم يُشاهَد ذلك بعد في لعبة تعمل؛ \
                ملفاتك الأصلية تبقى كما هي وتُستعاد بالضبط."
            .to_owned(),
        injilizi: "This build writes the patch container beside the game's own containers: \
                   the game's text translated in every culture it ships, an Arabic culture \
                   beside them, and the face where the engine reads its name. The engine \
                   mounts it by name and draws from it on its own, and that has not yet been \
                   watched happening in a running game; your original files are left as they \
                   are and are restored exactly."
            .to_owned(),
    })
}
