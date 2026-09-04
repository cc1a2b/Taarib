//! الصوت — Arabic voice packs, as a registry artifact of their own kind.
//!
//! ## The scope, and what is deliberately not in it
//!
//! Taarib **distributes and manages** voice packs. It does not create them.
//! There is no recording tool here, no synthesis, no audio editor, no
//! alignment tool, and none is planned — the product's competence is
//! Arabic text in games, and a half-built audio workstation bolted to the side
//! of it would serve nobody. Download, verify, install, uninstall. That is the
//! whole surface.
//!
//! Stating the boundary in the vocabulary crate rather than in a design
//! document is deliberate: this is the file somebody reaches for when they want
//! to add "just a small trimming feature", and the answer should be in front of
//! them when they arrive.
//!
//! ## Why a voice pack is not a field on a text patch
//!
//! Because the two are independent in every direction that matters:
//!
//! - Either may exist without the other. A game can have a voice pack and no
//!   text patch, and the reverse is the common case.
//! - They are **published by different people**. The contributor who dubbed a
//!   game is frequently not the one who translated its menus, and forcing them
//!   into one artifact would mean one of them owning the other's work.
//! - They are **installed and removed separately**. Removing voice must leave
//!   text untouched and the reverse; the two are never coupled.
//!
//! That independence is the reason the library card has *two* rings rather than
//! one badge with two states. The card's shape is downstream of this file.
//!
//! ## Bound the same way a text patch is
//!
//! Same game identity, same build identity, same content fingerprints, same
//! signature by the same owner, and listed in the same index shards. A voice
//! pack is a different *kind* of artifact, not a different registry. Anything
//! else would mean two publishing pipelines, two verification paths and two
//! places for a revocation to be missed.
//!
//! ## Size is stated before the download, not after
//!
//! Voice packs are large — a fully dubbed role-playing game is gigabytes where
//! its text patch is kilobytes. [`MulakhkhasSawt::hajm`] is therefore not a
//! detail on a properties panel; the interface shows it prominently before the
//! user commits, because a user who starts a four-gigabyte download believing
//! it is a translation patch has been misled by omission.

use serde::{Deserialize, Serialize};

use crate::bina::Basma;
use crate::musahim::MusahimId;
use crate::muharrik::AilatMuharrik;
use crate::ruqaa::{RuqaaId, RuqaaRevision, RukhsaRuqaa};

/// How a voice pack's audio reaches the game.
///
/// The distinction decides whether an install is reversible by putting files
/// back or by putting a container back, and those are different amounts of
/// disk and different failure modes — which is why it is a property of the
/// pack rather than something the installer works out per file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TareeqatTarkeebSawt {
    /// Loose files dropped beside the game's own, which the engine prefers.
    ///
    /// The good case. Uninstalling deletes what was added and touches nothing
    /// the game shipped, so the original audio was never modified at all.
    Tarakub,
    /// Loose files that replace the game's own, one for one.
    ///
    /// Each original is preserved before it is overwritten, exactly as a text
    /// patch's are.
    Istibdal,
    /// The game's audio container is rewritten.
    ///
    /// The expensive case, and the one that needs saying out loud: the original
    /// container is preserved whole and the patched one is written **as a
    /// separate file**, because a container rebuilt in place cannot be undone
    /// if the rebuild is interrupted. Uninstalling puts the original back and
    /// deletes the rebuilt one.
    IadatBina,
}

impl TareeqatTarkeebSawt {
    /// Whether this method modifies files the game shipped.
    ///
    /// False only for [`TareeqatTarkeebSawt::Tarakub`], and the difference is
    /// what the interface tells the user before installing: an additive pack
    /// survives a game update, and the other two do not.
    #[must_use]
    pub const fn yuaddil(self) -> bool {
        !matches!(self, Self::Tarakub)
    }

    /// Roughly how much extra disk an install needs beyond the pack itself.
    ///
    /// A multiplier over the pack's own size, because a container rebuild has
    /// to hold the original, the rebuilt copy, and the pack at once. Stated so
    /// the interface can refuse before starting rather than filling somebody's
    /// disk and failing halfway.
    #[must_use]
    pub const fn muddaaf_masaha(self) -> u32 {
        match self {
            Self::Tarakub | Self::Istibdal => 1,
            Self::IadatBina => 3,
        }
    }

    /// What the user is told before installing, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Tarakub => {
                "تُضاف ملفات الصوت بجوار ملفات اللعبة دون تعديل أيٍّ منها، وإزالتها تحذف ما \
                 أُضيف فقط."
            }
            Self::Istibdal => {
                "تُستبدل بعض ملفات صوت اللعبة، وتُحفظ نسخها الأصلية أولًا؛ الإزالة تُعيدها كما \
                 كانت."
            }
            Self::IadatBina => {
                "تُعاد كتابة حاوية صوت اللعبة. تُحفظ الحاوية الأصلية كاملةً وتُكتب المعدَّلة \
                 بجوارها، ويحتاج ذلك مساحة إضافية تعادل ثلاثة أضعاف حجم الحزمة."
            }
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Tarakub => {
                "Audio files are added beside the game's own and none of them are modified. \
                 Uninstalling deletes only what was added."
            }
            Self::Istibdal => {
                "Some of the game's audio files are replaced, and each original is preserved \
                 first. Uninstalling puts them back."
            }
            Self::IadatBina => {
                "The game's audio container is rebuilt. The original is preserved whole and the \
                 rebuilt one written beside it, which needs about three times the pack's size \
                 in free space."
            }
        }
    }
}

/// What a voice pack covers.
///
/// Coarser than a text patch's coverage, which counts strings, because audio
/// has no equivalent unit — a "line" is a recording, and a pack that dubbed
/// every cutscene and no ambient barks has covered the part that matters
/// without covering most of the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NitaqSawt {
    /// The main story's spoken dialogue.
    Qissa,
    /// Story plus side content.
    Kamil,
    /// Cutscenes only.
    Mashahid,
    /// A named subset the pack describes for itself.
    Juzii,
}

impl NitaqSawt {
    /// The label the card and the detail screen show, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Qissa => "حوار القصة الرئيسية",
            Self::Kamil => "كامل الحوار",
            Self::Mashahid => "المشاهد السينمائية فقط",
            Self::Juzii => "تغطية جزئية",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Qissa => "main story dialogue",
            Self::Kamil => "full dialogue",
            Self::Mashahid => "cutscenes only",
            Self::Juzii => "partial coverage",
        }
    }
}

/// A relationship between a voice pack and a text patch.
///
/// Recorded because it is real — a dub whose lines were timed against a
/// specific translation's subtitles genuinely does expect that translation —
/// and modelled as advice rather than as a constraint, because the alternative
/// is a product that refuses to install a voice pack somebody wants.
///
/// [`SilatRuqaa::yulzim`] is `false` for every variant. That is not an
/// oversight and the method exists to make it checkable: **the interface states
/// the relationship and never forces the pairing.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SilatRuqaa {
    /// This pack's subtitle timing was matched to a specific text patch.
    Tawqit {
        /// Which patch.
        ruqaa: RuqaaId,
        /// Its title, denormalized so a listing needs no second fetch.
        unwan: String,
    },
    /// The same contributor published both and recommends them together.
    Tawsiya {
        /// Which patch.
        ruqaa: RuqaaId,
        /// Its title.
        unwan: String,
    },
    /// This pack expects the game's own subtitles and none of Taarib's.
    Mustaqilla,
}

impl SilatRuqaa {
    /// Whether this relationship prevents installing without the other artifact.
    ///
    /// Always false. The product states relationships and never enforces them:
    /// a user who wants Arabic voice over English subtitles is making a choice
    /// somebody might not have anticipated, and it is theirs to make.
    #[expect(
        clippy::unused_self,
        reason = "the answer is a property of the relationship, not of the type; dropping \
                  the receiver would turn a per-variant predicate a caller can ask into a \
                  constant, which is the check this method exists to keep askable"
    )]
    #[must_use]
    pub const fn yulzim(&self) -> bool {
        false
    }

    /// The text patch this relates to, when there is one.
    #[must_use]
    pub const fn ruqaa(&self) -> Option<RuqaaId> {
        match self {
            Self::Tawqit { ruqaa, .. } | Self::Tawsiya { ruqaa, .. } => Some(*ruqaa),
            Self::Mustaqilla => None,
        }
    }

    /// What the interface says about it, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Tawqit { unwan, .. } => format!(
                "وُقِّتت هذه الحزمة على ترجمة «{unwan}». يمكن تثبيتها وحدها، وقد لا تتطابق \
                 الترجمة الظاهرة مع المنطوق."
            ),
            Self::Tawsiya { unwan, .. } => {
                format!("ينصح ناشرها بتثبيتها مع ترجمة «{unwan}».")
            }
            Self::Mustaqilla => {
                "لا ترتبط هذه الحزمة بترجمة نصية بعينها.".to_owned()
            }
        }
    }
}

/// A published voice pack, as the registry index lists it.
///
/// Field for field the shape of [`crate::ruqaa::MulakhkhasRuqaa`] wherever the
/// two mean the same thing — the same identity, revision, contributor,
/// licence, build bindings, fingerprints, package hash and mirror. Divergence
/// here would be divergence in the index, and the index is shared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MulakhkhasSawt {
    /// The lineage.
    pub id: RuqaaId,
    /// The published revision.
    pub murajaa: RuqaaRevision,
    /// The title the contributor gave it.
    pub unwan: String,
    /// Who made it.
    pub musahim: MusahimId,
    /// Their display name, denormalized so a listing needs no second fetch.
    pub ism_musahim: String,
    /// How much of the game's speech it covers.
    pub nitaq: NitaqSawt,
    /// A sentence the contributor wrote about the coverage, shown for
    /// [`NitaqSawt::Juzii`] where the enum alone says too little.
    pub wasf_nitaq: Option<String>,
    /// How many audio files it carries.
    pub adad_malaffat: u32,
    /// Total spoken duration in seconds, when the contributor measured it.
    pub muddat_thawani: Option<u64>,
    /// **The package size in bytes.** Shown prominently before download; see
    /// this module's header for why it is not a detail.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub hajm: u64,
    /// How it installs.
    pub tareeqa: TareeqatTarkeebSawt,
    /// Launcher build identifiers this pack was produced against.
    pub bina_manassa: Vec<String>,
    /// Content fingerprints it matches exactly.
    pub basmat: Vec<Basma>,
    /// The engine it targets.
    pub aila: AilatMuharrik,
    /// Its licence.
    pub rukhsa: RukhsaRuqaa,
    /// Its relationship to a text patch, if any.
    pub sila: SilatRuqaa,
    /// Average rating, when it has any.
    pub taqyeem: Option<f32>,
    /// How many ratings.
    pub adad_taqyeemat: u32,
    /// When this revision was published, RFC 3339.
    pub waqt_nashr: String,
    /// The hash of the package itself, verified during download.
    pub basmat_muhtawa: Basma,
    /// Where the package is, as a release asset.
    pub rabt: String,
    /// A mirror, tried when the primary is unreachable.
    pub rabt_mira: Option<String>,
}

impl MulakhkhasSawt {
    /// The size as the interface writes it, in the largest unit that keeps it
    /// readable.
    ///
    /// Formatted here rather than in the interface because the same number
    /// appears on the card, in the download list, in the detail screen and in
    /// the confirmation, and four call sites rounding independently is four
    /// chances to show a user three different sizes for one file.
    #[must_use]
    pub fn hajm_maqru(&self) -> String {
        const WAHDAT: [&str; 4] = ["B", "KB", "MB", "GB"];
        let mut qeema = self.hajm;
        let mut khana = 0_usize;
        // A shift rather than a division, and not to be clever: dividing by
        // 1024 is exactly a ten-bit shift, `integer_division` is denied
        // workspace-wide, and writing the operation as what it is avoids an
        // exception for a lint that is right about every other division in this
        // file. A float here would buy nothing but a rounding disagreement with
        // the download progress bar, which counts the same bytes.
        while qeema >= 1024 && khana.saturating_add(1) < WAHDAT.len() {
            qeema >>= 10;
            khana = khana.saturating_add(1);
        }
        let wahda = WAHDAT.get(khana).copied().unwrap_or("B");
        format!("{qeema} {wahda}")
    }

    /// How much free disk an install of this pack needs.
    ///
    /// The pack plus whatever its method needs on top. Checked before the
    /// download starts, not before the install — a user who downloads four
    /// gigabytes and is then told there is no room to install it has been made
    /// to wait for a refusal that was knowable at the start.
    #[must_use]
    pub fn masaha_matluba(&self) -> u64 {
        self.hajm.saturating_mul(u64::from(self.tareeqa.muddaaf_masaha()))
    }

    /// Whether this pack matches a game's content fingerprint exactly.
    #[must_use]
    pub fn tutabiq(&self, basma: &Basma) -> bool {
        self.basmat.contains(basma)
    }
}

/// A voice pack's state for one game, as the card's outer ring shows it.
///
/// The same four states a text patch has, deliberately: the card draws both
/// rings from one rule, and two state machines that were nearly the same would
/// be two places for "available" and "installed" to drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HalatSawt {
    /// Nothing in the registry for this game.
    LaShay,
    /// A pack exists for this build and can be installed now.
    Mutaha,
    /// A pack is installed.
    Mutabbaqa,
    /// A pack is installed and a newer revision exists.
    Tahdith,
}

impl HalatSawt {
    /// Whether the outer ring is painted at all.
    #[must_use]
    pub const fn tarsim(self) -> bool {
        !matches!(self, Self::LaShay)
    }

    /// Whether the ring is painted at full weight with its glow.
    #[must_use]
    pub const fn murakkaba(self) -> bool {
        matches!(self, Self::Mutabbaqa | Self::Tahdith)
    }

    /// The value the card component receives.
    ///
    /// Matches the `HalatRuqaa` union in `bitaqa.tsx` exactly. Kept as a method
    /// rather than relying on serde's rename, because the interface's union is
    /// hand-written and a silent rename on this side would break it at runtime
    /// rather than at compile time.
    #[must_use]
    pub const fn ramz_wajiha(self) -> &'static str {
        match self {
            Self::LaShay => "la-shay",
            Self::Mutaha => "mutaha",
            Self::Mutabbaqa => "mutabbaqa",
            Self::Tahdith => "tahdith",
        }
    }
}
