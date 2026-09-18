//! The stage machine: the one entry point, and what it does at each step.

use std::path::Path;
use std::time::Instant;

use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustalahat::nass::MudkhalNass;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_usus::khata::Tafsir as _;

use crate::istikhraj::JadwalMakhzun;
use crate::khata::{KhataTilqai, NatijatTilqai};
use crate::mashwar::{
    MALAF_JADWAL, MALAF_NUSUS, MUJALLAD_MASHRU, QaydMarhala, SijillMashwar, TarwisatMashwar,
};
use crate::natija::NatijatMashwar;
use crate::talab::TalabTilqai;
use crate::taqaddum::{MarhalaTilqai, Muraqib};
use crate::taqreer::{HalatMashwar, IhsaIstikhraj, IhsaTathbeet, TaqreerMarhala, TaqreerMashwar};
use crate::{bina, fahs, istikhraj, tarjama, tathbeet};

/// Runs the whole pipeline over one game.
///
/// Resuming is not a separate call: pass the same [`crate::mashwar::MashwarId`]
/// and the same runs root, and every stage the journal already closed is
/// skipped. The translation stage is entered even on a resume, because its own
/// journal is what decides which strings still need a provider — and that is
/// the only decision in this pipeline that costs money.
///
/// Cancellation is checked at every stage boundary, at every font, and inside
/// the translation stage between replies. Up to the install it is free: the run
/// directory is the only thing written. From the install it is *reversed*
/// rather than aborted — see [`crate::tathbeet`].
///
/// # Blocking
///
/// Six of the seven stages are synchronous and CPU-bound. On a multi-threaded
/// runtime they are run inside `tokio::task::block_in_place`, so the worker
/// they occupy is handed back to the scheduler; on a current-thread runtime
/// there is no such option and they block it, which is correct for a test
/// harness and is why the detection is at runtime rather than a feature.
pub async fn arrib(talab: &TalabTilqai<'_>) -> NatijatMashwar {
    let bidaya = Instant::now();
    let mujallad = talab.mujallad();
    let mut taqreer = TaqreerMashwar::jadeed();
    let mut muraqib = Muraqib::jadeed(talab.mukhbir, talab.khiyarat.saqf_takalif);

    let khata = match ijri(talab, &mujallad, &mut taqreer, &mut muraqib).await {
        Ok(()) => {
            taqreer.hala = HalatMashwar::Tammat;
            taqreer.marhala = MarhalaTilqai::Tamma;
            None
        },
        Err(khata) => {
            taqreer.hala = match &khata {
                KhataTilqai::Mulgha { marhala } => {
                    taqreer.marhala = *marhala;
                    HalatMashwar::Mulgha
                },
                KhataTilqai::YahtajIltiqat { .. } => HalatMashwar::TahtajIltiqat,
                _ => HalatMashwar::Tawaqqafat,
            };
            taqreer.sabab = Some(khata.injilizi());
            taqreer.ilaj = Some(ilaj(&khata));
            Some(khata)
        },
    };

    taqreer.milli = u64::try_from(bidaya.elapsed().as_millis()).unwrap_or(u64::MAX);
    muraqib.ballagh(
        MarhalaTilqai::Tamma,
        1,
        1,
        format!("{} in {} ms", taqreer.hala.wasf_injilizi(), taqreer.milli),
    );
    NatijatMashwar {
        id: talab.id,
        mujallad,
        taqreer,
        khata,
    }
}

/// The stage machine proper.
#[expect(
    clippy::too_many_lines,
    reason = "the seven stages are one linear sequence and splitting them would mean a struct \
              whose only purpose is to carry this function's locals between its own halves"
)]
async fn ijri(
    talab: &TalabTilqai<'_>,
    mujallad: &Path,
    taqreer: &mut TaqreerMashwar,
    muraqib: &mut Muraqib<'_>,
) -> NatijatTilqai<()> {
    let mut sijill = SijillMashwar::iftah(mujallad)?;
    taqreer.sijill_talif = sijill.talifa();
    sijill.ibda(TarwisatMashwar {
        isdar: crate::mashwar::ISDAR_SIJILL,
        id: talab.id,
        ism_luba: talab.luba.ism.to_owned(),
        jidhr_luba: talab.luba.jidhr.to_path_buf(),
        waqt: talab.khiyarat.waqt.clone(),
    })?;

    let mashru = mujallad.join(MUJALLAD_MASHRU);
    std::fs::create_dir_all(&mashru)
        .map_err(|sabab| crate::khata::khata_malaf(&mashru, "created", sabab))?;
    let masarat = bina::MasaratBina::jadeeda(mujallad);

    // --- 1 — the probe ------------------------------------------------------
    talab.miqbad.tahaqquq(MarhalaTilqai::Fahs)?;
    let mustanaf: Option<TaqreerImkaniyat> = match sijill.qayd(MarhalaTilqai::Fahs) {
        Some(QaydMarhala::Fahs { imkaniyat }) => Some((**imkaniyat).clone()),
        _ => None,
    };
    let imkaniyat: TaqreerImkaniyat = if let Some(imkaniyat) = mustanaf {
        istanif(taqreer, &sijill, MarhalaTilqai::Fahs, 1, Some(1));
        imkaniyat
    } else {
        let bidaya = Instant::now();
        let imkaniyat = hajiz(|| fahs::ijri(&talab.luba, &talab.khiyarat.waqt, muraqib))?;
        sajjil(
            &mut sijill,
            taqreer,
            talab,
            bidaya,
            QaydMarhala::Fahs {
                imkaniyat: Box::new(imkaniyat.clone()),
            },
            1,
            Some(1),
        )?;
        imkaniyat
    };
    taqreer.imkaniyat = Some(imkaniyat.clone());
    // Again, and on purpose. `fahs::ijri` already applied this gate to the
    // report it computed — but a resumed run does not call `fahs::ijri` at all,
    // it reads the report back out of a journal that an older build may have
    // written. Checking the value the run is actually about to use is what
    // makes the gate hold on the one path that skips the stage enforcing it.
    fahs::tahaqquq_jahiziya(&imkaniyat)?;

    // --- 2 — extraction -----------------------------------------------------
    taqreer.marhala = MarhalaTilqai::Istikhraj;
    talab.miqbad.tahaqquq(MarhalaTilqai::Istikhraj)?;
    let masar_jadwal = mashru.join(MALAF_JADWAL);
    // The capture session's own fingerprint, taken before the journal is
    // consulted, because it is half of the question "has extraction already
    // done the work this run is asking for". The other half is the table on
    // disk. A resume whose session is the one the journal recorded skips the
    // stage; a resume carrying a pass the person has played since does not, and
    // that is the only way a recording ever reaches a patch.
    let basmat_jalsa = talab
        .jalsat_iltiqat
        .map(istikhraj::basmat_jalsa)
        .transpose()?
        .unwrap_or_default();
    let makhzun = match sijill.qayd(MarhalaTilqai::Istikhraj) {
        Some(QaydMarhala::Istikhraj {
            maqrua,
            basmat_jalsa: sabiqa,
            ..
        }) if masar_jadwal.is_file() && *sabiqa == basmat_jalsa => {
            let makhzun = JadwalMakhzun::iqra(&masar_jadwal)?;
            istanif(
                taqreer,
                &sijill,
                MarhalaTilqai::Istikhraj,
                *maqrua,
                Some(*maqrua),
            );
            taqreer.istikhraj = Some(ihsa_min_makhzun(&makhzun));
            makhzun
        },
        _ => {
            let bidaya = Instant::now();
            let (makhzun, ihsa) = hajiz(|| {
                istikhraj::ijri(
                    talab.luba.jidhr,
                    imkaniyat.muharrik.aila,
                    talab.jalsat_iltiqat,
                    muraqib,
                )
            })?;
            makhzun.uktub(&masar_jadwal)?;
            let maqrua = ihsa.maqrua;
            taqreer.istikhraj = Some(ihsa.clone());
            sajjil(
                &mut sijill,
                taqreer,
                talab,
                bidaya,
                QaydMarhala::Istikhraj {
                    adad: ihsa.adad,
                    adad_zahir: ihsa.adad_zahir,
                    maqrua,
                    marfuda: ihsa.marfuda,
                    multaqat: ihsa.multaqat,
                    basmat_jalsa: basmat_jalsa.clone(),
                },
                maqrua,
                Some(maqrua),
            )?;
            makhzun
        },
    };

    // The project rows. Built from the table on the first pass and read back on
    // every later one, because a resumed run's translations live on them and
    // rebuilding from the table would throw those away.
    //
    // Read back *and topped up*: a table can grow between runs, and the way it
    // grows is the one that matters most. A capture session merges into the
    // extraction above — on a Unity release build, which ships no type tree,
    // that merge is the only way the text living in components ever arrives —
    // and a resume that read the old rows and ignored the new table discarded
    // exactly those strings. The run then reported the merge, translated
    // nothing, and shipped the patch it already had.
    let masar_nusus = mashru.join(MALAF_NUSUS);
    let madakhil: Vec<MudkhalNass> = if masar_nusus.is_file() {
        let sabiqa = tarjama::iqra_nusus(&masar_nusus)?;
        let (madakhil, jadeeda) = damm_jadeed(sabiqa, makhzun.jadwal.ila_mudkhalat());
        if jadeeda > 0 {
            tarjama::uktub_nusus(&masar_nusus, &madakhil)?;
            muraqib.ballagh_bila_majmu(
                MarhalaTilqai::Istikhraj,
                tul(madakhil.len()),
                format!("{jadeeda} string(s) the project did not have yet were added to it"),
            );
        }
        madakhil
    } else {
        // A first pass starts from the workshop's rows when the game has any,
        // and only then fills in from the table. Without that, a user who
        // corrected a hundred lines in the workshop and pressed translate again
        // would get a patch built from a fresh machine pass, their corrections
        // sitting in a project the run never read — and the only way to notice
        // would be to read the game. Rows the workshop already holds also come
        // with their translations, so the provider is never asked for them
        // twice.
        let asas = crate::warsha::masar(talab)
            .map(|jidhr| crate::warsha::iqra(&jidhr))
            .transpose()?
            .unwrap_or_default();
        let min_warsha = asas.len();
        let (madakhil, _) = damm_jadeed(asas, makhzun.jadwal.ila_mudkhalat());
        if min_warsha > 0 {
            muraqib.ballagh_bila_majmu(
                MarhalaTilqai::Istikhraj,
                tul(madakhil.len()),
                format!("{min_warsha} string(s) came from the workshop's existing translation"),
            );
        }
        tarjama::uktub_nusus(&masar_nusus, &madakhil)?;
        madakhil
    };

    // --- 3 — translation ----------------------------------------------------
    taqreer.marhala = MarhalaTilqai::Tarjama;
    // Entered on every run, resumed or not: `taarib_tarjama`'s own journal is
    // what decides which strings still need a provider, and skipping the stage
    // because this crate's journal already has a line would strand a run that
    // was cancelled half way through its list.
    let bidaya = Instant::now();
    let hasila = tarjama::ijri(
        talab.muzawwid,
        madakhil,
        &mashru,
        &talab.khiyarat,
        talab.miqbad,
        muraqib,
    )
    .await?;
    // Recorded before the stage's own failure is raised. A run stopped at its
    // ceiling or cancelled mid-list has spent money and translated strings, and
    // a report that dropped those numbers on the way out would be telling the
    // user nothing about the one stage that costs anything.
    taqreer.tarjama = Some(hasila.ihsa.clone());
    sajjil(
        &mut sijill,
        taqreer,
        talab,
        bidaya,
        QaydMarhala::Tarjama {
            mutabbaqa: hasila.ihsa.mutabbaqa,
            fashila: hasila.ihsa.fashila,
            munfaq: hasila.ihsa.munfaq,
            tawaqquf: hasila
                .ihsa
                .tawaqquf
                .clone()
                .or_else(|| hasila.khata.as_ref().map(ToString::to_string)),
        },
        hasila.ihsa.ujiba,
        Some(hasila.ihsa.muahhala),
    )?;
    if let Some(khata) = hasila.khata {
        return Err(khata);
    }
    let madakhil = hasila.madakhil;

    // The rows are final here, and this is where the workshop gets them. Not
    // after the install: a run that is stopped at the safety gate, or that was
    // asked not to install, has still produced a translation somebody can open
    // and correct, and putting the publish behind the last stage would throw
    // exactly those away.
    if let Some(jidhr_warsha) = crate::warsha::masar(talab) {
        let nashr = crate::warsha::anshir(
            &jidhr_warsha,
            talab.luba.huwiya(),
            talab.luba.ism,
            crate::warsha::bayan(
                Some(&imkaniyat),
                makhzun.rafd.clone(),
                &talab.wasf.isdar_taarib,
                &talab.khiyarat.waqt,
            ),
            &madakhil,
            &talab.khiyarat.waqt,
        )?;
        muraqib.ballagh_bila_majmu(
            MarhalaTilqai::Tarjama,
            tul(madakhil.len()),
            if nashr.munsha {
                format!(
                    "the workshop now has a project for this game, with {} string(s)",
                    nashr.majmu
                )
            } else {
                format!(
                    "the workshop's project for this game gained {} string(s) and now has {}",
                    nashr.jadeeda, nashr.majmu
                )
            },
        );
    }

    // --- 4 — fonts and sizes ------------------------------------------------
    taqreer.marhala = MarhalaTilqai::Takhtit;
    // Always recomputed. It is font validation and arithmetic over the table,
    // it is deterministic, and its output is borrowed by the compiler in the
    // same call — so a journal line would save nothing and could disagree with
    // the fonts the caller passed this time.
    let bidaya = Instant::now();
    let tahdeer = hajiz(|| bina::hayyi(talab, &madakhil, &masarat.khutut, muraqib))?;
    sajjil(
        &mut sijill,
        taqreer,
        talab,
        bidaya,
        QaydMarhala::Takhtit {
            khutut: tahdeer
                .khutut
                .iter()
                .map(|khatt| khatt.ism().to_owned())
                .collect(),
            ahjam_rubi: tahdeer
                .maqasat
                .ittihad()
                .iter()
                .map(|hajm| u32::from(hajm.rubi()))
                .collect(),
            azwaj: tahdeer.azwaj,
        },
        u64::try_from(tahdeer.khutut.len()).unwrap_or(u64::MAX),
        Some(u64::try_from(tahdeer.khutut.len()).unwrap_or(u64::MAX)),
    )?;

    // --- 5 and 6 — compile and seal -----------------------------------------
    taqreer.marhala = MarhalaTilqai::Tarqee;
    // A sealed container beside the journal is reused only when it was compiled
    // from exactly these translations.
    //
    // It used to be reused whenever it existed and opened, and that quietly
    // capped every resumed run at whatever the first one had translated: the
    // free provider stops when the endpoint refuses the machine, the resume
    // translates the rest, and the compile it needs was skipped because a
    // package was already there. The run then installed the first pass's patch
    // and reported success, so the only visible symptom was a game still in
    // English — the failure this whole stage exists to prevent.
    let basmat_nusus = bina::basmat_tarjamat(&madakhil);
    let sealed = sijill.tamma(MarhalaTilqai::Khatm)
        && masarat.huzma.is_file()
        && matches!(
            sijill.qayd(MarhalaTilqai::Tarqee),
            Some(QaydMarhala::Tarqee { basmat_nusus: sabiq, .. }) if *sabiq == basmat_nusus
        );
    if sealed && MalafRuqaa::iftah(&masarat.huzma).is_ok() {
        istanif(
            taqreer,
            &sijill,
            MarhalaTilqai::Tarqee,
            tahdeer.azwaj,
            Some(tahdeer.azwaj),
        );
        istanif(taqreer, &sijill, MarhalaTilqai::Khatm, 1, Some(1));
        taqreer.huzma = ihsa_min_sijill(&sijill, &masarat.huzma);
    } else {
        let (ihsa, milli_bina) = hajiz(|| {
            bina::ijmi(
                talab,
                &madakhil,
                &makhzun.rafd,
                &imkaniyat,
                &tahdeer,
                &masarat.huzma,
                muraqib,
            )
        })?;
        sajjil_bi_milli(
            &mut sijill,
            taqreer,
            talab,
            milli_bina.tarqee,
            QaydMarhala::Tarqee {
                hajm: ihsa.hajm,
                nusus: ihsa.nusus,
                takhtitat: ihsa.takhtitat,
                safahat: ihsa.safahat,
                basmat_nusus,
            },
            tahdeer.azwaj,
            Some(tahdeer.azwaj),
        )?;
        sajjil_bi_milli(
            &mut sijill,
            taqreer,
            talab,
            milli_bina.khatm,
            QaydMarhala::Khatm {
                miftah: ihsa.miftah.clone(),
                basma: ihsa.basma.clone(),
            },
            1,
            Some(1),
        )?;
        taqreer.huzma = Some(ihsa);
    }

    // --- 7 — the install ----------------------------------------------------
    taqreer.marhala = MarhalaTilqai::Tathbeet;
    if !talab.khiyarat.yathbut {
        taqreer.marhala = MarhalaTilqai::Khatm;
        return Ok(());
    }
    let nusakh = tathbeet::jidhr_nusakh(mujallad);
    if let Some(QaydMarhala::Tathbeet { adad_muhtawa, .. }) = sijill.qayd(MarhalaTilqai::Tathbeet) {
        istanif(
            taqreer,
            &sijill,
            MarhalaTilqai::Tathbeet,
            *adad_muhtawa,
            Some(*adad_muhtawa),
        );
        taqreer.tathbeet = Some(IhsaTathbeet {
            jidhr_nusakh: nusakh,
            adad_muhtawa: *adad_muhtawa,
            tahaqquq: "already installed by an earlier run of this same run".to_owned(),
            rujia: false,
            istiada: Vec::new(),
        });
        taqreer.marhala = MarhalaTilqai::Tathbeet;
        return Ok(());
    }

    let bidaya = Instant::now();
    let mut ihsa = hajiz(|| {
        tathbeet::ijri(
            talab,
            &imkaniyat,
            &masarat.huzma,
            &masarat.khutut,
            &nusakh,
            muraqib,
        )
    })?;
    sajjil(
        &mut sijill,
        taqreer,
        talab,
        bidaya,
        QaydMarhala::Tathbeet {
            jidhr_nusakh: crate::mashwar::MUJALLAD_NUSAKH.to_owned(),
            adad_muhtawa: ihsa.adad_muhtawa,
        },
        ihsa.adad_muhtawa,
        Some(ihsa.adad_muhtawa),
    )?;

    // The point of no return, reversed. A cancellation that arrived while
    // `thabbit` was writing is honoured here rather than mid-file: the install
    // completed, so the manifest is whole, so the restore can be exact.
    if talab.miqbad.mulgha() {
        ihsa.istiada = hajiz(|| tathbeet::irjaa(talab.luba.jidhr, &nusakh))?;
        ihsa.rujia = true;
        taqreer.tathbeet = Some(ihsa);
        sijill.insa_min(MarhalaTilqai::Tathbeet)?;
        return Err(KhataTilqai::Mulgha {
            marhala: MarhalaTilqai::Tathbeet,
        });
    }

    taqreer.tathbeet = Some(ihsa);
    taqreer.marhala = MarhalaTilqai::Tathbeet;
    Ok(())
}

/// Records a stage in the journal and in the report.
fn sajjil(
    sijill: &mut SijillMashwar,
    taqreer: &mut TaqreerMashwar,
    talab: &TalabTilqai<'_>,
    bidaya: Instant,
    qayd: QaydMarhala,
    munjaz: u64,
    majmu: Option<u64>,
) -> NatijatTilqai<()> {
    sajjil_bi_milli(sijill, taqreer, talab, milli(bidaya), qayd, munjaz, majmu)
}

/// The same, for a stage that measured its own duration.
fn sajjil_bi_milli(
    sijill: &mut SijillMashwar,
    taqreer: &mut TaqreerMashwar,
    talab: &TalabTilqai<'_>,
    milli: u64,
    qayd: QaydMarhala,
    munjaz: u64,
    majmu: Option<u64>,
) -> NatijatTilqai<()> {
    let marhala = qayd.marhala();
    sijill.sajjil(qayd, talab.khiyarat.lahza, milli)?;
    taqreer.marahil.push(TaqreerMarhala {
        marhala,
        munjaz,
        majmu,
        milli,
        mustanafa: false,
    });
    taqreer.marhala = marhala;
    Ok(())
}

/// Records a stage the journal already held.
fn istanif(
    taqreer: &mut TaqreerMashwar,
    sijill: &SijillMashwar,
    marhala: MarhalaTilqai,
    munjaz: u64,
    majmu: Option<u64>,
) {
    let milli = sijill
        .marahil()
        .iter()
        .find(|satr| satr.qayd.marhala() == marhala)
        .map_or(0, |satr| satr.milli);
    taqreer.marahil.push(TaqreerMarhala {
        marhala,
        munjaz,
        majmu,
        milli,
        mustanafa: true,
    });
    taqreer.marhala = marhala;
}

/// Adds rows the table has and the project does not, keeping every row the
/// project already carries.
///
/// Identity, not text: a row is the same row when its [`NassId`] is the same,
/// which is what the extractor computes from where a string was found. A row
/// that exists in both keeps the project's copy, translation and review state
/// included — the table's copy has neither and would overwrite them with
/// nothing.
///
/// Returns the rows and how many were added, because "the merge found
/// twenty-three strings and the project grew by twenty-three" is the only way a
/// reader can tell a merge that worked from one that was thrown away.
#[must_use]
pub fn damm_jadeed(
    sabiqa: Vec<MudkhalNass>,
    jadwal: Vec<MudkhalNass>,
) -> (Vec<MudkhalNass>, usize) {
    let mawjuda: std::collections::HashSet<_> = sabiqa.iter().map(|mudkhal| mudkhal.id).collect();
    let mut madakhil = sabiqa;
    let mut jadeeda = 0_usize;
    for mudkhal in jadwal {
        if mawjuda.contains(&mudkhal.id) {
            continue;
        }
        madakhil.push(mudkhal);
        jadeeda = jadeeda.saturating_add(1);
    }
    (madakhil, jadeeda)
}

/// A count as the progress reporter's own width.
fn tul(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

/// The package's numbers, rebuilt from the journal on a resumed run.
fn ihsa_min_sijill(sijill: &SijillMashwar, masar: &Path) -> Option<crate::taqreer::IhsaHuzma> {
    let (hajm, nusus, takhtitat, safahat) = match sijill.qayd(MarhalaTilqai::Tarqee)? {
        QaydMarhala::Tarqee {
            hajm,
            nusus,
            takhtitat,
            safahat,
            ..
        } => (*hajm, *nusus, *takhtitat, *safahat),
        _ => return None,
    };
    let (miftah, basma) = match sijill.qayd(MarhalaTilqai::Khatm)? {
        QaydMarhala::Khatm { miftah, basma } => (miftah.clone(), basma.clone()),
        _ => return None,
    };
    Some(crate::taqreer::IhsaHuzma {
        masar: masar.to_path_buf(),
        hajm,
        nusus,
        takhtitat,
        safahat,
        // Not journaled: the glyph count and the size list are the compiler's
        // own report, and inventing them on a resume would put a number in a
        // report that nothing produced.
        ashkal: 0,
        ahjam_rubi: Vec::new(),
        miftah: miftah.clone(),
        miftah_tatwir: bina::sittasi(&taarib_khatm::MIFTAH_TATWIR) == miftah,
        basma,
    })
}

/// The extraction report, rebuilt from a stored table on a resumed run.
fn ihsa_min_makhzun(makhzun: &JadwalMakhzun) -> IhsaIstikhraj {
    let tul = |qeema: usize| u64::try_from(qeema).unwrap_or(u64::MAX);
    IhsaIstikhraj {
        adad: tul(makhzun.jadwal.adad()),
        adad_zahir: tul(makhzun.jadwal.adad_zahir()),
        maqrua: tul(makhzun.rafd.maqrua.len()),
        marfuda: tul(makhzun.rafd.marfuda.len()),
        khasara: tul(makhzun.rafd.adad_khasara()),
        yanfa_iltiqat: makhzun.rafd.yanfa_iltiqat(),
        multaqat: makhzun.multaqat,
        satrat: makhzun.rafd.taqreer(),
    }
}

/// Milliseconds since an instant.
fn milli(bidaya: Instant) -> u64 {
    u64::try_from(bidaya.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// What the user can do next, in English.
fn ilaj(khata: &KhataTilqai) -> String {
    match khata {
        KhataTilqai::YahtajIltiqat { .. } => {
            "Launch the game once with capture enabled, then run this again and pass the session \
             file it wrote. Everything already done — the probe, the fonts — is kept."
                .to_owned()
        },
        KhataTilqai::LaNusus { .. } => {
            "Nothing in this game's files is readable text and capture would not change that. \
             There is nothing further this pipeline can do with it."
                .to_owned()
        },
        KhataTilqai::Mulgha { marhala } => {
            if marhala.ilgha_majaniya() {
                "The game was never written to. Running this again resumes from where it stopped, \
                 and no string is translated — or paid for — twice."
                    .to_owned()
            } else {
                "The install was completed and then reversed, so the game is exactly as it was. \
                 Running this again reinstalls without recompiling."
                    .to_owned()
            }
        },
        KhataTilqai::LaKhatt { .. } => {
            "Choose an Arabic font in Settings and run this again.".to_owned()
        },
        KhataTilqai::TathbeetMarfud { .. } => {
            "The patch was built and is in the run directory; only the install was refused."
                .to_owned()
        },
        KhataTilqai::TabaqaGhayrMadauma { .. } => {
            "This game cannot be patched automatically. Nothing was read and nothing was spent."
                .to_owned()
        },
        KhataTilqai::MuharrikGhayrJahiz { .. } => {
            "Nothing was read and nothing was spent. This is a gap in Taarib rather than in the \
             game: when the update that finishes this engine's in-game half arrives, run this \
             again and it will go through."
                .to_owned()
        },
        _ => "Run this again: every stage that finished is kept and will be skipped.".to_owned(),
    }
}

/// Runs a blocking stage without holding a runtime worker hostage.
///
/// `block_in_place` is the right tool and it panics on a current-thread
/// runtime, so the flavour is asked for rather than assumed. A harness on a
/// current-thread runtime gets a direct call, which blocks the only thread
/// there is — which is what it wanted anyway.
fn hajiz<T>(amal: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current().map(|maqbad| maqbad.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(amal),
        _ => amal(),
    }
}
