//! سبك المستودع — the maintainer's one command over `taarib_mustawda::sabk`.
//!
//! ```text
//! sabk --jidhr <repo> --tasalsul <n> --asas <https://...>
//!      [--mira <https://...>] [--huzma <file> --luba <uuid> --ism <title>]...
//! sabk --jidhr <repo> --mujtama <body.json>
//! ```
//!
//! Run it after every publication, with the sequence number incremented: a
//! client caches on that number and will not look again until it moves.
//!
//! The community index is cast the same way and by the same key. It is a
//! separate run because it changes on its own schedule — a team publishes on
//! their own page, not into this catalogue — and because it publishes no
//! package and needs no sequence number. Every other document this tool signs
//! decides what a client installs; that one decides which addresses a client
//! will open, which is why it is signed at all.

// The print ban exists so that no library writes to a terminal the application
// owns. This is not a library: it is a maintainer's one-command tool, run by
// hand when a catalogue is published, whose entire interface is a few lines on
// stdout and a refusal on stderr.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone CLI's output is its interface, not a library writing to a \
              terminal it does not own"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use taarib_khatm::MiftahKhass;
use taarib_mustalahat::khariji::RuqaaKharijiya;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::ruqaa::RuqaaId;
use taarib_mustawda::fahras::shareeha;
use taarib_mustawda::khariji::badhrat_kharijiya;
use taarib_mustawda::mujtama::{KatibFahrasMujtama, MASAR_FAHRAS_MUJTAMA};
use taarib_mustawda::sabk::{
    KhiyaratSabk, MadkhalKhariji, Mulghayat, ijri, madkhal_khariji, madkhal_min_huzma, min_unix,
    qaimat_sahb, uktub,
};

/// Casts the community index: the maintainer's body in, the signed document a
/// client verifies out.
///
/// Through `KatibFahrasMujtama`, which shares its canonical form with the
/// verifier for the same reason `KatibQaima` does: a second spelling of the
/// signed bytes is a second place for them to drift, and the drift produces
/// documents that verify against nothing. The body is validated by the reader's
/// own rules before it is signed, so an entry a client would refuse is refused
/// here, on the owner's machine, and never reaches one.
fn fahras_mujtama(masdar: &Path, jidhr: &Path, khass: &MiftahKhass) -> Result<(), String> {
    let bayt = fs::read(masdar).map_err(|khata| format!("{}: {khata}", masdar.display()))?;
    let waqt = Timestamp::from_second(unix_alan())
        .map_err(|khata| format!("the clock reads outside the calendar: {khata}"))?
        .to_string();
    let katib = KatibFahrasMujtama::min_bayt(&bayt)
        .and_then(|katib| katib.bi_waqt(&waqt))
        .map_err(|khata| khata.to_string())?;
    let wathiqa = katib.uktub(khass).map_err(|khata| khata.to_string())?;
    uktub(jidhr, MASAR_FAHRAS_MUJTAMA, &wathiqa)?;

    println!("  read {}", masdar.display());
    println!("    teams     {}", katib.adad_firaq());
    println!("    entries   {}", katib.adad_tarjamat());
    println!("    cast      {}", katib.waqt());
    println!("    key       {}", hex::encode(khass.aam().bayt()));
    println!(
        "  wrote {} ({} byte(s))",
        jidhr.join(MASAR_FAHRAS_MUJTAMA).display(),
        wathiqa.len()
    );
    Ok(())
}

/// Writes the third-party entries this build knows about, one file each.
///
/// The seeds are compiled in because they are measurements — an artifact's size
/// and digest, taken once from bytes that are gone — and what a maintainer has
/// to add is the one thing no measurement produces: the author's own permission
/// to publish their work. So the file that comes out of here is deliberately a
/// file that will not cast, and the refusal it earns names the field to fill in.
fn uktub_badhrat_kharijiya(wijha: &Path) -> Result<(), String> {
    fs::create_dir_all(wijha).map_err(|khata| format!("{}: {khata}", wijha.display()))?;
    for badhra in badhrat_kharijiya() {
        let masar = wijha.join(format!("{}.json", badhra.id));
        let bayt = serde_json::to_vec_pretty(&badhra).map_err(|khata| khata.to_string())?;
        fs::write(&masar, &bayt).map_err(|khata| format!("{}: {khata}", masar.display()))?;
        println!("  wrote {} ({} byte(s))", masar.display(), bayt.len());
        println!("    entry     {} {}", badhra.id, badhra.unwan);
        println!("    author    {}", badhra.nasab());
        match badhra.sabab_rafd() {
            Some(sabab) => println!("    TO FILL   {}", sabab.wasf_injilizi()),
            None => println!("    ready     nothing stops this entry"),
        }
    }
    Ok(())
}

/// Reads one `--khariji` file into a listing the cast will take.
fn iqra_khariji(masar: &Path) -> Result<MadkhalKhariji, String> {
    let bayt = fs::read(masar).map_err(|khata| format!("{}: {khata}", masar.display()))?;
    let madkhal: RuqaaKharijiya =
        serde_json::from_slice(&bayt).map_err(|khata| format!("{}: {khata}", masar.display()))?;
    madkhal_khariji(madkhal).map_err(|khata| format!("{}: {khata}", masar.display()))
}

/// One `--huzma` and the facts that cannot be read out of it.
struct TalabNashr {
    huzma: PathBuf,
    luba: LubaId,
    ism: String,
    tajawuz: Option<String>,
}

/// What the command line asked for.
struct Khiyarat {
    jidhr: Option<PathBuf>,
    /// The manifest sequence, required by every mode that writes a manifest and
    /// meaningless to the two that do not.
    tasalsul: Option<u64>,
    asas: Option<String>,
    mira: Option<String>,
    talabat: Vec<TalabNashr>,
    /// Third-party entries to list, each read from a file a maintainer edited.
    kharijiya: Vec<PathBuf>,
    ism_miftah: Option<String>,
    mulghayat: Mulghayat,
    /// Where to write the compiled-in revocation seed, when that is all this
    /// run is for.
    badhra: Option<PathBuf>,
    /// Where to write the compiled-in third-party seeds, when that is all this
    /// run is for.
    badhra_kharijiya: Option<PathBuf>,
    /// The community index body to sign into the repository, when that is all
    /// this run is for.
    mujtama: Option<PathBuf>,
}

const ISTIMAL: &str = "\
سبك — cast the registry from sealed packages

  sabk --jidhr <repo> --tasalsul <n> --asas <https://base/>
       [--mira <https://mirror/>] [--ism-miftah <keychain account>]
       [--huzma <file.ruqaa> --luba <game-uuid> --ism <title> [--tajawuz <why>]]...
       [--khariji <entry.json>]...
       [--mulgha <64-hex key> --sabab <why>]...
       [--mulgha-ruqaa <patch-uuid> --sabab <why>]...

  sabk --badhra assets/qaimat_sahb.json --tasalsul <n> [--ism-miftah <account>]

  sabk --badhra-kharijiya <dir>

  sabk --jidhr <repo> --mujtama <body.json> [--ism-miftah <account>]

  --jidhr       the repository working tree to write into (required)
  --tasalsul    the manifest sequence number; a client caches on it and will
                not look again until it moves, so increment it every time
  --asas        the base the repository path is resolved against. A base holding
                {ism} gets the asset's file name, which is the only form a
                forge's flat release area answers; one holding {masar} gets its
                repository path; one holding neither has the whole repository
                path — isdar/<id>/<name> — appended, which is what a directory
                server and a forge's raw-content root want, so give it the
                repository root and not the isdar directory
  --mira        an optional second address for the same assets, same spelling
  --ism-miftah  the keychain account holding the signing key
  --huzma       a sealed package to publish; repeatable, and each one must be
                followed by its --luba and --ism
  --tajawuz     publish the preceding --huzma even though its own coverage gate
                refuses it, giving the reason; the reason is written into the
                manifest, where every reader of the catalogue sees it
  --khariji     a third-party entry to list: a patch somebody else made, which
                Taarib fetches from its author, verifies against the pins in the
                file and installs through the same backup machinery, and never
                built. Repeatable. Nothing in the file is compiled and nothing
                is signed by its author, so the entry is refused unless it
                records written permission from whoever made the work — start
                from --badhra-kharijiya and fill the statement in
  --badhra-kharijiya
                write the third-party entries this build knows about into this
                directory, one JSON file each, and stop. Each ships with an
                empty permission statement, which is the one fact nobody here
                can supply: ask the author, write where they answered and when,
                and feed the file back through --khariji. Nothing else is
                written and no repository is touched.
  --mulgha      a signing key to revoke, as 64 lowercase hex; repeatable, and
                each one must be followed by its --sabab
  --mulgha-ruqaa
                a patch lineage to revoke, as its uuid; repeatable, and each one
                must be followed by its --sabab. The list is rebuilt whole on
                every cast, so every revocation still in force has to be named
                on every run — one left out stops being revoked
  --badhra      write only the compiled-in revocation seed to this path and
                stop. The seed is signed by the same key as the served list, so
                a build anchored to the release key needs one signed by it: a
                seed the anchor cannot verify stops the safety layer at startup,
                before a single game is scanned. Nothing else is written and no
                repository is touched.
  --mujtama     sign the community translations index and write it to
                fahras/tarjamat.json under --jidhr, then stop. The body is the
                hand-maintained file; every check a client runs is run here
                first, the revision is restamped with the moment of the cast,
                and any signature the body already carries is replaced. Fields
                this build does not read are carried through untouched. A client
                reads no entry, credits no maker and opens no address out of an
                index this key did not sign, so an index cast without this is an
                index nobody sees. Nothing else is written.

Every listing field except the game's identity and title is read out of the
package's own sealed metadata. A package with no signature is refused, and so is
one whose metadata says qabila_lil_nashr: false unless --tajawuz says why.
";

/// Reads the command line, or explains why it will not.
fn iqra_khiyarat() -> Result<Option<Khiyarat>, String> {
    let mut hujaj = std::env::args().skip(1);
    let (mut jidhr, mut tasalsul, mut asas, mut mira, mut ism_miftah) =
        (None, None, None, None, None);
    let mut badhra: Option<PathBuf> = None;
    let mut badhra_kharijiya: Option<PathBuf> = None;
    let mut mujtama: Option<PathBuf> = None;
    let mut talabat: Vec<TalabNashr> = Vec::new();
    let mut kharijiya: Vec<PathBuf> = Vec::new();
    let mut mulghayat = Mulghayat::default();
    // Which revocation the next `--sabab` belongs to: the kinds interleave on
    // the command line and each keeps its own list, so a shared "last one" is
    // the only way `--sabab` can name the thing directly before it.
    let mut akhir_ilgha: Option<bool> = None;
    let baad = |hujaj: &mut std::iter::Skip<std::env::Args>, wasm: &str| {
        hujaj.next().ok_or_else(|| format!("{wasm} needs a value"))
    };
    while let Some(hujja) = hujaj.next() {
        match hujja.as_str() {
            "--jidhr" => jidhr = Some(PathBuf::from(baad(&mut hujaj, "--jidhr")?)),
            "--tasalsul" => {
                let khaam = baad(&mut hujaj, "--tasalsul")?;
                tasalsul = Some(
                    khaam
                        .parse::<u64>()
                        .map_err(|_| format!("--tasalsul takes a whole number, not {khaam:?}"))?,
                );
            },
            "--asas" => asas = Some(baad(&mut hujaj, "--asas")?),
            "--mira" => mira = Some(baad(&mut hujaj, "--mira")?),
            "--ism-miftah" => ism_miftah = Some(baad(&mut hujaj, "--ism-miftah")?),
            "--badhra" => badhra = Some(PathBuf::from(baad(&mut hujaj, "--badhra")?)),
            "--badhra-kharijiya" => {
                badhra_kharijiya = Some(PathBuf::from(baad(&mut hujaj, "--badhra-kharijiya")?));
            },
            "--mujtama" => mujtama = Some(PathBuf::from(baad(&mut hujaj, "--mujtama")?)),
            "--khariji" => kharijiya.push(PathBuf::from(baad(&mut hujaj, "--khariji")?)),
            "--huzma" => {
                talabat.push(TalabNashr {
                    huzma: PathBuf::from(baad(&mut hujaj, "--huzma")?),
                    // Filled by the --luba and --ism that must follow.
                    luba: LubaId::min_uuid(uuid::Uuid::nil()),
                    ism: String::new(),
                    tajawuz: None,
                });
            },
            "--luba" => {
                let khaam = baad(&mut hujaj, "--luba")?;
                let uuid = uuid::Uuid::parse_str(&khaam)
                    .map_err(|_| format!("--luba takes a game uuid, not {khaam:?}"))?;
                talabat
                    .last_mut()
                    .ok_or_else(|| "--luba must follow a --huzma".to_owned())?
                    .luba = LubaId::min_uuid(uuid);
            },
            "--ism" => {
                let ism = baad(&mut hujaj, "--ism")?;
                talabat
                    .last_mut()
                    .ok_or_else(|| "--ism must follow a --huzma".to_owned())?
                    .ism = ism;
            },
            "--tajawuz" => {
                let sabab = baad(&mut hujaj, "--tajawuz")?;
                if sabab.trim().is_empty() {
                    return Err("--tajawuz needs a sentence, not an empty string".to_owned());
                }
                talabat
                    .last_mut()
                    .ok_or_else(|| "--tajawuz must follow a --huzma".to_owned())?
                    .tajawuz = Some(sabab);
            },
            "--mulgha" => {
                let khaam = baad(&mut hujaj, "--mulgha")?;
                // Checked here rather than at signing time so a mistyped key is
                // a refusal on the command line, not a list that verifies and
                // revokes nothing anybody has.
                if khaam.len() != 64 || !khaam.bytes().all(|q| q.is_ascii_hexdigit()) {
                    return Err(format!("--mulgha takes 64 hex characters, not {khaam:?}"));
                }
                mulghayat
                    .mafatih
                    .push((khaam.to_ascii_lowercase(), String::new()));
                akhir_ilgha = Some(true);
            },
            "--mulgha-ruqaa" => {
                let khaam = baad(&mut hujaj, "--mulgha-ruqaa")?;
                let uuid = uuid::Uuid::parse_str(&khaam)
                    .map_err(|_| format!("--mulgha-ruqaa takes a patch uuid, not {khaam:?}"))?;
                mulghayat
                    .ruqa
                    .push((RuqaaId::min_uuid(uuid), String::new()));
                akhir_ilgha = Some(false);
            },
            "--sabab" => {
                let sabab = baad(&mut hujaj, "--sabab")?;
                match akhir_ilgha {
                    Some(true) => {
                        mulghayat
                            .mafatih
                            .last_mut()
                            .ok_or_else(|| "--sabab must follow a --mulgha".to_owned())?
                            .1 = sabab;
                    },
                    Some(false) => {
                        mulghayat
                            .ruqa
                            .last_mut()
                            .ok_or_else(|| "--sabab must follow a --mulgha-ruqaa".to_owned())?
                            .1 = sabab;
                    },
                    None => {
                        return Err("--sabab must follow a --mulgha or a --mulgha-ruqaa".to_owned());
                    },
                }
            },
            // Asking for the usage is neither a failure nor a run. `None`
            // says so, and `main` keeps sole ownership of the exit code.
            "--help" | "-h" => {
                println!("{ISTIMAL}");
                return Ok(None);
            },
            akhar => return Err(format!("unknown argument {akhar:?}")),
        }
    }
    for talab in &talabat {
        if talab.ism.is_empty() {
            return Err(format!("{} was given no --ism", talab.huzma.display()));
        }
    }
    // An unexplained revocation is one nobody can act on, and the reason is
    // covered by the signature, so it is not something to leave blank.
    for (miftah, sabab) in &mulghayat.mafatih {
        if sabab.is_empty() {
            return Err(format!("--mulgha {miftah} was given no --sabab"));
        }
    }
    for (ruqaa, sabab) in &mulghayat.ruqa {
        if sabab.is_empty() {
            return Err(format!("--mulgha-ruqaa {ruqaa} was given no --sabab"));
        }
    }
    // The single-document modes are exclusive, of each other and of a cast:
    // each writes for its own reasons and would say nothing coherent about a
    // repository it also cast.
    let munfarida = [
        badhra.is_some(),
        badhra_kharijiya.is_some(),
        mujtama.is_some(),
    ]
    .iter()
    .filter(|wahid| **wahid)
    .count();
    if munfarida > 1 {
        return Err(
            "--badhra, --badhra-kharijiya and --mujtama each write on their own; run them \
             separately"
                .to_owned(),
        );
    }
    // Seed mode writes one file and reads no repository, so the two arguments
    // that name a repository are required for casting and meaningless here.
    if badhra.is_some() {
        if jidhr.is_some() || asas.is_some() || !talabat.is_empty() {
            return Err(
                "--badhra writes only the seed; it takes neither a repository nor a \
                        package"
                    .to_owned(),
            );
        }
    } else if badhra_kharijiya.is_some() {
        if jidhr.is_some() || asas.is_some() || !talabat.is_empty() || !kharijiya.is_empty() {
            return Err(
                "--badhra-kharijiya writes only the third-party seeds; it takes neither a \
                 repository nor an entry"
                    .to_owned(),
            );
        }
    } else if mujtama.is_some() {
        // The index does name a repository — it is written into one — but it
        // publishes no package and rides no manifest, so everything that
        // belongs to a cast is refused rather than silently ignored.
        if asas.is_some()
            || !talabat.is_empty()
            || !kharijiya.is_empty()
            || mulghayat.adad() != 0
            || tasalsul.is_some()
        {
            return Err(
                "--mujtama writes only the community index; it takes neither a package, a \
                 revocation nor a sequence number"
                    .to_owned(),
            );
        }
        if jidhr.is_none() {
            return Err("--jidhr is required".to_owned());
        }
    } else if jidhr.is_none() || asas.is_none() {
        return Err(if jidhr.is_none() {
            "--jidhr is required".to_owned()
        } else {
            "--asas is required".to_owned()
        });
    }
    if mujtama.is_none() && badhra_kharijiya.is_none() && tasalsul.is_none() {
        return Err("--tasalsul is required".to_owned());
    }
    Ok(Some(Khiyarat {
        jidhr,
        tasalsul,
        asas,
        mira,
        talabat,
        kharijiya,
        ism_miftah,
        mulghayat,
        badhra,
        badhra_kharijiya,
        mujtama,
    }))
}

/// The current time as whole seconds since the epoch.
///
/// The manifest records when it was published, and the client shows it. A clock
/// that cannot be read is a reason to refuse rather than to publish a lie about
/// when this catalogue was cast.
fn unix_alan() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(0)
}

fn main() -> std::process::ExitCode {
    if let Err(khata) = nafidh() {
        eprintln!("سبك: {khata}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// The whole run, so that every failure leaves through one place.
fn nafidh() -> Result<(), String> {
    let Some(khiyarat) = iqra_khiyarat().map_err(|khata| format!("{khata}\n\n{ISTIMAL}"))? else {
        return Ok(());
    };

    // Before the keychain is touched: a seed is public data about somebody
    // else's work and nothing here signs it.
    if let Some(wijha) = khiyarat.badhra_kharijiya.as_deref() {
        return uktub_badhrat_kharijiya(wijha);
    }

    // The signing key never leaves the keychain; only its public half is ever
    // written into the repository, inside the revocation list's signature.
    let khass = match khiyarat.ism_miftah.as_deref() {
        Some(ism) => taarib_khatm::mafatih::hat(ism),
        None => taarib_khatm::malik::hat_malik(),
    }
    .map_err(|khata| format!("no signing key in this machine's keychain: {khata}"))?;

    if let Some(wijha) = khiyarat.badhra.as_deref() {
        let tasalsul = khiyarat
            .tasalsul
            .ok_or_else(|| "--tasalsul is required".to_owned())?;
        let waqt = Timestamp::from_second(unix_alan())
            .map_err(|khata| format!("the clock reads outside the calendar: {khata}"))?
            .to_string();
        let qaima = qaimat_sahb(tasalsul, &waqt, &khass, &Mulghayat::default())?;
        if let Some(walid) = wijha.parent() {
            fs::create_dir_all(walid).map_err(|khata| format!("{}: {khata}", walid.display()))?;
        }
        fs::write(wijha, &qaima).map_err(|khata| format!("{}: {khata}", wijha.display()))?;
        println!("  seed      {}", wijha.display());
        println!("    sequence  {tasalsul}");
        println!("    issued    {waqt}");
        println!("    key       {}", hex::encode(khass.aam().bayt()));
        return Ok(());
    }

    if let Some(masdar) = khiyarat.mujtama.as_deref() {
        let jidhr = khiyarat
            .jidhr
            .as_deref()
            .ok_or_else(|| "--jidhr is required".to_owned())?;
        return fahras_mujtama(masdar, jidhr, &khass);
    }

    // Proved once here rather than unwrapped at each use: everything below
    // writes a repository, and the two single-document modes returned above.
    let jidhr = khiyarat
        .jidhr
        .as_deref()
        .ok_or_else(|| "--jidhr is required".to_owned())?;
    let asas = khiyarat
        .asas
        .as_deref()
        .ok_or_else(|| "--asas is required".to_owned())?;
    let tasalsul = khiyarat
        .tasalsul
        .ok_or_else(|| "--tasalsul is required".to_owned())?;

    let mut kharijiya = Vec::with_capacity(khiyarat.kharijiya.len());
    for masar in &khiyarat.kharijiya {
        let madkhal = iqra_khariji(masar)?;
        println!("  read {}", masar.display());
        println!(
            "    entry     {} {}",
            madkhal.madkhal().id,
            madkhal.madkhal().unwan
        );
        println!("    shard     {:02x}", shareeha(madkhal.luba()));
        println!("    author    {}", madkhal.madkhal().nasab());
        println!("    release   {}", madkhal.madkhal().isdar);
        println!("    builds    {}", madkhal.madkhal().abniya.join(", "));
        for qitaa in &madkhal.madkhal().qitaa {
            println!(
                "    artifact  {} — {} byte(s), sha256 {}",
                qitaa.ism, qitaa.hajm, qitaa.sha256
            );
        }
        println!("    mirrored  {}", madkhal.madkhal().yajuz_mira());
        kharijiya.push(madkhal);
    }

    let mut madakhil = Vec::with_capacity(khiyarat.talabat.len());
    for talab in &khiyarat.talabat {
        let madkhal = madkhal_min_huzma(
            &talab.huzma,
            talab.luba,
            &talab.ism,
            jidhr,
            asas,
            khiyarat.mira.as_deref(),
            talab.tajawuz.as_deref(),
            // A package on this command line was sealed by whoever made it, so
            // the seal is the identity.
            None,
        )?;
        println!("  read {}", talab.huzma.display());
        println!(
            "    listing   {} {}",
            madkhal.mulakhkhas.id, madkhal.mulakhkhas.murajaa
        );
        println!("    shard     {:02x}", shareeha(madkhal.luba));
        println!("    asset     {}", madkhal.masar_asl);
        println!("    hash      {}", madkhal.mulakhkhas.basmat_muhtawa);
        println!("    size      {} byte(s)", madkhal.mulakhkhas.hajm);
        println!("    primary   {}", madkhal.mulakhkhas.rabt);
        println!("    mirror    {:?}", madkhal.mulakhkhas.rabt_mira);
        println!(
            "    method    {}",
            madkhal.mulakhkhas.tareeqa.wasf_injilizi()
        );
        if let Some(tajawuz) = &madkhal.tajawuz {
            println!("    OVERRIDE  its coverage gate refuses this package:");
            for sabab in &tajawuz.asbab {
                println!("                {sabab}");
            }
            println!("              published anyway: {}", tajawuz.sabab);
        }
        madakhil.push(madkhal);
    }

    let mustawda = ijri(
        jidhr,
        madakhil,
        BTreeMap::new(),
        kharijiya,
        KhiyaratSabk {
            tasalsul,
            waqt: &min_unix(unix_alan()),
            // The command line is the operator's own record; it names every
            // revocation still in force on each run and nothing checks it
            // against a served list it may not be able to reach.
            adna_ilghaat: 0,
        },
        &khass,
        &khiyarat.mulghayat,
    )?;

    println!(
        "  wrote {} shard(s) and a manifest at sequence {} into {}",
        taarib_mustawda::fahras::ADAD_SHARAIH,
        mustawda.bayan.tasalsul,
        mustawda.jidhr.display()
    );
    println!(
        "  manifest schema {}, published {}, revocation list at {:?} with {} revoked key(s) and \
         {} revoked lineage(s)",
        mustawda.bayan.isdar,
        mustawda.bayan.waqt,
        mustawda.bayan.rabt_qaimat_sahb,
        khiyarat.mulghayat.mafatih.len(),
        khiyarat.mulghayat.ruqa.len()
    );
    Ok(())
}
