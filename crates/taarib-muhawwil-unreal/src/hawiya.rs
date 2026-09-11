//! الحاوية — the additive container that carries Taarib's Arabic into an Unreal game.
//!
//! Everything the runtime half of this adapter would do inside the process is
//! done here instead, offline, with the game shut down, in the one form the
//! engine already understands: a `.pak` mounted above the game's own. It holds
//! three things, and the engine needs no help reading any of them.
//!
//! 1. **The translation**, as the game's own `.locres` files rewritten. For
//!    every localization target the game ships, every culture's resource is
//!    read out of the game's containers, the entries the patch translates are
//!    replaced in place, and the file goes into the container at the same path.
//!    Every culture rather than one: which culture a player's engine resolves
//!    depends on their operating system and on files this installer never
//!    sees, and a container that overrode only `en` would show Arabic to one
//!    player and English to the next. An `ar` directory is added beside them,
//!    cloned from the native culture, so a system already asking for Arabic
//!    finds it.
//! 2. **The source fingerprints**, untouched. The engine applies a localized
//!    string only when the hash it stores matches the hash of the source text
//!    the package carries — see [`crate::mawarid::locres`] — and a rewritten
//!    entry keeps the fingerprint it had. So what the engine draws is Taarib's
//!    Arabic for exactly the strings the patch translated and the game's own
//!    text for every other one.
//! 3. **The face**, where the engine can be told about one. Slate draws every
//!    glyph a game's own fonts lack from one fallback face, and an engine whose
//!    resources carry [`crate::khatt::MIFTAH_IRTIDA`] resolves that face's
//!    file name through that localized string. The container carries Taarib's
//!    face under the directory the engine reads the name against and rewrites
//!    the string to name it. An engine whose resources carry no such string
//!    builds its fallback as a fixed composite that already holds an Arabic
//!    sub-font, so for it nothing is carried and nothing is named — and
//!    [`HalatKhatt`] says which of the two happened, because a report that
//!    said "font registered" for both would be describing one engine to the
//!    users of the other.
//!
//! Nothing here writes to disk. [`ibni`] returns the [`KatibPak`] and a report,
//! and the installer serialises the container through its own recorder, so it
//! is in the manifest and an uninstall deletes it. The game's own containers are
//! read and never written.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ISM_HAWIYA;
use crate::alam::WASM_ARABI;
use crate::isdar::{AQSA_HAWIYAT, Bina, Tabaa};
use crate::khata::KhataUnreal;
use crate::khatt::{
    ASL_IRTIDA, DALIL_KHUTUT_SLATE, FADAA_IRTIDA, KHATT_MUHARRIK_ARABI, MIFTAH_IRTIDA,
    madkhal_irtida, masar_khatt_slate, sighat_bayt,
};
use crate::mawarid::Mawrid as _;
use crate::mawarid::locmeta::MawridLocmeta;
use crate::mawarid::locres::{MawridLocres, basmat_asl};
use crate::mawarid::pak::{HawiyatPak, KatibPak};

/// The directory every localization target lives under, as Unreal spells it.
pub const DALIL_TADWEEL: &str = "Localization";

/// The most cultures written for one target.
///
/// The most heavily localized title this product has met ships fifteen. A
/// target declaring more than this is a shape this module has not seen, and
/// writing sixty-four rewritten resources into one container is where a patch
/// stops being small.
pub const AQSA_THAQAFAT: usize = 64;

/// The most localization targets read from one game.
pub const AQSA_AHDAF: usize = 16;

/// The engine's own fallback face, whose location inside the game's containers
/// tells this module where the engine reads faces from.
const MALAF_IRTIDA_MUHARRIK: &str = "DroidSansFallback.ttf";

// ---------------------------------------------------------------------------
// What the caller supplies
// ---------------------------------------------------------------------------

/// Where translations come from.
///
/// Implemented by the installer over the package's string table. The container
/// asks for the Arabic of each source string it finds in the game's own native
/// resource, so the table's shape never reaches this module.
pub trait Mutarjim {
    /// The Arabic for one source string, when the patch carries it.
    fn arabi(&self, asl: &str) -> Option<&str>;
}

/// A face for the container: its file stem, and its bytes.
///
/// The bytes are always Taarib's own, read from the component store by the
/// installer. There is no constructor that takes a path, for the reason
/// [`crate::khatt`] gives: a value that cannot name a file cannot name the
/// game's font asset.
#[derive(Clone)]
pub struct KhattHawiya {
    ism: String,
    bayt: Vec<u8>,
}

impl fmt::Debug for KhattHawiya {
    /// The byte count rather than the bytes: a face is a hundred kilobytes of
    /// hexadecimal where a log line wants one number.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KhattHawiya")
            .field("ism", &self.ism)
            .field("bayt", &format_args!("{} bytes", self.bayt.len()))
            .finish()
    }
}

impl KhattHawiya {
    /// A face the container will carry, from its file stem and its bytes.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhattMarfud`] when the stem is not one the engine can
    /// append `.ttf` to — see [`crate::khatt::ism_irtida_salih`] — or when the
    /// bytes are not a TrueType or OpenType font, which Slate's `FreeType`
    /// build would load as nothing at all.
    pub fn jadeed(ism: impl Into<String>, bayt: Vec<u8>) -> Result<Self, KhataUnreal> {
        let ism = ism.into();
        let _ = madkhal_irtida(&ism)?;
        if sighat_bayt(&bayt).is_none() {
            return Err(KhataUnreal::KhattMarfud {
                sabab: format!(
                    "the face \"{ism}\" is not a TrueType or OpenType font, and Slate would \
                     load it as nothing at all"
                ),
            });
        }
        Ok(Self { ism, bayt })
    }

    /// The file stem the engine is told.
    #[must_use]
    pub fn ism(&self) -> &str {
        &self.ism
    }

    /// The bytes the container carries.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }
}

// ---------------------------------------------------------------------------
// What was built
// ---------------------------------------------------------------------------

/// How a target's native culture was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasdarAsliya {
    /// The target's `.locmeta` named it, and the resource it named was there.
    Locmeta,
    /// No `.locmeta`, or one naming a culture the game does not ship: the
    /// culture whose stored source fingerprints match its own text is the
    /// native one, because the native resource is the source.
    Basmat,
}

impl MasdarAsliya {
    /// One phrase for the report.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Locmeta => "from the target's locmeta",
            Self::Basmat => "from the source fingerprints",
        }
    }

    /// The same phrase in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Locmeta => "من ملف locmeta الخاص بالهدف",
            Self::Basmat => "من بصمات النصوص الأصلية",
        }
    }
}

/// One localization target as it was found and rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HadafTadweel {
    /// The target's name — `Game`, `Engine`, or whatever the project called it.
    pub ism: String,
    /// The culture whose resource is the source text.
    pub thaqafa_asliya: String,
    /// How that culture was established.
    pub masdar_asliya: MasdarAsliya,
    /// Every culture written into the container, the game's own first.
    pub thaqafat: Vec<String>,
    /// The cultures the container adds beside the game's own.
    pub mudafa: Vec<String>,
    /// How many entries were replaced, summed over every culture.
    pub adad_mustabdal: usize,
    /// Whether the target's `.locmeta` was rewritten to list the added culture.
    pub locmeta_muaddal: bool,
}

/// What became of the face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalatKhatt {
    /// Taarib's face travels in the container and the engine is told its name.
    Mahmul {
        /// The file stem the engine reads.
        ism: String,
        /// Where the face sits in the container.
        masar: String,
    },
    /// The engine's own Arabic face is named; nothing is carried, because the
    /// installer supplied no face and the game already ships one.
    MinAlMuharrik {
        /// The file stem the engine reads.
        ism: String,
    },
    /// The engine reads no localized fallback-font name, so neither an entry
    /// nor a face was written; its own fallback composite carries Arabic.
    GhayrMaqru {
        /// The face the installer supplied and this module set aside.
        ism: Option<String>,
    },
    /// The engine reads a name and there is nothing to name: no face was
    /// supplied and the game ships none. Arabic will draw from the last-resort
    /// face, which is boxes.
    LaShay,
}

impl HalatKhatt {
    /// The face's stem, when one is named.
    #[must_use]
    pub fn ism_musamma(&self) -> Option<&str> {
        match self {
            Self::Mahmul { ism, .. } | Self::MinAlMuharrik { ism } => Some(ism),
            Self::GhayrMaqru { .. } | Self::LaShay => None,
        }
    }

    /// One line for the report.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Mahmul { ism, masar } => format!(
                "font: {ism} travels in the container at {masar} and is named as Slate's \
                 fallback face, which draws every glyph the game's own fonts lack"
            ),
            Self::MinAlMuharrik { ism } => format!(
                "font: the engine's own {ism} is named as Slate's fallback face; no face is \
                 carried"
            ),
            Self::GhayrMaqru { ism } => match ism {
                Some(ism) => format!(
                    "font: this engine reads no localized fallback-font name, so {ism} was not \
                     carried; the engine's own fallback composite carries an Arabic face"
                ),
                None => "font: this engine reads no localized fallback-font name; its own \
                         fallback composite carries an Arabic face"
                    .to_owned(),
            },
            Self::LaShay => "font: this engine reads a fallback-font name and there is no face \
                             to name — no face was supplied and the game ships none — so Arabic \
                             will draw from the last-resort face as boxes"
                .to_owned(),
        }
    }

    /// The same line in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Mahmul { ism, masar } => format!(
                "الخط: يحمل الحاوية الخطَّ {ism} في {masar} ويُسمَّى خطَّ الاحتياط في Slate، \
                 وهو الذي يرسم كل حرف تفتقده خطوط اللعبة نفسها"
            ),
            Self::MinAlMuharrik { ism } => format!(
                "الخط: يُسمَّى خطُّ المحرّك نفسه {ism} خطَّ الاحتياط في Slate، ولا يُحمل خط"
            ),
            Self::GhayrMaqru { ism } => match ism {
                Some(ism) => format!(
                    "الخط: هذا المحرّك لا يقرأ اسم خطّ احتياط مترجمًا، فلم يُحمل {ism}؛ خطّ \
                     الاحتياط المركّب في المحرّك يحمل خطًّا عربيًّا"
                ),
                None => "الخط: هذا المحرّك لا يقرأ اسم خطّ احتياط مترجمًا؛ خطّ الاحتياط \
                         المركّب فيه يحمل خطًّا عربيًّا"
                    .to_owned(),
            },
            Self::LaShay => "الخط: يقرأ هذا المحرّك اسم خطّ احتياط ولا خطَّ يُسمَّى — لم يُقدَّم \
                             خط ولا تشحن اللعبة خطًّا — فستُرسم العربية من خطّ الملاذ الأخير \
                             مربّعات"
                .to_owned(),
        }
    }
}

/// What [`ibni`] produced, for the install report and the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerHawiya {
    /// The container version written, which is the game's own capped at
    /// [`crate::mawarid::pak::ISDAR_KITABA`].
    pub isdar_pak: u32,
    /// The mount point, copied from the game's own container.
    pub nuqtat_wasl: String,
    /// Every localization target, in name order.
    pub ahdaf: Vec<HadafTadweel>,
    /// How many distinct source strings the patch translated, over every
    /// target.
    pub adad_masadir_mutarjama: usize,
    /// What became of the face.
    pub khatt: HalatKhatt,
    /// Every path the container holds, in the order it stores them.
    pub masarat: Vec<String>,
}

impl TaqreerHawiya {
    /// The report as lines, in English.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.ahdaf.len().saturating_add(2));
        sutur.push(format!(
            "container: pak version {} mounted at {}, {} file(s), {} source string(s) \
             translated",
            self.isdar_pak,
            self.nuqtat_wasl,
            self.masarat.len(),
            self.adad_masadir_mutarjama
        ));
        for hadaf in &self.ahdaf {
            let mudafa = if hadaf.mudafa.is_empty() {
                String::new()
            } else {
                format!(" ({} added)", hadaf.mudafa.join(", "))
            };
            sutur.push(format!(
                "  target {}: native {} {}; {} culture(s) rewritten{mudafa}; {} entries \
                 replaced{}",
                hadaf.ism,
                hadaf.thaqafa_asliya,
                hadaf.masdar_asliya.wasf(),
                hadaf.thaqafat.len(),
                hadaf.adad_mustabdal,
                if hadaf.locmeta_muaddal {
                    "; locmeta now lists it"
                } else {
                    ""
                }
            ));
        }
        sutur.push(format!("  {}", self.khatt.wasf()));
        sutur
    }

    /// The same report in Arabic, line for line.
    #[must_use]
    pub fn taqreer_arabi(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.ahdaf.len().saturating_add(2));
        sutur.push(format!(
            "الحاوية: إصدار pak {} عند نقطة الوصل {}، {} ملفًّا، {} نصًّا أصليًّا مترجمًا",
            self.isdar_pak,
            self.nuqtat_wasl,
            self.masarat.len(),
            self.adad_masadir_mutarjama
        ));
        for hadaf in &self.ahdaf {
            let mudafa = if hadaf.mudafa.is_empty() {
                String::new()
            } else {
                format!(" (أُضيفت {})", hadaf.mudafa.join("، "))
            };
            sutur.push(format!(
                "  الهدف {}: الأصلية {} {}؛ أُعيدت كتابة {} ثقافة{mudafa}؛ استُبدل {} مدخلًا{}",
                hadaf.ism,
                hadaf.thaqafa_asliya,
                hadaf.masdar_asliya.wasf_arabi(),
                hadaf.thaqafat.len(),
                hadaf.adad_mustabdal,
                if hadaf.locmeta_muaddal {
                    "؛ ويذكرها locmeta الآن"
                } else {
                    ""
                }
            ));
        }
        sutur.push(format!("  {}", self.khatt.wasf_arabi()));
        sutur
    }
}

// ---------------------------------------------------------------------------
// What was found
// ---------------------------------------------------------------------------

/// One resource inside one of the game's containers.
#[derive(Debug, Clone)]
struct MawqiMawrid {
    hawiya: usize,
    masar: String,
}

/// One target's resources, located but not yet read.
#[derive(Debug, Clone, Default)]
struct HadafKhaam {
    badiya: String,
    locmeta: Option<MawqiMawrid>,
    thaqafat: BTreeMap<String, MawqiMawrid>,
}

/// One target's resources, read and understood.
#[derive(Debug)]
struct HadafMaqru {
    ism: String,
    badiya: String,
    hawiya: usize,
    ism_malaf: String,
    locmeta: Option<(String, MawridLocmeta)>,
    thaqafat: BTreeMap<String, (String, MawridLocres)>,
    thaqafa_asliya: String,
    masdar_asliya: MasdarAsliya,
}

impl HadafMaqru {
    /// The native culture's resource.
    fn asliya(&self) -> Result<&MawridLocres, KhataUnreal> {
        self.thaqafat
            .get(&self.thaqafa_asliya)
            .map(|(_, mawrid)| mawrid)
            .ok_or_else(|| KhataUnreal::MawridTalif {
                ism: "locres",
                haql: "the native culture, which names a resource the target does not hold",
                qeema: 0,
                hadd: 0,
            })
    }

    /// Whether this target's native resource carries the fallback-face entry,
    /// which is the evidence that this engine reads one.
    fn yaqra_irtida(&self) -> bool {
        self.asliya()
            .is_ok_and(|mawrid| mawrid.jid(FADAA_IRTIDA, MIFTAH_IRTIDA).is_some())
    }

    /// Whether a culture is one this target will have a resource for once the
    /// container is written: shipped, or the one the container adds.
    fn tahwi(&self, thaqafa: &str) -> bool {
        thaqafa.eq_ignore_ascii_case(WASM_ARABI)
            || self
                .thaqafat
                .keys()
                .any(|mawjud| mawjud.eq_ignore_ascii_case(thaqafa))
    }
}

/// The translations one target's native resource resolves to.
#[derive(Debug, Default)]
struct Tarjamat {
    madakhil: Vec<((String, String), String)>,
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

/// Builds the additive container for one game.
///
/// `bina` is what [`crate::isdar::afhas`] established about the game; the
/// containers it lists are opened here, so an encrypted one is refused by name
/// rather than read as noise. `mutarjim` answers for the patch. `khatt` is the
/// face the installer took out of the component store, when the store holds
/// one; what becomes of it is [`TaqreerHawiya::khatt`]'s to say.
///
/// # Errors
///
/// [`KhataUnreal::PakMushaffar`] when a container needs a key nobody
/// supplied; [`KhataUnreal::MawridTalif`] when the game holds no localization
/// target this module can read — a game with none has nothing to translate
/// this way, and one whose text lives in IoStore chunks alone is outside this
/// build's reader; and whatever reading a resource, editing it, or serialising
/// the container raises.
pub fn ibni(
    bina: &Bina,
    mutarjim: &dyn Mutarjim,
    khatt: Option<KhattHawiya>,
) -> Result<(KatibPak, TaqreerHawiya), KhataUnreal> {
    let hawiyat = iftah_hawiyat(bina)?;
    let khaam = ijma_ahdaf(&hawiyat)?;
    if khaam.is_empty() {
        return Err(la_tadweel(bina));
    }

    let mut ahdaf: Vec<HadafMaqru> = Vec::with_capacity(khaam.len());
    for (ism, hadaf) in &khaam {
        ahdaf.push(iqra_hadaf(ism, hadaf, &hawiyat)?);
    }

    // The primary target decides the mount point and the version: `Game` when
    // there is one, because that is the container the engine mounts first and
    // the one a modder's patch is measured against.
    let raisi = ahdaf
        .iter()
        .find(|hadaf| hadaf.ism.eq_ignore_ascii_case("Game"))
        .or_else(|| ahdaf.first())
        .ok_or_else(|| la_tadweel(bina))?;
    let hawiyat_raisi = hawiyat.get(raisi.hawiya).ok_or_else(|| la_tadweel(bina))?;
    let nuqtat_wasl = hawiyat_raisi.fahras().nuqtat_wasl().to_owned();
    let isdar_luba = hawiyat_raisi.tadhyeel().isdar;
    let mut katib = KatibPak::bi_nuqtat_wasl(&nuqtat_wasl).bi_isdar(isdar_luba)?;

    let dalil_khutut = dalil_khutut(&hawiyat);
    let hala_khatt = qarrir_khatt(&ahdaf, khatt.as_ref(), &hawiyat, &dalil_khutut);
    let hamil = ahdaf
        .iter()
        .find(|hadaf| hadaf.yaqra_irtida())
        .map(|hadaf| hadaf.ism.clone());

    let mut masadir: BTreeSet<String> = BTreeSet::new();
    let mut taqarir: Vec<HadafTadweel> = Vec::with_capacity(ahdaf.len());

    for hadaf in &ahdaf {
        let tarjamat = tarjamat_min(hadaf.asliya()?, mutarjim, &mut masadir);

        let mut thaqafat: Vec<String> = hadaf.thaqafat.keys().cloned().collect();
        let mut mudafa: Vec<String> = Vec::new();
        if !hadaf.tahwi_mashhuna(WASM_ARABI) {
            thaqafat.push(WASM_ARABI.to_owned());
            mudafa.push(WASM_ARABI.to_owned());
        }
        if thaqafat.len() > AQSA_THAQAFAT {
            return Err(KhataUnreal::HajmMufrit {
                haql: "the cultures of one localization target",
                qeema: u64::try_from(thaqafat.len()).unwrap_or(u64::MAX),
                saqf: u64::try_from(AQSA_THAQAFAT).unwrap_or(u64::MAX),
            });
        }

        let mut adad_mustabdal = 0_usize;
        for thaqafa in &thaqafat {
            let (masar, mut mawrid) = match hadaf.thaqafat.get(thaqafa) {
                Some((masar, mawrid)) => (masar.clone(), mawrid.clone()),
                None => (
                    format!("{}{thaqafa}/{}", hadaf.badiya, hadaf.ism_malaf),
                    hadaf.asliya()?.clone(),
                ),
            };
            adad_mustabdal = adad_mustabdal.saturating_add(tabbiq(&mut mawrid, &tarjamat)?);
            if let Some(ism) = hala_khatt.ism_musamma()
                && hamil_li(&ahdaf, hamil.as_deref(), thaqafa) == Some(hadaf.ism.as_str())
            {
                mawrid.adif(FADAA_IRTIDA, MIFTAH_IRTIDA, ASL_IRTIDA, ism)?;
            }
            mawrid.ahsi_ihsaat();
            katib.daa(&masar, mawrid.ila_bayt()?)?;
        }

        let locmeta_muaddal = match &hadaf.locmeta {
            Some((masar, locmeta))
                if locmeta.isdar().yahwi_thaqafat() && !locmeta.yahwi_thaqafa(WASM_ARABI) =>
            {
                let mut jadeed = locmeta.clone();
                let _ = jadeed.adif_thaqafa(WASM_ARABI)?;
                katib.daa(masar, jadeed.ila_bayt()?)?;
                true
            },
            _ => false,
        };

        taqarir.push(HadafTadweel {
            ism: hadaf.ism.clone(),
            thaqafa_asliya: hadaf.thaqafa_asliya.clone(),
            masdar_asliya: hadaf.masdar_asliya,
            thaqafat,
            mudafa,
            adad_mustabdal,
            locmeta_muaddal,
        });
    }

    if let (HalatKhatt::Mahmul { masar, .. }, Some(khatt)) = (&hala_khatt, khatt.as_ref()) {
        katib.daa(masar, khatt.bayt().to_vec())?;
    }

    let masarat: Vec<String> = katib.masarat().map(str::to_owned).collect();
    let taqreer = TaqreerHawiya {
        isdar_pak: katib.isdar().raqm(),
        nuqtat_wasl,
        ahdaf: taqarir,
        adad_masadir_mutarjama: masadir.len(),
        khatt: hala_khatt,
        masarat,
    };
    Ok((katib, taqreer))
}

impl HadafMaqru {
    /// Whether the game itself ships a culture, compared as language tags are.
    fn tahwi_mashhuna(&self, thaqafa: &str) -> bool {
        self.thaqafat
            .keys()
            .any(|mawjud| mawjud.eq_ignore_ascii_case(thaqafa))
    }
}

/// The target whose resource carries the fallback-face entry for one culture.
///
/// The engine's own carrier — the target whose native resource already holds
/// the entry, which is `Engine` on every build that reads it — when it will
/// have a resource for that culture, so the entry is replaced where the engine
/// expects it and appears once. Otherwise the first target that will, so a
/// culture the engine target does not ship still gets the entry from the game
/// target's resource; the lookup is by namespace and key over every loaded
/// resource, so where it comes from does not matter as long as it is there
/// once.
fn hamil_li<'a>(ahdaf: &'a [HadafMaqru], hamil: Option<&str>, thaqafa: &str) -> Option<&'a str> {
    if let Some(hamil) = hamil
        && let Some(hadaf) = ahdaf.iter().find(|hadaf| hadaf.ism == hamil)
        && hadaf.tahwi(thaqafa)
    {
        return Some(hadaf.ism.as_str());
    }
    ahdaf
        .iter()
        .find(|hadaf| hadaf.tahwi(thaqafa))
        .map(|hadaf| hadaf.ism.as_str())
}

/// What becomes of the face, decided from evidence rather than from the
/// engine version: whether any native resource carries the entry the name is
/// read through.
fn qarrir_khatt(
    ahdaf: &[HadafMaqru],
    khatt: Option<&KhattHawiya>,
    hawiyat: &[HawiyatPak],
    dalil_khutut: &str,
) -> HalatKhatt {
    let yaqra = ahdaf.iter().any(HadafMaqru::yaqra_irtida);
    if !yaqra {
        return HalatKhatt::GhayrMaqru {
            ism: khatt.map(|khatt| khatt.ism().to_owned()),
        };
    }
    if let Some(khatt) = khatt {
        return HalatKhatt::Mahmul {
            ism: khatt.ism().to_owned(),
            masar: masar_khatt_slate(dalil_khutut, khatt.ism()),
        };
    }
    let muharrik = masar_khatt_slate(dalil_khutut, KHATT_MUHARRIK_ARABI).to_ascii_lowercase();
    let yashhan = hawiyat
        .iter()
        .flat_map(HawiyatPak::masarat)
        .any(|masar| masar.to_ascii_lowercase() == muharrik);
    if yashhan {
        HalatKhatt::MinAlMuharrik {
            ism: KHATT_MUHARRIK_ARABI.to_owned(),
        }
    } else {
        HalatKhatt::LaShay
    }
}

/// The directory the engine reads its fallback faces from, taken from where
/// the game's own fallback face sits; [`DALIL_KHUTUT_SLATE`] when the game
/// ships none.
fn dalil_khutut(hawiyat: &[HawiyatPak]) -> String {
    let dhayl = format!("/{}", MALAF_IRTIDA_MUHARRIK.to_ascii_lowercase());
    hawiyat
        .iter()
        .flat_map(HawiyatPak::masarat)
        .find(|masar| masar.to_ascii_lowercase().ends_with(&dhayl))
        .and_then(|masar| masar.rsplit_once('/').map(|(dalil, _)| dalil.to_owned()))
        .unwrap_or_else(|| DALIL_KHUTUT_SLATE.to_owned())
}

/// The translations a native resource resolves to, recording every source the
/// patch answered for.
fn tarjamat_min(
    asliya: &MawridLocres,
    mutarjim: &dyn Mutarjim,
    masadir: &mut BTreeSet<String>,
) -> Tarjamat {
    let mut tarjamat = Tarjamat::default();
    for (fadaa, madkhal) in asliya.madakhil() {
        let Some(asl) = asliya.nass_madkhal(madkhal) else {
            continue;
        };
        let Some(arabi) = mutarjim.arabi(asl) else {
            continue;
        };
        let _ = masadir.insert(asl.to_owned());
        tarjamat.madakhil.push((
            (
                fadaa.ism().nass().to_owned(),
                madkhal.miftah().nass().to_owned(),
            ),
            arabi.to_owned(),
        ));
    }
    tarjamat
}

/// Replaces every translated entry a resource holds, and counts them.
fn tabbiq(mawrid: &mut MawridLocres, tarjamat: &Tarjamat) -> Result<usize, KhataUnreal> {
    let mut adad = 0_usize;
    for ((fadaa, miftah), arabi) in &tarjamat.madakhil {
        if mawrid.istabdil(fadaa, miftah, arabi)? {
            adad = adad.saturating_add(1);
        }
    }
    Ok(adad)
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Opens the game's own `.pak` containers, Taarib's excluded.
///
/// A previous installation's container must not be a source: it would feed
/// this build's Arabic back in as the game's own text, and the source
/// fingerprints would then be fingerprints of Arabic.
fn iftah_hawiyat(bina: &Bina) -> Result<Vec<HawiyatPak>, KhataUnreal> {
    let mut hawiyat: Vec<HawiyatPak> = Vec::new();
    for masar in bina.hawiyat.iter().take(AQSA_HAWIYAT) {
        let pak = masar
            .extension()
            .and_then(|imtidad| imtidad.to_str())
            .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("pak"));
        let taarib = masar
            .file_name()
            .and_then(|ism| ism.to_str())
            .is_some_and(|ism| ism.eq_ignore_ascii_case(ISM_HAWIYA));
        if !pak || taarib {
            continue;
        }
        hawiyat.push(HawiyatPak::iftah(masar, None)?);
    }
    Ok(hawiyat)
}

/// Locates every localization target across the containers.
///
/// A later container overrides an earlier one for the same path, in the sorted
/// order [`crate::isdar::afhas`] lists them — an approximation of the engine's
/// mount order that is exact for every game that ships one container.
fn ijma_ahdaf(hawiyat: &[HawiyatPak]) -> Result<BTreeMap<String, HadafKhaam>, KhataUnreal> {
    let mut ahdaf: BTreeMap<String, HadafKhaam> = BTreeMap::new();
    for (fahras, hawiya) in hawiyat.iter().enumerate() {
        for masar in hawiya.masarat_locres() {
            let Some(ajza) = hallil(masar) else { continue };
            let Some(thaqafa) = ajza.thaqafa else { continue };
            let hadaf = madkhal_hadaf(&mut ahdaf, ajza.hadaf, &ajza.badiya)?;
            let _ = hadaf.thaqafat.insert(
                thaqafa.to_owned(),
                MawqiMawrid {
                    hawiya: fahras,
                    masar: masar.to_owned(),
                },
            );
        }
        for masar in hawiya.masarat_locmeta() {
            let Some(ajza) = hallil(masar) else { continue };
            if ajza.thaqafa.is_some() {
                continue;
            }
            let hadaf = madkhal_hadaf(&mut ahdaf, ajza.hadaf, &ajza.badiya)?;
            hadaf.locmeta = Some(MawqiMawrid {
                hawiya: fahras,
                masar: masar.to_owned(),
            });
        }
    }
    Ok(ahdaf)
}

/// One target's slot in the table, created within [`AQSA_AHDAF`].
fn madkhal_hadaf<'a>(
    ahdaf: &'a mut BTreeMap<String, HadafKhaam>,
    ism: &str,
    badiya: &str,
) -> Result<&'a mut HadafKhaam, KhataUnreal> {
    if !ahdaf.contains_key(ism) && ahdaf.len() >= AQSA_AHDAF {
        return Err(KhataUnreal::HajmMufrit {
            haql: "the localization targets of one game",
            qeema: u64::try_from(ahdaf.len().saturating_add(1)).unwrap_or(u64::MAX),
            saqf: u64::try_from(AQSA_AHDAF).unwrap_or(u64::MAX),
        });
    }
    let hadaf = ahdaf.entry(ism.to_owned()).or_default();
    if hadaf.badiya.is_empty() {
        hadaf.badiya = badiya.to_owned();
    }
    Ok(hadaf)
}

/// The parts of a localization path.
#[derive(Debug)]
struct AjzaMasar<'a> {
    badiya: String,
    hadaf: &'a str,
    thaqafa: Option<&'a str>,
}

/// Splits `<...>/Localization/<Target>/<Culture>/<file>.locres` or
/// `<...>/Localization/<Target>/<file>.locmeta` into its parts.
fn hallil(masar: &str) -> Option<AjzaMasar<'_>> {
    let ajza: Vec<&str> = masar.split('/').collect();
    let mawqi = ajza
        .iter()
        .position(|juz| juz.eq_ignore_ascii_case(DALIL_TADWEEL))?;
    let hadaf = ajza.get(mawqi.checked_add(1)?)?;
    let badiya = format!("{}/", ajza.get(..=mawqi.checked_add(1)?)?.join("/"));
    let baqi = ajza.len().checked_sub(mawqi)?;
    let thaqafa = match baqi {
        3 => None,
        4 => Some(*ajza.get(mawqi.checked_add(2)?)?),
        _ => return None,
    };
    Some(AjzaMasar {
        badiya,
        hadaf,
        thaqafa,
    })
}

/// Reads one target's resources and establishes its native culture.
fn iqra_hadaf(
    ism: &str,
    khaam: &HadafKhaam,
    hawiyat: &[HawiyatPak],
) -> Result<HadafMaqru, KhataUnreal> {
    let mut thaqafat: BTreeMap<String, (String, MawridLocres)> = BTreeMap::new();
    let mut hawiya = 0_usize;
    for (thaqafa, mawqi) in &khaam.thaqafat {
        let bayt = iqra(hawiyat, mawqi)?;
        let _ = thaqafat.insert(
            thaqafa.clone(),
            (mawqi.masar.clone(), MawridLocres::min_bayt(&bayt)?),
        );
        hawiya = mawqi.hawiya;
    }
    if thaqafat.is_empty() {
        return Err(KhataUnreal::MawridTalif {
            ism: "locres",
            haql: "a localization target with a locmeta and no culture resource",
            qeema: 0,
            hadd: 1,
        });
    }
    let locmeta = match &khaam.locmeta {
        Some(mawqi) => {
            let bayt = iqra(hawiyat, mawqi)?;
            Some((mawqi.masar.clone(), MawridLocmeta::min_bayt(&bayt)?))
        },
        None => None,
    };
    let (thaqafa_asliya, masdar_asliya) = thaqafa_asliya(&thaqafat, locmeta.as_ref());
    let ism_malaf = thaqafat
        .get(&thaqafa_asliya)
        .and_then(|(masar, _)| masar.rsplit('/').next())
        .map_or_else(|| format!("{ism}.locres"), str::to_owned);

    Ok(HadafMaqru {
        ism: ism.to_owned(),
        badiya: khaam.badiya.clone(),
        hawiya,
        ism_malaf,
        locmeta,
        thaqafat,
        thaqafa_asliya,
        masdar_asliya,
    })
}

/// One resource's bytes out of the container that holds it.
fn iqra(hawiyat: &[HawiyatPak], mawqi: &MawqiMawrid) -> Result<Vec<u8>, KhataUnreal> {
    let hawiya = hawiyat
        .get(mawqi.hawiya)
        .ok_or_else(|| KhataUnreal::MawridTalif {
            ism: "pak",
            haql: "a container index this build recorded and cannot find",
            qeema: u64::try_from(mawqi.hawiya).unwrap_or(u64::MAX),
            hadd: u64::try_from(hawiyat.len()).unwrap_or(u64::MAX),
        })?;
    hawiya.iqra_masar(&mawqi.masar)
}

/// Which culture is the source, and how that was decided.
///
/// The `.locmeta` when it names a culture the game ships; otherwise the
/// culture whose stored fingerprints match its own text most often, because
/// the native resource is the one whose text *is* the source. A game whose
/// manifest names `en-US-POSIX` and ships `en` — which is a real shipping
/// title — takes the second route, and lands on `en`. Ties fall to `en`, then
/// to name order, so two runs cannot disagree.
fn thaqafa_asliya(
    thaqafat: &BTreeMap<String, (String, MawridLocres)>,
    locmeta: Option<&(String, MawridLocmeta)>,
) -> (String, MasdarAsliya) {
    if let Some((_, locmeta)) = locmeta {
        let musamma = locmeta.thaqafa_asliya();
        if let Some(mawjud) = thaqafat
            .keys()
            .find(|thaqafa| thaqafa.eq_ignore_ascii_case(musamma))
        {
            return (mawjud.clone(), MasdarAsliya::Locmeta);
        }
    }

    let mut afdal: Option<(&str, u64, u64)> = None;
    for (thaqafa, (_, mawrid)) in thaqafat {
        let (mutabiq, kull) = darajat_asala(mawrid);
        let ahsan = match afdal {
            None => true,
            // Cross-multiplied rather than divided: two ratios compared without
            // a division and without a float.
            Some((sabiq, mutabiq_sabiq, kull_sabiq)) => {
                let hadha = u128::from(mutabiq).saturating_mul(u128::from(kull_sabiq));
                let dhak = u128::from(mutabiq_sabiq).saturating_mul(u128::from(kull));
                hadha > dhak || (hadha == dhak && thaqafa.eq_ignore_ascii_case("en") && sabiq != "en")
            },
        };
        if ahsan {
            afdal = Some((thaqafa.as_str(), mutabiq, kull));
        }
    }
    let ism = afdal
        .map(|(thaqafa, _, _)| thaqafa.to_owned())
        .unwrap_or_default();
    (ism, MasdarAsliya::Basmat)
}

/// How many entries of a resource carry their own text's fingerprint, out of
/// how many carry any text at all.
fn darajat_asala(mawrid: &MawridLocres) -> (u64, u64) {
    let mut mutabiq = 0_u64;
    let mut kull = 0_u64;
    for (_, madkhal) in mawrid.madakhil() {
        let Some(nass) = mawrid.nass_madkhal(madkhal) else {
            continue;
        };
        kull = kull.saturating_add(1);
        if basmat_asl(nass) == madkhal.basmat_asl() {
            mutabiq = mutabiq.saturating_add(1);
        }
    }
    (mutabiq, kull)
}

/// The refusal for a game with no readable localization target.
fn la_tadweel(bina: &Bina) -> KhataUnreal {
    let haql = if bina.tabaa == Tabaa::Iostore {
        "a localization target, of which none is in a .pak; this build reads .locres out of \
         .pak containers and not out of IoStore chunks"
    } else {
        "a localization target, of which the game's containers hold none"
    };
    KhataUnreal::MawridTalif {
        ism: "pak",
        haql,
        qeema: 0,
        hadd: 1,
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use super::{AjzaMasar, hallil};
    use crate::khatt::ism_irtida_salih;
    use crate::mawarid::locres::basmat_asl;

    #[test]
    fn a_culture_path_splits_into_target_culture_and_prefix() {
        let AjzaMasar {
            badiya,
            hadaf,
            thaqafa,
        } = hallil("Atlas/Content/Localization/Game/pt-BR/Game.locres").expect("a locres path");
        assert_eq!(badiya, "Atlas/Content/Localization/Game/");
        assert_eq!(hadaf, "Game");
        assert_eq!(thaqafa, Some("pt-BR"));
    }

    #[test]
    fn a_locmeta_path_has_no_culture() {
        let ajza = hallil("Atlas/Content/Localization/Game/Game.locmeta").expect("a locmeta path");
        assert_eq!(ajza.hadaf, "Game");
        assert!(ajza.thaqafa.is_none());
    }

    #[test]
    fn a_path_outside_localization_is_not_a_target() {
        assert!(hallil("Atlas/Content/UI/Fonts/Almarai-Regular.ufont").is_none());
        assert!(hallil("Atlas/Content/Localization/Game/en/deeper/Game.locres").is_none());
    }

    #[test]
    fn the_fallback_source_fingerprint_is_the_engines_own() {
        // Read off a shipping 4.13 engine resource: the entry ("Slate",
        // "FallbackFont") stores this fingerprint of "DroidSansFallback".
        assert_eq!(basmat_asl("DroidSansFallback"), 0x382c_36a1);
    }

    #[test]
    fn a_face_name_is_a_bare_stem() {
        assert!(ism_irtida_salih("IBMPlexSansArabic-Regular"));
        assert!(!ism_irtida_salih("IBMPlexSansArabic-Regular.ttf"));
        assert!(!ism_irtida_salih("khutut/IBMPlexSansArabic-Regular"));
        assert!(!ism_irtida_salih(""));
    }
}
