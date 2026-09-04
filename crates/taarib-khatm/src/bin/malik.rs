//! Owner-side provisioning: import the owner seed into this machine's keychain.

// The print ban exists so that no library writes to a terminal the application
// owns. This is not a library: it is a one-command tool whose entire interface
// is a line on stdout and a refusal on stderr, run by hand on the owner's
// machine. There is no `Khata` to return to anybody.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]

use std::io::Read as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut nassi = String::new();
    if std::io::stdin().read_to_string(&mut nassi).is_err() {
        eprintln!("read the 64-hex-character owner seed from stdin");
        return ExitCode::FAILURE;
    }
    let nassi = nassi.trim();
    if nassi.len() != 64 || !nassi.bytes().all(|b| b.is_ascii_hexdigit()) {
        eprintln!("the seed must be exactly 64 hexadecimal characters");
        return ExitCode::FAILURE;
    }
    let mut bidhra = [0u8; 32];
    if hex::decode_to_slice(nassi, &mut bidhra).is_err() {
        eprintln!("the seed did not decode as hexadecimal");
        return ExitCode::FAILURE;
    }

    match taarib_khatm::malik::khzin_malik(&bidhra) {
        Ok(aam) => {
            println!("owner key stored in the keychain; public half {}", hex::encode(aam.bayt()));
            ExitCode::SUCCESS
        }
        Err(khata) => {
            eprintln!("refused: {khata}");
            ExitCode::FAILURE
        }
    }
}
