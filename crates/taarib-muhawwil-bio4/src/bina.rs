//! البناء — turning real translated strings into a `BIO4` font.
//!
//! One call: shape the strings, collect the glyphs shaping actually produced,
//! rasterize and pack them through `taarib-lawha`, give each a cell, blit each
//! into its cell, and emit the metrics table and the texture that describe the
//! result.
//!
//! ## What this does not do
//!
//! It does not rasterize. `taarib_lawha::Lawha::ibni` does, through
//! `taarib_saff::rasm`, and this module reads the pages it produced. A second
//! rasterizer here would be a second answer to "what does this letter look like",
//! and the two would drift.
//!
//! It does not enumerate code points. The glyph set is exactly what
//! [`taarib_saff::Saff::khattit`] emitted for the strings the caller supplied —
//! every contextual form the font chose, every lam-alef the font ligated, and
//! nothing that was not drawn. An atlas built from a Unicode range would be
//! missing the ligatures and padded with letters the game never shows, in a
//! container whose whole budget is a few hundred cells.
//!
//! ## Placement inside a cell
//!
//! A cell is `pitch` texels square. A glyph is drawn at its own left bearing from
//! the cell's left edge and hung from `asas`, the baseline measured down from the
//! cell's top. Both are clamped into the cell, and every clamp is counted in
//! [`TaqreerBina::ashkal_maqsusa`] — a letter quietly moved half a texel to fit is
//! a letter that no longer sits on the same baseline as the one beside it, and
//! the number is there so a caller can refuse the font rather than ship it.
//!
//! ## The advance the container cannot carry
//!
//! A `BIO4` metrics entry is two bytes: the left and right edges of the ink. It
//! has no advance field. Measured against the one shipped atlas whose art still
//! matches its metrics, the stored pair is exactly the ink's bounding box in 548
//! of 548 cells — so that is what this module writes, and consecutive glyphs abut
//! ink to ink.
//!
//! For Arabic that is close to right and not identical to right: a shaped advance
//! includes side bearings the box does not. The difference is not swallowed —
//! [`TaqreerBina::farq_taqaddum`] is the total, in texels, between the advances
//! the shaper produced and the widths this container can express. A caller that
//! finds it large is looking at a font size the grid is too coarse for.

use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_lawha::misafa::KhiyaratMisafa;
use taarib_lawha::rasf::{KhiyaratRasf, Safha};
use taarib_lawha::tafrigh::JamiAshkal;
use taarib_lawha::{Lawha, MawdiShakl};
use taarib_saff::natija::TakhtitNass;
use taarib_saff::{KhiyaratTakhtit, Saff, SilsilatKhutut, TalabTakhtit};
use taarib_usus::khata::Natija;

use crate::khata::KhataBio4;
use crate::khatt::{KhattBio4, MadkhalKhana};
use crate::naql::{NaqlBio4, NassManqulBio4, TawzeeKhanatBio4};
use crate::shabaka::Shabaka;
use crate::sura::SuraMufakkaka;

/// The widest cell whose metrics fit the container's two bytes.
///
/// A `BIO4` metrics entry is two `u8`, so a cell wider than 255 texels could not
/// have its own right edge written down. Every shipped font is 28 or 20.
pub const AQSA_HAJM_KHANA: u32 = 255;

/// What a build needs to know that the strings and the fonts do not say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KhiyaratBina {
    /// The pixel size to shape and rasterize at.
    pub hajm: f32,
    /// The baseline, in texels down from a cell's top edge.
    ///
    /// This is the number that decides whether Arabic sits where the game's
    /// untranslated interface elements sit. Taking it from the font being
    /// replaced is the whole point of reading that font first.
    pub asas: u32,
    /// The `ImagePack` identifier to write into the new `.fnt`.
    ///
    /// Normally the one the font being replaced already carries, so the rebuilt
    /// font finds the rebuilt texture through the same name the game already
    /// resolves.
    pub hizma: [u8; 4],
    /// The grid of the font being replaced: its cell pitch, its texture width,
    /// and the cell budget its own atlas had.
    pub asli: Shabaka,
}

/// What a build cost and what it could not express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaqreerBina {
    /// Cells the transport allocated, the reserved blank included.
    pub khanat_mustaamala: u32,
    /// Cells the original grid provided.
    pub khanat_mutaha: u32,
    /// Glyphs whose placement had to be clamped into the cell.
    pub ashkal_maqsusa: u32,
    /// The worst single clamp, in texels.
    pub aqsa_tajawuz: u32,
    /// Total difference, in texels, between the advances shaping produced and
    /// the ink widths this container stores.
    pub farq_taqaddum: u32,
    /// Distinct cells in the atlas, the reserved blank excluded.
    pub ashkal: u32,
    /// Marks whose `GPOS` offset differed between two instances of the same
    /// `(base, mark)` pair; the first one seen is the one in the cell.
    pub alamat_mutanaziaa: u32,
    /// Marks that could not be composed into a base and were dropped.
    pub alamat_mafquda: u32,
}

/// A finished font: the container, the texture, the assignment and the lines.
#[derive(Debug, Clone)]
pub struct KhattMabni {
    /// The `.fnt`, ready to serialise.
    pub khatt: KhattBio4,
    /// The atlas, one byte of coverage per texel.
    pub sura: SuraMufakkaka,
    /// Which cell each shaped glyph was given.
    pub tawzee: TawzeeKhanatBio4,
    /// Each registered line, as the cells that draw it.
    pub nusus: Vec<NassManqulBio4>,
    /// What it cost.
    pub taqreer: TaqreerBina,
}

/// Builds a font from real strings.
///
/// `nusus` are the translated lines in logical order and ordinary Unicode. They
/// are shaped here, once, and both the atlas and the transported sequences come
/// out of that one shaping — which is what makes the images in the atlas the
/// images the sequences address.
///
/// # Errors
///
/// [`KhataBio4::BunyaGhayrMutawaqqaa`] when the cell pitch cannot be written in a
/// metrics entry, [`KhataBio4::KhanatNafida`] when the glyph set needs more cells
/// than the original grid had, [`KhataBio4::ShaklAkbarMinKhana`] for a glyph
/// larger than a cell, and whatever `taarib-saff` reports for a line it cannot
/// lay out or `taarib-lawha` for a glyph it cannot draw.
pub fn ibni(
    nusus: &[&str],
    khutut: &SilsilatKhutut,
    takhtit: &KhiyaratTakhtit,
    khiyarat: KhiyaratBina,
) -> Natija<KhattMabni> {
    let hajm_khana = khiyarat.asli.hajm_khana();
    if hajm_khana == 0 || hajm_khana > AQSA_HAJM_KHANA {
        return Err(KhataBio4::BunyaGhayrMutawaqqaa {
            haql: "cell pitch",
            qeema: u64::from(hajm_khana),
            sabab: "cannot be written into a metrics entry, which is two bytes",
        }
        .into());
    }

    let mut saff = Saff::jadeed();
    let mut naql = NaqlBio4::jadeed(khiyarat.asli.siaa());
    let mut jami = JamiAshkal::jadeed(NamatSafha::Taghtiya);
    let hajm_rubi = MiftahShakl::jadeed(0, 0, khiyarat.hajm, NamatSafha::Taghtiya, 0).hajm_rubi;

    for nass in nusus {
        let takhtit_nass = saff.khattit(&TalabTakhtit {
            nass,
            khutut,
            hajm: khiyarat.hajm,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: takhtit,
        })?;
        idif_ashkal(&mut jami, &takhtit_nass, khiyarat.hajm);
        naql.sajjil(&takhtit_nass, hajm_rubi)?;
    }

    let natija = naql.akmil()?;
    let tawzee = natija.tawzee;
    let lawha = Lawha::ibni(
        &jami.ashkal(),
        khutut,
        KhiyaratRasf::default(),
        KhiyaratMisafa::default(),
        NamatSafha::Taghtiya,
    )?;

    let madaa = tawzee.madaa();
    let shabaka = Shabaka::jadeeda(hajm_khana, khiyarat.asli.ard(), madaa)?;
    let mut sura = SuraMufakkaka::jadeeda(shabaka.ard(), shabaka.irtifa())?;

    // Cell zero is the blank every shipped font keeps, encoded the way they
    // encode it: a span whose left edge is at the pitch and whose right edge is
    // at zero. It is also where the pitch is written down.
    let farigha = MadkhalKhana::jadeed(u8::try_from(hajm_khana).unwrap_or(0), 0);
    let mut madakhil = vec![farigha; usize::try_from(madaa).unwrap_or(0)];
    let mut taqreer = TaqreerBina {
        khanat_mustaamala: madaa,
        khanat_mutaha: khiyarat.asli.siaa(),
        ashkal: u32::try_from(tawzee.adad()).unwrap_or(u32::MAX),
        alamat_mutanaziaa: natija.alamat_mutanaziaa,
        alamat_mafquda: natija.alamat_mafquda,
        ..TaqreerBina::default()
    };

    let hadd = i32::try_from(hajm_khana).unwrap_or(i32::MAX);
    for (miftah, khana) in tawzee.tawzee() {
        let Some(mawdi) = mawdi_shakl(&lawha, miftah.khatt, miftah.hajm_rubi, miftah.muarrif)
        else {
            // A glyph the transport allocated and the atlas does not hold means
            // the two were built from different sets, which cannot happen from
            // one shaping pass. Leaving the cell blank keeps the file valid and
            // the report honest rather than aborting a whole font over it.
            continue;
        };
        let Some((cx, cy)) = shabaka.mawdi(khana) else {
            continue;
        };

        // Everything is measured in the base glyph's pen space: the origin is the
        // pen, x grows right, y grows down, and the baseline is y = 0. The cell's
        // own placement is decided once, from the union of what goes into it, so
        // a mark can never be clipped away independently of its letter.
        let mut wadaa: Vec<(MawdiShakl, i32, i32)> = Vec::with_capacity(2);
        if !mawdi.khali() {
            wadaa.push((mawdi, i32::from(mawdi.izaha_s), -i32::from(mawdi.izaha_a)));
        }
        if let Some(alama) = miftah.alama {
            let izaha = natija.izahat.get(&miftah).copied().unwrap_or_default();
            if let Some(mawdi_alama) = mawdi_shakl(&lawha, miftah.khatt, miftah.hajm_rubi, alama)
                && !mawdi_alama.khali()
            {
                wadaa.push((
                    mawdi_alama,
                    izaha.s.saturating_add(i32::from(mawdi_alama.izaha_s)),
                    izaha.a.saturating_sub(i32::from(mawdi_alama.izaha_a)),
                ));
            }
        }

        if wadaa.is_empty() {
            // A space, a joiner, a mark the font draws nothing for. The cell keeps
            // the blank entry it was initialised with, and the advance the shaper
            // wanted for it is a cost the container cannot carry.
            taqreer.farq_taqaddum = taqreer
                .farq_taqaddum
                .saturating_add(taqaddum_texel(mawdi.taqaddum));
            continue;
        }

        let mut adna_s = i32::MAX;
        let mut adna_a = i32::MAX;
        let mut aqsa_s = i32::MIN;
        let mut aqsa_a = i32::MIN;
        for (qita, s, a) in &wadaa {
            adna_s = adna_s.min(*s);
            adna_a = adna_a.min(*a);
            aqsa_s = aqsa_s.max(s.saturating_add(i32::from(qita.ard)));
            aqsa_a = aqsa_a.max(a.saturating_add(i32::from(qita.irtifa)));
        }
        let ard = aqsa_s.saturating_sub(adna_s);
        let irtifa = aqsa_a.saturating_sub(adna_a);
        if ard > hadd || irtifa > hadd {
            return Err(KhataBio4::ShaklAkbarMinKhana {
                khatt: miftah.khatt,
                muarrif: miftah.muarrif,
                ard: u32::try_from(ard).unwrap_or(u32::MAX),
                irtifa: u32::try_from(irtifa).unwrap_or(u32::MAX),
                hajm_khana,
            }
            .into());
        }

        // Where the pen goes inside the cell: at the cell's left edge and on the
        // font's own baseline, unless the union does not fit there.
        let asas = i32::try_from(khiyarat.asas).unwrap_or(i32::MAX);
        let qalam_s = 0i32.clamp(-adna_s, hadd.saturating_sub(aqsa_s).max(-adna_s));
        let qalam_a = asas.clamp(-adna_a, hadd.saturating_sub(aqsa_a).max(-adna_a));
        let tajawuz = qalam_s.unsigned_abs().max(asas.abs_diff(qalam_a));
        if qalam_s != 0 || qalam_a != asas {
            taqreer.ashkal_maqsusa = taqreer.ashkal_maqsusa.saturating_add(1);
            taqreer.aqsa_tajawuz = taqreer.aqsa_tajawuz.max(tajawuz);
        }

        for (qita, s, a) in &wadaa {
            let Some(safha) = lawha.safahat.get(usize::from(qita.safha)) else {
                continue;
            };
            let bayt = iqta(safha, qita);
            let mawdi_s = qassir(s.saturating_add(qalam_s), hajm_khana);
            let mawdi_a = qassir(a.saturating_add(qalam_a), hajm_khana);
            sura.ulsuq(
                cx.saturating_add(mawdi_s),
                cy.saturating_add(mawdi_a),
                &bayt,
                u32::from(qita.ard),
                u32::from(qita.irtifa),
            );
        }

        let yasar = qassir(adna_s.saturating_add(qalam_s), hajm_khana);
        let yameen = qassir(aqsa_s.saturating_add(qalam_s), hajm_khana);
        if let Some(makan) = madakhil.get_mut(usize::try_from(khana).unwrap_or(usize::MAX)) {
            *makan = MadkhalKhana::jadeed(
                u8::try_from(yasar).unwrap_or(u8::MAX),
                u8::try_from(yameen).unwrap_or(u8::MAX),
            );
        }
        taqreer.farq_taqaddum = taqreer
            .farq_taqaddum
            .saturating_add(taqaddum_texel(mawdi.taqaddum).abs_diff(yameen.saturating_sub(yasar)));
    }

    let khatt = KhattBio4::jadeed(shabaka, madakhil, khiyarat.hizma)?;
    Ok(KhattMabni {
        khatt,
        sura,
        tawzee,
        nusus: natija.nusus,
        taqreer,
    })
}

/// Where one glyph's image is in the compiled atlas.
fn mawdi_shakl(lawha: &Lawha, khatt: u8, hajm_rubi: u16, muarrif: u32) -> Option<MawdiShakl> {
    lawha
        .khareeta
        .mawdi(MiftahShakl {
            khatt,
            bakat: 0,
            hajm_rubi,
            namat: NamatSafha::Taghtiya,
            muarrif,
        })
        .copied()
}

/// Adds every glyph a layout drew, at subpixel bucket zero.
///
/// Bucket zero and no other: a cell sits at a whole texel in a fixed grid, so
/// there is exactly one image of each glyph and the four buckets
/// `JamiAshkal::idif_takhtit` would collect are four copies of it. Three of them
/// would be cells taken out of a budget of a few hundred, and none of them would
/// ever be found — the transport's key has no bucket field to look one up with.
fn idif_ashkal(jami: &mut JamiAshkal, takhtit: &TakhtitNass, hajm: f32) {
    let mustaamal = if takhtit.hajm.is_finite() && takhtit.hajm > 0.0 {
        takhtit.hajm
    } else {
        hajm
    };
    for harf in &takhtit.huruf {
        jami.idif_shakl(MiftahShakl::jadeed(
            harf.khatt,
            harf.muarrif,
            mustaamal,
            NamatSafha::Taghtiya,
            0,
        ));
    }
}

/// One glyph's texels, lifted out of the page it was packed into.
fn iqta(safha: &Safha, mawdi: &MawdiShakl) -> Vec<u8> {
    let ard = usize::from(mawdi.ard);
    let irtifa = usize::from(mawdi.irtifa);
    let khatwa = usize::from(safha.ard);
    let mut qita = Vec::with_capacity(ard.saturating_mul(irtifa));
    for satr in 0..irtifa {
        let bidaya = usize::from(mawdi.a)
            .saturating_add(satr)
            .saturating_mul(khatwa)
            .saturating_add(usize::from(mawdi.s));
        let nihaya = bidaya.saturating_add(ard);
        match safha.bayt.get(bidaya..nihaya) {
            Some(masdar) => qita.extend_from_slice(masdar),
            None => qita.resize(qita.len().saturating_add(ard), 0),
        }
    }
    qita
}

/// A signed placement clamped into `[0, aqsa]`.
fn qassir(qeema: i32, aqsa: u32) -> u32 {
    if qeema <= 0 {
        return 0;
    }
    u32::try_from(qeema).unwrap_or(aqsa).min(aqsa)
}

/// An advance in pixels as whole texels, rounded to nearest.
fn taqaddum_texel(taqaddum: f32) -> u32 {
    if !taqaddum.is_finite() || taqaddum <= 0.0 {
        return 0;
    }
    let mudawwar = taqaddum.round().clamp(0.0, 65535.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a non-negative range well inside u32 on the line above"
    )]
    let texel = mudawwar as u32;
    texel
}
