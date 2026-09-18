//! # تجميع تعريب — the staging tool
//!
//! Gathers every already-built artifact of docs/tawzee.md §3 into the bundle's
//! resource tree, hashes each one, and writes `bayan_mukawwinat.json` last.
//!
//! It builds nothing. Cargo, dotnet, wasm-bindgen and esbuild run before it and
//! leave their outputs where the matrix says; this tool's whole contribution is
//! that a missing one is named here, on a build machine, instead of discovered
//! by a user whose install silently did nothing.
//!
//! Every absence is collected and printed together, and the exit status is
//! non-zero: an operator who has four things to rebuild learns that once.

// The print ban exists so that no library writes to a terminal the application
// owns. This is not a library: it is a one-command build-machine tool whose
// entire interface is the summary it prints and the refusals it names.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]
// Nothing in a binary crate is reachable from outside it, so every item shared
// between these modules has to be `pub(crate)`: `pub` is what `unreachable_pub`
// refuses here, and `pub(crate)` is what this lint calls redundant.
#![expect(
    clippy::redundant_pub_crate,
    reason = "the workspace denies `unreachable_pub`, which in a binary crate demands \
              exactly the `pub(crate)` this lint objects to"
)]

mod hadaf;
mod khata;
mod masfufa;
mod nasakh;
mod qufl;

use std::path::PathBuf;
use std::process::ExitCode;

use crate::khata::KhataTajmee;
use crate::masfufa::MasadirTajmee;
use crate::nasakh::{Mustaqarr, Taqm};

/// The workspace version this tool stamps into the manifest.
const ISDAR: &str = env!("CARGO_PKG_VERSION");

/// What the command line asked for.
struct Khiyarat {
    hadaf: String,
    jidhr: PathBuf,
    ahdaf: PathBuf,
    kharij: PathBuf,
    tawzee: Option<PathBuf>,
    taqm: Taqm,
    jalb: bool,
}

fn main() -> ExitCode {
    let khiyarat = match iqra_khiyarat() {
        Ok(Some(khiyarat)) => khiyarat,
        Ok(None) => return ExitCode::SUCCESS,
        Err(satr) => {
            eprintln!("{satr}");
            return ExitCode::FAILURE;
        },
    };

    match nafidh(&khiyarat) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(khata) => {
            eprintln!("{}", khata.satr());
            ExitCode::FAILURE
        },
    }
}

/// Runs one staging pass, returning how many artifacts were refused.
fn nafidh(khiyarat: &Khiyarat) -> Result<usize, KhataTajmee> {
    let hadaf = hadaf::min_muthallath(&khiyarat.hadaf)?;
    let masadir = MasadirTajmee {
        jidhr: khiyarat.jidhr.clone(),
        ahdaf: khiyarat.ahdaf.clone(),
        jalb: khiyarat.jalb,
    };

    let mut mustaqarr =
        Mustaqarr::iftah(&khiyarat.kharij, khiyarat.taqm, khiyarat.tawzee.as_deref())?;
    masfufa::jammi(hadaf, &masadir, &mut mustaqarr)?;

    let naqis = mustaqarr.naqis().len();
    if naqis > 0 {
        for khata in mustaqarr.naqis() {
            eprintln!("{}", khata.satr());
        }
        eprintln!("\ntajmee: {naqis} artifact(s) missing; neither document was written");
        return Ok(naqis);
    }

    let hisab = mustaqarr.hisab();
    let _ = mustaqarr.akhtim(ISDAR, hadaf.muthallath)?;
    println!(
        "tajmee: {} file(s), {} byte(s) staged for {} in {} [{}]",
        hisab.fi_alhuzma,
        hisab.hajm_alhuzma,
        hadaf.muthallath,
        khiyarat.kharij.display(),
        khiyarat.taqm.ism()
    );
    println!(
        "        catalogue: {} component(s), {} left out of this bundle, {} byte(s) to fetch",
        hisab.mukawwinat, hisab.kharij, hisab.hajm_kharij
    );
    if let Some(tawzee) = khiyarat.tawzee.as_ref() {
        println!("        objects:   {}", tawzee.display());
    }
    Ok(0)
}

/// Parses the command line. `Ok(None)` means help was printed.
fn iqra_khiyarat() -> Result<Option<Khiyarat>, String> {
    let mut hadaf: Option<String> = None;
    let mut jidhr = PathBuf::from(".");
    let mut ahdaf: Option<PathBuf> = None;
    let mut kharij: Option<PathBuf> = None;
    let mut tawzee: Option<PathBuf> = None;
    let mut taqm = Taqm::Kamil;
    let mut jalb = false;

    let mut wusata = std::env::args().skip(1);
    while let Some(wasita) = wusata.next() {
        match wasita.as_str() {
            "--hadaf" => hadaf = wusata.next(),
            "--jidhr" => jidhr = wusata.next().map(PathBuf::from).unwrap_or(jidhr),
            "--ahdaf" => ahdaf = wusata.next().map(PathBuf::from),
            "--kharij" => kharij = wusata.next().map(PathBuf::from),
            "--tawzee" => tawzee = wusata.next().map(PathBuf::from),
            "--taqm" => {
                let ism = wusata.next().unwrap_or_default();
                taqm = Taqm::min_ism(&ism).ok_or_else(|| {
                    format!("tajmee: --taqm takes kamil or nahif, not {ism}\n\n{MUSAADA}")
                })?;
            },
            "--jalb" => jalb = true,
            "-h" | "--help" => {
                println!("{MUSAADA}");
                return Ok(None);
            },
            majhul => return Err(format!("tajmee: unknown argument {majhul}")),
        }
    }

    let Some(hadaf) = hadaf else {
        return Err(format!("tajmee: --hadaf is required\n\n{MUSAADA}"));
    };
    let ahdaf = ahdaf.unwrap_or_else(|| jidhr.join("target"));
    let kharij = kharij.unwrap_or_else(|| jidhr.join("apps/studio/src-tauri/mawarid"));
    Ok(Some(Khiyarat {
        hadaf,
        jidhr,
        ahdaf,
        kharij,
        tawzee,
        taqm,
        jalb,
    }))
}

/// What `--help` prints, prerequisites included.
const MUSAADA: &str = "\
تجميع تعريب — stage the bundle's resource tree

    taarib-tajmee --hadaf <target-triple> [--jidhr <workspace>] [--ahdaf <target dir>]
                  [--kharij <out dir>] [--taqm kamil|nahif] [--tawzee <dir>] [--jalb]

    --hadaf    one of the targets in docs/tawzee.md §2 (required)
    --jidhr    workspace root (default: .)
    --ahdaf    cargo target directory (default: <jidhr>/target)
    --kharij   staging root (default: <jidhr>/apps/studio/src-tauri/mawarid)
    --taqm     which components the bundle carries (default: kamil)
    --tawzee   write the release's content-addressed component objects here
    --jalb     permit fetching locked artifacts that are not cached yet

Every run reads, hashes and refuses-if-absent every artifact of the matrix,
whichever set is asked for. --taqm decides only what the bundle carries:

    kamil   every component — 519,747,711 byte(s) of component tree on
            x86_64-pc-windows-msvc. This is the offline bundle, and it is what
            every build before the flag existed produced.
    nahif   everything but the six IL2CPP BepInEx components, which are
            449,711,337 of those bytes. A machine with no IL2CPP Unity game
            needs none of it; one that has such a game fetches the component it
            names, against the hashes in fihris_mukawwinat.json, which
            bayan_mukawwinat.json vouches for and the installer's signature
            covers.

--tawzee writes every component's files as objects named by their own sha256,
so the three byte-identical Unity generations of one backend and architecture
are stored once. One object store serves both variants and every platform.

This tool builds nothing, and on a clean checkout that means it stages nothing:
fourteen files across rows B, E, F, G, H and I have to be built first. The one
command that builds them in order and then runs this tool is

    scripts/isdar.sh --hadaf <target-triple> --jalb

and .github/workflows/isdar.yml is the same sequence on a runner. What it runs,
for the record, so that a single row can be rebuilt by hand:

    # rows C1-C4 — the BepInEx-side assemblies
    dotnet build unity/Taarib.Unity.sln -c Release

    # rows B, E, F, G — the game-side cdylibs, for EVERY triple in
    # hadaf::hamulat_alalaab: both Windows ones always, plus the host's own.
    # The feature is package-qualified because taarib-jisr and taarib-mudkhal
    # do not have one, and it is not optional: a payload built without it
    # exports no taarib_bidaya and is refused below by name.
    cargo build --release --target <payload-triple> \\
        -p taarib-jisr -p taarib-mudkhal -p taarib-tabaqa \\
        -p taarib-muhawwil-unreal -p taarib-muhawwil-godot \\
        --features taarib-tabaqa/hamula,taarib-muhawwil-unreal/hamula,\\
taarib-muhawwil-godot/hamula

    # row H1 — the CLI version must equal the wasm-bindgen crate in Cargo.lock
    cargo build --release --target wasm32-unknown-unknown -p taarib-wasm
    wasm-bindgen --target no-modules --out-name taarib_core \\
        --out-dir target/wasm-bindgen <the built .wasm>

    # rows I1, I2 — tsc for the RPG Maker plugin, which has to be ES5 and which
    # esbuild cannot lower to it; esbuild for the Electron runtime, which has
    # to load both as a CommonJS module and as a bare <script>
    node adapters-script/ibni.mjs

Rows H1, I1 and I2 are read from <jidhr>/target regardless of --ahdaf: none of
the three is a cargo output, and the cargo target directory is not theirs.
";
