//! The overlay's surrounding context, driven all the way to a real provider.
//!
//! `tests/qissa.rs` proves the session's arithmetic against an instrument it
//! controls. This file proves the other half: that the context the session
//! builds survives the trip through `taarib-tarjama`'s prompt builder and
//! arrives in the bytes a provider actually receives — and that the arm it
//! arrives through is the **local** one, the arm a user with no API key has.
//!
//! ## What is real here
//!
//! Everything except the model. The session is
//! [`taarib_tabaqa::qissa::Qissa`]; the provider is
//! `taarib_tarjama::muzawwidun::MuzawwidMuwafiqOpenAI::mahalli`, the same
//! constructor an Ollama user gets, which refuses any host that is not literally
//! loopback and which runs with no credential and a free meter; the adapter is
//! [`taarib_tabaqa::wasil_tarjama::MutarjimMuzawwid`], which is shipped code; the
//! request is a real HTTP request over a real socket, built by
//! `taarib_tarjama::siyaq::risalat_mustakhdim` and serialized by the provider.
//!
//! The one thing standing in for a model is what answers on the other end of the
//! socket: a listener that records the request body and returns a fixed
//! well-formed reply. That is what makes the assertion possible — the test reads
//! the bytes the model would have read.
//!
//! ## Why this needs a feature flag
//!
//! `taarib-tarjama` brings a Tokio runtime, an HTTP client and a bundled `SQLite`.
//! None of that belongs in the payload that is loaded into a game, so the
//! adapter and this test are both behind `tarjama` and neither is built by a
//! plain `cargo test --workspace`. Run it with:
//!
//! ```text
//! cargo test -p taarib-tabaqa --features tarjama --test siyaq_mahalli
//! ```

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use parking_lot::Mutex;
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::nass::TasnifNass;
use taarib_tabaqa::manatiq::MuarrifMintaqa;
use taarib_tabaqa::qissa::{KhiyaratQissa, Qissa};
use taarib_tabaqa::tatabbu::QiraaMulahaza;
use taarib_tabaqa::wajiha::{MustatilBiksel, SighatSath, WasfSath};
use taarib_tabaqa::wasil_tarjama::MutarjimMuzawwid;
use taarib_tarjama::muzawwidun::{IdadatMuwafiqOpenAI, MuzawwidMuwafiqOpenAI};

/// The surface the session positions against.
const SATH: WasfSath = WasfSath {
    ard: 1920,
    irtifa: 1080,
    sigha: SighatSath::Bgra8,
    sirgb: true,
};

/// The subtitle strip, in surface pixels.
const SUNDUQ: MustatilBiksel = MustatilBiksel {
    yasar: 400,
    aala: 900,
    ard: 1120,
    irtifa: 40,
};

/// The region the readings come from.
const MINTAQA: MuarrifMintaqa = MuarrifMintaqa::min_raqm(1);

/// The default poll interval, which every synthetic pass advances by.
const KHUTWA_MIKRO: u64 = 250_000;

// ---------------------------------------------------------------------------
// A loopback server that records what it was sent
// ---------------------------------------------------------------------------

/// What the stub received, and how to reach it.
#[derive(Debug)]
struct Khadim {
    minfadh: u16,
    ajsad: Arc<Mutex<Vec<String>>>,
}

impl Khadim {
    /// Binds a listener on loopback and answers `adad` requests on a thread.
    ///
    /// Bounded rather than endless so the thread ends on its own and the test
    /// process is not left holding a listener open.
    fn ibda(adad: usize) -> Self {
        let mustami = match TcpListener::bind("127.0.0.1:0") {
            Ok(mustami) => mustami,
            Err(sabab) => panic!("the loopback stub could not bind: {sabab}"),
        };
        let minfadh = match mustami.local_addr() {
            Ok(unwan) => unwan.port(),
            Err(sabab) => panic!("the loopback stub has no address: {sabab}"),
        };
        let ajsad = Arc::new(Mutex::new(Vec::new()));
        let ajsad_khayt = Arc::clone(&ajsad);
        let khayt = std::thread::Builder::new()
            .name("taarib-stub".to_owned())
            .spawn(move || {
                for _ in 0..adad {
                    let Ok((majra, _)) = mustami.accept() else {
                        break;
                    };
                    if let Some(jasad) = jawib(majra) {
                        ajsad_khayt.lock().push(jasad);
                    }
                }
            });
        if let Err(sabab) = khayt {
            panic!("the loopback stub's thread would not start: {sabab}");
        }
        Self { minfadh, ajsad }
    }

    /// The recorded request bodies, in the order they arrived.
    fn ajsad(&self) -> Vec<String> {
        self.ajsad.lock().clone()
    }
}

/// Reads one request, records its body, and writes a fixed well-formed reply.
///
/// The reply's shape is the provider's contract, not this test's invention:
/// `choices[0].message.content` must itself be a JSON *string* whose contents
/// parse as the translation object, or `hallil_radd` refuses it.
fn jawib(mut majra: TcpStream) -> Option<String> {
    let mut khaam = Vec::new();
    let mut daraq = [0_u8; 4096];
    let mut tul: Option<usize> = None;
    let mut bidayat_jasad: Option<usize> = None;

    loop {
        let Ok(qura) = majra.read(&mut daraq) else {
            return None;
        };
        if qura == 0 {
            break;
        }
        khaam.extend_from_slice(daraq.get(..qura)?);
        if bidayat_jasad.is_none()
            && let Some(mawdi) = mawdi_faragh(&khaam)
        {
            let tarwisa = String::from_utf8_lossy(khaam.get(..mawdi)?).to_lowercase();
            tul = tarwisa
                .lines()
                .find_map(|satr| satr.strip_prefix("content-length:"))
                .and_then(|qeema| qeema.trim().parse::<usize>().ok());
            bidayat_jasad = Some(mawdi.saturating_add(4));
        }
        if let (Some(bidaya), Some(matlub)) = (bidayat_jasad, tul)
            && khaam.len().saturating_sub(bidaya) >= matlub
        {
            break;
        }
    }

    let bidaya = bidayat_jasad?;
    let jasad = String::from_utf8_lossy(khaam.get(bidaya..)?).into_owned();

    let matn = serde_json::json!({
        "choices": [{
            "message": { "content": "{\"tarjama\":\"الطريق مغلق\",\"thiqa\":0.9}" },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 0, "completion_tokens": 0 }
    })
    .to_string();
    let radd = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{matn}",
        matn.len()
    );
    let _ = majra.write_all(radd.as_bytes());
    let _ = majra.flush();
    Some(jasad)
}

/// Where the blank line between the headers and the body starts.
fn mawdi_faragh(khaam: &[u8]) -> Option<usize> {
    khaam.windows(4).position(|nafidha| nafidha == b"\r\n\r\n")
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The fixture's identity.
///
/// [`MasdarLuba::Yadawi`] because nothing here is a real game: the readings are
/// constructed text and no title of any engine is installed on this machine.
fn luba() -> LubaId {
    LubaId::min_masdar(
        &MasdarLuba::Yadawi("taarib-siyaq-fixture".to_owned()),
        ISM_LUBA,
    )
}

/// The fixture's display name.
const ISM_LUBA: &str = "Synthetic Frames (a test fixture, not a game)";

/// One recognized line in the subtitle strip.
fn mulahaza(nass: &str) -> Vec<QiraaMulahaza> {
    vec![QiraaMulahaza {
        nass: nass.to_owned(),
        mawdi: SUNDUQ,
        thiqa: 92,
        maqisa: true,
    }]
}

// ---------------------------------------------------------------------------
// The proof
// ---------------------------------------------------------------------------

/// The previous line reaches the local provider inside the request body.
///
/// Three assertions, and each is about a different thing that could have been
/// quietly dropped: the arm actually used is the loopback one, the second line's
/// request carries the first line in the prompt's neighbours block, and the line
/// being translated is not itself listed as a neighbour.
#[test]
fn siyaq_yasil_ila_muzawwid_mahalli() {
    let khadim = Khadim::ibda(2);

    let mut idadat = IdadatMuwafiqOpenAI::ollama("qwen3");
    idadat.asas = format!("http://127.0.0.1:{}", khadim.minfadh);
    // `mahalli` is the credential-free constructor. It refuses any host that is
    // not literally loopback, forces the meter to free, and is exactly what a
    // user with no API key and a local model server gets.
    let muzawwid = match MuzawwidMuwafiqOpenAI::mahalli(idadat) {
        Ok(muzawwid) => muzawwid,
        Err(khata) => panic!("the local provider would not build: {khata}"),
    };
    let mutarjim = match MutarjimMuzawwid::jadeed(Arc::new(muzawwid)) {
        Ok(mutarjim) => mutarjim,
        Err(khata) => panic!("the provider adapter would not build: {khata}"),
    };
    assert_eq!(
        taarib_tabaqa::mutarjim::MutarjimTabaqa::ism(&mutarjim),
        "ollama",
        "the arm under test must be the local one"
    );

    let khiyarat = KhiyaratQissa {
        tasnif: TasnifNass::Hiwar,
        yasjil: false,
        ..KhiyaratQissa::iftiradiya()
    };
    let mut qissa = Qissa::jadeeda(luba(), ISM_LUBA, khiyarat)
        .bi_mutarjim(Box::new(mutarjim))
        .bi_qari("synthetic");
    qissa.ayyin_sath(SATH);

    let awwal = "The old road north is closed until the thaw.";
    let thani = "Take the ferry from the eastern pier instead.";
    let mut lahza = 0;
    for nass in [awwal, awwal, thani, thani] {
        let _ = qissa.aalij_sutur(MINTAQA, "subtitles", lahza, &mulahaza(nass));
        lahza = lahza.saturating_add(KHUTWA_MIKRO);
    }

    // The provider's own retry and backoff run on the adapter's runtime, so the
    // requests have landed by the time `aalij_sutur` returned — it blocks. What
    // is waited for here is only the stub thread's own push.
    let mut ajsad = khadim.ajsad();
    for _ in 0..200 {
        if ajsad.len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        ajsad = khadim.ajsad();
    }

    assert_eq!(
        ajsad.len(),
        2,
        "two settled lines must have produced two requests to the local server; {} arrived",
        ajsad.len()
    );

    let Some(jasad_thani) = ajsad.get(1) else {
        panic!("the second request never arrived");
    };
    let qeema: serde_json::Value = match serde_json::from_str(jasad_thani) {
        Ok(qeema) => qeema,
        Err(khata) => panic!("the second request body is not JSON: {khata}\n{jasad_thani}"),
    };
    let risala = qeema
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .and_then(|rasail| rasail.get(1))
        .and_then(|risala| risala.get("content"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    assert!(
        risala.contains(thani),
        "the request must carry the line being translated; it carried:\n{risala}"
    );
    assert!(
        risala.contains(awwal),
        "the request must carry the previous line as surrounding context; it carried:\n{risala}"
    );
    // The prompt builder writes each neighbour on its own line prefixed with a
    // pipe, under an Arabic heading that says they are context and must not be
    // translated. Asserting the marker rather than only the substring is what
    // distinguishes "the line reached the neighbours block" from "the line
    // happened to appear somewhere in the prompt".
    assert!(
        risala.contains(&format!("| {awwal}")),
        "the previous line must be in the prompt's neighbours block, not merely present \
         somewhere; the message was:\n{risala}"
    );

    let Some(jasad_awwal) = ajsad.first() else {
        panic!("the first request never arrived");
    };
    assert!(
        !jasad_awwal.contains(thani),
        "the opening line must not be given a neighbour that had not been said yet"
    );

    // And the arm carries its own marks: a local model name, a temperature, and
    // no bearer credential anywhere in the body.
    assert_eq!(
        qeema.get("model").and_then(serde_json::Value::as_str),
        Some("qwen3"),
        "the request must name the local model that was configured"
    );
}
