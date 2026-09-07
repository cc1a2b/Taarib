//! النص — a translatable string, its context, its constraints, and its status.
//!
//! Everything downstream of extraction works on [`MudkhalNass`]. A string is
//! never just a pair of texts: it carries where it came from, where it appears
//! in play, how much room it has, what markup and placeholders it must preserve,
//! and what is currently wrong with its translation.
//!
//! Markup and format placeholders are extracted at import into [`NitaqNasq`]
//! spans over *clean* text, so no translator, no machine-translation provider,
//! and no part of the layout engine ever sees a raw `<color=#ff0000>` or a bare
//! `%1$s`. That single decision is what makes placeholder corruption
//! detectable rather than inevitable.

use serde::{Deserialize, Serialize};
use taarib_usus::Khutura;
use uuid::Uuid;

use crate::NITAQ_TAARIB;
use crate::muraja::SijillMuraja;
use crate::musahim::MusahimId;
use crate::ruqaa::TareeqaTarjama;

/// A string's identity.
///
/// Derived from the container, the location inside it, and the source text, so
/// extracting the same build twice produces the same identities, and a string
/// that moved inside its file keeps its identity as long as its text and field
/// did not change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct NassId(Uuid);

impl NassId {
    /// Derives a string identity from its origin.
    #[must_use]
    pub fn min_mawqi(hawiya: &str, mawqi: &str, masdar: &str) -> Self {
        let bidhra = format!("{hawiya}\u{1}{mawqi}\u{1}{masdar}");
        Self(Uuid::new_v5(&NITAQ_TAARIB, bidhra.as_bytes()))
    }

    /// Wraps a stored identity.
    #[must_use]
    pub const fn min_uuid(qeema: Uuid) -> Self {
        Self(qeema)
    }

    /// The underlying value.
    #[must_use]
    pub const fn uuid(self) -> Uuid {
        self.0
    }

    /// A short form for dense tables, where a full identity is noise.
    #[must_use]
    pub fn mukhtasar(self) -> String {
        self.0.as_simple().to_string().chars().take(8).collect()
    }
}

impl std::fmt::Display for NassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

/// What a markup span does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum NawNasq {
    /// A colour, as the engine wrote it.
    Lawn {
        /// The colour value, `#rrggbb`, `#rrggbbaa`, or a named colour.
        qeema: String,
    },
    /// Bold.
    Ghaliz,
    /// Italic.
    Maail,
    /// Underline.
    TahtKhat,
    /// Strikethrough.
    Shatb,
    /// An explicit size, in the engine's own units.
    Hajm {
        /// The size.
        qeema: f32,
    },
    /// A different font family, named by the engine.
    Khatt {
        /// The family name.
        ism: String,
    },
    /// An inline sprite or icon, which occupies width and takes part in layout
    /// as a single atom but is drawn by the engine, not by Taarib.
    Sura {
        /// The engine's own reference to it.
        marja: String,
    },
    /// A link payload the engine resolves on click.
    Rabt {
        /// The payload.
        himl: String,
    },
    /// A format placeholder — `{0}`, `{name}`, `%s`, `%1$d`, `\V[3]`, `$gold`.
    /// Its text is opaque, its width is measured, and any translation that
    /// loses it is rejected before it can ship.
    Mawdi {
        /// The placeholder exactly as it appeared.
        khaam: String,
    },
    /// A hard line break the original text asked for.
    Satr,
    /// A typewriter pause, in the engine's own units.
    Tawaqquf {
        /// The duration.
        qeema: u32,
    },
    /// A region the engine is told not to interpret.
    BilaTahleel,
    /// An alignment override.
    Muhadhaha {
        /// `right`, `left`, `center`, or `justify` as the engine spells it.
        qeema: String,
    },
}

impl NawNasq {
    /// Whether this span is an atom that must survive translation untouched.
    #[must_use]
    pub const fn dharra(&self) -> bool {
        matches!(self, Self::Mawdi { .. } | Self::Sura { .. })
    }
}

/// One markup span over the clean text.
///
/// `bidaya` and `tul` are byte offsets into the clean UTF-8 text, never
/// character counts, because every stage downstream — shaping, bidi, cluster
/// mapping — works in bytes and a mixed unit is a bug waiting for its first
/// non-ASCII string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct NitaqNasq {
    /// Identifies this span within its string, so a glyph can be traced back to
    /// the style it belongs to.
    pub id: u16,
    /// Byte offset of the span's first byte in the clean text.
    pub bidaya: u32,
    /// Length of the span in bytes.
    pub tul: u32,
    /// What the span does.
    pub naw: NawNasq,
}

/// A rectangle in the game's own pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Mustatil {
    /// Left edge.
    pub s: f32,
    /// Top edge.
    pub a: f32,
    /// Width.
    pub ard: f32,
    /// Height.
    pub irtifa: f32,
}

/// How much room a string actually has.
///
/// Constraints are measured, not assumed: the pixel width comes from the
/// rectangle the original text was drawn into during runtime capture, and the
/// character limit comes from the engine where the engine imposes one.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct QuyudNass {
    /// A hard character limit imposed by the engine or the data format.
    pub aqsa_ahruf: Option<u32>,
    /// The width available in the interface, in the game's pixels.
    pub aqsa_ard: Option<f32>,
    /// The height available, where the text may wrap.
    pub aqsa_irtifa: Option<f32>,
    /// The size the game draws this string at.
    pub hajm_khatt: Option<f32>,
    /// The rectangle the original occupied on screen.
    pub mustatil: Option<Mustatil>,
    /// Whether the engine refuses to wrap this string.
    pub satr_wahid: bool,
}

/// Where a string comes from and where it appears.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct SiyaqNass {
    /// The container it was extracted from, relative to the game's root.
    pub hawiya: String,
    /// The location inside that container, precise enough to write back to:
    /// an object path, an event command index, a namespace and key, a line.
    pub mawqi: String,
    /// The field or property name, where the container has one.
    pub haql: Option<String>,
    /// The speaker, for dialogue.
    pub mutakallim: Option<String>,
    /// The lines around it, which is what makes machine translation of dialogue
    /// worth anything.
    pub jiwar: Vec<String>,
    /// The scene or screen it was seen in during runtime capture.
    pub mashhad: Option<String>,
    /// A screenshot crop showing where it appeared, content-addressed.
    pub laqta: Option<String>,
}

/// Something wrong, or possibly wrong, with a translation.
///
/// Every flag is computed from real data — pixel widths come from the layout
/// engine, terminology from the glossary — and every one links back to the
/// strings that caused it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum AlamJawda {
    /// The translation is wider than the space it has.
    KhatarTajawuz {
        /// Measured width of the translation, in the game's pixels.
        ard: f32,
        /// Width available.
        mutah: f32,
        /// The size it was measured at.
        hajm: f32,
    },
    /// Latin text survives inside an otherwise Arabic translation, which is
    /// usually an untranslated fragment rather than an intentional name.
    NassLatiniMutabaqqi {
        /// How many Latin words remain.
        adad: u32,
    },
    /// A glossary term was translated inconsistently.
    MustalahMukhtalif {
        /// The source term.
        mustalah: String,
        /// The canonical Arabic it should have used.
        mutawaqqa: String,
    },
    /// A placeholder or markup span was lost, duplicated, or corrupted.
    NasqMaksur {
        /// The atoms that are missing or damaged.
        mafqud: Vec<String>,
    },
    /// The provider reported low confidence in this translation.
    ThiqaMunkhafida {
        /// Confidence, 0.0 to 1.0.
        qeema: f32,
    },
    /// The translation is suspiciously longer or shorter than its source.
    NisbaShadha {
        /// Target length divided by source length.
        nisba: f32,
    },
    /// The target is empty while the source is not.
    Farigh,
    /// Two identical source strings were translated differently, which players
    /// notice immediately when both appear on the same screen.
    TarjamaMutanaqida {
        /// The other string.
        akhar: NassId,
    },
    /// A machine produced this translation and no human has read it yet.
    ///
    /// A workflow fact carried as a quality flag, deliberately: reviewers work
    /// from the flag list, and machine text with no *defect* flags would
    /// otherwise read as "nothing wrong here" and be skipped — which is exactly
    /// how unreviewed machine output ends up shipping under a contributor's
    /// name. Computed from the review record's machine-only state and removed
    /// by the same computation the moment a human touches the string; never
    /// synthesised from the text itself.
    AaliyaBilaMuraja,
}

impl AlamJawda {
    /// How much this flag matters.
    #[must_use]
    pub const fn khutura(&self) -> Khutura {
        match self {
            Self::NasqMaksur { .. } | Self::Farigh => Khutura::Fadih,
            Self::KhatarTajawuz { .. } | Self::MustalahMukhtalif { .. } => Khutura::Khatar,
            Self::NassLatiniMutabaqqi { .. }
            | Self::ThiqaMunkhafida { .. }
            | Self::NisbaShadha { .. }
            | Self::TarjamaMutanaqida { .. }
            | Self::AaliyaBilaMuraja => Khutura::Tanbeeh,
        }
    }

    /// The sentence the workspace shows beside the string, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::KhatarTajawuz { ard, mutah, hajm } => format!(
                "أعرض من المساحة المتاحة: {ard:.0} بكسل مقابل {mutah:.0} عند حجم {hajm:.0}."
            ),
            Self::NassLatiniMutabaqqi { adad } => {
                format!("بقيت {adad} كلمة لاتينية داخل الترجمة.")
            },
            Self::MustalahMukhtalif {
                mustalah,
                mutawaqqa,
            } => {
                format!("المصطلح «{mustalah}» تُرجم بغير المعتمد في المسرد: «{mutawaqqa}».")
            },
            Self::NasqMaksur { mafqud } => {
                format!("عناصر ناقصة أو تالفة في الترجمة: {}.", mafqud.join("، "))
            },
            Self::ThiqaMunkhafida { qeema } => {
                format!("ثقة الترجمة الآلية منخفضة ({:.0}٪).", qeema * 100.0)
            },
            Self::NisbaShadha { nisba } => {
                format!("طول الترجمة غير متناسب مع الأصل (النسبة {nisba:.1}).")
            },
            Self::Farigh => "النص الأصلي غير فارغ والترجمة فارغة.".to_owned(),
            Self::TarjamaMutanaqida { .. } => {
                "نص أصلي مطابق تُرجم بصيغة مختلفة في موضع آخر.".to_owned()
            },
            Self::AaliyaBilaMuraja => "ترجمة آلية لم يقرأها إنسان بعد.".to_owned(),
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::KhatarTajawuz { ard, mutah, hajm } => format!(
                "Wider than the space available: {ard:.0}px against {mutah:.0}px at size {hajm:.0}."
            ),
            Self::NassLatiniMutabaqqi { adad } => {
                format!("{adad} Latin words remain inside the translation.")
            },
            Self::MustalahMukhtalif {
                mustalah,
                mutawaqqa,
            } => format!(
                "The term \"{mustalah}\" was not translated as the glossary requires: \
                 \"{mutawaqqa}\"."
            ),
            Self::NasqMaksur { mafqud } => {
                format!(
                    "Missing or damaged elements in the translation: {}.",
                    mafqud.join(", ")
                )
            },
            Self::ThiqaMunkhafida { qeema } => {
                format!(
                    "Machine translation confidence is low ({:.0}%).",
                    qeema * 100.0
                )
            },
            Self::NisbaShadha { nisba } => {
                format!(
                    "Translation length is out of proportion with the source (ratio {nisba:.1})."
                )
            },
            Self::Farigh => "The source is not empty but the translation is.".to_owned(),
            Self::TarjamaMutanaqida { .. } => {
                "An identical source string was translated differently elsewhere.".to_owned()
            },
            Self::AaliyaBilaMuraja => "Machine-translated and not yet read by a human.".to_owned(),
        }
    }
}

/// What kind of string this is, derived from where it was found.
///
/// **Derived, never guessed from content.** A string reading `Attack` is an
/// item name in one game, a menu label in another and an animation state name
/// in a third, and nothing about the six characters distinguishes them. What
/// does distinguish them is the field, the asset and the object path they came
/// out of, which the extractor knows and the string does not carry.
///
/// The distinction matters because it is what the translator sees first and
/// what machine translation is prompted with: `Attack` as a menu label and
/// `Attack` as a dialogue line want different Arabic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TasnifNass {
    /// Spoken or narrated dialogue.
    Hiwar,
    /// A choice the player picks between.
    Ikhtiyar,
    /// The name of an item, skill, character or place.
    Ism,
    /// Descriptive body text for one of those.
    Wasf,
    /// A menu label, a button, a tab.
    Qaima,
    /// Hover text and help.
    Tafseer,
    /// A system message: saved, loaded, connection lost.
    Nizam,
    /// An error the player can see.
    Khata,
    /// Credits, licences, legal text.
    Nusub,
    /// **Not shown to a player.** An asset path, a node name, an enum label, a
    /// shader name, a debug line, an internal key.
    Dakhili,
    /// The extractor could not tell.
    ///
    /// Distinct from [`TasnifNass::Dakhili`] and the distinction is the whole
    /// point: "I know this is internal" and "I do not know what this is" lead
    /// to different interface defaults and different translator behaviour.
    Majhul,
}

impl TasnifNass {
    /// Whether a player is believed to see this string.
    ///
    /// The interface hides everything answering `false` by default and lets the
    /// translator reveal it. Nothing is ever deleted on the strength of this —
    /// see [`MudkhalNass::tasnif`] for why.
    #[must_use]
    pub const fn yaraha_allaib(self) -> bool {
        !matches!(self, Self::Dakhili)
    }

    /// The label the interface groups by, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Hiwar => "حوار",
            Self::Ikhtiyar => "خيار",
            Self::Ism => "اسم",
            Self::Wasf => "وصف",
            Self::Qaima => "قائمة",
            Self::Tafseer => "تفسير",
            Self::Nizam => "رسالة نظام",
            Self::Khata => "رسالة خطأ",
            Self::Nusub => "نصوص ثابتة",
            Self::Dakhili => "داخلي",
            Self::Majhul => "غير مصنَّف",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Hiwar => "dialogue",
            Self::Ikhtiyar => "choice",
            Self::Ism => "name",
            Self::Wasf => "description",
            Self::Qaima => "menu",
            Self::Tafseer => "tooltip",
            Self::Nizam => "system message",
            Self::Khata => "error text",
            Self::Nusub => "credits and legal",
            Self::Dakhili => "internal",
            Self::Majhul => "unclassified",
        }
    }
}

/// How a string reached the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MasdarIstikhraj {
    /// Read out of the game's files.
    Sakin,
    /// Seen on screen while the game ran.
    ///
    /// The only source that can measure the width a string actually had, which
    /// is why capture supplements static extraction rather than replacing it.
    Multaqat,
    /// Both, and they agree.
    Kilahuma,
    /// Both, and they **disagree** — the file says one thing and the screen
    /// showed another.
    ///
    /// A real and informative state, not a bug to be resolved by picking one.
    /// A game that substitutes at runtime — a name into a greeting, a plural
    /// form, a platform-specific button glyph — produces this legitimately, and
    /// the translator needs to know the file's text is not the whole story.
    Mutanaqid,
}

impl MasdarIstikhraj {
    /// Whether runtime capture contributed.
    #[must_use]
    pub const fn multaqat(self) -> bool {
        matches!(self, Self::Multaqat | Self::Kilahuma | Self::Mutanaqid)
    }

    /// Whether static extraction contributed, which is what makes a string
    /// writable back into the game's files.
    ///
    /// A capture-only string has no location to write to: it was seen on screen
    /// and its origin is unknown, so patching it needs the runtime path rather
    /// than the container rewriter.
    #[must_use]
    pub const fn sakin(self) -> bool {
        matches!(self, Self::Sakin | Self::Kilahuma | Self::Mutanaqid)
    }

    /// Folds a second sighting into the first.
    ///
    /// `muttafiq` is whether the two texts matched. Written as a fold rather
    /// than left to callers because merging capture into a static table happens
    /// once per string in a table of tens of thousands, and a caller getting
    /// the disagreement case wrong would silently lose the one signal that says
    /// the file's text is not what the player reads.
    #[must_use]
    pub const fn adif(self, akhar: Self, muttafiq: bool) -> Self {
        match (self, akhar) {
            (Self::Mutanaqid, _) | (_, Self::Mutanaqid) => Self::Mutanaqid,
            (Self::Sakin, Self::Sakin) => Self::Sakin,
            (Self::Multaqat, Self::Multaqat) => Self::Multaqat,
            _ if muttafiq => Self::Kilahuma,
            _ => Self::Mutanaqid,
        }
    }
}

/// One translatable string, and everything the product knows about it.
///
/// The unit every other phase moves: extraction produces these, the workspace
/// edits them, the compiler lays them out, and a patch is a set of them. The
/// source text and the translation are kept *clean* — markup and placeholders
/// live beside them in [`nasq_masdar`](Self::nasq_masdar) and
/// [`nasq_hadaf`](Self::nasq_hadaf) rather than inside the strings — because a
/// translator who has to preserve `<color=#ff0>{0}</color>` by hand will
/// eventually not, and a shaper handed markup will shape the markup.
///
/// Its review state is a value, not a settable field: see
/// [`muraja`](Self::muraja).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MudkhalNass {
    /// Its identity.
    pub id: NassId,
    /// The clean source text, with markup and placeholders lifted into spans.
    pub masdar: String,
    /// The clean Arabic translation, when there is one.
    pub hadaf: Option<String>,
    /// Where it stands in the workflow, who moved it there, and when.
    ///
    /// Carries its own state rather than exposing a settable field, which is
    /// what makes "a machine translation is never approved" a property of the
    /// type: the approval transition consumes an attestation only a human
    /// review can construct. See [`crate::muraja`].
    pub muraja: SijillMuraja,
    /// Where it came from and where it appears.
    pub siyaq: SiyaqNass,
    /// How much room it has.
    pub quyud: QuyudNass,
    /// Markup and placeholders over the source.
    pub nasq_masdar: Vec<NitaqNasq>,
    /// Markup and placeholders over the translation.
    pub nasq_hadaf: Vec<NitaqNasq>,
    /// How many times this exact source text occurs in the game.
    pub takrar: u32,
    /// The duplicate group, so one translation serves every identical source.
    pub majmua: Option<NassId>,
    /// Everything currently wrong with it.
    pub alamat: Vec<AlamJawda>,
    /// How the translation was produced.
    pub tareeqa: Option<TareeqaTarjama>,
    /// Which provider produced it, when it was machine-translated.
    pub muzawwid: Option<String>,
    /// Who last changed it.
    pub muharrir: Option<MusahimId>,
    /// When it was last changed, RFC 3339.
    pub akhir_tabdeel: Option<String>,
    /// What kind of string this is.
    ///
    /// **Never a reason to delete anything.** A translation project drowning in
    /// asset paths is unusable, so the interface hides
    /// [`TasnifNass::Dakhili`] by default — but every string is retained with
    /// its classification and its confidence, a translator can reveal them, and
    /// reclassifying one persists into the project. Silently dropping strings
    /// is how a game ships with one untranslated menu nobody can find the
    /// source of.
    pub tasnif: TasnifNass,
    /// How sure the extractor is of that classification, zero to a hundred.
    ///
    /// Carried beside the classification rather than folded into it, because
    /// "menu label, and I am certain" and "menu label, and I am guessing" want
    /// different treatment: the interface sorts the uncertain ones to the top
    /// of the review pass, and machine translation is prompted more cautiously
    /// for them.
    pub thiqat_tasnif: u8,
    /// How this string reached the table.
    pub masdar_istikhraj: MasdarIstikhraj,
    /// The encoding it was stored in, as the container declared it.
    ///
    /// Recorded rather than normalized away, because writing back has to
    /// produce the same encoding: a Shift-JIS RPG Maker archive that reads as
    /// UTF-8 on the way out is a game that will not start.
    pub tarmiz: Option<String>,
}

impl MudkhalNass {
    /// Whether anything blocks this string from shipping.
    #[must_use]
    pub fn yamnaa_alnashr(&self) -> bool {
        self.alamat
            .iter()
            .any(|q| matches!(q.khutura(), Khutura::Fadih))
    }

    /// The atoms that must appear, unchanged, in any translation of this string.
    #[must_use]
    pub fn dharrat(&self) -> Vec<String> {
        self.nasq_masdar
            .iter()
            .filter_map(|nitaq| match &nitaq.naw {
                NawNasq::Mawdi { khaam } => Some(khaam.clone()),
                NawNasq::Sura { marja } => Some(marja.clone()),
                _ => None,
            })
            .collect()
    }
}
