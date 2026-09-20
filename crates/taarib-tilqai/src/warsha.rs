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
use taarib_istikhraj::rafd::TaqreerRafd;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustalahat::nass::MudkhalNass;

use crate::khata::{NatijatTilqai, marfuda};
use crate::talab::TalabTilqai;
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
    luba: LubaId,
    ism_luba: &str,
    bayan: BayanIstikhraj,
    madakhil: &[MudkhalNass],
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
        // A project written before the record carried a binding cannot be
        // submitted, and the refusal reaches the user at the submission screen
        // where nothing can be done about it. This run is standing in front of
        // the game and has just measured it, so it is the one party able to
        // repair that — and only that: a record that already binds keeps its
        // fingerprint, which was taken when the game was the build the project
        // was made from.
        if !yarbut(&mashru.rasm().bayan) {
            mashru.ashil_bayan(bayan, waqt.to_owned()).map_err(rafd)?;
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
        luba,
        ism_luba.to_owned(),
        bayan,
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
/// The refusal report is carried over whole, because the question a contributor
/// asks the workshop three weeks later — "why does this project only have the
/// menus" — is answered by that report and by nothing else.
///
/// It delegates to the compile stage's own builder rather than assembling a
/// thinner record of its own. That is not tidiness. `IrtibatBina::min_bayan`
/// refuses a record naming neither a launcher build nor a fingerprint, because
/// a patch that declares nothing matches nothing safely — the installer would
/// have to refuse it always, which makes it useless, or accept it always, which
/// makes it dangerous. A second construction of this value left both of those
/// fields empty, so every project the automatic run published answered
/// `TAARIB-E-6107` the moment somebody tried to submit it.
///
/// # Errors
///
/// Whatever the fingerprint plan raises. The game's own files are read here, so
/// a game that has moved or been uninstalled cannot produce a binding.
pub fn bayan(
    jidhr_luba: &Path,
    imkaniyat: &TaqreerImkaniyat,
    rafd: &TaqreerRafd,
    isdar_taarib: &str,
    waqt: &str,
) -> NatijatTilqai<BayanIstikhraj> {
    crate::bina::bayan_istikhraj(jidhr_luba, rafd, imkaniyat, waqt, isdar_taarib)
        .map(|(bayan, _, _)| bayan)
}

/// The same record for a caller that cannot reach the game.
///
/// Recovery rebuilds a project from a run that finished, possibly weeks ago, on
/// a game that may since have moved or been uninstalled. Refusing to recover at
/// all in that case would trade an unsubmittable project for no project — and
/// the rows are the part a person cannot rebuild by hand, while the binding is
/// repaired by the next run that stands in front of the game. So this records
/// what is known and leaves the binding empty, and [`anshir`] fills it in later.
///
/// The run itself must never use this: publishing an unbindable project is the
/// defect `TAARIB-E-6107` reports, and the run is by definition standing in
/// front of the game.
#[must_use]
pub fn bayan_ghayr_murtabit(
    imkaniyat: Option<&TaqreerImkaniyat>,
    rafd: TaqreerRafd,
    isdar_taarib: &str,
    waqt: &str,
) -> BayanIstikhraj {
    BayanIstikhraj {
        aila: imkaniyat.map_or_else(String::new, |taqreer| {
            crate::bina::ism_aila(taqreer.muharrik.aila).to_owned()
        }),
        isdar: imkaniyat
            .and_then(|taqreer| taqreer.muharrik.isdar.as_ref().map(ToString::to_string)),
        bina_manassa: None,
        basmat_luba: None,
        turuq: Vec::new(),
        rafd,
        waqt: waqt.to_owned(),
        isdar_taarib: isdar_taarib.to_owned(),
    }
}

/// Whether a project's stored record can still bind a package to a build.
///
/// The one question `IrtibatBina::min_bayan` asks, asked here so a project
/// written before this record was carried properly is recognised and repaired
/// rather than failing at submission with nothing the user can act on.
#[must_use]
pub fn yarbut(bayan: &BayanIstikhraj) -> bool {
    bayan.bina_manassa.is_some() || bayan.basmat_luba.is_some()
}

/// Measures the installed game and writes the binding a project is missing.
///
/// The value `IrtibatBina::min_bayan` wants is a measurement of an installed
/// game, not something a person can type, so the only way out of `TAARIB-E-6107`
/// for a project already on disk is to stand in front of the game and take it.
/// Any surface that can reach the game can call this; the run's own publish does
/// its own repair inline because it has a freshly measured record in hand.
///
/// Answers whether it wrote anything. A record that **already binds is left
/// exactly as it is** and answers `false`: its fingerprint was taken when the
/// game was the build the project was made from, and replacing it with a
/// measurement of whatever is installed today would rebind somebody's work to a
/// build it was not made against.
///
/// The repaired record keeps the project's own extraction time, its refusal
/// report and the Taarib build that extracted it. Those describe how the strings
/// were obtained, which this does not change — only `waqt`, the moment the
/// header is written, is the caller's.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] carrying whatever the fingerprint plan said:
/// the game's own files are read here, so a game that has moved, been
/// uninstalled, or been updated past the containers the record names cannot be
/// measured. The caller is the one that knows how to say that to a user.
///
/// [`KhataTilqai::MarhalaMarfuda`]: crate::khata::KhataTilqai::MarhalaMarfuda
pub fn aslih(
    mashru: &mut MashruMaftuh,
    jidhr_luba: &Path,
    imkaniyat: &TaqreerImkaniyat,
    waqt: &str,
) -> NatijatTilqai<bool> {
    if yarbut(&mashru.rasm().bayan) {
        return Ok(false);
    }
    let (rafd, isdar_taarib, waqt_istikhraj) = {
        let sabiq = &mashru.rasm().bayan;
        (
            sabiq.rafd.clone(),
            sabiq.isdar_taarib.clone(),
            sabiq.waqt.clone(),
        )
    };
    let maqis = bayan(jidhr_luba, imkaniyat, &rafd, &isdar_taarib, &waqt_istikhraj)?;
    mashru
        .ashil_bayan(maqis, waqt.to_owned())
        .map_err(|khata| marfuda(MarhalaTilqai::Istikhraj, khata))?;
    Ok(true)
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::luba::MasdarLuba;
    use taarib_mustalahat::muharrik::{
        AilatMuharrik, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya, Muharrik, Tabaqa,
    };
    use taarib_mustalahat::muraja::SijillMuraja;
    use taarib_mustalahat::nass::{MasdarIstikhraj, NassId, QuyudNass, SiyaqNass, TasnifNass};
    use taarib_tarqee::irtibat::IrtibatBina;
    use taarib_usus::manassa::Mimariya;

    use super::*;

    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const WAQT: &str = "2026-01-01T00:00:00Z";

    /// When a repair runs, which is later than the extraction it repairs.
    const WAQT_ISLAH: &str = "2026-03-01T00:00:00Z";

    /// The one container the fixture game ships and the fixture extraction read.
    const HAWIYA: &str = "Luba_Data/resources.assets";

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

    /// A record carrying a binding, as every real one does.
    ///
    /// Built by hand rather than through [`bayan`], because that reads the
    /// game's own files and there is no game here. What matters to these tests
    /// is that a published project carries a binding at all — see
    /// `al_bayan_yahmil_irtibatan`.
    fn bayan_murtabit() -> BayanIstikhraj {
        BayanIstikhraj {
            aila: "unity".to_owned(),
            isdar: None,
            bina_manassa: Some("14680755".to_owned()),
            basmat_luba: None,
            turuq: Vec::new(),
            rafd: TaqreerRafd::default(),
            waqt: WAQT.to_owned(),
            isdar_taarib: "1.0.1".to_owned(),
        }
    }

    fn anshir_fi(jidhr: &Path, sufuf: &[MudkhalNass]) -> NatijatTilqai<NatijatNashr> {
        anshir(
            jidhr,
            LubaId::min_masdar(&MasdarLuba::Steam(480), "Luba Ikhtibar"),
            "Luba Ikhtibar",
            bayan_murtabit(),
            sufuf,
            WAQT,
        )
    }

    /// The defect behind `TAARIB-E-6107`: a project published with neither a
    /// launcher build nor a fingerprint cannot be submitted, and the refusal
    /// arrives at the submission screen with nothing the user can act on.
    #[test]
    fn al_bayan_yahmil_irtibatan() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr = muaqqat.path().join("mashru");
        let _ = anshir_fi(&jidhr, &[satr("a", Some("ألف"))])?;

        let mashru = MashruMaftuh::iftah(jidhr)?;
        assert!(
            yarbut(&mashru.rasm().bayan),
            "the published project records neither a launcher build nor a fingerprint, so              submission would refuse it"
        );
        Ok(())
    }

    /// A project written by an older build carries an unbindable record; the
    /// next run standing in front of the game repairs it rather than leaving
    /// the user to meet `TAARIB-E-6107` at the submission screen.
    #[test]
    fn nashr_yuslih_bayanan_la_yarbut() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr = muaqqat.path().join("mashru");

        // As the defective publish wrote it: rows, and a record binding nothing.
        let mut kasir = bayan_murtabit();
        kasir.bina_manassa = None;
        kasir.basmat_luba = None;
        {
            let mut mashru = MashruMaftuh::ansha(
                jidhr.clone(),
                LubaId::min_masdar(&MasdarLuba::Steam(480), "Luba Ikhtibar"),
                "Luba Ikhtibar".to_owned(),
                kasir,
                WAQT.to_owned(),
            )?;
            mashru.uktub_kul(&[satr("a", Some("ألف"))], WAQT.to_owned())?;
        }
        assert!(!yarbut(&MashruMaftuh::iftah(jidhr.clone())?.rasm().bayan));

        let _ = anshir_fi(&jidhr, &[satr("a", Some("ألف")), satr("b", Some("باء"))])?;
        assert!(
            yarbut(&MashruMaftuh::iftah(jidhr)?.rasm().bayan),
            "the run measured the game and the project still cannot be bound"
        );
        Ok(())
    }

    /// A record that already binds keeps what it has: its fingerprint was taken
    /// when the game was the build the project was made from.
    #[test]
    fn nashr_la_yaktub_fawq_bayan_yarbut() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr = muaqqat.path().join("mashru");
        let _ = anshir_fi(&jidhr, &[satr("a", Some("ألف"))])?;

        let qabl = MashruMaftuh::iftah(jidhr.clone())?.rasm().bayan.clone();
        let _ = anshir_fi(&jidhr, &[satr("a", Some("ألف")), satr("b", Some("باء"))])?;
        let baad = MashruMaftuh::iftah(jidhr)?.rasm().bayan.clone();
        assert_eq!(qabl.bina_manassa, baad.bina_manassa);
        Ok(())
    }

    /// And the check itself refuses a record that names neither.
    #[test]
    fn bayan_bila_huwiya_la_yarbut() {
        let mut bayan = bayan_murtabit();
        bayan.bina_manassa = None;
        bayan.basmat_luba = None;
        assert!(!yarbut(&bayan));
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

    /// An installed game holding the one container the extraction read, and the
    /// refusal report naming it — the shape a real extraction leaves behind.
    fn luba_ala_alqurs(jidhr: &Path) -> NatijatIkhtibar<TaqreerRafd> {
        let masar = jidhr.join(HAWIYA);
        if let Some(mujallad) = masar.parent() {
            std::fs::create_dir_all(mujallad)?;
        }
        std::fs::write(&masar, b"container bytes")?;
        Ok(rafd_yaqra_alhawiya())
    }

    /// The same report, for a game that is not on disk.
    fn rafd_yaqra_alhawiya() -> TaqreerRafd {
        let mut rafd = TaqreerRafd::jadeed();
        rafd.sajjil_qira(HAWIYA, 3, "fixture container");
        rafd
    }

    /// A project as the defective publish left it: rows, a refusal report, and
    /// a record binding nothing.
    fn mashru_bila_irtibat(jidhr: &Path, rafd: TaqreerRafd) -> NatijatIkhtibar {
        let mut bayan = bayan_murtabit();
        bayan.bina_manassa = None;
        bayan.basmat_luba = None;
        bayan.rafd = rafd;
        let mut mashru = MashruMaftuh::ansha(
            jidhr.to_path_buf(),
            LubaId::min_masdar(&MasdarLuba::Steam(480), "Luba Ikhtibar"),
            "Luba Ikhtibar".to_owned(),
            bayan,
            WAQT.to_owned(),
        )?;
        mashru.uktub_kul(&[satr("a", Some("ألف"))], WAQT.to_owned())?;
        Ok(())
    }

    /// The whole point: a project that answered `TAARIB-E-6107` passes the gate
    /// afterwards, without anybody translating anything a second time.
    #[test]
    fn islah_yaqees_alluba_fayamurr_min_bawwabat_alirtibat() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr_luba = muaqqat.path().join("luba");
        let rafd = luba_ala_alqurs(&jidhr_luba)?;
        let jidhr = muaqqat.path().join("mashru");
        mashru_bila_irtibat(&jidhr, rafd)?;

        let mut mashru = MashruMaftuh::iftah(jidhr.clone())?;
        assert!(
            IrtibatBina::min_bayan(&mashru.rasm().bayan, &[], None).is_err(),
            "the fixture already binds, so it is not the project the owner is stuck on"
        );
        assert!(aslih(&mut mashru, &jidhr_luba, &imkaniyat(), WAQT_ISLAH)?);

        let baad = MashruMaftuh::iftah(jidhr)?;
        assert!(
            baad.rasm().bayan.basmat_luba.is_some(),
            "the game was measured and the fingerprint was not written"
        );
        // The gate the submission runs, run here rather than re-stated.
        let irtibat = IrtibatBina::min_bayan(&baad.rasm().bayan, &[], None)?;
        assert_eq!(irtibat.adad_malaffat, 1);
        // The record still describes the extraction it always described; only
        // the header's write time moved.
        assert_eq!(baad.rasm().bayan.waqt, WAQT);
        assert_eq!(baad.rasm().bayan.isdar_taarib, "1.0.1");
        assert_eq!(baad.rasm().waqt_tabdeel, WAQT_ISLAH);
        Ok(())
    }

    /// A game that is no longer there cannot be measured, and the project is
    /// left exactly as it was rather than half-written.
    #[test]
    fn islah_yarfud_luba_ghayr_mawjuda() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr = muaqqat.path().join("mashru");
        mashru_bila_irtibat(&jidhr, rafd_yaqra_alhawiya())?;

        let mut mashru = MashruMaftuh::iftah(jidhr.clone())?;
        assert!(
            aslih(
                &mut mashru,
                &muaqqat.path().join("ghayr_mawjuda"),
                &imkaniyat(),
                WAQT_ISLAH,
            )
            .is_err()
        );
        let baad = MashruMaftuh::iftah(jidhr)?;
        assert!(!yarbut(&baad.rasm().bayan));
        assert_eq!(baad.rasm().waqt_tabdeel, WAQT);
        Ok(())
    }

    /// A record that already binds is not re-measured: its fingerprint was taken
    /// when the game was the build the project was made from, and the installed
    /// game may have moved on since.
    #[test]
    fn islah_la_yamiss_bayanan_yarbut() -> NatijatIkhtibar {
        let muaqqat = tempfile::tempdir()?;
        let jidhr_luba = muaqqat.path().join("luba");
        let rafd = luba_ala_alqurs(&jidhr_luba)?;
        let jidhr = muaqqat.path().join("mashru");
        {
            let mut bayan = bayan_murtabit();
            bayan.rafd = rafd;
            let mut mashru = MashruMaftuh::ansha(
                jidhr.clone(),
                LubaId::min_masdar(&MasdarLuba::Steam(480), "Luba Ikhtibar"),
                "Luba Ikhtibar".to_owned(),
                bayan,
                WAQT.to_owned(),
            )?;
            mashru.uktub_kul(&[satr("a", Some("ألف"))], WAQT.to_owned())?;
        }

        let mut mashru = MashruMaftuh::iftah(jidhr.clone())?;
        assert!(!aslih(&mut mashru, &jidhr_luba, &imkaniyat(), WAQT_ISLAH)?);

        let baad = MashruMaftuh::iftah(jidhr)?;
        assert_eq!(baad.rasm().bayan.bina_manassa.as_deref(), Some("14680755"));
        assert_eq!(
            baad.rasm().bayan.basmat_luba,
            None,
            "a record that already bound was re-measured against today's install"
        );
        assert_eq!(baad.rasm().waqt_tabdeel, WAQT);
        Ok(())
    }
}
