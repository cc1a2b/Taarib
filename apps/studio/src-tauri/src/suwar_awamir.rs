//! صور المكتبة — the artwork stage: real cover art, resolved after the grid is
//! already on screen.
//!
//! ## Two passes, and why
//!
//! A library scan must not wait on a CDN. [`crate::maktaba`] therefore answers
//! from Taarib's own artwork cache and nothing else: a key already on the game's
//! row resolves to a path with one `stat`, so a rescan draws real covers on the
//! first frame, and a game whose artwork has never been fetched arrives with the
//! generated plate the card already knows how to draw.
//!
//! Everything the cache does not have is resolved here, by
//! [`hassil_suwar_maktaba`], which the library screen calls *after* it has
//! rendered. Each game is announced on [`ISM_HADATH_GHILAF`] the moment its
//! artwork lands, so sixteen cards fill in one by one instead of together at the
//! end, and the command's own answer is the summary a diagnostics screen wants.
//!
//! The sources come from the scan that just ran, held in [`DhakiratSuwar`].
//! They are not re-derived: only the launcher adapters know where a launcher
//! keeps its pictures, and asking them again would mean running the whole scan a
//! second time to learn something it already knew.
//!
//! ## What is fetched, and what is not
//!
//! Every source that is a **file on disk** is read — the launcher already
//! downloaded it, it costs one `read`, and it works with the network unplugged.
//! Over the **network**, only the cover and the banner are fetched: those are
//! the two pictures the card can draw. The logo has no consumer in the interface
//! today, and a megabyte per game for something nothing paints is not a cost a
//! first scan should pay. A logo that is already on disk is still stored, so the
//! day a screen wants one it is there.
//!
//! ## Bounds
//!
//! Four games are resolved at once, matching the artwork store's own ceiling on
//! concurrent fetches, and each game is additionally given a wall-clock ceiling
//! of its own. Nothing here can fail a library: a source that 404s, a server
//! that stops talking, an image that will not decode — each degrades one picture
//! to [`None`], is named in [`HasilatSuwar::bila_suwar`], and leaves the other
//! fifteen games untouched.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use taarib_kashf::fahs::{MasadirSuwar, MasdarSura};
use taarib_kashf::suwar::KhaziantSuwar;
use taarib_makhzan::sijillat::{SijillAlaab, SijillSuwar, SuratMukhzana, TalabMaktaba};
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::luba::{LawnBariz, LubaId, SuwarLuba};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat::Masarat;
use tauri::Emitter as _;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// The window event one game's finished artwork is announced on.
///
/// Emitted once per game by [`hassil_suwar_maktaba`], including for a game that
/// ended with nothing: a card that never hears about itself would keep whatever
/// pending state the interface put it in forever.
pub const ISM_HADATH_GHILAF: &str = "taarib://ghilaf-luba";

/// The artwork cache's subdirectory under `makhbaa`, as `taarib-kashf` writes it.
pub const MUJALLAD_SUWAR: &str = "suwar";

/// How many games are resolved at once.
///
/// Four, matching the ceiling the artwork store puts on concurrent network
/// fetches. A higher number here would only queue more tasks against the same
/// four permits while holding more decoded images in memory at once.
const HADD_ALAAB_MUTAWAZIYA: usize = 4;

/// Wall-clock ceiling on one game's artwork, across every slot it resolves.
///
/// The store already bounds a single request at twenty seconds and a connection
/// at six. This is the bound on the *game*, which is a different quantity: two
/// slots, each of which may wait behind three other games for one of the four
/// fetch permits. Ninety seconds is past any honest worst case and short enough
/// that a server which accepts connections and then stops talking cannot hold a
/// slot in the concurrency limit for the life of the process.
const MUHLAT_LUBA: Duration = Duration::from_secs(90);

/// The `sura` ledger's name for the vertical cover.
const NAW_GHILAF: &str = "ghilaf";

/// The `sura` ledger's name for the wide banner.
const NAW_BATL: &str = "batl";

/// The `sura` ledger's name for the logo.
const NAW_SHIAR: &str = "shiar";

// ---------------------------------------------------------------------------
// What the scan hands over
// ---------------------------------------------------------------------------

/// One game's artwork sources, as the scan that found them left them.
#[derive(Debug, Clone)]
pub struct TalabSuwar {
    /// The game's name, so a refusal names a game rather than an identity.
    pub ism: String,
    /// Where its three pictures can be found.
    pub masadir: MasadirSuwar,
}

/// What the last scan learned about where every game's artwork lives.
///
/// Managed state, replaced wholesale by each scan. It exists because
/// [`MasadirSuwar`] is produced by the launcher adapters and by nothing else:
/// without it, resolving artwork after the grid is drawn would mean running the
/// whole discovery pipeline a second time.
///
/// Read rather than drained, so a screen that remounts and asks again gets the
/// same answer instead of an empty one. On a warm cache the repeat costs a hash
/// and a `stat` per game.
#[derive(Debug, Default)]
pub struct DhakiratSuwar(Mutex<BTreeMap<LubaId, TalabSuwar>>);

impl DhakiratSuwar {
    /// Replaces everything remembered with what this scan found.
    pub fn ikhzin(&self, dufa: BTreeMap<LubaId, TalabSuwar>) {
        *self.0.lock() = dufa;
    }

    /// A copy of everything remembered.
    #[must_use]
    pub fn lamha(&self) -> BTreeMap<LubaId, TalabSuwar> {
        self.0.lock().clone()
    }
}

// ---------------------------------------------------------------------------
// The wire
// ---------------------------------------------------------------------------

/// One game's artwork, the moment it lands.
///
/// Both pictures are absolute paths under the artwork cache, which is the one
/// directory registered with the asset protocol; the interface turns either
/// into an `<img>` source with `convertFileSrc`. A key is deliberately not sent:
/// it is a content address, not a location, and resolving it is the check that
/// keeps a stored value from becoming a path outside the cache.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct GhilafHie {
    /// Taarib's identity for the game, matching the library row's `muarrif`.
    pub muarrif: String,
    /// The vertical cover, 600x900 in Steam's layout, or `None` when neither the
    /// launcher's own cache nor the address it publishes produced one.
    pub ghilaf: Option<String>,
    /// The wide landscape banner — the hero image where a launcher has one, its
    /// store header otherwise. Present far more often than the cover, because
    /// every launcher downloads a header for its own list and only some of them
    /// download the portrait.
    pub batl: Option<String>,
    /// The dominant colour of the cover, falling back to the banner.
    ///
    /// Extracted once at import and stored beside the image, so this costs a
    /// six-byte read rather than a second decode. It is what lets a card tint
    /// its own frame from the artwork it is showing without reading pixels back
    /// out of a canvas.
    pub lawn: Option<LawnBariz>,
}

/// What one artwork pass did.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HasilatSuwar {
    /// How many games were looked at.
    pub tumma: u32,
    /// How many ended with a vertical cover.
    pub bi_ghilaf: u32,
    /// How many ended with a landscape banner.
    pub bi_batl: u32,
    /// How many took their cover from a file the launcher already had.
    pub min_qurs: u32,
    /// How many took their cover from an address the launcher publishes.
    ///
    /// A *source* count, not a socket count: an address whose image is already
    /// in Taarib's cache is answered from disk and still counted here, because
    /// what this reports is where the picture originally came from.
    pub min_shabaka: u32,
    /// The name of every game that ended with no cover and no banner.
    pub bila_suwar: Vec<String>,
    /// How long the pass took, in milliseconds.
    pub muddat_ms: u32,
}

// ---------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------

/// Resolves the artwork the library scan did not already have, streaming each
/// game as it lands.
///
/// Called by the library screen once the grid is rendered. Every game the last
/// scan saw is offered to the artwork store: a source already in the cache is a
/// hash and a `stat`, and only a genuine miss opens a socket. Each finished game
/// is emitted on [`ISM_HADATH_GHILAF`] immediately; the returned
/// [`HasilatSuwar`] is the summary, not the data, so a caller that only wants
/// the covers can ignore it entirely.
///
/// Safe to call again. A second pass over a warm cache resolves from disk and
/// re-emits the same paths, which is what makes a screen that remounts
/// mid-flight recover on its own.
///
/// # Errors
///
/// [`taarib_usus::masarat::KhataMasarat::TaadhurInsha`] when the artwork cache
/// directory cannot be created — a read-only data root, or a full disk. Nothing
/// else fails: a launcher whose picture is missing, a server that refuses, an
/// image that will not decode are each one `None` in one event.
#[tauri::command]
#[specta::specta]
pub async fn hassil_suwar_maktaba(
    tatbiq: tauri::AppHandle,
    masarat: tauri::State<'_, Masarat>,
    qaida: tauri::State<'_, Makhzan>,
    dhakira: tauri::State<'_, DhakiratSuwar>,
) -> Result<HasilatSuwar, Khata> {
    let bidaya = Instant::now();
    let dufa = dhakira.lamha();
    let khazina = Arc::new(khazina(&masarat)?);
    let tasrih = Arc::new(Semaphore::new(HADD_ALAAB_MUTAWAZIYA));

    let mut mahammat: JoinSet<(LubaId, TalabSuwar, Option<SuwarLuba>)> = JoinSet::new();
    for (id, talab) in dufa {
        let khazina = Arc::clone(&khazina);
        let tasrih = Arc::clone(&tasrih);
        let _ = mahammat.spawn(async move {
            // A closed semaphore is not reachable — nothing here closes it — but
            // the fallible form is the only one, so a failure degrades to
            // running unbounded rather than to a panic.
            let _idhn = tasrih.acquire_owned().await.ok();
            let matlub = talab_bila_shiar_baeed(&talab.masadir);
            let natija = tokio::time::timeout(MUHLAT_LUBA, khazina.jalb(&matlub)).await;
            (id, talab, natija.ok())
        });
    }

    let mut hasila = HasilatSuwar {
        tumma: 0,
        bi_ghilaf: 0,
        bi_batl: 0,
        min_qurs: 0,
        min_shabaka: 0,
        bila_suwar: Vec::new(),
        muddat_ms: 0,
    };
    let mut mahsulat: Vec<HasilatLuba> = Vec::new();

    while let Some(intiha) = mahammat.join_next().await {
        let (id, talab, suwar) = match intiha {
            Ok(wahida) => wahida,
            // The workspace unwinds rather than aborts precisely so that one
            // malformed image costs one card instead of the process.
            Err(sabab) => {
                tracing::warn!(sabab = %sabab, "an artwork task did not finish");
                continue;
            },
        };

        hasila.tumma = hasila.tumma.saturating_add(1);
        let Some(suwar) = suwar else {
            tracing::warn!(
                luba = %id,
                ism = %talab.ism,
                thawan = MUHLAT_LUBA.as_secs(),
                "artwork gave up on this game and on no other"
            );
            hasila.bila_suwar.push(talab.ism);
            continue;
        };

        let mahsula = ijma_hasila(&khazina, id, &talab, suwar);
        if mahsula.hie.ghilaf.is_some() {
            hasila.bi_ghilaf = hasila.bi_ghilaf.saturating_add(1);
            match talab.masadir.ghilaf {
                Some(MasdarSura::Malaf(_)) => {
                    hasila.min_qurs = hasila.min_qurs.saturating_add(1);
                },
                Some(MasdarSura::Rabt(_)) => {
                    hasila.min_shabaka = hasila.min_shabaka.saturating_add(1);
                },
                None => {},
            }
        }
        if mahsula.hie.batl.is_some() {
            hasila.bi_batl = hasila.bi_batl.saturating_add(1);
        }
        if mahsula.hie.ghilaf.is_none() && mahsula.hie.batl.is_none() {
            hasila.bila_suwar.push(talab.ism.clone());
        }

        if let Err(sabab) = tatbiq.emit(ISM_HADATH_GHILAF, mahsula.hie.clone()) {
            tracing::debug!(sabab = %sabab, "an artwork report was not delivered");
        }
        mahsulat.push(mahsula);
    }

    // The writes are the one part of this that is neither concurrent nor
    // cancellable, so they go off the runtime and happen once, after every card
    // on screen already has its picture.
    let makhzan = Makhzan::clone(&qaida);
    if let Err(sabab) = tokio::task::spawn_blocking(move || dawwin(&makhzan, &mahsulat)).await {
        tracing::warn!(sabab = %sabab, "the artwork bookkeeping task did not finish");
    }

    hasila.bila_suwar.sort_unstable();
    hasila.muddat_ms = u32::try_from(bidaya.elapsed().as_millis()).unwrap_or(u32::MAX);
    tracing::info!(
        tumma = hasila.tumma,
        bi_ghilaf = hasila.bi_ghilaf,
        bi_batl = hasila.bi_batl,
        min_qurs = hasila.min_qurs,
        min_shabaka = hasila.min_shabaka,
        bila_suwar = hasila.bila_suwar.len(),
        muddat_ms = hasila.muddat_ms,
        "the artwork pass finished"
    );
    Ok(hasila)
}

// ---------------------------------------------------------------------------
// Shared with the scan
// ---------------------------------------------------------------------------

/// Opens the artwork cache at the resolved data root.
///
/// # Errors
///
/// [`taarib_usus::masarat::KhataMasarat::TaadhurInsha`] when the directory
/// cannot be created.
pub fn khazina(masarat: &Masarat) -> Natija<KhaziantSuwar> {
    KhaziantSuwar::jadeeda(masarat.makhbaa().join(MUJALLAD_SUWAR))
}

/// The absolute path a stored artwork key resolves to, when the file is really
/// there.
///
/// [`None`] for a key that names nothing — an entry evicted since it was
/// written, or a value that is not a key at all. The interface draws the
/// generated plate for both, which is the same answer it gives a game whose
/// artwork was never fetched.
#[must_use]
pub fn masar_sura(khazina: &KhaziantSuwar, miftah: &str) -> Option<String> {
    khazina
        .mahalli(miftah)
        .map(|masar| masar.to_string_lossy().into_owned())
}

/// Every game's stored artwork keys, in two reads.
///
/// Two statements over the game table — the visible rows and the hidden ones —
/// rather than one indexed lookup per game, which on a library of six hundred is
/// six hundred round trips to draw one grid. Hidden games are included because
/// unhiding one must not cost it the cover it already had.
///
/// Answers with an empty map when the store cannot be read. Artwork is never a
/// reason to fail a scan: a library drawn entirely in plates is still a library,
/// and a library that refused to appear is not.
#[must_use]
pub fn suwar_makhzuna(makhzan: &Makhzan) -> BTreeMap<LubaId, SuwarLuba> {
    let mut natija = BTreeMap::new();
    for mukhfiya in [false, true] {
        let talab = TalabMaktaba {
            mukhfiya,
            ..TalabMaktaba::default()
        };
        match makhzan.bil_qira(|ittisal| SijillAlaab::jadeed(ittisal).qaima(&talab)) {
            Ok(sufuf) => {
                for luba in sufuf {
                    let _ = natija.insert(luba.id, luba.suwar);
                }
            },
            Err(khata) => {
                tracing::warn!(khata = %khata.li_sijill(), "stored artwork was not read");
            },
        }
    }
    natija
}

/// One game's stored artwork, as the library row carries it.
///
/// Reads nothing and fetches nothing: the keys come from the row the last
/// artwork pass wrote, and each is turned into a path only if the file is still
/// in the cache.
#[must_use]
pub fn ghilaf_makhzun(khazina: &KhaziantSuwar, suwar: &SuwarLuba) -> GhilafMakhzun {
    GhilafMakhzun {
        ghilaf: suwar
            .ghilaf
            .as_deref()
            .and_then(|miftah| masar_sura(khazina, miftah)),
        batl: suwar
            .batl
            .as_deref()
            .and_then(|miftah| masar_sura(khazina, miftah)),
        lawn: suwar.lawn,
    }
}

/// What the cache can answer for one game without touching the network.
#[derive(Debug, Clone, Default)]
pub struct GhilafMakhzun {
    /// The vertical cover's path, when the cache still holds it.
    pub ghilaf: Option<String>,
    /// The landscape banner's path, likewise.
    pub batl: Option<String>,
    /// The colour the cover was imported with.
    pub lawn: Option<LawnBariz>,
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// The sources to resolve for one game.
///
/// Everything on disk is kept. A logo that exists only as an address is dropped:
/// no screen draws one, and the fetch budget belongs to the two pictures the
/// card does draw.
fn talab_bila_shiar_baeed(masadir: &MasadirSuwar) -> MasadirSuwar {
    MasadirSuwar {
        ghilaf: masadir.ghilaf.clone(),
        batl: masadir.batl.clone(),
        shiar: masadir
            .shiar
            .clone()
            .filter(|masdar| matches!(masdar, MasdarSura::Malaf(_))),
    }
}

/// One finished game: what to announce, and what to write down.
#[derive(Debug)]
struct HasilatLuba {
    /// Taarib's identity for the game.
    id: LubaId,
    /// The keys and the colour, as the game's row stores them.
    suwar: SuwarLuba,
    /// A ledger row for every image this game now points at, which has to exist
    /// before the game may point at it.
    sujill: Vec<SuratMukhzana>,
    /// What the interface is told.
    hie: GhilafHie,
}

/// Turns one resolved game into the event and the rows behind it.
fn ijma_hasila(
    khazina: &KhaziantSuwar,
    id: LubaId,
    talab: &TalabSuwar,
    suwar: SuwarLuba,
) -> HasilatLuba {
    let mut sujill = Vec::new();
    let khanat = [
        (
            NAW_GHILAF,
            suwar.ghilaf.as_deref(),
            talab.masadir.ghilaf.as_ref(),
        ),
        (NAW_BATL, suwar.batl.as_deref(), talab.masadir.batl.as_ref()),
        (
            NAW_SHIAR,
            suwar.shiar.as_deref(),
            talab.masadir.shiar.as_ref(),
        ),
    ];

    // Two entries for three slots, in the order above: the interface is handed
    // the cover and the banner, and the logo is stored and written down without
    // being sent, because no screen draws one. The third index therefore falls
    // off the end of this array on purpose.
    let mut masarat: [Option<String>; 2] = [None, None];
    for (fahras, (naw, miftah, masdar)) in khanat.into_iter().enumerate() {
        let Some(miftah) = miftah else { continue };
        let Some(masar) = khazina.mahalli(miftah) else {
            continue;
        };
        sujill.push(SuratMukhzana {
            miftah: miftah.to_owned(),
            naw: naw.to_owned(),
            masdar: nass_masdar(masdar),
            hajm: std::fs::metadata(&masar).map_or(0, |bayanat| bayanat.len()),
            // Read from the header rather than guessed, or not read at all. The
            // studio has no image decoder of its own, and adding one to fill two
            // advisory columns would put a second decoder in the process for no
            // behaviour: the ledger evicts by recency and dedupes by key, and
            // neither consults a dimension.
            ard: None,
            irtifa: None,
        });
        if let Some(marsal) = masarat.get_mut(fahras) {
            *marsal = Some(masar.to_string_lossy().into_owned());
        }
    }

    let [ghilaf, batl] = masarat;
    HasilatLuba {
        id,
        hie: GhilafHie {
            muarrif: id.to_string(),
            ghilaf,
            batl,
            lawn: suwar.lawn,
        },
        suwar,
        sujill,
    }
}

/// The address or path an image came from, for the ledger's own record.
fn nass_masdar(masdar: Option<&MasdarSura>) -> Option<String> {
    match masdar? {
        MasdarSura::Malaf(masar) => Some(masar.to_string_lossy().into_owned()),
        MasdarSura::Rabt(rabt) => Some(rabt.clone()),
    }
}

// ---------------------------------------------------------------------------
// Bookkeeping
// ---------------------------------------------------------------------------

/// Writes every resolved game's artwork onto its row.
///
/// One transaction for the whole pass, with each game's writes isolated inside
/// it: a row that a constraint rejects is logged and skipped, and the games
/// around it still commit. The alternative — a transaction per game — is one
/// durable write per card on a library of six hundred, and the alternative to
/// *that* — one `?` out of the loop — is fifteen games losing their covers
/// because a sixteenth had a bad row.
///
/// Never returns a failure. Everything here is a cache pointer: the images are
/// already on disk and already on screen, and a pass that could not write them
/// down costs the next launch a hash and a `stat` per game, which is what the
/// artwork stage does anyway.
fn dawwin(makhzan: &Makhzan, mahsulat: &[HasilatLuba]) {
    let natija = makhzan.bi_muamala(|muamala| {
        let mut maktuba = 0_u32;
        for wahida in mahsulat {
            let mut salima = true;
            for sura in &wahida.sujill {
                // The game's row carries a foreign key onto this ledger, so the
                // image has to be known here before anything may point at it.
                if let Err(khata) = SijillSuwar::jadeed(muamala).sajjil(sura) {
                    tracing::warn!(
                        luba = %wahida.id,
                        miftah = %sura.miftah,
                        khata = %khata.li_sijill(),
                        "an artwork ledger row was refused"
                    );
                    salima = false;
                }
            }
            if !salima {
                continue;
            }
            match SijillAlaab::jadeed(muamala).sajjil_suwar(wahida.id, &wahida.suwar) {
                Ok(()) => maktuba = maktuba.saturating_add(1),
                Err(khata) => tracing::warn!(
                    luba = %wahida.id,
                    khata = %khata.li_sijill(),
                    "a game's artwork keys were not written"
                ),
            }
        }
        Ok(maktuba)
    });

    match natija {
        Ok(maktuba) => {
            tracing::debug!(maktuba, min = mahsulat.len(), "artwork keys written");
        },
        Err(khata) => {
            tracing::warn!(khata = %khata.li_sijill(), "the artwork pass wrote nothing down");
        },
    }
}

/// The artwork cache root, for a caller that only needs the path.
///
/// Exists so the asset protocol scope and the store agree on one spelling of
/// the directory instead of two.
#[must_use]
pub fn jidhr_suwar(masarat: &Masarat) -> PathBuf {
    masarat.makhbaa().join(MUJALLAD_SUWAR)
}
