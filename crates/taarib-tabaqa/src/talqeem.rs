//! التلقيم — the layer that feeds the overlay, between the text and the quads.
//!
//! Three modules in this crate each stop one step short of drawing, and they all
//! stop at the same step. [`crate::qira`] returns [`crate::qira::SatrMaqru`] — a
//! string and the pixel box it was read from. [`crate::lawhat_tahakkum`] returns
//! [`AnsurLawha`] — a normalized rectangle, a role, a colour and a `String`.
//! [`crate::rasm_tabaqa`] turns *already shaped* glyphs into quads. Between the
//! strings and the shaped glyphs there is exactly one missing stage: shape the
//! text, put every glyph it produced into the atlas, and hand the result to the
//! batch builder. Both of those modules say so in their own headers — "one layer
//! up shapes the text into them". This is that layer.
//!
//! Without it [`crate::wajiha::Tabaqa::itar`] has no caller, because nothing in
//! the crate can build the [`LawhatRasm`] it takes.
//!
//! ## Where a position comes from, and why it is the screen
//!
//! Tier 3 exists for games Taarib cannot patch. That is the whole premise, and
//! it settles the question: there is no hooked text draw call to read a position
//! out of, because a game whose text draw call could be hooked and understood is
//! a game an engine adapter handles at tier 1 or 2. There is no UI atlas either,
//! for the same reason — knowing where a game's dialogue box is means knowing
//! the game.
//!
//! What is left is the picture. The recognizer reads a line and reports the
//! rectangle it read it from, the translated Arabic is drawn into **that**
//! rectangle, and the plate behind it covers the original. The position is
//! therefore scraped, it is only ever as good as the recognizer, and it moves
//! when the game's own text moves because it is derived from it every frame.
//! That is the honest cost of this tier and it is the reason
//! [`crate::sidq::NASS_IFSAH_ARABI`] says recognition makes mistakes before the
//! overlay is allowed to start.
//!
//! ## One atlas page, deliberately
//!
//! [`crate::wajiha::Khattaf::arfa_lawha`] uploads **one** texture, and
//! [`crate::rasm_tabaqa::ibn_dufa`] normalizes each glyph against the dimensions
//! of the page it landed on. Two pages and one texture means every quad from the
//! second page samples the first — the wrong letter, not a missing one, which is
//! the failure [`taarib_lawha::namu`] goes to some length to make impossible
//! inside the atlas and which would be reintroduced here. So the runtime atlas
//! this module builds is capped at a single page, and when it fills it evicts by
//! least-recently-used rectangle exactly as it is designed to.
//!
//! ## A lost glyph is not a lost frame
//!
//! [`crate::khata`] states this crate's posture: a frame Taarib cannot draw into
//! is a frame that renders without Arabic, not an error anybody needs to see.
//! This module keeps to it. A glyph the atlas refuses is counted in
//! [`IhsaatTalqeem::ashkal_mafquda`], its reason is kept in
//! [`IhsaatTalqeem::akhir_radd`], and the rest of the frame is still drawn.
//! Counted and named rather than absorbed, because an atlas that keeps refusing
//! is a real defect and a silent one is a defect nobody finds.

use core::fmt;

use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_lawha::namu::{IhsaatNamu, LawhaHayya};
use taarib_lawha::rasf::KhiyaratRasf;
use taarib_saff::khatt::SilsilatKhutut;
use taarib_saff::natija::TakhtitNass;
use taarib_saff::talab::{
    IttijahAsas, KhiyaratTakhtit, LughaNass, Muhadhaha, NamatDabt, SiyasatTajawuz,
};
use taarib_saff::{Saff, TalabTakhtit};
use taarib_usus::khata::Natija;

use crate::lawhat_tahakkum::{AnsurLawha, DawrAnsur};
use crate::qira::SatrMaqru;
use crate::rasm_tabaqa::{AlwanTabaqa, BaniDufa, ibn_dufa, ila_rgba, qitaat_nass};
use crate::wajiha::{LawhatRasm, MustatilBiksel, QitaRasm, Tabaqa, WasfSath};

/// One translated line, and the screen rectangle the text it replaces occupied.
///
/// The rectangle is in **surface** pixels, not the captured image's. The trip
/// from one to the other is
/// [`crate::iltiqat_shasha::SuraMultaqata::ila_sath`]'s, made once by the caller
/// that holds both, which is why the recognizer's own
/// [`SatrMaqru::mawdi`] is not simply reused here without a conversion having
/// happened somewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatrMulaqqam {
    /// The Arabic, in logical order, as ordinary Unicode.
    ///
    /// Never presentation forms. Joining, the four contextual shapes and every
    /// ligature come out of the font's own `GSUB` and `GPOS` when this string is
    /// shaped, which is the one place in the product they are allowed to come
    /// from.
    pub nass: String,
    /// Where the original text was, in surface pixels.
    pub mawdi: MustatilBiksel,
    /// The recognizer's confidence in the line this was translated from, zero to
    /// a hundred.
    ///
    /// Carried rather than dropped because a caller may want to fade or skip a
    /// line the recognizer was unsure of, and the decision belongs to whoever
    /// assembles the frame rather than to this type.
    pub thiqa: u8,
}

impl SatrMulaqqam {
    /// A fed line from a recognized one and the Arabic that came back for it.
    ///
    /// `mawdi` is passed separately and deliberately: [`SatrMaqru::mawdi`] is in
    /// the captured image's coordinates, and using it unconverted is the one
    /// mistake this signature exists to make impossible to write by accident.
    #[must_use]
    pub fn min_maqru(maqru: &SatrMaqru, mawdi: MustatilBiksel, arabi: impl Into<String>) -> Self {
        Self { nass: arabi.into(), mawdi, thiqa: maqru.thiqa }
    }

    /// Whether this line would draw anything.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.nass.trim().is_empty() || self.mawdi.ard == 0 || self.mawdi.irtifa == 0
    }
}

/// How the feeder sizes, colours and budgets what it draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KhiyaratTalqeem {
    /// The em size a line is laid out at, as a share of its box height.
    ///
    /// A recognizer's box hugs the ink it found, and how much of an em that is
    /// depends on what the line happens to contain: a line with an ascender and
    /// a descender fills most of one, a line of capitals and x-height letters
    /// fills about seven tenths. Nothing in the box says which, so this is an
    /// estimate and cannot be anything else. A little under one keeps Arabic
    /// close to the size of the text it replaces without overshooting the common
    /// case.
    ///
    /// It is the starting size, not the final one — the overflow policy shrinks
    /// from here when the Arabic is wider than the space it has, which it often
    /// is.
    pub nisbat_hajm: f32,
    /// The floor the overflow policy may shrink to, in pixels.
    ///
    /// Text below this is text nobody can read, and drawing it would trade a
    /// legible overflow for an illegible fit.
    pub adna_hajm: f32,
    /// The ceiling on the starting size, in pixels.
    ///
    /// A recognizer that returns one enormous box for a whole screen would
    /// otherwise ask the atlas for a glyph larger than its page.
    pub aqsa_hajm: f32,
    /// The side of the single atlas page, in texels.
    pub dila_lawha: u16,
    /// How the page is rasterized.
    pub namat: NamatSafha,
    /// The palette translated lines and their plates are drawn in.
    pub alwan: AlwanTabaqa,
    /// The alpha the plate takes when it is covering text it replaces.
    ///
    /// [`AlwanTabaqa::iftiradiya`]'s plate is three quarters opaque on purpose —
    /// a plate that hides the art the player is looking at is a worse plate. But
    /// that reasoning is about a caption laid over scenery, and this tier's plate
    /// has a second job the palette knows nothing about: the sentence
    /// underneath it is bright interface text, and at three quarters it stays
    /// legible right through the Arabic. Two languages saying the same thing in
    /// the same rectangle is the failure this exists to prevent, so the default
    /// here is opaque and the choice is the caller's, per game.
    pub ataamat_hajb: u8,
}

impl KhiyaratTalqeem {
    /// The defaults.
    ///
    /// A 1024-square coverage page: one mebibyte held, four uploaded as RGBA8,
    /// and room for well over a thousand glyphs at interface sizes — which is
    /// several times the working set of any one screen of dialogue, and small
    /// enough to be memory taken from a process that is not ours without anybody
    /// noticing.
    #[must_use]
    pub const fn iftiradiya() -> Self {
        Self {
            nisbat_hajm: 0.9,
            adna_hajm: 9.0,
            aqsa_hajm: 160.0,
            dila_lawha: 1024,
            namat: NamatSafha::Taghtiya,
            alwan: AlwanTabaqa::iftiradiya(),
            ataamat_hajb: 0xFF,
        }
    }
}

impl Default for KhiyaratTalqeem {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

/// What the feeder has done, for the control panel and the diagnostics bundle.
#[derive(Debug, Clone, Default)]
pub struct IhsaatTalqeem {
    /// Translated lines shaped into the last frame.
    pub sutur: u32,
    /// Panel elements laid into the last frame.
    pub ansur: u32,
    /// Quads the last frame's batch carried.
    pub qitaat: u32,
    /// Glyphs the atlas would not take, over this feeder's life.
    ///
    /// Cumulative, not per frame. A number that climbs is an atlas too small for
    /// what is on screen at once, and the first frame it happens on is not the
    /// interesting one.
    pub ashkal_mafquda: u64,
    /// Atlas uploads handed to the backend.
    pub marfuaat: u64,
    /// Why the last glyph was refused, when one was.
    pub akhir_radd: Option<String>,
    /// The runtime atlas's own counters.
    pub lawha: IhsaatNamu,
}

/// The feeder: a shaper, a font chain, a runtime atlas, and the frame's batch.
///
/// One per overlay. It is not `Sync` and does not try to be — [`Saff`] is a
/// mutable cache and this runs on the render thread that is about to present.
pub struct Mulaqqim {
    saff: Saff,
    khutut: SilsilatKhutut,
    lawha: LawhaHayya,
    khiyarat: KhiyaratTalqeem,
    /// The layout decisions, built once because they never vary per line.
    takhtit: KhiyaratTakhtit,
    /// The page expanded to RGBA8, kept so an upload does not allocate on the
    /// render thread every time the atlas grows.
    rgba: Vec<u8>,
    /// Whether the page has changed since the backend last received it.
    muttasikha: bool,
    ihsaat: IhsaatTalqeem,
}

impl fmt::Debug for Mulaqqim {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The RGBA staging buffer is reported as a length. A debug line that
        // dumps four mebibytes of expanded atlas into a log is a log nobody can
        // read, which is the same judgement `taarib_lawha::rasf::Safha` makes.
        matbaa
            .debug_struct("Mulaqqim")
            .field("khutut", &self.khutut.adad())
            .field("lawha", &self.lawha)
            .field("khiyarat", &self.khiyarat)
            .field("rgba", &self.rgba.len())
            .field("muttasikha", &self.muttasikha)
            .field("ihsaat", &self.ihsaat)
            .finish_non_exhaustive()
    }
}

impl Mulaqqim {
    /// Builds a feeder over a font chain.
    ///
    /// The chain is the one the patch shipped or the one the overlay bundled;
    /// either way it is handed in rather than loaded here, because this crate
    /// reads no files from a game's render thread.
    ///
    /// # Errors
    ///
    /// Whatever [`LawhaHayya::jadeeda`] refuses — page dimensions the packer
    /// cannot use, or a budget that will not pay for one page.
    pub fn jadeed(khutut: SilsilatKhutut, khiyarat: KhiyaratTalqeem) -> Natija<Self> {
        let rasf = KhiyaratRasf {
            aqsa_ard: khiyarat.dila_lawha,
            aqsa_irtifa: khiyarat.dila_lawha,
            // One page. See this module's header: the backend uploads one
            // texture and a second page would be sampled as the first.
            aqsa_safahat: 1,
            ..KhiyaratRasf::default()
        };
        let mizaniya =
            usize::from(khiyarat.dila_lawha).saturating_mul(usize::from(khiyarat.dila_lawha));
        let lawha = LawhaHayya::jadeeda(rasf, khiyarat.namat, mizaniya)?;

        Ok(Self {
            saff: Saff::jadeed(),
            khutut,
            lawha,
            khiyarat,
            takhtit: takhtit_iftiradi(khiyarat.adna_hajm),
            rgba: Vec::new(),
            muttasikha: true,
            ihsaat: IhsaatTalqeem::default(),
        })
    }

    /// The options this feeder was built with.
    #[must_use]
    pub const fn khiyarat(&self) -> &KhiyaratTalqeem {
        &self.khiyarat
    }

    /// What it has done.
    #[must_use]
    pub const fn ihsaat(&self) -> &IhsaatTalqeem {
        &self.ihsaat
    }

    /// The runtime atlas, for a caller reading its counters.
    #[must_use]
    pub const fn lawha(&self) -> &LawhaHayya {
        &self.lawha
    }

    /// Whether the page has changed since the backend last received it.
    #[must_use]
    pub const fn tahtaj_rafan(&self) -> bool {
        self.muttasikha
    }

    /// Builds one frame's batch.
    ///
    /// In order: release the previous frame's atlas pins, shape every translated
    /// line into the rectangle it replaces, shape every panel element's label
    /// into its own rectangle, and finish the batch against `sath`. Plates are
    /// emitted before the glyphs that sit on them, because there is no depth
    /// buffer and submission order is the layering.
    ///
    /// The batch records `sath`, and [`Tabaqa::itar`] discards one whose surface
    /// no longer matches. That is the intended behaviour and not a failure: the
    /// next frame's batch is built against the new surface and is correct.
    ///
    /// # Errors
    ///
    /// Whatever [`Saff::khattit`] reports for text it cannot lay out, and
    /// whatever [`ibn_dufa`] reports for a surface with no area. A glyph the
    /// atlas refuses is **not** an error — see this module's header.
    pub fn ibni(
        &mut self,
        sath: WasfSath,
        sutur: &[SatrMulaqqam],
        ansur: &[AnsurLawha],
    ) -> Natija<LawhatRasm> {
        // Once per frame, before anything asks for a glyph. Skipping it fills
        // the atlas with rectangles the evictor may never take back.
        self.lawha.ibda_itar();

        let mut bani = BaniDufa::jadeed(sath);
        self.ihsaat.sutur = 0;
        self.ihsaat.ansur = 0;

        for satr in sutur {
            if satr.khali() {
                continue;
            }
            self.laqqim_satr(&mut bani, sath, satr)?;
            self.ihsaat.sutur = self.ihsaat.sutur.saturating_add(1);
        }

        for unsur in ansur {
            self.laqqim_unsur(&mut bani, sath, unsur)?;
            self.ihsaat.ansur = self.ihsaat.ansur.saturating_add(1);
        }

        let lawha = bani.ikhtim();
        self.ihsaat.qitaat = u32::try_from(lawha.qitaat.len()).unwrap_or(u32::MAX);
        self.ihsaat.lawha = self.lawha.ihsaat();
        Ok(lawha)
    }

    /// Uploads the atlas page to the backend, when it has changed.
    ///
    /// Returns whether an upload happened. Called between frames rather than
    /// inside one where it can be helped: expanding the page to RGBA8 and
    /// writing a texture is the most expensive thing this module does, and it is
    /// needed only when a glyph the atlas had never seen turned up.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataTabaqa::MawridFashil`] when the page's byte count does not match
    /// its declared dimensions, and whatever
    /// [`crate::wajiha::Khattaf::arfa_lawha`] refuses.
    pub fn irfa(&mut self, tabaqa: &mut Tabaqa) -> Natija<bool> {
        if !self.muttasikha {
            return Ok(false);
        }
        let Some(safha) = self.lawha.safahat().first() else {
            return Ok(false);
        };
        let (ard, irtifa) = (safha.ard, safha.irtifa);
        self.rgba = ila_rgba(&safha.bayt, ard, irtifa)?;
        tabaqa.arfa_lawha(&self.rgba, u32::from(ard), u32::from(irtifa))?;
        self.muttasikha = false;
        self.ihsaat.marfuaat = self.ihsaat.marfuaat.saturating_add(1);
        Ok(true)
    }

    /// Builds the frame, uploads the atlas if it changed, and draws it.
    ///
    /// This is the whole loop in one call, and it is the one a present hook
    /// makes. The order is not negotiable: the batch is built first because
    /// building it is what discovers the glyphs the atlas is missing, the upload
    /// happens second because a quad drawn before its glyph reached the texture
    /// samples whatever was in that rectangle before, and the draw happens last.
    ///
    /// `mikro` is the caller's measurement of the previous overlay draw, in
    /// microseconds, which is what [`Tabaqa::itar`] holds the frame budget
    /// against. The clock is the hook's: this runs on a render thread where the
    /// platform timer is already being read by the code that called us.
    ///
    /// # Errors
    ///
    /// As [`Mulaqqim::ibni`], [`Mulaqqim::irfa`] and [`Tabaqa::itar`].
    pub fn qaddim(
        &mut self,
        tabaqa: &mut Tabaqa,
        sath: WasfSath,
        sutur: &[SatrMulaqqam],
        ansur: &[AnsurLawha],
        mikro: u32,
    ) -> Natija<LawhatRasm> {
        let lawha = self.ibni(sath, sutur, ansur)?;
        let _ = self.irfa(tabaqa)?;
        tabaqa.itar(&lawha, mikro)?;
        Ok(lawha)
    }

    /// Empties the atlas, for a level transition that replaces the working set.
    ///
    /// The lifetime counters survive, because the question they answer is about
    /// the session rather than about the current contents.
    pub fn amsah(&mut self) {
        self.lawha.amsah();
        self.saff.amsah();
        self.muttasikha = true;
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Shapes one translated line into the rectangle it replaces, with a plate.
    fn laqqim_satr(
        &mut self,
        bani: &mut BaniDufa,
        sath: WasfSath,
        satr: &SatrMulaqqam,
    ) -> Natija<()> {
        let sunduq = satr.mawdi;
        let hajm = self.hajm_li(sunduq.irtifa);
        // Width constrains, height does not. A recognizer's box is the ink it
        // found and a line box is legitimately taller than its own ink, so
        // holding the layout to the box's height would report vertical overflow
        // on text that fits perfectly well and shrink it to the floor every
        // time. Only the width is a real limit: it is the room the original
        // occupied, and Arabic wider than that runs into whatever is beside it.
        let takhtit = khattit(
            &mut self.saff,
            &self.khutut,
            &self.takhtit,
            &satr.nass,
            hajm,
            Some(madaa(sunduq.ard)),
            None,
        )?;
        if takhtit.khali() {
            return Ok(());
        }

        let hajm_rubi = self.hayyi_ashkal(&takhtit);
        let alwan = AlwanTabaqa {
            lawh: [
                self.khiyarat.alwan.lawh[0],
                self.khiyarat.alwan.lawh[1],
                self.khiyarat.alwan.lawh[2],
                self.khiyarat.ataamat_hajb,
            ],
            ..self.khiyarat.alwan
        };
        // The pen is the layout's top-left corner in surface pixels, which is
        // what `Harf::s` and `Harf::a` are measured from. See `qita_min_harf`.
        let qitaat = ibn_dufa(
            &takhtit,
            self.lawha.khareeta(),
            sath,
            madaa(sunduq.yasar),
            wasat_amudi(sunduq, takhtit.irtifa),
            &alwan,
            hajm_rubi,
            self.khiyarat.namat,
            // The plate must hide the sentence this one replaces, not merely
            // back the Arabic. Passing the recognizer's own box is what makes
            // the original stop showing beside a shorter translation.
            Some(sunduq),
        )?;
        bani.adhif(qitaat);
        Ok(())
    }

    /// Lays one panel element into the batch: its plate, then its label.
    fn laqqim_unsur(
        &mut self,
        bani: &mut BaniDufa,
        sath: WasfSath,
        unsur: &AnsurLawha,
    ) -> Natija<()> {
        let Some(sunduq) = unsur.mustatil.fi_bikselat(sath.ard, sath.irtifa) else {
            // A one-pixel separator on a small window rounds to nothing. Not an
            // error anywhere it is produced, and not worth a zero-area quad.
            return Ok(());
        };

        // The panel hands out one colour per element, and a role is what says
        // whether that colour is a plate or the ink on one. The dark roles are
        // backgrounds with a label on top; the two light ones are the label
        // itself and have no plate of their own — the panel already drew one
        // underneath them.
        if lawh_lil_dawr(unsur.dawr) {
            bani.adhif_wahid(QitaRasm { mawdi: sunduq, khareeta: None, lawn: unsur.lawn });
        }

        let Some(nass) = unsur.nass.as_deref() else {
            return Ok(());
        };
        if nass.trim().is_empty() || !unsur.dawr.yahmil_nassan() {
            return Ok(());
        }

        // Inset so a label does not touch the edge of its own plate. A tenth of
        // the row height on each side, which stays right when the user scales
        // the panel's font rather than needing a second setting.
        let hashiya = madaa(sunduq.irtifa) * 0.1;
        let ard_mutah = (madaa(sunduq.ard) - hashiya * 2.0).max(1.0);
        let hajm = self.hajm_li(sunduq.irtifa);
        let khutut = &self.khutut;
        let takhtit =
            khattit(&mut self.saff, khutut, &self.takhtit, nass, hajm, Some(ard_mutah), None)?;
        if takhtit.khali() {
            return Ok(());
        }

        let hajm_rubi = self.hayyi_ashkal(&takhtit);
        let lawn = if lawh_lil_dawr(unsur.dawr) {
            // The plate carries the element's colour, so the label takes the
            // palette's text colour. `AnsurLawha::lawn` is already premultiplied
            // linear and is passed through untouched; converting it again is the
            // one mistake this branch exists to keep out.
            AlwanTabaqa::ila_khatti(self.khiyarat.alwan.nass)
        } else {
            unsur.lawn
        };
        let qitaat = qitaat_nass(
            &takhtit,
            self.lawha.khareeta(),
            madaa(sunduq.yasar) + hashiya,
            wasat_amudi(sunduq, takhtit.irtifa),
            lawn,
            hajm_rubi,
            self.khiyarat.namat,
        );
        bani.adhif(qitaat);
        Ok(())
    }

    /// The em size to start a box of this height at, inside the configured
    /// bounds.
    fn hajm_li(&self, irtifa: u32) -> f32 {
        let khaam = madaa(irtifa) * self.khiyarat.nisbat_hajm;
        if khaam.is_finite() {
            khaam.clamp(self.khiyarat.adna_hajm, self.khiyarat.aqsa_hajm)
        } else {
            self.khiyarat.adna_hajm
        }
    }

    /// Puts every glyph a layout produced into the atlas, and returns the
    /// quarter-pixel size its keys were built at.
    ///
    /// The size comes from [`TakhtitNass::hajm`] rather than from what was
    /// requested, because the overflow policy may have shrunk the text — and a
    /// key built at the requested size would name an image of a different size
    /// than the one the layout measured against.
    ///
    /// A refusal is counted and the glyph is left out. [`ibn_dufa`] skips a
    /// glyph the map does not hold, so the line draws with a gap rather than
    /// with a wrong letter in it.
    fn hayyi_ashkal(&mut self, takhtit: &TakhtitNass) -> u16 {
        let hajm_rubi = MiftahShakl::jadeed(0, 0, takhtit.hajm, self.khiyarat.namat, 0).hajm_rubi;
        for (khatt, muarrif) in takhtit.ashkal() {
            let miftah = MiftahShakl {
                khatt,
                // Zero, matching the bucket `ibn_dufa` builds its lookup key
                // with. A subpixel-positioned atlas needs the layout's own
                // fractional x on both sides of that lookup, and the batch
                // builder does not carry one — so the two agree at zero rather
                // than disagreeing at a bucket only one of them knows about.
                bakat: 0,
                hajm_rubi,
                namat: self.khiyarat.namat,
                muarrif,
            };
            if self.lawha.yahwi(miftah) {
                // Already packed. Pin it anyway: the batch is about to reference
                // the rectangle, and an unpinned rectangle is one this frame's
                // later glyphs could evict out from under it.
                self.lawha.ithbit(miftah);
                continue;
            }
            match self.lawha.shakl_min_silsila(miftah, &self.khutut) {
                Ok(_) => self.muttasikha = true,
                Err(khata) => {
                    self.ihsaat.ashkal_mafquda = self.ihsaat.ashkal_mafquda.saturating_add(1);
                    self.ihsaat.akhir_radd = Some(khata.injilizi.clone());
                },
            }
        }
        hajm_rubi
    }
}

/// Whether a role's colour is a plate rather than ink.
///
/// The panel's theme makes this unambiguous: the four dark roles and the
/// highlight are backgrounds a label sits on, and the two light ones are the
/// label. Written here rather than on [`DawrAnsur`] because it is this module's
/// reading of the theme, and a different renderer could legitimately read it
/// differently.
const fn lawh_lil_dawr(dawr: DawrAnsur) -> bool {
    matches!(
        dawr,
        DawrAnsur::Khalfiya
            | DawrAnsur::Tarwisa
            | DawrAnsur::Zir
            | DawrAnsur::Fasil
            | DawrAnsur::Ibraz
    )
}

/// The layout decisions every line in this tier is laid out with.
///
/// [`IttijahAsas::Tilqai`] rather than a forced right-to-left: the base
/// direction comes from the first strong character per rule P2/P3, so an Arabic
/// line reads right to left and a line that came back untranslated — a proper
/// noun, a number, a menu key — still reads left to right instead of being
/// reversed on screen.
///
/// [`SiyasatTajawuz::Taqlis`] rather than the default report-and-overflow:
/// Arabic is routinely longer than the English it replaces, the rectangle is not
/// negotiable because it is the space the original occupied, and text that spills
/// out of its own plate onto the game is worse than text a point smaller.
///
/// `satr_wahid` because every rectangle this tier draws into held **one** line
/// when the recognizer found it. Letting a translation wrap would put two lines
/// of Arabic in the height of one and over the game's next line of dialogue,
/// which is worse than one line a point smaller.
fn takhtit_iftiradi(adna: f32) -> KhiyaratTakhtit {
    KhiyaratTakhtit {
        ittijah: IttijahAsas::Tilqai,
        lugha: LughaNass::Arabi,
        dabt: NamatDabt::Bila,
        muhadhaha: Muhadhaha::Bidaya,
        tajawuz: SiyasatTajawuz::Taqlis { adna },
        satr_wahid: true,
        ..KhiyaratTakhtit::default()
    }
}

/// One layout, with the engine's caches borrowed apart from the rest of the
/// feeder.
///
/// A free function rather than a method because shaping needs `&mut Saff` while
/// the chain and the options are borrowed shared, and a `&mut self` method would
/// take all three exclusively.
fn khattit(
    saff: &mut Saff,
    khutut: &SilsilatKhutut,
    khiyarat: &KhiyaratTakhtit,
    nass: &str,
    hajm: f32,
    ard_mutah: Option<f32>,
    irtifa_mutah: Option<f32>,
) -> Natija<TakhtitNass> {
    saff.khattit(&TalabTakhtit {
        nass,
        khutut,
        hajm,
        ard_mutah,
        irtifa_mutah,
        nitaqat: &[],
        khiyarat,
    })
}

/// The pen's y that centres a layout of `irtifa` pixels in a box.
///
/// A line box is taller than the ink a recognizer measured, so a layout hung
/// from the box's top edge sits low and its descenders fall past the bottom of
/// the plate. Centring puts the Arabic where the eye expects the line it
/// replaces to be, and it is the only placement that stays right for both a box
/// taller than the layout and a box shorter than it.
const fn wasat_amudi(sunduq: MustatilBiksel, irtifa: f32) -> f32 {
    irtifa.mul_add(-0.5, madaa(sunduq.irtifa).mul_add(0.5, madaa(sunduq.aala)))
}

/// A surface dimension as a float.
///
/// `u32` to `f32` is exact below 2^24, which is four orders of magnitude past
/// any surface dimension that will exist. Written once so the lint is answered
/// in one place rather than at every call site.
const fn madaa(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and box dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}
