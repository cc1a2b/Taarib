//! المزوّدون — every translation service behind one trait, and the three rules
//! none of them may break.
//!
//! Seven providers, one contract. A provider receives a [`NassMahmi`] — the
//! tokenised text from [`crate::hima`], which is the *only* type that can sit
//! in that argument position and which has no accessor to the raw source — a
//! [`SiyaqTalab`] of context, and nothing else. It returns a
//! [`NatijatTarjama`] or a [`KhataTarjama`]. Everything provider-specific —
//! endpoint, request shape, authentication header, pricing, rate limit — lives
//! inside the provider and leaks nowhere.
//!
//! ## Rule one: a credential cannot be printed
//!
//! [`Itimad`] hand-writes `Debug` and `Display` to emit `Itimad(<redacted>)`
//! and nothing else — no prefix, no length, no first-and-last hint, because
//! each of those is a real leak: a length distinguishes providers, a prefix
//! (`sk-ant-…`) identifies the account class, and a first-and-last pair cuts
//! brute-force space. The redaction is structural rather than disciplinary: an
//! error type that carried an [`Itimad`] *could not* print it, a `tracing`
//! call that interpolated one would log eleven fixed characters, and a future
//! `#[derive(Debug)]` on a struct holding one still redacts. The value itself
//! lives in the operating system's secret store through `keyring`, never in a
//! project file, never in configuration, and there is deliberately no `serde`
//! implementation in either direction — a `Serialize` impl is a config-file
//! leak waiting for a refactor, and a `Deserialize` impl is a public
//! constructor from anywhere.
//!
//! ## Rule two: nothing paid happens without a credential and a confirmation
//!
//! Every paid provider's constructor takes an [`Itimad`] and an [`IdhnInfaq`]
//! by value. There is no constructor without them, so "ask a provider with no
//! key to translate" is not a runtime error — it is a program that does not
//! compile. Two constructors in this module take neither, and each is fenced
//! so that it cannot be pointed at a paid service to skip the rule:
//! [`MuzawwidMuwafiqOpenAI::mahalli`] refuses any endpoint that is not
//! loopback, and [`MuzawwidGoogleMajjani::jadeed`] speaks one fixed public
//! endpoint that has no account behind it to bill — Google Translate's free web
//! endpoint, the one every Unity translation mod uses. It is the provider a
//! fresh installation translates with, and it is honest about what that buys:
//! sentence-level quality, undocumented rate limits, and a service that may
//! refuse a machine it has flagged ([`KhataTarjama::MajjaniMahjub`]). A keyed
//! provider the user configures always takes precedence over it.
//!
//! Cost is enforced before sending, not discovered from a bill:
//! [`TakalifJarya`] reserves an upper-bound estimate against the confirmed
//! ceiling, refuses with [`KhataTarjama::SaqfTakalif`] *at* the ceiling, and
//! settles the reservation down to the provider's own reported usage after the
//! reply. For the token-priced providers the output bound is exact — the
//! request's `max_tokens` is a hard cap the API enforces — and the input
//! estimate is deliberately generous; see [`taqdir_rumuz_min_bayt`].
//!
//! ## Rule three: a confidence is reported or it is absent
//!
//! [`NatijatTarjama`] carries `thiqa: Option<f32>` beside `maqisa: bool`,
//! mirroring Phase 11's `SatrMaqru` exactly: a number is carried only when the
//! provider actually produced one, and `maqisa` says whose number it is. The
//! constructor that sets a confidence demands a [`DalilThiqa`], which only a
//! capability declaring `yublighu_thiqa` can mint — so a provider whose API
//! reports nothing *cannot* claim a number, not by convention but by type.
//!
//! Where this module refused to synthesise one, by name:
//!
//! - **`DeepL`** returns translations and a detected source language. No
//!   confidence exists in the reply and none is invented.
//! - **Google Cloud Translation v3** returns `translatedText` and, when the
//!   source was auto-detected, a `detectedLanguageCode`. Detection confidence
//!   is not translation confidence and is not borrowed as one.
//! - **Microsoft Translator** returns `detectedLanguage.score` — again a
//!   *detection* score. Reusing it as translation confidence would be a
//!   number that reads like a measurement and is not one; refused.
//! - **The three LLM providers** ask the model to self-assess in the reply
//!   schema. When the model omits the field or writes `null`, the result
//!   carries [`None`] — no default, no estimate, and nothing derived from
//!   token probabilities this crate does not request.
//!
//! ## Structured output only
//!
//! Each LLM provider requests its reply as data — Anthropic through a forced
//! tool call, OpenAI-compatible endpoints through `response_format`, Gemini
//! through `responseSchema` — and parses it with [`crate::siyaq::hallil_radd`]
//! / [`hallil_qeema`]. A reply that is not the requested structure is
//! [`KhataTarjama::RaddGhayrMufassal`]. **No natural-language reply is ever
//! pattern-matched for a translation**: scraping is how "Sure! Here's your
//! Arabic:" ships inside a dialogue box.
//!
//! ## The wire, per provider
//!
//! | provider | endpoint | structured output | usage reported |
//! | --- | --- | --- | --- |
//! | Anthropic | `POST /v1/messages` | tool `input_schema` + `tool_choice {type:"tool"}` | `usage.input_tokens` / `output_tokens` |
//! | OpenAI-compatible | `POST {base}/v1/chat/completions` | `response_format: json_schema` (or `json_object` for older local servers) | `usage.prompt_tokens` / `completion_tokens` |
//! | Google Gemini | `POST /v1beta/models/{model}:generateContent` | `generationConfig.responseMimeType` + `responseSchema` | `usageMetadata.promptTokenCount` / `candidatesTokenCount` |
//! | `DeepL` | `POST /v2/translate` | translation API — the envelope is the structure | none; characters counted locally |
//! | Google Cloud Translation | `POST /v3/projects/{p}:translateText` | translation API | none; characters counted locally |
//! | Microsoft Translator | `POST /translate?api-version=3.0` | translation API | `X-metered-usage` header, billable characters |
//! | Google Translate, free web endpoint | `GET /translate_a/single?client=gtx&dt=t` | undocumented JSON array, read positionally | none; nothing is billed |
//!
//! ## Model identifiers and prices are data with a date on them
//!
//! Every default model identifier, price table and request-parameter note in
//! this module was checked against the providers' own published pages on
//! [`TAREEKH_ASAR`], and each carries that constant rather than the phrase "at
//! the time of writing" — which ages into a claim nobody can check. Providers
//! retire identifiers on sixty days' notice, deprecate request parameters
//! outright, and change prices without notice at all, so all three are
//! overridable configuration and none of them is a constant in the sense of
//! being permanent. The free web endpoint has no published page at all, so its
//! request and reply shapes were checked live instead, on
//! [`TAREEKH_ASAR_GOOGLE_MAJJANI`].
//!
//! Two invariants make the staleness safe rather than merely visible. A price
//! table prices an identifier it does not recognise at its **highest** entry,
//! because the number guards a spending ceiling and the safe direction for an
//! unknown is over. A request parameter a table does not recognise is
//! **omitted**, because omitting one the model would have taken costs a little
//! quality and sending one it refuses costs the entire run.
//!
//! Rate limiting is enforced locally with `governor` before each request, and
//! a `429` is retried after the provider's own `Retry-After` when it sent one
//! — a client that retries on its own schedule against a server that stated
//! its schedule is asking to be banned. When backoff does not clear it, the
//! run stops with [`KhataTarjama::HaddMuadal`] rather than burning the
//! remaining strings against a closed door.

use std::fmt;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::hima::NassMahmi;
use crate::khata::{KhataTarjama, tul_u64};
use crate::siyaq::{self, RaddMufassal, SiyaqTalab, hallil_qeema, hallil_radd};

/// The service name every credential is filed under in the platform store.
///
/// One service, one account per provider, so the operating system's own
/// credential viewer shows a user exactly which keys this product holds and
/// lets them revoke any of them without touching Taarib.
pub const KHIDMAT_KHAZINA: &str = "taarib-tarjama";

/// The longest credential the store will accept, in bytes.
///
/// Four KiB. Real API keys are under two hundred bytes; a Google Cloud OAuth
/// access token can reach two KiB. Anything past four is not a credential —
/// it is a pasted document, and storing it would put that document in the
/// keychain under a name that says it is a key.
pub const AQSA_TUL_ITIMAD: usize = 4096;

/// How many times one request is attempted before its failure is final.
///
/// Four: the original try and three retries. Enough to ride out a rate-limit
/// window and a transient 5xx, few enough that a hard-down provider fails the
/// run in under two minutes instead of grinding on it for an hour.
pub const ADAD_MUHAWALAT: u32 = 4;

/// The first backoff step, in milliseconds, doubling per retry.
pub const ASAS_TARAJU_MILLI: u64 = 500;

/// The longest a server-requested `Retry-After` is honoured, in seconds.
///
/// One hundred and twenty. A provider asking for more than two minutes is not
/// rate limiting a burst, it is signalling sustained saturation — and holding
/// a batch open, silent, for longer than that is worse for the contributor
/// than stopping with a resumable [`KhataTarjama::HaddMuadal`] that says when
/// to come back.
pub const AQSA_TARAJU_THAWANI: u64 = 120;

/// Whole-request timeout for every provider call.
pub const MUHLAT_TALAB: Duration = Duration::from_secs(90);

/// Connect timeout for every provider call.
pub const MUHLAT_ITTISAL: Duration = Duration::from_secs(10);

/// The bytes-per-token divisor behind the input cost estimate.
///
/// Three. English prose averages a little over four bytes per token on every
/// tokenizer the LLM providers here use, and Arabic — more bytes per
/// character, more tokens per word — lands between three and four. Dividing
/// by three therefore *over*-estimates tokens for the text this product sends,
/// which is the correct direction for a number whose only job is to reserve
/// budget: see [`TakalifJarya`]. The reservation is corrected to the
/// provider's own reported count the moment the reply arrives.
pub const HADD_BAYT_LIL_RAMZ: u64 = 3;

/// The day every price, model identifier and dialect note in this module was
/// last checked against the providers' own published pages.
///
/// Written down because the alternative is "at the time of writing", which
/// ages into a claim nobody can check. Providers retire model identifiers on
/// sixty days' notice and change prices without any, so this date is the one
/// fact that tells a reader whether the tables below are still worth
/// believing — and a release far past it should re-check them rather than
/// assume.
pub const TAREEKH_ASAR: &str = "2026-09-05";

// ---------------------------------------------------------------------------
// الاعتماد — the credential
// ---------------------------------------------------------------------------

/// A provider credential, unprintable by construction.
///
/// Obtainable two ways only: [`Itimad::khazzin`], which writes the value into
/// the operating system's secret store and hands back the stored value, and
/// [`Itimad::min_khazina`], which reads a previously stored one. There is no
/// constructor from a bare string that does not pass through the store, no
/// `Clone`, and no `serde` in either direction — a `Serialize` impl would be
/// one refactor away from a credential in a config file, and a `Deserialize`
/// impl is a public constructor from any file that can name the type. This is
/// the same shape as `taarib-tabaqa`'s `Iqrar`: the rule lives in what the
/// type makes impossible, not in what callers remember.
///
/// `Debug` and `Display` are hand-written and both print `Itimad(<redacted>)`
/// — never a length, never a prefix, never first-and-last characters, because
/// all three genuinely leak (a prefix names the provider and account class, a
/// length fingerprints the key kind, an ends-pair shrinks search space). The
/// redaction being on the type means an error, a log line, or a diagnostics
/// bundle that somehow ends up holding one *cannot* disclose it.
pub struct Itimad {
    /// The secret. Private, and nothing outside this module can reach it;
    /// inside it, only the header-building code does.
    qeema: String,
}

impl Itimad {
    /// Stores a credential in the platform secret store and returns it.
    ///
    /// The single entry point for a key a user typed. The value goes into the
    /// operating system's store — Keychain, Windows Credential Manager, the
    /// Secret Service — under [`KHIDMAT_KHAZINA`] and the provider's name,
    /// and nowhere else: not into a project file, not into settings, not into
    /// anything this product syncs or exports.
    ///
    /// Blocking: the secret service can prompt or take a D-Bus round trip.
    /// Call it from the settings command handler, off the async runtime, or
    /// wrap it in `spawn_blocking` at the call site.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::BilaItimad`] when the text, after trimming, is empty,
    /// longer than [`AQSA_TUL_ITIMAD`], or contains control characters — that
    /// is not a malformed credential to store anyway, it is no credential.
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the platform store itself
    /// refuses: the store is part of the path to the provider, and a run
    /// cannot proceed to a paid endpoint past a store that cannot hold its
    /// key.
    pub fn khazzin(muzawwid: &str, nass: &str) -> Result<Self, KhataTarjama> {
        let nadhif = nass.trim();
        if nadhif.is_empty()
            || nadhif.len() > AQSA_TUL_ITIMAD
            || nadhif.chars().any(char::is_control)
        {
            return Err(KhataTarjama::BilaItimad {
                muzawwid: muzawwid.to_owned(),
            });
        }
        let madkhal = madkhal_khazina(muzawwid)?;
        madkhal
            .set_password(nadhif)
            .map_err(|khata| khata_khazina(muzawwid, &khata))?;
        Ok(Self {
            qeema: nadhif.to_owned(),
        })
    }

    /// Loads the stored credential for a provider, if one was ever entered.
    ///
    /// [`None`] is the ordinary first-run state, not a failure: nothing in
    /// this crate calls a paid endpoint before the user has entered a key and
    /// confirmed, so "no key yet" is where every project begins.
    ///
    /// Blocking, for the same reason as [`Itimad::khazzin`].
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the platform store exists but
    /// cannot be read — locked keychain, dead D-Bus session. Distinguished
    /// from [`None`] because "you have not entered a key" and "your key is in
    /// a store this machine cannot open" send the user to different fixes.
    pub fn min_khazina(muzawwid: &str) -> Result<Option<Self>, KhataTarjama> {
        let madkhal = madkhal_khazina(muzawwid)?;
        match madkhal.get_password() {
            Ok(qeema) => Ok(Some(Self { qeema })),
            Err(keyring::v1::Error::NoEntry) => Ok(None),
            Err(khata) => Err(khata_khazina(muzawwid, &khata)),
        }
    }

    /// Deletes a provider's stored credential.
    ///
    /// Idempotent: deleting a credential that is not there succeeds, because
    /// the state the caller asked for — no stored key — is the state that
    /// exists.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the store refused the
    /// deletion of a credential it holds — which matters, because a user who
    /// asked for a key to be gone and was silently told "done" while it
    /// remained would be trusting an assurance this product did not have.
    pub fn imsah(muzawwid: &str) -> Result<(), KhataTarjama> {
        let madkhal = madkhal_khazina(muzawwid)?;
        match madkhal.delete_credential() {
            Ok(()) | Err(keyring::v1::Error::NoEntry) => Ok(()),
            Err(khata) => Err(khata_khazina(muzawwid, &khata)),
        }
    }

    /// The secret, for building an authentication header and nothing else.
    ///
    /// Private on purpose. Every use is inside this module, inside a request
    /// builder, and the value never crosses the module boundary outward.
    fn qeema(&self) -> &str {
        &self.qeema
    }
}

impl fmt::Debug for Itimad {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        // No length, no prefix, no ends. See the type's documentation for why
        // each of those is a real leak and not a nicety.
        mukhraj.write_str("Itimad(<redacted>)")
    }
}

impl fmt::Display for Itimad {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj.write_str("Itimad(<redacted>)")
    }
}

/// The store entry for one provider's credential.
///
/// # Errors
///
/// [`KhataTarjama::MuzawwidGhayrMutah`] when the platform has no usable
/// credential store at all — `keyring`'s `NoDefaultStore` on a build whose
/// platform feature set matches nothing on this machine.
fn madkhal_khazina(muzawwid: &str) -> Result<keyring::v1::Entry, KhataTarjama> {
    keyring::v1::Entry::new(KHIDMAT_KHAZINA, muzawwid)
        .map_err(|khata| khata_khazina(muzawwid, &khata))
}

/// A secret-store failure, mapped to the variant that stops the run.
///
/// [`KhataTarjama::MuzawwidGhayrMutah`] is "could not be reached", and the
/// store is honestly part of the reach: a provider whose key sits behind a
/// store this machine cannot open is a provider this run cannot call, and the
/// variant already stops the run — which is the correct scope, since every
/// string would fail the same way. The message names the store, not the
/// secret; `keyring`'s errors carry no credential material.
fn khata_khazina(muzawwid: &str, khata: &keyring::v1::Error) -> KhataTarjama {
    KhataTarjama::MuzawwidGhayrMutah {
        muzawwid: muzawwid.to_owned(),
        sabab: format!("the platform secret store failed: {khata}"),
    }
}

// ---------------------------------------------------------------------------
// إذن الإنفاق — the explicit confirmation to spend
// ---------------------------------------------------------------------------

/// Proof that a user confirmed spending, with the ceiling they confirmed.
///
/// Every paid provider's constructor consumes one, so the sequence "construct
/// provider, translate, get billed" cannot begin without the confirmation
/// step having happened — the same structural discipline as
/// `taarib-tabaqa`'s tier-3 `Iqrar` and this crate's own review attestation.
/// It is deliberately not `Clone` and not `Copy`: one confirmation authorises
/// one meter, and a copyable permission would let a single "yes" fan out into
/// budgets the user never saw.
///
/// The constructor is honest about what it can and cannot enforce: it
/// witnesses that the confirmation dialog ran, it cannot prove the dialog was
/// truthful. What it *does* guarantee is that the ceiling shown is the
/// ceiling enforced, because the same value flows, unconverted, into
/// [`TakalifJarya`].
#[derive(Debug)]
pub struct IdhnInfaq {
    /// The ceiling, in nano-dollars ([`NamudhajTaklifa`]'s unit).
    saqf: u64,
    /// When it was confirmed, seconds since the Unix epoch, supplied by the
    /// caller — this crate reads no clock, per the workspace discipline.
    lahza: u64,
}

impl IdhnInfaq {
    /// Records that the user saw a cost ceiling and confirmed it.
    ///
    /// `saqf` is in nano-dollars — the unit every [`NamudhajTaklifa`] prices
    /// in — so one dollar is `1_000_000_000`. `lahza` is seconds since the
    /// Unix epoch.
    #[must_use]
    pub const fn baad_taakid(saqf: u64, lahza: u64) -> Self {
        Self { saqf, lahza }
    }

    /// The confirmed ceiling, in nano-dollars.
    #[must_use]
    pub const fn saqf(&self) -> u64 {
        self.saqf
    }

    /// When the confirmation happened, seconds since the Unix epoch.
    #[must_use]
    pub const fn lahza(&self) -> u64 {
        self.lahza
    }
}

// ---------------------------------------------------------------------------
// التكاليف — pricing, reservation, settlement
// ---------------------------------------------------------------------------

/// How a provider charges, in nano-dollars so the arithmetic is integral.
///
/// Nano-dollars per *single* token or character, not per million, because per
/// million forces a division this workspace denies and floating money is how
/// two sums of the same run disagree. Three dollars per million tokens is
/// `3_000` nano-dollars per token, exactly, and every published price this
/// module carries converts without remainder.
///
/// The prices baked into each provider's default configuration are the
/// provider's published list prices as of [`TAREEKH_ASAR`] and are plainly
/// data: a config can override them the day a price page changes, without a
/// code edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamudhajTaklifa {
    /// No cost at all — the local, loopback-only endpoint.
    Majani,
    /// Priced per token, input and output separately, as every LLM API is.
    BilRumuz {
        /// Nano-dollars per input token.
        idkhal: u64,
        /// Nano-dollars per output token.
        ikhraj: u64,
    },
    /// Priced per source character, as the dedicated translation APIs are.
    BilAhruf {
        /// Nano-dollars per character.
        harf: u64,
    },
}

impl NamudhajTaklifa {
    /// The cost of a request in token counts, in nano-dollars.
    ///
    /// Saturating throughout: a run that somehow reached `u64::MAX`
    /// nano-dollars — eighteen billion dollars — has long since hit any real
    /// ceiling, and saturation errs in the direction that keeps the ceiling
    /// promise.
    #[must_use]
    pub const fn min_rumuz(&self, idkhal: u64, ikhraj: u64) -> u64 {
        match self {
            Self::Majani | Self::BilAhruf { .. } => 0,
            Self::BilRumuz {
                idkhal: sir_idkhal,
                ikhraj: sir_ikhraj,
            } => idkhal
                .saturating_mul(*sir_idkhal)
                .saturating_add(ikhraj.saturating_mul(*sir_ikhraj)),
        }
    }

    /// The cost of a request in characters, in nano-dollars.
    #[must_use]
    pub const fn min_ahruf(&self, ahruf: u64) -> u64 {
        match self {
            Self::Majani | Self::BilRumuz { .. } => 0,
            Self::BilAhruf { harf } => ahruf.saturating_mul(*harf),
        }
    }

    /// Whether using this model costs anything at all.
    #[must_use]
    pub const fn madfu(&self) -> bool {
        !matches!(self, Self::Majani)
    }
}
/// A `u64` as a length, saturating.
///
/// The character counts the batch layer works in are `u64` and the tokeniser
/// estimate takes a length; on every target this builds for the two are the
/// same width, and saturating is the honest conversion for the one that is not.
#[must_use]
pub fn hajm_usize(qeema: u64) -> usize {
    usize::try_from(qeema).unwrap_or(usize::MAX)
}

/// An upper-bound token count for a prompt, from its byte length.
///
/// `ceil(bytes / 3)` — see [`HADD_BAYT_LIL_RAMZ`] for why three and why the
/// direction of the error matters. This is an estimate for *reserving* budget
/// only; the accumulated figure a user sees is settled to the provider's own
/// reported token count as each reply arrives, so the estimate's error lives
/// for exactly one in-flight request.
#[must_use]
pub fn taqdir_rumuz_min_bayt(bayt: usize) -> u64 {
    tul_u64(bayt).div_ceil(HADD_BAYT_LIL_RAMZ)
}

/// The running cost of one provider, against the ceiling its user confirmed.
///
/// The promise in [`KhataTarjama::SaqfTakalif`]'s documentation — stopped
/// **at** the ceiling, never past it — is kept here by reserving before
/// sending: [`TakalifJarya::ihjiz`] adds an upper-bound estimate atomically,
/// refuses when the sum would cross the ceiling, and the reservation is
/// settled down to the provider's reported usage (or released entirely when
/// nothing was sent) by the [`HajzTaklifa`] guard. Because the estimate is an
/// upper bound — output priced at the request's own `max_tokens`, input at
/// [`taqdir_rumuz_min_bayt`]'s generous divisor — no request is ever sent
/// that could legitimately bill past the line.
///
/// Atomic rather than locked because the batch layer runs providers
/// concurrently, and a mutex around a `u64` is a lock around an addition.
#[derive(Debug)]
pub struct TakalifJarya {
    /// Spent plus reserved, in nano-dollars.
    munfaq: AtomicU64,
    /// The confirmed ceiling, in nano-dollars.
    saqf: u64,
}

impl TakalifJarya {
    /// A meter authorised by an explicit confirmation.
    ///
    /// Consumes the [`IdhnInfaq`] — the permission becomes the meter, and
    /// cannot also become a second one.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "taking the permission by value is the guarantee: a borrow would leave the \
                  caller holding it and able to authorise a second meter"
    )]
    pub const fn min_idhn(idhn: IdhnInfaq) -> Self {
        Self {
            munfaq: AtomicU64::new(0),
            saqf: idhn.saqf(),
        }
    }

    /// The meter for a free provider: ceiling zero, and zero-cost
    /// reservations always fit under it.
    #[must_use]
    pub const fn majani() -> Self {
        Self {
            munfaq: AtomicU64::new(0),
            saqf: 0,
        }
    }

    /// What has been spent (plus at most one in-flight reservation), in
    /// nano-dollars. The number the interface shows beside the run.
    #[must_use]
    pub fn munfaq(&self) -> u64 {
        self.munfaq.load(Ordering::SeqCst)
    }

    /// The confirmed ceiling, in nano-dollars.
    #[must_use]
    pub const fn saqf(&self) -> u64 {
        self.saqf
    }

    /// Reserves an estimated cost, refusing at the ceiling.
    ///
    /// The returned guard releases the reservation on drop unless
    /// [`HajzTaklifa::thabbit`] settles it to the actual figure — so an error
    /// `?`-return between reservation and reply cannot strand phantom spend
    /// in the meter.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::SaqfTakalif`] when the reservation would cross the
    /// ceiling. Nothing has been sent when this returns; that is the point.
    fn ihjiz(&self, taqdir: u64) -> Result<HajzTaklifa<'_>, KhataTarjama> {
        let natija = self
            .munfaq
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |halin| {
                halin
                    .checked_add(taqdir)
                    .filter(|majmu| *majmu <= self.saqf || taqdir == 0)
            });
        match natija {
            Ok(_) => Ok(HajzTaklifa {
                hisab: self,
                taqdir,
                muthabbat: false,
            }),
            Err(halin) => Err(KhataTarjama::SaqfTakalif {
                munfaq: halin,
                saqf: self.saqf,
            }),
        }
    }
}

/// One in-flight reservation against a [`TakalifJarya`].
///
/// Settle it with [`HajzTaklifa::thabbit`] when the provider reported what
/// the request really cost; drop it unsettled and the reservation is released
/// whole, which is correct for every path where nothing billable happened —
/// a transport failure, a refused request, a reply that would not parse
/// arriving after a free error status.
struct HajzTaklifa<'hisab> {
    /// The meter the reservation sits in.
    hisab: &'hisab TakalifJarya,
    /// How much was reserved.
    taqdir: u64,
    /// Whether the reservation was settled, so drop does not release it twice.
    muthabbat: bool,
}

impl HajzTaklifa<'_> {
    /// Replaces the reservation with the actual cost.
    ///
    /// `fili` may exceed the estimate only in the sub-nano rounding sense for
    /// character-priced providers whose metering differs from ours by a
    /// header's whitespace; for token-priced providers the estimate is an
    /// upper bound by construction. Either way the meter ends holding actual
    /// spend, which is what the interface displays and what the next
    /// reservation is checked against.
    ///
    /// One `fetch_update`, never a `fetch_sub` followed by a `fetch_add`. The
    /// two-step form publishes a meter reading that is low by the whole
    /// reservation for the window between them, and the batch layer runs
    /// requests concurrently: a parallel [`TakalifJarya::ihjiz`] landing in
    /// that window is checked against a total this settlement is about to put
    /// back, so it can be admitted when the true total leaves no room. That is
    /// the ceiling being crossed by up to one reservation — exactly the
    /// promise [`KhataTarjama::SaqfTakalif`] makes and this type exists to
    /// keep.
    fn thabbit(mut self, fili: u64) {
        let _ = self
            .hisab
            .munfaq
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |halin| {
                Some(halin.saturating_sub(self.taqdir).saturating_add(fili))
            });
        self.muthabbat = true;
    }
}

impl Drop for HajzTaklifa<'_> {
    fn drop(&mut self) {
        if !self.muthabbat {
            // The subtraction cannot underflow across the meter as a whole:
            // this exact amount was added by the `ihjiz` that created the
            // guard, and nothing else removes it.
            let _ = self.hisab.munfaq.fetch_sub(self.taqdir, Ordering::SeqCst);
        }
    }
}

// ---------------------------------------------------------------------------
// القدرات — what a provider can honestly claim
// ---------------------------------------------------------------------------

/// What one provider can do, declared once and consulted everywhere.
///
/// Constructed only inside this module — the confidence field is private
/// precisely so that "does this provider report confidence" is a fact the
/// provider states about itself, not a flag a caller can set to launder a
/// number into [`NatijatTarjama`]. See [`DalilThiqa`].
#[derive(Debug, Clone)]
pub struct QudratMuzawwid {
    /// Whether the provider accepts several strings in one request.
    ///
    /// Consulted by the batch layer: a provider that batches gets its strings
    /// grouped, one that does not gets them pipelined under the rate limit.
    pub dufaat: bool,
    /// The largest tokenised text, in bytes, one request will carry.
    ///
    /// Enforced before anything is sent, so an oversized string fails itself
    /// — [`KhataTarjama::MudkhalMarfud`] — rather than costing a round trip
    /// to be refused by the wire.
    pub aqsa_hajm_talab: usize,
    /// The local request ceiling, per minute, enforced with `governor`
    /// *before* the provider's own limiter is ever tested.
    pub hadd_talabat: NonZeroU32,
    /// Whether the provider's replies carry a confidence of their own.
    ///
    /// Private, and the gate on [`QudratMuzawwid::dalil_thiqa`]. The dedicated
    /// translation APIs in this module all say `false`, because none of them
    /// reports one — see the module header for each refusal.
    yublighu_thiqa: bool,
    /// Whether the provider accepts a system instruction.
    ///
    /// The register work in [`crate::siyaq`] reaches providers that answer
    /// `true`; the others get at most the plain-text context line from
    /// [`crate::siyaq::siyaq_khatti`].
    pub yaqbal_tawjih: bool,
    /// How this provider charges.
    pub taklifa: NamudhajTaklifa,
}

impl QudratMuzawwid {
    /// Whether this provider's replies carry their own confidence.
    #[must_use]
    pub const fn yublighu_thiqa(&self) -> bool {
        self.yublighu_thiqa
    }

    /// How many strings may go in one request.
    ///
    /// One for a provider that does not batch, so the batch layer needs no
    /// special case: a chunk of one is a chunk. The ceiling for a provider that
    /// does is deliberately conservative — a request carrying two hundred
    /// strings is one whose failure costs two hundred strings, and the retry
    /// that follows re-sends every one of them.
    #[must_use]
    pub const fn aqsa_nusus(&self) -> usize {
        if self.dufaat { 50 } else { 1 }
    }

    /// What a request of this size will cost, in nano-dollars.
    ///
    /// An **estimate**, and the batch layer reserves against it before
    /// dispatching so that the cost ceiling is checked against money not yet
    /// spent. The provider settles the reservation to its own reported usage
    /// afterwards, so an estimate that was wrong costs accuracy in the running
    /// display and never causes the ceiling to be crossed.
    ///
    /// A token-priced model is estimated from characters through
    /// [`taqdir_rumuz_min_bayt`], because the caller has characters and the
    /// tokeniser is the provider's.
    #[must_use]
    pub fn qaddir_taklifa(&self, ahruf: u64) -> u64 {
        match self.taklifa {
            NamudhajTaklifa::Majani => 0,
            NamudhajTaklifa::BilAhruf { .. } => self.taklifa.min_ahruf(ahruf),
            NamudhajTaklifa::BilRumuz { .. } => {
                let idkhal = taqdir_rumuz_min_bayt(hajm_usize(ahruf));
                // Arabic output runs longer than English input in tokens far
                // more often than shorter, so the output side is estimated at
                // the input's size rather than at nothing. Under-reserving is
                // the failure that lets a run cross its own ceiling.
                self.taklifa.min_rumuz(idkhal, idkhal)
            },
        }
    }

    /// A token proving this provider reports its own confidence, or [`None`].
    ///
    /// The token is what [`NatijatTarjama`] requires before it will record a
    /// measured score, so a provider that reports nothing cannot construct one
    /// and cannot claim a confidence it never measured.
    #[must_use]
    pub const fn dalil_thiqa(&self) -> Option<DalilThiqa> {
        if self.yublighu_thiqa {
            Some(DalilThiqa(()))
        } else {
            None
        }
    }
}

/// Proof that a confidence came from a provider that reports one.
///
/// No public constructor; the only mint is [`QudratMuzawwid::dalil_thiqa`],
/// and only a capability declaring the fact yields one. It exists because the
/// tempting bug is real: a provider author "helpfully" filling the confidence
/// field with `0.9` for an API that reports nothing would poison every
/// threshold downstream with a number that reads like a measurement and is
/// not one — the exact failure Phase 11 documents for OCR engines.
#[derive(Debug)]
pub struct DalilThiqa(());

// ---------------------------------------------------------------------------
// النتيجة — one translation, and whose confidence it carries
// ---------------------------------------------------------------------------

/// One provider result: the translated text and what is known about it.
///
/// The text still carries its `hima` tokens — verification and restoration
/// belong to [`crate::hima::istaridd`], the single gate every reply passes
/// before it is believed, and doing it here would make six providers six
/// gates. A `NatijatTarjama` is therefore *transport* truth ("this is what
/// came back, this is what it cost") and never translation truth.
///
/// Fields are private and the constructors are the contract: a confidence can
/// enter only through [`NatijatTarjama::bi_thiqa`], which demands a
/// [`DalilThiqa`]. `maqisa` mirrors Phase 11's `SatrMaqru` — `false` means
/// any threshold filtering on the number is filtering on nothing, and the
/// number is simply absent rather than invented.
#[derive(Debug, Clone)]
pub struct NatijatTarjama {
    /// The reply's translated text, tokens intact.
    matn: String,
    /// The provider's own confidence, when it reported one.
    thiqa: Option<f32>,
    /// Whether `thiqa` is the provider's own number.
    maqisa: bool,
    /// What this request actually cost, in nano-dollars.
    taklifa: u64,
}

impl NatijatTarjama {
    /// A result from a provider that reported no confidence.
    ///
    /// The only constructor most providers can reach, and that is the design:
    /// `DeepL`, Google and Microsoft report nothing, so their results carry
    /// nothing.
    #[must_use]
    pub const fn bila_thiqa(matn: String, taklifa: u64) -> Self {
        Self {
            matn,
            thiqa: None,
            maqisa: false,
            taklifa,
        }
    }

    /// A result carrying the provider's own confidence.
    ///
    /// Requires the [`DalilThiqa`] only a confidence-reporting capability can
    /// mint. A non-finite value is treated as unreported rather than stored —
    /// a `NaN` confidence would poison every comparison downstream — and a
    /// finite one is clamped into `[0, 1]`.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the proof is consumed so one mint from `QudratMuzawwid::dalil_thiqa` backs \
                  one claimed confidence; a borrow would let it back any number of them"
    )]
    pub fn bi_thiqa(matn: String, taklifa: u64, thiqa: f32, dalil: DalilThiqa) -> Self {
        let DalilThiqa(()) = dalil;
        let salima = thiqa.is_finite().then(|| thiqa.clamp(0.0, 1.0));
        Self {
            matn,
            maqisa: salima.is_some(),
            thiqa: salima,
            taklifa,
        }
    }

    /// The translated text, `hima` tokens intact, ready for
    /// [`crate::hima::istaridd`].
    #[must_use]
    pub fn matn(&self) -> &str {
        &self.matn
    }

    /// The provider's own confidence, `0.0` to `1.0`, when it reported one.
    #[must_use]
    pub const fn thiqa(&self) -> Option<f32> {
        self.thiqa
    }

    /// Whether the confidence is the provider's own number.
    #[must_use]
    pub const fn maqisa(&self) -> bool {
        self.maqisa
    }

    /// What this request cost, in nano-dollars, from the provider's own cost
    /// model — settled to reported usage where the provider reports any.
    #[must_use]
    pub const fn taklifa(&self) -> u64 {
        self.taklifa
    }
}

/// Builds the result from a parsed structured reply, honestly.
///
/// The one place the confidence decision is made for all three LLM
/// providers, so it cannot be made three slightly different ways: a number is
/// carried when the reply had one *and* the capability can mint the proof,
/// and absent otherwise. Written as a free function rather than copied into
/// each provider because the copy is where the fourth, dishonest variant
/// would eventually appear.
fn natija_min_radd(radd: RaddMufassal, taklifa: u64, qudrat: &QudratMuzawwid) -> NatijatTarjama {
    match (radd.thiqa(), qudrat.dalil_thiqa()) {
        (Some(thiqa), Some(dalil)) => NatijatTarjama::bi_thiqa(radd.tarjama, taklifa, thiqa, dalil),
        _ => NatijatTarjama::bila_thiqa(radd.tarjama, taklifa),
    }
}

// ---------------------------------------------------------------------------
// السمة — the one trait
// ---------------------------------------------------------------------------

/// A translation provider.
///
/// The whole surface the batch layer sees. `tarjim` takes a [`NassMahmi`] and
/// a [`SiyaqTalab`] — and because [`NassMahmi`] is the only type with the
/// tokenised text and has no accessor to the raw source, there is nothing an
/// implementation *could* put on the wire except protected text. That is the
/// structural half of [`crate::hima`]'s guarantee and this signature is where
/// it binds.
#[async_trait::async_trait]
pub trait Muzawwid: fmt::Debug + Send + Sync {
    /// The provider's stable name — the string [`KhataTarjama`] variants
    /// carry, the keyring account, and the provenance every machine
    /// translation records.
    fn ism(&self) -> &str;

    /// The model or engine identifier this instance is configured to use,
    /// recorded as provenance beside the prompt version.
    fn namudhaj(&self) -> &str;

    /// What this provider can do. Cheap; called per string by the batch
    /// layer.
    fn qudrat(&self) -> QudratMuzawwid;

    /// The running cost meter, for display beside the run.
    fn takalif(&self) -> &TakalifJarya;

    /// Translates one protected string.
    ///
    /// The reply's text is returned with its tokens intact;
    /// [`crate::hima::istaridd`] is the caller's next step and the only
    /// verification gate.
    ///
    /// # Errors
    ///
    /// String failures — [`KhataTarjama::MudkhalMarfud`],
    /// [`KhataTarjama::RaddGhayrMufassal`] — fail this string and the run
    /// continues. Run failures — [`KhataTarjama::SaqfTakalif`],
    /// [`KhataTarjama::HaddMuadal`], [`KhataTarjama::MuzawwidGhayrMutah`] —
    /// stop the batch; [`KhataTarjama::yuqif_aljawla`] is the distinction
    /// made checkable.
    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama>;

    /// Translates a chunk in one request where the provider supports it.
    ///
    /// Defaulted to translating each string in turn, which is correct for every
    /// provider whose API takes one text — and is why
    /// [`QudratMuzawwid::dufaat`] is a *capability* rather than an assumption.
    /// A provider whose endpoint genuinely accepts an array (`DeepL`'s
    /// `text: [...]`) overrides this and issues one request; everything else
    /// gets the loop for free and behaves identically from the batch layer's
    /// side.
    ///
    /// The returned vector is positionally aligned with `talabat`. A provider
    /// overriding this **must** preserve that alignment: the batch layer maps
    /// results back to strings by position, and a reordered reply would attach
    /// each translation to the wrong string — silently, and in a way no
    /// verification downstream would catch, because every translation would be
    /// individually well-formed.
    ///
    /// # Errors
    ///
    /// As [`Muzawwid::tarjim`]. A chunk that fails as a whole is retried by the
    /// batch layer as single strings, so that one bad input does not cost the
    /// other forty-nine.
    async fn tarjim_dufa(
        &self,
        talabat: &[TalabTarjama<'_>],
    ) -> Result<Vec<NatijatTarjama>, KhataTarjama> {
        let mut natai = Vec::with_capacity(talabat.len());
        for talab in talabat {
            natai.push(self.tarjim(talab.mahmi, talab.talab).await?);
        }
        Ok(natai)
    }
}

/// One string in a chunk, with the context its prompt needs.
///
/// Borrowed rather than owned because a chunk is assembled per request and
/// thrown away, and copying a `SiyaqTalab` — which carries the surrounding
/// dialogue and every glossary match — fifty times per request would be fifty
/// allocations for data the caller already holds.
#[derive(Debug)]
pub struct TalabTarjama<'a> {
    /// The protected text. The only text-bearing field, by design.
    pub mahmi: &'a NassMahmi,
    /// Everything the prompt says about it.
    pub talab: &'a SiyaqTalab,
}

// ---------------------------------------------------------------------------
// جوهر الإرسال — the shared wire discipline
// ---------------------------------------------------------------------------

/// The rate-limit headers one provider is known to send.
///
/// Names differ per provider and several send none; a `None` here is "this
/// provider documents no such header", not "we did not bother".
#[derive(Debug, Clone, Copy)]
struct AsmaHudud {
    /// The remaining-requests header, where documented.
    baqi_talabat: Option<&'static str>,
    /// The reset-moment header, where documented.
    iadat_dabt: Option<&'static str>,
}

/// The HTTP core every provider shares: one client, one local rate limiter,
/// one retry discipline, one cost meter.
///
/// Providers own the *what* — endpoint, body, headers — and this owns the
/// *how often* and *what happens when it fails*, so six providers cannot
/// drift into six retry policies, five of which would be wrong in the same
/// review.
struct JawharIrsal {
    /// The provider's stable name, for errors and logs.
    ism: String,
    /// The one client, TLS-only except for the loopback constructor.
    amil: reqwest::Client,
    /// The local limiter, consulted before every send.
    hadd: DefaultDirectRateLimiter,
    /// The cost meter, authorised by the user's confirmation.
    takalif: TakalifJarya,
    /// Which rate-limit headers to read off replies.
    asma: AsmaHudud,
    /// The shortest wait before retrying a `429`, whatever the backoff schedule
    /// or the provider's own `Retry-After` says. Zero for every keyed provider
    /// — their limits are documented and their headers are honest — and raised
    /// by [`JawharIrsal::bi_ard_muadal`] for a service whose `429` means abuse
    /// detection, where retrying after half a second is what keeps it tripped.
    ard_muadal: Duration,
    /// Whether this provider presents no credential at all.
    ///
    /// Changes what a `401`/`403` *means*: for a keyed provider it is a key the
    /// service will not accept, for a credential-free service it is the service
    /// refusing this client. Set by [`JawharIrsal::bila_itimad`], and only the
    /// free web endpoint sets it — the loopback constructor also sends no key,
    /// but a `403` from a local server is a misconfigured server, not a block.
    bila_itimad: bool,
}

impl fmt::Debug for JawharIrsal {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Hand-written because the limiter's internals are noise and the
        // client's are worse; what a debugger wants is who and how much.
        mukhraj
            .debug_struct("JawharIrsal")
            .field("ism", &self.ism)
            .field("munfaq", &self.takalif.munfaq())
            .field("saqf", &self.takalif.saqf())
            .finish_non_exhaustive()
    }
}

impl JawharIrsal {
    /// Builds the core for one provider.
    ///
    /// `mahalli` is `true` only for the loopback constructor: it relaxes
    /// `https_only`, because a local Ollama-style server speaks plain HTTP on
    /// `127.0.0.1` and TLS to loopback protects against nothing that is not
    /// already inside the machine. Every remote provider gets a client that
    /// refuses plaintext outright. Redirects are refused for everyone: an API
    /// `POST` has no legitimate redirect, and following one with an
    /// authentication header attached is how a key visits a host it was never
    /// meant for.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the TLS stack cannot be
    /// built, which stops the run — every request would fail identically.
    fn jadeed(
        ism: String,
        mahalli: bool,
        hadd_talabat: NonZeroU32,
        takalif: TakalifJarya,
        asma: AsmaHudud,
    ) -> Result<Self, KhataTarjama> {
        let amil = reqwest::Client::builder()
            .user_agent(format!("Taarib/{}", taarib_usus::ISDAR))
            .timeout(MUHLAT_TALAB)
            .connect_timeout(MUHLAT_ITTISAL)
            .https_only(!mahalli)
            .redirect(reqwest::redirect::Policy::none())
            .referer(false)
            .build()
            .map_err(|khata| KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: ism.clone(),
                sabab: format!("the HTTP client could not be built: {khata}"),
            })?;
        Ok(Self {
            ism,
            amil,
            hadd: RateLimiter::direct(Quota::per_minute(hadd_talabat)),
            takalif,
            asma,
            ard_muadal: Duration::ZERO,
            bila_itimad: false,
        })
    }

    /// Replaces the per-minute limiter with a fixed gap between requests.
    ///
    /// A per-minute quota admits its whole minute as a burst, which is what a
    /// documented API limit means and the wrong shape for a public web endpoint
    /// that watches request *spacing*: four hundred requests in the first
    /// second of a minute is the traffic pattern its abuse detection exists to
    /// catch. One permit per `fasl`, no burst, is the honest translation of
    /// "a floor between requests". A zero gap is no limiter at all and is
    /// accepted as such, because the caller that asks for it is a test.
    fn bi_fasl(mut self, fasl: Duration) -> Self {
        // `Quota::with_period` is `None` only for a zero period.
        if let Some(hissa) = Quota::with_period(fasl) {
            self.hadd = RateLimiter::direct(hissa.allow_burst(NonZeroU32::MIN));
        } else {
            self.hadd = RateLimiter::direct(Quota::per_second(NonZeroU32::MAX));
        }
        self
    }

    /// Sets the floor on the wait after a `429`; see [`JawharIrsal::ard_muadal`].
    const fn bi_ard_muadal(mut self, ard: Duration) -> Self {
        self.ard_muadal = ard;
        self
    }

    /// Declares that this provider sends no credential; see
    /// [`JawharIrsal::bila_itimad`].
    const fn bila_itimad(mut self) -> Self {
        self.bila_itimad = true;
        self
    }

    /// The refusal a `401`/`403` amounts to for this provider.
    fn khata_rafd(&self, hala: reqwest::StatusCode) -> KhataTarjama {
        if self.bila_itimad {
            KhataTarjama::MajjaniMahjub {
                muzawwid: self.ism.clone(),
                sabab: format!("the service refused this client (HTTP {})", hala.as_u16()),
            }
        } else {
            KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: self.ism.clone(),
                sabab: format!("the credential was refused (HTTP {})", hala.as_u16()),
            }
        }
    }

    /// The failure rate limiting amounts to once every attempt has been spent.
    ///
    /// For a keyed provider it is [`KhataTarjama::HaddMuadal`], which the batch
    /// layer answers with a bounded number of further waits honouring the
    /// provider's own `Retry-After`. For a credential-free service it is the
    /// block itself: its `429` carries no schedule, comes from abuse detection
    /// rather than a quota, and clears in minutes or hours — so the run stops
    /// with a sentence that says so rather than waiting on a header that will
    /// not come.
    fn khata_muadal(&self, thawani: Option<u64>) -> KhataTarjama {
        if self.bila_itimad {
            KhataTarjama::MajjaniMahjub {
                muzawwid: self.ism.clone(),
                sabab: format!(
                    "rate limiting (HTTP 429) outlasted {ADAD_MUHAWALAT} attempts{}",
                    thawani.map_or_else(String::new, |thawani| format!(
                        ", the last asking for a {thawani}-second wait"
                    ))
                ),
            }
        } else {
            KhataTarjama::HaddMuadal {
                muzawwid: self.ism.clone(),
                thawani,
            }
        }
    }

    /// Sends one request under the shared discipline.
    ///
    /// In order, per attempt: wait for the local limiter, send, read the
    /// provider's rate-limit headers, then decide. Success returns the
    /// response unread. `429` retries after the provider's own `Retry-After`
    /// when it sent one — the server stated its schedule and retrying on a
    /// guessed one instead is how clients get banned — never sooner than
    /// [`JawharIrsal::ard_muadal`], and `5xx`/`408` and transport failures
    /// retry on doubling backoff. `401`/`403` stop the run at once: a refused
    /// credential fails every string identically and retrying it is asking
    /// the same question louder; for a credential-free service the same
    /// statuses are the service refusing this client, [`KhataTarjama::MajjaniMahjub`].
    /// Any other `4xx` fails this string with the provider's own words.
    ///
    /// The request is rebuilt through `ibn` on every attempt because a sent
    /// `reqwest` request is consumed; the closure owns the body data and
    /// rebuilding is a serialisation, not a second computation.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::HaddMuadal`] when rate limiting outlasted every
    /// attempt, [`KhataTarjama::MuzawwidGhayrMutah`] when the transport or
    /// the provider's servers did, or when the credential or quota was
    /// refused outright, [`KhataTarjama::MajjaniMahjub`] for either of those
    /// two refusals from a credential-free service, and
    /// [`KhataTarjama::MudkhalMarfud`] when the provider refused this
    /// particular input.
    async fn irsal<F>(&self, ibn: F) -> Result<reqwest::Response, KhataTarjama>
    where
        F: Fn(&reqwest::Client) -> reqwest::RequestBuilder + Send + Sync,
    {
        let mut sabab_akhir = SababIada::Naql("no attempt completed".to_owned());

        for muhawala in 0..ADAD_MUHAWALAT {
            if muhawala > 0 {
                let mudda = match &sabab_akhir {
                    SababIada::Muadal(thawani) => {
                        muddat_taraju(muhawala, *thawani).max(self.ard_muadal)
                    },
                    SababIada::Khadim(_) | SababIada::Naql(_) => muddat_taraju(muhawala, None),
                };
                tokio::time::sleep(mudda).await;
            }
            self.hadd.until_ready().await;

            let radd = match ibn(&self.amil).send().await {
                Ok(radd) => radd,
                Err(khata) => {
                    // `without_url` because a reqwest error's Display includes
                    // the full URL, and while nothing secret rides in this
                    // module's URLs, a rule with exceptions is not a rule.
                    let sabab = khata.without_url().to_string();
                    tracing::debug!(muzawwid = %self.ism, muhawala, sabab, "transport failure");
                    sabab_akhir = SababIada::Naql(sabab);
                    continue;
                },
            };

            self.sajjil_hudud(&radd);
            let hala = radd.status();
            if hala.is_success() {
                return Ok(radd);
            }
            if hala == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let thawani = muddat_intizar(&radd);
                tracing::debug!(muzawwid = %self.ism, muhawala, thawani, "rate limited");
                sabab_akhir = SababIada::Muadal(thawani);
                continue;
            }
            if hala == reqwest::StatusCode::UNAUTHORIZED || hala == reqwest::StatusCode::FORBIDDEN {
                return Err(self.khata_rafd(hala));
            }
            // DeepL's own status for an exhausted character quota. Terminal
            // for the run: the account, not the request, is out of budget.
            if hala.as_u16() == 456 {
                return Err(KhataTarjama::MuzawwidGhayrMutah {
                    muzawwid: self.ism.clone(),
                    sabab: "the account's translation quota is exhausted (HTTP 456)".to_owned(),
                });
            }
            if hala.is_server_error() || hala == reqwest::StatusCode::REQUEST_TIMEOUT {
                let raqm = hala.as_u16();
                tracing::debug!(muzawwid = %self.ism, muhawala, hala = raqm, "server failure");
                sabab_akhir = SababIada::Khadim(raqm);
                continue;
            }

            // Every remaining status is a 4xx about this request: too large,
            // malformed by the provider's lights, filtered. One string fails.
            let jasad = mukhtasar_jasad(radd).await;
            return Err(KhataTarjama::MudkhalMarfud {
                muzawwid: self.ism.clone(),
                sabab: format!("HTTP {}: {jasad}", hala.as_u16()),
            });
        }

        Err(match sabab_akhir {
            SababIada::Muadal(thawani) => self.khata_muadal(thawani),
            SababIada::Khadim(hala) => KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: self.ism.clone(),
                sabab: format!("HTTP {hala} persisted across {ADAD_MUHAWALAT} attempts"),
            },
            SababIada::Naql(sabab) => KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: self.ism.clone(),
                sabab,
            },
        })
    }

    /// Reads and logs the provider's rate-limit headers.
    ///
    /// Read for visibility, not for control: the local `governor` limiter is
    /// the control, and these headers are how an operator staring at a slow
    /// batch learns whether the provider or the local limit is the wall. A
    /// remaining count of zero is warned, because the next request is known
    /// to bounce before it is sent.
    fn sajjil_hudud(&self, radd: &reqwest::Response) {
        if let Some(ism_ras) = self.asma.baqi_talabat {
            match qeemat_ras_u64(radd, ism_ras) {
                Some(0) => tracing::warn!(
                    muzawwid = %self.ism,
                    "the provider reports zero remaining requests in this window"
                ),
                Some(baqi) => {
                    tracing::trace!(muzawwid = %self.ism, baqi, "provider request budget");
                },
                None => {},
            }
        }
        if let Some(ism_ras) = self.asma.iadat_dabt
            && let Some(qeema) = qeemat_ras(radd, ism_ras)
        {
            tracing::trace!(muzawwid = %self.ism, iadat_dabt = %qeema, "provider limit reset");
        }
    }
}

/// Why the last attempt did not return, for choosing the final error.
enum SababIada {
    /// Rate limited, with the provider's requested wait when it stated one.
    Muadal(Option<u64>),
    /// A server-side failure with this status.
    Khadim(u16),
    /// The transport failed with this description.
    Naql(String),
}

/// The provider's `Retry-After`, in whole seconds, when it sent a usable one.
///
/// Only the delta-seconds form is honoured. The HTTP-date form would need a
/// clock comparison against a header from a machine whose clock is not ours,
/// and misreading it as an enormous wait would hang a batch; the providers
/// this module targets all send delta seconds on `429`.
fn muddat_intizar(radd: &reqwest::Response) -> Option<u64> {
    qeemat_ras_u64(radd, "retry-after")
}

/// The wait before a retry: the server's own request when it made one,
/// bounded by [`AQSA_TARAJU_THAWANI`], and doubling backoff from
/// [`ASAS_TARAJU_MILLI`] otherwise.
fn muddat_taraju(muhawala: u32, talab_khadim: Option<u64>) -> Duration {
    if let Some(thawani) = talab_khadim {
        return Duration::from_secs(thawani.min(AQSA_TARAJU_THAWANI));
    }
    // Shift bounded to six: 500ms << 6 is 32s, and the attempt count keeps
    // the sum far under any timeout that matters.
    let milli = ASAS_TARAJU_MILLI << muhawala.min(6);
    Duration::from_millis(milli)
}

/// A response header as text, when present and readable.
fn qeemat_ras<'radd>(radd: &'radd reqwest::Response, ism: &str) -> Option<&'radd str> {
    radd.headers().get(ism)?.to_str().ok().map(str::trim)
}

/// A response header as an unsigned number, when it is one.
fn qeemat_ras_u64(radd: &reqwest::Response, ism: &str) -> Option<u64> {
    qeemat_ras(radd, ism)?.parse::<u64>().ok()
}

/// The first part of an error body, for a refusal message.
///
/// Bounded exactly like `hima`'s reply excerpts and for the same reason: the
/// provider's words are the useful part, four kilobytes of HTML error page
/// are not.
async fn mukhtasar_jasad(radd: reqwest::Response) -> String {
    match radd.text().await {
        Ok(jasad) => jasad.chars().take(200).collect(),
        Err(_) => "(the error body could not be read)".to_owned(),
    }
}

/// Reads a success body as text, for parsing as the requested structure.
///
/// A `2xx` whose body cannot be read is a reply this crate cannot check, and
/// an unchecked reply is never believed — so it maps to the same failure as
/// one that read but did not parse, with a fixed marker in the excerpt
/// position.
///
/// # Errors
///
/// [`KhataTarjama::RaddGhayrMufassal`] when the body could not be read.
async fn jasad_najah(radd: reqwest::Response, muzawwid: &str) -> Result<String, KhataTarjama> {
    radd.text()
        .await
        .map_err(|_| KhataTarjama::RaddGhayrMufassal {
            muzawwid: muzawwid.to_owned(),
            radd: "(the reply body could not be read)".to_owned(),
        })
}

/// The size refusal, shared by every provider's pre-flight check.
///
/// Checked before anything is reserved or sent so an oversized string costs
/// nothing and fails alone — the batch continues, which is exactly the
/// string-versus-run distinction [`KhataTarjama`] documents.
fn mudkhal_kabir(muzawwid: &str, hajm: usize, saqf: usize) -> KhataTarjama {
    KhataTarjama::MudkhalMarfud {
        muzawwid: muzawwid.to_owned(),
        sabab: format!("the tokenised text is {hajm} bytes, above the {saqf}-byte ceiling"),
    }
}

// ---------------------------------------------------------------------------
// Shared LLM constants
// ---------------------------------------------------------------------------

/// The tool name every structured LLM reply is forced through.
///
/// One name across providers so the parsing layer and the logs agree on what
/// a reply was asked to be. ASCII because Anthropic constrains tool names to
/// `[a-zA-Z0-9_-]`.
pub const ISM_ADAT_TARJAMA: &str = "sallim_tarjama";

/// The sampling temperature for an LLM request that will carry one.
///
/// Low, because the product wants the *same* Arabic for the same source —
/// `AlamJawda::TarjamaMutanaqida` exists precisely because players notice two
/// renderings of one string. Not zero, because a floor of variation helps the
/// model out of degenerate token loops on unusual inputs; two tenths is the
/// conventional compromise.
///
/// **Sent to some models and withheld from others**, which is not a
/// preference but a wire fact. Both `OpenAI`'s GPT-5 generation and
/// Anthropic's models from Opus 4.7 onwards answer a non-default temperature
/// with `HTTP 400` — `OpenAI` in so many words: *"Unsupported value:
/// 'temperature' does not support 0.2 with this model. Only the default (1)
/// value is supported."* This module reads a `400` as a refusal of the
/// particular input, so a temperature sent to a model that refuses it does
/// not fail once and get noticed: it fails every string of a run, each time
/// blaming a game string that was never the problem.
///
/// Gemini refuses nothing but warns of the same value: Google's Gemini 3
/// guidance is to drop an explicit temperature because one below the default
/// "may lead to unexpected behavior, such as looping" — a reply that is
/// generated, billed, and then discarded.
///
/// Which models receive it is therefore a per-provider table, not a constant:
/// [`hararat_anthropic`], [`hararat_gemini`], [`IdadatAnthropic::hararat`],
/// [`IdadatGemini::hararat`] and [`IdadatMuwafiqOpenAI::hararat`]. All three
/// omit for an identifier they do not recognise.
pub const HARARAT_TARJAMA: f32 = 0.2;

// ---------------------------------------------------------------------------
// Anthropic — /v1/messages, structured through a forced tool call
// ---------------------------------------------------------------------------

/// The Anthropic Messages endpoint.
pub const UNWAN_ANTHROPIC: &str = "https://api.anthropic.com/v1/messages";

/// The `anthropic-version` header value this module speaks.
pub const ISDAR_WAJIHAT_ANTHROPIC: &str = "2023-06-01";

/// Configuration for the Anthropic provider.
///
/// The [`Default`] carries the published list prices as of [`TAREEKH_ASAR`];
/// they are data, overridable the day the price page changes.
#[derive(Debug, Clone)]
pub struct IdadatAnthropic {
    /// The model identifier, e.g. `claude-sonnet-5`.
    pub namudhaj: String,
    /// The output token cap sent as `max_tokens`.
    ///
    /// Also the exact output bound the cost reservation uses: the API
    /// enforces it, so output spend cannot exceed it. Kept small — game
    /// strings are short and a runaway reply is cost, not value.
    pub aqsa_ikhraj: u32,
    /// The local request ceiling per minute.
    pub hadd_talabat: NonZeroU32,
    /// The price, per token, in nano-dollars.
    pub taklifa: NamudhajTaklifa,
    /// The sampling temperature, or [`None`] to omit the field entirely.
    ///
    /// [`None`] is not "use the provider's default by saying so" — it is *not
    /// sending the parameter*, which is a different request. Anthropic
    /// deprecated `temperature` from Claude Opus 4.7 onwards and the current
    /// models **reject** a non-default value with `HTTP 400`, so a hardcoded
    /// `0.2` turns every string of a run into a refusal on exactly the models
    /// a user would pick today. See [`hararat_anthropic`] for how the default
    /// is chosen and why the unknown case omits.
    pub hararat: Option<f32>,
}

impl Default for IdadatAnthropic {
    fn default() -> Self {
        let namudhaj = "claude-sonnet-5".to_owned();
        let taklifa = taklifat_anthropic(&namudhaj);
        let hararat = hararat_anthropic(&namudhaj);
        Self {
            namudhaj,
            aqsa_ikhraj: 1024,
            hadd_talabat: hadd_thabit(50),
            taklifa,
            hararat,
        }
    }
}

/// The published Anthropic list price for a model family, per token.
///
/// The price page as of [`TAREEKH_ASAR`], converted exactly to nano-dollars
/// per token: $10/$50 per million for the Fable and Mythos line, $5/$25 for
/// Opus, $2/$10 for Sonnet 5, $3/$15 for the Sonnet 4 line, $1/$5 for Haiku.
///
/// An unrecognised identifier gets the **highest** table entry, not the
/// cheapest and not zero: the number guards a spending ceiling, and the safe
/// direction for an unknown is over. That invariant is a claim about this
/// function's *arithmetic*, so the fallback moves whenever a new top of the
/// range is published — it sat at the Opus figure while Fable was already
/// twice that, which quietly halved the ceiling for every identifier the
/// table did not name.
#[must_use]
pub fn taklifat_anthropic(namudhaj: &str) -> NamudhajTaklifa {
    if namudhaj.contains("haiku") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 1_000,
            ikhraj: 5_000,
        }
    } else if namudhaj.contains("sonnet-5") || namudhaj.contains("sonnet5") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 2_000,
            ikhraj: 10_000,
        }
    } else if namudhaj.contains("sonnet") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 3_000,
            ikhraj: 15_000,
        }
    } else if namudhaj.contains("opus") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 5_000,
            ikhraj: 25_000,
        }
    } else {
        NamudhajTaklifa::BilRumuz {
            idkhal: 10_000,
            ikhraj: 50_000,
        }
    }
}

/// Whether this Anthropic model will accept a sampling temperature.
///
/// [`Some`] only for the identifier families documented to take one;
/// [`None`] — omit the field — for everything else, **including anything the
/// table does not recognise**. The asymmetry is the whole point and it is the
/// mirror image of [`taklifat_anthropic`]'s: omitting a temperature the model
/// would have accepted costs a little determinism on one provider, while
/// sending one the model rejects costs `HTTP 400` on *every* request, which
/// [`JawharIrsal::irsal`] classifies as a refusal of this particular input —
/// so a whole run would fail string by string with a message about the input
/// when nothing was ever wrong with it.
///
/// Anthropic deprecated `temperature`, `top_p` and `top_k` from Claude Opus
/// 4.7 onwards; on those models a non-default value is a `400`. Since a
/// future model is far likelier to continue that direction than to reverse
/// it, an unrecognised identifier omits.
#[must_use]
pub fn hararat_anthropic(namudhaj: &str) -> Option<f32> {
    let yaqbal = namudhaj.contains("haiku-4-5")
        || namudhaj.contains("sonnet-4-5")
        || namudhaj.contains("sonnet-4-6")
        || namudhaj.contains("opus-4-5")
        || namudhaj.contains("opus-4-6");
    yaqbal.then_some(HARARAT_TARJAMA)
}

/// The Anthropic provider.
///
/// Structured output is obtained the way the Messages API actually offers
/// it: a single tool whose `input_schema` is the reply schema, with
/// `tool_choice` forcing that tool — the model cannot answer except by
/// producing arguments that validate, and the arguments arrive as JSON data
/// in the `tool_use` block, never as prose to be scraped.
#[derive(Debug)]
pub struct MuzawwidAnthropic {
    /// The configuration this instance was built with.
    idadat: IdadatAnthropic,
    /// The credential. Present by construction; see [`MuzawwidAnthropic::jadeed`].
    itimad: Itimad,
    /// The shared wire core.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
}

impl MuzawwidAnthropic {
    /// The provider's stable name.
    pub const ISM: &'static str = "anthropic";

    /// Builds the provider — credential and confirmation required, by
    /// signature.
    ///
    /// There is no other constructor: an Anthropic provider without an
    /// [`Itimad`] and an [`IdhnInfaq`] is not an unusable value, it is an
    /// inexpressible one.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the HTTP client cannot be
    /// built.
    pub fn jadeed(
        idadat: IdadatAnthropic,
        itimad: Itimad,
        idhn: IdhnInfaq,
    ) -> Result<Self, KhataTarjama> {
        let jawhar = JawharIrsal::jadeed(
            Self::ISM.to_owned(),
            false,
            idadat.hadd_talabat,
            TakalifJarya::min_idhn(idhn),
            AsmaHudud {
                baqi_talabat: Some("anthropic-ratelimit-requests-remaining"),
                iadat_dabt: Some("anthropic-ratelimit-requests-reset"),
            },
        )?;
        let qudrat = QudratMuzawwid {
            dufaat: false,
            aqsa_hajm_talab: 32 * 1024,
            hadd_talabat: idadat.hadd_talabat,
            yublighu_thiqa: true,
            yaqbal_tawjih: true,
            taklifa: idadat.taklifa,
        };
        Ok(Self {
            idadat,
            itimad,
            jawhar,
            qudrat,
        })
    }
}

/// One Messages API request, as this module sends it.
#[derive(Debug, Serialize)]
struct TalabAnthropic<'talab> {
    /// The model identifier.
    model: &'talab str,
    /// The output cap the cost reservation relies on.
    max_tokens: u32,
    /// The system instruction from [`crate::siyaq::tawjih_nizam`].
    system: &'talab str,
    /// The single user turn.
    messages: Vec<RisalaAnthropic<'talab>>,
    /// The one tool whose input schema is the reply schema.
    tools: Vec<AdatAnthropic<'talab>>,
    /// Forces that tool: `{"type": "tool", "name": ISM_ADAT_TARJAMA}`.
    tool_choice: Value,
    /// [`HARARAT_TARJAMA`], or absent from the body entirely — see
    /// [`IdadatAnthropic::hararat`]. Skipped rather than sent as `null`,
    /// because `"temperature": null` is a value the API reads and rejects.
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// One message turn.
#[derive(Debug, Serialize)]
struct RisalaAnthropic<'talab> {
    /// `user` — this module sends exactly one turn.
    role: &'static str,
    /// The user message from [`crate::siyaq::risalat_mustakhdim`].
    content: &'talab str,
}

/// The tool definition carrying the reply schema.
#[derive(Debug, Serialize)]
struct AdatAnthropic<'talab> {
    /// [`ISM_ADAT_TARJAMA`].
    name: &'static str,
    /// What the tool is for, in the model's working language.
    description: &'static str,
    /// [`crate::siyaq::mukhattat_sarim`].
    input_schema: &'talab Value,
}

/// The Messages API reply, reduced to what this module reads.
#[derive(Debug, Deserialize)]
struct RaddAnthropic {
    /// The content blocks; the forced tool call arrives as one of them.
    #[serde(default)]
    content: Vec<QitatAnthropic>,
    /// Why generation stopped — `tool_use` is the success shape.
    #[serde(default)]
    stop_reason: Option<String>,
    /// Token accounting, used to settle the cost reservation.
    #[serde(default)]
    usage: Option<IstikhdamAnthropic>,
}

/// One content block.
#[derive(Debug, Deserialize)]
struct QitatAnthropic {
    /// `text` or `tool_use`.
    #[serde(rename = "type")]
    naw: String,
    /// The tool name, on `tool_use` blocks.
    #[serde(default)]
    name: Option<String>,
    /// The tool arguments — the structured reply itself.
    #[serde(default)]
    input: Option<Value>,
}

/// The reply's token accounting.
#[derive(Debug, Deserialize)]
struct IstikhdamAnthropic {
    /// Billed input tokens.
    #[serde(default)]
    input_tokens: Option<u64>,
    /// Billed output tokens.
    #[serde(default)]
    output_tokens: Option<u64>,
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidAnthropic {
    fn ism(&self) -> &str {
        Self::ISM
    }

    fn namudhaj(&self) -> &str {
        &self.idadat.namudhaj
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        if mahmi.matn().len() > self.qudrat.aqsa_hajm_talab {
            return Err(mudkhal_kabir(
                Self::ISM,
                mahmi.matn().len(),
                self.qudrat.aqsa_hajm_talab,
            ));
        }

        let tawjih = siyaq::tawjih_nizam(talab, mahmi, true);
        let risala = siyaq::risalat_mustakhdim(talab, mahmi);
        let mukhattat = siyaq::mukhattat_sarim(true);
        let hajm_mukhattat = mukhattat.to_string().len();

        // Reservation: input from the whole prompt's bytes, output at the
        // request's own hard cap — an upper bound the API itself enforces.
        let taqdir = self.idadat.taklifa.min_rumuz(
            taqdir_rumuz_min_bayt(
                tawjih
                    .len()
                    .saturating_add(risala.len())
                    .saturating_add(hajm_mukhattat),
            ),
            u64::from(self.idadat.aqsa_ikhraj),
        );
        let hajz = self.jawhar.takalif.ihjiz(taqdir)?;

        let jasad = TalabAnthropic {
            model: &self.idadat.namudhaj,
            max_tokens: self.idadat.aqsa_ikhraj,
            system: &tawjih,
            messages: vec![RisalaAnthropic {
                role: "user",
                content: &risala,
            }],
            tools: vec![AdatAnthropic {
                name: ISM_ADAT_TARJAMA,
                description: "Deliver the finished Arabic translation of the given game string.",
                input_schema: &mukhattat,
            }],
            tool_choice: json!({ "type": "tool", "name": ISM_ADAT_TARJAMA }),
            temperature: self.idadat.hararat,
        };

        let radd = self
            .jawhar
            .irsal(|amil| {
                amil.post(UNWAN_ANTHROPIC)
                    .header("x-api-key", self.itimad.qeema())
                    .header("anthropic-version", ISDAR_WAJIHAT_ANTHROPIC)
                    .json(&jasad)
            })
            .await?;

        // From here the wire said 2xx, so tokens were billed no matter what
        // the body turns out to hold: every failure path below settles the
        // reservation instead of releasing it, at the estimate when the
        // provider's own count never became readable.
        let nass_radd = match jasad_najah(radd, Self::ISM).await {
            Ok(nass) => nass,
            Err(khata) => {
                hajz.thabbit(taqdir);
                return Err(khata);
            },
        };
        let Ok(mufakkak) = serde_json::from_str::<RaddAnthropic>(&nass_radd) else {
            hajz.thabbit(taqdir);
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            });
        };

        // Settle the reservation to the provider's own accounting before any
        // verdict on the content: tokens were billed even for a refusal.
        let fili = mufakkak.usage.as_ref().map_or(taqdir, |istikhdam| {
            self.idadat.taklifa.min_rumuz(
                istikhdam.input_tokens.unwrap_or_default(),
                istikhdam.output_tokens.unwrap_or_default(),
            )
        });
        hajz.thabbit(fili);

        match mufakkak.stop_reason.as_deref() {
            Some("refusal") => {
                return Err(KhataTarjama::MudkhalMarfud {
                    muzawwid: Self::ISM.to_owned(),
                    sabab: "the model refused this input (stop_reason: refusal)".to_owned(),
                });
            },
            Some("max_tokens") => {
                return Err(KhataTarjama::MudkhalMarfud {
                    muzawwid: Self::ISM.to_owned(),
                    sabab: format!(
                        "the reply hit the {}-token output cap and is incomplete",
                        self.idadat.aqsa_ikhraj
                    ),
                });
            },
            _ => {},
        }

        let hujja = mufakkak
            .content
            .iter()
            .find(|qita| qita.naw == "tool_use" && qita.name.as_deref() == Some(ISM_ADAT_TARJAMA))
            .and_then(|qita| qita.input.as_ref())
            .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            })?;

        let mufassal = hallil_qeema(hujja, Self::ISM)?;
        Ok(natija_min_radd(mufassal, fili, &self.qudrat))
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible — /v1/chat/completions, including local loopback servers
// ---------------------------------------------------------------------------

/// The `OpenAI` platform's base address, for [`IdadatMuwafiqOpenAI::openai`].
pub const ASAS_OPENAI: &str = "https://api.openai.com";

/// The conventional local Ollama address, for
/// [`IdadatMuwafiqOpenAI::ollama`].
pub const ASAS_OLLAMA: &str = "http://127.0.0.1:11434";

/// Configuration for any endpoint speaking the Chat Completions dialect.
///
/// One provider type covers `OpenAI` itself, Azure-style gateways, and a local
/// Ollama, because the wire is the same shape; what differs — address,
/// pricing, whether the server knows the modern parameters — is data here.
#[derive(Debug, Clone)]
pub struct IdadatMuwafiqOpenAI {
    /// The provider's stable name: the keyring account, the error label, the
    /// provenance string. `openai` for the platform, `ollama` for the local
    /// default; a gateway gets whatever its operator calls it.
    pub ism: String,
    /// The base address; `/v1/chat/completions` is appended.
    pub asas: String,
    /// The model identifier, e.g. `gpt-5` or `qwen3`.
    pub namudhaj: String,
    /// The output token cap.
    pub aqsa_ikhraj: u32,
    /// The local request ceiling per minute.
    pub hadd_talabat: NonZeroU32,
    /// The price, per token, in nano-dollars.
    pub taklifa: NamudhajTaklifa,
    /// Whether the server speaks the modern dialect: `response_format` of
    /// type `json_schema` and `max_completion_tokens`.
    ///
    /// `false` falls back to `json_object` mode and `max_tokens`, which every
    /// older compatible server understands. The system instruction already
    /// says "JSON" in so many words, which `json_object` mode requires of the
    /// prompt.
    pub lahja_haditha: bool,
    /// The sampling temperature, or [`None`] to omit the field entirely.
    ///
    /// [`None`] on the `OpenAI` platform and [`Some`] on a local server, and
    /// the split is not a preference. Every GPT-5-and-later model answers a
    /// non-default `temperature` with `HTTP 400` —
    /// `"Unsupported value: 'temperature' does not support 0.2 with this
    /// model. Only the default (1) value is supported."` — so the field that
    /// was there to keep two renderings of one string identical was instead
    /// failing all of them. A local model behind an Ollama-compatible
    /// endpoint takes it normally, costs nothing to get wrong, and is where
    /// the determinism actually helps.
    pub hararat: Option<f32>,
}

impl IdadatMuwafiqOpenAI {
    /// The `OpenAI` platform with its published list price as of
    /// [`TAREEKH_ASAR`].
    ///
    /// `gpt-5` is the identifier here because it is still served and still on
    /// the price page at $1.25/$10 per million, with no announced retirement
    /// — unlike a default naming the current flagship, which would silently
    /// move every user onto `gpt-6-astra` at $10/$50. It *is* a legacy model,
    /// and a user who wants the current one names it in settings; that is
    /// what [`taklifat_openai`] is for, so the meter follows the name.
    #[must_use]
    pub fn openai() -> Self {
        let namudhaj = "gpt-5".to_owned();
        let taklifa = taklifat_openai(&namudhaj);
        Self {
            ism: "openai".to_owned(),
            asas: ASAS_OPENAI.to_owned(),
            namudhaj,
            aqsa_ikhraj: 1024,
            hadd_talabat: hadd_thabit(60),
            taklifa,
            lahja_haditha: true,
            hararat: None,
        }
    }

    /// A local Ollama server: loopback, free, and modern enough for
    /// `json_schema` since Ollama 0.5.
    #[must_use]
    pub fn ollama(namudhaj: impl Into<String>) -> Self {
        Self {
            ism: "ollama".to_owned(),
            asas: ASAS_OLLAMA.to_owned(),
            namudhaj: namudhaj.into(),
            aqsa_ikhraj: 1024,
            hadd_talabat: hadd_thabit(120),
            taklifa: NamudhajTaklifa::Majani,
            lahja_haditha: true,
            hararat: Some(HARARAT_TARJAMA),
        }
    }
}

/// The published `OpenAI` list price for a model identifier, per token.
///
/// The price page as of [`TAREEKH_ASAR`], in nano-dollars per token. It
/// exists because the other two token-priced providers have one and this one
/// did not: a caller that copied a model name out of settings into
/// [`IdadatMuwafiqOpenAI::namudhaj`] without also replacing
/// [`IdadatMuwafiqOpenAI::taklifa`] was metering `gpt-6-astra` at $10/$50 as
/// though it were `gpt-5` at $1.25/$10, and a ceiling computed eight times
/// too loose is not a ceiling.
///
/// Matched longest-first, because `gpt-5.4-mini` also contains `gpt-5`. An
/// unrecognised identifier gets the highest entry in the table, for the
/// reason [`taklifat_anthropic`] gives: the number guards a spending ceiling
/// and the safe direction for an unknown is over.
///
/// A self-hosted or gateway endpoint speaking this dialect is priced by
/// whatever its operator sets on the configuration, not here — this table
/// only knows the platform's own names.
#[must_use]
pub fn taklifat_openai(namudhaj: &str) -> NamudhajTaklifa {
    let asaar: [(&str, u64, u64); 12] = [
        ("gpt-6-astra", 10_000, 50_000),
        ("gpt-5.6-sol", 4_000, 20_000),
        ("gpt-5.6-terra", 2_000, 12_000),
        ("gpt-5.6-luna", 200, 1_200),
        ("gpt-5.5-pro", 30_000, 180_000),
        ("gpt-5.5", 5_000, 30_000),
        ("gpt-5.4-pro", 30_000, 180_000),
        ("gpt-5.4-mini", 750, 4_500),
        ("gpt-5.4-nano", 200, 1_250),
        ("gpt-5.4", 2_500, 15_000),
        ("gpt-5.2", 1_750, 14_000),
        ("gpt-5.1", 1_250, 10_000),
    ];
    for (ism, idkhal, ikhraj) in asaar {
        if namudhaj.contains(ism) {
            return NamudhajTaklifa::BilRumuz { idkhal, ikhraj };
        }
    }
    if namudhaj.contains("gpt-5-nano") {
        return NamudhajTaklifa::BilRumuz {
            idkhal: 50,
            ikhraj: 400,
        };
    }
    if namudhaj.contains("gpt-5-mini") {
        return NamudhajTaklifa::BilRumuz {
            idkhal: 250,
            ikhraj: 2_000,
        };
    }
    if namudhaj.contains("gpt-5") {
        return NamudhajTaklifa::BilRumuz {
            idkhal: 1_250,
            ikhraj: 10_000,
        };
    }
    NamudhajTaklifa::BilRumuz {
        idkhal: 30_000,
        ikhraj: 180_000,
    }
}

/// Any Chat-Completions-compatible provider.
///
/// Two constructors and the difference is the whole point:
/// [`MuzawwidMuwafiqOpenAI::jadeed`] demands a credential and a spending
/// confirmation and refuses plaintext addresses;
/// [`MuzawwidMuwafiqOpenAI::mahalli`] takes neither and refuses everything
/// *except* loopback — so the credential-free path structurally cannot reach
/// a paid host.
#[derive(Debug)]
pub struct MuzawwidMuwafiqOpenAI {
    /// The configuration this instance was built with.
    idadat: IdadatMuwafiqOpenAI,
    /// The credential — absent only on the loopback path.
    itimad: Option<Itimad>,
    /// The full endpoint, base joined with the fixed path.
    unwan: String,
    /// The shared wire core.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
}

impl MuzawwidMuwafiqOpenAI {
    /// Builds a remote provider — credential and confirmation required.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the base address does not
    /// parse, is not `https`, or the HTTP client cannot be built. A paid
    /// endpoint over plaintext would put the `Authorization` header on the
    /// wire readable, so it is refused here and again by the client's own
    /// `https_only`.
    pub fn jadeed(
        idadat: IdadatMuwafiqOpenAI,
        itimad: Itimad,
        idhn: IdhnInfaq,
    ) -> Result<Self, KhataTarjama> {
        let rabt = rabt_salih(&idadat.ism, &idadat.asas)?;
        if rabt.scheme() != "https" {
            return Err(KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: idadat.ism,
                sabab: "a remote endpoint must be https; plaintext would expose the credential"
                    .to_owned(),
            });
        }
        Self::ibn(idadat, Some(itimad), TakalifJarya::min_idhn(idhn), false)
    }

    /// Builds the loopback provider — no credential, no confirmation, and no
    /// way to point it anywhere but this machine.
    ///
    /// The address must resolve to loopback by *name* — `127.0.0.1`,
    /// `localhost`, or `::1` — not merely by routing, because a hostname that
    /// happens to resolve locally today is a config edit away from a paid
    /// endpoint reached without a credential check tomorrow.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the address does not parse,
    /// its host is not literally loopback, or the HTTP client cannot be
    /// built.
    pub fn mahalli(idadat: IdadatMuwafiqOpenAI) -> Result<Self, KhataTarjama> {
        let rabt = rabt_salih(&idadat.ism, &idadat.asas)?;
        // `host_str` keeps the brackets around an IPv6 literal, so both
        // spellings are listed rather than trimmed — trimming is where a
        // clever hostname would start negotiating.
        let mudif = rabt.host_str().unwrap_or_default();
        if !matches!(mudif, "127.0.0.1" | "localhost" | "::1" | "[::1]") {
            return Err(KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: idadat.ism,
                sabab: format!(
                    "the credential-free constructor accepts loopback only; {mudif:?} is not"
                ),
            });
        }
        let mut mahalliya = idadat;
        // A loopback server has no bill; whatever the config claimed, the
        // meter and the model both say free, so the two cannot disagree.
        mahalliya.taklifa = NamudhajTaklifa::Majani;
        Self::ibn(mahalliya, None, TakalifJarya::majani(), true)
    }

    /// The shared tail of both constructors.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the HTTP client cannot be
    /// built.
    fn ibn(
        idadat: IdadatMuwafiqOpenAI,
        itimad: Option<Itimad>,
        takalif: TakalifJarya,
        mahalli: bool,
    ) -> Result<Self, KhataTarjama> {
        let jawhar = JawharIrsal::jadeed(
            idadat.ism.clone(),
            mahalli,
            idadat.hadd_talabat,
            takalif,
            AsmaHudud {
                baqi_talabat: Some("x-ratelimit-remaining-requests"),
                iadat_dabt: Some("x-ratelimit-reset-requests"),
            },
        )?;
        let unwan = format!("{}/v1/chat/completions", idadat.asas.trim_end_matches('/'));
        let qudrat = QudratMuzawwid {
            dufaat: false,
            aqsa_hajm_talab: 32 * 1024,
            hadd_talabat: idadat.hadd_talabat,
            yublighu_thiqa: true,
            yaqbal_tawjih: true,
            taklifa: idadat.taklifa,
        };
        Ok(Self {
            idadat,
            itimad,
            unwan,
            jawhar,
            qudrat,
        })
    }
}

/// A base address that parses at all.
///
/// # Errors
///
/// [`KhataTarjama::MuzawwidGhayrMutah`] when it does not — a configuration
/// failure that would fail every request identically.
fn rabt_salih(ism: &str, asas: &str) -> Result<reqwest::Url, KhataTarjama> {
    reqwest::Url::parse(asas).map_err(|khata| KhataTarjama::MuzawwidGhayrMutah {
        muzawwid: ism.to_owned(),
        sabab: format!("the endpoint address does not parse: {khata}"),
    })
}

/// One Chat Completions request, as this module sends it.
#[derive(Debug, Serialize)]
struct TalabMuwafiq<'talab> {
    /// The model identifier.
    model: &'talab str,
    /// System turn then user turn, in that order.
    messages: Vec<RisalaMuwafiq<'talab>>,
    /// [`HARARAT_TARJAMA`], or absent from the body entirely — see
    /// [`IdadatMuwafiqOpenAI::hararat`]. Skipped rather than sent as `null`:
    /// the platform reads `null` as a value and refuses it.
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// The modern output cap; sent in the modern dialect only.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    /// The legacy output cap; sent in the legacy dialect only.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// `json_schema` with [`crate::siyaq::mukhattat_sarim`], or `json_object`
    /// in the legacy dialect.
    response_format: Value,
}

/// One message turn.
#[derive(Debug, Serialize)]
struct RisalaMuwafiq<'talab> {
    /// `system` or `user`.
    role: &'static str,
    /// The turn's text.
    content: &'talab str,
}

/// The Chat Completions reply, reduced to what this module reads.
#[derive(Debug, Deserialize)]
struct RaddMuwafiq {
    /// The choices; this module requests and reads exactly one.
    #[serde(default)]
    choices: Vec<KhiyarMuwafiq>,
    /// Token accounting, used to settle the cost reservation.
    #[serde(default)]
    usage: Option<IstikhdamMuwafiq>,
}

/// One choice.
#[derive(Debug, Deserialize)]
struct KhiyarMuwafiq {
    /// The assistant message carrying the JSON text.
    message: RisalatRaddMuwafiq,
    /// Why generation stopped; `length` and `content_filter` are failures.
    #[serde(default)]
    finish_reason: Option<String>,
}

/// The assistant message.
#[derive(Debug, Deserialize)]
struct RisalatRaddMuwafiq {
    /// The JSON text of the structured reply.
    #[serde(default)]
    content: Option<String>,
}

/// The reply's token accounting.
#[derive(Debug, Deserialize)]
struct IstikhdamMuwafiq {
    /// Billed input tokens.
    #[serde(default)]
    prompt_tokens: Option<u64>,
    /// Billed output tokens.
    #[serde(default)]
    completion_tokens: Option<u64>,
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidMuwafiqOpenAI {
    fn ism(&self) -> &str {
        &self.idadat.ism
    }

    fn namudhaj(&self) -> &str {
        &self.idadat.namudhaj
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        if mahmi.matn().len() > self.qudrat.aqsa_hajm_talab {
            return Err(mudkhal_kabir(
                self.ism(),
                mahmi.matn().len(),
                self.qudrat.aqsa_hajm_talab,
            ));
        }

        let tawjih = siyaq::tawjih_nizam(talab, mahmi, true);
        let risala = siyaq::risalat_mustakhdim(talab, mahmi);

        let sighat_radd = if self.idadat.lahja_haditha {
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": ISM_ADAT_TARJAMA,
                    "strict": true,
                    "schema": siyaq::mukhattat_sarim(true)
                }
            })
        } else {
            json!({ "type": "json_object" })
        };
        let hajm_sigha = sighat_radd.to_string().len();

        let taqdir = self.idadat.taklifa.min_rumuz(
            taqdir_rumuz_min_bayt(
                tawjih
                    .len()
                    .saturating_add(risala.len())
                    .saturating_add(hajm_sigha),
            ),
            u64::from(self.idadat.aqsa_ikhraj),
        );
        let hajz = self.jawhar.takalif.ihjiz(taqdir)?;

        let jasad = TalabMuwafiq {
            model: &self.idadat.namudhaj,
            messages: vec![
                RisalaMuwafiq {
                    role: "system",
                    content: &tawjih,
                },
                RisalaMuwafiq {
                    role: "user",
                    content: &risala,
                },
            ],
            temperature: self.idadat.hararat,
            max_completion_tokens: self.idadat.lahja_haditha.then_some(self.idadat.aqsa_ikhraj),
            max_tokens: (!self.idadat.lahja_haditha).then_some(self.idadat.aqsa_ikhraj),
            response_format: sighat_radd,
        };

        let radd =
            self.jawhar
                .irsal(|amil| {
                    let talab_khaam = amil.post(&self.unwan).json(&jasad);
                    match &self.itimad {
                        Some(itimad) => talab_khaam
                            .header("authorization", format!("Bearer {}", itimad.qeema())),
                        None => talab_khaam,
                    }
                })
                .await?;

        // A 2xx was billed whatever the body holds: settle, never release,
        // on every path below.
        let nass_radd = match jasad_najah(radd, self.ism()).await {
            Ok(nass) => nass,
            Err(khata) => {
                hajz.thabbit(taqdir);
                return Err(khata);
            },
        };
        let Ok(mufakkak) = serde_json::from_str::<RaddMuwafiq>(&nass_radd) else {
            hajz.thabbit(taqdir);
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: self.ism().to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            });
        };

        let fili = mufakkak.usage.as_ref().map_or(taqdir, |istikhdam| {
            self.idadat.taklifa.min_rumuz(
                istikhdam.prompt_tokens.unwrap_or_default(),
                istikhdam.completion_tokens.unwrap_or_default(),
            )
        });
        hajz.thabbit(fili);

        let khiyar = mufakkak
            .choices
            .first()
            .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                muzawwid: self.ism().to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            })?;

        match khiyar.finish_reason.as_deref() {
            Some("content_filter") => {
                return Err(KhataTarjama::MudkhalMarfud {
                    muzawwid: self.ism().to_owned(),
                    sabab: "the endpoint's content filter refused this string".to_owned(),
                });
            },
            Some("length") => {
                return Err(KhataTarjama::MudkhalMarfud {
                    muzawwid: self.ism().to_owned(),
                    sabab: format!(
                        "the reply hit the {}-token output cap and is incomplete",
                        self.idadat.aqsa_ikhraj
                    ),
                });
            },
            _ => {},
        }

        let nass_json =
            khiyar
                .message
                .content
                .as_deref()
                .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                    muzawwid: self.ism().to_owned(),
                    radd: nass_radd.chars().take(64).collect(),
                })?;

        let mufassal = hallil_radd(nass_json, self.ism())?;
        Ok(natija_min_radd(mufassal, fili, &self.qudrat))
    }
}

/// A nonzero per-minute limit from a literal.
///
/// Total without panicking: zero — which no caller in this file writes —
/// falls back to one request per minute, the most conservative limit rather
/// than the most permissive.
const fn hadd_thabit(qeema: u32) -> NonZeroU32 {
    match NonZeroU32::new(qeema) {
        Some(hadd) => hadd,
        None => NonZeroU32::MIN,
    }
}

// ---------------------------------------------------------------------------
// Google Gemini — models/{model}:generateContent with a response schema
// ---------------------------------------------------------------------------

/// The Gemini API's model collection root.
pub const ASAS_GEMINI: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Configuration for the Gemini provider.
#[derive(Debug, Clone)]
pub struct IdadatGemini {
    /// The model identifier, e.g. `gemini-2.5-pro`.
    pub namudhaj: String,
    /// The output token cap sent as `maxOutputTokens`.
    pub aqsa_ikhraj: u32,
    /// The local request ceiling per minute.
    pub hadd_talabat: NonZeroU32,
    /// The price, per token, in nano-dollars.
    pub taklifa: NamudhajTaklifa,
    /// The sampling temperature, or [`None`] to leave the field out of
    /// `generationConfig` entirely.
    ///
    /// Gemini differs from the other two LLM providers here and the
    /// difference is worth stating precisely: it still *accepts* the field —
    /// there is no `400` — and the harm is to the output rather than to the
    /// request. Google's own Gemini 3 guidance is to remove an explicit
    /// temperature and leave the default of `1.0`, because a value below it
    /// "may lead to unexpected behavior, such as looping or degraded
    /// performance". A looping reply is not a free failure: it is generated,
    /// billed, counted against the ceiling, and then thrown away by
    /// [`crate::hima::istaridd`] or the output cap — the run pays for garbage
    /// and retries it. See [`hararat_gemini`].
    pub hararat: Option<f32>,
}

impl Default for IdadatGemini {
    fn default() -> Self {
        // `gemini-2.5-pro` stood here and is a June 2025 model with three
        // newer stable Flash releases behind it; Google schedules the whole
        // 2.5 line for shutdown and developers report new projects already
        // getting "no longer available to new users" from it. A default that
        // only works for accounts which used it a year ago is not a default a
        // stranger can install on. The current Pro is a *preview*, which a
        // shipped default must not be — previews retire on two weeks' notice
        // — so the current stable Flash is what this names.
        let namudhaj = "gemini-3.8-flash".to_owned();
        let taklifa = taklifat_gemini(&namudhaj);
        let hararat = hararat_gemini(&namudhaj);
        Self {
            namudhaj,
            aqsa_ikhraj: 1024,
            hadd_talabat: hadd_thabit(120),
            taklifa,
            hararat,
        }
    }
}

/// Whether this Gemini model should be sent a sampling temperature.
///
/// [`Some`] for the 2.5 generation, which takes one normally and is steadier
/// with a low value; [`None`] for the 3.x generation and for anything the
/// table does not recognise, per Google's own migration guidance to drop an
/// explicit temperature on Gemini 3 and leave the default.
///
/// The unknown case omits for the reason [`hararat_anthropic`] gives, with
/// one substitution: the cost of guessing wrong is not a refused request here
/// but a reply that loops, bills, and is thrown away.
#[must_use]
pub fn hararat_gemini(namudhaj: &str) -> Option<f32> {
    namudhaj.contains("2.5").then_some(HARARAT_TARJAMA)
}

/// The published Gemini list price for a model family, per token.
///
/// The price page as of [`TAREEKH_ASAR`]: $0.30/$2.50 per million for 2.5
/// Flash, $1.50/$7.50 for the 3.x Flash line, $1.25/$10 for 2.5 Pro, and
/// $4/$18 for 3.1 Pro at its long-prompt tier.
///
/// Two of these are deliberately the *higher* of two published figures. The
/// 3.x Flash line currently bills $0.75/$3.75 under an introductory rate that
/// ends on the last day of 2026 and doubles the next morning, and the Pro
/// tiers split at a two-hundred-thousand-token prompt. Pricing a ceiling at
/// the promotional figure would under-reserve by half the moment the promotion
/// lapses, with nothing in the product noticing; taking the standing rate
/// costs a user some headroom and never costs them money they did not agree
/// to.
///
/// An unrecognised identifier gets the highest table entry; the number guards
/// a ceiling and the safe error is over. The `2.5` arms are matched before
/// the family arms because `gemini-2.5-flash` contains `flash` too.
#[must_use]
pub fn taklifat_gemini(namudhaj: &str) -> NamudhajTaklifa {
    if namudhaj.contains("2.5-flash") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 300,
            ikhraj: 2_500,
        }
    } else if namudhaj.contains("2.5-pro") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 1_250,
            ikhraj: 10_000,
        }
    } else if namudhaj.contains("flash") {
        NamudhajTaklifa::BilRumuz {
            idkhal: 1_500,
            ikhraj: 7_500,
        }
    } else {
        NamudhajTaklifa::BilRumuz {
            idkhal: 4_000,
            ikhraj: 18_000,
        }
    }
}

/// The Gemini provider.
///
/// Structured output through `generationConfig.responseMimeType:
/// "application/json"` plus `responseSchema` — the *portable* schema from
/// [`crate::siyaq::mukhattat_radd`], because Gemini's `OpenAPI` subset rejects
/// the `additionalProperties` field the strict variant carries.
///
/// The API key travels in the `x-goog-api-key` header and never in the
/// query string, although the API accepts both: a query string is copied
/// into proxy logs, server access logs and browser histories by
/// infrastructure that never looks at headers.
#[derive(Debug)]
pub struct MuzawwidGemini {
    /// The configuration this instance was built with.
    idadat: IdadatGemini,
    /// The credential. Present by construction.
    itimad: Itimad,
    /// The full endpoint, model baked in.
    unwan: String,
    /// The shared wire core.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
}

impl MuzawwidGemini {
    /// The provider's stable name.
    pub const ISM: &'static str = "gemini";

    /// Builds the provider — credential and confirmation required.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the model identifier
    /// contains characters that do not belong in a URL path segment — the
    /// identifier is spliced into the endpoint, and refusing strange input
    /// here is cheaper than discovering what the far end does with it — or
    /// when the HTTP client cannot be built.
    pub fn jadeed(
        idadat: IdadatGemini,
        itimad: Itimad,
        idhn: IdhnInfaq,
    ) -> Result<Self, KhataTarjama> {
        let salih = !idadat.namudhaj.is_empty()
            && idadat
                .namudhaj
                .chars()
                .all(|harf| harf.is_ascii_alphanumeric() || matches!(harf, '-' | '.' | '_'));
        if !salih {
            return Err(KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: Self::ISM.to_owned(),
                sabab: "the model identifier is not a plain URL path segment".to_owned(),
            });
        }
        let jawhar = JawharIrsal::jadeed(
            Self::ISM.to_owned(),
            false,
            idadat.hadd_talabat,
            TakalifJarya::min_idhn(idhn),
            AsmaHudud {
                baqi_talabat: None,
                iadat_dabt: None,
            },
        )?;
        let unwan = format!("{ASAS_GEMINI}/{}:generateContent", idadat.namudhaj);
        let qudrat = QudratMuzawwid {
            dufaat: false,
            aqsa_hajm_talab: 32 * 1024,
            hadd_talabat: idadat.hadd_talabat,
            yublighu_thiqa: true,
            yaqbal_tawjih: true,
            taklifa: idadat.taklifa,
        };
        Ok(Self {
            idadat,
            itimad,
            unwan,
            jawhar,
            qudrat,
        })
    }
}

/// One `generateContent` request, as this module sends it.
#[derive(Debug, Serialize)]
struct TalabGemini<'talab> {
    /// The system instruction, as a content object without a role.
    #[serde(rename = "systemInstruction")]
    tawjih: MuhtawaGemini<'talab>,
    /// The single user turn.
    contents: Vec<MuhtawaMawsumGemini<'talab>>,
    /// JSON mode, the response schema, the temperature and the output cap.
    #[serde(rename = "generationConfig")]
    idadat_tawlid: Value,
}

/// A content object: parts only.
#[derive(Debug, Serialize)]
struct MuhtawaGemini<'talab> {
    /// The text parts.
    parts: Vec<JuzGemini<'talab>>,
}

/// A content object with a role.
#[derive(Debug, Serialize)]
struct MuhtawaMawsumGemini<'talab> {
    /// `user` — this module sends exactly one turn.
    role: &'static str,
    /// The text parts.
    parts: Vec<JuzGemini<'talab>>,
}

/// One text part.
#[derive(Debug, Serialize)]
struct JuzGemini<'talab> {
    /// The text.
    text: &'talab str,
}

/// The `generateContent` reply, reduced to what this module reads.
#[derive(Debug, Deserialize)]
struct RaddGemini {
    /// The candidates; this module reads the first.
    #[serde(default)]
    candidates: Vec<MurashshahGemini>,
    /// Input-side refusal, present when the prompt itself was blocked.
    #[serde(rename = "promptFeedback", default)]
    tarji_talab: Option<TarjiGemini>,
    /// Token accounting, used to settle the cost reservation.
    #[serde(rename = "usageMetadata", default)]
    istikhdam: Option<IstikhdamGemini>,
}

/// One candidate.
#[derive(Debug, Deserialize)]
struct MurashshahGemini {
    /// The model's content, whose first part carries the JSON text.
    #[serde(default)]
    content: Option<MuhtawaRaddGemini>,
    /// Why generation stopped; anything but `STOP` is a failure here.
    #[serde(rename = "finishReason", default)]
    sabab_tawaqquf: Option<String>,
}

/// A reply content object.
#[derive(Debug, Deserialize)]
struct MuhtawaRaddGemini {
    /// The parts.
    #[serde(default)]
    parts: Vec<JuzRaddGemini>,
}

/// One reply part.
#[derive(Debug, Deserialize)]
struct JuzRaddGemini {
    /// The JSON text of the structured reply.
    #[serde(default)]
    text: Option<String>,
}

/// The input-blocked report.
#[derive(Debug, Deserialize)]
struct TarjiGemini {
    /// Why the prompt was blocked, when it was.
    #[serde(rename = "blockReason", default)]
    sabab_hajb: Option<String>,
}

/// The reply's token accounting.
#[derive(Debug, Deserialize)]
struct IstikhdamGemini {
    /// Billed input tokens.
    #[serde(rename = "promptTokenCount", default)]
    rumuz_idkhal: Option<u64>,
    /// Billed output tokens.
    #[serde(rename = "candidatesTokenCount", default)]
    rumuz_ikhraj: Option<u64>,
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidGemini {
    fn ism(&self) -> &str {
        Self::ISM
    }

    fn namudhaj(&self) -> &str {
        &self.idadat.namudhaj
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        if mahmi.matn().len() > self.qudrat.aqsa_hajm_talab {
            return Err(mudkhal_kabir(
                Self::ISM,
                mahmi.matn().len(),
                self.qudrat.aqsa_hajm_talab,
            ));
        }

        let tawjih = siyaq::tawjih_nizam(talab, mahmi, true);
        let risala = siyaq::risalat_mustakhdim(talab, mahmi);
        let mut idadat_tawlid = json!({
            "responseMimeType": "application/json",
            "responseSchema": siyaq::mukhattat_radd(true),
            "maxOutputTokens": self.idadat.aqsa_ikhraj
        });
        // Inserted rather than written into the literal, because the honest
        // absence is the key not being there at all — a `null` temperature is
        // a value Gemini would read.
        if let Some(hararat) = self.idadat.hararat
            && let Some(kain) = idadat_tawlid.as_object_mut()
        {
            let _ = kain.insert("temperature".to_owned(), json!(hararat));
        }
        let hajm_idadat = idadat_tawlid.to_string().len();

        let taqdir = self.idadat.taklifa.min_rumuz(
            taqdir_rumuz_min_bayt(
                tawjih
                    .len()
                    .saturating_add(risala.len())
                    .saturating_add(hajm_idadat),
            ),
            u64::from(self.idadat.aqsa_ikhraj),
        );
        let hajz = self.jawhar.takalif.ihjiz(taqdir)?;

        let jasad = TalabGemini {
            tawjih: MuhtawaGemini {
                parts: vec![JuzGemini { text: &tawjih }],
            },
            contents: vec![MuhtawaMawsumGemini {
                role: "user",
                parts: vec![JuzGemini { text: &risala }],
            }],
            idadat_tawlid,
        };

        let radd = self
            .jawhar
            .irsal(|amil| {
                amil.post(&self.unwan)
                    .header("x-goog-api-key", self.itimad.qeema())
                    .json(&jasad)
            })
            .await?;

        // A 2xx was billed whatever the body holds: settle, never release,
        // on every path below.
        let nass_radd = match jasad_najah(radd, Self::ISM).await {
            Ok(nass) => nass,
            Err(khata) => {
                hajz.thabbit(taqdir);
                return Err(khata);
            },
        };
        let Ok(mufakkak) = serde_json::from_str::<RaddGemini>(&nass_radd) else {
            hajz.thabbit(taqdir);
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            });
        };

        let fili = mufakkak.istikhdam.as_ref().map_or(taqdir, |istikhdam| {
            self.idadat.taklifa.min_rumuz(
                istikhdam.rumuz_idkhal.unwrap_or_default(),
                istikhdam.rumuz_ikhraj.unwrap_or_default(),
            )
        });
        hajz.thabbit(fili);

        if let Some(tarji) = &mufakkak.tarji_talab
            && let Some(sabab) = &tarji.sabab_hajb
        {
            return Err(KhataTarjama::MudkhalMarfud {
                muzawwid: Self::ISM.to_owned(),
                sabab: format!("the prompt was blocked: {sabab}"),
            });
        }

        let murashshah =
            mufakkak
                .candidates
                .first()
                .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                    muzawwid: Self::ISM.to_owned(),
                    radd: nass_radd.chars().take(64).collect(),
                })?;

        if let Some(sabab) = &murashshah.sabab_tawaqquf
            && sabab != "STOP"
        {
            return Err(KhataTarjama::MudkhalMarfud {
                muzawwid: Self::ISM.to_owned(),
                sabab: format!("generation stopped early ({sabab})"),
            });
        }

        let nass_json = murashshah
            .content
            .as_ref()
            .and_then(|muhtawa| muhtawa.parts.first())
            .and_then(|juz| juz.text.as_deref())
            .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            })?;

        let mufassal = hallil_radd(nass_json, Self::ISM)?;
        Ok(natija_min_radd(mufassal, fili, &self.qudrat))
    }
}

// ---------------------------------------------------------------------------
// DeepL — /v2/translate
// ---------------------------------------------------------------------------

/// The `DeepL` Pro endpoint.
pub const UNWAN_DEEPL: &str = "https://api.deepl.com/v2/translate";

/// The `DeepL` Free endpoint, used automatically for keys ending in `:fx`.
pub const UNWAN_DEEPL_HURR: &str = "https://api-free.deepl.com/v2/translate";

/// The target language of everything this product translates.
pub const HADAF_ARABI: &str = "AR";

/// Configuration for the `DeepL` provider.
#[derive(Debug, Clone)]
pub struct IdadatDeepL {
    /// The source language code, e.g. `EN` or `JA`.
    pub lughat_masdar: String,
    /// The local request ceiling per minute.
    pub hadd_talabat: NonZeroU32,
    /// The price, per source character, in nano-dollars — $25 per million
    /// characters, `DeepL`'s long-standing published API rate.
    ///
    /// **Re-check this before a release far past [`TAREEKH_ASAR`].** `DeepL`
    /// restructured its developer plans during 2026 and the per-character
    /// overage on the plan a new customer is now sold is reported above this
    /// figure. The number here is the one that could be confirmed against a
    /// published rate rather than a summary of one, and a money constant is
    /// the last place to write down a price nobody checked — but a ceiling
    /// computed from a rate that is too low is a ceiling that lets a run past
    /// it, so this is the entry to verify first.
    ///
    /// Must stay character-priced: `DeepL` bills characters, and the meter
    /// counts with [`NamudhajTaklifa::min_ahruf`], which prices a token model
    /// at zero — a misconfiguration that would record spend as free.
    pub taklifa: NamudhajTaklifa,
}

impl Default for IdadatDeepL {
    fn default() -> Self {
        Self {
            lughat_masdar: "EN".to_owned(),
            hadd_talabat: hadd_thabit(60),
            taklifa: NamudhajTaklifa::BilAhruf { harf: 25_000 },
        }
    }
}

/// The `DeepL` provider.
///
/// ## Why `tag_handling` and `ignore_tags` are deliberately not used
///
/// `DeepL` can be told to treat input as XML and leave named tags alone. Using
/// that here would mean wrapping every `hima` atom in an *invented* XML tag —
/// reintroducing exactly the class of markup-collision problem the ⟦n⟧
/// tokens were designed out of, and betting the placeholder's survival on
/// `DeepL`'s XML parser agreeing with this crate about tag boundaries. The
/// tokens do not need the favour: they are opaque punctuation-and-digit
/// clusters with no language for the engine to translate, they ride through
/// as symbols, and [`crate::hima::istaridd`] verifies every one on return —
/// so a token `DeepL` *did* damage fails the string loudly instead of shipping.
/// `preserve_formatting` is sent as `true` so the engine does not "helpfully"
/// normalise punctuation around them.
///
/// ## Confidence
///
/// The reply carries translations and a detected source language — no
/// confidence of any kind. None is invented: results are
/// [`NatijatTarjama::bila_thiqa`], with `maqisa` false, and the capability
/// says so.
#[derive(Debug)]
pub struct MuzawwidDeepL {
    /// The configuration this instance was built with.
    idadat: IdadatDeepL,
    /// The credential. Present by construction.
    itimad: Itimad,
    /// Pro or Free endpoint, chosen from the key's own suffix.
    unwan: &'static str,
    /// The shared wire core.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
}

impl MuzawwidDeepL {
    /// The provider's stable name.
    pub const ISM: &'static str = "deepl";

    /// Builds the provider — credential and confirmation required.
    ///
    /// The endpoint is chosen from the credential itself: `DeepL` issues Free
    /// keys with a `:fx` suffix and serves them only on the Free host, so
    /// asking the user "which plan" would be asking for a fact the key
    /// already states.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the HTTP client cannot be
    /// built.
    pub fn jadeed(
        idadat: IdadatDeepL,
        itimad: Itimad,
        idhn: IdhnInfaq,
    ) -> Result<Self, KhataTarjama> {
        let unwan = if itimad.qeema().ends_with(":fx") {
            UNWAN_DEEPL_HURR
        } else {
            UNWAN_DEEPL
        };
        let jawhar = JawharIrsal::jadeed(
            Self::ISM.to_owned(),
            false,
            idadat.hadd_talabat,
            TakalifJarya::min_idhn(idhn),
            AsmaHudud {
                baqi_talabat: None,
                iadat_dabt: None,
            },
        )?;
        let qudrat = QudratMuzawwid {
            dufaat: true,
            aqsa_hajm_talab: 100 * 1024,
            hadd_talabat: idadat.hadd_talabat,
            yublighu_thiqa: false,
            yaqbal_tawjih: false,
            taklifa: idadat.taklifa,
        };
        Ok(Self {
            idadat,
            itimad,
            unwan,
            jawhar,
            qudrat,
        })
    }
}

/// One `/v2/translate` request, as this module sends it.
#[derive(Debug, Serialize)]
struct TalabDeepL<'talab> {
    /// The strings to translate; this module sends one.
    text: Vec<&'talab str>,
    /// The source language.
    source_lang: &'talab str,
    /// [`HADAF_ARABI`].
    target_lang: &'static str,
    /// Keeps the engine from normalising punctuation around the tokens.
    preserve_formatting: bool,
    /// The situation, from [`crate::siyaq::siyaq_khatti`] — informs the
    /// translation without being translated. `DeepL` documents context as
    /// excluded from billing; the meter counts only `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

/// The `/v2/translate` reply.
#[derive(Debug, Deserialize)]
struct RaddDeepL {
    /// One entry per input string.
    #[serde(default)]
    translations: Vec<TarjamatDeepL>,
}

/// One translation entry.
#[derive(Debug, Deserialize)]
struct TarjamatDeepL {
    /// The translated text.
    text: String,
    /// What the engine believed the source language was. Read for the log
    /// only — it is not a confidence and is not treated as one.
    #[serde(rename = "detected_source_language", default)]
    lugha_muktashafa: Option<String>,
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidDeepL {
    fn ism(&self) -> &str {
        Self::ISM
    }

    fn namudhaj(&self) -> &'static str {
        "deepl-v2"
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        if mahmi.matn().len() > self.qudrat.aqsa_hajm_talab {
            return Err(mudkhal_kabir(
                Self::ISM,
                mahmi.matn().len(),
                self.qudrat.aqsa_hajm_talab,
            ));
        }

        // Character pricing is exact before sending: DeepL bills the
        // characters of `text`, which are counted here, so the reservation
        // *is* the cost and settlement changes nothing.
        let ahruf = tul_u64(mahmi.matn().chars().count());
        let taqdir = self.idadat.taklifa.min_ahruf(ahruf);
        let hajz = self.jawhar.takalif.ihjiz(taqdir)?;

        let jasad = TalabDeepL {
            text: vec![mahmi.matn()],
            source_lang: &self.idadat.lughat_masdar,
            target_lang: HADAF_ARABI,
            preserve_formatting: true,
            context: siyaq::siyaq_khatti(talab),
        };

        let radd = self
            .jawhar
            .irsal(|amil| {
                amil.post(self.unwan)
                    .header(
                        "authorization",
                        format!("DeepL-Auth-Key {}", self.itimad.qeema()),
                    )
                    .json(&jasad)
            })
            .await?;

        // A 2xx means the counted characters were billed; the settlement is
        // unconditional from here, parse outcome or not.
        let nass_radd = match jasad_najah(radd, Self::ISM).await {
            Ok(nass) => nass,
            Err(khata) => {
                hajz.thabbit(taqdir);
                return Err(khata);
            },
        };
        hajz.thabbit(taqdir);
        let mufakkak: RaddDeepL =
            serde_json::from_str(&nass_radd).map_err(|_| KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            })?;

        let tarjama =
            mufakkak
                .translations
                .first()
                .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                    muzawwid: Self::ISM.to_owned(),
                    radd: nass_radd.chars().take(64).collect(),
                })?;
        if tarjama.text.trim().is_empty() {
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: "(the translation came back empty)".to_owned(),
            });
        }
        if let Some(lugha) = &tarjama.lugha_muktashafa {
            tracing::trace!(muzawwid = Self::ISM, lugha, "detected source language");
        }

        // No confidence exists in a DeepL reply, so none is carried — the
        // refusal the module header names.
        Ok(NatijatTarjama::bila_thiqa(tarjama.text.clone(), taqdir))
    }
}

// ---------------------------------------------------------------------------
// Google Cloud Translation v3 — projects/{p}:translateText
// ---------------------------------------------------------------------------

/// The Cloud Translation v3 service root.
pub const ASAS_GOOGLE_SAHABI: &str = "https://translate.googleapis.com/v3";

/// How the Cloud Translation credential is presented.
///
/// The v3 API takes either an API key or a service-account OAuth access
/// token; the [`Itimad`] holds whichever the user has, and this says which
/// header carries it. Two variants rather than sniffing the value, because a
/// credential's shape is not a contract and guessing wrong sends a secret in
/// the wrong header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TawtheeqGoogle {
    /// An API key, sent as `x-goog-api-key`. Never in the query string: query
    /// strings are copied into access logs and proxy logs by infrastructure
    /// that never reads headers.
    MiftahApi,
    /// A service-account access token, sent as `Authorization: Bearer`.
    /// Minting and refreshing the token is the caller's affair — this module
    /// stores and sends credentials, it does not run OAuth flows.
    HamilOauth,
}

/// Configuration for the Cloud Translation provider.
#[derive(Debug, Clone)]
pub struct IdadatGoogleSahabi {
    /// The Google Cloud project identifier or number the requests bill to.
    pub mashru: String,
    /// The location, e.g. `us-central1`, or [`None`] for the global endpoint.
    pub makan: Option<String>,
    /// The source language code, lowercase ISO-639, e.g. `en` or `ja`.
    pub lughat_masdar: String,
    /// How the credential is presented.
    pub tawtheeq: TawtheeqGoogle,
    /// The local request ceiling per minute.
    pub hadd_talabat: NonZeroU32,
    /// The price, per source character, in nano-dollars — $20 per million
    /// characters, the published v3 NMT rate as of [`TAREEKH_ASAR`]. Must
    /// stay character-priced, for the reason [`IdadatDeepL::taklifa`]
    /// documents.
    pub taklifa: NamudhajTaklifa,
}

impl IdadatGoogleSahabi {
    /// The published defaults for a project, global location, English source.
    #[must_use]
    pub fn li_mashru(mashru: impl Into<String>) -> Self {
        Self {
            mashru: mashru.into(),
            makan: None,
            lughat_masdar: "en".to_owned(),
            tawtheeq: TawtheeqGoogle::MiftahApi,
            hadd_talabat: hadd_thabit(300),
            taklifa: NamudhajTaklifa::BilAhruf { harf: 20_000 },
        }
    }
}

/// The Cloud Translation v3 provider.
///
/// ## Confidence
///
/// A v3 `translateText` reply carries `translatedText` and, when the source
/// was auto-detected, a `detectedLanguageCode` — no score of any kind for the
/// translation itself. This module also does not borrow a language-detection
/// score from anywhere as a stand-in: results are
/// [`NatijatTarjama::bila_thiqa`] and the capability declares it.
#[derive(Debug)]
pub struct MuzawwidGoogleSahabi {
    /// The configuration this instance was built with.
    idadat: IdadatGoogleSahabi,
    /// The credential. Present by construction.
    itimad: Itimad,
    /// The full endpoint, project and location baked in.
    unwan: String,
    /// The shared wire core.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
}

impl MuzawwidGoogleSahabi {
    /// The provider's stable name.
    pub const ISM: &'static str = "google-translation";

    /// Builds the provider — credential and confirmation required.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the project or location is
    /// not a plain URL path segment — both are spliced into the endpoint and
    /// come from configuration, and configuration is input — or when the
    /// HTTP client cannot be built.
    pub fn jadeed(
        idadat: IdadatGoogleSahabi,
        itimad: Itimad,
        idhn: IdhnInfaq,
    ) -> Result<Self, KhataTarjama> {
        if !juz_masar_salih(&idadat.mashru) || !idadat.makan.as_deref().is_none_or(juz_masar_salih)
        {
            return Err(KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: Self::ISM.to_owned(),
                sabab: "the project or location is not a plain URL path segment".to_owned(),
            });
        }
        let ab = match &idadat.makan {
            Some(makan) => format!("projects/{}/locations/{makan}", idadat.mashru),
            None => format!("projects/{}", idadat.mashru),
        };
        let unwan = format!("{ASAS_GOOGLE_SAHABI}/{ab}:translateText");
        let jawhar = JawharIrsal::jadeed(
            Self::ISM.to_owned(),
            false,
            idadat.hadd_talabat,
            TakalifJarya::min_idhn(idhn),
            AsmaHudud {
                baqi_talabat: None,
                iadat_dabt: None,
            },
        )?;
        let qudrat = QudratMuzawwid {
            dufaat: true,
            // The v3 guidance is thirty thousand codepoints per request;
            // bounding the UTF-8 *bytes* at the same figure is strictly
            // tighter, so the check cannot pass a request the guidance
            // refuses.
            aqsa_hajm_talab: 30_000,
            hadd_talabat: idadat.hadd_talabat,
            yublighu_thiqa: false,
            yaqbal_tawjih: false,
            taklifa: idadat.taklifa,
        };
        Ok(Self {
            idadat,
            itimad,
            unwan,
            jawhar,
            qudrat,
        })
    }
}

/// Whether a configured value can be spliced into a URL path.
fn juz_masar_salih(qeema: &str) -> bool {
    !qeema.is_empty()
        && qeema
            .chars()
            .all(|harf| harf.is_ascii_alphanumeric() || matches!(harf, '-' | '_'))
}

/// One `translateText` request, as this module sends it.
#[derive(Debug, Serialize)]
struct TalabGoogleSahabi<'talab> {
    /// The strings to translate; this module sends one.
    contents: Vec<&'talab str>,
    /// `text/plain` — the default is HTML, which would make the engine parse
    /// the string for tags it does not have.
    #[serde(rename = "mimeType")]
    naw_muhtawa: &'static str,
    /// The source language.
    #[serde(rename = "sourceLanguageCode")]
    lughat_masdar: &'talab str,
    /// Always `ar`.
    #[serde(rename = "targetLanguageCode")]
    lughat_hadaf: &'static str,
}

/// The `translateText` reply.
#[derive(Debug, Deserialize)]
struct RaddGoogleSahabi {
    /// One entry per input string.
    #[serde(default)]
    translations: Vec<TarjamatGoogleSahabi>,
}

/// One translation entry.
#[derive(Debug, Deserialize)]
struct TarjamatGoogleSahabi {
    /// The translated text.
    #[serde(rename = "translatedText", default)]
    nass: Option<String>,
    /// The detected source language, when detection ran. A language code,
    /// not a confidence — and not converted into one.
    #[serde(rename = "detectedLanguageCode", default)]
    lugha_muktashafa: Option<String>,
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidGoogleSahabi {
    fn ism(&self) -> &str {
        Self::ISM
    }

    fn namudhaj(&self) -> &'static str {
        "translate-v3"
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        _talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        if mahmi.matn().len() > self.qudrat.aqsa_hajm_talab {
            return Err(mudkhal_kabir(
                Self::ISM,
                mahmi.matn().len(),
                self.qudrat.aqsa_hajm_talab,
            ));
        }

        // Billed per character of the source; exact before sending.
        let ahruf = tul_u64(mahmi.matn().chars().count());
        let taqdir = self.idadat.taklifa.min_ahruf(ahruf);
        let hajz = self.jawhar.takalif.ihjiz(taqdir)?;

        let jasad = TalabGoogleSahabi {
            contents: vec![mahmi.matn()],
            naw_muhtawa: "text/plain",
            lughat_masdar: &self.idadat.lughat_masdar,
            lughat_hadaf: "ar",
        };

        let radd = self
            .jawhar
            .irsal(|amil| {
                let talab_khaam = amil.post(&self.unwan).json(&jasad);
                match self.idadat.tawtheeq {
                    TawtheeqGoogle::MiftahApi => {
                        talab_khaam.header("x-goog-api-key", self.itimad.qeema())
                    },
                    TawtheeqGoogle::HamilOauth => talab_khaam
                        .header("authorization", format!("Bearer {}", self.itimad.qeema())),
                }
            })
            .await?;

        // A 2xx means the counted characters were billed; settle
        // unconditionally from here.
        let nass_radd = match jasad_najah(radd, Self::ISM).await {
            Ok(nass) => nass,
            Err(khata) => {
                hajz.thabbit(taqdir);
                return Err(khata);
            },
        };
        hajz.thabbit(taqdir);
        let mufakkak: RaddGoogleSahabi =
            serde_json::from_str(&nass_radd).map_err(|_| KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            })?;

        let tarjama =
            mufakkak
                .translations
                .first()
                .ok_or_else(|| KhataTarjama::RaddGhayrMufassal {
                    muzawwid: Self::ISM.to_owned(),
                    radd: nass_radd.chars().take(64).collect(),
                })?;
        let nass = tarjama.nass.as_deref().unwrap_or_default();
        if nass.trim().is_empty() {
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: "(the translation came back empty)".to_owned(),
            });
        }
        if let Some(lugha) = &tarjama.lugha_muktashafa {
            tracing::trace!(muzawwid = Self::ISM, lugha, "detected source language");
        }

        // v3 reports no translation confidence at all; none is carried and
        // none is borrowed from language detection — the refusal the module
        // header names.
        Ok(NatijatTarjama::bila_thiqa(nass.to_owned(), taqdir))
    }
}

// ---------------------------------------------------------------------------
// Microsoft Translator — /translate?api-version=3.0
// ---------------------------------------------------------------------------

/// The Microsoft Translator endpoint.
pub const UNWAN_MICROSOFT: &str = "https://api.cognitive.microsofttranslator.com/translate";

/// Configuration for the Microsoft Translator provider.
#[derive(Debug, Clone)]
pub struct IdadatMicrosoft {
    /// The source language code, lowercase, e.g. `en` or `ja`.
    pub lughat_masdar: String,
    /// The resource's region, e.g. `westeurope`, sent as
    /// `Ocp-Apim-Subscription-Region`. Required for regional resources and
    /// absent for global ones, which is why it is optional here.
    pub mintaqa: Option<String>,
    /// The local request ceiling per minute.
    pub hadd_talabat: NonZeroU32,
    /// The price, per billed character, in nano-dollars — $10 per million
    /// characters, the published S1 pay-as-you-go rate as of
    /// [`TAREEKH_ASAR`]. Must stay character-priced, for the reason
    /// [`IdadatDeepL::taklifa`] documents.
    pub taklifa: NamudhajTaklifa,
}

impl Default for IdadatMicrosoft {
    fn default() -> Self {
        Self {
            lughat_masdar: "en".to_owned(),
            mintaqa: None,
            hadd_talabat: hadd_thabit(100),
            taklifa: NamudhajTaklifa::BilAhruf { harf: 10_000 },
        }
    }
}

/// The Microsoft Translator provider.
///
/// ## Confidence
///
/// The reply's only score is `detectedLanguage.score`, which measures how
/// sure the service is about *which language the source is* — nothing about
/// the translation. Presenting it as translation confidence would be exactly
/// the synthetic number this crate refuses everywhere, so results are
/// [`NatijatTarjama::bila_thiqa`] and the score is read for the trace log
/// only.
///
/// ## Cost settlement
///
/// Uniquely among the dedicated APIs, Microsoft states the billed figure on
/// the reply: the `X-metered-usage` header carries the number of characters
/// charged. The reservation is settled to that figure when it is present and
/// readable, so the meter shows the provider's own arithmetic rather than
/// this module's.
#[derive(Debug)]
pub struct MuzawwidMicrosoft {
    /// The configuration this instance was built with.
    idadat: IdadatMicrosoft,
    /// The credential. Present by construction.
    itimad: Itimad,
    /// The shared wire core.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
}

impl MuzawwidMicrosoft {
    /// The provider's stable name.
    pub const ISM: &'static str = "microsoft";

    /// Builds the provider — credential and confirmation required.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the configured region is not
    /// a plain header token, or when the HTTP client cannot be built.
    pub fn jadeed(
        idadat: IdadatMicrosoft,
        itimad: Itimad,
        idhn: IdhnInfaq,
    ) -> Result<Self, KhataTarjama> {
        if !idadat.mintaqa.as_deref().is_none_or(juz_masar_salih) {
            return Err(KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: Self::ISM.to_owned(),
                sabab: "the region is not a plain header token".to_owned(),
            });
        }
        let jawhar = JawharIrsal::jadeed(
            Self::ISM.to_owned(),
            false,
            idadat.hadd_talabat,
            TakalifJarya::min_idhn(idhn),
            AsmaHudud {
                baqi_talabat: None,
                iadat_dabt: None,
            },
        )?;
        let qudrat = QudratMuzawwid {
            dufaat: true,
            // The service limit is fifty thousand characters per request
            // across the array; bytes bound characters from above.
            aqsa_hajm_talab: 50_000,
            hadd_talabat: idadat.hadd_talabat,
            yublighu_thiqa: false,
            yaqbal_tawjih: false,
            taklifa: idadat.taklifa,
        };
        Ok(Self {
            idadat,
            itimad,
            jawhar,
            qudrat,
        })
    }
}

/// One element of the request array.
#[derive(Debug, Serialize)]
struct NassMicrosoft<'talab> {
    /// The string to translate; the field name is the API's own.
    #[serde(rename = "Text")]
    nass: &'talab str,
}

/// One element of the reply array.
#[derive(Debug, Deserialize)]
struct RaddMicrosoft {
    /// The translations for this input, one per requested target.
    #[serde(default)]
    translations: Vec<TarjamatMicrosoft>,
    /// Present when the source language was auto-detected.
    #[serde(rename = "detectedLanguage", default)]
    lugha_muktashafa: Option<KashfLughaMicrosoft>,
}

/// One translation entry.
#[derive(Debug, Deserialize)]
struct TarjamatMicrosoft {
    /// The translated text.
    #[serde(default)]
    text: Option<String>,
}

/// The language-detection report.
#[derive(Debug, Deserialize)]
struct KashfLughaMicrosoft {
    /// The detected language code.
    #[serde(default)]
    language: Option<String>,
    /// Confidence **in the detection**, zero to one. Logged, never promoted
    /// into a translation confidence — see the provider's documentation.
    #[serde(default)]
    score: Option<f64>,
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidMicrosoft {
    fn ism(&self) -> &str {
        Self::ISM
    }

    fn namudhaj(&self) -> &'static str {
        "translator-v3.0"
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        _talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        if mahmi.matn().len() > self.qudrat.aqsa_hajm_talab {
            return Err(mudkhal_kabir(
                Self::ISM,
                mahmi.matn().len(),
                self.qudrat.aqsa_hajm_talab,
            ));
        }

        let ahruf = tul_u64(mahmi.matn().chars().count());
        let taqdir = self.idadat.taklifa.min_ahruf(ahruf);
        let hajz = self.jawhar.takalif.ihjiz(taqdir)?;

        let jasad = vec![NassMicrosoft { nass: mahmi.matn() }];

        let radd = self
            .jawhar
            .irsal(|amil| {
                let talab_khaam = amil
                    .post(UNWAN_MICROSOFT)
                    .query(&[
                        ("api-version", "3.0"),
                        ("from", self.idadat.lughat_masdar.as_str()),
                        ("to", "ar"),
                        ("textType", "plain"),
                    ])
                    .header("ocp-apim-subscription-key", self.itimad.qeema())
                    .json(&jasad);
                match &self.idadat.mintaqa {
                    Some(mintaqa) => talab_khaam.header("ocp-apim-subscription-region", mintaqa),
                    None => talab_khaam,
                }
            })
            .await?;

        // The billed figure, from the provider's own accounting header, read
        // before the body consumes the response.
        let mahsub = qeemat_ras_u64(&radd, "x-metered-usage")
            .map(|ahruf_mahsuba| self.idadat.taklifa.min_ahruf(ahruf_mahsuba));
        let fili = mahsub.unwrap_or(taqdir);

        // A 2xx was billed whatever the body holds; settle unconditionally.
        let nass_radd = match jasad_najah(radd, Self::ISM).await {
            Ok(nass) => nass,
            Err(khata) => {
                hajz.thabbit(fili);
                return Err(khata);
            },
        };
        hajz.thabbit(fili);
        let mufakkak: Vec<RaddMicrosoft> =
            serde_json::from_str(&nass_radd).map_err(|_| KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            })?;

        let awwal = mufakkak.first().and_then(|radd_wahid| {
            radd_wahid
                .translations
                .first()
                .map(|tarjama| (radd_wahid, tarjama))
        });
        let Some((radd_wahid, tarjama)) = awwal else {
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: nass_radd.chars().take(64).collect(),
            });
        };

        let nass = tarjama.text.as_deref().unwrap_or_default();
        if nass.trim().is_empty() {
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: "(the translation came back empty)".to_owned(),
            });
        }
        if let Some(kashf) = &radd_wahid.lugha_muktashafa {
            let lugha = kashf.language.as_deref().unwrap_or("?");
            let daraja = kashf.score.unwrap_or_default();
            tracing::trace!(muzawwid = Self::ISM, lugha, daraja, "language detection");
        }

        // `detectedLanguage.score` measures detection, not translation; the
        // result carries no confidence — the refusal the module header names.
        Ok(NatijatTarjama::bila_thiqa(nass.to_owned(), fili))
    }
}

// ---------------------------------------------------------------------------
// Google Translate's free web endpoint — translate_a/single?client=gtx
// ---------------------------------------------------------------------------

/// The free web endpoint: the request translate.google.com's own widget makes,
/// and the one Unity translation mods have made for a decade.
pub const UNWAN_GOOGLE_MAJJANI: &str = "https://translate.googleapis.com/translate_a/single";

/// The day the free endpoint's request and reply shapes were last checked
/// live.
///
/// A `GET` carrying `client=gtx&sl=auto&tl=ar&dt=t&q=…` answered a JSON array
/// whose first element is a list of `[translated, source, …]` sentence
/// segments and whose third is the detected source language, and a `⟦0⟧`
/// token in the text came back intact. There is no published page to check
/// against and no contract behind the shape; a release far past this date
/// should send one request by hand before trusting [`tarjama_min_gtx`].
pub const TAREEKH_ASAR_GOOGLE_MAJJANI: &str = "2026-09-12";

/// The most text one request carries, in characters.
///
/// Five thousand, the figure the widget's own text box stops at. The text
/// travels in the query string of a `GET`, so the ceiling is also what keeps
/// the URL inside what the endpoint accepts. A longer string is refused before
/// anything is sent — [`KhataTarjama::MajjaniTawil`] — so it costs no round
/// trip and does not disturb the spacing the other strings depend on.
pub const AQSA_AHRUF_GOOGLE_MAJJANI: usize = 5000;

/// The shortest gap between two requests.
///
/// Four hundred milliseconds, measured rather than guessed. At a hundred and
/// fifty, with four requests in flight, a run bursts at twenty-odd a second and
/// the endpoint's abuse detection refuses the machine within the first few
/// hundred strings — which ends the run with nothing compiled, because a
/// translation stage that answered no string has nothing to give the compiler.
/// At this spacing, with the concurrency below, a whole game went through in
/// one pass: three thousand seven hundred and eighteen strings, no refusal.
///
/// Slower is the point. A free endpoint that finishes in twenty-five minutes is
/// worth more than one that stops in ninety seconds.
pub const FASL_GOOGLE_MAJJANI: Duration = Duration::from_millis(400);

/// The per-minute ceiling [`FASL_GOOGLE_MAJJANI`] amounts to, for the batch
/// layer's own limiter, which counts per minute.
pub const HADD_TALABAT_GOOGLE_MAJJANI: u32 = 150;

/// How many requests may be in flight at once, whatever the caller's own
/// concurrency is.
///
/// Two. Four multiplied the burst rate by four against a service that watches
/// spacing, and the gap above is only a floor between *starts* — four in flight
/// means four arrive inside one gap.
pub const TAWAZI_GOOGLE_MAJJANI: usize = 2;

/// The shortest wait before a `429` is retried.
///
/// Two minutes. The endpoint's `429` carries no `Retry-After`, so the shared
/// schedule alone would come back after half a second — which, against abuse
/// detection, is the behaviour that turns a warning into a block.
///
/// It was five seconds, and five seconds is not a cool-down: this service's
/// refusal clears in minutes, so four attempts twenty seconds apart all landed
/// inside the same refusal and the run ended having translated nothing. Waiting
/// two minutes between attempts costs a run that would have failed eight
/// minutes, and saves the whole translation.
pub const ARD_MUADAL_GOOGLE_MAJJANI: Duration = Duration::from_secs(120);

/// The `sl` value that asks the endpoint to detect the source language.
pub const LUGHAT_MASDAR_TILQAIYA: &str = "auto";

/// Configuration for the free web endpoint.
///
/// Small on purpose: there is no model to pick, no price to override and no
/// credential to file. The address is configuration only so that a test can
/// stand a loopback server in its place, and the two timing values so that the
/// same test does not wait five seconds per retry.
#[derive(Debug, Clone)]
pub struct IdadatGoogleMajjani {
    /// The endpoint. Loopback over plain HTTP is accepted, for tests; any
    /// other host must be `https`.
    pub asas: String,
    /// The source language code sent as `sl`, or [`LUGHAT_MASDAR_TILQAIYA`]
    /// to let the endpoint detect it — the default, because nothing in the
    /// settings says what language a game is in, and the detected code comes
    /// back on every reply anyway.
    pub lughat_masdar: String,
    /// The gap between requests; see [`FASL_GOOGLE_MAJJANI`].
    pub fasl: Duration,
    /// The floor on the wait after a `429`; see [`ARD_MUADAL_GOOGLE_MAJJANI`].
    pub ard_muadal: Duration,
}

impl Default for IdadatGoogleMajjani {
    fn default() -> Self {
        Self {
            asas: UNWAN_GOOGLE_MAJJANI.to_owned(),
            lughat_masdar: LUGHAT_MASDAR_TILQAIYA.to_owned(),
            fasl: FASL_GOOGLE_MAJJANI,
            ard_muadal: ARD_MUADAL_GOOGLE_MAJJANI,
        }
    }
}

/// Google Translate's free web endpoint.
///
/// ## What it is, and is not
///
/// It is the request the translate.google.com widget makes, reached without a
/// key by a client that identifies itself. It is not an API: nothing is
/// documented, nothing is promised, and the shape read below was observed on
/// [`TAREEKH_ASAR_GOOGLE_MAJJANI`] rather than read off a contract. It is in
/// this module because the alternative was a product that could not translate
/// one string until its user held a paid account, and because the mods this
/// product replaces have run against exactly this endpoint for years. It is
/// what a fresh installation translates with; a provider the user configures
/// takes precedence the moment it is switched on.
///
/// ## One string per request
///
/// The reply splits the text into sentence segments and nothing marks where
/// one input would end and the next begin, so joining several strings into one
/// request and splitting the answer back would attach translations to the
/// wrong strings — silently, the failure [`Muzawwid::tarjim_dufa`] warns of.
/// So `dufaat` is `false` and every string is its own request, spaced by
/// [`FASL_GOOGLE_MAJJANI`] and held to [`TAWAZI_GOOGLE_MAJJANI`] in flight by
/// the provider's own semaphore: the batch layer bounds a run as well, but the
/// endpoint's tolerance is this provider's fact to enforce, not a caller's to
/// remember.
///
/// ## What is not sent
///
/// No instruction, no glossary, no surrounding lines: the endpoint has nowhere
/// to put them, and [`QudratMuzawwid::yaqbal_tawjih`] says so rather than
/// pretending. Placeholders reach it as `hima` tokens exactly as they reach
/// every other provider — protection is applied by the caller around
/// [`Muzawwid::tarjim`], and this provider neither adds a layer nor skips one.
///
/// ## Confidence and cost
///
/// The reply carries no confidence and none is invented. The meter is
/// [`TakalifJarya::majani`] and every result settles at zero.
#[derive(Debug)]
pub struct MuzawwidGoogleMajjani {
    /// The configuration this instance was built with.
    idadat: IdadatGoogleMajjani,
    /// The shared wire core, with the request gap, the `429` floor and the
    /// credential-free refusal mapping set.
    jawhar: JawharIrsal,
    /// The declared capabilities, built once.
    qudrat: QudratMuzawwid,
    /// The in-flight bound.
    tawazi: tokio::sync::Semaphore,
    /// The `User-Agent` sent on every request: the conventional compatible
    /// form, naming this product and its version. The endpoint is a web
    /// endpoint and is spoken to as one, by a client that says who it is.
    wakeel: String,
}

impl MuzawwidGoogleMajjani {
    /// The provider's stable name.
    pub const ISM: &'static str = "google-majjani";

    /// The "model" recorded as provenance: the endpoint's own client
    /// identifier, which is the only one it has.
    pub const NAMUDHAJ: &'static str = "gtx";

    /// Builds the provider — no credential and no confirmation, because there
    /// is no account to bill.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::MuzawwidGhayrMutah`] when the endpoint address does not
    /// parse, names a remote host over plaintext — the text of a game is not
    /// the network's to read in transit — or when the HTTP client cannot be
    /// built.
    pub fn jadeed(idadat: IdadatGoogleMajjani) -> Result<Self, KhataTarjama> {
        let rabt = rabt_salih(Self::ISM, &idadat.asas)?;
        // Same list, same reason, as `MuzawwidMuwafiqOpenAI::mahalli`: loopback
        // by name, because a hostname that resolves locally today is a config
        // edit away from somewhere else tomorrow.
        let mudif = rabt.host_str().unwrap_or_default();
        let mahalli = matches!(mudif, "127.0.0.1" | "localhost" | "::1" | "[::1]");
        if !mahalli && rabt.scheme() != "https" {
            return Err(KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: Self::ISM.to_owned(),
                sabab: "a remote endpoint must be https; plaintext would put the game's text \
                        on the wire readable"
                    .to_owned(),
            });
        }
        let jawhar = JawharIrsal::jadeed(
            Self::ISM.to_owned(),
            mahalli,
            hadd_thabit(HADD_TALABAT_GOOGLE_MAJJANI),
            TakalifJarya::majani(),
            AsmaHudud {
                baqi_talabat: None,
                iadat_dabt: None,
            },
        )?
        .bi_fasl(idadat.fasl)
        .bi_ard_muadal(idadat.ard_muadal)
        .bila_itimad();
        let qudrat = QudratMuzawwid {
            dufaat: false,
            // The ceiling is in characters, and a character is at most four
            // bytes of UTF-8; stated in bytes here because that is the unit
            // the capability speaks, and stated loosely because the exact
            // refusal is `tarjim`'s, in characters.
            aqsa_hajm_talab: AQSA_AHRUF_GOOGLE_MAJJANI.saturating_mul(4),
            hadd_talabat: hadd_thabit(HADD_TALABAT_GOOGLE_MAJJANI),
            yublighu_thiqa: false,
            yaqbal_tawjih: false,
            taklifa: NamudhajTaklifa::Majani,
        };
        Ok(Self {
            idadat,
            jawhar,
            qudrat,
            tawazi: tokio::sync::Semaphore::new(TAWAZI_GOOGLE_MAJJANI),
            wakeel: format!("Mozilla/5.0 (compatible; Taarib/{})", taarib_usus::ISDAR),
        })
    }
}

/// The free endpoint's reply, reduced to what this module reads.
#[derive(Debug, PartialEq, Eq)]
struct RaddGtx {
    /// Every sentence segment's translation, concatenated in order.
    tarjama: String,
    /// What the endpoint believed the source language was. Logged only; it
    /// is not a confidence and is not treated as one.
    lugha_muktashafa: Option<String>,
}

/// Reads the free endpoint's positional reply.
///
/// `[[[translated, source, …], …], null, detected, …]`: the translation is the
/// concatenation of every segment's first element over the first array, and
/// the detected language sits third. Segments whose first element is not text
/// are skipped rather than failed — the endpoint pads with `null` entries for
/// data this module does not request — and an array with no text at all is
/// reported as an empty translation, which the caller refuses. [`None`] when
/// the value is not the shape at all: no leading array, or a first element
/// that is not a list.
fn tarjama_min_gtx(qeema: &Value) -> Option<RaddGtx> {
    let maqati = qeema.get(0)?.as_array()?;
    let mut tarjama = String::new();
    for maqta in maqati {
        if let Some(juz) = maqta.get(0).and_then(Value::as_str) {
            tarjama.push_str(juz);
        }
    }
    let lugha_muktashafa = qeema.get(2).and_then(Value::as_str).map(str::to_owned);
    Some(RaddGtx {
        tarjama,
        lugha_muktashafa,
    })
}

/// Whether a `2xx` body is markup rather than the JSON array.
///
/// The endpoint's block page — "our systems have detected unusual traffic" —
/// is HTML, and it has been seen behind a success status as well as behind a
/// `429`. The JSON reply always opens with `[`, so a body opening with `<` is
/// a page, whatever its status said.
fn safha_la_json(jasad: &str) -> bool {
    jasad.trim_start().starts_with('<')
}

#[async_trait::async_trait]
impl Muzawwid for MuzawwidGoogleMajjani {
    fn ism(&self) -> &str {
        Self::ISM
    }

    fn namudhaj(&self) -> &'static str {
        Self::NAMUDHAJ
    }

    fn qudrat(&self) -> QudratMuzawwid {
        self.qudrat.clone()
    }

    fn takalif(&self) -> &TakalifJarya {
        &self.jawhar.takalif
    }

    async fn tarjim(
        &self,
        mahmi: &NassMahmi,
        _talab: &SiyaqTalab,
    ) -> Result<NatijatTarjama, KhataTarjama> {
        let ahruf = mahmi.matn().chars().count();
        if ahruf > AQSA_AHRUF_GOOGLE_MAJJANI {
            return Err(KhataTarjama::MajjaniTawil {
                muzawwid: Self::ISM.to_owned(),
                ahruf,
                saqf: AQSA_AHRUF_GOOGLE_MAJJANI,
            });
        }

        // Held for the whole request, including its retries: the bound is on
        // what is in flight against the endpoint, not on what has been asked.
        // The semaphore is never closed, so the failure arm is unreachable in
        // practice; it is mapped rather than unwrapped because this crate runs
        // inside other people's game processes and does not panic there.
        let _idhn = self
            .tawazi
            .acquire()
            .await
            .map_err(|_| KhataTarjama::MuzawwidGhayrMutah {
                muzawwid: Self::ISM.to_owned(),
                sabab: "the in-flight bound was closed".to_owned(),
            })?;

        // No reservation: the meter is the free one and nothing settles
        // against it. `tl` is lowercase `ar` — the endpoint's own spelling,
        // not the ISO uppercase `DeepL` takes.
        let radd = self
            .jawhar
            .irsal(|amil| {
                amil.get(&self.idadat.asas)
                    .query(&[
                        ("client", Self::NAMUDHAJ),
                        ("sl", self.idadat.lughat_masdar.as_str()),
                        ("tl", "ar"),
                        ("dt", "t"),
                        ("q", mahmi.matn()),
                    ])
                    .header("user-agent", &self.wakeel)
            })
            .await?;

        let nass_radd = jasad_najah(radd, Self::ISM).await?;
        if nass_radd.trim().is_empty() {
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: "(the reply body is empty)".to_owned(),
            });
        }
        if safha_la_json(&nass_radd) {
            return Err(KhataTarjama::MajjaniMahjub {
                muzawwid: Self::ISM.to_owned(),
                sabab: "the reply is an HTML page where JSON was expected, which is the block \
                        page the service serves a client it has flagged"
                    .to_owned(),
            });
        }
        let ghayr_mufassal = || KhataTarjama::RaddGhayrMufassal {
            muzawwid: Self::ISM.to_owned(),
            radd: nass_radd.chars().take(64).collect(),
        };
        let qeema: Value = serde_json::from_str(&nass_radd).map_err(|_| ghayr_mufassal())?;
        let RaddGtx {
            tarjama,
            lugha_muktashafa,
        } = tarjama_min_gtx(&qeema).ok_or_else(ghayr_mufassal)?;
        if tarjama.trim().is_empty() {
            return Err(KhataTarjama::RaddGhayrMufassal {
                muzawwid: Self::ISM.to_owned(),
                radd: "(the translation came back empty)".to_owned(),
            });
        }
        if let Some(lugha) = &lugha_muktashafa {
            tracing::trace!(muzawwid = Self::ISM, lugha, "detected source language");
        }

        // Nothing in the reply measures the translation, so nothing is
        // carried — the refusal the module header names, one more time.
        Ok(NatijatTarjama::bila_thiqa(tarjama, 0))
    }
}

// ---------------------------------------------------------------------------
// الاختبارات — the money and the credential, proved without spending either
// ---------------------------------------------------------------------------

/// Tests for the two things in this module that cost a stranger something when
/// they are wrong: the spending ceiling and the credential's redaction.
///
/// Inside the module rather than in `tests/` deliberately. Both guarantees are
/// enforced by *private* state — [`Itimad::qeema`] has no public reader and
/// [`TalabAnthropic`] has no public constructor — so an integration test could
/// only check them through a live request, which is the one thing a test of
/// this module must never make. From in here they are checked directly, and
/// every one of these runs with no network, no keychain and no key.
#[cfg(test)]
mod ikhtibarat {
    use super::*;

    /// A credential for testing, built past the store the real constructors go
    /// through. Only this module can, which is the point being tested.
    fn itimad_wahmi(qeema: &str) -> Itimad {
        Itimad {
            qeema: qeema.to_owned(),
        }
    }

    /// A `#[derive(Debug)]` struct holding a credential — the shape a
    /// provider is, and the path a key would take to a log.
    #[derive(Debug)]
    struct Hamil {
        /// Never read by name: the derived `Debug` is what is under test.
        #[expect(dead_code, reason = "the derived Debug is what is under test")]
        itimad: Itimad,
    }

    #[test]
    fn itimad_la_yutba_abadan() {
        let sirr = "sk-ant-api03-REAL-LOOKING-SECRET-VALUE";
        let itimad = itimad_wahmi(sirr);

        // Both formatters, and the check is on the whole rendering rather than
        // on "does it contain the secret": a redaction that leaked a prefix or
        // a length would pass a containment check and still identify the
        // account class and the key kind.
        assert_eq!(format!("{itimad:?}"), "Itimad(<redacted>)");
        assert_eq!(format!("{itimad}"), "Itimad(<redacted>)");

        // The same through a `#[derive(Debug)]` container, which is how one
        // would actually reach a log: a provider struct printed whole.
        let matbu = format!("{:?}", Hamil { itimad });
        assert!(!matbu.contains(sirr));
        assert!(!matbu.contains("sk-ant"));
        assert!(matbu.contains("<redacted>"));
    }

    #[test]
    fn alhajz_yarfud_ind_alsaqf_la_baadah() {
        let hisab = TakalifJarya::min_idhn(IdhnInfaq::baad_taakid(100, 0));

        let awwal = hisab.ihjiz(60);
        assert!(awwal.is_ok());
        // 60 + 60 crosses 100, so it is the reservation that is refused —
        // before anything is sent, which is what "stops at the ceiling" means.
        assert!(matches!(
            hisab.ihjiz(60),
            Err(KhataTarjama::SaqfTakalif { .. })
        ));
        // Exactly at the ceiling is allowed; only past it is not.
        assert!(hisab.ihjiz(40).is_ok());
        drop(awwal);
    }

    #[test]
    fn alhajz_yutlaq_ind_isqatih_bila_tathbeet() {
        let hisab = TakalifJarya::min_idhn(IdhnInfaq::baad_taakid(100, 0));
        {
            let _hajz = hisab.ihjiz(100);
            assert_eq!(hisab.munfaq(), 100);
        }
        // Dropped unsettled: nothing was sent, so nothing is owed and the
        // whole ceiling is available again.
        assert_eq!(hisab.munfaq(), 0);
    }

    #[test]
    fn attathbeet_yastabdil_altaqdir_bilfili() {
        let hisab = TakalifJarya::min_idhn(IdhnInfaq::baad_taakid(1_000, 0));
        let hajz = hisab.ihjiz(500);
        assert!(hajz.is_ok());
        if let Ok(hajz) = hajz {
            hajz.thabbit(120);
        }
        // Reservation out, actual in, in one transition — not 500 removed and
        // 120 added as two visible states.
        assert_eq!(hisab.munfaq(), 120);
    }

    #[test]
    fn assaqf_yasmud_taht_altawazi() {
        // The regression this exists for: settlement used to be a `fetch_sub`
        // followed by a `fetch_add`, and between them the meter read low by a
        // whole reservation. A parallel `ihjiz` landing in that window was
        // measured against a total about to be restored, so it could be
        // admitted when the true total left no room — the ceiling crossed by
        // up to one request per race. Reserving and settling the same amount
        // makes every cycle a clean `+wahda`, so a correct meter admits
        // exactly `SAQF / wahda` of them however the threads interleave, and
        // an over-admission is a request that would have been sent past a
        // ceiling the user was promised.
        //
        // Contended deliberately: every thread hammers reserve-then-settle
        // with no work between, so settlements are constantly in flight while
        // other threads are reading the meter. A gentler loop reproduces
        // nothing, which is exactly how the defect survived being read — the
        // first version of this test did eight cycles per thread, passed
        // against the broken code, and proved only that the test was too
        // small. Against the two-step settlement this shape reports 40001
        // admissions for a 40000 ceiling; the rounds are there because one
        // round catches it most of the time rather than every time, and a
        // regression test for a race should not be a coin flip.
        const SAQF: u64 = 40_000;
        const WAHDA: u64 = 1;

        for _ in 0..4_u32 {
            let hisab = TakalifJarya::min_idhn(IdhnInfaq::baad_taakid(SAQF, 0));
            let najah = AtomicU64::new(0);

            std::thread::scope(|nitaq| {
                for _ in 0..8_u32 {
                    let _ = nitaq.spawn(|| {
                        while let Ok(hajz) = hisab.ihjiz(WAHDA) {
                            let _ = najah.fetch_add(1, Ordering::SeqCst);
                            hajz.thabbit(WAHDA);
                        }
                    });
                }
            });

            assert_eq!(
                najah.load(Ordering::SeqCst),
                SAQF,
                "reservations admitted past the ceiling"
            );
            assert_eq!(hisab.munfaq(), SAQF);
            assert!(hisab.munfaq() <= hisab.saqf());
        }
    }

    #[test]
    fn taqdir_arrumuz_yubalighu_wala_yuqallil() {
        // The divisor's whole job is to err high; a reservation built on an
        // under-count is a ceiling that lets a request past it.
        assert_eq!(taqdir_rumuz_min_bayt(0), 0);
        assert_eq!(taqdir_rumuz_min_bayt(1), 1);
        assert_eq!(taqdir_rumuz_min_bayt(3), 1);
        assert_eq!(taqdir_rumuz_min_bayt(4), 2);
    }

    /// The output cost of a price, for comparing table entries.
    fn ikhraj(taklifa: NamudhajTaklifa) -> u64 {
        taklifa.min_rumuz(0, 1)
    }

    /// The input cost of a price, for comparing table entries.
    fn idkhal(taklifa: NamudhajTaklifa) -> u64 {
        taklifa.min_rumuz(1, 0)
    }

    #[test]
    fn almajhul_yusaar_bi_aala_madkhal_fi_kul_jadwal() {
        // Every price table documents the same invariant — an identifier the
        // table does not name is priced at the highest entry, because the
        // number guards a ceiling and the safe direction for an unknown is
        // over. The Anthropic table quietly stopped satisfying its own claim
        // when a model twice the price of Opus was published and the fallback
        // stayed at the Opus figure, so the claim is checked here rather than
        // asserted in prose.
        let masmuha = [
            "claude-haiku-4-5-20251001",
            "claude-sonnet-5",
            "claude-sonnet-4-5",
            "claude-opus-5",
            "claude-fable-5-1",
        ];
        let majhul_anthropic = taklifat_anthropic("a-model-nobody-has-shipped-yet");
        for ism in masmuha {
            let maruf = taklifat_anthropic(ism);
            assert!(
                idkhal(maruf) <= idkhal(majhul_anthropic),
                "input price of {ism}"
            );
            assert!(
                ikhraj(maruf) <= ikhraj(majhul_anthropic),
                "output price of {ism}"
            );
        }

        let asmaa_openai = [
            "gpt-5",
            "gpt-5-mini",
            "gpt-5-nano",
            "gpt-5.1",
            "gpt-5.2",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-5.5",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-6-astra",
        ];
        let majhul_openai = taklifat_openai("some-gateway-model");
        for ism in asmaa_openai {
            let maruf = taklifat_openai(ism);
            assert!(
                idkhal(maruf) <= idkhal(majhul_openai),
                "input price of {ism}"
            );
            assert!(
                ikhraj(maruf) <= ikhraj(majhul_openai),
                "output price of {ism}"
            );
        }

        let asmaa_gemini = [
            "gemini-2.5-flash",
            "gemini-2.5-pro",
            "gemini-3.8-flash",
            "gemini-3.6-flash",
        ];
        let majhul_gemini = taklifat_gemini("gemini-99-ultra");
        for ism in asmaa_gemini {
            let maruf = taklifat_gemini(ism);
            assert!(
                idkhal(maruf) <= idkhal(majhul_gemini),
                "input price of {ism}"
            );
            assert!(
                ikhraj(maruf) <= ikhraj(majhul_gemini),
                "output price of {ism}"
            );
        }
    }

    #[test]
    fn jadawil_alasaar_tumayyiz_alajyal() {
        // A substring table is one careless arm away from pricing the wrong
        // generation: `gemini-2.5-flash` contains `flash`, `gpt-5.4-mini`
        // contains `gpt-5`, and `claude-sonnet-5` contains `sonnet`.
        assert_eq!(
            taklifat_gemini("gemini-2.5-flash"),
            NamudhajTaklifa::BilRumuz {
                idkhal: 300,
                ikhraj: 2_500
            }
        );
        assert_eq!(
            taklifat_gemini("gemini-3.8-flash"),
            NamudhajTaklifa::BilRumuz {
                idkhal: 1_500,
                ikhraj: 7_500
            }
        );
        assert_eq!(
            taklifat_openai("gpt-5.4-mini"),
            NamudhajTaklifa::BilRumuz {
                idkhal: 750,
                ikhraj: 4_500
            }
        );
        assert_eq!(
            taklifat_openai("gpt-5"),
            NamudhajTaklifa::BilRumuz {
                idkhal: 1_250,
                ikhraj: 10_000
            }
        );
        assert_eq!(
            taklifat_anthropic("claude-sonnet-5"),
            NamudhajTaklifa::BilRumuz {
                idkhal: 2_000,
                ikhraj: 10_000
            }
        );
        assert_eq!(
            taklifat_anthropic("claude-sonnet-4-5"),
            NamudhajTaklifa::BilRumuz {
                idkhal: 3_000,
                ikhraj: 15_000
            }
        );
    }

    #[test]
    fn alhararat_tughfal_lima_yarfuduha() {
        // Anthropic answers a non-default temperature with HTTP 400 from Opus
        // 4.7 onwards, and this module's shared wire code reads a 400 as a
        // refusal of the *input* — so sending it would fail every string of a
        // run with a message blaming text that was never the problem. The
        // unknown case omits, which is always accepted.
        assert_eq!(
            hararat_anthropic("claude-sonnet-4-5"),
            Some(HARARAT_TARJAMA)
        );
        assert_eq!(
            hararat_anthropic("claude-haiku-4-5-20251001"),
            Some(HARARAT_TARJAMA)
        );
        assert_eq!(hararat_anthropic("claude-opus-5"), None);
        assert_eq!(hararat_anthropic("claude-sonnet-5"), None);
        assert_eq!(hararat_anthropic("claude-fable-5-1"), None);
        assert_eq!(hararat_anthropic("a-model-nobody-has-shipped-yet"), None);

        // Gemini refuses nothing, but a temperature under the default is what
        // Google's own Gemini 3 guidance says causes looping — a reply that
        // bills and is then thrown away.
        assert_eq!(hararat_gemini("gemini-2.5-pro"), Some(HARARAT_TARJAMA));
        assert_eq!(hararat_gemini("gemini-2.5-flash"), Some(HARARAT_TARJAMA));
        assert_eq!(hararat_gemini("gemini-3.8-flash"), None);
        assert_eq!(hararat_gemini("gemini-3.1-pro-preview"), None);
        assert_eq!(hararat_gemini("gemini-99-ultra"), None);

        // The shipped defaults must agree with the tables they are built from.
        let anthropic = IdadatAnthropic::default();
        assert_eq!(anthropic.hararat, hararat_anthropic(&anthropic.namudhaj));
        assert_eq!(anthropic.taklifa, taklifat_anthropic(&anthropic.namudhaj));
        let gemini = IdadatGemini::default();
        assert_eq!(gemini.hararat, hararat_gemini(&gemini.namudhaj));
        assert_eq!(gemini.taklifa, taklifat_gemini(&gemini.namudhaj));
        // The platform default omits; the loopback default sends, because a
        // local model accepts it and costs nothing to get wrong.
        assert_eq!(IdadatMuwafiqOpenAI::openai().hararat, None);
        assert_eq!(
            IdadatMuwafiqOpenAI::ollama("qwen3").hararat,
            Some(HARARAT_TARJAMA)
        );
    }

    #[test]
    fn haql_alhararat_yaghib_min_aljasad_la_yursal_faarighan() {
        // `Option` alone is not the guarantee: serialised without
        // `skip_serializing_if` a `None` becomes `"temperature": null`, which
        // is a value the API reads and refuses exactly like `0.2`. So the
        // check is on the body that actually goes on the wire.
        let mukhattat = siyaq::mukhattat_sarim(true);
        let bina = |hararat: Option<f32>| {
            serde_json::to_value(TalabAnthropic {
                model: "claude-sonnet-5",
                max_tokens: 16,
                system: "s",
                messages: vec![RisalaAnthropic {
                    role: "user",
                    content: "u",
                }],
                tools: vec![AdatAnthropic {
                    name: ISM_ADAT_TARJAMA,
                    description: "d",
                    input_schema: &mukhattat,
                }],
                tool_choice: json!({ "type": "tool", "name": ISM_ADAT_TARJAMA }),
                temperature: hararat,
            })
            .ok()
        };
        assert_eq!(
            bina(None).and_then(|jasad| jasad.get("temperature").cloned()),
            None,
            "a None temperature must leave the Anthropic body entirely"
        );
        assert!(
            bina(Some(HARARAT_TARJAMA))
                .and_then(|jasad| jasad.get("temperature").cloned())
                .is_some()
        );

        let bina_muwafiq = |hararat: Option<f32>| {
            serde_json::to_value(TalabMuwafiq {
                model: "gpt-5",
                messages: vec![RisalaMuwafiq {
                    role: "user",
                    content: "u",
                }],
                temperature: hararat,
                max_completion_tokens: Some(16),
                max_tokens: None,
                response_format: json!({ "type": "json_object" }),
            })
            .ok()
        };
        assert_eq!(
            bina_muwafiq(None).and_then(|jasad| jasad.get("temperature").cloned()),
            None,
            "a None temperature must leave the Chat Completions body entirely"
        );
        assert!(
            bina_muwafiq(Some(HARARAT_TARJAMA))
                .and_then(|jasad| jasad.get("temperature").cloned())
                .is_some()
        );
    }

    #[test]
    fn albina_almahalli_yarfud_kul_ma_laysa_wasl_dakhili() {
        // The credential-free constructor is the one path that reaches a
        // provider without a key or a confirmation, so the only thing keeping
        // it from being a way to skip both is that it cannot be pointed
        // anywhere but this machine.
        for asas in [
            "https://api.openai.com",
            "http://127.0.0.1.evil.example",
            "http://localhost.attacker.test",
            "http://169.254.169.254",
        ] {
            let mut tarkib = IdadatMuwafiqOpenAI::ollama("qwen3");
            tarkib.asas = asas.to_owned();
            assert!(
                MuzawwidMuwafiqOpenAI::mahalli(tarkib).is_err(),
                "the loopback constructor accepted {asas}"
            );
        }
        for asas in ["http://127.0.0.1:11434", "http://localhost:11434"] {
            let mut tarkib = IdadatMuwafiqOpenAI::ollama("qwen3");
            tarkib.asas = asas.to_owned();
            let mabni = MuzawwidMuwafiqOpenAI::mahalli(tarkib);
            assert!(mabni.is_ok(), "the loopback constructor refused {asas}");
            // Whatever the configuration claimed, a loopback server has no
            // bill, so the meter and the model cannot disagree about it.
            if let Ok(muzawwid) = mabni {
                assert_eq!(muzawwid.qudrat().taklifa, NamudhajTaklifa::Majani);
                assert!(!muzawwid.qudrat().taklifa.madfu());
            }
        }
    }

    #[test]
    fn albina_albaeed_yarfud_annass_alwadih() {
        // A paid endpoint over plaintext puts the Authorization header on the
        // wire readable.
        let mut tarkib = IdadatMuwafiqOpenAI::openai();
        tarkib.asas = "http://api.openai.com".to_owned();
        let mabni = MuzawwidMuwafiqOpenAI::jadeed(
            tarkib,
            itimad_wahmi("sk-test"),
            IdhnInfaq::baad_taakid(1_000, 0),
        );
        assert!(matches!(
            mabni,
            Err(KhataTarjama::MuzawwidGhayrMutah { .. })
        ));
    }

    #[test]
    fn assaqf_yuqif_aljawla_wa_khalal_alhimaya_la() {
        // The batch loop asks these two questions instead of enumerating
        // variants, so a variant added later inherits the right behaviour.
        assert!(
            KhataTarjama::SaqfTakalif {
                munfaq: 10,
                saqf: 10
            }
            .yuqif_aljawla()
        );
        assert!(
            KhataTarjama::BilaItimad {
                muzawwid: MuzawwidAnthropic::ISM.to_owned()
            }
            .yuqif_aljawla()
        );
        let marfud = KhataTarjama::MudkhalMarfud {
            muzawwid: MuzawwidAnthropic::ISM.to_owned(),
            sabab: "HTTP 400".to_owned(),
        };
        assert!(!marfud.yuqif_aljawla());
        assert!(!marfud.khalal_himaya());
    }

    #[test]
    fn attaraju_yahtarim_tulb_alkhadim_wa_yahudduh() {
        // Waiting less than a server asked is how one 429 becomes a ban;
        // waiting an unbounded amount is a batch that hangs silently.
        assert_eq!(muddat_taraju(1, Some(5)), Duration::from_secs(5));
        assert_eq!(
            muddat_taraju(1, Some(AQSA_TARAJU_THAWANI.saturating_add(600))),
            Duration::from_secs(AQSA_TARAJU_THAWANI)
        );
        assert_eq!(
            muddat_taraju(1, None),
            Duration::from_millis(ASAS_TARAJU_MILLI * 2)
        );
        // The shift is bounded, so a long retry chain cannot overflow it.
        assert!(muddat_taraju(u32::MAX, None) <= Duration::from_millis(ASAS_TARAJU_MILLI << 6));
    }

    // -----------------------------------------------------------------------
    // The free web endpoint, against a loopback stand-in
    // -----------------------------------------------------------------------

    use std::io::{Read as _, Write as _};
    use std::sync::Arc;

    use taarib_mustalahat::nass::{QuyudNass, SiyaqNass, TasnifNass};
    use taarib_usus::khata::{Khutwa, QismIdadat, Tafsir as _};

    use crate::hima::ihmi;

    /// Anything a test can fail on: the crate's own error, the stub's I/O, or a
    /// runtime refusal.
    type NatijatIkhtibar = Result<(), Box<dyn std::error::Error>>;

    /// One scripted reply from the stand-in.
    struct RaddWahmi {
        /// The status code.
        hala: u16,
        /// Extra header lines, each ending in `\r\n`.
        ruus: &'static str,
        /// The body.
        jasad: String,
    }

    impl RaddWahmi {
        fn json(jasad: &str) -> Self {
            Self {
                hala: 200,
                ruus: "Content-Type: application/json; charset=utf-8\r\n",
                jasad: jasad.to_owned(),
            }
        }

        fn html(hala: u16) -> Self {
            Self {
                hala,
                ruus: "Content-Type: text/html; charset=utf-8\r\n",
                jasad: "<!DOCTYPE html><html><body>Our systems have detected unusual \
                        traffic from your computer network.</body></html>"
                    .to_owned(),
            }
        }

        /// A `429` that states no wait, so the floor is the only wait there is.
        fn muadal() -> Self {
            Self {
                hala: 429,
                ruus: "Retry-After: 0\r\n",
                jasad: String::new(),
            }
        }
    }

    /// The reason phrase the stub writes; the client reads the number.
    const fn sabab_hala(hala: u16) -> &'static str {
        match hala {
            200 => "OK",
            403 => "Forbidden",
            429 => "Too Many Requests",
            _ => "Status",
        }
    }

    /// A loopback HTTP/1.1 server answering a fixed script of replies, one
    /// connection each, and keeping every request line it saw.
    ///
    /// The listener is dropped when the script runs out, so a client that asks
    /// once more than scripted meets a refused connection — a fast, named
    /// transport failure — rather than a socket that hangs until the request
    /// timeout.
    struct KhadimWahmi {
        /// The endpoint address to point the provider at.
        asas: String,
        /// Every request line, in arrival order.
        talabat: Arc<parking_lot::Mutex<Vec<String>>>,
    }

    impl KhadimWahmi {
        fn shaghghil(rudud: Vec<RaddWahmi>) -> std::io::Result<Self> {
            let mustami = std::net::TcpListener::bind("127.0.0.1:0")?;
            let unwan = mustami.local_addr()?;
            let talabat = Arc::new(parking_lot::Mutex::new(Vec::new()));
            let sijill = Arc::clone(&talabat);
            drop(std::thread::spawn(move || {
                for radd in rudud {
                    let Ok((mut maqbas, _)) = mustami.accept() else {
                        return;
                    };
                    let _ = maqbas.set_read_timeout(Some(Duration::from_secs(5)));
                    let mut ras: Vec<u8> = Vec::new();
                    let mut hajira = [0_u8; 1024];
                    while let Ok(maqru) = maqbas.read(&mut hajira) {
                        if maqru == 0 {
                            break;
                        }
                        ras.extend_from_slice(hajira.get(..maqru).unwrap_or_default());
                        if ras.windows(4).any(|nafidha| nafidha == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let satr = String::from_utf8_lossy(&ras)
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    sijill.lock().push(satr);
                    let jawab = format!(
                        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
                        radd.hala,
                        sabab_hala(radd.hala),
                        radd.jasad.len(),
                        radd.ruus,
                        radd.jasad
                    );
                    let _ = maqbas.write_all(jawab.as_bytes());
                    let _ = maqbas.flush();
                }
            }));
            Ok(Self {
                asas: format!("http://{unwan}/translate_a/single"),
                talabat,
            })
        }

        fn talabat(&self) -> Vec<String> {
            self.talabat.lock().clone()
        }
    }

    /// The reply the endpoint gave on `TAREEKH_ASAR_GOOGLE_MAJJANI`, verbatim
    /// but for the per-segment metadata this module never reads.
    const RADD_GTX: &str = r#"[[["اضغط على أي مفتاح للمتابعة. ","Press any key to continue. ",null,null,3,null,null,[[]],[[["a40074848905d12c107f984aac5d9776","en_ar_2023q1.md"]]]],["درجاتك هي ⟦0⟧ نقطة.","Your score is ⟦0⟧ points.",null,null,3,null,null,[[]],[[["a40074848905d12c107f984aac5d9776","en_ar_2023q1.md"]]]]],null,"en",null,null,null,1,[],[["en"],null,[1],["en"]]]"#;

    /// The provider pointed at a stand-in, with no request spacing and no
    /// `429` floor unless a test sets one.
    fn majjani(
        khadim: &KhadimWahmi,
        ard_muadal: Duration,
    ) -> Result<MuzawwidGoogleMajjani, KhataTarjama> {
        MuzawwidGoogleMajjani::jadeed(IdadatGoogleMajjani {
            asas: khadim.asas.clone(),
            lughat_masdar: LUGHAT_MASDAR_TILQAIYA.to_owned(),
            fasl: Duration::ZERO,
            ard_muadal,
        })
    }

    /// A context with nothing in it, which is all this provider can use.
    fn talab_farigh() -> SiyaqTalab {
        SiyaqTalab {
            tasnif: TasnifNass::Majhul,
            siyaq: SiyaqNass::default(),
            quyud: QuyudNass::default(),
            mustalahat: Vec::new(),
            dhakira: Vec::new(),
            ism_luba: None,
            thiqat_tasnif: 0,
        }
    }

    #[test]
    fn almajjani_yaqra_alradd_almawdi_kama_huwa() -> NatijatIkhtibar {
        // The shape observed live: the translation is the first element of
        // every segment in the first array, the detected language sits third,
        // and everything else is skipped rather than parsed.
        let qeema: Value = serde_json::from_str(RADD_GTX)?;
        let radd = tarjama_min_gtx(&qeema).ok_or("the live shape must parse")?;
        assert_eq!(
            radd.tarjama,
            "اضغط على أي مفتاح للمتابعة. درجاتك هي ⟦0⟧ نقطة."
        );
        assert_eq!(radd.lugha_muktashafa.as_deref(), Some("en"));

        // Segments padded with `null` where text would be are skipped, not
        // failed; a shape with no leading array is not the shape.
        let mubattan: Value =
            serde_json::from_str(r#"[[["حفظ","Save"],[null,null,"transliteration"]],null,"en"]"#)?;
        assert_eq!(
            tarjama_min_gtx(&mubattan).map(|radd| radd.tarjama),
            Some("حفظ".to_owned())
        );
        assert_eq!(tarjama_min_gtx(&json!({"error": "x"})), None);
        assert_eq!(tarjama_min_gtx(&json!(["nope", null, "en"])), None);
        Ok(())
    }

    #[tokio::test]
    async fn almajjani_yutarjim_bila_miftah_wa_bila_thaman() -> NatijatIkhtibar {
        let khadim = KhadimWahmi::shaghghil(vec![RaddWahmi::json(RADD_GTX)])?;
        let muzawwid = majjani(&khadim, Duration::ZERO)?;

        // Everything the provider claims about itself, checked once: free,
        // no confidence, no batching, no instruction, and the names the
        // journal will record.
        assert_eq!(muzawwid.ism(), "google-majjani");
        assert_eq!(muzawwid.namudhaj(), "gtx");
        let qudrat = muzawwid.qudrat();
        assert_eq!(qudrat.taklifa, NamudhajTaklifa::Majani);
        assert!(!qudrat.taklifa.madfu());
        assert!(!qudrat.yublighu_thiqa());
        assert!(qudrat.dalil_thiqa().is_none());
        assert!(!qudrat.dufaat);
        assert_eq!(qudrat.aqsa_nusus(), 1);
        assert!(!qudrat.yaqbal_tawjih);
        assert_eq!(muzawwid.takalif().saqf(), 0);

        let mahmi = ihmi("Press any key to continue. Your score is ⟦0⟧ points.", &[])?;
        let natija = muzawwid.tarjim(&mahmi, &talab_farigh()).await?;
        assert_eq!(
            natija.matn(),
            "اضغط على أي مفتاح للمتابعة. درجاتك هي ⟦0⟧ نقطة."
        );
        assert_eq!(natija.taklifa(), 0);
        assert_eq!(natija.thiqa(), None);
        assert!(!natija.maqisa());
        assert_eq!(muzawwid.takalif().munfaq(), 0);

        // The wire: a GET with the widget's own parameters, the text
        // percent-encoded in the query string, Arabic as the target.
        let talabat = khadim.talabat();
        let satr = talabat.first().ok_or("one request reached the stand-in")?;
        assert!(satr.starts_with("GET /translate_a/single?"), "{satr}");
        for juz in ["client=gtx", "sl=auto", "tl=ar", "dt=t", "q=Press"] {
            assert!(satr.contains(juz), "{juz} missing from {satr}");
        }
        assert!(
            satr.contains("%E2%9F%A6"),
            "the protection token travels percent-encoded: {satr}"
        );
        assert_eq!(talabat.len(), 1, "one string, one request");
        Ok(())
    }

    #[tokio::test]
    async fn almajjani_yarfud_jasadan_farighan_wa_ghayr_mufassal() -> NatijatIkhtibar {
        let khadim = KhadimWahmi::shaghghil(vec![
            RaddWahmi::json(""),
            RaddWahmi::json("[[[\"\",\"Save\"]],null,\"en\"]"),
            RaddWahmi::json("[\"not\", \"the\", \"shape\"]"),
        ])?;
        let muzawwid = majjani(&khadim, Duration::ZERO)?;
        let talab = talab_farigh();

        for sabab in ["an empty body", "an empty translation", "a foreign shape"] {
            let mahmi = ihmi("Save", &[])?;
            let khata = muzawwid.tarjim(&mahmi, &talab).await.err().ok_or(sabab)?;
            assert!(
                matches!(khata, KhataTarjama::RaddGhayrMufassal { .. }),
                "{sabab}: {khata}"
            );
            // Fails the string, not the run: the next string may parse fine.
            assert!(!khata.yuqif_aljawla(), "{sabab}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn almajjani_yusammi_safhat_alhajb() -> NatijatIkhtibar {
        // The block page behind a success status, and behind the 403 the
        // service also uses. Both are the service refusing this client, both
        // stop the run, and both say so in a sentence that names the free
        // service and points at Settings.
        let khadim = KhadimWahmi::shaghghil(vec![RaddWahmi::html(200), RaddWahmi::html(403)])?;
        let muzawwid = majjani(&khadim, Duration::ZERO)?;
        let talab = talab_farigh();

        for hala in [200_u16, 403] {
            let mahmi = ihmi("Load", &[])?;
            let khata = muzawwid
                .tarjim(&mahmi, &talab)
                .await
                .err()
                .ok_or("a block page is a refusal")?;
            assert!(
                matches!(khata, KhataTarjama::MajjaniMahjub { .. }),
                "HTTP {hala}: {khata}"
            );
            assert!(khata.yuqif_aljawla(), "HTTP {hala}");
            assert!(khata.arabi().contains("الإعدادات ← المزوّدون"), "HTTP {hala}");
            assert!(khata.arabi().contains("google-majjani"), "HTTP {hala}");
            assert!(
                khata.injilizi().contains("Settings, under Providers"),
                "HTTP {hala}"
            );
            assert!(khata.injilizi().contains("google-majjani"), "HTTP {hala}");
            assert!(
                matches!(
                    khata.khutwa(),
                    Khutwa::FathIdadat {
                        qism: QismIdadat::Muzawwidun
                    }
                ),
                "HTTP {hala}"
            );
        }
        assert_eq!(khadim.talabat().len(), 2, "a 403 is not retried");
        Ok(())
    }

    #[tokio::test]
    async fn almajjani_yuid_baad_429_thumma_yatawaqqaf_bismih() -> NatijatIkhtibar {
        // Every attempt answered 429: the shared retry discipline spends all
        // of them, and what comes out is the free service's own refusal, not
        // the keyed providers' "backoff did not clear it".
        let mahjub =
            KhadimWahmi::shaghghil((0..ADAD_MUHAWALAT).map(|_| RaddWahmi::muadal()).collect())?;
        let muzawwid = majjani(&mahjub, Duration::ZERO)?;
        let mahmi = ihmi("Quit", &[])?;
        let khata = muzawwid
            .tarjim(&mahmi, &talab_farigh())
            .await
            .err()
            .ok_or("a 429 on every attempt is a refusal")?;
        assert!(
            matches!(khata, KhataTarjama::MajjaniMahjub { .. }),
            "{khata}"
        );
        assert!(khata.to_string().contains("429"), "{khata}");
        assert_eq!(
            mahjub.talabat().len(),
            hajm_usize(u64::from(ADAD_MUHAWALAT)),
            "exactly the attempt budget, no more"
        );

        // One 429 then a reply: the string succeeds on the retry, and the
        // retry waited at least the floor even though the server asked for no
        // wait at all — the floor is what stops a warning becoming a block.
        let ard = Duration::from_millis(200);
        let mutaaffi =
            KhadimWahmi::shaghghil(vec![RaddWahmi::muadal(), RaddWahmi::json(RADD_GTX)])?;
        let muzawwid = majjani(&mutaaffi, ard)?;
        let mahmi = ihmi("Quit", &[])?;
        let bidaya = std::time::Instant::now();
        let natija = muzawwid.tarjim(&mahmi, &talab_farigh()).await?;
        assert!(bidaya.elapsed() >= ard, "the 429 floor was not honoured");
        assert!(natija.matn().contains("⟦0⟧"));
        assert_eq!(mutaaffi.talabat().len(), 2);
        // With no floor, a stated wait of zero is honoured as zero: the floor
        // above is the only reason the retry waited.
        assert_eq!(muddat_taraju(1, Some(0)), Duration::ZERO);
        Ok(())
    }

    #[tokio::test]
    async fn almajjani_yarfud_ma_fawq_alsaqf_qabl_alirsal() -> NatijatIkhtibar {
        // Nothing listens here; the refusal has to happen before a connection
        // is attempted, or the test fails on the transport instead.
        let muzawwid = MuzawwidGoogleMajjani::jadeed(IdadatGoogleMajjani {
            asas: "http://127.0.0.1:9/translate_a/single".to_owned(),
            ..IdadatGoogleMajjani::default()
        })?;
        let tawil: String = "ع".repeat(AQSA_AHRUF_GOOGLE_MAJJANI.saturating_add(1));
        let mahmi = ihmi(&tawil, &[])?;
        let khata = muzawwid
            .tarjim(&mahmi, &talab_farigh())
            .await
            .err()
            .ok_or("a string over the ceiling is refused")?;
        match &khata {
            KhataTarjama::MajjaniTawil { ahruf, saqf, .. } => {
                assert_eq!(*ahruf, AQSA_AHRUF_GOOGLE_MAJJANI.saturating_add(1));
                assert_eq!(*saqf, AQSA_AHRUF_GOOGLE_MAJJANI);
            },
            ghayr => return Err(format!("expected MajjaniTawil, got {ghayr}").into()),
        }
        // One string, not the run — and the sentence says where longer
        // strings get translated.
        assert!(!khata.yuqif_aljawla());
        assert!(khata.arabi().contains("الإعدادات ← المزوّدون"));
        assert!(khata.injilizi().contains("Settings, under Providers"));
        assert!(khata.injilizi().contains("5001"));
        Ok(())
    }

    #[test]
    fn almajjani_yarfud_annass_alwadih_illa_ila_alwasl_addakhili() {
        // No credential rides on these requests, but a game's text does, and
        // the same loopback-by-name rule the local constructor keeps applies.
        for asas in [
            "http://translate.googleapis.com/translate_a/single",
            "http://127.0.0.1.evil.example/translate_a/single",
        ] {
            let mabni = MuzawwidGoogleMajjani::jadeed(IdadatGoogleMajjani {
                asas: asas.to_owned(),
                ..IdadatGoogleMajjani::default()
            });
            assert!(
                matches!(mabni, Err(KhataTarjama::MuzawwidGhayrMutah { .. })),
                "{asas} was accepted"
            );
        }
        assert!(MuzawwidGoogleMajjani::jadeed(IdadatGoogleMajjani::default()).is_ok());
        assert!(
            MuzawwidGoogleMajjani::jadeed(IdadatGoogleMajjani {
                asas: "http://localhost:1/translate_a/single".to_owned(),
                ..IdadatGoogleMajjani::default()
            })
            .is_ok()
        );
        // The shipped default is the endpoint checked on the recorded date,
        // over TLS, with detection left to the service.
        let iftiradi = IdadatGoogleMajjani::default();
        assert_eq!(iftiradi.asas, UNWAN_GOOGLE_MAJJANI);
        assert!(iftiradi.asas.starts_with("https://"));
        assert_eq!(iftiradi.lughat_masdar, LUGHAT_MASDAR_TILQAIYA);
        assert_eq!(iftiradi.fasl, FASL_GOOGLE_MAJJANI);
        assert_eq!(iftiradi.ard_muadal, ARD_MUADAL_GOOGLE_MAJJANI);
    }

    #[test]
    fn safhat_alhajb_tuqra_min_awwal_harf() {
        assert!(safha_la_json("<!DOCTYPE html><html>"));
        assert!(safha_la_json("  \n<html lang=\"en\">"));
        assert!(!safha_la_json("[[[\"حفظ\",\"Save\"]],null,\"en\"]"));
        assert!(!safha_la_json(""));
    }
}
