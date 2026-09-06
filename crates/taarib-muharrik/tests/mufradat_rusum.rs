//! مفردات الرسوم — the probe's graphics vocabulary against the overlay's
//! backends, checked rather than remembered.
//!
//! Two enumerations name the same thing from opposite ends.
//! `taarib_mustalahat::muharrik::WajihaRusum` is what the probe *reports*:
//! serialized, persisted, generated into Studio's TypeScript, able to say
//! "Metal" for an API no backend draws and able to say "not determined".
//! `taarib_tabaqa::wajiha::WajihatRusum` is what the overlay *implements*:
//! exactly one at a time, always built, and split into two OpenGL profiles that
//! no import table can tell apart.
//!
//! They are separate on purpose — the reasoning is on `WajihaRusum` itself —
//! and the split has already drifted once, expensively. Three backends were
//! written and shipped (Direct3D 8, 9 and 10) while the probe had no value for
//! any of them and no import signature that produced one, so Resident Evil 4
//! reported an empty graphics list with `d3d9.dll` in its own import table.
//! The product could serve the game and simultaneously said it could not see
//! what the game used.
//!
//! The rule is one-directional and it is what these tests hold:
//!
//! 1. every backend the overlay ships must be nameable by the probe, and
//! 2. every API the probe can name a backend for must have an import signature
//!    that actually produces it — because a value nothing sets is a value that
//!    reports nothing.
//!
//! The converse is deliberately *not* required. The probe naming an API with no
//! backend is correct behaviour and [`WajihaRusum::Metal`] is the live case.
//!
//! [`mufrada`] is an exhaustive match, so a ninth backend does not compile until
//! somebody has decided what the probe calls it. That is the half of this file
//! that cannot be forgotten; the assertions are the half that catches the rest.

use taarib_muharrik::dalail::thunai::rusum_maarufa;
use taarib_mustalahat::muharrik::WajihaRusum;
use taarib_tabaqa::wajiha::WajihatRusum;

/// What the probe calls one overlay backend.
///
/// Many-to-one in exactly one place, and it is not an oversight:
/// [`WajihatRusum::OpenGlThabit`] and [`WajihatRusum::OpenGl`] are two backends
/// behind one import. Every OpenGL process on Windows links `opengl32.dll`
/// whatever profile its context asks for, so a probe reading files cannot
/// separate a fixed-function context from a modern one — the overlay separates
/// them inside the game by asking the live context which entry points it has.
/// A probe value for the split would be a probe forced to guess, and the guess
/// would be wrong for every game shipped in the fifteen years both profiles
/// were current.
const fn mufrada(wajiha: WajihatRusum) -> WajihaRusum {
    match wajiha {
        WajihatRusum::Direct3D8 => WajihaRusum::D3d8,
        WajihatRusum::Direct3D9 => WajihaRusum::D3d9,
        WajihatRusum::Direct3D10 => WajihaRusum::D3d10,
        WajihatRusum::Direct3D11 => WajihaRusum::D3d11,
        WajihatRusum::Direct3D12 => WajihaRusum::D3d12,
        WajihatRusum::OpenGl | WajihatRusum::OpenGlThabit => WajihaRusum::OpenGl,
        WajihatRusum::Vulkan => WajihaRusum::Vulkan,
    }
}

/// Every backend the overlay ships, as the probe names it.
fn mufradat_khattafat() -> Vec<WajihaRusum> {
    let mut rusum: Vec<WajihaRusum> = Vec::new();
    for wajiha in WajihatRusum::jamee() {
        let mufrada = mufrada(wajiha);
        if !rusum.contains(&mufrada) {
            rusum.push(mufrada);
        }
    }
    rusum
}

/// `WajihaRusum::lahu_khattaf` must agree with the backends that exist, in both
/// directions.
///
/// Both halves matter and they fail differently. A backend added without the
/// claim being updated means the report tells a user the overlay has nowhere to
/// attach to a game it can in fact draw over. A claim left standing after a
/// backend is removed is the same lie inverted, and worse, because it is the
/// direction that promises rather than withholds.
#[test]
fn iddiaa_al_khattaf_yutabiq_al_mabni() {
    let mabniya = mufradat_khattafat();

    for wajiha in WajihaRusum::JAMEE {
        assert_eq!(
            wajiha.lahu_khattaf(),
            mabniya.contains(&wajiha),
            "{}: lahu_khattaf() says {} and the overlay's own backend list says {}",
            wajiha.ism(),
            wajiha.lahu_khattaf(),
            mabniya.contains(&wajiha),
        );
    }
}

/// Every API with a backend must have an import signature that produces it.
///
/// This is the half that was actually broken. Adding a value to
/// [`WajihaRusum`] costs nothing and changes nothing on its own: the probe
/// reports what a detector sets, and a value no detector can set is a value
/// that never appears in a report. Direct3D 8, 9 and 10 would have passed a
/// vocabulary-only check the day they were added and still reported nothing.
#[test]
fn kull_khattaf_lahu_tawqi_istirad() {
    let maarufa = rusum_maarufa();

    for wajiha in mufradat_khattafat() {
        assert!(
            maarufa.contains(&wajiha),
            "{} has an overlay backend and no import in taarib_muharrik::dalail::thunai::DALALAT \
             that establishes it, so a game rendering with it reports no graphics API at all",
            wajiha.ism(),
        );
    }
}

/// The probe may name an API no backend draws, and must never "recognise" the
/// absence of one.
///
/// Metal is the live case for the first half: a macOS binary importing
/// `Metal.framework` is read correctly and no backend draws it, and that is a
/// build fact the report is entitled to state. Deleting the value to make the
/// two lists match would force the probe to report an API it plainly read as
/// undetermined — the same defect this file exists for, inverted.
///
/// The second half is a different kind of mistake: [`WajihaRusum::Majhula`] is
/// the absence of an answer, and a signature that produced it would be a
/// detector claiming to have observed that nothing was observed.
/// `HasilatFahs::daa_rusum` drops it, so such an entry would be silently inert
/// rather than loud.
#[test]
fn al_mufradat_awsa_min_al_khattafat_wa_la_tashmal_al_majhula() {
    let maarufa = rusum_maarufa();

    assert!(
        maarufa.contains(&WajihaRusum::Metal),
        "Metal is still read out of a macOS import table and must stay nameable",
    );
    assert!(
        !WajihaRusum::Metal.lahu_khattaf(),
        "a Metal backend now exists; mufrada() and this test both need updating",
    );
    assert!(
        !maarufa.contains(&WajihaRusum::Majhula),
        "an import signature resolves to Majhula, which HasilatFahs::daa_rusum discards — the \
         entry is inert and the API it meant to establish is unreported",
    );
    assert!(
        !WajihaRusum::Majhula.lahu_khattaf(),
        "nothing can attach to an API that was never determined",
    );
}

/// The three values this file was written for, pinned by name.
///
/// A regression test rather than a design one. The evidence is a real import
/// table — `Bin32/bio4.exe` imports `d3d9.dll` and `d3dx9_43.dll` — and the
/// three signatures are what turns that reading into a reported API. Removing
/// any of them puts the product back where it was: a built backend, a readable
/// import, and a report that says the graphics API was not determined.
#[test]
fn al_ajyal_al_qadima_lahaa_tawaqee() {
    let maarufa = rusum_maarufa();

    for wajiha in [WajihaRusum::D3d8, WajihaRusum::D3d9, WajihaRusum::D3d10] {
        assert!(
            maarufa.contains(&wajiha),
            "{} lost its import signature",
            wajiha.ism(),
        );
        assert!(
            wajiha.lahu_khattaf(),
            "{} lost its overlay backend",
            wajiha.ism()
        );
    }
}
