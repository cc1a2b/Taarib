//! The readiness report, held to the shape a caller may act on.
//!
//! These tests cannot check whether the table is *true* — no program can read
//! off which of its own functions have callers, which is why `docs/tashghil.md`
//! is a grep and this module's header says so. What they can check is that the
//! report is internally consistent, that its verdict follows from its rows
//! rather than from an opinion typed beside them, and that every row carries the
//! sentences a user is going to be shown. A row with an empty sentence is a
//! screen with a blank on it.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; refusing to panic here would mean a test \
              that cannot fail"
)]

use taarib_muhawwil_unreal::jahiziya::{
    HalatQudra, QUDRAT, Qudra, bayan, hala, hamula_mabniya, jahiziya, naqs, taqreer,
};
use taarib_mustalahat::muharrik::JahiziyatTashghil;

#[test]
fn every_capability_carries_both_sentences() {
    for qudra in QUDRAT {
        let bayan = bayan(qudra);
        assert!(
            !bayan.arabi.trim().is_empty(),
            "{} has no Arabic sentence",
            qudra.ism()
        );
        assert!(
            !bayan.injilizi.trim().is_empty(),
            "{} has no English sentence",
            qudra.ism()
        );
        assert!(!qudra.ism().is_empty());
        assert!(!qudra.wahda().is_empty());
    }
}

#[test]
fn arabic_goes_in_as_ordinary_unicode() {
    // Presentation forms are deliberately never produced anywhere in this
    // project: the shaper produces them, a source string must not carry them.
    for bayan in taqreer() {
        for harf in bayan.arabi.chars() {
            let raqm = u32::from(harf);
            assert!(
                !(0xFB50..=0xFDFF).contains(&raqm) && !(0xFE70..=0xFEFF).contains(&raqm),
                "{} carries an Arabic presentation form: U+{raqm:04X}",
                bayan.qudra.ism()
            );
        }
    }
    if let Some(hadd) = naqs() {
        for harf in hadd.arabi.chars() {
            let raqm = u32::from(harf);
            assert!(
                !(0xFB50..=0xFDFF).contains(&raqm) && !(0xFE70..=0xFEFF).contains(&raqm),
                "the readiness sentence carries U+{raqm:04X}"
            );
        }
    }
}

#[test]
fn the_report_agrees_with_the_states_it_reports() {
    let taqreer = taqreer();
    assert_eq!(taqreer.len(), QUDRAT.len());
    for (bayan, qudra) in taqreer.iter().zip(QUDRAT) {
        assert_eq!(bayan.qudra, qudra);
        assert_eq!(bayan.hala, hala(qudra));
    }
}

#[test]
fn the_verdict_follows_from_the_rows() {
    let kull = QUDRAT.iter().all(|qudra| hala(*qudra).tajri());
    let yaktub = hala(Qudra::Kitaba).tajri();
    let mutawaqqa = if yaktub {
        if kull {
            JahiziyatTashghil::Mukammala
        } else {
            JahiziyatTashghil::Naqisa
        }
    } else {
        JahiziyatTashghil::Ghaiba
    };
    assert_eq!(jahiziya(), mutawaqqa);
}

#[test]
fn a_sentence_is_offered_exactly_when_something_is_missing() {
    assert_eq!(naqs().is_some(), jahiziya() != JahiziyatTashghil::Mukammala);
}

#[test]
fn the_shaping_rung_tracks_the_feature_that_carries_it() {
    // The one mechanical row. Without `hamula` this crate exports no bootstrap,
    // so nothing drives `Tashghil` and the row must say so.
    let mutawaqqa = if hamula_mabniya() {
        HalatQudra::Amila
    } else {
        HalatQudra::Ghaiba
    };
    assert_eq!(hala(Qudra::Tashghil), mutawaqqa);
}

#[test]
fn reading_and_writing_are_what_this_build_actually_does() {
    // If this ever stops holding, the extractor has lost its Unreal path and
    // the row above it is the first thing to check.
    assert_eq!(hala(Qudra::Qiraa), HalatQudra::Amila);
    // The container reaches the game through the installer's Unreal arm.
    assert_eq!(hala(Qudra::Kitaba), HalatQudra::Amila);
    // And the honest headline: the container is written, the engine draws it
    // on its own, and nobody has yet watched it happen — so not complete.
    assert_eq!(jahiziya(), JahiziyatTashghil::Naqisa);
    // The in-process corrections still reach nothing, and the row says so.
    assert_eq!(hala(Qudra::Qiyas), HalatQudra::Ghaiba);
}
