//! رسم الطبقة — turning shaped Arabic into quads, and the proof that this crate
//! has no text of its own.
//!
//! Every glyph the overlay draws arrives here as a
//! [`taarib_saff::natija::Harf`] — a glyph identifier with a position, produced
//! by shaping a real string against a real font. This module never sees a
//! string. It cannot lay one out, it cannot pick a font for one, and it cannot
//! decide which direction one runs in, because it never has one to make those
//! decisions about.
//!
//! That is deliberate and it is the point of the module. The control panel's own
//! labels go through `saff` exactly as a translated line of dialogue does. A
//! tier-3 renderer that drew its own interface text would be the one place in
//! this product where Arabic could be wrong in a way nothing else catches — the
//! game text would be checked by every test the shaping engine has, and the
//! overlay's own "الترجمة" button would be checked by nobody.
//!
//! ## What this module actually does
//!
//! Three things, none of them typographic:
//!
//! 1. **Look each glyph up in the atlas** and turn its
//!    [`taarib_lawha::khareeta::MawdiShakl`] into normalized texture
//!    coordinates.
//! 2. **Place the quad** at the pen position the layout already computed, offset
//!    by the glyph's bearings. The convention is stated once, in
//!    [`qita_min_harf`], because a bearing sign error is invisible in code
//!    review and obvious on screen, and having it in one place means it is
//!    wrong everywhere or nowhere.
//! 3. **Size a backing plate** to the shaped text so light Arabic on a light
//!    scene is still readable.
//!
//! ## Premultiplied alpha, and why the colour is not what you would guess
//!
//! Every [`crate::wajiha::QitaRasm`] carries premultiplied linear RGBA, and the
//! backends configure their blend as `ONE, INV_SRC_ALPHA` to match. Straight
//! alpha would need `SRC_ALPHA, INV_SRC_ALPHA`, which is one enum different and
//! looks identical on opaque text — the difference only shows up on the
//! antialiased edge of a glyph, as a dark halo, on exactly the light backgrounds
//! where the overlay is hardest to read anyway.
//!
//! Colours also enter this module as sRGB and leave it linear.
//! [`AlwanTabaqa`] does the conversion with the real piecewise transfer function
//! rather than a 2.2 power, because the two disagree most in the near-black
//! range that antialiased text edges live in.
//!
//! ## Ordering
//!
//! Plates before glyphs, and both in submission order. There is no depth buffer
//! — the backends all disable depth test and depth write — so the batch's order
//! *is* the layering, and a glyph emitted before its plate is a glyph the plate
//! covers.

use taarib_lawha::khareeta::{KhareetatAshkal, MawdiShakl, MiftahShakl, NamatSafha};
use taarib_saff::natija::{Harf, TakhtitNass};

use crate::khata::KhataTabaqa;
use crate::wajiha::{LawhatRasm, MustatilBiksel, MustatilNisbi, QitaRasm, WasfSath};

/// The largest atlas this build will accept for upload.
///
/// Sixteen thousand three hundred and eighty-four on a side, which is the
/// maximum 2D texture dimension every API in this crate guarantees at its
/// minimum supported feature level. Checked here rather than in the four
/// backends so a rejection reads the same on all of them.
pub const AQSA_DILA_LAWHA: u32 = 16_384;

/// The overlay's palette, in sRGB, as a user would name the colours.
///
/// Stored encoded and converted on use rather than stored linear, for one
/// reason: these are values a user edits in a settings file, and `#e8e8e8` is a
/// colour somebody can reason about while `0.8069` is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlwanTabaqa {
    /// Translated text.
    pub nass: [u8; 4],
    /// The plate behind translated text.
    pub lawh: [u8; 4],
    /// The control panel's background.
    pub khalfiyat_lawha: [u8; 4],
    /// Headings and the selected row.
    pub tashdeed: [u8; 4],
    /// Secondary text: timestamps, confidences, the frame budget.
    pub thanawi: [u8; 4],
    /// A region's outline while the editor is open.
    pub hudud_mintaqa: [u8; 4],
}

impl AlwanTabaqa {
    /// The default palette.
    ///
    /// A near-black plate at seventy-five percent rather than pure black at
    /// full: an opaque plate hides the art the player is looking at, and pure
    /// black next to a dark scene reads as a hole rather than as a caption. The
    /// text is not pure white for the same reason in reverse — full white on a
    /// dark plate blooms on an OLED panel and is genuinely harder to read.
    #[must_use]
    pub const fn iftiradiya() -> Self {
        Self {
            nass: [0xF2, 0xF2, 0xF0, 0xFF],
            lawh: [0x0C, 0x0C, 0x10, 0xBF],
            khalfiyat_lawha: [0x14, 0x16, 0x1C, 0xF2],
            tashdeed: [0x7A, 0xC8, 0xFF, 0xFF],
            thanawi: [0xA0, 0xA4, 0xAC, 0xFF],
            hudud_mintaqa: [0x4A, 0xE0, 0x8C, 0xE6],
        }
    }

    /// One colour as premultiplied linear RGBA, ready for a quad.
    ///
    /// The order matters and is easy to get backwards: **decode to linear
    /// first, then premultiply**. Premultiplying the encoded values and
    /// decoding afterwards is a different function, and it makes half-covered
    /// glyph edges too bright — which looks like a font rendering problem and
    /// is not one.
    #[must_use]
    pub fn ila_khatti(lawn: [u8; 4]) -> [f32; 4] {
        let alfa = qeema_min_bayt(lawn[3]);
        [
            min_srgb(lawn[0]) * alfa,
            min_srgb(lawn[1]) * alfa,
            min_srgb(lawn[2]) * alfa,
            alfa,
        ]
    }
}

impl Default for AlwanTabaqa {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

/// A byte channel as a float between zero and one.
fn qeema_min_bayt(bayt: u8) -> f32 {
    f32::from(bayt) / 255.0
}

/// One sRGB-encoded channel decoded to linear.
///
/// The real piecewise curve, not a 2.2 power. They differ by up to four percent,
/// and they differ most below about 0.04 — which is exactly the range a glyph's
/// antialiased edge occupies. Using the approximation makes small text look
/// slightly bolder than the same text drawn by the game, which is the kind of
/// difference nobody can name and everybody notices.
fn min_srgb(bayt: u8) -> f32 {
    let qeema = qeema_min_bayt(bayt);
    if qeema <= 0.040_449_936 {
        qeema / 12.92
    } else {
        ((qeema + 0.055) / 1.055).powf(2.4)
    }
}

/// Where a glyph's image sits in the atlas, normalized against its page.
///
/// [`None`] when the glyph has no image, which is the correct and common answer
/// for a space — and is why this returns an option rather than a zero-area
/// rectangle a caller would have to remember to check.
fn khareetat_shakl(mawdi: &MawdiShakl, ard_safha: u16, irtifa_safha: u16) -> Option<MustatilNisbi> {
    if mawdi.ard == 0 || mawdi.irtifa == 0 || ard_safha == 0 || irtifa_safha == 0 {
        return None;
    }
    let ard = f32::from(ard_safha);
    let irtifa = f32::from(irtifa_safha);
    Some(MustatilNisbi {
        yasar: f32::from(mawdi.s) / ard,
        aala: f32::from(mawdi.a) / irtifa,
        ard: f32::from(mawdi.ard) / ard,
        irtifa: f32::from(mawdi.irtifa) / irtifa,
    })
}

/// One glyph as a quad, at a pen position.
///
/// **The placement convention, stated once for the whole crate.** The pen sits
/// at `(qalam_s, qalam_a)`, which is the **top-left corner of the layout** in
/// surface pixels — not a baseline. That is what [`Harf::s`] and [`Harf::a`] are
/// measured from: both are documented as offsets from the layout's left and top
/// edges, and `harf.a` on a baseline glyph is therefore the line's own baseline
/// offset rather than zero. A glyph's image goes at:
///
/// * `x = pen_x + harf.s + mawdi.izaha_s`
/// * `y = pen_y + harf.a - mawdi.izaha_a`
///
/// `izaha_s` is a left bearing and adds. `izaha_a` is a **top** bearing measured
/// upward from the baseline, and the surface's y axis grows downward, so it
/// *subtracts*. That sign is the single most common error in glyph placement and
/// the reason this is one function rather than four copies in four backends.
///
/// The corner rather than the baseline is also what a caller actually has. Every
/// position this tier draws at is a rectangle — a recognizer's box, a panel
/// element — and a rectangle has a top edge and no baseline.
///
/// Returns [`None`] for a glyph with no image, so a caller can pass the whole
/// line through and get back only what draws.
#[must_use]
pub fn qita_min_harf(
    harf: &Harf,
    mawdi: &MawdiShakl,
    khareeta: MustatilNisbi,
    qalam_s: f32,
    qalam_a: f32,
    lawn: [f32; 4],
) -> Option<QitaRasm> {
    if mawdi.ard == 0 || mawdi.irtifa == 0 {
        return None;
    }
    let yasar = qalam_s + harf.s + f32::from(mawdi.izaha_s);
    let aala = qalam_a + harf.a - f32::from(mawdi.izaha_a);
    Some(QitaRasm {
        mawdi: MustatilBiksel {
            yasar: ila_biksel(yasar),
            aala: ila_biksel(aala),
            ard: u32::from(mawdi.ard),
            irtifa: u32::from(mawdi.irtifa),
        },
        khareeta: Some(khareeta),
        lawn,
    })
}

/// A float position as a pixel coordinate, clamped at zero.
///
/// Rounded rather than truncated. A glyph whose left edge lands at 10.6 belongs
/// at 11: truncating puts every glyph up to a pixel to the left of where the
/// layout put it, which accumulates across a line into visibly tighter spacing
/// than the shaper produced.
fn ila_biksel(qeema: f32) -> u32 {
    if qeema <= 0.0 {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive by the guard above and bounded by the surface dimension the caller \
                  laid out against, which is far below u32::MAX"
    )]
    {
        qeema.round().min(f32::from(u16::MAX)) as u32
    }
}

/// Builds the draw batch for one laid-out block of text.
///
/// Takes the layout `saff` produced and the atlas `lawha` packed, and emits the
/// plate followed by the glyphs. Nothing here shapes, measures, or reorders:
/// [`TakhtitNass::huruf`] is already in visual order, which is what
/// "already in visual order: this is where it is drawn" means on
/// [`Harf::s`].
///
/// `hajm_rubi` is the quarter-pixel size the atlas was keyed with, and it must
/// come from [`TakhtitNass::hajm`] — the size the text was *finally* laid out
/// at, which differs from the requested one when the overflow policy shrank it.
/// A key built from the requested size names an image of a different size than
/// the one the layout measured against.
///
/// `(qalam_s, qalam_a)` is the layout's top-left corner in surface pixels. See
/// [`qita_min_harf`] for the whole convention.
///
/// `mahjub` is the rectangle this text **replaces**, when it replaces one, and
/// the plate is grown to cover it. That is the difference between a caption and
/// a translation: a caption needs a backing wide enough to read its own text
/// against, and a translation needs one wide enough to hide the sentence
/// underneath it. Arabic is usually shorter than the English it replaces once
/// the overflow policy has had its say, so a plate fitted to the Arabic alone
/// leaves the tail of the original showing beside it — which is not a cosmetic
/// defect but two languages on screen saying the same thing. Pass [`None`] for
/// text that covers nothing, which is what the overlay's own captions are.
///
/// # Errors
///
/// [`KhataTabaqa::MintaqaKharij`] when the text would be placed outside the
/// surface entirely — which means the caller positioned against a surface that
/// has since changed, and drawing it would put a caption off-screen rather than
/// nowhere.
pub fn ibn_dufa(
    takhtit: &TakhtitNass,
    khareeta: &KhareetatAshkal,
    sath: WasfSath,
    qalam_s: f32,
    qalam_a: f32,
    alwan: &AlwanTabaqa,
    hajm_rubi: u16,
    namat: NamatSafha,
    mahjub: Option<MustatilBiksel>,
) -> Result<Vec<QitaRasm>, KhataTabaqa> {
    if sath.ard == 0 || sath.irtifa == 0 {
        return Err(KhataTabaqa::MintaqaKharij {
            mintaqa: "the overlay's own text".to_owned(),
            ard: sath.ard,
            irtifa: sath.irtifa,
        });
    }

    let mut qitaat: Vec<QitaRasm> = Vec::with_capacity(takhtit.huruf.len().saturating_add(1));

    // The plate first, because there is no depth buffer and submission order is
    // the layering. Sized to the layout's own extents plus a margin
    // proportional to the text size, so it stays right when the user scales the
    // font rather than needing a second setting nobody would think to change.
    if !takhtit.huruf.is_empty() {
        let hamish = (takhtit.hajm * 0.35).max(2.0);
        if let Some(lawh) = qita_lawh(takhtit, qalam_s, qalam_a, hamish, alwan.lawh, mahjub) {
            qitaat.push(lawh);
        }
    }

    qitaat.extend(qitaat_nass(
        takhtit,
        khareeta,
        qalam_s,
        qalam_a,
        AlwanTabaqa::ila_khatti(alwan.nass),
        hajm_rubi,
        namat,
    ));
    Ok(qitaat)
}

/// The glyphs of a laid-out block, in one colour, with no plate.
///
/// [`ibn_dufa`] is this plus a backing plate, and calls it — so the glyph loop
/// and the atlas lookup exist once in this crate rather than once per caller.
/// This is the entry point for text that already has a background: the control
/// panel's own labels sit on plates [`crate::lawhat_tahakkum`] laid out, and a
/// second plate under each one would darken every button by drawing twice.
///
/// `lawn` is **premultiplied linear** RGBA and is passed through untouched, so a
/// caller holding a colour that is already in that form — every
/// [`crate::lawhat_tahakkum::AnsurLawha::lawn`] is — hands it over without a
/// second conversion. Converting an already-converted colour is the defect this
/// signature exists to make visible.
#[must_use]
pub fn qitaat_nass(
    takhtit: &TakhtitNass,
    khareeta: &KhareetatAshkal,
    qalam_s: f32,
    qalam_a: f32,
    lawn: [f32; 4],
    hajm_rubi: u16,
    namat: NamatSafha,
) -> Vec<QitaRasm> {
    let mut qitaat: Vec<QitaRasm> = Vec::with_capacity(takhtit.huruf.len());
    for harf in &takhtit.huruf {
        let miftah = MiftahShakl {
            khatt: harf.khatt,
            bakat: 0,
            hajm_rubi,
            namat,
            muarrif: harf.muarrif,
        };
        let Some(mawdi) = khareeta.mawdi(miftah) else {
            // A glyph the atlas does not hold is skipped rather than substituted.
            // Drawing a notdef box in its place would put a visible defect on
            // screen for a glyph the player might never have needed; leaving a
            // gap is the smaller wrong answer, and the atlas is rebuilt from
            // the shaped set on the next upload anyway.
            continue;
        };
        let Some((ard_safha, irtifa_safha)) = khareeta.abaad_safha(mawdi.safha) else {
            continue;
        };
        let Some(khareetat) = khareetat_shakl(mawdi, ard_safha, irtifa_safha) else {
            continue;
        };
        if let Some(qita) = qita_min_harf(harf, mawdi, khareetat, qalam_s, qalam_a, lawn) {
            qitaat.push(qita);
        }
    }
    qitaat
}

/// The backing plate for a laid-out block, grown to cover what it replaces.
///
/// [`None`] when the layout has no extent, which is what an empty string
/// produces and is not worth a zero-area quad in the batch.
fn qita_lawh(
    takhtit: &TakhtitNass,
    qalam_s: f32,
    qalam_a: f32,
    hamish: f32,
    lawn: [u8; 4],
    mahjub: Option<MustatilBiksel>,
) -> Option<QitaRasm> {
    if takhtit.ard <= 0.0 || takhtit.irtifa <= 0.0 {
        return None;
    }
    // The pen is the layout's top-left corner, and `takhtit.irtifa` is measured
    // from that same corner — so the plate's top is the pen minus the margin,
    // and nothing here needs a font metric.
    //
    // This subtracted the first line's baseline offset before anything called
    // it, which put the plate one ascent above the glyphs `qita_min_harf` was
    // placing from the same pen. The two functions were written against two
    // different meanings of `qalam_a`; `Harf::a` decides which one is real.
    let aala = qalam_a - hamish;

    // Horizontally the plate follows the *lines*, not `takhtit.ard`. The width
    // is the widest line's content, which says nothing about where that content
    // sits inside the width it was given — and in this tier it never sits at
    // zero: Arabic is laid out right-aligned in the rectangle it replaces, so
    // every line's `bidaya` is the far side of that rectangle. Sizing from
    // `qalam_s` alone put the plate at the left of the box while the glyphs were
    // drawn at the right, which is a plate covering the wrong half of the
    // original text.
    let (bidaya, nihaya) = takhtit.sutur.iter().fold((f32::MAX, f32::MIN), |(adna, aqsa), satr| {
        (adna.min(satr.bidaya), aqsa.max(satr.bidaya + satr.ard))
    });
    let (bidaya, nihaya) =
        if bidaya <= nihaya { (bidaya, nihaya) } else { (0.0, takhtit.ard) };

    let lil_nass = MustatilBiksel {
        yasar: ila_biksel(qalam_s + bidaya - hamish),
        aala: ila_biksel(aala),
        ard: ila_biksel(hamish.mul_add(2.0, nihaya - bidaya)).max(1),
        irtifa: ila_biksel(hamish.mul_add(2.0, takhtit.irtifa)).max(1),
    };
    Some(QitaRasm {
        mawdi: mahjub.map_or(lil_nass, |asli| ittihad(lil_nass, asli)),
        khareeta: None,
        lawn: AlwanTabaqa::ila_khatti(lawn),
    })
}

/// The smallest rectangle containing both.
///
/// One plate rather than two overlapping ones: the plate is translucent, and two
/// of them over the same pixels is that colour applied twice — a darker band
/// wherever they overlap, which is exactly where the text is.
const fn ittihad(awwal: MustatilBiksel, thani: MustatilBiksel) -> MustatilBiksel {
    let yasar = if awwal.yasar < thani.yasar { awwal.yasar } else { thani.yasar };
    let aala = if awwal.aala < thani.aala { awwal.aala } else { thani.aala };
    let yameen_a = awwal.yasar.saturating_add(awwal.ard);
    let yameen_b = thani.yasar.saturating_add(thani.ard);
    let asfal_a = awwal.aala.saturating_add(awwal.irtifa);
    let asfal_b = thani.aala.saturating_add(thani.irtifa);
    let yameen = if yameen_a > yameen_b { yameen_a } else { yameen_b };
    let asfal = if asfal_a > asfal_b { asfal_a } else { asfal_b };
    MustatilBiksel {
        yasar,
        aala,
        ard: yameen.saturating_sub(yasar),
        irtifa: asfal.saturating_sub(aala),
    }
}

/// A solid rectangle, for a panel background, a separator, or a region outline.
///
/// Takes normalized coordinates because everything the control panel and the
/// region editor produce is normalized — a panel laid out in pixels would be
/// half the screen at 4K and unreadable at 720p.
///
/// Returns [`None`] when the rectangle does not land on a pixel, which is what a
/// one-pixel separator on a small window becomes, and which is not worth a quad.
#[must_use]
pub fn qita_musmata(nisbi: MustatilNisbi, sath: WasfSath, lawn: [u8; 4]) -> Option<QitaRasm> {
    let mawdi = nisbi.fi_bikselat(sath.ard, sath.irtifa)?;
    Some(QitaRasm { mawdi, khareeta: None, lawn: AlwanTabaqa::ila_khatti(lawn) })
}

/// A hollow rectangle as four solid edges.
///
/// Used for a region's outline in the editor. Four quads rather than a shader
/// because the overlay's pipeline draws exactly one thing — a textured or solid
/// quad — and a second pipeline for outlines would be a second set of state for
/// every backend to save and restore.
#[must_use]
pub fn qitaat_itar(
    nisbi: MustatilNisbi,
    sath: WasfSath,
    lawn: [u8; 4],
    sumk: u32,
) -> Vec<QitaRasm> {
    let Some(mawdi) = nisbi.fi_bikselat(sath.ard, sath.irtifa) else {
        return Vec::new();
    };
    let sumk = sumk.max(1).min(mawdi.ard.min(mawdi.irtifa));
    let khatti = AlwanTabaqa::ila_khatti(lawn);
    let dakhil_irtifa = mawdi.irtifa.saturating_sub(sumk.saturating_mul(2));

    let mut qitaat = Vec::with_capacity(4);
    // Top and bottom span the full width; the sides span only what is between
    // them, so the corners are covered exactly once. Overlapping corners on a
    // translucent outline are visibly darker squares at each corner, which is a
    // defect a reviewer would have to look for and a player would see at once.
    qitaat.push(QitaRasm {
        mawdi: MustatilBiksel { irtifa: sumk, ..mawdi },
        khareeta: None,
        lawn: khatti,
    });
    qitaat.push(QitaRasm {
        mawdi: MustatilBiksel {
            aala: mawdi.aala.saturating_add(mawdi.irtifa).saturating_sub(sumk),
            irtifa: sumk,
            ..mawdi
        },
        khareeta: None,
        lawn: khatti,
    });
    if dakhil_irtifa > 0 {
        let aala = mawdi.aala.saturating_add(sumk);
        qitaat.push(QitaRasm {
            mawdi: MustatilBiksel { aala, ard: sumk, irtifa: dakhil_irtifa, ..mawdi },
            khareeta: None,
            lawn: khatti,
        });
        qitaat.push(QitaRasm {
            mawdi: MustatilBiksel {
                yasar: mawdi.yasar.saturating_add(mawdi.ard).saturating_sub(sumk),
                aala,
                ard: sumk,
                irtifa: dakhil_irtifa,
            },
            khareeta: None,
            lawn: khatti,
        });
    }
    qitaat
}

/// Assembles a whole frame's quads into the batch a backend draws.
///
/// The batch records the surface it was built for, and
/// [`crate::wajiha::Tabaqa::itar`] discards a batch whose surface no longer
/// matches rather than scaling it. Scaling would be the obvious kindness and it
/// is the wrong one: scaled glyphs are blurry glyphs, and the next frame's batch
/// is correct.
#[derive(Debug)]
pub struct BaniDufa {
    sath: WasfSath,
    qitaat: Vec<QitaRasm>,
}

impl BaniDufa {
    /// A builder for one surface.
    #[must_use]
    pub const fn jadeed(sath: WasfSath) -> Self {
        Self { sath, qitaat: Vec::new() }
    }

    /// The surface this batch is being built for.
    #[must_use]
    pub const fn sath(&self) -> WasfSath {
        self.sath
    }

    /// Adds quads, in submission order.
    pub fn adhif(&mut self, qitaat: impl IntoIterator<Item = QitaRasm>) {
        self.qitaat.extend(qitaat);
    }

    /// Adds one quad.
    pub fn adhif_wahid(&mut self, qita: QitaRasm) {
        self.qitaat.push(qita);
    }

    /// How many quads are in the batch.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.qitaat.len()
    }

    /// Finishes the batch, dropping anything that lands entirely off-surface.
    ///
    /// The clip is a whole-quad test rather than a per-quad scissor, and that is
    /// the right trade here: a glyph half off the edge of the screen is
    /// something the layout should not have produced, and clipping it would hide
    /// a positioning bug behind a correct-looking picture. Dropping it makes the
    /// bug visible as a missing letter, which is what gets it fixed.
    #[must_use]
    pub fn ikhtim(self) -> LawhatRasm {
        let ard = self.sath.ard;
        let irtifa = self.sath.irtifa;
        let qitaat = self
            .qitaat
            .into_iter()
            .filter(|qita| {
                qita.mawdi.ard > 0
                    && qita.mawdi.irtifa > 0
                    && qita.mawdi.yasar < ard
                    && qita.mawdi.aala < irtifa
            })
            .collect();
        LawhatRasm { qitaat, sath: self.sath }
    }
}

/// The atlas bytes a backend uploads, and the shape they are in.
///
/// [`taarib_lawha`] rasterizes coverage as one byte per texel and distance
/// fields the same way, and every backend in this crate uploads RGBA8 — one
/// texture format across four APIs rather than four format paths. The expansion
/// happens here, once, rather than in each backend.
///
/// **The coverage goes in all four channels**, which is to say the page is
/// premultiplied white, because everything downstream of it is premultiplied
/// too. Each backend's fragment stage computes `texel * quad_colour` and blends
/// `ONE, INV_SRC_ALPHA`; with a premultiplied quad colour `(R·a, G·a, B·a, a)`
/// the result a coverage `c` must produce is `(R·a·c, G·a·c, B·a·c, a·c)`, and
/// only a `(c, c, c, c)` texel gives that.
///
/// This wrote `(255, 255, 255, c)` before anything called it, on the reasoning
/// that the alpha alone carries the coverage and putting it in RGB as well would
/// apply it twice. That reasoning is right for *straight* alpha and wrong here:
/// with RGB left at one, `texel * colour` leaves the colour channels at full
/// strength across the whole glyph bitmap and varies only alpha, so every glyph
/// draws as a solid rectangle of its own bounding box. It is not a subtle
/// difference and it is visible in the first frame anything renders — which is
/// how it was found, and why `examples/talqeem_burhan.rs` exists.
///
/// # Errors
///
/// [`KhataTabaqa::HajmMufrit`] when a page exceeds [`AQSA_DILA_LAWHA`] on either
/// axis, and [`KhataTabaqa::MawridFashil`] when a page's byte count does not
/// match its declared dimensions — which means the atlas was built wrong and
/// uploading it would put arbitrary memory on screen.
pub fn ila_rgba(bayt: &[u8], ard: u16, irtifa: u16) -> Result<Vec<u8>, KhataTabaqa> {
    let ard32 = u32::from(ard);
    let irtifa32 = u32::from(irtifa);
    if ard32 > AQSA_DILA_LAWHA || irtifa32 > AQSA_DILA_LAWHA {
        return Err(KhataTabaqa::HajmMufrit {
            haql: "an atlas page's dimension",
            qeema: u64::from(ard32.max(irtifa32)),
            saqf: u64::from(AQSA_DILA_LAWHA),
        });
    }
    let mutawaqqa = u64::from(ard32).saturating_mul(u64::from(irtifa32));
    if mutawaqqa != crate::khata::tul_u64(bayt.len()) {
        return Err(KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas page",
            sabab: format!(
                "the page declares {ard}×{irtifa}, which is {mutawaqqa} texel(s), and carries \
                 {} byte(s)",
                bayt.len()
            ),
        });
    }

    let mut kharj = Vec::with_capacity(bayt.len().saturating_mul(4));
    for taghtiya in bayt {
        kharj.extend_from_slice(&[*taghtiya, *taghtiya, *taghtiya, *taghtiya]);
    }
    Ok(kharj)
}
