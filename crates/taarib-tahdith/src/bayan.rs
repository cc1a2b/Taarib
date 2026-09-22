//! The signed channel manifest `tahdith.json`, verified before it is read.

use serde::Deserialize;
use taarib_khatm::{HawiyatThiqa, MIFTAH_TATWIR, MiftahAam, MiftahKhass, MirsatThiqa};

use crate::khata::{KhataTahdith, NatijatTahdith};

/// The manifest format this build reads and writes.
///
/// The number at the document's root, exactly as the revocation list carries
/// its own: a manifest from a format this build does not read is refused rather
/// than parsed field by field until something happens to fit.
pub const ISDAR_BAYAN: u32 = 1;

/// The released channel, as the manifest spells it.
pub const QANAT_MUSTAQIRR: &str = "mustaqirr";

/// The pre-release channel, as the manifest spells it.
pub const QANAT_TAJRIBI: &str = "tajribi";

/// Every channel name a client asks for.
///
/// And therefore every name an entry may be cast under: an entry on any other
/// channel is one no client would ever select, so the writer refuses it instead
/// of publishing it.
pub const QANAWAT: [&str; 2] = [QANAT_MUSTAQIRR, QANAT_TAJRIBI];

/// The verified channel manifest: one entry per target, channel and version.
///
/// The verifying constructor is [`ihlil`]: it checks the detached owner
/// signature over the exact manifest bytes before a field of them is parsed.
/// There is no `Deserialize` on this type — parsing goes through the private
/// wire form inside the verifier, exactly as the revocation list's does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BayanTahdith {
    /// One entry per target, channel and offered version.
    pub madakhil: Vec<MadkhalTahdith>,
}

/// One downloadable update for one target on one channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalTahdith {
    /// The target triple, as the tables in `docs/tawzee.md` §2 name it.
    pub hadaf: String,
    /// The channel this entry belongs to: `mustaqirr` or `tajribi`.
    pub qanat: String,
    /// The version this entry offers, three dot-separated numbers.
    ///
    /// Per entry rather than per manifest, because that is what a channel is
    /// for: `tajribi` exists to offer a version ahead of `mustaqirr`, and one
    /// version at the document's root would either hold the pre-release channel
    /// back to the released one or offer a release client a build that has no
    /// entry for it.
    pub isdar: String,
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
/// digits, when verified bytes do not parse as a manifest, or when they are a
/// manifest format this build does not read,
/// [`KhataTahdith::TawqeeTatwir`] when a release anchor is offered a channel
/// signed by the committed development key — refused by that name, exactly as
/// a dev-signed patch is — and [`KhataTahdith::TawqeeGhayrSalih`] when the
/// signature does not verify against the anchor at all.
pub fn ihlil(matn: &[u8], tawqee_hex: &str, mirsa: &MirsatThiqa) -> NatijatTahdith<BayanTahdith> {
    let Some(tawqee) = min_hex::<64>(tawqee_hex) else {
        return Err(KhataTahdith::BayanTalif {
            sabab: "the detached signature is not 128 lowercase hex digits".to_owned(),
        });
    };

    // An anchor that is not a canonical Ed25519 point can vouch for nothing.
    let miftah = MiftahAam::min_bayt(&mirsa.miftah).map_err(|_| KhataTahdith::TawqeeGhayrSalih)?;

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

    let khaam: BayanKhaam =
        serde_json::from_slice(matn).map_err(|khata| KhataTahdith::BayanTalif {
            sabab: khata.to_string(),
        })?;

    if khaam.isdar != ISDAR_BAYAN {
        return Err(KhataTahdith::BayanTalif {
            sabab: format!(
                "the manifest is format {}, and this build reads format {ISDAR_BAYAN}",
                khaam.isdar
            ),
        });
    }

    Ok(BayanTahdith {
        madakhil: khaam
            .madakhil
            .into_iter()
            .map(|madkhal| MadkhalTahdith {
                hadaf: madkhal.hadaf,
                qanat: madkhal.qanat,
                isdar: madkhal.isdar,
                rabt: madkhal.rabt,
                hajm: madkhal.hajm,
                sha256: madkhal.sha256,
                adna_isdar: madkhal.adna_isdar,
            })
            .collect(),
    })
}

/// Selects the entry a client on `hadaf`, following `qanat` and running
/// `isdar_hali`, should download — or `Ok(None)` when the channel offers it
/// nothing newer.
///
/// Versions are three dot-separated numbers compared numerically, most
/// significant first, so `1.10.0` is newer than `1.9.9`. Among the entries this
/// client could actually install the newest wins, which is what makes a stepped
/// channel work: a client too old for the latest package is offered the newest
/// package whose floor it does meet, rather than nothing.
///
/// A channel with no entry at all for this target is `Ok(None)` and not a
/// failure. It is the ordinary state of a manifest that covers only the other
/// channel, and a client whose platform is simply not on a pre-release channel
/// has nothing to be told and nothing to do.
///
/// # Errors
///
/// [`KhataTahdith::BayanTalif`] when the running version or an entry's versions
/// are not three dot-separated numbers, and [`KhataTahdith::AdnaIsdarFawq`]
/// when a newer version is offered but every entry offering one declares a
/// floor above the running version — the running build is too old for any
/// package on this channel to update it in place.
pub fn intiqa<'a>(
    bayan: &'a BayanTahdith,
    hadaf: &str,
    qanat: &str,
    isdar_hali: &str,
) -> NatijatTahdith<Option<&'a MadkhalTahdith>> {
    let hali = fakk_isdar(isdar_hali).ok_or_else(|| KhataTahdith::BayanTalif {
        sabab: format!("the running version {isdar_hali:?} is not three dot-separated numbers"),
    })?;

    let mut mumkin: Option<(&MadkhalTahdith, (u64, u64, u64))> = None;
    let mut baeed: Option<(&MadkhalTahdith, (u64, u64, u64))> = None;

    for madkhal in &bayan.madakhil {
        if madkhal.hadaf != hadaf || madkhal.qanat != qanat {
            continue;
        }
        let marud = fakk_isdar(&madkhal.isdar).ok_or_else(|| KhataTahdith::BayanTalif {
            sabab: format!(
                "the offered version {:?} is not three dot-separated numbers",
                madkhal.isdar
            ),
        })?;
        if marud <= hali {
            continue;
        }
        let adna = fakk_isdar(&madkhal.adna_isdar).ok_or_else(|| KhataTahdith::BayanTalif {
            sabab: format!(
                "the entry's minimum version {:?} is not three dot-separated numbers",
                madkhal.adna_isdar
            ),
        })?;

        if adna <= hali {
            if mumkin.is_none_or(|(_, ahsan)| marud > ahsan) {
                mumkin = Some((madkhal, marud));
            }
        // The closest floor, so the refusal names the smallest step this build
        // would have to take by hand rather than the largest.
        } else if baeed.is_none_or(|(_, aqrab)| adna < aqrab) {
            baeed = Some((madkhal, adna));
        }
    }

    if let Some((madkhal, _)) = mumkin {
        return Ok(Some(madkhal));
    }
    if let Some((madkhal, _)) = baeed {
        return Err(KhataTahdith::AdnaIsdarFawq {
            adna: madkhal.adna_isdar.clone(),
            hali: isdar_hali.to_owned(),
        });
    }
    Ok(None)
}

/// Casts the channel manifest: entries in, the bytes a client verifies out.
///
/// The counterpart to [`ihlil`], and the only thing that produces a
/// `tahdith.json` at all. It shares its canonical form with the verifier for
/// the same reason the revocation list's writer does — a second spelling of the
/// signed bytes is a second place for them to drift, and the drift produces
/// documents that verify against nothing — and it validates every entry by the
/// reader's own rules before signing, so an entry a client would refuse is
/// refused here, on the owner's machine, and never reaches one.
///
/// It cannot construct a [`BayanTahdith`]: [`ihlil`] stays the only constructor
/// of the verified type.
#[derive(Debug, Default)]
pub struct KatibBayan {
    madakhil: Vec<MadkhalTahdith>,
}

impl KatibBayan {
    /// An empty manifest.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            madakhil: Vec::new(),
        }
    }

    /// The entries of a manifest that was already cast, so a run that publishes
    /// one channel keeps the other.
    #[must_use]
    pub fn min_bayan(bayan: &BayanTahdith) -> Self {
        Self {
            madakhil: bayan.madakhil.clone(),
        }
    }

    /// Drops every entry on `qanat`, which is what casting that channel does to
    /// it before the run's own entries are added.
    #[must_use]
    pub fn bila_qanat(mut self, qanat: &str) -> Self {
        self.madakhil.retain(|madkhal| madkhal.qanat != qanat);
        self
    }

    /// Adds an entry, replacing any that names the same target, channel and
    /// version.
    #[must_use]
    pub fn adif(mut self, madkhal: MadkhalTahdith) -> Self {
        self.madakhil.retain(|mawjud| {
            mawjud.hadaf != madkhal.hadaf
                || mawjud.qanat != madkhal.qanat
                || mawjud.isdar != madkhal.isdar
        });
        self.madakhil.push(madkhal);
        self
    }

    /// Every entry the manifest would be cast with.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalTahdith] {
        &self.madakhil
    }

    /// The signed document and its detached signature, hex-encoded.
    ///
    /// The signature covers the exact bytes returned beside it, which are the
    /// exact bytes that must be written to `tahdith.json`: re-serializing them
    /// anywhere between here and the repository invalidates the signature.
    ///
    /// # Errors
    ///
    /// [`KhataTahdith::BayanTalif`] naming the entry and the field when an
    /// entry is one no client could use — an unknown channel, a version that is
    /// not three dot-separated numbers, a floor above the version it gates, a
    /// digest that is not 64 lowercase hex digits, an empty package, or an
    /// address that is not `https` — and when the document does not serialize.
    pub fn uktub(&self, khass: &MiftahKhass) -> NatijatTahdith<(Vec<u8>, String)> {
        let mut murattab = self.madakhil.clone();
        for madkhal in &murattab {
            fahs_madkhal(madkhal)?;
        }
        // A deterministic order so two casts of the same entries produce the
        // same bytes, and a diff of the repository shows what actually moved.
        murattab.sort_by(|awwal, thani| {
            awwal
                .hadaf
                .cmp(&thani.hadaf)
                .then_with(|| awwal.qanat.cmp(&thani.qanat))
                .then_with(|| fakk_isdar(&thani.isdar).cmp(&fakk_isdar(&awwal.isdar)))
        });

        let wathiqa = serde_json::json!({
            "isdar": ISDAR_BAYAN,
            "madakhil": murattab
                .iter()
                .map(|madkhal| serde_json::json!({
                    "hadaf": madkhal.hadaf,
                    "qanat": madkhal.qanat,
                    "isdar": madkhal.isdar,
                    "rabt": madkhal.rabt,
                    "hajm": madkhal.hajm,
                    "sha256": madkhal.sha256,
                    "adna_isdar": madkhal.adna_isdar,
                }))
                .collect::<Vec<_>>(),
        });
        let matn =
            serde_json::to_vec_pretty(&wathiqa).map_err(|khata| KhataTahdith::BayanTalif {
                sabab: khata.to_string(),
            })?;
        let tawqee = hex::encode(khass.waqqi(&matn));
        Ok((matn, tawqee))
    }
}

/// Every rule the reader would judge one entry by, applied before it is signed.
fn fahs_madkhal(madkhal: &MadkhalTahdith) -> NatijatTahdith<()> {
    let marfud = |sabab: String| KhataTahdith::BayanTalif { sabab };
    let ism = format!("{} on {}", madkhal.hadaf, madkhal.qanat);

    if madkhal.hadaf.trim().is_empty() {
        return Err(marfud("an entry names no target triple".to_owned()));
    }
    if !QANAWAT.contains(&madkhal.qanat.as_str()) {
        return Err(marfud(format!(
            "{ism}: {:?} is not a channel any client asks for",
            madkhal.qanat
        )));
    }
    let Some(marud) = fakk_isdar(&madkhal.isdar) else {
        return Err(marfud(format!(
            "{ism}: the offered version {:?} is not three dot-separated numbers",
            madkhal.isdar
        )));
    };
    let Some(adna) = fakk_isdar(&madkhal.adna_isdar) else {
        return Err(marfud(format!(
            "{ism}: the minimum version {:?} is not three dot-separated numbers",
            madkhal.adna_isdar
        )));
    };
    if adna > marud {
        return Err(marfud(format!(
            "{ism}: the minimum version {} is above the {} it gates, so no build could take it",
            madkhal.adna_isdar, madkhal.isdar
        )));
    }
    if min_hex::<32>(&madkhal.sha256).is_none() {
        return Err(marfud(format!(
            "{ism}: the digest is not 64 lowercase hex digits"
        )));
    }
    if madkhal.hajm == 0 {
        return Err(marfud(format!("{ism}: the package is declared empty")));
    }
    if !madkhal.rabt.starts_with("https://") {
        return Err(marfud(format!(
            "{ism}: {:?} is not https, and an update is never fetched in the clear",
            madkhal.rabt
        )));
    }
    Ok(())
}

/// The wire form parsed after verification. Private and never constructed by
/// hand, so no path but [`ihlil`] can produce a [`BayanTahdith`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BayanKhaam {
    isdar: u32,
    madakhil: Vec<MadkhalKhaam>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MadkhalKhaam {
    hadaf: String,
    qanat: String,
    isdar: String,
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
    if !nass
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return None;
    }
    let mut khraj = [0u8; N];
    hex::decode_to_slice(nass, &mut khraj).ok()?;
    Some(khraj)
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_khatm::{HawiyatThiqa, MirsatThiqa};

    use super::{
        BayanTahdith, KatibBayan, MadkhalTahdith, MiftahKhass, QANAT_MUSTAQIRR, QANAT_TAJRIBI,
        ihlil, intiqa,
    };
    use crate::khata::KhataTahdith;

    /// What every test here answers with, so a fixture failure propagates with
    /// `?`. `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const HADAF: &str = "x86_64-pc-windows-msvc";
    const AKHAR: &str = "x86_64-unknown-linux-gnu";
    const BASMA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn miftah() -> MiftahKhass {
        MiftahKhass::min_bayt(&[7u8; 32])
    }

    fn mirsa(khass: &MiftahKhass) -> MirsatThiqa {
        MirsatThiqa {
            miftah: khass.aam().bayt(),
            hawiya: HawiyatThiqa::Isdar,
        }
    }

    fn madkhal(hadaf: &str, qanat: &str, isdar: &str, adna: &str) -> MadkhalTahdith {
        MadkhalTahdith {
            hadaf: hadaf.to_owned(),
            qanat: qanat.to_owned(),
            isdar: isdar.to_owned(),
            rabt: format!("https://example.invalid/taarib-{isdar}-{hadaf}.exe"),
            hajm: 1024,
            sha256: BASMA.to_owned(),
            adna_isdar: adna.to_owned(),
        }
    }

    /// Casts a manifest and reads it straight back through the verifier, which
    /// is the only way a `BayanTahdith` can be had at all.
    fn dawra(katib: &KatibBayan) -> NatijatIkhtibar<BayanTahdith> {
        let khass = miftah();
        let (matn, tawqee) = katib.uktub(&khass)?;
        Ok(ihlil(&matn, &tawqee, &mirsa(&khass))?)
    }

    #[test]
    fn al_katib_wa_al_qari_yatafiqan() -> NatijatIkhtibar {
        let bayan = dawra(
            &KatibBayan::jadeed()
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0"))
                .adif(madkhal(AKHAR, QANAT_MUSTAQIRR, "1.2.0", "1.0.0")),
        )?;

        assert_eq!(bayan.madakhil.len(), 2);
        let mukhtar = intiqa(&bayan, HADAF, QANAT_MUSTAQIRR, "1.1.0")?
            .ok_or("the entry for this target was not selected")?;
        assert_eq!(mukhtar.isdar, "1.2.0");
        assert_eq!(mukhtar.hadaf, HADAF);
        Ok(())
    }

    /// The whole reason the version moved onto the entry: a manifest that
    /// offers a pre-release build says nothing to a client on the released
    /// channel, rather than failing for it.
    #[test]
    fn qanat_tajribiya_wahdaha_la_taqul_shayan_lilmustaqirr() -> NatijatIkhtibar {
        let bayan =
            dawra(&KatibBayan::jadeed().adif(madkhal(HADAF, QANAT_TAJRIBI, "1.3.0", "1.0.0")))?;

        assert_eq!(
            intiqa(&bayan, HADAF, QANAT_MUSTAQIRR, "1.2.0")?,
            None,
            "a released client is offered nothing and told nothing"
        );
        assert!(intiqa(&bayan, HADAF, QANAT_TAJRIBI, "1.2.0")?.is_some());
        Ok(())
    }

    /// A target the manifest does not cover is the same silence, not a refusal.
    #[test]
    fn hadaf_ghayr_madhkur_la_yakhtim_bikhata() -> NatijatIkhtibar {
        let bayan =
            dawra(&KatibBayan::jadeed().adif(madkhal(AKHAR, QANAT_MUSTAQIRR, "1.3.0", "1.0.0")))?;
        assert_eq!(intiqa(&bayan, HADAF, QANAT_MUSTAQIRR, "1.2.0")?, None);
        Ok(())
    }

    /// The newest package this build can actually take, not the newest package.
    #[test]
    fn yakhtar_ahdath_ma_yastatee_akhdhahu() -> NatijatIkhtibar {
        let bayan = dawra(
            &KatibBayan::jadeed()
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "3.0.0", "2.0.0"))
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "2.0.0", "1.0.0"))
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "1.5.0", "1.0.0")),
        )?;

        let mukhtar = intiqa(&bayan, HADAF, QANAT_MUSTAQIRR, "1.1.0")?
            .ok_or("the reachable step was not selected")?;
        assert_eq!(mukhtar.isdar, "2.0.0", "3.0.0 needs 2.0.0 first");

        let baad = intiqa(&bayan, HADAF, QANAT_MUSTAQIRR, "2.0.0")?
            .ok_or("the second step was not selected")?;
        assert_eq!(baad.isdar, "3.0.0");
        Ok(())
    }

    /// When no step is reachable the refusal names the closest floor, which is
    /// the smallest install the user has to do by hand.
    #[test]
    fn la_khatwa_mumkina_tusamma_aqrab_hadd() -> NatijatIkhtibar {
        let bayan = dawra(
            &KatibBayan::jadeed()
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "3.0.0", "2.5.0"))
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "2.0.0", "1.9.0")),
        )?;

        match intiqa(&bayan, HADAF, QANAT_MUSTAQIRR, "1.0.0") {
            Err(KhataTahdith::AdnaIsdarFawq { adna, hali }) => {
                assert_eq!(adna, "1.9.0");
                assert_eq!(hali, "1.0.0");
                Ok(())
            },
            akhar => Err(format!("expected AdnaIsdarFawq, got {akhar:?}").into()),
        }
    }

    /// One byte moved in the document and the signature covers nothing.
    #[test]
    fn tabdeel_bayt_yubtil_al_tawqee() -> NatijatIkhtibar {
        let khass = miftah();
        let (mut matn, tawqee) = KatibBayan::jadeed()
            .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0"))
            .uktub(&khass)?;
        let mawdi = matn
            .iter()
            .position(|bayt| *bayt == b'2')
            .ok_or("the fixture carries no digit to move")?;
        let hadaf = matn
            .get_mut(mawdi)
            .ok_or("the position just found is gone")?;
        *hadaf = b'9';

        match ihlil(&matn, &tawqee, &mirsa(&khass)) {
            Err(KhataTahdith::TawqeeGhayrSalih) => Ok(()),
            akhar => Err(format!("expected TawqeeGhayrSalih, got {akhar:?}").into()),
        }
    }

    /// Casting a channel replaces that channel and leaves the other standing.
    #[test]
    fn sabk_qanat_la_yamahu_al_ukhra() -> NatijatIkhtibar {
        let awwal = dawra(
            &KatibBayan::jadeed()
                .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0"))
                .adif(madkhal(HADAF, QANAT_TAJRIBI, "1.3.0", "1.0.0")),
        )?;

        let thani = dawra(
            &KatibBayan::min_bayan(&awwal)
                .bila_qanat(QANAT_TAJRIBI)
                .adif(madkhal(HADAF, QANAT_TAJRIBI, "1.4.0", "1.2.0")),
        )?;

        assert_eq!(
            intiqa(&thani, HADAF, QANAT_MUSTAQIRR, "1.0.0")?.map(|m| m.isdar.clone()),
            Some("1.2.0".to_owned()),
            "the released channel was not touched"
        );
        assert_eq!(
            intiqa(&thani, HADAF, QANAT_TAJRIBI, "1.2.0")?.map(|m| m.isdar.clone()),
            Some("1.4.0".to_owned()),
            "the pre-release channel was replaced, not appended to"
        );
        Ok(())
    }

    /// The same entries cast twice produce the same bytes, whatever order they
    /// were added in.
    #[test]
    fn al_sabk_hatmi() -> NatijatIkhtibar {
        let khass = miftah();
        let (awwal, tawqee_awwal) = KatibBayan::jadeed()
            .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0"))
            .adif(madkhal(AKHAR, QANAT_TAJRIBI, "1.3.0", "1.0.0"))
            .uktub(&khass)?;
        let (thani, tawqee_thani) = KatibBayan::jadeed()
            .adif(madkhal(AKHAR, QANAT_TAJRIBI, "1.3.0", "1.0.0"))
            .adif(madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0"))
            .uktub(&khass)?;

        assert_eq!(awwal, thani);
        assert_eq!(tawqee_awwal, tawqee_thani);
        Ok(())
    }

    /// Every rule the reader would judge an entry by is applied before it is
    /// signed, so a manifest nobody could use is refused on the owner's machine.
    #[test]
    fn al_katib_yarfud_ma_yarfuduhu_al_qari() -> NatijatIkhtibar {
        let khass = miftah();
        let marfud = |madkhal: MadkhalTahdith, juz: &str| -> NatijatIkhtibar {
            match KatibBayan::jadeed().adif(madkhal).uktub(&khass) {
                Err(KhataTahdith::BayanTalif { sabab }) if sabab.contains(juz) => Ok(()),
                akhar => {
                    Err(format!("expected BayanTalif mentioning {juz:?}, got {akhar:?}").into())
                },
            }
        };

        marfud(
            madkhal(HADAF, "layliya", "1.2.0", "1.0.0"),
            "not a channel any client asks for",
        )?;
        marfud(
            madkhal(HADAF, QANAT_MUSTAQIRR, "1.2", "1.0.0"),
            "three dot-separated numbers",
        )?;
        marfud(
            madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.3.0"),
            "is above the",
        )?;

        let mut basma_kabira = madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0");
        basma_kabira.sha256 = basma_kabira.sha256.to_uppercase();
        marfud(basma_kabira, "64 lowercase hex digits")?;

        let mut farigha = madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0");
        farigha.hajm = 0;
        marfud(farigha, "declared empty")?;

        let mut maftuha = madkhal(HADAF, QANAT_MUSTAQIRR, "1.2.0", "1.0.0");
        maftuha.rabt = "http://example.invalid/taarib.exe".to_owned();
        marfud(maftuha, "not https")
    }
}
