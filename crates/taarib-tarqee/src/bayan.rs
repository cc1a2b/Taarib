//! البيان — what a package says about itself.
//!
//! The `BAYAN` section, as JSON. Everything a reviewer, an installer, the
//! registry and a future build of Taarib need in order to decide what to do
//! with a `.ruqaa` without decompressing a single table.
//!
//! ## Why this is serialized and never deserialized
//!
//! [`BayanHuzma`] derives [`Serialize`] and not `Deserialize`, and that is not
//! an oversight to be corrected the first time somebody wants to read a patch's
//! metadata back.
//!
//! Two reasons, and the first is the smaller one: `taarib_ruqaa::qari` already
//! hands the section back as a [`serde_json::Value`], on purpose, so that a
//! patch built by a newer Taarib carrying fields this build has never heard of
//! is *readable* rather than a parse failure. A typed round trip would turn
//! every added field into a compatibility break for every older client.
//!
//! The second is the one that matters. Several of the reports in here —
//! [`crate::tahdid_maqasat::TaqreerMaqasat`] above all — hold types whose
//! constructors enforce an invariant, and a `Deserialize` impl is a public
//! constructor. A build that could parse a manifest back into these types could
//! be handed a document asserting a size the rasterizer never accepted, or a
//! gate certificate for content nobody generated. Reading the section as data
//! and reading it as *this product's own conclusions* are different acts, and
//! only the first one is safe to perform on a file that arrived over a network.
//!
//! ## There is no build timestamp here
//!
//! `taarib_ruqaa::katib` guarantees that the same project compiled twice
//! produces byte-identical output, and every field in this manifest is chosen
//! to keep that true: no time, no path, no machine name, no username, nothing
//! ordered by a hash map.
//!
//! A patch's publication time is the registry's record, added when it is
//! published, and it lives in `MulakhkhasRuqaa::waqt_nashr`. Putting a build
//! time in the container instead would mean two compiles of one unchanged
//! project hash differently — which makes a rebuild indistinguishable from
//! tampering, invalidates every mirror, and costs the owner a re-signature for
//! nothing.

use serde::Serialize;
use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};

use crate::bawwaba::ShahadatBawwaba;
use crate::fuhusat::{IjtiyazFuhus, WasfHuzma};
use crate::irtibat::IrtibatBina;
use crate::taghtiya_ruqaa::TaqrirTaghtiya;
use crate::tahdid_maqasat::TaqreerMaqasat;
use crate::tahweel::SiyasatHuzma;
use crate::takhtit::{MizaniyatIqama, TaqreerTakhtit};
use crate::taqrir_tajawuz::TaqrirTajawuz;

/// The manifest schema this build writes.
///
/// Separate from `taarib_ruqaa::tarwisa::ISDAR_SIYAGHA`, which versions the
/// *container*. The two move independently and conflating them would force a
/// container format change every time a report grew a field.
pub const MUKHATTAT_BAYAN: u32 = 1;

/// Which engine a patch is for, as the package records it.
///
/// Three values rather than one, because they answer three different questions
/// an installer asks: which adapter to load, which injection route to use, and
/// what the user was promised about how well it would work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MuharrikHuzma {
    /// The engine family, which decides the adapter.
    pub aila: AilatMuharrik,
    /// The scripting backend, which decides the injection route.
    pub khalfiya: KhalfiyaBarmajiya,
    /// The support tier the user was shown before installing.
    pub tabaqa: Tabaqa,
}

/// How many strings the hard checks covered, and how many were approved.
///
/// Copied out of [`crate::fuhusat::IjtiyazFuhus`] as the package is written.
/// The token itself cannot travel — it is not [`Clone`], has no public fields
/// and no `Deserialize` — and it is not supposed to: what ships is the *record*
/// that the checks ran, which a reviewer reads, and not a token that would
/// assert it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SijillFuhus {
    /// How many strings were checked.
    pub nusus: usize,
    /// How many of them a human review approved.
    pub muakkada: usize,
}

impl From<IjtiyazFuhus> for SijillFuhus {
    /// Spends the token to mint the record.
    ///
    /// By value, so the counts can only be written once per run of the checks —
    /// the same reason [`crate::mujammi::ijmaa`] consumes the token rather than
    /// borrowing it.
    fn from(ijtiyaz: IjtiyazFuhus) -> Self {
        Self {
            nusus: ijtiyaz.nusus(),
            muakkada: ijtiyaz.muakkada(),
        }
    }
}

/// Everything the package declares about itself.
#[derive(Debug, Clone, Serialize)]
pub struct BayanHuzma {
    /// The manifest schema version, first field so a reader that understands
    /// nothing else can still decide whether to keep going.
    pub mukhattat: u32,
    /// The container format version this manifest was written beside.
    ///
    /// Duplicated from the header on purpose. The header is fixed-layout bytes
    /// a five-language reader parses; this is the human-readable copy that
    /// appears in a bug report when somebody pastes the manifest and not the
    /// hex dump.
    pub isdar_siyagha: u16,
    /// The patch's identity, stable across revisions.
    pub id: RuqaaId,
    /// Which revision of it this is.
    pub murajaa: RuqaaRevision,
    /// What the contributor declared.
    pub wasf: WasfHuzma,
    /// Which engine, backend and support tier.
    pub muharrik: MuharrikHuzma,
    /// What builds and fonts this patch is bound to.
    pub irtibat: IrtibatBina,
    /// What the asset gate certified.
    pub bawwaba: ShahadatBawwaba,
    /// That the hard checks ran, and over how much.
    pub fuhus: SijillFuhus,
    /// Which sizes were discovered, and which strings had none.
    pub maqasat: TaqreerMaqasat,
    /// What precomputation produced and what it skipped.
    pub takhtit: TaqreerTakhtit,
    /// The runtime atlas budget and growth policy, so an adapter reads its plan
    /// rather than compiling one in.
    pub iqama: MizaniyatIqama,
    /// Coverage, measured.
    pub taghtiya: TaqrirTaghtiya,
    /// Every string that overruns, every string that could not be checked.
    pub tajawuz: TaqrirTajawuz,
    /// Every layout decision the compile was made under, in the container's own
    /// encoding, so a reviewer reads them without decompressing a table.
    pub siyasa: SiyasatHuzma,
}

impl BayanHuzma {
    /// The manifest as the bytes the `BAYAN` section stores.
    ///
    /// Compact rather than pretty. A manifest is read by machines and by people
    /// who have already piped it through a formatter, and the indentation of a
    /// coverage report over ten thousand strings is measured in kilobytes that
    /// every player downloads.
    ///
    /// # Errors
    ///
    /// [`serde_json::Error`] when a report will not serialize, which in
    /// practice means a non-finite float reached a field that holds one. That
    /// is worth failing a compile over: `NaN` in a manifest is a number nobody
    /// measured, written down as though somebody had.
    pub fn ila_bayt(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// The sentence the compile report ends with.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} r{} for {} — {}; {}; {}",
            self.wasf.unwan,
            self.murajaa.qeema(),
            self.wasf.ism_luba,
            self.bawwaba.wasf(),
            self.maqasat.wasf(),
            self.takhtit.wasf()
        )
    }
}
