//! Stage 7 — the safety gate, the transactional install, and the reversal.
//!
//! ## Where cancellation stops being free
//!
//! Everything before this stage writes only inside the run directory, so
//! cancelling costs a deleted folder. This stage writes into the game, and the
//! honest statement is not "cancellation is impossible from here" — the install
//! records every byte it replaces and every file it adds, so it is always
//! reversible — but that reversal is a *second pass over the game's files*
//! rather than nothing at all.
//!
//! So the rule this module implements: `taarib_tathbeet::thabbit` is never
//! interrupted part-way. A cancellation arriving while it runs is honoured the
//! moment it returns, by immediately restoring from the manifest it just wrote.
//! What the user gets either way is a game byte-identical to the one they had.

use std::path::{Path, PathBuf};

use taarib_aman::{NatijatFahs, TalabFahs, fahs};
use taarib_khatm::MudaqqiqEd25519;
use taarib_mustalahat::bina::BinaId;
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tarqee::irtibat::IrtibatBina;
use taarib_tathbeet::NatijatTathbeet;
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::masar_tathbeet::{TalabTathbeet, WadaMuhtawa, thabbit};
use taarib_tathbeet::mawdi::WajhatLuba;
use taarib_tathbeet::nusus::{self, IdhnNusus, Nashir};
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, istiada_nass};
use taarib_tathbeet::tarkib::QararTabaqa;

use crate::khata::{KhataTilqai, NatijatTilqai, khata_malaf, marfuda};
use crate::talab::TalabTilqai;
use crate::taqaddum::{MarhalaTilqai, Muraqib};
use crate::taqreer::IhsaTathbeet;

/// What a package declares about itself, as far as an installer needs it.
///
/// Read back out of the sealed container rather than carried in this crate's
/// journal, because the container is the authority: a resumed run installs the
/// package that is on disk, and a journal that disagreed with it would install
/// one patch under another patch's identity.
#[derive(Debug, Clone)]
pub struct BayanMaqru {
    /// The patch's lineage.
    pub id: RuqaaId,
    /// Which revision it is.
    pub murajaa: RuqaaRevision,
    /// What builds and fonts it binds to.
    pub irtibat: IrtibatBina,
}

/// Reads the three fields an install needs out of a sealed package's manifest.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] when the package will not open, its metadata
/// section will not parse, or it names no binding — each of which means the
/// file on disk is not a package this build can install.
pub fn bayan(malaf: &MalafRuqaa) -> NatijatTilqai<BayanMaqru> {
    let ruqaa = malaf
        .ruqaa()
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    let jeyson = ruqaa
        .bayan_json()
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    Ok(BayanMaqru {
        id: haql(&jeyson, "id")?,
        murajaa: haql(&jeyson, "murajaa")?,
        irtibat: haql(&jeyson, "irtibat")?,
    })
}

/// One named field of a manifest, deserialized into what the caller expects.
fn haql<T: serde::de::DeserializeOwned>(jeyson: &serde_json::Value, ism: &str) -> NatijatTilqai<T> {
    let qeema = jeyson.get(ism).cloned().ok_or_else(|| {
        marfuda(
            MarhalaTilqai::Tathbeet,
            format!("the manifest names no `{ism}`"),
        )
    })?;
    serde_json::from_value(qeema).map_err(|khata| {
        marfuda(
            MarhalaTilqai::Tathbeet,
            format!("the manifest's `{ism}` did not read: {khata}"),
        )
    })
}

/// Installs the sealed patch into the game, through the gate and the guard.
///
/// The five sub-steps are counted, because "installing…" with no denominator is
/// exactly the indeterminate spinner this pipeline refuses: fingerprint the
/// build, pass the gate, write, verify, record.
///
/// # Errors
///
/// [`KhataTilqai::TathbeetMarfud`] when the safety gate refuses — an
/// unacknowledged product statement, an anti-cheat association, a signature
/// that does not verify under the caller's anchor, a revoked package, or an
/// unacknowledged multiplayer warning.
///
/// [`KhataTilqai::MarhalaMarfuda`] when the installer itself refuses.
///
/// [`KhataTilqai::Mulgha`] when the run was cancelled *before* the install
/// began. A cancellation arriving after that point is not returned here: the
/// install completes and [`irjaa`] reverses it, which is the difference between
/// a game that is whole and a game that is half-patched.
///
/// [`KhataTilqai::MarhalaMarfuda`] also when the capability report says the
/// safety layer refuses this game: the report is the only thing that knows that,
/// and this pipeline writes into the game's own text.
pub fn ijri(
    talab: &TalabTilqai<'_>,
    imkaniyat: &TaqreerImkaniyat,
    masar_huzma: &Path,
    jidhr_khutut: &Path,
    jidhr_nusakh: &Path,
    muraqib: &Muraqib<'_>,
) -> NatijatTilqai<IhsaTathbeet> {
    const KHUTUWAT: u64 = 5;
    talab.miqbad.tahaqquq(MarhalaTilqai::Tathbeet)?;

    // The tier decision, taken once, from the report stage 2 already produced.
    // This pipeline deploys nothing, so it has no plan — but the script-engine
    // write it *does* perform still needs to know the tier and the refusal, and
    // neither is a thing a game directory can be asked about.
    let qarar = QararTabaqa::min_taqreer(imkaniyat)
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;

    std::fs::create_dir_all(jidhr_nusakh)
        .map_err(|sabab| khata_malaf(jidhr_nusakh, "created", sabab))?;
    let malaf =
        MalafRuqaa::iftah(masar_huzma).map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    let maqru = bayan(&malaf)?;

    muraqib.ballagh(
        MarhalaTilqai::Tathbeet,
        1,
        KHUTUWAT,
        "fingerprinting the installed build through the package's own recipe",
    );
    let (basma, adad_malaffat) = maqru
        .irtibat
        .mukhattat
        .ihsab(talab.luba.jidhr)
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    let bina = BinaId {
        manassa: None,
        basma,
        adad_malaffat,
        waqt: talab.khiyarat.waqt.clone(),
    };

    muraqib.ballagh(MarhalaTilqai::Tathbeet, 2, KHUTUWAT, "the safety gate");
    let luba_id = talab.luba.huwiya();
    let idhn = match fahs(&TalabFahs {
        luba: luba_id,
        jidhr_luba: talab.luba.jidhr,
        appid: talab.aman.appid,
        jidhr_steam: talab.aman.jidhr_steam,
        malaf: &malaf,
        mirsa: talab.aman.mirsa,
        qaima: talab.aman.qaima,
        iqrar: talab.aman.iqrar,
        iqrar_shabaka: talab.aman.iqrar_shabaka,
    }) {
        NatijatFahs::Masmuh(idhn) => idhn,
        NatijatFahs::Marfud(rafd) => {
            return Err(KhataTilqai::TathbeetMarfud {
                sabab: rafd.injilizi(),
            });
        },
    };

    let tarif = TarifLuba {
        luba: luba_id,
        masdar: talab.luba.masdar.clone(),
        ism: talab.luba.ism.to_owned(),
        jidhr: talab.luba.jidhr.to_path_buf(),
        ruqaa: maqru.id,
        murajaa: maqru.murajaa,
        basma_bina: Some(basma),
    };
    let wajha = WajhatLuba::dakhil_taarib(&talab.khiyarat.wijha)
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    let tanfidhi = talab
        .luba
        .tanfidhi
        .and_then(|masar| masar.file_name())
        .map_or_else(
            || talab.luba.ism.to_owned(),
            |ism| ism.to_string_lossy().into_owned(),
        );
    // Placed as this engine's adapter reads it: the sealed bytes for every
    // engine but Unity, whose takeover carries no decompressor and reads only
    // the uncompressed working copy.
    //
    // This pipeline wrote the sealed container straight into the game, and the
    // one engine it is finished for is the one engine that cannot read it. The
    // plugin refused the whole patch at `TAARIB-E-6007` — "section 1 is
    // compressed" — so a run that extracted, translated, compiled, sealed and
    // installed without a single failure left the game entirely in English,
    // with the reason in the BepInEx log and nowhere else. The command the
    // Studio's install button calls has always done this; the one-button run
    // reached the same installer by another door and skipped it.
    let bayt_ruqaa = taarib_tathbeet::masar_tathbeet::muhtawa_ruqaa(
        imkaniyat.muharrik.aila,
        malaf.bayt().to_vec(),
        masar_huzma,
    )
    .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    let mut muhtawa = vec![WadaMuhtawa {
        wajha,
        bayt: bayt_ruqaa,
    }];
    // The faces the package was shaped against travel with it, out of the run's
    // own font directory — the copies `bina::hayyi` validated and the compiler
    // shaped against, so the fingerprints in the package match by construction.
    //
    // Skipped here as well until now, and for a Unity game it is not optional:
    // the takeover rasterises through `taarib_jisr` with the patch's own faces
    // and refuses at launch when `taarib/khutut/` is empty. This run installed
    // into a game that already had one from an earlier install, which is the
    // only reason it drew anything at all.
    muhtawa.extend(
        taarib_tathbeet::masar_tathbeet::muhtawa_khutut(
            imkaniyat.muharrik.aila,
            &maqru.irtibat.khutut,
            jidhr_khutut,
        )
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?,
    );
    let talab_tathbeet = TalabTathbeet {
        luba: &tarif,
        bina: &bina,
        irtibat: &maqru.irtibat,
        muhtawa,
        iqrar_taqribi: false,
        tanfidhi: &tanfidhi,
        // The same root the safety gate above was given, for the same reason:
        // one resolution of where Steam is, not two that can disagree.
        jidhr_steam: talab.aman.jidhr_steam,
    };

    muraqib.ballagh(
        MarhalaTilqai::Tathbeet,
        3,
        KHUTUWAT,
        "writing, through the recording guard",
    );
    let natija = thabbit(
        &talab_tathbeet,
        &idhn,
        &malaf,
        &MudaqqiqEd25519,
        jidhr_nusakh,
        format!("taarib-tilqai — automatic run {}", talab.id),
        // No framework is deployed here. Deciding what framework and adapter
        // build a tier belongs to is `taarib_tathbeet::tarkib`'s job against a
        // component store, and a component store is something the caller either
        // has or does not; inventing one would be inventing a binary to put in
        // somebody's game.
        //
        // The script-engine write is still this step's, and it is now asked for
        // explicitly rather than happening on its own beside it: an RPG Maker or
        // Ren'Py game is patched by replacing its own shipped text, which is the
        // write the tier governs. No Ren'Py face is registered, because no
        // deployment step here places one and a settings file naming a font that
        // was never deployed is a game rendered in boxes.
        |nashir: &mut Nashir<'_>| -> NatijatTathbeet<()> {
            nashir.raqqi(
                IdhnNusus::min_qarar(qarar),
                nusus::makhzan_mukawwinat().as_deref(),
            )
        },
    )
    .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;

    muraqib.ballagh(
        MarhalaTilqai::Tathbeet,
        4,
        KHUTUWAT,
        "verifying every path it wrote",
    );
    let ihsa = IhsaTathbeet {
        jidhr_nusakh: jidhr_nusakh.to_path_buf(),
        adad_muhtawa: u64::try_from(natija.adad_muhtawa).unwrap_or(u64::MAX),
        tahaqquq: natija.tahaqquq.injilizi().to_owned(),
        rujia: false,
        istiada: Vec::new(),
    };
    muraqib.ikhtim(
        MarhalaTilqai::Tathbeet,
        KHUTUWAT,
        format!(
            "{} file(s) placed; {}; backups at {}",
            ihsa.adad_muhtawa,
            ihsa.tahaqquq,
            jidhr_nusakh.display()
        ),
    );
    Ok(ihsa)
}

/// Reverses an install, restoring the game to exactly what it shipped as.
///
/// [`SiyasatIstiada::Sarima`] rather than the default: the caller asked for the
/// game back, and a policy that would leave four files as some updater had
/// rewritten them is a policy that answers a different question. A refusal here
/// is worth surfacing, which is why it is an error and not a log line.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] when the restore cannot run or cannot
/// complete, which is the one state in this pipeline that needs a human: the
/// game has Taarib's bytes in it and the manifest could not take them out.
pub fn irjaa(jidhr_luba: &Path, jidhr_nusakh: &Path) -> NatijatTilqai<Vec<String>> {
    if !Tathbeet::mawjud(jidhr_nusakh, NawTathbeet::Nass) {
        return Ok(vec![
            "no manifest was written, so there was nothing to restore".to_owned(),
        ]);
    }
    let mut radd = RadLaShay;
    let taqreer = istiada_nass(jidhr_luba, jidhr_nusakh, SiyasatIstiada::Sarima, &mut radd)
        .map_err(|khata| marfuda(MarhalaTilqai::Tathbeet, khata))?;
    Ok(taqreer.taqreer())
}

/// The backup root inside a run directory.
#[must_use]
pub fn jidhr_nusakh(jidhr_mashwar: &Path) -> PathBuf {
    jidhr_mashwar.join(crate::mashwar::MUJALLAD_NUSAKH)
}
