//! The signed channel manifest `tahdith.json`, verified before it is read.

use serde::Deserialize;
use taarib_khatm::{HawiyatThiqa, MIFTAH_TATWIR, MiftahAam, MirsatThiqa};

use crate::khata::{KhataTahdith, NatijatTahdith};

/// The verified channel manifest: the version on offer and its per-target
/// entries.
///
/// The verifying constructor is [`ihlil`]: it checks the detached owner
/// signature over the exact manifest bytes before a field of them is parsed.
/// There is no `Deserialize` on this type — parsing goes through the private
/// wire form inside the verifier, exactly as the revocation list's does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BayanTahdith {
    /// The version the channel offers, three dot-separated numbers.
    pub isdar: String,
    /// One entry per target and channel.
    pub madakhil: Vec<MadkhalTahdith>,
}

/// One downloadable update for one target on one channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalTahdith {
    /// The target triple, as the tables in `docs/tawzee.md` §2 name it.
    pub hadaf: String,
    /// The channel this entry belongs to: `mustaqirr` or `tajribi`.
    pub qanat: String,
    /// Where the package is downloaded from.
    pub rabt: String,
    /// The package's exact size in bytes.
    pub hajm: u64,
    /// The SHA-256 of the package bytes, 64 lowercase hex digits.
    pub sha256: String,
    /// The lowest version this package may update from.
    pub adna_isdar: String,
}

/// Verifies a channel manifest's detached signature, then parses it.
///
/// `matn` is the exact bytes of `tahdith.json` and `tawqee_hex` the detached
/// 64-byte Ed25519 signature over those bytes, hex-encoded. The signature is
/// checked against `mirsa` — the anchor compiled into this client, the same
/// one every patch verifies under — before a single field is read, so an
/// unverified manifest is never parsed at all.
///
/// # Errors
///
/// [`KhataTahdith::BayanTalif`] when the signature is not 128 lowercase hex
/// digits or when verified bytes do not parse as a manifest,
/// [`KhataTahdith::TawqeeTatwir`] when a release anchor is offered a channel
/// signed by the committed development key — refused by that name, exactly as
/// a dev-signed patch is — and [`KhataTahdith::TawqeeGhayrSalih`] when the
/// signature does not verify against the anchor at all.
pub fn ihlil(
    matn: &[u8],
    tawqee_hex: &str,
    mirsa: &MirsatThiqa,
) -> NatijatTahdith<BayanTahdith> {
    let Some(tawqee) = min_hex::<64>(tawqee_hex) else {
        return Err(KhataTahdith::BayanTalif {
            sabab: "the detached signature is not 128 lowercase hex digits".to_owned(),
        });
    };

    // An anchor that is not a canonical Ed25519 point can vouch for nothing.
    let miftah = MiftahAam::min_bayt(&mirsa.miftah)
        .map_err(|_| KhataTahdith::TawqeeGhayrSalih)?;

    if !miftah.tahaqquq(matn, &tawqee) {
        // Identity-aware refusal: under a release anchor, a manifest the
        // committed development key signed is named specifically rather than
        // treated as merely unverifiable.
        let tatwir = matches!(mirsa.hawiya, HawiyatThiqa::Isdar)
            && MiftahAam::min_bayt(&MIFTAH_TATWIR)
                .is_ok_and(|muallan| muallan.tahaqquq(matn, &tawqee));
        if tatwir {
            return Err(KhataTahdith::TawqeeTatwir);
        }
        return Err(KhataTahdith::TawqeeGhayrSalih);
    }

    let khaam: BayanKhaam = serde_json::from_slice(matn)
        .map_err(|khata| KhataTahdith::BayanTalif { sabab: khata.to_string() })?;

    Ok(BayanTahdith {
        isdar: khaam.isdar,
        madakhil: khaam
            .madakhil
            .into_iter()
            .map(|madkhal| MadkhalTahdith {
                hadaf: madkhal.hadaf,
                qanat: madkhal.qanat,
                rabt: madkhal.rabt,
                hajm: madkhal.hajm,
                sha256: madkhal.sha256,
                adna_isdar: madkhal.adna_isdar,
            })
            .collect(),
    })
}

/// Selects the entry a client on `hadaf`, following `qanat` and running
/// `isdar_hali`, should download — or `Ok(None)` when the running version is
/// already the offered one or newer.
///
/// Versions are three dot-separated numbers compared numerically, most
/// significant first, so `1.10.0` is newer than `1.9.9`.
///
/// # Errors
///
/// [`KhataTahdith::BayanTalif`] when the running version, the offered version
/// or the entry's floor is not three dot-separated numbers,
/// [`KhataTahdith::LaMadkhal`] when a newer version is offered but no entry
/// covers this target on this channel, and [`KhataTahdith::AdnaIsdarFawq`]
/// when the entry's `adna_isdar` exceeds the running version — the running
/// build is too old for this package to update in place.
pub fn intiqa<'a>(
    bayan: &'a BayanTahdith,
    hadaf: &str,
    qanat: &str,
    isdar_hali: &str,
) -> NatijatTahdith<Option<&'a MadkhalTahdith>> {
    let hali = fakk_isdar(isdar_hali).ok_or_else(|| KhataTahdith::BayanTalif {
        sabab: format!("the running version {isdar_hali:?} is not three dot-separated numbers"),
    })?;
    let marud = fakk_isdar(&bayan.isdar).ok_or_else(|| KhataTahdith::BayanTalif {
        sabab: format!(
            "the offered version {:?} is not three dot-separated numbers",
            bayan.isdar
        ),
    })?;

    if marud <= hali {
        return Ok(None);
    }

    let madkhal = bayan
        .madakhil
        .iter()
        .find(|madkhal| madkhal.hadaf == hadaf && madkhal.qanat == qanat)
        .ok_or_else(|| KhataTahdith::LaMadkhal {
            hadaf: hadaf.to_owned(),
            qanat: qanat.to_owned(),
        })?;

    let adna = fakk_isdar(&madkhal.adna_isdar).ok_or_else(|| KhataTahdith::BayanTalif {
        sabab: format!(
            "the entry's minimum version {:?} is not three dot-separated numbers",
            madkhal.adna_isdar
        ),
    })?;
    if adna > hali {
        return Err(KhataTahdith::AdnaIsdarFawq {
            adna: madkhal.adna_isdar.clone(),
            hali: isdar_hali.to_owned(),
        });
    }

    Ok(Some(madkhal))
}

/// The wire form parsed after verification. Private and never constructed by
/// hand, so no path but [`ihlil`] can produce a [`BayanTahdith`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BayanKhaam {
    isdar: String,
    madakhil: Vec<MadkhalKhaam>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MadkhalKhaam {
    hadaf: String,
    qanat: String,
    rabt: String,
    hajm: u64,
    sha256: String,
    adna_isdar: String,
}

/// A version as three numbers, most significant first — the comparison order.
fn fakk_isdar(nass: &str) -> Option<(u64, u64, u64)> {
    let mut ajza = nass.split('.');
    let kabir = juz_raqmi(ajza.next()?)?;
    let wasat = juz_raqmi(ajza.next()?)?;
    let saghir = juz_raqmi(ajza.next()?)?;
    if ajza.next().is_some() {
        return None;
    }
    Some((kabir, wasat, saghir))
}

/// One version component: decimal digits only, within `u64`.
fn juz_raqmi(juz: &str) -> Option<u64> {
    if juz.is_empty() || !juz.bytes().all(|bayt| bayt.is_ascii_digit()) {
        return None;
    }
    juz.parse::<u64>().ok()
}

fn min_hex<const N: usize>(nass: &str) -> Option<[u8; N]> {
    // lowercase hex only — the canonical form
    if !nass.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return None;
    }
    let mut khraj = [0u8; N];
    hex::decode_to_slice(nass, &mut khraj).ok()?;
    Some(khraj)
}
