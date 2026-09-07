//! Stage 1 — the engine probe, the resolution, and the capability report.
//!
//! The cheapest stage and the one that decides whether the rest is worth
//! running: a game the report refuses is a game this pipeline stops at, before
//! it has read a container or spent a cent.

use taarib_muharrik::{Mifhas, SiyaqFahs, imkaniyat, tahdid};
use taarib_mustalahat::muharrik::{AilatMuharrik, Hadd, TaqreerImkaniyat};

use crate::khata::{KhataTilqai, NatijatTilqai};
use crate::talab::LubaTilqai;
use crate::taqaddum::{MarhalaTilqai, Muraqib};

/// Probes the game and resolves what can be done to it.
///
/// The denominator here is the detector count, which is known before the probe
/// starts. It is not ticked per detector: `Mifhas::ifhas` runs a *second* pass
/// with an executable the first pass discovered, and reproducing that outside
/// the crate to interleave a callback would duplicate resolution logic that
/// would then rot. The probe is milliseconds; two reports is the honest shape.
///
/// # Errors
///
/// [`KhataTilqai::JidhrGhayrSalih`] when the game root is not a directory.
///
/// [`KhataTilqai::FahsFashil`] when the probe itself will not run.
///
/// [`KhataTilqai::TabaqaGhayrMadauma`] and
/// [`KhataTilqai::MuharrikGhayrJahiz`] when [`tahaqquq_jahiziya`] refuses the
/// report — an anti-cheat association, a launcher that forbids modification, or
/// an engine whose in-game half this build has not finished. Refused is a
/// verdict, not a bug, and it stops the run here rather than at the install
/// gate, which is the difference between a wasted minute and a wasted hour of
/// paid translation.
pub fn ijri(
    luba: &LubaTilqai<'_>,
    waqt: &str,
    muraqib: &Muraqib<'_>,
) -> NatijatTilqai<TaqreerImkaniyat> {
    if !luba.jidhr.is_dir() {
        return Err(KhataTilqai::JidhrGhayrSalih {
            masar: luba.jidhr.to_path_buf(),
        });
    }

    let adad = u64::try_from(taarib_muharrik::kul().len()).unwrap_or(u64::MAX);
    muraqib.ballagh(
        MarhalaTilqai::Fahs,
        0,
        adad,
        format!(
            "running {adad} engine detector(s) over {}",
            luba.jidhr.display()
        ),
    );

    let siyaq = SiyaqFahs {
        jidhr: luba.jidhr,
        tanfidhi: luba.tanfidhi,
        ism: luba.ism,
        nizam: luba.nizam,
        beea: luba.beea,
    };
    let jami = Mifhas::jadeed()
        .ifhas(&siyaq)
        .map_err(|khata| KhataTilqai::FahsFashil {
            sabab: khata.injilizi,
        })?;

    let muharrik = tahdid::hall(&jami);
    let taqreer = imkaniyat::taqreer(muharrik, &[], waqt.to_owned());

    tahaqquq_jahiziya(&taqreer)?;

    muraqib.ikhtim(
        MarhalaTilqai::Fahs,
        adad,
        format!(
            "{:?} at tier {} — {}",
            taqreer.muharrik.aila,
            taqreer.tabaqa.raqm(),
            taqreer.sabab_injilizi
        ),
    );
    Ok(taqreer)
}

// ---------------------------------------------------------------------------
// الجاهزية — the one gate, and the one sentence it refuses with
// ---------------------------------------------------------------------------

/// Refuses a game this pipeline cannot honestly take to the end.
///
/// Two refusals, in the order a user needs to hear them. The safety layer's
/// comes first because it is a fact about *this game* that no release changes.
/// The readiness one comes second because it is a fact about *this build* that
/// a release does change, and merging the two would tell a user to give up when
/// the honest answer is to wait.
///
/// This is the only gate, and there is deliberately no table of engines in it.
/// It reads [`TaqreerImkaniyat::jahiziya`], which `taarib_muharrik`'s
/// `imkaniyat::jahiziya` is the sole author of — so on the day an engine's
/// adapter is finished and that one arm changes, this gate opens for that engine
/// with nothing here to edit and nothing here to forget.
///
/// # Errors
///
/// [`KhataTilqai::TabaqaGhayrMadauma`] when the report refuses the game
/// outright — an anti-cheat association, a launcher that forbids modification.
///
/// [`KhataTilqai::MuharrikGhayrJahiz`] when nothing this build installs for the
/// detected engine puts readable Arabic on the screen, so a completed run would
/// hand the user a sound patch and a game they still cannot read. Deliberately
/// not "an unchanged game": on GameMaker and on Ren'Py before 7.4 the text does
/// change, into glyphs the engine has no pictures for.
pub fn tahaqquq_jahiziya(taqreer: &TaqreerImkaniyat) -> NatijatTilqai<()> {
    if taqreer.marfuda {
        // The first limit rather than the tier's reason: for a refused report
        // `imkaniyat::taqreer` puts the refusal itself there, naming what was
        // detected, and that is the sentence worth showing.
        let sabab = taqreer.hudud.first().map_or_else(
            || taqreer.sabab_injilizi.clone(),
            |hadd| hadd.injilizi.clone(),
        );
        return Err(KhataTilqai::TabaqaGhayrMadauma {
            tabaqa: taqreer.tabaqa.raqm(),
            sabab,
        });
    }

    match naqs_jahiziya(taqreer) {
        None => Ok(()),
        Some(sabab) => Err(KhataTilqai::MuharrikGhayrJahiz {
            muharrik: taqreer.muharrik.aila.ism().to_owned(),
            sabab_arabi: sabab.arabi,
            sabab_injilizi: sabab.injilizi,
        }),
    }
}

/// Why the one-button run is not offered for this game, or [`None`] when it is.
///
/// The verdict a screen draws and the refusal a start command raises are the
/// same sentence, and they are the same sentence because they are built here.
/// A caller that wants to *decide* uses [`tahaqquq_jahiziya`]; a caller that
/// wants to *say* uses this, and gets a [`Hadd`] because a bilingual pair of
/// complete sentences is exactly what that type is.
///
/// [`None`] for a game the safety layer already refused, and that is the
/// capability report's own contract rather than an omission here: a refused
/// report carries no readiness sentence, because why Taarib's adapter is
/// unfinished is beside the point for a game Taarib will not touch at all.
#[must_use]
pub fn naqs_jahiziya(taqreer: &TaqreerImkaniyat) -> Option<Hadd> {
    if taqreer.marfuda || taqreer.jahiziya.tasil() {
        return None;
    }

    // Reaching here means the run would not put readable Arabic on the screen.
    // It does **not** mean the game is left untouched, and the sentences below
    // used to say it did. Two engines disprove it: a GameMaker install rewrites
    // `data.win`'s string pool, and a Ren'Py game older than 7.4 takes the
    // per-character path — in both, the text really changes and the engine then
    // cannot draw it, so the player gets blank or broken menus rather than the
    // English they had. Promising "an unchanged game" there is the one direction
    // this sentence must never be wrong in, because it is the sentence somebody
    // reads while deciding whether it is safe to try.
    let (tafsil_arabi, tafsil_injilizi) = taqreer.naqs.as_ref().map_or_else(bila_tafsil, |naqs| {
        (naqs.arabi.clone(), naqs.injilizi.clone())
    });
    Some(Hadd {
        arabi: format!(
            "المحرّك المكتشَف: {}. الجزء الذي يضع العربية داخل ألعاب هذا المحرّك لم يكتمل في هذا \
             الإصدار من تعريب، فلو مضت الجولة إلى آخرها لخرجت برقعة سليمة لا تظهر معها عربية \
             مقروءة على الشاشة — ولهذا لا يُعرض التعريب التلقائي هنا. {tafsil_arabi}",
            taqreer.muharrik.aila.ism()
        ),
        injilizi: format!(
            "Detected engine: {}. The part of Taarib that puts Arabic inside games on this \
             engine is not finished in this build, so a run taken to the end would produce a \
             sound patch and no readable Arabic on screen — which is why the one-button run is \
             not offered here. {tafsil_injilizi}",
            ism_injilizi(taqreer.muharrik.aila)
        ),
    })
}

/// The engine's name as the English half of a refusal can use it.
///
/// `AilatMuharrik::ism` answers `Majhul` in Arabic, which is right everywhere it
/// is read as a label and wrong in the middle of an English sentence.
const fn ism_injilizi(aila: AilatMuharrik) -> &'static str {
    match aila {
        AilatMuharrik::Majhul => "none that Taarib recognises",
        _ => aila.ism(),
    }
}

/// What the refusal says when the report carries no sentence of its own.
///
/// Reachable only through a stored report written before the readiness field
/// existed — a run resuming from an old journal. Saying so, and saying what
/// refreshes it, beats inventing a gap this build cannot actually name.
fn bila_tafsil() -> (String, String) {
    (
        "ولا تفصيل أدقّ متاح لأنّ تقرير الإمكانيات المحفوظ لهذه اللعبة كُتب بإصدار أقدم من \
         تعريب؛ أعد فحص اللعبة ليُكتب من جديد."
            .to_owned(),
        "No finer detail is available, because the stored capability report for this game was \
         written by an older build of Taarib; rescan the game to have it written again."
            .to_owned(),
    )
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::muharrik::{JahiziyatTashghil, KhalfiyaBarmajiya, Muharrik};
    use taarib_usus::khata::{Ramz, Tafsir as _};
    use taarib_usus::manassa::Mimariya;

    use super::*;
    use crate::khata::TILQAI;

    /// Every engine family the probe can resolve to, so a test that walks the
    /// gate walks all of it rather than the two families somebody remembered.
    /// Every engine family, from the enum's own list rather than a copy.
    ///
    /// This was a hand-kept array and it held ten families after the set had
    /// grown to seventeen, so the exhaustiveness test below silently skipped
    /// seven engines — Bio4 and all six named in the last two phases — which is
    /// the opposite of what an exhaustiveness test is for.
    const KUL_AILAT: [AilatMuharrik; 17] = AilatMuharrik::KUL;

    /// A capability report for one engine family, built the way the probe
    /// builds one: through `imkaniyat::taqreer`, so the readiness verdict under
    /// test is the real table's answer and not a value a test invented.
    fn taqreer_li(aila: AilatMuharrik) -> TaqreerImkaniyat {
        let muharrik = Muharrik {
            aila,
            isdar: None,
            khalfiya: KhalfiyaBarmajiya::Majhula,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X8664,
            thiqa: 95,
            dalail: Vec::new(),
        };
        imkaniyat::taqreer(muharrik, &[], "2026-01-01T00:00:00Z".to_owned())
    }

    /// The same, with the readiness verdict forced — which is how the ready
    /// direction is testable at all while every arm of the real table is
    /// `ghaiba`. Forcing the field rather than the gate is deliberate: it
    /// exercises the production path end to end and will keep passing
    /// unchanged on the day an engine's real arm moves.
    fn taqreer_bi_jahiziya(aila: AilatMuharrik, jahiziya: JahiziyatTashghil) -> TaqreerImkaniyat {
        let mut taqreer = taqreer_li(aila);
        taqreer.jahiziya = jahiziya;
        if jahiziya == JahiziyatTashghil::Mukammala {
            taqreer.naqs = None;
        }
        taqreer
    }

    /// An engine with no working in-game half is refused, by name, in both
    /// languages, under this crate's own code.
    #[test]
    fn ghayr_jahiz_yurfad() {
        let taqreer = taqreer_bi_jahiziya(AilatMuharrik::Unity, JahiziyatTashghil::Ghaiba);
        let natija = tahaqquq_jahiziya(&taqreer);
        assert!(
            matches!(&natija, Err(KhataTilqai::MuharrikGhayrJahiz { .. })),
            "an engine with no in-game half was let through the gate"
        );

        // Read back through `Tafsir`, which is how the interface reads it: a
        // pass here is about the sentences a user is actually shown. The
        // fallback is unreachable — the assertion above already settled it.
        let (ramz, arabi, injilizi) = natija.err().map_or_else(
            || (Ramz::jadeed(0), String::new(), String::new()),
            |khata| (khata.ramz(), khata.arabi(), khata.injilizi()),
        );
        assert_eq!(ramz, Ramz::jadeed(TILQAI + 13));
        assert!(arabi.contains("Unity"), "{arabi}");
        assert!(injilizi.contains("Unity"), "{injilizi}");

        // The refusal has to say what is missing, not only that something is.
        // Asserted against the report's own sentence rather than against a
        // phrase copied into this file, so that rewording the table's Unity arm
        // cannot leave this passing while the user is told less.
        let naqs = taqreer
            .naqs
            .as_ref()
            .map_or_else(String::new, |naqs| naqs.injilizi.clone());
        assert!(!naqs.is_empty(), "the Unity arm carries no sentence");
        assert!(injilizi.contains(&naqs), "{injilizi}");
    }

    /// An engine whose in-game half is finished passes, and has nothing to say.
    #[test]
    fn jahiz_yamurr() {
        let taqreer = taqreer_bi_jahiziya(AilatMuharrik::Unity, JahiziyatTashghil::Mukammala);
        assert!(tahaqquq_jahiziya(&taqreer).is_ok());
        assert!(naqs_jahiziya(&taqreer).is_none());
    }

    /// A partly-finished engine passes too. `naqisa` means a player does see
    /// Arabic, and refusing there would withhold a run that works.
    #[test]
    fn naqisa_tamurr() {
        let taqreer = taqreer_bi_jahiziya(AilatMuharrik::Unity, JahiziyatTashghil::Naqisa);
        assert!(tahaqquq_jahiziya(&taqreer).is_ok());
        assert!(naqs_jahiziya(&taqreer).is_none());
    }

    /// The gate carries no engine table of its own: for every family, what it
    /// decides is exactly what `imkaniyat::jahiziya` decided. This is the test
    /// that must keep passing without an edit when an adapter is finished.
    #[test]
    fn al_bawwaba_tatba_al_jahiziya() {
        for aila in KUL_AILAT {
            let taqreer = taqreer_li(aila);
            let (jahiziya, _) = imkaniyat::jahiziya(&taqreer.muharrik);
            assert_eq!(
                jahiziya.tasil(),
                tahaqquq_jahiziya(&taqreer).is_ok(),
                "{aila:?} passes the gate iff its readiness verdict says it reaches the screen"
            );
        }
    }

    /// A game the safety layer refuses keeps that refusal. It is the more
    /// serious of the two, and unlike this one it is not undone by an update.
    #[test]
    fn al_marfuda_tahtafiz_bi_sababiha() {
        let mut taqreer = taqreer_bi_jahiziya(AilatMuharrik::Unity, JahiziyatTashghil::Ghaiba);
        taqreer.marfuda = true;
        assert!(matches!(
            tahaqquq_jahiziya(&taqreer),
            Err(KhataTilqai::TabaqaGhayrMadauma { .. })
        ));
        assert!(naqs_jahiziya(&taqreer).is_none());
    }
}
