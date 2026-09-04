//! What a run reports about itself, whether it finished, stopped or was
//! cancelled.
//!
//! Every type here is serializable and holds no borrow, because the caller
//! showing this to a user is on the other side of a process boundary from the
//! run. The failure itself is *not* here — it is
//! [`crate::natija::NatijatMashwar::khata`] — because a `KhataTilqai` carries
//! an `std::io::Error` and a report has to cross a wire.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use taarib_mustalahat::muharrik::TaqreerImkaniyat;

use crate::taqaddum::MarhalaTilqai;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalatMashwar {
    /// Every stage the options asked for closed.
    Tammat,
    /// The user cancelled.
    Mulgha,
    /// Nothing could be read statically and runtime capture is the remedy.
    ///
    /// A first-class outcome, not a failure: the next step is the user's, it is
    /// a step that works, and the report says so in those words.
    TahtajIltiqat,
    /// A stage refused and the run stopped.
    Tawaqqafat,
}

impl HalatMashwar {
    /// The stable identifier, for logs and structured context.
    #[must_use]
    pub const fn ramz(self) -> &'static str {
        match self {
            Self::Tammat => "tammat",
            Self::Mulgha => "mulgha",
            Self::TahtajIltiqat => "tahtaj_iltiqat",
            Self::Tawaqqafat => "tawaqqafat",
        }
    }

    /// The sentence a user reads, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Tammat => "اكتملت الجولة",
            Self::Mulgha => "أُلغيت الجولة، ولم تُمَسّ اللعبة",
            Self::TahtajIltiqat => "تحتاج هذه اللعبة إلى التقاط أثناء التشغيل",
            Self::Tawaqqafat => "توقّفت الجولة",
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Tammat => "The run finished",
            Self::Mulgha => "The run was cancelled and the game was left untouched",
            Self::TahtajIltiqat => "This game needs a runtime capture pass",
            Self::Tawaqqafat => "The run stopped",
        }
    }
}

/// One stage, as the report shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaqreerMarhala {
    /// Which stage.
    pub marhala: MarhalaTilqai,
    /// What it accounted for.
    pub munjaz: u64,
    /// What it set out to account for, when that was knowable.
    pub majmu: Option<u64>,
    /// How long it took, in milliseconds. Zero for a stage that was skipped
    /// because the journal already held it.
    pub milli: u64,
    /// Whether this stage was skipped because a previous run closed it.
    pub mustanafa: bool,
}

/// What extraction found and what it refused.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IhsaIstikhraj {
    /// Distinct strings in the table.
    pub adad: u64,
    /// How many of them a player is believed to see.
    pub adad_zahir: u64,
    /// Containers read.
    pub maqrua: u64,
    /// Containers refused.
    pub marfuda: u64,
    /// Refusals that lost strings.
    pub khasara: u64,
    /// Whether running the game with capture on would recover anything.
    pub yanfa_iltiqat: bool,
    /// Whether a capture session was merged into this table.
    pub multaqat: bool,
    /// The refusal report's own summary, one line per group.
    pub satrat: Vec<String>,
}

/// What the translation stage did, and what it cost.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IhsaTarjama {
    /// Strings eligible for machine translation when the stage opened.
    pub muahhala: u64,
    /// Strings this run got an answer for, success or failure.
    ///
    /// Distinct from `mutabbaqa`, which counts what the *fold* wrote onto the
    /// table — and on a resumed run the fold writes the answers a previous run
    /// paid for as well. This is the number the progress bar divides by
    /// `muahhala`, and mixing the two is how a resumed run reports 16 of 14.
    pub ujiba: u64,
    /// Strings the fold wrote a translation onto, across every run.
    pub mutabbaqa: u64,
    /// Strings that failed, with their reasons recorded on the entry.
    pub fashila: u64,
    /// Failures the placeholder guard caused, which are the ones worth
    /// separating: they are the pipeline refusing to ship a string that would
    /// have crashed the game's formatter.
    pub marfuda_bil_hima: u64,
    /// Strings skipped because a previous run had already answered them.
    pub min_sijill: u64,
    /// Strings skipped because they already carried a translation.
    pub mutarjama_musbaqan: u64,
    /// Settled spend, in nano-dollars, across every run of this project.
    pub munfaq: u64,
    /// The ceiling this run honoured, when one was set.
    pub saqf: Option<u64>,
    /// Whether the run stopped at the ceiling rather than walking its list.
    pub balagha_alsaqf: bool,
    /// What stopped the run early, when something did.
    pub tawaqquf: Option<String>,
    /// The provider's name, recorded as provenance on every string it produced.
    pub muzawwid: String,
    /// The model identifier.
    pub namudhaj: String,
}

/// What the compiler produced.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IhsaHuzma {
    /// Where the sealed package is.
    pub masar: PathBuf,
    /// Its size in bytes.
    pub hajm: u64,
    /// Strings it holds.
    pub nusus: u64,
    /// Precomputed layouts it holds.
    pub takhtitat: u64,
    /// Atlas pages it holds.
    pub safahat: u64,
    /// Distinct glyphs the atlas was built from.
    pub ashkal: u64,
    /// The sizes it can draw at, in quarter-pixels.
    pub ahjam_rubi: Vec<u32>,
    /// The public half of the sealing key, hexadecimal.
    pub miftah: String,
    /// Whether that key is the committed development key.
    pub miftah_tatwir: bool,
    /// The container's content hash.
    pub basma: String,
}

/// What the install did, and how it can be undone.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IhsaTathbeet {
    /// The backup root, which is what an uninstall is pointed at.
    pub jidhr_nusakh: PathBuf,
    /// How many content files were placed.
    pub adad_muhtawa: u64,
    /// The post-write verification, in its own words.
    pub tahaqquq: String,
    /// Whether the install was reversed before this run returned, which is what
    /// a cancellation arriving after the point of no return produces.
    pub rujia: bool,
    /// The restore's own report, when one ran.
    pub istiada: Vec<String>,
}

/// Everything a run has to say about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaqreerMashwar {
    /// How it ended.
    pub hala: HalatMashwar,
    /// The stage it ended in.
    pub marhala: MarhalaTilqai,
    /// Every stage, in order.
    pub marahil: Vec<TaqreerMarhala>,
    /// What the probe concluded.
    pub imkaniyat: Option<TaqreerImkaniyat>,
    /// What extraction found.
    pub istikhraj: Option<IhsaIstikhraj>,
    /// What translation did.
    pub tarjama: Option<IhsaTarjama>,
    /// What the compiler produced.
    pub huzma: Option<IhsaHuzma>,
    /// What the install did.
    pub tathbeet: Option<IhsaTathbeet>,
    /// Why it ended the way it did, in English.
    pub sabab: Option<String>,
    /// What the user can do next, in English.
    pub ilaj: Option<String>,
    /// Lines of a previous journal that would not parse.
    pub sijill_talif: usize,
    /// How long the whole run took, in milliseconds.
    pub milli: u64,
}

impl TaqreerMashwar {
    /// A report for a run that has not done anything yet.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            hala: HalatMashwar::Tawaqqafat,
            marhala: MarhalaTilqai::Fahs,
            marahil: Vec::new(),
            imkaniyat: None,
            istikhraj: None,
            tarjama: None,
            huzma: None,
            tathbeet: None,
            sabab: None,
            ilaj: None,
            sijill_talif: 0,
            milli: 0,
        }
    }

    /// The report as lines a log or a terminal can print.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut satr = vec![format!(
            "{} at stage {} ({}), in {} ms",
            self.hala.wasf_injilizi(),
            self.marhala.raqm(),
            self.marhala.wasf_injilizi(),
            self.milli
        )];
        for marhala in &self.marahil {
            let majmu = marhala
                .majmu
                .map_or_else(|| "-".to_owned(), |majmu| majmu.to_string());
            satr.push(format!(
                "  {} {:<22} {:>7}/{:<7} {:>7} ms{}",
                marhala.marhala.raqm(),
                marhala.marhala.wasf_injilizi(),
                marhala.munjaz,
                majmu,
                marhala.milli,
                if marhala.mustanafa {
                    "  (resumed from the journal)"
                } else {
                    ""
                }
            ));
        }
        if let Some(istikhraj) = &self.istikhraj {
            satr.push(format!(
                "  extraction: {} string(s), {} a player sees, {} container(s) read, {} refused",
                istikhraj.adad, istikhraj.adad_zahir, istikhraj.maqrua, istikhraj.marfuda
            ));
        }
        if let Some(tarjama) = &self.tarjama {
            satr.push(format!(
                "  translation: {}/{} answered this run, {} applied to the table in total, \
                 {} failed ({} refused by the placeholder guard), {} nano-$ spent{}",
                tarjama.ujiba,
                tarjama.muahhala,
                tarjama.mutabbaqa,
                tarjama.fashila,
                tarjama.marfuda_bil_hima,
                tarjama.munfaq,
                tarjama
                    .saqf
                    .map_or_else(String::new, |saqf| format!(" of a {saqf} ceiling"))
            ));
        }
        if let Some(huzma) = &self.huzma {
            satr.push(format!(
                "  package: {} byte(s), {} string(s), {} layout(s), {} atlas page(s), {} glyph(s)",
                huzma.hajm, huzma.nusus, huzma.takhtitat, huzma.safahat, huzma.ashkal
            ));
        }
        if let Some(tathbeet) = &self.tathbeet {
            satr.push(format!(
                "  install: {} file(s) placed, backups at {}{}",
                tathbeet.adad_muhtawa,
                tathbeet.jidhr_nusakh.display(),
                if tathbeet.rujia { ", and reversed" } else { "" }
            ));
        }
        if let Some(sabab) = &self.sabab {
            satr.push(format!("  because: {sabab}"));
        }
        if let Some(ilaj) = &self.ilaj {
            satr.push(format!("  next   : {ilaj}"));
        }
        if self.sijill_talif > 0 {
            satr.push(format!(
                "  {} line(s) of a previous journal would not parse and were skipped",
                self.sijill_talif
            ));
        }
        satr
    }
}
