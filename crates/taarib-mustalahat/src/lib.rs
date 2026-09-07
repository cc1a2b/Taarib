//! # مصطلحات تعريب — the shared vocabulary
//!
//! The concepts every part of Taarib talks about, defined exactly once:
//!
//! | module | concept |
//! | --- | --- |
//! | [`luba`] | a game, and the launchers that know it |
//! | [`muharrik`] | an engine, its backend, and what Taarib can do to it |
//! | [`bina`] | a build, and the content fingerprint that outlives it |
//! | [`ruqaa`] | a patch, its lineage, its method, and its licence |
//! | [`nass`] | a string, its context, its constraints, and its status |
//! | [`taghtiya`] | how much of a game is actually translated |
//! | [`musahim`] | a contributor and their standing |
//! | [`sawt`] | a voice pack: a registry artifact of its own kind, bound and signed exactly as a patch is, and installed and removed independently of one |
//! | [`ghiyab`] | why a game somebody owns is not in their library, in words they can act on |
//! | [`lawha_badila`] | the card a game with no artwork gets — a designed state, not an error one |
//! | [`badaa`] | the five situations behind one "translate this game" menu entry, and the honest verdict shown before any work starts |
//!
//! Each type derives `Serialize`, `Deserialize`, and — under the `wajiha` and
//! `mukhattatat` features — `specta::Type` and `schemars::JsonSchema`. One
//! definition therefore produces the Rust type, the TypeScript type the Studio
//! frontend imports, and the JSON Schema the registry validates against. There
//! is no second definition of any of these anywhere in the product, and no
//! hand-written TypeScript interface describing them.

pub mod badaa;
pub mod bina;
pub mod ghiyab;
pub mod lawha_badila;
pub mod luba;
pub mod muharrik;
pub mod muraja;
pub mod musahim;
pub mod nass;
pub mod ruqaa;
pub mod sawt;
pub mod taghtiya;

pub use bina::{Basma, BinaId, MutabaqaBina};
pub use luba::{HalatLuba, LawnBariz, Luba, LubaId, MasdarLuba, SuwarLuba};
pub use muharrik::{
    AilatMuharrik, Daleel, Hadd, IsdarMuharrik, ItarNusus, JawdaMutawaqqaa, KhalfiyaBarmajiya,
    Muharrik, Tabaqa, TaqreerImkaniyat, WajihaRusum,
};
pub use musahim::{Musahim, MusahimId, Sumaa};
pub use nass::{
    AlamJawda, MudkhalNass, Mustatil, NassId, NawNasq, NitaqNasq, QuyudNass, SiyaqNass,
};
pub use ruqaa::{HalatRuqaa, MulakhkhasRuqaa, RukhsaRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama};
pub use taghtiya::Taghtiya;

/// The namespace every deterministic Taarib identifier is derived under.
///
/// Fixed forever: changing it would give every game on every machine a new
/// identity and orphan every patch, every project, and every backup manifest
/// that referenced the old one.
pub const NITAQ_TAARIB: uuid::Uuid = uuid::uuid!("7a1b4ec2-0f3d-5a97-9c6e-2b8d41f0a3c5");

/// Normalizes a game or product name so that the same game discovered through
/// two launchers, spelled two ways, resolves to one identity.
///
/// Lowercases, drops everything that is not a letter or a digit, and collapses
/// the result. `"Hollow Knight™"`, `"hollow knight"` and `"Hollow  Knight"` all
/// become `"hollowknight"`, while `"Hollow Knight: Silksong"` stays distinct.
#[must_use]
pub fn wahhid_ism(ism: &str) -> String {
    ism.chars()
        .filter(|h| h.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
