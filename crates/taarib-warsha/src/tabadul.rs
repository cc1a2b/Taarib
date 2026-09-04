//! Interchange export that must survive Phase 14's own importer before it is handed to anyone.

use std::collections::BTreeMap;
use std::path::Path;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::MudkhalNass;
use taarib_tarqee::mustawrid::{
    self, HalatWarid, IstiradKhiyarat, MudkhalWarid, SababRafd, SighatIstirad,
};

use crate::khata::{KhataWarsha, NatijatWarsha};

/// The `original` attribute of an exported `<file>`, which re-imports as the entries' context.
pub const WASM_MALAF: &str = "Taarib";

/// The `Plural-Forms` header an exported catalogue declares: Arabic's six forms,
/// in the exact expression the importer recognises as canonical.
pub const SIGHAT_JAMA_PO: &str = "nplurals=6; plural=(n==0 ? 0 : n==1 ? 1 : n==2 ? 2 : \
                                  n%100>=3 && n%100<=10 ? 3 : n%100>=11 ? 4 : 5);";

/// The fingerprint the verification import is attributed to.
///
/// The verification result is compared and discarded — nothing is applied — but
/// the importer's state ceiling depends on whether a contributor is named, and
/// without one every state collapses to machine output and the state check
/// proves nothing.
const BASMAT_TASDIQ: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// How many characters of a source text a message quotes.
const TUL_IQTIBAS: usize = 48;

// ---------------------------------------------------------------------------
// The formats
// ---------------------------------------------------------------------------

/// An interchange format this build writes.
///
/// Six variants, each backed by a serializer and proved by re-import: the
/// dispatch in [`saddir_wa_athbit`] has no wildcard arm, so a variant added
/// without a serializer does not compile. Unity Localization's CSV is absent
/// deliberately — this module exports the plain table instead, and a format
/// with no variant here is a format that cannot be half-written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SighatTabadul {
    /// XLIFF 1.2 — `<trans-unit>` with `<source>` and `<target>`.
    Xliff12,
    /// XLIFF 2.0 — `<unit>` and `<segment>`, a different grammar entirely.
    Xliff20,
    /// gettext PO.
    GettextPo,
    /// TMX 1.4 translation memory.
    Tmx,
    /// A comma-delimited table in the importer's default column layout:
    /// key, source, target.
    Csv,
    /// `XUnity.AutoTranslator`'s `original=translation` plain text.
    XUnityAutoTranslator,
}

impl SighatTabadul {
    /// The importer-side format every export of this format is proved against.
    #[must_use]
    pub const fn sighat_istirad(self) -> SighatIstirad {
        match self {
            Self::Xliff12 => SighatIstirad::Xliff12,
            Self::Xliff20 => SighatIstirad::Xliff20,
            Self::GettextPo => SighatIstirad::GettextPo,
            Self::Tmx => SighatIstirad::Tmx,
            Self::Csv => SighatIstirad::Csv,
            Self::XUnityAutoTranslator => SighatIstirad::XUnityAutoTranslator,
        }
    }

    /// The format's name, as its own specification spells it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        self.sighat_istirad().ism()
    }

    /// The sentence the export screen shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        self.sighat_istirad().wasf_arabi()
    }

    /// The extension an export of this format is usually saved with.
    #[must_use]
    pub const fn imtidad(self) -> &'static str {
        match self {
            Self::Xliff12 | Self::Xliff20 => "xlf",
            Self::GettextPo => "po",
            Self::Tmx => "tmx",
            Self::Csv => "csv",
            Self::XUnityAutoTranslator => "txt",
        }
    }

    /// Whether the format can carry a row that has no translation yet.
    ///
    /// An XLIFF target can be empty and stated, a PO `msgstr` can be empty and
    /// a CSV cell can be blank. A translation memory unit and an XUnity cache
    /// line cannot say "not yet" — an empty right-hand side there is a defect,
    /// not a status — so those rows are skipped and counted instead.
    #[must_use]
    pub const fn tusaddir_bila_tarjama(self) -> bool {
        match self {
            Self::Xliff12 | Self::Xliff20 | Self::GettextPo | Self::Csv => true,
            Self::Tmx | Self::XUnityAutoTranslator => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Options and report
// ---------------------------------------------------------------------------

/// What a caller may change about an export.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KhiyaratTasdir {
    /// The source language declared in the file, as BCP-47. Must not be Arabic:
    /// the TMX importer would read the source segment as the translation.
    pub lugha_masdar: String,
    /// The target language declared in the file, as BCP-47. Must be Arabic:
    /// the PO and TMX importers refuse anything else by name.
    pub lugha_hadaf: String,
}

impl Default for KhiyaratTasdir {
    fn default() -> Self {
        Self { lugha_masdar: "en".to_owned(), lugha_hadaf: "ar".to_owned() }
    }
}

/// Why a row was left out of an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SababTakhatti {
    /// The row has no translation and the format has no way to say so.
    BilaTarjama,
    /// The escaped source would start an `XUnity.AutoTranslator` line with
    /// `//`, which its reader treats as a comment.
    YabdaKaTaaliq,
    /// The escaped source would start an `XUnity.AutoTranslator` line with
    /// `r:` or `sr:`, which its reader treats as a regular-expression rule.
    YabdaKaQaidaNamatiya,
    /// The row has an empty source and no context, and gettext reserves the
    /// empty `msgid` for the catalogue header.
    MasdarFarighPo,
}

impl SababTakhatti {
    /// The sentence the export report shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::BilaTarjama => "الصفّ بلا ترجمة، والصيغة لا تعبّر عن ذلك.",
            Self::YabdaKaTaaliq => "النص الأصلي يبدأ بما يقرؤه XUnity تعليقًا (//).",
            Self::YabdaKaQaidaNamatiya => {
                "النص الأصلي يبدأ بما يقرؤه XUnity قاعدة تعبير نمطي (r: أو sr:)."
            }
            Self::MasdarFarighPo => {
                "النص الأصلي فارغ بلا سياق، وgettext يحجز msgid الفارغ للترويسة."
            }
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::BilaTarjama => "the row has no translation and the format cannot say so",
            Self::YabdaKaTaaliq => {
                "the escaped source would start the line with //, which XUnity reads as a comment"
            }
            Self::YabdaKaQaidaNamatiya => {
                "the escaped source would start the line with r: or sr:, which XUnity reads as a \
                 regular-expression rule"
            }
            Self::MasdarFarighPo => {
                "the source is empty with no context, and gettext reserves the empty msgid for \
                 the header"
            }
        }
    }
}

/// One row an export left out, named.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QaydTakhatti {
    /// The row's short identity and the start of its source text.
    pub wasf: String,
    /// Why it was left out.
    pub sabab: SababTakhatti,
}

/// What one export wrote, and what it deliberately did not.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqreerTasdir {
    /// Which format was written.
    pub sigha: SighatTabadul,
    /// How many rows were offered.
    pub sufuf: usize,
    /// How many rows were written with a translation.
    pub musaddara: usize,
    /// How many rows were written as explicitly untranslated.
    pub bila_tarjama: usize,
    /// Every row that was left out, each with its reason.
    pub mutakhatta: Vec<QaydTakhatti>,
}

impl TaqreerTasdir {
    /// An empty report for a format and a row count.
    #[must_use]
    pub const fn jadeed(sigha: SighatTabadul, sufuf: usize) -> Self {
        Self { sigha, sufuf, musaddara: 0, bila_tarjama: 0, mutakhatta: Vec::new() }
    }

    /// How many rows the report accounts for, written or not.
    #[must_use]
    pub const fn adad_muhasab(&self) -> usize {
        self.musaddara.saturating_add(self.bila_tarjama).saturating_add(self.mutakhatta.len())
    }

    /// Whether every offered row is accounted for.
    #[must_use]
    pub const fn muhasaba_mutawazina(&self) -> bool {
        self.adad_muhasab() == self.sufuf
    }

    /// The sentence the export screen shows afterwards.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} of {} row(s) written as translations to {}, {} written as untranslated, {} left \
             out and listed by reason; the file was re-imported and compared before it was \
             handed over",
            self.musaddara,
            self.sufuf,
            self.sigha.ism(),
            self.bila_tarjama,
            self.mutakhatta.len()
        )
    }
}

/// An export that has already round-tripped through its own importer.
///
/// The only way this type is produced is [`saddir_wa_athbit`], after the bytes
/// were parsed back and every record compared, so holding one is holding the
/// proof.
#[derive(Debug, Clone)]
pub struct HasilatTasdir {
    /// The serialized file, UTF-8.
    pub bayt: Vec<u8>,
    /// What was written and what was left out.
    pub taqreer: TaqreerTasdir,
}

// ---------------------------------------------------------------------------
// The state mapping, and its inverse through the importer
// ---------------------------------------------------------------------------

/// The XLIFF 1.2 `state` and `state-qualifier` written for a review state.
const fn halat_nuskha_ula(hala: HalatMuraja) -> (&'static str, Option<&'static str>) {
    match hala {
        HalatMuraja::LamTutarjam => ("needs-translation", None),
        HalatMuraja::TarjamaAaliya => ("translated", Some("mt-suggestion")),
        HalatMuraja::Musawwada => ("translated", None),
        HalatMuraja::LilMuraja | HalatMuraja::Marfuda => ("needs-review-translation", None),
        HalatMuraja::Muakkada => ("final", None),
    }
}

/// The XLIFF 2.0 `state` and `subState` written for a review state.
const fn halat_nuskha_thaniya(hala: HalatMuraja) -> (&'static str, Option<&'static str>) {
    match hala {
        HalatMuraja::TarjamaAaliya => ("translated", Some("taarib:mt")),
        HalatMuraja::Musawwada => ("translated", None),
        // 2.0 has no needs-review state; `initial` with a target present is the
        // one value the importer maps back to review, so an untranslated string
        // and one awaiting review are the same pair on the wire and are told
        // apart by whether a target is written beside them.
        HalatMuraja::LamTutarjam | HalatMuraja::LilMuraja => ("initial", None),
        HalatMuraja::Marfuda => ("initial", Some("taarib:rejected")),
        HalatMuraja::Muakkada => ("final", None),
    }
}

/// Whether a review state is written as gettext's `#, fuzzy` flag.
const fn alam_taswid_po(hala: HalatMuraja) -> bool {
    matches!(
        hala,
        HalatMuraja::TarjamaAaliya | HalatMuraja::LilMuraja | HalatMuraja::Marfuda
    )
}

/// The state and unconfirmed flag every exported state must come back as.
///
/// The importer lands everything on the draft side — approval is a human's
/// attestation, not an attribute — so this table is the importer's own mapping
/// applied to what [`halat_nuskha_ula`], [`halat_nuskha_thaniya`] and
/// [`alam_taswid_po`] write, under the draft ceiling an attributed import has.
/// The round-trip proof asserts this value, not identity.
const fn hala_mutawaqqaa(sigha: SighatTabadul, hala: HalatMuraja) -> (HalatWarid, bool) {
    match sigha {
        SighatTabadul::Xliff12 => match hala {
            HalatMuraja::TarjamaAaliya => (HalatWarid::Aaliya, false),
            HalatMuraja::LilMuraja | HalatMuraja::Marfuda => (HalatWarid::LilMuraja, false),
            HalatMuraja::LamTutarjam | HalatMuraja::Musawwada | HalatMuraja::Muakkada => {
                (HalatWarid::Musawwada, false)
            }
        },
        SighatTabadul::Xliff20 => match hala {
            HalatMuraja::TarjamaAaliya => (HalatWarid::Aaliya, false),
            HalatMuraja::LamTutarjam | HalatMuraja::LilMuraja | HalatMuraja::Marfuda => {
                (HalatWarid::LilMuraja, false)
            }
            HalatMuraja::Musawwada | HalatMuraja::Muakkada => (HalatWarid::Musawwada, false),
        },
        SighatTabadul::GettextPo => (HalatWarid::Musawwada, alam_taswid_po(hala)),
        SighatTabadul::Tmx | SighatTabadul::Csv => (HalatWarid::Musawwada, false),
        SighatTabadul::XUnityAutoTranslator => (HalatWarid::Aaliya, false),
    }
}

// ---------------------------------------------------------------------------
// What each written record must come back as
// ---------------------------------------------------------------------------

/// A declined outcome the export predicts, matched by kind against the
/// importer's [`SababRafd`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SababMutawaqqa {
    /// The importer declines an empty translation.
    HadafFarigh,
    /// The importer declines a translation identical to its source.
    HadafKaAlmasdar,
    /// The importer declines the PO header pseudo-entry as metadata.
    Taalim,
}

impl SababMutawaqqa {
    /// Whether the importer's reason is the one this record predicted.
    const fn yutabiq(self, sabab: &SababRafd) -> bool {
        matches!(
            (self, sabab),
            (Self::HadafFarigh, SababRafd::HadafFarigh)
                | (Self::HadafKaAlmasdar, SababRafd::HadafKaAlmasdar)
                | (Self::Taalim, SababRafd::Taalim)
        )
    }

    /// The predicted reason's name, for a mismatch message.
    const fn ism(self) -> &'static str {
        match self {
            Self::HadafFarigh => "an empty translation",
            Self::HadafKaAlmasdar => "a translation identical to its source",
            Self::Taalim => "the catalogue's own metadata entry",
        }
    }
}

/// What one written record must come back as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tawaqqu {
    /// Parsed into the entry list, with this state and unconfirmed flag.
    Yuqbal {
        /// The state per [`hala_mutawaqqaa`].
        hala: HalatWarid,
        /// The unconfirmed flag per the format's own vocabulary.
        muallam: bool,
    },
    /// Read completely and declined for exactly this reason.
    Yurfad(SababMutawaqqa),
}

/// One record exactly as it was written, kept for the round-trip comparison.
#[derive(Debug, Clone)]
struct QaydTasdir {
    /// The key the importer must hand back.
    miftah: Option<String>,
    /// The context the importer must hand back.
    siyaq: Option<String>,
    /// The source text, byte for byte.
    masdar: String,
    /// The target text, byte for byte.
    hadaf: Option<String>,
    /// The outcome the importer must produce.
    tawaqqu: Tawaqqu,
}

/// The identity a record and a re-imported entry are matched on.
type MiftahMuqarana = (Option<String>, Option<String>, String, Option<String>);

/// A record's matching identity.
fn miftah_qayd(qayd: &QaydTasdir) -> MiftahMuqarana {
    (qayd.miftah.clone(), qayd.siyaq.clone(), qayd.masdar.clone(), qayd.hadaf.clone())
}

/// A re-imported entry's matching identity.
fn miftah_warid(warid: &MudkhalWarid) -> MiftahMuqarana {
    (warid.miftah.clone(), warid.siyaq.clone(), warid.masdar.clone(), warid.hadaf.clone())
}

// ---------------------------------------------------------------------------
// Row helpers
// ---------------------------------------------------------------------------

/// The row's translation, with an empty string counting as none.
fn hadaf_faal(saff: &MudkhalNass) -> Option<&str> {
    saff.hadaf.as_deref().filter(|nass| !nass.is_empty())
}

/// The row's short identity and the start of its source, for a message.
fn wasf_saff(saff: &MudkhalNass) -> String {
    let bidaya: String = saff.masdar.chars().take(TUL_IQTIBAS).collect();
    format!("{} ({bidaya})", saff.id.mukhtasar())
}

/// A written record's label, for a message.
fn wasf_qayd(qayd: &QaydTasdir) -> String {
    match &qayd.miftah {
        Some(miftah) => miftah.clone(),
        None => qayd.masdar.chars().take(TUL_IQTIBAS).collect(),
    }
}

/// The context string a row exports: container and location joined.
fn siyaq_saff(saff: &MudkhalNass) -> Option<String> {
    let hawiya = saff.siyaq.hawiya.trim();
    let mawqi = saff.siyaq.mawqi.trim();
    match (hawiya.is_empty(), mawqi.is_empty()) {
        (true, true) => None,
        (false, true) => Some(hawiya.to_owned()),
        (true, false) => Some(mawqi.to_owned()),
        (false, false) => Some(format!("{hawiya}/{mawqi}")),
    }
}

/// The outcome a translated record is expected to produce on re-import.
///
/// The importer declines a target byte-identical to a non-empty source as a
/// passthrough, so that pair is predicted declined rather than exported blind.
fn tawaqqu_tarjama(sigha: SighatTabadul, saff: &MudkhalNass, hadaf: &str) -> Tawaqqu {
    if hadaf == saff.masdar && !saff.masdar.is_empty() {
        return Tawaqqu::Yurfad(SababMutawaqqa::HadafKaAlmasdar);
    }
    let (hala, muallam) = hala_mutawaqqaa(sigha, saff.muraja.hala());
    Tawaqqu::Yuqbal { hala, muallam }
}

// ---------------------------------------------------------------------------
// Escaping, each the exact inverse of its importer
// ---------------------------------------------------------------------------

/// Escapes a value the way `XUnity.AutoTranslator`'s reader unescapes it.
fn hurub_xunity(matn: &str) -> String {
    let mut natija = String::with_capacity(matn.len());
    for harf in matn.chars() {
        match harf {
            '\\' => natija.push_str("\\\\"),
            '=' => natija.push_str("\\="),
            '\n' => natija.push_str("\\n"),
            '\r' => natija.push_str("\\r"),
            '\t' => natija.push_str("\\t"),
            _ => natija.push(harf),
        }
    }
    natija
}

/// Escapes a quoted PO run the way the importer's C-unescaper resolves it.
fn hurub_po(matn: &str) -> String {
    use std::fmt::Write as _;

    let mut natija = String::with_capacity(matn.len());
    for harf in matn.chars() {
        match harf {
            '"' => natija.push_str("\\\""),
            '\\' => natija.push_str("\\\\"),
            '\n' => natija.push_str("\\n"),
            '\t' => natija.push_str("\\t"),
            '\r' => natija.push_str("\\r"),
            '\u{7}' => natija.push_str("\\a"),
            '\u{8}' => natija.push_str("\\b"),
            '\u{B}' => natija.push_str("\\v"),
            '\u{C}' => natija.push_str("\\f"),
            // Three octal digits always, so a digit after the escape cannot extend it.
            harf if u32::from(harf) < 0x20 => {
                let _ = write!(natija, "\\{:03o}", u32::from(harf));
            }
            harf => natija.push(harf),
        }
    }
    natija
}

/// One CSV field, quoted exactly when the `csv` reader would need it to be.
fn haql_csv(haql: &str) -> String {
    if !haql.contains([',', '"', '\n', '\r']) {
        return haql.to_owned();
    }
    let mut natija = String::with_capacity(haql.len().saturating_add(2));
    natija.push('"');
    for harf in haql.chars() {
        if harf == '"' {
            natija.push('"');
        }
        natija.push(harf);
    }
    natija.push('"');
    natija
}

/// Writes one PO field, in gettext's multi-line continuation form when the
/// text spans lines.
fn uktub_nass_po(nass: &mut String, kalima: &str, matn: &str) {
    if matn.contains('\n') {
        nass.push_str(kalima);
        nass.push_str(" \"\"\n");
        for juz in matn.split_inclusive('\n') {
            nass.push('"');
            nass.push_str(&hurub_po(juz));
            nass.push_str("\"\n");
        }
        return;
    }
    nass.push_str(kalima);
    nass.push_str(" \"");
    nass.push_str(&hurub_po(matn));
    nass.push_str("\"\n");
}

// ---------------------------------------------------------------------------
// The plain-text serializers
// ---------------------------------------------------------------------------

/// Serializes an `XUnity.AutoTranslator` cache.
fn saddir_xunity(
    madakhil: &[MudkhalNass],
    taqreer: &mut TaqreerTasdir,
) -> (String, Vec<QaydTasdir>) {
    let mut nass = String::from("// Taarib — XUnity.AutoTranslator export\n");
    let mut quyud = Vec::with_capacity(madakhil.len());
    for saff in madakhil {
        let Some(hadaf) = hadaf_faal(saff) else {
            taqreer
                .mutakhatta
                .push(QaydTakhatti { wasf: wasf_saff(saff), sabab: SababTakhatti::BilaTarjama });
            continue;
        };
        let masdar = hurub_xunity(&saff.masdar);
        if masdar.trim_start().starts_with("//") {
            taqreer
                .mutakhatta
                .push(QaydTakhatti { wasf: wasf_saff(saff), sabab: SababTakhatti::YabdaKaTaaliq });
            continue;
        }
        if masdar.starts_with("r:") || masdar.starts_with("sr:") {
            taqreer.mutakhatta.push(QaydTakhatti {
                wasf: wasf_saff(saff),
                sabab: SababTakhatti::YabdaKaQaidaNamatiya,
            });
            continue;
        }
        nass.push_str(&masdar);
        nass.push('=');
        nass.push_str(&hurub_xunity(hadaf));
        nass.push('\n');
        taqreer.musaddara = taqreer.musaddara.saturating_add(1);
        quyud.push(QaydTasdir {
            miftah: None,
            siyaq: None,
            masdar: saff.masdar.clone(),
            hadaf: Some(hadaf.to_owned()),
            tawaqqu: tawaqqu_tarjama(SighatTabadul::XUnityAutoTranslator, saff, hadaf),
        });
    }
    (nass, quyud)
}

/// Serializes the plain comma-delimited table the importer's default column
/// layout reads: key, source, target, with a header row.
fn saddir_csv(madakhil: &[MudkhalNass], taqreer: &mut TaqreerTasdir) -> (String, Vec<QaydTasdir>) {
    let mut nass = String::from("key,source,target\n");
    let mut quyud = Vec::with_capacity(madakhil.len());
    for saff in madakhil {
        let miftah = saff.id.to_string();
        let hadaf = hadaf_faal(saff);
        nass.push_str(&haql_csv(&miftah));
        nass.push(',');
        nass.push_str(&haql_csv(&saff.masdar));
        nass.push(',');
        nass.push_str(&haql_csv(hadaf.unwrap_or_default()));
        nass.push('\n');
        let tawaqqu = if let Some(matn) = hadaf {
            taqreer.musaddara = taqreer.musaddara.saturating_add(1);
            tawaqqu_tarjama(SighatTabadul::Csv, saff, matn)
        } else {
            taqreer.bila_tarjama = taqreer.bila_tarjama.saturating_add(1);
            Tawaqqu::Yurfad(SababMutawaqqa::HadafFarigh)
        };
        quyud.push(QaydTasdir {
            miftah: Some(miftah),
            siyaq: None,
            masdar: saff.masdar.clone(),
            hadaf: Some(hadaf.unwrap_or_default().to_owned()),
            tawaqqu,
        });
    }
    (nass, quyud)
}

/// Serializes a gettext PO catalogue.
fn saddir_po(
    madakhil: &[MudkhalNass],
    khiyarat: &KhiyaratTasdir,
    taqreer: &mut TaqreerTasdir,
) -> (String, Vec<QaydTasdir>) {
    let tarwisa = format!(
        "Project-Id-Version: Taarib\nMIME-Version: 1.0\nContent-Type: text/plain; \
         charset=UTF-8\nContent-Transfer-Encoding: 8bit\nLanguage: {}\nPlural-Forms: \
         {SIGHAT_JAMA_PO}\n",
        khiyarat.lugha_hadaf
    );
    let mut nass = String::new();
    uktub_nass_po(&mut nass, "msgid", "");
    uktub_nass_po(&mut nass, "msgstr", &tarwisa);
    let mut quyud = Vec::with_capacity(madakhil.len().saturating_add(1));
    quyud.push(QaydTasdir {
        miftah: None,
        siyaq: None,
        masdar: String::new(),
        hadaf: Some(tarwisa),
        tawaqqu: Tawaqqu::Yurfad(SababMutawaqqa::Taalim),
    });

    for saff in madakhil {
        let siyaq = siyaq_saff(saff);
        if saff.masdar.is_empty() && siyaq.is_none() {
            taqreer.mutakhatta.push(QaydTakhatti {
                wasf: wasf_saff(saff),
                sabab: SababTakhatti::MasdarFarighPo,
            });
            continue;
        }
        let hadaf = hadaf_faal(saff);
        nass.push('\n');
        if hadaf.is_some() && alam_taswid_po(saff.muraja.hala()) {
            nass.push_str("#, fuzzy\n");
        }
        if let Some(matn_siyaq) = siyaq.as_deref() {
            uktub_nass_po(&mut nass, "msgctxt", matn_siyaq);
        }
        uktub_nass_po(&mut nass, "msgid", &saff.masdar);
        uktub_nass_po(&mut nass, "msgstr", hadaf.unwrap_or_default());
        let tawaqqu = if let Some(matn) = hadaf {
            taqreer.musaddara = taqreer.musaddara.saturating_add(1);
            tawaqqu_tarjama(SighatTabadul::GettextPo, saff, matn)
        } else {
            taqreer.bila_tarjama = taqreer.bila_tarjama.saturating_add(1);
            Tawaqqu::Yurfad(SababMutawaqqa::HadafFarigh)
        };
        quyud.push(QaydTasdir {
            miftah: Some(saff.masdar.clone()).filter(|matn| !matn.is_empty()),
            siyaq,
            masdar: saff.masdar.clone(),
            hadaf: Some(hadaf.unwrap_or_default().to_owned()),
            tawaqqu,
        });
    }
    (nass, quyud)
}

// ---------------------------------------------------------------------------
// The XML serializers
// ---------------------------------------------------------------------------

/// The XML writer's refusal, which a write into memory can still report.
fn khata_katib(khata: &std::io::Error) -> KhataWarsha {
    KhataWarsha::Mawrid { sabab: format!("the XML writer refused an event: {khata}") }
}

/// Writes one event.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] carrying whatever the writer said.
fn uktub(katib: &mut Writer<Vec<u8>>, hadath: Event<'_>) -> NatijatWarsha<()> {
    katib.write_event(hadath).map_err(|khata| khata_katib(&khata))
}

/// One element holding exactly one text run, escaped by the writer.
///
/// # Errors
///
/// As [`uktub`].
fn unsur_nassi(katib: &mut Writer<Vec<u8>>, ism: &str, matn: &str) -> NatijatWarsha<()> {
    uktub(katib, Event::Start(BytesStart::new(ism)))?;
    uktub(katib, Event::Text(BytesText::new(matn)))?;
    uktub(katib, Event::End(BytesEnd::new(ism)))
}

/// The finished document as UTF-8 text.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] when the buffer is not UTF-8, which escaped writes
/// of UTF-8 input do not produce.
fn nass_katib(katib: Writer<Vec<u8>>) -> NatijatWarsha<String> {
    String::from_utf8(katib.into_inner()).map_err(|khata| KhataWarsha::Mawrid {
        sabab: format!("the XML writer produced bytes that are not UTF-8: {khata}"),
    })
}

/// A writer indented for humans; text runs stay unindented, so no whitespace
/// enters a `<source>` or a `<target>`.
fn katib_jadeed() -> Writer<Vec<u8>> {
    Writer::new_with_indent(Vec::new(), b' ', 2)
}

/// Serializes an XLIFF 1.2 document.
///
/// # Errors
///
/// As [`uktub`].
fn saddir_xliff_ula(
    madakhil: &[MudkhalNass],
    khiyarat: &KhiyaratTasdir,
    taqreer: &mut TaqreerTasdir,
) -> NatijatWarsha<(String, Vec<QaydTasdir>)> {
    let mut katib = katib_jadeed();
    uktub(&mut katib, Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    let mut jidhr = BytesStart::new("xliff");
    jidhr.push_attribute(("version", "1.2"));
    jidhr.push_attribute(("xmlns", "urn:oasis:names:tc:xliff:document:1.2"));
    uktub(&mut katib, Event::Start(jidhr))?;
    let mut malaf = BytesStart::new("file");
    malaf.push_attribute(("original", WASM_MALAF));
    malaf.push_attribute(("source-language", khiyarat.lugha_masdar.as_str()));
    malaf.push_attribute(("target-language", khiyarat.lugha_hadaf.as_str()));
    malaf.push_attribute(("datatype", "plaintext"));
    uktub(&mut katib, Event::Start(malaf))?;
    uktub(&mut katib, Event::Start(BytesStart::new("body")))?;

    let mut quyud = Vec::with_capacity(madakhil.len());
    for saff in madakhil {
        let miftah = saff.id.to_string();
        let hadaf = hadaf_faal(saff);
        let mut wahda = BytesStart::new("trans-unit");
        wahda.push_attribute(("id", miftah.as_str()));
        if hadaf.is_some() && saff.muraja.hala() == HalatMuraja::Muakkada {
            wahda.push_attribute(("approved", "yes"));
        }
        uktub(&mut katib, Event::Start(wahda))?;
        unsur_nassi(&mut katib, "source", &saff.masdar)?;
        let tawaqqu = if let Some(matn) = hadaf {
            let (hala, muhaddid) = halat_nuskha_ula(saff.muraja.hala());
            let mut alhadaf = BytesStart::new("target");
            alhadaf.push_attribute(("state", hala));
            if let Some(qeema) = muhaddid {
                alhadaf.push_attribute(("state-qualifier", qeema));
            }
            uktub(&mut katib, Event::Start(alhadaf))?;
            uktub(&mut katib, Event::Text(BytesText::new(matn)))?;
            uktub(&mut katib, Event::End(BytesEnd::new("target")))?;
            taqreer.musaddara = taqreer.musaddara.saturating_add(1);
            tawaqqu_tarjama(SighatTabadul::Xliff12, saff, matn)
        } else {
            let (hala, _) = halat_nuskha_ula(HalatMuraja::LamTutarjam);
            let mut alhadaf = BytesStart::new("target");
            alhadaf.push_attribute(("state", hala));
            uktub(&mut katib, Event::Empty(alhadaf))?;
            taqreer.bila_tarjama = taqreer.bila_tarjama.saturating_add(1);
            Tawaqqu::Yurfad(SababMutawaqqa::HadafFarigh)
        };
        uktub(&mut katib, Event::End(BytesEnd::new("trans-unit")))?;
        quyud.push(QaydTasdir {
            miftah: Some(miftah),
            siyaq: Some(WASM_MALAF.to_owned()),
            masdar: saff.masdar.clone(),
            hadaf: Some(hadaf.unwrap_or_default().to_owned()),
            tawaqqu,
        });
    }

    uktub(&mut katib, Event::End(BytesEnd::new("body")))?;
    uktub(&mut katib, Event::End(BytesEnd::new("file")))?;
    uktub(&mut katib, Event::End(BytesEnd::new("xliff")))?;
    Ok((nass_katib(katib)?, quyud))
}

/// Serializes an XLIFF 2.0 document.
///
/// # Errors
///
/// As [`uktub`].
fn saddir_xliff_thaniya(
    madakhil: &[MudkhalNass],
    khiyarat: &KhiyaratTasdir,
    taqreer: &mut TaqreerTasdir,
) -> NatijatWarsha<(String, Vec<QaydTasdir>)> {
    let mut katib = katib_jadeed();
    uktub(&mut katib, Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    let mut jidhr = BytesStart::new("xliff");
    jidhr.push_attribute(("version", "2.0"));
    jidhr.push_attribute(("xmlns", "urn:oasis:names:tc:xliff:document:2.0"));
    jidhr.push_attribute(("srcLang", khiyarat.lugha_masdar.as_str()));
    jidhr.push_attribute(("trgLang", khiyarat.lugha_hadaf.as_str()));
    uktub(&mut katib, Event::Start(jidhr))?;
    let mut malaf = BytesStart::new("file");
    malaf.push_attribute(("id", WASM_MALAF));
    uktub(&mut katib, Event::Start(malaf))?;

    let mut quyud = Vec::with_capacity(madakhil.len());
    for saff in madakhil {
        let miftah = saff.id.to_string();
        let hadaf = hadaf_faal(saff);
        let mut wahda = BytesStart::new("unit");
        wahda.push_attribute(("id", miftah.as_str()));
        uktub(&mut katib, Event::Start(wahda))?;
        let (hala, fariya) = match hadaf {
            Some(_) => halat_nuskha_thaniya(saff.muraja.hala()),
            None => halat_nuskha_thaniya(HalatMuraja::LamTutarjam),
        };
        let mut maqta = BytesStart::new("segment");
        maqta.push_attribute(("state", hala));
        if let Some(qeema) = fariya {
            maqta.push_attribute(("subState", qeema));
        }
        uktub(&mut katib, Event::Start(maqta))?;
        unsur_nassi(&mut katib, "source", &saff.masdar)?;
        let tawaqqu = if let Some(matn) = hadaf {
            unsur_nassi(&mut katib, "target", matn)?;
            taqreer.musaddara = taqreer.musaddara.saturating_add(1);
            tawaqqu_tarjama(SighatTabadul::Xliff20, saff, matn)
        } else {
            uktub(&mut katib, Event::Empty(BytesStart::new("target")))?;
            taqreer.bila_tarjama = taqreer.bila_tarjama.saturating_add(1);
            Tawaqqu::Yurfad(SababMutawaqqa::HadafFarigh)
        };
        uktub(&mut katib, Event::End(BytesEnd::new("segment")))?;
        uktub(&mut katib, Event::End(BytesEnd::new("unit")))?;
        quyud.push(QaydTasdir {
            miftah: Some(miftah),
            siyaq: Some(WASM_MALAF.to_owned()),
            masdar: saff.masdar.clone(),
            hadaf: Some(hadaf.unwrap_or_default().to_owned()),
            tawaqqu,
        });
    }

    uktub(&mut katib, Event::End(BytesEnd::new("file")))?;
    uktub(&mut katib, Event::End(BytesEnd::new("xliff")))?;
    Ok((nass_katib(katib)?, quyud))
}

/// Serializes a TMX 1.4 translation memory.
///
/// # Errors
///
/// As [`uktub`].
fn saddir_tmx(
    madakhil: &[MudkhalNass],
    khiyarat: &KhiyaratTasdir,
    taqreer: &mut TaqreerTasdir,
) -> NatijatWarsha<(String, Vec<QaydTasdir>)> {
    let mut katib = katib_jadeed();
    uktub(&mut katib, Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    let mut jidhr = BytesStart::new("tmx");
    jidhr.push_attribute(("version", "1.4"));
    uktub(&mut katib, Event::Start(jidhr))?;
    let mut tarwisa = BytesStart::new("header");
    tarwisa.push_attribute(("creationtool", WASM_MALAF));
    tarwisa.push_attribute(("creationtoolversion", "19"));
    tarwisa.push_attribute(("segtype", "sentence"));
    tarwisa.push_attribute(("o-tmf", WASM_MALAF));
    tarwisa.push_attribute(("adminlang", "en"));
    tarwisa.push_attribute(("srclang", khiyarat.lugha_masdar.as_str()));
    tarwisa.push_attribute(("datatype", "plaintext"));
    uktub(&mut katib, Event::Empty(tarwisa))?;
    uktub(&mut katib, Event::Start(BytesStart::new("body")))?;

    let mut quyud = Vec::with_capacity(madakhil.len());
    for saff in madakhil {
        let Some(hadaf) = hadaf_faal(saff) else {
            taqreer
                .mutakhatta
                .push(QaydTakhatti { wasf: wasf_saff(saff), sabab: SababTakhatti::BilaTarjama });
            continue;
        };
        let miftah = saff.id.to_string();
        let mut wahda = BytesStart::new("tu");
        wahda.push_attribute(("tuid", miftah.as_str()));
        uktub(&mut katib, Event::Start(wahda))?;
        for (ramz, matn) in [
            (khiyarat.lugha_masdar.as_str(), saff.masdar.as_str()),
            (khiyarat.lugha_hadaf.as_str(), hadaf),
        ] {
            let mut juz = BytesStart::new("tuv");
            juz.push_attribute(("xml:lang", ramz));
            uktub(&mut katib, Event::Start(juz))?;
            unsur_nassi(&mut katib, "seg", matn)?;
            uktub(&mut katib, Event::End(BytesEnd::new("tuv")))?;
        }
        uktub(&mut katib, Event::End(BytesEnd::new("tu")))?;
        taqreer.musaddara = taqreer.musaddara.saturating_add(1);
        quyud.push(QaydTasdir {
            miftah: Some(miftah),
            siyaq: None,
            masdar: saff.masdar.clone(),
            hadaf: Some(hadaf.to_owned()),
            tawaqqu: tawaqqu_tarjama(SighatTabadul::Tmx, saff, hadaf),
        });
    }

    uktub(&mut katib, Event::End(BytesEnd::new("body")))?;
    uktub(&mut katib, Event::End(BytesEnd::new("tmx")))?;
    Ok((nass_katib(katib)?, quyud))
}

// ---------------------------------------------------------------------------
// The round-trip proof
// ---------------------------------------------------------------------------

/// Feeds an export back through Phase 14's importer and compares every record.
///
/// The import is attributed to a fixed synthetic identity so the state ceiling
/// is the draft side rather than machine output; the result is compared and
/// discarded, and nothing is ever applied from it.
///
/// # Errors
///
/// A sentence naming the first entry that was lost, changed, or produced from
/// nowhere.
fn athbit(
    sigha: SighatTabadul,
    nass: &str,
    quyud: &[QaydTasdir],
    khiyarat: &KhiyaratTasdir,
) -> Result<(), String> {
    let musahim = MusahimId::jadeed(BASMAT_TASDIQ)
        .map_err(|khata| format!("the verification identity could not be built: {khata}"))?;
    let istirad = IstiradKhiyarat {
        ramz_lugha: khiyarat.lugha_hadaf.clone(),
        musahim: Some(musahim),
        ..IstiradKhiyarat::default()
    };
    let milaff = mustawrid::iqra_bi_sigha(
        sigha.sighat_istirad(),
        Path::new("taarib-tabadul"),
        nass,
        &istirad,
    )
    .map_err(|khata| format!("the importer refused the exported document: {khata}"))?;

    let mut baqiya: BTreeMap<MiftahMuqarana, Vec<&QaydTasdir>> = BTreeMap::new();
    for qayd in quyud {
        baqiya.entry(miftah_qayd(qayd)).or_default().push(qayd);
    }

    for warid in &milaff.madakhil {
        let Some(qaima) = baqiya.get_mut(&miftah_warid(warid)) else {
            return Err(format!(
                "the importer produced an entry the export never wrote, or its text changed in \
                 flight: {}",
                warid.wasf()
            ));
        };
        let tamm = qaima.iter().position(|qayd| {
            matches!(qayd.tawaqqu, Tawaqqu::Yuqbal { hala, muallam }
                if hala == warid.hala && muallam == warid.muallam)
        });
        if let Some(mawdi) = tamm {
            let _ = qaima.remove(mawdi);
            continue;
        }
        let badil = qaima.iter().find_map(|qayd| match qayd.tawaqqu {
            Tawaqqu::Yuqbal { hala, muallam } => Some((hala, muallam)),
            Tawaqqu::Yurfad(_) => None,
        });
        return Err(match badil {
            Some((hala, muallam)) => format!(
                "the entry {} came back in state {:?} (unconfirmed: {}) where the inverse \
                 mapping expects {:?} (unconfirmed: {})",
                warid.wasf(),
                warid.hala,
                warid.muallam,
                hala,
                muallam
            ),
            None => format!(
                "the entry {} was accepted by the importer where the export expected it declined",
                warid.wasf()
            ),
        });
    }

    for marfud in &milaff.marfuda {
        let warid = &marfud.warid;
        let Some(qaima) = baqiya.get_mut(&miftah_warid(warid)) else {
            return Err(format!(
                "the importer declined an entry the export never wrote ({}): {}",
                warid.wasf(),
                marfud.sabab.wasf_injilizi()
            ));
        };
        let tamm = qaima.iter().position(|qayd| {
            matches!(qayd.tawaqqu, Tawaqqu::Yurfad(sabab) if sabab.yutabiq(&marfud.sabab))
        });
        if let Some(mawdi) = tamm {
            let _ = qaima.remove(mawdi);
            continue;
        }
        let badil = qaima.iter().find_map(|qayd| match qayd.tawaqqu {
            Tawaqqu::Yurfad(sabab) => Some(sabab),
            Tawaqqu::Yuqbal { .. } => None,
        });
        return Err(match badil {
            Some(sabab) => format!(
                "the entry {} was declined because {} where {} was predicted",
                warid.wasf(),
                marfud.sabab.wasf_injilizi(),
                sabab.ism()
            ),
            None => format!(
                "the entry {} was declined ({}) where the export expected it accepted",
                warid.wasf(),
                marfud.sabab.wasf_injilizi()
            ),
        });
    }

    for qaima in baqiya.values() {
        if let Some(qayd) = qaima.first() {
            return Err(format!(
                "the entry {} never came back from the importer",
                wasf_qayd(qayd)
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The public entry
// ---------------------------------------------------------------------------

/// Refuses language options the importers would refuse or mis-read.
///
/// # Errors
///
/// [`KhataWarsha::TabadulGhayrMutabiq`] naming the offending language.
fn tahaqquq_lughat(sigha: SighatTabadul, khiyarat: &KhiyaratTasdir) -> NatijatWarsha<()> {
    if !mustawrid::lugha_arabiya(&khiyarat.lugha_hadaf) {
        return Err(KhataWarsha::TabadulGhayrMutabiq {
            sigha: sigha.ism(),
            sabab: format!(
                "the declared target language {:?} is not Arabic; the PO and TMX importers \
                 refuse such a file, so nothing was written",
                khiyarat.lugha_hadaf
            ),
        });
    }
    if mustawrid::lugha_arabiya(&khiyarat.lugha_masdar) {
        return Err(KhataWarsha::TabadulGhayrMutabiq {
            sigha: sigha.ism(),
            sabab: format!(
                "the declared source language {:?} is Arabic; the TMX importer would take the \
                 source segment as the translation, so nothing was written",
                khiyarat.lugha_masdar
            ),
        });
    }
    Ok(())
}

/// Serializes a string table to an interchange format and proves it round-trips.
///
/// The bytes go back through Phase 14's own importer, and are handed over only
/// when every record came back with its key, context, source, target and mapped
/// state intact.
///
/// # Errors
///
/// [`KhataWarsha::TabadulGhayrMutabiq`] when the declared languages make the
/// export unreadable by its own importer, or when any record failed to survive
/// the round trip — the sentence names the first one, and the bytes are not
/// returned. [`KhataWarsha::Mawrid`] when the XML writer itself refuses, which
/// a write into memory does not do.
pub fn saddir_wa_athbit(
    sigha: SighatTabadul,
    madakhil: &[MudkhalNass],
    khiyarat: &KhiyaratTasdir,
) -> NatijatWarsha<HasilatTasdir> {
    tahaqquq_lughat(sigha, khiyarat)?;
    let mut taqreer = TaqreerTasdir::jadeed(sigha, madakhil.len());
    let (nass, quyud) = match sigha {
        SighatTabadul::Xliff12 => saddir_xliff_ula(madakhil, khiyarat, &mut taqreer)?,
        SighatTabadul::Xliff20 => saddir_xliff_thaniya(madakhil, khiyarat, &mut taqreer)?,
        SighatTabadul::Tmx => saddir_tmx(madakhil, khiyarat, &mut taqreer)?,
        SighatTabadul::GettextPo => saddir_po(madakhil, khiyarat, &mut taqreer),
        SighatTabadul::Csv => saddir_csv(madakhil, &mut taqreer),
        SighatTabadul::XUnityAutoTranslator => saddir_xunity(madakhil, &mut taqreer),
    };
    athbit(sigha, &nass, &quyud, khiyarat)
        .map_err(|sabab| KhataWarsha::TabadulGhayrMutabiq { sigha: sigha.ism(), sabab })?;
    tracing::debug!(
        sigha = sigha.ism(),
        musaddara = taqreer.musaddara,
        bila_tarjama = taqreer.bila_tarjama,
        mutakhatta = taqreer.mutakhatta.len(),
        "interchange export round-tripped through its own importer"
    );
    Ok(HasilatTasdir { bayt: nass.into_bytes(), taqreer })
}

/// Serializes, proves the round trip, and only then writes the file.
///
/// # Errors
///
/// As [`saddir_wa_athbit`], and [`KhataWarsha::KhataMalaf`] when the proved
/// bytes cannot be written to `masar`.
pub fn saddir_ila_malaf(
    sigha: SighatTabadul,
    madakhil: &[MudkhalNass],
    khiyarat: &KhiyaratTasdir,
    masar: &Path,
) -> NatijatWarsha<TaqreerTasdir> {
    let hasila = saddir_wa_athbit(sigha, madakhil, khiyarat)?;
    std::fs::write(masar, &hasila.bayt).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar: masar.to_path_buf(),
        amal: "writing a proved interchange export",
        sabab,
    })?;
    Ok(hasila.taqreer)
}
