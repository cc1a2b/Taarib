//! # تلقائي تعريب — the automatic arabization pipeline
//!
//! A player owns a game with no community patch. They press one button and, a
//! few minutes later, they are playing it in Arabic. Everything here serves
//! that sentence, and nothing here is a new capability: every stage already
//! existed and was already proven to chain. What this crate is, is the machine
//! that drives them — with real progress, a resume that does not re-buy work
//! already paid for, a cancel that leaves the game exactly as it was, and a
//! refusal that routes to a path that works instead of a dead end.
//!
//! ## The seven stages
//!
//! | # | stage | crate | what it produces |
//! | --- | --- | --- | --- |
//! | 1 | [`fahs`] | `taarib-muharrik` | the engine, the tier, the confidence, the limits |
//! | 2 | [`istikhraj`] | `taarib-istikhraj` | the string table **and** the refusal report |
//! | 3 | [`tarjama`] | `taarib-tarjama` | Arabic, every reply through the placeholder guard |
//! | 4 | [`bina::hayyi`] | `taarib-saff` | validated fonts and every size to lay out at |
//! | 5 | [`bina::ijmi`] | `taarib-tarqee`, `taarib-lawha` | layouts, the atlas, the container |
//! | 6 | [`bina::ijmi`] | `taarib-khatm` | the seal |
//! | 7 | [`tathbeet`] | `taarib-aman`, `taarib-tathbeet` | the permit, then the install |
//!
//! ## Four properties, and where each one lives
//!
//! **Progress is real.** Every stage reports what it is doing and how far
//! through it is, against a denominator it actually knows: strings translated
//! of how many, fonts validated of how many, files placed of how many. There is
//! no timer anywhere in this crate. [`taqaddum::Taqaddum::majmu`] is an
//! [`Option`] and exactly two callers pass [`None`] — the static extraction
//! pass, whose container count is not knowable until the walk that *is* the
//! work has happened, and the compiler's inner layout pass, whose glyph set is
//! whatever shaping turns out to produce. Both say so in the report's own
//! words. Everywhere else a denominator exists and is used. Progress leaves the
//! crate through [`taqaddum::MukhbirTaqaddum`], a three-variant enum the caller
//! picks — silent, a channel, or a callback — which is the shape
//! `taarib_mustawda::tanzeel::nazzil` established.
//!
//! **Resuming is not a mode.** Pass the same [`mashwar::MashwarId`] and the
//! same runs root. Every stage the journal closed is skipped; the translation
//! stage is always *entered*, because `taarib_tarjama`'s own per-string journal
//! is the thing that knows which strings were paid for. A user who closes the
//! application half way through four thousand strings and reopens it continues,
//! and does not buy one string twice. [`mashwar`] documents where the journal
//! lives and argues for why it is a file in the run directory rather than a row
//! in the store.
//!
//! **Cancelling is free until the install, and reversible after it.**
//! [`ilgha::MiqbadIlgha`] is checked at every stage boundary, at every font, and
//! between translation replies — where it does not poll but abandons the run's
//! future outright, so four requests in flight stop in milliseconds rather than
//! in ninety seconds. Everything before stage 7 writes only inside the run
//! directory. Stage 7 writes into the game through `taarib_tathbeet`'s
//! recording guard, and a cancellation arriving during it is honoured by
//! letting the install *finish* and then restoring from the manifest it just
//! wrote — because a half-written install is the one outcome worse than either.
//!
//! **Failure is honest.** A game whose containers are all type-tree-stripped or
//! whose text the game itself encrypts produces no strings, and this crate does
//! not report that as "no text found". It reports
//! [`khata::KhataTilqai::YahtajIltiqat`]: *run the game once with capture
//! enabled, then come back*. That is a real path — the session file comes back
//! in through [`talab::TalabTilqai::jalsat_iltiqat`] and is merged into the
//! table by `taarib_istikhraj::dammij` — and it is offered rather than
//! described.
//!
//! The same honesty runs one stage earlier, and it is the one refusal that is
//! about Taarib rather than about the game. An engine whose in-game half this
//! build has not finished is refused by [`fahs::tahaqquq_jahiziya`] before a
//! container is opened or a cent is spent, because the alternative is a run
//! that succeeds at every stage, compiles a valid signed patch, installs it,
//! reports success — and changes nothing the player sees. That gate reads
//! `TaqreerImkaniyat::jahiziya` and nothing else, so it opens by itself for
//! each engine on the day that engine's adapter is finished.
//!
//! ## Cost
//!
//! Translation is the only stage that spends money. The ceiling is
//! [`talab::KhiyaratTilqai::saqf_takalif`], in nano-dollars, enforced by
//! `taarib_tarjama::dufaat` by reserving before dispatch — so the request that
//! *would* cross the ceiling is never sent — and cumulative across resumes,
//! because the journal's recorded spend seeds the ledger. Every progress report
//! carries the spend and the ceiling. A user running a local model through
//! `MuzawwidMuwafiqOpenAI::mahalli` pays nothing and completes the whole flow,
//! and that path is not a degraded one: it is the same code with a free meter.
//!
//! ## What this crate never does
//!
//! It does not read a clock — every timestamp is
//! [`talab::KhiyaratTilqai::lahza`] and `waqt`, supplied by the caller, so a
//! journal is reproducible. It does not mint a revocation list, an
//! acknowledgement record or a trust anchor; those arrive in
//! [`talab::MudkhalatAman`] from the only party that can have obtained them
//! honestly. It does not know the interface exists. And it never labels its
//! output as anything but what it is: every patch it compiles declares
//! `TareeqaTarjama::AaliyaFaqat` — machine translation, with no human review.

pub mod bina;
pub mod fahs;
pub mod ilgha;
pub mod istikhraj;
pub mod khata;
pub mod mashwar;
pub mod natija;
pub mod talab;
pub mod tanfidh;
pub mod taqaddum;
pub mod taqreer;
pub mod tarjama;
pub mod tathbeet;

pub use crate::bina::{MasaratBina, MilliBina, Tahdeer};
pub use crate::fahs::{naqs_jahiziya, tahaqquq_jahiziya};
pub use crate::ilgha::MiqbadIlgha;
pub use crate::istikhraj::JadwalMakhzun;
pub use crate::khata::{KhataTilqai, NatijatTilqai};
pub use crate::mashwar::{
    ISDAR_SIJILL, MashwarId, MashwarMawjuz, QaydMarhala, SijillMashwar, TarwisatMashwar, ijrud,
};
pub use crate::natija::NatijatMashwar;
pub use crate::talab::{KhiyaratTilqai, LubaTilqai, MudkhalatAman, TalabTilqai, WasfTilqai};
pub use crate::tanfidh::arrib;
pub use crate::taqaddum::{MarhalaTilqai, MukhbirTaqaddum, Muraqib, Taqaddum};
pub use crate::taqreer::{
    HalatMashwar, IhsaHuzma, IhsaIstikhraj, IhsaTarjama, IhsaTathbeet, TaqreerMarhala,
    TaqreerMashwar,
};
