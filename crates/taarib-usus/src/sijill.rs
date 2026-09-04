//! السجلّ — logging and diagnostics.
//!
//! One event pipeline serves three consumers with different needs:
//!
//! * a **JSON-lines file**, rotated daily and pruned by age and total size,
//!   which is what a diagnostics bundle carries;
//! * a **live ring buffer** the Diagnostics screen reads without touching the
//!   disk, so a user can watch what is happening while it happens;
//! * a **human line on stderr** when a console is attached, for the injected
//!   libraries running inside a game where nothing else is watching.
//!
//! Redaction happens **at the layer**, not at the call site. A call site that
//! has to remember to redact will eventually forget, and the one it forgets
//! will be the credential. Field names that look like secrets, values that look
//! like API keys, and the user's own home directory are rewritten on their way
//! into every one of the three consumers.
//!
//! **The message body is covered too**, not only the structured fields. A key
//! interpolated into a `tracing!` format string is the same disclosure as a key
//! in a field, and it is the easier of the two to write by accident.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt as _};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt as _;

use crate::idadat::MustawaSijill;
use crate::khata::{Khata, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use crate::khata_min;
use crate::masarat::{Masarat, insha_mujallad};

/// How many events the live buffer keeps. Roughly a session's worth of activity
/// at the default level, and bounded so a runaway loop cannot exhaust memory.
const SIAT_HALQA: usize = 4096;

/// Field names whose values are never recorded, whatever they contain.
///
/// Matched as substrings of the lowercased name, which is why the list is
/// shorter than the set of names it covers: `key` also catches `api_key`, and
/// `sirr` also catches `kalimat_sirr` and the bare `sirr` argument the Studio's
/// `khzin_itimad_muzawwid` command takes — the name a real credential travels
/// under in this codebase, and the one the earlier `kalimat_sirr` entry missed.
const HUQUL_SIRRIYA: [&str; 9] = [
    "miftah",
    "key",
    "token",
    "secret",
    "sirr",
    "password",
    "authorization",
    "cookie",
    "credential",
];

/// Credential prefixes, matched at the start of a run or of an `=`/`:`
/// separated part of one.
///
/// Anchored rather than searched for anywhere in the value, because the
/// alternative redacts ordinary English: `task-`, `disk-` and `risk-` all
/// contain `sk-`. `sk-` covers Anthropic's `sk-ant-` as well, and `aiza` is
/// Google's `AIza`, lowercased with the value.
const BIDAYAT_SIRR: [&str; 5] = ["sk-", "ghp_", "github_pat_", "xoxb-", "aiza"];

/// HTTP auth schemes whose credential has no shape of its own: it is identified
/// by the word in front of it and by nothing about itself.
const MUQADDIMAT_TAWTHIQ: [&str; 2] = ["bearer", "basic"];

/// What replaces a redacted value.
const MAHJOOB: &str = "«محجوب»";

/// One recorded event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
pub struct Hadath {
    /// RFC 3339 timestamp.
    pub waqt: String,
    /// Level name.
    pub mustawa: String,
    /// The module that emitted it.
    pub hadaf: String,
    /// The event's message.
    pub risala: String,
    /// The enclosing spans, outermost first — which is how an event is
    /// attributed to a game, a patch, and a stage.
    pub nitaq: Vec<String>,
    /// Structured fields, already redacted.
    pub huqul: BTreeMap<String, String>,
}

static HALQA: OnceLock<Arc<Mutex<VecDeque<Hadath>>>> = OnceLock::new();

fn halqa() -> &'static Arc<Mutex<VecDeque<Hadath>>> {
    HALQA.get_or_init(|| Arc::new(Mutex::new(VecDeque::with_capacity(SIAT_HALQA))))
}

/// Keeps the background log writer alive.
///
/// Dropping it flushes and stops the writer thread, so it is held for the
/// lifetime of the process and never discarded with `let _ =`.
pub struct HarisSijill {
    _haris: Option<WorkerGuard>,
    masar: PathBuf,
    bi_malaf: bool,
}

impl std::fmt::Debug for HarisSijill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarisSijill").field("masar", &self.masar).finish_non_exhaustive()
    }
}

impl HarisSijill {
    /// The directory the current log is being written into.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// Whether a log file is actually being written.
    ///
    /// `false` when the directory could not be created: diagnostics still
    /// reach the live buffer and the Diagnostics screen, and the bundle a user
    /// exports will say so rather than appear empty for no stated reason.
    #[must_use]
    pub const fn bi_malaf(&self) -> bool {
        self.bi_malaf
    }
}

/// Starts the diagnostics pipeline for the desktop application.
///
/// # Errors
///
/// Fails when the log directory cannot be created, or when a subscriber has
/// already been installed in this process.
pub fn hayyi(masarat: &Masarat, mustawa: MustawaSijill) -> Natija<HarisSijill> {
    hayyi_fi_masar(&masarat.sijillat(), "taarib", mustawa, true)
}

/// Starts the diagnostics pipeline for an injected library, writing into the
/// game's own directory under a name that says which game it belongs to.
///
/// # Errors
///
/// As [`hayyi`].
pub fn hayyi_haqn(mujallad: &Path, ism: &str, mustawa: MustawaSijill) -> Natija<HarisSijill> {
    hayyi_fi_masar(mujallad, ism, mustawa, false)
}

fn hayyi_fi_masar(
    mujallad: &Path,
    ism: &str,
    mustawa: MustawaSijill,
    maa_shasha: bool,
) -> Natija<HarisSijill> {
    // A log directory that cannot be made is not a reason to have no
    // diagnostics: the ring buffer and the console layer need no file, and the
    // one machine that most needs its diagnostics read is the machine whose
    // disk is refusing writes.
    let bi_malaf = insha_mujallad(mujallad).is_ok();

    // `rolling::daily` opens eagerly and has no error channel, so it is only
    // built once the directory is known to exist.
    let katib_haris = bi_malaf.then(|| {
        let mudawwir = tracing_appender::rolling::daily(mujallad, format!("{ism}.jsonl"));
        tracing_appender::non_blocking(mudawwir)
    });
    let (katib, haris) = match katib_haris {
        Some((katib, haris)) => (Some(katib), Some(haris)),
        None => (None, None),
    };

    let murashih = tracing_subscriber::EnvFilter::try_new(mustawa.tawjeeh()).map_err(|q| {
        Khata::min_tafsir(&KhataSijill::MurashihGhayrSalih { tafsil: q.to_string() })
    })?;

    let tabaqa = TabaqatTaarib { katib: katib.map(Mutex::new), maa_shasha };

    tracing_subscriber::registry()
        .with(murashih)
        .with(tabaqa)
        .try_init()
        .map_err(|q| Khata::min_tafsir(&KhataSijill::SabaqTanseeb { tafsil: q.to_string() }))?;

    if !bi_malaf {
        tracing::warn!(
            mujallad = %mujallad.display(),
            "the log directory could not be created; diagnostics are in memory only"
        );
    }
    Ok(HarisSijill { _haris: haris, masar: mujallad.to_path_buf(), bi_malaf })
}

/// The most recent events, newest last, for the Diagnostics screen.
#[must_use]
pub fn ahdath_akhira(adad: usize) -> Vec<Hadath> {
    let halqa = halqa().lock();
    let bidaya = halqa.len().saturating_sub(adad);
    halqa.iter().skip(bidaya).cloned().collect()
}

/// Empties the live buffer, for a user who wants to watch one operation on its
/// own without the noise before it.
pub fn nazzif_halqa() {
    halqa().lock().clear();
}

/// Deletes log files older than `ayyam` days, then, if the directory still
/// exceeds `hadd_mb`, deletes the oldest remaining files until it does not.
///
/// Both limits matter: age keeps a machine that is used daily tidy, and size
/// keeps a machine that ran one enormous extraction from filling its disk.
///
/// # Errors
///
/// Fails when the directory cannot be listed. A file that cannot be deleted is
/// reported through the log and skipped, because failing a cleanup is never
/// worth failing the operation that triggered it.
pub fn nazzif_sijillat(mujallad: &Path, ayyam: u32, hadd_mb: u64) {
    let Ok(qaima) = std::fs::read_dir(mujallad) else {
        return;
    };

    let alaan = std::time::SystemTime::now();
    let hadd_umr = std::time::Duration::from_secs(u64::from(ayyam) * 24 * 60 * 60);
    let mut mawjud: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();

    for madkhal in qaima.flatten() {
        let masar = madkhal.path();
        if !masar.is_file() {
            continue;
        }
        let Ok(bayanat) = madkhal.metadata() else { continue };
        let waqt = bayanat.modified().unwrap_or(alaan);
        let hajm = bayanat.len();

        if alaan.duration_since(waqt).unwrap_or_default() > hadd_umr {
            if let Err(khata) = std::fs::remove_file(&masar) {
                tracing::warn!(
                    masar = %masar.display(),
                    sabab = %khata.kind_name(),
                    "could not delete an expired log file"
                );
            }
            continue;
        }
        mawjud.push((masar, waqt, hajm));
    }

    let mut majmu: u64 = mawjud.iter().map(|(_, _, h)| *h).sum();
    let hadd = hadd_mb.saturating_mul(1024 * 1024);
    if majmu <= hadd {
        return;
    }

    mawjud.sort_by_key(|(_, waqt, _)| *waqt);
    for (masar, _, hajm) in mawjud {
        if majmu <= hadd {
            break;
        }
        if std::fs::remove_file(&masar).is_ok() {
            majmu = majmu.saturating_sub(hajm);
        }
    }
}

/// Names an I/O error kind without formatting the whole error, so a cleanup
/// warning cannot leak a path it was not asked to log.
trait IsmNaw {
    fn kind_name(&self) -> String;
}

impl IsmNaw for std::io::Error {
    fn kind_name(&self) -> String {
        format!("{:?}", self.kind())
    }
}

/// Rewrites anything that looks like a credential or a personal path.
///
/// Three rules, applied in order:
///
/// 1. a field whose *name* looks like a secret is replaced entirely;
/// 2. a value carrying anything credential-shaped is replaced entirely;
/// 3. the user's home directory is rewritten to `~`, so a log can be shared
///    without disclosing a real name.
#[must_use]
pub fn hajjib(ism: &str, qeema: &str) -> String {
    let ism_saghir = ism.to_ascii_lowercase();
    if HUQUL_SIRRIYA.iter().any(|s| ism_saghir.contains(s)) {
        return MAHJOOB.to_owned();
    }
    hajjib_qeema(qeema)
}

/// Rules 2 and 3 of [`hajjib`], for a value whose field name is already known
/// to be innocent.
///
/// The whole value goes, not the offending run: a field value *is* the thing
/// being recorded, so there is no surrounding sentence to save.
fn hajjib_qeema(qeema: &str) -> String {
    let mut sabiq = "";
    for juz in qeema.split_whitespace() {
        if yushbih_sirr(juz) || baad_muqaddima(sabiq) {
            return MAHJOOB.to_owned();
        }
        sabiq = juz;
    }
    hajjib_manzil(qeema)
}

/// Redacts the credential-shaped runs inside a message, keeping the prose.
///
/// A message is a sentence with values interpolated into it, so [`hajjib_qeema`]
/// applied whole would throw away *what happened* along with the secret. Each
/// whitespace-delimited run is judged on its own here and only the runs that
/// lose are replaced, which leaves a line that still says what the code was
/// doing when it nearly leaked a key.
fn hajjib_risala(qeema: &str) -> String {
    let mut makhraj = String::with_capacity(qeema.len());
    let mut sabiq = "";
    for juz in qeema.split_inclusive(char::is_whitespace) {
        let matn = juz.trim_end_matches(char::is_whitespace);
        let fasil = juz.get(matn.len()..).unwrap_or_default();
        if !matn.is_empty() {
            if yushbih_sirr(matn) || baad_muqaddima(sabiq) {
                makhraj.push_str(MAHJOOB);
            } else {
                makhraj.push_str(matn);
            }
            sabiq = matn;
        }
        makhraj.push_str(fasil);
    }
    hajjib_manzil(&makhraj)
}

/// Whether the run *before* this one is an auth scheme name, which makes this
/// one the credential whatever it looks like.
fn baad_muqaddima(sabiq: &str) -> bool {
    let mujarrad = sabiq.trim_end_matches([':', '=', ',']).to_ascii_lowercase();
    MUQADDIMAT_TAWTHIQ.contains(&mujarrad.as_str())
}

/// Whether one whitespace-delimited run looks like a credential.
///
/// Four shapes, each here because a provider this product talks to issues keys
/// in it:
///
/// * one of [`BIDAYAT_SIRR`] at the start of the run or of an `=`/`:` separated
///   part of it, so `miftah=sk-…` and `…?key=AIza…` both lose;
/// * `DeepL`'s Free key — a UUID with a `:fx` suffix, which is 39 characters and
///   matches neither length rule below. The Pro key is a *bare* UUID and is
///   deliberately not matched here: it is indistinguishable from the patch and
///   game identifiers this product's logs are full of, so the field-name rule is
///   what has to catch that one;
/// * exactly 32 hexadecimal characters, which is an Azure subscription key —
///   the Microsoft translator's credential, and short enough to slip under the
///   catch-all. *Exactly* 32, because this project renders its own blake3 and
///   SHA-256 hashes as 64, and redacting those would empty the diagnostics of
///   the identifiers that make them worth reading;
/// * any unbroken run of 40 or more key-shaped characters containing a digit,
///   which is the catch-all the three named shapes sit inside.
fn yushbih_sirr(juz: &str) -> bool {
    let saghir = juz.to_ascii_lowercase();

    if let Some(asas) = saghir.strip_suffix(":fx")
        && shakl_uuid(asas)
    {
        return true;
    }

    if saghir
        .split(['=', ':'])
        .any(|far| BIDAYAT_SIRR.iter().any(|bidaya| far.starts_with(bidaya)))
    {
        return true;
    }

    saghir
        .split(|h: char| !(h.is_ascii_alphanumeric() || h == '_' || h == '-'))
        .any(|mutawasil| {
            (mutawasil.len() == 32 && mutawasil.bytes().all(|b| b.is_ascii_hexdigit()))
                || (mutawasil.len() >= 40 && mutawasil.bytes().any(|b| b.is_ascii_digit()))
        })
}

/// Whether a run is a canonical hyphenated UUID — `8-4-4-4-12` hex digits.
fn shakl_uuid(juz: &str) -> bool {
    let mut ajzaa = juz.split('-');
    let mutabiq = [8_usize, 4, 4, 4, 12].into_iter().all(|tul| {
        ajzaa
            .next()
            .is_some_and(|far| far.len() == tul && far.bytes().all(|b| b.is_ascii_hexdigit()))
    });
    // `all` stops at the first mismatch, so the tail check only runs when every
    // group matched and the iterator is where it should be.
    mutabiq && ajzaa.next().is_none()
}

fn hajjib_manzil(qeema: &str) -> String {
    static MANZIL: OnceLock<Option<String>> = OnceLock::new();
    let manzil = MANZIL.get_or_init(|| {
        directories::BaseDirs::new().map(|q| q.home_dir().to_string_lossy().to_string())
    });
    match manzil {
        Some(m) if !m.is_empty() && qeema.contains(m.as_str()) => qeema.replace(m.as_str(), "~"),
        _ => qeema.to_owned(),
    }
}

struct TabaqatTaarib {
    katib: Option<Mutex<tracing_appender::non_blocking::NonBlocking>>,
    maa_shasha: bool,
}

impl std::fmt::Debug for TabaqatTaarib {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabaqatTaarib")
            .field("maa_shasha", &self.maa_shasha)
            .finish_non_exhaustive()
    }
}

impl<S> Layer<S> for TabaqatTaarib
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, hadath_khaam: &Event<'_>, siyaq: Context<'_, S>) {
        let mut jami = JamiHuqul::default();
        hadath_khaam.record(&mut jami);

        let mut nitaq = Vec::new();
        if let Some(mawjud) = siyaq.event_scope(hadath_khaam) {
            for daira in mawjud.from_root() {
                nitaq.push(daira.name().to_owned());
            }
        }

        let bayanat = hadath_khaam.metadata();
        let hadath = Hadath {
            waqt: jiff::Timestamp::now().to_string(),
            mustawa: bayanat.level().to_string(),
            hadaf: bayanat.target().to_owned(),
            risala: jami.risala,
            nitaq,
            huqul: jami.huqul,
        };

        if let Ok(satr) = serde_json::to_string(&hadath)
            && let Some(katib) = self.katib.as_ref() {
                let mut katib = katib.lock();
                let _ = katib.write_all(satr.as_bytes());
                let _ = katib.write_all(b"\n");
            }

        if self.maa_shasha {
            let mut satr = String::new();
            let _ = write!(satr, "{} {} {}", hadath.mustawa, hadath.hadaf, hadath.risala);
            for (miftah, qeema) in &hadath.huqul {
                let _ = write!(satr, " {miftah}={qeema}");
            }
            let mut khuruj = std::io::stderr().lock();
            let _ = writeln!(khuruj, "{satr}");
        }

        let mut halqa = halqa().lock();
        if halqa.len() == SIAT_HALQA {
            let _ = halqa.pop_front();
        }
        halqa.push_back(hadath);
    }
}

#[derive(Default)]
struct JamiHuqul {
    risala: String,
    huqul: BTreeMap<String, String>,
}

impl JamiHuqul {
    fn daa(&mut self, haql: &Field, qeema: &str) {
        if haql.name() == "message" {
            self.risala = hajjib_risala(qeema);
        } else {
            let _ = self.huqul.insert(haql.name().to_owned(), hajjib(haql.name(), qeema));
        }
    }
}

impl Visit for JamiHuqul {
    fn record_str(&mut self, haql: &Field, qeema: &str) {
        self.daa(haql, qeema);
    }

    fn record_debug(&mut self, haql: &Field, qeema: &dyn std::fmt::Debug) {
        self.daa(haql, &format!("{qeema:?}"));
    }

    fn record_i64(&mut self, haql: &Field, qeema: i64) {
        self.daa(haql, &qeema.to_string());
    }

    fn record_u64(&mut self, haql: &Field, qeema: u64) {
        self.daa(haql, &qeema.to_string());
    }

    fn record_f64(&mut self, haql: &Field, qeema: f64) {
        self.daa(haql, &qeema.to_string());
    }

    fn record_bool(&mut self, haql: &Field, qeema: bool) {
        self.daa(haql, &qeema.to_string());
    }

    fn record_error(&mut self, haql: &Field, qeema: &(dyn std::error::Error + 'static)) {
        self.daa(haql, &qeema.to_string());
    }
}

/// Failures of the diagnostics pipeline itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataSijill {
    /// The level filter could not be built.
    #[error("invalid log filter: {tafsil}")]
    MurashihGhayrSalih {
        /// What the filter parser reported.
        tafsil: String,
    },

    /// A subscriber was already installed in this process.
    #[error("a log subscriber is already installed: {tafsil}")]
    SabaqTanseeb {
        /// What the subscriber reported.
        tafsil: String,
    },
}

impl Tafsir for KhataSijill {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::SIJILL
                + match self {
                    Self::MurashihGhayrSalih { .. } => 0,
                    Self::SabaqTanseeb { .. } => 1,
                },
        )
    }

    fn arabi(&self) -> String {
        match self {
            Self::MurashihGhayrSalih { .. } => {
                "مستوى التسجيل المحدَّد غير صالح، وسيُستخدم المستوى الافتراضي.".to_owned()
            }
            Self::SabaqTanseeb { .. } => {
                "نظام التسجيل مُهيَّأ مسبقًا في هذه العملية.".to_owned()
            }
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MurashihGhayrSalih { .. } => {
                "The configured log level is not valid; the default will be used.".to_owned()
            }
            Self::SabaqTanseeb { .. } => {
                "Logging is already initialised in this process.".to_owned()
            }
        }
    }

    fn khutwa(&self) -> Khutwa {
        Khutwa::FathTashkhis
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MurashihGhayrSalih { tafsil } | Self::SabaqTanseeb { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataSijill);

#[cfg(test)]
mod ikhtibarat {
    use tracing_subscriber::layer::SubscriberExt as _;

    use super::{MAHJOOB, TabaqatTaarib, ahdath_akhira, hajjib, hajjib_risala, nazzif_halqa};

    /// A real Anthropic-shaped key: the prefix rule.
    const MIFTAH_SK: &str = "sk-ant-api03-7Qw3rTyU1oPa2SdF4gHj6KlZ8xCvB0nM5eR7tY9uI3oP1aS2dF";
    /// A real Azure/Microsoft translator key: thirty-two hexadecimal digits.
    const MIFTAH_MICROSOFT: &str = "0f9c1b7a4e2d6805f3a1c9b7e5d4028a";
    /// A `DeepL` Free key: a UUID with the suffix that picks the Free host.
    const MIFTAH_DEEPL: &str = "279a2e9b-0c14-4d7f-9b2a-6f1e8c3d5a70:fx";

    #[test]
    fn ism_al_haql_yahjib_al_qeema() {
        // `sirr` is the name the Studio's credential command actually uses, and
        // the one the denylist used to miss.
        assert_eq!(hajjib("sirr", "hunting-season"), MAHJOOB);
        assert_eq!(hajjib("kalimat_sirr", "hunting-season"), MAHJOOB);
        assert_eq!(hajjib("api_key", "hunting-season"), MAHJOOB);
        assert_eq!(hajjib("Authorization", "hunting-season"), MAHJOOB);
    }

    #[test]
    fn shakl_al_qeema_yahjibuha() {
        assert_eq!(hajjib("radd", MIFTAH_SK), MAHJOOB);
        assert_eq!(hajjib("radd", MIFTAH_MICROSOFT), MAHJOOB);
        assert_eq!(hajjib("radd", MIFTAH_DEEPL), MAHJOOB);
        assert_eq!(hajjib("radd", "Bearer 8xK2p"), MAHJOOB);
        assert_eq!(hajjib("unwan", "https://api.example.test/v1?key=AIzaSyB7n2Qd"), MAHJOOB);
    }

    #[test]
    fn al_nass_al_aadi_yanju() {
        // Every one of these contains `sk-`, and the old whole-value `contains`
        // rule replaced all three outright.
        for salim in ["task-list", "disk-cache", "risk-report"] {
            assert_eq!(hajjib("marhala", salim), salim);
        }
        // A bare UUID is this product's own identifier shape, not a credential.
        assert_eq!(
            hajjib("ruqaa", "279a2e9b-0c14-4d7f-9b2a-6f1e8c3d5a70"),
            "279a2e9b-0c14-4d7f-9b2a-6f1e8c3d5a70"
        );
    }

    #[test]
    fn matn_al_risala_yahjib_al_miftah_wahdah() {
        let risala = hajjib_risala(&format!("uploading with {MIFTAH_SK} to the forge"));
        assert!(!risala.contains(MIFTAH_SK), "the key survived: {risala}");
        assert_eq!(risala, format!("uploading with {MAHJOOB} to the forge"));

        // The scheme name is what identifies a bearer credential; nothing about
        // the credential itself does.
        let tarwisa = hajjib_risala("sent Authorization: Bearer 8xK2p to the endpoint");
        assert_eq!(tarwisa, format!("sent Authorization: Bearer {MAHJOOB} to the endpoint"));
    }

    #[test]
    fn al_tabaqa_tahjib_fi_kull_mawqi() {
        // A unique marker, so a parallel test writing to the same global ring
        // buffer cannot be mistaken for this one's event.
        const ALAMA: &str = "kansuyub-9f31";

        nazzif_halqa();
        let tabaqa = TabaqatTaarib { katib: None, maa_shasha: false };
        let mushtarik = tracing_subscriber::registry().with(tabaqa);
        tracing::subscriber::with_default(mushtarik, || {
            tracing::info!(
                sirr = MIFTAH_SK,
                radd = MIFTAH_MICROSOFT,
                "{ALAMA} handshake finished with {MIFTAH_DEEPL}"
            );
        });

        let mut wusul = ahdath_akhira(64);
        wusul.retain(|hadath| hadath.risala.contains(ALAMA));
        assert_eq!(wusul.len(), 1, "the layer did not record the event it was handed");
        let Some(hadath) = wusul.first() else { return };

        // The field-name position.
        assert_eq!(hadath.huqul.get("sirr").map(String::as_str), Some(MAHJOOB));
        // The field-value position: an innocent name, a key-shaped value.
        assert_eq!(hadath.huqul.get("radd").map(String::as_str), Some(MAHJOOB));
        // The message-body position, which used to be recorded verbatim.
        assert!(!hadath.risala.contains(MIFTAH_DEEPL), "the key survived: {}", hadath.risala);
        assert!(hadath.risala.contains(ALAMA), "the prose did not survive: {}", hadath.risala);
    }
}
