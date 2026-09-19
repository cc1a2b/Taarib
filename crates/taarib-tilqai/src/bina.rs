//! Stages 4, 5 and 6 — the fonts and sizes, the compile, and the seal.
//!
//! Three stages rather than one because they fail for three different reasons
//! and a user needs to be told which: a font that will not validate is a
//! settings problem, a string that will not lay out is a translation problem,
//! and a signature that will not verify is a key problem. They are in one file
//! because they are one linear sequence over one set of borrowed inputs, and
//! splitting them would mean a struct whose only purpose is to carry the
//! sequence's own locals between two functions.
//!
//! The layout work itself is not done here. `taarib_tarqee::mujammi::ijmaa`
//! runs `takhtit::sabbiq` internally — shaping every string at every discovered
//! size through `taarib-saff`, then rasterizing the glyph set into an atlas
//! through `taarib-lawha` — and running it a second time out here to tick a
//! progress bar would double the most expensive part of the compile. Stage 4 is
//! therefore everything that *decides* the layout work, with a real count of
//! the (string, size) pairs the compiler is about to do; stage 5 reports what
//! came out.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_istikhraj::mashru::{BayanIstikhraj, TareeqatHawiya};
use taarib_istikhraj::rafd::TaqreerRafd;
use taarib_khatm::{MIFTAH_TATWIR, MiftahKhass, MudaqqiqEd25519};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustalahat::nass::MudkhalNass;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision, TareeqaTarjama};
use taarib_ruqaa::katib;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_ruqaa::tarwisa::Tarwisa;
use taarib_ruqaa::tawqee::{DawrMiftah, Khwarizmiya, KutlatTawqee};
use taarib_saff::khatt::{MawridKhatt, SilsilatKhutut};
use taarib_tarqee::bawwaba::KhattMujammaa;
use taarib_tarqee::bayan::MuharrikHuzma;
use taarib_tarqee::fuhusat::{MudkhalatFahs, WasfHuzma, ijri as ijri_fuhus};
use taarib_tarqee::irtibat::{IrtibatBina, MukhattatBasma};
use taarib_tarqee::mujammi::{MudkhalatTajmee, ijmaa};
use taarib_tarqee::taghtiya_ruqaa::ihsib_taghtiya;
use taarib_tarqee::tahdid_maqasat::{IktishafMaqasat, TaqreerMaqasat};
use taarib_tarqee::takhtit::KhiyaratTasbeeq;

use crate::khata::{KhataTilqai, NatijatTilqai, khata_malaf, marfuda};
use crate::talab::TalabTilqai;
use crate::taqaddum::{MarhalaTilqai, Muraqib};
use crate::taqreer::IhsaHuzma;

/// The fonts, validated, gated and staged, and the sizes to lay out at.
#[derive(Debug)]
pub struct Tahdeer {
    /// The shaping chain.
    pub silsila: SilsilatKhutut,
    /// The same fonts as records the container declares, in the same order.
    ///
    /// `taarib_saff::natija::Harf::khatt` is an index into the chain and the
    /// container stores it verbatim, so a chain and a font list that disagree
    /// produce a patch whose every glyph names the wrong face. Keeping them in
    /// one struct is what stops them from drifting apart.
    pub khutut: Vec<KhattMujammaa>,
    /// Which sizes each string was measured at.
    pub maqasat: TaqreerMaqasat,
    /// How many (string, size) pairs the compiler will lay out.
    pub azwaj: u64,
}

/// Stage 4 — validate the fonts, stage them, and decide every size.
///
/// # Errors
///
/// [`KhataTilqai::LaKhatt`] when no font was supplied, or when the first one
/// will not pass the validator or the font-directory gate. Not a warning: a
/// patch with no font is a patch that draws nothing.
///
/// [`KhataTilqai::KhataMalaf`] when a font cannot be copied into the run
/// directory.
///
/// [`KhataTilqai::Mulgha`] when the run was cancelled.
pub fn hayyi(
    talab: &TalabTilqai<'_>,
    nusus: &[MudkhalNass],
    jidhr_khutut: &Path,
    muraqib: &Muraqib<'_>,
) -> NatijatTilqai<Tahdeer> {
    talab.miqbad.tahaqquq(MarhalaTilqai::Takhtit)?;

    if talab.khutut.is_empty() {
        return Err(KhataTilqai::LaKhatt {
            sabab: "no font was supplied; a patch has to carry the face it was shaped against"
                .to_owned(),
        });
    }
    let adad_khutut = tul(talab.khutut.len());
    muraqib.ballagh(
        MarhalaTilqai::Takhtit,
        0,
        adad_khutut,
        format!("validating {adad_khutut} font(s)"),
    );

    fs::create_dir_all(jidhr_khutut)
        .map_err(|sabab| khata_malaf(jidhr_khutut, "created", sabab))?;
    let mut mawarid: Vec<Arc<MawridKhatt>> = Vec::with_capacity(talab.khutut.len());
    let mut khutut: Vec<KhattMujammaa> = Vec::with_capacity(talab.khutut.len());
    for (fahras, masdar) in talab.khutut.iter().enumerate() {
        talab.miqbad.tahaqquq(MarhalaTilqai::Takhtit)?;
        let ism = masdar.file_name().ok_or_else(|| KhataTilqai::LaKhatt {
            sabab: format!("{} does not name a file", masdar.display()),
        })?;
        let wijha = jidhr_khutut.join(ism);
        // Copied rather than referenced in place: the font-directory gate
        // canonicalizes and requires the font to be *inside* the directory it
        // was handed, which is what stops a manifest from naming a font
        // somewhere else on the machine that a later run would not find.
        let _ = fs::copy(masdar, &wijha)
            .map_err(|sabab| khata_malaf(masdar, "copied into the run directory", sabab))?;
        let bayt = Arc::new(fs::read(&wijha).map_err(|sabab| khata_malaf(&wijha, "read", sabab))?);
        let mawrid = MawridKhatt::jadeed(bayt, 0).map_err(|khata| KhataTilqai::LaKhatt {
            sabab: format!("{}: {}", wijha.display(), khata.injilizi),
        })?;
        let khatt = KhattMujammaa::min_majmua(jidhr_khutut, &wijha, 0).map_err(|khata| {
            KhataTilqai::LaKhatt {
                sabab: format!("{}: {khata}", wijha.display()),
            }
        })?;
        mawarid.push(Arc::new(mawrid));
        khutut.push(khatt);
        muraqib.ballagh(
            MarhalaTilqai::Takhtit,
            tul(fahras.saturating_add(1)),
            adad_khutut,
            format!(
                "{} passed the validator and the font-directory gate",
                wijha.display()
            ),
        );
    }
    let silsila = SilsilatKhutut::jadeeda(mawarid).map_err(|khata| KhataTilqai::LaKhatt {
        sabab: khata.injilizi,
    })?;

    let maqasat = maqasat(nusus, &talab.khiyarat.ahjam);
    let azwaj = tul(nusus
        .iter()
        .filter(|mudkhal| mudkhal.hadaf.is_some())
        .map(|mudkhal| {
            maqasat
                .maqasat_nass(mudkhal.id)
                .map_or_else(|| maqasat.ittihad().len(), |nass| nass.ahjam().len())
        })
        .sum::<usize>());

    muraqib.ikhtim(
        MarhalaTilqai::Takhtit,
        adad_khutut,
        format!(
            "{adad_khutut} font(s), {} distinct size(s), {azwaj} (string, size) pair(s) to lay out",
            maqasat.ittihad().len()
        ),
    );
    Ok(Tahdeer {
        silsila,
        khutut,
        maqasat,
        azwaj,
    })
}

/// Every size every translated string is laid out at.
///
/// Each string is registered at every configured size rather than at one, which
/// is the difference between a patch that draws correctly in the menu and the
/// subtitles and one that draws correctly in the menu and rescales everywhere
/// else. A size a capture session actually observed is registered too, through
/// `sajjil_mudkhal`, and wins nothing over the configured set — it joins it.
fn maqasat(nusus: &[MudkhalNass], ahjam: &[f32]) -> TaqreerMaqasat {
    let mut iktishaf = IktishafMaqasat::jadeed();
    for mudkhal in nusus {
        iktishaf.sajjil_mudkhal(mudkhal);
    }
    for hajm in ahjam {
        iktishaf.sajjil_muhawwil_amm(*hajm);
        for mudkhal in nusus.iter().filter(|mudkhal| mudkhal.hadaf.is_some()) {
            iktishaf.sajjil_muhawwil(mudkhal.id, *hajm);
        }
    }
    iktishaf.ahsi(nusus)
}

/// The fingerprint of every translation a package would be compiled from.
///
/// A resumed run has to decide whether the container beside it is still the
/// container this table would produce, and the honest answer is "only if the
/// translations have not moved since". Counting them would miss an edit that
/// replaced one string without adding any — which is exactly what the review
/// workspace does — so the identity and the target text of every translated row
/// go into the hash.
///
/// Order-independent by construction: the rows are hashed into a set that is
/// sorted before it is folded, because the table's order is the extractor's and
/// a resume has no reason to preserve it.
#[must_use]
pub fn basmat_tarjamat(nusus: &[MudkhalNass]) -> String {
    let mut basmat: Vec<[u8; 32]> = nusus
        .iter()
        .filter_map(|mudkhal| {
            let hadaf = mudkhal.hadaf.as_deref()?;
            if hadaf.trim().is_empty() {
                return None;
            }
            let mut hasib = blake3::Hasher::new();
            hasib.update(mudkhal.id.to_string().as_bytes());
            hasib.update(b"\0");
            hasib.update(hadaf.as_bytes());
            Some(*hasib.finalize().as_bytes())
        })
        .collect();
    basmat.sort_unstable();
    let mut hasib = blake3::Hasher::new();
    for basma in &basmat {
        hasib.update(basma);
    }
    sittasi(hasib.finalize().as_bytes())
}

/// The extraction provenance the build binding is formed from.
///
/// One recipe entry per container extraction actually read, each with its
/// content hash, and the fingerprint the recipe produces over the game as it
/// stands now. This is what makes a patch refuse to install onto a build it was
/// not made for.
/// The extraction record a project and a package are both bound by.
///
/// Public to the crate because the workshop publish needs the *same* record the
/// compile stage builds, not a reduced one. A project whose record names neither
/// a launcher build nor a fingerprint cannot be submitted at all — `IrtibatBina`
/// refuses it, because a patch that declares nothing matches nothing safely —
/// and a second, thinner construction of this value is exactly how that happened.
pub(crate) fn bayan_istikhraj(
    luba: &Path,
    rafd: &TaqreerRafd,
    imkaniyat: &TaqreerImkaniyat,
    waqt: &str,
    isdar_taarib: &str,
) -> NatijatTilqai<(BayanIstikhraj, Basma, u32)> {
    let mut turuq: Vec<TareeqatHawiya> = Vec::new();
    for qira in &rafd.maqrua {
        if turuq.iter().any(|tareeqa| tareeqa.hawiya == qira.hawiya) {
            continue;
        }
        let basma = fs::read(luba.join(&qira.hawiya))
            .ok()
            .map(|bayt| Basma::min_bayt(*blake3::hash(&bayt).as_bytes()));
        turuq.push(TareeqatHawiya {
            hawiya: qira.hawiya.clone(),
            tareeqa: qira.wasf.clone(),
            basma,
        });
    }

    let mut bayan = BayanIstikhraj {
        aila: ism_aila(imkaniyat.muharrik.aila).to_owned(),
        isdar: imkaniyat
            .muharrik
            .isdar
            .as_ref()
            .map(|isdar| isdar.khaam.clone()),
        bina_manassa: None,
        basmat_luba: None,
        turuq,
        rafd: rafd.clone(),
        waqt: waqt.to_owned(),
        isdar_taarib: isdar_taarib.to_owned(),
    };
    let mukhattat =
        MukhattatBasma::min_bayan(&bayan).map_err(|khata| marfuda(MarhalaTilqai::Tarqee, khata))?;
    let (basma, adad) = mukhattat
        .ihsab(luba)
        .map_err(|khata| marfuda(MarhalaTilqai::Tarqee, khata))?;
    bayan.basmat_luba = Some(basma);
    Ok((bayan, basma, adad))
}

/// How long each of the two stages this function runs actually took.
///
/// Returned rather than measured by the caller, because the caller cannot see
/// the boundary: compiling and sealing are one call here, and timing them from
/// outside would report the seal as taking as long as the compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MilliBina {
    /// Milliseconds the compile took.
    pub tarqee: u64,
    /// Milliseconds the seal took.
    pub khatm: u64,
}

/// Stages 5 and 6 — compile the patch, then seal it.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] for anything the gate, the binding, the
/// compiler or the writer refuses, carrying the refusing crate's own words.
///
/// [`KhataTilqai::KhataMalaf`] when the package cannot be written.
///
/// [`KhataTilqai::Mulgha`] when the run was cancelled.
pub fn ijmi(
    talab: &TalabTilqai<'_>,
    nusus: &[MudkhalNass],
    rafd: &TaqreerRafd,
    imkaniyat: &TaqreerImkaniyat,
    tahdeer: &Tahdeer,
    wijhat_huzma: &Path,
    muraqib: &Muraqib<'_>,
) -> NatijatTilqai<(IhsaHuzma, MilliBina)> {
    let bidaya = std::time::Instant::now();
    talab.miqbad.tahaqquq(MarhalaTilqai::Tarqee)?;

    let mutarjama: Vec<MudkhalNass> = {
        let mut mutarjama: Vec<MudkhalNass> = nusus
            .iter()
            .filter(|mudkhal| {
                mudkhal
                    .hadaf
                    .as_deref()
                    .is_some_and(|hadaf| !hadaf.trim().is_empty())
            })
            .cloned()
            .collect();
        mutarjama.sort_by_key(|mudkhal| mudkhal.id);
        mutarjama
    };

    muraqib.ballagh_bila_majmu(
        MarhalaTilqai::Tarqee,
        0,
        format!(
            "laying out {} string(s) at {} size(s) and building the atlas — the glyph set is \
             whatever shaping turns out to produce, so it cannot be counted before it exists",
            mutarjama.len(),
            tahdeer.maqasat.ittihad().len()
        ),
    );

    let wasf = WasfHuzma {
        unwan: talab.unwan(),
        ism_musahim: talab.wasf.ism_musahim.clone(),
        ism_luba: talab.luba.ism.to_owned(),
        rukhsa: talab.wasf.rukhsa.clone(),
        // The honest label, and not a setting. Nothing in this pipeline is
        // reviewed by a person, so nothing it produces may claim to be.
        tareeqa: TareeqaTarjama::AaliyaFaqat,
        isdar_taarib: talab.wasf.isdar_taarib.clone(),
    };
    let ijtiyaz = ijri_fuhus(&MudkhalatFahs {
        madakhil: &mutarjama,
        wasf: &wasf,
        takhtitat_fashila: &[],
    })
    .map_err(|khata| marfuda(MarhalaTilqai::Tarqee, khata))?;

    let (bayan, _, _) = bayan_istikhraj(
        talab.luba.jidhr,
        rafd,
        imkaniyat,
        &talab.khiyarat.waqt,
        &talab.wasf.isdar_taarib,
    )?;
    let irtibat = IrtibatBina::min_bayan(&bayan, &tahdeer.khutut, None)
        .map_err(|khata| marfuda(MarhalaTilqai::Tarqee, khata))?;

    talab.miqbad.tahaqquq(MarhalaTilqai::Tarqee)?;

    let taghtiya = ihsib_taghtiya(&mutarjama, None, None);
    let khiyarat = KhiyaratTasbeeq::default();
    // The overflow report is not built here: the compiler measures it from
    // the layouts it ships, so a captured width is compared against a real
    // layout rather than reported as having none.
    let mudkhalat = MudkhalatTajmee {
        nusus: &mutarjama,
        id: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::AWWAL,
        wasf: &wasf,
        muharrik: MuharrikHuzma {
            aila: imkaniyat.muharrik.aila,
            khalfiya: imkaniyat.muharrik.khalfiya,
            tabaqa: imkaniyat.tabaqa,
        },
        irtibat: &irtibat,
        maqasat: &tahdeer.maqasat,
        taghtiya: &taghtiya,
        khiyarat: &khiyarat,
        mustawa: None,
    };

    let mut huzma = ijmaa(&mudkhalat, ijtiyaz, &tahdeer.silsila, &tahdeer.khutut)
        .map_err(|khata| marfuda(MarhalaTilqai::Tarqee, khata))?;
    let hajm = tul(huzma.hajm());
    let milli_tarqee = milli(bidaya);
    muraqib.ikhtim(
        MarhalaTilqai::Tarqee,
        tahdeer.azwaj,
        format!("{}: {hajm} byte(s)", huzma.bayan.wasf()),
    );

    // --- stage 6, the seal ---------------------------------------------------
    let bidaya = std::time::Instant::now();
    talab.miqbad.tahaqquq(MarhalaTilqai::Khatm)?;
    muraqib.ballagh(MarhalaTilqai::Khatm, 0, 1, "sealing the package");

    let aam = talab.miftah.aam().bayt();
    let basma = Tarwisa::min_bayt(huzma.bayt.bayt())
        .map_err(|khata| marfuda(MarhalaTilqai::Khatm, khata))?
        .basma;
    let waqt = i64::try_from(talab.khiyarat.lahza).unwrap_or(i64::MAX);
    let mut kutla = KutlatTawqee {
        // The contributor seals their own package here. The owner's signature
        // is added at approval, by the owner, and never by this crate.
        dawr: DawrMiftah::Malik,
        khwarizmiya: Khwarizmiya::Ed25519,
        waqt,
        miftah: aam,
        tawqee: [0; 64],
    };
    kutla.tawqee = talab.miftah.waqqi(&kutla.risala(&basma));
    katib::khatm(huzma.bayt.bayt_mut(), &kutla, &MudaqqiqEd25519)
        .map_err(|khata| marfuda(MarhalaTilqai::Khatm, khata))?;

    taarib_usus::masarat::kitaba_dharra(wijhat_huzma, huzma.bayt.bayt()).map_err(|sabab| {
        khata_malaf(
            wijhat_huzma,
            "written",
            std::io::Error::other(sabab.to_string()),
        )
    })?;
    // Reopened the way an installer opens it — memory-mapped, framing and
    // content hash validated — because a package that only this process can
    // read is a package nobody can install.
    let _ =
        MalafRuqaa::iftah(wijhat_huzma).map_err(|khata| marfuda(MarhalaTilqai::Khatm, khata))?;

    let ihsa = IhsaHuzma {
        masar: wijhat_huzma.to_path_buf(),
        hajm,
        nusus: tul(huzma.bayan.bawwaba.nusus),
        takhtitat: tul(huzma.bayan.bawwaba.takhtitat),
        safahat: tul(huzma.bayan.bawwaba.safahat),
        ashkal: tul(huzma.bayan.takhtit.ashkal),
        ahjam_rubi: huzma
            .bayan
            .takhtit
            .ahjam_rubi
            .iter()
            .map(|rubi| u32::from(*rubi))
            .collect(),
        miftah: sittasi(&aam),
        miftah_tatwir: aam == MIFTAH_TATWIR,
        basma: sittasi(&basma),
    };
    muraqib.ikhtim(
        MarhalaTilqai::Khatm,
        1,
        format!(
            "sealed under key {} and reopened through the installer's reader",
            ihsa.miftah
        ),
    );
    Ok((
        ihsa,
        MilliBina {
            tarqee: milli_tarqee,
            khatm: milli(bidaya),
        },
    ))
}

/// Milliseconds since an instant.
fn milli(bidaya: std::time::Instant) -> u64 {
    u64::try_from(bidaya.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The contributor identity a run signs under: the fingerprint of its own
/// public signing key.
///
/// # Errors
///
/// [`KhataTilqai::MarhalaMarfuda`] when the fingerprint is not one
/// `MusahimId` accepts, which cannot happen for a key of the right length and
/// is checked rather than assumed.
pub fn musahim(miftah: &MiftahKhass) -> NatijatTilqai<taarib_mustalahat::musahim::MusahimId> {
    taarib_mustalahat::musahim::MusahimId::jadeed(sittasi(&miftah.aam().bayt()))
        .map_err(|khata| marfuda(MarhalaTilqai::Khatm, khata))
}

/// Every path in the run directory this stage writes to.
#[derive(Debug, Clone)]
pub struct MasaratBina {
    /// Where fonts are staged.
    pub khutut: PathBuf,
    /// Where the sealed package lands.
    pub huzma: PathBuf,
}

impl MasaratBina {
    /// The paths under one run directory.
    #[must_use]
    pub fn jadeeda(jidhr_mashwar: &Path) -> Self {
        Self {
            khutut: jidhr_mashwar.join(crate::mashwar::MUJALLAD_KHUTUT),
            huzma: jidhr_mashwar.join(crate::mashwar::MALAF_HUZMA),
        }
    }
}

/// Lowercase hexadecimal.
#[must_use]
pub fn sittasi(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    bayt.iter().fold(
        String::with_capacity(bayt.len().saturating_mul(2)),
        |mut nass, b| {
            let _ = write!(nass, "{b:02x}");
            nass
        },
    )
}

/// The recipe's name for an engine family.
///
/// Written out rather than lower-casing `AilatMuharrik::ism`, because that
/// method returns Arabic for the unknown family and a build recipe's engine
/// field is an identifier other tools read.
pub(crate) const fn ism_aila(aila: taarib_mustalahat::muharrik::AilatMuharrik) -> &'static str {
    use taarib_mustalahat::muharrik::AilatMuharrik as A;
    match aila {
        A::Unity => "unity",
        A::Unreal => "unreal",
        A::Godot => "godot",
        A::RpgMakerMv => "rpgmaker-mv",
        A::RpgMakerMz => "rpgmaker-mz",
        A::RpgMakerVxAce => "rpgmaker-vxace",
        A::Renpy => "renpy",
        A::GameMaker => "gamemaker",
        A::Electron => "electron",
        A::Bio4 => "bio4",
        A::Frostbite => "frostbite",
        A::BlackSpace => "blackspace",
        A::Alchemy => "alchemy",
        A::Dantelion => "dantelion",
        A::Rage => "rage",
        A::Snowdrop => "snowdrop",
        A::Majhul => "unknown",
    }
}

/// A count as the report's own width.
fn tul(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::muraja::SijillMuraja;
    use taarib_mustalahat::nass::{MasdarIstikhraj, NassId, QuyudNass, SiyaqNass, TasnifNass};

    use super::*;

    /// One table row, translated or not.
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

    /// A translation added, and a translation changed, both move the
    /// fingerprint. Between them they are every way a table can come to deserve
    /// a package it does not have: a resumed run that translated more, and a
    /// review that corrected one word.
    #[test]
    fn basma_tatabba_kulla_tarjama() {
        let asas = vec![satr("a", Some("ألف")), satr("b", None)];
        let awwal = basmat_tarjamat(&asas);

        let azyad = vec![satr("a", Some("ألف")), satr("b", Some("باء"))];
        assert_ne!(
            awwal,
            basmat_tarjamat(&azyad),
            "a resumed run's new translation left the fingerprint alone, so its package \
             would never be rebuilt"
        );

        let muharrar = vec![satr("a", Some("ألفٌ أخرى")), satr("b", None)];
        assert_ne!(
            awwal,
            basmat_tarjamat(&muharrar),
            "an edited translation left the fingerprint alone"
        );
    }

    /// The order the table happens to be in is the extractor's, and a resume has
    /// no reason to preserve it. An untranslated row contributes nothing, so a
    /// run that only extracted more strings does not force a recompile.
    #[test]
    fn basma_la_tubali_bil_tarteeb_wala_bighayr_almutarjam() {
        let awwal = vec![satr("a", Some("ألف")), satr("b", Some("باء"))];
        let maqlub = vec![satr("b", Some("باء")), satr("a", Some("ألف"))];
        assert_eq!(basmat_tarjamat(&awwal), basmat_tarjamat(&maqlub));

        let mazeed = vec![
            satr("a", Some("ألف")),
            satr("b", Some("باء")),
            satr("c", None),
            satr("d", Some("   ")),
        ];
        assert_eq!(
            basmat_tarjamat(&awwal),
            basmat_tarjamat(&mazeed),
            "a row with no translation, or only blanks, is not something to rebuild for"
        );
    }

    /// Two rows that carry the same Arabic are not one row. Hashing the target
    /// alone would collapse them, and a table that lost a row would keep the
    /// package that still has it.
    #[test]
    fn satran_binafs_altarjama_yabqayan_ithnayn() {
        let ithnan = vec![satr("a", Some("ألف")), satr("b", Some("ألف"))];
        let wahid = vec![satr("a", Some("ألف"))];
        assert_ne!(basmat_tarjamat(&ithnan), basmat_tarjamat(&wahid));
    }
}
