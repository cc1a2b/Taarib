//! نصوص — the five script engines, normalized.
//!
//! An adapter over the Phase 10 readers in `taarib_muhawwil_nusus`. The RPG
//! Maker JSON scanner, the Ruby `Marshal` reader, the Ren'Py `.rpy` scanner and
//! `.rpyc` pickle walker, the RGSSAD and `.rpa` archives, the GameMaker `FORM`
//! container and the asar reader all live there. Nothing in this file parses a
//! container.
//!
//! ## What each engine gives up, and what it costs
//!
//! | engine | the location that survives a rebuild | an engine-supplied key? |
//! | --- | --- | --- |
//! | RPG Maker MV/MZ | file, event id, page, command index | no |
//! | Ren'Py | statement kind and speaker, in a source file | **yes**, when the game ships a translation |
//! | VX Ace | script file, class, instance variable | no |
//! | GameMaker | the string pool, and what references the entry | no |
//! | Electron | a JSON pointer, or the script the literal is in | no |
//!
//! Only Ren'Py has an identity of its own, and only conditionally: Ren'Py
//! derives a statement identifier from a hash of the statement's regenerated
//! source, which only the engine can compute — so this module never invents one.
//! It reads them out of a translation the game already ships, where they are
//! exact, and leaves [`MawqiNass::miftah_muharrik`] empty when the game ships
//! none, under the ambiguity rule that keeps that recovery
//! honest — the Ren'Py section below states it.
//!
//! ## RPG Maker is the one engine whose data says what a string is
//!
//! Every other engine here hands over a string and leaves the classification to
//! be inferred from a field name. RPG Maker hands over an event command *code*,
//! and the code is a definition rather than a hint: 101 is a message header, 401
//! is a line of the message, 102 is a choice list, 402 is the branch for one
//! chosen option, 405 is a line of scrolling text. There is no inference in
//! reading 401 as dialogue, so those classifications carry
//! [`crate::tasnif::THIQAT_BUNYA`] and no other engine in this file does.
//!
//! ## Escape codes are lifted here the same way they are everywhere else
//!
//! The RPG Maker reader already splits `\C[1]` and `\V[3]` out into its own span
//! model, and this module deliberately does **not** carry that model forward. It
//! passes the raw string to [`crate::tasnif::ansha_mudkhal`], which lifts markup
//! through `taarib_saff::nasq` like every other adapter in this crate. The two
//! models exist for different jobs — the reader's is for splicing bytes back
//! into a JSON literal, the vocabulary's is for showing a translator what must
//! survive — and converting between them here would be a third opinion about
//! what `\V[3]` means.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use taarib_muhawwil_nusus::khata::KhataNusus;
use taarib_muhawwil_nusus::{electron, gamemaker, renpy, rpgmaker, vxace};
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_mustalahat::nass::TasnifNass;

use crate::jadwal::{JadwalNusus, MawqiNass};
use crate::rafd::{SababRafd, TaqreerRafd};
use crate::tasnif::{THIQAT_BUNYA, TalabMudkhal, ansha_mudkhal};

/// How deep the walk for script and data files goes.
pub const UMQ_MASH: usize = 24;

/// How many filesystem entries the walk will look at.
pub const AQSA_MALAFAT: usize = 400_000;

/// The largest text file this module will scan.
pub const AQSA_MALAF_NASSI: u64 = 32 * 1024 * 1024;

/// The largest `Marshal` stream this module will read.
pub const AQSA_RVDATA: u64 = 256 * 1024 * 1024;

/// The largest GameMaker container this module will read.
pub const AQSA_HAWIYA: u64 = 2 * 1024 * 1024 * 1024;

/// How many neighbouring lines are kept beside a line of dialogue.
///
/// Two on each side. Machine translation of a line without its neighbours
/// produces the wrong pronoun and the wrong register roughly as often as it
/// produces the right one; five lines is enough context to fix that and short
/// enough that a translator reading the table is not reading the whole scene.
pub const JIWAR_ATRAF: usize = 2;

/// How sure a `System.json` term's own position makes this module.
///
/// Ninety-two rather than [`THIQAT_BUNYA`], because the RPG Maker editor's own
/// vocabulary tree states what each subtree is for — `terms.messages` really are
/// the system messages and `terms.commands` really are the menu commands — but
/// the tree is a convention of the editor rather than a tagged field, and a
/// plugin can and does add its own keys under it.
pub const THIQAT_MUSTALAH: u8 = 92;

/// The ceiling on a classification derived from a script literal.
///
/// Fifty. The Electron extractor finds string literals in JavaScript with a
/// documented heuristic — it says so itself — and no classification of a string
/// can be more certain than the extraction that produced it. Capping here rather
/// than in [`crate::tasnif`] keeps that a property of *this source* instead of a
/// rule the classifier would then have to apply to sources that do not need it.
pub const THIQAT_BARMAJI: u8 = 50;

/// How sure a GameMaker reference census makes this module about an asset name.
///
/// Ninety. A string reached only from a chunk that holds asset records is that
/// asset's name, which the container states by where the pointer lives rather
/// than by anything about the text.
pub const THIQAT_MARJA_ASL: u8 = 90;

/// How sure a GameMaker census makes this module about an unreferenced string.
///
/// Sixty-two, and lower than [`THIQAT_MARJA_ASL`] on purpose. An entry nothing
/// points at is usually a leftover from a deleted asset — but it is *sometimes*
/// a string the census failed to attribute, and this number is the difference
/// between the two possibilities rather than a claim to have distinguished them.
pub const THIQAT_MARJA_YATIM: u8 = 62;

/// Pulls every string out of a game built on one of the five script engines.
///
/// `aila` decides which reader runs. An engine this module does not cover
/// returns an empty table and a report saying which module does, because
/// answering "no strings" for a Unity game would be a lie the caller cannot
/// tell from the truth.
#[must_use]
pub fn istakhrij(jidhr: &Path, aila: AilatMuharrik) -> (JadwalNusus, TaqreerRafd) {
    let mut jadwal = JadwalNusus::jadeed();
    let mut taqreer = TaqreerRafd::jadeed();

    match aila {
        AilatMuharrik::RpgMakerMv | AilatMuharrik::RpgMakerMz => {
            min_rpgmaker(&mut jadwal, &mut taqreer, jidhr);
        }
        AilatMuharrik::RpgMakerVxAce => min_vxace(&mut jadwal, &mut taqreer, jidhr),
        AilatMuharrik::Renpy => min_renpy(&mut jadwal, &mut taqreer, jidhr),
        AilatMuharrik::GameMaker => min_gamemaker(&mut jadwal, &mut taqreer, jidhr),
        AilatMuharrik::Electron => min_electron(&mut jadwal, &mut taqreer, jidhr),
        AilatMuharrik::Unity | AilatMuharrik::Unreal | AilatMuharrik::Godot => {
            taqreer.sajjil(
                jidhr.display().to_string(),
                None,
                SababRafd::SighaMajhula {
                    wujid: format!(
                        "a {} game, which the script-engine extractor does not read — it is \
                         read by this crate's own module for that engine",
                        aila.ism()
                    ),
                },
            );
        }
        AilatMuharrik::Majhul => {
            taqreer.sajjil(
                jidhr.display().to_string(),
                None,
                SababRafd::SighaMajhula {
                    wujid: "a game whose engine was not identified, so no reader was chosen"
                        .to_owned(),
                },
            );
        }
    }

    (jadwal, taqreer)
}

// ---------------------------------------------------------------------------
// RPG Maker MV and MZ
// ---------------------------------------------------------------------------

/// Reads an RPG Maker MV or MZ project.
///
/// ## Why the location contains an index, and why that is safe here
///
/// A dialogue line's location is `data/Map001.json` plus the JSON path
/// `/events/3/pages/0/list/12/parameters/0`. Three of those numbers are stable
/// identities the editor itself preserves — `events/3` is event id 3, `pages/0`
/// is the first page tab, `parameters/0` is the command's first argument — and
/// one of them, `list/12`, is a position in the page's command list, which
/// renumbers when a designer inserts a line above it.
///
/// [`crate::jadwal`]'s header calls a bare array index the *more dangerous* of
/// the two bad choices, because it survives a renumber syntactically and the
/// patch then writes the wrong Arabic into every subsequent line. That failure
/// requires identity to be a function of position **alone**. It is not: for a
/// string with no engine-supplied key, [`MawqiNass::huwiya`] hashes the
/// container, the location *and the source text together*. A renumbered line
/// therefore produces a different identity, comes back as removed-and-added
/// rather than as a silent mismatch, and its translation is kept and marked
/// orphaned rather than applied to somebody else's line.
///
/// So the cost of the index is that a designer inserting a line orphans the
/// translations after it in that one page, which Phase 15 re-matches by text.
/// The cost of removing it would be that two identical lines in one page could
/// not be told apart at write time. Given the choice, the recoverable cost wins,
/// and RPG Maker's own editor preserving the list order is what keeps it rare.
fn min_rpgmaker(jadwal: &mut JadwalNusus, taqreer: &mut TaqreerRafd, jidhr: &Path) {
    let bunya = match rpgmaker::BunyatMashru::iktashif(jidhr) {
        Ok(bunya) => bunya,
        Err(khata) => {
            taqreer.sajjil(jidhr.display().to_string(), None, sabab_min_khata(&khata));
            return;
        }
    };

    let sijill = match rpgmaker::istakhrij(&bunya) {
        Ok(sijill) => sijill,
        Err(khata) => {
            taqreer.sajjil(jidhr.display().to_string(), None, sabab_min_khata(&khata));
            return;
        }
    };

    if sijill.madakhil.is_empty() {
        taqreer.sajjil(jidhr.display().to_string(), None, SababRafd::BilaNusus);
        return;
    }

    for malaf in &sijill.malaffat {
        let madakhil = sijill.li_malaf(malaf);
        let mut adad = 0_usize;

        for (khana, madkhal) in madakhil.iter().enumerate() {
            if madkhal.khaam.trim().is_empty() {
                continue;
            }
            let mawqi = MawqiNass {
                hawiya: malaf.clone(),
                asl: None,
                mawqi: madkhal.masar.clone(),
                haql: Some(madkhal.naw.ism().to_owned()),
                // RPG Maker has no localization system. There is no developer
                // chosen key to inherit, and inventing one from the path would
                // be Taarib's key wearing the engine's name.
                miftah_muharrik: None,
            };

            let mut talab = TalabMudkhal::jadeed(mawqi, &madkhal.khaam)
                .bi_tarmiz(Some("UTF-8".to_owned()));
            if let Some((tasnif, thiqa)) = tasrih_rpgmaker(madkhal) {
                talab = talab.bi_tasrih(tasnif, thiqa);
            }
            if matches!(madkhal.naw, rpgmaker::NawMadkhal::SatrNass) {
                talab = talab
                    .bi_mutakallim(mutakallim_rpgmaker(&madakhil, khana))
                    .bi_jiwar(jiwar_rpgmaker(&madakhil, khana));
            }

            jadwal.adif(ansha_mudkhal(talab));
            adad = adad.saturating_add(1);
        }

        if adad > 0 {
            taqreer.sajjil_qira(
                malaf.clone(),
                adad,
                format!("{} data file", bunya.isdar().ism()),
            );
        }
    }
}

/// The kind an RPG Maker record's own structure states.
///
/// [`None`] where the structure does not state one — a database field or a
/// plugin parameter — and the shared classifier then reads the JSON path, which
/// is where `description` and `name` live.
fn tasrih_rpgmaker(madkhal: &rpgmaker::MadkhalNusus) -> Option<(TasnifNass, u8)> {
    use rpgmaker::NawMadkhal;

    match madkhal.naw {
        // Command 101's fourth parameter is the name drawn in the message
        // window's header, and `/displayName` is the name drawn over the map.
        // Both are proper names and nothing else.
        NawMadkhal::IsmMutakallim | NawMadkhal::IsmKhareeta => {
            Some((TasnifNass::Ism, THIQAT_BUNYA))
        }
        // Command 401 is one line of the message the window is showing, and 405
        // is one line of the scrolling text window. Both are narration or speech
        // by the format's definition.
        NawMadkhal::SatrNass | NawMadkhal::NassMutamarrir => {
            Some((TasnifNass::Hiwar, THIQAT_BUNYA))
        }
        // Command 102 is the choice list and 402 is the branch header for one
        // chosen option, which is the same string echoed.
        NawMadkhal::Ikhtiyar => Some((TasnifNass::Ikhtiyar, THIQAT_BUNYA)),
        NawMadkhal::MustalahNizam => tasrih_mustalah(&madkhal.masar),
        // A database row's field and a plugin's parameter are named by the
        // developer, so the name is the evidence and the shared classifier is
        // where names are read.
        NawMadkhal::HaqlQaeda | NawMadkhal::BarametrMulhaq => None,
    }
}

/// What a `System.json` term's position in the editor's vocabulary tree states.
fn tasrih_mustalah(masar: &str) -> Option<(TasnifNass, u8)> {
    let munkhafid = masar.to_ascii_lowercase();
    let naw = if munkhafid.contains("/terms/messages/") {
        TasnifNass::Nizam
    } else if munkhafid.contains("/terms/commands/") || munkhafid.contains("/terms/basic/") {
        TasnifNass::Qaima
    } else if munkhafid.contains("/terms/params/")
        || munkhafid.contains("/armortypes")
        || munkhafid.contains("/weapontypes")
        || munkhafid.contains("/skilltypes")
        || munkhafid.contains("/equiptypes")
        || munkhafid.contains("/elements")
        || munkhafid.contains("/gametitle")
        || munkhafid.contains("/currencyunit")
    {
        TasnifNass::Ism
    } else {
        return None;
    };
    Some((naw, THIQAT_MUSTALAH))
}

/// Whether a record is a line the message or scroll window draws.
///
/// A free function rather than a closure, because its callers hold a `&&` into a
/// slice of references and deref coercion at a call site is what turns that into
/// the `&` the body wants.
const fn satr_hiwar(madkhal: &rpgmaker::MadkhalNusus) -> bool {
    matches!(
        madkhal.naw,
        rpgmaker::NawMadkhal::SatrNass | rpgmaker::NawMadkhal::NassMutamarrir
    )
}

/// The `/list/` prefix a command's JSON path shares with its siblings.
///
/// Two commands in one event page have identical paths up to and including
/// `/list/`, and nothing else does. That is what makes "the name header for this
/// line" an exact question rather than a proximity guess: a 101 in a different
/// event, or on a different page, has a different prefix and is not a candidate
/// however close it sits in the file.
fn qaimat_amr(masar: &str) -> Option<&str> {
    const FASIL: &str = "/list/";
    let mawqi = masar.find(FASIL)?;
    masar.get(..mawqi.checked_add(FASIL.len())?)
}

/// The speaker a message line's own command list declares.
///
/// Command 101 writes the header for the message that follows it, so the nearest
/// preceding 101 *in the same command list* is the speaker, exactly. There is no
/// nearest-neighbour heuristic here — a scan that left the list stops.
fn mutakallim_rpgmaker(
    madakhil: &[&rpgmaker::MadkhalNusus],
    khana: usize,
) -> Option<String> {
    let hali = madakhil.get(khana)?;
    let qaima = qaimat_amr(&hali.masar)?;
    let mut sabiq = khana;
    while let Some(fahras) = sabiq.checked_sub(1) {
        sabiq = fahras;
        let madkhal = madakhil.get(fahras)?;
        if qaimat_amr(&madkhal.masar) != Some(qaima) {
            return None;
        }
        if matches!(madkhal.naw, rpgmaker::NawMadkhal::IsmMutakallim) {
            return Some(madkhal.khaam.clone());
        }
    }
    None
}

/// The lines around a message line, from the same command list.
fn jiwar_rpgmaker(madakhil: &[&rpgmaker::MadkhalNusus], khana: usize) -> Vec<String> {
    let Some(hali) = madakhil.get(khana) else {
        return Vec::new();
    };
    let Some(qaima) = qaimat_amr(&hali.masar) else {
        return Vec::new();
    };

    let mut qabl: Vec<String> = Vec::new();
    let mut sabiq = khana;
    while qabl.len() < JIWAR_ATRAF
        && let Some(fahras) = sabiq.checked_sub(1)
    {
        sabiq = fahras;
        let Some(madkhal) = madakhil.get(fahras) else { break };
        if qaimat_amr(&madkhal.masar) != Some(qaima) {
            break;
        }
        if satr_hiwar(madkhal) {
            qabl.push(madkhal.khaam.clone());
        }
    }
    qabl.reverse();

    let mut baad: Vec<String> = Vec::new();
    let mut taali = khana;
    while baad.len() < JIWAR_ATRAF {
        let Some(fahras) = taali.checked_add(1) else { break };
        taali = fahras;
        let Some(madkhal) = madakhil.get(fahras) else { break };
        if qaimat_amr(&madkhal.masar) != Some(qaima) {
            break;
        }
        if satr_hiwar(madkhal) {
            baad.push(madkhal.khaam.clone());
        }
    }

    qabl.extend(baad);
    qabl
}

// ---------------------------------------------------------------------------
// RPG Maker VX Ace
// ---------------------------------------------------------------------------

/// Event command codes whose meaning the format states outright.
///
/// The same numbers MV and MZ use, because VX Ace is where they came from.
const RUMUZ_VXACE: &[(i64, TasnifNass)] = &[
    (401, TasnifNass::Hiwar),
    (405, TasnifNass::Hiwar),
    (102, TasnifNass::Ikhtiyar),
    (402, TasnifNass::Ikhtiyar),
    (320, TasnifNass::Ism),
    (324, TasnifNass::Ism),
    (325, TasnifNass::Wasf),
];

/// Reads a VX Ace project, loose or packed into `Game.rgss3a`.
///
/// ## The arena handle is not an address
///
/// [`vxace::MustalahVxAce::muashir`] identifies a string inside one decoded
/// `Marshal` graph, and it is exactly as durable as that graph: re-saving the
/// file from the editor renumbers every handle in it. It is what the *patcher*
/// addresses a replacement by, in the same process that decoded the stream, and
/// it is deliberately not what identity is derived from here.
///
/// What survives instead is the script file, the Ruby class the string sat in,
/// and the instance variable or parameter slot that held it — `Data/Map001` plus
/// `RPG::EventCommand` plus `parameters[0]`. That is stable across a re-save and
/// it is what a person reading the table can act on.
fn min_vxace(jadwal: &mut JadwalNusus, taqreer: &mut TaqreerRafd, jidhr: &Path) {
    let mut wujida = false;

    // Loose data first, because RGSS3 mounts the archive and then lets a real
    // file on disk shadow a member of the same name — so a project holding both
    // runs the loose one, and extracting the archived copy would offer a
    // translator text the game does not load.
    let mut asmaa_loose: BTreeSet<String> = BTreeSet::new();
    for masar in malaffat_bi_lahiqa(jidhr, &["rvdata2"]) {
        let hawiya = nisbi(jidhr, &masar);
        let _ = asmaa_loose.insert(ism_asfal(&hawiya));
        wujida = true;
        match qira_malaf(&masar, AQSA_RVDATA) {
            Ok(bayt) => sajjil_silsila_vxace(jadwal, taqreer, &hawiya, None, &bayt),
            Err(khata) => taqreer.sajjil(hawiya, None, sabab_min_khata(&khata)),
        }
    }

    for masar in malaffat_bi_lahiqa(jidhr, &["rgss3a", "rgssad", "rgss2a"]) {
        let hawiya = nisbi(jidhr, &masar);
        let qari = match vxace::Hawiya::iqra(&masar) {
            Ok(qari) => qari,
            Err(khata) => {
                taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
                continue;
            }
        };
        wujida = true;
        let madakhil: Vec<vxace::MadkhalRgss> = qari.madakhil().to_vec();
        for madkhal in &madakhil {
            let munkhafid = madkhal.masar.replace('\\', "/").to_ascii_lowercase();
            if !munkhafid.ends_with(".rvdata2") {
                continue;
            }
            if asmaa_loose.contains(&ism_asfal(&munkhafid)) {
                continue;
            }
            match qari.istakhrij(madkhal) {
                Ok(bayt) => sajjil_silsila_vxace(
                    jadwal,
                    taqreer,
                    &hawiya,
                    Some(madkhal.masar.clone()),
                    &bayt,
                ),
                Err(khata) => taqreer.sajjil(
                    hawiya.clone(),
                    Some(madkhal.masar.clone()),
                    sabab_min_khata(&khata),
                ),
            }
        }
    }

    if !wujida {
        taqreer.sajjil(
            jidhr.display().to_string(),
            None,
            SababRafd::SighaMajhula {
                wujid: "no Data/*.rvdata2 and no Game.rgss3a under this directory".to_owned(),
            },
        );
    }
}

/// Decodes one `Marshal` stream and folds its strings into the table.
fn sajjil_silsila_vxace(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    asl: Option<String>,
    bayt: &[u8],
) {
    let silsila = match vxace::iqra_silsila(bayt) {
        Ok(silsila) => silsila,
        Err(khata) => {
            taqreer.sajjil(hawiya.to_owned(), asl, sabab_min_khata(&khata));
            return;
        }
    };

    let ism = asl.as_deref().unwrap_or(hawiya);
    let mustalahat = vxace::iltiqat_nusus(&silsila, ism);
    if mustalahat.is_empty() {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }

    let mut adad = 0_usize;
    for mustalah in &mustalahat {
        if mustalah.nass.trim().is_empty() {
            continue;
        }
        let mawqi = MawqiNass {
            hawiya: hawiya.to_owned(),
            asl: asl.clone(),
            mawqi: mustalah.sanf.clone(),
            haql: Some(mustalah.haql.clone()),
            miftah_muharrik: None,
        };
        let mut talab = TalabMudkhal::jadeed(mawqi, &mustalah.nass);
        if let Some(ramz) = mustalah.ramz
            && let Some((_, tasnif)) = RUMUZ_VXACE.iter().find(|(matlub, _)| *matlub == ramz)
        {
            talab = talab.bi_tasrih(*tasnif, THIQAT_BUNYA);
        }
        // The encoding is not recorded, and that is deliberate rather than an
        // omission. A `Marshal` string carries its own encoding tag, the reader
        // decodes with it, and the same reader writes the replacement back
        // through `Silsila::baddil_nass` — so the tag never leaves the reader
        // and there is nothing here for this module to preserve. Recording a
        // guessed "UTF-8" would be a claim about a Shift-JIS game that the
        // writer would then have to ignore.
        jadwal.adif(ansha_mudkhal(talab));
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }
    taqreer.sajjil_qira(hawiya.to_owned(), adad, "RPG Maker VX Ace Marshal stream");
}

// ---------------------------------------------------------------------------
// Ren'Py
// ---------------------------------------------------------------------------

/// Reads a Ren'Py game: loose `.rpy` and `.rpyc`, and both inside `.rpa`.
///
/// ## Recovering Ren'Py's own identifier, without inventing one
///
/// Ren'Py derives a statement identifier from a hash of the statement's
/// regenerated source. Only the engine can regenerate that source byte for byte,
/// so this crate never computes one — but a game that ships *any* translation
/// hands over a complete, exact map from identifier to source line, because the
/// identifier is derived from the source statement and is therefore the same in
/// the French translation as in the Arabic one.
///
/// That map is keyed by source text, and source text is not unique: two
/// characters saying `Yes.` are two statements with two identifiers. So an entry
/// whose source text maps to **more than one** identifier is left without one.
/// Taking either would be a coin flip that produces a patch writing one
/// character's line into another's mouth, and the derived identity that replaces
/// it is merely less durable rather than wrong.
///
/// ## `game/tl/` is read for identifiers and never for source
///
/// A file under `game/tl/` is somebody's translation. The scanner already skips
/// `translate` blocks, so re-extracting one would be harmless — but the rest of
/// such a file is that translator's own strings, and offering a Ren'Py game's
/// French as the English to translate from is exactly the failure the exclusion
/// exists to prevent.
fn min_renpy(jadwal: &mut JadwalNusus, taqreer: &mut TaqreerRafd, jidhr: &Path) {
    let masarat_rpy = malaffat_bi_lahiqa(jidhr, &["rpy"]);
    let masarat_rpyc = malaffat_bi_lahiqa(jidhr, &["rpyc"]);
    let masarat_rpa = malaffat_bi_lahiqa(jidhr, &["rpa"]);

    // Every source whose compiled twin should therefore be skipped. The reader's
    // own rule: a `.rpyc` is read only where the source is absent, because the
    // source carries comments, formatting and statement order the pickle does
    // not.
    let mut asmaa_masdar: BTreeSet<String> = BTreeSet::new();
    for masar in &masarat_rpy {
        asmaa_masdar.insert(jidhr_ism(&nisbi(jidhr, masar)));
    }

    let mut khareeta: BTreeMap<String, Option<String>> = BTreeMap::new();
    for masar in &masarat_rpy {
        let hawiya = nisbi(jidhr, masar);
        if !fi_mujallad_tarjama(&hawiya) {
            continue;
        }
        if let Ok(nass) = qira_nass(masar) {
            damma_muarrifat(&mut khareeta, &nass);
        }
    }
    for masar in &masarat_rpa {
        let Ok(qari) = renpy::Hawiya::iqra(masar) else { continue };
        let madakhil: Vec<renpy::MadkhalRpa> = qari.bi_lahiqa(".rpy").cloned().collect();
        for madkhal in &madakhil {
            if !fi_mujallad_tarjama(&madkhal.masar) {
                continue;
            }
            if let Ok(bayt) = qari.istakhrij(madkhal)
                && let Ok(nass) = std::str::from_utf8(&bayt)
            {
                damma_muarrifat(&mut khareeta, nass);
            }
        }
    }

    let mut wujida = false;

    for masar in &masarat_rpy {
        let hawiya = nisbi(jidhr, masar);
        if fi_mujallad_tarjama(&hawiya) {
            continue;
        }
        wujida = true;
        match qira_nass(masar) {
            Ok(nass) => {
                let sijillat = renpy::iltiqat_min_rpy(&nass, &hawiya);
                sajjil_sijillat(jadwal, taqreer, &khareeta, &hawiya, None, &sijillat, ".rpy");
            }
            Err(khata) => taqreer.sajjil(hawiya, None, sabab_min_khata(&khata)),
        }
    }

    for masar in &masarat_rpyc {
        let hawiya = nisbi(jidhr, masar);
        if fi_mujallad_tarjama(&hawiya) || asmaa_masdar.contains(&jidhr_ism(&hawiya)) {
            continue;
        }
        wujida = true;
        // The reader's own ceiling rather than a second number invented here: a
        // `.rpyc` this build would refuse to walk is one it should also refuse
        // to read off the disk, and the two limits agreeing is what keeps the
        // refusal reason accurate.
        let bayt = match qira_malaf(masar, renpy::AQSA_RPYC) {
            Ok(bayt) => bayt,
            Err(khata) => {
                taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
                continue;
            }
        };
        match renpy::iltiqat_min_rpyc(&bayt, &hawiya) {
            Ok(sijillat) => {
                sajjil_sijillat(jadwal, taqreer, &khareeta, &hawiya, None, &sijillat, ".rpyc");
            }
            Err(khata) => taqreer.sajjil(hawiya, None, sabab_min_khata(&khata)),
        }
    }

    for masar in &masarat_rpa {
        let hawiya = nisbi(jidhr, masar);
        let qari = match renpy::Hawiya::iqra(masar) {
            Ok(qari) => qari,
            Err(khata) => {
                taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
                continue;
            }
        };
        wujida = true;
        let madakhil: Vec<renpy::MadkhalRpa> = qari.madakhil().to_vec();
        let asmaa_dakhili: BTreeSet<String> = madakhil
            .iter()
            .filter(|madkhal| lahiqatuhu(&madkhal.masar, "rpy"))
            .map(|madkhal| jidhr_ism(&madkhal.masar))
            .collect();

        for madkhal in &madakhil {
            let masdar = lahiqatuhu(&madkhal.masar, "rpy");
            let mutarjam = lahiqatuhu(&madkhal.masar, "rpyc");
            if !masdar && !mutarjam {
                continue;
            }
            if fi_mujallad_tarjama(&madkhal.masar) {
                continue;
            }
            if mutarjam
                && (asmaa_dakhili.contains(&jidhr_ism(&madkhal.masar))
                    || asmaa_masdar.contains(&jidhr_ism(&madkhal.masar)))
            {
                continue;
            }
            let bayt = match qari.istakhrij(madkhal) {
                Ok(bayt) => bayt,
                Err(khata) => {
                    taqreer.sajjil(
                        hawiya.clone(),
                        Some(madkhal.masar.clone()),
                        sabab_min_khata(&khata),
                    );
                    continue;
                }
            };
            let sijillat = if masdar {
                match std::str::from_utf8(&bayt) {
                    Ok(nass) => renpy::iltiqat_min_rpy(nass, &madkhal.masar),
                    Err(khata) => {
                        taqreer.sajjil(
                            hawiya.clone(),
                            Some(madkhal.masar.clone()),
                            SababRafd::Talif {
                                sabab: format!(
                                    "a .rpy that is not valid UTF-8 at byte {}",
                                    khata.valid_up_to()
                                ),
                            },
                        );
                        continue;
                    }
                }
            } else {
                match renpy::iltiqat_min_rpyc(&bayt, &madkhal.masar) {
                    Ok(sijillat) => sijillat,
                    Err(khata) => {
                        taqreer.sajjil(
                            hawiya.clone(),
                            Some(madkhal.masar.clone()),
                            sabab_min_khata(&khata),
                        );
                        continue;
                    }
                }
            };
            sajjil_sijillat(
                jadwal,
                taqreer,
                &khareeta,
                &hawiya,
                Some(madkhal.masar.clone()),
                &sijillat,
                if masdar { ".rpy" } else { ".rpyc" },
            );
        }
    }

    if !wujida {
        taqreer.sajjil(
            jidhr.display().to_string(),
            None,
            SababRafd::SighaMajhula {
                wujid: "no .rpy, .rpyc or .rpa under this directory".to_owned(),
            },
        );
    }
}

/// Whether a path sits under Ren'Py's translation directory.
fn fi_mujallad_tarjama(masar: &str) -> bool {
    let munkhafid = masar.replace('\\', "/").to_ascii_lowercase();
    munkhafid.starts_with("game/tl/") || munkhafid.contains("/game/tl/")
}

/// Folds one translation file's identifiers into the map, marking a source text
/// that two identifiers claim as ambiguous rather than picking one.
fn damma_muarrifat(khareeta: &mut BTreeMap<String, Option<String>>, nass: &str) {
    for (muarrif, asl) in renpy::muarrifat_min_tarjama(nass) {
        // Bound to a local before the match, so the read borrow ends before an
        // arm writes. A `match` on the lookup itself would hold the map borrowed
        // across the whole statement.
        let mawjud = khareeta.get(&asl).cloned();
        match mawjud {
            None => {
                let _ = khareeta.insert(asl, Some(muarrif));
            }
            Some(Some(sabiq)) if sabiq == muarrif => {}
            Some(_) => {
                let _ = khareeta.insert(asl, None);
            }
        }
    }
}

/// Folds one file's Ren'Py records into the table.
fn sajjil_sijillat(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    khareeta: &BTreeMap<String, Option<String>>,
    hawiya: &str,
    asl: Option<String>,
    sijillat: &[renpy::Sijill],
    wasf: &str,
) {
    if sijillat.is_empty() {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }

    let mut adad = 0_usize;
    for (khana, sijill) in sijillat.iter().enumerate() {
        if sijill.asl.trim().is_empty() {
            continue;
        }
        let muarrif = sijill
            .muarrif
            .clone()
            .or_else(|| khareeta.get(&sijill.asl).cloned().flatten());

        let mawqi = MawqiNass {
            hawiya: hawiya.to_owned(),
            asl: asl.clone(),
            // The statement kind and the speaker, never the line number. A line
            // number is a position: adding a comment at the top of a script
            // moves every statement in it, and an identity derived from one
            // would orphan a whole file's translations for an edit that changed
            // no text at all.
            mawqi: match sijill.mutakallim.as_deref() {
                Some(mutakallim) if !mutakallim.is_empty() => {
                    format!("{}/{mutakallim}", sijill.naw.ism())
                }
                _ => sijill.naw.ism().to_owned(),
            },
            haql: None,
            miftah_muharrik: muarrif,
        };

        let talab = TalabMudkhal::jadeed(mawqi, &sijill.asl)
            .bi_tasrih(tasnif_renpy(sijill.naw), THIQAT_BUNYA)
            .bi_mutakallim(sijill.mutakallim.clone())
            .bi_jiwar(jiwar_renpy(sijillat, khana))
            .bi_tarmiz(Some("UTF-8".to_owned()));
        // A Ren'Py statement is delivered through the engine's own translation
        // mechanism, so the localization signal is true whether or not an
        // identifier was recovered.
        jadwal.adif(ansha_mudkhal(talab.bi_nizam_tawtin()));
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }
    taqreer.sajjil_qira(hawiya.to_owned(), adad, format!("Ren'Py {wasf} script"));
}

/// What a Ren'Py record's own mechanism states it is.
///
/// Exact in all three cases: a `say` statement is speech, a `menu` item is a
/// choice the player picks between, and anything the engine delivers through
/// `translate <language> strings:` is interface text by construction.
const fn tasnif_renpy(naw: renpy::NawSijill) -> TasnifNass {
    match naw {
        renpy::NawSijill::Hiwar => TasnifNass::Hiwar,
        renpy::NawSijill::Ikhtiyar => TasnifNass::Ikhtiyar,
        renpy::NawSijill::Mustalah => TasnifNass::Qaima,
    }
}

/// The dialogue around one record, in the order the file has it.
fn jiwar_renpy(sijillat: &[renpy::Sijill], khana: usize) -> Vec<String> {
    let mut jiwar = Vec::new();
    let bidaya = khana.saturating_sub(JIWAR_ATRAF);
    let nihaya = khana.saturating_add(JIWAR_ATRAF).saturating_add(1);
    for (fahras, sijill) in sijillat.iter().enumerate() {
        if fahras < bidaya || fahras >= nihaya || fahras == khana {
            continue;
        }
        if matches!(sijill.naw, renpy::NawSijill::Hiwar) {
            jiwar.push(sijill.asl.clone());
        }
    }
    jiwar
}

// ---------------------------------------------------------------------------
// GameMaker
// ---------------------------------------------------------------------------

/// The chunk that holds compiled bytecode.
const QITAA_RAMZ: [u8; 4] = *b"CODE";

/// Reads a GameMaker `data.win`, cross-linking `STRG` against the reference
/// census.
///
/// ## What references a string is the strongest signal this container gives
///
/// `STRG` is one flat pool holding every string in the game: dialogue, room
/// names, sprite names, variable names, shader source, audio group names. The
/// text itself distinguishes almost none of those. What does distinguish them is
/// **where the pointers to them live**, which the Phase 10 reader enumerates into
/// a census:
///
/// * reached only from a chunk that holds asset records — `SPRT`, `OBJT`, `ROOM`,
///   `VARI`, `FUNC` — the string is that asset's or variable's *name*, and the
///   container says so by where the pointer sits rather than by anything about
///   the characters. That is [`TasnifNass::Dakhili`] at [`THIQAT_MARJA_ASL`];
/// * reached from `CODE`, the bytecode chunk, the string is a literal the game
///   loads at runtime, which is where every drawn string comes from. No verdict
///   is forced there — a `CODE` literal is also how a sprite is looked up by name
///   — so the shared classifier decides and the reference merely stops the
///   asset-name rule from firing;
/// * reached from nothing at all, the entry is usually a leftover from a deleted
///   asset. [`THIQAT_MARJA_YATIM`] is lower than the others because it is
///   *usually*.
///
/// The orphan rule is applied **only when the census actually enumerated
/// `CODE`**. [`gamemaker::JadwalMaraji::mafhuma`] exists to tell "no reference
/// into this region" apart from "this region was never examined", and treating
/// the second as the first would file every string in the game as a leftover.
///
/// ## The pool index is not part of the location
///
/// A `STRG` index is a bare array index — the thing [`crate::jadwal`]'s header
/// forbids — and a rebuild renumbers the whole pool. So the location is the pool
/// and the referrers, and Phase 14 resolves the index at patch time by matching
/// the recorded source text against the pool of the container it is actually
/// writing to. That is both stable across rebuilds and correct for the container
/// in hand, which an index recorded at extraction time is neither.
fn min_gamemaker(jadwal: &mut JadwalNusus, taqreer: &mut TaqreerRafd, jidhr: &Path) {
    let Some(masar) = gamemaker::MifhasGameMaker::hawiya(jidhr) else {
        taqreer.sajjil(
            jidhr.display().to_string(),
            None,
            SababRafd::SighaMajhula {
                wujid: "no GameMaker data container under this directory".to_owned(),
            },
        );
        return;
    };

    let hawiya = nisbi(jidhr, &masar);
    let bayt = match qira_malaf(&masar, AQSA_HAWIYA) {
        Ok(bayt) => bayt,
        Err(khata) => {
            taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
            return;
        }
    };
    let qari = match gamemaker::HawiyatGameMaker::min_bayt(bayt, &masar) {
        Ok(qari) => qari,
        Err(khata) => {
            taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
            return;
        }
    };

    let ramz_mafhum = qari.maraji().mafhuma(QITAA_RAMZ);
    let mut adad = 0_usize;

    for (khana, madkhal) in qari.hawd().madakhil().iter().enumerate() {
        if madkhal.nass.trim().is_empty() {
            continue;
        }
        let Ok(fahras) = u32::try_from(khana) else {
            continue;
        };

        let qitaa = qitaa_maraji(&qari, fahras);
        let min_ramz = qitaa.iter().any(|ism| ism == "CODE");
        let wasf_marja = if qitaa.is_empty() {
            "unreferenced".to_owned()
        } else {
            qitaa.join("+")
        };

        let mawqi = MawqiNass {
            hawiya: hawiya.clone(),
            asl: None,
            mawqi: "STRG".to_owned(),
            haql: Some(wasf_marja),
            miftah_muharrik: None,
        };
        let mut talab =
            TalabMudkhal::jadeed(mawqi, &madkhal.nass).bi_tarmiz(Some("UTF-8".to_owned()));
        if !min_ramz {
            if !qitaa.is_empty() {
                talab = talab.bi_tasrih(TasnifNass::Dakhili, THIQAT_MARJA_ASL);
            } else if ramz_mafhum {
                talab = talab.bi_tasrih(TasnifNass::Dakhili, THIQAT_MARJA_YATIM);
            }
        }
        jadwal.adif(ansha_mudkhal(talab));
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya, None, SababRafd::BilaNusus);
        return;
    }

    let mut wasf = format!(
        "GameMaker {} container, STRG cross-linked to {} recorded reference(s)",
        qari.jeel().ism(),
        qari.maraji().adad()
    );
    if !ramz_mafhum {
        // Said out loud rather than left implicit. Without a fully enumerated
        // CODE chunk the difference between "nothing points at this string" and
        // "the pointers were never counted" is unavailable, and every
        // classification below leans on that difference.
        wasf.push_str(
            "; the CODE chunk was not enumerated exhaustively, so unreferenced entries were \
             not treated as leftovers",
        );
    }
    taqreer.sajjil_qira(hawiya, adad, wasf);
}

/// The chunks that hold a pointer to one pool entry, by name, deduplicated.
fn qitaa_maraji(qari: &gamemaker::HawiyatGameMaker, fahras: u32) -> Vec<String> {
    let mut asmaa: BTreeSet<String> = BTreeSet::new();
    for marja in qari.maraji().maraji_nass(fahras) {
        if let Some(qitaa) = qari.jadwal().iter().find(|qitaa| qitaa.yahwi(marja.mawqi)) {
            let _ = asmaa.insert(qitaa.ism_nass());
        }
    }
    asmaa.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Electron and NW.js
// ---------------------------------------------------------------------------

/// How sure an HTML element's own semantics make this module.
///
/// Seventy-eight. `<button>` really is a control a player reads and `<p>` really
/// is body text — the element *is* a declaration — but HTML's vocabulary is used
/// loosely enough in application interfaces that it is a weaker declaration than
/// an RPG Maker command code.
pub const THIQAT_UNSUR: u8 = 78;

/// How many bytes of HTML this module will walk, per archive.
pub const AQSA_MASH_HTML: usize = 24 * 1024 * 1024;

/// What each HTML element states about the text inside it.
const UNASIR: &[(&str, TasnifNass)] = &[
    ("title", TasnifNass::Qaima),
    ("button", TasnifNass::Qaima),
    ("a", TasnifNass::Qaima),
    ("label", TasnifNass::Qaima),
    ("legend", TasnifNass::Qaima),
    ("summary", TasnifNass::Qaima),
    ("th", TasnifNass::Qaima),
    ("option", TasnifNass::Ikhtiyar),
    ("h1", TasnifNass::Ism),
    ("h2", TasnifNass::Ism),
    ("h3", TasnifNass::Ism),
    ("h4", TasnifNass::Qaima),
    ("h5", TasnifNass::Qaima),
    ("h6", TasnifNass::Qaima),
    ("p", TasnifNass::Wasf),
    ("li", TasnifNass::Wasf),
    ("td", TasnifNass::Wasf),
    ("blockquote", TasnifNass::Wasf),
    ("figcaption", TasnifNass::Wasf),
];

/// Attributes whose value is drawn, with what each one is.
const SIFAT_MARSUMA: &[(&str, TasnifNass)] = &[
    ("title", TasnifNass::Tafseer),
    ("alt", TasnifNass::Tafseer),
    ("placeholder", TasnifNass::Tafseer),
    ("aria-label", TasnifNass::Tafseer),
    ("label", TasnifNass::Qaima),
];

/// Elements HTML never closes, which therefore never pop the path stack.
const UNASIR_FARIGHA: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

/// Reads an Electron or NW.js application out of its `.asar`.
///
/// Three sources, and they do not have the same error rate — which is why the
/// distinction survives into the table rather than being flattened here:
///
/// * **JSON resource files** are exact. The application looks strings up in them
///   by key, and every string value in one is text by construction. The JSON
///   pointer is the structural location.
/// * **JavaScript string literals** are found by a documented heuristic in the
///   Phase 10 reader, and every classification derived from one is capped at
///   [`THIQAT_BARMAJI`]. A verdict cannot be more certain than the extraction
///   that produced it.
/// * **HTML text nodes** are walked here, because the reader does not read HTML.
///   The element names the kind.
///
/// The literal's ordinal within its script — the reader's `js:<n>` — is
/// deliberately dropped from the location. It is a bare array index, a rebuild
/// renumbers it, and the script's own path plus the source text identify the
/// literal without it.
fn min_electron(jadwal: &mut JadwalNusus, taqreer: &mut TaqreerRafd, jidhr: &Path) {
    let masarat = malaffat_bi_lahiqa(jidhr, &["asar"]);
    if masarat.is_empty() {
        taqreer.sajjil(
            jidhr.display().to_string(),
            None,
            SababRafd::SighaMajhula {
                wujid: "no .asar archive under this directory".to_owned(),
            },
        );
        return;
    }

    for masar in &masarat {
        let hawiya = nisbi(jidhr, masar);
        let qari = match electron::HawiyatAsar::min_masar(masar) {
            Ok(qari) => qari,
            Err(khata) => {
                taqreer.sajjil(hawiya, None, sabab_min_khata(&khata));
                continue;
            }
        };

        match electron::istakhrij(&qari) {
            Ok(hasad) => sajjil_hasad(jadwal, taqreer, &hawiya, &hasad),
            Err(khata) => taqreer.sajjil(hawiya.clone(), None, sabab_min_khata(&khata)),
        }
        sajjil_html(jadwal, taqreer, &hawiya, &qari);
    }
}

/// Folds the reader's JSON and script harvest into the table.
fn sajjil_hasad(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    hasad: &electron::HasadNusus,
) {
    for (masar, sabab) in &hasad.matruka {
        taqreer.sajjil(
            hawiya.to_owned(),
            Some(masar.clone()),
            SababRafd::Talif { sabab: sabab.clone() },
        );
    }

    if hasad.nusus.is_empty() {
        taqreer.sajjil(hawiya.to_owned(), None, SababRafd::BilaNusus);
        return;
    }

    let mut adad = 0_usize;
    for nass in &hasad.nusus {
        if nass.nass.trim().is_empty() {
            continue;
        }
        let barmaji = matches!(nass.naw, electron::NawNass::Barmaji);
        let mawqi = MawqiNass {
            hawiya: hawiya.to_owned(),
            asl: Some(nass.masdar.clone()),
            mawqi: if barmaji { "js".to_owned() } else { nass.miftah.clone() },
            haql: None,
            miftah_muharrik: None,
        };
        let talab =
            TalabMudkhal::jadeed(mawqi, &nass.nass).bi_tarmiz(Some("UTF-8".to_owned()));
        let mut mudkhal = ansha_mudkhal(talab);
        if barmaji {
            mudkhal.thiqa = mudkhal.thiqa.min(THIQAT_BARMAJI);
        }
        jadwal.adif(mudkhal);
        adad = adad.saturating_add(1);
    }

    taqreer.sajjil_qira(
        hawiya.to_owned(),
        adad,
        format!(
            "asar archive, {} file(s) read: {} exact from JSON resources, {} heuristic from \
             scripts",
            hasad.maqrua,
            hasad.adad_mawarid(),
            hasad.nusus.len().saturating_sub(hasad.adad_mawarid())
        ),
    );
}

/// Walks every HTML document in the archive for its text nodes.
fn sajjil_html(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    qari: &electron::HawiyatAsar,
) {
    let mut mizaniya = AQSA_MASH_HTML;
    let mut adad = 0_usize;

    for madkhal in qari.madakhil() {
        if !madkhal.mahzum() || !matches!(madkhal.imtidad().as_str(), "html" | "htm") {
            continue;
        }
        if madkhal.masar.contains("node_modules/") {
            continue;
        }
        let Some(bayt) = qari.muhtawa(&madkhal.masar) else {
            continue;
        };
        if bayt.len() > mizaniya {
            taqreer.sajjil(
                hawiya.to_owned(),
                Some(madkhal.masar.clone()),
                SababRafd::TajawuzHadd {
                    hadd: "the per-archive budget for walking HTML documents".to_owned(),
                    qeema: u64::try_from(bayt.len()).unwrap_or(u64::MAX),
                    saqf: u64::try_from(AQSA_MASH_HTML).unwrap_or(u64::MAX),
                },
            );
            break;
        }
        let munaqqa = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bayt);
        let Ok(nass) = std::str::from_utf8(munaqqa) else {
            taqreer.sajjil(
                hawiya.to_owned(),
                Some(madkhal.masar.clone()),
                SababRafd::Talif { sabab: "an HTML document that is not valid UTF-8".to_owned() },
            );
            continue;
        };
        mizaniya = mizaniya.saturating_sub(bayt.len());

        for wujud in nusus_html(nass) {
            let tasnif = match wujud.haql.as_deref() {
                Some(sifa) => SIFAT_MARSUMA
                    .iter()
                    .find(|(ism, _)| *ism == sifa)
                    .map(|(_, tasnif)| *tasnif),
                None => UNASIR
                    .iter()
                    .find(|(ism, _)| *ism == wujud.unsur)
                    .map(|(_, tasnif)| *tasnif),
            };
            let mawqi = MawqiNass {
                hawiya: hawiya.to_owned(),
                asl: Some(madkhal.masar.clone()),
                mawqi: wujud.mawqi,
                haql: wujud.haql,
                miftah_muharrik: None,
            };
            let mut talab =
                TalabMudkhal::jadeed(mawqi, &wujud.nass).bi_tarmiz(Some("UTF-8".to_owned()));
            if let Some(tasnif) = tasnif {
                talab = talab.bi_tasrih(tasnif, THIQAT_UNSUR);
            }
            jadwal.adif(ansha_mudkhal(talab));
            adad = adad.saturating_add(1);
        }
    }

    if adad > 0 {
        taqreer.sajjil_qira(
            hawiya.to_owned(),
            adad,
            "HTML documents inside the asar, walked for text nodes and drawn attributes",
        );
    }
}

/// One string found in an HTML document.
#[derive(Debug)]
struct NassHtml {
    /// The element path from the document root, with ids and classes attached.
    mawqi: String,
    /// The attribute the text came from, or [`None`] for a text node.
    haql: Option<String>,
    /// The text.
    nass: String,
    /// The element that holds it.
    unsur: String,
}

/// Walks an HTML document for its text nodes and its drawn attributes.
///
/// ## The path is context, not an address
///
/// HTML permits omitted end tags — `<p>` and `<li>` most often — and a walker
/// that pushes on every start and pops on every end drifts when it meets one.
/// That is tolerable here and would not be tolerable in a container reader,
/// because Phase 10's Electron patcher replaces text in the **live DOM** through
/// the injected renderer, matching on the source string rather than on a path
/// recorded at extraction time. A drifted path costs a translator context; it
/// cannot cost a player a swapped label.
///
/// `<script>` and `<style>` contents are skipped outright. Their text is code and
/// a stylesheet, and the Phase 10 reader already extracts script literals with a
/// scanner that knows JavaScript.
fn nusus_html(nass: &str) -> Vec<NassHtml> {
    use quick_xml::events::Event;

    let mut qari = quick_xml::Reader::from_str(nass);
    // HTML is not XML: void elements have no end tag and attribute values are
    // sometimes unquoted. Turning the end-name check off is what lets the walk
    // reach the end of a real application's document instead of stopping at its
    // first `<br>`.
    qari.config_mut().check_end_names = false;
    // Not `trim_text(true)`: an entity reference arrives as its own event, so a
    // run is trimmed once it is whole, not at every seam inside it.
    qari.config_mut().trim_text(false);

    let mut kawm: Vec<String> = Vec::new();
    let mut asmaa: Vec<String> = Vec::new();
    let mut natija: Vec<NassHtml> = Vec::new();
    let mut tajahul = 0_usize;
    let mut madad = String::new();

    loop {
        match qari.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(Event::Start(marka)) => {
                asdir_nass(&mut natija, &mut madad, &kawm, &asmaa);
                let ism = ism_marka(marka.local_name().as_ref());
                if UNASIR_FARIGHA.iter().any(|farigh| *farigh == ism) {
                    if tajahul == 0 {
                        kawm.push(juz_unsur(&ism, &marka));
                        sifat_html(&mut natija, &kawm, &ism, &marka);
                        let _ = kawm.pop();
                    }
                    continue;
                }
                if tajahul > 0 {
                    tajahul = tajahul.saturating_add(1);
                } else if ism == "script" || ism == "style" {
                    tajahul = 1;
                }
                kawm.push(juz_unsur(&ism, &marka));
                if tajahul == 0 {
                    sifat_html(&mut natija, &kawm, &ism, &marka);
                }
                asmaa.push(ism);
            }
            Ok(Event::Empty(marka)) => {
                asdir_nass(&mut natija, &mut madad, &kawm, &asmaa);
                if tajahul > 0 {
                    continue;
                }
                let ism = ism_marka(marka.local_name().as_ref());
                kawm.push(juz_unsur(&ism, &marka));
                sifat_html(&mut natija, &kawm, &ism, &marka);
                let _ = kawm.pop();
            }
            Ok(Event::End(_)) => {
                asdir_nass(&mut natija, &mut madad, &kawm, &asmaa);
                if tajahul > 0 {
                    tajahul = tajahul.saturating_sub(1);
                }
                let _ = kawm.pop();
                let _ = asmaa.pop();
            }
            Ok(Event::Text(jism)) => {
                if tajahul > 0 {
                    continue;
                }
                madad.push_str(&jism.xml10_content());
            }
            Ok(Event::GeneralRef(marja)) => {
                if tajahul > 0 {
                    continue;
                }
                // An entity no XML document predefines — `&nbsp;`, `&mdash;` —
                // goes back in as it was written. Resolving it would need the
                // HTML5 table, and dropping it would hand a translator a string
                // the document does not contain.
                match taarib_usus::kayanat::hall_marja(&marja) {
                    Some(hall) => madad.push_str(&hall),
                    None => madad.push_str(&taarib_usus::kayanat::nass_marja(&marja)),
                }
            }
            Ok(_) => {}
        }
    }
    asdir_nass(&mut natija, &mut madad, &kawm, &asmaa);
    natija
}

/// Emits the accumulated text run at the position it was read from, if any.
fn asdir_nass(
    natija: &mut Vec<NassHtml>,
    madad: &mut String,
    kawm: &[String],
    asmaa: &[String],
) {
    let munaqqa = madad.trim();
    if !munaqqa.is_empty() {
        natija.push(NassHtml {
            mawqi: kawm.join("/"),
            haql: None,
            nass: munaqqa.to_owned(),
            unsur: asmaa.last().cloned().unwrap_or_default(),
        });
    }
    madad.clear();
}

/// An element's name, lowercased.
fn ism_marka(khaam: &str) -> String {
    khaam.to_ascii_lowercase()
}

/// One path segment: the element, plus its id or its first class.
///
/// The id is what a designer and a script both address the element by, so it is
/// what makes a path mean the same thing after the document is reflowed. The
/// first class is the fallback, and the bare element name is the last resort.
fn juz_unsur(ism: &str, marka: &quick_xml::events::BytesStart<'_>) -> String {
    if let Some(muarrif) = sifa_marka(marka, "id") {
        return format!("{ism}#{muarrif}");
    }
    if let Some(sanf) = sifa_marka(marka, "class")
        && let Some(awwal) = sanf.split_whitespace().next()
    {
        return format!("{ism}.{awwal}");
    }
    ism.to_owned()
}

/// One attribute's value, unescaped.
fn sifa_marka(marka: &quick_xml::events::BytesStart<'_>, matlub: &str) -> Option<String> {
    for sifa in marka.attributes().flatten() {
        if sifa.key.local_name().as_ref().eq_ignore_ascii_case(matlub) {
            return sifa
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(std::borrow::Cow::into_owned);
        }
    }
    None
}

/// Records the drawn attributes of one element.
fn sifat_html(
    natija: &mut Vec<NassHtml>,
    kawm: &[String],
    ism: &str,
    marka: &quick_xml::events::BytesStart<'_>,
) {
    for (sifa, _) in SIFAT_MARSUMA {
        let Some(qeema) = sifa_marka(marka, sifa) else {
            continue;
        };
        if qeema.trim().is_empty() {
            continue;
        }
        natija.push(NassHtml {
            mawqi: kawm.join("/"),
            haql: Some((*sifa).to_owned()),
            nass: qeema.trim().to_owned(),
            unsur: ism.to_owned(),
        });
    }

    // `value` is drawn only on the controls that render it. On a hidden input or
    // a checkbox it is a submitted datum and not text, so it is read from the
    // three elements where it is a label and from nowhere else.
    if matches!(ism, "button" | "option" | "optgroup")
        && let Some(qeema) = sifa_marka(marka, "value")
        && !qeema.trim().is_empty()
    {
        natija.push(NassHtml {
            mawqi: kawm.join("/"),
            haql: Some("value".to_owned()),
            nass: qeema.trim().to_owned(),
            unsur: ism.to_owned(),
        });
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Every file under `jidhr` with one of the given lowercase extensions.
///
/// Skips [`taarib_muhawwil_nusus::DALIL_NUSAKH`], which is where Phase 10 keeps
/// the byte-exact originals of every file it patched. Reading those would put a
/// second copy of every string in the game into the table under a second
/// container name, and a translator would be shown each line twice with no way
/// to tell which one the game loads.
fn malaffat_bi_lahiqa(jidhr: &Path, lawahiq: &[&str]) -> Vec<PathBuf> {
    let mut masarat = Vec::new();
    let sayr = walkdir::WalkDir::new(jidhr)
        .max_depth(UMQ_MASH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|madkhal| {
            madkhal.file_name().to_str() != Some(taarib_muhawwil_nusus::DALIL_NUSAKH)
        })
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
        let munkhafid = lahiqa.to_ascii_lowercase();
        if lawahiq.iter().any(|matlub| *matlub == munkhafid) {
            masarat.push(masar);
        }
    }

    // Sorted so that two extractions of one build read the files in one order,
    // which is what makes the read report diffable between runs.
    masarat.sort();
    masarat
}

/// A path relative to the game's root, with forward slashes on every platform.
///
/// The separator is normalized because the container name is hashed into
/// [`taarib_mustalahat::nass::NassId`], and a project extracted on Windows and
/// re-extracted on Linux has to produce the same identities or every translation
/// in it orphans itself on the second machine.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    let juz = masar.strip_prefix(jidhr).unwrap_or(masar);
    juz.components()
        .map(|qism| qism.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// A path's last component, lowercased.
fn ism_asfal(masar: &str) -> String {
    let munkhafid = masar.replace('\\', "/").to_ascii_lowercase();
    munkhafid.rsplit('/').next().unwrap_or(&munkhafid).to_owned()
}

/// Whether an archive-relative path carries `lahiqa` as its extension.
///
/// `lahiqa` is spelled without its dot. Case-insensitive: a Ren'Py archive
/// built on Windows can hold `SCRIPT.RPYC`, and a case-sensitive test would
/// walk past it and lose the whole script.
fn lahiqatuhu(masar: &str, lahiqa: &str) -> bool {
    Path::new(masar).extension().is_some_and(|mawjuda| mawjuda.eq_ignore_ascii_case(lahiqa))
}

/// A Ren'Py path with its `.rpy` or `.rpyc` extension removed, lowercased.
///
/// What pairs a compiled script with its source, so that a game shipping both
/// is read from the source — which carries the statement order, the comments and
/// the speaker names the pickle does not.
fn jidhr_ism(masar: &str) -> String {
    let munkhafid = masar.replace('\\', "/").to_ascii_lowercase();
    munkhafid
        .strip_suffix(".rpyc")
        .or_else(|| munkhafid.strip_suffix(".rpy"))
        .unwrap_or(&munkhafid)
        .to_owned()
}

/// Reads a whole file, refusing one above `saqf` before reserving anything.
fn qira_malaf(masar: &Path, saqf: u64) -> Result<Vec<u8>, KhataNusus> {
    let bayanat = std::fs::metadata(masar)
        .map_err(|sabab| KhataNusus::KhataMalaf { masar: masar.to_path_buf(), sabab })?;
    if bayanat.len() > saqf {
        return Err(KhataNusus::HajmMufrit {
            haql: "a game data file",
            qeema: bayanat.len(),
            saqf,
        });
    }
    std::fs::read(masar)
        .map_err(|sabab| KhataNusus::KhataMalaf { masar: masar.to_path_buf(), sabab })
}

/// Reads a whole text file, refusing one that is not valid UTF-8.
///
/// Refused rather than decoded lossily, because a replacement character in a
/// source string is a character a translator will faithfully carry into the
/// translation and a player will eventually read.
fn qira_nass(masar: &Path) -> Result<String, KhataNusus> {
    let bayt = qira_malaf(masar, AQSA_MALAF_NASSI)?;
    let munaqqa = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bayt);
    std::str::from_utf8(munaqqa).map(str::to_owned).map_err(|khata| {
        KhataNusus::NassGhayrSalih {
            malaf: masar.display().to_string(),
            tarmiz: "UTF-8",
            mawqi: u64::try_from(khata.valid_up_to()).unwrap_or(u64::MAX),
        }
    })
}

/// Turns a reader's refusal into the reason the report shows.
///
/// Explicit rather than a catch-all, because the reason decides which remedy the
/// interface offers. Only the refusals with no better home fall through to
/// [`SababRafd::Talif`].
fn sabab_min_khata(khata: &KhataNusus) -> SababRafd {
    match khata {
        KhataNusus::KhataMalaf { sabab, .. } => {
            SababRafd::TaadhurQira { sabab: sabab.to_string() }
        }
        KhataNusus::MiftahMafqud { sabab, .. } => SababRafd::Mushaffar { wasf: sabab.clone() },
        KhataNusus::MiftahGhayrSalih { sabab, .. } => {
            SababRafd::Mushaffar { wasf: (*sabab).to_owned() }
        }
        KhataNusus::IsdarGhayrMadum { sigha, wujid, adna, aqsa } => {
            SababRafd::IsdarGhayrMadum {
                sigha: (*sigha).to_owned(),
                wujid: wujid.to_string(),
                madum: format!("{adna} to {aqsa}"),
            }
        }
        KhataNusus::HajmMufrit { haql, qeema, saqf } => {
            SababRafd::TajawuzHadd { hadd: (*haql).to_owned(), qeema: *qeema, saqf: *saqf }
        }
        KhataNusus::SihrGhayrMutabaq { sigha, .. } => SababRafd::SighaMajhula {
            wujid: format!("a file that does not carry the {sigha} signature"),
        },
        KhataNusus::BunyaGhayrMutawaqqaa { malaf, haql } => SababRafd::SighaMajhula {
            wujid: format!("{malaf}: this build expected {haql} and did not find it"),
        },
        akhar => SababRafd::Talif { sabab: akhar.to_string() },
    }
}
