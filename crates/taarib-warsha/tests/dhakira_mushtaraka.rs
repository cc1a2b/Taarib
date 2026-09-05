//! The accumulation proof: what a second player of the same game gets free,
//! and what stops one player's bad recognition becoming everyone's.
//!
//! Two real memory files in two real directories, a real signed share moved
//! between them as bytes, and a translator that is a local stub counting its
//! own calls. Nothing is mocked except the translator, which is a stub on
//! purpose — a paid provider would make the assertions about cost untestable.
//!
//! **The game identities are real** and come from this machine's own Steam
//! library: `1245620` and `367520` are ELDEN RING's and Hollow Knight's
//! actual application identifiers, so the per-game keying is exercised
//! against identities the product would really compute.
//!
//! **The recognized strings are authored fixtures**, not text extracted from
//! either game. They are shaped like what an overlay reads — a subtitle, an
//! item name, an interface label, a prompt — but no OCR engine ran and no
//! game file was parsed to produce them. Saying otherwise would be presenting
//! a fixture as a game.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::collections::BTreeMap;
use std::path::Path;

use taarib_khatm::MiftahKhass;
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::TasnifNass;
use taarib_tarjama::dhakira::{
    AslQayd, Dhakira, NawAsl, QaydJadid, ThiqatQira, miftah_muwahhad,
};
use taarib_tarjama::mulahazat::{DhakiratTabaqa, MulahazaTabaqa};
use taarib_warsha::mushtaraka::{
    self, IqraratMusharaka, KhiyaratMusharaka, TahdheerMusharaka,
};

/// ELDEN RING, as this machine's Steam library identifies it.
const APPID_ELDEN: u32 = 1_245_620;

/// Hollow Knight, likewise.
const APPID_HOLLOW: u32 = 367_520;

/// The recognizer the fixtures claim to have come from.
const QARI: &str = "windows-ocr";

/// The provider the stub stands in for.
const MUZAWWID: &str = "mahalli";

/// A local stub translator that counts every line it was actually asked to
/// translate.
///
/// This is the number every cost assertion in this file is about. It is a
/// table rather than a model because the point being proved is how many times
/// something is asked, not what it answers.
#[derive(Debug, Default)]
struct MutarjimMahalli {
    jadwal: BTreeMap<String, String>,
    nida: u64,
}

impl MutarjimMahalli {
    fn jadeed(azwaj: &[(&str, &str)]) -> Self {
        Self {
            jadwal: azwaj
                .iter()
                .map(|(injilizi, arabi)| ((*injilizi).to_owned(), (*arabi).to_owned()))
                .collect(),
            nida: 0,
        }
    }

    fn tarjim(&mut self, nass: &str) -> String {
        self.nida = self.nida.saturating_add(1);
        self.jadwal.get(nass).cloned().unwrap_or_else(|| format!("[{nass}]"))
    }
}

/// The authored fixtures: what an overlay might read off a screen, and the
/// Arabic a translator gives back.
fn azwaj() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Press E to open the door", "اضغط E لفتح الباب"),
        ("You have obtained a Golden Rune", "حصلت على رونة ذهبية"),
        ("Save and Quit", "احفظ واخرج"),
        ("Are you sure?", "هل أنت متأكد؟"),
        ("The old road north is closed", "الطريق القديم شمالًا مغلق"),
        ("Restore your flask at a site of grace", "املأ قارورتك عند موضع نعمة"),
        ("Inventory is full", "الحقيبة ممتلئة"),
        ("Loading", "جارٍ التحميل"),
    ]
}

/// One play session's worth of readings: every line, with the repeats a
/// recognizer really produces when a box sits on screen for two hundred
/// frames.
fn jalsa_qira() -> Vec<&'static str> {
    let mut sutur = Vec::new();
    for (injilizi, _) in azwaj() {
        // A line is read out of every poll while it is on screen.
        for _ in 0..5 {
            sutur.push(injilizi);
        }
    }
    // And the player walks back past two of them.
    sutur.push("Save and Quit");
    sutur.push("Are you sure?");
    sutur
}

fn luba_elden() -> LubaId {
    LubaId::min_masdar(&MasdarLuba::Steam(APPID_ELDEN), "ELDEN RING")
}

fn luba_hollow() -> LubaId {
    LubaId::min_masdar(&MasdarLuba::Steam(APPID_HOLLOW), "Hollow Knight")
}

fn iftah(jidhr: &Path) -> Dhakira {
    match Dhakira::min_masar(&jidhr.join("dhakira.db")) {
        Ok(dhakira) => dhakira,
        Err(khata) => panic!("the memory would not open: {khata}"),
    }
}

fn jalsa(jidhr: &Path, luba: LubaId, ism: &str) -> DhakiratTabaqa {
    DhakiratTabaqa::jadeeda(iftah(jidhr), luba).bi_ism_luba(ism)
}

/// Plays one session: ask the memory, translate only on a miss, record what
/// was translated. This is the loop the overlay runs, minus the drawing.
fn ishghal(
    tabaqa: &mut DhakiratTabaqa,
    mutarjim: &mut MutarjimMahalli,
    sutur: &[&str],
    thiqa: ThiqatQira,
) {
    for satr in sutur {
        let radd = match tabaqa.istafhim(satr) {
            Ok(radd) => radd,
            Err(khata) => panic!("the memory refused a lookup: {khata}"),
        };
        if radd.majjaniya() {
            continue;
        }
        let arabi = mutarjim.tarjim(satr);
        let mulahaza = MulahazaTabaqa::jadeeda(*satr, arabi, tabaqa.luba(), thiqa)
            .bi_tasnif(TasnifNass::Hiwar)
            .bi_mintaqa("شريط الحوار")
            .bi_muharrikayn(Some(QARI.to_owned()), Some(MUZAWWID.to_owned()));
        match tabaqa.sajjil(&mulahaza) {
            Ok(true) => {}
            Ok(false) => panic!("a fixture line was rejected as unusable: {satr}"),
            Err(khata) => panic!("the memory refused a write: {khata}"),
        }
    }
}

fn miftah(bidhra: u8) -> MiftahKhass {
    MiftahKhass::min_bayt(&[bidhra; 32])
}

fn huwiya(khass: &MiftahKhass) -> MusahimId {
    let mut nass = String::with_capacity(64);
    for bayt in khass.aam().bayt() {
        use std::fmt::Write as _;
        let _ = write!(nass, "{bayt:02x}");
    }
    match MusahimId::jadeed(nass) {
        Ok(id) => id,
        Err(khata) => panic!("a key fingerprint is not a contributor identity: {khata}"),
    }
}

/// A count as the width every statistic is kept in.
fn adad(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

fn muajjal() -> tempfile::TempDir {
    match tempfile::tempdir() {
        Ok(dalil) => dalil,
        Err(khata) => panic!("no temporary directory: {khata}"),
    }
}

// ---------------------------------------------------------------------------

/// The same recognized string, read many times, costs exactly one translation.
#[test]
fn satr_maqru_marratan_thaniya_la_yukallif_shayan() {
    let dalil = muajjal();
    let mut tabaqa = jalsa(dalil.path(), luba_elden(), "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());

    let sutur = jalsa_qira();
    ishghal(&mut tabaqa, &mut mutarjim, &sutur, ThiqatQira::maqisa(92));

    let mumayyaza = adad(azwaj().len());
    assert_eq!(
        mutarjim.nida, mumayyaza,
        "{} readings of {mumayyaza} distinct lines cost {} translations",
        sutur.len(),
        mutarjim.nida
    );
    assert_eq!(tabaqa.ihsaat().tarjamat_madfua(), mumayyaza);
    assert_eq!(tabaqa.ihsaat().istifsarat, adad(sutur.len()));
    // Every repeat inside the session was answered without touching the file.
    assert_eq!(
        tabaqa.ihsaat().isabat_jalsa,
        adad(sutur.len()) - mumayyaza,
        "{}",
        tabaqa.ihsaat().wasf()
    );
}

/// A second run of the same game costs nothing at all.
#[test]
fn tashghil_thani_lil_luba_nafsiha_la_yukallif_shayan() {
    let dalil = muajjal();
    let sutur = jalsa_qira();

    {
        let mut tabaqa = jalsa(dalil.path(), luba_elden(), "ELDEN RING");
        let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
        ishghal(&mut tabaqa, &mut mutarjim, &sutur, ThiqatQira::maqisa(92));
        assert_eq!(mutarjim.nida, adad(azwaj().len()));
    }

    // A new process, a new session, the same file on disk.
    let mut tabaqa = jalsa(dalil.path(), luba_elden(), "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut tabaqa, &mut mutarjim, &sutur, ThiqatQira::maqisa(92));

    assert_eq!(mutarjim.nida, 0, "the second run paid for {} lines", mutarjim.nida);
    assert_eq!(tabaqa.ihsaat().tarjamat_madfua(), 0);
    assert_eq!(
        tabaqa.ihsaat().isabat_dhakira,
        adad(azwaj().len()),
        "{}",
        tabaqa.ihsaat().wasf()
    );
}

/// A share from another player supplies lines the importing machine never saw.
#[test]
fn dhakirat_laaib_akhar_tuzawwid_asturan_lam_tura() {
    let dalil_awwal = muajjal();
    let dalil_thani = muajjal();
    let luba = luba_elden();

    // Player one walks past every line.
    let mut awwal = jalsa(dalil_awwal.path(), luba, "ELDEN RING");
    let mut mutarjim_awwal = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut awwal, &mut mutarjim_awwal, &jalsa_qira(), ThiqatQira::maqisa(92));
    assert_eq!(mutarjim_awwal.nida, adad(azwaj().len()));

    // Player two has only ever seen the save prompt.
    let mut thani = jalsa(dalil_thani.path(), luba, "ELDEN RING");
    let mut mutarjim_thani = MutarjimMahalli::jadeed(&azwaj());
    ishghal(
        &mut thani,
        &mut mutarjim_thani,
        &["Save and Quit"],
        ThiqatQira::maqisa(88),
    );
    assert_eq!(mutarjim_thani.nida, 1);

    // Player one shares, having been shown exactly what leaves.
    let khass = miftah(7);
    let musawwada =
        match mushtaraka::ijma(awwal.dhakira(), luba, "ELDEN RING", KhiyaratMusharaka::iftiradiya())
        {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    assert_eq!(musawwada.adad(), azwaj().len());
    assert_eq!(musawwada.adad_maqis(), azwaj().len());
    assert_eq!(musawwada.tahdheerat(), vec![TahdheerMusharaka::MuhtawaShakhsi]);

    let mut iqrarat = IqraratMusharaka::jadeeda();
    iqrarat.aqirr(TahdheerMusharaka::MuhtawaShakhsi);
    let idhn = match musawwada.idhn(&iqrarat) {
        Ok(idhn) => idhn,
        Err(khata) => panic!("the permit was refused: {khata}"),
    };
    let bayt = match mushtaraka::saddir(
        &musawwada,
        idhn,
        huwiya(&khass),
        &khass,
        "2026-09-05T10:00:00Z".to_owned(),
    ) {
        Ok(bayt) => bayt,
        Err(khata) => panic!("writing the share refused: {khata}"),
    };

    // Player two reads it, verifying against the key they expect.
    let huzma = match mushtaraka::istawrid(&bayt, &khass.aam()) {
        Ok(huzma) => huzma,
        Err(khata) => panic!("reading the share refused: {khata}"),
    };
    assert_eq!(huzma.luba(), luba);
    assert_eq!(huzma.tarwisa().adad, adad(azwaj().len()));
    assert_eq!(huzma.tarwisa().adad_maqis, adad(azwaj().len()));

    let mut dhakira_thani = thani.ila_dhakira();
    let taqreer =
        match mushtaraka::idmij(&mut dhakira_thani, &huzma, KhiyaratMusharaka::iftiradiya()) {
            Ok(taqreer) => taqreer,
            Err(khata) => panic!("the merge refused: {khata}"),
        };
    assert_eq!(taqreer.sujjilat, adad(azwaj().len()));
    assert_eq!(taqreer.marfuda, 0);
    assert_eq!(taqreer.talifa, 0);

    // Now player two plays the whole game and pays nothing.
    let mut thani =
        DhakiratTabaqa::jadeeda(dhakira_thani, luba).bi_ism_luba("ELDEN RING");
    let mut mutarjim_thani = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut thani, &mut mutarjim_thani, &jalsa_qira(), ThiqatQira::maqisa(88));
    assert_eq!(
        mutarjim_thani.nida, 0,
        "the importing machine still paid for {} line(s): {}",
        mutarjim_thani.nida,
        thani.ihsaat().wasf()
    );

    // And what they got is labelled for what it is.
    let radd = match thani.istafhim("The old road north is closed") {
        Ok(radd) => radd,
        Err(khata) => panic!("the memory refused a lookup: {khata}"),
    };
    match radd {
        taarib_tarjama::mulahazat::RaddTabaqa::Jahiza { naw, thiqa, .. } => {
            assert_eq!(naw, NawAsl::Mulahaza);
            assert_eq!(thiqa, ThiqatQira::maqisa(92));
        }
        taarib_tarjama::mulahazat::RaddTabaqa::Majhula => {
            panic!("the imported line came back unknown");
        }
    }
}

/// A share is keyed per game: importing ELDEN RING's readings does not make
/// Hollow Knight's lines free.
#[test]
fn al_hissa_makhtuma_bil_luba() {
    let dalil = muajjal();
    let luba = luba_elden();
    let mut tabaqa = jalsa(dalil.path(), luba, "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut tabaqa, &mut mutarjim, &jalsa_qira(), ThiqatQira::maqisa(92));

    let dhakira = tabaqa.ila_dhakira();
    let musawwada = match mushtaraka::ijma(
        &dhakira,
        luba_hollow(),
        "Hollow Knight",
        KhiyaratMusharaka::iftiradiya(),
    ) {
        Ok(musawwada) => musawwada,
        Err(khata) => panic!("gathering refused: {khata}"),
    };
    assert_eq!(musawwada.adad(), 0, "another game's readings leaked into the share");
    assert!(musawwada.tahdheerat().is_empty());

    let khass = miftah(9);
    let idhn = match musawwada.idhn(&IqraratMusharaka::jadeeda()) {
        Ok(idhn) => idhn,
        Err(khata) => panic!("the permit was refused for an empty draft: {khata}"),
    };
    let natija = mushtaraka::saddir(
        &musawwada,
        idhn,
        huwiya(&khass),
        &khass,
        "2026-09-05T10:00:00Z".to_owned(),
    );
    assert!(natija.is_err(), "an empty share was written");
}

/// An observation never displaces text a human reviewed, however confident.
#[test]
fn al_mulahaza_la_tuzih_nassan_rajaahu_insan() {
    let dalil = muajjal();
    let luba = luba_elden();
    let mut dhakira = iftah(dalil.path());

    let injilizi = "The old road north is closed";
    let bashari = "الطريق الشمالي القديم مغلق";
    let musahim = match MusahimId::jadeed("a".repeat(64)) {
        Ok(id) => id,
        Err(khata) => panic!("bad fixture identity: {khata}"),
    };
    if let Err(khata) = dhakira.sajjil(&QaydJadid {
        masdar: injilizi.to_owned(),
        hadaf: bashari.to_owned(),
        tasnif: TasnifNass::Hiwar,
        mashru: Some("mashru-elden".to_owned()),
        luba: Some(luba.to_string()),
        ism_luba: Some("ELDEN RING".to_owned()),
        siyaq: None,
        nasq_masdar: Vec::new(),
        nasq_hadaf: Vec::new(),
        asl: AslQayd::Bashari { musahim: Some(musahim) },
    }) {
        panic!("the reviewed pair would not store: {khata}");
    }

    // A perfectly confident, repeatedly corroborated reading of the same line.
    let mut tabaqa = DhakiratTabaqa::jadeeda(dhakira, luba);
    for _ in 0..40 {
        let mulahaza = MulahazaTabaqa::jadeeda(
            injilizi,
            "الطريق القديم شمالًا مغلق",
            luba,
            ThiqatQira::maqisa(100),
        )
        .bi_tasnif(TasnifNass::Hiwar);
        // A fresh session each time would be the same; the point is the file.
        if let Err(khata) = tabaqa.sajjil(&mulahaza) {
            panic!("the reading would not store: {khata}");
        }
    }

    let tatbiq = match tabaqa.dhakira().tatbiq_tamm(injilizi) {
        Ok(Some(tatbiq)) => tatbiq,
        Ok(None) => panic!("the memory lost the pair"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    };
    assert_eq!(tatbiq.qayd.hadaf, bashari, "a reading outranked reviewed text");
    assert_eq!(tatbiq.naw(), NawAsl::Bashari);
}

/// A reading of a pair a human already reviewed does not demote it, and does
/// not become human either.
#[test]
fn al_daraja_tartaqi_wa_la_tanzil() {
    let dalil = muajjal();
    let luba = luba_hollow();
    let mut dhakira = iftah(dalil.path());

    let injilizi = "Inventory is full";
    let arabi = "الحقيبة ممتلئة";
    let mulahaza =
        MulahazaTabaqa::jadeeda(injilizi, arabi, luba, ThiqatQira::maqisa(71))
            .bi_tasnif(TasnifNass::Nizam);
    if let Err(khata) = dhakira.sajjil(&mulahaza.ila_qayd()) {
        panic!("the reading would not store: {khata}");
    }

    let musahim = match MusahimId::jadeed("b".repeat(64)) {
        Ok(id) => id,
        Err(khata) => panic!("bad fixture identity: {khata}"),
    };
    // The same pair, now reviewed by a person, in a project.
    let mut qayd = mulahaza.ila_qayd();
    qayd.asl = AslQayd::Bashari { musahim: Some(musahim) };
    if let Err(khata) = dhakira.sajjil(&qayd) {
        panic!("the review would not store: {khata}");
    }

    let baad_muraja = match dhakira.tatbiq_tamm(injilizi) {
        Ok(Some(tatbiq)) => tatbiq,
        Ok(None) => panic!("the memory lost the pair"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    };
    assert_eq!(baad_muraja.naw(), NawAsl::Bashari);
    // The reading's own facts survive the promotion: how this sentence
    // started life stays true.
    assert_eq!(baad_muraja.qayd.asl.qari, None);
    assert_eq!(baad_muraja.qayd.asl.thiqa_qira, ThiqatQira::maqisa(71));

    // And observing it again does not take the review away.
    if let Err(khata) = dhakira.sajjil(&mulahaza.ila_qayd()) {
        panic!("the second reading would not store: {khata}");
    }
    let baad_qira = match dhakira.tatbiq_tamm(injilizi) {
        Ok(Some(tatbiq)) => tatbiq,
        Ok(None) => panic!("the memory lost the pair"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    };
    assert_eq!(baad_qira.naw(), NawAsl::Bashari, "a reading demoted a review");
    assert!(baad_qira.qayd.asl.muraja_bashariya);
}

/// A measured reading supersedes an unmeasured one; an unmeasured one never
/// supersedes a measured one, whatever the stand-in number would have been.
#[test]
fn al_qira_al_maqisa_tazih_ghayr_al_maqisa() {
    let dalil = muajjal();
    let luba = luba_hollow();
    let mut dhakira = iftah(dalil.path());
    let injilizi = "Save and Quit";
    let arabi = "احفظ واخرج";

    let sajjil = |dhakira: &mut Dhakira, thiqa: ThiqatQira| {
        let mulahaza = MulahazaTabaqa::jadeeda(injilizi, arabi, luba, thiqa);
        if let Err(khata) = dhakira.sajjil(&mulahaza.ila_qayd()) {
            panic!("the reading would not store: {khata}");
        }
    };
    let thiqa = |dhakira: &Dhakira| match dhakira.tatbiq_tamm(injilizi) {
        Ok(Some(tatbiq)) => tatbiq.qayd.asl.thiqa_qira,
        Ok(None) => panic!("the memory lost the pair"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    };

    // An engine that measures nothing sees it first.
    sajjil(&mut dhakira, ThiqatQira::Ghayr);
    assert_eq!(thiqa(&dhakira), ThiqatQira::Ghayr);

    // An engine that does measure sees it, and its number takes over.
    sajjil(&mut dhakira, ThiqatQira::maqisa(64));
    assert_eq!(thiqa(&dhakira), ThiqatQira::maqisa(64));

    // The unmeasured engine sees it again and does not take it back.
    sajjil(&mut dhakira, ThiqatQira::Ghayr);
    assert_eq!(thiqa(&dhakira), ThiqatQira::maqisa(64));

    // A better measurement supersedes; a worse one does not.
    sajjil(&mut dhakira, ThiqatQira::maqisa(97));
    assert_eq!(thiqa(&dhakira), ThiqatQira::maqisa(97));
    sajjil(&mut dhakira, ThiqatQira::maqisa(20));
    assert_eq!(thiqa(&dhakira), ThiqatQira::maqisa(97));

    // Five sightings were recorded, all of them.
    match dhakira.tatbiq_tamm(injilizi) {
        Ok(Some(tatbiq)) => assert_eq!(tatbiq.qayd.asl.mushahadat, 5),
        Ok(None) => panic!("the memory lost the pair"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    }
}

/// An unmeasured reading clears no floor, so the default export leaves it out
/// and including it is an acknowledged decision.
#[test]
fn ghayr_al_maqisa_tustathna_bil_iftirad() {
    let dalil = muajjal();
    let luba = luba_hollow();
    let mut tabaqa = jalsa(dalil.path(), luba, "Hollow Knight");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut tabaqa, &mut mutarjim, &jalsa_qira(), ThiqatQira::Ghayr);

    let dhakira = tabaqa.ila_dhakira();
    let iftiradi =
        match mushtaraka::ijma(&dhakira, luba, "Hollow Knight", KhiyaratMusharaka::iftiradiya()) {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    assert_eq!(iftiradi.adad(), 0, "unmeasured readings were shared by default");

    let khiyarat = KhiyaratMusharaka::iftiradiya().maa_ghayr_maqisa();
    let mawsa = match mushtaraka::ijma(&dhakira, luba, "Hollow Knight", khiyarat) {
        Ok(musawwada) => musawwada,
        Err(khata) => panic!("gathering refused: {khata}"),
    };
    assert_eq!(mawsa.adad(), azwaj().len());
    assert_eq!(mawsa.adad_maqis(), 0);
    assert_eq!(
        mawsa.tahdheerat(),
        vec![TahdheerMusharaka::MuhtawaShakhsi, TahdheerMusharaka::LamTuqas]
    );

    // Acknowledging only one of the two is not consent.
    let mut naqisa = IqraratMusharaka::jadeeda();
    naqisa.aqirr(TahdheerMusharaka::MuhtawaShakhsi);
    assert!(mawsa.idhn(&naqisa).is_err(), "a permit was minted with a warning outstanding");
}

/// Nothing leaves without consent, and a permit cannot be spent on a
/// different payload.
#[test]
fn la_tughadir_bayanat_bila_idhn_mutabiq() {
    let dalil = muajjal();
    let luba = luba_elden();
    let mut tabaqa = jalsa(dalil.path(), luba, "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut tabaqa, &mut mutarjim, &["Save and Quit"], ThiqatQira::maqisa(90));

    let saghira =
        match mushtaraka::ijma(tabaqa.dhakira(), luba, "ELDEN RING", KhiyaratMusharaka::iftiradiya())
        {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    assert_eq!(saghira.adad(), 1);
    // No acknowledgement at all: the screen-content warning is outstanding.
    assert!(saghira.idhn(&IqraratMusharaka::jadeeda()).is_err());

    let mut iqrarat = IqraratMusharaka::jadeeda();
    iqrarat.aqirr(TahdheerMusharaka::MuhtawaShakhsi);
    let idhn_saghir = match saghira.idhn(&iqrarat) {
        Ok(idhn) => idhn,
        Err(khata) => panic!("the permit was refused: {khata}"),
    };

    // The player keeps playing and the pile grows under the permit.
    ishghal(&mut tabaqa, &mut mutarjim, &jalsa_qira(), ThiqatQira::maqisa(90));
    let kabira =
        match mushtaraka::ijma(tabaqa.dhakira(), luba, "ELDEN RING", KhiyaratMusharaka::iftiradiya())
        {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    assert_eq!(kabira.adad(), azwaj().len());
    assert_ne!(kabira.basma(), saghira.basma());

    let khass = miftah(3);
    let natija = mushtaraka::saddir(
        &kabira,
        idhn_saghir,
        huwiya(&khass),
        &khass,
        "2026-09-05T10:00:00Z".to_owned(),
    );
    assert!(natija.is_err(), "a permit for one reading wrote eight");
}

/// A share carries observations and nothing else, whatever else the memory
/// holds.
#[test]
fn al_hissa_la_tahmil_illa_al_mulahazat() {
    let dalil = muajjal();
    let luba = luba_elden();
    let mut dhakira = iftah(dalil.path());
    let musahim = match MusahimId::jadeed("c".repeat(64)) {
        Ok(id) => id,
        Err(khata) => panic!("bad fixture identity: {khata}"),
    };

    let asas = |masdar: &str, hadaf: &str, asl: AslQayd| QaydJadid {
        masdar: masdar.to_owned(),
        hadaf: hadaf.to_owned(),
        tasnif: TasnifNass::Qaima,
        mashru: None,
        luba: Some(luba.to_string()),
        ism_luba: Some("ELDEN RING".to_owned()),
        siyaq: None,
        nasq_masdar: Vec::new(),
        nasq_hadaf: Vec::new(),
        asl,
    };

    let sufuf = [
        asas("Continue", "متابعة", AslQayd::Bashari { musahim: Some(musahim) }),
        asas(
            "New Game",
            "لعبة جديدة",
            AslQayd::AaliFaqat { muzawwid: Some(MUZAWWID.to_owned()), thiqa: Some(0.9) },
        ),
        asas(
            "Settings",
            "الإعدادات",
            AslQayd::Mulahaza {
                qari: Some(QARI.to_owned()),
                muzawwid: Some(MUZAWWID.to_owned()),
                thiqa: ThiqatQira::maqisa(95),
            },
        ),
    ];
    for qayd in &sufuf {
        if let Err(khata) = dhakira.sajjil(qayd) {
            panic!("a fixture row would not store: {khata}");
        }
    }

    let musawwada =
        match mushtaraka::ijma(&dhakira, luba, "ELDEN RING", KhiyaratMusharaka::iftiradiya()) {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    assert_eq!(musawwada.adad(), 1, "the share carried more than the readings");
    assert_eq!(musawwada.qayyid().first().map(|q| q.asl.as_str()), Some("Settings"));

    // And the memory really did hold all three.
    assert_eq!(dhakira.adad_luba(&luba.to_string(), NawAsl::Bashari).unwrap_or(0), 1);
    assert_eq!(dhakira.adad_luba(&luba.to_string(), NawAsl::Aali).unwrap_or(0), 1);
    assert_eq!(dhakira.adad_luba(&luba.to_string(), NawAsl::Mulahaza).unwrap_or(0), 1);
}

/// Imported readings land as readings, whatever the importing machine does
/// with them afterwards — a share cannot manufacture provenance.
#[test]
fn al_mustawrad_yabqa_mulahaza() {
    let dalil_awwal = muajjal();
    let dalil_thani = muajjal();
    let luba = luba_elden();

    let mut awwal = jalsa(dalil_awwal.path(), luba, "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut awwal, &mut mutarjim, &jalsa_qira(), ThiqatQira::maqisa(92));

    let khass = miftah(11);
    let bayt = saddir_kamil(awwal.dhakira(), luba, &khass);
    let huzma = match mushtaraka::istawrid(&bayt, &khass.aam()) {
        Ok(huzma) => huzma,
        Err(khata) => panic!("reading the share refused: {khata}"),
    };

    let mut thani = iftah(dalil_thani.path());
    if let Err(khata) = mushtaraka::idmij(&mut thani, &huzma, KhiyaratMusharaka::iftiradiya()) {
        panic!("the merge refused: {khata}");
    }

    assert_eq!(thani.adad_luba(&luba.to_string(), NawAsl::Bashari).unwrap_or(9), 0);
    assert_eq!(thani.adad_luba(&luba.to_string(), NawAsl::Aali).unwrap_or(9), 0);
    assert_eq!(
        thani.adad_luba(&luba.to_string(), NawAsl::Mulahaza).unwrap_or(0),
        adad(azwaj().len())
    );

    // The sharer's sighting counts are not taken as stated: one import is one
    // sighting, however many the file claims.
    match thani.tatbiq_tamm("Save and Quit") {
        Ok(Some(tatbiq)) => {
            assert_eq!(tatbiq.naw(), NawAsl::Mulahaza);
            assert_eq!(tatbiq.qayd.asl.mushahadat, 1);
        }
        Ok(None) => panic!("an imported line is missing"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    }
}

/// A share whose bytes were changed after signing is refused, and so is one
/// checked against another key.
#[test]
fn hissa_muabbatha_turfad() {
    let dalil = muajjal();
    let luba = luba_elden();
    let mut tabaqa = jalsa(dalil.path(), luba, "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(&mut tabaqa, &mut mutarjim, &jalsa_qira(), ThiqatQira::maqisa(92));

    let khass = miftah(13);
    let bayt = saddir_kamil(tabaqa.dhakira(), luba, &khass);
    assert!(mushtaraka::istawrid(&bayt, &khass.aam()).is_ok());

    // Somebody else's key.
    assert!(
        mushtaraka::istawrid(&bayt, &miftah(14).aam()).is_err(),
        "a share verified against a key that did not sign it"
    );

    // One byte of the compressed body.
    let mut mubaddal = bayt.clone();
    let akhir = mubaddal.len().saturating_sub(3);
    if let Some(bayta) = mubaddal.get_mut(akhir) {
        *bayta ^= 0xFF;
    }
    assert!(
        mushtaraka::istawrid(&mubaddal, &khass.aam()).is_err(),
        "a share with a changed body was accepted"
    );

    // The header's own count, which the signature covers.
    let Some(fasl) = bayt.iter().position(|harf| *harf == b'\n') else {
        panic!("the share has no header line");
    };
    let tarwisa = String::from_utf8_lossy(bayt.get(..fasl).unwrap_or_default()).to_string();
    let matlub = format!("\"adad\":{}", azwaj().len());
    assert!(tarwisa.contains(&matlub), "the header does not read as expected: {tarwisa}");
    let mut mazur = tarwisa.replacen(&matlub, "\"adad\":9", 1).into_bytes();
    mazur.extend_from_slice(bayt.get(fasl..).unwrap_or_default());
    assert_ne!(mazur, bayt);
    assert!(
        mushtaraka::istawrid(&mazur, &khass.aam()).is_err(),
        "a share with an edited header was accepted"
    );
}

/// Gathers and signs a whole share, acknowledging every warning it raises.
fn saddir_kamil(dhakira: &Dhakira, luba: LubaId, khass: &MiftahKhass) -> Vec<u8> {
    let musawwada =
        match mushtaraka::ijma(dhakira, luba, "ELDEN RING", KhiyaratMusharaka::iftiradiya()) {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    let mut iqrarat = IqraratMusharaka::jadeeda();
    for tahdheer in musawwada.tahdheerat() {
        iqrarat.aqirr(tahdheer);
    }
    let idhn = match musawwada.idhn(&iqrarat) {
        Ok(idhn) => idhn,
        Err(khata) => panic!("the permit was refused: {khata}"),
    };
    match mushtaraka::saddir(
        &musawwada,
        idhn,
        huwiya(khass),
        khass,
        "2026-09-05T10:00:00Z".to_owned(),
    ) {
        Ok(bayt) => bayt,
        Err(khata) => panic!("writing the share refused: {khata}"),
    }
}

/// A memory written by the build before the third kind existed upgrades
/// without relabelling anything, and its rows come back as what they were.
///
/// Builds a genuine schema-1 file — migration 1's own statements, its own
/// checksum, rows inserted through the columns that build knew — and opens it
/// with this build. That is the only way to exercise migration 2's `UPDATE`
/// against populated data: a fresh file runs both migrations over an empty
/// table and proves nothing about existing rows.
#[test]
fn dhakirat_isdar_awwal_turaqqa_bila_taghyeer_asl() {
    let dalil = muajjal();
    let masar = dalil.path().join("qadeema.db");

    {
        let ittisal = match rusqlite::Connection::open(&masar) {
            Ok(ittisal) => ittisal,
            Err(khata) => panic!("SQLite would not open the file: {khata}"),
        };
        let Some(hijra) = taarib_tarjama::dhakira::HIJRAT_DHAKIRA.first() else {
            panic!("this build defines no migrations");
        };
        assert_eq!(hijra.raqm, 1);
        let bina = format!(
            "CREATE TABLE hijrat_dhakira (
                 raqm  INTEGER PRIMARY KEY,
                 ism   TEXT NOT NULL,
                 basma TEXT NOT NULL,
                 waqt  TEXT NOT NULL
             ) STRICT;
             {}
             INSERT INTO hijrat_dhakira (raqm, ism, basma, waqt)
             VALUES (1, '{}', '{}', '2026-01-01T00:00:00.000Z');

             INSERT INTO miftah_bahth (miftah, tul) VALUES ('continue', 8);
             INSERT INTO qayd (
                 miftah, masdar, hadaf, hadaf_muwahhad, tasnif,
                 muraja_bashariya, musahim, waqt)
             VALUES (1, 'Continue', 'متابعة', 'متابعه', 'qaima', 1,
                     '{}', '2026-01-01T00:00:00.000Z');

             INSERT INTO miftah_bahth (miftah, tul) VALUES ('new game', 8);
             INSERT INTO qayd (
                 miftah, masdar, hadaf, hadaf_muwahhad, tasnif,
                 muraja_bashariya, muzawwid, thiqa, waqt)
             VALUES (2, 'New Game', 'لعبة جديدة', 'لعبه جديده', 'qaima', 0,
                     'mahalli', 0.9, '2026-01-01T00:00:00.000Z');",
            hijra.jumal,
            hijra.ism,
            hijra.basma(),
            "d".repeat(64),
        );
        if let Err(khata) = ittisal.execute_batch(&bina) {
            panic!("the schema-1 fixture would not build: {khata}");
        }
    }

    let dhakira = match Dhakira::min_masar(&masar) {
        Ok(dhakira) => dhakira,
        Err(khata) => panic!("the upgrade refused: {khata}"),
    };

    let bashari = match dhakira.tatbiq_tamm("Continue") {
        Ok(Some(tatbiq)) => tatbiq,
        Ok(None) => panic!("the reviewed row was lost in the upgrade"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    };
    assert_eq!(bashari.naw(), NawAsl::Bashari);
    assert!(bashari.qayd.asl.muraja_bashariya);
    assert_eq!(bashari.qayd.asl.thiqa_qira, ThiqatQira::Ghayr);
    assert_eq!(bashari.qayd.asl.mushahadat, 0);

    let aali = match dhakira.tatbiq_tamm("New Game") {
        Ok(Some(tatbiq)) => tatbiq,
        Ok(None) => panic!("the machine row was lost in the upgrade"),
        Err(khata) => panic!("the lookup refused: {khata}"),
    };
    // The row a build with no third kind wrote must not become a reading.
    assert_eq!(aali.naw(), NawAsl::Aali);
    assert!(!aali.qayd.asl.muraja_bashariya);
    assert_eq!(aali.qayd.asl.qari, None);
    assert_eq!(aali.qayd.asl.thiqa_qira, ThiqatQira::Ghayr);

    // Nothing from a schema-1 memory is shareable, because nothing in one is
    // an observation.
    let luba = luba_elden();
    let musawwada =
        match mushtaraka::ijma(&dhakira, luba, "ELDEN RING", KhiyaratMusharaka::iftiradiya()) {
            Ok(musawwada) => musawwada,
            Err(khata) => panic!("gathering refused: {khata}"),
        };
    assert_eq!(musawwada.adad(), 0);
}

/// The folded key is what makes a repeat a repeat, so a reading that differs
/// only in case or spacing costs nothing the second time either.
#[test]
fn al_takrar_yuqas_bil_miftah_al_muwahhad() {
    assert_eq!(miftah_muwahhad("Save  and Quit "), miftah_muwahhad("save and quit"));

    let dalil = muajjal();
    let mut tabaqa = jalsa(dalil.path(), luba_elden(), "ELDEN RING");
    let mut mutarjim = MutarjimMahalli::jadeed(&azwaj());
    ishghal(
        &mut tabaqa,
        &mut mutarjim,
        &["Save and Quit", "SAVE AND QUIT", "Save  and  Quit"],
        ThiqatQira::maqisa(80),
    );
    assert_eq!(mutarjim.nida, 1, "three renderings of one line cost {}", mutarjim.nida);
}
