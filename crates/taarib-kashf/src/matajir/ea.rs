//! متجر إي إيه — the EA app and Origin before it.
//!
//! One publisher, two clients, three catalogue artefacts, and no single file
//! that lists everything. Origin wrote a `.mfst` file per installed item into a
//! machine-wide directory; the EA app inherited that directory for some titles
//! and keeps its own `InstallData` tree for others; and every install of either
//! era carries a `__Installer\installerdata.xml` inside the game's own folder
//! describing what it is. This adapter reads all three and merges them by
//! install path.
//!
//! ```text
//! %PROGRAMDATA%\EA Desktop\InstallData\**\*.mfst
//! %PROGRAMDATA%\Origin\LocalContent\**\*.mfst
//! %PROGRAMDATA%\Origin\local.xml            the default install root
//! <install root>\__Installer\installerdata.xml
//! ```
//!
//! Windows only, and honestly so: neither client runs natively on Linux or
//! macOS, and EA games on Linux run under a compatibility layer that Heroic or
//! Steam owns — a different adapter, a different catalogue.
//!
//! # What is deliberately not read
//!
//! The modern EA app keeps its authoritative installed-games list in an
//! encrypted blob under `%PROGRAMDATA%\EA Desktop\`, keyed from machine
//! hardware identifiers. Taarib does not decrypt it. Working around another
//! product's encryption to read its private state is not something a program
//! that asks users to trust it with their game files should do, and it would
//! break on the first key rotation anyway. Everything below is plaintext that
//! the installer wrote to be read.
//!
//! # Telling a `.mfst` apart from XML
//!
//! `.mfst` is not one format. Origin wrote most of them as a URL-encoded query
//! string — `?currentstate=kCompleted&id=1026023&installpath=C%3a%5c…` — with
//! no XML anywhere in it; a minority, and most of the EA app era files, are
//! ordinary XML documents. They are told apart by content, never by name: a
//! byte order mark and leading whitespace are skipped and the first real
//! character is examined. `<` means XML and the file goes to the manifest
//! parser; anything else is treated as a query string, where `?` is optional
//! and the pairs are split on `&` and `=`. A file that is neither yields no
//! pairs, produces no install path, and degrades to one [`TanbihFahs`].
//!
//! # Reading `installerdata.xml` defensively
//!
//! These files span more than a decade of installer versions. The element
//! names moved (`<contentID>` alone, then wrapped in `<contentIDs>`), the game
//! version appears as an attribute in some and as element text in others, and
//! `<runtime>` may hold one launcher or several. So the file is walked as an
//! event stream with a bounded element stack rather than deserialized into a
//! fixed shape: elements that are recognised contribute, elements that are not
//! are ignored, and a malformed document contributes whatever it produced
//! before the error instead of contributing nothing.
//!
//! # Artwork
//!
//! Left empty. Neither client stores a per-game image that can be addressed
//! from the manifests — the EA app's images live in a Chromium cache keyed by
//! request hash — and inventing a URL would put a broken image in the library.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::{dakhil, qira_nass};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs, TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "ea";

/// Platforms this adapter can find anything on.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Windows];

/// The per-install manifest, relative to a game's own root.
const BAYAN_TATHBEET: [&str; 2] = ["__Installer", "installerdata.xml"];

/// How deep the `.mfst` search descends. Origin nests one directory per title
/// and the EA app one or two; three is generous and bounded.
const UMQ_BAHTH: usize = 3;

/// The largest element nesting this reader will track. A document deeper than
/// this is not an installer manifest.
const HADD_UMQ_XML: usize = 64;

/// A hard stop on the event stream, so a pathological document cannot hold a
/// scan open.
const HADD_AHDATH_XML: usize = 200_000;

/// Anti-cheat services, matched case-insensitively against the executables the
/// manifest names. Hints for Phase 16, never conclusions.
const ATHAR_HIMAYA: [(&str, &str); 6] = [
    ("easyanticheat", "EasyAntiCheat"),
    ("eaanticheat", "EA AntiCheat"),
    ("battleye", "BattlEye"),
    ("beservice", "BattlEye"),
    ("punkbuster", "PunkBuster"),
    ("denuvo", "Denuvo Anti-Cheat"),
];

/// The EA app, and Origin.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarEa;

impl MatjarEa {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarEa {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "إي إيه"
    }

    fn ism_injilizi(&self) -> &'static str {
        "EA app"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        if let Some(tajawuz) = siyaq.manassat.ea.as_ref() {
            return Some(tajawuz.clone());
        }
        // Existence only. The EA app's directory is preferred because it is the
        // client that is still shipping; an Origin-only machine reports Origin.
        let bayanat = siyaq.bayanat_barnamij.as_ref()?;
        let ea = bayanat.join("EA Desktop");
        if ea.is_dir() {
            return Some(ea);
        }
        let origin = bayanat.join("Origin");
        origin.is_dir().then_some(origin)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when a configured launcher
    /// root is not there. Nothing else is fatal: an unreadable `.mfst`, a
    /// manifest that will not parse, and a game whose directory was deleted are
    /// each one [`TanbihFahs`] and cost only that entry.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }
        if let Some(tajawuz) = siyaq.manassat.ea.as_ref()
            && !tajawuz.exists()
        {
            return Err(KhataKashf::JidhrMuhaddadMafqud {
                matjar: MUARRIF,
                masar: tajawuz.clone(),
            }
            .into());
        }

        let judhur = judhur_matjar(siyaq);
        if judhur.is_empty() {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, judhur.first().cloned());
        let mut murashahat: BTreeMap<String, MurashahEa> = BTreeMap::new();

        for jidhr in &judhur {
            for masar in malaffat_mfst(jidhr) {
                match murashah_min_mfst(&masar) {
                    Ok(Some(murashah)) => {
                        damm_murashah(&mut murashahat, murashah, siyaq.nizam);
                    },
                    Ok(None) => {},
                    Err(tanbih) => natija.tanbihat.push(tanbih),
                }
            }
        }

        for jidhr in judhur_tathbeet(siyaq, &judhur) {
            let qaima = match std::fs::read_dir(&jidhr) {
                Ok(qaima) => qaima,
                Err(sabab) => {
                    // A whole install root that will not list is a set of games
                    // this scan did not see, not one game that degraded.
                    natija.tanbihat.push(TanbihFahs::fahras(
                        MUARRIF,
                        jidhr.display().to_string(),
                        format!("cannot list this EA install folder ({sabab})"),
                    ));
                    continue;
                },
            };
            for madkhal in qaima.flatten() {
                let masar = madkhal.path();
                if masar.is_dir() && masar_bayan(&masar).is_file() {
                    damm_murashah(
                        &mut murashahat,
                        MurashahEa { jidhr: masar, muarrif: None, muktamila: true },
                        siyaq.nizam,
                    );
                }
            }
        }

        let mut maruf: BTreeSet<String> = BTreeSet::new();
        for murashah in murashahat.into_values() {
            match luba_min_murashah(&murashah) {
                Ok(luba) => {
                    let muarrif_luba = luba.masdar.muarrif();
                    if maruf.insert(muarrif_luba) {
                        natija.alaab.push(luba);
                    }
                },
                Err(tanbih) => natija.tanbihat.push(tanbih),
            }
        }

        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let judhur = judhur_matjar(siyaq);
        let mut muraqaba: Vec<PathBuf> = judhur
            .iter()
            .flat_map(|jidhr| [jidhr.join("InstallData"), jidhr.join("LocalContent")])
            .filter(|masar| masar.is_dir())
            .collect();
        muraqaba.extend(judhur_tathbeet(siyaq, &judhur));
        muraqaba.sort();
        muraqaba.dedup();
        muraqaba
    }
}

/// The client data roots that exist on this machine.
fn judhur_matjar(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
    if let Some(tajawuz) = siyaq.manassat.ea.as_ref() {
        return vec![tajawuz.clone()];
    }
    let Some(bayanat) = siyaq.bayanat_barnamij.as_ref() else {
        return Vec::new();
    };
    [bayanat.join("EA Desktop"), bayanat.join("Origin")]
        .into_iter()
        .filter(|masar| masar.is_dir())
        .collect()
}

/// Directories that hold game installations rather than client data.
///
/// Two sources, both verified: the `DownloadInPlaceDir` setting the clients
/// write into `local.xml`, and the conventional install roots — used only when
/// they are actually there, so nothing is invented for a machine that never had
/// them.
fn judhur_tathbeet(siyaq: &SiyaqFahs, judhur: &[PathBuf]) -> Vec<PathBuf> {
    let mut mawadi: Vec<PathBuf> = Vec::new();
    for jidhr in judhur {
        let masar = jidhr.join("local.xml");
        if masar.is_file()
            && let Ok(nass) = qira_nass(&masar)
        {
            mawadi.extend(judhur_min_idadat(&nass));
        }
    }

    // Both program directories, from the context: the two clients have shipped
    // 32-bit and 64-bit installers over their lives, so a machine can have one
    // set of games under each.
    for jidhr in &siyaq.mujalladat_baramij {
        mawadi.push(jidhr.join("EA Games"));
        mawadi.push(jidhr.join("Origin Games"));
    }
    mawadi.extend(siyaq.manassat.mujalladat_idafiya.iter().cloned());

    mawadi.retain(|masar| masar.is_dir());
    mawadi.sort();
    mawadi.dedup();
    mawadi
}

/// The install roots named in a client's `local.xml`.
///
/// The file is a flat `<Settings><Setting key="…" value="…"/></Settings>`, and
/// the attribute order is not stable across versions, so both attributes are
/// read off the same element rather than positionally.
fn judhur_min_idadat(nass: &str) -> Vec<PathBuf> {
    let mut judhur = Vec::new();
    let mut qari = Reader::from_str(bila_bom(nass));
    qari.config_mut().trim_text(true);
    let mut adad = 0usize;
    loop {
        adad = adad.saturating_add(1);
        if adad > HADD_AHDATH_XML {
            break;
        }
        match qari.read_event() {
            Ok(Event::Start(unsur) | Event::Empty(unsur)) => {
                if !ism_hua(unsur.name().local_name().as_ref(), "setting") {
                    continue;
                }
                let mut miftah = None;
                let mut qeema = None;
                for sifa in unsur.attributes().flatten() {
                    let ism = sifa.key.local_name();
                    let nass_sifa = sifa
                        .normalized_value(XmlVersion::Implicit1_0)
                        .unwrap_or_default()
                        .into_owned();
                    if ism_hua(ism.as_ref(), "key") {
                        miftah = Some(nass_sifa);
                    } else if ism_hua(ism.as_ref(), "value") {
                        qeema = Some(nass_sifa);
                    }
                }
                if let (Some(miftah), Some(qeema)) = (miftah, qeema)
                    && miftah.eq_ignore_ascii_case("DownloadInPlaceDir")
                    && !qeema.trim().is_empty()
                {
                    judhur.push(PathBuf::from(qeema.trim()));
                }
            },
            Ok(Event::Eof) | Err(_) => break,
            Ok(_) => {},
        }
    }
    judhur
}

/// Every `.mfst` file under a client data root.
fn malaffat_mfst(jidhr: &Path) -> Vec<PathBuf> {
    let mut malaffat: Vec<PathBuf> = [jidhr.join("InstallData"), jidhr.join("LocalContent")]
        .into_iter()
        .filter(|masar| masar.is_dir())
        .flat_map(|masar| {
            walkdir::WalkDir::new(masar)
                .max_depth(UMQ_BAHTH)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|madkhal| madkhal.file_type().is_file())
                .map(walkdir::DirEntry::into_path)
                .filter(|masar| imtidad_hua(masar, "mfst"))
                .collect::<Vec<PathBuf>>()
        })
        .collect();
    malaffat.sort();
    malaffat
}

/// One install directory, before its manifest has been read.
#[derive(Debug, Clone)]
struct MurashahEa {
    /// The installation root.
    jidhr: PathBuf,
    /// The content identifier the `.mfst` named, when it named one.
    muarrif: Option<String>,
    /// Whether the client considers the install finished.
    muktamila: bool,
}

/// Adds a candidate, keeping the entry that knows the most about it.
fn damm_murashah(
    murashahat: &mut BTreeMap<String, MurashahEa>,
    murashah: MurashahEa,
    nizam: NizamTashghil,
) {
    let miftah = muwahhad(&murashah.jidhr, nizam);
    match murashahat.get_mut(&miftah) {
        Some(mawjud) => {
            if mawjud.muarrif.is_none() {
                mawjud.muarrif = murashah.muarrif;
            }
            mawjud.muktamila = mawjud.muktamila && murashah.muktamila;
        },
        None => {
            let _ = murashahat.insert(miftah, murashah);
        },
    }
}

/// Reads one `.mfst`, in whichever of its two shapes it is written.
fn murashah_min_mfst(masar: &Path) -> Result<Option<MurashahEa>, TanbihFahs> {
    let nass = qira_nass(masar).map_err(|khata| {
        TanbihFahs::jadeed(MUARRIF, masar.display().to_string(), khata.injilizi)
    })?;

    if hiya_xml(&nass) {
        // An XML `.mfst` is a copy of the installer manifest: it describes the
        // content but never carries an install path, because it was written
        // before the user chose one. Nothing here can locate a game with it, so
        // it is dropped without a warning — the install it describes is found
        // through the install-root scan, and its own
        // `__Installer\installerdata.xml` is read there.
        return Ok(None);
    }

    let huqul = huqul_istifsar(&nass);
    let masar_tathbeet = huqul
        .get("installpath")
        .or_else(|| huqul.get("dipinstallpath"))
        .map(String::as_str)
        .map(str::trim)
        .filter(|qeema| !qeema.is_empty());

    let Some(masar_tathbeet) = masar_tathbeet else {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            "this install manifest names no install path, so the game it describes cannot be \
             located"
                .to_owned(),
        ));
    };

    let jidhr = PathBuf::from(masar_tathbeet);
    if !jidhr.is_dir() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            "the client still lists this game, but its install directory is gone".to_owned(),
        ));
    }

    let muarrif = huqul
        .get("id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|qeema| !qeema.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            masar
                .file_stem()
                .and_then(|ism| ism.to_str())
                .map(str::trim)
                .filter(|ism| !ism.is_empty())
                .map(str::to_owned)
        });

    let muktamila = huqul
        .get("currentstate")
        .map(String::as_str)
        .is_none_or(|hala| hala.eq_ignore_ascii_case("kCompleted"));

    Ok(Some(MurashahEa { jidhr, muarrif, muktamila }))
}

/// Splits a URL-encoded query string into its pairs.
///
/// Keys are lowercased because the clients are not consistent about their case;
/// values are percent-decoded, with `+` read as a space, as the encoding
/// requires. A pair without `=` is kept with an empty value rather than
/// dropped, so a malformed manifest still yields whatever it did contain.
fn huqul_istifsar(nass: &str) -> BTreeMap<String, String> {
    let mut huqul = BTreeMap::new();
    let jism = nass.trim().trim_start_matches('?');
    for zawj in jism.split('&') {
        if zawj.trim().is_empty() {
            continue;
        }
        let (miftah, qeema) = zawj.split_once('=').unwrap_or((zawj, ""));
        let miftah = fak_tarmiz(miftah).trim().to_lowercase();
        if miftah.is_empty() {
            continue;
        }
        let _ = huqul.insert(miftah, fak_tarmiz(qeema));
    }
    huqul
}

/// Percent-decoding, byte by byte and entirely bounds-checked.
///
/// An incomplete or non-hexadecimal escape is left exactly as it was written
/// rather than guessed at, and the result is decoded as UTF-8 with invalid
/// sequences replaced — a Windows path in a legacy code page will lose the
/// characters that were never UTF-8 rather than fail the whole entry.
fn fak_tarmiz(nass: &str) -> String {
    let bayt = nass.as_bytes();
    let mut kharij: Vec<u8> = Vec::with_capacity(bayt.len());
    let mut fahras: usize = 0;
    while let Some(&harf) = bayt.get(fahras) {
        match harf {
            b'+' => {
                kharij.push(b' ');
                fahras = fahras.saturating_add(1);
            },
            b'%' => {
                let ala = bayt.get(fahras.saturating_add(1)).copied().and_then(raqm_sittashari);
                let adna = bayt.get(fahras.saturating_add(2)).copied().and_then(raqm_sittashari);
                if let (Some(ala), Some(adna)) = (ala, adna) {
                    kharij.push((ala << 4) | adna);
                    fahras = fahras.saturating_add(3);
                } else {
                    kharij.push(harf);
                    fahras = fahras.saturating_add(1);
                }
            },
            _ => {
                kharij.push(harf);
                fahras = fahras.saturating_add(1);
            },
        }
    }
    String::from_utf8_lossy(&kharij).into_owned()
}

/// One hexadecimal digit, or nothing.
const fn raqm_sittashari(harf: u8) -> Option<u8> {
    match harf {
        b'0'..=b'9' => Some(harf - b'0'),
        b'a'..=b'f' => Some(harf - b'a' + 10),
        b'A'..=b'F' => Some(harf - b'A' + 10),
        _ => None,
    }
}

/// Whether a document is XML, decided by its first real character rather than
/// by its name.
fn hiya_xml(nass: &str) -> bool {
    bila_bom(nass).trim_start().starts_with('<')
}

// ---------------------------------------------------------------------------
// installerdata.xml
// ---------------------------------------------------------------------------

/// One entry in a manifest's `<runtime>` block.
#[derive(Debug, Clone, Default)]
struct MushghilEa {
    /// The executable, still carrying its `[INSTALLDIR]` token.
    masar: Option<String>,
    /// Whether the manifest marks this launcher as a trial.
    tajriba: bool,
}

/// What a `__Installer\installerdata.xml` says about the install it sits in.
#[derive(Debug, Clone, Default)]
struct BayanTathbeetEa {
    /// Every content identifier the manifest declares, in document order.
    muarrifat: Vec<String>,
    /// The title, preferring the English locale.
    unwan: Option<String>,
    /// The game version the manifest declares.
    isdar: Option<String>,
    /// The disk space the installer reserved, which is the only size either
    /// client records.
    masaha: Option<u64>,
    /// The runtime launchers, in document order.
    mushghilat: Vec<MushghilEa>,
}

/// The per-install manifest path for an install root.
fn masar_bayan(jidhr: &Path) -> PathBuf {
    BAYAN_TATHBEET.iter().fold(jidhr.to_path_buf(), |mabni, juz| mabni.join(juz))
}

/// Walks an installer manifest, keeping what it recognises.
///
/// Returns what was collected before any error, because a manifest that is
/// truncated still names the game in its first few elements.
fn iqra_bayan_tathbeet(nass: &str) -> BayanTathbeetEa {
    let mut bayan = BayanTathbeetEa::default();
    let mut qari = Reader::from_str(bila_bom(nass));
    // Not `trim_text(true)`: an entity reference is delivered as its own event,
    // so a value is trimmed once it is whole rather than at every seam in it.
    qari.config_mut().trim_text(false);

    let mut madad = String::new();
    let mut kawm: Vec<String> = Vec::new();
    let mut mushghil: Option<MushghilEa> = None;
    let mut lugha_unwan: Option<String> = None;
    let mut unwan_injilizi = false;
    let mut adad = 0usize;

    loop {
        adad = adad.saturating_add(1);
        if adad > HADD_AHDATH_XML {
            break;
        }
        match qari.read_event() {
            Ok(Event::Start(unsur)) => {
                let ism = ism_unsur(unsur.name().local_name().as_ref());
                if ism == "launcher" && kawm.iter().any(|walid| walid == "runtime") {
                    mushghil = Some(MushghilEa::default());
                }
                if ism == "gametitle" {
                    lugha_unwan = sifa_nass(&unsur, "locale");
                }
                if ism == "gameversion"
                    && let Some(qeema) = sifa_nass(&unsur, "version")
                {
                    let _ = bayan.isdar.get_or_insert(qeema);
                }
                madad.clear();
                if kawm.len() < HADD_UMQ_XML {
                    kawm.push(ism);
                } else {
                    break;
                }
            },
            Ok(Event::Empty(unsur)) => {
                let ism = ism_unsur(unsur.name().local_name().as_ref());
                if ism == "gameversion"
                    && let Some(qeema) = sifa_nass(&unsur, "version")
                {
                    let _ = bayan.isdar.get_or_insert(qeema);
                }
            },
            Ok(Event::Text(nass_unsur)) => madad.push_str(&nass_unsur.xml10_content()),
            Ok(Event::GeneralRef(marja)) => {
                match taarib_usus::kayanat::hall_marja(&marja) {
                    Some(hall) => madad.push_str(&hall),
                    None => madad.push_str(&taarib_usus::kayanat::nass_marja(&marja)),
                }
            },
            Ok(Event::End(unsur)) => {
                let ism_nihaya = ism_unsur(unsur.name().local_name().as_ref());
                let qeema = std::mem::take(&mut madad).trim().to_owned();
                let ism = kawm.last().cloned().unwrap_or_default();
                if !qeema.is_empty() {
                    match ism.as_str() {
                        "contentid" => bayan.muarrifat.push(qeema),
                        "gametitle" => {
                            let injilizi = lugha_unwan
                                .as_deref()
                                .is_none_or(|lugha| lugha.to_lowercase().starts_with("en"));
                            if bayan.unwan.is_none() || (injilizi && !unwan_injilizi) {
                                unwan_injilizi = injilizi;
                                bayan.unwan = Some(qeema);
                            }
                        },
                        "gameversion" => {
                            let _ = bayan.isdar.get_or_insert(qeema);
                        },
                        "requireddiskspace" => {
                            bayan.masaha = qeema.parse::<u64>().ok();
                        },
                        "filepath" => {
                            if let Some(hali) = mushghil.as_mut() {
                                let _ = hali.masar.get_or_insert(qeema);
                            }
                        },
                        "trial" => {
                            if let Some(hali) = mushghil.as_mut() {
                                hali.tajriba = qeema == "1" || qeema.eq_ignore_ascii_case("true");
                            }
                        },
                        _ => {},
                    }
                }
                if ism_nihaya == "launcher"
                    && let Some(hali) = mushghil.take()
                {
                    bayan.mushghilat.push(hali);
                }
                if ism_nihaya == "gametitle" {
                    lugha_unwan = None;
                }
                let _ = kawm.pop();
            },
            Ok(Event::Eof) | Err(_) => break,
            Ok(_) => {},
        }
    }
    bayan
}

/// An element or attribute name, lowercased for comparison.
fn ism_unsur(ism: &str) -> String {
    ism.trim().to_lowercase()
}

/// Whether a name matches, without regard to case.
fn ism_hua(ism: &str, matlub: &str) -> bool {
    ism.trim().eq_ignore_ascii_case(matlub)
}

/// One attribute of an element, unescaped.
fn sifa_nass(unsur: &quick_xml::events::BytesStart<'_>, matlub: &str) -> Option<String> {
    unsur
        .attributes()
        .flatten()
        .find_map(|sifa| {
            ism_hua(sifa.key.local_name().as_ref(), matlub).then(|| {
                sifa.normalized_value(XmlVersion::Implicit1_0)
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            })
        })
        .filter(|qeema| !qeema.is_empty())
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Turns one candidate install directory into a game.
fn luba_min_murashah(murashah: &MurashahEa) -> Result<LubaMuktashafa, TanbihFahs> {
    let masar_bayan_tathbeet = masar_bayan(&murashah.jidhr);
    let bayan = if masar_bayan_tathbeet.is_file() {
        match qira_nass(&masar_bayan_tathbeet) {
            Ok(nass) => iqra_bayan_tathbeet(&nass),
            Err(khata) => {
                return Err(TanbihFahs::jadeed(
                    MUARRIF,
                    masar_bayan_tathbeet.display().to_string(),
                    khata.injilizi,
                ));
            },
        }
    } else {
        BayanTathbeetEa::default()
    };

    // Identity comes from the content identifier, because that is what the
    // registry shards on. The `.mfst` name is the same identifier for the
    // Origin-era titles that have one, and is the fallback when the manifest
    // declares several and none of them matches.
    let muarrif = murashah
        .muarrif
        .clone()
        .filter(|qeema| bayan.muarrifat.is_empty() || bayan.muarrifat.contains(qeema))
        .or_else(|| bayan.muarrifat.first().cloned())
        .or_else(|| murashah.muarrif.clone());

    let Some(muarrif) = muarrif else {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            murashah.jidhr.display().to_string(),
            "this install declares no content identifier in either its manifest or its install \
             record, so it has no identity a patch could be published against"
                .to_owned(),
        ));
    };

    let tanfidhi = bayan
        .mushghilat
        .iter()
        .filter_map(|mushghil| mushghil.masar.as_deref())
        .find_map(|qeema| masar_mushghil(&murashah.jidhr, qeema))
        .filter(|masar| masar.is_file());

    let mut simat = Vec::new();
    if bayan.mushghilat.iter().any(|mushghil| mushghil.tajriba) {
        simat.push(SimatLuba::LaysatLuba("trial".to_owned()));
    }
    for himaya in himayat(&bayan) {
        simat.push(SimatLuba::HimayaMuhtamala(himaya));
    }

    Ok(LubaMuktashafa {
        masdar: MasdarLuba::Ea(muarrif),
        hala_matjar: None,
        ism: bayan
            .unwan
            .clone()
            .or_else(|| ism_min_mujallad(&murashah.jidhr))
            .unwrap_or_else(|| murashah.jidhr.display().to_string()),
        jidhr: murashah.jidhr.clone(),
        tanfidhi,
        // `requiredDiskSpace` is what the installer reserved, and is the only
        // size either client records. It is reported as the launcher's own
        // number, not measured by walking the directory — a scan that walked
        // every game's file tree would take minutes on a large library.
        hajm: bayan.masaha.unwrap_or(0),
        bina_manassa: bayan.isdar,
        // Neither client records an update or last-played time in any file this
        // adapter can read, so nothing is claimed.
        akhir_tahdith: None,
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        suwar: MasadirSuwar::default(),
        khiyarat_tashghil: None,
        muktamila: murashah.muktamila,
        simat,
    })
}

/// Anti-cheat services named by any launcher in the manifest.
fn himayat(bayan: &BayanTathbeetEa) -> Vec<String> {
    let mut wujida: BTreeSet<String> = BTreeSet::new();
    for mushghil in &bayan.mushghilat {
        let Some(masar) = mushghil.masar.as_deref() else {
            continue;
        };
        let munkhafid = masar.to_ascii_lowercase();
        for (athar, ism) in ATHAR_HIMAYA {
            if munkhafid.contains(athar) {
                let _ = wujida.insert(ism.to_owned());
            }
        }
    }
    wujida.into_iter().collect()
}

/// Resolves a launcher path out of the manifest's own token vocabulary.
///
/// `[INSTALLDIR]` is the only token that can be resolved from what a scan
/// knows. A path built on any other token — the common application data
/// directory, a program files directory — is left alone rather than guessed at,
/// and Phase 5 finds the executable by probing the directory instead.
fn masar_mushghil(jidhr: &Path, qeema: &str) -> Option<PathBuf> {
    let munazzam = qeema.trim().replace('\\', "/");
    let baqi = munazzam
        .strip_prefix("[INSTALLDIR]")
        .or_else(|| munazzam.strip_prefix("[installdir]"))?;
    let baqi = baqi.trim_start_matches('/');
    if baqi.is_empty() {
        return None;
    }
    dakhil(jidhr, baqi).ok()
}

/// An install directory's own name.
fn ism_min_mujallad(jidhr: &Path) -> Option<String> {
    jidhr
        .file_name()
        .map(|ism| ism.to_string_lossy().trim().to_owned())
        .filter(|ism| !ism.is_empty())
}

/// Whether a path carries an extension, compared without regard to case.
fn imtidad_hua(masar: &Path, imtidad: &str) -> bool {
    masar
        .extension()
        .and_then(|qeema| qeema.to_str())
        .is_some_and(|qeema| qeema.eq_ignore_ascii_case(imtidad))
}

/// A path in the one form two paths can be compared in on this platform.
fn muwahhad(masar: &Path, nizam: NizamTashghil) -> String {
    let nass = masar.to_string_lossy().replace('\\', "/");
    let nass = nass.trim_end_matches('/').to_owned();
    if nizam.hassas_lil_ahruf() { nass } else { nass.to_lowercase() }
}

/// Strips a byte order mark, which the installer writes in front of some
/// manifests and which upsets an XML reader that was told the encoding.
fn bila_bom(nass: &str) -> &str {
    nass.strip_prefix('\u{feff}').unwrap_or(nass)
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// A Windows-shaped context whose machine directories are the ones the test
    /// built rather than this machine's.
    ///
    /// `nizam` is a value rather than a compile-time fact, which is what lets
    /// a Windows-only adapter be exercised from a Linux host — and the whole
    /// point of the conversion is checkable only because of it: nothing here
    /// sets an environment variable, so if the adapter still read
    /// `%PROGRAMDATA%` inline these tests would be reading the developer's own
    /// machine instead of the fixture, and would fail on it.
    fn siyaq(manzil: &Path, bayanat: Option<&Path>, baramij: &[PathBuf]) -> SiyaqFahs {
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, manzil);
        siyaq.bayanat_barnamij = bayanat.map(Path::to_path_buf);
        siyaq.mujalladat_baramij = baramij.to_vec();
        siyaq
    }

    #[test]
    fn al_mawqi_min_bayanat_al_barnamij_fi_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let bayanat = masrah.path().join("ProgramData");
        fs::create_dir_all(bayanat.join("EA Desktop"))?;
        fs::create_dir_all(bayanat.join("Origin"))?;

        // The EA app wins over Origin because it is the client still shipping.
        let siyaq = siyaq(masrah.path(), Some(&bayanat), &[]);
        assert_eq!(
            MatjarEa::jadeed().mawqi(&siyaq),
            Some(bayanat.join("EA Desktop")),
            "the data root must come from the context"
        );
        assert_eq!(judhur_matjar(&siyaq).len(), 2, "both client roots are real here");
        Ok(())
    }

    #[test]
    fn bila_bayanat_barnamij_la_shaya() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;

        // What a Linux or macOS context looks like: no machine-wide data
        // folder at all. The old resolver returned `C:\ProgramData` here, which
        // is a path that cannot exist on the machine asking.
        let siyaq = siyaq(masrah.path(), None, &[]);
        assert_eq!(MatjarEa::jadeed().mawqi(&siyaq), None);
        assert!(judhur_matjar(&siyaq).is_empty());
        Ok(())
    }

    #[test]
    fn judhur_al_tathbeet_min_mujalladat_al_baramij() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let baramij =
            [masrah.path().join("Program Files (x86)"), masrah.path().join("Program Files")];
        fs::create_dir_all(baramij[0].join("Origin Games"))?;
        fs::create_dir_all(baramij[1].join("EA Games"))?;

        // Both program directories are searched, and neither is a `C:` literal:
        // on a machine whose Windows is not on `C:` the old code looked in a
        // folder that was not there.
        let judhur = judhur_tathbeet(&siyaq(masrah.path(), None, &baramij), &[]);
        assert!(judhur.contains(&baramij[0].join("Origin Games")), "{judhur:?}");
        assert!(judhur.contains(&baramij[1].join("EA Games")), "{judhur:?}");
        assert_eq!(judhur.len(), 2, "only the directories that exist: {judhur:?}");
        Ok(())
    }
}
