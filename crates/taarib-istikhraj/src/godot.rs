//! غودو — Godot's translations and scene text, normalized.
//!
//! An adapter over the Phase 9 readers in `taarib_muhawwil_godot::pck`. The PCK
//! framing, the binary resource format, the `Translation` property layout and
//! the `OptimizedTranslation` hash table are all read there; nothing in this file
//! parses a byte of any of them.
//!
//! Two things in here *are* parsers, and both are text formats with no reader in
//! the workspace: gettext `.po`, and Godot's text scene and resource form
//! (`.tscn` / `.tres`). Neither is a container: they are line-oriented text, they
//! are read with a scanner and not a byte cursor, and a failure in either loses
//! one file rather than corrupting one. Writing them here is the alternative to
//! not reading them at all.
//!
//! ## The message id is Godot's own key
//!
//! `tr("MSG")` looks a string up by its message id. The developer chose that id,
//! wrote it into the scene, and keeps it stable — so it goes into
//! [`MawqiNass::miftah_muharrik`] and identity stops depending on the text. A
//! game that rewrites a line under an unchanged id comes back as *changed*
//! rather than as *removed and added*, and the translator keeps their work.
//!
//! The locale joins the key. Godot ships one `.translation` per locale under the
//! same ids, so the English and the French of one line would otherwise hash to
//! one identity and the fold in [`crate::jadwal::JadwalNusus::adif`] would keep
//! whichever was read first. There is no `.locmeta` equivalent here — the source
//! locale lives in `project.godot`, which is compiled to a binary form this
//! module does not read — so rather than guess that English is the source, every
//! locale is kept, each with its own identity, and the read report names the
//! locale each entry came from.
//!
//! ## `OptimizedTranslation` cannot be enumerated, and that is not a defect here
//!
//! The optimized form stores a perfect hash table: bucket seeds, 32-bit key
//! hashes, and the translated strings. **The source message ids are not in the
//! file.** The reader says so in as many words, and no amount of work on this
//! side recovers them — a hash is not invertible.
//!
//! So this module does the one honest thing available. It collects every message
//! id it learned from the plain `Translation` resources and the `.po` files in
//! the same game, probes the optimized table with each one through the reader's
//! own lookup, and takes what comes back. When it learned no ids at all, it
//! refuses the resource and names the reason — and the refusal it records is one
//! whose remedy is "play the game once with capture on", which is exactly right:
//! those strings *are* drawn on screen, and capture reads them there.
//!
//! ## Scene text is located by node path, never by index
//!
//! A `.tscn` node's identity is its path from the scene root —
//! `Menu/Panel/PlayButton` — which is what the editor shows, what a script
//! addresses it by, and what survives the designer reordering the file. The
//! property name completes it. An index into the file's section list would
//! survive a reorder syntactically and point at a different node, which is the
//! failure [`crate::jadwal`]'s header calls the dangerous one.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use taarib_muhawwil_godot::khata::KhataGodot;
use taarib_muhawwil_godot::pck::hawiya::HawiyaMaftuha;
use taarib_muhawwil_godot::pck::tarjama::{
    MawridTarjama, Qeema, Tarjama, TarjamaMurakkaza,
};
use taarib_muhawwil_godot::pck::Mawrid as _;
use taarib_mustalahat::nass::TasnifNass;

use crate::jadwal::{JadwalNusus, MawqiNass};
use crate::rafd::{SababRafd, TaqreerRafd};
use crate::tasnif::{TalabMudkhal, ansha_mudkhal};

/// How deep the walk for loose resources goes.
pub const UMQ_MASH: usize = 24;

/// How many filesystem entries the walk will look at.
pub const AQSA_MALAFAT: usize = 400_000;

/// The largest text file this module will scan.
///
/// Sixteen mebibytes. A `.po` for a large visual novel reaches a few megabytes;
/// past this it is a generated file or a mistake, and either way scanning it line
/// by line is not the right use of a translator's afternoon.
pub const AQSA_MALAF_NASSI: u64 = 16 * 1024 * 1024;

/// The largest binary resource this module will parse.
pub const AQSA_MAWRID: u64 = 64 * 1024 * 1024;

/// How many bytes of binary resources this module will parse, per container.
pub const AQSA_MASH_MAWARID: u64 = 256 * 1024 * 1024;

/// How sure this module is that a `Control`'s own text property is drawn.
///
/// Eighty-two: the format *declares* that the string is the control's rendered
/// text, which is exact, but says nothing about whether it is a button, a title
/// or a line of narration, which is not. A single number covering both would be
/// dishonest in one direction or the other.
pub const THIQAT_KHASIYA: u8 = 82;

/// Properties whose value Godot draws, with what each one is.
///
/// Kept here rather than in [`crate::tasnif`] because two of them are engine
/// nuance rather than general vocabulary: `dialog_text` is an `AcceptDialog`'s
/// message and not a line of dialogue, and the shared classifier's `dialog`
/// keyword would get it exactly backwards. An adapter that knows better says so
/// through the declaration channel instead of teaching the shared table a
/// Godot-only exception.
const KHASAIS_MARSUMA: &[(&str, TasnifNass)] = &[
    ("text", TasnifNass::Qaima),
    ("button_text", TasnifNass::Qaima),
    ("ok_button_text", TasnifNass::Qaima),
    ("cancel_button_text", TasnifNass::Qaima),
    ("placeholder_text", TasnifNass::Tafseer),
    ("tooltip_text", TasnifNass::Tafseer),
    ("hint_tooltip", TasnifNass::Tafseer),
    ("title", TasnifNass::Qaima),
    ("window_title", TasnifNass::Qaima),
    ("dialog_text", TasnifNass::Nizam),
];

/// Pulls every string out of a Godot game.
#[must_use]
pub fn istakhrij(jidhr: &Path) -> (JadwalNusus, TaqreerRafd) {
    let mut jadwal = JadwalNusus::jadeed();
    let mut taqreer = TaqreerRafd::jadeed();
    let masarat = masarat_lil_mash(jidhr);

    // Every message id learned anywhere in the game, and every optimized table
    // held back until they are all in. An `OptimizedTranslation` can only be
    // read through ids it does not itself contain, so probing one before the
    // plain resources have been read would silently find nothing.
    let mut muarrifat: BTreeSet<String> = BTreeSet::new();
    let mut murakkazat: Vec<(String, Option<String>, TarjamaMurakkaza)> = Vec::new();

    for masar in &masarat.tarjama {
        let hawiya = nisbi(jidhr, masar);
        match qira_malaf(masar, AQSA_MAWRID) {
            Ok(bayt) => sajjil_tarjama(
                &mut jadwal,
                &mut taqreer,
                &mut muarrifat,
                &mut murakkazat,
                &hawiya,
                None,
                &bayt,
            ),
            Err(khata) => taqreer.sajjil(hawiya, None, sabab_min_khata(&khata)),
        }
    }

    for masar in &masarat.po {
        let hawiya = nisbi(jidhr, masar);
        match qira_nass(masar) {
            Ok(nass) => {
                sajjil_po(&mut jadwal, &mut taqreer, &mut muarrifat, &hawiya, None, &nass);
            }
            Err(khata) => taqreer.sajjil(hawiya, None, sabab_min_khata(&khata)),
        }
    }

    for masar in &masarat.mashhad {
        let hawiya = nisbi(jidhr, masar);
        match qira_nass(masar) {
            Ok(nass) => {
                if sajjil_mashhad(&mut jadwal, &mut taqreer, &hawiya, None, &nass) == 0 {
                    taqreer.sajjil(hawiya, None, SababRafd::BilaNusus);
                }
            }
            Err(khata) => taqreer.sajjil(hawiya, None, sabab_min_khata(&khata)),
        }
    }

    for masar in &masarat.hazma {
        min_hazma(
            &mut jadwal,
            &mut taqreer,
            &mut muarrifat,
            &mut murakkazat,
            jidhr,
            masar,
        );
    }

    for (hawiya, asl, murakkaza) in murakkazat {
        sajjil_murakkaza(&mut jadwal, &mut taqreer, &muarrifat, &hawiya, asl, &murakkaza);
    }

    (jadwal, taqreer)
}

/// The files a walk of the game's root turned up, sorted by what they are.
#[derive(Debug, Default)]
struct MasaratGodot {
    /// Loose `.translation` resources.
    tarjama: Vec<PathBuf>,
    /// Loose `.po` catalogues.
    po: Vec<PathBuf>,
    /// Loose `.tscn` and `.tres` text resources.
    mashhad: Vec<PathBuf>,
    /// `.pck` packages, and executables with one appended.
    hazma: Vec<PathBuf>,
}

/// Walks the game's root once and sorts what it finds.
fn masarat_lil_mash(jidhr: &Path) -> MasaratGodot {
    let mut masarat = MasaratGodot::default();
    let sayr = walkdir::WalkDir::new(jidhr)
        .max_depth(UMQ_MASH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .take(AQSA_MALAFAT);

    for madkhal in sayr {
        if !madkhal.file_type().is_file() {
            continue;
        }
        let masar = madkhal.into_path();
        let Some(lahiqa) = masar.extension().and_then(std::ffi::OsStr::to_str) else {
            continue;
        };
        match lahiqa.to_ascii_lowercase().as_str() {
            "translation" => masarat.tarjama.push(masar),
            "po" => masarat.po.push(masar),
            "tscn" | "tres" => masarat.mashhad.push(masar),
            "pck" => masarat.hazma.push(masar),
            _ => {}
        }
    }

    masarat.tarjama.sort();
    masarat.po.sort();
    masarat.mashhad.sort();
    masarat.hazma.sort();
    masarat
}

/// Reads everything translatable out of one `.pck`.
fn min_hazma(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muarrifat: &mut BTreeSet<String>,
    murakkazat: &mut Vec<(String, Option<String>, TarjamaMurakkaza)>,
    jidhr: &Path,
    masar: &Path,
) {
    let hawiya = nisbi(jidhr, masar);
    let hazma = match HawiyaMaftuha::iftah(masar) {
        Ok(hazma) => hazma,
        Err(khata) => {
            taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
            return;
        }
    };

    let masarat: Vec<String> =
        hazma.hawiya().madakhil().iter().map(|madkhal| madkhal.masar.clone()).collect();
    if masarat.is_empty() {
        taqreer.sajjil(hawiya, None, SababRafd::BilaNusus);
        return;
    }

    let mut mizaniya = AQSA_MASH_MAWARID;
    let mut mashahid_marfuda = 0_usize;
    let mut bila_nusus = 0_usize;

    for asl in &masarat {
        let lahiqa = lahiqat(asl);
        match lahiqa.as_str() {
            "translation" => {
                let Ok(bayt) = hazma.istakhrij(asl, None) else {
                    continue;
                };
                sajjil_tarjama(
                    jadwal,
                    taqreer,
                    muarrifat,
                    murakkazat,
                    &hawiya,
                    Some(asl.clone()),
                    &bayt,
                );
            }
            "po" => {
                let Ok(bayt) = hazma.istakhrij(asl, None) else {
                    continue;
                };
                match std::str::from_utf8(&bayt) {
                    Ok(nass) => {
                        sajjil_po(jadwal, taqreer, muarrifat, &hawiya, Some(asl.clone()), nass);
                    }
                    Err(khata) => taqreer.sajjil(
                        hawiya.clone(),
                        Some(asl.clone()),
                        SababRafd::Talif {
                            sabab: format!(
                                "a .po that is not valid UTF-8 at byte {}",
                                khata.valid_up_to()
                            ),
                        },
                    ),
                }
            }
            "tscn" | "tres" => {
                let Ok(bayt) = hazma.istakhrij(asl, None) else {
                    continue;
                };
                if let Ok(nass) = std::str::from_utf8(&bayt)
                    && sajjil_mashhad(jadwal, taqreer, &hawiya, Some(asl), nass) == 0
                {
                    bila_nusus = bila_nusus.saturating_add(1);
                }
            }
            "res" | "scn" => {
                if mizaniya == 0 {
                    mashahid_marfuda = mashahid_marfuda.saturating_add(1);
                    continue;
                }
                let Ok(bayt) = hazma.istakhrij(asl, None) else {
                    continue;
                };
                let tul = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
                if tul > AQSA_MAWRID {
                    mashahid_marfuda = mashahid_marfuda.saturating_add(1);
                    continue;
                }
                mizaniya = mizaniya.saturating_sub(tul);
                match sajjil_mawrid_binai(jadwal, taqreer, &hawiya, Some(asl), &bayt) {
                    Some(0) => bila_nusus = bila_nusus.saturating_add(1),
                    Some(_) => {}
                    None => mashahid_marfuda = mashahid_marfuda.saturating_add(1),
                }
            }
            _ => {}
        }
    }

    if bila_nusus > 0 {
        taqreer.sajjil_qira(
            hawiya.clone(),
            0,
            format!("{bila_nusus} Godot resource(s) that were read and hold no text"),
        );
    }

    if mashahid_marfuda > 0 {
        // Aggregated deliberately. Godot's binary scene format packs many
        // resources into one file with variant tags the translation reader does
        // not decode, so a packed scene is refused by construction — and a game
        // has thousands of them. One line saying how many is a report somebody
        // reads; four thousand identical lines is a wall nobody does.
        taqreer.sajjil(
            hawiya,
            None,
            SababRafd::SighaMajhula {
                wujid: format!(
                    "{mashahid_marfuda} binary scene or resource file(s) whose contents are \
                     not the single-resource shape this build's resource reader accepts"
                ),
            },
        );
    }
}

/// Reads one `.translation` resource.
///
/// The plain form is folded into the table immediately. The optimized form is
/// held back — see this module's header — because it can only be read through
/// message ids that come from somewhere else.
fn sajjil_tarjama(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muarrifat: &mut BTreeSet<String>,
    murakkazat: &mut Vec<(String, Option<String>, TarjamaMurakkaza)>,
    hawiya: &str,
    asl: Option<String>,
    bayt: &[u8],
) {
    let mawrid = match MawridTarjama::min_bayt(bayt) {
        Ok(mawrid) => mawrid,
        Err(khata) => {
            taqreer.sajjil(hawiya.to_owned(), asl, sabab_min_khata(&khata));
            return;
        }
    };

    if mawrid.murakkaza() {
        match TarjamaMurakkaza::min_mawrid(&mawrid) {
            Ok(murakkaza) => murakkazat.push((hawiya.to_owned(), asl, murakkaza)),
            Err(khata) => taqreer.sajjil(hawiya.to_owned(), asl, sabab_min_khata(&khata)),
        }
        return;
    }

    if !mawrid.tarjama() {
        // A `.translation` extension over a resource that is not a translation
        // class. Handled as an ordinary resource rather than refused: the string
        // properties are still strings, and the classifier will say what they
        // look like.
        if sajjil_khasais(jadwal, taqreer, hawiya, asl.as_ref(), &mawrid) == 0 {
            taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        }
        return;
    }

    let tarjama = match Tarjama::min_mawrid(&mawrid) {
        Ok(tarjama) => tarjama,
        Err(khata) => {
            taqreer.sajjil(hawiya.to_owned(), asl, sabab_min_khata(&khata));
            return;
        }
    };

    let thaqafa = tarjama.thaqafa().to_owned();
    let mut adad = 0_usize;
    for (muarrif, hadaf) in tarjama.rasail() {
        let _ = muarrifat.insert(muarrif.clone());
        // The message id is what the game looks the string up by; the value is
        // what it draws. An empty value means the id is untranslated in this
        // locale and the engine falls back to the id itself, which is then the
        // text on screen.
        let (khaam, haql) = if hadaf.trim().is_empty() {
            (muarrif.as_str(), "message id")
        } else {
            (hadaf.as_str(), "message")
        };
        if khaam.trim().is_empty() {
            continue;
        }
        adif_risala(jadwal, hawiya, asl.as_ref(), &thaqafa, muarrif, khaam, haql);
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }
    taqreer.sajjil_qira(
        hawiya.to_owned(),
        adad,
        format!("Godot Translation for locale \"{thaqafa}\""),
    );
}

/// Probes one `OptimizedTranslation` with every message id the game gave up
/// elsewhere.
fn sajjil_murakkaza(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muarrifat: &BTreeSet<String>,
    hawiya: &str,
    asl: Option<String>,
    murakkaza: &TarjamaMurakkaza,
) {
    use std::fmt::Write as _;

    if muarrifat.is_empty() {
        taqreer.sajjil(
            hawiya.to_owned(),
            asl,
            SababRafd::SighaMajhula {
                wujid: "an OptimizedTranslation, which stores its source message ids only as \
                        hashes — the translated strings are in the file and the keys they \
                        answer to are not recoverable from it"
                    .to_owned(),
            },
        );
        return;
    }

    let thaqafa = murakkaza.thaqafa().to_owned();
    let mut adad = 0_usize;
    let mut madghut = 0_usize;
    for muarrif in muarrifat {
        match murakkaza.ibhath(muarrif) {
            Ok(Some(khaam)) => {
                if khaam.trim().is_empty() {
                    continue;
                }
                adif_risala(jadwal, hawiya, asl.as_ref(), &thaqafa, muarrif, &khaam, "message");
                adad = adad.saturating_add(1);
            }
            // A miss is the ordinary case: an id from one locale's catalogue
            // simply is not in this table.
            Ok(None) => {}
            // A SMAZ-compressed entry. The reader refuses to expand it rather
            // than guessing at the codebook, so the string is lost and counted
            // rather than replaced with something plausible.
            Err(_) => madghut = madghut.saturating_add(1),
        }
    }

    if adad == 0 {
        taqreer.sajjil(
            hawiya.to_owned(),
            asl,
            SababRafd::SighaMajhula {
                wujid: format!(
                    "an OptimizedTranslation for locale \"{thaqafa}\" that answered to none \
                     of the {} message id(s) this game's other catalogues declare",
                    muarrifat.len()
                ),
            },
        );
        return;
    }

    let mut wasf = format!(
        "Godot OptimizedTranslation for locale \"{thaqafa}\", read by probing it with the \
         message ids from this game's plain catalogues"
    );
    if madghut > 0 {
        let _ = write!(wasf, "; {madghut} entr(y/ies) are SMAZ-compressed and were not expanded");
    }
    taqreer.sajjil_qira(hawiya.to_owned(), adad, wasf);
}

/// Folds one translated message into the table.
fn adif_risala(
    jadwal: &mut JadwalNusus,
    hawiya: &str,
    asl: Option<&String>,
    thaqafa: &str,
    muarrif: &str,
    khaam: &str,
    haql: &'static str,
) {
    let mawqi = MawqiNass {
        hawiya: hawiya.to_owned(),
        asl: asl.cloned(),
        mawqi: format!("{thaqafa}/{muarrif}"),
        haql: Some(haql.to_owned()),
        // The locale joins the engine's key because the same id carries
        // different text in every locale, and one identity for all of them would
        // let the fold keep whichever was read first.
        miftah_muharrik: Some(format!("{thaqafa}\u{1}{muarrif}")),
    };
    let talab = TalabMudkhal::jadeed(mawqi, khaam).bi_nizam_tawtin();
    jadwal.adif(ansha_mudkhal(talab));
}

/// Harvests the string properties of a binary resource that is not a
/// translation.
///
/// [`None`] when the resource could not be read at all — a packed scene cannot
/// be, because the reader accepts single-resource files and refuses variant tags
/// outside the set a translation uses. The caller aggregates those rather than
/// writing one refusal per file, because a game has thousands of them and a
/// report nobody reads is a report that does not exist.
fn sajjil_mawrid_binai(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    asl: Option<&String>,
    bayt: &[u8],
) -> Option<usize> {
    let mawrid = MawridTarjama::min_bayt(bayt).ok()?;
    Some(sajjil_khasais(jadwal, taqreer, hawiya, asl, &mawrid))
}

/// Records every string a resource's properties hold, and answers how many.
///
/// Records a read only when it found something. A resource with no strings is
/// counted by its caller, which knows whether one such file is worth a line of
/// its own or whether four thousand of them are worth one.
fn sajjil_khasais(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    asl: Option<&String>,
    mawrid: &MawridTarjama,
) -> usize {
    let naw = mawrid.naw().to_owned();
    let mut adad = 0_usize;
    for khasiya in mawrid.khasais() {
        adad = adad.saturating_add(adif_qeema(
            jadwal,
            hawiya,
            asl,
            &naw,
            &khasiya.ism,
            &khasiya.qeema,
        ));
    }
    if adad > 0 {
        taqreer.sajjil_qira(hawiya.to_owned(), adad, format!("Godot {naw} resource"));
    }
    adad
}

/// Adds every string inside one property value, and answers how many.
///
/// Recursive over arrays and dictionaries, because Godot resources routinely
/// keep a list of lines in a `PackedStringArray` and a table of them in a
/// `Dictionary`, and a walker that only looked at top-level strings would miss
/// every one of those. The depth is bounded by the reader, which refuses nesting
/// past its own limit before this ever sees the value.
fn adif_qeema(
    jadwal: &mut JadwalNusus,
    hawiya: &str,
    asl: Option<&String>,
    naw: &str,
    masar: &str,
    qeema: &Qeema,
) -> usize {
    match qeema {
        Qeema::Nass(nass) | Qeema::IsmNass(nass) => {
            if nass.trim().is_empty() {
                return 0;
            }
            adif_khasiya(jadwal, hawiya, asl, naw, masar, nass);
            1
        }
        Qeema::Nusus(nusus) => {
            let mut adad = 0_usize;
            for (khana, nass) in nusus.iter().enumerate() {
                if nass.trim().is_empty() {
                    continue;
                }
                adif_khasiya(jadwal, hawiya, asl, naw, &format!("{masar}[{khana}]"), nass);
                adad = adad.saturating_add(1);
            }
            adad
        }
        Qeema::Masfufa { anasir, .. } => {
            let mut adad = 0_usize;
            for (khana, unsur) in anasir.iter().enumerate() {
                let dakhili = format!("{masar}[{khana}]");
                adad = adad.saturating_add(adif_qeema(
                    jadwal, hawiya, asl, naw, &dakhili, unsur,
                ));
            }
            adad
        }
        Qeema::Qamus { azwaj, .. } => {
            let mut adad = 0_usize;
            for (miftah, qeemat_zawj) in azwaj {
                // The key names the entry, so it is the structural location
                // rather than a string of its own. A dictionary key that is not
                // text has no name to give and the pair is addressed by nothing,
                // which is the one case this declines rather than inventing an
                // index for.
                let Some(ism) = miftah.nass() else {
                    continue;
                };
                let dakhili = format!("{masar}/{ism}");
                adad = adad.saturating_add(adif_qeema(
                    jadwal,
                    hawiya,
                    asl,
                    naw,
                    &dakhili,
                    qeemat_zawj,
                ));
            }
            adad
        }
        Qeema::Faragh
        | Qeema::Mantiqi(_)
        | Qeema::Sahih(_)
        | Qeema::SahihTawil(_)
        | Qeema::Bayt(_)
        | Qeema::Sahihat(_) => 0,
    }
}

/// Folds one resource property into the table.
fn adif_khasiya(
    jadwal: &mut JadwalNusus,
    hawiya: &str,
    asl: Option<&String>,
    naw: &str,
    masar: &str,
    khaam: &str,
) {
    let mawqi = MawqiNass {
        hawiya: hawiya.to_owned(),
        asl: asl.cloned(),
        mawqi: naw.to_owned(),
        haql: Some(masar.to_owned()),
        // A resource property has no engine-supplied key: the developer named
        // the property, not the string, and two resources of one class share
        // every property name. Identity is derived, which is the right answer
        // and the reason a scene string re-matches less well than a `tr()` one.
        miftah_muharrik: None,
    };
    let mut talab = TalabMudkhal::jadeed(mawqi, khaam);
    if let Some((_, tasnif)) = KHASAIS_MARSUMA.iter().find(|(ism, _)| *ism == masar) {
        talab = talab.bi_tasrih(*tasnif, THIQAT_KHASIYA);
    }
    jadwal.adif(ansha_mudkhal(talab));
}

// ---------------------------------------------------------------------------
// gettext `.po`
//
// A line-oriented text format, scanned rather than parsed. There is no `.po`
// reader in this workspace and a `.po` is not a container, so this is the one
// place it can live. The scanner handles the three forms Godot's importer
// produces — a plain entry, a contextual entry, and a plural entry — and treats
// anything else as a comment, which is what gettext itself does with a directive
// it does not know.
// ---------------------------------------------------------------------------

/// One catalogue entry.
#[derive(Debug, Default)]
struct MadkhalPo {
    /// The `msgctxt`, when the entry has one.
    siyaq: Option<String>,
    /// The `msgid`, which is Godot's message id.
    muarrif: String,
    /// The `msgstr` values, in order. Index zero for a singular entry.
    tarjamat: Vec<String>,
}

/// Reads a `.po` catalogue.
///
/// The header entry — the one with an empty `msgid` — is kept separately because
/// its `msgstr` is the header block, and the `Language:` field in it is the only
/// place a `.po` names its own locale.
fn madakhil_po(nass: &str) -> (Vec<MadkhalPo>, Option<String>) {
    let mut madakhil: Vec<MadkhalPo> = Vec::new();
    let mut hali = MadkhalPo::default();
    let mut fi_madkhal = false;
    let mut lugha: Option<String> = None;
    // Which field the continuation lines append to.
    let mut hadaf = HaqlPo::Laashay;

    for satr in nass.lines() {
        let munaqqa = satr.trim();
        if munaqqa.is_empty() {
            if fi_madkhal {
                khatim_madkhal(&mut madakhil, &mut lugha, std::mem::take(&mut hali));
                fi_madkhal = false;
            }
            hadaf = HaqlPo::Laashay;
            continue;
        }
        if munaqqa.starts_with('#') {
            hadaf = HaqlPo::Laashay;
            continue;
        }

        if let Some(baqi) = munaqqa.strip_prefix("msgctxt ") {
            if fi_madkhal && hadaf.badat_madkhalan() {
                khatim_madkhal(&mut madakhil, &mut lugha, std::mem::take(&mut hali));
            }
            fi_madkhal = true;
            hadaf = HaqlPo::Siyaq;
            hali.siyaq = Some(awwal_iqtibas(baqi));
            continue;
        }
        if munaqqa.starts_with("msgid_plural ") {
            // The plural form is a second source string for the same entry.
            // Godot never reads it — its `tr_n` path uses the plural rules on
            // the singular id — so it is skipped rather than recorded as a
            // second string a translator would be asked to translate twice.
            hadaf = HaqlPo::Laashay;
            continue;
        }
        if let Some(baqi) = munaqqa.strip_prefix("msgid ") {
            // A `msgstr` already seen means the previous entry is finished. That
            // is the reliable boundary: catalogues generated without blank lines
            // between entries are common, and keying the boundary on the blank
            // line alone would fold two entries into one and lose the first.
            if fi_madkhal && matches!(hadaf, HaqlPo::Tarjama) {
                khatim_madkhal(&mut madakhil, &mut lugha, std::mem::take(&mut hali));
            }
            fi_madkhal = true;
            hadaf = HaqlPo::Muarrif;
            hali.muarrif = awwal_iqtibas(baqi);
            continue;
        }
        if let Some(baqi) = munaqqa.strip_prefix("msgstr") {
            hadaf = HaqlPo::Tarjama;
            hali.tarjamat.push(awwal_iqtibas(baqi));
            continue;
        }

        // A bare quoted line continues whatever came before it.
        if munaqqa.starts_with('"') {
            let qitaa = awwal_iqtibas(munaqqa);
            match hadaf {
                HaqlPo::Siyaq => {
                    if let Some(siyaq) = hali.siyaq.as_mut() {
                        siyaq.push_str(&qitaa);
                    }
                }
                HaqlPo::Muarrif => hali.muarrif.push_str(&qitaa),
                HaqlPo::Tarjama => {
                    if let Some(akhir) = hali.tarjamat.last_mut() {
                        akhir.push_str(&qitaa);
                    }
                }
                HaqlPo::Laashay => {}
            }
        }
    }
    if fi_madkhal {
        khatim_madkhal(&mut madakhil, &mut lugha, hali);
    }
    (madakhil, lugha)
}

/// Which field a continuation line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HaqlPo {
    /// Nothing is open.
    Laashay,
    /// A `msgctxt`.
    Siyaq,
    /// A `msgid`.
    Muarrif,
    /// A `msgstr`.
    Tarjama,
}

impl HaqlPo {
    /// Whether a field of this kind means an entry is already under way.
    const fn badat_madkhalan(self) -> bool {
        !matches!(self, Self::Laashay)
    }
}

/// Files a finished entry, pulling the locale out of the header entry.
fn khatim_madkhal(
    madakhil: &mut Vec<MadkhalPo>,
    lugha: &mut Option<String>,
    madkhal: MadkhalPo,
) {
    if madkhal.muarrif.is_empty() && madkhal.siyaq.is_none() {
        if lugha.is_none() {
            *lugha = madkhal
                .tarjamat
                .first()
                .and_then(|raas| lugha_min_tarwisa(raas))
                .filter(|ism| !ism.is_empty());
        }
        return;
    }
    madakhil.push(madkhal);
}

/// The `Language:` field of a `.po` header block.
///
/// The header's `msgstr` arrives with its escapes already resolved, so the field
/// values are separated by real newlines and `.lines()` is the right split. An
/// earlier version trimmed a trailing `\n` off the value here, which quietly
/// turned the locale `en` into `e`.
fn lugha_min_tarwisa(tarwisa: &str) -> Option<String> {
    tarwisa
        .lines()
        .find_map(|satr| satr.trim().strip_prefix("Language:"))
        .map(|qeema| qeema.trim().to_owned())
}

/// Reads one `.po` catalogue into the table.
fn sajjil_po(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muarrifat: &mut BTreeSet<String>,
    hawiya: &str,
    asl: Option<String>,
    nass: &str,
) {
    let (madakhil, lugha) = madakhil_po(nass);
    if madakhil.is_empty() {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }

    // The locale comes from the header when the file declares one and from the
    // file's own name when it does not — `fr.po`, `game.fr.po`. Never guessed
    // beyond those two, because a wrong locale would collapse two languages onto
    // one identity, and an unnamed locale merely makes the key longer.
    let thaqafa = lugha
        .or_else(|| thaqafa_min_ism(asl.as_deref().unwrap_or(hawiya)))
        .unwrap_or_else(|| "an unnamed locale".to_owned());

    let mut adad = 0_usize;
    for madkhal in &madakhil {
        let _ = muarrifat.insert(madkhal.muarrif.clone());
        let tarjama = madkhal.tarjamat.first().map(String::as_str).unwrap_or_default();
        let (khaam, haql) = if tarjama.trim().is_empty() {
            (madkhal.muarrif.as_str(), "msgid")
        } else {
            (tarjama, "msgstr")
        };
        if khaam.trim().is_empty() {
            continue;
        }

        let mut muarrif = String::with_capacity(madkhal.muarrif.len() + 16);
        if let Some(siyaq) = madkhal.siyaq.as_deref() {
            muarrif.push_str(siyaq);
            muarrif.push('\u{4}');
        }
        muarrif.push_str(&madkhal.muarrif);

        let mawqi = MawqiNass {
            hawiya: hawiya.to_owned(),
            asl: asl.clone(),
            mawqi: format!("{thaqafa}/{muarrif}"),
            haql: Some(haql.to_owned()),
            // gettext's own key is the context and the id joined by U+0004,
            // which is exactly how gettext hashes them. Using the same joiner
            // means an id Taarib derived and an id the tooling derived are the
            // same string.
            miftah_muharrik: Some(format!("{thaqafa}\u{1}{muarrif}")),
        };
        jadwal.adif(ansha_mudkhal(TalabMudkhal::jadeed(mawqi, khaam).bi_nizam_tawtin()));
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }
    taqreer.sajjil_qira(
        hawiya.to_owned(),
        adad,
        format!("gettext .po catalogue for locale \"{thaqafa}\""),
    );
}

// ---------------------------------------------------------------------------
// Godot's text scene and resource form
// ---------------------------------------------------------------------------

/// Reads a `.tscn` or `.tres` and folds its text properties into the table.
///
/// ## What this scanner deliberately does not do
///
/// Only single-line property values are read. Godot writes a multi-line value
/// for arrays of resources, for embedded sub-resource bodies and for very long
/// `PackedStringArray`s, and following those correctly means tracking bracket
/// depth through a value grammar that also contains strings containing brackets.
/// A scanner that got that wrong would attribute one node's text to another,
/// which is worse than not reading it: the wrong write-back destination produces
/// a game with two labels swapped and no error anywhere.
///
/// The limit is recorded rather than hidden — a file whose text is entirely in
/// multi-line values contributes nothing, answers zero, and its caller says so.
fn sajjil_mashhad(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    asl: Option<&String>,
    nass: &str,
) -> usize {
    let mut mawqi_hali = String::new();
    let mut jidhr_mashhad: Option<String> = None;
    let mut adad = 0_usize;

    for satr in nass.lines() {
        let munaqqa = satr.trim();
        if munaqqa.is_empty() || munaqqa.starts_with(';') {
            continue;
        }

        if let Some(jism) = munaqqa.strip_prefix('[').and_then(|juz| juz.strip_suffix(']')) {
            let (naw, sifat) = qism_mashhad(jism);
            mawqi_hali = masar_qism(naw, &sifat, &mut jidhr_mashhad);
            continue;
        }

        let Some((ism, qeema)) = munaqqa.split_once('=') else {
            continue;
        };
        let ism = ism.trim();
        let qeema = qeema.trim();
        if ism.is_empty() || mawqi_hali.is_empty() || qeema_marjaiya(qeema) {
            continue;
        }

        for (khana, khaam) in iqtibasat(qeema).into_iter().enumerate() {
            if khaam.trim().is_empty() {
                continue;
            }
            let haql = if khana == 0 { ism.to_owned() } else { format!("{ism}[{khana}]") };
            let mawqi = MawqiNass {
                hawiya: hawiya.to_owned(),
                asl: asl.cloned(),
                mawqi: mawqi_hali.clone(),
                haql: Some(haql),
                miftah_muharrik: None,
            };
            let mut talab = TalabMudkhal::jadeed(mawqi, &khaam);
            if let Some((_, tasnif)) = KHASAIS_MARSUMA.iter().find(|(marsuma, _)| *marsuma == ism)
            {
                talab = talab.bi_tasrih(*tasnif, THIQAT_KHASIYA);
            }
            jadwal.adif(ansha_mudkhal(talab));
            adad = adad.saturating_add(1);
        }
    }

    if adad > 0 {
        taqreer.sajjil_qira(hawiya.to_owned(), adad, "Godot text scene or resource");
    }
    adad
}

/// Whether a property value is one of the format's own reference forms.
///
/// `ExtResource("2_a8x1w")`, `SubResource("Label_9")`, `NodePath("Menu/Play")`
/// and `&"metadata_key"` all carry a quoted argument, and in none of them is
/// that argument text a player reads: they are ids, node addresses and
/// `StringName`s. Skipping them is not a heuristic about what the string looks
/// like — it is reading the value's own constructor, which the format writes
/// explicitly.
fn qeema_marjaiya(qeema: &str) -> bool {
    const MARAJI: &[&str] = &[
        "ExtResource(",
        "SubResource(",
        "Resource(",
        "NodePath(",
        "StringName(",
        "NodePath (",
        "preload(",
        "load(",
    ];
    qeema.starts_with("&\"") || MARAJI.iter().any(|marja| qeema.starts_with(marja))
}

/// Splits a section header into its kind and its attributes.
///
/// `node name="Title" type="Label" parent="Menu"` yields `node` and the three
/// pairs.
/// An unquoted value — `load_steps=3` in a `[gd_scene]` header — is skipped
/// rather than paired with the next quoted run it can find. Pairing it would
/// attach `uid="uid://..."`'s value to `load_steps`'s name and, in a section
/// where that mattered, would put a node under the wrong parent.
fn qism_mashhad(jism: &str) -> (&str, Vec<(String, String)>) {
    let (naw, mut baqi) = jism.split_once(char::is_whitespace).unwrap_or((jism, ""));
    let mut sifat = Vec::new();

    while let Some(mawqi) = baqi.find('=') {
        let Some(ism_khaam) = baqi.get(..mawqi) else {
            break;
        };
        let Some(badu) = baqi.get(mawqi.saturating_add(1)..) else {
            break;
        };
        let mudakhkhal = badu.trim_start();
        let Some(jism_qeema) = mudakhkhal.strip_prefix('"') else {
            baqi = badu;
            continue;
        };
        let Some(tul) = tul_iqtibas(jism_qeema) else {
            break;
        };
        // The last whitespace-separated token, so that a skipped unquoted value
        // does not leave its own text glued to the front of the next name.
        let ism = ism_khaam
            .trim()
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or(ism_khaam)
            .to_owned();
        sifat.push((ism, fukk_iqtibas(jism_qeema.get(..tul).unwrap_or_default())));
        baqi = jism_qeema.get(tul.saturating_add(1)..).unwrap_or_default();
    }
    (naw, sifat)
}

/// The structural location a section header names.
///
/// For a node that is the **absolute** path from the scene root —
/// `Main/Menu/PlayButton` — which is what the editor shows, what a script
/// addresses it by, and what survives the designer reordering the file.
///
/// The root is tracked rather than inferred per line, and that is not fussiness.
/// Godot omits `parent` on the root and writes `parent="."` on every direct
/// child of it, so a rule that read those two the same way would give a root
/// named `Panel` and a child of the root named `Panel` the same location — and
/// then Phase 14 would write one node's Arabic into the other.
fn masar_qism(naw: &str, sifat: &[(String, String)], jidhr: &mut Option<String>) -> String {
    let jid = |matlub: &str| {
        sifat.iter().find(|(ism, _)| ism == matlub).map(|(_, qeema)| qeema.as_str())
    };
    match naw {
        "node" => {
            let ism = jid("name").unwrap_or("");
            match jid("parent") {
                None => {
                    // The first parentless node is the scene root. A second one
                    // does not occur in a file the engine wrote, and if it did,
                    // keeping the first is the reading that leaves every path
                    // already emitted correct.
                    if jidhr.is_none() {
                        *jidhr = Some(ism.to_owned());
                    }
                    ism.to_owned()
                }
                Some(walid) => {
                    let asas = jidhr.as_deref().unwrap_or("");
                    if walid == "." {
                        format!("{asas}/{ism}")
                    } else {
                        format!("{asas}/{walid}/{ism}")
                    }
                }
            }
        }
        "sub_resource" | "ext_resource" => {
            // A sub-resource's id is written by the editor and kept across
            // saves, which makes it the only stable name one has. It is not the
            // engine's *localization* key, so it does not become a
            // `miftah_muharrik`; it is a structural location and nothing more.
            let muarrif = jid("id").unwrap_or("");
            let sanf = jid("type").unwrap_or(naw);
            format!("{naw}:{sanf}:{muarrif}")
        }
        "resource" => "resource".to_owned(),
        "editable" | "connection" | "gd_scene" | "gd_resource" => String::new(),
        akhar => akhar.to_owned(),
    }
}

/// Every double-quoted run on a line, unescaped.
///
/// Written by hand rather than with a regular expression because Godot escapes
/// with a backslash, including a backslash before a quotation mark, and a
/// scanner that got that wrong would split one string into two at an escaped
/// quote and offer a translator half a sentence.
fn iqtibasat(satr: &str) -> Vec<String> {
    let mut natija = Vec::new();
    let mut baqi = satr;
    while let Some(bidaya) = baqi.find('"') {
        let jism = baqi.get(bidaya.saturating_add(1)..).unwrap_or_default();
        let Some(tul) = tul_iqtibas(jism) else {
            break;
        };
        natija.push(fukk_iqtibas(jism.get(..tul).unwrap_or_default()));
        baqi = jism.get(tul.saturating_add(1)..).unwrap_or_default();
    }
    natija
}

/// The first quoted run on a line, or the empty string when there is none.
fn awwal_iqtibas(satr: &str) -> String {
    iqtibasat(satr).into_iter().next().unwrap_or_default()
}

/// How many bytes of `jism` lie before the closing quotation mark.
///
/// [`None`] when the run never closes, which is a truncated file and is reported
/// by the caller getting nothing rather than by inventing a terminator.
fn tul_iqtibas(jism: &str) -> Option<usize> {
    let mut haarib = false;
    for (mawqi, harf) in jism.char_indices() {
        if haarib {
            haarib = false;
            continue;
        }
        match harf {
            '\\' => haarib = true,
            '"' => return Some(mawqi),
            _ => {}
        }
    }
    None
}

/// Turns the body of a quoted run into the text it denotes.
fn fukk_iqtibas(jism: &str) -> String {
    let mut natija = String::with_capacity(jism.len());
    let mut huruf = jism.chars();
    while let Some(harf) = huruf.next() {
        if harf != '\\' {
            natija.push(harf);
            continue;
        }
        match huruf.next() {
            Some('n') => natija.push('\n'),
            Some('t') => natija.push('\t'),
            Some('r') => natija.push('\r'),
            Some('0') => natija.push('\0'),
            // Everything else stands for itself, which is what both gettext and
            // Godot's own parser do with an escape they do not define.
            Some(akhar) => natija.push(akhar),
            None => natija.push('\\'),
        }
    }
    natija
}

/// The locale a file name declares, when it declares one.
///
/// `fr.po`, `ui.fr.po` and `ui.fr_FR.po` all name a locale in the component
/// before the extension, and the shape of a locale tag is what makes it safe to
/// read one out of a filename: two or three letters, optionally a separator and
/// a two-to-four character region. `messages.po` does not match that shape and
/// yields [`None`], because calling its locale `messages` would put a fictitious
/// locale into every identity in the file.
fn thaqafa_min_ism(masar: &str) -> Option<String> {
    let ism = masar.rsplit('/').next().unwrap_or(masar);
    if !ism.to_ascii_lowercase().ends_with(".po") {
        return None;
    }
    let jidhr = ism.get(..ism.len().checked_sub(3)?)?;
    let murashah = jidhr.rsplit('.').next().unwrap_or(jidhr);
    shakl_thaqafa(murashah).then(|| murashah.to_owned())
}

/// Whether a token has the shape of a BCP 47 locale tag Godot would accept.
fn shakl_thaqafa(murashah: &str) -> bool {
    let (lugha, iqlim) = match murashah.split_once(['_', '-']) {
        Some((lugha, iqlim)) => (lugha, Some(iqlim)),
        None => (murashah, None),
    };
    let tul_lugha = lugha.chars().count();
    if !(2..=3).contains(&tul_lugha) || !lugha.chars().all(|harf| harf.is_ascii_alphabetic()) {
        return false;
    }
    match iqlim {
        None => true,
        Some(iqlim) => {
            let tul = iqlim.chars().count();
            (2..=4).contains(&tul) && iqlim.chars().all(|harf| harf.is_ascii_alphanumeric())
        }
    }
}

/// The lowercase extension of a container-relative path.
fn lahiqat(masar: &str) -> String {
    match masar.rsplit_once('.') {
        Some((_, lahiqa)) if !lahiqa.contains('/') => lahiqa.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// A path relative to the game's root, with forward slashes on every platform.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    let juz = masar.strip_prefix(jidhr).unwrap_or(masar);
    juz.components()
        .map(|qism| qism.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Reads a whole file, refusing one above `saqf` before reserving anything.
fn qira_malaf(masar: &Path, saqf: u64) -> Result<Vec<u8>, KhataGodot> {
    let bayanat = std::fs::metadata(masar)
        .map_err(|sabab| KhataGodot::KhataMalaf { masar: masar.to_path_buf(), sabab })?;
    if bayanat.len() > saqf {
        return Err(KhataGodot::HajmMufrit {
            haql: "a loose Godot resource",
            qeema: bayanat.len(),
            saqf,
        });
    }
    std::fs::read(masar)
        .map_err(|sabab| KhataGodot::KhataMalaf { masar: masar.to_path_buf(), sabab })
}

/// Reads a whole text file, refusing one that is not valid UTF-8.
///
/// Refused rather than decoded lossily, because a replacement character in a
/// source string is a character a translator will faithfully carry into the
/// translation.
fn qira_nass(masar: &Path) -> Result<String, KhataGodot> {
    let bayt = qira_malaf(masar, AQSA_MALAF_NASSI)?;
    let munaqqa = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bayt);
    std::str::from_utf8(munaqqa).map(str::to_owned).map_err(|khata| {
        KhataGodot::NassGhayrSalih {
            fahras: 0,
            mawqi: u32::try_from(khata.valid_up_to()).unwrap_or(u32::MAX),
        }
    })
}

/// Turns a reader's refusal into the reason the report shows.
fn sabab_min_khata(khata: &KhataGodot) -> SababRafd {
    match khata {
        KhataGodot::KhataMalaf { sabab, .. } => {
            SababRafd::TaadhurQira { sabab: sabab.to_string() }
        }
        KhataGodot::PckMushaffar { .. } => SababRafd::Mushaffar {
            wasf: "an encrypted PCK, and no key was supplied".to_owned(),
        },
        KhataGodot::MiftahGhayrSalih { sabab, .. } => {
            SababRafd::Mushaffar { wasf: (*sabab).to_owned() }
        }
        KhataGodot::IsdarGhayrMadum { wujid, aqsa } => SababRafd::IsdarGhayrMadum {
            sigha: "Godot PCK".to_owned(),
            wujid: wujid.to_string(),
            madum: format!("up to and including {aqsa}"),
        },
        KhataGodot::HajmMufrit { haql, qeema, saqf } => {
            SababRafd::TajawuzHadd { hadd: (*haql).to_owned(), qeema: *qeema, saqf: *saqf }
        }
        KhataGodot::SihrGhayrMutabaq { .. } => SababRafd::SighaMajhula {
            wujid: "a file that does not carry Godot's package or resource magic".to_owned(),
        },
        KhataGodot::DaghtMajhul { naw } => SababRafd::SighaMajhula {
            wujid: format!(
                "an entry compressed with Godot mode {naw}, which this build cannot expand"
            ),
        },
        akhar => SababRafd::Talif { sabab: akhar.to_string() },
    }
}
