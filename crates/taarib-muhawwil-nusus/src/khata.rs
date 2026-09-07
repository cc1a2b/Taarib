//! أخطاء محوّلات النصوص — what stops a patcher, and what merely picks a
//! different rung.
//!
//! This crate patches files rather than processes, and that changes the shape of
//! failure completely. Nothing here degrades gracefully in the way a runtime
//! hook does: a `data.win` written with one offset wrong is a game that will not
//! start, and there is no partial success to salvage. So the default posture in
//! this module is refusal, and it is stricter than in any adapter that came
//! before it.
//!
//! Three kinds of failure live here, and they are deliberately not equal.
//!
//! **Container failures** — a chunk that runs past the file, an index that does
//! not check out, an archive key that does not decrypt — are always
//! [`Khutura::Khatar`]. The alternative to refusing is writing a container the
//! engine mounts and then chokes on.
//!
//! **Rung failures** — an engine that turned out not to shape after all, a hook
//! point that is absent, a font that could not be registered — are
//! [`Khutura::Tanbeeh`], because the tier probe's whole purpose is to descend to
//! a rung that works. A rung declining is the ladder functioning, not the
//! adapter failing.
//!
//! **Reversibility failures** are the strictest of all, and they are the reason
//! [`KhataNusus::NuskhaMafquda`] and [`KhataNusus::IstiadaGhayrMutabaqa`] exist.
//! This crate modifies files that are the game. If the backup that would undo
//! that modification cannot be written, or a restore does not reproduce the
//! recorded fingerprint, the correct behaviour is to stop before touching
//! anything rather than to proceed and hope. A patcher that can install and
//! cannot uninstall is a patcher that has taken somebody's game hostage.
//!
//! ## Error codes
//!
//! The adapters share [`arqam::MUHAWWIL`] (4300); `AWWAL` gives this crate 4370–4378.
//! module starts at seventy. A permanent code is permanent — users paste them
//! into bug reports and search for them years later — so the bands never
//! overlap and the gaps are left deliberately.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The first code this module uses within the adapters' band.
///
/// Seventy, leaving Unity's ten, Unreal's twenty and Godot's twenty-three
/// untouched. See this module's header.
const AWWAL: u16 = 70;

/// Failures of the script-engine patchers.
#[derive(Debug, thiserror::Error)]
pub enum KhataNusus {
    /// A file could not be opened, read or written.
    #[error("{masar} could not be read or written")]
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
    /// Checked against the *declared* number before a byte is reserved. A game's
    /// data file is a file somebody else produced, and a count field in it is a
    /// number that decides how much memory this process is about to take.
    #[error("{haql} declares {qeema}, above the ceiling of {saqf}")]
    HajmMufrit {
        /// Which field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// A file does not begin with the bytes its format defines.
    #[error("{masar} does not carry the {sigha} signature")]
    SihrGhayrMutabaq {
        /// Which file.
        masar: PathBuf,
        /// Which format was expected.
        sigha: &'static str,
    },

    /// A format version is one this build does not read.
    #[error("{sigha} version {wujid} is outside the {adna}..={aqsa} this build reads")]
    IsdarGhayrMadum {
        /// Which format.
        sigha: &'static str,
        /// The version found.
        wujid: u32,
        /// The lowest version this build reads.
        adna: u32,
        /// The highest.
        aqsa: u32,
    },

    /// A field inside a container is not consistent with the file around it.
    #[error("{sigha}: {haql} is {qeema}, outside 0..{hadd}")]
    HawiyaTalifa {
        /// Which format.
        sigha: &'static str,
        /// Which field.
        haql: &'static str,
        /// What it said.
        qeema: u64,
        /// The bound it broke.
        hadd: u64,
    },

    /// An archive is obfuscated or encrypted and the key could not be obtained.
    ///
    /// For these engines the key is normally recoverable from the game's own
    /// data — RGSSAD carries a seed in its header, RPG Maker MV writes its key
    /// into `System.json`. This fires when that recovery failed, not when a key
    /// was withheld: Taarib reads a key the game itself publishes and does not
    /// attempt to derive one that it does not.
    #[error("{masar} is obfuscated and no key could be recovered from the game's own data")]
    MiftahMafqud {
        /// Which archive.
        masar: PathBuf,
        /// Where the key was looked for.
        sabab: String,
    },

    /// A recovered key does not produce sensible plaintext.
    #[error("the key recovered for {masar} does not decode it")]
    MiftahGhayrSalih {
        /// Which archive.
        masar: PathBuf,
        /// How that was determined.
        sabab: &'static str,
    },

    /// Decompressing part of a container failed.
    #[error("{sigha}: a compressed section could not be expanded")]
    FakkFashil {
        /// Which format.
        sigha: &'static str,
        /// What the decompressor reported.
        tafsil: String,
    },

    /// A section expanded to a size other than the one it declared.
    #[error("{sigha}: a section declared {muallan} bytes and produced {fili}")]
    HajmGhayrMutabaq {
        /// Which format.
        sigha: &'static str,
        /// What it declared.
        muallan: u64,
        /// What it produced.
        fili: u64,
    },

    /// A structured data file is not the shape its engine requires.
    ///
    /// The JSON was valid JSON, or the Marshal stream was a valid Marshal
    /// stream, and the thing inside it was not what the engine reads — a map
    /// whose event list is a number, a database row that is not an object.
    #[error("{malaf}: {haql} is not the shape this engine reads")]
    BunyaGhayrMutawaqqaa {
        /// Which file.
        malaf: String,
        /// Which member.
        haql: String,
    },

    /// A string in a game's data is not valid text in its declared encoding.
    ///
    /// Refused rather than replaced. A lossy conversion would put replacement
    /// characters into a game's dialogue and call it a translation.
    #[error("a string in {malaf} is not valid {tarmiz} at byte {mawqi}")]
    NassGhayrSalih {
        /// Which file.
        malaf: String,
        /// Which encoding it was supposed to be.
        tarmiz: &'static str,
        /// The first byte that is not valid.
        mawqi: u64,
    },

    /// Rewriting a container would not round-trip.
    ///
    /// Raised by a writer that has just re-serialized what it read and found the
    /// bytes differ in a region nothing was supposed to change. It is the only
    /// honest check available for a format this crate reconstructs rather than
    /// edits in place, and it fires before anything reaches the game's
    /// directory.
    #[error("{sigha}: re-serializing an untouched region changed {adad} byte(s)")]
    DawraGhayrMutabaqa {
        /// Which format.
        sigha: &'static str,
        /// How many bytes differ.
        adad: u64,
    },

    /// The original of a file about to be modified could not be preserved.
    ///
    /// Fatal, always, and before any write. This crate modifies files that *are*
    /// the game; proceeding without a restorable original would mean a patch
    /// that can install and cannot uninstall.
    #[error("the original of {masar} could not be preserved, so it was not modified")]
    NuskhaMafquda {
        /// Which file.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// A restore did not reproduce the fingerprint that was recorded.
    ///
    /// The uninstall equivalent, and equally fatal. A restore that is assumed
    /// rather than verified is a restore that silently leaves a modified game
    /// behind.
    #[error("{masar} was restored and does not match its recorded fingerprint")]
    IstiadaGhayrMutabaqa {
        /// Which file.
        masar: PathBuf,
        /// What was recorded.
        muallana: String,
        /// What the restored bytes hash to.
        mahsuba: String,
    },

    /// The tier probe could not decide which rung applies.
    ///
    /// Distinct from an engine being unsupported: the engine is known and the
    /// evidence about its shaping is contradictory or absent. Refusing is right,
    /// because guessing high takes over an engine that shapes correctly and
    /// guessing low leaves Arabic unjoined.
    #[error("the shaping tier could not be established: {sabab}")]
    TabaqaMajhula {
        /// What each source reported.
        sabab: String,
    },

    /// A payload could not be installed through the engine's own mechanism.
    ///
    /// A rung declining rather than the adapter failing: the plugin list could
    /// not be edited, the translation directory could not be created, the script
    /// list had no room. The caller descends.
    #[error("the {alia} payload could not be installed: {sabab}")]
    HimlMarfud {
        /// Which mechanism was tried.
        alia: &'static str,
        /// Why.
        sabab: String,
    },

    /// The bundled font could not be installed for this engine.
    #[error("the patch's font could not be installed: {sabab}")]
    KhattMarfud {
        /// Why.
        sabab: String,
    },

    /// A font's glyph table could not be rebuilt to hold the patch's glyphs.
    ///
    /// GameMaker's case specifically: the font chunk is a baked glyph table
    /// indexed by character, and rebuilding it means rewriting every offset that
    /// points into it from elsewhere in the container.
    #[error("the font glyph table could not be rebuilt: {sabab}")]
    JadwalAshkalMarfud {
        /// Why.
        sabab: String,
    },
}

impl Tafsir for KhataNusus {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUHAWWIL
                + AWWAL
                + match self {
                    Self::KhataMalaf { .. } => 0,
                    Self::MalafQaseer { .. } => 1,
                    Self::HajmMufrit { .. } => 2,
                    Self::SihrGhayrMutabaq { .. } => 3,
                    Self::IsdarGhayrMadum { .. } => 4,
                    Self::HawiyaTalifa { .. } => 5,
                    Self::MiftahMafqud { .. } => 6,
                    Self::MiftahGhayrSalih { .. } => 7,
                    Self::FakkFashil { .. } => 8,
                    Self::HajmGhayrMutabaq { .. } => 9,
                    Self::BunyaGhayrMutawaqqaa { .. } => 10,
                    Self::NassGhayrSalih { .. } => 11,
                    Self::DawraGhayrMutabaqa { .. } => 12,
                    Self::NuskhaMafquda { .. } => 13,
                    Self::IstiadaGhayrMutabaqa { .. } => 14,
                    Self::TabaqaMajhula { .. } => 15,
                    Self::HimlMarfud { .. } => 16,
                    Self::KhattMarfud { .. } => 17,
                    Self::JadwalAshkalMarfud { .. } => 18,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // A rung declining is the ladder working. The probe descends.
            Self::HimlMarfud { .. }
            | Self::KhattMarfud { .. }
            | Self::JadwalAshkalMarfud { .. } => Khutura::Tanbeeh,

            // Everything else either writes a broken game file or means the
            // patch cannot be undone, and both are worse than not patching.
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => "تعذّر فتح ملف من ملفات اللعبة أو الكتابة إليه.".to_owned(),
            Self::MalafQaseer { .. } => {
                "أحد ملفات اللعبة أقصر مما تعلنه ترويسته؛ يبدو أنه تالف أو ناقص.".to_owned()
            },
            Self::HajmMufrit { .. } => {
                "أحد ملفات اللعبة يعلن حجمًا أكبر بكثير مما تحتاجه أي لعبة حقيقية، ورُفض \
                 قبل حجز أي ذاكرة له."
                    .to_owned()
            },
            Self::SihrGhayrMutabaq { .. } => {
                "ملف لا يحمل العلامة التي تبدأ بها ملفات هذه الصيغة؛ إمّا أنه ليس منها \
                 وإمّا أنه تالف."
                    .to_owned()
            },
            Self::IsdarGhayrMadum { .. } => {
                "إصدار هذه الصيغة خارج ما تقرؤه هذه النسخة من تعريب.".to_owned()
            },
            Self::HawiyaTalifa { .. } => {
                "أحد حقول حاوية اللعبة يشير خارج حدودها؛ الملف تالف.".to_owned()
            },
            Self::MiftahMafqud { .. } => {
                "أرشيف اللعبة مموَّه، ولم يُعثر على مفتاحه في بيانات اللعبة نفسها.".to_owned()
            },
            Self::MiftahGhayrSalih { .. } => {
                "المفتاح المستخرَج من بيانات اللعبة لا يفكّ أرشيفها.".to_owned()
            },
            Self::FakkFashil { .. } => "تعذّر فكّ ضغط جزء من ملفات اللعبة؛ الملف تالف.".to_owned(),
            Self::HajmGhayrMutabaq { .. } => {
                "أحد الأجزاء أنتج بعد فكّ الضغط حجمًا غير الذي أعلنه.".to_owned()
            },
            Self::BunyaGhayrMutawaqqaa { .. } => {
                "أحد ملفات بيانات اللعبة ليس بالبنية التي يقرؤها محرّكها؛ قد يكون معدَّلًا \
                 بأداة أخرى."
                    .to_owned()
            },
            Self::NassGhayrSalih { .. } => {
                "أحد النصوص في بيانات اللعبة ليس ترميزًا صالحًا، ورُفض بدل استبداله بمحارف \
                 بديلة تُقرأ كأنها ترجمة."
                    .to_owned()
            },
            Self::DawraGhayrMutabaqa { .. } => {
                "تعذّر إعادة كتابة ملف اللعبة كما كان تمامًا، فأُوقف الترقيع قبل المساس به. \
                 هذا يحمي الملف بدل المخاطرة به."
                    .to_owned()
            },
            Self::NuskhaMafquda { .. } => {
                "تعذّر حفظ نسخة أصلية من الملف قبل تعديله، فلم يُعدَّل. لا يعدّل تعريب ملفًا \
                 لا يستطيع إرجاعه."
                    .to_owned()
            },
            Self::IstiadaGhayrMutabaqa { .. } => {
                "أُعيد الملف الأصلي ولم تطابق بصمته ما سُجِّل له؛ لم تكتمل إزالة الترقيع.".to_owned()
            },
            Self::TabaqaMajhula { .. } => {
                "تعذّر تحديد ما إذا كان هذا المحرّك يشكّل العربية بنفسه، ولا يخمّن تعريب: \
                 التخمين إمّا يستولي على محرّك سليم وإمّا يترك الحروف غير متّصلة."
                    .to_owned()
            },
            Self::HimlMarfud { .. } => {
                "تعذّر تركيب حمولة تعريب عبر آلية المحرّك نفسه؛ يُجرَّب مسار آخر.".to_owned()
            },
            Self::KhattMarfud { .. } => "تعذّر تركيب خطّ الرقعة في هذه اللعبة.".to_owned(),
            Self::JadwalAshkalMarfud { .. } => {
                "تعذّرت إعادة بناء جدول أشكال الخطّ داخل ملف اللعبة.".to_owned()
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
            Self::SihrGhayrMutabaq { masar, sigha } => format!(
                "{} does not carry the {sigha} signature; it is either not that format or it \
                 is corrupt.",
                masar.display()
            ),
            Self::IsdarGhayrMadum {
                sigha,
                wujid,
                adna,
                aqsa,
            } => format!(
                "This {sigha} is version {wujid} and this build reads {adna} through {aqsa}."
            ),
            Self::HawiyaTalifa {
                sigha,
                haql,
                qeema,
                hadd,
            } => format!("A {sigha} container is corrupt: {haql} is {qeema}, outside 0..{hadd}."),
            Self::MiftahMafqud { masar, sabab } => format!(
                "{} is obfuscated and no key could be recovered from the game's own data. \
                 {sabab}",
                masar.display()
            ),
            Self::MiftahGhayrSalih { masar, sabab } => format!(
                "The key recovered for {} does not decode it: {sabab}",
                masar.display()
            ),
            Self::FakkFashil { sigha, tafsil } => {
                format!("A {sigha} section could not be expanded: {tafsil}")
            },
            Self::HajmGhayrMutabaq {
                sigha,
                muallan,
                fili,
            } => format!("A {sigha} section declared {muallan} bytes and produced {fili}."),
            Self::BunyaGhayrMutawaqqaa { malaf, haql } => format!(
                "{malaf}: {haql} is not the shape this engine reads. The file may have been \
                 edited by another tool."
            ),
            Self::NassGhayrSalih {
                malaf,
                tarmiz,
                mawqi,
            } => format!(
                "A string in {malaf} is not valid {tarmiz} at byte {mawqi}. It was refused \
                 rather than replaced with substitution characters that would read as a \
                 translation."
            ),
            Self::DawraGhayrMutabaqa { sigha, adad } => format!(
                "Re-serializing an untouched region of this {sigha} changed {adad} byte(s), \
                 so patching stopped before the game's file was written. This protects the \
                 file rather than risking it."
            ),
            Self::NuskhaMafquda { masar, sabab } => format!(
                "The original of {} could not be preserved, so it was not modified: {sabab}. \
                 Taarib does not modify a file it cannot put back.",
                masar.display()
            ),
            Self::IstiadaGhayrMutabaqa {
                masar,
                muallana,
                mahsuba,
            } => format!(
                "{} was restored and hashes to {mahsuba}, not the recorded {muallana}; the \
                 uninstall did not complete.",
                masar.display()
            ),
            Self::TabaqaMajhula { sabab } => format!(
                "Whether this engine shapes Arabic on its own could not be established, and \
                 Taarib will not guess: guessing one way takes over an engine that was \
                 already correct, and guessing the other leaves letters unjoined. {sabab}"
            ),
            Self::HimlMarfud { alia, sabab } => format!(
                "The {alia} payload could not be installed: {sabab}. Another route is tried."
            ),
            Self::KhattMarfud { sabab } => {
                format!("The patch's font could not be installed: {sabab}")
            },
            Self::JadwalAshkalMarfud { sabab } => {
                format!("The font's glyph table could not be rebuilt: {sabab}")
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::IsdarGhayrMadum { .. } => Khutwa::TahdithTaarib,
            Self::NuskhaMafquda { .. } | Self::IstiadaGhayrMutabaqa { .. } => {
                Khutwa::IblaghLilMalik
            },
            Self::MalafQaseer { .. }
            | Self::SihrGhayrMutabaq { .. }
            | Self::HawiyaTalifa { .. }
            | Self::NassGhayrSalih { .. }
            | Self::FakkFashil { .. }
            | Self::HajmGhayrMutabaq { .. }
            | Self::BunyaGhayrMutawaqqaa { .. }
            | Self::DawraGhayrMutabaqa { .. } => Khutwa::TahaqquqSalamatLuba,
            _ => Khutwa::FathTashkhis,
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
            Self::SihrGhayrMutabaq { masar, sigha } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
            },
            Self::IsdarGhayrMadum {
                sigha,
                wujid,
                adna,
                aqsa,
            } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("adna", QeemaSiyaq::Raqm(i64::from(*adna)));
                daa("aqsa", QeemaSiyaq::Raqm(i64::from(*aqsa)));
            },
            Self::HawiyaTalifa {
                sigha,
                haql,
                qeema,
                hadd,
            } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("hadd", QeemaSiyaq::Hajm(*hadd));
            },
            // Not merged with the arm below despite the identical shape: this
            // `sabab` is a `String` read from the filesystem and that one is a
            // `&'static str`, so one pattern cannot bind both.
            #[expect(
                clippy::match_same_arms,
                reason = "the two `sabab` fields are not one type"
            )]
            Self::MiftahMafqud { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MiftahGhayrSalih { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass((*sabab).to_owned()));
            },
            Self::FakkFashil { sigha, tafsil } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::HajmGhayrMutabaq {
                sigha,
                muallan,
                fili,
            } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("muallan", QeemaSiyaq::Hajm(*muallan));
                daa("fili", QeemaSiyaq::Hajm(*fili));
            },
            Self::BunyaGhayrMutawaqqaa { malaf, haql } => {
                daa("malaf", QeemaSiyaq::Nass(malaf.clone()));
                daa("haql", QeemaSiyaq::Nass(haql.clone()));
            },
            Self::NassGhayrSalih {
                malaf,
                tarmiz,
                mawqi,
            } => {
                daa("malaf", QeemaSiyaq::Nass(malaf.clone()));
                daa("tarmiz", QeemaSiyaq::Nass((*tarmiz).to_owned()));
                daa("mawqi", QeemaSiyaq::Hajm(*mawqi));
            },
            Self::DawraGhayrMutabaqa { sigha, adad } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("adad", QeemaSiyaq::Hajm(*adad));
            },
            Self::NuskhaMafquda { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::IstiadaGhayrMutabaqa {
                masar,
                muallana,
                mahsuba,
            } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            },
            Self::TabaqaMajhula { sabab }
            | Self::KhattMarfud { sabab }
            | Self::JadwalAshkalMarfud { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::HimlMarfud { alia, sabab } => {
                daa("alia", QeemaSiyaq::Nass((*alia).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataNusus);

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
