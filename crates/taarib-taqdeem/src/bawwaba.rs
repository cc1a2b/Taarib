//! البوّابة — the pre-flight gate, and the proof the submit path cannot forge.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use taarib_aman::kharijiya::{idhn_katabi, mira_masmuha};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::khariji::RuqaaKharijiya;
use taarib_mustalahat::luba::HukmLughaRasmiya;
use taarib_mustalahat::nass::{MudkhalNass, NassId};
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_tarqee::bawwaba::ShahadatBawwaba;
use taarib_tarqee::fuhusat::{self, AQSA_AMTHILA, FashalTakhtit, IjtiyazFuhus, MudkhalatFahs};
use taarib_tarqee::khata::KhataTarqee;
use taarib_tarqee::taghtiya_ruqaa::{HADD_AWWAL_LILNASHR, HADD_TAGHTIYA_LILNASHR};
use taarib_tarqee::taqrir_tajawuz::TaqrirTajawuz;
use taarib_usus::Tafsir as _;

use crate::khata::{KhataTaqdeem, NatijatTaqdeem};
use crate::musawwada::Musawwada;

/// The by-string coverage floor, taken from the compiler's own publishable
/// threshold so this gate never holds a second number.
pub const HADD_TAGHTIYA: f64 = HADD_TAGHTIYA_LILNASHR;

/// The opening-session floor, from the same source and for the same reason.
pub const HADD_AWWAL: f64 = HADD_AWWAL_LILNASHR;

/// How many strings needing a rewrite before the overflow warning fires.
///
/// Twenty: below that a contributor rewrites them faster than reading a warning.
pub const HADD_TAJAWUZ: u32 = 20;

/// The untranslated share, in per cent, above which the remainder warning fires.
///
/// Twenty-five: a quarter of a game left in its own language is worth saying so.
pub const NISBAT_MUTABAQQI: u32 = 25;

/// How many offending strings one checklist row links to before it stops.
///
/// Five hundred: enough to open a filtered view, short of a second string table.
pub const AQSA_MARBUT: usize = 500;

/// A check that blocks submission until it passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FahsHasim {
    /// The hard checks a package cannot be produced past.
    Fuhusat,
    /// The asset gate's certificate, verified and never recomputed.
    Shahada,
    /// Coverage against the publishable floor.
    Taghtiya,
    /// The contributor's own published patch for the same build.
    Takrar,
    /// Import mappings nobody has decided on.
    IrtibatIstirad,
    /// An outside translation with no established right to redistribute it.
    HuquqAlmasdar,
    /// The game's publisher already ships Arabic.
    ///
    /// Last in the enum and **first** in [`Self::KULL`], because it is the one
    /// check whose answer makes every other one pointless: a patch for a game
    /// that is already in Arabic does not become publishable by reaching
    /// ninety percent coverage, and a contributor who reads the checklist
    /// top-down should meet this before they start fixing anything else.
    LughaRasmiya,
}

impl FahsHasim {
    /// Every blocking check, in the order the checklist shows them.
    pub const KULL: [Self; 7] = [
        Self::LughaRasmiya,
        Self::Fuhusat,
        Self::Shahada,
        Self::Taghtiya,
        Self::Takrar,
        Self::IrtibatIstirad,
        Self::HuquqAlmasdar,
    ];

    /// The row's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Fuhusat => "الفحوصات الصارمة",
            Self::Shahada => "شهادة البوّابة",
            Self::Taghtiya => "التغطية",
            Self::Takrar => "تكرار رقعة منشورة",
            Self::IrtibatIstirad => "ارتباطات الاستيراد",
            Self::HuquqAlmasdar => "حقوق المصدر المستورَد",
            Self::LughaRasmiya => "لغة رسمية من الناشر",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Fuhusat => "The hard checks",
            Self::Shahada => "The asset gate certificate",
            Self::Taghtiya => "Coverage",
            Self::Takrar => "Duplicate of a published patch",
            Self::IrtibatIstirad => "Import mappings",
            Self::HuquqAlmasdar => "Rights to the imported source",
            Self::LughaRasmiya => "Official Arabic from the publisher",
        }
    }
}

/// A warning that does not block, and that the gate will not mint past until it
/// is acknowledged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tahdheer {
    /// Many strings overrun the room they have badly enough to rewrite.
    TajawuzKatheer,
    /// A large part of the game is still in its original language.
    MutabaqqiKabeer,
    /// The declared method is machine translation with no human review.
    TareeqaAaliya,
    /// Terminology disagrees with the project's glossary.
    MustalahMukhtalif,
}

impl Tahdheer {
    /// Every warning, in the order the checklist shows them.
    pub const KULL: [Self; 4] = [
        Self::TajawuzKatheer,
        Self::MutabaqqiKabeer,
        Self::TareeqaAaliya,
        Self::MustalahMukhtalif,
    ];

    /// The row's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::TajawuzKatheer => "تجاوزات كثيرة",
            Self::MutabaqqiKabeer => "متبقٍّ كبير دون ترجمة",
            Self::TareeqaAaliya => "ترجمة آلية دون مراجعة",
            Self::MustalahMukhtalif => "مصطلحات تخالف المسرد",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::TajawuzKatheer => "A high overflow count",
            Self::MutabaqqiKabeer => "A large untranslated remainder",
            Self::TareeqaAaliya => "Machine translation with no review",
            Self::MustalahMukhtalif => "Terminology outside the glossary",
        }
    }
}

/// Which warnings the contributor has acknowledged, one at a time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Iqrarat {
    muqarra: BTreeSet<Tahdheer>,
}

impl Iqrarat {
    /// Nothing acknowledged.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Acknowledges one warning.
    pub fn aqirr(&mut self, tahdheer: Tahdheer) {
        let _ = self.muqarra.insert(tahdheer);
    }

    /// Withdraws one acknowledgement.
    pub fn asqit(&mut self, tahdheer: Tahdheer) {
        let _ = self.muqarra.remove(&tahdheer);
    }

    /// Whether one warning is acknowledged.
    #[must_use]
    pub fn muqarr(&self, tahdheer: Tahdheer) -> bool {
        self.muqarra.contains(&tahdheer)
    }

    /// How many warnings are acknowledged.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.muqarra.len()
    }
}

/// Which check a checklist row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandFahs {
    /// A blocking check.
    Hasim(FahsHasim),
    /// A warning.
    Tahdheer(Tahdheer),
}

impl BandFahs {
    /// The row's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Hasim(fahs) => fahs.wasf_arabi(),
            Self::Tahdheer(tahdheer) => tahdheer.wasf_arabi(),
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Hasim(fahs) => fahs.wasf_injilizi(),
            Self::Tahdheer(tahdheer) => tahdheer.wasf_injilizi(),
        }
    }
}

/// What one row concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalatBand {
    /// Passed, or did not fire.
    Ijtaz,
    /// Failed, and the submission cannot be sent.
    Rasab,
    /// Fired, and nobody has acknowledged it.
    YantazirIqrar,
    /// Fired, and the contributor acknowledged it.
    Muqarr,
}

impl HalatBand {
    /// Whether this row keeps the gate shut.
    #[must_use]
    pub const fn yamnaa(self) -> bool {
        matches!(self, Self::Rasab | Self::YantazirIqrar)
    }

    /// The row's state, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Ijtaz => "اجتاز",
            Self::Rasab => "أخفق",
            Self::YantazirIqrar => "ينتظر إقرارًا",
            Self::Muqarr => "مُقَرّ به",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Ijtaz => "passed",
            Self::Rasab => "failed",
            Self::YantazirIqrar => "waiting for an acknowledgement",
            Self::Muqarr => "acknowledged",
        }
    }
}

/// One row of the checklist: a check, its verdict, and the strings it points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SatrFahs {
    /// Which check.
    pub band: BandFahs,
    /// What it concluded.
    pub hala: HalatBand,
    /// The strings it links to, at most [`AQSA_MARBUT`] of them.
    pub nusus: Vec<NassId>,
    /// How many strings it found, which may exceed the number it links to.
    pub adad: usize,
    /// The sentence beside the row, in Arabic.
    pub tafsil_arabi: String,
    /// The same, in English.
    pub tafsil_injilizi: String,
}

/// Every check, with its verdict and its strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QaimatFahs {
    sutur: Vec<SatrFahs>,
}

impl QaimatFahs {
    /// Every row, blocking checks first.
    #[must_use]
    pub fn sutur(&self) -> &[SatrFahs] {
        &self.sutur
    }

    /// One row.
    #[must_use]
    pub fn satr(&self, band: BandFahs) -> Option<&SatrFahs> {
        self.sutur.iter().find(|satr| satr.band == band)
    }

    /// Every failing row.
    #[must_use]
    pub fn rasiba(&self) -> Vec<&SatrFahs> {
        self.sutur
            .iter()
            .filter(|satr| satr.hala == HalatBand::Rasab)
            .collect()
    }

    /// Every warning still waiting for an acknowledgement.
    #[must_use]
    pub fn tantazir_iqrar(&self) -> Vec<&SatrFahs> {
        self.sutur
            .iter()
            .filter(|satr| satr.hala == HalatBand::YantazirIqrar)
            .collect()
    }

    /// Whether nothing fails and nothing is left unacknowledged.
    #[must_use]
    pub fn jahiza(&self) -> bool {
        !self.sutur.iter().any(|satr| satr.hala.yamnaa())
    }

    /// Every string any row points at, once each.
    #[must_use]
    pub fn nusus_muashshara(&self) -> Vec<NassId> {
        let farida: BTreeSet<NassId> = self
            .sutur
            .iter()
            .flat_map(|satr| satr.nusus.iter().copied())
            .collect();
        farida.into_iter().collect()
    }

    /// The sentence above the checklist, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        format!(
            "{} فحصًا لازمًا لم يُجتَز، و{} تنبيهًا ينتظر إقرارك.",
            self.rasiba().len(),
            self.tantazir_iqrar().len()
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        format!(
            "{} blocking check(s) have not passed, and {} warning(s) are waiting for your \
             acknowledgement.",
            self.rasiba().len(),
            self.tantazir_iqrar().len()
        )
    }
}

/// Proof that every blocking check passed and every warning was acknowledged.
///
/// **No public constructor, no public fields, no `Deserialize`, not `Clone`.**
/// [`ijri`] is the only place one is made, and it owns the [`IjtiyazFuhus`] that
/// run consumed, so the hard checks provably ran over this exact submission.
#[derive(Debug)]
pub struct IjtiyazTaqdeem {
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    basmat_huzma: Basma,
    fuhus: IjtiyazFuhus,
    qaima: QaimatFahs,
}

impl IjtiyazTaqdeem {
    /// The patch lineage this proof was minted for.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The revision it was minted for.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// The package fingerprint it was minted against.
    #[must_use]
    pub const fn basmat_huzma(&self) -> Basma {
        self.basmat_huzma
    }

    /// The hard checks' own proof, consumed by this one.
    #[must_use]
    pub const fn fuhus(&self) -> &IjtiyazFuhus {
        &self.fuhus
    }

    /// The checklist as it stood when the proof was minted.
    #[must_use]
    pub const fn qaima(&self) -> &QaimatFahs {
        &self.qaima
    }

    /// Whether this proof was minted for exactly this submission and revision.
    #[must_use]
    pub fn yakhuss(&self, musawwada: &Musawwada) -> bool {
        self.ruqaa == musawwada.id()
            && self.murajaa == musawwada.murajaa()
            && self.basmat_huzma == musawwada.basmat_huzma()
    }
}

/// Everything the gate reads, all of it supplied rather than fetched.
#[derive(Debug, Clone, Copy)]
pub struct MudkhalatBawwaba<'a> {
    /// The submission being checked.
    pub musawwada: &'a Musawwada,
    /// The project's strings, as the compile saw them.
    pub madakhil: &'a [MudkhalNass],
    /// Strings the precomputation stage could not lay out.
    pub takhtitat_fashila: &'a [FashalTakhtit],
    /// Everything this contributor has already published.
    pub manshurat: &'a [MulakhkhasRuqaa],
    /// Strings whose terminology disagrees with the project's glossary.
    pub mukhalafat_mustalah: &'a [NassId],
    /// Whether the game's publisher already ships Arabic, when that was
    /// established.
    ///
    /// Supplied rather than computed, like everything else here: the verdict
    /// needs the game's install root, the launcher's declared languages and a
    /// reader for the engine's own containers, and this crate has none of the
    /// three. [`None`] means nobody asked — a headless publish, a game that is
    /// no longer installed — and the row passes saying so, because refusing a
    /// finished submission on a check that did not run would be the gate
    /// asserting something it did not observe.
    pub lugha_rasmiya: Option<&'a HukmLughaRasmiya>,
}

impl<'a> MudkhalatBawwaba<'a> {
    const fn mudkhalat_fahs(&self) -> MudkhalatFahs<'a> {
        let musawwada: &'a Musawwada = self.musawwada;
        MudkhalatFahs {
            madakhil: self.madakhil,
            wasf: musawwada.wasf(),
            takhtitat_fashila: self.takhtitat_fashila,
        }
    }
}

/// Runs every check and renders the checklist, minting nothing.
#[must_use]
pub fn ifhas(mudkhalat: &MudkhalatBawwaba<'_>, iqrarat: &Iqrarat) -> QaimatFahs {
    match fuhusat::ijri(&mudkhalat.mudkhalat_fahs()) {
        Ok(_) => ijmaa(mudkhalat, iqrarat, None),
        Err(khata) => ijmaa(mudkhalat, iqrarat, Some(&khata)),
    }
}

/// Runs every check and mints the proof only when nothing blocks and nothing is
/// left unacknowledged.
///
/// # Errors
///
/// [`KhataTaqdeem::Mukarrar`] when the sole failure is the contributor's own
/// published patch for the same build, [`KhataTaqdeem::BawwabaMaghlaqa`] when
/// any other blocking check failed, and [`KhataTaqdeem::TahdheerBilaIqrar`]
/// when only warnings are outstanding.
pub fn ijri(mudkhalat: &MudkhalatBawwaba<'_>, iqrarat: &Iqrarat) -> NatijatTaqdeem<IjtiyazTaqdeem> {
    match fuhusat::ijri(&mudkhalat.mudkhalat_fahs()) {
        Ok(fuhus) => {
            let qaima = ijmaa(mudkhalat, iqrarat, None);
            if !qaima.jahiza() {
                return Err(khata_qaima(&qaima, mudkhalat));
            }
            let musawwada = mudkhalat.musawwada;
            Ok(IjtiyazTaqdeem {
                ruqaa: musawwada.id(),
                murajaa: musawwada.murajaa(),
                basmat_huzma: musawwada.basmat_huzma(),
                fuhus,
                qaima,
            })
        },
        Err(khata) => {
            let qaima = ijmaa(mudkhalat, iqrarat, Some(&khata));
            Err(khata_qaima(&qaima, mudkhalat))
        },
    }
}

fn ijmaa(
    mudkhalat: &MudkhalatBawwaba<'_>,
    iqrarat: &Iqrarat,
    khata: Option<&KhataTarqee>,
) -> QaimatFahs {
    let musawwada = mudkhalat.musawwada;
    let taghtiya = musawwada.taghtiya();
    let (ghayr_mutarjama, adad_ghayr) = nusus_ghayr_mutarjama(mudkhalat.madakhil);
    let siaa = FahsHasim::KULL.len().saturating_add(Tahdheer::KULL.len());
    let mut sutur: Vec<SatrFahs> = Vec::with_capacity(siaa);

    sutur.push(satr_lugha_rasmiya(mudkhalat.lugha_rasmiya));
    sutur.push(satr_fuhusat(mudkhalat, khata));
    sutur.push(satr_shahada(musawwada));
    sutur.push(satr_taghtiya(
        musawwada,
        ghayr_mutarjama.clone(),
        adad_ghayr,
    ));
    sutur.push(satr_takrar(mudkhalat));
    sutur.push(satr_irtibat(musawwada));
    sutur.push(satr_huquq_masdar(musawwada));

    let (nusus_tajawuz, adad_tajawuz) = nusus_yastahiqq_iaada(musawwada.tajawuz());
    let yastahiqq = musawwada.tajawuz().mulakhkhas.yastahiqq_iaada();
    sutur.push(satr_tahdheer(
        Tahdheer::TajawuzKatheer,
        yastahiqq >= HADD_TAJAWUZ,
        iqrarat,
        nusus_tajawuz,
        adad_tajawuz,
        format!("{yastahiqq} عبارة تستحقّ إعادة الصياغة قبل الإرسال."),
        format!("{yastahiqq} string(s) overrun badly enough to be worth rewriting."),
    ));

    let kabeer = u64::from(taghtiya.mutabaqqi()).saturating_mul(100)
        >= u64::from(taghtiya.majmu).saturating_mul(u64::from(NISBAT_MUTABAQQI));
    sutur.push(satr_tahdheer(
        Tahdheer::MutabaqqiKabeer,
        taghtiya.majmu > 0 && kabeer,
        iqrarat,
        ghayr_mutarjama,
        adad_ghayr,
        format!("{} عبارة ما زالت دون ترجمة.", taghtiya.mutabaqqi()),
        format!("{} string(s) are still untranslated.", taghtiya.mutabaqqi()),
    ));

    let tareeqa = musawwada.tareeqa();
    sutur.push(satr_tahdheer(
        Tahdheer::TareeqaAaliya,
        tareeqa.yahtaj_iqrar(),
        iqrarat,
        Vec::new(),
        0,
        format!("الطريقة المعلنة: {}.", tareeqa.wasf_arabi()),
        format!("The declared method is: {}.", tareeqa.wasf_injilizi()),
    ));

    let mukhalafat = mudkhalat.mukhalafat_mustalah;
    sutur.push(satr_tahdheer(
        Tahdheer::MustalahMukhtalif,
        !mukhalafat.is_empty(),
        iqrarat,
        mukhalafat.iter().copied().take(AQSA_MARBUT).collect(),
        mukhalafat.len(),
        format!("{} عبارة تخالف مسرد المشروع.", mukhalafat.len()),
        format!(
            "{} string(s) disagree with the project glossary.",
            mukhalafat.len()
        ),
    ));

    QaimatFahs { sutur }
}

/// Reads the official-Arabic verdict into a blocking row.
///
/// The bar is [`HukmLughaRasmiya::yatakallam_arabi`] and not "fully Arabic". A
/// game whose menus ship in Arabic and whose subtitles do not is still a game
/// somebody paid translators for, and a patch that rewrites the menus on its
/// way to the subtitles overwrites the half that was already right. `Mubhama`
/// is refused on the same terms: Arabic is shipped and only its extent is
/// unknown, and publishing over it on the strength of not knowing how much
/// would be the gate guessing in the one direction a player cannot undo.
///
/// The user-facing setting that reveals such a game in their own library does
/// not reach here, deliberately. That setting is one person's judgement about
/// one installation of one game; publishing is a judgement about everybody's.
fn satr_lugha_rasmiya(hukm: Option<&HukmLughaRasmiya>) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::LughaRasmiya);
    let Some(hukm) = hukm else {
        return satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            "لم يُفحص ما إذا كان ناشر اللعبة يشحن عربية رسمية، فلم يُبنَ على هذا الفحص شيء."
                .to_owned(),
            "Whether this game's publisher already ships Arabic was not checked, so nothing \
             here rests on it."
                .to_owned(),
        );
    };
    let hala = hukm.hala();
    if !hukm.yatakallam_arabi() {
        return satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            format!("{} (ثقة {}٪).", hala.ism_arabi(), hukm.thiqa),
            format!("{} ({}% confidence).", hala.ism_injilizi(), hukm.thiqa),
        );
    }
    // The heaviest observation, which is the first: `ihsib` sorts the evidence
    // by weight before it builds the verdict, so a contributor reading this row
    // is reading the strongest reason and not an arbitrary one.
    let daleel = hukm
        .dalail
        .first()
        .map_or_else(String::new, |awwal| format!(" {}", awwal.wasf));
    satr(
        band,
        HalatBand::Rasab,
        Vec::new(),
        0,
        format!(
            "ناشر هذه اللعبة يشحن عربية بالفعل: {} (ثقة {}٪). ترجمة آلية فوق ترجمة بشرية مدفوعة \
             ومراجَعة ليست إضافة، ولا يقبل المستودع رقعة نصّية لها.",
            hala.ism_arabi(),
            hukm.thiqa
        ),
        format!(
            "This game's publisher already ships Arabic: {} ({}% confidence).{daleel} Machine \
             output over paid, reviewed human translation is not an addition, and the registry \
             does not accept a text patch for it.",
            hala.ism_injilizi(),
            hukm.thiqa
        ),
    )
}

fn satr_fuhusat(mudkhalat: &MudkhalatBawwaba<'_>, khata: Option<&KhataTarqee>) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::Fuhusat);
    match khata {
        None => satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            format!(
                "اجتازت الفحوصات الصارمة على {} عبارة.",
                mudkhalat.madakhil.len()
            ),
            format!(
                "The hard checks passed over {} string(s).",
                mudkhalat.madakhil.len()
            ),
        ),
        Some(khata) => {
            let nusus = nusus_min_khata(khata, mudkhalat.madakhil);
            let adad = nusus.len();
            satr(
                band,
                HalatBand::Rasab,
                nusus,
                adad,
                khata.arabi(),
                khata.injilizi(),
            )
        },
    }
}

/// Reads the asset gate's certificate into a blocking row.
///
/// Takes the submission rather than the certificate, and that is the whole
/// reason for the signature. The sentence this row renders — the package
/// carries no byte of the game — is a count over the permitted content of a
/// package Taarib compiled, and it is true of exactly that and nothing else. A
/// third-party entry is the opposite kind of thing: an archive built out of the
/// game's own containers by somebody else, which Taarib fetches and never
/// compiled, and a mirrored one puts those bytes on Taarib's own
/// infrastructure. Handed a bare [`ShahadatBawwaba`], any caller holding one
/// could render that sentence over whatever they happened to be looking at.
/// Taking a [`Musawwada`] means the row can only be built over the thing the
/// certificate was computed from, and a [`RuqaaKharijiya`] has no path to one —
/// which is why [`ifhas_khariji`] has no certificate row at all rather than a
/// certificate row that says nothing.
fn satr_shahada(musawwada: &Musawwada) -> SatrFahs {
    let shahada = musawwada.shahada();
    let band = BandFahs::Hasim(FahsHasim::Shahada);
    match khalal_shahada(shahada) {
        Some((arabi, injilizi)) => satr(band, HalatBand::Rasab, Vec::new(), 0, arabi, injilizi),
        None => satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            format!(
                "لا تحمل الحزمة أيّ بايت من اللعبة: {} عبارة، و{} تخطيطًا، و{} صفحة، و{} فرقًا.",
                shahada.nusus, shahada.takhtitat, shahada.safahat, shahada.furuq
            ),
            shahada.wasf(),
        ),
    }
}

fn satr_taghtiya(musawwada: &Musawwada, nusus: Vec<NassId>, adad: usize) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::Taghtiya);
    let taghtiya = musawwada.taghtiya();
    if taghtiya.qabila_lil_nashr() {
        return satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            taghtiya.wasf_arabi(),
            taghtiya.wasf_injilizi(),
        );
    }
    satr(
        band,
        HalatBand::Rasab,
        nusus,
        adad,
        format!(
            "{} الحدّ الأدنى للنشر {:.0}٪ من النصوص و{:.0}٪ مما يظهر في أول ساعة.",
            taghtiya.wasf_arabi(),
            HADD_TAGHTIYA * 100.0,
            HADD_AWWAL * 100.0
        ),
        format!(
            "{} The publishable floor is {:.0}% of strings and {:.0}% of the opening session.",
            taghtiya.wasf_injilizi(),
            HADD_TAGHTIYA * 100.0,
            HADD_AWWAL * 100.0
        ),
    )
}

fn satr_takrar(mudkhalat: &MudkhalatBawwaba<'_>) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::Takrar);
    match mukarrar(mudkhalat) {
        Some(manshura) => satr(
            band,
            HalatBand::Rasab,
            Vec::new(),
            0,
            format!(
                "لديك رقعة منشورة تغطّي البناء نفسه: {} {}.",
                manshura.unwan, manshura.murajaa
            ),
            format!(
                "You have already published {} {}, which covers one of these builds.",
                manshura.unwan, manshura.murajaa
            ),
        ),
        None => satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            "لا رقعة منشورة لك تغطّي هذه الأبنية.".to_owned(),
            "No patch you have published covers these builds.".to_owned(),
        ),
    }
}

/// Whether every outside translation this submission used may be redistributed.
///
/// Blocking, and deliberately so. A patch built from somebody else's work is
/// published to strangers under the owner's signing key, and the moment to
/// establish that there was a right to do that is before the key is used, not
/// after the author finds their translation in a catalogue they were never
/// asked about. An import that names no author at all fails this too: a file
/// that came from somewhere with nobody recorded is exactly the case this
/// exists for. Naming them is not the same as being allowed to publish them.
fn satr_huquq_masdar(musawwada: &Musawwada) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::HuquqAlmasdar);
    let bila_haqq = musawwada.masadir_bila_haqq();
    if bila_haqq.is_empty() {
        let adad = musawwada.masadir_kharijiya().len();
        let (arabi, injilizi) = if adad == 0 {
            (
                "لا ترجمة مستورَدة من خارج تعريب.".to_owned(),
                "Nothing was imported from outside Taarib.".to_owned(),
            )
        } else {
            (
                format!("{adad} مصدرًا مستورَدًا، ولكلٍّ حقُّ إعادة نشرٍ مُثبَت."),
                format!("{adad} imported source(s), each with an established right to republish."),
            )
        };
        return satr(band, HalatBand::Ijtaz, Vec::new(), 0, arabi, injilizi);
    }
    let asmaa: Vec<String> = bila_haqq
        .iter()
        .map(|sijill| {
            sijill.masdar_khariji.as_ref().map_or_else(
                || sijill.masdar.display().to_string(),
                |masdar| masdar.ism.clone(),
            )
        })
        .collect();
    let qaima = asmaa.join("، ");
    let qaima_injilizi = asmaa.join(", ");
    satr(
        band,
        HalatBand::Rasab,
        Vec::new(),
        bila_haqq.len(),
        format!(
            "لا يثبت حقُّ إعادة نشر ما استُورد من: {qaima}. سجّل رخصةً تسمح بإعادة النشر، أو \
             إذنًا مكتوبًا من صاحب العمل، وإلّا فاحذف المستورَد قبل التقديم."
        ),
        format!(
            "No right to republish what was imported from: {qaima_injilizi}. Record a licence \
             that permits redistribution, or written permission from whoever made it, or remove \
             the imported text before submitting."
        ),
    )
}

/// Whether one third-party entry may be published at all.
///
/// [`FahsHasim::HuquqAlmasdar`] again, deliberately: a row that answered the
/// rights question differently depending on which screen asked would be two
/// answers to one question. What differs is the subject.
/// [`satr_huquq_masdar`] asks about *strings* a contributor imported into a
/// patch Taarib compiled; this asks about a whole finished work republished
/// under its author's name, fetched and installed by strangers on the
/// registry's word — so the bar is the author's own written grant and a licence
/// file nobody here read is not it.
///
/// Both halves are `taarib_aman::kharijiya`'s, which is where the pre-install
/// gate and the installer read them from as well. A second spelling of "may
/// this be published, may this be hosted" is a second place for it to drift,
/// and drift here means publishing somebody's work without having asked.
fn satr_huquq_khariji(madkhal: &RuqaaKharijiya) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::HuquqAlmasdar);
    let nasab = madkhal.nasab();
    if !idhn_katabi(&madkhal.masdar) {
        return satr(
            band,
            HalatBand::Rasab,
            Vec::new(),
            1,
            format!(
                "«{}» لـ{nasab}: عملٌ خارجيّ لا إذن مكتوبًا مسجَّلًا له من صاحبه. المستودع لا \
                 ينشر عمل غيره بلا إذن، والرخصة وحدها لا تكفي هنا: اكتب أين أذِن صاحبُ العمل \
                 ومتى، ليتمكّن القارئ من التحقّق.",
                madkhal.unwan
            ),
            format!(
                "{}, by {nasab}, has no written permission on record. The registry does not \
                 publish somebody else's work without one, and a licence alone is not enough \
                 here: write where the author granted it and when, so a reader can go and check.",
                madkhal.unwan
            ),
        );
    }
    if madkhal.mira.min_sijill() && !mira_masmuha(&madkhal.masdar) {
        return satr(
            band,
            HalatBand::Rasab,
            Vec::new(),
            1,
            format!(
                "«{}» لـ{nasab}: مضبوطٌ على استضافة ملفّاته في المستودع، والإذن المسجَّل لا \
                 يشمل الاستضافة. إعادة النشر والاستضافة إذنان منفصلان، والملفّات تبقى عند \
                 صاحبها حتى يأذن بنقلها.",
                madkhal.unwan
            ),
            format!(
                "{} is set to be served from the registry and the recorded permission does not \
                 cover hosting. Redistribution and hosting are separate grants: the files stay \
                 on {nasab}'s own endpoint until they say otherwise.",
                madkhal.unwan
            ),
        );
    }
    let (min_arabi, min_injilizi) = if madkhal.mira.min_sijill() {
        ("المستودع", "the registry")
    } else {
        ("صاحب العمل", "its author")
    };
    satr(
        band,
        HalatBand::Ijtaz,
        Vec::new(),
        0,
        format!(
            "«{}» لـ{nasab}: إذنٌ مكتوب مسجَّل، والملفّات تُجلَب من {min_arabi}.",
            madkhal.unwan
        ),
        format!(
            "{} by {nasab}: a written permission is on record, and the files are fetched from \
             {min_injilizi}.",
            madkhal.unwan
        ),
    )
}

/// The gate's verdict over one third-party entry.
///
/// One row, and the rows that are missing are as deliberate as the one that is
/// here. Coverage, the hard checks, the duplicate check and the import mappings
/// are all statements about a package Taarib compiled; there is no package
/// here, so there is nothing for them to be true or false of, and emitting them
/// would be the checklist asserting what nobody measured.
///
/// The certificate is the one that matters. It says a package carries zero
/// bytes of the game, and a third-party artifact is the game's own containers
/// repacked by somebody else — the sentence is simply false over one. It is
/// absent here because [`satr_shahada`] takes a [`Musawwada`] and nothing turns
/// a [`RuqaaKharijiya`] into one, so the exclusion holds without anybody
/// remembering it.
#[must_use]
pub fn ifhas_khariji(madkhal: &RuqaaKharijiya) -> QaimatFahs {
    QaimatFahs {
        sutur: vec![satr_huquq_khariji(madkhal)],
    }
}

fn satr_irtibat(musawwada: &Musawwada) -> SatrFahs {
    let band = BandFahs::Hasim(FahsHasim::IrtibatIstirad);
    let muallaqa = musawwada.irtibatat_muallaqa();
    if muallaqa.is_empty() {
        return satr(
            band,
            HalatBand::Ijtaz,
            Vec::new(),
            0,
            "لا ارتباط استيراد معلّقًا.".to_owned(),
            "Every import mapping has been decided.".to_owned(),
        );
    }
    let nusus: Vec<NassId> = muallaqa
        .iter()
        .flat_map(|irtibat| irtibat.murashshahat.iter().copied())
        .take(AQSA_MARBUT)
        .collect();
    satr(
        band,
        HalatBand::Rasab,
        nusus,
        muallaqa.len(),
        format!("{} ارتباط استيراد ينتظر قرارك.", muallaqa.len()),
        format!(
            "{} import mapping(s) are still waiting on you.",
            muallaqa.len()
        ),
    )
}

fn satr_tahdheer(
    tahdheer: Tahdheer,
    waqaa: bool,
    iqrarat: &Iqrarat,
    nusus: Vec<NassId>,
    adad: usize,
    arabi: String,
    injilizi: String,
) -> SatrFahs {
    let band = BandFahs::Tahdheer(tahdheer);
    if !waqaa {
        return satr(band, HalatBand::Ijtaz, Vec::new(), 0, arabi, injilizi);
    }
    let hala = if iqrarat.muqarr(tahdheer) {
        HalatBand::Muqarr
    } else {
        HalatBand::YantazirIqrar
    };
    satr(band, hala, nusus, adad, arabi, injilizi)
}

const fn satr(
    band: BandFahs,
    hala: HalatBand,
    nusus: Vec<NassId>,
    adad: usize,
    tafsil_arabi: String,
    tafsil_injilizi: String,
) -> SatrFahs {
    SatrFahs {
        band,
        hala,
        nusus,
        adad,
        tafsil_arabi,
        tafsil_injilizi,
    }
}

/// The first incoherence in the certificate, or [`None`] when it has none.
fn khalal_shahada(shahada: &ShahadatBawwaba) -> Option<(String, String)> {
    if shahada.bayt_min_alluba != 0 {
        return Some((
            format!(
                "الشهادة تُقرّ بوجود {} بايت من محتوى اللعبة الأصلي داخل الحزمة.",
                shahada.bayt_min_alluba
            ),
            format!(
                "The certificate states {} byte(s) of original game content in the package.",
                shahada.bayt_min_alluba
            ),
        ));
    }
    if shahada.nusus.saturating_add(shahada.furuq) == 0 {
        return Some((
            "الشهادة لا تعدّ أيّ عبارة ولا أيّ فرق حاوية؛ الحزمة لا تحمل ترجمة.".to_owned(),
            "The certificate counts no translated string and no container difference, so the \
             package carries no translation."
                .to_owned(),
        ));
    }
    if shahada.safahat > 0 && shahada.khutut.is_empty() {
        return Some((
            format!(
                "الشهادة تعدّ {} صفحة رسوم دون بصمة خطٍّ واحدة.",
                shahada.safahat
            ),
            format!(
                "The certificate counts {} atlas page(s) and not one font fingerprint.",
                shahada.safahat
            ),
        ));
    }
    None
}

fn mukarrar<'a>(mudkhalat: &MudkhalatBawwaba<'a>) -> Option<&'a MulakhkhasRuqaa> {
    let musawwada = mudkhalat.musawwada;
    let irtibat = musawwada.irtibat();
    let sahib = &musawwada.musahim().musahim;
    mudkhalat.manshurat.iter().find(|manshura| {
        if manshura.id == musawwada.id() || &manshura.musahim != sahib {
            return false;
        }
        manshura
            .bina_manassa
            .iter()
            .any(|bina| irtibat.manassat.contains(bina))
            || manshura
                .basmat
                .iter()
                .any(|basma| irtibat.basmat.contains(basma))
    })
}

fn nusus_ghayr_mutarjama(madakhil: &[MudkhalNass]) -> (Vec<NassId>, usize) {
    let mut nusus: Vec<NassId> = Vec::new();
    let mut adad = 0_usize;
    for madkhal in madakhil {
        if madkhal
            .hadaf
            .as_deref()
            .is_some_and(|hadaf| !hadaf.is_empty())
        {
            continue;
        }
        adad = adad.saturating_add(1);
        if nusus.len() < AQSA_MARBUT {
            nusus.push(madkhal.id);
        }
    }
    (nusus, adad)
}

fn nusus_yastahiqq_iaada(tajawuz: &TaqrirTajawuz) -> (Vec<NassId>, usize) {
    let farida: BTreeSet<NassId> = tajawuz
        .tajawuzat
        .iter()
        .filter(|madkhal| madkhal.shidda.yastahiqq_iaada())
        .map(|madkhal| madkhal.nass)
        .collect();
    let adad = farida.len();
    (farida.into_iter().take(AQSA_MARBUT).collect(), adad)
}

fn nusus_min_khata(khata: &KhataTarqee, madakhil: &[MudkhalNass]) -> Vec<NassId> {
    match khata {
        KhataTarqee::NasqMaksur { amthila, .. }
        | KhataTarqee::IaatimadGhayrMashru { amthila, .. } => nusus_min_amthila(madakhil, amthila),
        KhataTarqee::TakhtitFashil { nass, .. } => nusus_bi_nass(madakhil, nass),
        _ => Vec::new(),
    }
}

fn nusus_min_amthila(madakhil: &[MudkhalNass], amthila: &[String]) -> Vec<NassId> {
    // `fuhusat` writes each example as the string's identity followed by the
    // reason, so the identity is the prefix and nothing here re-runs a check.
    madakhil
        .iter()
        .filter(|madkhal| {
            let id = madkhal.id.to_string();
            amthila.iter().any(|mithal| mithal.starts_with(&id))
        })
        .map(|madkhal| madkhal.id)
        .take(AQSA_MARBUT)
        .collect()
}

fn nusus_bi_nass(madakhil: &[MudkhalNass], nass: &str) -> Vec<NassId> {
    madakhil
        .iter()
        .filter(|madkhal| {
            madkhal
                .hadaf
                .as_deref()
                .is_some_and(|hadaf| hadaf.starts_with(nass))
        })
        .map(|madkhal| madkhal.id)
        .take(AQSA_MARBUT)
        .collect()
}

fn khata_qaima(qaima: &QaimatFahs, mudkhalat: &MudkhalatBawwaba<'_>) -> KhataTaqdeem {
    let rasiba = qaima.rasiba();
    if let [wahid] = rasiba.as_slice()
        && wahid.band == BandFahs::Hasim(FahsHasim::Takrar)
        && let Some(manshura) = mukarrar(mudkhalat)
    {
        return KhataTaqdeem::Mukarrar {
            ruqaa: format!("{} {} ({})", manshura.unwan, manshura.murajaa, manshura.id),
        };
    }
    if rasiba.is_empty() {
        return KhataTaqdeem::TahdheerBilaIqrar {
            adad: qaima.tantazir_iqrar().len(),
        };
    }
    KhataTaqdeem::BawwabaMaghlaqa {
        adad: rasiba.len(),
        amthila: rasiba
            .iter()
            .take(AQSA_AMTHILA)
            .map(|rasib| format!("{}: {}", rasib.band.wasf_injilizi(), rasib.tafsil_injilizi))
            .collect(),
    }
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::khariji::HalatMira;
    use taarib_mustalahat::luba::{DaleelLugha, HalatLughaRasmiya, NawDaleelLugha, TughtiyaLugha};
    use taarib_mustalahat::ruqaa::IdhnMasdar;

    use super::*;

    /// The verdict Taarib really produces for Little Nightmares Enhanced
    /// Edition, transcribed from a run against the installed game: culture `ar`
    /// at 345 entries against a 350-entry reference, plus the store listing.
    fn hukm_lnee() -> HukmLughaRasmiya {
        HukmLughaRasmiya {
            wajiha: TughtiyaLugha::Muakkada,
            nusus: TughtiyaLugha::Muakkada,
            thiqa: 80,
            dalail: vec![DaleelLugha {
                naw: NawDaleelLugha::MawridMuharrik,
                wasf: "Unreal compiles culture `ar` into this game's own localization target \
                       `Game` with 345 entries, 98% of the 350 its reference culture carries, \
                       340 of them in Arabic script"
                    .to_owned(),
                mawqi: Some(
                    "Atlas-WindowsNoEditor.pak!Atlas/Content/Localization/Game/ar/Game.locres"
                        .to_owned(),
                ),
                wazn: 95,
            }],
            majhul: Vec::new(),
            lughat_muallana: vec!["arabic".to_owned(), "english".to_owned()],
            isdar_fahs: 1,
            waqt: "2026-09-04T00:00:00Z".to_owned(),
        }
    }

    fn hukm_bila_arabiya(thiqa: u8, dalail: Vec<DaleelLugha>) -> HukmLughaRasmiya {
        HukmLughaRasmiya {
            wajiha: TughtiyaLugha::Ghaiba,
            nusus: TughtiyaLugha::Ghaiba,
            thiqa,
            dalail,
            majhul: Vec::new(),
            lughat_muallana: Vec::new(),
            isdar_fahs: 1,
            waqt: "2026-09-04T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn lugha_rasmiya_kamila_taghliq_albawwaba() {
        let hukm = hukm_lnee();
        let satr = satr_lugha_rasmiya(Some(&hukm));
        assert_eq!(satr.hala, HalatBand::Rasab);
        assert!(satr.hala.yamnaa());
        assert!(satr.tafsil_injilizi.contains("Full Arabic"));
        assert!(satr.tafsil_injilizi.contains("345 entries"));

        let qaima = QaimatFahs { sutur: vec![satr] };
        assert!(
            !qaima.jahiza(),
            "a submission for a game with official Arabic cannot be sent"
        );
        assert_eq!(qaima.rasiba().len(), 1);
    }

    #[test]
    fn lugha_rasmiya_mubhama_taghliq_albawwaba_aydan() {
        // Arabic is shipped and only its extent is unknown. Publishing over it
        // on the strength of not knowing how much would be a guess in the one
        // direction a player cannot undo.
        let mut hukm = hukm_lnee();
        hukm.nusus = TughtiyaLugha::Majhula;
        assert_eq!(hukm.hala(), HalatLughaRasmiya::Mubhama);
        assert_eq!(satr_lugha_rasmiya(Some(&hukm)).hala, HalatBand::Rasab);
    }

    #[test]
    fn lugha_rasmiya_juziya_taghliq_albawwaba_aydan() {
        // Menus in Arabic and dialogue not. Still somebody's paid work, and a
        // patch that rewrites the menus to reach the dialogue overwrites it.
        let mut hukm = hukm_lnee();
        hukm.nusus = TughtiyaLugha::Ghaiba;
        assert_eq!(hukm.hala(), HalatLughaRasmiya::WajihaFaqat);
        assert_eq!(satr_lugha_rasmiya(Some(&hukm)).hala, HalatBand::Rasab);
    }

    #[test]
    fn ghiyab_alarabiya_yajtaz() {
        let daleel = DaleelLugha {
            naw: NawDaleelLugha::MawridMuharrik,
            wasf: "Unreal compiles 13 culture(s) and none of them is Arabic".to_owned(),
            mawqi: None,
            wazn: 95,
        };
        let satr = satr_lugha_rasmiya(Some(&hukm_bila_arabiya(95, vec![daleel])));
        assert_eq!(satr.hala, HalatBand::Ijtaz);
        assert!(!satr.hala.yamnaa());
    }

    #[test]
    fn ghiyab_ghayr_hasim_yajtaz_kadhalik() {
        // Nothing was observed at all, so the `Ghaib` is the absence of
        // evidence. The gate lets it through: refusing an evening's work on a
        // check that could not run would be the gate asserting what it did not
        // see, which is the same mistake in the other direction.
        let hukm = hukm_bila_arabiya(0, Vec::new());
        assert!(!hukm.hasim());
        assert_eq!(satr_lugha_rasmiya(Some(&hukm)).hala, HalatBand::Ijtaz);
    }

    #[test]
    fn ghiyab_alhukm_yaqul_annahu_lam_yufhas() {
        let satr = satr_lugha_rasmiya(None);
        assert_eq!(satr.hala, HalatBand::Ijtaz);
        assert!(satr.tafsil_injilizi.contains("was not checked"));
    }

    /// The seed as it ships: a written grant whose statement nobody filled in
    /// is not a grant anybody can go and check, so the gate shuts.
    #[test]
    fn khariji_bila_bayan_yughliq_albawwaba() -> Result<(), Box<dyn std::error::Error>> {
        let qaima = ifhas_khariji(&taarib_mustawda::khariji::badhrat_rtea());
        assert!(!qaima.jahiza());
        let satr = qaima
            .satr(BandFahs::Hasim(FahsHasim::HuquqAlmasdar))
            .ok_or("the third-party checklist has no rights row")?;
        assert_eq!(satr.hala, HalatBand::Rasab);
        assert!(
            satr.tafsil_injilizi.contains("no written permission"),
            "{}",
            satr.tafsil_injilizi
        );
        assert!(
            satr.tafsil_injilizi.contains("Emad Adel"),
            "{}",
            satr.tafsil_injilizi
        );
        Ok(())
    }

    #[test]
    fn khariji_bi_bayan_yajtaz() -> Result<(), Box<dyn std::error::Error>> {
        let mut madkhal = taarib_mustawda::khariji::badhrat_rtea();
        madkhal.masdar.idhn = IdhnMasdar::Katabi {
            bayan: "granted by the author on X, 2026-09-21, https://x.example/post/1".to_owned(),
        };
        let qaima = ifhas_khariji(&madkhal);
        assert!(qaima.jahiza());
        let satr = qaima
            .satr(BandFahs::Hasim(FahsHasim::HuquqAlmasdar))
            .ok_or("the third-party checklist has no rights row")?;
        assert_eq!(satr.hala, HalatBand::Ijtaz);
        assert!(satr.tafsil_injilizi.contains("fetched from its author"));
        Ok(())
    }

    /// Redistribution and hosting are separate grants, and the gate takes
    /// `taarib_aman::kharijiya`'s answer on the second one.
    #[test]
    fn mira_bila_idhn_tughliq_albawwaba() -> Result<(), Box<dyn std::error::Error>> {
        let mut madkhal = taarib_mustawda::khariji::badhrat_rtea();
        madkhal.masdar.idhn = IdhnMasdar::Katabi {
            bayan: "granted by the author on X, 2026-09-21, https://x.example/post/1".to_owned(),
        };
        madkhal.mira = HalatMira::MinAlsijill {
            rabt: format!("khariji/{}/update.zip", madkhal.id),
        };
        let qaima = ifhas_khariji(&madkhal);
        assert!(!qaima.jahiza());
        let satr = qaima
            .satr(BandFahs::Hasim(FahsHasim::HuquqAlmasdar))
            .ok_or("the third-party checklist has no rights row")?;
        assert_eq!(satr.hala, HalatBand::Rasab);
        assert!(satr.tafsil_injilizi.contains("separate grants"));
        Ok(())
    }

    /// The certificate says a package holds zero bytes of the game, and a
    /// third-party artifact is the game's own containers repacked. The row is
    /// absent rather than passing, and it is absent because `satr_shahada`
    /// takes a submission and nothing turns one of these into one.
    #[test]
    fn qaimat_alkhariji_la_tahmil_shahadat_albawwaba() {
        let qaima = ifhas_khariji(&taarib_mustawda::khariji::badhrat_rtea());
        assert!(
            qaima.satr(BandFahs::Hasim(FahsHasim::Shahada)).is_none(),
            "a third-party entry has no package for the certificate to be about"
        );
        assert!(qaima.satr(BandFahs::Hasim(FahsHasim::Taghtiya)).is_none());
        assert_eq!(qaima.sutur().len(), 1);
    }

    #[test]
    fn tarteeb_alqaima_yabda_bilugha_alrasmiya() {
        // A contributor reading the checklist top-down meets the one check
        // whose answer makes every other one pointless first.
        assert_eq!(FahsHasim::KULL.first(), Some(&FahsHasim::LughaRasmiya));
        // The count is the guard against a check being added to the enum and
        // left out of the list the checklist actually walks, which would be a
        // blocking check nobody is ever shown.
        assert_eq!(FahsHasim::KULL.len(), 7);
        assert!(FahsHasim::KULL.contains(&FahsHasim::HuquqAlmasdar));
    }
}
