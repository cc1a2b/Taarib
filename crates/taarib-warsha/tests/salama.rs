//! The no-silent-loss proof: a string file with good rows and damaged lines
//! opens with every damaged line named, is never rewritten by a read, and is
//! set aside only by an explicit rescue that keeps the original byte for byte.
//!
//! Everything here runs against a real project directory written by the real
//! project store, damaged by appending real bytes to its real file. The rows
//! are authored fixtures shaped like extracted strings; no game was read.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::io::Write as _;
use std::path::Path;

use taarib_istikhraj::mashru::{BayanIstikhraj, MALAF_NUSUS, MashruMaftuh};
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::muraja::SijillMuraja;
use taarib_mustalahat::nass::{
    MasdarIstikhraj, MudkhalNass, NassId, QuyudNass, SiyaqNass, TasnifNass,
};
use taarib_warsha::khata::KhataWarsha;
use taarib_warsha::salama::{self, HalatNusus, MalafTaqreerInqadh, SababTalaf, WASM_TALIF};

/// The moment every write in these tests carries.
const WAQT: &str = "2026-09-06T10:00:00Z";

/// A torn head: identity and source text intact, everything after cut off —
/// what an interrupted append leaves when the crash lands past the first two
/// fields.
const RAAS_MABTUR: &str =
    r#"{"id":"1b4e28ba-2fa1-5d68-9d3a-3a0f0b1c2d3e","masdar":"Press any key","hadaf":nu"#;

/// Bytes that are not UTF-8 at all, then a partial object.
const BAYT_TALIFA: &[u8] = b"\xff\xfe\x00{\"id\":\"";

/// Valid JSON that is not a row.
const SHAKL_GHARIB: &str = r#"{"x":1,"y":[2,3]}"#;

fn saf(mawqi: &str, masdar: &str) -> MudkhalNass {
    MudkhalNass {
        id: NassId::min_mawqi("data/menu.json", mawqi, masdar),
        masdar: masdar.to_owned(),
        hadaf: Some(format!("ترجمة {mawqi}")),
        muraja: SijillMuraja::jadeed(),
        siyaq: SiyaqNass {
            hawiya: "data/menu.json".to_owned(),
            mawqi: mawqi.to_owned(),
            ..SiyaqNass::default()
        },
        quyud: QuyudNass::default(),
        nasq_masdar: Vec::new(),
        nasq_hadaf: Vec::new(),
        takrar: 1,
        majmua: None,
        alamat: Vec::new(),
        tareeqa: None,
        muzawwid: None,
        muharrir: None,
        akhir_tabdeel: None,
        tasnif: TasnifNass::Ikhtiyar,
        thiqat_tasnif: 80,
        masdar_istikhraj: MasdarIstikhraj::Sakin,
        tarmiz: None,
    }
}

fn mashru_jadeed(jidhr: &Path) -> MashruMaftuh {
    let luba = LubaId::min_masdar(&MasdarLuba::Steam(480), "Spacewar");
    MashruMaftuh::ansha(
        jidhr.to_path_buf(),
        luba,
        "Spacewar".to_owned(),
        BayanIstikhraj::default(),
        WAQT.to_owned(),
    )
    .expect("a fresh project directory")
}

/// A project of `adad` good rows, committed to disk.
fn mashru_bi_sufuf(jidhr: &Path, adad: usize) -> MashruMaftuh {
    let mut mashru = mashru_jadeed(jidhr);
    let sufuf: Vec<MudkhalNass> = (0..adad)
        .map(|raqm| saf(&format!("menu/{raqm}"), &format!("Option {raqm}")))
        .collect();
    mashru.adif(sufuf).expect("rows accepted");
    mashru.ikhtim(WAQT.to_owned()).expect("project closed");
    mashru
}

/// Appends the three damaged lines this file tests with, raw.
fn atlif(jidhr: &Path) {
    let mut malaf = std::fs::OpenOptions::new()
        .append(true)
        .open(jidhr.join(MALAF_NUSUS))
        .expect("the string file opens for append");
    malaf.write_all(RAAS_MABTUR.as_bytes()).unwrap();
    malaf.write_all(b"\n").unwrap();
    malaf.write_all(BAYT_TALIFA).unwrap();
    malaf.write_all(b"\n").unwrap();
    malaf.write_all(SHAKL_GHARIB.as_bytes()).unwrap();
    malaf.write_all(b"\n").unwrap();
}

fn asma(jidhr: &Path) -> Vec<String> {
    let mut asma: Vec<String> = std::fs::read_dir(jidhr)
        .expect("the project directory lists")
        .map(|mudkhal| {
            mudkhal
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    asma.sort();
    asma
}

#[test]
fn altalifa_tuhsa_bi_arqamiha_wa_asbabiha() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mashru = mashru_bi_sufuf(muaqqat.path(), 5);
    atlif(muaqqat.path());

    let qiraa = salama::iqra_nusus(&mashru).expect("the file reads");

    assert_eq!(qiraa.hala(), HalatNusus::Talifa);
    assert!(qiraa.mawjud);
    assert_eq!(qiraa.sufuf.len(), 5, "every good row survives");
    assert_eq!(qiraa.talifa.len(), 3, "every damaged line is counted");

    let arqam: Vec<usize> = qiraa.talifa.iter().map(|satr| satr.raqm).collect();
    assert_eq!(arqam, vec![6, 7, 8], "line numbers are the file's own");

    let mabtur = &qiraa.talifa[0];
    assert!(matches!(mabtur.sabab, SababTalaf::Sigha { .. }));
    assert_eq!(
        mabtur.huwiya.map(|id| id.uuid().to_string()).as_deref(),
        Some("1b4e28ba-2fa1-5d68-9d3a-3a0f0b1c2d3e"),
        "a torn line still names the string it was"
    );
    assert_eq!(mabtur.masdar.as_deref(), Some("Press any key"));

    let tarmiz = &qiraa.talifa[1];
    assert_eq!(tarmiz.sabab, SababTalaf::Tarmiz);
    assert_eq!(
        tarmiz.huwiya, None,
        "a head cut inside the identity is not guessed"
    );
    assert_eq!(tarmiz.tul, BAYT_TALIFA.len());

    let gharib = &qiraa.talifa[2];
    assert!(matches!(gharib.sabab, SababTalaf::Sigha { .. }));
    assert_eq!(gharib.muqtataf, SHAKL_GHARIB);
    assert!(!gharib.sabab.wasf_arabi().is_empty());
    assert!(!gharib.sabab.wasf_injilizi().is_empty());
}

#[test]
fn alqiraa_la_taktub_shayan() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mashru = mashru_bi_sufuf(muaqqat.path(), 4);
    atlif(muaqqat.path());
    let qabl = std::fs::read(muaqqat.path().join(MALAF_NUSUS)).unwrap();
    let asma_qabl = asma(muaqqat.path());

    let _ = salama::iqra_nusus(&mashru).expect("the file reads");

    assert_eq!(
        std::fs::read(muaqqat.path().join(MALAF_NUSUS)).unwrap(),
        qabl
    );
    assert_eq!(asma(muaqqat.path()), asma_qabl);
}

#[test]
fn alfarigh_yumayyaz_min_altalif() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mashru = mashru_jadeed(muaqqat.path());

    let ghaib = salama::iqra_nusus(&mashru).expect("an absent file reads as empty");
    assert_eq!(ghaib.hala(), HalatNusus::Farigh);
    assert!(!ghaib.mawjud);
    assert_eq!(ghaib.hajm, 0);

    std::fs::write(muaqqat.path().join(MALAF_NUSUS), b"\n\n").unwrap();
    let khali = salama::iqra_nusus(&mashru).expect("a blank file reads as empty");
    assert_eq!(khali.hala(), HalatNusus::Farigh);
    assert!(khali.mawjud);

    std::fs::write(muaqqat.path().join(MALAF_NUSUS), BAYT_TALIFA).unwrap();
    let talif = salama::iqra_nusus(&mashru).expect("a damaged file reads");
    assert_eq!(talif.hala(), HalatNusus::Talifa);
    assert!(talif.sufuf.is_empty());
    assert_eq!(talif.talifa.len(), 1);
}

#[test]
fn alsalim_yuqra_salima() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mashru = mashru_bi_sufuf(muaqqat.path(), 3);

    let qiraa = salama::iqra_nusus(&mashru).expect("the file reads");

    assert_eq!(qiraa.hala(), HalatNusus::Salima);
    assert!(qiraa.salima());
    assert_eq!(qiraa.sufuf.len(), 3);
}

#[test]
fn alinqadh_yahfaz_alasl_bayt_bi_bayt_thumma_yuktub_alnajin() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mut mashru = mashru_bi_sufuf(muaqqat.path(), 5);
    atlif(muaqqat.path());
    let asl = std::fs::read(muaqqat.path().join(MALAF_NUSUS)).unwrap();

    let taqreer = salama::anqidh(&mut mashru, WAQT).expect("the rescue runs");

    assert_eq!(taqreer.najin, 5);
    assert_eq!(taqreer.talifa.len(), 3);
    assert_eq!(taqreer.basma, blake3::hash(&asl).to_hex().to_string());

    let mahfudh = std::fs::read(&taqreer.mahfudh).expect("the preserved copy exists");
    assert_eq!(mahfudh, asl, "the damaged file is preserved byte for byte");
    let ism_mahfudh = taqreer
        .mahfudh
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(ism_mahfudh, format!("{WASM_TALIF}.20260906T100000Z.jsonl"));

    let marfud = std::fs::read(&taqreer.marfud).expect("the rejected lines exist");
    let mut mutawaqqa = Vec::new();
    mutawaqqa.extend_from_slice(RAAS_MABTUR.as_bytes());
    mutawaqqa.push(b'\n');
    mutawaqqa.extend_from_slice(BAYT_TALIFA);
    mutawaqqa.push(b'\n');
    mutawaqqa.extend_from_slice(SHAKL_GHARIB.as_bytes());
    mutawaqqa.push(b'\n');
    assert_eq!(
        marfud, mutawaqqa,
        "exactly the damaged lines, raw, in order"
    );

    let bayt_taqreer = std::fs::read(&taqreer.taqreer).expect("the report exists");
    let malaf: MalafTaqreerInqadh =
        serde_json::from_slice(&bayt_taqreer).expect("the report parses");
    assert_eq!(malaf.najin, 5);
    assert_eq!(malaf.waqt, WAQT);
    assert_eq!(malaf.asl, MALAF_NUSUS);
    assert_eq!(malaf.mahfudh, ism_mahfudh);
    assert_eq!(malaf.basma, taqreer.basma);
    assert_eq!(malaf.hajm, u64::try_from(asl.len()).unwrap());
    let arqam: Vec<usize> = malaf.talifa.iter().map(|satr| satr.raqm).collect();
    assert_eq!(arqam, vec![6, 7, 8]);

    let baad = salama::iqra_nusus(&mashru).expect("the live table reads");
    assert_eq!(baad.hala(), HalatNusus::Salima);
    assert_eq!(baad.sufuf.len(), 5);
    assert_eq!(
        mashru.rasm().adad,
        5,
        "the header counts what the live table holds"
    );

    let (sufuf_asl, _) = salama::hallil_jsonl(&asl);
    assert_eq!(
        baad.sufuf, sufuf_asl,
        "the survivors are the rows that read, unchanged"
    );
}

#[test]
fn alinqadh_yarfud_bila_talaf_wa_la_yaktub() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mut mashru = mashru_bi_sufuf(muaqqat.path(), 2);
    let asma_qabl = asma(muaqqat.path());
    let qabl = std::fs::read(muaqqat.path().join(MALAF_NUSUS)).unwrap();

    let natija = salama::anqidh(&mut mashru, WAQT);

    assert!(matches!(natija, Err(KhataWarsha::LaTalaf)));
    assert_eq!(asma(muaqqat.path()), asma_qabl, "nothing was written");
    assert_eq!(
        std::fs::read(muaqqat.path().join(MALAF_NUSUS)).unwrap(),
        qabl
    );
}

#[test]
fn alinqadh_marratayn_la_yaktub_fawq_almahfudh() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mut mashru = mashru_bi_sufuf(muaqqat.path(), 3);
    atlif(muaqqat.path());
    let awwal = salama::anqidh(&mut mashru, WAQT).expect("the first rescue runs");
    let mahfudh_awwal = std::fs::read(&awwal.mahfudh).unwrap();

    atlif(muaqqat.path());
    let thani = salama::anqidh(&mut mashru, WAQT).expect("the second rescue runs");

    assert_ne!(
        thani.mahfudh, awwal.mahfudh,
        "a second rescue at the same moment gets a new name"
    );
    assert_eq!(
        std::fs::read(&awwal.mahfudh).unwrap(),
        mahfudh_awwal,
        "the first copy is untouched"
    );
    assert_eq!(thani.najin, 3);
}

#[test]
fn alinqadh_ala_malaf_kullihi_talif_yutriku_jadwalan_farighan_wa_yahfaz_alasl() {
    let muaqqat = tempfile::tempdir().expect("a scratch directory");
    let mut mashru = mashru_jadeed(muaqqat.path());
    std::fs::write(muaqqat.path().join(MALAF_NUSUS), BAYT_TALIFA).unwrap();

    let qabl = salama::iqra_nusus(&mashru).unwrap();
    assert_eq!(qabl.hala(), HalatNusus::Talifa);
    assert!(
        qabl.sufuf.is_empty(),
        "nothing read, and that is not an empty project"
    );

    let taqreer = salama::anqidh(&mut mashru, WAQT).expect("the rescue runs");

    assert_eq!(taqreer.najin, 0);
    assert_eq!(std::fs::read(&taqreer.mahfudh).unwrap(), BAYT_TALIFA);
    let baad = salama::iqra_nusus(&mashru).unwrap();
    assert_eq!(
        baad.hala(),
        HalatNusus::Farigh,
        "after the choice, the live table is honestly empty"
    );
}
