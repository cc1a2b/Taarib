//! أخطاء المخزن — what can go wrong between a value and the row that holds it.
//!
//! Four families, and the boundaries between them matter more than the count:
//!
//! 1. **The file cannot be opened or configured.** A missing directory, a
//!    permission the operating system refuses, a pragma the connection would not
//!    accept. Nothing has been read yet, so nothing is at risk.
//! 2. **The schema does not match this build.** The database declares a version
//!    newer than this binary understands, or a migration this build carries no
//!    longer matches the one that was actually applied. Both are refusals, and
//!    both are explained in [`crate::hijra`].
//! 3. **A statement failed.** A constraint rejected a write, a busy timeout
//!    expired, a query could not be prepared. The statement's own identity is
//!    carried as structured context so a log line names the operation and the
//!    table rather than quoting SQL back at a user.
//! 4. **The data is not what it was written as.** A discriminator column holds a
//!    value no variant answers to, a row that must exist does not, the file
//!    itself fails an integrity check. These are reported, always, and never
//!    repaired by starting over — a "let's rebuild the database" recovery path
//!    would delete a translator's projects to fix a typo in one column.
//!
//! Every variant keeps its data structured. No `rusqlite::Error` is ever
//! stringified into a message the interface shows; it nests as a source and
//! contributes its `SQLite` result code as context, because "database is locked"
//! and "disk image is malformed" need entirely different sentences and entirely
//! different next actions even though `SQLite` reports both the same way.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;

/// Failures of the local store.
#[derive(Debug, thiserror::Error)]
pub enum KhataMakhzan {
    /// The database file could not be opened.
    #[error("cannot open the database at {masar}")]
    TaadhurFath {
        /// The file, as `usus::masarat` resolved it.
        masar: PathBuf,
        /// What `SQLite` reported.
        #[source]
        sabab: rusqlite::Error,
    },

    /// A per-connection pragma was refused.
    ///
    /// Separated from an ordinary statement failure because these run before any
    /// data is touched, and a connection that could not enable foreign keys or
    /// WAL is a connection that must not be handed out at all rather than one
    /// that quietly works with the guarantees switched off.
    #[error("connection pragma {pragma} was refused")]
    TaadhurDabt {
        /// The pragma that failed.
        pragma: &'static str,
        /// What `SQLite` reported.
        #[source]
        sabab: rusqlite::Error,
    },

    /// The pool had no connection to give within its timeout.
    #[error("no connection available from the pool within {muhla_thawan}s")]
    TaadhurBirka {
        /// How long the caller waited, in seconds.
        muhla_thawan: u64,
        /// What the pool reported.
        #[source]
        sabab: r2d2::Error,
    },

    /// A statement failed for a reason that is neither corruption nor a lock.
    #[error("{amaliya} failed on {jadwal}")]
    TaadhurJumla {
        /// The operation, named in the code rather than quoted as SQL.
        amaliya: &'static str,
        /// The table it ran against.
        jadwal: &'static str,
        /// What `SQLite` reported.
        #[source]
        sabab: rusqlite::Error,
    },

    /// Another writer held the database for longer than the busy timeout.
    ///
    /// On a normal machine this is a scan and an install colliding, and retrying
    /// resolves it. It is a distinct variant because telling a user their
    /// database is corrupt when it is merely busy would be a lie that costs them
    /// a support conversation.
    #[error("the database was locked by another writer for more than {muhla_thawan}s")]
    QaidaMuqfala {
        /// The operation that gave up.
        amaliya: &'static str,
        /// The busy timeout that expired, in seconds.
        muhla_thawan: u64,
    },

    /// The file is not a readable `SQLite` database, or its pages are damaged.
    ///
    /// Reported and never acted on. The store does not delete, rename, truncate
    /// or recreate the file, because everything a user cannot get back — their
    /// translation projects, their patch lineage, their backup manifests — lives
    /// in it, and a recovery path that starts fresh is a recovery path that
    /// destroys exactly the data nobody else has a copy of.
    #[error("the database file is damaged: {tafsil}")]
    QaidaTalifa {
        /// What the integrity check or `SQLite` itself reported.
        tafsil: String,
    },

    /// The database was written by a newer build of Taarib.
    #[error("database schema is {mawjud}, this build understands {madum}")]
    IsdarAhdath {
        /// The version recorded in the file.
        mawjud: u32,
        /// The highest version this build defines.
        madum: u32,
    },

    /// A migration failed while being applied.
    #[error("migration {raqm} ({ism}) failed")]
    HijraFashila {
        /// The migration's number.
        raqm: u32,
        /// Its name, as this build carries it.
        ism: &'static str,
        /// What `SQLite` reported.
        #[source]
        sabab: rusqlite::Error,
    },

    /// A migration recorded as applied does not match the one this build ships.
    ///
    /// This means a shipped migration was edited after release. Two users on the
    /// same version number would then hold two different schemas, and every
    /// later migration would be written against whichever one its author
    /// happened to have. The store refuses rather than continue on a schema it
    /// cannot describe.
    #[error("migration {raqm} ({ism}) was applied as {masjala}, this build carries {mahmula}")]
    BasmaHijraMukhtalifa {
        /// The migration's number.
        raqm: u32,
        /// Its name, as this build carries it.
        ism: &'static str,
        /// The checksum recorded when it was applied.
        masjala: String,
        /// The checksum of the statement this build carries.
        mahmula: String,
    },

    /// The database records a migration this build does not define.
    ///
    /// Distinct from [`Self::IsdarAhdath`]: the version counter can be in range
    /// while an individual step is unknown, which happens when someone runs a
    /// build from a branch that never carried that migration.
    #[error("the database records migration {raqm}, which this build does not define")]
    HijraMajhula {
        /// The number with no definition.
        raqm: u32,
    },

    /// The applied migrations have a hole in them.
    ///
    /// Migration 5 is recorded but 3 is not. The schema is then whatever the
    /// missing step would have produced, minus that step, which is a shape no
    /// build has ever been tested against. Applying 3 now, on top of 5, would
    /// run it against tables it was never written for.
    #[error("migration {raqm} is missing although migration {aqsa} is recorded")]
    HijraNaqisa {
        /// The step that was never applied.
        raqm: u32,
        /// The highest step that was.
        aqsa: u32,
    },

    /// A stored value does not decode into the type it was written from.
    ///
    /// Almost always a discriminator column — a launcher family, an engine
    /// backend, an injection tier — holding text no variant answers to. The row
    /// is reported with the offending value verbatim so the fix is one `UPDATE`
    /// rather than an afternoon of guessing.
    #[error("{jadwal}.{amud} holds {qeema}, which decodes to nothing")]
    SafTalif {
        /// The table.
        jadwal: &'static str,
        /// The column.
        amud: &'static str,
        /// The value as stored.
        qeema: String,
    },

    /// A path cannot be stored because it is not valid Unicode.
    ///
    /// `SQLite`'s `TEXT` is UTF-8 and a Linux path is an arbitrary byte string, so
    /// the two do not always meet. The store refuses rather than writing a lossy
    /// rendering: an install root that came back from the database with a
    /// replacement character in it would send the installer to create a
    /// directory beside the game instead of writing into it.
    #[error("{amud} cannot hold {masar}: the path is not valid Unicode")]
    MasarGhayrNassi {
        /// The column it was going into.
        amud: &'static str,
        /// The path, rendered lossily for the message only.
        masar: PathBuf,
    },

    /// A row the caller named does not exist.
    #[error("no row in {jadwal} for {miftah}")]
    SafMafqud {
        /// The table.
        jadwal: &'static str,
        /// The key that found nothing.
        miftah: String,
    },

    /// A transaction could not be started, committed, or rolled back.
    #[error("the transaction could not be {marhala}")]
    TaadhurMuamala {
        /// Which step failed: `begun`, `committed`, or `rolled back`.
        marhala: &'static str,
        /// What `SQLite` reported.
        #[source]
        sabab: rusqlite::Error,
    },
}

impl KhataMakhzan {
    /// Classifies a `rusqlite` failure into the variant that actually describes
    /// it.
    ///
    /// `SQLite` reports a locked database, a malformed page image, and a broken
    /// constraint through one error type. Collapsing all three into "a statement
    /// failed" would mean the interface offers "retry" for corruption and
    /// "report this" for a lock, so the result code is inspected once, here,
    /// and never again at a call site.
    #[must_use]
    pub fn min_rusqlite(
        amaliya: &'static str,
        jadwal: &'static str,
        muhla_thawan: u64,
        sabab: rusqlite::Error,
    ) -> Self {
        use rusqlite::ffi::ErrorCode as RamzSqlite;

        if let rusqlite::Error::SqliteFailure(qeema, risala) = &sabab {
            match qeema.code {
                RamzSqlite::DatabaseCorrupt | RamzSqlite::NotADatabase => {
                    return Self::QaidaTalifa {
                        tafsil: risala.clone().unwrap_or_else(|| qeema.to_string()),
                    };
                },
                RamzSqlite::DatabaseBusy | RamzSqlite::DatabaseLocked => {
                    return Self::QaidaMuqfala {
                        amaliya,
                        muhla_thawan,
                    };
                },
                _ => {},
            }
        }

        Self::TaadhurJumla {
            amaliya,
            jadwal,
            sabab,
        }
    }
}

impl Tafsir for KhataMakhzan {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MAKHZAN
                + match self {
                    Self::TaadhurFath { .. } => 0,
                    Self::TaadhurDabt { .. } => 1,
                    Self::TaadhurBirka { .. } => 2,
                    Self::TaadhurJumla { .. } => 3,
                    Self::QaidaMuqfala { .. } => 4,
                    Self::QaidaTalifa { .. } => 5,
                    Self::IsdarAhdath { .. } => 6,
                    Self::HijraFashila { .. } => 7,
                    Self::BasmaHijraMukhtalifa { .. } => 8,
                    Self::HijraMajhula { .. } => 9,
                    Self::SafTalif { .. } => 10,
                    Self::SafMafqud { .. } => 11,
                    Self::TaadhurMuamala { .. } => 12,
                    Self::HijraNaqisa { .. } => 13,
                    Self::MasarGhayrNassi { .. } => 14,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Damage, or a schema this build cannot describe. Something needs a
            // person's attention before the store is written to again.
            Self::QaidaTalifa { .. }
            | Self::IsdarAhdath { .. }
            | Self::BasmaHijraMukhtalifa { .. }
            | Self::HijraMajhula { .. }
            | Self::HijraNaqisa { .. }
            | Self::HijraFashila { .. }
            | Self::SafTalif { .. } => Khutura::Fadih,
            // The operation did not happen, and the user asked for it.
            Self::TaadhurFath { .. }
            | Self::TaadhurDabt { .. }
            | Self::TaadhurBirka { .. }
            | Self::TaadhurJumla { .. }
            | Self::TaadhurMuamala { .. }
            | Self::MasarGhayrNassi { .. }
            | Self::SafMafqud { .. } => Khutura::Khatar,
            // Transient by definition: another writer had it.
            Self::QaidaMuqfala { .. } => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::TaadhurFath { masar, .. } => {
                format!("تعذّر فتح قاعدة بيانات تعريب: {}", masar.display())
            },
            Self::TaadhurDabt { pragma, .. } => format!(
                "تعذّر ضبط الاتصال بقاعدة البيانات (الخاصية {pragma})، ولن يُستخدم اتصال ناقص \
                 الضمانات."
            ),
            Self::TaadhurBirka { muhla_thawan, .. } => {
                format!("لم يتوفّر اتصال بقاعدة البيانات خلال {muhla_thawan} ثانية. جرّب مرة أخرى.")
            },
            Self::TaadhurJumla { .. } => "تعذّرت عملية على قاعدة بيانات تعريب المحلية.".to_owned(),
            Self::QaidaMuqfala { muhla_thawan, .. } => format!(
                "قاعدة البيانات مشغولة بعملية أخرى منذ أكثر من {muhla_thawan} ثانية. \
                 قد يكون فحص المكتبة أو تثبيت رقعة جاريًا؛ جرّب مرة أخرى."
            ),
            Self::QaidaTalifa { .. } => {
                "ملف قاعدة بيانات تعريب تالف. لن يُحذف ولن يُعاد إنشاؤه تلقائيًا لأن مشاريع \
                 الترجمة والنسخ الاحتياطية بداخله؛ أرسل حزمة التشخيص."
                    .to_owned()
            },
            Self::IsdarAhdath { mawjud, madum } => format!(
                "قاعدة البيانات كُتبت بإصدار أحدث من تعريب (مخطّط {mawjud} مقابل {madum}). \
                 حدِّث البرنامج؛ فتحها بهذا الإصدار قد يفقد جزءًا منها."
            ),
            Self::HijraFashila { raqm, .. } => format!(
                "فشلت ترقية قاعدة البيانات رقم {raqm}، وأُعيدت القاعدة إلى حالتها السابقة كاملة."
            ),
            Self::BasmaHijraMukhtalifa { raqm, .. } => format!(
                "الترقية رقم {raqm} المطبَّقة على هذه القاعدة تختلف عمّا يحمله هذا الإصدار. \
                 لا يمكن الاعتماد على شكل الجداول، فتوقّف تعريب عن الكتابة."
            ),
            Self::HijraMajhula { raqm } => {
                format!("قاعدة البيانات تسجّل الترقية رقم {raqm} وهي غير معرَّفة في هذا الإصدار.")
            },
            Self::HijraNaqisa { raqm, aqsa } => format!(
                "الترقية رقم {raqm} غير مطبَّقة رغم تسجيل الترقية رقم {aqsa} بعدها. \
                 شكل الجداول غير معروف، فتوقّف تعريب عن الكتابة."
            ),
            Self::SafTalif { jadwal, amud, .. } => format!(
                "قيمة غير معروفة في العمود {amud} من الجدول {jadwal}؛ لن تُقرأ حتى لا يُبنى \
                 عليها قرار خاطئ."
            ),
            Self::MasarGhayrNassi { masar, .. } => format!(
                "المسار {} يحتوي بايتات ليست بترميز UTF-8، ولن يُحفظ ناقصًا حتى لا يُكتب في \
                 موضع خاطئ لاحقًا.",
                masar.display()
            ),
            Self::SafMafqud { .. } => {
                "السجلّ المطلوب غير موجود في قاعدة البيانات المحلية. أعد فحص المكتبة.".to_owned()
            },
            Self::TaadhurMuamala { .. } => {
                "تعذّر إتمام عملية الكتابة في قاعدة البيانات، ولم يُكتب منها شيء.".to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::TaadhurFath { masar, .. } => {
                format!("Cannot open Taarib's database: {}", masar.display())
            },
            Self::TaadhurDabt { pragma, .. } => format!(
                "The database connection refused the {pragma} pragma; a connection missing that \
                 guarantee will not be used."
            ),
            Self::TaadhurBirka { muhla_thawan, .. } => format!(
                "No database connection became available within {muhla_thawan}s. Try again."
            ),
            Self::TaadhurJumla {
                amaliya, jadwal, ..
            } => {
                format!("A local database operation failed: {amaliya} on {jadwal}.")
            },
            Self::QaidaMuqfala { muhla_thawan, .. } => format!(
                "The database was held by another writer for more than {muhla_thawan}s. A \
                 library scan or a patch install may be running; try again."
            ),
            Self::QaidaTalifa { tafsil } => format!(
                "Taarib's database file is damaged ({tafsil}). It will not be deleted or \
                 recreated — your translation projects and backup manifests are in it. Send a \
                 diagnostics bundle."
            ),
            Self::IsdarAhdath { mawjud, madum } => format!(
                "This database was written by a newer Taarib (schema {mawjud}; this build reads \
                 {madum}). Update Taarib — opening it here would lose part of it."
            ),
            Self::HijraFashila { raqm, ism, .. } => format!(
                "Database migration {raqm} ({ism}) failed. The database was rolled back whole to \
                 the version it was at."
            ),
            Self::BasmaHijraMukhtalifa { raqm, ism, .. } => format!(
                "Migration {raqm} ({ism}) was applied to this database in a different form from \
                 the one this build carries. The schema cannot be trusted, so Taarib stopped."
            ),
            Self::HijraMajhula { raqm } => {
                format!("The database records migration {raqm}, which this build does not define.")
            },
            Self::HijraNaqisa { raqm, aqsa } => format!(
                "Migration {raqm} was never applied although migration {aqsa} after it was. The \
                 schema is a shape no build has been tested against, so Taarib stopped."
            ),
            Self::SafTalif {
                jadwal,
                amud,
                qeema,
            } => format!(
                "Column {amud} of table {jadwal} holds {qeema}, which is not a value Taarib \
                 defines. The row will not be read rather than be read wrongly."
            ),
            Self::MasarGhayrNassi { amud, masar } => format!(
                "The path {} contains bytes that are not UTF-8. It will not be stored lossily, \
                 because {amud} read back with a replacement character would point somewhere \
                 that is not the original.",
                masar.display()
            ),
            Self::SafMafqud { jadwal, miftah } => {
                format!("No row in {jadwal} for {miftah}. Rescan the library.")
            },
            Self::TaadhurMuamala { marhala, .. } => {
                format!("A database transaction could not be {marhala}; nothing was written.")
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::IsdarAhdath { .. } | Self::HijraMajhula { .. } => Khutwa::TahdithTaarib,
            Self::QaidaTalifa { .. }
            | Self::BasmaHijraMukhtalifa { .. }
            | Self::HijraNaqisa { .. }
            | Self::HijraFashila { .. }
            | Self::SafTalif { .. } => Khutwa::IblaghLilMalik,
            Self::QaidaMuqfala { .. } | Self::TaadhurBirka { .. } | Self::TaadhurMuamala { .. } => {
                Khutwa::AadaMuhawala
            },
            Self::SafMafqud { .. } => Khutwa::AadaFahsMaktaba,
            Self::MasarGhayrNassi { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba,
            },
            Self::TaadhurFath { .. } | Self::TaadhurDabt { .. } | Self::TaadhurJumla { .. } => {
                Khutwa::FathTashkhis
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TaadhurFath { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                siyaq.extend(siyaq_sqlite(sabab));
            },
            Self::TaadhurDabt { pragma, sabab } => {
                let _ = siyaq.insert("pragma".to_owned(), QeemaSiyaq::Nass((*pragma).to_owned()));
                siyaq.extend(siyaq_sqlite(sabab));
            },
            Self::TaadhurBirka { muhla_thawan, .. } => {
                let _ = siyaq.insert("muhla".to_owned(), QeemaSiyaq::Hajm(*muhla_thawan));
            },
            Self::TaadhurJumla {
                amaliya,
                jadwal,
                sabab,
            } => {
                let _ = siyaq.insert(
                    "amaliya".to_owned(),
                    QeemaSiyaq::Nass((*amaliya).to_owned()),
                );
                let _ = siyaq.insert("jadwal".to_owned(), QeemaSiyaq::Nass((*jadwal).to_owned()));
                siyaq.extend(siyaq_sqlite(sabab));
            },
            Self::QaidaMuqfala {
                amaliya,
                muhla_thawan,
            } => {
                let _ = siyaq.insert(
                    "amaliya".to_owned(),
                    QeemaSiyaq::Nass((*amaliya).to_owned()),
                );
                let _ = siyaq.insert("muhla".to_owned(), QeemaSiyaq::Hajm(*muhla_thawan));
            },
            Self::QaidaTalifa { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::IsdarAhdath { mawjud, madum } => {
                let _ = siyaq.insert("mawjud".to_owned(), QeemaSiyaq::Raqm(i64::from(*mawjud)));
                let _ = siyaq.insert("madum".to_owned(), QeemaSiyaq::Raqm(i64::from(*madum)));
            },
            Self::HijraFashila { raqm, ism, sabab } => {
                let _ = siyaq.insert("hijra".to_owned(), QeemaSiyaq::Raqm(i64::from(*raqm)));
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass((*ism).to_owned()));
                siyaq.extend(siyaq_sqlite(sabab));
            },
            Self::BasmaHijraMukhtalifa {
                raqm,
                ism,
                masjala,
                mahmula,
            } => {
                let _ = siyaq.insert("hijra".to_owned(), QeemaSiyaq::Raqm(i64::from(*raqm)));
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass((*ism).to_owned()));
                let _ = siyaq.insert("masjala".to_owned(), QeemaSiyaq::Nass(masjala.clone()));
                let _ = siyaq.insert("mahmula".to_owned(), QeemaSiyaq::Nass(mahmula.clone()));
            },
            Self::HijraMajhula { raqm } => {
                let _ = siyaq.insert("hijra".to_owned(), QeemaSiyaq::Raqm(i64::from(*raqm)));
            },
            Self::HijraNaqisa { raqm, aqsa } => {
                let _ = siyaq.insert("hijra".to_owned(), QeemaSiyaq::Raqm(i64::from(*raqm)));
                let _ = siyaq.insert("aqsa".to_owned(), QeemaSiyaq::Raqm(i64::from(*aqsa)));
            },
            Self::SafTalif {
                jadwal,
                amud,
                qeema,
            } => {
                let _ = siyaq.insert("jadwal".to_owned(), QeemaSiyaq::Nass((*jadwal).to_owned()));
                let _ = siyaq.insert("amud".to_owned(), QeemaSiyaq::Nass((*amud).to_owned()));
                let _ = siyaq.insert("qeema".to_owned(), QeemaSiyaq::Nass(qeema.clone()));
            },
            Self::MasarGhayrNassi { amud, masar } => {
                let _ = siyaq.insert("amud".to_owned(), QeemaSiyaq::Nass((*amud).to_owned()));
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            },
            Self::SafMafqud { jadwal, miftah } => {
                let _ = siyaq.insert("jadwal".to_owned(), QeemaSiyaq::Nass((*jadwal).to_owned()));
                let _ = siyaq.insert("miftah".to_owned(), QeemaSiyaq::Nass(miftah.clone()));
            },
            Self::TaadhurMuamala { marhala, sabab } => {
                let _ = siyaq.insert(
                    "marhala".to_owned(),
                    QeemaSiyaq::Nass((*marhala).to_owned()),
                );
                siyaq.extend(siyaq_sqlite(sabab));
            },
        }
        siyaq
    }
}

/// Builds structured context out of a `rusqlite` failure without stringifying
/// the whole error.
///
/// The primary and extended result codes are what a maintainer greps a
/// diagnostics bundle for: `SQLITE_CONSTRAINT_FOREIGNKEY` (extended code 787)
/// names the exact constraint family that rejected a write, and losing it into a
/// prose sentence loses the diagnosis with it.
#[must_use]
pub fn siyaq_sqlite(khata: &rusqlite::Error) -> BTreeMap<String, QeemaSiyaq> {
    let mut siyaq = BTreeMap::new();
    if let rusqlite::Error::SqliteFailure(qeema, risala) = khata {
        let _ = siyaq.insert(
            "sqlite".to_owned(),
            QeemaSiyaq::Nass(format!("{:?}", qeema.code)),
        );
        let _ = siyaq.insert(
            "sqlite_mumtadd".to_owned(),
            QeemaSiyaq::Raqm(i64::from(qeema.extended_code)),
        );
        if let Some(nass) = risala {
            let _ = siyaq.insert("sqlite_risala".to_owned(), QeemaSiyaq::Nass(nass.clone()));
        }
    } else {
        let _ = siyaq.insert("sqlite".to_owned(), QeemaSiyaq::Nass(khata.to_string()));
    }
    siyaq
}

khata_min!(KhataMakhzan);
