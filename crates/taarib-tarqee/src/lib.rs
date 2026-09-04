//! # ترقيع تعريب — the patch compiler
//!
//! A reviewed translation becomes a compact, signed `.ruqaa` that a game can
//! consume with zero layout work at runtime. Everything expensive — shaping,
//! bidi, line breaking, justification, rasterization, packing — happens here,
//! once, on a desktop, so that none of it happens in a game loop.
//!
//! Built in **Phase 14** alongside `taarib-ruqaa`, on Phases 1, 2, 12 and 13.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `tahdid_maqasat` | size discovery: every text size the game actually draws at, learned from the capability probe, from runtime capture, and from the adapter's reported font sizes |
//! | `takhtit` | precompilation — every translated string run through `saff` at every discovered size, inside its real available width, with its real style spans, and the positioned glyphs stored |
//! | `bawwaba` | the asset gate, which proves before a package is written that it contains no original game asset |
//! | `fuhusat` | the four hard checks, and the proof that they ran which the assembler will not build without |
//! | `taqrir_tajawuz` | the overflow report: measured width, available width, overflow in pixels and percent, the size measured at, and the interface element, sorted by severity |
//! | `taghtiya_ruqaa` | coverage: raw, duplicate-collapsed, per interface element and weighted, with the provenance of every weight |
//! | `irtibat` | binding — the launcher identifiers, build ids and `Basma` fingerprints the patch declares itself compatible with |
//! | `tahweel` | the project's vocabulary translated into the container's fixed-layout records, and every deliberate loss written down in one place |
//! | `bayan` | the manifest: what a package says about itself, serialized and never deserialized |
//! | `mujammi` | the assembler, the only function here that produces a file, and the round trip it verifies its own output through |
//! | `mustawrid` | importers: XUnity.AutoTranslator plain text, XLIFF 1.2 and 2.0, gettext PO/POT, TMX, CSV with configurable columns, and Unity Localization's own CSV export |
//! | `khata` | every refusal the compiler can produce, in band 6100 |
//!
//! Atlas compilation is not a module here. The exact glyph set falls out of the
//! layouts `takhtit` produced and is handed to `taarib-lawha` to pack in the
//! mode the patch declares, which is the only correct order: a glyph set
//! collected any earlier would be a guess about what shaping was going to do.
//!
//! ## The runtime path exists only for what the compiler could not see
//!
//! A shipped patch that shapes at runtime for text the compiler already knew
//! about is a compiler bug, not a performance characteristic. Player names,
//! procedurally composed sentences and runtime-captured strings are the only
//! things `namu` should ever be asked to grow the atlas for, and the growth
//! counters in the diagnostics exist precisely so that a patch which is
//! constantly growing gets noticed and recompiled.
//!
//! ## The asset gate is a proof, not a warning
//!
//! Before a package is written, `bawwaba` traces every byte in it to one of
//! four origins: translated text, layout computed from that text, glyphs
//! rasterized from a bundled or user-supplied font whose hash is in
//! `khutut.json`, or metadata the contributor authored. Any input that cannot
//! be traced fails the compile, and the failure names the offending source.
//!
//! There is no acknowledgement, no override and no "compile anyway". Decision 5
//! is what lets Taarib's rendering path work on engine versions that do not
//! exist yet, and it is also what keeps the catalogue distributable — a patch
//! containing a game's own asset is not a patch, it is redistribution of
//! somebody else's work, and the gate makes that mechanically impossible rather
//! than a matter of contributor good faith. Phase 18 re-runs the same gate at
//! submission, against the compiled bytes, for the same reason.
//!
//! ## The overflow report ships inside the patch
//!
//! Arabic is frequently longer than English at the same font size, and a menu
//! whose labels overrun their buttons is what "an amateur translation" looks
//! like in a screenshot. Every string is measured through `saff` against the
//! real constraint recorded by extraction, and the result — sorted by severity,
//! each entry jumping to its own live preview — is shown to the contributor
//! before submission and to the reviewer during review, and stays in the patch
//! metadata afterwards. The only way to prevent overflow is to measure it, and
//! the only honest measurement is the one the game will reproduce.
//!
//! ## Hard constraints
//!
//! - Layouts are precomputed for every known string at every discovered size.
//! - No original game asset in the package, enforced by the gate.
//! - Compilation is deterministic: the same project produces a byte-identical
//!   package, so a rebuild does not invalidate mirrors or look like tampering.
//! - Every import maps onto the extracted string table with matched and
//!   unmatched entries reported and resolvable. An import that silently drops
//!   what it could not match is an import that loses somebody's work.
//! - The package is signed before it leaves the compiler — self-signed by the
//!   contributor's key, which is what Phase 18's pre-flight gate verifies. The
//!   owner's signature is added at approval and never here. The *key* is not
//!   here either: [`mujammi::HuzmaMabniya::akhtim`] takes a block that has
//!   already been produced, and `taarib-khatm` is deliberately not a dependency
//!   of this crate. See the note in its manifest.
//!
//! ## Two things this crate makes unrepresentable rather than checks
//!
//! Both are the same construction — a token with no public constructor, no
//! public fields, no `Deserialize` and no [`Clone`], minted by exactly one
//! function and consumed by exactly one other — and both exist because the
//! alternative is an `if` that a future edit deletes without leaving a trace.
//!
//! [`bawwaba::IthbatTawlid`] is minted only by [`bawwaba::rassim`], which
//! rasterizes glyph keys with fonts that passed a canonicalized containment
//! test against Taarib's own font directory. Bytes reach a package only inside
//! [`bawwaba::MuhtawaMasmuh`], which has no variant holding an unattributed
//! `Vec<u8>`. There is no parameter anywhere on that path through which a byte
//! from a game directory could arrive.
//!
//! [`fuhusat::IjtiyazFuhus`] is minted only by [`fuhusat::ijri`], which returns
//! it if and only if all four hard checks passed, and it is consumed by
//! [`mujammi::ijmaa`], which is the only function in this crate that writes a
//! file. A package produced past a failing check is not a bug to be avoided; it
//! is a program that does not compile.

pub mod bawwaba;
pub mod bayan;
pub mod fuhusat;
pub mod irtibat;
pub mod khata;
pub mod mujammi;
pub mod mustawrid;
pub mod taghtiya_ruqaa;
pub mod tahdid_maqasat;
pub mod tahweel;
pub mod takhtit;
pub mod taqrir_tajawuz;

pub use crate::bawwaba::{
    FarqHawiya, IthbatTawlid, KhattMujammaa, MuhtawaMasmuh, SafhatMasmuha, ShahadatBawwaba,
    rassim,
};
pub use crate::bayan::{BayanHuzma, MUKHATTAT_BAYAN, MuharrikHuzma, SijillFuhus};
pub use crate::fuhusat::{FashalTakhtit, IjtiyazFuhus, MudkhalatFahs, WasfHuzma, ijri};
pub use crate::irtibat::{
    BasmatKhatt, HukmIrtibat, IrtibatBina, MukhattatBasma, NitaqBina, SababMutabaqa,
};
pub use crate::khata::{KhataTarqee, SababMasar};
pub use crate::mujammi::{HuzmaMabniya, MudkhalatTajmee, ijmaa};
pub use crate::tahdid_maqasat::{HajmMuqannan, IktishafMaqasat, TaqreerMaqasat};
pub use crate::tahweel::SiyasatHuzma;
pub use crate::takhtit::{KhiyaratTasbeeq, MizaniyatIqama, SiyasatNamu, TakhtitMusbaq, sabbiq};
pub use crate::taqrir_tajawuz::{BaniTaqrirTajawuz, MudkhalQiyas, ShiddatTajawuz, TaqrirTajawuz};
pub use crate::taghtiya_ruqaa::{TaqrirTaghtiya, ihsib_taghtiya};
