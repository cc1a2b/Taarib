//! Publishing an automatic run's table into the workshop's project store.
//!
//! The run writes its rows into its own directory, under its own run
//! identifier, in its own format. The workshop opens one project per *game*,
//! under the game's identity, in `taarib_istikhraj`'s format. Nothing joined
//! the two, so a user who ran the one-button translation and then opened the
//! workshop was told there was no project for this game yet and to start a
//! translation — after finishing one. That is what this module closes.
//!
//! **The project's copy of a row always wins.** A game translated twice, or
//! translated and then edited, has rows in the workshop that carry a human's
//! wording, a review state and an editor's name; the run's copy carries a
//! machine's. Merging in the other direction would silently replace reviewed
//! work with a fresh machine pass, and the user would find out by reading the
//! game. So a row present in both keeps what the workshop holds, and only rows
//! the workshop has never seen are added.

use std::path::Path;

use taarib_istikhraj::mashru::{BayanIstikhraj, MALAF_MASHRU, MashruMaftuh};
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustalahat::nass::MudkhalNass;

use crate::istikhraj::JadwalMakhzun;
use crate::khata::{NatijatTilqai, marfuda};
use crate::talab::{LubaTilqai, TalabTilqai};
use crate::taqaddum::MarhalaTilqai;

/// What publishing did, so the run's progress line can say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NatijatNashr {
    /// How many rows the workshop project holds now.
    pub majmu: usize,
    /// How many of them this run added.
    pub jadeeda: usize,
    /// Whether the project did not exist before this run.
    pub munsha: bool,
}

/// Writes the run's rows into the workshop project for this game.
///
/// Creates the project when the game has none, and otherwise adds only the
/// rows it does not already carry.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] when the project store cannot be created,
/// opened, read or written. It is raised rather than swallowed because a run
/// that reports success and leaves the workshop empty is the defect this
/// module exists to remove, and hiding a write failure would reproduce it.
///
/// [`KhataTilqai::MarhalaMarfuda`]: crate::khata::KhataTilqai::MarhalaMarfuda
pub fn anshir(
    jidhr: &Path,
    luba: &LubaTilqai<'_>,
    imkaniyat: &TaqreerImkaniyat,
    makhzun: &JadwalMakhzun,
    madakhil: &[MudkhalNass],
    isdar_taarib: &str,
    waqt: &str,
) -> NatijatTilqai<NatijatNashr> {
    let rafd =
        |khata: taarib_istikhraj::khata::KhataIstikhraj| marfuda(MarhalaTilqai::Istikhraj, khata);

    if jidhr.join(MALAF_MASHRU).is_file() {
        let mut mashru = MashruMaftuh::iftah(jidhr.to_path_buf()).map_err(rafd)?;
        // The damaged-row count is discarded on purpose: a project whose file
        // has unreadable lines is the workshop's own rescue flow, and a run
        // must not rewrite a torn file from a partial read — so nothing is
        // written back unless every row read.
        let (sabiqa, talifa) = mashru.iqra_nusus().map_err(rafd)?;
        if talifa > 0 {
            return Ok(NatijatNashr {
                majmu: sabiqa.len(),
                jadeeda: 0,
                munsha: false,
            });
        }
        let (madmuja, jadeeda) = crate::tanfidh::damm_jadeed(sabiqa, madakhil.to_vec());
        let majmu = madmuja.len();
        if jadeeda > 0 {
            mashru.uktub_kul(&madmuja, waqt.to_owned()).map_err(rafd)?;
        }
        return Ok(NatijatNashr {
            majmu,
            jadeeda,
            munsha: false,
        });
    }

    let mut mashru = MashruMaftuh::ansha(
        jidhr.to_path_buf(),
        luba.huwiya(),
        luba.ism.to_owned(),
        bayan(imkaniyat, makhzun, isdar_taarib, waqt),
        waqt.to_owned(),
    )
    .map_err(rafd)?;
    mashru.uktub_kul(madakhil, waqt.to_owned()).map_err(rafd)?;
    mashru.ikhtim(waqt.to_owned()).map_err(rafd)?;
    Ok(NatijatNashr {
        majmu: madakhil.len(),
        jadeeda: madakhil.len(),
        munsha: true,
    })
}

/// The rows the workshop already holds for this game, or none at all.
///
/// A game with no project, a project with no rows, and a project whose file has
/// unreadable lines all answer the same way: with nothing. The last of those is
/// deliberate — a damaged file is the workshop's own rescue flow, and a run that
/// seeded itself from the survivors would quietly ship a translation missing
/// whatever did not read.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] when a project exists and its header cannot
/// be opened, which means a project is there and this run cannot see it — the
/// one case that must not pass silently.
///
/// [`KhataTilqai::MarhalaMarfuda`]: crate::khata::KhataTilqai::MarhalaMarfuda
pub fn iqra(jidhr: &Path) -> NatijatTilqai<Vec<MudkhalNass>> {
    if !jidhr.join(MALAF_MASHRU).is_file() {
        return Ok(Vec::new());
    }
    let mashru = MashruMaftuh::iftah(jidhr.to_path_buf())
        .map_err(|khata| marfuda(MarhalaTilqai::Istikhraj, khata))?;
    let (sufuf, talifa) = mashru
        .iqra_nusus()
        .map_err(|khata| marfuda(MarhalaTilqai::Istikhraj, khata))?;
    Ok(if talifa > 0 { Vec::new() } else { sufuf })
}

/// Where this run's rows belong in the caller's project store, if it has one.
#[must_use]
pub fn masar(talab: &TalabTilqai<'_>) -> Option<std::path::PathBuf> {
    talab
        .mashari
        .map(|jidhr| jidhr.join(talab.luba.huwiya().to_string()))
}

/// What the project records about how its strings were obtained.
///
/// The refusal report is carried over whole, because the question a
/// contributor asks the workshop three weeks later — "why does this project
/// only have the menus" — is answered by that report and by nothing else.
fn bayan(
    imkaniyat: &TaqreerImkaniyat,
    makhzun: &JadwalMakhzun,
    isdar_taarib: &str,
    waqt: &str,
) -> BayanIstikhraj {
    BayanIstikhraj {
        aila: crate::bina::ism_aila(imkaniyat.muharrik.aila).to_owned(),
        isdar: imkaniyat.muharrik.isdar.as_ref().map(ToString::to_string),
        bina_manassa: None,
        basmat_luba: None,
        turuq: Vec::new(),
        rafd: makhzun.rafd.clone(),
        waqt: waqt.to_owned(),
        isdar_taarib: isdar_taarib.to_owned(),
    }
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::luba::MasdarLuba;
    use taarib_mustalahat::muharrik::{
        AilatMuharrik, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya, Muharrik, Tabaqa,
    };
    use taarib_mustalahat::muraja::SijillMuraja;
    use taarib_mustalahat::nass::{MasdarIstikhraj, NassId, QuyudNass, SiyaqNass, TasnifNass};
    use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

    use super::*;

    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const WAQT: &str = "2026-01-01T00:00:00Z";

    fn satr(mawqi: &str, hadaf: Option<&str>) -> MudkhalNass {
        MudkhalNass {
            id: NassId::min_mawqi("hawiya", mawqi, "masdar"),
            masdar: format!("source {mawqi}"),
            hadaf: hadaf.map(str::to_owned),
            muraja: SijillMuraja::jadeed(),
            siyaq: SiyaqNass::default(),
            quyud: QuyudNass::default(),
            nasq_masdar: Vec::new(),
            nasq_hadaf: Vec::new(),
            takrar: 1,
            majmua: None,
            alamat: Vec::new(),
            tareeqa: None,
            muzawwid: None,
            muharrir: None,
            akhir_tabdeel: None,
            tasnif: TasnifNass::Ism,
            thiqat_tasnif: 100,
            masdar_istikhraj: MasdarIstikhraj::Sakin,
            tarmiz: Some("UTF-8".to_owned()),
        }
    }

    fn imkaniyat() -> TaqreerImkaniyat {
        TaqreerImkaniyat {
            muharrik: Muharrik {
                aila: AilatMuharrik::Unity,
                isdar: None,
                khalfiya: KhalfiyaBarmajiya::Mono,
                itarat: Vec::new(),
                rusum: Vec::new(),
                mimariya: Mimariya::X8664,
                thiqa: 100,
                dalail: Vec::new(),
            },
            tabaqa: Tabaqa::Kamil,
            sabab_arabi: String::new(),
            sabab_injilizi: String::new(),
            jahiziya: JahiziyatTashghil::Mukammala,
            naqs: None,
            anzimat_qabila: Vec::new(),
            jawda: JawdaMutawaqqaa::Mumtaza,
            hudud: Vec::new(),
            marfuda: false,
            isdar_fahs: 10,
            waqt: WAQT.to_owned(),
        }
    }

    fn makhzun() -> JadwalMakhzun {
        JadwalMakhzun {
            jadwal: taarib_istikhraj::jadwal::JadwalNusus::default(),
            rafd: taarib_istikhraj::rafd::TaqreerRafd::default(),
            multaqat: false,
        }
    }

    fn luba(jidhr: &Path) -> LubaTilqai<'_> {
        LubaTilqai {
            jidhr,
            ism: "Luba Ikhtibar",
            masdar: MasdarLuba::Steam(480),
            tanfidhi: None,
            nizam: NizamTashghil::Windows,
            beea: &BeeatTawafuq::Asli,
        }
    }

    fn anshir_fi(jidhr: &Path, sufuf: &[MudkhalNass]) -> NatijatTilqai<NatijatNashr> {
        anshir(
            jidhr,
            &luba(Path::new("/luba")),
            &imkaniyat(),
            &makhzun(),
            sufuf,
            "1.0.1",
            WAQT,
        )
    }

    /// The defect this module was written for: a finished run left the workshop
    /// with no project, so the screen told the user to start a translation they
    /// had just finished.
    #[test]
    fn awwal_nashr_yabni_mashruan_taqrauhu_al_warsha() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr = muaqqat.path().join("mashru");
        let sufuf = vec![satr("a", Some("ألف")), satr("b", None)];

        let nashr = anshir_fi(&jidhr, &sufuf)?;
        assert!(nashr.munsha, "the project did not exist before this run");
        assert_eq!(nashr.majmu, 2);

        let maqrua = iqra(&jidhr)?;
        assert_eq!(
            maqrua.len(),
            2,
            "the workshop reads back what the run wrote"
        );
        let awwal = maqrua.first().ok_or("the first row")?;
        assert_eq!(
            awwal.hadaf.as_deref(),
            Some("ألف"),
            "the translation travelled with the row"
        );
        Ok(())
    }

    /// A second run must not replace a human's wording with a machine's.
    #[test]
    fn nashr_thani_yahfadh_tahrir_al_warsha_wa_yudif_al_jadeed() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr = muaqqat.path().join("mashru");
        let _ = anshir_fi(&jidhr, &[satr("a", Some("آلة"))])?;

        // What the workshop would hold after somebody corrected that row.
        let mut muharrara = iqra(&jidhr)?;
        muharrara
            .first_mut()
            .ok_or("the row the run just wrote")?
            .hadaf = Some("مصحّحة".to_owned());
        {
            let mut mashru = MashruMaftuh::iftah(jidhr.clone())?;
            mashru.uktub_kul(&muharrara, WAQT.to_owned())?;
        }

        let nashr = anshir_fi(&jidhr, &[satr("a", Some("آلة")), satr("b", Some("باء"))])?;
        assert!(!nashr.munsha);
        assert_eq!(nashr.jadeeda, 1, "only the row the workshop lacked");
        assert_eq!(nashr.majmu, 2);

        let maqrua = iqra(&jidhr)?;
        let alif = maqrua
            .iter()
            .find(|mudkhal| mudkhal.id == satr("a", None).id)
            .ok_or("the edited row survived")?;
        assert_eq!(
            alif.hadaf.as_deref(),
            Some("مصحّحة"),
            "the correction was not overwritten by the machine pass"
        );
        Ok(())
    }

    /// A game nobody has translated reads as no rows rather than as a failure.
    #[test]
    fn qiraa_bila_mashru_tuid_la_shay() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        assert!(iqra(&muaqqat.path().join("ghayr_mawjud"))?.is_empty());
        Ok(())
    }
}
