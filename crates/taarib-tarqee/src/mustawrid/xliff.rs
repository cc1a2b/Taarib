//! ملفات XLIFF — versions 1.2 and 2.0, which share a name and nothing else.
//!
//! Two grammars, two parsers. 1.2 nests `<trans-unit>` inside `<body>` inside
//! `<file>` and carries its translation state on the `<target>`; 2.0 nests
//! `<segment>` inside `<unit>` inside `<group>` inside `<file>`, moved the
//! state onto the segment, replaced the inline vocabulary wholesale and added a
//! `subState` for a tool's own vocabulary. A single parser branching on the
//! version would be two parsers sharing a name, which is what the specification
//! already is and is not a reason to repeat it here.
//!
//! ## Inline placeholder tags are preserved, never flattened
//!
//! `<ph id="1"/>` is the player's name. `<x id="2"/>` is the item count.
//! `<g id="3">` opens the colour the sentence is drawn in. A reader that drops
//! them to get "the text" produces a string that reads perfectly and has lost
//! the thing the sentence was about — and the loss is invisible in every
//! review, because what is left is grammatical.
//!
//! So the content of `<source>` and `<target>` is reconstructed with its inline
//! elements written back into the string exactly as they appeared, attributes
//! and all. The result is a string that still carries `<ph id="1"/>`, which is
//! then handed to this workspace's one markup parser on the way into the table,
//! the same way an extracted string is.
//!
//! On top of that, the **standalone** placeholders — `<x/>`, `<ph/>`, `<bx/>`,
//! `<ex/>` in 1.2, `<ph/>`, `<sc/>`, `<ec/>` in 2.0 — are collected from both
//! sides and compared. A target that lost one is declined with
//! [`SababRafd::DharratMafquda`], on exactly the reasoning
//! `taarib_tarjama::hima`'s header gives: a lost placeholder ships, renders,
//! and takes down the game's own formatter in front of a player. Paired style
//! tags going missing is a formatting defect and is recorded as a note, not a
//! refusal.
//!
//! ## `state="final"` is a draft here, and that is not a bug
//!
//! | XLIFF 1.2 `state` | here | | XLIFF 2.0 `state` | here |
//! | --- | --- | --- | --- | --- |
//! | `new`, `needs-translation` | declined when there is no target | | `initial` with a target | needs review |
//! | `needs-adaptation`, `needs-l10n` | draft | | `initial` with no target | declined |
//! | `needs-review-translation`, `needs-review-adaptation`, `needs-review-l10n` | needs review | | `translated`, `reviewed` | draft |
//! | `translated` | draft | | `final` | **draft** |
//! | `signed-off`, `final` | **draft** | | `subState` naming `mt` | machine |
//! | `state-qualifier="mt-suggestion"` | machine | | `subState` naming `fuzzy` | proposal |
//! | `state-qualifier="fuzzy-match"` | proposal | | | |
//!
//! `final` and `signed-off` mean another tool's reviewer signed the string off.
//! That is a real fact about the file and it is not this project's review.
//! Taarib's [`taarib_mustalahat::muraja::HalatMuraja::Muakkada`] means *a named
//! contributor read this Arabic sentence and is putting their name on it in a
//! published patch*, and it is reachable only by consuming an attestation that
//! has no public constructor. There is no mapping from an attribute to that,
//! and this module could not express one if it wanted to: every state it
//! produces is a [`HalatWarid`], which has no approved variant.
//!
//! `approved="yes"` on a 1.2 `<trans-unit>` is read and recorded in the entry's
//! raw state string, for the report, and changes nothing.
//!
//! ## External DTDs and entity declarations are refused by name
//!
//! Any `<!DOCTYPE …>` fails the import, whether it references an external
//! subset or declares entities internally. This is the classic XXE shape: a
//! document that declares `<!ENTITY xxe SYSTEM "file:///etc/passwd">` and
//! expands it inside a `<target>` turns a translation importer into a file
//! reader, and a document declaring a nest of entities that expand into each
//! other turns it into a memory exhaustion. Neither is a translation file, no
//! interchange format needs a DTD, and refusing the construct outright is
//! cheaper to be sure of than refusing the ways it can be abused.
//!
//! The *use* of an entity is refused as well as its declaration. Anything
//! beyond `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` and a numeric character
//! reference fails the document by name, in a string scan run before the parser
//! starts. Refusing both halves is deliberate: with only the declaration check,
//! what happens to a reference nothing declared would be whatever the XML
//! library of the day does with it, and a library that reports it as a separate
//! event rather than as an error would turn a refusal into a silently dropped
//! fragment of somebody's sentence the next time this workspace bumps a
//! version.
//!
//! An `<?xml encoding="…"?>` declaration naming anything but a Unicode encoding
//! is refused for the same reason [`super::nusus_basita`] refuses an unmarked
//! non-UTF-8 file: the bytes were decoded before the declaration was read, and
//! a document that says it is Windows-1256 has already been read as something
//! else.

use std::collections::BTreeMap;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use super::{
    FahrasAstur, HalatWarid, IstiradKhiyarat, MilaffWarid, MudkhalMarfud, MudkhalWarid, SababRafd,
    SighatIstirad,
};
use crate::khata::KhataTarqee;

/// The reader every XML importer in this crate builds.
pub(crate) type QariXml<'a> = Reader<&'a [u8]>;

/// The standalone inline elements of XLIFF 1.2.
///
/// `<g>` and `<mrk>` are deliberately absent: they wrap translatable text and
/// are paired, so losing one is a formatting defect rather than a deleted
/// value.
const DHARRAT_NUSKHA_ULA: [&str; 4] = ["x", "ph", "bx", "ex"];

/// The standalone inline elements of XLIFF 2.0.
///
/// `<pc>` is absent for the same reason `<g>` is above.
const DHARRAT_NUSKHA_THANIYA: [&str; 3] = ["ph", "sc", "ec"];

/// The encodings an XML declaration may name.
const TARMIZAT_MASMUHA: [&str; 5] = ["utf-8", "utf8", "utf-16", "utf-16le", "utf-16be"];

// ---------------------------------------------------------------------------
// Shared XML machinery
// ---------------------------------------------------------------------------
//
// TMX's inline vocabulary is XLIFF's with different element names and the same
// shape, so one reconstruction routine serves both and `po_tmx` uses these.
// A second copy would be a second opinion about what "preserved" means, and the
// two would diverge the first time either learned a new element.

/// A reader configured the way every importer here needs it.
///
/// `trim_text` is **off**. Leading and trailing whitespace inside a `<source>`
/// is part of the string — a menu label written `" Continue "` is padded on
/// purpose in more games than one would like — and trimming it here would
/// change the text before anything could compare it.
pub(crate) fn qari_jadeed(nass: &str) -> QariXml<'_> {
    let mut qari = Reader::from_str(nass);
    let idad = qari.config_mut();
    idad.trim_text(false);
    idad.expand_empty_elements = false;
    idad.check_end_names = true;
    qari
}

/// An element's local name, lowercased and without its namespace prefix.
pub(crate) fn ism_mahalli(marka: &BytesStart<'_>) -> String {
    marka.local_name().as_ref().to_ascii_lowercase()
}

/// One attribute by local name, with entity references resolved.
///
/// Local name rather than the full one, so `xml:lang` and a bare `lang` are the
/// same attribute — TMX 1.1 wrote the second and 1.4 writes the first, and a
/// reader that knew only one of them would silently find no Arabic in half the
/// translation memories in circulation.
pub(crate) fn sifa(marka: &BytesStart<'_>, matlub: &str) -> Option<String> {
    for natija in marka.attributes() {
        let Ok(attr) = natija else { continue };
        if !attr.key.local_name().as_ref().eq_ignore_ascii_case(matlub) {
            continue;
        }
        return Some(attr.normalized_value(XmlVersion::Implicit1_0).map_or_else(
            |_| attr.value.as_ref().to_owned(),
            std::borrow::Cow::into_owned,
        ));
    }
    None
}

/// An element written back exactly as it was read.
///
/// Attribute values are copied in their **escaped** form, straight out of the
/// document, rather than unescaped and re-escaped. A round trip through an
/// escaper is a round trip through somebody's opinion about which characters
/// need escaping, and the point of preserving an inline tag is that it comes
/// out the way it went in.
pub(crate) fn marka_kamila(marka: &BytesStart<'_>, farigha: bool) -> String {
    let mut nass = String::from("<");
    nass.push_str(marka.name().as_ref());
    for natija in marka.attributes() {
        let Ok(attr) = natija else { continue };
        nass.push(' ');
        nass.push_str(attr.key.as_ref());
        nass.push_str("=\"");
        nass.push_str(attr.value.as_ref());
        nass.push('"');
    }
    if farigha {
        nass.push_str("/>");
    } else {
        nass.push('>');
    }
    nass
}

/// What one element's content came to.
#[derive(Debug, Clone, Default)]
pub(crate) struct MuhtawaDakhili {
    /// The text with every inline element written back into it.
    pub(crate) nass: String,
    /// The standalone placeholders found, in document order, each identified by
    /// its own serialized start tag so that two `<ph>` elements with different
    /// ids are two different atoms.
    pub(crate) dharrat: Vec<String>,
    /// The paired style elements found, for a note when one goes missing.
    pub(crate) azwaj: Vec<String>,
}

/// Reads an element's content up to its matching end tag.
///
/// `ism` is the element being closed, `dharrat` the local names that count as
/// standalone placeholders, and everything else inside is written back into the
/// string verbatim.
///
/// # Errors
///
/// A sentence naming what went wrong: a document type declaration, an entity
/// that could not be resolved, an unexpected end of document, or whatever the
/// XML reader itself refused.
pub(crate) fn iqra_muhtawa(
    qari: &mut QariXml<'_>,
    ism: &str,
    dharrat: &[&str],
) -> Result<MuhtawaDakhili, String> {
    let mut muhtawa = MuhtawaDakhili::default();
    let mut umq = 0_usize;
    loop {
        match qari.read_event() {
            Ok(Event::Start(marka)) => {
                let dakhili = ism_mahalli(&marka);
                if dakhili == ism {
                    umq = umq.saturating_add(1);
                }
                if dharrat.contains(&dakhili.as_str()) {
                    muhtawa.dharrat.push(marka_kamila(&marka, true));
                } else if dakhili != ism {
                    muhtawa.azwaj.push(marka_kamila(&marka, false));
                }
                muhtawa.nass.push_str(&marka_kamila(&marka, false));
            },
            Ok(Event::Empty(marka)) => {
                let dakhili = ism_mahalli(&marka);
                if dharrat.contains(&dakhili.as_str()) {
                    muhtawa.dharrat.push(marka_kamila(&marka, true));
                }
                muhtawa.nass.push_str(&marka_kamila(&marka, true));
            },
            Ok(Event::End(nihaya)) => {
                let dakhili = nihaya.local_name().as_ref().to_ascii_lowercase();
                if dakhili == ism {
                    if umq == 0 {
                        return Ok(muhtawa);
                    }
                    umq = umq.saturating_sub(1);
                }
                muhtawa.nass.push_str("</");
                muhtawa.nass.push_str(nihaya.name().as_ref());
                muhtawa.nass.push('>');
            },
            Ok(Event::Text(matn)) => {
                muhtawa.nass.push_str(&matn.xml10_content());
            },
            Ok(Event::GeneralRef(marja)) => {
                let Some(maqru) = taarib_usus::kayanat::hall_marja(&marja) else {
                    let maktub = taarib_usus::kayanat::nass_marja(&marja);
                    return Err(format!(
                        "the entity reference {maktub} inside <{ism}> could not be resolved. \
                         Nothing is substituted for it: an unresolvable entity in a translation \
                         file is either a damaged document or an attempt to have this importer \
                         read something for the author."
                    ));
                };
                muhtawa.nass.push_str(&maqru);
            },
            Ok(Event::CData(matn)) => {
                muhtawa.nass.push_str(matn.into_inner().as_ref());
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => {
                return Err(format!(
                    "the document ended inside <{ism}>, which never closed"
                ));
            },
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }
}

/// Skips an element and everything under it.
///
/// Used for the subtrees an importer deliberately does not read: 1.2's
/// `<alt-trans>` alternatives, 2.0's `<mtc:matches>` candidate list, a
/// `<bin-unit>`'s binary payload. Skipping is not dropping — the caller records
/// that it skipped and why.
///
/// # Errors
///
/// As [`iqra_muhtawa`].
pub(crate) fn tajawaz(qari: &mut QariXml<'_>, ism: &str) -> Result<(), String> {
    let mut umq = 0_usize;
    loop {
        match qari.read_event() {
            Ok(Event::Start(marka)) if ism_mahalli(&marka) == ism => {
                umq = umq.saturating_add(1);
            },
            Ok(Event::End(nihaya)) => {
                let dakhili = nihaya.local_name().as_ref().to_ascii_lowercase();
                if dakhili == ism {
                    if umq == 0 {
                        return Ok(());
                    }
                    umq = umq.saturating_sub(1);
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => {
                return Err(format!(
                    "the document ended inside <{ism}>, which never closed"
                ));
            },
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }
}

/// The refusal a document type declaration earns.
pub(crate) fn rafd_doctype() -> String {
    "the document carries a <!DOCTYPE …> declaration and is refused. No interchange format this \
     build reads needs one, and the construct is how an XML parser is turned into a file reader \
     — an entity declared SYSTEM \"file:///…\" and expanded inside a <target> would put the \
     contents of that file into a translation. The declaration is refused whole rather than \
     inspected, because being sure a DTD is harmless is harder than doing without it."
        .to_owned()
}

/// Checks an XML declaration's encoding against what was actually decoded.
///
/// # Errors
///
/// A sentence naming the declared encoding. The bytes were decoded before this
/// could be read, so a declaration naming a legacy encoding means the text in
/// hand is already wrong — and the failure is silent, because a
/// mis-decoded document is still well-formed XML.
pub(crate) fn tahaqquq_tarmiz_xml(muallan: &str) -> Result<(), String> {
    let musawwa = muallan.trim().to_ascii_lowercase();
    if TARMIZAT_MASMUHA.contains(&musawwa.as_str()) {
        return Ok(());
    }
    Err(format!(
        "the XML declaration says encoding=\"{muallan}\", which is not a Unicode encoding. The \
         bytes were decoded as UTF-8 or by the file's byte-order mark before this line could be \
         read, so the text in hand is already the wrong text — and it is still well-formed XML, \
         which is why this is refused rather than noticed later."
    ))
}

/// The two checks every XML document in this crate passes before it is parsed.
///
/// The encoding declaration against what was actually decoded, and every entity
/// reference against the five XML defines for itself. Both are string scans over
/// the whole document, run once, before an element is read — a check that runs
/// during parsing has already let the parser act on the thing it was checking.
///
/// # Errors
///
/// A sentence naming the declared encoding, or the offending entity reference.
pub(crate) fn tahaqquq_wathiqa(nass: &str) -> Result<(), String> {
    if let Some(muallan) = tarmiz_muallan(nass) {
        tahaqquq_tarmiz_xml(&muallan)?;
    }
    tahaqquq_maraji_kayanat(nass)
}

/// The encoding an XML declaration names, read straight out of the text.
///
/// Scanned rather than taken off the reader's declaration event, because the
/// check has to happen before a single element is read and because a string
/// scan of `<?xml … ?>` is the one part of this file that cannot be changed out
/// from under it by an XML library's API moving.
pub(crate) fn tarmiz_muallan(nass: &str) -> Option<String> {
    let bidaya = nass.find("<?xml")?;
    let baad = nass.get(bidaya..)?;
    let nihaya = baad.find("?>")?;
    let bayan = baad.get(..nihaya)?;
    let mawqi = bayan.find("encoding")?;
    let qeema = bayan
        .get(mawqi.saturating_add("encoding".len())..)?
        .trim_start();
    let jasad = qeema.strip_prefix('=')?.trim_start();
    let iqtibas = jasad.chars().next()?;
    if iqtibas != '"' && iqtibas != '\'' {
        return None;
    }
    let dakhil = jasad.get(1..)?;
    let tul = dakhil.find(iqtibas)?;
    Some(dakhil.get(..tul)?.to_owned())
}

/// Refuses any entity reference that is not one of XML's five predefined ones
/// or a numeric character reference.
///
/// Run on the raw text before the parser sees it, and it is the second half of
/// the XXE refusal. The first half rejects `<!DOCTYPE …>`, which is the only
/// place an entity can be *declared*; this one rejects the *use* of anything
/// beyond `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` and `&#…;`.
///
/// Belt and braces on purpose. Without the second check, a reference to an
/// entity that was never declared would depend on how the XML library of the
/// day chooses to report it — an error in one release, a separate event in the
/// next — and a version bump could turn a refusal into a silently dropped
/// fragment of somebody's sentence. A string scan cannot drift that way.
///
/// # Errors
///
/// A sentence naming the reference that was found.
pub(crate) fn tahaqquq_maraji_kayanat(nass: &str) -> Result<(), String> {
    const MAARUFA: [&str; 5] = ["amp", "lt", "gt", "quot", "apos"];
    let mut baqi = nass;
    while let Some(mawqi) = baqi.find('&') {
        let baad = baqi.get(mawqi.saturating_add(1)..).unwrap_or_default();
        let tul = baad
            .char_indices()
            .take(64)
            .find(|(_, harf)| {
                !harf.is_ascii_alphanumeric() && !matches!(harf, '#' | '.' | '-' | '_')
            })
            .map_or(baad.len(), |(izaha, _)| izaha);
        let ism = baad.get(..tul).unwrap_or_default();
        let munhi = baad.get(tul..).unwrap_or_default().starts_with(';');
        if munhi && !ism.is_empty() && !MAARUFA.contains(&ism) && !ism.starts_with('#') {
            return Err(format!(
                "the document uses the entity reference &{ism};, which no declaration in it \
                 defines — declarations are refused outright. An undeclared reference is either \
                 a damaged file or the second half of an XXE attempt, and substituting anything \
                 for it would put content this importer chose into a translation."
            ));
        }
        baqi = baad;
    }
    Ok(())
}

/// The header of a document: the doctype guard and the root element.
///
/// Consumes events up to and including the root start tag, and hands the root
/// back so a caller can read the version and language attributes off it.
///
/// # Errors
///
/// A document type declaration, a malformed document, or no root element at
/// all.
pub(crate) fn iqra_jidhr(
    qari: &mut QariXml<'_>,
) -> Result<(String, BTreeMap<String, String>), String> {
    loop {
        match qari.read_event() {
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Start(marka) | Event::Empty(marka)) => {
                let ism = ism_mahalli(&marka);
                let mut sifat = BTreeMap::new();
                for natija in marka.attributes() {
                    let Ok(attr) = natija else { continue };
                    let miftah = attr.key.local_name().as_ref().to_ascii_lowercase();
                    let qeema = attr.normalized_value(XmlVersion::Implicit1_0).map_or_else(
                        |_| attr.value.as_ref().to_owned(),
                        std::borrow::Cow::into_owned,
                    );
                    let _ = sifat.insert(miftah, qeema);
                }
                return Ok((ism, sifat));
            },
            Ok(Event::Eof) => return Err("the document has no root element".to_owned()),
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }
}

/// The reader's current position as a one-based line.
pub(crate) fn satr_hali(qari: &QariXml<'_>, astur: &FahrasAstur) -> usize {
    astur.satr(usize::try_from(qari.buffer_position()).unwrap_or(usize::MAX))
}

// ---------------------------------------------------------------------------
// XLIFF 1.2
// ---------------------------------------------------------------------------

/// Reads an XLIFF 1.2 document.
///
/// Walks `<file>` and `<group>` for the context a key is built from, then every
/// `<trans-unit>`. `<alt-trans>` is read past deliberately — its `<target>`
/// elements are a tool's alternative suggestions, not the translation, and
/// importing them would put a rejected candidate into the project — and the
/// count of what was skipped goes into the result's remarks.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] for a document type declaration, an encoding
/// declaration naming a legacy encoding, a root element that is not `<xliff>`,
/// an unresolvable entity reference, or a malformed document.
pub fn iqra_nuskha_ula(
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    let rafd = |sabab: String| KhataTarqee::IstiradFashil {
        masar: masar.to_path_buf(),
        sabab,
    };
    tahaqquq_wathiqa(nass).map_err(rafd)?;
    let astur = FahrasAstur::jadeed(nass);
    let mut qari = qari_jadeed(nass);
    let (ism_jidhr, sifat_jidhr) = iqra_jidhr(&mut qari).map_err(rafd)?;
    if ism_jidhr != "xliff" {
        return Err(rafd(format!(
            "the root element is <{ism_jidhr}> and an XLIFF document's root is <xliff>"
        )));
    }

    let mut milaff = MilaffWarid::jadeed(SighatIstirad::Xliff12, astur.adad());
    let saqf = khiyarat.hala_asasiya(SighatIstirad::Xliff12);
    let mut majmuat: Vec<String> = Vec::new();
    let mut badail = 0_usize;
    let mut thunaiya = 0_usize;
    if let Some(nuskha) = sifat_jidhr.get("version") {
        milaff
            .tanbihat
            .push(format!("the document declares XLIFF version {nuskha}"));
    }

    loop {
        let satr = satr_hali(&qari, &astur);
        match qari.read_event() {
            Ok(Event::Start(marka)) => match ism_mahalli(&marka).as_str() {
                "file" => {
                    if milaff.lugha_masdar.is_none() {
                        milaff.lugha_masdar = sifa(&marka, "source-language");
                    }
                    if milaff.lugha_hadaf.is_none() {
                        milaff.lugha_hadaf = sifa(&marka, "target-language");
                    }
                    // Pushed unconditionally, empty name included, because the
                    // end tag pops unconditionally and a conditional push would
                    // unbalance the stack on the first `<file>` with no
                    // `original` attribute.
                    majmuat.push(sifa(&marka, "original").unwrap_or_default());
                },
                "group" => majmuat.push(sifa(&marka, "id").unwrap_or_default()),
                "header" => tajawaz(&mut qari, "header").map_err(rafd)?,
                "alt-trans" => {
                    tajawaz(&mut qari, "alt-trans").map_err(rafd)?;
                    badail = badail.saturating_add(1);
                },
                "bin-unit" => {
                    tajawaz(&mut qari, "bin-unit").map_err(rafd)?;
                    thunaiya = thunaiya.saturating_add(1);
                },
                "trans-unit" => {
                    let (warid, sabab) =
                        iqra_wahda_ula(&mut qari, &marka, &majmuat, saqf, satr).map_err(rafd)?;
                    match sabab {
                        Some(sabab) => milaff.marfuda.push(MudkhalMarfud { warid, sabab }),
                        None => milaff.madakhil.push(warid),
                    }
                },
                _ => {},
            },
            Ok(Event::Empty(marka)) => {
                if ism_mahalli(&marka) == "trans-unit" {
                    // A self-closing unit has neither source nor target.
                    let mut warid = MudkhalWarid::jadeed(String::new(), None, satr);
                    warid.miftah = sifa(&marka, "resname").or_else(|| sifa(&marka, "id"));
                    warid.hala = saqf;
                    milaff.marfuda.push(MudkhalMarfud {
                        warid,
                        sabab: SababRafd::BilaHadaf,
                    });
                }
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "group" || ism == "file" {
                    let _ = majmuat.pop();
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd(rafd_doctype())),
            Ok(Event::Eof) => break,
            Ok(_) => {},
            Err(khata) => {
                return Err(rafd(format!(
                    "the document is not well-formed XML: {khata}"
                )));
            },
        }
    }

    if badail > 0 {
        milaff.tanbihat.push(format!(
            "{badail} <alt-trans> block(s) were read past. They hold a tool's alternative \
             suggestions rather than the translation, and importing one would put a candidate \
             somebody rejected into the project."
        ));
    }
    if thunaiya > 0 {
        milaff.tanbihat.push(format!(
            "{thunaiya} <bin-unit> element(s) were read past: they carry binary payloads, not \
             translatable text."
        ));
    }
    Ok(milaff)
}

/// One `<trans-unit>` and everything under it.
///
/// # Errors
///
/// As [`iqra_muhtawa`].
fn iqra_wahda_ula(
    qari: &mut QariXml<'_>,
    marka: &BytesStart<'_>,
    majmuat: &[String],
    saqf: HalatWarid,
    satr: usize,
) -> Result<(MudkhalWarid, Option<SababRafd>), String> {
    let muarrif = sifa(marka, "id").unwrap_or_default();
    let musamma = sifa(marka, "resname");
    let mamnu = sifa(marka, "translate").is_some_and(|qeema| qeema.eq_ignore_ascii_case("no"));
    let muattamad = sifa(marka, "approved");

    let mut masdar = String::new();
    let mut dharrat_masdar = Vec::new();
    let mut hadaf: Option<String> = None;
    let mut dharrat_hadaf = Vec::new();
    let mut mulahazat = Vec::new();
    let mut marja = Vec::new();
    let mut hala_khaam: Option<String> = None;
    let mut hala = HalatWarid::Musawwada;
    let mut muallam = false;
    let mut azwaj_masdar = 0_usize;
    let mut azwaj_hadaf = 0_usize;

    loop {
        match qari.read_event() {
            Ok(Event::Start(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "source" => {
                    let muhtawa = iqra_muhtawa(qari, "source", &DHARRAT_NUSKHA_ULA)?;
                    masdar = muhtawa.nass;
                    dharrat_masdar = muhtawa.dharrat;
                    azwaj_masdar = muhtawa.azwaj.len();
                },
                "target" => {
                    let (hala_wahda, muallam_wahda) = hala_nuskha_ula(
                        sifa(&dakhili, "state").as_deref(),
                        sifa(&dakhili, "state-qualifier").as_deref(),
                    );
                    hala = hala_wahda;
                    muallam = muallam_wahda;
                    hala_khaam = Some(wasf_hala_ula(
                        sifa(&dakhili, "state").as_deref(),
                        sifa(&dakhili, "state-qualifier").as_deref(),
                        muattamad.as_deref(),
                    ));
                    let muhtawa = iqra_muhtawa(qari, "target", &DHARRAT_NUSKHA_ULA)?;
                    hadaf = Some(muhtawa.nass);
                    dharrat_hadaf = muhtawa.dharrat;
                    azwaj_hadaf = muhtawa.azwaj.len();
                },
                "note" => {
                    let muhtawa = iqra_muhtawa(qari, "note", &[])?;
                    if !muhtawa.nass.trim().is_empty() {
                        mulahazat.push(muhtawa.nass.trim().to_owned());
                    }
                },
                "context" => {
                    let naw =
                        sifa(&dakhili, "context-type").unwrap_or_else(|| "context".to_owned());
                    let muhtawa = iqra_muhtawa(qari, "context", &[])?;
                    if !muhtawa.nass.trim().is_empty() {
                        marja.push(format!("{naw}={}", muhtawa.nass.trim()));
                    }
                },
                // Segmented source, and the alternatives a tool proposed. The
                // first duplicates `<source>` with `<mrk>` boundaries added and
                // the second is not the translation; both are read past.
                "seg-source" => tajawaz(qari, "seg-source")?,
                "alt-trans" => tajawaz(qari, "alt-trans")?,
                _ => {},
            },
            Ok(Event::Empty(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "source" => masdar = String::new(),
                "target" => {
                    let (hala_wahda, muallam_wahda) = hala_nuskha_ula(
                        sifa(&dakhili, "state").as_deref(),
                        sifa(&dakhili, "state-qualifier").as_deref(),
                    );
                    hala = hala_wahda;
                    muallam = muallam_wahda;
                    hala_khaam = Some(wasf_hala_ula(
                        sifa(&dakhili, "state").as_deref(),
                        sifa(&dakhili, "state-qualifier").as_deref(),
                        muattamad.as_deref(),
                    ));
                    hadaf = Some(String::new());
                },
                _ => {},
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "trans-unit" {
                    break;
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => {
                return Err("the document ended inside a <trans-unit>".to_owned());
            },
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }

    // The developer's own key beats a serial number, and the loser is kept as a
    // note rather than dropped: an import that discarded the `id` would make a
    // report unmatchable against the file it came from.
    let (miftah, badil) = match musamma {
        Some(musamma) if musamma != muarrif => (musamma, Some(format!("id={muarrif}"))),
        Some(musamma) => (musamma, None),
        None => (muarrif, None),
    };
    if let Some(badil) = badil {
        mulahazat.push(badil);
    }

    let mut warid = MudkhalWarid::jadeed(masdar, hadaf, satr);
    warid.miftah = Some(miftah).filter(|qeema| !qeema.is_empty());
    warid.siyaq = siyaq_majmuat(majmuat);
    warid.mulahazat = mulahazat;
    warid.marja = marja;
    warid.hala_khaam = hala_khaam;
    warid.hala = hala.adna(saqf);
    warid.muallam = muallam;
    warid.dharrat_masdar = dharrat_masdar;
    warid.dharrat_hadaf = dharrat_hadaf;
    if azwaj_masdar > azwaj_hadaf {
        warid.mulahazat.push(format!(
            "the source carries {azwaj_masdar} paired inline tag(s) and the target {azwaj_hadaf}"
        ));
    }

    let sabab = if mamnu {
        Some(SababRafd::MamnuMinAttarjama)
    } else {
        rafd_wahda(&warid)
    };
    Ok((warid, sabab))
}

/// The group path a unit sits under, as one string.
fn siyaq_majmuat(majmuat: &[String]) -> Option<String> {
    let mawjuda: Vec<&str> = majmuat
        .iter()
        .map(String::as_str)
        .filter(|juz| !juz.is_empty())
        .collect();
    if mawjuda.is_empty() {
        None
    } else {
        Some(mawjuda.join("/"))
    }
}

/// An XLIFF 1.2 `state` and `state-qualifier` as this project's vocabulary.
///
/// `final` and `signed-off` land on the draft side. See this module's header:
/// another tool's sign-off is a fact about that tool, and Taarib's approval is
/// a named human's attestation that they read the Arabic.
fn hala_nuskha_ula(hala: Option<&str>, muhaddid: Option<&str>) -> (HalatWarid, bool) {
    let asas = match hala.map(str::to_ascii_lowercase).as_deref() {
        Some("needs-review-translation" | "needs-review-adaptation" | "needs-review-l10n") => {
            HalatWarid::LilMuraja
        },
        _ => HalatWarid::Musawwada,
    };
    match muhaddid.map(str::to_ascii_lowercase).as_deref() {
        Some("mt-suggestion") => (HalatWarid::Aaliya, false),
        Some("fuzzy-match") => (asas, true),
        _ => (asas, false),
    }
}

/// The state attributes verbatim, for the report.
fn wasf_hala_ula(hala: Option<&str>, muhaddid: Option<&str>, muattamad: Option<&str>) -> String {
    let mut ajza = Vec::with_capacity(3);
    if let Some(hala) = hala {
        ajza.push(format!("state={hala}"));
    }
    if let Some(muhaddid) = muhaddid {
        ajza.push(format!("state-qualifier={muhaddid}"));
    }
    if let Some(muattamad) = muattamad {
        // Read, recorded, and without effect. See this module's header.
        ajza.push(format!("approved={muattamad}"));
    }
    if ajza.is_empty() {
        "no state declared".to_owned()
    } else {
        ajza.join(" ")
    }
}

/// The reason a fully parsed unit is declined, if there is one.
///
/// Shared by both XLIFF parsers and by TMX, so "a target that lost a
/// placeholder" means the same thing in all three.
pub(crate) fn rafd_wahda(warid: &MudkhalWarid) -> Option<SababRafd> {
    let hadaf = warid.hadaf.as_deref()?;
    if hadaf.is_empty() {
        return Some(SababRafd::HadafFarigh);
    }
    let mafqud = warid.dharrat_mafquda();
    if !mafqud.is_empty() {
        return Some(SababRafd::DharratMafquda { mafqud });
    }
    if hadaf == warid.masdar && !warid.masdar.is_empty() {
        return Some(SababRafd::HadafKaAlmasdar);
    }
    None
}

// ---------------------------------------------------------------------------
// XLIFF 2.0
// ---------------------------------------------------------------------------

/// Reads an XLIFF 2.0 document.
///
/// `<file>` and `<group>` build the context, `<unit>` holds the notes and the
/// original data, and each `<segment>` under a unit becomes one entry. An
/// `<ignorable>` is exactly what its name says — whitespace and punctuation
/// between segments — and is not an entry.
///
/// A unit with one segment is keyed by the unit's own id, which is what a table
/// exported from this project would have used. A unit with several is keyed
/// `unit/segment`, because the segments are different strings and giving them
/// one key would make the second overwrite the first.
///
/// `<mtc:matches>` — the translation-candidate module — is read past. Its
/// contents are a tool's suggestions with their scores, not the translation.
///
/// # Errors
///
/// As [`iqra_nuskha_ula`].
pub fn iqra_nuskha_thaniya(
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    let rafd = |sabab: String| KhataTarqee::IstiradFashil {
        masar: masar.to_path_buf(),
        sabab,
    };
    tahaqquq_wathiqa(nass).map_err(rafd)?;
    let astur = FahrasAstur::jadeed(nass);
    let mut qari = qari_jadeed(nass);
    let (ism_jidhr, sifat_jidhr) = iqra_jidhr(&mut qari).map_err(rafd)?;
    if ism_jidhr != "xliff" {
        return Err(rafd(format!(
            "the root element is <{ism_jidhr}> and an XLIFF document's root is <xliff>"
        )));
    }

    let mut milaff = MilaffWarid::jadeed(SighatIstirad::Xliff20, astur.adad());
    milaff.lugha_masdar = sifat_jidhr.get("srclang").cloned();
    milaff.lugha_hadaf = sifat_jidhr.get("trglang").cloned();
    let saqf = khiyarat.hala_asasiya(SighatIstirad::Xliff20);
    let mut majmuat: Vec<String> = Vec::new();
    let mut murashshahat = 0_usize;

    loop {
        let satr = satr_hali(&qari, &astur);
        match qari.read_event() {
            Ok(Event::Start(marka)) => match ism_mahalli(&marka).as_str() {
                "file" => majmuat.push(
                    sifa(&marka, "id")
                        .or_else(|| sifa(&marka, "original"))
                        .unwrap_or_default(),
                ),
                "group" => majmuat.push(sifa(&marka, "id").unwrap_or_default()),
                "matches" => {
                    tajawaz(&mut qari, "matches").map_err(rafd)?;
                    murashshahat = murashshahat.saturating_add(1);
                },
                "unit" => {
                    let wahda = iqra_wahda_thaniya(&mut qari, &marka, &majmuat, saqf, satr)
                        .map_err(rafd)?;
                    for (warid, sabab) in wahda {
                        match sabab {
                            Some(sabab) => milaff.marfuda.push(MudkhalMarfud { warid, sabab }),
                            None => milaff.madakhil.push(warid),
                        }
                    }
                },
                _ => {},
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "group" || ism == "file" {
                    let _ = majmuat.pop();
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd(rafd_doctype())),
            Ok(Event::Eof) => break,
            Ok(_) => {},
            Err(khata) => {
                return Err(rafd(format!(
                    "the document is not well-formed XML: {khata}"
                )));
            },
        }
    }

    if murashshahat > 0 {
        milaff.tanbihat.push(format!(
            "{murashshahat} <mtc:matches> block(s) were read past: they hold a tool's candidate \
             translations and their scores, not the translation."
        ));
    }
    Ok(milaff)
}

/// One `<unit>`, which produces one entry per `<segment>` under it.
///
/// # Errors
///
/// As [`iqra_muhtawa`].
fn iqra_wahda_thaniya(
    qari: &mut QariXml<'_>,
    marka: &BytesStart<'_>,
    majmuat: &[String],
    saqf: HalatWarid,
    satr: usize,
) -> Result<Vec<(MudkhalWarid, Option<SababRafd>)>, String> {
    let muarrif = sifa(marka, "id").unwrap_or_default();
    let musamma = sifa(marka, "name");
    let mamnu = sifa(marka, "translate").is_some_and(|qeema| qeema.eq_ignore_ascii_case("no"));

    let mut mulahazat: Vec<String> = Vec::new();
    let mut bayanat: BTreeMap<String, String> = BTreeMap::new();
    let mut maqati: Vec<MaqtaWahda> = Vec::new();

    loop {
        match qari.read_event() {
            Ok(Event::Start(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "note" => {
                    let muhtawa = iqra_muhtawa(qari, "note", &[])?;
                    if !muhtawa.nass.trim().is_empty() {
                        mulahazat.push(muhtawa.nass.trim().to_owned());
                    }
                },
                "data" => {
                    let muarrif_bayan = sifa(&dakhili, "id").unwrap_or_default();
                    let muhtawa = iqra_muhtawa(qari, "data", &[])?;
                    let _ = bayanat.insert(muarrif_bayan, muhtawa.nass);
                },
                "segment" => maqati.push(iqra_maqta(qari, &dakhili)?),
                // Whitespace and punctuation between segments. Not a string.
                "ignorable" => tajawaz(qari, "ignorable")?,
                "matches" => tajawaz(qari, "matches")?,
                _ => {},
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "unit" {
                    break;
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => return Err("the document ended inside a <unit>".to_owned()),
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }

    let miftah_wahda = musamma.filter(|ism| !ism.is_empty()).unwrap_or(muarrif);
    let wahid = maqati.len() <= 1;
    let mut madakhil = Vec::with_capacity(maqati.len());
    for (mawdi, maqta) in maqati.into_iter().enumerate() {
        let miftah = if wahid {
            miftah_wahda.clone()
        } else {
            let muarrif_maqta = maqta
                .muarrif
                .clone()
                .unwrap_or_else(|| mawdi.saturating_add(1).to_string());
            format!("{miftah_wahda}/{muarrif_maqta}")
        };

        let mut warid = MudkhalWarid::jadeed(maqta.masdar, Some(maqta.hadaf), satr);
        warid.miftah = Some(miftah).filter(|qeema| !qeema.is_empty());
        warid.siyaq = siyaq_majmuat(majmuat);
        warid.mulahazat.clone_from(&mulahazat);
        for marja_bayan in maraji_bayanat(&maqta.dharrat_masdar, &bayanat) {
            warid.mulahazat.push(marja_bayan);
        }
        warid.hala_khaam = Some(maqta.hala_khaam);
        warid.hala = maqta.hala.adna(saqf);
        warid.muallam = maqta.muallam;
        warid.dharrat_masdar = maqta.dharrat_masdar;
        warid.dharrat_hadaf = maqta.dharrat_hadaf;
        if maqta.azwaj_masdar > maqta.azwaj_hadaf {
            warid.mulahazat.push(format!(
                "the source carries {} paired inline tag(s) and the target {}",
                maqta.azwaj_masdar, maqta.azwaj_hadaf
            ));
        }

        let sabab = if mamnu {
            Some(SababRafd::MamnuMinAttarjama)
        } else if !maqta.laha_hadaf {
            Some(SababRafd::BilaHadaf)
        } else {
            rafd_wahda(&warid)
        };
        madakhil.push((warid, sabab));
    }
    Ok(madakhil)
}

/// One `<segment>`, before it becomes an entry.
///
/// Constructed field by field rather than through [`Default`], because the
/// review state has no sensible default: a segment whose state was never read
/// must carry the value the mapping produced, and a `Default` impl on
/// [`HalatWarid`] would be a value any struct could acquire by forgetting to
/// set one.
#[derive(Debug)]
struct MaqtaWahda {
    /// The segment's own id, when it declared one.
    muarrif: Option<String>,
    /// The source, with inline elements preserved.
    masdar: String,
    /// The target, likewise.
    hadaf: String,
    /// Whether a `<target>` element was present at all, which is different from
    /// its being empty.
    laha_hadaf: bool,
    /// The standalone placeholders in the source.
    dharrat_masdar: Vec<String>,
    /// The same for the target.
    dharrat_hadaf: Vec<String>,
    /// How many paired style elements the source carried.
    azwaj_masdar: usize,
    /// The same for the target.
    azwaj_hadaf: usize,
    /// The mapped state.
    hala: HalatWarid,
    /// Whether the file marked it unconfirmed.
    muallam: bool,
    /// The state attributes verbatim.
    hala_khaam: String,
}

/// Reads one `<segment>`.
///
/// # Errors
///
/// As [`iqra_muhtawa`].
fn iqra_maqta(qari: &mut QariXml<'_>, marka: &BytesStart<'_>) -> Result<MaqtaWahda, String> {
    let hala_nass = sifa(marka, "state");
    let hala_fariya = sifa(marka, "substate");
    let (hala, muallam) = hala_nuskha_thaniya(hala_nass.as_deref(), hala_fariya.as_deref());
    let mut maqta = MaqtaWahda {
        muarrif: sifa(marka, "id"),
        masdar: String::new(),
        hadaf: String::new(),
        laha_hadaf: false,
        dharrat_masdar: Vec::new(),
        dharrat_hadaf: Vec::new(),
        azwaj_masdar: 0,
        azwaj_hadaf: 0,
        hala,
        muallam,
        hala_khaam: wasf_hala_thaniya(hala_nass.as_deref(), hala_fariya.as_deref()),
    };

    loop {
        match qari.read_event() {
            Ok(Event::Start(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "source" => {
                    let muhtawa = iqra_muhtawa(qari, "source", &DHARRAT_NUSKHA_THANIYA)?;
                    maqta.masdar = muhtawa.nass;
                    maqta.dharrat_masdar = muhtawa.dharrat;
                    maqta.azwaj_masdar = muhtawa.azwaj.len();
                },
                "target" => {
                    let muhtawa = iqra_muhtawa(qari, "target", &DHARRAT_NUSKHA_THANIYA)?;
                    maqta.hadaf = muhtawa.nass;
                    maqta.dharrat_hadaf = muhtawa.dharrat;
                    maqta.azwaj_hadaf = muhtawa.azwaj.len();
                    maqta.laha_hadaf = true;
                },
                _ => {},
            },
            Ok(Event::Empty(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "source" => maqta.masdar = String::new(),
                "target" => maqta.laha_hadaf = true,
                _ => {},
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "segment" {
                    return Ok(maqta);
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => return Err("the document ended inside a <segment>".to_owned()),
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }
}

/// The `<originalData>` a segment's placeholders point at.
///
/// XLIFF 2.0 splits a placeholder into the tag inside the sentence and the
/// original code in a separate `<data>` element, joined by `dataRef`. The tag
/// is preserved in the string as it stands; the code it refers to is resolved
/// here and attached as a note, so a translator reading the entry can see that
/// `<ph id="1" dataRef="d1"/>` is a `%s` rather than an opaque marker.
fn maraji_bayanat(dharrat: &[String], bayanat: &BTreeMap<String, String>) -> Vec<String> {
    let mut maraji = Vec::new();
    for dharra in dharrat {
        let Some(mawqi) = dharra.find("dataRef=\"") else {
            continue;
        };
        let baad = dharra
            .get(mawqi.saturating_add("dataRef=\"".len())..)
            .unwrap_or_default();
        let Some(tul) = baad.find('"') else { continue };
        let marja = baad.get(..tul).unwrap_or_default();
        if let Some(khaam) = bayanat.get(marja) {
            maraji.push(format!("dataRef {marja} = {khaam}"));
        }
    }
    maraji
}

/// An XLIFF 2.0 `state` and `subState` as this project's vocabulary.
///
/// `final` lands on the draft side, exactly as `signed-off` does in 1.2. See
/// this module's header.
///
/// `subState` is an **open** vocabulary by design: the specification requires
/// only a prefix and a colon, and every tool invents its own values. So it is
/// matched on substrings rather than against a table that would be wrong the
/// first time a new tool wrote to it, and a value nothing recognises leaves the
/// state exactly as `state` set it rather than guessing.
fn hala_nuskha_thaniya(hala: Option<&str>, fariya: Option<&str>) -> (HalatWarid, bool) {
    let asas = match hala.map(str::to_ascii_lowercase).as_deref() {
        // Work has started and nobody has confirmed it. With a target present
        // that is precisely "wants a second pair of eyes"; with no target the
        // entry is declined before this matters.
        Some("initial") => HalatWarid::LilMuraja,
        _ => HalatWarid::Musawwada,
    };
    let Some(fariya) = fariya.map(str::to_ascii_lowercase) else {
        return (asas, false);
    };
    let aali = fariya
        .split([':', '-', '_'])
        .any(|juz| juz == "mt" || juz.starts_with("mt"));
    let muallam = fariya.contains("fuzzy");
    if aali {
        (HalatWarid::Aaliya, muallam)
    } else {
        (asas, muallam)
    }
}

/// The 2.0 state attributes verbatim, for the report.
fn wasf_hala_thaniya(hala: Option<&str>, fariya: Option<&str>) -> String {
    let mut ajza = Vec::with_capacity(2);
    if let Some(hala) = hala {
        ajza.push(format!("state={hala}"));
    }
    if let Some(fariya) = fariya {
        ajza.push(format!("subState={fariya}"));
    }
    if ajza.is_empty() {
        "no state declared".to_owned()
    } else {
        ajza.join(" ")
    }
}
