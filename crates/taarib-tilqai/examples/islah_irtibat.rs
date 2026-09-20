//! Repairs a project whose record cannot bind a package to a game build.
//!
//! `IrtibatBina::min_bayan` refuses a record naming neither a launcher build
//! nor a fingerprint, so a project carrying neither answers `TAARIB-E-6107` at
//! submission — and the value it wants is a measurement of an installed game,
//! not something a person can type. Projects published before the run learned
//! to record one are in exactly that state.
//!
//! Takes the measurement from the game and writes it, using the same builder
//! the compile stage uses so the repaired record is the one a fresh run would
//! have written. A record that already binds is left alone: its fingerprint was
//! taken when the game was the build the project was made from.
//!
//!     islah_irtibat <project-dir> <game-dir> <run-journal-dir>

// A one-command repair run by hand; its output is its interface.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use taarib_istikhraj::mashru::MashruMaftuh;
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_tilqai::mashwar::{QaydMarhala, SijillMashwar};
use taarib_tilqai::taqaddum::MarhalaTilqai;

fn main() -> ExitCode {
    let hujaj: Vec<String> = std::env::args().skip(1).collect();
    let [mashru_masar, luba_masar, mashwar_masar] = match hujaj.as_slice() {
        [a, b, c] => [a.clone(), b.clone(), c.clone()],
        _ => {
            eprintln!("usage: islah_irtibat <project-dir> <game-dir> <run-journal-dir>");
            return ExitCode::from(2);
        },
    };

    let mut mashru = match MashruMaftuh::iftah(PathBuf::from(&mashru_masar)) {
        Ok(mashru) => mashru,
        Err(khata) => {
            eprintln!("{mashru_masar}: {khata}");
            return ExitCode::from(2);
        },
    };
    if taarib_tilqai::warsha::yarbut(&mashru.rasm().bayan) {
        println!("already binds; nothing to repair");
        return ExitCode::SUCCESS;
    }

    // The probe the run already took, rather than a fresh one: the record must
    // describe the build the project was made from.
    let Ok(sijill) = SijillMashwar::iftah(&PathBuf::from(&mashwar_masar)) else {
        eprintln!("{mashwar_masar}: no run journal");
        return ExitCode::from(2);
    };
    let Some(QaydMarhala::Fahs { imkaniyat }) = sijill.qayd(MarhalaTilqai::Fahs) else {
        eprintln!("{mashwar_masar}: the journal records no capability report");
        return ExitCode::from(2);
    };
    let imkaniyat: &TaqreerImkaniyat = imkaniyat;

    let waqt = mashru.rasm().waqt_tabdeel.clone();
    if let Err(khata) =
        taarib_tilqai::warsha::aslih(&mut mashru, &PathBuf::from(&luba_masar), imkaniyat, &waqt)
    {
        eprintln!("the game could not be measured: {khata}");
        return ExitCode::from(2);
    }

    let bayan = &mashru.rasm().bayan;
    println!("repaired");
    println!("  bina_manassa  {:?}", bayan.bina_manassa);
    println!("  basmat_luba   {:?}", bayan.basmat_luba);
    println!("  turuq         {}", bayan.turuq.len());
    ExitCode::SUCCESS
}
