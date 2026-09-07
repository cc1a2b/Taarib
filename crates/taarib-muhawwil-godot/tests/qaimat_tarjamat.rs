//! The game's own translation list, and what rung one does when nobody read it.
//!
//! `locale/translations` — `internationalization/locale/translations` on Godot
//! 4 — replaces the whole list rather than adding to it. A Godot 3 game
//! shipping `en`, `fr`, `de` and `es` whose override names only the patch's
//! resource starts with Arabic and nothing else, and stays that way until the
//! player finds and deletes `override.cfg`. Until this suite, both deliveries
//! built that list from a `Vec::new()` nobody was required to fill, and the
//! success line then reported it as "the game's own 0 entries beside the
//! patch's" — as success.
//!
//! So the list is a two-state value the caller supplies at construction, and
//! what is checked here is the writer's behaviour in each state: read and empty
//! writes the patch alone, read and full writes the game's entries first, and
//! not read writes the key nowhere — refusing the whole rung on Godot 3, where
//! the key is the only delivery there is, and leaving the key out on Godot 4,
//! where the extension is a second route to the same translation.

#![allow(
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; the lints are written for library code, and refusing to panic here \
              would mean a test that cannot fail"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory; the product's own recursive deletes go through `HadafHadhf`"
)]

use std::path::{Path, PathBuf};

use taarib_muhawwil_godot::khadim_nusus::{
    KhadimNusus, Lahja, MIFTAH_ITTIJAH_JIDHR, MIFTAH_TARJAMAT as MIFTAH_RABI, MadkhalIdad,
    MalafTajawuz, QeemaIdad, Rutba, TarjamatLuba,
};
use taarib_muhawwil_godot::pck::tarjama::{JeelMawrid, ShaklRasail, Tarjama};
use taarib_muhawwil_godot::tawseel::{
    MIFTAH_TARJAMAT as MIFTAH_THALITH, MawdiTarjama, RutbatThalith, TawseelThalith,
};

/// The entries a game that ships two languages declares.
const LUBA: [&str; 2] = ["res://locale/en.translation", "res://locale/fr.translation"];

/// Where the Godot 3 delivery puts its resource, as the setting names it.
const MARJA_THALITH: &str = "user://taarib/ar.translation";

/// Where the Godot 4 delivery's resource lives inside the patch package.
const MARJA_RABI: &str = "res://taarib/ar.translation";

/// A directory of this test's own, removed and recreated so a rerun starts
/// clean.
fn mujallad_ikhtibar(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-godot-qaima-{ism}"));
    let _ = std::fs::remove_dir_all(&masar);
    assert!(
        std::fs::create_dir_all(&masar).is_ok(),
        "{} could not be created",
        masar.display()
    );
    masar
}

/// The patch's messages.
fn tarjama() -> Tarjama {
    let mut tarjama = Tarjama::jadeeda("ar", ShaklRasail::Masfufa);
    assert!(tarjama.daa("Hello", "مرحبا").is_ok());
    assert!(tarjama.daa("New Game", "لعبة جديدة").is_ok());
    tarjama
}

/// The Godot 3 delivery against a directory, with the list the test chose.
fn thalith(jidhr: &Path, qaima: TarjamatLuba) -> TawseelThalith {
    let Ok(mawdi) = MawdiTarjama::mustakhdim(&jidhr.join("mustakhdim"), "taarib/ar.translation")
    else {
        panic!("the user-directory location was refused");
    };
    TawseelThalith::jadeed(MalafTajawuz::fi_mujallad(jidhr), mawdi, qaima)
        .bi_tarjama(tarjama())
        .bi_muharrik((3, 6))
}

/// The Godot 4 delivery against a directory, with the list the test chose.
fn rabi(jidhr: &Path, qaima: TarjamatLuba) -> KhadimNusus {
    let Ok(bayt) = tarjama().ila_bayt(JeelMawrid::Rabi, (4, 3)) else {
        panic!("the Godot 4 resource did not write");
    };
    let Ok(khadim) = KhadimNusus::jadeed(MalafTajawuz::fi_mujallad(jidhr), qaima).bi_tarjama(bayt)
    else {
        panic!("a two-message translation is under every ceiling");
    };
    khadim
}

/// The list value written under one key, if that key is written at all.
fn qaima_min(madakhil: &[MadkhalIdad], miftah: &str) -> Option<Vec<String>> {
    let madkhal = madakhil.iter().find(|madkhal| madkhal.miftah == miftah)?;
    match &madkhal.qeema {
        QeemaIdad::Qaima(qaima) => Some(qaima.clone()),
        akhar => panic!("{miftah} is not a list: {akhar:?}"),
    }
}

/// The game's two entries, owned.
fn qaimat_luba() -> Vec<String> {
    LUBA.iter().map(|masar| (*masar).to_owned()).collect()
}

// ---------------------------------------------------------------------------
// Godot 3
// ---------------------------------------------------------------------------

/// A list nobody read is not written, and neither is anything else: rung one
/// has no way to name the resource that does not replace the game's list.
#[test]
fn thalith_qaima_ghayr_maqrua_la_yaktub_shayan() {
    let jidhr = mujallad_ikhtibar("thalith-lam-tuqra");
    let tawseel = thalith(
        &jidhr,
        TarjamatLuba::LamTuqra {
            sabab: "the package is encrypted".to_owned(),
        },
    );
    assert!(tawseel.qaimat_tarjamat().is_none());
    assert!(
        qaima_min(&tawseel.madakhil(), MIFTAH_THALITH).is_none(),
        "the key must not be written from a list nobody read"
    );

    let Err(khata) = tawseel.hayyi() else {
        panic!("rung one cannot deliver without the key, so the ladder must refuse")
    };
    let sabab = khata.to_string();
    assert!(
        sabab.contains("the package is encrypted"),
        "the caller's reason is lost: {sabab}"
    );
    assert!(
        sabab.contains("take away every language"),
        "the refusal must say why: {sabab}"
    );
    assert!(
        !tawseel.mawdi().mutlaq_masar().exists(),
        "a resource nothing names is a file nothing loads, and must not be written"
    );
    assert!(
        !tawseel.tajawuz().masar().exists(),
        "an override without the key would only force a locale the game cannot serve"
    );
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A list that was read and found empty is a fact about the game: the patch's
/// entry is written alone, and the game loses nothing.
#[test]
fn thalith_qaima_maqrua_farigha_taktub_alruqaa_wahdaha() {
    let jidhr = mujallad_ikhtibar("thalith-farigha");
    let tawseel = thalith(&jidhr, TarjamatLuba::Maqrua(Vec::new()));
    assert_eq!(
        qaima_min(&tawseel.madakhil(), MIFTAH_THALITH),
        Some(vec![MARJA_THALITH.to_owned()])
    );

    let Ok(natija) = tawseel.hayyi() else {
        panic!("a game that declares no translations loses nothing to the patch's list")
    };
    assert!(natija.wusul());
    let Ok(nass) = std::fs::read_to_string(tawseel.tajawuz().masar()) else {
        panic!("the override was not written");
    };
    assert!(nass.contains(MARJA_THALITH));
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A list that was read is written back first, and the success line counts it.
#[test]
fn thalith_qaimat_alluba_tasbiq_alruqaa() {
    let jidhr = mujallad_ikhtibar("thalith-kamila");
    let tawseel = thalith(&jidhr, TarjamatLuba::Maqrua(qaimat_luba()));
    let qaima = qaima_min(&tawseel.madakhil(), MIFTAH_THALITH).unwrap_or_default();
    assert_eq!(qaima.len(), 3, "{qaima:?}");
    assert_eq!(qaima.first().map(String::as_str), LUBA.first().copied());
    assert_eq!(qaima.last().map(String::as_str), Some(MARJA_THALITH));

    let Ok(natija) = tawseel.hayyi() else {
        panic!("the delivery installed nothing")
    };
    let mulahaza = natija
        .thaqafa
        .rutab
        .iter()
        .find(|rutba| rutba.rutba == RutbatThalith::Idadat)
        .map(|rutba| rutba.mulahaza.clone())
        .unwrap_or_default();
    assert!(
        mulahaza.contains("game's own 2 entry/entries"),
        "{mulahaza}"
    );
    let _ = std::fs::remove_dir_all(&jidhr);
}

// ---------------------------------------------------------------------------
// Godot 4
// ---------------------------------------------------------------------------

/// A list nobody read leaves the key out and writes everything else, because
/// the extension is a second route to the translation and the direction, the
/// text driver and the font have no other rung one.
#[test]
fn rabi_qaima_ghayr_maqrua_yatruk_almiftah_wa_yaktub_albaqi() {
    let jidhr = mujallad_ikhtibar("rabi-lam-tuqra");
    let khadim = rabi(
        &jidhr,
        TarjamatLuba::LamTuqra {
            sabab: "project.binary did not parse".to_owned(),
        },
    );
    let madakhil = khadim.madakhil();
    assert!(qaima_min(&madakhil, MIFTAH_RABI).is_none(), "{madakhil:?}");
    assert!(
        madakhil
            .iter()
            .any(|madkhal| madkhal.miftah == MIFTAH_ITTIJAH_JIDHR),
        "the other keys are still rung one's to write"
    );

    let Ok(natija) = khadim.hayyi(None) else {
        panic!("Godot 4 has the extension as a second route, so rung one writes what it may")
    };
    let Ok(nass) = std::fs::read_to_string(khadim.tajawuz().masar()) else {
        panic!("the override was not written");
    };
    assert!(!nass.contains("translations="), "{nass}");
    assert!(nass.contains("root_node_layout_direction="), "{nass}");
    let mulahaza = natija
        .thaqafa
        .rutab
        .iter()
        .find(|rutba| rutba.rutba == Rutba::Idadat)
        .map(|rutba| rutba.mulahaza.clone())
        .unwrap_or_default();
    assert!(
        mulahaza.contains("project.binary did not parse"),
        "{mulahaza}"
    );
    assert!(mulahaza.contains("left unwritten"), "{mulahaza}");
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A list that was read is written back in full, the game's entries first.
#[test]
fn rabi_qaima_maqrua_tuktab_kamila() {
    let jidhr = mujallad_ikhtibar("rabi-kamila");
    let khadim = rabi(&jidhr, TarjamatLuba::Maqrua(qaimat_luba()));
    let qaima = qaima_min(&khadim.madakhil(), MIFTAH_RABI).unwrap_or_default();
    assert_eq!(qaima.len(), 3, "{qaima:?}");
    assert_eq!(qaima.first().map(String::as_str), LUBA.first().copied());
    assert_eq!(qaima.last().map(String::as_str), Some(MARJA_RABI));

    let Ok(natija) = khadim.hayyi(None) else {
        panic!("the offline run writes the override")
    };
    let mulahaza = natija
        .thaqafa
        .rutab
        .iter()
        .find(|rutba| rutba.rutba == Rutba::Idadat)
        .map(|rutba| rutba.mulahaza.clone())
        .unwrap_or_default();
    assert!(!mulahaza.contains("left unwritten"), "{mulahaza}");
    let _ = std::fs::remove_dir_all(&jidhr);
}

// ---------------------------------------------------------------------------
// The override file
// ---------------------------------------------------------------------------

/// Being handed nothing to write is not a write, and does not report as one.
#[test]
fn tajawuz_bila_madakhil_yarfud_wa_la_yaktub() {
    let jidhr = mujallad_ikhtibar("tajawuz-farigh");
    let tajawuz = MalafTajawuz::fi_mujallad(&jidhr);
    assert!(
        tajawuz.aktub_bi(Lahja::Rabi, &[]).is_err(),
        "an empty write is not a write"
    );
    assert!(tajawuz.aktub(&[]).is_err());
    assert!(!tajawuz.masar().exists());
    let _ = std::fs::remove_dir_all(&jidhr);
}
