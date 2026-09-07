//! gettext PO و TMX — the two formats that arrive from outside the games
//! industry, and the two with the subtlest ways of losing a string.
//!
//! ## PO: the context is part of the identity
//!
//! Two entries with the same `msgid` and different `msgctxt` are two different
//! strings. That is the entire reason `msgctxt` exists — `"Open"` the verb on a
//! button and `"Open"` the adjective in a status line want different Arabic —
//! and a reader that keyed on `msgid` alone would collapse them, translate both
//! with whichever it saw last, and produce a catalogue that is wrong in a way
//! nobody can see by reading it.
//!
//! So the context travels with the entry, in [`MudkhalWarid::siyaq`], and the
//! matcher tries the context-qualified key before the bare one.
//!
//! ## PO: Arabic has six plural forms, and the header says how many
//!
//! `msgstr[3]` means nothing on its own. Its meaning comes from the
//! `Plural-Forms` header, and the two rules that matter here are not
//! compatible:
//!
//! ```text
//! nplurals=2; plural=(n != 1);
//!   index 0 = one            index 1 = other
//!
//! nplurals=6; plural=(n==0 ? 0 : n==1 ? 1 : n==2 ? 2 :
//!                    n%100>=3 && n%100<=10 ? 3 : n%100>=11 ? 4 : 5);
//!   index 0 = zero  1 = one  2 = two  3 = few  4 = many  5 = other
//! ```
//!
//! In the two-form rule index 1 is *other*; in the Arabic six-form rule index 1
//! is *one* and *other* is index 5. Reading a two-form catalogue's indices with
//! the Arabic rule puts the plural sentence in the singular slot, and does it
//! silently, in every plural string in the file.
//!
//! The header is therefore **parsed, not assumed**: `nplurals` is read out of
//! the `Plural-Forms` value and the form count comes from it. A plural entry in
//! a file that declares no `Plural-Forms` is declined with
//! [`SababRafd::SighatJamaMajhula`], and one in a file declaring a count that is
//! not six is declined with [`SababRafd::SuwarJamaGhayrArabiya`]. Neither is
//! mapped across, because there is no correct mapping — only a plausible one.
//!
//! The `plural=` expression itself is compared against the canonical Arabic
//! rule and a difference is reported as a remark rather than a refusal: six
//! forms indexed in CLDR order is unambiguous in practice, and a file that
//! spelled the expression differently but declared six is still an Arabic
//! catalogue.
//!
//! A plural entry that survives all of that is still **never applied**. A
//! project row holds one target and the entry holds six, and choosing one
//! silently is the mistake this module exists to not make — so it arrives as a
//! proposal carrying every form.
//!
//! ## PO: `#, fuzzy` is a proposal
//!
//! A fuzzy entry is one gettext propagated from a similar string and nobody
//! confirmed. It is usually close and it is sometimes about a different thing
//! entirely. It imports as a proposal and never as a translation.
//!
//! ## TMX: several Arabic variants is normal, and picking one silently is not
//!
//! A translation memory routinely carries `ar`, `ar-SA` and `ar-EG` segments in
//! the same `<tu>`. The one matching [`IstiradKhiyarat::ramz_lugha`] is taken,
//! then a bare `ar`, then the first in document order — and **the file's
//! remarks say which was taken and which others were there**. A project that
//! ends up with Egyptian Arabic in half its menus should be able to find out
//! when that happened.
//!
//! Inline elements — `<bpt>`, `<ept>`, `<ph>`, `<it>`, `<hi>`, `<sub>` — are
//! preserved into the string exactly as XLIFF's are, through the same routine.
//! See [`super::xliff`]'s header for why flattening them is a silent deletion.

use std::collections::BTreeMap;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};

use super::xliff::{
    QariXml, iqra_jidhr, iqra_muhtawa, ism_mahalli, qari_jadeed, rafd_doctype, rafd_wahda,
    satr_hali, sifa, tahaqquq_wathiqa, tajawaz,
};
use super::{
    FahrasAstur, FiatJama, HalatWarid, IstiradKhiyarat, MilaffWarid, MudkhalMarfud, MudkhalWarid,
    NawWarid, SababRafd, SighatIstirad, SuratJama, lugha_arabiya, martabat_arabiya,
};
use crate::khata::KhataTarqee;

/// How many plural forms Arabic has.
///
/// Six. Not a tunable: it is a property of the language, and a file declaring
/// anything else is a file for a different language whose indices do not carry
/// over.
pub const SUWAR_JAMA_ARABIYA: usize = 6;

/// The `plural=` expression a correct Arabic catalogue carries.
///
/// Compared with all whitespace removed. A file that agrees on the count and
/// differs here gets a remark, not a refusal — see this module's header.
pub const TAABEER_JAMA_ARABI: &str = "(n==0?0:n==1?1:n==2?2:n%100>=3&&n%100<=10?3:n%100>=11?4:5)";

/// The standalone inline elements of TMX.
///
/// `<bpt>` and `<ept>` are the two halves of a pair and `<hi>` wraps
/// translatable text, so none of them is here: losing one is a formatting
/// defect. `<ph>` is a standalone code and `<it>` is an isolated half of a pair
/// whose partner is in another segment — both are values, and losing one
/// deletes something the sentence was about.
const DHARRAT_TMX: [&str; 2] = ["ph", "it"];

// ---------------------------------------------------------------------------
// gettext PO
// ---------------------------------------------------------------------------

/// Which accumulator a bare `"…"` continuation line appends to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AkhirHaql {
    /// `msgctxt`.
    Siyaq,
    /// `msgid`.
    Muarrif,
    /// `msgid_plural`.
    Jama,
    /// `msgstr`.
    Tarjama,
    /// `msgstr[N]`.
    Sura(usize),
    /// `#| msgid`, the previous source a fuzzy entry was propagated from.
    Sabiq,
    /// Nothing is open; a continuation here is a malformed file and is kept as
    /// an unplaced line rather than appended to whatever came before.
    LaShay,
}

/// One PO entry as it accumulates across lines.
#[derive(Debug, Default)]
struct MudkhalPo {
    satr: usize,
    siyaq: Option<String>,
    muarrif: Option<String>,
    jama: Option<String>,
    tarjama: Option<String>,
    suwar: BTreeMap<usize, String>,
    mulahazat: Vec<String>,
    maraji: Vec<String>,
    alamat: Vec<String>,
    sabiq: Vec<String>,
    mahjur: bool,
}

impl MudkhalPo {
    /// Whether nothing has been written into this entry yet.
    fn farigh(&self) -> bool {
        self.siyaq.is_none()
            && self.muarrif.is_none()
            && self.tarjama.is_none()
            && self.suwar.is_empty()
            && self.mulahazat.is_empty()
            && self.maraji.is_empty()
            && self.alamat.is_empty()
    }

    /// Whether gettext marked this entry as unconfirmed.
    fn muallam(&self) -> bool {
        self.alamat
            .iter()
            .any(|alam| alam.eq_ignore_ascii_case("fuzzy"))
    }

    /// Appends a raw quoted run to whichever field is open.
    fn adif(&mut self, haql: AkhirHaql, khaam: &str) {
        match haql {
            AkhirHaql::Siyaq => self.siyaq.get_or_insert_with(String::new).push_str(khaam),
            AkhirHaql::Muarrif => self.muarrif.get_or_insert_with(String::new).push_str(khaam),
            AkhirHaql::Jama => self.jama.get_or_insert_with(String::new).push_str(khaam),
            AkhirHaql::Tarjama => self.tarjama.get_or_insert_with(String::new).push_str(khaam),
            AkhirHaql::Sura(fahras) => {
                self.suwar.entry(fahras).or_default().push_str(khaam);
            },
            AkhirHaql::Sabiq => {
                if let Some(akhir) = self.sabiq.last_mut() {
                    akhir.push_str(khaam);
                } else {
                    self.sabiq.push(khaam.to_owned());
                }
            },
            AkhirHaql::LaShay => {},
        }
    }
}

/// Reads a gettext PO or POT catalogue.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] when the catalogue's `Language:` header names
/// a language that is not Arabic. That is refused rather than imported, because
/// a French catalogue matched against an Arabic project by source text would
/// match perfectly and write French into every row it touched.
pub fn iqra_po(
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    let mut madakhil: Vec<MudkhalPo> = Vec::new();
    let mut ghayr_maqrua: Vec<(usize, String)> = Vec::new();
    let mut hali = MudkhalPo::default();
    let mut akhir = AkhirHaql::LaShay;

    for (mawdi, satr_khaam) in nass.lines().enumerate() {
        let raqm = mawdi.saturating_add(1);
        let satr = satr_khaam.trim_end_matches('\r');
        let mahdhub = satr.trim();
        if hali.satr == 0 {
            hali.satr = raqm;
        }

        if mahdhub.is_empty() {
            if !hali.farigh() {
                madakhil.push(std::mem::take(&mut hali));
            }
            akhir = AkhirHaql::LaShay;
            continue;
        }

        let (jasad, mahjur) = match mahdhub.strip_prefix("#~") {
            Some(baqi) => (baqi.trim_start(), true),
            None => (mahdhub, false),
        };
        if mahjur {
            hali.mahjur = true;
        }

        if !mahjur && let Some(baqi) = jasad.strip_prefix('#') {
            taliq_po(&mut hali, baqi, &mut akhir);
            continue;
        }

        if jasad.starts_with('"') {
            if akhir == AkhirHaql::LaShay {
                ghayr_maqrua.push((raqm, satr.to_owned()));
            } else {
                let khaam = iqtibasat(jasad);
                hali.adif(akhir, &khaam);
            }
            continue;
        }

        if let Some((haql, baqi)) = kalimat_po(jasad) {
            // A new `msgid` or `msgctxt` while one is already open ends the
            // previous entry. Catalogues without blank lines between
            // entries are unusual and legal, and a reader that needed the
            // blank line would merge every entry in one into a single
            // string.
            let yabda = matches!(haql, AkhirHaql::Muarrif | AkhirHaql::Siyaq);
            if yabda && (hali.muarrif.is_some() || hali.tarjama.is_some()) {
                madakhil.push(std::mem::take(&mut hali));
                hali.satr = raqm;
                hali.mahjur = mahjur;
            }
            akhir = haql;
            let khaam = iqtibasat(baqi);
            hali.adif(haql, &khaam);
        } else {
            ghayr_maqrua.push((raqm, satr.to_owned()));
            akhir = AkhirHaql::LaShay;
        }
    }
    if !hali.farigh() {
        madakhil.push(hali);
    }

    banni_po(masar, nass, madakhil, ghayr_maqrua, khiyarat)
}

/// Files one PO comment line onto the entry it belongs to.
fn taliq_po(hali: &mut MudkhalPo, baqi: &str, akhir: &mut AkhirHaql) {
    match baqi.chars().next() {
        Some('.') => {
            let matn = baqi.get(1..).unwrap_or_default().trim();
            if !matn.is_empty() {
                hali.mulahazat.push(matn.to_owned());
            }
        },
        Some(':') => {
            let matn = baqi.get(1..).unwrap_or_default();
            for marja in matn.split_whitespace() {
                hali.maraji.push(marja.to_owned());
            }
        },
        Some(',') => {
            let matn = baqi.get(1..).unwrap_or_default();
            for alam in matn.split(',') {
                let alam = alam.trim();
                if !alam.is_empty() {
                    hali.alamat.push(alam.to_owned());
                }
            }
        },
        Some('|') => {
            let matn = baqi.get(1..).unwrap_or_default().trim();
            hali.sabiq.push(iqtibasat(matn));
            *akhir = AkhirHaql::Sabiq;
        },
        _ => {
            let matn = baqi.trim();
            if !matn.is_empty() {
                hali.mulahazat.push(matn.to_owned());
            }
        },
    }
}

/// The keyword a PO line opens, and the rest of the line.
fn kalimat_po(jasad: &str) -> Option<(AkhirHaql, &str)> {
    // `msgid_plural` before `msgid`: the shorter keyword is a prefix of the
    // longer one, and testing it first would read every plural source as a
    // second singular source.
    if let Some(baqi) = jasad.strip_prefix("msgid_plural") {
        return Some((AkhirHaql::Jama, baqi));
    }
    if let Some(baqi) = jasad.strip_prefix("msgctxt") {
        return Some((AkhirHaql::Siyaq, baqi));
    }
    if let Some(baqi) = jasad.strip_prefix("msgid") {
        return Some((AkhirHaql::Muarrif, baqi));
    }
    if let Some(baqi) = jasad.strip_prefix("msgstr") {
        let mahdhub = baqi.trim_start();
        if let Some(dakhil) = mahdhub.strip_prefix('[') {
            let tul = dakhil.find(']')?;
            let raqm = dakhil.get(..tul)?.trim().parse::<usize>().ok()?;
            let baad = dakhil.get(tul.saturating_add(1)..)?;
            return Some((AkhirHaql::Sura(raqm), baad));
        }
        return Some((AkhirHaql::Tarjama, baqi));
    }
    None
}

/// Every quoted run on a line, concatenated, with escapes still in place.
///
/// The escapes are resolved once at the end rather than per line, because a
/// `\` at the very end of a quoted run continues into the next one and
/// resolving early would consume it against the wrong character.
fn iqtibasat(baqi: &str) -> String {
    let mut khaam = String::new();
    let mut dakhil = false;
    let mut mailat = false;
    for harf in baqi.chars() {
        if !dakhil {
            if harf == '"' {
                dakhil = true;
            }
            continue;
        }
        if mailat {
            khaam.push('\\');
            khaam.push(harf);
            mailat = false;
            continue;
        }
        match harf {
            '\\' => mailat = true,
            '"' => dakhil = false,
            _ => khaam.push(harf),
        }
    }
    khaam
}

/// Resolves the C escapes gettext writes.
///
/// `\n \t \r \" \\ \a \b \f \v`, octal `\OOO` and hex `\xHH`.
///
/// A numeric escape whose value is above `0x7F` is **left exactly as written**.
/// Such a byte is a raw byte in whatever encoding the file was authored in, and
/// this crate has already decided it does not guess encodings: turning `\xE9`
/// into `é` assumes Latin-1 and turning it into `U+00E9` assumes the same thing
/// with extra steps. Preserving it keeps the string honest and visible.
#[must_use]
pub fn fukk_hurub_c(khaam: &str) -> String {
    let mut natija = String::with_capacity(khaam.len());
    let mut ahruf = khaam.chars().peekable();
    while let Some(harf) = ahruf.next() {
        if harf != '\\' {
            natija.push(harf);
            continue;
        }
        match ahruf.next() {
            Some('n') => natija.push('\n'),
            Some('t') => natija.push('\t'),
            Some('r') => natija.push('\r'),
            Some('"') => natija.push('"'),
            // A trailing backslash is preserved for the same reason an unknown
            // escape is: the next read has to see the byte that was written.
            Some('\\') | None => natija.push('\\'),
            Some('a') => natija.push('\u{7}'),
            Some('b') => natija.push('\u{8}'),
            Some('f') => natija.push('\u{C}'),
            Some('v') => natija.push('\u{B}'),
            Some('x') => {
                let mut qeema = 0_u32;
                let mut adad = 0_usize;
                while adad < 2 {
                    let Some(raqm) = ahruf.peek().and_then(|harf| harf.to_digit(16)) else {
                        break;
                    };
                    let _ = ahruf.next();
                    qeema = qeema.saturating_mul(16).saturating_add(raqm);
                    adad = adad.saturating_add(1);
                }
                daa_raqmi(&mut natija, qeema, adad, 'x');
            },
            Some(bidaya @ '0'..='7') => {
                let mut qeema = bidaya.to_digit(8).unwrap_or(0);
                let mut adad = 1_usize;
                while adad < 3 {
                    let Some(raqm) = ahruf.peek().and_then(|harf| harf.to_digit(8)) else {
                        break;
                    };
                    let _ = ahruf.next();
                    qeema = qeema.saturating_mul(8).saturating_add(raqm);
                    adad = adad.saturating_add(1);
                }
                daa_raqmi(&mut natija, qeema, adad, 'o');
            },
            Some(akhar) => {
                natija.push('\\');
                natija.push(akhar);
            },
        }
    }
    natija
}

/// Writes a numeric escape's value, or writes the escape back unresolved.
fn daa_raqmi(natija: &mut String, qeema: u32, adad: usize, naw: char) {
    use std::fmt::Write as _;

    if adad == 0 {
        natija.push('\\');
        natija.push(naw);
        return;
    }
    match char::from_u32(qeema) {
        Some(harf) if qeema < 0x80 => natija.push(harf),
        // Above ASCII the value is a byte in an encoding this module refuses to
        // guess. It goes back as it came.
        _ => {
            if naw == 'x' {
                let _ = write!(natija, "\\x{qeema:x}");
            } else {
                let _ = write!(natija, "\\{qeema:o}");
            }
        },
    }
}

/// Turns the accumulated PO entries into the shared result model.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] when the `Language:` header names a language
/// that is not Arabic.
fn banni_po(
    masar: &Path,
    nass: &str,
    madakhil: Vec<MudkhalPo>,
    ghayr_maqrua: Vec<(usize, String)>,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    let mut milaff = MilaffWarid::jadeed(SighatIstirad::GettextPo, nass.lines().count());
    let saqf = khiyarat.hala_asasiya(SighatIstirad::GettextPo);

    let tarwisa = madakhil
        .iter()
        .find(|mudkhal| {
            mudkhal.siyaq.is_none() && mudkhal.muarrif.as_deref().is_some_and(str::is_empty)
        })
        .and_then(|mudkhal| mudkhal.tarjama.as_deref())
        .map(fukk_hurub_c)
        .map(|matn| tarwisat_po(&matn))
        .unwrap_or_default();

    if let Some(lugha) = tarwisa.get("language")
        && !lugha.trim().is_empty()
        && !lugha_arabiya(lugha)
    {
        return Err(KhataTarqee::IstiradFashil {
            masar: masar.to_path_buf(),
            sabab: format!(
                "the catalogue declares Language: {lugha}, which is not Arabic. It is refused \
                 rather than imported: matched by source text against an Arabic project it would \
                 match every row perfectly and write {lugha} into all of them."
            ),
        });
    }
    milaff.lugha_hadaf = tarwisa.get("language").cloned();

    let (suwar_muallana, taabeer) = sighat_jama(tarwisa.get("plural-forms").map(String::as_str));
    if let (Some(suwar), Some(taabeer)) = (suwar_muallana, taabeer.as_deref())
        && suwar == SUWAR_JAMA_ARABIYA
    {
        let musawwa: String = taabeer
            .chars()
            .filter(|harf| !harf.is_whitespace())
            .collect();
        let mutawaqqa: String = TAABEER_JAMA_ARABI
            .chars()
            .filter(|harf| !harf.is_whitespace())
            .collect();
        if musawwa.trim_end_matches(';') != mutawaqqa {
            milaff.tanbihat.push(format!(
                "Plural-Forms declares six forms with the expression {taabeer}, which is not the \
                 canonical Arabic rule. The six indices are read in CLDR order (zero, one, two, \
                 few, many, other) regardless; check the file if that is not what it meant."
            ));
        }
    }
    match suwar_muallana {
        Some(suwar) => milaff
            .tanbihat
            .push(format!("Plural-Forms declares {suwar} plural form(s)")),
        None => milaff
            .tanbihat
            .push("the catalogue declares no Plural-Forms header".to_owned()),
    }

    for (raqm, satr) in ghayr_maqrua {
        let warid = MudkhalWarid::jadeed(satr.clone(), None, raqm);
        milaff.marfuda.push(MudkhalMarfud {
            warid,
            sabab: SababRafd::SatrGhayrMafhum {
                juz: satr.chars().take(80).collect(),
            },
        });
    }

    let mut ruit: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut mukarrara = 0_usize;

    for mudkhal in madakhil {
        let (warid, sabab) = min_mudkhal_po(&mudkhal, saqf, suwar_muallana);
        if sabab.is_none()
            && let Some(miftah) = warid.miftah.clone()
        {
            let siyaq = warid.siyaq.clone().unwrap_or_default();
            let adad = ruit.entry((siyaq, miftah)).or_insert(0);
            *adad = adad.saturating_add(1);
            if *adad > 1 {
                mukarrara = mukarrara.saturating_add(1);
            }
        }
        match sabab {
            Some(sabab) => milaff.marfuda.push(MudkhalMarfud { warid, sabab }),
            None => milaff.madakhil.push(warid),
        }
    }

    if mukarrara > 0 {
        milaff.tanbihat.push(format!(
            "{mukarrara} entry(ies) repeat a msgctxt and msgid pair that already appeared. They \
             are kept separately rather than merged — a duplicate is a defect in the catalogue \
             and collapsing it would hide which of the two translations was taken."
        ));
    }
    Ok(milaff)
}

/// One PO entry as an incoming entry, with the reason it is declined if it is.
fn min_mudkhal_po(
    mudkhal: &MudkhalPo,
    saqf: HalatWarid,
    suwar_muallana: Option<usize>,
) -> (MudkhalWarid, Option<SababRafd>) {
    let masdar = mudkhal
        .muarrif
        .as_deref()
        .map(fukk_hurub_c)
        .unwrap_or_default();
    let siyaq = mudkhal.siyaq.as_deref().map(fukk_hurub_c);
    let mufrad = mudkhal.tarjama.as_deref().map(fukk_hurub_c);

    let mut warid = MudkhalWarid::jadeed(masdar.clone(), mufrad.clone(), mudkhal.satr);
    warid.miftah = Some(masdar.clone()).filter(|qeema| !qeema.is_empty());
    warid.siyaq = siyaq;
    warid.mulahazat.clone_from(&mudkhal.mulahazat);
    warid.marja.clone_from(&mudkhal.maraji);
    warid.hala = saqf;
    warid.muallam = mudkhal.muallam();
    if !mudkhal.alamat.is_empty() {
        warid.hala_khaam = Some(format!("#, {}", mudkhal.alamat.join(", ")));
    }
    for sabiq in &mudkhal.sabiq {
        warid
            .mulahazat
            .push(format!("previously: {}", fukk_hurub_c(sabiq)));
    }

    if mudkhal.mahjur {
        return (warid, Some(SababRafd::Mahjura));
    }
    // The header pseudo-entry: an empty msgid with no context. Its `msgstr` is
    // the header block, which is metadata and not a string anybody translates.
    if masdar.is_empty() && warid.siyaq.is_none() {
        return (warid, Some(SababRafd::Taalim));
    }

    if let Some(asl_jama) = mudkhal.jama.as_deref().map(fukk_hurub_c) {
        return jama_po(warid, mudkhal, &asl_jama, suwar_muallana);
    }

    let sabab = match mufrad.as_deref() {
        None => Some(SababRafd::BilaHadaf),
        Some("") => Some(SababRafd::HadafFarigh),
        Some(matn) if matn == masdar => Some(SababRafd::HadafKaAlmasdar),
        Some(_) => None,
    };
    (warid, sabab)
}

/// A plural entry, with its indices mapped onto Arabic's six categories.
///
/// Every path out of here either produces a fully parsed plural set or declines
/// it by name. Nothing is mapped across a form-count mismatch — see this
/// module's header for the two rules that make that unsafe.
fn jama_po(
    mut warid: MudkhalWarid,
    mudkhal: &MudkhalPo,
    asl_jama: &str,
    suwar_muallana: Option<usize>,
) -> (MudkhalWarid, Option<SababRafd>) {
    let Some(muallan) = suwar_muallana else {
        return (warid, Some(SababRafd::SighatJamaMajhula));
    };
    if muallan != SUWAR_JAMA_ARABIYA {
        return (
            warid,
            Some(SababRafd::SuwarJamaGhayrArabiya { adad: muallan }),
        );
    }
    if let Some(aqsa) = mudkhal.suwar.keys().max()
        && *aqsa >= SUWAR_JAMA_ARABIYA
    {
        // The file declares six and carries a seventh index. It disagrees with
        // itself and there is no index to map that onto.
        return (
            warid,
            Some(SababRafd::SuwarJamaGhayrArabiya {
                adad: aqsa.saturating_add(1),
            }),
        );
    }

    let mut suwar = Vec::with_capacity(SUWAR_JAMA_ARABIYA);
    for fahras in 0..SUWAR_JAMA_ARABIYA {
        let Some(fia) = FiatJama::min_fahras(fahras) else {
            continue;
        };
        let nass = mudkhal
            .suwar
            .get(&fahras)
            .map(String::as_str)
            .map(fukk_hurub_c);
        suwar.push(SuratJama {
            fia,
            nass: nass.unwrap_or_default(),
        });
    }

    // The `other` form is the entry's single target, offered as the proposal's
    // default. Every form travels with it, so nothing about the set is lost by
    // the project row holding one string.
    let ghayr = suwar
        .iter()
        .find(|sura| sura.fia == FiatJama::Ghayr)
        .map(|sura| sura.nass.clone())
        .unwrap_or_default();
    let adad_mamlua = suwar.iter().filter(|sura| !sura.nass.is_empty()).count();

    warid.hadaf = Some(ghayr);
    warid.naw = NawWarid::Jama {
        asl_jama: asl_jama.to_owned(),
        suwar,
    };
    warid.mulahazat.push(format!("msgid_plural: {asl_jama}"));

    if adad_mamlua == 0 {
        return (warid, Some(SababRafd::HadafFarigh));
    }
    // Not declined: it goes through as a proposal, because a project row holds
    // one target and picking a form silently is the mistake. See
    // `super::wasm_iqtirah`, which routes any `NawWarid::Jama` entry there.
    (warid, None)
}

/// The header block as a map, keys lowercased.
fn tarwisat_po(matn: &str) -> BTreeMap<String, String> {
    let mut tarwisa = BTreeMap::new();
    for satr in matn.lines() {
        let Some(fasl) = satr.find(':') else { continue };
        let miftah = satr
            .get(..fasl)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let qeema = satr
            .get(fasl.saturating_add(1)..)
            .unwrap_or_default()
            .trim();
        if !miftah.is_empty() {
            let _ = tarwisa.insert(miftah, qeema.to_owned());
        }
    }
    tarwisa
}

/// The declared form count and plural expression out of a `Plural-Forms` value.
///
/// Parsed rather than assumed. `nplurals` is tested before `plural` because the
/// second name is a suffix of the first, and testing the short one first would
/// read `nplurals=6` as an expression.
#[must_use]
pub fn sighat_jama(qeema: Option<&str>) -> (Option<usize>, Option<String>) {
    let Some(qeema) = qeema else {
        return (None, None);
    };
    let mut adad = None;
    let mut taabeer = None;
    for juz in qeema.split(';') {
        let mahdhub = juz.trim();
        if let Some(baqi) = mahdhub.strip_prefix("nplurals") {
            adad = baqi
                .trim_start()
                .strip_prefix('=')
                .and_then(|raqm| raqm.trim().parse().ok());
        } else if let Some(baqi) = mahdhub.strip_prefix("plural") {
            taabeer = baqi
                .trim_start()
                .strip_prefix('=')
                .map(|matn| matn.trim().to_owned());
        }
    }
    (adad, taabeer)
}

// ---------------------------------------------------------------------------
// TMX 1.4
// ---------------------------------------------------------------------------

/// One `<tuv>` of a translation unit.
#[derive(Debug, Clone)]
struct TuvWarid {
    /// The language tag off `xml:lang` or `lang`.
    ramz: String,
    /// The segment text with inline elements preserved.
    nass: String,
    /// Its standalone placeholders.
    dharrat: Vec<String>,
    /// How many paired inline elements it carried.
    azwaj: usize,
}

/// Reads a TMX translation memory.
///
/// Every `<tu>` becomes one entry: its Arabic `<tuv>` is the target and the
/// source-language `<tuv>` is the source. `<prop>` and `<note>` become the
/// entry's notes, and `tuid` becomes its key.
///
/// # Errors
///
/// [`KhataTarqee::IstiradFashil`] for a document type declaration, an encoding
/// declaration naming a legacy encoding, a root element that is not `<tmx>`, a
/// TMX version this build does not read, an unresolvable entity, or a malformed
/// document.
pub fn iqra_tmx(
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
    if ism_jidhr != "tmx" {
        return Err(rafd(format!(
            "the root element is <{ism_jidhr}> and a TMX document's root is <tmx>"
        )));
    }
    if let Some(nuskha) = sifat_jidhr.get("version")
        && !nuskha.starts_with("1.")
    {
        return Err(rafd(format!(
            "the document declares TMX version {nuskha}. This build reads the 1.x family, whose \
             <tu>/<tuv>/<seg> shape it understands, and refuses anything else rather than \
             reading part of it and reporting a complete import."
        )));
    }

    let mut milaff = MilaffWarid::jadeed(SighatIstirad::Tmx, astur.adad());
    let saqf = khiyarat.hala_asasiya(SighatIstirad::Tmx);
    let mut lugha_masdar: Option<String> = None;
    let mut arabiyat: BTreeMap<String, usize> = BTreeMap::new();
    let mut muzdawija = 0_usize;

    loop {
        let satr = satr_hali(&qari, &astur);
        match qari.read_event() {
            Ok(Event::Start(marka)) => match ism_mahalli(&marka).as_str() {
                "header" => {
                    lugha_masdar = sifa(&marka, "srclang");
                    if let Some(adat) = sifa(&marka, "creationtool") {
                        milaff.tanbihat.push(format!("created by {adat}"));
                    }
                    tajawaz(&mut qari, "header").map_err(rafd)?;
                },
                "tu" => {
                    let wahda = iqra_tu(&mut qari, &marka).map_err(rafd)?;
                    let mustanad = wahda
                        .srclang
                        .clone()
                        .or_else(|| lugha_masdar.clone())
                        .unwrap_or_default();
                    let (warid, sabab, wujida) =
                        min_tu(wahda, &mustanad, saqf, satr, &khiyarat.ramz_lugha);
                    // Counted once per unit, not once per segment: a unit that
                    // repeats a tag is malformed and should not make the file
                    // summary say the variant is twice as common as it is.
                    let mut farida = wujida;
                    farida.sort();
                    farida.dedup();
                    if farida.len() > 1 {
                        muzdawija = muzdawija.saturating_add(1);
                    }
                    for ramz in farida {
                        let adad = arabiyat.entry(ramz).or_insert(0);
                        *adad = adad.saturating_add(1);
                    }
                    match sabab {
                        Some(sabab) => milaff.marfuda.push(MudkhalMarfud { warid, sabab }),
                        None => milaff.madakhil.push(warid),
                    }
                },
                _ => {},
            },
            // A self-closing `<header/>` carries its whole content in its
            // attributes and has no end tag to read up to. Handled apart from
            // the paired form, because reading past a close that does not exist
            // would consume the rest of the document.
            Ok(Event::Empty(marka)) => {
                if ism_mahalli(&marka) == "header" {
                    lugha_masdar = sifa(&marka, "srclang");
                    if let Some(adat) = sifa(&marka, "creationtool") {
                        milaff.tanbihat.push(format!("created by {adat}"));
                    }
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

    milaff.lugha_masdar = lugha_masdar;
    if arabiyat.len() > 1 {
        let wasf: Vec<String> = arabiyat
            .iter()
            .map(|(ramz, adad)| format!("{ramz} in {adad} unit(s)"))
            .collect();
        milaff.tanbihat.push(format!(
            "the file carries more than one Arabic variant: {}. The one matching {:?} was taken \
             where present, then a bare \"ar\", then the first in document order; every unit \
             where the choice mattered says so in its own notes.",
            wasf.join(", "),
            khiyarat.ramz_lugha
        ));
    } else if let Some((ramz, adad)) = arabiyat.iter().next() {
        milaff
            .tanbihat
            .push(format!("Arabic taken from {ramz} in {adad} unit(s)"));
    }
    if muzdawija > 0 {
        milaff.tanbihat.push(format!(
            "{muzdawija} unit(s) carried several Arabic segments and one was chosen; the others \
             were not imported and are named on each entry."
        ));
    }
    milaff.lugha_hadaf = arabiyat
        .iter()
        .max_by_key(|(_, adad)| **adad)
        .map(|(ramz, _)| ramz.clone());
    Ok(milaff)
}

/// One `<tu>` before a language is chosen out of it.
#[derive(Debug)]
struct TuWarid {
    /// `tuid`.
    muarrif: Option<String>,
    /// A `srclang` the unit declared for itself, overriding the header's.
    srclang: Option<String>,
    /// `creationtool`, `creationid` and friends, for the report.
    wasf_asl: Option<String>,
    /// `<prop>` and `<note>` content.
    mulahazat: Vec<String>,
    /// Every `<tuv>`, in document order.
    tuwuv: Vec<TuvWarid>,
}

/// Reads one `<tu>` and everything under it.
///
/// # Errors
///
/// As [`iqra_muhtawa`].
fn iqra_tu(qari: &mut QariXml<'_>, marka: &BytesStart<'_>) -> Result<TuWarid, String> {
    let mut wahda = TuWarid {
        muarrif: sifa(marka, "tuid"),
        srclang: sifa(marka, "srclang"),
        wasf_asl: wasf_asl_tu(marka),
        mulahazat: Vec::new(),
        tuwuv: Vec::new(),
    };

    loop {
        match qari.read_event() {
            Ok(Event::Start(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "prop" => {
                    let naw = sifa(&dakhili, "type").unwrap_or_else(|| "prop".to_owned());
                    let muhtawa = iqra_muhtawa(qari, "prop", &[])?;
                    if !muhtawa.nass.trim().is_empty() {
                        wahda
                            .mulahazat
                            .push(format!("{naw}={}", muhtawa.nass.trim()));
                    }
                },
                "note" => {
                    let muhtawa = iqra_muhtawa(qari, "note", &[])?;
                    if !muhtawa.nass.trim().is_empty() {
                        wahda.mulahazat.push(muhtawa.nass.trim().to_owned());
                    }
                },
                "tuv" => {
                    // `xml:lang` in 1.4 and a bare `lang` in 1.1. `sifa` matches
                    // on the local name, so one lookup finds either.
                    let ramz = sifa(&dakhili, "lang").unwrap_or_default();
                    wahda.tuwuv.push(iqra_tuv(qari, ramz)?);
                },
                _ => {},
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "tu" {
                    return Ok(wahda);
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => return Err("the document ended inside a <tu>".to_owned()),
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }
}

/// Reads one `<tuv>` up to its end tag.
///
/// # Errors
///
/// As [`iqra_muhtawa`].
fn iqra_tuv(qari: &mut QariXml<'_>, ramz: String) -> Result<TuvWarid, String> {
    let mut tuv = TuvWarid {
        ramz,
        nass: String::new(),
        dharrat: Vec::new(),
        azwaj: 0,
    };
    loop {
        match qari.read_event() {
            Ok(Event::Start(dakhili)) => match ism_mahalli(&dakhili).as_str() {
                "seg" => {
                    let muhtawa = iqra_muhtawa(qari, "seg", &DHARRAT_TMX)?;
                    tuv.nass = muhtawa.nass;
                    tuv.dharrat = muhtawa.dharrat;
                    tuv.azwaj = muhtawa.azwaj.len();
                },
                "prop" => tajawaz(qari, "prop")?,
                "note" => tajawaz(qari, "note")?,
                _ => {},
            },
            Ok(Event::Empty(dakhili)) => {
                if ism_mahalli(&dakhili) == "seg" {
                    tuv.nass = String::new();
                }
            },
            Ok(Event::End(nihaya)) => {
                let ism = nihaya.local_name().as_ref().to_ascii_lowercase();
                if ism == "tuv" {
                    return Ok(tuv);
                }
            },
            Ok(Event::DocType(_)) => return Err(rafd_doctype()),
            Ok(Event::Eof) => return Err("the document ended inside a <tuv>".to_owned()),
            Ok(_) => {},
            Err(khata) => return Err(format!("the document is not well-formed XML: {khata}")),
        }
    }
}

/// The provenance attributes of a `<tu>`, as one line for the report.
fn wasf_asl_tu(marka: &BytesStart<'_>) -> Option<String> {
    let mut ajza = Vec::with_capacity(3);
    for ism in ["creationtool", "creationid", "changeid"] {
        if let Some(qeema) = sifa(marka, ism) {
            ajza.push(format!("{ism}={qeema}"));
        }
    }
    if ajza.is_empty() {
        None
    } else {
        Some(ajza.join(" "))
    }
}

/// Chooses a source and an Arabic target out of one unit.
///
/// Returns the entry, the reason it is declined if it is, and every Arabic tag
/// the unit carried — the last so the file-level summary can say which variants
/// were in the memory and how often.
fn min_tu(
    wahda: TuWarid,
    srclang: &str,
    saqf: HalatWarid,
    satr: usize,
    mufaddal: &str,
) -> (MudkhalWarid, Option<SababRafd>, Vec<String>) {
    let arabiyat: Vec<String> = wahda
        .tuwuv
        .iter()
        .filter(|tuv| lugha_arabiya(&tuv.ramz))
        .map(|tuv| tuv.ramz.clone())
        .collect();

    let mukhtar = wahda
        .tuwuv
        .iter()
        .enumerate()
        .filter_map(|(mawdi, tuv)| {
            martabat_arabiya(&tuv.ramz, mufaddal).map(|martaba| (martaba, mawdi, tuv))
        })
        .min_by_key(|(martaba, mawdi, _)| (*martaba, *mawdi))
        .map(|(_, mawdi, tuv)| (mawdi, tuv.clone()));

    // The source segment: the one in the declared source language, else the
    // first that is not Arabic. `*all*` is TMX's own way of saying the memory
    // has no single source language, so it falls through to the second rule.
    let masdar_tuv = wahda
        .tuwuv
        .iter()
        .enumerate()
        .find(|(mawdi, tuv)| {
            mukhtar.as_ref().is_none_or(|(makan, _)| makan != mawdi)
                && !srclang.eq_ignore_ascii_case("*all*")
                && tuv.ramz.eq_ignore_ascii_case(srclang)
        })
        .or_else(|| {
            wahda
                .tuwuv
                .iter()
                .enumerate()
                .find(|(_, tuv)| !lugha_arabiya(&tuv.ramz))
        })
        .map(|(_, tuv)| tuv.clone());

    let masdar = masdar_tuv
        .as_ref()
        .map(|tuv| tuv.nass.clone())
        .unwrap_or_default();
    let Some((_, arabi)) = mukhtar else {
        let mut warid = MudkhalWarid::jadeed(masdar, None, satr);
        warid.miftah.clone_from(&wahda.muarrif);
        warid.mulahazat = wahda.mulahazat;
        warid.hala = saqf;
        return (warid, Some(SababRafd::LughaGhayrMawjuda), arabiyat);
    };

    let mut warid = MudkhalWarid::jadeed(masdar, Some(arabi.nass.clone()), satr);
    warid.miftah = wahda.muarrif.clone().filter(|qeema| !qeema.is_empty());
    warid.mulahazat = wahda.mulahazat;
    warid.hala = saqf;
    warid.hala_khaam = wahda.wasf_asl;
    warid.dharrat_masdar = masdar_tuv
        .as_ref()
        .map(|tuv| tuv.dharrat.clone())
        .unwrap_or_default();
    warid.dharrat_hadaf.clone_from(&arabi.dharrat);
    warid
        .mulahazat
        .push(format!("Arabic taken from xml:lang={}", arabi.ramz));

    let ukhra: Vec<&String> = arabiyat
        .iter()
        .filter(|ramz| **ramz != arabi.ramz)
        .collect();
    if !ukhra.is_empty() {
        let asmaa: Vec<String> = ukhra.iter().map(|ramz| (*ramz).clone()).collect();
        warid.mulahazat.push(format!(
            "also carried Arabic segments in {}",
            asmaa.join(", ")
        ));
    }
    if let Some(masdar_tuv) = masdar_tuv.as_ref()
        && masdar_tuv.azwaj > arabi.azwaj
    {
        warid.mulahazat.push(format!(
            "the source carries {} paired inline tag(s) and the Arabic {}",
            masdar_tuv.azwaj, arabi.azwaj
        ));
    }

    let sabab = rafd_wahda(&warid);
    (warid, sabab, arabiyat)
}
