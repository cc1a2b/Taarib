//! Checks a revocation list file against the trust anchor this build carries.
//!
//! Written to answer one question that could not otherwise be answered without
//! launching the product: when a client reports `TAARIB-E-6303`, is the list it
//! refused the one the registry serves, or the one compiled into the binary?
//! Those have different causes and different fixes — a stale published list
//! against a re-keyed build, versus a build whose `badhra` stage was skipped —
//! and guessing between them costs a release.
//!
//! Verification needs only the public anchor, so this runs anywhere; it never
//! opens the keychain and cannot sign. Point it at a `qaima.json` and export
//! `TAARIB_MIFTAH_ISDAR` with `--features taarib-khatm/isdar` to ask the
//! question as a release build would ask it.

// The print ban exists so that no library writes to a terminal the application
// owns. This is not a library: it is a one-command check whose entire interface
// is two lines on stdout, run by hand. There is no `Khata` to return to anybody.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(masar) = std::env::args().nth(1) else {
        eprintln!("usage: tahaqquq_qaima <qaima.json>");
        return ExitCode::from(2);
    };
    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(khata) => {
            eprintln!("{masar}: {khata}");
            return ExitCode::from(2);
        },
    };
    let miftah = match taarib_khatm::MiftahAam::min_bayt(&taarib_khatm::MIRSAT_MALIK.miftah) {
        Ok(miftah) => miftah,
        Err(khata) => {
            eprintln!("the anchor this build carries is not a public key: {khata}");
            return ExitCode::from(2);
        },
    };
    println!("anchor  {}", hex::encode(taarib_khatm::MIRSAT_MALIK.miftah));
    match taarib_aman::qaimat_sahb::QaimatSahb::min_bayt(&bayt, &miftah) {
        Ok(qaima) => {
            println!("VERIFIES  sequence {}", qaima.tasalsul());
            ExitCode::SUCCESS
        },
        Err(khata) => {
            println!("REFUSED   {khata}");
            ExitCode::FAILURE
        },
    }
}
