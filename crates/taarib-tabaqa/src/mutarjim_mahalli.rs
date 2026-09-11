//! المترجم المحلّي — the one translation provider a payload can reach without
//! linking a TLS stack, and the sentence it shows when it cannot reach one.
//!
//! The payload is a `cdylib` inside somebody's game and links no HTTP client,
//! no async runtime and no keyring — see [`crate::mutarjim`]. What it can do
//! with the standard library alone is open a TCP socket to a plain-HTTP
//! endpoint and speak the Chat Completions dialect at it, which is exactly
//! what a local model server — Ollama, LM Studio, llama.cpp — offers on
//! loopback with no credential. That is the provider this module implements,
//! and it is the only one: an elected provider that needs a secure connection
//! or a stored credential is reported as unreachable *from the overlay*, in a
//! sentence that names the alternative, rather than silently translating
//! nothing.
//!
//! ## The wire
//!
//! `POST {base}/v1/chat/completions`, a system turn and a user turn, the reply
//! requested as a JSON object with one field, exactly the contract
//! `taarib-tarjama`'s OpenAI-compatible arm uses in its legacy dialect — the
//! dialect every local server understands. The reply is parsed as data and
//! never pattern-matched: a model that answered in prose has failed the
//! request, and its prose does not reach the screen.
//!
//! ## Blocking is the point
//!
//! Every call here blocks. It runs on [`crate::qissa::KhaytQissa`]'s worker,
//! which exists to be blocked on, and the timeouts are bounded so a server that
//! stopped answering costs one settled line a few seconds rather than the
//! session.

use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::{Value, json};
use taarib_usus::idadat::{HalatMuzawwidin, IdadatMuzawwid, NawMuzawwid};

use crate::khata::KhataTabaqa;
use crate::mutarjim::{MutarjimTabaqa, RaddSatr, TalabSatr};

/// How long a connection attempt may take.
const MUHLAT_ITTISAL: Duration = Duration::from_millis(1_500);

/// How long one request may take end to end, once connected.
///
/// A local model on a laptop CPU answers a short line in two to eight seconds;
/// forty is past any answer worth drawing, and a line that took longer than
/// that has already left the screen.
const MUHLAT_RADD: Duration = Duration::from_secs(40);

/// The most bytes a reply may carry before it is refused as not a reply.
const AQSA_HAJM_RADD: usize = 1 << 20;

/// How many consecutive connection failures put the provider to sleep.
const AQSA_FASHAL_MUTATALI: u32 = 3;

/// How long it sleeps before the next settled line tries again.
const MUDAT_SUKUN: Duration = Duration::from_secs(30);

/// The output token cap sent with every request.
const AQSA_RUMUZ: u32 = 512;

/// The sampling temperature, matching `taarib-tarjama`'s local arm.
const HARARA: f32 = 0.2;

/// The JSON field the reply carries the translation in.
const HAQL_TARJAMA: &str = "tarjama";

/// Whether the overlay can translate, stated for the panel.
///
/// Four states rather than an `Option`, because the three ways of having no
/// translator mean different things to the player: nothing configured is a
/// settings task, an unreachable provider is a different-provider task, and
/// unreadable settings is a bug report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalatMutarjim {
    /// A provider the overlay can reach is attached.
    Mutah {
        /// The provider's own identifier.
        ism: String,
        /// The model it will be asked for.
        namudhaj: String,
        /// The endpoint, as configured.
        asas: String,
    },
    /// No provider is elected on this machine.
    Ghaib(HalatMuzawwidin),
    /// A provider is elected, and the overlay cannot reach it.
    GhayrQabil {
        /// The provider's identifier.
        ism: String,
        /// Why, in English.
        sabab: String,
        /// The same, in Arabic.
        sabab_arabi: String,
    },
    /// The settings file could not be read, so nothing is known.
    LaIdadat {
        /// What went wrong.
        sabab: String,
    },
}

impl HalatMutarjim {
    /// Whether a provider is attached.
    #[must_use]
    pub const fn mutah(&self) -> bool {
        matches!(self, Self::Mutah { .. })
    }

    /// The sentence the panel shows, in English.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Mutah {
                ism,
                namudhaj,
                asas,
            } => format!("translating through {ism} ({namudhaj}) at {asas}"),
            Self::Ghaib(hala) => format!(
                "no translator: {} Lines already in the cache or in the installed patch are \
                 still drawn.",
                hala.injilizi()
            ),
            Self::GhayrQabil { ism, sabab, .. } => format!(
                "the elected provider {ism} cannot be reached from inside a game: {sabab} Lines \
                 already in the cache or in the installed patch are still drawn."
            ),
            Self::LaIdadat { sabab } => {
                format!("the settings could not be read, so no translator is attached: {sabab}")
            },
        }
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Mutah {
                ism,
                namudhaj,
                asas,
            } => format!("الترجمة عبر {ism} ({namudhaj}) على {asas}"),
            Self::Ghaib(hala) => format!(
                "لا مترجم: {} تُرسم السطور الموجودة في الخزينة أو في الرقعة المثبّتة كما هي.",
                hala.arabi()
            ),
            Self::GhayrQabil {
                ism, sabab_arabi, ..
            } => format!(
                "المزوّد المحدّد {ism} لا يمكن الوصول إليه من داخل اللعبة: {sabab_arabi} تُرسم \
                 السطور الموجودة في الخزينة أو في الرقعة المثبّتة كما هي."
            ),
            Self::LaIdadat { sabab } => {
                format!("تعذّرت قراءة الإعدادات فلم يُربط مترجم: {sabab}")
            },
        }
    }
}

/// Where the provider is reached: a plain-HTTP host, port and path prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Unwan {
    mudeef: String,
    manfadh: u16,
    masar: String,
}

impl Unwan {
    /// Parses a base address, accepting plain HTTP only.
    ///
    /// The refusal names the scheme rather than saying "invalid": an `https`
    /// base is a perfectly valid address that this build cannot speak to, and
    /// the player's fix is a different provider rather than a corrected URL.
    fn min_asas(asas: &str) -> Result<Self, (String, String)> {
        let asas = asas.trim();
        let Some(baqi) = asas.strip_prefix("http://") else {
            if asas.starts_with("https://") {
                return Err((
                    "it is reachable only over a secure connection, and the overlay carries no \
                     TLS stack; a local model server over plain HTTP (Ollama, LM Studio) is \
                     what the overlay can use."
                        .to_owned(),
                    "لا يُوصَل إليه إلا باتصال مؤمَّن، والطبقة لا تحمل مكدّس TLS؛ ما تستطيع الطبقة \
                     استعماله هو خادم نموذج محلّي عبر HTTP عادي (Ollama أو LM Studio)."
                        .to_owned(),
                ));
            }
            return Err((
                format!("its endpoint \"{asas}\" is not an http:// address."),
                format!("عنوانه \"{asas}\" ليس عنوان http://."),
            ));
        };
        let (mudeef_manfadh, masar) = match baqi.find('/') {
            Some(mawdi) => {
                let (awwal, thani) = baqi.split_at(mawdi);
                (awwal, thani.trim_end_matches('/').to_owned())
            },
            None => (baqi, String::new()),
        };
        let (mudeef, manfadh) = match mudeef_manfadh.rsplit_once(':') {
            Some((mudeef, manfadh)) if !mudeef.contains(']') || mudeef.ends_with(']') => {
                let manfadh = manfadh.parse::<u16>().map_err(|_| {
                    (
                        format!("its port \"{manfadh}\" is not a number."),
                        format!("منفذه \"{manfadh}\" ليس رقمًا."),
                    )
                })?;
                (mudeef.trim_matches(['[', ']']).to_owned(), manfadh)
            },
            _ => (mudeef_manfadh.trim_matches(['[', ']']).to_owned(), 80),
        };
        if mudeef.is_empty() {
            return Err((
                "its endpoint names no host.".to_owned(),
                "عنوانه لا يسمّي مضيفًا.".to_owned(),
            ));
        }
        Ok(Self {
            mudeef,
            manfadh,
            masar,
        })
    }
}

/// The connection's recent history, for the cooldown.
#[derive(Debug, Default)]
struct HalatWusul {
    fashal_mutatali: u32,
    sakin_hatta: Option<Instant>,
}

/// A local Chat Completions server, behind the overlay's provider seam.
#[derive(Debug)]
pub struct MutarjimMahalli {
    ism: String,
    namudhaj: String,
    asas: String,
    unwan: Unwan,
    wusul: Mutex<HalatWusul>,
}

impl MutarjimMahalli {
    /// Builds the provider the settings elect, or explains why the overlay
    /// cannot use it.
    ///
    /// Only [`NawMuzawwid::Mahalli`] and [`NawMuzawwid::OpenAiMutawafiq`] over
    /// plain HTTP with no stored credential are reachable; everything else is a
    /// [`HalatMutarjim::GhayrQabil`] with the reason. `None` for the elected
    /// provider yields [`HalatMutarjim::Ghaib`] with the settings' own account
    /// of why nothing is elected.
    #[must_use]
    pub fn min_idadat(
        muntakhab: Option<&IdadatMuzawwid>,
        hala: HalatMuzawwidin,
    ) -> (Option<Self>, HalatMutarjim) {
        let Some(tarif) = muntakhab else {
            return (None, HalatMutarjim::Ghaib(hala));
        };
        let ghayr_qabil = |sabab: String, sabab_arabi: String| {
            (
                None,
                HalatMutarjim::GhayrQabil {
                    ism: tarif.muarrif.clone(),
                    sabab,
                    sabab_arabi,
                },
            )
        };
        if !matches!(
            tarif.naw,
            NawMuzawwid::Mahalli | NawMuzawwid::OpenAiMutawafiq
        ) {
            return ghayr_qabil(
                "it speaks a dialect the overlay does not carry; only a local model server \
                 with an OpenAI-compatible endpoint is reachable from inside a game."
                    .to_owned(),
                "يتكلّم لهجة لا تحملها الطبقة؛ لا يُوصَل من داخل اللعبة إلا إلى خادم نموذج محلّي \
                 بواجهة متوافقة مع OpenAI."
                    .to_owned(),
            );
        }
        if tarif.hisab_miftah.is_some() {
            return ghayr_qabil(
                "it needs a stored credential, and the overlay reads no keychain; a local \
                 model server that needs no key is what the overlay can use."
                    .to_owned(),
                "يحتاج إلى مفتاح محفوظ، والطبقة لا تقرأ سلسلة المفاتيح؛ ما تستطيع الطبقة استعماله \
                 هو خادم نموذج محلّي لا يحتاج إلى مفتاح."
                    .to_owned(),
            );
        }
        let asas = tarif
            .asas
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_owned());
        let unwan = match Unwan::min_asas(&asas) {
            Ok(unwan) => unwan,
            Err((sabab, sabab_arabi)) => return ghayr_qabil(sabab, sabab_arabi),
        };
        if tarif.namudhaj.trim().is_empty() {
            return ghayr_qabil(
                "no model name is configured for it.".to_owned(),
                "لم يُضبط له اسم نموذج.".to_owned(),
            );
        }
        let mutarjim = Self {
            ism: tarif.muarrif.clone(),
            namudhaj: tarif.namudhaj.clone(),
            asas: asas.clone(),
            unwan,
            wusul: Mutex::new(HalatWusul::default()),
        };
        (
            Some(mutarjim),
            HalatMutarjim::Mutah {
                ism: tarif.muarrif.clone(),
                namudhaj: tarif.namudhaj.clone(),
                asas,
            },
        )
    }

    /// A provider at an explicit address, for a harness with a loopback stub.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the address is not plain HTTP.
    pub fn jadeed(ism: &str, asas: &str, namudhaj: &str) -> Result<Self, KhataTabaqa> {
        let unwan = Unwan::min_asas(asas).map_err(|(sabab, _)| KhataTabaqa::MawridFashil {
            mawrid: "local translator endpoint",
            sabab,
        })?;
        Ok(Self {
            ism: ism.to_owned(),
            namudhaj: namudhaj.to_owned(),
            asas: asas.trim().to_owned(),
            unwan,
            wusul: Mutex::new(HalatWusul::default()),
        })
    }

    /// The endpoint, as configured.
    #[must_use]
    pub fn asas(&self) -> &str {
        &self.asas
    }

    /// The system instruction: role, register, the one-line constraint and the
    /// output contract.
    ///
    /// Written for the register [`crate::mutarjim::TASNIF_IFTIRADI`] names —
    /// this tier reads dialogue boxes and menus and knows nothing more about a
    /// line than which of those it came from. The contract wording is the one
    /// `taarib-tarjama` uses, so a server tuned against Studio's requests sees
    /// the same instruction here.
    fn tawjih(talab: &TalabSatr<'_>) -> String {
        let mut nass = String::with_capacity(768);
        nass.push_str(
            "أنت مترجم ألعاب محترف تنقل نصوص لعبة من لغتها الأصلية إلى العربية. النص التالي \
             سطر واحد قُرئ من شاشة اللعبة آليًا، فقد يحمل خطأ قراءة في حرف أو حرفين؛ ترجم \
             المعنى المقصود.",
        );
        if let Some(ism) = talab.ism_luba {
            let _ = write!(nass, "\nاللعبة: {ism}.");
        }
        nass.push_str(
            "\n\nإن كان السطر حوارًا فترجمه بعربية فصيحة مبسّطة قريبة من إيقاع الكلام، بجمل \
             قصيرة يقولها متحدث فعلًا. وإن كان عنصر واجهة — زرًا أو بند قائمة — فترجمه بفصحى \
             موجزة على عرف الواجهات العربية: المصدر لا فعل الأمر، كلمة أو كلمتان، بلا نقطة في \
             الآخر. أبقِ الأرقام وأسماء الأعلام والمفاتيح كما هي.",
        );
        let _ = write!(
            nass,
            "\n\nيُعرض هذا النص في سطر واحد لا يلتف، في مساحة عرضها {} بكسل؛ قدّم أوجز صياغة \
             صحيحة تفي بالمعنى.",
            talab.mawdi.ard
        );
        let _ = write!(
            nass,
            "\n\nأجب بكائن JSON واحد لا شيء غيره: لا شرح قبله ولا بعده ولا علامات تنسيق حوله. \
             الحقل «{HAQL_TARJAMA}» يحمل الترجمة العربية وحدها."
        );
        nass
    }

    /// The user message: what was said before, then the line, fenced last.
    fn risala(talab: &TalabSatr<'_>) -> String {
        let mut nass = String::with_capacity(512);
        if !talab.siyaq.jiwar.is_empty() {
            nass.push_str("السطور التي سبقت هذا السطر على الشاشة، للسياق فقط ولا تُترجم:\n");
            for satr in &talab.siyaq.jiwar {
                let _ = writeln!(nass, "- {satr}");
            }
            nass.push('\n');
        }
        let _ = write!(nass, "النص المطلوب ترجمته:\n«{}»", talab.asl);
        nass
    }

    /// The request body, in the legacy dialect every local server accepts.
    fn jasad(&self, talab: &TalabSatr<'_>) -> String {
        json!({
            "model": self.namudhaj,
            "messages": [
                { "role": "system", "content": Self::tawjih(talab) },
                { "role": "user", "content": Self::risala(talab) },
            ],
            "temperature": HARARA,
            "max_tokens": AQSA_RUMUZ,
            "stream": false,
            "response_format": { "type": "json_object" },
        })
        .to_string()
    }

    /// Sends one request and returns the reply body.
    fn irsil(&self, jasad: &str) -> Result<String, KhataTabaqa> {
        let fashal = |sabab: String| KhataTabaqa::MawridFashil {
            mawrid: "local translator connection",
            sabab,
        };
        let unwan = format!("{}:{}", self.unwan.mudeef, self.unwan.manfadh);
        let mut anawin = unwan
            .to_socket_addrs()
            .map_err(|sabab| fashal(format!("{unwan} does not resolve: {sabab}")))?;
        let Some(awwal) = anawin.next() else {
            return Err(fashal(format!("{unwan} resolves to no address")));
        };
        let mut maqbas = TcpStream::connect_timeout(&awwal, MUHLAT_ITTISAL)
            .map_err(|sabab| fashal(format!("{unwan} refused the connection: {sabab}")))?;
        maqbas
            .set_read_timeout(Some(MUHLAT_RADD))
            .and_then(|()| maqbas.set_write_timeout(Some(MUHLAT_ITTISAL)))
            .map_err(|sabab| fashal(format!("the socket would not take a timeout: {sabab}")))?;

        let talab = format!(
            "POST {}/v1/chat/completions HTTP/1.1\r\nHost: {}\r\nContent-Type: \
             application/json\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: \
             {}\r\n\r\n{jasad}",
            self.unwan.masar,
            unwan,
            jasad.len()
        );
        maqbas
            .write_all(talab.as_bytes())
            .map_err(|sabab| fashal(format!("the request could not be sent: {sabab}")))?;

        let mut khaam: Vec<u8> = Vec::with_capacity(4096);
        let mut haajiz = [0_u8; 8192];
        loop {
            let adad = match maqbas.read(&mut haajiz) {
                Ok(0) => break,
                Ok(adad) => adad,
                Err(sabab) if sabab.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(sabab) => {
                    return Err(fashal(format!("the reply could not be read: {sabab}")));
                },
            };
            let Some(qita) = haajiz.get(..adad) else {
                break;
            };
            khaam.extend_from_slice(qita);
            if khaam.len() > AQSA_HAJM_RADD {
                return Err(fashal(format!(
                    "the reply exceeded {AQSA_HAJM_RADD} bytes, which is not a translation"
                )));
            }
        }
        jasad_min_radd(&khaam).map_err(fashal)
    }

    /// Records a connection outcome, for the cooldown.
    fn sajjil_wusul(&self, najah: bool) {
        let mut wusul = self.wusul.lock();
        if najah {
            wusul.fashal_mutatali = 0;
            wusul.sakin_hatta = None;
            return;
        }
        wusul.fashal_mutatali = wusul.fashal_mutatali.saturating_add(1);
        if wusul.fashal_mutatali >= AQSA_FASHAL_MUTATALI {
            wusul.sakin_hatta = Some(Instant::now() + MUDAT_SUKUN);
        }
    }
}

/// The body of an HTTP/1.1 reply, with the status checked and the transfer
/// encoding undone.
fn jasad_min_radd(khaam: &[u8]) -> Result<String, String> {
    let fasil = khaam
        .windows(4)
        .position(|nafidha| nafidha == b"\r\n\r\n")
        .ok_or_else(|| "the reply carried no header block".to_owned())?;
    let ruus = khaam.get(..fasil).unwrap_or_default();
    let jasad = khaam.get(fasil.saturating_add(4)..).unwrap_or_default();
    let ruus = String::from_utf8_lossy(ruus);
    let mut sutur = ruus.lines();
    let hala = sutur.next().unwrap_or_default();
    let ramz = hala
        .split_whitespace()
        .nth(1)
        .and_then(|raqm| raqm.parse::<u16>().ok())
        .ok_or_else(|| format!("the status line \"{hala}\" is not HTTP"))?;

    let mut mujazza = false;
    let mut tul: Option<usize> = None;
    for satr in sutur {
        let Some((ism, qeema)) = satr.split_once(':') else {
            continue;
        };
        let ism = ism.trim().to_ascii_lowercase();
        let qeema = qeema.trim();
        if ism == "transfer-encoding" && qeema.to_ascii_lowercase().contains("chunked") {
            mujazza = true;
        } else if ism == "content-length" {
            tul = qeema.parse::<usize>().ok();
        }
    }

    let jasad = if mujazza {
        fukk_tajzia(jasad)?
    } else {
        let hadd = tul.unwrap_or(jasad.len()).min(jasad.len());
        jasad.get(..hadd).unwrap_or_default().to_vec()
    };
    let nass = String::from_utf8_lossy(&jasad).into_owned();
    if !(200..300).contains(&ramz) {
        let muqtataf: String = nass.chars().take(160).collect();
        return Err(format!("the server answered HTTP {ramz}: {muqtataf}"));
    }
    Ok(nass)
}

/// Undoes `Transfer-Encoding: chunked`.
fn fukk_tajzia(khaam: &[u8]) -> Result<Vec<u8>, String> {
    let mut kharj = Vec::with_capacity(khaam.len());
    let mut mawdi = 0_usize;
    loop {
        let baqi = khaam.get(mawdi..).unwrap_or_default();
        let nihayat_satr = baqi
            .windows(2)
            .position(|nafidha| nafidha == b"\r\n")
            .ok_or_else(|| "a chunk size line was not terminated".to_owned())?;
        let satr = String::from_utf8_lossy(baqi.get(..nihayat_satr).unwrap_or_default());
        let hajm_nass = satr.split(';').next().unwrap_or_default().trim();
        let hajm = usize::from_str_radix(hajm_nass, 16)
            .map_err(|_| format!("\"{hajm_nass}\" is not a chunk size"))?;
        mawdi = mawdi
            .saturating_add(nihayat_satr)
            .saturating_add(2);
        if hajm == 0 {
            break;
        }
        let qita = khaam
            .get(mawdi..mawdi.saturating_add(hajm))
            .ok_or_else(|| "a chunk was shorter than its declared size".to_owned())?;
        kharj.extend_from_slice(qita);
        mawdi = mawdi.saturating_add(hajm).saturating_add(2);
    }
    Ok(kharj)
}

/// The translation out of a Chat Completions reply.
///
/// The assistant's content is parsed as JSON and the one field read. A model
/// that wrapped its object in a code fence is tolerated — local models do it
/// often enough that refusing it would refuse a correct answer for its
/// packaging — and everything else is a refusal that names what came back.
fn tarjama_min_radd(nass: &str) -> Result<String, String> {
    let qeema: Value = serde_json::from_str(nass.trim())
        .map_err(|sabab| format!("the reply is not JSON: {sabab}"))?;
    if let Some(khata) = qeema.get("error") {
        let risala = khata
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_else(|| khata.as_str().unwrap_or("unnamed"));
        return Err(format!("the server reported an error: {risala}"));
    }
    let muhtawa = qeema
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|khiyarat| khiyarat.first())
        .and_then(|awwal| awwal.get("message"))
        .and_then(|risala| risala.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| "the reply carries no assistant content".to_owned())?;
    let munaqqah = muhtawa.trim();
    let munaqqah = munaqqah
        .strip_prefix("```json")
        .or_else(|| munaqqah.strip_prefix("```"))
        .map_or(munaqqah, |baqi| baqi.trim_end_matches("```"))
        .trim();
    let kain: Value = serde_json::from_str(munaqqah).map_err(|_| {
        let muqtataf: String = munaqqah.chars().take(64).collect();
        format!("the model did not answer with a JSON object: {muqtataf}")
    })?;
    let tarjama = kain
        .get(HAQL_TARJAMA)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|nass| !nass.is_empty())
        .ok_or_else(|| format!("the object carries no \"{HAQL_TARJAMA}\" string"))?;
    Ok(tarjama.lines().collect::<Vec<_>>().join(" "))
}

impl MutarjimTabaqa for MutarjimMahalli {
    fn ism(&self) -> &str {
        &self.ism
    }

    fn mutah(&self) -> bool {
        let wusul = self.wusul.lock();
        wusul
            .sakin_hatta
            .is_none_or(|hatta| Instant::now() >= hatta)
    }

    fn tarjim(&self, talab: &TalabSatr<'_>) -> Result<RaddSatr, KhataTabaqa> {
        let jasad = self.jasad(talab);
        let radd = match self.irsil(&jasad) {
            Ok(radd) => radd,
            Err(khata) => {
                self.sajjil_wusul(false);
                return Err(khata);
            },
        };
        self.sajjil_wusul(true);
        let tarjama = tarjama_min_radd(&radd).map_err(|sabab| KhataTabaqa::MawridFashil {
            mawrid: "local translator reply",
            sabab,
        })?;
        Ok(RaddSatr::aaliya(tarjama))
    }
}

#[cfg(test)]
mod ikhtibarat {
    use super::*;

    #[test]
    fn unwan_yuqra_bi_manfadh_wa_masar() {
        let unwan = Unwan::min_asas("http://127.0.0.1:11434/").unwrap_or_else(|(s, _)| {
            Unwan {
                mudeef: s,
                manfadh: 0,
                masar: String::new(),
            }
        });
        assert_eq!(unwan.mudeef, "127.0.0.1");
        assert_eq!(unwan.manfadh, 11434);
        assert_eq!(unwan.masar, "");

        let unwan = Unwan::min_asas("http://box.local/gateway/v1/").unwrap_or_else(|(s, _)| {
            Unwan {
                mudeef: s,
                manfadh: 0,
                masar: String::new(),
            }
        });
        assert_eq!(unwan.mudeef, "box.local");
        assert_eq!(unwan.manfadh, 80);
        assert_eq!(unwan.masar, "/gateway/v1");
    }

    #[test]
    fn https_yurfad_bi_sabab_yusammi_al_badeel() {
        let radd = Unwan::min_asas("https://api.openai.com");
        assert!(matches!(radd, Err((ref s, _)) if s.contains("TLS")));
    }

    #[test]
    fn jasad_al_radd_bi_tajzia_yufakk() {
        let khaam = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n1\r\n \
                      \r\n5\r\nworld\r\n0\r\n\r\n";
        assert_eq!(
            jasad_min_radd(khaam).as_deref(),
            Ok("hello world")
        );
    }

    #[test]
    fn jasad_al_radd_bi_tul_yuqass() {
        let khaam = b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nabcdef";
        assert_eq!(jasad_min_radd(khaam).as_deref(), Ok("abcd"));
        let khata = b"HTTP/1.1 500 Boom\r\nContent-Length: 3\r\n\r\nbad";
        assert!(jasad_min_radd(khata).is_err());
    }

    #[test]
    fn tarjama_tuqra_min_kain_wa_min_siyaj() {
        let radd = json!({
            "choices": [{ "message": { "content": "{\"tarjama\": \"متابعة\"}" } }]
        })
        .to_string();
        assert_eq!(tarjama_min_radd(&radd).as_deref(), Ok("متابعة"));

        let musayyaj = json!({
            "choices": [{ "message": { "content": "```json\n{\"tarjama\": \"لعبة جديدة\"}\n```" } }]
        })
        .to_string();
        assert_eq!(tarjama_min_radd(&musayyaj).as_deref(), Ok("لعبة جديدة"));

        let nathr = json!({
            "choices": [{ "message": { "content": "Sure! Here it is: متابعة" } }]
        })
        .to_string();
        assert!(tarjama_min_radd(&nathr).is_err());
    }
}
