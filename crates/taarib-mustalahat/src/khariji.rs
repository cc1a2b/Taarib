//! الرقعة الخارجية — a patch somebody else made, that Taarib lists and installs
//! but never built.
//!
//! Most of the Arabic game translations that exist are finished work on engines
//! Taarib has no adapter for, delivered by replacing or overlaying the game's
//! own files. Taarib cannot compile one of those and must not pretend to. What
//! it can do is fetch it from the author, verify it against a pin, install it
//! through the same backup machinery every other install goes through so it can
//! always be undone, and credit whoever made it — on the card, on the detail
//! screen and in the catalogue, in both languages.
//!
//! This is the installable escalation of the community index in
//! `taarib_mustawda::mujtama`, which records the same kind of work and only
//! opens its page. An entry here is one the owner additionally pinned, so a
//! client can be handed the bytes rather than a link.
//!
//! **Nothing here carries a byte.** An artifact is an address, a size and a
//! hash; the bytes live on the author's endpoint and reach a machine only
//! through the download path, which checks them against [`QitaatTanzeel::sha256`]
//! before anything is written. That is also what keeps a third-party entry
//! outside the asset gate's certificate: the certificate counts what a *package*
//! contains, a package is assembled out of `taarib_tarqee::bawwaba::MuhtawaMasmuh`,
//! and none of these types can become one.

use serde::{Deserialize, Serialize};

use crate::luba::LubaId;
use crate::ruqaa::{IdhnMasdar, MasdarKhariji, RuqaaId};

/// A patch somebody else made, that Taarib lists and installs but never built.
///
/// The whole record lives here rather than in a sealed package, because there is
/// no sealed package: nothing in this entry was compiled by Taarib, so there is
/// no container to carry its metadata and no signature of the maker's over it.
/// The registry index is the only place it exists, which is why the index
/// carries the entry whole rather than a summary pointing at a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct RuqaaKharijiya {
    /// The lineage, minted by Taarib. The work is somebody else's; the identity
    /// this catalogue lists it under is Taarib's own.
    pub id: RuqaaId,
    /// The title, as the work is known.
    pub unwan: String,
    /// The game it patches.
    pub luba: LubaId,
    /// The game as each launcher names it, so one entry covers Steam, the
    /// Rockstar launcher and Epic rather than needing three.
    pub hawiyat_manassa: Vec<String>,
    /// Game builds this patch supports, e.g. `["1311", "1436", "1491"]`.
    pub abniya: Vec<String>,
    /// Who made it, where, under what licence, and what permits republishing it.
    pub masdar: MasdarKhariji,
    /// The team name, when the work is a team's.
    pub fariq: Option<String>,
    /// The version the author released, in the author's own numbering.
    pub isdar: String,
    /// Everything fetched, each pinned.
    pub qitaa: Vec<QitaatTanzeel>,
    /// What the install writes and what it must clear first.
    pub takhtit: TakhtitKhariji,
    /// Safety sentences shown before install, both languages.
    pub tahdheerat: Vec<TahdheerKhariji>,
    /// Mirroring is off unless the permission covers it and the owner enabled
    /// it, so the absent value is the safe one and an entry written before this
    /// field existed reads as fetch-from-the-author.
    #[serde(default)]
    pub mira: HalatMira,
    /// The work's own home page, which is not the address its author lives at.
    ///
    /// [`MasdarKhariji::rabt`] points at whoever made it, because that is where
    /// a credit has to lead. A reader who wants the patch's own instructions,
    /// changelog and screenshots wants somewhere else, and collapsing the two
    /// sends one of those two readers to the wrong place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rabt_safha: Option<String>,
    /// The author's own metadata feed, when they publish one.
    ///
    /// What a re-pin starts from. A version the owner has not pinned is not
    /// installable, so somebody has to go and read what the author released;
    /// the alternative to recording where they announce it is rediscovering the
    /// address on every release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rabt_tahdith: Option<String>,
}

/// One fetched artifact, pinned by hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct QitaatTanzeel {
    /// The file name the artifact is stored and reported under.
    pub ism: String,
    /// The author's own endpoint. Never a mirror by default.
    pub rabt: String,
    /// The size in bytes, as measured when the pin was taken.
    ///
    /// Carried beside the digest rather than instead of it: a size alone is
    /// trivially matched, and a digest alone lets a server stream until the disk
    /// fills before anything has a chance to disagree.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub hajm: u64,
    /// Lowercase hex sha256. A mismatch is refused by name, never accepted.
    ///
    /// sha256 rather than the BLAKE3 every Taarib-built artifact is pinned with,
    /// because this pin is taken over somebody else's file: the number a
    /// maintainer can check against the author's own publication is the one
    /// worth recording.
    pub sha256: String,
    /// Which game builds need this artifact; empty means all of them.
    pub abniya: Vec<String>,
}

impl QitaatTanzeel {
    /// Whether this artifact applies to one installed build.
    #[must_use]
    pub fn yantabiq(&self, bina: &str) -> bool {
        self.abniya.is_empty() || self.abniya.iter().any(|mudam| mudam == bina)
    }

    /// Whether another pin names the same file with the same size and digest.
    ///
    /// The hex is compared case-insensitively. A registry row and a hand-written
    /// entry disagree about letter case far more often than about bytes, and a
    /// refusal over case would teach whoever met it to stop reading refusals.
    #[must_use]
    pub fn yutabiq(&self, akhar: &Self) -> bool {
        self.ism == akhar.ism
            && self.hajm == akhar.hajm
            && self.sha256.eq_ignore_ascii_case(&akhar.sha256)
    }

    /// Whether the pin is a digest at all: 64 lowercase hex characters.
    ///
    /// Checked as a shape before anything is fetched, because a pin that cannot
    /// match any file turns the download's verification into a refusal nobody
    /// can act on — the bytes arrived, they hash to something, and the entry
    /// declared a string that was never a hash.
    #[must_use]
    pub fn basma_salima(&self) -> bool {
        self.sha256.len() == 64 && self.sha256.bytes().all(|harf| harf.is_ascii_hexdigit())
    }
}

/// What the install writes and what it must clear first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TakhtitKhariji {
    /// Paths, relative to the game root, the artifacts unpack to.
    pub yaktub: Vec<String>,
    /// Paths the author's instructions say to remove before installing.
    ///
    /// Every one is backed up before removal and restored byte-identical on
    /// uninstall. That is the whole difference between this install and the one
    /// the author ships: their instructions say delete, and a deleted file is
    /// gone.
    pub yahdhif: Vec<String>,
}

impl TakhtitKhariji {
    /// Whether one unpacked path falls inside what this layout declares.
    ///
    /// An archive is untrusted input, and an entry landing outside the declared
    /// layout is a file nobody reviewed arriving in somebody's game. Matching is
    /// on whole path segments — `lml` covers `lml/RTEA/x.yldb` and does not
    /// cover `lmlx` — because a prefix match on raw text would let
    /// `version.dll.bak` pass as `version.dll`.
    #[must_use]
    pub fn yasmah(&self, nisbi: &str) -> bool {
        self.yaktub.iter().any(|musallam| {
            let musallam = musallam.trim_matches('/');
            nisbi == musallam
                || nisbi
                    .strip_prefix(musallam)
                    .is_some_and(|baqi| baqi.starts_with('/'))
        })
    }
}

/// One safety sentence shown before an install, in both languages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TahdheerKhariji {
    /// The Arabic wording, which is the one most readers will see.
    pub arabi: String,
    /// The English wording.
    pub injilizi: String,
}

/// Where a client fetches a third-party artifact from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum HalatMira {
    /// Fetched from the author, always. The default.
    #[default]
    MinAlmuallif,
    /// Mirrored on the registry.
    ///
    /// Only reachable when the permission record explicitly covers mirroring and
    /// the owner turned it on for this entry. Redistribution and mirroring are
    /// separate grants: an author who says "put it in your launcher" has not
    /// thereby said "host my file", and a registry that read the first as the
    /// second would be making their bandwidth decision for them.
    MinAlsijill {
        /// The registry's own endpoint for the artifact.
        rabt: String,
    },
}

impl HalatMira {
    /// Whether this entry would fetch from anywhere but the author.
    #[must_use]
    pub const fn min_sijill(&self) -> bool {
        matches!(self, Self::MinAlsijill { .. })
    }
}

/// Why a third-party entry may not be published.
///
/// One reason at a time, in the order they are checked: the first unanswered
/// question is the one a maintainer has to answer, and the others only exist
/// because of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SababRafdKhariji {
    /// Nothing establishes a right to redistribute the work.
    BilaIdhn,
    /// A licence was read, and a licence is not what this needs.
    IdhnRukhsaFaqat,
    /// The author's grant is recorded with no statement of where it was given.
    BayanFarigh,
    /// The entry is mirrored and the permission record does not cover mirroring.
    MiraBilaIdhn,
    /// The entry is mirrored and names no address to serve the copy from.
    MiraBilaRabt,
}

impl SababRafdKhariji {
    /// The refusal, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::BilaIdhn => {
                "لا يثبت لهذا العمل إذنٌ بإعادة النشر، والمستودع لا ينشر عمل غيره بلا إذنٍ مكتوب من صاحبه."
            },
            Self::IdhnRukhsaFaqat => {
                "الرخصة وحدها لا تكفي لعملٍ خارجيّ كامل؛ يلزم إذنٌ مكتوب من صاحب العمل نفسه."
            },
            Self::BayanFarigh => {
                "الإذن مسجَّل بلا بيان: اكتب أين أعطاه صاحبه ومتى، ليتمكّن القارئ من التحقّق منه."
            },
            Self::MiraBilaIdhn => {
                "المِرآة مطلوبة والإذن المسجَّل لا يشمل استضافة الملفّ؛ إعادة النشر والمِرآة إذنان منفصلان."
            },
            Self::MiraBilaRabt => "المِرآة مطلوبة بلا عنوانٍ تُخدَم منه.",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::BilaIdhn => {
                "Nothing establishes a right to redistribute this work, and the registry does not \
                 publish somebody else's work without written permission from whoever made it."
            },
            Self::IdhnRukhsaFaqat => {
                "A licence alone is not enough for a whole outside work; this needs written \
                 permission from the author themselves."
            },
            Self::BayanFarigh => {
                "The permission is recorded with no statement: write where the author gave it and \
                 when, so a reader can go and check it."
            },
            Self::MiraBilaIdhn => {
                "Mirroring is asked for and the recorded permission does not cover hosting the \
                 file; redistribution and mirroring are separate grants."
            },
            Self::MiraBilaRabt => "Mirroring is asked for with no address to serve the copy from.",
        }
    }
}

impl RuqaaKharijiya {
    /// Why this entry may not be published, or [`None`] when nothing stops it.
    ///
    /// A licence is deliberately not enough here, and that is the one place this
    /// rule is stricter than [`MasdarKhariji::yajuz_nashruh`]. That one answers
    /// for a *translation somebody imported into their own project*, where a
    /// permissive licence over the text really does settle the question. This
    /// one answers for a whole finished work republished under its author's
    /// name, fetched and installed by strangers on the registry's word — and the
    /// ordinary case is a repository that declares no licence at all. A licence
    /// file nobody here read cannot carry that, so the only thing that does is
    /// the author saying so, in words a reader can go and check.
    #[must_use]
    pub fn sabab_rafd(&self) -> Option<SababRafdKhariji> {
        match &self.masdar.idhn {
            IdhnMasdar::LamYuthbat => return Some(SababRafdKhariji::BilaIdhn),
            IdhnMasdar::Rukhsa => return Some(SababRafdKhariji::IdhnRukhsaFaqat),
            IdhnMasdar::Katabi { bayan } if bayan.trim().is_empty() => {
                return Some(SababRafdKhariji::BayanFarigh);
            },
            IdhnMasdar::Katabi { .. } => {},
        }
        match &self.mira {
            HalatMira::MinAlmuallif => None,
            HalatMira::MinAlsijill { .. } if !self.masdar.yasmah_bilmira => {
                Some(SababRafdKhariji::MiraBilaIdhn)
            },
            HalatMira::MinAlsijill { rabt } if rabt.trim().is_empty() => {
                Some(SababRafdKhariji::MiraBilaRabt)
            },
            HalatMira::MinAlsijill { .. } => None,
        }
    }

    /// Whether this entry may be published.
    #[must_use]
    pub fn yajuz_nashruha(&self) -> bool {
        self.sabab_rafd().is_none()
    }

    /// Whether the registry may host this entry's bytes.
    ///
    /// Two answers have to agree: the permission on record has to cover
    /// mirroring at all, and the owner has to have turned it on for this entry.
    /// Either alone leaves the file on the author's endpoint, which is the
    /// default and the state every entry starts in.
    #[must_use]
    pub const fn yajuz_mira(&self) -> bool {
        self.masdar.yasmah_bilmira && self.mira.min_sijill()
    }

    /// The artifacts that apply to one installed build, in the declared order.
    ///
    /// RTEA is the shape this exists for: `update.zip` is every build and
    /// `extra.zip` is 1311 and 1436 alone, so fetching `extra.zip` into 1491
    /// would write a loader set that build does not take.
    #[must_use]
    pub fn qitaa_li_bina<'a>(&'a self, bina: &str) -> Vec<&'a QitaatTanzeel> {
        self.qitaa
            .iter()
            .filter(|qitaa| qitaa.yantabiq(bina))
            .collect()
    }

    /// Whether this entry declares support for one installed build.
    ///
    /// An empty list is "every build", for an entry whose author never said.
    #[must_use]
    pub fn yadam_bina(&self, bina: &str) -> bool {
        self.abniya.is_empty() || self.abniya.iter().any(|mudam| mudam == bina)
    }

    /// The author and team as one credit line, in the author's own spelling.
    ///
    /// Taarib never presents somebody else's work as its own, and the cheapest
    /// way to keep that true in every place the entry is rendered is for there
    /// to be one place the credit is composed.
    #[must_use]
    pub fn nasab(&self) -> String {
        match &self.fariq {
            Some(fariq) => format!("{} · {fariq}", self.masdar.ism),
            None => self.masdar.ism.clone(),
        }
    }

    /// The first artifact whose pin is not a digest, when the entry has one.
    #[must_use]
    pub fn qitaa_bila_basma(&self) -> Option<&QitaatTanzeel> {
        self.qitaa.iter().find(|qitaa| !qitaa.basma_salima())
    }
}

#[cfg(test)]
mod ikhtibarat_khariji {
    use super::{
        HalatMira, QitaatTanzeel, RuqaaKharijiya, SababRafdKhariji, TahdheerKhariji, TakhtitKhariji,
    };
    use crate::luba::{LubaId, MasdarLuba};
    use crate::ruqaa::{IdhnMasdar, MasdarKhariji, RukhsaRuqaa, RuqaaId};

    fn madkhal(idhn: IdhnMasdar, yasmah_bilmira: bool, mira: HalatMira) -> RuqaaKharijiya {
        RuqaaKharijiya {
            id: RuqaaId::jadeeda(),
            unwan: "RTEA".to_owned(),
            luba: LubaId::min_masdar(&MasdarLuba::Steam(1_174_180), "Red Dead Redemption 2"),
            hawiyat_manassa: vec!["Red Dead Redemption 2".to_owned()],
            abniya: vec!["1491".to_owned()],
            masdar: MasdarKhariji {
                ism: "Emad Adel".to_owned(),
                rabt: "https://github.com/emadadeldev/rtea".to_owned(),
                rukhsa: RukhsaRuqaa::MilkiyaKhassa,
                idhn,
                yasmah_bilmira,
            },
            fariq: Some("Redemption Team".to_owned()),
            isdar: "1.7".to_owned(),
            qitaa: vec![QitaatTanzeel {
                ism: "update.zip".to_owned(),
                rabt: "https://rt.emadadeldev.workers.dev/?lml-update".to_owned(),
                hajm: 25_578_943,
                sha256: "ffd54019f146b8db8a02d6413c592222ae9de397c522631d3b65fe882b3e1343"
                    .to_owned(),
                abniya: Vec::new(),
            }],
            takhtit: TakhtitKhariji {
                yaktub: vec!["lml".to_owned(), "dinput8.dll".to_owned()],
                yahdhif: vec!["version.dll".to_owned()],
            },
            tahdheerat: vec![TahdheerKhariji {
                arabi: "طور القصة فقط.".to_owned(),
                injilizi: "Story mode only.".to_owned(),
            }],
            mira,
            rabt_safha: None,
            rabt_tahdith: None,
        }
    }

    const fn katabi() -> IdhnMasdar {
        IdhnMasdar::Katabi {
            bayan: String::new(),
        }
    }

    fn bi_bayan() -> IdhnMasdar {
        IdhnMasdar::Katabi {
            bayan: "granted on X, 2026-09-21, https://x.example/post/1".to_owned(),
        }
    }

    /// The ordinary case, and the one the seed ships in: a written grant whose
    /// statement nobody has filled in is not a grant anybody can go and check.
    #[test]
    fn bayan_farigh_yamna_alnashr() {
        let bilaa = madkhal(katabi(), false, HalatMira::MinAlmuallif);
        assert_eq!(bilaa.sabab_rafd(), Some(SababRafdKhariji::BayanFarigh));
        assert!(!bilaa.yajuz_nashruha());
    }

    /// A licence settles an imported string table and does not settle a whole
    /// republished work, so `Rukhsa` is named rather than quietly accepted.
    #[test]
    fn rukhsa_wahdaha_la_takfi() {
        let bi_rukhsa = madkhal(IdhnMasdar::Rukhsa, false, HalatMira::MinAlmuallif);
        assert_eq!(
            bi_rukhsa.sabab_rafd(),
            Some(SababRafdKhariji::IdhnRukhsaFaqat)
        );
        let bilaa = madkhal(IdhnMasdar::LamYuthbat, false, HalatMira::MinAlmuallif);
        assert_eq!(bilaa.sabab_rafd(), Some(SababRafdKhariji::BilaIdhn));
    }

    #[test]
    fn idhn_katabi_bi_bayan_yasmah() {
        let madkhal = madkhal(bi_bayan(), false, HalatMira::MinAlmuallif);
        assert_eq!(madkhal.sabab_rafd(), None);
        assert!(madkhal.yajuz_nashruha());
        assert!(!madkhal.yajuz_mira());
        assert_eq!(madkhal.nasab(), "Emad Adel · Redemption Team");
    }

    /// Mirroring needs both halves: a grant that covers it, and the owner
    /// turning it on for this entry. Neither alone moves the bytes.
    #[test]
    fn almira_tahtaj_idhnan_wa_tashgheelan() {
        let sijill = HalatMira::MinAlsijill {
            rabt: "khariji/rtea/update.zip".to_owned(),
        };

        let bila_idhn = madkhal(bi_bayan(), false, sijill.clone());
        assert_eq!(bila_idhn.sabab_rafd(), Some(SababRafdKhariji::MiraBilaIdhn));
        assert!(!bila_idhn.yajuz_mira());

        let mutfaa = madkhal(bi_bayan(), true, HalatMira::MinAlmuallif);
        assert!(mutfaa.yajuz_nashruha());
        assert!(
            !mutfaa.yajuz_mira(),
            "a grant that covers mirroring does not by itself turn it on"
        );

        let mushaghghala = madkhal(bi_bayan(), true, sijill);
        assert_eq!(mushaghghala.sabab_rafd(), None);
        assert!(mushaghghala.yajuz_mira());
    }

    #[test]
    fn mira_bila_rabt_marfuda() {
        let madkhal = madkhal(
            bi_bayan(),
            true,
            HalatMira::MinAlsijill {
                rabt: String::new(),
            },
        );
        assert_eq!(madkhal.sabab_rafd(), Some(SababRafdKhariji::MiraBilaRabt));
    }

    /// An artifact with no build list is for every build; one with a list is for
    /// exactly those, which is how RTEA's `extra.zip` reaches 1311 and 1436 and
    /// not 1491.
    #[test]
    fn qitaa_tatba_abniyataha() {
        let mut madkhal = madkhal(bi_bayan(), false, HalatMira::MinAlmuallif);
        madkhal.abniya = vec!["1311".to_owned(), "1436".to_owned(), "1491".to_owned()];
        madkhal.qitaa.push(QitaatTanzeel {
            ism: "extra.zip".to_owned(),
            rabt: "https://rt.emadadeldev.workers.dev/?lml-extra".to_owned(),
            hajm: 1_541_075,
            sha256: "82d3008d4d77066a4336a853861603ef50b3c5d64d4cfe45f52236156ea41715".to_owned(),
            abniya: vec!["1311".to_owned(), "1436".to_owned()],
        });
        assert_eq!(madkhal.qitaa_li_bina("1311").len(), 2);
        assert_eq!(madkhal.qitaa_li_bina("1491").len(), 1);
        assert!(madkhal.yadam_bina("1436"));
        assert!(!madkhal.yadam_bina("1207"));
    }

    /// The declared layout is matched on whole segments, so a file whose name
    /// merely starts with a permitted one does not pass as it.
    #[test]
    fn takhtit_yutabiq_maqati_kamila() {
        let madkhal = madkhal(bi_bayan(), false, HalatMira::MinAlmuallif);
        assert!(madkhal.takhtit.yasmah("lml"));
        assert!(madkhal.takhtit.yasmah("lml/RTEA/Subtitles/texts/a.yldb"));
        assert!(!madkhal.takhtit.yasmah("lmlx/a.yldb"));
        assert!(!madkhal.takhtit.yasmah("dinput8.dll.bak"));
    }

    /// A pin that is not a digest cannot match any file, so it is caught as a
    /// shape before a byte is fetched.
    #[test]
    fn basma_ghayr_salima_tuktashaf() {
        let mut madkhal = madkhal(bi_bayan(), false, HalatMira::MinAlmuallif);
        assert!(madkhal.qitaa_bila_basma().is_none());
        if let Some(qitaa) = madkhal.qitaa.first_mut() {
            qitaa.sha256.truncate(40);
        }
        assert_eq!(
            madkhal.qitaa_bila_basma().map(|qitaa| qitaa.ism.as_str()),
            Some("update.zip")
        );
    }
}
