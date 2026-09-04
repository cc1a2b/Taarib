//! المراجعة الداخلية — the in-project review workflow.
//!
//! The review *vocabulary* — the states, the attestation, the history — lives in
//! [`taarib_mustalahat::muraja`], because `MudkhalNass` carries it and every
//! phase from extraction to publication reads it. Defining it here would have
//! meant the project's own entry type depending on the translation crate to
//! describe its own status.
//!
//! This module is the re-export downstream code imports, and the place the
//! in-project workflow that is genuinely this crate's — comment threads, bulk
//! transitions — attaches to.
//!
//! The guarantee the vocabulary carries is restated here because it is the one
//! thing a reader of this crate must not miss: **a machine translation cannot
//! reach [`HalatMuraja::Muakkada`]**. Approval consumes a [`ShahadatMuraja`],
//! which has no public constructor and no `Deserialize`, so there is no
//! sequence of calls and no stored document that approves text nobody read.

pub use taarib_mustalahat::muraja::{
    HalatMuraja, IntiqalMuraja, ShahadatMuraja, SijillMuraja,
};
