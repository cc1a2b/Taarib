//! اللوحة البديلة — the card that has no picture, designed to be looked at.
//!
//! The last rung of the artwork cascade in `taarib-kashf`, and the only one
//! that always succeeds. For a large indie or itch.io library it will be most
//! of the grid, which is the fact that decides everything about how it is
//! built: this is not an error state with a placeholder graphic, it is a
//! *designed* state that has to hold its own beside real cover art without
//! competing with it.
//!
//! ## Why the field is not grey
//!
//! A wall of identical grey rectangles reads as a broken grid — the eye sees a
//! failure to load rather than a set of games. So each plate derives a hue from
//! a hash of the game's identity, which makes forty plates read as forty
//! distinct things.
//!
//! And why the hue is nonetheless nearly grey: a plate sitting beside a real
//! cover must lose. [`HUDUD_ISHBAA`] and [`HUDUD_IDAA`] keep every derived field
//! inside a narrow, quiet band — saturation in single digits, luminance within a
//! few percent of the surface behind it. The variation is enough to separate one
//! card from its neighbour and not enough to draw the eye away from a game that
//! has artwork. A designer's instinct here is to make the plates attractive; the
//! correct instinct is to make them *recede*.
//!
//! ## Deterministic, and stable across machines
//!
//! The hue comes from BLAKE3 over the game's identity string, so the same game
//! is the same colour on every machine and after every rescan. A plate that
//! changed colour when the library refreshed would be worse than a grey one,
//! because the user would notice the change and learn nothing from it.
//!
//! ## What is deliberately absent
//!
//! No icon. No placeholder image glyph. No question mark, no camera, no broken-
//! picture symbol. Every one of those says "something went wrong here", and
//! nothing went wrong: this game simply has no cover art, which is an ordinary
//! property of a great many good games.
//!
//! ## This module produces a description, not pixels
//!
//! [`LawhaBadila`] carries a colour, a title, a wrapped and sized layout, and a
//! launcher mark. The interface draws it — in the same renderer, with the same
//! Arabic shaping, that every other piece of text in the product goes through.
//! Rasterizing here would mean a second text path, and this product does not
//! have one.

use serde::{Deserialize, Serialize};

use crate::luba::{LawnBariz, MasdarLuba};

/// The saturation band a derived field may occupy, in percent.
///
/// Four to eleven. Below four the hue is invisible and every plate is grey
/// again; above eleven a plate starts to read as a coloured tile and competes
/// with the artwork beside it. The band is narrow on purpose — this is the
/// single number that decides whether a grid of plates looks considered or
/// looks broken.
pub const HUDUD_ISHBAA: (u8, u8) = (4, 11);

/// The lightness band a derived field may occupy, in percent, on the dark theme.
///
/// Fifteen to twenty-two, against a `--sath-1` of `#121415` which sits near
/// eight. Every plate is therefore lighter than the surface it sits on — it
/// reads as a card rather than as a hole — and the seven-point spread is what
/// separates two plates side by side.
pub const HUDUD_IDAA: (u8, u8) = (15, 22);

/// The lightness band on the light theme.
///
/// Eighty-four to ninety-one, against a `--sath-1` of `#ffffff`. Inverted from
/// the dark band and for the same reason: darker than the surface, so the plate
/// still reads as a card, with the same seven-point spread.
pub const HUDUD_IDAA_FATIH: (u8, u8) = (84, 91);

/// The longest title this build will lay out before it gives up and truncates.
///
/// One hundred and twenty characters. Long enough for every real title
/// including the subtitle-heavy Japanese imports and the "Definitive Edition
/// Remastered" chains; short enough that a title made of a thousand characters —
/// which some itch.io entries genuinely are — cannot be laid out at a size
/// nobody can read and then rendered anyway.
pub const AQSA_TUL_UNWAN: usize = 120;

/// How many lines a title may wrap to.
///
/// Two, because the well it is drawn into is wide and short. It was three when
/// the card was a 2:3 portrait and the well was 220 × 340; the card is now a
/// 460:215 landscape and the well is 292 × 136, where three lines at the
/// largest step measure taller than the well is deep and the third would be
/// drawn outside it. The extra width pays for the lost line — twenty
/// characters fit on a line at the largest step now against fifteen before, so
/// two lines hold more than three did.
pub const AQSA_SUTUR: usize = 2;

/// The size steps a title is scaled down through, largest first, in points at
/// the card's baseline width.
///
/// Steps rather than a continuous fit, because a continuously fitted title is a
/// different size on every card and a grid of them looks unset. Four steps give
/// enough range to fit a long title without producing a page of mismatched
/// sizes.
pub const KHUTUWAT_HAJM: [u8; 4] = [28, 22, 18, 15];

/// Which theme the plate is being drawn for.
///
/// Affects only the lightness band. The hue and saturation derivation is
/// identical, so a game keeps its colour identity when the user switches themes
/// — which is the point of deriving it from the identity in the first place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SimatLawha {
    /// The dark theme, which is the default.
    Daken,
    /// The light theme.
    Fatih,
}

impl SimatLawha {
    /// The lightness band for this theme.
    #[must_use]
    pub const fn hudud_idaa(self) -> (u8, u8) {
        match self {
            Self::Daken => HUDUD_IDAA,
            Self::Fatih => HUDUD_IDAA_FATIH,
        }
    }

    /// Whether text on this plate should be light.
    #[must_use]
    pub const fn nass_fatih(self) -> bool {
        matches!(self, Self::Daken)
    }
}

/// One line of a laid-out title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct SatrUnwan {
    /// The text of this line, already broken at a word boundary.
    pub nass: String,
    /// Whether this line ends in an ellipsis because the title was absurd.
    pub maqsus: bool,
}

/// A plate, fully described and ready to draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct LawhaBadila {
    /// The flat field colour.
    pub lawn: LawnBariz,
    /// The title, wrapped and ready to set.
    pub sutur: Vec<SatrUnwan>,
    /// The point size the title is set at, from [`KHUTUWAT_HAJM`].
    pub hajm: u8,
    /// The launcher's Arabic name, for the small mark in the lower corner.
    ///
    /// Owned, not `&'static str`: a borrowed field makes the whole plate — and
    /// every scan result that carries one — deserializable only for
    /// `'de: 'static`, which means not deserializable at all.
    pub alamat_matjar: String,
    /// Whether the title should be set in a light colour.
    pub nass_fatih: bool,
}

impl LawhaBadila {
    /// Builds the plate for one game.
    ///
    /// `ism` is the game's display name exactly as the launcher gave it —
    /// unnormalized, because the plate shows what the user's launcher shows and
    /// a normalized title would be a second, subtly different name for the same
    /// game.
    ///
    /// The layout is computed here rather than in the interface so that a title
    /// which does not fit is discovered once, in one place, with one rule. Three
    /// interfaces each deciding when to drop a size step would be three
    /// different-looking grids.
    #[must_use]
    pub fn jadeeda(masdar: &MasdarLuba, ism: &str, sima: SimatLawha) -> Self {
        let lawn = lawn_min_huwiya(&masdar.muarrif(), sima);
        let (sutur, hajm) = rattib_unwan(ism);
        Self {
            lawn,
            sutur,
            hajm,
            alamat_matjar: masdar.ism_arabi().to_owned(),
            nass_fatih: sima.nass_fatih(),
        }
    }

    /// The field colour as `#rrggbb`, which is how it reaches the stylesheet.
    #[must_use]
    pub fn lawn_sittasi(&self) -> String {
        self.lawn.sittasi()
    }

    /// Whether the title had to be truncated.
    ///
    /// Surfaced so the interface can offer the full name in a tooltip. A
    /// truncated title with no way to see the rest is a card that hides
    /// information rather than one that fits it.
    #[must_use]
    pub fn maqsusa(&self) -> bool {
        self.sutur.iter().any(|satr| satr.maqsus)
    }
}

/// Derives a plate's colour from a game's identity.
///
/// BLAKE3 over the identity string, then three bytes of it steer hue,
/// saturation and lightness inside their bands. The identity rather than the
/// title, because a title can change — a launcher fixing its own metadata, a
/// game renamed for a re-release — and a plate that changed colour for that
/// reason would be a change the user notices and cannot explain.
#[must_use]
pub fn lawn_min_huwiya(huwiya: &str, sima: SimatLawha) -> LawnBariz {
    // Three independent hashes rather than three bytes of one, because adjacent
    // bits of a single value produce hues that cluster — and a grid where every
    // plate is a slightly different blue has failed at the one thing this
    // function exists for. Each stream is salted differently, so the three are
    // uncorrelated.
    let bayt_lawn = bayt_min_basma(basma_nass(huwiya, b"lawn"));
    let bayt_ishbaa = bayt_min_basma(basma_nass(huwiya, b"ishbaa"));
    let bayt_idaa = bayt_min_basma(basma_nass(huwiya, b"idaa"));

    let hu = u16::from(bayt_lawn).saturating_mul(360).saturating_div(256);
    let (ishbaa_min, ishbaa_max) = HUDUD_ISHBAA;
    let (idaa_min, idaa_max) = sima.hudud_idaa();
    let ishbaa = fi_nitaq(bayt_ishbaa, ishbaa_min, ishbaa_max);
    let idaa = fi_nitaq(bayt_idaa, idaa_min, idaa_max);

    min_hsl(hu, ishbaa, idaa)
}

/// Maps a byte onto an inclusive band.
///
/// Integer arithmetic throughout: `integer_division` is denied workspace-wide
/// and this is one of the places where a float would be the obvious reach and
/// would buy nothing — the output is a percentage in whole numbers either way.
fn fi_nitaq(bayt: u8, adna: u8, aqsa: u8) -> u8 {
    if aqsa <= adna {
        return adna;
    }
    let madaa = u16::from(aqsa.saturating_sub(adna)).saturating_add(1);
    let izaha = u16::from(bayt).saturating_mul(madaa).saturating_div(256);
    let natija = u16::from(adna).saturating_add(izaha);
    u8::try_from(natija.min(u16::from(aqsa))).unwrap_or(adna)
}

/// HSL to RGB, in integers.
///
/// Written out rather than pulled from a crate because the whole conversion is
/// twenty lines and the alternative is a dependency for one function. Hue is
/// degrees, saturation and lightness are percentages; every intermediate is
/// scaled by a thousand so the arithmetic stays in `i32` without a float
/// anywhere.
#[expect(
    clippy::many_single_char_names,
    reason = "c, x, m and the r/g/b triple are the published HSL-to-RGB names the \
              formulas in the comments below are written in; renaming them would \
              detach the code from the reference it is transcribed from"
)]
#[must_use]
pub fn min_hsl(hu: u16, ishbaa: u8, idaa: u8) -> LawnBariz {
    const MIQYAS: i32 = 1_000;

    let hu = i32::from(hu % 360);
    let sat = i32::from(ishbaa).saturating_mul(MIQYAS).saturating_div(100);
    let lig = i32::from(idaa).saturating_mul(MIQYAS).saturating_div(100);

    // c = (1 - |2L - 1|) * S
    let mutlaq = (lig.saturating_mul(2).saturating_sub(MIQYAS)).abs();
    let c = MIQYAS
        .saturating_sub(mutlaq)
        .saturating_mul(sat)
        .saturating_div(MIQYAS);

    // x = c * (1 - |((H / 60) mod 2) - 1|), with H/60 kept scaled.
    let qitaa = hu.saturating_mul(MIQYAS).saturating_div(60);
    let dakhil = qitaa % (2 * MIQYAS);
    let x = c
        .saturating_mul(MIQYAS.saturating_sub((dakhil.saturating_sub(MIQYAS)).abs()))
        .saturating_div(MIQYAS);

    let m = lig.saturating_sub(c.saturating_div(2));

    let (r, g, b) = match hu {
        0..=59 => (c, x, 0),
        60..=119 => (x, c, 0),
        120..=179 => (0, c, x),
        180..=239 => (0, x, c),
        240..=299 => (x, 0, c),
        _ => (c, 0, x),
    };

    LawnBariz {
        ahmar: qanat(r.saturating_add(m)),
        akhdar: qanat(g.saturating_add(m)),
        azraq: qanat(b.saturating_add(m)),
    }
}

/// A scaled channel value as a byte, clamped.
fn qanat(qeema: i32) -> u8 {
    let mutadarraj = qeema.saturating_mul(255).saturating_div(1_000);
    u8::try_from(mutadarraj.clamp(0, 255)).unwrap_or(0)
}

/// Wraps a title to at most three lines, dropping through the size steps.
///
/// The rule, in order:
///
/// 1. Try the largest size. Wrap at word boundaries into at most three lines,
///    where a line's capacity is derived from the size — a bigger face fits
///    fewer characters.
/// 2. If it does not fit, drop to the next size and try again.
/// 3. If it does not fit at the smallest size, truncate the last line with an
///    ellipsis. This is the only case that truncates, and
///    [`AQSA_TUL_UNWAN`] means it takes a genuinely absurd title to reach it.
///
/// Wrapping by character count rather than by measured width is deliberate and
/// worth stating: measuring needs the shaping engine, the shaping engine is
/// three crates away, and a plate is laid out for every card in a ten-thousand
/// game library on every theme change. The estimate is tuned to be conservative,
/// so a title that would have just fitted drops a size step rather than
/// overflowing — the failure direction that is invisible instead of broken.
#[must_use]
pub fn rattib_unwan(ism: &str) -> (Vec<SatrUnwan>, u8) {
    let munaqqa = ism.trim();
    if munaqqa.is_empty() {
        return (Vec::new(), KHUTUWAT_HAJM.last().copied().unwrap_or(15));
    }

    let mahdud: String = munaqqa.chars().take(AQSA_TUL_UNWAN).collect();
    let kalimat: Vec<&str> = mahdud.split_whitespace().collect();

    for hajm in KHUTUWAT_HAJM {
        let saa = saat_al_satr(hajm);
        if let Some(sutur) = laff(&kalimat, saa) {
            return (sutur, hajm);
        }
    }

    // Every size failed. Wrap at the smallest and cut the last line.
    let hajm = KHUTUWAT_HAJM.last().copied().unwrap_or(15);
    let saa = saat_al_satr(hajm);
    let mut sutur = laff_bila_hadd(&kalimat, saa);
    sutur.truncate(AQSA_SUTUR);
    if let Some(akhir) = sutur.last_mut() {
        akhir.maqsus = true;
        let mut nass: String = akhir.nass.chars().take(saa.saturating_sub(1)).collect();
        nass.push('…');
        akhir.nass = nass;
    }
    (sutur, hajm)
}

/// How many characters a line holds at a given size.
///
/// Derived from the card's baseline drawn width and the observation that the
/// interface Arabic face averages close to half its point size per character.
/// Conservative by design — see [`rattib_unwan`] for why the estimate errs
/// toward dropping a size step.
///
/// The width below is the artwork well's, and the card scales the chosen size
/// by the ratio of the well it is actually drawn into to this number — so the
/// two must name the same well. `ARD_MARSUM_ASAS` in the card component is the
/// other half of that pair.
fn saat_al_satr(hajm: u8) -> usize {
    const ARD_MARSUM: usize = 292;
    let nisf = usize::from(hajm).max(1);
    ARD_MARSUM.saturating_mul(2).saturating_div(nisf).max(4)
}

/// Wraps into at most [`AQSA_SUTUR`] lines, or [`None`] if it will not fit.
fn laff(kalimat: &[&str], saa: usize) -> Option<Vec<SatrUnwan>> {
    let sutur = laff_bila_hadd(kalimat, saa);
    (sutur.len() <= AQSA_SUTUR).then_some(sutur)
}

/// Wraps into as many lines as it takes.
///
/// A single word longer than the line capacity gets its own line rather than
/// being split: breaking inside a word is wrong in Arabic, where the letters
/// join, and a mid-word break would produce two fragments that each shape as
/// though they were whole words.
fn laff_bila_hadd(kalimat: &[&str], saa: usize) -> Vec<SatrUnwan> {
    let mut sutur: Vec<SatrUnwan> = Vec::new();
    let mut haliy = String::new();

    for kalima in kalimat {
        let tul_kalima = kalima.chars().count();
        if haliy.is_empty() {
            haliy.push_str(kalima);
            continue;
        }
        let mutawaqqa = haliy
            .chars()
            .count()
            .saturating_add(1)
            .saturating_add(tul_kalima);
        if mutawaqqa <= saa {
            haliy.push(' ');
        } else {
            sutur.push(SatrUnwan {
                nass: std::mem::take(&mut haliy),
                maqsus: false,
            });
        }
        haliy.push_str(kalima);
    }
    if !haliy.is_empty() {
        sutur.push(SatrUnwan {
            nass: haliy,
            maqsus: false,
        });
    }
    sutur
}

/// FNV-1a over a salt and a game's identity.
///
/// Not a cryptographic hash, and saying so plainly is better than reaching for
/// one out of habit. The question this answers is "which of eight quiet greys is
/// this game", and the only party who could gain by forging an answer is the
/// user, forging a colour on their own screen. BLAKE3 is used everywhere in this
/// product where a fingerprint has to resist an adversary — a backup's contents,
/// a patch's signature, an install's shape. A plate's hue is not one of those
/// places, and pulling a hashing crate into the vocabulary crate for it would be
/// a dependency bought with nothing.
///
/// The salt is mixed in first so that the three streams driving hue, saturation
/// and lightness cannot agree even for identities that differ in one byte.
fn basma_nass(nass: &str, milh: &[u8]) -> u64 {
    const BIDAYA: u64 = 0xcbf2_9ce4_8422_2325;
    const ADAD_AWWALI: u64 = 0x0000_0100_0000_01b3;

    milh.iter()
        .chain(b"\0")
        .chain(nass.as_bytes())
        .fold(BIDAYA, |basma, wahid| {
            (basma ^ u64::from(*wahid)).wrapping_mul(ADAD_AWWALI)
        })
}

/// One byte of a hash, taken from the high end.
///
/// The top bits rather than the bottom: FNV-1a's final multiply leaves its
/// weakest avalanche in the low byte, so a set of identities differing only in
/// their last character would map to a run of adjacent values there.
const fn bayt_min_basma(basma: u64) -> u8 {
    (basma >> 56) as u8
}
