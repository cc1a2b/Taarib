//! المتاجر — one submodule per source, all answering the same question.
//!
//! Eighteen adapters, one [`Matjar`] trait, and nothing else in the product
//! knows which of them a game came from. That is the whole point of the shape:
//! the library grid, the engine probe, the installer and the registry client all
//! work on [`LubaMuktashafa`] and then on `Luba`, and adding a nineteenth source
//! means adding a file here and a line in [`kul`].
//!
//! What the adapters have in common is more interesting than what differs. Each
//! one reads a format somebody else designed and did not document, on a machine
//! where that launcher may be half-installed, mid-update, or pointing at a drive
//! that is not plugged in. So each one is written to the same three rules:
//!
//! 1. **Read-only, always.** Nothing here writes a file, edits a launcher's
//!    configuration, or opens another product's live database for writing. The
//!    three adapters that read a running application's `SQLite` file — GOG's
//!    Galaxy, itch's butler and Lutris's `pga.db` — open it read-only with
//!    immutable flags for exactly that reason.
//! 2. **Never require the launcher to be running**, and never start one. A
//!    scanner that launched Steam to enumerate games is a scanner nobody leaves
//!    enabled.
//! 3. **Degrade one entry, never the scan.** A manifest that will not parse is a
//!    [`TanbihFahs`](crate::fahs::TanbihFahs) on the result and the other two
//!    hundred games still arrive. A *source* that will not read — the catalogue
//!    directory itself, one library, one drive, one nominated folder — is a
//!    [`TanbihFahs::fahras`](crate::fahs::TanbihFahs::fahras), which marks the
//!    result [`Naqisa`](crate::fahs::HalatFahsMatjar::Naqisa) so that nothing
//!    downstream mistakes what the scan did not see for what is not there. And
//!    a root that is present with no catalogue under it is that — installed,
//!    unreadable — never "not installed": [`Matjar::mawqi`] and
//!    [`Matjar::ifhas`] must agree about whether a launcher is here.
//!
//! ## Which of these is a real identity
//!
//! Six adapters deliberately do not mint an identity of their own.
//!
//! [`heroic`] wraps: a game installed through Heroic is an Epic or a GOG game,
//! and `MasdarLuba::Heroic(Box::new(..))` unwraps through `aila()` and
//! `muarrif()` so that its identity is byte-identical to the one the Windows
//! adapter for that store would derive. A patch published against an Epic app
//! name is therefore offered, unchanged, to a Linux user who installed the game
//! through Heroic. Heroic is how the game is installed, not what it is.
//!
//! [`steam`] reports non-Steam shortcuts as `MasdarLuba::Yadawi`, because a
//! shortcut's app id is a checksum generated on one machine and indexes nothing
//! in Steam's catalogue — the same game added on two computers gets two
//! different numbers. Calling it a Steam identity would shard the registry under
//! a number that means something different on every machine.
//!
//! [`playnite`] wraps when it can and admits when it cannot:
//! `MasdarLuba::Playnite` carries the store behind an entry where the record
//! names one and `None` where it does not, which is most of them — a Playnite
//! library is full of manually added and emulated entries with no store identity
//! to resolve to. It is also the one adapter here that cannot read its
//! launcher's game list at all, and says so on every scan; the reasoning is the
//! whole of that module's header.
//!
//! [`legendary`] wraps unconditionally, because there is only one thing behind
//! it: legendary is a client for Epic and nothing else, so every entry is
//! `MasdarLuba::Legendary(Box::new(Epic(..)))` and unwraps to the identity the
//! Epic adapter would derive on Windows.
//!
//! [`lutris`] wraps when its `service` column names a store and admits when it
//! does not. A Lutris entry installed from GOG carries `Gog(1207658691)` inside
//! `MasdarLutris::asl` and shards under `gog`; a hand-written install script, a
//! disc image and an emulated cartridge carry `None` and shard under `lutris`,
//! because they genuinely have no upstream identity and inventing one would key
//! a patch against a store identifier the game does not have. The Lutris slug is
//! kept either way — it is the only way the game can be launched.
//!
//! [`bottles`] is the exception that proves the rule: it mints an identity of
//! its own, `bottles:<bottle>/<program>`, because a bottle is a Wine prefix
//! rather than a store and there is nothing behind it to unwrap to. Both halves
//! are required, and that module's header argues why each of them alone would
//! collide.
//!
//! [`mahmul`] mints the same `MasdarLuba::Yadawi` path hash [`yadawi`] does, by
//! calling that module's own function rather than repeating it — a game the
//! scanner finds under a nominated folder and the same game added by hand
//! through Settings are one library row. Everything it produces is marked
//! `SimatLuba::MuktashafaBilIstidlal`, because nothing vouched for it.

pub mod amazon;
pub mod battlenet;
pub mod bottles;
pub mod ea;
pub mod epic;
pub mod gog;
pub mod heroic;
pub mod itch;
pub mod legendary;
pub mod lutris;
pub mod mahmul;
pub mod playnite;
pub mod riot;
pub mod rockstar;
pub mod steam;
pub mod ubisoft;
pub mod vdf;
pub mod xbox;
pub mod yadawi;

use crate::fahs::Matjar;

pub use crate::matajir::amazon::MatjarAmazon;
pub use crate::matajir::battlenet::MatjarBattleNet;
pub use crate::matajir::bottles::MatjarBottles;
pub use crate::matajir::ea::MatjarEa;
pub use crate::matajir::epic::MatjarEpic;
pub use crate::matajir::gog::MatjarGog;
pub use crate::matajir::heroic::MatjarHeroic;
pub use crate::matajir::itch::MatjarItch;
pub use crate::matajir::legendary::MatjarLegendary;
pub use crate::matajir::lutris::MatjarLutris;
pub use crate::matajir::mahmul::MatjarMahmul;
pub use crate::matajir::playnite::MatjarPlaynite;
pub use crate::matajir::riot::MatjarRiot;
pub use crate::matajir::rockstar::MatjarRockstar;
pub use crate::matajir::steam::MatjarSteam;
pub use crate::matajir::ubisoft::MatjarUbisoft;
pub use crate::matajir::xbox::MatjarXbox;
pub use crate::matajir::yadawi::MatjarYadawi;

/// Every adapter, in the order a scan runs them.
///
/// Steam first because it is the source most users have and the one whose
/// result the interface can show soonest; the two path-based adapters last
/// because they are the fallback that catches whatever the launchers did not,
/// and running them last lets them see that a path has already been claimed.
///
/// Order is otherwise not significant: identities are derived from each
/// launcher's own catalogue, so no adapter's answer depends on another having
/// run.
///
/// [`MatjarMahmul`] appears here with no claimed paths and no extra roots,
/// which is the honest default: it scans the folders in the user's settings and
/// nothing else. A caller that wants it to *deduplicate* against the launchers —
/// so that nominating a Steam library folder does not produce a second copy of
/// every Steam game — runs the launcher adapters first and builds it with
/// [`MatjarMahmul::mahjuza_min_fahs`], through [`crate::Kashif::min_matajir`].
/// The claim set is a fact only a completed scan produces, so it cannot come
/// from a constructor that runs before one.
///
/// [`MatjarPlaynite`] likewise takes its portable data roots from the caller.
/// A portable Playnite lives wherever the user put the folder and no probe can
/// enumerate that, so [`MatjarPlaynite::bi_judhur`] is the seam and this
/// default only checks the conventional locations.
#[must_use]
pub fn kul() -> Vec<Box<dyn Matjar>> {
    vec![
        // A unit struct, unlike every other adapter here: everything Steam
        // needs is
        // resolved per scan from `SiyaqFahs`, so it carries no state worth a
        // constructor.
        Box::new(MatjarSteam),
        Box::new(MatjarEpic::jadeed()),
        Box::new(MatjarGog::jadeed()),
        Box::new(MatjarHeroic::jadeed()),
        Box::new(MatjarEa::jadeed()),
        Box::new(MatjarUbisoft::jadeed()),
        Box::new(MatjarBattleNet::jadeed()),
        Box::new(MatjarXbox::jadeed()),
        Box::new(MatjarItch::jadeed()),
        Box::new(MatjarAmazon::jadeed()),
        Box::new(MatjarRockstar::jadeed()),
        Box::new(MatjarRiot::jadeed()),
        Box::new(MatjarLutris::jadeed()),
        Box::new(MatjarBottles::jadeed()),
        Box::new(MatjarLegendary::jadeed()),
        Box::new(MatjarPlaynite::jadeed()),
        Box::new(MatjarYadawi::jadeed()),
        Box::new(MatjarMahmul::jadeed()),
    ]
}
