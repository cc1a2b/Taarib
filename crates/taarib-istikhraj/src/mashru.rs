//! المشروع — the translation project on disk: resumable, versioned, and
//! diffable against a newer build.
//!
//! The output of extraction is not a table in memory. It is a directory a
//! contributor can close their laptop on, reopen a month later, and continue —
//! and that Phase 20F's card entry opens, Phase 13 translates inside, Phase 14
//! compiles, and Phase 19 reconciles between two people.
//!
//! ## Written as extraction proceeds, not after it
//!
//! A large Unity game is tens of thousands of strings across hundreds of
//! containers, and reading them all takes minutes. Writing the project only at
//! the end means a contributor stares at a progress bar with nothing to look
//! at, and a crash at ninety percent costs everything.
//!
//! So [`Mashru::adif_dufa`] commits a batch at a time and the workshop can open
//! the project while extraction is still running. The cost is that a project
//! can exist in an incomplete state, which is why [`Mashru::muktamil`] is a
//! recorded fact rather than an inference from the file being present.
//!
//! ## Re-extraction is a diff, never a replacement
//!
//! This is where stable identity pays for itself. Re-extracting against a newer
//! build produces [`crate::jadwal::FarqJadwal`], and
//! [`Mashru::hajir`] migrates every translation whose identity survived. What
//! the contributor is shown afterwards is the genuinely new work and nothing
//! else.
//!
//! **A string that vanished keeps its translation.** It is marked orphaned and
//! retained, because a string that disappeared in one build routinely returns
//! in the next — a seasonal event, a disabled feature, a hotfix that reverted.
//! A contributor who lost four hundred lines to a publisher's hotfix would not
//! use this product again, and they would be right not to.
//!
//! ## What the provenance record is for
//!
//! [`BayanIstikhraj`] records the engine, the build, the container fingerprints,
//! which method produced each entry, and what was refused. That is not
//! bookkeeping: Phase 15 compares the recorded build against the installed one
//! to decide whether a re-extraction is needed at all, and the refusal report is
//! what tells a contributor returning to an old project why it only ever had
//! seventy percent of the game.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::nass::{MudkhalNass, NassId};
use taarib_usus::masarat;
use taarib_usus::mukhattat::{self, DhuMukhattat};

use crate::jadwal::{FarqJadwal, JadwalNusus};
use crate::khata::KhataIstikhraj;
use crate::rafd::TaqreerRafd;

/// The project file's schema version.
///
/// Bumped when the on-disk shape changes incompatibly. An older build meeting a
/// newer project **refuses** rather than migrating — see
/// [`KhataIstikhraj::MukhattatGhayrMafhum`]. A project is somebody's work, and a
/// migration that guessed wrong would corrupt it in a way they would not notice
/// until they shipped.
pub const ISDAR_MUKHATTAT: u32 = 1;

/// The project's own file, inside the project directory.
pub const MALAF_MASHRU: &str = "mashru.json";

/// The strings, written as JSON Lines so a batch can be appended.
///
/// One object per line, appended as extraction proceeds. A single JSON array
/// would have to be rewritten whole on every batch, which for a fifty-thousand
/// string game is a re-serialization per container.
pub const MALAF_NUSUS: &str = "nusus.jsonl";

/// How many strings are committed at a time.
///
/// Five hundred. Small enough that a crash costs a fraction of a second's work
/// and the workshop sees results quickly; large enough that the write is not
/// the bottleneck. The number is a trade between those two and nothing else.
pub const HAJM_DUFA: usize = 500;

/// Which method produced an entry, recorded per container rather than per
/// string.
///
/// Per container because that is the granularity at which the answer differs: a
/// `.locres` was read one way and every string in it the same way. Recording it
/// per string would be the same value stored ten thousand times.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TareeqatHawiya {
    /// The container.
    pub hawiya: String,
    /// How it was read, in words: `locres v3`, `type tree`, `runtime capture`.
    pub tareeqa: String,
    /// Its fingerprint at extraction time, so Phase 15 can tell whether it
    /// changed without re-reading it.
    pub basma: Option<Basma>,
}

/// Everything about how a project's strings were obtained.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BayanIstikhraj {
    /// The engine family, as Phase 5 identified it.
    pub aila: String,
    /// The engine version, where it was determined.
    pub isdar: Option<String>,
    /// The launcher's build identifier at extraction time.
    pub bina_manassa: Option<String>,
    /// The game's content fingerprint at extraction time.
    ///
    /// What Phase 15 compares against to decide whether the installed game is
    /// still the one this project was built from.
    pub basmat_luba: Option<Basma>,
    /// How each container was read.
    pub turuq: Vec<TareeqatHawiya>,
    /// What was refused, and why.
    ///
    /// Stored in the project rather than shown once and discarded: a
    /// contributor returning after a month needs to be able to answer "why does
    /// this project only have the menus" without re-running extraction.
    pub rafd: TaqreerRafd,
    /// When extraction ran, RFC 3339.
    pub waqt: String,
    /// Which Taarib build ran it.
    pub isdar_taarib: String,
}

/// A translation project.
///
/// The header only — the strings live beside it in [`MALAF_NUSUS`] and are
/// streamed rather than held, because a fifty-thousand-string project loaded
/// whole to answer "what is this project" is fifty thousand allocations for a
/// title and a count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mashru {
    /// Which game.
    pub luba: LubaId,
    /// The game's display name, for the workshop's title bar.
    pub ism_luba: String,
    /// How the strings were obtained.
    pub bayan: BayanIstikhraj,
    /// How many strings the project holds.
    pub adad: usize,
    /// Whether extraction finished.
    ///
    /// A recorded fact rather than an inference from the file existing, because
    /// the project is written incrementally and an interrupted extraction
    /// leaves a perfectly well-formed file holding half a game.
    pub muktamil: bool,
    /// When it was created, RFC 3339.
    pub waqt_insha: String,
    /// When it was last written, RFC 3339.
    pub waqt_tabdeel: String,
}

impl DhuMukhattat for Mashru {
    const ISM: &'static str = "mashru";

    const ISDAR: u32 = ISDAR_MUKHATTAT;

    fn hijra(min: u32, _qeema: serde_json::Value) -> taarib_usus::khata::Natija<serde_json::Value> {
        // No migration exists and none is invented. A project is a
        // contributor's work; a migration that guessed at a shape it had never
        // seen would corrupt it silently, and the corruption would surface
        // after they shipped. Refusing costs them an update prompt, which is
        // the cheaper of the two by a wide margin.
        Err(taarib_usus::khata::Khata::from(KhataIstikhraj::MukhattatGhayrMafhum {
            masar: PathBuf::from(MALAF_MASHRU),
            wujid: min,
            madum: ISDAR_MUKHATTAT,
        }))
    }
}

/// A project open on disk, with its strings streamed rather than held.
#[derive(Debug)]
pub struct MashruMaftuh {
    jidhr: PathBuf,
    rasm: Mashru,
    dufa: Vec<MudkhalNass>,
}

impl MashruMaftuh {
    /// Creates a project directory and writes its header.
    ///
    /// The header is written **before** any string, so an extraction that dies
    /// on its first container still leaves a project that names the game, the
    /// engine, and the fact that it is incomplete. A directory holding strings
    /// and no header would be a project nothing could open.
    ///
    /// # Errors
    ///
    /// [`KhataIstikhraj::TaadhurKitabatMashru`] when the directory cannot be
    /// made or the header cannot be written.
    pub fn ansha(
        jidhr: PathBuf,
        luba: LubaId,
        ism_luba: String,
        bayan: BayanIstikhraj,
        waqt: String,
    ) -> Result<Self, KhataIstikhraj> {
        masarat::insha_mujallad(&jidhr).map_err(|khata| {
            KhataIstikhraj::TaadhurKitabatMashru {
                masar: jidhr.clone(),
                sabab: khata.injilizi,
            }
        })?;

        let rasm = Mashru {
            luba,
            ism_luba,
            bayan,
            adad: 0,
            muktamil: false,
            waqt_insha: waqt.clone(),
            waqt_tabdeel: waqt,
        };
        let maftuh = Self { jidhr, rasm, dufa: Vec::with_capacity(HAJM_DUFA) };
        maftuh.iktub_rasm()?;
        Ok(maftuh)
    }

    /// Reopens a project that already exists on disk.
    ///
    /// # Errors
    ///
    /// [`KhataIstikhraj::TaadhurKitabatMashru`] naming the header when it is
    /// absent, unreadable, or written by a newer schema.
    pub fn iftah(jidhr: PathBuf) -> Result<Self, KhataIstikhraj> {
        let masar = jidhr.join(MALAF_MASHRU);
        let rasm: Mashru = mukhattat::iqra_malaf(&masar).map_err(|khata| {
            KhataIstikhraj::TaadhurKitabatMashru { masar, sabab: khata.injilizi }
        })?;
        Ok(Self { jidhr, rasm, dufa: Vec::with_capacity(HAJM_DUFA) })
    }

    /// Replaces the whole string table atomically.
    ///
    /// A crash mid-save leaves the previous table whole rather than a torn one.
    ///
    /// # Errors
    ///
    /// [`KhataIstikhraj::TaadhurKitabatMashru`] when serialization or the
    /// atomic write fails.
    pub fn uktub_kul(
        &mut self,
        madakhil: &[MudkhalNass],
        waqt: String,
    ) -> Result<(), KhataIstikhraj> {
        let masar = self.jidhr.join(MALAF_NUSUS);
        let mut bayt = Vec::new();
        for mudkhal in madakhil {
            let satr = serde_json::to_vec(mudkhal).map_err(|khata| {
                KhataIstikhraj::TaadhurKitabatMashru {
                    masar: masar.clone(),
                    sabab: khata.to_string(),
                }
            })?;
            bayt.extend_from_slice(&satr);
            bayt.push(b'\n');
        }
        masarat::kitaba_dharra(&masar, &bayt).map_err(|khata| {
            KhataIstikhraj::TaadhurKitabatMashru { masar, sabab: khata.injilizi }
        })?;
        self.dufa.clear();
        self.rasm.adad = madakhil.len();
        self.rasm.waqt_tabdeel = waqt;
        self.iktub_rasm()
    }

    /// The project's directory.
    #[must_use]
    pub fn jidhr(&self) -> &Path {
        &self.jidhr
    }

    /// The header.
    #[must_use]
    pub const fn rasm(&self) -> &Mashru {
        &self.rasm
    }

    /// Adds strings, committing whenever a batch fills.
    ///
    /// # Errors
    ///
    /// [`KhataIstikhraj::TaadhurKitabatMashru`] when a batch cannot be written.
    pub fn adif(&mut self, madakhil: Vec<MudkhalNass>) -> Result<(), KhataIstikhraj> {
        for mudkhal in madakhil {
            self.dufa.push(mudkhal);
            if self.dufa.len() >= HAJM_DUFA {
                self.adif_dufa()?;
            }
        }
        Ok(())
    }

    /// Commits whatever is pending.
    ///
    /// Appends rather than rewriting, which is the whole reason the strings are
    /// JSON Lines: a fifty-thousand-string project re-serialized on every batch
    /// would spend more time writing than extracting.
    ///
    /// # Errors
    ///
    /// [`KhataIstikhraj::TaadhurKitabatMashru`] when the append fails.
    pub fn adif_dufa(&mut self) -> Result<(), KhataIstikhraj> {
        if self.dufa.is_empty() {
            return Ok(());
        }
        let masar = self.jidhr.join(MALAF_NUSUS);
        let mut nass = String::with_capacity(self.dufa.len().saturating_mul(256));
        for mudkhal in &self.dufa {
            match serde_json::to_string(mudkhal) {
                Ok(satr) => {
                    nass.push_str(&satr);
                    nass.push('\n');
                }
                Err(sabab) => {
                    return Err(KhataIstikhraj::TaadhurKitabatMashru {
                        masar,
                        sabab: sabab.to_string(),
                    });
                }
            }
        }

        ilhaq(&masar, nass.as_bytes())?;
        self.rasm.adad = self.rasm.adad.saturating_add(self.dufa.len());
        self.dufa.clear();
        Ok(())
    }

    /// Commits the last batch and marks the project complete.
    ///
    /// # Errors
    ///
    /// As [`MashruMaftuh::adif_dufa`], plus a failure to rewrite the header.
    pub fn ikhtim(&mut self, waqt: String) -> Result<(), KhataIstikhraj> {
        self.adif_dufa()?;
        self.rasm.muktamil = true;
        self.rasm.waqt_tabdeel = waqt;
        self.iktub_rasm()
    }

    /// Writes the header atomically.
    fn iktub_rasm(&self) -> Result<(), KhataIstikhraj> {
        let masar = self.jidhr.join(MALAF_MASHRU);
        let bayt = mukhattat::iktub(&self.rasm).map_err(|khata| {
            KhataIstikhraj::TaadhurKitabatMashru { masar: masar.clone(), sabab: khata.injilizi }
        })?;
        masarat::kitaba_dharra(&masar, &bayt).map_err(|khata| {
            KhataIstikhraj::TaadhurKitabatMashru { masar, sabab: khata.injilizi }
        })
    }

    /// Reads every string back.
    ///
    /// A malformed line is **skipped with a count**, not fatal. A project is a
    /// contributor's work and one torn line — which an interrupted append
    /// genuinely produces — must not cost them the other forty thousand.
    ///
    /// # Errors
    ///
    /// [`KhataIstikhraj::TaadhurKitabatMashru`] when the file exists and cannot
    /// be read at all. A file that is simply absent yields an empty list, which
    /// is what a project whose extraction died before its first batch looks
    /// like.
    pub fn iqra_nusus(&self) -> Result<(Vec<MudkhalNass>, usize), KhataIstikhraj> {
        let masar = self.jidhr.join(MALAF_NUSUS);
        if !masar.is_file() {
            return Ok((Vec::new(), 0));
        }
        let bayt = std::fs::read(&masar).map_err(|sabab| {
            KhataIstikhraj::TaadhurKitabatMashru { masar, sabab: sabab.to_string() }
        })?;

        let mut madakhil = Vec::new();
        let mut talifa = 0_usize;
        // Split on bytes rather than decoding the whole file first: an
        // interrupted append leaves invalid UTF-8 at the tail, and decoding
        // first would turn one torn line into a total refusal.
        for satr in bayt.split(|bayt| *bayt == b'\n') {
            if satr.is_empty() {
                continue;
            }
            match std::str::from_utf8(satr).ok().and_then(|nass| {
                serde_json::from_str::<MudkhalNass>(nass).ok()
            }) {
                Some(mudkhal) => madakhil.push(mudkhal),
                None => talifa = talifa.saturating_add(1),
            }
        }
        Ok((madakhil, talifa))
    }
}

/// Appends bytes to a file, creating it if it does not exist.
///
/// Not atomic, and deliberately: an atomic append is a full rewrite, which is
/// exactly what JSON Lines exists to avoid. The failure this exposes is a torn
/// last line after a crash, and [`MashruMaftuh::iqra_nusus`] handles that by
/// skipping it — a bounded, recoverable cost, against re-serializing the whole
/// project five hundred strings at a time.
fn ilhaq(masar: &Path, bayt: &[u8]) -> Result<(), KhataIstikhraj> {
    use std::io::Write as _;

    let mut malaf = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(masar)
        .map_err(|sabab| KhataIstikhraj::TaadhurKitabatMashru {
            masar: masar.to_path_buf(),
            sabab: sabab.to_string(),
        })?;
    malaf.write_all(bayt).map_err(|sabab| KhataIstikhraj::TaadhurKitabatMashru {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

/// What a re-extraction did to an existing project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaqreerHijra {
    /// How many translations moved across untouched.
    pub muhajjara: usize,
    /// How many strings are new and untranslated.
    pub jadeeda: usize,
    /// How many kept their identity and changed their source text.
    pub mughayyara: usize,
    /// How many are no longer in the build, whose translations were kept.
    pub yatima: usize,
    /// How many the diff said were removed.
    ///
    /// Normally equal to [`TaqreerHijra::yatima`]. A disagreement is not a bug
    /// to assert against — it means the project held rows the newly extracted
    /// table never contained, which a hand-added string or a project from an
    /// older schema legitimately produces. Recorded so the diagnostics bundle
    /// can show both numbers rather than one of them silently winning.
    pub mahdhufa_fi_alfarq: usize,
}

impl TaqreerHijra {
    /// The sentence the update screen shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} translation(s) carried over automatically, {} new string(s) to translate, {} \
             whose source text changed and need review, {} no longer in this build (their \
             translations are kept in case the strings return)",
            self.muhajjara, self.jadeeda, self.mughayyara, self.yatima
        )
    }
}

/// Migrates an existing project's translations onto a freshly extracted table.
///
/// The payoff for stable identity, and the operation Phase 15 runs after a store
/// update. Every entry whose identity survived keeps its translation, its
/// status, and its history.
///
/// A string whose **source text changed** under a surviving identity keeps its
/// translation and is flagged for review rather than being
/// cleared. The old Arabic is very often still right — a typo fixed in the
/// English, a trailing space removed — and throwing it away would make a
/// publisher's punctuation pass cost a contributor a day. Flagging it puts the
/// judgement where it belongs.
///
/// A string that **vanished** keeps everything and is marked orphaned. See this
/// module's header for why.
///
/// `lahza` is seconds since the Unix epoch, supplied by the caller because this
/// crate reads no clock — the same discipline every timestamped operation in
/// this product follows, so a migration replayed against the same inputs
/// produces the same history.
#[must_use]
pub fn hajir(
    qadeem: Vec<MudkhalNass>,
    jadeed: &JadwalNusus,
    farq: &FarqJadwal,
    lahza: u64,
) -> (Vec<MudkhalNass>, Vec<MudkhalNass>, TaqreerHijra) {
    let mut sabiq: BTreeMap<NassId, MudkhalNass> =
        qadeem.into_iter().map(|mudkhal| (mudkhal.id, mudkhal)).collect();
    let mut taqreer = TaqreerHijra::default();

    let mut hali: Vec<MudkhalNass> = Vec::with_capacity(jadeed.adad());
    for mut mudkhal in jadeed.ila_mudkhalat() {
        match sabiq.remove(&mudkhal.id) {
            Some(qadeema) => {
                let taghayyar = qadeema.masdar != mudkhal.masdar;
                mudkhal.hadaf = qadeema.hadaf;
                mudkhal.nasq_hadaf = qadeema.nasq_hadaf;
                mudkhal.tareeqa = qadeema.tareeqa;
                mudkhal.muzawwid = qadeema.muzawwid;
                mudkhal.muharrir = qadeema.muharrir;
                mudkhal.akhir_tabdeel = qadeema.akhir_tabdeel;
                // A translator's own reclassification survives re-extraction.
                // The extractor's guess must not overwrite a human's decision,
                // and the confidence is what tells them apart: a hand
                // reclassification is recorded at full confidence.
                if qadeema.thiqat_tasnif >= 100 {
                    mudkhal.tasnif = qadeema.tasnif;
                    mudkhal.thiqat_tasnif = qadeema.thiqat_tasnif;
                }

                // The whole review record moves across, history included: a
                // contributor's transitions are their work as much as the
                // translation is, and a migration that kept the text and
                // dropped who approved it would lose the only evidence that
                // anybody did.
                mudkhal.muraja = qadeema.muraja;
                if mudkhal.hadaf.is_some() {
                    if taghayyar {
                        // The source text moved under a surviving identity, so
                        // the old Arabic may or may not still be right. Flagged
                        // rather than cleared — see this function's doc.
                        mudkhal.muraja.tlub_muraja(None, lahza, Some(
                            "the source text changed in this build".to_owned(),
                        ));
                        taqreer.mughayyara = taqreer.mughayyara.saturating_add(1);
                    } else {
                        taqreer.muhajjara = taqreer.muhajjara.saturating_add(1);
                    }
                }
            }
            None => taqreer.jadeeda = taqreer.jadeeda.saturating_add(1),
        }
        hali.push(mudkhal);
    }

    // Whatever is left in `sabiq` is gone from the build. Their translations are
    // kept, out of the active table but not deleted.
    //
    // Orphans are taken from what actually survived the walk rather than from
    // `farq.mahdhufa`, and the difference matters: the diff was computed from
    // two tables and this was computed from the project's real rows, so a
    // project holding a row the diff never saw — one hand-added, one from an
    // older schema — is still carried across. The diff supplies the *count* the
    // report quotes, and the walk supplies the rows.
    let yatima: Vec<MudkhalNass> = sabiq.into_values().collect();
    taqreer.yatima = yatima.len();
    taqreer.mahdhufa_fi_alfarq = farq.mahdhufa.len();

    (hali, yatima, taqreer)
}
