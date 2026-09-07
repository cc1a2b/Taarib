//! The core answered against the games actually installed on the machine it was written on.

use std::error::Error;
use std::path::{Path, PathBuf};

use taarib_aman::kashf_himaya::{
    DaleelHimaya, HalatMatjar, IjmaaHimaya, NawDaleel as NawDaleelHimaya, NawHimaya, Thiqa,
};
use taarib_aman::kashf_shabaka::{
    DalalatShabaka, DaleelShabaka, IjmaaShabaka, NawDaleel as NawDaleelShabaka,
};
use taarib_aql::{
    Aql, HalatFahs, HalatHimaya, HalatMakhzan, HalatMash, HalatMustawda, HuwiyatLuba, Mani,
    MasahAman, MasahWukala, MasdarMarifa, MawqifMustakhdim, MudkhalatAql, Muntaj, NawKhatar,
    NawMani, NitaqMani,
};
use taarib_kashf::fahs::SimatLuba;
use taarib_muharrik::imkaniyat::taqreer;
use taarib_mustalahat::luba::{
    DaleelLugha, HukmLughaRasmiya, LubaId, MasdarLuba, NawDaleelLugha, TughtiyaLugha,
};
use taarib_mustalahat::muharrik::{
    AilatMuharrik, Daleel, KhalfiyaBarmajiya, Muharrik, NawDaleel, TaqreerImkaniyat, WajihaRusum,
};
use taarib_usus::idadat::HalatMuzawwidin;
use taarib_usus::manassa::Mimariya;

/// Every test returns this so that a fixture failure propagates with `?`.
/// `unwrap` and `expect` are denied workspace-wide, tests included.
type NatijatIkhtibar = Result<(), Box<dyn Error>>;

/// The instant every fixture report is stamped with, so nothing here reads a
/// clock and two runs produce the same values.
const WAQT: &str = "2026-09-06T00:00:00Z";

// ---------------------------------------------------------------------------
// Fixtures, each one a game from the sweep of this machine's library
// ---------------------------------------------------------------------------

/// An engine as `tahdid::hall` resolved it, with the confidence the sweep read.
const fn muharrik(aila: AilatMuharrik, thiqa: u8, rusum: Vec<WajihaRusum>) -> Muharrik {
    Muharrik {
        aila,
        isdar: None,
        khalfiya: KhalfiyaBarmajiya::Majhula,
        itarat: Vec::new(),
        rusum,
        mimariya: Mimariya::X8664,
        thiqa,
        dalail: Vec::new(),
    }
}

/// One observation, as a detector writes it.
fn daleel(wasf: &str, wazn: u8) -> Daleel {
    Daleel {
        naw: NawDaleel::TawqiThunai,
        wasf: wasf.to_owned(),
        mawqi: None,
        wazn,
    }
}

/// ELDEN RING as the sweep found it: Majhul at 49, D3D12 and DXGI from the
/// import table, and Easy Anti-Cheat under Epic Online Services on disk.
fn eldenring_muharrik() -> Muharrik {
    let mut asas = muharrik(AilatMuharrik::Majhul, 49, vec![WajihaRusum::D3d12]);
    asas.dalail.push(daleel("imports d3d12.dll", 60));
    asas.dalail.push(daleel("imports dxgi.dll", 30));
    asas
}

/// The anti-cheat evidence ELDEN RING's own folder produces.
fn himaya_eldenring(jidhr: &Path) -> IjmaaHimaya {
    IjmaaHimaya {
        jidhr: jidhr.to_path_buf(),
        adilla: vec![
            DaleelHimaya {
                naw: NawHimaya::EasyAntiCheat,
                sinf: NawDaleelHimaya::MalafMawjud,
                ayn: "EasyAntiCheat/EasyAntiCheat_x64.dll".to_owned(),
                masar: Some(jidhr.join("EasyAntiCheat").join("EasyAntiCheat_x64.dll")),
                thiqa: Thiqa::Muakkada,
            },
            DaleelHimaya {
                naw: NawHimaya::EasyAntiCheatEos,
                sinf: NawDaleelHimaya::WahdaMuhammala,
                ayn: "eossdk-win64-shipping.dll".to_owned(),
                masar: Some(jidhr.join("Game").join("eldenring.exe")),
                thiqa: Thiqa::Muakkada,
            },
        ],
        thughrat: Vec::new(),
        mabtur: false,
    }
}

/// An anti-cheat scan that ran, read the catalogue, and matched nothing.
///
/// Named for the empty evidence rather than for emptiness, because the two are
/// different states and only one of them is this one.
fn himaya_bila_adilla(jidhr: &Path) -> IjmaaHimaya {
    IjmaaHimaya {
        jidhr: jidhr.to_path_buf(),
        adilla: Vec::new(),
        thughrat: Vec::new(),
        mabtur: false,
    }
}

/// A multiplayer scan that ran and found nothing.
fn shabaka_bila_dalail(jidhr: &Path) -> IjmaaShabaka {
    IjmaaShabaka {
        jidhr: jidhr.to_path_buf(),
        dalail: Vec::new(),
        thughrat: Vec::new(),
        mabtur: false,
    }
}

/// The identity of one Steam game, with no Steam root needed.
fn huwiya(appid: u32, ism: &str, simat: Vec<SimatLuba>) -> HuwiyatLuba {
    let masdar = MasdarLuba::Steam(appid);
    let jidhr = PathBuf::from(format!("/mnt/f/SteamLibrary/steamapps/common/{ism}"));
    HuwiyatLuba {
        id: LubaId::min_masdar(&masdar, ism),
        ism: ism.to_owned(),
        masadir: vec![masdar],
        jidhr,
        tanfidhi: None,
        jidhr_steam: Some(PathBuf::from("/mnt/d/Program Files (x86)/Steam")),
        simat,
        muktamila: true,
        hala_matjar: None,
    }
}

/// The inputs for one game, with every scan answering "ran and found nothing"
/// unless a test replaces it.
fn mudkhalat(huwiya: HuwiyatLuba, fahs: HalatFahs) -> MudkhalatAql {
    let jidhr = huwiya.jidhr.clone();
    MudkhalatAql {
        huwiya,
        fahs,
        aman: MasahAman {
            himaya: himaya_bila_adilla(&jidhr),
            halat_matjar: HalatMatjar::Maqru,
            shabaka: shabaka_bila_dalail(&jidhr),
            hala: HalatMash::Jara,
        },
        lugha: None,
        wukala: MasahWukala {
            jidhr: jidhr.clone(),
            qaima: Vec::new(),
            thughra: None,
        },
        makhzan: HalatMakhzan::ghayr_mafhus(&jidhr),
        mustawda: None,
        mawqif: MawqifMustakhdim::default(),
    }
}

/// The official-Arabic verdict for a game whose publisher ships the lot.
fn lugha_kamila() -> HukmLughaRasmiya {
    HukmLughaRasmiya {
        wajiha: TughtiyaLugha::Muakkada,
        nusus: TughtiyaLugha::Muakkada,
        thiqa: 90,
        dalail: vec![DaleelLugha {
            naw: NawDaleelLugha::MawridMuharrik,
            wasf: "ar culture holds 345 entries against a 350-entry reference".to_owned(),
            mawqi: Some("Content/Localization/Game/ar/Game.locres".to_owned()),
            wazn: 90,
        }],
        majhul: Vec::new(),
        lughat_muallana: vec!["arabic".to_owned()],
        isdar_fahs: 1,
        waqt: WAQT.to_owned(),
    }
}

/// Builds the real capability report for one engine and one set of launcher
/// hints, through the producer rather than by hand.
fn taqreer_min(muharrik: Muharrik, simat: &[SimatLuba]) -> TaqreerImkaniyat {
    taqreer(muharrik, simat, WAQT.to_owned())
}

// ---------------------------------------------------------------------------
// The worked example
// ---------------------------------------------------------------------------

/// ELDEN RING: refused, and every surface refuses it for the same reason.
///
/// Three answers could apply at once here — the hard scan finds Easy Anti-Cheat,
/// the capability report refuses on the launcher's hint, and the engine is
/// unrecognised so the overlay tier's adapter is unfinished. The product used to
/// name whichever of those the surface happened to reach first.
#[test]
fn eldenring_yurfad_bi_sabab_wahid_fi_kul_ijaba() -> NatijatIkhtibar {
    let huwiya = huwiya(
        1_245_620,
        "ELDEN RING",
        vec![SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())],
    );
    let jidhr = huwiya.jidhr.clone();
    let taqreer = taqreer_min(eldenring_muharrik(), &huwiya.simat);
    // The producer's own answer, before this crate touches it.
    assert!(
        taqreer.marfuda,
        "the capability report already refuses this game"
    );

    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman.himaya = himaya_eldenring(&jidhr);
    let aql = Aql::jadeed(mudkhalat);

    // 1. Which product, and why.
    let muntaj = aql.muntaj();
    assert_eq!(muntaj.qeema, Muntaj::LaShay);
    assert_eq!(
        muntaj.qeema.raqm(),
        None,
        "a refused game has no tier number to print"
    );

    // 2. The blockers, in the one order.
    let mawani = aql.mawani();
    let awwal = mawani
        .first()
        .ok_or("a refused game has at least one blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::Himaya);
    assert!(
        awwal.qeema.nihai(),
        "anti-cheat is a fact no release changes"
    );

    // 3. Every other answer names the same blocker, word for word.
    let waad = aql.waad();
    assert_eq!(waad.qeema.sabab_arabi, awwal.qeema.arabi);
    assert_eq!(waad.qeema.sabab_injilizi, awwal.qeema.injilizi);
    let hudud = aql.hudud();
    assert_eq!(
        hudud.len(),
        1,
        "a refused game carries one sentence, not the engine's list"
    );
    let hadd = hudud.first().ok_or("one limit")?;
    assert_eq!(hadd.qeema.injilizi, awwal.qeema.injilizi);

    // 4. The sentence is the safety layer's own and it names the evidence.
    assert!(
        awwal.qeema.injilizi.contains("Easy Anti-Cheat"),
        "the refusal names what was found: {}",
        awwal.qeema.injilizi
    );
    assert!(
        awwal.qeema.injilizi.contains("permanently ban"),
        "{}",
        awwal.qeema.injilizi
    );
    assert!(
        !awwal.qeema.arabi.is_empty(),
        "and it says so in Arabic too"
    );

    // 5. And it can say which input produced it — both of them.
    assert!(
        awwal.min(MasdarMarifa::KashfHimaya),
        "the disk scan is in the chain"
    );
    assert!(
        awwal.min(MasdarMarifa::Imkaniyat),
        "so is the capability report"
    );
    assert!(
        awwal.min(MasdarMarifa::Iktishaf),
        "and so is the launcher hint behind it"
    );
    let ayn: Vec<&str> = awwal
        .shawahid
        .iter()
        .map(|shahid| shahid.wasf.as_str())
        .collect();
    assert!(
        ayn.iter()
            .any(|wasf| wasf.contains("EasyAntiCheat_x64.dll")),
        "the chain names the file on disk: {ayn:?}"
    );

    // 6. The readiness gap is real and is not the sentence a refused game gets.
    assert_eq!(aql.hala_himaya().qeema, HalatHimaya::Mahmiya);
    assert!(!aql.jahiz_lil_tathbeet());
    Ok(())
}

// ---------------------------------------------------------------------------
// The divergence this crate exists to prevent
// ---------------------------------------------------------------------------

/// Three blockers standing at once produce one reason, and it is the same one
/// from every question.
///
/// This is the `hukm`/`ibda` defect written as an assertion: one command named
/// anti-cheat and the other named the publisher's own Arabic for the same game,
/// because each applied its own order.
#[test]
fn thalathat_mawani_maan_tuti_sababan_wahidan() -> NatijatIkhtibar {
    let huwiya = huwiya(
        1_245_620,
        "ELDEN RING",
        vec![SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())],
    );
    let jidhr = huwiya.jidhr.clone();
    let taqreer = taqreer_min(eldenring_muharrik(), &huwiya.simat);
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman.himaya = himaya_eldenring(&jidhr);
    mudkhalat.lugha = Some(lugha_kamila());
    let aql = Aql::jadeed(mudkhalat);

    let mawani = aql.mawani();
    let anwa: Vec<NawMani> = mawani.iter().map(|mani| mani.qeema.naw).collect();
    assert!(anwa.contains(&NawMani::Himaya), "{anwa:?}");
    assert!(anwa.contains(&NawMani::LughaRasmiya), "{anwa:?}");

    // Nothing is hidden by being outranked, and the order is not the caller's.
    let rutab: Vec<u8> = mawani.iter().map(|mani| mani.qeema.rutba()).collect();
    let mut murattaba = rutab.clone();
    murattaba.sort_unstable();
    assert_eq!(
        rutab, murattaba,
        "the list arrives sorted by the one ordering"
    );

    // Every question answers with the top blocker, not with whichever one it
    // happened to evaluate first.
    let awwal = mawani.first().ok_or("at least one blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::Himaya);
    assert_eq!(aql.waad().qeema.sabab_injilizi, awwal.qeema.injilizi);
    assert_eq!(
        aql.mani_awwal().ok_or("a top blocker")?.qeema.naw,
        NawMani::Himaya
    );
    assert_eq!(
        aql.hudud().first().ok_or("one limit")?.qeema.injilizi,
        awwal.qeema.injilizi
    );
    Ok(())
}

/// GTA V Enhanced: `BattlEye` on disk, refused, and the reason names it.
#[test]
fn gta_yurfad_bi_battleye() -> NatijatIkhtibar {
    let huwiya = huwiya(3_240_220, "Grand Theft Auto V Enhanced", Vec::new());
    let jidhr = huwiya.jidhr.clone();
    // The sweep read no graphics module out of this executable at all: the
    // renderer is loaded dynamically and D3D12 came off a shipped redistributable.
    let taqreer = taqreer_min(muharrik(AilatMuharrik::Majhul, 37, Vec::new()), &[]);
    assert!(
        !taqreer.marfuda,
        "the launcher hint says nothing about this one"
    );

    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman.himaya = IjmaaHimaya {
        jidhr: jidhr.clone(),
        adilla: vec![DaleelHimaya {
            naw: NawHimaya::BattlEye,
            sinf: NawDaleelHimaya::MalafMawjud,
            ayn: "BattlEye/BEService_x64.exe".to_owned(),
            masar: Some(jidhr.join("BattlEye").join("BEService_x64.exe")),
            thiqa: Thiqa::Muakkada,
        }],
        thughrat: Vec::new(),
        mabtur: false,
    };
    let aql = Aql::jadeed(mudkhalat);

    // The capability report does not refuse this game and the core does, because
    // the core holds an input the report never sees.
    assert_eq!(aql.muntaj().qeema, Muntaj::LaShay);
    let awwal = aql.mani_awwal().ok_or("a blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::Himaya);
    assert!(
        awwal.qeema.injilizi.contains("BattlEye"),
        "{}",
        awwal.qeema.injilizi
    );
    assert!(awwal.min(MasdarMarifa::KashfHimaya));
    assert!(
        !awwal.min(MasdarMarifa::Imkaniyat),
        "the report had nothing to contribute here"
    );
    Ok(())
}

/// The two anti-cheat sources naming different things is visible, not averaged.
#[test]
fn masdaray_al_himaya_idha_ikhtalafa_zahara() -> NatijatIkhtibar {
    let huwiya = huwiya(
        1_245_620,
        "ELDEN RING",
        vec![SimatLuba::HimayaMuhtamala("BattlEye".to_owned())],
    );
    let jidhr = huwiya.jidhr.clone();
    let taqreer = taqreer_min(eldenring_muharrik(), &huwiya.simat);
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman.himaya = himaya_eldenring(&jidhr);
    let aql = Aql::jadeed(mudkhalat);

    let ikhtilaf = aql
        .ikhtilaf_himaya()
        .ok_or("the two sources disagree here")?;
    assert!(
        ikhtilaf
            .min_kashf
            .iter()
            .any(|ism| ism.contains("Easy Anti-Cheat"))
    );
    assert_eq!(ikhtilaf.min_iktishaf, vec!["BattlEye".to_owned()]);
    // The refusal still stands: both are evidence for refusing.
    assert_eq!(aql.muntaj().qeema, Muntaj::LaShay);
    Ok(())
}

// ---------------------------------------------------------------------------
// The gate that skipped silently
// ---------------------------------------------------------------------------

/// A Steam game with no Steam root is refused, not permitted.
///
/// The evidence list here is byte-identical to a clean game's — that identity is
/// the whole defect — so the answer comes off [`HalatMatjar`] instead.
#[test]
fn luba_steam_bila_jidhr_turfad_wa_la_tumarrar() -> NatijatIkhtibar {
    let mut huwiya = huwiya(945_360, "Among Us", Vec::new());
    huwiya.jidhr_steam = None;
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Unity, 99, vec![WajihaRusum::OpenGl]),
        &[],
    );
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman.halat_matjar = HalatMatjar::JidhrMajhul;
    let aql = Aql::jadeed(mudkhalat);

    assert_eq!(aql.hala_himaya().qeema, HalatHimaya::LamYajri);
    assert!(
        !aql.hala_himaya().qeema.yasmah(),
        "an unrunnable check is not a passed check"
    );
    let awwal = aql.mani_awwal().ok_or("a blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::FahsHimayaLamYajri);
    assert!(awwal.min(MasdarMarifa::FahrasMatjar));
    assert!(
        awwal
            .qeema
            .injilizi
            .contains("silence here is not a clean result"),
        "{}",
        awwal.qeema.injilizi
    );
    assert_eq!(aql.muntaj().qeema, Muntaj::LaShay);
    Ok(())
}

/// A scan that ran and matched nothing says exactly that, and not "clean".
///
/// `NawHimaya` is a closed set, so an anti-cheat this build has no signature for
/// produces the same empty list a clean game does. The answer is worded as a
/// claim about Taarib's list, and it carries the scan's own coverage limits.
#[test]
fn la_tawqee_laysa_naqiyan() -> NatijatIkhtibar {
    let huwiya = huwiya(3_405_690, "FC 26", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Majhul, 54, vec![WajihaRusum::D3d12]),
        &[],
    );
    let aql = Aql::jadeed(mudkhalat(huwiya, HalatFahs::mafhusa(taqreer)));

    let hala = aql.hala_himaya();
    assert_eq!(hala.qeema, HalatHimaya::LaTawqee);
    assert!(hala.qeema.yasmah());
    let wasf = &hala
        .shawahid
        .first()
        .ok_or("the negative answer carries its own chain")?
        .wasf;
    assert!(
        wasf.contains("no signature in this build's list matched"),
        "the claim is about the signature list, not about the game: {wasf}"
    );
    Ok(())
}

/// A scan that never ran is refused, and is not mistaken for one that passed.
///
/// The sibling of [`la_tawqee_laysa_naqiyan`], and the more dangerous half. That
/// test pins what a scan says when it runs; this one pins that the absence of a
/// scan is not that answer. Both inputs carry an empty evidence list, an empty
/// gap list and `mabtur: false` — the bytes are identical — so the only thing
/// telling them apart is [`HalatMash`], and the only thing that makes it matter
/// is that the decision reads it. `LaTawqee` is the sole verdict
/// [`HalatHimaya::yasmah`] admits, and before this the un-scanned game got it.
#[test]
fn al_mash_alladhi_lam_yajri_yurfad() -> NatijatIkhtibar {
    let huwiya = huwiya(3_405_690, "FC 26", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Frostbite, 95, vec![WajihaRusum::D3d12]),
        &[],
    );
    let jidhr = huwiya.jidhr.clone();
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman = MasahAman::lam_yumsah(&jidhr);
    let aql = Aql::jadeed(mudkhalat);

    let hala = aql.hala_himaya();
    assert_eq!(
        hala.qeema,
        HalatHimaya::LamYajri,
        "an un-run scan is not a scan that matched nothing"
    );
    assert!(!hala.qeema.yasmah(), "and nothing may be installed on it");

    let mani = aql
        .mawani()
        .into_iter()
        .find(|mani| mani.qeema.naw == NawMani::FahsHimayaLamYajri)
        .ok_or("the refusal reaches the blocker list, not only the screen")?;
    assert_eq!(
        mani.qeema.nitaq(),
        NitaqMani::Kul,
        "it stops a hand-installed patch too, not only the automatic run"
    );
    assert!(
        mani.qeema
            .injilizi
            .contains("no anti-cheat scan has been run"),
        "the sentence names the real cause: {}",
        mani.qeema.injilizi
    );
    assert!(
        !mani.qeema.arabi.is_empty(),
        "and it exists in Arabic, which is the language the product speaks"
    );
    assert!(aql.marfuda(), "the game is refused outright");
    assert_eq!(aql.muntaj().qeema, Muntaj::LaShay);
    Ok(())
}

/// The un-run scan and the unreadable catalogue are one blocker, not two.
///
/// Both are "the check did not run" and both carry `NawMani::FahsHimayaLamYajri`.
/// A game in both states at once must still get one sentence, because a screen
/// with room for one reason shows the first and a second copy of the same
/// heading with a different body is how a user stops trusting either.
#[test]
fn sababa_lam_yajri_la_yatakarraran() {
    let huwiya = huwiya(3_405_690, "FC 26", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Frostbite, 95, vec![WajihaRusum::D3d12]),
        &[],
    );
    let jidhr = huwiya.jidhr.clone();
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    let mut aman = MasahAman::lam_yumsah(&jidhr);
    aman.halat_matjar = HalatMatjar::JidhrMajhul;
    mudkhalat.aman = aman;
    let aql = Aql::jadeed(mudkhalat);

    let adad = aql
        .mawani()
        .iter()
        .filter(|mani| mani.qeema.naw == NawMani::FahsHimayaLamYajri)
        .count();
    assert_eq!(adad, 1, "one cause reaches the screen, not both");
}

/// No provider configured blocks the automatic run and nothing else.
///
/// The scope is the whole finding. A machine with no provider can still install
/// a published patch — the patch is already translated and never reaches a
/// provider — so a blocker that stopped everything would be a lie in the
/// expensive direction, telling most users the product does not work for them.
/// `NitaqMani::Tashghil` is what says that, and this pins it.
///
/// It also pins where the sentence comes from. Before this the automatic-run
/// command composed its own, so that screen said it and the game screen, reading
/// the same core, said nothing about the same machine.
#[test]
fn la_muzawwid_yamna_al_tashghil_wahdah() -> NatijatIkhtibar {
    let huwiya = huwiya(367_520, "Hollow Knight", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Renpy, 99, vec![WajihaRusum::OpenGl]),
        &[],
    );
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.mawqif.muzawwidun = Some(HalatMuzawwidin::Faragh);
    let aql = Aql::jadeed(mudkhalat);

    let mani = aql
        .mawani()
        .into_iter()
        .find(|mani| mani.qeema.naw == NawMani::LaMuzawwid)
        .ok_or("an unconfigured provider list reaches the blocker list")?;
    assert_eq!(
        mani.qeema.nitaq(),
        NitaqMani::Tashghil,
        "it stops the automatic run and not a hand-installed patch"
    );
    assert!(!mani.qeema.nihai(), "and it is not a permanent fact");
    assert!(
        !aql.marfuda(),
        "so the game itself is not refused for want of a provider"
    );
    assert!(
        mani.qeema.injilizi.contains("published patches"),
        "the sentence says what still works: {}",
        mani.qeema.injilizi
    );
    assert!(mani.min(MasdarMarifa::Idadat));
    Ok(())
}

/// A question nobody put is not a blocker.
///
/// `MawqifMustakhdim::default()` is a real input — `MudkhalatAql::ijma` uses it
/// — so a default that asserted "no provider is configured" would put this
/// blocker on every game assembled without the settings. That is the same
/// mistake as reporting an un-run scan as a clean one, in a cheaper place.
#[test]
fn muzawwid_ghayr_masul_anhu_laysa_maniyan() {
    let huwiya = huwiya(367_520, "Hollow Knight", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Renpy, 99, vec![WajihaRusum::OpenGl]),
        &[],
    );
    let mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    assert_eq!(
        mudkhalat.mawqif.muzawwidun, None,
        "the fixture leaves the question unasked, as `default` does"
    );
    let aql = Aql::jadeed(mudkhalat);
    assert!(
        !aql.mawani()
            .iter()
            .any(|mani| mani.qeema.naw == NawMani::LaMuzawwid),
        "and an unasked question produces no blocker"
    );
}

/// A multiplayer scan that could not finish asks for consent anyway.
///
/// The anti-cheat answer beside this one has always carried its own coverage
/// limits; the multiplayer answer threw them away, and returned "no risk" on the
/// strength of a walk that stopped early. The markers are a closed set and the
/// walk has a bound, so a title with its own netcode and no store categories
/// produced exactly the silence a single-player game produces — and the
/// acknowledgement this risk exists to collect was never asked for.
///
/// Asking costs a tick. Not asking costs a ban from a server that checks the
/// files its players are running.
#[test]
fn shabaka_mabtura_tastadhin() -> NatijatIkhtibar {
    let huwiya = huwiya(2_567_870, "Chained Together", Vec::new());
    let jidhr = huwiya.jidhr.clone();
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Unreal, 99, vec![WajihaRusum::D3d11]),
        &[],
    );
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    // No evidence at all, and a walk that stopped at its bound — byte-identical
    // to a finished single-player scan but for this one flag.
    mudkhalat.aman.shabaka.mabtur = true;
    let aql = Aql::jadeed(mudkhalat);

    let khatar = aql
        .makhatir()
        .into_iter()
        .find(|khatar| khatar.qeema.naw == NawKhatar::LaabJamai)
        .ok_or("a truncated multiplayer scan asks rather than staying silent")?;
    assert!(!khatar.qeema.muqarr, "and it is unanswered until answered");
    assert!(
        khatar.qeema.injilizi.contains("stopped at a bound"),
        "the sentence says the scan did not finish, not that the game is \
         multiplayer: {}",
        khatar.qeema.injilizi
    );
    assert!(
        !khatar.qeema.arabi.is_empty(),
        "and it exists in Arabic too"
    );
    assert!(
        !aql.jahiz_lil_tathbeet(),
        "an unanswered risk holds the install"
    );
    Ok(())
}

/// A finished scan that found nothing still asks nothing.
///
/// The other half of [`shabaka_mabtura_tastadhin`], and the one that keeps the
/// fix from being "prompt everybody". A complete walk over a single-player game
/// is entitled to its negative answer.
#[test]
fn shabaka_kamila_bila_dalail_la_tastadhin() {
    let huwiya = huwiya(367_520, "Hollow Knight", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Renpy, 99, vec![WajihaRusum::OpenGl]),
        &[],
    );
    let aql = Aql::jadeed(mudkhalat(huwiya, HalatFahs::mafhusa(taqreer)));
    assert!(
        !aql.makhatir()
            .iter()
            .any(|khatar| khatar.qeema.naw == NawKhatar::LaabJamai),
        "a complete walk over a single-player game asks nothing"
    );
}

// ---------------------------------------------------------------------------
// Probed and unrecognised is not never probed
// ---------------------------------------------------------------------------

/// The library row that could not tell the two apart.
#[test]
fn al_majhul_al_mafhus_ghayr_al_lam_yufhas() -> NatijatIkhtibar {
    // Avatar: probed, and the probe could not recognise the engine. Tier 3.
    let mafhusa = Aql::jadeed(mudkhalat(
        huwiya(2_840_770, "Avatar Frontiers of Pandora", Vec::new()),
        HalatFahs::mafhusa(taqreer_min(
            muharrik(AilatMuharrik::Majhul, 37, vec![WajihaRusum::D3d12]),
            &[],
        )),
    ));
    // The same game before anybody looked at it.
    let ghayr_mafhusa = Aql::jadeed(mudkhalat(
        huwiya(2_840_770, "Avatar Frontiers of Pandora", Vec::new()),
        HalatFahs::ghayr_mafhusa(),
    ));

    assert_eq!(mafhusa.muntaj().qeema, Muntaj::TabaqaFawqiya);
    assert_eq!(mafhusa.muntaj().qeema.raqm(), Some(3));
    assert!(
        !mafhusa.marfuda(),
        "an unrecognised engine still gets the overlay"
    );

    assert_eq!(ghayr_mafhusa.muntaj().qeema, Muntaj::Majhul);
    assert_eq!(
        ghayr_mafhusa.muntaj().qeema.raqm(),
        None,
        "no tier to print for a game nobody read"
    );
    let awwal = ghayr_mafhusa
        .mani_awwal()
        .ok_or("an unexamined game is blocked on being read")?;
    assert_eq!(awwal.qeema.naw, NawMani::LamYufhas);
    assert!(
        !awwal.qeema.nihai(),
        "this one is closed by examining the game"
    );
    assert!(awwal.min(MasdarMarifa::Bitaqa));

    // And the two say different things to a reader.
    assert_ne!(
        mafhusa.waad().qeema.sabab_injilizi,
        ghayr_mafhusa.waad().qeema.sabab_injilizi
    );
    assert_ne!(
        mafhusa.muntaj().qeema.ism_injilizi(),
        ghayr_mafhusa.muntaj().qeema.ism_injilizi()
    );
    Ok(())
}

/// A directory that is not a game at all is refused twice, and says one thing.
///
/// Steamworks Common Redistributables is a folder of .NET, VC++ and DirectX
/// installers. Both halves are pinned here because both are load-bearing and
/// they fail differently: the probe refuses it *when told* — which matters
/// because the probe's report is the half that gets persisted and read back
/// without the core — and the core refuses it regardless, which matters because
/// a caller can always hand the probe less than it knows.
#[test]
fn mudkhal_laysa_luba_la_yanal_taqreeran() -> NatijatIkhtibar {
    let huwiya = huwiya(
        228_980,
        "Steamworks Shared",
        vec![SimatLuba::LaysatLuba("Tool".to_owned())],
    );

    // Told what the launcher knows, the producer refuses on its own.
    let mubulligh = taqreer_min(
        muharrik(AilatMuharrik::Majhul, 15, Vec::new()),
        &huwiya.simat,
    );
    assert!(
        mubulligh.marfuda,
        "a persisted report must not describe a tier over a folder of installers"
    );
    assert!(
        mubulligh.sabab_injilizi.contains("Tool")
            && mubulligh.sabab_injilizi.contains("not a game"),
        "and it names what the launcher called it: {}",
        mubulligh.sabab_injilizi
    );

    // Not told, it still writes a tier-3 report — so the core is not redundant.
    let samit = taqreer_min(muharrik(AilatMuharrik::Majhul, 15, Vec::new()), &[]);
    assert_eq!(samit.tabaqa.raqm(), 3);
    assert!(!samit.marfuda);

    let aql = Aql::jadeed(mudkhalat(huwiya, HalatFahs::mafhusa(samit)));
    assert_eq!(aql.muntaj().qeema, Muntaj::LaShay);
    let awwal = aql.mani_awwal().ok_or("a blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::LaysatLuba);
    assert!(
        awwal.qeema.injilizi.contains("Tool"),
        "{}",
        awwal.qeema.injilizi
    );
    assert!(awwal.min(MasdarMarifa::Iktishaf));

    // The two refusals are the same sentence, which is the point of doing it in
    // two places rather than an accident of doing it twice.
    assert_eq!(
        mubulligh.sabab_injilizi, awwal.qeema.injilizi,
        "one entry, one sentence, whichever surface the reader arrived on"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The tier-1 games, whose adapters are unfinished
// ---------------------------------------------------------------------------

/// Among Us: tier 1, and nothing reaches the screen in this build.
///
/// The two are separate answers and both are kept. A patch may still be
/// installed; the one-button run may not.
#[test]
fn unity_yanal_al_istibdal_wa_la_yasil_shay() -> NatijatIkhtibar {
    let huwiya = huwiya(945_360, "Among Us", Vec::new());
    let mut engine = muharrik(AilatMuharrik::Unity, 99, vec![WajihaRusum::OpenGl]);
    engine.khalfiya = KhalfiyaBarmajiya::Il2cpp;
    let taqreer = taqreer_min(engine, &[]);
    let aql = Aql::jadeed(mudkhalat(huwiya, HalatFahs::mafhusa(taqreer)));

    let waad = aql.waad();
    assert_eq!(waad.qeema.muntaj, Muntaj::Istibdal);
    assert_eq!(waad.qeema.muntaj.raqm(), Some(1));
    assert!(waad.qeema.muntaj.yaktub(), "tier 1 writes into the game");
    assert!(
        !waad.qeema.tasil(),
        "and this build's adapter does not reach the screen"
    );
    let naqs = waad
        .qeema
        .naqs
        .as_ref()
        .ok_or("an unfinished tier names what is missing")?;
    assert!(
        naqs.injilizi
            .contains("native library it calls into is not in this package"),
        "the sentence is the probe's own: {}",
        naqs.injilizi
    );

    // Not refused. The blocker that does stand reaches only the automatic run.
    assert!(!aql.marfuda());
    let mawani = aql.mawani();
    assert_eq!(mawani.len(), 1, "{mawani:?}");
    let awwal = mawani.first().ok_or("one blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::JahiziyaGhaiba);
    assert_eq!(awwal.qeema.nitaq(), NitaqMani::Tashghil);
    assert!(!awwal.qeema.nihai(), "an update closes it");
    assert!(!aql.jahiz_lil_tashghil());
    Ok(())
}

/// Resident Evil 4: Capcom BIO4, tier 1, and no installation reaches it.
#[test]
fn bio4_yanal_al_istibdal_wa_la_yamurru_tarkeeb() -> NatijatIkhtibar {
    let huwiya = huwiya(254_700, "Resident Evil 4", Vec::new());
    let taqreer = taqreer_min(muharrik(AilatMuharrik::Bio4, 96, Vec::new()), &[]);
    let aql = Aql::jadeed(mudkhalat(huwiya, HalatFahs::mafhusa(taqreer)));

    let waad = aql.waad();
    assert_eq!(waad.qeema.muntaj, Muntaj::Istibdal);
    let naqs = waad.qeema.naqs.as_ref().ok_or("a gap sentence")?;
    assert!(
        naqs.injilizi
            .contains("no installation reaches this game at all"),
        "{}",
        naqs.injilizi
    );
    // The chain names the report and, through it, the engine and the tier it
    // was read off — which is what makes a wrong product traceable to a wrong
    // identification rather than to this crate.
    let wasf = &waad
        .shawahid
        .first()
        .ok_or("the promise names its input")?
        .wasf;
    assert!(wasf.contains("Capcom BIO4"), "{wasf}");
    assert!(wasf.contains("tier 1"), "{wasf}");
    assert!(wasf.contains("ghaiba"), "{wasf}");
    Ok(())
}

// ---------------------------------------------------------------------------
// The publisher's own Arabic, and the setting that reveals it
// ---------------------------------------------------------------------------

/// Official Arabic blocks by default and becomes a risk when the user says so.
#[test]
fn al_lugha_al_rasmiya_mani_thumma_khatar() -> NatijatIkhtibar {
    let huwiya = huwiya(424_840, "Little Nightmares", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Unreal, 97, vec![WajihaRusum::D3d11]),
        &[],
    );
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.lugha = Some(lugha_kamila());

    let mughlaq = Aql::jadeed(mudkhalat.clone());
    let awwal = mughlaq.mani_awwal().ok_or("a blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::LughaRasmiya);
    assert!(
        awwal.min(MasdarMarifa::LughaRasmiya),
        "the verdict's own evidence is in the chain"
    );
    assert!(
        awwal.min(MasdarMarifa::Idadat),
        "and so is the setting that left it standing"
    );
    assert_eq!(mughlaq.muntaj().qeema, Muntaj::LaShay);
    assert!(
        mughlaq
            .makhatir()
            .iter()
            .all(|khatar| khatar.qeema.naw != NawKhatar::LughaRasmiyaMutajawaza),
        "a blocker is not also a risk"
    );

    mudkhalat.mawqif.istibdal_lugha_rasmiya = true;
    let maftuh = Aql::jadeed(mudkhalat);
    assert!(
        maftuh
            .mawani()
            .iter()
            .all(|mani| mani.qeema.naw != NawMani::LughaRasmiya),
        "the setting reveals the surface"
    );
    let khatar = maftuh
        .makhatir()
        .into_iter()
        .find(|khatar| khatar.qeema.naw == NawKhatar::LughaRasmiyaMutajawaza)
        .ok_or("it becomes a risk instead of disappearing")?;
    assert!(
        khatar.qeema.muallaq(),
        "and it is unanswered until the user answers it"
    );
    assert!(khatar.min(MasdarMarifa::Idadat));
    // And the product itself is unchanged: the setting reveals, it does not
    // promote.
    assert_eq!(maftuh.muntaj().qeema, Muntaj::Istibdal);
    Ok(())
}

// ---------------------------------------------------------------------------
// Risks
// ---------------------------------------------------------------------------

/// An untouched box is a real no, and answering it opens the gate.
#[test]
fn al_khatar_yabqa_muallaqan_hatta_yujab() {
    let huwiya = huwiya(2_567_870, "ChainedTogether", vec![SimatLuba::JamaiOnline]);
    let jidhr = huwiya.jidhr.clone();
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Unreal, 99, vec![WajihaRusum::D3d12]),
        &[],
    );
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.aman.shabaka = IjmaaShabaka {
        jidhr,
        dalail: vec![DaleelShabaka {
            naw: NawDaleelShabaka::FiaSteam,
            dalala: DalalatShabaka::MutaaddidOnline,
            ayn: "category 1: Multi-player".to_owned(),
            masar: None,
        }],
        thughrat: Vec::new(),
        mabtur: false,
    };

    let bila_iqrar = Aql::jadeed(mudkhalat.clone());
    let muallaqa: Vec<NawKhatar> = bila_iqrar
        .makhatir_muallaqa()
        .map(|khatar| khatar.qeema.naw)
        .collect();
    assert!(muallaqa.contains(&NawKhatar::BayanAwwal), "{muallaqa:?}");
    assert!(muallaqa.contains(&NawKhatar::LaabJamai), "{muallaqa:?}");
    assert!(
        !bila_iqrar.jahiz_lil_tathbeet(),
        "nothing is written on an unanswered question"
    );

    mudkhalat.mawqif.iqrarat = vec![NawKhatar::BayanAwwal, NawKhatar::LaabJamai];
    let bi_iqrar = Aql::jadeed(mudkhalat);
    assert_eq!(bi_iqrar.makhatir_muallaqa().count(), 0);
    assert!(bi_iqrar.jahiz_lil_tathbeet());
    assert!(!bi_iqrar.marfuda(), "and none of this was ever a refusal");
}

/// A patch that matches the build only approximately asks before it installs.
#[test]
fn al_mutabaqa_al_taqribiya_tastadhin() -> NatijatIkhtibar {
    let huwiya = huwiya(367_520, "Hollow Knight", Vec::new());
    let taqreer = taqreer_min(
        muharrik(AilatMuharrik::Unity, 99, vec![WajihaRusum::D3d11]),
        &[],
    );
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer));
    mudkhalat.mustawda = Some(HalatMustawda {
        adad_ruqaa: 2,
        adad_aswat: 0,
        afdal: Some(taarib_mustalahat::bina::MutabaqaBina::Nitaq),
        yahtaj_iqrar: true,
        naqis: false,
    });
    let aql = Aql::jadeed(mudkhalat);

    let khatar = aql
        .makhatir()
        .into_iter()
        .find(|khatar| khatar.qeema.naw == NawKhatar::MutabaqaTaqribiya)
        .ok_or("an approximate match is asked about")?;
    assert!(khatar.min(MasdarMarifa::Mustawda));
    assert_eq!(
        khatar.qeema.injilizi,
        taarib_mustalahat::bina::MutabaqaBina::Nitaq.wasf_injilizi(),
        "the sentence is the matcher's own"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The component store
// ---------------------------------------------------------------------------

/// A component the manifest lists and the store does not hold travels as
/// evidence on the readiness answer.
#[test]
fn al_mukawwin_al_mafqud_yasil_ka_shahid() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let jidhr = masrah.path();
    std::fs::write(
        jidhr.join("bayan_mukawwinat.json"),
        br#"{"mukhattat":1,"isdar":"1.0.0","hadaf":"x86_64-unknown-linux-gnu",
            "milaffat":[{"masar":"mukawwinat/bepinex/BepInEx.dll","hajm":4096,
            "sha256":"0000000000000000000000000000000000000000000000000000000000000000"}]}"#,
    )?;

    let makhzan = HalatMakhzan::ifhas(jidhr, &["bepinex"]);
    let naqisa: Vec<&str> = makhzan
        .naqisa()
        .map(|mukawwin| mukawwin.ism.as_str())
        .collect();
    assert_eq!(
        naqisa,
        vec!["bepinex"],
        "the manifest lists it and the store does not hold it"
    );

    let huwiya = huwiya(945_360, "Among Us", Vec::new());
    let mut engine = muharrik(AilatMuharrik::Unity, 99, vec![WajihaRusum::OpenGl]);
    engine.khalfiya = KhalfiyaBarmajiya::Il2cpp;
    let mut mudkhalat = mudkhalat(huwiya, HalatFahs::mafhusa(taqreer_min(engine, &[])));
    mudkhalat.makhzan = makhzan;
    let aql = Aql::jadeed(mudkhalat);

    let awwal = aql.mani_awwal().ok_or("the readiness blocker")?;
    assert_eq!(awwal.qeema.naw, NawMani::JahiziyaGhaiba);
    assert!(
        awwal.min(MasdarMarifa::MakhzanMukawwinat),
        "why nothing reached the screen names the absent component too"
    );
    // The verdict itself is still the report's: the store contributes evidence,
    // never an answer.
    assert!(awwal.min(MasdarMarifa::Imkaniyat));
    Ok(())
}

// ---------------------------------------------------------------------------
// The properties the whole thing rests on
// ---------------------------------------------------------------------------

/// The ordering is total, strict, and declared in one place.
#[test]
fn al_tarteeb_wahid_wa_la_yatakarrar() {
    let rutab: Vec<u8> = NawMani::KUL.iter().map(|naw| naw.rutba()).collect();
    let mut farida = rutab.clone();
    farida.sort_unstable();
    farida.dedup();
    assert_eq!(
        farida.len(),
        rutab.len(),
        "two blockers share a rank: {rutab:?}"
    );
    let mut murattaba = rutab.clone();
    murattaba.sort_unstable();
    assert_eq!(rutab, murattaba, "KUL is not in rank order: {rutab:?}");

    // Everything that stops the run only sorts after everything that stops
    // everything, so a surface filtering on scope never reorders the list.
    let mut ra_al_tashghil = false;
    for naw in NawMani::KUL {
        match naw.nitaq() {
            NitaqMani::Tashghil => ra_al_tashghil = true,
            NitaqMani::Kul => {
                assert!(
                    !ra_al_tashghil,
                    "{naw:?} outranks a run-only blocker and stops everything"
                );
            },
        }
    }

    let rutab: Vec<u8> = NawKhatar::KUL.iter().map(|naw| naw.rutba()).collect();
    let mut murattaba = rutab.clone();
    murattaba.sort_unstable();
    assert_eq!(
        rutab, murattaba,
        "the risks are not in the order they are asked: {rutab:?}"
    );
}

/// Every answer, for every game, names at least one input.
///
/// The property that makes this a mind rather than a cache: an answer that
/// cannot say what produced it is an answer this crate invented.
#[test]
fn kul_ijaba_tusammi_madkhalan() {
    let jidhr_eldenring = PathBuf::from("/mnt/f/SteamLibrary/steamapps/common/ELDEN RING");
    let mut marfuda = mudkhalat(
        huwiya(
            1_245_620,
            "ELDEN RING",
            vec![SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())],
        ),
        HalatFahs::mafhusa(taqreer_min(
            eldenring_muharrik(),
            &[SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())],
        )),
    );
    marfuda.aman.himaya = himaya_eldenring(&jidhr_eldenring);

    let mut unity_engine = muharrik(AilatMuharrik::Unity, 99, vec![WajihaRusum::OpenGl]);
    unity_engine.khalfiya = KhalfiyaBarmajiya::Mono;
    let halat = vec![
        marfuda,
        mudkhalat(
            huwiya(945_360, "Among Us", Vec::new()),
            HalatFahs::mafhusa(taqreer_min(unity_engine, &[])),
        ),
        mudkhalat(
            huwiya(2_840_770, "Avatar", Vec::new()),
            HalatFahs::mafhusa(taqreer_min(
                muharrik(AilatMuharrik::Majhul, 37, vec![WajihaRusum::D3d12]),
                &[],
            )),
        ),
        mudkhalat(
            huwiya(731_490, "Crash", Vec::new()),
            HalatFahs::ghayr_mafhusa(),
        ),
    ];

    for hala in halat {
        let ism = hala.huwiya.ism.clone();
        let aql = Aql::jadeed(hala);
        assert!(aql.muntaj().musnad(), "{ism}: the product names no input");
        assert!(aql.waad().musnad(), "{ism}: the promise names no input");
        assert!(
            aql.hala_himaya().musnad(),
            "{ism}: the anti-cheat answer names no input"
        );
        for mani in aql.mawani() {
            assert!(
                mani.musnad(),
                "{ism}: blocker {:?} names no input",
                mani.qeema.naw
            );
        }
        for khatar in aql.makhatir() {
            assert!(
                khatar.musnad(),
                "{ism}: risk {:?} names no input",
                khatar.qeema.naw
            );
        }
        for hadd in aql.hudud() {
            assert!(hadd.musnad(), "{ism}: a limit names no input");
        }
    }
}

/// Both languages are always present on every sentence a user could be shown.
#[test]
fn kul_jumla_bi_lughatayn() {
    let jidhr = PathBuf::from("/mnt/f/SteamLibrary/steamapps/common/ELDEN RING");
    let simat = vec![SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())];
    let mut mudkhalat = mudkhalat(
        huwiya(1_245_620, "ELDEN RING", simat.clone()),
        HalatFahs::mafhusa(taqreer_min(eldenring_muharrik(), &simat)),
    );
    mudkhalat.aman.himaya = himaya_eldenring(&jidhr);
    mudkhalat.lugha = Some(lugha_kamila());
    let aql = Aql::jadeed(mudkhalat);

    let jumlatan = |mani: &Mani| (mani.arabi.clone(), mani.injilizi.clone());
    for mani in aql.mawani() {
        let (arabi, injilizi) = jumlatan(&mani.qeema);
        assert!(
            !arabi.trim().is_empty(),
            "{:?} has no Arabic",
            mani.qeema.naw
        );
        assert!(
            !injilizi.trim().is_empty(),
            "{:?} has no English",
            mani.qeema.naw
        );
    }
    for khatar in aql.makhatir() {
        assert!(
            !khatar.qeema.arabi.trim().is_empty(),
            "{:?}",
            khatar.qeema.naw
        );
        assert!(
            !khatar.qeema.injilizi.trim().is_empty(),
            "{:?}",
            khatar.qeema.naw
        );
    }
    let waad = aql.waad();
    assert!(!waad.qeema.sabab_arabi.trim().is_empty());
    assert!(!waad.qeema.sabab_injilizi.trim().is_empty());
    assert!(!waad.qeema.ism_arabi.is_empty());
    assert!(!waad.qeema.ism_injilizi.is_empty());
}

/// The product name is the tier's own word, never a second copy of it.
#[test]
fn ism_al_muntaj_huwa_ism_al_tabaqa() {
    use taarib_mustalahat::muharrik::Tabaqa;
    for tabaqa in [Tabaqa::Kamil, Tabaqa::RasmMubashir, Tabaqa::TarjamaFawqiya] {
        let muntaj = Muntaj::min_tabaqa(tabaqa);
        assert_eq!(
            muntaj.tabaqa(),
            Some(tabaqa),
            "the mapping does not round-trip"
        );
        assert_eq!(muntaj.ism_arabi(), tabaqa.ism_arabi());
        assert_eq!(muntaj.ism_injilizi(), tabaqa.ism_injilizi());
        assert_eq!(muntaj.raqm(), Some(tabaqa.raqm()));
    }
    assert_eq!(Muntaj::LaShay.raqm(), None);
    assert_eq!(Muntaj::Majhul.raqm(), None);
    assert_ne!(Muntaj::LaShay.ism_injilizi(), Muntaj::Majhul.ism_injilizi());
}

/// The graphics list is the probe's, read through rather than copied.
#[test]
fn qaimat_al_rusum_hiya_qaimat_al_fahs() {
    let rusum = vec![WajihaRusum::D3d12, WajihaRusum::OpenGl];
    let taqreer = taqreer_min(muharrik(AilatMuharrik::Majhul, 49, rusum.clone()), &[]);
    let aql = Aql::jadeed(mudkhalat(
        huwiya(570_940, "DARK SOULS REMASTERED", Vec::new()),
        HalatFahs::mafhusa(taqreer),
    ));
    assert_eq!(aql.rusum(), rusum.as_slice());

    // A game nobody examined reports no graphics API and does not pretend to.
    let bila = Aql::jadeed(mudkhalat(
        huwiya(570_940, "DARK SOULS REMASTERED", Vec::new()),
        HalatFahs::ghayr_mafhusa(),
    ));
    assert!(bila.rusum().is_empty());
    assert!(bila.muharrik().is_none());
}
