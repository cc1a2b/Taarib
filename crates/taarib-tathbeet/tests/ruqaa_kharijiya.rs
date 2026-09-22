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
    HalatMira, QitaatTanzeel, RuqaaKharijiya, TahdheerKhariji, TahdidBina, TakhtitKhariji,
};
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{IdhnMasdar, MasdarKhariji, RukhsaRuqaa, RuqaaId, RuqaaRevision};
use taarib_tathbeet::bayan::{Muthabbit as _, NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::jalb_khariji::NaqilKhariji;
use taarib_tathbeet::khariji::{
    TalabTathbeetKhariji, azil_khariji, jidhr_nusakh_khariji, kharijiyat_mathbita, thabbit_khariji,
};
use taarib_tathbeet::khata::{KhataTathbeet, NatijatTathbeet};
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, TaqreerIstiada, istiada_nass};
use taarib_tathbeet::tasadum::{ALAMAT_KHARIJIYA, JihatTasadum, la_yatasadam_maa_khariji};
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

/// The directory chain `lml/RTEA/Subtitles/texts/a.yldb` brings into being,
/// outermost first.
///
/// Four levels, none of which the game has. What makes them worth naming is that
/// only the innermost is mentioned anywhere — by the archive entry — and an
/// uninstall that removed that one and left `lml/` standing would have returned
/// a game the store did not ship.
const MUJALLADAT_ALTATHBEET: [&str; 4] = [
    "lml",
    "lml/RTEA",
    "lml/RTEA/Subtitles",
    "lml/RTEA/Subtitles/texts",
];

/// Every path RTEA's own HOW-TO orders deleted before the install: three
/// directories and seventeen files, spelled the way the author spells them.
///
/// The author's installer deletes these and cannot put them back. Putting them
/// back is the whole of why installing their work through Taarib is worth
/// anything, so each one is preserved before it is removed and restored byte for
/// byte on the way out.
const QAIMAT_HADHF: [&str; 20] = [
    "lml",
    "redteamassets",
    "mod backup",
    "asiloader",
    "dinput8",
    "installscript",
    "installscript_sdk",
    "redteamload.dll",
    "version.dll",
    "vfs",
    "lml.ini",
    "Nlog",
    "ModManager.core.dll",
    "ModManager.log",
    "ModManager.NativeInterop.dll",
    "NativeTrainer.asi",
    "ScriptHookRDR2.dll",
    "ScriptHookRDR2.log",
    "vfs.asi",
    "vfs.log",
];

/// A game holding every path on that list a game is *allowed* to be holding.
///
/// Six of the twenty — `lml`, `redteamassets`, `lml.ini`, `ModManager.core.dll`,
/// `ScriptHookRDR2.dll` and `vfs.asi` — are [`ALAMAT_KHARIJIYA`], and a game
/// holding one of those is a game [`la_yatasadam_maa_khariji`] refuses before the
/// remove-list is ever read. They are absent here by construction rather than by
/// oversight, and [`qaimat_hadhf_almuallif_taud_bayt_bi_bayt`] asserts that is
/// why rather than passing over them.
///
/// `dinput8.dll` is not on the author's list and is here on purpose: it is the
/// slot RTEA's own loader takes, a player may already have an ASI loader sitting
/// in it, and a path the archive *overwrites* is the only kind whose manifest
/// line carries both the game's original and the patch's own bytes.
const LUBA_BI_QAIMAT_HADHF: [(&str, &[u8]); 19] = [
    ("RDR2.exe", b"MZ the game's own executable"),
    ("GameNetworkingSockets.dll", b"MZ networking"),
    ("x64/data.rpf", b"the game's own archive, untouched"),
    ("dinput8.dll", b"an ASI loader the player already had"),
    ("asiloader", b"the author's list: asiloader"),
    ("dinput8", b"the author's list: dinput8"),
    ("installscript", b"the author's list: installscript"),
    ("installscript_sdk", b"the author's list: installscript_sdk"),
    ("redteamload.dll", b"the author's list: redteamload.dll"),
    ("version.dll", b"the author's list: version.dll"),
    ("vfs", b"the author's list: vfs"),
    ("Nlog", b"the author's list: Nlog"),
    ("ModManager.log", b"the author's list: ModManager.log"),
    (
        "ModManager.NativeInterop.dll",
        b"the author's list: ModManager.NativeInterop.dll",
    ),
    ("NativeTrainer.asi", b"the author's list: NativeTrainer.asi"),
    (
        "ScriptHookRDR2.log",
        b"the author's list: ScriptHookRDR2.log",
    ),
    ("vfs.log", b"the author's list: vfs.log"),
    (
        "mod backup/keep.bin",
        b"a file the player put in `mod backup`",
    ),
    ("mod backup/qadim/keep.bin", b"and one a level below it"),
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

/// Writes a fixture game tree.
fn ibni_luba(jidhr: &Path, asliya: &[(&str, &[u8])]) -> Natija {
    for (nisbi, bayt) in asliya {
        let masar = jidhr.join(nisbi);
        if let Some(walid) = masar.parent() {
            std::fs::create_dir_all(walid)?;
        }
        std::fs::write(masar, bayt)?;
    }
    Ok(())
}

/// Whether a remove-list path is one of the names [`ALAMAT_KHARIJIYA`] treats as
/// proof that a translation somebody else made is already installed.
///
/// Case-folded, because that is how [`la_yatasadam_maa_khariji`] reads them: the
/// host filesystem under Proton is case-sensitive and the game is not.
fn alama_kharijiya(nisbi: &str) -> bool {
    ALAMAT_KHARIJIYA
        .iter()
        .any(|alama| alama.eq_ignore_ascii_case(nisbi))
}

/// Whether a directory is there and holds no regular file at any depth.
fn mujallad_bila_malaffat(masar: &Path) -> bool {
    masar.is_dir()
        && walkdir::WalkDir::new(masar)
            .follow_links(false)
            .into_iter()
            .flatten()
            .all(|madkhal| !madkhal.file_type().is_file())
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
        // The fixture states no correspondence between a launcher's build id and
        // the game's own version, which is what an entry whose author never
        // published one looks like.
        tahdid_bina: TahdidBina::default(),
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
/// The gate over a build that was determined, which is every case but the
/// escape's own.
fn bawwaba(
    jidhr_luba: &Path,
    ruqaa: &RuqaaKharijiya,
    iqrar_tahdheerat: bool,
) -> NatijatFahsKhariji {
    bawwaba_bi_bina(
        jidhr_luba,
        ruqaa,
        &ShurutBawwaba {
            bina: "1491",
            iqrar_tahdheerat,
            iqrar_bina_majhula: false,
        },
    )
}

/// The same gate, with the build and the undetermined-build acknowledgement
/// both spelled out.
///
/// An undetermined build reaches the gate as an empty string — the value an
/// entry drawing no build distinction is installed with — because there is no
/// version to compare and inventing one to get past the gate would be the guess
/// the whole three-state answer exists to refuse.
/// What a gate run is being asked, when a test needs to say more than the
/// common case. Gathered rather than passed loose so a call site says which
/// answer each flag is giving.
struct ShurutBawwaba<'a> {
    bina: &'a str,
    iqrar_tahdheerat: bool,
    iqrar_bina_majhula: bool,
}

fn bawwaba_bi_bina(
    jidhr_luba: &Path,
    ruqaa: &RuqaaKharijiya,
    shurut: &ShurutBawwaba<'_>,
) -> NatijatFahsKhariji {
    let ShurutBawwaba {
        bina,
        iqrar_tahdheerat,
        iqrar_bina_majhula,
    } = *shurut;
    let iqrar = sijill_iqrar();
    fahs_khariji(&TalabFahsKhariji {
        luba: luba_id(),
        ism_luba: "Red Dead Redemption 2",
        jidhr_luba,
        appid: None,
        jidhr_steam: None,
        bina,
        ruqaa,
        iqrar: Some(&iqrar),
        iqrar_tahdheerat,
        iqrar_bina_majhula,
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
        Self::bi_shakl(&LUBA_ASLIYA, madakhil, |_| {})
    }

    /// The same staged run, over a game and an entry the caller shapes.
    ///
    /// `tashkeel` runs before the gate sees the entry, so a test that widens the
    /// declared layout or the remove-list is putting the gate and the installer
    /// in front of the same document rather than editing one behind the other's
    /// back.
    fn bi_shakl(
        asliya: &[(&str, &[u8])],
        madakhil: &[(&str, &[u8])],
        tashkeel: impl FnOnce(&mut RuqaaKharijiya),
    ) -> Result<(Self, NaqilThabit), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let jidhr = mujallad.path().to_path_buf();
        let luba = jidhr.join("luba");
        let nusakh = jidhr.join("nusakh");
        let tajmee = jidhr.join("tajmee");
        std::fs::create_dir_all(&luba)?;
        std::fs::create_dir_all(&nusakh)?;
        ibni_luba(&luba, asliya)?;

        let bayt = izim(madakhil)?;
        let mut ruqaa = madkhal(&bayt);
        tashkeel(&mut ruqaa);
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

/// The directories the install brought into being go; the ones the game shipped
/// stay, holding what they held.
///
/// `x64/` is Red Dead Redemption 2's own archive directory. A patch that drops
/// one file into it and an uninstall that then removed the directory would take
/// `data.rpf` with it — data the patch never owned, cannot put back, and that a
/// comparison over files alone would not even notice going.
#[test]
fn mujalladat_altathbeet_tazul_walqadima_tabqa() -> Natija {
    let mut madakhil = ARSHIF_UPDATE.to_vec();
    madakhil.push(("x64/rtea.rpf", b"one file dropped beside the game's own"));
    let (masrah, mut naqil) = Masrah::bi_shakl(&LUBA_ASLIYA, &madakhil, |ruqaa| {
        ruqaa.takhtit.yaktub.push("x64".to_owned());
    })?;

    let qabl = lamha(&masrah.luba);
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let natija = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;
    assert_eq!(natija.adad_maktub, madakhil.len());

    for nisbi in MUJALLADAT_ALTATHBEET {
        assert!(masrah.luba.join(nisbi).is_dir(), "{nisbi} was not created");
    }
    assert!(masrah.luba.join("x64/rtea.rpf").is_file());

    let taqreer = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Muhafiza,
    )?;
    assert_eq!(
        taqreer.mujalladat_muzala,
        MUJALLADAT_ALTATHBEET.len(),
        "the four the install created, and not the one it only wrote into"
    );
    assert_eq!(taqreer.mujalladat_matruka, 0);
    assert!(taqreer.baqaya.is_empty());

    // Every level, not only the one the archive entry named.
    for nisbi in MUJALLADAT_ALTATHBEET {
        assert!(
            !masrah.luba.join(nisbi).exists(),
            "{nisbi} outlived the patch that created it"
        );
    }
    assert!(
        masrah.luba.join("x64").is_dir(),
        "a directory the game shipped was removed because the patch wrote into it"
    );
    assert_eq!(
        std::fs::read(masrah.luba.join("x64/data.rpf"))?,
        b"the game's own archive, untouched"
    );
    assert!(!masrah.luba.join("x64/rtea.rpf").exists());
    assert_eq!(lamha(&masrah.luba), qabl);
    Ok(())
}

/// Every path the author's own instructions order deleted comes back byte for
/// byte, and a directory on that list keeps standing while its files go.
///
/// Fourteen of the twenty reach this test. The other six are
/// [`ALAMAT_KHARIJIYA`] — a game holding one of them is a game whose third-party
/// install is refused before the remove-list is read — so they are asserted
/// absent from the fixture and from the restored tree rather than passed over,
/// and `al_tasadum_marfud_fi_al_ittijahayn` is where that refusal is proved.
#[test]
fn qaimat_hadhf_almuallif_taud_bayt_bi_bayt() -> Natija {
    // Pinned, so that a marker list which grew to swallow the whole remove-list
    // would fail here rather than turn every loop below into a no-op.
    assert_eq!(
        QAIMAT_HADHF
            .iter()
            .filter(|nisbi| alama_kharijiya(nisbi))
            .count(),
        6,
        "six of the author's twenty are names the collision gate refuses a game for"
    );

    let (masrah, mut naqil) = Masrah::bi_shakl(&LUBA_BI_QAIMAT_HADHF, &ARSHIF_UPDATE, |ruqaa| {
        ruqaa.takhtit.yahdhif = QAIMAT_HADHF.iter().map(|&nisbi| nisbi.to_owned()).collect();
    })?;
    let qabl = lamha(&masrah.luba);

    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let natija = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;
    assert_eq!(
        natija.adad_mahdhuf, 15,
        "the thirteen loose files on the list plus the two inside `mod backup`"
    );

    for nisbi in QAIMAT_HADHF {
        if alama_kharijiya(nisbi) {
            // Present now only because the patch itself wrote it.
            continue;
        }
        if nisbi == "mod backup" {
            // A directory has no bytes to preserve, so removing one is a change
            // nothing can undo. Emptying it is the whole of what the author's
            // instruction is for.
            assert!(mujallad_bila_malaffat(&masrah.luba.join(nisbi)), "{nisbi}");
            assert!(mujallad_bila_malaffat(
                &masrah.luba.join("mod backup/qadim")
            ));
        } else {
            assert!(
                !masrah.luba.join(nisbi).exists(),
                "{nisbi} survived the author's remove-list"
            );
        }
    }

    let taqreer = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Muhafiza,
    )?;
    assert!(taqreer.mustabdala.is_empty());
    assert!(taqreer.baqaya.is_empty());
    assert_eq!(
        taqreer.mustaada, 16,
        "the fifteen the remove-list took out, and the loader slot the archive overwrote"
    );

    let baad = lamha(&masrah.luba);
    for nisbi in QAIMAT_HADHF {
        if alama_kharijiya(nisbi) {
            assert!(
                !qabl.contains_key(nisbi),
                "{nisbi} is a marker, and this game would have been refused"
            );
            assert!(
                !masrah.luba.join(nisbi).exists(),
                "{nisbi} outlived the patch that wrote it"
            );
            continue;
        }
        // The path itself and, for a directory on the list, everything under it.
        let bidaya = format!("{nisbi}/");
        let mutaathir: Vec<&String> = qabl
            .keys()
            .filter(|miftah| miftah.as_str() == nisbi || miftah.starts_with(bidaya.as_str()))
            .collect();
        assert!(
            !mutaathir.is_empty(),
            "{nisbi} was never in the fixture game"
        );
        for miftah in mutaathir {
            assert_eq!(
                baad.get(miftah),
                qabl.get(miftah),
                "{miftah} did not come back byte for byte"
            );
        }
    }
    assert_eq!(
        baad, qabl,
        "the uninstall did not return the game tree byte for byte"
    );
    Ok(())
}

/// Removing twice reports nothing the second time, and takes nothing with it.
///
/// The manifest outlives a completed removal on purpose — it is what
/// `nazzif_nusakh` consumes later — so a second press of the same button reopens
/// it, finds every line already done and returns an empty report. A failure here,
/// or a count, would read to the person pressing it as "the first removal lost
/// something".
#[test]
fn al_izala_marratayn_la_tuwhi_bi_faqd() -> Natija {
    let (masrah, mut naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let qabl = lamha(&masrah.luba);
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let _ = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;

    let awwal = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Muhafiza,
    )?;
    assert!(awwal.nazif());
    assert!(awwal.majmu() > 0);

    let thani = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Muhafiza,
    )?;
    assert_eq!(
        thani,
        TaqreerIstiada::jadeed(NawTathbeet::Nass, "Red Dead Redemption 2"),
        "the second run claimed work that was already done"
    );
    assert_eq!(lamha(&masrah.luba), qabl);
    Ok(())
}

/// A removal that cannot finish names what is left, and the next run finishes it
/// rather than starting over.
///
/// The interruption is the documented one: something replaced a file the patch
/// had written, here the loader slot the archive overwrote.
/// [`SiyasatIstiada::Sarima`] is the policy for a caller that wants an exact
/// revert and would rather be stopped than told afterwards, and stopping is what
/// leaves a manifest half-walked — the same state a crash or a closed laptop
/// leaves, reached deterministically.
#[test]
fn al_izala_almuqataa_tustanaf_min_haythu_waqafat() -> Natija {
    let (masrah, mut naqil) = Masrah::bi_shakl(&LUBA_BI_QAIMAT_HADHF, &ARSHIF_UPDATE, |ruqaa| {
        ruqaa.takhtit.yahdhif = QAIMAT_HADHF.iter().map(|&nisbi| nisbi.to_owned()).collect();
    })?;
    let qabl = lamha(&masrah.luba);
    let tarif = tarif_luba(&masrah.luba, masrah.ruqaa.id);
    let idhn = idhn(&masrah.luba, &masrah.ruqaa)?;
    let _ = thabbit_khariji(&masrah.talab(&tarif), idhn, &mut naqil)?;

    let majmu_sijillat = Tathbeet::istanif(
        &masrah.luba,
        &jidhr_nusakh_khariji(&masrah.nusakh, masrah.ruqaa.id),
        NawTathbeet::Nass,
    )?
    .bayan()
    .adad();

    let slot = masrah.luba.join("dinput8.dll");
    std::fs::write(&slot, b"a third tool wrote its own loader over the patch's")?;

    let natija = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Sarima,
    );
    let (munjaz, mutabaqqi) = match &natija {
        Err(KhataTathbeet::IstiadaNaqisa {
            munjaz,
            mutabaqqi,
            sabab,
            ..
        }) => {
            assert!(
                matches!(**sabab, KhataTathbeet::MalafMustabdal { .. }),
                "got {sabab:?}"
            );
            (*munjaz, mutabaqqi.clone())
        },
        akhar => return Err(format!("expected a partial removal, got {akhar:?}").into()),
    };
    assert!(munjaz > 0, "the run stopped before it had done anything");
    assert_eq!(
        munjaz + mutabaqqi.len(),
        majmu_sijillat,
        "the refusal accounts for every line of the manifest"
    );
    assert!(
        mutabaqqi.iter().any(|masar| masar == "dinput8.dll"),
        "the path that stopped the run is named among what is still modified"
    );

    // Whatever took the slot is put back to what the patch wrote, and the same
    // button is pressed again.
    let Some((_, jism)) = ARSHIF_UPDATE.iter().find(|(ism, _)| *ism == "dinput8.dll") else {
        return Err("the fixture archive stopped carrying a loader".into());
    };
    std::fs::write(&slot, jism)?;

    let taqreer = azil_khariji(
        &masrah.luba,
        &masrah.nusakh,
        masrah.ruqaa.id,
        SiyasatIstiada::Sarima,
    )?;
    assert_eq!(
        taqreer.majmu(),
        mutabaqqi.len(),
        "the second run redid lines the first had already finished"
    );
    assert!(taqreer.nazif());
    assert_eq!(
        lamha(&masrah.luba),
        qabl,
        "the resumed removal did not finish the job"
    );
    Ok(())
}

/// A build nobody could determine, accepted, installs.
///
/// The escape the interface offers has to actually open the gate, or the tick
/// is a control that does nothing: the person reads that Taarib could not tell
/// which build this is, says go ahead, and the install still stops — on a
/// refusal naming a build that was never read.
#[test]
fn bina_majhula_maa_iqrar_tamurr() -> Natija {
    let (masrah, _naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    match bawwaba_bi_bina(
        &masrah.luba,
        &masrah.ruqaa,
        &ShurutBawwaba {
            bina: "",
            iqrar_tahdheerat: true,
            iqrar_bina_majhula: true,
        },
    ) {
        NatijatFahsKhariji::Masmuh(_) => Ok(()),
        NatijatFahsKhariji::Marfud(rafd) => {
            Err(format!("the accepted escape was refused anyway: {rafd:?}").into())
        },
    }
}

/// The same build, not accepted, is refused — and the refusal says the build was
/// never determined rather than naming an empty one.
#[test]
fn bina_majhula_bila_iqrar_turfad() -> Natija {
    let (masrah, _naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let marfud = match bawwaba_bi_bina(
        &masrah.luba,
        &masrah.ruqaa,
        &ShurutBawwaba {
            bina: "",
            iqrar_tahdheerat: true,
            iqrar_bina_majhula: false,
        },
    ) {
        NatijatFahsKhariji::Marfud(rafd) => *rafd,
        NatijatFahsKhariji::Masmuh(_) => {
            return Err("an undetermined build was installed without being accepted".into());
        },
    };
    let RafdKhariji::BinaGhayrMadumma { ref bina, .. } = marfud else {
        return Err(format!("expected a build refusal, got {marfud:?}").into());
    };
    assert!(bina.is_empty(), "the refusal invented a build: {bina:?}");

    // "the installed build ()" names nothing and reads as a bug. The two states
    // get two sentences, and this is the one that says nothing was read.
    let injilizi = marfud.injilizi();
    assert!(
        injilizi.contains("never determined"),
        "the refusal does not say the build was undetermined: {injilizi}"
    );
    assert!(
        !injilizi.contains("build ()"),
        "the refusal rendered an empty build as a value: {injilizi}"
    );
    assert!(
        marfud.arabi().contains("لم يُحدَّد"),
        "the Arabic refusal does not say the build was undetermined: {}",
        marfud.arabi()
    );
    Ok(())
}

/// Accepting an undetermined build does not lift a build that *was* determined
/// and is not covered.
///
/// The one invariant holding the escape apart from a blanket override. "I could
/// not tell" is an absence of information a person may accept the risk of; "I
/// can tell, and this patch was never tested against it" is an established fact,
/// and no acknowledgement turns it into support.
#[test]
fn iqrar_almajhul_la_yarfa_bina_maqrua_ghayr_madumma() -> Natija {
    let (masrah, _naqil) = Masrah::jadeed(&ARSHIF_UPDATE)?;
    let marfud = match bawwaba_bi_bina(
        &masrah.luba,
        &masrah.ruqaa,
        &ShurutBawwaba {
            bina: "1207",
            iqrar_tahdheerat: true,
            iqrar_bina_majhula: true,
        },
    ) {
        NatijatFahsKhariji::Marfud(rafd) => *rafd,
        NatijatFahsKhariji::Masmuh(_) => {
            return Err("the undetermined-build escape lifted a determined mismatch".into());
        },
    };
    let RafdKhariji::BinaGhayrMadumma { bina, .. } = marfud else {
        return Err(format!("expected a build refusal, got {marfud:?}").into());
    };
    assert_eq!(bina, "1207", "the refusal named a build nobody installed");
    Ok(())
}
