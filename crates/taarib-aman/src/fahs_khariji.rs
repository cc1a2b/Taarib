//! The gate for a patch Taarib did not build: the only place a third-party
//! install authorisation is minted.
//!
//! [`crate::fahs::fahs`] answers "may this sealed package be installed?" and
//! leans on a signature to do it. There is no signature anywhere in a
//! third-party patch, and there never will be — adding one would mean Taarib
//! signing somebody else's work, which is the one thing `khatm` exists to make
//! impossible. So this gate asks a different set of questions and mints a
//! different permit, and the install path for third-party patches takes that
//! permit and no other.
//!
//! The questions, in the order a refusal is most worth reaching first:
//!
//! 1. **The product statement.** Unacknowledged is unacknowledged whoever wrote
//!    the patch.
//! 2. **Anti-cheat**, through the same scan and the same outright refusal as the
//!    signed path — including the refusal when the scan could not run, because
//!    an unread store catalogue produces exactly the report a clean game does.
//! 3. **Permission from the author.** A third-party entry whose permission is
//!    not a written grant is refused here. A licence file is not a grant and the
//!    absence of one is not a permissive licence.
//! 4. **Mirroring.** `HalatMira::MinAlsijill` is refused unless the recorded
//!    permission covers redistribution; the author's own endpoint is the
//!    default and nothing turns it off silently.
//! 5. **The warnings.** Every entry carries its author's safety sentences, and a
//!    game with an online mode gets one more that this gate owns and writes
//!    itself. Nothing is minted until the person has acknowledged the list they
//!    were shown.
//!
//! ## Why the online warning is the gate's and not the entry's
//!
//! Red Dead Redemption 2's story mode has no anti-cheat, so step 2 clears it and
//! the patch is perfectly safe to install. Red Dead Online is a different
//! program sharing the same files, and it does have anti-cheat. An entry author
//! can be trusted to describe their own patch; they cannot be relied on to warn
//! about the account consequences of a mode they never tested. So the sentence
//! is [`tahdheer_laab_shabaki`], it is composed here, it is added to whatever
//! the entry says whenever the network scan finds an online mode, and the permit
//! carries it — so the install records what the person was actually told rather
//! than a sentence reconstructed afterwards.

use std::path::{Path, PathBuf};

use taarib_mustalahat::luba::LubaId;

use crate::idhn::IdhnTathbeetKhariji;
use crate::iqrar::{SijillIqrar, yahtaj_iqrar};
use crate::kashf_himaya::{HalatMatjar, IjmaaHimaya, ifhas_himaya_bi_qiraa, mahmiya};
use crate::kashf_shabaka::{IjmaaShabaka, ifhas_shabaka_bi_qiraa};
use crate::kharijiya::{RuqaaKharijiya, TahdheerKhariji, idhn_katabi, mira_masmuha};
use crate::matjar::QiraatMatjar;

/// The sentence this gate adds for a game that is also played online.
///
/// Not an entry author's to write and not a per-game string in a table. The
/// facts it states are about the *product*: Taarib's third-party installs
/// replace files the game ships, a title with an online mode runs the same files
/// there, and the account rather than the installation is what is at risk. One
/// sentence, composed once, carried into the permit and recorded by the install.
#[must_use]
pub fn tahdheer_laab_shabaki(ism_luba: &str) -> TahdheerKhariji {
    TahdheerKhariji {
        arabi: format!(
            "هذه الرقعة لطور القصّة في {ism_luba} وحده. تعمل الأطوار الشبكية بملفّات اللعبة \
             نفسها التي تُستبدل هنا، ومكافحة الغشّ فيها تفحص هذه الملفّات: الدخول إلى الطور \
             الشبكي والملفّات معدَّلة يعرّض حسابك للحظر، والحظر يقع على الحساب لا على النسخة. \
             أزِل التعريب قبل اللعب شبكيًّا — تُعيده الإزالة إلى حاله بايتًا بايت."
        ),
        injilizi: format!(
            "This patch is for {ism_luba} story mode only. The online mode runs on the same \
             game files this replaces, and its anti-cheat inspects exactly those files: \
             playing online with modified files puts your account at risk of a ban, and the \
             ban lands on the account rather than on the installation. Uninstall before \
             playing online — the uninstall returns the game byte for byte."
        ),
    }
}

/// Why a third-party install was refused. Every variant names its evidence.
#[derive(Debug)]
pub enum RafdKhariji {
    /// The first-run statement has not been acknowledged.
    IqrarNaqis,

    /// The game runs anti-cheat. No override exists.
    Himaya(Box<IjmaaHimaya>),

    /// The anti-cheat check could not run, so its silence proves nothing.
    ///
    /// The same refusal [`crate::fahs::Rafd::FahsMatjarLamYajri`] carries, for
    /// the same reason: VAC is declared in Steam's catalogue and leaves nothing
    /// at all in a game folder, so a scan that never read the catalogue produces
    /// exactly the report a genuinely clean game produces.
    FahsMatjarLamYajri {
        /// The `appinfo.vdf` that was tried, when a Steam root was known at all.
        masar: Option<PathBuf>,
        /// Why, as the same short label the scan's gap list carries.
        sabab: String,
    },

    /// The entry records no written permission from the author.
    ///
    /// The refusal that keeps this feature from being a mirror of the internet.
    /// A patch with no recorded grant is not installed, whatever its licence
    /// file says and whatever the absence of one is taken to mean.
    IdhnMasdarNaqis {
        /// The entry, as it is listed.
        unwan: String,
        /// Who it credits, so the message says whom to ask.
        masdar: String,
    },

    /// The entry asks to be fetched from the registry's mirror and its recorded
    /// permission does not cover that.
    MiraGhayrMasmuha {
        /// The entry, as it is listed.
        unwan: String,
        /// The mirror endpoint that was declared.
        rabt: String,
    },

    /// The entry declares no artifacts for the build that is installed.
    ///
    /// Refused rather than reduced to "install what applies": an entry that
    /// names builds and does not name this one has not been tested against it,
    /// and an install that silently skipped every artifact would report success
    /// over a game nothing was written into.
    BinaGhayrMadumma {
        /// The entry, as it is listed.
        unwan: String,
        /// The build that is installed.
        bina: String,
        /// The builds the entry declares.
        abniya: Vec<String>,
    },

    /// The warnings the gate put in front of the person were not acknowledged.
    ///
    /// Carries the list, so a caller that reached this by showing nothing can
    /// show exactly what it should have.
    TahdheeratGhayrMuqarra {
        /// Every sentence that had to be acknowledged, in display order.
        tahdheerat: Vec<TahdheerKhariji>,
        /// Whether the online sentence is among them.
        shabaki: bool,
    },
}

impl RafdKhariji {
    /// The sentence shown to the user, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        match self {
            Self::IqrarNaqis => "لم يُقرَّ بيان تعريب بعد.".to_owned(),
            Self::Himaya(ijmaa) => {
                let mut nass = "تعمل هذه اللعبة بنظام مكافحة غش، ولا تُثبَّت فيها رقعة؛ قد \
                                يُحظر حسابك حظرًا دائمًا. الدليل:"
                    .to_owned();
                for daleel in &ijmaa.adilla {
                    nass.push_str("\n- ");
                    nass.push_str(&daleel.arabi());
                }
                nass
            },
            Self::FahsMatjarLamYajri { masar, sabab } => {
                let mawdi = masar.as_ref().map_or_else(
                    || "لم يُعرف موضع تثبيت ستيم على هذا الجهاز".to_owned(),
                    |masar| format!("تعذّرت قراءة {} ({sabab})", masar.display()),
                );
                format!(
                    "لم يُستكمل فحص مكافحة الغش: حماية VAC لا تُعلَن إلا في فهرس متجر ستيم، ولا \
                     تترك أثرًا في مجلّد اللعبة، فسكوت الفحص هنا ليس براءة. {mawdi}. لا يُثبَّت \
                     شيء قبل قراءة الفهرس."
                )
            },
            Self::IdhnMasdarNaqis { unwan, masdar } => format!(
                "«{unwan}» عملُ {masdar}، ولا يحمل سجلّ تعريب إذنًا مكتوبًا منه بتوزيعه أو \
                 تثبيته. خلوّ العمل من رخصة ليس رخصةً مفتوحة، بل هو احتفاظٌ بكلّ الحقوق. لا \
                 يُثبَّت شيء حتى يُسجَّل إذن صاحبه."
            ),
            Self::MiraGhayrMasmuha { unwan, rabt } => format!(
                "طُلب جلب «{unwan}» من نسخة السجلّ ({rabt})، والإذن المسجَّل لا يشمل المرآة. \
                 المرآة معطّلة افتراضيًّا ولا تُفتح إلا بإذنٍ يذكرها. يبقى موضع المؤلّف هو \
                 المصدر."
            ),
            // An undetermined build reaches here as an empty string, and
            // "البناء المثبَّت ()" names nothing. The two states get the two
            // sentences they deserve: one says a build was read and is not
            // covered, the other says no build was read at all.
            Self::BinaGhayrMadumma {
                unwan,
                bina,
                abniya,
            } if bina.is_empty() => format!(
                "لم يُحدَّد بناء اللعبة، ولم يُقبَل التثبيت على بناء غير محدَّد. تذكر «{unwan}» \
                 الأبنية: {}. لم يُجلب شيء ولم يُكتب شيء.",
                abniya.join("، ")
            ),
            Self::BinaGhayrMadumma {
                unwan,
                bina,
                abniya,
            } => format!(
                "«{unwan}» لا يعلن دعم بناء اللعبة المثبَّت ({bina}). الأبنية المدعومة: {}. لم \
                 يُجلب شيء ولم يُكتب شيء.",
                abniya.join("، ")
            ),
            Self::TahdheeratGhayrMuqarra { tahdheerat, .. } => {
                let mut nass =
                    "لم تُقرّ التحذيرات التي تسبق هذا التثبيت، ولا يُمنح إذن التثبيت دونها:".to_owned();
                for tahdheer in tahdheerat {
                    nass.push_str("\n- ");
                    nass.push_str(&tahdheer.arabi);
                }
                nass
            },
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        match self {
            Self::IqrarNaqis => "the first-run statement has not been acknowledged".to_owned(),
            Self::Himaya(ijmaa) => {
                let mut nass =
                    "this game runs anti-cheat; installing can permanently ban your account. \
                     Evidence:"
                        .to_owned();
                for daleel in &ijmaa.adilla {
                    nass.push_str("\n- ");
                    nass.push_str(&daleel.injilizi());
                }
                nass
            },
            Self::FahsMatjarLamYajri { masar, sabab } => {
                let mawdi = masar.as_ref().map_or_else(
                    || "no Steam installation could be located on this machine".to_owned(),
                    |masar| format!("{} could not be read ({sabab})", masar.display()),
                );
                format!(
                    "the anti-cheat check did not finish: VAC is declared only in Steam's \
                     catalogue and leaves nothing in the game folder, so silence here is not a \
                     clean result. {mawdi}. Nothing is installed until the catalogue is read."
                )
            },
            Self::IdhnMasdarNaqis { unwan, masdar } => format!(
                "{unwan} is {masdar}'s work and Taarib holds no written permission from them to \
                 distribute or install it. No licence file is not a permissive licence; it is \
                 the absence of one, and the absence of one reserves every right. Nothing is \
                 installed until the permission is on record."
            ),
            Self::MiraGhayrMasmuha { unwan, rabt } => format!(
                "{unwan} was asked to be fetched from the registry mirror ({rabt}) and the \
                 recorded permission does not cover mirroring. Mirroring is off by default and \
                 is only opened by a permission that names it. The author's own endpoint stays \
                 the source."
            ),
            Self::BinaGhayrMadumma {
                unwan,
                bina,
                abniya,
            } if bina.is_empty() => format!(
                "The game's build was never determined, and installing for an undetermined build \
                 was not accepted. {unwan} names builds: {}. Nothing was fetched and nothing was \
                 written.",
                abniya.join(", ")
            ),
            Self::BinaGhayrMadumma {
                unwan,
                bina,
                abniya,
            } => format!(
                "{unwan} does not declare support for the installed build ({bina}). It declares: \
                 {}. Nothing was fetched and nothing was written.",
                abniya.join(", ")
            ),
            Self::TahdheeratGhayrMuqarra { tahdheerat, .. } => {
                let mut nass = "the warnings shown before this install were not acknowledged, \
                                and no install authorisation is minted without them:"
                    .to_owned();
                for tahdheer in tahdheerat {
                    nass.push_str("\n- ");
                    nass.push_str(&tahdheer.injilizi);
                }
                nass
            },
        }
    }
}

/// The gate's verdict.
#[derive(Debug)]
pub enum NatijatFahsKhariji {
    /// Every check passed; the authorisation is minted.
    Masmuh(IdhnTathbeetKhariji),
    /// A check refused, with its reason.
    Marfud(Box<RafdKhariji>),
}

/// Everything the gate needs to decide about one third-party entry.
pub struct TalabFahsKhariji<'a> {
    /// The game, as Taarib identifies it.
    pub luba: LubaId,
    /// The game's display name, which the online warning names.
    pub ism_luba: &'a str,
    /// Its install root.
    pub jidhr_luba: &'a Path,
    /// Its Steam app id, when it has one.
    pub appid: Option<u32>,
    /// The Steam install root, for the `appinfo.vdf` reads.
    pub jidhr_steam: Option<&'a Path>,
    /// The installed build, as the entry's build list spells them.
    ///
    /// Empty when nothing determined it — neither a correspondence the entry
    /// declared nor a probe of the game's own files. That is a different state
    /// from a build that was determined and is not covered, and it is the only
    /// state [`Self::iqrar_bina_majhula`] can lift.
    pub bina: &'a str,
    /// Whether the person accepted installing for a build nobody determined.
    ///
    /// Only consulted when [`Self::bina`] is empty. A build that *was*
    /// determined and is not covered stays refused however this is set: that
    /// refusal rests on something established, and an acknowledgement cannot
    /// make an entry support a build its maker never tested it against.
    pub iqrar_bina_majhula: bool,
    /// The catalogue entry being installed.
    pub ruqaa: &'a RuqaaKharijiya,
    /// The first-run acknowledgement record, when one exists.
    pub iqrar: Option<&'a SijillIqrar>,
    /// Whether the user acknowledged every warning [`tahdheerat`] returns.
    ///
    /// The caller's job is to show that exact list and pass `true` only once the
    /// person has agreed to it. A caller that shows nothing and passes `true` is
    /// lying to its own user; a caller that shows the list and passes `false`
    /// gets a refusal carrying the list, which is the shape a confirmation
    /// dialog needs.
    pub iqrar_tahdheerat: bool,
}

impl std::fmt::Debug for TalabFahsKhariji<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabFahsKhariji")
            .field("luba", &self.luba)
            .field("bina", &self.bina)
            .field("ruqaa", &self.ruqaa.id)
            .field("iqrar_tahdheerat", &self.iqrar_tahdheerat)
            .finish()
    }
}

/// Every sentence that must be in front of the person before this entry is
/// installed, in display order.
///
/// The entry's own warnings first, in the order its author wrote them, then the
/// gate's online sentence when the scan found an online mode. Callers use this
/// to build the confirmation screen; the gate uses the same function to decide
/// what has to be acknowledged, so the list shown and the list required cannot
/// come apart.
#[must_use]
pub fn tahdheerat(
    ruqaa: &RuqaaKharijiya,
    ism_luba: &str,
    shabaka: &IjmaaShabaka,
) -> Vec<TahdheerKhariji> {
    let mut qaima = ruqaa.tahdheerat.clone();
    if shabaka.online() {
        qaima.push(tahdheer_laab_shabaki(ism_luba));
    }
    qaima
}

/// Runs every refusal check and mints [`IdhnTathbeetKhariji`] only if all pass.
///
/// Nothing here reaches the network and nothing here writes: the verdict is a
/// function of the entry, the game folder and the store catalogue. The fetch and
/// the install both take the permit this produces, so an entry that was never
/// put through here has no way to reach either.
#[must_use]
pub fn fahs_khariji(talab: &TalabFahsKhariji<'_>) -> NatijatFahsKhariji {
    if yahtaj_iqrar(talab.iqrar) {
        return NatijatFahsKhariji::Marfud(Box::new(RafdKhariji::IqrarNaqis));
    }

    // One read of `appcache/appinfo.vdf` for both scans below, as the signed
    // path does: it is measured in megabytes on a mature account.
    let matjar = QiraatMatjar::iqra(talab.appid, talab.jidhr_steam);

    let (himaya, halat_matjar) = ifhas_himaya_bi_qiraa(talab.jidhr_luba, &matjar);
    if mahmiya(&himaya) {
        return NatijatFahsKhariji::Marfud(Box::new(RafdKhariji::Himaya(Box::new(himaya))));
    }
    if let Some(rafd) = rafd_matjar(&halat_matjar) {
        return NatijatFahsKhariji::Marfud(Box::new(rafd));
    }

    if !idhn_katabi(&talab.ruqaa.masdar) {
        return NatijatFahsKhariji::Marfud(Box::new(RafdKhariji::IdhnMasdarNaqis {
            unwan: talab.ruqaa.unwan.clone(),
            masdar: talab.ruqaa.nasab(),
        }));
    }

    let mira = talab.ruqaa.mira.clone();
    let masmuha = mira_masmuha(&talab.ruqaa.masdar);
    if let crate::kharijiya::HalatMira::MinAlsijill { rabt } = &mira
        && !masmuha
    {
        return NatijatFahsKhariji::Marfud(Box::new(RafdKhariji::MiraGhayrMasmuha {
            unwan: talab.ruqaa.unwan.clone(),
            rabt: rabt.clone(),
        }));
    }

    let qitaa: Vec<_> = talab
        .ruqaa
        .qitaa_li_bina(talab.bina)
        .into_iter()
        .cloned()
        .collect();
    // Nothing determined the build, and the person said to go ahead anyway. The
    // build list cannot be checked against a value nobody established, so the
    // one check left is the one that always mattered: that there is something to
    // install. A build that *was* determined and is not covered is untouched by
    // this — that refusal rests on a fact, and no acknowledgement makes an entry
    // support a build its maker never tried.
    let majhul = talab.bina.is_empty();
    let marfud_lilbina = if majhul {
        !talab.iqrar_bina_majhula
    } else {
        !talab.ruqaa.yadam_bina(talab.bina)
    };
    if marfud_lilbina || qitaa.is_empty() {
        return NatijatFahsKhariji::Marfud(Box::new(RafdKhariji::BinaGhayrMadumma {
            unwan: talab.ruqaa.unwan.clone(),
            bina: talab.bina.to_owned(),
            abniya: talab.ruqaa.abniya.clone(),
        }));
    }

    let shabaka = ifhas_shabaka_bi_qiraa(talab.jidhr_luba, &matjar);
    let tahdheerat = tahdheerat(talab.ruqaa, talab.ism_luba, &shabaka);
    if !talab.iqrar_tahdheerat {
        return NatijatFahsKhariji::Marfud(Box::new(RafdKhariji::TahdheeratGhayrMuqarra {
            tahdheerat,
            shabaki: shabaka.online(),
        }));
    }

    NatijatFahsKhariji::Masmuh(IdhnTathbeetKhariji::jadeed(
        talab.luba,
        talab.ruqaa.id,
        talab.ruqaa.isdar.clone(),
        qitaa,
        tahdheerat,
        mira.min_sijill() && masmuha,
    ))
}

/// The refusal an unread store catalogue earns, or [`None`] when it was read or
/// never applied to this game at all.
///
/// The same three-way answer [`crate::fahs`] takes, and deliberately its own
/// copy rather than a shared helper reaching across a module boundary: the
/// mapping is four lines and the two gates must be free to refuse differently
/// without one of them silently changing the other.
fn rafd_matjar(hala: &HalatMatjar) -> Option<RafdKhariji> {
    match hala {
        HalatMatjar::GhayrMatlub | HalatMatjar::Maqru => None,
        HalatMatjar::JidhrMajhul => Some(RafdKhariji::FahsMatjarLamYajri {
            masar: None,
            sabab: "no Steam root was given for a Steam game".to_owned(),
        }),
        HalatMatjar::Mutaadhdhir { masar, sabab } => Some(RafdKhariji::FahsMatjarLamYajri {
            masar: Some(masar.clone()),
            sabab: sabab.clone(),
        }),
    }
}
