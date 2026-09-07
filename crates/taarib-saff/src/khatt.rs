//! الخط — font resources: one shared byte buffer, one identity, and the check
//! that stands between a font and a patch nobody can read.
//!
//! Two of the eight architectural decisions live in this file.
//!
//! **Decision 4 — one font byte buffer feeds both shaping and rasterization.**
//! A [`MawridKhatt`] owns exactly one `Arc<Vec<u8>>` and nothing anywhere else
//! that duplicates it. [`MawridKhatt::khatt`] rebuilds a `FontRef` over those
//! same bytes on demand, which is cheap because a `FontRef` is a table
//! directory over a borrowed slice, not a decoded font. The reason this is a
//! rule rather than a preference is that `skrifa::FontRef`, `harfrust::FontRef`
//! and `read_fonts::FontRef` are not three types that happen to agree — they
//! are the *same* type, re-exported by all three crates from one `read-fonts`.
//! Shaping and rasterization therefore walk the same table directory, resolve
//! the same collection offset, apply the same variation coordinates, and number
//! `CFF2` charstrings identically. The classic mismatch bug, where the shaper's
//! glyph 4211 is a different glyph from the rasterizer's glyph 4211 because two
//! parsers disagreed about a face index, is not made unlikely here. It is made
//! unrepresentable.
//!
//! **Decision 6 — a font without complete Arabic tables is rejected at load,
//! with a reason.** HarfRust has no Arabic fallback shaper: nothing synthesises
//! joining when a font lacks `GSUB`. Such a font does not draw ugly Arabic, it
//! draws twenty-eight disconnected letterforms, silently, and the person who
//! chose it experiences that as "the patch is broken" with nothing to act on.
//! [`MawridKhatt::fahs_arabi`] converts that silent runtime failure into a loud
//! one at the moment the font is chosen, and every failure it returns names one
//! specific missing table, feature or character.

use std::fmt;
use std::hash::Hasher as _;
use std::sync::Arc;

use read_fonts::types::Tag;
use read_fonts::{FileRef, ReadError, TableProvider as _};
use rustc_hash::{FxHashSet, FxHasher};
use skrifa::charmap::Charmap;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::string::StringId;
use skrifa::{FontRef, GlyphId, MetadataProvider as _};
use taarib_usus::khata::{Khata, Natija};

use crate::khata::{KhataKhatt, KhataSaff};

/// The tables Arabic shaping cannot work without, checked in this order so the
/// first failure names the most fundamental thing that is missing.
const JADAWIL_MATLUBA: [(&str, Tag); 3] = [
    ("cmap", Tag::new(b"cmap")),
    ("GSUB", Tag::new(b"GSUB")),
    ("GPOS", Tag::new(b"GPOS")),
];

/// The `GSUB` features that turn characters into joined letterforms.
///
/// `init`, `medi` and `fina` are contextual joining; `rlig` is where lam-alef
/// lives, and lam-alef is mandatory in Arabic orthography rather than
/// decorative.
///
/// `isol` is deliberately absent, and this list is shorter than the four
/// joining forms for a reason worth stating. A substitution feature exists to
/// replace the nominal glyph; the nominal glyph a `cmap` yields for an Arabic
/// letter already *is* its isolated form, so a font only defines `isol` when
/// the isolated form differs from that default. Requiring it therefore does not
/// test whether a font can draw isolated letters — every conforming Arabic font
/// can — it tests an implementation detail of how the designer chose to encode
/// them. Surveying every Arabic-shaping face on a developer machine found two
/// professionally commissioned families, IBM Plex Sans Arabic and Dubai, that
/// omit `isol` and shape correctly; requiring the tag rejected both, including
/// the one this product ships in its own interface.
const SIFAT_GSUB_MATLUBA: [(&str, Tag); 4] = [
    ("init", Tag::new(b"init")),
    ("medi", Tag::new(b"medi")),
    ("fina", Tag::new(b"fina")),
    ("rlig", Tag::new(b"rlig")),
];

/// The `GPOS` feature that attaches a diacritic to its base.
///
/// Without it marks do not float slightly wrong; they land on the baseline at
/// the pen position, on top of the next letter.
const SIFAT_GPOS_MATLUBA: [(&str, Tag); 1] = [("mark", Tag::new(b"mark"))];

/// The characters a font must cover before Taarib will shape Arabic with it.
///
/// The twenty-eight letters; hamza and the three alef forms `lam` ligates with,
/// because لا لآ لأ لإ are required forms and a font missing one of them breaks
/// on ordinary words; `teh marbuta` and `alef maqsura`, which end a large share
/// of all Arabic words; `tatweel`, without which there is nothing for kashida
/// justification to elongate; and the five marks vocalised text cannot be
/// written without.
const HURUF_MATLUBA: [char; 40] = [
    // alef, beh, teh, theh, jeem, hah, khah, dal
    '\u{0627}', '\u{0628}', '\u{062A}', '\u{062B}', '\u{062C}', '\u{062D}', '\u{062E}', '\u{062F}',
    // thal, reh, zain, seen, sheen, sad, dad, tah
    '\u{0630}', '\u{0631}', '\u{0632}', '\u{0633}', '\u{0634}', '\u{0635}', '\u{0636}', '\u{0637}',
    // zah, ain, ghain, feh, qaf, kaf, lam, meem
    '\u{0638}', '\u{0639}', '\u{063A}', '\u{0641}', '\u{0642}', '\u{0643}', '\u{0644}', '\u{0645}',
    // noon, heh, waw, yeh
    '\u{0646}', '\u{0647}', '\u{0648}', '\u{064A}',
    // hamza, alef madda, alef hamza above, alef hamza below
    '\u{0621}', '\u{0622}', '\u{0623}', '\u{0625}',
    // teh marbuta, alef maqsura, tatweel
    '\u{0629}', '\u{0649}', '\u{0640}', // fatha, damma, kasra, shadda, sukun
    '\u{064E}', '\u{064F}', '\u{0650}', '\u{0651}', '\u{0652}',
];

/// Glyphs asked for, in order, when the `OS/2` table does not declare a cap
/// height. `H` is the Latin convention; `alef` is the Arabic one, and it is the
/// letter Arabic type has always been proportioned against.
const NAMADHIJ_KABITAL: [char; 3] = ['H', 'E', '\u{0627}'];

/// Glyphs asked for, in order, when the `OS/2` table does not declare an
/// x-height. `heh` is the Arabic analogue: its loop sits at the body height
/// that the short letters share.
const NAMADHIJ_SAGHIR: [char; 3] = ['x', 'o', '\u{0647}'];

/// The most fonts one chain can hold.
///
/// A laid-out glyph records which font it came from in a `u8`
/// ([`crate::natija::Harf::khatt`]), so a font past index 255 could never be
/// named in the output. A chain that long is a caller's mistake rather than a
/// configuration, and [`SilsilatKhutut::jadeeda`] refuses it by name rather
/// than keeping a prefix and letting the loss reappear later as missing glyphs.
const AQSA_KHUTUT: usize = 256;

/// A font's identity, derived from its bytes and its face index.
///
/// Decision 4 says a font is identified by the identity of its *bytes*, never
/// by a path or a family name: a path can be re-resolved to a different file
/// after an update, and two unrelated families can share a name. The atlas
/// records this value in every glyph key and the layout cache keys on it, so
/// two resources built from identical bytes must compare equal — and they do.
///
/// The hash is `FxHash`, fast and non-cryptographic, and that is deliberate:
/// this value keys an in-process cache. It is not a signature, it is not a
/// security boundary, and nothing in Taarib decides whether to *trust* a font
/// by looking at it. Integrity of a published patch is BLAKE3 through
/// `taarib-mustalahat`'s `Basma`, and the two are not interchangeable —
/// `FxHash` mixes at pointer width, so this value is stable within one process
/// and is never written to disk.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HuwiyatKhatt(u64);

impl HuwiyatKhatt {
    /// Hashes a buffer and a face index into an identity.
    fn min_bayt(bayt: &[u8], fahras: u32) -> Self {
        let mut hashi = FxHasher::default();
        // The length goes in on its own so that two different buffers cannot
        // collide merely by compressing to the same digest inside `write`.
        hashi.write_usize(bayt.len());
        hashi.write(bayt);
        hashi.write_u32(fahras);
        Self(hashi.finish())
    }

    /// The raw value, for use as a cache key or a map index.
    #[must_use]
    pub const fn qeema(self) -> u64 {
        self.0
    }
}

impl fmt::Display for HuwiyatKhatt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl fmt::Debug for HuwiyatKhatt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HuwiyatKhatt({self})")
    }
}

/// One variation axis of a variable font.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MihwarKhatt {
    /// The four-character axis tag, for example `wght` or `wdth`.
    pub wasm: [u8; 4],
    /// The smallest value the font defines for this axis.
    pub adna: f32,
    /// The largest.
    pub aqsa: f32,
    /// The value the font uses when the caller asks for nothing.
    pub iftiradi: f32,
}

impl MihwarKhatt {
    /// The tag as text, for diagnostics and for
    /// [`KhataKhatt::MihwarMajhul`](crate::khata::KhataKhatt::MihwarMajhul).
    #[must_use]
    pub fn wasm_nass(self) -> String {
        String::from_utf8_lossy(&self.wasm).into_owned()
    }

    /// Brings a requested value inside the range the font actually defines.
    ///
    /// A weight of 900 asked of an axis that stops at 700 is not an error worth
    /// refusing a whole layout over; it is a request the font answers as best
    /// it can, and clamping is what the OpenType variation model does with it.
    #[must_use]
    pub const fn qayyid(self, qeema: f32) -> f32 {
        // `max` then `min` rather than `clamp`, which panics on a reversed or
        // NaN range — and a font is free to declare one.
        qeema.max(self.adna).min(self.aqsa)
    }
}

/// A font's own metrics, scaled to a pixel size.
///
/// Every value here comes from the font. Nothing in this struct is a fraction
/// of the size, an average, or a constant: a line height invented from the
/// requested size is the reason Arabic text collides with the box above it in
/// interfaces that were laid out for Latin.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QiyasatKhatt {
    /// Distance from the baseline to the top of the alignment box, in pixels.
    pub suud: f32,
    /// Distance from the baseline to the bottom, in pixels, as a positive
    /// number — the font stores it negative, and every caller wants it positive.
    pub hubut: f32,
    /// The extra space the font recommends between lines, in pixels.
    pub fajwa: f32,
    /// The line height the font recommends: ascent plus descent plus gap.
    pub irtifa_satr: f32,
    /// Height of a capital letter, in pixels.
    pub uluw_kabital: f32,
    /// Height of a lowercase `x`, or its Arabic equivalent, in pixels.
    pub uluw_saghir: f32,
    /// Design units per em, which is the scale everything above was derived at.
    pub wahdat: u16,
}

/// A loaded font: one shared immutable byte buffer, its identity, and what the
/// engine needs to know about it without parsing it again.
///
/// This type carries **Decision 4**. It holds exactly one `Arc<Vec<u8>>` and no
/// second copy of those bytes in any other form — no cached parse, no cloned
/// `Vec`, no separate rasterizer handle. [`MawridKhatt::khatt`] rebuilds a
/// `FontRef` from that one buffer whenever anything needs one, and every
/// consumer of this crate is handed the same buffer to borrow.
///
/// The reason the rule is stated this strongly is that it is what makes
/// Decision 3's guarantee total rather than probable. `skrifa::FontRef` and
/// `harfrust::FontRef` are literally the same `read_fonts::FontRef`: one table
/// directory, one collection offset, one `loca`/`glyf` pair, one `CFF2`
/// charstring index. A glyph id that comes out of shaping resolves, in the
/// rasterizer, to the outline the shaper meant — not because the two libraries
/// were tested against each other, but because there is only one parse. Loading
/// the same file twice into two independent parsers would reintroduce exactly
/// the silent, data-dependent, font-specific mismatch that decision exists to
/// delete, so this type gives no way to do it.
///
/// It is also memory this product does not own twice, in a library that will be
/// resident inside somebody else's game process.
#[derive(Clone)]
pub struct MawridKhatt {
    /// The one buffer. Shaping borrows it, rasterization borrows it, metrics
    /// borrow it, and nothing copies it.
    bayt: Arc<Vec<u8>>,
    fahras: u32,
    huwiya: HuwiyatKhatt,
    aila: String,
    namat: String,
    mahawir: Vec<MihwarKhatt>,
}

impl MawridKhatt {
    /// Loads a font from bytes and validates it for Arabic.
    ///
    /// `fahras` is the face index inside a `ttc`/`otc` collection, and `0` for
    /// an ordinary `ttf`/`otf`.
    ///
    /// # Errors
    ///
    /// [`KhataKhatt::TahleelFashil`] when the bytes are not a font;
    /// [`KhataKhatt::FahrasKharij`] when the face index is past the end of a
    /// collection; and then whatever [`MawridKhatt::fahs_arabi`] finds —
    /// [`KhataKhatt::JadwalMafqud`], [`KhataKhatt::SifaMafquda`] or
    /// [`KhataKhatt::TaghtiyaNaqisa`], each naming one specific missing thing.
    pub fn jadeed(bayt: Arc<Vec<u8>>, fahras: u32) -> Natija<Self> {
        let mawrid = Self::bina(bayt, fahras)?;
        mawrid.fahs_arabi()?;
        Ok(mawrid)
    }

    /// Loads a font from bytes without the Arabic check.
    ///
    /// This exists for the Latin and monospace faces at the end of a fallback
    /// chain, which carry the digits, the identifiers and the embedded English
    /// a game is full of and are never asked to shape a single Arabic letter.
    /// It is not an escape hatch for an Arabic font that failed validation:
    /// nothing in the product calls it for a font that will carry Arabic text.
    ///
    /// # Errors
    ///
    /// [`KhataKhatt::TahleelFashil`] when the bytes are not a font, and
    /// [`KhataKhatt::FahrasKharij`] when the face index is past the end of a
    /// collection.
    pub fn jadeed_latini(bayt: Arc<Vec<u8>>, fahras: u32) -> Natija<Self> {
        Self::bina(bayt, fahras)
    }

    /// Parses the buffer once to read the names and axes, then keeps only the
    /// buffer.
    fn bina(bayt: Arc<Vec<u8>>, fahras: u32) -> Natija<Self> {
        // The parse is scoped so its borrow of the buffer ends before the
        // buffer is moved into the resource: one buffer, one owner, no clone.
        let (aila, namat, mahawir) = {
            let khatt = tahleel(bayt.as_slice(), fahras)?;
            let mahawir: Vec<MihwarKhatt> = khatt
                .axes()
                .iter()
                .map(|mihwar| MihwarKhatt {
                    wasm: mihwar.tag().to_be_bytes(),
                    adna: mihwar.min_value(),
                    aqsa: mihwar.max_value(),
                    iftiradi: mihwar.default_value(),
                })
                .collect();
            (
                nass_khatt(
                    &khatt,
                    StringId::TYPOGRAPHIC_FAMILY_NAME,
                    StringId::FAMILY_NAME,
                ),
                nass_khatt(
                    &khatt,
                    StringId::TYPOGRAPHIC_SUBFAMILY_NAME,
                    StringId::SUBFAMILY_NAME,
                ),
                mahawir,
            )
        };
        let huwiya = HuwiyatKhatt::min_bayt(bayt.as_slice(), fahras);
        Ok(Self {
            bayt,
            fahras,
            huwiya,
            aila,
            namat,
            mahawir,
        })
    }

    /// This font's identity, which is what the atlas and the layout cache key
    /// on.
    #[must_use]
    pub const fn huwiya(&self) -> HuwiyatKhatt {
        self.huwiya
    }

    /// The one shared buffer.
    ///
    /// Returned as a reference to the `Arc` so a caller that needs to keep the
    /// font alive clones the reference count rather than the bytes.
    #[must_use]
    pub const fn bayt(&self) -> &Arc<Vec<u8>> {
        &self.bayt
    }

    /// The face index inside a collection, `0` for a single-face file.
    #[must_use]
    pub const fn fahras(&self) -> u32 {
        self.fahras
    }

    /// The family name, preferring the typographic family over the legacy one
    /// so that a face like `Semi Bold` reports the family a designer meant
    /// rather than the four-style family Windows once required.
    #[must_use]
    pub fn aila(&self) -> &str {
        &self.aila
    }

    /// The style name within the family — `Regular`, `Bold`, `Italic`.
    #[must_use]
    pub fn namat(&self) -> &str {
        &self.namat
    }

    /// The variation axes this font defines, empty for a static font.
    #[must_use]
    pub fn mahawir(&self) -> &[MihwarKhatt] {
        &self.mahawir
    }

    /// Whether this font is variable.
    #[must_use]
    pub const fn mutaghayyir(&self) -> bool {
        !self.mahawir.is_empty()
    }

    /// The font's own metrics, scaled to `hajm` pixels.
    ///
    /// Everything is scaled from the font's units per em. Where `OS/2` declares
    /// a cap height or an x-height, that value is used. Where it does not — and
    /// plenty of Arabic fonts do not, because neither concept is native to the
    /// script — the value is measured from the **actual outline bounds** of a
    /// representative glyph rather than guessed as a fraction of the size. A
    /// constant fraction is what makes a Naskh face and a Kufi face at the same
    /// pixel size disagree about where the middle of a line is, and an
    /// interface that centres text on that number visibly steps between them.
    ///
    /// If no representative glyph exists in the font, the height is reported as
    /// zero. Zero is honest; an invented number is not.
    ///
    /// `hajm` is taken as given. A size that is zero or negative is refused
    /// earlier, by the layout request, as
    /// [`KhataSaff::HajmGhayrSalih`](crate::khata::KhataSaff::HajmGhayrSalih);
    /// this function does not second-guess it, because a metrics query at an
    /// unusual size is a legitimate thing for the workspace to ask.
    #[must_use]
    pub fn qiyasat(&self, hajm: f32) -> QiyasatKhatt {
        // The bytes parsed at construction and the buffer is immutable, so this
        // cannot fail. A zeroed result is the non-panicking answer if it ever
        // somehow does.
        let Ok(khatt) = self.khatt() else {
            return QiyasatKhatt::default();
        };
        let hajm_bikselat = Size::new(hajm);
        let qiyas = khatt.metrics(hajm_bikselat, LocationRef::default());
        let kharita = khatt.charmap();
        let hudud = khatt.glyph_metrics(hajm_bikselat, LocationRef::default());

        let suud = qiyas.ascent;
        // The font stores descent as a negative distance from the baseline.
        let hubut = -qiyas.descent;
        let fajwa = qiyas.leading;

        QiyasatKhatt {
            suud,
            hubut,
            fajwa,
            irtifa_satr: suud + hubut + fajwa,
            uluw_kabital: qiyas
                .cap_height
                .unwrap_or_else(|| uluw_min_shakl(&kharita, &hudud, &NAMADHIJ_KABITAL)),
            uluw_saghir: qiyas
                .x_height
                .unwrap_or_else(|| uluw_min_shakl(&kharita, &hudud, &NAMADHIJ_SAGHIR)),
            wahdat: qiyas.units_per_em,
        }
    }

    /// The glyph identifier this font maps a character to, if any.
    ///
    /// This is a coverage query, not a shaping shortcut. The identifier it
    /// returns is the *nominal* one from `cmap`, before any substitution, and
    /// it is never what gets drawn: contextual forms and ligatures come out of
    /// `GSUB` during shaping. Nothing downstream draws from this value.
    #[must_use]
    pub fn muarrif(&self, harf: char) -> Option<u32> {
        let khatt = self.khatt().ok()?;
        khatt.charmap().map(harf).map(GlyphId::to_u32)
    }

    /// Whether this font covers a character.
    ///
    /// A mapping to glyph 0 counts as no coverage: glyph 0 is `.notdef`, the
    /// empty box, and a font that maps a character there has not covered it.
    #[must_use]
    pub fn yughatti(&self, harf: char) -> bool {
        matches!(self.muarrif(harf), Some(muarrif) if muarrif != GlyphId::NOTDEF.to_u32())
    }

    /// Decision 6, enforced.
    ///
    /// Checks, in this order and stopping at the first failure so the message
    /// names one specific thing:
    ///
    /// 1. `cmap`, `GSUB` and `GPOS` are present;
    /// 2. `GSUB` carries `init`, `medi`, `fina`, `isol` and `rlig`;
    /// 3. `GPOS` carries `mark`;
    /// 4. the font covers the representative Arabic set — the twenty-eight
    ///    letters, hamza and the alef forms `lam` ligates with, `teh marbuta`,
    ///    `alef maqsura`, the tatweel, and the five common marks.
    ///
    /// The order is the order of consequence. Missing `GSUB` means no Arabic at
    /// all; missing `init` means twenty-eight disconnected letterforms; missing
    /// `mark` means diacritics stacked on the baseline; a coverage hole means
    /// empty boxes in ordinary words. Each is a different sentence to the
    /// person choosing the font, and each one of them is actionable.
    ///
    /// Features are read from the top-level `FeatureList` of each table, which
    /// enumerates every feature the font defines for any script. A tag that is
    /// absent there is absent everywhere.
    ///
    /// # Errors
    ///
    /// [`KhataKhatt::JadwalMafqud`] naming the missing table,
    /// [`KhataKhatt::SifaMafquda`] naming the missing feature,
    /// [`KhataKhatt::TaghtiyaNaqisa`] naming how many characters are missing and
    /// which one comes first, or [`KhataKhatt::TahleelFashil`] when a table is
    /// present but does not parse — a font lying about its own contents.
    pub fn fahs_arabi(&self) -> Natija<()> {
        let khatt = self.khatt()?;

        for (ism, wasm) in JADAWIL_MATLUBA {
            if khatt.table_data(wasm).is_none() {
                return Err(Khata::min_tafsir(&KhataKhatt::JadwalMafqud { jadwal: ism }));
            }
        }

        let gsub = khatt.gsub().map_err(|sabab| talif(&sabab))?;
        let qaimat_gsub = gsub.feature_list().map_err(|sabab| talif(&sabab))?;
        let sifat_gsub = qaimat_gsub.feature_records();
        for (ism, wasm) in SIFAT_GSUB_MATLUBA {
            if !sifat_gsub.iter().any(|sifa| sifa.feature_tag() == wasm) {
                return Err(Khata::min_tafsir(&KhataKhatt::SifaMafquda { sifa: ism }));
            }
        }

        let gpos = khatt.gpos().map_err(|sabab| talif(&sabab))?;
        let qaimat_gpos = gpos.feature_list().map_err(|sabab| talif(&sabab))?;
        let sifat_gpos = qaimat_gpos.feature_records();
        for (ism, wasm) in SIFAT_GPOS_MATLUBA {
            if !sifat_gpos.iter().any(|sifa| sifa.feature_tag() == wasm) {
                return Err(Khata::min_tafsir(&KhataKhatt::SifaMafquda { sifa: ism }));
            }
        }

        let kharita = khatt.charmap();
        let mut adad: u32 = 0;
        let mut awwal: Option<u32> = None;
        for harf in HURUF_MATLUBA {
            if kharita
                .map(harf)
                .is_none_or(|muarrif| muarrif == GlyphId::NOTDEF)
            {
                adad = adad.saturating_add(1);
                let _ = awwal.get_or_insert_with(|| u32::from(harf));
            }
        }
        if let Some(awwal) = awwal {
            return Err(Khata::min_tafsir(&KhataKhatt::TaghtiyaNaqisa {
                adad,
                awwal,
            }));
        }

        Ok(())
    }

    /// Whether this font covers every character of a string the caller intends
    /// to render with it.
    ///
    /// This is what the patch compiler asks before it commits a font to a
    /// patch: the strings are known, so the coverage question has a real answer
    /// ahead of time rather than a box in the middle of a sentence at runtime.
    ///
    /// Invisible controls — bidirectional overrides, isolates, joiners, the
    /// byte-order mark — are skipped. The shaper consumes them and never asks
    /// for a glyph, so requiring one would reject every font in existence.
    ///
    /// # Errors
    ///
    /// [`KhataKhatt::TaghtiyaNaqisa`] with the number of distinct characters
    /// missing and the first one in reading order, so the message can name a
    /// real character rather than a count.
    pub fn fahs_taghtiya(&self, huruf: &str) -> Natija<()> {
        let khatt = self.khatt()?;
        let kharita = khatt.charmap();
        let mut ruiya: FxHashSet<char> = FxHashSet::default();
        let mut adad: u32 = 0;
        let mut awwal: Option<u32> = None;

        for harf in huruf.chars() {
            if ghayr_marii(harf) || !ruiya.insert(harf) {
                continue;
            }
            if kharita
                .map(harf)
                .is_none_or(|muarrif| muarrif == GlyphId::NOTDEF)
            {
                adad = adad.saturating_add(1);
                let _ = awwal.get_or_insert_with(|| u32::from(harf));
            }
        }

        if let Some(awwal) = awwal {
            return Err(Khata::min_tafsir(&KhataKhatt::TaghtiyaNaqisa {
                adad,
                awwal,
            }));
        }

        Ok(())
    }

    /// The shared buffer, parsed.
    ///
    /// This is the single door to the font's tables, and it is the whole of
    /// Decision 4 in one function: the returned `FontRef` borrows the same bytes
    /// every other consumer borrows, so the shaper, the rasterizer and the
    /// metrics queries cannot disagree about what glyph 4211 is. It is also
    /// cheap — a `FontRef` is a table directory over a slice, not a decoded
    /// font — which is why nothing here caches one and risks a second parse
    /// drifting away from the first.
    ///
    /// # Errors
    ///
    /// [`KhataKhatt::TahleelFashil`] or [`KhataKhatt::FahrasKharij`], neither of
    /// which can happen after construction succeeded, because the buffer is
    /// immutable and the index was checked then.
    pub fn khatt(&self) -> Natija<FontRef<'_>> {
        tahleel(self.bayt.as_slice(), self.fahras)
    }
}

impl fmt::Debug for MawridKhatt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The buffer is reported as a length. A debug line that dumps a
        // megabyte of font into a log is a log nobody can read.
        f.debug_struct("MawridKhatt")
            .field("huwiya", &self.huwiya)
            .field("fahras", &self.fahras)
            .field("aila", &self.aila)
            .field("namat", &self.namat)
            .field("mahawir", &self.mahawir.len())
            .field("bayt", &self.bayt.len())
            .finish()
    }
}

/// An ordered chain of fonts, tried in order for each character.
///
/// This is a fallback *chain*, not a fallback shaper. Nothing here synthesises
/// a letterform: when the primary font does not cover a character, the next
/// font that does is used for that run, and the layout records which font each
/// glyph came from so the atlas knows where to fetch its outline. A run is
/// never split in the middle of a joined word for any reason except a genuine
/// coverage boundary, because a boundary inside a word severs the join and no
/// later stage can put it back.
#[derive(Debug, Clone)]
pub struct SilsilatKhutut {
    /// The first font, held separately so that [`SilsilatKhutut::awwal`] is
    /// total without an index or an unwrap. This is an `Arc` clone of the first
    /// entry of `khutut` — a reference count, not a second byte buffer, so
    /// Decision 4 still holds.
    awwal: Arc<MawridKhatt>,
    khutut: Vec<Arc<MawridKhatt>>,
}

impl SilsilatKhutut {
    /// Builds a chain.
    ///
    /// # Errors
    ///
    /// [`KhataSaff::SilsilaFarigha`] when the chain is empty. There is nothing
    /// to draw with, and every later stage would have to invent an answer.
    ///
    /// [`KhataKhatt::SilsilaTaweela`] when it holds more than 256 fonts. A
    /// positioned glyph names its font in a single byte
    /// ([`crate::natija::Harf::khatt`]), so a font past index 255 could never be
    /// referred to by anything this engine produces. Refusing is better than
    /// keeping the first 256: a caller that assembled a chain that long has a
    /// bug, and dropping the tail would surface it much later as glyphs missing
    /// from the game rather than here, as a sentence naming the count.
    pub fn jadeeda(khutut: Vec<Arc<MawridKhatt>>) -> Natija<Self> {
        if khutut.len() > AQSA_KHUTUT {
            let adad = u32::try_from(khutut.len()).unwrap_or(u32::MAX);
            return Err(Khata::min_tafsir(&KhataKhatt::SilsilaTaweela { adad }));
        }
        let Some(awwal) = khutut.first().map(Arc::clone) else {
            return Err(Khata::min_tafsir(&KhataSaff::SilsilaFarigha));
        };
        Ok(Self { awwal, khutut })
    }

    /// A chain of one font.
    ///
    /// # Errors
    ///
    /// Cannot fail. The signature matches [`SilsilatKhutut::jadeeda`] so that a
    /// caller can move between the two without changing how it handles the
    /// result.
    pub fn wahid(khatt: Arc<MawridKhatt>) -> Natija<Self> {
        Self::jadeeda(vec![khatt])
    }

    /// The primary font — the one the text is designed around and the one
    /// whose metrics set the line height.
    #[must_use]
    pub const fn awwal(&self) -> &Arc<MawridKhatt> {
        &self.awwal
    }

    /// The font at an index, or `None` when the index is past the chain.
    #[must_use]
    pub fn khatt(&self, fahras: u8) -> Option<&Arc<MawridKhatt>> {
        self.khutut.get(usize::from(fahras))
    }

    /// How many fonts the chain holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.khutut.len()
    }

    /// Picks the font that will draw one character.
    ///
    /// The preferred font wins when it covers the character — that is how a
    /// style span pins a word to a particular face. Otherwise the first font in
    /// the chain that covers it wins. When no font covers it at all the answer
    /// is index 0, which will render `.notdef`: an empty box, visible in the
    /// game, traceable in the overflow report, and fixable by adding a font to
    /// the chain.
    ///
    /// That last case is deliberate and it is not a fallback in disguise. The
    /// alternative — refusing to lay out the line — turns one missing character
    /// into a blank menu, and the alternative to *that* — substituting some
    /// other glyph — is a lie in the player's face. A visible box is the honest
    /// answer, and it is the one a translator can see and report.
    #[must_use]
    pub fn ikhtiyar(&self, harf: char, mufaddal: Option<u8>) -> u8 {
        if let Some(fahras) = mufaddal
            && self.khatt(fahras).is_some_and(|khatt| khatt.yughatti(harf))
        {
            return fahras;
        }
        for (fahras, khatt) in self.khutut.iter().enumerate() {
            if khatt.yughatti(harf) {
                // A chain longer than 256 was refused at construction, so every
                // index here fits in a u8.
                return u8::try_from(fahras).unwrap_or(0);
            }
        }
        0
    }

    /// Every font in the chain, in order.
    #[must_use]
    pub fn khutut(&self) -> &[Arc<MawridKhatt>] {
        &self.khutut
    }
}

/// Parses one face out of a buffer, handling collections.
///
/// `ttc` and `otc` files hold several faces behind one table-directory offset
/// table. `FileRef` tells the two shapes apart, and an index past the end is
/// reported as [`KhataKhatt::FahrasKharij`] with the real count, so the message
/// can say how many faces the file actually has.
fn tahleel(bayt: &[u8], fahras: u32) -> Natija<FontRef<'_>> {
    let malaf = FileRef::new(bayt).map_err(|sabab| talif(&sabab))?;
    match malaf {
        FileRef::Font(khatt) => {
            if fahras == 0 {
                Ok(khatt)
            } else {
                Err(Khata::min_tafsir(&KhataKhatt::FahrasKharij {
                    fahras,
                    adad: 1,
                }))
            }
        },
        FileRef::Collection(majmua) => {
            let adad = majmua.len();
            if fahras >= adad {
                return Err(Khata::min_tafsir(&KhataKhatt::FahrasKharij {
                    fahras,
                    adad,
                }));
            }
            majmua.get(fahras).map_err(|sabab| talif(&sabab))
        },
    }
}

/// Wraps a parser failure as context rather than as the message. The sentence
/// the user reads is Taarib's; what `read-fonts` reported travels underneath it
/// for the diagnostics bundle.
fn talif(sabab: &ReadError) -> Khata {
    Khata::min_tafsir(&KhataKhatt::TahleelFashil {
        tafsil: sabab.to_string(),
    })
}

/// Reads a name from the `name` table, preferring the typographic id over the
/// legacy one.
///
/// The legacy family and subfamily ids exist to squeeze a whole family into the
/// four styles a 1990s system could express, so a nine-weight family reports
/// itself as several families named `Cairo Light`, `Cairo SemiBold` and so on.
/// The typographic ids carry what the designer meant. An empty string is
/// returned when the font has neither, which means the `name` table is broken —
/// and that is not, on its own, a reason to refuse to draw with it.
fn nass_khatt(khatt: &FontRef<'_>, mufaddal: StringId, ihtiyati: StringId) -> String {
    khatt
        .localized_strings(mufaddal)
        .english_or_first()
        .or_else(|| khatt.localized_strings(ihtiyati).english_or_first())
        .map(|nass| nass.to_string())
        .unwrap_or_default()
}

/// Measures a height from the real outline bounds of the first representative
/// glyph the font actually has.
///
/// Bounds are already in pixels because [`GlyphMetrics`] was built at the
/// requested size. A glyph whose top is at or below the baseline is skipped:
/// it is either `.notdef` or a substitution that tells us nothing about the
/// height we are asking after.
fn uluw_min_shakl(kharita: &Charmap<'_>, hudud: &GlyphMetrics<'_>, namadhij: &[char]) -> f32 {
    for harf in namadhij {
        if let Some(muarrif) = kharita.map(*harf)
            && let Some(hadd) = hudud.bounds(muarrif)
            && hadd.y_max > 0.0
        {
            return hadd.y_max;
        }
    }
    0.0
}

/// Characters the shaper consumes without ever asking the font for a glyph.
///
/// Bidirectional embeddings, overrides and isolates, the joiners, the word
/// joiner, and the byte-order mark are all instructions to the layout rather
/// than things to draw. A font is not incomplete for lacking them.
fn ghayr_marii(harf: char) -> bool {
    harf.is_control()
        || matches!(
            u32::from(harf),
            0x200B..=0x200F | 0x2028..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F | 0xFEFF
        )
}
