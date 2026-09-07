//! The launch options Taarib adds to Steam, and the ones it must never take
//! away.
//!
//! Steam keeps one `localconfig.vdf` per signed-in account, and a game's launch
//! options are one string in it. It is a string the *user* owns: a `-dx11`, a
//! `--skip-launcher`, a `%command%` wrapper another tool put there. Taarib adds
//! an environment assignment in front of it because a framework deployed into a
//! Proton game is loaded from nowhere else, and it therefore has to be able to
//! take that assignment back out again without touching a character of what was
//! already there.
//!
//! Both halves have to exist or neither may. An install that writes and an
//! uninstall that cannot remove leaves Taarib's preload in the field forever; an
//! uninstall that removes without a per-account record of what was really there
//! restores one account's options into another account's field. That asymmetry
//! is what this file is a proof against, so it exercises the two together and
//! never separately:
//!
//! 1. four shapes of user-authored value survive a full cycle byte-identically —
//!    a bare flag, a flag whose argument is quoted and contains spaces, another
//!    tool's `%command%` wrapper, and a field that was empty or absent;
//! 2. a field edited by hand after the install stops the uninstall by name,
//!    rather than being overwritten with a value that is no longer anybody's;
//! 3. the install refuses while Steam is up, because Steam rewrites this file
//!    from memory when it exits and an edit made now is erased with no trace;
//! 4. a machine with two signed-in accounts has both written and both restored,
//!    each against its *own* previous value.
//!
//! The fixture is authored rather than copied, but its shape is the real one:
//! tabs, `"key"\t\t"value"`, `apps` five levels down under
//! `UserLocalConfigStore/Software/Valve/Steam`, and a block after `apps` so that
//! an edit which moved the rest of the file would show up as a difference.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_tathbeet::bayan::{MahallIdad, NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::itlaq::{
    KhiyaratSteam, MANASSA_STEAM, MUTAGHAYYIR_TAJAWUZ, RadItlaq, TAJAWUZ_TAARIB,
    naffidh_talabat_steam, talabat_steam,
};
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, istiada_nass};
use taarib_usus::khata::Tafsir as _;
use taarib_usus::manassa::{HalatTashghil, halat_tashghil};

/// The game this fixture patches, as Steam numbers it.
const APP: &str = "3241660";

/// Another game in the same file, which nothing here may touch.
const APP_AKHAR: &str = "620";

/// The assignment the deployment asks for, exactly as `tarkib` records it.
fn matlub() -> String {
    format!("{MUTAGHAYYIR_TAJAWUZ}=\"{TAJAWUZ_TAARIB}\" %command%")
}

/// Whether this machine is one where a launcher edit may happen at all.
///
/// Not a skip. A machine with Steam open — or one inside a sandbox that cannot
/// see whether Steam is open — must *refuse*, and that refusal is asserted
/// instead of the cycle. Both are the behaviour under test; which one runs is
/// decided by the machine rather than by this file.
fn steam_maftuh() -> bool {
    ["steam.exe", "steam", "steam_osx"]
        .iter()
        .any(|ism| !matches!(halat_tashghil(ism), HalatTashghil::LaTashtaghil))
}

/// One account's `localconfig.vdf`, with this game's launch options as given.
///
/// `khiyarat` is written into the file verbatim, so a caller passing
/// `-name \"x y\"` has to escape it the way `KeyValues` does — which is the
/// point: the bytes this function produces are the bytes the cycle must return
/// the file to.
fn wathiqat_hisab(khiyarat: Option<&str>) -> String {
    let satr_khiyarat = khiyarat
        .map(|qeema| format!("\t\t\t\t\t\t\"LaunchOptions\"\t\t\"{qeema}\"\n"))
        .unwrap_or_default();
    format!(
        "\"UserLocalConfigStore\"\n\
         {{\n\
         \t\"Software\"\n\
         \t{{\n\
         \t\t\"Valve\"\n\
         \t\t{{\n\
         \t\t\t\"Steam\"\n\
         \t\t\t{{\n\
         \t\t\t\t\"apps\"\n\
         \t\t\t\t{{\n\
         \t\t\t\t\t\"{APP_AKHAR}\"\n\
         \t\t\t\t\t{{\n\
         \t\t\t\t\t\t\"LastPlayed\"\t\t\"1788705467\"\n\
         \t\t\t\t\t\t\"LaunchOptions\"\t\t\"-novid -console\"\n\
         \t\t\t\t\t}}\n\
         \t\t\t\t\t\"{APP}\"\n\
         \t\t\t\t\t{{\n\
         \t\t\t\t\t\t\"LastPlayed\"\t\t\"1779710648\"\n\
         {satr_khiyarat}\
         \t\t\t\t\t\t\"Playtime\"\t\t\"73\"\n\
         \t\t\t\t\t}}\n\
         \t\t\t\t}}\n\
         \t\t\t}}\n\
         \t\t}}\n\
         \t}}\n\
         \t\"Broadcast\"\n\
         \t{{\n\
         \t\t\"Permissions\"\t\t\"1\"\n\
         \t}}\n\
         }}\n"
    )
}

/// A temporary machine: a Steam root with accounts, a game, and a backup root.
struct Masrah {
    _dalil: tempfile::TempDir,
    steam: PathBuf,
    luba: PathBuf,
    nusakh: PathBuf,
    hisabat: Vec<PathBuf>,
}

impl Masrah {
    /// One account per entry, each with the launch options it is given.
    fn jadeed(hisabat: &[Option<&str>]) -> Self {
        let dalil = tempfile::tempdir().expect("a temporary directory");
        let steam = dalil.path().join("Steam");
        let luba = dalil.path().join("luba");
        let nusakh = dalil.path().join("nusakh");
        fs::create_dir_all(&luba).expect("the game directory");
        fs::create_dir_all(&nusakh).expect("the backup directory");
        fs::write(luba.join("Luba.exe"), b"the game").expect("a game file");

        let mut masarat = Vec::with_capacity(hisabat.len());
        for (mawdi, khiyarat) in hisabat.iter().enumerate() {
            // Account numbers Steam would recognise, and never zero: that
            // directory is the one Steam keeps for "no account" and holds no
            // launch options.
            let raqm = 827_580_076_u64 + mawdi as u64;
            let tahyia = steam.join("userdata").join(raqm.to_string()).join("config");
            fs::create_dir_all(&tahyia).expect("an account configuration directory");
            let masar = tahyia.join("localconfig.vdf");
            fs::write(&masar, wathiqat_hisab(*khiyarat)).expect("an account configuration");
            masarat.push(masar);
        }

        Self { _dalil: dalil, steam, luba, nusakh, hisabat: masarat }
    }

    fn tarif(&self) -> TarifLuba {
        let masdar = MasdarLuba::Steam(APP.parse().expect("a numeric app id"));
        TarifLuba {
            luba: LubaId::min_masdar(&masdar, "Luba"),
            masdar,
            ism: "Luba".to_owned(),
            jidhr: self.luba.clone(),
            ruqaa: RuqaaId::jadeeda(),
            murajaa: RuqaaRevision::jadeeda(1),
            basma_bina: None,
        }
    }

    /// The bytes of every account file, in account order.
    fn wathaiq(&self) -> Vec<String> {
        self.hisabat
            .iter()
            .map(|masar| fs::read_to_string(masar).expect("an account configuration"))
            .collect()
    }

    /// This game's launch options in one account, as the file now holds them.
    fn khiyarat(&self, hisab: usize) -> Option<String> {
        KhiyaratSteam::jadeeda(
            self.hisabat.get(hisab).expect("an account this fixture has").clone(),
            APP,
        )
        .qeema_haliya()
        .expect("the account file parses")
    }

    /// The install, through the same manifest the real pipeline uses.
    ///
    /// The requirement is recorded exactly as `tarkib::rakkib_mukawwin` records
    /// it — same variant, same launcher identifier, same composed value — so
    /// what is performed here is what a real deployment states, not a shape
    /// invented for the test.
    fn thabbit(&self) -> Result<usize, KhataTathbeet> {
        let mut tathbeet =
            Tathbeet::ibda(&self.nusakh, NawTathbeet::Nass, &self.tarif(), "dawra")
                .expect("an installation session");
        tathbeet
            .sajjil_idad(
                MahallIdad::KhiyaratTashghil {
                    manassa: MANASSA_STEAM.to_owned(),
                    muarrif_luba: format!("{MANASSA_STEAM}:{APP}"),
                },
                None,
                Some(matlub()),
            )
            .expect("the deployment records what the field must hold");

        let talabat = talabat_steam(tathbeet.bayan());
        assert_eq!(talabat.len(), 1, "one requirement was recorded, so one must be found");
        naffidh_talabat_steam(&mut tathbeet, &talabat, Some(self.steam.as_path()))
    }

    /// The uninstall, through the entry point every production caller uses —
    /// including the restorer they all pass, which knows nothing about Steam.
    fn azil(&self) -> Result<usize, KhataTathbeet> {
        istiada_nass(&self.luba, &self.nusakh, SiyasatIstiada::Muhafiza, &mut RadLaShay)
            .map(|taqreer| taqreer.idadat_mustaada)
    }
}

/// Runs the install and says whether this machine allowed it.
///
/// On a machine where Steam is up, the refusal is asserted here and the caller
/// stops — the guard is then the thing being proved, and proving it is not
/// weaker than proving the cycle.
fn thabbit_aw_athbit_al_rafd(masrah: &Masrah) -> bool {
    let qabl = masrah.wathaiq();
    let natija = masrah.thabbit();
    if steam_maftuh() {
        let khata = natija.expect_err("an edit Steam would overwrite from memory must refuse");
        assert!(
            matches!(
                khata,
                KhataTathbeet::MunassaTaamal { .. } | KhataTathbeet::HalatManassaMajhula { .. }
            ),
            "the refusal must be the launcher guard's: {khata:?}"
        );
        assert_eq!(
            masrah.wathaiq(),
            qabl,
            "and not one byte of any account file may have been written first"
        );
        return false;
    }
    let adad = natija.expect("the install applies the requirement");
    assert_eq!(adad, masrah.hisabat.len(), "every account file is written, not just the first");
    true
}

// ---------------------------------------------------------------------------
// 1. A user's own launch options survive the whole cycle
// ---------------------------------------------------------------------------

/// One shape of user-authored value, taken all the way there and back.
fn dawra_kamila(mawjuda: Option<&str>, muzhara: Option<&str>) {
    let masrah = Masrah::jadeed(&[mawjuda]);
    let qabl = masrah.wathaiq();
    if !thabbit_aw_athbit_al_rafd(&masrah) {
        return;
    }

    // The install put Taarib's assignment in and left the user's text alone.
    let baad = masrah.khiyarat(0).expect("the field now holds something");
    assert!(baad.contains(TAJAWUZ_TAARIB), "the loader override is in the field: {baad}");
    assert_eq!(baad.matches(MUTAGHAYYIR_TAJAWUZ).count(), 1, "and exactly once: {baad}");
    if let Some(muzhara) = muzhara {
        assert!(baad.contains(muzhara), "the user's own text is still there: {baad}");
    }
    assert_eq!(
        masrah.wathaiq().first().map(|nass| nass.contains("-novid -console")),
        Some(true),
        "and the other game in the same file was not touched"
    );

    assert_eq!(masrah.azil().expect("the uninstall completes"), 2);

    assert_eq!(
        masrah.wathaiq(),
        qabl,
        "the account file must come back byte-identical: what Taarib added is gone and \
         what the user wrote is exactly as they wrote it"
    );
}

#[test]
fn alam_mujarrad_yanju_min_al_dawra() {
    dawra_kamila(Some("-dx11"), Some("-dx11"));
}

#[test]
fn alam_bi_muamil_muqtabas_yanju_min_al_dawra() {
    // A quoted argument with spaces is one word to the launcher and three to
    // anything that splits on whitespace. The escaping is `KeyValues`', which is
    // what the file really carries.
    dawra_kamila(Some("-name \\\"The Player One\\\" -dx11"), Some("The Player One"));
}

#[test]
fn ghilaf_adat_ukhra_yanju_min_al_dawra() {
    // MangoHud, gamemoderun, a Proton launch script: somebody else's wrapper
    // already owns `%command%`, and Taarib's assignment has to go inside it
    // rather than in front of it or after it.
    dawra_kamila(Some("mangohud %command% -vulkan"), Some("mangohud"));
}

#[test]
fn haql_farigh_yanju_min_al_dawra() {
    // The key is present and holds nothing, which is not the same thing as the
    // key being absent — the uninstall has to leave the empty key behind rather
    // than delete a line the file shipped with.
    dawra_kamila(Some(""), None);
}

#[test]
fn haql_ghayr_mawjud_yanju_min_al_dawra() {
    // No key at all. The install inserts one and the uninstall must remove it
    // whole, indentation and line break included.
    dawra_kamila(None, None);
    let masrah = Masrah::jadeed(&[None]);
    if !thabbit_aw_athbit_al_rafd(&masrah) {
        return;
    }
    assert!(
        masrah.wathaiq().first().is_some_and(|nass| nass.contains("\"LaunchOptions\"")),
        "the key was inserted"
    );
    assert_eq!(masrah.azil().expect("the uninstall completes"), 2);
    let baad = masrah.wathaiq();
    let nass = baad.first().expect("one account");
    assert_eq!(
        nass.matches("\"LaunchOptions\"").count(),
        1,
        "only the other game's key is left: {nass}"
    );
}

// ---------------------------------------------------------------------------
// 2. A field edited by hand after the install
// ---------------------------------------------------------------------------

#[test]
fn tadeel_yadawi_baad_al_tathbeet_yuwqif_al_izala_wa_yusammi_al_sabab() {
    let masrah = Masrah::jadeed(&[Some("-dx11")]);
    if !thabbit_aw_athbit_al_rafd(&masrah) {
        return;
    }

    // The person opens Steam's properties dialog and adds a flag of their own.
    let masar = masrah.hisabat.first().expect("one account").clone();
    let nass = fs::read_to_string(&masar).expect("the account configuration");
    let muharrar = nass.replace("-dx11\"", "-dx11 -windowed\"");
    assert_ne!(muharrar, nass, "the fixture edit has to actually change the field");
    fs::write(&masar, &muharrar).expect("the hand edit");

    let khata = masrah.azil().expect_err("an uninstall may not overwrite a field it did not write");
    let sabab = khata.injilizi();
    assert!(sabab.contains("edited after the install"), "{sabab}");
    assert!(sabab.contains("localconfig.vdf"), "the refusal names which file: {sabab}");
    assert!(sabab.contains(APP), "and which game: {sabab}");
    assert!(!khata.arabi().is_empty(), "and it says so in Arabic too");

    assert_eq!(
        fs::read_to_string(&masar).expect("the account configuration"),
        muharrar,
        "and the field is left exactly as the person left it"
    );
}

#[test]
fn haql_massahahu_al_mustakhdim_la_yubath_min_jadeed() {
    // The other side of the same rule. If the field is empty, what Taarib added
    // is verifiably gone; putting the recorded previous value back would
    // resurrect options the person deleted themselves.
    let masrah = Masrah::jadeed(&[Some("-dx11")]);
    if !thabbit_aw_athbit_al_rafd(&masrah) {
        return;
    }
    let masar = masrah.hisabat.first().expect("one account").clone();
    let nass = fs::read_to_string(&masar).expect("the account configuration");
    let bidaya = nass.find("\"LaunchOptions\"\t\t\"WINEDLLOVERRIDES").expect("Taarib's own line");
    let nihaya = nass.get(bidaya..).and_then(|baqi| baqi.find('\n')).expect("the end of it");
    let mamsuh = format!(
        "{}\"LaunchOptions\"\t\t\"\"{}",
        nass.get(..bidaya).unwrap_or_default(),
        nass.get(bidaya.saturating_add(nihaya)..).unwrap_or_default()
    );
    fs::write(&masar, &mamsuh).expect("the hand edit");

    assert_eq!(masrah.azil().expect("the uninstall completes over a cleared field"), 2);
    assert_eq!(
        fs::read_to_string(&masar).expect("the account configuration"),
        mamsuh,
        "nothing is written back over a field the person cleared"
    );
}

// ---------------------------------------------------------------------------
// 3. Steam running
// ---------------------------------------------------------------------------

#[test]
fn al_tathbeet_yarfud_wa_steam_yaamal() {
    // On a machine with Steam up this is the whole test; on one without it, the
    // same call has to succeed, and asserting both is what makes this a proof
    // about the guard rather than about the machine it ran on.
    let masrah = Masrah::jadeed(&[Some("-dx11")]);
    let qabl = masrah.wathaiq();
    match masrah.thabbit() {
        Err(khata) => {
            assert!(
                steam_maftuh(),
                "the only reason to refuse here is a launcher that was not seen closed: {khata:?}"
            );
            assert!(matches!(
                khata,
                KhataTathbeet::MunassaTaamal { .. } | KhataTathbeet::HalatManassaMajhula { .. }
            ));
            assert_eq!(masrah.wathaiq(), qabl, "and nothing was written before refusing");
        }
        Ok(adad) => {
            assert!(!steam_maftuh(), "a launcher that was seen running must not have been edited");
            assert_eq!(adad, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Two accounts on one machine
// ---------------------------------------------------------------------------

#[test]
fn hisaban_ala_jihaz_wahid_yunalan_al_ithnayn_wa_yustaadan() {
    // The two accounts start from different places on purpose: one person set a
    // flag, the other never opened the properties dialog. A single record could
    // hold only one of those previous values, and restoring it into both files
    // is how an uninstall writes one person's launch options into the other
    // person's account.
    let masrah = Masrah::jadeed(&[Some("-dx11"), None]);
    let qabl = masrah.wathaiq();
    if !thabbit_aw_athbit_al_rafd(&masrah) {
        return;
    }

    let awwal = masrah.khiyarat(0).expect("the first account's field");
    let thani = masrah.khiyarat(1).expect("the second account's field");
    assert!(awwal.contains(TAJAWUZ_TAARIB), "the first account is written: {awwal}");
    assert!(thani.contains(TAJAWUZ_TAARIB), "and so is the second: {thani}");
    assert!(awwal.contains("-dx11"), "the first account keeps its own flag: {awwal}");
    assert!(!thani.contains("-dx11"), "and the second never acquires it: {thani}");

    // Two per-account records plus the requirement they were written from.
    assert_eq!(masrah.azil().expect("the uninstall completes"), 3);
    assert_eq!(
        masrah.wathaiq(),
        qabl,
        "both accounts come back byte-identical, each to its own previous value"
    );
}

#[test]
fn al_izala_tabni_kuttabaha_min_al_bayan_wahdahu() {
    // The restorer takes no configuration: the record names the account file, so
    // an uninstall on a machine whose Steam has since moved — or whose settings
    // were never asked — still edits exactly the files the install edited.
    let masrah = Masrah::jadeed(&[Some("-dx11"), None]);
    if !thabbit_aw_athbit_al_rafd(&masrah) {
        return;
    }

    let tathbeet = Tathbeet::istanif(&masrah.luba, &masrah.nusakh, NawTathbeet::Nass)
        .expect("the manifest the install left");
    let sijillat: Vec<_> = tathbeet.bayan().idadat().cloned().collect();
    let radd = RadItlaq::min_sijillat(&sijillat);
    assert_eq!(radd.adad(), 2, "one writer per account, built from the manifest alone");

    for sijill in &sijillat {
        assert!(
            radd.yamlik(&sijill.mahall),
            "every record this install wrote is one the launch integration owns: {}",
            sijill.muarrif
        );
        if let MahallIdad::MalafIdad { masar, .. } = &sijill.mahall {
            assert!(
                masrah.hisabat.iter().any(|hisab| hisab == Path::new(masar)),
                "and it names a real account file: {masar}"
            );
        }
    }
}

#[test]
fn jidhr_steam_bila_hisabat_yuwqif_al_tathbeet() {
    // An empty listing is not a clean result. A Steam root with no account
    // configuration under it is a root the requirement cannot be written to, and
    // an install that walked zero accounts and returned "nothing to do" would
    // report success over a framework nothing will ever load.
    let masrah = Masrah::jadeed(&[]);
    fs::create_dir_all(masrah.steam.join("userdata")).expect("an empty userdata directory");

    let khata = masrah.thabbit().expect_err("a root with no account file must stop the install");
    if steam_maftuh() {
        // On this machine the guard fires first, which is also correct.
        assert!(matches!(
            khata,
            KhataTathbeet::MunassaTaamal { .. } | KhataTathbeet::HalatManassaMajhula { .. }
        ));
        return;
    }
    assert!(matches!(khata, KhataTathbeet::IdadGhayrMunaffadh { .. }), "{khata:?}");
    assert!(khata.injilizi().contains("nowhere to put the launch options"), "{}", khata.injilizi());
}

#[test]
fn talab_bila_sijillat_hisabat_la_yudda_mustaadan() {
    // A requirement recorded and never performed. The restorer must not answer
    // for it: reporting it restored would be reporting a setting put back that
    // was never applied, which is the exact failure this whole file exists to
    // make impossible.
    let mahall = MahallIdad::KhiyaratTashghil {
        manassa: MANASSA_STEAM.to_owned(),
        muarrif_luba: format!("{MANASSA_STEAM}:{APP}"),
    };
    assert!(!RadItlaq::min_sijillat(&[]).yamlik(&mahall));

    let masrah = Masrah::jadeed(&[Some("-dx11")]);
    let mut tathbeet =
        Tathbeet::ibda(&masrah.nusakh, NawTathbeet::Nass, &masrah.tarif(), "dawra")
            .expect("an installation session");
    tathbeet
        .sajjil_idad(mahall, None, Some(matlub()))
        .expect("a requirement recorded and deliberately not performed");
    drop(tathbeet);

    let khata = masrah.azil().expect_err("an unperformed requirement is not a restored setting");
    assert!(khata.injilizi().contains("no way to write settings back"), "{}", khata.injilizi());
    assert_eq!(
        masrah.khiyarat(0).as_deref(),
        Some("-dx11"),
        "and the field was never touched by either half"
    );
}
