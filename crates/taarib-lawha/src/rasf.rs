//! الرصف — packing glyph rectangles into pages, through `etagere`'s shelf
//! allocator.
//!
//! ## Why shelves
//!
//! Glyphs from one font at one size are all roughly one height. Shelf packing —
//! rows of a common height, filled left to right — is close to optimal for that
//! distribution while staying cheap enough to run inside a game process, and,
//! unlike the guillotine and maximal-rectangles families, it supports
//! deallocation of an individual item without a repack. That last property is
//! not a nicety here: [`crate::namu`] evicts by *rectangle*, and a packer that
//! could only reclaim whole pages would throw away hundreds of glyphs to make
//! room for one.
//!
//! ## Padding, and the failure it prevents
//!
//! A glyph is never allocated at exactly its own size. The rectangle asked of
//! the allocator is `hashw` pixels larger on every side, and the glyph is drawn
//! inside it, leaving a gutter of zeros around every letter.
//!
//! The reason is bilinear filtering. A sampler asked for a texel at the very
//! edge of a glyph's rectangle averages that texel with its neighbour *outside*
//! the rectangle. With no gutter, the neighbour is whatever letter the packer
//! happened to place next door, so a `ba` drawn at a fractional scale picks up a
//! sliver of the `noon` beside it: a faint smear along one edge of some letters,
//! at some sizes, on some pages. It is invisible in a screenshot of the atlas
//! and unmistakable in motion, and it is undiagnosable from a bug report. Two
//! texels of zeros between any two glyphs — one from each rectangle's gutter —
//! makes it impossible instead.
//!
//! A gutter of zeros is the correct neutral value in both page modes: zero
//! coverage is no ink, and a zero in a distance field is the saturated *outside*
//! value.
//!
//! ## A page is allocated at the ceiling and finished at its extent
//!
//! [`KhiyaratRasf::aqsa_ard`] and [`KhiyaratRasf::aqsa_irtifa`] are a *maximum*,
//! not a size. The packer has to open its first page before it has seen a single
//! rectangle — a shelf allocator is fed one item at a time and cannot be told in
//! advance how tall the result will be — so it opens the page at the maximum,
//! because a page opened at a guess is a page the next glyph overflows, and
//! overflowing costs a whole extra page rather than a few rows.
//!
//! That is right for the allocator and wrong for what ships. A pack of a few
//! hundred glyphs fills the first ninety-odd rows of a 4096-row page and leaves
//! the rest zero, and every client that downloads the patch allocates all of it.
//! So the height a page was *allocated* at and the height it is *finished* at
//! are two different numbers: [`Rasif::irtifa_lazim`] reports the second, taken
//! from the bottom-most row the allocator actually handed out, and
//! [`Safha::iqtati`] is what a caller that has finished packing uses to give the
//! rest back.
//!
//! The crop is **height only**, and that is not a limitation being tolerated —
//! it is what makes it provable. Page bytes are row-major from the top with no
//! row padding, so dropping trailing rows leaves every remaining byte at the
//! same offset it already had and every glyph at the same `(s, a)` it was
//! packed at. Narrowing a page would restride every row and move every byte of
//! every glyph after the first, for rows the shelf allocator fills left to right
//! and rarely leaves much of anyway.
//!
//! ## Pages are single-channel R8
//!
//! Per Decision 5, Taarib draws with primitives that exist in every version of
//! every engine, and colour is not one of them. A page carries one byte per
//! texel — coverage or distance — and every colour in the finished text comes
//! from the material the adapter binds, per style span, at draw time. There is
//! no colour in an atlas, no per-glyph tint, and no second channel waiting to
//! hold one.
//!
//! ## Determinism
//!
//! The same sequence of allocations must produce byte-identical pages, or a
//! recompiled patch churns its own content hash and every mirror that already
//! had it has to fetch it again.
//!
//! Three things secure that here. [`AllocatorOptions`] is written out in full
//! rather than defaulted — alignment `1x1`, horizontal shelves, a single column
//! — so a future change to `etagere`'s defaults cannot silently move every
//! rectangle in every patch; those three values are the whole of the allocator's
//! configurable behaviour, and the shelf algorithm underneath them is a pure
//! function of the allocation order. Page bytes are zero-filled at creation, so
//! every texel no glyph covers is a known value rather than uninitialised
//! memory. And nothing in this module iterates a hash map to decide where
//! anything goes: the map here is only ever consulted by key.

use etagere::{AllocId, AllocatorOptions, AtlasAllocator, Size, size2};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use taarib_saff::rasm::{NamatRasm, SurahHarf};
use taarib_usus::khata::Natija;

use crate::khareeta::{MiftahShakl, NamatSafha};
use crate::khata::KhataLawha;

/// The widest and tallest page the conservative GPU profile accepts.
///
/// Every desktop GPU Taarib will meet supports 4096; 2048 is the floor for the
/// oldest integrated parts and the mobile-derived drivers some Proton stacks
/// still expose.
pub const BUD_MUHAFIZ: u16 = 2048;

/// The default page dimension.
pub const BUD_IFTIRADI: u16 = 4096;

/// The default gutter, in pixels, around every glyph.
///
/// One pixel on each side puts two pixels of zeros between any two neighbouring
/// glyphs, which is exactly what a bilinear tap at a rectangle's edge can reach.
pub const HASHW_IFTIRADI: u16 = 1;

/// The default page budget.
///
/// Eight pages of 4096 is 128 MB of R8 texture — far past what any patch should
/// need, and low enough that a runaway glyph set is reported rather than
/// swallowing a game's video memory.
pub const AQSA_SAFAHAT_IFTIRADI: u16 = 8;

/// How pages are sized, padded and budgeted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KhiyaratRasf {
    /// The widest page this atlas may create. Never exceeded: the packer opens
    /// another page instead of scaling a glyph down.
    pub aqsa_ard: u16,
    /// The tallest page this atlas may create.
    pub aqsa_irtifa: u16,
    /// The gutter around every glyph, in pixels, applied on all four sides.
    pub hashw: u16,
    /// Whether page dimensions are reduced to a power of two.
    ///
    /// Some old drivers, and every GL ES 2 path a Proton stack can fall back
    /// to, only sample non-power-of-two textures with clamped wrapping and no
    /// mipmaps. Since a page may never *exceed* its declared maximum, honouring
    /// this means taking the largest power of two that fits rather than rounding
    /// upward — which is the same number whenever the maximum is itself a power
    /// of two, and both shipped profiles are.
    pub quwwat_ithnayn: bool,
    /// How many pages this atlas may open.
    pub aqsa_safahat: u16,
}

impl Default for KhiyaratRasf {
    fn default() -> Self {
        Self {
            aqsa_ard: BUD_IFTIRADI,
            aqsa_irtifa: BUD_IFTIRADI,
            hashw: HASHW_IFTIRADI,
            quwwat_ithnayn: false,
            aqsa_safahat: AQSA_SAFAHAT_IFTIRADI,
        }
    }
}

impl KhiyaratRasf {
    /// The conservative profile: 2048 pages, rounded to a power of two.
    #[must_use]
    pub fn muhafiz() -> Self {
        Self {
            aqsa_ard: BUD_MUHAFIZ,
            aqsa_irtifa: BUD_MUHAFIZ,
            quwwat_ithnayn: true,
            ..Self::default()
        }
    }
}

/// One texture page: its dimensions, what its bytes mean, and the bytes.
///
/// One byte per texel, row-major from the top, no row padding. This is the
/// buffer an adapter uploads verbatim as an `R8` texture.
// Clone, because the compiler hands finished pages to the patch container while
// the packer keeps its own copy to go on allocating into. `Debug` is written by
// hand below rather than derived: a derived one would dump several megabytes of
// texels into a log line.
#[derive(Clone, PartialEq, Eq)]
pub struct Safha {
    /// Width in texels.
    pub ard: u16,
    /// Height in texels.
    pub irtifa: u16,
    /// What the bytes mean.
    pub namat: NamatSafha,
    /// The texels.
    pub bayt: Vec<u8>,
}

impl core::fmt::Debug for Safha {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The buffer is reported as a length. A debug line that dumps sixteen
        // megabytes of atlas into a log is a log nobody can read.
        matbaa
            .debug_struct("Safha")
            .field("ard", &self.ard)
            .field("irtifa", &self.irtifa)
            .field("namat", &self.namat)
            .field("bayt", &self.bayt.len())
            .finish()
    }
}

impl Safha {
    /// An empty page, zero-filled.
    fn jadeeda(ard: u16, irtifa: u16, namat: NamatSafha) -> Self {
        let hajm = usize::from(ard).saturating_mul(usize::from(irtifa));
        Self {
            ard,
            irtifa,
            namat,
            bayt: vec![0u8; hajm],
        }
    }

    /// How many bytes this page costs.
    #[must_use]
    pub const fn bayt_hajm(&self) -> usize {
        self.bayt.len()
    }

    /// Drops the rows below `irtifa`, giving their bytes back.
    ///
    /// For a caller that has finished packing and is about to ship the page. A
    /// height greater than the page already has is ignored rather than treated
    /// as a request to grow: this hands unused rows back, and there is nothing
    /// meaningful to put in rows that were never allocated.
    ///
    /// Every glyph keeps the position it was packed at — see the module header
    /// for why a height-only crop can promise that and a width change cannot —
    /// so a caller's only remaining duty is to record the new height wherever it
    /// recorded the old one, because normalized texture coordinates are derived
    /// from it.
    ///
    /// The buffer is shrunk rather than merely truncated. A `Vec` that keeps its
    /// capacity would give the bytes back on paper and hold sixteen megabytes of
    /// them for as long as the page is alive, which is most of a compile.
    pub fn iqtati(&mut self, irtifa: u16) {
        if irtifa >= self.irtifa {
            return;
        }
        let hajm = usize::from(self.ard).saturating_mul(usize::from(irtifa));
        self.bayt.truncate(hajm);
        self.bayt.shrink_to_fit();
        self.irtifa = irtifa;
    }
}

/// The packer: pages, one shelf allocator each, and what is live in them.
pub struct Rasif {
    khiyarat: KhiyaratRasf,
    namat: NamatSafha,
    ard: u16,
    irtifa: u16,
    safahat: Vec<Safha>,
    muwazziun: Vec<AtlasAllocator>,
    /// Per page, the allocations this packer believes are live, and the glyph
    /// size inside each.
    ///
    /// `etagere` asserts on a stale or repeated identifier rather than
    /// returning an error, and an assert inside a game process is a crash the
    /// player blames on the game. This map is what makes
    /// [`Rasif::harrir`] total: an identifier that is not in it is ignored, so
    /// the allocator is only ever handed identifiers it issued and has not yet
    /// reclaimed.
    hayya: Vec<FxHashMap<u32, (u16, u16)>>,
    /// Per page, one past the bottom-most row any allocation has reached.
    ///
    /// A high-water mark, so [`Rasif::harrir`] does not lower it. That is
    /// deliberate rather than an oversight: this number exists to say how much
    /// of a page a caller may give back, and a mark that fell when a rectangle
    /// was reclaimed would let a page be cropped through a shelf the allocator
    /// still considers open and will hand out again.
    asfal: Vec<u16>,
    /// Texels covered by glyphs, as opposed to texels covered by allocations.
    mustaghal: u64,
}

impl core::fmt::Debug for Rasif {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        matbaa
            .debug_struct("Rasif")
            .field("khiyarat", &self.khiyarat)
            .field("namat", &self.namat)
            .field("ard", &self.ard)
            .field("irtifa", &self.irtifa)
            .field("safahat", &self.safahat.len())
            .field("istighlal", &self.istighlal())
            .finish_non_exhaustive()
    }
}

impl Rasif {
    /// Builds a packer and opens its first page.
    ///
    /// The first page exists from the start: an atlas with no page cannot hold
    /// anything, and a lazily created first page would make the page count a
    /// number that means one thing before the first glyph and another after it,
    /// which is exactly the number [`crate::namu`] counts growth events with.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::AbaadSafhaGhayrSaliha`] when either page dimension is zero,
    /// or when the two multiply past what the shelf allocator counts area in —
    /// `65535x65535` is expressible as a pair of dimensions and is not
    /// expressible as an area, and the allocator asserts on it rather than
    /// reporting it.
    ///
    /// [`KhataLawha::SafahatNafidat`] when the page budget is zero.
    pub fn jadeed(khiyarat: KhiyaratRasf, namat: NamatSafha) -> Natija<Self> {
        if khiyarat.aqsa_safahat == 0 {
            return Err(KhataLawha::SafahatNafidat { adad: 0, hadd: 0 }.into());
        }
        let (ard, irtifa) = abaad_safha(khiyarat)?;

        let mut rasif = Self {
            khiyarat,
            namat,
            ard,
            irtifa,
            safahat: Vec::new(),
            muwazziun: Vec::new(),
            hayya: Vec::new(),
            asfal: Vec::new(),
            mustaghal: 0,
        };
        rasif.iftah_safha();
        Ok(rasif)
    }

    /// The dimensions this packer *allocates* a page at, after the
    /// power-of-two policy.
    ///
    /// The maximum, and what every open page currently measures. A page that has
    /// finished being packed can usually be finished at
    /// [`Rasif::irtifa_lazim`] instead.
    #[must_use]
    pub const fn abaad(&self) -> (u16, u16) {
        (self.ard, self.irtifa)
    }

    /// The height one page still needs once packing is over.
    ///
    /// The bottom-most row the allocator handed out on that page — gutters
    /// included, because the gutter below the last shelf is the row a bilinear
    /// tap at the bottom edge of those glyphs reads — put through the same
    /// power-of-two policy the maximum went through, and floored at one row
    /// because a page that exists has to have one.
    ///
    /// The policy is applied *upward* here where [`KhiyaratRasf::quwwat_ithnayn`]
    /// applies it downward. The two are the same rule seen from opposite sides:
    /// a maximum may not be exceeded, so it rounds down to the largest power of
    /// two that fits; a requirement may not be undercut, so it rounds up to the
    /// smallest that holds it. Rounding a requirement down would crop through
    /// the glyphs it was measured from.
    ///
    /// A page index past the last page reports the allocation height, which is
    /// the answer that cannot shrink anything that does exist.
    #[must_use]
    pub fn irtifa_lazim(&self, safha: u16) -> u16 {
        let mustaghal = self
            .asfal
            .get(usize::from(safha))
            .copied()
            .unwrap_or(self.irtifa);
        bud_lazim(mustaghal, self.irtifa, self.khiyarat.quwwat_ithnayn)
    }

    /// What every page in this packer holds.
    #[must_use]
    pub const fn namat(&self) -> NamatSafha {
        self.namat
    }

    /// The options this packer was built with.
    #[must_use]
    pub const fn khiyarat(&self) -> &KhiyaratRasf {
        &self.khiyarat
    }

    /// Finds a place for a rectangle, opening a page if it has to.
    ///
    /// Returns `(page index, glyph x, glyph y, allocation identifier)`. The
    /// coordinates are where the *glyph* goes — the gutter has already been
    /// stepped over — so the pair can be handed straight to
    /// [`Rasif::irsim_fi`]. The identifier is what [`Rasif::harrir`] reclaims
    /// the rectangle with.
    ///
    /// A bare rectangle has no glyph identity, so a failure from this function
    /// names glyph 0 of font 0. A caller that holds a key should use
    /// [`Rasif::khassis_li`], which names the letter that did not fit.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::AbaadSafhaGhayrSaliha`] for a rectangle with a zero extent.
    ///
    /// [`KhataLawha::ShaklAkbarMinSafha`] when the rectangle plus its gutter is
    /// larger than a whole page. This is never resolved by scaling: a glyph
    /// drawn smaller than the advance it was measured with produces a line that
    /// no longer fits the box the overflow report cleared it for.
    ///
    /// [`KhataLawha::SafahatNafidat`] when every page the budget allows is open
    /// and none of them has room.
    pub fn khassis(&mut self, ard: u16, irtifa: u16) -> Natija<(u16, u16, u16, u32)> {
        self.khassis_li(None, ard, irtifa)
    }

    /// [`Rasif::khassis`], with the glyph identity carried into any failure.
    ///
    /// # Errors
    ///
    /// As [`Rasif::khassis`].
    pub fn khassis_li(
        &mut self,
        miftah: Option<MiftahShakl>,
        ard: u16,
        irtifa: u16,
    ) -> Natija<(u16, u16, u16, u32)> {
        if ard == 0 || irtifa == 0 {
            return Err(KhataLawha::AbaadSafhaGhayrSaliha { ard, irtifa }.into());
        }

        let hashw = u32::from(self.khiyarat.hashw);
        let hashi = hashw.saturating_mul(2);
        let ard_kamil = u32::from(ard).saturating_add(hashi);
        let irtifa_kamil = u32::from(irtifa).saturating_add(hashi);

        if ard_kamil > u32::from(self.ard) || irtifa_kamil > u32::from(self.irtifa) {
            let (khatt, muarrif) = miftah.map_or((0, 0), |m| (m.khatt, m.muarrif));
            return Err(KhataLawha::ShaklAkbarMinSafha {
                khatt,
                muarrif,
                ard: ila_bud(ard_kamil),
                irtifa: ila_bud(irtifa_kamil),
                hadd_ard: self.ard,
                hadd_irtifa: self.irtifa,
            }
            .into());
        }

        // Both fit in a u16 by the check above, and the allocator counts in i32.
        let matlub: Size = size2(ila_sahih(ard_kamil), ila_sahih(irtifa_kamil));

        for fahras in 0..self.safahat.len() {
            if let Some(mawdi) = self.hawil(fahras, matlub, ard, irtifa) {
                return Ok(mawdi);
            }
        }

        // Every open page is full. Open another, if the budget allows one.
        let adad = ila_raqm_safha(self.safahat.len());
        if adad >= self.khiyarat.aqsa_safahat {
            return Err(KhataLawha::SafahatNafidat {
                adad,
                hadd: self.khiyarat.aqsa_safahat,
            }
            .into());
        }
        let fahras = self.iftah_safha();
        self.hawil(fahras, matlub, ard, irtifa).ok_or_else(|| {
            // A rectangle that fits the page dimensions cannot fail to fit an
            // empty page. Reaching here means the two checks disagree, which is
            // a defect in this module rather than a capacity problem.
            let (khatt, muarrif) = miftah.map_or((0, 0), |m| (m.khatt, m.muarrif));
            KhataLawha::ShaklAkbarMinSafha {
                khatt,
                muarrif,
                ard: ila_bud(ard_kamil),
                irtifa: ila_bud(irtifa_kamil),
                hadd_ard: self.ard,
                hadd_irtifa: self.irtifa,
            }
            .into()
        })
    }

    /// Gives one page's allocator a chance at a rectangle.
    ///
    /// Returns the glyph's own position, inside the gutter, and records the
    /// allocation so [`Rasif::harrir`] can reclaim it later.
    fn hawil(
        &mut self,
        fahras: usize,
        matlub: Size,
        ard: u16,
        irtifa: u16,
    ) -> Option<(u16, u16, u16, u32)> {
        let hashw = self.khiyarat.hashw;
        let muwazzi = self.muwazziun.get_mut(fahras)?;
        let takhsees = muwazzi.allocate(matlub)?;

        let s = u16::try_from(takhsees.rectangle.min.x).ok();
        let a = u16::try_from(takhsees.rectangle.min.y).ok();
        let (Some(s), Some(a)) = (s, a) else {
            // The allocator returned a corner outside the page it was built
            // with. Give the rectangle straight back rather than recording a
            // position nothing can draw into.
            muwazzi.deallocate(takhsees.id);
            return None;
        };

        let hawiya = takhsees.id.serialize();
        let _ = self.hayya.get_mut(fahras)?.insert(hawiya, (ard, irtifa));
        self.mustaghal = self.mustaghal.saturating_add(masaha(ard, irtifa));

        // Taken from the allocator's own rectangle rather than derived from the
        // glyph and the gutter: the rectangle is what the shelf actually
        // occupies, and a number recomputed from two other numbers is a number
        // that can disagree with it.
        if let Some(qa) = self.asfal.get_mut(fahras) {
            let hadd = u16::try_from(takhsees.rectangle.max.y).unwrap_or(self.irtifa);
            *qa = (*qa).max(hadd.min(self.irtifa));
        }

        Some((
            ila_raqm_safha(fahras),
            s.saturating_add(hashw),
            a.saturating_add(hashw),
            hawiya,
        ))
    }

    /// Opens a page and returns its index.
    fn iftah_safha(&mut self) -> usize {
        let khiyarat = AllocatorOptions {
            // Written out rather than defaulted: these three values are the
            // whole of the allocator's configurable behaviour, and a change to
            // `etagere`'s defaults must not move every rectangle in every patch
            // Taarib has ever compiled.
            alignment: size2(1, 1),
            vertical_shelves: false,
            num_columns: 1,
        };
        let qiyas: Size = size2(i32::from(self.ard), i32::from(self.irtifa));
        self.safahat
            .push(Safha::jadeeda(self.ard, self.irtifa, self.namat));
        self.muwazziun
            .push(AtlasAllocator::with_options(qiyas, &khiyarat));
        self.hayya.push(FxHashMap::default());
        self.asfal.push(0);
        self.safahat.len().saturating_sub(1)
    }

    /// Reclaims one rectangle and clears its texels.
    ///
    /// An identifier this packer did not issue, or has already reclaimed, is
    /// ignored. That is deliberate: `etagere` asserts on a stale identifier, and
    /// an assert raised inside somebody's game is a crash they will blame on the
    /// game rather than on the patch.
    ///
    /// The texels are zeroed rather than left as they were. Shelf packing reuses
    /// a freed item for the next rectangle that fits it, and the next rectangle
    /// is usually *smaller*; without the clear, the old letter's tail survives
    /// inside the new rectangle's gutter and bleeds into it under bilinear
    /// filtering — the same defect the gutter exists to prevent, arriving by the
    /// other door.
    pub fn harrir(&mut self, safha: u16, muarrif_takhsees: u32) {
        let fahras = usize::from(safha);
        let Some(hayya) = self.hayya.get_mut(fahras) else {
            return;
        };
        let Some((ard, irtifa)) = hayya.remove(&muarrif_takhsees) else {
            return;
        };
        self.mustaghal = self.mustaghal.saturating_sub(masaha(ard, irtifa));

        let Some(muwazzi) = self.muwazziun.get_mut(fahras) else {
            return;
        };
        // Live in this packer's own record, so the identifier is one the
        // allocator issued and has not reclaimed: `get` and `deallocate` are
        // both being handed exactly what they document as valid.
        let hawiya = AllocId::deserialize(muarrif_takhsees);
        let mustatil = muwazzi.get(hawiya);
        muwazzi.deallocate(hawiya);

        let s0 = u16::try_from(mustatil.min.x).unwrap_or(0);
        let a0 = u16::try_from(mustatil.min.y).unwrap_or(0);
        let s1 = u16::try_from(mustatil.max.x).unwrap_or(0);
        let a1 = u16::try_from(mustatil.max.y).unwrap_or(0);
        if let Some(waraqa) = self.safahat.get_mut(fahras) {
            imsah_mustatil(waraqa, s0, a0, s1.saturating_sub(s0), a1.saturating_sub(a0));
        }
    }

    /// Draws one rasterized glyph into a page.
    ///
    /// `s` and `a` are the glyph's own top-left corner — what
    /// [`Rasif::khassis`] returned, gutter already stepped over.
    ///
    /// Everything is checked before a single byte is written. A blit that
    /// validated as it went could leave half a letter in a page after failing,
    /// and a half-written glyph is worse than a refused one: it is a defect that
    /// survives into the compiled patch and draws.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::AbaadSafhaGhayrSaliha`] when the page does not exist, when
    /// the glyph does not lie inside it, or when the bitmap's length disagrees
    /// with the dimensions it declares.
    ///
    /// [`KhataLawha::NamatMukhtalif`] when a coverage bitmap is written into a
    /// distance-field page or the reverse. The two encode different things in
    /// the same byte, and mixing them produces a page that is wrong in a way no
    /// shader can detect.
    pub fn irsim_fi(&mut self, safha: u16, s: u16, a: u16, surah: &SurahHarf) -> Natija<()> {
        let namat_safha = self.namat;
        if !yutabiq(namat_safha, surah.namat) {
            return Err(KhataLawha::NamatMukhtalif {
                safha: namat_safha.bayt(),
                surah: ramz_rasm(surah.namat),
            }
            .into());
        }
        if surah.khali() {
            // A space, a joiner, a mark the font draws nothing for. Roughly a
            // fifth of the glyphs in a line of Arabic prose have no outline;
            // writing nothing is the correct result, not a failure.
            return Ok(());
        }

        let fahras = usize::from(safha);
        let hadaf = self
            .safahat
            .get_mut(fahras)
            .ok_or(KhataLawha::AbaadSafhaGhayrSaliha { ard: 0, irtifa: 0 })?;

        let ard = u16::try_from(surah.ard).unwrap_or(u16::MAX);
        let irtifa = u16::try_from(surah.irtifa).unwrap_or(u16::MAX);
        let daakhil = u32::from(s).saturating_add(u32::from(ard)) <= u32::from(hadaf.ard)
            && u32::from(a).saturating_add(u32::from(irtifa)) <= u32::from(hadaf.irtifa);
        let tul = usize::from(ard).saturating_mul(usize::from(irtifa));
        if !daakhil || tul == 0 || surah.bayt.len() != tul {
            return Err(KhataLawha::AbaadSafhaGhayrSaliha { ard, irtifa }.into());
        }

        let khatwa = usize::from(hadaf.ard);
        let s0 = usize::from(s);
        let ard_h = usize::from(ard);
        for satr in 0..u32::from(irtifa) {
            let Some(masdar) = surah.satr(satr) else {
                break;
            };
            let hadaf_satr = usize::from(a).saturating_add(usize::try_from(satr).unwrap_or(0));
            let bidaya = hadaf_satr.saturating_mul(khatwa).saturating_add(s0);
            let nihaya = bidaya.saturating_add(ard_h);
            if let Some(makan) = hadaf.bayt.get_mut(bidaya..nihaya)
                && makan.len() == masdar.len()
            {
                makan.copy_from_slice(masdar);
            }
        }
        Ok(())
    }

    /// The pages, in order.
    #[must_use]
    pub fn safahat(&self) -> &[Safha] {
        &self.safahat
    }

    /// The pages, mutably, for a caller compressing or uploading them in place.
    pub fn safahat_mut(&mut self) -> &mut [Safha] {
        &mut self.safahat
    }

    /// How many pages are open.
    #[must_use]
    pub const fn adad_safahat(&self) -> usize {
        self.safahat.len()
    }

    /// Total bytes the pages occupy.
    #[must_use]
    pub fn bayt(&self) -> u64 {
        self.safahat
            .iter()
            .map(|safha| u64::try_from(safha.bayt.len()).unwrap_or(0))
            .sum()
    }

    /// The fraction of allocated area that glyph pixels actually cover.
    ///
    /// The denominator is what the shelf allocator handed out, gutters and shelf
    /// rounding included; the numerator is the glyphs themselves. A number well
    /// below one is not waste to be optimised away — it is the price of the
    /// gutter and of shelves quantized to a common height — but a number that
    /// *falls* between two compiles of the same project means the glyph set
    /// grew a size range that packs badly, which is worth seeing.
    ///
    /// Zero when nothing has been allocated.
    #[must_use]
    pub fn istighlal(&self) -> f32 {
        let mukhassas: u64 = self
            .muwazziun
            .iter()
            .map(|muwazzi| u64::try_from(muwazzi.allocated_space()).unwrap_or(0))
            .sum();
        if mukhassas == 0 {
            return 0.0;
        }
        ila_kasr(self.mustaghal) / ila_kasr(mukhassas)
    }

    /// Empties every page and reclaims every rectangle, keeping one page.
    ///
    /// The allocations are dropped rather than deallocated one by one, and the
    /// page buffers are refilled with zeros rather than freed, so an atlas that
    /// is cleared between levels does not return its memory to the allocator and
    /// ask for it again a moment later inside a game's frame budget.
    pub fn amsah(&mut self) {
        self.safahat.truncate(1);
        self.muwazziun.truncate(1);
        self.hayya.truncate(1);
        self.asfal.truncate(1);
        self.asfal.fill(0);
        for safha in &mut self.safahat {
            safha.bayt.fill(0);
        }
        for muwazzi in &mut self.muwazziun {
            muwazzi.clear();
        }
        for hayya in &mut self.hayya {
            hayya.clear();
        }
        self.mustaghal = 0;
        if self.safahat.is_empty() {
            let _ = self.iftah_safha();
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The dimensions a set of options *allocates* a page at, without building a
/// packer.
///
/// A caller sizing a byte budget needs to know what one page will cost before it
/// can decide how many pages the budget allows, and a page's cost is not
/// `aqsa_ard * aqsa_irtifa` whenever the power-of-two policy is on.
///
/// This is the allocation size and stays the allocation size. It is what a
/// *runtime* atlas costs — `crate::namu::LawhaHayya` opens pages at it and never
/// crops, because a runtime atlas exists to hold glyphs nobody has seen yet —
/// and it is deliberately not what a finished compiled page measures. For that,
/// see [`Rasif::irtifa_lazim`] and [`Safha::iqtati`].
///
/// # Errors
///
/// [`KhataLawha::AbaadSafhaGhayrSaliha`], exactly as [`Rasif::jadeed`].
pub fn abaad_masmuha(khiyarat: KhiyaratRasf) -> Natija<(u16, u16)> {
    abaad_safha(khiyarat)
}

/// Resolves the page dimensions from the options, applying the power-of-two
/// policy and refusing dimensions the allocator cannot work with.
///
/// The maximum, and deliberately so: this is what a page is *opened* at, and a
/// page opened at anything less than the maximum is a page some later glyph
/// overflows into a second one — which costs a whole page to save a few rows,
/// and cannot be undone once the pack has been spread across two. What a page
/// finishes at is [`Rasif::irtifa_lazim`], which is measurable only after
/// packing and is applied there.
fn abaad_safha(khiyarat: KhiyaratRasf) -> Natija<(u16, u16)> {
    let ard = bud_masmuh(khiyarat.aqsa_ard, khiyarat.quwwat_ithnayn);
    let irtifa = bud_masmuh(khiyarat.aqsa_irtifa, khiyarat.quwwat_ithnayn);
    // The allocator keeps its area in an i32 and asserts on an overflow rather
    // than reporting one, so the multiplication is checked here where it can
    // still become a sentence.
    let masaha_kulliya = i32::from(ard).checked_mul(i32::from(irtifa));
    if ard == 0 || irtifa == 0 || masaha_kulliya.is_none() {
        return Err(KhataLawha::AbaadSafhaGhayrSaliha { ard, irtifa }.into());
    }
    Ok((ard, irtifa))
}

/// The largest usable dimension no greater than `bud`.
const fn bud_masmuh(bud: u16, quwwat_ithnayn: bool) -> u16 {
    if !quwwat_ithnayn || bud == 0 || bud.is_power_of_two() {
        return bud;
    }
    // `bud` is at least one, so it has at most fifteen leading zeros and the
    // shift stays inside a u16.
    1u16 << (15u32.saturating_sub(bud.leading_zeros()))
}

/// The smallest usable dimension no smaller than `mustaghal`, and never above
/// `aqsa`.
///
/// The upward counterpart of [`bud_masmuh`]. `aqsa` has already been through
/// that function, so under the power-of-two policy it is itself a power of two
/// and the rounded requirement cannot pass it; the clamp is there for the other
/// policy, where `aqsa` is whatever the caller declared.
const fn bud_lazim(mustaghal: u16, aqsa: u16, quwwat_ithnayn: bool) -> u16 {
    // A page that exists has at least one row, whether or not anything was ever
    // packed into it.
    let lazim = if mustaghal == 0 { 1 } else { mustaghal };
    if lazim >= aqsa {
        return aqsa;
    }
    if !quwwat_ithnayn {
        return lazim;
    }
    match lazim.checked_next_power_of_two() {
        Some(mudawwar) if mudawwar <= aqsa => mudawwar,
        _ => aqsa,
    }
}

/// Whether a page's declared mode and a bitmap's are the same thing.
const fn yutabiq(safha: NamatSafha, surah: NamatRasm) -> bool {
    matches!(
        (safha, surah),
        (NamatSafha::Taghtiya, NamatRasm::Taghtiya)
            | (NamatSafha::Masafa, NamatRasm::Masafa { .. })
    )
}

/// The page-mode byte a rasterization mode corresponds to, for diagnostics.
const fn ramz_rasm(namat: NamatRasm) -> u8 {
    match namat {
        NamatRasm::Taghtiya => NamatSafha::Taghtiya.bayt(),
        NamatRasm::Masafa { .. } => NamatSafha::Masafa.bayt(),
    }
}

/// Zeroes a rectangle of a page.
fn imsah_mustatil(safha: &mut Safha, s: u16, a: u16, ard: u16, irtifa: u16) {
    if ard == 0 || irtifa == 0 {
        return;
    }
    let khatwa = usize::from(safha.ard);
    let s0 = usize::from(s);
    let ard_h = usize::from(ard);
    for satr in 0..usize::from(irtifa) {
        let bidaya = usize::from(a)
            .saturating_add(satr)
            .saturating_mul(khatwa)
            .saturating_add(s0);
        let nihaya = bidaya.saturating_add(ard_h);
        if let Some(makan) = safha.bayt.get_mut(bidaya..nihaya) {
            makan.fill(0);
        }
    }
}

/// The area of a rectangle, in texels.
fn masaha(ard: u16, irtifa: u16) -> u64 {
    u64::from(ard).saturating_mul(u64::from(irtifa))
}

/// A padded dimension, clamped into the range an error can report.
fn ila_bud(qeema: u32) -> u16 {
    u16::try_from(qeema).unwrap_or(u16::MAX)
}

/// A padded dimension as the allocator's signed extent.
fn ila_sahih(qeema: u32) -> i32 {
    i32::try_from(qeema).unwrap_or(i32::MAX)
}

/// A page index as the `u16` every glyph position carries.
fn ila_raqm_safha(fahras: usize) -> u16 {
    u16::try_from(fahras).unwrap_or(u16::MAX)
}

/// The `f32` form of an area, for the occupancy ratio.
#[expect(
    clippy::cast_precision_loss,
    reason = "areas are bounded by 65535 * 65535 * the page budget; the ratio this feeds is a \
              diagnostic, and a few parts per million of rounding in it changes no decision"
)]
const fn ila_kasr(qeema: u64) -> f32 {
    qeema as f32
}
