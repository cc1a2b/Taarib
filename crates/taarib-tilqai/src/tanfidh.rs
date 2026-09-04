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
    let makhzun = match sijill.qayd(MarhalaTilqai::Istikhraj) {
        Some(QaydMarhala::Istikhraj { maqrua, .. }) if masar_jadwal.is_file() => {
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
    let masar_nusus = mashru.join(MALAF_NUSUS);
    let madakhil: Vec<MudkhalNass> = if masar_nusus.is_file() {
        tarjama::iqra_nusus(&masar_nusus)?
    } else {
        let madakhil = makhzun.jadwal.ila_mudkhalat();
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
    let sealed = sijill.tamma(MarhalaTilqai::Khatm) && masarat.huzma.is_file();
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
    let mut ihsa = hajiz(|| tathbeet::ijri(talab, &masarat.huzma, &nusakh, muraqib))?;
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

/// The package's numbers, rebuilt from the journal on a resumed run.
fn ihsa_min_sijill(sijill: &SijillMashwar, masar: &Path) -> Option<crate::taqreer::IhsaHuzma> {
    let (hajm, nusus, takhtitat, safahat) = match sijill.qayd(MarhalaTilqai::Tarqee)? {
        QaydMarhala::Tarqee {
            hajm,
            nusus,
            takhtitat,
            safahat,
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
