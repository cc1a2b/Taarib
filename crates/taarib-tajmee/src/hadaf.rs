//! The five targets of the matrix, and the file naming each one implies.

use taarib_usus::manassa::{Mimariya, NizamTashghil};

use crate::khata::{KhataTajmee, NatijatTajmee};

/// One bundle target: the triple it builds for and the platform it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hadaf {
    /// The Rust target triple.
    pub(crate) muthallath: &'static str,
    /// The operating system the bundle runs on.
    pub(crate) nizam: NizamTashghil,
    /// The architecture the bundle runs on.
    pub(crate) mimariya: Mimariya,
}

/// Every target docs/tawzee.md §2 names, in its order.
pub(crate) const AHDAF: [Hadaf; 4] = [
    Hadaf {
        muthallath: "x86_64-pc-windows-msvc",
        nizam: NizamTashghil::Windows,
        mimariya: Mimariya::X8664,
    },
    Hadaf {
        muthallath: "x86_64-unknown-linux-gnu",
        nizam: NizamTashghil::Linux,
        mimariya: Mimariya::X8664,
    },
    Hadaf {
        muthallath: "aarch64-apple-darwin",
        nizam: NizamTashghil::Mac,
        mimariya: Mimariya::Aarch64,
    },
    Hadaf {
        muthallath: "x86_64-apple-darwin",
        nizam: NizamTashghil::Mac,
        mimariya: Mimariya::X8664,
    },
];

/// The target a triple names.
///
/// # Errors
///
/// [`KhataTajmee::HadafMajhul`] for a triple the matrix does not carry. The
/// Steam Deck is not a target of its own: it takes the Linux `AppImage`.
pub(crate) fn min_muthallath(muthallath: &str) -> NatijatTajmee<Hadaf> {
    AHDAF
        .into_iter()
        .find(|hadaf| hadaf.muthallath == muthallath)
        .ok_or_else(|| KhataTajmee::HadafMajhul { hadaf: muthallath.to_owned() })
}

/// The game-side payload targets every bundle carries.
///
/// Windows always, because a Linux or macOS client installing into a Wine or
/// Proton game deploys the Windows payloads; the host's own platform as well,
/// for games that run natively.
#[must_use]
pub(crate) fn hamulat_alalaab(hadaf: Hadaf) -> Vec<(NizamTashghil, Mimariya)> {
    let mut hamulat = vec![
        (NizamTashghil::Windows, Mimariya::X8664),
        (NizamTashghil::Windows, Mimariya::X86),
    ];
    if hadaf.nizam != NizamTashghil::Windows {
        hamulat.push((hadaf.nizam, hadaf.mimariya));
    }
    hamulat
}

/// The Rust target triple that builds a payload for a platform pair.
#[must_use]
pub(crate) const fn muthallath_hamula(nizam: NizamTashghil, mimariya: Mimariya) -> &'static str {
    match (nizam, mimariya) {
        (NizamTashghil::Windows, Mimariya::X8664) => "x86_64-pc-windows-msvc",
        (NizamTashghil::Windows, Mimariya::X86) => "i686-pc-windows-msvc",
        (NizamTashghil::Windows, Mimariya::Aarch64) => "aarch64-pc-windows-msvc",
        (NizamTashghil::Linux, Mimariya::X8664) => "x86_64-unknown-linux-gnu",
        (NizamTashghil::Linux, Mimariya::X86) => "i686-unknown-linux-gnu",
        (NizamTashghil::Linux, Mimariya::Aarch64) => "aarch64-unknown-linux-gnu",
        (NizamTashghil::Mac, Mimariya::Aarch64) => "aarch64-apple-darwin",
        (NizamTashghil::Mac, Mimariya::X8664 | Mimariya::X86) => "x86_64-apple-darwin",
    }
}
