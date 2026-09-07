//! What a three-hour dialogue session costs, asserted rather than claimed.
//!
//! Every test here is about a number: how many times a translation provider is
//! asked for something. That number is what a player pays, in money on a metered
//! provider and in latency on a local one, and it is the only measure by which
//! the overlay's session layer is better or worse than reading the screen every
//! frame and sending whatever comes back.
//!
//! ## What is real here and what is not
//!
//! **Real**: [`taarib_tabaqa::qissa::Qissa`], the line tracker, the settle
//! policy, the refresh governor, the translation memory contract, the reading
//! history, and the perceptual-hash gate over genuinely synthesized pixels.
//! Every count asserted below is produced by the shipped code paths.
//!
//! **Not real**: the recognizer and the provider. There is no GPU and no game
//! here, so the recognized lines are constructed rather than read off a frame,
//! and the provider is a counting implementation of
//! [`taarib_tabaqa::mutarjim::MutarjimTabaqa`] rather than a model. One test —
//! [`nisfa_alitar_wa_ma_khalfah`] — does go through a real
//! [`taarib_tabaqa::qira::Qari`], but that recognizer decodes a band position
//! painted into the capture rather than reading text, so it exercises the frame
//! path and the change gate and says nothing whatever about OCR accuracy.
//!
//! That is the honest boundary of what can be proven without a display: these
//! tests prove what the session does *with* recognized lines, not that
//! recognition produces the right ones. `tests/siyaq_mahalli.rs` — which needs
//! the `tarjama` feature — is where a real provider is driven end to end.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::nass::TasnifNass;
use taarib_tabaqa::iltiqat_shasha::SuraMultaqata;
use taarib_tabaqa::khata::KhataTabaqa;
use taarib_tabaqa::manatiq::{MuarrifMintaqa, QaidatTarjama};
use taarib_tabaqa::mutarjim::{
    DhakiraTabaqa, MutarjimTabaqa, QaydTabaqa, RaddSatr, TalabDhakira, TalabSatr,
};
use taarib_tabaqa::qissa::{HalatDaf, HalatKhayt, KhaytQissa, KhiyaratQissa, Munassiq, Qissa};
use taarib_tabaqa::tatabbu::QiraaMulahaza;
use taarib_tabaqa::wajiha::{MeezaniyatItar, MustatilBiksel, SighatSath, WasfSath};
use taarib_tabaqa::watira::{MunazzimWatira, NAFIDHAT_TADAHWUR, TaghyeerWatira};

/// The surface every test positions against.
const SATH: WasfSath = WasfSath {
    ard: 1920,
    irtifa: 1080,
    sigha: SighatSath::Bgra8,
    sirgb: true,
};

/// The subtitle strip, in surface pixels.
const SUNDUQ: MustatilBiksel = MustatilBiksel {
    yasar: 400,
    aala: 900,
    ard: 1120,
    irtifa: 40,
};

/// The default poll interval, which every synthetic pass advances by.
const KHUTWA_MIKRO: u64 = 250_000;

/// The region every test reads from.
const MINTAQA: MuarrifMintaqa = MuarrifMintaqa::min_raqm(1);

// ---------------------------------------------------------------------------
// Instruments
// ---------------------------------------------------------------------------

/// One recorded request: the source, the neighbouring lines, the scene.
type Talab = (String, Vec<String>, String);

/// What a counting translator has been asked, shared between the session that
/// holds it and the test that reads it.
#[derive(Debug, Default)]
struct HalatAadd {
    talabat: AtomicU64,
    sijill: Mutex<Vec<Talab>>,
}

/// A translator that counts what it was asked and remembers the context.
///
/// Not a stand-in for a model — it produces no plausible Arabic and does not
/// pretend to. It exists to make the one number these tests are about
/// observable, and to let the surrounding-context assertion look at exactly what
/// a provider would have received.
#[derive(Debug, Default, Clone)]
struct MutarjimAadd(Arc<HalatAadd>);

impl MutarjimAadd {
    fn adad(&self) -> u64 {
        self.0.talabat.load(Ordering::Relaxed)
    }

    fn sijill(&self) -> Vec<Talab> {
        self.0.sijill.lock().clone()
    }

    fn usul(&self) -> Vec<String> {
        self.0
            .sijill
            .lock()
            .iter()
            .map(|(asl, _, _)| asl.clone())
            .collect()
    }
}

impl MutarjimTabaqa for MutarjimAadd {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait's signature is `&str`, and an impl cannot narrow it to `&'static \
                  str`; a provider that names itself after its own configuration needs the \
                  borrow"
    )]
    fn ism(&self) -> &str {
        "counting"
    }

    fn mutah(&self) -> bool {
        true
    }

    fn tarjim(&self, talab: &TalabSatr<'_>) -> Result<RaddSatr, KhataTabaqa> {
        let _ = self.0.talabat.fetch_add(1, Ordering::Relaxed);
        self.0.sijill.lock().push((
            talab.asl.to_owned(),
            talab.siyaq.jiwar.clone(),
            talab.siyaq.mashhad.clone().unwrap_or_default(),
        ));
        // The Arabic is a marker, not a translation. Nothing downstream of the
        // session reads it in these tests, and a fake sentence that looked like
        // a translation would be the one thing in this file that could be
        // mistaken for a real one.
        Ok(RaddSatr::aaliya(format!("«{}»", talab.asl)))
    }
}

/// A translation memory that survives a session, on disk.
///
/// The shipped memory is the workspace's `SQLite` one; this is a file-backed
/// implementation of the same contract, written here so that "a second
/// playthrough costs nothing" can be asserted against a genuinely new process
/// state — a new session, a new tracker, a new lookup table read back off a
/// file — rather than against a map that never went away.
///
/// It keys on the source text alone rather than on the game as well, because
/// every test that uses it runs one game. The shipped memory keys on both and
/// scores a same-game bonus on top; nothing here depends on that difference.
#[derive(Debug)]
struct DhakiraMalaf {
    masar: PathBuf,
    qiyud: Mutex<BTreeMap<String, String>>,
}

impl DhakiraMalaf {
    fn iftah(masar: PathBuf) -> Self {
        let mut qiyud = BTreeMap::new();
        if let Ok(nass) = std::fs::read_to_string(&masar) {
            for satr in nass.lines() {
                if let Some((asl, arabi)) = satr.split_once('\t') {
                    let _ = qiyud.insert(asl.to_owned(), arabi.to_owned());
                }
            }
        }
        Self {
            masar,
            qiyud: Mutex::new(qiyud),
        }
    }

    fn iktub(&self) -> std::io::Result<()> {
        let qiyud = self.qiyud.lock();
        let mut nass = String::new();
        for (asl, arabi) in qiyud.iter() {
            nass.push_str(asl);
            nass.push('\t');
            nass.push_str(arabi);
            nass.push('\n');
        }
        std::fs::write(&self.masar, nass)
    }
}

impl DhakiraTabaqa for DhakiraMalaf {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait's signature is `&str`; see the note on the translator above"
    )]
    fn ism(&self) -> &str {
        "file"
    }

    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr> {
        self.qiyud
            .lock()
            .get(talab.asl)
            .map(|arabi| RaddSatr::aaliya(arabi.clone()))
    }

    fn sajjil(&self, qayd: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa> {
        let _ = self
            .qiyud
            .lock()
            .insert(qayd.asl.to_owned(), qayd.arabi.to_owned());
        self.iktub().map_err(|sabab| KhataTabaqa::KhataMalaf {
            masar: self.masar.clone(),
            sabab,
        })
    }

    fn daaima(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The identity of the fixture these tests read.
///
/// [`MasdarLuba::Yadawi`] on purpose: it is the variant for something a user
/// pointed at directly, and it is the only one that does not assert a store
/// entry. **Nothing here is a real game.** No Unity, Unreal, Ren'Py, RPG Maker
/// or GameMaker title is involved; the readings below are constructed text.
fn luba() -> LubaId {
    LubaId::min_masdar(
        &MasdarLuba::Yadawi("taarib-qissa-fixture".to_owned()),
        ISM_LUBA,
    )
}

/// The fixture's display name.
const ISM_LUBA: &str = "Synthetic Frames (a test fixture, not a game)";

/// Options with the settle policy left at its shipped defaults.
fn khiyarat() -> KhiyaratQissa {
    KhiyaratQissa {
        tasnif: TasnifNass::Hiwar,
        yasjil: false,
        ..KhiyaratQissa::iftiradiya()
    }
}

/// A session with a counting translator and a chosen memory.
fn jalsa(mutarjim: &MutarjimAadd, dhakira: Arc<dyn DhakiraTabaqa>) -> Qissa {
    let mut qissa = Qissa::jadeeda(luba(), ISM_LUBA, khiyarat())
        .bi_mutarjim(Box::new(mutarjim.clone()))
        .bi_dhakira(dhakira)
        .bi_qari("synthetic");
    qissa.ayyin_sath(SATH);
    qissa
}

/// A fresh session-lifetime memory, for the tests that are not about
/// persistence.
fn dhakirat_jalsa() -> Arc<dyn DhakiraTabaqa> {
    Arc::new(taarib_tabaqa::mutarjim::DhakiraJalsaMushtaraka::iftiradiya())
}

/// One recognized line in the subtitle strip.
fn mulahaza(nass: &str) -> Vec<QiraaMulahaza> {
    vec![QiraaMulahaza {
        nass: nass.to_owned(),
        mawdi: SUNDUQ,
        thiqa: 92,
        maqisa: true,
    }]
}

/// Feeds a sequence of readings in, one poll interval apart.
///
/// Returns the microsecond counter after the last one, so a caller can keep
/// going from where it left off.
fn mrir(qissa: &mut Qissa, bidaya: u64, qiraat: &[&str]) -> u64 {
    let mut lahza = bidaya;
    for qiraa in qiraat {
        let _ = qissa.aalij_sutur(MINTAQA, "subtitles", lahza, &mulahaza(qiraa));
        lahza = lahza.saturating_add(KHUTWA_MIKRO);
    }
    lahza
}

/// One line of a long synthetic conversation.
///
/// Built from three coprime word lists so that a thousand consecutive lines are
/// a thousand distinct sentences that differ from their neighbours in the first
/// word as well as the middle — which is what dialogue does and what a counter
/// embedded in an otherwise identical sentence does not. A fixture whose lines
/// differed only by an index would be measuring the tracker against text no game
/// produces, and would pass for the wrong reason.
fn satr_hiwar(raqm: u32) -> String {
    const AFAAL: [&str; 7] = [
        "Take", "Follow", "Guard", "Sell", "Burn", "Forget", "Deliver",
    ];
    const ASMA: [&str; 11] = [
        "ferry", "lantern", "ledger", "pass", "cellar", "banner", "mill", "quarry", "shrine",
        "causeway", "toll",
    ];
    const AMAKIN: [&str; 13] = [
        "eastern pier",
        "north gate",
        "old mill",
        "salt marsh",
        "broken bridge",
        "upper ward",
        "tanner's row",
        "fisher's stair",
        "chapel yard",
        "dry well",
        "stone circle",
        "watch tower",
        "long barrow",
    ];
    let fil = AFAAL
        .get(usize::try_from(raqm % 7).unwrap_or(0))
        .copied()
        .unwrap_or("Take");
    let ism = ASMA
        .get(usize::try_from(raqm % 11).unwrap_or(0))
        .copied()
        .unwrap_or("ferry");
    let makan = AMAKIN
        .get(usize::try_from(raqm % 13).unwrap_or(0))
        .copied()
        .unwrap_or("north gate");
    format!("{fil} the {ism} at the {makan} before the thaw sets in.")
}

/// A flat capture of one colour, in the surface's own format.
///
/// Real bytes in the real format, so [`Qissa::hal_taghayyarat`] runs the whole
/// shipped preprocessing and hashing path over them rather than a shortcut.
fn sura(qeema: u8) -> SuraMultaqata {
    sura_bi_shareet(qeema, None)
}

/// The same capture with an optional dark band across its middle.
///
/// The band is what a line of text looks like to an 8×8 average hash: several
/// cells move across the mean at once. Its absence and presence are the two
/// pictures the gate has to tell apart.
fn sura_bi_shareet(qeema: u8, shareet: Option<u32>) -> SuraMultaqata {
    let (ard, irtifa) = (SUNDUQ.ard, SUNDUQ.irtifa);
    let sia = usize::try_from(ard.saturating_mul(irtifa).saturating_mul(4)).unwrap_or(0);
    let mut bayt = Vec::with_capacity(sia);
    for y in 0..irtifa {
        for _ in 0..ard {
            let fi_shareet = shareet.is_some_and(|aala| y >= aala && y < aala + 12);
            let qanat = if fi_shareet { 20 } else { qeema };
            bayt.extend_from_slice(&[qanat, qanat, qanat, 0xFF]);
        }
    }
    match SuraMultaqata::jadeeda(bayt, ard, irtifa, SighatSath::Bgra8, SUNDUQ) {
        Ok(sura) => sura,
        Err(khata) => panic!("the synthetic capture would not build: {khata}"),
    }
}

// ---------------------------------------------------------------------------
// One line on screen for a long time is one translation
// ---------------------------------------------------------------------------

/// Two hundred readings of the same unchanged sentence cost one request.
///
/// This is the headline number. A pipeline with no line identity sends one
/// request per recognized line, so the same subtitle held on screen for fifty
/// seconds at four polls a second is two hundred requests for one sentence. The
/// tracker gives it one identity, the settle policy asks for it once, and every
/// later reading matches the identity that is already there.
#[test]
fn satr_wahid_tarjama_wahida() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());

    let qiraat = vec!["The old road north is closed until the thaw."; 200];
    let _ = mrir(&mut qissa, 0, &qiraat);

    assert_eq!(
        mutarjim.adad(),
        1,
        "200 readings of one unchanged sentence cost {} request(s); a per-frame pipeline would \
         have sent 200",
        mutarjim.adad()
    );
    let ihsaat = qissa.ihsaat();
    assert_eq!(ihsaat.maqruaat, 200, "every reading must reach the tracker");
    assert_eq!(
        ihsaat.talabat, 1,
        "the session's own counter must agree with the provider's"
    );
    assert_eq!(
        ihsaat.muwaffara(),
        199,
        "the saving is the difference between the two counters, and both are printed"
    );
}

// ---------------------------------------------------------------------------
// A reveal is translated once, at the end
// ---------------------------------------------------------------------------

/// A sentence typed out over five polls is translated once, complete.
///
/// The failure this defends against is not "an extra request" — it is four
/// requests that each translate a fragment nobody said, followed by a fifth that
/// translates the sentence. The partial translations are not merely wasted: on a
/// pipeline that drew them, the player watches Arabic change under them four
/// times while the English is still appearing.
#[test]
fn kashf_tadreeji_tarjama_wahida() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());

    let kamil = "The old road north is closed until the thaw.";
    let kashf = [
        "The old",
        "The old road nor",
        "The old road north is",
        "The old road north is closed unt",
        kamil,
        kamil,
        kamil,
        kamil,
    ];
    let _ = mrir(&mut qissa, 0, &kashf);

    assert_eq!(
        mutarjim.adad(),
        1,
        "a five-step reveal cost {} request(s); it must cost exactly one",
        mutarjim.adad()
    );
    let usul = mutarjim.usul();
    assert_eq!(
        usul.first().map(String::as_str),
        Some(kamil),
        "the one request must carry the finished sentence, not a fragment; it carried {:?}",
        usul.first()
    );
    assert_eq!(
        qissa.mutatabbi().ihsaat().hawiyat,
        1,
        "every step of one reveal must be the same line, not {} separate ones",
        qissa.mutatabbi().ihsaat().hawiyat
    );
}

/// A genuinely different sentence ends the old line and starts a new one.
///
/// The other half of the same rule. Identity that never ends is identity that
/// translates the first line of a game and nothing else.
#[test]
fn satr_jadeed_hawiya_jadeeda() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());

    let lahza = mrir(
        &mut qissa,
        0,
        &["The old road north is closed until the thaw."; 4],
    );
    let _ = mrir(
        &mut qissa,
        lahza,
        &["Take the ferry from the eastern pier instead."; 4],
    );

    assert_eq!(
        mutarjim.adad(),
        2,
        "two distinct sentences must cost exactly two requests, not {}",
        mutarjim.adad()
    );
    assert_eq!(
        qissa.mutatabbi().ihsaat().hawiyat,
        2,
        "two sentences are two identities, not {}",
        qissa.mutatabbi().ihsaat().hawiyat
    );
}

// ---------------------------------------------------------------------------
// Consecutive lines carry context
// ---------------------------------------------------------------------------

/// The second line reaches the provider with the first one in view.
///
/// And the first reaches it with nothing, because there was nothing. A context
/// mechanism that fabricated a neighbour for the opening line of a game would be
/// telling the model something that was never said.
#[test]
fn assatr_attali_yahmil_ma_qablah() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());

    let awwal = "The old road north is closed until the thaw.";
    let thani = "Take the ferry from the eastern pier instead.";
    let thalith = "It sails at dawn, and not again until the day after.";

    let mut lahza = mrir(&mut qissa, 0, &[awwal; 3]);
    lahza = mrir(&mut qissa, lahza, &[thani; 3]);
    let _ = mrir(&mut qissa, lahza, &[thalith; 3]);

    let sijill = mutarjim.sijill();
    assert_eq!(
        sijill.len(),
        3,
        "three sentences must produce three requests"
    );

    let Some((asl_awwal, jiwar_awwal, _)) = sijill.first() else {
        panic!("the first request is missing");
    };
    assert_eq!(asl_awwal, awwal);
    assert!(
        jiwar_awwal.is_empty(),
        "the opening line has nothing before it, and its context carried {jiwar_awwal:?}"
    );

    let Some((asl_thani, jiwar_thani, _)) = sijill.get(1) else {
        panic!("the second request is missing");
    };
    assert_eq!(asl_thani, thani);
    assert_eq!(
        jiwar_thani.as_slice(),
        [awwal.to_owned()].as_slice(),
        "the second line must be translated with the first in view; it carried {jiwar_thani:?}"
    );

    let Some((asl_thalith, jiwar_thalith, mashhad)) = sijill.get(2) else {
        panic!("the third request is missing");
    };
    assert_eq!(asl_thalith, thalith);
    assert_eq!(
        jiwar_thalith.as_slice(),
        [awwal.to_owned(), thani.to_owned()].as_slice(),
        "the third line must carry both lines before it, oldest first; it carried \
         {jiwar_thalith:?}"
    );
    assert!(
        !jiwar_thalith.contains(&thalith.to_owned()),
        "a line must never appear in its own context"
    );
    assert_eq!(
        mashhad, "subtitles",
        "the region's name is what the context calls the scene it was read in"
    );
}

// ---------------------------------------------------------------------------
// A second playthrough costs nothing
// ---------------------------------------------------------------------------

/// The same frames, a new session, a memory read back off a file: no requests.
///
/// The second session shares nothing with the first but the file. Its tracker is
/// new, so every line is a new identity that settles and asks for a translation;
/// the memory is what answers all of them, which is the whole point of the
/// contract in [`taarib_tabaqa::mutarjim::DhakiraTabaqa`].
#[test]
fn shawt_thani_bila_taklifa() {
    let mujallad = std::env::temp_dir().join(format!("taarib-qissa-{}", std::process::id()));
    if let Err(khata) = std::fs::create_dir_all(&mujallad) {
        panic!("the scratch directory would not be created: {khata}");
    }
    let masar = mujallad.join("dhakira.tsv");
    let _ = std::fs::remove_file(&masar);

    let hiwar = [
        "The old road north is closed until the thaw.",
        "Take the ferry from the eastern pier instead.",
        "It sails at dawn, and not again until the day after.",
    ];

    let awwal = MutarjimAadd::default();
    {
        let dhakira: Arc<dyn DhakiraTabaqa> = Arc::new(DhakiraMalaf::iftah(masar.clone()));
        let mut qissa = jalsa(&awwal, dhakira);
        let mut lahza = 0;
        for satr in &hiwar {
            lahza = mrir(&mut qissa, lahza, &[*satr; 3]);
        }
    }
    assert_eq!(
        awwal.adad(),
        3,
        "the first run must pay for each of the three lines once, not {} time(s)",
        awwal.adad()
    );

    let thani = MutarjimAadd::default();
    {
        // A different memory *object*, opened from the same file, alongside a
        // session that has never seen any of these lines.
        let dhakira: Arc<dyn DhakiraTabaqa> = Arc::new(DhakiraMalaf::iftah(masar.clone()));
        let mut qissa = jalsa(&thani, dhakira);
        let mut lahza = 0;
        for satr in &hiwar {
            lahza = mrir(&mut qissa, lahza, &[*satr; 3]);
        }
        assert_eq!(
            qissa.ihsaat().isabat_dhakira,
            3,
            "all three lines must be answered from the memory"
        );
    }
    assert_eq!(
        thani.adad(),
        0,
        "a second run over the same lines must cost nothing, and cost {} request(s)",
        thani.adad()
    );

    let _ = std::fs::remove_file(&masar);
    let _ = std::fs::remove_dir(&mujallad);
}

// ---------------------------------------------------------------------------
// The change gate keeps recognition off a static screen
// ---------------------------------------------------------------------------

/// An unchanged region is not recognized twice.
///
/// The gate that stands between forty thousand poll opportunities and the number
/// of recognition passes actually run. These are genuine pixels through the
/// shipped preprocessing and hashing path.
#[test]
fn mintaqa_thabita_la_tuqra_marratayn() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());

    let bi_nass = sura_bi_shareet(200, Some(14));
    let bila_nass = sura(200);

    let awwal = qissa
        .hal_taghayyarat(MINTAQA, QaidatTarjama::IndaTaghyeer, &bi_nass)
        .unwrap_or_else(|khata| panic!("the gate refused the first capture: {khata}"));
    assert!(
        awwal,
        "the first capture of a region has nothing to compare against and must pass"
    );

    for marra in 0..8 {
        let mukarrar = qissa
            .hal_taghayyarat(MINTAQA, QaidatTarjama::IndaTaghyeer, &bi_nass)
            .unwrap_or_else(|khata| panic!("the gate refused repeat {marra}: {khata}"));
        assert!(
            !mukarrar,
            "repeat {marra} of an identical capture passed the gate; recognition would have run \
             on an unchanged screen"
        );
    }

    let baada = qissa
        .hal_taghayyarat(MINTAQA, QaidatTarjama::IndaTaghyeer, &bila_nass)
        .unwrap_or_else(|khata| panic!("the gate refused the changed capture: {khata}"));
    assert!(
        baada,
        "a region whose contents genuinely changed must pass the gate"
    );

    // The continuous rule is documented as paying every interval regardless.
    // A gate that quietly held it back would make that documentation false.
    let mustamirra = qissa
        .hal_taghayyarat(MINTAQA, QaidatTarjama::Mustamirra, &bila_nass)
        .unwrap_or_else(|khata| panic!("the gate refused a continuous capture: {khata}"));
    assert!(
        mustamirra,
        "the continuous rule bypasses the change gate by design"
    );
}

// ---------------------------------------------------------------------------
// Over budget degrades the refresh rate, and says so
// ---------------------------------------------------------------------------

/// Sustained over-budget frames slow the refresh rate and report that they did.
///
/// What must *not* happen is the overlay dropping the frames it draws: the
/// Arabic already on screen is nearly free to redraw, and losing it is the one
/// thing a player sees immediately. What happens instead is that the overlay
/// goes back to read the screen half as often, and both the log line and the
/// panel sentence say so with the real numbers.
#[test]
fn tajawuz_almeezaniya_yukhaffid_alwatira() {
    let mut watira = MunazzimWatira::jadeed(250_000, MeezaniyatItar::SAQF_IFTIRADI);
    let asas = watira.fasil_mikro();
    assert!(!watira.mutadahwira(), "a fresh governor is not degraded");

    let mut taghyeer: Option<TaghyeerWatira> = None;
    for _ in 0..NAFIDHAT_TADAHWUR {
        taghyeer = watira
            .sajjil_itar(MeezaniyatItar::SAQF_IFTIRADI * 3)
            .or(taghyeer);
    }

    let Some(TaghyeerWatira::Tadahwur {
        min_mikro,
        ila_mikro,
        daraja,
    }) = taghyeer
    else {
        panic!(
            "{NAFIDHAT_TADAHWUR} consecutive frames at three times the budget reported no \
             degradation at all: {taghyeer:?}"
        );
    };
    assert_eq!(
        min_mikro, asas,
        "the change must name the interval it started from"
    );
    assert_eq!(
        ila_mikro, 500_000,
        "one degradation step doubles the interval; it went to {ila_mikro} µs"
    );
    assert_eq!(daraja, 1, "one window of overrun is one step");
    assert_eq!(
        watira.fasil_mikro(),
        500_000,
        "the governor must be polling at the new rate"
    );
    assert!(
        watira.mutadahwira(),
        "the panel's one boolean must say the overlay is struggling"
    );

    let wasf = watira.wasf();
    assert!(
        wasf.contains("degraded"),
        "the report must say plainly that it degraded; it said: {wasf}"
    );
    assert!(
        wasf.contains("4.00/s") && wasf.contains("2.00/s"),
        "the report must carry both the requested rate and the rate in force; it said: {wasf}"
    );
    assert!(
        watira.itarat_fawq() >= u64::from(NAFIDHAT_TADAHWUR),
        "every over-budget frame must be counted"
    );

    let unwan = watira.unwan();
    assert!(
        unwan.contains("خُفّضت"),
        "the in-game panel must say it in Arabic too; it said: {unwan}"
    );

    // And it comes back, but only after a much longer run of healthy frames.
    let mut taafi: Option<TaghyeerWatira> = None;
    for _ in 0..(NAFIDHAT_TADAHWUR * 8) {
        taafi = watira.sajjil_itar(500).or(taafi);
    }
    let Some(TaghyeerWatira::Taafi {
        ila_mikro, daraja, ..
    }) = taafi
    else {
        panic!("a long run of cheap frames did not recover the rate: {taafi:?}");
    };
    assert_eq!(ila_mikro, asas, "recovery returns to the base interval");
    assert_eq!(daraja, 0, "and to no degradation at all");
    assert!(
        !watira.mutadahwira(),
        "a recovered governor is not degraded"
    );
}

/// Degrading does not stop the overlay drawing what it already has.
///
/// The distinction the whole governor exists for. The coordinator keeps handing
/// out the last completed snapshot on every frame while the rate falls.
#[test]
fn attadahwur_la_yusqit_alitarat() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());
    let _ = mrir(
        &mut qissa,
        0,
        &["The old road north is closed until the thaw."; 3],
    );

    let mut munassiq = Munassiq::jadeed(
        qissa.manshura(),
        MunazzimWatira::jadeed(250_000, MeezaniyatItar::SAQF_IFTIRADI),
    );

    let mut marsuma = 0_u32;
    let mut khaffad = false;
    for _ in 0..(NAFIDHAT_TADAHWUR * 2) {
        if munassiq
            .itar(MeezaniyatItar::SAQF_IFTIRADI * 3)
            .is_some_and(TaghyeerWatira::tadahwur)
        {
            khaffad = true;
        }
        if !munassiq.laqta(SATH).is_empty() {
            marsuma = marsuma.saturating_add(1);
        }
    }

    assert!(
        khaffad,
        "the rate must have been given up under a sustained overrun"
    );
    assert_eq!(
        marsuma,
        NAFIDHAT_TADAHWUR * 2,
        "every one of the {} frames must still have had a line to draw; {marsuma} did",
        NAFIDHAT_TADAHWUR * 2
    );
    assert_eq!(
        munassiq.tark_qufl(),
        0,
        "nothing here contends for the snapshot lock"
    );
}

// ---------------------------------------------------------------------------
// A three-hour session, in one arithmetic statement
// ---------------------------------------------------------------------------

/// Forty-three thousand readings of a real dialogue rhythm cost one request per
/// line.
///
/// The session this whole module is written for, compressed: a thousand distinct
/// lines, each held on screen for about eleven seconds at four polls a second,
/// which is what a dialogue-heavy game actually looks like. The assertion is the
/// only one that matters — the number of provider requests equals the number of
/// distinct things that were said.
#[test]
fn jalsa_tawila_taklifat_satr_wahid() {
    const SUTUR: u32 = 1_000;
    const MARRAT: usize = 43;
    const MARRAT_RAQM: u64 = 43;

    let mutarjim = MutarjimAadd::default();
    let mut qissa = jalsa(&mutarjim, dhakirat_jalsa());

    let mut lahza = 0;
    let mut mutamayyiza = std::collections::BTreeSet::new();
    for raqm in 0..SUTUR {
        let satr = satr_hiwar(raqm);
        let _ = mutamayyiza.insert(satr.clone());
        let qiraat: Vec<&str> = vec![satr.as_str(); MARRAT];
        lahza = mrir(&mut qissa, lahza, &qiraat);
    }
    assert_eq!(
        mutamayyiza.len(),
        usize::try_from(SUTUR).unwrap_or(0),
        "the fixture must produce {SUTUR} genuinely distinct lines, and produced {}",
        mutamayyiza.len()
    );

    let ihsaat = qissa.ihsaat();
    let mutawaqqa = u64::from(SUTUR);
    assert_eq!(
        ihsaat.maqruaat,
        mutawaqqa * MARRAT_RAQM,
        "every reading must have reached the tracker"
    );
    assert_eq!(
        mutarjim.adad(),
        mutawaqqa,
        "{} distinct lines cost {} request(s); one per line is the whole claim",
        SUTUR,
        mutarjim.adad()
    );
    assert_eq!(
        ihsaat.muwaffara(),
        mutawaqqa * MARRAT_RAQM.saturating_sub(1),
        "the saving against a per-frame pipeline is the difference of two printed counters"
    );
}

// ---------------------------------------------------------------------------
// The two halves, joined
// ---------------------------------------------------------------------------

/// A recognizer that reads the band position out of a real capture.
///
/// **Not OCR.** It is a deterministic [`taarib_tabaqa::qira::Qari`] that
/// consumes the genuine pixels the capture carries and returns which of a small
/// set of lines the band in them stands for. It exists so that the worker, the
/// change gate, the coordinate conversion and the frame path can be exercised as
/// one thing without a model file or a display; nothing about recognition
/// accuracy is claimed or tested by it.
#[derive(Debug)]
struct QariShareet;

impl taarib_tabaqa::qira::Qari for QariShareet {
    fn ism(&self) -> &'static str {
        "synthetic band reader"
    }

    fn mutah(&self) -> bool {
        true
    }

    fn lughat(&self) -> &[&str] {
        &["en"]
    }

    fn iqra(
        &mut self,
        sura: &SuraMultaqata,
    ) -> Result<Vec<taarib_tabaqa::qira::SatrMaqru>, KhataTabaqa> {
        let ard = usize::try_from(sura.ard()).unwrap_or(1).max(1);
        let bayt = sura.bayt();
        let mut aala: Option<u32> = None;
        for (saf, satr) in bayt.chunks_exact(ard.saturating_mul(4)).enumerate() {
            if satr.first().copied().unwrap_or(255) < 100 {
                aala = Some(u32::try_from(saf).unwrap_or(0));
                break;
            }
        }
        let Some(aala) = aala else {
            return Err(KhataTabaqa::LaNassMaqru {
                mintaqa: "synthetic".to_owned(),
                thiqa: None,
            });
        };
        let fahras = usize::try_from(aala)
            .unwrap_or(0)
            .checked_div(14)
            .unwrap_or(0);
        let nass = HIWAR_SHAREET
            .get(fahras)
            .copied()
            .unwrap_or("An unreadable line.");
        Ok(vec![taarib_tabaqa::qira::SatrMaqru::jadeed(
            nass,
            MustatilBiksel {
                yasar: 0,
                aala,
                ard: sura.ard(),
                irtifa: 12,
            },
            90,
            true,
        )])
    }
}

/// The three lines the band positions stand for.
const HIWAR_SHAREET: [&str; 3] = [
    "The old road north is closed until the thaw.",
    "Take the ferry from the eastern pier instead.",
    "It sails at dawn, and not again until the day after.",
];

/// The whole path: capture on the frame, process off it, draw what finished.
///
/// The only test here that runs the worker thread and the coordinator together.
/// It asserts the three things the split exists for — the frame half never
/// recognizes, the off-frame half does all of it, and the frame half always has
/// something to draw once the first result has landed.
#[test]
fn nisfa_alitar_wa_ma_khalfah() {
    let mutarjim = MutarjimAadd::default();
    let mut qissa = Qissa::jadeeda(luba(), ISM_LUBA, khiyarat())
        .bi_mutarjim(Box::new(mutarjim.clone()))
        .bi_dhakira(dhakirat_jalsa())
        .bi_qari("synthetic band reader");
    qissa.ayyin_sath(SATH);
    let manshura = qissa.manshura();

    let qari = taarib_tabaqa::qira::IkhtiyarQari::min_qari(
        Box::new(QariShareet),
        vec!["chosen by the test".to_owned()],
    );
    let mut khayt = match KhaytQissa::ibda(qissa, qari) {
        Ok(khayt) => khayt,
        Err(khata) => panic!("the worker would not start: {khata}"),
    };

    let mut munassiq = Munassiq::jadeed(
        manshura,
        MunazzimWatira::jadeed(250_000, MeezaniyatItar::SAQF_IFTIRADI),
    );

    // Three lines, each held for six polls, at a healthy frame cost.
    let mut lahza = 0_u64;
    for marra in 0..18_u32 {
        let aala = marra.checked_div(6).unwrap_or(0).saturating_mul(14);
        let _ = munassiq.itar(400);
        let _ = khayt.adfa(
            MINTAQA,
            "subtitles",
            QaidatTarjama::IndaTaghyeer,
            lahza,
            sura_bi_shareet(200, Some(aala)),
        );
        lahza = lahza.saturating_add(KHUTWA_MIKRO);
        // The worker is a real thread. Nothing here waits on it for the frame
        // path — that is the point — so the loop simply gives it room.
        std::thread::sleep(std::time::Duration::from_millis(4));
    }

    khayt.awqif();
    for _ in 0..200 {
        let _ = munassiq.itar(400);
        if !munassiq.laqta(SATH).is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert_eq!(
        khayt.matruka(),
        0,
        "the queue must not have overflowed at this pace"
    );
    assert!(
        khayt.muaalaja() > 0,
        "the worker must have processed captures"
    );
    assert_eq!(
        khayt.akhta(),
        0,
        "no pass should have refused: {}",
        khayt.wasf()
    );
    assert_eq!(khayt.hala(), HalatKhayt::Salima, "and the worker says so");
    assert_eq!(
        mutarjim.adad(),
        3,
        "three distinct lines behind eighteen captures cost {} request(s)",
        mutarjim.adad()
    );
    let usul = mutarjim.usul();
    assert_eq!(
        usul,
        HIWAR_SHAREET
            .iter()
            .map(|nass| (*nass).to_owned())
            .collect::<Vec<_>>(),
        "the requests must be the three lines, in the order they were on screen"
    );
    assert!(
        !munassiq.laqta(SATH).is_empty(),
        "the frame half must have a completed result to draw"
    );

    // The boxes reach the render half in *surface* pixels: the recognizer
    // reported them inside the captured image, and the session moved them.
    let laqta = munassiq.laqta(SATH);
    for satr in laqta {
        assert!(
            satr.mawdi.aala >= SUNDUQ.aala && satr.mawdi.yasar >= SUNDUQ.yasar,
            "a drawn line is at {:?}, which is still in the capture's own coordinates",
            satr.mawdi
        );
    }
}

// ---------------------------------------------------------------------------
// A dead recognizer and a noisy frame are not the same integer
// ---------------------------------------------------------------------------

/// A recognizer whose engine is gone.
///
/// Every read answers [`KhataTabaqa::QariGhayrMutah`], which is what the
/// Windows engine does once its language pack is removed mid-session and what
/// the portable one does once an antivirus quarantines its model files.
#[derive(Debug)]
struct QariMayyit;

impl taarib_tabaqa::qira::Qari for QariMayyit {
    fn ism(&self) -> &'static str {
        "dead engine"
    }

    fn mutah(&self) -> bool {
        false
    }

    fn lughat(&self) -> &[&str] {
        &["en"]
    }

    fn iqra(
        &mut self,
        _: &SuraMultaqata,
    ) -> Result<Vec<taarib_tabaqa::qira::SatrMaqru>, KhataTabaqa> {
        Err(KhataTabaqa::QariGhayrMutah {
            sabab: "the language pack for [en-US] was removed while the game was running"
                .to_owned(),
        })
    }
}

/// A recognizer that refuses one capture for that capture's own reason, then
/// reads normally — a frame the GPU would not hand back, followed by ordinary
/// empty regions.
#[derive(Debug, Default)]
struct QariMutaqallib {
    rafada: bool,
}

impl taarib_tabaqa::qira::Qari for QariMutaqallib {
    fn ism(&self) -> &'static str {
        "one bad frame"
    }

    fn mutah(&self) -> bool {
        true
    }

    fn lughat(&self) -> &[&str] {
        &["en"]
    }

    fn iqra(
        &mut self,
        _: &SuraMultaqata,
    ) -> Result<Vec<taarib_tabaqa::qira::SatrMaqru>, KhataTabaqa> {
        if !self.rafada {
            self.rafada = true;
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the frame could not be read back this once".to_owned(),
            });
        }
        Err(KhataTabaqa::LaNassMaqru {
            mintaqa: "synthetic".to_owned(),
            thiqa: None,
        })
    }
}

/// A worker over a session and a recognizer the test chose.
fn khayt_bi_qari(qari: Box<dyn taarib_tabaqa::qira::Qari>) -> KhaytQissa {
    let mut qissa = Qissa::jadeeda(luba(), ISM_LUBA, khiyarat())
        .bi_mutarjim(Box::new(MutarjimAadd::default()))
        .bi_dhakira(dhakirat_jalsa())
        .bi_qari("chosen by the test");
    qissa.ayyin_sath(SATH);
    let ikhtiyar =
        taarib_tabaqa::qira::IkhtiyarQari::min_qari(qari, vec!["chosen by the test".to_owned()]);
    match KhaytQissa::ibda(qissa, ikhtiyar) {
        Ok(khayt) => khayt,
        Err(khata) => panic!("the worker would not start: {khata}"),
    }
}

/// Posts one capture on the rule that bypasses the picture gate, so it reaches
/// the recognizer rather than being held back as unchanged.
fn adfa_wahida(khayt: &KhaytQissa, lahza: u64) -> HalatDaf {
    khayt.adfa(
        MINTAQA,
        "subtitles",
        QaidatTarjama::Mustamirra,
        lahza,
        sura_bi_shareet(200, Some(14)),
    )
}

/// Waits, bounded, for the worker to make a condition true.
///
/// The worker is a real thread and nothing on the frame path waits for it, so
/// the test does the waiting — and says so if it ran out of patience.
fn intazir(shart: impl Fn() -> bool) {
    for _ in 0..400 {
        if shart() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the worker did not reach the expected state in two seconds");
}

/// A recognizer that is gone stops the session, says why in both languages,
/// closes the door on further captures, and is reopened by translate-now.
///
/// This is the headline: before, the same sequence produced `1 refused`, then
/// `2 refused`, then `14400 refused`, with the reason discarded at the moment
/// it was known and the worker recognizing nothing four times a second for
/// the rest of the session.
#[test]
fn qari_mayyit_yuqif_al_jalsa_wa_yusammi_al_sabab() {
    let mut khayt = khayt_bi_qari(Box::new(QariMayyit));

    assert_eq!(
        adfa_wahida(&khayt, 0),
        HalatDaf::Qubilat,
        "the first capture is posted"
    );
    intazir(|| khayt.akhta() >= 1);

    let hala = khayt.hala();
    let HalatKhayt::Mutawaqqifa { sabab, sabab_arabi } = hala.clone() else {
        panic!("a dead recognizer must stop the session, not merely count: {hala:?}");
    };
    assert!(hala.mutawaqqifa());
    assert!(
        sabab.contains("language pack"),
        "the reason survives verbatim: {sabab}"
    );
    assert!(!sabab_arabi.trim().is_empty(), "and is readable in Arabic");

    // The door is closed: no queueing, no recognition, and the refusals are
    // counted apart from both the drops and the failed passes.
    assert_eq!(adfa_wahida(&khayt, KHUTWA_MIKRO), HalatDaf::Rufidat);
    assert_eq!(adfa_wahida(&khayt, KHUTWA_MIKRO * 2), HalatDaf::Rufidat);
    assert_eq!(khayt.marfuda(), 2, "refused at the door, twice");
    assert_eq!(khayt.matruka(), 0, "refused is not dropped");
    assert_eq!(
        khayt.akhta(),
        1,
        "a closed door costs no further recognition"
    );

    let wasf = khayt.wasf();
    assert!(
        wasf.contains("language pack"),
        "the panel sentence carries the reason: {wasf}"
    );
    assert!(
        wasf.contains("stopped"),
        "and says the session is stopped: {wasf}"
    );
    assert!(
        wasf.contains("2 refused at the door"),
        "and counts the door: {wasf}"
    );
    let wasf_arabi = khayt.wasf_arabi();
    assert!(
        wasf_arabi.contains("توقّفت"),
        "the Arabic says stopped too: {wasf_arabi}"
    );

    // Translate-now is the user's way back in. The next pass either succeeds
    // or stops the session again with a fresh reason — here, the same one.
    khayt.iqra_alan();
    assert_eq!(
        adfa_wahida(&khayt, KHUTWA_MIKRO * 3),
        HalatDaf::Qubilat,
        "reopened"
    );
    intazir(|| khayt.akhta() >= 2);
    assert!(
        khayt.hala().mutawaqqifa(),
        "the engine is still dead, and the worker says so again"
    );
    assert_eq!(
        adfa_wahida(&khayt, KHUTWA_MIKRO * 4),
        HalatDaf::Rufidat,
        "and the door is shut"
    );

    khayt.awqif();
}

/// One capture's refusal is recorded as one capture's, and the session goes
/// on.
///
/// The other half of the split: a noisy frame must not close the door, or a
/// game with one unreadable frame an hour would have its overlay stop on it.
#[test]
fn rafd_aabir_la_yuqif_al_jalsa() {
    let mut khayt = khayt_bi_qari(Box::new(QariMutaqallib::default()));

    assert_eq!(adfa_wahida(&khayt, 0), HalatDaf::Qubilat);
    intazir(|| khayt.akhta() >= 1);

    let hala = khayt.hala();
    let HalatKhayt::Aabira { sabab, .. } = hala.clone() else {
        panic!("one capture's refusal must be recorded as one capture's: {hala:?}");
    };
    assert!(!hala.mutawaqqifa());
    assert!(sabab.contains("read back"), "the reason survives: {sabab}");

    // The door is open, the next capture is processed, and the record of the
    // last refusal stays readable rather than being erased by a success.
    assert_eq!(
        adfa_wahida(&khayt, KHUTWA_MIKRO),
        HalatDaf::Qubilat,
        "the door stays open"
    );
    intazir(|| khayt.muaalaja() >= 1);
    assert_eq!(khayt.marfuda(), 0, "nothing was refused at the door");
    assert_eq!(khayt.akhta(), 1, "one refusal, and it stayed one");
    assert!(matches!(khayt.hala(), HalatKhayt::Aabira { .. }));
    let wasf = khayt.wasf();
    assert!(
        wasf.contains("one capture's"),
        "the panel says which kind it was: {wasf}"
    );
    assert!(
        !wasf.contains("stopped"),
        "and does not say the session stopped: {wasf}"
    );

    khayt.awqif();
}

// ---------------------------------------------------------------------------
// Provenance survives, and an invented confidence does not
// ---------------------------------------------------------------------------

/// A memory that records the provenance it was handed and answers as an
/// observation.
#[derive(Debug, Default)]
struct DhakiraShahida {
    qiyud: Mutex<BTreeMap<String, String>>,
    thiqat: Mutex<Vec<Option<u8>>>,
}

impl DhakiraTabaqa for DhakiraShahida {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait's signature is `&str`; see the note on the translator above"
    )]
    fn ism(&self) -> &str {
        "witness"
    }

    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr> {
        // Answered as an observation, which is what the shared memory returns
        // for anything a person did not write: read off somebody's screen and
        // machine-translated from that reading.
        self.qiyud
            .lock()
            .get(talab.asl)
            .map(|arabi| RaddSatr::mulahaza(arabi.clone()))
    }

    fn sajjil(&self, qayd: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa> {
        self.thiqat.lock().push(qayd.thiqa);
        let _ = self
            .qiyud
            .lock()
            .insert(qayd.asl.to_owned(), qayd.arabi.to_owned());
        Ok(())
    }

    fn daaima(&self) -> bool {
        true
    }
}

/// One reading with the recognizer's own `maqisa` bit set as given.
fn mulahaza_bi_qiyas(nass: &str, thiqa: u8, maqisa: bool) -> Vec<QiraaMulahaza> {
    vec![QiraaMulahaza {
        nass: nass.to_owned(),
        mawdi: SUNDUQ,
        thiqa,
        maqisa,
    }]
}

/// An engine that measures nothing hands the memory nothing, not a constant.
///
/// Two of the three recognizers this crate ships to report no per-line
/// confidence, and `SatrMaqru` carries a constant 80 in their place with a bit
/// beside it saying so. If that constant reaches the shared memory as though an
/// engine had produced it, the guard that withholds an unmeasured accumulation
/// from being shared is defeated silently — it would see a measured 80 for every
/// line on every Windows and Linux machine.
///
/// The same bit has to survive into the reading history, or a later harvest from
/// the file cannot tell the two apart either.
#[test]
fn qiyas_ghayr_mawjud_la_yusbih_raqman() {
    let mutarjim = MutarjimAadd::default();
    let shahida = Arc::new(DhakiraShahida::default());
    let sijill = taarib_tabaqa::sijill_qira::SijillMushtarak::jadeed(
        taarib_tabaqa::sijill_qira::SijillQira::jadeed(luba()),
    );
    let mut qissa = Qissa::jadeeda(
        luba(),
        ISM_LUBA,
        KhiyaratQissa {
            tasnif: TasnifNass::Hiwar,
            ..KhiyaratQissa::iftiradiya()
        },
    )
    .bi_mutarjim(Box::new(mutarjim.clone()))
    .bi_dhakira(Arc::<DhakiraShahida>::clone(&shahida))
    .bi_sijill(sijill.clone())
    .bi_qari("synthetic");
    qissa.ayyin_sath(SATH);

    // A line no engine measured: the confidence field carries the stand-in.
    let ghayr = "The old road north is closed until the thaw.";
    for lahza in [0, KHUTWA_MIKRO] {
        let _ = qissa.aalij_sutur(
            MINTAQA,
            "subtitles",
            lahza,
            &mulahaza_bi_qiyas(ghayr, 80, false),
        );
    }
    // A line macOS Vision measured at 62, which is below the stand-in.
    let maqis = "Take the ferry from the eastern pier instead.";
    for lahza in [KHUTWA_MIKRO * 2, KHUTWA_MIKRO * 3] {
        let _ = qissa.aalij_sutur(
            MINTAQA,
            "subtitles",
            lahza,
            &mulahaza_bi_qiyas(maqis, 62, true),
        );
    }

    assert_eq!(
        mutarjim.adad(),
        2,
        "both lines must have been translated once each"
    );
    let thiqat = shahida.thiqat.lock().clone();
    assert_eq!(
        thiqat,
        vec![None, Some(62)],
        "the unmeasured line must reach the memory as `None`, not as the constant 80; it \
         arrived as {thiqat:?}"
    );

    let madakhil = sijill.laqta(8);
    let ghayr_madkhal = madakhil.iter().find(|madkhal| madkhal.asl == ghayr);
    let maqis_madkhal = madakhil.iter().find(|madkhal| madkhal.asl == maqis);
    let (Some(ghayr_madkhal), Some(maqis_madkhal)) = (ghayr_madkhal, maqis_madkhal) else {
        panic!(
            "both lines must be in the reading history; it holds {} row(s)",
            madakhil.len()
        );
    };
    assert!(
        !ghayr_madkhal.maqisa,
        "the history must record that nothing measured this reading"
    );
    assert!(
        maqis_madkhal.maqisa,
        "and that something measured the other one"
    );
    assert_eq!(
        maqis_madkhal.thiqa, 62,
        "a measured reading keeps its own number rather than the stand-in"
    );
    assert_eq!(
        ghayr_madkhal.unwan_thiqa(),
        "لم يُقِس المحرّك ثقته",
        "an unmeasured reading is not a weak reading, and the panel must not call it one"
    );
}

/// A line the memory answered is recorded as a screen reading, not as this
/// session's own machine translation.
///
/// The distinction a player scrolling back needs: "Taarib translated this game's
/// text for you" and "somebody's overlay read a screen, a machine translated
/// that reading, and this session reused the answer" are different claims, and
/// only one of them is true for a memory hit.
#[test]
fn isabat_dhakira_tusajjal_kaqiraa() {
    let mutarjim = MutarjimAadd::default();
    let shahida = Arc::new(DhakiraShahida::default());
    let sijill = taarib_tabaqa::sijill_qira::SijillMushtarak::jadeed(
        taarib_tabaqa::sijill_qira::SijillQira::jadeed(luba()),
    );
    let nass = "The old road north is closed until the thaw.";
    let _ = shahida.sajjil(&QaydTabaqa {
        asl: nass,
        arabi: "الطريق الشمالي مغلق حتى ذوبان الثلج.",
        luba: luba(),
        ism_luba: Some(ISM_LUBA),
        mintaqa: Some("subtitles"),
        tasnif: TasnifNass::Hiwar,
        qari: Some("some other player's recognizer"),
        muzawwid: Some("some other player's provider"),
        thiqa: None,
    });

    let mut qissa = Qissa::jadeeda(
        luba(),
        ISM_LUBA,
        KhiyaratQissa {
            tasnif: TasnifNass::Hiwar,
            ..KhiyaratQissa::iftiradiya()
        },
    )
    .bi_mutarjim(Box::new(mutarjim.clone()))
    .bi_dhakira(Arc::<DhakiraShahida>::clone(&shahida))
    .bi_sijill(sijill.clone())
    .bi_qari("synthetic");
    qissa.ayyin_sath(SATH);
    let _ = mrir(&mut qissa, 0, &[nass; 3]);

    assert_eq!(
        mutarjim.adad(),
        0,
        "the memory answered, so nothing was translated"
    );
    let madakhil = sijill.laqta(4);
    let Some(madkhal) = madakhil.first() else {
        panic!("the line must be in the reading history");
    };
    assert_eq!(
        madkhal.masdar,
        Some(taarib_tabaqa::sijill_qira::MasdarTarjama::Mulahaza),
        "a memory hit is a screen reading somebody else made, and the history must say so \
         rather than calling it this session's machine translation"
    );
    assert_eq!(
        madkhal
            .masdar
            .map(taarib_tabaqa::sijill_qira::MasdarTarjama::unwan),
        Some("قراءة شاشة سابقة"),
        "and the panel's Arabic label must say it too"
    );
}
