//! الرفض — what was skipped, why, and what the user can do about it.
//!
//! Every extraction produces one of these, and the interface shows it **before
//! the project opens**. That ordering is the whole point of the module.
//!
//! ## Why this exists at all
//!
//! Extraction is never complete. An IL2CPP release build strips the type layout
//! and its serialized fields cannot be read; an encrypted asset archive has no
//! recoverable key; a container is a format this build does not know. Those are
//! ordinary outcomes, not exceptional ones, and a tool that meets them by
//! quietly extracting what it can and presenting the result as *the strings*
//! has misled its user in the way that costs them the most: they translate
//! seventy percent, ship, and discover the rest in play.
//!
//! So refusal is a first-class output. [`TaqreerRafd`] is returned beside the
//! table, never instead of it, and it names each skipped container, the reason
//! in words a contributor can act on, and the remedy — which is very often
//! "run the game once with capture on".
//!
//! ## Refusing is not failing
//!
//! Nothing in this module is an error in the [`Result`] sense. An extraction
//! that read four containers and refused two succeeded: it produced a table and
//! a report, and the report is what makes the table trustworthy. The error type
//! in [`crate::khata`] is for extractions that could not run at all.
//!
//! ## The rule this module enforces
//!
//! **A container that cannot be read correctly yields nothing.** Not a partial
//! table, not a best guess, not a byte-pattern scan presented as extraction. A
//! plausible-but-wrong string table produces a patch that corrupts a game, and
//! the corruption is discovered by a player rather than by the contributor.
//! [`SababRafd`] has no variant meaning "read partially" for exactly that
//! reason, and [`TaqreerRafd::sajjil`] takes a reason rather than a count.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Why a container was skipped.
///
/// Ordered by what the user can do about it: the ones with a real remedy first,
/// then the ones that need a decision, then the ones that are simply outside
/// what this build knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababRafd {
    /// A Unity object's serialized layout is not in the build.
    ///
    /// **The single most common refusal in this product**, and the one the
    /// runtime-capture path exists for. An IL2CPP release build strips the type
    /// tree, and without it a field's bytes cannot be interpreted: a length
    /// prefix followed by a string is byte-identical to two integers, and there
    /// is no evidence in the file that distinguishes them.
    ///
    /// Guessing here is not a degraded read, it is a fabricated one. A wrong
    /// field boundary produces text that looks like text and is not, and it
    /// reaches a player as corruption.
    BilaShajaratAnwa {
        /// The object's type, when the file names one.
        ///
        /// Not `naw`: that is this enum's own discriminant tag, and serde
        /// refuses a variant field that shadows it.
        naw_kaen: Option<String>,
        /// How many objects of it were skipped.
        adad: usize,
    },
    /// The container is encrypted and the key was not recoverable.
    ///
    /// Refused rather than attempted. A wrong key does not fail loudly — it
    /// produces bytes, and those bytes parse as *something*.
    Mushaffar {
        /// What the encryption looks like, where that is identifiable.
        wasf: String,
    },
    /// The format is one this build does not read.
    SighaMajhula {
        /// What was found, as specifically as it could be named.
        wujid: String,
    },
    /// A format this build reads, at a version it does not.
    IsdarGhayrMadum {
        /// The format.
        sigha: String,
        /// The version found.
        wujid: String,
        /// The versions this build handles.
        madum: String,
    },
    /// The container is damaged: a header that does not check out, a table that
    /// runs past the end of the file.
    Talif {
        /// What was wrong.
        sabab: String,
    },
    /// The file could not be opened or read.
    TaadhurQira {
        /// What the filesystem said.
        sabab: String,
    },
    /// The container was read and holds no text.
    ///
    /// Not a problem, and recorded anyway. A user asking "why did my textures
    /// archive contribute nothing" deserves the answer "because it is textures"
    /// rather than silence.
    BilaNusus,
    /// This build will not read the member, and nothing the user does changes
    /// that.
    ///
    /// The distinction from every other arm is who has to act. A stripped type
    /// tree is answered by capture, a damaged file by the launcher's verify, a
    /// missing key by nobody — but each of those is a statement about the
    /// game. This one is a statement about Taarib: the container is intact, the
    /// text is there, and the reader declines. It exists because the
    /// alternatives were being borrowed and each borrowing told the user
    /// something false — that an intact game was damaged, or that capturing a
    /// metadata file would help.
    HadUlBina {
        /// Which member, as specifically as it can be named.
        wujid: String,
        /// Why this build declines it.
        sabab: String,
    },
    /// Reading it would have exceeded a ceiling this build enforces.
    ///
    /// A container declaring a hundred million strings is a damaged container
    /// or a hostile one, and either way the answer is to refuse before
    /// allocating rather than after.
    ///
    /// It is not only those two, though, and the wording everywhere must allow
    /// for the third: a real game refused here on a container that was entirely
    /// intact, because the ceiling was simply lower than what that game builds.
    /// So this says a limit was reached, never that the game is broken.
    TajawuzHadd {
        /// Which ceiling.
        // Owned, not `&'static str`: a borrowed field would make this whole
        // enum — and every report that carries it — deserializable only
        // for `'de: 'static`, which means not deserializable at all.
        hadd: String,
        /// What the container declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },
}

impl SababRafd {
    /// Whether running the game with capture on would recover these strings.
    ///
    /// True for the two refusals that are about *reading the file* rather than
    /// about the file being absent or broken. A stripped type tree and an
    /// unreadable format both describe strings that exist and are drawn on
    /// screen; capture sees them there. A corrupt archive holds nothing to see.
    ///
    /// This drives the remedy the interface offers, which is why it is a method
    /// rather than a comparison at the call site: a new refusal added without
    /// updating a scattered comparison would offer the wrong remedy silently.
    #[must_use]
    pub const fn yuslihuhu_iltiqat(&self) -> bool {
        matches!(
            self,
            // Encryption belongs here for the same reason the other two do, and
            // its absence was backwards in the way that matters most: a game
            // that encrypts its own text decrypts it before drawing it, and
            // drawing is exactly what capture observes. Refusing the remedy
            // here told the owner of such a game there was nothing further to
            // try, when the one thing that works was a click away.
            Self::BilaShajaratAnwa { .. } | Self::SighaMajhula { .. } | Self::Mushaffar { .. }
        )
    }

    /// Whether this refusal means strings were genuinely lost.
    ///
    /// False for [`SababRafd::BilaNusus`], which is a container that had nothing
    /// to give. Counting it as a loss would make every extraction report look
    /// worse than it was and train users to ignore the report.
    #[must_use]
    pub const fn khasara(&self) -> bool {
        !matches!(self, Self::BilaNusus)
    }

    /// A stable short name for the kind of refusal, for grouping and logs.
    #[must_use]
    pub const fn miftah(&self) -> &'static str {
        match self {
            Self::BilaShajaratAnwa { .. } => "bila_shajarat_anwa",
            Self::Mushaffar { .. } => "mushaffar",
            Self::SighaMajhula { .. } => "sigha_majhula",
            Self::IsdarGhayrMadum { .. } => "isdar_ghayr_madum",
            Self::Talif { .. } => "talif",
            Self::TaadhurQira { .. } => "taadhur_qira",
            Self::BilaNusus => "bila_nusus",
            Self::TajawuzHadd { .. } => "tajawuz_hadd",
            Self::HadUlBina { .. } => "had_ul_bina",
        }
    }

    /// The same refusal, widened to say it applied to `adad` members of one
    /// container.
    ///
    /// The count goes into the field the sentence already prints rather than
    /// into a new one, so the shape every stored and displayed report has stays
    /// what it was. Unchanged below two, and unchanged for the two reasons a
    /// member count would not describe: a container read and empty, and a
    /// Unity object count, which [`MujammiRafd`] sums instead.
    #[must_use]
    pub fn bi_adad(self, adad: usize) -> Self {
        if adad < 2 {
            return self;
        }
        match self {
            Self::Mushaffar { wasf } => Self::Mushaffar {
                wasf: format!("{wasf}; {adad} members of this container are locked the same way"),
            },
            Self::SighaMajhula { wujid } => Self::SighaMajhula {
                wujid: format!("{adad} members of this container, each {wujid}"),
            },
            Self::IsdarGhayrMadum {
                sigha,
                wujid,
                madum,
            } => Self::IsdarGhayrMadum {
                sigha,
                wujid: format!("{wujid} across {adad} members of this container"),
                madum,
            },
            Self::Talif { sabab } => Self::Talif {
                sabab: format!("{sabab}; {adad} members of this container refused the same way"),
            },
            Self::TaadhurQira { sabab } => Self::TaadhurQira {
                sabab: format!("{sabab}; {adad} members of this container refused the same way"),
            },
            Self::HadUlBina { wujid, sabab } => Self::HadUlBina {
                wujid: format!("{adad} members of this container, each {wujid}"),
                sabab,
            },
            Self::TajawuzHadd { hadd, qeema, saqf } => Self::TajawuzHadd {
                hadd: format!("{hadd}, in {adad} members of this container,"),
                qeema,
                saqf,
            },
            Self::BilaShajaratAnwa { .. } | Self::BilaNusus => self,
        }
    }

    /// The reason, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        match self {
            Self::BilaShajaratAnwa { adad, .. } => format!(
                "هذه اللعبة مبنيّة بطريقة تحذف وصف بنية بياناتها، فلا يمكن قراءة نصوص {adad} \
                 كائنًا منها من الملفات. النصوص موجودة وتُعرض في اللعبة، ويلتقطها تعريب أثناء \
                 اللعب."
            ),
            Self::Mushaffar { .. } => {
                "هذه الحاوية مشفَّرة ولم يُعثر على مفتاحها. لم يُستخرج منها شيء، ولم تُخمَّن \
                 محتوياتها."
                    .to_owned()
            },
            Self::SighaMajhula { wujid } => {
                format!("الصيغة ({wujid}) ليست مما تقرؤه هذه النسخة.")
            },
            Self::IsdarGhayrMadum { sigha, wujid, .. } => {
                format!("صيغة {sigha} بإصدار ({wujid}) لا تعرفه هذه النسخة.")
            },
            Self::Talif { .. } => "الحاوية تالفة أو ناقصة، ولم يُستخرج منها شيء.".to_owned(),
            Self::TaadhurQira { .. } => "تعذّرت قراءة الملف.".to_owned(),
            Self::BilaNusus => "قُرئت الحاوية ولا تحتوي نصوصًا.".to_owned(),
            Self::HadUlBina { wujid, sabab } => {
                format!("{wujid}: لا تقرأه هذه النسخة من تعريب ({sabab}).")
            },
            Self::TajawuzHadd { .. } => {
                "تعلن الحاوية حجمًا أكبر مما تسمح به هذه النسخة، ورُفضت قبل حجز أي ذاكرة.".to_owned()
            },
        }
    }

    /// The reason, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        match self {
            Self::BilaShajaratAnwa { naw_kaen, adad } => {
                let ism = naw_kaen.as_deref().unwrap_or("its objects");
                format!(
                    "This game is built in a way that strips the description of its own data \
                     layout, so the text inside {adad} object(s) of {ism} cannot be read from \
                     the files. Without that layout a length prefix followed by a string is \
                     byte-identical to two integers, and guessing would produce text that is \
                     not text. The strings exist and are drawn on screen; capture reads them \
                     there."
                )
            },
            Self::Mushaffar { wasf } => format!(
                "This container is encrypted ({wasf}) and no key was recoverable. Nothing was \
                 extracted and nothing was guessed."
            ),
            Self::SighaMajhula { wujid } => {
                format!("{wujid} is not a format this build reads.")
            },
            Self::IsdarGhayrMadum {
                sigha,
                wujid,
                madum,
            } => {
                format!("{sigha} version {wujid} is not one this build handles; it reads {madum}.")
            },
            Self::Talif { sabab } => format!("The container is damaged: {sabab}"),
            Self::TaadhurQira { sabab } => format!("The file could not be read: {sabab}"),
            Self::BilaNusus => "Read, and holds no text.".to_owned(),
            Self::HadUlBina { wujid, sabab } => {
                format!("{wujid}: this build of Taarib does not read it ({sabab}).")
            },
            Self::TajawuzHadd { hadd, qeema, saqf } => format!(
                "{hadd} declares {qeema}, above this build's ceiling of {saqf}. Refused before \
                 allocating."
            ),
        }
    }

    /// What the user can do, in Arabic.
    #[must_use]
    pub const fn ilaj_arabi(&self) -> &'static str {
        match self {
            Self::BilaShajaratAnwa { .. } | Self::SighaMajhula { .. } => {
                "شغّل اللعبة مرة واحدة مع تفعيل الالتقاط، ثم عُد."
            },
            Self::Mushaffar { .. } => "لا يمكن لتعريب فتح هذه الحاوية.",
            Self::IsdarGhayrMadum { .. } => "حدِّث تعريب؛ قد تدعم نسخة أحدث هذا الإصدار.",
            Self::Talif { .. } => "اطلب من المتجر التحقّق من ملفات اللعبة.",
            Self::TaadhurQira { .. } => "تأكّد من صلاحيات الملف ومن أن اللعبة ليست قيد التشغيل.",
            Self::BilaNusus => "لا شيء مطلوب.",
            Self::HadUlBina { .. } => "حدِّث تعريب؛ قد تقرأه نسخة أحدث. اللعبة سليمة.",
            Self::TajawuzHadd { .. } => "أبلغ عن هذه اللعبة؛ الحدّ هنا حدُّ تعريب لا عيبٌ في اللعبة.",
        }
    }

    /// What the user can do, in English.
    #[must_use]
    pub const fn ilaj_injilizi(&self) -> &'static str {
        match self {
            Self::BilaShajaratAnwa { .. } | Self::SighaMajhula { .. } => {
                "Play the game once with capture enabled, then come back."
            },
            Self::Mushaffar { .. } => "Taarib cannot open this container.",
            Self::IsdarGhayrMadum { .. } => "Update Taarib; a newer build may read this version.",
            Self::Talif { .. } => "Ask the launcher to verify the game's files.",
            Self::TaadhurQira { .. } => {
                "Check the file's permissions, and that the game is not running."
            },
            Self::BilaNusus => "Nothing to do.",
            Self::HadUlBina { .. } => "Update Taarib; a newer build may read it. The game is fine.",
            // Not "the container may be damaged". This ceiling is Taarib's own,
            // and a real game tripped it on a container that is perfectly
            // intact — so the report is about raising a limit here, not about
            // anything wrong on the user's machine, and the sentence must not
            // send them to verify files that are fine.
            Self::TajawuzHadd { .. } => {
                "Report this game; the limit is Taarib's, not a fault in the game."
            },
        }
    }
}

/// One skipped container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MadkhalRafd {
    /// The container, relative to the game's root.
    pub hawiya: String,
    /// The asset inside it, when the refusal was narrower than the container.
    pub asl: Option<String>,
    /// Why.
    pub sabab: SababRafd,
}

/// One container that was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MadkhalQira {
    /// The container.
    pub hawiya: String,
    /// How many strings came out of it.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub adad: usize,
    /// What kind of container it was, in the engine's own words.
    pub wasf: String,
}

/// What an extraction read and what it refused.
///
/// Returned beside the table, never instead of it. An extraction that refused
/// every container still returns a report, and that report is the useful output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TaqreerRafd {
    /// Every container that was read.
    pub maqrua: Vec<MadkhalQira>,
    /// Every container that was skipped.
    pub marfuda: Vec<MadkhalRafd>,
}

impl TaqreerRafd {
    /// An empty report.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            maqrua: Vec::new(),
            marfuda: Vec::new(),
        }
    }

    /// Records a container that was read.
    pub fn sajjil_qira(&mut self, hawiya: impl Into<String>, adad: usize, wasf: impl Into<String>) {
        self.maqrua.push(MadkhalQira {
            hawiya: hawiya.into(),
            adad,
            wasf: wasf.into(),
        });
    }

    /// Records a container that was skipped.
    ///
    /// Takes a reason and not a count of what was salvaged, because there is no
    /// salvage: a refused container contributes nothing. See this module's
    /// header.
    pub fn sajjil(&mut self, hawiya: impl Into<String>, asl: Option<String>, sabab: SababRafd) {
        self.marfuda.push(MadkhalRafd {
            hawiya: hawiya.into(),
            asl,
            sabab,
        });
    }

    /// How many strings were extracted in total.
    #[must_use]
    pub fn adad_nusus(&self) -> usize {
        self.maqrua.iter().map(|q| q.adad).sum()
    }

    /// How many containers were refused in a way that lost strings.
    #[must_use]
    pub fn adad_khasara(&self) -> usize {
        self.marfuda.iter().filter(|q| q.sabab.khasara()).count()
    }

    /// Whether running the game with capture on would recover anything.
    ///
    /// What the interface consults before offering the capture path. Offering it
    /// for a library of corrupt archives would be offering a remedy that cannot
    /// work.
    #[must_use]
    pub fn yanfa_iltiqat(&self) -> bool {
        self.marfuda.iter().any(|q| q.sabab.yuslihuhu_iltiqat())
    }

    /// Refusals grouped by reason, so the report reads as four lines rather than
    /// four hundred.
    ///
    /// A Unity game with a stripped type tree produces one refusal per object
    /// type across hundreds of files, and listing them individually is a wall
    /// nobody reads. Grouped, it becomes "the type layout is missing, in 340
    /// containers, and here is what to do".
    #[must_use]
    pub fn majmua(&self) -> BTreeMap<String, Vec<&MadkhalRafd>> {
        let mut majmuat: BTreeMap<String, Vec<&MadkhalRafd>> = BTreeMap::new();
        for madkhal in &self.marfuda {
            majmuat
                .entry(madkhal.sabab.miftah().to_owned())
                .or_default()
                .push(madkhal);
        }
        majmuat
    }

    /// The report as lines, for the log and the diagnostics bundle.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut satr = vec![format!(
            "{} container(s) read, {} string(s) extracted, {} container(s) refused",
            self.maqrua.len(),
            self.adad_nusus(),
            self.marfuda.len()
        )];
        for (_, majmua) in self.majmua() {
            if let Some(awwal) = majmua.first() {
                satr.push(format!("  {} × {}", majmua.len(), awwal.sabab.injilizi()));
                satr.push(format!("      remedy: {}", awwal.sabab.ilaj_injilizi()));
            }
        }
        satr
    }

    /// Folds another report into this one.
    ///
    /// Extraction runs per container family and each produces its own report;
    /// this is how they become one thing the interface shows.
    pub fn dammij(&mut self, akhar: Self) {
        self.maqrua.extend(akhar.maqrua);
        self.marfuda.extend(akhar.marfuda);
    }
}

/// One reason's share of a container's member refusals.
#[derive(Debug)]
struct MajmuatRafd {
    /// The first member refused this way, which is the one the entry names.
    asl: String,
    /// The first refusal, which is the one the entry carries.
    sabab: SababRafd,
    /// How many members were refused this way.
    adad: usize,
}

/// Member refusals inside one container, folded into one entry per reason.
///
/// A container is read member by member, and the reader refuses each one it
/// cannot open on its own. Dropping those refusals is the defect this crate
/// keeps finding: an encrypted package whose every member was skipped in
/// silence is byte-identical, in the report, to a package that holds no text.
/// Recording every one is the other failure: an export whose encryption filter
/// matched every scene fails thousands of times the same way, and a report with
/// a line per member is a wall nobody reads. So they are folded here, one line
/// per kind of reason, naming the first member and carrying the count — which
/// keeps the refusal's variant, and with it the remedy the interface offers.
#[derive(Debug)]
pub struct MujammiRafd {
    hawiya: String,
    majmuat: BTreeMap<&'static str, MajmuatRafd>,
}

impl MujammiRafd {
    /// A collector for one container.
    #[must_use]
    pub fn jadeed(hawiya: impl Into<String>) -> Self {
        Self {
            hawiya: hawiya.into(),
            majmuat: BTreeMap::new(),
        }
    }

    /// Records one member the reader refused.
    ///
    /// The first refusal of each kind is kept whole; a later one of the same
    /// kind only raises the count. The one exception is a Unity object count,
    /// which is summed so that "objects skipped" stays a number of objects.
    pub fn sajjil(&mut self, asl: impl Into<String>, sabab: SababRafd) {
        match self.majmuat.entry(sabab.miftah()) {
            std::collections::btree_map::Entry::Vacant(khana) => {
                let _ = khana.insert(MajmuatRafd {
                    asl: asl.into(),
                    sabab,
                    adad: 1,
                });
            },
            std::collections::btree_map::Entry::Occupied(mut khana) => {
                let majmua = khana.get_mut();
                majmua.adad = majmua.adad.saturating_add(1);
                if let (
                    SababRafd::BilaShajaratAnwa { adad, .. },
                    SababRafd::BilaShajaratAnwa { adad: zaid, .. },
                ) = (&mut majmua.sabab, &sabab)
                {
                    *adad = adad.saturating_add(*zaid);
                }
            },
        }
    }

    /// How many members have been refused so far, over every reason.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.majmuat
            .values()
            .map(|majmua| majmua.adad)
            .fold(0, usize::saturating_add)
    }

    /// Whether nothing was refused.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.majmuat.is_empty()
    }

    /// Writes the folded refusals into the report, one per reason.
    pub fn ikhtim(self, taqreer: &mut TaqreerRafd) {
        let Self { hawiya, majmuat } = self;
        for majmua in majmuat.into_values() {
            taqreer.sajjil(
                hawiya.clone(),
                Some(majmua.asl),
                majmua.sabab.bi_adad(majmua.adad),
            );
        }
    }
}
