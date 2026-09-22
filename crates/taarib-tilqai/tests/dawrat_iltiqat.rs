//! دورة الالتقاط — the capture loop, driven end to end over real session files.
//!
//! The loop this file pins is: play once with capture on → read the session →
//! merge it into the static extraction → top the project rows up → the row count
//! moves. Every link in it has been broken at some point, and each break looked
//! like success from the outside: a run that reported a merge and translated
//! nothing, a session file whose last forty bytes were missing and cost the
//! whole capture, a resumed run that read the old rows and ignored the new
//! table. None of those is visible in a log.
//!
//! So the inputs here are not synthesised. `tests/jalsat/` holds three sessions
//! an adapter actually wrote inside R.E.P.O. — one rich and clean, one whose
//! last line is cut off, and the clean re-run of that same pass — and
//! `tests/jalsat/sakina.json` is a slice of that game's own static extraction,
//! taken from the localization bundles, which on a Unity release build is the
//! only half a file reader can open. That combination is the owner's exact
//! situation: menus readable, everything drawn by a component not.

use std::error::Error;
use std::path::{Path, PathBuf};

use taarib_istikhraj::dammij::{KhiyaratDammij, dammij_iltiqat};
use taarib_istikhraj::iltiqat::{JalsaMuhammala, iqra_jalsa};
use taarib_istikhraj::jadwal::{JadwalNusus, MudkhalMustakhraj};
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_mustalahat::nass::{MasdarIstikhraj, MudkhalNass};
use taarib_tarqee::taghtiya_ruqaa::{MajmuatAwwal, ihsib_taghtiya};
use taarib_tilqai::istikhraj::{HADD_JALSA, ajmaa_mulahazat, basmat_jalsa, basmat_mulahazat};
use taarib_tilqai::mashwar::{
    ISDAR_SIJILL, MashwarId, QaydMarhala, SijillMashwar, TarwisatMashwar,
};
use taarib_tilqai::{KhataTilqai, MarhalaTilqai, MukhbirTaqaddum, Muraqib, damm_jadeed};

/// Every test returns this so that a fixture failure propagates with `?`.
type NatijatIkhtibar = Result<(), Box<dyn Error>>;

/// The rich session: ninety-five distinct strings, read cleanly.
const KAMILA: &str = "kamila.jsonl";

/// The same pass as [`SALIMA`], written by a build whose footer was one closing
/// brace short — so its final line does not parse.
const MAQTUA: &str = "maqtua.jsonl";

/// That pass again, two minutes later, written whole.
const SALIMA: &str = "salima.jsonl";

fn jalsa(ism: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("jalsat")
        .join(ism)
}

fn iqra(ism: &str) -> Result<JalsaMuhammala, Box<dyn Error>> {
    Ok(iqra_jalsa(&jalsa(ism), HADD_JALSA)?)
}

/// The static half, as the game's own localization bundles gave it.
fn sakina() -> Result<JadwalNusus, Box<dyn Error>> {
    let khaam: Vec<MudkhalMustakhraj> = serde_json::from_str(include_str!("jalsat/sakina.json"))?;
    let mut jadwal = JadwalNusus::jadeed();
    for madkhal in khaam {
        jadwal.adif(madkhal);
    }
    Ok(jadwal)
}

/// Every distinct string one session recovered.
fn nusus(muhammala: &JalsaMuhammala) -> Vec<String> {
    let mut asma: Vec<String> = muhammala
        .mulahazat()
        .map(|mulahaza| mulahaza.nass.clone())
        .collect();
    asma.sort();
    asma
}

/// The rich session reads whole, and reads as what it is: a capture that was
/// never closed.
///
/// The footer is optional and the count proves it — ninety-five strings came
/// back out of a file with no `khitam` line. What it is not is silent:
/// `iktamalat` is false and the diagnostics line says the session ended without
/// one, because a player who alt-F4s out of a game leaves exactly this, and a
/// reader that called it complete would be asserting the capture covered
/// everything the player reached.
#[test]
fn jalsa_bila_khitam_tuqra_kamila_wa_taqul_annaha_bila_khitam() -> NatijatIkhtibar {
    let muhammala = iqra(KAMILA)?;

    assert_eq!(muhammala.adad(), 95, "the session's own distinct strings");
    assert_eq!(muhammala.sutur_talifa, 0, "no line in it is damaged");
    assert!(!muhammala.mabtura, "nothing was cut off");
    assert!(
        !muhammala.iktamalat(),
        "this session carries no footer, and the reader must say so"
    );
    assert!(
        muhammala
            .taqreer()
            .iter()
            .any(|satr| satr.contains("ended without a footer")),
        "the diagnostics line names the missing footer: {:?}",
        muhammala.taqreer()
    );
    Ok(())
}

/// A session whose last line is cut off costs that line and nothing else.
///
/// `maqtua.jsonl` and `salima.jsonl` are the same pass over the same game,
/// written two minutes apart by two builds; the first one's footer is one
/// closing brace short. Comparing the two is the whole assertion: the damaged
/// file must yield the *same thirty-eight strings*, because the alternative —
/// refusing the file, or stopping at the bad line — throws away a play session
/// somebody cannot cheaply repeat.
///
/// It is also reported as a truncated tail rather than as corruption.
/// `sutur_talifa` stays at zero and `mabtura` is set, which is the distinction
/// between "the process died with bytes in flight", which is ordinary, and "a
/// line in the middle of this file is wrong", which is not.
#[test]
fn jalsa_maqtuat_alnihaya_la_taklif_illa_satraha() -> NatijatIkhtibar {
    let maqtua = iqra(MAQTUA)?;
    let salima = iqra(SALIMA)?;

    assert_eq!(
        nusus(&maqtua),
        nusus(&salima),
        "a cut-off final line must cost nothing above it"
    );
    assert_eq!(maqtua.adad(), 38);
    assert!(maqtua.mabtura, "the tail was cut off");
    assert_eq!(
        maqtua.sutur_talifa, 0,
        "a truncated tail is not counted as a damaged line"
    );
    assert!(
        !maqtua.iktamalat(),
        "the footer is the line that was lost, so the session did not close"
    );
    assert!(
        salima.iktamalat(),
        "the re-run of the same pass closed cleanly"
    );
    assert!(
        maqtua.taqreer().iter().any(|satr| satr.contains("cut off")),
        "the diagnostics line names the truncation: {:?}",
        maqtua.taqreer()
    );
    Ok(())
}

/// The merge itself: the table grows by exactly the strings capture saw and the
/// files did not, and by nothing else.
///
/// The three numbers under test are the ones that made this loop look like it
/// was working when it was not. A merge that matched nothing would double every
/// menu label; a merge that kept nothing would leave the HUD unreachable; a
/// merge that dropped the static rows nobody played past would punish a short
/// capture. So all three are asserted: the matches fold into the rows that were
/// already there, the capture-only strings arrive as new rows, and the static
/// rows the session never reached are left exactly as they were.
#[test]
fn damm_aljalsa_yuzid_almultaqat_faqat() -> NatijatIkhtibar {
    let mut jadwal = sakina()?;
    let qabl = jadwal.adad();
    let muhammala = iqra(KAMILA)?;
    let mulahazat: Vec<_> = muhammala.mulahazat().cloned().collect();

    let taqreer = dammij_iltiqat(&mut jadwal, &mulahazat, &KhiyaratDammij::jadeeda("kamila"));
    let baad = jadwal.adad();

    assert_eq!(taqreer.mulahazat, 95, "every observation was offered");
    assert!(
        taqreer.majmu_mutabaqa() > 0,
        "the menu labels the files already held must match rather than duplicate"
    );
    assert!(
        taqreer.multaqat_faqat > 0,
        "the HUD strings no container holds must be kept on their own"
    );
    assert_eq!(
        taqreer.majmu_mutabaqa() + taqreer.multaqat_faqat,
        95,
        "every observation either matched or was kept; none was silently dropped"
    );
    assert_eq!(
        baad,
        qabl + taqreer.multaqat_faqat,
        "the table grows by the capture-only strings and by nothing else"
    );
    assert_eq!(
        taqreer.muhmala, 0,
        "nothing is discarded while `yahfaz_multaqat_faqat` is on"
    );

    // Absence of evidence: the static rows nobody played past are untouched, not
    // demoted. A forty-minute session never opens every screen, and a merge that
    // reported those as absent would be reporting the length of the play.
    assert!(
        !taqreer.sakina_lam_tura.is_empty(),
        "a session this short cannot have reached every static row"
    );
    for huwiya in &taqreer.sakina_lam_tura {
        let madkhal = jadwal
            .madkhal(*huwiya)
            .ok_or("a static row the session never saw is still in the table")?;
        assert_eq!(
            madkhal.masdar,
            MasdarIstikhraj::Sakin,
            "a row capture never reached keeps its static provenance"
        );
    }

    // And the other direction: a row both halves saw carries both.
    let kilahuma = jadwal
        .madakhil()
        .filter(|madkhal| madkhal.masdar == MasdarIstikhraj::Kilahuma)
        .count();
    assert!(
        kilahuma > 0,
        "a string the files held and the screen showed is recorded as both"
    );
    Ok(())
}

/// The stage the run actually calls, on an engine whose text no file reader
/// opens: it refuses without a session and produces a table with one.
///
/// This is the pair the whole capture route rests on. Without the session the
/// stage must raise [`KhataTilqai::YahtajIltiqat`] — *play once with capture on*
/// — and not "this game has no text", because the two send a person to opposite
/// places. With the session it must come back with the strings and with
/// `multaqat` set, which is what the journal records and what a resumed run
/// reads back.
#[test]
fn marhalat_alistikhraj_tarfud_bila_jalsa_wa_tanjah_maaha() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let mukhbir = MukhbirTaqaddum::Samit;
    let muraqib = Muraqib::jadeed(&mukhbir, None);

    let bila = taarib_tilqai::istikhraj::ijri(masrah.path(), AilatMuharrik::Renpy, None, &muraqib);
    let Err(KhataTilqai::YahtajIltiqat { .. }) = bila else {
        return Err(
            "a game this build reads no container format for must ask for a capture".into(),
        );
    };

    let (makhzun, ihsa) = taarib_tilqai::istikhraj::ijri(
        masrah.path(),
        AilatMuharrik::Renpy,
        Some(&jalsa(KAMILA)),
        &muraqib,
    )?;
    assert!(ihsa.multaqat, "the run records that a capture contributed");
    assert_eq!(ihsa.adad, 95, "the stage reports the strings it recovered");
    assert_eq!(makhzun.jadwal.adad(), 95);
    assert!(
        makhzun.multaqat,
        "the stored table remembers the capture, so a resume does not lose it"
    );
    // Capture is direct evidence that a player saw the string, which outranks
    // anything a classifier concluded from a field name. A run whose visible
    // count came back zero here would compile a patch of nothing.
    assert_eq!(
        ihsa.adad_zahir, 95,
        "every captured string was on screen by definition"
    );
    Ok(())
}

/// The last link: the project rows gain what the table gained, and keep every
/// translation they already carried.
///
/// This is where a resumed run used to lose the merge. It read the rows it wrote
/// on the first pass, ignored the table the capture had just grown, translated
/// nothing new and shipped the patch it already had — reporting the merge the
/// whole way. So the assertion is both halves at once: the count grows by
/// exactly the new identities, and the row that was already translated comes
/// back with its Arabic still on it.
#[test]
fn damm_jadeed_yuzid_alsufuf_wayahfaz_altarjamat() -> NatijatIkhtibar {
    let mut jadwal = sakina()?;
    let mut sabiqa: Vec<MudkhalNass> = jadwal.ila_mudkhalat();
    let mutarjam = sabiqa.first_mut().ok_or("the static slice is not empty")?;
    let huwiya_mutarjama = mutarjam.id;
    mutarjam.hadaf = Some("الإعدادات".to_owned());

    let muhammala = iqra(KAMILA)?;
    let mulahazat: Vec<_> = muhammala.mulahazat().cloned().collect();
    let taqreer = dammij_iltiqat(&mut jadwal, &mulahazat, &KhiyaratDammij::jadeeda("kamila"));

    let qabl = sabiqa.len();
    let (madakhil, jadeeda) = damm_jadeed(sabiqa, jadwal.ila_mudkhalat());

    assert_eq!(
        jadeeda, taqreer.multaqat_faqat,
        "the project gains exactly the rows the merge added"
    );
    assert_eq!(madakhil.len(), qabl + jadeeda);
    let baqiya = madakhil
        .iter()
        .find(|mudkhal| mudkhal.id == huwiya_mutarjama)
        .ok_or("the translated row survives the top-up")?;
    assert_eq!(
        baqiya.hadaf.as_deref(),
        Some("الإعدادات"),
        "a row that exists in both keeps the project's copy, translation included"
    );
    Ok(())
}

/// A second pass keeps improving coverage, and a third adds nothing new.
///
/// The screen tells a person that playing further reaches more, so that has to
/// be true of the machinery and not only of the sentence. Merging the short
/// session first and the rich one after it must leave the table with what both
/// saw; merging the same session twice must leave it unchanged, because a pass
/// that re-added its own rows would inflate the coverage number every time
/// somebody pressed the button.
#[test]
fn jalsa_thaniya_tuzid_wa_thalitha_mukarrara_la_tuzid() -> NatijatIkhtibar {
    let mut jadwal = sakina()?;
    let qabl = jadwal.adad();

    let qasira: Vec<_> = iqra(SALIMA)?.mulahazat().cloned().collect();
    let awwal = dammij_iltiqat(&mut jadwal, &qasira, &KhiyaratDammij::jadeeda("salima"));
    let baad_awwal = jadwal.adad();
    assert_eq!(baad_awwal, qabl + awwal.multaqat_faqat);

    let kamila: Vec<_> = iqra(KAMILA)?.mulahazat().cloned().collect();
    let thani = dammij_iltiqat(&mut jadwal, &kamila, &KhiyaratDammij::jadeeda("kamila"));
    let baad_thani = jadwal.adad();
    assert!(
        baad_thani > baad_awwal,
        "the longer session reaches screens the shorter one did not"
    );
    assert_eq!(baad_thani, baad_awwal + thani.multaqat_faqat);

    let thalith = dammij_iltiqat(&mut jadwal, &kamila, &KhiyaratDammij::jadeeda("kamila"));
    assert_eq!(
        jadwal.adad(),
        baad_thani,
        "merging a session that has already been merged adds nothing"
    );
    assert_eq!(
        thalith.multaqat_faqat, 0,
        "every string in it now matches a row that is already there"
    );
    Ok(())
}

/// A resumed run notices a pass played since the last one, and does not redo a
/// pass it already merged.
///
/// This is the link that was missing, and it failed in the worst possible
/// direction: the pipeline skips extraction when its journal says the stage
/// closed and the table is on disk, so a resume handed a recording opened
/// neither. It reported the merge the first run had made, translated nothing
/// new, compiled the package it already had, installed it, and reported success
/// — the whole capture route failing silently, on the one path anybody would
/// ever use it from.
///
/// The rule is the fingerprint, and the three cases below are all of it: a
/// journal from a build that predates the field, a journal that recorded a
/// different pass, and a journal that recorded this one.
#[test]
fn istinaf_yaarif_aljalsa_aljadida_wa_la_yuid_alqadima() -> NatijatIkhtibar {
    let kamila = basmat_jalsa(&jalsa(KAMILA))?;
    let salima = basmat_jalsa(&jalsa(SALIMA))?;
    assert_eq!(
        kamila,
        basmat_jalsa(&jalsa(KAMILA))?,
        "the fingerprint of one file does not move between reads"
    );
    assert_ne!(
        kamila, salima,
        "two different passes must not answer with one fingerprint"
    );

    let masrah = tempfile::tempdir()?;
    let mut sijill = SijillMashwar::iftah(masrah.path())?;
    sijill.ibda(TarwisatMashwar {
        isdar: ISDAR_SIJILL,
        id: MashwarId::jadeed(),
        ism_luba: "R.E.P.O.".to_owned(),
        jidhr_luba: masrah.path().to_path_buf(),
        waqt: "2026-09-18T00:00:00Z".to_owned(),
    })?;

    // A journal line from a build that predates the field. It must read back as
    // the empty fingerprint, which matches no session — so the first resume that
    // carries one re-extracts instead of quietly ignoring it.
    let qadeem = masrah.path().join("mashwar.jsonl");
    let mut nass = std::fs::read_to_string(&qadeem)?;
    nass.push_str(
        "{\"naw\":\"marhala\",\"lahza\":0,\"milli\":0,\"qayd\":{\"marhala\":\"istikhraj\",\
         \"adad\":3718,\"adad_zahir\":3717,\"maqrua\":28,\"marfuda\":16,\"multaqat\":false}}\n",
    );
    std::fs::write(&qadeem, nass)?;
    let sabiq = SijillMashwar::iftah(masrah.path())?;
    let Some(QaydMarhala::Istikhraj { basmat_jalsa, .. }) = sabiq.qayd(MarhalaTilqai::Istikhraj)
    else {
        return Err("the old journal line reads back as an extraction record".into());
    };
    assert!(
        basmat_jalsa.is_empty(),
        "a journal with no fingerprint field reads as the empty one"
    );
    assert_ne!(
        *basmat_jalsa, kamila,
        "which is what makes the next resume with a recording re-extract"
    );

    // And a journal that recorded one: the same pass matches, a later pass does
    // not.
    sijill.sajjil(
        QaydMarhala::Istikhraj {
            adad: 3751,
            adad_zahir: 3750,
            maqrua: 28,
            marfuda: 16,
            multaqat: true,
            basmat_jalsa: salima.clone(),
        },
        0,
        0,
    )?;
    let baad = SijillMashwar::iftah(masrah.path())?;
    let Some(QaydMarhala::Istikhraj { basmat_jalsa, .. }) = baad.qayd(MarhalaTilqai::Istikhraj)
    else {
        return Err("the record round-trips through the journal".into());
    };
    assert_eq!(
        *basmat_jalsa, salima,
        "the merged pass is what was recorded"
    );
    assert_ne!(
        *basmat_jalsa, kamila,
        "a pass played since is a different fingerprint, so extraction runs again"
    );
    Ok(())
}

/// A capture survives a re-run that is offered no session file.
///
/// This is the loss as it happened. The session file lives in the *game's* own
/// directory and was the only copy of the pass; extraction rebuilds its table
/// from the game's files on every run. One run folded the pass in, a later one
/// was offered nothing — the file having been overwritten, cleaned, or consumed
/// — re-extracted, and every `Multaqat` row went with it, taking the measured
/// widths and the opening-session membership that ride on provenance. The
/// person had played the game and there was nothing left to show for it.
///
/// The store beside the game's runs is what makes the second run carry what the
/// first recorded, and this asserts all three things that were lost: the
/// observations, the provenance on the rows, and the flag the publish gate
/// reads off them.
#[test]
fn iltiqat_yanju_min_tashghila_bila_jalsa() -> NatijatIkhtibar {
    let amal = tempfile::tempdir()?;

    let awwal = ajmaa_mulahazat(amal.path(), Some(&jalsa(KAMILA)))?;
    assert!(
        !awwal.is_empty(),
        "the offered pass was read into the store"
    );
    let basma_awwal = basmat_mulahazat(&awwal);

    let mut jadwal_awwal = sakina()?;
    let _ = dammij_iltiqat(
        &mut jadwal_awwal,
        &awwal,
        &KhiyaratDammij::jadeeda("iltiqat"),
    );
    let sufuf_awwal = jadwal_awwal.ila_mudkhalat();
    let multaqat_awwal: Vec<_> = sufuf_awwal
        .iter()
        .filter(|mudkhal| mudkhal.masdar_istikhraj.multaqat())
        .map(|mudkhal| mudkhal.id)
        .collect();
    assert!(!multaqat_awwal.is_empty(), "the merge marked captured rows");

    // The second run is offered nothing at all.
    let thani = ajmaa_mulahazat(amal.path(), None)?;
    assert_eq!(thani.len(), awwal.len(), "the store kept every observation");
    assert_eq!(
        basmat_mulahazat(&thani),
        basma_awwal,
        "an unchanged store fingerprints the same, so a resume still skips the stage"
    );

    let mut jadwal_thani = sakina()?;
    let _ = dammij_iltiqat(
        &mut jadwal_thani,
        &thani,
        &KhiyaratDammij::jadeeda("iltiqat"),
    );
    let sufuf_thani = jadwal_thani.ila_mudkhalat();
    let multaqat_thani: Vec<_> = sufuf_thani
        .iter()
        .filter(|mudkhal| mudkhal.masdar_istikhraj.multaqat())
        .map(|mudkhal| mudkhal.id)
        .collect();
    assert_eq!(
        multaqat_thani, multaqat_awwal,
        "every captured row, and its provenance, survived a run with no session"
    );

    let majmua = MajmuatAwwal::min_madakhil(&sufuf_thani)
        .ok_or("the opening set is read back off the surviving rows")?;
    assert!(!majmua.khaliya());
    let taqrir = ihsib_taghtiya(&sufuf_thani, None, Some(&majmua));
    assert!(
        taqrir.jalsat_awwal_masjjala,
        "the opening session stays recorded across a re-run"
    );
    Ok(())
}
