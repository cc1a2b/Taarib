//! أخطاء الترجمة — what fails a string, what fails a run, and the difference.
//!
//! Three classes, and confusing them produces two different bad products.
//!
//! **A string failure** stops one string and nothing else. A placeholder that
//! did not come back, a provider that refused this particular input, a reply
//! that would not parse. The run continues, the string stays untranslated with
//! its reason recorded, and the contributor retries it or translates it by hand.
//! These are the ordinary weather of machine translation and modelling them as
//! run failures would abort a ten-thousand-string batch over one bad reply.
//!
//! **A run failure** stops the batch. No credential, a budget ceiling reached, a
//! provider that is down. The project keeps everything already written — every
//! result is committed as it arrives — and the run resumes from where it
//! stopped.
//!
//! **A configuration failure** stops before anything is attempted. A glossary
//! file that will not parse, a memory database that cannot be opened.
//!
//! ## Nothing here ever carries a credential
//!
//! Not in a variant, not in a context map, not in a `Display`. The credential
//! type in [`crate::muzawwidun`] redacts in both `Debug` and `Display`, so a
//! variant that carried one could not leak it — but no variant carries one
//! anyway, because the narrower guarantee is the one worth having: a key cannot
//! reach a log through this module because it is never put into this module.
//!
//! ## Error codes
//!
//! This crate owns [`arqam::TARJAMA`], which is 5100, in its entirety.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Failures of the translation pipeline.
#[derive(Debug, thiserror::Error)]
pub enum KhataTarjama {
    // --- protection: every one of these fails exactly one string ------------
    /// An expected token did not come back.
    #[error("{ramz} is missing from the reply")]
    RamzMafqud {
        /// The token that was sent and did not return.
        ramz: String,
        /// The first part of what came back instead, so a contributor can see
        /// what the model did.
        radd: String,
    },

    /// A token came back more than once.
    ///
    /// A model that repeated a placeholder has produced a string that would
    /// substitute the same value twice, which the game's own formatter will
    /// either reject or render wrong. There is no correct repair: deleting one
    /// of them guesses which.
    #[error("{ramz} came back {adad} times; it was sent once")]
    RamzMukarrar {
        /// The token.
        ramz: String,
        /// How many times it appeared.
        adad: usize,
    },

    /// The reply contains a token that was never sent.
    #[error("{ramz} is in the reply and was never sent")]
    RamzDakhil {
        /// The invented token.
        ramz: String,
        /// The first part of the reply.
        radd: String,
    },

    /// A span's closing token precedes its opening one.
    #[error("span {fahras} closes before it opens")]
    RamzMaqlub {
        /// Which span.
        fahras: usize,
    },

    /// A token came back damaged rather than moved.
    ///
    /// Distinct from [`KhataTarjama::RamzMafqud`] and the distinction is the
    /// useful part of the report: a missing token means the model dropped it,
    /// and a damaged one means the model tried to reproduce it and failed —
    /// which is a prompt problem rather than a capability problem.
    #[error("a token came back damaged: {sabab}")]
    RamzTalif {
        /// The damaged fragment.
        juz: String,
        /// What is wrong with it.
        sabab: String,
    },

    /// The string carries more markup than this build will protect.
    #[error("{adad} spans, above this build's ceiling of {saqf}")]
    RumuzKathira {
        /// How many the string has.
        adad: usize,
        /// The ceiling.
        saqf: usize,
    },

    /// A span names a range that is not inside its own clean text.
    ///
    /// The extractor and the text disagree, which means one of them is wrong
    /// and splicing on the disagreement would corrupt the string being
    /// protected. Refused rather than clamped: a clamped range silently moves a
    /// placeholder's boundary.
    #[error("a span at {bidaya}+{tul} is outside its {madaa}-byte text")]
    NitaqKharij {
        /// The span's start.
        bidaya: u32,
        /// Its length.
        tul: u32,
        /// The text's length.
        madaa: u32,
    },

    // --- providers: these fail one string or one run, per variant -----------
    /// The provider refused this particular input.
    #[error("{muzawwid} refused this string: {sabab}")]
    MudkhalMarfud {
        /// Which provider.
        muzawwid: String,
        /// Why, in the provider's own words.
        sabab: String,
    },

    /// The provider's reply could not be parsed as the structure that was
    /// requested.
    ///
    /// Every provider in this crate is asked for structured output and the
    /// reply is parsed as data. A reply that does not parse is a failure rather
    /// than something to extract prose from with a pattern — see
    /// [`crate::muzawwidun`] for why a natural-language reply is never scraped.
    #[error("{muzawwid} returned a reply that is not the requested structure")]
    RaddGhayrMufassal {
        /// Which provider.
        muzawwid: String,
        /// The first part of the reply.
        radd: String,
    },

    /// The provider is not reachable.
    #[error("{muzawwid} could not be reached: {sabab}")]
    MuzawwidGhayrMutah {
        /// Which provider.
        muzawwid: String,
        /// What the transport said.
        sabab: String,
    },

    /// No credential has been supplied for this provider.
    ///
    /// **Not a failure to recover from by trying anyway.** Nothing in this
    /// crate calls a paid endpoint before a user has entered a credential and
    /// confirmed, so this is the state a run starts in rather than one it
    /// falls into.
    #[error("{muzawwid} has no credential")]
    BilaItimad {
        /// Which provider.
        muzawwid: String,
    },

    /// The provider's rate limit was hit and backoff did not clear it.
    #[error("{muzawwid} is rate limiting and backoff did not clear it")]
    HaddMuadal {
        /// Which provider.
        muzawwid: String,
        /// How long the provider asked us to wait, in seconds, when it said.
        thawani: Option<u64>,
    },

    /// A string is longer than the free web service takes in one request.
    ///
    /// Fails one string. The free endpoint is a `GET` whose text rides in the
    /// query string, and the ceiling is the endpoint's, not this crate's: a
    /// longer string is refused before it is sent, so it costs no round trip
    /// and does not disturb the request spacing the other strings depend on.
    #[error("{muzawwid} takes at most {saqf} characters per request; this string is {ahruf}")]
    MajjaniTawil {
        /// Which provider.
        muzawwid: String,
        /// The string's length, in characters.
        ahruf: usize,
        /// The per-request ceiling, in characters.
        saqf: usize,
    },

    /// A credential-free service is refusing this machine's requests.
    ///
    /// Stops the run. A `403`, a block page served where JSON was expected, or
    /// rate limiting that outlasted every retry all mean the same thing for a
    /// service with no account behind it: it has flagged this client, and the
    /// next string would meet the same wall. Distinct from
    /// [`KhataTarjama::MuzawwidGhayrMutah`] because the remedy differs — there
    /// is no credential to fix and no status page to wait on, only time, or a
    /// keyed provider.
    #[error("the free service {muzawwid} is refusing this machine's requests: {sabab}")]
    MajjaniMahjub {
        /// Which provider.
        muzawwid: String,
        /// What the wire said.
        sabab: String,
    },

    // --- runs ---------------------------------------------------------------
    /// The run reached the cost ceiling the user set.
    ///
    /// Stopped **at** the ceiling rather than past it: the ceiling is a promise
    /// about what will be spent, and a run that noticed after exceeding it
    /// would have broken that promise before reporting it.
    #[error("the run stopped at its cost ceiling of {saqf} after spending {munfaq}")]
    SaqfTakalif {
        /// What has been spent, in the smallest unit of the run's currency.
        munfaq: u64,
        /// The ceiling.
        saqf: u64,
    },

    // --- configuration ------------------------------------------------------
    /// A glossary file could not be read or parsed.
    #[error("the glossary at {masar} could not be read: {sabab}")]
    MasradTalif {
        /// The file.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// The translation memory could not be opened.
    #[error("the translation memory at {masar} could not be opened: {sabab}")]
    DhakiraMughlaqa {
        /// The database.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// A project file could not be written while a run was committing results.
    #[error("{masar} could not be written")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// What the filesystem said.
        sabab: std::io::Error,
    },
}

impl KhataTarjama {
    /// Whether this failure stops one string or the whole run.
    ///
    /// The distinction the module header opens with, made checkable so that a
    /// batch loop does not have to enumerate variants — and so that a variant
    /// added later without updating a scattered comparison cannot silently
    /// abort a run it should only have skipped a string in.
    #[must_use]
    pub const fn yuqif_aljawla(&self) -> bool {
        matches!(
            self,
            Self::BilaItimad { .. }
                | Self::HaddMuadal { .. }
                | Self::MajjaniMahjub { .. }
                | Self::SaqfTakalif { .. }
                | Self::MuzawwidGhayrMutah { .. }
                | Self::MasradTalif { .. }
                | Self::DhakiraMughlaqa { .. }
                | Self::KhataMalaf { .. }
        )
    }

    /// Whether this failure is one of the protection refusals.
    ///
    /// Consulted by the quality-flag layer, which turns exactly these into
    /// [`taarib_mustalahat::nass::AlamJawda::NasqMaksur`] on the entry. Written
    /// as a method so that adding a protection failure adds it to the flag as
    /// well, rather than to a list somewhere that a reviewer has to remember.
    #[must_use]
    pub const fn khalal_himaya(&self) -> bool {
        matches!(
            self,
            Self::RamzMafqud { .. }
                | Self::RamzMukarrar { .. }
                | Self::RamzDakhil { .. }
                | Self::RamzMaqlub { .. }
                | Self::RamzTalif { .. }
                | Self::RumuzKathira { .. }
                | Self::NitaqKharij { .. }
        )
    }
}

impl Tafsir for KhataTarjama {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::TARJAMA
                + match self {
                    Self::RamzMafqud { .. } => 0,
                    Self::RamzMukarrar { .. } => 1,
                    Self::RamzDakhil { .. } => 2,
                    Self::RamzMaqlub { .. } => 3,
                    Self::RamzTalif { .. } => 4,
                    Self::RumuzKathira { .. } => 5,
                    Self::NitaqKharij { .. } => 6,
                    Self::MudkhalMarfud { .. } => 7,
                    Self::RaddGhayrMufassal { .. } => 8,
                    Self::MuzawwidGhayrMutah { .. } => 9,
                    Self::BilaItimad { .. } => 10,
                    Self::HaddMuadal { .. } => 11,
                    Self::SaqfTakalif { .. } => 12,
                    Self::MasradTalif { .. } => 13,
                    Self::DhakiraMughlaqa { .. } => 14,
                    Self::KhataMalaf { .. } => 15,
                    Self::MajjaniTawil { .. } => 16,
                    Self::MajjaniMahjub { .. } => 17,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // One string did not translate. The run is fine and the source is
            // intact, which is the whole point of refusing rather than
            // repairing.
            Self::RamzMafqud { .. }
            | Self::RamzMukarrar { .. }
            | Self::RamzDakhil { .. }
            | Self::RamzMaqlub { .. }
            | Self::RamzTalif { .. }
            | Self::RumuzKathira { .. }
            | Self::MudkhalMarfud { .. }
            | Self::MajjaniTawil { .. }
            | Self::RaddGhayrMufassal { .. } => Khutura::Tanbeeh,

            // The user set a ceiling and it was honoured. Information, not a
            // fault.
            Self::SaqfTakalif { .. } | Self::BilaItimad { .. } => Khutura::Maluma,

            // A span and its text disagree, which means an extraction is wrong
            // and every string from that container is suspect. The rest stop
            // the run outright.
            Self::NitaqKharij { .. }
            | Self::MuzawwidGhayrMutah { .. }
            | Self::HaddMuadal { .. }
            | Self::MajjaniMahjub { .. }
            | Self::MasradTalif { .. }
            | Self::DhakiraMughlaqa { .. }
            | Self::KhataMalaf { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::RamzMafqud { ramz, .. } => format!(
                "أعادت الخدمة ترجمةً ينقصها العنصر المحفوظ ({ramz})، وهو رمز تستبدله اللعبة \
                 بقيمة أثناء التشغيل. تُركت هذه العبارة بلا ترجمة بدل إصلاحها بالتخمين."
            ),
            Self::RamzMukarrar { ramz, .. } => format!(
                "تكرّر العنصر المحفوظ ({ramz}) في الترجمة وقد أُرسل مرة واحدة. تُركت العبارة \
                 بلا ترجمة."
            ),
            Self::RamzDakhil { ramz, .. } => format!(
                "أضافت الخدمة عنصرًا محفوظًا ({ramz}) لم يكن في الأصل، وتُركت العبارة بلا ترجمة."
            ),
            Self::RamzMaqlub { .. } => {
                "عاد أحد نطاقات التنسيق مقلوبًا، وتُركت العبارة بلا ترجمة.".to_owned()
            },
            Self::RamzTalif { .. } => {
                "عاد أحد العناصر المحفوظة تالفًا، وتُركت العبارة بلا ترجمة.".to_owned()
            },
            Self::RumuzKathira { .. } => {
                "تحتوي هذه العبارة على وسوم أكثر مما يمكن حمايته بأمان، ولم تُرسل للترجمة."
                    .to_owned()
            },
            Self::NitaqKharij { .. } => {
                "لا يتطابق أحد نطاقات التنسيق مع نص العبارة، ويبدو أن الاستخراج غير سليم.".to_owned()
            },
            Self::MudkhalMarfud { muzawwid, .. } => {
                format!("رفضت خدمة ({muzawwid}) هذه العبارة.")
            },
            Self::RaddGhayrMufassal { muzawwid, .. } => {
                format!("أعادت خدمة ({muzawwid}) ردًّا ليس بالصيغة المطلوبة، ولم يُقرأ.")
            },
            Self::MuzawwidGhayrMutah { muzawwid, .. } => {
                format!("تعذّر الوصول إلى خدمة ({muzawwid}).")
            },
            Self::BilaItimad { muzawwid } => format!(
                "لم تُدخل بعدُ مفتاح خدمة ({muzawwid}). لا يتصل تعريب بأي خدمة مدفوعة قبل \
                 إدخالك المفتاح وتأكيدك."
            ),
            Self::HaddMuadal { muzawwid, .. } => {
                format!("تجاوزت الطلبات حدّ خدمة ({muzawwid})، ولم تنفع إعادة المحاولة.")
            },
            Self::MajjaniTawil {
                muzawwid,
                ahruf,
                saqf,
            } => format!(
                "هذه العبارة {ahruf} حرفًا، وخدمة الترجمة المجانية ({muzawwid}) لا تقبل أكثر من \
                 {saqf} حرفًا في الطلب الواحد. تُركت بلا ترجمة؛ مزوّد بمفتاح تضيفه في الإعدادات \
                 ← المزوّدون يترجم العبارات الأطول."
            ),
            Self::MajjaniMahjub { muzawwid, sabab } => format!(
                "خدمة الترجمة المجانية ({muzawwid}) ترفض طلبات هذا الجهاز الآن: {sabab}. هي خدمة \
                 غير رسمية بحدود غير معلنة، وقد يدوم الحجب دقائق أو ساعات. ما تُرجم محفوظ؛ انتظر \
                 ثم أعد المحاولة، أو أضف مزوّدًا بمفتاح في الإعدادات ← المزوّدون فيتقدّم عليها."
            ),
            Self::SaqfTakalif { .. } => {
                "بلغت الجولة سقف التكلفة الذي حدّدته وتوقّفت عنده. ما تُرجم محفوظ، ويمكن \
                 المتابعة برفع السقف."
                    .to_owned()
            },
            Self::MasradTalif { .. } => "تعذّرت قراءة ملف المسرد.".to_owned(),
            Self::DhakiraMughlaqa { .. } => "تعذّر فتح ذاكرة الترجمة.".to_owned(),
            Self::KhataMalaf { .. } => "تعذّرت الكتابة في ملفات المشروع.".to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            // The two free-service refusals say where the better provider is
            // chosen; `Display` stays the terse form the log wants.
            Self::MajjaniTawil {
                muzawwid,
                ahruf,
                saqf,
            } => format!(
                "This string is {ahruf} characters and the free service ({muzawwid}) takes at \
                 most {saqf} per request. It is left untranslated; a keyed provider added in \
                 Settings, under Providers, handles longer strings."
            ),
            Self::MajjaniMahjub { muzawwid, sabab } => format!(
                "The free service ({muzawwid}) is refusing this machine's requests: {sabab}. It \
                 is unofficial, with undocumented limits, and a block can last minutes or \
                 hours. Everything translated so far is kept; wait and retry, or add a keyed \
                 provider in Settings, under Providers, which takes precedence."
            ),
            _ => self.to_string(),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // A prompt or a provider problem. Another provider very often
            // succeeds on the same string, and a transport or rate-limit
            // refusal clears on its own.
            Self::RamzMafqud { .. }
            | Self::RamzMukarrar { .. }
            | Self::RamzDakhil { .. }
            | Self::RamzMaqlub { .. }
            | Self::RamzTalif { .. }
            | Self::RaddGhayrMufassal { .. }
            | Self::MudkhalMarfud { .. }
            | Self::MuzawwidGhayrMutah { .. }
            | Self::HaddMuadal { .. } => Khutwa::AadaMuhawala,

            // The free service has no key to fix and no account to top up; the
            // one thing a user can change is which provider does the work.
            Self::BilaItimad { .. }
            | Self::SaqfTakalif { .. }
            | Self::MajjaniTawil { .. }
            | Self::MajjaniMahjub { .. } => Khutwa::FathIdadat {
                qism: taarib_usus::khata::QismIdadat::Muzawwidun,
            },

            // An extraction produced a span that does not fit its text. The
            // diagnostics bundle names the container.
            Self::NitaqKharij { .. } | Self::RumuzKathira { .. } => Khutwa::FathTashkhis,

            Self::MasradTalif { masar, .. } | Self::DhakiraMughlaqa { masar, .. } => {
                let _ = masar;
                Khutwa::IblaghLilMusahim
            },
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        // Handled before the general map exists. `siyaq_io` returns a different
        // map, and inserting the path into the one built below would insert it
        // into something that is then discarded — a bug four earlier crates in
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
            Self::RamzMafqud { ramz, radd } | Self::RamzDakhil { ramz, radd } => {
                daa("ramz", QeemaSiyaq::Nass(ramz.clone()));
                daa("radd", QeemaSiyaq::Nass(radd.clone()));
            },
            Self::RamzMukarrar { ramz, adad } => {
                daa("ramz", QeemaSiyaq::Nass(ramz.clone()));
                daa("adad", QeemaSiyaq::Hajm(tul_u64(*adad)));
            },
            Self::RamzMaqlub { fahras } => daa("fahras", QeemaSiyaq::Hajm(tul_u64(*fahras))),
            Self::RamzTalif { juz, sabab } => {
                daa("juz", QeemaSiyaq::Nass(juz.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::RumuzKathira { adad, saqf } => {
                daa("adad", QeemaSiyaq::Hajm(tul_u64(*adad)));
                daa("saqf", QeemaSiyaq::Hajm(tul_u64(*saqf)));
            },
            Self::NitaqKharij { bidaya, tul, madaa } => {
                daa("bidaya", QeemaSiyaq::Hajm(u64::from(*bidaya)));
                daa("tul", QeemaSiyaq::Hajm(u64::from(*tul)));
                daa("madaa", QeemaSiyaq::Hajm(u64::from(*madaa)));
            },
            Self::MudkhalMarfud { muzawwid, sabab }
            | Self::MuzawwidGhayrMutah { muzawwid, sabab }
            | Self::MajjaniMahjub { muzawwid, sabab } => {
                daa("muzawwid", QeemaSiyaq::Nass(muzawwid.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MajjaniTawil {
                muzawwid,
                ahruf,
                saqf,
            } => {
                daa("muzawwid", QeemaSiyaq::Nass(muzawwid.clone()));
                daa("ahruf", QeemaSiyaq::Hajm(tul_u64(*ahruf)));
                daa("saqf", QeemaSiyaq::Hajm(tul_u64(*saqf)));
            },
            Self::RaddGhayrMufassal { muzawwid, radd } => {
                daa("muzawwid", QeemaSiyaq::Nass(muzawwid.clone()));
                daa("radd", QeemaSiyaq::Nass(radd.clone()));
            },
            Self::BilaItimad { muzawwid } => daa("muzawwid", QeemaSiyaq::Nass(muzawwid.clone())),
            Self::HaddMuadal { muzawwid, thawani } => {
                daa("muzawwid", QeemaSiyaq::Nass(muzawwid.clone()));
                if let Some(thawani) = thawani {
                    daa("thawani", QeemaSiyaq::Hajm(*thawani));
                }
            },
            Self::SaqfTakalif { munfaq, saqf } => {
                daa("munfaq", QeemaSiyaq::Hajm(*munfaq));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            },
            Self::MasradTalif { masar, sabab } | Self::DhakiraMughlaqa { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            // Handled above.
            Self::KhataMalaf { .. } => {},
        }
        siyaq
    }
}

khata_min!(KhataTarjama);

/// A length as a `u64`, without a cast that can wrap.
#[must_use]
pub fn tul_u64(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}
