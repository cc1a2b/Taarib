//! النتيجة — what comes out: ordered visual lines of positioned glyphs.
//!
//! The output is deliberately flat. Glyphs live in one contiguous vector and a
//! line names a range into it, because every consumer walks it in order and
//! three of them walk it inside a game's render loop: a nested structure would
//! mean a pointer chase per line and an allocation per layout, in the one place
//! this product cannot afford either.
//!
//! Every glyph carries its cluster index back into the logical text. That single
//! field is what makes a caret land between graphemes in a right-to-left
//! selection, what lets a typewriter effect reveal a joined word one letter at a
//! time, and what lets the review console jump from a rendered line back to the
//! string that produced it.

use core::ops::Range;

use crate::talab::Ittijah;

/// A positioned glyph, ready to be drawn.
///
/// Note what is not here: a codepoint. Nothing downstream of shaping is allowed
/// to know what character a glyph came from, because a code path that knows
/// could be tempted to draw from it — which is exactly the presentation-form
/// pipeline Decision 1 exists to make impossible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Harf {
    /// The glyph identifier, in the font at [`Harf::khatt`].
    pub muarrif: u32,
    /// Horizontal position of the glyph's origin, in pixels from the layout's
    /// left edge. Already in visual order: this is where it is drawn.
    pub s: f32,
    /// Vertical position of the glyph's origin, in pixels down from the layout's
    /// top edge.
    pub a: f32,
    /// The advance this glyph contributed, in pixels. Zero for marks.
    pub taqaddum: f32,
    /// Byte offset into the logical clean text of the cluster this glyph belongs
    /// to. Several glyphs share a cluster; a ligature covers several.
    pub anqud: u32,
    /// The style span this glyph inherited, so colour and effects survive
    /// reordering.
    pub nitaq: u16,
    /// Index into the font chain of the font this glyph belongs to.
    pub khatt: u8,
    /// Whether this glyph is a combining mark positioned onto a base rather than
    /// a letter advancing the pen.
    pub alama: bool,
}

/// A laid-out line, in visual order.
#[derive(Debug, Clone, PartialEq)]
pub struct SatrMansuq {
    /// The glyphs of this line, as a range into [`TakhtitNass::huruf`].
    pub huruf: Range<u32>,
    /// The baseline's vertical position, in pixels from the layout's top.
    pub asas: f32,
    /// Where the line box begins horizontally, after alignment.
    pub bidaya: f32,
    /// The measured width of the line's content.
    pub ard: f32,
    /// The line box's height.
    pub irtifa: f32,
    /// How far the tallest content rises above the baseline.
    pub suud: f32,
    /// How far the deepest content falls below it.
    pub hubut: f32,
    /// The byte range of logical text this line covers.
    pub mantiqi: Range<u32>,
    /// The line's own base direction, which is the paragraph's unless an
    /// isolate changed it.
    pub ittijah: Ittijah,
    /// Whether this is the last line of the paragraph, which is what stops
    /// justification stretching a short final line across the full width.
    pub akhir: bool,
    /// How much width justification added to this line, kept so a caller can
    /// tell a justified line from a naturally full one.
    pub dabt: f32,
}

/// A measured rectangle, in the layout's own pixel space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MustatilNass {
    /// Left edge.
    pub s: f32,
    /// Top edge.
    pub a: f32,
    /// Width.
    pub ard: f32,
    /// Height.
    pub irtifa: f32,
}

/// What overflowed, by how much, and where.
///
/// This is not an error. It is a measurement the caller asked for: the patch
/// compiler turns it into the overflow report, the review console sorts by it,
/// and the workspace shows it beside the string as it is typed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaqreerTajawuz {
    /// The widest line's measured width, in pixels.
    pub ard: f32,
    /// The width that was available.
    pub ard_mutah: f32,
    /// The total height the text needed.
    pub irtifa: f32,
    /// The height that was available, when one was given.
    pub irtifa_mutah: Option<f32>,
    /// The first line that exceeded the width.
    pub awwal_satr: u32,
    /// How many lines exceeded it.
    pub adad_sutur: u32,
}

impl TaqreerTajawuz {
    /// How far past the available width the worst line went, in pixels.
    #[must_use]
    pub fn zaid(&self) -> f32 {
        (self.ard - self.ard_mutah).max(0.0)
    }

    /// The overflow as a fraction of the available width, which is what a
    /// severity sort orders by: forty pixels over a button is catastrophic and
    /// forty pixels over a subtitle is invisible.
    #[must_use]
    pub fn nisba(&self) -> f32 {
        if self.ard_mutah <= 0.0 {
            0.0
        } else {
            self.zaid() / self.ard_mutah
        }
    }

    /// Whether the text also ran past the height it was given.
    #[must_use]
    pub fn tajawuz_irtifa(&self) -> bool {
        self.irtifa_mutah.is_some_and(|mutah| self.irtifa > mutah)
    }
}

/// What the font chain could not draw, counted from what shaping produced.
///
/// Not an error, and not a reason to refuse a layout. It is the answer to a
/// question that previously had no answer anywhere in the product: a character
/// that no font in the chain covers shapes to glyph 0, and glyph 0 is the empty
/// rectangle a player reads as "this patch is broken". The layout reserves that
/// character's width and draws nothing in it, which is the smaller wrong answer
/// — and this record is how the decision stops being silent, so the patch
/// compiler can name the string and the reviewer can add a font to the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaghtiyaNaqisa {
    /// How many glyphs of this layout came back as `.notdef`.
    ///
    /// Glyphs and not characters: a character the chain cannot draw produces one
    /// `.notdef` per place it appears, and a layout drawn at several sizes counts
    /// each of them, because each is a place a rectangle would have been.
    pub adad: u32,
    /// The byte offset, in the logical text, of the first cluster that produced
    /// one. The character itself is read from the caller's own string at this
    /// offset — nothing downstream of shaping carries a codepoint, here as
    /// everywhere.
    pub awwal_anqud: u32,
}

/// A finished layout.
///
/// Also a reusable buffer: [`TakhtitNass::amsah`] empties it while keeping its
/// allocations, which is how the ABI's hot path lays out a frame's worth of text
/// without allocating.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TakhtitNass {
    /// Every glyph, in visual order, grouped by line.
    pub huruf: Vec<Harf>,
    /// The lines, in reading order from the top.
    pub sutur: Vec<SatrMansuq>,
    /// The width of the widest line.
    pub ard: f32,
    /// The total height of every line box.
    pub irtifa: f32,
    /// The paragraph's resolved base direction.
    pub ittijah: Ittijah,
    /// The size the text was finally laid out at, which differs from the size
    /// requested when the overflow policy shrank it to fit.
    pub hajm: f32,
    /// Present when the text did not fit.
    pub tajawuz: Option<TaqreerTajawuz>,
    /// Whether the text was truncated by the overflow policy.
    pub maqsus: bool,
    /// Present when the font chain could not draw every character of the text.
    ///
    /// The width of each such character is still reserved in the lines above and
    /// nothing is drawn in it, so a layout carrying this is a layout with holes
    /// in it, not a layout that is wrong.
    pub taghtiya_naqisa: Option<TaghtiyaNaqisa>,
}

impl TakhtitNass {
    /// An empty layout at a given direction and size.
    #[must_use]
    pub const fn farigh(ittijah: Ittijah, hajm: f32) -> Self {
        Self {
            huruf: Vec::new(),
            sutur: Vec::new(),
            ard: 0.0,
            irtifa: 0.0,
            ittijah,
            hajm,
            tajawuz: None,
            maqsus: false,
            taghtiya_naqisa: None,
        }
    }

    /// Empties the layout while keeping every allocation, so the next layout
    /// into this buffer touches the allocator zero times.
    pub fn amsah(&mut self) {
        self.huruf.clear();
        self.sutur.clear();
        self.ard = 0.0;
        self.irtifa = 0.0;
        self.tajawuz = None;
        self.maqsus = false;
        self.taghtiya_naqisa = None;
    }

    /// Whether anything was laid out.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.huruf.is_empty()
    }

    /// The glyphs of one line.
    #[must_use]
    pub fn huruf_satr(&self, satr: &SatrMansuq) -> &[Harf] {
        let bidaya = satr.huruf.start as usize;
        let nihaya = satr.huruf.end as usize;
        self.huruf.get(bidaya..nihaya).unwrap_or(&[])
    }

    /// The layout's bounding box.
    #[must_use]
    pub const fn mustatil(&self) -> MustatilNass {
        MustatilNass {
            s: 0.0,
            a: 0.0,
            ard: self.ard,
            irtifa: self.irtifa,
        }
    }

    /// The line containing a logical byte offset, for mapping a cursor or a
    /// search hit back to where it is drawn.
    #[must_use]
    pub fn satr_ind(&self, mawqi: u32) -> Option<&SatrMansuq> {
        self.sutur
            .iter()
            .find(|satr| mawqi >= satr.mantiqi.start && mawqi < satr.mantiqi.end)
    }

    /// The visual position of a logical byte offset — where a caret goes.
    ///
    /// In mixed-direction text one logical offset can sit at two visual places
    /// at once, at the seam between an Arabic run and a Latin one. This returns
    /// the position on the side of the run that *contains* the offset, which is
    /// the behaviour a person typing expects: the caret stays with the text they
    /// are writing, not with the text they are about to leave.
    #[must_use]
    pub fn mawqi_anqud(&self, mawqi: u32) -> Option<(f32, f32)> {
        let satr = self.satr_ind(mawqi)?;
        // The glyph whose cluster is the closest one at or before the offset:
        // the cluster the caret is inside, rather than the one it is next to.
        let mut afdal: Option<&Harf> = None;
        for harf in self.huruf_satr(satr) {
            if harf.alama || harf.anqud > mawqi {
                continue;
            }
            if afdal.is_none_or(|sabiq| harf.anqud > sabiq.anqud) {
                afdal = Some(harf);
            }
        }
        let harf = afdal?;
        // The caret sits after the glyph in reading order, which is on its left
        // in a right-to-left run and on its right in a left-to-right one.
        let s = if satr.ittijah == Ittijah::Yameen {
            harf.s
        } else {
            harf.s + harf.taqaddum
        };
        Some((s, satr.asas))
    }

    /// The rectangles covering a logical byte range, one per visual run.
    ///
    /// A selection that crosses a direction boundary is genuinely two
    /// rectangles, not one. Drawing it as one is the bug every text field that
    /// has never been used in Arabic ships with.
    #[must_use]
    pub fn mustatilat_tahdid(&self, nitaq: Range<u32>) -> Vec<MustatilNass> {
        // A tenth of a pixel: wide enough to absorb the rounding of accumulated
        // advances, narrow enough that a real gap between two visual runs is
        // never mistaken for one.
        const TASAMUH: f32 = 0.1;

        let mut mustatilat = Vec::new();
        for satr in &self.sutur {
            let mut jari: Option<MustatilNass> = None;
            for harf in self.huruf_satr(satr) {
                let dakhil = harf.anqud >= nitaq.start && harf.anqud < nitaq.end;
                if !dakhil || harf.alama {
                    // A mark adds nothing to a selection rectangle: it sits on a
                    // base that is already covered, and its own advance is zero.
                    if !dakhil && let Some(mustatil) = jari.take() {
                        mustatilat.push(mustatil);
                    }
                    continue;
                }
                // Glyphs are in visual order within the line, so a selected
                // glyph continues the current rectangle only when it starts
                // where that rectangle ends. Anywhere else means the selection
                // crossed a direction boundary and genuinely needs a second
                // rectangle — the case a single-rectangle implementation gets
                // visibly wrong the first time it meets mixed text.
                let yatasil = jari.as_ref().is_some_and(|mustatil| {
                    (harf.s - (mustatil.s + mustatil.ard)).abs() <= TASAMUH
                });
                if yatasil {
                    if let Some(mustatil) = jari.as_mut() {
                        mustatil.ard += harf.taqaddum;
                    }
                } else {
                    if let Some(mustatil) = jari.take() {
                        mustatilat.push(mustatil);
                    }
                    jari = Some(MustatilNass {
                        s: harf.s,
                        a: satr.asas - satr.suud,
                        ard: harf.taqaddum,
                        irtifa: satr.irtifa,
                    });
                }
            }
            if let Some(mustatil) = jari.take() {
                mustatilat.push(mustatil);
            }
        }
        mustatilat
    }

    /// Every distinct `(font index, glyph id)` this layout draws.
    ///
    /// This is how the atlas learns what to rasterize: from what shaping
    /// actually produced, never from a guessed range of characters.
    #[must_use]
    pub fn ashkal(&self) -> Vec<(u8, u32)> {
        let mut ashkal: Vec<(u8, u32)> = self
            .huruf
            .iter()
            .map(|harf| (harf.khatt, harf.muarrif))
            .collect();
        ashkal.sort_unstable();
        ashkal.dedup();
        ashkal
    }
}
