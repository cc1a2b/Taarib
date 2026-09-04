//! Every refusal the staging tool can produce, in band 8200.

use std::path::PathBuf;

use taarib_usus::khata::arqam;

/// A staging failure, carrying the matrix row it belongs to.
#[derive(Debug, thiserror::Error)]
pub(crate) enum KhataTajmee {
    /// An artifact the matrix names is not where its source says it is.
    #[error("[{saf}] {masar} is absent; build it before staging")]
    MukawwinGhaib {
        /// The matrix row, e.g. `B1`.
        saf: &'static str,
        /// Where it was looked for.
        masar: PathBuf,
    },

    /// A locked download's hash does not match its lock entry.
    #[error("[{saf}] {muarrif} hashes to {mahsuba}, not the locked {muqfal}")]
    QuflGhayrMutabiq {
        /// The matrix row.
        saf: &'static str,
        /// The lock entry's identifier.
        muarrif: String,
        /// The hash the lock declares.
        muqfal: String,
        /// The hash the bytes produced.
        mahsuba: String,
    },

    /// A lock entry carries no hash, so nothing about it can be trusted.
    #[error("[{saf}] the lock entry {muarrif} carries no verified hash")]
    QuflNaqis {
        /// The matrix row.
        saf: &'static str,
        /// The lock entry's identifier.
        muarrif: String,
    },

    /// A payload was built without its bootstrap, so a game would load it and
    /// nothing would happen.
    #[error("[{saf}] {} exports no taarib_bidaya; rebuild with --features hamula", masar.display())]
    BidayaGhaiba {
        /// The matrix row.
        saf: &'static str,
        /// The payload that is missing its entry point.
        masar: PathBuf,
    },

    /// A staging prerequisite is not on `PATH`.
    #[error("{adaa} is not on PATH; it is a staging prerequisite")]
    AdaaGhaiba {
        /// The program's name.
        adaa: &'static str,
    },

    /// A staging prerequisite ran and failed.
    #[error("{adaa} failed while {amal}: {sabab}")]
    AdaaFashila {
        /// The program's name.
        adaa: &'static str,
        /// What it was doing.
        amal: String,
        /// What it reported.
        sabab: String,
    },

    /// A local file could not be read or written.
    #[error("{masar} could not be read or written while {amal}")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// What was being attempted.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A target triple the matrix does not carry.
    #[error("{hadaf} is not one of the targets in docs/tawzee.md §2")]
    HadafMajhul {
        /// What was asked for.
        hadaf: String,
    },
}

impl KhataTajmee {
    /// The permanent code, allocated from this crate's band.
    #[must_use]
    pub(crate) const fn raqm(&self) -> u16 {
        arqam::TAJMEE
            + match self {
                Self::MukawwinGhaib { .. } => 0,
                Self::QuflGhayrMutabiq { .. } => 1,
                Self::QuflNaqis { .. } => 2,
                Self::BidayaGhaiba { .. } => 7,
                Self::AdaaGhaiba { .. } => 3,
                Self::AdaaFashila { .. } => 4,
                Self::KhataMalaf { .. } => 5,
                Self::HadafMajhul { .. } => 8,
            }
    }

    /// The line the operator reads, code first.
    #[must_use]
    pub(crate) fn satr(&self) -> String {
        format!("TAARIB-E-{} {self}", self.raqm())
    }
}

/// The staging result type.
pub(crate) type NatijatTajmee<T> = Result<T, KhataTajmee>;
