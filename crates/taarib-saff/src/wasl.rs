//! الوصل — shaping: where characters stop being characters.
//!
//! This is the stage Decision 1 exists for. Every contextual form, every
//! ligature including the mandatory lam-alef, every diacritic placement and
//! every cursive attachment in this product is produced here, by HarfRust,
//! reading the font's own `GSUB` and `GPOS`. There is no table in this file
//! mapping a codepoint to a presentation form, no fallback that draws isolated
//! letters when a font disappoints, and no path by which a codepoint survives
//! past this module: what leaves is [`HarfMashkul`], and a `HarfMashkul` has no
//! field that could hold a character.
//!
//! ## What this stage is handed, and what it refuses to work out for itself
//!
//! A [`MaqtaMantiqi`] arrives already decided. Its direction and its embedding
//! level came from the bidirectional algorithm, its script from character
//! properties, its language from the patch, its font from the fallback chain,
//! its size from the style span. This module reads all of that and infers none
//! of it. That restraint is the point: a second, independent guess at direction
//! or script inside the shaper would agree with the first one on pure Arabic and
//! pure Latin, and disagree exactly on the mixed text the product exists to get
//! right — a bug that passes every simple test and fails every real sentence.
//!
//! Two things this stage deliberately does not do, so that nobody does them
//! twice:
//!
//! - **Diacritic policy.** [`crate::talab::SiyasatTashkeel`] is applied during
//!   normalization, before the clean text exists. Stripping marks here would
//!   move every byte offset after the first mark, and every cluster index this
//!   module emits would then point into a string no other stage holds.
//! - **Letter and word spacing.** `tabaud_ahruf` and `tabaud_kalimat` are
//!   applied by `qiyas`, on the line, after justification has decided how much
//!   room there is. Baking them into shaped advances would corrupt the width
//!   `kashida` reshapes from, and in Arabic letter spacing is destructive —
//!   it visibly tears joined letters apart — so it belongs where the policy is
//!   applied knowingly rather than as a silent constant added to every advance.
//!
//! ## What leaves here is ink, or it does not leave
//!
//! A character that the layout consumes rather than draws — a line separator, a
//! paragraph separator, a bidirectional control, the byte-order mark — produces
//! no glyph. [`crate::khatt::ghayr_marii`] is the engine's one definition of
//! that set, and this is the last stage that can apply it: after this file a
//! glyph carries no character, so no stage downstream could tell an instruction
//! apart from a letter even if it wanted to. The adapters that draw a patch are
//! written in C#, JavaScript, Python and Ruby as well as in Rust, and a rule
//! left for each of them to remember is a rule one of them forgets — the
//! symptom being the empty `.notdef` box a font returns for a newline, sitting
//! at the end of every line of a patched paragraph and alone on every blank
//! line between them. See [`ahdhif_ghayr_almarii`].
//!
//! What *is* text and still comes back as `.notdef` stays. That is a character
//! no font in the chain covers — real information, and the only evidence of it
//! that exists — so this stage keeps it and `qiyas` decides: the width is
//! reserved, nothing is drawn in it, and the layout carries
//! [`crate::natija::TaghtiyaNaqisa`] naming how many there were and where the
//! first one is.
//!
//! ## Units
//!
//! HarfRust is asked for its output in **font units** — no scale is set on the
//! shape call — and this module converts to **pixels** once, here, with the
//! font's units-per-em and the run's own size. Everything downstream of this
//! file is in pixels and nothing downstream needs to know what a font unit is.
//! The conversion is done in floating point rather than by handing HarfRust an
//! integer scale, because `rasm` positions glyphs at subpixel offsets and an
//! integer-scaled shaper would have rounded that information away before this
//! stage ever saw it.
//!
//! Vertical offsets keep the font's sign convention: positive `izaha_a` is
//! **upward**, which is what skrifa's outlines use as well, so a mark's offset
//! and the outline it is drawn from agree without a correction. A renderer whose
//! screen coordinates grow downward negates once, at the draw call.
//!
//! ## Clusters
//!
//! Every glyph carries `anqud`: the byte offset, **into the whole clean text**,
//! of the first character of the cluster it came from. Not into the run — into
//! the text, so that a run boundary never shifts the meaning of an index. A
//! ligature covers several source characters and reports one cluster; several
//! marks share the cluster of the base they attach to; nothing is ever left
//! without one.
//!
//! Downstream, that field is the only bridge back from a drawn glyph to the
//! string it came from. It is what makes a caret land between graphemes in a
//! right-to-left selection, what lets a typewriter effect reveal a joined word
//! one letter at a time without unjoining it, and what lets the review console
//! jump from a rendered line to the source line. An index that is off by a run
//! start is not a rounding error; it puts the caret in a different word.
//!
//! ## Joining, and why it cannot be recovered later
//!
//! [`SifatWasl`] is the one thing in the output that no later stage could
//! reconstruct. A glyph identifier does not say whether it is joined to its
//! neighbour; a positive advance does not say whether the joint between two
//! glyphs is one the script elongates at. Both facts live in the *characters*
//! the cluster came from, and this is the last stage that still has them. So
//! this stage resolves every cluster back to its source characters, asks
//! [`crate::lugha::naw_wasl`] how each one joins and
//! [`crate::lugha::rutbat_kashida`] how good a joint is, and writes the answer
//! into every glyph before the characters go out of scope forever.
//!
//! Marks are transparent to that analysis. A `shadda` between two joined letters
//! does not break the join, and treating it as if it did is how a renderer ends
//! up refusing to elongate a vocalised word — or, worse, elongating inside one.

use std::collections::hash_map::Entry;
use std::sync::Arc;

use harfrust::{
    BufferClusterLevel, BufferFlags, Direction, Feature, FontRef, GlyphInfo, Language, Script,
    ShapeOptions, ShaperData, ShaperInstance, Tag, UnicodeBuffer, Variation,
};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use taarib_usus::khata::{Khata, Natija};

use crate::khata::{KhataKhatt, KhataSaff};
use crate::khatt::{HuwiyatKhatt, MawridKhatt, SilsilatKhutut, ghayr_marii};
use crate::lugha::{NawWasl, naw_wasl, rutbat_kashida};
use crate::maqta::{HarfMashkul, Kitaba, MaqtaMantiqi, MaqtaMashkul, SifatWasl};
use crate::talab::{Ittijah, KhiyaratTakhtit, LughaNass, NitaqUslub, SifaIdafiya, Uslub};

/// The `GDEF` glyph class that means "combining mark".
///
/// The same number HarfRust reads to decide that a glyph is positioned onto a
/// base rather than advancing the pen, and therefore the same answer.
const SINF_ALAMA: u16 = 3;

/// How many characters of context are handed to the shaper on each side of a
/// run. HarfRust stores five and ignores the rest, so asking for more would be
/// work thrown away.
const TUL_SIYAQ: usize = 5;

/// The feature set Taarib declares for a cursively joining script, in the order
/// shaping thinks in. Documented group by group on [`sifat_alkitaba`].
const SIFAT_WASL_KAMILA: [[u8; 4]; 15] = [
    *b"ccmp", *b"locl", *b"isol", *b"fina", *b"medi", *b"init", *b"rlig", *b"rclt", *b"calt",
    *b"liga", *b"mset", *b"curs", *b"kern", *b"mark", *b"mkmk",
];

/// The feature set for scripts that do not join cursively.
///
/// The same shape as the Arabic set with the four joining features and the
/// cursive attachment removed, because a Latin font has no `init` and a `curs`
/// lookup it does not have costs a plan entry to discover. `clig` replaces
/// `mset`: Latin's contextual ligatures are a real thing and Arabic's are
/// handled by `rlig` and `rclt`.
const SIFAT_MUSTAQIMA: [[u8; 4]; 10] = [
    *b"ccmp", *b"locl", *b"rlig", *b"rclt", *b"calt", *b"liga", *b"clig", *b"kern", *b"mark",
    *b"mkmk",
];

/// The features whose masks HarfRust's Arabic shaper owns per glyph.
///
/// These are the only features in the declared set that a caller must never
/// switch **on** through the user-feature channel. See [`sifat_harfrust`].
const SIFAT_YAMLIKUHA_ALMUSHAKKIL: [[u8; 4]; 7] = [
    *b"isol", *b"fina", *b"fin2", *b"fin3", *b"medi", *b"med2", *b"init",
];

/// The `locl` tag, needed by name because its presence depends on the language.
const WASM_LOCL: [u8; 4] = *b"locl";

/// The variable-font weight axis.
const MIHWAR_WAZN: [u8; 4] = *b"wght";

/// The variable-font italic axis: a binary switch between upright and italic.
const MIHWAR_MAIL: [u8; 4] = *b"ital";

/// The variable-font slant axis: an angle in counter-clockwise degrees from
/// vertical, so a slanted face lives at the *lower* end of the range.
const MIHWAR_MAYL: [u8; 4] = *b"slnt";

/// A set of glyph identifiers, held as bits.
///
/// Built once per font so that asking "is this glyph a mark?" costs a shift and
/// a mask instead of a binary search through `GDEF`'s class definition on every
/// glyph of every run of every frame.
#[derive(Debug)]
struct MajmuatAshkal {
    bitat: Box<[u64]>,
    hadd: u32,
}

impl MajmuatAshkal {
    /// An empty set able to hold identifiers below `hadd`.
    fn jadida(hadd: u32) -> Self {
        let tul = usize::try_from(hadd).unwrap_or(usize::MAX).div_ceil(64);
        Self {
            bitat: vec![0_u64; tul].into_boxed_slice(),
            hadd,
        }
    }

    /// Adds an identifier, ignoring one that is out of range — which means the
    /// font's own tables disagree with its own `maxp`, and is not this stage's
    /// failure to report.
    fn dai(&mut self, muarrif: u32) {
        if muarrif >= self.hadd {
            return;
        }
        let Ok(mawqi) = usize::try_from(muarrif >> 6) else {
            return;
        };
        if let Some(kalima) = self.bitat.get_mut(mawqi) {
            *kalima |= 1_u64 << (muarrif & 63);
        }
    }

    /// Whether an identifier is in the set.
    fn yahwi(&self, muarrif: u32) -> bool {
        if muarrif >= self.hadd {
            return false;
        }
        usize::try_from(muarrif >> 6)
            .ok()
            .and_then(|mawqi| self.bitat.get(mawqi))
            .is_some_and(|kalima| kalima & (1_u64 << (muarrif & 63)) != 0)
    }
}

/// One shaping cluster, resolved back to the characters that produced it.
///
/// A cluster is the unit that maps glyphs to text: a ligature covers several
/// characters and reports one cluster, several marks share the cluster of the
/// base they attach to. Everything joining-related is decided per cluster and
/// then written onto that cluster's glyphs, because a joint exists *between
/// clusters*, never inside one.
#[derive(Debug, Clone, Copy)]
struct Anqud {
    /// Byte offset of the cluster's first character in the clean text.
    bidaya: u32,
    /// The first character that is not a mark, which decides how this cluster
    /// joins to what precedes it. `None` when the cluster is nothing but marks.
    awwal: Option<char>,
    /// The last character that is not a mark, which decides how it joins to
    /// what follows.
    akhir: Option<char>,
    /// Joined to the logically previous non-transparent cluster.
    wasl_qabl: bool,
    /// Joined to the logically next one.
    wasl_baad: bool,
    /// Elongation rank of the joint on the logically preceding side.
    rutba_qabl: u8,
    /// Elongation rank of the joint on the logically following side.
    rutba_baad: u8,
}

/// A font prepared for shaping.
///
/// Holds the parsed layout caches HarfRust needs — coverage digests, lookup
/// indices, the character map cache — which is the expensive half of shaping and
/// the half that depends only on the font. The cheap half, a
/// [`harfrust::Shaper`], borrows the font bytes and is therefore rebuilt on each
/// call; it is a handful of table lookups and cannot be stored here without
/// making this type self-referential.
///
/// One of these per font identity, held by [`MakhzanTashkeel`]. Decision 4 is
/// what makes that identity trustworthy: the font is its bytes, so two
/// resources built from the same buffer are the same font no matter what path
/// or family name they were found by.
pub struct MushakkilKhatt {
    khatt: Arc<MawridKhatt>,
    bayanat: ShaperData,
    huwiya: HuwiyatKhatt,
    wahdat: f32,
    alamat: MajmuatAshkal,
}

impl core::fmt::Debug for MushakkilKhatt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MushakkilKhatt")
            .field("wahdat", &self.wahdat)
            .field("alamat", &self.alamat.hadd)
            .finish_non_exhaustive()
    }
}

impl MushakkilKhatt {
    /// Prepares a font for shaping.
    ///
    /// Parses the layout tables once and records which glyphs the font's `GDEF`
    /// calls marks, so that neither has to be done again for this font.
    ///
    /// # Errors
    ///
    /// Returns whatever [`MawridKhatt::khatt`] reports when the bytes cannot be
    /// parsed, and [`KhataKhatt::JadwalMafqud`] when the font declares a
    /// units-per-em of zero — a `head` table that cannot be used, which would
    /// make every advance in every run either zero or infinite.
    pub fn jadeed(khatt: &Arc<MawridKhatt>) -> Natija<Self> {
        let wahdat_kham = khatt.qiyasat(1.0).wahdat;
        if wahdat_kham == 0 {
            return Err(Khata::min_tafsir(&KhataKhatt::JadwalMafqud {
                jadwal: "head",
            }));
        }

        // The FontRef borrows `khatt`, and ShaperData does not: it copies out
        // the ranges and caches it needs. That is what lets this type hold the
        // prepared data and the Arc together without being self-referential.
        let bayanat = {
            let font = khatt.khatt()?;
            ShaperData::new(&font)
        };
        let alamat = alamat_alkhatt(khatt)?;

        Ok(Self {
            huwiya: khatt.huwiya(),
            khatt: Arc::clone(khatt),
            bayanat,
            wahdat: f32::from(wahdat_kham),
            alamat,
        })
    }

    /// The identity of the font this shaper was built for.
    #[must_use]
    pub const fn huwiya(&self) -> HuwiyatKhatt {
        self.huwiya
    }

    /// The font resource, shared with rasterization and metrics.
    ///
    /// Decision 4 in one method: the same buffer that shaped is the buffer the
    /// outline comes from, so a glyph identifier means one thing in this
    /// product.
    #[must_use]
    pub const fn mawrid(&self) -> &Arc<MawridKhatt> {
        &self.khatt
    }

    /// Shapes one run.
    ///
    /// `nass` is the whole clean text, not the run's slice, because cluster
    /// indices are byte offsets into the whole text and because the characters
    /// on either side of the run are handed to the shaper as context: a run
    /// boundary that falls inside a word — which a bold span in the middle of a
    /// word produces — would otherwise sever the join there, and no later stage
    /// could put it back.
    ///
    /// A run carrying a [`crate::talab::Dharra`] is never shaped. See
    /// [`MushakkilKhatt::shakkil_bi_uslub`] for the rest of the contract.
    ///
    /// # Errors
    ///
    /// As [`MushakkilKhatt::shakkil_bi_uslub`].
    pub fn shakkil(
        &self,
        nass: &str,
        maqta: &MaqtaMantiqi,
        khiyarat: &KhiyaratTakhtit,
    ) -> Natija<MaqtaMashkul> {
        self.shakkil_bi_uslub(nass, maqta, None, khiyarat)
    }

    /// Shapes one run with its resolved style span in hand.
    ///
    /// [`MaqtaMantiqi`] carries a style span *identifier*, not the span itself,
    /// so a run alone cannot say what weight it wants. A caller that owns the
    /// span table resolves it and calls this; [`MushakkilKhatt::shakkil`] is the
    /// same call for text whose style needs no variation. Only the fields that
    /// change letterforms are read — `wazn` and `maail`. Colour, offset and
    /// spacing are the caller's to carry through, and this stage never sees
    /// them.
    ///
    /// # Errors
    ///
    /// - [`KhataSaff::HajmGhayrSalih`] when the run's size is not a positive,
    ///   finite number of pixels.
    /// - [`KhataSaff::NitaqKharij`] when the run's byte range falls outside the
    ///   clean text, and [`KhataSaff::HaddNitaqTalif`] when one of its ends
    ///   splits a UTF-8 sequence. Either means the run splitter and the text
    ///   have gone out of step, and shaping half a character would hide that.
    /// - [`KhataSaff::TashkeelFashil`] when a non-empty, non-atom run produces
    ///   no glyphs at all, and when it produces nothing but `.notdef` for
    ///   characters the font's own character map claims to cover. Both are
    ///   Decision 6's validation failing one stage late: a font that passed the
    ///   table check and still cannot shape this text. Returning an empty run
    ///   instead would delete the text silently, which is the failure mode this
    ///   product exists to replace.
    pub fn shakkil_bi_uslub(
        &self,
        nass: &str,
        maqta: &MaqtaMantiqi,
        uslub: Option<&Uslub>,
        khiyarat: &KhiyaratTakhtit,
    ) -> Natija<MaqtaMashkul> {
        if !(maqta.hajm.is_finite() && maqta.hajm > 0.0) {
            return Err(Khata::min_tafsir(&KhataSaff::HajmGhayrSalih {
                hajm: maqta.hajm,
            }));
        }

        // An atom occupies a position and is never shaped. It still takes part
        // in breaking and justification as one indivisible unit, which is
        // exactly why it is given a width here rather than being dropped: a
        // placeholder that lost its width would let a line that overflows in the
        // game measure as if it fit.
        if let Some(dharra) = maqta.dharra {
            let mut mashkul = MaqtaMashkul::min_asl(maqta.clone());
            mashkul.ard = dharra.ard;
            mashkul.suud = (dharra.asas + dharra.irtifa).max(0.0);
            mashkul.hubut = (-dharra.asas).max(0.0);
            return Ok(mashkul);
        }

        let nass_maqta = qita_almaqta(nass, maqta)?;
        if nass_maqta.is_empty() {
            return Ok(MaqtaMashkul::min_asl(maqta.clone()));
        }

        let lugha = lugha_almaqta(maqta, khiyarat);
        let sifat = sifat_alkitaba(maqta.kitaba, lugha, &khiyarat.sifat);
        let sifat_hr = sifat_harfrust(&sifat);

        let font = self.khatt.khatt()?;
        // Decision 3 and Decision 4 meeting: these coordinates are normalized
        // against the same `fvar`/`avar` the rasterizer will normalize against,
        // out of the same byte buffer. `rasm` MUST apply the identical
        // coordinates when it resolves the outline, or the advance measured here
        // and the outline drawn there belong to two different instances of the
        // font and every joined word drifts apart by a fraction of a pixel that
        // accumulates across a line.
        let instans = uslub.and_then(|u| self.instans(&font, u));
        let mushakkil_hr = self
            .bayanat
            .shaper(&font)
            .instance(instans.as_ref())
            .build();

        let mut mahfaza = UnicodeBuffer::new();
        mahfaza.set_direction(ittijah_harfrust(maqta.ittijah));
        if let Some(kitaba) = kitaba_harfrust(maqta.kitaba) {
            mahfaza.set_script(kitaba);
        }
        if let Some(wasm) = lugha_harfrust(lugha) {
            mahfaza.set_language(wasm);
        }
        mahfaza.set_flags(alamat_almahfaza(nass, maqta));
        // The default already, but stated: a cluster must never be allowed to
        // split a grapheme, or a caret could land between a base and its own
        // diacritic.
        mahfaza.set_cluster_level(BufferClusterLevel::MonotoneGraphemes);

        // Order matters and is not obvious: `add` clears the post-context, so
        // the trailing context must be set after the last character goes in.
        mahfaza.set_pre_context(siyaq_qabl(nass, maqta.nitaq.start));
        let bidaya = maqta.nitaq.start;
        for (izaha, harf) in nass_maqta.char_indices() {
            // Clusters are absolute byte offsets into the whole clean text, not
            // into the run. Downstream they are what maps a caret, a selection
            // and a typewriter effect back to the logical string, and a
            // run-relative offset would map them into the wrong word.
            let mawqi = u32::try_from(izaha)
                .unwrap_or(u32::MAX)
                .saturating_add(bidaya);
            mahfaza.add(harf, mawqi);
        }
        mahfaza.set_post_context(siyaq_baad(nass, maqta.nitaq.end));

        // No scale is set: the output is in font units and is converted to
        // pixels below, once.
        let khiyarat_hr = ShapeOptions::new().features(&sifat_hr);
        let natija = mushakkil_hr.shape(mahfaza, khiyarat_hr);

        let mawaqi = natija.glyph_positions();
        let mawsufat = natija.glyph_infos();
        if mawsufat.is_empty() {
            return Err(fashal_attashkeel(maqta, None));
        }
        if mawsufat.iter().all(|wasf| wasf.glyph_id == 0)
            && nass_maqta
                .chars()
                .any(|harf| self.khatt.muarrif(harf).is_some_and(|m| m != 0))
        {
            return Err(fashal_attashkeel(
                maqta,
                Some("the font's character map covers this text but shaping produced only .notdef"),
            ));
        }

        let miqyas = maqta.hajm / self.wahdat;
        let mut huruf: Vec<HarfMashkul> = Vec::with_capacity(mawsufat.len());
        for (wasf, mawdi) in mawsufat.iter().zip(mawaqi.iter()) {
            let alama = self.alamat.yahwi(wasf.glyph_id);
            huruf.push(HarfMashkul {
                muarrif: wasf.glyph_id,
                // The tashkeel guarantee made structural rather than trusted: a
                // mark contributes nothing to width, ever. HarfRust already
                // zeroes mark advances by `GDEF`, so this normally changes
                // nothing — it exists so the invariant holds even for a font
                // whose tables and whose metrics disagree.
                taqaddum_s: if alama {
                    0.0
                } else {
                    kasr(mawdi.x_advance) * miqyas
                },
                taqaddum_a: if alama {
                    0.0
                } else {
                    kasr(mawdi.y_advance) * miqyas
                },
                // Offsets are never zeroed. A mark's position is what `GPOS`
                // mark attachment resolved it to, and nothing here second-guesses
                // it: no synthesised placement, no stacking by hand. That is
                // what Decision 1 means at this stage.
                izaha_s: kasr(mawdi.x_offset) * miqyas,
                izaha_a: kasr(mawdi.y_offset) * miqyas,
                anqud: wasf.cluster,
                alama,
                wasl: SifatWasl::default(),
            });
        }

        if maqta.kitaba.tasil() {
            asil_alwasl(nass, maqta, mawsufat, &mut huruf);
        }

        // Ordered after joining analysis on purpose: until here the glyph array
        // and the shaper's output are index for index, and `asil_alwasl` reads
        // both. It also needs these characters present — a line separator is
        // `Non_Joining`, and it is what stops the last letter of one line
        // joining to the first letter of the next.
        if nass_maqta.chars().any(ghayr_marii) {
            ahdhif_ghayr_almarii(nass, maqta, mawsufat, &mut huruf);
        }

        let qiyasat = self.khatt.qiyasat(maqta.hajm);
        let mut mashkul = MaqtaMashkul {
            asl: maqta.clone(),
            huruf,
            ard: 0.0,
            suud: qiyasat.suud,
            hubut: qiyasat.hubut,
        };
        mashkul.qis();
        Ok(mashkul)
    }

    /// Resolves a style into variation coordinates, when the font has the axes
    /// the style asks for.
    ///
    /// Returns `None` for a static font, for a style that asks for nothing, and
    /// for a style whose axes this font does not have — in which case the font
    /// is shaped at its default instance rather than at an invented one.
    fn instans(&self, font: &FontRef<'_>, uslub: &Uslub) -> Option<ShaperInstance> {
        let mahawir = self.khatt.mahawir();
        if mahawir.is_empty() {
            return None;
        }

        let mut taghyeerat: SmallVec<[Variation; 2]> = SmallVec::new();
        if let Some(wazn) = uslub.wazn
            && let Some(mihwar) = mahawir.iter().find(|m| m.wasm == MIHWAR_WAZN)
        {
            taghyeerat.push(Variation {
                tag: Tag::new(&MIHWAR_WAZN),
                value: f32::from(wazn).clamp(mihwar.adna, mihwar.aqsa),
            });
        }
        if uslub.maail {
            if let Some(mihwar) = mahawir.iter().find(|m| m.wasm == MIHWAR_MAIL) {
                taghyeerat.push(Variation {
                    tag: Tag::new(&MIHWAR_MAIL),
                    value: 1.0_f32.clamp(mihwar.adna, mihwar.aqsa),
                });
            } else if let Some(mihwar) = mahawir.iter().find(|m| m.wasm == MIHWAR_MAYL) {
                // `slnt` is measured counter-clockwise from vertical, so the
                // slanted end of the axis is its minimum, not its maximum.
                taghyeerat.push(Variation {
                    tag: Tag::new(&MIHWAR_MAYL),
                    value: mihwar.adna,
                });
            }
        }

        if taghyeerat.is_empty() {
            None
        } else {
            Some(ShaperInstance::from_variations(
                font,
                taghyeerat.iter().copied(),
            ))
        }
    }
}

/// One prepared shaper per font identity.
///
/// A game redraws the same menu every frame, and preparing a font's layout
/// tables costs far more than shaping a short run with them. This keeps one
/// [`MushakkilKhatt`] per [`HuwiyatKhatt`] and nothing else: it is a font cache,
/// not a layout cache. The layout cache — keyed on text, size, width and
/// everything else a layout depends on, bounded by bytes rather than entries —
/// lives in `taarib-jisr`, where the game loop can see its hit rate.
///
/// Entries are never evicted. The bound is the number of distinct fonts a patch
/// declares, which is a handful; a store that evicted fonts would spend its time
/// re-parsing the one font every string uses.
pub struct MakhzanTashkeel {
    mushakkilat: FxHashMap<HuwiyatKhatt, MushakkilKhatt>,
}

impl core::fmt::Debug for MakhzanTashkeel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MakhzanTashkeel")
            .field("adad", &self.mushakkilat.len())
            .finish()
    }
}

impl Default for MakhzanTashkeel {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl MakhzanTashkeel {
    /// An empty store.
    #[must_use]
    pub fn jadeed() -> Self {
        Self {
            mushakkilat: FxHashMap::default(),
        }
    }

    /// The prepared shaper for a font, preparing it on first use.
    ///
    /// # Errors
    ///
    /// As [`MushakkilKhatt::jadeed`], on the first call for a given font
    /// identity only. A font that failed to prepare is not remembered as
    /// failed, because the failure is a property of the bytes and the bytes
    /// cannot change under a `MawridKhatt`.
    pub fn mushakkil(&mut self, khatt: &Arc<MawridKhatt>) -> Natija<&MushakkilKhatt> {
        match self.mushakkilat.entry(khatt.huwiya()) {
            Entry::Occupied(mawjud) => Ok(mawjud.into_mut()),
            Entry::Vacant(khali) => Ok(khali.insert(MushakkilKhatt::jadeed(khatt)?)),
        }
    }

    /// How many fonts are prepared.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mushakkilat.len()
    }

    /// Drops every prepared font.
    ///
    /// Called when a patch is unloaded or a font chain is replaced. The parsed
    /// caches are the largest thing this crate holds resident inside somebody
    /// else's game process, and holding them for a patch that is no longer
    /// installed is memory taken from the game for nothing.
    pub fn amsah(&mut self) {
        self.mushakkilat.clear();
    }
}

/// Shapes every run of a paragraph in order.
///
/// Runs are shaped independently but not in isolation: each is handed the
/// characters on either side of it as context, so a run boundary that a style
/// change dropped in the middle of a word does not sever the join there.
///
/// # Errors
///
/// [`KhataSaff::SilsilaFarigha`] when a run names a font the chain does not
/// have, and otherwise whatever [`MushakkilKhatt::shakkil`] reports for the
/// first run that fails. Failure is not partial: a paragraph with one
/// unshapeable run is a paragraph that cannot be drawn, and returning the runs
/// that did work would put a hole in the middle of a sentence.
pub fn shakkil_maqati(
    nass: &str,
    maqati: &[MaqtaMantiqi],
    khutut: &SilsilatKhutut,
    khiyarat: &KhiyaratTakhtit,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<Vec<MaqtaMashkul>> {
    shakkil_maqati_bi_asalib(nass, maqati, &[], khutut, khiyarat, makhzan)
}

/// Shapes every run of a paragraph with the style table in hand.
///
/// The same as [`shakkil_maqati`] except that each run's style span is looked up
/// and its weight and slant applied as variation coordinates. A caller with
/// variable fonts and a `<b>` in its markup wants this one.
///
/// # Errors
///
/// As [`shakkil_maqati`].
pub fn shakkil_maqati_bi_asalib(
    nass: &str,
    maqati: &[MaqtaMantiqi],
    nitaqat: &[NitaqUslub],
    khutut: &SilsilatKhutut,
    khiyarat: &KhiyaratTakhtit,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<Vec<MaqtaMashkul>> {
    let mut natija: Vec<MaqtaMashkul> = Vec::with_capacity(maqati.len());
    for maqta in maqati {
        let Some(khatt) = khutut.khatt(maqta.khatt) else {
            return Err(Khata::min_tafsir(&KhataSaff::SilsilaFarigha)
                .ma("khatt", u32::from(maqta.khatt))
                .ma("adad", khutut.adad()));
        };
        let uslub = nitaqat
            .iter()
            .find(|nitaq| nitaq.id == maqta.uslub)
            .map(|nitaq| nitaq.uslub);
        let mushakkil = makhzan.mushakkil(khatt)?;
        natija.push(mushakkil.shakkil_bi_uslub(nass, maqta, uslub.as_ref(), khiyarat)?);
    }
    Ok(natija)
}

/// The OpenType features Taarib declares for a script and language.
///
/// The returned pairs are the record of what is in force: the layout cache keys
/// on them, diagnostics print them, and a compiled patch carries them so that a
/// patch renders the same in five years as it does today. Tags are returned as
/// raw bytes rather than as a shaper type so that the stages which only need to
/// *record* the set — measurement, justification, the cache in `taarib-jisr` —
/// do not take a dependency on HarfRust to do it.
///
/// For a cursively joining script the set is `ccmp`, `locl`, `isol`, `fina`,
/// `medi`, `init`, `rlig`, `rclt`, `calt`, `liga`, `mset`, `curs`, `kern`,
/// `mark`, `mkmk`. Each group does something none of the others can:
///
/// - **Preparation.** `ccmp` composes and decomposes. It runs first because
///   everything after it assumes the sequence is already in the form the font's
///   tables were built to match.
/// - **Localized forms.** `locl` is the entire difference between correct
///   Persian and Arabic-shaped Persian. Persian and Urdu share the script but
///   not the letterforms: `kaf`, `yeh` and `heh` are drawn differently, and a
///   font expresses that through `locl` lookups keyed on the **language
///   system**. That means `locl` does nothing whatsoever unless the buffer
///   carries a language — so it is driven by [`LughaNass`], it is declared here
///   only when a language was actually declared, and
///   [`MushakkilKhatt::shakkil`] puts the same language on the buffer. Shaping
///   Persian without it produces text a Persian reader calls wrong, and no
///   choice of font repairs it.
/// - **Contextual forms.** `isol`, `fina`, `medi`, `init` select the isolated,
///   final, medial and initial shape of each letter. These four are what make
///   Arabic Arabic instead of twenty-eight disconnected letterforms. They are
///   declared here because they are part of the record of what is in force —
///   but they are deliberately *not* forwarded to the shaper as caller
///   features, because the shaper owns their per-glyph masks and a global
///   request would apply each of them to every letter at once.
/// - **Ligatures and contextual alternates.** `rlig` is required, not
///   decorative: mandatory lam-alef lives there, along with the hundreds of
///   required forms a Naskh font carries. `rclt` and `calt` are the contextual
///   rewrites that make a sequence of medial forms actually meet. `liga` is
///   discretionary ligation. `mset` is Arabic mark positioning in fonts that
///   predate `GPOS` mark attachment.
/// - **Cursive attachment.** `curs` is `GPOS` moving one glyph's entry point
///   onto its neighbour's exit point, which is how a Nastaliq or a good Naskh
///   font makes a joined word ride a curve rather than sit on a flat baseline.
/// - **Positioning.** `kern` spaces letter pairs; `mark` attaches a diacritic
///   to its base; `mkmk` stacks a second diacritic onto the first. Between them
///   they are the whole of the fourth failure Decision 1 exists to remove.
///
/// A script that does not join cursively gets the same set with the four
/// contextual forms and `curs` removed and `clig` in place of `mset`.
///
/// Caller features from [`KhiyaratTakhtit::sifat`] are applied **last** and by
/// tag, so a caller can raise a feature to an alternate, add one that is not in
/// the default set, or set it to zero and turn a default off. Turning `kern` off
/// for a game whose interface was measured without it is a real request, and
/// there is no other way to make it.
#[must_use]
pub fn sifat_alkitaba(
    kitaba: Kitaba,
    lugha: LughaNass,
    idafiya: &[SifaIdafiya],
) -> Vec<([u8; 4], u32)> {
    let asas: &[[u8; 4]] = if kitaba.tasil() {
        &SIFAT_WASL_KAMILA
    } else {
        &SIFAT_MUSTAQIMA
    };

    let mut sifat: Vec<([u8; 4], u32)> = Vec::with_capacity(asas.len() + idafiya.len());
    for wasm in asas {
        // A `locl` with no language system to select resolves to the default
        // and changes nothing, so declaring it would put noise in the record
        // that every cache key and every patch then carries forever.
        if *wasm == WASM_LOCL && matches!(lugha, LughaNass::Tilqai) {
            continue;
        }
        sifat.push((*wasm, 1));
    }

    for zaida in idafiya {
        if let Some(mawjud) = sifat.iter_mut().find(|(wasm, _)| *wasm == zaida.wasm) {
            mawjud.1 = zaida.qeema;
        } else {
            sifat.push((zaida.wasm, zaida.qeema));
        }
    }

    sifat
}

/// Turns the declared feature set into the features HarfRust is actually handed.
///
/// This is not a straight translation, and the difference is the single most
/// dangerous thing in this file.
///
/// HarfRust's Arabic shaper registers `isol`, `fina`, `medi` and `init` as
/// **non-global** features, which is what gives each of them its own mask bit;
/// the joining state machine then sets exactly one of those bits on each glyph
/// according to that glyph's neighbours. A user feature, on the other hand, is
/// registered as *global* when it covers the whole buffer — and when a global
/// request for the same tag is merged into the shaper's entry, the feature stops
/// having a bit of its own and moves onto the global bit, which is set on every
/// glyph. Asking for `init=1` therefore does not "make sure initial forms work":
/// it applies the initial-form lookup to **every letter in the run**. The text
/// that comes out is not Arabic.
///
/// So a non-zero value for a feature the shaper owns is not forwarded. A value
/// of **zero** is forwarded for every tag without exception, because a global
/// request at value zero drops the feature out of the plan entirely — which is
/// precisely what "turn `init` off" should mean, and is how HarfRust disables a
/// feature internally. Values above one on those four tags are dropped with the
/// rest: they are binary features, and a font that answers `init=2` is answering
/// a question nobody asked.
fn sifat_harfrust(sifat: &[([u8; 4], u32)]) -> Vec<Feature> {
    sifat
        .iter()
        .filter(|(wasm, qeema)| *qeema == 0 || !SIFAT_YAMLIKUHA_ALMUSHAKKIL.contains(wasm))
        .map(|(wasm, qeema)| Feature::new(Tag::new(wasm), *qeema, ..))
        .collect()
}

/// Resolves a run's byte range against the clean text.
///
/// A run whose range has drifted out of step with the text it names is not
/// shaped at a best guess: shaping half a character produces a plausible-looking
/// glyph for a sequence that does not exist, and every cluster index after it
/// points into the wrong word.
///
/// # Errors
///
/// [`KhataSaff::NitaqKharij`] when the range leaves the text and
/// [`KhataSaff::HaddNitaqTalif`] when one of its ends splits a UTF-8 sequence.
fn qita_almaqta<'a>(nass: &'a str, maqta: &MaqtaMantiqi) -> Natija<&'a str> {
    let tul = u32::try_from(nass.len()).unwrap_or(u32::MAX);
    let kharij = || {
        Khata::min_tafsir(&KhataSaff::NitaqKharij {
            id: maqta.uslub,
            bidaya: maqta.nitaq.start,
            nihaya: maqta.nitaq.end,
            tul,
        })
    };

    if maqta.nitaq.start > maqta.nitaq.end || maqta.nitaq.end > tul {
        return Err(kharij());
    }
    let (Ok(bidaya), Ok(nihaya)) = (
        usize::try_from(maqta.nitaq.start),
        usize::try_from(maqta.nitaq.end),
    ) else {
        return Err(kharij());
    };
    for (mawqi, hadd) in [(maqta.nitaq.start, bidaya), (maqta.nitaq.end, nihaya)] {
        if !nass.is_char_boundary(hadd) {
            return Err(Khata::min_tafsir(&KhataSaff::HaddNitaqTalif {
                id: maqta.uslub,
                mawqi,
            }));
        }
    }
    nass.get(bidaya..nihaya).ok_or_else(kharij)
}

/// Builds the shaping failure, optionally naming which of its two shapes it
/// took: nothing at all, or nothing but `.notdef` for text the font claims.
fn fashal_attashkeel(maqta: &MaqtaMantiqi, sabab: Option<&str>) -> Khata {
    let khata = Khata::min_tafsir(&KhataSaff::TashkeelFashil {
        tul: maqta.tul(),
        script: maqta.kitaba.0,
    });
    match sabab {
        Some(nass) => khata.ma("sabab", nass),
        None => khata,
    }
}

/// The direction the shaper is told, taken from the run and never guessed.
const fn ittijah_harfrust(ittijah: Ittijah) -> Direction {
    match ittijah {
        Ittijah::Yameen => Direction::RightToLeft,
        Ittijah::Yasar => Direction::LeftToRight,
    }
}

/// The script the shaper is told, taken from the run and never guessed.
///
/// [`Kitaba`] holds a lowercase OpenType tag and HarfRust wants an ISO 15924
/// one; the conversion is case-insensitive, so `arab` and `Arab` arrive at the
/// same script. A tag HarfRust does not recognise becomes its unknown script
/// rather than being flattened into Latin, which keeps a script Taarib has no
/// opinion about on the default shaper instead of on the wrong one.
const fn kitaba_harfrust(kitaba: Kitaba) -> Option<Script> {
    Script::from_iso15924_tag(Tag::new(&kitaba.0))
}

/// The language the shaper is told.
///
/// HarfRust wants a BCP 47 tag and maps it to the OpenType language system tag
/// itself — `fa` becomes `FAR `, `ur` becomes `URD ` — so handing it the
/// OpenType tag directly would fail to match anything. An undeclared language
/// stays undeclared: forcing `ar` onto text that might be Persian is exactly the
/// mistake `locl` exists to prevent.
fn lugha_harfrust(lugha: LughaNass) -> Option<Language> {
    if matches!(lugha, LughaNass::Tilqai) {
        None
    } else {
        Language::new(lugha.wasm())
    }
}

/// The language in force for a run: the run's own, falling back to the request's.
///
/// The run is more specific — a script-detection pass may have decided that this
/// particular stretch is Persian inside a document declared Arabic — so it wins
/// wherever it has an opinion.
const fn lugha_almaqta(maqta: &MaqtaMantiqi, khiyarat: &KhiyaratTakhtit) -> LughaNass {
    if matches!(maqta.lugha, LughaNass::Tilqai) {
        khiyarat.lugha
    } else {
        maqta.lugha
    }
}

/// The buffer flags for a run.
///
/// `PRODUCE_SAFE_TO_INSERT_TATWEEL` is asked for because it is the shaper's own
/// answer to "would elongating here disturb shaping", and [`asil_alwasl`] uses
/// it to confirm what the characters already said. The text-boundary flags are
/// set only for the runs that really are at the edges of the paragraph: they are
/// what makes a paragraph beginning with a stray combining mark show a dotted
/// circle instead of a mark floating over nothing, and setting them on every run
/// would put a dotted circle at the start of every mark-initial run in the
/// middle of a sentence.
fn alamat_almahfaza(nass: &str, maqta: &MaqtaMantiqi) -> BufferFlags {
    let mut alamat = BufferFlags::PRODUCE_SAFE_TO_INSERT_TATWEEL;
    if maqta.nitaq.start == 0 {
        alamat |= BufferFlags::BEGINNING_OF_TEXT;
    }
    if usize::try_from(maqta.nitaq.end).is_ok_and(|nihaya| nihaya >= nass.len()) {
        alamat |= BufferFlags::END_OF_TEXT;
    }
    alamat
}

/// The characters immediately before a run, for the shaper's leading context.
fn siyaq_qabl(nass: &str, hatta: u32) -> &str {
    let Ok(hadd) = usize::try_from(hatta) else {
        return "";
    };
    let Some(sabiq) = nass.get(..hadd) else {
        return "";
    };
    let mut bidaya = sabiq.len();
    for (mawqi, _) in sabiq.char_indices().rev().take(TUL_SIYAQ) {
        bidaya = mawqi;
    }
    sabiq.get(bidaya..).unwrap_or("")
}

/// The characters immediately after a run, for the shaper's trailing context.
fn siyaq_baad(nass: &str, min: u32) -> &str {
    let Ok(hadd) = usize::try_from(min) else {
        return "";
    };
    let Some(lahiq) = nass.get(hadd..) else {
        return "";
    };
    let mut nihaya = 0;
    for (mawqi, harf) in lahiq.char_indices().take(TUL_SIYAQ) {
        nihaya = mawqi.saturating_add(harf.len_utf8());
    }
    lahiq.get(..nihaya).unwrap_or("")
}

/// Reads which glyphs a font's `GDEF` classifies as combining marks.
///
/// This is the same question HarfRust asks when it decides to zero a glyph's
/// advance and hand it to mark attachment, answered from the same table, so the
/// two cannot disagree about what a mark is. It is read once per font because
/// the answer is a property of the font and of nothing else.
///
/// A font with no `GDEF` glyph classes produces an empty set, and its marks then
/// carry whatever advance and position `GPOS` gave them. That is the honest
/// outcome: Decision 6's validation is what refuses such a font for Arabic in
/// the first place, and inventing a mark classification here would be this file
/// deciding something the font declined to say.
///
/// # Errors
///
/// Whatever [`MawridKhatt::khatt`] reports when the bytes cannot be parsed.
fn alamat_alkhatt(khatt: &Arc<MawridKhatt>) -> Natija<MajmuatAshkal> {
    use read_fonts::TableProvider as _;

    let font = khatt.khatt()?;
    let adad = font.maxp().map_or(0, |maxp| u32::from(maxp.num_glyphs()));
    let mut alamat = MajmuatAshkal::jadida(adad);

    if let Ok(gdef) = font.gdef()
        && let Some(Ok(asnaf)) = gdef.glyph_class_def()
    {
        for (muarrif, sinf) in asnaf.iter() {
            if sinf == SINF_ALAMA {
                alamat.dai(u32::from(muarrif.to_u16()));
            }
        }
    }

    Ok(alamat)
}

/// Fills in the joining and elongation properties of every glyph in a run.
///
/// The whole of kashida justification rests on this function, and getting it
/// wrong has one very visible symptom: elongation appearing where Arabic never
/// elongates, which is the most recognisable mark of a renderer that does not
/// understand the script.
///
/// The reasoning, in order:
///
/// 1. **Clusters are resolved back to characters.** Each distinct cluster value
///    in the output is a byte offset into the clean text; sorted, they partition
///    the run. Within a cluster, marks are skipped — a `shadda` between two
///    joined letters is transparent and must not break the join — leaving the
///    first and last real letters, which are the only two that can join to
///    anything outside the cluster. For a lam-alef ligature that is `lam` on one
///    side and `alef` on the other, which is exactly right: the ligature joins
///    backwards like a `lam` and stops dead like an `alef`.
/// 2. **Joins are decided between clusters, never inside one.** A join exists
///    where the earlier letter can join forwards and the later one can join
///    backwards. A cluster of nothing but marks is skipped entirely, so the
///    letters on either side of it stay joined to each other.
/// 3. **Ranks come from the pair at the joint**, in logical order, which is what
///    the classical priority order is stated in.
/// 4. **The answer is mapped from logical order to visual order.** The shaper's
///    output is in visual order, so in a right-to-left run the glyph that
///    follows in the array is the one that *precedes* in the text. Applying the
///    logical answer to visual neighbours without that flip stretches the wrong
///    side of every letter in the language.
/// 5. **Only the edges of a cluster carry a join.** A cluster is indivisible for
///    elongation, so an interior glyph of a multi-glyph cluster reports no join
///    and can never be chosen as a candidate.
fn asil_alwasl(
    nass: &str,
    maqta: &MaqtaMantiqi,
    mawsufat: &[GlyphInfo],
    huruf: &mut [HarfMashkul],
) {
    let anaqid = anaqid_almaqta(nass, maqta, huruf);
    if anaqid.is_empty() {
        return;
    }

    // The shaper's own verdict on where a tatweel may be inserted without
    // disturbing shaping, used to confirm what the characters said. It is only
    // consulted when the shaper produced it at all: a run in which no glyph
    // carries the flag is a run the flag says nothing about, and letting that
    // silence veto every candidate would disable kashida wholesale.
    let yunataq = mawsufat.iter().any(GlyphInfo::safe_to_insert_tatweel);
    let min_alyameen = matches!(maqta.ittijah, Ittijah::Yameen);

    let mut bidaya = 0_usize;
    while bidaya < huruf.len() {
        let Some(anqud) = huruf.get(bidaya).map(|harf| harf.anqud) else {
            break;
        };
        let mut nihaya = bidaya.saturating_add(1);
        while huruf.get(nihaya).is_some_and(|harf| harf.anqud == anqud) {
            nihaya = nihaya.saturating_add(1);
        }

        if let Ok(fahras) = anaqid.binary_search_by_key(&anqud, |wahid| wahid.bidaya)
            && let Some(wahid) = anaqid.get(fahras).copied()
        {
            // In a right-to-left run the array runs the other way from the
            // text, so the join that shows up on a glyph's left is the one
            // the text records on its logically following side. `wasl_yasar`
            // is the join on the left of this cluster's glyphs, `wasl_yameen`
            // the one on their right, and `rutba` the rank of that right-hand
            // joint — the one an elongation would be inserted into.
            let (wasl_yasar, wasl_yameen, rutba) = if min_alyameen {
                (wahid.wasl_baad, wahid.wasl_qabl, wahid.rutba_qabl)
            } else {
                (wahid.wasl_qabl, wahid.wasl_baad, wahid.rutba_baad)
            };

            let mut awwal_asl: Option<usize> = None;
            let mut akhir_asl: Option<usize> = None;
            for mawqi in bidaya..nihaya {
                if huruf.get(mawqi).is_some_and(|harf| !harf.alama) {
                    if awwal_asl.is_none() {
                        awwal_asl = Some(mawqi);
                    }
                    akhir_asl = Some(mawqi);
                }
            }

            for mawqi in bidaya..nihaya {
                let masmuh = !yunataq
                    || mawsufat
                        .get(mawqi)
                        .is_some_and(GlyphInfo::safe_to_insert_tatweel);
                let Some(harf) = huruf.get_mut(mawqi) else {
                    continue;
                };
                if harf.alama {
                    // A mark joins nothing and is never an elongation
                    // point. Leaving it at the default is what keeps
                    // justification from stretching a diacritic.
                    continue;
                }
                // Only the outermost glyphs of a cluster carry a join: a
                // cluster is one indivisible thing for elongation, and an
                // interior glyph reporting a join would let justification
                // stretch a place that is inside a single letterform.
                let hadd_yasar = awwal_asl == Some(mawqi);
                let hadd_yameen = akhir_asl == Some(mawqi);
                let baad = hadd_yameen && wasl_yameen;
                let rutba_harf = if baad { rutba } else { 0 };
                harf.wasl = SifatWasl {
                    qabl: hadd_yasar && wasl_yasar,
                    baad,
                    // `madd` is the script's half of the question: this joint
                    // is one Arabic elongates at, and the shaper agrees a
                    // tatweel can go in without disturbing shaping. The
                    // font's half — whether it offers a stretched form
                    // through `jstf` or an elongation-aware alternate — is
                    // `kashida`'s to add, because reading `jstf` here would
                    // be this stage answering a justification question.
                    madd: baad && rutba_harf > 0 && masmuh,
                    rutba: rutba_harf,
                };
            }
        }

        bidaya = nihaya;
    }
}

/// Removes the glyphs that stand for characters the layout consumes rather than
/// draws.
///
/// A newline is not text. Neither is a paragraph separator, a bidirectional
/// override, or the byte-order mark: each one is an instruction to a stage that
/// has already read it — `taqtee` for the break, `ittijah` for the direction —
/// and none of them has a letterform in any font. Handed to the shaper they map
/// through `cmap` to nothing, come back as glyph 0, and are drawn as the empty
/// box that glyph is, at that box's own advance. A patched dialogue page then
/// carries a tofu at the end of every line and one alone on every blank line
/// between paragraphs, and every line it ends measures wider than the text on
/// it.
///
/// So the rule is stated on the *character*, not on the glyph: a cluster made
/// entirely of [`ghayr_marii`] characters produces nothing, whatever the font
/// answered for it. A font that maps a line feed to a blank glyph — plenty do —
/// would otherwise still charge the line for its width.
///
/// A whole cluster has to be invisible before any of it goes. A cluster is the
/// unit that maps glyphs back to characters, so one that mixed an invisible
/// character with a real one would be a ligature over both, and dropping its
/// glyph would delete the letter with it.
fn ahdhif_ghayr_almarii(
    nass: &str,
    maqta: &MaqtaMantiqi,
    mawsufat: &[GlyphInfo],
    huruf: &mut Vec<HarfMashkul>,
) {
    let mut hudud: SmallVec<[u32; 32]> = mawsufat.iter().map(|wasf| wasf.cluster).collect();
    hudud.sort_unstable();
    hudud.dedup();

    huruf.retain(|harf| {
        // The cluster runs to the next distinct cluster start, or to the end of
        // the run for the last one. Cluster values are byte offsets into the
        // whole clean text, so the two ends are directly comparable.
        let nihaya = hudud
            .get(hudud.partition_point(|bidaya| *bidaya <= harf.anqud))
            .copied()
            .unwrap_or(maqta.nitaq.end);
        !anqud_ghayr_marii(nass, harf.anqud, nihaya)
    });
}

/// Whether every character of one cluster is one the layout consumes rather
/// than draws.
///
/// An empty or unresolvable range answers `false`: a cluster nothing can be read
/// from is not evidence that there is nothing to draw, and deleting a glyph on
/// that basis would lose text.
fn anqud_ghayr_marii(nass: &str, bidaya: u32, nihaya: u32) -> bool {
    let (Ok(min), Ok(ila)) = (usize::try_from(bidaya), usize::try_from(nihaya)) else {
        return false;
    };
    nass.get(min..ila)
        .is_some_and(|juz| !juz.is_empty() && juz.chars().all(ghayr_marii))
}

/// Resolves a run's clusters back to the characters that produced them, and
/// works out which of them are joined to which.
fn anaqid_almaqta(
    nass: &str,
    maqta: &MaqtaMantiqi,
    huruf: &[HarfMashkul],
) -> SmallVec<[Anqud; 32]> {
    let mut mawaqi: SmallVec<[u32; 32]> = huruf.iter().map(|harf| harf.anqud).collect();
    mawaqi.sort_unstable();
    mawaqi.dedup();

    let mut anaqid: SmallVec<[Anqud; 32]> = SmallVec::with_capacity(mawaqi.len());
    for (fahras, bidaya) in mawaqi.iter().copied().enumerate() {
        let nihaya = mawaqi
            .get(fahras.saturating_add(1))
            .copied()
            .unwrap_or(maqta.nitaq.end);
        let (awwal, akhir) = tarafa_alanqud(nass, bidaya, nihaya);
        anaqid.push(Anqud {
            bidaya,
            awwal,
            akhir,
            wasl_qabl: false,
            wasl_baad: false,
            rutba_qabl: 0,
            rutba_baad: 0,
        });
    }

    // Walked in logical order — ascending byte offsets — because joining is a
    // property of the text, not of the order the glyphs came back in.
    let mut sabiq: Option<usize> = None;
    for fahras in 0..anaqid.len() {
        // Every borrow of `anaqid` is resolved into an owned `char` on its own
        // statement, so that nothing is still borrowing the vector when the two
        // ends of a joint are written back into it.
        let hadi = anaqid.get(fahras).and_then(|wahid| wahid.awwal);
        let Some(awwal) = hadi else {
            // Nothing but marks: transparent, and skipped without disturbing
            // `sabiq`, so the letters on either side of it stay joined.
            continue;
        };

        let khatim = sabiq
            .and_then(|mawqi| anaqid.get(mawqi))
            .and_then(|wahid| wahid.akhir);
        if let (Some(mawqi_sabiq), Some(akhir_sabiq)) = (sabiq, khatim)
            && yasil(akhir_sabiq, awwal)
        {
            let rutba = rutbat_kashida(akhir_sabiq, awwal);
            if let Some(wahid) = anaqid.get_mut(mawqi_sabiq) {
                wahid.wasl_baad = true;
                wahid.rutba_baad = rutba;
            }
            if let Some(wahid) = anaqid.get_mut(fahras) {
                wahid.wasl_qabl = true;
                wahid.rutba_qabl = rutba;
            }
        }
        sabiq = Some(fahras);
    }

    anaqid
}

/// The first and last non-mark characters of a cluster.
///
/// `None` for both when the cluster is nothing but marks, which is how a mark
/// cluster becomes transparent to joining rather than a break in it.
fn tarafa_alanqud(nass: &str, bidaya: u32, nihaya: u32) -> (Option<char>, Option<char>) {
    let (Ok(min), Ok(ila)) = (usize::try_from(bidaya), usize::try_from(nihaya)) else {
        return (None, None);
    };
    let Some(juz) = nass.get(min..ila) else {
        return (None, None);
    };

    let mut awwal: Option<char> = None;
    let mut akhir: Option<char> = None;
    for harf in juz.chars() {
        if matches!(naw_wasl(harf), NawWasl::Shaffaf) {
            continue;
        }
        if awwal.is_none() {
            awwal = Some(harf);
        }
        akhir = Some(harf);
    }
    (awwal, akhir)
}

/// Whether two characters in logical order are cursively joined.
///
/// The earlier one must be able to join forwards, which only a dual-joining
/// letter does; the later one must be able to join backwards, which both a
/// dual-joining and a right-joining letter do. An `alef` after a `beh` is
/// joined; a `beh` after an `alef` is not, and stretching between them is the
/// error this predicate exists to prevent.
fn yasil(qabl: char, baad: char) -> bool {
    matches!(naw_wasl(qabl), NawWasl::Yasil)
        && matches!(naw_wasl(baad), NawWasl::Yasil | NawWasl::YaqifBaad)
}

/// A shaper position, in font units, as a number that can be scaled.
#[allow(
    clippy::cast_precision_loss,
    reason = "font units are bounded by the design grid, far inside the range f32 represents \
              exactly; the alternative is f64 arithmetic on every glyph of every frame"
)]
const fn kasr(qeema: i32) -> f32 {
    qeema as f32
}
