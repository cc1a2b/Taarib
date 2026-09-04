//! أخطاء الترقيع — what stops a compile, and the one class that is not
//! negotiable.
//!
//! Three classes, and the middle one is the phase's whole posture.
//!
//! **Hard-check failures** stop the compile outright. A broken placeholder
//! anywhere, a string that will not render, missing required metadata, an
//! approval state that could not have been reached legitimately. These are the
//! same checks Phase 18 runs on submission, and running them here means a
//! contributor learns before they submit rather than after a reviewer tells
//! them. They are not overridable — a flag that let a contributor compile past
//! a broken placeholder would be a flag somebody uses at two in the morning to
//! ship a patch that crashes a game's formatter.
//!
//! **Gate failures** stop the compile and are the most serious thing in this
//! crate. They mean something tried to put content into a package that has no
//! proof of origin. In practice they are unreachable — [`crate::bawwaba`] makes
//! the situation unrepresentable — and they exist for the case the type system
//! cannot cover: a font that is not where it was said to be, an atlas that
//! failed to rasterize. See that module for why the guarantee is structural and
//! this is only its edge.
//!
//! **Warnings do not live here at all.** A high overflow count, a large
//! untranslated remainder, a machine-only translation: none of those stop a
//! compile, all of them travel *in* the package so the reviewer sees what the
//! contributor saw. They are data, not errors, and putting them in this enum
//! would have made "compile succeeded" and "compile succeeded with nothing
//! worth mentioning" the same outcome.
//!
//! ## Error codes
//!
//! This crate owns [`arqam::TARQEE`], which is 6100, in its entirety.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Failures of the patch compiler.
#[derive(Debug, thiserror::Error)]
pub enum KhataTarqee {
    // --- the gate ------------------------------------------------------------
    /// A font was offered that does not live in Taarib's font directory.
    ///
    /// Refused by location rather than by name: a list of permitted file names
    /// is something a contributor edits, and the moment it is editable it stops
    /// being a boundary. The path is canonicalized first, so
    /// `khutut/../../game/font.ttf` fails here rather than passing a string
    /// comparison.
    #[error("{masar} is not inside the bundled font directory at {jidhr}")]
    KhattKharijMajmua {
        /// The font that was offered.
        masar: PathBuf,
        /// The directory it had to be inside.
        jidhr: PathBuf,
    },

    /// The atlas could not be rasterized.
    #[error("the glyph atlas could not be built: {sabab}")]
    RasfFashil {
        /// What the atlas pipeline said.
        sabab: String,
    },

    // --- hard checks ---------------------------------------------------------
    /// A string carries a broken placeholder.
    ///
    /// The hard check that exists because of Phase 13's whole protection
    /// module. A patch that shipped a `%d` the translation turned into `%s`
    /// crashes the game's own formatter in front of a player, and no amount of
    /// coverage makes that acceptable.
    #[error("{adad} string(s) carry a broken placeholder and the package was not written")]
    NasqMaksur {
        /// How many.
        adad: usize,
        /// The first few, so the message names something actionable rather than
        /// a count.
        amthila: Vec<String>,
    },

    /// A string could not be laid out.
    #[error("{nass} could not be laid out at {hajm}px: {sabab}")]
    TakhtitFashil {
        /// A short form of the string.
        nass: String,
        /// The size it was being laid out at.
        hajm: f32,
        /// What the layout engine said.
        sabab: String,
    },

    /// A translation reached approved without a review that could have produced
    /// it.
    ///
    /// Unreachable through this product's own types — Phase 13's approval
    /// consumes an attestation with no public constructor and no
    /// `Deserialize` — and checked anyway, because a project file is a document
    /// on disk that a person can edit. The type system governs what this build
    /// can *do*; it cannot govern what a text editor can write.
    #[error("{adad} string(s) claim approval with no review recorded")]
    IaatimadGhayrMashru {
        /// How many.
        adad: usize,
        /// The first few identities.
        amthila: Vec<String>,
    },

    /// Required metadata is missing.
    #[error("the package cannot be written without {haql}")]
    BayanNaqis {
        /// Which field.
        haql: &'static str,
    },

    // --- binding -------------------------------------------------------------
    /// A file the patch wants to fingerprint is not there.
    #[error("{masar} was to be fingerprinted and is not present")]
    MalafIrtibatMafqud {
        /// The file.
        masar: PathBuf,
    },

    /// The project names no game build to bind against.
    #[error("the project records no build identity to bind this package to")]
    BilaBina,

    // --- import --------------------------------------------------------------
    /// An import file could not be read or parsed.
    #[error("{masar} could not be imported: {sabab}")]
    IstiradFashil {
        /// The file.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// An import file is in a format this build does not read.
    #[error("{masar} is not a format this build imports")]
    SighatIstiradMajhula {
        /// The file.
        masar: PathBuf,
        /// What was found.
        wujid: String,
    },

    // --- packaging -----------------------------------------------------------
    /// The container writer refused.
    #[error("the package container could not be written: {sabab}")]
    KitabatHuzmaFashila {
        /// What the writer said.
        sabab: String,
    },

    /// A package claims a format version this build does not read.
    ///
    /// Refused by name, never partially read. A package from a future format
    /// holds sections this build has no meaning for, and reading the ones it
    /// recognises would produce a patch that installs a subset of what its
    /// author built — silently, and looking complete.
    #[error("the package is format {wujid} and this build reads {madum}")]
    SighatHuzmaGhayrMaduma {
        /// The version found.
        wujid: u32,
        /// The version this build reads.
        madum: u32,
    },

    /// A signature block would not seal this package.
    ///
    /// Reached when the block was made for a different container: a signature
    /// commits to a content hash, so one produced for yesterday's build will
    /// not verify against today's. Failing here, on the machine that holds the
    /// key, is the only place that failure is cheap — the alternative is a file
    /// that looks sealed to anything glancing at its header and is refused by
    /// every client that actually checks.
    #[error("the package could not be sealed: {sabab}")]
    KhatmFashil {
        /// What the container said.
        sabab: String,
    },

    /// A compile produced output that did not read back as what was written.
    ///
    /// The compiler reads its own package before returning it. A container that
    /// writes and does not read is one that reaches an installer and fails on
    /// somebody's machine instead of on the machine that built it.
    #[error("the package did not read back as written: {sabab}")]
    DawraGhayrMutabaqa {
        /// What differed.
        sabab: String,
    },

    /// A file could not be read or written.
    #[error("{masar} could not be read or written")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// What the filesystem said.
        sabab: std::io::Error,
    },

    // --- the fingerprint recipe ----------------------------------------------
    /// A path cannot appear in a fingerprint recipe.
    ///
    /// Two situations, both refusals rather than repairs.
    ///
    /// A path that is not valid UTF-8 cannot be written into a recipe without a
    /// lossy conversion, and a lossy conversion maps two different files onto
    /// one name — which would make a fingerprint agree across builds that
    /// differ. Skipping the file instead would be worse: a fingerprint that
    /// silently omits a container matches a build whose text changed.
    ///
    /// A path that escapes the game root — absolute, or carrying `..` — is
    /// refused because the recipe travels inside a downloaded package and the
    /// installer runs it against the user's own disk. A recipe is a list of
    /// files a stranger's file will cause this machine to open, so it may name
    /// only files under the game it claims to patch.
    #[error("{masar} cannot appear in a fingerprint recipe: {sabab}")]
    MasarGhayrSalih {
        /// The offending path.
        masar: PathBuf,
        /// Which of the two rules it broke.
        sabab: SababMasar,
    },
}

/// Why a path was refused a place in a fingerprint recipe.
///
/// An enum rather than a message, because the two cases have different
/// severities — one is a machine whose filenames this format cannot express,
/// the other is a package trying to make an installer open a file outside the
/// game — and deciding that by matching on the text of a string would be a
/// distinction a reworded message silently erases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SababMasar {
    /// The path is not valid UTF-8, so it cannot be written into a recipe
    /// without a conversion that maps two different files onto one name.
    GhayrUtf8,
    /// The path is absolute, or climbs out of the game root with `..`.
    KharijAlJidhr,
}

impl SababMasar {
    /// The clause the message ends with.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::GhayrUtf8 => "it is not valid UTF-8",
            Self::KharijAlJidhr => "it leaves the game directory",
        }
    }
}

impl std::fmt::Display for SababMasar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.wasf())
    }
}

impl KhataTarqee {
    /// Whether this failure is one of the hard checks a package cannot be
    /// produced past.
    ///
    /// Written as a method rather than left to a comparison at the call site, so
    /// that a hard check added later is a hard check everywhere rather than in
    /// whichever list somebody remembered to update.
    #[must_use]
    pub const fn fahs_sarim(&self) -> bool {
        matches!(
            self,
            Self::NasqMaksur { .. }
                | Self::TakhtitFashil { .. }
                | Self::IaatimadGhayrMashru { .. }
                | Self::BayanNaqis { .. }
        )
    }

    /// Whether this failure is the gate refusing content.
    #[must_use]
    pub const fn khalal_bawwaba(&self) -> bool {
        matches!(self, Self::KhattKharijMajmua { .. } | Self::RasfFashil { .. })
    }
}

impl Tafsir for KhataTarqee {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::TARQEE
                + match self {
                    Self::KhattKharijMajmua { .. } => 0,
                    Self::RasfFashil { .. } => 1,
                    Self::NasqMaksur { .. } => 2,
                    Self::TakhtitFashil { .. } => 3,
                    Self::IaatimadGhayrMashru { .. } => 4,
                    Self::BayanNaqis { .. } => 5,
                    Self::MalafIrtibatMafqud { .. } => 6,
                    Self::BilaBina => 7,
                    Self::IstiradFashil { .. } => 8,
                    Self::SighatIstiradMajhula { .. } => 9,
                    Self::KitabatHuzmaFashila { .. } => 10,
                    Self::SighatHuzmaGhayrMaduma { .. } => 11,
                    Self::DawraGhayrMutabaqa { .. } => 12,
                    Self::KhataMalaf { .. } => 13,
                    Self::MasarGhayrSalih { .. } => 14,
                    Self::KhatmFashil { .. } => 15,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Something tried to put unattributed content into a package, or a
            // package claims an approval nobody made. Both are the product's
            // own guarantees failing.
            Self::KhattKharijMajmua { .. } | Self::IaatimadGhayrMashru { .. } => Khutura::Fadih,
            // A recipe that names a path outside the game is a package asking
            // an installer to open somebody else's file. A path this format
            // cannot spell is a machine's filenames, not an attempt at one.
            Self::MasarGhayrSalih { sabab, .. } => match sabab {
                SababMasar::KharijAlJidhr => Khutura::Fadih,
                SababMasar::GhayrUtf8 => Khutura::Khatar,
            },
            // A hard check caught work that is not ready. The contributor
            // learns now instead of after submitting.
            Self::NasqMaksur { .. } | Self::TakhtitFashil { .. } | Self::BayanNaqis { .. } => {
                Khutura::Tanbeeh
            }
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhattKharijMajmua { .. } => {
                "طُلب بناء الحزمة بخطٍّ من خارج خطوط تعريب، ورُفض. لا تحتوي الحزم إلا على ما \
                 يولّده تعريب."
                    .to_owned()
            }
            Self::RasfFashil { .. } => "تعذّر بناء أطلس الأشكال.".to_owned(),
            Self::NasqMaksur { adad, .. } => format!(
                "في {adad} عبارة عناصر محفوظة مكسورة. لم تُبنَ الحزمة: عبارة تفقد عنصرًا \
                 محفوظًا تُعطّل مُنسّق اللعبة أمام اللاعب."
            ),
            Self::TakhtitFashil { .. } => {
                "تعذّر تخطيط إحدى العبارات، ولم تُبنَ الحزمة.".to_owned()
            }
            Self::IaatimadGhayrMashru { adad, .. } => format!(
                "{adad} عبارة تدّعي الاعتماد دون مراجعة مسجّلة. رُفض البناء: الاعتماد شهادة \
                 إنسان قرأ النص."
            ),
            Self::BayanNaqis { haql } => {
                format!("لا يمكن بناء الحزمة بدون ({haql}).")
            }
            Self::MalafIrtibatMafqud { .. } => {
                "أحد الملفات التي تربط الحزمة باللعبة غير موجود.".to_owned()
            }
            Self::BilaBina => {
                "لا يسجّل المشروع إصدار بناءٍ لربط الحزمة به.".to_owned()
            }
            Self::IstiradFashil { .. } => "تعذّر استيراد الملف.".to_owned(),
            Self::SighatIstiradMajhula { .. } => {
                "صيغة الملف ليست مما تستورده هذه النسخة.".to_owned()
            }
            Self::KitabatHuzmaFashila { .. } => "تعذّرت كتابة حاوية الحزمة.".to_owned(),
            Self::SighatHuzmaGhayrMaduma { .. } => {
                "الحزمة بصيغة لا تعرفها هذه النسخة، ولم تُقرأ جزئيًّا.".to_owned()
            }
            Self::DawraGhayrMutabaqa { .. } => {
                "لم تُقرأ الحزمة المكتوبة كما كُتبت، ولم تُسلَّم.".to_owned()
            }
            Self::KhatmFashil { .. } => {
                "لا يختم هذا التوقيعُ هذه الحزمة، ورُفض ختمها به.".to_owned()
            }
            Self::KhataMalaf { .. } => "تعذّرت قراءة ملف أو الكتابة إليه.".to_owned(),
            Self::MasarGhayrSalih { sabab, .. } => match sabab {
                SababMasar::GhayrUtf8 => {
                    "أحد مسارات اللعبة ليس بترميز UTF-8، ولا تُكتب البصمة بمسارٍ مُحوّل.".to_owned()
                }
                SababMasar::KharijAlJidhr => {
                    "تطلب وصفة البصمة ملفًّا خارج مجلّد اللعبة، ورُفضت.".to_owned()
                }
            },
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // The contributor fixes these in the workshop.
            Self::NasqMaksur { .. }
            | Self::TakhtitFashil { .. }
            | Self::IaatimadGhayrMashru { .. }
            | Self::BayanNaqis { .. } => Khutwa::FathNusus,
            Self::KhattKharijMajmua { .. } | Self::RasfFashil { .. } => Khutwa::IadatTarkibIttar,
            Self::MalafIrtibatMafqud { .. }
            | Self::BilaBina
            | Self::MasarGhayrSalih { .. } => Khutwa::TahaqquqSalamatLuba,
            Self::SighatHuzmaGhayrMaduma { .. } => Khutwa::TahdithTaarib,
            Self::IstiradFashil { masar, .. } | Self::SighatIstiradMajhula { masar, .. } => {
                let _ = masar;
                Khutwa::IkhtiyarMasar { matlub: MasarMatlub::MalafRuqaa }
            }
            Self::KitabatHuzmaFashila { .. }
            | Self::DawraGhayrMutabaqa { .. }
            | Self::KhatmFashil { .. } => Khutwa::IblaghLilMusahim,
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        // Handled before the general map exists. `siyaq_io` returns a different
        // map, and inserting the path into the one built below would insert it
        // into something that is then discarded — a bug five earlier crates in
        // this workspace shipped before it was found.
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
            Self::KhattKharijMajmua { masar, jidhr } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("jidhr", QeemaSiyaq::Masar(jidhr.clone()));
            }
            Self::RasfFashil { sabab }
            | Self::KitabatHuzmaFashila { sabab }
            | Self::KhatmFashil { sabab }
            | Self::DawraGhayrMutabaqa { sabab } => daa("sabab", QeemaSiyaq::Nass(sabab.clone())),
            Self::NasqMaksur { adad, amthila }
            | Self::IaatimadGhayrMashru { adad, amthila } => {
                daa("adad", QeemaSiyaq::Hajm(tul_u64(*adad)));
                daa("amthila", QeemaSiyaq::Qaima(amthila.clone()));
            }
            Self::TakhtitFashil { nass, hajm, sabab } => {
                daa("nass", QeemaSiyaq::Nass(nass.clone()));
                daa("hajm", QeemaSiyaq::Kasr(f64::from(*hajm)));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::BayanNaqis { haql } => daa("haql", QeemaSiyaq::Nass((*haql).to_owned())),
            Self::MalafIrtibatMafqud { masar } => daa("masar", QeemaSiyaq::Masar(masar.clone())),
            Self::MasarGhayrSalih { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.wasf().to_owned()));
            }
            Self::IstiradFashil { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::SighatIstiradMajhula { masar, wujid } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("wujid", QeemaSiyaq::Nass(wujid.clone()));
            }
            Self::SighatHuzmaGhayrMaduma { wujid, madum } => {
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("madum", QeemaSiyaq::Raqm(i64::from(*madum)));
            }
            // `BilaBina` carries no fields; `KhataMalaf` returned its own map above.
            Self::BilaBina | Self::KhataMalaf { .. } => {}
        }
        siyaq
    }
}

khata_min!(KhataTarqee);

/// A length as a `u64`, without a cast that can wrap.
#[must_use]
pub fn tul_u64(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}
