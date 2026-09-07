//! Owner-side provisioning: mint the release signing key and publish its anchor.
//!
//! ## What this is for
//!
//! A release build of Taarib anchors to a public key injected at build time
//! through `TAARIB_MIFTAH_ISDAR`, and refuses to build without one when the
//! `isdar` feature is on. This mints the pair behind that anchor.
//!
//! Three things it will not do, none of them by policy and all of them by
//! construction:
//!
//! - **It never writes a private key to a file.** There is no argument that
//!   asks for one, no branch that opens a file for the seed, and the seed is
//!   not printed. The only sink for the private half is
//!   [`taarib_khatm::mafatih::khzin`], which is the OS keychain.
//! - **It refuses to write the anchor over an existing file.** An anchor that
//!   is silently replaced is a build that silently changes what it trusts.
//! - **It refuses to replace a stored key without being told twice.** The
//!   second telling is the passphrase, typed again.
//!
//! The passphrase is the **keychain's**, not this tool's. Taarib has one
//! custody mechanism for private keys — the platform credential store, through
//! `keyring` — and the protection on the seed is whatever that store gives it:
//! the login keyring's passphrase on Linux, the login keychain's on macOS, the
//! user account's DPAPI material on Windows. This tool does not derive the key
//! from the passphrase and does not encrypt anything with it, because either
//! would put a second, weaker custody path beside the one the product already
//! has. What it does is demand the passphrase before it writes, and demand it
//! again before it overwrites, so the write cannot happen on an unattended
//! terminal — and it names, on the way out, exactly what is protecting the
//! seed, so the owner can see whether their store is locked or wide open.

// The print ban exists so that no library writes to a terminal the application
// owns. This is not a library: it is a one-command tool whose entire interface
// is a few lines on stdout and a refusal on stderr, run by hand on the owner's
// machine. There is no `Khata` to return to anybody.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]

use std::io::{BufRead as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use taarib_khatm::malik::{ISM_MIFTAH_MALIK, MIFTAH_TATWIR};
use taarib_khatm::{MiftahAam, MiftahKhass, mafatih};

/// Shortest passphrase the tool will accept before it writes.
const HADD_IBARA: usize = 12;

/// The keychain account the probe writes under, which is never a signing key.
const ISM_JASS: &str = "isdar.jass";

/// What the probe stores, so a stray entry is self-describing.
const QEEMAT_JASS: &str = "taarib isdar keychain probe";

/// Words in a file name that mean the operator expected a private key there.
const ASMAA_SIRR: [&str; 6] = ["seed", "secret", "private", "priv", ".key", "bidhra"];

fn main() -> ExitCode {
    let hujaj: Vec<String> = std::env::args().skip(1).collect();
    let amr = hujaj.first().map(String::as_str);
    let natija = match amr {
        Some("wallid") => wallid(&hujaj),
        Some("mirsa") => mirsa(&hujaj),
        Some("tahaqquq") => tahaqquq(&hujaj),
        _ => {
            istikhdam();
            return ExitCode::from(2);
        },
    };
    match natija {
        Ok(()) => ExitCode::SUCCESS,
        Err(sabab) => {
            eprintln!("refused: {sabab}");
            ExitCode::FAILURE
        },
    }
}

fn istikhdam() {
    println!("taarib-isdar — the release signing key and its public anchor");
    println!();
    println!("  isdar wallid   --mirsa <file> [--ism <account>]");
    println!("      Mints a release key, stores the private half in this machine's keychain,");
    println!("      and writes the 64-hex public anchor to <file>. Refuses to overwrite it.");
    println!();
    println!("  isdar mirsa    [--ism <account>]");
    println!("      Prints the public anchor of the key already in the keychain.");
    println!();
    println!("  isdar tahaqquq --mirsa <file> [--ism <account>]");
    println!("      Checks that the stored key still derives the anchor in <file>.");
    println!();
    println!("  The private half is never written to a file, never printed, and never");
    println!("  placed in an environment variable. The only place it goes is the keychain.");
}

// ---------------------------------------------------------------------------
// wallid — mint
// ---------------------------------------------------------------------------

/// Mints a key, stores it, and writes the anchor.
fn wallid(hujaj: &[String]) -> Result<(), String> {
    let ism = ism_hisab(hujaj);
    let mirsa = masar_mirsa(hujaj)?;
    if mirsa.exists() {
        return Err(format!(
            "{} already exists. An anchor that is silently replaced is a build that silently \
             changes what it trusts; move the old one aside deliberately.",
            mirsa.display()
        ));
    }
    // The anchor is the only file this tool writes, and it must be writable
    // before a key exists rather than after, so a failure here cannot leave a
    // stored key whose anchor was never published.
    let walid = mirsa.parent().filter(|walid| !walid.as_os_str().is_empty());
    if let Some(walid) = walid
        && !walid.is_dir()
    {
        return Err(format!("{} is not a directory", walid.display()));
    }

    println!("keychain: {}", wasf_khazna());
    jass_khazna()?;
    println!("  a probe entry was written, read back and deleted, so the store is reachable");

    let mawjud = mafatih::hat(&ism).ok();
    if let Some(qadeem) = &mawjud {
        println!();
        println!(
            "a key is already stored under {ism:?}: {}",
            hex::encode(qadeem.aam().bayt())
        );
        println!("replacing it makes every package signed under the old one unverifiable by a");
        println!("client built against its anchor. The passphrase is asked for twice below.");
    }

    println!();
    println!("The keychain is about to be written. If your store is locked, the operating");
    println!("system will ask for its passphrase; type it there. Type it here as well, so");
    println!("this write cannot happen on a terminal nobody is sitting at.");
    let ibara = iqra_ibara("passphrase: ")?;
    if ibara.chars().count() < HADD_IBARA {
        return Err(format!(
            "a keychain passphrase of fewer than {HADD_IBARA} characters is not one this tool \
             will act on"
        ));
    }
    let thaniya = iqra_ibara(if mawjud.is_some() {
        "passphrase again, to confirm the replacement: "
    } else {
        "passphrase again: "
    })?;
    if ibara != thaniya {
        return Err("the two passphrases differ".to_owned());
    }
    drop(ibara);
    drop(thaniya);

    let aam = mafatih::wallid(&ism).map_err(|khata| khata.to_string())?;
    let bayt = aam.bayt();
    if bayt == MIFTAH_TATWIR {
        // Astronomically improbable, and a build would refuse the anchor anyway
        // — but the tool is what publishes the anchor, so it refuses first.
        return Err("the generated key is the committed development key".to_owned());
    }

    // Read it back through the same door a signing run uses, so what is written
    // to the anchor is what the keychain will actually hand back.
    let mukhazzan = mafatih::hat(&ism).map_err(|khata| khata.to_string())?;
    if mukhazzan.aam().bayt() != bayt {
        return Err("the keychain did not hand back the key that was stored".to_owned());
    }

    let nassi = hex::encode(bayt);
    std::fs::write(&mirsa, format!("{nassi}\n"))
        .map_err(|khata| format!("{}: {khata}", mirsa.display()))?;

    println!();
    println!("stored under the keychain account {ism:?}; the private half is in the store above");
    println!("and nowhere else. This tool wrote exactly one file, and it holds the public half:");
    println!();
    println!("  {}", mirsa.display());
    println!("  {nassi}");
    println!();
    println!("Build a release client with it:");
    println!();
    println!("  cargo clean -p taarib-khatm");
    println!("  TAARIB_MIFTAH_ISDAR={nassi} \\");
    println!("    cargo build --release --features taarib-khatm/isdar");
    println!();
    // The clean is not decoration. `option_env!` is not an input cargo tracks,
    // so a target directory that already holds this crate compiled against a
    // previous anchor is reused in silence, and the binary that comes out
    // trusts a key the owner has just retired.
    println!("The clean is not optional in a target directory that has built this crate");
    println!("before: cargo does not track the variable, so it would reuse the object file");
    println!("compiled against the previous anchor and say nothing.");
    println!();
    println!("Back the private half up by minting nothing: there is no export. If this");
    println!("machine's keychain is lost, the anchor is retired and a new one is published.");
    Ok(())
}

// ---------------------------------------------------------------------------
// mirsa — read the anchor back
// ---------------------------------------------------------------------------

/// Prints the public anchor of the stored key.
fn mirsa(hujaj: &[String]) -> Result<(), String> {
    let ism = ism_hisab(hujaj);
    let khass = mafatih::hat(&ism).map_err(|khata| khata.to_string())?;
    let bayt = khass.aam().bayt();
    println!("{}", hex::encode(bayt));
    if bayt == MIFTAH_TATWIR {
        eprintln!(
            "warning: that is the committed development key, whose private half is unprotected \
             by design. A release build refuses it by name."
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// tahaqquq — check the pair still matches
// ---------------------------------------------------------------------------

/// Checks that the stored key still derives the anchor in a file.
fn tahaqquq(hujaj: &[String]) -> Result<(), String> {
    let ism = ism_hisab(hujaj);
    let mirsa = masar_mirsa(hujaj)?;
    let nassi =
        std::fs::read_to_string(&mirsa).map_err(|khata| format!("{}: {khata}", mirsa.display()))?;
    let muallan = min_sittashari(nassi.trim())?;
    let khass = mafatih::hat(&ism).map_err(|khata| khata.to_string())?;
    let mahsub = khass.aam().bayt();
    if muallan == mahsub {
        println!(
            "the key stored under {ism:?} derives the anchor in {}",
            mirsa.display()
        );
        println!("{}", hex::encode(mahsub));
        Ok(())
    } else {
        Err(format!(
            "the anchor in {} is {}, but the stored key derives {}",
            mirsa.display(),
            hex::encode(muallan),
            hex::encode(mahsub)
        ))
    }
}

// ---------------------------------------------------------------------------
// The pieces
// ---------------------------------------------------------------------------

/// Writes, reads back and deletes a probe entry, so an unreachable or locked
/// store fails here with a sentence rather than later with a keychain error.
fn jass_khazna() -> Result<(), String> {
    let jass = MiftahKhass::min_bayt(&basmat_jass());
    mafatih::khzin(ISM_JASS, &jass).map_err(|khata| khata.to_string())?;
    let radd = mafatih::hat(ISM_JASS).map_err(|khata| khata.to_string())?;
    let mutabiq = radd.bayt() == basmat_jass();
    mafatih::imsah(ISM_JASS).map_err(|khata| khata.to_string())?;
    if mutabiq {
        Ok(())
    } else {
        Err("the keychain did not hand back the probe it was given".to_owned())
    }
}

/// The probe's 32 bytes: a hash of a constant, so the value is not a key
/// anybody could mistake for one and is the same on every run.
fn basmat_jass() -> [u8; 32] {
    *blake3::hash(QEEMAT_JASS.as_bytes()).as_bytes()
}

/// Which credential store the platform will answer with.
const fn wasf_khazna() -> &'static str {
    if cfg!(target_os = "windows") {
        "the Windows Credential Manager, protected by this user account's DPAPI material"
    } else if cfg!(target_os = "macos") {
        "the macOS login keychain, protected by its own passphrase"
    } else {
        "the freedesktop Secret Service (GNOME Keyring, KWallet or equivalent), protected by \
         the passphrase on the collection it stores into — which is the login passphrase on a \
         default install, and nothing at all on a store that was created unlocked"
    }
}

/// The keychain account to use: the owner key's own name unless one is named.
fn ism_hisab(hujaj: &[String]) -> String {
    qeemat_hujja(hujaj, "--ism").unwrap_or_else(|| ISM_MIFTAH_MALIK.to_owned())
}

/// The anchor file, which every subcommand but `mirsa` requires.
fn masar_mirsa(hujaj: &[String]) -> Result<PathBuf, String> {
    let masar = qeemat_hujja(hujaj, "--mirsa")
        .ok_or("--mirsa <file> names where the public anchor is written")?;
    let masar = PathBuf::from(masar);
    hurr_min_sirr(&masar)?;
    Ok(masar)
}

/// Refuses an anchor path that reads as a private-key destination.
///
/// The tool cannot write a private key — nothing in it opens a file for one —
/// but an operator who believes it can is an operator who will look for the
/// seed in that file and, not finding it, look somewhere worse. Naming the
/// refusal is cheaper than the confusion.
fn hurr_min_sirr(masar: &Path) -> Result<(), String> {
    let ism = masar
        .file_name()
        .and_then(|ism| ism.to_str())
        .unwrap_or_default()
        .to_lowercase();
    for kalima in ASMAA_SIRR {
        if ism.contains(kalima) {
            return Err(format!(
                "{} names a private-key file, and this tool writes only the public anchor. \
                 Name it for what it holds.",
                masar.display()
            ));
        }
    }
    Ok(())
}

/// The value that follows a flag, in either `--flag value` or `--flag=value`.
fn qeemat_hujja(hujaj: &[String], alam: &str) -> Option<String> {
    let mut baqi = hujaj.iter();
    while let Some(hujja) = baqi.next() {
        if let Some(qeema) = hujja
            .strip_prefix(alam)
            .and_then(|baqi| baqi.strip_prefix('='))
        {
            return Some(qeema.to_owned());
        }
        if hujja == alam {
            return baqi.next().cloned();
        }
    }
    None
}

/// Reads one line from the terminal.
///
/// Echoed, and deliberately not hidden: hiding it would need a terminal crate
/// this crate does not depend on, and the value is not stored, compared against
/// anything on disk, or sent anywhere — it is a second pair of hands on the
/// keyboard, which is what it is described as being.
fn iqra_ibara(mutalaba: &str) -> Result<String, String> {
    print!("{mutalaba}");
    std::io::stdout()
        .flush()
        .map_err(|khata| khata.to_string())?;
    let mut satr = String::new();
    let adad = std::io::stdin()
        .lock()
        .read_line(&mut satr)
        .map_err(|khata| format!("the terminal could not be read: {khata}"))?;
    if adad == 0 {
        return Err("nothing was typed".to_owned());
    }
    Ok(satr.trim_end_matches(['\r', '\n']).to_owned())
}

/// Parses a 64-hex anchor.
fn min_sittashari(nassi: &str) -> Result<[u8; 32], String> {
    if nassi.len() != 64 || !nassi.bytes().all(|bayt| bayt.is_ascii_hexdigit()) {
        return Err("an anchor is exactly 64 hexadecimal characters".to_owned());
    }
    let mut bayt = [0_u8; 32];
    hex::decode_to_slice(nassi, &mut bayt)
        .map_err(|khata| format!("the anchor did not decode: {khata}"))?;
    // Round-tripped through the key type, so a 64-hex string that is not a
    // point on the curve is refused here rather than at the next release build.
    MiftahAam::min_bayt(&bayt).map_err(|khata| khata.to_string())?;
    Ok(bayt)
}
