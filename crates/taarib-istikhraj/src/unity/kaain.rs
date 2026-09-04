//! الكائنات — reading a Unity object's fields through the type tree, and
//! refusing to read them at all when the file does not carry one.
//!
//! [`crate::unity::hawiya`] produced a [`Mulsal`] holding an object table and,
//! for each type, either a [`ShajaratAnwa`] or nothing. This module is what
//! turns the first case into strings and the second case into a refusal.
//!
//! ## The rule this module exists to enforce
//!
//! **Without the type tree, nothing is extracted from the object.** Not a
//! partial read, not a scan for byte runs that look like text, not a
//! "recovered by pattern" table.
//!
//! The reason is not caution, it is arithmetic. Consider eight bytes reading
//! `05 00 00 00 48 65 6C 6C`. With a type tree saying `string` they are a
//! length-prefixed `Hello`; with one saying `SInt32, SInt32` they are `5` and
//! `1819043144`; with one saying `float, float` they are `7.0e-45` and
//! `2.7e+23`. Every one of those readings consumes exactly eight bytes and every
//! one of them leaves the cursor in the same place. **There is no evidence in
//! the file that distinguishes them.**
//!
//! A pattern scanner picks the reading that produces printable ASCII, which is
//! the one that looks best and is right by luck. The failure is not that it
//! produces nonsense — nonsense would be caught. The failure is that it produces
//! a *plausible* table: forty-one real strings and nine fragments that begin
//! mid-word or run three fields together. Those nine get translated, compiled
//! into a patch, and written back over field boundaries that were never there.
//! The corruption is found by a player, in a shipped build, and traced to Taarib.
//!
//! So [`iqra_kaain`] refuses, [`HasilatMulsal`] carries the refusal beside
//! whatever else was read, and the remedy the report offers is runtime capture —
//! where the strings are drawn on screen and can simply be read, with no
//! inference at all.
//!
//! ## Alignment is the other way a correct-looking read goes wrong
//!
//! Unity pads to a four-byte boundary after any field whose node carries
//! [`RAYA_MUHADHAHA`][super::hawiya::RAYA_MUHADHAHA], and that padding appears
//! in no field's declared size. A
//! walk that ignores one align flag is one to three bytes early for the whole
//! rest of the object — and, again, does not fail. It reads the next `string`'s
//! length prefix from three bytes of the previous field plus one of the real
//! prefix, gets a small number, and produces a short string of real characters.
//!
//! Every composite read in [`iqra_qeema`] therefore honours the flag on the node
//! *and* the promotion rule Unity's own reader uses: an `Array` or `map` child
//! whose node carries the flag aligns the parent. Getting that promotion wrong
//! is the single most reported bug in every third-party Unity reader.
//!
//! ## A `[SerializeReference]` field's layout is not in the node being walked
//!
//! One shape breaks the rule that the type tree describes everything under a
//! node: `ManagedReferencesRegistry`. Unity writes a `[SerializeReference]`
//! field as a `rid` and parks the object itself in a registry at the end of the
//! component, where each entry is a managed type name — `class`, `ns`, `asm` —
//! followed by that object's bytes. The node the tree gives those bytes,
//! `ReferencedObjectData`, is **empty**: no children, and a declared width of
//! zero. The layout lives in the file's *second* type list,
//! [`Mulsal::anwa_marjiiya`][super::hawiya::Mulsal::anwa_marjiiya] — `m_RefTypes`
//! — keyed by the three names the entry just wrote down.
//!
//! Walking the entry through the empty node consumes nothing and leaves the
//! cursor sitting on the payload, so the next entry's `class` length prefix is
//! read out of the middle of the previous object. The number that comes back is
//! ASCII text — `1734440295` is `guag`, four bytes of the word `language` — and
//! the file is reported as damaged. That is why [`SiyaqQira`] carries the
//! reference-type list beside the tree: a managed reference is walked against
//! the type the registry declares for it, or the object is refused, and never
//! walked against a node that describes nothing.
//!
//! ## What identity is built from here
//!
//! [`MawqiNass::mawqi`] gets the **object path** — `Canvas/Panel/DialogueText`,
//! built by walking `Transform.m_Father` up through `GameObject.m_Name`. That is
//! the structural position a contributor recognises and the thing that survives
//! the file being rewritten. [`MawqiNass::haql`] gets the component's class name
//! and the field path inside it, `TextMeshProUGUI.m_text`.
//!
//! An array index does appear inside `haql` for a `string[]` field, because a
//! bare list of strings has no other handle. It is confined to `haql` and never
//! reaches `mawqi`, and
//! [`MawqiNass::huwiya`][crate::jadwal::MawqiNass::huwiya] additionally hashes
//! the source text — so inserting an element renumbers the ones after it into
//! *new* identities rather than into each other's. The result is that the
//! translations show up as orphaned and re-matchable, which
//! [`FarqJadwal`][crate::jadwal::FarqJadwal] handles, instead of being silently
//! applied to the wrong lines, which nothing handles.
//!
//! Where the game ships a real localization system, none of that derivation is
//! used: Unity Localization's entry id and I2's term are the developer's own
//! stable keys, they go in [`MawqiNass::miftah_muharrik`], and identity comes
//! from them.
//!
//! ## Classification is not decided here
//!
//! Every entry is built through [`crate::tasnif::ansha_mudkhal`], which lifts
//! the markup, calls the one classifier this product has, and assembles the row.
//! This module's whole contribution to that decision is *supplying the names* —
//! the object path in [`MawqiNass::mawqi`], the component class and field in
//! [`MawqiNass::haql`], and the fact that a string came out of a localization
//! table. Nothing here scores a string.
//!
//! The one exception is [`TalabMudkhal::bi_tasrih`], and it is used exactly
//! twice, in both cases for a kind the *format declares* rather than one this
//! module infers: `m_Name` is Unity's own field for an object's asset name, and
//! I2's non-zero `TermType` says outright that the row holds an asset reference
//! rather than text. A guess in either place would belong in `crate::tasnif`
//! where it could compete with the other signals, which is precisely why it is
//! not one.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};
use taarib_mustalahat::nass::{QuyudNass, TasnifNass};

use crate::jadwal::{MawqiNass, MudkhalMustakhraj};
use crate::rafd::SababRafd;
use crate::tasnif::{THIQAT_BUNYA, TalabMudkhal, ansha_mudkhal};

use super::hawiya::{
    KhataQira, MadkhalKaain, MarjaKhariji, Mulsal, NawMulsal, Qari, SANF_GAMEOBJECT,
    SANF_MONOBEHAVIOUR, SANF_MONOSCRIPT, SANF_RECTTRANSFORM, SANF_TEXTASSET, SANF_TRANSFORM,
    ShajaratAnwa, UqdatShajara, tul_u64,
};

// ---------------------------------------------------------------------------
// Ceilings
// ---------------------------------------------------------------------------

/// How deep a value tree may nest before the read is refused.
///
/// Sixty-four. Unity's serializer refuses deeper than this itself, so a type
/// tree that nests further is a corrupt tree — and the reader is recursive, so
/// the alternative to a ceiling is a stack overflow, which is a crash this
/// process cannot catch and cannot report.
pub const AQSA_UMQ_QIRA: u8 = 64;

/// The most elements one array or map is reserved for up front.
///
/// Sixty-five thousand. Larger arrays are read, they simply grow as they go: the
/// declared count is a number from the file, and reserving on it directly is how
/// a corrupt length becomes an allocation failure instead of a refusal. The real
/// bound on the count is the bytes remaining, checked separately, because every
/// element occupies at least one byte.
pub const AQSA_HAJZ_ANASIR: usize = 65_536;

/// The longest string this build lifts out of an object.
///
/// A quarter of a mebibyte. A `TextAsset` holding a whole CSV reaches this, and
/// past it the value is a data file rather than a string — keeping it would put
/// a megabyte of unsplittable content in front of a translator as one row.
pub const AQSA_TUL_NASS: usize = 256 * 1024;

/// How many objects one file's path index will read.
///
/// Half a million. The index reads every `GameObject`, `Transform` and
/// `MonoScript` in the file so that a string's object path can be built, and a
/// large open-world scene reaches six figures. Past the ceiling the index is
/// marked truncated and paths fall back to the component's own name, which is
/// weaker identity honestly labelled rather than a wrong one.
pub const AQSA_KAAINAT_FAHRAS: usize = 500_000;

/// The oldest `ManagedReferencesRegistry` layout this build reads.
///
/// One. `int version`, then a `RefIds` vector of `{ rid, type, data }`.
pub const ISDAR_SIJILL_MARAJI_ADNA: i64 = 1;

/// The newest `ManagedReferencesRegistry` layout this build reads.
///
/// Two, which is what Unity 2022.3 writes. The two versions differ in what a
/// `rid` *means* — version one numbers references from zero within the object,
/// version two assigns each a globally unique id — and in not one byte of the
/// framing, which is why both are read by the same walk.
///
/// A third version is refused per object rather than assumed to be the same
/// again. Assuming would be the silent desynchronisation this whole module
/// exists to prevent, and refusing costs the objects that actually hold a
/// managed reference and nothing else in the file.
pub const ISDAR_SIJILL_MARAJI_AQSA: i64 = 2;

/// How many shared localization tables one thread's index holds.
///
/// Four thousand. A game ships one `SharedTableData` per string-table
/// collection and a large one has a few dozen; four thousand is far past that
/// and bounds a long-lived process — Studio extracts many games in one run —
/// against an index that only ever grows. Past it a shared table is still read
/// and still used inside its own container, it is simply not published for
/// another container to find.
pub const AQSA_JADAWIL_MUSHTARAKA: usize = 4096;

/// How far up a `Transform` parent chain the path walk goes.
///
/// Sixty-four. Deeper than any real hierarchy, and the walk also carries a
/// visited set: a `m_Father` cycle is not something Unity writes, but it is
/// something a corrupt file contains, and an unbounded walk on one never
/// returns.
pub const AQSA_UMQ_MASAR: usize = 64;

// ---------------------------------------------------------------------------
// Refusal
// ---------------------------------------------------------------------------

/// Why one object could not be read.
///
/// Per object, never per file. A `MonoBehaviour` whose type was stripped sits
/// beside a `TextAsset` whose type was not, in the same file, and refusing the
/// file for the first would lose the second.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KhataKaain {
    /// **The refusal this module exists for.** The object's serialized layout is
    /// not in the file.
    #[error("no type tree for {}", naw.as_deref().unwrap_or("this object"))]
    BilaShajaratAnwa {
        /// The type's name, when the file names one. It usually does not: a
        /// stripped build has no tree to take the name from, and the class id is
        /// all that is left.
        naw: Option<String>,
    },
    /// The tree is present and names a field this build cannot resolve.
    ///
    /// Treated exactly like a missing tree, and for the same reason: an unnamed
    /// node has an unknown width, so nothing after it can be read, and an
    /// unnamed field has an unknown identity, so two of them in one type would
    /// derive one [`NassId`][taarib_mustalahat::nass::NassId] for two strings.
    #[error(
        "type tree node {fahras} of {} names an unresolvable field",
        naw.as_deref().unwrap_or("this type")
    )]
    IsmMajhul {
        /// The type's name.
        naw: Option<String>,
        /// Which node.
        fahras: u32,
        /// The name offset that did not resolve.
        izaha: u32,
    },
    /// A field's bytes ran out, or a field held something the format cannot mean.
    #[error("field `{ism}`: {sabab}")]
    HaqlTalif {
        /// The field, as the type tree named it.
        ism: String,
        /// What went wrong underneath.
        sabab: KhataQira,
    },
    /// The object's own byte range is not inside the file.
    #[error(transparent)]
    Qira(#[from] KhataQira),
    /// The type tree's shape is not one the reader knows how to walk.
    ///
    /// A `vector` whose second node is not `Array`, a `map` with no `pair`. Not
    /// a truncation and not a version this build refuses — a structure that
    /// contradicts the format. Refused rather than walked past, because the
    /// reader would have to invent the missing node's width to continue.
    #[error("type tree for `{naw}` is malformed: {sabab}")]
    ShajaraShadha {
        /// The type whose tree it is.
        naw: String,
        /// What was wrong with it.
        sabab: String,
    },
    /// The value nested deeper than [`AQSA_UMQ_QIRA`], or an array declared more
    /// elements than the object has bytes.
    #[error("{hadd} declares {qeema}, above the ceiling of {saqf}")]
    TajawuzHadd {
        /// Which ceiling.
        hadd: &'static str,
        /// What was declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },
    /// A `[SerializeReference]` field names a managed type the file's
    /// reference-type list does not describe.
    ///
    /// Refused rather than skipped. The entry's payload follows immediately and
    /// its width is whatever that type's layout says it is, so a reader that
    /// walked past it would resume in the middle of an object — which is the
    /// desynchronisation that reports an intact file as damaged. Reported as a
    /// missing layout, because that is what it is: the bytes are there and
    /// nothing in the file says what they mean.
    #[error("no reference-type layout for managed type `{ism}`")]
    NawMarjiMajhul {
        /// The managed type's name, namespace included when it has one.
        ism: String,
    },
    /// The `ManagedReferencesRegistry` declares a version this build does not
    /// read.
    ///
    /// Separate from a corrupt field on purpose: the file is intact and this
    /// build is behind it. Reported as an unknown format so the remedy offered
    /// is runtime capture rather than "verify your game files", which would
    /// send a user to check a file that has nothing wrong with it.
    #[error("managed-reference registry version {isdar}, and this build reads versions 1 and 2")]
    SijillMarajiGhayrMadum {
        /// The version the file declared.
        isdar: i64,
    },
    /// A `TextAsset`'s bytes are not UTF-8 and were not converted.
    ///
    /// Refused rather than decoded lossily. A `TextAsset` is frequently a
    /// Shift-JIS or CP1252 script for a game whose author never intended it to
    /// be Unicode, and running it through a lossy conversion produces text with
    /// U+FFFD where every accented character was — which a translator would
    /// translate and a compiler would write back, destroying the original
    /// encoding in the process.
    #[error("`{asl}` is not UTF-8 at byte {mawqi}; its encoding is not declared anywhere")]
    TarmizGhayrMaruf {
        /// The asset's name.
        asl: String,
        /// The first byte that was not valid UTF-8.
        mawqi: usize,
    },
}

impl KhataKaain {
    /// The refusal the report shows for this failure.
    ///
    /// The mapping is the whole point of the type: three different internal
    /// faults all mean "the layout is not in the file" to a user, and one of
    /// them — an unresolvable common-string offset — would otherwise be reported
    /// as a corrupt container and send the user to verify their game files for
    /// no reason.
    #[must_use]
    pub fn ila_sabab(&self) -> SababRafd {
        match self {
            Self::BilaShajaratAnwa { naw } | Self::IsmMajhul { naw, .. } => {
                SababRafd::BilaShajaratAnwa { naw_kaen: naw.clone(), adad: 1 }
            }
            Self::NawMarjiMajhul { ism } => {
                SababRafd::BilaShajaratAnwa { naw_kaen: Some(ism.clone()), adad: 1 }
            }
            Self::TarmizGhayrMaruf { asl, .. } => SababRafd::SighaMajhula {
                wujid: format!("a TextAsset (`{asl}`) in an undeclared non-UTF-8 encoding"),
            },
            Self::SijillMarajiGhayrMadum { isdar } => SababRafd::SighaMajhula {
                wujid: format!("a Unity managed-reference registry of version {isdar}"),
            },
            Self::TajawuzHadd { hadd, qeema, saqf } => {
                SababRafd::TajawuzHadd { hadd: (*hadd).to_owned(), qeema: *qeema, saqf: *saqf }
            }
            Self::HaqlTalif { .. } | Self::Qira(_) | Self::ShajaraShadha { .. } => {
                SababRafd::Talif { sabab: self.to_string() }
            }
        }
    }

    /// Whether this failure means the layout was missing rather than damaged.
    ///
    /// What the caller groups on: a file with four thousand stripped
    /// `MonoBehaviour`s produces one report line, and a file with one corrupt
    /// object produces a different one.
    #[must_use]
    pub const fn bila_shajara(&self) -> bool {
        matches!(
            self,
            Self::BilaShajaratAnwa { .. } | Self::IsmMajhul { .. } | Self::NawMarjiMajhul { .. }
        )
    }
}

// ---------------------------------------------------------------------------
// The value model
// ---------------------------------------------------------------------------

/// A reference from one object to another.
///
/// Unity writes it as `m_FileID` and `m_PathID`, and every cross-object link in
/// a scene is one: a `MonoBehaviour` to its `GameObject`, a `Transform` to its
/// parent, a localization table to its shared data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ishara {
    /// Which file the target is in. `0` is this one; anything else indexes the
    /// external table.
    pub fahras_malaf: i32,
    /// The target's path id inside that file.
    pub hawiya_masar: i64,
}

impl Ishara {
    /// Whether the target is in this same file, and so resolvable here.
    #[must_use]
    pub const fn mahalli(self) -> bool {
        self.fahras_malaf == 0
    }

    /// Whether this reference points at nothing.
    #[must_use]
    pub const fn khali(self) -> bool {
        self.hawiya_masar == 0
    }
}

/// One value read out of an object.
///
/// Deliberately not `PartialEq`: the float arm would give the type a comparison
/// that answers `false` for two NaNs and `true` for `0.0` and `-0.0`, and a
/// reader whose values compare by an unclear rule invites a caller to compare
/// them.
#[derive(Debug, Clone)]
pub enum QeemaHaql {
    /// Any signed integer field, widened.
    Sahih(i64),
    /// Any unsigned integer field, widened.
    Ghayr(u64),
    /// `float`, at the width the file stored it.
    ///
    /// Kept separate from `double` rather than widened into one arm, because
    /// the one consumer of these — a font size going into
    /// [`QuyudNass::hajm_khatt`] — is an `f32`, and widening on the way in would
    /// force a narrowing cast on the way out for no gain. Unity writes
    /// `m_fontSize` as a `float`; this reads it as one.
    Ashri(f32),
    /// `double`.
    AshriMuda(f64),
    /// `bool`.
    Mantiqi(bool),
    /// A length-prefixed, four-byte-aligned string.
    Nass(String),
    /// `TypelessData`, or an array whose element is a single byte.
    ///
    /// Byte arrays are kept as bytes rather than as a vector of values, and that
    /// is a correctness measure as much as a performance one: a two-megabyte
    /// `m_Script` read element by element would be two million enum values, tens
    /// of megabytes of allocation for a field whose whole content is the bytes.
    Bayt(Vec<u8>),
    /// A `vector`, a `set`, a `staticvector` or any other array-shaped field.
    Masfufa(Vec<Self>),
    /// A struct: its fields, in serialization order.
    ///
    /// Ordered rather than hashed, because serialization order *is* the object's
    /// structure and a translator reading a report wants the fields in the order
    /// the developer wrote them.
    Sijil(Vec<(String, Self)>),
    /// A `map`, as its pairs in file order.
    Khareeta(Vec<(Self, Self)>),
}

impl QeemaHaql {
    /// The string, when this value is one.
    #[must_use]
    pub const fn nass(&self) -> Option<&str> {
        match self {
            Self::Nass(nass) => Some(nass.as_str()),
            _ => None,
        }
    }

    /// The signed value, when this value is an integer of either signedness.
    ///
    /// A `u64` above `i64::MAX` answers `None` rather than wrapping into a
    /// negative: every caller here uses this for an entry id or a count, and a
    /// silently negative id would key a localization entry under a number the
    /// game never wrote.
    #[must_use]
    pub fn sahih(&self) -> Option<i64> {
        match self {
            Self::Sahih(qeema) => Some(*qeema),
            Self::Ghayr(qeema) => i64::try_from(*qeema).ok(),
            _ => None,
        }
    }

    /// The single-precision value, when this value is a `float`.
    ///
    /// A `double` answers `None` rather than being narrowed. Nothing in this
    /// crate reads a measurement out of a `double` field, and narrowing one
    /// silently would produce a font size that is not the one the file holds.
    #[must_use]
    pub const fn ashri(&self) -> Option<f32> {
        match self {
            Self::Ashri(qeema) => Some(*qeema),
            _ => None,
        }
    }

    /// The boolean, when this value is one.
    #[must_use]
    pub const fn mantiqi(&self) -> Option<bool> {
        match self {
            Self::Mantiqi(qeema) => Some(*qeema),
            _ => None,
        }
    }

    /// The bytes, when this value is a byte run.
    #[must_use]
    pub const fn bayt(&self) -> Option<&[u8]> {
        match self {
            Self::Bayt(bayt) => Some(bayt.as_slice()),
            _ => None,
        }
    }

    /// The elements, when this value is an array.
    #[must_use]
    pub const fn anasir(&self) -> Option<&[Self]> {
        match self {
            Self::Masfufa(anasir) => Some(anasir.as_slice()),
            _ => None,
        }
    }

    /// One named field, when this value is a struct.
    #[must_use]
    pub fn haql(&self, ism: &str) -> Option<&Self> {
        match self {
            Self::Sijil(huqul) => {
                huqul.iter().find(|(mawjud, _)| mawjud == ism).map(|(_, qeema)| qeema)
            }
            _ => None,
        }
    }

    /// A field reached by a chain of names, for a value nested in a struct.
    #[must_use]
    pub fn masar_haql(&self, asma: &[&str]) -> Option<&Self> {
        let mut hali = self;
        for ism in asma {
            hali = hali.haql(ism)?;
        }
        Some(hali)
    }

    /// This value as an object reference, when it has that shape.
    ///
    /// Recognised by its fields rather than by its type name, because the type
    /// name is `PPtr<Transform>` in one file and `PPtr<$SomeGeneratedName>` in
    /// another, and both serialize the same two integers.
    #[must_use]
    pub fn ishara(&self) -> Option<Ishara> {
        let fahras = self.haql("m_FileID")?.sahih()?;
        let masar = self.haql("m_PathID")?.sahih()?;
        Some(Ishara {
            fahras_malaf: i32::try_from(fahras).ok()?,
            hawiya_masar: masar,
        })
    }
}

// ---------------------------------------------------------------------------
// The reader
// ---------------------------------------------------------------------------

/// Everything the value reader needs beyond the node it is standing on.
///
/// The tree alone is not enough for one shape. A `[SerializeReference]` field's
/// bytes are laid out by a type the `ManagedReferencesRegistry` names *in the
/// data*, and the node the tree gives them — `ReferencedObjectData` — is empty.
/// The layouts live in the file's reference-type list, so the list travels with
/// the tree rather than being fetched at the one place it is needed: the walk
/// recurses into a reference type's own tree, and that recursion has to be able
/// to reach the list again for a reference nested inside a reference.
#[derive(Debug, Clone, Copy)]
pub struct SiyaqQira<'s> {
    /// The tree whose nodes are being walked.
    pub shajara: &'s ShajaratAnwa,
    /// The file's `m_RefTypes`, which describe its managed references.
    pub anwa_marjiiya: &'s [NawMulsal],
    /// Whether `shajara` is a reference type's tree being walked as one managed
    /// reference's payload rather than as a whole object.
    ///
    /// Unity generates a reference type's tree as though the type were
    /// serialized on its own, so the tree ends with a
    /// `ManagedReferencesRegistry` of its own — and that registry is never
    /// written for a nested object, because every managed reference in one
    /// `UnityEngine.Object` is flattened into *that object's* single registry.
    /// Reading it would take the next entry's `rid` for a version number, which
    /// is exactly the desynchronisation this flag exists to avoid.
    pub jism_marji: bool,
}

impl<'s> SiyaqQira<'s> {
    /// The same file's reference types, against the tree of one managed
    /// reference's payload.
    #[must_use]
    pub const fn li_marji(self, shajara: &'s ShajaratAnwa) -> Self {
        Self { shajara, anwa_marjiiya: self.anwa_marjiiya, jism_marji: true }
    }
}

/// Reads one object's fields through its type tree.
///
/// The one entry point. Every refusal in this module is decided here, before a
/// byte of the object is read: no type, no tree, an empty tree or a tree with an
/// unresolvable name all stop before the cursor moves.
///
/// # Errors
///
/// [`KhataKaain::BilaShajaratAnwa`] when the build stripped the layout,
/// [`KhataKaain::IsmMajhul`] when it kept a layout this build cannot fully name,
/// [`KhataKaain::HaqlTalif`] when a field's bytes ran out,
/// [`KhataKaain::ShajaraShadha`] for a tree whose shape contradicts the format,
/// [`KhataKaain::TajawuzHadd`] for a value that nests or repeats past a
/// ceiling, and — for a `[SerializeReference]` field —
/// [`KhataKaain::NawMarjiMajhul`] when the registry names a managed type
/// `m_RefTypes` does not describe and
/// [`KhataKaain::SijillMarajiGhayrMadum`] for a registry version this build
/// does not read.
pub fn iqra_kaain(
    mulsal: &Mulsal<'_>,
    madkhal: &MadkhalKaain,
) -> Result<QeemaHaql, KhataKaain> {
    let naw = mulsal
        .naw_kaain(madkhal)
        .ok_or(KhataKaain::BilaShajaratAnwa { naw: None })?;
    let shajara = naw
        .shajara
        .as_ref()
        .ok_or(KhataKaain::BilaShajaratAnwa { naw: None })?;

    let ism_naw = shajara.uqad().first().and_then(|uqda| uqda.naw.clone());
    if shajara.khali() {
        return Err(KhataKaain::BilaShajaratAnwa { naw: ism_naw });
    }
    if let Some((fahras, izaha)) = shajara.majhula() {
        return Err(KhataKaain::IsmMajhul { naw: ism_naw, fahras, izaha });
    }

    let mut qari = mulsal.qari_kaain(madkhal)?;
    let siyaq =
        SiyaqQira { shajara, anwa_marjiiya: mulsal.anwa_marjiiya(), jism_marji: false };
    iqra_qeema(&siyaq, 0, &mut qari, 0)
}

/// Reads the value of one type tree node, leaving the cursor after it.
///
/// The caller advances its own index with
/// [`ShajaratAnwa::nihayat_farr`], which is why this does not return one: the
/// tree's levels already say where a subtree ends, and threading a mutable index
/// through the recursion — which is how the reference implementations do it — is
/// the part of those implementations that is hardest to check.
///
/// `fahras` indexes [`SiyaqQira::shajara`]; the rest of the context is carried
/// because one node type — `ManagedReferencesRegistry` — is laid out by a
/// reference type rather than by its own subtree. See [`SiyaqQira`].
///
/// # Errors
///
/// The same set as [`iqra_kaain`], less the two that are decided before the read
/// begins.
pub fn iqra_qeema(
    siyaq: &SiyaqQira<'_>,
    fahras: usize,
    qari: &mut Qari<'_>,
    umq: u8,
) -> Result<QeemaHaql, KhataKaain> {
    if umq > AQSA_UMQ_QIRA {
        return Err(KhataKaain::TajawuzHadd {
            hadd: "value nesting depth",
            qeema: u64::from(umq),
            saqf: u64::from(AQSA_UMQ_QIRA),
        });
    }

    let shajara = siyaq.shajara;
    let uqda = shajara.uqad().get(fahras).ok_or_else(|| KhataKaain::ShajaraShadha {
        naw: ism_jidhr(shajara),
        sabab: format!("node {fahras} is past the end of a {}-node tree", shajara.uqad().len()),
    })?;
    let naw = uqda.naw.clone().ok_or_else(|| KhataKaain::IsmMajhul {
        naw: Some(ism_jidhr(shajara)),
        fahras: u32::try_from(fahras).unwrap_or(u32::MAX),
        izaha: 0,
    })?;
    let ism = uqda.ism.clone().unwrap_or_default();
    let mut yuhadhi = uqda.yuhadhi();

    // Scalars first, and by an exhaustive name match rather than by declared
    // byte size. Two different types share a size — `float` and `SInt32` are both
    // four bytes — and reading one as the other produces a value that is wrong
    // and a cursor that is right, which is the failure that survives review.
    if let Some(natija) = qeema_basita(&naw, qari) {
        let qeema = natija.map_err(|sabab| KhataKaain::HaqlTalif { ism, sabab })?;
        if yuhadhi {
            qari.hadhi(4);
        }
        return Ok(qeema);
    }

    let qeema = match naw.as_str() {
        "map" => {
            let (mafateeh, qiyam) = huqul_khareeta(shajara, fahras)?;
            // An `Array` child carrying the align flag aligns the *map*, not
            // itself. Unity's own reader does this promotion and a reader that
            // does not is three bytes out for the rest of the object.
            if shajara
                .uqad()
                .get(fahras.saturating_add(1))
                .is_some_and(UqdatShajara::yuhadhi)
            {
                yuhadhi = true;
            }
            let adad = adad_mutakarrir(qari, "map size", &ism)?;
            let mut azwaj = Vec::with_capacity(hajz_mabdai(adad));
            for _ in 0..adad {
                let miftah = iqra_qeema(siyaq, mafateeh, qari, umq.saturating_add(1))?;
                let qeema = iqra_qeema(siyaq, qiyam, qari, umq.saturating_add(1))?;
                azwaj.push((miftah, qeema));
            }
            QeemaHaql::Khareeta(azwaj)
        }
        // The one node whose subtree does not describe its own bytes. See this
        // module's header: the payload's layout is in `m_RefTypes`, named by
        // three strings the entry writes immediately before it.
        NAW_SIJILL_MARAJI => iqra_sijill_maraji(siyaq, fahras, qari, umq)?,
        "TypelessData" => {
            // A size and then that many raw bytes, with no element nodes to
            // walk. The two child nodes exist in the tree and describe nothing
            // the reader consumes, which is why the subtree is skipped whole.
            let adad = adad_mutakarrir(qari, "typeless data size", &ism)?;
            let bayt = qari
                .iqra_bayt("typeless data", u64::from(adad))
                .map_err(|sabab| KhataKaain::HaqlTalif { ism: ism.clone(), sabab })?;
            QeemaHaql::Bayt(bayt.to_vec())
        }
        _ => {
            let ibn = fahras.saturating_add(1);
            let masfufa = shajara
                .uqad()
                .get(ibn)
                .is_some_and(|uqda| uqda.naw.as_deref() == Some("Array") || uqda.masfufa);
            if masfufa {
                if shajara
                    .uqad()
                    .get(ibn)
                    .is_some_and(UqdatShajara::yuhadhi)
                {
                    yuhadhi = true;
                }
                iqra_masfufa(siyaq, fahras, qari, umq, &ism)?
            } else {
                iqra_sijil(siyaq, fahras, qari, umq)?
            }
        }
    };

    if yuhadhi {
        qari.hadhi(4);
    }
    Ok(qeema)
}

/// Every scalar type name Unity writes, and nothing else.
///
/// `None` means "this is a composite", which is the only way the caller
/// distinguishes the two: a type name that is not in this list is a struct, an
/// array or a map, and there is no flag in the node that says which.
fn qeema_basita(naw: &str, qari: &mut Qari<'_>) -> Option<Result<QeemaHaql, KhataQira>> {
    let natija = match naw {
        "SInt8" => qari.iqra_i8("SInt8").map(|q| QeemaHaql::Sahih(i64::from(q))),
        "UInt8" | "char" => qari.iqra_u8("UInt8").map(|q| QeemaHaql::Ghayr(u64::from(q))),
        "short" | "SInt16" => qari.iqra_i16("SInt16").map(|q| QeemaHaql::Sahih(i64::from(q))),
        "ushort" | "UInt16" | "unsigned short" => {
            qari.iqra_u16("UInt16").map(|q| QeemaHaql::Ghayr(u64::from(q)))
        }
        "int" | "SInt32" => qari.iqra_i32("SInt32").map(|q| QeemaHaql::Sahih(i64::from(q))),
        // `Type*` is Unity's own name for a runtime type handle serialized as
        // four bytes. It is a scalar despite the name looking like a pointer,
        // and treating it as a struct would look for children it does not have.
        "uint" | "UInt32" | "unsigned int" | "Type*" => {
            qari.iqra_u32("UInt32").map(|q| QeemaHaql::Ghayr(u64::from(q)))
        }
        "long long" | "SInt64" => qari.iqra_i64("SInt64").map(QeemaHaql::Sahih),
        "unsigned long long" | "UInt64" | "FileSize" => {
            qari.iqra_u64("UInt64").map(QeemaHaql::Ghayr)
        }
        "float" => qari.iqra_f32("float").map(QeemaHaql::Ashri),
        "double" => qari.iqra_f64("double").map(QeemaHaql::AshriMuda),
        "bool" => qari.iqra_bool("bool").map(QeemaHaql::Mantiqi),
        // The string's own four-byte padding is consumed by the read. The node
        // usually carries the align flag too, and aligning an already-aligned
        // cursor moves nothing, so the double application is harmless and the
        // single application is not optional.
        "string" => qari.iqra_nass_muhadhah("string").map(QeemaHaql::Nass),
        _ => return None,
    };
    Some(natija)
}

/// Reads an array-shaped field: `vector`, `set`, `staticvector`, `Array`.
fn iqra_masfufa(
    siyaq: &SiyaqQira<'_>,
    fahras: usize,
    qari: &mut Qari<'_>,
    umq: u8,
    ism: &str,
) -> Result<QeemaHaql, KhataKaain> {
    let shajara = siyaq.shajara;
    // vector → Array → { size, data }. The element node is the fourth, and its
    // position is fixed by the format rather than searched for, because a search
    // that found the wrong node would read the wrong width per element.
    let bayanat = fahras.saturating_add(3);
    let uqdat_bayanat = shajara.uqad().get(bayanat).ok_or_else(|| KhataKaain::ShajaraShadha {
        naw: ism_jidhr(shajara),
        sabab: format!("array field `{ism}` has no element node"),
    })?;

    let adad = adad_mutakarrir(qari, "array size", ism)?;

    // A byte array is read as bytes. Element-by-element it would be correct and
    // ruinous: a two-megabyte `m_Script` becomes two million enum values. The
    // shortcut is byte-identical only when the element carries no align flag,
    // which Unity never sets on a byte element — so the flag is checked rather
    // than assumed.
    let bayti = matches!(uqdat_bayanat.naw.as_deref(), Some("UInt8" | "SInt8" | "char"));
    if bayti && !uqdat_bayanat.yuhadhi() {
        let bayt = qari
            .iqra_bayt("byte array", u64::from(adad))
            .map_err(|sabab| KhataKaain::HaqlTalif { ism: ism.to_owned(), sabab })?;
        return Ok(QeemaHaql::Bayt(bayt.to_vec()));
    }

    let mut anasir = Vec::with_capacity(hajz_mabdai(adad));
    for _ in 0..adad {
        anasir.push(iqra_qeema(siyaq, bayanat, qari, umq.saturating_add(1))?);
    }
    Ok(QeemaHaql::Masfufa(anasir))
}

/// Reads a struct: every direct child of `fahras`, in order.
fn iqra_sijil(
    siyaq: &SiyaqQira<'_>,
    fahras: usize,
    qari: &mut Qari<'_>,
    umq: u8,
) -> Result<QeemaHaql, KhataKaain> {
    let shajara = siyaq.shajara;
    let nihaya = shajara.nihayat_farr(fahras);

    // A node with no children, a type name this reader does not know, and a
    // declared width above zero is the one shape that would desynchronise
    // silently: the struct branch reads nothing, the cursor stays where it was,
    // and every field after it is read from the wrong place while still
    // producing values. Unity has no such node — a leaf is always one of the
    // scalar type names — so seeing one means the tree is not what it claims,
    // and the object is refused instead of read four bytes out.
    if nihaya <= fahras.saturating_add(1)
        && let Some(uqda) = shajara.uqad().get(fahras)
            && uqda.hajm > 0 {
                return Err(KhataKaain::ShajaraShadha {
                    naw: ism_jidhr(shajara),
                    sabab: format!(
                        "field `{}` has type `{}`, which this build does not know how to \
                         read, no children to read instead, and a declared width of {} byte(s)",
                        uqda.ism.as_deref().unwrap_or("(unnamed)"),
                        uqda.naw_aw_faragh(),
                        uqda.hajm
                    ),
                });
            }

    let mut huqul: Vec<(String, QeemaHaql)> = Vec::new();
    let mut i = fahras.saturating_add(1);
    while i < nihaya {
        // A registry inside a managed reference's own tree describes bytes that
        // are not there. See [`SiyaqQira::jism_marji`].
        if siyaq.jism_marji
            && shajara.uqad().get(i).and_then(|uqda| uqda.naw.as_deref())
                == Some(NAW_SIJILL_MARAJI)
        {
            let baad = shajara.nihayat_farr(i);
            i = if baad > i { baad } else { i.saturating_add(1) };
            continue;
        }
        let ism = shajara
            .uqad()
            .get(i)
            .and_then(|uqda| uqda.ism.clone())
            .ok_or_else(|| KhataKaain::IsmMajhul {
                naw: Some(ism_jidhr(shajara)),
                fahras: u32::try_from(i).unwrap_or(u32::MAX),
                izaha: 0,
            })?;
        let qeema = iqra_qeema(siyaq, i, qari, umq.saturating_add(1))?;
        huqul.push((ism, qeema));
        let baad = shajara.nihayat_farr(i);
        // A subtree that does not advance would loop forever. It cannot happen
        // for a well-formed tree — `nihayat_farr` always returns at least
        // `i + 1` — and the guard is here because "cannot happen" is what every
        // infinite loop in a binary reader was before it shipped.
        i = if baad > i { baad } else { i.saturating_add(1) };
    }
    Ok(QeemaHaql::Sijil(huqul))
}

// ---------------------------------------------------------------------------
// Managed references
// ---------------------------------------------------------------------------

/// Unity's type name for the `[SerializeReference]` registry.
const NAW_SIJILL_MARAJI: &str = "ManagedReferencesRegistry";

/// The node inside one registry entry whose layout the entry names at runtime.
///
/// Always empty in the tree — no children, width zero — which is precisely why
/// it has to be recognised by name and walked against `m_RefTypes` instead.
const NAW_BAYANAT_MARJI: &str = "ReferencedObjectData";

/// Reads a `ManagedReferencesRegistry`: every `[SerializeReference]` object the
/// component holds, each against the type the registry declares for it.
///
/// The shape is `int version` then a `RefIds` vector of `{ rid, type, data }`,
/// where `type` is the managed class, namespace and assembly written as three
/// aligned strings and `data` is that object's own serialization.
///
/// The registry's own children are found by *name*, because the registry is the
/// one field whose subtree Unity may extend — a build that added a field to
/// `ReferencedObject` would shift every offset and a name-driven walk would
/// still find the nodes it needs. Inside `RefIds` the walk is positional again,
/// because `vector → Array → { size, data }` is the fixed shape of every array
/// in the format and is checked here exactly as [`iqra_masfufa`] checks it.
fn iqra_sijill_maraji(
    siyaq: &SiyaqQira<'_>,
    fahras: usize,
    qari: &mut Qari<'_>,
    umq: u8,
) -> Result<QeemaHaql, KhataKaain> {
    let shajara = siyaq.shajara;
    let shadha = |sabab: &str| KhataKaain::ShajaraShadha {
        naw: ism_jidhr(shajara),
        sabab: sabab.to_owned(),
    };

    let uqdat_isdar = shajara
        .ibn(fahras, "version")
        .ok_or_else(|| shadha("a managed-reference registry with no `version`"))?;
    let uqdat_qaima = shajara
        .ibn(fahras, "RefIds")
        .ok_or_else(|| shadha("a managed-reference registry with no `RefIds`"))?;

    let isdar = iqra_qeema(siyaq, uqdat_isdar, qari, umq.saturating_add(1))?
        .sahih()
        .ok_or_else(|| shadha("a managed-reference registry whose `version` is not an integer"))?;
    if !(ISDAR_SIJILL_MARAJI_ADNA..=ISDAR_SIJILL_MARAJI_AQSA).contains(&isdar) {
        return Err(KhataKaain::SijillMarajiGhayrMadum { isdar });
    }

    // `RefIds` is an ordinary vector — `vector → Array → { size, data }` — and
    // only its element is special, so the element node sits where it does for
    // every other array and is checked rather than assumed.
    let masfufa = uqdat_qaima.saturating_add(1);
    if shajara.uqad().get(masfufa).and_then(|uqda| uqda.naw.as_deref()) != Some("Array") {
        return Err(shadha("a `RefIds` whose second node is not `Array`"));
    }
    let unsur = uqdat_qaima.saturating_add(3);
    if shajara.uqad().get(unsur).is_none() {
        return Err(shadha("a `RefIds` with no element node"));
    }

    let adad = adad_mutakarrir(qari, "managed reference count", "RefIds")?;
    let mut maraji = Vec::with_capacity(hajz_mabdai(adad));
    for _ in 0..adad {
        maraji.push(iqra_marji(siyaq, unsur, qari, umq.saturating_add(1))?);
    }

    // The vector's own alignment, with the same `Array`-child promotion every
    // other array-shaped field gets. Applied here because the vector is read
    // inline rather than through `iqra_masfufa`, and skipping it would leave the
    // cursor three bytes early for whatever follows the registry.
    let yuhadhi = shajara.uqad().get(uqdat_qaima).is_some_and(UqdatShajara::yuhadhi)
        || shajara.uqad().get(masfufa).is_some_and(UqdatShajara::yuhadhi);
    if yuhadhi {
        qari.hadhi(4);
    }

    Ok(QeemaHaql::Sijil(vec![
        ("version".to_owned(), QeemaHaql::Sahih(isdar)),
        ("RefIds".to_owned(), QeemaHaql::Masfufa(maraji)),
    ]))
}

/// Reads one registry entry: its `rid`, the managed type it names, and the
/// object itself walked against that type's layout.
fn iqra_marji(
    siyaq: &SiyaqQira<'_>,
    fahras: usize,
    qari: &mut Qari<'_>,
    umq: u8,
) -> Result<QeemaHaql, KhataKaain> {
    let shajara = siyaq.shajara;
    let shadha = |sabab: &str| KhataKaain::ShajaraShadha {
        naw: ism_jidhr(shajara),
        sabab: sabab.to_owned(),
    };

    let uqdat_rid = shajara
        .ibn(fahras, "rid")
        .ok_or_else(|| shadha("a managed reference with no `rid`"))?;
    let uqdat_naw = shajara
        .ibn(fahras, "type")
        .ok_or_else(|| shadha("a managed reference with no `type`"))?;
    let uqdat_bayanat = shajara
        .ibn(fahras, "data")
        .ok_or_else(|| shadha("a managed reference with no `data`"))?;
    if shajara.uqad().get(uqdat_bayanat).and_then(|uqda| uqda.naw.as_deref())
        != Some(NAW_BAYANAT_MARJI)
    {
        return Err(shadha("a managed reference whose `data` is not `ReferencedObjectData`"));
    }

    let rid = iqra_qeema(siyaq, uqdat_rid, qari, umq.saturating_add(1))?;
    let naw = iqra_qeema(siyaq, uqdat_naw, qari, umq.saturating_add(1))?;
    let sanf = naw.haql("class").and_then(QeemaHaql::nass).unwrap_or_default().to_owned();
    let nitaq = naw.haql("ns").and_then(QeemaHaql::nass).unwrap_or_default().to_owned();
    let tajmee = naw.haql("asm").and_then(QeemaHaql::nass).unwrap_or_default().to_owned();

    // Three empty names is Unity's own way of writing a reference to nothing:
    // the entry exists so the `rid` resolves, and no payload follows it.
    let bayanat = if sanf.is_empty() && nitaq.is_empty() && tajmee.is_empty() {
        QeemaHaql::Sijil(Vec::new())
    } else {
        let marjii = shajara_marji(siyaq.anwa_marjiiya, &sanf, &nitaq, &tajmee).ok_or_else(
            || KhataKaain::NawMarjiMajhul { ism: ism_naw_marji(&nitaq, &sanf) },
        )?;
        iqra_qeema(&siyaq.li_marji(marjii), 0, qari, umq.saturating_add(1))?
    };
    if shajara.uqad().get(uqdat_bayanat).is_some_and(UqdatShajara::yuhadhi) {
        qari.hadhi(4);
    }
    // The element node's own flag, which `iqra_masfufa` would have applied for
    // an ordinary array element and which nothing else applies here.
    if shajara.uqad().get(fahras).is_some_and(UqdatShajara::yuhadhi) {
        qari.hadhi(4);
    }

    Ok(QeemaHaql::Sijil(vec![
        ("rid".to_owned(), rid),
        ("type".to_owned(), naw),
        ("data".to_owned(), bayanat),
    ]))
}

/// The reference type whose three names a registry entry just wrote down.
///
/// Matched on all three, class and namespace and assembly, because a managed
/// type is only identified by all three: two packages ship a `Comment` and a
/// game that has both would otherwise read one against the other's layout.
fn shajara_marji<'s>(
    anwa: &'s [NawMulsal],
    sanf: &str,
    nitaq: &str,
    tajmee: &str,
) -> Option<&'s ShajaratAnwa> {
    anwa.iter()
        .find(|naw| {
            naw.ism_sanf.as_deref() == Some(sanf)
                && naw.nitaq_asma.as_deref() == Some(nitaq)
                && naw.ism_tajmee.as_deref() == Some(tajmee)
        })
        .and_then(|naw| naw.shajara.as_ref())
        .filter(|shajara| !shajara.khali() && shajara.majhula().is_none())
}

/// A managed type's name for a refusal message.
fn ism_naw_marji(nitaq: &str, sanf: &str) -> String {
    if nitaq.is_empty() { sanf.to_owned() } else { format!("{nitaq}.{sanf}") }
}

/// The key and value node indices of a `map`.
///
/// `map → Array → { size, pair → { first, second } }`. Each hop is checked
/// rather than assumed, because a tree that does not have this shape is one the
/// reader would otherwise walk with the wrong element widths.
fn huqul_khareeta(
    shajara: &ShajaratAnwa,
    fahras: usize,
) -> Result<(usize, usize), KhataKaain> {
    let shadha = |sabab: String| KhataKaain::ShajaraShadha { naw: ism_jidhr(shajara), sabab };

    let masfufa = fahras.saturating_add(1);
    if shajara.uqad().get(masfufa).and_then(|uqda| uqda.naw.as_deref()) != Some("Array") {
        return Err(shadha("a `map` whose second node is not `Array`".to_owned()));
    }
    let zawj = fahras.saturating_add(3);
    let awwal = fahras.saturating_add(4);
    if shajara.uqad().get(zawj).is_none() || shajara.uqad().get(awwal).is_none() {
        return Err(shadha("a `map` with no key and value nodes".to_owned()));
    }
    let thani = shajara.nihayat_farr(awwal);
    if shajara.uqad().get(thani).is_none() {
        return Err(shadha("a `map` with a key node and no value node".to_owned()));
    }
    Ok((awwal, thani))
}

/// Reads a repeat count and refuses one the object cannot hold.
///
/// The bytes remaining are the real bound, and they are a tight one: every
/// element of every array occupies at least one byte, so a count above the
/// remaining length is a count that cannot be honoured whatever the element type
/// is. Checking it here means a corrupt count is refused before a `Vec` is
/// sized, rather than after the allocator has been asked for four gigabytes.
fn adad_mutakarrir(
    qari: &mut Qari<'_>,
    haql: &'static str,
    ism: &str,
) -> Result<u32, KhataKaain> {
    let khaam = qari
        .iqra_i32(haql)
        .map_err(|sabab| KhataKaain::HaqlTalif { ism: ism.to_owned(), sabab })?;
    let adad = u32::try_from(khaam).map_err(|_| KhataKaain::TajawuzHadd {
        hadd: haql,
        qeema: u64::MAX,
        saqf: u64::from(u32::MAX),
    })?;
    let baqi = qari.baqi();
    if u64::from(adad) > baqi {
        return Err(KhataKaain::TajawuzHadd { hadd: haql, qeema: u64::from(adad), saqf: baqi });
    }
    Ok(adad)
}

/// How much to reserve for a declared element count.
///
/// Clamped, because the count came out of the file. An array that really does
/// hold more grows as it is filled, which costs a handful of reallocations on a
/// container no game ships and avoids reserving a gigabyte on a number a corrupt
/// file supplied.
fn hajz_mabdai(adad: u32) -> usize {
    usize::try_from(adad).unwrap_or(AQSA_HAJZ_ANASIR).min(AQSA_HAJZ_ANASIR)
}

/// The tree's root type name, for a message.
fn ism_jidhr(shajara: &ShajaratAnwa) -> String {
    shajara
        .uqad()
        .first()
        .and_then(|uqda| uqda.naw.clone())
        .unwrap_or_else(|| "an unnamed type".to_owned())
}

// ---------------------------------------------------------------------------
// The object graph
// ---------------------------------------------------------------------------

/// The scene hierarchy, read once, so a string can say where it lives.
///
/// A `MonoBehaviour` on its own knows only its own path id, which is a number
/// that changes whenever the developer rebuilds. What does not change is
/// `Canvas/HUD/QuestPanel/TitleText` — the names a developer typed and a
/// contributor recognises — and reconstructing it takes three links this index
/// holds: the component's `m_GameObject`, that `GameObject`'s `Transform`, and
/// that `Transform`'s `m_Father`.
///
/// Building it costs one pass over every `GameObject`, `Transform`,
/// `RectTransform` and `MonoScript` in the file. That is the price of structural
/// identity, and the alternative — keying strings on the path id — is the array
/// index this product's identity model rejects in
/// [`crate::jadwal`], with the same consequence: a rebuild renumbers everything
/// and the patch is silently applied to the wrong objects.
#[derive(Debug, Default)]
pub struct FahrasKaainat {
    asma: FxHashMap<i64, String>,
    tahwil_ila_kaain: FxHashMap<i64, i64>,
    kaain_ila_tahwil: FxHashMap<i64, i64>,
    abu: FxHashMap<i64, i64>,
    asma_scripts: FxHashMap<i64, String>,
    manqus: bool,
    akhta: usize,
}

impl FahrasKaainat {
    /// Reads the hierarchy out of one file.
    ///
    /// Never fails. An object that cannot be read contributes nothing to the
    /// index and is counted in [`FahrasKaainat::akhta`]: a broken `Transform`
    /// costs one path segment, and refusing the whole index over it would cost
    /// every string in the file its structural position.
    #[must_use]
    pub fn ibni(mulsal: &Mulsal<'_>) -> Self {
        let mut fahras = Self::default();
        if !mulsal.ladayhi_shajarat_anwa() {
            // Without a tree there is nothing to read the hierarchy from either.
            // Marked truncated so callers report the weaker identity rather than
            // presenting a bare component name as a path.
            fahras.manqus = true;
            return fahras;
        }

        let mut adad = 0_usize;
        for madkhal in mulsal.kaainat() {
            if !matches!(
                madkhal.sanf,
                SANF_GAMEOBJECT | SANF_TRANSFORM | SANF_RECTTRANSFORM | SANF_MONOSCRIPT
            ) {
                continue;
            }
            if adad >= AQSA_KAAINAT_FAHRAS {
                fahras.manqus = true;
                break;
            }
            adad = adad.saturating_add(1);

            let Ok(qeema) = iqra_kaain(mulsal, madkhal) else {
                fahras.akhta = fahras.akhta.saturating_add(1);
                continue;
            };
            fahras.adif(madkhal.sanf, madkhal.hawiya_masar, &qeema);
        }
        fahras
    }

    /// Files one read object into the index.
    fn adif(&mut self, sanf: i32, hawiya: i64, qeema: &QeemaHaql) {
        match sanf {
            SANF_GAMEOBJECT => {
                if let Some(ism) = qeema.haql("m_Name").and_then(QeemaHaql::nass) {
                    let _ = self.asma.insert(hawiya, ism.to_owned());
                }
            }
            SANF_TRANSFORM | SANF_RECTTRANSFORM => {
                if let Some(kaain) = qeema.haql("m_GameObject").and_then(QeemaHaql::ishara)
                    && kaain.mahalli() && !kaain.khali() {
                        let _ = self.tahwil_ila_kaain.insert(hawiya, kaain.hawiya_masar);
                        let _ = self.kaain_ila_tahwil.insert(kaain.hawiya_masar, hawiya);
                    }
                if let Some(walid) = qeema.haql("m_Father").and_then(QeemaHaql::ishara)
                    && walid.mahalli() && !walid.khali() {
                        let _ = self.abu.insert(hawiya, walid.hawiya_masar);
                    }
            }
            SANF_MONOSCRIPT => {
                let ism = qeema.haql("m_ClassName").and_then(QeemaHaql::nass).unwrap_or("");
                if !ism.is_empty() {
                    // The namespace is joined in because two packages ship a
                    // `LanguageSource` and only the namespace tells them apart,
                    // and the classifier reads this name.
                    let nitaq =
                        qeema.haql("m_Namespace").and_then(QeemaHaql::nass).unwrap_or("");
                    let kamil = if nitaq.is_empty() {
                        ism.to_owned()
                    } else {
                        format!("{nitaq}.{ism}")
                    };
                    let _ = self.asma_scripts.insert(hawiya, kamil);
                }
            }
            _ => {}
        }
    }

    /// Whether the index is incomplete, so paths may be missing.
    #[must_use]
    pub const fn manqus(&self) -> bool {
        self.manqus
    }

    /// How many objects failed to read while the index was built.
    #[must_use]
    pub const fn akhta(&self) -> usize {
        self.akhta
    }

    /// One `GameObject`'s name.
    #[must_use]
    pub fn ism_kaain(&self, hawiya: i64) -> Option<&str> {
        self.asma.get(&hawiya).map(String::as_str)
    }

    /// One `MonoScript`'s class name, namespace included.
    ///
    /// This is what makes a component identifiable: a `MonoBehaviour` is
    /// otherwise an anonymous bag of fields, and `TextMeshProUGUI.m_text` versus
    /// `SaveSlotDebug.m_text` is the difference between a line a player reads and
    /// one they never see.
    #[must_use]
    pub fn ism_script(&self, hawiya: i64) -> Option<&str> {
        self.asma_scripts.get(&hawiya).map(String::as_str)
    }

    /// The slash-joined path from the scene root down to one `GameObject`.
    ///
    /// `None` when the object is not in the index — which happens for a
    /// component whose `GameObject` lives in another file, and for every object
    /// when the file has no type tree.
    ///
    /// The walk carries a visited set and a depth ceiling. A `m_Father` cycle is
    /// not something Unity writes and is exactly what a corrupt file contains,
    /// and an unbounded parent walk on one never returns.
    #[must_use]
    pub fn masar_kaain(&self, kaain: i64) -> Option<String> {
        let mut ajza: Vec<&str> = Vec::new();
        let mut zurat: FxHashSet<i64> = FxHashSet::default();
        let mut hali = kaain;

        for _ in 0..AQSA_UMQ_MASAR {
            if !zurat.insert(hali) {
                break;
            }
            let Some(ism) = self.asma.get(&hali) else {
                break;
            };
            ajza.push(ism.as_str());
            let Some(tahwil) = self.kaain_ila_tahwil.get(&hali) else {
                break;
            };
            let Some(walid) = self.abu.get(tahwil) else {
                break;
            };
            let Some(kaain_walid) = self.tahwil_ila_kaain.get(walid) else {
                break;
            };
            hali = *kaain_walid;
        }

        if ajza.is_empty() {
            return None;
        }
        ajza.reverse();
        Some(ajza.join("/"))
    }
}

// ---------------------------------------------------------------------------
// Harvesting
// ---------------------------------------------------------------------------

/// The most strings one object contributes.
///
/// Eight thousand. A component holding more than that is a data table rather
/// than a component, and the ceiling stops one corrupt array from filling the
/// project with fragments before anything else in the file is read.
pub const AQSA_NUSUS_LIKAAIN: usize = 8192;

/// How many neighbouring lines are kept beside a string from a string array.
///
/// Two either side. Enough for machine translation to see who is answering
/// whom, and few enough that the table does not carry five copies of every line.
pub const AQSA_JIWAR: usize = 2;

/// How many distinct non-grouping refusals one file reports individually.
///
/// Sixty-four. Past that the report is a wall nobody reads, and the refusals
/// past it are the same handful of reasons repeated — which
/// [`TaqreerRafd::majmua`][crate::rafd::TaqreerRafd::majmua] would collapse
/// anyway.
pub const AQSA_RAFD_MUFASSAL: usize = 64;

/// One string found inside an object, with where it sat and what surrounded it.
#[derive(Debug, Clone)]
struct NassMahsud {
    masar: String,
    nass: String,
    jiwar: Vec<String>,
}

/// Walks a read object and collects every string in it, with its field path.
///
/// The path is the field chain — `m_Entries{quest_01}.m_Text` — and it is what
/// becomes [`MawqiNass::haql`]. Array indices appear in it and map keys appear
/// in it in preference to indices, because a key is the handle the developer
/// chose and an index is the one the serializer happened to assign.
///
/// Empty and whitespace-only strings are not collected. That is not dropping a
/// string: an unset field has no source text to translate and no target to write
/// back, and a project whose first four hundred rows are blank is one a
/// translator abandons.
fn ijma_nusus(qeema: &QeemaHaql, masar: &mut String, khazina: &mut Vec<NassMahsud>, umq: u8) {
    if umq > AQSA_UMQ_QIRA || khazina.len() >= AQSA_NUSUS_LIKAAIN {
        return;
    }
    match qeema {
        QeemaHaql::Nass(nass) => {
            if !nass.trim().is_empty() && nass.len() <= AQSA_TUL_NASS {
                khazina.push(NassMahsud {
                    masar: masar.clone(),
                    nass: nass.clone(),
                    jiwar: Vec::new(),
                });
            }
        }
        QeemaHaql::Sijil(huqul) => {
            for (ism, qeema) in huqul {
                let tul = masar.len();
                if !masar.is_empty() {
                    masar.push('.');
                }
                masar.push_str(ism);
                ijma_nusus(qeema, masar, khazina, umq.saturating_add(1));
                masar.truncate(tul);
            }
        }
        QeemaHaql::Masfufa(anasir) => {
            for (fahras, unsur) in anasir.iter().enumerate() {
                let tul = masar.len();
                masar.push('[');
                masar.push_str(&fahras.to_string());
                masar.push(']');
                let qabl = khazina.len();
                ijma_nusus(unsur, masar, khazina, umq.saturating_add(1));
                // A run of strings in one array is a script: the lines around a
                // line are its context, and that context is what makes machine
                // translation of dialogue worth anything. Attached here because
                // this is the only place that knows the elements are siblings.
                if unsur.nass().is_some()
                    && let Some(mahsud) = khazina.get_mut(qabl) {
                        mahsud.jiwar = jiwar_masfufa(anasir, fahras);
                    }
                masar.truncate(tul);
            }
        }
        QeemaHaql::Khareeta(azwaj) => {
            for (fahras, (miftah, qeema)) in azwaj.iter().enumerate() {
                let tul = masar.len();
                masar.push('{');
                match miftah.nass() {
                    Some(nass) => masar.push_str(nass),
                    None => match miftah.sahih() {
                        Some(raqm) => masar.push_str(&raqm.to_string()),
                        None => masar.push_str(&fahras.to_string()),
                    },
                }
                masar.push('}');
                // The key is harvested too, under a `.key` suffix. The suffix is
                // not decoration: `crate::tasnif` matches `key` as a whole path
                // segment and answers internal for it, so a map key classifies
                // correctly through the shared rule instead of through a second
                // rule written here. And it *is* harvested — a `map<string,int>`
                // of English words has its text on the key side, and refusing to
                // look would lose exactly those.
                let tul_miftah = masar.len();
                masar.push_str(".key");
                ijma_nusus(miftah, masar, khazina, umq.saturating_add(1));
                masar.truncate(tul_miftah);
                ijma_nusus(qeema, masar, khazina, umq.saturating_add(1));
                masar.truncate(tul);
            }
        }
        QeemaHaql::Sahih(_)
        | QeemaHaql::Ghayr(_)
        | QeemaHaql::Ashri(_)
        | QeemaHaql::AshriMuda(_)
        | QeemaHaql::Mantiqi(_)
        | QeemaHaql::Bayt(_) => {}
    }
}

/// The lines around one element of a string array.
fn jiwar_masfufa(anasir: &[QeemaHaql], fahras: usize) -> Vec<String> {
    let awwal = fahras.saturating_sub(AQSA_JIWAR);
    let akhir = fahras.saturating_add(AQSA_JIWAR).saturating_add(1).min(anasir.len());
    let mut jiwar = Vec::new();
    let mut i = awwal;
    while i < akhir {
        if i != fahras
            && let Some(nass) = anasir.get(i).and_then(QeemaHaql::nass)
                && !nass.trim().is_empty() && nass.len() <= AQSA_TUL_NASS {
                    jiwar.push(nass.to_owned());
                }
        i = i.saturating_add(1);
    }
    jiwar
}

/// The speaker a dialogue line in this object belongs to, when the object names
/// one.
///
/// Read from the object rather than inferred from the text. A component with a
/// `m_SpeakerName` beside its `m_Text` is telling the reader who says the line,
/// and that is worth carrying: machine translation of Arabic dialogue needs the
/// speaker's gender, and the speaker's name is the only evidence of it available
/// without running the game.
fn mutakallim_min_kaain(qeema: &QeemaHaql) -> Option<String> {
    const HUQUL: &[&str] = &[
        "m_SpeakerName",
        "speakerName",
        "SpeakerName",
        "m_CharacterName",
        "characterName",
        "CharacterName",
        "actorName",
        "ActorName",
        "m_Speaker",
        "speaker",
    ];
    for ism in HUQUL {
        if let Some(nass) = qeema.haql(ism).and_then(QeemaHaql::nass)
            && !nass.trim().is_empty() {
                return Some(nass.to_owned());
            }
    }
    None
}

/// What the file itself says about how much room a string has.
///
/// Only what is actually written down. A character limit on an input field is a
/// hard bound the engine enforces and it is read; a pixel width is not in the
/// file at all and is left for runtime capture, which measures it. Inventing a
/// width from a `RectTransform`'s `m_SizeDelta` would produce a number that is
/// wrong whenever the layout is driven by a parent — which, for a UI built with
/// Unity's layout groups, is always.
fn quyud_min_kaain(qeema: &QeemaHaql) -> QuyudNass {
    let mut quyud = QuyudNass::default();

    if let Some(hadd) = qeema.haql("m_CharacterLimit").and_then(QeemaHaql::sahih)
        && hadd > 0 {
            quyud.aqsa_ahruf = u32::try_from(hadd).ok();
        }
    for ism in ["m_fontSize", "m_FontSize", "m_fontSizeBase"] {
        if let Some(hajm) = qeema.haql(ism).and_then(QeemaHaql::ashri)
            && hajm > 0.0 {
                quyud.hajm_khatt = Some(hajm);
                break;
            }
    }
    // TextMeshPro spells it `m_enableWordWrapping`; a `false` there is the
    // engine refusing to wrap, which is exactly what `satr_wahid` means.
    for ism in ["m_enableWordWrapping", "m_EnableWordWrapping", "m_TextWrappingMode"] {
        if let Some(yalif) = qeema.haql(ism).and_then(QeemaHaql::mantiqi) {
            quyud.satr_wahid = !yalif;
            break;
        }
    }
    if let Some(namat) = qeema.haql("m_LineType").and_then(QeemaHaql::sahih) {
        // Unity's `InputField.LineType`: 0 is SingleLine.
        if namat == 0 {
            quyud.satr_wahid = true;
        }
    }
    quyud
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// What one `SerializedFile` produced.
///
/// The table and the report, together and never one without the other. A file
/// that refused every object still returns this, and the refusals in it are the
/// useful output — see [`crate::rafd`].
#[derive(Debug, Default)]
pub struct HasilatMulsal {
    /// Every string that was read.
    pub madakhil: Vec<MudkhalMustakhraj>,
    /// Every reason something was not.
    pub marfudat: Vec<SababRafd>,
    /// Whether the object index was truncated, so some strings carry a weaker
    /// structural position than they would have.
    pub manqus: bool,
}

/// The bench one file's extraction works on.
///
/// Carries the container names every entry needs and the two output lists, so
/// the readers below take one parameter instead of five and cannot be called
/// with the container and the asset the wrong way round.
#[derive(Debug)]
struct Warsha<'w> {
    hawiya: &'w str,
    asl: Option<&'w str>,
    madakhil: Vec<MudkhalMustakhraj>,
    linat: Vec<SababRafd>,
}

impl Warsha<'_> {
    /// Records a string.
    fn adif(&mut self, mudkhal: MudkhalMustakhraj) {
        self.madakhil.push(mudkhal);
    }

    /// Records a refusal that cost one field rather than one object.
    ///
    /// Deduplicated and capped: a file with four thousand over-long `TextAsset`s
    /// says so once.
    fn lin(&mut self, sabab: SababRafd) {
        if self.linat.len() < AQSA_RAFD_MUFASSAL && !self.linat.contains(&sabab) {
            self.linat.push(sabab);
        }
    }

    /// Where a string sits, with the container names filled in.
    fn mawqi(
        &self,
        mawqi: String,
        haql: Option<String>,
        miftah: Option<String>,
    ) -> MawqiNass {
        MawqiNass {
            hawiya: self.hawiya.to_owned(),
            asl: self.asl.map(str::to_owned),
            mawqi,
            haql,
            miftah_muharrik: miftah,
        }
    }
}

/// Reads every string one `SerializedFile` holds.
///
/// The routing is the whole function: a file with no type tree produces one
/// refusal and no strings, and a file with one produces strings from the two
/// classes that carry them. `GameObject` and `Transform` are read — the index
/// needs them — and contribute no entries of their own, because a
/// `GameObject`'s name is already in the table as the `mawqi` of every string
/// on it, and emitting it separately would add one internal row per object in
/// the scene for text that is not text.
#[must_use]
pub fn istakhrij_mulsal(
    mulsal: &Mulsal<'_>,
    hawiya: &str,
    asl: Option<&str>,
) -> HasilatMulsal {
    if !mulsal.ladayhi_shajarat_anwa() {
        // Only the classes that would have carried text are counted. Reporting
        // every object in the file would say "180 000 objects could not be read"
        // for a file whose meshes and textures were never going to contribute a
        // string, and a report that overstates the loss is one users learn to
        // ignore.
        let adad = mulsal
            .kaainat()
            .iter()
            .filter(|madkhal| {
                matches!(madkhal.sanf, SANF_TEXTASSET | SANF_MONOBEHAVIOUR)
            })
            .count();
        // A stripped tree over a file with no text-bearing object lost nothing:
        // there was no string in it to read with or without a layout. Calling
        // that a loss made every `globalgamemanagers` and `level0` count as
        // unread text, three lossy refusals on a game that lost zero strings,
        // and — because a stripped tree is the refusal capture remedies — a
        // report telling the user to run capture for files that hold nothing.
        if adad == 0 {
            return HasilatMulsal {
                madakhil: Vec::new(),
                marfudat: vec![SababRafd::BilaNusus],
                manqus: false,
            };
        }
        return HasilatMulsal {
            madakhil: Vec::new(),
            marfudat: vec![SababRafd::BilaShajaratAnwa { naw_kaen: None, adad }],
            manqus: true,
        };
    }

    let fahras = FahrasKaainat::ibni(mulsal);
    // The file's own name, and the key every shared table in it is published
    // under. A bundled `SerializedFile` is named by the bundle's directory entry
    // — the `CAB-…` an external reference in another bundle spells out — and a
    // loose one by its path, which is reduced to its file name for the same
    // reason.
    let mushtarak =
        ijma_bayanat_mushtaraka(mulsal, &fahras, ism_malaf(asl.unwrap_or(hawiya)));
    let mut warsha = Warsha { hawiya, asl, madakhil: Vec::new(), linat: Vec::new() };
    let mut naqisa: BTreeMap<Option<String>, usize> = BTreeMap::new();

    for madkhal in mulsal.kaainat() {
        let natija = match madkhal.sanf {
            SANF_TEXTASSET => istakhrij_nass_asl(mulsal, madkhal, &mut warsha),
            SANF_MONOBEHAVIOUR => {
                istakhrij_suluk(mulsal, madkhal, &fahras, &mushtarak, &mut warsha)
            }
            _ => continue,
        };
        if let Err(khata) = natija {
            if khata.bila_shajara() {
                let miftah = match &khata {
                    KhataKaain::BilaShajaratAnwa { naw }
                    | KhataKaain::IsmMajhul { naw, .. } => naw.clone(),
                    KhataKaain::NawMarjiMajhul { ism } => Some(ism.clone()),
                    _ => None,
                };
                *naqisa.entry(miftah).or_insert(0_usize) += 1;
            } else {
                warsha.lin(khata.ila_sabab());
            }
        }
    }

    let mut marfudat: Vec<SababRafd> = naqisa
        .into_iter()
        .map(|(naw, adad)| SababRafd::BilaShajaratAnwa { naw_kaen: naw, adad })
        .collect();
    marfudat.extend(warsha.linat);

    HasilatMulsal { madakhil: warsha.madakhil, marfudat, manqus: fahras.manqus() }
}

/// The class one `MonoBehaviour` is an instance of, decided without reading it.
///
/// Two sources, in order, and the second is not a fallback for exotic builds.
/// The type tree's root usually names the script's own class — `StringTable`,
/// `TextMeshProUGUI` — because Unity generated the tree for that class. But when
/// the tree was generated for the *base* type the root is literally
/// `MonoBehaviour`, which is what every Addressables bundle in R.E.P.O. writes
/// for every localization asset in the game.
///
/// The second source is the file's own script-type table: the type's
/// `fahras_script` indexes it, the entry names a `MonoScript`, and that
/// script's `m_ClassName` is already in the object index. Costing nothing
/// beyond two lookups, it is what lets the shared-table pre-pass recognise a
/// `SharedTableData` *before* deciding whether to read it — and without it the
/// pre-pass finds nothing, every string table loses its keys, and the shared
/// tables are harvested as though their entry keys were prose.
fn ism_mukawwin(
    mulsal: &Mulsal<'_>,
    madkhal: &MadkhalKaain,
    fahras: &FahrasKaainat,
) -> Option<String> {
    let naw = mulsal.naw_kaain(madkhal)?;
    if let Some(ism) = naw.shajara.as_ref().and_then(|shajara| shajara.uqad().first())
        .and_then(|uqda| uqda.naw.as_deref())
        && sath_akhir(ism) != "MonoBehaviour"
    {
        return Some(ism.to_owned());
    }
    let marja = usize::try_from(naw.fahras_script)
        .ok()
        .and_then(|fahras_script| mulsal.scripts().get(fahras_script))?;
    if marja.fahras_malaf != 0 {
        // The script lives in another file, so its class name is not in this
        // file's index and inventing one would name the component wrongly.
        return None;
    }
    fahras.ism_script(marja.hawiya_masar).map(str::to_owned)
}

/// The last segment of a dotted managed name.
///
/// `UnityEngine.Localization.Tables.StringTable` is `StringTable`, and matching
/// on the last segment rather than on the whole name is what lets this recognise
/// the same class across the package versions that moved it between namespaces.
fn sath_akhir(ism: &str) -> &str {
    ism.rsplit('.').next().unwrap_or(ism)
}

// ---------------------------------------------------------------------------
// TextAsset
// ---------------------------------------------------------------------------

/// Reads a `TextAsset` (class 49): its name, and its content as raw bytes.
///
/// The content is deliberately **not** read through [`iqra_qeema`]. Unity
/// declares `m_Script` as a `string`, and the generic reader would decode it as
/// UTF-8 and refuse the entire object when it is not — losing the asset's name
/// as well, and reporting a Shift-JIS script as a corrupt container. Reading the
/// same field as a length and a byte run, and deciding about the encoding
/// afterwards, is the difference between a refusal that says what is wrong and
/// one that sends the user to verify their game files.
///
/// # Errors
///
/// [`KhataKaain::BilaShajaratAnwa`] and [`KhataKaain::IsmMajhul`] as for any
/// object, [`KhataKaain::HaqlTalif`] for a truncated field, and
/// [`KhataKaain::TarmizGhayrMaruf`] when the content is not UTF-8 and the
/// container declares no encoding to convert it from.
fn istakhrij_nass_asl(
    mulsal: &Mulsal<'_>,
    madkhal: &MadkhalKaain,
    warsha: &mut Warsha<'_>,
) -> Result<(), KhataKaain> {
    let naw = mulsal
        .naw_kaain(madkhal)
        .ok_or(KhataKaain::BilaShajaratAnwa { naw: None })?;
    let shajara = naw
        .shajara
        .as_ref()
        .ok_or(KhataKaain::BilaShajaratAnwa { naw: None })?;
    let ism_naw = shajara.uqad().first().and_then(|uqda| uqda.naw.clone());
    if shajara.khali() {
        return Err(KhataKaain::BilaShajaratAnwa { naw: ism_naw });
    }
    if let Some((fahras, izaha)) = shajara.majhula() {
        return Err(KhataKaain::IsmMajhul { naw: ism_naw, fahras, izaha });
    }

    let mut qari = mulsal.qari_kaain(madkhal)?;
    let siyaq =
        SiyaqQira { shajara, anwa_marjiiya: mulsal.anwa_marjiiya(), jism_marji: false };
    let mut ism = String::new();
    let mut khaam: Option<Vec<u8>> = None;

    let nihaya = shajara.nihayat_farr(0);
    let mut i = 1_usize;
    while i < nihaya {
        let uqda = shajara.uqad().get(i).ok_or_else(|| KhataKaain::ShajaraShadha {
            naw: ism_jidhr(shajara),
            sabab: format!("node {i} vanished mid-walk"),
        })?;
        let ism_haql = uqda.ism.clone().unwrap_or_default();
        let bayti = ism_haql == "m_Script" && uqda.naw.as_deref() == Some("string");
        if bayti {
            let bayt = iqra_nass_khaam(&mut qari, "m_Script")?;
            if uqda.yuhadhi() {
                qari.hadhi(4);
            }
            khaam = Some(bayt);
        } else {
            let qeema = iqra_qeema(&siyaq, i, &mut qari, 1)?;
            if ism_haql == "m_Name"
                && let Some(nass) = qeema.nass() {
                    nass.clone_into(&mut ism);
                }
        }
        let baad = shajara.nihayat_farr(i);
        i = if baad > i { baad } else { i.saturating_add(1) };
    }

    let mawqi_asl = if ism.is_empty() {
        "TextAsset".to_owned()
    } else {
        format!("TextAsset/{ism}")
    };

    if !ism.is_empty() {
        let mawqi = warsha.mawqi(mawqi_asl.clone(), Some("m_Name".to_owned()), None);
        // Declared, not inferred. `m_Name` is Unity's own field for an object's
        // asset name and holds nothing else in any build, which is exactly what
        // `tasrih` is for — the shared classifier would otherwise read the
        // segment `name` and answer `Ism`, turning every asset name in the game
        // into a character name a translator is asked to render in Arabic.
        warsha.adif(ansha_mudkhal(
            TalabMudkhal::jadeed(mawqi, &ism)
                .bi_tasrih(TasnifNass::Dakhili, THIQAT_BUNYA)
                .bi_tarmiz(Some(TARMIZ_MULSAL.to_owned())),
        ));
    }

    let Some(bayt) = khaam else {
        return Ok(());
    };
    if bayt.len() > AQSA_TUL_NASS {
        warsha.lin(SababRafd::TajawuzHadd {
            hadd: "TextAsset content length".to_owned(),
            qeema: tul_u64(bayt.len()),
            saqf: tul_u64(AQSA_TUL_NASS),
        });
        return Ok(());
    }

    // A byte-order mark is stripped from the text and recorded in the encoding,
    // because writing back has to reproduce it: a `.json` a game reads with a
    // parser that expects the mark will not load without it.
    let (jism, tarmiz) = match bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(baqi) => (baqi, "UTF-8 with BOM"),
        None => (bayt.as_slice(), "UTF-8"),
    };
    let nass = std::str::from_utf8(jism).map_err(|khata| KhataKaain::TarmizGhayrMaruf {
        asl: if ism.is_empty() { "an unnamed TextAsset".to_owned() } else { ism.clone() },
        mawqi: khata.valid_up_to(),
    })?;
    if nass.trim().is_empty() {
        return Ok(());
    }

    // No `tasrih` here. What a `TextAsset`'s content *is* depends entirely on
    // what the developer put in it, and the asset's own name — which is in the
    // location this entry carries — is the evidence the shared classifier reads.
    // A file called `dialogue_ch1.json` classifies as dialogue and one called
    // `spawn_table.csv` does not, and neither judgement belongs in this module.
    let mawqi = warsha.mawqi(mawqi_asl, Some("m_Script".to_owned()), None);
    warsha.adif(ansha_mudkhal(
        TalabMudkhal::jadeed(mawqi, nass).bi_tarmiz(Some(tarmiz.to_owned())),
    ));
    Ok(())
}

/// Reads a length-prefixed run without decoding it.
///
/// Unity's aligned string, minus the UTF-8 step. The four-byte padding is
/// consumed here exactly as the decoding reader consumes it, because the cursor
/// has to end in the same place either way.
fn iqra_nass_khaam(qari: &mut Qari<'_>, ism: &'static str) -> Result<Vec<u8>, KhataKaain> {
    let tul = qari
        .iqra_i32(ism)
        .map_err(|sabab| KhataKaain::HaqlTalif { ism: ism.to_owned(), sabab })?;
    let adad = u32::try_from(tul).map_err(|_| KhataKaain::TajawuzHadd {
        hadd: "raw string length",
        qeema: u64::MAX,
        saqf: u64::from(u32::MAX),
    })?;
    let baqi = qari.baqi();
    if u64::from(adad) > baqi {
        return Err(KhataKaain::TajawuzHadd {
            hadd: "raw string length",
            qeema: u64::from(adad),
            saqf: baqi,
        });
    }
    let bayt = qari
        .iqra_bayt(ism, u64::from(adad))
        .map_err(|sabab| KhataKaain::HaqlTalif { ism: ism.to_owned(), sabab })?
        .to_vec();
    qari.hadhi(4);
    Ok(bayt)
}

// ---------------------------------------------------------------------------
// MonoBehaviour
// ---------------------------------------------------------------------------

/// Reads a `MonoBehaviour` (class 114) through its type tree.
///
/// **When the tree is absent this refuses and extracts nothing.** That is the
/// rule this whole module is built around and it is enforced one level down, in
/// [`iqra_kaain`], so that every path into an object goes through it.
///
/// With a tree, the object is walked generically and every string in it is
/// collected with its field path. Three shapes are then recognised by the
/// component's own class name and handled specially, because each carries a key
/// the engine promises to keep stable and Taarib's derived identity should defer
/// to it: Unity Localization's `StringTable`, its `SharedTableData`, and I2
/// Localization's `LanguageSource`.
///
/// # Errors
///
/// [`KhataKaain::BilaShajaratAnwa`] when the build stripped the layout, and the
/// read failures of [`iqra_kaain`] otherwise.
fn istakhrij_suluk(
    mulsal: &Mulsal<'_>,
    madkhal: &MadkhalKaain,
    fahras: &FahrasKaainat,
    mushtarak: &SiyaqTawtin,
    warsha: &mut Warsha<'_>,
) -> Result<(), KhataKaain> {
    let ism_shajara = ism_mukawwin(mulsal, madkhal, fahras);
    if ism_shajara.as_deref().map(sath_akhir) == Some("SharedTableData") {
        // Consumed by the pre-pass. Its own strings are the entry keys, which
        // are the developer's identifiers rather than text, and they are already
        // carried by every entry that uses them as `miftah_muharrik`.
        //
        // Consumed, not swallowed: a table the pre-pass could not read is
        // re-raised here so the report says so, because it is the reason every
        // string in its collection is about to lose its key.
        return match mushtarak.akhta.get(&madkhal.hawiya_masar) {
            Some(khata) => Err(khata.clone()),
            None => Ok(()),
        };
    }

    let qeema = iqra_kaain(mulsal, madkhal)?;

    // [`ism_mukawwin`] answered from the tree root or the script-type table.
    // When neither had it, the object's own `m_Script` still might: a file whose
    // script-type table is empty — Unity writes none before format 11 — leaves
    // the reference in the object as the only link to the class name.
    let mukawwin = ism_shajara.or_else(|| {
        qeema
            .haql("m_Script")
            .and_then(QeemaHaql::ishara)
            .filter(|ishara| ishara.mahalli() && !ishara.khali())
            .and_then(|ishara| fahras.ism_script(ishara.hawiya_masar))
            .map(str::to_owned)
    });
    let sath = mukawwin.as_deref().map_or("", sath_akhir);

    if sath == "StringTable" {
        let bayanat = jadwal_mushtarak(&qeema, mushtarak, mulsal.kharijiyat());
        istakhrij_jadwal_tawtin(&qeema, bayanat.as_deref(), warsha);
        return Ok(());
    }
    if sath.contains("LanguageSource") {
        istakhrij_masdar_lughat(&qeema, warsha);
        return Ok(());
    }

    let masar_kaain = qeema
        .haql("m_GameObject")
        .and_then(QeemaHaql::ishara)
        .filter(|ishara| ishara.mahalli() && !ishara.khali())
        .and_then(|ishara| fahras.masar_kaain(ishara.hawiya_masar));

    // Without an object path the component's class name is the position. It is
    // weaker — two `TextMeshProUGUI`s in one file share it — and it is honest:
    // identity also hashes the field and the text, so two components sharing a
    // class, a field *and* a string are one string as far as translation is
    // concerned, which is what `JadwalNusus` folds them into anyway.
    let mawqi_nass = masar_kaain.unwrap_or_else(|| match &mukawwin {
        Some(ism) => ism.clone(),
        None => format!("MonoBehaviour/class {}", madkhal.sanf),
    });

    let mut nusus = Vec::new();
    let mut masar = String::new();
    ijma_nusus(&qeema, &mut masar, &mut nusus, 0);
    if nusus.len() >= AQSA_NUSUS_LIKAAIN {
        warsha.lin(SababRafd::TajawuzHadd {
            hadd: "strings in one component".to_owned(),
            qeema: tul_u64(nusus.len()),
            saqf: tul_u64(AQSA_NUSUS_LIKAAIN),
        });
    }

    let quyud = quyud_min_kaain(&qeema);
    let mutakallim = mutakallim_min_kaain(&qeema);

    for mahsud in nusus {
        // The component's class name goes in front of the field path, so the
        // shared classifier sees `TextMeshProUGUI.m_text` rather than a bare
        // `m_text`. That is the whole of this module's contribution to
        // classification: it supplies the names, `crate::tasnif` applies the
        // rule, and there is exactly one rule in this product.
        let haql = match &mukawwin {
            Some(ism) => format!("{ism}.{}", mahsud.masar),
            None => mahsud.masar.clone(),
        };
        // The component's character limit and font size describe the text it
        // *draws*, not its name or its asset paths. Attached by field rather
        // than by classification, because a limit on the wrong string tells
        // Phase 14 that a node name has to fit in twelve characters and it will
        // reject a perfectly good translation on the strength of it.
        let quyud_hali = if huwa_haql_marsum(&mahsud.masar) {
            quyud.clone()
        } else {
            QuyudNass::default()
        };
        let mawqi = warsha.mawqi(mawqi_nass.clone(), Some(haql), None);
        let talab = TalabMudkhal::jadeed(mawqi, &mahsud.nass)
            .bi_mutakallim(mutakallim.clone())
            .bi_jiwar(mahsud.jiwar)
            .bi_tarmiz(Some(TARMIZ_MULSAL.to_owned()));
        let mut mudkhal = ansha_mudkhal(TalabMudkhal { quyud: quyud_hali, ..talab });

        // The speaker and the neighbouring lines belong to prose. Cleared once
        // the shared classifier has spoken, because a speaker name attached to a
        // button label is noise a translator has to read past on every row.
        if mudkhal.tasnif != TasnifNass::Hiwar {
            mudkhal.mutakallim = None;
        }
        if mudkhal.tasnif == TasnifNass::Dakhili {
            mudkhal.jiwar = Vec::new();
        }
        warsha.adif(mudkhal);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unity Localization
// ---------------------------------------------------------------------------

/// One `SharedTableData`: the collection's name and its entry-id-to-key map.
///
/// Read in its own pass because a `StringTable` points at it and may be read
/// before it. Keeping the map is what turns an entry id — a number the package
/// assigns from a counter — into `MainMenu/play_button`, which is the key the
/// developer typed and the one that survives a rebuild.
#[derive(Debug, Default)]
struct BayanatMushtaraka {
    ism_majmua: String,
    mafateeh: FxHashMap<i64, String>,
}

/// What the shared-table pre-pass learned about one `SerializedFile`.
///
/// Carries the file's own name rather than deriving it twice, because that name
/// is both the key every shared table in this file is published under and the
/// key a *local* `m_SharedData` reference is looked up by, and the two have to
/// be spelled identically or a table finds nothing in its own container.
#[derive(Debug)]
struct SiyaqTawtin {
    malaf: String,
    akhta: FxHashMap<i64, KhataKaain>,
}

thread_local! {
    /// **Where a shared table found in one container becomes visible to
    /// another.**
    ///
    /// Unity Addressables splits a `SharedTableData` into its own bundle and
    /// leaves every `StringTable` pointing at it across a file boundary —
    /// `m_FileID` names an entry of the external table, not an object in this
    /// file. Extraction reads one `SerializedFile` at a time and never holds
    /// two, so a table read there cannot be reached from here without somewhere
    /// for it to live in between.
    ///
    /// This is that somewhere, and three things decide its shape.
    ///
    /// It is keyed on `(file name, path id)` because that is the only identity
    /// the reference actually carries. The external record's GUID is sixteen
    /// zero bytes in every Addressables bundle observed, and the `StringTable`
    /// has no field naming the collection — its only link is the path
    /// `archive:/CAB-…/CAB-…`, whose last segment is exactly the name the
    /// containing bundle gives that `SerializedFile`. So the name is normalised
    /// the same way on both sides and the pair is exact rather than a guess.
    ///
    /// It is **thread-local**, not process-global. The walk in
    /// [`crate::unity::istakhrij`] is sequential, so one thread sees every
    /// container of one game in sorted order; scoping the index to that thread
    /// means two extractions running at once cannot see each other's tables,
    /// which a global map would have allowed and no key would have prevented.
    /// Two extractions run one after another on the *same* thread still share
    /// it, and the residual there is narrow: a bundled file's name is a content
    /// hash, so two games naming one are naming the same bytes, and a table in
    /// a loose file is republished under its own key by the pre-pass before any
    /// string table in that file is read — so a same-file reference always sees
    /// its own game's table, whatever ran before it.
    ///
    /// It resolves **forwards only**: a string table read before its shared
    /// table falls back to `#id` exactly as it did before this index existed.
    /// That is a real limit, and it is deterministic rather than incidental —
    /// the walk is sorted, so the answer is the same on every machine — and the
    /// package's own group naming puts `localization-assets-shared` ahead of
    /// `localization-string-tables-*`. Resolving backwards would need a second
    /// pass over containers this module does not own and cannot start.
    static MUSHTARAK: RefCell<FxHashMap<(String, i64), Rc<BayanatMushtaraka>>> =
        RefCell::new(FxHashMap::default());
}

/// The name a cross-container reference is keyed on.
///
/// The last path segment, lowercased. An external record spells its target
/// `archive:/CAB-186b…/CAB-186b…` inside a bundle and `sharedassets1.assets`
/// beside a loose file, and the publishing side knows the same file as
/// `CAB-186b…` or `REPO_Data/sharedassets1.assets`. Reducing both to the bare
/// name is what makes those two spellings the same key.
fn ism_malaf(masar: &str) -> String {
    masar.rsplit(['/', '\\']).next().unwrap_or(masar).to_ascii_lowercase()
}

/// Publishes one shared table under the file it was read from.
fn asjil_mushtarak(malaf: &str, hawiya: i64, bayanat: BayanatMushtaraka) {
    MUSHTARAK.with_borrow_mut(|khareeta| {
        // Bounded, because nothing clears this index and Studio extracts many
        // games in one process. Past the ceiling a table is not published at
        // all and every string that would have used it falls back to `#id`,
        // which is the behaviour this index replaced — a ceiling that degrades
        // to the old answer rather than to a wrong one. Re-publishing an
        // already-known key is always allowed, so a second read of the same
        // file refreshes rather than being dropped.
        if khareeta.len() >= AQSA_JADAWIL_MUSHTARAKA
            && !khareeta.contains_key(&(malaf.to_owned(), hawiya))
        {
            return;
        }
        let _ = khareeta.insert((malaf.to_owned(), hawiya), Rc::new(bayanat));
    });
}

/// The shared table one reference names, from whichever container held it.
fn jid_mushtarak(malaf: &str, hawiya: i64) -> Option<Rc<BayanatMushtaraka>> {
    MUSHTARAK.with_borrow(|khareeta| khareeta.get(&(malaf.to_owned(), hawiya)).map(Rc::clone))
}

/// Reads every `SharedTableData` in the file and publishes it.
///
/// A table that would not read is *kept*, as the failure it was. It used to be
/// dropped in silence here, which cost twice: the report never mentioned it, and
/// every `StringTable` that pointed at it lost its keys with no explanation
/// anywhere. Now the failure is returned by path id and re-raised when the walk
/// reaches that object, so it goes through the same refusal machinery as any
/// other unreadable component.
fn ijma_bayanat_mushtaraka(
    mulsal: &Mulsal<'_>,
    fahras: &FahrasKaainat,
    malaf: String,
) -> SiyaqTawtin {
    let mut akhta: FxHashMap<i64, KhataKaain> = FxHashMap::default();
    for madkhal in mulsal.kaainat() {
        if madkhal.sanf != SANF_MONOBEHAVIOUR {
            continue;
        }
        if ism_mukawwin(mulsal, madkhal, fahras).as_deref().map(sath_akhir)
            != Some("SharedTableData")
        {
            continue;
        }
        let qeema = match iqra_kaain(mulsal, madkhal) {
            Ok(qeema) => qeema,
            Err(khata) => {
                let _ = akhta.insert(madkhal.hawiya_masar, khata);
                continue;
            }
        };
        let ism_majmua = qeema
            .haql("m_TableCollectionName")
            .and_then(QeemaHaql::nass)
            .unwrap_or_default()
            .to_owned();
        let mut mafateeh: FxHashMap<i64, String> = FxHashMap::default();
        if let Some(anasir) = qeema.haql("m_Entries").and_then(QeemaHaql::anasir) {
            for unsur in anasir {
                let (Some(raqm), Some(miftah)) = (
                    unsur.haql("m_Id").and_then(QeemaHaql::sahih),
                    unsur.haql("m_Key").and_then(QeemaHaql::nass),
                ) else {
                    continue;
                };
                let _ = mafateeh.insert(raqm, miftah.to_owned());
            }
        }
        asjil_mushtarak(
            &malaf,
            madkhal.hawiya_masar,
            BayanatMushtaraka { ism_majmua, mafateeh },
        );
    }
    SiyaqTawtin { malaf, akhta }
}

/// The shared table a `StringTable`'s `m_SharedData` points at.
///
/// A local reference is resolved through the same index the cross-container one
/// is, under this file's own name, so there is one lookup rather than two that
/// could disagree.
fn jadwal_mushtarak(
    qeema: &QeemaHaql,
    siyaq: &SiyaqTawtin,
    kharijiyat: &[MarjaKhariji],
) -> Option<Rc<BayanatMushtaraka>> {
    let ishara = qeema.haql("m_SharedData").and_then(QeemaHaql::ishara)?;
    if ishara.khali() {
        return None;
    }
    if ishara.mahalli() {
        return jid_mushtarak(&siyaq.malaf, ishara.hawiya_masar);
    }
    // `m_FileID` indexes the external table from one, because zero is this file.
    let fahras = usize::try_from(ishara.fahras_malaf).ok()?.checked_sub(1)?;
    let khariji = kharijiyat.get(fahras)?;
    jid_mushtarak(&ism_malaf(&khariji.masar), ishara.hawiya_masar)
}

/// Reads a Unity Localization `StringTable`.
///
/// **Every locale's table is read, and the locale is part of the engine key.**
/// A game ships one `StringTable` asset per locale and they all carry the same
/// entry ids; folding them together would need a rule for which locale is the
/// source, and there is nothing in the asset that says. Guessing "English"
/// works until a Japanese game with no English table presents its French
/// translation as the source text, and a translator produces Arabic translated
/// from French without ever being told. So all of them are kept, distinguished,
/// and the interface picks.
fn istakhrij_jadwal_tawtin(
    qeema: &QeemaHaql,
    bayanat: Option<&BayanatMushtaraka>,
    warsha: &mut Warsha<'_>,
) {
    let majmua = bayanat.map_or("", |bayanat| bayanat.ism_majmua.as_str());
    let lugha = qeema
        .masar_haql(&["m_LocaleId", "m_Code"])
        .and_then(QeemaHaql::nass)
        .unwrap_or_default();

    let Some(anasir) = qeema.haql("m_TableData").and_then(QeemaHaql::anasir) else {
        return;
    };

    for unsur in anasir {
        let Some(nass) = unsur.haql("m_Localized").and_then(QeemaHaql::nass) else {
            continue;
        };
        if nass.trim().is_empty() || nass.len() > AQSA_TUL_NASS {
            continue;
        }
        let raqm = unsur.haql("m_Id").and_then(QeemaHaql::sahih);
        let miftah_madkhal = raqm
            .and_then(|raqm| bayanat.and_then(|bayanat| bayanat.mafateeh.get(&raqm)))
            .map(String::as_str);
        let huwiya = match (miftah_madkhal, raqm) {
            (Some(miftah), _) => miftah.to_owned(),
            (None, Some(raqm)) => format!("#{raqm}"),
            // No key and no id is an entry with no identity of its own. Skipped
            // rather than given a positional one, because a positional identity
            // in a table the developer edits is the renumbering failure this
            // product's identity model exists to avoid.
            (None, None) => continue,
        };

        let mawqi_nass = if majmua.is_empty() {
            format!("StringTable/{huwiya}")
        } else {
            format!("StringTable/{majmua}/{huwiya}")
        };
        let haql = if lugha.is_empty() {
            "m_Localized".to_owned()
        } else {
            format!("m_Localized@{lugha}")
        };
        let mawqi = warsha.mawqi(
            mawqi_nass,
            Some(haql),
            Some(miftah_tawtin(majmua, &huwiya, lugha)),
        );
        // `bi_nizam_tawtin` is the signal that matters here and it is a fact
        // rather than a hint: a developer put this string in a localization
        // table, which is the one act that settles whether a player reads it.
        // The shared classifier treats it as rank three and refuses to call any
        // of these internal, however key-shaped the entry's name looks.
        warsha.adif(ansha_mudkhal(
            TalabMudkhal::jadeed(mawqi, nass)
                .bi_nizam_tawtin()
                .bi_tarmiz(Some(TARMIZ_MULSAL.to_owned())),
        ));
    }
}

/// The engine key one localization entry gets.
///
/// Namespaced with the system's name so a Unity Localization key and an I2 term
/// spelled the same cannot collide, and suffixed with the locale so the same
/// entry in two locales does not.
fn miftah_tawtin(majmua: &str, madkhal: &str, lugha: &str) -> String {
    let mut miftah = String::from("UnityLocalization/");
    if !majmua.is_empty() {
        miftah.push_str(majmua);
        miftah.push('/');
    }
    miftah.push_str(madkhal);
    if !lugha.is_empty() {
        miftah.push('@');
        miftah.push_str(lugha);
    }
    miftah
}

// ---------------------------------------------------------------------------
// I2 Localization
// ---------------------------------------------------------------------------

/// Reads an I2 Localization `LanguageSource`.
///
/// I2 stores everything in one component: a list of languages, and a list of
/// terms each holding one string per language in the same order. The term is the
/// developer's own key — it is what the game passes to
/// `LocalizationManager.GetTranslation` — so it goes in
/// [`MawqiNass::miftah_muharrik`] and identity comes from it rather than from
/// the term's position in the list, which changes whenever anyone adds a line.
///
/// `TermType` is honoured: I2 uses the same table for sprite, font and audio
/// references, and those rows hold asset names rather than text. They are kept
/// and classified internal rather than dropped, because a row misfiled as an
/// asset that is actually a line is recoverable and a deleted one is not.
fn istakhrij_masdar_lughat(qeema: &QeemaHaql, warsha: &mut Warsha<'_>) {
    let masdar = qeema.haql("mSource").unwrap_or(qeema);
    let rumuz: Vec<String> = masdar
        .haql("mLanguages")
        .and_then(QeemaHaql::anasir)
        .map(|anasir| {
            anasir
                .iter()
                .map(|lugha| {
                    lugha
                        .haql("Code")
                        .and_then(QeemaHaql::nass)
                        .or_else(|| lugha.haql("Name").and_then(QeemaHaql::nass))
                        .unwrap_or_default()
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default();

    let Some(mustalahat) = masdar.haql("mTerms").and_then(QeemaHaql::anasir) else {
        return;
    };

    for mustalah in mustalahat {
        let Some(ism) = mustalah.haql("Term").and_then(QeemaHaql::nass) else {
            continue;
        };
        if ism.trim().is_empty() {
            continue;
        }
        let naw_mustalah = mustalah.haql("TermType").and_then(QeemaHaql::sahih).unwrap_or(0);
        let Some(lughat) = mustalah.haql("Languages").and_then(QeemaHaql::anasir) else {
            continue;
        };

        for (fahras, qeemat) in lughat.iter().enumerate() {
            let Some(nass) = qeemat.nass() else {
                continue;
            };
            if nass.trim().is_empty() || nass.len() > AQSA_TUL_NASS {
                continue;
            }
            let ramz = rumuz.get(fahras).map_or("", String::as_str);
            let mawqi_nass = format!("I2/{ism}");
            let haql = if ramz.is_empty() {
                format!("Languages[{fahras}]")
            } else {
                format!("Languages[{fahras}]@{ramz}")
            };
            let mut miftah = format!("I2/{ism}");
            if !ramz.is_empty() {
                miftah.push('@');
                miftah.push_str(ramz);
            }

            let mawqi = warsha.mawqi(mawqi_nass, Some(haql), Some(miftah));
            let talab = TalabMudkhal::jadeed(mawqi, nass)
                .bi_tarmiz(Some(TARMIZ_MULSAL.to_owned()));
            // I2 stores sprite, font and audio references in the same term
            // table as its text, and `TermType` says which. A non-zero type is
            // the container declaring that this row holds an asset name, which
            // is a statement rather than a guess and so goes in `tasrih`. Kept
            // and marked internal rather than dropped: a row misfiled as an
            // asset that is really a line is recoverable, and a deleted one is
            // not.
            let talab = if naw_mustalah == 0 {
                talab.bi_nizam_tawtin()
            } else {
                talab.bi_tasrih(TasnifNass::Dakhili, THIQAT_BUNYA)
            };
            warsha.adif(ansha_mudkhal(talab));
        }
    }
}

/// Whether a field is one a text component actually draws.
///
/// Deliberately a short exact list rather than a keyword scan. The only thing
/// this decides is whether the component's measured constraints — a character
/// limit the engine enforces, a font size — are attached to a string, and
/// attaching a limit to the wrong field is how a valid translation gets rejected
/// in Phase 14 for overflowing a box it was never going in.
fn huwa_haql_marsum(masar: &str) -> bool {
    let akhir = masar.rsplit('.').next().unwrap_or(masar);
    let bila_fahras = match akhir.find(['[', '{']) {
        Some(mawqi) => akhir.get(..mawqi).unwrap_or(akhir),
        None => akhir,
    };
    matches!(
        bila_fahras,
        "m_text" | "m_Text" | "text" | "m_Placeholder" | "m_placeholder" | "m_Localized"
    )
}

/// The encoding every string in a `SerializedFile` is stored in.
///
/// Recorded on each entry rather than left empty because Phase 14 writes back in
/// the encoding this field names, and because the one Unity field that is *not*
/// guaranteed UTF-8 — a `TextAsset`'s content — is read as bytes and refused
/// when it is not, so an entry carrying this label has actually been checked.
const TARMIZ_MULSAL: &str = "UTF-8";
