//! توحيد الترجمة — a project that disagrees with itself still compiles.
//!
//! The failing case these tests exist for is not hypothetical. A real drive of a
//! 3 718-string table died with the container writer's own words — *two
//! different translations for one source string is 32 bytes; the format fixes it
//! at 31* — and produced no patch. Nothing about that project was unusual: its
//! rows are per container and per path, so a menu label appears dozens of times,
//! and each row went to the translation provider on its own.
//!
//! [`ikhtibar_al_katib_ma_zal_yarfud`] holds the writer's refusal in place, so
//! that the fix is a decision the compiler makes and not a guard somebody
//! removed. Every other test drives the whole of
//! [`taarib_tarqee::ijmaa`] — a real font, a real atlas, a real container that is
//! reopened and read back — over a project carrying exactly that disagreement.
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
use taarib_mustalahat::muraja::{ShahadatMuraja, SijillMuraja};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{
    MasdarIstikhraj, MudkhalNass, NassId, QuyudNass, SiyaqNass, TasnifNass,
};
use taarib_mustalahat::ruqaa::{RukhsaRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama};
use taarib_ruqaa::aqsam::NawQism;
use taarib_ruqaa::katib::Katib;
use taarib_ruqaa::qari::{self, Ruqaa, miftah_min_nass};
use taarib_saff::{MawridKhatt, SilsilatKhutut};
use taarib_tarqee::irtibat::BasmatKhatt;
use taarib_tarqee::mujammi::HuzmaMabniya;
use taarib_tarqee::{
    IktishafMaqasat, IrtibatBina, KhattMujammaa, KhiyaratTasbeeq, MudkhalatFahs, MudkhalatTajmee,
    MuharrikHuzma, MukhattatBasma, WasfHuzma, ihsib_taghtiya, ijmaa, ijri,
};

/// The size every string in these projects is drawn at, in pixels.
const HAJM: f32 = 20.0;

/// The two readings of `Back` a provider produced for one game.
const RUJU: &str = "رجوع";
/// The second one, which the format cannot hold beside the first.
const AWDA: &str = "العودة";

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
    panic!(
        "no Arabic font found at or above {}",
        env!("CARGO_MANIFEST_DIR")
    );
}

/// One string, with everything a compile needs and nothing it does not.
fn mudkhal(hawiya: &str, masdar: &str, hadaf: &str) -> MudkhalNass {
    MudkhalNass {
        id: NassId::min_mawqi(hawiya, "0", masdar),
        masdar: masdar.to_owned(),
        hadaf: Some(hadaf.to_owned()),
        muraja: SijillMuraja::jadeed(),
        siyaq: SiyaqNass {
            hawiya: hawiya.to_owned(),
            mawqi: "0".to_owned(),
            ..SiyaqNass::default()
        },
        quyud: QuyudNass {
            hajm_khatt: Some(HAJM),
            ..QuyudNass::default()
        },
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

/// The same string, approved by a human review.
///
/// Built through the one transition that reaches approval, so the compiler's
/// hard checks — which refuse a record that claims approval with no human
/// behind it — pass on it the way they would on a real project.
fn muakkad(mut wahid: MudkhalNass) -> MudkhalNass {
    let murajii = MusahimId::jadeed("a".repeat(64)).expect("a 64-character fingerprint");
    wahid
        .muraja
        .sajjil_musawwada(murajii.clone(), 1_700_000_000);
    wahid
        .muraja
        .iaatimad(ShahadatMuraja::baad_muraja(murajii, 1_700_000_100), None);
    wahid.tareeqa = Some(TareeqaTarjama::AaliyaThumBashariya);
    wahid
}

/// Compiles a project the whole way: checks, layout, atlas, container, and the
/// compiler's own read-back of what it wrote.
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
        unwan: "توحيد الترجمة".to_owned(),
        ism_musahim: "ikhtibar".to_owned(),
        ism_luba: "R.E.P.O.".to_owned(),
        rukhsa: RukhsaRuqaa::Cc0,
        tareeqa: TareeqaTarjama::AaliyaFaqat,
        isdar_taarib: "1.0.0".to_owned(),
    };
    let ijtiyaz = ijri(&MudkhalatFahs {
        madakhil: nusus,
        wasf: &wasf,
        takhtitat_fashila: &[],
    })
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

/// The translation the finished container actually holds for one source string.
///
/// Read back out of the written bytes rather than out of the compiler's own
/// data, because what the writer believes it stored is exactly the thing under
/// test.
fn tarjama_fil_huzma(huzma: &HuzmaMabniya, masdar: &str) -> String {
    let khaam = huzma.bayt.bayt();
    let ruqaa = Ruqaa::iftah(khaam).expect("the package reopens");
    let qism = ruqaa.qism(NawQism::Nusus).expect("a string section");
    let jadwal = qari::nusus(qism.bayt()).expect("the string section reads back");
    jadwal
        .tarjama(miftah_min_nass(masdar))
        .unwrap_or_else(|| panic!("{masdar:?} is not in the container"))
        .to_owned()
}

/// How many rows the container holds.
fn adad_sufuf(huzma: &HuzmaMabniya) -> usize {
    let khaam = huzma.bayt.bayt();
    let ruqaa = Ruqaa::iftah(khaam).expect("the package reopens");
    let qism = ruqaa.qism(NawQism::Nusus).expect("a string section");
    qari::nusus(qism.bayt())
        .expect("the string section reads back")
        .sijillat
        .len()
}

// ---------------------------------------------------------------------------
// The refusal that must stay
// ---------------------------------------------------------------------------

/// The container writer still refuses two translations for one source string.
///
/// This is the exact failure the real drive died on, and it is kept: the format
/// holds one row per source string, so a writer that accepted both would produce
/// a table whose lookup answer depends on which row a binary search landed on.
/// The fix is that the compiler no longer *offers* it two, not that the writer
/// stopped minding.
#[test]
fn ikhtibar_al_katib_ma_zal_yarfud() {
    let mut katib = Katib::jadeed();
    let awwal = katib.nass("Back", RUJU);
    assert!(awwal.is_ok(), "the first translation is accepted");
    assert!(
        katib.nass("Back", RUJU).is_ok(),
        "the same translation again folds onto one row rather than failing"
    );
    assert!(
        katib.nass("Back", AWDA).is_err(),
        "a second, different translation for one source string is still refused"
    );
}

// ---------------------------------------------------------------------------
// The compile that used to die
// ---------------------------------------------------------------------------

/// A project whose rows disagree about one string now produces a package.
///
/// Two containers each hold a `Back`, translated differently, and a third row
/// agrees with the first. Before the fold this compile ended at
/// `KitabatHuzmaFashila` with the writer's message and wrote nothing.
#[test]
fn ikhtibar_mashru_mutanaqid_yubna() {
    let nusus = vec![
        mudkhal("ui/menu.txt", "Back", RUJU),
        mudkhal("ui/pause.txt", "Back", AWDA),
        mudkhal("ui/shop.txt", "Back", RUJU),
        mudkhal("ui/menu.txt", "Play", "ابدأ"),
    ];
    let huzma = ijmi(&nusus);

    assert_eq!(
        adad_sufuf(&huzma),
        2,
        "one row per source string, not one per project entry"
    );
    assert_eq!(
        tarjama_fil_huzma(&huzma, "Back"),
        RUJU,
        "the reading two of the three entries already carried is the one that ships"
    );
    assert_eq!(
        tarjama_fil_huzma(&huzma, "Play"),
        "ابدأ",
        "the undisputed string is untouched"
    );

    assert_eq!(huzma.tawhid.len(), 1, "exactly one source string disagreed");
    let Some(wahid) = huzma.tawhid.first() else {
        panic!("the disagreement is reported")
    };
    assert_eq!(wahid.masdar, "Back");
    assert_eq!(wahid.mukhtara, RUJU);
    assert_eq!(wahid.muwafiqa, 2);
    assert_eq!(
        wahid.matruka,
        vec![(AWDA.to_owned(), 1)],
        "what was set aside is named, with the number of entries that carried it"
    );
}

/// A project that agrees with itself reports nothing and is unaffected.
#[test]
fn ikhtibar_mashru_muttafiq_la_yuwahhad() {
    let nusus = vec![
        mudkhal("ui/menu.txt", "Back", RUJU),
        mudkhal("ui/pause.txt", "Back", RUJU),
        mudkhal("ui/menu.txt", "Play", "ابدأ"),
    ];
    let huzma = ijmi(&nusus);

    assert!(
        huzma.tawhid.is_empty(),
        "nothing disagreed, so nothing is reported"
    );
    assert_eq!(adad_sufuf(&huzma), 2);
    assert_eq!(tarjama_fil_huzma(&huzma, "Back"), RUJU);
}

/// A translation a human approved beats one that merely occurs more often.
#[test]
fn ikhtibar_al_muakkada_taghlib_al_aghlabiya() {
    let nusus = vec![
        mudkhal("ui/menu.txt", "Back", RUJU),
        mudkhal("ui/shop.txt", "Back", RUJU),
        muakkad(mudkhal("ui/pause.txt", "Back", AWDA)),
    ];
    let huzma = ijmi(&nusus);

    assert_eq!(
        tarjama_fil_huzma(&huzma, "Back"),
        AWDA,
        "somebody read this one and said yes; the other two are a provider repeating itself"
    );
    let Some(wahid) = huzma.tawhid.first() else {
        panic!("the disagreement is reported")
    };
    assert_eq!(wahid.mukhtara, AWDA);
    assert_eq!(
        wahid.muwafiqa, 1,
        "the winner is reported with its own count, not the majority's"
    );
}

/// The choice does not depend on the order the entries arrive in.
///
/// The rule is decided by the data — approval, then agreement, then the lowest
/// identity — so a caller that sorted its project differently, or an importer
/// that walked containers in another order, gets the same patch.
#[test]
fn ikhtibar_al_ikhtiyar_la_yatabi_al_tarteeb() {
    let mut nusus = vec![
        mudkhal("ui/menu.txt", "Back", RUJU),
        mudkhal("ui/pause.txt", "Back", AWDA),
        mudkhal("ui/shop.txt", "Back", RUJU),
    ];
    let amami = tarjama_fil_huzma(&ijmi(&nusus), "Back");
    nusus.reverse();
    let khalfi = tarjama_fil_huzma(&ijmi(&nusus), "Back");

    assert_eq!(
        amami, khalfi,
        "the same project in the other order picks the same translation"
    );
    assert_eq!(amami, RUJU);
}

/// A losing entry's precomputed layout does not survive into the package.
///
/// The subtler half of the fix. Layouts are stored per string handle, and every
/// entry that disagreed collapses onto one handle — so resolving the
/// disagreement only where the row is written would leave a layout under the
/// winning string whose glyphs spell the losing one, and the game would draw it.
/// The two readings here differ in length, which is what makes that visible:
/// `رجوع` is four letters and `العودة` is six.
#[test]
fn ikhtibar_al_takhtit_yatbaa_al_tarjama_al_fayiza() {
    let nusus = vec![
        mudkhal("ui/menu.txt", "Back", RUJU),
        mudkhal("ui/pause.txt", "Back", AWDA),
        mudkhal("ui/shop.txt", "Back", RUJU),
    ];
    let huzma = ijmi(&nusus);
    assert_eq!(tarjama_fil_huzma(&huzma, "Back"), RUJU);

    let khaam = huzma.bayt.bayt();
    let ruqaa = Ruqaa::iftah(khaam).expect("the package reopens");
    let qism = ruqaa.qism(NawQism::Takhtit).expect("a layout section");
    let jadwal = qari::takhtit(qism.bayt()).expect("the layout section reads back");
    assert_eq!(
        jadwal.ruus.len(),
        1,
        "one string at one size is one layout, not three"
    );

    let Some(ras) = jadwal.ruus.first() else {
        panic!("the layout is present")
    };
    let (huruf, _) = jadwal.muhtawa(ras).expect("the layout's glyphs");
    assert_eq!(
        huruf.len(),
        RUJU.chars().count(),
        "the stored layout spells the translation the container holds"
    );
    assert_ne!(
        huruf.len(),
        AWDA.chars().count(),
        "and not the one that was set aside — no ligature makes these two lengths agree"
    );
}
