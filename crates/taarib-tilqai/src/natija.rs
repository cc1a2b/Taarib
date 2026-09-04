//! What one run hands back.

use std::path::PathBuf;

use crate::khata::KhataTilqai;
use crate::mashwar::MashwarId;
use crate::taqreer::{HalatMashwar, TaqreerMashwar};

/// One finished, stopped or cancelled run.
///
/// Not a `Result`, for the same reason `taarib_tarjama::dufaat::TaqreerJawla`
/// is not one: a stopped run is not a void run. Extraction happened, money was
/// spent, a package may be on disk, and throwing all of that away to return a
/// bare error would make the resume contract unusable — the caller would have
/// nothing to show and nothing to resume from.
#[derive(Debug)]
pub struct NatijatMashwar {
    /// The run.
    pub id: MashwarId,
    /// Its directory, which is what a resume is pointed at.
    pub mujallad: PathBuf,
    /// Everything it has to say about itself. Serializable, and safe to send
    /// across a process boundary.
    pub taqreer: TaqreerMashwar,
    /// What stopped it, when something did.
    ///
    /// Kept beside the report rather than inside it because it carries an
    /// `std::io::Error`, which cannot cross a wire. The report already holds
    /// the rendered sentence.
    pub khata: Option<KhataTilqai>,
}

impl NatijatMashwar {
    /// Whether the run finished everything its options asked for.
    #[must_use]
    pub const fn tammat(&self) -> bool {
        matches!(self.taqreer.hala, HalatMashwar::Tammat)
    }

    /// Whether this game needs a runtime capture pass before it can be
    /// arabized at all.
    #[must_use]
    pub const fn yahtaj_iltiqat(&self) -> bool {
        matches!(self.taqreer.hala, HalatMashwar::TahtajIltiqat)
    }

    /// Whether the run stopped because the user cancelled.
    #[must_use]
    pub const fn mulgha(&self) -> bool {
        matches!(self.taqreer.hala, HalatMashwar::Mulgha)
    }

    /// The sealed package, when one was produced.
    #[must_use]
    pub fn huzma(&self) -> Option<&std::path::Path> {
        self.taqreer
            .huzma
            .as_ref()
            .map(|huzma| huzma.masar.as_path())
    }
}
