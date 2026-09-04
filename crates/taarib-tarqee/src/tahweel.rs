//! التحويل — the project's vocabulary, translated into the container's records.
//!
//! Two data models describe the same facts. The project's — [`NitaqNasq`],
//! [`QuyudNass`], [`MiftahShakl`] — is rich, owns its strings, and is what a
//! workshop edits. The container's is a run of fixed-layout structs that C#,
//! JavaScript, Python and Ruby overlay onto mapped bytes without a parser.
//!
//! This module is the only place the first becomes the second, and it exists as
//! a module rather than as a scatter of `From` impls because the conversion is
//! **lossy in specific, deliberate ways** and every one of those losses needs
//! to be written down next to the others rather than discovered one field at a
//! time by somebody debugging a patch.
//!
//! ## What is lost, and why each loss is correct
//!
//! A span's *payload* does not survive. [`NawNasq::Khatt`] names a font family
//! the engine knows; [`NawNasq::Rabt`] carries a link payload;
//! [`NawNasq::Muhadhaha`] carries an alignment word. The container has one
//! `u16` of flags, one `u32` of colour and one `f32` of size per span, and
//! nowhere to put a string.
//!
//! That is not a shortcoming to be worked around. The adapter reading these
//! records is drawing glyphs the compiler already positioned; it is not
//! re-running the engine's own markup interpreter. A span whose only content is
//! a payload the renderer cannot act on is a span that would occupy four bytes
//! per string in every patch to be ignored at every draw.
//!
//! What *is* kept is everything that changes pixels: whether the run is bold,
//! italic, coloured, resized, an inline sprite, or an atom the shaper must not
//! touch. Those decide what is drawn. The rest decides what the engine would
//! have done, and the engine is no longer the one drawing.
//!
//! ## Alignment, direction and the overflow policy are *not* lost
//!
//! They move from the span table to the constraint record, which is where the
//! adapter looks for them, and they are per-string rather than per-span because
//! no engine this product targets aligns two halves of one label differently.
//!
//! ## Zero is not a value
//!
//! Several container fields use zero to mean "not set": a span's colour, a
//! span's size override, a constraint's available width. Every one of those is
//! paired with a flag, or with a documented "zero or less means unbounded", and
//! this module never writes a meaningful zero into a field whose zero means
//! absent. A colour of `0x00000000` written without [`ALAM_NITAQ_LAWN`] would
//! be read as "no colour", which is right — transparent black is not a colour
//! anybody asked to draw text in.

use taarib_lawha::khareeta::{MawdiShakl, MiftahShakl};
use taarib_mustalahat::nass::{MudkhalNass, NawNasq, NitaqNasq, QuyudNass, TasnifNass};
use taarib_ruqaa::jadawil::{
    ALAM_NITAQ_ASWAD, ALAM_NITAQ_DHARRA, ALAM_NITAQ_LAWN, ALAM_NITAQ_MAAIL, ALAM_NITAQ_SURA,
    ALAM_QAYD_HAJM_TILQAI, ALAM_QAYD_HIWAR, ALAM_QAYD_SATR_WAHID, SijillMawdiShakl,
    SijillMiftahShakl, SijillNitaq, SijillQayd,
};
use taarib_saff::talab::{
    IttijahAsas, KhiyaratTakhtit, LughaNass, Muhadhaha, NamatDabt, SiyasatArqam, SiyasatTajawuz,
    SiyasatTashkeel,
};

/// The alignment code the container stores: leading edge.
pub const MUHADHAHA_BIDAYA: u8 = 0;
/// Trailing edge.
pub const MUHADHAHA_NIHAYA: u8 = 1;
/// Centred.
pub const MUHADHAHA_WASAT: u8 = 2;
/// Justified.
pub const MUHADHAHA_MADDUD: u8 = 3;

/// The direction code: let the text decide.
pub const ITTIJAH_TILQAI: u8 = 0;
/// Right to left.
pub const ITTIJAH_YAMEEN: u8 = 1;
/// Left to right.
pub const ITTIJAH_YASAR: u8 = 2;

/// The overflow code: measure it, report it, draw it as it is.
pub const TAJAWUZ_TAQREER: u8 = 0;
/// Shrink until it fits.
pub const TAJAWUZ_TASGHEER: u8 = 1;
/// Cut it off.
pub const TAJAWUZ_QASS: u8 = 2;

/// Converts one markup span.
///
/// `id` is preserved exactly, because a positioned glyph's `nitaq` field names
/// it and a renumbering here would silently repaint the wrong run.
///
/// `nass` is left at zero: the writer overwrites it with the string's final
/// index at [`taarib_ruqaa::katib::Katib::ikhtim`], after the table sort, and a
/// value written here would be a value that stops being true.
#[must_use]
pub fn nitaq(masdar: &NitaqNasq) -> SijillNitaq {
    let mut alam: u16 = 0;
    let mut lawn: u32 = 0;
    let mut hajm: f32 = 0.0;

    match &masdar.naw {
        NawNasq::Lawn { qeema } => {
            if let Some(muhallal) = hallil_lawn(qeema) {
                lawn = muhallal;
                alam |= ALAM_NITAQ_LAWN;
            }
        }
        NawNasq::Ghaliz => alam |= ALAM_NITAQ_ASWAD,
        NawNasq::Maail => alam |= ALAM_NITAQ_MAAIL,
        NawNasq::Hajm { qeema } => {
            // A size override of zero or less is not a size. Writing it would
            // land in a field whose zero already means "no override", so the
            // record would say the same thing either way — and the flagless
            // path is the honest one.
            if *qeema > 0.0 {
                hajm = *qeema;
            }
        }
        NawNasq::Sura { .. } => alam |= ALAM_NITAQ_SURA | ALAM_NITAQ_DHARRA,
        NawNasq::Mawdi { .. } => alam |= ALAM_NITAQ_DHARRA,
        // Underline, strikethrough, hard breaks, typewriter pauses, no-parse
        // regions, link payloads, font-family switches and alignment overrides.
        // The first two have no container flag because no adapter draws a rule
        // under a glyph run from the atlas; the rest are engine instructions
        // rather than pixels. See this module's header.
        NawNasq::TahtKhat
        | NawNasq::Shatb
        | NawNasq::Khatt { .. }
        | NawNasq::Rabt { .. }
        | NawNasq::Satr
        | NawNasq::Tawaqquf { .. }
        | NawNasq::BilaTahleel
        | NawNasq::Muhadhaha { .. } => {}
    }

    SijillNitaq { nass: 0, bidaya: masdar.bidaya, tul: masdar.tul, lawn, hajm, id: masdar.id, alam }
}

/// Parses a colour the engine wrote, or answers [`None`].
///
/// `#rgb`, `#rrggbb` and `#rrggbbaa` are accepted, with or without the hash.
/// Named colours are **not** guessed: "red" means a different value in Unity's
/// rich text, in RPG Maker's colour table and in a Ren'Py style, and picking
/// one of them would recolour text in two engines out of three. A colour that
/// cannot be read produces no flag, and the run draws in the base colour — a
/// visible, correctable result rather than a confidently wrong one.
#[must_use]
pub fn hallil_lawn(qeema: &str) -> Option<u32> {
    let nass = qeema.trim().trim_start_matches('#');
    if !nass.bytes().all(|bayt| bayt.is_ascii_hexdigit()) {
        return None;
    }
    let arkam: Vec<u32> = nass
        .chars()
        .map(|harf| harf.to_digit(16).unwrap_or(0))
        .collect();

    let rakm = |fahras: usize| arkam.get(fahras).copied().unwrap_or(0);
    let zawj = |ala: usize, adna: usize| (rakm(ala) << 4) | rakm(adna);
    let mudaaf = |fahras: usize| (rakm(fahras) << 4) | rakm(fahras);

    let (ahmar, akhdar, azraq, shaffaf) = match arkam.len() {
        3 => (mudaaf(0), mudaaf(1), mudaaf(2), 0xff),
        4 => (mudaaf(0), mudaaf(1), mudaaf(2), mudaaf(3)),
        6 => (zawj(0, 1), zawj(2, 3), zawj(4, 5), 0xff),
        8 => (zawj(0, 1), zawj(2, 3), zawj(4, 5), zawj(6, 7)),
        _ => return None,
    };
    Some((ahmar << 24) | (akhdar << 16) | (azraq << 8) | shaffaf)
}

/// The overflow byte, and the size floor that goes with it.
///
/// Returned together because they are one decision.
/// [`SiyasatTajawuz::Taqlis`] carries the floor it shrinks to, and a record
/// that stored the code without the floor would tell an adapter to shrink with
/// no limit — down to a size at which the text is present, illegible and
/// technically fitting.
#[must_use]
pub const fn ramz_tajawuz(siyasa: SiyasatTajawuz) -> (u8, f32) {
    match siyasa {
        SiyasatTajawuz::Ballagh => (TAJAWUZ_TAQREER, 0.0),
        SiyasatTajawuz::Taqlis { adna } => (TAJAWUZ_TASGHEER, adna),
        SiyasatTajawuz::Ikhtisar => (TAJAWUZ_QASS, 0.0),
    }
}

/// The alignment byte.
#[must_use]
pub const fn ramz_muhadhaha(muhadhaha: Muhadhaha) -> u8 {
    match muhadhaha {
        Muhadhaha::Bidaya => MUHADHAHA_BIDAYA,
        Muhadhaha::Nihaya => MUHADHAHA_NIHAYA,
        Muhadhaha::Wasat => MUHADHAHA_WASAT,
        Muhadhaha::Dabt => MUHADHAHA_MADDUD,
    }
}

/// The base-direction byte.
#[must_use]
pub const fn ramz_ittijah(ittijah: IttijahAsas) -> u8 {
    match ittijah {
        IttijahAsas::Tilqai => ITTIJAH_TILQAI,
        IttijahAsas::Yameen => ITTIJAH_YAMEEN,
        IttijahAsas::Yasar => ITTIJAH_YASAR,
    }
}

/// The justification byte: 0 none, 1 spaces, 2 kashida, 3 kashida then spaces.
#[must_use]
pub const fn ramz_dabt(dabt: NamatDabt) -> u8 {
    match dabt {
        NamatDabt::Bila => 0,
        NamatDabt::Masafat => 1,
        NamatDabt::Kashida => 2,
        NamatDabt::KashidaThummaMasafat => 3,
    }
}

/// The diacritics byte: 0 keep, 1 strip, 2 keep in dialogue only.
#[must_use]
pub const fn ramz_tashkeel(tashkeel: SiyasatTashkeel) -> u8 {
    match tashkeel {
        SiyasatTashkeel::Ibqa => 0,
        SiyasatTashkeel::Hadhf => 1,
        SiyasatTashkeel::IbqaFilHiwar => 2,
    }
}

/// The digits byte: 0 leave, 1 European, 2 Arabic-Indic, 3 Eastern.
#[must_use]
pub const fn ramz_arqam(arqam: SiyasatArqam) -> u8 {
    match arqam {
        SiyasatArqam::KamaHiya => 0,
        SiyasatArqam::Latini => 1,
        SiyasatArqam::Arabi => 2,
        SiyasatArqam::Farisi => 3,
    }
}

/// The language byte: 0 detect, 1 Arabic, 2 Persian, 3 Urdu, 4 Latin.
#[must_use]
pub const fn ramz_lugha(lugha: LughaNass) -> u8 {
    match lugha {
        LughaNass::Tilqai => 0,
        LughaNass::Arabi => 1,
        LughaNass::Farisi => 2,
        LughaNass::Urdu => 3,
        LughaNass::Latini => 4,
    }
}

/// Converts one string's recorded constraint.
///
/// `nass` is left at zero for the same reason as in [`nitaq`].
///
/// `khiyarat` is the **same** [`KhiyaratTakhtit`] precomputation laid the string
/// out with, and every policy byte in the record is derived from it rather than
/// chosen here. That is the point of the record: an adapter falling back to the
/// runtime path has to reproduce the decisions the compiled layouts were made
/// under, and a second set of defaults living in the compiler would be a second
/// answer to every one of those questions — visible only as text that shapes one
/// way when it comes out of the container and another way when it does not.
///
/// An unmeasured width becomes zero, which the container documents as
/// unbounded. That is the correct reading: nothing measured the box, so nothing
/// should pretend to know where its edge is, and an adapter told "unbounded"
/// falls back to the engine's own wrapping rather than to a made-up number.
#[must_use]
pub fn qayd(quyud: &QuyudNass, tasnif: TasnifNass, khiyarat: &KhiyaratTakhtit) -> SijillQayd {
    let mut alam: u32 = 0;
    // Either the engine said so for this string, or the compile forbade it for
    // every string. Both are real reasons a line may not break.
    if quyud.satr_wahid || khiyarat.satr_wahid {
        alam |= ALAM_QAYD_SATR_WAHID;
    }
    if matches!(tasnif, TasnifNass::Hiwar) {
        alam |= ALAM_QAYD_HIWAR;
    }

    let (tajawuz, hajm_adna) = ramz_tajawuz(khiyarat.tajawuz);
    if hajm_adna > 0.0 {
        alam |= ALAM_QAYD_HAJM_TILQAI;
    }

    SijillQayd {
        nass: 0,
        ard_mutah: quyud.aqsa_ard.unwrap_or(0.0),
        irtifa_mutah: quyud.aqsa_irtifa.unwrap_or(0.0),
        hajm: quyud.hajm_khatt.unwrap_or(0.0),
        // Meaningful only with `ALAM_QAYD_HAJM_TILQAI`, which is set exactly
        // when the shrink policy supplied a floor. A floor written without the
        // flag would be a number an adapter might act on for an element the
        // compile never intended to shrink.
        hajm_adna,
        // Zero lets the font's own metrics decide, which is what `None` means.
        irtifa_satr: khiyarat.irtifa_satr.unwrap_or(0.0),
        tabaud_ahruf: khiyarat.tabaud_ahruf,
        tabaud_kalimat: khiyarat.tabaud_kalimat,
        alam,
        muhadhaha: ramz_muhadhaha(khiyarat.muhadhaha),
        ittijah: ramz_ittijah(khiyarat.ittijah),
        dabt: ramz_dabt(khiyarat.dabt),
        tashkeel: ramz_tashkeel(khiyarat.tashkeel),
        arqam: ramz_arqam(khiyarat.arqam),
        tajawuz,
        lugha: ramz_lugha(khiyarat.lugha),
        hashw0: 0,
        hashw1: 0,
    }
}

/// The layout decisions a compile made, in the container's own encoding.
///
/// The same bytes [`qayd`] writes into every constraint record, gathered once
/// for the manifest. Two reasons for the duplication, and neither is
/// convenience.
///
/// A patch may legitimately have **no** `QIYUD` section at all: a game whose
/// strings carry no measured width and no character limit produces no useful
/// constraint records, and the section is omitted rather than filled with rows
/// of zeroes. Without this, such a patch would carry no record anywhere of the
/// direction, justification, diacritic and digit policy it was compiled under —
/// and those are precisely the decisions a reviewer questions.
///
/// And a reviewer reading a manifest should not have to decompress a POD table
/// and index into it to find out whether a patch strips diacritics.
///
/// Serializable in both directions, unlike the reports around it: these are
/// settings, not measurements. There is no invariant a stored document could
/// forge — the worst a hand-edited copy produces is a manifest that disagrees
/// with the constraint records, and the records are what an adapter reads.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SiyasatHuzma {
    /// Where a line sits: 0 leading edge, 1 trailing, 2 centre, 3 filled.
    pub muhadhaha: u8,
    /// Base direction: 0 automatic, 1 right to left, 2 left to right.
    pub ittijah: u8,
    /// Surplus width: 0 none, 1 spaces, 2 kashida, 3 kashida then spaces.
    pub dabt: u8,
    /// Diacritics: 0 keep, 1 strip, 2 keep in dialogue only.
    pub tashkeel: u8,
    /// Digits: 0 leave, 1 European, 2 Arabic-Indic, 3 Eastern Arabic-Indic.
    pub arqam: u8,
    /// Overflow: 0 report, 1 shrink to fit, 2 truncate.
    pub tajawuz: u8,
    /// Language: 0 detect, 1 Arabic, 2 Persian, 3 Urdu, 4 Latin.
    pub lugha: u8,
    /// The floor the shrink policy stops at, or zero when it does not shrink.
    pub hajm_adna: f32,
    /// Line height in pixels; zero lets the font's metrics decide.
    pub irtifa_satr: f32,
    /// Extra spacing between glyphs, in pixels.
    pub tabaud_ahruf: f32,
    /// Extra spacing added to every space, in pixels.
    pub tabaud_kalimat: f32,
    /// Whether the compile forbade wrapping for every string.
    pub satr_wahid: bool,
}

impl SiyasatHuzma {
    /// Reads the decisions out of the options precomputation used.
    #[must_use]
    pub fn min_khiyarat(khiyarat: &KhiyaratTakhtit) -> Self {
        let (tajawuz, hajm_adna) = ramz_tajawuz(khiyarat.tajawuz);
        Self {
            muhadhaha: ramz_muhadhaha(khiyarat.muhadhaha),
            ittijah: ramz_ittijah(khiyarat.ittijah),
            dabt: ramz_dabt(khiyarat.dabt),
            tashkeel: ramz_tashkeel(khiyarat.tashkeel),
            arqam: ramz_arqam(khiyarat.arqam),
            tajawuz,
            lugha: ramz_lugha(khiyarat.lugha),
            hajm_adna,
            irtifa_satr: khiyarat.irtifa_satr.unwrap_or(0.0),
            tabaud_ahruf: khiyarat.tabaud_ahruf,
            tabaud_kalimat: khiyarat.tabaud_kalimat,
            satr_wahid: khiyarat.satr_wahid,
        }
    }
}

/// Whether a string's constraint carries anything worth storing.
///
/// A record of all zeroes says nothing an adapter can use and costs forty
/// bytes plus an index in every patch that ships it. This is what the assembler
/// asks before adding one.
#[must_use]
pub const fn qayd_mufid(quyud: &QuyudNass, tasnif: TasnifNass) -> bool {
    quyud.aqsa_ard.is_some()
        || quyud.aqsa_irtifa.is_some()
        || quyud.hajm_khatt.is_some()
        || quyud.satr_wahid
        || matches!(tasnif, TasnifNass::Hiwar)
}

/// Converts a glyph key.
#[must_use]
pub const fn miftah_shakl(miftah: MiftahShakl) -> SijillMiftahShakl {
    SijillMiftahShakl {
        muarrif: miftah.muarrif,
        hajm_rubi: miftah.hajm_rubi,
        khatt: miftah.khatt,
        bakat: miftah.bakat,
    }
}

/// Converts a glyph's place in the atlas.
///
/// The rasterization mode does not travel in the key record: the container
/// declares one mode for the whole atlas in the header's flags, because a page
/// is coverage or distance field and cannot be both, and a per-glyph mode would
/// let a patch describe an atlas that cannot exist.
#[must_use]
pub const fn mawdi_shakl(mawdi: &MawdiShakl) -> SijillMawdiShakl {
    SijillMawdiShakl {
        taqaddum: mawdi.taqaddum,
        s: mawdi.s,
        a: mawdi.a,
        ard: mawdi.ard,
        irtifa: mawdi.irtifa,
        izaha_s: mawdi.izaha_s,
        izaha_a: mawdi.izaha_a,
        safha: mawdi.safha,
        hashw: 0,
    }
}

/// Every span over one string's translation, converted.
///
/// Spans over the *source* are deliberately not converted. The container's text
/// is the translation; a span whose offsets index the English would point into
/// the middle of an Arabic code point.
#[must_use]
pub fn nitaqat_hadaf(mudkhal: &MudkhalNass) -> Vec<SijillNitaq> {
    mudkhal.nasq_hadaf.iter().map(nitaq).collect()
}
