//! النقل — the cell transport: shaped glyph identifiers to cell indices.
//!
//! Read `taarib_lawha::naql`'s header before this one. The argument is the same
//! argument and it is made there at length; what follows is what changes when the
//! address space is a cell index in a baked atlas rather than a private-use
//! codepoint in a bitmap font.
//!
//! ## What this is
//!
//! *Resident Evil 4* draws one image per code point out of a fixed grid. It has
//! no shaper, so writing Arabic into `BIO4/text/*.dct` as ordinary code points
//! produces a row of unjoined isolated letters in the wrong order. The real text
//! is therefore shaped **first**, by `taarib-saff`, using the real font's `GSUB`
//! and `GPOS`; each distinct shaped glyph is given a cell in the rebuilt atlas;
//! and the game is handed the sequence of those cells.
//!
//! ## Decision 1 forbids Unicode presentation forms. This is not that.
//!
//! A presentation-form pipeline maps code points to code points and has to
//! *re-derive* the shaping from a table that knows only a letter and its
//! neighbours — losing every ligature, every contextual alternate and every mark
//! attachment the font would have made, while producing a string that still
//! looks like text and sorts, searches and reads wrong.
//!
//! This maps shaped glyph identifiers to cell indices. The shaping already
//! happened. A cell index is an integer into one atlas this build generated, and
//! it means nothing anywhere else.
//!
//! ## The invariants, and where they are structural
//!
//! 1. **A key cannot contain a character.** [`MiftahKhanaBio4`] has four fields
//!    — font index, size, base glyph id, composed mark glyph id — and no
//!    character field. There is no constructor that takes a `char`, so nothing
//!    here can allocate a cell *for a character*; the function that would does
//!    not exist.
//! 2. **There is no inverse into text.** [`TawzeeKhanatBio4::miftah`] maps a cell
//!    back to `(font, size, glyph id)` for diagnostics. Nothing maps a cell to a
//!    character, to a string, or to a string-table entry.
//! 3. **A transported sequence never prints as text.** [`NassManqulBio4`] hides
//!    its cells behind a manual `Debug` that writes `#12` and never a glyph, so a
//!    log line cannot produce something a reader might copy out as Arabic.
//! 4. **Cell zero is never allocated.** Every one of the thirty-two shipped fonts
//!    encodes its first cell as blank, and the game's own text depends on that.
//!    Allocation starts at one.
//!
//! ## What is lost, said plainly
//!
//! A string that has gone through this is **no longer text**. It is not
//! searchable, not selectable, not copyable, not readable by a screen reader, and
//! cannot be re-shaped or concatenated. Those are the same costs
//! `taarib_lawha::naql` lists, and they are why this is the last rung: *Resident
//! Evil 4* is a 2005 engine with a baked atlas and no font file, and there is no
//! rung under this one.
//!
//! ## Determinism
//!
//! Cell *n* is the *n*-th key of a `BTreeSet` walked in ascending
//! `(font, size, base glyph id, mark)` order. Nothing depends on the order
//! strings were registered in, on a hash seed, or on hash-map iteration — there
//! is no hash map in this file.
//!
//! ## What a cell has to be asked for by, and where this does not yet agree
//!
//! [`crate::kharita`] now holds the other half of this: the game selects a cell by
//! looking a code point up in a table inside `bio4.exe`, and a cell's number is
//! that code point's *index* in the table. So the sequence this module produces is
//! written into `BIO4/text/*.dct` as the code points whose indices those cells are
//! — `kharita::ramz_khana` is that conversion.
//!
//! Two things follow, and the second is a gap this module still has:
//!
//! 1. The budget passed to [`NaqlBio4::jadeed`] is bounded by the *table*, not
//!    only by the atlas. A Latin build addresses 260 cells however large the
//!    texture is, unless the executable is patched.
//! 2. **Allocation is contiguous and the table is not.** Five Latin indices — 32,
//!    183, 185, 187 and 189 — repeat a code point an earlier index already claimed
//!    and can therefore never be asked for. This module hands out `1, 2, 3, …` and
//!    would put a glyph in one of them. `kharita::khanat_hayya` is the sequence
//!    that skips them, and making the pool walk that instead of `AWWAL_KHANA..` is
//!    the change the dictionary writer will need. It is not made here, because
//!    renumbering cells for a writer that does not exist yet would be choosing the
//!    numbering twice.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use taarib_saff::natija::{Harf, TakhtitNass};

use crate::khata::{KhataBio4, tul_u64};

/// The most glyphs one registered line may carry.
///
/// A line of a *Resident Evil 4* subtitle is tens of glyphs. Four thousand is far
/// past every one of them, and the ceiling is what stops a patch a stranger
/// produced from making this module reserve memory proportional to a number it
/// declared.
pub const AQSA_HURUF_NASS: u64 = 4096;

/// The most lines one transport may carry.
pub const AQSA_NUSUS: u64 = 100_000;

/// The first cell a transport may allocate.
///
/// Cell zero is the blank every shipped font keeps. See invariant 4.
pub const AWWAL_KHANA: u32 = 1;

/// What makes one transported cell distinct from another.
///
/// Four fields, and deliberately not five. There is no character field and no
/// constructor that takes one; that is invariant 1, and it is why this type is
/// declared before anything that allocates.
///
/// ## Why a mark is part of the key
///
/// A `BIO4` cell has one image and one horizontal span, and the span is the only
/// advance the container carries. A combining mark's advance is zero, which no
/// span can express: a cell wide enough to draw is a cell that moves the pen, so
/// a mark given its own cell lands *after* the letter it belongs over instead of
/// on it. That is not a rounding error, it is the container having no way to say
/// what `GPOS` said.
///
/// So a mark does not get a cell. It is composed into its base's cell, at the
/// offset `GPOS` placed it, and the key of that cell is the pair. `عَ` and `عُ`
/// are two cells of one letter, which is the same trade the atlas already makes
/// for the four contextual forms — and it is the only reading under which a
/// vocalised word draws correctly.
///
/// The field order is the **allocation order** and is load-bearing: the derived
/// `Ord` compares font index, then size, then base glyph, then mark, and freezing
/// walks that order. Reordering these fields would renumber every cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MiftahKhanaBio4 {
    /// Index into the patch's font chain.
    pub khatt: u8,
    /// The pixel size in quarter-pixels, as `taarib_lawha` records it.
    pub hajm_rubi: u16,
    /// The base glyph identifier, as shaping produced it.
    pub muarrif: u32,
    /// The combining mark composed into the same cell, if there is one.
    pub alama: Option<u32>,
}

impl MiftahKhanaBio4 {
    /// A key for a bare glyph.
    #[must_use]
    pub const fn jadeed(khatt: u8, hajm_rubi: u16, muarrif: u32) -> Self {
        Self { khatt, hajm_rubi, muarrif, alama: None }
    }

    /// The key for one shaped glyph at one size.
    #[must_use]
    pub const fn min_harf(harf: &Harf, hajm_rubi: u16) -> Self {
        Self { khatt: harf.khatt, hajm_rubi, muarrif: harf.muarrif, alama: None }
    }

    /// The same cell with a mark composed into it.
    #[must_use]
    pub const fn bi_alama(self, alama: u32) -> Self {
        Self { alama: Some(alama), ..self }
    }
}

impl fmt::Display for MiftahKhanaBio4 {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(matbaa, "font {} glyph {} at {}q", self.khatt, self.muarrif, self.hajm_rubi)?;
        match self.alama {
            Some(alama) => write!(matbaa, " with mark {alama}"),
            None => Ok(()),
        }
    }
}

/// One line, as the sequence of cells that draws it.
///
/// **This is not text.** It is a sequence of addresses into one generated atlas,
/// in visual order, and it has no meaning outside the build that produced it.
#[derive(Clone, PartialEq, Eq)]
pub struct NassManqulBio4 {
    khanat: Vec<u32>,
}

impl NassManqulBio4 {
    /// The cells, in the order the game draws them.
    #[must_use]
    pub fn khanat(&self) -> &[u32] {
        &self.khanat
    }

    /// How many cells the line draws.
    #[must_use]
    pub const fn tul(&self) -> usize {
        self.khanat.len()
    }
}

impl fmt::Debug for NassManqulBio4 {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Invariant 3: cells, never glyphs. A diagnostics bundle must not be a
        // place someone can copy a run of Arabic out of.
        matbaa.write_str("NassManqulBio4[")?;
        for (fahras, khana) in self.khanat.iter().enumerate() {
            if fahras > 0 {
                matbaa.write_str(" ")?;
            }
            write!(matbaa, "#{khana}")?;
        }
        matbaa.write_str("]")
    }
}

/// The frozen assignment of cells to shaped glyphs.
#[derive(Debug, Clone, Default)]
pub struct TawzeeKhanatBio4 {
    tawzee: BTreeMap<MiftahKhanaBio4, u32>,
    aks: BTreeMap<u32, MiftahKhanaBio4>,
}

impl TawzeeKhanatBio4 {
    /// The cell a glyph was given.
    #[must_use]
    pub fn khana(&self, miftah: MiftahKhanaBio4) -> Option<u32> {
        self.tawzee.get(&miftah).copied()
    }

    /// What a cell holds, for diagnostics.
    ///
    /// `(font, size, glyph id)`, which is not a character and cannot be turned
    /// into one. That is invariant 2: this is the only direction back, and it
    /// stops at the shaper's own vocabulary.
    #[must_use]
    pub fn miftah(&self, khana: u32) -> Option<MiftahKhanaBio4> {
        self.aks.get(&khana).copied()
    }

    /// Every assignment, in ascending key order.
    pub fn tawzee(&self) -> impl Iterator<Item = (MiftahKhanaBio4, u32)> + '_ {
        self.tawzee.iter().map(|(miftah, khana)| (*miftah, *khana))
    }

    /// How many cells are assigned.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.tawzee.len()
    }

    /// One past the highest cell assigned, which is how many cells the atlas
    /// needs including the reserved blank.
    #[must_use]
    pub fn madaa(&self) -> u32 {
        self.aks.keys().next_back().map_or(AWWAL_KHANA, |akhir| akhir.saturating_add(1))
    }
}

/// Where a composed mark sits, in whole texels, relative to its base's origin.
///
/// Positive `a` is **down**, which is the direction a texture grows and the
/// opposite of the direction `GPOS` measures in. Converting once, here, is what
/// stops the sign from being decided twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct IzahatAlama {
    /// Rightwards from the base's pen position.
    pub s: i32,
    /// Downwards from the base's baseline.
    pub a: i32,
}

/// Everything a finished transport hands back.
#[derive(Debug, Clone, Default)]
pub struct NatijatNaqlBio4 {
    /// Which cell each key was given.
    pub tawzee: TawzeeKhanatBio4,
    /// Each registered line, as the cells that draw it.
    pub nusus: Vec<NassManqulBio4>,
    /// Where each composed mark goes inside its base's cell.
    pub izahat: BTreeMap<MiftahKhanaBio4, IzahatAlama>,
    /// Marks whose `GPOS` offset differed between two instances of the same
    /// `(base, mark)` pair.
    ///
    /// One cell can hold one offset. When a font places the same mark over the
    /// same base at two heights — which happens with stacked marks and with
    /// contextual `mkmk` — the first is kept and this counts the rest. A caller
    /// that finds the number unacceptable declines the transport; a caller that
    /// never looked would not have known, which is why the number exists.
    pub alamat_mutanaziaa: u32,
    /// Marks that could not be composed into a base and were dropped.
    ///
    /// A mark with no preceding base in the line, or a second mark over a base
    /// that already carries one. A cell holds one mark, and a base with two would
    /// need a key this container cannot address.
    pub alamat_mafquda: u32,
}

/// A transport under construction: the pool of cells, and the lines to emit.
#[derive(Debug, Clone)]
pub struct NaqlBio4 {
    hawd: BTreeSet<MiftahKhanaBio4>,
    sutur: Vec<Vec<MiftahKhanaBio4>>,
    izahat: BTreeMap<MiftahKhanaBio4, IzahatAlama>,
    alamat_mutanaziaa: u32,
    alamat_mafquda: u32,
    saa: u32,
}

impl NaqlBio4 {
    /// A transport that may allocate `saa` cells in total, blank included.
    ///
    /// The size comes from the font being replaced: a *Resident Evil 4* atlas is
    /// as many cells as its grid holds and not one more, and a transport that did
    /// not know the budget in advance would discover it after rasterizing
    /// everything.
    #[must_use]
    pub const fn jadeed(saa: u32) -> Self {
        Self {
            hawd: BTreeSet::new(),
            sutur: Vec::new(),
            izahat: BTreeMap::new(),
            alamat_mutanaziaa: 0,
            alamat_mafquda: 0,
            saa,
        }
    }

    /// Registers one laid-out line.
    ///
    /// `hajm_rubi` is the size in quarter-pixels the atlas keys by, which for
    /// this engine is one number for the whole font file.
    ///
    /// The glyphs are taken in the order the layout produced them, which is
    /// **visual** order — `taarib-saff` has already run the bidirectional
    /// algorithm and reordered the line — so the sequence can be written straight
    /// into a game that draws left to right and knows nothing about direction.
    ///
    /// Combining marks are folded into the cell of the glyph they were attached
    /// to rather than given cells of their own; see [`MiftahKhanaBio4`] for why
    /// the container leaves no other option. The mark's own place in the sequence
    /// disappears with it, which is correct: it is drawn as part of its base.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HajmMufrit`] when the line carries more than
    /// [`AQSA_HURUF_NASS`] glyphs or the transport already holds
    /// [`AQSA_NUSUS`] lines.
    pub fn sajjil(&mut self, takhtit: &TakhtitNass, hajm_rubi: u16) -> Result<(), KhataBio4> {
        let tul = tul_u64(takhtit.huruf.len());
        if tul > AQSA_HURUF_NASS {
            return Err(KhataBio4::HajmMufrit {
                haql: "glyphs in one transported line",
                qeema: tul,
                saqf: AQSA_HURUF_NASS,
            });
        }
        if tul_u64(self.sutur.len()) >= AQSA_NUSUS {
            return Err(KhataBio4::HajmMufrit {
                haql: "transported lines",
                qeema: tul_u64(self.sutur.len()).saturating_add(1),
                saqf: AQSA_NUSUS,
            });
        }

        // A mark is attached to its base by **cluster**, not by adjacency. In a
        // right-to-left run the layout is already in visual order, and the
        // reordering puts a mark *before* the letter it sits on — so a reader
        // that took "the glyph in front of it" would attach every Arabic mark to
        // the wrong letter, or to nothing at all at the start of a line. The
        // cluster index survives the reorder, and every mark shares its base's.
        let mut satr: Vec<MiftahKhanaBio4> = Vec::with_capacity(takhtit.huruf.len());
        for harf in &takhtit.huruf {
            if harf.alama {
                continue;
            }
            let mut miftah = MiftahKhanaBio4::min_harf(harf, hajm_rubi);
            let mut wujidat = false;
            for alama in &takhtit.huruf {
                if !alama.alama || alama.anqud != harf.anqud {
                    continue;
                }
                if wujidat {
                    // One cell, one mark. A second would need a key this
                    // container cannot address, and choosing between them would
                    // be guessing which one the reader can do without.
                    self.alamat_mafquda = self.alamat_mafquda.saturating_add(1);
                    continue;
                }
                wujidat = true;
                miftah = miftah.bi_alama(alama.muarrif);
                let izaha =
                    IzahatAlama { s: ila_sahih(alama.s - harf.s), a: ila_sahih(alama.a - harf.a) };
                match self.izahat.get(&miftah) {
                    Some(mawjud) if *mawjud != izaha => {
                        self.alamat_mutanaziaa = self.alamat_mutanaziaa.saturating_add(1);
                    }
                    Some(_) => {}
                    None => {
                        let _ = self.izahat.insert(miftah, izaha);
                    }
                }
            }
            satr.push(miftah);
        }

        // Marks whose cluster holds no base glyph at all. There is nothing to
        // compose them into and they are counted rather than dropped quietly.
        for alama in &takhtit.huruf {
            if alama.alama
                && !takhtit.huruf.iter().any(|harf| !harf.alama && harf.anqud == alama.anqud)
            {
                self.alamat_mafquda = self.alamat_mafquda.saturating_add(1);
            }
        }

        // The pool is fed from the line's *final* keys, after composition. A base
        // that only ever appears carrying a mark must not also occupy a bare
        // cell: cells are the budget, and one wasted per vocalised letter is a
        // font that does not fit.
        for miftah in &satr {
            let _ = self.hawd.insert(*miftah);
        }
        self.sutur.push(satr);
        Ok(())
    }

    /// How many distinct cells the pool needs, blank excluded.
    #[must_use]
    pub fn adad_ashkal(&self) -> usize {
        self.hawd.len()
    }

    /// Marks that could not be composed into a base.
    #[must_use]
    pub const fn alamat_mafquda(&self) -> u32 {
        self.alamat_mafquda
    }

    /// How many lines are waiting to be emitted.
    #[must_use]
    pub const fn adad_nusus(&self) -> usize {
        self.sutur.len()
    }

    /// Freezes the pool into an assignment and emits every registered line.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::KhanatNafida`] when the pool needs more cells than the font's
    /// grid provides. It is never truncated: a truncated assignment draws the
    /// wrong letters, and wrong letters read to a player as a corrupt game rather
    /// than as a limit that was reached.
    pub fn akmil(&self) -> Result<NatijatNaqlBio4, KhataBio4> {
        let matlub = u32::try_from(self.hawd.len())
            .unwrap_or(u32::MAX)
            .saturating_add(AWWAL_KHANA);
        if matlub > self.saa {
            return Err(KhataBio4::KhanatNafida { matlub, mutah: self.saa });
        }

        let mut tawzee = TawzeeKhanatBio4::default();
        for (fahras, miftah) in self.hawd.iter().enumerate() {
            let khana = u32::try_from(fahras).unwrap_or(u32::MAX).saturating_add(AWWAL_KHANA);
            let _ = tawzee.tawzee.insert(*miftah, khana);
            let _ = tawzee.aks.insert(khana, *miftah);
        }

        let mut nusus = Vec::with_capacity(self.sutur.len());
        for satr in &self.sutur {
            let mut khanat = Vec::with_capacity(satr.len());
            for miftah in satr {
                // Every key in a line was inserted into the pool by `sajjil`, so
                // a miss here would mean the pool and the lines disagree, which
                // cannot happen without a field being mutated in between. The
                // blank cell is the answer that keeps a line the right length
                // rather than silently shorter.
                khanat.push(tawzee.khana(*miftah).unwrap_or(0));
            }
            nusus.push(NassManqulBio4 { khanat });
        }

        Ok(NatijatNaqlBio4 {
            tawzee,
            nusus,
            izahat: self.izahat.clone(),
            alamat_mutanaziaa: self.alamat_mutanaziaa,
            alamat_mafquda: self.alamat_mafquda,
        })
    }
}

/// A shaped offset in pixels as whole texels, rounded to nearest.
const fn ila_sahih(qeema: f32) -> i32 {
    if !qeema.is_finite() {
        return 0;
    }
    let mudawwar = qeema.round().clamp(-32768.0, 32767.0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i16 range on the line above, which is well inside i32"
    )]
    let texel = mudawwar as i32;
    texel
}
