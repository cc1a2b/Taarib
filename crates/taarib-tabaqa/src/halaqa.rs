//! الحلقة — the per-frame loop: capture, recognize, cache, shape, draw.
//!
//! Every module in this crate did one step of tier 3 and stopped. The backends
//! hooked a present and drew a batch; [`crate::iltiqat_shasha`] read a rectangle
//! back; [`crate::qira`] read text out of it; [`crate::qissa`] tracked lines and
//! asked a translator; [`crate::talqeem`] shaped Arabic into quads. Nothing
//! called the next step from the last, so the overlay opened with a game,
//! attached to its picture, and stayed empty. This module is the calls.
//!
//! ## What runs on the frame, and what does not
//!
//! [`Halaqa::itar`] is called once per present, from the hook, and does four
//! things in order: pick up whatever the worker has finished, read the panel's
//! chords, capture the regions whose clock says it is time, and draw the most
//! recent completed translation over the frame. Recognition and translation
//! never run here; [`crate::qissa::KhaytQissa`] does both on its own thread and
//! this loop only posts captures to it and reads its snapshot back. The capture
//! is the one expensive thing on the frame — a GPU read-back — and its cost is
//! measured into the refresh governor, which slows the capture rate down rather
//! than dropping frames when it is more than the budget can bear.
//!
//! ## The region a game never configured
//!
//! Regions are drawn by a player, and a game with none is the ordinary case for
//! a tier that exists for games nobody has configured. So when the set holds
//! no region a timer polls, the loop reads the **whole surface** as one implied
//! region, once a second, on every interval — the change gate's hash is too
//! coarse to see a line change inside a whole screen, and the tracker is what
//! keeps a line that has not changed from being translated twice. That is the
//! cadence that makes a menu translate itself without anybody drawing a box,
//! and it is slow enough that the read-back stays a small fraction of the
//! frame budget.
//!
//! ## A blank overlay says why
//!
//! The panel carries every counter and every reason, and a caption at the top
//! of the screen carries the one sentence that matters while the panel is
//! closed: the recognizer is gone, no translator is attached, or nothing has
//! been read yet. The caption disappears the moment Arabic is being drawn.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use taarib_mustalahat::luba::LubaId;
use taarib_saff::{MawridKhatt, SilsilatKhutut};
use taarib_usus::idadat::{Idadat, MakhzanIdadat, MuharrikQira};
use taarib_usus::khata::Tafsir as _;
use taarib_usus::masarat::Masarat;

use crate::dakhl::RasidDakhl;
use crate::iltiqat_shasha::{MuqayyidMuadal, SuraMultaqata};
use crate::khata::KhataTabaqa;
use crate::khazina::{DhakiraMalaf, DhakiraMurakkaba, DhakiraRuqaa, masar_khazina};
use crate::lawhat_tahakkum::{
    Amr, AnsurLawha, AtharAmr, DawrAnsur, HalatTarjamaLawha, Ikhtisarat, LawhatTahakkum,
};
use crate::manatiq::{MajmuatManatiq, MuarrifMintaqa, QaidatTarjama};
use crate::mutarjim::{DhakiraTabaqa, MutarjimTabaqa};
use crate::mutarjim_mahalli::{HalatMutarjim, MutarjimMahalli};
use crate::qira::{IdadatQira, IkhtiyarQari};
use crate::qissa::{
    HalatDaf, HalatKhayt, IhsaatQissa, KhaytQissa, MintaqaMustahiqqa, Munassiq, Qissa,
};
use crate::sijill_qira::{SijillMushtarak, SijillQira};
use crate::talqeem::{KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
use crate::tatabbu::IhsaatTatabbu;
use crate::wajiha::{HalatTabaqa, LawhatRasm, MustatilNisbi, Tabaqa, WasfSath};
use crate::watira::MunazzimWatira;

/// The identity of the implied whole-surface region.
///
/// Zero, which a region set never hands out — its counter starts at one — so
/// the tracker, the history and the picture gate can key on it beside real
/// regions without a collision.
pub const MINTAQAT_SHASHA: MuarrifMintaqa = MuarrifMintaqa::min_raqm(0);

/// The implied region's name, as the history and the panel show it.
pub const ISM_SHASHA: &str = "الشاشة كلّها";

/// The directory, under the data root's components directory, that holds the
/// portable recognizer's models.
pub const DALIL_NAMADHIJ: &str = "ocrs";

/// The source language read when nothing names one.
///
/// English. The portable models read the Latin alphabet and nothing else, the
/// platform engines are asked for this tag and fall back to the user's own
/// packs, and a game with no adapter and no patch metadata is, overwhelmingly,
/// an English one.
pub const LUGHA_IFTIRADIYA: &str = "en";

/// How the loop paces itself and what it draws with.
#[derive(Debug, Clone)]
pub struct KhiyaratHalaqa {
    /// The implied whole-surface region's interval, in milliseconds.
    ///
    /// One second. A full-surface read-back is the most expensive capture the
    /// loop makes, and a menu that takes a second to translate itself reads as
    /// prompt; the governor slows this further when the frame cannot afford it.
    pub fasil_shasha_milli: u32,
    /// How many regions may be captured on one frame.
    ///
    /// Two. Several regions falling due on the same frame would otherwise put
    /// several read-backs on it at once, and the third can wait a frame.
    pub aqsa_iltiqatat_lil_itar: usize,
    /// How long the "reading, nothing yet" caption stays after the first
    /// frame, in microseconds, when nothing is wrong and nothing is drawn.
    pub muhlat_tanbih_mikro: u64,
    /// How the feeder sizes, colours and budgets what it draws.
    pub talqeem: KhiyaratTalqeem,
    /// The refresh governor to start from.
    pub watira: MunazzimWatira,
    /// The source language, for the cache file and the recognizer.
    pub lugha: String,
}

impl KhiyaratHalaqa {
    /// The defaults.
    #[must_use]
    pub fn iftiradiya() -> Self {
        Self {
            fasil_shasha_milli: 1_000,
            aqsa_iltiqatat_lil_itar: 2,
            muhlat_tanbih_mikro: 12_000_000,
            talqeem: KhiyaratTalqeem::iftiradiya(),
            watira: MunazzimWatira::iftiradi(),
            lugha: LUGHA_IFTIRADIYA.to_owned(),
        }
    }

}

impl Default for KhiyaratHalaqa {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

/// What the capture side of the loop did, which nothing counted before.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IhsaatHalaqa {
    /// Frames the loop ran on.
    pub itarat: u64,
    /// Captures read back from the backend.
    pub iltiqatat: u64,
    /// Captures the backend or the geometry check refused.
    pub iltiqatat_fashila: u64,
    /// Captures the worker queued.
    pub mursala: u64,
    /// Captures dropped because the worker was behind.
    pub matruka: u64,
    /// Captures refused at the door because recognition had stopped.
    pub marfuda: u64,
    /// Frames on which translated lines were drawn.
    pub itarat_bi_arabi: u64,
    /// Lines in the most recent batch.
    pub sutur_akhira: u32,
    /// Frames the feeder or the backend refused to draw.
    pub akhta_rasm: u64,
    /// The most recent capture's cost on the frame, in microseconds.
    pub iltiqat_mikro: u32,
    /// The most recent draw's cost, in microseconds.
    pub rasm_mikro: u32,
}

impl IhsaatHalaqa {
    /// The sentence the panel and the log carry, in English.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut wasf = format!(
            "{} frame(s); {} capture(s) read back, {} refused; {} queued, {} dropped behind the \
             worker, {} refused at the door; Arabic drawn on {} frame(s), {} line(s) in the last; \
             last capture {} µs, last draw {} µs",
            self.itarat,
            self.iltiqatat,
            self.iltiqatat_fashila,
            self.mursala,
            self.matruka,
            self.marfuda,
            self.itarat_bi_arabi,
            self.sutur_akhira,
            self.iltiqat_mikro,
            self.rasm_mikro
        );
        if self.akhta_rasm > 0 {
            let _ = write!(wasf, "; {} frame(s) the draw refused", self.akhta_rasm);
        }
        wasf
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let mut wasf = format!(
            "{} إطارًا؛ {} التقاطة قُرئت من العارض و{} رُفضت؛ {} أُرسلت للعامل و{} أُسقطت لتأخّره \
             و{} رُفضت عند الباب؛ رُسمت العربية في {} إطارًا، {} سطرًا في الأخير؛ آخر التقاطة {} \
             ميكروثانية وآخر رسم {} ميكروثانية",
            self.itarat,
            self.iltiqatat,
            self.iltiqatat_fashila,
            self.mursala,
            self.matruka,
            self.marfuda,
            self.itarat_bi_arabi,
            self.sutur_akhira,
            self.iltiqat_mikro,
            self.rasm_mikro
        );
        if self.akhta_rasm > 0 {
            let _ = write!(wasf, "؛ ورفض الرسم {} إطارًا", self.akhta_rasm);
        }
        wasf
    }
}

/// Everything the bootstrap resolved for the loop, handed over in one value.
///
/// Resolved outside rather than inside so the loop can be built in a harness
/// from a software backend, a stub recognizer and a scratch cache, and so that
/// the bootstrap — which has the patch, the settings and the data root — is the
/// one place that decides where each of these comes from.
pub struct MakunatHalaqa {
    /// The font chain the Arabic is shaped with.
    pub khutut: SilsilatKhutut,
    /// The recognizer, or why there is none.
    pub qari: Result<IkhtiyarQari, KhataTabaqa>,
    /// The session, with its memory, translator and history already attached.
    pub qissa: Qissa,
    /// What the panel says about the translator.
    pub hala_mutarjim: HalatMutarjim,
    /// The on-disk cache, for its counters; the session holds it as a layer.
    pub khazina: Option<Arc<DhakiraMalaf>>,
    /// The history, shared with the session, for the panel's page.
    pub sijill: Option<SijillMushtarak>,
    /// The regions the player drew, or an empty set.
    pub manatiq: MajmuatManatiq,
    /// The chord table.
    pub ikhtisarat: Ikhtisarat,
    /// The game's display name, for the panel's title.
    pub ism_luba: String,
    /// What was decided while resolving, for the log.
    pub athar: Vec<String>,
}

impl std::fmt::Debug for MakunatHalaqa {
    fn fmt(&self, matbaa: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        matbaa
            .debug_struct("MakunatHalaqa")
            .field("khutut", &self.khutut.adad())
            .field("qari", &self.qari.as_ref().map(IkhtiyarQari::ism))
            .field("hala_mutarjim", &self.hala_mutarjim)
            .field("manatiq", &self.manatiq.adad())
            .field("ism_luba", &self.ism_luba)
            .finish_non_exhaustive()
    }
}

/// The loop: one overlay, one feeder, one worker, one panel.
pub struct Halaqa {
    tabaqa: Tabaqa,
    mulaqqim: Mulaqqim,
    munassiq: Munassiq,
    khayt: Option<KhaytQissa>,
    sabab_la_qari: Option<(String, String)>,
    manatiq: MajmuatManatiq,
    muaqqit_shasha: MuqayyidMuadal,
    lawha: LawhatTahakkum,
    rasid: RasidDakhl,
    hala_mutarjim: HalatMutarjim,
    khazina: Option<Arc<DhakiraMalaf>>,
    sijill: Option<SijillMushtarak>,
    khiyarat: KhiyaratHalaqa,
    ihsaat: IhsaatHalaqa,
    ihsaat_qissa: IhsaatQissa,
    ihsaat_tatabbu: IhsaatTatabbu,
    hala_khayt: HalatKhayt,
    akhir_rafd: Option<String>,
    sath_akhir: Option<WasfSath>,
    akhir_rasm_mikro: u32,
    akhir_kull_mikro: u32,
    bidayat_mikro: Option<u64>,
    akhir_khata: Option<String>,
    athar: Vec<String>,
}

impl std::fmt::Debug for Halaqa {
    fn fmt(&self, matbaa: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        matbaa
            .debug_struct("Halaqa")
            .field("tabaqa", &self.tabaqa.hala())
            .field("khayt", &self.khayt.is_some())
            .field("manatiq", &self.manatiq.adad())
            .field("hala_mutarjim", &self.hala_mutarjim)
            .field("ihsaat", &self.ihsaat)
            .finish_non_exhaustive()
    }
}

impl Halaqa {
    /// Builds the loop over a started overlay and starts the worker.
    ///
    /// A recognizer that could not be built does not stop the loop: the overlay
    /// still draws the panel and the caption that says why nothing else is
    /// drawn, which is the one thing a blank overlay owes. A worker thread that
    /// cannot be spawned is the same case with a different sentence.
    ///
    /// # Errors
    ///
    /// Whatever [`Mulaqqim::jadeed`] refuses — an atlas page the packer cannot
    /// use — which leaves nothing to draw with at all.
    pub fn ibda(
        tabaqa: Tabaqa,
        makunat: MakunatHalaqa,
        khiyarat: KhiyaratHalaqa,
    ) -> Result<Self, KhataTabaqa> {
        let mut athar = makunat.athar;
        let mulaqqim = match Mulaqqim::jadeed(makunat.khutut, khiyarat.talqeem) {
            Ok(mulaqqim) => mulaqqim,
            Err(khata) => {
                // Torn down rather than dropped: the backend behind it holds
                // device resources, and `ahmil` is documented for exactly this.
                let mut tabaqa = tabaqa;
                let _ = tabaqa.aghliq();
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "runtime glyph atlas",
                    sabab: khata.injilizi,
                });
            },
        };

        let manshura = makunat.qissa.manshura();
        let (khayt, sabab_la_qari) = match makunat.qari {
            Ok(qari) => {
                for satr in qari.athar() {
                    athar.push(format!("recognizer: {satr}"));
                }
                match KhaytQissa::ibda(makunat.qissa, qari) {
                    Ok(khayt) => (Some(khayt), None),
                    Err(khata) => {
                        athar.push(format!("the recognition worker would not start: {khata}"));
                        (None, Some((khata.to_string(), khata.arabi())))
                    },
                }
            },
            Err(khata) => {
                athar.push(format!("no recognizer: {khata}"));
                (None, Some((khata.to_string(), khata.arabi())))
            },
        };

        let mut lawha = LawhatTahakkum::jadeeda(makunat.ism_luba);
        lawha.ihmil_ikhtisarat(makunat.ikhtisarat);
        lawha.hadith_manatiq(&makunat.manatiq);
        lawha.hadith_tabaqa(tabaqa.hala(), Some(tabaqa.wajiha()));
        athar.push(format!("translator: {}", makunat.hala_mutarjim.wasf()));
        if let Some(khazina) = makunat.khazina.as_ref() {
            athar.push(format!("cache: {}", khazina.wasf()));
        }
        athar.push(format!(
            "regions: {} defined, {} on a timer{}",
            makunat.manatiq.adad(),
            makunat.manatiq.ala_muaqqit().count(),
            if makunat.manatiq.ala_muaqqit().next().is_none() {
                format!(
                    "; the whole surface is read as one region every {} ms, changed or not",
                    khiyarat.fasil_shasha_milli
                )
            } else {
                String::new()
            }
        ));

        let fasil_shasha = u64::from(khiyarat.fasil_shasha_milli).saturating_mul(1_000);
        Ok(Self {
            tabaqa,
            mulaqqim,
            munassiq: Munassiq::jadeed(manshura, khiyarat.watira.clone()),
            khayt,
            sabab_la_qari,
            manatiq: makunat.manatiq,
            muaqqit_shasha: MuqayyidMuadal::jadeed(fasil_shasha, 0),
            lawha,
            rasid: RasidDakhl::jadeed(),
            hala_mutarjim: makunat.hala_mutarjim,
            khazina: makunat.khazina,
            sijill: makunat.sijill,
            khiyarat,
            ihsaat: IhsaatHalaqa::default(),
            ihsaat_qissa: IhsaatQissa::default(),
            ihsaat_tatabbu: IhsaatTatabbu::default(),
            hala_khayt: HalatKhayt::Salima,
            akhir_rafd: None,
            sath_akhir: None,
            akhir_rasm_mikro: 0,
            akhir_kull_mikro: 0,
            bidayat_mikro: None,
            akhir_khata: None,
            athar,
        })
    }

    /// The overlay under the loop.
    #[must_use]
    pub const fn tabaqa(&self) -> &Tabaqa {
        &self.tabaqa
    }

    /// The overlay, to pause or resume it.
    pub const fn tabaqa_mut(&mut self) -> &mut Tabaqa {
        &mut self.tabaqa
    }

    /// The panel, for a caller reading its state.
    #[must_use]
    pub const fn lawha(&self) -> &LawhatTahakkum {
        &self.lawha
    }

    /// The panel, to drive it from a harness.
    pub const fn lawha_mut(&mut self) -> &mut LawhatTahakkum {
        &mut self.lawha
    }

    /// The capture side's counters.
    #[must_use]
    pub const fn ihsaat(&self) -> &IhsaatHalaqa {
        &self.ihsaat
    }

    /// The worker, when one is running.
    #[must_use]
    pub const fn khayt(&self) -> Option<&KhaytQissa> {
        self.khayt.as_ref()
    }

    /// What the panel says about the translator.
    #[must_use]
    pub const fn hala_mutarjim(&self) -> &HalatMutarjim {
        &self.hala_mutarjim
    }

    /// Why there is no recognizer, when there is none.
    #[must_use]
    pub fn sabab_la_qari(&self) -> Option<&str> {
        self.sabab_la_qari.as_ref().map(|(sabab, _)| sabab.as_str())
    }

    /// The most recent failure the loop absorbed, when there was one.
    #[must_use]
    pub fn akhir_khata(&self) -> Option<&str> {
        self.akhir_khata.as_deref()
    }

    /// What has happened, for the log.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Takes the trace lines written since the last call, for a log that is
    /// appended to as things happen.
    pub fn khudh_athar(&mut self) -> Vec<String> {
        std::mem::take(&mut self.athar)
    }

    /// The lines the last frame drew, in surface pixels.
    #[must_use]
    pub fn sutur_marsuma(&self) -> &[SatrMulaqqam] {
        self.sath_akhir
            .map_or(&[], |sath| self.munassiq.laqta(sath))
    }

    /// The whole account, in English, for the log and the diagnostics bundle.
    #[must_use]
    pub fn khulasa(&self) -> String {
        let mut wasf = String::new();
        let _ = writeln!(wasf, "overlay: {}", self.tabaqa.hala().ism());
        let _ = writeln!(wasf, "budget: {}", self.tabaqa.meezaniya().wasf());
        let _ = writeln!(wasf, "capture: {}", self.ihsaat.wasf());
        let _ = writeln!(wasf, "session: {}", self.ihsaat_qissa.wasf());
        let _ = writeln!(wasf, "tracking: {}", self.ihsaat_tatabbu.wasf());
        let _ = writeln!(wasf, "refresh: {}", self.munassiq.watira().wasf());
        match self.khayt.as_ref() {
            Some(khayt) => {
                let _ = writeln!(wasf, "worker: {}", khayt.wasf());
            },
            None => {
                let _ = writeln!(
                    wasf,
                    "worker: none — {}",
                    self.sabab_la_qari().unwrap_or("no reason recorded")
                );
            },
        }
        let _ = writeln!(wasf, "translator: {}", self.hala_mutarjim.wasf());
        if let Some(khazina) = self.khazina.as_ref() {
            let _ = writeln!(wasf, "cache: {}", khazina.wasf());
        }
        if let Some(rafd) = self.akhir_rafd.as_deref() {
            let _ = writeln!(wasf, "last refused read: {rafd}");
        }
        if let Some(khata) = self.akhir_khata.as_deref() {
            let _ = writeln!(wasf, "last failure: {khata}");
        }
        wasf
    }

    /// Runs one frame.
    ///
    /// `saa` is the hook's monotonic microsecond clock, read here at the frame's
    /// start, after the captures and after the draw: the draw's own cost is
    /// what [`Tabaqa::itar`] holds the frame budget against, and the whole
    /// frame's cost — read-back included — is what the refresh governor sees.
    /// `awamir` are commands a caller already resolved, added to the ones the
    /// chord reader finds.
    ///
    /// # Errors
    ///
    /// A recoverable draw failure, after which the next frame is attempted, or
    /// the terminal [`KhataTabaqa::HalaGhayrMustaada`], after which the overlay
    /// has disabled itself and the caller should stop calling.
    pub fn itar(&mut self, saa: &dyn Fn() -> u64, awamir: &[Amr]) -> Result<(), KhataTabaqa> {
        let bidaya = saa();
        self.ihsaat.itarat = self.ihsaat.itarat.saturating_add(1);
        if self.bidayat_mikro.is_none() {
            self.bidayat_mikro = Some(bidaya);
        }
        if let Some(taghyeer) = self.munassiq.itar(self.akhir_kull_mikro) {
            self.athar.push(taghyeer.satr());
        }

        let Some(sath) = self.tabaqa.sath() else {
            // No validated surface: an empty draw is what validates one, and
            // the next frame draws against it.
            let khaliya = LawhatRasm::khaliya(WasfSath {
                ard: 0,
                irtifa: 0,
                sigha: crate::wajiha::SighatSath::Rgba8,
                sirgb: false,
            });
            let natija = self.tabaqa.itar(&khaliya, self.akhir_rasm_mikro);
            self.akhir_kull_mikro = mikro_bayn(bidaya, saa());
            return natija;
        };
        if self.sath_akhir != Some(sath) {
            self.sath_taghayyar(sath);
        }

        let mut kull_awamir = self
            .rasid
            .iqra(self.lawha.ikhtisarat(), self.lawha.hala().zahira());
        kull_awamir.extend_from_slice(awamir);
        for amr in kull_awamir {
            self.naffidh(amr);
        }

        if self.tabaqa.hala().tarsim() {
            self.iltaqit(bidaya, sath, saa);
        }
        self.iqra_al_amil();
        self.haddith_lawha();

        let sutur: Vec<SatrMulaqqam> = self.munassiq.laqta(sath).to_vec();
        let mut ansur = self.lawha.bina(sath);
        if let Some(tanbih) = self.tanbih(bidaya, sutur.is_empty()) {
            ansur.push(tanbih);
        }

        let qabl_rasm = saa();
        let natija = self.mulaqqim.qaddim(
            &mut self.tabaqa,
            sath,
            &sutur,
            &ansur,
            self.akhir_rasm_mikro,
        );
        let baad = saa();
        self.akhir_rasm_mikro = mikro_bayn(qabl_rasm, baad);
        self.akhir_kull_mikro = mikro_bayn(bidaya, baad);
        self.ihsaat.rasm_mikro = self.akhir_rasm_mikro;

        match natija {
            Ok(_) => {
                let adad = u32::try_from(sutur.len()).unwrap_or(u32::MAX);
                self.ihsaat.sutur_akhira = adad;
                if adad > 0 {
                    if self.ihsaat.itarat_bi_arabi == 0 {
                        self.athar.push(format!(
                            "first Arabic drawn: {adad} line(s) at frame {}",
                            self.ihsaat.itarat
                        ));
                    }
                    self.ihsaat.itarat_bi_arabi = self.ihsaat.itarat_bi_arabi.saturating_add(1);
                }
                Ok(())
            },
            Err(khata) => {
                self.ihsaat.akhta_rasm = self.ihsaat.akhta_rasm.saturating_add(1);
                self.akhir_khata = Some(khata.injilizi.clone());
                if matches!(self.tabaqa.hala(), HalatTabaqa::Muattala) {
                    self.athar
                        .push(format!("the overlay disabled itself: {}", khata.injilizi));
                    return Err(KhataTabaqa::HalaGhayrMustaada {
                        hala: "the overlay",
                        sabab: khata.injilizi,
                    });
                }
                Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay frame",
                    sabab: khata.injilizi,
                })
            },
        }
    }

    /// Tells the backend the surface is about to be resized under it.
    ///
    /// # Errors
    ///
    /// As [`Tabaqa::qabl_taghyeer_hajm`].
    pub fn qabl_taghyeer_hajm(&mut self) -> Result<(), KhataTabaqa> {
        self.sath_akhir = None;
        self.tabaqa.qabl_taghyeer_hajm()
    }

    /// Stops the worker, waits for it, and shuts the overlay down.
    ///
    /// # Errors
    ///
    /// As [`Tabaqa::aghliq`].
    pub fn aghliq(&mut self) -> Result<(), KhataTabaqa> {
        if let Some(mut khayt) = self.khayt.take() {
            khayt.awqif();
        }
        if let Some(sijill) = self.sijill.as_ref() {
            let _ = sijill.aghliq();
        }
        self.tabaqa.aghliq()
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Forgets everything positioned against the previous surface.
    fn sath_taghayyar(&mut self, sath: WasfSath) {
        self.munassiq.sath_taghayyar();
        if let Some(khayt) = self.khayt.as_ref() {
            khayt.sath_taghayyar(sath);
        }
        self.muaqqit_shasha.ansa();
        self.sath_akhir = Some(sath);
        self.athar
            .push(format!("surface {}×{} {}", sath.ard, sath.irtifa, sath.sigha.ism()));
    }

    /// Applies one command to the panel and the overlay.
    fn naffidh(&mut self, amr: Amr) {
        match self.lawha.naffidh(amr) {
            AtharAmr::BaddilTabaqa => {
                if self.tabaqa.hala().tarsim() {
                    self.tabaqa.awqif();
                } else {
                    self.tabaqa.istanif();
                }
            },
            AtharAmr::IqafMuaqqat => self.tabaqa.awqif(),
            AtharAmr::TarjimAlan => {
                self.munassiq.iqra_alan();
                self.muaqqit_shasha.ansa();
                if let Some(khayt) = self.khayt.as_ref() {
                    khayt.iqra_alan();
                }
                self.athar.push("translate-now requested".to_owned());
            },
            AtharAmr::LaShay
            | AtharAmr::Ughliqat
            | AtharAmr::Intaqalat(_)
            | AtharAmr::TaghayyarAtaama(_)
            | AtharAmr::TaghayyarKhat(_) => {},
        }
    }

    /// The regions due this frame: the player's, or the implied whole surface.
    fn mustahiqqa(&mut self, lahza_mikro: u64, sath: WasfSath) -> Vec<MintaqaMustahiqqa> {
        let mut mustahiqqa = self.munassiq.mustahiqqa(lahza_mikro, &self.manatiq, sath);
        if self.manatiq.ala_muaqqit().next().is_some() {
            return mustahiqqa;
        }
        let fasil = u64::from(self.khiyarat.fasil_shasha_milli)
            .saturating_mul(1_000)
            .max(self.munassiq.watira().fasil_mikro());
        self.muaqqit_shasha.ihdud(fasil);
        if self.muaqqit_shasha.hal_yalzam(lahza_mikro)
            && let Some(mustatil) = MustatilNisbi::KAAMIL.fi_bikselat(sath.ard, sath.irtifa)
        {
            mustahiqqa.push(MintaqaMustahiqqa {
                muarrif: MINTAQAT_SHASHA,
                ism: ISM_SHASHA.to_owned(),
                mustatil,
                // Read every interval, changed or not. The change gate is an
                // 8×8 average hash, built for a rectangle drawn around a
                // dialogue box where the text is most of the picture; over a
                // whole screen one cell is a hundred and sixty pixels wide,
                // and a menu moving sixty of them — or one subtitle replaced
                // by another of the same length — does not move a cell across
                // its mean. Measured: forty captures of a moved menu, one
                // recognized. The tracker still gives a line one identity, so
                // what this costs is a recognition per interval and never a
                // second translation.
                qaida: QaidatTarjama::Mustamirra,
            });
        }
        mustahiqqa
    }

    /// Reads back every region that is due and posts it to the worker.
    fn iltaqit(&mut self, lahza_mikro: u64, sath: WasfSath, saa: &dyn Fn() -> u64) {
        if self.khayt.is_none() {
            return;
        }
        let mustahiqqa = self.mustahiqqa(lahza_mikro, sath);
        let saqf = self.khiyarat.aqsa_iltiqatat_lil_itar.max(1);
        for mintaqa in mustahiqqa.into_iter().take(saqf) {
            let qabl = saa();
            let natija = self
                .tabaqa
                .iltaqit_bikselat(mintaqa.mustatil, &mintaqa.ism)
                .and_then(|bayt| {
                    SuraMultaqata::jadeeda(
                        bayt,
                        mintaqa.mustatil.ard,
                        mintaqa.mustatil.irtifa,
                        sath.sigha,
                        mintaqa.mustatil,
                    )
                });
            self.ihsaat.iltiqat_mikro = mikro_bayn(qabl, saa());
            let sura = match natija {
                Ok(sura) => sura,
                Err(khata) => {
                    // Counted here, because it was counted nowhere: a backend
                    // that refuses every read-back is an overlay that is blank
                    // for a reason the worker never hears about.
                    self.ihsaat.iltiqatat_fashila = self.ihsaat.iltiqatat_fashila.saturating_add(1);
                    self.akhir_khata = Some(format!("capture of {}: {khata}", mintaqa.ism));
                    continue;
                },
            };
            self.ihsaat.iltiqatat = self.ihsaat.iltiqatat.saturating_add(1);
            let Some(khayt) = self.khayt.as_ref() else {
                return;
            };
            match khayt.adfa(
                mintaqa.muarrif,
                &mintaqa.ism,
                mintaqa.qaida,
                lahza_mikro,
                sura,
            ) {
                HalatDaf::Qubilat => self.ihsaat.mursala = self.ihsaat.mursala.saturating_add(1),
                HalatDaf::Turikat => self.ihsaat.matruka = self.ihsaat.matruka.saturating_add(1),
                HalatDaf::Rufidat => self.ihsaat.marfuda = self.ihsaat.marfuda.saturating_add(1),
            }
        }
    }

    /// Copies what the worker last said, without waiting for it.
    fn iqra_al_amil(&mut self) {
        let Some(khayt) = self.khayt.as_ref() else {
            return;
        };
        if let Some(hala) = khayt.hala_in_amkan() {
            if hala.mutawaqqifa() && !self.hala_khayt.mutawaqqifa() {
                self.athar.push(format!("recognition stopped: {}", hala.wasf()));
            }
            self.hala_khayt = hala;
        }
        if let Some(ihsaat) = khayt.ihsaat_in_amkan() {
            self.ihsaat_qissa = ihsaat;
        }
        if let Some(ihsaat) = khayt.tatabbu_in_amkan() {
            self.ihsaat_tatabbu = ihsaat;
        }
        if let Some(rafd) = khayt.akhir_rafd_in_amkan() {
            self.akhir_rafd = rafd;
        }
    }

    /// Refreshes the panel's snapshots.
    ///
    /// The cheap ones every frame; the ones that format strings or copy the
    /// history every thirtieth, which at sixty frames a second is twice a
    /// second and is as often as a person reads a number.
    fn haddith_lawha(&mut self) {
        self.lawha.hadith_meezaniya(*self.tabaqa.meezaniya());
        self.lawha
            .hadith_tabaqa(self.tabaqa.hala(), Some(self.tabaqa.wajiha()));
        if self.khayt.is_some() {
            self.lawha.hadith_khayt(self.hala_khayt.clone());
        }
        if self.ihsaat.itarat != 1 && !self.ihsaat.itarat.is_multiple_of(30) {
            return;
        }
        if let Some(sijill) = self.sijill.as_ref()
            && let (Some(madakhil), Some(taqreer)) =
                (sijill.laqta_in_amkan(64), sijill.taqreer_in_amkan())
        {
            self.lawha.hadith_sijill(madakhil, taqreer);
        }
        let (dhakira_qasir, dhakira) = self.khazina.as_ref().map_or_else(
            || ("لا خزينة".to_owned(), "لا خزينة على القرص.".to_owned()),
            |khazina| {
                let ihsaat = khazina.ihsaat();
                let qasir = ihsaat.nisbat_isaba().map_or_else(
                    || format!("{} زوجًا", ihsaat.adad),
                    |nisba| format!("إصابة {nisba:.0}٪ من {} زوجًا", ihsaat.adad),
                );
                (qasir, khazina.wasf_arabi())
            },
        );
        let mutarjim_qasir = match &self.hala_mutarjim {
            HalatMutarjim::Mutah { ism, namudhaj, .. } => format!("{ism} ({namudhaj})"),
            HalatMutarjim::Ghaib(_) => "غير مضبوط".to_owned(),
            HalatMutarjim::GhayrQabil { ism, .. } => format!("{ism}: لا يُوصَل إليه"),
            HalatMutarjim::LaIdadat { .. } => "تعذّرت قراءة الإعدادات".to_owned(),
        };
        let mut qissa = self.ihsaat_qissa.wasf_arabi();
        let _ = write!(qissa, ". {}", self.ihsaat_tatabbu.wasf());
        self.lawha.hadith_tarjama(HalatTarjamaLawha {
            mutarjim_mutah: self.hala_mutarjim.mutah(),
            mutarjim_qasir,
            mutarjim: self.hala_mutarjim.wasf_arabi(),
            dhakira_qasir,
            dhakira,
            qissa,
            iltiqat: self.ihsaat.wasf_arabi(),
            akhir_rafd: self.akhir_rafd.clone(),
        });
    }

    /// The one-line caption that says why nothing is drawn, while the panel is
    /// closed.
    fn tanbih(&self, lahza_mikro: u64, la_sutur: bool) -> Option<AnsurLawha> {
        if self.lawha.hala().zahira() || !la_sutur || !self.tabaqa.hala().tarsim() {
            return None;
        }
        let nass = if let Some((_, sabab_arabi)) = self.sabab_la_qari.as_ref() {
            format!("تعريب: لا يوجد محرّك قراءة — {sabab_arabi}")
        } else if let HalatKhayt::Mutawaqqifa { sabab_arabi, .. } = &self.hala_khayt {
            format!("تعريب: توقّفت القراءة — {sabab_arabi}")
        } else if !self.hala_mutarjim.mutah()
            && self.ihsaat_tatabbu.istiqrarat > 0
            && self.ihsaat_qissa.isabat_dhakira == 0
        {
            format!(
                "تعريب: قُرئ نصّ ولا مترجم يترجمه — {}",
                match &self.hala_mutarjim {
                    HalatMutarjim::Ghaib(hala) => hala.arabi().to_owned(),
                    hala => hala.wasf_arabi(),
                }
            )
        } else {
            let mundhu = lahza_mikro.saturating_sub(self.bidayat_mikro.unwrap_or(lahza_mikro));
            if mundhu > self.khiyarat.muhlat_tanbih_mikro {
                return None;
            }
            if self.ihsaat_qissa.maqruaat == 0 {
                "تعريب: تقرأ الشاشة… لم يُقرأ نصّ بعد".to_owned()
            } else {
                format!(
                    "تعريب: قُرئ {} سطرًا، وتنتظر ترجمتها",
                    self.ihsaat_qissa.maqruaat
                )
            }
        };
        let sath = self.sath_akhir?;
        if sath.ard == 0 || sath.irtifa == 0 {
            return None;
        }
        let irtifa = (LawhatTahakkum::IRTIFA_SAF_BIKSEL / madaa(sath.irtifa)).clamp(0.01, 0.2);
        Some(AnsurLawha::bi_nass(
            MustatilNisbi {
                yasar: 0.15,
                aala: 0.012,
                ard: 0.7,
                irtifa,
            },
            DawrAnsur::Tarwisa,
            nass,
            self.lawha.hala().ataama(),
        ))
    }
}

/// The microseconds between two readings of the hook's clock, saturating.
fn mikro_bayn(qabl: u64, baad: u64) -> u32 {
    u32::try_from(baad.saturating_sub(qabl)).unwrap_or(u32::MAX)
}

/// A surface dimension as a float.
const fn madaa(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

// ---------------------------------------------------------------------------
// Resolving the parts
// ---------------------------------------------------------------------------

/// The tier's own directory under the data root.
#[must_use]
pub fn mujallad_tabaqa(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join(crate::DALIL_TABAQA)
}

/// The settings, read once, or the defaults with the reason they are defaults.
#[must_use]
pub fn idadat(masarat: &Masarat) -> (Arc<Idadat>, Option<String>) {
    match MakhzanIdadat::iftah(masarat) {
        Ok(makhzan) => (makhzan.hali(), None),
        Err(khata) => (Arc::new(Idadat::default()), Some(khata.injilizi)),
    }
}

/// The directories an Arabic font may be found in, most specific first.
///
/// Beside the payload, the user's own font folder, the fonts the registry
/// cache staged, and the components directory — then the same for the folder
/// the settings name. A font found beside the payload wins because that is
/// where an installer that wanted a particular face for a particular game would
/// put it.
#[must_use]
pub fn mujalladat_khutut(masarat: &Masarat, idadat: &Idadat, mujallad_hamula: &Path) -> Vec<PathBuf> {
    let mut mujalladat = vec![
        mujallad_hamula.to_path_buf(),
        masarat.khutut(),
        masarat.makhbaa().join("sans"),
        masarat.makhbaa().join("khutut"),
        masarat.makhbaa(),
        masarat.mukawwinat().join("khutut"),
        masarat.mukawwinat(),
    ];
    if let Some(masar) = idadat.khutut.masar_khutut_mustakhdim.as_ref() {
        mujalladat.push(masar.clone());
    }
    mujalladat
}

/// The first font in the given directories that passes the Arabic checks.
///
/// Files named for Arabic — `Arabic`, `Naskh`, `Kufi`, `Amiri` — are tried
/// before the rest of a directory, and each directory is searched one level
/// deep. Every candidate goes through [`MawridKhatt::jadeed`], which is the
/// Decision 6 gate: a face that does not join, or lacks a letter, is not a
/// face this tier draws with.
///
/// # Errors
///
/// [`KhataTabaqa::MawridFashil`] naming every directory searched when none of
/// them holds a usable face, which is the sentence the log needs.
pub fn ijad_khutut(mujalladat: &[PathBuf]) -> Result<(SilsilatKhutut, PathBuf), KhataTabaqa> {
    const ASMA_ARABIYA: [&str; 5] = ["arabic", "naskh", "kufi", "amiri", "arab"];
    let mut murashahat: Vec<PathBuf> = Vec::new();
    for mujallad in mujalladat {
        let mut hadha = malaffat_khutut(mujallad, 1);
        hadha.sort_by_key(|masar| {
            let ism = masar
                .file_name()
                .map(|ism| ism.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let arabi = ASMA_ARABIYA.iter().any(|wasm| ism.contains(wasm));
            (!arabi, ism)
        });
        murashahat.extend(hadha);
    }
    let mut asbab: Vec<String> = Vec::new();
    for masar in &murashahat {
        let Ok(bayt) = fs::read(masar) else {
            continue;
        };
        match MawridKhatt::jadeed(Arc::new(bayt), 0) {
            Ok(khatt) => match SilsilatKhutut::wahid(Arc::new(khatt)) {
                Ok(silsila) => return Ok((silsila, masar.clone())),
                Err(khata) => asbab.push(format!("{}: {}", masar.display(), khata.injilizi)),
            },
            Err(khata) => {
                if asbab.len() < 8 {
                    asbab.push(format!("{}: {}", masar.display(), khata.injilizi));
                }
            },
        }
    }
    let mujalladat_nass = mujalladat
        .iter()
        .map(|masar| masar.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(KhataTabaqa::MawridFashil {
        mawrid: "Arabic font",
        sabab: format!(
            "no font that passes the Arabic checks was found in [{mujalladat_nass}]; {} \
             candidate(s) tried{}",
            murashahat.len(),
            if asbab.is_empty() {
                String::new()
            } else {
                format!(": {}", asbab.join(" | "))
            }
        ),
    })
}

/// Every `.ttf`/`.otf` under a directory, to a depth.
fn malaffat_khutut(mujallad: &Path, umq: u32) -> Vec<PathBuf> {
    let mut malaffat = Vec::new();
    let Ok(madakhil) = fs::read_dir(mujallad) else {
        return malaffat;
    };
    for madkhal in madakhil.flatten() {
        let masar = madkhal.path();
        if masar.is_dir() {
            if umq > 0 {
                malaffat.extend(malaffat_khutut(&masar, umq.saturating_sub(1)));
            }
            continue;
        }
        let lahiqa = masar
            .extension()
            .map(|lahiqa| lahiqa.to_string_lossy().to_ascii_lowercase());
        if matches!(lahiqa.as_deref(), Some("ttf" | "otf")) {
            malaffat.push(masar);
        }
    }
    malaffat
}

/// The recognizer the settings ask for, over the model directories that exist.
///
/// # Errors
///
/// As [`IkhtiyarQari::ikhtar`], carrying the whole trail.
pub fn ikhtar_qari(
    masarat: &Masarat,
    idadat: &Idadat,
    mujallad_hamula: &Path,
    lugha: &str,
) -> Result<IkhtiyarQari, KhataTabaqa> {
    let murashahat = [
        mujallad_hamula.join(DALIL_NAMADHIJ),
        masarat.mukawwinat().join(DALIL_NAMADHIJ),
        masarat.makhbaa().join(DALIL_NAMADHIJ),
    ];
    let mujallad = murashahat
        .iter()
        .find(|masar| masar.is_dir())
        .cloned()
        .unwrap_or_else(|| masarat.mukawwinat().join(DALIL_NAMADHIJ));
    let mut iadadat = IdadatQira::jadeeda(lugha, mujallad);
    iadadat.yufaddil_al_mahmul = matches!(idadat.tabaqa.muharrik_qira, MuharrikQira::Mahmul);
    IkhtiyarQari::ikhtar(&iadadat)
}

/// The translator the settings elect, and the sentence about it.
#[must_use]
pub fn mutarjim_min_idadat(
    idadat: &Idadat,
    sabab_la_idadat: Option<&str>,
) -> (Option<Box<dyn MutarjimTabaqa>>, HalatMutarjim) {
    if let Some(sabab) = sabab_la_idadat {
        return (
            None,
            HalatMutarjim::LaIdadat {
                sabab: sabab.to_owned(),
            },
        );
    }
    let (mutarjim, hala) = MutarjimMahalli::min_idadat(
        idadat.muzawwidun.muntakhab(),
        idadat.muzawwidun.hala(),
    );
    let mutarjim: Option<Box<dyn MutarjimTabaqa>> = match mutarjim {
        Some(mutarjim) => Some(Box::new(mutarjim)),
        None => None,
    };
    (mutarjim, hala)
}

/// The layered memory: the patch's strings, then the on-disk cache.
#[must_use]
pub fn dhakira_murakkaba(
    ruqaa: Option<Arc<DhakiraRuqaa>>,
    khazina: Arc<DhakiraMalaf>,
) -> Arc<dyn DhakiraTabaqa> {
    let mut qabl: Vec<Arc<dyn DhakiraTabaqa>> = Vec::new();
    if let Some(ruqaa) = ruqaa {
        qabl.push(ruqaa);
    }
    Arc::new(DhakiraMurakkaba::jadeeda(qabl, khazina))
}

/// Opens the on-disk cache for a language under the tier's directory.
///
/// # Errors
///
/// As [`DhakiraMalaf::iftah`].
pub fn iftah_khazina(mujallad_tabaqa: &Path, lugha: &str) -> Result<Arc<DhakiraMalaf>, KhataTabaqa> {
    DhakiraMalaf::iftah(masar_khazina(mujallad_tabaqa, lugha), lugha).map(Arc::new)
}

/// The game's identity and name as the patch's manifest records them, when it
/// does.
///
/// The manifest is read by field name rather than through the builder's type,
/// which lives in a crate the payload does not link. A manifest without the
/// fields is not an error — a font-only patch has no game — and the caller
/// falls back to what it can see.
#[must_use]
pub fn luba_min_bayan(bayan: &Value) -> (Option<LubaId>, Option<String>, Option<String>) {
    let wasf = bayan.get("wasf").unwrap_or(bayan);
    let luba = wasf
        .get("luba")
        .or_else(|| bayan.get("luba"))
        .and_then(|qeema| serde_json::from_value::<LubaId>(qeema.clone()).ok());
    let ism = wasf
        .get("ism_luba")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|ism| !ism.is_empty())
        .map(str::to_owned);
    let lugha = wasf
        .get("lugha_asl")
        .or_else(|| wasf.get("lugha"))
        .or_else(|| bayan.get("lugha_asl"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|lugha| !lugha.is_empty())
        .map(str::to_owned);
    (luba, ism, lugha)
}

/// The region set for a game, read from disk or empty.
///
/// A set that exists and cannot be read is reported and replaced by an empty
/// one *in memory only*: the loop never writes the set back, so the file on
/// disk is untouched for the editor that can.
#[must_use]
pub fn manatiq_li(
    mujallad_tabaqa: &Path,
    luba: LubaId,
    ism_luba: &str,
    athar: &mut Vec<String>,
) -> MajmuatManatiq {
    let masar = MajmuatManatiq::masar_malaf(mujallad_tabaqa, luba);
    match MajmuatManatiq::hammil_aw_jadeeda(&masar, luba, ism_luba) {
        Ok(majmua) => {
            for satr in majmua.mulahazat() {
                athar.push(format!("regions: {satr}"));
            }
            majmua
        },
        Err(khata) => {
            athar.push(format!(
                "regions: {} could not be read, so none are used: {khata}",
                masar.display()
            ));
            MajmuatManatiq::jadeeda(luba, ism_luba)
        },
    }
}

/// The reading history for a game, opened on disk.
///
/// The game's directory under the tier's is created first: the history writes
/// its header on open, and a first run has nothing there yet.
#[must_use]
pub fn sijill_li(
    mujallad_tabaqa: &Path,
    luba: LubaId,
    saa: usize,
    athar: &mut Vec<String>,
) -> Option<SijillMushtarak> {
    let masar = SijillQira::masar_malaf(mujallad_tabaqa, luba);
    if let Some(walid) = masar.parent()
        && let Err(sabab) = fs::create_dir_all(walid)
    {
        athar.push(format!(
            "history: {} could not be created, so nothing is recorded: {sabab}",
            walid.display()
        ));
        return None;
    }
    match SijillQira::iftah(&masar, luba, saa) {
        Ok(sijill) => Some(SijillMushtarak::jadeed(sijill)),
        Err(khata) => {
            athar.push(format!(
                "history: {} could not be opened, so nothing is recorded: {khata}",
                masar.display()
            ));
            None
        },
    }
}

/// The chord table for the tier, read from disk or the shipped defaults.
///
/// A missing file is the defaults already — [`Ikhtisarat::hammil`] says so —
/// and only a file that exists and cannot be used is worth a line in the log.
#[must_use]
pub fn ikhtisarat_li(mujallad_tabaqa: &Path, athar: &mut Vec<String>) -> Ikhtisarat {
    let masar = Ikhtisarat::masar_malaf(mujallad_tabaqa);
    match Ikhtisarat::hammil(&masar) {
        Ok(ikhtisarat) => ikhtisarat,
        Err(khata) => {
            athar.push(format!(
                "shortcuts: {} could not be read, so the defaults apply: {khata}",
                masar.display()
            ));
            Ikhtisarat::iftiradiya()
        },
    }
}

