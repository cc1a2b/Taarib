//! الدفعات — batch runs: many strings, one provider, one budget, and a promise
//! that stopping never loses anything.
//!
//! A batch run is where this crate touches money and other people's servers,
//! and both facts shape every structure in this module.
//!
//! ## The four promises, and what breaks without each
//!
//! **The ceiling is honoured *at* the ceiling, never past it.** Cost is
//! reserved against the budget *before* a request is dispatched, from the
//! provider's own estimate, and settled to the actual afterwards. A run that
//! checks after the reply has already spent the money it is about to refuse to
//! spend — it has broken the ceiling's promise before reporting it, and the
//! contributor finds out from their card statement rather than from this
//! product. With concurrent requests in flight the reservation is the only
//! correct shape: two chunks each individually under the ceiling can jointly
//! exceed it, and only an atomic reserve can see that before either sends.
//!
//! **Every result is committed the moment it arrives.** Each translated or
//! failed string is appended, immediately and individually, to a journal file
//! inside the project directory — the same JSON Lines append discipline the
//! project itself uses for its strings, and for a stronger reason: an
//! extraction batch lost to a crash is re-read for free, but a translation
//! lost to a crash is *paid for again*. That is why this journal commits per
//! line where the project commits per five hundred.
//!
//! **A run resumes exactly where it stopped.** The journal is the checkpoint,
//! explicitly: on open it is read back, every string it already answers is
//! skipped, and the money it records as spent seeds the ledger so a resumed
//! run still honours the original ceiling cumulatively. Nothing is inferred
//! from "the project happens to have been written" — the project's own file is
//! updated by [`tabbiq_sijill`] *from* the journal, not the other way around.
//!
//! **Progress does not lie.** Translated, skipped and remaining are counts;
//! failures are counts *with their reasons*, per string. A progress bar that
//! says "3 failed" and cannot say why is asking the contributor to re-run the
//! batch to find out, at full price.
//!
//! ## Protection is not optional and has no second path
//!
//! Every string is tokenised by [`crate::hima::ihmi`] before a provider sees
//! it, and every reply is verified by [`crate::hima::istaridd`] before it is
//! believed. [`NassMahmi`] has no accessor for the raw source, so this module
//! *could not* send unprotected text even by mistake; a retry builds a fresh
//! [`NassMahmi`] because one protected string answers one request, exactly
//! once.
//!
//! ## String failures, run failures, and the one rate-limit exception
//!
//! [`KhataTarjama::yuqif_aljawla`] decides whether a failure stops the run or
//! only the string — this module never enumerates variants to decide that,
//! because a variant added later must inherit the right behaviour without
//! anyone remembering this file exists. The single variant handled specially
//! *before* that check is the rate limit: a 429 with a `Retry-After` is an
//! instruction to wait, not a verdict, so it earns a bounded number of waits
//! that honour the header — and only when those are exhausted is it the
//! "backoff did not clear it" failure its variant describes, at which point
//! [`KhataTarjama::yuqif_aljawla`] stops the run like any other.
//!
//! ## The interface this module consumes from [`crate::muzawwidun`]
//!
//! Providers are behind [`crate::muzawwidun`]'s trait. This module reads from
//! it: the provider's name, its [`QudratMuzawwid`] — whether it accepts
//! multi-string requests, how many strings and characters fit in one, and its
//! request rate — a pre-dispatch cost estimate, and per-string results
//! carrying the raw reply, a confidence score with its `maqisa` marker, and
//! the actual cost where the provider reports one.
//!
//! [`NassMahmi`]: crate::hima::NassMahmi
//! [`QudratMuzawwid`]: crate::muzawwidun::QudratMuzawwid

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::StreamExt as _;
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NassId, NitaqNasq};
use taarib_mustalahat::ruqaa::TareeqaTarjama;

use crate::alamat::{AtabatAlamat, ThiqaMublagha, alam_min_khata, ihsib_alamat_nass, thabbit_alamat};
use crate::hima::{NassMahmi, NassMustaad, ihmi, istaridd};
use crate::khata::{KhataTarjama, tul_u64};
use crate::muraja_dakhiliya::SijillMuraja;
use crate::muzawwidun::{Muzawwid, NatijatTarjama, QudratMuzawwid, TalabTarjama};
use crate::siyaq::SiyaqTalab;

/// The run journal's file name, inside the project directory.
///
/// Beside the project's own `mashru.json` and `nusus.jsonl`, because the
/// journal *is* project data: it records paid-for work, it must travel with
/// the project when the directory is copied, and a checkpoint stored anywhere
/// else — a cache directory, a temp file — is a checkpoint that is not there
/// on the machine the contributor resumes on.
pub const MALAF_SIJILL_JAWLA: &str = "jawlat_tarjama.jsonl";

/// How a batch run behaves. Everything a contributor can turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KhiyaratJawla {
    /// The cost ceiling, in the smallest unit of the run's currency, or
    /// [`None`] for no ceiling.
    ///
    /// Enforced by reservation before dispatch — see the module header. A
    /// resumed run's ledger starts at what the journal already recorded as
    /// spent, so the ceiling is cumulative across resumes of the same run; a
    /// contributor who wants to spend more raises it.
    pub saqf_takalif: Option<u64>,
    /// How many attempts one string gets before it is failed with its reason.
    ///
    /// Three. Reply-side protection failures and malformed replies are very
    /// often prompt luck — the same string succeeds on the next attempt — but
    /// each attempt costs money, and a string that failed three times is a
    /// string a human should look at rather than a slot machine to keep
    /// pulling.
    pub aqsa_muhawalat: u32,
    /// How many rate-limit waits the run tolerates before treating the limit
    /// as the run-stopping failure its variant describes.
    ///
    /// Separate from `aqsa_muhawalat` because a 429 is not a failure of the
    /// string — retrying a *different* string would hit the same limit — and
    /// billing it against the string's budget would fail strings the provider
    /// never even judged.
    pub aqsa_intizar_muadal: u32,
    /// How many requests may be in flight at once against this provider.
    ///
    /// Four. High enough that request latency overlaps; low enough that a
    /// provider's burst limits are not what ends the run. Bounded per run —
    /// and a run is per provider — so one provider's generosity never sets
    /// another's load.
    pub tawazi: usize,
    /// The first backoff delay, in milliseconds. Doubles per attempt.
    pub asas_tarajua_millithania: u64,
    /// The largest backoff delay, in milliseconds, whatever the doubling says.
    pub aqsa_tarajua_millithania: u64,
    /// The moment this run started, seconds since the Unix epoch.
    ///
    /// Supplied by the caller, never read from a clock here — the discipline
    /// every record-writing module in this product follows, so a journal is
    /// reproducible and a test does not depend on the wall. Every journal line
    /// and review transition this run writes is stamped with the run's moment,
    /// which is honest at the granularity anything downstream reads it.
    pub lahza: u64,
    /// The same moment as RFC 3339.
    ///
    /// The runner itself never touches entry timestamps — it writes the
    /// journal, and the journal is numeric time. This field exists so the run
    /// configuration carries everything the *whole* workflow needs: a caller
    /// that folds the journal immediately hands this to [`tabbiq_sijill`] as
    /// its `waqt`, rather than reading a second clock and stamping the fold a
    /// few seconds after the run it belongs to.
    pub waqt: String,
    /// The thresholds the arrival-time quality flags are computed against.
    pub atabat: AtabatAlamat,
}

impl Default for KhiyaratJawla {
    fn default() -> Self {
        Self {
            saqf_takalif: None,
            aqsa_muhawalat: 3,
            aqsa_intizar_muadal: 2,
            tawazi: 4,
            asas_tarajua_millithania: 500,
            aqsa_tarajua_millithania: 30_000,
            lahza: 0,
            waqt: String::new(),
            atabat: AtabatAlamat::default(),
        }
    }
}

/// The cost ledger: what is spent, what is reserved, and the one gate.
///
/// The reserve-then-settle shape exists because requests are concurrent. A
/// check against `munfaq` alone would let two in-flight chunks, each under the
/// ceiling on its own, jointly pass it — the classic time-of-check gap — so a
/// dispatch first *reserves* its estimate atomically, and the ceiling test is
/// made against spent-plus-reserved. When the reply arrives the reservation is
/// settled to the actual cost where the provider reports one, or confirmed at
/// the estimate where it does not.
#[derive(Debug, Default)]
struct DaftarTakalif {
    /// Settled spend.
    munfaq: u64,
    /// Reserved by requests currently in flight.
    mahjuz: u64,
}

impl DaftarTakalif {
    /// A ledger already holding what a previous run's journal recorded.
    const fn min_sabiq(munfaq: u64) -> Self {
        Self { munfaq, mahjuz: 0 }
    }

    /// Reserves an estimate, or refuses because the ceiling would be passed.
    ///
    /// Refusal is exact: the reservation that *would* cross the ceiling is the
    /// one refused, so the run stops at the ceiling with the money for the
    /// refused request unspent. The error carries what was actually spent, not
    /// spent-plus-reserved, because in-flight reservations settle before the
    /// run returns and quoting them would report money that was never paid.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::SaqfTakalif`] when spending the estimate would exceed
    /// the ceiling.
    const fn ihjiz(&mut self, taqdeer: u64, saqf: Option<u64>) -> Result<(), KhataTarjama> {
        let baad = self.munfaq.saturating_add(self.mahjuz).saturating_add(taqdeer);
        if let Some(saqf) = saqf
            && baad > saqf
        {
            return Err(KhataTarjama::SaqfTakalif { munfaq: self.munfaq, saqf });
        }
        self.mahjuz = self.mahjuz.saturating_add(taqdeer);
        Ok(())
    }

    /// Settles a reservation to what was actually spent.
    ///
    /// `fieli` is the provider-reported cost when there is one, else the
    /// estimate stands. An actual above the estimate is accepted and recorded
    /// — the ledger's job is to be true, and the overshoot is what makes the
    /// *next* reservation refuse — and an actual below it releases the
    /// difference for later strings.
    fn saffi(&mut self, taqdeer: u64, fieli: Option<u64>) -> u64 {
        self.mahjuz = self.mahjuz.saturating_sub(taqdeer);
        let mablagh = fieli.unwrap_or(taqdeer);
        self.munfaq = self.munfaq.saturating_add(mablagh);
        mablagh
    }

    /// Releases a reservation whose request never completed.
    const fn afrij(&mut self, taqdeer: u64) {
        self.mahjuz = self.mahjuz.saturating_sub(taqdeer);
    }
}

/// One string's outcome, as the journal records it.
///
/// Carries everything needed to update the project entry later, so that
/// [`tabbiq_sijill`] is a pure fold of journal into table and never has to
/// re-derive anything a crash could have interrupted the derivation of.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum HasilatNass {
    /// Translated, verified by [`crate::hima::istaridd`], and accepted.
    Tarjumat {
        /// The restored clean Arabic.
        hadaf: String,
        /// The spans over it, offsets recomputed for the Arabic.
        nasq: Vec<NitaqNasq>,
        /// The provider's confidence, kept only when it measured one.
        thiqa: Option<f32>,
        /// The quality flags computable at arrival time.
        ///
        /// Overflow is **not** among them: the runner has no fonts in flight,
        /// so per [`crate::alamat`]'s discipline the flag is absent rather
        /// than estimated, and the workshop recomputes flags once fonts are
        /// loaded.
        alamat: Vec<AlamJawda>,
        /// The review record, opened at machine-translated.
        ///
        /// [`SijillMuraja::sajjil_aali`] is the recording, and it lands in a
        /// state that cannot reach approved — journaling the record whole is
        /// what lets a resumed project rebuild review state without a code
        /// path that could accidentally mint a different one.
        muraja: SijillMuraja,
    },
    /// Failed, with the reason a contributor reads.
    Fashilat {
        /// Why, in the error's own words.
        sabab: String,
        /// Whether the failure was a protection refusal — a placeholder lost,
        /// duplicated, invented or damaged.
        himaya: bool,
        /// The flags the failure produced, normally one
        /// [`AlamJawda::NasqMaksur`] when `himaya` is true.
        alamat: Vec<AlamJawda>,
    },
}

/// One journal line: which string, what happened, what it cost, when.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaydJawla {
    /// The string.
    pub id: NassId,
    /// Which provider handled it.
    pub muzawwid: String,
    /// What it cost, in the smallest currency unit — settled, not estimated.
    ///
    /// Recorded for failures too: a reply that failed verification was still
    /// paid for, and a ledger that forgot failed spend would let a resumed run
    /// overshoot the ceiling by exactly the cost of every failure.
    pub taklifa: u64,
    /// The run's moment, seconds since the Unix epoch.
    pub lahza: u64,
    /// What happened.
    pub hasila: HasilatNass,
}

/// The run journal: the checkpoint, explicit and on disk.
///
/// One JSON object per line, appended per result. On open the whole file is
/// read back into a map keyed by string identity, **last line wins** — a
/// string retried by a later run has both lines in the file and the newer
/// outcome in the map, which is also why the journal is never rewritten in
/// place: the history of attempts is itself the record of what was paid for.
///
/// A torn final line — the signature of a crash mid-append — is skipped with
/// a count, exactly as the project's own string file handles it: one torn
/// line must not cost the contributor the nine thousand whole ones above it.
#[derive(Debug)]
pub struct SijillJawla {
    /// The journal file.
    masar: PathBuf,
    /// Every string already answered, last outcome per string.
    sabiq: BTreeMap<NassId, QaydJawla>,
    /// What the journal records as already spent, across every line.
    munfaq_sabiq: u64,
    /// How many lines would not parse.
    talifa: usize,
}

impl SijillJawla {
    /// Opens the journal inside a project directory, reading back whatever a
    /// previous run left.
    ///
    /// A missing file is an empty journal — the state every first run starts
    /// in — not an error.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::KhataMalaf`] when the file exists and cannot be read,
    /// which stops the run before it spends anything: a run that cannot see
    /// what was already paid for cannot honour a cumulative ceiling.
    pub fn iftah(mujallad_mashru: &Path) -> Result<Self, KhataTarjama> {
        let masar = mujallad_mashru.join(MALAF_SIJILL_JAWLA);
        let mut sijill = Self {
            masar: masar.clone(),
            sabiq: BTreeMap::new(),
            munfaq_sabiq: 0,
            talifa: 0,
        };
        if !masar.is_file() {
            return Ok(sijill);
        }
        let bayt = std::fs::read(&masar)
            .map_err(|sabab| KhataTarjama::KhataMalaf { masar, sabab })?;

        // Bytes first, UTF-8 per line: a crash mid-append leaves invalid UTF-8
        // at the tail, and decoding the whole file first would turn one torn
        // line into a total refusal to resume.
        for satr in bayt.split(|bayt| *bayt == b'\n') {
            if satr.is_empty() {
                continue;
            }
            let qayd = std::str::from_utf8(satr)
                .ok()
                .and_then(|nass| serde_json::from_str::<QaydJawla>(nass).ok());
            match qayd {
                Some(qayd) => {
                    sijill.munfaq_sabiq = sijill.munfaq_sabiq.saturating_add(qayd.taklifa);
                    let _ = sijill.sabiq.insert(qayd.id, qayd);
                }
                None => sijill.talifa = sijill.talifa.saturating_add(1),
            }
        }
        Ok(sijill)
    }

    /// Whether a string is already answered by this journal.
    #[must_use]
    pub fn ajaba(&self, id: NassId) -> bool {
        self.sabiq.contains_key(&id)
    }

    /// The recorded outcome for a string, when there is one.
    #[must_use]
    pub fn qayd(&self, id: NassId) -> Option<&QaydJawla> {
        self.sabiq.get(&id)
    }

    /// Every recorded outcome, for [`tabbiq_sijill`] and the review console.
    #[must_use]
    pub const fn quyud(&self) -> &BTreeMap<NassId, QaydJawla> {
        &self.sabiq
    }

    /// What the journal records as spent, which seeds a resumed run's ledger.
    #[must_use]
    pub const fn munfaq_sabiq(&self) -> u64 {
        self.munfaq_sabiq
    }

    /// How many lines of a previous journal would not parse.
    #[must_use]
    pub const fn talifa(&self) -> usize {
        self.talifa
    }

    /// Commits one result: appended to disk first, then to the in-memory map.
    ///
    /// Disk first, because the order is the crash guarantee: a record in
    /// memory and not on disk is a translation a crash un-pays for, while the
    /// reverse merely re-reads a line on resume. Not batched, deliberately —
    /// see the module header for why this journal diverges from the project's
    /// five-hundred-line batches.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::KhataMalaf`] when the append fails, which
    /// [`KhataTarjama::yuqif_aljawla`] classifies as run-stopping: a run that
    /// cannot commit results is a run that would pay for work it cannot keep.
    pub fn sajjil(&mut self, qayd: QaydJawla) -> Result<(), KhataTarjama> {
        let satr = serde_json::to_string(&qayd).map_err(|sabab| KhataTarjama::KhataMalaf {
            masar: self.masar.clone(),
            sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, sabab),
        })?;
        let mut nass = satr;
        nass.push('\n');
        ilhaq_sijill(&self.masar, nass.as_bytes())?;
        let _ = self.sabiq.insert(qayd.id, qayd);
        Ok(())
    }
}

/// Appends bytes to the journal, creating it on first write, and puts them on
/// the disk before returning.
///
/// Not atomic, exactly as the project's own append is not: atomic would mean
/// rewrite-whole, and the recoverable cost of a torn last line is the price of
/// never re-serializing ten thousand paid-for results to add one.
///
/// ## Why this one fsyncs when the project's own append does not
///
/// `write_all` hands the bytes to the operating system and returns. That is
/// enough to survive *this process* dying — the crate header's "a crash costs
/// at most the requests in flight" — and it is not enough to survive the
/// machine stopping, which is the case the header actually names: **"nothing
/// is paid for twice because a laptop closed"**. A closed laptop that never
/// wakes, a pulled plug, a dead battery, a hard reset all discard whatever the
/// page cache had not written, and every discarded line is a translation the
/// contributor paid for and will be charged for again on resume.
///
/// So this flushes and returns only once the filesystem says the line is
/// durable. The cost is one `fdatasync` per translated string, on a path that
/// has just waited on a network round trip to a translation service — tens of
/// microseconds guarding something that cost tens of milliseconds and real
/// money. `sync_data` rather than `sync_all` because the file's modification
/// time is not what is being protected; the bytes are.
fn ilhaq_sijill(masar: &Path, bayt: &[u8]) -> Result<(), KhataTarjama> {
    use std::io::Write as _;

    let khata = |sabab| KhataTarjama::KhataMalaf { masar: masar.to_path_buf(), sabab };
    let mut malaf = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(masar)
        .map_err(khata)?;
    malaf.write_all(bayt).map_err(khata)?;
    // A failed sync is a run-stopping `KhataMalaf` like a failed write, and
    // for the same reason: `sajjil`'s caller must not count a result as
    // committed when the disk has not said it is.
    malaf.sync_data().map_err(khata)
}

/// Why strings were skipped, separately countable.
///
/// One number labelled "skipped" invites exactly the support question it
/// cannot answer: "why did it only translate half my game?" The split is the
/// answer — already done by a previous run, already translated by a human,
/// frozen against bulk work, or nothing there to translate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TafsilTakhatti {
    /// Already answered by this run's journal — the resume case.
    pub fi_alsijill: usize,
    /// Already carrying a translation in the project.
    pub mutarjama_musbaqan: usize,
    /// Frozen: `SijillMuraja::mujammad` means bulk operations keep out.
    pub mujammada: usize,
    /// The source itself is empty or whitespace.
    pub farigha: usize,
}

impl TafsilTakhatti {
    /// Every skip, together.
    #[must_use]
    pub const fn majmu(&self) -> usize {
        self.fi_alsijill
            .saturating_add(self.mutarjama_musbaqan)
            .saturating_add(self.mujammada)
            .saturating_add(self.farigha)
    }
}

/// Where a run stands, at any moment during it and after it.
///
/// Failures carry their reasons per string. This is the honesty requirement
/// stated in the module header made structural: there is no field for a bare
/// failure count, so a caller displaying progress *has* the reasons and can
/// show them, and a count is derived by asking the list its length.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaqaddumJawla {
    /// Successfully translated and committed, this run.
    pub mutarjama: usize,
    /// Failed, with the reason each one failed.
    pub fashila: Vec<(NassId, String)>,
    /// Skipped, split by why.
    pub mutakhattaha: TafsilTakhatti,
    /// Eligible and not yet attempted.
    pub mutabaqqiya: usize,
    /// Settled spend so far, smallest currency unit, including previous runs
    /// recorded in the journal.
    pub munfaq: u64,
    /// The ceiling this run honours, when one is set.
    pub saqf: Option<u64>,
}

/// What a finished — or stopped — run reports.
#[derive(Debug)]
pub struct TaqreerJawla {
    /// The final progress state.
    pub taqaddum: TaqaddumJawla,
    /// What stopped the run early, when something did.
    ///
    /// [`None`] means the run walked its whole list. [`Some`] is always a
    /// failure for which [`KhataTarjama::yuqif_aljawla`] is true — a ceiling
    /// reached, a provider down, a journal that would not write — and
    /// everything committed before it is committed still; re-running with the
    /// same project resumes past it.
    pub tawaqquf: Option<KhataTarjama>,
    /// Torn or unreadable lines found in a previous journal on open.
    pub sijill_talif: usize,
}

/// One protected string waiting to be dispatched.
///
/// Borrows its entry from the run's table rather than cloning it: the table is
/// immutable for the whole run, the entry's context — speaker, surrounding
/// lines, constraints — is read at dispatch time to build the request, and a
/// clone per string would double a fifty-thousand-string run's memory for no
/// answer that the borrow does not already give.
#[derive(Debug)]
struct BandMuallaq<'a> {
    /// The string's identity, for the journal.
    id: NassId,
    /// The protected text for the first attempt. A retry builds a fresh one —
    /// one protected string answers one request, exactly once.
    mahmi: NassMahmi,
    /// The protected text's length in characters: the cost-estimate basis,
    /// and what chunk packing sums against the provider's request size.
    ahruf: u64,
    /// The entry, for request context, atoms, and the success clone.
    mudkhal: &'a MudkhalNass,
}

/// Splits the pending strings into requests sized by the provider's own
/// capability — never by a fixed number.
///
/// A fixed chunk of, say, fifty was the obvious first design and it fails in
/// both directions at once: fifty one-word menu labels is a fraction of what a
/// large-context model accepts per request, so the run pays fifty times the
/// per-request overhead it needed to; and fifty full dialogue paragraphs
/// overruns a small provider's request size, so the provider truncates or
/// refuses and the whole chunk fails for a packing decision the provider never
/// made. The provider states its capability in [`QudratMuzawwid`]; packing
/// reads it.
///
/// A provider that does not batch gets one string per request. A single
/// string larger by itself than the per-request character budget is sent alone
/// anyway rather than dropped here: whether it is too big is the provider's
/// judgement to make, and its refusal then fails that one string with the
/// provider's own reason instead of a silent disappearance at packing time.
fn qassim_dufaat<'a>(
    bunud: Vec<BandMuallaq<'a>>,
    qudrat: &QudratMuzawwid,
) -> Vec<Vec<BandMuallaq<'a>>> {
    if !qudrat.dufaat {
        return bunud.into_iter().map(|band| vec![band]).collect();
    }
    let aqsa_nusus = qudrat.aqsa_nusus().max(1);
    let mut dufaat: Vec<Vec<BandMuallaq<'a>>> = Vec::new();
    let mut haliya: Vec<BandMuallaq<'a>> = Vec::new();
    let mut ahruf_haliya = 0_u64;

    for band in bunud {
        let yamla = !haliya.is_empty()
            && (haliya.len() >= aqsa_nusus
                || ahruf_haliya.saturating_add(band.ahruf) > tul_u64(qudrat.aqsa_hajm_talab));
        if yamla {
            dufaat.push(std::mem::take(&mut haliya));
            ahruf_haliya = 0;
        }
        ahruf_haliya = ahruf_haliya.saturating_add(band.ahruf);
        haliya.push(band);
    }
    if !haliya.is_empty() {
        dufaat.push(haliya);
    }
    dufaat
}

/// The backoff delay for an attempt: exponential from the configured base,
/// capped at the configured ceiling.
fn muddat_tarajua(khiyarat: &KhiyaratJawla, muhawala: u32) -> Duration {
    let mudaaf = 1_u64.checked_shl(muhawala).unwrap_or(u64::MAX);
    let milli = khiyarat
        .asas_tarajua_millithania
        .saturating_mul(mudaaf)
        .min(khiyarat.aqsa_tarajua_millithania);
    Duration::from_millis(milli)
}

/// The wait a rate-limit failure earns.
///
/// The provider's own `Retry-After`, when it sent one, wins over the
/// exponential schedule whenever it is longer — waiting *less* than the server
/// asked is how a client turns one 429 into a ban — and the exponential floor
/// still applies when the header asked for less than the schedule would have
/// waited anyway.
fn muddat_intizar_muadal(
    khiyarat: &KhiyaratJawla,
    muhawala: u32,
    thawani: Option<u64>,
) -> Duration {
    let tarajua = muddat_tarajua(khiyarat, muhawala);
    match thawani {
        Some(thawani) => tarajua.max(Duration::from_secs(thawani)),
        None => tarajua,
    }
}

/// The provider's request limiter, or none when it declares no limit.
fn hadd_muadal(qudrat: &QudratMuzawwid) -> Option<DefaultDirectRateLimiter> {
    Some(qudrat.hadd_talabat)
        .map(|fi_daqiqa| RateLimiter::direct(Quota::per_minute(fi_daqiqa)))
}

/// Everything the concurrent workers share.
struct HalatJawla<'a> {
    /// The cost ledger.
    daftar: Mutex<DaftarTakalif>,
    /// The journal — the checkpoint.
    sijill: Mutex<SijillJawla>,
    /// Live progress.
    taqaddum: Mutex<TaqaddumJawla>,
    /// The first run-stopping failure, kept whole for the report.
    tawaqquf: Mutex<Option<KhataTarjama>>,
    /// Raised when the run must stop; workers check it before starting work.
    awqif: AtomicBool,
    /// The provider's request limiter.
    hadd: Option<DefaultDirectRateLimiter>,
    /// Where live progress is published, when the caller wants it.
    nashir: Option<&'a tokio::sync::watch::Sender<TaqaddumJawla>>,
}

impl std::fmt::Debug for HalatJawla<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The limiter and the watch sender have no useful Debug of their own;
        // what a debugger wants from a run state is the progress.
        f.debug_struct("HalatJawla")
            .field("taqaddum", &self.taqaddum.lock())
            .field("awqif", &self.awqif.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl HalatJawla<'_> {
    /// Publishes the current progress to the watcher, when there is one.
    fn anshur(&self) {
        if let Some(nashir) = self.nashir {
            let hali = self.taqaddum.lock().clone();
            let _ = nashir.send_replace(hali);
        }
    }

    /// Records a run-stopping failure and raises the stop flag.
    ///
    /// First failure wins: with several workers in flight, the second and
    /// third stoppers are almost always consequences of the first — a closed
    /// connection pool, the same ceiling — and reporting a consequence as the
    /// cause sends the contributor debugging the wrong thing.
    fn awqif_aljawla(&self, khata: KhataTarjama) {
        {
            let mut tawaqquf = self.tawaqquf.lock();
            if tawaqquf.is_none() {
                *tawaqquf = Some(khata);
            }
        }
        self.awqif.store(true, Ordering::SeqCst);
    }

    /// Whether the run has been told to stop.
    fn mutawaqqif(&self) -> bool {
        self.awqif.load(Ordering::SeqCst)
    }
}

/// What one dispatched request came back as.
#[derive(Debug)]
enum RaddTalab {
    /// A reply per string, each paired with what it settled as costing.
    Najah(Vec<(NatijatTarjama, u64)>),
    /// The request failed; the caller classifies it with
    /// [`KhataTarjama::yuqif_aljawla`].
    Fashal(KhataTarjama),
    /// The run was stopped by another worker while this one was waiting; no
    /// outcome exists and none is recorded.
    Mutawaqqif,
}

/// Dispatches one request: rate gate, cost reservation, the call, settlement.
///
/// The order inside is the module's promises in miniature and none of the
/// steps commute:
///
/// 1. **The rate gate first.** Waiting *after* reserving would hold budget
///    hostage during the wait and starve parallel workers of ceiling room they
///    could have used.
/// 2. **Reserve before dispatch.** The one place the ceiling is enforced; a
///    refusal here is the run stopping at the ceiling with the refused money
///    unspent.
/// 3. **Dispatch.**
/// 4. **Settle or release.** A reply settles per string at the provider's
///    actual where it reports one. A *transport* failure releases the whole
///    reservation — a request the provider never completed is assumed unbilled,
///    which is how the APIs this crate targets meter — and a reply that
///    arrived malformed (wrong count) settles at the estimate, because a
///    malformed reply was still generated and still billed.
///
/// Rate-limit failures get a bounded number of waits honouring `Retry-After`
/// before they are allowed to be the run-stopping failure their variant
/// describes; see the module header for why this is the one variant handled
/// by name.
///
/// The settled-at-estimate cost of a malformed multi-string reply lives in the
/// ledger but under no journal line — no single string owns it. A crash
/// between that settlement and the degraded per-string outcomes loses at most
/// that one chunk's estimate from the resumed ledger; bounded, and preferred
/// over inventing a journal line that would mark some string answered when it
/// is not.
async fn talab_wahid<M>(
    muzawwid: &M,
    talabat: &[TalabTarjama<'_>],
    takdirat: &[u64],
    hala: &HalatJawla<'_>,
    khiyarat: &KhiyaratJawla,
) -> RaddTalab
where
    M: Muzawwid + ?Sized + Sync,
{
    let majmu: u64 = takdirat.iter().fold(0_u64, |jam, takdir| jam.saturating_add(*takdir));
    let mut intizar = 0_u32;

    loop {
        if hala.mutawaqqif() {
            return RaddTalab::Mutawaqqif;
        }
        if let Some(hadd) = &hala.hadd {
            hadd.until_ready().await;
        }

        // The ceiling gate. Refusing here, before the request exists, is what
        // "stops at the ceiling" means; every other placement checks after
        // money moved.
        let hajz = hala.daftar.lock().ihjiz(majmu, khiyarat.saqf_takalif);
        if let Err(khata) = hajz {
            return RaddTalab::Fashal(khata);
        }

        match muzawwid.tarjim_dufa(talabat).await {
            Ok(natijat) => {
                if natijat.len() != talabat.len() {
                    let adad = natijat.len();
                    let munfaq_alan = {
                        let mut daftar = hala.daftar.lock();
                        let _ = daftar.saffi(majmu, None);
                        daftar.munfaq
                    };
                    hala.taqaddum.lock().munfaq = munfaq_alan;
                    hala.anshur();
                    return RaddTalab::Fashal(KhataTarjama::RaddGhayrMufassal {
                        muzawwid: muzawwid.ism().to_owned(),
                        radd: format!(
                            "{adad} result(s) for {} string(s) in one request",
                            talabat.len()
                        ),
                    });
                }

                let mut azwaj: Vec<(NatijatTarjama, u64)> = Vec::with_capacity(natijat.len());
                let munfaq_alan = {
                    let mut daftar = hala.daftar.lock();
                    for (natija, takdir) in natijat.into_iter().zip(takdirat.iter()) {
                        let mablagh = daftar.saffi(*takdir, Some(natija.taklifa()));
                        azwaj.push((natija, mablagh));
                    }
                    daftar.munfaq
                };
                hala.taqaddum.lock().munfaq = munfaq_alan;
                hala.anshur();
                return RaddTalab::Najah(azwaj);
            }
            Err(khata) => {
                hala.daftar.lock().afrij(majmu);
                if let KhataTarjama::HaddMuadal { thawani, .. } = &khata
                    && intizar < khiyarat.aqsa_intizar_muadal
                {
                    let mudda = muddat_intizar_muadal(khiyarat, intizar, *thawani);
                    tracing::debug!(
                        muzawwid = %muzawwid.ism(),
                        thawani = mudda.as_secs(),
                        "حدّ المعدل بلغ؛ انتظار قبل إعادة الإرسال"
                    );
                    tokio::time::sleep(mudda).await;
                    intizar = intizar.saturating_add(1);
                    continue;
                }
                return RaddTalab::Fashal(khata);
            }
        }
    }
}

/// Builds a failure journal line for one string.
fn qayd_fashal(
    id: NassId,
    ism_muzawwid: &str,
    taklifa: u64,
    lahza: u64,
    khata: &KhataTarjama,
    dharrat: &[String],
) -> QaydJawla {
    let alamat: Vec<AlamJawda> = alam_min_khata(khata, dharrat).into_iter().collect();
    QaydJawla {
        id,
        muzawwid: ism_muzawwid.to_owned(),
        taklifa,
        lahza,
        hasila: HasilatNass::Fashilat {
            sabab: khata.to_string(),
            himaya: khata.khalal_himaya(),
            alamat,
        },
    }
}

/// Commits one success: journal line first, then the progress counters.
///
/// If the journal append fails the translation is **not** counted as done and
/// the run stops — an uncommitted translation counted as done would be skipped
/// by the next resume and its money silently lost, which is precisely the
/// failure the journal exists to make impossible. The string stays in
/// "remaining", and the resumed run pays for it again knowingly; the journal
/// failure being run-stopping is what keeps that double payment to one string.
fn sajjil_najah(
    hala: &HalatJawla<'_>,
    khiyarat: &KhiyaratJawla,
    ism_muzawwid: &str,
    id: NassId,
    mudkhal: &MudkhalNass,
    mustaad: NassMustaad,
    natija: &NatijatTarjama,
    taklifa: u64,
) {
    let mut muraja = SijillMuraja::jadeed();
    muraja.sajjil_aali(khiyarat.lahza);

    // The flags computable at arrival: no fonts are in flight, so the overflow
    // measurement is deliberately absent rather than estimated, and glossary
    // flags arrive later from the glossary pass. See `crate::alamat`.
    let mut muswadda = mudkhal.clone();
    muswadda.hadaf = Some(mustaad.naqi.clone());
    muswadda.nasq_hadaf.clone_from(&mustaad.nasq);
    // A provider that reported nothing leaves `maqisa` false, and `alamat`
    // ignores the value entirely in that case — the zero is the filler the
    // struct's own documentation describes, never a measured score.
    let thiqa =
        ThiqaMublagha { qeema: natija.thiqa().unwrap_or(0.0), maqisa: natija.maqisa() };
    let alamat = ihsib_alamat_nass(
        &muswadda,
        Some(&muraja),
        Some(&thiqa),
        &[],
        None,
        &khiyarat.atabat,
    );

    let qayd = QaydJawla {
        id,
        muzawwid: ism_muzawwid.to_owned(),
        taklifa,
        lahza: khiyarat.lahza,
        hasila: HasilatNass::Tarjumat {
            hadaf: mustaad.naqi,
            nasq: mustaad.nasq,
            thiqa: natija.maqisa().then_some(natija.thiqa()).flatten(),
            alamat,
            muraja,
        },
    };
    let tadwin = hala.sijill.lock().sajjil(qayd);
    if let Err(khata) = tadwin {
        hala.awqif_aljawla(khata);
        return;
    }
    {
        let mut taqaddum = hala.taqaddum.lock();
        taqaddum.mutarjama = taqaddum.mutarjama.saturating_add(1);
        taqaddum.mutabaqqiya = taqaddum.mutabaqqiya.saturating_sub(1);
    }
    hala.anshur();
}

/// Commits one failure, with its reason, to the journal and the progress.
///
/// `taklifa` is what the attempts on this string actually settled as costing —
/// failed verification is still paid for, and a journal that recorded failures
/// as free would let a resumed run's ledger drift under the true spend by the
/// price of every failure.
fn sajjil_fashal(
    hala: &HalatJawla<'_>,
    khiyarat: &KhiyaratJawla,
    ism_muzawwid: &str,
    id: NassId,
    mudkhal: &MudkhalNass,
    khata: &KhataTarjama,
    taklifa: u64,
) {
    let qayd = qayd_fashal(id, ism_muzawwid, taklifa, khiyarat.lahza, khata, &mudkhal.dharrat());
    let tadwin = hala.sijill.lock().sajjil(qayd);
    if let Err(khata_sijill) = tadwin {
        hala.awqif_aljawla(khata_sijill);
        return;
    }
    {
        let mut taqaddum = hala.taqaddum.lock();
        taqaddum.fashila.push((id, khata.to_string()));
        taqaddum.mutabaqqiya = taqaddum.mutabaqqiya.saturating_sub(1);
    }
    hala.anshur();
}

/// Translates one string, with its bounded retries and its backoff.
///
/// `muhawala_bidaya` is how many attempts this string has already consumed —
/// zero for a fresh string, one for a string arriving from a failed or
/// partially failed multi-string request — and when it is nonzero the original
/// [`NassMahmi`] has already answered its one request, so every attempt here
/// re-protects from the source. `infaq_bidaya` is what those earlier attempts
/// settled as costing, so the final journal line carries the string's whole
/// price rather than its last attempt's.
///
/// Failures that [`KhataTarjama::yuqif_aljawla`] classifies as run-stopping
/// stop the run — including on the *last* attempt, where the lazier shape
/// "just fail the string" would keep the other workers dispatching against a
/// ceiling already known to be reached.
async fn adi_band<M>(
    muzawwid: &M,
    band: BandMuallaq<'_>,
    hala: &HalatJawla<'_>,
    khiyarat: &KhiyaratJawla,
    ism_muzawwid: &str,
    muhawala_bidaya: u32,
    infaq_bidaya: u64,
) where
    M: Muzawwid + ?Sized + Sync,
{
    let BandMuallaq { id, mahmi, ahruf, mudkhal } = band;
    let mut mahmi_hali = (muhawala_bidaya == 0).then_some(mahmi);
    let mut infaq = infaq_bidaya;
    let aqsa = khiyarat.aqsa_muhawalat.max(1);
    let mut muhawala = muhawala_bidaya;
    let mut akhir: Option<KhataTarjama> = None;

    while muhawala < aqsa {
        if hala.mutawaqqif() {
            return;
        }
        if muhawala > muhawala_bidaya {
            tokio::time::sleep(muddat_tarajua(khiyarat, muhawala)).await;
        }

        let mahmi = match mahmi_hali.take() {
            Some(mahmi) => mahmi,
            None => match ihmi(&mudkhal.masdar, &mudkhal.nasq_masdar) {
                Ok(mahmi) => mahmi,
                Err(khata) => {
                    // Deterministic and pre-dispatch: protecting the same
                    // source fails the same way every time, so retrying it
                    // would be three identical refusals for the price of the
                    // log lines.
                    sajjil_fashal(hala, khiyarat, ism_muzawwid, id, mudkhal, &khata, infaq);
                    return;
                }
            },
        };

        let siyaq = SiyaqTalab::min_mudkhal(mudkhal);
        let talabat = [TalabTarjama { mahmi: &mahmi, talab: &siyaq }];
        let takdirat = [muzawwid.qudrat().qaddir_taklifa(ahruf)];

        match talab_wahid(muzawwid, &talabat, &takdirat, hala, khiyarat).await {
            RaddTalab::Mutawaqqif => return,
            RaddTalab::Fashal(khata) => {
                if khata.yuqif_aljawla() {
                    hala.awqif_aljawla(khata);
                    return;
                }
                akhir = Some(khata);
            }
            RaddTalab::Najah(azwaj) => match azwaj.into_iter().next() {
                Some((natija, mablagh)) => {
                    infaq = infaq.saturating_add(mablagh);
                    match istaridd(&mahmi, natija.matn()) {
                        Ok(mustaad) => {
                            sajjil_najah(
                                hala, khiyarat, ism_muzawwid, id, mudkhal, mustaad, &natija,
                                infaq,
                            );
                            return;
                        }
                        Err(khata) => {
                            if khata.yuqif_aljawla() {
                                hala.awqif_aljawla(khata);
                                return;
                            }
                            akhir = Some(khata);
                        }
                    }
                }
                None => {
                    akhir = Some(KhataTarjama::RaddGhayrMufassal {
                        muzawwid: ism_muzawwid.to_owned(),
                        radd: "an empty result set for a one-string request".to_owned(),
                    });
                }
            },
        }
        muhawala = muhawala.saturating_add(1);
    }

    if let Some(khata) = akhir {
        sajjil_fashal(hala, khiyarat, ism_muzawwid, id, mudkhal, &khata, infaq);
    }
}

/// Processes one provider-sized chunk, degrading to single strings on failure.
///
/// A multi-string request that fails, or whose reply fails verification for
/// *some* of its strings, is not a reason to fail all of them: the classic
/// batch pathology is one string with an odd placeholder poisoning its whole
/// chunk, run after run. So a failed chunk is re-dispatched one string at a
/// time, each with its own remaining retry budget and its own eventual reason
/// — the batch was an efficiency, and its failure returns to the shape that
/// tells the truth per string.
async fn adi_dufa<M>(
    muzawwid: &M,
    dufa: Vec<BandMuallaq<'_>>,
    hala: &HalatJawla<'_>,
    khiyarat: &KhiyaratJawla,
    ism_muzawwid: &str,
) where
    M: Muzawwid + ?Sized + Sync,
{
    if hala.mutawaqqif() {
        return;
    }
    if dufa.len() <= 1 {
        if let Some(band) = dufa.into_iter().next() {
            adi_band(muzawwid, band, hala, khiyarat, ism_muzawwid, 0, 0).await;
        }
        return;
    }

    // Built up front and held for the whole request, because `TalabTarjama`
    // borrows: a context constructed inside the `map` would be dropped before
    // the borrow it hands out is used.
    let siyaqat: Vec<SiyaqTalab> =
        dufa.iter().map(|band| SiyaqTalab::min_mudkhal(band.mudkhal)).collect();
    let talabat: Vec<TalabTarjama<'_>> = dufa
        .iter()
        .zip(siyaqat.iter())
        .map(|(band, siyaq)| TalabTarjama { mahmi: &band.mahmi, talab: siyaq })
        .collect();
    let takdirat: Vec<u64> =
        dufa.iter().map(|band| muzawwid.qudrat().qaddir_taklifa(band.ahruf)).collect();

    match talab_wahid(muzawwid, &talabat, &takdirat, hala, khiyarat).await {
        RaddTalab::Mutawaqqif => (),
        RaddTalab::Fashal(khata) => {
            drop(talabat);
            if khata.yuqif_aljawla() {
                hala.awqif_aljawla(khata);
                return;
            }
            tracing::debug!(
                muzawwid = %ism_muzawwid,
                adad = dufa.len(),
                sabab = %khata,
                "فشلت دفعة كاملة؛ إعادة الإرسال نصًّا نصًّا"
            );
            for band in dufa {
                if hala.mutawaqqif() {
                    return;
                }
                if khiyarat.aqsa_muhawalat <= 1 {
                    // The chunk consumed the whole attempt budget; the batch's
                    // failure is each string's failure, named.
                    sajjil_fashal(
                        hala, khiyarat, ism_muzawwid, band.id, band.mudkhal, &khata, 0,
                    );
                } else {
                    adi_band(muzawwid, band, hala, khiyarat, ism_muzawwid, 1, 0).await;
                }
            }
        }
        RaddTalab::Najah(azwaj) => {
            drop(talabat);
            for (band, (natija, mablagh)) in dufa.into_iter().zip(azwaj) {
                if hala.mutawaqqif() {
                    return;
                }
                match istaridd(&band.mahmi, natija.matn()) {
                    Ok(mustaad) => {
                        sajjil_najah(
                            hala,
                            khiyarat,
                            ism_muzawwid,
                            band.id,
                            band.mudkhal,
                            mustaad,
                            &natija,
                            mablagh,
                        );
                    }
                    Err(khata) => {
                        if khata.yuqif_aljawla() {
                            hala.awqif_aljawla(khata);
                            return;
                        }
                        if khiyarat.aqsa_muhawalat <= 1 {
                            sajjil_fashal(
                                hala,
                                khiyarat,
                                ism_muzawwid,
                                band.id,
                                band.mudkhal,
                                &khata,
                                mablagh,
                            );
                        } else {
                            // This string's reply failed verification inside a
                            // chunk that otherwise succeeded. Its protected
                            // text has answered its one request, so the retry
                            // starts at attempt one and re-protects — and it
                            // carries the money the failed reply cost.
                            adi_band(
                                muzawwid, band, hala, khiyarat, ism_muzawwid, 1, mablagh,
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }
}

/// Runs one batch: every eligible string in the table, against one provider,
/// under one budget, committed as it lands.
///
/// The run's contract, in the order things happen:
///
/// 1. **The journal opens first.** Its recorded outcomes are the checkpoint:
///    every string it already answers is skipped, and its recorded spend seeds
///    the ledger, so the ceiling is cumulative across resumes of the same
///    project. A journal that cannot be *read* stops the run before anything
///    is spent — a run blind to prior spend cannot honour a cumulative
///    ceiling.
/// 2. **Eligibility is decided per string, and every skip is counted by
///    reason:** already answered, frozen, already translated, or empty.
///    Frozen strings and already-translated strings are
///    never machine-retranslated by a bulk run — that is the freeze's whole
///    meaning.
/// 3. **Every eligible string is protected by [`crate::hima::ihmi`]** before
///    anything else; a string protection refuses is failed on the spot, with
///    its [`AlamJawda::NasqMaksur`] flag, and never reaches a provider.
/// 4. **Chunks are sized from the provider's own [`QudratMuzawwid`]**, never
///    from a constant, and dispatched with at most [`KhiyaratJawla::tawazi`]
///    requests in flight.
/// 5. **Each result — success or failure, with reason — is appended to the
///    journal the moment it exists**, so a crash, a network death or a user
///    stop costs at most the requests literally in flight.
///
/// Returns a report rather than a `Result`, because a stopped run is not a
/// void run: everything committed before the stop is committed, the progress
/// is real, and [`TaqreerJawla::tawaqquf`] carries what stopped it. Callers
/// resume by calling this again with the same project directory; the journal
/// does the rest.
///
/// `nashir`, when given, receives a fresh [`TaqaddumJawla`] after every
/// change, which is what the workshop's progress panel watches.
///
/// The table is borrowed immutably: the run writes the *journal*, and the
/// table is updated afterwards by [`tabbiq_sijill`] — from the journal, so
/// that what the table says and what the checkpoint says can never disagree
/// about which of them is the record.
pub async fn shaghghil_jawla<M>(
    muzawwid: &M,
    madakhil: &[MudkhalNass],
    mujallad_mashru: &Path,
    khiyarat: &KhiyaratJawla,
    nashir: Option<&tokio::sync::watch::Sender<TaqaddumJawla>>,
) -> TaqreerJawla
where
    M: Muzawwid + ?Sized + Sync,
{
    let mut sijill = match SijillJawla::iftah(mujallad_mashru) {
        Ok(sijill) => sijill,
        Err(khata) => {
            return TaqreerJawla {
                taqaddum: TaqaddumJawla {
                    saqf: khiyarat.saqf_takalif,
                    ..TaqaddumJawla::default()
                },
                tawaqquf: Some(khata),
                sijill_talif: 0,
            };
        }
    };
    let sijill_talif = sijill.talifa();
    let munfaq_sabiq = sijill.munfaq_sabiq();
    let ism_muzawwid = muzawwid.ism();
    let qudrat = muzawwid.qudrat();

    // A paid run with no ceiling is a legitimate thing to ask for and an easy
    // thing to arrive at by accident: `saqf_takalif` is an `Option` whose
    // `None` reads as "unset" at every call site and means "unlimited" here,
    // so a provider whose budget setting was simply never filled in reaches
    // this line indistinguishable from one whose owner meant it. Said out
    // loud, once, at the top of the run — the log is where a contributor
    // looking at an unexpected bill goes first, and a silence there is a
    // question this product could have answered and did not.
    if khiyarat.saqf_takalif.is_none() && qudrat.taklifa.madfu() {
        tracing::warn!(
            muzawwid = %ism_muzawwid,
            "جولة مدفوعة بلا سقف تكلفة؛ لن تتوقّف عند أي مبلغ"
        );
    }

    let mut mutakhattaha = TafsilTakhatti::default();
    let mut fashila: Vec<(NassId, String)> = Vec::new();
    let mut bunud: Vec<BandMuallaq<'_>> = Vec::new();

    for mudkhal in madakhil {
        if sijill.ajaba(mudkhal.id) {
            mutakhattaha.fi_alsijill = mutakhattaha.fi_alsijill.saturating_add(1);
            continue;
        }
        if mudkhal.muraja.mujammad() {
            mutakhattaha.mujammada = mutakhattaha.mujammada.saturating_add(1);
            continue;
        }
        if mudkhal.hadaf.is_some() {
            mutakhattaha.mutarjama_musbaqan =
                mutakhattaha.mutarjama_musbaqan.saturating_add(1);
            continue;
        }
        if mudkhal.masdar.trim().is_empty() {
            mutakhattaha.farigha = mutakhattaha.farigha.saturating_add(1);
            continue;
        }
        match ihmi(&mudkhal.masdar, &mudkhal.nasq_masdar) {
            Ok(mahmi) => {
                let ahruf = tul_u64(mahmi.matn().chars().count());
                bunud.push(BandMuallaq { id: mudkhal.id, mahmi, ahruf, mudkhal });
            }
            Err(khata) => {
                // A string protection refuses never reaches a provider: it is
                // failed here, journaled here, and carries its broken-markup
                // flag from here. Free of charge — nothing was dispatched.
                let qayd = qayd_fashal(
                    mudkhal.id,
                    ism_muzawwid,
                    0,
                    khiyarat.lahza,
                    &khata,
                    &mudkhal.dharrat(),
                );
                if let Err(khata_sijill) = sijill.sajjil(qayd) {
                    return TaqreerJawla {
                        taqaddum: TaqaddumJawla {
                            mutarjama: 0,
                            fashila,
                            mutakhattaha,
                            mutabaqqiya: bunud.len(),
                            munfaq: munfaq_sabiq,
                            saqf: khiyarat.saqf_takalif,
                        },
                        tawaqquf: Some(khata_sijill),
                        sijill_talif,
                    };
                }
                fashila.push((mudkhal.id, khata.to_string()));
            }
        }
    }

    let hala = HalatJawla {
        daftar: Mutex::new(DaftarTakalif::min_sabiq(munfaq_sabiq)),
        sijill: Mutex::new(sijill),
        taqaddum: Mutex::new(TaqaddumJawla {
            mutarjama: 0,
            fashila,
            mutakhattaha,
            mutabaqqiya: bunud.len(),
            munfaq: munfaq_sabiq,
            saqf: khiyarat.saqf_takalif,
        }),
        tawaqquf: Mutex::new(None),
        awqif: AtomicBool::new(false),
        hadd: hadd_muadal(&qudrat),
        nashir,
    };
    hala.anshur();

    let dufaat = qassim_dufaat(bunud, &qudrat);
    futures::stream::iter(dufaat)
        .for_each_concurrent(khiyarat.tawazi.max(1), |dufa| {
            adi_dufa(muzawwid, dufa, &hala, khiyarat, ism_muzawwid)
        })
        .await;

    let HalatJawla { taqaddum, tawaqquf, .. } = hala;
    TaqreerJawla {
        taqaddum: taqaddum.into_inner(),
        tawaqquf: tawaqquf.into_inner(),
        sijill_talif,
    }
}

/// What folding a journal into a table did, number by number.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaqreerTatbiq {
    /// Entries that received a journaled translation.
    pub mutabbaqa: usize,
    /// Entries whose journaled failure had its flags recorded.
    pub fashila_muallama: usize,
    /// Entries holding a journaled translation that was **not** applied,
    /// because a human's newer work was in the way.
    pub mahmiya: usize,
    /// Journal outcomes whose string is no longer in the table — a
    /// re-extraction removed it between the run and the fold. Counted rather
    /// than erred on: the journal is history and history outliving its subject
    /// is normal.
    pub bila_mudkhal: usize,
}

/// Folds a run's journal into the project's entries.
///
/// The second half of the run's commit story: [`shaghghil_jawla`] writes the
/// journal, this writes the table *from* the journal, and because the fold is
/// pure — everything it sets comes off a journal line — running it twice, or
/// after a crash, or on a machine that only received the project directory,
/// produces the same table.
///
/// What it will not do:
///
/// - **Overwrite a human.** A journaled translation lands only where the entry
///   is untranslated or where the existing translation is itself unreviewed
///   machine output ([`TareeqaTarjama::AaliyaFaqat`]); a frozen entry
///   (`SijillMuraja::mujammad`) is never touched. A run's journal can be
///   folded days later, and a contributor who hand-translated a string in
///   between must win — the alternative destroys exactly the work this
///   product exists to protect.
/// - **Mint review state.** The live review record is advanced through
///   [`SijillMuraja::sajjil_aali`] on the entry's own record — the one
///   legitimate recording method — so an
///   existing record keeps its history and gains a transition rather than
///   being replaced by the journal's snapshot. The snapshot in the journal
///   stays what it is: the run's own record, for resume and audit.
///
/// Failed strings do not change workflow state — they stay untranslated — but
/// their flags, including [`AlamJawda::NasqMaksur`] for protection refusals,
/// are recorded on the entry so the review console can surface them.
///
/// `waqt` is the fold's moment as RFC 3339, caller-supplied like every
/// timestamp in this product, stamped as `akhir_tabdeel` on entries the fold
/// changed.
pub fn tabbiq_sijill(
    madakhil: &mut [MudkhalNass],
    sijill: &SijillJawla,
    waqt: &str,
) -> TaqreerTatbiq {
    let mut taqreer = TaqreerTatbiq::default();
    let mut mawjuda = 0_usize;

    for mudkhal in madakhil.iter_mut() {
        let Some(qayd) = sijill.qayd(mudkhal.id) else { continue };
        mawjuda = mawjuda.saturating_add(1);

        match &qayd.hasila {
            HasilatNass::Tarjumat { hadaf, nasq, alamat, .. } => {
                let aali_qadeem =
                    matches!(mudkhal.tareeqa, Some(TareeqaTarjama::AaliyaFaqat));
                let yuktab = mudkhal.muraja.qabil_lil_kitaba_aliyan()
                    && (mudkhal.hadaf.is_none() || aali_qadeem);
                if !yuktab {
                    taqreer.mahmiya = taqreer.mahmiya.saturating_add(1);
                    continue;
                }
                mudkhal.hadaf = Some(hadaf.clone());
                mudkhal.nasq_hadaf.clone_from(nasq);
                // Recorded through the one legitimate method, which lands in
                // `TarjamaAaliya` with no author. A machine translation cannot
                // reach a human state here or anywhere else.
                // The journal's own moment, not the fold's: the translation
                // happened when the provider answered, and a resumed run folding
                // yesterday's journal must record yesterday.
                mudkhal.muraja.sajjil_aali(qayd.lahza);
                mudkhal.tareeqa = Some(TareeqaTarjama::AaliyaFaqat);
                mudkhal.muzawwid = Some(qayd.muzawwid.clone());
                // No person made this change, and recording one would put a
                // name on text its owner never saw.
                mudkhal.muharrir = None;
                mudkhal.akhir_tabdeel = Some(waqt.to_owned());
                thabbit_alamat(mudkhal, alamat.clone());

                taqreer.mutabbaqa = taqreer.mutabbaqa.saturating_add(1);
            }
            HasilatNass::Fashilat { alamat, .. } => {
                thabbit_alamat(mudkhal, alamat.clone());
                taqreer.fashila_muallama = taqreer.fashila_muallama.saturating_add(1);
            }
        }
    }

    taqreer.bila_mudkhal = sijill.quyud().len().saturating_sub(mawjuda);
    taqreer
}

/// Every provider-reported confidence score the journal holds, by string.
///
/// The provider's score is journal data: it exists at the moment of
/// translation and nowhere on the entry afterwards, so a later flag recompute
/// — [`crate::alamat::ihsib_mashru`] — cannot re-judge confidence without
/// being handed the scores back. This is that hand-back. Strings whose
/// provider reported nothing (`maqisa` false at run time, journaled as an
/// absent score) are simply not in the map, which downstream reads as "no
/// score", never as "score of zero".
#[must_use]
pub fn thiqat_min_sijill(sijill: &SijillJawla) -> BTreeMap<NassId, ThiqaMublagha> {
    let mut thiqat = BTreeMap::new();
    for (id, qayd) in sijill.quyud() {
        if let HasilatNass::Tarjumat { thiqa: Some(qeema), .. } = &qayd.hasila {
            let _ = thiqat.insert(*id, ThiqaMublagha { qeema: *qeema, maqisa: true });
        }
    }
    thiqat
}

