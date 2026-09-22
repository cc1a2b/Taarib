//! النشر إلى المستودع — what actually reaches the registry, and what a refused
//! push leaves behind.
//!
//! Driven against a real git remote on a real filesystem rather than a mock.
//! Two of the three behaviours under test only exist at that level: the
//! sequence number is read back out of the manifest the previous push left in
//! the remote, and a rejected push is reported by `git2` through an update
//! callback rather than through the return value — a remote that accepts
//! everything would never exercise either.
//!
//! The owner key is a throwaway keypair minted here. Nothing in this file
//! reads the machine's keychain, and the real owner key never appears in a
//! test, a fixture or an environment variable.

use std::error::Error;
use std::path::Path;

use taarib_khatm::MiftahKhass;
use taarib_mustalahat::khariji::{FapsBina, HalatMira, RuqaaKharijiya, TahdidBina};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{IdhnMasdar, RuqaaId, RuqaaRevision};
use taarib_mustawda::fahras::{BayanMustawda, ShareehaMuwaththaqa, shareeha};
use taarib_mustawda::khariji::badhrat_rtea;
use taarib_mustawda::masadir::masar_shareeha;
use taarib_mustawda::sabk::masar_mira;
use taarib_ruqaa::katib::Katib;
use taarib_taqdeem::hawiya::SalahiyatMalik;
use taarib_taqdeem::khata::KhataTaqdeem;
use taarib_taqdeem::nashr::waqqi;
use taarib_taqdeem::nashr_mustawda::{
    MadkhalNashr, NatijatNashrMustawda, SijillNashr, TalabNashrMustawda, unshur,
};

/// Every test returns this so a setup failure propagates with `?`.
/// `unwrap` and `expect` are denied workspace-wide, tests included.
type NatijatIkhtibar = Result<(), Box<dyn Error>>;

/// The branch the catalogue is served from, as `masadir.rasmi` names it.
const FAR: &str = "main";

/// The registry root a client reads, which is also the base every asset
/// address is resolved against.
const ASAS: &str = "https://raw.githubusercontent.com/cc1a2b/taarib-registry/main";

/// A throwaway owner key and the authority it proves.
///
/// The seed is a constant rather than random so a failure is reproducible; it
/// is not a key anything but this file has ever seen.
fn malik() -> Result<(SalahiyatMalik, MiftahKhass), Box<dyn Error>> {
    let khass = MiftahKhass::min_bayt(&[7_u8; 32]);
    let salahiya = SalahiyatMalik::bi_miftah(&khass, &khass.aam().bayt())
        .ok_or("a key must prove itself against its own public half")?;
    Ok((salahiya, khass))
}

/// A bare repository standing in for the forge, addressed as a local path.
fn mustawda_baid(jidhr: &Path) -> Result<String, git2::Error> {
    let _ = git2::Repository::init_bare(jidhr)?;
    Ok(jidhr.display().to_string())
}

const fn talab<'a>(
    jidhr: &'a Path,
    manshurat: &'a Path,
    rabt: &'a str,
    waqt: &'a str,
) -> TalabNashrMustawda<'a> {
    TalabNashrMustawda {
        jidhr,
        mujallad_manshurat: manshurat,
        rabt_git: rabt,
        far: FAR,
        asas_rabt: ASAS,
        login: "cc1a2b",
        sirr: "",
        ism_musahim: "cc1a2b",
        barid: "cc1a2b@musahim.taarib.invalid",
        waqt,
    }
}

/// Reads one file out of the remote's `main`, which is the only honest way to
/// ask what another machine would fetch.
fn min_almustawda(rabt: &str, nisbi: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mustawda = git2::Repository::open(rabt)?;
    let raas = mustawda
        .find_reference(&format!("refs/heads/{FAR}"))?
        .peel_to_commit()?;
    let madkhal = raas.tree()?.get_path(Path::new(nisbi))?;
    Ok(madkhal
        .to_object(&mustawda)?
        .peel_to_blob()?
        .content()
        .to_vec())
}

/// The manifest sequence the remote is currently serving.
fn tasalsul_almustawda(rabt: &str) -> Result<u64, Box<dyn Error>> {
    let bayt = min_almustawda(rabt, "bayan.json")?;
    let bayan: serde_json::Value = serde_json::from_slice(&bayt)?;
    bayan
        .get("tasalsul")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "the manifest carries no sequence".into())
}

/// A publish run against `rabt`, with the ledger as given.
fn unshur_fi(
    sijill: &SijillNashr,
    jidhr: &Path,
    manshurat: &Path,
    rabt: &str,
    waqt: &str,
) -> Result<NatijatNashrMustawda, Box<dyn Error>> {
    let (salahiya, khass) = malik()?;
    let natija = unshur(
        &salahiya,
        &khass,
        sijill,
        &talab(jidhr, manshurat, rabt, waqt),
    )?;
    Ok(natija)
}

/// The first publish into a repository nobody has pushed to: the catalogue
/// lands, the manifest starts at one, and the signed revocation list is beside
/// it.
#[test]
fn awwal_nashr_yabni_almustawda_min_alsifr() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let nuskha = masrah.path().join("nuskha");
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let natija = unshur_fi(
        &SijillNashr::default(),
        &nuskha,
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;

    assert_eq!(natija.tasalsul, 1, "a fresh registry starts at one");
    assert_eq!(natija.far, FAR);
    assert_eq!(tasalsul_almustawda(&rabt)?, 1);
    // All 256 shards, not only the ones with content: a client asks for every
    // shard its games fall in and treats a missing one as a failed fetch.
    assert!(!min_almustawda(&rabt, "sharaih/00.json")?.is_empty());
    assert!(!min_almustawda(&rabt, "sharaih/ff.json")?.is_empty());
    assert!(!min_almustawda(&rabt, "sahb/qaima.json")?.is_empty());
    Ok(())
}

/// The sequence moves on every publish, and it is read back out of what the
/// registry is actually serving rather than counted locally. A client caches on
/// that number and will not look again until it changes.
#[test]
fn nashr_mutakarrir_yuharrik_altasalsul() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let nuskha = masrah.path().join("nuskha");
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let awwal = unshur_fi(
        &SijillNashr::default(),
        &nuskha,
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;
    let thani = unshur_fi(
        &SijillNashr::default(),
        &nuskha,
        &manshurat,
        &rabt,
        "2026-09-21T15:51:00Z",
    )?;

    assert_eq!(awwal.tasalsul, 1);
    assert_eq!(thani.tasalsul, 2, "the second cast is one past the first");
    assert_ne!(awwal.iltizam, thani.iltizam);
    assert_eq!(tasalsul_almustawda(&rabt)?, 2);
    Ok(())
}

/// A second machine — a fresh checkout that has never published — takes the
/// sequence from the registry rather than starting over. Starting over would
/// publish a manifest every client refuses as older than the one it holds.
#[test]
fn nuskha_jadeeda_taqra_altasalsul_min_almustawda() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let _ = unshur_fi(
        &SijillNashr::default(),
        &masrah.path().join("awwal"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;
    let thani = unshur_fi(
        &SijillNashr::default(),
        &masrah.path().join("thani"),
        &manshurat,
        &rabt,
        "2026-09-21T15:51:00Z",
    )?;

    assert_eq!(thani.tasalsul, 2);
    Ok(())
}

/// A registry that cannot be reached leaves nothing claiming to be published.
///
/// The ledger is taken by shared reference and the caller stamps it from the
/// result, so there is no path on which a run that failed here marks anything
/// published. The test holds that shape: the error names the step, the ledger
/// comes back identical, and no manifest exists anywhere.
#[test]
fn mustawda_la_yusal_ilayh_la_yunshar_shayan() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let ghayb = masrah.path().join("la-shay-huna");
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;
    let sijill = SijillNashr::default();
    let qabl = sijill.clone();

    let khata = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &ghayb.display().to_string(),
        "2026-09-20T15:51:00Z",
    )
    .err()
    .ok_or("an unreachable registry must not read as a publish")?;
    let matn = khata.to_string();
    assert!(matn.contains("fetching the registry"), "{matn}");

    assert_eq!(sijill, qabl, "a failed run writes nothing to the ledger");
    assert!(!ghayb.exists(), "and creates no registry of its own");
    Ok(())
}

/// A push the remote will not take is a failure, not a success with a warning.
///
/// `git2` reports a server-side rejection through an update callback and
/// returns `Ok` from `push` itself, so a caller that only checks the `Result`
/// would mark every submission published over a rejection. A stale lock on the
/// branch is the cheapest deterministic way to make the remote refuse the ref
/// while still answering the fetch.
#[test]
fn dafa_marfud_yarudd_khata_wa_la_yughayyir_almustawda() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let baid = masrah.path().join("baid");
    let rabt = mustawda_baid(&baid)?;
    std::fs::create_dir_all(baid.join("refs/heads"))?;
    std::fs::write(baid.join(format!("refs/heads/{FAR}.lock")), b"")?;

    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;
    let sijill = SijillNashr::default();
    let qabl = sijill.clone();

    let khata = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )
    .err()
    .ok_or("a refused push must not read as a publish")?;
    let matn = khata.to_string();
    assert!(matn.contains("pushing to the registry"), "{matn}");

    // The ledger is what the caller reads to decide which submissions are
    // published, and a failed run must leave it saying exactly what it said
    // before — which is why `unshur` takes it by shared reference.
    assert_eq!(sijill, qabl);
    // And the registry is untouched: no manifest, so nothing claims a sequence
    // that was never served.
    assert!(min_almustawda(&rabt, "bayan.json").is_err());
    Ok(())
}

/// Every approved package refused and nothing else to serve: the run stops
/// rather than pushing an empty catalogue at a fresh sequence number, which
/// would tell every client to refetch and then hand them nothing.
#[test]
fn rafd_alkul_yuqif_alnashr_wa_la_yunshar_fahras_farigh() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let mut sijill = SijillNashr::default();
    sijill.sajjil(madkhal(RuqaaId::jadeeda(), None)?);

    let khata = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )
    .err()
    .ok_or("an all-refused cast must not read as a publish")?;
    let matn = khata.to_string();
    assert!(
        matn.contains("every approved package was refused"),
        "{matn}"
    );
    assert!(min_almustawda(&rabt, "bayan.json").is_err());
    Ok(())
}

/// One unreadable package does not wedge the catalogue: it is skipped and
/// reported while it is only approved, and everything readable is published
/// around it.
#[test]
fn madkhal_bila_huzma_yurfad_wa_yamurr_albaqi() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let salim = RuqaaId::jadeeda();
    let murajaa = RuqaaRevision::jadeeda(1);
    let ism_malaf = format!("{salim}-r1-00000000.ruqaa");
    ikhtim_huzma(&manshurat.join(&ism_malaf), salim, murajaa)?;

    let ghaib = RuqaaId::jadeeda();
    let mut sijill = SijillNashr::default();
    sijill.sajjil(MadkhalNashr {
        ruqaa: salim,
        murajaa,
        luba: luba(),
        ism_luba: ISM_LUBA.to_owned(),
        musahim: musahim()?,
        ism_malaf,
        waqt_iaatimad: "2026-09-20T15:51:00Z".to_owned(),
        tasalsul: None,
        tajawuz: None,
    });
    sijill.sajjil(madkhal(ghaib, None)?);

    let natija = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;

    assert_eq!(natija.fahras.len(), 1);
    assert_eq!(natija.fahras.first().map(|listing| listing.id), Some(salim));
    assert_eq!(natija.marfuda.len(), 1);
    assert_eq!(
        natija.marfuda.first().map(|marfud| marfud.ruqaa),
        Some(ghaib)
    );
    Ok(())
}

/// The same entry, but already in the catalogue: skipping it would rebuild the
/// shards without its listing and delete a live one, so the whole run stops
/// instead.
#[test]
fn madkhal_manshur_la_yasqut_min_alfahras_bisamt() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let mut sijill = SijillNashr::default();
    sijill.sajjil(madkhal(RuqaaId::jadeeda(), Some(3))?);

    let khata = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )
    .err()
    .ok_or("dropping a published listing must not be silent")?;
    let matn = khata.to_string();
    assert!(matn.contains("already in the catalogue"), "{matn}");
    Ok(())
}

/// A third-party entry is refused by the gate until the owner records the
/// author's permission, and the seed ships with that statement empty.
#[test]
fn madkhal_khariji_bila_bayan_la_yadkhul_alsijill() -> NatijatIkhtibar {
    let mut sijill = SijillNashr::default();
    let khata = sijill
        .sajjil_khariji(badhrat_rtea())
        .err()
        .ok_or("an entry with no recorded permission must not reach the ledger")?;
    assert!(
        sabab_albawwaba(&khata)?
            .iter()
            .any(|mithal| mithal.contains("no written permission on record")),
        "{khata:?}"
    );
    assert_eq!(sijill.kharijiya().count(), 0);
    Ok(())
}

/// The sentences a shut gate gave, which is where the refusal actually is —
/// the error's own text is a count.
fn sabab_albawwaba(khata: &KhataTaqdeem) -> Result<&[String], Box<dyn Error>> {
    match khata {
        KhataTaqdeem::BawwabaMaghlaqa { amthila, .. } => Ok(amthila),
        akhar => Err(format!("the gate refused for another reason: {akhar}").into()),
    }
}

/// Mirroring needs its own grant. The entry publishes from its author with a
/// written permission on record, and asking the registry to serve the bytes is
/// refused until the author has said that too.
#[test]
fn mira_marfuda_illa_bi_idhn_almuallif() -> NatijatIkhtibar {
    let mut sijill = SijillNashr::default();

    let mut mamnua = khariji_bi_bayan();
    mamnua.mira = HalatMira::MinAlsijill {
        rabt: masar_mira(mamnua.id, "update.zip"),
    };
    let khata = sijill
        .sajjil_khariji(mamnua)
        .err()
        .ok_or("hosting somebody's bytes needs a grant that covers hosting")?;
    assert!(
        sabab_albawwaba(&khata)?
            .iter()
            .any(|mithal| mithal.contains("separate grants")),
        "{khata:?}"
    );

    // From the author, which is the default and what the seed ships as.
    sijill.sajjil_khariji(khariji_bi_bayan())?;
    assert_eq!(sijill.kharijiya().count(), 1);
    Ok(())
}

/// The whole path: a third-party entry in the ledger, cast, pushed, and read
/// back out of the remote the way a client reads it — the manifest first, then
/// the shard verified against the hash the manifest declares.
#[test]
fn madkhal_khariji_yasil_ila_shareeha_yaqrauha_alameel() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let madkhal = khariji_bi_bayan();
    let mut sijill = SijillNashr::default();
    sijill.sajjil_khariji(madkhal.clone())?;

    let natija = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;
    assert_eq!(natija.kharijiya.len(), 1);
    assert!(
        natija.fahras.is_empty(),
        "a third-party entry is not a patch Taarib built"
    );

    let bayan = BayanMustawda::min_bayt(&min_almustawda(&rabt, "bayan.json")?, None)?;
    let raqm = shareeha(madkhal.luba);
    let bayt = min_almustawda(&rabt, &masar_shareeha(raqm)?)?;
    // The client's own path: the bytes are hashed against the manifest before a
    // single record is parsed, so this is what another machine would really see.
    let shareeha = ShareehaMuwaththaqa::min_bayt(raqm, &bayt, &bayan)?;
    let qaima = shareeha.kharijiya(madkhal.luba);
    assert_eq!(qaima, &[madkhal.clone()][..]);
    assert!(shareeha.ruqaa(madkhal.luba).is_empty());

    let awwal = qaima.first().ok_or("the shard carries no entry")?;
    assert_eq!(awwal.nasab(), "Emad Adel · Redemption Team");
    assert_eq!(awwal.qitaa_li_bina("1491").len(), 1);
    assert_eq!(awwal.qitaa_li_bina("1311").len(), 2);

    // The build declaration, byte for byte, out of the document another machine
    // would really fetch. A probe that does not survive this trip resolves to
    // `Majhula` on every install, which is exactly the state this feature
    // exists to leave — so the journey is the test, not the struct literal.
    assert_eq!(awwal.tahdid_bina, madkhal.tahdid_bina);
    assert_eq!(
        awwal.tahdid_bina.faps,
        Some(FapsBina::MawridIsdar {
            masar: "RDR2.exe".to_owned(),
            juz: 2,
        })
    );
    assert!(
        awwal.tahdid_bina.tanazur.is_empty(),
        "the entry declares no launcher correspondence and the catalogue must not invent one"
    );
    Ok(())
}

/// An entry that declares no way to tell builds apart is published, reaches the
/// shard, and reads back declaring nothing.
///
/// The ordinary third-party entry, and the one a gate written too tightly would
/// delete from every catalogue: its author never related their numbering to a
/// launcher's, which is not a malformed declaration but the absence of one.
#[test]
fn madkhal_khariji_bila_tahdid_bina_yunshar() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let mut madkhal = khariji_bi_bayan();
    madkhal.tahdid_bina = TahdidBina::default();
    let mut sijill = SijillNashr::default();
    sijill.sajjil_khariji(madkhal.clone())?;

    let natija = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;
    assert_eq!(natija.kharijiya.len(), 1);

    let bayan = BayanMustawda::min_bayt(&min_almustawda(&rabt, "bayan.json")?, None)?;
    let raqm = shareeha(madkhal.luba);
    let bayt = min_almustawda(&rabt, &masar_shareeha(raqm)?)?;
    let shareeha = ShareehaMuwaththaqa::min_bayt(raqm, &bayt, &bayan)?;
    let awwal = shareeha
        .kharijiya(madkhal.luba)
        .first()
        .ok_or("the shard carries no entry")?;
    assert_eq!(awwal.tahdid_bina, TahdidBina::default());
    Ok(())
}

/// A probe aimed outside the game never reaches a catalogue.
///
/// The path is joined onto the player's game directory and read there, so this
/// is the last machine that can refuse it — after the push it is a signed
/// document telling every client which file to open.
#[test]
fn faps_kharij_alluba_la_yasil_ila_shareeha() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let mut madkhal = khariji_bi_bayan();
    madkhal.tahdid_bina.faps = Some(FapsBina::MawridIsdar {
        masar: "../../../etc/passwd".to_owned(),
        juz: 2,
    });
    let mut sijill = SijillNashr::default();
    sijill.sajjil_khariji(madkhal)?;

    let khata = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )
    .err()
    .ok_or("a probe that leaves the game must not be published")?;
    let matn = khata.to_string();
    assert!(
        matn.contains("every approved package was refused"),
        "{matn}"
    );
    assert!(
        min_almustawda(&rabt, "bayan.json").is_err(),
        "nothing is served when the only entry was refused"
    );
    Ok(())
}

/// The RTEA seed with the one fact nobody here can invent filled in.
///
/// The statement is a fixture's, not a claim about the real author: the seed
/// ships blank precisely so that the owner has to go and ask.
fn khariji_bi_bayan() -> RuqaaKharijiya {
    let mut madkhal = badhrat_rtea();
    madkhal.masdar.idhn = IdhnMasdar::Katabi {
        bayan: "fixture only — no permission has actually been asked for".to_owned(),
    };
    madkhal
}

/// One ledger row for a package that is not on disk.
fn madkhal(ruqaa: RuqaaId, tasalsul: Option<u64>) -> Result<MadkhalNashr, Box<dyn Error>> {
    Ok(MadkhalNashr {
        ruqaa,
        murajaa: RuqaaRevision::jadeeda(1),
        luba: luba(),
        ism_luba: ISM_LUBA.to_owned(),
        musahim: musahim()?,
        ism_malaf: format!("{ruqaa}-r1-00000000.ruqaa"),
        waqt_iaatimad: "2026-09-20T15:51:00Z".to_owned(),
        tasalsul,
        tajawuz: None,
    })
}

/// The contributor every fixture credits. A fingerprint this test file made up,
/// which is all a listing's `musahim` ever is.
fn musahim() -> Result<MusahimId, Box<dyn Error>> {
    Ok(MusahimId::jadeed("22".repeat(32))?)
}

/// The whole point of the two-step publish: after a successful run the
/// catalogue another machine reads carries this patch, and the address in its
/// listing is an `https` URL that resolves to the asset the same push put in
/// the tree.
///
/// This is the defect the change exists to close. The listing used to carry a
/// path on the owner's own disk, which nobody else could fetch — and nothing
/// checked, because nothing was ever published.
#[test]
fn huzma_manshura_tahmil_unwan_tanzil_haqiqi() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let rabt = mustawda_baid(&masrah.path().join("baid"))?;
    let manshurat = masrah.path().join("manshurat");
    std::fs::create_dir_all(&manshurat)?;

    let ruqaa = RuqaaId::jadeeda();
    let murajaa = RuqaaRevision::jadeeda(1);
    let ism_malaf = format!("{ruqaa}-r1-00000000.ruqaa");
    ikhtim_huzma(&manshurat.join(&ism_malaf), ruqaa, murajaa)?;

    let mut sijill = SijillNashr::default();
    sijill.sajjil(MadkhalNashr {
        ruqaa,
        murajaa,
        luba: luba(),
        ism_luba: ISM_LUBA.to_owned(),
        musahim: musahim()?,
        ism_malaf,
        waqt_iaatimad: "2026-09-20T15:51:00Z".to_owned(),
        tasalsul: None,
        tajawuz: None,
    });

    let natija = unshur_fi(
        &sijill,
        &masrah.path().join("nuskha"),
        &manshurat,
        &rabt,
        "2026-09-20T15:51:00Z",
    )?;

    assert!(natija.marfuda.is_empty(), "{:?}", natija.marfuda);
    let mulakhkhas = natija.fahras.first().ok_or("the cast published nothing")?;
    let mutawaqqa = format!("{ASAS}/isdar/{ruqaa}/r-e-p-o-r1.ruqaa");
    assert_eq!(mulakhkhas.rabt, mutawaqqa);

    // Not only what the cast returned: what the registry is now serving. The
    // shard is the document a client actually reads.
    let raqm = shareeha(luba());
    let bayt = min_almustawda(&rabt, &format!("sharaih/{raqm:02x}.json"))?;
    let muhtawa: serde_json::Value = serde_json::from_slice(&bayt)?;
    let mansur = muhtawa
        .get("ruqaa")
        .and_then(|ruqa| ruqa.get(luba().to_string()))
        .and_then(|qaima| qaima.get(0))
        .ok_or("the shard carries no listing for this game")?;
    assert_eq!(
        mansur.get("rabt").and_then(serde_json::Value::as_str),
        Some(mutawaqqa.as_str())
    );

    // And the address resolves against the tree the same push wrote: the
    // repository path under the registry root is the asset itself.
    let asl = min_almustawda(&rabt, &format!("isdar/{ruqaa}/r-e-p-o-r1.ruqaa"))?;
    assert!(
        !asl.is_empty(),
        "the listing points at an asset that exists"
    );
    Ok(())
}

/// The game every fixture targets.
const ISM_LUBA: &str = "R.E.P.O.";

const fn luba() -> LubaId {
    LubaId::min_uuid(uuid::Uuid::from_u128(
        0x0192_6d42_dc5d_74b2_b27d_5c44_41a0_3a3f,
    ))
}

/// A sealed package, built through the container's own writer and signed with
/// the throwaway owner key through the one function allowed to sign.
///
/// The manifest carries exactly the fields a cast reads out of it; every one of
/// them is serialized from the real type rather than spelled as JSON, so a
/// change to the wire form breaks this here instead of in a catalogue.
fn ikhtim_huzma(
    masar: &Path,
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
) -> Result<(), Box<dyn Error>> {
    use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
    use taarib_mustalahat::ruqaa::{RukhsaRuqaa, TareeqaTarjama};
    use taarib_mustalahat::taghtiya::Taghtiya;

    let taghtiya = Taghtiya {
        majmu: 2,
        mutarjam: 2,
        muakkad: 2,
        majmu_takrar: 2,
        mutarjam_takrar: 2,
        majmu_awwal: 2,
        mutarjam_awwal: 2,
    };
    let bayan = serde_json::json!({
        "isdar": 1,
        "id": ruqaa,
        "murajaa": murajaa,
        "irtibat": {
            "manassat": ["steam:480"],
            "basmat": ["1111111111111111111111111111111111111111111111111111111111111111"],
            "adad_malaffat": 1,
            "mukhattat": { "isdar": 1, "hawiyat": ["luba.exe"] },
            "nitaq": null,
            "khutut": []
        },
        "wasf": {
            "unwan": "تعريب R.E.P.O.",
            "ism_musahim": "cc1a2b",
            "rukhsa": RukhsaRuqaa::Cc0,
            "tareeqa": TareeqaTarjama::BashariyaKamila
        },
        "muharrik": {
            "aila": AilatMuharrik::Unity,
            "khalfiya": KhalfiyaBarmajiya::Majhula,
            "tabaqa": Tabaqa::TarjamaFawqiya
        },
        "taghtiya": { "kulli": taghtiya, "qabila_lil_nashr": true, "asbab": [] }
    });

    let mut katib = Katib::jadeed();
    let _ = katib.bayan(&serde_json::to_vec(&bayan)?);
    let _ = katib.nass("Start Game", "ابدأ اللعب")?;
    let _ = katib.nass("Options", "الخيارات")?;
    let bayt = katib.ikhtim()?;

    let (salahiya, khass) = malik()?;
    let musahim = MusahimId::jadeed(hex::encode(khass.aam().bayt()))?;
    let makhtuma = waqqi(
        &salahiya,
        &khass,
        musahim,
        "2026-09-20T15:51:00Z".to_owned(),
        1_790_000_000,
        ruqaa,
        murajaa,
        luba(),
        bayt,
    )?;
    std::fs::write(masar, makhtuma.bayt())?;
    Ok(())
}
