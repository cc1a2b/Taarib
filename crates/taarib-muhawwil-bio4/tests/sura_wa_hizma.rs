//! The two texel containers, and the measurement the whole crate rests on.
//!
//! # The measurement
//!
//! [`madakhil_tutabiq_biksilat_alluba`] is the one that matters. It reads
//! `BIO4/Font/report_zh-cn.fnt` and `BIO4/ImagePack/13000009.pack` out of an
//! installed copy of *Resident Evil 4*, decodes the pack's `DXT5` payload, walks
//! the cell grid the metrics table describes, measures where the ink actually is
//! in each cell, and asserts that it is exactly where the metrics say.
//!
//! That pair was chosen because it is one of the few the Ultimate HD Edition did
//! not re-author: its texture is still 1024×448, the dimensions its `.fnt`
//! declares. For `common_*`, `event*` and `stage*` the shipped texture is twice
//! those dimensions and is different art, and for the traditional-Chinese `ss_*`
//! fonts it is a different shape whose cells no longer line up with the metrics
//! beside them — so those cannot be used to check a reading of the metrics, and
//! this test does not pretend otherwise.
//!
//! On a machine without the game the test returns early and the round trips below
//! still run. What it can prove there is that this crate agrees with itself; what
//! it proves here is that this crate agrees with Capcom.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "a test that needs the installed game has to be told where it is, and there is no \
              configuration layer inside a test binary to resolve that through"
)]

use std::fs;
use std::path::PathBuf;

use taarib_muhawwil_bio4::hizma::{Hizma, HuwiyatHizma, ifkak_dds, irsim_dds};
use taarib_muhawwil_bio4::khatt::KhattBio4;
use taarib_muhawwil_bio4::sura::{LawhatAlwan, SuraMufakkaka, ifkak_c4, irsim_c4};

/// Anything at or above this is ink, for the cell-box measurement.
///
/// A `DXT5` alpha block interpolates, so the texel just outside a stroke is not
/// zero. Thirty-two of 255 is well below the faintest antialiased edge in the
/// shipped atlas and well above its interpolation noise; the measurement below
/// is insensitive to it over a wide range.
const HADD_HIBR: u8 = 32;

/// Where an installed copy of the game might be, in the order they are tried.
///
/// `TAARIB_RE4` first, so a machine with the game somewhere else — or a
/// directory holding bytes copied out of one — can say where without this file
/// being edited.
///
/// Returning [`None`] makes the tests that need the game pass without checking
/// anything, which is correct on a build machine and is a trap on a machine that
/// has the game and cannot see it. `TAARIB_RE4_ILZAM` closes the trap: set it and
/// a missing install is a failure instead of a silent skip.
const MURASHAHAT: [&str; 3] = [
    "/mnt/f/SteamLibrary/steamapps/common/Resident Evil 4",
    "/mnt/d/Program Files (x86)/Steam/steamapps/common/Resident Evil 4",
    "F:/SteamLibrary/steamapps/common/Resident Evil 4",
];

fn mujallad_luba() -> Option<PathBuf> {
    if let Ok(muallan) = std::env::var("TAARIB_RE4") {
        let masar = PathBuf::from(muallan);
        if masar.is_dir() {
            return Some(masar);
        }
    }
    let wujid = MURASHAHAT.into_iter().map(PathBuf::from).find(|masar| masar.is_dir());
    assert!(
        !(wujid.is_none() && std::env::var_os("TAARIB_RE4_ILZAM").is_some()),
        "TAARIB_RE4_ILZAM is set and no Resident Evil 4 install was found; \
         point TAARIB_RE4 at one, or at a directory holding its BIO4 subtree"
    );
    wujid
}

/// An image with a horizontal ramp and a diagonal stroke, for the round trips.
fn ayina(ard: u32, irtifa: u32) -> SuraMufakkaka {
    let mut sura = SuraMufakkaka::jadeeda(ard, irtifa).unwrap();
    for a in 0..irtifa {
        for s in 0..ard {
            let ramp = u8::try_from(
                (s * 255).checked_div(ard.max(1)).unwrap_or(0),
            )
            .unwrap_or(255);
            let khatt = if s == a { 255 } else { ramp };
            sura.daa_texel(s, a, khatt);
        }
    }
    sura
}

#[test]
fn lawhat_alalwan_tadur() {
    let lawhat = LawhatAlwan::taarib();
    let bayt = lawhat.ila_bayt();
    let thaniya = LawhatAlwan::min_bayt(&bayt).expect("sixteen entries should parse");
    assert_eq!(lawhat, thaniya);
    assert_eq!(lawhat.alfa(0), 0, "index zero is fully transparent");
    assert_eq!(lawhat.alfa(15), 255, "index fifteen is fully opaque");
}

#[test]
fn c4_yadur_baad_altakmim() {
    let lawhat = LawhatAlwan::taarib();
    let asl = ayina(64, 40);
    let marsuma = irsim_c4(&asl, &lawhat).expect("a 64x40 image should encode");
    let mufakkaka = ifkak_c4(&marsuma, 64, 40, &lawhat).expect("and decode");
    assert_eq!(mufakkaka.ard, 64);
    assert_eq!(mufakkaka.irtifa, 40);

    // The palette has eight opacities, so the first pass quantizes. The second
    // must not move anything: an encoder that was not idempotent on its own
    // output would drift a little every time a font was rebuilt.
    let thaniya = irsim_c4(&mufakkaka, &lawhat).expect("re-encoding quantized texels");
    assert_eq!(marsuma, thaniya, "C4 encoding must be idempotent on its own output");
}

#[test]
fn c4_yahfath_hudud_alkutla() {
    // A width that is not a whole number of 8x8 blocks: the padding must be
    // transparent and must not pull a neighbouring texel into the image.
    let lawhat = LawhatAlwan::taarib();
    let mut asl = SuraMufakkaka::jadeeda(12, 5).unwrap();
    asl.daa_texel(11, 4, 255);
    let marsuma = irsim_c4(&asl, &lawhat).unwrap();
    let mufakkaka = ifkak_c4(&marsuma, 12, 5, &lawhat).unwrap();
    assert_eq!(mufakkaka.texel(11, 4), Some(255));
    assert_eq!(mufakkaka.texel(0, 0), Some(0));
}

#[test]
fn dds_yadur_bikhata_saghira() {
    let asl = ayina(64, 32);
    let marsuma = irsim_dds(&asl).expect("a 64x32 image should encode");
    let mufakkaka = ifkak_dds(&marsuma).expect("and decode");
    assert_eq!(mufakkaka.ard, 64);
    assert_eq!(mufakkaka.irtifa, 32);

    let mut aqsa = 0u16;
    for a in 0..32 {
        for s in 0..64 {
            let qabl = asl.texel(s, a).unwrap();
            let baad = mufakkaka.texel(s, a).unwrap();
            aqsa = aqsa.max(u16::from(qabl.abs_diff(baad)));
        }
    }
    // `BC3` fits sixteen alphas onto eight interpolated values, so a block
    // holding a full ramp cannot be exact. Anything past a sixteenth of the
    // range would mean the endpoints or the index search are wrong.
    assert!(aqsa <= 16, "worst DXT5 alpha error was {aqsa}, which is past interpolation loss");
}

#[test]
fn dds_bilon_wahid_yadur_tamaman() {
    let mut asl = SuraMufakkaka::jadeeda(8, 8).unwrap();
    for a in 0..8 {
        for s in 0..8 {
            asl.daa_texel(s, a, if a < 4 { 255 } else { 0 });
        }
    }
    let marsuma = irsim_dds(&asl).unwrap();
    let mufakkaka = ifkak_dds(&marsuma).unwrap();
    assert_eq!(asl.bayt, mufakkaka.bayt, "two alpha values per block must be exact");
}

#[test]
fn hizma_tadur() {
    let hawiya = HuwiyatHizma([0x09, 0x00, 0x00, 0x18]);
    let himl = irsim_dds(&ayina(32, 16)).unwrap();
    let hizma = Hizma { hawiya, himl };
    let bayt = hizma.ila_bayt().expect("a pack should serialise");
    let thaniya = Hizma::min_bayt(&bayt).expect("and parse back");
    assert_eq!(hizma, thaniya);
    assert_eq!(thaniya.ila_bayt().unwrap(), bayt);
}

#[test]
fn hizmat_alluba_tadur_bayt_bi_bayt() {
    let Some(luba) = mujallad_luba() else {
        return;
    };
    let mut adad = 0usize;
    for ism in ["13000009.pack", "18000009.pack", "16000006.pack"] {
        let masar = luba.join("BIO4/ImagePack").join(ism);
        let Ok(bayt) = fs::read(&masar) else {
            continue;
        };
        let hizma = Hizma::min_bayt(&bayt).unwrap_or_else(|khata| panic!("{ism}: {khata}"));
        assert_eq!(hizma.ila_bayt().unwrap(), bayt, "{ism} did not round-trip");
        assert_eq!(hizma.hawiya.ism(), ism, "the header identifier must name the file");
        adad = adad.saturating_add(1);
    }
    assert!(adad > 0, "the install was found but none of the named packs were readable");
}

#[test]
fn madakhil_tutabiq_biksilat_alluba() {
    let Some(luba) = mujallad_luba() else {
        return;
    };
    let masar_khatt = luba.join("BIO4/Font/report_zh-cn.fnt");
    let masar_hizma = luba.join("BIO4/ImagePack/13000009.pack");
    let (Ok(bayt_khatt), Ok(bayt_hizma)) = (fs::read(&masar_khatt), fs::read(&masar_hizma)) else {
        return;
    };

    let khatt = KhattBio4::min_bayt(&bayt_khatt).expect("report_zh-cn.fnt should parse");
    let hizma = Hizma::min_bayt(&bayt_hizma).expect("13000009.pack should parse");
    let sura = ifkak_dds(&hizma.himl).expect("its payload is a DXT5 DDS");
    let shabaka = khatt.shabaka().expect("the grid law should hold");

    assert_eq!(
        (sura.ard, sura.irtifa),
        (shabaka.ard(), shabaka.irtifa()),
        "this is the pair whose shipped texture still matches its declared dimensions"
    );

    let hajm = shabaka.hajm_khana();
    let mut muwafiq = 0usize;
    let mut mukhalif: Vec<String> = Vec::new();
    for fahras in 0..khatt.adad_khanat() {
        let raqm = u32::try_from(fahras).unwrap();
        let madkhal = khatt.madakhil[fahras];
        let (cx, cy) = shabaka.mawdi(raqm).expect("every counted cell is in the grid");

        let mut yasar: Option<u32> = None;
        let mut yameen: Option<u32> = None;
        for amud in 0..hajm {
            let mahbur = (0..hajm)
                .any(|satr| sura.texel(cx + amud, cy + satr).unwrap_or(0) >= HADD_HIBR);
            if mahbur {
                yasar = Some(yasar.unwrap_or(amud));
                yameen = Some(amud + 1);
            }
        }

        let mahsub = match (yasar, yameen) {
            (Some(bidaya), Some(nihaya)) => (bidaya, nihaya),
            _ => (u32::from(madkhal.yasar), u32::from(madkhal.yasar)),
        };
        let muallan = (u32::from(madkhal.yasar), u32::from(madkhal.yameen));
        if madkhal.khali() {
            if yasar.is_none() {
                muwafiq += 1;
            } else {
                mukhalif.push(format!("cell {fahras}: declared blank, ink at {mahsub:?}"));
            }
        } else if mahsub == muallan {
            muwafiq += 1;
        } else {
            mukhalif.push(format!("cell {fahras}: declared {muallan:?}, measured {mahsub:?}"));
        }
    }

    assert!(
        mukhalif.is_empty(),
        "{} of {} cells disagreed with the shipped texture; first few: {:?}",
        mukhalif.len(),
        khatt.adad_khanat(),
        mukhalif.get(..5.min(mukhalif.len())).unwrap_or(&[])
    );
    assert_eq!(
        muwafiq,
        548,
        "report_zh-cn.fnt describes 548 cells and every one of them must be where it says"
    );
}
