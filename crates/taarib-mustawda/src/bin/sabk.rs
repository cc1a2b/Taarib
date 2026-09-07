//! سبك المستودع — casting the registry: sealed packages in, a served repository out.
//!
//! The maintainer-side counterpart to `taarib-mustawda`'s reader. It writes the
//! manifest, all 256 shards and the signed revocation list that a client then
//! fetches, and it derives every listing field from the package's own sealed
//! metadata rather than from anything typed twice.
//!
//! ```text
//! sabk --jidhr <repo> --tasalsul <n> --asas <https://...>
//!      [--mira <https://...>] [--huzma <file> --luba <uuid> --ism <title>]...
//! ```
//!
//! Run it after every publication, with the sequence number incremented: a
//! client caches on that number and will not look again until it moves.

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

use jiff::{SignedDuration, Timestamp};
use taarib_aman::qaimat_sahb::KatibQaima;
use taarib_khatm::MiftahKhass;
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{
    MulakhkhasRuqaa, RukhsaRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama,
};
use taarib_mustalahat::sawt::MulakhkhasSawt;
use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_mustawda::fahras::{BayanMustawda, MuhtawaShareeha, TajawuzNashr, shareeha};
use taarib_mustawda::masadir::{MASAR_BAYAN, masar_shareeha};
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tarqee::irtibat::IrtibatBina;
use taarib_tarqee::taghtiya_ruqaa::SababAdamAlnashr;

/// Where the revocation list sits inside the repository.
///
/// A repository-relative path, because `jalb_qaimat_sahb` refuses an absolute
/// address: a manifest that could aim the fetch at a host of its choosing is a
/// manifest no offline mirror can serve.
pub const MASAR_QAIMAT_SAHB: &str = "sahb/qaima.json";

/// The directory release assets live under, inside the same repository.
pub const MUJALLAD_ISDAR: &str = "isdar";

/// Everything one published patch contributes to the registry.
#[derive(Debug)]
pub struct MadkhalManshur {
    /// The game the shard keys on.
    pub luba: LubaId,
    /// The listing itself.
    pub mulakhkhas: MulakhkhasRuqaa,
    /// The package's own binding, kept beside the listing so the walk can hand
    /// it to the installer without re-deriving it.
    pub irtibat: IrtibatBina,
    /// Where the release asset landed inside the repository.
    pub masar_asl: String,
    /// The record written when this package was published over its own
    /// coverage gate's refusal, and [`None`] when the gate passed it.
    pub tajawuz: Option<TajawuzNashr>,
}

/// What a finished cast produced.
#[derive(Debug)]
#[allow(dead_code, reason = "the cast's whole product, reported by the caller")]
pub struct Mustawda {
    /// The repository root on disk.
    pub jidhr: PathBuf,
    /// The manifest that was written.
    pub bayan: BayanMustawda,
    /// Which shard indices carry content.
    pub sharaih: Vec<u16>,
    /// Every listing published, in the order it was given.
    pub madakhil: Vec<MadkhalManshur>,
}

/// Reads a sealed package and turns it into the listing a registry publishes.
///
/// The listing is derived from the package's own metadata section — its
/// identity, revision, title, coverage, engine, licence and build binding — and
/// from the bytes of the file itself, whose BLAKE3 hash is what
/// `taarib_mustawda::tanzeel` verifies the download against. Nothing is typed in
/// twice.
///
/// **The package's own coverage gate is honoured here.** Every compiled patch
/// carries `taghtiya.qabila_lil_nashr`, the verdict
/// `taarib_tarqee::taghtiya_ruqaa` reached over its own strings, and a caster
/// that read the coverage numbers while ignoring the verdict computed from them
/// would publish exactly the packages the project decided not to publish — and
/// would do it under whatever provenance the metadata happened to declare. A
/// refusing gate is therefore a refusal here, and `tajawuz` is the only way
/// past it: a sentence the operator has to write, which is recorded in the
/// manifest rather than swallowed.
///
/// # Errors
///
/// A sentence naming what in the package would not read, and the gate's own
/// reasons when it refuses and no override was given.
pub fn madkhal_min_huzma(
    masar_huzma: &Path,
    luba: LubaId,
    ism_luba: &str,
    jidhr_mustawda: &Path,
    asas_rabt: &str,
    asas_mira: Option<&str>,
    tajawuz: Option<&str>,
) -> Result<MadkhalManshur, String> {
    let bayt =
        fs::read(masar_huzma).map_err(|khata| format!("{}: {khata}", masar_huzma.display()))?;
    let basmat_muhtawa = Basma::min_bayt(*blake3::hash(&bayt).as_bytes());

    let malaf = MalafRuqaa::iftah(masar_huzma).map_err(|khata| khata.to_string())?;
    let ruqaa = malaf.ruqaa().map_err(|khata| khata.to_string())?;
    let kutla = ruqaa.tawqee();
    if !kutla.muwaqqaa() {
        return Err(
            "the package carries no signature; a registry publishes sealed packages only"
                .to_owned(),
        );
    }
    let bayan = ruqaa.bayan_json().map_err(|khata| khata.to_string())?;

    let haql = |ism: &str| -> Result<serde_json::Value, String> {
        bayan
            .get(ism)
            .cloned()
            .ok_or_else(|| format!("the manifest has no {ism:?} field"))
    };
    let id: RuqaaId = serde_json::from_value(haql("id")?).map_err(|k| k.to_string())?;
    let murajaa: RuqaaRevision =
        serde_json::from_value(haql("murajaa")?).map_err(|k| k.to_string())?;
    let irtibat: IrtibatBina =
        serde_json::from_value(haql("irtibat")?).map_err(|k| k.to_string())?;

    let wasf = haql("wasf")?;
    let unwan = nass(&wasf, "unwan")?;
    let ism_musahim = nass(&wasf, "ism_musahim")?;
    let rukhsa: RukhsaRuqaa = serde_json::from_value(
        wasf.get("rukhsa")
            .cloned()
            .ok_or("the manifest declares no licence")?,
    )
    .map_err(|k| k.to_string())?;
    let tareeqa: TareeqaTarjama = serde_json::from_value(
        wasf.get("tareeqa")
            .cloned()
            .ok_or("the manifest declares no translation method")?,
    )
    .map_err(|k| k.to_string())?;

    let muharrik = haql("muharrik")?;
    let aila: AilatMuharrik = serde_json::from_value(
        muharrik
            .get("aila")
            .cloned()
            .ok_or("the manifest declares no engine family")?,
    )
    .map_err(|k| k.to_string())?;
    let khalfiya: KhalfiyaBarmajiya = serde_json::from_value(
        muharrik
            .get("khalfiya")
            .cloned()
            .ok_or("the manifest declares no scripting backend")?,
    )
    .map_err(|k| k.to_string())?;
    let tabaqa: Tabaqa = serde_json::from_value(
        muharrik
            .get("tabaqa")
            .cloned()
            .ok_or("the manifest declares no support tier")?,
    )
    .map_err(|k| k.to_string())?;

    let taqrir = haql("taghtiya")?;
    let taghtiya_kulli: Taghtiya = serde_json::from_value(
        taqrir
            .get("kulli")
            .cloned()
            .ok_or("the manifest carries no coverage")?,
    )
    .map_err(|k| k.to_string())?;

    let tajawuz = bawwabat_taghtiya(&taqrir, id, murajaa, tajawuz)?;

    // The contributor *is* their key: `MusahimId` refuses anything that is not
    // a public key fingerprint, so the listing cannot claim an identity the
    // signature does not back.
    let musahim = MusahimId::jadeed(hex::encode(kutla.miftah)).map_err(|k| k.to_string())?;

    let waqt_nashr = min_unix(kutla.waqt);
    let ism_malaf = format!("{}-r{}.ruqaa", slug(ism_luba), murajaa.qeema());
    let masar_asl = format!("{MUJALLAD_ISDAR}/{id}/{ism_malaf}");

    // Both addresses are resolved before a byte is written, so a base the
    // caster refuses leaves the repository exactly as it found it rather than
    // an orphan asset in `isdar/` that no listing names.
    let rabt = rabt_asl(asas_rabt, &masar_asl, &ism_malaf)?;
    let rabt_mira = asas_mira
        .map(|asas| rabt_asl(asas, &masar_asl, &ism_malaf))
        .transpose()?;

    let wijha = jidhr_mustawda.join(MUJALLAD_ISDAR).join(id.to_string());
    fs::create_dir_all(&wijha).map_err(|khata| format!("{}: {khata}", wijha.display()))?;
    fs::write(wijha.join(&ism_malaf), &bayt)
        .map_err(|khata| format!("{}: {khata}", wijha.join(&ism_malaf).display()))?;

    let mulakhkhas = MulakhkhasRuqaa {
        id,
        murajaa,
        unwan,
        musahim,
        ism_musahim,
        taghtiya: taghtiya_kulli,
        adad_nusus: taghtiya_kulli.mutarjam,
        hajm: bayt.len() as u64,
        bina_manassa: irtibat.manassat.clone(),
        basmat: irtibat.basmat.clone(),
        aila,
        khalfiya,
        tabaqa,
        tareeqa,
        rukhsa,
        taqyeem: None,
        adad_taqyeemat: 0,
        waqt_nashr,
        basmat_muhtawa,
        rabt,
        rabt_mira,
    };

    Ok(MadkhalManshur {
        luba,
        mulakhkhas,
        irtibat,
        masar_asl,
        tajawuz,
    })
}

/// Applies the package's own coverage gate.
///
/// The verdict is read out of the sealed metadata rather than recomputed here.
/// `TaqrirTaghtiya::qabila_lil_nashr` is `Taghtiya::qabila_lil_nashr` already
/// narrowed by the blocking causes, and a caster that re-derived it from the
/// summary alone would answer `true` for a project whose opening was never
/// measured — an empty first-hour denominator reads as a perfect first hour,
/// which is the whole reason `SababAdamAlnashr::BilaJalsatAwwal` exists.
///
/// # Errors
///
/// A refusal listing every blocking cause, when the gate says no and no
/// override sentence was given.
fn bawwabat_taghtiya(
    taqrir: &serde_json::Value,
    id: RuqaaId,
    murajaa: RuqaaRevision,
    tajawuz: Option<&str>,
) -> Result<Option<TajawuzNashr>, String> {
    let qabila = taqrir
        .get("qabila_lil_nashr")
        .and_then(serde_json::Value::as_bool)
        .ok_or("the manifest's coverage report carries no publishable verdict")?;
    if qabila {
        if tajawuz.is_some() {
            return Err(format!(
                "{id} r{} needs no --tajawuz: its own coverage gate already permits it",
                murajaa.qeema()
            ));
        }
        return Ok(None);
    }

    let asbab: Vec<SababAdamAlnashr> = taqrir
        .get("asbab")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|khata| format!("the coverage report's reasons would not read: {khata}"))?
        .unwrap_or_default();
    let hasima: Vec<String> = asbab
        .iter()
        .filter(|sabab| sabab.hasim())
        .map(|sabab| sabab.wasf_injilizi())
        .collect();
    // A refusing verdict with no blocking cause is a report that contradicts
    // itself, and publishing on the strength of it would be publishing on the
    // strength of a bug.
    let matn = if hasima.is_empty() {
        vec!["the report gives no cause, which is itself a reason to look".to_owned()]
    } else {
        hasima
    };

    let Some(sabab) = tajawuz else {
        return Err(format!(
            "{id} r{} declares qabila_lil_nashr: false — its own coverage gate refuses to \
             publish it:\n    {}\n  Publish it anyway with --tajawuz <why>, which is recorded \
             in the manifest for every reader of the catalogue.",
            murajaa.qeema(),
            matn.join("\n    ")
        ));
    };

    Ok(Some(TajawuzNashr {
        ruqaa: id,
        murajaa,
        asbab: matn,
        sabab: sabab.to_owned(),
    }))
}

/// The absolute address one release asset is served from.
///
/// A forge and a CDN disagree about the shape of that address, and both have to
/// be spellable. GitHub's release area is flat — an asset is a name, not a path,
/// so `.../releases/download/<tag>/<name>.ruqaa` is the only form it answers —
/// while a CDN mirroring the repository serves the file where the tree puts it,
/// at `isdar/<id>/<name>.ruqaa`. A base carrying `{ism}` gets the file name and
/// a base carrying `{masar}` gets the repository path; a base carrying neither
/// has the repository path appended, which is what a plain directory server and
/// an offline mirror want.
///
/// The last form is the one that bites. `--asas` reads as *where the release
/// assets are served from*, and in this tree they are served from `isdar/`, so
/// the natural thing to write is a base ending in that segment — which then
/// gets the repository path appended to it and produces `isdar/isdar/<id>/…`,
/// an address no mirror answers and nothing else checks. The doubling is caught
/// here, on the command line, rather than shipped inside a signed listing.
///
/// # Errors
///
/// A sentence naming the base and both correct spellings, when the base already
/// ends in the release directory or when the join does not produce an `https`
/// address.
fn rabt_asl(asas: &str, masar_asl: &str, ism_malaf: &str) -> Result<String, String> {
    if asas.contains("{ism}") || asas.contains("{masar}") {
        return tahaqquq_rabt(
            asas.replace("{masar}", masar_asl)
                .replace("{ism}", ism_malaf),
        );
    }

    let maqsus = asas.trim_end_matches('/');
    if maqsus
        .rsplit('/')
        .next()
        .is_some_and(|akhir| akhir == MUJALLAD_ISDAR)
    {
        return Err(format!(
            "{asas:?} already ends in {MUJALLAD_ISDAR:?}, and a base with no placeholder has \
             the whole repository path {masar_asl:?} appended to it — which would publish \
             {maqsus}/{masar_asl}. Give the repository root instead, or spell the asset out \
             with {{ism}} for a flat release area or {{masar}} for a repository path."
        ));
    }
    tahaqquq_rabt(format!("{maqsus}/{masar_asl}"))
}

/// Refuses an address a client would not fetch.
///
/// `masadir::rabt_kamil` refuses a non-`https` registry source at fetch time,
/// and `tanzeel` refuses one at download time. Both of those are on the reader's
/// machine, hours after the listing was signed; this is the same rule applied on
/// the machine that writes it.
fn tahaqquq_rabt(rabt: String) -> Result<String, String> {
    let muhallal = reqwest::Url::parse(&rabt)
        .map_err(|khata| format!("{rabt:?} is not a usable asset address: {khata}"))?;
    if muhallal.scheme() != "https" {
        return Err(format!(
            "{rabt:?} is not https, and a package is never fetched in the clear"
        ));
    }
    Ok(rabt)
}

/// Writes the whole repository: shards, manifest, revocation list.
///
/// Shards are written first and hashed as they are written, because the
/// manifest is nothing but the list of those hashes; writing it from anything
/// other than the bytes that actually landed would let the two disagree.
///
/// **All 256 shards are written, empty ones included, and the manifest declares
/// every one of them.** That is not a choice: `jalb::jalb_sharaih` asks for
/// every shard the user's games fall in and treats a shard that does not
/// resolve as a failure of the whole fetch, so a catalogue that published only
/// its non-empty shards would show *nothing at all* to any user who owns a game
/// in an empty one — which, on a fresh registry, is every user. An empty shard
/// is 23 bytes and the full manifest is about 20 KB.
///
/// # Errors
///
/// A sentence naming the write or the encoding that failed.
pub fn ijri(
    jidhr: &Path,
    madakhil: Vec<MadkhalManshur>,
    aswat: BTreeMap<LubaId, Vec<MulakhkhasSawt>>,
    tasalsul: u64,
    waqt: &str,
    miftah_malik: &MiftahKhass,
    mulghayat: &[(String, String)],
) -> Result<Mustawda, String> {
    let mut mahtawayat: BTreeMap<u16, MuhtawaShareeha> = BTreeMap::new();
    for madkhal in &madakhil {
        let raqm = shareeha(madkhal.luba);
        mahtawayat
            .entry(raqm)
            .or_default()
            .ruqaa
            .entry(madkhal.luba)
            .or_default()
            .push(madkhal.mulakhkhas.clone());
    }
    for (luba, qaima) in aswat {
        let raqm = shareeha(luba);
        mahtawayat
            .entry(raqm)
            .or_default()
            .aswat
            .insert(luba, qaima);
    }

    let mut basmat: BTreeMap<u16, Basma> = BTreeMap::new();
    for raqm in 0..taarib_mustawda::fahras::ADAD_SHARAIH {
        let farigha = MuhtawaShareeha::default();
        let muhtawa = mahtawayat.get(&raqm).unwrap_or(&farigha);
        let bayt = serde_json::to_vec(muhtawa).map_err(|khata| khata.to_string())?;
        let nisbi = masar_shareeha(raqm).map_err(|khata| khata.to_string())?;
        uktub(jidhr, &nisbi, &bayt)?;
        basmat.insert(raqm, Basma::min_bayt(*blake3::hash(&bayt).as_bytes()));
    }

    let qaima = qaimat_sahb(tasalsul, waqt, miftah_malik, mulghayat)?;
    uktub(jidhr, MASAR_QAIMAT_SAHB, &qaima)?;

    let bayan = BayanMustawda {
        isdar: taarib_mustawda::fahras::ISDAR_BAYAN,
        tasalsul,
        waqt: waqt.to_owned(),
        sharaih: basmat,
        rabt_qaimat_sahb: MASAR_QAIMAT_SAHB.to_owned(),
        tajawuzat: madakhil
            .iter()
            .filter_map(|madkhal| madkhal.tajawuz.clone())
            .collect(),
    };
    let bayt = serde_json::to_vec_pretty(&bayan).map_err(|khata| khata.to_string())?;
    uktub(jidhr, MASAR_BAYAN, &bayt)?;

    let sharaih = bayan.sharaih.keys().copied().collect();
    Ok(Mustawda {
        jidhr: jidhr.to_path_buf(),
        bayan,
        sharaih,
        madakhil,
    })
}

/// How long a cast list stays current before every client reports it stale.
///
/// A year from the cast: long enough that a machine that never comes online
/// again keeps installing from its local copy, short enough that a registry
/// nobody has re-signed in a year is reported as exactly that. It used to be a
/// fixed calendar date, which every cast after it would have shipped already
/// expired.
const MUDDAT_SALAHIYA: SignedDuration = SignedDuration::from_hours(24 * 365);

/// The signed revocation list, through the one writer that shares its
/// canonical form with the verifier.
///
/// The form used to be spelled here a second time, by hand, and a byte of
/// drift between the two would have produced lists that verify against
/// nothing. `KatibQaima` produces bytes and cannot produce a `QaimatSahb`, so
/// the verifier is still the only constructor of that type.
fn qaimat_sahb(
    tasalsul: u64,
    waqt: &str,
    khass: &MiftahKhass,
    mulghayat: &[(String, String)],
) -> Result<Vec<u8>, String> {
    let usdirat = waqt
        .parse::<Timestamp>()
        .map_err(|khata| format!("{waqt:?} is not an RFC 3339 timestamp: {khata}"))?;
    let salih_hatta = usdirat
        .checked_add(MUDDAT_SALAHIYA)
        .map_err(|khata| format!("the validity window runs off the calendar: {khata}"))?;

    let mut katib = KatibQaima::jadeed(tasalsul, waqt, &salih_hatta.to_string());
    for (miftah, sabab) in mulghayat {
        let mut khaam = [0_u8; 32];
        hex::decode_to_slice(miftah, &mut khaam)
            .map_err(|khata| format!("{miftah:?} is not a 64-hex key: {khata}"))?;
        katib = katib.ilgha_miftah(khaam, sabab, waqt);
    }
    katib.uktub(khass).map_err(|khata| khata.to_string())
}

fn uktub(jidhr: &Path, nisbi: &str, bayt: &[u8]) -> Result<(), String> {
    let masar = jidhr.join(nisbi);
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid).map_err(|khata| format!("{}: {khata}", walid.display()))?;
    }
    fs::write(&masar, bayt).map_err(|khata| format!("{}: {khata}", masar.display()))
}

fn nass(qeema: &serde_json::Value, ism: &str) -> Result<String, String> {
    qeema
        .get(ism)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("the manifest has no {ism:?} field"))
}

/// A lowercase, hyphen-separated file name, so the asset path stays inside the
/// byte set `masar_salih` accepts and a mirror never has to escape it.
fn slug(nass: &str) -> String {
    let mut makhraj = String::with_capacity(nass.len());
    let mut fasil = false;
    for harf in nass.chars() {
        if harf.is_ascii_alphanumeric() {
            if fasil && !makhraj.is_empty() {
                makhraj.push('-');
            }
            fasil = false;
            makhraj.push(harf.to_ascii_lowercase());
        } else {
            fasil = true;
        }
    }
    if makhraj.is_empty() {
        "ruqaa".to_owned()
    } else {
        makhraj
    }
}

/// Unix seconds as RFC 3339, which is what every listing field in the registry
/// is written in.
fn min_unix(thawani: i64) -> String {
    // `jiff` owns every calendar rule in this workspace, leap years included.
    // This was forty lines of hand-rolled civil-date arithmetic, carried over
    // from a scratch crate where jiff genuinely was not a dependency. Here it
    // is one, and a second implementation of the Gregorian calendar is a second
    // place to be wrong about February.
    Timestamp::from_second(thawani).map_or_else(
        |_| "1970-01-01T00:00:00Z".to_owned(),
        |waqt| waqt.to_string(),
    )
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
    jidhr: PathBuf,
    tasalsul: u64,
    asas: String,
    mira: Option<String>,
    talabat: Vec<TalabNashr>,
    ism_miftah: Option<String>,
    mulghayat: Vec<(String, String)>,
}

const ISTIMAL: &str = "\
سبك — cast the registry from sealed packages

  sabk --jidhr <repo> --tasalsul <n> --asas <https://base/>
       [--mira <https://mirror/>] [--ism-miftah <keychain account>]
       [--huzma <file.ruqaa> --luba <game-uuid> --ism <title> [--tajawuz <why>]]...
       [--mulgha <64-hex key> --sabab <why>]...

  --jidhr       the repository working tree to write into (required)
  --tasalsul    the manifest sequence number; a client caches on it and will
                not look again until it moves, so increment it every time
  --asas        the base the repository path is resolved against. A base holding
                {ism} gets the asset's file name, which is the only form a
                forge's flat release area answers; one holding {masar} gets its
                repository path; one holding neither has the whole repository
                path — isdar/<id>/<name> — appended, which is what a directory
                server wants, so give it the repository root and not the isdar
                directory
  --mira        an optional second address for the same assets, same spelling
  --ism-miftah  the keychain account holding the signing key
  --huzma       a sealed package to publish; repeatable, and each one must be
                followed by its --luba and --ism
  --tajawuz     publish the preceding --huzma even though its own coverage gate
                refuses it, giving the reason; the reason is written into the
                manifest, where every reader of the catalogue sees it
  --mulgha      a signing key to revoke, as 64 lowercase hex; repeatable, and
                each one must be followed by its --sabab

Every listing field except the game's identity and title is read out of the
package's own sealed metadata. A package with no signature is refused, and so is
one whose metadata says qabila_lil_nashr: false unless --tajawuz says why.
";

/// Reads the command line, or explains why it will not.
fn iqra_khiyarat() -> Result<Option<Khiyarat>, String> {
    let mut hujaj = std::env::args().skip(1);
    let (mut jidhr, mut tasalsul, mut asas, mut mira, mut ism_miftah) =
        (None, None, None, None, None);
    let mut talabat: Vec<TalabNashr> = Vec::new();
    let mut mulghayat: Vec<(String, String)> = Vec::new();
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
                mulghayat.push((khaam.to_ascii_lowercase(), String::new()));
            },
            "--sabab" => {
                let sabab = baad(&mut hujaj, "--sabab")?;
                mulghayat
                    .last_mut()
                    .ok_or_else(|| "--sabab must follow a --mulgha".to_owned())?
                    .1 = sabab;
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
    for (miftah, sabab) in &mulghayat {
        if sabab.is_empty() {
            return Err(format!("--mulgha {miftah} was given no --sabab"));
        }
    }
    Ok(Some(Khiyarat {
        jidhr: jidhr.ok_or_else(|| "--jidhr is required".to_owned())?,
        tasalsul: tasalsul.ok_or_else(|| "--tasalsul is required".to_owned())?,
        asas: asas.ok_or_else(|| "--asas is required".to_owned())?,
        mira,
        talabat,
        ism_miftah,
        mulghayat,
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

    // The signing key never leaves the keychain; only its public half is ever
    // written into the repository, inside the revocation list's signature.
    let khass = match khiyarat.ism_miftah.as_deref() {
        Some(ism) => taarib_khatm::mafatih::hat(ism),
        None => taarib_khatm::malik::hat_malik(),
    }
    .map_err(|khata| format!("no signing key in this machine's keychain: {khata}"))?;

    let mut madakhil = Vec::with_capacity(khiyarat.talabat.len());
    for talab in &khiyarat.talabat {
        let madkhal = madkhal_min_huzma(
            &talab.huzma,
            talab.luba,
            &talab.ism,
            &khiyarat.jidhr,
            &khiyarat.asas,
            khiyarat.mira.as_deref(),
            talab.tajawuz.as_deref(),
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
        &khiyarat.jidhr,
        madakhil,
        BTreeMap::new(),
        khiyarat.tasalsul,
        &min_unix(unix_alan()),
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
        "  manifest schema {}, published {}, revocation list at {:?} with {} revoked key(s)",
        mustawda.bayan.isdar,
        mustawda.bayan.waqt,
        mustawda.bayan.rabt_qaimat_sahb,
        khiyarat.mulghayat.len()
    );
    Ok(())
}

#[cfg(test)]
mod fahs {
    use std::error::Error;

    use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
    use taarib_tarqee::taghtiya_ruqaa::SababAdamAlnashr;

    use super::{bawwabat_taghtiya, rabt_asl};

    /// Every test returns this so a setup failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    const ID: &str = "01a06d42-dc5d-74b2-b27d-5c4441a03a3f";
    const ASL: &str = "isdar/01a06d42-dc5d-74b2-b27d-5c4441a03a3f/r-e-p-o-r2.ruqaa";
    const ISM: &str = "r-e-p-o-r2.ruqaa";

    fn hawiya() -> Result<(RuqaaId, RuqaaRevision), uuid::Error> {
        Ok((
            RuqaaId::min_uuid(uuid::Uuid::parse_str(ID)?),
            RuqaaRevision::jadeeda(2),
        ))
    }

    /// A coverage report shaped the way a sealed package carries one.
    fn taqrir(
        qabila: bool,
        asbab: &[SababAdamAlnashr],
    ) -> Result<serde_json::Value, serde_json::Error> {
        let asbab: Vec<serde_json::Value> = asbab
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<_, _>>()?;
        Ok(serde_json::json!({ "qabila_lil_nashr": qabila, "asbab": asbab }))
    }

    #[test]
    fn bawwaba_tasmah_bila_tajawuz() -> NatijatIkhtibar {
        let (id, murajaa) = hawiya()?;
        assert_eq!(
            bawwabat_taghtiya(&taqrir(true, &[])?, id, murajaa, None),
            Ok(None)
        );
        Ok(())
    }

    #[test]
    fn bawwaba_tarfud_tajawuzan_la_yalzam() -> NatijatIkhtibar {
        let (id, murajaa) = hawiya()?;
        let khata = bawwabat_taghtiya(&taqrir(true, &[])?, id, murajaa, Some("why"))
            .err()
            .ok_or("an override over a passing gate must be refused")?;
        assert!(khata.contains("needs no --tajawuz"), "{khata}");
        Ok(())
    }

    /// The whole point of the change: a refusing gate stops the cast.
    #[test]
    fn bawwaba_tarfud_huzma_ghayr_qabila() -> NatijatIkhtibar {
        let (id, murajaa) = hawiya()?;
        let taqrir = taqrir(false, &[SababAdamAlnashr::BilaJalsatAwwal])?;
        let khata = bawwabat_taghtiya(&taqrir, id, murajaa, None)
            .err()
            .ok_or("a package its own gate refuses must not cast")?;
        assert!(khata.contains("qabila_lil_nashr: false"), "{khata}");
        assert!(khata.contains("capture session"), "{khata}");
        assert!(khata.contains("--tajawuz"), "{khata}");
        Ok(())
    }

    #[test]
    fn tajawuz_yusajjil_asbab_albawwaba() -> NatijatIkhtibar {
        let (id, murajaa) = hawiya()?;
        let taqrir = taqrir(
            false,
            &[
                SababAdamAlnashr::BilaJalsatAwwal,
                // Not blocking, so it must not appear in the record.
                SababAdamAlnashr::MarfudaBaqiya { adad: 3 },
            ],
        )?;
        let tajawuz = bawwabat_taghtiya(&taqrir, id, murajaa, Some("for the client's own walk"))?
            .ok_or("an override over a refusing gate must be recorded")?;
        assert_eq!(tajawuz.ruqaa, id);
        assert_eq!(tajawuz.murajaa, murajaa);
        assert_eq!(tajawuz.sabab, "for the client's own walk");
        assert_eq!(
            tajawuz.asbab.len(),
            1,
            "only blocking causes: {:?}",
            tajawuz.asbab
        );
        Ok(())
    }

    /// A refusal with no cause is a report contradicting itself, and the record
    /// says so rather than reading as an override of nothing.
    #[test]
    fn bawwaba_tusajjil_taqriran_bila_sabab() -> NatijatIkhtibar {
        let (id, murajaa) = hawiya()?;
        let tajawuz = bawwabat_taghtiya(&taqrir(false, &[])?, id, murajaa, Some("anyway"))?
            .ok_or("an override must be recorded")?;
        assert_eq!(
            tajawuz.asbab,
            vec!["the report gives no cause, which is itself a reason to look".to_owned()]
        );
        Ok(())
    }

    #[test]
    fn bawwaba_tarfud_taqriran_bila_hukm() -> NatijatIkhtibar {
        let (id, murajaa) = hawiya()?;
        let khata = bawwabat_taghtiya(&serde_json::json!({}), id, murajaa, None)
            .err()
            .ok_or("a report with no verdict is not a report")?;
        assert!(khata.contains("no publishable verdict"), "{khata}");
        Ok(())
    }

    #[test]
    fn rabt_ism_yamla_mintaqat_alisdar_almusattaha() -> NatijatIkhtibar {
        let rabt = rabt_asl(
            "https://forge.example/releases/download/nashr-2/{ism}",
            ASL,
            ISM,
        )?;
        assert_eq!(
            rabt,
            format!("https://forge.example/releases/download/nashr-2/{ISM}")
        );
        Ok(())
    }

    #[test]
    fn rabt_masar_yamla_masar_almustawda() -> NatijatIkhtibar {
        let rabt = rabt_asl(
            "https://cdn.example/gh/owner/repo@nashr-2/{masar}",
            ASL,
            ISM,
        )?;
        assert_eq!(
            rabt,
            format!("https://cdn.example/gh/owner/repo@nashr-2/{ASL}")
        );
        Ok(())
    }

    #[test]
    fn rabt_bila_qalab_yulhiq_masar_almustawda() -> NatijatIkhtibar {
        let rabt = rabt_asl("https://cdn.example/gh/owner/repo@nashr-2/", ASL, ISM)?;
        assert_eq!(
            rabt,
            format!("https://cdn.example/gh/owner/repo@nashr-2/{ASL}")
        );
        Ok(())
    }

    /// The defect this refusal exists for: `--asas .../isdar` reads as *where
    /// the assets are* and silently produced `isdar/isdar/…`.
    #[test]
    fn rabt_yarfud_takrar_mujallad_alisdar() -> NatijatIkhtibar {
        let khata = rabt_asl("https://cdn.example/gh/owner/repo@nashr-2/isdar", ASL, ISM)
            .err()
            .ok_or("a base already ending in the release directory doubles it")?;
        assert!(khata.contains("already ends in"), "{khata}");
        assert!(
            khata.contains("{ism}") && khata.contains("{masar}"),
            "{khata}"
        );
        Ok(())
    }

    #[test]
    fn rabt_yarfud_takrar_maa_maylan_akhir() {
        assert!(rabt_asl("https://cdn.example/repo/isdar/", ASL, ISM).is_err());
    }

    #[test]
    fn rabt_yarfud_ghayr_almuamman() -> NatijatIkhtibar {
        let khata = rabt_asl("http://cdn.example/repo", ASL, ISM)
            .err()
            .ok_or("a package is never fetched in the clear")?;
        assert!(khata.contains("not https"), "{khata}");
        Ok(())
    }

    #[test]
    fn rabt_yarfud_asasan_la_yuhallal() {
        assert!(rabt_asl("not an address at all", ASL, ISM).is_err());
    }
}
