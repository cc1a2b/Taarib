//! المسرد — the glossary: one approved Arabic form per term, checked against
//! every string, and one action to make a project agree with itself.
//!
//! A glossary term is a promise: wherever the game says `Guild Hall`, the
//! Arabic says «قاعة النقابة» — not a synonym, not a nicer phrasing somebody
//! preferred on a Tuesday. Players notice broken promises of this kind
//! instantly, because the term is the handle they use to think about the
//! game: an item renamed between its shop entry and its tooltip reads as two
//! items, and a quest that says both is a quest players cannot finish.
//!
//! ## What this module is, and pointedly is not
//!
//! It is an **in-memory model with file import**. It holds terms, finds them
//! in strings, flags violations, and *proposes* repairs. It writes nothing:
//! not to the project, not to a database, not back to the file it was loaded
//! from. Persisting the glossary is the project layer's job (the types
//! serialize for exactly that), and applying a repair is the project layer's
//! job too — [`wahhid_tadarub`] returns a list of edits instead of applying
//! them, because a glossary that wrote to the project would be a second
//! writer, and two writers over one project file is how Phase 19's merge
//! ends up reconciling a conflict no human created.
//!
//! ## Word-boundary matching, and the `SHOP` problem
//!
//! Finding a source term must be case-insensitive and must not fire inside a
//! longer word: a glossary term `HP` occurs in `Restore 50 HP` and does not
//! occur in `SHOP`. The matcher works on a **folded character sequence that
//! remembers where each character came from** (`HarfMatwi`): the text is
//! folded with the same functions the translation memory keys with, each
//! folded character carrying the original character and its byte offset.
//! A term matches at a position only when
//!
//! * the folded characters match, *and*
//! * the original character before the match — not the folded one — is
//!   absent or is not a word character, *and*
//! * the original character after it is absent or is not a word character,
//!   *and*
//! * the match does not begin or end in the middle of one original
//!   character's fold expansion. Unicode lowercasing may turn one character
//!   into several (Turkish `İ` is the canonical case), and a term must not
//!   claim half of one original letter and call the cut a word boundary.
//!
//! Word characters are alphanumerics plus `_`, so `PLAYER_HP` does not
//! match `HP` — a `snake_case` identifier is a token, not the word, and firing
//! inside it is the same class of mistake as `SHOP`. This is the discipline
//! Phase 12's classifier already established for its short tokens: `ui`
//! matches a whole path segment and never a substring, for exactly this
//! reason.
//!
//! ## The Arabic side is deliberately *not* boundary-matched
//!
//! Checking that a translation contains the approved form uses normalized
//! **substring containment**, not word boundaries, and the asymmetry is
//! Arabic grammar: clitics attach directly to the word, so the approved
//! «سيف» legitimately appears as «والسيف», «بالسيف», «سيفه». A
//! boundary-aware check would flag every one of those correct translations.
//! The costs are opposite — Latin substring matching produces false
//! *positives* (`SHOP`), Arabic boundary matching produces false
//! *negatives* (every clitic) — so each side gets the check whose failure
//! mode it does not have. Containment is tested over
//! [`crate::dhakira::miftah_muwahhad`]-folded text, so «القوّة» satisfies an
//! approved «قوة» regardless of tashkeel.
//!
//! ## One normalization, owned elsewhere
//!
//! Every fold in this module — the matcher's, the containment check's, the
//! consistency grouping's — is imported from [`crate::dhakira`]:
//! [`wahhid_arabi`], [`wahhid_latini`], [`miftah_muwahhad`]. It lives there
//! and not here because there it is a **persisted contract**: the memory
//! stores folded keys on disk, so changing the fold is a schema migration,
//! and the module that owns the schema owns the function. The glossary holds
//! no persistent state and simply borrows the definition. The day the two
//! modules disagreed about whether «القوّة» equals «القوة», a translation
//! would pass the glossary and miss the memory in the same breath, and
//! neither module's tests would see anything wrong.
//!
//! ## Enforcement produces the existing flag
//!
//! A string containing a term whose translation lacks the approved form gets
//! [`AlamJawda::MustalahMukhtalif`] — the variant Phase 1 already defined,
//! severity and wording included. No parallel flag type: two flags meaning
//! one thing is how an interface shows the same problem twice with two
//! different sentences.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NassId, TasnifNass};

use crate::dhakira::{miftah_muwahhad, wahhid_arabi, wahhid_latini};
use crate::khata::KhataTarjama;

/// The extensions [`Masrad::min_malaf`] accepts, and nothing else.
///
/// Listed rather than sniffed: a file's first bytes are a worse statement of
/// intent than its name, and a loader that guessed would "successfully" read
/// a CSV as a one-column TSV and import garbage with a clean conscience.
pub const LAWAHIQ_MASRAD: &[&str] = &["json", "tsv", "tab"];

/// Where a term applies.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NitaqMustalah {
    /// This project only.
    ///
    /// The default — including for imported files that do not say. A file of
    /// unknown provenance that silently became machine-global would push its
    /// vocabulary into every other project on the machine, and the user
    /// would meet it weeks later in a game it has nothing to do with.
    /// Promotion to global is an explicit act.
    #[default]
    MashruHali,
    /// Every project on this machine.
    ///
    /// For the vocabulary that genuinely repeats across games — interface
    /// verbs, platform terms, the community's settled renderings.
    Aam,
}

impl NitaqMustalah {
    /// The label the interface shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::MashruHali => "هذا المشروع",
            Self::Aam => "كل المشاريع",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::MashruHali => "this project",
            Self::Aam => "all projects",
        }
    }
}

/// One glossary term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MustalahMasrad {
    /// The source form, as it appears in the game's text.
    pub masdar: String,
    /// The approved Arabic form. For a do-not-translate term this is the
    /// source form itself — see [`MustalahMasrad::la_yutarjam`].
    pub arabi: String,
    /// A translator's note: why this form, what it must never be confused
    /// with, the gender it takes.
    #[serde(default)]
    pub mulahaza: Option<String>,
    /// Where the term applies.
    #[serde(default)]
    pub nitaq: NitaqMustalah,
    /// The term must appear untranslated: a brand, an acronym the community
    /// keeps in Latin, a proper noun the developer requires intact.
    ///
    /// When set, `arabi` carries the source form and enforcement demands the
    /// *source* form survive in the translation — through the same
    /// boundary-aware matcher, so `HP` required intact is not satisfied by
    /// an Arabic sentence that happens to contain `SHOP` transliterated.
    #[serde(default)]
    pub la_yutarjam: bool,
}

impl MustalahMasrad {
    /// A term with an approved Arabic form, scoped to the current project.
    #[must_use]
    pub fn jadeed(masdar: impl Into<String>, arabi: impl Into<String>) -> Self {
        Self {
            masdar: masdar.into(),
            arabi: arabi.into(),
            mulahaza: None,
            nitaq: NitaqMustalah::MashruHali,
            la_yutarjam: false,
        }
    }

    /// A do-not-translate term: the approved form is the source form.
    #[must_use]
    pub fn la_yutarjam(masdar: impl Into<String>) -> Self {
        let masdar = masdar.into();
        Self {
            arabi: masdar.clone(),
            masdar,
            mulahaza: None,
            nitaq: NitaqMustalah::MashruHali,
            la_yutarjam: true,
        }
    }

    /// The same term, widened to every project on the machine.
    #[must_use]
    pub const fn aam(mut self) -> Self {
        self.nitaq = NitaqMustalah::Aam;
        self
    }

    /// The same term, with a note attached.
    #[must_use]
    pub fn bi_mulahaza(mut self, mulahaza: impl Into<String>) -> Self {
        self.mulahaza = Some(mulahaza.into());
        self
    }

    /// The term's identity for deduplication and lookup: the folded source
    /// form, from the shared key function.
    #[must_use]
    pub fn miftah(&self) -> String {
        miftah_muwahhad(&self.masdar)
    }

    /// Whether the term is well-formed enough to enforce.
    ///
    /// A term whose source folds to nothing matches everything and nothing;
    /// a term with no approved form demands nothing. Both are import
    /// mistakes, and the loaders refuse them by name rather than letting an
    /// unenforceable term sit in the list looking enforced.
    #[must_use]
    pub fn salih(&self) -> bool {
        !self.miftah().is_empty() && !miftah_muwahhad(&self.arabi).is_empty()
    }
}

// ---------------------------------------------------------------------------
// المطابقة — the folded sequence and the boundary-aware matcher
// ---------------------------------------------------------------------------

/// One folded character that remembers its origin.
///
/// The matcher compares `harf` (the folded character) and judges boundaries
/// by `asl` (the original character) at `mawqi` (its byte offset in the
/// original string). Keeping all three is what lets the match be
/// case-insensitive and diacritic-insensitive while the boundary test reads
/// the text the translator actually wrote — the folded neighbourhood of a
/// match says nothing about whether the original had a letter there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HarfMatwi {
    /// The folded character.
    harf: char,
    /// The original character this folded character came from.
    asl: char,
    /// The original character's byte offset.
    mawqi: usize,
}

/// Whether a character glues itself to a word.
///
/// Alphanumerics in any script, plus `_`: `PLAYER_HP` is one token, and a
/// term firing inside it is the `SHOP` mistake wearing a different costume.
/// The apostrophe is deliberately *not* here — `HP's` is the term `HP` plus
/// grammar, and excluding it would unmatch half of English possessives.
fn harf_kalima(harf: char) -> bool {
    harf.is_alphanumeric() || harf == '_'
}

/// Folds a string into a matchable sequence, one entry per folded character,
/// whitespace runs collapsed to a single space.
///
/// The fold is the shared one — [`wahhid_arabi`] then [`wahhid_latini`],
/// applied per original character so each folded character can carry its
/// origin. Characters the fold deletes outright (tashkeel, tatweel)
/// contribute no entry: «القوّة» and «القوة» produce identical sequences,
/// which is precisely the equality the module header promises.
fn tasalsul_matwi(nass: &str) -> Vec<HarfMatwi> {
    let mut natija: Vec<HarfMatwi> = Vec::with_capacity(nass.len());
    // The byte offset of the whitespace run waiting to be emitted, if one is.
    // The space entry keeps the *run's own* offset, never the offset of the
    // character after it — the boundary test compares offsets to detect a
    // match starting mid-expansion, and a space that borrowed its
    // neighbour's offset would read as that neighbour's tail and veto every
    // match that follows a space.
    let mut faragh: Option<usize> = None;
    for (mawqi, asl) in nass.char_indices() {
        if asl.is_whitespace() {
            if faragh.is_none() {
                faragh = Some(mawqi);
            }
            continue;
        }
        let mufrad: String = asl.to_string();
        for harf in wahhid_latini(&wahhid_arabi(&mufrad)).chars() {
            if let Some(mawqi_faragh) = faragh.take()
                && !natija.is_empty()
            {
                natija.push(HarfMatwi {
                    harf: ' ',
                    asl: ' ',
                    mawqi: mawqi_faragh,
                });
            }
            natija.push(HarfMatwi { harf, asl, mawqi });
        }
    }
    natija
}

/// Folds a needle the same way, keeping only the folded characters.
fn ibra_matwiya(nass: &str) -> Vec<char> {
    tasalsul_matwi(nass)
        .into_iter()
        .map(|harf| harf.harf)
        .collect()
}

/// Every boundary-respecting occurrence of a folded needle in a folded
/// sequence, as byte offsets into the original string.
///
/// The two mid-expansion checks (`bidaya_saliha` / `nihaya_saliha` comparing
/// `mawqi`) are the subtle quarter of this function. Unicode lowercasing is
/// allowed to expand one character into several — `İ` becomes `i` plus a
/// combining mark — and without the offset comparison a needle could start
/// on the second folded character of one original letter and call itself
/// boundary-clean because the character *before that letter* was a space.
/// Today's fold table happens to keep every expansion single; the guard is
/// what keeps a future table edit from silently changing what "word
/// boundary" means.
fn mawaqi_ibra(matn: &[HarfMatwi], ibra: &[char]) -> Vec<usize> {
    let mut natija = Vec::new();
    if ibra.is_empty() || matn.len() < ibra.len() {
        return natija;
    }

    let akhir_bidaya = matn.len().saturating_sub(ibra.len());
    for bidaya in 0..=akhir_bidaya {
        let mutatabiq = matn
            .iter()
            .skip(bidaya)
            .take(ibra.len())
            .map(|harf| harf.harf)
            .eq(ibra.iter().copied());
        if !mutatabiq {
            continue;
        }

        let Some(awwal) = matn.get(bidaya) else {
            continue;
        };
        let nihaya = bidaya.saturating_add(ibra.len());
        let Some(akhir) = matn.get(nihaya.saturating_sub(1)) else {
            continue;
        };

        // Before the match: nothing, or a non-word character — judged on the
        // original text — and never the tail of the same original character.
        let bidaya_saliha = match bidaya.checked_sub(1).and_then(|qabl| matn.get(qabl)) {
            None => true,
            Some(qabl) => qabl.mawqi != awwal.mawqi && !harf_kalima(qabl.asl),
        };
        // After the match, symmetrically.
        let nihaya_saliha = match matn.get(nihaya) {
            None => true,
            Some(baad) => baad.mawqi != akhir.mawqi && !harf_kalima(baad.asl),
        };

        if bidaya_saliha && nihaya_saliha {
            natija.push(awwal.mawqi);
        }
    }
    natija
}

/// Whether any character is Arabic script.
///
/// Decides which containment check a form gets: the Arabic blocks that
/// actually occur in game text and user input — the base block, the
/// supplement, and the presentation forms some tools still emit.
fn fihi_arabi(nass: &str) -> bool {
    nass.chars().any(|harf| {
        matches!(harf,
            '\u{0600}'..='\u{06FF}'
                | '\u{0750}'..='\u{077F}'
                | '\u{08A0}'..='\u{08FF}'
                | '\u{FB50}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}'
        )
    })
}

/// Whether a translation contains an approved form.
///
/// Arabic forms: normalized substring containment, because clitics attach
/// directly («والسيف» satisfies «سيف»). Latin forms — the do-not-translate
/// case — go through the boundary-aware matcher, because Latin has the
/// opposite failure mode: `HP` must not be satisfied by `SHOP` surviving in
/// the output. The module header walks through why the asymmetry is the
/// design rather than an inconsistency.
fn yahtawi_shakl(hadaf: &str, shakl: &str) -> bool {
    if fihi_arabi(shakl) {
        let matn = miftah_muwahhad(hadaf);
        let matlub = miftah_muwahhad(shakl);
        !matlub.is_empty() && matn.contains(&matlub)
    } else {
        let matn = tasalsul_matwi(hadaf);
        let ibra = ibra_matwiya(shakl);
        !ibra.is_empty() && !mawaqi_ibra(&matn, &ibra).is_empty()
    }
}

// ---------------------------------------------------------------------------
// المسرد — the assembled glossary
// ---------------------------------------------------------------------------

/// One term with its matcher state precomputed.
///
/// The needle is folded once at insertion instead of once per string
/// checked, because [`Masrad::afhas`] runs over every string of a project —
/// tens of thousands of calls — and refolding a static needle inside that
/// loop is the kind of quadratic nobody notices until the first full-project
/// check takes a minute.
#[derive(Debug, Clone)]
struct MustalahMuaadd {
    /// The term as the user sees it.
    mustalah: MustalahMasrad,
    /// Its folded needle, for the boundary matcher.
    ibra: Vec<char>,
}

/// The glossary: every term in force for one project, project-scoped and
/// global entries merged, with the project's entries winning collisions.
///
/// Assembled per project by the project layer — typically
/// [`Masrad::min_nusus`] for the seed, [`Masrad::min_malaf`] plus
/// [`Masrad::admij`] for imports — and consulted read-only after that. It
/// intentionally has no save method; see the module header for why this
/// module never writes.
#[derive(Debug, Clone, Default)]
pub struct Masrad {
    /// The terms, in insertion order, matcher-ready.
    mustalahat: Vec<MustalahMuaadd>,
    /// Folded source form → index into `mustalahat`, the identity that
    /// deduplication and precedence operate on.
    faharis: BTreeMap<String, usize>,
}

impl Masrad {
    /// An empty glossary.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// A glossary from a ready list, applying the same precedence as
    /// [`Masrad::adkhil`] entry by entry.
    #[must_use]
    pub fn bi_mustalahat(mustalahat: Vec<MustalahMasrad>) -> Self {
        let mut masrad = Self::jadeed();
        masrad.admij(mustalahat);
        masrad
    }

    /// How many terms are in force.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.mustalahat.len()
    }

    /// Every term in force, in insertion order.
    pub fn mustalahat(&self) -> impl Iterator<Item = &MustalahMasrad> {
        self.mustalahat.iter().map(|muaadd| &muaadd.mustalah)
    }

    /// Adds one term, returning whether it took effect.
    ///
    /// Collisions are decided by scope, and the rule is the reason the
    /// method cannot just push: a **project term beats a global term** for
    /// the same source form — the project deliberately overrode the shared
    /// vocabulary, and an import order that happened to load the global
    /// file second must not quietly undo that decision. Same-scope
    /// collisions take the newcomer, because re-importing an updated file
    /// is how a glossary is edited. An unenforceable term (empty source or
    /// empty approved form after folding) is declined; file loaders refuse
    /// those by name before ever reaching here, so a `false` from this
    /// method on a hand-built term is the same message without a file to
    /// blame.
    pub fn adkhil(&mut self, mustalah: MustalahMasrad) -> bool {
        if !mustalah.salih() {
            tracing::debug!(masdar = %mustalah.masdar, "term is unenforceable; not added");
            return false;
        }
        let miftah = mustalah.miftah();
        let muaadd = MustalahMuaadd {
            ibra: ibra_matwiya(&mustalah.masdar),
            mustalah,
        };

        match self.faharis.get(&miftah) {
            None => {
                self.mustalahat.push(muaadd);
                let _ = self
                    .faharis
                    .insert(miftah, self.mustalahat.len().saturating_sub(1));
                true
            },
            Some(fahras) => {
                let Some(qaim) = self.mustalahat.get_mut(*fahras) else {
                    return false;
                };
                let yahkum = match (qaim.mustalah.nitaq, muaadd.mustalah.nitaq) {
                    // The project's own choice stands.
                    (NitaqMustalah::MashruHali, NitaqMustalah::Aam) => false,
                    // Global widened, or same scope refreshed.
                    _ => true,
                };
                if yahkum {
                    *qaim = muaadd;
                } else {
                    tracing::debug!(
                        masdar = %muaadd.mustalah.masdar,
                        "global term yielded to the project's own entry"
                    );
                }
                yahkum
            },
        }
    }

    /// Adds a list of terms under the same precedence.
    pub fn admij(&mut self, mustalahat: Vec<MustalahMasrad>) {
        for mustalah in mustalahat {
            let _ = self.adkhil(mustalah);
        }
    }

    /// The terms whose source form occurs — boundary-respecting — in a
    /// source string.
    ///
    /// What the prompt builder asks before a request goes out: these terms,
    /// with their approved forms and notes, are what the model is told to
    /// honour. The scan folds the string once and runs every needle over
    /// the one folded sequence.
    #[must_use]
    pub fn mustalahat_fi(&self, nass: &str) -> Vec<&MustalahMasrad> {
        let matn = tasalsul_matwi(nass);
        self.mustalahat
            .iter()
            .filter(|muaadd| !mawaqi_ibra(&matn, &muaadd.ibra).is_empty())
            .map(|muaadd| &muaadd.mustalah)
            .collect()
    }

    /// Checks one string against every term in force.
    ///
    /// A flag is raised when the source contains a term and the translation
    /// does not contain its approved form —
    /// [`AlamJawda::MustalahMukhtalif`], the variant the vocabulary crate
    /// already defines, carrying the term and the form the reviewer should
    /// have seen. An untranslated string raises nothing here: there is no
    /// translation to be wrong yet, and an empty target is
    /// [`AlamJawda::Farigh`]'s finding, computed by the flag layer that owns
    /// emptiness — one condition, one flag, one sentence in the interface.
    #[must_use]
    pub fn afhas(&self, mudkhal: &MudkhalNass) -> Vec<AlamJawda> {
        let Some(hadaf) = mudkhal.hadaf.as_deref() else {
            return Vec::new();
        };
        if miftah_muwahhad(hadaf).is_empty() {
            return Vec::new();
        }

        let matn = tasalsul_matwi(&mudkhal.masdar);
        let mut alamat = Vec::new();
        for muaadd in &self.mustalahat {
            if mawaqi_ibra(&matn, &muaadd.ibra).is_empty() {
                continue;
            }
            if !yahtawi_shakl(hadaf, &muaadd.mustalah.arabi) {
                alamat.push(AlamJawda::MustalahMukhtalif {
                    mustalah: muaadd.mustalah.masdar.clone(),
                    mutawaqqa: muaadd.mustalah.arabi.clone(),
                });
            }
        }
        alamat
    }

    /// Seeds a glossary from the proper nouns Phase 12 identified.
    ///
    /// Names — [`TasnifNass::Ism`] — are where inconsistency hurts most and
    /// where the extractor already did the hard part: it knows which strings
    /// name an item, a character, a place. Every `Ism` entry that has a
    /// translation contributes; a name rendered several ways contributes its
    /// **most frequent** rendering, with any human-confirmed rendering
    /// preferred over any count — a reviewer's decision outweighs a
    /// machine's repetition. Names with no translation yet are skipped, not
    /// invented: a glossary term without an approved form enforces nothing
    /// and would only look like coverage.
    ///
    /// The divergent renderings this method papers over do not disappear:
    /// [`Masrad::tadarubat`] reports every one of them with its occurrences,
    /// and that is the intended division of labour — seeding picks a
    /// defensible default, the consistency pass surfaces the disagreement
    /// for a human to settle.
    ///
    /// Seeded terms are project-scoped: they came out of this game's text
    /// and have no business renaming things in other projects.
    #[must_use]
    pub fn min_nusus(nusus: &[MudkhalNass]) -> Self {
        /// One rendering of one name, while votes are counted.
        #[derive(Debug, Default)]
        struct Sura {
            khaam: String,
            adad: u32,
            muakkada: bool,
        }
        /// One name's candidates.
        #[derive(Debug, Default)]
        struct Tarshih {
            masdar_khaam: String,
            suwar: BTreeMap<String, Sura>,
        }

        let mut tarshihat: BTreeMap<String, Tarshih> = BTreeMap::new();

        for mudkhal in nusus {
            if mudkhal.tasnif != TasnifNass::Ism {
                continue;
            }
            let miftah = miftah_muwahhad(&mudkhal.masdar);
            if miftah.is_empty() {
                continue;
            }
            let tarshih = tarshihat.entry(miftah).or_default();
            if tarshih.masdar_khaam.is_empty() {
                mudkhal.masdar.trim().clone_into(&mut tarshih.masdar_khaam);
            }

            let Some(hadaf) = mudkhal.hadaf.as_deref() else {
                continue;
            };
            let miftah_hadaf = miftah_muwahhad(hadaf);
            if miftah_hadaf.is_empty() {
                continue;
            }
            let sura = tarshih.suwar.entry(miftah_hadaf).or_default();
            if sura.khaam.is_empty() {
                hadaf.trim().clone_into(&mut sura.khaam);
            }
            sura.adad = sura.adad.saturating_add(1);
            sura.muakkada = sura.muakkada
                || mudkhal.muraja.hala() == taarib_mustalahat::muraja::HalatMuraja::Muakkada;
        }

        let mut masrad = Self::jadeed();
        for (miftah, tarshih) in tarshihat {
            // Confirmed renderings first, then the count, then the folded
            // form itself so ties break the same way on every run.
            let mukhtara = tarshih
                .suwar
                .iter()
                .max_by_key(|(miftah_hadaf, sura)| {
                    (
                        sura.muakkada,
                        sura.adad,
                        std::cmp::Reverse((*miftah_hadaf).clone()),
                    )
                })
                .map(|(_, sura)| sura.khaam.clone());
            let Some(arabi) = mukhtara else { continue };

            let la_yutarjam = miftah_muwahhad(&arabi) == miftah;
            let mustalah = MustalahMasrad {
                masdar: tarshih.masdar_khaam,
                arabi,
                mulahaza: Some("استُخلص تلقائيًا من أسماء اللعبة المستخرجة".to_owned()),
                nitaq: NitaqMustalah::MashruHali,
                la_yutarjam,
            };
            let _ = masrad.adkhil(mustalah);
        }
        masrad
    }

    /// Every glossary term the project renders inconsistently.
    ///
    /// A conflict is a term whose **whole-string occurrences** — strings
    /// whose entire source *is* the term, which is what item names, skill
    /// names and menu entries are — carry two or more distinct translations
    /// after folding. Every occurrence is reported with its [`NassId`],
    /// grouped by rendering, most frequent rendering first.
    ///
    /// Deliberately limited to whole-string occurrences, and the limit is
    /// what makes the resolution safe: for a string that *is* the term, the
    /// rendering is the whole target and rewriting it is exact. For a term
    /// buried in a sentence, extracting "the rendering this sentence used"
    /// would be a guess, and a repair built on a guess rewrites prose it
    /// does not understand. Buried occurrences are still policed — that is
    /// [`Masrad::afhas`], which checks containment of the approved form and
    /// flags without rewriting.
    #[must_use]
    pub fn tadarubat(&self, nusus: &[MudkhalNass]) -> Vec<TadarubMustalah> {
        // Fold every string once, keyed by folded source, so the per-term
        // pass below is lookups rather than refolds — a project has tens of
        // thousands of strings and this method exists to be run on all of
        // them at once.
        let mut bil_masdar: BTreeMap<String, Vec<&MudkhalNass>> = BTreeMap::new();
        for mudkhal in nusus {
            let miftah = miftah_muwahhad(&mudkhal.masdar);
            if miftah.is_empty() {
                continue;
            }
            bil_masdar.entry(miftah).or_default().push(mudkhal);
        }

        let mut natija = Vec::new();
        for muaadd in &self.mustalahat {
            let miftah = muaadd.mustalah.miftah();
            let Some(huduth) = bil_masdar.get(&miftah) else {
                continue;
            };

            let mut suwar: BTreeMap<String, SuratMustalah> = BTreeMap::new();
            for mudkhal in huduth {
                let Some(hadaf) = mudkhal.hadaf.as_deref() else {
                    continue;
                };
                let miftah_hadaf = miftah_muwahhad(hadaf);
                if miftah_hadaf.is_empty() {
                    continue;
                }
                let sura = suwar.entry(miftah_hadaf).or_insert_with(|| SuratMustalah {
                    shakl: hadaf.trim().to_owned(),
                    mawaqi: Vec::new(),
                });
                sura.mawaqi.push(mudkhal.id);
            }

            if suwar.len() < 2 {
                continue;
            }

            let mut suwar: Vec<SuratMustalah> = suwar.into_values().collect();
            // Most frequent first; the folded form breaks ties so the list
            // is the same on every run.
            suwar.sort_by(|awwal, thani| {
                thani
                    .mawaqi
                    .len()
                    .cmp(&awwal.mawaqi.len())
                    .then_with(|| awwal.shakl.cmp(&thani.shakl))
            });

            natija.push(TadarubMustalah {
                mustalah: muaadd.mustalah.masdar.clone(),
                mutamad: muaadd.mustalah.arabi.clone(),
                suwar,
            });
        }
        natija
    }

    /// Loads terms from a glossary file, dispatching on its extension.
    ///
    /// `.json` is read by [`Masrad::min_json`], `.tsv` and `.tab` by
    /// [`Masrad::min_tsv`]. Anything else is refused by name — see
    /// [`LAWAHIQ_MASRAD`] for why sniffing would be worse.
    ///
    /// Returns the terms rather than a [`Masrad`], because importing is two
    /// decisions and only the first belongs to the file: what the terms
    /// are, and then — the caller's — what they merge into and with what
    /// precedence ([`Masrad::admij`]).
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MasradTalif`] naming the file for an unreadable
    /// path, an unrecognised extension, or any malformed content the format
    /// loaders refuse.
    pub fn min_malaf(masar: &Path) -> Result<Vec<MustalahMasrad>, KhataTarjama> {
        let lahiqa = masar
            .extension()
            .and_then(|q| q.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();

        if !LAWAHIQ_MASRAD.contains(&lahiqa.as_str()) {
            return Err(KhataTarjama::MasradTalif {
                masar: masar.to_path_buf(),
                sabab: format!(
                    "extension {lahiqa:?} is not a glossary format; accepted: {}",
                    LAWAHIQ_MASRAD.join(", ")
                ),
            });
        }

        let nass = std::fs::read_to_string(masar).map_err(|q| KhataTarjama::MasradTalif {
            masar: masar.to_path_buf(),
            sabab: format!("it could not be read: {q}"),
        })?;
        // A UTF-8 BOM is Windows editors being helpful; stripping it here
        // means the first term of a TSV does not grow an invisible prefix.
        let nass = nass.strip_prefix('\u{FEFF}').unwrap_or(&nass);

        if lahiqa == "json" {
            Self::min_json(masar, nass)
        } else {
            Self::min_tsv(masar, nass)
        }
    }

    /// Parses a JSON glossary: an array of [`MustalahMasrad`] objects.
    ///
    /// `masdar` and `arabi` are required; `mulahaza`, `nitaq`
    /// (`"mashru_hali"` or `"aam"`, defaulting to the project scope) and
    /// `la_yutarjam` are optional. A do-not-translate entry may leave
    /// `arabi` empty and it is filled with the source form, which is what
    /// the flag means.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MasradTalif`] when the JSON does not parse, when an
    /// entry is unenforceable (named by position and source form), or when
    /// two entries collide on the same folded source form — a duplicate is
    /// refused by name, never merged silently, because a file that says two
    /// things about one term is a file its author needs to see, not a coin
    /// this loader flips.
    pub fn min_json(masar: &Path, nass: &str) -> Result<Vec<MustalahMasrad>, KhataTarjama> {
        let mut mustalahat: Vec<MustalahMasrad> =
            serde_json::from_str(nass).map_err(|q| KhataTarjama::MasradTalif {
                masar: masar.to_path_buf(),
                sabab: format!("it is not a JSON array of glossary terms: {q}"),
            })?;

        let mut maruf: BTreeMap<String, usize> = BTreeMap::new();
        for (fahras, mustalah) in mustalahat.iter_mut().enumerate() {
            let raqm = fahras.saturating_add(1);
            if mustalah.la_yutarjam && miftah_muwahhad(&mustalah.arabi).is_empty() {
                mustalah.arabi = mustalah.masdar.clone();
            }
            tahaqquq_mustalah(masar, raqm, mustalah)?;
            sajjil_miftah(masar, &mut maruf, mustalah.miftah(), raqm, &mustalah.masdar)?;
        }
        Ok(mustalahat)
    }

    /// Parses a TSV glossary.
    ///
    /// The column contract, exactly:
    ///
    /// | column | content | required |
    /// | --- | --- | --- |
    /// | 1 | `masdar` — the source form | yes, non-empty |
    /// | 2 | `arabi` — the approved Arabic form | yes, non-empty |
    /// | 3 | `nitaq` — `mashru` or `aam`; empty means `mashru` | no |
    /// | 4 | `mulahaza` — a free-text note | no |
    ///
    /// Cells are separated by single tabs and trimmed of surrounding
    /// spaces. Blank lines and lines starting with `#` are skipped, so a
    /// file can carry a commented header. Line endings may be LF or CRLF.
    /// `la_yutarjam` is **not expressible in TSV** — a flag column invites
    /// the ambiguity TSV exists to avoid; use JSON for those entries, or
    /// write the source form in both columns, which means the same thing.
    ///
    /// A malformed row is **refused by name — line number and first cell —
    /// never skipped**: a glossary loader that drops rows it does not
    /// understand imports nine hundred of a thousand terms, reports
    /// success, and the missing hundred surface one by one as terminology
    /// flags nobody can explain.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MasradTalif`] naming the line for: a row with fewer
    /// than 2 or more than 4 cells, an empty source or approved form, a
    /// scope cell that is neither `mashru` nor `aam` nor empty, or two rows
    /// colliding on the same folded source form.
    pub fn min_tsv(masar: &Path, nass: &str) -> Result<Vec<MustalahMasrad>, KhataTarjama> {
        let mut mustalahat = Vec::new();
        let mut maruf: BTreeMap<String, usize> = BTreeMap::new();

        for (fahras, satr_khaam) in nass.lines().enumerate() {
            let raqm = fahras.saturating_add(1);
            let satr = satr_khaam.trim_end_matches('\r');
            if satr.trim().is_empty() || satr.trim_start().starts_with('#') {
                continue;
            }

            let khanat: Vec<&str> = satr.split('\t').map(str::trim).collect();
            let awwal = khanat.first().copied().unwrap_or_default();
            if khanat.len() < 2 || khanat.len() > 4 {
                return Err(khata_satr(
                    masar,
                    raqm,
                    awwal,
                    &format!(
                        "has {} cells; a glossary row is masdar<TAB>arabi with optional \
                         nitaq and mulahaza cells",
                        khanat.len()
                    ),
                ));
            }

            let masdar = khanat.first().copied().unwrap_or_default();
            let arabi = khanat.get(1).copied().unwrap_or_default();
            let nitaq = match khanat.get(2).copied().unwrap_or_default() {
                "" | "mashru" => NitaqMustalah::MashruHali,
                "aam" => NitaqMustalah::Aam,
                gharib => {
                    return Err(khata_satr(
                        masar,
                        raqm,
                        masdar,
                        &format!("scope cell is {gharib:?}; it must be mashru, aam, or empty"),
                    ));
                },
            };
            let mulahaza = khanat
                .get(3)
                .copied()
                .filter(|nass_mulahaza| !nass_mulahaza.is_empty())
                .map(str::to_owned);

            let mustalah = MustalahMasrad {
                masdar: masdar.to_owned(),
                arabi: arabi.to_owned(),
                mulahaza,
                nitaq,
                la_yutarjam: miftah_muwahhad(masdar) == miftah_muwahhad(arabi),
            };
            tahaqquq_mustalah(masar, raqm, &mustalah)?;
            sajjil_miftah(masar, &mut maruf, mustalah.miftah(), raqm, &mustalah.masdar)?;
            mustalahat.push(mustalah);
        }
        Ok(mustalahat)
    }
}

/// Refuses an unenforceable term by position and source form.
fn tahaqquq_mustalah(
    masar: &Path,
    raqm: usize,
    mustalah: &MustalahMasrad,
) -> Result<(), KhataTarjama> {
    if mustalah.miftah().is_empty() {
        return Err(khata_satr(
            masar,
            raqm,
            &mustalah.masdar,
            "has an empty source form",
        ));
    }
    if miftah_muwahhad(&mustalah.arabi).is_empty() {
        return Err(khata_satr(
            masar,
            raqm,
            &mustalah.masdar,
            "has no approved form; a term that demands nothing enforces nothing",
        ));
    }
    Ok(())
}

/// Records a term's folded source form against its position, refusing a
/// collision by naming both rows.
fn sajjil_miftah(
    masar: &Path,
    maruf: &mut BTreeMap<String, usize>,
    miftah: String,
    raqm: usize,
    masdar: &str,
) -> Result<(), KhataTarjama> {
    if let Some(sabiq) = maruf.insert(miftah, raqm) {
        return Err(khata_satr(
            masar,
            raqm,
            masdar,
            &format!(
                "duplicates entry {sabiq}; one file may not say two things about one term \
                 — delete one of them"
            ),
        ));
    }
    Ok(())
}

/// A malformed glossary entry, named: file, position, first cell, and what
/// is wrong with it.
fn khata_satr(masar: &Path, raqm: usize, awwal: &str, sabab: &str) -> KhataTarjama {
    KhataTarjama::MasradTalif {
        masar: masar.to_path_buf(),
        sabab: format!("entry {raqm} ({awwal:?}) {sabab}"),
    }
}

// ---------------------------------------------------------------------------
// الاتساق — conflicts and the one-action resolution
// ---------------------------------------------------------------------------

/// One rendering of a conflicted term: the form, and every string using it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct SuratMustalah {
    /// The rendering as it appears in the project — the raw form of its
    /// first occurrence, so what the reviewer is shown is text a translator
    /// actually wrote, not a folded key.
    pub shakl: String,
    /// Every string using this rendering, in project order.
    pub mawaqi: Vec<NassId>,
}

/// One term the project renders inconsistently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TadarubMustalah {
    /// The source term.
    pub mustalah: String,
    /// The glossary's approved form — the default the interface offers when
    /// the reviewer resolves, though any observed rendering may be chosen
    /// instead.
    pub mutamad: String,
    /// The observed renderings, most frequent first.
    pub suwar: Vec<SuratMustalah>,
}

impl TadarubMustalah {
    /// How many strings are involved across every rendering.
    #[must_use]
    pub fn adad_mawaqi(&self) -> usize {
        self.suwar.iter().map(|sura| sura.mawaqi.len()).sum()
    }
}

/// One proposed edit: replace a string's target, whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TadeelMustalah {
    /// The string to edit.
    pub id: NassId,
    /// Its current target, carried so the applier can refuse a stale edit:
    /// a target that changed since the conflict was computed is a string
    /// somebody is working on, and overwriting it would race a human.
    pub min: String,
    /// The target it should become.
    pub ila: String,
}

/// Resolves a conflict to one chosen form, as a list of edits — and only a
/// list of edits.
///
/// `mukhtar` is the rendering the human chose: the approved form, or any of
/// the observed ones. Every occurrence whose current target does not
/// already fold to the choice gets one whole-target edit; occurrences
/// already agreeing produce nothing, and occurrences whose entry has
/// vanished or changed since the conflict was computed are skipped rather
/// than guessed at — the `min` field lets the applier catch the remainder.
///
/// **Nothing is applied here.** The project file has exactly one writer,
/// the project layer, which owns the review-state transition each edit
/// implies and records who resolved and when. A glossary that wrote
/// targets directly would be a second writer bypassing that record — the
/// review history would show strings changing with no transition, which is
/// indistinguishable from corruption because it is corruption.
///
/// Whole-target replacement is exact, not approximate, because
/// [`Masrad::tadarubat`] only ever reports whole-string occurrences — see
/// its documentation for why buried occurrences are flagged instead of
/// rewritten.
#[must_use]
pub fn wahhid_tadarub(
    tadarub: &TadarubMustalah,
    mukhtar: &str,
    nusus: &[MudkhalNass],
) -> Vec<TadeelMustalah> {
    let miftah_mukhtar = miftah_muwahhad(mukhtar);
    if miftah_mukhtar.is_empty() {
        return Vec::new();
    }

    let bil_id: BTreeMap<NassId, &MudkhalNass> =
        nusus.iter().map(|mudkhal| (mudkhal.id, mudkhal)).collect();

    let mut taadilat = Vec::new();
    for sura in &tadarub.suwar {
        for id in &sura.mawaqi {
            let Some(mudkhal) = bil_id.get(id) else {
                continue;
            };
            let Some(hadaf) = mudkhal.hadaf.as_deref() else {
                continue;
            };
            if miftah_muwahhad(hadaf) == miftah_mukhtar {
                continue;
            }
            taadilat.push(TadeelMustalah {
                id: *id,
                min: hadaf.to_owned(),
                ila: mukhtar.to_owned(),
            });
        }
    }
    taadilat
}
