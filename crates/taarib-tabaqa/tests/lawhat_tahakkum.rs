//! The recognition worker's state reaches the panel, asserted.
//!
//! The worker keeps the identity of its last refusal so that a recognizer gone
//! for the session and one noisy frame are not the same integer. That is worth
//! nothing if the count is all a player ever sees, so these tests build the
//! panel the way the hook does and read the elements it lays out: the header
//! says the session has stopped on every page, the main page carries the
//! reason, and the budget page quotes the worker's sentence in full. A panel
//! that merged the three states back into "healthy" — or into a count — fails
//! here.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use taarib_tabaqa::lawhat_tahakkum::{LawhatTahakkum, SafhatLawha};
use taarib_tabaqa::qissa::HalatKhayt;
use taarib_tabaqa::wajiha::{HalatTabaqa, SighatSath, WajihatRusum, WasfSath};

/// The surface the panel is laid out against.
const SATH: WasfSath = WasfSath {
    ard: 1920,
    irtifa: 1080,
    sigha: SighatSath::Bgra8,
    sirgb: true,
};

/// An open panel over a running overlay, on the requested page.
fn lawha(safha: SafhatLawha) -> LawhatTahakkum {
    let mut lawha = LawhatTahakkum::jadeeda("Synthetic Frames (a test fixture, not a game)");
    lawha.hadith_tabaqa(HalatTabaqa::Amila, Some(WajihatRusum::Direct3D11));
    lawha.hala_mut().irfa();
    lawha.hala_mut().ruh_ila(safha);
    lawha
}

/// Every string the panel would shape, in layout order.
fn nusus(lawha: &LawhatTahakkum) -> Vec<String> {
    lawha
        .bina(SATH)
        .into_iter()
        .filter_map(|ansur| ansur.nass)
        .collect()
}

/// The stopped state, with the reason a removed language pack produces.
fn mutawaqqifa() -> HalatKhayt {
    HalatKhayt::Mutawaqqifa {
        sabab: "no on-screen text recognizer is available: the language pack for [en-US] was \
                removed while the game was running"
            .to_owned(),
        sabab_arabi: "لا يوجد محرّك قراءة نصوص متاح على هذا الجهاز، ولا يمكن للطبقة قراءة ما \
                      على الشاشة بدونه."
            .to_owned(),
    }
}

/// A stopped session is on the status line of every page, and the main page
/// carries the reason.
#[test]
fn al_tawaqquf_yazhar_fi_kulli_safha_wa_sababuh_fi_al_raisiya() {
    for safha in SafhatLawha::jamee() {
        let mut lawha = lawha(safha);
        lawha.hadith_khayt(mutawaqqifa());
        let nusus = nusus(&lawha);
        assert!(
            nusus.iter().any(|nass| nass.contains("القراءة متوقّفة")),
            "{safha:?}: the status line must say recognition stopped: {nusus:?}"
        );
    }

    let mut lawha = lawha(SafhatLawha::Raisiya);
    lawha.hadith_khayt(mutawaqqifa());
    let nusus = nusus(&lawha);
    assert!(
        nusus.iter().any(|nass| nass == "متوقّفة"),
        "the main page's recognition row must read 'stopped': {nusus:?}"
    );
    assert!(
        nusus.iter().any(|nass| nass.contains("محرّك قراءة نصوص")),
        "the main page must carry the reason itself: {nusus:?}"
    );
    assert!(
        nusus.iter().any(|nass| nass.contains("إعادة القراءة")),
        "and say that a re-read is the way back in: {nusus:?}"
    );
}

/// One capture's refusal is shown as that, and the session is not called
/// stopped anywhere.
#[test]
fn al_rafd_al_aabir_la_yuqal_anhu_tawaqquf() {
    let mut lawha = lawha(SafhatLawha::Raisiya);
    lawha.hadith_khayt(HalatKhayt::Aabira {
        sabab: "the frame could not be read back: this once".to_owned(),
        sabab_arabi: "تعذّر التقاط صورة الإطار من كرت الشاشة.".to_owned(),
    });
    let nusus = nusus(&lawha);
    assert!(nusus.iter().any(|nass| nass == "رفض عابر"), "{nusus:?}");
    assert!(
        !nusus.iter().any(|nass| nass.contains("متوقّفة")),
        "a transient refusal must not be shown as a stopped session: {nusus:?}"
    );
}

/// The budget page quotes the worker's own sentence, whichever state it is in,
/// and a panel that has not asked the worker says nothing rather than
/// "healthy".
#[test]
fn safhat_al_meezaniya_taqtabis_jumlat_al_aamil() {
    let mut lawha = lawha(SafhatLawha::Meezaniya);
    assert!(
        !nusus(&lawha).iter().any(|nass| nass == "حالة القراءة:"),
        "an unasked worker is not a healthy one, and is not shown as one"
    );
    assert_eq!(lawha.khayt(), None);

    lawha.hadith_khayt(HalatKhayt::Salima);
    let nusus_salima = nusus(&lawha);
    assert!(
        nusus_salima.iter().any(|nass| nass == "حالة القراءة:"),
        "{nusus_salima:?}"
    );
    assert!(
        nusus_salima
            .iter()
            .any(|nass| nass == &HalatKhayt::Salima.wasf_arabi()),
        "the healthy sentence is quoted verbatim: {nusus_salima:?}"
    );

    lawha.hadith_khayt(mutawaqqifa());
    let nusus_mutawaqqifa = nusus(&lawha);
    assert!(
        nusus_mutawaqqifa
            .iter()
            .any(|nass| nass == &mutawaqqifa().wasf_arabi()),
        "the stopped sentence is quoted verbatim, reason included: {nusus_mutawaqqifa:?}"
    );
    assert_eq!(lawha.khayt(), Some(&mutawaqqifa()));
}
