//! الذاكرة المشتركة — one player's accumulated screen readings, in a form the
//! next player of the same game can use.
//!
//! An overlay session leaves behind a per-game pile of
//! [`taarib_tarjama::dhakira::NawAsl::Mulahaza`] records: lines somebody
//! walked past, read off the screen, and had machine-translated. That pile is
//! the only thing in this product that gets better because somebody *played*
//! rather than because somebody worked, and it is worthless to anyone else
//! while it sits in one machine's `dhakira.db`.
//!
//! ## Why this is not a `.ruqaa`
//!
//! A patch is a signed container an installer writes into a game directory.
//! It goes through `taarib-aman`'s inspection, `taarib-tathbeet`'s permit, a
//! backup, and an uninstaller that can put the game back. Every one of those
//! exists because a patch **modifies somebody's game**.
//!
//! A memory share modifies nothing. It is read into a `SQLite` file that only
//! Taarib opens; deleting it costs a player some cached lines and touches no
//! game. Routing it through the patch container would mean minting install
//! permits for something that never installs, and would put screen readings —
//! the weakest text in the product — inside the same artifact type as
//! reviewed translations, where a listing cannot tell them apart. So it is a
//! different artifact with different rules, and the rules are stricter in the
//! one direction that matters:
//!
//! * **It carries observations and nothing else.** There is no field in
//!   [`QaydMushtarak`] for a contributor, a review state, or a project. A
//!   reviewed translation leaves a machine through `taarib-taqdeem`'s
//!   submission path or it does not leave at all. This is why an import
//!   cannot manufacture human provenance *however the file is edited*: the
//!   claim is not expressible, not merely rejected.
//! * **Every entry carries its recognizer's confidence, or says there was
//!   none.** [`taarib_tarjama::dhakira::ThiqatQira`] travels whole. A share
//!   whose readings were unmeasured says so on every row and in its header,
//!   and the importer can act on it.
//! * **Nothing leaves without a name on it.** The bytes are signed with the
//!   sharer's key and carry their [`MusahimId`], exactly as a rating or a
//!   report does in `taarib_mustawda::taqyeem`. A share with a broken
//!   signature is refused, so a bad share is attributable and revocable
//!   rather than anonymous.
//! * **Nothing leaves without the user having seen what leaves.**
//!   [`IdhnMusharaka`] is minted only against the fingerprint of the exact
//!   entry set, and [`saddir`] takes one by value — the same construction
//!   `taarib_taqdeem::bawwaba::IjtiyazTaqdeem` uses for submission and
//!   `taarib_tabaqa::sidq::Iqrar` uses for the tier-3 disclosure.
//!
//! ## The file
//!
//! One file, not a directory, because a share is one release asset and one
//! thing to hand somebody:
//!
//! ```text
//! <one line of JSON: the header>\n<zstd(JSON Lines: one entry per line)>
//! ```
//!
//! The header is a single line because `serde_json::to_vec` emits no raw
//! newlines, so the split point is unambiguous without a length field. The
//! body is compressed as one stream rather than per line — a memory share is
//! thousands of short, extremely similar strings, which is the case zstd's
//! window is for. The header records the body's BLAKE3 and its uncompressed
//! length, and the signature is over a canonical message that includes that
//! hash, so signing the header signs the body.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use taarib_khatm::{MiftahAam, MiftahKhass};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::TasnifNass;
use taarib_tarjama::dhakira::{
    AslQayd, Dhakira, NawAsl, QaydDhakira, QaydId, QaydJadid, ThiqatQira, miftah_muwahhad,
};
use taarib_tarjama::khata::KhataTarjama;
use taarib_usus::khata::Tafsir;

use crate::khata::{KhataWarsha, NatijatWarsha};

/// The share format's schema version.
pub const ISDAR_MUSHTARAKA: u32 = 1;

/// The extension a share file carries.
///
/// Deliberately not `.ruqaa`: the operating system associates that extension
/// with the installer, and a double-click on a memory share must not open an
/// install flow for something that installs nothing.
pub const LAHIQA: &str = "dhakira";

/// zstd level for the body.
///
/// Nineteen rather than the bundle's three. A share is written once, read
/// many times, and downloaded by everyone who plays the game; the body is
/// thousands of short and extremely similar strings, which is exactly where
/// the high levels earn their time.
pub const MUSTAWA_DAGHT: i32 = 19;

/// The largest uncompressed body this build will decompress.
///
/// Sixty-four mebibytes, which is far past any real accumulation — a hundred
/// thousand readings of two hundred characters is about twenty. The ceiling
/// exists because the body arrives compressed from a stranger and a
/// decompressor with no bound is a decompression bomb waiting for one.
pub const AQSA_HAJM_JASAD: u64 = 64 * 1024 * 1024;

/// The largest header line this build will read.
///
/// A header is a few hundred bytes. The bound stops a file whose first
/// megabyte contains no newline from being buffered as one "line".
pub const AQSA_HAJM_TARWISA: usize = 64 * 1024;

/// The default confidence floor an export applies to measured readings.
///
/// Sixty, which is the boundary `taarib_tabaqa::sijill_qira` already draws
/// between "reading may be wrong" and "weak reading" in the panel a player
/// sees. Sharing what the product itself labels weak would be putting the
/// worst readings in front of the most people.
pub const ATABAT_THIQA: u8 = 60;

/// How many records one export page reads.
pub const HAJM_SAFHA: u32 = 256;

/// Domain separator for a signed share header.
///
/// Byte-identical in shape to `taarib_mustawda::taqyeem`'s separators and
/// distinct in content, so a signature made over a rating can never verify as
/// a share and the reverse.
const FASIL_MATN: &[u8] = b"taarib.dhakira.mushtaraka.v1\0";

// ---------------------------------------------------------------------------
// القيد — one shared reading
// ---------------------------------------------------------------------------

/// One reading, as it travels.
///
/// Every field is something a recognizer or a translator produced. There is
/// no contributor field, no review state, and no project: see this module's
/// header for why their absence is the guarantee rather than a simplification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QaydMushtarak {
    /// What the recognizer read.
    pub asl: String,
    /// The Arabic it was given.
    pub arabi: String,
    /// What kind of string it appears to be.
    pub tasnif: TasnifNass,
    /// The overlay region it came from, when the sharer's overlay named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mintaqa: Option<String>,
    /// Which recognizer read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qari: Option<String>,
    /// Which provider translated the reading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muzawwid: Option<String>,
    /// What the recognizer said about the reading, or that it said nothing.
    pub thiqa: ThiqatQira,
    /// How many independent sightings the sharer's memory recorded.
    ///
    /// Travels because it is the tie-break between two readings of one line,
    /// and an importer that dropped it would be throwing away the only
    /// evidence that distinguishes a line seen forty times from a line seen
    /// once. It is *not* trusted as stated — see [`idmij`].
    pub mushahadat: u32,
}

impl QaydMushtarak {
    /// The stored record as a shareable entry, or [`None`] when it is not one.
    ///
    /// [`None`] for anything that is not an observation. That refusal is the
    /// last of three: the export query already asks only for
    /// [`NawAsl::Mulahaza`] rows, [`QaydMushtarak`] has no way to express a
    /// human origin, and this returns nothing for a record whose rank says
    /// otherwise. Three refusals for one rule because it is the rule that
    /// decides whether any of this is a good idea.
    #[must_use]
    pub fn min_qayd(qayd: &QaydDhakira) -> Option<Self> {
        if qayd.asl.naw != NawAsl::Mulahaza {
            return None;
        }
        Some(Self {
            asl: qayd.masdar.clone(),
            arabi: qayd.hadaf.clone(),
            tasnif: qayd.tasnif,
            mintaqa: qayd.siyaq.clone(),
            qari: qayd.asl.qari.clone(),
            muzawwid: qayd.asl.muzawwid.clone(),
            thiqa: qayd.asl.thiqa_qira,
            mushahadat: qayd.asl.mushahadat,
        })
    }

    /// Whether this entry is usable at all: both sides fold to a real key.
    #[must_use]
    pub fn salih(&self) -> bool {
        !miftah_muwahhad(&self.asl).is_empty() && !miftah_muwahhad(&self.arabi).is_empty()
    }

    /// The entry as a memory record for one game.
    ///
    /// The origin is [`AslQayd::Mulahaza`] unconditionally and there is no
    /// parameter that could make it anything else.
    #[must_use]
    pub fn ila_qayd(&self, luba: LubaId, ism_luba: Option<String>) -> QaydJadid {
        QaydJadid {
            masdar: self.asl.clone(),
            hadaf: self.arabi.clone(),
            tasnif: self.tasnif,
            mashru: None,
            luba: Some(luba.to_string()),
            ism_luba,
            siyaq: self.mintaqa.clone(),
            nasq_masdar: Vec::new(),
            nasq_hadaf: Vec::new(),
            asl: AslQayd::Mulahaza {
                qari: self.qari.clone(),
                muzawwid: self.muzawwid.clone(),
                thiqa: self.thiqa,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// الترويسة — the header, which is what gets signed
// ---------------------------------------------------------------------------

/// What a share says about itself, before anybody decompresses it.
///
/// Everything a person needs to decide whether to import it — how many
/// readings, from which game, by whom, how many of them anybody measured —
/// is here, so the decision does not require trusting the body first.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TarwisatMushtaraka {
    /// The share schema this file was written with.
    pub isdar: u32,
    /// The game the readings came from.
    pub luba: LubaId,
    /// Its display name, for the import screen.
    pub ism_luba: String,
    /// Who shared it.
    pub musahim: MusahimId,
    /// When, RFC 3339, as the caller supplied it. Nothing here reads a clock.
    pub waqt: String,
    /// How many entries the body holds.
    pub adad: u64,
    /// How many of them carry a confidence a recognizer actually measured.
    ///
    /// Stated in the header because it is the single most important number
    /// for deciding whether to trust a share, and burying it in the body
    /// would mean decompressing a stranger's file to find out.
    pub adad_maqis: u64,
    /// The lowest measured confidence in the body, when any entry has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adna_thiqa: Option<u8>,
    /// The floor the sharer's export applied to measured readings.
    pub atabaa: u8,
    /// BLAKE3 of the uncompressed body, lowercase hex.
    pub basma: String,
    /// The uncompressed body's length in bytes.
    pub hajm: u64,
    /// The sharer's signing key, lowercase hex.
    pub miftah: String,
    /// The signature over the canonical message, lowercase hex.
    pub tawqee: String,
}

impl TarwisatMushtaraka {
    /// How many entries carry no measurement at all.
    #[must_use]
    pub const fn adad_ghayr_maqis(&self) -> u64 {
        self.adad.saturating_sub(self.adad_maqis)
    }

    /// The share as one line, in English, for a log and a report.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut wasf = format!(
            "{} reading(s) of {} shared by {}, {} measured and {} unmeasured",
            self.adad,
            self.ism_luba,
            self.musahim.mukhtasar(),
            self.adad_maqis,
            self.adad_ghayr_maqis(),
        );
        match self.adna_thiqa {
            Some(adna) => {
                let _ = write!(wasf, ", lowest measured confidence {adna}");
            },
            None => wasf.push_str(", no measured confidence anywhere in it"),
        }
        wasf
    }

    /// The same line in Arabic, for the import screen.
    #[must_use]
    pub fn unwan(&self) -> String {
        let mut unwan = format!(
            "{} سطرًا من «{}» شاركها {}؛ {} منها ثقة قراءتها مقيسة و{} غير مقيسة.",
            self.adad,
            self.ism_luba,
            self.musahim.mukhtasar(),
            self.adad_maqis,
            self.adad_ghayr_maqis(),
        );
        if self.adna_thiqa.is_none() {
            unwan.push_str(" ولا يحمل أيّ سطر فيها قياسًا للثقة.");
        }
        unwan
    }

    /// The canonical message the sharer signs and every reader reproduces.
    ///
    /// Domain separator, then the version, then every field that must not be
    /// swappable, each length-prefixed so no two different headers can
    /// produce the same bytes. Same construction as
    /// `taarib_mustawda::taqyeem`'s, for the same reason: a canonical form
    /// assembled by concatenation without lengths lets two different values
    /// collide across a field boundary.
    #[must_use]
    pub fn matn(&self, miftah: &[u8; 32]) -> Vec<u8> {
        let mut matn = Vec::with_capacity(256);
        matn.extend_from_slice(FASIL_MATN);
        matn.extend_from_slice(&ISDAR_MUSHTARAKA.to_le_bytes());
        matn.extend_from_slice(self.luba.uuid().as_bytes());
        lp(&mut matn, self.ism_luba.as_bytes());
        lp(&mut matn, self.musahim.nass().as_bytes());
        lp(&mut matn, self.waqt.as_bytes());
        matn.extend_from_slice(&self.adad.to_le_bytes());
        matn.extend_from_slice(&self.adad_maqis.to_le_bytes());
        matn.push(self.adna_thiqa.unwrap_or(0));
        matn.push(u8::from(self.adna_thiqa.is_some()));
        matn.push(self.atabaa);
        lp(&mut matn, self.basma.as_bytes());
        matn.extend_from_slice(&self.hajm.to_le_bytes());
        matn.extend_from_slice(miftah);
        matn
    }
}

// ---------------------------------------------------------------------------
// الإذن — consent, as a value nothing can forge
// ---------------------------------------------------------------------------

/// What a sharing screen must tell the user before anything leaves.
///
/// Warnings, not blocking checks: each one is a true thing about the payload
/// that a person might reasonably still choose to share. The gate is that
/// they have to say so about *this* payload, per warning, by name.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TahdheerMusharaka {
    /// The readings were taken off the user's own screen.
    ///
    /// The one that matters most and the one a checkbox about "sharing
    /// translations" would never surface. The overlay recognizes whatever is
    /// drawn: a character the player named, another player's account name in
    /// a multiplayer roster, a friends list, a launcher notification, a chat
    /// line. Nothing in this crate can tell those from dialogue, and no
    /// filter is offered that pretends to — the honest move is to say so and
    /// let the person who was looking at the screen decide.
    MuhtawaShakhsi,

    /// Some readings carry no measured confidence.
    ///
    /// Windows Runtime OCR and the bundled portable engine report none at
    /// all, so on Windows and on Linux this fires for essentially every
    /// share. Sharing them anyway is defensible — an unmeasured reading is
    /// not a bad reading — but it must be a decision rather than a default,
    /// because the receiving player has no way to tell them from measured
    /// ones except by this bit travelling with them.
    LamTuqas,

    /// Some readings are below the confidence floor.
    ///
    /// Only reachable when the caller lowered [`KhiyaratMusharaka::atabaa`]
    /// beneath [`ATABAT_THIQA`], which is a thing a caller may legitimately
    /// want and should not do by accident.
    ThiqaMunkhafida,
}

impl TahdheerMusharaka {
    /// The warning's text, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::MuhtawaShakhsi => {
                "هذه الأسطر قُرئت من شاشتك أثناء اللعب، وقد تتضمّن اسم شخصيتك أو \
                 أسماء لاعبين آخرين أو أيّ نصّ آخر كان معروضًا."
            },
            Self::LamTuqas => "بعض هذه الأسطر لا يحمل قياسًا لثقة القراءة؛ محرّك التعرّف لم يُصدر رقمًا.",
            Self::ThiqaMunkhafida => "بعض هذه الأسطر ثقة قراءتها دون الحدّ المعتاد.",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::MuhtawaShakhsi => {
                "These lines were read off your screen while you played, and may \
                 include your character's name, other players' names, or anything \
                 else that was on screen."
            },
            Self::LamTuqas => {
                "Some of these lines carry no measured reading confidence; the \
                 recognizer reported none."
            },
            Self::ThiqaMunkhafida => "Some of these lines were read with below-normal confidence.",
        }
    }
}

/// Which warnings the user has acknowledged.
///
/// The same shape `taarib_taqdeem::bawwaba::Iqrarat` uses, restated here
/// rather than reused because the two vocabularies are different: that one
/// acknowledges facts about a translation package, this one acknowledges
/// facts about screen readings, and one type carrying both would let a
/// submission's acknowledgement satisfy a share's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IqraratMusharaka {
    muqarra: BTreeSet<TahdheerMusharaka>,
}

impl IqraratMusharaka {
    /// Nothing acknowledged.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Acknowledges one warning.
    pub fn aqirr(&mut self, tahdheer: TahdheerMusharaka) {
        let _ = self.muqarra.insert(tahdheer);
    }

    /// Withdraws one acknowledgement.
    pub fn asqit(&mut self, tahdheer: TahdheerMusharaka) {
        let _ = self.muqarra.remove(&tahdheer);
    }

    /// Whether one warning is acknowledged.
    #[must_use]
    pub fn muqarr(&self, tahdheer: TahdheerMusharaka) -> bool {
        self.muqarra.contains(&tahdheer)
    }
}

/// How an export is scoped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KhiyaratMusharaka {
    /// The floor a measured reading must clear.
    pub atabaa: u8,
    /// Whether readings with no measurement at all are included.
    ///
    /// `false` by default, and turning it on raises
    /// [`TahdheerMusharaka::LamTuqas`]. It is off by default even though it
    /// excludes almost everything a Windows or Linux player accumulates,
    /// because the alternative default is shipping unmeasured readings
    /// silently — and the fix for the exclusion is one acknowledged warning,
    /// while the fix for the silence is nothing.
    pub ghayr_maqisa: bool,
}

impl KhiyaratMusharaka {
    /// The default scope: measured readings at or above [`ATABAT_THIQA`].
    #[must_use]
    pub const fn iftiradiya() -> Self {
        Self {
            atabaa: ATABAT_THIQA,
            ghayr_maqisa: false,
        }
    }

    /// Includes readings a recognizer never measured.
    #[must_use]
    pub const fn maa_ghayr_maqisa(mut self) -> Self {
        self.ghayr_maqisa = true;
        self
    }

    /// Sets the floor for measured readings.
    #[must_use]
    pub const fn bi_atabaa(mut self, atabaa: u8) -> Self {
        self.atabaa = atabaa;
        self
    }

    /// Whether one reading passes this scope.
    #[must_use]
    pub const fn yaqbal(self, thiqa: ThiqatQira) -> bool {
        match thiqa {
            ThiqatQira::Ghayr => self.ghayr_maqisa,
            ThiqatQira::Maqisa { .. } => thiqa.yajtaz(self.atabaa),
        }
    }
}

/// What an export would contain, gathered and not yet sent.
///
/// The value a sharing screen shows. It holds the actual entries, so the
/// count, the confidence summary and the fingerprint the user acknowledges
/// all describe the same bytes that will be written — rather than a preview
/// computed from one query and a payload built later from another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusawwadatMusharaka {
    luba: LubaId,
    ism_luba: String,
    khiyarat: KhiyaratMusharaka,
    qayyid: Vec<QaydMushtarak>,
    tahdheerat: BTreeSet<TahdheerMusharaka>,
    basma: String,
}

impl MusawwadatMusharaka {
    /// The game.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// Its display name.
    #[must_use]
    pub fn ism_luba(&self) -> &str {
        &self.ism_luba
    }

    /// The entries that would be written.
    #[must_use]
    pub fn qayyid(&self) -> &[QaydMushtarak] {
        &self.qayyid
    }

    /// How many.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.qayyid.len()
    }

    /// How many carry a measured confidence.
    #[must_use]
    pub fn adad_maqis(&self) -> usize {
        self.qayyid.iter().filter(|qayd| qayd.thiqa.qisat()).count()
    }

    /// Every warning this payload raises, sorted.
    #[must_use]
    pub fn tahdheerat(&self) -> Vec<TahdheerMusharaka> {
        self.tahdheerat.iter().copied().collect()
    }

    /// The fingerprint of the exact entry set.
    ///
    /// BLAKE3 over the serialized body, which is the same body [`saddir`]
    /// writes. It is what makes an acknowledgement be *about* something: an
    /// [`IdhnMusharaka`] minted for one draft cannot be spent on another,
    /// because the permit carries this and [`saddir`] checks it.
    #[must_use]
    pub fn basma(&self) -> &str {
        &self.basma
    }

    /// The permit, when every raised warning is acknowledged.
    ///
    /// # Errors
    ///
    /// [`KhataWarsha::TahdheeratMuallaqa`] naming exactly the warnings that
    /// are still unacknowledged, so a screen can highlight them rather than
    /// saying "something is missing".
    pub fn idhn(&self, iqrarat: &IqraratMusharaka) -> NatijatWarsha<IdhnMusharaka> {
        let naqisa: Vec<TahdheerMusharaka> = self
            .tahdheerat
            .iter()
            .copied()
            .filter(|tahdheer| !iqrarat.muqarr(*tahdheer))
            .collect();
        if !naqisa.is_empty() {
            return Err(KhataWarsha::TahdheeratMuallaqa {
                asma: naqisa
                    .iter()
                    .map(|t| t.wasf_injilizi().to_owned())
                    .collect(),
            });
        }
        Ok(IdhnMusharaka {
            basma: self.basma.clone(),
        })
    }
}

/// Proof that a person was shown what would leave their machine, and agreed.
///
/// No public constructor, no public fields, not `Deserialize`, not `Clone`.
/// The only way to obtain one is [`MusawwadatMusharaka::idhn`], which
/// requires the draft *and* an acknowledgement of every warning that draft
/// raised. [`saddir`] takes one by value and checks its fingerprint against
/// the draft it is writing, so a permit cannot be minted against a small,
/// clean preview and spent on a large one.
///
/// The construction is `taarib_taqdeem::bawwaba::IjtiyazTaqdeem`'s, and it is
/// used here for the same reason: skipping the consent means deleting a
/// parameter from a public signature, which is visible in a review.
#[derive(Debug)]
pub struct IdhnMusharaka {
    basma: String,
}

impl IdhnMusharaka {
    /// The entry set this permit was granted for.
    #[must_use]
    pub fn basma(&self) -> &str {
        &self.basma
    }
}

// ---------------------------------------------------------------------------
// التصدير — gathering and writing
// ---------------------------------------------------------------------------

/// Gathers one game's shareable readings out of a memory.
///
/// Pages [`Dhakira::safha_luba`] for [`NawAsl::Mulahaza`] rows only, keeps
/// the ones `khiyarat` accepts, and raises a warning for every true thing
/// about the result. [`TahdheerMusharaka::MuhtawaShakhsi`] is raised whenever
/// there is anything to share at all, because it is true of every screen
/// reading and there is no test that could make it false.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] when the memory refuses a page, and
/// [`KhataWarsha::Mawrid`] when the body cannot be serialized.
pub fn ijma(
    dhakira: &Dhakira,
    luba: LubaId,
    ism_luba: impl Into<String>,
    khiyarat: KhiyaratMusharaka,
) -> NatijatWarsha<MusawwadatMusharaka> {
    let muarrif = luba.to_string();
    let hadd_safha = usize::try_from(HAJM_SAFHA).unwrap_or(usize::MAX);
    let mut qayyid: Vec<QaydMushtarak> = Vec::new();
    let mut baad: Option<QaydId> = None;
    let mut tahdheerat = BTreeSet::new();

    loop {
        let safha = dhakira
            .safha_luba(&muarrif, NawAsl::Mulahaza, baad, HAJM_SAFHA)
            .map_err(|khata| mawrid(&khata))?;
        let Some(akhir) = safha.last() else {
            break;
        };
        baad = Some(akhir.id);

        for qayd in &safha {
            let Some(mushtarak) = QaydMushtarak::min_qayd(qayd) else {
                continue;
            };
            if !mushtarak.salih() || !khiyarat.yaqbal(mushtarak.thiqa) {
                continue;
            }
            if !mushtarak.thiqa.qisat() {
                let _ = tahdheerat.insert(TahdheerMusharaka::LamTuqas);
            } else if !mushtarak.thiqa.yajtaz(ATABAT_THIQA) {
                let _ = tahdheerat.insert(TahdheerMusharaka::ThiqaMunkhafida);
            }
            qayyid.push(mushtarak);
        }

        if safha.len() < hadd_safha {
            break;
        }
    }

    if !qayyid.is_empty() {
        let _ = tahdheerat.insert(TahdheerMusharaka::MuhtawaShakhsi);
    }

    let jasad = ila_jasad(&qayyid)?;
    Ok(MusawwadatMusharaka {
        luba,
        ism_luba: ism_luba.into(),
        khiyarat,
        qayyid,
        tahdheerat,
        basma: blake3::hash(&jasad).to_hex().to_string(),
    })
}

/// Writes a gathered draft as a signed share.
///
/// Takes the permit by value: a caller who has one has shown the user this
/// exact entry set. The fingerprint is re-checked against the body actually
/// being written, so the permit cannot outlive the draft it was granted for.
///
/// `waqt` is an RFC 3339 timestamp the caller supplies; nothing here reads a
/// clock, exactly as [`crate::tasdir::saddir`] does not.
///
/// # Errors
///
/// [`KhataWarsha::MusharakaFarigha`] when the draft holds no entries —
/// signing an empty share would put a file in the world that costs a
/// download and answers nothing — and [`KhataWarsha::IdhnGhayrMutabiq`] when
/// the permit was granted for a different entry set.
/// [`KhataWarsha::Mawrid`] when the body cannot be serialized or compressed.
pub fn saddir(
    musawwada: &MusawwadatMusharaka,
    idhn: IdhnMusharaka,
    musahim: MusahimId,
    khass: &MiftahKhass,
    waqt: String,
) -> NatijatWarsha<Vec<u8>> {
    if musawwada.qayyid.is_empty() {
        return Err(KhataWarsha::MusharakaFarigha);
    }

    // Destructured rather than read through a reference, so the permit is
    // genuinely spent here: a caller cannot hold one back for a second write.
    let IdhnMusharaka { basma: mamnuh } = idhn;
    let jasad = ila_jasad(&musawwada.qayyid)?;
    let basma = blake3::hash(&jasad).to_hex().to_string();
    if basma != mamnuh {
        return Err(KhataWarsha::IdhnGhayrMutabiq);
    }

    let miftah = khass.aam().bayt();
    let maqis: Vec<u8> = musawwada
        .qayyid
        .iter()
        .filter_map(|qayd| qayd.thiqa.mia())
        .collect();
    let mut tarwisa = TarwisatMushtaraka {
        isdar: ISDAR_MUSHTARAKA,
        luba: musawwada.luba,
        ism_luba: musawwada.ism_luba.clone(),
        musahim,
        waqt,
        adad: u64::try_from(musawwada.qayyid.len()).unwrap_or(u64::MAX),
        adad_maqis: u64::try_from(maqis.len()).unwrap_or(u64::MAX),
        adna_thiqa: maqis.iter().copied().min(),
        atabaa: musawwada.khiyarat.atabaa,
        basma,
        hajm: u64::try_from(jasad.len()).unwrap_or(u64::MAX),
        miftah: hex(&miftah),
        tawqee: String::new(),
    };
    tarwisa.tawqee = hex(&khass.waqqi(&tarwisa.matn(&miftah)));

    let mut bayt = serde_json::to_vec(&tarwisa).map_err(|sabab| KhataWarsha::Mawrid {
        sabab: sabab.to_string(),
    })?;
    bayt.push(b'\n');
    let madghut =
        zstd::encode_all(jasad.as_slice(), MUSTAWA_DAGHT).map_err(|sabab| KhataWarsha::Mawrid {
            sabab: sabab.to_string(),
        })?;
    bayt.extend_from_slice(&madghut);
    Ok(bayt)
}

// ---------------------------------------------------------------------------
// الاستيراد — reading somebody else's
// ---------------------------------------------------------------------------

/// A share whose header parsed, whose body hashed to what the header
/// declared, and whose signature verified against the key the header names.
///
/// No public constructor, no public fields, no `Deserialize`. The only way to
/// obtain one is [`istawrid`], which does all three checks before a single
/// entry is handed out — the same construction
/// `taarib_mustawda::fahras::ShareehaMuwaththaqa` uses for index shards, and
/// for the same reason: unverified content that cannot be represented cannot
/// be read by mistake.
#[derive(Debug, Clone)]
pub struct HuzmaMuwaththaqa {
    tarwisa: TarwisatMushtaraka,
    qayyid: Vec<QaydMushtarak>,
}

impl HuzmaMuwaththaqa {
    /// What the share says about itself.
    #[must_use]
    pub const fn tarwisa(&self) -> &TarwisatMushtaraka {
        &self.tarwisa
    }

    /// The verified entries.
    #[must_use]
    pub fn qayyid(&self) -> &[QaydMushtarak] {
        &self.qayyid
    }

    /// The game these readings came from.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.tarwisa.luba
    }
}

/// Reads a share, verifying the schema, the hash and the signature.
///
/// The key is the caller's, not the file's. The header names a key and the
/// signature is checked against it, but a file that carries its own key and
/// vouches for itself proves only that somebody had *a* key — so this
/// function requires the caller to say which key it expects, exactly as
/// `taarib_mustawda::taqyeem::TaqyeemMuwaqqa::min_bayt` does. Where that key
/// comes from is the caller's problem and it is a real one: a registry
/// listing, a revocation list, or a person pasting a fingerprint.
///
/// # Errors
///
/// [`KhataWarsha::HuzmaTalifa`] when the file has no header line, the header
/// does not parse, the body does not decompress or disagrees with its
/// recorded hash or length, or an entry does not deserialize;
/// [`KhataWarsha::IsdarMajhul`] for a schema this build does not read; and
/// [`KhataWarsha::TawqeeGhayrSalih`] when the header names another key or the
/// signature does not verify.
pub fn istawrid(bayt: &[u8], miftah: &MiftahAam) -> NatijatWarsha<HuzmaMuwaththaqa> {
    let hadd = bayt.len().min(AQSA_HAJM_TARWISA);
    let Some(fasl) = bayt.iter().take(hadd).position(|harf| *harf == b'\n') else {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: "the file has no header line in its first 64 KiB".to_owned(),
        });
    };
    let (nass_tarwisa, baqi) = bayt.split_at(fasl);

    let tarwisa: TarwisatMushtaraka =
        serde_json::from_slice(nass_tarwisa).map_err(|sabab| KhataWarsha::HuzmaTalifa {
            sabab: format!("the share header does not parse: {sabab}"),
        })?;
    if tarwisa.isdar != ISDAR_MUSHTARAKA {
        return Err(KhataWarsha::IsdarMajhul {
            wujid: tarwisa.isdar,
            madum: ISDAR_MUSHTARAKA,
        });
    }
    if tarwisa.hajm > AQSA_HAJM_JASAD {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!(
                "the header declares a {} byte body and this build reads at most {}",
                tarwisa.hajm, AQSA_HAJM_JASAD
            ),
        });
    }

    // The signature first, before a byte is decompressed. A file from a
    // stranger that fails this is not worth spending a decompressor on, and
    // the declared length is only trustworthy once somebody has vouched
    // for it.
    let Some(muallan) = bayt_hex::<32>(&tarwisa.miftah) else {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: "the header's key field is not lowercase hex".to_owned(),
        });
    };
    let Some(tawqee) = bayt_hex::<64>(&tarwisa.tawqee) else {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: "the header's signature field is not lowercase hex".to_owned(),
        });
    };
    if muallan != miftah.bayt() || !miftah.tahaqquq(&tarwisa.matn(&muallan), &tawqee) {
        return Err(KhataWarsha::TawqeeGhayrSalih);
    }

    // `split_at` left the newline at the head of the remainder.
    let madghut = baqi.get(1..).unwrap_or_default();
    let jasad =
        zstd::bulk::decompress(madghut, usize::try_from(tarwisa.hajm).unwrap_or(usize::MAX))
            .map_err(|sabab| KhataWarsha::HuzmaTalifa {
                sabab: format!("the share body does not decompress: {sabab}"),
            })?;
    if u64::try_from(jasad.len()).unwrap_or(u64::MAX) != tarwisa.hajm {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!(
                "the body is {} bytes where the header records {}",
                jasad.len(),
                tarwisa.hajm
            ),
        });
    }
    if blake3::hash(&jasad).to_hex().to_string() != tarwisa.basma {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: "the body does not match the hash the header signed".to_owned(),
        });
    }

    let qayyid = min_jasad(&jasad)?;
    if u64::try_from(qayyid.len()).unwrap_or(u64::MAX) != tarwisa.adad {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!(
                "the body holds {} entries where the header records {}",
                qayyid.len(),
                tarwisa.adad
            ),
        });
    }
    Ok(HuzmaMuwaththaqa { tarwisa, qayyid })
}

/// Counts of what an import did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqreerIstirad {
    /// Entries read from the share.
    pub zurat: u64,
    /// Entries recorded — inserted new, or folded into an existing row.
    pub sujjilat: u64,
    /// Entries the local floor rejected.
    pub marfuda: u64,
    /// Entries that fold to an unusable key and could never be looked up.
    pub talifa: u64,
}

/// Merges a verified share into a memory.
///
/// Every entry re-enters through [`Dhakira::sajjil`], so both ratchets apply:
/// a reading of a pair this machine has already reviewed leaves the review in
/// place, and a reading can never raise a pair's rank. That is the guarantee
/// that makes importing a stranger's readings survivable — the worst a bad
/// share can do is add a row that ranks below everything the importer already
/// had, and be outranked the moment anybody reviews the line.
///
/// `khiyarat` is applied **again** on the way in, and that is not redundant
/// with the exporter's floor: the exporter chose their own, the header only
/// states it, and a share whose sharer set the floor to zero is exactly the
/// share an importer wants to filter. The two are independent decisions by
/// two different people and both are enforced.
///
/// `mushahadat` is deliberately not trusted as stated. The share carries the
/// sharer's count, and a file can claim any number; the import records **one**
/// sighting per entry. So corroboration accumulates from real independent
/// imports rather than from one file asserting it, which is the difference
/// between a tie-break and a lever anybody can pull.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] when a write into the memory fails.
pub fn idmij(
    hadaf: &mut Dhakira,
    huzma: &HuzmaMuwaththaqa,
    khiyarat: KhiyaratMusharaka,
) -> NatijatWarsha<TaqreerIstirad> {
    let mut taqreer = TaqreerIstirad::default();
    let ism_luba = Some(huzma.tarwisa.ism_luba.clone());
    for qayd in &huzma.qayyid {
        taqreer.zurat = taqreer.zurat.saturating_add(1);
        if !qayd.salih() {
            taqreer.talifa = taqreer.talifa.saturating_add(1);
            continue;
        }
        if !khiyarat.yaqbal(qayd.thiqa) {
            taqreer.marfuda = taqreer.marfuda.saturating_add(1);
            continue;
        }
        hadaf
            .sajjil(&qayd.ila_qayd(huzma.tarwisa.luba, ism_luba.clone()))
            .map_err(|khata| mawrid(&khata))?;
        taqreer.sujjilat = taqreer.sujjilat.saturating_add(1);
    }
    Ok(taqreer)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The entries as the JSON Lines body, one document per line.
///
/// JSON Lines rather than one array, matching every other append-shaped file
/// in this product: a torn tail line costs one entry rather than the whole
/// document, and a body can be produced without holding a serializer open
/// over the entire set.
fn ila_jasad(qayyid: &[QaydMushtarak]) -> NatijatWarsha<Vec<u8>> {
    let mut jasad = Vec::with_capacity(qayyid.len().saturating_mul(192));
    for qayd in qayyid {
        let satr = serde_json::to_vec(qayd).map_err(|sabab| KhataWarsha::Mawrid {
            sabab: sabab.to_string(),
        })?;
        jasad.extend_from_slice(&satr);
        jasad.push(b'\n');
    }
    Ok(jasad)
}

/// The body back as entries, refusing a malformed line by number.
///
/// Refused rather than skipped, unlike a reading history: the body's hash was
/// signed, so a line that will not parse is not a torn write — it is a file
/// whose author produced something this build does not read, and dropping it
/// silently would make the imported count disagree with the signed one.
fn min_jasad(jasad: &[u8]) -> NatijatWarsha<Vec<QaydMushtarak>> {
    let mut qayyid = Vec::new();
    for (raqm, satr) in jasad.split(|harf| *harf == b'\n').enumerate() {
        if satr.is_empty() {
            continue;
        }
        let qayd: QaydMushtarak =
            serde_json::from_slice(satr).map_err(|sabab| KhataWarsha::HuzmaTalifa {
                sabab: format!("entry {} does not parse: {sabab}", raqm.saturating_add(1)),
            })?;
        qayyid.push(qayd);
    }
    Ok(qayyid)
}

/// The memory's own refusal, wrapped as this crate's.
fn mawrid(khata: &KhataTarjama) -> KhataWarsha {
    KhataWarsha::Mawrid {
        sabab: khata.injilizi(),
    }
}

/// Length-prefixed, little-endian, as `taarib_mustawda::taqyeem` does it.
fn lp(matn: &mut Vec<u8>, bayt: &[u8]) {
    matn.extend_from_slice(&u64::try_from(bayt.len()).unwrap_or(u64::MAX).to_le_bytes());
    matn.extend_from_slice(bayt);
}

fn hex(bayt: &[u8]) -> String {
    bayt.iter().fold(
        String::with_capacity(bayt.len().saturating_mul(2)),
        |mut khraj, w| {
            let _ = write!(khraj, "{w:02x}");
            khraj
        },
    )
}

/// Lowercase hex only, and exactly the declared width — the canonical form.
fn bayt_hex<const N: usize>(nass: &str) -> Option<[u8; N]> {
    if nass.len() != N.checked_mul(2)? {
        return None;
    }
    if nass
        .bytes()
        .any(|w| !w.is_ascii_hexdigit() || w.is_ascii_uppercase())
    {
        return None;
    }
    let mut khraj = [0u8; N];
    let mut azwaj = nass.as_bytes().chunks_exact(2);
    for khana in &mut khraj {
        let nassi = std::str::from_utf8(azwaj.next()?).ok()?;
        *khana = u8::from_str_radix(nassi, 16).ok()?;
    }
    Some(khraj)
}
