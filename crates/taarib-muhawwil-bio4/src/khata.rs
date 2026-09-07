//! أخطاء محوّل بيو٤ — what stops the BIO4 adapter, and why almost all of it
//! refuses rather than degrades.
//!
//! This adapter rewrites files that *are* the game: a `.fnt` whose embedded TPL
//! declares a height the glyph grid does not produce is a font the game samples
//! past the end of, and the visible result is a row of garbage where a menu used
//! to be. There is no partial success to keep, so every container failure here
//! is [`Khutura::Khatar`] and the caller is expected to write nothing.
//!
//! The two exceptions are the transport's own limits — [`KhataBio4::KhanatNafida`]
//! and [`KhataBio4::ShaklAkbarMinKhana`]. Those are a *rung* declining: the cell
//! grid a shipped font was authored with is a fixed budget, and a glyph set that
//! does not fit one is a reason to pick a different font size or a different
//! font file, not a reason to call the patch broken. They are warnings so the
//! tier probe can descend instead of stopping.
//!
//! ## Error codes
//!
//! The adapters share [`arqam::MUHAWWIL`] (4300). Unity holds 0–9, Unreal 20–39,
//! Godot 40–62 and the script engines 70–88; `AWWAL` gives this crate 4400–4410,
//! with the gap below it left deliberately. A shipped code is permanent — users
//! paste them into bug reports years later — so the bands never overlap and are
//! never renumbered.

use std::collections::BTreeMap;

use taarib_usus::khata::{Khutura, Khutwa, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;

/// The first code this module uses within the adapters' band.
///
/// A hundred, leaving room below for the four adapters that came first. See this
/// module's header.
const AWWAL: u16 = 100;

/// Failures of the BIO4 adapter.
#[derive(Debug, thiserror::Error)]
pub enum KhataBio4 {
    /// A container ended before a field the format requires.
    #[error("{haql} needs {matlub} byte(s) at offset {mawqi} and only {tul} are present")]
    MalafQaseer {
        /// Which field ran out.
        haql: &'static str,
        /// Where the field starts.
        mawqi: u64,
        /// How many bytes the container has in total.
        tul: u64,
        /// How many were needed.
        matlub: u64,
    },

    /// The TPL block does not begin with `0x1234_5678`.
    ///
    /// Read as a little-endian word, which is how the PC port stores it inside a
    /// `.fnt`. The standalone `.tpl` files beside it store the same constant as
    /// the byte sequence `12 34 56 78`; the two are not the same bytes, and a
    /// reader that accepted either would accept a file it cannot parse.
    #[error("the TPL block starts with {wujid:#010x} and not the expected 0x12345678")]
    SihrGhayrMutabaq {
        /// The word that was there.
        wujid: u32,
    },

    /// A structural field carries a value this format cannot mean.
    #[error("{haql} is {qeema}, which {sabab}")]
    BunyaGhayrMutawaqqaa {
        /// The field.
        haql: &'static str,
        /// What it held.
        qeema: u64,
        /// Why that cannot be right.
        sabab: &'static str,
    },

    /// The TPL declares a texel format this build does not read or write.
    ///
    /// Named rather than numbered, because the answer to `GX_TF_CMPR` is
    /// different from the answer to a format that does not exist: the first is a
    /// real GameCube format nothing in `BIO4/Font` uses, and the second is a
    /// corrupt file.
    #[error("the TPL image format is {raqm} ({ism}), which this build does not handle")]
    SighatSuraGhayrMaduma {
        /// The raw enumerant.
        raqm: u32,
        /// Its GameCube name, or `unknown` when it is not one.
        ism: &'static str,
    },

    /// The TPL declares a palette format this build does not read or write.
    #[error("the TPL palette format is {raqm} ({ism}), which this build does not handle")]
    SighatLawnGhayrMaduma {
        /// The raw enumerant.
        raqm: u32,
        /// Its GameCube name, or `unknown` when it is not one.
        ism: &'static str,
    },

    /// The cell grid derived from the metrics does not reproduce the height the
    /// TPL declares.
    ///
    /// The single check that keeps this adapter honest. Every field of a BIO4
    /// font is redundant with the others — the cell size, the entry count and the
    /// declared texture height determine each other — so a file that fails this
    /// is a file whose layout is not the one this crate believes in, and writing
    /// a new one from that belief would produce a font sampled at the wrong
    /// stride.
    #[error(
        "a {hajm_khana}px grid {aamida} cell(s) wide holding {khanat} cell(s) needs \
         {irtifa_mahsub} row(s) of texels and the TPL declares {irtifa_muallan}"
    )]
    ShabakaGhayrMutasiqa {
        /// The cell pitch in texels.
        hajm_khana: u32,
        /// Cells per row.
        aamida: u32,
        /// Cells the metrics table describes, less the trailing off-atlas run.
        khanat: u32,
        /// The height the TPL image header carries.
        irtifa_muallan: u32,
        /// The height the grid needs.
        irtifa_mahsub: u32,
    },

    /// More shaped glyphs than the font's grid has cells.
    #[error("the glyph set needs {matlub} cell(s) and this font's grid holds {mutah}")]
    KhanatNafida {
        /// How many cells the set needs.
        matlub: u32,
        /// How many the grid has.
        mutah: u32,
    },

    /// A rasterized glyph is larger than one cell.
    #[error(
        "glyph {muarrif} of font {khatt} rasterized to {ard}x{irtifa} and a cell is \
         {hajm_khana}x{hajm_khana}"
    )]
    ShaklAkbarMinKhana {
        /// Index into the patch's font chain.
        khatt: u8,
        /// The glyph identifier shaping produced.
        muarrif: u32,
        /// The bitmap's width.
        ard: u32,
        /// The bitmap's height.
        irtifa: u32,
        /// The cell pitch.
        hajm_khana: u32,
    },

    /// A declared size exceeds what this build will allocate for it.
    ///
    /// Checked against the number the file declares, before a byte is reserved.
    /// A game's data file is a file somebody else produced.
    #[error("{haql} declares {qeema} and this build allows {saqf}")]
    HajmMufrit {
        /// The field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// A texel payload's length disagrees with the format and dimensions that
    /// describe it.
    #[error("a {ard}x{irtifa} {sigha} image needs {matlub} byte(s) of texels and {tul} were given")]
    HimlGhayrMutabaq {
        /// The format's short name.
        sigha: &'static str,
        /// Declared width.
        ard: u32,
        /// Declared height.
        irtifa: u32,
        /// Bytes supplied.
        tul: u64,
        /// Bytes required.
        matlub: u64,
    },

    /// The metrics a caller asked for cannot describe a font.
    #[error("{sabab}")]
    QiyasatMarfuda {
        /// What was wrong with them.
        sabab: String,
    },
}

impl Tafsir for KhataBio4 {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUHAWWIL
                + AWWAL
                + match self {
                    Self::MalafQaseer { .. } => 0,
                    Self::SihrGhayrMutabaq { .. } => 1,
                    Self::BunyaGhayrMutawaqqaa { .. } => 2,
                    Self::SighatSuraGhayrMaduma { .. } => 3,
                    Self::SighatLawnGhayrMaduma { .. } => 4,
                    Self::ShabakaGhayrMutasiqa { .. } => 5,
                    Self::KhanatNafida { .. } => 6,
                    Self::ShaklAkbarMinKhana { .. } => 7,
                    Self::HajmMufrit { .. } => 8,
                    Self::HimlGhayrMutabaq { .. } => 9,
                    Self::QiyasatMarfuda { .. } => 10,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // The grid is a budget the game's own author fixed. Exceeding it is
            // a reason to choose a smaller size or a narrower face, and the
            // caller is expected to try one — not to stop.
            Self::KhanatNafida { .. } | Self::ShaklAkbarMinKhana { .. } => Khutura::Tanbeeh,
            _ => Khutura::Khatar,
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhanatNafida { .. } | Self::ShaklAkbarMinKhana { .. } => {
                Khutwa::IkhtiyarKhattAakhar
            },
            Self::MalafQaseer { .. }
            | Self::SihrGhayrMutabaq { .. }
            | Self::BunyaGhayrMutawaqqaa { .. }
            | Self::ShabakaGhayrMutasiqa { .. }
            | Self::HajmMufrit { .. }
            | Self::HimlGhayrMutabaq { .. } => Khutwa::TahaqquqSalamatLuba,
            Self::SighatSuraGhayrMaduma { .. } | Self::SighatLawnGhayrMaduma { .. } => {
                Khutwa::TahdithTaarib
            },
            Self::QiyasatMarfuda { .. } => Khutwa::FathTashkhis,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MalafQaseer { .. } => {
                "أحد ملفات خطوط اللعبة أقصر مما تعلنه ترويسته؛ يبدو أنه تالف أو ناقص.".to_owned()
            },
            Self::SihrGhayrMutabaq { .. } => {
                "ملف خطٍّ لا يحمل العلامة التي تبدأ بها ملفات هذه الصيغة.".to_owned()
            },
            Self::BunyaGhayrMutawaqqaa { .. } => {
                "بنية ملف الخطّ لا تطابق ما تفهمه هذه النسخة من تعريب.".to_owned()
            },
            Self::SighatSuraGhayrMaduma { .. } | Self::SighatLawnGhayrMaduma { .. } => {
                "ملف الخطّ يستعمل صيغة صورة لا تدعمها هذه النسخة من تعريب.".to_owned()
            },
            Self::ShabakaGhayrMutasiqa { .. } => {
                "شبكة خانات الخطّ لا تنتج الارتفاع الذي يعلنه الملف؛ رُفض قبل الكتابة.".to_owned()
            },
            Self::KhanatNafida { .. } => {
                "أشكال العربية المطلوبة أكثر من الخانات التي يتيحها خطّ اللعبة.".to_owned()
            },
            Self::ShaklAkbarMinKhana { .. } => {
                "أحد الأشكال أكبر من خانة واحدة في شبكة خطّ اللعبة.".to_owned()
            },
            Self::HajmMufrit { .. } => {
                "ملف الخطّ يعلن حجمًا أكبر مما يقبله هذا البناء، ورُفض قبل حجز أي ذاكرة له.".to_owned()
            },
            Self::HimlGhayrMutabaq { .. } => {
                "طول بيانات الصورة لا يوافق أبعادها ولا صيغتها.".to_owned()
            },
            Self::QiyasatMarfuda { .. } => "قياسات الخطّ المطلوبة غير صالحة.".to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::MalafQaseer {
                haql,
                mawqi,
                tul,
                matlub,
            } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("mawqi", QeemaSiyaq::Hajm(*mawqi));
                daa("tul", QeemaSiyaq::Hajm(*tul));
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
            },
            Self::SihrGhayrMutabaq { wujid } => {
                daa("wujid", QeemaSiyaq::Hajm(u64::from(*wujid)));
            },
            Self::BunyaGhayrMutawaqqaa { haql, qeema, sabab } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("sabab", QeemaSiyaq::Nass((*sabab).to_owned()));
            },
            Self::SighatSuraGhayrMaduma { raqm, ism }
            | Self::SighatLawnGhayrMaduma { raqm, ism } => {
                daa("raqm", QeemaSiyaq::Hajm(u64::from(*raqm)));
                daa("ism", QeemaSiyaq::Nass((*ism).to_owned()));
            },
            Self::ShabakaGhayrMutasiqa {
                hajm_khana,
                aamida,
                khanat,
                irtifa_muallan,
                irtifa_mahsub,
            } => {
                daa("hajm_khana", QeemaSiyaq::Hajm(u64::from(*hajm_khana)));
                daa("aamida", QeemaSiyaq::Hajm(u64::from(*aamida)));
                daa("khanat", QeemaSiyaq::Hajm(u64::from(*khanat)));
                daa(
                    "irtifa_muallan",
                    QeemaSiyaq::Hajm(u64::from(*irtifa_muallan)),
                );
                daa("irtifa_mahsub", QeemaSiyaq::Hajm(u64::from(*irtifa_mahsub)));
            },
            Self::KhanatNafida { matlub, mutah } => {
                daa("matlub", QeemaSiyaq::Hajm(u64::from(*matlub)));
                daa("mutah", QeemaSiyaq::Hajm(u64::from(*mutah)));
            },
            Self::ShaklAkbarMinKhana {
                khatt,
                muarrif,
                ard,
                irtifa,
                hajm_khana,
            } => {
                daa("khatt", QeemaSiyaq::Hajm(u64::from(*khatt)));
                daa("muarrif", QeemaSiyaq::Hajm(u64::from(*muarrif)));
                daa("ard", QeemaSiyaq::Hajm(u64::from(*ard)));
                daa("irtifa", QeemaSiyaq::Hajm(u64::from(*irtifa)));
                daa("hajm_khana", QeemaSiyaq::Hajm(u64::from(*hajm_khana)));
            },
            Self::HajmMufrit { haql, qeema, saqf } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            },
            Self::HimlGhayrMutabaq {
                sigha,
                ard,
                irtifa,
                tul,
                matlub,
            } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("ard", QeemaSiyaq::Hajm(u64::from(*ard)));
                daa("irtifa", QeemaSiyaq::Hajm(u64::from(*irtifa)));
                daa("tul", QeemaSiyaq::Hajm(*tul));
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
            },
            Self::QiyasatMarfuda { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataBio4);

/// A length as a `u64`, saturating on a platform where `usize` is wider — which
/// is none this product targets, and is still not a reason to write a cast the
/// compiler cannot prove.
#[must_use]
pub fn tul_u64(tul: usize) -> u64 {
    u64::try_from(tul).unwrap_or(u64::MAX)
}
