//! العقل — one game held whole, projected once, with the chain behind every answer.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_aql::{
    Aql, HalatFahs, HuwiyatLuba, Khatar, Mani, MawqifMustakhdim, MudkhalatAql, Musnad, NawKhatar,
    NitaqMani, Shahid,
};
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::luba::{Luba, LubaId};
use taarib_mustalahat::muharrik::{Hadd, Tabaqa, TaqreerImkaniyat};
use taarib_tathbeet::{HajatItar, hajat_itar};
use taarib_usus::idadat::MakhzanIdadat;
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;
use taarib_usus::manassa::NizamTashghil;
use taarib_usus::masarat::Masarat;

use crate::luba_awamir::{huwiya, ijlib_luba, jidhr_steam, simat_luba, taqreer_luba};

/// One held input behind one answer.
///
/// Diagnostic rather than user-facing, and the core says why: the sentences a
/// player reads travel in both languages on the blocker, the risk and the
/// promise, and these carry each producer's own words in whatever language that
/// producer wrote them. Translating an observation would put a sentence in the
/// trail no producer ever wrote.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ShahidHie {
    /// Which held input it came from, as a stable machine name.
    pub masdar: String,
    /// The producing module, as a maintainer reading a bundle would look for it.
    pub muntij: String,
    /// What that input said.
    pub wasf: String,
    /// Where it was seen, when the input names a place.
    pub mawqi: Option<String>,
}

/// One standing blocker, already in the one order.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ManiHie {
    /// Which blocker, as the discriminant's own machine name.
    pub naw: String,
    /// Where it sits in the one order; lower is more serious.
    pub rutba: u8,
    /// How far it reaches: `kul` stops everything, `tashghil` only the run.
    pub nitaq: String,
    /// Whether it is a fact about the game that nothing will change.
    pub nihai: bool,
    /// The producer's own sentence, in Arabic.
    pub arabi: String,
    /// The same in English.
    pub injilizi: String,
    /// Every input it rests on.
    pub shawahid: Vec<ShahidHie>,
}

/// One standing risk, and whether it has been answered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KhatarHie {
    /// Which risk, as the discriminant's own machine name.
    pub naw: String,
    /// Where it sits in the order it is asked in.
    pub rutba: u8,
    /// What the user is being asked to accept, in Arabic.
    pub arabi: String,
    /// The same in English.
    pub injilizi: String,
    /// Whether the answer is already on record.
    pub muqarr: bool,
    /// Every input it rests on.
    pub shawahid: Vec<ShahidHie>,
}

/// One named limit, with what said so.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HaddHie {
    /// The limit, in Arabic.
    pub arabi: String,
    /// The same in English.
    pub injilizi: String,
    /// Every input it rests on.
    pub shawahid: Vec<ShahidHie>,
}

/// The two anti-cheat sources naming different things about one game.
///
/// Surfaced rather than averaged. The launcher's catalogue hint and the scan of
/// the game's own files are independent and either can be wrong; the refusal
/// stands on both, and which of them was wrong has to stay visible.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct IkhtilafHimayaHie {
    /// What the scan of the game's files and the store catalogue named.
    pub min_kashf: Vec<String>,
    /// What the launcher's own metadata named.
    pub min_iktishaf: Vec<String>,
}

/// Everything the core answers about one game, in one record.
///
/// The discriminants cross the wire, not a tier number: `muntaj`, each blocker's
/// `naw` and each risk's `naw` are the core's own stable machine names, which
/// are byte-identical to the `snake_case` `serde` spelling of the same variants —
/// asserted by [`ikhtibarat::al_asma_hiya_asma_serde`]. A surface therefore
/// branches on what a thing *is* rather than on a rank or an error code.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct AqlLubaHie {
    /// Taarib's identity for the game.
    pub muarrif: String,
    /// The name its launcher gives it, verbatim.
    pub ism: String,
    /// Which of the products this game gets: `istibdal`, `rasm_mubashir`,
    /// `tabaqa_fawqiya`, `la_shay` or `majhul`.
    pub muntaj: String,
    /// The tier number behind that product, when it is a product at all.
    ///
    /// Absent for `la_shay` and `majhul`, because a number printed beside
    /// "nothing" is a number that contradicts the word next to it.
    pub tabaqa_raqm: Option<u8>,
    /// The product's name in Arabic.
    pub ism_arabi: String,
    /// The same in English.
    pub ism_injilizi: String,
    /// Why this product and not a better one, in Arabic.
    pub sabab_arabi: String,
    /// The same reason in English.
    pub sabab_injilizi: String,
    /// Whether this build delivers what the product promises: `mukammala`,
    /// `naqisa` or `ghaiba`.
    pub jahiziya: String,
    /// What is unfinished, named, in Arabic, when anything is.
    pub naqs_arabi: Option<String>,
    /// The same in English.
    pub naqs_injilizi: Option<String>,
    /// What produced the product answer.
    pub shawahid_muntaj: Vec<ShahidHie>,
    /// Everything standing between this game and Taarib, most serious first.
    ///
    /// Already sorted by the core's one ordering. A surface with room for one
    /// sentence takes the first entry; it does not re-rank.
    pub mawani: Vec<ManiHie>,
    /// Every risk that has to be accepted before anything is written, in the
    /// order they are asked.
    pub makhatir: Vec<KhatarHie>,
    /// Everything that will not work for this game, named specifically.
    pub hudud: Vec<HaddHie>,
    /// What the anti-cheat question was actually answered with: `mahmiya`,
    /// `lam_yajri` or `la_tawqee`.
    ///
    /// The negative answer is not "clean". It states that this build's signature
    /// list was looked for and matched nothing, which is a smaller claim, and
    /// its chain carries every place the scan could not reach.
    pub hala_himaya: String,
    /// What produced that answer.
    pub shawahid_himaya: Vec<ShahidHie>,
    /// The two anti-cheat sources disagreeing, when they do.
    pub ikhtilaf_himaya: Option<IkhtilafHimayaHie>,
    /// Whether anything stops Taarib touching this game at all.
    pub marfuda: bool,
    /// Whether a patch may be installed right now: nothing blocks it outright
    /// and every risk has been answered.
    pub jahiz_lil_tathbeet: bool,
    /// The above, and an adapter that reaches the screen.
    pub jahiz_lil_tashghil: bool,
}

/// How far a blocker reaches, as the wire spells it.
///
/// A total match rather than a serialization, so a third scope added to the core
/// fails this build instead of arriving as an unhandled string.
const fn ism_nitaq(nitaq: NitaqMani) -> &'static str {
    match nitaq {
        NitaqMani::Kul => "kul",
        NitaqMani::Tashghil => "tashghil",
    }
}

fn shahid_hie(shahid: &Shahid) -> ShahidHie {
    ShahidHie {
        masdar: shahid.masdar.ism().to_owned(),
        muntij: shahid.masdar.muntij().to_owned(),
        wasf: shahid.wasf.clone(),
        mawqi: shahid.mawqi.clone(),
    }
}

fn shawahid_hie(shawahid: &[Shahid]) -> Vec<ShahidHie> {
    shawahid.iter().map(shahid_hie).collect()
}

fn mani_hie(mani: &Musnad<Mani>) -> ManiHie {
    ManiHie {
        naw: mani.qeema.naw.ism().to_owned(),
        rutba: mani.qeema.rutba(),
        nitaq: ism_nitaq(mani.qeema.nitaq()).to_owned(),
        nihai: mani.qeema.nihai(),
        arabi: mani.qeema.arabi.clone(),
        injilizi: mani.qeema.injilizi.clone(),
        shawahid: shawahid_hie(&mani.shawahid),
    }
}

fn khatar_hie(khatar: &Musnad<Khatar>) -> KhatarHie {
    KhatarHie {
        naw: khatar.qeema.naw.ism().to_owned(),
        rutba: khatar.qeema.rutba(),
        arabi: khatar.qeema.arabi.clone(),
        injilizi: khatar.qeema.injilizi.clone(),
        muqarr: khatar.qeema.muqarr,
        shawahid: shawahid_hie(&khatar.shawahid),
    }
}

fn hadd_hie(hadd: &Musnad<Hadd>) -> HaddHie {
    HaddHie {
        arabi: hadd.qeema.arabi.clone(),
        injilizi: hadd.qeema.injilizi.clone(),
        shawahid: shawahid_hie(&hadd.shawahid),
    }
}

/// The whole projection, off a built core.
///
/// Split from the command so the projection can be exercised against inputs
/// assembled by hand; `tauri::State` cannot be built outside a running
/// application, and a body written inside a command is a body nothing can drive.
#[must_use]
pub fn aql_hie(aql: &Aql, id: LubaId, ism: &str) -> AqlLubaHie {
    let waad = aql.waad();
    let himaya = aql.hala_himaya();
    let ikhtilaf = aql.ikhtilaf_himaya();
    AqlLubaHie {
        muarrif: id.to_string(),
        ism: ism.to_owned(),
        muntaj: waad.qeema.muntaj.ism().to_owned(),
        tabaqa_raqm: waad.qeema.muntaj.raqm(),
        ism_arabi: waad.qeema.ism_arabi.to_owned(),
        ism_injilizi: waad.qeema.ism_injilizi.to_owned(),
        sabab_arabi: waad.qeema.sabab_arabi.clone(),
        sabab_injilizi: waad.qeema.sabab_injilizi.clone(),
        jahiziya: waad.qeema.jahiziya.ism().to_owned(),
        naqs_arabi: waad.qeema.naqs.as_ref().map(|naqs| naqs.arabi.clone()),
        naqs_injilizi: waad.qeema.naqs.as_ref().map(|naqs| naqs.injilizi.clone()),
        shawahid_muntaj: shawahid_hie(&waad.shawahid),
        mawani: aql.mawani().iter().map(mani_hie).collect(),
        makhatir: aql.makhatir().iter().map(khatar_hie).collect(),
        hudud: aql.hudud().iter().map(hadd_hie).collect(),
        hala_himaya: himaya.qeema.ism().to_owned(),
        shawahid_himaya: shawahid_hie(&himaya.shawahid),
        ikhtilaf_himaya: ikhtilaf.map(|ikhtilaf| IkhtilafHimayaHie {
            min_kashf: ikhtilaf.min_kashf,
            min_iktishaf: ikhtilaf.min_iktishaf,
        }),
        marfuda: aql.marfuda(),
        jahiz_lil_tathbeet: aql.jahiz_lil_tathbeet(),
        jahiz_lil_tashghil: aql.jahiz_lil_tashghil(),
    }
}

/// Everything the core knows about one game, and every answer it gives.
///
/// **This one walks the game directory.** The two hard scans behind
/// `hala_himaya` and the multiplayer risk read the game's files and its store
/// catalogue, and the proxy survey reads the loader directory beside them; on a
/// large installation that is seconds. It is deliberate: `MasahAman::faragh`
/// would hand the core a scan that never ran, and the core would answer
/// `la_tawqee` with nothing behind it — an absence presented as a finding, which
/// is the exact failure this crate exists to prevent. Every surface that asks
/// for it therefore asks on purpose, not on mount.
///
/// # Errors
///
/// [`KhataAqlAmr::LubaBilaMasdar`] when the game's record carries no launcher
/// identity, and whatever the identity parse, the store, the probe or the Steam
/// lookup raise. Neither scan itself fails: a place that cannot be read becomes
/// a gap in the chain rather than an error out of here.
#[tauri::command]
#[specta::specta]
pub fn aql_luba(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, std::sync::Arc<MakhzanIdadat>>,
) -> Result<AqlLubaHie, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let ism = luba.ism.clone();
    let mudkhalat = ijma_mudkhalat(&masarat, &makhzan, &idadat, id, &luba)?;
    Ok(aql_hie(&Aql::jadeed(mudkhalat), id, &ism))
}

/// Assembles every producer's answer for one game.
///
/// Three inputs are deliberately not filled and each is named where it is left
/// out: the registry's build match needs the network, the acknowledgements for
/// the four per-install risks have no record on disk, and the component the
/// store is asked about is the framework table's rather than the plan's.
fn ijma_mudkhalat(
    masarat: &Masarat,
    makhzan: &Makhzan,
    idadat: &MakhzanIdadat,
    id: LubaId,
    luba: &Luba,
) -> Natija<MudkhalatAql> {
    let hali = idadat.hali();
    let simat = simat_luba(makhzan, id)?;
    let taqreer = taqreer_luba(makhzan, luba, &simat, false)?;
    let jidhr_steam = jidhr_steam(masarat, &hali)?;

    let asma = mukawwinat_matluba(&taqreer, luba);
    let asma: Vec<&str> = asma.iter().map(String::as_str).collect();
    let jidhr_makhzan = masarat.mukawwinat();

    let huwiya = huwiyat_luba(id, luba, &simat, jidhr_steam)?;
    let fahs = HalatFahs::mafhusa(taqreer);
    let mut mudkhalat = MudkhalatAql::ijma(huwiya, fahs, &jidhr_makhzan, &asma);
    if let Some(hukm) = crate::luba_awamir::hukm_mukhazzan(id) {
        mudkhalat = mudkhalat.bi_lugha(hukm);
    }
    Ok(mudkhalat.bi_mawqif(MawqifMustakhdim::min_idadat(
        &hali,
        iqrarat_musajjala(masarat),
    )))
}

/// The game's identity as the core holds it.
///
/// `muktamila` and `hala_matjar` are the launcher's own download state, and the
/// store's [`Luba`] record does not carry either — only the discovery record
/// does, and it is not what survives a scan. They are stated as "on disk and
/// complete" here because the probe above has already refused a game that is
/// not: `taqreer_luba` raises `LubaGhayrMawjuda` for an absent root, so nothing
/// reaches this line without its files being there. The core's own
/// `NawMani::GhayrHadira` blocker is therefore unreachable through this command
/// and would need `muktamila` kept on the stored row to fire.
fn huwiyat_luba(
    id: LubaId,
    luba: &Luba,
    simat: &[taarib_kashf::fahs::SimatLuba],
    jidhr_steam: Option<PathBuf>,
) -> Natija<HuwiyatLuba> {
    let masdar = luba.masadir.first().cloned().ok_or_else(|| {
        Khata::from(KhataAqlAmr::LubaBilaMasdar {
            ism: luba.ism.clone(),
        })
    })?;
    Ok(HuwiyatLuba {
        id,
        ism: luba.ism.clone(),
        masdar,
        jidhr: luba.jidhr.clone(),
        tanfidhi: luba.tanfidhi.clone(),
        jidhr_steam,
        simat: simat.to_vec(),
        muktamila: true,
        hala_matjar: None,
    })
}

/// The framework component this game would need, for the store to be asked
/// about it.
///
/// Evidence only — the core never takes a readiness verdict off the component
/// store — and deliberately incomplete in one named direction. The tier-aware
/// table is `taarib_tathbeet::tarkib::hajat_maa_tabaqa`, which is private, so
/// this asks the public per-engine table instead and skips the overlay tier,
/// where `khutta` returns before any component is required at all. The two
/// disagree for exactly two families: GameMaker and RPG Maker VX Ace, whose
/// answer the tier overrides. For those the store is asked about nothing and the
/// chain is silent, which is the safe direction — a missing component is not
/// claimed where it was not looked for.
fn mukawwinat_matluba(taqreer: &TaqreerImkaniyat, luba: &Luba) -> Vec<String> {
    if taqreer.tabaqa == Tabaqa::TarjamaFawqiya {
        return Vec::new();
    }
    match hajat_itar(&taqreer.muharrik, NizamTashghil::hali(), &luba.beea) {
        HajatItar::Matlub(mukawwin) => vec![mukawwin.ism.clone()],
        HajatItar::LaHaja(_) => Vec::new(),
    }
}

/// The risks the user has actually answered.
///
/// Only the first-run statement has a record. The other four are answered at the
/// door of the action they gate — the install screen's own boxes, the run's own
/// `iqrarShabaka` — and nothing persists them, so the core reports them as still
/// standing. That is the answer that refuses rather than the one that proceeds,
/// which is the direction a consent gate has to fail in.
///
/// A record that will not read is treated as no record, for the same reason:
/// the acknowledgement is the thing being proved, and an unreadable file proves
/// nothing.
fn iqrarat_musajjala(masarat: &Masarat) -> Vec<NawKhatar> {
    let masar = crate::tathbeet_awamir::masar_iqrar(masarat);
    match taarib_aman::iqrar::iqra(&masar) {
        Ok(sijill) if !taarib_aman::iqrar::yahtaj_iqrar(sijill.as_ref()) => {
            vec![NawKhatar::BayanAwwal]
        },
        Ok(_) => Vec::new(),
        Err(sabab) => {
            tracing::warn!(
                masar = %masar.display(),
                sabab = %sabab,
                "the first-run acknowledgement record does not read; treated as unacknowledged"
            );
            Vec::new()
        },
    }
}

/// Failures of the core's own surface.
#[derive(Debug, thiserror::Error)]
pub enum KhataAqlAmr {
    /// The game's record carries no launcher identity, so the core cannot be
    /// told which store the game belongs to.
    #[error("{ism} has no launcher identity, so no store catalogue can be consulted for it")]
    LubaBilaMasdar {
        /// The game's display name.
        ism: String,
    },
}

impl Tafsir for KhataAqlAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::LubaBilaMasdar { .. } => 140,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Nothing was scanned and nothing was decided; the library row is
            // incomplete and a rescan writes it again.
            Self::LubaBilaMasdar { .. } => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::LubaBilaMasdar { .. } => {
                "سجلّ هذه اللعبة لا يحمل هوية منصّة، ولا يمكن سؤال فهرس المتجر عنها. أعد فحص \
                 المكتبة."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::LubaBilaMasdar { .. } => {
                "This game's record carries no launcher identity, so no store catalogue can be \
                 consulted for it. Scan the library again."
                    .to_owned()
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::LubaBilaMasdar { .. } => Khutwa::AadaMuhawala,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::LubaBilaMasdar { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataAqlAmr);

#[cfg(test)]
mod ikhtibarat {
    use taarib_aql::{HalatMakhzan, MasahAman, MasahWukala, MasdarMarifa, Muntaj, NawMani};
    use taarib_kashf::fahs::SimatLuba;
    use taarib_mustalahat::luba::MasdarLuba;

    use super::*;

    /// What every test here answers with, so a fixture failure propagates with
    /// `?`. `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// The wire name of a value, as `serde` would write it.
    fn ism_serde<T: serde::Serialize>(qeema: &T) -> NatijatIkhtibar<String> {
        match serde_json::to_value(qeema)? {
            serde_json::Value::String(ism) => Ok(ism),
            akhar => Err(format!("{akhar:?} is not a string discriminant").into()),
        }
    }

    /// The machine names this module puts on the wire are the serde spellings.
    ///
    /// The projection sends `Muntaj::ism()`, `NawMani::ism()` and
    /// `NawKhatar::ism()` rather than serializing the enums, because the core
    /// does not derive `specta::Type` and this crate may not edit it. That is
    /// only safe while the two spellings agree — otherwise a surface written
    /// against the documented `snake_case` discriminant would silently match
    /// nothing — so it is asserted here for every variant rather than trusted.
    #[test]
    fn al_asma_hiya_asma_serde() -> NatijatIkhtibar {
        for muntaj in [
            Muntaj::Istibdal,
            Muntaj::RasmMubashir,
            Muntaj::TabaqaFawqiya,
            Muntaj::LaShay,
            Muntaj::Majhul,
        ] {
            assert_eq!(ism_serde(&muntaj)?, muntaj.ism(), "Muntaj::{muntaj:?}");
        }
        for naw in NawMani::KUL {
            assert_eq!(ism_serde(&naw)?, naw.ism(), "NawMani::{naw:?}");
        }
        for naw in NawKhatar::KUL {
            assert_eq!(ism_serde(&naw)?, naw.ism(), "NawKhatar::{naw:?}");
        }
        for nitaq in [NitaqMani::Kul, NitaqMani::Tashghil] {
            assert_eq!(ism_serde(&nitaq)?, ism_nitaq(nitaq), "NitaqMani::{nitaq:?}");
        }
        Ok(())
    }

    /// ELDEN RING, on a machine where Steam's own catalogue cannot be found.
    ///
    /// The projection is asserted rather than the core — the core has its own
    /// tests against these games — and what is asserted is exactly what a
    /// surface reads. Two blockers stand at once here: the anti-cheat check
    /// could not be completed, and nobody has examined the game. The one that
    /// reaches a screen with room for a single sentence is the first, at rank 1,
    /// and it is not the one the screen would have picked by asking about the
    /// engine. The product is therefore `la_shay` with no tier number beside it,
    /// and the chain names the producer behind each answer.
    #[test]
    fn eldenring_yasil_ila_alshasha_marfudan() -> NatijatIkhtibar {
        let jidhr = PathBuf::from("/nowhere/ELDEN RING");
        let masdar = MasdarLuba::Steam(1_245_620);
        let id = LubaId::min_masdar(&masdar, "ELDEN RING");
        let huwiya = HuwiyatLuba {
            id,
            ism: "ELDEN RING".to_owned(),
            masdar,
            jidhr: jidhr.clone(),
            tanfidhi: None,
            jidhr_steam: None,
            simat: vec![SimatLuba::HimayaMuhtamala("Easy Anti-Cheat".to_owned())],
            muktamila: true,
            hala_matjar: None,
        };
        // No report at all: the honest state for a game nobody has examined, and
        // the one that proves `LamYufhas` and `Majhul` are different answers
        // from an unrecognised engine at tier three.
        //
        // The safety scan is the real producer over a real (absent) directory
        // for a Steam game with no Steam root, rather than `MasahAman::faragh`.
        // The two are not the same input and the difference is the finding
        // worth keeping: `faragh` reports the catalogue as *never owed*, so the
        // core answers `la_tawqee` — "the scan ran and matched nothing" — about
        // a walk that never happened. Running the producer instead reports the
        // catalogue as owed and unread, which is what a Steam game with no
        // Steam root really is.
        let mudkhalat = MudkhalatAql {
            aman: MasahAman::ifhas(&jidhr, huwiya.appid_steam(), None),
            huwiya,
            fahs: HalatFahs::ghayr_mafhusa(),
            lugha: None,
            wukala: MasahWukala::imsah(&jidhr),
            makhzan: HalatMakhzan::ghayr_mafhus(&jidhr),
            mustawda: None,
            mawqif: MawqifMustakhdim::default(),
        };
        let hie = aql_hie(&Aql::jadeed(mudkhalat), id, "ELDEN RING");

        assert_eq!(
            hie.muntaj,
            Muntaj::LaShay.ism(),
            "a blocked game gets no product"
        );
        assert_eq!(hie.tabaqa_raqm, None, "and it prints no tier number");
        assert!(
            hie.marfuda,
            "a blocker of scope `kul` refuses the game outright"
        );
        assert!(!hie.jahiz_lil_tathbeet);
        assert!(!hie.jahiz_lil_tashghil);

        let awwal = hie.mawani.first().ok_or("a game this shape has blockers")?;
        assert_eq!(
            awwal.naw,
            NawMani::FahsHimayaLamYajri.ism(),
            "a check that did not run outranks a game nobody examined"
        );
        assert_eq!(awwal.nitaq, "kul");
        assert!(!awwal.nihai, "and it is not final — a Steam root fixes it");
        assert!(!awwal.arabi.is_empty() && !awwal.injilizi.is_empty());
        assert!(
            awwal
                .shawahid
                .iter()
                .any(|shahid| shahid.masdar == MasdarMarifa::FahrasMatjar.ism()),
            "every answer names the input behind it"
        );
        assert_eq!(
            hie.sabab_arabi, awwal.arabi,
            "the promise carries the first blocker's own sentence, not a second opinion"
        );
        let lam_yufhas = hie
            .mawani
            .iter()
            .find(|mani| mani.naw == NawMani::LamYufhas.ism())
            .ok_or("the unexamined game is still reported, just not first")?;
        assert!(
            lam_yufhas
                .shawahid
                .iter()
                .any(|shahid| shahid.masdar == MasdarMarifa::Bitaqa.ism())
        );

        // The catalogue was owed and never read, so its silence is not reported
        // as a clean result.
        assert_eq!(hie.hala_himaya, "lam_yajri");
        assert!(!hie.shawahid_himaya.is_empty());
        Ok(())
    }

    /// A game nobody has examined answers `majhul`, not "unknown engine, tier 3".
    ///
    /// The one blocker standing is `LamYufhas`, which is the single case the
    /// core turns into a product of `Majhul` rather than `LaShay` — a game with
    /// no reason yet, as against a game with a final one. A library row that
    /// collapsed the two is the defect this distinction exists for.
    #[test]
    fn lam_yufhas_wahdah_yuti_majhul() {
        let jidhr = PathBuf::from("/nowhere/luba-gog");
        // Not a Steam game: no catalogue is owed, so nothing outranks the fact
        // that this game has never been looked at.
        let masdar = MasdarLuba::Gog(1);
        let id = LubaId::min_masdar(&masdar, "luba");
        let huwiya = HuwiyatLuba {
            id,
            ism: "luba".to_owned(),
            masdar,
            jidhr: jidhr.clone(),
            tanfidhi: None,
            jidhr_steam: None,
            simat: Vec::new(),
            muktamila: true,
            hala_matjar: None,
        };
        let mudkhalat = MudkhalatAql {
            aman: MasahAman::ifhas(&jidhr, huwiya.appid_steam(), None),
            huwiya,
            fahs: HalatFahs::ghayr_mafhusa(),
            lugha: None,
            wukala: MasahWukala::imsah(&jidhr),
            makhzan: HalatMakhzan::ghayr_mafhus(&jidhr),
            mustawda: None,
            mawqif: MawqifMustakhdim::default(),
        };
        let hie = aql_hie(&Aql::jadeed(mudkhalat), id, "luba");

        assert_eq!(hie.muntaj, Muntaj::Majhul.ism());
        assert_eq!(hie.tabaqa_raqm, None);
        assert_eq!(
            hie.mawani.first().map(|mani| mani.naw.as_str()),
            Some(NawMani::LamYufhas.ism())
        );
    }

    /// The blockers reach a surface already sorted, so no surface re-ranks them.
    ///
    /// This is the property the whole core exists for: `hukm_tilqai` and
    /// `ibda_tilqai` once named different refusals for one game because each
    /// applied its own order.
    #[test]
    fn al_mawani_tasil_murattaba() {
        let jidhr = PathBuf::from("/nowhere/luba");
        let masdar = MasdarLuba::Steam(1);
        let id = LubaId::min_masdar(&masdar, "luba");
        let huwiya = HuwiyatLuba {
            id,
            ism: "luba".to_owned(),
            masdar,
            jidhr: jidhr.clone(),
            tanfidhi: None,
            jidhr_steam: None,
            // Two blockers at once, pushed in the wrong order on purpose.
            simat: vec![
                SimatLuba::MuhakatRum("SNES".to_owned()),
                SimatLuba::LaysatLuba("Tool".to_owned()),
            ],
            muktamila: true,
            hala_matjar: None,
        };
        let mudkhalat = MudkhalatAql {
            huwiya,
            fahs: HalatFahs::ghayr_mafhusa(),
            aman: MasahAman::faragh(&jidhr),
            lugha: None,
            wukala: MasahWukala::imsah(&jidhr),
            makhzan: HalatMakhzan::ghayr_mafhus(&jidhr),
            mustawda: None,
            mawqif: MawqifMustakhdim::default(),
        };
        let hie = aql_hie(&Aql::jadeed(mudkhalat), id, "luba");
        let rutab: Vec<u8> = hie.mawani.iter().map(|mani| mani.rutba).collect();
        let mut murattaba = rutab.clone();
        murattaba.sort_unstable();
        assert_eq!(rutab, murattaba, "the wire order is the core's order");
        assert_eq!(
            hie.mawani.first().map(|mani| mani.naw.as_str()),
            Some(NawMani::LaysatLuba.ism()),
            "not-a-game outranks an emulated title, whatever order they were seen in"
        );
    }
}
