//! الدمج — folding a capture session into a static extraction.
//!
//! Static extraction knows *where* a string lives and nothing about whether a
//! player ever sees it. Capture knows a string was on screen, how wide the box
//! was, and nothing about which file it came from — an injected adapter sees a
//! `string` reaching a text component, not the `.locres` entry three layers
//! below it. Neither is complete and the missing halves are exactly
//! complementary, so the merge key can only be **the text itself**.
//!
//! ## The four outcomes, and why the two boring ones are the important ones
//!
//! | outcome | what this module does |
//! | --- | --- |
//! | both saw the same string | [`MasdarIstikhraj::Kilahuma`], and the measured width folds into the static entry |
//! | capture saw a string static extraction never had | kept, [`MasdarIstikhraj::Multaqat`], with a location naming the session |
//! | static extraction has a string capture never saw | **left completely alone** |
//! | the two disagree | [`MasdarIstikhraj::Mutanaqid`], and both texts are kept |
//!
//! The third row is the one that is easy to get wrong. A string that capture
//! never saw is not a string that is not shown — it is a string that was not
//! shown *during this session*, and a session is somebody playing for forty
//! minutes. The ending, the shop, the death screen, the settings sub-panel
//! nobody opened: all absent, all real. Demoting them would punish a short
//! capture, which is the only kind of capture most people will ever run, and it
//! would do so silently. Absence of evidence is not evidence, so those entries
//! are not touched at all — they are listed in
//! [`TaqreerDammij::sakina_lam_tura`], which is a report, not a verdict.
//!
//! ## The merge itself is not written here
//!
//! Every fold goes through [`JadwalNusus::adif`]. That function owns the
//! provenance rules, the "tightest observed width wins" constraint fold, and the
//! classification precedence, and it owns them in one place on purpose: the
//! comment on [`MasdarIstikhraj::adif`] says plainly that a caller getting the
//! disagreement case wrong would silently lose the one signal saying the file's
//! text is not what the player reads. This module's job is to decide *which*
//! static entry a captured string belongs to and to hand `adif` an entry with
//! the right provenance stamped on it. It never merges two entries itself.
//!
//! ## One consequence worth stating plainly
//!
//! [`JadwalNusus::adif`] counts every call as a sighting of the text, so folding
//! a capture nudges that text's occurrence count up by one. That is the table's
//! own definition of the counter and this module does not work around it: a
//! private counter maintained here would disagree with the table's, and two
//! counters that disagree are worse than one that is generous. Capture's real
//! occurrence counts — the four hundred thousand frames a label was drawn for —
//! are in [`TaqreerDammij::marrat`], where the weighting Phase 13 wants can read
//! them without polluting the table.

use std::collections::{BTreeMap, BTreeSet};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use taarib_mustalahat::nass::{MasdarIstikhraj, NassId, NawNasq, NitaqNasq, TasnifNass};

use crate::iltiqat::MulahazaMutakarrira;
use crate::jadwal::{JadwalNusus, MawqiNass, MudkhalMustakhraj, irfa_nasq};

// ---------------------------------------------------------------------------
// Ceilings and defaults
// ---------------------------------------------------------------------------

/// The confidence a direct observation carries when it reclassifies a string.
///
/// The maximum, and only ever applied to a static entry classified
/// [`TasnifNass::Dakhili`]. See [`crate::tasnif::aid_tasnif_bad_iltiqat`] for
/// why that is the
/// only case, and for why it matters less than it looks.
pub const THIQAT_MUSHAHADA: u8 = 100;

/// The fewest literal characters a template must have to be matched against.
///
/// Four. A template that is almost entirely placeholders — `{0}: {1}`, `{a}/{b}`
/// — matches an enormous fraction of a game's text under any substitution rule,
/// and every one of those matches would be wrong. Four characters of literal
/// text is not much of a filter, and it is enough to exclude the templates that
/// are pure structure.
pub const HADD_ADNA_THABIT: usize = 4;

/// The longest single literal run a template must have.
///
/// Three. Total literal length alone is not enough: `{0}:{1}:{2}:{3}:` has four
/// literal characters spread across four segments and anchors nothing. A run of
/// three consecutive characters is a real anchor.
pub const HADD_ADNA_MIRSAT: usize = 3;

/// The most characters one placeholder may be considered to have been replaced
/// by.
///
/// Two hundred and fifty-six. A substitution is a name, a number, an item, a
/// button glyph, a short list. A "substitution" that swallowed four thousand
/// characters is not a substitution, it is the matcher finding a coincidence,
/// and a coincidence recorded as [`MasdarIstikhraj::Mutanaqid`] tells a
/// translator their file is wrong when it is not.
pub const AQSA_TABDEEL: usize = 256;

/// The most characters all of one template's placeholders may absorb together.
pub const AQSA_TABDEEL_KULLI: usize = 1024;

/// How many characters of a template's anchor go into the index key.
///
/// Eight. Long enough that `You have ` and `You lost ` land in different
/// buckets, short enough that looking a captured string up costs eight prefix
/// probes and eight suffix probes rather than a scan over its whole length.
pub const TUL_MIFTAH_FAHRAS: usize = 8;

/// The cap on templates anchored at neither end.
///
/// A template like `{0} of {1}` has an empty first and last segment, so neither
/// the prefix nor the suffix index can hold it and every captured string has to
/// be tested against it directly. That family is small in every real game, and
/// the cap exists so that a game where it is not small produces a report saying
/// so rather than a merge that takes twenty minutes with no explanation.
pub const AQSA_QAWALIB_MUTLAQA: usize = 4096;

/// The most templates tested against a single captured string.
///
/// Five hundred and twelve. Reached only when hundreds of templates share an
/// eight-character prefix, which happens — `You have `, `Press ` — and which
/// would otherwise turn the merge quadratic. Truncations are counted in
/// [`TaqreerDammij::murashshahat_maqsusa`] so a merge that gave up on candidates
/// says so.
pub const AQSA_MURASHSHAHAT: usize = 512;

/// The scheme that marks a location as belonging to a capture session rather
/// than to a file.
pub const BADIA_ILTIQAT: &str = "iltiqat:";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// How a merge behaves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhiyaratDammij {
    /// The session's name, which becomes the `hawiya` of every capture-only
    /// string's location.
    pub jalsa: String,

    /// Whether to attempt placeholder-aware matching.
    ///
    /// On by default. Turning it off makes every runtime substitution a
    /// capture-only string, which is not wrong — the string really was on screen
    /// — but loses the connection to the template a translator has to edit.
    pub yutabiq_bi_tabdeel: bool,

    /// Whether to keep captured strings that matched nothing.
    ///
    /// On by default, and turning it off is a real choice rather than an
    /// optimisation: a game that composes all its text at runtime produces
    /// nothing *but* capture-only strings, and a merge that discarded them would
    /// produce an empty result for exactly the games capture exists for.
    pub yahfaz_multaqat_faqat: bool,

    /// The fewest literal characters a template needs.
    pub hadd_adna_thabit: usize,

    /// The longest single literal run a template needs.
    pub hadd_adna_mirsat: usize,

    /// The most characters one placeholder may absorb.
    pub aqsa_tabdeel: usize,

    /// The most characters all placeholders may absorb together.
    pub aqsa_tabdeel_kulli: usize,
}

impl KhiyaratDammij {
    /// Options with this build's defaults.
    #[must_use]
    pub fn jadeeda(jalsa: impl Into<String>) -> Self {
        Self {
            jalsa: jalsa.into(),
            yutabiq_bi_tabdeel: true,
            yahfaz_multaqat_faqat: true,
            hadd_adna_thabit: HADD_ADNA_THABIT,
            hadd_adna_mirsat: HADD_ADNA_MIRSAT,
            aqsa_tabdeel: AQSA_TABDEEL,
            aqsa_tabdeel_kulli: AQSA_TABDEEL_KULLI,
        }
    }
}

// ---------------------------------------------------------------------------
// The placeholder-aware template
// ---------------------------------------------------------------------------

/// One static entry expressed as literal runs separated by placeholders.
///
/// Built from the entry's clean text and its own `nasq` spans — not by scanning
/// the text for brace-shaped things. That distinction is the whole reason this
/// works across five engine families: `{0}`, `{name}`, `%1$s`, `\V[3]`, `$gold`
/// and `[player]` are all placeholders in some dialect and all ordinary text in
/// another, and the spans were produced by the one markup parser in this product
/// which already knows which dialect this container speaks. A second opinion
/// here would disagree with the first the moment either learned a new dialect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QalabMawdi {
    /// The static entry this came from.
    pub huwiya: NassId,
    /// The literal runs, in order. Always one more than there are placeholders.
    pub qitaa: Vec<String>,
    /// The placeholders' raw texts, in order.
    pub mawdi: Vec<String>,
    /// How many characters of literal text the template has in total.
    pub ahruf_thabita: usize,
}

impl QalabMawdi {
    /// Builds a template from a static entry, or refuses it.
    ///
    /// Refuses when the entry has no placeholders (nothing to substitute), when
    /// its literal content is too thin to anchor a match, or when a span's byte
    /// range does not land on character boundaries in the clean text. That last
    /// one is not a formality: a span and a text that disagree about where a
    /// placeholder is describe a string neither of them actually holds, and a
    /// template built from them would match by accident or not at all.
    #[must_use]
    pub fn min_madkhal(
        huwiya: NassId,
        naqi: &str,
        nasq: &[NitaqNasq],
        khiyarat: &KhiyaratDammij,
    ) -> Option<Self> {
        let nitaqat = dharrat_murattaba(nasq);
        if nitaqat.is_empty() {
            return None;
        }

        let mut qitaa = Vec::with_capacity(nitaqat.len().saturating_add(1));
        let mut mawdi = Vec::with_capacity(nitaqat.len());
        let mut nihaya_sabiqa = 0_usize;

        for (bidaya, tul, khaam) in nitaqat {
            let nihaya = bidaya.checked_add(tul)?;
            if bidaya < nihaya_sabiqa || nihaya > naqi.len() {
                return None;
            }
            let qita = naqi.get(nihaya_sabiqa..bidaya)?;
            // The placeholder's own bytes are looked up but discarded: the clean
            // text holds a substitute character there, not the raw placeholder,
            // and reading it only proves the span is on a boundary.
            let _ = naqi.get(bidaya..nihaya)?;
            qitaa.push(qita.to_owned());
            mawdi.push(khaam);
            nihaya_sabiqa = nihaya;
        }
        qitaa.push(naqi.get(nihaya_sabiqa..)?.to_owned());

        let ahruf_thabita = qitaa.iter().map(|q| q.chars().count()).sum::<usize>();
        let atwal = qitaa.iter().map(|q| q.chars().count()).max().unwrap_or(0);
        if ahruf_thabita < khiyarat.hadd_adna_thabit || atwal < khiyarat.hadd_adna_mirsat {
            return None;
        }

        Some(Self {
            huwiya,
            qitaa,
            mawdi,
            ahruf_thabita,
        })
    }

    /// The literal run this template starts with.
    #[must_use]
    pub fn bidaya(&self) -> &str {
        self.qitaa.first().map_or("", String::as_str)
    }

    /// The literal run this template ends with.
    #[must_use]
    pub fn nihaya(&self) -> &str {
        self.qitaa.last().map_or("", String::as_str)
    }

    /// Matches a captured string against this template.
    ///
    /// ## The algorithm, and why it is not a substring check
    ///
    /// A template is `s0 · hole · s1 · hole · … · sn`, which is a wildcard
    /// pattern with the wildcards pinned to known positions. The prefix is
    /// anchored with `starts_with`, the suffix with `ends_with`, and the
    /// interior runs are found leftmost inside the window between them. Greedy
    /// leftmost matching is a complete decision procedure for this shape — taking
    /// the earliest occurrence of an interior run never blocks a later one,
    /// because any later choice leaves strictly less room — so a match that
    /// exists is found and a match that is reported exists.
    ///
    /// A substring check would accept `Hello, {name}` against `Say Hello, then
    /// leave` and against every other string containing `Hello,`. This does not:
    /// the prefix and the suffix are anchored, the interior runs must appear in
    /// order, and every hole is bounded.
    ///
    /// Returns what each placeholder was replaced by, in order.
    #[must_use]
    pub fn tabaq(&self, multaqat: &str, khiyarat: &KhiyaratDammij) -> Option<Vec<String>> {
        let awwal = self.bidaya();
        let akhir = self.nihaya();
        if !multaqat.starts_with(awwal) || !multaqat.ends_with(akhir) {
            return None;
        }

        let bidaya = awwal.len();
        let nihaya = multaqat.len().checked_sub(akhir.len())?;
        if nihaya < bidaya {
            // The prefix and the suffix overlap, so no assignment of the holes
            // can produce this string.
            return None;
        }
        let nafidha = multaqat.get(bidaya..nihaya)?;

        let mut badail = Vec::with_capacity(self.mawdi.len());
        let mut kulli = 0_usize;
        let mut mawqi = 0_usize;

        // Every run except the first and the last; the first is the anchored
        // prefix and the last is the anchored suffix.
        let dakhiliya = self.qitaa.get(1..self.qitaa.len().saturating_sub(1))?;
        for qita in dakhiliya {
            let baqi = nafidha.get(mawqi..)?;
            let mawdi_qita = baqi.find(qita.as_str())?;
            let badil = baqi.get(..mawdi_qita)?;
            kulli = tasjil_badil(&mut badail, badil, kulli, khiyarat)?;
            mawqi = mawqi.checked_add(mawdi_qita)?.checked_add(qita.len())?;
        }
        let badil = nafidha.get(mawqi..)?;
        let _ = tasjil_badil(&mut badail, badil, kulli, khiyarat)?;

        if badail.len() == self.mawdi.len() {
            Some(badail)
        } else {
            None
        }
    }

    /// How specific this template is, for choosing between several that match.
    ///
    /// More literal characters first, then fewer placeholders. A template with
    /// more fixed text and fewer holes is constraining more of the captured
    /// string and is the better explanation of it.
    const fn tafdil(&self) -> (usize, std::cmp::Reverse<usize>, NassId) {
        (
            self.ahruf_thabita,
            std::cmp::Reverse(self.mawdi.len()),
            self.huwiya,
        )
    }
}

/// Records one substituted fragment, enforcing both bounds.
fn tasjil_badil(
    badail: &mut Vec<String>,
    badil: &str,
    kulli: usize,
    khiyarat: &KhiyaratDammij,
) -> Option<usize> {
    let tul = badil.chars().count();
    if tul > khiyarat.aqsa_tabdeel {
        return None;
    }
    let majmu = kulli.checked_add(tul)?;
    if majmu > khiyarat.aqsa_tabdeel_kulli {
        return None;
    }
    badail.push(badil.to_owned());
    Some(majmu)
}

/// The atom spans of a static entry, sorted and non-overlapping.
///
/// Style spans are dropped: an italic run is not a substitution point and
/// treating it as one would let a template match text that never contained it.
/// Overlapping atoms are dropped rather than merged, because an atom inside
/// another atom is a parse this module cannot interpret and guessing would
/// produce a template with a hole where the text has none.
fn dharrat_murattaba(nasq: &[NitaqNasq]) -> Vec<(usize, usize, String)> {
    let mut nitaqat: Vec<(usize, usize, String)> = nasq
        .iter()
        .filter_map(|nitaq| {
            let khaam = match &nitaq.naw {
                NawNasq::Mawdi { khaam } => khaam.clone(),
                NawNasq::Sura { marja } => marja.clone(),
                _ => return None,
            };
            Some((
                usize::try_from(nitaq.bidaya).ok()?,
                usize::try_from(nitaq.tul).ok()?,
                khaam,
            ))
        })
        .collect();
    nitaqat.sort_by_key(|(bidaya, tul, _)| (*bidaya, *tul));

    let mut munaqqa: Vec<(usize, usize, String)> = Vec::with_capacity(nitaqat.len());
    let mut nihaya_sabiqa = 0_usize;
    for (bidaya, tul, khaam) in nitaqat {
        if bidaya < nihaya_sabiqa {
            continue;
        }
        let Some(nihaya) = bidaya.checked_add(tul) else {
            continue;
        };
        nihaya_sabiqa = nihaya;
        munaqqa.push((bidaya, tul, khaam));
    }
    munaqqa
}

// ---------------------------------------------------------------------------
// The by-content index
// ---------------------------------------------------------------------------

/// What a substitution match found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutabaqatQalab {
    /// The static entry whose template matched.
    pub huwiya: NassId,
    /// Its placeholders, in order.
    pub mawdi: Vec<String>,
    /// What each was replaced by, in order.
    pub badail: Vec<String>,
    /// Whether more than one template matched and this one was chosen.
    pub mubham: bool,
    /// Whether the candidate list was cut short by [`AQSA_MURASHSHAHAT`].
    pub maqsusa: bool,
}

/// Every static entry, indexed by the content a capture can be matched against.
///
/// Built once and reusable across sessions, which is the reason it is a type
/// rather than a local: merging four evenings of capture into one project should
/// walk a hundred thousand static entries once, not four times.
///
/// ## Why there is an index at all
///
/// The exact half could be a linear scan and would be unusable: twenty thousand
/// captured strings against a hundred thousand static entries is two billion
/// string comparisons. The substitution half is worse, because each test is a
/// prefix check plus a suffix check plus interior searches.
///
/// So templates are anchored. A template whose first literal run is non-empty is
/// filed under that run's first [`TUL_MIFTAH_FAHRAS`] characters; one whose
/// first run is empty but whose last is not — `{name} has arrived` — is filed
/// under the last run's final characters; one anchored at neither end goes into
/// a short list that is scanned. Looking a captured string up costs at most
/// sixteen hash lookups and that list. The two anchored families cover
/// essentially every template a real game has, because interface text is written
/// by people and people put words around their placeholders.
#[derive(Debug, Default)]
pub struct FahrasNusus {
    /// Source texts exactly as stored, to the entries holding them.
    tamm_khaam: FxHashMap<String, Vec<NassId>>,
    /// Clean texts, for adapters that strip markup before the takeover point.
    tamm_naqi: FxHashMap<String, Vec<NassId>>,
    /// Every template.
    qawalib: Vec<QalabMawdi>,
    /// Templates by the opening characters of their first literal run.
    bi_bidaya: FxHashMap<String, Vec<usize>>,
    /// Templates by the closing characters of their last literal run.
    bi_nihaya: FxHashMap<String, Vec<usize>>,
    /// Templates anchored at neither end.
    mutlaqa: Vec<usize>,
    /// Every static identity, for the never-seen report.
    huwiyat: BTreeSet<NassId>,
    /// Templates refused because [`AQSA_QAWALIB_MUTLAQA`] was reached.
    qawalib_mustabada: usize,
}

impl FahrasNusus {
    /// Indexes a static table.
    #[must_use]
    pub fn min_jadwal(jadwal: &JadwalNusus, khiyarat: &KhiyaratDammij) -> Self {
        let mut fahras = Self::default();
        for madkhal in jadwal.madakhil() {
            let huwiya = madkhal.huwiya();
            let _ = fahras.huwiyat.insert(huwiya);
            fahras
                .tamm_khaam
                .entry(madkhal.khaam.clone())
                .or_default()
                .push(huwiya);
            if madkhal.naqi != madkhal.khaam {
                fahras
                    .tamm_naqi
                    .entry(madkhal.naqi.clone())
                    .or_default()
                    .push(huwiya);
            }
            if let Some(qalab) =
                QalabMawdi::min_madkhal(huwiya, &madkhal.naqi, &madkhal.nasq, khiyarat)
            {
                fahras.aqhim(qalab);
            }
        }
        fahras
    }

    /// How many static entries were indexed.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.huwiyat.len()
    }

    /// How many templates were built.
    #[must_use]
    pub const fn adad_qawalib(&self) -> usize {
        self.qawalib.len()
    }

    /// How many templates were refused for want of room in the unanchored list.
    #[must_use]
    pub const fn qawalib_mustabada(&self) -> usize {
        self.qawalib_mustabada
    }

    /// Every static identity.
    pub fn huwiyat(&self) -> impl Iterator<Item = NassId> + '_ {
        self.huwiyat.iter().copied()
    }

    /// The entries whose stored text is exactly this.
    #[must_use]
    pub fn tabaq_khaam(&self, nass: &str) -> &[NassId] {
        self.tamm_khaam.get(nass).map_or(&[], Vec::as_slice)
    }

    /// The entries whose *clean* text is exactly this.
    ///
    /// A separate lookup, and a separate count in the report, because the two
    /// mean different things. A capture that equals the stored text saw exactly
    /// what the file holds. A capture that equals only the clean text saw the
    /// file's words with the file's markup already consumed by the engine — a
    /// `<b>` that never reached the screen as characters. The player read the
    /// same sentence, so this is agreement rather than contradiction, but a
    /// contributor debugging why a bold tag vanished deserves to be able to tell
    /// the two apart.
    #[must_use]
    pub fn tabaq_naqi(&self, nass: &str) -> &[NassId] {
        self.tamm_naqi.get(nass).map_or(&[], Vec::as_slice)
    }

    /// Finds the template that best explains a captured string.
    ///
    /// Several can match — `Level {0}` and `Level {0} reached` are different
    /// templates and a capture reading `Level 7` matches only the first, but
    /// close pairs do occur. The winner is the most specific: most literal
    /// characters, then fewest placeholders, then the lowest identity. The last
    /// tiebreaker is arbitrary and, the part that matters, **stable** — merging
    /// the same session into the same table twice picks the same template, so a
    /// project does not reshuffle its own contradictions on every merge.
    #[must_use]
    pub fn tabaq_bi_tabdeel(
        &self,
        nass: &str,
        khiyarat: &KhiyaratDammij,
    ) -> Option<MutabaqatQalab> {
        let (murashshahat, maqsusa) = self.murashshahat(nass);
        let mut afdal: Option<(&QalabMawdi, Vec<String>)> = None;
        let mut adad = 0_usize;

        for fahras in murashshahat {
            let Some(qalab) = self.qawalib.get(fahras) else {
                continue;
            };
            let Some(badail) = qalab.tabaq(nass, khiyarat) else {
                continue;
            };
            adad = adad.saturating_add(1);
            let ahsan = afdal
                .as_ref()
                .is_none_or(|(sabiq, _)| qalab.tafdil() > sabiq.tafdil());
            if ahsan {
                afdal = Some((qalab, badail));
            }
        }

        let (qalab, badail) = afdal?;
        Some(MutabaqatQalab {
            huwiya: qalab.huwiya,
            mawdi: qalab.mawdi.clone(),
            badail,
            mubham: adad > 1,
            maqsusa,
        })
    }

    /// Files one template under whichever end anchors it.
    fn aqhim(&mut self, qalab: QalabMawdi) {
        let fahras = self.qawalib.len();
        let bidaya = miftah_bidaya(qalab.bidaya());
        let nihaya = miftah_nihaya(qalab.nihaya());

        match (bidaya, nihaya) {
            (Some(miftah), _) => self.bi_bidaya.entry(miftah).or_default().push(fahras),
            (None, Some(miftah)) => self.bi_nihaya.entry(miftah).or_default().push(fahras),
            (None, None) => {
                if self.mutlaqa.len() >= AQSA_QAWALIB_MUTLAQA {
                    self.qawalib_mustabada = self.qawalib_mustabada.saturating_add(1);
                    return;
                }
                self.mutlaqa.push(fahras);
            },
        }
        self.qawalib.push(qalab);
    }

    /// Every template worth testing against this string, and whether the list
    /// was cut short.
    fn murashshahat(&self, nass: &str) -> (Vec<usize>, bool) {
        let mut murashshahat: Vec<usize> = Vec::new();
        let mut mushahad: FxHashSet<usize> = FxHashSet::default();
        let mut maqsusa = false;

        let mut damm = |qaima: &[usize], murashshahat: &mut Vec<usize>, maqsusa: &mut bool| {
            for fahras in qaima {
                if murashshahat.len() >= AQSA_MURASHSHAHAT {
                    *maqsusa = true;
                    return;
                }
                if mushahad.insert(*fahras) {
                    murashshahat.push(*fahras);
                }
            }
        };

        let ahruf: Vec<char> = nass.chars().take(TUL_MIFTAH_FAHRAS).collect();
        for tul in 1..=ahruf.len() {
            let Some(bidaya) = ahruf.get(..tul) else {
                continue;
            };
            let miftah: String = bidaya.iter().collect();
            if let Some(qaima) = self.bi_bidaya.get(&miftah) {
                damm(qaima, &mut murashshahat, &mut maqsusa);
            }
        }

        let mut akhira: Vec<char> = nass.chars().rev().take(TUL_MIFTAH_FAHRAS).collect();
        akhira.reverse();
        for tul in 1..=akhira.len() {
            let Some(nihaya) = akhira.get(akhira.len().saturating_sub(tul)..) else {
                continue;
            };
            let miftah: String = nihaya.iter().collect();
            if let Some(qaima) = self.bi_nihaya.get(&miftah) {
                damm(qaima, &mut murashshahat, &mut maqsusa);
            }
        }

        damm(&self.mutlaqa, &mut murashshahat, &mut maqsusa);
        (murashshahat, maqsusa)
    }
}

/// The index key for a template's opening run, when it has one.
fn miftah_bidaya(qita: &str) -> Option<String> {
    if qita.is_empty() {
        return None;
    }
    Some(qita.chars().take(TUL_MIFTAH_FAHRAS).collect())
}

/// The index key for a template's closing run, when it has one.
fn miftah_nihaya(qita: &str) -> Option<String> {
    if qita.is_empty() {
        return None;
    }
    let mut ahruf: Vec<char> = qita.chars().rev().take(TUL_MIFTAH_FAHRAS).collect();
    ahruf.reverse();
    Some(ahruf.into_iter().collect())
}

// ---------------------------------------------------------------------------
// The disagreement
// ---------------------------------------------------------------------------

/// A string whose file text and screen text are not the same string.
///
/// ## Why both texts live here and not in the table
///
/// The table holds one source text per entry, and for a substituting game there
/// is exactly one correct answer for what that text should be: the **template**.
/// `Hello, {name}` is what a translator has to edit, because it is what the game
/// will re-instantiate with a different name every time. Writing `Hello, Alice`
/// into the table as a second entry would put a string in front of a translator
/// that they must not translate, and one they cannot write back anywhere;
/// overwriting the template with it would be worse, and is precisely what
/// [`JadwalNusus::adif`]'s doc comment forbids.
///
/// So the entry stays the template, its provenance becomes
/// [`MasdarIstikhraj::Mutanaqid`], and the instantiation is kept here — with the
/// placeholders and what each was replaced by — so the workspace can show
/// "the file says this, the screen showed that" beside the string. That is the
/// information a translator actually needs: it tells them `{name}` really is a
/// name and not a code, which decides whether the Arabic around it needs a
/// definite article, a vocative, or agreement they would otherwise guess at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutabaqaMustabdala {
    /// The static entry.
    pub huwiya: NassId,
    /// The file's text, exactly as stored.
    pub khaam_sakin: String,
    /// The file's text with markup and placeholders lifted out.
    pub naqi_sakin: String,
    /// What the screen showed.
    pub nass_multaqat: String,
    /// The template's placeholders, in order.
    pub mawdi: Vec<String>,
    /// What each was replaced by, in order.
    pub badail: Vec<String>,
    /// The component that drew it.
    pub masar_mukawwin: String,
    /// How many times capture saw it.
    pub marrat: u64,
}

impl MutabaqaMustabdala {
    /// The sentence the workspace shows beside the string.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let azwaj: Vec<String> = self
            .mawdi
            .iter()
            .zip(self.badail.iter())
            .map(|(mawdi, badil)| {
                if badil.is_empty() {
                    format!("{mawdi} → (nothing)")
                } else {
                    format!("{mawdi} → {badil}")
                }
            })
            .collect();
        format!(
            "The file holds \"{}\"; the screen showed \"{}\" ({}).",
            self.naqi_sakin,
            self.nass_multaqat,
            azwaj.join(", ")
        )
    }
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// What a merge did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaqreerDammij {
    /// The session that was merged.
    pub jalsa: String,
    /// How many captured strings were offered.
    pub mulahazat: usize,
    /// How many static entries the table held before the merge.
    pub sakina: usize,
    /// Captured strings that equalled a static entry's stored text exactly.
    pub mutabaqa_tamma: usize,
    /// Captured strings that equalled a static entry's *clean* text.
    ///
    /// See [`FahrasNusus::tabaq_naqi`] for why this is counted apart.
    pub mutabaqa_naqiya: usize,
    /// Captured strings matched through a placeholder substitution.
    pub mutabaqa_bi_tabdeel: usize,
    /// Captured strings that matched nothing and were kept on their own.
    pub multaqat_faqat: usize,
    /// Captured strings that matched nothing and were discarded.
    ///
    /// Non-zero only when [`KhiyaratDammij::yahfaz_multaqat_faqat`] is off.
    pub muhmala: usize,
    /// Static entries no capture ever reached.
    ///
    /// The identities and not merely a count, because the useful question is
    /// "which screens has nobody visited yet", and that is a list a contributor
    /// can act on. **Not a verdict**: see this module's header.
    pub sakina_lam_tura: Vec<NassId>,
    /// Every string whose file text and screen text disagree.
    pub tanaqudat: Vec<MutabaqaMustabdala>,
    /// Substitution matches where more than one template fitted.
    pub mubhama: usize,
    /// Captured strings whose markup the parser refused.
    ///
    /// Kept anyway, with no spans. A capture is evidence that a string was on
    /// screen, and a markup dialect this build cannot parse does not make that
    /// evidence false — it makes the spans unavailable, which costs placeholder
    /// verification for that one string and nothing else.
    pub nasq_marfud: usize,
    /// Captured strings whose candidate template list was cut short.
    pub murashshahat_maqsusa: usize,
    /// How many templates the index built.
    pub qawalib: usize,
    /// How many templates were refused for want of room.
    pub qawalib_mustabada: usize,
    /// How many times capture drew each matched string.
    ///
    /// Kept here rather than folded into the table's own occurrence count, which
    /// counts places in the game's data. These are frames. Mixing the two would
    /// make the duplicate-group weighting meaningless — a label drawn every frame
    /// would outweigh a hundred distinct lines of dialogue.
    pub marrat: BTreeMap<NassId, u64>,
}

impl TaqreerDammij {
    /// How many captured strings reached a static entry, by any route.
    #[must_use]
    pub const fn majmu_mutabaqa(&self) -> usize {
        self.mutabaqa_tamma
            .saturating_add(self.mutabaqa_naqiya)
            .saturating_add(self.mutabaqa_bi_tabdeel)
    }

    /// How many static entries capture confirmed.
    #[must_use]
    pub const fn sakina_musahada(&self) -> usize {
        self.sakina.saturating_sub(self.sakina_lam_tura.len())
    }

    /// The fraction of the static table capture actually reached.
    ///
    /// **This is the real coverage number**, and the reason the one a live
    /// session reports is called an estimate. A session can hold ninety percent
    /// as many strings as the static table and still have confirmed a third of
    /// it, because the two sets overlap rather than nest. Only a merge, matching
    /// by content, knows the intersection.
    #[must_use]
    pub fn nisbat_musahada(&self) -> f32 {
        nisba(self.sakina_musahada(), self.sakina)
    }

    /// The report as lines, for the log and the diagnostics bundle.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "merged capture session \"{}\": {} captured string(s) against {} static entry/entries",
            self.jalsa, self.mulahazat, self.sakina
        )];
        sutur.push(format!(
            "  {} matched exactly, {} matched the clean text, {} matched through a runtime \
             substitution",
            self.mutabaqa_tamma, self.mutabaqa_naqiya, self.mutabaqa_bi_tabdeel
        ));
        sutur.push(format!(
            "  {} capture-only string(s), which have no file to be written back to",
            self.multaqat_faqat
        ));
        sutur.push(format!(
            "  {} static entry/entries were never seen — which means this session did not reach \
             them, not that they are unused",
            self.sakina_lam_tura.len()
        ));
        sutur.push(format!(
            "  capture confirmed {:.0}% of the static table",
            self.nisbat_musahada() * 100.0
        ));
        if !self.tanaqudat.is_empty() {
            sutur.push(format!(
                "  {} string(s) read differently on screen than in the file; both texts are kept",
                self.tanaqudat.len()
            ));
        }
        if self.mubhama > 0 {
            sutur.push(format!(
                "  {} substitution(s) fitted more than one template; the most specific was chosen",
                self.mubhama
            ));
        }
        if self.nasq_marfud > 0 {
            sutur.push(format!(
                "  {} captured string(s) had markup this build could not parse; they were kept \
                 without spans",
                self.nasq_marfud
            ));
        }
        if self.murashshahat_maqsusa > 0 || self.qawalib_mustabada > 0 {
            sutur.push(format!(
                "  {} candidate list(s) were cut short and {} template(s) were not indexed; some \
                 substitutions may have been missed",
                self.murashshahat_maqsusa, self.qawalib_mustabada
            ));
        }
        sutur
    }
}

/// A ratio that treats an empty denominator as complete.
///
/// An empty static table is fully confirmed: there was nothing to confirm. The
/// opposite convention would report a game with no readable containers as zero
/// percent covered by a capture that found every string it has.
#[expect(
    clippy::cast_precision_loss,
    reason = "string counts are far below 2^53, and the result is a percentage rendered to the \
              nearest whole number"
)]
fn nisba(juz: usize, kull: usize) -> f32 {
    if kull == 0 {
        return 1.0;
    }
    juz as f32 / kull as f32
}

// ---------------------------------------------------------------------------
// The merge
// ---------------------------------------------------------------------------

/// Folds a capture session into a static extraction.
///
/// Builds the index and merges in one call. A caller merging several sessions
/// into one table should build a [`FahrasNusus`] once and use
/// [`dammij_bi_fahras`] instead — the index is the expensive half and it does not
/// change between sessions.
#[must_use]
pub fn dammij_iltiqat(
    jadwal: &mut JadwalNusus,
    mulahazat: &[MulahazaMutakarrira],
    khiyarat: &KhiyaratDammij,
) -> TaqreerDammij {
    let fahras = FahrasNusus::min_jadwal(jadwal, khiyarat);
    dammij_bi_fahras(jadwal, &fahras, mulahazat, khiyarat)
}

/// The same, against an index that already exists.
///
/// The index describes the table as it was when the index was built, so merging
/// a second session through the same index will not match against the
/// capture-only entries the first session added. That is deliberate and it is
/// the safe direction: a capture-only entry has no file behind it, so matching a
/// later capture against it would fold two runtime observations into one entry
/// whose location names an arbitrary one of the two sessions. Two entries
/// sharing a text are pointed at one duplicate-group leader by
/// [`JadwalNusus::ila_mudkhalat`] anyway, so one translation still serves both.
#[must_use]
pub fn dammij_bi_fahras(
    jadwal: &mut JadwalNusus,
    fahras: &FahrasNusus,
    mulahazat: &[MulahazaMutakarrira],
    khiyarat: &KhiyaratDammij,
) -> TaqreerDammij {
    let mut taqreer = TaqreerDammij {
        jalsa: khiyarat.jalsa.clone(),
        mulahazat: mulahazat.len(),
        sakina: fahras.adad(),
        qawalib: fahras.adad_qawalib(),
        qawalib_mustabada: fahras.qawalib_mustabada(),
        ..TaqreerDammij::default()
    };
    let mut malmusa: FxHashSet<NassId> = FxHashSet::default();

    for mulahaza in mulahazat {
        let khaam = fahras.tabaq_khaam(&mulahaza.nass);
        if !khaam.is_empty() {
            taqreer.mutabaqa_tamma = taqreer.mutabaqa_tamma.saturating_add(1);
            iftil_ala(
                jadwal,
                khaam,
                mulahaza,
                MasdarIstikhraj::Multaqat,
                &mut taqreer,
            );
            malmusa.extend(khaam.iter().copied());
            continue;
        }

        let naqi = fahras.tabaq_naqi(&mulahaza.nass);
        if !naqi.is_empty() {
            taqreer.mutabaqa_naqiya = taqreer.mutabaqa_naqiya.saturating_add(1);
            iftil_ala(
                jadwal,
                naqi,
                mulahaza,
                MasdarIstikhraj::Multaqat,
                &mut taqreer,
            );
            malmusa.extend(naqi.iter().copied());
            continue;
        }

        if khiyarat.yutabiq_bi_tabdeel
            && let Some(mutabaqa) = fahras.tabaq_bi_tabdeel(&mulahaza.nass, khiyarat)
        {
            if mutabaqa.mubham {
                taqreer.mubhama = taqreer.mubhama.saturating_add(1);
            }
            if mutabaqa.maqsusa {
                taqreer.murashshahat_maqsusa = taqreer.murashshahat_maqsusa.saturating_add(1);
            }
            taqreer.mutabaqa_bi_tabdeel = taqreer.mutabaqa_bi_tabdeel.saturating_add(1);
            sajjil_tanaqud(jadwal, &mutabaqa, mulahaza, &mut taqreer);
            let huwiyat = [mutabaqa.huwiya];
            iftil_ala(
                jadwal,
                &huwiyat,
                mulahaza,
                MasdarIstikhraj::Mutanaqid,
                &mut taqreer,
            );
            let _ = malmusa.insert(mutabaqa.huwiya);
            continue;
        }

        if !khiyarat.yahfaz_multaqat_faqat {
            taqreer.muhmala = taqreer.muhmala.saturating_add(1);
            continue;
        }
        taqreer.multaqat_faqat = taqreer.multaqat_faqat.saturating_add(1);
        let (madkhal, nasq_maqru) = madkhal_multaqat(&khiyarat.jalsa, mulahaza);
        if !nasq_maqru {
            taqreer.nasq_marfud = taqreer.nasq_marfud.saturating_add(1);
        }
        let huwiya = madkhal.huwiya();
        let adad = taqreer.marrat.entry(huwiya).or_insert(0);
        *adad = adad.saturating_add(mulahaza.marrat);
        jadwal.adif(madkhal);
    }

    taqreer.sakina_lam_tura = fahras
        .huwiyat()
        .filter(|huwiya| !malmusa.contains(huwiya))
        .collect();
    taqreer
}

/// Folds one capture into every static entry that matched it.
///
/// ## Folding into *every* match, and why that is right rather than merely easy
///
/// A text can appear in several places in a game's data — `Yes` four hundred
/// times — and a capture cannot tell which occurrence it saw. Folding the
/// measured width into all of them over-constrains: the narrow confirmation
/// dialog's width is applied to the wide one too.
///
/// That over-constraint is correct here, for two reasons. It is conservative in
/// the direction that matters — a translation that fits the narrowest occurrence
/// fits every occurrence, whereas the opposite error ships text that clips off
/// the screen. And the duplicate-group model makes it exact rather than merely
/// safe: [`JadwalNusus::ila_mudkhalat`] points every entry sharing a text at one
/// leader, so there is *one* translation for all four hundred occurrences of
/// `Yes`, and the constraint that translation must satisfy is the tightest box
/// any of them was drawn in. Which is what folding into all of them produces.
fn iftil_ala(
    jadwal: &mut JadwalNusus,
    huwiyat: &[NassId],
    mulahaza: &MulahazaMutakarrira,
    masdar: MasdarIstikhraj,
    taqreer: &mut TaqreerDammij,
) {
    for huwiya in huwiyat {
        if iftil(jadwal, *huwiya, mulahaza, masdar) {
            let adad = taqreer.marrat.entry(*huwiya).or_insert(0);
            *adad = adad.saturating_add(mulahaza.marrat);
        }
    }
}

/// Hands one capture-derived entry to [`JadwalNusus::adif`].
///
/// The entry carries the **static** entry's own location and stored text, so its
/// identity is the static entry's identity and `adif` folds rather than inserts.
/// Everything else on it is what capture contributes: the measured constraints,
/// the scene, and the provenance stamp.
///
/// The provenance is stamped here rather than derived by `adif`, and that is the
/// one thing this function must not get wrong.
/// [`MasdarIstikhraj::adif`] resolves `Mutanaqid` on either side to `Mutanaqid`
/// unconditionally, so passing it is how a substitution match survives the fold;
/// passing `Multaqat` with matching texts is how an exact match becomes
/// `Kilahuma`. Getting that backwards would either lose the contradiction signal
/// or invent one.
fn iftil(
    jadwal: &mut JadwalNusus,
    huwiya: NassId,
    mulahaza: &MulahazaMutakarrira,
    masdar: MasdarIstikhraj,
) -> bool {
    let Some(sakin) = jadwal.madkhal(huwiya) else {
        return false;
    };
    let (tasnif, thiqa) = crate::tasnif::aid_tasnif_bad_iltiqat(sakin.tasnif);
    let qayd = MudkhalMustakhraj {
        mawqi: sakin.mawqi.clone(),
        khaam: sakin.khaam.clone(),
        naqi: sakin.naqi.clone(),
        nasq: sakin.nasq.clone(),
        tasnif,
        thiqa,
        masdar,
        tarmiz: sakin.tarmiz.clone(),
        quyud: mulahaza.quyud.clone(),
        mutakallim: None,
        jiwar: Vec::new(),
        mashhad: mulahaza.mashhad.clone(),
    };
    jadwal.adif(qayd);
    true
}

/// Records the file text beside the screen text for one disagreement.
fn sajjil_tanaqud(
    jadwal: &JadwalNusus,
    mutabaqa: &MutabaqatQalab,
    mulahaza: &MulahazaMutakarrira,
    taqreer: &mut TaqreerDammij,
) {
    let Some(sakin) = jadwal.madkhal(mutabaqa.huwiya) else {
        return;
    };
    taqreer.tanaqudat.push(MutabaqaMustabdala {
        huwiya: mutabaqa.huwiya,
        khaam_sakin: sakin.khaam.clone(),
        naqi_sakin: sakin.naqi.clone(),
        nass_multaqat: mulahaza.nass.clone(),
        mawdi: mutabaqa.mawdi.clone(),
        badail: mutabaqa.badail.clone(),
        masar_mukawwin: mulahaza.masar_mukawwin.clone(),
        marrat: mulahaza.marrat,
    });
}

/// What a captured string becomes when nothing in the files matches it.
///
/// ## Such a string has nowhere to be written back to
///
/// Its location names a capture session, not a container, and there is no
/// `Data/Map001.json` with a field to rewrite. That is not a defect in the
/// record — it is the truth about the string. A game that builds a sentence at
/// runtime from three fragments and a number has no stored sentence, and no
/// amount of searching its files will produce one.
///
/// Phase 14 keys on exactly this. [`MasdarIstikhraj::sakin`] answers `false`
/// here, and a string that answers `false` cannot go down the container-rewriter
/// path: there is no container. It goes down the **runtime** path instead — the
/// injected adapter holds the translation and substitutes it at the takeover
/// point, matching on the drawn text, which is the only place the string exists
/// as a whole. A patch compiler that tried to write these back would either
/// silently drop them or corrupt a file trying to find them.
///
/// The second value is whether the markup parser accepted the text. When it did
/// not, the entry is kept with its raw text as its clean text and no spans, and
/// the caller counts it: a dialect this build cannot parse costs placeholder
/// verification for that string, and nothing else.
#[must_use]
pub fn madkhal_multaqat(jalsa: &str, mulahaza: &MulahazaMutakarrira) -> (MudkhalMustakhraj, bool) {
    let (naqi, nasq, maqru) = match irfa_nasq(&mulahaza.nass) {
        Ok((naqi, nasq)) => (naqi, nasq, true),
        Err(_) => (mulahaza.nass.clone(), Vec::new(), false),
    };
    let madkhal = MudkhalMustakhraj {
        mawqi: mawqi_iltiqat(jalsa, mulahaza),
        khaam: mulahaza.nass.clone(),
        naqi,
        nasq,
        // Seen on screen, so not internal; nothing about a component path says
        // whether it is dialogue, a label or an error, so it is unclassified
        // rather than guessed. `Majhul` and `Dakhili` are distinct in the
        // vocabulary for exactly this: "I do not know" is not "I know it is
        // hidden", and the interface and the translator treat them differently.
        tasnif: TasnifNass::Majhul,
        thiqa: THIQAT_MUSHAHADA,
        masdar: MasdarIstikhraj::Multaqat,
        tarmiz: None,
        quyud: mulahaza.quyud.clone(),
        mutakallim: None,
        jiwar: Vec::new(),
        mashhad: mulahaza.mashhad.clone(),
    };
    (madkhal, maqru)
}

/// The location of a string that only a capture session ever saw.
///
/// `hawiya` names the session rather than a file, prefixed with
/// [`BADIA_ILTIQAT`] so nothing downstream mistakes it for a path and tries to
/// open it. The component path goes in `mawqi`, which is structural in the
/// engine's own terms and therefore survives a rebuild — the property identity
/// depends on. The scene goes in `asl` and the screen state in `haql`.
///
/// Two sessions that both saw the same unlocatable string produce two entries,
/// because the session is part of the identity. That is the right trade: making
/// the identity session-independent would make merging one session twice
/// idempotent and merging two sessions lossy in a different way, and the
/// duplicate-group machinery already handles the consequence — the two entries
/// share a text, so [`JadwalNusus::ila_mudkhalat`] points them at one leader and
/// one translation serves both.
#[must_use]
pub fn mawqi_iltiqat(jalsa: &str, mulahaza: &MulahazaMutakarrira) -> MawqiNass {
    // A component path is the location. When an adapter could not produce one —
    // a text system with no scene graph behind it — the dedup key stands in: it
    // is derived from the text and the (empty) path, so it is stable across
    // re-merges of the same session, which is what identity needs.
    let mawqi = if mulahaza.masar_mukawwin.trim().is_empty() {
        format!("#{}", mulahaza.miftah)
    } else {
        mulahaza.masar_mukawwin.clone()
    };
    MawqiNass {
        hawiya: format!("{BADIA_ILTIQAT}{jalsa}"),
        asl: mulahaza.mashhad.clone(),
        mawqi,
        haql: mulahaza.shasha.clone(),
        miftah_muharrik: None,
    }
}

/// Whether a location came from a capture session rather than from a container.
///
/// What Phase 14 asks before choosing a delivery path. A named function rather
/// than a prefix comparison spelled out at four call sites, because a fifth call
/// site that forgot the prefix would send a runtime-only string to the container
/// rewriter and the failure would be silent — an attempt to rewrite a file whose
/// name is a session's name, which does not exist.
#[must_use]
pub fn min_iltiqat(mawqi: &MawqiNass) -> bool {
    mawqi.hawiya.starts_with(BADIA_ILTIQAT)
}
