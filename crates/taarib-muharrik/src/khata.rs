//! أخطاء المحرّك — the very few things that stop a probe.
//!
//! Almost nothing here is fatal, for the same reason almost nothing in discovery
//! is: a probe that failed because one header was unreadable would leave a user
//! with a game card that says nothing, and "unknown engine" is a real answer
//! that the product handles gracefully — tier 3 exists precisely for it.
//!
//! So a corrupt asset container, a version string in a format this build does
//! not recognise, a directory that cannot be listed: none of those are errors.
//! They are the absence of evidence, and the probe reports what it did see. The
//! variants below are for the narrow set of conditions where there is nothing to
//! probe at all, or where continuing would mean reporting a conclusion drawn
//! from something other than the game.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Failures of engine identification.
#[derive(Debug, thiserror::Error)]
pub enum KhataMuharrik {
    /// The game's directory is not there.
    ///
    /// Ordinary rather than alarming: a launcher's catalogue outlives the files
    /// it describes, so this is what probing a game the user deleted by hand
    /// looks like.
    #[error("game directory does not exist: {jidhr}")]
    JidhrMafqud {
        /// Where the game was supposed to be.
        jidhr: PathBuf,
    },

    /// The game's directory exists and cannot be read.
    #[error("cannot read game directory {jidhr}")]
    TaadhurQiraatJidhr {
        /// The directory.
        jidhr: PathBuf,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// The directory holds no executable and no engine marker of any kind.
    ///
    /// Distinguished from "unknown engine" deliberately. An unknown engine is a
    /// game whose files are there and whose engine this build does not
    /// recognise, and it gets a tier 3 report. This is a directory with nothing
    /// in it that resembles a game, which usually means the path points at a
    /// launcher's own folder, a save directory, or a partially deleted install.
    #[error("nothing under {jidhr} resembles a game")]
    LaYushbihLuba {
        /// The directory.
        jidhr: PathBuf,
        /// How many entries were looked at before giving up.
        adad: u32,
    },

    /// A detector walked past its entry ceiling without finishing.
    ///
    /// Not a failure of the game: a game with more files than the probe will
    /// look at. Recorded so that a capability report built from a truncated walk
    /// is never presented as though it were complete.
    #[error("stopped after {adad} entries under {jidhr}")]
    TajawuzHadd {
        /// The directory.
        jidhr: PathBuf,
        /// The ceiling that was hit.
        adad: u32,
    },

    /// The probe was asked about a path that is not inside the game.
    ///
    /// A symbolic link inside a game directory pointing somewhere else, or a
    /// relative path from a manifest with `..` in it. Refused rather than
    /// followed: a probe that read outside the game would report an engine the
    /// game does not use.
    #[error("{masar} is outside the game directory {jidhr}")]
    MasarKharij {
        /// The offending path.
        masar: PathBuf,
        /// The game's root.
        jidhr: PathBuf,
    },
}

impl Tafsir for KhataMuharrik {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUHARRIK
                + match self {
                    Self::JidhrMafqud { .. } => 0,
                    Self::TaadhurQiraatJidhr { .. } => 1,
                    Self::LaYushbihLuba { .. } => 2,
                    Self::TajawuzHadd { .. } => 3,
                    Self::MasarKharij { .. } => 4,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::TajawuzHadd { .. } => Khutura::Tanbeeh,
            Self::MasarKharij { .. } => Khutura::Fadih,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::JidhrMafqud { jidhr } => format!(
                "مجلد اللعبة غير موجود: {}. قد تكون اللعبة حُذفت دون أن يعرف المتجر، أو أن \
                 القرص غير متصل.",
                jidhr.display()
            ),
            Self::TaadhurQiraatJidhr { jidhr, .. } => format!(
                "تعذّرت قراءة مجلد اللعبة: {}. تحقّق من صلاحيات المجلد.",
                jidhr.display()
            ),
            Self::LaYushbihLuba { jidhr, .. } => format!(
                "لا يوجد داخل {} ما يشبه لعبة. تأكد من أن المسار يشير إلى مجلد اللعبة نفسه لا \
                 إلى مجلد المتجر أو مجلد الحفظ.",
                jidhr.display()
            ),
            Self::TajawuzHadd { adad, .. } => format!(
                "توقّف الفحص بعد {adad} ملف. تقرير القدرات مبني على ما فُحص فقط، وقد يكون \
                 ناقصًا."
            ),
            Self::MasarKharij { .. } => {
                "رُفض مسار يشير خارج مجلد اللعبة. لن يُفحص، لأن فحصه قد يصف محركًا لا تستخدمه \
                 اللعبة."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::JidhrMafqud { jidhr } => format!(
                "The game folder does not exist: {}. The game may have been deleted without the \
                 launcher noticing, or its drive may be disconnected.",
                jidhr.display()
            ),
            Self::TaadhurQiraatJidhr { jidhr, .. } => {
                format!(
                    "Cannot read the game folder: {}. Check its permissions.",
                    jidhr.display()
                )
            },
            Self::LaYushbihLuba { jidhr, .. } => format!(
                "Nothing under {} resembles a game. Check that the path points at the game's own \
                 folder rather than the launcher's or a save folder.",
                jidhr.display()
            ),
            Self::TajawuzHadd { adad, .. } => format!(
                "The probe stopped after {adad} files. The capability report covers only what was \
                 examined and may be incomplete."
            ),
            Self::MasarKharij { .. } => {
                "Refused a path pointing outside the game folder. It will not be examined, because \
                 doing so could describe an engine the game does not use."
                    .to_owned()
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::JidhrMafqud { .. } => Khutwa::AadaFahsMaktaba,
            Self::TaadhurQiraatJidhr { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::LaYushbihLuba { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba,
            },
            Self::TajawuzHadd { .. } | Self::MasarKharij { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::JidhrMafqud { jidhr } => {
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            },
            Self::TaadhurQiraatJidhr { jidhr, sabab } => {
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
                siyaq.extend(siyaq_io(sabab));
            },
            Self::LaYushbihLuba { jidhr, adad } | Self::TajawuzHadd { jidhr, adad } => {
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Raqm(i64::from(*adad)));
            },
            Self::MasarKharij { masar, jidhr } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataMuharrik);
