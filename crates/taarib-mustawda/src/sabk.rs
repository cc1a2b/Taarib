//! سبك المستودع — casting the registry: sealed packages in, a served repository out.
//!
//! The maintainer's side of everything [`crate::jalb`] reads. It writes the
//! manifest, all 256 shards and the signed revocation list that a client then
//! fetches, and it derives every listing field from the package's own sealed
//! metadata rather than from anything typed twice.
//!
//! Two callers share it: the `sabk` command, run by hand, and the Studio's
//! review console, which casts the owner's own catalogue after an approval. A
//! second implementation of the manifest would be a second place for its shape
//! to drift, and drift here produces a catalogue that verifies against nothing.
//!
//! Failures are sentences rather than an enum. Every one of them is an
//! operator's next action — a base address spelled wrong, a package whose own
//! coverage gate refuses it, a directory that would not open — and there is no
//! caller that branches on which: the Studio wraps the sentence in
//! `KhataTaqdeem::NashrFashil` beside the step it failed at, and the command
//! prints it.

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
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tarqee::irtibat::IrtibatBina;
use taarib_tarqee::taghtiya_ruqaa::SababAdamAlnashr;

use crate::fahras::{BayanMustawda, MuhtawaShareeha, TajawuzNashr, shareeha};
use crate::masadir::{MASAR_BAYAN, masar_shareeha};

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

/// Everything the cast withdraws from circulation, carried into the signed
/// revocation list.
///
/// The list is rebuilt whole on every cast rather than amended, so anything
/// missing here stops being revoked. That is why the caller is expected to hand
/// over its complete record, and why [`ijri`] refuses a list that carries fewer
/// revocations than the one already served — see `adna_ilghaat`.
#[derive(Debug, Clone, Default)]
pub struct Mulghayat {
    /// Signing keys, as 64 lowercase hex characters, each with its reason.
    pub mafatih: Vec<(String, String)>,
    /// Patch lineages, each with its reason.
    pub ruqa: Vec<(RuqaaId, String)>,
}

impl Mulghayat {
    /// How many revocations this list carries across both kinds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.mafatih.len().saturating_add(self.ruqa.len())
    }
}

/// Reads a sealed package and turns it into the listing a registry publishes.
///
/// The listing is derived from the package's own metadata section — its
/// identity, revision, title, coverage, engine, licence and build binding — and
/// from the bytes of the file itself, whose BLAKE3 hash is what
/// [`crate::tanzeel`] verifies the download against. Nothing is typed in twice.
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
/// `musahim` says who made the patch, and is needed only when the package no
/// longer carries their seal. A contributor's own build is signed by them, so
/// [`None`] reads the identity out of the signature and the listing cannot
/// claim one the signature does not back. A package the owner approved is
/// re-sealed with the *owner's* key — that is what approval is — and after that
/// the maker's identity is nowhere in the file, so a listing derived from the
/// signature alone would credit the owner for every patch anybody wrote, and
/// the pre-flight gate that asks "have you already published one of these"
/// would stop recognising anyone's work as their own.
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
    musahim: Option<MusahimId>,
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
    // Absent on a patch translated from nothing but the game, and on every
    // package sealed before the field existed; carried into the listing when
    // it is there, because the catalogue is where the credit is read.
    let masdar_khariji: Option<taarib_mustalahat::ruqaa::MasdarKhariji> = wasf
        .get("masdar_khariji")
        .filter(|qeema| !qeema.is_null())
        .map(|qeema| serde_json::from_value(qeema.clone()))
        .transpose()
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

    // Absent an answer from the caller, the contributor *is* their key:
    // `MusahimId` refuses anything that is not a public key fingerprint, so the
    // listing cannot claim an identity the signature does not back.
    let musahim = match musahim {
        Some(musahim) => musahim,
        None => MusahimId::jadeed(hex::encode(kutla.miftah)).map_err(|k| k.to_string())?,
    };

    let waqt_nashr = min_unix(kutla.waqt);
    let ism_malaf = ism_asl(ism_luba, murajaa);
    let masar_asl = masar_asl(id, &ism_malaf);

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
        masdar_khariji,
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

/// The file name one package is served under: the game's title, slugged, with
/// its revision.
#[must_use]
pub fn ism_asl(ism_luba: &str, murajaa: RuqaaRevision) -> String {
    format!("{}-r{}.ruqaa", slug(ism_luba), murajaa.qeema())
}

/// The repository path one package's asset occupies.
#[must_use]
pub fn masar_asl(id: RuqaaId, ism_malaf: &str) -> String {
    format!("{MUJALLAD_ISDAR}/{id}/{ism_malaf}")
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
/// has the repository path appended, which is what a plain directory server, a
/// forge's raw-content endpoint and an offline mirror want.
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
pub fn rabt_asl(asas: &str, masar_asl: &str, ism_malaf: &str) -> Result<String, String> {
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

/// Everything one cast needs that is not a package.
#[derive(Debug, Clone, Copy)]
pub struct KhiyaratSabk<'a> {
    /// The manifest sequence to publish at. A client caches on it and will not
    /// look again until it moves.
    pub tasalsul: u64,
    /// When the cast happened, RFC 3339.
    pub waqt: &'a str,
    /// How many revocations the served list already carries, which this cast
    /// may not go below.
    pub adna_ilghaat: usize,
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
/// **The revocation list is rebuilt, not amended.** It carries no history of
/// its own, so a cast handed fewer revocations than the served list already
/// holds would un-revoke the difference — a key the owner withdrew would start
/// verifying again on every machine that refreshed. `adna_ilghaat` is the
/// served list's own count, and going below it is refused here rather than
/// discovered by whoever installs the patch that was pulled.
///
/// # Errors
///
/// A sentence naming the write, the encoding, or the revocation count that
/// failed.
pub fn ijri(
    jidhr: &Path,
    madakhil: Vec<MadkhalManshur>,
    aswat: BTreeMap<LubaId, Vec<MulakhkhasSawt>>,
    khiyarat: KhiyaratSabk<'_>,
    miftah_malik: &MiftahKhass,
    mulghayat: &Mulghayat,
) -> Result<Mustawda, String> {
    if mulghayat.adad() < khiyarat.adna_ilghaat {
        return Err(format!(
            "this cast carries {} revocation(s) and the list it replaces carries {}; a \
             rebuilt list that drops one un-revokes it on every machine that refreshes. \
             Name the missing revocations before casting.",
            mulghayat.adad(),
            khiyarat.adna_ilghaat
        ));
    }

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
    for raqm in 0..crate::fahras::ADAD_SHARAIH {
        let farigha = MuhtawaShareeha::default();
        let muhtawa = mahtawayat.get(&raqm).unwrap_or(&farigha);
        let bayt = serde_json::to_vec(muhtawa).map_err(|khata| khata.to_string())?;
        let nisbi = masar_shareeha(raqm).map_err(|khata| khata.to_string())?;
        uktub(jidhr, &nisbi, &bayt)?;
        basmat.insert(raqm, Basma::min_bayt(*blake3::hash(&bayt).as_bytes()));
    }

    let qaima = qaimat_sahb(khiyarat.tasalsul, khiyarat.waqt, miftah_malik, mulghayat)?;
    uktub(jidhr, MASAR_QAIMAT_SAHB, &qaima)?;

    let bayan = BayanMustawda {
        isdar: crate::fahras::ISDAR_BAYAN,
        tasalsul: khiyarat.tasalsul,
        waqt: khiyarat.waqt.to_owned(),
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
///
/// # Errors
///
/// A sentence naming the timestamp, the key or the write that failed.
pub fn qaimat_sahb(
    tasalsul: u64,
    waqt: &str,
    khass: &MiftahKhass,
    mulghayat: &Mulghayat,
) -> Result<Vec<u8>, String> {
    let usdirat = waqt
        .parse::<Timestamp>()
        .map_err(|khata| format!("{waqt:?} is not an RFC 3339 timestamp: {khata}"))?;
    let salih_hatta = usdirat
        .checked_add(MUDDAT_SALAHIYA)
        .map_err(|khata| format!("the validity window runs off the calendar: {khata}"))?;

    let mut katib = KatibQaima::jadeed(tasalsul, waqt, &salih_hatta.to_string());
    for (miftah, sabab) in &mulghayat.mafatih {
        let mut khaam = [0_u8; 32];
        hex::decode_to_slice(miftah, &mut khaam)
            .map_err(|khata| format!("{miftah:?} is not a 64-hex key: {khata}"))?;
        katib = katib.ilgha_miftah(khaam, sabab, waqt);
    }
    for (ruqaa, sabab) in &mulghayat.ruqa {
        katib = katib.ilgha_ruqaa(*ruqaa, sabab, waqt);
    }
    katib.uktub(khass).map_err(|khata| khata.to_string())
}

/// Writes one repository-relative path, making its directory first.
///
/// # Errors
///
/// A sentence naming the directory or the file that would not be written.
pub fn uktub(jidhr: &Path, nisbi: &str, bayt: &[u8]) -> Result<(), String> {
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
#[must_use]
pub fn min_unix(thawani: i64) -> String {
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

#[cfg(test)]
mod fahs {
    use std::error::Error;

    use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
    use taarib_tarqee::taghtiya_ruqaa::SababAdamAlnashr;

    use super::{bawwabat_taghtiya, ism_asl, masar_asl, rabt_asl};

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

    /// The registry the product ships against: the raw-content root, with the
    /// repository path appended. This is the address a listing has to carry for
    /// another user to install anything at all.
    #[test]
    fn rabt_jidhr_alkhaam_yuntij_unwan_tanzil() -> NatijatIkhtibar {
        let ism = ism_asl("R.E.P.O.", RuqaaRevision::jadeeda(2));
        let (id, _) = hawiya()?;
        let masar = masar_asl(id, &ism);
        let rabt = rabt_asl(
            "https://raw.githubusercontent.com/cc1a2b/taarib-registry/main",
            &masar,
            &ism,
        )?;
        assert_eq!(
            rabt,
            "https://raw.githubusercontent.com/cc1a2b/taarib-registry/main/isdar/\
             01a06d42-dc5d-74b2-b27d-5c4441a03a3f/r-e-p-o-r2.ruqaa"
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
