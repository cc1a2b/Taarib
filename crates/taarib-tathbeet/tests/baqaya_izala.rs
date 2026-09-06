//! The residue an uninstall cannot take back, and the record that must outlive
//! it.
//!
//! Taarib deploys a framework into the game. That framework then writes its own
//! log, its own cache and its own configuration **into the game**, the first
//! time the game runs — after the manifest was sealed, so no record names them.
//! Measured on two real Unity titles: R.E.P.O. came back from a successful
//! uninstall with eight entries it did not have before, Among Us with six. The
//! product's promise is that an uninstall returns the game byte-identically, and
//! for every engine whose install deploys `BepInEx` that promise was false.
//!
//! Three things compounded it and only the third was fatal. Nothing swept for
//! unrecorded files; `azil_mujallad` turned `DirectoryNotEmpty` into a counter;
//! and it **marked the line done anyway**, so `nazzif_nusakh` would afterwards
//! delete the manifest and the backups — after which nothing on the machine knew
//! those files had ever been Taarib's. The residue outlived the record of it.
//!
//! What is asserted here is the answer chosen for that:
//!
//! 1. the default names every leftover entry instead of counting them;
//! 2. the manifest line stays outstanding, so [`nazzif_nusakh`] **refuses** and
//!    the evidence survives — this is the test the whole file exists for;
//! 3. emptying the directory and running again finishes the line, and the
//!    cleanup then works, so the honest state is recoverable rather than stuck;
//! 4. [`SiyasatIstiada::Kanasa`] returns the tree to byte-identity, and only
//!    then is the record allowed to go;
//! 5. the sweep is bounded by a proof rather than by care — the game root
//!    cannot be a target;
//! 6. the dry run lists the residue **before** anything is deleted, which is the
//!    only thing that makes choosing the sweep a decision rather than a hope;
//! 7. a leftover nested under a recorded subdirectory is attributed once, to the
//!    directory that holds it, rather than repeated under every ancestor.
//!
//! The tree is authored; the shape of the leftovers is not. `sijill_tashghil.log`
//! stands in for `BepInEx/LogOutput.log`, `idadat/itar.cfg` for
//! `BepInEx/config/BepInEx.cfg`, and `mulhaq/` for the `BepInEx/plugins/` a
//! player drops their own mods into — the entry that makes an unconditional
//! sweep unacceptable as a default.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_tathbeet::bayan::{Muthabbit, NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::taraju::{
    RadLaShay, SiyasatIstiada, istiada_nass, khutta, nazzif_nusakh,
};

/// The largest fixture body written. The proof is about names and bytes, not
/// volume.
const AQSA_JISM: usize = 512;

/// Writes a file with deterministic, position-dependent content.
///
/// Not zeroes: these tests compare a tree against itself across an operation,
/// and a directory of identical zero-filled files would compare equal even if
/// the operation had swapped two of them.
fn iktub(masar: &Path, hajm: usize) {
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid).expect("a fixture directory");
    }
    let tul = hajm.min(AQSA_JISM);
    let ism = masar.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let badhra = ism.bytes().fold(17_u8, |akk, bayt| akk.wrapping_mul(31).wrapping_add(bayt));
    let jism: Vec<u8> =
        (0..tul).map(|mawdi| badhra.wrapping_add(u8::try_from(mawdi % 251).unwrap_or(0))).collect();
    fs::write(masar, jism).expect("a fixture file");
}

/// A game directory as the store shipped it, with an empty directory of its own.
///
/// The empty `Capture/` is deliberate. A restore that consumed a directory the
/// game shipped would be as wrong as one that left a directory behind, and only
/// a listing that carries directories can tell the difference.
fn ibni_luba(jidhr: &Path) {
    iktub(&jidhr.join("Luba.exe"), 9_139_840);
    iktub(&jidhr.join("steam_api64.dll"), 316_568);
    iktub(&jidhr.join("Luba_Data/sharedassets0.assets"), 4096);
    fs::create_dir_all(jidhr.join("Capture")).expect("the game's own empty directory");
}

fn tarif(jidhr: &Path) -> TarifLuba {
    let masdar = MasdarLuba::Steam(3_241_660);
    TarifLuba {
        luba: LubaId::min_masdar(&masdar, "R.E.P.O."),
        masdar,
        ism: "R.E.P.O.".to_owned(),
        jidhr: jidhr.to_path_buf(),
        ruqaa: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::jadeeda(1),
        basma_bina: None,
    }
}

/// Every entry under `jidhr`: files with their bytes, directories named as
/// themselves.
///
/// Directories are in the listing on purpose — a restore that leaves an empty
/// `taarib/` behind has not given the game back, and a file-only comparison
/// cannot see that.
fn shajara(jidhr: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut jadwal = BTreeMap::new();
    for madkhal in walkdir::WalkDir::new(jidhr).sort_by_file_name() {
        let madkhal = madkhal.expect("walking the game directory");
        let Ok(nisbi) = madkhal.path().strip_prefix(jidhr) else { continue };
        if nisbi.as_os_str().is_empty() {
            continue;
        }
        let ism = nisbi.to_string_lossy().replace('\\', "/");
        let qeema = if madkhal.file_type().is_dir() {
            None
        } else {
            Some(fs::read(madkhal.path()).expect("reading a game file"))
        };
        let _ = jadwal.insert(ism, qeema);
    }
    jadwal
}

/// The names in a listing, for a message that says which entry appeared.
fn asmaa(jadwal: &BTreeMap<String, Option<Vec<u8>>>) -> Vec<&str> {
    jadwal.keys().map(String::as_str).collect()
}

/// A temporary directory, a game inside it, and a backup root beside it.
struct Masrah {
    _dalil: tempfile::TempDir,
    luba: PathBuf,
    nusakh: PathBuf,
}

impl Masrah {
    fn jadeed() -> Self {
        let dalil = tempfile::tempdir().expect("a temporary directory");
        let luba = dalil.path().join("luba");
        let nusakh = dalil.path().join("nusakh");
        fs::create_dir_all(&luba).expect("the game directory");
        fs::create_dir_all(&nusakh).expect("the backup directory");
        ibni_luba(&luba);
        Self { _dalil: dalil, luba, nusakh }
    }

    /// The install, through the recorder that a real deployment uses.
    ///
    /// [`Muthabbit::ansha`] is the same method `tarkib` writes a framework's
    /// files with, and it is what records the directory chain — which is the
    /// mechanism under test, so it is used rather than imitated.
    fn thabbit(&self) {
        let mut tathbeet =
            Tathbeet::ibda(&self.nusakh, NawTathbeet::Nass, &tarif(&self.luba), "dawra")
                .expect("an installation session");
        tathbeet
            .ansha(&self.luba.join("winhttp.dll"), b"the loader")
            .expect("the loader lands at the game root");
        tathbeet
            .ansha(&self.luba.join("itar/core/itar.dll"), b"the framework")
            .expect("the framework lands two levels down");
        tathbeet
            .ansha(&self.luba.join("itar/mulhaq/taarib.dll"), b"the payload")
            .expect("the payload lands beside where a player would put theirs");
    }

    /// One play session, from the framework's point of view.
    ///
    /// Everything here is written *after* the manifest was sealed and through
    /// nothing Taarib owns, which is precisely why no record names it.
    fn ishtaghil(&self) {
        iktub(&self.luba.join("itar/sijill_tashghil.log"), 748);
        iktub(&self.luba.join("itar/idadat/itar.cfg"), 5564);
        iktub(&self.luba.join("itar/mulhaq/mud_allaib.dll"), 4096);
        fs::create_dir_all(self.luba.join("itar/makhbaa"))
            .expect("a cache directory the framework creates and leaves empty");
    }

    fn masar_bayan(&self) -> PathBuf {
        self.nusakh.join(NawTathbeet::Nass.ism_bayan())
    }
}

// ---------------------------------------------------------------------------
// 1. The residue is named, not counted
// ---------------------------------------------------------------------------

#[test]
fn al_baqaya_tusamma_wa_la_tuadd() {
    let masrah = Masrah::jadeed();
    masrah.thabbit();
    masrah.ishtaghil();

    let taqreer = istiada_nass(
        &masrah.luba,
        &masrah.nusakh,
        SiyasatIstiada::Muhafiza,
        &mut RadLaShay,
    )
    .expect("the uninstall runs to the end and reports what it could not take back");

    assert_eq!(
        taqreer.mujalladat_matruka, 2,
        "`itar` and `itar/mulhaq` both hold files Taarib did not write: {taqreer:?}"
    );
    assert!(!taqreer.nazif(), "and the game is therefore not what it was");

    let mut mismar: Vec<(&str, Vec<&str>)> = taqreer
        .baqaya
        .iter()
        .map(|baqiya| {
            (baqiya.mujallad.as_str(), baqiya.madakhil.iter().map(String::as_str).collect())
        })
        .collect();
    mismar.sort_by(|awwal, thani| awwal.0.cmp(thani.0));
    assert_eq!(
        mismar,
        vec![
            ("itar", vec![
                "itar/idadat/",
                "itar/idadat/itar.cfg",
                "itar/makhbaa/",
                "itar/sijill_tashghil.log",
            ]),
            ("itar/mulhaq", vec!["itar/mulhaq/mud_allaib.dll"]),
        ],
        "every leftover entry is named, directories included and marked as such; a count \
         alone tells a user something is left and withholds the only part they can act on"
    );
    assert!(taqreer.maknusa.is_empty(), "and nothing was removed: {:?}", taqreer.maknusa);

    let sutur = taqreer.taqreer();
    assert!(
        sutur.iter().any(|satr| satr.contains("Taarib did not put there")),
        "the report says so in its own words: {sutur:?}"
    );
    assert!(
        sutur.iter().any(|satr| satr.contains("itar/sijill_tashghil.log")),
        "and names the file rather than only the directory: {sutur:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. The record is not discarded while the residue exists
// ---------------------------------------------------------------------------

/// The one that matters. A line marked done is a line `nazzif_nusakh` consumes,
/// and consuming the last one deletes the manifest and the preserved originals —
/// after which the leftovers standing in the game are named by no record on the
/// machine and no later run can find them. So the line is not marked done.
#[test]
fn al_sijill_la_yuhdhaf_ma_dama_athar_baq() {
    let masrah = Masrah::jadeed();
    masrah.thabbit();
    masrah.ishtaghil();

    let taqreer = istiada_nass(
        &masrah.luba,
        &masrah.nusakh,
        SiyasatIstiada::Muhafiza,
        &mut RadLaShay,
    )
    .expect("the uninstall completes");
    assert!(taqreer.mujalladat_matruka > 0);

    let khata = nazzif_nusakh(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect_err("the cleanup must refuse while anything is still standing in the game");

    // The reason lives in the variant's payload, not its one-line summary —
    // and it is the reason, not the summary, that a user is shown.
    let nass = taarib_usus::khata::Tafsir::injilizi(&khata);
    assert!(
        nass.contains("itar"),
        "and must name the directories rather than refusing blankly: {nass}"
    );
    assert!(
        nass.contains("attributable"),
        "saying why the record is being kept, which is the fact a user needs: {nass}"
    );

    assert!(
        masrah.masar_bayan().is_file(),
        "the manifest survives — it is the only thing that says these files came with Taarib"
    );
    assert!(
        masrah.nusakh.join(NawTathbeet::Nass.ism_mujallad()).is_dir(),
        "and so do the preserved originals it names"
    );

    // Reopening it proves the record is not merely present but *outstanding*:
    // a manifest whose lines were all marked done would be consumed by the very
    // next cleanup, whatever is still on the disk.
    let baqi = Tathbeet::istanif(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect("the manifest reopens");
    assert_eq!(
        baqi.bayan().qaimat_mutabaqqi(),
        vec!["itar".to_owned(), "itar/mulhaq".to_owned()],
        "and the outstanding lines are exactly the directories that would not go"
    );
}

// ---------------------------------------------------------------------------
// 3. The honest state is recoverable, not stuck
// ---------------------------------------------------------------------------

/// Leaving the line outstanding would be a bad answer if it were permanent. A
/// user who deletes the leftovers by hand and runs the uninstall again must find
/// the job finished, not a manifest that refuses forever.
#[test]
fn tafrigh_almujallad_yutimm_alsatr() {
    let masrah = Masrah::jadeed();
    masrah.thabbit();
    masrah.ishtaghil();

    let awwal = istiada_nass(
        &masrah.luba,
        &masrah.nusakh,
        SiyasatIstiada::Muhafiza,
        &mut RadLaShay,
    )
    .expect("the first run completes and leaves the directories");
    assert_eq!(awwal.mujalladat_matruka, 2);

    // The user removes what the report named, by hand, the way anybody would.
    fs::remove_file(masrah.luba.join("itar/sijill_tashghil.log")).expect("a named leftover");
    fs::remove_file(masrah.luba.join("itar/idadat/itar.cfg")).expect("a named leftover");
    fs::remove_dir(masrah.luba.join("itar/idadat")).expect("a named leftover directory");
    fs::remove_dir(masrah.luba.join("itar/makhbaa")).expect("a named leftover directory");
    fs::remove_file(masrah.luba.join("itar/mulhaq/mud_allaib.dll")).expect("their own mod");

    let thani = istiada_nass(
        &masrah.luba,
        &masrah.nusakh,
        SiyasatIstiada::Muhafiza,
        &mut RadLaShay,
    )
    .expect("the second run picks the manifest up where the first left it");
    assert_eq!(thani.mujalladat_matruka, 0, "nothing is left standing now: {thani:?}");
    assert_eq!(
        thani.mujalladat_muzala, 2,
        "and the two lines the first run left outstanding came off — `itar/core` was empty \
         and already done, which is what makes this a resumption and not a repeat: {thani:?}"
    );
    assert!(thani.nazif());

    let _ = nazzif_nusakh(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect("and only now may the record go");
    assert!(!masrah.masar_bayan().exists(), "the manifest is gone");
}

// ---------------------------------------------------------------------------
// 4. The sweep, and byte-identity
// ---------------------------------------------------------------------------

/// The other answer, for a user who has read the list and wants their game back.
/// It is the only operation in this module that destroys something nobody
/// recorded, which is why it is opt-in and why the report says what went.
#[test]
fn alkans_yuid_alluba_mutabiqa_bayt_bi_bayt() {
    let masrah = Masrah::jadeed();
    let qabl = shajara(&masrah.luba);
    masrah.thabbit();
    masrah.ishtaghil();
    assert_ne!(shajara(&masrah.luba), qabl, "the install and the session both landed");

    let taqreer =
        istiada_nass(&masrah.luba, &masrah.nusakh, SiyasatIstiada::Kanasa, &mut RadLaShay)
            .expect("the sweep completes");

    assert_eq!(taqreer.mujalladat_matruka, 0, "nothing was left: {taqreer:?}");
    assert!(taqreer.nazif());
    assert!(taqreer.baqaya.is_empty(), "and nothing is reported as left: {:?}", taqreer.baqaya);

    let maknusa: Vec<&str> = taqreer.maknusa.iter().map(|b| b.mujallad.as_str()).collect();
    assert_eq!(
        maknusa,
        vec!["itar/mulhaq", "itar"],
        "what it removed is named, deepest first, in the order it removed them — a sweep \
         that reported only a count would have destroyed a player's mod silently"
    );
    assert!(
        taqreer.taqreer().iter().any(|satr| satr.contains("mud_allaib.dll")),
        "including the file that was a player's own: {:?}",
        taqreer.taqreer()
    );

    let baad = shajara(&masrah.luba);
    assert_eq!(
        asmaa(&baad),
        asmaa(&qabl),
        "not one entry may remain that was not there before the install"
    );
    assert!(baad == qabl, "and every file the game shipped still holds exactly its own bytes");

    let _ = nazzif_nusakh(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect("with nothing left in the game, the record may go");
    assert!(!masrah.masar_bayan().exists());
}

// ---------------------------------------------------------------------------
// 5. The sweep is bounded by a proof, not by care
// ---------------------------------------------------------------------------

/// A sweep aimed at the game root would delete the game. Nothing in `taraju`
/// checks for that; the constructor does, and this is the assertion that the
/// guard is the type rather than a call-site habit somebody must remember.
///
/// The second half is the reason a sweep is defensible at all: the manifest
/// records a directory only when it did not exist at install time, so `Capture/`
/// — which the game shipped — is not a record, is never a target, and survives
/// a sweep that removes everything beside it.
#[test]
fn alkans_la_yastati_bulugh_jidhr_alluba() {
    let masrah = Masrah::jadeed();
    assert!(
        taarib_usus::masarat::hadaf_hadhf_fi_luba(&masrah.luba, &masrah.luba).is_err(),
        "the game directory itself cannot be made into a deletion target"
    );
    assert!(
        taarib_usus::masarat::hadaf_hadhf_fi_luba(&masrah.luba, &masrah.nusakh).is_err(),
        "and neither can anything outside it"
    );
    assert!(
        taarib_usus::masarat::hadaf_hadhf_fi_luba(&masrah.luba, &masrah.luba.join("itar")).is_ok(),
        "while the directory the manifest actually records is a target"
    );

    masrah.thabbit();
    masrah.ishtaghil();
    // A directory the game shipped, with something in it. `sajjil_mujallad`
    // refuses to record a directory that already exists, so it is not a record,
    // so it is not a sweep target however aggressive the policy.
    iktub(&masrah.luba.join("Capture/lightat.png"), 64);

    let _ = istiada_nass(&masrah.luba, &masrah.nusakh, SiyasatIstiada::Kanasa, &mut RadLaShay)
        .expect("the sweep completes");

    assert!(masrah.luba.join("Luba.exe").is_file(), "the game is still there");
    assert!(
        masrah.luba.join("Capture/lightat.png").is_file(),
        "and a file in a directory the game shipped is untouched by a sweep of Taarib's"
    );
    assert!(!masrah.luba.join("itar").exists(), "while Taarib's own directory is gone");
}

// ---------------------------------------------------------------------------
// 6. The dry run shows the list before anything is deleted
// ---------------------------------------------------------------------------

/// Consent to a sweep is only consent if the list came first. This is the call
/// a confirmation screen makes, and it must produce the same names the sweep
/// will act on, while every one of those files is still on the disk.
#[test]
fn alkhutta_tasrud_albaqaya_qabl_hadhf_ayya_bayt() {
    let masrah = Masrah::jadeed();
    masrah.thabbit();
    masrah.ishtaghil();

    let mukhattat = khutta(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect("the dry run reads the manifest");

    assert!(
        !mukhattat.nazif(),
        "an uninstall that will leave a framework's log behind is not a clean one, and the \
         screen that says it is has made the product's central promise untrue"
    );
    let mut asmaa_baqaya: Vec<&str> =
        mukhattat.baqaya.iter().map(|baqiya| baqiya.mujallad.as_str()).collect();
    asmaa_baqaya.sort_unstable();
    assert_eq!(asmaa_baqaya, vec!["itar", "itar/mulhaq"]);
    assert!(
        mukhattat
            .baqaya
            .iter()
            .flat_map(|baqiya| &baqiya.madakhil)
            .any(|ism| ism == "itar/mulhaq/mud_allaib.dll"),
        "and a player's own file is on the list before they are asked: {:?}",
        mukhattat.baqaya
    );

    // Nothing moved. A dry run that deleted anything would not be one.
    assert!(masrah.luba.join("itar/sijill_tashghil.log").is_file());
    assert!(masrah.luba.join("winhttp.dll").is_file());
}

// ---------------------------------------------------------------------------
// 7. A nested leftover is attributed once
// ---------------------------------------------------------------------------

/// `itar/mulhaq/mud_allaib.dll` sits inside a recorded directory that sits
/// inside another recorded directory. Walking each recorded directory naively
/// would report it under both, so a user reading the confirmation screen would
/// be told a two-file residue is a three-file one and would be counting the same
/// deletion twice.
#[test]
fn baqiya_dakhil_mujallad_musajjal_tunsab_marra_wahida() {
    let masrah = Masrah::jadeed();
    masrah.thabbit();
    masrah.ishtaghil();

    let mukhattat = khutta(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect("the dry run reads the manifest");

    let kul: Vec<&String> =
        mukhattat.baqaya.iter().flat_map(|baqiya| &baqiya.madakhil).collect();
    let mut farid = kul.clone();
    farid.sort_unstable();
    farid.dedup();
    assert_eq!(kul.len(), farid.len(), "no entry appears twice: {kul:?}");

    let sahib: Vec<&str> = mukhattat
        .baqaya
        .iter()
        .filter(|baqiya| baqiya.madakhil.iter().any(|ism| ism.ends_with("mud_allaib.dll")))
        .map(|baqiya| baqiya.mujallad.as_str())
        .collect();
    assert_eq!(
        sahib,
        vec!["itar/mulhaq"],
        "and it is attributed to the directory that holds it, not to its grandparent"
    );

    let adad_kulli: usize = mukhattat.baqaya.iter().map(|baqiya| baqiya.adad).sum();
    assert_eq!(adad_kulli, 5, "four entries under itar, one under itar/mulhaq");
}
