//! التوحيد — deciding when two catalogue entries are one game.
//!
//! A user with Steam, Heroic and Lutris installed can have the same game listed
//! three times, and the three entries agree on almost nothing. Steam calls it
//! `HELLDIVERS™ 2`, Lutris calls it `helldivers-2`, and Heroic reports whatever
//! Epic's catalogue says. Their identifiers are from different namespaces. Their
//! install paths may or may not be the same directory. Only one of them knows a
//! build number.
//!
//! ## Why not the title
//!
//! Because titles are not identifiers and treating them as ones fails in both
//! directions at once. Trademark symbols, edition suffixes, regional subtitles
//! and Roman numerals make one game look like three; and `Resident Evil 2` (1998)
//! against `Resident Evil 2` (2019) makes two games look like one. A library that
//! merged those two would show the user one card, with one patch state, for two
//! games that share nothing but a name — and the patch it offered would be for
//! whichever the merge happened to keep.
//!
//! Normalizing harder does not fix it. It makes the first failure rarer and the
//! second one *more* likely, because every normalization step removes exactly
//! the characters that distinguish editions.
//!
//! ## What is actually compared
//!
//! **The bytes on disk**, through [`BasmatMuhtawa`] — a fingerprint over the
//! install's shape rather than its contents. Two entries pointing at the same
//! directory are the same game because they are literally the same files. Two
//! entries pointing at different directories that hold the same executable, at
//! the same size, under the same relative path, are the same game installed
//! twice.
//!
//! Hashing whole games is not an option: a scan runs at startup and a
//! two-hundred-game library is terabytes. So the fingerprint is over the
//! **manifest of the install** — the relative paths and sizes of the largest
//! files in the top two levels — which is stable across copies, stable across
//! launchers, and cheap enough to compute for every game on every scan.
//!
//! ## The one thing this module will not do
//!
//! Merge on a *near* match. There is no similarity score and no threshold,
//! because a threshold is a number somebody lowers when a game they own does not
//! merge, and lowering it merges two games that are not the same. Entries either
//! produce the same fingerprint or they are separate games, and a user who
//! believes two separate cards are one game can say so by hand — a decision that
//! is recorded, reversible, and theirs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::MasdarLuba;

use crate::fahs::LubaMuktashafa;

/// How many files the fingerprint is built from.
///
/// Sixteen. Enough that two different games cannot plausibly agree on all of
/// them, few enough that the manifest is built from one shallow directory walk.
/// Taking the *largest* files rather than the first sixteen is what makes it
/// stable: a launcher that adds a log file or a settings file changes the small
/// end of the distribution and never the large end.
pub const ADAD_ASHHAD: usize = 16;

/// How deep the walk goes when building a fingerprint.
///
/// Two levels. Every engine this product supports puts its bulk — the pak files,
/// the asset bundles, the `data.win`, the executable — within two levels of the
/// install root. Going deeper costs directory reads on every scan and adds
/// nothing that changes an answer.
pub const UMQ_BAHTH: usize = 2;

/// How many entries the walk will visit before it stops.
///
/// Ten thousand. A bound rather than a tuning knob: a game with a
/// hundred-thousand-file asset tree would otherwise make a library scan take
/// minutes, and the sixteen largest files are found long before the limit.
pub const AQSA_MADAKHIL: usize = 10_000;

/// One file's contribution to an install's fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ShahidMalaf {
    /// Size in bytes. Ordered first, because the fingerprint takes the largest.
    hajm: u64,
    /// The path relative to the install root, with separators normalized.
    nisbi: String,
}

/// A fingerprint over an installation's shape.
///
/// Two installs of the same game produce the same value; two different games do
/// not. It is not a content hash and does not claim to be — it cannot tell a
/// patched install from an unpatched one, which is deliberate: a game the user
/// has already translated must still deduplicate against the same game in
/// another launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BasmatMuhtawa(Basma);

impl BasmatMuhtawa {
    /// The fingerprint as the string that reaches the store and the interface.
    #[must_use]
    pub fn nass(&self) -> String {
        self.0.to_string()
    }

    /// The underlying fingerprint, for a caller that stores it alongside others.
    #[must_use]
    pub const fn basma(&self) -> Basma {
        self.0
    }
}

/// Builds an install's fingerprint.
///
/// Walks two levels, keeps the sixteen largest files, sorts them by size and
/// then by path, and hashes that manifest. **Sorting is what makes it stable
/// across launchers**: two directory listings of the same tree arrive in
/// whatever order each filesystem returns, and a fingerprint over unsorted input
/// would differ between two reads of the same directory.
///
/// Returns [`None`] when the directory cannot be read or holds no files at all.
/// A game with no fingerprint is never merged with anything, which is the safe
/// direction — an unmerged duplicate is a visible annoyance and a wrong merge
/// loses a game.
#[must_use]
pub fn basmat_tathbeet(jidhr: &Path) -> Option<BasmatMuhtawa> {
    let mut shuhud: Vec<ShahidMalaf> = Vec::new();
    let mut adad: usize = 0;
    ijma_shuhud(jidhr, jidhr, 0, &mut shuhud, &mut adad);
    if shuhud.is_empty() {
        return None;
    }

    // Largest first, then by path so that two files of identical size cannot
    // swap places between scans.
    shuhud.sort_by(|awwal, thani| {
        thani
            .hajm
            .cmp(&awwal.hajm)
            .then_with(|| awwal.nisbi.cmp(&thani.nisbi))
    });
    shuhud.truncate(ADAD_ASHHAD);

    let mut hashib = blake3::Hasher::new();
    // A domain separator, so a fingerprint from this module can never collide
    // with one of the other BLAKE3 values this product stores — a backup's file
    // hash, a patch's identity — if the two ever end up in one table.
    hashib.update(b"taarib:tawheed:v1\n");
    for shahid in &shuhud {
        hashib.update(&shahid.hajm.to_le_bytes());
        hashib.update(b"\0");
        hashib.update(shahid.nisbi.as_bytes());
        hashib.update(b"\n");
    }
    Some(BasmatMuhtawa(Basma::min_bayt(
        *hashib.finalize().as_bytes(),
    )))
}

/// Collects file witnesses from one directory level.
///
/// Recursion is bounded by [`UMQ_BAHTH`] and the total visit count by
/// [`AQSA_MADAKHIL`], both checked on entry rather than after the work, so a
/// pathological tree costs a bounded number of `read_dir` calls and not one per
/// directory it contains.
fn ijma_shuhud(
    jidhr: &Path,
    haliy: &Path,
    umq: usize,
    shuhud: &mut Vec<ShahidMalaf>,
    adad: &mut usize,
) {
    if umq > UMQ_BAHTH || *adad >= AQSA_MADAKHIL {
        return;
    }
    let Ok(qaima) = std::fs::read_dir(haliy) else {
        return;
    };

    for madkhal in qaima {
        if *adad >= AQSA_MADAKHIL {
            return;
        }
        *adad = adad.saturating_add(1);
        let Ok(madkhal) = madkhal else { continue };
        let Ok(naw) = madkhal.file_type() else {
            continue;
        };

        // Symlinks are not followed. A launcher that symlinks a shared runtime
        // into every game's directory would otherwise make every game on the
        // machine fingerprint the same, which is the worst possible failure of
        // this module.
        if naw.is_symlink() {
            continue;
        }
        if naw.is_dir() {
            ijma_shuhud(jidhr, &madkhal.path(), umq.saturating_add(1), shuhud, adad);
            continue;
        }
        if !naw.is_file() {
            continue;
        }
        let Ok(bayanat) = madkhal.metadata() else {
            continue;
        };
        let masar = madkhal.path();
        let Some(nisbi) = masar_nisbi(jidhr, &masar) else {
            continue;
        };
        shuhud.push(ShahidMalaf {
            hajm: bayanat.len(),
            nisbi,
        });
    }
}

/// A path relative to the install root, with separators normalized to `/`.
///
/// Normalizing is what lets a Windows install and the same game seen through a
/// Wine prefix on Linux produce the same fingerprint. [`None`] when the path is
/// not inside the root or is not valid UTF-8 — both of which make it unusable as
/// a stable key rather than merely awkward.
fn masar_nisbi(jidhr: &Path, masar: &Path) -> Option<String> {
    let baqi = masar.strip_prefix(jidhr).ok()?;
    let mut nateeja = String::new();
    for juz in baqi.components() {
        let std::path::Component::Normal(ism) = juz else {
            return None;
        };
        let nass = ism.to_str()?;
        if !nateeja.is_empty() {
            nateeja.push('/');
        }
        // Case-folded, because two launchers describing the same NTFS directory
        // routinely disagree on capitalisation and the filesystem does not care.
        nateeja.push_str(&nass.to_lowercase());
    }
    if nateeja.is_empty() {
        None
    } else {
        Some(nateeja)
    }
}

/// One game, and every install of it that was found.
#[derive(Debug, Clone)]
pub struct LubaMuwahhada {
    /// The install this game's record is built from.
    ///
    /// The first non-manager source wins, because a store knows its own build
    /// identity and a manager wrapping it reports whatever the store told it —
    /// second-hand. When every source is a manager, the first found wins, which
    /// is a stable answer because [`crate::matajir::kul`] runs adapters in a
    /// fixed order.
    pub asasi: LubaMuktashafa,
    /// The other installs, in the order they were found.
    pub thanawiya: Vec<LubaMuktashafa>,
    /// The fingerprint that merged them, when they were merged by one.
    pub basma: Option<BasmatMuhtawa>,
}

impl LubaMuwahhada {
    /// Every launcher identity this game is known by, primary first.
    #[must_use]
    pub fn masadir(&self) -> Vec<MasdarLuba> {
        let mut masadir = vec![self.asasi.masdar.clone()];
        masadir.extend(self.thanawiya.iter().map(|luba| luba.masdar.clone()));
        masadir
    }

    /// How many installs this game has.
    #[must_use]
    pub const fn adad_tathbeetat(&self) -> usize {
        self.thanawiya.len().saturating_add(1)
    }

    /// Whether more than one launcher reports this game.
    #[must_use]
    pub const fn mukarrara(&self) -> bool {
        !self.thanawiya.is_empty()
    }
}

/// What one deduplication pass did, for the scan's log.
#[derive(Debug, Clone, Default)]
pub struct TaqreerTawheed {
    /// How many catalogue entries went in.
    pub madkhalat: usize,
    /// How many distinct games came out.
    pub alaab: usize,
    /// How many entries merged into an existing game by identifier.
    pub bil_muarrif: usize,
    /// How many merged by content fingerprint.
    pub bil_basma: usize,
    /// How many produced no fingerprint and were therefore never merged.
    pub bila_basma: usize,
}

impl TaqreerTawheed {
    /// The line the scan writes.
    #[must_use]
    pub fn satr(&self) -> String {
        format!(
            "{} catalogue entries merged into {} games ({} by identifier, {} by content \
             fingerprint, {} unfingerprinted and left separate)",
            self.madkhalat, self.alaab, self.bil_muarrif, self.bil_basma, self.bila_basma
        )
    }
}

/// Merges catalogue entries into games.
///
/// Two passes, and the order is the design:
///
/// 1. **By resolved identifier.** A Heroic entry wrapping `epic:abc` and Epic's
///    own entry for `abc` are the same game and both say so —
///    [`MasdarLuba::asl`] unwraps the manager. This pass costs nothing and
///    catches every manager-plus-store duplicate, which is most of them.
/// 2. **By content fingerprint**, for what is left. Two stores that sold the
///    same game under different identifiers, and the same game installed twice
///    from different sources.
///
/// Identifier before fingerprint because an identifier match is *certain* and a
/// fingerprint match is merely very strong. Running the cheap certain test first
/// also means the expensive one runs over a smaller set.
///
/// Within a merged game, the primary is the first entry whose source is not a
/// manager — see [`LubaMuwahhada::asasi`] for why.
#[must_use]
pub fn wahhid(madkhalat: Vec<LubaMuktashafa>) -> (Vec<LubaMuwahhada>, TaqreerTawheed) {
    let mut taqreer = TaqreerTawheed {
        madkhalat: madkhalat.len(),
        ..TaqreerTawheed::default()
    };

    // Pass one: by the identity behind any manager wrapping it.
    let mut bil_muarrif: BTreeMap<String, Vec<LubaMuktashafa>> = BTreeMap::new();
    let mut tarteeb: Vec<String> = Vec::new();
    for madkhal in madkhalat {
        let miftah = madkhal.masdar.asl().muarrif();
        let majmua = bil_muarrif.entry(miftah.clone()).or_default();
        if majmua.is_empty() {
            tarteeb.push(miftah);
        } else {
            taqreer.bil_muarrif = taqreer.bil_muarrif.saturating_add(1);
        }
        majmua.push(madkhal);
    }

    // Pass two: fingerprint whatever survived as its own group, and merge groups
    // that agree. A group is fingerprinted from its primary's install root; two
    // installs of one game in one group already agree by construction.
    let mut bil_basma: BTreeMap<String, Vec<LubaMuwahhada>> = BTreeMap::new();
    let mut bila_basma: Vec<LubaMuwahhada> = Vec::new();

    for miftah in tarteeb {
        let Some(majmua) = bil_muarrif.remove(&miftah) else {
            continue;
        };
        let Some(luba) = ibn_muwahhada(majmua) else {
            continue;
        };

        if let Some(basma) = basmat_tathbeet(&luba.asasi.jidhr) {
            let nass = basma.nass();
            let dalu = bil_basma.entry(nass).or_default();
            if !dalu.is_empty() {
                taqreer.bil_basma = taqreer.bil_basma.saturating_add(1);
            }
            dalu.push(LubaMuwahhada {
                basma: Some(basma),
                ..luba
            });
        } else {
            taqreer.bila_basma = taqreer.bila_basma.saturating_add(1);
            bila_basma.push(luba);
        }
    }

    let mut alaab: Vec<LubaMuwahhada> = Vec::new();
    for (_, majmua) in bil_basma {
        if let Some(mudmaja) = idmaj(majmua) {
            alaab.push(mudmaja);
        }
    }
    alaab.extend(bila_basma);
    taqreer.alaab = alaab.len();
    (alaab, taqreer)
}

/// Builds one game from a group of entries that share an identity.
///
/// The primary is the first non-manager entry, falling back to the first entry
/// when every source is a manager. Returns [`None`] only for an empty group,
/// which the caller's construction makes impossible — but returning an option
/// rather than indexing is what keeps `indexing_slicing` satisfied honestly
/// instead of with an exception.
fn ibn_muwahhada(majmua: Vec<LubaMuktashafa>) -> Option<LubaMuwahhada> {
    let mawdi = majmua
        .iter()
        .position(|luba| !luba.masdar.mudir())
        .unwrap_or(0);
    let mut baqi = majmua;
    if mawdi >= baqi.len() {
        return None;
    }
    let asasi = baqi.remove(mawdi);
    Some(LubaMuwahhada {
        asasi,
        thanawiya: baqi,
        basma: None,
    })
}

/// Folds several fingerprint-equal games into one.
fn idmaj(majmua: Vec<LubaMuwahhada>) -> Option<LubaMuwahhada> {
    let mut baqi = majmua;
    // Same rule as within a group: a store outranks a manager, because the store
    // is the one that knows the build the patch has to match.
    let mawdi = baqi
        .iter()
        .position(|luba| !luba.asasi.masdar.mudir())
        .unwrap_or(0);
    if mawdi >= baqi.len() {
        return None;
    }
    let mut awwal = baqi.remove(mawdi);
    for akhar in baqi {
        awwal.thanawiya.push(akhar.asasi);
        awwal.thanawiya.extend(akhar.thanawiya);
    }
    Some(awwal)
}

/// A merge the user made by hand, or refused.
///
/// The escape hatch this module's header promises. Two games the fingerprint
/// left separate can be joined, and two it joined can be split — both recorded,
/// both reversible, and both surviving a rescan, which is the part that makes
/// them worth having. A correction the next scan undoes is not a correction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QararMustakhdim {
    /// These identifiers are one game, whatever the fingerprints say.
    Damm {
        /// Every identifier that belongs to the game, as `MasdarLuba::muarrif`
        /// strings.
        masadir: Vec<String>,
    },
    /// This identifier is its own game and must not be merged into anything.
    Fasl {
        /// The identifier to keep separate.
        masdar: String,
    },
}

/// The user's manual corrections, applied after the automatic passes.
///
/// Applied after rather than instead, so a correction survives a game moving
/// between launchers: the automatic passes run normally and the corrections
/// adjust their output. A correction that replaced the automatic result would
/// have to be re-made every time anything changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QararatMustakhdim {
    /// Every correction, in the order they were made.
    pub qararat: Vec<QararMustakhdim>,
}

impl QararatMustakhdim {
    /// No corrections.
    #[must_use]
    pub const fn khaliya() -> Self {
        Self {
            qararat: Vec::new(),
        }
    }

    /// Whether an identifier is pinned apart.
    #[must_use]
    pub fn mafsul(&self, masdar: &str) -> bool {
        self.qararat
            .iter()
            .any(|qarar| matches!(qarar, QararMustakhdim::Fasl { masdar: q } if q == masdar))
    }

    /// The identifiers a given one has been manually joined to.
    #[must_use]
    pub fn mad_mumah(&self, masdar: &str) -> Vec<&str> {
        self.qararat
            .iter()
            .filter_map(|qarar| match qarar {
                QararMustakhdim::Damm { masadir } if masadir.iter().any(|q| q == masdar) => {
                    Some(masadir)
                },
                _ => None,
            })
            .flatten()
            .filter(|q| q.as_str() != masdar)
            .map(String::as_str)
            .collect()
    }
}

/// Applies the user's corrections to an automatic result.
///
/// Splits run before joins. A user who split two games apart and then joined one
/// of them to a third means all three of those things; running joins first would
/// let a join re-merge what a split had just separated, and the outcome would
/// depend on the order the corrections happened to be stored in.
#[must_use]
pub fn atbiq_qararat(alaab: Vec<LubaMuwahhada>, qararat: &QararatMustakhdim) -> Vec<LubaMuwahhada> {
    let mut natija: Vec<LubaMuwahhada> = Vec::with_capacity(alaab.len());

    for luba in alaab {
        let (mafsula, mubqat): (Vec<LubaMuktashafa>, Vec<LubaMuktashafa>) = luba
            .thanawiya
            .into_iter()
            .partition(|thanawi| qararat.mafsul(&thanawi.masdar.muarrif()));

        natija.push(LubaMuwahhada {
            thanawiya: mubqat,
            ..luba
        });
        for munfasila in mafsula {
            natija.push(LubaMuwahhada {
                asasi: munfasila,
                thanawiya: Vec::new(),
                basma: None,
            });
        }
    }

    let mut mudmaja: Vec<LubaMuwahhada> = Vec::with_capacity(natija.len());
    'kharij: for luba in natija {
        let muarrif = luba.asasi.masdar.muarrif();
        if qararat.mafsul(&muarrif) {
            mudmaja.push(luba);
            continue;
        }
        let shuraka = qararat.mad_mumah(&muarrif);
        if !shuraka.is_empty() {
            for sabiqa in &mut mudmaja {
                let mawjud = std::iter::once(sabiqa.asasi.masdar.muarrif())
                    .chain(sabiqa.thanawiya.iter().map(|q| q.masdar.muarrif()))
                    .any(|q| shuraka.contains(&q.as_str()));
                if mawjud {
                    sabiqa.thanawiya.push(luba.asasi);
                    sabiqa.thanawiya.extend(luba.thanawiya);
                    continue 'kharij;
                }
            }
        }
        mudmaja.push(luba);
    }
    mudmaja
}

/// The install roots of every game in a merged set.
///
/// The library watches these, and a game with three installs is watched in three
/// places — a game updated through Lutris while Steam's copy sits untouched is
/// still a change the grid must show.
#[must_use]
pub fn judhur(alaab: &[LubaMuwahhada]) -> Vec<PathBuf> {
    let mut judhur: Vec<PathBuf> = Vec::new();
    for luba in alaab {
        judhur.push(luba.asasi.jidhr.clone());
        judhur.extend(luba.thanawiya.iter().map(|q| q.jidhr.clone()));
    }
    judhur.sort();
    judhur.dedup();
    judhur
}
