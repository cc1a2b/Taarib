//! أخطاء محوّل غودو — what stops this adapter, and what merely picks a different
//! path through it.
//!
//! This crate serves two engines, and their failures do not mean the same thing.
//!
//! Under **Godot 4** almost nothing can go badly. The engine's text server is
//! correct, the extension interface is documented and stable, and the delivery
//! is a resource pack the engine mounts natively. A failure there is a font that
//! did not register or a locale that did not take, and the game still runs in
//! its original language — so those are warnings.
//!
//! Under **Godot 3** the adapter is drawing the text itself, and a failure is
//! the difference between Arabic and unjoined letters in the wrong order. Worse,
//! Godot 3 has three strategies stacked behind each other — hook the draw call,
//! or fall back to the glyph transport, or decline — so a failure at one level
//! is not a failure of the phase, it is a decision to descend. The variants
//! below distinguish "this level did not work" from "no level worked", because
//! reporting the first as fatal would refuse a translation the transport could
//! have delivered.
//!
//! The **package formats** behave like every other container in this product: a
//! field that does not check out is a refusal naming it, because writing a
//! `.pck` the engine mounts and then fails to read surfaces to a player as a
//! game that no longer starts.
//!
//! ## Error codes
//!
//! The adapters share [`arqam::MUHAWWIL`] (4300); `AWWAL` gives this crate 4340–4361.
//! starts at forty. A permanent code is permanent and users paste them into bug
//! reports; two subsystems answering to one number is a search that returns the
//! wrong page years later.
//!
//! That is also why nothing here was renumbered when the glyph transport moved
//! to `taarib-lawha`. The transport now refuses in that crate's own band, and
//! [`KhataGodot::NaqlMarfud`] and [`KhataGodot::NaqlMumtali`] stay exactly where
//! they were, as the codes this adapter prints when the Godot 3 ladder reaches
//! its last rung and that rung declines.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_lawha::KhataLawha;
use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The first code this module uses within the adapters' band.
///
/// Forty, leaving the Unity capture path's ten and the Unreal adapter's twenty
/// untouched. See this module's header.
const AWWAL: u16 = 40;

/// Failures of the Godot adapter.
#[derive(Debug, thiserror::Error)]
pub enum KhataGodot {
    /// A file could not be opened, mapped, read or written.
    #[error("{masar} could not be read")]
    KhataMalaf {
        /// The path that failed.
        masar: PathBuf,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A file ended before a field this format requires.
    #[error("{haql} needs {matlub} bytes and only {tul} are present")]
    MalafQaseer {
        /// Which field ran out.
        haql: &'static str,
        /// How many bytes there were.
        tul: u64,
        /// How many were needed.
        matlub: u64,
    },

    /// A declared size exceeds what this build will allocate for it.
    #[error("{haql} declares {qeema}, above the ceiling of {saqf}")]
    HajmMufrit {
        /// Which field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// The Godot version could not be determined.
    ///
    /// Consequential here in a way it is not for other engines: the whole
    /// strategy branches on 3 against 4, and a wrong answer would run the full
    /// takeover against an engine that needs none of it.
    #[error("the Godot version could not be determined: {sabab}")]
    IsdarMajhul {
        /// What every source reported, in one sentence.
        sabab: String,
    },

    /// The Godot version is one this adapter has no strategy for.
    #[error("Godot {wujid} is outside the 3.x and 4.x this adapter handles")]
    MuharrikGhayrMadum {
        /// What was found.
        wujid: String,
    },

    /// A package does not begin, or end, with the bytes the format defines.
    #[error("{masar} does not carry the PCK magic")]
    SihrGhayrMutabaq {
        /// Which file.
        masar: PathBuf,
    },

    /// A package's format version is one this build does not read.
    #[error("PCK version {wujid} is above the {aqsa} this build reads")]
    IsdarGhayrMadum {
        /// The version found.
        wujid: u32,
        /// The highest version this build reads.
        aqsa: u32,
    },

    /// A field inside a package or a resource is not consistent with the file.
    #[error("{ism}: {haql} is {qeema}, outside 0..{hadd}")]
    HawiyaTalifa {
        /// Which format.
        ism: &'static str,
        /// Which field.
        haql: &'static str,
        /// What it said.
        qeema: u64,
        /// The bound it broke.
        hadd: u64,
    },

    /// A package is encrypted and no key was supplied.
    ///
    /// Godot encrypts a `.pck` with a key the game's own export template
    /// carries. Taarib does not go looking for it, because a tool that
    /// recovered keys would be a tool with a second purpose. The user supplies
    /// it or the package is not read.
    #[error("{masar} is encrypted and no key was supplied")]
    PckMushaffar {
        /// Which package.
        masar: PathBuf,
    },

    /// The supplied key does not decrypt the package.
    #[error("the supplied key does not decrypt {masar}")]
    MiftahGhayrSalih {
        /// Which package.
        masar: PathBuf,
        /// Why, in one sentence.
        sabab: &'static str,
    },

    /// A package entry's checksum does not match its bytes.
    #[error("entry \"{madkhal}\" does not match its recorded checksum")]
    BasmaGhayrMutabaqa {
        /// Which entry.
        madkhal: String,
    },

    /// Decompressing a package entry failed.
    #[error("an entry could not be expanded")]
    FakkFashil {
        /// What the decompressor reported.
        tafsil: String,
    },

    /// An entry expanded to a size other than the one it declared.
    #[error("an entry declared {muallan} bytes and produced {fili}")]
    HajmGhayrMutabaq {
        /// What it declared.
        muallan: u64,
        /// What it produced.
        fili: u64,
    },

    /// A package declares a compression method this build cannot expand.
    #[error("an entry uses compression mode {naw}, which this build cannot expand")]
    DaghtMajhul {
        /// The mode number the package declared.
        naw: u32,
    },

    /// A translation resource is not the shape the format defines.
    #[error("the translation resource is malformed: {haql}")]
    TarjamaTalifa {
        /// Which field.
        haql: &'static str,
    },

    /// A string in a resource is not valid UTF-8.
    ///
    /// Refused rather than replaced. A lossy conversion would put replacement
    /// characters into a game's dialogue and call it a translation.
    #[error("string {fahras} is not valid UTF-8 at byte {mawqi}")]
    NassGhayrSalih {
        /// Which record.
        fahras: u32,
        /// The first byte that is not valid.
        mawqi: u32,
    },

    /// The extension could not be registered with Godot 4.
    #[error("the Taarib extension could not register with Godot: {sabab}")]
    ImtidadMarfud {
        /// Why.
        sabab: String,
    },

    /// The patch's font could not be registered.
    #[error("the patch's font could not be registered: {sabab}")]
    KhattMarfud {
        /// Why.
        sabab: String,
    },

    /// The Arabic locale could not be activated.
    #[error("the Arabic locale could not be activated: {sabab}")]
    ThaqafaMarfuda {
        /// Why.
        sabab: String,
    },

    /// A detour could not be installed in a Godot 3 process.
    ///
    /// Not fatal by itself: this is the signal to descend to the glyph
    /// transport, which needs no hook at all. It becomes fatal only when the
    /// transport also declines.
    #[error("the hook on {hadaf} could not be installed: {tafsil}")]
    KhatfFashil {
        /// Which function.
        hadaf: &'static str,
        /// What the hooking layer reported.
        tafsil: String,
    },

    /// Godot 3's low-level canvas API could not be reached.
    #[error("the engine's canvas API could not be reached: {sabab}")]
    RasmGhayrMutah {
        /// Why, naming what was tried.
        sabab: String,
    },

    /// The glyph transport could not be generated.
    ///
    /// The end of the Godot 3 ladder. When this fires, no path remains and the
    /// game falls back to tier 3 with an honest explanation.
    #[error("the glyph transport could not be generated: {sabab}")]
    NaqlMarfud {
        /// Why.
        sabab: String,
    },

    /// More glyphs were needed than the transport can address.
    ///
    /// The private-use area is finite. A script with more distinct shaped
    /// glyphs than it holds cannot be transported, and that is a refusal rather
    /// than a silent truncation — a truncated glyph table draws the wrong
    /// letters, which reads as a corrupt font rather than as a limit reached.
    #[error("the transport needs {matlub} glyph slots and holds {mutah}")]
    NaqlMumtali {
        /// How many distinct glyphs the patch needs.
        matlub: u64,
        /// How many the private-use area provides.
        mutah: u64,
    },
}

impl Tafsir for KhataGodot {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUHAWWIL
                + AWWAL
                + match self {
                    Self::KhataMalaf { .. } => 0,
                    Self::MalafQaseer { .. } => 1,
                    Self::HajmMufrit { .. } => 2,
                    Self::IsdarMajhul { .. } => 3,
                    Self::MuharrikGhayrMadum { .. } => 4,
                    Self::SihrGhayrMutabaq { .. } => 5,
                    Self::IsdarGhayrMadum { .. } => 6,
                    Self::HawiyaTalifa { .. } => 7,
                    Self::PckMushaffar { .. } => 8,
                    Self::MiftahGhayrSalih { .. } => 9,
                    Self::BasmaGhayrMutabaqa { .. } => 10,
                    Self::FakkFashil { .. } => 11,
                    Self::HajmGhayrMutabaq { .. } => 12,
                    Self::DaghtMajhul { .. } => 13,
                    Self::TarjamaTalifa { .. } => 14,
                    Self::NassGhayrSalih { .. } => 15,
                    Self::ImtidadMarfud { .. } => 16,
                    Self::KhattMarfud { .. } => 17,
                    Self::ThaqafaMarfuda { .. } => 18,
                    Self::KhatfFashil { .. } => 19,
                    Self::RasmGhayrMutah { .. } => 20,
                    Self::NaqlMarfud { .. } => 21,
                    Self::NaqlMumtali { .. } => 22,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Each of these is a rung declining, not the ladder ending. The
            // Godot 3 strategy descends from hooking to the transport, and the
            // Godot 4 strategy leaves the game running in its own language.
            Self::KhatfFashil { .. }
            | Self::RasmGhayrMutah { .. }
            | Self::KhattMarfud { .. }
            | Self::ThaqafaMarfuda { .. }
            | Self::ImtidadMarfud { .. } => Khutura::Tanbeeh,

            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => "تعذّر فتح ملف من ملفات اللعبة أو الكتابة إليه.".to_owned(),
            Self::MalafQaseer { .. } => {
                "أحد ملفات اللعبة أقصر مما تعلنه ترويسته؛ يبدو أنه تالف.".to_owned()
            },
            Self::HajmMufrit { .. } => {
                "أحد ملفات اللعبة يعلن حجمًا أكبر بكثير مما تحتاجه أي لعبة حقيقية، ورُفض \
                 قبل حجز أي ذاكرة له."
                    .to_owned()
            },
            Self::IsdarMajhul { .. } => {
                "تعذّر تحديد إصدار غودو لهذه اللعبة، والاستراتيجية تختلف اختلافًا تامًا بين \
                 الإصدارين الثالث والرابع."
                    .to_owned()
            },
            Self::MuharrikGhayrMadum { .. } => {
                "إصدار غودو في هذه اللعبة خارج ما يتعامل معه هذا المحوّل.".to_owned()
            },
            Self::SihrGhayrMutabaq { .. } => {
                "الملف لا يحمل علامة حزمة غودو؛ إمّا أنه ليس حزمة وإمّا أنه تالف.".to_owned()
            },
            Self::IsdarGhayrMadum { .. } => {
                "إصدار صيغة الحزمة أحدث مما تقرؤه هذه النسخة من تعريب.".to_owned()
            },
            Self::HawiyaTalifa { .. } => {
                "أحد حقول حزمة اللعبة يشير خارج حدودها؛ الملف تالف.".to_owned()
            },
            Self::PckMushaffar { .. } => {
                "حزمة اللعبة مشفَّرة، ولم يُزوَّد تعريب بمفتاح فكّها.".to_owned()
            },
            Self::MiftahGhayrSalih { .. } => "المفتاح المُعطى لا يفكّ تشفير حزمة اللعبة.".to_owned(),
            Self::BasmaGhayrMutabaqa { .. } => {
                "أحد مدخلات الحزمة لا يطابق بصمته المسجَّلة؛ الملف تغيّر أو تلف.".to_owned()
            },
            Self::FakkFashil { .. } => "تعذّر فكّ ضغط أحد مدخلات الحزمة؛ الملف تالف.".to_owned(),
            Self::HajmGhayrMutabaq { .. } => {
                "أحد المدخلات أنتج بعد فكّ الضغط حجمًا غير الذي أعلنه.".to_owned()
            },
            Self::DaghtMajhul { .. } => {
                "أحد المدخلات مضغوط بطريقة لا تعرفها هذه النسخة من تعريب.".to_owned()
            },
            Self::TarjamaTalifa { .. } => {
                "مورد الترجمة داخل الحزمة ليس بالشكل الذي تعرّفه الصيغة.".to_owned()
            },
            Self::NassGhayrSalih { .. } => {
                "أحد النصوص في موارد اللعبة ليس ترميزًا صالحًا، ورُفض بدل استبداله بمحارف \
                 بديلة تُقرأ كأنها ترجمة."
                    .to_owned()
            },
            Self::ImtidadMarfud { .. } => {
                "تعذّر تسجيل امتداد تعريب في غودو؛ تعمل اللعبة بلغتها الأصلية.".to_owned()
            },
            Self::KhattMarfud { .. } => {
                "تعذّر تسجيل خطّ الرقعة، وستُرسم النصوص بخطّ اللعبة الأصلي.".to_owned()
            },
            Self::ThaqafaMarfuda { .. } => {
                "تعذّر تفعيل اللغة العربية تلقائيًا؛ قد تحتاج إلى اختيارها من قائمة اللغات."
                    .to_owned()
            },
            Self::KhatfFashil { .. } => {
                "تعذّر اعتراض رسم النصوص في هذه اللعبة؛ يُجرَّب مسار النقل بدلًا منه.".to_owned()
            },
            Self::RasmGhayrMutah { .. } => {
                "تعذّر الوصول إلى واجهة الرسم في هذه اللعبة؛ يُجرَّب مسار النقل بدلًا منه.".to_owned()
            },
            Self::NaqlMarfud { .. } => {
                "تعذّر توليد جدول الأشكال البديل، ولم يبقَ مسار آخر لهذه اللعبة.".to_owned()
            },
            Self::NaqlMumtali { .. } => {
                "نصوص هذه اللعبة تحتاج أشكالًا أكثر مما يتّسع له جدول النقل، ورُفض التوليد \
                 بدل قصّه وإنتاج حروف خاطئة."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::KhataMalaf { masar, .. } => {
                format!("{} could not be opened or written.", masar.display())
            },
            Self::MalafQaseer { haql, tul, matlub } => format!(
                "A game file is truncated: {haql} needs {matlub} bytes and the file has {tul}."
            ),
            Self::HajmMufrit { haql, qeema, saqf } => format!(
                "A game file declares {haql} = {qeema}, far above the {saqf} any real game \
                 needs; it was refused before any memory was reserved for it."
            ),
            Self::IsdarMajhul { sabab } => format!(
                "The Godot version could not be determined, and the strategy differs \
                 completely between 3.x and 4.x. {sabab}"
            ),
            Self::MuharrikGhayrMadum { wujid } => {
                format!("Godot {wujid} is outside the 3.x and 4.x this adapter handles.")
            },
            Self::SihrGhayrMutabaq { masar } => format!(
                "{} does not carry the PCK magic; it is either not a package or it is \
                 corrupt.",
                masar.display()
            ),
            Self::IsdarGhayrMadum { wujid, aqsa } => {
                format!("This package is PCK version {wujid} and this build reads up to {aqsa}.")
            },
            Self::HawiyaTalifa {
                ism,
                haql,
                qeema,
                hadd,
            } => {
                format!("A {ism} is corrupt: {haql} is {qeema}, outside 0..{hadd}.")
            },
            Self::PckMushaffar { masar } => format!(
                "{} is encrypted and no key was supplied. Taarib does not search for keys; \
                 supply the game's own key to read it.",
                masar.display()
            ),
            Self::MiftahGhayrSalih { masar, sabab } => {
                format!(
                    "The supplied key does not decrypt {}: {sabab}",
                    masar.display()
                )
            },
            Self::BasmaGhayrMutabaqa { madkhal } => format!(
                "Package entry \"{madkhal}\" does not match its recorded checksum; the file \
                 changed or is damaged."
            ),
            Self::FakkFashil { tafsil } => {
                format!("A package entry could not be expanded: {tafsil}")
            },
            Self::HajmGhayrMutabaq { muallan, fili } => {
                format!("An entry declared {muallan} bytes and produced {fili}.")
            },
            Self::DaghtMajhul { naw } => {
                format!("An entry uses compression mode {naw}, which this build cannot expand.")
            },
            Self::TarjamaTalifa { haql } => {
                format!("The translation resource is malformed: {haql}.")
            },
            Self::NassGhayrSalih { fahras, mawqi } => format!(
                "String {fahras} is not valid UTF-8 at byte {mawqi}. It was refused rather \
                 than replaced with substitution characters that would read as a translation."
            ),
            Self::ImtidadMarfud { sabab } => format!(
                "The Taarib extension could not register with Godot, so the game runs in its \
                 original language. {sabab}"
            ),
            Self::KhattMarfud { sabab } => format!(
                "The patch's font could not be registered, so text will draw with the game's \
                 own font. {sabab}"
            ),
            Self::ThaqafaMarfuda { sabab } => format!(
                "Arabic could not be activated automatically; you may need to choose it from \
                 the game's own language menu. {sabab}"
            ),
            Self::KhatfFashil { hadaf, tafsil } => format!(
                "Text drawing could not be intercepted at {hadaf}: {tafsil}. The glyph \
                 transport is tried instead."
            ),
            Self::RasmGhayrMutah { sabab } => format!(
                "The engine's canvas API could not be reached: {sabab}. The glyph transport \
                 is tried instead."
            ),
            Self::NaqlMarfud { sabab } => format!(
                "The glyph transport could not be generated, and no path remains for this \
                 game. {sabab}"
            ),
            Self::NaqlMumtali { matlub, mutah } => format!(
                "This game's text needs {matlub} distinct glyph slots and the transport holds \
                 {mutah}. Generation was refused rather than truncated, because a truncated \
                 glyph table draws the wrong letters."
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::IsdarGhayrMadum { .. } | Self::MuharrikGhayrMadum { .. } => Khutwa::TahdithTaarib,
            Self::MalafQaseer { .. }
            | Self::HawiyaTalifa { .. }
            | Self::NassGhayrSalih { .. }
            | Self::FakkFashil { .. }
            | Self::HajmGhayrMutabaq { .. }
            | Self::BasmaGhayrMutabaqa { .. }
            | Self::TarjamaTalifa { .. }
            | Self::SihrGhayrMutabaq { .. } => Khutwa::TahaqquqSalamatLuba,
            // Spelled out rather than left to a wildcard: a variant added later
            // would otherwise inherit "open diagnostics" silently, which is the
            // right answer for these thirteen and not necessarily for the next.
            Self::PckMushaffar { .. }
            | Self::MiftahGhayrSalih { .. }
            | Self::DaghtMajhul { .. }
            | Self::HajmMufrit { .. }
            | Self::ImtidadMarfud { .. }
            | Self::IsdarMajhul { .. }
            | Self::KhatfFashil { .. }
            | Self::KhattMarfud { .. }
            | Self::NaqlMarfud { .. }
            | Self::NaqlMumtali { .. }
            | Self::RasmGhayrMutah { .. }
            | Self::ThaqafaMarfuda { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        // Handled before the closure below exists, for two reasons. It is the one
        // variant whose context comes from somewhere else — `siyaq_io` builds the
        // I/O half — and the path has to be merged into that rather than into a
        // fresh map, because a bundle that named the error kind and not the file
        // it happened to would be missing the one thing a reader needs. Doing it
        // here also keeps the closure's mutable borrow out of the merge.
        if let Self::KhataMalaf { masar, sabab } = self {
            let mut siyaq = siyaq_io(sabab);
            let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            return siyaq;
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            // Answered above, before the closure borrowed the map.
            Self::KhataMalaf { .. } => {},
            Self::MalafQaseer { haql, tul, matlub } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("tul", QeemaSiyaq::Hajm(*tul));
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
            },
            Self::HajmMufrit { haql, qeema, saqf } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            },
            Self::IsdarMajhul { sabab }
            | Self::ImtidadMarfud { sabab }
            | Self::KhattMarfud { sabab }
            | Self::ThaqafaMarfuda { sabab }
            | Self::RasmGhayrMutah { sabab }
            | Self::NaqlMarfud { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MuharrikGhayrMadum { wujid } => {
                daa("wujid", QeemaSiyaq::Nass(wujid.clone()));
            },
            Self::SihrGhayrMutabaq { masar } | Self::PckMushaffar { masar } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
            },
            Self::IsdarGhayrMadum { wujid, aqsa } => {
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("aqsa", QeemaSiyaq::Raqm(i64::from(*aqsa)));
            },
            Self::HawiyaTalifa {
                ism,
                haql,
                qeema,
                hadd,
            } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("hadd", QeemaSiyaq::Hajm(*hadd));
            },
            Self::MiftahGhayrSalih { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass((*sabab).to_owned()));
            },
            Self::BasmaGhayrMutabaqa { madkhal } => {
                daa("madkhal", QeemaSiyaq::Nass(madkhal.clone()));
            },
            Self::FakkFashil { tafsil } => {
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::HajmGhayrMutabaq { muallan, fili } => {
                daa("muallan", QeemaSiyaq::Hajm(*muallan));
                daa("fili", QeemaSiyaq::Hajm(*fili));
            },
            Self::DaghtMajhul { naw } => {
                daa("daght", QeemaSiyaq::Raqm(i64::from(*naw)));
            },
            Self::TarjamaTalifa { haql } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
            },
            Self::NassGhayrSalih { fahras, mawqi } => {
                daa("fahras", QeemaSiyaq::Raqm(i64::from(*fahras)));
                daa("mawqi", QeemaSiyaq::Raqm(i64::from(*mawqi)));
            },
            Self::KhatfFashil { hadaf, tafsil } => {
                daa("hadaf", QeemaSiyaq::Nass((*hadaf).to_owned()));
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::NaqlMumtali { matlub, mutah } => {
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
                daa("mutah", QeemaSiyaq::Hajm(*mutah));
            },
        }
        siyaq
    }
}

khata_min!(KhataGodot);

/// A refusal from the glyph transport, said in the adapter's own vocabulary.
///
/// `naql` moved to `taarib-lawha` when GameMaker turned out to need the same
/// capability, and it refuses in that crate's error type. The Godot 3 ladder
/// reports in this one, so the five refusals the transport raises are delegated
/// onto the five variants this module already had for exactly them — the same
/// fields, the same sentences, the same permanent codes. Nothing was renumbered
/// and nothing a user has already pasted into a bug report changed meaning.
///
/// Anything else `taarib-lawha` can refuse — a glyph larger than a page, an
/// exhausted page budget — reaches this adapter only *through* the transport,
/// and it does end the ladder, so it arrives as
/// [`KhataGodot::NaqlMarfud`]. Its own permanent code travels in the sentence
/// rather than being dropped, because a reader who is told `TAARIB-E-3002` can
/// look up the atlas failure that actually happened, and a reader told only
/// "the transport was refused" cannot.
impl From<KhataLawha> for KhataGodot {
    fn from(khata: KhataLawha) -> Self {
        match khata {
            KhataLawha::NaqlMarfud { sabab } => Self::NaqlMarfud { sabab },
            KhataLawha::NaqlMumtali { matlub, mutah } => Self::NaqlMumtali { matlub, mutah },
            KhataLawha::KhattMarfud { sabab } => Self::KhattMarfud { sabab },
            KhataLawha::HajmMufrit { haql, qeema, saqf } => Self::HajmMufrit { haql, qeema, saqf },
            KhataLawha::KhataMalaf { masar, sabab } => Self::KhataMalaf { masar, sabab },
            akhar => Self::NaqlMarfud {
                sabab: format!("{} {}", akhar.ramz(), akhar.injilizi()),
            },
        }
    }
}

/// A length as a `u64`, saturating on a platform where `usize` is wider — which
/// is none this product targets, and is still not a reason to write a cast the
/// compiler cannot prove.
#[must_use]
pub fn tul_u64(tul: usize) -> u64 {
    u64::try_from(tul).unwrap_or(u64::MAX)
}

/// A container offset as a `usize`, or [`None`] when it does not fit — which on
/// a 32-bit target is the ordinary case for a hostile length, not an edge case.
#[must_use]
pub fn hajm_usize(qeema: u64) -> Option<usize> {
    usize::try_from(qeema).ok()
}
