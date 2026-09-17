//! Failures of an automatic run, and what a user can do about each one.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, siyaq_io};
use taarib_usus::khata_min;

use crate::taqaddum::MarhalaTilqai;

/// The result type of every fallible step of an automatic run.
pub type NatijatTilqai<T> = Result<T, KhataTilqai>;

/// The error-code block this crate allocates from.
///
/// Re-exported from `taarib_usus::khata::arqam`, which is the one place every
/// block is named, so that two crates can never quietly claim one number. It
/// was briefly declared here while this crate was being written and its owner
/// could not edit `taarib-usus`; that copy is gone.
pub use taarib_usus::khata::arqam::TILQAI;

/// Failures of the automatic pipeline.
///
/// Refusals that are *outcomes* are not here: a game with nothing extractable,
/// a translation the guard rejected, a string that would not fit — those are
/// reported by [`crate::taqreer::TaqreerMashwar`] and the run keeps going or
/// stops cleanly. What is here is the run failing to proceed.
#[derive(Debug, thiserror::Error)]
pub enum KhataTilqai {
    /// The game directory is not a directory, or is not readable.
    #[error("{masar} is not a readable game directory")]
    JidhrGhayrSalih {
        /// The path that was handed in.
        masar: PathBuf,
    },

    /// The engine probe could not run at all.
    #[error("the engine probe failed: {sabab}")]
    FahsFashil {
        /// Why.
        sabab: String,
    },

    /// The resolved tier is one this pipeline cannot patch without help the
    /// caller has not supplied.
    #[error("this game resolved to tier {tabaqa}, which this pipeline cannot patch: {sabab}")]
    TabaqaGhayrMadauma {
        /// The tier number.
        tabaqa: u8,
        /// Why, in the capability report's own words.
        sabab: String,
    },

    /// The engine was identified and nothing this build installs for it
    /// reaches the screen.
    ///
    /// Deliberately not folded into [`Self::TabaqaGhayrMadauma`], and the two
    /// must stay apart. That one is a fact about the *game* — an anti-cheat
    /// association, a launcher that forbids modification — and no release of
    /// Taarib changes it. This one is a fact about the *build*: the adapter
    /// that would put Arabic inside this engine is unfinished, and a release
    /// does change it. A user can act on the difference, which is the whole
    /// reason `taarib_mustalahat::muharrik::JahiziyatTashghil` exists as a
    /// field of its own beside the tier.
    #[error("{sabab_injilizi}")]
    MuharrikGhayrJahiz {
        /// The detected engine, as the interface names it. Carried for the
        /// log line and the diagnostics bundle; the sentences already say it.
        muharrik: String,
        /// The whole refusal in Arabic: the engine, and what is missing.
        sabab_arabi: String,
        /// The same sentence in English.
        sabab_injilizi: String,
    },

    /// Static extraction produced nothing and runtime capture is the remedy.
    ///
    /// The one refusal that is an error rather than an outcome, because there
    /// is nothing further this run can do and the next action is the user's.
    #[error(
        "no string could be read out of this game's files. Run the game once with capture \
         enabled, then come back: {sabab}"
    )]
    YahtajIltiqat {
        /// What the refusal report said, condensed.
        sabab: String,
    },

    /// Static extraction produced nothing and capture would not help either.
    #[error(
        "no string could be read out of this game's files, and capture would not help: {sabab}"
    )]
    LaNusus {
        /// What the refusal report said, condensed.
        sabab: String,
    },

    /// A capture session file was named and could not be read.
    #[error("the capture session {masar} could not be read: {sabab}")]
    JalsaGhayrMaqrua {
        /// The session file.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// No font was supplied, or none of those supplied is usable.
    #[error("no usable Arabic font: {sabab}")]
    LaKhatt {
        /// Why.
        sabab: String,
    },

    /// The translation stage produced no translated string at all.
    #[error("the translation run produced nothing to compile: {sabab}")]
    LaTarjama {
        /// Why: the provider's stopping failure, or the plain count.
        sabab: String,
    },

    /// A stage delegated to another crate and that crate refused.
    ///
    /// Carries the stage so the interface can say which half of the run
    /// stopped without parsing the message.
    #[error("{marhala} refused: {sabab}")]
    MarhalaMarfuda {
        /// Which stage.
        marhala: MarhalaTilqai,
        /// The refusal, in the refusing crate's own words.
        sabab: String,
    },

    /// The safety gate refused to permit the install.
    #[error("the install was refused by the safety gate: {sabab}")]
    TathbeetMarfud {
        /// The gate's own refusal.
        sabab: String,
    },

    /// A file in the run directory could not be read or written.
    #[error("{masar} could not be {amal}: {sabab}")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// What was being attempted, for the message.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// The run's string table is larger than this build will read.
    #[error(
        "the run's string table at {} is {hajm} bytes, over the {hadd} this build reads",
        masar.display()
    )]
    NususKabira {
        /// The table.
        masar: PathBuf,
        /// What the directory entry said.
        hajm: u64,
        /// The cap it passed.
        hadd: u64,
    },

    /// The run journal holds a record this build does not understand.
    #[error(
        "the run journal at {masar} is from a newer build (format {wujid}, this build {maqru})"
    )]
    SijillAhdath {
        /// The journal.
        masar: PathBuf,
        /// The version found.
        wujid: u32,
        /// The version this build writes.
        maqru: u32,
    },

    /// The run was cancelled.
    ///
    /// An error rather than a status so that every `?` in the pipeline is a
    /// cancellation point; [`crate::tanfidh::arrib`] turns it back into
    /// [`crate::taqreer::HalatMashwar::Mulgha`] before it returns.
    #[error("the run was cancelled during {marhala}")]
    Mulgha {
        /// Where it was cancelled.
        marhala: MarhalaTilqai,
    },
}

impl Tafsir for KhataTilqai {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            TILQAI
                + match self {
                    Self::JidhrGhayrSalih { .. } => 0,
                    Self::FahsFashil { .. } => 1,
                    Self::TabaqaGhayrMadauma { .. } => 2,
                    Self::YahtajIltiqat { .. } => 3,
                    Self::LaNusus { .. } => 4,
                    Self::JalsaGhayrMaqrua { .. } => 5,
                    Self::LaKhatt { .. } => 6,
                    Self::LaTarjama { .. } => 7,
                    Self::MarhalaMarfuda { .. } => 8,
                    Self::TathbeetMarfud { .. } => 9,
                    Self::KhataMalaf { .. } => 10,
                    Self::SijillAhdath { .. } => 11,
                    Self::Mulgha { .. } => 12,
                    // Allocated after `Mulgha` rather than beside the refusal
                    // it sits next to in the enum: a code is permanent, and
                    // renumbering the twelve above it to make the enum read
                    // tidily would silently retarget every log line and bug
                    // report that already names one of them.
                    Self::MuharrikGhayrJahiz { .. } => 13,
                    Self::NususKabira { .. } => 14,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Neither is a fault: one is the product telling the user the
            // remedy it has, the other is the user's own stop.
            Self::YahtajIltiqat { .. } | Self::Mulgha { .. } => Khutura::Maluma,
            // A gate refusing is the gate working. The third is the product
            // declining to hand somebody a patch that would change nothing on
            // their screen, which is the correct outcome and not a fault.
            Self::TathbeetMarfud { .. }
            | Self::TabaqaGhayrMadauma { .. }
            | Self::MuharrikGhayrJahiz { .. } => Khutura::Tanbeeh,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::JidhrGhayrSalih { .. } => "المسار المُعطى ليس مجلّد لعبة يمكن قراءته.".to_owned(),
            Self::FahsFashil { .. } => "تعذّر فحص محرّك اللعبة.".to_owned(),
            Self::TabaqaGhayrMadauma { tabaqa, .. } => {
                format!("هذه اللعبة في الطبقة {tabaqa}، ولا يستطيع التعريب التلقائي ترقيعها.")
            },
            // Verbatim. The sentence was written once, by `fahs::naqs_jahiziya`,
            // out of the capability report's own words, so that the verdict
            // screen and this refusal cannot drift into saying two things.
            Self::MuharrikGhayrJahiz { sabab_arabi, .. } => sabab_arabi.clone(),
            Self::YahtajIltiqat { .. } => {
                "لم يُقرأ أيّ نصّ من ملفات اللعبة. شغّل اللعبة مرّة واحدة مع تفعيل الالتقاط، \
                 ثمّ عُد."
                    .to_owned()
            },
            Self::LaNusus { .. } => {
                "لم يُقرأ أيّ نصّ من ملفات اللعبة، والالتقاط لن يفيد هنا.".to_owned()
            },
            Self::JalsaGhayrMaqrua { .. } => "تعذّرت قراءة ملف جلسة الالتقاط.".to_owned(),
            Self::LaKhatt { .. } => "لا يوجد خطّ عربيّ صالح لبناء الرقعة.".to_owned(),
            Self::LaTarjama { .. } => "لم تُنتج جولة الترجمة نصًّا واحدًا قابلًا للتجميع.".to_owned(),
            Self::MarhalaMarfuda { marhala, .. } => {
                format!("رُفضت المرحلة: {}.", marhala.wasf_arabi())
            },
            Self::TathbeetMarfud { .. } => "رفضت بوّابة الأمان تثبيت هذه الرقعة.".to_owned(),
            Self::KhataMalaf { .. } => "تعذّرت قراءة ملف في مجلّد الجولة أو الكتابة إليه.".to_owned(),
            Self::NususKabira { .. } => {
                "جدول نصوص هذه الجولة أكبر ممّا يقرأه هذا الإصدار، فلم يُقرأ منه شيء.".to_owned()
            },
            Self::SijillAhdath { .. } => {
                "سجلّ الجولة مكتوب بنسخة أحدث من تعريب؛ حدِّث البرنامج.".to_owned()
            },
            Self::Mulgha { marhala } => {
                format!(
                    "أُلغيت الجولة أثناء: {}. لم تُمَسّ اللعبة.",
                    marhala.wasf_arabi()
                )
            },
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::JidhrGhayrSalih { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba,
            },
            Self::FahsFashil { .. } => Khutwa::AadaFahsMuharrik,
            // Nothing on this machine opens any of the three. Two are permanent
            // and the third is closed by a Taarib release, which the sentence
            // says — offering "check for updates" beside it would turn "wait"
            // into "click here", and there is nothing behind the click.
            Self::TabaqaGhayrMadauma { .. }
            | Self::LaNusus { .. }
            | Self::MuharrikGhayrJahiz { .. } => Khutwa::LaShay,
            // There is a screen for this and it is the whole point of the
            // variant: capture is offered, not merely described.
            Self::YahtajIltiqat { .. } => Khutwa::FathNusus,
            Self::JalsaGhayrMaqrua { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladManassa,
            },
            Self::LaKhatt { .. } => Khutwa::IkhtiyarKhattAakhar,
            Self::LaTarjama { .. } | Self::MarhalaMarfuda { .. } | Self::NususKabira { .. } => {
                Khutwa::FathTashkhis
            },
            Self::TathbeetMarfud { .. } => Khutwa::TahaqquqSalamatLuba,
            Self::KhataMalaf { sabab, .. } => {
                taarib_usus::khata::khutwa_io(sabab, MasarMatlub::MujalladManassa)
            },
            Self::SijillAhdath { .. } => Khutwa::TahdithTaarib,
            Self::Mulgha { .. } => Khutwa::AadaMuhawala,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        if let Self::KhataMalaf { masar, amal, sabab } = self {
            let mut siyaq = siyaq_io(sabab);
            let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            let _ = siyaq.insert("amal".to_owned(), QeemaSiyaq::Nass((*amal).to_owned()));
            return siyaq;
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::KhataMalaf { .. } => {},
            Self::JidhrGhayrSalih { masar } => daa("masar", QeemaSiyaq::Masar(masar.clone())),
            Self::FahsFashil { sabab }
            | Self::YahtajIltiqat { sabab }
            | Self::LaNusus { sabab }
            | Self::LaKhatt { sabab }
            | Self::LaTarjama { sabab }
            | Self::TathbeetMarfud { sabab } => daa("sabab", QeemaSiyaq::Nass(sabab.clone())),
            Self::TabaqaGhayrMadauma { tabaqa, sabab } => {
                daa("tabaqa", QeemaSiyaq::Raqm(i64::from(*tabaqa)));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MuharrikGhayrJahiz {
                muharrik,
                sabab_injilizi,
                ..
            } => {
                daa("muharrik", QeemaSiyaq::Nass(muharrik.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab_injilizi.clone()));
            },
            Self::JalsaGhayrMaqrua { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::NususKabira { masar, hajm, hadd } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("hajm", QeemaSiyaq::Hajm(*hajm));
                daa("hadd", QeemaSiyaq::Hajm(*hadd));
            },
            Self::MarhalaMarfuda { marhala, sabab } => {
                daa("marhala", QeemaSiyaq::Nass(marhala.ramz().to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::SijillAhdath {
                masar,
                wujid,
                maqru,
            } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("maqru", QeemaSiyaq::Raqm(i64::from(*maqru)));
            },
            Self::Mulgha { marhala } => {
                daa("marhala", QeemaSiyaq::Nass(marhala.ramz().to_owned()));
            },
        }
        siyaq
    }
}

/// One I/O failure, with the action that was being attempted.
pub(crate) fn khata_malaf(
    masar: &std::path::Path,
    amal: &'static str,
    sabab: std::io::Error,
) -> KhataTilqai {
    KhataTilqai::KhataMalaf {
        masar: masar.to_path_buf(),
        amal,
        sabab,
    }
}

/// One stage's refusal, in the refusing crate's own words.
pub(crate) fn marfuda(marhala: MarhalaTilqai, sabab: impl std::fmt::Display) -> KhataTilqai {
    KhataTilqai::MarhalaMarfuda {
        marhala,
        sabab: sabab.to_string(),
    }
}

khata_min!(KhataTilqai);
