//! المخطّط — versioned data schemas and forward-only migration.
//!
//! Every structure Taarib persists — settings, projects, submissions, patch
//! metadata, registry records, installation manifests — carries the version of
//! the schema it was written with:
//!
//! ```json
//! { "mukhattat": 3, "ism": "…", "…": "…" }
//! ```
//!
//! Reading applies migrations one step at a time until the value matches the
//! running build. A file written by a *newer* build is refused rather than
//! guessed at: a partial read of a structure this build does not understand
//! would silently drop the fields it could not see, and then write them away
//! for good on the next save. Refusing costs the user one update; guessing
//! costs them their work.

use std::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::khata::{Khata, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use crate::khata_min;

/// The field every versioned structure carries.
pub const HAQL: &str = "mukhattat";

/// A structure that is persisted and therefore versioned.
pub trait DhuMukhattat: Serialize + DeserializeOwned + Sized {
    /// The structure's name, used in error messages and diagnostics.
    const ISM: &'static str;

    /// The schema version this build writes.
    const ISDAR: u32;

    /// Migrates a value one step, from `min` to `min + 1`.
    ///
    /// Implementations handle each historical version explicitly. A version
    /// with no arm is a gap in the chain and is reported as one rather than
    /// skipped.
    ///
    /// # Errors
    ///
    /// Returns [`KhataMukhattat::HijraNaqisa`] when no migration exists for the
    /// given step, and any error the migration itself produces.
    fn hijra(min: u32, qeema: Value) -> Natija<Value>;
}

/// Reads a versioned structure from JSON bytes, migrating it forward.
///
/// # Errors
///
/// Fails when the bytes are not JSON, are not an object, carry no version, were
/// written by a newer build, have a gap in their migration chain, or do not fit
/// the target structure after migration.
pub fn iqra<T: DhuMukhattat>(bayt: &[u8]) -> Natija<T> {
    let qeema: Value = serde_json::from_slice(bayt).map_err(|q| {
        Khata::min_tafsir(&KhataMukhattat::TaadhurTahleel {
            ism: T::ISM,
            satr: q.line(),
            amud: q.column(),
        })
    })?;
    min_qeema(qeema)
}

/// Reads a versioned structure from an already-parsed JSON value.
///
/// # Errors
///
/// As [`iqra`], minus the parse failure.
pub fn min_qeema<T: DhuMukhattat>(mut qeema: Value) -> Natija<T> {
    let Some(kain) = qeema.as_object() else {
        return Err(Khata::min_tafsir(&KhataMukhattat::LaysaKainan { ism: T::ISM }));
    };

    let Some(mawjud) = kain.get(HAQL).and_then(Value::as_u64) else {
        return Err(Khata::min_tafsir(&KhataMukhattat::IsdarMafqud { ism: T::ISM }));
    };
    let mut mawjud = u32::try_from(mawjud).unwrap_or(u32::MAX);

    if mawjud > T::ISDAR {
        return Err(Khata::min_tafsir(&KhataMukhattat::IsdarAhdath {
            ism: T::ISM,
            mawjud,
            madum: T::ISDAR,
        }));
    }

    while mawjud < T::ISDAR {
        qeema = T::hijra(mawjud, qeema)?;
        mawjud += 1;
        if let Some(kain) = qeema.as_object_mut() {
            let _ = kain.insert(HAQL.to_owned(), Value::from(mawjud));
        }
    }

    // Taken back out, symmetrically with `iktub` putting it in. The version is
    // this module's metadata and never a field of the structure, so leaving it
    // in the object would force every persisted type to either declare a field
    // it does not use or stay permanently tolerant of unknown keys — and that
    // tolerance is what lets a misspelt key take a whole subtree of somebody's
    // configuration down with it, silently.
    if let Some(kain) = qeema.as_object_mut() {
        let _ = kain.remove(HAQL);
    }

    serde_json::from_value(qeema).map_err(|q| {
        Khata::min_tafsir(&KhataMukhattat::TaadhurTahweel {
            ism: T::ISM,
            tafsil: q.to_string(),
        })
    })
}

/// Serializes a versioned structure, stamping it with the current version.
///
/// # Errors
///
/// Fails when the structure does not serialize to a JSON object, which means it
/// was declared as something other than a struct or a map and cannot carry a
/// version field.
pub fn iktub<T: DhuMukhattat>(qeema: &T) -> Natija<Vec<u8>> {
    let mut khaam = serde_json::to_value(qeema).map_err(|q| {
        Khata::min_tafsir(&KhataMukhattat::TaadhurTahweel {
            ism: T::ISM,
            tafsil: q.to_string(),
        })
    })?;

    let Some(kain) = khaam.as_object_mut() else {
        return Err(Khata::min_tafsir(&KhataMukhattat::LaysaKainan { ism: T::ISM }));
    };
    let _ = kain.insert(HAQL.to_owned(), Value::from(T::ISDAR));

    serde_json::to_vec_pretty(&khaam).map_err(|q| {
        Khata::min_tafsir(&KhataMukhattat::TaadhurTahweel {
            ism: T::ISM,
            tafsil: q.to_string(),
        })
    })
}

/// Reads a versioned structure from a file.
///
/// # Errors
///
/// As [`iqra`], plus any failure reading the file.
#[cfg(all(feature = "nizam", not(target_family = "wasm")))]
pub fn iqra_malaf<T: DhuMukhattat>(masar: &std::path::Path) -> Natija<T> {
    let bayt = crate::masarat::qira(masar)?;
    iqra::<T>(&bayt).map_err(|q| q.ma("masar", masar))
}

/// Atomically writes a versioned structure to a file.
///
/// # Errors
///
/// As [`iktub`], plus any failure writing the file.
#[cfg(all(feature = "nizam", not(target_family = "wasm")))]
pub fn iktub_malaf<T: DhuMukhattat>(masar: &std::path::Path, qeema: &T) -> Natija<()> {
    let bayt = iktub(qeema)?;
    crate::masarat::kitaba_dharra(masar, &bayt)
}

/// Moves a field to a new name during a migration, leaving the value untouched.
///
/// Renames are the most common migration by a wide margin, and doing them by
/// hand in every implementation is where migration bugs come from.
#[must_use]
pub fn ghayyir_ism(mut qeema: Value, min: &str, ila: &str) -> Value {
    if let Some(kain) = qeema.as_object_mut()
        && let Some(mawjud) = kain.remove(min)
    {
        let _ = kain.insert(ila.to_owned(), mawjud);
    }
    qeema
}

/// Adds a field with a default when it is absent, for a migration that
/// introduces a new setting.
#[must_use]
pub fn adif_iftiradi(mut qeema: Value, haql: &str, iftiradi: Value) -> Value {
    if let Some(kain) = qeema.as_object_mut()
        && !kain.contains_key(haql)
    {
        let _ = kain.insert(haql.to_owned(), iftiradi);
    }
    qeema
}

/// Removes a field a later version stopped carrying.
#[must_use]
pub fn ihdhif(mut qeema: Value, haql: &str) -> Value {
    if let Some(kain) = qeema.as_object_mut() {
        let _ = kain.remove(haql);
    }
    qeema
}

/// Failures of schema versioning.
#[derive(Debug, thiserror::Error)]
pub enum KhataMukhattat {
    /// The stored data was written by a newer build of Taarib.
    #[error("{ism} is schema {mawjud}, this build understands {madum}")]
    IsdarAhdath {
        /// The structure.
        ism: &'static str,
        /// The version found in the file.
        mawjud: u32,
        /// The version this build writes.
        madum: u32,
    },

    /// The stored data carries no version field at all.
    #[error("{ism} carries no schema version")]
    IsdarMafqud {
        /// The structure.
        ism: &'static str,
    },

    /// The stored data is not a JSON object and cannot carry a version.
    #[error("{ism} is not a JSON object")]
    LaysaKainan {
        /// The structure.
        ism: &'static str,
    },

    /// The stored data is not valid JSON.
    #[error("{ism} is not valid JSON at {satr}:{amud}")]
    TaadhurTahleel {
        /// The structure.
        ism: &'static str,
        /// Line of the parse failure.
        satr: usize,
        /// Column of the parse failure.
        amud: usize,
    },

    /// No migration is defined for a step in the chain.
    #[error("{ism} has no migration from schema {min}")]
    HijraNaqisa {
        /// The structure.
        ism: &'static str,
        /// The version with no way forward.
        min: u32,
    },

    /// The value does not fit the structure after migration.
    #[error("{ism} does not match its schema: {tafsil}")]
    TaadhurTahweel {
        /// The structure.
        ism: &'static str,
        /// What serde reported, kept as context rather than as the message.
        tafsil: String,
    },
}

impl Tafsir for KhataMukhattat {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUKHATTAT
                + match self {
                    Self::IsdarAhdath { .. } => 0,
                    Self::IsdarMafqud { .. } => 1,
                    Self::LaysaKainan { .. } => 2,
                    Self::TaadhurTahleel { .. } => 3,
                    Self::HijraNaqisa { .. } => 4,
                    Self::TaadhurTahweel { .. } => 5,
                },
        )
    }

    fn arabi(&self) -> String {
        match self {
            Self::IsdarAhdath { .. } => {
                "هذه البيانات كُتبت بإصدار أحدث من تعريب. حدِّث البرنامج لفتحها؛ فتحها بهذا \
                 الإصدار قد يفقد جزءًا منها."
                    .to_owned()
            }
            Self::IsdarMafqud { .. } | Self::LaysaKainan { .. } => {
                "ملف بيانات تالف: لا يحمل رقم مخطّطه. لن يُقرأ لتفادي إتلاف ما فيه.".to_owned()
            }
            Self::TaadhurTahleel { satr, amud, .. } => {
                format!("ملف بيانات تالف عند السطر {satr} والعمود {amud}.")
            }
            Self::HijraNaqisa { min, .. } => {
                format!("لا توجد ترقية من المخطّط رقم {min}؛ هذه البيانات أقدم مما يقرؤه تعريب.")
            }
            Self::TaadhurTahweel { .. } => {
                "ملف بيانات لا يطابق شكله المتوقّع بعد الترقية.".to_owned()
            }
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::IsdarAhdath { mawjud, madum, .. } => format!(
                "This data was written by a newer Taarib (schema {mawjud}; this build reads \
                 {madum}). Update Taarib to open it — opening it here would lose part of it."
            ),
            Self::IsdarMafqud { .. } | Self::LaysaKainan { .. } => {
                "Corrupt data file: it carries no schema version, so it will not be read."
                    .to_owned()
            }
            Self::TaadhurTahleel { satr, amud, .. } => {
                format!("Corrupt data file at line {satr}, column {amud}.")
            }
            Self::HijraNaqisa { min, .. } => {
                format!("No migration from schema {min}; this data predates what Taarib reads.")
            }
            Self::TaadhurTahweel { .. } => {
                "Data file does not match its expected shape after migration.".to_owned()
            }
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::IsdarAhdath { .. } => Khutwa::TahdithTaarib,
            _ => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        let ism = match self {
            Self::IsdarAhdath { ism, mawjud, madum } => {
                let _ = siyaq.insert("mawjud".to_owned(), QeemaSiyaq::Raqm(i64::from(*mawjud)));
                let _ = siyaq.insert("madum".to_owned(), QeemaSiyaq::Raqm(i64::from(*madum)));
                *ism
            }
            Self::IsdarMafqud { ism } | Self::LaysaKainan { ism } => *ism,
            Self::TaadhurTahleel { ism, satr, amud } => {
                let _ = siyaq.insert("satr".to_owned(), QeemaSiyaq::Hajm(*satr as u64));
                let _ = siyaq.insert("amud".to_owned(), QeemaSiyaq::Hajm(*amud as u64));
                *ism
            }
            Self::HijraNaqisa { ism, min } => {
                let _ = siyaq.insert("min".to_owned(), QeemaSiyaq::Raqm(i64::from(*min)));
                *ism
            }
            Self::TaadhurTahweel { ism, tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
                *ism
            }
        };
        let _ = siyaq.insert("mabna".to_owned(), QeemaSiyaq::Nass(ism.to_owned()));
        siyaq
    }
}

khata_min!(KhataMukhattat);
