//! # ترجمة تعريب — from a string table to reviewed Arabic
//!
//! Machine assistance that understands it is translating a game: that a button
//! label is not a sentence, that a line of dialogue has a speaker and lines
//! before it, that `{0}` must come back exactly as it went out, and that a
//! string which does not fit its box is not translated no matter how good the
//! Arabic is.
//!
//! Built in **Phase 13**, on Phases 1 and 12. It is also, quietly, one of the
//! reasons `saff` is a standalone library: every width in this crate is
//! measured by shaping the real Arabic in the real font at the real size, and
//! an overflow warning computed any other way would be a guess presented as a
//! fact.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `muzawwidun` | providers behind one trait — Anthropic, `OpenAI` and any OpenAI-compatible endpoint, Google Gemini, `DeepL`, Google and Microsoft translation, local models through an Ollama-compatible endpoint, and Google Translate's free web endpoint, which needs no key and is what a fresh installation translates with — each with its own model list, pricing table, rate limits and context window |
//! | `siyaq` | game-appropriate prompting: name, genre, speaker, surrounding lines, interface context, the character or pixel constraint, the applicable glossary entries, the declared tone, and the placeholder atoms that must survive |
//! | `hima` | placeholder and markup protection in three enforced steps — extract to opaque atoms, present them in a form the model will not translate, and validate every returned string |
//! | `dhakira` | translation memory: source-target pairs with fuzzy matching, scoring and provenance, per project and across projects |
//! | `mulahazat` | the overlay's own accumulation: readings recorded per game, a session map in front of the memory, and the counter that says how many translations were actually paid for |
//! | `masrad` | the glossary: canonical Arabic for names, places, items, abilities and systems, with part of speech, gender, plural, a do-not-translate flag, and notes |
//! | `dufaat` | batching: bounded concurrency per provider, token-bucket rate limiting, cost accumulated against a budget with a hard stop, checkpointed resume, and per-string retry with backoff |
//! | `alamat` | quality flags, computed after every change, each one actionable and each one linking to the strings that caused it |
//! | `muraja_dakhiliya` | the in-project review workflow: per-string status, threaded comments, and a full change history with author, timestamp and previous value |
//!
//! ## `hima` is the load-bearing part
//!
//! A translation that reads beautifully and dropped a `%1$d` is a crash or a
//! blank in someone's save file. So placeholders never leave the machine as
//! placeholders: they are extracted into atoms before the request is built,
//! presented in a form models reliably leave alone, and checked on return —
//! every atom present, none duplicated, none reordered in a way that breaks a
//! positional format specifier, no stray tag introduced by a helpful model.
//!
//! A failed validation is a rejection, not a warning. The request is retried
//! with a corrective instruction, and after a bounded number of retries the
//! string is flagged for human attention rather than accepted. There is no path
//! by which a string with broken atoms enters a project.
//!
//! ## Memory is shared across games by default
//!
//! Item names, interface verbs, "Save", "Load", "Are you sure?", "Press any
//! key" — these repeat endlessly across titles, and inconsistency between them
//! is exactly what makes a body of community translations feel amateur. So
//! `dhakira` is cross-project by default. Reuse is always *proposed*, never
//! applied silently: a match at 92% similarity is a suggestion with its score
//! and its provenance shown, and a human decides.
//!
//! ## Quality flags are measurements
//!
//! Overflow risk comes from `saff` against the string's real pixel constraint.
//! Untranslated leftovers are Latin text found inside Arabic output.
//! Terminology inconsistency is checked against both the glossary and the
//! project's own prior choices. Broken or missing placeholders come from
//! `hima`. Machine confidence below threshold, and suspicious length ratios in
//! both directions, round it out. Every flag links to the strings that produced
//! it, because a flag a translator cannot act on is decoration.
//!
//! ## Hard constraints
//!
//! - Credentials live in the OS keychain. No credential ever appears in a
//!   configuration file, a log, an error, a crash report, or a diagnostics
//!   bundle. The one provider that needs none — Google Translate's free web
//!   endpoint — is unofficial and says so in every sentence it produces:
//!   sentence-level quality, undocumented limits, and a keyed provider takes
//!   precedence the moment one is configured.
//! - No translation with a broken placeholder is ever accepted into a project.
//! - Every machine translation is marked as machine-produced and carries its
//!   provider, model and prompt version until a human confirms it. Prompts are
//!   versioned so a result can still be explained a year later.
//! - Cost is computed before a request is sent, and the budget stop is enforced
//!   client-side rather than discovered from a provider's bill.
//! - An interrupted batch resumes exactly where it stopped. Nothing is paid for
//!   twice because a laptop closed.

pub mod alamat;
pub mod dhakira;
pub mod dufaat;
pub mod hima;
pub mod khata;
pub mod masrad;
pub mod mulahazat;
pub mod muraja_dakhiliya;
pub mod muzawwidun;
pub mod siyaq;

pub use crate::hima::{NassMahmi, NassMustaad, ihmi, istaridd};
pub use crate::khata::KhataTarjama;
pub use crate::muraja_dakhiliya::{HalatMuraja, ShahadatMuraja, SijillMuraja};
