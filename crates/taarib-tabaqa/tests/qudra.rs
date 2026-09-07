//! The fourth verdict, asserted.
//!
//! `qudra.rs` had three answers — supported, limited, refused — and every
//! producer that could not ask a question had to pick one of them for a fact it
//! did not have. Two picked "supported" by returning early with nothing to say.
//! [`HukmQudra::Majhula`] is the answer for that case, and these tests are what
//! fails if it is ever folded back into one of the other three: it clears no
//! gate, it is not an evidence-backed verdict, it outranks every known limit
//! and yields to a known refusal, and a report carrying one is not clean.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use taarib_tabaqa::qudra::{HukmQudra, MilShasha, QudratTarkeeb, SababQudra};
use taarib_tabaqa::wajiha::WajihatRusum;

/// Every verdict, worst last.
const AHKAM: [HukmQudra; 4] = [
    HukmQudra::Kamila,
    HukmQudra::Naqisa,
    HukmQudra::Majhula,
    HukmQudra::Mustaheela,
];

/// A report with no findings, to add to.
const fn farigh() -> QudratTarkeeb {
    QudratTarkeeb::jadeeda(WajihatRusum::Direct3D9, MilShasha::KhilalAlJihaz)
}

/// "Not determined" offers nothing and proves nothing.
///
/// Only the two verdicts that established something clear the gate — the rule
/// `taarib_usus::manassa::HalatTashghil::yamnaa` applies to a process table
/// that could not be read. Merging the unknown into "supported" or into
/// "limited" flips the first assertion; merging it into "refused" flips the
/// second.
#[test]
fn al_majhul_la_yaftah_al_bawwaba_wala_yuthbit() {
    assert!(HukmQudra::Kamila.qabila());
    assert!(HukmQudra::Naqisa.qabila());
    assert!(
        !HukmQudra::Majhula.qabila(),
        "an unanswered question is not a yes"
    );
    assert!(!HukmQudra::Mustaheela.qabila());

    assert!(HukmQudra::Kamila.hasim());
    assert!(HukmQudra::Naqisa.hasim());
    assert!(
        !HukmQudra::Majhula.hasim(),
        "an unanswered question is not evidence"
    );
    assert!(
        HukmQudra::Mustaheela.hasim(),
        "a refusal is a determination"
    );
}

/// An unknown outranks every known limit and yields to a known refusal.
#[test]
fn tarteeb_al_ahkam() {
    for zawj in AHKAM.windows(2) {
        let (Some(adna), Some(aala)) = (zawj.first(), zawj.get(1)) else {
            panic!("the window is two wide by construction");
        };
        assert!(adna < aala, "{adna} must sort below {aala}");
    }
    assert!(HukmQudra::Naqisa < HukmQudra::Majhula);
    assert!(HukmQudra::Majhula < HukmQudra::Mustaheela);
}

/// A report that could not ask one question is not a clean report, whatever
/// else it found.
#[test]
fn taqrir_bi_sual_bila_jawab_laysa_kamilan() {
    let taqrir = farigh()
        .maa(SababQudra::kamila("وُجدت المكتبة.", "the module is loaded"))
        .maa(SababQudra::majhula(
            "لم تُسأل صيغة البكسل.",
            "the pixel format could not be asked for",
        ));
    assert_eq!(taqrir.hukm(), HukmQudra::Majhula);
    assert!(
        !taqrir.qabila(),
        "a report with an unasked stopping question offers nothing"
    );
    assert_eq!(taqrir.majhulat().count(), 1);
    assert!(
        taqrir
            .sutur()
            .iter()
            .any(|satr| satr.contains("not determined")),
        "the bundle line must carry the word: {:?}",
        taqrir.sutur()
    );

    let mahdud = farigh()
        .maa(SababQudra::naqisa("حدّ.", "a limit"))
        .maa(SababQudra::majhula("مجهول.", "an unknown"));
    assert_eq!(
        mahdud.hukm(),
        HukmQudra::Majhula,
        "an unknown outranks a known limit"
    );

    let marfud = farigh()
        .maa(SababQudra::majhula("مجهول.", "an unknown"))
        .maa(SababQudra::mustaheela("رفض.", "a refusal"));
    assert_eq!(
        marfud.hukm(),
        HukmQudra::Mustaheela,
        "a known refusal is not made less certain"
    );

    let nazeef = farigh().maa(SababQudra::kamila("سليم.", "fine"));
    assert_eq!(nazeef.hukm(), HukmQudra::Kamila);
    assert_eq!(nazeef.majhulat().count(), 0);
    assert!(
        farigh().qabila(),
        "a report with nothing to say found nothing wrong"
    );
}

/// Every verdict has its own word in both languages.
#[test]
fn likulli_hukm_kalimatuh() {
    for (fahras, hukm) in AHKAM.iter().enumerate() {
        assert!(!hukm.ism().is_empty());
        assert!(!hukm.ism_arabi().is_empty());
        for akhar in AHKAM.iter().skip(fahras.saturating_add(1)) {
            assert_ne!(
                hukm.ism(),
                akhar.ism(),
                "{hukm:?} and {akhar:?} share a word"
            );
            assert_ne!(
                hukm.ism_arabi(),
                akhar.ism_arabi(),
                "{hukm:?} and {akhar:?} share a word"
            );
        }
    }
    assert_eq!(HukmQudra::Majhula.ism(), "not determined");
    assert_eq!(HukmQudra::Majhula.to_string(), "not determined");
}
