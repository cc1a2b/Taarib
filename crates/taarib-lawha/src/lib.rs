//! # لوحة تعريب — the glyph atlas
//!
//! A required glyph set becomes packed single-channel texture pages plus a map
//! that says where every glyph landed. It runs twice in the product's life:
//! offline, when a patch is compiled and the whole set is known, and online,
//! inside a game process, for text the compiler could never have seen.
//!
//! Built in **Phase 2**, directly on top of `taarib-saff`, because the required
//! glyph set cannot be known without shaping.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `tafrigh` | glyph set collection — shape the patch's real strings at the real sizes and collect the glyph ids that actually come out |
//! | `rasf` | shelf packing through `etagere`: padding, page policy, a maximum page dimension, and multi-page output when one page will not hold the set |
//! | `misafa` | exact Euclidean distance transform (Felzenszwalb–Huttenlocher, two passes) over the high-resolution coverage bitmap, encoded to 8 bits with the zero level at 128 |
//! | `khareeta` | the glyph map: page, pixel rectangle, normalized UV rectangle, bearings, advance, the size and mode it was rasterized at, and the font identity it came from |
//! | `namu` (see below) | runtime growth: allocate into free shelf space, add pages up to a budget, and evict by least-recently-used glyph *rectangles* |
//! | `naql` | the glyph-identifier transport: private-use slots for shaped glyph ids, and a bitmap glyph table addressed by them, for an engine whose only way to name a glyph is a character code |
//!
//! ## Why the transport lives here
//!
//! `naql` is not an atlas, and it is here for a structural reason rather than a
//! thematic one. Godot 3 and GameMaker both draw text one image per character
//! code with no shaping, and both need exactly the same thing: shaped glyph ids
//! named by private-use codepoints, with Taarib's own rasterized images behind
//! them. One adapter crate cannot depend on another, and the transport cannot
//! live in `taarib-jisr` because `jisr` already depends on this crate. So it
//! sits beside the atlas whose images it addresses, on `taarib-saff`'s own
//! shaped-glyph type — and it is a transport, never a presentation-form
//! pipeline. Its own header is the argument, and it is worth reading before the
//! module is judged.
//!
//! ## Why the set is collected by shaping, not by enumeration
//!
//! Enumerating Unicode ranges is the obvious way to build an atlas and it is
//! wrong in both directions at once. It misses every ligature and contextual
//! alternate the font would have produced — lam-alef, the `rlig` forms, the
//! `calt` alternates that make Naskh look like Naskh — because those glyphs
//! have no codepoint to enumerate. And it includes thousands of glyphs the game
//! will never draw, which is memory taken from a process that is not ours.
//! Shaping the real strings produces exactly the set that will be asked for,
//! and nothing else.
//!
//! Collection is therefore keyed by `(font identity, glyph id, pixel size,
//! rasterization mode, subpixel bucket)`. Never by codepoint: a codepoint is
//! not a glyph, and treating it as one is the same category error Decision 1
//! exists to prevent.
//!
//! ## Coverage against SDF
//!
//! Coverage pages are sharper at fixed small sizes and are the default for
//! interface text. SDF pages let one atlas serve every size the game asks for,
//! which is what makes auto-sizing text and world-space text work at all. The
//! choice is per patch, is recorded in the patch, and the adapter obeys it
//! without asking — an adapter that decided this for itself would produce a
//! different result from the one the compiler measured, and every overflow
//! report in the patch would become a lie.
//!
//! ## Hard constraints
//!
//! - Packing is deterministic. The same input set produces byte-identical
//!   pages, so recompiling an unchanged project does not churn its own content
//!   hash and invalidate every mirror that already has it.
//! - Pages are single-channel R8. Colour comes from the material, per style
//!   span, at draw time.
//! - No page exceeds the declared maximum dimension — 4096 by default, 2048 on
//!   the conservative profile. The packer splits into another page rather than
//!   scaling a glyph down to fit.
//! - No *compiled* page is shipped at that maximum either. The maximum is what a
//!   page is allocated at, because the shelf allocator has to open a page before
//!   it has seen the set; once the set is packed, [`Lawha::ibni`] returns the
//!   rows nothing reached. A few hundred glyphs occupy the first hundred-odd
//!   rows of a 4096-row page, and shipping the other four thousand would make
//!   every client that downloads the patch allocate sixteen megabytes for a
//!   texture whose ink fits in a fraction of one.
//! - Runtime eviction never invalidates a rectangle referenced by the frame
//!   being built, and the glyph map is updated atomically, so no frame ever
//!   samples a rectangle that has been reassigned underneath it.
//! - Growth events are counted and reported. An atlas that grows constantly at
//!   runtime means the compiler missed strings, and the diagnostics say so
//!   instead of quietly absorbing the cost.

pub mod khareeta;
pub mod khata;
pub mod misafa;
pub mod namu;
pub mod naql;
pub mod rasf;
pub mod tafrigh;

use taarib_saff::khatt::SilsilatKhutut;
use taarib_saff::rasm::{MAWADI_TAHAZZUZ, MihwarQeema, NamatRasm, Rassam, SurahHarf};
use taarib_usus::khata::Natija;

pub use crate::khareeta::{Ihdathiyat, KhareetatAshkal, MawdiShakl, MiftahShakl, NamatSafha};
pub use crate::khata::KhataLawha;
pub use crate::misafa::{KhiyaratMisafa, intishar_munasib};
pub use crate::namu::{IhsaatNamu, LawhaHayya};
pub use crate::naql::{MasdarLawha, Naql, NamatKhana, NatijatNaql, QiyasatNaql, TaqreerNaql};
pub use crate::rasf::{KhiyaratRasf, Rasif, Safha};
pub use crate::tafrigh::JamiAshkal;

/// A compiled atlas: the pages, and the map that says what is in them.
///
/// This is the offline product — what the patch compiler builds once and what a
/// game loads and uploads. Its runtime counterpart is [`LawhaHayya`], which
/// packs glyphs the compiler never saw.
#[derive(Debug, Clone)]
pub struct Lawha {
    /// The pages, in index order.
    pub safahat: Vec<Safha>,
    /// Where every glyph is.
    pub khareeta: KhareetatAshkal,
    /// How the pages were rasterized.
    pub namat: NamatSafha,
}

impl Lawha {
    /// Rasterizes and packs a glyph set.
    ///
    /// The set must come from shaping real strings — [`JamiAshkal`] is how it is
    /// gathered — because a set assembled any other way is either missing the
    /// ligatures the font would have produced or padded with thousands of glyphs
    /// nothing draws.
    ///
    /// Packing order is by descending height, then descending width, then by
    /// key. Height first because that is what shelf packing wants and it can be
    /// the difference between two pages and one; by key last because two
    /// compiles of the same project must produce byte-identical pages, and a set
    /// with ties resolved arbitrarily would not.
    ///
    /// Each finished page is then cropped to the rows the pack reached, which
    /// is the difference between a patch that ships a hundred kilobytes of
    /// glyphs and one that ships a hundred kilobytes of glyphs inside sixteen
    /// megabytes of zeros. The crop happens *after* packing rather than by
    /// opening a smaller page: a smaller page can overflow into a second one,
    /// which costs a whole page to save a few rows, while cropping what was
    /// packed cannot change where anything went. Determinism is unaffected —
    /// the cropped height is a function of the same allocation sequence that
    /// produced the pages.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::KhattKharijSilsila`] when the set names a font the chain
    /// does not hold, which means the atlas and the chain were built from
    /// different patches. [`KhataLawha::ShaklAkbarMinSafha`] for a glyph that
    /// cannot fit a page at all, [`KhataLawha::SafahatNafidat`] when the page
    /// budget runs out, and whatever the rasterizer reports for a glyph it
    /// cannot draw.
    pub fn ibni(
        ashkal: &[MiftahShakl],
        khutut: &SilsilatKhutut,
        khiyarat: KhiyaratRasf,
        misafa: KhiyaratMisafa,
        namat: NamatSafha,
    ) -> Natija<Self> {
        let mut rasif = Rasif::jadeed(khiyarat, namat)?;
        let mut khareeta = KhareetatAshkal::jadeeda();

        // One rasterizer per font, not per glyph: preparing one parses the
        // font's outline tables, and a set of two thousand glyphs over three
        // fonts would otherwise parse them two thousand times.
        let mut rassamun: Vec<Option<Rassam>> = Vec::new();
        rassamun.resize_with(khutut.adad(), || None);

        // Rasterize first, then pack, so that the packer can be handed the set
        // in the order it packs best rather than the order the glyphs arrived.
        let mut suwar: Vec<(MiftahShakl, SurahHarf)> = Vec::with_capacity(ashkal.len());
        for miftah in ashkal {
            let fahras = usize::from(miftah.khatt);
            let mawjud = match rassamun.get(fahras) {
                Some(Some(_)) => true,
                Some(None) => false,
                None => {
                    return Err(KhataLawha::KhattKharijSilsila {
                        khatt: miftah.khatt,
                        adad: u32::try_from(khutut.adad()).unwrap_or(u32::MAX),
                    }
                    .into());
                }
            };
            if !mawjud {
                let Some(khatt) = khutut.khatt(miftah.khatt) else {
                    return Err(KhataLawha::KhattKharijSilsila {
                        khatt: miftah.khatt,
                        adad: u32::try_from(khutut.adad()).unwrap_or(u32::MAX),
                    }
                    .into());
                };
                let rassam = Rassam::jadeed(khatt)?;
                if let Some(khana) = rassamun.get_mut(fahras) {
                    *khana = Some(rassam);
                }
            }
            let Some(Some(rassam)) = rassamun.get(fahras) else {
                return Err(KhataLawha::KhattKharijSilsila {
                    khatt: miftah.khatt,
                    adad: u32::try_from(khutut.adad()).unwrap_or(u32::MAX),
                }
                .into());
            };
            suwar.push((*miftah, irsim_shakl(rassam, *miftah, namat, misafa)?));
        }

        suwar.sort_by(|(miftah_a, surah_a), (miftah_b, surah_b)| {
            surah_b
                .irtifa
                .cmp(&surah_a.irtifa)
                .then(surah_b.ard.cmp(&surah_a.ard))
                .then(miftah_a.cmp(miftah_b))
        });

        for (miftah, surah) in &suwar {
            let mawdi = daa_shakl(&mut rasif, *miftah, surah)?;
            khareeta.daa(*miftah, mawdi);
        }

        // Packing is over, so the pages can stop being the size the allocator
        // needed them to be and become the size the pack turned out to need.
        // The crop is height-only and every glyph keeps the position it was
        // packed at, so nothing above is revisited — but the recorded page
        // dimensions are what normalized texture coordinates are derived from,
        // so they are registered from the cropped page and not from the
        // allocation.
        let mut safahat: Vec<Safha> = rasif.safahat().to_vec();
        for (fahras, safha) in safahat.iter_mut().enumerate() {
            let raqm = u16::try_from(fahras).unwrap_or(u16::MAX);
            safha.iqtati(rasif.irtifa_lazim(raqm));
            khareeta.sajjil_safha(raqm, safha.ard, safha.irtifa);
        }

        Ok(Self { safahat, khareeta, namat })
    }

    /// What the pages cost, in bytes, before compression.
    #[must_use]
    pub fn bayt(&self) -> usize {
        self.safahat.iter().map(Safha::bayt_hajm).sum()
    }

    /// How many glyphs the atlas holds.
    #[must_use]
    pub fn adad_ashkal(&self) -> usize {
        self.khareeta.adad()
    }

    /// How many pages it took.
    #[must_use]
    pub const fn adad_safahat(&self) -> usize {
        self.safahat.len()
    }
}

/// Rasterizes one glyph in whichever mode the atlas is being built in.
///
/// The two modes are not interchangeable and the difference is not cosmetic: a
/// coverage page is gamma-encoded for display, while a distance field must be
/// generated from *linear* coverage or every letter in the patch comes out
/// dilated. Routing both through one function is what stops that decision from
/// being made twice, differently.
fn irsim_shakl(
    rassam: &Rassam,
    miftah: MiftahShakl,
    namat: NamatSafha,
    misafa: KhiyaratMisafa,
) -> Natija<SurahHarf> {
    match namat {
        NamatSafha::Taghtiya => {
            let tahazzuz = f32::from(miftah.bakat) / f32::from(MAWADI_TAHAZZUZ);
            let mawadi: [MihwarQeema; 0] = [];
            rassam.irsim(miftah.muarrif, miftah.hajm(), NamatRasm::Taghtiya, tahazzuz, &mawadi)
        }
        NamatSafha::Masafa => misafa::masafa_shakl(rassam, miftah.muarrif, misafa),
    }
}

/// Allocates a rectangle for a glyph and draws it there.
///
/// A glyph with no image — a space, a joiner, a mark the font draws nothing for
/// — takes no rectangle at all. It still gets a map entry, because its advance
/// is what a line's width is made of, and an adapter that had to special-case a
/// missing entry would be an adapter that eventually forgot to.
fn daa_shakl(rasif: &mut Rasif, miftah: MiftahShakl, surah: &SurahHarf) -> Natija<MawdiShakl> {
    if surah.ard == 0 || surah.irtifa == 0 {
        return Ok(MawdiShakl {
            safha: 0,
            s: 0,
            a: 0,
            ard: 0,
            irtifa: 0,
            izaha_s: 0,
            izaha_a: 0,
            taqaddum: surah.taqaddum,
        });
    }

    let ard = u16::try_from(surah.ard).unwrap_or(u16::MAX);
    let irtifa = u16::try_from(surah.irtifa).unwrap_or(u16::MAX);
    let (safha, s, a, _) = rasif.khassis_li(Some(miftah), ard, irtifa)?;
    rasif.irsim_fi(safha, s, a, surah)?;

    Ok(MawdiShakl {
        safha,
        s,
        a,
        ard,
        irtifa,
        izaha_s: i16::try_from(surah.izaha_s).unwrap_or(i16::MAX),
        izaha_a: i16::try_from(surah.izaha_a).unwrap_or(i16::MAX),
        taqaddum: surah.taqaddum,
    })
}
