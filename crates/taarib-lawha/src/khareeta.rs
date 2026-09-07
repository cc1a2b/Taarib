//! الخريطة — what is in the atlas, and where.
//!
//! A glyph is identified by the font it came from and the identifier shaping
//! produced, never by a character. That is Decision 1 restated at the atlas
//! layer: an atlas keyed by codepoint could only ever hold one image per
//! character, which is another way of saying it could not hold Arabic, where one
//! character has four contextual forms and any number of ligated ones.
//!
//! The key also carries the pixel size, the rasterization mode and the subpixel
//! bucket, because those produce genuinely different images of the same glyph.
//! Leaving any of them out of the key is how an atlas ends up drawing a
//! sixteen-pixel glyph where a twenty-four-pixel one belongs.

use core::ops::Range;

use rustc_hash::FxHashMap;

/// How a glyph was rasterized.
///
/// Stored as a byte because this value crosses into the patch container, where
/// every field is a fixed-width record that C#, JavaScript, Python and Ruby all
/// read by struct overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NamatSafha {
    /// Eight-bit antialiased coverage. Sharper at a fixed small size, and the
    /// default for interface text.
    Taghtiya = 0,
    /// A signed distance field. One page serves every size, which is what makes
    /// auto-sizing text and world-space text work.
    Masafa = 1,
}

impl NamatSafha {
    /// The byte written into the patch container.
    #[must_use]
    pub const fn bayt(self) -> u8 {
        self as u8
    }

    /// Reads the byte back, defaulting to coverage for anything unrecognised —
    /// a patch from a newer build declaring a mode this one does not know draws
    /// something legible rather than nothing.
    #[must_use]
    pub const fn min_bayt(bayt: u8) -> Self {
        match bayt {
            1 => Self::Masafa,
            _ => Self::Taghtiya,
        }
    }
}

/// Everything that makes one rasterized image of a glyph distinct from another.
///
/// Ordered, so that a glyph set sorts deterministically and two compiles of the
/// same patch pack the same pages in the same order and produce byte-identical
/// output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MiftahShakl {
    /// Index into the patch's font chain. Not a font identity: identities are
    /// process-local hashes, and a patch has to name its fonts in a way that
    /// survives being written to disk and read on another machine.
    pub khatt: u8,
    /// The subpixel bucket, from `taarib_saff::rasm::bakat_tahazzuz`.
    pub bakat: u8,
    /// The pixel size in quarter-pixels, so that a size of 13.25 is a distinct
    /// key from 13.5 without carrying a float in a hash key.
    pub hajm_rubi: u16,
    /// How it was rasterized.
    pub namat: NamatSafha,
    /// The glyph identifier, as shaping produced it.
    pub muarrif: u32,
}

impl MiftahShakl {
    /// Builds a key from a pixel size, rounding to the nearest quarter pixel.
    ///
    /// Quantizing here rather than at the call site is what keeps the atlas
    /// finite: a caller laying out at every fractional size a spring animation
    /// passes through would otherwise ask for a new image per frame.
    #[must_use]
    pub fn jadeed(khatt: u8, muarrif: u32, hajm: f32, namat: NamatSafha, bakat: u8) -> Self {
        let rubi = (hajm * 4.0).round().clamp(0.0, f32::from(u16::MAX));
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to the u16 range and non-negative on the line above"
        )]
        let hajm_rubi = rubi as u16;
        Self {
            khatt,
            bakat,
            hajm_rubi,
            namat,
            muarrif,
        }
    }

    /// The pixel size this key was built from.
    #[must_use]
    pub fn hajm(self) -> f32 {
        f32::from(self.hajm_rubi) / 4.0
    }
}

/// Where one glyph lives, and how to draw it.
///
/// Positions are pixels in the page, not normalized coordinates. The normalized
/// form is derived on demand by [`KhareetatAshkal::ihdathiyat`], because a page
/// that grows at runtime would invalidate every stored normalized rectangle the
/// moment it did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MawdiShakl {
    /// Which page.
    pub safha: u16,
    /// Left edge in the page, in pixels.
    pub s: u16,
    /// Top edge in the page, in pixels.
    pub a: u16,
    /// Width in pixels.
    pub ard: u16,
    /// Height in pixels.
    pub irtifa: u16,
    /// Left bearing: how far right of the pen position the image starts.
    pub izaha_s: i16,
    /// Top bearing: how far above the baseline the image's top edge sits.
    pub izaha_a: i16,
    /// The glyph's advance at this size, carried so an adapter drawing from the
    /// atlas alone never has to reach back into the font.
    pub taqaddum: f32,
}

impl MawdiShakl {
    /// Whether this glyph has no image at all, which is the correct and common
    /// answer for a space.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.ard == 0 || self.irtifa == 0
    }

    /// The rectangle as `(left, top, right, bottom)` in page pixels.
    #[must_use]
    pub const fn mustatil(&self) -> (u16, u16, u16, u16) {
        (
            self.s,
            self.a,
            self.s.saturating_add(self.ard),
            self.a.saturating_add(self.irtifa),
        )
    }
}

/// Normalized texture coordinates, ready for a vertex buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ihdathiyat {
    /// Left edge, 0.0 to 1.0.
    pub s0: f32,
    /// Top edge.
    pub a0: f32,
    /// Right edge.
    pub s1: f32,
    /// Bottom edge.
    pub a1: f32,
}

/// The map from glyph keys to their places in the atlas.
///
/// Two representations, on purpose. The hash map is what a runtime lookup uses.
/// The sorted vector is what the patch container stores, because a container
/// section that is sorted can be searched by an adapter with a binary search
/// over a memory-mapped slice, with no parsing and no allocation — which is the
/// difference between an adapter that costs nothing per frame and one that
/// builds a hash map at load time inside somebody's game.
#[derive(Debug, Clone, Default)]
pub struct KhareetatAshkal {
    mawadi: FxHashMap<MiftahShakl, MawdiShakl>,
    abaad: Vec<(u16, u16)>,
}

impl KhareetatAshkal {
    /// An empty map.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Records where a glyph was placed.
    pub fn daa(&mut self, miftah: MiftahShakl, mawdi: MawdiShakl) {
        let _ = self.mawadi.insert(miftah, mawdi);
    }

    /// Removes a glyph, for the runtime evictor.
    pub fn ihdhif(&mut self, miftah: MiftahShakl) -> Option<MawdiShakl> {
        self.mawadi.remove(&miftah)
    }

    /// Where a glyph is, if it is in the atlas at all.
    #[must_use]
    pub fn mawdi(&self, miftah: MiftahShakl) -> Option<&MawdiShakl> {
        self.mawadi.get(&miftah)
    }

    /// How many glyphs are mapped.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mawadi.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.mawadi.is_empty()
    }

    /// Records a page's dimensions, so texture coordinates can be derived.
    pub fn sajjil_safha(&mut self, safha: u16, ard: u16, irtifa: u16) {
        let fahras = usize::from(safha);
        if self.abaad.len() <= fahras {
            self.abaad.resize(fahras + 1, (0, 0));
        }
        if let Some(khana) = self.abaad.get_mut(fahras) {
            *khana = (ard, irtifa);
        }
    }

    /// A page's dimensions.
    #[must_use]
    pub fn abaad_safha(&self, safha: u16) -> Option<(u16, u16)> {
        self.abaad
            .get(usize::from(safha))
            .copied()
            .filter(|(ard, irtifa)| *ard > 0 && *irtifa > 0)
    }

    /// How many pages the atlas has.
    #[must_use]
    pub const fn adad_safahat(&self) -> usize {
        self.abaad.len()
    }

    /// Normalized texture coordinates for a placed glyph.
    ///
    /// Returns [`None`] when the glyph's page has no recorded size, which can
    /// only happen if a caller placed a glyph without registering the page it
    /// went into.
    #[must_use]
    pub fn ihdathiyat(&self, mawdi: &MawdiShakl) -> Option<Ihdathiyat> {
        let (ard, irtifa) = self.abaad_safha(mawdi.safha)?;
        let ard = f32::from(ard);
        let irtifa = f32::from(irtifa);
        Some(Ihdathiyat {
            s0: f32::from(mawdi.s) / ard,
            a0: f32::from(mawdi.a) / irtifa,
            s1: f32::from(mawdi.s.saturating_add(mawdi.ard)) / ard,
            a1: f32::from(mawdi.a.saturating_add(mawdi.irtifa)) / irtifa,
        })
    }

    /// Every entry, sorted by key.
    ///
    /// This is the order the patch container stores, and it is deterministic:
    /// two compiles of the same project produce the same sequence, which is what
    /// lets a rebuilt patch keep its content hash when nothing changed.
    #[must_use]
    pub fn murattaba(&self) -> Vec<(MiftahShakl, MawdiShakl)> {
        let mut madkhalat: Vec<(MiftahShakl, MawdiShakl)> = self
            .mawadi
            .iter()
            .map(|(miftah, mawdi)| (*miftah, *mawdi))
            .collect();
        madkhalat.sort_unstable_by_key(|(miftah, _)| *miftah);
        madkhalat
    }

    /// Every key currently mapped, sorted.
    #[must_use]
    pub fn mafatih(&self) -> Vec<MiftahShakl> {
        let mut mafatih: Vec<MiftahShakl> = self.mawadi.keys().copied().collect();
        mafatih.sort_unstable();
        mafatih
    }

    /// The keys belonging to one page, for an evictor that wants to reclaim a
    /// whole page rather than individual rectangles.
    #[must_use]
    pub fn mafatih_safha(&self, safha: u16) -> Vec<MiftahShakl> {
        let mut mafatih: Vec<MiftahShakl> = self
            .mawadi
            .iter()
            .filter(|(_, mawdi)| mawdi.safha == safha)
            .map(|(miftah, _)| *miftah)
            .collect();
        mafatih.sort_unstable();
        mafatih
    }

    /// The range of glyph identifiers present for one font at one size, which
    /// the compiler reports so a reviewer can see at a glance whether a patch's
    /// atlas covers what its strings need.
    #[must_use]
    pub fn nitaq_muarrifat(&self, khatt: u8, hajm_rubi: u16) -> Option<Range<u32>> {
        let mut adna = u32::MAX;
        let mut aqsa = 0u32;
        let mut wujid = false;
        for miftah in self.mawadi.keys() {
            if miftah.khatt == khatt && miftah.hajm_rubi == hajm_rubi {
                adna = adna.min(miftah.muarrif);
                aqsa = aqsa.max(miftah.muarrif);
                wujid = true;
            }
        }
        wujid.then(|| adna..aqsa.saturating_add(1))
    }

    /// Empties the map while keeping its allocations.
    pub fn amsah(&mut self) {
        self.mawadi.clear();
        self.abaad.clear();
    }
}
