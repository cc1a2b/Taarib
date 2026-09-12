//! الإعدادات — configuration.
//!
//! One typed tree with defaults, layered file → environment → command line,
//! saved atomically, and hot-swappable at runtime so a setting change reaches
//! every reader without a restart.
//!
//! No secret is ever stored here. Machine-translation credentials live in the
//! operating system's keychain and are referenced by account name only, so a
//! settings file can be copied between machines, attached to a bug report, or
//! committed by mistake without leaking anything.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use arc_swap::ArcSwap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::khata::{Khata, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam};
use crate::khata_min;
use crate::masarat::{ASMAA_GHAYR_IDADAT, Masarat};
use crate::mukhattat::{self, DhuMukhattat};
use crate::{Lugha, NizamArqam};

/// The prefix an environment override carries; `__` separates nesting levels,
/// so `TAARIB_TASHKHIS__MUSTAWA=tafsil` sets `tashkhis.mustawa`.
const BADIYAT_BEEA: &str = "TAARIB_";

/// The whole settings tree.
///
/// Every field in this tree and its containers is one of exactly two kinds.
///
/// **Required** — no serde attribute. A settings file that is missing the field
/// is a damaged one, and quietly substituting the default is the failure this
/// module exists to refuse: it is how somebody loses their provider setup
/// without being told. A file written by an older build reaches this type only
/// after `mukhattat` has migrated it forward, and a migration that introduces a
/// field adds it explicitly through `mukhattat::adif_iftiradi`, so every field
/// is present by the time serde sees it. That migration, not a serde default,
/// is where forward compatibility comes from here.
///
/// **Genuinely optional** — `Option<T>`, where having no value *is* the
/// setting: no override was chosen, no acknowledgement was given, no credential
/// was linked. Optional in the domain, still required on the wire — serde
/// writes `null` and reading demands the key — so `T | null` stays exact.
///
/// There is deliberately no third kind. No field carries `#[serde(default)]`,
/// and the reason is worth keeping written down, because the attribute looks
/// free and is not.
///
/// It would buy tolerance for a field added without a schema bump, at two
/// prices this tree cannot pay:
///
/// * `specta` renders any defaulted field as `field?: T`, including an
///   `Option<T>` one, which becomes `field?: T | null`. Nothing here ever omits
///   a field on the way out — there is no `skip_serializing_if` in this module
///   — so the `?` would be false in the direction Studio reads, and every read
///   of that field would widen to `| undefined` against a value that is always
///   there.
/// * `haddith_idadat` takes the whole tree back and
///   [`MakhzanIdadat::ghayyir`] writes it. An optional field is therefore a
///   standing licence for the interface to leave a key out, and the omission
///   would be saved to disk as a silent reset of whatever the user had set —
///   the same loss, arriving from the other side.
///
/// A field that genuinely may be absent from the wire would need
/// `skip_serializing_if` to match, and that is a wire-format change, not an
/// attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Idadat {
    /// Interface language.
    pub lugha: Lugha,
    /// Which digits the interface writes numbers with.
    pub arqam: NizamArqam,
    /// Light, dark, or follow the system.
    pub sima: Sima,
    /// Interface density.
    pub kathafa: Kathafa,
    /// High-contrast token set.
    pub tabayun_aali: bool,
    /// Honour the system's reduced-motion preference. On by default; turning it
    /// off does not add motion, it only stops Taarib from removing it.
    pub ihtiram_taqleel_haraka: bool,
    /// Launcher locations and scanning behaviour.
    pub manassat: IdadatManassat,
    /// Where patches and caches live.
    pub takhzin: IdadatTakhzin,
    /// Default fonts.
    pub khutut: IdadatKhutut,
    /// Machine-translation providers.
    pub muzawwidun: IdadatMuzawwidin,
    /// Registry sources, mirrors, and offline shares.
    pub masadir: IdadatMasadir,
    /// Self-update behaviour.
    pub tahdith: IdadatTahdith,
    /// Logging and diagnostics.
    pub tashkhis: IdadatTashkhis,
    /// The universal overlay.
    pub tabaqa: IdadatTabaqa,
    /// The studio's remappable global shortcuts.
    pub ikhtisarat: IdadatIkhtisarat,
    /// Offer arabization for games whose publisher already ships Arabic.
    ///
    /// Off, and the default has to be off. A game with official Arabic was
    /// translated by paid humans, reviewed by native speakers and tested in
    /// context; a patch over it replaces all of that with machine output, and
    /// the player does not find out until they are inside the game reading it.
    /// Taarib therefore takes those games out of its own arabization surface
    /// rather than putting a warning next to a button it still offers.
    ///
    /// Turning it on is for the case the exclusion gets wrong: a publisher who
    /// shipped a machine translation of their own, or one whose Arabic is
    /// unreadable in practice. That case is real and rare, which is exactly
    /// the shape of a setting rather than a default.
    ///
    /// It reveals the surface. It does not change the verdict, does not hide
    /// the badge, and does not touch the registry — a submission for such a
    /// game is refused by the pre-flight gate whatever this is set to, because
    /// this setting is one person's judgement about their own installation and
    /// publishing is a judgement about everybody's.
    pub istibdal_lugha_rasmiya: bool,
    /// The recorded first-run acknowledgement, absent until it is given.
    pub iqrar: Option<Iqrar>,
}

impl Default for Idadat {
    fn default() -> Self {
        Self {
            lugha: Lugha::Arabi,
            arqam: NizamArqam::Latini,
            sima: Sima::Daken,
            kathafa: Kathafa::Murih,
            tabayun_aali: false,
            ihtiram_taqleel_haraka: true,
            manassat: IdadatManassat::default(),
            takhzin: IdadatTakhzin::default(),
            khutut: IdadatKhutut::default(),
            muzawwidun: IdadatMuzawwidin::default(),
            masadir: IdadatMasadir::default(),
            tahdith: IdadatTahdith::default(),
            tashkhis: IdadatTashkhis::default(),
            tabaqa: IdadatTabaqa::default(),
            ikhtisarat: IdadatIkhtisarat::default(),
            istibdal_lugha_rasmiya: false,
            iqrar: None,
        }
    }
}

/// The studio's global shortcuts, stored as `ctrl+k`-style chords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatIkhtisarat {
    /// Opens the command palette.
    pub lawha: String,
    /// Undoes the last recorded step.
    pub taraju: String,
}

impl Default for IdadatIkhtisarat {
    fn default() -> Self {
        Self {
            lawha: "ctrl+k".to_owned(),
            taraju: "ctrl+z".to_owned(),
        }
    }
}

impl DhuMukhattat for Idadat {
    const ISM: &'static str = "idadat";

    /// Moves whenever a field in this tree is added, removed or retyped, and
    /// `hijra` gains the arm that reaches the new shape — for a new field, one
    /// [`mukhattat::adif_iftiradi`] carrying the same value `Default` gives it.
    ///
    /// This is the only thing that keeps an older settings file readable. The
    /// tree carries no serde defaults, for the reasons on [`Idadat`], so a field
    /// added without moving this number turns every existing file into a
    /// refusal.
    const ISDAR: u32 = 2;

    fn hijra(min: u32, qeema: Value) -> Natija<Value> {
        match min {
            // Phase 27 added the official-Arabic override. Absent means off,
            // which is the same answer `Default` gives, so an existing file
            // keeps the exclusion rather than silently opting into it.
            1 => Ok(mukhattat::adif_iftiradi(
                qeema,
                "istibdal_lugha_rasmiya",
                Value::Bool(false),
            )),
            _ => Err(Khata::min_tafsir(&mukhattat::KhataMukhattat::HijraNaqisa {
                ism: Self::ISM,
                min,
            })),
        }
    }
}

/// Light, dark, or whatever the desktop says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Sima {
    /// Near-black foundation. The default.
    #[default]
    Daken,
    /// The light token set.
    Fatih,
    /// Follow the operating system.
    Nizam,
}

/// How much the interface fits on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Kathafa {
    /// Tight rows — the Steam Deck default at 1280×800.
    Mudmaj,
    /// The default.
    #[default]
    Murih,
    /// Everything one step larger, for high-resolution displays and for anyone
    /// who needs it.
    Kabir,
}

/// Launcher locations and scanning behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatManassat {
    /// Steam's root, when it is somewhere Taarib did not find on its own.
    pub steam: Option<PathBuf>,
    /// The Epic Games launcher's root.
    pub epic: Option<PathBuf>,
    /// GOG Galaxy's root.
    pub gog: Option<PathBuf>,
    /// The EA app's root.
    pub ea: Option<PathBuf>,
    /// Ubisoft Connect's root.
    pub ubisoft: Option<PathBuf>,
    /// The Battle.net client's root.
    pub battlenet: Option<PathBuf>,
    /// The Xbox app's package root.
    pub xbox: Option<PathBuf>,
    /// The itch.io app's root.
    pub itch: Option<PathBuf>,
    /// Heroic's configuration root.
    pub heroic: Option<PathBuf>,
    /// The Amazon Games client's data root.
    pub amazon: Option<PathBuf>,
    /// The Rockstar Games Launcher's root.
    ///
    /// Rockstar keeps its catalogue in the registry rather than on disk, so this
    /// override does not point at a catalogue the way the others do — it names
    /// the launcher's own install folder, which is what the "open the launcher
    /// at this game" button needs when the registry does not record it.
    pub rockstar: Option<PathBuf>,
    /// The Riot client's metadata root.
    pub riot: Option<PathBuf>,
    /// Lutris's data root, holding `pga.db`.
    pub lutris: Option<PathBuf>,
    /// Bottles' data root, holding one directory per bottle.
    pub bottles: Option<PathBuf>,
    /// Legendary's configuration root, holding `installed.json`.
    pub legendary: Option<PathBuf>,
    /// Playnite's library root, in either installed or portable mode.
    pub playnite: Option<PathBuf>,
    /// Extra folders to scan for games, for people who keep libraries outside
    /// any launcher.
    pub mujalladat_idafiya: Vec<PathBuf>,
    /// Watch library folders and refresh as they change.
    pub fahs_tilqai: bool,
}

/// Where patches and caches live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatTakhzin {
    /// Patch storage, when the user moved it off the system drive.
    // Read by `masarat::Masarat::maa_jidhr_ruqaa`, and applied *there* because
    // the ordering allows nowhere else: the settings file is located through a
    // layout, so the layout is resolved first and re-rooted with this once the
    // settings are open — before `takid` creates the directories, so the chosen
    // root is the one that gets made. Deliberately not a doc comment: specta
    // copies those into the committed `awamir.ts`, and this note is for the
    // Rust side only.
    pub jidhr_ruqaa: Option<PathBuf>,
    /// How large the registry and artwork cache may grow, in megabytes.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub hadd_makhbaa_mb: u64,
    /// Keep original-file backups after a patch is uninstalled. On by default:
    /// the backup is what makes an uninstall exact, and it is small.
    pub ibqa_nusakh: bool,
}

impl Default for IdadatTakhzin {
    fn default() -> Self {
        Self {
            jidhr_ruqaa: None,
            hadd_makhbaa_mb: 2048,
            ibqa_nusakh: true,
        }
    }
}

/// Default fonts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatKhutut {
    /// The interface font family, by the name it carries in `khutut.json`.
    pub khatt_wajiha: String,
    /// The font a new patch starts with.
    pub khatt_luba_iftiradi: String,
    /// A folder of the user's own fonts, validated on import.
    pub masar_khutut_mustakhdim: Option<PathBuf>,
}

impl Default for IdadatKhutut {
    fn default() -> Self {
        Self {
            khatt_wajiha: "IBMPlexSansArabic".to_owned(),
            khatt_luba_iftiradi: "NotoNaskhArabic".to_owned(),
            masar_khutut_mustakhdim: None,
        }
    }
}

/// A machine-translation provider.
///
/// `deny_unknown_fields` matters more here than anywhere else in the tree: a
/// misspelt key inside a provider entry would otherwise be dropped in silence,
/// and the provider would come back missing its endpoint or its keychain
/// account with nothing said.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatMuzawwid {
    /// A stable identifier the user chose, unique within the list.
    pub muarrif: String,
    /// Which API shape this provider speaks.
    pub naw: NawMuzawwid,
    /// The model name sent with every request.
    pub namudhaj: String,
    /// A custom endpoint, for self-hosted and compatible services.
    pub asas: Option<String>,
    /// The keychain account holding the credential. The credential itself is
    /// never in this file.
    pub hisab_miftah: Option<String>,
    /// Whether the provider is available for use.
    pub mufaal: bool,
    /// Client-side request ceiling per minute.
    ///
    /// Zero is not "no limit": it means "no client-side limit was chosen", and
    /// the provider's own built-in default applies instead. That substitution
    /// happens where the provider is built, and it was undocumented — a field
    /// showing `0` beside the word "limit" reads as unlimited and is the
    /// opposite of what it does.
    pub hadd_talabat: u32,
    /// A spend ceiling in US dollars for one translation run, or [`None`] for
    /// no ceiling at all.
    ///
    /// **Per run, not per month.** Nothing in this product tracks a calendar
    /// period: the value is converted once when a run starts and bounds that
    /// run's own spending, cumulatively across resumes of the *same* run
    /// because a resumed run seeds its ledger from the journal it is resuming.
    /// A second run starts the count again. This said "monthly" and enforced
    /// nothing of the kind, which is the worst direction for a promise about
    /// money to be wrong in — somebody sets twenty dollars, reads the word
    /// month, and believes the number is a limit on something it does not
    /// limit.
    ///
    /// **[`None`] means unlimited, and it is the value a newly added provider
    /// starts on.** No ceiling is enforced anywhere for a provider left this
    /// way: the run is bounded only by the string table running out. A
    /// ceiling is what makes the "stops at the ceiling and never crosses it"
    /// promise true, and there is no ceiling to stop at until this is set.
    pub mizaniya: Option<f64>,
}

/// The API shape a provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NawMuzawwid {
    /// Anthropic's messages API.
    Anthropic,
    /// Any endpoint speaking the `OpenAI` chat-completions shape.
    OpenAiMutawafiq,
    /// Google Gemini.
    Gemini,
    /// `DeepL`.
    Deepl,
    /// Google's translation API.
    GoogleTarjama,
    /// Microsoft's translation API.
    MicrosoftTarjama,
    /// A local model behind an Ollama-compatible endpoint.
    Mahalli,
    /// Google Translate's free web endpoint — the one the browser widget and
    /// every Unity translation mod use. No credential, no cost, and unofficial:
    /// sentence-level quality and undocumented rate limits. Built into every
    /// installation as the provider of last resort, see
    /// [`IdadatMuzawwid::google_majjani`]; it can also be added to the list by
    /// hand, which is how somebody makes it win over a keyed provider they
    /// would rather keep switched on.
    GoogleMajjani,
}

/// The identifier the built-in free provider reports itself under.
///
/// Reserved: no entry in the user's list may carry it. The election reports the
/// identifier of whatever it elected, and two providers under one name would
/// make that report ambiguous on the one screen where it matters — the one
/// that says which provider is about to be used.
pub const MUARRIF_GOOGLE_MAJJANI: &str = "google-majjani";

/// The model name the built-in free provider records as provenance: the
/// endpoint's own client identifier, which is the only "model" it has.
pub const NAMUDHAJ_GOOGLE_MAJJANI: &str = "gtx";

impl IdadatMuzawwid {
    /// The built-in free provider, as the entry the election falls back to.
    ///
    /// Not stored in the list and never written to a settings file: it exists
    /// in every installation by construction, so nothing can delete it, and a
    /// settings file written by a build from before it existed elects it exactly
    /// as a fresh installation does. `hadd_talabat` is zero — the provider's own
    /// request spacing applies, and it is not a number the user tunes — and
    /// `mizaniya` is [`None`] because there is nothing to cap.
    #[must_use]
    pub fn google_majjani() -> Self {
        Self {
            muarrif: MUARRIF_GOOGLE_MAJJANI.to_owned(),
            naw: NawMuzawwid::GoogleMajjani,
            namudhaj: NAMUDHAJ_GOOGLE_MAJJANI.to_owned(),
            asas: None,
            hisab_miftah: None,
            mufaal: true,
            hadd_talabat: 0,
            mizaniya: None,
        }
    }
}

/// The configured providers.
///
/// The default is an empty list with no elected provider, and that is the state
/// a fresh installation is in. It is a real state rather than a missing one:
/// installing a published patch never reaches a provider, and new machine
/// translation goes through the built-in free provider until the user adds one
/// of their own — see [`Self::muntakhab_aw_majjani`]. [`Self::hala`] is where
/// the list's state is stated, once, so that no surface has to decide it again.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatMuzawwidin {
    /// Every provider the user has set up.
    pub qaima: Vec<IdadatMuzawwid>,
    /// Which one a new batch uses, by identifier.
    pub iftiradi: Option<String>,
}

impl IdadatMuzawwidin {
    /// The provider a new translation would actually use, or [`None`].
    ///
    /// The election rule, written here rather than at each caller. `iftiradi` is
    /// honoured when it names an entry that is both present and switched on; a
    /// default naming a disabled or deleted provider falls through to the first
    /// enabled one, because refusing to translate over a stale preference helps
    /// nobody. Which of the two happened is exactly what [`Self::hala`] reports,
    /// so the fallthrough is never the silent part.
    #[must_use]
    pub fn muntakhab(&self) -> Option<&IdadatMuzawwid> {
        self.iftiradi
            .as_ref()
            .and_then(|ism| {
                self.qaima
                    .iter()
                    .find(|tarif| &tarif.muarrif == ism && tarif.mufaal)
            })
            .or_else(|| self.qaima.iter().find(|tarif| tarif.mufaal))
    }

    /// The provider a new translation actually uses: the elected one, or the
    /// built-in free one when nothing is elected.
    ///
    /// The second half of the election rule, written beside the first so that
    /// no caller composes its own. A keyed or local provider the user set up
    /// and switched on always wins; the built-in one stands in only when the
    /// list elects nothing — empty, or every entry off — which is why a fresh
    /// installation translates and why adding a provider is a strict upgrade
    /// rather than a switch. Owned rather than borrowed because the fallback
    /// is not in the list to borrow from.
    #[must_use]
    pub fn muntakhab_aw_majjani(&self) -> IdadatMuzawwid {
        self.muntakhab()
            .cloned()
            .unwrap_or_else(IdadatMuzawwid::google_majjani)
    }

    /// What this list amounts to on this machine.
    #[must_use]
    pub fn hala(&self) -> HalatMuzawwidin {
        let Some(muntakhab) = self.muntakhab() else {
            return if self.qaima.is_empty() {
                HalatMuzawwidin::Faragh
            } else {
                HalatMuzawwidin::Muattala
            };
        };
        match &self.iftiradi {
            Some(ism) if ism != &muntakhab.muarrif => HalatMuzawwidin::Badeel,
            _ => HalatMuzawwidin::Mukhtar,
        }
    }
}

/// What the configured provider list amounts to, and therefore which provider
/// a new translation goes through.
///
/// Four states rather than a boolean, because "nothing of your own is elected"
/// has two causes with two different remedies and the product used to give
/// both of them the same sentence — telling somebody who has a provider
/// switched off to go and add one. The fourth is not a refusal at all: it is a
/// preference that is quietly not being honoured, which matters because the
/// provider being used instead of the chosen one is the provider being charged
/// for.
///
/// None of the four withholds translation. The first two used to, and the
/// sentence a user read then was a dead end: a fresh installation could not
/// translate one string until a paid account existed somewhere. The built-in
/// free provider — [`IdadatMuzawwid::google_majjani`] — now stands in for
/// exactly those two states, so their sentences say which service is doing the
/// work, what it is worth, and where a better one is chosen. Every sentence
/// still states the other half of the truth as well: installing a published
/// patch reaches no provider, and a message that reads as "no provider" reads
/// as "nothing works" to somebody who came here to install one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HalatMuzawwidin {
    /// Not one provider has been added. The state a fresh installation is in,
    /// and the built-in free provider does the translating.
    Faragh,
    /// Providers are configured and every one of them is switched off, so the
    /// built-in free provider does the translating.
    Muattala,
    /// A provider is elected, and it is the one the default names — or the only
    /// enabled one, with no default set.
    Mukhtar,
    /// A provider is elected, but it is not the one the default names: that one
    /// is switched off, or is no longer in the list.
    Badeel,
}

impl HalatMuzawwidin {
    /// The sentence a user reads, in Arabic.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::Faragh => {
                "لا مزوّد ترجمة من إعدادك على هذا الجهاز، فتمرّ ترجمة النصّ الجديد عبر خدمة \
                 ترجمة Google المجانية المضمّنة: غير رسمية، بجودة الجملة الواحدة، بحدود طلبات \
                 غير معلنة، ولا تكلّف شيئًا. مزوّد بمفتاح — أو نموذج محلي — تضيفه في الإعدادات \
                 ← المزوّدون يتقدّم عليها متى فُعّل. تثبيت الرقع المنشورة يعمل كما هو ولا يمرّ \
                 بمزوّد أصلًا."
            },
            Self::Muattala => {
                "كلّ المزوّدين المضبوطين على هذا الجهاز معطَّلون، فتمرّ ترجمة النصّ الجديد عبر \
                 خدمة ترجمة Google المجانية المضمّنة — غير رسمية، بجودة الجملة الواحدة، بحدود \
                 غير معلنة — حتى تفعّل أحدهم في الإعدادات ← المزوّدون. تثبيت الرقع المنشورة \
                 يعمل كما هو."
            },
            Self::Mukhtar => "المزوّد الافتراضي مفعّل، وهو الذي تستخدمه أيّ ترجمة جديدة.",
            Self::Badeel => {
                "المزوّد المحدَّد افتراضيًّا غير مفعّل أو لم يعد في القائمة، فستستخدم الترجمة \
                 الجديدة أوّل مزوّد مفعّل بدلًا منه — وهو المزوّد الذي ستُحتسب تكلفته. راجع \
                 «افتراضي» في الإعدادات ← المزوّدون إن لم يكن هذا ما تقصده."
            },
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::Faragh => {
                "No provider of your own is configured on this machine, so new text is \
                 translated through the built-in free Google Translate web service: \
                 unofficial, sentence-level quality, undocumented rate limits, and no cost. A \
                 keyed provider — or a local model — added in Settings, under Providers, takes \
                 precedence once enabled. Installing published patches still works and never \
                 reaches a provider at all."
            },
            Self::Muattala => {
                "Every provider configured on this machine is switched off, so new text is \
                 translated through the built-in free Google Translate web service — \
                 unofficial, sentence-level quality, undocumented rate limits — until you \
                 enable one in Settings, under Providers. Installing published patches still \
                 works."
            },
            Self::Mukhtar => {
                "The default provider is enabled, and it is the one any new translation uses."
            },
            Self::Badeel => {
                "The provider marked as the default is switched off or is no longer in the \
                 list, so a new translation uses the first enabled provider instead — and \
                 that is the one being charged for. Check which provider is marked default \
                 in Settings, Providers, if that is not what you meant."
            },
        }
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Faragh => "faragh",
            Self::Muattala => "muattala",
            Self::Mukhtar => "mukhtar",
            Self::Badeel => "badeel",
        }
    }
}

/// Registry sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatMasadir {
    /// The canonical registry repository.
    pub rasmi: String,
    /// Mirrors, tried in order when the canonical source is unreachable.
    pub maraya: Vec<String>,
    /// Local directories and network shares that carry a registry copy, which
    /// make the whole product work with no internet at all.
    pub mahalliya: Vec<PathBuf>,
    /// The forge OAuth client identifier for device-flow submission; absent
    /// until the registry operator provisions one, and submission stays a
    /// local handoff that says so.
    pub muarrif_amil: Option<String>,
    /// The staging upload endpoint template, `{ism}` for the file name; absent
    /// until the registry operator provisions the staging release.
    pub rabt_tajheez: Option<String>,
    /// How often the background refresh checks the global manifest, in minutes.
    pub fatra_tahdith: u32,
    /// Never touch the network, for people who want that guarantee.
    pub wadaa_ghayr_muttasil: bool,
}

impl Default for IdadatMasadir {
    fn default() -> Self {
        Self {
            rasmi: "https://github.com/cc1a2b/taarib-registry".to_owned(),
            maraya: vec!["https://cdn.jsdelivr.net/gh/cc1a2b/taarib-registry@main".to_owned()],
            mahalliya: Vec::new(),
            muarrif_amil: None,
            rabt_tajheez: None,
            fatra_tahdith: 180,
            wadaa_ghayr_muttasil: false,
        }
    }
}

/// Self-update behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatTahdith {
    /// Download and stage updates without being asked. The update still applies
    /// on restart, never mid-session.
    pub tilqai: bool,
    /// Which release channel to follow.
    pub qanat: QanatTahdith,
    /// Check when the application starts.
    pub fahs_ind_bad: bool,
}

impl Default for IdadatTahdith {
    fn default() -> Self {
        Self {
            tilqai: false,
            qanat: QanatTahdith::Mustaqirr,
            fahs_ind_bad: true,
        }
    }
}

/// A release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum QanatTahdith {
    /// Released builds.
    #[default]
    Mustaqirr,
    /// Pre-release builds, for contributors who want adapter fixes early.
    Tajribi,
}

/// Logging and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatTashkhis {
    /// How much detail is recorded.
    pub mustawa: MustawaSijill,
    /// How many days of logs to keep.
    pub ayyam_hifz: u32,
    /// The ceiling on total log size, in megabytes.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub hadd_hajm_mb: u64,
}

impl Default for IdadatTashkhis {
    fn default() -> Self {
        Self {
            mustawa: MustawaSijill::Maluma,
            ayyam_hifz: 14,
            hadd_hajm_mb: 256,
        }
    }
}

/// How much detail the log records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MustawaSijill {
    /// Failures only.
    Khata,
    /// Failures and warnings.
    Tanbeeh,
    /// The default: what happened, without the internals.
    #[default]
    Maluma,
    /// Enough detail to explain an adapter's decisions.
    Tafsil,
    /// Everything, including per-string layout decisions. Large and slow.
    Tatabbu,
}

impl MustawaSijill {
    /// The `tracing` filter directive for this level.
    #[must_use]
    pub const fn tawjeeh(self) -> &'static str {
        match self {
            Self::Khata => "error",
            Self::Tanbeeh => "warn",
            Self::Maluma => "info",
            Self::Tafsil => "debug",
            Self::Tatabbu => "trace",
        }
    }
}

/// Where a recognised line of text is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MawqiTabaqa {
    /// Over the original text, on a backing plate sized to the Arabic.
    #[default]
    Fawq,
    /// Beside the original text.
    Bijanib,
    /// In a fixed panel at the bottom of the screen.
    Lawha,
}

/// Which engine reads the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MuharrikQira {
    /// The operating system's own recognizer, which is fast and already
    /// installed. The default where one exists.
    #[default]
    NizamAsli,
    /// The bundled portable recognizer, used everywhere else.
    Mahmul,
}

/// The universal overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IdadatTabaqa {
    /// Whether the overlay may be used at all.
    pub mufaal: bool,
    /// Which recognizer to use.
    pub muharrik_qira: MuharrikQira,
    /// Backing-plate opacity, 0.0 to 1.0.
    pub shaffafiya: f32,
    /// Overlay text size in pixels.
    pub hajm_khatt: f32,
    /// Where the Arabic is placed relative to the original.
    pub mawqi: MawqiTabaqa,
    /// How many recognised lines the reading history keeps.
    pub tul_sijill_qira: u32,
    /// Keyboard shortcuts, action name to accelerator.
    pub ikhtisarat: BTreeMap<String, String>,
}

impl Default for IdadatTabaqa {
    fn default() -> Self {
        let mut ikhtisarat = BTreeMap::new();
        let _ = ikhtisarat.insert("tabdeel".to_owned(), "Alt+T".to_owned());
        let _ = ikhtisarat.insert("tarjim_alaan".to_owned(), "Alt+Space".to_owned());
        let _ = ikhtisarat.insert("tahrir_manatiq".to_owned(), "Alt+R".to_owned());
        let _ = ikhtisarat.insert("sijill_qira".to_owned(), "Alt+H".to_owned());
        let _ = ikhtisarat.insert("lawhat_tahakkum".to_owned(), "Alt+`".to_owned());
        Self {
            mufaal: false,
            muharrik_qira: MuharrikQira::NizamAsli,
            shaffafiya: 0.82,
            hajm_khatt: 22.0,
            mawqi: MawqiTabaqa::Fawq,
            tul_sijill_qira: 500,
            ikhtisarat,
        }
    }
}

/// The recorded acknowledgement that Taarib modifies game files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Iqrar {
    /// Which revision of the statement was agreed to. A changed statement is a
    /// new acknowledgement, not an assumed one.
    pub isdar_bayan: u32,
    /// When it was given, RFC 3339.
    pub waqt: String,
    /// Which build of Taarib asked.
    pub isdar_taarib: String,
}

/// The live settings store.
///
/// Readers take a snapshot with [`MakhzanIdadat::hali`], which is a pointer
/// read and never blocks. Writers replace the whole tree, save it atomically,
/// and notify watchers.
pub struct MakhzanIdadat {
    masar: PathBuf,
    hali: ArcSwap<Idadat>,
    muraqiboon: Mutex<Vec<Box<dyn Fn(&Idadat) + Send + Sync>>>,
    /// Held across the whole of [`MakhzanIdadat::ghayyir`]. Reads go through
    /// [`ArcSwap`] and never touch it; writers take it so that read, modify,
    /// save and publish are one step rather than four interleavable ones.
    qufl_tabdeel: Mutex<()>,
}

impl std::fmt::Debug for MakhzanIdadat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MakhzanIdadat")
            .field("masar", &self.masar)
            .field("muraqiboon", &self.muraqiboon.lock().len())
            .finish_non_exhaustive()
    }
}

impl MakhzanIdadat {
    /// Loads settings, applying the environment layer over the file layer.
    ///
    /// A missing file is not a failure — it is a first run, and produces the
    /// defaults. A *corrupt* file is a failure, because silently replacing a
    /// user's configuration with defaults is how people lose their provider
    /// setup and their library paths.
    ///
    /// A file that is merely missing one field counts as corrupt here, and if a
    /// shipped build ever produces that, the cause is a field added without
    /// moving [`Idadat::ISDAR`] and giving `hijra` an arm — not the file. The
    /// user's settings are still intact on disk either way: nothing is written
    /// until a change succeeds.
    ///
    /// # Errors
    ///
    /// Fails when the file exists but cannot be read, parsed, or migrated.
    pub fn iftah(masarat: &Masarat) -> Natija<Self> {
        let masar = masarat.malaf_idadat();
        let asas = if masar.exists() {
            mukhattat::iqra_malaf::<Idadat>(&masar)?
        } else {
            Idadat::default()
        };
        let maa_beea = tabaqat_beea(asas)?;
        Ok(Self {
            masar,
            hali: ArcSwap::from_pointee(maa_beea),
            muraqiboon: Mutex::new(Vec::new()),
            qufl_tabdeel: Mutex::new(()),
        })
    }

    /// Builds a store over an explicit value, without touching the disk.
    #[must_use]
    pub fn min_qeema(masar: PathBuf, idadat: Idadat) -> Self {
        Self {
            masar,
            hali: ArcSwap::from_pointee(idadat),
            muraqiboon: Mutex::new(Vec::new()),
            qufl_tabdeel: Mutex::new(()),
        }
    }

    /// The current settings. Cheap enough to call on any path.
    #[must_use]
    pub fn hali(&self) -> Arc<Idadat> {
        self.hali.load_full()
    }

    /// Applies a change, saves it atomically, and notifies watchers.
    ///
    /// The change is only published after the save succeeds, so a failed write
    /// leaves the running configuration exactly as the file on disk says it is.
    ///
    /// Serialized against other changes. Reading the current value, applying
    /// the closure, writing the file and publishing the result is one step:
    /// two callers that each read the same base would otherwise each write a
    /// whole settings tree built from it, and the second would erase the
    /// first's field along with the file it had already saved.
    ///
    /// # Errors
    ///
    /// Fails when the settings cannot be serialized or written.
    pub fn ghayyir(&self, tabdeel: impl FnOnce(&mut Idadat)) -> Natija<()> {
        let _haris = self.qufl_tabdeel.lock();
        let mut jadeed = (*self.hali.load_full()).clone();
        tabdeel(&mut jadeed);
        mukhattat::iktub_malaf(&self.masar, &jadeed)?;
        let jadeed = Arc::new(jadeed);
        self.hali.store(Arc::clone(&jadeed));
        for muraqib in self.muraqiboon.lock().iter() {
            muraqib(&jadeed);
        }
        Ok(())
    }

    /// Registers a watcher, called after every successful change.
    pub fn raqib(&self, muraqib: impl Fn(&Idadat) + Send + Sync + 'static) {
        self.muraqiboon.lock().push(Box::new(muraqib));
    }

    /// Where the settings file lives.
    #[must_use]
    pub fn masar(&self) -> &std::path::Path {
        &self.masar
    }
}

/// Applies `TAARIB_*` environment overrides over a settings value.
///
/// # Errors
///
/// Fails when an override names a path that does not exist in the tree, or
/// carries a value the field cannot hold — both of which are typing mistakes
/// worth reporting rather than ignoring.
fn tabaqat_beea(asas: Idadat) -> Natija<Idadat> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this function is the configuration layer the rule points every other \
                  caller towards"
    )]
    let mutaghayyirat: Vec<(String, String)> = std::env::vars()
        .filter(|(miftah, _)| miftah.starts_with(BADIYAT_BEEA))
        // Some names carrying this prefix are not settings paths at all — the
        // two data roots, read before a settings file can be located, and the
        // build script's target triple, which cargo also injects at run time.
        // Scanning those resolved them against the settings tree, found no such
        // field, and refused. The refusal is right for a typo and catastrophic
        // for these: two of the three made the application refuse to start when
        // used exactly as documented.
        .filter(|(miftah, _)| !ASMAA_GHAYR_IDADAT.contains(&miftah.as_str()))
        .collect();

    if mutaghayyirat.is_empty() {
        return Ok(asas);
    }

    let mut qeema = serde_json::to_value(&asas).map_err(|q| {
        Khata::min_tafsir(&KhataIdadat::TaadhurTahweel {
            tafsil: q.to_string(),
        })
    })?;

    for (miftah, nass) in mutaghayyirat {
        let masar: Vec<String> = miftah
            .trim_start_matches(BADIYAT_BEEA)
            .to_ascii_lowercase()
            .split("__")
            .map(str::to_owned)
            .collect();
        // A bare value that is not JSON is taken as a string, so
        // TAARIB_LUGHA=injilizi works without quoting.
        let mahmul =
            serde_json::from_str::<Value>(&nass).unwrap_or_else(|_| Value::String(nass.clone()));
        daa_fi_masar(&mut qeema, &masar, mahmul, &miftah)?;
    }

    serde_json::from_value(qeema).map_err(|q| {
        Khata::min_tafsir(&KhataIdadat::TaadhurTahweel {
            tafsil: q.to_string(),
        })
    })
}

fn daa_fi_masar(hadaf: &mut Value, masar: &[String], qeema: Value, asl: &str) -> Natija<()> {
    let Some((awwal, baqi)) = masar.split_first() else {
        return Ok(());
    };
    let Some(kain) = hadaf.as_object_mut() else {
        return Err(Khata::min_tafsir(&KhataIdadat::TajawuzMajhul {
            miftah: asl.to_owned(),
        }));
    };
    if baqi.is_empty() {
        if !kain.contains_key(awwal) {
            return Err(Khata::min_tafsir(&KhataIdadat::TajawuzMajhul {
                miftah: asl.to_owned(),
            }));
        }
        let _ = kain.insert(awwal.clone(), qeema);
        return Ok(());
    }
    let Some(dakhili) = kain.get_mut(awwal) else {
        return Err(Khata::min_tafsir(&KhataIdadat::TajawuzMajhul {
            miftah: asl.to_owned(),
        }));
    };
    daa_fi_masar(dakhili, baqi, qeema, asl)
}

/// Failures of configuration.
#[derive(Debug, thiserror::Error)]
pub enum KhataIdadat {
    /// An environment override names a setting that does not exist.
    #[error("unknown setting override: {miftah}")]
    TajawuzMajhul {
        /// The environment variable that was set.
        miftah: String,
    },

    /// The settings tree does not round-trip through JSON.
    #[error("settings do not match their schema: {tafsil}")]
    TaadhurTahweel {
        /// What serde reported.
        tafsil: String,
    },
}

impl Tafsir for KhataIdadat {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::IDADAT
                + match self {
                    Self::TajawuzMajhul { .. } => 0,
                    Self::TaadhurTahweel { .. } => 1,
                },
        )
    }

    fn arabi(&self) -> String {
        match self {
            Self::TajawuzMajhul { miftah } => {
                format!("متغيّر البيئة {miftah} يشير إلى إعداد غير موجود، ولم يُطبَّق.")
            },
            Self::TaadhurTahweel { .. } => {
                "ملف الإعدادات لا يطابق الشكل المتوقّع، ولم يُحمَّل حفاظًا على ما فيه.".to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::TajawuzMajhul { miftah } => {
                format!("Environment variable {miftah} names a setting that does not exist.")
            },
            Self::TaadhurTahweel { tafsil } => {
                format!("The settings file does not match its expected shape: {tafsil}")
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        Khutwa::FathIdadat {
            qism: QismIdadat::Tashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TajawuzMajhul { miftah } => {
                let _ = siyaq.insert("miftah".to_owned(), QeemaSiyaq::Nass(miftah.clone()));
            },
            Self::TaadhurTahweel { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataIdadat);

#[cfg(test)]
mod ikhtibarat {
    use super::{
        HalatMuzawwidin, Idadat, IdadatMuzawwid, IdadatMuzawwidin, MUARRIF_GOOGLE_MAJJANI,
        NAMUDHAJ_GOOGLE_MAJJANI, NawMuzawwid,
    };

    fn tarif(muarrif: &str, mufaal: bool) -> IdadatMuzawwid {
        IdadatMuzawwid {
            muarrif: muarrif.to_owned(),
            naw: NawMuzawwid::Mahalli,
            namudhaj: "qwen2.5:7b".to_owned(),
            asas: None,
            hisab_miftah: None,
            mufaal,
            hadd_talabat: 60,
            mizaniya: None,
        }
    }

    /// A fresh installation has no provider of its own, and that is a state
    /// with a name — and it still translates, through the built-in free one.
    #[test]
    fn al_tarkeeb_al_jadeed_faragh() {
        let idadat = Idadat::default();
        assert!(idadat.muzawwidun.qaima.is_empty());
        assert_eq!(idadat.muzawwidun.iftiradi, None);
        assert_eq!(idadat.muzawwidun.hala(), HalatMuzawwidin::Faragh);
        assert_eq!(idadat.muzawwidun.muntakhab(), None);
        let badeel = idadat.muzawwidun.muntakhab_aw_majjani();
        assert_eq!(badeel.naw, NawMuzawwid::GoogleMajjani);
        assert_eq!(badeel.muarrif, MUARRIF_GOOGLE_MAJJANI);
        assert_eq!(badeel.namudhaj, NAMUDHAJ_GOOGLE_MAJJANI);
        assert!(badeel.mufaal);
        assert_eq!(badeel.mizaniya, None, "there is nothing to cap");
        assert_eq!(badeel.hisab_miftah, None, "and no key to look up");
    }

    /// The built-in provider stands in only when the list elects nothing: a
    /// provider the user set up and switched on always wins over it, and a
    /// list of switched-off providers falls back to it exactly as an empty one
    /// does.
    #[test]
    fn al_majjani_yaqif_makan_al_ghaib_faqat() {
        let mufaal = IdadatMuzawwidin {
            qaima: vec![tarif("ollama", true)],
            iftiradi: None,
        };
        assert_eq!(mufaal.muntakhab_aw_majjani().muarrif, "ollama");

        let muattal = IdadatMuzawwidin {
            qaima: vec![tarif("ollama", false)],
            iftiradi: Some("ollama".to_owned()),
        };
        assert_eq!(
            muattal.muntakhab_aw_majjani().naw,
            NawMuzawwid::GoogleMajjani
        );
        assert_eq!(
            muattal.muntakhab_aw_majjani(),
            IdadatMuzawwidin::default().muntakhab_aw_majjani(),
            "off and absent fall back to the same entry"
        );
    }

    /// Configured and switched off is not the same answer as nothing configured:
    /// the two have different remedies, and both used to produce one sentence
    /// telling the second user to add a provider they already had.
    #[test]
    fn al_muattal_laysa_kal_faragh() {
        let muzawwidun = IdadatMuzawwidin {
            qaima: vec![tarif("ollama", false)],
            iftiradi: None,
        };
        assert_eq!(muzawwidun.hala(), HalatMuzawwidin::Muattala);
        assert_eq!(muzawwidun.muntakhab(), None);
        assert_ne!(
            HalatMuzawwidin::Muattala.arabi(),
            HalatMuzawwidin::Faragh.arabi()
        );
        assert_ne!(
            HalatMuzawwidin::Muattala.injilizi(),
            HalatMuzawwidin::Faragh.injilizi()
        );
    }

    /// A default that is switched off elects somebody else, and says so.
    #[test]
    fn al_iftiradi_al_muattal_yuntij_badeelan() {
        let muzawwidun = IdadatMuzawwidin {
            qaima: vec![tarif("ghali", false), tarif("rakhees", true)],
            iftiradi: Some("ghali".to_owned()),
        };
        assert_eq!(muzawwidun.hala(), HalatMuzawwidin::Badeel);
        assert_eq!(
            muzawwidun.muntakhab().map(|tarif| tarif.muarrif.as_str()),
            Some("rakhees")
        );
    }

    /// A default that is present and on is the ordinary case and is silent.
    #[test]
    fn al_iftiradi_al_mufaal_mukhtar() {
        let muzawwidun = IdadatMuzawwidin {
            qaima: vec![tarif("ghali", true), tarif("rakhees", true)],
            iftiradi: Some("ghali".to_owned()),
        };
        assert_eq!(muzawwidun.hala(), HalatMuzawwidin::Mukhtar);
        assert_eq!(
            muzawwidun.muntakhab().map(|tarif| tarif.muarrif.as_str()),
            Some("ghali")
        );
    }

    /// No default at all, with one provider on, is the ordinary case too — not a
    /// substitution, because nothing was chosen to be substituted for.
    #[test]
    fn bila_iftiradi_awwal_mufaal_mukhtar() {
        let muzawwidun = IdadatMuzawwidin {
            qaima: vec![tarif("awwal", false), tarif("thani", true)],
            iftiradi: None,
        };
        assert_eq!(muzawwidun.hala(), HalatMuzawwidin::Mukhtar);
        assert_eq!(
            muzawwidun.muntakhab().map(|tarif| tarif.muarrif.as_str()),
            Some("thani")
        );
    }

    /// No state withholds translation any more, so the two that used to say
    /// "does not start" now say which service is doing the work and where a
    /// better one is chosen. Every sentence still carries the half a bare "no
    /// provider" leaves out: installing a published patch reaches no provider.
    #[test]
    fn al_halatan_al_faraghiyatan_tusammiyan_al_majjani() {
        for hala in [
            HalatMuzawwidin::Faragh,
            HalatMuzawwidin::Muattala,
            HalatMuzawwidin::Mukhtar,
            HalatMuzawwidin::Badeel,
        ] {
            assert!(!hala.ism().is_empty());
            assert!(!hala.arabi().is_empty());
            assert!(!hala.injilizi().is_empty());
        }
        for hala in [HalatMuzawwidin::Faragh, HalatMuzawwidin::Muattala] {
            assert!(
                hala.injilizi()
                    .contains("Installing published patches still works")
            );
            assert!(hala.arabi().contains("تثبيت الرقع المنشورة يعمل"));
            assert!(hala.injilizi().contains("free Google Translate"));
            assert!(hala.injilizi().contains("Settings, under Providers"));
            assert!(hala.arabi().contains("Google المجانية"));
            assert!(hala.arabi().contains("الإعدادات ← المزوّدون"));
        }
        assert!(!HalatMuzawwidin::Mukhtar.injilizi().contains("free"));
    }
}
