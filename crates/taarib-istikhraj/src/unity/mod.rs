//! يونيتي — walking a Unity game's data directory and reading what is readable
//! out of it.
//!
//! Unity is the engine most of the games this product exists for are built in,
//! and it is also the one where extraction most often has to say no. This module
//! is the top of that: it finds the containers, dispatches each to
//! [`hawiya`] for framing and [`kaain`] for objects, and folds the results into
//! one [`JadwalNusus`] and one [`TaqreerRafd`].
//!
//! ```text
//! MyGame/
//!   MyGame.exe
//!   MyGame_Data/
//!     globalgamemanagers            SerializedFile, loose
//!     globalgamemanagers.assets     SerializedFile, loose
//!     resources.assets              SerializedFile, loose
//!     sharedassets0.assets          SerializedFile, loose
//!     level0, level1, …             SerializedFile, loose, no extension
//!     resources.assets.resS         raw streamed data — read by nothing here
//!     StreamingAssets/
//!       aa/StandaloneWindows64/
//!         ui_assets_all_<hash>.bundle   UnityFS, holding SerializedFiles
//! ```
//!
//! ## The walk is deterministic, and that is a correctness property
//!
//! Entries are visited in sorted order. That is not tidiness: string identity is
//! derived from position and the duplicate-group leader in
//! [`JadwalNusus::ila_mudkhalat`] is the lowest identity in a group, so an
//! extraction that read containers in directory order would produce a different
//! grouping on a machine whose filesystem enumerated them differently. Two
//! contributors extracting the same build would then get two different projects,
//! and the diff between them would look like the game had changed.
//!
//! ## Everything is bounded before it is allocated
//!
//! Every ceiling below is named, and each of them is a refusal rather than a
//! truncation: a container above one is reported and skipped whole, never read
//! partially. See [`crate::rafd`] for why there is no "read partially" outcome
//! anywhere in this crate.
//!
//! ## Encrypted, or merely unknown?
//!
//! A file in a Unity data directory whose header matches no known signature is
//! one of two things, and the report has to say which, because the remedies are
//! opposite: an unknown format is worth reporting and worth capturing at
//! runtime, and an encrypted one cannot be opened by anything Taarib will ever
//! ship.
//!
//! The rule, in order:
//!
//! 1. **The container's own declaration wins.** A `UnityFS` bundle with flag
//!    `0x400` set says it is encrypted, and that is a fact rather than an
//!    inference. It is refused as
//!    [`SababRafd::Mushaffar`] and no measurement is taken.
//! 2. **Otherwise, positive evidence is required**, and it is all four of: the
//!    file's *name* says it should have been a container (a `.bundle`, a
//!    `.assets`, a `levelN`); no signature matched; the file is at least
//!    [`HAJM_AYYINA`] bytes, below which a byte histogram means nothing; and the
//!    Shannon entropy of its first [`HAJM_AYYINA`] bytes is at or above
//!    [`HADD_ANTRUBIYA`] bits per byte. Only then is it
//!    [`SababRafd::Mushaffar`].
//! 3. **Everything else is [`SababRafd::SighaMajhula`]**, and the bias is
//!    deliberate. Calling an unknown format "encrypted" tells the user their
//!    game cannot be translated, which is a wrong answer they cannot check;
//!    calling an encrypted file "an unknown format" offers them runtime capture,
//!    which works regardless. The failure mode of guessing low is a redundant
//!    suggestion, and of guessing high is a user giving up on a game Taarib
//!    could have handled.
//!
//! The heuristic is honest about what it cannot see. A bundle whose header was
//! `XORed` with a short repeating key has exactly the entropy of the plaintext
//! header — low — and is reported as an unknown format. That is the safe
//! direction, and it is why the entropy test is only ever allowed to *raise* a
//! refusal to `Mushaffar` and never to lower one.
//!
//! ## Which locale is the source
//!
//! A game using Unity Localization ships one `StringTable` per locale, all of
//! them holding the same keys with the same entry ids. The Stalked 3 ships
//! eleven, which is eleven renderings of three hundred and forty-eight strings;
//! reading all of them put four thousand two hundred and sixty-five rows in
//! front of a translator for a game that has three hundred and forty-eight, and
//! a machine-translation run then paid for the other eleven twelfths.
//!
//! **Nothing in the game's files says which locale it draws**, and that is a
//! finding rather than a gap in this reader. The locale set is in
//! `localization-locales`, and every `Locale` in The Stalked 3 carries the same
//! sort order and no default flag. The asset that *would* say —
//! `LocalizationSettings`, with its startup selectors and project locale — is in
//! the preloaded assets of `globalgamemanagers.assets`, whose type tree this
//! build stripped, so it is unreadable there for the same reason the rest of the
//! game's own components are. The Addressables catalog names the eleven `Locale`
//! assets and no default among them. What the game's assembly does record is a
//! `Languageselector` writing the player's pick to `PlayerPrefs` — per machine,
//! per user, outside the install, and therefore something this walk must not
//! read: extraction derives identity from what it sees, and a table that
//! depended on the player's saved language would give two contributors of the
//! same build two different projects.
//!
//! So the choice is Taarib's, it is stated, and it is made on what the game
//! ships rather than assumed. [`ikhtar_lugha`] takes the locales seen and
//! answers with [`LUGHAT_MASDAR`] when the game ships it, with the one locale in
//! that language when the game ships a regional spelling of it and no other, and
//! with nothing at all otherwise — and "nothing" means every locale is read,
//! exactly as before, with the locale folded into the engine key so none of them
//! collides with another. That last arm is what keeps this from being an
//! assumption that games are written in English: a game shipping only Japanese
//! and Korean has no locale this build can rank, and it is not ranked.
//!
//! The decision is taken **after** the walk, in [`ikhtim_tawtin`], and not while
//! reading. Each locale is a separate Addressables bundle, so the file in hand
//! knows its own locale and nothing about the ten beside it; deciding per file
//! would mean depending on the order the package happens to name its groups in.
//! The rows are held in [`kaain::MajmuatLugha`] until the full list exists.
//!
//! What was set aside is in the report, per container, as
//! [`SababRafd::LughaGhayrMukhtara`] with its locale and its row count — and
//! that refusal answers `false` to [`SababRafd::khasara`], because every key
//! those rows carry is in the table once, under the locale that was read.

pub mod hawiya;
pub mod kaain;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use walkdir::WalkDir;

use crate::jadwal::JadwalNusus;
use crate::rafd::{SababRafd, TaqreerRafd};

use hawiya::{AQSA_HAJM_MALAF, Huzma, KhataQira, Mulsal, TawqiHuzma, tul_u64};
use kaain::{MajmuatLugha, istakhrij_mulsal};

// ---------------------------------------------------------------------------
// Ceilings
// ---------------------------------------------------------------------------

/// How deep below the game's root the walk descends.
///
/// Sixteen. A Unity addressables tree is `Data/StreamingAssets/aa/<platform>/`,
/// four levels, and a game shipping its DLC as nested packs adds a few more.
/// Sixteen is far past anything observed and stops a symlink loop that
/// `follow_links(false)` did not already stop — a junction on Windows is not a
/// symlink and `walkdir` follows it.
pub const AQSA_UMQ_MASH: usize = 16;

/// How many directory entries the walk visits before it gives up.
///
/// Two hundred thousand. A large game's data directory holds a few thousand
/// files; two hundred thousand means the walk was pointed at a drive root rather
/// than at a game, and continuing would spend minutes producing nothing. The
/// ceiling is reported rather than silently hit, so the user learns their path
/// was wrong instead of learning their game has no text.
pub const AQSA_MADAKHIL: usize = 200_000;

/// How many containers are actually opened and read.
///
/// Eight thousand. `sharedassets*.assets` and `level*` run to the low hundreds
/// even for a large game, and an addressables build adds a bundle per group.
/// Past this the extraction is reading something that is not a game.
pub const AQSA_HAWIYAT: usize = 8192;

/// How many bytes the entropy test looks at.
///
/// Four kibibytes. A byte histogram over less than this is noise — 256 buckets
/// need enough samples to be a distribution — and more than this measures the
/// payload rather than the header, which is the region the question is about.
pub const HAJM_AYYINA: usize = 4096;

/// The entropy at or above which a header is treated as ciphertext.
///
/// Seven and a half bits per byte, out of a possible eight. Compressed and
/// encrypted data both sit above this; structured binary with lengths, offsets
/// and ASCII names sits well below it, usually under six. The threshold is only
/// consulted for a file that already failed every signature test, so compressed
/// containers — which have signatures — never reach it.
pub const HADD_ANTRUBIYA: f64 = 7.5;

/// How many walk errors are reported individually before they are summarised.
///
/// Thirty-two. A game directory the user does not have permission to read
/// produces one error per entry, and a report that is nine thousand identical
/// permission failures is one nobody reads to the end of.
pub const AQSA_AKHTA_MASH: usize = 32;

/// How many data directories one game root may have.
///
/// Eight. A Unity game ships one `<name>_Data`, and a launcher bundling two
/// builds side by side ships two. Eight is generous and stops a directory full
/// of unrelated `*_Data` folders from turning one extraction into forty.
pub const AQSA_JUDHUR: usize = 8;

/// The files whose presence marks a directory as a Unity data directory.
///
/// Every Unity player writes `globalgamemanagers` — it holds the build settings
/// the runtime reads before anything else — except a build that packed
/// everything into `data.unity3d`, which mobile and some console-derived PC
/// ports do. Testing for these rather than for a `_Data` suffix means a game
/// whose folder was renamed is still found, and a folder called `Foo_Data` that
/// holds somebody's spreadsheets is not.
pub const ALAMAT_BAYANAT: &[&str] = &[
    "globalgamemanagers",
    "resources.assets",
    "data.unity3d",
    "unity_builtin_extra",
];

/// The locale Taarib reads a multi-locale Unity game's string tables from.
///
/// `en`. **This is a statement about Taarib, not about the game**, and the
/// difference is the whole reason it is written down here rather than decided in
/// [`ikhtar_lugha`]'s body. It is the source language this product already
/// records for a project — `mashru.lugha_masdar` defaults to `'en'` in
/// `taarib-makhzan`'s schema and `taarib-tarjama`'s memory carries the same
/// column — so a Unity extraction that read some other locale would hand the
/// translation runner text in a language the project says it is not translating
/// from.
///
/// It is a *preference*, resolved against what the game actually ships, and it
/// never becomes an assertion about a game's authoring language:
/// [`ikhtar_lugha`] declines to choose at all when this locale is not on offer,
/// and a game shipping only Japanese and Korean is read whole. Extraction will
/// not be the place that decides a Japanese game was written in English.
///
/// Making it settable belongs to the project record rather than to this
/// constant — see [`ikhtar_lugha`].
pub const LUGHAT_MASDAR: &str = "en";

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Reads every Unity container under `jidhr` and returns what came out and what
/// did not.
///
/// Never fails and never panics. A directory that does not exist, a file that
/// cannot be opened, a bundle in a compression this build refuses and an object
/// whose layout was stripped are all *outcomes*, recorded in the report beside
/// the table — see [`crate::rafd`] for why refusal is a first-class output
/// rather than an error.
///
/// The table may legitimately be empty. An IL2CPP release build with the type
/// tree stripped produces exactly that: nothing extracted, one refusal per
/// container, and a report whose remedy is to run the game once with capture on.
#[must_use]
pub fn istakhrij(jidhr: &Path) -> (JadwalNusus, TaqreerRafd) {
    let mut jadwal = JadwalNusus::jadeed();
    let mut taqreer = TaqreerRafd::jadeed();
    let mut hisab = Hisab::default();

    let mut muallaqa: Vec<JadwalMuallaq> = Vec::new();

    let (judhur, wujidat) = judhur_bayanat(jidhr);
    for mabda in &judhur {
        imshi(
            jidhr,
            mabda,
            &mut hisab,
            &mut jadwal,
            &mut taqreer,
            &mut muallaqa,
        );
        if hisab.tawaqqaf {
            break;
        }
    }

    ikhtim_tawtin(muallaqa, &mut jadwal, &mut taqreer);

    if hisab.akhta > AQSA_AKHTA_MASH {
        taqreer.sajjil(
            masar_nisbi(jidhr, jidhr),
            None,
            SababRafd::TajawuzHadd {
                hadd: "unreadable directory entries".to_owned(),
                qeema: tul_u64(hisab.akhta),
                saqf: tul_u64(AQSA_AKHTA_MASH),
            },
        );
    }

    // Nothing found and nothing that looked like a Unity install. Said plainly,
    // because the alternative is a report reading "0 containers read" that a
    // user reasonably interprets as "this game has no text" when what actually
    // happened is that they pointed the tool at the wrong folder.
    if !wujidat && hisab.maqrua == 0 {
        taqreer.sajjil(
            masar_nisbi(jidhr, jidhr),
            None,
            SababRafd::SighaMajhula {
                wujid: format!(
                    "`{}`, which holds no Unity data directory — none of {} is anywhere under it",
                    jidhr.display(),
                    ALAMAT_BAYANAT.join(", ")
                ),
            },
        );
    }

    (jadwal, taqreer)
}

/// One Unity Localization string table, held until the walk is over.
///
/// Carries where it came from — which the report needs — and the index of the
/// container's own read entry, so that the rows that end up in the table are
/// counted against the container they came out of rather than disappearing from
/// the report's own arithmetic.
#[derive(Debug)]
struct JadwalMuallaq {
    hawiya: String,
    asl: Option<String>,
    fahras_qira: usize,
    majmua: MajmuatLugha,
}

/// The running totals one extraction keeps across every data directory.
///
/// Shared rather than per-root, because the ceilings are about this process's
/// time and memory: a game with four data directories does not get four times
/// the budget.
#[derive(Debug, Default)]
struct Hisab {
    zurat: usize,
    maqrua: usize,
    akhta: usize,
    tawaqqaf: bool,
}

/// Where to start walking, and whether the answer was found or assumed.
///
/// Returning the second half matters: a root that *was* identified as a Unity
/// install and produced nothing is a different report from one that was never
/// identified at all, and only the second is worth telling the user they may
/// have chosen the wrong folder.
///
/// The search is deliberately shallow — the game root, its immediate children,
/// and the two fixed macOS and Linux layouts. A recursive hunt for
/// `globalgamemanagers` would find one inside a mod manager's backup folder and
/// extract a build the player is not running.
fn judhur_bayanat(jidhr: &Path) -> (Vec<PathBuf>, bool) {
    if huwa_jidhr_bayanat(jidhr) {
        return (vec![jidhr.to_path_buf()], true);
    }

    let mut judhur: Vec<PathBuf> = Vec::new();

    // `MyGame_Data` beside `MyGame.exe`, which is every Windows and Linux build.
    if let Ok(qira) = std::fs::read_dir(jidhr) {
        let mut asma: Vec<PathBuf> = qira
            .flatten()
            .map(|madkhal| madkhal.path())
            .filter(|masar| masar.is_dir())
            .collect();
        // Sorted for the same reason the walk is: two machines enumerating the
        // same install must produce the same identities.
        asma.sort();
        for masar in asma {
            if judhur.len() >= AQSA_JUDHUR {
                break;
            }
            if huwa_jidhr_bayanat(&masar) {
                judhur.push(masar);
            }
        }
    }

    // `MyGame.app/Contents/Resources/Data`, and the layout a few Linux ports use.
    for nisbi in ["Contents/Resources/Data", "Data"] {
        if judhur.len() >= AQSA_JUDHUR {
            break;
        }
        let masar = jidhr.join(nisbi);
        if huwa_jidhr_bayanat(&masar) && !judhur.contains(&masar) {
            judhur.push(masar);
        }
    }

    if judhur.is_empty() {
        // Nothing recognised. Walk what was given anyway: a contributor pointing
        // at an already-extracted folder of `.bundle` files is a real case, and
        // refusing it because it has no `globalgamemanagers` would refuse a
        // directory this reader handles perfectly well.
        return (vec![jidhr.to_path_buf()], false);
    }
    (judhur, true)
}

/// Whether a directory is a Unity data directory.
fn huwa_jidhr_bayanat(masar: &Path) -> bool {
    if !masar.is_dir() {
        return false;
    }
    ALAMAT_BAYANAT
        .iter()
        .any(|alama| masar.join(alama).is_file())
}

/// Walks one data directory and reads every container in it.
fn imshi(
    jidhr: &Path,
    mabda: &Path,
    hisab: &mut Hisab,
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muallaqa: &mut Vec<JadwalMuallaq>,
) {
    // Sorted, because identity and duplicate grouping depend on the order
    // containers are read in. See this module's header.
    let mashi = WalkDir::new(mabda)
        .max_depth(AQSA_UMQ_MASH)
        .follow_links(false)
        .sort_by_file_name();

    for natija in mashi {
        if hisab.zurat >= AQSA_MADAKHIL {
            taqreer.sajjil(
                masar_nisbi(jidhr, mabda),
                None,
                SababRafd::TajawuzHadd {
                    hadd: "directory entries under the game root".to_owned(),
                    qeema: tul_u64(hisab.zurat),
                    saqf: tul_u64(AQSA_MADAKHIL),
                },
            );
            hisab.tawaqqaf = true;
            return;
        }
        hisab.zurat = hisab.zurat.saturating_add(1);

        let madkhal = match natija {
            Ok(madkhal) => madkhal,
            Err(khata) => {
                hisab.akhta = hisab.akhta.saturating_add(1);
                if hisab.akhta <= AQSA_AKHTA_MASH {
                    let masar = khata
                        .path()
                        .map_or_else(|| mabda.to_path_buf(), Path::to_path_buf);
                    taqreer.sajjil(
                        masar_nisbi(jidhr, &masar),
                        None,
                        SababRafd::TaadhurQira {
                            sabab: khata.to_string(),
                        },
                    );
                }
                continue;
            },
        };
        if !madkhal.file_type().is_file() {
            continue;
        }

        // Lossy on purpose. A non-UTF-8 filename is possible on Linux and the
        // matching below is all ASCII, so a replacement character can only make
        // a name fail to match — never make the wrong one match.
        let ism = madkhal.file_name().to_string_lossy().into_owned();
        let nisbi = masar_nisbi(jidhr, madkhal.path());

        if huwa_janibi(&ism) {
            // The other half of a container this walk already read. Recorded
            // rather than skipped in silence, because "why did my 400 MB
            // resources.assets.resS contribute nothing" deserves the answer
            // "because it is the raw texture and audio bytes of a file that did".
            taqreer.sajjil(nisbi, None, SababRafd::BilaNusus);
            continue;
        }

        let musamma = ism_hawiya(&ism);
        if !musamma && !fi_tadfuq(madkhal.path()) {
            continue;
        }

        if hisab.maqrua >= AQSA_HAWIYAT {
            taqreer.sajjil(
                nisbi,
                None,
                SababRafd::TajawuzHadd {
                    hadd: "containers opened".to_owned(),
                    qeema: tul_u64(hisab.maqrua),
                    saqf: tul_u64(AQSA_HAWIYAT),
                },
            );
            hisab.tawaqqaf = true;
            return;
        }

        let khareeta = match iftah(madkhal.path()) {
            Ok(khareeta) => khareeta,
            Err(sabab) => {
                // A file the walk found by name and could not open is worth
                // reporting; one it only guessed at is not, or every unreadable
                // video in StreamingAssets becomes a refusal.
                if musamma {
                    taqreer.sajjil(nisbi, None, sabab);
                }
                continue;
            },
        };
        let bayt: &[u8] = &khareeta;

        // A file under StreamingAssets with an unrecognised name earns a report
        // only if it looks like a container. Everything else there — audio,
        // video, configuration — is not a thing this module failed to read.
        if !musamma && TawqiHuzma::min_bayt(bayt).is_none() && !Mulsal::yabdu_mulsalan(bayt) {
            continue;
        }

        hisab.maqrua = hisab.maqrua.saturating_add(1);
        iqra_hawiya(bayt, &ism, &nisbi, jadwal, taqreer, muallaqa);
    }
}

// ---------------------------------------------------------------------------
// Finding the containers
// ---------------------------------------------------------------------------

/// A path relative to the game's root, with forward slashes on every platform.
///
/// The separator is normalised because this string becomes
/// [`MawqiNass::hawiya`][crate::jadwal::MawqiNass::hawiya], which is hashed into
/// every identity in the file. A project extracted on Windows and re-extracted
/// on Linux would otherwise diff as a total rewrite, which is precisely the
/// failure the identity model exists to prevent.
fn masar_nisbi(jidhr: &Path, masar: &Path) -> String {
    let nisbi = masar.strip_prefix(jidhr).unwrap_or(masar);
    let nassi = nisbi.to_string_lossy();
    if nassi.is_empty() {
        ".".to_owned()
    } else {
        nassi.replace('\\', "/")
    }
}

/// Whether a filename says this file is a Unity container.
///
/// A name test rather than a content test, and it runs first for a reason: a
/// file the *name* promises is a container gets a refusal in the report when it
/// turns out not to be readable, and one the name promises nothing about does
/// not. That is what keeps the report about the game's text instead of about
/// every `.mp4` in `StreamingAssets`.
fn ism_hawiya(ism: &str) -> bool {
    let saghir = ism.to_ascii_lowercase();
    for lahiqa in [".assets", ".bundle", ".unity3d", ".assetbundle"] {
        if saghir.ends_with(lahiqa) {
            return true;
        }
    }
    if saghir == "globalgamemanagers" || saghir.starts_with("globalgamemanagers.") {
        return true;
    }
    if saghir == "data.unity3d" || saghir == "unity_builtin_extra" {
        return true;
    }
    // `level0`, `level1`, … — scene files, no extension. The digit test matters:
    // `level0.resS` must not match here, and `leveleditor.dll` must not either.
    saghir
        .strip_prefix("level")
        .is_some_and(|baqi| !baqi.is_empty() && baqi.chars().all(|harf| harf.is_ascii_digit()))
}

/// Whether a filename is the raw side-car of a container rather than a container.
///
/// `resources.assets.resS` holds the texture and audio bytes that
/// `resources.assets` points into. It has no structure of its own, no header and
/// no strings, and reading it would produce nothing at any cost.
fn huwa_janibi(ism: &str) -> bool {
    let saghir = ism.to_ascii_lowercase();
    [".ress", ".resource", ".manifest", ".info"]
        .iter()
        .any(|lahiqa| saghir.ends_with(lahiqa))
}

/// Whether a path is inside `StreamingAssets`.
///
/// Addressables bundles live there under names that carry a content hash and
/// frequently no extension at all, so the name test cannot find them and the
/// signature test has to. Comparing components rather than searching the whole
/// path avoids matching a game installed in a folder someone called
/// `StreamingAssets`.
fn fi_tadfuq(masar: &Path) -> bool {
    masar.components().any(|juz| {
        juz.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("StreamingAssets")
    })
}

/// Opens and maps one container.
///
/// # Errors
///
/// A [`SababRafd`] rather than an [`std::io::Error`], because every failure here
/// is a refusal the report shows: a file that cannot be opened, an empty one,
/// and one above [`AQSA_HAJM_MALAF`] are three different things to tell a user
/// and only the first is about the filesystem.
fn iftah(masar: &Path) -> Result<Mmap, SababRafd> {
    let malaf = File::open(masar).map_err(|khata| SababRafd::TaadhurQira {
        sabab: khata.to_string(),
    })?;
    let tul = malaf
        .metadata()
        .map_err(|khata| SababRafd::TaadhurQira {
            sabab: khata.to_string(),
        })?
        .len();
    if tul == 0 {
        return Err(SababRafd::BilaNusus);
    }
    if tul > AQSA_HAJM_MALAF {
        return Err(SababRafd::TajawuzHadd {
            hadd: "container size".to_owned(),
            qeema: tul,
            saqf: AQSA_HAJM_MALAF,
        });
    }

    // SAFETY: `Mmap::map` is unsafe because the mapping's contents become
    // undefined if another process truncates or writes the file while it is
    // mapped. Extraction reads a game's *installed* files, which this crate
    // never writes — see the read-only guarantee in `crate` — and the mapping
    // lives only for the duration of this one container's read, so the window is
    // the few milliseconds it takes to parse a header and copy out strings. The
    // alternative is reading a two-gibibyte `data.unity3d` into a `Vec`, which
    // costs the memory the game itself needs on the machine doing the extraction.
    let khareeta = unsafe { Mmap::map(&malaf) };
    khareeta.map_err(|khata| SababRafd::TaadhurQira {
        sabab: khata.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Reading one container
// ---------------------------------------------------------------------------

/// Dispatches one container by what its first bytes actually are.
///
/// The signature decides, not the extension. A `.bundle` that is a
/// `SerializedFile` and a `.assets` that is a bundle both exist — Unity's own
/// build pipeline produces the first for a single-asset group — and trusting the
/// name over the header would refuse both.
fn iqra_hawiya(
    bayt: &[u8],
    ism: &str,
    hawiya: &str,
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muallaqa: &mut Vec<JadwalMuallaq>,
) {
    if TawqiHuzma::min_bayt(bayt).is_some() {
        iqra_huzma(bayt, hawiya, jadwal, taqreer, muallaqa);
        return;
    }
    if Mulsal::yabdu_mulsalan(bayt) {
        let _ = iqra_mulsal(bayt, hawiya, None, jadwal, taqreer, muallaqa);
        return;
    }
    taqreer.sajjil(hawiya.to_owned(), None, sabab_majhul(bayt, ism));
}

/// Reads a bundle and every `SerializedFile` inside it.
fn iqra_huzma(
    bayt: &[u8],
    hawiya: &str,
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muallaqa: &mut Vec<JadwalMuallaq>,
) {
    let huzma = match Huzma::iqra(bayt) {
        Ok(huzma) => huzma,
        Err(khata) => {
            taqreer.sajjil(hawiya.to_owned(), None, sabab_min_qira(&khata));
            return;
        },
    };

    let muharrik = if huzma.isdar_muharrik().is_empty() {
        huzma.isdar_muharrik_ala().to_owned()
    } else {
        huzma.isdar_muharrik().to_owned()
    };

    for uqda in huzma.uqad() {
        let Some(mihtawa) = huzma.mihtawa(uqda) else {
            taqreer.sajjil(
                hawiya.to_owned(),
                Some(uqda.masar.clone()),
                SababRafd::Talif {
                    sabab: format!(
                        "directory entry `{}` claims {} byte(s) at {}, outside a {}-byte payload",
                        uqda.masar,
                        uqda.hajm,
                        uqda.izaha,
                        huzma.bayanat().len()
                    ),
                },
            );
            continue;
        };

        if huwa_janibi(&uqda.masar) {
            taqreer.sajjil(
                hawiya.to_owned(),
                Some(uqda.masar.clone()),
                SababRafd::BilaNusus,
            );
            continue;
        }
        if TawqiHuzma::min_bayt(mihtawa).is_some() {
            // Unity does not nest bundles, and a reader that unwrapped one
            // anyway would be following a structure no shipped build produces.
            taqreer.sajjil(
                hawiya.to_owned(),
                Some(uqda.masar.clone()),
                SababRafd::SighaMajhula {
                    wujid: "a Unity bundle nested inside another bundle".to_owned(),
                },
            );
            continue;
        }
        if !Mulsal::yabdu_mulsalan(mihtawa) {
            taqreer.sajjil(
                hawiya.to_owned(),
                Some(uqda.masar.clone()),
                SababRafd::BilaNusus,
            );
            continue;
        }

        let _ = iqra_mulsal(
            mihtawa,
            hawiya,
            Some(&uqda.masar),
            jadwal,
            taqreer,
            muallaqa,
        );
    }

    // The bundle itself is recorded as read even when every file inside it was
    // refused, with a count of zero. A container that produced nothing and a
    // container that was never opened are different facts, and the report has to
    // be able to tell a user which one happened.
    taqreer.sajjil_qira(
        hawiya.to_owned(),
        0,
        format!(
            "{} bundle, format {}, {} block(s), {} file(s), written by {muharrik}",
            huzma.tawqi().ism(),
            huzma.isdar(),
            huzma.kutal().len(),
            huzma.uqad().len()
        ),
    );
}

/// Reads one `SerializedFile` and folds its strings into the table.
///
/// Returns how many entries it produced, which is what the report shows beside
/// the container.
fn iqra_mulsal(
    bayt: &[u8],
    hawiya: &str,
    asl: Option<&str>,
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    muallaqa: &mut Vec<JadwalMuallaq>,
) -> usize {
    let mulsal = match Mulsal::iqra(bayt) {
        Ok(mulsal) => mulsal,
        Err(khata) => {
            taqreer.sajjil(
                hawiya.to_owned(),
                asl.map(str::to_owned),
                sabab_min_qira(&khata),
            );
            return 0;
        },
    };

    let hasila = istakhrij_mulsal(&mulsal, hawiya, asl);
    let adad = hasila.madakhil.len();
    for madkhal in hasila.madakhil {
        jadwal.adif(madkhal);
    }
    for sabab in hasila.marfudat {
        taqreer.sajjil(hawiya.to_owned(), asl.map(str::to_owned), sabab);
    }

    // "Declared but empty" is worth saying out loud. A build that says it kept
    // the layout and did not is a broken build; one that says it stripped it is
    // an intentional one, and only the first is worth a bug report to the
    // game's publisher.
    let wasf_shajara = if mulsal.ladayhi_shajarat_anwa() {
        "type tree present"
    } else if mulsal.yudai_shajarat_anwa() {
        "type tree declared but empty"
    } else {
        "type tree stripped"
    };
    let muharrik = if mulsal.isdar_muharrik().is_empty() {
        "an unnamed engine build"
    } else {
        mulsal.isdar_muharrik()
    };
    let asasi = format!(
        "SerializedFile format {}, {} object(s), {wasf_shajara}, written by {muharrik}",
        mulsal.isdar(),
        mulsal.kaainat().len()
    );
    let wasf = match asl {
        Some(asl) => format!("{asasi} ({asl})"),
        None => asasi,
    };
    taqreer.sajjil_qira(hawiya.to_owned(), adad, wasf);
    // The read entry this file's localization rows will be counted against once
    // the source locale is chosen. Taken here, immediately after the entry was
    // pushed, because that is the only moment its position is known without
    // searching the report for a container name that is not unique — a bundle
    // holding four `SerializedFile`s records four entries under one name.
    let fahras_qira = taqreer.maqrua.len().saturating_sub(1);
    for majmua in hasila.tawtin {
        muallaqa.push(JadwalMuallaq {
            hawiya: hawiya.to_owned(),
            asl: asl.map(str::to_owned),
            fahras_qira,
            majmua,
        });
    }
    // Only when there *was* a tree. A stripped file has no index either, and
    // saying so twice — once as a missing layout and once as a truncated index —
    // would double-count one fact in the report.
    if hasila.manqus && mulsal.ladayhi_shajarat_anwa() {
        // Said once per file rather than once per string: the object index was
        // truncated, so some entries carry a component name where they would
        // have carried a full object path, and a contributor comparing two
        // extractions deserves to know why the positions differ.
        taqreer.sajjil(
            hawiya.to_owned(),
            asl.map(str::to_owned),
            SababRafd::TajawuzHadd {
                hadd: "objects indexed for structural paths".to_owned(),
                qeema: tul_u64(mulsal.kaainat().len()),
                saqf: tul_u64(kaain::AQSA_KAAINAT_FAHRAS),
            },
        );
    }
    adad
}

// ---------------------------------------------------------------------------
// Unity Localization: which locale the project gets
// ---------------------------------------------------------------------------

/// The locale a Unity game's string tables are read from, out of the ones it
/// ships.
///
/// Three answers, in order:
///
/// 1. [`LUGHAT_MASDAR`] itself, when the game ships it.
/// 2. The one locale whose language subtag is [`LUGHAT_MASDAR`]'s, when the game
///    ships a regional spelling of it and no other — `en-GB` alone stands in for
///    `en`. Two of them and no bare one is a choice this function will not make
///    silently: picking `en-GB` over `en-AU` would put one region's spelling in
///    the table as the source text of the whole game, which is the same refusal
///    [`crate::unreal`] makes for a declared native culture with no file.
/// 3. `None` — meaning **read every locale**, with the locale in the engine key,
///    exactly as this reader did before it chose at all. This is the arm that
///    keeps the whole mechanism from being an assumption that games are written
///    in English.
///
/// Comparison is ASCII-case-insensitive because a locale code's region subtag is
/// conventionally upper case and its language subtag is not — Unity writes
/// `pt-BR` and `zh-Hans` — and a case-sensitive match would make `EN` a locale
/// this build had never heard of.
///
/// ## Why the answer is not settable here
///
/// A contributor who owns a German copy, or who would rather translate from
/// English on a German install, is asking for a *project* setting and not a
/// parameter: re-extraction is a diff against the same project, so a source
/// locale that this call took as an argument and nothing stored would revert to
/// the default on the next scan and every row in the project would orphan at
/// once. The place it belongs is the project record beside `lugha_masdar`, which
/// is a change to [`crate::mashru`] and to what the workshop writes, and it is
/// deferred to that rather than half-built here. Until then the report names
/// both the locale that was read and the ones that were not, so the choice is
/// visible even though it is not yet adjustable.
#[must_use]
pub fn ikhtar_lugha(mawjuda: &BTreeSet<String>) -> Option<String> {
    if let Some(ramz) = mawjuda
        .iter()
        .find(|ramz| ramz.eq_ignore_ascii_case(LUGHAT_MASDAR))
    {
        return Some(ramz.clone());
    }
    let mut murashahun = mawjuda
        .iter()
        .filter(|ramz| lugha_min_ramz(ramz).eq_ignore_ascii_case(LUGHAT_MASDAR));
    let awwal = murashahun.next()?;
    if murashahun.next().is_some() {
        return None;
    }
    Some(awwal.clone())
}

/// The language subtag of a locale code: `pt` out of `pt-BR`.
fn lugha_min_ramz(ramz: &str) -> &str {
    ramz.split(['-', '_']).next().unwrap_or(ramz)
}

/// What one container's string tables contributed, once the choice was made.
#[derive(Debug, Default)]
struct HisabTawtin {
    lughat: BTreeSet<String>,
    mahfuza: usize,
    manhiya: usize,
}

/// Chooses the source locale and folds the string tables that survive it into
/// the table.
///
/// Everything this function decides needs the whole walk to have happened, which
/// is the only reason it is a second pass — see this module's header.
fn ikhtim_tawtin(
    muallaqa: Vec<JadwalMuallaq>,
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
) {
    if muallaqa.is_empty() {
        return;
    }
    let mawjuda: BTreeSet<String> = muallaqa
        .iter()
        .filter_map(|muallaq| muallaq.majmua.lugha.clone())
        .collect();
    let kathira = mawjuda.len() > 1;
    let mukhtara = if kathira {
        ikhtar_lugha(&mawjuda)
    } else {
        // One locale is not a choice, and saying it was one would put a decision
        // in the report of every single-language game in the library.
        None
    };

    let mut hisabat: BTreeMap<usize, HisabTawtin> = BTreeMap::new();
    let mut manhiya: BTreeMap<(String, Option<String>, String), usize> = BTreeMap::new();

    for muallaq in muallaqa {
        let JadwalMuallaq {
            hawiya,
            asl,
            fahras_qira,
            majmua,
        } = muallaq;
        let hisab = hisabat.entry(fahras_qira).or_default();
        if let Some(lugha) = &majmua.lugha {
            let _ = hisab.lughat.insert(lugha.clone());
        }

        // A table with no locale code of its own is kept whatever was chosen: it
        // cannot be compared against the choice, and setting aside a table
        // because it failed to name itself would lose rows nothing else holds.
        let hujiba = match (mukhtara.as_deref(), majmua.lugha.as_deref()) {
            (Some(mukhtara), Some(lugha)) if !lugha.eq_ignore_ascii_case(mukhtara) => {
                Some(lugha.to_owned())
            },
            _ => None,
        };

        let adad = majmua.madakhil.len();
        if let Some(lugha) = hujiba {
            hisab.manhiya = hisab.manhiya.saturating_add(adad);
            *manhiya.entry((hawiya, asl, lugha)).or_insert(0_usize) += adad;
            continue;
        }
        hisab.mahfuza = hisab.mahfuza.saturating_add(adad);
        for madkhal in majmua.madakhil {
            jadwal.adif(madkhal);
        }
    }

    let asma_kull = asma_lughat(&mawjuda);
    for (fahras, hisab) in hisabat {
        let Some(qira) = taqreer.maqrua.get_mut(fahras) else {
            continue;
        };
        qira.adad = qira.adad.saturating_add(hisab.mahfuza);
        qira.wasf.push_str(&wasf_tawtin(
            &hisab,
            mukhtara.as_deref(),
            kathira,
            &asma_kull,
        ));
    }

    // Nothing reaches `manhiya` unless a locale was chosen, so the outer `if` is
    // what makes that a fact of the code rather than a comment about it.
    if let Some(mukhtara) = mukhtara {
        for ((hawiya, asl, lugha), adad) in manhiya {
            taqreer.sajjil(
                hawiya,
                asl,
                SababRafd::LughaGhayrMukhtara {
                    lugha,
                    mukhtara: mukhtara.clone(),
                    adad,
                },
            );
        }
    }
}

/// The clause appended to a container's read entry, saying what its string
/// tables were and what became of them.
fn wasf_tawtin(
    hisab: &HisabTawtin,
    mukhtara: Option<&str>,
    kathira: bool,
    asma_kull: &str,
) -> String {
    let asma = asma_lughat(&hisab.lughat);
    match mukhtara {
        Some(mukhtara) if hisab.manhiya > 0 && hisab.mahfuza > 0 => format!(
            "; Unity Localization string table(s) in [{asma}], of which {} string(s) were read \
             in {mukhtara} and {} set aside",
            hisab.mahfuza, hisab.manhiya
        ),
        Some(mukhtara) if hisab.manhiya > 0 => format!(
            "; Unity Localization string table(s) in [{asma}], {} string(s) set aside — \
             {mukhtara} is the locale being read",
            hisab.manhiya
        ),
        Some(mukhtara) => {
            format!(
                "; Unity Localization string table(s) in [{asma}], read as the source locale ({mukhtara}) out of [{asma_kull}]"
            )
        },
        None if kathira => format!(
            "; Unity Localization string table(s) in [{asma}], read — this game ships \
             [{asma_kull}] and none of them is the locale Taarib translates from \
             ({LUGHAT_MASDAR}), so every locale is kept with the locale in the engine key"
        ),
        None => format!("; Unity Localization string table(s) in [{asma}]"),
    }
}

/// A locale set as the report prints it.
fn asma_lughat(lughat: &BTreeSet<String>) -> String {
    if lughat.is_empty() {
        return "no declared locale".to_owned();
    }
    lughat
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Refusal
// ---------------------------------------------------------------------------

/// Turns a container-level read failure into the refusal the report shows.
fn sabab_min_qira(khata: &KhataQira) -> SababRafd {
    match khata {
        KhataQira::TajawuzHadd { hadd, qeema, saqf } => SababRafd::TajawuzHadd {
            hadd: (*hadd).to_owned(),
            qeema: *qeema,
            saqf: *saqf,
        },
        KhataQira::SighaMajhula { wujid } => SababRafd::SighaMajhula {
            wujid: wujid.clone(),
        },
        KhataQira::Mushaffar { wasf } => SababRafd::Mushaffar { wasf: wasf.clone() },
        KhataQira::IsdarGhayrMadum {
            sigha,
            wujid,
            madum,
        } => SababRafd::IsdarGhayrMadum {
            sigha: (*sigha).to_owned(),
            wujid: wujid.clone(),
            madum: (*madum).to_owned(),
        },
        // An unresolvable field name is not a damaged container: the file is
        // intact and this build simply cannot name one of its fields. Reporting
        // it as damage would send the user to verify their game files, which
        // would come back clean and teach them the report is unreliable.
        KhataQira::IsmMajhul { .. } => SababRafd::BilaShajaratAnwa {
            naw_kaen: None,
            adad: 1,
        },
        KhataQira::MalafQaseer { .. }
        | KhataQira::HaqlTalif { .. }
        | KhataQira::NassGhayrSalih { .. }
        | KhataQira::DaghtTalif { .. } => SababRafd::Talif {
            sabab: khata.to_string(),
        },
    }
}

/// Decides between "encrypted" and "a format this build does not read".
///
/// The full argument is in this module's header. In short: positive evidence is
/// required for `Mushaffar`, the evidence is a high-entropy header in a file
/// whose *name* promised a container, and everything else is `SighaMajhula` —
/// because the cost of guessing "unknown" for an encrypted file is a redundant
/// suggestion, and the cost of guessing "encrypted" for an unknown one is a user
/// abandoning a game this product could have handled.
fn sabab_majhul(bayt: &[u8], ism: &str) -> SababRafd {
    if ism_hawiya(ism) && bayt.len() >= HAJM_AYYINA {
        let ayyina = bayt.get(..HAJM_AYYINA).unwrap_or(bayt);
        let qeema = antrubiya(ayyina);
        if qeema >= HADD_ANTRUBIYA {
            return SababRafd::Mushaffar {
                wasf: format!(
                    "no Unity signature, and the first {HAJM_AYYINA} bytes of `{ism}` carry \
                     {qeema:.2} bits of entropy per byte, at or above the {HADD_ANTRUBIYA} \
                     this build treats as ciphertext"
                ),
            };
        }
    }
    SababRafd::SighaMajhula {
        wujid: format!("`{ism}`, whose header matches no Unity container signature"),
    }
}

/// Shannon entropy of a byte run, in bits per byte.
///
/// Zero for a run of one repeated byte and eight for a uniform one. Written out
/// rather than taken from a crate because it is nine lines and because the one
/// place it is used needs it to be exactly this: a histogram over the *bytes*,
/// with no windowing and no compression-ratio proxy, so that the threshold in
/// [`HADD_ANTRUBIYA`] means what the module header says it means.
fn antrubiya(bayt: &[u8]) -> f64 {
    if bayt.is_empty() {
        return 0.0;
    }
    let mut tikrar = [0_u32; 256];
    for qeema in bayt {
        if let Some(khana) = tikrar.get_mut(usize::from(*qeema)) {
            *khana = khana.saturating_add(1);
        }
    }
    let kulli = f64::from(u32::try_from(bayt.len()).unwrap_or(u32::MAX));
    let mut natija = 0.0_f64;
    for khana in tikrar {
        if khana == 0 {
            continue;
        }
        let ihtimal = f64::from(khana) / kulli;
        natija -= ihtimal * ihtimal.log2();
    }
    natija
}
