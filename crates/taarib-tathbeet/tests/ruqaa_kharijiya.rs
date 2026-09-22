//! A patch Taarib did not build, installed and taken back out again.
//!
//! The shape is RTEA's, measured: a Red Dead Redemption 2 translation delivered
//! as a zip of loader DLLs and a `lml/` tree of language databases, whose own
//! HOW-TO tells the user to delete `version.dll` — the name Taarib's loader is
//! published as — and a list of other files before installing.
//!
//! The load-bearing test in this file is
//! [`al_izala_tuid_al_shajara_bayt_bi_bayt`]. Everything else here guards a way
//! in; that one guards the reason any of it exists. A user chooses Taarib over
//! the author's own installer for exactly one reason — that Taarib can undo it —
//! and if the uninstall does not return the game tree byte for byte, including
//! the files the author's instructions said to delete, then this feature is
//! worth nothing and should not ship.
//!
//! Nothing here reaches a network. The transport is a table of addresses to
//! bodies, which is the whole reason the transfer is a port rather than an HTTP
//! client wired into the installer.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use taarib_aman::fahs_khariji::{NatijatFahsKhariji, RafdKhariji, TalabFahsKhariji, fahs_khariji};
use taarib_aman::idhn::IdhnTathbeetKhariji;
use taarib_aman::iqrar::{ISDAR_NASS, SijillIqrar};
use taarib_mustalahat::khariji::{
    HalatMira, QitaatTanzeel, RuqaaKharijiya, TahdheerKhariji, TakhtitKhariji,
};
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{IdhnMasdar, MasdarKhariji, RukhsaRuqaa, RuqaaId, RuqaaRevision};
use taarib_tathbeet::bayan::{Muthabbit as _, NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::jalb_khariji::NaqilKhariji;
use taarib_tathbeet::khariji::{
    TalabTathbeetKhariji, azil_khariji, jidhr_nusakh_khariji, kharijiyat_mathbita, thabbit_khariji,
};
use taarib_tathbeet::khata::{KhataTathbeet, NatijatTathbeet};
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, istiada_nass};
use taarib_tathbeet::tasadum::{JihatTasadum, la_yatasadam_maa_khariji};
use taarib_usus::khata::{Khutwa, Tafsir as _};

type Natija = Result<(), Box<dyn std::error::Error>>;

/// The game executable's name. No process anywhere carries it, so the
/// running-game guard clears on any machine this runs on.
const TANFIDHI: &str = "la-tujad-hadhihi-al-luba-abadan.exe";

const RABT_UPDATE: &str = "https://rt.emadadeldev.workers.dev/?lml-update";

// ---------------------------------------------------------------------------
// The fixture game, the fixture archive, and the fixture transport
// ---------------------------------------------------------------------------

/// The game as it is before anything touches it.
///
/// `GameNetworkingSockets.dll` is there so the network scan finds an online
/// mode and the gate composes its own Red Dead Online warning; the rest is the
/// shape of the real thing, including the three paths the author's HOW-TO says
/// to delete. `lml/` is deliberately **absent**: a game that already has one is
/// a game with a translation in it, and the collision refusal would — correctly
/// — stop the install before it began.
const LUBA_ASLIYA: [(&str, &[u8]); 6] = [
    ("RDR2.exe", b"MZ the game's own executable"),
    ("GameNetworkingSockets.dll", b"MZ networking"),
    ("x64/data.rpf", b"the game's own archive, untouched"),
    ("version.dll", b"an earlier proxy the HOW-TO says to delete"),
    ("ScriptHookRDR2.log", b"a log the HOW-TO says to delete"),
    (
        "mod backup/keep.bin",
        b"inside a directory the HOW-TO says to delete",
    ),
];

/// What the patch's archive writes into the game root.
const ARSHIF_UPDATE: [(&str, &[u8]); 7] = [
    ("dinput8.dll", b"the patch's own loader"),
    ("lml.ini", b"[lml]\nenabled=1\n"),
    ("ModManager.Core.dll", b"mod manager"),
    ("ScriptHookRDR2.dll", b"script hook"),
    ("vfs.asi", b"virtual file system"),
    (
        "lml/RTEA/Subtitles/texts/a.yldb",
        b"the first language database",
    ),
    (
        "lml/RTEA/Subtitles/texts/b.yldb",
        b"the second language database",
    ),
];

/// A transport that serves bodies from a table instead of from the internet.
#[derive(Debug, Default)]
struct NaqilThabit {
    mawad: BTreeMap<String, Vec<u8>>,
    talabat: Vec<String>,
}

impl NaqilKhariji for NaqilThabit {
    fn ijlib(&mut self, ism: &str, rabt: &str, hadaf: &Path) -> NatijatTathbeet<()> {
        self.talabat.push(rabt.to_owned());
        let Some(bayt) = self.mawad.get(rabt) else {
            return Err(KhataTathbeet::TanzeelMutaadhdhir {
                ism: ism.to_owned(),
                rabt: rabt.to_owned(),
                sabab: "no such address in the fixture".to_owned(),
            });
        };
        std::fs::write(hadaf, bayt).map_err(|sabab| KhataTathbeet::KhataMalaf {
            masar: hadaf.to_path_buf(),
            amal: "writing a fixture body",
            sabab,
        })
    }
}

/// Builds a zip in memory from a list of entries.
fn izim(madakhil: &[(&str, &[u8])]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut katib = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let khiyarat =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (ism, bayt) in madakhil {
        katib.start_file(*ism, khiyarat)?;
        katib.write_all(bayt)?;
    }
    Ok(katib.finish()?.into_inner())
}

fn sha256(bayt: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(bayt))
}

/// Writes the fixture game tree.
fn ibni_luba(jidhr: &Path) -> Natija {
    for (nisbi, bayt) in LUBA_ASLIYA {
        let masar = jidhr.join(nisbi);
        if let Some(walid) = masar.parent() {
            std::fs::create_dir_all(walid)?;
        }
        std::fs::write(masar, bayt)?;
    }
    Ok(())
}

/// Every path under a root with its content fingerprint, for a before/after
/// comparison.
///
/// Directories are in the map too, and that is not padding: an uninstall that
/// restored every byte and left `lml/` standing has not given the game back as
/// it was, and a comparison over files alone would call that a pass.
fn lamha(jidhr: &Path) -> BTreeMap<String, String> {
    let mut lamha = BTreeMap::new();
    for madkhal in walkdir::WalkDir::new(jidhr)
        .min_depth(1)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .flatten()
    {
        let Ok(nisbi) = madkhal.path().strip_prefix(jidhr) else {
            continue;
        };
        let miftah = nisbi.to_string_lossy().replace('\\', "/");
        let qeema = if madkhal.file_type().is_dir() {
            "<a directory>".to_owned()
        } else {
            std::fs::read(madkhal.path()).map_or_else(
                |khata| format!("<unreadable: {khata}>"),
                |bayt| blake3::hash(&bayt).to_hex().to_string(),
            )
        };
        let _ = lamha.insert(miftah, qeema);
    }
    lamha
}

// ---------------------------------------------------------------------------
// The catalogue entry and the permit
// ---------------------------------------------------------------------------

fn madkhal(bayt_update: &[u8]) -> RuqaaKharijiya {
    RuqaaKharijiya {
        id: RuqaaId::jadeeda(),
        unwan: "RTEA".to_owned(),
        luba: luba_id(),
        hawiyat_manassa: vec!["Red Dead Redemption 2".to_owned()],
        abniya: vec!["1491".to_owned()],
        masdar: MasdarKhariji {
            ism: "Emad Adel".to_owned(),
            rabt: "https://github.com/emadadeldev/rtea".to_owned(),
            rukhsa: RukhsaRuqaa::MilkiyaKhassa,
            idhn: IdhnMasdar::Katabi {
                bayan: "granted on X, 2026-09-21, https://x.example/post/1".to_owned(),
            },
            yasmah_bilmira: false,
        },
        fariq: Some("Redemption Team".to_owned()),
        isdar: "1.7".to_owned(),
        qitaa: vec![QitaatTanzeel {
            ism: "update.zip".to_owned(),
            rabt: RABT_UPDATE.to_owned(),
            hajm: bayt_update.len() as u64,
            sha256: sha256(bayt_update),
            abniya: Vec::new(),
        }],
        takhtit: TakhtitKhariji {
            yaktub: vec![
                "dinput8.dll".to_owned(),
                "lml".to_owned(),
                "lml.ini".to_owned(),
                "ModManager.Core.dll".to_owned(),
                "ScriptHookRDR2.dll".to_owned(),
                "vfs.asi".to_owned(),
            ],
            yahdhif: vec![
                "version.dll".to_owned(),
                "ScriptHookRDR2.log".to_owned(),
                "mod backup".to_owned(),
            ],
        },
        tahdheerat: vec![TahdheerKhariji {
            arabi: "طور القصّة فقط.".to_owned(),
            injilizi: "Story mode only.".to_owned(),
        }],
        mira: HalatMira::MinAlmuallif,
        rabt_safha: None,
        rabt_tahdith: None,
    }
}

fn luba_id() -> LubaId {
    LubaId::min_masdar(&MasdarLuba::Steam(1_174_180), "Red Dead Redemption 2")
}

fn tarif_luba(jidhr_luba: &Path, ruqaa: RuqaaId) -> TarifLuba {
    TarifLuba {
        luba: luba_id(),
        masdar: MasdarLuba::Steam(1_174_180),
        ism: "Red Dead Redemption 2".to_owned(),
        jidhr: jidhr_luba.to_path_buf(),
        ruqaa,
        murajaa: RuqaaRevision::AWWAL,
        basma_bina: None,
    }
}

fn sijill_iqrar() -> SijillIqrar {
    SijillIqrar {
        isdar_nass: ISDAR_NASS,
        waqt: "2026-09-22T00:00:00Z".to_owned(),
        isdar_taarib: env!("CARGO_PKG_VERSION").to_owned(),
        lugha_nass: None,
    }
}

/// Puts one entry through the real gate and returns what it minted.
fn bawwaba(
    jidhr_luba: &Path,
    ruqaa: &RuqaaKharijiya,
    iqrar_tahdheerat: bool,
) -> NatijatFahsKhariji {
    let iqrar = sijill_iqrar();
    fahs_khariji(&TalabFahsKhariji {
        luba: luba_id(),
        ism_luba: "Red Dead Redemption 2",
        jidhr_luba,
        appid: None,
        jidhr_steam: None,
        bina: "1491",
        ruqaa,
        iqrar: Some(&iqrar),
        iqrar_tahdheerat,
    })
}

fn idhn(jidhr_luba: &Path, ruqaa: &RuqaaKharijiya) -> Result<IdhnTathbeetKhariji, String> {
    match bawwaba(jidhr_luba, ruqaa, true) {
        NatijatFahsKhariji::Masmuh(idhn) => Ok(idhn),
        NatijatFahsKhariji::Marfud(rafd) => Err(rafd.injilizi()),
    }
}

/// One whole staged run: a temporary root, a game in it, an entry, and a
/// transport that serves the entry's one artifact.
struct Masrah {
    _mujallad: tempfile::TempDir,
    jidhr: PathBuf,
    luba: PathBuf,
    nusakh: PathBuf,
    tajmee: PathBuf,
    ruqaa: RuqaaKharijiya,
}

impl Masrah {
    fn jadeed(
        madakhil: &[(&str, &[u8])],
    ) -> Result<(Self, NaqilThabit), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let jidhr = mujallad.path().to_path_buf();
        let luba = jidhr.join("luba");
        let nusakh = jidhr.join("nusakh");
        let tajmee = jidhr.join("tajmee");
        std::fs::create_dir_all(&luba)?;
        std::fs::create_dir_all(&nusakh)?;
        ibni_luba(&luba)?;

        let bayt = izim(madakhil)?;
        let ruqaa = madkhal(&bayt);
        let mut naqil = NaqilThabit::default();
        let _ = naqil.mawad.insert(RABT_UPDATE.to_owned(), bayt);

        Ok((
            Self {
                _mujallad: mujallad,
                jidhr,
                luba,
                nusakh,
                tajmee,
                ruqaa,
            },
            naqil,
        ))
    }

    fn talab<'a>(&'a self, tarif: &'a TarifLuba) -> TalabTathbeetKhariji<'a> {
        TalabTathbeetKhariji {
            luba: tarif,
            ruqaa: &self.ruqaa,
            tanfidhi: TANFIDHI,
            jidhr_nusakh: &self.nusakh,
            jidhr_tajmee: &self.tajmee,
        }
    }
}

// ---------------------------------------------------------------------------
// The acceptance tests
// ---------------------------------------------------------------------------

/// The one that decides whether the feature ships.
///
/// Fingerprint every path in the game, install, prove the game really changed,
/// uninstall, and require the fingerprints to be identical — including the three
/// paths the author's own instructions ordered deleted, which their own
/// installer removes and cannot put back.
#[test]
fn al_izala_tuid_al_shajara_bayt_bi_bayt() -> Natija {
    let (masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let qabl = lamha(&masrah.luba);
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;

    let natija = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;
    assert_eq!(natija.adad_maktub, ARSHIF_UPDATE.len());
    assert_eq!(
        natija.adad_mahdhuf, 3,
        "version.dll, the log, and the one file inside `mod backup`"
    );

    // The install really happened: the patch's files are in, and the three the
    // author's instructions name are out.
    let baad = lamha(&masrah.luba);
    assert_ne!(qabl, baad);
    assert!(
        masrah
            .luba
            .join("lml/RTEA/Subtitles/texts/a.yldb")
            .is_file()
    );
    assert!(masrah.luba.join("dinput8.dll").is_file());
    assert!(!masrah.luba.join("version.dll").exists());
    assert!(!masrah.luba.join("ScriptHookRDR2.log").exists());
    assert!(!masrah.luba.join("mod backup/keep.bin").exists());

    // Staging is not left behind, and nothing of the artifact survives in it.
    assert!(
        !masrah
            .tajmee
            .join(masrah.ruqaa.id.uuid().as_simple().to_string())
            .exists()
    );

    let taqreer = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Muhafiza,
    )?;
    assert!(taqreer.mustabdala.is_empty());
    assert!(
        taqreer.baqaya.is_empty(),
        "nothing of the patch is left inside a directory it created"
    );
    assert_eq!(
        taqreer.mustaada, 3,
        "the three the remove-list took out are written back and verified"
    );

    assert_eq!(
        lamha(&masrah.luba),
        qabl,
        "the uninstall did not return the game tree byte for byte"
    );
    Ok(())
}

/// A pin that does not match is refused by name, and the game is not opened.
#[test]
fn basma_ghayr_mutabiqa_turfad_bil_ism_wa_la_taktub_shayan() -> Natija {
    let (masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    // The registry pins one release and the endpoint now serves another: same
    // name, different bytes, and a size the entry's pin no longer describes.
    let mughayyar = izim(&[("dinput8.dll", b"a different release entirely")])?;
    let _ = naqil.mawad.insert(RABT_UPDATE.to_owned(), mughayyar);

    let qabl = lamha(&masrah.luba);
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let khata = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil).err();

    let musamma = match &khata {
        Some(
            KhataTathbeet::HajmQitaaGhayrMutabiq { ism, .. }
            | KhataTathbeet::BasmatQitaaGhayrMutabiqa { ism, .. },
        ) => Some(ism.clone()),
        _ => None,
    };
    assert_eq!(
        musamma,
        Some("update.zip".to_owned()),
        "the refusal names the artifact that disagreed, got {khata:?}"
    );

    assert_eq!(lamha(&masrah.luba), qabl, "nothing reached the game");
    assert!(
        kharijiyat_mathbita(&masrah.nusakh).is_empty(),
        "no manifest was opened, because verification runs before the rollback point"
    );
    assert!(
        !masrah
            .tajmee
            .join(masrah.ruqaa.id.uuid().as_simple().to_string())
            .exists(),
        "the copy that disagreed is not left in staging for a later run to resume from"
    );
    Ok(())
}

/// A pin whose size matches and whose digest does not is the case a size-only
/// check waves through, so it gets its own proof.
#[test]
fn basma_mukhtalifa_bi_nafs_al_hajm_turfad() -> Natija {
    let (masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let mut mughayyar = izim(&ARSHIF_UPDATE)?;
    // The zip's first local header byte, flipped: the same length, different
    // bytes, and no longer a readable archive either — which the digest catches
    // first and must catch first.
    if let Some(awwal) = mughayyar.first_mut() {
        *awwal ^= 0xff;
    }
    let _ = naqil.mawad.insert(RABT_UPDATE.to_owned(), mughayyar);

    let qabl = lamha(&masrah.luba);
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let khata = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil).err();
    assert!(
        matches!(khata, Some(KhataTathbeet::BasmatQitaaGhayrMutabiqa { .. })),
        "a matching size does not excuse a digest that disagrees"
    );
    assert_eq!(lamha(&masrah.luba), qabl);
    Ok(())
}

/// An archive entry that tries to leave the game root is refused.
#[test]
fn madkhal_yatasarrab_min_jidhr_al_luba_marfud() -> Natija {
    let mut madakhil = ARSHIF_UPDATE.to_vec();
    madakhil.push(("../evil.dll", b"outside the game entirely"));
    let (masrah, mut naqil) = Masrah::jadeed(&madakhil)?;

    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let khata = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil).err();

    let madkhal = match &khata {
        Some(KhataTathbeet::MadkhalKharijAlTakhtit { madkhal, .. }) => Some(madkhal.clone()),
        _ => None,
    };
    assert_eq!(
        madkhal,
        Some("../evil.dll".to_owned()),
        "the refusal names the entry, got {khata:?}"
    );
    assert!(
        !masrah.jidhr.join("evil.dll").exists(),
        "nothing was written beside the game directory"
    );
    Ok(())
}

/// Inside the game root is not enough: the entry declares what it writes.
#[test]
fn madkhal_kharij_al_takhtit_marfud() -> Natija {
    let mut madakhil = ARSHIF_UPDATE.to_vec();
    madakhil.push((
        "redteamassets/extra.bin",
        b"a path the entry never declared",
    ));
    let (masrah, mut naqil) = Masrah::jadeed(&madakhil)?;

    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let khata = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil).err();
    assert!(matches!(
        khata,
        Some(KhataTathbeet::MadkhalKharijAlTakhtit { .. })
    ));
    assert!(!masrah.luba.join("redteamassets").exists());
    Ok(())
}

/// Both directions, each naming what has to go first.
#[test]
fn al_tasadum_marfud_fi_al_ittijahayn() -> Natija {
    // A third-party loader is already there and Taarib's own install refuses.
    let (masrah, _naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    std::fs::write(masrah.luba.join("vfs.asi"), b"somebody else's loader")?;
    let khata = la_yatasadam_maa_khariji(&masrah.luba, &masrah.nusakh).err();
    let jiha = match &khata {
        Some(KhataTathbeet::TasadumRuqaa { jiha, alamat, .. }) => {
            assert!(alamat.iter().any(|alama| alama == "vfs.asi"));
            Some(*jiha)
        },
        _ => None,
    };
    assert_eq!(jiha, Some(JihatTasadum::Khariji));
    // Refusing is half of it: the message has to name what was found and say
    // which one has to go, in both languages.
    if let Some(khata) = &khata {
        assert!(khata.injilizi().contains("vfs.asi"));
        assert!(khata.injilizi().contains("Remove it first"));
        assert!(khata.arabi().contains("vfs.asi"));
        assert!(khata.arabi().contains("أزِلها أوّلًا"));
        // Somebody else's mod is not this product's to delete for them.
        assert_eq!(khata.khutwa(), Khutwa::LaShay);
    }

    // Taarib's own patch is already there and the third-party install refuses.
    let (thani, mut naqil_thani) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let tarif_taarib = tarif_luba(&thani.luba, RuqaaId::jadeeda());
    let mut taarib = Tathbeet::ibda(
        &thani.nusakh,
        NawTathbeet::Nass,
        &tarif_taarib,
        "taarib-nafsuh",
    )?;
    taarib.ansha(
        &thani.luba.join("taarib/nusus.ruqaa"),
        b"Taarib's own package",
    )?;
    drop(taarib);

    let tarif = tarif_luba(&thani.luba, thani.ruqaa.id);
    let idhn = idhn(&thani.luba, &thani.ruqaa)?;
    let khata = thabbit_khariji(&thani.talab(&tarif), idhn, &mut naqil_thani).err();
    let jiha = match &khata {
        Some(KhataTathbeet::TasadumRuqaa { jiha, .. }) => Some(*jiha),
        _ => None,
    };
    assert_eq!(jiha, Some(JihatTasadum::Taarib), "got {khata:?}");
    if let Some(khata) = &khata {
        assert!(
            khata
                .injilizi()
                .contains("Uninstall the Taarib patch first")
        );
        assert!(khata.arabi().contains("أزِل رقعة تعريب أوّلًا"));
        // This side does have a button, and it is Taarib's own.
        assert_eq!(khata.khutwa(), Khutwa::IlghaTathbeet);
    }
    assert!(
        kharijiyat_mathbita(&thani.nusakh).is_empty(),
        "the refusal happened before any manifest was opened"
    );
    Ok(())
}

/// A zip entry that is a symbolic link is refused before it is expanded.
///
/// A link's content is a path, so extracting one is a write to wherever that
/// path leads — and the target is chosen by whoever packed the archive.
#[test]
fn madkhal_rabt_ramzi_marfud() -> Natija {
    let mut katib = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let khiyarat =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (ism, bayt) in ARSHIF_UPDATE {
        katib.start_file(ism, khiyarat)?;
        katib.write_all(bayt)?;
    }
    katib.add_symlink("lml/escape.yldb", "../../../../etc/passwd", khiyarat)?;
    let bayt = katib.finish()?.into_inner();

    let (mut masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    masrah.ruqaa = madkhal(&bayt);
    let _ = naqil.mawad.insert(RABT_UPDATE.to_owned(), bayt);

    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let khata = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil).err();
    let sabab = match &khata {
        Some(KhataTathbeet::MadkhalKharijAlTakhtit { sabab, .. }) => Some(sabab.clone()),
        _ => None,
    };
    assert!(
        sabab.is_some_and(|sabab| sabab.contains("symbolic link")),
        "got {khata:?}"
    );
    assert!(!masrah.luba.join("lml/escape.yldb").exists());
    Ok(())
}

/// Two installations in one game, and neither uninstall touches the other.
#[test]
fn izalat_wahida_la_tamiss_malaffat_al_ukhra() -> Natija {
    let (masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let qabl = lamha(&masrah.luba);

    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let _ = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;

    // Taarib's own install goes in afterwards, through the recorder directly:
    // the gate would refuse it now, and what this test is about is the two
    // uninstalls rather than the gate.
    let tarif_taarib = tarif_luba(&masrah.luba, RuqaaId::jadeeda());
    let mut taarib = Tathbeet::ibda(
        &masrah.nusakh,
        NawTathbeet::Nass,
        &tarif_taarib,
        "taarib-nafsuh",
    )?;
    taarib.ansha(
        &masrah.luba.join("taarib/nusus.ruqaa"),
        b"Taarib's own package",
    )?;
    drop(taarib);

    // Taarib's own uninstall first. It must leave every one of the third-party
    // patch's files alone — and must not put back the file the third party's
    // remove-list took out, because that original belongs to the other manifest.
    let taarib_taqreer = istiada_nass(
        &masrah.luba,
        &masrah.nusakh,
        SiyasatIstiada::Muhafiza,
        &mut RadLaShay,
    )?;
    assert_eq!(taarib_taqreer.mahdhufa, 1);
    assert!(!masrah.luba.join("taarib").exists());
    assert!(masrah.luba.join("dinput8.dll").is_file());
    assert!(
        masrah
            .luba
            .join("lml/RTEA/Subtitles/texts/a.yldb")
            .is_file()
    );
    assert!(
        !masrah.luba.join("version.dll").exists(),
        "the other manifest's original is not Taarib's own uninstall to restore"
    );

    // And now the third-party one, which finds its own manifest exactly as it
    // left it and returns the game to where it started.
    let _ = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Muhafiza,
    )?;
    assert_eq!(lamha(&masrah.luba), qabl);
    Ok(())
}

/// The gate mints nothing until the online sentence it writes itself has been
/// put in front of the person.
#[test]
fn al_bawwaba_tahmil_tahdheer_al_laab_al_shabaki() -> Natija {
    let (masrah, _naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;

    let marfud = bawwaba(&masrah.luba, &masrah.ruqaa, false);
    let (shabaki, qaima) = match marfud {
        NatijatFahsKhariji::Marfud(rafd) => match *rafd {
            RafdKhariji::TahdheeratGhayrMuqarra {
                tahdheerat,
                shabaki,
            } => (shabaki, tahdheerat),
            akhar => {
                return Err(format!("expected a warnings refusal, got {akhar:?}").into());
            },
        },
        NatijatFahsKhariji::Masmuh(_) => {
            return Err("the gate minted a permit with nothing acknowledged".into());
        },
    };
    assert!(shabaki, "the fixture ships a networking library");
    assert!(
        qaima
            .iter()
            .any(|tahdheer| tahdheer.injilizi.contains("story mode only")),
        "the entry's own warning is in the list"
    );
    assert!(
        qaima
            .iter()
            .any(|tahdheer| tahdheer.injilizi.contains("puts your account at risk")),
        "the gate's own online sentence is in the list"
    );
    assert!(
        qaima
            .iter()
            .any(|tahdheer| tahdheer.arabi.contains("يعرّض حسابك للحظر")),
        "and in Arabic, which is the rendering most readers see"
    );

    // Acknowledged, the same list is what the permit carries — so the install
    // records what the person was actually shown rather than a sentence
    // reconstructed afterwards.
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    assert_eq!(idhn.tahdheerat().len(), qaima.len());
    assert!(
        idhn.tahdheerat()
            .iter()
            .any(|tahdheer| tahdheer.injilizi.contains("puts your account at risk"))
    );
    Ok(())
}

/// The permit is the authority, not the entry it was handed beside.
#[test]
fn madkhal_muaddal_baad_al_bawwaba_la_yuthabbat() -> Natija {
    let (mut masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;

    // The gate approved this artifact at this digest; the entry now pins
    // another. Fetching against the edited entry is precisely what the permit
    // exists to stop.
    if let Some(qitaa) = masrah.ruqaa.qitaa.first_mut() {
        qitaa.sha256 = "0".repeat(64);
    }
    let khata = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil).err();
    assert!(
        matches!(khata, Some(KhataTathbeet::IdhnKharijiGhayrMutabiq { .. })),
        "got {khata:?}"
    );
    assert!(naqil.talabat.is_empty(), "nothing was fetched");
    Ok(())
}

/// The install put its manifest where an uninstall can find it, and nowhere that
/// collides with Taarib's own.
#[test]
fn al_bayan_yaskun_mujallad_al_madkhal_wahdahu() -> Natija {
    let (masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let natija = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;

    assert_eq!(
        natija.jidhr_nusakh,
        jidhr_nusakh_khariji(&masrah.nusakh, masrah.ruqaa.id)
    );
    assert_eq!(
        kharijiyat_mathbita(&masrah.nusakh),
        vec![natija.jidhr_nusakh]
    );
    assert!(
        !Tathbeet::mawjud(&masrah.nusakh, NawTathbeet::Nass),
        "a third-party install never occupies the directory Taarib's own uses"
    );
    Ok(())
}
