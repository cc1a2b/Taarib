//! بذرة الرقع الخارجية — the third-party entries the registry ships knowing about.
//!
//! One function per entry, compiled in rather than read off disk, because these
//! are measurements: an artifact's size and digest were taken once, from bytes
//! that no longer exist anywhere, and a file a maintainer can edit is a pin a
//! maintainer can edit. The cast reads them through
//! [`crate::sabk::madkhal_khariji`] like any other entry and refuses them on
//! exactly the same terms.
//!
//! **Every seed ships with an empty permission statement.** `IdhnMasdar::Katabi
//! { bayan }` is blank here and is meant to stay blank until the owner has
//! actually asked the author and has somewhere to point at for the answer.
//! Writing a plausible sentence into it would be inventing the one fact the
//! whole gate rests on, so the seed is deliberately a thing that does not
//! publish: [`crate::sabk::madkhal_khariji`] refuses it, by name, until
//! somebody fills the statement in.

use std::collections::BTreeMap;

use taarib_mustalahat::khariji::{
    FapsBina, HalatMira, QitaatTanzeel, RuqaaKharijiya, TahdheerKhariji, TahdidBina, TakhtitKhariji,
};
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{IdhnMasdar, MasdarKhariji, RukhsaRuqaa, RuqaaId};
use uuid::Uuid;

/// The lineage RTEA is listed under.
///
/// Written out rather than minted, because the identity is what a client's
/// installed-patch record, its update check and its uninstall all key on: a
/// fresh one on every cast would make every machine see a second patch it has
/// never installed, beside the one it has.
const HAWIYAT_RTEA: Uuid = Uuid::from_u128(0x0199_97a0_7e4a_7b21_9f3c_5a1d_0c8e_4b60);

/// Red Dead Redemption 2, as Steam identifies it.
///
/// One identity across the Rockstar launcher and Epic too: `hawiyat_manassa`
/// carries the names those launchers use, and the game is the same game.
const TATBEEQ_STEAM_RDR2: u32 = 1_174_180;

/// Every third-party entry this build knows about.
///
/// What `--badhra-kharijiya` writes out, and the list a fixture walks. Each one
/// ships unpublishable; see the module header for why that is the point.
#[must_use]
pub fn badhrat_kharijiya() -> Vec<RuqaaKharijiya> {
    vec![badhrat_rtea()]
}

/// RTEA — the Arabic translation of Red Dead Redemption 2 by Emad Adel and the
/// Redemption Team, version 1.7.
///
/// The artifacts, their sizes and their digests were measured against the
/// release of 2026-09-11 and the author's own endpoints; the archives they were
/// measured from have since been deleted, so these digests are the pins and a
/// byte that disagrees with them is refused by name rather than installed.
///
/// `tahdid_bina` is the one field here that was **not** measured, and its own
/// comment says why and what a correction costs.
///
/// The entry fetches from the author. Mirroring is off — the repository
/// declares no licence at all, which reserves every right, and nobody has
/// asked about hosting the file.
#[must_use]
pub fn badhrat_rtea() -> RuqaaKharijiya {
    RuqaaKharijiya {
        id: RuqaaId::min_uuid(HAWIYAT_RTEA),
        unwan: "RTEA".to_owned(),
        luba: LubaId::min_masdar(
            &MasdarLuba::Steam(TATBEEQ_STEAM_RDR2),
            "Red Dead Redemption 2",
        ),
        hawiyat_manassa: vec!["Red Dead Redemption 2".to_owned()],
        abniya: vec!["1311".to_owned(), "1436".to_owned(), "1491".to_owned()],
        // **Not measured against an install.** RDR2 is on no machine this was
        // written on — no `RDR2.exe`, no `appmanifest_1174180` — so this is
        // written from how Rockstar versions the game rather than from a file
        // anybody here opened. Declaring it instead of hardcoding it is what
        // makes being wrong cheap: a wrong `masar` or `juz` is corrected here
        // and every client picks the correction up on its next index refresh,
        // with no build and no release.
        //
        // No `tanazur`. Steam's `buildid` moves per depot and per branch, so a
        // correspondence nobody verified would resolve *confidently* to the
        // wrong build — strictly worse than the `Majhula` the card already has
        // words for.
        tahdid_bina: TahdidBina {
            tanazur: BTreeMap::new(),
            faps: Some(FapsBina::MawridIsdar {
                masar: "RDR2.exe".to_owned(),
                juz: 2,
            }),
        },
        masdar: MasdarKhariji {
            ism: "Emad Adel".to_owned(),
            rabt: "https://github.com/emadadeldev/rtea".to_owned(),
            // The GitHub API answers `license: null` for this repository. No
            // licence file is not a permissive licence; it is the absence of
            // one, and the absence of one reserves every right.
            rukhsa: RukhsaRuqaa::MilkiyaKhassa,
            // Left empty on purpose. The owner writes where the author granted
            // this and when, and until they do the gate refuses the entry.
            idhn: IdhnMasdar::Katabi {
                bayan: String::new(),
            },
            yasmah_bilmira: false,
        },
        fariq: Some("Redemption Team".to_owned()),
        isdar: "1.7".to_owned(),
        qitaa: vec![
            QitaatTanzeel {
                ism: "update.zip".to_owned(),
                rabt: "https://rt.emadadeldev.workers.dev/?lml-update".to_owned(),
                hajm: 25_578_943,
                sha256: "ffd54019f146b8db8a02d6413c592222ae9de397c522631d3b65fe882b3e1343"
                    .to_owned(),
                abniya: Vec::new(),
            },
            QitaatTanzeel {
                ism: "extra.zip".to_owned(),
                rabt: "https://rt.emadadeldev.workers.dev/?lml-extra".to_owned(),
                hajm: 1_541_075,
                sha256: "82d3008d4d77066a4336a853861603ef50b3c5d64d4cfe45f52236156ea41715"
                    .to_owned(),
                // 1491 takes update.zip alone; fetching this into it would
                // write a loader set that build does not take.
                abniya: vec!["1311".to_owned(), "1436".to_owned()],
            },
        ],
        takhtit: TakhtitKhariji {
            yaktub: vec![
                "dinput8.dll".to_owned(),
                "lml".to_owned(),
                "lml.ini".to_owned(),
                "ModManager.Core.dll".to_owned(),
                "ModManager.NativeInterop.dll".to_owned(),
                "NLog.dll".to_owned(),
                "README.txt".to_owned(),
                "ScriptHookRDR2.dll".to_owned(),
                "vfs.asi".to_owned(),
            ],
            // The author's own HOW-TO, in the author's own spelling — the odd
            // casing and the extensionless entries included, because this list
            // is matched against what is on somebody's disk and a tidied-up
            // copy would miss what the untidy original names. Their
            // instructions say delete; Taarib backs every one of these up
            // first and puts it back byte-identical on uninstall.
            yahdhif: vec![
                "lml".to_owned(),
                "redteamassets".to_owned(),
                "mod backup".to_owned(),
                "asiloader".to_owned(),
                "dinput8".to_owned(),
                "installscript".to_owned(),
                "installscript_sdk".to_owned(),
                "redteamload.dll".to_owned(),
                "version.dll".to_owned(),
                "vfs".to_owned(),
                "lml.ini".to_owned(),
                "Nlog".to_owned(),
                "ModManager.core.dll".to_owned(),
                "ModManager.log".to_owned(),
                "ModManager.NativeInterop.dll".to_owned(),
                "NativeTrainer.asi".to_owned(),
                "ScriptHookRDR2.dll".to_owned(),
                "ScriptHookRDR2.log".to_owned(),
                "vfs.asi".to_owned(),
                "vfs.log".to_owned(),
            ],
        },
        tahdheerat: vec![TahdheerKhariji {
            arabi: "هذه الرقعة لطور القصة وحده. «ريد ديد أونلاين» يعمل عليه نظام مكافحة غشّ، \
                    واللعب فيه بملفّات لعبة معدَّلة يعرّض حسابك للحظر؛ أزل الرقعة قبل الدخول \
                    إليه."
                .to_owned(),
            injilizi: "This patch is for story mode alone. Red Dead Online runs an anti-cheat, \
                       and playing it with modified game files puts your account at risk of a \
                       ban; remove the patch before going online."
                .to_owned(),
        }],
        mira: HalatMira::MinAlmuallif,
        rabt_safha: Some("https://emadadeldev.github.io/rtea/".to_owned()),
        rabt_tahdith: Some(
            "https://raw.githubusercontent.com/emadadeldev/rtea/main/api.json".to_owned(),
        ),
    }
}

#[cfg(test)]
mod fuhus {
    use std::error::Error;

    use taarib_mustalahat::khariji::{FapsBina, SababRafdKhariji};
    use taarib_mustalahat::ruqaa::IdhnMasdar;

    use super::badhrat_rtea;

    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// The seed ships unpublishable, and that is the point: the one fact the
    /// gate rests on is the author's own word, and nobody here has it yet.
    #[test]
    fn albadhra_tushhan_bila_bayan() {
        let badhra = badhrat_rtea();
        assert_eq!(
            badhra.sabab_rafd(),
            Some(SababRafdKhariji::BayanFarigh),
            "the seed must not ship with a permission statement somebody invented"
        );
        assert!(!badhra.yajuz_nashruha());
        assert!(!badhra.yajuz_mira());
    }

    /// The measurements, which are the reason this is compiled in rather than
    /// read off a file anybody can edit.
    #[test]
    fn almaqaees_mathbuta() -> NatijatIkhtibar {
        let badhra = badhrat_rtea();
        assert_eq!(badhra.nasab(), "Emad Adel · Redemption Team");
        assert_eq!(badhra.isdar, "1.7");
        assert!(badhra.qitaa_bila_basma().is_none());

        let tahdith = badhra
            .qitaa
            .iter()
            .find(|qitaa| qitaa.ism == "update.zip")
            .ok_or("the seed carries no update.zip")?;
        assert_eq!(tahdith.hajm, 25_578_943);
        assert_eq!(
            tahdith.sha256,
            "ffd54019f146b8db8a02d6413c592222ae9de397c522631d3b65fe882b3e1343"
        );

        let idafi = badhra
            .qitaa
            .iter()
            .find(|qitaa| qitaa.ism == "extra.zip")
            .ok_or("the seed carries no extra.zip")?;
        assert_eq!(idafi.hajm, 1_541_075);
        assert_eq!(
            idafi.sha256,
            "82d3008d4d77066a4336a853861603ef50b3c5d64d4cfe45f52236156ea41715"
        );

        // 1491 takes update.zip alone, and the other two take both.
        assert_eq!(badhra.qitaa_li_bina("1491").len(), 1);
        assert_eq!(badhra.qitaa_li_bina("1311").len(), 2);
        assert_eq!(badhra.qitaa_li_bina("1436").len(), 2);
        assert!(!badhra.yadam_bina("1207"));
        Ok(())
    }

    /// RDR2 is versioned by its own executable, so the seed says how to read it
    /// and says nothing about Steam — the correspondence nobody here measured
    /// is the one an entry must not invent.
    #[test]
    fn albadhra_tunassu_ala_faps_bila_tanazur() -> NatijatIkhtibar {
        let badhra = badhrat_rtea();
        assert!(
            badhra.tahdid_bina.tanazur.is_empty(),
            "a Steam buildid correspondence would be a claim nobody verified"
        );
        let faps = badhra
            .tahdid_bina
            .faps
            .as_ref()
            .ok_or("the seed states no way to read the installed build")?;
        let FapsBina::MawridIsdar { masar, juz } = faps;
        assert_eq!(masar, "RDR2.exe");
        // `1.0.1491.50`, counting from zero, is the 1491 the entry pins for.
        assert_eq!(*juz, 2);
        assert!(badhra.yadam_bina("1491"));
        Ok(())
    }

    /// Both languages carry the same warning, because the screen that shows it
    /// shows one or the other and a player reading either has to be told about
    /// Red Dead Online.
    #[test]
    fn altahdheer_bilughatayn() -> NatijatIkhtibar {
        let badhra = badhrat_rtea();
        let tahdheer = badhra
            .tahdheerat
            .first()
            .ok_or("the seed states no safety warning")?;
        assert!(tahdheer.injilizi.contains("Red Dead Online"));
        assert!(tahdheer.injilizi.contains("story mode"));
        assert!(tahdheer.arabi.contains("ريد ديد أونلاين"));
        assert!(tahdheer.arabi.contains("طور القصة"));
        Ok(())
    }

    /// Everything the author's HOW-TO says to delete is recorded, so the
    /// installer can back it up first — which is the whole reason a user would
    /// take this patch through Taarib rather than through its own installer.
    #[test]
    fn qaimat_alhadhf_kamila() {
        let badhra = badhrat_rtea();
        for masar in [
            "lml",
            "redteamassets",
            "mod backup",
            "version.dll",
            "ScriptHookRDR2.dll",
            "vfs.log",
        ] {
            assert!(
                badhra.takhtit.yahdhif.iter().any(|wahid| wahid == masar),
                "{masar} is on the author's remove list and not on this one"
            );
        }
        assert_eq!(badhra.takhtit.yahdhif.len(), 20);
        assert!(badhra.takhtit.yasmah("lml/RTEA/Subtitles/texts/a.yldb"));
        assert!(!badhra.takhtit.yasmah("redteamassets/x"));
    }

    /// Filling the statement in is all it takes, which is what makes the empty
    /// one a decision somebody has to make rather than a wall.
    #[test]
    fn bayan_haqiqi_yafutuh_albawwaba() {
        let mut badhra = badhrat_rtea();
        badhra.masdar.idhn = IdhnMasdar::Katabi {
            bayan: "granted by the author on X, 2026-09-21, https://x.example/post/1".to_owned(),
        };
        assert_eq!(badhra.sabab_rafd(), None);
        assert!(badhra.yajuz_nashruha());
    }
}
