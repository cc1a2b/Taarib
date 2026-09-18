//! Unity Localization across several locales, and which one reaches the project.
//!
//! A game using the Localization package ships one `StringTable` per locale, all
//! of them carrying the same keys under the same entry ids. Reading all of them
//! puts one game's text in front of a translator as many times as the publisher
//! shipped languages, and the machine-translation run then pays for every copy.
//!
//! The fixture here is a real `SerializedFile` written byte for byte — format
//! 21, a flat type tree, one `SharedTableData` and three `StringTable`s — rather
//! than a description of one, because the two things under test are both
//! properties of the real reader: that it recognises those assets when the build
//! left **no resolvable class name** on them, which is what an Addressables
//! build does and what made this defect invisible; and that having recognised
//! them it puts one locale in the table and says what it did with the others.
//!
//! There is also a measurement against an installed game, off by default and run
//! by pointing `TAARIB_LUBA_UNITY` at one. It is the only way to check the
//! numbers this change exists for against something a publisher shipped.

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; the lints are written for library code, and refusing to panic here \
              would mean a test that cannot fail. The measurement prints what it measured, \
              which is the whole point of running it."
)]
#![expect(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory; the product's own recursive deletes go through `HadafHadhf`"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use taarib_istikhraj::rafd::{SababRafd, TaqreerRafd};
use taarib_istikhraj::unity::{LUGHAT_MASDAR, ikhtar_lugha, istakhrij};

// ---------------------------------------------------------------------------
// The fixture: a SerializedFile written the way Unity writes one
// ---------------------------------------------------------------------------

/// The format version the fixture is written in.
///
/// Twenty-one: the last one before Unity moved the real sizes into a second
/// header, which keeps the writer to one header without costing any of the
/// structure under test — the flat type tree, the sixteen-byte hashes and the
/// eight-byte path ids are all the same in 22, which is what The Stalked 3
/// ships.
const ISDAR: u32 = 21;

/// Unity's class id for `MonoBehaviour`, which every localization asset is one
/// of.
const SANF_MONOBEHAVIOUR: i32 = 114;

/// The engine build the fixture claims to have been written by.
const ISDAR_MUHARRIK: &str = "6000.0.3f1";

/// One node of a flat type tree, as this fixture declares it.
struct Uqda {
    mustawa: u8,
    naw: &'static str,
    ism: &'static str,
    masfufa: bool,
}

/// A node at a given depth.
const fn uqda(mustawa: u8, naw: &'static str, ism: &'static str) -> Uqda {
    Uqda {
        mustawa,
        naw,
        ism,
        masfufa: false,
    }
}

/// An `Array` marker node, which is what tells the reader the parent repeats.
const fn masfufa(mustawa: u8) -> Uqda {
    Uqda {
        mustawa,
        naw: "Array",
        ism: "Array",
        masfufa: true,
    }
}

/// `SharedTableData`: the collection name and the entry-id-to-key map.
///
/// The root is named `MonoBehaviour` on purpose. That is exactly what an
/// Addressables build writes — the tree is generated for the base type and the
/// script reference points into another file — and it is the condition under
/// which the reader has to recognise the asset from the fields rather than from
/// a name.
fn shajarat_mushtarak() -> Vec<Uqda> {
    vec![
        uqda(0, "MonoBehaviour", "Base"),
        uqda(1, "string", "m_Name"),
        uqda(1, "string", "m_TableCollectionName"),
        uqda(1, "vector", "m_Entries"),
        masfufa(2),
        uqda(3, "int", "size"),
        uqda(3, "SharedTableEntry", "data"),
        uqda(4, "SInt64", "m_Id"),
        uqda(4, "string", "m_Key"),
    ]
}

/// `StringTable`: one locale's rows, and a pointer to the shared key map.
fn shajarat_jadwal() -> Vec<Uqda> {
    vec![
        uqda(0, "MonoBehaviour", "Base"),
        uqda(1, "string", "m_Name"),
        uqda(1, "LocaleIdentifier", "m_LocaleId"),
        uqda(2, "string", "m_Code"),
        uqda(1, "PPtr<SharedTableData>", "m_SharedData"),
        uqda(2, "int", "m_FileID"),
        uqda(2, "SInt64", "m_PathID"),
        uqda(1, "vector", "m_TableData"),
        masfufa(2),
        uqda(3, "int", "size"),
        uqda(3, "StringTableEntry", "data"),
        uqda(4, "SInt64", "m_Id"),
        uqda(4, "string", "m_Localized"),
    ]
}

/// A growable little-endian buffer that knows how to align the way Unity does.
#[derive(Default)]
struct Katib {
    bayt: Vec<u8>,
}

impl Katib {
    fn u8(&mut self, qeema: u8) {
        self.bayt.push(qeema);
    }

    fn u16(&mut self, qeema: u16) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    fn i16(&mut self, qeema: i16) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    fn u32(&mut self, qeema: u32) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    fn i32(&mut self, qeema: i32) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    fn i64(&mut self, qeema: i64) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    fn u64(&mut self, qeema: u64) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    fn khaam(&mut self, qeema: &[u8]) {
        self.bayt.extend_from_slice(qeema);
    }

    /// A NUL-terminated string, which is how the metadata stores its own names.
    fn munahi(&mut self, qeema: &str) {
        self.bayt.extend_from_slice(qeema.as_bytes());
        self.bayt.push(0);
    }

    /// A length-prefixed, four-byte-aligned string, which is how an object
    /// stores one.
    fn muhadhah(&mut self, qeema: &str) {
        let bayt = qeema.as_bytes();
        self.i32(i32::try_from(bayt.len()).expect("a fixture string shorter than 2 GiB"));
        self.khaam(bayt);
        self.hadhi();
    }

    /// Pads to the next four-byte boundary.
    fn hadhi(&mut self) {
        while !self.bayt.len().is_multiple_of(4) {
            self.bayt.push(0);
        }
    }

    const fn tul(&self) -> usize {
        self.bayt.len()
    }
}

/// One object's serialized bytes, with the type it is laid out by.
struct Kaain {
    hawiya_masar: i64,
    fahras_naw: usize,
    jism: Vec<u8>,
}

/// A `SharedTableData` object's bytes.
fn jism_mushtarak(ism: &str, majmua: &str, madakhil: &[(i64, &str)]) -> Vec<u8> {
    let mut katib = Katib::default();
    katib.muhadhah(ism);
    katib.muhadhah(majmua);
    katib.i32(i32::try_from(madakhil.len()).expect("a fixture entry count"));
    for (raqm, miftah) in madakhil {
        katib.i64(*raqm);
        katib.muhadhah(miftah);
    }
    katib.bayt
}

/// A `StringTable` object's bytes.
fn jism_jadwal(ism: &str, lugha: &str, mushtarak: i64, sufuf: &[(i64, &str)]) -> Vec<u8> {
    let mut katib = Katib::default();
    katib.muhadhah(ism);
    katib.muhadhah(lugha);
    katib.i32(0);
    katib.i64(mushtarak);
    katib.i32(i32::try_from(sufuf.len()).expect("a fixture row count"));
    for (raqm, nass) in sufuf {
        katib.i64(*raqm);
        katib.muhadhah(nass);
    }
    katib.bayt
}

/// Writes one type's record: the class id, the hashes, and the flat tree.
fn uktub_naw(katib: &mut Katib, uqad: &[Uqda]) {
    katib.i32(SANF_MONOBEHAVIOUR);
    // Not stripped, and no script index. A script index of -1 is what leaves
    // `ism_mukawwin` with nothing to answer with, which is the case under test.
    katib.u8(0);
    katib.i16(-1);
    katib.khaam(&[0_u8; 16]);
    katib.khaam(&[0_u8; 16]);

    let mut hajz: Vec<u8> = Vec::new();
    let izaha = |ism: &str, hajz: &mut Vec<u8>| -> u32 {
        let mawqi = u32::try_from(hajz.len()).expect("a fixture name buffer under 4 GiB");
        hajz.extend_from_slice(ism.as_bytes());
        hajz.push(0);
        mawqi
    };

    let mut uqad_khaam: Vec<(u8, u8, u32, u32)> = Vec::new();
    for unsur in uqad {
        let izahat_naw = izaha(unsur.naw, &mut hajz);
        let izahat_ism = izaha(unsur.ism, &mut hajz);
        uqad_khaam.push((
            unsur.mustawa,
            u8::from(unsur.masfufa),
            izahat_naw,
            izahat_ism,
        ));
    }

    katib.i32(i32::try_from(uqad_khaam.len()).expect("a fixture node count"));
    katib.i32(i32::try_from(hajz.len()).expect("a fixture name buffer size"));
    for (fahras, (mustawa, rayat, izahat_naw, izahat_ism)) in uqad_khaam.iter().enumerate() {
        katib.u16(1);
        katib.u8(*mustawa);
        katib.u8(*rayat);
        katib.u32(*izahat_naw);
        katib.u32(*izahat_ism);
        // A declared width of -1 everywhere. The reader takes every width from
        // the type name, and a node that declared a positive width with no
        // children is the one shape it refuses outright.
        katib.i32(-1);
        katib.i32(i32::try_from(fahras).expect("a fixture node index"));
        katib.u32(0);
        // Format 19 added a reference hash per node.
        katib.u64(0);
    }
    katib.khaam(&hajz);
    // Format 21 added the dependency list, which a fixture has none of.
    katib.i32(0);
}

/// A `SerializedFile` header is twenty bytes, and big-endian whatever the rest
/// of the file is.
const HAJM_RAAS: usize = 20;

/// A whole `SerializedFile` holding the given types and objects.
fn ibni_mulsal(anwa: &[Vec<Uqda>], kaainat: &[Kaain]) -> Vec<u8> {
    let mut wasfiya = Katib::default();
    wasfiya.munahi(ISDAR_MUHARRIK);
    // Standalone Windows 64, which is what every fixture in this crate claims.
    wasfiya.i32(19);
    // The type tree is present, which is the whole premise of this file.
    wasfiya.u8(1);

    wasfiya.i32(i32::try_from(anwa.len()).expect("a fixture type count"));
    for uqad in anwa {
        uktub_naw(&mut wasfiya, uqad);
    }

    wasfiya.i32(i32::try_from(kaainat.len()).expect("a fixture object count"));
    // Object bodies are laid out first so that each one's offset is known while
    // the table that names it is being written.
    let mut izahat: Vec<u32> = Vec::new();
    let mut bayanat = Katib::default();
    for kaain in kaainat {
        bayanat.hadhi();
        izahat.push(u32::try_from(bayanat.tul()).expect("a fixture data offset"));
        bayanat.khaam(&kaain.jism);
    }
    for (kaain, izaha) in kaainat.iter().zip(&izahat) {
        // From format 14 the path id is eight bytes preceded by padding to a
        // four-byte boundary, and the padding is in no field's declared size.
        wasfiya.hadhi();
        wasfiya.i64(kaain.hawiya_masar);
        wasfiya.u32(*izaha);
        wasfiya.u32(u32::try_from(kaain.jism.len()).expect("a fixture object under 4 GiB"));
        wasfiya.i32(i32::try_from(kaain.fahras_naw).expect("a fixture type index"));
    }

    // No script types, no externals, no reference types.
    wasfiya.i32(0);
    wasfiya.i32(0);
    wasfiya.i32(0);

    // Everything after the header follows the endianness byte, which this
    // fixture writes as little.
    let mut izahat_bayanat = HAJM_RAAS + wasfiya.tul();
    izahat_bayanat = izahat_bayanat.next_multiple_of(4);

    let hajm_malaf = izahat_bayanat + bayanat.tul();
    let mut malaf: Vec<u8> = Vec::with_capacity(hajm_malaf);
    let hajm_wasfiya = u32::try_from(izahat_bayanat - HAJM_RAAS).expect("a fixture metadata size");
    malaf.extend_from_slice(&hajm_wasfiya.to_be_bytes());
    malaf.extend_from_slice(
        &u32::try_from(hajm_malaf)
            .expect("a fixture size")
            .to_be_bytes(),
    );
    malaf.extend_from_slice(&ISDAR.to_be_bytes());
    malaf.extend_from_slice(
        &u32::try_from(izahat_bayanat)
            .expect("a fixture data offset")
            .to_be_bytes(),
    );
    malaf.push(0);
    malaf.extend_from_slice(&[0_u8; 3]);
    malaf.extend_from_slice(&wasfiya.bayt);
    while malaf.len() < izahat_bayanat {
        malaf.push(0);
    }
    malaf.extend_from_slice(&bayanat.bayt);
    malaf
}

/// The three keys every locale's table in the fixture holds.
const MAFATEEH: [(i64, &str); 3] = [(11, "menu/play"), (22, "menu/quit"), (33, "hud/objective")];

/// One locale's rows, in that locale.
fn sufuf(lugha: &str) -> Vec<(i64, String)> {
    match lugha {
        "en" => vec![
            (11, "Play".to_owned()),
            (22, "Quit".to_owned()),
            (33, "Find the key".to_owned()),
        ],
        "de" => vec![
            (11, "Spielen".to_owned()),
            (22, "Beenden".to_owned()),
            (33, "Finde den Schlüssel".to_owned()),
        ],
        _ => vec![
            (11, "プレイ".to_owned()),
            (22, "終了".to_owned()),
            (33, "鍵を見つける".to_owned()),
        ],
    }
}

/// A Unity install holding one `resources.assets` with a shared table and one
/// `StringTable` per locale named.
fn ibni_luba(masar: &Path, lughat: &[&str]) {
    let mushtarak_id = 1_i64;
    let mut kaainat = vec![Kaain {
        hawiya_masar: mushtarak_id,
        fahras_naw: 0,
        jism: jism_mushtarak("Menu Shared Data", "Menu", &MAFATEEH),
    }];
    for (fahras, lugha) in lughat.iter().enumerate() {
        let sufuf_lugha = sufuf(lugha);
        let mustaar: Vec<(i64, &str)> = sufuf_lugha
            .iter()
            .map(|(raqm, nass)| (*raqm, nass.as_str()))
            .collect();
        kaainat.push(Kaain {
            hawiya_masar: i64::try_from(fahras).expect("a fixture index") + 2,
            fahras_naw: 1,
            jism: jism_jadwal(&format!("Menu_{lugha}"), lugha, mushtarak_id, &mustaar),
        });
    }

    let bayt = ibni_mulsal(&[shajarat_mushtarak(), shajarat_jadwal()], &kaainat);
    let bayanat = masar.join("Ikhtibar_Data");
    std::fs::create_dir_all(&bayanat).expect("the fixture's data directory");
    std::fs::write(bayanat.join("resources.assets"), &bayt).expect("the fixture");
}

/// A directory of this test's own, removed and recreated so a rerun starts
/// clean.
fn mujallad_ikhtibar(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-istikhraj-tawtin-{ism}"));
    let _ = std::fs::remove_dir_all(&masar);
    std::fs::create_dir_all(&masar).expect("the scratch directory");
    masar
}

/// Every locale the report says was set aside, with its row count.
fn manhiya(taqreer: &TaqreerRafd) -> Vec<(String, String, usize)> {
    let mut natija: Vec<(String, String, usize)> = taqreer
        .marfuda
        .iter()
        .filter_map(|madkhal| match &madkhal.sabab {
            SababRafd::LughaGhayrMukhtara {
                lugha,
                mukhtara,
                adad,
            } => Some((lugha.clone(), mukhtara.clone(), *adad)),
            _ => None,
        })
        .collect();
    natija.sort();
    natija
}

// ---------------------------------------------------------------------------
// The regression this change exists for
// ---------------------------------------------------------------------------

/// Eleven locales in, one locale out — and the other ten named in the report.
#[test]
fn jadawil_muta_addidat_allughat_tuti_lugha_wahida() {
    let jidhr = mujallad_ikhtibar("mutaaddida");
    let lughat = [
        "de", "en", "es-ES", "fr", "it", "ja", "ko", "pt-BR", "ru", "tr", "zh-Hans",
    ];
    ibni_luba(&jidhr, &lughat);

    let (jadwal, taqreer) = istakhrij(&jidhr);
    let madakhil = jadwal.ila_mudkhalat();

    // Three keys, once each. Not thirty-three, which is what reading every
    // locale produced, and not zero, which is what a fixture the reader failed
    // to recognise would produce.
    assert_eq!(
        madakhil.len(),
        MAFATEEH.len(),
        "one locale's rows and nothing else, got {:?}",
        madakhil
            .iter()
            .map(|madkhal| madkhal.masdar.as_str())
            .collect::<Vec<_>>()
    );

    let nusus: BTreeSet<&str> = madakhil
        .iter()
        .map(|madkhal| madkhal.masdar.as_str())
        .collect();
    assert_eq!(
        nusus,
        BTreeSet::from(["Play", "Quit", "Find the key"]),
        "the English table is the one that reached the project"
    );

    // The developer's own key is the identity, which is only possible because
    // the `SharedTableData` was recognised and consumed as the key map rather
    // than harvested as prose.
    for madkhal in &madakhil {
        assert!(
            madkhal.siyaq.mawqi.contains("StringTable/Menu/"),
            "a row keyed by the collection and the developer's key, got {}",
            madkhal.siyaq.mawqi
        );
    }
    assert!(
        madakhil
            .iter()
            .any(|madkhal| madkhal.siyaq.mawqi.contains("StringTable/Menu/menu/play")),
        "the shared table's key reached the row"
    );

    // Every locale that was not read is in the report, by name, with what it
    // would have contributed.
    let manhiya = manhiya(&taqreer);
    assert_eq!(manhiya.len(), lughat.len() - 1, "ten locales set aside");
    for (lugha, mukhtara, adad) in &manhiya {
        assert_ne!(lugha, "en", "the locale that was read is not set aside");
        assert_eq!(mukhtara, "en", "the report names the locale that was read");
        assert_eq!(*adad, MAFATEEH.len(), "with its own row count");
    }
    let asma: BTreeSet<&str> = manhiya.iter().map(|(lugha, _, _)| lugha.as_str()).collect();
    assert_eq!(
        asma,
        lughat
            .iter()
            .copied()
            .filter(|lugha| *lugha != "en")
            .collect::<BTreeSet<_>>()
    );

    // Set aside is not lost, and the report has to say so: every key those rows
    // carried is in the table once, in the locale that was read.
    for madkhal in &taqreer.marfuda {
        if matches!(madkhal.sabab, SababRafd::LughaGhayrMukhtara { .. }) {
            assert!(
                !madkhal.sabab.khasara(),
                "a set-aside locale is not counted as lost text"
            );
            assert!(
                !madkhal.sabab.yuslihuhu_iltiqat(),
                "and capture is not offered as its remedy"
            );
        }
    }

    // The container's own read entry says what happened, so a user learns it
    // from the report rather than from the source.
    let qira = taqreer
        .maqrua
        .iter()
        .find(|qira| qira.hawiya.ends_with("resources.assets"))
        .expect("the container was read");
    assert_eq!(qira.adad, MAFATEEH.len());
    assert!(
        qira.wasf.contains("Unity Localization string table(s)") && qira.wasf.contains("set aside"),
        "the read entry names the decision, got {}",
        qira.wasf
    );

    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A game shipping no locale this build reads from is read whole, not ranked.
///
/// The arm that stops the mechanism from being an assumption that games are
/// written in English. Two locales, neither of them the one Taarib translates
/// from, and both survive with the locale in the key.
#[test]
fn bila_lughat_masdar_tuqra_kull_allughat() {
    let jidhr = mujallad_ikhtibar("bila-masdar");
    ibni_luba(&jidhr, &["ja", "de"]);

    let (jadwal, taqreer) = istakhrij(&jidhr);
    let madakhil = jadwal.ila_mudkhalat();

    assert_eq!(
        madakhil.len(),
        MAFATEEH.len() * 2,
        "both locales are kept when neither can be ranked"
    );
    assert!(
        manhiya(&taqreer).is_empty(),
        "and nothing is reported as set aside"
    );

    let qira = taqreer
        .maqrua
        .iter()
        .find(|qira| qira.hawiya.ends_with("resources.assets"))
        .expect("the container was read");
    assert!(
        qira.wasf.contains("every locale is kept"),
        "the read entry says why nothing was chosen, got {}",
        qira.wasf
    );

    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A game shipping one locale is not told a choice was made about it.
#[test]
fn lugha_wahida_bila_qarar() {
    let jidhr = mujallad_ikhtibar("wahida");
    ibni_luba(&jidhr, &["ja"]);

    let (jadwal, taqreer) = istakhrij(&jidhr);
    assert_eq!(jadwal.ila_mudkhalat().len(), MAFATEEH.len());
    assert!(manhiya(&taqreer).is_empty());

    let qira = taqreer
        .maqrua
        .iter()
        .find(|qira| qira.hawiya.ends_with("resources.assets"))
        .expect("the container was read");
    assert!(
        !qira.wasf.contains("set aside") && !qira.wasf.contains("source locale"),
        "one locale is not a decision, got {}",
        qira.wasf
    );

    let _ = std::fs::remove_dir_all(&jidhr);
}

/// The rule [`ikhtar_lugha`] applies, stated as the cases it has to get right.
#[test]
fn qaidat_ikhtiyar_allugha() {
    let majmua = |lughat: &[&str]| -> BTreeSet<String> {
        lughat.iter().map(|lugha| (*lugha).to_owned()).collect()
    };

    assert_eq!(
        ikhtar_lugha(&majmua(&["de", "en", "ja"])).as_deref(),
        Some(LUGHAT_MASDAR),
        "the source locale itself, when the game ships it"
    );
    assert_eq!(
        ikhtar_lugha(&majmua(&["de", "EN"])).as_deref(),
        Some("EN"),
        "a locale code's case is the publisher's business, not a different locale"
    );
    assert_eq!(
        ikhtar_lugha(&majmua(&["en-GB", "fr"])).as_deref(),
        Some("en-GB"),
        "a regional spelling stands in when it is the only one in that language"
    );
    assert_eq!(
        ikhtar_lugha(&majmua(&["en-GB", "en-AU", "fr"])),
        None,
        "two regional spellings and no bare one is a choice this build will not make"
    );
    assert_eq!(
        ikhtar_lugha(&majmua(&["ja", "ko", "zh-Hans"])),
        None,
        "a game shipping no locale Taarib reads from is not ranked at all"
    );
    assert_eq!(ikhtar_lugha(&BTreeSet::new()), None, "and nor is no game");
}

// ---------------------------------------------------------------------------
// The measurement
// ---------------------------------------------------------------------------

/// Extraction over an installed Unity game, for the row counts this change is
/// judged on.
///
/// Off unless `TAARIB_LUBA_UNITY` names a game directory, because a test that
/// needs somebody's Steam library is not a test CI can run. It reads the game's
/// files and never launches it.
#[test]
fn qiyas_luba_munassaba() {
    let Ok(jidhr) = std::env::var("TAARIB_LUBA_UNITY") else {
        return;
    };
    let (jadwal, taqreer) = istakhrij(Path::new(&jidhr));
    let madakhil = jadwal.ila_mudkhalat();

    println!("{jidhr}");
    println!("  rows in the table: {}", madakhil.len());
    for qira in &taqreer.maqrua {
        if qira.adad > 0 || qira.wasf.contains("Unity Localization") {
            println!("  read {:>6}  {}", qira.adad, qira.hawiya);
            println!("             {}", qira.wasf);
        }
    }
    let manhiya = manhiya(&taqreer);
    let majmu: usize = manhiya.iter().map(|(_, _, adad)| *adad).sum();
    println!(
        "  set aside: {} locale(s), {majmu} row(s) — {}",
        manhiya.len(),
        manhiya
            .iter()
            .map(|(lugha, _, adad)| format!("{lugha}={adad}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for satr in taqreer.taqreer() {
        println!("  {satr}");
    }
}
