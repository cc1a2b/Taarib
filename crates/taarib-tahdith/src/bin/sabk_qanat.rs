//! سبك القناة — cast the update channel from built packages.
//!
//! The command line around [`taarib_tahdith::sabk`]: it reads the arguments,
//! opens the signing key in this machine's keychain, and prints what was
//! written. Everything the manifest is made of lives in the library module, so
//! there is no second spelling of it to drift.
//!
//! ```text
//! sabk_qanat --jidhr <repo> --qanat mustaqirr --isdar 1.3.0
//!            --asas 'https://github.com/cc1a2b/taarib/releases/download/v1.3.0/{ism}'
//!            --huzma dist/taarib-1.3.0-x64.exe --hadaf x86_64-pc-windows-msvc
//! ```

// The print ban exists so that no library writes to a terminal the application
// owns. This is not a library: it is a maintainer's one-command tool, run by
// hand when a release is published, whose entire interface is a few lines on
// stdout and a refusal on stderr.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]

use std::path::PathBuf;

use taarib_tahdith::bayan::{KatibBayan, QANAT_MUSTAQIRR, QANAT_TAJRIBI, QANAWAT};
use taarib_tahdith::sabk::{
    Huzma, ISM_BAYAN, ISM_TAWQEE, KhiyaratSabk, adna_iftiradi, iqra_sabiq, madkhal_min_huzma, uktub,
};

/// One `--huzma` and the facts that cannot be read out of the file.
#[derive(Debug)]
struct TalabMadkhal {
    huzma: PathBuf,
    hadaf: String,
    rabt: Option<String>,
}

/// What the command line asked for.
#[derive(Debug)]
struct Khiyarat {
    jidhr: PathBuf,
    qanat: String,
    isdar: String,
    adna: String,
    asas: Option<String>,
    ism_miftah: Option<String>,
    talabat: Vec<TalabMadkhal>,
}

const ISTIMAL: &str = "\
سبك القناة — cast the update channel from built packages

  sabk_qanat --jidhr <repo> --qanat <mustaqirr|tajribi> --isdar <x.y.z>
             [--adna <x.y.z>] [--asas <https://base/{ism}>]
             [--ism-miftah <keychain account>]
             [--huzma <file> --hadaf <target-triple> [--rabt <https://…>]]...

  --jidhr       the registry working tree to write tahdith.json into (required)
  --qanat       the channel being cast. Casting a channel replaces every entry
                on it and touches no other channel, so publishing a pre-release
                never withdraws the released one
  --isdar       the version these packages are, three dot-separated numbers.
                Required with a --huzma and meaningless without one
  --adna        the lowest version this release may update in place, defaulting
                to --isdar's own major.0.0. A client below it is told to install
                by hand rather than offered a package that cannot take it
  --asas        the release-asset base every entry's address is built from. A
                base holding {ism} gets the package's file name, which is the
                only form a forge's flat release area answers; one holding
                neither has the file name appended
  --ism-miftah  the keychain account holding the signing key
  --huzma       a built package to publish; repeatable, and each one must be
                followed by its --hadaf
  --hadaf       the target triple the preceding --huzma runs on, as the tables
                in docs/tawzee.md §2 name it
  --rabt        the full download address for the preceding --huzma, when it is
                not under --asas

The size and the SHA-256 a client verifies the download against are computed
from the package file here and are never typed in. The manifest already in the
repository is verified against this same key before it is merged into, and a
manifest it did not sign is a refusal. Giving no --huzma at all withdraws the
channel: its entries are dropped and the manifest is re-signed without them.
";

fn main() -> std::process::ExitCode {
    if let Err(khata) = nafidh() {
        eprintln!("سبك القناة: {khata}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// The whole run, so that every failure leaves through one place.
fn nafidh() -> Result<(), String> {
    let Some(khiyarat) = iqra_khiyarat().map_err(|khata| format!("{khata}\n\n{ISTIMAL}"))? else {
        return Ok(());
    };

    // The signing key never leaves the keychain; only its public half is ever
    // written into the repository, inside the detached signature.
    let khass = match khiyarat.ism_miftah.as_deref() {
        Some(ism) => taarib_khatm::mafatih::hat(ism),
        None => taarib_khatm::malik::hat_malik(),
    }
    .map_err(|khata| format!("no signing key in this machine's keychain: {khata}"))?;

    let sabiq = iqra_sabiq(&khiyarat.jidhr, &khass.aam())?;
    let mut katib = match sabiq.as_ref() {
        Some(bayan) => KatibBayan::min_bayan(bayan),
        None => KatibBayan::jadeed(),
    }
    .bila_qanat(&khiyarat.qanat);

    println!("  channel   {}", khiyarat.qanat);
    if khiyarat.talabat.is_empty() {
        println!("    withdrawn; this run publishes no package");
    } else {
        println!("    version   {}", khiyarat.isdar);
        println!("    floor     {}", khiyarat.adna);
    }
    println!("    carried   {} entry(ies) from other channels", {
        katib.madakhil().len()
    });

    let mushtarak = KhiyaratSabk {
        qanat: &khiyarat.qanat,
        isdar: &khiyarat.isdar,
        adna: &khiyarat.adna,
        asas: khiyarat.asas.as_deref(),
    };
    for talab in &khiyarat.talabat {
        let madkhal = madkhal_min_huzma(
            &Huzma {
                masar: &talab.huzma,
                hadaf: &talab.hadaf,
                rabt: talab.rabt.as_deref(),
            },
            &mushtarak,
        )?;
        println!("  read {}", talab.huzma.display());
        println!("    target    {}", madkhal.hadaf);
        println!("    size      {} byte(s)", madkhal.hajm);
        println!("    sha256    {}", madkhal.sha256);
        println!("    from      {}", madkhal.rabt);
        katib = katib.adif(madkhal);
    }

    let (matn, tawqee) = katib
        .uktub(&khass)
        .map_err(|khata| format!("the manifest would not cast: {khata}"))?;
    uktub(&khiyarat.jidhr, &matn, &tawqee)?;

    println!(
        "  wrote {} ({} byte(s), {} entry(ies))",
        khiyarat.jidhr.join(ISM_BAYAN).display(),
        matn.len(),
        katib.madakhil().len()
    );
    println!("  wrote {}", khiyarat.jidhr.join(ISM_TAWQEE).display());
    println!("    key       {}", hex::encode(khass.aam().bayt()));
    Ok(())
}

/// Reads the command line, or explains why it will not.
fn iqra_khiyarat() -> Result<Option<Khiyarat>, String> {
    let mut hujaj = std::env::args().skip(1);
    let (mut jidhr, mut qanat, mut isdar, mut adna, mut asas, mut ism_miftah) =
        (None, None, None, None, None, None);
    let mut talabat: Vec<TalabMadkhal> = Vec::new();
    let baad = |hujaj: &mut std::iter::Skip<std::env::Args>, wasm: &str| {
        hujaj.next().ok_or_else(|| format!("{wasm} needs a value"))
    };

    while let Some(hujja) = hujaj.next() {
        match hujja.as_str() {
            "--jidhr" => jidhr = Some(PathBuf::from(baad(&mut hujaj, "--jidhr")?)),
            "--qanat" => qanat = Some(baad(&mut hujaj, "--qanat")?),
            "--isdar" => isdar = Some(baad(&mut hujaj, "--isdar")?),
            "--adna" => adna = Some(baad(&mut hujaj, "--adna")?),
            "--asas" => asas = Some(baad(&mut hujaj, "--asas")?),
            "--ism-miftah" => ism_miftah = Some(baad(&mut hujaj, "--ism-miftah")?),
            "--huzma" => talabat.push(TalabMadkhal {
                huzma: PathBuf::from(baad(&mut hujaj, "--huzma")?),
                hadaf: String::new(),
                rabt: None,
            }),
            "--hadaf" => {
                let qeema = baad(&mut hujaj, "--hadaf")?;
                let akhir = talabat
                    .last_mut()
                    .ok_or_else(|| "--hadaf comes after the --huzma it belongs to".to_owned())?;
                akhir.hadaf = qeema;
            },
            "--rabt" => {
                let qeema = baad(&mut hujaj, "--rabt")?;
                let akhir = talabat
                    .last_mut()
                    .ok_or_else(|| "--rabt comes after the --huzma it belongs to".to_owned())?;
                akhir.rabt = Some(qeema);
            },
            "--help" | "-h" => {
                println!("{ISTIMAL}");
                return Ok(None);
            },
            akhar => return Err(format!("unknown argument {akhar:?}")),
        }
    }

    let jidhr = jidhr.ok_or_else(|| "--jidhr is required".to_owned())?;
    let qanat = qanat.ok_or_else(|| "--qanat is required".to_owned())?;
    if !QANAWAT.contains(&qanat.as_str()) {
        return Err(format!(
            "--qanat {qanat:?} is neither {QANAT_MUSTAQIRR} nor {QANAT_TAJRIBI}"
        ));
    }
    // A withdrawal publishes no package, so the version and the floor it would
    // have carried are facts about nothing; asking for them would be asking the
    // operator to invent one.
    let isdar = match isdar {
        Some(isdar) => isdar,
        None if talabat.is_empty() => String::new(),
        None => return Err("--isdar is required".to_owned()),
    };
    let adna = match adna {
        Some(adna) => adna,
        None if talabat.is_empty() => String::new(),
        None => adna_iftiradi(&isdar)
            .ok_or_else(|| format!("--isdar {isdar:?} has no leading version number"))?,
    };
    for talab in &talabat {
        if talab.hadaf.is_empty() {
            return Err(format!("{} was given no --hadaf", talab.huzma.display()));
        }
    }

    Ok(Some(Khiyarat {
        jidhr,
        qanat,
        isdar,
        adna,
        asas,
        ism_miftah,
        talabat,
    }))
}
