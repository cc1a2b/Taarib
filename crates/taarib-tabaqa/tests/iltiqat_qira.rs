//! The join between preprocessing and recognition, and the refusal that sits
//! on top of it.
//!
//! Every garbage string asserted on below is a **real** read the portable
//! engine produced during the phase-29 measurement, against real Steam library
//! artwork and real in-game screenshots. They are quoted rather than invented
//! so that the thresholds are being tested against the failures they were set
//! from.

use std::error::Error;

use taarib_tabaqa::iltiqat_shasha::{
    IdadatTahsin, MuhassinSura, SuraMultaqata, SuratRamadiya,
};
use taarib_tabaqa::qira::{
    HududQubul, HukmQira, SatrMaqru, THIQA_GHAYR_MAQISA, ThiqatMintaqa, hukm_bunya,
    hukm_tawafuq, ihsa_bunya, kalima_mushawwaha, khilaf_qiraatayn,
};
use taarib_tabaqa::wajiha::{MustatilBiksel, SighatSath};

/// What every test here returns, so a failure inside a helper says which call
/// refused and why rather than unwinding on an `expect` message somebody wrote
/// before they knew what could go wrong.
type Natija = Result<(), Box<dyn Error>>;

/// A rectangle of one flat colour, as a capture.
fn multaqata(ard: u32, irtifa: u32, lawn: [u8; 3]) -> Result<SuraMultaqata, Box<dyn Error>> {
    let mut bayt = Vec::new();
    for _ in 0..(ard as usize) * (irtifa as usize) {
        bayt.extend_from_slice(&[lawn[2], lawn[1], lawn[0], 255]);
    }
    Ok(SuraMultaqata::jadeeda(
        bayt,
        ard,
        irtifa,
        SighatSath::Bgra8,
        MustatilBiksel { yasar: 300, aala: 700, ard, irtifa },
    )?)
}

fn satr(nass: &str) -> SatrMaqru {
    SatrMaqru::jadeed(
        nass,
        MustatilBiksel { yasar: 0, aala: 0, ard: 100, irtifa: 20 },
        THIQA_GHAYR_MAQISA,
        false,
    )
}

// ---------------------------------------------------------------------------
// The bridge
// ---------------------------------------------------------------------------

#[test]
fn grayscale_reaches_a_recognizer_with_its_values_intact() -> Natija {
    let ramadi = SuratRamadiya::jadeeda(vec![0, 64, 128, 255], 2, 2)?;
    let sura = ramadi.ila_multaqata()?;
    assert_eq!(sura.sigha(), SighatSath::Rgba8);
    assert_eq!(sura.ard(), 2);
    assert_eq!(sura.irtifa(), 2);

    // The value has to survive into all three channels, because a recognizer
    // that is handed one channel reads a sheared image and reports no text.
    let rgb = sura.ila_rgb()?;
    assert_eq!(rgb, vec![0, 0, 0, 64, 64, 64, 128, 128, 128, 255, 255, 255]);
    Ok(())
}

#[test]
fn a_short_region_is_upscaled_and_the_boxes_come_back_at_the_original_scale() -> Natija {
    // Twelve pixels of text is under the 20-pixel floor, so the factor is two.
    let sura = multaqata(80, 12, [30, 30, 34])?;
    let mut muhassin = MuhassinSura::jadeed(IdadatTahsin::default());
    let muhassana = muhassin.hassin_lil_qari(&sura)?;

    assert_eq!(muhassana.mudaaf(), 2);
    assert_eq!(muhassana.sura().irtifa(), 24);
    assert_eq!(muhassana.sura().ard(), 160);
    assert_eq!(muhassana.mintaqa(), sura.mintaqa());

    // A box the recognizer reports inside the upscaled image has to land back
    // on the surface, not at twice the offset from the region's origin.
    let mahalli = MustatilBiksel { yasar: 20, aala: 4, ard: 60, irtifa: 16 };
    let sathi = muhassana.ila_sath(mahalli);
    assert_eq!(
        sathi,
        MustatilBiksel { yasar: 310, aala: 702, ard: 30, irtifa: 8 },
        "the factor has to be divided out before the region's origin is added"
    );
    Ok(())
}

#[test]
fn a_tall_enough_region_is_not_upscaled_and_maps_one_to_one() -> Natija {
    let sura = multaqata(64, 40, [200, 200, 205])?;
    let mut muhassin = MuhassinSura::jadeed(IdadatTahsin::default());
    let muhassana = muhassin.hassin_lil_qari(&sura)?;
    assert_eq!(muhassana.mudaaf(), 1);
    let mahalli = MustatilBiksel { yasar: 7, aala: 9, ard: 11, irtifa: 13 };
    assert_eq!(
        muhassana.ila_sath(mahalli),
        MustatilBiksel { yasar: 307, aala: 709, ard: 11, irtifa: 13 }
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Word shape
// ---------------------------------------------------------------------------

#[test]
fn ordinary_game_text_is_not_malformed() {
    for kalima in [
        "Continue", "QUIT", "MATCH", "Radahn's", "503425", "90:00", "Waypoint", "XI", "A",
        "progress?", "Blacksmith's", "24", "/", "-", "Professional", "78%",
    ] {
        assert!(!kalima_mushawwaha(kalima), "{kalima} is ordinary game text");
    }
}

#[test]
fn the_four_malformed_shapes_are_caught() {
    // A case change inside a word.
    assert!(kalima_mushawwaha("iAW"));
    assert!(kalima_mushawwaha("TMMCOMANTCNLiRIUESSOSLO"));
    // Letters and digits in one token.
    assert!(kalima_mushawwaha("Me2"));
    assert!(kalima_mushawwaha("O1O00TC"));
    assert!(kalima_mushawwaha("BEUA8G"));
    // A stray single character that is not a word.
    assert!(kalima_mushawwaha("e"));
    assert!(kalima_mushawwaha("B"));
    // All punctuation, more than one character.
    assert!(kalima_mushawwaha("?!"));
}

#[test]
fn a_hyphen_resets_the_case_test_and_that_is_a_known_hole() {
    // `REBSDTo-EH` is a real garbage token the engine produced, and it is not
    // caught: the hyphen resets the case state, because `well-Known` is
    // ordinary text and a rule that fired on it would refuse real dialogue.
    // The line the token came from is refused anyway, on its other tokens —
    // which is the honest shape of this gate. It judges lines, not words.
    assert!(!kalima_mushawwaha("REBSDTo-EH"));
    assert!(!kalima_mushawwaha("well-Known"));
    let hudud = HududQubul::default();
    let khaam = "e d B R M BEUA8G ?R THENE DARK REVIVAL? REBSDTo-EH";
    assert!(
        !hukm_bunya(&[satr(khaam)], &hudud).maqbul(),
        "five stray single letters and a letter-digit token carry the line"
    );
}

#[test]
fn an_unreadable_glyph_marker_is_junk_and_a_question_mark_is_not() {
    // The portable engine emits `?` for a glyph it could not identify. At the
    // end of a token it is punctuation; in the middle it is the engine saying
    // it did not read that region.
    assert_eq!(ihsa_bunya("Save your progress?").alamat_majhula, 0);
    assert_eq!(ihsa_bunya("What?!").alamat_majhula, 0, "punctuation may follow punctuation");
    assert_eq!(ihsa_bunya("i?").alamat_majhula, 0);

    assert_eq!(ihsa_bunya("FW?REDLANTERN").alamat_majhula, 1);
    assert_eq!(ihsa_bunya("?R THENE").alamat_majhula, 1);
}

// ---------------------------------------------------------------------------
// The structural gate
// ---------------------------------------------------------------------------

#[test]
fn real_reads_of_real_game_ui_are_accepted() {
    let hudud = HududQubul::default();
    for nass in [
        "QUIT MATCH RESTART OPTIONS MATCH FACTS PERFORMANCE HIGHLIGHTER",
        "90:00 Serie A XI",
        "Spectral Steed Whistle",
        "Teleport to Waypoint T",
        "Unturned Dedicated Server",
        "503425",
        "The old road north is closed.",
        "Autosaving. Do not turn off the console.",
    ] {
        assert_eq!(
            hukm_bunya(&[satr(nass)], &hudud),
            HukmQira::Maqbul,
            "{nass} was read off a real game screen"
        );
    }
}

#[test]
fn measured_garbage_is_refused_with_a_sentence() {
    let hudud = HududQubul::default();
    for nass in [
        // From Steam library logos the engine could not read.
        "N 4 2 ? i m i? iAW w e Me2 AND DARKREVIVALSJJieTHE",
        "TMMCOMANTCNLiRIUESSOSLO",
        "FW?REDLANTERN",
        "e d B R M BEUA8G ?R THENE DARK REVIVAL? REBSDTo-EH",
        "C [ Wu O1O00TC",
    ] {
        let hukm = hukm_bunya(&[satr(nass)], &hudud);
        assert!(hukm.sabab().is_some(), "{nass} should be refused");
        assert!(
            hukm.sabab().is_some_and(|s| s.len() > 30),
            "the refusal has to be a sentence a player can act on, not a code"
        );
    }
}

#[test]
fn a_region_read_as_almost_nothing_is_refused() {
    let hudud = HududQubul::default();
    assert!(!hukm_bunya(&[satr("S")], &hudud).maqbul());
    assert!(!hukm_bunya(&[], &hudud).maqbul());
}

#[test]
fn short_sparse_garbage_gets_past_the_structural_gate() {
    // Both of these are real reads of images that carry no text at all — an
    // Elden Ring splash screen and a Steam controller diagram. Nothing about
    // their *shape* is wrong: `Ew` and `Wu` are legal letter sequences, `[` is
    // legal punctuation, and four characters is not too few for `1044`. The
    // structural gate cannot refuse them and does not pretend to; the
    // corroboration gate below is what catches this class.
    let hudud = HududQubul::default();
    assert!(hukm_bunya(&[satr("Ew 4")], &hudud).maqbul());
    // And a plausible wrong read of a real wordmark: nothing about the shape of
    // `MARVEL GUPKNS` is wrong, and the ground truth is
    // `MARVEL GUARDIANS OF THE GALAXY`.
    assert!(hukm_bunya(&[satr("MARVEL GUPKNS")], &hudud).maqbul());
    // And a wordmark read as one long capital run: structurally it is
    // indistinguishable from `HIGHLIGHTER`, and the ground truth is
    // `THE RED LANTERN`.
    assert!(hukm_bunya(&[satr("AWWCOMANTERNLAUUEICL")], &hudud).maqbul());
}

// ---------------------------------------------------------------------------
// The corroboration gate
// ---------------------------------------------------------------------------

#[test]
fn two_paths_that_agree_are_accepted_and_two_that_diverge_are_not() {
    let hudud = HududQubul::default();

    // Measured: the raw path and the preprocessed path on a Steam logo whose
    // ground truth is "Unturned Dedicated Server" — one character apart, and
    // the preprocessed one is right.
    let khaam = [satr("unturned Dedicated Server")];
    let muhassan = [satr("Unturned Dedicated Server")];
    assert_eq!(hukm_tawafuq(&khaam, &muhassan, &hudud), HukmQira::Maqbul);
    assert!(khilaf_qiraatayn(&khaam, &muhassan) < 0.1);

    // Measured: the same two paths on "THE RED LANTERN", which neither read.
    let khaam = [satr("TMMCOMANTCNLiRIUESSOSLO")];
    let muhassan = [satr("AMARCOMAE")];
    assert!(!hukm_tawafuq(&khaam, &muhassan, &hudud).maqbul());
}

#[test]
fn a_region_with_no_text_makes_the_two_paths_disagree() {
    // Both of these are what the engine returned for images that carry no text.
    let hudud = HududQubul::default();
    let khaam = [satr("C [ Wu O1O00TC")];
    let muhassan = [satr("ooo W O8C")];
    assert!(!hukm_tawafuq(&khaam, &muhassan, &hudud).maqbul());
}

// ---------------------------------------------------------------------------
// Honest confidence
// ---------------------------------------------------------------------------

#[test]
fn an_engine_that_measures_no_confidence_reports_none_rather_than_the_stand_in() {
    let hudud = HududQubul::default();
    let taqreer = ThiqatMintaqa::min_sutur(&[satr("Classic Match")], &hudud);
    assert_eq!(taqreer.thiqa, None, "the stand-in must never reach a caller as a number");
    assert!(taqreer.wasf().contains("unmeasured"));
    assert!(taqreer.hukm.maqbul());
}

#[test]
fn an_engine_that_does_measure_confidence_reports_its_lowest_line() {
    let hudud = HududQubul::default();
    let mawdi = MustatilBiksel { yasar: 0, aala: 0, ard: 100, irtifa: 20 };
    let sutur = [
        SatrMaqru::jadeed("Classic Match", mawdi, 91, true),
        SatrMaqru::jadeed("Serie A XI", mawdi, 74, true),
    ];
    let taqreer = ThiqatMintaqa::min_sutur(&sutur, &hudud);
    assert_eq!(taqreer.thiqa, Some(74), "a paragraph is as trustworthy as its worst line");
    assert!(taqreer.wasf().contains("74%"));
}

#[test]
fn one_unmeasured_line_makes_the_whole_region_unmeasured() {
    let hudud = HududQubul::default();
    let mawdi = MustatilBiksel { yasar: 0, aala: 0, ard: 100, irtifa: 20 };
    let sutur = [
        SatrMaqru::jadeed("Classic Match", mawdi, 91, true),
        SatrMaqru::jadeed("Serie A XI", mawdi, THIQA_GHAYR_MAQISA, false),
    ];
    assert_eq!(ThiqatMintaqa::min_sutur(&sutur, &hudud).thiqa, None);
}
