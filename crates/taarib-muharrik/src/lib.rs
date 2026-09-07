//! # محرّك تعريب — what the game is, and what can be done to it
//!
//! For every discovered game: determine exactly what it was built with, and
//! then state exactly what Taarib can and cannot do to it, in language a player
//! understands. This report drives every downstream dispatch decision in the
//! product — which adapter loads, which tier applies, which framework gets
//! installed — and it is also the thing shown to a user before they commit to
//! anything.
//!
//! Built in **Phase 5**, on top of Phase 4's discovery.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `dalail` | evidence collection from four independent sources, each producing evidence rather than a conclusion |
//! | `tahdid` | resolution: evidence weighted and combined into a `Muharrik` with a confidence value, the evidence retained alongside it |
//! | `imkaniyat` | the capability report — a product artifact, written for a player first |
//! | `bitaqa` | the per-game record persisted with the probe's own version stamp |
//!
//! ## Four sources, none of them trusted alone
//!
//! **Directory shape** — `*_Data/`, `Engine/Binaries/`, `renpy/`, `www/js/`,
//! `data.win`, `resources/app.asar`, `Game.rgss3a`, `*.pck`, `*.pak`, `*.utoc`.
//! Cheap, and wrong often enough to matter: a launcher that bundles a second
//! engine, a game shipping a stray directory from its toolchain.
//!
//! **Binary signatures** — PE, ELF and Mach-O parsed for imported modules
//! (`UnityPlayer`, `GameAssembly`, `mono-2.0-bdwgc`, `d3d11`, `d3d12`,
//! `vulkan-1`, `opengl32`), section names, embedded version strings, and
//! Unreal's `FEngineVersion` structure.
//!
//! **Embedded metadata** — Unity's `globalgamemanagers` / `data.unity3d`
//! version header, IL2CPP's `global-metadata.dat` header version, Godot's PCK
//! magic and version, Ren'Py's version tuple, NW.js and Electron versions from
//! their resources, GameMaker's `GEN8` chunk.
//!
//! **Asset container headers** — the `UnityFS` signature and its compression
//! flags, the `.pak` footer magic and version, the `.utoc` header, PCK v1
//! against v2, the `.rgss3a` key.
//!
//! `tahdid` combines them. Conflicting evidence is *reported*, not averaged,
//! because an average of two contradictory readings is a confident wrong
//! answer. An unknown engine is a first-class result: it resolves to tier 3
//! with an honest explanation, never to a guess that sends an adapter at
//! something it cannot handle.
//!
//! ## The capability report is a product artifact
//!
//! `imkaniyat` is not a debug dump. For each game it states the engine family
//! and version, the scripting backend, the UI frameworks detected, the graphics
//! APIs present, the process architecture, and whether a compatibility layer is
//! involved. Then it states the text systems actually reachable — TextMeshPro,
//! Unity UI Text, NGUI, `FairyGUI`, world-space `TextMesh`, Slate, Godot's
//! `Label` and `RichTextLabel`, RPG Maker's `Window_Base`, Ren'Py `Text`,
//! GameMaker's `draw_text`, DOM text — the tier that applies and why, the
//! expected quality in plain Arabic, and the limitations the user will actually
//! meet, named specifically rather than hedged. Text baked into images will not
//! be translated, and the report says that sentence, in Arabic, before the user
//! installs anything.
//!
//! ## Hard constraints
//!
//! - Probing is read-only, bounded in time, and never loads a game module into
//!   Taarib's own process. Files are memory-mapped and parsed; nothing is
//!   executed, and no game code runs at any point.
//! - Every conclusion carries its evidence, verbatim, into the diagnostics
//!   bundle. A detection that cannot be explained cannot be improved.
//! - `bitaqa` records the probe version. When a Taarib update improves
//!   detection, affected games re-probe automatically instead of serving a
//!   stale conclusion forever.
//! - The report is written for a player first and an engineer second. The
//!   technical detail is present, and it is secondary.

pub mod bitaqa;
pub mod dalail;
pub mod fahs;
pub mod imkaniyat;
pub mod khata;
pub mod tahdid;

use std::path::PathBuf;
use std::time::Instant;

use taarib_kashf::fahs::SimatLuba;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muharrik::{Muharrik, TaqreerImkaniyat};
use taarib_usus::khata::Natija;

pub use crate::bitaqa::{BitaqatMuharrik, SababFahs, SijillBitaqat};
pub use crate::dalail::kul;
pub use crate::fahs::{Fahis, HasilatFahs, JamiHasilat, SiyaqFahs};
pub use crate::imkaniyat::ISDAR_FAHS;
pub use crate::khata::KhataMuharrik;

/// The probe.
///
/// Owns the detectors and runs them. Like the scanner in `taarib-kashf`, it is
/// not a service: it holds no connection, no cache and no thread. Build one,
/// probe, drop it.
pub struct Mifhas {
    fuhus: Vec<Box<dyn Fahis>>,
}

impl std::fmt::Debug for Mifhas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let asmaa: Vec<&'static str> = self.fuhus.iter().map(|fahis| fahis.ism()).collect();
        f.debug_struct("Mifhas").field("fuhus", &asmaa).finish()
    }
}

impl Default for Mifhas {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl Mifhas {
    /// A probe over every detector.
    #[must_use]
    pub fn jadeed() -> Self {
        Self { fuhus: kul() }
    }

    /// A probe over a chosen subset, for re-examining one aspect of a game
    /// without re-reading everything.
    #[must_use]
    pub fn min_fuhus(fuhus: Vec<Box<dyn Fahis>>) -> Self {
        Self { fuhus }
    }

    /// The detectors this probe will run.
    #[must_use]
    pub fn fuhus(&self) -> &[Box<dyn Fahis>] {
        &self.fuhus
    }

    /// Collects every detector's observations, in two passes.
    ///
    /// The two passes exist for one concrete failure. Several launchers name no
    /// executable, and for an Unreal game the real binary is
    /// `<Project>/Binaries/Win64/<Project>-Win64-Shipping.exe` — three levels
    /// down, behind a root-level shim. Only the structural detector goes looking
    /// for it, because only it knows the naming rule that proves which file it
    /// is. Run in one pass, the binary detector would have nothing to read for
    /// exactly the games whose engine version is hardest to establish any other
    /// way.
    ///
    /// So: pass one runs every detector against the context as given. If any of
    /// them resolved an executable the caller did not know about, pass two
    /// re-runs the detectors that read executables, with that path in hand. The
    /// second pass is skipped when nothing better was found, which is the
    /// common case.
    ///
    /// The condition is "a different executable", not "no executable". A caller
    /// that names one can still name the wrong one: a real game shipped a
    /// 195 KB bootstrap shim at its root beside an 82.8 MB binary three levels
    /// down, and discovery chose the shim. The structural detector resolved the
    /// real binary anyway, and gating on `is_none` threw that away — so the
    /// version detector read the shim, found nothing in it, and the game came
    /// back with no engine version at all while the exact tag sat in a file on
    /// disk. Being handed a path is not evidence that the path is right.
    ///
    /// Re-running is safe by construction: detectors hold no state and observe
    /// rather than conclude, so a second look adds evidence and cannot retract
    /// any. Duplicate observations of the same fact do not inflate confidence —
    /// [`tahdid::thiqa`] corroborates per detector, not per observation — so the
    /// second pass replaces the first result for those detectors rather than
    /// appending to it.
    ///
    /// # Errors
    ///
    /// Only when the game directory itself cannot be examined. A detector that
    /// fails is dropped with a warning and the probe continues, because a
    /// capability report built from nine detectors is worth far more to a user
    /// than no report at all.
    pub fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<JamiHasilat> {
        let bidaya = Instant::now();

        if !siyaq.jidhr.exists() {
            return Err(KhataMuharrik::JidhrMafqud {
                jidhr: siyaq.jidhr.to_path_buf(),
            }
            .into());
        }

        let mut jami = JamiHasilat::default();
        for fahis in &self.fuhus {
            match fahis.ifhas(siyaq) {
                Ok(hasila) if hasila.wajad() => jami.hasilat.push((fahis.ism(), hasila)),
                Ok(_) => {},
                Err(khata) => {
                    tracing::warn!(
                        fahis = fahis.ism(),
                        ramz = %khata.ramz,
                        "a detector failed; the probe continues without it"
                    );
                },
            }
        }

        if let Some(tanfidhi) = tanfidhi_muktashaf(&jami)
            && siyaq.tanfidhi != Some(tanfidhi.as_path())
        {
            let thani = SiyaqFahs {
                tanfidhi: Some(&tanfidhi),
                ..siyaq.clone()
            };
            for fahis in &self.fuhus {
                let Ok(hasila) = fahis.ifhas(&thani) else {
                    continue;
                };
                if !hasila.wajad() {
                    continue;
                }
                match jami.hasilat.iter_mut().find(|(ism, _)| *ism == fahis.ism()) {
                    Some((_, sabiqa)) => *sabiqa = hasila,
                    None => jami.hasilat.push((fahis.ism(), hasila)),
                }
            }
        }

        jami.muddat = bidaya.elapsed();
        Ok(jami)
    }

    /// Probes a game and produces the report a player reads.
    ///
    /// The whole phase in one call: gather evidence, resolve it into one engine,
    /// and turn that into a capability report accounting for whatever the game's
    /// launcher said about anti-cheat and multiplayer.
    ///
    /// `waqt` is supplied rather than read from a clock, so that this crate has
    /// no clock in it. A probe whose answer depends on when it ran is a probe
    /// whose answer cannot be reproduced from a diagnostics bundle.
    ///
    /// # Errors
    ///
    /// As [`Mifhas::ifhas`].
    pub fn taqreer(
        &self,
        siyaq: &SiyaqFahs<'_>,
        simat: &[SimatLuba],
        waqt: String,
    ) -> Natija<TaqreerImkaniyat> {
        let jami = self.ifhas(siyaq)?;
        let muharrik = tahdid::hall(&jami);
        Ok(imkaniyat::taqreer(muharrik, simat, waqt))
    }

    /// Probes a game and resolves an engine, without building the report.
    ///
    /// For the callers that dispatch on the engine rather than display it — the
    /// installer choosing a framework, the extractor choosing a container
    /// reader.
    ///
    /// # Errors
    ///
    /// As [`Mifhas::ifhas`].
    pub fn muharrik(&self, siyaq: &SiyaqFahs<'_>) -> Natija<Muharrik> {
        Ok(tahdid::hall(&self.ifhas(siyaq)?))
    }
}

/// The executable a detector resolved that the caller did not know about.
///
/// Takes the first one offered rather than voting, because a detector only sets
/// this field when an engine's own naming rule *proves* which file it is —
/// `Foo_Data` implies `Foo`, a `.app` bundle implies its `Contents/MacOS` binary
/// — and a proof does not need a second opinion. A detector that merely guessed
/// would be a detector violating the observe-never-conclude rule.
fn tanfidhi_muktashaf(jami: &JamiHasilat) -> Option<PathBuf> {
    jami.hasilat
        .iter()
        .find_map(|(_, hasila)| hasila.tanfidhi.clone())
}

/// Builds a probe record from a finished report.
///
/// The bridge between this phase and the store: a report becomes a row, stamped
/// with the probe version that produced it, so that a later Taarib whose
/// detectors improved can find it and look again.
#[must_use]
pub fn bitaqa_min_taqreer(
    luba: LubaId,
    taqreer: TaqreerImkaniyat,
    siyaq: &SiyaqFahs<'_>,
) -> BitaqatMuharrik {
    let basma = bitaqa::basmat_jidhr(siyaq.jidhr, siyaq.tanfidhi);
    BitaqatMuharrik::jadeeda(luba, taqreer, basma)
}
