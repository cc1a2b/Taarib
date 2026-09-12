//! فهرس التعريبات المجتمعية — the community index: what other teams already
//! made for a game, where it lives, and who made it.
//!
//! `fahras/tarjamat.json` in the registry repository lists Arabic translations
//! other teams published on their own pages, with the facts each page states.
//! The registry links; it hosts nothing in the list, redistributes nothing and
//! signs nothing here — an entry is a credit and an address, not a patch. So
//! this module is deliberately weaker than [`crate::fahras`]: no manifest hash
//! vouches for the bytes, because nothing is installed from them. What it keeps
//! from the shard path is the rest of the discipline — the same source chain,
//! the same repository-relative path, a byte cap, a schema version this build
//! refuses to guess past, validation of every field the product reads, an
//! atomic cache under the data root, and a refresh record beside it so a stale
//! answer can say how stale it is and which source last refused.

use std::collections::BTreeSet;
use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use taarib_mustalahat::wahhid_ism;
use taarib_usus::masarat::{Masarat, kitaba_dharra};

use crate::jalb::MUJALLAD_MAKHBAA;
use crate::khata::{KhataMustawda, NatijatMustawda};
use crate::masadir::SilsilatMasadir;

/// The repository path of the community index.
pub const MASAR_FAHRAS_MUJTAMA: &str = "fahras/tarjamat.json";

/// The index schema this build reads.
pub const ISDAR_FAHRAS_MUJTAMA: u32 = 1;

/// Largest index accepted, in bytes.
///
/// The live index is under four hundred kilobytes for a few hundred entries,
/// so this admits an index ten times the size before anything is refused. It
/// is judged after the chain's own transfer cap has already bounded the bytes
/// taken, and it exists for the cache as much as for the network: a file under
/// the data root that has grown past it is not an index this build wrote.
pub const HADD_HAJM_FAHRAS_MUJTAMA: u64 = 4 * 1024 * 1024;

/// How long a fetched index counts as current.
///
/// A day. The index changes when a team publishes, which is weeks apart, and a
/// stale answer costs nothing but a credit shown a day late; asking the forge
/// on every visit to a game screen would cost a request per visit for that.
pub const NAFIDHAT_FAHRAS_MUJTAMA: SignedDuration = SignedDuration::from_hours(24);

/// The directory under the registry cache the index and its record live in —
/// the index's own repository directory, so a cached copy and a bundled
/// mirror's copy are the same file in the same place.
const MUJALLAD_FAHRAS: &str = "fahras";

/// The cached index's file name, the repository's own.
const ISM_MALAF_FAHRAS: &str = "tarjamat.json";

/// The refresh record beside it.
const ISM_MALAF_SIJILL: &str = "tarjamat.sijill.json";

/// The record format this build writes.
const ISDAR_SIJILL: u32 = 1;

/// Longest address accepted from the index, the same bound a listing's
/// release asset is held to.
const HADD_TUL_RABT: usize = 2048;

/// Longest identifier accepted for an entry or a team.
const HADD_TUL_MUARRIF: usize = 128;

/// What a cache write is doing, for the failure that names it.
const AMAL_KITABA: &str = "caching the community index";

/// يقرأ اسمًا من الفهرس ويجيب بالمجهول عمّا لا يعرفه هذا الإصدار — reads a
/// `snake_case` name from the index, answering the unknown variant for a name
/// this build does not know.
///
/// `#[serde(other)]` said exactly this in one line, and is what these seven
/// enums carried. It was removed because `specta` refuses that attribute on an
/// enum that is not tagged, and a tagged representation would change the wire
/// format the index is actually written in. While any one of them carried it the
/// studio's whole interface vocabulary could not be regenerated: the generator
/// stops at the first offender, names it, and writes nothing — so one attribute
/// froze every binding in the product.
///
/// The behaviour is unchanged. An unknown name becomes the unknown variant
/// rather than failing the parse, which is the whole point: the index is written
/// by a later version of this project than the client reading it, and one new
/// coverage kind must not cost a user their whole list.
macro_rules! naw_min_nass {
    ($naw:ident { $($nass:literal => $farq:ident),+ $(,)? } wa_illa $majhul:ident) => {
        impl<'de> Deserialize<'de> for $naw {
            fn deserialize<M>(mufakkik: M) -> Result<Self, M::Error>
            where
                M: serde::Deserializer<'de>,
            {
                let nass = <std::borrow::Cow<'de, str>>::deserialize(mufakkik)?;
                Ok(match nass.as_ref() {
                    $($nass => Self::$farq,)+
                    _ => Self::$majhul,
                })
            }
        }
    };
}

/// Where a translation's home page lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MudifTarjama {
    /// A Steam Workshop item.
    SteamWorkshop,
    /// A Nexus Mods page.
    Nexusmods,
    /// A Thunderstore package.
    Thunderstore,
    /// A `GameBanana` page.
    Gamebanana,
    /// A GitHub repository or release.
    Github,
    /// An itch.io page.
    Itch,
    /// The Outer Wilds mod database.
    Outerwildsmods,
    /// The team's own site.
    MawqiAlfariq,
    /// A blog.
    Mudawwana,
    /// A host kind this build does not know: the index grew a kind after this
    /// build shipped. The page's own address still says where it is.
    Majhul,
}

naw_min_nass!(MudifTarjama {
    "steam_workshop" => SteamWorkshop,
    "nexusmods" => Nexusmods,
    "thunderstore" => Thunderstore,
    "gamebanana" => Gamebanana,
    "github" => Github,
    "itch" => Itch,
    "outerwildsmods" => Outerwildsmods,
    "mawqi_alfariq" => MawqiAlfariq,
    "mudawwana" => Mudawwana,
} wa_illa Majhul);

/// How much of the game a translation covers, as its page states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum TaghtiyaMujtama {
    /// Interface and dialogue both.
    Kamila,
    /// The interface only.
    Wajiha,
    /// Dialogue or subtitles only.
    Hiwar,
    /// Part of the game, by the page's own account.
    Juziya,
    /// The page does not say.
    GhayrMusarraha,
    /// A coverage kind this build does not know.
    Majhul,
}

naw_min_nass!(TaghtiyaMujtama {
    "kamila" => Kamila,
    "wajiha" => Wajiha,
    "hiwar" => Hiwar,
    "juziya" => Juziya,
    "ghayr_musarraha" => GhayrMusarraha,
} wa_illa Majhul);

/// How a translation was made, when its page says so.
///
/// The same three values Taarib's own listings carry in
/// `taarib_mustalahat::ruqaa::TareeqaTarjama`, plus the honest fourth: most
/// pages do not say, and the index records only what a page states or what
/// named translators imply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum TareeqaMujtama {
    /// Translated by people, start to finish.
    BashariyaKamila,
    /// Machine-translated, then reviewed by people.
    AaliyaThumBashariya,
    /// Machine-translated with no review.
    AaliyaFaqat,
    /// The page does not say.
    GhayrMusarraha,
    /// A method this build does not know.
    Majhul,
}

naw_min_nass!(TareeqaMujtama {
    "bashariya_kamila" => BashariyaKamila,
    "aaliya_thum_bashariya" => AaliyaThumBashariya,
    "aaliya_faqat" => AaliyaFaqat,
    "ghayr_musarraha" => GhayrMusarraha,
} wa_illa Majhul);

/// Under what terms a translation is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum NawRukhsa {
    /// The page states no terms.
    GhayrMusarraha,
    /// Terms stated in the page's own words.
    Musarraha,
    /// A recognised licence, named by its SPDX identifier.
    Spdx,
    /// A licence kind this build does not know.
    Majhul,
}

naw_min_nass!(NawRukhsa {
    "ghayr_musarraha" => GhayrMusarraha,
    "musarraha" => Musarraha,
    "spdx" => Spdx,
} wa_illa Majhul);

/// Whether a translation is free, sold, or behind a subscription.
///
/// Carried to the screen because it is the one fact a person meets the moment
/// they follow the link, and a list that hid it would be sending people to a
/// checkout without saying so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum TawzeeTarjama {
    /// Free to download.
    Majjani,
    /// Sold through a checkout.
    Madfua,
    /// Subscription-only.
    Vip,
    /// A distribution kind this build does not know.
    Majhul,
}

naw_min_nass!(TawzeeTarjama {
    "majjani" => Majjani,
    "madfua" => Madfua,
    "vip" => Vip,
} wa_illa Majhul);

/// Where a translation stands, as its page states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum HalatTarjamaMujtama {
    /// Released.
    Nashita,
    /// An initial or beta release.
    Awwaliya,
    /// Work in progress with no release yet.
    QaydAltatwir,
    /// Discontinued by its author.
    Mahjura,
    /// A status this build does not know.
    Majhul,
}

naw_min_nass!(HalatTarjamaMujtama {
    "nashita" => Nashita,
    "awwaliya" => Awwaliya,
    "qayd_altatwir" => QaydAltatwir,
    "mahjura" => Mahjura,
} wa_illa Majhul);

/// What a page's download figure counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum NawTanzeelat {
    /// Downloads.
    Tanzeelat,
    /// Workshop subscribers.
    Mushtarikun,
    /// A count kind this build does not know.
    Majhul,
}

naw_min_nass!(NawTanzeelat {
    "tanzeelat" => Tanzeelat,
    "mushtarikun" => Mushtarikun,
} wa_illa Majhul);

/// One team, as the index credits it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FariqMujtama {
    /// The identifier entries refer to it by.
    pub muarrif: String,
    /// The team's name in Latin script.
    pub ism: String,
    /// The team's Arabic name, when it has one.
    pub ism_arabi: Option<String>,
    /// The team's own page.
    pub rabt: String,
}

/// The game an entry is for, as the entry names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LubaMujtama {
    /// The game's name.
    pub ism: String,
    /// Its Arabic name, when the page gives one.
    pub ism_arabi: Option<String>,
}

/// The terms a translation is offered under, as far as its page states them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RukhsaMujtama {
    /// How the terms are stated.
    pub naw: NawRukhsa,
    /// The SPDX identifier, when `naw` is [`NawRukhsa::Spdx`].
    pub muarrif: Option<String>,
    /// The statement as written.
    pub nass: Option<String>,
}

/// How an entry's facts were read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TahaqquqMujtama {
    /// The day the page or endpoint was read, `YYYY-MM-DD`.
    pub waqt: String,
    /// The address that was actually fetched.
    pub rabt: String,
    /// Anything the reading had to say for itself.
    pub mulahaza: Option<String>,
}

/// One community translation.
///
/// Every field is what the translation's own page states, read on the day
/// [`Self::tahaqquq`] records. Nothing here is Taarib's judgement of the work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tarjama {
    /// The entry's identifier, unique in the index.
    pub muarrif: String,
    /// The game.
    pub luba: LubaMujtama,
    /// The Steam application id when the game is on Steam — the same number
    /// Taarib keys a Steam game on.
    pub steam_appid: Option<u32>,
    /// The team, as a reference into the index's team list.
    pub fariq: Option<String>,
    /// The author or team as the page names them.
    pub muallif: String,
    /// Where the home page lives.
    pub mudif: MudifTarjama,
    /// The home page.
    pub rabt: String,
    /// Coverage, as a kind.
    pub taghtiya: TaghtiyaMujtama,
    /// Coverage, in the page's own words.
    pub taghtiya_nass: Option<String>,
    /// How it was translated.
    pub tareeqa: TareeqaMujtama,
    /// The terms.
    pub rukhsa: RukhsaMujtama,
    /// Free, sold, or subscription-only.
    pub tawzee: TawzeeTarjama,
    /// Where it stands.
    pub hala: HalatTarjamaMujtama,
    /// The version the page states.
    pub isdar: Option<String>,
    /// When it was published, `YYYY-MM-DD`.
    pub waqt_alnashr: Option<String>,
    /// When it was last updated, `YYYY-MM-DD`.
    pub akhir_tahdith: Option<String>,
    /// The download or subscriber count the page shows.
    pub tanzeelat: Option<u64>,
    /// What that count counts.
    pub tanzeelat_naw: Option<NawTanzeelat>,
    /// How these facts were read.
    pub tahaqquq: TahaqquqMujtama,
}

/// The community index.
///
/// Unknown fields are tolerated everywhere in it: the index is maintained by
/// hand in a public repository and grows fields faster than this build ships,
/// and a client that refused an index for carrying a field it does not read
/// would refuse every index the day after any of them gained one. What is
/// read is validated; see [`Self::min_bayt`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FahrasMujtama {
    /// The index schema version.
    pub isdar: u32,
    /// When this revision of the index was cast, RFC 3339.
    pub waqt: String,
    /// Every team an entry may refer to.
    pub firaq: Vec<FariqMujtama>,
    /// Every translation.
    pub tarjamat: Vec<Tarjama>,
}

impl FahrasMujtama {
    /// Parses an index, refusing one over the byte cap, one of a schema this
    /// build does not read, and one whose read fields do not hold together.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::FahrasMujtamaKabir`] when the bytes pass
    /// [`HADD_HAJM_FAHRAS_MUJTAMA`], and [`KhataMustawda::FahrasMujtamaTalif`]
    /// when they are not JSON of this shape, when `isdar` is not
    /// [`ISDAR_FAHRAS_MUJTAMA`], or when an entry names a team the index does
    /// not list, an address that is not `https`, an identifier outside the
    /// index's own alphabet, a date that is not `YYYY-MM-DD`, or a duplicate.
    pub fn min_bayt(bayt: &[u8]) -> NatijatMustawda<Self> {
        let hajm = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
        if hajm > HADD_HAJM_FAHRAS_MUJTAMA {
            return Err(KhataMustawda::FahrasMujtamaKabir {
                hajm,
                hadd: HADD_HAJM_FAHRAS_MUJTAMA,
            });
        }
        let fahras: Self =
            serde_json::from_slice(bayt).map_err(|khata| talif(khata.to_string()))?;
        if fahras.isdar != ISDAR_FAHRAS_MUJTAMA {
            return Err(talif(format!(
                "index schema {} is not the {ISDAR_FAHRAS_MUJTAMA} this build reads",
                fahras.isdar
            )));
        }
        fahras.tahaqqaq()?;
        Ok(fahras)
    }

    /// The team an identifier names, when the index lists it.
    #[must_use]
    pub fn fariq(&self, muarrif: &str) -> Option<&FariqMujtama> {
        self.firaq.iter().find(|fariq| fariq.muarrif == muarrif)
    }

    /// When this revision was cast.
    ///
    /// `None` only for an index that did not pass [`Self::min_bayt`], which
    /// refuses an unparsable stamp; kept as an option so a caller holding a
    /// deserialized value from elsewhere is not promised a parse it did not run.
    #[must_use]
    pub fn waqt_lahza(&self) -> Option<Timestamp> {
        self.waqt.parse::<Timestamp>().ok()
    }

    /// Every host the index links to — each entry's page and each team's page
    /// — lowercased, for a caller deciding which addresses it will open.
    #[must_use]
    pub fn mudifun(&self) -> BTreeSet<String> {
        self.tarjamat
            .iter()
            .map(|tarjama| tarjama.rabt.as_str())
            .chain(self.firaq.iter().map(|fariq| fariq.rabt.as_str()))
            .filter_map(mudif_rabt)
            .collect()
    }

    fn tahaqqaq(&self) -> NatijatMustawda<()> {
        if self.waqt.parse::<Timestamp>().is_err() {
            return Err(talif(format!(
                "the index stamp {:?} is not RFC 3339",
                self.waqt
            )));
        }

        let mut firaq: BTreeSet<&str> = BTreeSet::new();
        for fariq in &self.firaq {
            if !muarrif_salih(&fariq.muarrif) {
                return Err(talif(format!(
                    "team identifier {:?} is outside the index's alphabet",
                    fariq.muarrif
                )));
            }
            if !firaq.insert(fariq.muarrif.as_str()) {
                return Err(talif(format!("team {:?} is listed twice", fariq.muarrif)));
            }
            if fariq.ism.trim().is_empty() {
                return Err(talif(format!("team {:?} has no name", fariq.muarrif)));
            }
            if !rabt_salih(&fariq.rabt) {
                return Err(talif(format!(
                    "team {:?} links to {:?}, which is not an https address",
                    fariq.muarrif, fariq.rabt
                )));
            }
        }

        let mut muarrifat: BTreeSet<&str> = BTreeSet::new();
        for tarjama in &self.tarjamat {
            let muarrif = tarjama.muarrif.as_str();
            if !muarrif_salih(muarrif) {
                return Err(talif(format!(
                    "entry identifier {muarrif:?} is outside the index's alphabet"
                )));
            }
            if !muarrifat.insert(muarrif) {
                return Err(talif(format!("entry {muarrif:?} is listed twice")));
            }
            if tarjama.luba.ism.trim().is_empty() {
                return Err(talif(format!("entry {muarrif:?} names no game")));
            }
            if tarjama.muallif.trim().is_empty() {
                return Err(talif(format!("entry {muarrif:?} credits nobody")));
            }
            if tarjama.steam_appid == Some(0) {
                return Err(talif(format!("entry {muarrif:?} carries Steam id 0")));
            }
            if let Some(fariq) = &tarjama.fariq
                && !firaq.contains(fariq.as_str())
            {
                return Err(talif(format!(
                    "entry {muarrif:?} names team {fariq:?}, which the index does not list"
                )));
            }
            if !rabt_salih(&tarjama.rabt) {
                return Err(talif(format!(
                    "entry {muarrif:?} links to {:?}, which is not an https address",
                    tarjama.rabt
                )));
            }
            if !rabt_salih(&tarjama.tahaqquq.rabt) {
                return Err(talif(format!(
                    "entry {muarrif:?} was read from {:?}, which is not an https address",
                    tarjama.tahaqquq.rabt
                )));
            }
            if !tareekh_salih(&tarjama.tahaqquq.waqt) {
                return Err(talif(format!(
                    "entry {muarrif:?} was read on {:?}, which is not a date",
                    tarjama.tahaqquq.waqt
                )));
            }
            for (ism, tareekh) in [
                ("published", &tarjama.waqt_alnashr),
                ("updated", &tarjama.akhir_tahdith),
            ] {
                if let Some(tareekh) = tareekh
                    && !tareekh_salih(tareekh)
                {
                    return Err(talif(format!(
                        "entry {muarrif:?} was {ism} on {tareekh:?}, which is not a date"
                    )));
                }
            }
            if tarjama.rukhsa.naw == NawRukhsa::Spdx
                && tarjama
                    .rukhsa
                    .muarrif
                    .as_deref()
                    .is_none_or(|spdx| spdx.trim().is_empty())
            {
                return Err(talif(format!(
                    "entry {muarrif:?} claims an SPDX licence and names none"
                )));
            }
        }
        Ok(())
    }
}

/// The entries that are for one game, Steam id first and name second.
///
/// A Steam id is the registry's own key for a Steam game, so an entry carrying
/// the same one is that game and nothing else. The normalized name — the same
/// [`wahhid_ism`] the store keys `ism_muwahhad` on — is the fallback for a game
/// bought elsewhere, or an entry whose page never named the Steam id. The two
/// are not merged freely: for a game that has a Steam id, an entry carrying a
/// *different* one is a different Steam application whatever its title, so only
/// entries with no id at all are matched by name beside it. A game with no id
/// takes every entry its name matches.
#[must_use]
pub fn tarjamat_li_luba<'a>(
    fahras: &'a FahrasMujtama,
    steam_appid: Option<u32>,
    ism_muwahhad: &str,
) -> Vec<&'a Tarjama> {
    let bil_ism = |tarjama: &Tarjama| {
        !ism_muwahhad.is_empty()
            && (wahhid_ism(&tarjama.luba.ism) == ism_muwahhad
                || tarjama
                    .luba
                    .ism_arabi
                    .as_deref()
                    .is_some_and(|arabi| wahhid_ism(arabi) == ism_muwahhad))
    };

    let mut natija: Vec<&'a Tarjama> = Vec::new();
    match steam_appid {
        Some(appid) => {
            natija.extend(
                fahras
                    .tarjamat
                    .iter()
                    .filter(|tarjama| tarjama.steam_appid == Some(appid)),
            );
            natija.extend(
                fahras
                    .tarjamat
                    .iter()
                    .filter(|tarjama| tarjama.steam_appid.is_none() && bil_ism(tarjama)),
            );
        },
        None => natija.extend(fahras.tarjamat.iter().filter(|tarjama| bil_ism(tarjama))),
    }
    natija
}

/// Where the cached index lives under the data root.
#[must_use]
pub fn masar_makhbaa_mujtama(masarat: &Masarat) -> PathBuf {
    mujallad_makhbaa(masarat).join(ISM_MALAF_FAHRAS)
}

fn mujallad_makhbaa(masarat: &Masarat) -> PathBuf {
    masarat
        .makhbaa()
        .join(MUJALLAD_MAKHBAA)
        .join(MUJALLAD_FAHRAS)
}

fn masar_sijill(masarat: &Masarat) -> PathBuf {
    mujallad_makhbaa(masarat).join(ISM_MALAF_SIJILL)
}

/// The last time a source served the index, and which.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NajahJalbMujtama {
    /// When.
    pub waqt: Timestamp,
    /// The source, as the registry client names sources.
    pub masdar: String,
}

/// How one attempt to fetch the index ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum NatijatJalbMujtama {
    /// An index was served, read and cached.
    Najah {
        /// The source that served it.
        masdar: String,
    },
    /// No source answered.
    MustawdaGhayrMutah {
        /// What the last source said.
        sabab: String,
    },
    /// A source answered with something this build could not read.
    FahrasTalif {
        /// The source.
        masdar: String,
        /// Why the answer was refused.
        sabab: String,
    },
}

/// One attempt, with its time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuhawalatJalbMujtama {
    /// When the attempt was made.
    pub waqt: Timestamp,
    /// What it found.
    pub natija: NatijatJalbMujtama,
}

/// The refresh record beside the cached index.
///
/// Kept apart from the index file so the index stays byte-for-byte what the
/// source served, and so a stale answer can say when it was fetched rather
/// than leaving that to a file time a copied data directory does not keep.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillJalbMujtama {
    /// The record format version.
    #[serde(default)]
    pub isdar: u32,
    /// The last time a source served the index.
    #[serde(default)]
    pub akhir_najah: Option<NajahJalbMujtama>,
    /// The latest attempt, whatever it found.
    #[serde(default)]
    pub akhir_muhawala: Option<MuhawalatJalbMujtama>,
}

/// The cached index beside its record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FahrasMukhazzan {
    /// The index as it was last served.
    pub fahras: FahrasMujtama,
    /// When, and from where.
    pub sijill: SijillJalbMujtama,
}

impl FahrasMukhazzan {
    /// How old the cached index is at `al_aan`, or `None` when the record does
    /// not say — a record that was lost, or one written by a hand-placed copy.
    #[must_use]
    pub fn umr(&self, al_aan: Timestamp) -> Option<SignedDuration> {
        self.sijill
            .akhir_najah
            .as_ref()
            .map(|najah| al_aan.duration_since(najah.waqt))
    }

    /// Whether the cached index still counts as current at `al_aan`.
    #[must_use]
    pub fn hadith(&self, al_aan: Timestamp) -> bool {
        self.umr(al_aan).is_some_and(dakhil_al_nafidha)
    }
}

/// Whether an age falls inside the refresh window.
///
/// A negative age is a record stamped in the future — a clock set back since
/// the fetch — and is not current: the window is measured forward from the
/// fetch, and a fetch nothing can date has no window to be inside.
const fn dakhil_al_nafidha(umr: SignedDuration) -> bool {
    !umr.is_negative() && umr.as_secs() <= NAFIDHAT_FAHRAS_MUJTAMA.as_secs()
}

/// The cached index, when one is on disk and reads.
///
/// Synchronous and never a failure: this is what a library scan reads on the
/// thread it runs on, and a missing, oversized or unreadable cache is a scan
/// with no community marks rather than a scan that stops. Each of those is a
/// line in the log, because a cache this build wrote and cannot read back is
/// worth knowing about even when nothing depends on it.
#[must_use]
pub fn iqra_makhbaa(masarat: &Masarat) -> Option<FahrasMukhazzan> {
    let masar = masar_makhbaa_mujtama(masarat);
    let bayanat = match std::fs::metadata(&masar) {
        Ok(bayanat) => bayanat,
        Err(khata) if khata.kind() == std::io::ErrorKind::NotFound => return None,
        Err(khata) => {
            tracing::warn!(masar = %masar.display(), khata = %khata, "the cached community index could not be examined");
            return None;
        },
    };
    if !bayanat.is_file() || bayanat.len() > HADD_HAJM_FAHRAS_MUJTAMA {
        tracing::warn!(
            masar = %masar.display(),
            hajm = bayanat.len(),
            "the cached community index is not a file this build wrote, and is ignored"
        );
        return None;
    }
    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(khata) => {
            tracing::warn!(masar = %masar.display(), khata = %khata, "the cached community index could not be read");
            return None;
        },
    };
    let fahras = match FahrasMujtama::min_bayt(&bayt) {
        Ok(fahras) => fahras,
        Err(khata) => {
            tracing::warn!(masar = %masar.display(), khata = %khata, "the cached community index is unreadable and is ignored");
            return None;
        },
    };
    Some(FahrasMukhazzan {
        fahras,
        sijill: sijill_jalb(masarat),
    })
}

/// The refresh record, empty when there is none or it does not read.
#[must_use]
pub fn sijill_jalb(masarat: &Masarat) -> SijillJalbMujtama {
    let masar = masar_sijill(masarat);
    match std::fs::read(&masar) {
        Ok(bayt) => match serde_json::from_slice::<SijillJalbMujtama>(&bayt) {
            Ok(sijill) => sijill,
            Err(khata) => {
                tracing::warn!(
                    masar = %masar.display(),
                    khata = %khata,
                    "the community index refresh record is unreadable and is treated as empty"
                );
                SijillJalbMujtama::default()
            },
        },
        Err(khata) if khata.kind() == std::io::ErrorKind::NotFound => SijillJalbMujtama::default(),
        Err(khata) => {
            tracing::warn!(
                masar = %masar.display(),
                khata = %khata,
                "the community index refresh record could not be read and is treated as empty"
            );
            SijillJalbMujtama::default()
        },
    }
}

/// Where the index an answer holds came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AslFahrasMujtama {
    /// The cache, still within its window; no source was asked.
    Makhbaa {
        /// How old it is.
        umr: SignedDuration,
    },
    /// A source, just now.
    Masdar {
        /// Which, as the registry client names sources.
        masdar: String,
    },
    /// The cache, past its window, because every source refused.
    MakhbaaQadeem {
        /// How old it is, when the record says.
        umr: Option<SignedDuration>,
        /// What the last source said, or why the answer that came was refused.
        sabab: String,
    },
}

impl AslFahrasMujtama {
    /// Whether the answer is current: served now, or cached within the window.
    #[must_use]
    pub const fn hadith(&self) -> bool {
        matches!(self, Self::Makhbaa { .. } | Self::Masdar { .. })
    }

    /// One line for the log.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Makhbaa { umr } => {
                format!("community index served from the cache, {} old", saat(*umr))
            },
            Self::Masdar { masdar } => format!("community index fetched from {masdar}"),
            Self::MakhbaaQadeem { umr, sabab } => format!(
                "community index served stale from the cache ({}) because no source served a \
                 usable one: {sabab}",
                umr.map_or_else(
                    || "age unknown".to_owned(),
                    |umr| format!("{} old", saat(umr))
                )
            ),
        }
    }
}

/// An age in whole hours, for a log line.
fn saat(umr: SignedDuration) -> String {
    format!("{}h", umr.as_hours())
}

/// An index and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FahrasMujtamaMajlub {
    /// The index.
    pub fahras: FahrasMujtama,
    /// Its provenance.
    pub asl: AslFahrasMujtama,
}

/// The community index: from the cache while it is current, otherwise from the
/// first source in the chain that serves a readable one, and from the stale
/// cache when none does.
///
/// A fetched index is written under the data root and its refresh record
/// stamped `al_aan`, so the next call inside [`NAFIDHAT_FAHRAS_MUJTAMA`] asks
/// no source. A cache that cannot be written is logged and the fetched index is
/// still answered; nothing about the answer depends on the write.
///
/// # Errors
///
/// Only when there is nothing to answer with: no cached index at all, and
/// either [`KhataMustawda::FahrasMujtamaGhayrMutah`] because no source
/// answered, or whatever [`FahrasMujtama::min_bytes`] refused the one answer
/// that came.
///
/// [`FahrasMujtama::min_bytes`]: FahrasMujtama::min_bayt
pub async fn jalb_fahras_mujtama(
    silsila: &SilsilatMasadir,
    masarat: &Masarat,
    al_aan: Timestamp,
) -> NatijatMustawda<FahrasMujtamaMajlub> {
    let mukhazzan = {
        let masarat = masarat.clone();
        tokio::task::spawn_blocking(move || iqra_makhbaa(&masarat))
            .await
            .unwrap_or_else(|khata| {
                tracing::warn!(khata = %khata, "the community index cache read did not finish");
                None
            })
    };
    // Current: answered without a source being asked. Anything else on disk is
    // kept as the fallback for the fetch below.
    let mukhazzan = match mukhazzan {
        Some(mukhazzan) => match mukhazzan.umr(al_aan) {
            Some(umr) if dakhil_al_nafidha(umr) => {
                return Ok(FahrasMujtamaMajlub {
                    fahras: mukhazzan.fahras,
                    asl: AslFahrasMujtama::Makhbaa { umr },
                });
            },
            _ => Some(mukhazzan),
        },
        None => None,
    };

    let (natija, khata, sabab) = match silsila.jalb_maa_masdar(MASAR_FAHRAS_MUJTAMA).await {
        Ok((bayt, masdar)) => match FahrasMujtama::min_bayt(&bayt) {
            Ok(fahras) => {
                ila_makhbaa(masarat, bayt, masdar.clone(), al_aan).await;
                return Ok(FahrasMujtamaMajlub {
                    fahras,
                    asl: AslFahrasMujtama::Masdar { masdar },
                });
            },
            // A source that answers with something unreadable is recorded as
            // such rather than as unreachable: the two are different facts
            // about the registry, and the record is where the difference lives.
            Err(khata) => {
                let sabab = khata.to_string();
                (
                    NatijatJalbMujtama::FahrasTalif {
                        masdar: masdar.clone(),
                        sabab: sabab.clone(),
                    },
                    khata,
                    format!("{masdar}: {sabab}"),
                )
            },
        },
        Err(khata) => {
            let sabab = khata.to_string();
            (
                NatijatJalbMujtama::MustawdaGhayrMutah {
                    sabab: sabab.clone(),
                },
                KhataMustawda::FahrasMujtamaGhayrMutah {
                    sabab: sabab.clone(),
                },
                sabab,
            )
        },
    };
    sajjil(
        masarat,
        MuhawalatJalbMujtama {
            waqt: al_aan,
            natija,
        },
    )
    .await;

    match mukhazzan {
        Some(mukhazzan) => {
            let umr = mukhazzan.umr(al_aan);
            Ok(FahrasMujtamaMajlub {
                fahras: mukhazzan.fahras,
                asl: AslFahrasMujtama::MakhbaaQadeem { umr, sabab },
            })
        },
        None => Err(khata),
    }
}

/// Writes a served index and stamps the record, reporting a failure and no more.
async fn ila_makhbaa(masarat: &Masarat, bayt: Vec<u8>, masdar: String, al_aan: Timestamp) {
    let masarat = masarat.clone();
    let natija =
        tokio::task::spawn_blocking(move || uktub_makhbaa(&masarat, &bayt, masdar, al_aan)).await;
    match natija {
        Ok(Ok(())) => {},
        Ok(Err(khata)) => {
            tracing::warn!(khata = %khata, "the community index was not cached");
        },
        Err(khata) => {
            tracing::warn!(khata = %khata, "the community index cache task did not finish");
        },
    }
}

fn uktub_makhbaa(
    masarat: &Masarat,
    bayt: &[u8],
    masdar: String,
    al_aan: Timestamp,
) -> NatijatMustawda<()> {
    let masar = masar_makhbaa_mujtama(masarat);
    kitaba_dharra(&masar, bayt).map_err(|khata| KhataMustawda::KhataMalaf {
        masar: masar.clone(),
        amal: AMAL_KITABA,
        sabab: std::io::Error::other(khata.li_sijill()),
    })?;
    let mut sijill = sijill_jalb(masarat);
    sijill.akhir_najah = Some(NajahJalbMujtama {
        waqt: al_aan,
        masdar: masdar.clone(),
    });
    sijill.akhir_muhawala = Some(MuhawalatJalbMujtama {
        waqt: al_aan,
        natija: NatijatJalbMujtama::Najah { masdar },
    });
    uktub_sijill(masarat, &sijill)
}

/// Records an attempt that served nothing usable, reporting a failure and no
/// more: the attempt's outcome still reaches the caller either way.
async fn sajjil(masarat: &Masarat, muhawala: MuhawalatJalbMujtama) {
    let masarat = masarat.clone();
    let natija = tokio::task::spawn_blocking(move || {
        let mut sijill = sijill_jalb(&masarat);
        sijill.akhir_muhawala = Some(muhawala);
        uktub_sijill(&masarat, &sijill)
    })
    .await;
    match natija {
        Ok(Ok(())) => {},
        Ok(Err(khata)) => {
            tracing::warn!(khata = %khata, "the community index fetch attempt was not recorded");
        },
        Err(khata) => {
            tracing::warn!(khata = %khata, "the community index record task did not finish");
        },
    }
}

fn uktub_sijill(masarat: &Masarat, sijill: &SijillJalbMujtama) -> NatijatMustawda<()> {
    let masar = masar_sijill(masarat);
    let mubayyan = SijillJalbMujtama {
        isdar: ISDAR_SIJILL,
        ..sijill.clone()
    };
    let bayt = serde_json::to_vec_pretty(&mubayyan).map_err(|khata| KhataMustawda::KhataMalaf {
        masar: masar.clone(),
        amal: AMAL_KITABA,
        sabab: std::io::Error::other(khata),
    })?;
    kitaba_dharra(&masar, &bayt).map_err(|khata| KhataMustawda::KhataMalaf {
        masar: masar.clone(),
        amal: AMAL_KITABA,
        sabab: std::io::Error::other(khata.li_sijill()),
    })
}

const fn talif(sabab: String) -> KhataMustawda {
    KhataMustawda::FahrasMujtamaTalif { sabab }
}

/// Whether an identifier is in the alphabet the index's schema fixes:
/// `^[a-z0-9][a-z0-9-]*$`, and not absurdly long.
fn muarrif_salih(muarrif: &str) -> bool {
    !muarrif.is_empty()
        && muarrif.len() <= HADD_TUL_MUARRIF
        && muarrif
            .bytes()
            .next()
            .is_some_and(|awwal| awwal.is_ascii_lowercase() || awwal.is_ascii_digit())
        && muarrif
            .bytes()
            .all(|harf| harf.is_ascii_lowercase() || harf.is_ascii_digit() || harf == b'-')
}

/// Whether an address is one this product will show and offer to open: `https`,
/// within the length a listing's asset is held to, and free of whitespace and
/// control characters.
fn rabt_salih(rabt: &str) -> bool {
    rabt.len() > "https://".len()
        && rabt.len() <= HADD_TUL_RABT
        && !rabt
            .chars()
            .any(|harf| harf.is_whitespace() || harf.is_control())
        && rabt
            .get(.."https://".len())
            .is_some_and(|badiya| badiya.eq_ignore_ascii_case("https://"))
}

/// Whether a date is `YYYY-MM-DD` in shape.
fn tareekh_salih(tareekh: &str) -> bool {
    tareekh.len() == 10
        && tareekh
            .bytes()
            .enumerate()
            .all(|(mawqi, harf)| match mawqi {
                4 | 7 => harf == b'-',
                _ => harf.is_ascii_digit(),
            })
}

/// The lowercased host of an address the index carries, when it has one.
fn mudif_rabt(rabt: &str) -> Option<String> {
    reqwest::Url::parse(rabt)
        .ok()
        .and_then(|rabt| rabt.host_str().map(str::to_ascii_lowercase))
}

#[cfg(test)]
mod fuhus {
    #![allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "a test reports failure by panicking; the lints are written for library \
                  code, and honouring them here would mean a test that cannot fail"
    )]

    use super::{
        FahrasMujtama, HADD_HAJM_FAHRAS_MUJTAMA, HalatTarjamaMujtama, MudifTarjama, NawRukhsa,
        NawTanzeelat, TaghtiyaMujtama, TareeqaMujtama, TawzeeTarjama, tarjamat_li_luba,
    };
    use crate::khata::KhataMustawda;

    /// A trimmed copy of the live index: three teams, seven entries, two of
    /// them for one Steam application, one with no Steam id at all.
    const AYYINA: &str = include_str!("../tests/mujtama/tarjamat.json");

    fn fahras() -> FahrasMujtama {
        match FahrasMujtama::min_bayt(AYYINA.as_bytes()) {
            Ok(fahras) => fahras,
            Err(khata) => panic!("the fixture index did not parse: {khata}"),
        }
    }

    fn muarrifat(tarjamat: &[&super::Tarjama]) -> Vec<String> {
        tarjamat
            .iter()
            .map(|tarjama| tarjama.muarrif.clone())
            .collect()
    }

    #[test]
    fn al_ayyina_tuqra_bi_kull_haql_yuqra() {
        let fahras = fahras();
        assert_eq!(fahras.isdar, 1);
        assert!(fahras.waqt_lahza().is_some());
        assert_eq!(fahras.firaq.len(), 3);
        assert_eq!(fahras.tarjamat.len(), 7);

        let hesham = fahras
            .tarjamat
            .iter()
            .find(|tarjama| tarjama.muarrif == "007-first-light-hesham")
            .expect("the fixture carries the Nexus entry");
        assert_eq!(hesham.steam_appid, Some(3_768_760));
        assert_eq!(hesham.mudif, MudifTarjama::Nexusmods);
        assert_eq!(hesham.taghtiya, TaghtiyaMujtama::Kamila);
        assert_eq!(hesham.tareeqa, TareeqaMujtama::GhayrMusarraha);
        assert_eq!(hesham.rukhsa.naw, NawRukhsa::GhayrMusarraha);
        assert_eq!(hesham.tawzee, TawzeeTarjama::Majjani);
        assert_eq!(hesham.hala, HalatTarjamaMujtama::Nashita);
        assert_eq!(hesham.tanzeelat, Some(35_452));
        assert_eq!(hesham.tanzeelat_naw, Some(NawTanzeelat::Tanzeelat));
        assert_eq!(hesham.tahaqquq.waqt, "2026-09-11");
        let fariq = fahras
            .fariq("hesham")
            .expect("the team the entry names is listed");
        assert_eq!(fariq.ism_arabi.as_deref(), Some("تعريبات هشام"));

        let balatro = fahras
            .tarjamat
            .iter()
            .find(|tarjama| tarjama.muarrif == "balatro-bloodyarz")
            .expect("the fixture carries the SPDX entry");
        assert_eq!(balatro.rukhsa.naw, NawRukhsa::Spdx);
        assert!(balatro.rukhsa.muarrif.is_some());
        assert!(balatro.fariq.is_none());
    }

    #[test]
    fn al_mutabaqa_bil_appid_awwalan_thumma_bil_ism() {
        let fahras = fahras();

        let bil_appid = tarjamat_li_luba(&fahras, Some(3_768_760), "007firstlight");
        assert_eq!(
            muarrifat(&bil_appid),
            ["007-first-light-hesham", "007-first-light-play-in-arabic"]
        );

        // A game with no Steam id — a copy from another store — is matched on
        // its name, and takes an entry that does carry a Steam id.
        let bil_ism = tarjamat_li_luba(&fahras, None, "blackmesa");
        assert_eq!(muarrifat(&bil_ism), ["black-mesa-unofficial-arabic"]);

        // A Steam game whose id no entry carries still takes an entry that has
        // no id at all and the same name.
        let bila_appid = tarjamat_li_luba(&fahras, Some(999), "am2rmetroid2fanremake");
        assert_eq!(
            muarrifat(&bila_appid),
            ["am2r-metroid-2-fan-remake-eternal-dream"]
        );

        // But never an entry carrying a *different* Steam id: same title, other
        // application.
        let mukhtalif = tarjamat_li_luba(&fahras, Some(999), "007firstlight");
        assert!(mukhtalif.is_empty());

        // Nothing matches an empty name, whatever the index holds.
        assert!(tarjamat_li_luba(&fahras, None, "").is_empty());
    }

    #[test]
    fn al_mudifun_hum_mudifu_al_rawabit() {
        let mudifun = fahras().mudifun();
        assert!(mudifun.contains("www.nexusmods.com"));
        assert!(mudifun.contains("steamcommunity.com"));
        assert!(mudifun.contains("etrdream.com"));
        assert!(
            mudifun
                .iter()
                .all(|mudif| mudif == &mudif.to_ascii_lowercase())
        );
    }

    #[test]
    fn isdar_majhul_marfud() {
        let nass = AYYINA.replacen("\"isdar\": 1,", "\"isdar\": 2,", 1);
        assert_ne!(
            nass, AYYINA,
            "the fixture's schema stamp was not found to change"
        );
        match FahrasMujtama::min_bayt(nass.as_bytes()) {
            Err(KhataMustawda::FahrasMujtamaTalif { sabab }) => {
                assert!(sabab.contains("schema 2"), "{sabab}");
            },
            akhar => panic!("a schema this build does not read was accepted: {akhar:?}"),
        }
    }

    #[test]
    fn al_hajm_al_mufrit_marfud_qabl_al_qira() {
        let hadd = usize::try_from(HADD_HAJM_FAHRAS_MUJTAMA).expect("the cap fits a usize");
        let kabir = vec![b' '; hadd + 1];
        match FahrasMujtama::min_bayt(&kabir) {
            Err(KhataMustawda::FahrasMujtamaKabir {
                hajm,
                hadd: muallan,
            }) => {
                assert_eq!(hajm, HADD_HAJM_FAHRAS_MUJTAMA + 1);
                assert_eq!(muallan, HADD_HAJM_FAHRAS_MUJTAMA);
            },
            akhar => panic!("an index over the cap was not refused as such: {akhar:?}"),
        }
    }

    #[test]
    fn al_huqul_al_majhula_tutasamah_wa_al_qiyam_al_majhula_tuqra_majhul() {
        let nass = AYYINA
            .replacen(
                "\"mudif\": \"nexusmods\"",
                "\"mudif\": \"modrinth\", \"lawn\": \"azraq\"",
                1,
            )
            .replacen(
                "\"taghtiya\": \"kamila\"",
                "\"taghtiya\": \"sawt_faqat\"",
                1,
            );
        let fahras = match FahrasMujtama::min_bayt(nass.as_bytes()) {
            Ok(fahras) => fahras,
            Err(khata) => {
                panic!("a field and a value this build does not know were refused: {khata}")
            },
        };
        let hesham = fahras
            .tarjamat
            .iter()
            .find(|tarjama| tarjama.muarrif == "007-first-light-hesham")
            .expect("the edited entry is still there");
        assert_eq!(hesham.mudif, MudifTarjama::Majhul);
        assert_eq!(hesham.taghtiya, TaghtiyaMujtama::Majhul);
    }

    #[test]
    fn ma_yuqra_yuhaqqaq() {
        let fariq_majhul = AYYINA.replacen("\"fariq\": \"hesham\"", "\"fariq\": \"la-ahad\"", 1);
        assert!(matches!(
            FahrasMujtama::min_bayt(fariq_majhul.as_bytes()),
            Err(KhataMustawda::FahrasMujtamaTalif { .. })
        ));

        let rabt_http = AYYINA.replacen(
            "\"rabt\": \"https://www.nexusmods.com/007firstlight/mods/11\"",
            "\"rabt\": \"http://www.nexusmods.com/007firstlight/mods/11\"",
            1,
        );
        assert_ne!(rabt_http, AYYINA);
        assert!(matches!(
            FahrasMujtama::min_bayt(rabt_http.as_bytes()),
            Err(KhataMustawda::FahrasMujtamaTalif { .. })
        ));

        let mukarrar = AYYINA.replacen(
            "\"muarrif\": \"007-first-light-play-in-arabic\"",
            "\"muarrif\": \"007-first-light-hesham\"",
            1,
        );
        assert_ne!(mukarrar, AYYINA);
        assert!(matches!(
            FahrasMujtama::min_bayt(mukarrar.as_bytes()),
            Err(KhataMustawda::FahrasMujtamaTalif { .. })
        ));

        let tareekh = AYYINA.replacen(
            "\"waqt_alnashr\": \"2026-05-27\"",
            "\"waqt_alnashr\": \"May 2026\"",
            1,
        );
        assert_ne!(tareekh, AYYINA);
        assert!(matches!(
            FahrasMujtama::min_bayt(tareekh.as_bytes()),
            Err(KhataMustawda::FahrasMujtamaTalif { .. })
        ));

        assert!(matches!(
            FahrasMujtama::min_bayt(b"{\"isdar\": 1}"),
            Err(KhataMustawda::FahrasMujtamaTalif { .. })
        ));
    }
}
