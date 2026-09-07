//! تقرير التجاوز — the package's overflow report is measured, and says so.
//!
//! The failure these tests hold shut: every caller of the compiler once handed
//! it an empty overflow report, so every package declared "no measured
//! overflow" over a project nobody had measured, and the publish gate drew a
//! green check for it. The report is now built inside precomputation from the
//! layouts that ship, and these tests drive the whole compile — a real font, a
//! real container reopened and read back — over a project holding one string
//! of each kind: one that fits, one that plainly overflows, one with no width
//! to compare against, one with no size to lay out at, and one untranslated.
//!
//! The Arabic is shaped, not counted: a translated string is routinely wider
//! than the Latin it replaces, and the width the report holds is the width
//! `taarib-saff` produced for it through the same font the package bundles.
//!
//! # The font
//!
//! IBM Plex Sans Arabic Regular, staged into `apps/studio/src/khutut/` and
//! pinned by content hash in `assets/aqfal/qufl_khutut.json`. Nothing here
//! downloads anything, and nothing is mocked.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
use taarib_mustalahat::muraja::SijillMuraja;
use taarib_mustalahat::nass::{
    MasdarIstikhraj, MudkhalNass, NassId, QuyudNass, SiyaqNass, TasnifNass,
};
use taarib_mustalahat::ruqaa::{RukhsaRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama};
use taarib_ruqaa::qari::Ruqaa;
use taarib_saff::{MawridKhatt, SilsilatKhutut};
use taarib_tarqee::irtibat::BasmatKhatt;
use taarib_tarqee::mujammi::HuzmaMabniya;
use taarib_tarqee::taqrir_tajawuz::{
    AQSA_GHAYR_MUFASSAL, BaniTaqrirTajawuz, HalatQiyasTajawuz, HalatTahaqquq, MudkhalQiyas,
    NawAdamAltahaqquq, SababAdamAltahaqquq, TaqrirTajawuz,
};
use taarib_tarqee::{
    IktishafMaqasat, IrtibatBina, KhattMujammaa, KhiyaratTasbeeq, MudkhalatFahs, MudkhalatTajmee,
    MuharrikHuzma, MukhattatBasma, ShiddatTajawuz, WasfHuzma, ihsib_taghtiya, ijmaa, ijri,
};

/// The size every measured string is drawn at, in pixels.
const HAJM: f32 = 20.0;

/// A button label that fits comfortably: three letters in a wide box.
const YASA: &str = "حفظ";

/// A button label that plainly does not: a whole sentence in a box built for
/// one short word, on an element the engine refuses to wrap.
const LA_YASA: &str = "العودة إلى القائمة الرئيسية للعبة";

/// A label with no recorded width: the state every string is in after static
/// extraction, because no string table format records the box it is drawn into.
const BILA_ARD: &str = "خيارات";

/// A label with no discovered size, so there is nothing to lay it out at.
const BILA_HAJM: &str = "إنهاء";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The staged Arabic face, found by walking up from this crate.
fn masar_khatt() -> PathBuf {
    const MURASHAH: &str = "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf";

    let mut dalil: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    while let Some(jidhr) = dalil {
        let murashah = jidhr.join(MURASHAH);
        if murashah.is_file() {
            return murashah;
        }
        dalil = jidhr.parent();
    }
    panic!("no Arabic font found at or above {}", env!("CARGO_MANIFEST_DIR"));
}

/// One string with the constraint a capture session would have recorded for
/// it, or none.
fn mudkhal(hawiya: &str, masdar: &str, hadaf: Option<&str>, quyud: QuyudNass) -> MudkhalNass {
    MudkhalNass {
        id: NassId::min_mawqi(hawiya, "0", masdar),
        masdar: masdar.to_owned(),
        hadaf: hadaf.map(str::to_owned),
        muraja: SijillMuraja::jadeed(),
        siyaq: SiyaqNass {
            hawiya: hawiya.to_owned(),
            mawqi: "0".to_owned(),
            ..SiyaqNass::default()
        },
        quyud,
        nasq_masdar: Vec::new(),
        nasq_hadaf: Vec::new(),
        takrar: 1,
        majmua: None,
        alamat: Vec::new(),
        tareeqa: Some(TareeqaTarjama::AaliyaFaqat),
        muzawwid: None,
        muharrir: None,
        akhir_tabdeel: None,
        tasnif: TasnifNass::Qaima,
        thiqat_tasnif: 90,
        masdar_istikhraj: MasdarIstikhraj::Sakin,
        tarmiz: None,
    }
}

/// A single-line button with a measured box.
fn quyud_zirr(ard: f32) -> QuyudNass {
    QuyudNass {
        hajm_khatt: Some(HAJM),
        aqsa_ard: Some(ard),
        satr_wahid: true,
        ..QuyudNass::default()
    }
}

/// The five-string project every compile here is driven over.
fn mashru() -> Vec<MudkhalNass> {
    vec![
        mudkhal("ui/menu.txt", "Save", Some(YASA), quyud_zirr(400.0)),
        mudkhal("ui/menu.txt", "Back to the game's main menu", Some(LA_YASA), quyud_zirr(40.0)),
        mudkhal(
            "ui/menu.txt",
            "Options",
            Some(BILA_ARD),
            QuyudNass { hajm_khatt: Some(HAJM), ..QuyudNass::default() },
        ),
        mudkhal("ui/menu.txt", "Quit", Some(BILA_HAJM), QuyudNass::default()),
        mudkhal("ui/menu.txt", "Load", None, quyud_zirr(400.0)),
    ]
}

fn id(nusus: &[MudkhalNass], masdar: &str) -> NassId {
    nusus
        .iter()
        .find(|wahid| wahid.masdar == masdar)
        .unwrap_or_else(|| panic!("{masdar:?} is in the project"))
        .id
}

/// Compiles the project the whole way: checks, layout, atlas, container, and
/// the compiler's own read-back of what it wrote.
fn ijmi(nusus: &[MudkhalNass]) -> HuzmaMabniya {
    let masar = masar_khatt();
    let jidhr_khutut = masar.parent().expect("the font directory");
    let bayt = fs::read(&masar).unwrap_or_else(|_| panic!("{} unreadable", masar.display()));
    let mawrid = MawridKhatt::jadeed(Arc::new(bayt), 0)
        .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
    let silsila = SilsilatKhutut::wahid(Arc::new(mawrid)).expect("a one-font chain");
    let khutut = vec![
        KhattMujammaa::min_majmua(jidhr_khutut, &masar, 0)
            .expect("a font inside the font directory"),
    ];

    let wasf = WasfHuzma {
        unwan: "تقرير التجاوز".to_owned(),
        ism_musahim: "ikhtibar".to_owned(),
        ism_luba: "R.E.P.O.".to_owned(),
        rukhsa: RukhsaRuqaa::Cc0,
        tareeqa: TareeqaTarjama::AaliyaFaqat,
        isdar_taarib: "1.0.0".to_owned(),
    };
    let ijtiyaz = ijri(&MudkhalatFahs { madakhil: nusus, wasf: &wasf, takhtitat_fashila: &[] })
        .expect("the hard checks pass on this project");

    let irtibat = IrtibatBina {
        manassat: vec!["ikhtibar-1".to_owned()],
        basmat: Vec::new(),
        adad_malaffat: 1,
        mukhattat: MukhattatBasma::min_masarat(["Game.exe"]).expect("a one-file recipe"),
        nitaq: None,
        khutut: khutut.iter().map(BasmatKhatt::min_mujammaa).collect(),
    };

    let mut iktishaf = IktishafMaqasat::jadeed();
    for wahid in nusus {
        iktishaf.sajjil_mudkhal(wahid);
    }
    let maqasat = iktishaf.ahsi(nusus);

    let taghtiya = ihsib_taghtiya(nusus, None, None);
    let khiyarat = KhiyaratTasbeeq::default();
    let mudkhalat = MudkhalatTajmee {
        nusus,
        id: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::AWWAL,
        wasf: &wasf,
        muharrik: MuharrikHuzma {
            aila: AilatMuharrik::Unity,
            khalfiya: KhalfiyaBarmajiya::Il2cpp,
            tabaqa: Tabaqa::Kamil,
        },
        irtibat: &irtibat,
        maqasat: &maqasat,
        taghtiya: &taghtiya,
        khiyarat: &khiyarat,
        mustawa: None,
    };

    ijmaa(&mudkhalat, ijtiyaz, &silsila, &khutut).expect("the package compiles")
}

/// The report as the package's own metadata section holds it — read back out
/// of the written bytes, because what the writer believes it stored is exactly
/// the thing under test.
fn taqrir_fil_huzma(huzma: &HuzmaMabniya) -> TaqrirTajawuz {
    let ruqaa = Ruqaa::iftah(huzma.bayt.bayt()).expect("the package reopens");
    let bayan = ruqaa.bayan_json().expect("the metadata section reads back as JSON");
    let tajawuz = bayan.get("tajawuz").expect("the manifest carries the overflow report");
    serde_json::from_value(tajawuz.clone()).expect("the report deserializes from the manifest")
}

// ---------------------------------------------------------------------------
// The three states, from one compile
// ---------------------------------------------------------------------------

/// One compile measures what it can, and keeps what it cannot apart.
#[test]
fn ikhtibar_thalathat_ahwal_min_tajmee_wahid() {
    let nusus = mashru();
    let huzma = ijmi(&nusus);
    let taqrir = &huzma.bayan.tajawuz;

    assert_eq!(taqrir.hala(), HalatQiyasTajawuz::Juzi, "{}", taqrir.wasf_injilizi());
    assert_eq!(taqrir.mulakhkhas.maqis, 2, "two strings had a box and a layout to compare");
    assert_eq!(taqrir.mulakhkhas.salim, 1, "one of them fitted");
    assert_eq!(taqrir.mulakhkhas.mutajawiz, 1, "one of them did not");
    assert_eq!(
        taqrir.mulakhkhas.ghayr_mutahaqqaq,
        2,
        "the string with no width and the string with no size were submitted and not measured"
    );
    assert_eq!(taqrir.mulakhkhas.nusus_ghayr_mutahaqqaqa, 2);
}

/// The overflowing string is measured over, through the real shaper.
#[test]
fn ikhtibar_almutajawiz_yuqas_mutajawizan() {
    let nusus = mashru();
    let huzma = ijmi(&nusus);
    let taqrir = &huzma.bayan.tajawuz;

    let Some(aswa) = taqrir.aswa() else { panic!("something overran") };
    assert_eq!(aswa.nass, id(&nusus, "Back to the game's main menu"));
    assert!(
        aswa.ard_maqis > aswa.ard_mutah,
        "{}px of shaped Arabic in a {}px box",
        aswa.ard_maqis,
        aswa.ard_mutah
    );
    assert!(
        aswa.shidda >= ShiddatTajawuz::Shadid,
        "a sentence in a one-word button is at least severe, got {:?}",
        aswa.shidda
    );
    assert_eq!(taqrir.tajawuzat.len(), 1, "exactly one (string, size) pair overran");
    assert!(taqrir.mulakhkhas.yastahiqq_iaada() >= 1, "and it is worth rewriting");
}

/// Per string, the four answers are four answers.
#[test]
fn ikhtibar_hal_kull_nass() {
    let nusus = mashru();
    let huzma = ijmi(&nusus);
    let taqrir = &huzma.bayan.tajawuz;

    assert!(matches!(
        taqrir.hal_nass(id(&nusus, "Back to the game's main menu")),
        HalatTahaqquq::Mutajawiz { adad: 1, .. }
    ));
    assert!(matches!(
        taqrir.hal_nass(id(&nusus, "Options")),
        HalatTahaqquq::GhayrMutahaqqaq { adad: 1 }
    ));
    assert!(matches!(
        taqrir.hal_nass(id(&nusus, "Quit")),
        HalatTahaqquq::GhayrMutahaqqaq { adad: 1 }
    ));
    // The package copy drops the passing rows and says so, so the fitting
    // string answers "never submitted" here and "fitted" in the count.
    assert!(taqrir.salima_muqallama);
    assert_eq!(taqrir.hal_nass(id(&nusus, "Save")), HalatTahaqquq::LamYuqas);
    assert_eq!(taqrir.mulakhkhas.salim, 1);
    assert_eq!(
        taqrir.hal_nass(id(&nusus, "Load")),
        HalatTahaqquq::LamYuqas,
        "an untranslated string is coverage's business and is not submitted"
    );
}

/// Each unmeasurable string names its own cause, and the remedy differs.
#[test]
fn ikhtibar_kull_sabab_bismih() {
    let nusus = mashru();
    let huzma = ijmi(&nusus);
    let taqrir = &huzma.bayan.tajawuz;

    let sabab = |masdar: &str| {
        let nass = id(&nusus, masdar);
        taqrir
            .ghayr_qabil_lil_tahaqquq
            .iter()
            .find(|madkhal| madkhal.nass == nass)
            .unwrap_or_else(|| panic!("{masdar:?} is in the unverifiable list"))
            .sabab
    };
    assert_eq!(sabab("Options"), SababAdamAltahaqquq::BilaArdMutah);
    assert_eq!(sabab("Quit"), SababAdamAltahaqquq::BilaHajm);
    assert_ne!(
        NawAdamAltahaqquq::BilaArdMutah.wasf_injilizi(),
        NawAdamAltahaqquq::BilaHajm.wasf_injilizi()
    );
    assert_eq!(taqrir.mulakhkhas.hasab_sabab.get("bila_ard_mutah"), Some(&1));
    assert_eq!(taqrir.mulakhkhas.hasab_sabab.get("bila_hajm"), Some(&1));
    assert_eq!(
        NawAdamAltahaqquq::min_miftah("bila_hajm"),
        Some(NawAdamAltahaqquq::BilaHajm),
        "the summary key words itself again"
    );
    assert_eq!(NawAdamAltahaqquq::min_miftah("min_bina_ajdad"), None);
}

/// The report survives the trip through the package's metadata section.
#[test]
fn ikhtibar_altaqrir_yaqra_min_alhuzma() {
    let nusus = mashru();
    let huzma = ijmi(&nusus);
    let maqru = taqrir_fil_huzma(&huzma);

    assert_eq!(maqru, huzma.bayan.tajawuz, "what was written is what reads back");
    assert_eq!(maqru.hala(), HalatQiyasTajawuz::Juzi);
    assert_eq!(maqru.mulakhkhas.ghayr_mutahaqqaq, 2);
    assert_eq!(maqru.tajawuzat.len(), 1);
}

/// A project measured clean is the only project that answers "fully measured".
#[test]
fn ikhtibar_alkamil_yahtaj_kull_zawj() {
    let nusus = vec![mudkhal("ui/menu.txt", "Save", Some(YASA), quyud_zirr(400.0))];
    let huzma = ijmi(&nusus);
    let taqrir = &huzma.bayan.tajawuz;

    assert_eq!(taqrir.hala(), HalatQiyasTajawuz::Kamil);
    assert!(taqrir.tajawuzat.is_empty());
    assert!(taqrir.ghayr_qabil_lil_tahaqquq.is_empty());
    assert_eq!(taqrir.mulakhkhas.salim, 1);
}

/// A project with no width recorded anywhere — every statically extracted
/// project — answers "not measured", never "no overflow".
#[test]
fn ikhtibar_bila_ard_laysa_najahan() {
    let nusus = vec![mudkhal(
        "ui/menu.txt",
        "Options",
        Some(BILA_ARD),
        QuyudNass { hajm_khatt: Some(HAJM), ..QuyudNass::default() },
    )];
    let huzma = ijmi(&nusus);
    let taqrir = &huzma.bayan.tajawuz;

    assert_eq!(taqrir.hala(), HalatQiyasTajawuz::LamYuqas);
    assert!(taqrir.tajawuzat.is_empty(), "nothing overran, because nothing was compared");
    assert_eq!(taqrir.mulakhkhas.maqis, 0);
    assert_eq!(taqrir.mulakhkhas.ghayr_mutahaqqaq, 1);
    assert_eq!(taqrir.mulakhkhas.nisbat_altahaqquq(), Some(0.0));
}

/// An empty report is "nothing to measure", and is not the same as measured.
#[test]
fn ikhtibar_alfarigh_laysa_maqisan() {
    let farigh = TaqrirTajawuz::farigh();
    assert_eq!(farigh.hala(), HalatQiyasTajawuz::Farigh);
    assert!(!farigh.hala().kamil());
    assert_eq!(farigh.mulakhkhas.nisbat_altahaqquq(), None);
}

// ---------------------------------------------------------------------------
// The package copy
// ---------------------------------------------------------------------------

/// Past the cap the rows are cut, the counts are not, and the cut is recorded.
#[test]
fn ikhtibar_alqass_yahfaz_aladad() {
    let mut bani = BaniTaqrirTajawuz::jadeed();
    let adad = AQSA_GHAYR_MUFASSAL + 1;
    let nusus: Vec<MudkhalNass> = (0..adad)
        .map(|fihris| {
            let mut wahid = mudkhal(
                "ui/credits.txt",
                &format!("line {fihris}"),
                Some("سطر"),
                QuyudNass { hajm_khatt: Some(HAJM), ..QuyudNass::default() },
            );
            // The last one is a button; every other one is a credit line.
            wahid.tasnif =
                if fihris == adad - 1 { TasnifNass::Qaima } else { TasnifNass::Nusub };
            wahid
        })
        .collect();
    for wahid in &nusus {
        bani.sajjil(&MudkhalQiyas { madkhal: wahid, hajm: HAJM, takhtit: None, takhtit_asl: None });
    }
    let kamil = bani.ikhtim();
    assert_eq!(kamil.ghayr_qabil_lil_tahaqquq.len(), adad);
    assert!(!kamil.ghayr_muqallam);

    let lil_huzma = kamil.clone().lil_huzma();
    assert!(lil_huzma.ghayr_muqallam, "the cut is recorded");
    assert_eq!(lil_huzma.ghayr_qabil_lil_tahaqquq.len(), AQSA_GHAYR_MUFASSAL);
    assert_eq!(
        lil_huzma.mulakhkhas.ghayr_mutahaqqaq,
        u32::try_from(adad).expect("fits"),
        "the count is whole"
    );
    assert_eq!(lil_huzma.hala(), kamil.hala());
    let Some(awwal) = lil_huzma.ghayr_qabil_lil_tahaqquq.first() else { panic!("rows kept") };
    assert_eq!(awwal.tasnif, TasnifNass::Qaima, "the button survives the cut before the credits");
}

/// A report written before the cut existed still reads.
#[test]
fn ikhtibar_taqrir_qadeem_yaqra() {
    let mut qadeem = serde_json::to_value(TaqrirTajawuz::farigh()).expect("serializes");
    let Some(kaain) = qadeem.as_object_mut() else { panic!("an object") };
    let _ = kaain.remove("ghayr_muqallam");
    let maqru: TaqrirTajawuz = serde_json::from_value(qadeem).expect("reads without the field");
    assert!(!maqru.ghayr_muqallam);
}
