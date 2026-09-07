//! الذاكرة — the translation memory: every confirmed pair, kept once, offered
//! everywhere, and never allowed to lie about where it came from.
//!
//! One `SQLite` file per machine, `dhakira.db`, inside the directory
//! [`Masarat::dhakira`] resolves — the same data root the storage crate lives
//! in. It is deliberately **not** a table inside `taarib.db`, and the reasons
//! are worth writing down because the alternative looks tidier:
//!
//! * **`taarib.db` is an index, not an archive.** `taarib-makhzan` runs at
//!   `synchronous = NORMAL` on the stated ground that nothing in that file is
//!   the only copy of anything — a lost transaction there costs a rescan. A
//!   memory segment is the opposite: it is a human's finished sentence, and it
//!   exists nowhere else once the project that produced it is archived. It
//!   belongs in a file whose durability is set for that — this connection
//!   runs `synchronous = FULL` — and whose write rate, one commit per
//!   confirmed string at human pace, can afford it.
//! * **Fuzzy lookup needs an index a ledger cannot be.** The store's ledgers
//!   are typed accessors and none of them is a generic query surface — that is
//!   one of `taarib-makhzan`'s hard constraints, and it is a good one. But
//!   near-match retrieval needs a trigram table, a length band, and a
//!   shared-count ranking, which is a query shape of its own. Growing the
//!   ledger until it can express that would repeal the constraint; a second
//!   database keeps both promises.
//! * **A memory is a portable artifact.** Contributors carry it between
//!   machines and hand it to each other the way they hand a TMX file around.
//!   `taarib.db` is machine state — library, scans, installs — and copying it
//!   copies things that are not true on the next machine.
//!
//! What *is* taken from the storage crate is every convention it established:
//! WAL journal mode, `foreign_keys` on, the same busy timeout
//! ([`taarib_makhzan::wasl::MUHLA_INSHIGHAL`], imported rather than restated),
//! forward-only numbered migrations checksummed and applied one transaction
//! each, refusal of a file written by a newer build, every statement a
//! `&'static str` with every value bound, and a corrupt file reported rather
//! than recreated. There is no connection pool here, and that is also a
//! convention decision rather than an omission: the memory has exactly one
//! consumer — the translation pipeline, on a blocking thread — and a pool
//! would size concurrency for readers this file does not have.
//!
//! ## The provenance rule, which is the load-bearing part
//!
//! [`crate::muraja_dakhiliya`] establishes that a machine translation can
//! never become approved except through a human attestation. The memory obeys
//! the same principle at one remove, because the memory is precisely the
//! mechanism by which a translation escapes the project where somebody vetted
//! it — or nobody did:
//!
//! * Every record carries [`MasdarDhakira`]: whether the original was
//!   **human-reviewed**, which project and game it came from, and when.
//! * An **exact match applies automatically** — and lands as memory-sourced,
//!   never as human work. [`TatbiqDhakira::halat_tadwin`] answers
//!   [`HalatMuraja::TarjamaAaliya`] unconditionally, so the project records
//!   the application through the author-less path and the string still needs
//!   a human here, whatever happened to it elsewhere.
//! * A **fuzzy match is never applied at all.** It is returned as an
//!   [`IqtirahDhakira`] with its similarity shown, and a human decides. A 93%
//!   match is a different sentence; silently shipping it is how "Attack the
//!   guard" becomes "Attack the garden" in somebody's quest log.
//! * The human-reviewed bit **ratchets upward only**. Recording a genuinely
//!   human-confirmed pair upgrades it; a machine re-encountering the pair can
//!   never set it, and marking a reuse touches counters and nothing else. An
//!   entry born machine-only therefore stays machine-only through any number
//!   of projects, which is the exact failure this module exists to prevent:
//!   a machine sentence acquiring a human reputation by travelling.
//!
//! ## The third kind
//!
//! A pair can also arrive from the overlay, which reads a line off a picture
//! and machine-translates the reading. That is neither of the two things
//! above, and calling it either would be a lie in one direction or the other:
//! it is not human work, and it is not a machine translation of the game's
//! own string — its *source text* is a guess. [`NawAsl`] is the third kind,
//! stored as a rank rather than a tag so the index can order by it, and
//! [`ThiqatQira`] is what the recognizer said about the guess, including the
//! common case where the recognizer says nothing at all and no number is
//! invented for it.
//!
//! The rank ratchets exactly as the review bit does, in both directions of
//! the same inequality: an observation of a reviewed pair leaves the rank at
//! human, and no number of observations ever raises one. Every ordering in
//! this module puts the rank first, so an observation cannot outrank reviewed
//! text however confident or however often seen — which is the property that
//! makes accumulating observations, and later sharing them, safe to do at
//! all.
//!
//! ## Why scoring is not just similarity
//!
//! A candidate is ranked by similarity **plus** agreement bonuses for sharing
//! the string's classification and its game — see [`nuqat_iqtirah`] for the
//! numbers and the worked example the task is calibrated against: a 92% match
//! from the same game's dialogue outranks a 95% match from another game's
//! menu, because the three points of edit distance are recoverable by a human
//! in seconds, while a menu label's register pasted into dialogue reads wrong
//! in ways a similarity score cannot see. Provenance and reuse break ties but
//! never inflate the *displayed* similarity — the number the translator sees
//! is always the measured one.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension as _, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use taarib_makhzan::wasl::MUHLA_INSHIGHAL;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{NitaqNasq, TasnifNass};
use taarib_mustalahat::ruqaa::TareeqaTarjama;
use taarib_usus::masarat::{self, Masarat};

use crate::khata::KhataTarjama;
use crate::muraja_dakhiliya::HalatMuraja;

/// The memory's file name inside [`Masarat::dhakira`].
///
/// A name, not a path: the directory comes from `usus::masarat` exactly as
/// `taarib.db`'s does, so no code here ever computes a location from the
/// environment.
pub const MALAF_DHAKIRA: &str = "dhakira.db";

/// The lowest similarity a fuzzy candidate may have and still be offered.
///
/// Per-mille, so 700 is 70% — the floor the established CAT tools settled on
/// after years of translators closing anything lower unread. Below it a
/// "match" is two sentences that happen to share furniture, and offering it
/// costs the translator a read for nothing.
pub const ADNA_TASHABUH: u16 = 700;

/// How many suggestions one lookup returns at most.
///
/// Five. A suggestion list is read top to bottom until one is good enough,
/// and past a handful the reading costs more than retranslating.
pub const AQSA_IQTIRAHAT: u32 = 5;

/// How many candidate *keys* the trigram stage may hand to the edit-distance
/// stage.
///
/// This is the bound that keeps a lookup from degenerating into a full scan:
/// however common the query's trigrams are, at most this many distinct source
/// keys — ordered by how many trigrams they share with the query — are ever
/// measured. 128 is far past where the right answer lives (shared-trigram
/// count correlates strongly with edit distance) and still cheap: 128
/// bounded-length distance computations are microseconds.
pub const SAQF_MURASHSHAHIN: u32 = 128;

/// How many distinct trigrams of the query participate in candidate retrieval.
///
/// A five-thousand-character cutscene monologue would otherwise put thousands
/// of rows into the probe table and join them all. The first 96 in sorted
/// order are plenty to find every plausible neighbour of a string that long,
/// and the cap makes the retrieval cost a constant.
pub const AQSA_THULATHIYAT: usize = 96;

/// How many characters of each side the edit distance considers.
///
/// Two strings that agree for 512 characters and diverge after are the same
/// translation unit for every purpose this module has; an uncapped comparison
/// over a pair of full cutscene scripts is quadratic work for a distinction
/// nobody acts on.
pub const AQSA_HURUF_MUQARANA: usize = 512;

/// How many memory rows are read per surviving candidate key.
///
/// One source key can hold several targets — the same English rendered
/// differently by different games. Eight covers every real spread without
/// letting one promiscuous key flood the suggestion list.
pub const HADD_SUFUF_MIFTAH: u32 = 8;

/// How many candidate keys survive the distance stage into row retrieval.
pub const HADD_MAFATIH_QARIBA: usize = 16;

/// The per-connection configuration, applied once at open.
///
/// The same block `taarib-makhzan` applies, with one deliberate difference:
/// `synchronous = FULL`. That store documents `NORMAL` as defensible because
/// nothing in it is the only copy of anything; every row here is the only
/// copy of a finished translation, commits arrive at human pace, and an fsync
/// per confirmed string is a price nobody can feel.
const TAHYIA: &str = "\
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -4096;";

/// Prepared statements kept on the connection.
///
/// The working set is small — an upsert, a handful of lookups, the trigram
/// insert — but the trigram insert runs once per trigram per new key, and
/// re-parsing it there would be the hot path parsing identical SQL hundreds
/// of times per imported file.
const SAAT_JUMAL: usize = 32;

// ---------------------------------------------------------------------------
// التوحيد — the shared normalization
// ---------------------------------------------------------------------------
//
// These functions live *here* and are imported by `masrad`, not duplicated
// there, because the folded form is persisted: `miftah_bahth.miftah` and
// `qayd.hadaf_muwahhad` are computed by these functions and written to disk.
// That makes their behaviour part of the schema's contract — broadening the
// fold is a migration that rebuilds the key tables, not a code tweak — and
// the module that owns the schema version must own the function whose change
// forces one. The glossary imports them so that the two modules can never
// disagree about whether «القوّة» and «القوة» are the same word; the day they
// disagreed, a string would pass the glossary and miss the memory, and the
// bug would be unreproducible from either module alone.

/// Unifies Arabic text for matching.
///
/// Strips tashkeel (U+064B–U+0652 and the superscript alef U+0670) and tatweel
/// (U+0640), folds the hamza-carrying and wasla alef forms أ إ آ ٱ to bare ا,
/// ta marbuta ة to ه, and alef maqsura ى to ي.
///
/// Exactly these, and deliberately no more. Every rule answers a way Arabic
/// text genuinely varies between a translator's keyboard and a game's data —
/// vocalisation, decorative elongation, hamza seat, final-form spelling —
/// without erasing a distinction a reader would call a different word. The
/// list is closed on purpose: the output is a **persisted lookup key** (see
/// the section comment above), so adding a rule later means a migration that
/// re-keys the whole memory, and that cost is only worth paying for a rule
/// with evidence behind it.
///
/// Non-Arabic characters pass through untouched, which is what lets
/// [`miftah_muwahhad`] compose this with the Latin fold in either order.
#[must_use]
pub fn wahhid_arabi(nass: &str) -> String {
    let mut natija = String::with_capacity(nass.len());
    for harf in nass.chars() {
        match harf {
            // Tashkeel (fathatan through sukun, plus the dagger alef) is
            // vocalisation and tatweel is layout; neither is language.
            '\u{064B}'..='\u{0652}' | '\u{0670}' | '\u{0640}' => {},
            // Alef with hamza above/below, madda, or wasla.
            '\u{0623}' | '\u{0625}' | '\u{0622}' | '\u{0671}' => natija.push('\u{0627}'),
            // Ta marbuta reads as ha at a match boundary.
            '\u{0629}' => natija.push('\u{0647}'),
            // Alef maqsura and ya are interchangeable in game data.
            '\u{0649}' => natija.push('\u{064A}'),
            akhar => natija.push(akhar),
        }
    }
    natija
}

/// Folds Latin text for matching: full Unicode lowercasing, precomposed
/// Latin diacritics reduced to their base letter, and combining marks
/// (U+0300–U+036F) dropped.
///
/// The reason this is not `to_lowercase` alone is a real string: a game
/// ships `Café` in a menu and a memory holds `cafe` from another game, and
/// byte comparison calls them strangers. `SQLite`'s `NOCASE` is worse still —
/// it folds ASCII only, which is the same reason the storage crate folds its
/// library sort keys in Rust rather than in a collation.
///
/// Arabic passes through unchanged: it has no case, and the combining range
/// stripped here is the Latin one, not the Arabic marks — those belong to
/// [`wahhid_arabi`], which strips them under its own documented rules.
#[must_use]
pub fn wahhid_latini(nass: &str) -> String {
    let mut natija = String::with_capacity(nass.len());
    for harf in nass.chars() {
        // A combining mark contributes nothing to a lookup key whether it
        // arrived precomposed or not; dropping it makes the two encodings of
        // `é` produce one key without pulling in a normalization crate.
        if ('\u{0300}'..='\u{036F}').contains(&harf) {
            continue;
        }
        match asas_latini(harf) {
            Some(asas) => natija.push(asas),
            None => {
                for saghir in harf.to_lowercase() {
                    natija.push(saghir);
                }
            },
        }
    }
    natija
}

/// The base letter of a precomposed Latin letter-with-diacritic, lowercased.
///
/// Covers the Latin-1 Supplement and the Latin Extended-A letters that
/// actually occur in game text — the French, German, Spanish, Portuguese,
/// Polish, Czech and Turkish alphabets. [`None`] means the character is not
/// a decorated Latin letter and the ordinary lowercase fold applies.
const fn asas_latini(harf: char) -> Option<char> {
    Some(match harf {
        'À'..='Å' | 'à'..='å' | 'Ā' | 'ā' | 'Ă' | 'ă' | 'Ą' | 'ą' => 'a',
        'Ç' | 'ç' | 'Ć' | 'ć' | 'Č' | 'č' => 'c',
        'Ď' | 'ď' | 'Đ' | 'đ' => 'd',
        'È'..='Ë' | 'è'..='ë' | 'Ē' | 'ē' | 'Ė' | 'ė' | 'Ę' | 'ę' | 'Ě' | 'ě' => 'e',
        'Ğ' | 'ğ' | 'Ģ' | 'ģ' => 'g',
        'Ì'..='Ï' | 'ì'..='ï' | 'Ī' | 'ī' | 'İ' | 'ı' | 'Į' | 'į' => 'i',
        'Ķ' | 'ķ' => 'k',
        'Ĺ' | 'ĺ' | 'Ļ' | 'ļ' | 'Ľ' | 'ľ' | 'Ł' | 'ł' => 'l',
        'Ñ' | 'ñ' | 'Ń' | 'ń' | 'Ņ' | 'ņ' | 'Ň' | 'ň' => 'n',
        'Ò'..='Ö' | 'Ø' | 'ò'..='ö' | 'ø' | 'Ō' | 'ō' | 'Ő' | 'ő' => 'o',
        'Ŕ' | 'ŕ' | 'Ř' | 'ř' => 'r',
        'Ś' | 'ś' | 'Ş' | 'ş' | 'Š' | 'š' => 's',
        'Ţ' | 'ţ' | 'Ť' | 'ť' => 't',
        'Ù'..='Ü' | 'ù'..='ü' | 'Ū' | 'ū' | 'Ů' | 'ů' | 'Ű' | 'ű' | 'Ų' | 'ų' => 'u',
        'Ý' | 'ý' | 'ÿ' | 'Ÿ' => 'y',
        'Ź' | 'ź' | 'Ż' | 'ż' | 'Ž' | 'ž' => 'z',
        // ß is deliberately absent: mapping it to a single `s` would make
        // `Straße` fold to `strase`, which matches neither spelling. It
        // stays itself and matches itself.
        _ => return None,
    })
}

/// The persisted lookup key of a piece of text: Arabic unification, then the
/// Latin fold, then whitespace collapsed to single spaces and trimmed.
///
/// One function for both sides. The source of a game string is usually Latin
/// and the target Arabic, but bilingual games mix scripts inside one string,
/// and a key function that branched on "which side is this" would give the
/// mixed string two keys depending on who asked.
///
/// **Changing this function is a schema change.** The output is stored in
/// `miftah_bahth.miftah` and `qayd.hadaf_muwahhad`; a build whose fold
/// disagrees with the one that wrote the file will silently miss exact
/// matches it should have found. Any edit here ships with a migration that
/// recomputes both columns and the trigram table.
#[must_use]
pub fn miftah_muwahhad(nass: &str) -> String {
    let mafrud = wahhid_latini(&wahhid_arabi(nass));
    let mut natija = String::with_capacity(mafrud.len());
    let mut faragh = false;
    for harf in mafrud.trim().chars() {
        if harf.is_whitespace() {
            faragh = true;
            continue;
        }
        if faragh && !natija.is_empty() {
            natija.push(' ');
        }
        faragh = false;
        natija.push(harf);
    }
    natija
}

// ---------------------------------------------------------------------------
// التشابه — similarity, in integers
// ---------------------------------------------------------------------------

/// A similarity, in per-mille (0 ..= 1000).
///
/// An integer on purpose. Similarity scores get compared, sorted, and
/// thresholded on every lookup, and every one of those operations on a float
/// is either an exact-equality trap (`float_cmp` is denied in this workspace
/// for the classic reason) or a `total_cmp` incantation someone eventually
/// forgets. Per-mille keeps a decimal place more than the interface shows,
/// costs nothing, and makes every comparison exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct Tashabuh(u16);

impl Tashabuh {
    /// A perfect match.
    pub const TAMM: Self = Self(1000);

    /// Wraps a per-mille value, clamped to the meaningful range.
    #[must_use]
    pub const fn min_alf(qeema: u16) -> Self {
        if qeema > 1000 {
            Self(1000)
        } else {
            Self(qeema)
        }
    }

    /// The value, per-mille.
    #[must_use]
    pub const fn alf(self) -> u16 {
        self.0
    }

    /// The value as a whole percentage, rounded half-up, for display.
    #[must_use]
    #[expect(
        clippy::integer_division,
        reason = "denominator is the constant 10; flooring after +5 is round-half-up, \
                  which is the display rounding wanted here"
    )]
    pub const fn mia(self) -> u16 {
        // Saturating because a value above 1000 is constructible through
        // deserialization; the clamp in `min_alf` guards construction, not
        // decoding, and display arithmetic must not be the thing that trips.
        self.0.saturating_add(5) / 10
    }

    /// Whether this similarity clears the floor a suggestion must clear.
    #[must_use]
    pub const fn yustahaqq_iqtirah(self) -> bool {
        self.0 >= ADNA_TASHABUH
    }
}

impl fmt::Display for Tashabuh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.mia())
    }
}

/// Levenshtein distance over two character slices, two-row dynamic program.
///
/// Written without a single index expression — the workspace denies
/// `indexing_slicing`, and the usual `d[i][j]` formulation is exactly the
/// kind of code where an off-by-one lives for a year. The row is walked with
/// `iter_mut().zip`, and the three running values (`qutri` the diagonal,
/// `sabiq` the cell to the left, `*khana` the cell above) are named for what
/// they are in the recurrence.
fn masafat_tahrir(awwal: &[char], thani: &[char]) -> usize {
    if awwal.is_empty() {
        return thani.len();
    }
    if thani.is_empty() {
        return awwal.len();
    }

    // Row 0: distance from the empty prefix is the prefix length.
    let mut saff: Vec<usize> = (0..=thani.len()).collect();

    for (fahras, harf_awwal) in awwal.iter().enumerate() {
        // The previous row's column 0 held `fahras`; it becomes the diagonal.
        let mut qutri = fahras;
        let mut sabiq = fahras.saturating_add(1);
        if let Some(amud_awwal) = saff.first_mut() {
            *amud_awwal = sabiq;
        }
        for (khana, harf_thani) in saff.iter_mut().skip(1).zip(thani.iter()) {
            let tabdil = if harf_awwal == harf_thani {
                qutri
            } else {
                qutri.saturating_add(1)
            };
            let jadid = tabdil
                .min(khana.saturating_add(1))
                .min(sabiq.saturating_add(1));
            qutri = *khana;
            *khana = jadid;
            sabiq = jadid;
        }
    }
    saff.last().copied().unwrap_or(0)
}

/// Normalized similarity of two **already-folded** keys, per-mille.
///
/// `1000 − ⌊1000·distance ÷ max(len)⌋`, over at most
/// [`AQSA_HURUF_MUQARANA`] characters of each side. Flooring the
/// dissimilarity means the similarity is never overstated, which is the
/// direction to be wrong in for a number a human uses to decide how much to
/// trust a sentence.
#[must_use]
pub fn tashabuh_miftahayn(awwal: &str, thani: &str) -> Tashabuh {
    let huruf_awwal: Vec<char> = awwal.chars().take(AQSA_HURUF_MUQARANA).collect();
    let huruf_thani: Vec<char> = thani.chars().take(AQSA_HURUF_MUQARANA).collect();

    let akbar = huruf_awwal.len().max(huruf_thani.len());
    if akbar == 0 {
        // Two empty strings are the same string.
        return Tashabuh::TAMM;
    }

    let masafa = masafat_tahrir(&huruf_awwal, &huruf_thani);
    #[expect(
        clippy::integer_division,
        reason = "akbar was checked non-zero above; flooring the scaled distance floors \
                  the dissimilarity, so similarity is understated, never overstated"
    )]
    let bud = masafa.saturating_mul(1000) / akbar;
    let alf = u16::try_from(1000_usize.saturating_sub(bud)).unwrap_or(0);
    Tashabuh::min_alf(alf)
}

// ---------------------------------------------------------------------------
// الثلاثيات — the trigram probe
// ---------------------------------------------------------------------------

/// The padding sentinel around each word before trigram extraction.
///
/// U+0002 cannot occur in a key: [`miftah_muwahhad`] emits only characters
/// that were in the text plus U+0020, and control characters never survive
/// extraction. Padding with two sentinels in front and one behind — the
/// `pg_trgm` scheme — makes a word's first character participate in two
/// trigrams and its boundary in one, which is what lets `save` and `saved`
/// share most of their set while `save` and `wave` share less of it.
const HASHW: char = '\u{0002}';

/// The distinct trigrams of a folded key, word by word, capped at
/// [`AQSA_THULATHIYAT`].
///
/// Word-level rather than whole-string, so that `Load Game` and `Game Load`
/// probe as neighbours — menu strings reorder across games constantly and a
/// whole-string window would make word order matter far more at the probe
/// stage than it matters at the distance stage that actually decides.
///
/// The set is sorted (it comes out of a `BTreeSet`) before the cap is
/// applied, so which trigrams a very long string contributes is a property
/// of the string, not of iteration order.
#[must_use]
pub fn thulathiyat_miftah(miftah: &str) -> Vec<String> {
    let mut majmua: BTreeSet<String> = BTreeSet::new();
    for kalima in miftah.split_whitespace() {
        let mut huruf: Vec<char> = Vec::with_capacity(kalima.chars().count().saturating_add(3));
        huruf.push(HASHW);
        huruf.push(HASHW);
        huruf.extend(kalima.chars());
        huruf.push(HASHW);
        for nafidha in huruf.windows(3) {
            let mut juz = String::with_capacity(12);
            for harf in nafidha {
                juz.push(*harf);
            }
            let _ = majmua.insert(juz);
        }
    }
    majmua.into_iter().take(AQSA_THULATHIYAT).collect()
}

// ---------------------------------------------------------------------------
// المصدر — provenance
// ---------------------------------------------------------------------------

/// What a recognizer said about how sure it was — or that it said nothing.
///
/// Two variants and no third, because there are two situations and the
/// difference between them is the whole point.
/// `taarib_tabaqa::qira`'s header states it for the reading side: neither
/// Windows Runtime OCR nor the bundled portable engine reports a confidence,
/// and the number those engines' lines carry is that crate's fixed stand-in
/// (`THIQA_GHAYR_MAQISA`, eighty), not a measurement. Only macOS Vision
/// reports a real per-line number.
///
/// So the memory does not store a number for an engine that measured none. A
/// `u8` field with an eighty in it is indistinguishable, three hops later,
/// from a genuine eighty — and an eighty that came from a constant will be
/// compared against thresholds, ranked against real measurements, and shown
/// to a player as if somebody had measured something. [`ThiqatQira::Ghayr`]
/// carries no number at all, so there is nothing to mistake.
///
/// [`ThiqatQira::yajtaz`] is where that refusal becomes a rule rather than a
/// note: an unmeasured reading clears **no** floor, however low.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum ThiqatQira {
    /// The recognizer reported this number itself.
    Maqisa {
        /// Zero to a hundred, as the engine reported it.
        mia: u8,
    },
    /// The recognizer reports no confidence, and none was invented for it.
    Ghayr,
}

impl ThiqatQira {
    /// A measured reading, clamped to the range the engines report in.
    #[must_use]
    pub const fn maqisa(mia: u8) -> Self {
        Self::Maqisa {
            mia: if mia > 100 { 100 } else { mia },
        }
    }

    /// The number, when there is one.
    ///
    /// [`None`] is not "zero confidence"; it is "no measurement exists".
    #[must_use]
    pub const fn mia(self) -> Option<u8> {
        match self {
            Self::Maqisa { mia } => Some(mia),
            Self::Ghayr => None,
        }
    }

    /// Whether a recognizer measured this at all.
    #[must_use]
    pub const fn qisat(self) -> bool {
        matches!(self, Self::Maqisa { .. })
    }

    /// Whether this reading clears a confidence floor.
    ///
    /// An unmeasured reading clears nothing, and that is deliberate rather
    /// than conservative: a floor is a statement about a measurement, and
    /// applying it to a reading that has none would be answering a question
    /// nobody can answer. A caller who wants unmeasured readings anyway asks
    /// for them explicitly — see `taarib_warsha`'s sharing permit, where
    /// including them is a warning the user acknowledges by name.
    #[must_use]
    pub const fn yajtaz(self, atabaa: u8) -> bool {
        match self {
            Self::Maqisa { mia } => mia >= atabaa,
            Self::Ghayr => false,
        }
    }

    /// The label a suggestion card shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Maqisa { mia } if mia >= 85 => "قراءة واضحة",
            Self::Maqisa { mia } if mia >= 60 => "قراءة محتملة الخطأ",
            Self::Maqisa { .. } => "قراءة ضعيفة",
            Self::Ghayr => "ثقة القراءة غير مقيسة",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Maqisa { mia } if mia >= 85 => "Clear reading",
            Self::Maqisa { mia } if mia >= 60 => "Reading may be wrong",
            Self::Maqisa { .. } => "Weak reading",
            Self::Ghayr => "Reading confidence unmeasured",
        }
    }
}

/// The three kinds of thing the memory can hold, ordered by how much a
/// person had to do with them.
///
/// The product's own provenance field ([`TareeqaTarjama`]) already separates
/// `BashariyaKamila` from `AaliyaFaqat`. An overlay observation is neither: a
/// machine *read* it off a picture and then a machine *translated* the
/// reading, so it carries two error sources where a machine translation of an
/// extracted string carries one. Folding it into `AaliFaqat` would lose the
/// only fact that distinguishes it, which is exactly the fact a reviewer
/// needs. Hence a third kind, at every layer: this enum, the `daraja_asl`
/// column, the ranking, and the shared artifact's wire format.
///
/// [`NawAsl::daraja`] is the stored ordering key, and it is what makes
/// "an observation never displaces reviewed text" a property of an index
/// rather than of a comparison somebody has to remember to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NawAsl {
    /// An overlay observation: recognized off a screen, then machine-translated.
    Mulahaza,
    /// A machine translation of a string somebody extracted from the game.
    Aali,
    /// A human wrote, edited or reviewed it.
    Bashari,
}

impl NawAsl {
    /// The stored rank: higher is more trustworthy, and the ranking is total.
    ///
    /// Written as `i64` because it is bound straight into the column and
    /// compared by the index; a `u8` here would be a cast at every call site.
    #[must_use]
    pub const fn daraja(self) -> i64 {
        match self {
            Self::Mulahaza => 0,
            Self::Aali => 1,
            Self::Bashari => 2,
        }
    }

    /// The kind a stored rank denotes, or [`None`] for a value no build wrote.
    #[must_use]
    pub const fn min_daraja(daraja: i64) -> Option<Self> {
        match daraja {
            0 => Some(Self::Mulahaza),
            1 => Some(Self::Aali),
            2 => Some(Self::Bashari),
            _ => None,
        }
    }

    /// Whether a human had read the text at origin.
    #[must_use]
    pub const fn bashari(self) -> bool {
        matches!(self, Self::Bashari)
    }

    /// The label a suggestion card shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Mulahaza => "قراءة من طبقة اللعب",
            Self::Aali => "ترجمة آلية دون مراجعة",
            Self::Bashari => "راجعها إنسان",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Mulahaza => "Read off the screen while playing",
            Self::Aali => "Machine only, unreviewed",
            Self::Bashari => "Human-reviewed",
        }
    }
}

/// Where a translation stood, review-wise, at the moment it entered the
/// memory.
///
/// The write-side twin of [`MasdarDhakira`]. It has three variants and no
/// rejected one, which is the same trick [`crate::muraja_dakhiliya`] plays
/// with its attestation: the state "a rejected translation, stored for
/// reuse" is not expressible, so no future bulk-import can store one by
/// forgetting to check.
#[derive(Debug, Clone, PartialEq)]
pub enum AslQayd {
    /// A human wrote, edited, or reviewed this pair themselves.
    Bashari {
        /// Who, when their identity is known.
        musahim: Option<MusahimId>,
    },
    /// A machine produced it and no human had read it when it was stored.
    ///
    /// Stored anyway — a machine-only memory is still worth consulting — but
    /// stored *as* machine-only, and [`Dhakira::sajjil`] guarantees that no
    /// amount of reuse launders it into anything else.
    AaliFaqat {
        /// The provider that produced it, for the suggestion card.
        muzawwid: Option<String>,
        /// The provider's own confidence, 0.0 to 1.0, when it reported one.
        thiqa: Option<f32>,
    },
    /// The overlay read this line off a screen and a machine translated the
    /// reading.
    ///
    /// The weakest thing the memory holds, and the only one whose *source
    /// text* may be wrong: a machine translation of an extracted string at
    /// least translated the string the game actually ships. Kept distinct
    /// from [`AslQayd::AaliFaqat`] everywhere for that reason — see
    /// [`NawAsl`].
    Mulahaza {
        /// Which recognizer read it, as `taarib_tabaqa::qira` names itself.
        qari: Option<String>,
        /// The provider that translated the reading.
        muzawwid: Option<String>,
        /// What the recognizer said about the reading — or that it said
        /// nothing.
        thiqa: ThiqatQira,
    },
}

impl AslQayd {
    /// Builds the origin from a review state, or refuses.
    ///
    /// [`None`] for [`HalatMuraja::LamTutarjam`] — there is nothing to store
    /// — and for [`HalatMuraja::Marfuda`], because a translation a human
    /// read and rejected must not be offered to the next project as if the
    /// rejection had not happened. Every drafted, flagged, approved, or
    /// machine state maps to the honest variant.
    ///
    /// There is deliberately no review state that produces
    /// [`AslQayd::Mulahaza`]: an observation does not come from a project's
    /// review record at all, it comes from a screen, and the only way to
    /// construct one is to say so.
    #[must_use]
    pub fn min_halat(
        halat: HalatMuraja,
        musahim: Option<MusahimId>,
        muzawwid: Option<String>,
        thiqa: Option<f32>,
    ) -> Option<Self> {
        match halat {
            HalatMuraja::LamTutarjam | HalatMuraja::Marfuda => None,
            HalatMuraja::TarjamaAaliya => Some(Self::AaliFaqat { muzawwid, thiqa }),
            HalatMuraja::Musawwada | HalatMuraja::LilMuraja | HalatMuraja::Muakkada => {
                Some(Self::Bashari { musahim })
            },
        }
    }

    /// Whether a human had read the pair at origin.
    #[must_use]
    pub const fn bashari(&self) -> bool {
        matches!(self, Self::Bashari { .. })
    }

    /// Which of the three kinds this origin is.
    #[must_use]
    pub const fn naw(&self) -> NawAsl {
        match self {
            Self::Bashari { .. } => NawAsl::Bashari,
            Self::AaliFaqat { .. } => NawAsl::Aali,
            Self::Mulahaza { .. } => NawAsl::Mulahaza,
        }
    }

    /// The reading confidence, which only an observation has.
    ///
    /// [`ThiqatQira::Ghayr`] for the other two kinds, and that is the honest
    /// answer rather than a placeholder: nobody read a picture to produce
    /// them, so there is no reading to have been confident about.
    #[must_use]
    pub const fn thiqat_qira(&self) -> ThiqatQira {
        match self {
            Self::Mulahaza { thiqa, .. } => *thiqa,
            Self::Bashari { .. } | Self::AaliFaqat { .. } => ThiqatQira::Ghayr,
        }
    }
}

/// One origin flattened into the columns the upsert binds.
///
/// A struct rather than a tuple returned from a `match`, because the tuple
/// grew to nine positional values the day the third kind arrived and a
/// transposed pair of `Option<String>`s in it would compile, store a provider
/// name in the recognizer column, and be invisible until somebody read a
/// suggestion card and wondered why their OCR engine had translated
/// something.
#[derive(Debug)]
struct BayanatAsl {
    muraja_bashariya: i64,
    daraja_asl: i64,
    musahim: Option<String>,
    muzawwid: Option<String>,
    thiqa: Option<f64>,
    qari: Option<String>,
    thiqa_qira: Option<i64>,
    thiqa_maqisa: i64,
    mushahadat: i64,
}

impl BayanatAsl {
    /// The one place the three kinds become columns.
    ///
    /// Total over [`AslQayd`], so a fourth kind would fail to compile here
    /// rather than silently store as a machine translation.
    fn min_asl(asl: &AslQayd) -> Self {
        let naw = asl.naw();
        let mut bayanat = Self {
            muraja_bashariya: i64::from(naw.bashari()),
            daraja_asl: naw.daraja(),
            musahim: None,
            muzawwid: None,
            thiqa: None,
            qari: None,
            thiqa_qira: None,
            thiqa_maqisa: 0,
            // Only an observation is a sighting; re-recording a reviewed pair
            // must not inflate the corroboration count of a reading.
            mushahadat: 0,
        };
        match asl {
            AslQayd::Bashari { musahim } => {
                bayanat.musahim = musahim.as_ref().map(|m| m.nass().to_owned());
            },
            AslQayd::AaliFaqat { muzawwid, thiqa } => {
                bayanat.muzawwid.clone_from(muzawwid);
                bayanat.thiqa = thiqa.map(f64::from);
            },
            AslQayd::Mulahaza {
                qari,
                muzawwid,
                thiqa,
            } => {
                bayanat.qari.clone_from(qari);
                bayanat.muzawwid.clone_from(muzawwid);
                bayanat.mushahadat = 1;
                if let Some(mia) = thiqa.mia() {
                    bayanat.thiqa_qira = Some(i64::from(mia));
                    bayanat.thiqa_maqisa = 1;
                }
            },
        }
        bayanat
    }
}

/// Where a stored pair came from, read back with every lookup.
///
/// This struct is the memory's answer to the approval rule in
/// [`crate::muraja_dakhiliya`]: the one fact that must survive any number of
/// hops between projects is whether a human ever read this sentence, and it
/// survives as data on the row rather than as a convention in the callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MasdarDhakira {
    /// The project it came from, if recorded.
    pub mashru: Option<String>,
    /// The game it came from, as a stable identifier.
    pub luba: Option<String>,
    /// The game's display name, kept denormalized so a suggestion can name
    /// its origin after the game has left the library.
    pub ism_luba: Option<String>,
    /// Which of the three kinds the pair is, at its best point so far.
    ///
    /// Ratchets upward only, exactly as [`MasdarDhakira::muraja_bashariya`]
    /// does and for the same reason: an observation arriving after a review
    /// must not demote the row, and no number of observations can promote
    /// one.
    pub naw: NawAsl,
    /// Whether a human had read the pair when it was stored — or has
    /// genuinely reviewed it in any project since. Reuse never sets this;
    /// see [`Dhakira::sajjil`].
    pub muraja_bashariya: bool,
    /// The contributor, where the origin was human and known.
    pub musahim: Option<MusahimId>,
    /// The machine provider, where one produced the pair.
    pub muzawwid: Option<String>,
    /// The recognizer that read the line, where the pair was ever observed.
    ///
    /// Kept even after the pair is reviewed, because "this sentence started
    /// life as a screen reading" stays true and stays worth knowing.
    pub qari: Option<String>,
    /// The best reading confidence recorded for the pair, or
    /// [`ThiqatQira::Ghayr`] when it was never observed or never measured.
    pub thiqa_qira: ThiqatQira,
    /// How many times the pair has been *observed* — read off a screen and
    /// translated to this same Arabic.
    ///
    /// Corroboration, not reuse: [`QaydDhakira::marrat`] counts a translator
    /// taking the pair, this counts independent sightings of it. It breaks
    /// ties between equally confident observations, on the reasoning that a
    /// misread line tends not to be misread the same way twice.
    pub mushahadat: u32,
    /// When the pair entered the memory, RFC 3339, from the database's own
    /// clock so two rows in one transaction cannot disagree.
    pub waqt: String,
}

// ---------------------------------------------------------------------------
// القيد — one stored pair
// ---------------------------------------------------------------------------

/// A stored pair's identity inside this machine's memory file.
///
/// A row id, and deliberately nothing grander: memory records are per-machine
/// and never cross a wire, so a UUID would be ceremony. It is a newtype so a
/// call site cannot hand a `NassId`-shaped number where a memory row belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct QaydId(#[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))] i64);

impl QaydId {
    /// Wraps a stored row id.
    #[must_use]
    pub const fn min_raqm(raqm: i64) -> Self {
        Self(raqm)
    }

    /// The underlying row id.
    #[must_use]
    pub const fn raqm(self) -> i64 {
        self.0
    }
}

/// One pair as the memory returns it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct QaydDhakira {
    /// Its identity in this machine's memory.
    pub id: QaydId,
    /// The source text as the origin project saw it — clean text, with atom
    /// text (a `%s`, a `{name}`) in place, exactly as [`crate::hima`]
    /// expects it.
    pub masdar: String,
    /// The Arabic, clean text likewise.
    pub hadaf: String,
    /// The kind of string it was at origin, which is what the scoring
    /// agreement bonus compares against.
    pub tasnif: TasnifNass,
    /// A one-line context from the origin — a speaker, a scene — for the
    /// suggestion card. Never used for matching.
    pub siyaq: Option<String>,
    /// The origin's markup and placeholder spans over `masdar`.
    pub nasq_masdar: Vec<NitaqNasq>,
    /// The origin's spans over `hadaf`, so an exact application can restore
    /// the translation's own formatting rather than shipping bare text.
    pub nasq_hadaf: Vec<NitaqNasq>,
    /// The provider's confidence at origin, where a machine reported one.
    pub thiqa: Option<f32>,
    /// How many times the pair has been offered and taken.
    pub marrat: u32,
    /// When it was last taken, RFC 3339.
    pub akhir_istikhdam: Option<String>,
    /// Where it came from and whether a human ever read it.
    pub asl: MasdarDhakira,
}

/// A confirmed pair on its way into the memory.
///
/// "Confirmed" means *committed into a project* — a draft a human typed, a
/// machine translation the pipeline accepted, an approved string. What it
/// cannot mean is a rejected translation or an empty one, and that is
/// enforced by [`AslQayd`] having no variant for either.
#[derive(Debug, Clone, PartialEq)]
pub struct QaydJadid {
    /// The clean source text.
    pub masdar: String,
    /// The clean Arabic.
    pub hadaf: String,
    /// What kind of string this is.
    pub tasnif: TasnifNass,
    /// The project storing it.
    pub mashru: Option<String>,
    /// The game it belongs to, as a stable identifier.
    pub luba: Option<String>,
    /// The game's display name.
    pub ism_luba: Option<String>,
    /// A one-line context for the suggestion card.
    pub siyaq: Option<String>,
    /// The source's spans.
    pub nasq_masdar: Vec<NitaqNasq>,
    /// The translation's spans.
    pub nasq_hadaf: Vec<NitaqNasq>,
    /// Who or what produced it, and whether a human read it.
    pub asl: AslQayd,
}

/// One lookup request.
///
/// A struct rather than positional arguments, for the reason
/// `taarib-istikhraj`'s classifier gives for the same shape: requests gain
/// fields over time, and a positional call site is how a caller ends up
/// passing its game id where the next caller passes a project id.
#[derive(Debug, Clone, Copy)]
pub struct TalabDhakira<'a> {
    /// The clean source text to match.
    pub masdar: &'a str,
    /// The string's classification, which candidates are scored against.
    pub tasnif: TasnifNass,
    /// The current game's identifier, for the same-game bonus.
    pub luba: Option<&'a str>,
    /// How many suggestions to return at most.
    pub hadd: u32,
}

impl<'a> TalabDhakira<'a> {
    /// A request with the default suggestion budget.
    #[must_use]
    pub const fn jadeed(masdar: &'a str, tasnif: TasnifNass) -> Self {
        Self {
            masdar,
            tasnif,
            luba: None,
            hadd: AQSA_IQTIRAHAT,
        }
    }

    /// Names the current game, enabling the same-game bonus.
    #[must_use]
    pub const fn bi_luba(mut self, luba: &'a str) -> Self {
        self.luba = Some(luba);
        self
    }
}

/// The agreement bonus for a candidate that shares the query's
/// classification.
///
/// Forty per-mille. Calibrated with [`ALAWAT_LUBA`] against the worked case
/// in the module header: 920 + 40 + 30 = 990 beats a bare 950, so a 92%
/// same-game dialogue hit outranks a 95% foreign menu hit — but 990 still
/// loses to a true 1000, so agreement can never beat an exact match. The
/// classification bonus is the larger of the two because register damage is
/// the worse failure: a menu label's clipped imperative pasted into dialogue
/// reads wrong to every player, while a good line from another game is
/// merely unfamiliar.
pub const ALAWAT_TASNIF: u16 = 40;

/// The agreement bonus for a candidate from the same game.
///
/// Thirty per-mille — see [`ALAWAT_TASNIF`] for the calibration. Same game
/// means same register, same character voices, same established vocabulary,
/// which is worth real edit-distance points but slightly less than being the
/// same *kind* of text.
pub const ALAWAT_LUBA: u16 = 30;

/// A near match, offered and never applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct IqtirahDhakira {
    /// The stored pair, provenance included.
    pub qayd: QaydDhakira,
    /// The measured similarity — always the raw measurement. Bonuses move a
    /// suggestion up the list; they never touch this number, because a
    /// translator calibrates their trust on it and an inflated one teaches
    /// them the wrong calibration.
    pub tashabuh: Tashabuh,
    /// Whether the candidate shares the query's classification.
    pub nafs_tasnif: bool,
    /// Whether it comes from the query's game.
    pub nafs_luba: bool,
    /// The rank score: similarity plus agreement bonuses. Kept on the value
    /// so the interface can re-sort a merged list without re-deriving the
    /// weights.
    pub nuqat: u16,
}

/// The two facts a candidate can agree with the query on, each worth a bonus.
///
/// Named fields rather than two positional `bool`s: the pair is
/// interchangeable at a call site, and swapping them mis-ranks every
/// suggestion by the difference between [`ALAWAT_TASNIF`] and [`ALAWAT_LUBA`]
/// without failing anything a compiler or a test would notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TawafuqIqtirah {
    /// The candidate shares the query's classification.
    pub nafs_tasnif: bool,
    /// The candidate comes from the query's game.
    pub nafs_luba: bool,
}

/// The rank score of a candidate: measured similarity plus the agreement
/// bonuses [`ALAWAT_TASNIF`] and [`ALAWAT_LUBA`].
#[must_use]
pub const fn nuqat_iqtirah(tashabuh: Tashabuh, tawafuq: TawafuqIqtirah) -> u16 {
    let mut nuqat = tashabuh.alf();
    if tawafuq.nafs_tasnif {
        nuqat = nuqat.saturating_add(ALAWAT_TASNIF);
    }
    if tawafuq.nafs_luba {
        nuqat = nuqat.saturating_add(ALAWAT_LUBA);
    }
    nuqat
}

/// An exact match, applied automatically — and marked for what it is.
///
/// The whole reason this is a distinct type rather than a suggestion with
/// similarity 1000: the two have different contracts. A suggestion is inert
/// data; an application changes a project, and the change must land with
/// memory provenance rather than human provenance. The methods on this type
/// are the contract, and the project layer records what they answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TatbiqDhakira {
    /// The stored pair being applied.
    pub qayd: QaydDhakira,
}

impl TatbiqDhakira {
    /// The review state the project must record for an automatic
    /// application: [`HalatMuraja::TarjamaAaliya`], unconditionally.
    ///
    /// Unconditionally is the point, and it mirrors the header of
    /// [`crate::muraja_dakhiliya`]: even when the origin was human-reviewed,
    /// *no human in this project has read this string*, and recording
    /// anything a human state would claim otherwise. The transition is made
    /// through the author-less path (`sajjil_aali`), the string surfaces in
    /// this project's review queue like any machine output, and the origin's
    /// human review is visible on the provenance card — where it informs the
    /// reviewer instead of replacing them.
    ///
    /// Takes no `self` so that "unconditionally" is a fact of the signature:
    /// there is no instance for a future edit to branch on.
    #[must_use]
    pub const fn halat_tadwin() -> HalatMuraja {
        HalatMuraja::TarjamaAaliya
    }

    /// The production method the project should record for the string.
    ///
    /// [`TareeqaTarjama::AaliyaThumBashariya`] when a human reviewed the pair
    /// at its origin — the text has passed human eyes, the application here
    /// was mechanical — and [`TareeqaTarjama::AaliyaFaqat`] when no human
    /// ever has. **Never** [`TareeqaTarjama::BashariyaKamila`]: that variant
    /// asserts a person produced the string in this project, and a memory
    /// application asserting it would be the exact forgery the provenance
    /// rule exists to prevent, laundered through an enum instead of a bulk
    /// action.
    ///
    /// An overlay observation lands as [`TareeqaTarjama::AaliyaFaqat`],
    /// which is the closest of the three and still an overstatement — that
    /// variant says a machine translated the game's own string, and an
    /// observation's source text was itself read off a picture. The enum has
    /// no fourth variant and this crate does not own it, so the distinction
    /// is carried where this crate *can* carry it: [`TatbiqDhakira::naw`],
    /// which the interface shows beside the method rather than instead of it.
    #[must_use]
    pub const fn tareeqa(&self) -> TareeqaTarjama {
        if self.qayd.asl.muraja_bashariya {
            TareeqaTarjama::AaliyaThumBashariya
        } else {
            TareeqaTarjama::AaliyaFaqat
        }
    }

    /// Which of the three kinds the applied pair is.
    ///
    /// The fact [`TatbiqDhakira::tareeqa`] cannot express, kept next to it so
    /// a caller reading one is looking at the other.
    #[must_use]
    pub const fn naw(&self) -> NawAsl {
        self.qayd.asl.naw
    }
}

/// What one lookup yielded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct HasadDhakira {
    /// The exact match to apply, when the memory holds one. The best-ranked
    /// row of the exact key: human-reviewed origins first, then most
    /// reused. Safe to apply precisely because [`TatbiqDhakira`] lands it
    /// unapproved — the human gate is downstream, not skipped.
    pub tatbiq: Option<TatbiqDhakira>,
    /// Near matches and any further exact-key rows, best rank first. Never
    /// applied by anything; shown with their similarity and provenance.
    pub iqtirahat: Vec<IqtirahDhakira>,
}

// ---------------------------------------------------------------------------
// TasnifNass ↔ the stored text
// ---------------------------------------------------------------------------

/// The classification as the `tasnif` column stores it.
///
/// Written out rather than routed through `serde_json`, because the column
/// has a `CHECK` constraint that lists these exact strings and a serializer
/// setting changed in another crate must not be able to invalidate a schema.
const fn ramz_tasnif(tasnif: TasnifNass) -> &'static str {
    match tasnif {
        TasnifNass::Hiwar => "hiwar",
        TasnifNass::Ikhtiyar => "ikhtiyar",
        TasnifNass::Ism => "ism",
        TasnifNass::Wasf => "wasf",
        TasnifNass::Qaima => "qaima",
        TasnifNass::Tafseer => "tafseer",
        TasnifNass::Nizam => "nizam",
        TasnifNass::Khata => "khata",
        TasnifNass::Nusub => "nusub",
        TasnifNass::Dakhili => "dakhili",
        TasnifNass::Majhul => "majhul",
    }
}

/// The stored text back as a classification.
///
/// [`None`] for anything else, which the caller reports as a damaged row
/// naming the value — a database edited by hand is a support conversation,
/// not a default.
fn tasnif_min_ramz(nass: &str) -> Option<TasnifNass> {
    Some(match nass {
        "hiwar" => TasnifNass::Hiwar,
        "ikhtiyar" => TasnifNass::Ikhtiyar,
        "ism" => TasnifNass::Ism,
        "wasf" => TasnifNass::Wasf,
        "qaima" => TasnifNass::Qaima,
        "tafseer" => TasnifNass::Tafseer,
        "nizam" => TasnifNass::Nizam,
        "khata" => TasnifNass::Khata,
        "nusub" => TasnifNass::Nusub,
        "dakhili" => TasnifNass::Dakhili,
        "majhul" => TasnifNass::Majhul,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// الهجرات — the schema, forward only
// ---------------------------------------------------------------------------

/// The bookkeeping table, created before any migration runs.
///
/// Outside the numbered migrations for the reason `taarib-makhzan` states:
/// migration 1 must record itself somewhere, and that somewhere cannot be a
/// thing migration 1 created.
const JADWAL_HIJRAT: &str = "\
CREATE TABLE IF NOT EXISTS hijrat_dhakira (
    raqm  INTEGER PRIMARY KEY,
    ism   TEXT NOT NULL,
    basma TEXT NOT NULL,
    waqt  TEXT NOT NULL
) STRICT;";

/// One forward step of the memory's schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HijraDhakira {
    /// Its number, which is also the schema version it produces.
    pub raqm: u32,
    /// A short name, so the upgrade history reads as prose in `sqlite3`.
    pub ism: &'static str,
    /// The statements, applied as one batch inside one transaction.
    pub jumal: &'static str,
}

impl HijraDhakira {
    /// The checksum of the statements this build carries for this step —
    /// FNV-1a, the storage crate's own change detector, for the storage
    /// crate's own reason: it catches a maintainer editing shipped history,
    /// which is a mistake, not an attack.
    #[must_use]
    pub fn basma(&self) -> String {
        basma_nass(self.jumal)
    }
}

/// Migration 1 — the whole memory schema.
///
/// Three tables. `miftah_bahth` holds each **distinct folded source key**
/// once, so the trigram table and the edit-distance stage pay per distinct
/// source rather than per stored pair — a menu string that four hundred
/// games share is one key, one trigram set, one distance computation.
/// `qayd` holds the pairs, several per key when games disagree about the
/// Arabic. `thulathi` is the probe: `(trigram, key)` pairs whose primary key
/// is also the index the candidate query walks.
///
/// The indices are the hundred-thousand-entry plan:
///
/// * exact lookup: `miftah_bahth.miftah` UNIQUE → key id → `qayd_bil_miftah`
///   prefix scan, already ordered human-first, most-reused-first;
/// * fuzzy probe: `thulathi` primary key `(juz, miftah)` turns "which keys
///   share this trigram" into a range scan per trigram, grouped and capped
///   before anything touches `qayd`;
/// * dedupe: `qayd_wahid` on `(lugha_hadaf, miftah, hadaf_muwahhad)` keeps
///   one row per distinct pair — the *normalized* target, so «القوّة» and
///   «القوة» are one row with one reuse count, not two rows splitting it.
const HIJRA_1: &str = "\
CREATE TABLE miftah_bahth (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    -- The folded source key, from miftah_muwahhad. Folding happens in Rust,
    -- never in a collation, for the reason taarib-makhzan gives about its
    -- own sort keys: NOCASE folds ASCII and leaves Arabic to byte order.
    miftah TEXT NOT NULL UNIQUE,
    -- Character count of the key, for the length band that keeps the fuzzy
    -- probe from measuring strings that cannot possibly clear the floor.
    tul    INTEGER NOT NULL CHECK (tul >= 1)
) STRICT;

CREATE TABLE qayd (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    miftah           INTEGER NOT NULL REFERENCES miftah_bahth (id),
    masdar           TEXT NOT NULL,
    hadaf            TEXT NOT NULL,
    -- The folded target, from the same miftah_muwahhad, for the dedupe
    -- index and for the glossary's consistency pass.
    hadaf_muwahhad   TEXT NOT NULL,
    tasnif           TEXT NOT NULL CHECK (tasnif IN (
                         'hiwar', 'ikhtiyar', 'ism', 'wasf', 'qaima', 'tafseer',
                         'nizam', 'khata', 'nusub', 'dakhili', 'majhul')),
    lugha_masdar     TEXT NOT NULL DEFAULT 'en',
    lugha_hadaf      TEXT NOT NULL DEFAULT 'ar',
    mashru           TEXT,
    luba             TEXT,
    ism_luba         TEXT,
    siyaq            TEXT,
    -- The provenance bit. 0 = no human has ever read this pair. Writes may
    -- only move it 0 -> 1, and only through sajjil; see the module header.
    muraja_bashariya INTEGER NOT NULL CHECK (muraja_bashariya IN (0, 1)),
    musahim          TEXT,
    muzawwid         TEXT,
    thiqa            REAL CHECK (thiqa IS NULL OR (thiqa >= 0.0 AND thiqa <= 1.0)),
    -- Span lists as JSON arrays, so an exact application can restore the
    -- translation's own formatting.
    nasq_masdar      TEXT NOT NULL DEFAULT '[]',
    nasq_hadaf       TEXT NOT NULL DEFAULT '[]',
    marrat           INTEGER NOT NULL DEFAULT 0 CHECK (marrat >= 0),
    waqt             TEXT NOT NULL,
    akhir_istikhdam  TEXT
) STRICT;

CREATE UNIQUE INDEX qayd_wahid
    ON qayd (lugha_hadaf, miftah, hadaf_muwahhad);

CREATE INDEX qayd_bil_miftah
    ON qayd (miftah, muraja_bashariya DESC, marrat DESC);

CREATE TABLE thulathi (
    juz    TEXT NOT NULL,
    miftah INTEGER NOT NULL REFERENCES miftah_bahth (id) ON DELETE CASCADE,
    PRIMARY KEY (juz, miftah)
) STRICT, WITHOUT ROWID;

CREATE INDEX thulathi_bil_miftah ON thulathi (miftah);";

/// Migration 1, named.
const AL_ASAS: HijraDhakira = HijraDhakira {
    raqm: 1,
    ism: "al-asas",
    jumal: HIJRA_1,
};

/// Migration 2 — the third provenance kind, and what a reading knows.
///
/// Migration 1 encoded provenance as one bit, `muraja_bashariya`, which can
/// say "a human read it" and "a machine wrote it" and has no room for the
/// third thing an overlay produces. `daraja_asl` is that room: 2 human, 1
/// machine, 0 observation, and it is a rank rather than a tag so the index
/// can order by it directly.
///
/// The reading columns are the honest ones. `thiqa_qira` is **nullable and
/// stays null for an engine that measures nothing** — a stand-in written here
/// would be indistinguishable from a measurement one query later — and
/// `thiqa_maqisa` is the bit that says which. `qari` names the engine.
/// `mushahadat` counts independent sightings, which is the tie-break between
/// two equally confident readings.
///
/// Existing rows migrate to 2 or 1 by their existing bit, so nothing already
/// stored becomes an observation. That is the only defensible direction: a
/// build that guessed the other way would relabel every machine translation
/// in every existing memory as a screen reading.
///
/// The index is dropped and rebuilt because its whole job is to hand the
/// exact-match stage its rows already in the order the supersession rule
/// wants — see [`Dhakira::ibhath`]. Leaving the old one would mean sorting
/// in Rust on every lookup and having the rule live in two places.
const HIJRA_2: &str = "\
ALTER TABLE qayd ADD COLUMN daraja_asl INTEGER NOT NULL DEFAULT 1
    CHECK (daraja_asl IN (0, 1, 2));

ALTER TABLE qayd ADD COLUMN qari TEXT;

ALTER TABLE qayd ADD COLUMN thiqa_qira INTEGER
    CHECK (thiqa_qira IS NULL OR (thiqa_qira >= 0 AND thiqa_qira <= 100));

ALTER TABLE qayd ADD COLUMN thiqa_maqisa INTEGER NOT NULL DEFAULT 0
    CHECK (thiqa_maqisa IN (0, 1));

ALTER TABLE qayd ADD COLUMN mushahadat INTEGER NOT NULL DEFAULT 0
    CHECK (mushahadat >= 0);

UPDATE qayd SET daraja_asl = 2 WHERE muraja_bashariya = 1;

DROP INDEX qayd_bil_miftah;

CREATE INDEX qayd_bil_miftah ON qayd (
    miftah,
    daraja_asl DESC,
    thiqa_maqisa DESC,
    thiqa_qira DESC,
    mushahadat DESC,
    marrat DESC);";

/// Migration 2, named.
const AL_MULAHAZA: HijraDhakira = HijraDhakira {
    raqm: 2,
    ism: "al-mulahaza",
    jumal: HIJRA_2,
};

/// Every migration this build defines, ascending.
///
/// Append only: editing a shipped entry changes its checksum and every
/// existing memory file will refuse to open, which is the intended failure and
/// far better than two schemas sharing a number.
pub const HIJRAT_DHAKIRA: &[HijraDhakira] = &[AL_ASAS, AL_MULAHAZA];

/// The highest memory-schema version this build understands.
///
/// A file declaring more is refused, never opened — the user is running an
/// older Taarib against a newer memory, and the honest answer is to say so.
pub const ISDAR_DHAKIRA: u32 = AL_MULAHAZA.raqm;

/// FNV-1a over the bytes of a migration, sixteen hex characters — the same
/// function, constants and rendering as `taarib-makhzan`'s, restated here
/// because the two crates' migration histories are independent and a shared
/// helper would invite a shared history.
fn basma_nass(nass: &str) -> String {
    const ASAS: u64 = 0xcbf2_9ce4_8422_2325;
    const MUDARIB: u64 = 0x0000_0100_0000_01b3;

    let mut basma = ASAS;
    for bayt in nass.as_bytes() {
        basma ^= u64::from(*bayt);
        basma = basma.wrapping_mul(MUDARIB);
    }
    format!("{basma:016x}")
}

// ---------------------------------------------------------------------------
// الذاكرة — the handle
// ---------------------------------------------------------------------------

/// The translation memory: one `SQLite` file, one connection, one schema.
///
/// Synchronous, exactly as the storage crate is synchronous: every call
/// blocks the thread it is made on, and callers inside Studio reach it
/// through `spawn_blocking`. There is no pool because there is no
/// concurrency to pool for — the pipeline consults the memory string by
/// string on one worker, and WAL keeps a second process (a second Studio
/// window) from blocking behind it.
pub struct Dhakira {
    ittisal: Connection,
    masar: PathBuf,
}

impl fmt::Debug for Dhakira {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The connection's own Debug would print nothing useful; the path is
        // the identity that matters in a log line.
        f.debug_struct("Dhakira")
            .field("masar", &self.masar)
            .finish_non_exhaustive()
    }
}

impl Dhakira {
    /// Opens the machine's memory at the location `usus::masarat` resolved,
    /// creating the file if it is absent and migrating it if it is behind.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the directory cannot be
    /// created, the file cannot be opened or configured, the file is not a
    /// Taarib memory, or the schema cannot be brought forward — including
    /// the refusal when the file was written by a newer build.
    pub fn iftah(masarat: &Masarat) -> Result<Self, KhataTarjama> {
        let mujallad = masarat.dhakira();
        let masar = mujallad.join(MALAF_DHAKIRA);
        masarat::insha_mujallad(&mujallad).map_err(|q| KhataTarjama::DhakiraMughlaqa {
            masar: masar.clone(),
            sabab: format!("its directory could not be created: {q}"),
        })?;
        Self::min_masar(&masar)
    }

    /// Opens a memory at an explicit path.
    ///
    /// For the owner's sandbox and for importing a colleague's memory file;
    /// every normal caller goes through [`Dhakira::iftah`] so the path keeps
    /// coming from one place.
    ///
    /// # Errors
    ///
    /// As [`Dhakira::iftah`].
    pub fn min_masar(masar: &Path) -> Result<Self, KhataTarjama> {
        let ittisal = Connection::open(masar).map_err(|q| KhataTarjama::DhakiraMughlaqa {
            masar: masar.to_path_buf(),
            sabab: format!("SQLite could not open it: {q}"),
        })?;

        ittisal
            .busy_timeout(MUHLA_INSHIGHAL)
            .and_then(|()| ittisal.execute_batch(TAHYIA))
            .map_err(|q| KhataTarjama::DhakiraMughlaqa {
                masar: masar.to_path_buf(),
                sabab: format!("it could not be configured: {q}"),
            })?;
        ittisal.set_prepared_statement_cache_capacity(SAAT_JUMAL);

        let mut dhakira = Self {
            ittisal,
            masar: masar.to_path_buf(),
        };
        dhakira.tahaqquq_namat_sijill();
        dhakira.tahaqquq_malaf_ajnabi()?;
        dhakira.rahhil()?;
        Ok(dhakira)
    }

    /// The file this memory was opened from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// Warns when the journal mode is not WAL, without refusing to run.
    ///
    /// The storage crate's own posture, for the storage crate's own case: a
    /// data directory on a network share cannot give WAL its shared memory,
    /// and a memory that refuses to open there is worse for that user than
    /// one that is occasionally slow.
    fn tahaqquq_namat_sijill(&self) {
        let namat: Result<String, rusqlite::Error> =
            self.ittisal
                .query_row("PRAGMA journal_mode", [], |saf| saf.get(0));
        match namat {
            Ok(qeema) if qeema.eq_ignore_ascii_case("wal") => {},
            Ok(qeema) => tracing::warn!(
                namat = %qeema,
                masar = %self.masar.display(),
                "translation memory journal mode is not WAL; a second process will block \
                 behind writes. This normally means the data directory is on a network \
                 filesystem."
            ),
            Err(q) => tracing::warn!(sabab = %q, "could not read the memory's journal mode"),
        }
    }

    /// Refuses a file that has tables and no migration history.
    ///
    /// That is somebody else's `SQLite` database sitting at the memory's path
    /// — or a memory damaged in exactly the wrong place — and running our
    /// migrations into it would interleave two schemas in one file. The
    /// refusal names the path; nothing is deleted, renamed, or "repaired",
    /// per the storage crate's third hard constraint, because the situation
    /// where this fires is the situation where the file's contents matter
    /// most to whoever owns them.
    fn tahaqquq_malaf_ajnabi(&self) -> Result<(), KhataTarjama> {
        self.ittisal
            .execute_batch(JADWAL_HIJRAT)
            .map_err(|q| self.khata_jumla("create", "hijrat_dhakira", &q))?;

        let khutuwat: i64 = self
            .ittisal
            .query_row("SELECT count(*) FROM hijrat_dhakira", [], |saf| saf.get(0))
            .map_err(|q| self.khata_jumla("read", "hijrat_dhakira", &q))?;
        if khutuwat > 0 {
            return Ok(());
        }

        let ghariba: i64 = self
            .ittisal
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table'
                   AND name NOT LIKE 'sqlite_%'
                   AND name <> 'hijrat_dhakira'",
                [],
                |saf| saf.get(0),
            )
            .map_err(|q| self.khata_jumla("inspect", "sqlite_master", &q))?;

        if ghariba > 0 {
            return Err(KhataTarjama::DhakiraMughlaqa {
                masar: self.masar.clone(),
                sabab: format!(
                    "the file holds {ghariba} tables and no Taarib migration history, so it \
                     is not a Taarib translation memory; refusing rather than writing into \
                     somebody else's database"
                ),
            });
        }
        Ok(())
    }

    /// Brings the file to this build's schema, one transaction per step.
    ///
    /// The refusal set is `taarib-makhzan`'s, restated: newer than this
    /// build, a recorded step this build does not define, a hole in the
    /// history, or a checksum that does not match the shipped text.
    fn rahhil(&mut self) -> Result<(), KhataTarjama> {
        let mutabbaqa = self.khutuwat_musajjala()?;
        let min_isdar = self.tahaqquq_tareekh(&mutabbaqa)?;

        for hijra in HIJRAT_DHAKIRA.iter().filter(|h| h.raqm > min_isdar) {
            self.tabbiq(hijra)?;
            tracing::info!(
                hijra = hijra.raqm,
                ism = hijra.ism,
                masar = %self.masar.display(),
                "translation memory schema migration applied"
            );
        }
        Ok(())
    }

    /// Every step the file records, as `(number, checksum)`, ascending.
    fn khutuwat_musajjala(&self) -> Result<Vec<(u32, String)>, KhataTarjama> {
        let mut jumla = self
            .ittisal
            .prepare("SELECT raqm, basma FROM hijrat_dhakira ORDER BY raqm")
            .map_err(|q| self.khata_jumla("prepare", "hijrat_dhakira", &q))?;

        let sufuf = jumla
            .query_map([], |saf| {
                let raqm: i64 = saf.get(0)?;
                let basma: String = saf.get(1)?;
                Ok((raqm, basma))
            })
            .map_err(|q| self.khata_jumla("query", "hijrat_dhakira", &q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let (raqm, basma) = saf.map_err(|q| self.khata_jumla("read", "hijrat_dhakira", &q))?;
            natija.push((u32::try_from(raqm).unwrap_or(u32::MAX), basma));
        }
        Ok(natija)
    }

    /// Checks the recorded history against this build's, and returns the
    /// version the file is at.
    fn tahaqquq_tareekh(&self, mutabbaqa: &[(u32, String)]) -> Result<u32, KhataTarjama> {
        let Some(aqsa) = mutabbaqa.iter().map(|(raqm, _)| *raqm).max() else {
            return Ok(0);
        };

        if aqsa > ISDAR_DHAKIRA {
            return Err(KhataTarjama::DhakiraMughlaqa {
                masar: self.masar.clone(),
                sabab: format!(
                    "it was written by a newer Taarib (memory schema {aqsa}, this build \
                     understands {ISDAR_DHAKIRA}); update Taarib rather than letting an old \
                     build guess at a schema it has never seen"
                ),
            });
        }

        for (raqm, basma) in mutabbaqa {
            let Some(hijra) = HIJRAT_DHAKIRA.iter().find(|h| h.raqm == *raqm) else {
                return Err(KhataTarjama::DhakiraMughlaqa {
                    masar: self.masar.clone(),
                    sabab: format!("it records migration {raqm}, which this build does not define"),
                });
            };
            let mahmula = hijra.basma();
            if mahmula != *basma {
                return Err(KhataTarjama::DhakiraMughlaqa {
                    masar: self.masar.clone(),
                    sabab: format!(
                        "migration {raqm} ({}) was applied as {basma} but this build carries \
                         {mahmula}; a shipped migration was edited, and continuing would give \
                         two users two schemas behind one version number",
                        hijra.ism
                    ),
                });
            }
        }

        // The applied set must be the contiguous prefix 1..=aqsa. A hole is
        // a shape no build produced, and running the missing step now would
        // run it against tables written by the steps that came after it.
        for hijra in HIJRAT_DHAKIRA.iter().filter(|h| h.raqm <= aqsa) {
            if !mutabbaqa.iter().any(|(raqm, _)| *raqm == hijra.raqm) {
                return Err(KhataTarjama::DhakiraMughlaqa {
                    masar: self.masar.clone(),
                    sabab: format!(
                        "migration {} is missing below {aqsa}; the schema is a shape no \
                         build produced",
                        hijra.raqm
                    ),
                });
            }
        }

        Ok(aqsa)
    }

    /// Applies one step together with the row that records it, atomically.
    fn tabbiq(&mut self, hijra: &HijraDhakira) -> Result<(), KhataTarjama> {
        let masar = self.masar.clone();
        let khata = |marhala: &str, q: &rusqlite::Error| KhataTarjama::DhakiraMughlaqa {
            masar: masar.clone(),
            sabab: format!("migration {} ({}) {marhala}: {q}", hijra.raqm, hijra.ism),
        };

        let muamala = self
            .ittisal
            .transaction()
            .map_err(|q| khata("could not begin", &q))?;
        muamala
            .execute_batch("PRAGMA defer_foreign_keys = ON;")
            .map_err(|q| khata("could not defer foreign keys", &q))?;
        muamala
            .execute_batch(hijra.jumal)
            .map_err(|q| khata("failed and was rolled back whole", &q))?;
        let _ = muamala
            .execute(
                "INSERT INTO hijrat_dhakira (raqm, ism, basma, waqt)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![hijra.raqm, hijra.ism, hijra.basma()],
            )
            .map_err(|q| khata("could not be recorded", &q))?;
        muamala.commit().map_err(|q| khata("could not commit", &q))
    }

    /// A statement failure as this crate's configuration error, with the
    /// operation and table named as data rather than assembled into prose at
    /// seventeen call sites.
    fn khata_jumla(
        &self,
        amal: &'static str,
        jadwal: &'static str,
        sabab: &rusqlite::Error,
    ) -> KhataTarjama {
        KhataTarjama::DhakiraMughlaqa {
            masar: self.masar.clone(),
            sabab: format!("{amal} on {jadwal}: {sabab}"),
        }
    }

    /// Stores one confirmed pair, or counts a re-confirmation of one already
    /// known.
    ///
    /// One `IMMEDIATE` transaction covers the key row, its trigrams, and the
    /// pair row, so an interrupted import cannot leave a key the probe finds
    /// and the pair table cannot resolve.
    ///
    /// ## The provenance ratchet
    ///
    /// The upsert writes `muraja_bashariya` and `daraja_asl` as
    /// `max(existing, incoming)` — a genuinely human confirmation upgrades
    /// the pair wherever it happens, and a machine re-encountering the pair
    /// in its fifth project cannot demote or launder anything. Combined with
    /// [`Dhakira::alim_istikhdam`] touching only counters, there is no write
    /// path in this module by which reuse manufactures a human review, which
    /// is this module's restatement of the rule in
    /// [`crate::muraja_dakhiliya`].
    ///
    /// The rank ratchet is the same guarantee one level wider, and it runs in
    /// both directions of the same inequality: an overlay observation of a
    /// pair a human has reviewed leaves `daraja_asl` at 2, and no number of
    /// observations ever raises one above 0. So a screen reading cannot
    /// acquire the reputation of reviewed text by being seen a thousand
    /// times, which is the failure that makes sharing observations dangerous
    /// at all.
    ///
    /// ## The supersession rule for a reading
    ///
    /// When the same pair is observed again, the row keeps the **better**
    /// reading rather than the latest one:
    ///
    /// * a measured reading always replaces an unmeasured one, and an
    ///   unmeasured one never replaces a measured one, whatever their
    ///   nominal numbers — `thiqa_maqisa` is a `max`, so the measured bit
    ///   only ever goes up;
    /// * between two measured readings the higher confidence wins;
    /// * `mushahadat` accumulates either way, so corroboration is recorded
    ///   even by a sighting that did not improve the confidence.
    ///
    /// Recency is deliberately **not** in that list. "Latest wins" would let
    /// one bad late read overwrite a good early one, which is precisely the
    /// direction this whole module is trying not to fail in; recency only
    /// breaks ties, and it does so at the ordering stage
    /// ([`Dhakira::ibhath`]) through the row id rather than here.
    ///
    /// ## What is silently not stored, and why silently
    ///
    /// A pair whose source or target folds to an empty key is skipped with a
    /// debug log rather than refused: no lookup could ever match it, so
    /// nothing is lost — while an error here is run-stopping
    /// ([`KhataTarjama::DhakiraMughlaqa`] halts the batch), and halting ten
    /// thousand commits over one whitespace string would be the wrong trade
    /// in the other direction.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when a statement fails or the
    /// transaction cannot commit.
    pub fn sajjil(&mut self, qayd: &QaydJadid) -> Result<(), KhataTarjama> {
        let miftah = miftah_muwahhad(&qayd.masdar);
        let hadaf_muwahhad = miftah_muwahhad(&qayd.hadaf);
        if miftah.is_empty() || hadaf_muwahhad.is_empty() {
            tracing::debug!(
                masdar = %qayd.masdar,
                "pair folds to an empty key and cannot be looked up; not stored"
            );
            return Ok(());
        }

        let masar = self.masar.clone();
        let khata_huquul = |sabab: serde_json::Error| KhataTarjama::DhakiraMughlaqa {
            masar: masar.clone(),
            sabab: format!("a span list could not be encoded as JSON: {sabab}"),
        };
        let nasq_masdar = serde_json::to_string(&qayd.nasq_masdar).map_err(&khata_huquul)?;
        let nasq_hadaf = serde_json::to_string(&qayd.nasq_hadaf).map_err(&khata_huquul)?;

        let bayanat = BayanatAsl::min_asl(&qayd.asl);

        let masar_khata = self.masar.clone();
        let khata = move |amal: &'static str, jadwal: &'static str, q: rusqlite::Error| {
            KhataTarjama::DhakiraMughlaqa {
                masar: masar_khata.clone(),
                sabab: format!("{amal} on {jadwal}: {q}"),
            }
        };

        let muamala = self
            .ittisal
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|q| khata("begin", "muamala", q))?;

        // The key row, inserted once per distinct folded source. The probe
        // table is populated only on first sight, because the trigram set is
        // a function of the key and the key is immutable.
        let mawjud: Option<i64> = muamala
            .prepare_cached("SELECT id FROM miftah_bahth WHERE miftah = ?1")
            .map_err(|q| khata("prepare select", "miftah_bahth", q))?
            .query_row(params![miftah], |saf| saf.get(0))
            .optional()
            .map_err(|q| khata("select", "miftah_bahth", q))?;

        let miftah_id = if let Some(id) = mawjud {
            id
        } else {
            let tul = i64::try_from(miftah.chars().count()).unwrap_or(i64::MAX);
            let _ = muamala
                .prepare_cached("INSERT INTO miftah_bahth (miftah, tul) VALUES (?1, ?2)")
                .map_err(|q| khata("prepare insert", "miftah_bahth", q))?
                .execute(params![miftah, tul])
                .map_err(|q| khata("insert", "miftah_bahth", q))?;
            let id = muamala.last_insert_rowid();

            let mut idkhal = muamala
                .prepare_cached("INSERT OR IGNORE INTO thulathi (juz, miftah) VALUES (?1, ?2)")
                .map_err(|q| khata("prepare insert", "thulathi", q))?;
            for juz in thulathiyat_miftah(&miftah) {
                let _ = idkhal
                    .execute(params![juz, id])
                    .map_err(|q| khata("insert", "thulathi", q))?;
            }
            id
        };

        let _ = muamala
            .prepare_cached(JUMLA_TASJIL)
            .map_err(|q| khata("prepare upsert", "qayd", q))?
            .execute(params![
                miftah_id,
                qayd.masdar,
                qayd.hadaf,
                hadaf_muwahhad,
                ramz_tasnif(qayd.tasnif),
                qayd.mashru,
                qayd.luba,
                qayd.ism_luba,
                qayd.siyaq,
                bayanat.muraja_bashariya,
                bayanat.musahim,
                bayanat.muzawwid,
                bayanat.thiqa,
                nasq_masdar,
                nasq_hadaf,
                bayanat.daraja_asl,
                bayanat.qari,
                bayanat.thiqa_qira,
                bayanat.thiqa_maqisa,
                bayanat.mushahadat,
            ])
            .map_err(|q| khata("upsert", "qayd", q))?;

        muamala.commit().map_err(|q| khata("commit", "muamala", q))
    }

    /// Looks a source string up: an exact application if the memory holds
    /// one, and ranked suggestions either way.
    ///
    /// Three bounded stages, so a lookup's cost is a function of the query
    /// and never of the memory's size:
    ///
    /// 1. **Exact** — one indexed equality on the folded key.
    /// 2. **Probe** — the query's trigrams (at most [`AQSA_THULATHIYAT`])
    ///    against the `thulathi` index, length-banded, grouped by key,
    ///    capped at [`SAQF_MURASHSHAHIN`] candidate keys. No stage ever
    ///    walks the pair table.
    /// 3. **Measure** — real edit distance over the surviving candidates
    ///    only, floor [`ADNA_TASHABUH`], then at most
    ///    [`HADD_SUFUF_MIFTAH`] rows fetched per surviving key.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when a statement fails or a stored
    /// row does not decode — a damaged memory is reported, never skipped
    /// past, because rows silently dropped from a lookup are translations
    /// the user cannot tell were lost.
    pub fn ibhath(&self, talab: &TalabDhakira<'_>) -> Result<HasadDhakira, KhataTarjama> {
        let miftah = miftah_muwahhad(talab.masdar);
        if miftah.is_empty() {
            return Ok(HasadDhakira {
                tatbiq: None,
                iqtirahat: Vec::new(),
            });
        }

        // Stage 1: the exact key. The first row is the application — the
        // statement already ordered the rows by the supersession rule, so the
        // best-supported reading of the key is the one that comes back — and
        // any further rows (other games' renderings of the same source)
        // become perfect-similarity suggestions.
        let mut tamma = self.sufuf_miftah_nassi(&miftah)?.into_iter();
        let tatbiq = tamma.next().map(|qayd| TatbiqDhakira { qayd });
        let mut iqtirahat: Vec<IqtirahDhakira> = tamma
            .map(|qayd| ila_iqtirah(qayd, Tashabuh::TAMM, talab))
            .collect();

        // Stages 2 and 3: the near neighbourhood.
        for (miftah_id, tashabuh) in self.mafatih_qariba(&miftah)? {
            for qayd in self.sufuf_miftah_raqami(miftah_id)? {
                iqtirahat.push(ila_iqtirah(qayd, tashabuh, talab));
            }
        }

        // Rank: score first, then provenance, then reuse. Provenance is a
        // tie-breaker and never part of the score, so a human-reviewed 80%
        // cannot outrank a machine 85% — trust affects which of two equals
        // is shown first, not how similar a sentence claims to be.
        iqtirahat.sort_by(|awwal, thani| {
            (
                thani.nuqat,
                thani.qayd.asl.muraja_bashariya,
                thani.qayd.marrat,
            )
                .cmp(&(
                    awwal.nuqat,
                    awwal.qayd.asl.muraja_bashariya,
                    awwal.qayd.marrat,
                ))
        });
        iqtirahat.truncate(usize::try_from(talab.hadd).unwrap_or(usize::MAX));

        Ok(HasadDhakira { tatbiq, iqtirahat })
    }

    /// The exact match alone, without building the near neighbourhood.
    ///
    /// [`Dhakira::ibhath`]'s stages 2 and 3 — the trigram probe and the edit
    /// distances — exist to produce suggestions a human reads and decides on.
    /// A caller that will never show a suggestion, because it has no human in
    /// front of it, pays for all of that and discards it. The overlay is that
    /// caller: it runs inside somebody's game, once per recognized line, and
    /// a fuzzy match is not something it is allowed to apply. So it asks this
    /// instead, which is one indexed equality.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the statement fails or a stored
    /// row does not decode.
    pub fn tatbiq_tamm(&self, masdar: &str) -> Result<Option<TatbiqDhakira>, KhataTarjama> {
        let miftah = miftah_muwahhad(masdar);
        if miftah.is_empty() {
            return Ok(None);
        }
        Ok(self
            .sufuf_miftah_nassi(&miftah)?
            .into_iter()
            .next()
            .map(|qayd| TatbiqDhakira { qayd }))
    }

    /// Records that a stored pair was actually taken — applied exactly or
    /// accepted from a suggestion.
    ///
    /// Touches `marrat` and `akhir_istikhdam` and deliberately nothing
    /// else. This method is the only thing the apply path is allowed to
    /// call, and its narrowness is the enforcement: however many projects a
    /// pair passes through, passing through cannot change what the pair
    /// *is* — see the provenance ratchet on [`Dhakira::sajjil`].
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the statement fails.
    pub fn alim_istikhdam(&self, id: QaydId) -> Result<(), KhataTarjama> {
        let _ = self
            .ittisal
            .prepare_cached(
                "UPDATE qayd
                 SET marrat = marrat + 1,
                     akhir_istikhdam = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
            )
            .map_err(|q| self.khata_jumla("prepare update", "qayd", &q))?
            .execute(params![id.raqm()])
            .map_err(|q| self.khata_jumla("update", "qayd", &q))?;
        Ok(())
    }

    /// How many pairs the memory holds.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the query cannot run.
    pub fn adad(&self) -> Result<u64, KhataTarjama> {
        let adad: i64 = self
            .ittisal
            .query_row("SELECT count(*) FROM qayd", [], |saf| saf.get(0))
            .map_err(|q| self.khata_jumla("count", "qayd", &q))?;
        Ok(u64::try_from(adad).unwrap_or(0))
    }

    /// One page of every stored pair, ascending by row id, origin intact.
    ///
    /// The read half of a memory merge: each returned record re-enters another
    /// memory through [`Dhakira::sajjil`], whose upsert is where the
    /// human-review ratchet lives — so iteration exposes the origin bit and
    /// cannot launder it.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the statement fails or a row does
    /// not decode.
    pub fn safha(&self, baad: Option<QaydId>, hadd: u32) -> Result<Vec<QaydDhakira>, KhataTarjama> {
        let mut jumla = self
            .ittisal
            .prepare_cached(JUMLA_SAFHA)
            .map_err(|q| self.khata_jumla("prepare", "qayd", &q))?;
        let min = baad.map_or(0, QaydId::raqm);
        let sufuf = jumla
            .query_map(params![min, i64::from(hadd)], KhaamQayd::min_saf)
            .map_err(|q| self.khata_jumla("query", "qayd", &q))?;
        self.ijma_sufuf(sufuf)
    }

    /// One page of the pairs of a single game and a single origin kind.
    ///
    /// The read half of a **share**, as [`Dhakira::safha`] is the read half
    /// of a merge, and narrower than it on purpose. A share is per game
    /// because a memory's usefulness to the next player is a per-game fact,
    /// and per kind because the only kind that may be shared this way is
    /// [`NawAsl::Mulahaza`] — see `taarib_warsha`'s sharing module for why a
    /// reviewed translation leaves the machine through the submission path or
    /// not at all.
    ///
    /// `luba` is matched against the stored identifier exactly, not folded: a
    /// game identity is a UUID rendering, and folding one would be folding a
    /// key that has no linguistic content to fold.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the statement fails or a row
    /// does not decode.
    pub fn safha_luba(
        &self,
        luba: &str,
        naw: NawAsl,
        baad: Option<QaydId>,
        hadd: u32,
    ) -> Result<Vec<QaydDhakira>, KhataTarjama> {
        let mut jumla = self
            .ittisal
            .prepare_cached(JUMLA_SAFHA_LUBA)
            .map_err(|q| self.khata_jumla("prepare", "qayd", &q))?;
        let min = baad.map_or(0, QaydId::raqm);
        let sufuf = jumla
            .query_map(
                params![min, i64::from(hadd), luba, naw.daraja()],
                KhaamQayd::min_saf,
            )
            .map_err(|q| self.khata_jumla("query", "qayd", &q))?;
        self.ijma_sufuf(sufuf)
    }

    /// How many pairs one game holds of one origin kind.
    ///
    /// The count a sharing screen shows before anything is written, so the
    /// user is told the size of what would leave the machine rather than
    /// discovering it from a file listing afterwards.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the query cannot run.
    pub fn adad_luba(&self, luba: &str, naw: NawAsl) -> Result<u64, KhataTarjama> {
        let adad: i64 = self
            .ittisal
            .query_row(
                "SELECT count(*) FROM qayd WHERE luba = ?1 AND daraja_asl = ?2",
                params![luba, naw.daraja()],
                |saf| saf.get(0),
            )
            .map_err(|q| self.khata_jumla("count", "qayd", &q))?;
        Ok(u64::try_from(adad).unwrap_or(0))
    }

    /// The rows of one key, looked up by the key's text — the exact stage.
    fn sufuf_miftah_nassi(&self, miftah: &str) -> Result<Vec<QaydDhakira>, KhataTarjama> {
        let mut jumla = self
            .ittisal
            .prepare_cached(JUMLA_SUFUF_NASSI)
            .map_err(|q| self.khata_jumla("prepare", "qayd", &q))?;
        let sufuf = jumla
            .query_map(
                params![miftah, i64::from(HADD_SUFUF_MIFTAH)],
                KhaamQayd::min_saf,
            )
            .map_err(|q| self.khata_jumla("query", "qayd", &q))?;
        self.ijma_sufuf(sufuf)
    }

    /// The rows of one key, looked up by the key's row id — the fuzzy stage.
    fn sufuf_miftah_raqami(&self, miftah_id: i64) -> Result<Vec<QaydDhakira>, KhataTarjama> {
        let mut jumla = self
            .ittisal
            .prepare_cached(JUMLA_SUFUF_RAQAMI)
            .map_err(|q| self.khata_jumla("prepare", "qayd", &q))?;
        let sufuf = jumla
            .query_map(
                params![miftah_id, i64::from(HADD_SUFUF_MIFTAH)],
                KhaamQayd::min_saf,
            )
            .map_err(|q| self.khata_jumla("query", "qayd", &q))?;
        self.ijma_sufuf(sufuf)
    }

    /// Drains a row iterator into decoded records, failing on the first
    /// damaged row rather than dropping it.
    fn ijma_sufuf<F>(
        &self,
        sufuf: rusqlite::MappedRows<'_, F>,
    ) -> Result<Vec<QaydDhakira>, KhataTarjama>
    where
        F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<KhaamQayd>,
    {
        let mut natija = Vec::new();
        for saf in sufuf {
            let khaam = saf.map_err(|q| self.khata_jumla("read", "qayd", &q))?;
            natija.push(khaam.ila_qayd(&self.masar)?);
        }
        Ok(natija)
    }

    /// The candidate keys near a query key, with their measured similarity.
    ///
    /// The probe: the query's trigrams go into a per-connection temp table
    /// (`temp_store = MEMORY`, so this is an in-memory set with SQL join
    /// semantics), and one constant statement joins them against `thulathi`,
    /// bands by stored key length, groups by key, and caps the group count.
    /// The temp table exists because a variable-length `IN (…)` list would
    /// mean building statement text from values, which this module — like
    /// the storage crate — never does.
    fn mafatih_qariba(&self, miftah: &str) -> Result<Vec<(i64, Tashabuh)>, KhataTarjama> {
        let ajza = thulathiyat_miftah(miftah);
        if ajza.is_empty() {
            return Ok(Vec::new());
        }

        self.ittisal
            .execute_batch(
                "CREATE TEMP TABLE IF NOT EXISTS bahth_ajza (
                     juz TEXT PRIMARY KEY
                 ) WITHOUT ROWID;
                 DELETE FROM bahth_ajza;",
            )
            .map_err(|q| self.khata_jumla("reset", "bahth_ajza", &q))?;

        {
            let mut idkhal = self
                .ittisal
                .prepare_cached("INSERT OR IGNORE INTO bahth_ajza (juz) VALUES (?1)")
                .map_err(|q| self.khata_jumla("prepare insert", "bahth_ajza", &q))?;
            for juz in &ajza {
                let _ = idkhal
                    .execute(params![juz])
                    .map_err(|q| self.khata_jumla("insert", "bahth_ajza", &q))?;
            }
        }

        let (adna_tul, aqsa_tul) = hudud_tul(miftah.chars().count());

        let mut jumla = self
            .ittisal
            .prepare_cached(
                "SELECT m.id, m.miftah, count(*) AS mushtarak
                 FROM bahth_ajza b
                 JOIN thulathi t ON t.juz = b.juz
                 JOIN miftah_bahth m ON m.id = t.miftah
                 WHERE m.tul BETWEEN ?1 AND ?2
                 GROUP BY m.id
                 ORDER BY mushtarak DESC
                 LIMIT ?3",
            )
            .map_err(|q| self.khata_jumla("prepare", "thulathi", &q))?;

        let sufuf = jumla
            .query_map(
                params![adna_tul, aqsa_tul, i64::from(SAQF_MURASHSHAHIN)],
                |saf| {
                    let id: i64 = saf.get(0)?;
                    let nass: String = saf.get(1)?;
                    let mushtarak: i64 = saf.get(2)?;
                    Ok((id, nass, mushtarak))
                },
            )
            .map_err(|q| self.khata_jumla("query", "thulathi", &q))?;

        let mut murashshahun: Vec<(i64, Tashabuh, i64)> = Vec::new();
        for saf in sufuf {
            let (id, nass, mushtarak) =
                saf.map_err(|q| self.khata_jumla("read", "thulathi", &q))?;
            if nass == miftah {
                // The exact key was already served by stage 1; measuring it
                // again would only produce a duplicate suggestion at 100%.
                continue;
            }
            let tashabuh = tashabuh_miftahayn(miftah, &nass);
            if tashabuh.yustahaqq_iqtirah() {
                murashshahun.push((id, tashabuh, mushtarak));
            }
        }

        // Best similarity first; shared-trigram count as the deterministic
        // tie-breaker so equal-distance candidates do not reorder between
        // runs on the whim of GROUP BY.
        murashshahun.sort_by(|awwal, thani| {
            (thani.1, thani.2)
                .cmp(&(awwal.1, awwal.2))
                .then(awwal.0.cmp(&thani.0))
        });
        murashshahun.truncate(HADD_MAFATIH_QARIBA);
        Ok(murashshahun
            .into_iter()
            .map(|(id, tashabuh, _)| (id, tashabuh))
            .collect())
    }
}

/// Dresses a decoded row as a suggestion, scored against the request.
fn ila_iqtirah(qayd: QaydDhakira, tashabuh: Tashabuh, talab: &TalabDhakira<'_>) -> IqtirahDhakira {
    let nafs_tasnif = qayd.tasnif == talab.tasnif;
    let nafs_luba = match (talab.luba, qayd.asl.luba.as_deref()) {
        (Some(matlub), Some(makhzun)) => matlub == makhzun,
        _ => false,
    };
    let nuqat = nuqat_iqtirah(
        tashabuh,
        TawafuqIqtirah {
            nafs_tasnif,
            nafs_luba,
        },
    );
    IqtirahDhakira {
        qayd,
        tashabuh,
        nafs_tasnif,
        nafs_luba,
        nuqat,
    }
}

/// The write statement: one pair in, or one re-confirmation folded into the
/// row that is already there.
///
/// Lifted out of [`Dhakira::sajjil`] once it grew past forty lines, so the
/// ratchets and the supersession rule read as one block of SQL instead of as
/// a wall inside a function that is also doing transaction handling.
const JUMLA_TASJIL: &str = "\
INSERT INTO qayd (
    miftah, masdar, hadaf, hadaf_muwahhad, tasnif,
    mashru, luba, ism_luba, siyaq,
    muraja_bashariya, musahim, muzawwid, thiqa,
    nasq_masdar, nasq_hadaf,
    daraja_asl, qari, thiqa_qira, thiqa_maqisa, mushahadat,
    marrat, waqt)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
        ?16, ?17, ?18, ?19, ?20,
        0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
ON CONFLICT (lugha_hadaf, miftah, hadaf_muwahhad) DO UPDATE SET
    masdar           = excluded.masdar,
    hadaf            = excluded.hadaf,
    tasnif           = excluded.tasnif,
    mashru           = coalesce(excluded.mashru, qayd.mashru),
    luba             = coalesce(excluded.luba, qayd.luba),
    ism_luba         = coalesce(excluded.ism_luba, qayd.ism_luba),
    siyaq            = coalesce(excluded.siyaq, qayd.siyaq),
    -- The ratchet: 0 -> 1 only, never back. See sajjil's doc.
    muraja_bashariya = max(qayd.muraja_bashariya, excluded.muraja_bashariya),
    -- The same ratchet one level wider: observation -> machine -> human, and
    -- never the other way. An observation of reviewed text leaves this at 2.
    daraja_asl       = max(qayd.daraja_asl, excluded.daraja_asl),
    musahim          = coalesce(excluded.musahim, qayd.musahim),
    muzawwid         = coalesce(excluded.muzawwid, qayd.muzawwid),
    thiqa            = coalesce(excluded.thiqa, qayd.thiqa),
    qari             = coalesce(excluded.qari, qayd.qari),
    -- Supersession, in two lines. `thiqa_maqisa` only rises, so a reading
    -- that measured nothing can never replace one that did; and among two
    -- measured readings the better number wins. `max` of two non-null
    -- integers is exactly that, and the only branch that needs writing is
    -- the one where the incoming reading is the first measured one.
    thiqa_maqisa     = max(qayd.thiqa_maqisa, excluded.thiqa_maqisa),
    thiqa_qira       = CASE
                         WHEN excluded.thiqa_maqisa = 0 THEN qayd.thiqa_qira
                         WHEN qayd.thiqa_maqisa = 0 THEN excluded.thiqa_qira
                         ELSE max(qayd.thiqa_qira, excluded.thiqa_qira)
                       END,
    -- Corroboration accumulates even when the sighting did not improve the
    -- confidence: seeing the same reading twice is itself evidence.
    mushahadat       = qayd.mushahadat + excluded.mushahadat,
    nasq_masdar      = excluded.nasq_masdar,
    nasq_hadaf       = excluded.nasq_hadaf";

/// Assembles a pair query around the one column list, at compile time.
///
/// A macro over `concat!` rather than hand-copied statements, because the
/// decoder ([`KhaamQayd::min_saf`]) reads columns by position: the day two
/// statements listed columns in different orders, every field of one of them
/// would decode into its neighbour and no error would fire. That drift had
/// already started — the merge page carried its own copy of the list — and
/// adding five columns to one copy and not the other is how it ends.
/// `concat!` keeps the result a `&'static str`, which keeps this module
/// inside the storage crate's rule that statement text is never assembled at
/// runtime. The literals are repeated inside the arms because `concat!`
/// takes literals and will not expand a helper macro in their place.
///
/// ## The `tafdeel` arm is the supersession rule
///
/// Read top to bottom, its `ORDER BY` is the whole policy:
///
/// 1. `daraja_asl` — reviewed text outranks machine text outranks a screen
///    reading, always. No confidence and no amount of corroboration moves an
///    observation above a reviewed translation, because the ranking is
///    lexicographic and this is the first key.
/// 2. `thiqa_maqisa` — a measured reading outranks an unmeasured one, even
///    when the unmeasured one's nominal number would be higher. An
///    unmeasured reading has no number; see [`ThiqatQira`].
/// 3. `thiqa_qira` — between measured readings, the more confident one.
/// 4. `mushahadat` — between equally confident readings, the one seen more
///    often. A misread line tends not to be misread the same way twice.
/// 5. `marrat` — between those, the one translators have actually taken more.
/// 6. `q.id` — ascending, so the order is total and *stable*.
///
/// Recency is deliberately absent, including at the last step. "Newest wins"
/// is the rule that lets one bad late reading replace a good early one, and
/// between two readings equal on every measurable ground the better property
/// is that the answer does not change under the player: what they saw
/// yesterday is what they see today unless something actually improved. The
/// improvement path is the upsert in [`JUMLA_TASJIL`], not this ordering.
///
/// The `tarteeb_saf` arm orders by row id instead, because paging a whole
/// memory needs a cursor the preference order cannot give.
macro_rules! jumla_qayd {
    (tafdeel $shart:literal) => {
        concat!(
            "SELECT q.id, q.masdar, q.hadaf, q.tasnif, q.siyaq, ",
            "q.nasq_masdar, q.nasq_hadaf, q.thiqa, q.marrat, ",
            "q.akhir_istikhdam, q.mashru, q.luba, q.ism_luba, ",
            "q.muraja_bashariya, q.musahim, q.muzawwid, q.waqt, ",
            "q.daraja_asl, q.qari, q.thiqa_qira, q.thiqa_maqisa, q.mushahadat ",
            "FROM qayd q ",
            $shart,
            " ORDER BY q.daraja_asl DESC, q.thiqa_maqisa DESC, q.thiqa_qira DESC, ",
            "q.mushahadat DESC, q.marrat DESC, q.id LIMIT ?2"
        )
    };
    (tarteeb_saf $shart:literal) => {
        concat!(
            "SELECT q.id, q.masdar, q.hadaf, q.tasnif, q.siyaq, ",
            "q.nasq_masdar, q.nasq_hadaf, q.thiqa, q.marrat, ",
            "q.akhir_istikhdam, q.mashru, q.luba, q.ism_luba, ",
            "q.muraja_bashariya, q.musahim, q.muzawwid, q.waqt, ",
            "q.daraja_asl, q.qari, q.thiqa_qira, q.thiqa_maqisa, q.mushahadat ",
            "FROM qayd q ",
            $shart,
            " ORDER BY q.id LIMIT ?2"
        )
    };
}

/// The merge page: every pair after a cursor, in stable row order.
const JUMLA_SAFHA: &str = jumla_qayd!(tarteeb_saf "WHERE q.id > ?1");

/// The export page: one game's pairs of one origin kind, in stable row order.
const JUMLA_SAFHA_LUBA: &str =
    jumla_qayd!(tarteeb_saf "WHERE q.id > ?1 AND q.luba = ?3 AND q.daraja_asl = ?4");

/// Exact stage: pairs by key text, in preference order.
const JUMLA_SUFUF_NASSI: &str =
    jumla_qayd!(tafdeel "JOIN miftah_bahth m ON m.id = q.miftah WHERE m.miftah = ?1");

/// Fuzzy stage: pairs by key id, same columns, same ordering.
const JUMLA_SUFUF_RAQAMI: &str = jumla_qayd!(tafdeel "WHERE q.miftah = ?1");

/// The length band a candidate key must sit in to possibly clear
/// [`ADNA_TASHABUH`] against a query of `tul` characters.
///
/// From the metric itself: similarity ≥ τ needs distance ≤ (1−τ)·max(n, L),
/// and distance is at least |n − L|, so L must lie in
/// [⌊n·τ⌋ , ⌈n/τ⌉] with τ as a per-mille ratio. Everything outside the band
/// is excluded by the index before it is ever measured, which is most of
/// the table for short queries — a four-character key is never measured
/// against a paragraph.
fn hudud_tul(tul: usize) -> (i64, i64) {
    let tul = u64::try_from(tul).unwrap_or(u64::MAX);
    #[expect(
        clippy::integer_division,
        reason = "constant denominator 1000; flooring widens the band downward, which can \
                  only admit an extra candidate, never exclude a valid one"
    )]
    let adna_raw = tul.saturating_mul(u64::from(ADNA_TASHABUH)) / 1000;
    #[expect(
        clippy::integer_division,
        reason = "ADNA_TASHABUH is a non-zero constant; the +denominator-1 makes this a \
                  ceiling, widening the band upward — again only ever admitting more"
    )]
    let aqsa_raw = tul
        .saturating_mul(1000)
        .saturating_add(u64::from(ADNA_TASHABUH).saturating_sub(1))
        / u64::from(ADNA_TASHABUH);
    (
        i64::try_from(adna_raw.max(1)).unwrap_or(i64::MAX),
        i64::try_from(aqsa_raw.max(1)).unwrap_or(i64::MAX),
    )
}

/// One `qayd` row before its domain types are rebuilt.
#[derive(Debug)]
struct KhaamQayd {
    id: i64,
    masdar: String,
    hadaf: String,
    tasnif: String,
    siyaq: Option<String>,
    nasq_masdar: String,
    nasq_hadaf: String,
    thiqa: Option<f64>,
    marrat: i64,
    akhir_istikhdam: Option<String>,
    mashru: Option<String>,
    luba: Option<String>,
    ism_luba: Option<String>,
    muraja_bashariya: i64,
    musahim: Option<String>,
    muzawwid: Option<String>,
    waqt: String,
    daraja_asl: i64,
    qari: Option<String>,
    thiqa_qira: Option<i64>,
    thiqa_maqisa: i64,
    mushahadat: i64,
}

impl KhaamQayd {
    /// Reads the columns in [`jumla_qayd`]'s order.
    fn min_saf(saf: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: saf.get(0)?,
            masdar: saf.get(1)?,
            hadaf: saf.get(2)?,
            tasnif: saf.get(3)?,
            siyaq: saf.get(4)?,
            nasq_masdar: saf.get(5)?,
            nasq_hadaf: saf.get(6)?,
            thiqa: saf.get(7)?,
            marrat: saf.get(8)?,
            akhir_istikhdam: saf.get(9)?,
            mashru: saf.get(10)?,
            luba: saf.get(11)?,
            ism_luba: saf.get(12)?,
            muraja_bashariya: saf.get(13)?,
            musahim: saf.get(14)?,
            muzawwid: saf.get(15)?,
            waqt: saf.get(16)?,
            daraja_asl: saf.get(17)?,
            qari: saf.get(18)?,
            thiqa_qira: saf.get(19)?,
            thiqa_maqisa: saf.get(20)?,
            mushahadat: saf.get(21)?,
        })
    }

    /// Rebuilds the domain record, refusing a damaged row by name.
    fn ila_qayd(self, masar: &Path) -> Result<QaydDhakira, KhataTarjama> {
        let raqm = self.id;
        let khata_saf = |amud: &str, qeema: &str| KhataTarjama::DhakiraMughlaqa {
            masar: masar.to_path_buf(),
            sabab: format!("row {raqm} holds {qeema:?} in {amud}, which does not decode"),
        };

        let Some(tasnif) = tasnif_min_ramz(&self.tasnif) else {
            return Err(khata_saf("tasnif", &self.tasnif));
        };

        // A rank no build ever wrote is a hand-edited or corrupted row, and
        // guessing a kind for it would be inventing provenance. Refused by
        // name, exactly as an unknown classification is.
        let Some(naw) = NawAsl::min_daraja(self.daraja_asl) else {
            return Err(khata_saf("daraja_asl", &self.daraja_asl.to_string()));
        };
        if naw.bashari() != (self.muraja_bashariya != 0) {
            return Err(khata_saf(
                "daraja_asl",
                &format!(
                    "{} beside muraja_bashariya {}",
                    self.daraja_asl, self.muraja_bashariya
                ),
            ));
        }

        // The measured bit decides whether there is a number at all, so a row
        // that claims a measurement and stores none — or stores one it does
        // not claim — is a row whose confidence cannot be read honestly.
        let thiqa_qira = match (self.thiqa_maqisa, self.thiqa_qira) {
            (0, None) => ThiqatQira::Ghayr,
            (1, Some(mia)) => ThiqatQira::maqisa(u8::try_from(mia).unwrap_or(u8::MAX)),
            (maqisa, mia) => {
                return Err(khata_saf(
                    "thiqa_qira",
                    &format!("{mia:?} beside thiqa_maqisa {maqisa}"),
                ));
            },
        };

        let musahim = match self.musahim {
            None => None,
            Some(nass) => match MusahimId::jadeed(nass.clone()) {
                Ok(id) => Some(id),
                Err(_) => return Err(khata_saf("musahim", &nass)),
            },
        };

        let nasq_masdar: Vec<NitaqNasq> = serde_json::from_str(&self.nasq_masdar)
            .map_err(|_| khata_saf("nasq_masdar", &self.nasq_masdar))?;
        let nasq_hadaf: Vec<NitaqNasq> = serde_json::from_str(&self.nasq_hadaf)
            .map_err(|_| khata_saf("nasq_hadaf", &self.nasq_hadaf))?;

        #[expect(
            clippy::cast_possible_truncation,
            reason = "thiqa is constrained to 0.0..=1.0 by the schema CHECK and is a \
                      display value; the f32 narrowing cannot change what a card shows"
        )]
        let thiqa = self.thiqa.map(|q| q as f32);

        Ok(QaydDhakira {
            id: QaydId::min_raqm(self.id),
            masdar: self.masdar,
            hadaf: self.hadaf,
            tasnif,
            siyaq: self.siyaq,
            nasq_masdar,
            nasq_hadaf,
            thiqa,
            marrat: u32::try_from(self.marrat).unwrap_or(0),
            akhir_istikhdam: self.akhir_istikhdam,
            asl: MasdarDhakira {
                mashru: self.mashru,
                luba: self.luba,
                ism_luba: self.ism_luba,
                naw,
                muraja_bashariya: self.muraja_bashariya != 0,
                musahim,
                muzawwid: self.muzawwid,
                qari: self.qari,
                thiqa_qira,
                mushahadat: u32::try_from(self.mushahadat).unwrap_or(u32::MAX),
                waqt: self.waqt,
            },
        })
    }
}
