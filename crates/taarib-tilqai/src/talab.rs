//! Everything one automatic run is handed.

use std::path::{Path, PathBuf};

use taarib_aman::iqrar::SijillIqrar;
use taarib_aman::qaimat_sahb::QaimatSahb;
use taarib_khatm::{MiftahKhass, MirsatThiqa};
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::RukhsaRuqaa;
use taarib_tarjama::muzawwidun::Muzawwid;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

use crate::ilgha::MiqbadIlgha;
use crate::mashwar::MashwarId;
use crate::taqaddum::MukhbirTaqaddum;

/// The default sizes every string is laid out at, in pixels.
///
/// Several sizes, not one, and not "whatever the game declares": a game that
/// exposes no font size anywhere still draws its menu at one size and its
/// subtitles at another, and a patch carrying a single size has a bitmap for
/// one of them and a rescale for the rest. These are the sizes the Studio's own
/// engine-wide reports supply when nothing measured is available; a run with
/// runtime capture behind it will find more in the table and lay out at those
/// too.
///
/// It was 18, 24 and 32 alone, and the top of that range is the problem: a
/// title, a headline on an in-game screen, a large menu label are all drawn far
/// above 32, and a glyph rasterised at 32 and magnified to 64 is visibly soft
/// beside the engine's own text, which is an outline scaled to any size. The
/// first game patched by somebody other than its author showed exactly that —
/// a crisp English paragraph under a smudged Arabic heading.
///
/// Doubling upward rather than adding a few: each size is an atlas of its own,
/// so the cost of one more is real, and powers of two mean any size a game
/// picks is within a factor of √2 of one that was rasterised — the point at
/// which magnification stops being visible.
pub const AHJAM_IFTIRADIYA: &[f32] = &[18.0, 24.0, 32.0, 48.0, 64.0, 96.0];

/// The name the patch is written under inside the game's `taarib/` folder.
pub const WIJHAT_IFTIRADIYA: &str = "nusus.ruqaa";

/// The game this run is about.
#[derive(Debug, Clone)]
pub struct LubaTilqai<'a> {
    /// Its install root. Read for extraction and fingerprinting; written only
    /// by the install stage, and only through `taarib_tathbeet`'s guard.
    pub jidhr: &'a Path,
    /// Its name, as discovery reported it.
    pub ism: &'a str,
    /// Which launcher it came from, which is half of its identity.
    pub masdar: MasdarLuba,
    /// Its executable, when discovery found one. The probe is materially worse
    /// without it — several detectors read the binary — so a caller that has
    /// one should always pass it.
    pub tanfidhi: Option<&'a Path>,
    /// The operating system the *files on disk* are for, which on a Steam Deck
    /// is not the operating system this process is running on.
    pub nizam: NizamTashghil,
    /// The compatibility layer it runs under, if any.
    pub beea: &'a BeeatTawafuq,
}

impl LubaTilqai<'_> {
    /// The game's identity, derived the way the rest of the product derives it.
    #[must_use]
    pub fn huwiya(&self) -> LubaId {
        LubaId::min_masdar(&self.masdar, self.ism)
    }
}

/// The three records the safety gate reads before it permits an install.
///
/// None of them is minted here. A revocation list this crate invented would be
/// a revocation list nobody signed, and a first-run acknowledgement this crate
/// wrote would be an acknowledgement nobody gave; both come from the caller,
/// which is the only party that can have obtained them honestly.
#[derive(Debug, Clone, Copy)]
pub struct MudkhalatAman<'a> {
    /// The trust anchor the package's signature is checked against.
    pub mirsa: &'a MirsatThiqa,
    /// The verified revocation list.
    pub qaima: &'a QaimatSahb,
    /// The first-run acknowledgement, when one has been recorded.
    pub iqrar: Option<&'a SijillIqrar>,
    /// The Steam application id, when the game is a Steam game.
    pub appid: Option<u32>,
    /// The Steam library root, when there is one.
    pub jidhr_steam: Option<&'a Path>,
    /// Whether the user acknowledged the multiplayer warning.
    pub iqrar_shabaka: bool,
}

/// What the finished patch declares about itself.
#[derive(Debug, Clone)]
pub struct WasfTilqai {
    /// The patch's title. Defaulted from the game's name when absent.
    pub unwan: Option<String>,
    /// The contributor's display name.
    pub ism_musahim: String,
    /// The licence the patch is published under.
    pub rukhsa: RukhsaRuqaa,
    /// The version of Taarib that built it.
    pub isdar_taarib: String,
}

/// How a run behaves. Everything a user can turn.
#[derive(Debug, Clone)]
pub struct KhiyaratTilqai {
    /// The cost ceiling, in nano-dollars, or [`None`] for no ceiling.
    ///
    /// Cumulative across resumes of the same run, because the translation
    /// journal's recorded spend seeds the ledger. A user who wants to spend
    /// more raises it; a user on the loopback provider never meets it, because
    /// a free provider's every estimate is zero.
    pub saqf_takalif: Option<u64>,
    /// How many requests may be in flight at once.
    pub tawazi: usize,
    /// How many attempts one string gets before it is failed with its reason.
    pub aqsa_muhawalat: u32,
    /// The sizes every string is laid out at, in pixels.
    pub ahjam: Vec<f32>,
    /// Whether to install at the end.
    ///
    /// `false` stops after signing, with the sealed package in the run
    /// directory. The one option that changes whether the run ever touches the
    /// game at all.
    pub yathbut: bool,
    /// The file name the patch is written under inside `taarib/`.
    pub wijha: String,
    /// The run's moment, seconds since the Unix epoch.
    ///
    /// Supplied by the caller, never read from a clock here: this crate stamps
    /// journal lines and review transitions, and a record whose time came from
    /// inside it would be unreproducible.
    pub lahza: u64,
    /// The same moment as RFC 3339.
    pub waqt: String,
}

impl KhiyaratTilqai {
    /// Options with this build's defaults, at a caller-supplied moment.
    #[must_use]
    pub fn jadeeda(lahza: u64, waqt: impl Into<String>) -> Self {
        Self {
            saqf_takalif: None,
            tawazi: 4,
            aqsa_muhawalat: 3,
            ahjam: AHJAM_IFTIRADIYA.to_vec(),
            yathbut: true,
            wijha: WIJHAT_IFTIRADIYA.to_owned(),
            lahza,
            waqt: waqt.into(),
        }
    }
}

/// One automatic run.
///
/// Every field is a borrow except the two owned option groups, and that
/// asymmetry is deliberate: the borrows are things the caller already has and
/// this crate must not copy — a game directory, a signing key, a provider — and
/// the owned ones are decisions the run carries into its own journal.
pub struct TalabTilqai<'a> {
    /// The run's identity. A resumed run passes the same one.
    pub id: MashwarId,
    /// The runs root. This run's directory is `<root>/<id>`.
    pub jidhr_amal: &'a Path,
    /// The game.
    pub luba: LubaTilqai<'a>,
    /// The translation provider. A loopback provider costs nothing and the
    /// whole run completes on it.
    pub muzawwid: &'a dyn Muzawwid,
    /// The key the patch is sealed with. Never leaves this process, and never
    /// reaches `taarib-tarqee`.
    pub miftah: &'a MiftahKhass,
    /// The fonts to bundle, in chain order: the first is the one shaping
    /// reaches for and every one after it is a fallback.
    pub khutut: &'a [PathBuf],
    /// A runtime capture session to merge into the static extraction.
    ///
    /// This is the second half of the honest-failure path: a game whose strings
    /// cannot be read statically is told to play once with capture enabled, and
    /// this is where the file it produced comes back in.
    pub jalsat_iltiqat: Option<&'a Path>,

    /// The component store the framework is deployed from, when the caller has
    /// one.
    ///
    /// Deciding *what* framework and adapter a tier needs is
    /// `taarib_tathbeet::tarkib`'s job, and it needs a store to take the
    /// binaries from — which is why this is the caller's to supply and not this
    /// crate's to invent. What was wrong was leaving it out altogether: without
    /// it the run writes the patch and its fonts into the game and no loader,
    /// so on a game that had never been patched before nothing reads what was
    /// written, the run reports success, and the game starts in its original
    /// language with the whole translation sitting on disk beside it.
    pub mukawwinat: Option<&'a Path>,

    /// The workshop's project store, when the caller keeps one.
    ///
    /// The run writes its rows here, under the game's identity, as soon as the
    /// translation stage has finished with them. Without it the run's table
    /// lives only in the run directory under the run's own identifier, and the
    /// workshop — which opens one project per game — has nothing to show: it
    /// tells the user no project exists for this game yet and to start a
    /// translation, immediately after they finished one. The rows the workshop
    /// already holds are never overwritten; see [`crate::warsha::anshir`].
    pub mashari: Option<&'a Path>,
    /// What the finished patch declares.
    pub wasf: WasfTilqai,
    /// The three records the install gate reads.
    pub aman: MudkhalatAman<'a>,
    /// How the run behaves.
    pub khiyarat: KhiyaratTilqai,
    /// Where progress goes.
    pub mukhbir: &'a MukhbirTaqaddum,
    /// How the run is stopped.
    pub miqbad: &'a MiqbadIlgha,
}

impl std::fmt::Debug for TalabTilqai<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The key is deliberately absent: a `{:?}` of a run request must never
        // be a way to get a private key into a log.
        f.debug_struct("TalabTilqai")
            .field("id", &self.id)
            .field("jidhr_amal", &self.jidhr_amal)
            .field("luba", &self.luba)
            .field("muzawwid", &self.muzawwid.ism())
            .field("khutut", &self.khutut)
            .field("jalsat_iltiqat", &self.jalsat_iltiqat)
            .field("mukawwinat", &self.mukawwinat)
            .field("mashari", &self.mashari)
            .field("wasf", &self.wasf)
            .field("khiyarat", &self.khiyarat)
            .finish_non_exhaustive()
    }
}

impl TalabTilqai<'_> {
    /// This run's directory.
    #[must_use]
    pub fn mujallad(&self) -> PathBuf {
        self.id.mujallad(self.jidhr_amal)
    }

    /// The patch's title, defaulted from the game's name.
    #[must_use]
    pub fn unwan(&self) -> String {
        self.wasf
            .unwan
            .clone()
            .unwrap_or_else(|| format!("{} — Arabic", self.luba.ism))
    }
}
