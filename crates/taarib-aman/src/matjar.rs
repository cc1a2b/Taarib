//! متجر — Steam's catalogue, read once for a whole pipeline run.
//!
//! `appcache/appinfo.vdf` is the one file two independent checks in this crate
//! both need, and for the same underlying reason: neither fact they are after
//! is written into a game's own folder. VAC is declared in Steam's catalogue
//! and nowhere else, so the anti-cheat scan has to open it; whether a game has
//! servers is a store category, so the multiplayer scan has to open it too.
//!
//! Each probe used to open it for itself. [`crate::fahs::fahs`] runs both
//! before it will authorise an installation, so the file was read and parsed
//! **twice per install**. It is not a small file: on the machine this was
//! measured on it is 1 781 495 bytes in the newest framing — magic
//! `0x07564429`, with a string table — and [`vdf::murur_appinfo`] materialises
//! every app entry's tree in turn to reach the one app being asked about, so
//! the second pass cost everything the first did. A mature account makes that
//! file larger, never smaller.
//!
//! So the read happens here, once, and both projections run inside the single
//! traversal. Which half a caller wanted is a field on [`QiraatMatjar`], not a
//! second visit to the disk.
//!
//! This is also the only place in the crate that opens the catalogue. The two
//! probes used to export a `fahs_matjar` apiece, which is the shape that
//! produced the double read in the first place: two public functions, each
//! opening the same file, with nothing to stop a third caller adding a third
//! read.

use std::io;
use std::path::{Path, PathBuf};

use taarib_kashf::matajir::vdf;

use crate::kashf_himaya::{DaleelHimaya, HalatMatjar, adillat_appinfo};
use crate::kashf_shabaka::{DaleelShabaka, dalail_appinfo};

/// One reading of Steam's catalogue: what it declared about an app, and
/// whether it was read at all.
///
/// Both evidence lists are empty when the catalogue said nothing *and* when it
/// could not be opened, and those are not the same fact — that is the whole
/// argument on [`HalatMatjar`]. Neither list may be acted on without reading
/// [`Self::hala`] beside it.
#[derive(Debug, Clone)]
pub struct QiraatMatjar {
    /// The anti-cheat signatures the catalogue records against this app.
    pub himaya: Vec<DaleelHimaya>,
    /// The multiplayer categories and server metadata it records against it.
    pub shabaka: Vec<DaleelShabaka>,
    /// What became of the read.
    pub hala: HalatMatjar,
}

impl QiraatMatjar {
    /// Reads the catalogue for one app, once.
    ///
    /// Never fails: a catalogue that cannot be opened becomes
    /// [`HalatMatjar::Mutaadhdhir`] on the result rather than an error, so that
    /// a caller cannot receive that fact and discard it by writing `.ok()`.
    #[must_use]
    pub fn iqra(appid: Option<u32>, jidhr_steam: Option<&Path>) -> Self {
        match (appid, jidhr_steam) {
            (None, _) => Self::bila(HalatMatjar::GhayrMatlub),
            (Some(_), None) => Self::bila(HalatMatjar::JidhrMajhul),
            (Some(appid), Some(jidhr_steam)) => {
                let masar = masar_appinfo(jidhr_steam);
                match istikhraj(&masar, appid) {
                    Ok((himaya, shabaka)) => Self { himaya, shabaka, hala: HalatMatjar::Maqru },
                    // The short kind label rather than a sentence: it is what
                    // the refusal text and both gap lists already carry, and
                    // the path it happened to is the field standing beside it.
                    Err(khata) => Self::bila(HalatMatjar::Mutaadhdhir {
                        masar,
                        sabab: format!("{:?}", khata.kind()),
                    }),
                }
            },
        }
    }

    /// A reading that found nothing because it never happened.
    const fn bila(hala: HalatMatjar) -> Self {
        Self { himaya: Vec::new(), shabaka: Vec::new(), hala }
    }
}

/// Where Steam keeps the catalogue under an installation root.
#[must_use]
pub fn masar_appinfo(jidhr_steam: &Path) -> PathBuf {
    jidhr_steam.join("appcache").join("appinfo.vdf")
}

/// The single read and the single traversal.
///
/// # Errors
///
/// The underlying [`std::io::Error`] when `appinfo.vdf` cannot be read, and an
/// [`std::io::ErrorKind::Other`] carrying the reader's message when the
/// catalogue's own framing is corrupt. A malformed *app* is not a failure:
/// [`vdf::murur_appinfo`] hands that entry over as an `Err` and the sweep
/// continues at the next one, which is why one bad app costs one app.
fn istikhraj(masar: &Path, appid: u32) -> Result<(Vec<DaleelHimaya>, Vec<DaleelShabaka>), io::Error>
{
    let bayt = std::fs::read(masar)?;

    let mut himaya: Vec<DaleelHimaya> = Vec::new();
    let mut shabaka: Vec<DaleelShabaka> = Vec::new();
    let natija = vdf::murur_appinfo(masar, &bayt, &mut |madkhal| {
        let Ok(madkhal) = madkhal else {
            return;
        };
        if madkhal.app != appid {
            return;
        }
        // Both projections off the one tree, while it is still built. Walking
        // the file again for the other half is what this module exists to stop.
        himaya.extend(adillat_appinfo(&madkhal.bayanat, masar));
        shabaka.extend(dalail_appinfo(&madkhal.bayanat, masar));
    });
    natija.map_err(|khata| io::Error::other(khata.injilizi))?;
    Ok((himaya, shabaka))
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    #[test]
    fn bila_muarrif_la_yuqra_shay() {
        let qiraa = QiraatMatjar::iqra(None, Some(Path::new("/steam")));
        assert_eq!(qiraa.hala, HalatMatjar::GhayrMatlub);
        assert!(qiraa.himaya.is_empty() && qiraa.shabaka.is_empty());
    }

    #[test]
    fn muarrif_bila_jidhr_yubqi_al_sual_bila_jawab() {
        let qiraa = QiraatMatjar::iqra(Some(730), None);
        assert_eq!(qiraa.hala, HalatMatjar::JidhrMajhul);
        assert!(qiraa.hala.lam_yuqra(), "an unaskable question is not an answered one");
    }

    #[test]
    fn fahras_ghayr_mawjud_yusajjal_ka_mutaadhdhir() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_steam = masrah.path().join("Steam");
        fs::create_dir_all(&jidhr_steam)?;

        let qiraa = QiraatMatjar::iqra(Some(730), Some(&jidhr_steam));
        assert_eq!(
            qiraa.hala,
            HalatMatjar::Mutaadhdhir {
                masar: masar_appinfo(&jidhr_steam),
                sabab: "NotFound".to_owned(),
            }
        );
        Ok(())
    }

    #[test]
    fn tarwisa_talifa_tabqa_khata_qiraa_wahid() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_steam = masrah.path().join("Steam");
        fs::create_dir_all(jidhr_steam.join("appcache"))?;
        // Four bytes that are not one of the three magics this build reads.
        fs::write(masar_appinfo(&jidhr_steam), b"\x00\x00\x00\x00")?;

        // A corrupt framing is one failure for both halves, not one each: the
        // file is opened once, so it can only fail once.
        let qiraa = QiraatMatjar::iqra(Some(730), Some(&jidhr_steam));
        assert!(qiraa.hala.lam_yuqra(), "{:?}", qiraa.hala);
        assert!(qiraa.himaya.is_empty() && qiraa.shabaka.is_empty());
        Ok(())
    }
}
