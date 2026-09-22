//! The decisions a state says it accepts, checked against the transitions.
//!
//! [`HalatTaqdeem::afal_mutaha`] is what the Review Console draws its controls
//! from, so a decision it names and a transition that refuses it would put a
//! button on screen whose only outcome is `TAARIB-E-9068`. Nothing here
//! restates the rules: every case runs the transition the console runs, with
//! arguments nothing but the state can refuse, and compares the outcome with
//! what the state promised.

use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_taqdeem::muraja::NawIjraMuraja;
use taarib_taqdeem::musawwada::{HalatTaqdeem, Musawwada};
use taarib_tarqee::taghtiya_ruqaa::TaqrirTaghtiya;
use taarib_tarqee::taqrir_tajawuz::TaqrirTajawuz;

/// A fingerprint-shaped constant; nothing here verifies one.
const BASMA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The instant every transition under test is stamped with.
const WAQT: &str = "2026-09-20T16:00:00Z";

/// Words, so that a transition wanting a written reason gets one and only the
/// state is left to refuse it.
const MAKTUB: &str = "the two lines the contributor has to change";

/// A saved draft in one state, with every report left at its own default.
fn musawwada_fi(hala: &HalatTaqdeem) -> Option<Musawwada> {
    let rasm = serde_json::json!({
        "isdar": 1,
        "id": "01a0bf7d-d844-7bf3-b0e1-0f93d8c6a5b7",
        "murajaa": 1,
        "hala": serde_json::to_value(hala).ok()?,
        "tareekh": [],
        "luba": "bfa449ba-027a-5907-b784-c52495a046c2",
        "masadir": [{ "manassa": "steam", "muarrif": 1 }],
        "irtibat": {
            "manassat": [],
            "basmat": [BASMA],
            "adad_malaffat": 1,
            "mukhattat": { "isdar": 1, "hawiyat": ["Nuzha_Data/resources.assets"] },
            "nitaq": null,
            "khutut": []
        },
        "basmat_asliya": [BASMA],
        "masar_huzma": "nuzha-r1.ruqaa",
        "basmat_huzma": BASMA,
        "hajm_huzma": 1024,
        "bayan": {},
        "wasf": {
            "unwan": "نزهة تجريبية",
            "ism_musahim": "00000000",
            "ism_luba": "نزهة تجريبية",
            "rukhsa": { "naw": "cc_by_sa" },
            "tareeqa": "bashariya_kamila",
            "isdar_taarib": "1.0.1"
        },
        "shahada": {
            "nusus": 3,
            "takhtitat": 0,
            "safahat": 0,
            "khutut": [],
            "furuq": 0,
            "bayt_min_alluba": 0
        },
        "musahim": {
            "musahim": BASMA,
            "ism": "00000000",
            "itimad": null,
            "miftah": vec![0_u8; 32]
        },
        "sharh": "نزهة تجريبية",
        "taghyeerat": "أول مراجعة",
        "taghtiya": serde_json::to_value(Taghtiya::default()).ok()?,
        "taqrir_taghtiya": serde_json::to_value(TaqrirTaghtiya::default()).ok()?,
        "tajawuz": serde_json::to_value(TaqrirTajawuz::default()).ok()?,
        "istirad": [],
        "ansha": "2026-09-20T15:44:50Z"
    });
    serde_json::from_value(rasm).ok()
}

/// Every state, one of each.
fn kull_alhalat() -> Vec<HalatTaqdeem> {
    vec![
        HalatTaqdeem::Musawwada,
        HalatTaqdeem::Muqaddama {
            waqt: WAQT.to_owned(),
        },
        HalatTaqdeem::QaydMuraja {
            waqt: WAQT.to_owned(),
        },
        HalatTaqdeem::MatlubTaadil {
            waqt: WAQT.to_owned(),
            mulakhkhas: MAKTUB.to_owned(),
            nusus: Vec::new(),
        },
        HalatTaqdeem::MawafaqYunshar {
            waqt: WAQT.to_owned(),
        },
        HalatTaqdeem::Manshura {
            waqt: WAQT.to_owned(),
            murajaa: taarib_mustalahat::ruqaa::RuqaaRevision::AWWAL,
        },
        HalatTaqdeem::Marfuda {
            waqt: WAQT.to_owned(),
            sabab: MAKTUB.to_owned(),
        },
        HalatTaqdeem::Mashuba {
            waqt: WAQT.to_owned(),
        },
        HalatTaqdeem::Masbuba {
            waqt: WAQT.to_owned(),
            sabab: MAKTUB.to_owned(),
        },
    ]
}

/// One state's position in the list above.
///
/// A `match` with no wildcard, so a tenth state added to `HalatTaqdeem` stops
/// this file compiling until [`kull_alhalat`] names it too — which is the only
/// thing standing between "every state is covered" and "every state somebody
/// remembered".
const fn tarteeb(hala: &HalatTaqdeem) -> usize {
    match hala {
        HalatTaqdeem::Musawwada => 0,
        HalatTaqdeem::Muqaddama { .. } => 1,
        HalatTaqdeem::QaydMuraja { .. } => 2,
        HalatTaqdeem::MatlubTaadil { .. } => 3,
        HalatTaqdeem::MawafaqYunshar { .. } => 4,
        HalatTaqdeem::Manshura { .. } => 5,
        HalatTaqdeem::Marfuda { .. } => 6,
        HalatTaqdeem::Mashuba { .. } => 7,
        HalatTaqdeem::Masbuba { .. } => 8,
    }
}

/// Runs the decision the console runs, and answers whether it went through.
///
/// [`NawIjraMuraja::Taaliq`] answers [`None`]: a comment moves the submission
/// nowhere, so there is no transition to compare against and nothing here can
/// stand in for one. It is checked separately, against its own rule.
fn naffidh(musawwada: Musawwada, ijra: NawIjraMuraja) -> Option<bool> {
    Some(match ijra {
        NawIjraMuraja::Taaliq => return None,
        NawIjraMuraja::TalabTaadil => musawwada.tulib_taadil(WAQT, MAKTUB, Vec::new()).is_ok(),
        NawIjraMuraja::Rafd => musawwada.rufidat(WAQT, MAKTUB).is_ok(),
        // Both halves, because approving is what the console calls one action
        // and the submission calls two: the approval and the publication it
        // authorises. A state that accepts the first and refuses the second
        // would leave a submission stranded mid-publish.
        NawIjraMuraja::Iaatimad => musawwada
            .wufiq_alayha(WAQT)
            .and_then(|mowafaqa| mowafaqa.nushirat(WAQT))
            .is_ok(),
        NawIjraMuraja::Sahb => musawwada.suhibat_min_almalik(WAQT, MAKTUB).is_ok(),
    })
}

#[test]
fn kull_hala_madhkura_marra_wahida() {
    let halat = kull_alhalat();
    let mut mawajid: Vec<usize> = halat.iter().map(tarteeb).collect();
    mawajid.sort_unstable();
    mawajid.dedup();
    assert_eq!(
        mawajid.len(),
        halat.len(),
        "two entries of kull_alhalat name the same state"
    );
    assert_eq!(
        mawajid,
        (0..halat.len()).collect::<Vec<usize>>(),
        "kull_alhalat does not cover every state exactly once"
    );
}

#[test]
fn afal_mutaha_tutabiq_alintiqalat() {
    let halat = kull_alhalat();
    let masawid: Vec<Musawwada> = halat.iter().filter_map(musawwada_fi).collect();
    // Built up front and counted, because a fixture that quietly stops reading
    // back would leave every assertion below passing over nothing.
    assert_eq!(
        masawid.len(),
        halat.len(),
        "the draft this file builds does not read back in every state"
    );
    for (hala, musawwada) in halat.iter().zip(masawid.iter()) {
        let mutaha = hala.afal_mutaha();
        for ijra in NawIjraMuraja::KULL {
            // A transition consumes the submission, so each decision is tried
            // against its own copy of the same starting state.
            let Some(najahat) = naffidh(musawwada.clone(), ijra) else {
                continue;
            };
            assert_eq!(
                mutaha.contains(&ijra),
                najahat,
                "in {} the console is told {} is {}, and the transition {}",
                hala.ism(),
                ijra.ramz(),
                if mutaha.contains(&ijra) {
                    "on offer"
                } else {
                    "withheld"
                },
                if najahat { "accepted it" } else { "refused it" }
            );
        }
    }
}

#[test]
fn altaaliq_maftuh_illa_ala_musawwada_mahalliya() {
    for hala in kull_alhalat() {
        // A comment needs somebody to address, and a local draft has not
        // reached the owner; everywhere else there is a conversation to add to,
        // a finished submission included.
        let mutawaqqa = !matches!(hala, HalatTaqdeem::Musawwada);
        assert_eq!(
            hala.afal_mutaha().contains(&NawIjraMuraja::Taaliq),
            mutawaqqa,
            "the comment box is offered wrongly in {}",
            hala.ism()
        );
    }
}

#[test]
fn almanshura_taqbal_alsahb_wahdah() {
    let manshura = HalatTaqdeem::Manshura {
        waqt: WAQT.to_owned(),
        murajaa: taarib_mustalahat::ruqaa::RuqaaRevision::AWWAL,
    };
    let mutaha = manshura.afal_mutaha();
    // The defect this file exists for: the console offered Approve on this
    // state, and pressing it answered TAARIB-E-9068.
    assert!(!mutaha.contains(&NawIjraMuraja::Iaatimad));
    assert!(!mutaha.contains(&NawIjraMuraja::TalabTaadil));
    assert!(!mutaha.contains(&NawIjraMuraja::Rafd));
    assert!(mutaha.contains(&NawIjraMuraja::Sahb));
    assert!(mutaha.contains(&NawIjraMuraja::Taaliq));
}
