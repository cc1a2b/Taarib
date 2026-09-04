//! الصور — a game's artwork, and the colour its own cover lends the interface.
//!
//! Three pictures per game: the vertical cover the library grid shows, the wide
//! banner behind the detail header, and the small logo overlaid on it. This
//! module finds them, stores them, and pulls one colour out of the cover that
//! the interface uses in two places — as that game's accent on its detail
//! screen (section 8.3) and as the placeholder painted on the card while the
//! image is still decoding (section 8.7).
//!
//! ## Local first, always
//!
//! Every launcher on the machine has already downloaded its own artwork: Steam
//! keeps `librarycache`, GOG Galaxy keeps an image cache, Xbox ships the logo
//! inside the package. Reading those files costs one `read`, works with the
//! network unplugged, and cannot be rate-limited or taken offline by anybody.
//! A network fetch is the fallback for what is genuinely absent, never the
//! first choice, and an adapter that has a local file hands over
//! [`MasdarSura::Malaf`]; this module never turns a local source into a
//! request.
//!
//! ## Never on the scan's critical path
//!
//! Nothing in the [`Matjar`](crate::fahs::Matjar) trait mentions this module.
//! A scan produces [`MasadirSuwar`] — *where* the artwork can be found — and
//! returns. The library grid renders from that, with each card painted in its
//! stored colour and no image at all, and [`KhaziantSuwar::jalb`] is driven
//! afterwards by whatever is showing on screen. This is structural rather than
//! a matter of discipline: a scan cannot wait on the network because a scan has
//! no way to reach it.
//!
//! ## Content-addressed, sharded, deduplicated
//!
//! A stored image is named by the BLAKE3 hash of its *source* — the URL, or the
//! local path together with the size and modification time that make one
//! version of that file distinct from the next. Two games that point at the
//! same picture therefore share one file. A launcher that replaces a cover in
//! place produces a new key, because the size and timestamp moved, so a
//! refreshed cover is never masked by a stale cache entry.
//!
//! The layout mirrors the patch store in [`taarib_usus::masarat`]: the first
//! byte of the digest becomes a directory, so a library of forty thousand
//! images spreads across 256 directories instead of piling into one that no
//! file manager and no `readdir` enjoys.
//!
//! ```text
//! makhbaa/suwar/
//!   3f/2c91…d0.jpg        the image, exactly as it was fetched
//!   3f/2c91…d0.jpg.lawn   "#rrggbb", the colour extracted at import
//! ```
//!
//! The sidecar exists so that the warm path is cheap. Without it, answering
//! "what colour is this game?" for a cached cover would mean decoding a
//! multi-megapixel JPEG again on every launch. Eight bytes next to the image is
//! a better trade, and when the sidecar is missing or unreadable the colour is
//! simply derived again and rewritten.
//!
//! ## What the byte limits are for
//!
//! Everything here decodes data that came off the network. A PNG header can
//! declare 60000×60000 pixels in a file of eleven kilobytes; decoding it asks
//! the allocator for fourteen gigabytes. Dimensions are therefore read from the
//! header and checked *before* any decode is attempted, and rejected as
//! [`SuraGhayrSaliha`](crate::khata::KhataKashf::SuraGhayrSaliha). The decoder
//! is additionally given its own [`Limits`] as a second line of defence, in
//! case a format ever lets a header understate what it will allocate.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use image::{DynamicImage, ImageFormat, ImageReader, Limits, Rgba};
use taarib_mustalahat::luba::{LawnBariz, SuwarLuba};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat::{
    KhataMasarat, hadhf, insha_mujallad, kitaba_dharra, kitaba_dharra_nass, qira_nass,
};
use tokio::sync::{OnceCell, Semaphore};
use walkdir::WalkDir;

use crate::fahs::{MasadirSuwar, MasdarSura};
use crate::khata::KhataKashf;

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

/// Longest a single fetch may take from the first byte of the request to the
/// last byte of the body.
///
/// A cover is at most a few hundred kilobytes. Twenty seconds is generous for
/// a slow mobile connection and short enough that a CDN which accepts the
/// connection and then stops talking cannot hold a permit forever.
const MUHLAT_JALB: Duration = Duration::from_secs(20);

/// Longest the connection itself may take to establish.
const MUHLAT_ITTISAL: Duration = Duration::from_secs(6);

/// Largest artwork body accepted, in bytes.
///
/// Twelve megabytes is roughly forty times the largest real Steam hero image.
/// The cap exists for the server that answers a request for a 200 KiB cover
/// with an endless stream, which would otherwise fill memory and then the disk.
const HADD_HAJM_SURA: u64 = 12 * 1024 * 1024;

/// How many artwork fetches may be in flight at once, across every game.
///
/// Four keeps a library of six hundred games from opening six hundred sockets
/// against one CDN, which is indistinguishable from an attack from the other
/// side and gets an address rate-limited.
const HADD_MUTAWAZI: usize = 4;

/// How many redirect hops a fetch may follow.
///
/// Launcher artwork endpoints redirect once, occasionally twice, to a CDN
/// hostname. Three is room for that and nothing else: a longer chain is either
/// a loop or somebody walking the client somewhere it did not ask to go.
const HADD_TAHWIL: usize = 3;

/// Longest URL accepted from a launcher catalogue.
const HADD_TUL_RABT: usize = 2048;

/// Largest declared side, in pixels, before the image is refused undecoded.
const HADD_BUD: u32 = 16_384;

/// Largest declared pixel count before the image is refused undecoded.
///
/// Forty megapixels is far past any cover art and far below what it takes to
/// hurt: at four bytes a pixel this is a 160 MiB allocation, which the machine
/// can absorb, while the 3.6-gigapixel maximum a PNG header can declare is not.
const HADD_BIKSILAT: u64 = 40_000_000;

/// Largest allocation the decoder itself is permitted, in bytes.
const HADD_TAKHSIS: u64 = 192 * 1024 * 1024;

/// The file extensions this build can decode, in probe order.
///
/// Exactly the three formats the workspace enables on the `image` crate. A
/// fourth entry here without the matching feature would be a cache that stores
/// files it can never read back.
const IMTIDADAT: [&str; 3] = ["jpg", "png", "webp"];

/// The extension the colour sidecar carries, appended whole so that the image's
/// own extension stays visible.
const IMTIDAD_LAWN: &str = "lawn";

/// Length of a BLAKE3 digest written as lowercase hexadecimal.
const TUL_BASMA: usize = 64;

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

/// The artwork cache: a content-addressed directory, a bounded HTTP client, and
/// the dominant colour of everything it holds.
///
/// Cheap to clone conceptually but deliberately not [`Clone`]: the semaphore
/// that bounds concurrency has to be *one* semaphore, or a caller that cloned
/// the store would quietly double the number of sockets it opens. Share it
/// behind an [`std::sync::Arc`] instead.
#[derive(Debug)]
pub struct KhaziantSuwar {
    /// The root every stored file lives under.
    jidhr: PathBuf,
    /// Built on first use rather than in the constructor, so that a machine
    /// which never fetches anything never constructs a TLS stack, and so that
    /// construction does not have to happen inside an async runtime.
    amil: OnceCell<reqwest::Client>,
    /// The global ceiling on concurrent fetches.
    tasrih: Semaphore,
}

impl KhaziantSuwar {
    /// Opens the store at `jidhr`, creating it if it is not there.
    ///
    /// `jidhr` is expected to be `Masarat::makhbaa().join("suwar")` in the
    /// product; it is a parameter rather than derived here so that the sandbox
    /// and a portable installation get their own, and so that this module never
    /// resolves a user directory of its own.
    ///
    /// # Errors
    ///
    /// Returns [`KhataMasarat::TaadhurInsha`] when the directory cannot be
    /// created — a read-only data root, or a disk that is full.
    pub fn jadeeda(jidhr: PathBuf) -> Natija<Self> {
        insha_mujallad(&jidhr)?;
        Ok(Self {
            jidhr,
            amil: OnceCell::new(),
            tasrih: Semaphore::new(HADD_MUTAWAZI),
        })
    }

    /// The root of the store.
    #[must_use]
    pub fn jidhr(&self) -> &Path {
        &self.jidhr
    }

    /// Where a stored key lives, when it is really there.
    ///
    /// The key is what [`SuwarLuba`] carries: `<64 hex digits>.<extension>`.
    /// Anything else — a caller that stored a path, a database row corrupted
    /// into something else, a value that arrived from outside — returns
    /// [`None`] rather than being joined onto the root. That refusal is the
    /// only reason this function exists instead of a `join` at each call site:
    /// the key is the one part of the layout that comes from stored data, and a
    /// key of `../../../etc` must resolve to nothing at all.
    #[must_use]
    pub fn mahalli(&self, miftah: &str) -> Option<PathBuf> {
        let masar = masar_miftah(&self.jidhr, miftah)?;
        masar.is_file().then_some(masar)
    }

    /// The stored key for a digest, if any of the decodable formats is present.
    ///
    /// The extension is not recoverable from the digest — it depends on what
    /// the source turned out to be — so the three candidates are probed. Three
    /// `stat` calls on a warm cache is the price of keeping the format visible
    /// in the file name, which is what lets the web view pick a decoder without
    /// sniffing bytes.
    fn mawjud(&self, asas: &str) -> Option<(String, PathBuf)> {
        IMTIDADAT.into_iter().find_map(|imtidad| {
            let miftah = format!("{asas}.{imtidad}");
            let masar = masar_miftah(&self.jidhr, &miftah)?;
            masar.is_file().then_some((miftah, masar))
        })
    }
}

// ---------------------------------------------------------------------------
// Keys and paths
// ---------------------------------------------------------------------------

/// The digest a source is stored under.
///
/// BLAKE3 over a descriptor built by [`wasf_masdar`]. Not a security boundary
/// and not asked to be one — no path component is ever taken from the source
/// text, only from this fixed-length hexadecimal digest, so a hostile catalogue
/// entry cannot steer a write anywhere. What the hash does buy is that the same
/// picture referenced by two games is fetched once and stored once.
fn basmat(wasf: &str) -> String {
    blake3::hash(wasf.as_bytes()).to_hex().to_string()
}

/// The descriptor a source hashes to.
///
/// A URL identifies its own content well enough: launcher artwork endpoints
/// either embed a content hash in the path or are stable for the life of the
/// build. A local file does not — Steam overwrites `library_600x900.jpg` in
/// place when a publisher changes the cover — so the file's size and
/// modification time join the path. A replaced file therefore lands on a new
/// key and the new artwork appears, instead of the cache pinning last year's
/// picture forever.
fn wasf_masdar(masdar: &MasdarSura, hajm: u64, waqt_nano: u128) -> String {
    match masdar {
        MasdarSura::Malaf(masar) => {
            format!("malaf:{}:{hajm}:{waqt_nano}", masar.display())
        }
        MasdarSura::Rabt(rabt) => format!("rabt:{rabt}"),
    }
}

/// Splits a stored key into its digest and its extension, rejecting anything
/// that is not exactly what this module writes.
fn hallil_miftah(miftah: &str) -> Option<(&str, &str)> {
    let (asas, imtidad) = miftah.rsplit_once('.')?;
    let salih = asas.len() == TUL_BASMA
        && asas.bytes().all(|q| q.is_ascii_hexdigit() && !q.is_ascii_uppercase());
    if !salih || !IMTIDADAT.contains(&imtidad) {
        return None;
    }
    Some((asas, imtidad))
}

/// The sharded path a key maps to, without touching the filesystem.
fn masar_miftah(jidhr: &Path, miftah: &str) -> Option<PathBuf> {
    let (asas, imtidad) = hallil_miftah(miftah)?;
    let badiya = asas.get(..2)?;
    let baqi = asas.get(2..)?;
    Some(jidhr.join(badiya).join(format!("{baqi}.{imtidad}")))
}

/// The colour sidecar sitting beside an image.
///
/// Appended rather than substituted, so `…d0.jpg` becomes `…d0.jpg.lawn` and
/// the image it belongs to stays legible in a directory listing.
fn masar_lawn(masar: &Path) -> PathBuf {
    let mut ism = masar.as_os_str().to_os_string();
    ism.push(".");
    ism.push(IMTIDAD_LAWN);
    PathBuf::from(ism)
}

/// The extension a decoded format is stored under.
///
/// [`None`] for every format this build cannot decode, which is how a server
/// that answers with a TIFF or an animated GIF is refused before it reaches
/// the cache rather than after.
const fn imtidad_min_sigha(sigha: ImageFormat) -> Option<&'static str> {
    match sigha {
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::Png => Some("png"),
        ImageFormat::WebP => Some("webp"),
        _ => None,
    }
}

/// Reads a colour sidecar, tolerating every way it can be absent or wrong.
///
/// A missing, truncated, or hand-edited sidecar is not an error worth a code:
/// the colour is derived again from the image next to it and the sidecar is
/// rewritten. This returns [`None`] and lets the caller decide.
fn iqra_lawn(masar: &Path) -> Option<LawnBariz> {
    let nass = qira_nass(masar).ok()?;
    hallil_sittasi(nass.trim())
}

/// Parses `#rrggbb`.
fn hallil_sittasi(nass: &str) -> Option<LawnBariz> {
    let raqm = nass.strip_prefix('#')?;
    if raqm.len() != 6 {
        return None;
    }
    let qanat = |min: usize, ila: usize| -> Option<u8> {
        u8::from_str_radix(raqm.get(min..ila)?, 16).ok()
    };
    Some(LawnBariz {
        ahmar: qanat(0, 2)?,
        akhdar: qanat(2, 4)?,
        azraq: qanat(4, 6)?,
    })
}

// ---------------------------------------------------------------------------
// Acquisition
// ---------------------------------------------------------------------------

impl KhaziantSuwar {
    /// Resolves every artwork source a game has into stored keys, fetching only
    /// what is genuinely missing.
    ///
    /// Never fails. A source that cannot be read, a server that will not
    /// answer, a file that is not a decodable image — each degrades that one
    /// picture to [`None`] and is recorded in the log with the address that
    /// caused it. A game with no artwork is a card painted in the default
    /// accent, which is a library that works; a scan that failed because a CDN
    /// was down is not.
    ///
    /// The three slots are resolved concurrently with each other, and the
    /// global semaphore bounds how many fetches exist across every game being
    /// resolved at once.
    ///
    /// The colour is taken from the cover, falling back to the banner when
    /// there is no cover. It is never taken from the logo: a logo is mostly
    /// transparent and what remains is usually one flat brand colour that says
    /// nothing about the game.
    pub async fn jalb(&self, masadir: &MasadirSuwar) -> SuwarLuba {
        if masadir.khali() {
            return SuwarLuba::default();
        }

        let (ghilaf, batl, shiar) = tokio::join!(
            self.hassil(masadir.ghilaf.as_ref(), "ghilaf"),
            self.hassil(masadir.batl.as_ref(), "batl"),
            self.hassil(masadir.shiar.as_ref(), "shiar"),
        );

        let lawn = ghilaf
            .as_ref()
            .and_then(|(_, lawn)| *lawn)
            .or_else(|| batl.as_ref().and_then(|(_, lawn)| *lawn));

        SuwarLuba {
            ghilaf: ghilaf.map(|(miftah, _)| miftah),
            batl: batl.map(|(miftah, _)| miftah),
            shiar: shiar.map(|(miftah, _)| miftah),
            lawn,
        }
    }

    /// One slot: identify the source, answer from the cache when it is there,
    /// and otherwise read or fetch it, validate it, and store it.
    async fn hassil(
        &self,
        masdar: Option<&MasdarSura>,
        naw: &'static str,
    ) -> Option<(String, Option<LawnBariz>)> {
        let masdar = masdar?;

        // A local file's identity includes what it currently is, not only where
        // it is, so that a launcher replacing a cover in place is a cache miss
        // rather than a permanently stale picture.
        let (hajm, waqt_nano) = match masdar {
            MasdarSura::Malaf(masar) => {
                // A launcher listing a cover it never downloaded is ordinary,
                // not a fault, so this is a debug line rather than a warning.
                let bayanat = match tokio::fs::metadata(masar).await {
                    Ok(bayanat) => bayanat,
                    Err(khata) => {
                        tracing::debug!(
                            khana = naw,
                            masar = %masar.display(),
                            sabab = %khata,
                            "the launcher's own artwork file is not there"
                        );
                        return None;
                    },
                };
                let waqt = bayanat
                    .modified()
                    .ok()
                    .and_then(|q| q.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |q| q.as_nanos());
                (bayanat.len(), waqt)
            },
            MasdarSura::Rabt(_) => (0, 0),
        };

        let asas = basmat(&wasf_masdar(masdar, hajm, waqt_nano));

        if let Some((miftah, masar)) = self.mawjud(&asas) {
            let lawn = match iqra_lawn(&masar_lawn(&masar)) {
                Some(lawn) => Some(lawn),
                None => istaanif_lawn(masar).await,
            };
            return Some((miftah, lawn));
        }

        let bayt = match masdar {
            MasdarSura::Malaf(masar) => {
                if hajm > HADD_HAJM_SURA {
                    sajjil(
                        naw,
                        &KhataKashf::SuraGhayrSaliha {
                            rabt: masar.display().to_string(),
                            tafsil: format!("{hajm} bytes exceeds the {HADD_HAJM_SURA} byte limit"),
                        },
                    );
                    return None;
                }
                tokio::fs::read(masar).await.ok()?
            },
            MasdarSura::Rabt(rabt) => self.jalb_shabaka(rabt, naw).await?,
        };

        let jidhr = self.jidhr.clone();
        let asas_muhawwal = asas.clone();
        // Decoding a multi-megapixel cover and writing it back are both
        // blocking, and doing either on a runtime worker stalls every other
        // task on that thread — which on a two-core machine is visible as a
        // library that stutters while artwork arrives.
        let natija =
            tokio::task::spawn_blocking(move || khazzin(&jidhr, &asas_muhawwal, &bayt)).await;

        match natija {
            Ok(Ok((miftah, lawn))) => Some((miftah, Some(lawn))),
            Ok(Err(khata)) => {
                tracing::warn!(khana = naw, khata = %khata.li_sijill(), "artwork was not stored");
                None
            },
            // The workspace unwinds rather than aborts precisely so that a
            // malformed image which trips an assertion inside a decoder costs
            // one picture instead of the process.
            Err(khata) => {
                sajjil(
                    naw,
                    &KhataKashf::SuraGhayrSaliha {
                        rabt: asas,
                        tafsil: format!("the decoder did not survive this image: {khata}"),
                    },
                );
                None
            },
        }
    }

    /// Fetches one image over the network, bounded on every axis that can be
    /// used against a client: scheme, time, body size, concurrency, redirects.
    async fn jalb_shabaka(&self, rabt: &str, naw: &'static str) -> Option<Vec<u8>> {
        if !rabt_amin(rabt) {
            sajjil(
                naw,
                &KhataKashf::TaadhurJalbSura {
                    rabt: rabt.chars().take(HADD_TUL_RABT).collect(),
                    tafsil: "refused: artwork is fetched over https only, from a plain address"
                        .to_owned(),
                },
            );
            return None;
        }

        // Held for the whole request. Acquiring before the client is built also
        // means the first four callers do not race to build four TLS stacks.
        let _tasrih = self.tasrih.acquire().await.ok()?;
        let amil = self.amil(naw).await?;

        let mut radd = match amil.get(rabt).send().await {
            Ok(radd) => radd,
            Err(khata) => {
                sajjil(
                    naw,
                    &KhataKashf::TaadhurJalbSura {
                        rabt: rabt.to_owned(),
                        tafsil: khata.to_string(),
                    },
                );
                return None;
            },
        };

        if !radd.status().is_success() {
            sajjil(
                naw,
                &KhataKashf::TaadhurJalbSura {
                    rabt: rabt.to_owned(),
                    tafsil: format!("the server answered {}", radd.status()),
                },
            );
            return None;
        }

        // The declared length is a hint, not a promise: a chunked response
        // declares nothing, and a hostile one can declare anything. It is used
        // to refuse early and to size the buffer, and the real enforcement is
        // the running total below.
        let matluub = radd.content_length().unwrap_or(0);
        if matluub > HADD_HAJM_SURA {
            sajjil(
                naw,
                &KhataKashf::TaadhurJalbSura {
                    rabt: rabt.to_owned(),
                    tafsil: format!("declared {matluub} bytes, over the {HADD_HAJM_SURA} limit"),
                },
            );
            return None;
        }

        let mut bayt: Vec<u8> = Vec::with_capacity(usize::try_from(matluub).unwrap_or(0));
        loop {
            match radd.chunk().await {
                Ok(Some(qita)) => {
                    let majmu = u64::try_from(bayt.len().saturating_add(qita.len()))
                        .unwrap_or(u64::MAX);
                    if majmu > HADD_HAJM_SURA {
                        sajjil(
                            naw,
                            &KhataKashf::TaadhurJalbSura {
                                rabt: rabt.to_owned(),
                                tafsil: format!(
                                    "the body passed {HADD_HAJM_SURA} bytes and was abandoned"
                                ),
                            },
                        );
                        return None;
                    }
                    bayt.extend_from_slice(&qita);
                },
                Ok(None) => break,
                Err(khata) => {
                    sajjil(
                        naw,
                        &KhataKashf::TaadhurJalbSura {
                            rabt: rabt.to_owned(),
                            tafsil: khata.to_string(),
                        },
                    );
                    return None;
                },
            }
        }

        (!bayt.is_empty()).then_some(bayt)
    }

    /// The HTTP client, built once on first use.
    async fn amil(&self, naw: &'static str) -> Option<&reqwest::Client> {
        let natija = self.amil.get_or_try_init(|| async { bina_amil() }).await;
        match natija {
            Ok(amil) => Some(amil),
            Err(khata) => {
                sajjil(
                    naw,
                    &KhataKashf::TaadhurJalbSura {
                        rabt: "(client)".to_owned(),
                        tafsil: khata.to_string(),
                    },
                );
                None
            },
        }
    }
}

/// Builds the one client every artwork fetch goes through.
///
/// `https_only` is the load-bearing setting and it is not decoration. Artwork
/// is untrusted input on its way into an image decoder; over plaintext, anyone
/// between the machine and the CDN chooses what that decoder is handed. It also
/// closes the redirect downgrade, where an `https` address answers with a
/// `Location:` pointing at `http` and a client that only checked the first URL
/// follows it happily.
fn bina_amil() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(format!("Taarib/{}", taarib_usus::ISDAR))
        .timeout(MUHLAT_JALB)
        .connect_timeout(MUHLAT_ITTISAL)
        .redirect(reqwest::redirect::Policy::limited(HADD_TAHWIL))
        .https_only(true)
        // A referer would tell the CDN which game the user is looking at. It is
        // not needed to fetch a picture, so it is not sent.
        .referer(false)
        .build()
}

/// Whether an address from a launcher catalogue is one worth opening.
fn rabt_amin(rabt: &str) -> bool {
    if rabt.len() <= "https://".len() || rabt.len() > HADD_TUL_RABT {
        return false;
    }
    // A control character or a space in a URL is either a corrupt catalogue
    // entry or an attempt at request smuggling through a naive client.
    if rabt.bytes().any(|q| q.is_ascii_control() || q == b' ') {
        return false;
    }
    rabt.get(.."https://".len()).is_some_and(|q| q.eq_ignore_ascii_case("https://"))
}

/// Re-derives the colour of an image already in the cache and rewrites its
/// sidecar, off the runtime.
///
/// Reached when the sidecar was evicted, never written, or damaged. Returns
/// [`None`] rather than an error: the caller still has a perfectly good image
/// and a game without an accent falls back to the default one.
async fn istaanif_lawn(masar: PathBuf) -> Option<LawnBariz> {
    let janib = masar_lawn(&masar);
    tokio::task::spawn_blocking(move || {
        let bayt = std::fs::read(&masar).ok()?;
        let lawn = lawn_bariz(&bayt).ok()?;
        if let Err(khata) = kitaba_dharra_nass(&janib, &lawn.sittasi()) {
            tracing::debug!(khata = %khata.li_sijill(), "colour sidecar was not rewritten");
        }
        Some(lawn)
    })
    .await
    .ok()
    .flatten()
}

/// Validates, decodes, colours and stores one image.
///
/// The bytes are written exactly as they arrived. Re-encoding would cost a
/// second lossy generation on every JPEG cover for no gain, and would make the
/// stored file stop matching the digest of the source it is named after.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the bytes are not a decodable image or
/// declare dimensions that would not be safe to decode, and the write errors of
/// [`taarib_usus::masarat`] when the store cannot be written.
fn khazzin(jidhr: &Path, asas: &str, bayt: &[u8]) -> Natija<(String, LawnBariz)> {
    let (sura, sigha) = ifhas_sura(bayt, asas)?;
    let imtidad = imtidad_min_sigha(sigha).ok_or_else(|| KhataKashf::SuraGhayrSaliha {
        rabt: asas.to_owned(),
        tafsil: format!("{sigha:?} is not a format this build stores"),
    })?;

    let miftah = format!("{asas}.{imtidad}");
    let masar = masar_miftah(jidhr, &miftah)
        .ok_or_else(|| KhataMasarat::BasmaGhayrSaliha { basma: asas.to_owned() })?;

    let lawn = lawn_bariz_min_sura(&sura);

    // Atomic, so a crash mid-write leaves no half-image that would later decode
    // into a stripe of garbage and be cached as this game's colour.
    kitaba_dharra(&masar, bayt)?;
    kitaba_dharra_nass(&masar_lawn(&masar), &lawn.sittasi())?;

    Ok((miftah, lawn))
}

/// Decodes an image, refusing anything that would be dangerous to decode.
///
/// The order matters. The header is parsed for its declared dimensions and
/// those are checked *first*, because the check exists to prevent an
/// allocation, and an allocation that has already happened cannot be prevented.
/// Only then is the decoder run, and it is given its own limits as well, so a
/// format whose header understates what it expands to is still bounded.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] for every rejection, carrying `wasf` as the
/// address so a diagnostics bundle names the picture that was refused.
fn ifhas_sura(bayt: &[u8], wasf: &str) -> Natija<(DynamicImage, ImageFormat)> {
    let hajm = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
    if bayt.is_empty() || hajm > HADD_HAJM_SURA {
        return Err(KhataKashf::SuraGhayrSaliha {
            rabt: wasf.to_owned(),
            tafsil: format!("{hajm} bytes is not a usable image body"),
        }
        .into());
    }

    let qari = ImageReader::new(Cursor::new(bayt)).with_guessed_format().map_err(|khata| {
        KhataKashf::SuraGhayrSaliha { rabt: wasf.to_owned(), tafsil: khata.to_string() }
    })?;

    let sigha = qari.format().ok_or_else(|| KhataKashf::SuraGhayrSaliha {
        rabt: wasf.to_owned(),
        tafsil: "the bytes match no image format".to_owned(),
    })?;
    if imtidad_min_sigha(sigha).is_none() {
        return Err(KhataKashf::SuraGhayrSaliha {
            rabt: wasf.to_owned(),
            tafsil: format!("{sigha:?} is not one of the formats this build decodes"),
        }
        .into());
    }

    let (ard, tul) = qari.into_dimensions().map_err(|khata| KhataKashf::SuraGhayrSaliha {
        rabt: wasf.to_owned(),
        tafsil: khata.to_string(),
    })?;
    let biksilat = u64::from(ard).saturating_mul(u64::from(tul));
    let takhsis = biksilat.saturating_mul(4);
    if ard == 0 || tul == 0 || ard > HADD_BUD || tul > HADD_BUD || biksilat > HADD_BIKSILAT {
        return Err(KhataKashf::SuraGhayrSaliha {
            rabt: wasf.to_owned(),
            tafsil: format!(
                "declares {ard}x{tul}, which would allocate about {takhsis} bytes before a \
                 single pixel is examined"
            ),
        }
        .into());
    }

    let mut qari = ImageReader::new(Cursor::new(bayt)).with_guessed_format().map_err(|khata| {
        KhataKashf::SuraGhayrSaliha { rabt: wasf.to_owned(), tafsil: khata.to_string() }
    })?;
    qari.limits(hudud_amina());

    let sura = qari.decode().map_err(|khata| KhataKashf::SuraGhayrSaliha {
        rabt: wasf.to_owned(),
        tafsil: khata.to_string(),
    })?;

    Ok((sura, sigha))
}

/// The ceiling the decoder itself is held to.
///
/// Built up from no limits rather than down from the crate's defaults, so that
/// every bound in it is one this module chose and none of them can be relaxed
/// by a future change to what `image` considers reasonable.
fn hudud_amina() -> Limits {
    let mut hudud = Limits::no_limits();
    hudud.max_image_width = Some(HADD_BUD);
    hudud.max_image_height = Some(HADD_BUD);
    hudud.max_alloc = Some(HADD_TAKHSIS);
    hudud
}

/// Logs one degraded picture with its full explanation.
fn sajjil(naw: &'static str, sabab: &KhataKashf) {
    let khata = Khata::min_tafsir(sabab);
    tracing::warn!(khana = naw, khata = %khata.li_sijill(), "artwork unavailable");
}

// ---------------------------------------------------------------------------
// The dominant colour
// ---------------------------------------------------------------------------
//
// The whole algorithm, and why it is not three lines.
//
// The obvious implementation — average every pixel — produces mud. It produces
// mud for a reason worth stating, because the reason dictates the fix: averaging
// two complementary colours gives grey, and every cover art has complements in
// it. A red logo on a green field averages to brown. A blue sky over orange sand
// averages to grey. Run that over a hundred games and the library grid is a
// hundred cards in five shades of the same brown, which is worse than no colour
// at all, because it looks deliberate.
//
// What a person actually names as "the colour of that cover" is not the mean.
// It is the most *characteristic* colour: a region large enough to notice and
// chromatic enough to describe. So the algorithm finds regions, and then chooses
// among them.
//
//   1. Downsample to 96 px on the long side.
//
//      A 2:3 cover becomes 64x96, about six thousand pixels. Three reasons, all
//      of them load-bearing. It makes the clustering instant instead of taking
//      a quarter second per game. The box filter averages locally, which kills
//      JPEG chroma noise and dithering that would otherwise produce dozens of
//      near-identical clusters. And it destroys single-pixel features, so one
//      specular highlight or one stuck sensor pixel cannot become a game's
//      accent — a colour has to occupy real area to survive the resize.
//
//   2. Convert to Oklab.
//
//      Clustering needs a distance that matches perception, and sRGB does not
//      have one: the same numeric step is a different amount of visible change
//      at different points in the cube, so k-means in sRGB splits smooth
//      gradients and merges colours a person would never confuse. Oklab is
//      near-uniform, so Euclidean distance means roughly "how different these
//      look". It also gives the two axes the rejections below are written in
//      terms of: L is perceptual lightness, and the length of (a, b) is chroma.
//      "Too dark" and "too grey" become one number each instead of a heuristic
//      over three channels.
//
//   3. Reject pixels that cannot be an accent, before clustering.
//
//      Filtering afterwards is not the same thing: a cluster is an average, and
//      an average of rejected and acceptable pixels lands somewhere in between
//      and passes. The rejections are applied per pixel, and each exists for a
//      specific failure that is otherwise guaranteed:
//
//      - Transparent (alpha < 128). A logo is mostly transparent, and the RGB
//        stored under a fully transparent pixel is whatever the encoder left
//        there, frequently black or white garbage. Including it would drag
//        every logo's colour toward that garbage.
//
//      - Near-black (L < 0.16). This is the rejection that matters most. Cover
//        art is disproportionately dark — night skies, black bars, drop
//        shadows, letterboxing — and a cover that is 70% black sky would yield
//        black as its accent from any frequency-based method. Black is also
//        exactly the interface foundation (sath-0, #0B0C0D), so an accent of
//        black is an accent nobody can see: the button, the active row and the
//        focus ring all vanish into the background.
//
//      - Near-white (L > 0.92). White is the primary text colour (nass-1,
//        #E8EAEC). An accent that is white collides with body text, and the
//        hierarchy that tells a user what is a label and what is an action
//        disappears. Covers are full of white — titles, ESRB boxes, logos.
//
//      - Near-grey (chroma < 0.045). Grey is what the entire neutral scale is
//        made of, from sath-0 to nass-1. A grey accent does not read as a
//        choice, it reads as a bug. This also removes the anti-aliased edges
//        between a dark background and light text, which are numerous and would
//        otherwise form a large, convincing, meaningless cluster.
//
//   4. Cluster with k-means, k = 5, seeded deterministically.
//
//      Five is enough to separate a cover into background, subject, skin, logo
//      and accent without splitting one region across two centres. Seeding is
//      farthest-point (Gonzalez): the first centre is the sample furthest from
//      the mean, and each next centre is the sample furthest from everything
//      chosen so far. This is deterministic, which is not a detail — with
//      random seeding, the same cover yields a slightly different accent on
//      every launch, and an interface whose colour drifts for no visible reason
//      is an interface that feels broken. It also spreads the seeds, which is
//      what k-means++ buys with randomness, without needing an RNG at all.
//
//   5. Choose among the clusters. Do not take the largest.
//
//      Taking the largest cluster is only a slightly better average. On a cover
//      that is mostly a dark blue-grey sky with a small burning orange centre,
//      the largest cluster is the sky and the answer a person would give is the
//      orange. The score is therefore a product of four terms, described at
//      `nuqta` below: damped frequency, saturating chroma, a lightness band that
//      is legible on near-black, and a penalty for impersonating a status
//      colour.
//
//   6. Clamp into the range section 8.3 requires, and verify it.
//
//      The winner is then forced into a lightness and chroma band that stays
//      legible against sath-0, gamut-mapped back into sRGB by reducing chroma
//      while holding hue and lightness, and finally *measured*: the contrast
//      ratio against the foundation is computed from `LawnBariz::idaa` and the
//      lightness raised until it clears 4.5:1. Clamping to a band that is
//      usually enough is a guess; measuring is not.
//
// When a cover has no chromatic content at all — a black-and-white cover, a
// pure greyscale render — the filter empties and there is nothing to choose.
// That case returns the default accent, which is exactly what section 8.3 says
// the interface does with a colour "too desaturated to read as intentional".
// The same applies when the winner lands on top of a status colour: an accent
// indistinguishable from the danger red makes every danger badge ambiguous, so
// it is rejected back to the default rather than shipped.

/// Long side of the analysis thumbnail, in pixels.
const QIYAS_AYINA: u32 = 96;

/// How many regions a cover is decomposed into.
const ADAD_ANAQID: usize = 5;

/// Ceiling on k-means iterations. Convergence is normally reached in six or
/// seven; the ceiling is there so that a pathological image cannot spin.
const JAWLAT_TAJMEE: usize = 16;

/// Total centroid movement below which the clustering is called settled.
const HADD_ISTIQRAR: f32 = 1.0e-4;

/// The epsilon every float comparison in this module is written against.
const HADD_SIFR: f32 = 1.0e-6;

/// Alpha below which a pixel is not considered to be part of the picture.
const HADD_SHAFAFIYA: u8 = 128;

/// Oklab lightness below which a sample is discarded as effectively black.
const IDAA_AYINA_DUNYA: f32 = 0.16;

/// Oklab lightness above which a sample is discarded as effectively white.
const IDAA_AYINA_ULYA: f32 = 0.92;

/// Oklab chroma below which a sample is discarded as effectively grey.
const ISHBAA_AYINA_ADNA: f32 = 0.045;

/// Share of the thumbnail that must survive the rejections for the result to be
/// treated as a real colour rather than an accident of a few stray pixels.
const NISBAT_AYINAT_DUNYA: f32 = 0.02;

/// Floor on that share in absolute terms, for very small source images.
const ADAD_AYINAT_ADNA: usize = 24;

/// Chroma at which the colourfulness term of the score reaches one half.
const ISHBAA_NISF: f32 = 0.06;

/// Oklab lightness the scoring prefers: bright enough to carry the interface,
/// dark enough that white text still sits on it.
const IDAA_MITHALIYA: f32 = 0.62;

/// Width of the lightness preference, as the standard deviation of its
/// Gaussian. At 0.26 a cluster two thirds of the way to black still scores a
/// sixth rather than nothing, so a genuinely dark game is dark, not neutral.
const ITISAA_IDAA: f32 = 0.26;

/// Lightness floor after clamping. Below this the accent cannot reach 4.5:1
/// against the near-black foundation at any hue.
const IDAA_MUTAMADA_DUNYA: f32 = 0.58;

/// Lightness ceiling after clamping. Above this the accent stops being
/// distinguishable from the light end of the text scale.
const IDAA_MUTAMADA_ULYA: f32 = 0.80;

/// Chroma floor after clamping. The centroid of a cluster is always less
/// chromatic than its members, so the grey test is applied a second time here.
const ISHBAA_MUTAMAD_ADNA: f32 = 0.06;

/// Chroma ceiling after clamping. Past this, sRGB clips hard at these
/// lightnesses and a fully saturated fill vibrates against near-black.
const ISHBAA_MUTAMAD_AQSA: f32 = 0.16;

/// Oklab distance to a status colour within which the score is penalised.
const NITAQ_ILTIBAS: f32 = 0.10;

/// Oklab distance to a status colour within which the result is abandoned in
/// favour of the default accent, because at this range it *is* that colour.
const NITAQ_RAFD: f32 = 0.04;

/// What the score is multiplied by at the centre of a status colour.
const UQUBAT_HALA: f32 = 0.35;

/// Contrast ratio the accent must reach against the application background.
const NISBAT_TABAYUN_DUNYA: f32 = 4.5;

/// Step by which lightness is raised while chasing that ratio.
const KHUTWAT_IDAA: f32 = 0.02;

/// Ceiling on those steps.
const JAWLAT_TABAYUN: usize = 16;

/// Ceiling on gamut-mapping iterations.
const JAWLAT_NITAQ: usize = 24;

/// Factor chroma is multiplied by on each gamut-mapping iteration.
const HUBUT_ISHBAA: f32 = 0.96;

/// `sath-0`, the application background every accent is judged against.
const SATH_ASAS: LawnBariz = LawnBariz { ahmar: 0x0B, akhdar: 0x0C, azraq: 0x0D };

/// `tamyeez`, the default accent, and the answer when a cover yields nothing
/// usable.
const TAMYEEZ: LawnBariz = LawnBariz { ahmar: 0x2A, akhdar: 0x9D, azraq: 0x8F };

/// `najah`, `tanbeeh`, `khatar`, `maalumat` — the four status colours an accent
/// must not be mistaken for.
const ALWAN_HALA: [LawnBariz; 4] = [
    LawnBariz { ahmar: 0x57, akhdar: 0xA7, azraq: 0x73 },
    LawnBariz { ahmar: 0xC9, akhdar: 0x9A, azraq: 0x2E },
    LawnBariz { ahmar: 0xD0, akhdar: 0x53, azraq: 0x53 },
    LawnBariz { ahmar: 0x5B, akhdar: 0x8D, azraq: 0xBE },
];

/// Extracts a game's colour from encoded image bytes.
///
/// The bytes are validated exactly as a fetched image is — format, declared
/// dimensions, decoder allocation ceiling — so this is safe to call on anything,
/// including a file a user pointed at.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the bytes are empty, larger than the
/// artwork limit, in a format this build does not decode, declare dimensions
/// that would be unsafe to decode, or fail to decode.
pub fn lawn_bariz(bayt: &[u8]) -> Natija<LawnBariz> {
    let (sura, _) = ifhas_sura(bayt, "in-memory image")?;
    Ok(lawn_bariz_min_sura(&sura))
}

/// Extracts a game's colour from an image that is already decoded.
///
/// Always answers. An image with no chromatic content, an image one pixel
/// across, a fully transparent image — each returns the default accent, because
/// the interface needs a colour and the honest colour for artwork that has none
/// is the one the interface would have used anyway.
#[must_use]
pub fn lawn_bariz_min_sura(sura: &DynamicImage) -> LawnBariz {
    let ayinat = ayinat_min_sura(sura);
    let kul = ayinat.len();
    let hadd = usize::max(
        ADAD_AYINAT_ADNA,
        ila_hajm(ila_kasr(kul) * NISBAT_AYINAT_DUNYA),
    );

    let maqbula: Vec<Oklab> = ayinat.into_iter().filter(|ayina| maqbul(*ayina)).collect();
    if maqbula.len() < hadd {
        return TAMYEEZ;
    }

    let anaqid = jammi(&maqbula);
    let Some(fayiz) = anaqid
        .iter()
        .filter(|unqud| unqud.adad > 0)
        .max_by(|awwal, thani| {
            nuqta(awwal, maqbula.len())
                .partial_cmp(&nuqta(thani, maqbula.len()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    else {
        return TAMYEEZ;
    };

    let lawn = qayyid_lil_wajiha(fayiz.markaz);

    // The last rejection section 8.3 asks for, applied to what the interface
    // will actually paint rather than to the raw centroid.
    let mawdi = Oklab::min_lawn(lawn);
    if ALWAN_HALA.iter().any(|hala| mawdi.masafa(Oklab::min_lawn(*hala)) < NITAQ_RAFD) {
        return TAMYEEZ;
    }

    lawn
}

// ---------------------------------------------------------------------------
// Oklab
// ---------------------------------------------------------------------------

/// A colour in Oklab.
///
/// The three axes keep the names the space itself gives them. `L`, `a` and `b`
/// are not abbreviations of English words that a domain name could replace —
/// they are the published names of the coefficients in the matrices below, and
/// renaming them would make those matrices impossible to check against the
/// reference.
#[derive(Debug, Clone, Copy)]
struct Oklab {
    /// Perceptual lightness, 0 at black and 1 at white.
    l: f32,
    /// Green to red.
    a: f32,
    /// Blue to yellow.
    b: f32,
}

impl Oklab {
    /// From linear-light sRGB components.
    fn min_khatti(ahmar: f32, akhdar: f32, azraq: f32) -> Self {
        let tawil = saff(0.412_221_47, 0.536_332_54, 0.051_445_995, ahmar, akhdar, azraq);
        let mutawassit = saff(0.211_903_5, 0.680_699_5, 0.107_396_96, ahmar, akhdar, azraq);
        let qasir = saff(0.088_302_46, 0.281_718_85, 0.629_978_7, ahmar, akhdar, azraq);

        let (tawil, mutawassit, qasir) = (tawil.cbrt(), mutawassit.cbrt(), qasir.cbrt());

        Self {
            l: saff(0.210_454_26, 0.793_617_8, -0.004_072_047, tawil, mutawassit, qasir),
            a: saff(1.977_998_5, -2.428_592_2, 0.450_593_7, tawil, mutawassit, qasir),
            b: saff(0.025_904_037, 0.782_771_77, -0.808_675_77, tawil, mutawassit, qasir),
        }
    }

    /// From an 8-bit sRGB triple.
    fn min_bayt(ahmar: u8, akhdar: u8, azraq: u8) -> Self {
        Self::min_khatti(fakk_gamma(ahmar), fakk_gamma(akhdar), fakk_gamma(azraq))
    }

    /// From an interface colour.
    fn min_lawn(lawn: LawnBariz) -> Self {
        Self::min_bayt(lawn.ahmar, lawn.akhdar, lawn.azraq)
    }

    /// Back to linear-light sRGB, which may be outside `0.0..=1.0` when the
    /// colour is outside the sRGB gamut.
    fn ila_khatti(self) -> (f32, f32, f32) {
        let tawil = saff(1.0, 0.396_337_78, 0.215_803_76, self.l, self.a, self.b);
        let mutawassit = saff(1.0, -0.105_561_346, -0.063_854_17, self.l, self.a, self.b);
        let qasir = saff(1.0, -0.089_484_18, -1.291_485_5, self.l, self.a, self.b);

        let (tawil, mutawassit, qasir) =
            (tawil * tawil * tawil, mutawassit * mutawassit * mutawassit, qasir * qasir * qasir);

        (
            saff(4.076_741_7, -3.307_711_6, 0.230_969_94, tawil, mutawassit, qasir),
            saff(-1.268_438, 2.609_757_4, -0.341_319_38, tawil, mutawassit, qasir),
            saff(-0.004_196_086_3, -0.703_418_6, 1.707_614_7, tawil, mutawassit, qasir),
        )
    }

    /// Distance from the neutral axis: how colourful this is.
    fn ishbaa(self) -> f32 {
        self.a.hypot(self.b)
    }

    /// The hue angle, in radians.
    fn tadarruj(self) -> f32 {
        self.b.atan2(self.a)
    }

    /// Rebuilds a colour from lightness, chroma and hue.
    fn min_lch(l: f32, ishbaa: f32, tadarruj: f32) -> Self {
        Self { l, a: ishbaa * tadarruj.cos(), b: ishbaa * tadarruj.sin() }
    }

    /// Perceived difference between two colours.
    fn masafa(self, akhar: Self) -> f32 {
        let (di, da, db) = (self.l - akhar.l, self.a - akhar.a, self.b - akhar.b);
        db.mul_add(db, da.mul_add(da, di * di)).sqrt()
    }
}

/// One row of a 3x3 colour matrix, applied to one colour.
///
/// Fused multiply-adds, chained so that the coefficients stay in the published
/// matrix order at every call site instead of being turned inside out by the
/// nesting that writing `mul_add` by hand would force.
fn saff(m0: f32, m1: f32, m2: f32, x: f32, y: f32, z: f32) -> f32 {
    m2.mul_add(z, m1.mul_add(y, m0 * x))
}

/// One sRGB byte as linear light.
fn fakk_gamma(qeema: u8) -> f32 {
    let n = f32::from(qeema) / 255.0;
    if n <= 0.040_45 { n / 12.92 } else { ((n + 0.055) / 1.055).powf(2.4) }
}

/// One linear-light component back to the sRGB transfer curve.
fn rukkib_gamma(qeema: f32) -> f32 {
    if qeema <= 0.003_130_8 {
        12.92 * qeema
    } else {
        1.055_f32.mul_add(qeema.powf(1.0 / 2.4), -0.055)
    }
}

// ---------------------------------------------------------------------------
// Sampling, clustering, scoring
// ---------------------------------------------------------------------------

/// Downsamples the artwork and converts every opaque pixel to Oklab.
///
/// Transparent pixels never enter the sample set at all. The bytes stored in
/// the colour channels of a fully transparent pixel are undefined — encoders
/// leave black, white, or whatever was in the buffer — and a logo, which is
/// mostly transparent, would otherwise be described entirely by that.
fn ayinat_min_sura(sura: &DynamicImage) -> Vec<Oklab> {
    let biksilat = if sura.width() > QIYAS_AYINA || sura.height() > QIYAS_AYINA {
        sura.thumbnail(QIYAS_AYINA, QIYAS_AYINA).to_rgba8()
    } else {
        sura.to_rgba8()
    };

    let sia = usize::try_from(biksilat.width().saturating_mul(biksilat.height())).unwrap_or(0);
    let mut ayinat = Vec::with_capacity(sia);
    for biksil in biksilat.pixels() {
        let Rgba([ahmar, akhdar, azraq, shafafiya]) = *biksil;
        if shafafiya < HADD_SHAFAFIYA {
            continue;
        }
        ayinat.push(Oklab::min_bayt(ahmar, akhdar, azraq));
    }
    ayinat
}

/// Whether one sample can contribute to an accent.
///
/// The three tests are the near-black, near-white and near-grey rejections
/// described above. They are applied per sample rather than to the finished
/// answer because a cluster is a mean, and a mean of black sky and orange fire
/// is a brown nobody chose.
fn maqbul(ayina: Oklab) -> bool {
    ayina.l >= IDAA_AYINA_DUNYA
        && ayina.l <= IDAA_AYINA_ULYA
        && ayina.ishbaa() >= ISHBAA_AYINA_ADNA
}

/// One region of the artwork.
#[derive(Debug, Clone, Copy)]
struct Anqud {
    /// Where this region sits in Oklab.
    markaz: Oklab,
    /// Running sum of the members assigned this round.
    majmu: (f32, f32, f32),
    /// How many members were assigned this round.
    adad: u32,
}

impl Anqud {
    /// A cluster seeded at a colour, with nothing assigned yet.
    const fn jadeed(markaz: Oklab) -> Self {
        Self { markaz, majmu: (0.0, 0.0, 0.0), adad: 0 }
    }

    /// Clears the accumulator before an assignment pass.
    const fn ifragh(&mut self) {
        self.majmu = (0.0, 0.0, 0.0);
        self.adad = 0;
    }

    /// Assigns one sample.
    fn dumm(&mut self, ayina: Oklab) {
        self.majmu.0 += ayina.l;
        self.majmu.1 += ayina.a;
        self.majmu.2 += ayina.b;
        self.adad = self.adad.saturating_add(1);
    }

    /// Moves the centre onto the mean of what was assigned, and reports how far
    /// it travelled.
    ///
    /// A cluster that captured nothing keeps its seed and reports no movement.
    /// Re-seeding it would be the textbook repair, but here it would only find
    /// another point in an already well-separated set, and leaving it empty
    /// costs nothing: scoring ignores empty clusters.
    fn thabbit(&mut self) -> f32 {
        if self.adad == 0 {
            return 0.0;
        }
        let adad = ila_kasr(usize::try_from(self.adad).unwrap_or(usize::MAX));
        let jadeed = Oklab {
            l: self.majmu.0 / adad,
            a: self.majmu.1 / adad,
            b: self.majmu.2 / adad,
        };
        let haraka = jadeed.masafa(self.markaz);
        self.markaz = jadeed;
        haraka
    }
}

/// Farthest-point seeding.
///
/// The first centre is the sample furthest from the mean of the set — the most
/// distinctive colour on the cover rather than its average. Each subsequent
/// centre is the sample furthest from every centre already chosen, which
/// spreads the seeds across the whole distribution.
///
/// This is Gonzalez's k-centre heuristic, and the reason it is here rather than
/// k-means++ is that it needs no randomness. The same artwork must always
/// produce the same accent: a colour that shifts between launches would make
/// the interface look unstable for no reason a user could ever explain, and
/// would make a bug report about a wrong colour unreproducible.
fn budhur(ayinat: &[Oklab]) -> Vec<Oklab> {
    let Some(awwal) = ayinat.first() else {
        return Vec::new();
    };

    let adad = ila_kasr(ayinat.len());
    let mut majmu = (0.0_f32, 0.0_f32, 0.0_f32);
    for ayina in ayinat {
        majmu.0 += ayina.l;
        majmu.1 += ayina.a;
        majmu.2 += ayina.b;
    }
    let wasat = Oklab { l: majmu.0 / adad, a: majmu.1 / adad, b: majmu.2 / adad };

    let mut budhur = Vec::with_capacity(ADAD_ANAQID);
    let mut abad = *awwal;
    let mut aqsa = -1.0_f32;
    for ayina in ayinat {
        let masafa = ayina.masafa(wasat);
        if masafa > aqsa {
            aqsa = masafa;
            abad = *ayina;
        }
    }
    budhur.push(abad);

    while budhur.len() < ADAD_ANAQID {
        let mut murashah: Option<Oklab> = None;
        let mut abad_adna = 0.0_f32;
        for ayina in ayinat {
            let adna = budhur
                .iter()
                .map(|badhra| ayina.masafa(*badhra))
                .fold(f32::INFINITY, f32::min);
            if adna > abad_adna {
                abad_adna = adna;
                murashah = Some(*ayina);
            }
        }
        // Everything left is a duplicate of a centre already chosen, which is
        // what a flat two-colour cover looks like. More centres would be empty.
        match murashah {
            Some(badhra) if abad_adna > HADD_SIFR => budhur.push(badhra),
            _ => break,
        }
    }

    budhur
}

/// Lloyd's algorithm over the accepted samples.
fn jammi(ayinat: &[Oklab]) -> Vec<Anqud> {
    let mut anaqid: Vec<Anqud> = budhur(ayinat).into_iter().map(Anqud::jadeed).collect();
    if anaqid.is_empty() {
        return anaqid;
    }

    for _ in 0..JAWLAT_TAJMEE {
        for unqud in &mut anaqid {
            unqud.ifragh();
        }

        for ayina in ayinat {
            let mut aqrab = 0_usize;
            let mut adna = f32::INFINITY;
            for (fahras, unqud) in anaqid.iter().enumerate() {
                let masafa = ayina.masafa(unqud.markaz);
                if masafa < adna {
                    adna = masafa;
                    aqrab = fahras;
                }
            }
            if let Some(unqud) = anaqid.get_mut(aqrab) {
                unqud.dumm(*ayina);
            }
        }

        let mut haraka = 0.0_f32;
        for unqud in &mut anaqid {
            haraka += unqud.thabbit();
        }
        if haraka < HADD_ISTIQRAR {
            break;
        }
    }

    anaqid
}

/// How well a cluster would serve as this game's accent.
///
/// Four factors, multiplied, each answering a different way the naive choice
/// goes wrong:
///
/// - **Damped frequency**, the square root of the cluster's share. The square
///   root is the whole point: with the raw share this reduces to "take the
///   largest cluster", which is the failure being avoided. Damped, a cluster
///   covering an eighth of the cover can beat one covering half of it if it is
///   markedly more colourful — which is exactly the judgement a person makes
///   when they say a mostly-grey cover with a red logo "is the red one".
///
/// - **Saturating colourfulness**, `c / (c + 0.06)`. Rising, so more chroma is
///   always better, but flattening, so a neon fringe cannot beat a rich, broad
///   colour purely by being loud. There is no threshold in it, so two clusters
///   near the grey boundary are ordered rather than both being zeroed.
///
/// - **A lightness band**, a Gaussian centred at 0.62. Colours near that
///   lightness read cleanly on the near-black foundation and still take white
///   text. Very dark clusters score low because they will have to be lightened
///   so far to be legible that little of the original remains; very light ones
///   score low because they approach the text colour.
///
/// - **A status penalty**, falling to 0.35 at the centre of `najah`, `tanbeeh`,
///   `khatar` or `maalumat`. A penalty rather than a rejection, deliberately: a
///   game whose cover really is red should get red, only nudged off the exact
///   red that means danger. It merely means that when a second, unambiguous
///   cluster exists, that one wins.
fn nuqta(unqud: &Anqud, kul: usize) -> f32 {
    if unqud.adad == 0 || kul == 0 {
        return 0.0;
    }

    let adad = ila_kasr(usize::try_from(unqud.adad).unwrap_or(usize::MAX));
    let hissa = (adad / ila_kasr(kul)).sqrt();

    let ishbaa = unqud.markaz.ishbaa();
    let lawniya = ishbaa / (ishbaa + ISHBAA_NISF);

    let inhiraf = (unqud.markaz.l - IDAA_MITHALIYA) / ITISAA_IDAA;
    let mulaama = (-(inhiraf * inhiraf)).exp();

    let uquba = ALWAN_HALA
        .iter()
        .map(|hala| {
            let masafa = unqud.markaz.masafa(Oklab::min_lawn(*hala));
            if masafa >= NITAQ_ILTIBAS {
                1.0
            } else {
                (1.0 - UQUBAT_HALA).mul_add(masafa / NITAQ_ILTIBAS, UQUBAT_HALA)
            }
        })
        .fold(1.0_f32, f32::min);

    hissa * lawniya * mulaama * uquba
}

// ---------------------------------------------------------------------------
// Making the answer legible
// ---------------------------------------------------------------------------

/// Forces a cluster centre into the range section 8.3 requires of an accent.
///
/// Three stages. Lightness and chroma are clamped into the band that stays
/// readable against `sath-0`. The result is gamut-mapped back into sRGB by
/// lowering chroma while holding hue and lightness, because hue is what a
/// person recognises as "the game's colour" and clipping channels shifts it.
/// Then the contrast ratio against the foundation is *measured* with
/// [`LawnBariz::idaa`] and lightness raised in small steps until it clears
/// 4.5:1 — a clamp is a prediction, and this is the check that the prediction
/// held for this particular hue.
fn qayyid_lil_wajiha(markaz: Oklab) -> LawnBariz {
    let ishbaa_asli = markaz.ishbaa();
    // Hue is undefined on the neutral axis, and an undefined hue would be
    // whatever atan2 makes of two zeros. The default accent's hue is the
    // sensible substitute, since a colourless cover is heading there anyway.
    let tadarruj = if ishbaa_asli > HADD_SIFR {
        markaz.tadarruj()
    } else {
        Oklab::min_lawn(TAMYEEZ).tadarruj()
    };

    let ishbaa = ishbaa_asli.clamp(ISHBAA_MUTAMAD_ADNA, ISHBAA_MUTAMAD_AQSA);
    let mut idaa = markaz.l.clamp(IDAA_MUTAMADA_DUNYA, IDAA_MUTAMADA_ULYA);
    let mut lawn = ila_nitaq(idaa, ishbaa, tadarruj);

    for _ in 0..JAWLAT_TABAYUN {
        if nisbat_tabayun(lawn, SATH_ASAS) >= NISBAT_TABAYUN_DUNYA
            || idaa >= IDAA_MUTAMADA_ULYA - HADD_SIFR
        {
            break;
        }
        idaa = (idaa + KHUTWAT_IDAA).min(IDAA_MUTAMADA_ULYA);
        lawn = ila_nitaq(idaa, ishbaa, tadarruj);
    }

    lawn
}

/// Converts lightness, chroma and hue to sRGB, reducing chroma until the colour
/// fits inside the gamut.
///
/// Blue and violet hues have far less room at high lightness than yellow does,
/// so a fixed chroma ceiling is not enough on its own; this walks the chroma
/// down until the conversion lands inside the cube. Falling out of the loop
/// clamps, which only happens for a hue and lightness combination with almost
/// no chroma available at all.
fn ila_nitaq(idaa: f32, ishbaa: f32, tadarruj: f32) -> LawnBariz {
    let mut hali = ishbaa;
    for _ in 0..JAWLAT_NITAQ {
        let (ahmar, akhdar, azraq) = Oklab::min_lch(idaa, hali, tadarruj).ila_khatti();
        if dakhil_nitaq(ahmar) && dakhil_nitaq(akhdar) && dakhil_nitaq(azraq) {
            return min_khatti_ila_lawn(ahmar, akhdar, azraq);
        }
        hali *= HUBUT_ISHBAA;
    }
    let (ahmar, akhdar, azraq) = Oklab::min_lch(idaa, hali, tadarruj).ila_khatti();
    min_khatti_ila_lawn(ahmar, akhdar, azraq)
}

/// Whether one linear component is inside the sRGB cube, with a tolerance so
/// that a value one part in a million past the boundary is not treated as an
/// out-of-gamut colour.
fn dakhil_nitaq(qeema: f32) -> bool {
    (-HADD_SIFR..=1.0 + HADD_SIFR).contains(&qeema)
}

/// Linear-light components to an interface colour.
fn min_khatti_ila_lawn(ahmar: f32, akhdar: f32, azraq: f32) -> LawnBariz {
    let qanat = |qeema: f32| ila_bayt(rukkib_gamma(qeema.clamp(0.0, 1.0)) * 255.0);
    LawnBariz { ahmar: qanat(ahmar), akhdar: qanat(akhdar), azraq: qanat(azraq) }
}

/// The WCAG contrast ratio between two colours, from their relative luminance.
fn nisbat_tabayun(awwal: LawnBariz, thani: LawnBariz) -> f32 {
    let (a, b) = (awwal.idaa(), thani.idaa());
    let (ala, adna) = if a > b { (a, b) } else { (b, a) };
    (ala + 0.05) / (adna + 0.05)
}

// ---------------------------------------------------------------------------
// Numeric conversions
// ---------------------------------------------------------------------------

/// The `f32` form of a sample or cluster count.
#[expect(
    clippy::cast_precision_loss,
    reason = "counts are bounded by the analysis thumbnail, at most 96*96 = 9216 samples, far \
              inside f32's exact integer range"
)]
const fn ila_kasr(qeema: usize) -> f32 {
    qeema as f32
}

/// The `usize` form of a computed sample threshold, with anything negative or
/// non-finite treated as zero.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 0.0..=65535.0 before the conversion, so the number \
              converted is a non-negative whole number inside u16's range"
)]
fn ila_hajm(qeema: f32) -> usize {
    if !qeema.is_finite() {
        return 0;
    }
    usize::from(qeema.clamp(0.0, f32::from(u16::MAX)) as u16)
}

/// Rounds a 0..=255 channel value to the byte a colour stores.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the argument is clamped into 0.0..=255.0 and a half added before the cast, so the \
              value converted is a non-negative whole number inside u8's range"
)]
fn ila_bayt(qeema: f32) -> u8 {
    if !qeema.is_finite() {
        return 0;
    }
    (qeema.clamp(0.0, 255.0) + 0.5).min(255.0) as u8
}

// ---------------------------------------------------------------------------
// Accounting and eviction
// ---------------------------------------------------------------------------

impl KhaziantSuwar {
    /// How many bytes the store currently occupies.
    ///
    /// Walks without following symbolic links. That is not caution for its own
    /// sake: a link inside the cache pointing at the filesystem root would
    /// otherwise make this walk the whole machine, and would make
    /// [`nazzif`](Self::nazzif) delete things outside the cache to make room.
    ///
    /// A file whose metadata cannot be read is skipped rather than failing the
    /// count, because a cache measurement that refuses to answer is worse than
    /// one that is a few kilobytes short.
    ///
    /// # Errors
    ///
    /// [`KhataMasarat::TaadhurQira`] when the root exists but cannot be read at
    /// all. A root that does not exist yet is not an error; it is zero bytes.
    pub fn masaha(&self) -> Natija<u64> {
        match std::fs::metadata(&self.jidhr) {
            Ok(_) => {},
            Err(khata) if khata.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(sabab) => {
                return Err(
                    KhataMasarat::TaadhurQira { masar: self.jidhr.clone(), sabab }.into()
                );
            },
        }

        let mut majmu = 0_u64;
        for madkhal in WalkDir::new(&self.jidhr).follow_links(false).into_iter().flatten() {
            if !madkhal.file_type().is_file() {
                continue;
            }
            if let Ok(bayanat) = madkhal.metadata() {
                majmu = majmu.saturating_add(bayanat.len());
            }
        }
        Ok(majmu)
    }

    /// Brings the store back under `hadd_bayt` by deleting the least recently
    /// used artwork, and reports how many bytes that reclaimed.
    ///
    /// Recency is the file's access time where the filesystem keeps one, and
    /// its modification time where it does not. That fallback is worth being
    /// honest about: most Linux systems mount with `relatime`, and many people
    /// mount with `noatime`, so on those machines this is least-recently-*written*
    /// rather than least-recently-*read*. The alternative — writing a timestamp
    /// on every cache hit — would turn every library scroll into a burst of
    /// writes, which is a worse trade for a cache whose entries are all
    /// individually cheap to rebuild.
    ///
    /// Only images are candidates. A colour sidecar is deleted with the image
    /// it belongs to, never on its own, so the store never keeps a colour for a
    /// picture that is gone. An orphaned sidecar left by an interrupted run is
    /// eight bytes and is rewritten the next time that image is imported.
    ///
    /// A file that cannot be deleted — held open by the web view on Windows, or
    /// on a read-only mount — is skipped, and eviction continues with the next
    /// candidate. The failure is only returned when nothing at all could be
    /// reclaimed and the store is still over budget, because at that point the
    /// budget genuinely cannot be enforced and the caller needs to know.
    ///
    /// # Errors
    ///
    /// [`KhataMasarat::TaadhurQira`] when the store cannot be measured, and
    /// [`KhataMasarat::TaadhurHadhf`] when the store is over budget and no
    /// candidate could be removed.
    pub fn nazzif(&self, hadd_bayt: u64) -> Natija<u64> {
        let mut majmu = self.masaha()?;
        if majmu <= hadd_bayt {
            return Ok(0);
        }

        let mut murashahat: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
        for madkhal in WalkDir::new(&self.jidhr).follow_links(false).into_iter().flatten() {
            if !madkhal.file_type().is_file() {
                continue;
            }
            let masar = madkhal.path();
            let imtidad = masar.extension().and_then(std::ffi::OsStr::to_str).unwrap_or_default();
            if !IMTIDADAT.contains(&imtidad) {
                continue;
            }
            let Ok(bayanat) = madkhal.metadata() else { continue };
            let waqt = bayanat
                .accessed()
                .or_else(|_| bayanat.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            murashahat.push((masar.to_path_buf(), bayanat.len(), waqt));
        }

        murashahat.sort_by_key(|murashah| murashah.2);

        let mut muharrar = 0_u64;
        let mut awwal_khata: Option<Khata> = None;

        for (masar, hajm, _) in murashahat {
            if majmu <= hadd_bayt {
                break;
            }

            let janib = masar_lawn(&masar);
            let hajm_janib = std::fs::metadata(&janib).map_or(0, |bayanat| bayanat.len());

            match hadhf(&masar) {
                Ok(()) => {
                    majmu = majmu.saturating_sub(hajm);
                    muharrar = muharrar.saturating_add(hajm);
                },
                Err(khata) => {
                    tracing::debug!(
                        masar = %masar.display(),
                        khata = %khata.li_sijill(),
                        "artwork could not be evicted"
                    );
                    if awwal_khata.is_none() {
                        awwal_khata = Some(khata);
                    }
                    continue;
                },
            }

            if hadhf(&janib).is_ok() {
                majmu = majmu.saturating_sub(hajm_janib);
                muharrar = muharrar.saturating_add(hajm_janib);
            }
        }

        self.iknus_farigha();

        match awwal_khata {
            Some(khata) if muharrar == 0 && majmu > hadd_bayt => Err(khata),
            _ => Ok(muharrar),
        }
    }

    /// Removes shard directories that eviction emptied.
    ///
    /// Every failure is ignored on purpose: a shard that is not empty, a shard
    /// another thread is writing into at this instant, and a shard that is
    /// already gone are all the normal outcome, and none of them is worth a
    /// line in a log the user might read.
    fn iknus_farigha(&self) {
        let Ok(qaima) = std::fs::read_dir(&self.jidhr) else {
            return;
        };
        for madkhal in qaima.flatten() {
            if madkhal.file_type().is_ok_and(|naw| naw.is_dir()) {
                let _ = std::fs::remove_dir(madkhal.path());
            }
        }
    }
}
