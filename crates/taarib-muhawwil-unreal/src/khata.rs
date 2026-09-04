//! أخطاء محوّل أنريل — what stops this adapter, and what merely narrows it.
//!
//! The Unreal adapter has two halves with very different failure characters,
//! and the split runs through this file.
//!
//! The **offline half** — reading and writing `.locres`, `.locmeta`, string
//! tables, `.pak` and IoStore containers — is parsing files somebody else
//! produced. Every failure there is a refusal that names a field, because the
//! alternative to refusing is writing a container the engine will mount and
//! then fail to read, which surfaces to a player as a game that no longer
//! starts. There is nothing graceful about a half-written `.pak`.
//!
//! The **runtime half** — forcing full shaping, registering the font,
//! correcting measurement — is asking a live process to behave differently.
//! Failures there degrade one capability and are reported as warnings, because
//! a game whose menus are Arabic but whose wrapping is Slate's own is still a
//! translated game, and refusing to run at all would be trading a flaw for a
//! total loss.
//!
//! So: [`Khutura::Khatar`] for anything that touches a container, and
//! [`Khutura::Tanbeeh`] for the runtime corrections that can be individually
//! absent. The one exception is [`KhataUnreal::MiftahGhayrSalih`], which is a
//! failure the *user* can fix by supplying the right key, and which therefore
//! carries an action rather than an apology.
//!
//! ## Error codes
//!
//! The adapters share [`arqam::MUHAWWIL`] (4300); `AWWAL` gives this crate 4320–4339.
//! module starts at twenty and leaves the gap deliberately: a permanent code is
//! permanent, users paste them into bug reports, and two subsystems answering
//! to one number is a search that returns the wrong page years later.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The first code this module uses within the adapters' band.
///
/// Twenty rather than zero, because 4300 through 4309 are the Unity capture
/// path's. See this module's header.
const AWWAL: u16 = 20;

/// Failures of the Unreal adapter.
#[derive(Debug, thiserror::Error)]
pub enum KhataUnreal {
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
    ///
    /// Every one of these ceilings is checked against the *declared* number
    /// before a byte is reserved. A localization file is a file that arrived
    /// with a game, and a count field in it is a number that decides how much
    /// memory this process is about to take.
    #[error("{haql} declares {qeema}, above the ceiling of {saqf}")]
    HajmMufrit {
        /// Which field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// The engine version could not be determined.
    ///
    /// Not fatal on its own: the container readers key off their own format
    /// versions, which are written into the files. This matters for the runtime
    /// half, where a Slate correction is version-specific.
    #[error("the Unreal version could not be determined: {sabab}")]
    IsdarMajhul {
        /// What every source reported, in one sentence.
        sabab: String,
    },

    /// A localization file does not begin with the bytes the format defines.
    #[error("{malaf} does not carry the {ism} magic")]
    SihrGhayrMutabaq {
        /// Which file.
        malaf: PathBuf,
        /// Which format was expected.
        ism: &'static str,
    },

    /// A file's format version is one this build does not read.
    #[error("{ism} version {wujid} is above the {aqsa} this build reads")]
    IsdarGhayrMadum {
        /// Which format.
        ism: &'static str,
        /// The version found.
        wujid: u32,
        /// The highest version this build reads.
        aqsa: u32,
    },

    /// A field inside a localization resource is not consistent with the file.
    #[error("{ism}: {haql} is {qeema}, outside 0..{hadd}")]
    MawridTalif {
        /// Which format.
        ism: &'static str,
        /// Which field.
        haql: &'static str,
        /// What it said.
        qeema: u64,
        /// The bound it broke.
        hadd: u64,
    },

    /// A string in a localization resource is not valid UTF-16.
    ///
    /// Refused rather than replaced. A lossy conversion would put replacement
    /// characters into a game's dialogue and call it a translation.
    #[error("string {fahras} is not valid UTF-16 at unit {mawqi}")]
    NassGhayrSalih {
        /// Which record.
        fahras: u32,
        /// The first code unit that is not valid.
        mawqi: u32,
    },

    /// A `.pak` is encrypted and no key was supplied.
    ///
    /// The one failure in this module that is entirely the user's to resolve,
    /// and it is not a defeat of anything: an encrypted `.pak` is encrypted
    /// against casual inspection, the key ships inside the game's own
    /// executable, and a player extracting their own game's text is doing
    /// nothing this product needs to be shy about. Taarib does not go looking
    /// for the key, because a tool that recovered keys would be a tool with a
    /// second purpose. The user supplies it or the container is not read.
    #[error("{masar} is encrypted and no AES key was supplied")]
    PakMushaffar {
        /// Which container.
        masar: PathBuf,
    },

    /// The supplied AES key is not the right shape, or does not decrypt.
    #[error("the AES key does not decrypt {masar}")]
    MiftahGhayrSalih {
        /// Which container.
        masar: PathBuf,
        /// Why, in one sentence.
        sabab: &'static str,
    },

    /// A container declares a compression method this build cannot expand.
    ///
    /// Oodle is the usual one, and the reason is a licence rather than an
    /// engineering limit: Taarib may not ship an Oodle decompressor. When the
    /// game's own module exports one, it is used; when it does not, this is the
    /// refusal.
    #[error("{ism} block uses compression \"{naw}\", which this build cannot expand")]
    DaghtMajhul {
        /// Which container format.
        ism: &'static str,
        /// The method name the container declared.
        naw: String,
    },

    /// Decompressing a container block failed.
    #[error("{ism}: a compressed block could not be expanded")]
    FakkFashil {
        /// Which container format.
        ism: &'static str,
        /// What the decompressor reported.
        tafsil: String,
    },

    /// A block expanded to a size other than the one it declared.
    #[error("{ism}: a block declared {muallan} bytes and produced {fili}")]
    HajmGhayrMutabaq {
        /// Which container format.
        ism: &'static str,
        /// What it declared.
        muallan: u64,
        /// What it produced.
        fili: u64,
    },

    /// A container entry's hash does not match its bytes.
    #[error("{ism}: entry \"{madkhal}\" does not match its recorded hash")]
    BasmaGhayrMutabaqa {
        /// Which container format.
        ism: &'static str,
        /// Which entry.
        madkhal: String,
    },

    /// Slate could not be reached in this process.
    ///
    /// A warning rather than a failure: the localization resources are already
    /// in place by the time this is attempted, so the game shows Arabic text.
    /// What is lost is the shaping correction, and a game rendering unshaped
    /// Arabic is a game whose problem is visible and diagnosable rather than a
    /// game that did not start.
    #[error("Slate could not be reached in this process: {sabab}")]
    SlateGhayrMawjud {
        /// Why, naming what was tried.
        sabab: String,
    },

    /// A detour could not be installed.
    #[error("the hook on {hadaf} could not be installed: {tafsil}")]
    KhatfFashil {
        /// Which function.
        hadaf: &'static str,
        /// What the hooking layer reported.
        tafsil: String,
    },

    /// Full shaping could not be forced on.
    ///
    /// The single most consequential runtime correction: without it Slate uses
    /// its fast non-shaping path, and Arabic drawn through that path is
    /// unjoined letters in visual order — which is the exact failure this
    /// product exists to prevent.
    #[error("full text shaping could not be forced on: {sabab}")]
    TashghilFashil {
        /// Why, naming every route that was tried.
        sabab: String,
    },

    /// The bundled font could not be registered with Slate.
    #[error("the patch's font could not be registered with Slate: {sabab}")]
    KhattMarfud {
        /// Why.
        sabab: String,
    },

    /// The culture could not be added or made active.
    #[error("the Arabic culture could not be activated: {sabab}")]
    AlamMarfud {
        /// Why.
        sabab: String,
    },

    /// A configuration file could not be updated.
    ///
    /// The injection-free routes — forcing shaping through `[SystemSettings]`,
    /// selecting the culture through `[Internationalization]` — are writes into
    /// the game's own ini files. They are additive, reversible, and the first
    /// thing tried, so a failure here pushes the adapter onto the injected
    /// route rather than stopping it.
    #[error("{masar} could not be updated: {sabab}")]
    IdadatMarfuda {
        /// Which file.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },
}

impl Tafsir for KhataUnreal {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUHAWWIL
                + AWWAL
                + match self {
                    Self::KhataMalaf { .. } => 0,
                    Self::MalafQaseer { .. } => 1,
                    Self::HajmMufrit { .. } => 2,
                    Self::IsdarMajhul { .. } => 3,
                    Self::SihrGhayrMutabaq { .. } => 4,
                    Self::IsdarGhayrMadum { .. } => 5,
                    Self::MawridTalif { .. } => 6,
                    Self::NassGhayrSalih { .. } => 7,
                    Self::PakMushaffar { .. } => 8,
                    Self::MiftahGhayrSalih { .. } => 9,
                    Self::DaghtMajhul { .. } => 10,
                    Self::FakkFashil { .. } => 11,
                    Self::HajmGhayrMutabaq { .. } => 12,
                    Self::BasmaGhayrMutabaqa { .. } => 13,
                    Self::SlateGhayrMawjud { .. } => 14,
                    Self::KhatfFashil { .. } => 15,
                    Self::TashghilFashil { .. } => 16,
                    Self::KhattMarfud { .. } => 17,
                    Self::AlamMarfud { .. } => 18,
                    Self::IdadatMarfuda { .. } => 19,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // The runtime corrections each degrade one capability. The game is
            // already showing Arabic by the time any of them runs, because the
            // localization resources went in offline.
            Self::SlateGhayrMawjud { .. }
            | Self::KhatfFashil { .. }
            | Self::KhattMarfud { .. }
            | Self::AlamMarfud { .. }
            | Self::IdadatMarfuda { .. } => Khutura::Tanbeeh,

            // Everything else either produces a container the engine will
            // choke on, or means the text itself is wrong.
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => {
                "تعذّر فتح ملف من ملفات اللعبة أو الكتابة إليه.".to_owned()
            }
            Self::MalafQaseer { .. } => {
                "أحد ملفات الترجمة في اللعبة أقصر مما تعلنه ترويسته؛ يبدو أنه تالف.".to_owned()
            }
            Self::HajmMufrit { .. } => {
                "أحد ملفات اللعبة يعلن حجمًا أكبر بكثير مما تحتاجه أي لعبة حقيقية، ورُفض \
                 قبل حجز أي ذاكرة له."
                    .to_owned()
            }
            Self::IsdarMajhul { .. } => {
                "تعذّر تحديد إصدار محرّك أنريل لهذه اللعبة، فبعض التصحيحات الخاصة بالإصدار \
                 لن تُطبَّق."
                    .to_owned()
            }
            Self::SihrGhayrMutabaq { .. } => {
                "ملف لا يحمل العلامة التي تبدأ بها ملفات هذه الصيغة؛ إمّا أنه ليس منها وإمّا \
                 أنه تالف."
                    .to_owned()
            }
            Self::IsdarGhayrMadum { .. } => {
                "إصدار هذه الصيغة أحدث مما تقرؤه هذه النسخة من تعريب.".to_owned()
            }
            Self::MawridTalif { .. } => {
                "أحد موارد الترجمة في اللعبة يشير خارج حدوده؛ الملف تالف.".to_owned()
            }
            Self::NassGhayrSalih { .. } => {
                "أحد النصوص في موارد اللعبة ليس ترميزًا صالحًا، ورُفض بدل استبداله بمحارف \
                 بديلة تُقرأ كأنها ترجمة."
                    .to_owned()
            }
            Self::PakMushaffar { .. } => {
                "حاوية اللعبة مشفَّرة، ولم يُزوَّد تعريب بمفتاح فكّها.".to_owned()
            }
            Self::MiftahGhayrSalih { .. } => {
                "المفتاح المُعطى لا يفكّ تشفير حاوية اللعبة.".to_owned()
            }
            Self::DaghtMajhul { .. } => {
                "إحدى كتل الحاوية مضغوطة بطريقة لا يملك تعريب رخصة شحن فاكّها، ولم يجد فاكًّا \
                 في اللعبة نفسها."
                    .to_owned()
            }
            Self::FakkFashil { .. } => {
                "تعذّر فكّ ضغط إحدى كتل حاوية اللعبة؛ الملف تالف.".to_owned()
            }
            Self::HajmGhayrMutabaq { .. } => {
                "إحدى الكتل أنتجت بعد فكّ الضغط حجمًا غير الذي أعلنته.".to_owned()
            }
            Self::BasmaGhayrMutabaqa { .. } => {
                "أحد مدخلات الحاوية لا يطابق بصمته المسجَّلة؛ الملف تغيّر أو تلف.".to_owned()
            }
            Self::SlateGhayrMawjud { .. } => {
                "تعذّر الوصول إلى نظام النصوص في اللعبة، فلن تُطبَّق تصحيحات التشكيل. النص \
                 العربي مثبَّت وسيظهر."
                    .to_owned()
            }
            Self::KhatfFashil { .. } => {
                "تعذّر تركيب أحد التصحيحات داخل اللعبة؛ يعمل الباقي.".to_owned()
            }
            Self::TashghilFashil { .. } => {
                "تعذّر تشغيل التشكيل الكامل في هذه اللعبة، وقد تظهر الحروف العربية غير \
                 متّصلة."
                    .to_owned()
            }
            Self::KhattMarfud { .. } => {
                "تعذّر تسجيل خطّ الرقعة في اللعبة، وستُرسم النصوص بخطّ اللعبة الأصلي.".to_owned()
            }
            Self::AlamMarfud { .. } => {
                "تعذّر تفعيل اللغة العربية في اللعبة تلقائيًا؛ قد تحتاج إلى اختيارها من قائمة \
                 اللغات."
                    .to_owned()
            }
            Self::IdadatMarfuda { .. } => {
                "تعذّرت الكتابة في ملف إعدادات اللعبة؛ جُرِّب مسار آخر.".to_owned()
            }
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::KhataMalaf { masar, .. } => {
                format!("{} could not be opened or written.", masar.display())
            }
            Self::MalafQaseer { haql, tul, matlub } => format!(
                "A localization file is truncated: {haql} needs {matlub} bytes and the file \
                 has {tul}."
            ),
            Self::HajmMufrit { haql, qeema, saqf } => format!(
                "A game file declares {haql} = {qeema}, far above the {saqf} any real game \
                 needs; it was refused before any memory was reserved for it."
            ),
            Self::IsdarMajhul { sabab } => format!(
                "The Unreal version of this game could not be determined, so the \
                 version-specific corrections will not be applied. {sabab}"
            ),
            Self::SihrGhayrMutabaq { malaf, ism } => format!(
                "{} does not begin with the {ism} magic; it is either not that format or it \
                 is corrupt.",
                malaf.display()
            ),
            Self::IsdarGhayrMadum { ism, wujid, aqsa } => format!(
                "This {ism} is version {wujid} and this build reads up to {aqsa}."
            ),
            Self::MawridTalif { ism, haql, qeema, hadd } => format!(
                "A {ism} resource is corrupt: {haql} is {qeema}, outside 0..{hadd}."
            ),
            Self::NassGhayrSalih { fahras, mawqi } => format!(
                "String {fahras} in the game's resources is not valid UTF-16 at code unit \
                 {mawqi}. It was refused rather than replaced with substitution characters \
                 that would read as a translation."
            ),
            Self::PakMushaffar { masar } => format!(
                "{} is encrypted and no AES key was supplied. Taarib does not search for \
                 keys; supply the game's own key to read it.",
                masar.display()
            ),
            Self::MiftahGhayrSalih { masar, sabab } => format!(
                "The supplied AES key does not decrypt {}: {sabab}",
                masar.display()
            ),
            Self::DaghtMajhul { ism, naw } => format!(
                "A {ism} block uses compression \"{naw}\". Taarib has no licence to ship a \
                 decompressor for it and found none exported by the game."
            ),
            Self::FakkFashil { ism, tafsil } => {
                format!("A {ism} block could not be expanded: {tafsil}")
            }
            Self::HajmGhayrMutabaq { ism, muallan, fili } => format!(
                "A {ism} block declared {muallan} bytes and produced {fili}."
            ),
            Self::BasmaGhayrMutabaqa { ism, madkhal } => format!(
                "{ism} entry \"{madkhal}\" does not match its recorded hash; the file changed \
                 or is damaged."
            ),
            Self::SlateGhayrMawjud { sabab } => format!(
                "Slate could not be reached in this process, so the shaping corrections were \
                 not applied. The Arabic text is installed and will display. {sabab}"
            ),
            Self::KhatfFashil { hadaf, tafsil } => format!(
                "The correction on {hadaf} could not be installed: {tafsil}. The rest are \
                 running."
            ),
            Self::TashghilFashil { sabab } => format!(
                "Full text shaping could not be forced on for this game, so Arabic letters \
                 may appear unjoined. {sabab}"
            ),
            Self::KhattMarfud { sabab } => format!(
                "The patch's font could not be registered, so text will draw with the game's \
                 own font. {sabab}"
            ),
            Self::AlamMarfud { sabab } => format!(
                "Arabic could not be activated automatically; you may need to choose it from \
                 the game's own language menu. {sabab}"
            ),
            Self::IdadatMarfuda { masar, sabab } => format!(
                "{} could not be written: {sabab}. Another route was tried.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::IsdarGhayrMadum { .. } => Khutwa::TahdithTaarib,
            Self::MalafQaseer { .. }
            | Self::MawridTalif { .. }
            | Self::NassGhayrSalih { .. }
            | Self::FakkFashil { .. }
            | Self::HajmGhayrMutabaq { .. }
            | Self::BasmaGhayrMutabaqa { .. }
            | Self::SihrGhayrMutabaq { .. } => Khutwa::TahaqquqSalamatLuba,
            // Spelled out rather than left to a wildcard: a variant added later
            // would otherwise inherit "open diagnostics" silently, which is the
            // right answer for these eleven and not necessarily for the twelfth.
            Self::PakMushaffar { .. }
            | Self::MiftahGhayrSalih { .. }
            | Self::AlamMarfud { .. }
            | Self::DaghtMajhul { .. }
            | Self::HajmMufrit { .. }
            | Self::IdadatMarfuda { .. }
            | Self::IsdarMajhul { .. }
            | Self::KhatfFashil { .. }
            | Self::KhattMarfud { .. }
            | Self::SlateGhayrMawjud { .. }
            | Self::TashghilFashil { .. } => Khutwa::FathTashkhis,
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
            Self::KhataMalaf { .. } => {}
            Self::MalafQaseer { haql, tul, matlub } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("tul", QeemaSiyaq::Hajm(*tul));
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
            }
            Self::HajmMufrit { haql, qeema, saqf } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            }
            Self::IsdarMajhul { sabab }
            | Self::SlateGhayrMawjud { sabab }
            | Self::TashghilFashil { sabab }
            | Self::KhattMarfud { sabab }
            | Self::AlamMarfud { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::SihrGhayrMutabaq { malaf, ism } => {
                daa("malaf", QeemaSiyaq::Masar(malaf.clone()));
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
            }
            Self::IsdarGhayrMadum { ism, wujid, aqsa } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("aqsa", QeemaSiyaq::Raqm(i64::from(*aqsa)));
            }
            Self::MawridTalif { ism, haql, qeema, hadd } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("hadd", QeemaSiyaq::Hajm(*hadd));
            }
            Self::NassGhayrSalih { fahras, mawqi } => {
                daa("fahras", QeemaSiyaq::Raqm(i64::from(*fahras)));
                daa("mawqi", QeemaSiyaq::Raqm(i64::from(*mawqi)));
            }
            Self::PakMushaffar { masar } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
            }
            Self::MiftahGhayrSalih { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass((*sabab).to_owned()));
            }
            Self::DaghtMajhul { ism, naw } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("daght", QeemaSiyaq::Nass(naw.clone()));
            }
            Self::FakkFashil { ism, tafsil } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            }
            Self::HajmGhayrMutabaq { ism, muallan, fili } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("muallan", QeemaSiyaq::Hajm(*muallan));
                daa("fili", QeemaSiyaq::Hajm(*fili));
            }
            Self::BasmaGhayrMutabaqa { ism, madkhal } => {
                daa("sigha", QeemaSiyaq::Nass((*ism).to_owned()));
                daa("madkhal", QeemaSiyaq::Nass(madkhal.clone()));
            }
            Self::KhatfFashil { hadaf, tafsil } => {
                daa("hadaf", QeemaSiyaq::Nass((*hadaf).to_owned()));
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            }
            Self::IdadatMarfuda { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataUnreal);

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
