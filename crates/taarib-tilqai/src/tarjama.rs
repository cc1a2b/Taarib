//! Stage 3 — machine translation, through the placeholder guard.
//!
//! The expensive stage, in time and in money, and therefore the one the resume
//! contract exists for. Almost nothing here is this crate's own work:
//! `taarib_tarjama::dufaat::shaghghil_jawla` owns the batching, the rate
//! limiting, the retry, the cost ledger and the journal, and
//! `taarib_tarjama::hima` owns the guard that refuses a reply which dropped,
//! invented or duplicated a placeholder. What this module adds is three things
//! that crate deliberately does not do: it turns the run's watch channel into
//! this crate's progress reports, it makes the whole stage abandonable in
//! milliseconds, and it folds the journal back into the table on **every** exit
//! path — including a cancellation — so that money already spent is money never
//! spent again.

use std::fs;
use std::path::Path;

use taarib_mustalahat::nass::{MudkhalNass, TasnifNass};
use taarib_tarjama::alamat::AtabatAlamat;
use taarib_tarjama::dufaat::{
    HasilatNass, KhiyaratJawla, SijillJawla, TaqaddumJawla, shaghghil_jawla, tabbiq_sijill,
};
use taarib_tarjama::muzawwidun::Muzawwid;

use crate::ilgha::MiqbadIlgha;
use crate::khata::{KhataTilqai, NatijatTilqai, khata_malaf};
use crate::talab::KhiyaratTilqai;
use crate::taqaddum::{MarhalaTilqai, Muraqib};
use crate::taqreer::IhsaTarjama;

/// Largest run table accepted, in bytes.
///
/// A finished row — its source, its Arabic, the lines around it and the
/// provenance of the translation — serializes to about a kilobyte, measured on
/// the tables this pipeline writes. Two hundred and fifty-six mebibytes is
/// therefore a quarter of a million strings: several times the script of the
/// most dialogue-heavy game anyone would put through a paid run, and far past
/// anything this pipeline has produced. It is judged from the directory entry
/// before a byte is taken, because what it bounds is the read itself — the
/// file's bytes, and then the rows parsed out of them — and a file past it is
/// not a table a run of this build wrote.
pub const HADD_HAJM_NUSUS: u64 = 256 * 1024 * 1024;

/// What one translation stage produced.
///
/// Returned whole even when the stage stopped, for the same reason
/// `taarib_tarjama::dufaat::TaqreerJawla` is not a `Result`: a stopped run is
/// not a void run, and the numbers — what was spent, whether the ceiling was
/// reached, which strings the guard refused — are exactly what a user needs
/// when it stops. [`HasilatMarhala::khata`] carries the failure for the caller
/// to raise after it has recorded them.
#[derive(Debug)]
pub struct HasilatMarhala {
    /// The table, with every journaled translation folded into it.
    pub madakhil: Vec<MudkhalNass>,
    /// What the stage did, and what it cost.
    pub ihsa: IhsaTarjama,
    /// What stopped it, when something did.
    pub khata: Option<KhataTilqai>,
}

/// Reads the project rows back, refusing a table over the byte cap.
///
/// The size comes from the directory entry and is judged before the read, not
/// after it: a run that died mid-write, or anything else able to write into the
/// run directory, otherwise decides how much memory the next open allocates —
/// and the workshop's recovery meets that same file on every open.
///
/// # Errors
///
/// [`KhataTilqai::NususKabira`] when the file passes [`HADD_HAJM_NUSUS`], and
/// [`KhataTilqai::KhataMalaf`] when it cannot be examined or read, or does not
/// parse.
pub fn iqra_nusus(masar: &Path) -> NatijatTilqai<Vec<MudkhalNass>> {
    let bayan = fs::metadata(masar).map_err(|sabab| khata_malaf(masar, "examined", sabab))?;
    let hajm = bayan.len();
    if hajm > HADD_HAJM_NUSUS {
        return Err(KhataTilqai::NususKabira {
            masar: masar.to_path_buf(),
            hajm,
            hadd: HADD_HAJM_NUSUS,
        });
    }
    let bayt = fs::read(masar).map_err(|sabab| khata_malaf(masar, "read", sabab))?;
    serde_json::from_slice(&bayt)
        .map_err(|sabab| khata_malaf(masar, "parsed", std::io::Error::other(sabab)))
}

/// Writes the project rows, atomically.
///
/// # Errors
///
/// [`KhataTilqai::KhataMalaf`] when they cannot be serialized or written.
pub fn uktub_nusus(masar: &Path, madakhil: &[MudkhalNass]) -> NatijatTilqai<()> {
    let bayt = serde_json::to_vec(madakhil)
        .map_err(|sabab| khata_malaf(masar, "serialized", std::io::Error::other(sabab)))?;
    taarib_usus::masarat::kitaba_dharra(masar, &bayt)
        .map_err(|sabab| khata_malaf(masar, "written", std::io::Error::other(sabab.to_string())))
}

/// Translates every eligible string, and folds the result back into the table.
///
/// The denominator is exact and known before a request goes out: the strings
/// this run is eligible to translate, which is the table minus the ones a
/// previous run already answered, minus the ones a human froze, minus the ones
/// already carrying a translation, minus the empty ones, minus the ones the
/// classifier named an identifier rather than a sentence. Every one of those
/// exclusions is counted separately and reported, because "it only did half my
/// game" needs an answer and one number labelled *skipped* is not one.
///
/// # Errors
///
/// [`KhataTilqai::KhataMalaf`] when the journal or the table cannot be read or
/// written — which is fatal here rather than survivable, because a run that
/// cannot see what it already paid for cannot honour a cumulative ceiling.
///
/// Only the failures that stop the stage before it can report anything are
/// returned as errors. A cancellation and an exhausted ceiling are carried in
/// [`HasilatMarhala::khata`] instead, so the caller records the numbers first
/// and raises the failure second.
pub async fn ijri(
    muzawwid: &dyn Muzawwid,
    mut madakhil: Vec<MudkhalNass>,
    mujallad_mashru: &Path,
    khiyarat: &KhiyaratTilqai,
    miqbad: &MiqbadIlgha,
    muraqib: &mut Muraqib<'_>,
) -> NatijatTilqai<HasilatMarhala> {
    miqbad.tahaqquq(MarhalaTilqai::Tarjama)?;

    // Opened here as well as inside the runner, and for a different reason:
    // the eligible count is this stage's denominator and it cannot be known
    // without knowing what a previous run already answered.
    let sabiq = SijillJawla::iftah(mujallad_mashru)
        .map_err(|khata| crate::khata::marfuda(MarhalaTilqai::Tarjama, khata))?;
    let muahhala = tul(madakhil
        .iter()
        .filter(|mudkhal| muahhal(mudkhal, &sabiq))
        .count());
    let munfaq_sabiq = sabiq.munfaq_sabiq();
    drop(sabiq);

    muraqib.sajjil_infaq(munfaq_sabiq);
    muraqib.ballagh(
        MarhalaTilqai::Tarjama,
        0,
        muahhala,
        format!(
            "{muahhala} string(s) to translate through {} / {}",
            muzawwid.ism(),
            muzawwid.namudhaj()
        ),
    );

    let khiyarat_jawla = KhiyaratJawla {
        saqf_takalif: khiyarat.saqf_takalif,
        aqsa_muhawalat: khiyarat.aqsa_muhawalat,
        aqsa_intizar_muadal: 2,
        tawazi: khiyarat.tawazi.max(1),
        asas_tarajua_millithania: 500,
        aqsa_tarajua_millithania: 30_000,
        lahza: khiyarat.lahza,
        waqt: khiyarat.waqt.clone(),
        atabat: AtabatAlamat::default(),
        // The one-button run is the unattended case by definition. A row the
        // classifier called internal is a key, and a provider asked about a key
        // answers with prose the game would then draw in place of it.
        yashmal_dakhili: false,
    };

    let (mursil, mut mustaqbil) = tokio::sync::watch::channel(TaqaddumJawla::default());

    // Scoped, because the run's future borrows the table and the sender for as
    // long as it exists, and the fold below needs the table back by value.
    //
    // The stage is abandoned by dropping that future, not by asking it to stop:
    // `shaghghil_jawla` has no external stop and does not need one. Every
    // result is appended to the journal the moment it exists, so the most a
    // dropped run can lose is the requests literally in flight — and those were
    // never journaled, so they were never paid for as far as the resume is
    // concerned.
    let (taqreer, mulgha) = {
        let jawla = shaghghil_jawla(
            muzawwid,
            &madakhil,
            mujallad_mashru,
            &khiyarat_jawla,
            Some(&mursil),
        );
        tokio::pin!(jawla);
        let mut taqreer = None;
        let mut mulgha = false;
        loop {
            tokio::select! {
                natija = &mut jawla => {
                    taqreer = Some(natija);
                    break;
                }
                () = miqbad.intazir() => {
                    mulgha = true;
                    break;
                }
                Ok(()) = mustaqbil.changed() => {
                    let hali = mustaqbil.borrow_and_update().clone();
                    ballagh(muraqib, &hali, muahhala);
                }
            }
        }
        (taqreer, mulgha)
    };
    drop(mursil);

    let hali = mustaqbil.borrow().clone();
    ballagh(muraqib, &hali, muahhala);

    // The fold happens on both exits. A cancelled run whose journal was never
    // folded would re-translate — and re-pay for — every string it had already
    // received, which is the one outcome this stage exists to make impossible.
    let sijill = SijillJawla::iftah(mujallad_mashru)
        .map_err(|khata| crate::khata::marfuda(MarhalaTilqai::Tarjama, khata))?;
    let tatbiq = tabbiq_sijill(&mut madakhil, &sijill, &khiyarat.waqt);
    uktub_nusus(
        &mujallad_mashru.join(crate::mashwar::MALAF_NUSUS),
        &madakhil,
    )?;

    let marfuda_bil_hima = tul(sijill
        .quyud()
        .values()
        .filter(|qayd| matches!(qayd.hasila, HasilatNass::Fashilat { himaya: true, .. }))
        .count());
    let (munfaq, saqf, tawaqquf) = taqreer.as_ref().map_or_else(
        || (hali.munfaq, khiyarat.saqf_takalif, None),
        |taqreer| {
            (
                taqreer.taqaddum.munfaq,
                taqreer.taqaddum.saqf,
                taqreer.tawaqquf.as_ref().map(ToString::to_string),
            )
        },
    );
    let balagha_alsaqf = taqreer.as_ref().is_some_and(|taqreer| {
        matches!(
            taqreer.tawaqquf,
            Some(taarib_tarjama::KhataTarjama::SaqfTakalif { .. })
        )
    });
    muraqib.sajjil_infaq(munfaq);

    let ihsa = IhsaTarjama {
        muahhala,
        ujiba: tul(hali.mutarjama.saturating_add(hali.fashila.len())),
        mutabbaqa: tul(tatbiq.mutabbaqa),
        fashila: tul(hali.fashila.len()),
        marfuda_bil_hima,
        min_sijill: tul(hali.mutakhattaha.fi_alsijill),
        mutarjama_musbaqan: tul(hali.mutakhattaha.mutarjama_musbaqan),
        munfaq,
        saqf,
        balagha_alsaqf,
        tawaqquf,
        muzawwid: muzawwid.ism().to_owned(),
        namudhaj: muzawwid.namudhaj().to_owned(),
    };

    if mulgha {
        return Ok(HasilatMarhala {
            madakhil,
            ihsa,
            khata: Some(KhataTilqai::Mulgha {
                marhala: MarhalaTilqai::Tarjama,
            }),
        });
    }

    if !madakhil.iter().any(|mudkhal| mudkhal.hadaf.is_some()) {
        let sabab = ihsa.tawaqquf.clone().unwrap_or_else(|| {
            format!(
                "{} eligible string(s), {} failed, none translated",
                ihsa.muahhala, ihsa.fashila
            )
        });
        return Ok(HasilatMarhala {
            madakhil,
            ihsa,
            khata: Some(KhataTilqai::LaTarjama { sabab }),
        });
    }

    // Reported at what was answered, not at the eligible total: a run that
    // walked its whole list ends at `muahhala` anyway, and one that stopped at
    // its ceiling must not close the stage at a hundred percent.
    muraqib.ballagh(
        MarhalaTilqai::Tarjama,
        ihsa.ujiba,
        muahhala,
        format!(
            "{} answered, {} failed ({} refused by the placeholder guard), {} nano-$ spent",
            ihsa.ujiba, ihsa.fashila, ihsa.marfuda_bil_hima, ihsa.munfaq
        ),
    );
    Ok(HasilatMarhala {
        madakhil,
        ihsa,
        khata: None,
    })
}

/// Whether a string is one this run would dispatch.
///
/// Mirrors `shaghghil_jawla`'s own eligibility test, in its order, because this
/// is the denominator the progress bar divides by and a denominator that
/// disagreed with the runner would produce a bar that stops at ninety-four
/// percent and never moves.
fn muahhal(mudkhal: &MudkhalNass, sijill: &SijillJawla) -> bool {
    !sijill.ajaba(mudkhal.id)
        && !mudkhal.muraja.mujammad()
        && mudkhal.hadaf.is_none()
        && !mudkhal.masdar.trim().is_empty()
        && mudkhal.tasnif != TasnifNass::Dakhili
}

/// One run-state snapshot, as a progress report.
fn ballagh(muraqib: &mut Muraqib<'_>, hali: &TaqaddumJawla, muahhala: u64) {
    muraqib.sajjil_infaq(hali.munfaq);
    let munjaz = tul(hali.mutarjama.saturating_add(hali.fashila.len()));
    muraqib.ballagh(
        MarhalaTilqai::Tarjama,
        munjaz,
        muahhala,
        format!(
            "{} translated, {} failed, {} skipped ({} already answered, {} already translated, \
             {} frozen, {} empty, {} identifiers)",
            hali.mutarjama,
            hali.fashila.len(),
            hali.mutakhattaha.majmu(),
            hali.mutakhattaha.fi_alsijill,
            hali.mutakhattaha.mutarjama_musbaqan,
            hali.mutakhattaha.mujammada,
            hali.mutakhattaha.farigha,
            hali.mutakhattaha.dakhiliya
        ),
    );
}

/// A count as the report's own width.
fn tul(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_usus::khata::{Ramz, Tafsir as _};

    use super::*;

    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// A table past the cap is refused, and refused without reading it.
    #[test]
    fn iqra_nusus_yarfud_ma_fawq_alhadd() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let masar = muaqqat.path().join("nusus.json");
        // Extended rather than filled: the cap is judged from the directory
        // entry before a byte is taken, so a test that proves it costs a sparse
        // file and no memory — which is the property under test.
        fs::File::create(&masar)?.set_len(HADD_HAJM_NUSUS + 1)?;

        let khata = iqra_nusus(&masar).err().ok_or("a table past the cap")?;
        let KhataTilqai::NususKabira { hajm, hadd, .. } = &khata else {
            return Err(format!("the refusal names the size, not {khata}").into());
        };
        assert_eq!(*hajm, HADD_HAJM_NUSUS + 1);
        assert_eq!(*hadd, HADD_HAJM_NUSUS);
        assert_eq!(khata.ramz(), Ramz::jadeed(crate::khata::TILQAI + 14));
        Ok(())
    }

    /// A table under the cap still round-trips, and an absent one is still a
    /// file failure rather than the size refusal.
    #[test]
    fn iqra_nusus_yamurr_bila_tagheer() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let masar = muaqqat.path().join("nusus.json");
        uktub_nusus(&masar, &[])?;
        assert!(iqra_nusus(&masar)?.is_empty());

        let ghaib = iqra_nusus(&muaqqat.path().join("la-shay.json"))
            .err()
            .ok_or("an absent table")?;
        assert!(
            matches!(ghaib, KhataTilqai::KhataMalaf { .. }),
            "an absent table is a file failure, not a table over the cap"
        );
        Ok(())
    }
}
