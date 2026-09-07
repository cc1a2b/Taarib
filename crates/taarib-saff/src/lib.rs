//! # صف تعريب — the Arabic text engine
//!
//! Logical-order text goes in; ordered visual lines of positioned glyphs come
//! out. That is the whole contract, and every other crate in this workspace is
//! downstream of it.
//!
//! A layout call carries the text, a font resource (validated bytes, shared and
//! reference-counted), a pixel size, an available width and an optional height,
//! style spans, and options — base direction, justification mode, feature set,
//! declared language. What comes back is visual lines of glyphs, each glyph
//! carrying its glyph id, x, y, advance, the cluster index back into the
//! logical text, and the id of the style span it belongs to. Nothing in that
//! output is a codepoint. A codepoint reaching a draw call would mean shaping
//! was skipped, and Decision 1 removes the possibility structurally: from the
//! layout stage onward there is no parameter anywhere that accepts one.
//!
//! Built in **Phase 1**, before anything that consumes it, because the whole
//! product rests on this crate being correct rather than convenient.
//!
//! ## The pipeline, in exactly this order
//!
//! Each stage is a module, each is independently callable, and none skips
//! ahead. The order is not stylistic: break opportunities are a property of
//! logical text, joining is a property of shaped runs, and elongation is a
//! property of joining, so computing any of them out of sequence produces an
//! answer that is wrong in a way that only shows up on real sentences.
//!
//! | module | stage |
//! | --- | --- |
//! | `nasq` | markup and placeholder extraction: Unity rich text, Unreal `<Style>`, `BBCode`, and every format placeholder dialect games actually ship, reduced to clean text plus a span table plus atomic replacements |
//! | `ittijah` | the Unicode Bidirectional Algorithm in full — paragraph level, embeddings, overrides, isolates, weak and neutral resolution, BD16 bracket pairs — plus `ittijah::tarteeb`, the L2 reorder applied per line after breaking |
//! | `taqtee` | UAX #14 line break opportunities over the *logical* text, with grapheme cluster and word boundaries for cursor movement and typewriter effects |
//! | `wasl` | shaping through HarfRust, one call per direction-and-style run, with `init`/`medi`/`fina`/`isol`/`rlig`/`calt`/`mark`/`mkmk`/`curs`/`ccmp` and `locl` driven by the declared language |
//! | `qiyas` | measurement and reflow from real shaped advances, plus the public metrics queries: string width, line height, ascent, descent, gap, cap height, x-height |
//! | `kashida` | justification by elongation, with candidates ranked from shaped joining behaviour and the run reshaped after distribution, never by inserting U+0640 and hoping |
//! | `tashkeel` | the diacritic guarantee that holds across every stage: marks are glyphs, their advances are zero, and they never break lines or absorb elongation |
//! | `rasm` | rasterization through skrifa: 8-bit coverage and signed distance fields, hinting off, gamma-correct, subpixel positions quantized so the cache stays finite |
//! | `khatt` | font resources: load from bytes, validate against Decision 6, expose families, styles, axes and named instances, hold the one shared byte buffer, resolve the fallback chain per run |
//! | `lugha` | script and language detection, normalization, digit policy, joining types, and the classical kashida ranking |
//! | `talab` | what a caller hands in: the request and the decisions recorded into a patch |
//! | `maqta` | what the stages hand one another: logical runs, shaped runs, joining information, break opportunities |
//! | `natija` | the layout result, shaped so it can be serialized into a patch, handed across the ABI, or walked into a mesh without transformation |
//! | `khata` | what can go wrong, in two sentences and one action |
//!
//! ## Why the boundaries are drawn this tightly
//!
//! This crate knows nothing about games, engines, launchers, graphics APIs, or
//! files. There is no `std::fs` and no `std::net` in it, no global state, no
//! logging sink of its own, and no `unsafe` outside two byte-cast helpers whose
//! invariants are written down and locally proven. Fonts arrive as bytes that
//! somebody else read. Diagnostics leave as returned values, not as log lines.
//!
//! Two things follow from that, and both of them are the point. First, the
//! crate compiles to `wasm32-unknown-unknown` unchanged, which is what makes
//! the RPG Maker and Electron adapters possible without a second
//! implementation of Arabic shaping in JavaScript. Second, it is testable,
//! reviewable and valuable entirely on its own — a constraint that keeps it
//! correct, because a text engine that can reach for a file or a log when it is
//! confused will eventually do so instead of returning a real answer.
//!
//! ## Hard constraints
//!
//! - No Unicode presentation forms are produced by any path, ever. Contextual
//!   forms, lam-alef, and every ligature come from the font's own `GSUB` and
//!   `GPOS`. When shaping fails, that is a reported error, not a licence to
//!   degrade (Decision 1).
//! - HarfRust is the only shaper and skrifa is the only rasterizer, and they
//!   read the same `read-fonts` stack, so a glyph id from shaping is by
//!   construction the same glyph id the outline comes from (Decisions 2 and 3).
//! - One immutable, reference-counted font byte buffer feeds shaping,
//!   rasterization and metrics. Nothing loads the same font twice into two
//!   parsers, and glyph identity is `(font identity, glyph id)` (Decision 4).
//! - Markup and placeholders never reach `ittijah` or `wasl`. A placeholder is
//!   an atom with a measured width; a tag is a span; neither becomes a glyph.
//! - Every public function returns `Natija<T>`. Nothing panics on malformed
//!   input — not on lone surrogates, unpaired isolates, unterminated markup, or
//!   a font that is lying about which tables it has.
//! - Presentation forms are read in exactly one place, in the normalization
//!   path, when a translator pastes text from a legacy source. They are
//!   decomposed to canonical characters immediately and never written back.
//!
//! ## Using it
//!
//! [`Saff`] is the engine. It owns the shaping caches and runs the stages in
//! order; everything else in the crate is reachable on its own for callers that
//! need one stage rather than all of them.
//!
//! ```no_run
//! use std::sync::Arc;
//! use taarib_saff::{Saff, SilsilatKhutut, MawridKhatt, TalabTakhtit, KhiyaratTakhtit};
//!
//! # fn main() -> taarib_usus::khata::Natija<()> {
//! let bayt = Arc::new(Vec::new());                       // the font's bytes
//! let khatt = Arc::new(MawridKhatt::jadeed(bayt, 0)?);   // validated on load
//! let khutut = SilsilatKhutut::wahid(khatt)?;
//! let khiyarat = KhiyaratTakhtit::default();
//!
//! let mut saff = Saff::jadeed();
//! let takhtit = saff.khattit(&TalabTakhtit {
//!     nass: "مرحبًا بالعالم",
//!     khutut: &khutut,
//!     hajm: 24.0,
//!     ard_mutah: Some(320.0),
//!     irtifa_mutah: None,
//!     nitaqat: &[],
//!     khiyarat: &khiyarat,
//! })?;
//!
//! for satr in &takhtit.sutur {
//!     for harf in takhtit.huruf_satr(satr) {
//!         let _ = (harf.muarrif, harf.s, harf.a);        // glyph id and where it goes
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod ittijah;
pub mod kashida;
pub mod khata;
pub mod khatt;
pub mod lugha;
pub mod maqta;
pub mod nasq;
pub mod natija;
pub mod qiyas;
pub mod rasm;
pub mod talab;
pub mod taqtee;
pub mod tashkeel;
pub mod wasl;

use taarib_usus::khata::Natija;

pub use crate::khata::{KhataKhatt, KhataSaff, SababNasq};
pub use crate::khatt::{HuwiyatKhatt, MawridKhatt, MihwarKhatt, QiyasatKhatt, SilsilatKhutut};
pub use crate::maqta::{
    DhuMustawa, FursatQat, HarfMashkul, Kitaba, MaqtaMantiqi, MaqtaMashkul, SifatWasl,
};
pub use crate::nasq::{KhiyaratNasq, LahjatNasq, NassNaqi};
pub use crate::natija::{Harf, MustatilNass, SatrMansuq, TakhtitNass, TaqreerTajawuz};
pub use crate::qiyas::QiyasNass;
pub use crate::rasm::{NamatRasm, Rassam, SurahHarf};
pub use crate::talab::{
    Dharra, Ittijah, IttijahAsas, KhiyaratTakhtit, LughaNass, Muhadhaha, NamatDabt, NitaqUslub,
    SifaIdafiya, SiyasatArqam, SiyasatTajawuz, SiyasatTashkeel, TalabTakhtit, Uslub,
};

/// The engine.
///
/// Holds the caches that make repeated layout cheap — the prepared shaper for
/// each font, which is the expensive part of shaping and the part a game pays
/// for every frame if nobody keeps it. Everything else the engine needs lives in
/// the request.
///
/// One `Saff` is not thread-safe to share, by design: it is a mutable cache, and
/// a lock around it on a render thread would cost more than the cache saves.
/// Hold one per thread, or one per context across the ABI in [`taarib-jisr`],
/// where the layout cache proper lives.
///
/// [`taarib-jisr`]: https://github.com/cc1a2b/taarib
#[derive(Debug)]
pub struct Saff {
    makhzan: wasl::MakhzanTashkeel,
}

impl Default for Saff {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl Saff {
    /// A new engine with empty caches.
    #[must_use]
    pub fn jadeed() -> Self {
        Self {
            makhzan: wasl::MakhzanTashkeel::jadeed(),
        }
    }

    /// Empties the shaping caches, for a caller that has finished with a set of
    /// fonts and wants the memory back.
    pub fn amsah(&mut self) {
        self.makhzan.amsah();
    }

    /// How many fonts are currently prepared.
    #[must_use]
    pub fn adad_khutut(&self) -> usize {
        self.makhzan.adad()
    }

    /// The shaping cache, for callers that drive the stages themselves.
    pub const fn makhzan(&mut self) -> &mut wasl::MakhzanTashkeel {
        &mut self.makhzan
    }

    /// Lays text out.
    ///
    /// Runs every stage in order: the diacritic and digit policies, the
    /// bidirectional analysis, run splitting, break opportunities, shaping,
    /// measurement and reflow, reordering, justification, and positioning.
    ///
    /// The cluster index on every returned glyph refers to **the text you passed
    /// in**, not to the text after the policies rewrote it. That remapping is
    /// done here rather than left to the caller, because a caret that lands in
    /// the wrong place after diacritics were stripped is a defect nobody
    /// attributes to the digit policy that caused it.
    ///
    /// # Errors
    ///
    /// Whatever any stage reports: an unusable width or size, a style span that
    /// does not fit its text, markup nested past the bidirectional algorithm's
    /// limit, or a font that cannot shape the run it was given.
    pub fn khattit(&mut self, talab: &TalabTakhtit<'_>) -> Natija<TakhtitNass> {
        // The overwhelmingly common case is a request whose policies change
        // nothing, and preparing it would copy the string and the span list for
        // no reason. Skip straight to the pipeline.
        if !yahtaj_tahdeer(talab.khiyarat) {
            return self.khattit_mubashir(talab);
        }
        let muhaddar = hayyi(talab.nass, talab.nitaqat, talab.khiyarat);
        let mut takhtit = self.khattit_mubashir(&TalabTakhtit {
            nass: &muhaddar.nass,
            nitaqat: &muhaddar.nitaqat,
            ..*talab
        })?;
        if !muhaddar.bila_taghyeer() {
            muhaddar.aid_almawaqi(&mut takhtit);
        }
        Ok(takhtit)
    }

    /// Lays text out into an existing result, reusing its allocations.
    ///
    /// The layout is built in the engine's own buffers and moved into `hadaf`'s,
    /// so a caller that lays out every frame stops paying the allocator once its
    /// buffers have grown to the size its text needs. The genuinely
    /// allocation-free path — where a repeated string is not laid out at all —
    /// is the layout cache in `taarib-jisr`, which this is the foundation for.
    ///
    /// # Errors
    ///
    /// As [`Saff::khattit`].
    pub fn khattit_fi(&mut self, talab: &TalabTakhtit<'_>, hadaf: &mut TakhtitNass) -> Natija<()> {
        let takhtit = self.khattit(talab)?;
        hadaf.amsah();
        hadaf.huruf.extend_from_slice(&takhtit.huruf);
        hadaf.sutur.extend_from_slice(&takhtit.sutur);
        hadaf.ard = takhtit.ard;
        hadaf.irtifa = takhtit.irtifa;
        hadaf.ittijah = takhtit.ittijah;
        hadaf.hajm = takhtit.hajm;
        hadaf.tajawuz = takhtit.tajawuz;
        hadaf.maqsus = takhtit.maqsus;
        Ok(())
    }

    /// Measures text without positioning glyphs, applying the same policies and
    /// running the same pipeline.
    ///
    /// # Errors
    ///
    /// As [`Saff::khattit`].
    pub fn qis(&mut self, talab: &TalabTakhtit<'_>) -> Natija<QiyasNass> {
        if !yahtaj_tahdeer(talab.khiyarat) {
            return qiyas::qis_bi_makhzan(talab, &mut self.makhzan);
        }
        let muhaddar = hayyi(talab.nass, talab.nitaqat, talab.khiyarat);
        let talab_muhaddar = TalabTakhtit {
            nass: &muhaddar.nass,
            nitaqat: &muhaddar.nitaqat,
            ..*talab
        };
        qiyas::qis_bi_makhzan(&talab_muhaddar, &mut self.makhzan)
    }

    /// Lays out text that still carries its markup.
    ///
    /// Extracts the markup and the placeholders first, then lays out what is
    /// left. The returned cluster indices refer to the **clean** text in the
    /// returned [`NassNaqi`], which is also what the style spans are measured
    /// against — so a caller mapping a glyph back to a character maps it into
    /// `naqi.nass`, and reaches the original through
    /// [`nasq::aid_binaa`] when it needs the raw bytes back.
    ///
    /// # Errors
    ///
    /// Whatever markup extraction reports for malformed input, plus whatever
    /// [`Saff::khattit`] reports.
    pub fn khattit_khaam(
        &mut self,
        khaam: &str,
        khiyarat_nasq: &KhiyaratNasq,
        khutut: &SilsilatKhutut,
        hajm: f32,
        ard_mutah: Option<f32>,
        khiyarat: &KhiyaratTakhtit,
    ) -> Natija<(NassNaqi, TakhtitNass)> {
        let naqi = nasq::istakhrij(khaam, khiyarat_nasq)?;
        let takhtit = self.khattit(&TalabTakhtit {
            nass: &naqi.nass,
            khutut,
            hajm,
            ard_mutah,
            irtifa_mutah: None,
            nitaqat: &naqi.nitaqat,
            khiyarat,
        })?;
        Ok((naqi, takhtit))
    }

    /// The pipeline itself, over text the policies have already been applied to
    /// — or over text that needed none.
    fn khattit_mubashir(&mut self, talab: &TalabTakhtit<'_>) -> Natija<TakhtitNass> {
        qiyas::tahaqquq(talab)?;

        let tahleel = ittijah::TahleelIttijah::jadeed(talab.nass, talab.khiyarat.ittijah)?;
        if talab.nass.is_empty() {
            return Ok(TakhtitNass::farigh(tahleel.ittijah_asas(), talab.hajm));
        }

        let lugha = lugha_almatlub(talab.nass, talab.khiyarat);
        let mantiqiya = ittijah::qassim(
            talab.nass,
            &tahleel,
            talab.nitaqat,
            talab.khutut,
            lugha,
            talab.hajm,
        )?;
        let mashkula = wasl::shakkil_maqati_bi_asalib(
            talab.nass,
            &mantiqiya,
            talab.nitaqat,
            talab.khutut,
            talab.khiyarat,
            &mut self.makhzan,
        )?;
        let furas = taqtee::furas_qat(talab.nass, lugha);
        qiyas::rattib_sutur(
            talab.nass,
            mashkula,
            &furas,
            talab,
            &tahleel,
            &mut self.makhzan,
        )
    }
}

/// Resolves the declared language, detecting it from the text when the caller
/// asked for detection.
///
/// Kept in one place because two stages need the answer — shaping keys `locl` on
/// it and segmentation tailors on it — and two independent detections that
/// disagree would shape a line in one language and break it in another.
#[must_use]
pub fn lugha_almatlub(nass: &str, khiyarat: &KhiyaratTakhtit) -> LughaNass {
    match khiyarat.lugha {
        LughaNass::Tilqai => lugha::iktashif_lugha(nass),
        muhaddada => muhaddada,
    }
}

/// Text after the diacritic and digit policies, with everything needed to map
/// offsets back to what the caller handed in.
#[derive(Debug, Clone)]
pub struct NassMuhaddar {
    /// The prepared text.
    pub nass: String,
    /// The caller's style spans, remapped onto the prepared text. Spans that
    /// covered only removed characters are dropped, because a span of zero
    /// length over text that no longer exists cannot style anything.
    pub nitaqat: Vec<NitaqUslub>,
    /// Breakpoints where the offset delta changes, as `(original, prepared)`.
    /// Empty means the text was not changed at all.
    nuqat: Vec<(u32, u32)>,
}

impl NassMuhaddar {
    /// Whether the policies left the text exactly as it was.
    #[must_use]
    pub const fn bila_taghyeer(&self) -> bool {
        self.nuqat.is_empty()
    }

    /// Maps an offset in the caller's text to the prepared text.
    #[must_use]
    pub fn ila_muhaddar(&self, asli: u32) -> u32 {
        // The breakpoints are sorted ascending in both columns, so one binary
        // search serves either direction. A linear scan would be simpler and
        // would also be quadratic on a paragraph of numerals, which is exactly
        // the text this map exists for.
        let fahras = self
            .nuqat
            .partition_point(|(min_asli, _)| *min_asli <= asli);
        match fahras.checked_sub(1).and_then(|i| self.nuqat.get(i)) {
            Some((min_asli, min_muhaddar)) => {
                min_muhaddar.saturating_add(asli.saturating_sub(*min_asli))
            },
            None => asli,
        }
    }

    /// Maps an offset in the prepared text back to the caller's text.
    ///
    /// An offset inside a character that the policies removed maps to the start
    /// of the segment it fell in, which is where a caret belongs when the
    /// character under it no longer exists.
    #[must_use]
    pub fn ila_asli(&self, muhaddar: u32) -> u32 {
        let fahras = self
            .nuqat
            .partition_point(|(_, min_muhaddar)| *min_muhaddar <= muhaddar);
        match fahras.checked_sub(1).and_then(|i| self.nuqat.get(i)) {
            Some((min_asli, min_muhaddar)) => {
                min_asli.saturating_add(muhaddar.saturating_sub(*min_muhaddar))
            },
            None => muhaddar,
        }
    }

    /// Rewrites every cluster index and every logical range in a finished layout
    /// so they refer to the caller's text rather than to the prepared text.
    fn aid_almawaqi(&self, takhtit: &mut TakhtitNass) {
        for harf in &mut takhtit.huruf {
            harf.anqud = self.ila_asli(harf.anqud);
        }
        for satr in &mut takhtit.sutur {
            satr.mantiqi = self.ila_asli(satr.mantiqi.start)..self.ila_asli(satr.mantiqi.end);
        }
    }
}

/// Whether the request's policies would change the text at all.
///
/// Checked before preparing rather than after, because preparing copies the
/// string and the span list, and the answer is no for most of the strings this
/// engine will ever lay out.
#[must_use]
pub const fn yahtaj_tahdeer(khiyarat: &KhiyaratTakhtit) -> bool {
    let yahdhif = matches!(khiyarat.tashkeel, SiyasatTashkeel::Hadhf)
        || (matches!(khiyarat.tashkeel, SiyasatTashkeel::IbqaFilHiwar) && !khiyarat.hiwar);
    yahdhif || !matches!(khiyarat.arqam, SiyasatArqam::KamaHiya)
}

/// Applies the diacritic and digit policies, remapping the style spans.
///
/// Both policies change the text's length: stripping a `fatha` removes two
/// bytes, and mapping `5` to `٥` turns one byte into two. Every byte offset the
/// caller holds — every style span boundary — has to move with it, and every
/// offset the engine produces has to move back. Doing that here, once, is why
/// no later stage has to know the policies exist.
#[must_use]
pub fn hayyi(nass: &str, nitaqat: &[NitaqUslub], khiyarat: &KhiyaratTakhtit) -> NassMuhaddar {
    let yahdhif = matches!(khiyarat.tashkeel, SiyasatTashkeel::Hadhf)
        || (matches!(khiyarat.tashkeel, SiyasatTashkeel::IbqaFilHiwar) && !khiyarat.hiwar);
    let yubaddil = !matches!(khiyarat.arqam, SiyasatArqam::KamaHiya);

    if !yahtaj_tahdeer(khiyarat) {
        return NassMuhaddar {
            nass: nass.to_owned(),
            nitaqat: nitaqat.to_vec(),
            nuqat: Vec::new(),
        };
    }

    let mut mabni = String::with_capacity(nass.len());
    let mut nuqat: Vec<(u32, u32)> = Vec::new();
    for (mawqi, harf) in nass.char_indices() {
        let tul_asli = harf.len_utf8();
        let asli = u32::try_from(mawqi).unwrap_or(u32::MAX);
        let muhaddar = u32::try_from(mabni.len()).unwrap_or(u32::MAX);

        if yahdhif && tashkeel::huwa_alama(harf) {
            nuqat.push((
                asli.saturating_add(u32::try_from(tul_asli).unwrap_or(0)),
                muhaddar,
            ));
            continue;
        }

        let makhruj = if yubaddil {
            raqm_badeel(harf, khiyarat.arqam)
        } else {
            harf
        };
        mabni.push(makhruj);
        if makhruj.len_utf8() != tul_asli {
            nuqat.push((
                asli.saturating_add(u32::try_from(tul_asli).unwrap_or(0)),
                muhaddar.saturating_add(u32::try_from(makhruj.len_utf8()).unwrap_or(0)),
            ));
        }
    }

    let muhaddar = NassMuhaddar {
        nass: mabni,
        nitaqat: Vec::new(),
        nuqat,
    };
    let manqula = nitaqat
        .iter()
        .filter_map(|nitaq| {
            let bidaya = muhaddar.ila_muhaddar(nitaq.bidaya);
            let nihaya = muhaddar.ila_muhaddar(nitaq.nihaya());
            // An atom keeps its span even at zero length: its width is what the
            // line still has to make room for.
            if nihaya <= bidaya && nitaq.uslub.dharra.is_none() {
                return None;
            }
            Some(NitaqUslub {
                tul: nihaya.saturating_sub(bidaya),
                bidaya,
                ..*nitaq
            })
        })
        .collect();

    NassMuhaddar {
        nitaqat: manqula,
        ..muhaddar
    }
}

/// Maps one digit into the requested system, leaving everything else alone.
///
/// Works across all three systems rather than only from ASCII, because a
/// translation may already contain Arabic-Indic digits that a Persian patch
/// needs as Eastern Arabic-Indic, and a patch that maps only from ASCII would
/// leave those untouched and produce a line with two digit systems in it.
fn raqm_badeel(harf: char, siyasa: SiyasatArqam) -> char {
    let qeema = match harf {
        '0'..='9' => harf as u32 - '0' as u32,
        '\u{0660}'..='\u{0669}' => harf as u32 - 0x0660,
        '\u{06F0}'..='\u{06F9}' => harf as u32 - 0x06F0,
        _ => return harf,
    };
    let asas = match siyasa {
        SiyasatArqam::KamaHiya => return harf,
        SiyasatArqam::Latini => '0' as u32,
        SiyasatArqam::Arabi => 0x0660,
        SiyasatArqam::Farisi => 0x06F0,
    };
    char::from_u32(asas + qeema).unwrap_or(harf)
}
