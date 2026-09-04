//! الأخطاء عبر الحدود — how a failure crosses into JavaScript without losing
//! anything.
//!
//! Every failure in Taarib is a [`Khata`]: a permanent code, a sentence in
//! Arabic, the same sentence in English, and one concrete next action. The C
//! ABI carries those four things through a status code plus retrieval
//! functions; JavaScript has a richer native failure value, so here the whole
//! thing becomes one thrown `Error` and nothing has to be retrieved:
//!
//! - `message` — the Arabic sentence, because user-visible text in this
//!   product is Arabic first and `message` is what an unhandled rejection
//!   prints;
//! - `name` — `"TaaribKhata"`, so a `catch` can tell this module's failures
//!   from the host's with one comparison and no `instanceof` across realms;
//! - `ramz` — the permanent code as text, `TAARIB-E-2501`, exactly what a
//!   user pastes into a bug report;
//! - `injilizi` — the English sentence;
//! - `khutwa` — the next-action discriminant, numbered identically to the C
//!   ABI's so an adapter ported from C# keeps its `switch` unchanged;
//! - `khutura` — the severity discriminant;
//! - `sijill` — the one-line log rendering of the whole cause chain, for the
//!   adapter's own console output.
//!
//! `throw new Error("failed")` would discard the entire model; this module
//! exists so no export can.

use js_sys::Reflect;
use taarib_usus::khata::{Khata, Khutura as KhuturaDakhili, Khutwa as KhutwaDakhili, Natija};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// The next action a failure carries, as JavaScript sees it.
///
/// These are names for the numbers in the thrown error's `khutwa` property.
/// The numbering is the C ABI's (`taarib-jisr`'s `khata_c::raqm_khutwa`),
/// value for value, and is additive-only: a new action takes the next free
/// number, and an adapter that meets a number it does not know shows the
/// sentence without a button rather than showing nothing.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Khutwa {
    /// Nothing to do; the message is complete on its own.
    LaShay = 0,
    /// Try the same operation again.
    AadaMuhawala = 1,
    /// Rescan the library.
    AadaFahsMaktaba = 2,
    /// Re-probe this game's engine.
    AadaFahsMuharrik = 3,
    /// Point Taarib at a path it could not find.
    IkhtiyarMasar = 4,
    /// Choose a different font — the current one cannot render Arabic.
    IkhtiyarKhattAakhar = 5,
    /// Open a section of Settings.
    FathIdadat = 6,
    /// Open Diagnostics.
    FathTashkhis = 7,
    /// Open the overflow report.
    FathTaqreerTajawuz = 8,
    /// Open the offending strings in the workspace.
    FathNusus = 9,
    /// Update Taarib itself.
    TahdithTaarib = 10,
    /// Reinstall the framework for this game.
    IadatTarkibIttar = 11,
    /// Uninstall the patch and restore the game.
    IlghaTathbeet = 12,
    /// Re-match the patch against the game's new build.
    IadatMutabaqaBina = 13,
    /// Verify the game's files through its own launcher.
    TahaqquqSalamatLuba = 14,
    /// Contact the patch's contributor.
    IblaghLilMusahim = 15,
    /// Send a diagnostics bundle to the project owner.
    IblaghLilMalik = 16,
    /// Free disk space and retry.
    TahrirMasaha = 17,
    /// Grant the permission the operating system refused.
    ManhSalahiya = 18,
}

/// How badly a failure hurts, as JavaScript sees it.
///
/// Names for the numbers in the thrown error's `khutura` property.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Khutura {
    /// Worth recording, invisible to the user.
    Maluma = 0,
    /// The operation continued with a reduced result.
    Tanbeeh = 1,
    /// The operation failed.
    Khatar = 2,
    /// The operation failed and left something needing attention.
    Fadih = 3,
}

/// The `name` every thrown error carries, so a `catch` block can recognise
/// this module's failures without `instanceof`.
pub(crate) const ISM_KHATA: &str = "TaaribKhata";

/// Converts a [`Khata`] into the JavaScript error value the exports throw.
///
/// The Arabic sentence is the `message`; the code, the English sentence, the
/// action and the severity ride as own-properties. Attaching a property can
/// itself fail only on a frozen or exotic object, which a freshly constructed
/// `Error` is not; the results are discarded rather than unwrapped because a
/// diagnostic that panics while being built would be the one failure worse
/// than the one it describes.
pub(crate) fn ila_js(khata: &Khata) -> JsValue {
    let mabni = js_sys::Error::new(&khata.arabi);
    mabni.set_name(ISM_KHATA);
    let qeema = JsValue::from(mabni);
    let _ = Reflect::set(
        &qeema,
        &JsValue::from_str("ramz"),
        &JsValue::from_str(&khata.ramz.to_string()),
    );
    let _ = Reflect::set(
        &qeema,
        &JsValue::from_str("injilizi"),
        &JsValue::from_str(&khata.injilizi),
    );
    let _ = Reflect::set(
        &qeema,
        &JsValue::from_str("khutwa"),
        &JsValue::from(raqm_khutwa(&khata.khutwa)),
    );
    let _ = Reflect::set(
        &qeema,
        &JsValue::from_str("khutura"),
        &JsValue::from(raqm_khutura(khata.khutura)),
    );
    let _ = Reflect::set(
        &qeema,
        &JsValue::from_str("sijill"),
        &JsValue::from_str(&khata.li_sijill()),
    );
    qeema
}

/// Maps an engine result into the exported form, converting the failure into
/// a throwable error.
///
/// # Errors
///
/// Whatever the wrapped call reported, as the `Error` described on
/// [`ila_js`].
pub(crate) fn min_natija<T>(natija: Natija<T>) -> Result<T, JsValue> {
    natija.map_err(|khata| ila_js(&khata))
}

/// The action discriminant, numbered exactly as the C ABI numbers it.
///
/// This table and `taarib-jisr`'s `khata_c::raqm_khutwa` must never disagree:
/// an adapter that runs against the native library on desktop and against
/// this module inside NW.js switches on the same numbers in both places.
fn raqm_khutwa(khutwa: &KhutwaDakhili) -> u32 {
    match khutwa {
        KhutwaDakhili::LaShay => 0,
        KhutwaDakhili::AadaMuhawala => 1,
        KhutwaDakhili::AadaFahsMaktaba => 2,
        KhutwaDakhili::AadaFahsMuharrik => 3,
        KhutwaDakhili::IkhtiyarMasar { .. } => 4,
        KhutwaDakhili::IkhtiyarKhattAakhar => 5,
        KhutwaDakhili::FathIdadat { .. } => 6,
        KhutwaDakhili::FathTashkhis => 7,
        KhutwaDakhili::FathTaqreerTajawuz => 8,
        KhutwaDakhili::FathNusus => 9,
        KhutwaDakhili::TahdithTaarib => 10,
        KhutwaDakhili::IadatTarkibIttar => 11,
        KhutwaDakhili::IlghaTathbeet => 12,
        KhutwaDakhili::IadatMutabaqaBina => 13,
        KhutwaDakhili::TahaqquqSalamatLuba => 14,
        KhutwaDakhili::IblaghLilMusahim => 15,
        KhutwaDakhili::IblaghLilMalik => 16,
        KhutwaDakhili::TahrirMasaha => 17,
        KhutwaDakhili::ManhSalahiya => 18,
    }
}

/// The severity discriminant behind [`Khutura`].
fn raqm_khutura(khutura: KhuturaDakhili) -> u32 {
    match khutura {
        KhuturaDakhili::Maluma => 0,
        KhuturaDakhili::Tanbeeh => 1,
        KhuturaDakhili::Khatar => 2,
        KhuturaDakhili::Fadih => 3,
    }
}
