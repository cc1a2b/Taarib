//! النصوص البسيطة — the three line- and cell-oriented formats:
//! `XUnity.AutoTranslator`'s translation cache, a delimited table with columns
//! the caller names, and Unity Localization's own CSV export.
//!
//! ## The encoding is established, never guessed
//!
//! This module decodes every file the crate imports, including the XML ones,
//! and it does so under one rule: **UTF-8, or a byte-order mark, or a refusal.**
//!
//! - UTF-8 with a mark and UTF-8 without one are both read.
//! - UTF-16LE and UTF-16BE are read when the file carries the mark that says
//!   so, and an odd byte count or an unpaired surrogate is a refusal rather
//!   than a `U+FFFD` substituted into somebody's sentence.
//! - UTF-32 in either order is refused by name. Nothing writes translation
//!   files in it and reading two bytes of a four-byte unit produces garbage.
//! - Anything else — a Windows-1256 file from a decade-old fan patch, a
//!   Shift-JIS export, a CP-1252 spreadsheet — is **refused by name**.
//!
//! The last one is the decision worth defending, because it costs a real user
//! something. A mis-decoded Arabic file does not fail: it succeeds, and
//! produces mojibake. `Ù…Ø±Ø­Ø¨Ø§` is a perfectly valid string. It matches, it
//! imports, it lays out, it renders, and it ships — and the first person to
//! notice is a player. Every automatic detector is a statistical guess, and a
//! guess that is wrong one time in fifty across a forty-thousand-string project
//! is eight hundred corrupted sentences that no check downstream can
//! distinguish from a translation somebody meant.
//!
//! So there is no legacy decoder here and no `encoding_rs` dependency. A
//! contributor with a Windows-1256 file converts it once, outside, with a tool
//! that shows them the result — which is a step they can verify and this
//! module cannot.
//!
//! ## The three formats
//!
//! **`XUnity.AutoTranslator`** writes `original=translation`, one per line, with
//! `//` comments and two kinds of regular-expression rule mixed into the same
//! file under the `r:` and `sr:` prefixes. Those rules are parsed, marked as
//! rules, and declined from the string mapping — never applied as literal text.
//! See [`NawWarid::QaidaNamatiya`].
//!
//! **CSV** is read through the `csv` crate, so a quoted field holding an
//! embedded newline stays one field. Columns are named by index, or resolved
//! from a header when there is one and the index was left unset.
//!
//! **Unity Localization's export** carries one column per locale. The Arabic
//! column is found by its BCP-47 code; if there is no Arabic column the import
//! is refused with the columns that *are* present listed by name, because the
//! alternative — taking the second locale column, or the last one — writes
//! French into an Arabic project.

use std::path::Path;

use super::{
    HalatWarid, IstiradKhiyarat, KashfTarwisa, MilaffWarid, MudkhalMarfud, MudkhalWarid, NawWarid,
    SababRafd, SighatIstirad, TarmizMuhaddad, lugha_arabiya, martabat_arabiya, mawqi_fasil_xunity,
    ramz_lugha_min_unwan,
};
use crate::khata::KhataTarqee;

/// The UTF-8 byte-order mark.
const ALAMAT_UTF8: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The UTF-16 little-endian mark.
const ALAMAT_UTF16_SAGHIR: [u8; 2] = [0xFF, 0xFE];

/// The UTF-16 big-endian mark.
const ALAMAT_UTF16_KABIR: [u8; 2] = [0xFE, 0xFF];

/// The UTF-32 little-endian mark, which begins with the UTF-16 one and must
/// therefore be tested first.
const ALAMAT_UTF32_SAGHIR: [u8; 4] = [0xFF, 0xFE, 0x00, 0x00];

/// The UTF-32 big-endian mark.
const ALAMAT_UTF32_KABIR: [u8; 4] = [0x00, 0x00, 0xFE, 0xFF];

/// How much of a malformed line a refusal quotes.
const TUL_IQTIBAS: usize = 80;

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Turns a file's bytes into text, or refuses to.
///
/// The single decoding path for every format in this crate. A caller's
/// [`IstiradKhiyarat::tarmiz`] overrides detection — except where it
/// contradicts a byte-order mark the file actually carries, which is refused,
/// because a mark is a fact and an override is an opinion and one of the two
/// has to be wrong.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] for a UTF-32 mark, for an override that
/// contradicts the file's mark, for a UTF-16 file with an odd byte count or an
/// unpaired surrogate, and for bytes that are neither valid UTF-8 nor marked as
/// anything else — that last one naming the offset of the first byte that is
/// not UTF-8, so a contributor can look at it.
pub fn fak_tarmiz(
    masar: &Path,
    bayt: &[u8],
    khiyarat: &IstiradKhiyarat,
) -> Result<String, KhataTarqee> {
    let rafd = |sabab: String| KhataTarqee::IstiradFashil {
        masar: masar.to_path_buf(),
        sabab,
    };

    if bayt.starts_with(&ALAMAT_UTF32_SAGHIR) || bayt.starts_with(&ALAMAT_UTF32_KABIR) {
        return Err(rafd(
            "the file carries a UTF-32 byte-order mark. No translation tool writes UTF-32 and \
             reading it as UTF-16 would produce a string of nulls, so it is refused rather than \
             attempted. Re-save it as UTF-8."
                .to_owned(),
        ));
    }

    let (malhuz, jasad) = if let Some(baqi) = bayt.strip_prefix(&ALAMAT_UTF8) {
        (Some(TarmizMuhaddad::Utf8), baqi)
    } else if let Some(baqi) = bayt.strip_prefix(&ALAMAT_UTF16_SAGHIR) {
        (Some(TarmizMuhaddad::Utf16Saghir), baqi)
    } else if let Some(baqi) = bayt.strip_prefix(&ALAMAT_UTF16_KABIR) {
        (Some(TarmizMuhaddad::Utf16Kabir), baqi)
    } else {
        (None, bayt)
    };

    if let (Some(malhuz), Some(matlub)) = (malhuz, khiyarat.tarmiz)
        && malhuz != matlub
    {
        return Err(rafd(format!(
            "the file carries a {} byte-order mark and {} was demanded. One of the two is wrong \
             and decoding either way risks mojibake, so neither is attempted.",
            malhuz.ism(),
            matlub.ism()
        )));
    }

    match malhuz.or(khiyarat.tarmiz) {
        Some(TarmizMuhaddad::Utf16Saghir) => min_utf16(masar, jasad, false),
        Some(TarmizMuhaddad::Utf16Kabir) => min_utf16(masar, jasad, true),
        Some(TarmizMuhaddad::Utf8) | None => min_utf8(masar, jasad),
    }
}

/// UTF-8 bytes as text, refusing anything that is not.
fn min_utf8(masar: &Path, bayt: &[u8]) -> Result<String, KhataTarqee> {
    match std::str::from_utf8(bayt) {
        Ok(nass) => Ok(nass.trim_start_matches('\u{FEFF}').to_owned()),
        Err(khata) => Err(KhataTarqee::IstiradFashil {
            masar: masar.to_path_buf(),
            sabab: format!(
                "byte {} is not valid UTF-8 and the file carries no byte-order mark, so its \
                 encoding cannot be established. It is refused rather than guessed: a legacy \
                 encoding decoded as the wrong one produces text that looks like a translation \
                 and is not. Convert the file to UTF-8 and import it again.",
                khata.valid_up_to()
            ),
        }),
    }
}

/// UTF-16 bytes as text, in the given order.
///
/// An unpaired surrogate is a refusal and not a `U+FFFD`. A replacement
/// character in the middle of an Arabic sentence is a defect that survives
/// every check downstream and surfaces in a screenshot.
fn min_utf16(masar: &Path, bayt: &[u8], kabir: bool) -> Result<String, KhataTarqee> {
    let rafd = |sabab: String| KhataTarqee::IstiradFashil {
        masar: masar.to_path_buf(),
        sabab,
    };
    let azwaj = bayt.chunks_exact(2);
    if !azwaj.remainder().is_empty() {
        return Err(rafd(format!(
            "the file is marked UTF-16 and has {} bytes, which is not a whole number of 16-bit \
             units. It is truncated, and reading it would shift every character after the cut.",
            bayt.len()
        )));
    }

    let wahdat = azwaj.map(|zawj| {
        let awwal = zawj.first().copied().unwrap_or(0);
        let thani = zawj.get(1).copied().unwrap_or(0);
        if kabir {
            u16::from_be_bytes([awwal, thani])
        } else {
            u16::from_le_bytes([awwal, thani])
        }
    });

    let mut natija = String::with_capacity(bayt.len());
    for (mawdi, harf) in char::decode_utf16(wahdat).enumerate() {
        match harf {
            Ok(harf) => natija.push(harf),
            Err(_) => {
                return Err(rafd(format!(
                    "16-bit unit {mawdi} is an unpaired surrogate. The file is not well-formed \
                     UTF-16 and substituting a replacement character would put a visible defect \
                     into a translation that every later check would accept."
                )));
            },
        }
    }
    Ok(natija.trim_start_matches('\u{FEFF}').to_owned())
}

// ---------------------------------------------------------------------------
// XUnity.AutoTranslator
// ---------------------------------------------------------------------------

/// Reads an `XUnity.AutoTranslator` translation cache.
///
/// The grammar, in full:
///
/// | line | meaning |
/// | --- | --- |
/// | `// anything` | a comment |
/// | *(blank)* | ignored |
/// | `original=translation` | a literal pair, split at the first **unescaped** `=` |
/// | `r:"pattern"=replacement` | a regular-expression rule |
/// | `sr:"pattern"=replacement` | a splitter regular-expression rule |
///
/// Inside a value, `\n`, `\r`, `\t`, `\\` and `\=` are escapes and everything
/// else after a backslash is left as written — the tool itself round-trips
/// unknown escapes rather than erroring, and diverging from that here would
/// mean this reader and the game's own plugin disagreed about what a string is.
///
/// Placeholders in the game's own syntax pass through untouched. `{0}`, `%s`,
/// `<color=#ff0000>` and RPG Maker's `\V[3]` are all things the source game
/// interprets, and this module's only job with them is to not damage them —
/// note that the third one contains an `=` and is protected by the split being
/// at the *first* one, and the fourth by unknown escapes being preserved.
///
/// # Errors
///
/// This parser does not fail on content: a line it cannot place becomes a
/// declined entry with [`SababRafd::SatrGhayrMafhum`] and is reported. The
/// [`Result`] exists because the signature is shared with the parsers that can
/// refuse a document outright.
#[expect(
    clippy::unnecessary_wraps,
    reason = "one of the six parsers `iqra_bi_sigha` dispatches to, all of which share this \
              signature. This grammar happens to have no whole-document refusal; narrowing \
              its return type would make the dispatch heterogeneous for that accident alone."
)]
pub fn iqra_xunity(
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    tracing::debug!(masar = %masar.display(), "reading an XUnity.AutoTranslator cache");
    let mut milaff = MilaffWarid::jadeed(SighatIstirad::XUnityAutoTranslator, nass.lines().count());
    // The tool writes machine output. Whoever ran it, that is what is in the
    // file, so the ceiling is the machine state whatever the caller supplied.
    let hala = SighatIstirad::XUnityAutoTranslator
        .saqf_hala()
        .adna(khiyarat.hala_asasiya(SighatIstirad::XUnityAutoTranslator));

    for (mawdi, satr_khaam) in nass.lines().enumerate() {
        let satr = satr_khaam.trim_end_matches('\r');
        let raqm = mawdi.saturating_add(1);
        if satr.trim().is_empty() || satr.trim_start().starts_with("//") {
            continue;
        }

        if let Some(baqi) = satr.strip_prefix("sr:").or_else(|| satr.strip_prefix("r:")) {
            let mujazzi = satr.starts_with("sr:");
            match qaida_namatiya(baqi, mujazzi, raqm) {
                Some(warid) => milaff.marfuda.push(MudkhalMarfud {
                    warid,
                    sabab: SababRafd::QaidaNamatiya,
                }),
                None => milaff.marfuda.push(MudkhalMarfud {
                    warid: satr_ghayr_mafhum(satr, raqm),
                    sabab: SababRafd::SatrGhayrMafhum { juz: iqtibas(satr) },
                }),
            }
            continue;
        }

        let Some(fasl) = mawqi_fasil_xunity(satr) else {
            milaff.marfuda.push(MudkhalMarfud {
                warid: satr_ghayr_mafhum(satr, raqm),
                sabab: SababRafd::SatrGhayrMafhum { juz: iqtibas(satr) },
            });
            continue;
        };

        let masdar = fukk_hurub_xunity(satr.get(..fasl).unwrap_or_default());
        let hadaf = fukk_hurub_xunity(satr.get(fasl.saturating_add(1)..).unwrap_or_default());

        let mut warid = MudkhalWarid::jadeed(masdar, Some(hadaf.clone()), raqm);
        warid.hala = hala;
        // XUnity keys a cache by the original text; there is no separate key.
        warid.miftah = None;
        if let Some(sabab) = rafd_zahir(&warid) {
            milaff.marfuda.push(MudkhalMarfud { warid, sabab });
        } else {
            milaff.madakhil.push(warid);
        }
    }

    let qawaid = milaff
        .marfuda
        .iter()
        .filter(|marfud| matches!(marfud.sabab, SababRafd::QaidaNamatiya))
        .count();
    if qawaid > 0 {
        milaff.tanbihat.push(format!(
            "{qawaid} regular-expression rule(s) were read and listed. They are not literal \
             strings and are not mapped onto the table; reproduce them in the runtime rules."
        ));
    }
    Ok(milaff)
}

/// One `r:` or `sr:` rule, or [`None`] when the line does not have the shape.
///
/// The pattern is recorded exactly as written and **never compiled**. This
/// crate owns no regular-expression engine on purpose: a pattern evaluated by a
/// different engine than the game's plugin uses matches different text, and a
/// rule that this module decided was equivalent to a literal would be a rule
/// applied to the wrong strings.
fn qaida_namatiya(baqi: &str, mujazzi: bool, satr: usize) -> Option<MudkhalWarid> {
    let jasad = baqi.strip_prefix('"')?;
    let mut namat = String::new();
    let mut ahruf = jasad.char_indices();
    let mut nihaya = None;
    while let Some((izaha, harf)) = ahruf.next() {
        match harf {
            '\\' => {
                namat.push('\\');
                if let Some((_, talin)) = ahruf.next() {
                    namat.push(talin);
                }
            },
            '"' => {
                nihaya = Some(izaha);
                break;
            },
            _ => namat.push(harf),
        }
    }
    let nihaya = nihaya?;
    let baad = jasad.get(nihaya.saturating_add(1)..)?;
    let badil = baad.strip_prefix('=')?;

    let mut warid = MudkhalWarid::jadeed(namat.clone(), None, satr);
    warid.naw = NawWarid::QaidaNamatiya {
        namat,
        badil: fukk_hurub_xunity(badil),
        mujazzi,
    };
    warid.hala = HalatWarid::Aaliya;
    Some(warid)
}

/// The entry that stands for a line the grammar does not define.
fn satr_ghayr_mafhum(satr: &str, raqm: usize) -> MudkhalWarid {
    MudkhalWarid::jadeed(satr.to_owned(), None, raqm)
}

/// The first [`TUL_IQTIBAS`] characters of a line, for a message.
fn iqtibas(satr: &str) -> String {
    satr.chars().take(TUL_IQTIBAS).collect()
}

/// Resolves the escapes `XUnity.AutoTranslator` writes.
///
/// `\n`, `\r`, `\t`, `\\` and `\=`. **An unknown escape is preserved whole**,
/// backslash included, because the game's own placeholder dialects use
/// backslash — RPG Maker's `\V[3]` and `\C[2]` among them — and consuming the
/// backslash would silently turn a variable reference into the letter `V`.
fn fukk_hurub_xunity(khaam: &str) -> String {
    let mut natija = String::with_capacity(khaam.len());
    let mut ahruf = khaam.chars();
    while let Some(harf) = ahruf.next() {
        if harf != '\\' {
            natija.push(harf);
            continue;
        }
        match ahruf.next() {
            Some('n') => natija.push('\n'),
            Some('r') => natija.push('\r'),
            Some('t') => natija.push('\t'),
            // A trailing backslash is preserved for the same reason an unknown
            // escape is: the next read has to see the byte that was written.
            Some('\\') | None => natija.push('\\'),
            Some('=') => natija.push('='),
            Some(akhar) => {
                natija.push('\\');
                natija.push(akhar);
            },
        }
    }
    natija
}

/// The reason an otherwise well-formed pair is declined, if there is one.
///
/// Shared by all three formats in this file so that "an empty translation" and
/// "a translation identical to its source" mean the same thing in a CSV as in
/// an XUnity cache.
fn rafd_zahir(warid: &MudkhalWarid) -> Option<SababRafd> {
    match warid.hadaf.as_deref() {
        None => Some(SababRafd::BilaHadaf),
        Some("") => Some(SababRafd::HadafFarigh),
        Some(hadaf) if hadaf == warid.masdar && !warid.masdar.is_empty() => {
            Some(SababRafd::HadafKaAlmasdar)
        },
        Some(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Delimited tables
// ---------------------------------------------------------------------------

/// The header names a key column answers to.
const ASMAA_MIFTAH: [&str; 7] = [
    "key",
    "id",
    "identifier",
    "name",
    "msgctxt",
    "term",
    "string id",
];

/// The header names a source column answers to.
const ASMAA_MASDAR: [&str; 9] = [
    "source",
    "original",
    "english",
    "source text",
    "msgid",
    "text",
    "en",
    "en-us",
    "untranslated",
];

/// The header names a target column answers to.
const ASMAA_HADAF: [&str; 8] = [
    "target",
    "translation",
    "translated",
    "arabic",
    "msgstr",
    "ar",
    "ar-sa",
    "value",
];

/// The header names a comment column answers to.
const ASMAA_MULAHAZA: [&str; 6] = [
    "comment",
    "comments",
    "note",
    "notes",
    "description",
    "context",
];

/// Reads a delimited table with the columns the caller named.
///
/// Parsed through the `csv` crate rather than by splitting on the delimiter,
/// which is what makes a quoted field containing an embedded newline stay one
/// field. A hand-split reader turns a two-line dialogue cell into two half-rows
/// and reports both, and the import looks like it worked.
///
/// Records of unequal width are tolerated — a spreadsheet routinely omits
/// trailing empty cells — and a record too short for a column the caller asked
/// for becomes a declined entry naming the column and the width, rather than an
/// empty string that would read as an untranslated row.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] when the delimiter is not a single byte that
/// can be one, when the file will not parse as a table at all, or when the
/// target column can be resolved neither from the caller's index nor from a
/// header.
pub fn iqra_jadwal(
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    let rafd = |sabab: String| KhataTarqee::IstiradFashil {
        masar: masar.to_path_buf(),
        sabab,
    };
    let amida = &khiyarat.amida;
    tahaqquq_mahdid(masar, amida.mahdid)?;
    let sijillat = sijillat_csv(masar, nass, amida.mahdid)?;

    let laha_tarwisa = match amida.tarwisa {
        KashfTarwisa::Mawjuda => true,
        KashfTarwisa::Ghaiba => false,
        KashfTarwisa::Talqai => sijillat
            .first()
            .is_some_and(|(_, huqul)| tabdu_tarwisa(huqul)),
    };
    let tarwisa: Vec<String> = if laha_tarwisa {
        sijillat
            .first()
            .map(|(_, huqul)| huqul.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let miftah = amida
        .miftah
        .or_else(|| amud_bilism(&tarwisa, &ASMAA_MIFTAH));
    let masdar = amida
        .masdar
        .or_else(|| amud_bilism(&tarwisa, &ASMAA_MASDAR));
    let hadaf = amida.hadaf.or_else(|| amud_bilism(&tarwisa, &ASMAA_HADAF));
    let mulahaza = amida
        .mulahaza
        .or_else(|| amud_bilism(&tarwisa, &ASMAA_MULAHAZA));

    let Some(hadaf) = hadaf else {
        return Err(rafd(format!(
            "no target column: none was given and the header {} does not name one. Set the \
             target column index explicitly.",
            asmaa_amida(&tarwisa)
        )));
    };
    if masdar.is_none() && miftah.is_none() {
        return Err(rafd(format!(
            "neither a key column nor a source column could be resolved from the header {}, so a \
             row cannot be attached to anything in the project.",
            asmaa_amida(&tarwisa)
        )));
    }

    let mut milaff = MilaffWarid::jadeed(SighatIstirad::Csv, nass.lines().count());
    let hala = khiyarat.hala_asasiya(SighatIstirad::Csv);
    if laha_tarwisa {
        milaff
            .tanbihat
            .push(format!("header row read as {}", asmaa_amida(&tarwisa)));
    }

    for (raqm, huqul) in sijillat.iter().skip(usize::from(laha_tarwisa)) {
        if huqul.iter().all(|haql| haql.trim().is_empty()) {
            continue;
        }
        let (warid, sabab) = sajjil_min_huqul(huqul, *raqm, miftah, masdar, hadaf, mulahaza, hala);
        // A record that could not supply a column is already declined; anything
        // that did supply one still faces the checks every format shares.
        let sabab = sabab.or_else(|| rafd_zahir(&warid));
        match sabab {
            Some(sabab) => milaff.marfuda.push(MudkhalMarfud { warid, sabab }),
            None => milaff.madakhil.push(warid),
        }
    }
    Ok(milaff)
}

/// One entry out of one record, beside the reason it was declined if it was.
///
/// An entry always comes back, refusal or not, so a record that could not
/// supply a column is still reported with whatever it did hold rather than
/// disappearing into a count. Not a [`Result`]: both outcomes carry the same
/// entry, and only one of them carries a reason as well.
fn sajjil_min_huqul(
    huqul: &[String],
    raqm: usize,
    miftah: Option<usize>,
    masdar: Option<usize>,
    hadaf: usize,
    mulahaza: Option<usize>,
    hala: HalatWarid,
) -> (MudkhalWarid, Option<SababRafd>) {
    let nass_masdar = masdar.and_then(|fahras| huqul.get(fahras)).cloned();
    let nass_miftah = miftah.and_then(|fahras| huqul.get(fahras)).cloned();
    let asas = nass_masdar
        .or_else(|| nass_miftah.clone())
        .unwrap_or_default();

    let Some(nass_hadaf) = huqul.get(hadaf) else {
        let mut warid = MudkhalWarid::jadeed(asas, None, raqm);
        warid.miftah = nass_miftah;
        warid.hala = hala;
        return (
            warid,
            Some(SababRafd::AmudMafqud {
                fahras: hadaf,
                mawjud: huqul.len(),
            }),
        );
    };

    let mut warid = MudkhalWarid::jadeed(asas, Some(nass_hadaf.clone()), raqm);
    warid.miftah = nass_miftah.filter(|qeema| !qeema.trim().is_empty());
    warid.hala = hala;
    if let Some(nass_mulahaza) = mulahaza.and_then(|fahras| huqul.get(fahras))
        && !nass_mulahaza.trim().is_empty()
    {
        warid.mulahazat.push(nass_mulahaza.clone());
    }
    (warid, None)
}

/// Reads a whole delimited file into records with their line numbers.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] carrying whatever the `csv` reader said —
/// an unterminated quoted field, most often, which is the failure that would
/// otherwise swallow the rest of the file into one enormous cell.
fn sijillat_csv(
    masar: &Path,
    nass: &str,
    mahdid: u8,
) -> Result<Vec<(usize, Vec<String>)>, KhataTarqee> {
    let mut qari = csv::ReaderBuilder::new()
        .delimiter(mahdid)
        .has_headers(false)
        // Trailing empty cells are omitted by every spreadsheet that ever
        // exported a translation sheet. Refusing the file over it would refuse
        // most real files; the short-record case is reported per entry instead.
        .flexible(true)
        .double_quote(true)
        .from_reader(nass.as_bytes());

    let mut sijillat = Vec::new();
    for natija in qari.records() {
        let sijill = natija.map_err(|khata| KhataTarqee::IstiradFashil {
            masar: masar.to_path_buf(),
            sabab: format!("the file does not parse as a delimited table: {khata}"),
        })?;
        let raqm = sijill.position().map_or(0, |mawqi| {
            usize::try_from(mawqi.line()).unwrap_or(usize::MAX)
        });
        sijillat.push((raqm, sijill.iter().map(str::to_owned).collect()));
    }
    Ok(sijillat)
}

/// Checks that a delimiter can be one.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] for a quote character, a line terminator or a
/// byte outside ASCII. The `csv` reader takes a single byte, so a multi-byte
/// delimiter is not expressible; saying so by name is better than the first
/// byte of one silently becoming the delimiter.
fn tahaqquq_mahdid(masar: &Path, mahdid: u8) -> Result<(), KhataTarqee> {
    if mahdid == b'"' || mahdid == b'\r' || mahdid == b'\n' || !mahdid.is_ascii() {
        return Err(KhataTarqee::IstiradFashil {
            masar: masar.to_path_buf(),
            sabab: format!(
                "{mahdid:#04x} cannot be a delimiter: the quote character, the line terminators \
                 and any byte outside ASCII are excluded."
            ),
        });
    }
    Ok(())
}

/// Whether a record reads as a header rather than as data.
fn tabdu_tarwisa(huqul: &[String]) -> bool {
    huqul.iter().any(|haql| {
        let musawwa = haql.trim().to_lowercase();
        ASMAA_MIFTAH.contains(&musawwa.as_str())
            || ASMAA_MASDAR.contains(&musawwa.as_str())
            || ASMAA_HADAF.contains(&musawwa.as_str())
            || ASMAA_MULAHAZA.contains(&musawwa.as_str())
    })
}

/// The index of the first header cell matching one of a set of names.
fn amud_bilism(tarwisa: &[String], asmaa: &[&str]) -> Option<usize> {
    tarwisa.iter().position(|haql| {
        let musawwa = haql.trim().to_lowercase();
        asmaa.contains(&musawwa.as_str())
    })
}

/// A header row as a readable list, for a message.
fn asmaa_amida(tarwisa: &[String]) -> String {
    if tarwisa.is_empty() {
        return "(no header row)".to_owned();
    }
    let asmaa: Vec<String> = tarwisa
        .iter()
        .map(|haql| format!("{:?}", haql.trim()))
        .collect();
    format!("[{}]", asmaa.join(", "))
}

// ---------------------------------------------------------------------------
// Unity Localization's CSV export
// ---------------------------------------------------------------------------

/// One locale column of a Unity Localization export.
#[derive(Debug, Clone)]
struct AmudLugha {
    /// Where it is.
    fahras: usize,
    /// The BCP-47 code out of its heading.
    ramz: String,
    /// The heading exactly as written, for a message.
    unwan: String,
    /// Whether it is the locale's comment column rather than its text column.
    mulahaza: bool,
}

/// Reads Unity Localization's own CSV export.
///
/// The shape Unity writes is `Key`, optionally `Id` and `Shared Comments`, then
/// one column per locale — headed either `Arabic(ar)`, `Arabic (ar-SA)` or the
/// bare code — and optionally a `… Comments` column beside each locale.
///
/// ## The Arabic column is found by its code, or the import is refused
///
/// There is no positional fallback, no "the last column", no "the second
/// locale". A project whose export happens to be `Key,English(en),French(fr)`
/// would take French under any of those rules, and French written into an
/// Arabic project is a defect that reads as a translation all the way to a
/// screenshot.
///
/// When several Arabic columns exist — `ar`, `ar-SA` and `ar-EG` in one export
/// is ordinary — the one matching [`IstiradKhiyarat::ramz_lugha`] wins, then a
/// bare `ar`, then the first in column order. **Which one was taken and which
/// ones existed both go into the result's remarks.** Picking silently is how a
/// project ends up with Egyptian Arabic in half its menus and nobody able to
/// say when it happened.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] when the file has no header, when the first
/// column is not `Key`, or when no column is Arabic — the last one listing
/// every column heading the file does have.
pub fn iqra_wahdat_tawteen(
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    let rafd = |sabab: String| KhataTarqee::IstiradFashil {
        masar: masar.to_path_buf(),
        sabab,
    };
    tahaqquq_mahdid(masar, khiyarat.amida.mahdid)?;
    let sijillat = sijillat_csv(masar, nass, khiyarat.amida.mahdid)?;

    let Some((_, tarwisa)) = sijillat.first() else {
        return Err(rafd(
            "the file is empty. A Unity Localization export always begins with a header row."
                .to_owned(),
        ));
    };
    let awwal = tarwisa
        .first()
        .map(|haql| haql.trim().to_owned())
        .unwrap_or_default();
    if !awwal.eq_ignore_ascii_case("key") {
        return Err(rafd(format!(
            "the first column is {awwal:?} and a Unity Localization export begins with \"Key\". \
             The header is {}. Import it as a plain table with explicit column indices instead.",
            asmaa_amida(tarwisa)
        )));
    }

    let aamida = aamidat_lugha(tarwisa);
    let arabiya = ikhtar_arabiya(&aamida, &khiyarat.ramz_lugha);
    let Some(arabiya) = arabiya else {
        return Err(rafd(format!(
            "no Arabic locale column. The columns present are {}. Nothing here is imported \
             rather than taking a column that is not Arabic.",
            asmaa_amida(tarwisa)
        )));
    };

    let mut milaff = MilaffWarid::jadeed(SighatIstirad::UnityLocalizationCsv, nass.lines().count());
    milaff.lugha_hadaf = Some(arabiya.ramz.clone());

    let ukhra: Vec<String> = aamida
        .iter()
        .filter(|amud| !amud.mulahaza && amud.fahras != arabiya.fahras && lugha_arabiya(&amud.ramz))
        .map(|amud| amud.unwan.clone())
        .collect();
    if ukhra.is_empty() {
        milaff
            .tanbihat
            .push(format!("Arabic taken from the column {:?}", arabiya.unwan));
    } else {
        milaff.tanbihat.push(format!(
            "Arabic taken from the column {:?}; the file also carries {}, which were not \
             imported",
            arabiya.unwan,
            ukhra.join(", ")
        ));
    }

    let masdar = amud_masdar(&aamida, arabiya.fahras);
    match masdar.as_ref() {
        Some(amud) => {
            milaff.lugha_masdar = Some(amud.ramz.clone());
            milaff.tanbihat.push(format!(
                "source text taken from the column {:?}",
                amud.unwan
            ));
        },
        None => milaff
            .tanbihat
            .push("no non-Arabic locale column, so rows are matched by their key alone".to_owned()),
    }

    let mushtaraka = amud_bilism(tarwisa, &["shared comments"]);
    let mulahaza_arabiya = aamida
        .iter()
        .find(|amud| amud.mulahaza && amud.ramz.eq_ignore_ascii_case(&arabiya.ramz))
        .map(|amud| amud.fahras);
    let hala = khiyarat.hala_asasiya(SighatIstirad::UnityLocalizationCsv);

    for (raqm, huqul) in sijillat.iter().skip(1) {
        if huqul.iter().all(|haql| haql.trim().is_empty()) {
            continue;
        }
        let miftah = huqul.first().cloned().unwrap_or_default();
        let nass_masdar = masdar
            .as_ref()
            .and_then(|amud| huqul.get(amud.fahras))
            .cloned()
            .unwrap_or_else(|| miftah.clone());

        let Some(nass_hadaf) = huqul.get(arabiya.fahras) else {
            let mut warid = MudkhalWarid::jadeed(nass_masdar, None, *raqm);
            warid.miftah = Some(miftah);
            warid.hala = hala;
            milaff.marfuda.push(MudkhalMarfud {
                warid,
                sabab: SababRafd::AmudMafqud {
                    fahras: arabiya.fahras,
                    mawjud: huqul.len(),
                },
            });
            continue;
        };

        let mut warid = MudkhalWarid::jadeed(nass_masdar, Some(nass_hadaf.clone()), *raqm);
        warid.miftah = Some(miftah).filter(|qeema| !qeema.trim().is_empty());
        warid.hala = hala;
        for fahras in [mushtaraka, mulahaza_arabiya].into_iter().flatten() {
            if let Some(nass_mulahaza) = huqul.get(fahras)
                && !nass_mulahaza.trim().is_empty()
            {
                warid.mulahazat.push(nass_mulahaza.clone());
            }
        }
        match rafd_zahir(&warid) {
            Some(sabab) => milaff.marfuda.push(MudkhalMarfud { warid, sabab }),
            None => milaff.madakhil.push(warid),
        }
    }
    Ok(milaff)
}

/// Every column of a header that names a locale.
///
/// Column zero is skipped unconditionally: it is `Key`, and a project whose
/// keys happen to be two-letter words should not have its key column read as a
/// locale.
fn aamidat_lugha(tarwisa: &[String]) -> Vec<AmudLugha> {
    let mut aamida = Vec::new();
    for (fahras, unwan) in tarwisa.iter().enumerate().skip(1) {
        let mahdhub = unwan.trim();
        let musawwa = mahdhub.to_lowercase();
        let (asas, mulahaza) = match musawwa.strip_suffix(" comments") {
            Some(_) => (
                mahdhub
                    .get(..mahdhub.len().saturating_sub(" comments".len()))
                    .unwrap_or(mahdhub),
                true,
            ),
            None => (mahdhub, false),
        };
        if let Some(ramz) = ramz_lugha_min_unwan(asas) {
            aamida.push(AmudLugha {
                fahras,
                ramz,
                unwan: mahdhub.to_owned(),
                mulahaza,
            });
        }
    }
    aamida
}

/// The Arabic text column to take, by preference then by column order.
fn ikhtar_arabiya(aamida: &[AmudLugha], mufaddal: &str) -> Option<AmudLugha> {
    aamida
        .iter()
        .filter(|amud| !amud.mulahaza)
        .filter_map(|amud| martabat_arabiya(&amud.ramz, mufaddal).map(|martaba| (martaba, amud)))
        .min_by_key(|(martaba, amud)| (*martaba, amud.fahras))
        .map(|(_, amud)| amud.clone())
}

/// The column to read source text from.
///
/// English by preference, because that is what a Unity project's source locale
/// almost always is and what a translator reads beside the Arabic; otherwise
/// the first non-Arabic locale column, in column order.
fn amud_masdar(aamida: &[AmudLugha], mustathna: usize) -> Option<AmudLugha> {
    let mutah = || {
        aamida.iter().filter(move |amud| {
            !amud.mulahaza && amud.fahras != mustathna && !lugha_arabiya(&amud.ramz)
        })
    };
    mutah()
        .find(|amud| {
            let awwal = amud.ramz.split(['-', '_']).next().unwrap_or_default();
            awwal.eq_ignore_ascii_case("en")
        })
        .or_else(|| mutah().min_by_key(|amud| amud.fahras))
        .cloned()
}
