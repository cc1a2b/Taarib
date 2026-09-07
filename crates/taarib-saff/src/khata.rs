//! أخطاء الصف — what can go wrong between text and glyphs, and what a person
//! can do about it.
//!
//! Two families. [`KhataSaff`] covers the layout pipeline: a caller asked for
//! something impossible, or the text carried something the pipeline refuses to
//! guess at. [`KhataKhatt`] covers font resources, and exists almost entirely to
//! serve Decision 6 — a font without Arabic tables must fail loudly at load,
//! naming the missing table, rather than silently producing isolated letterforms
//! that the user experiences as a broken patch with no diagnosis.

use std::collections::BTreeMap;

use taarib_usus::khata::{Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;

/// Failures of the layout pipeline.
#[derive(Debug, thiserror::Error)]
pub enum KhataSaff {
    /// The available width is zero or negative, so no line can ever fit.
    #[error("available width {ard} is not positive")]
    ArdGhayrSalih {
        /// The width that was asked for.
        ard: f32,
    },

    /// The pixel size is zero, negative, or beyond what the rasterizer will
    /// produce a sane atlas for.
    #[error("pixel size {hajm} is out of range")]
    HajmGhayrSalih {
        /// The size that was asked for.
        hajm: f32,
    },

    /// A style span points outside the text it styles.
    #[error("style span {id} covers {bidaya}..{nihaya} of {tul} bytes")]
    NitaqKharij {
        /// The offending span.
        id: u16,
        /// Its first byte.
        bidaya: u32,
        /// One past its last byte.
        nihaya: u32,
        /// The length of the clean text in bytes.
        tul: u32,
    },

    /// A style span begins or ends inside a UTF-8 sequence.
    ///
    /// Spans are byte ranges over the clean text, and a boundary that splits a
    /// character would split a cluster, which would put a diacritic in one style
    /// and its base letter in another.
    #[error("style span {id} boundary at byte {mawqi} is not a character boundary")]
    HaddNitaqTalif {
        /// The offending span.
        id: u16,
        /// The byte offset that is not a boundary.
        mawqi: u32,
    },

    /// Two style spans of the same kind overlap.
    #[error("style spans {awwal} and {thani} overlap")]
    NitaqMutadakhil {
        /// One span.
        awwal: u16,
        /// The other.
        thani: u16,
    },

    /// Markup could not be parsed, and guessing would silently drop a tag into
    /// the visible text or swallow real content.
    #[error("malformed markup at byte {mawqi}: {sabab}")]
    NasqTalif {
        /// Where the parse failed.
        mawqi: u32,
        /// Which rule was broken.
        sabab: SababNasq,
    },

    /// The text nests directional embeddings and isolates deeper than the
    /// Unicode Bidirectional Algorithm defines.
    ///
    /// Real text never does this; text that does is either machine-generated or
    /// hostile, and the algorithm's own answer is to stop.
    #[error("directional nesting exceeds the maximum depth of {aqsa}")]
    UmqIttijahTajawuz {
        /// The maximum explicit depth the algorithm defines.
        aqsa: u8,
    },

    /// Shaping returned no glyphs for text that is not empty.
    ///
    /// This is the failure Decision 6's validation exists to prevent, caught one
    /// stage later: a font that passed validation but still cannot shape this
    /// particular run.
    #[error("shaping produced no glyphs for {tul} bytes of {script:?} text")]
    TashkeelFashil {
        /// How many bytes went in.
        tul: u32,
        /// The script of the run.
        script: [u8; 4],
    },

    /// The font chain is empty, so there is nothing to shape with.
    #[error("the font chain is empty")]
    SilsilaFarigha,

    /// A line overflows and there is no break opportunity inside it, while the
    /// overflow policy says to fail rather than shrink or truncate.
    #[error("a run of {ard} exceeds {mutah} with no break opportunity")]
    TaadhurAlqat {
        /// The measured width of the unbreakable run.
        ard: f32,
        /// The width available.
        mutah: f32,
    },
}

/// Which markup rule was broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SababNasq {
    /// A tag was opened and never closed.
    WasmMaftuh,
    /// A closing tag matches no open tag.
    WasmMughlaqZaid,
    /// A tag's attribute could not be read as the kind of value it must be.
    QeemaTalifa,
    /// A format placeholder is malformed — an unclosed brace, a positional
    /// index with no conversion, an escape at the end of the string.
    MawdiTalif,
    /// Nesting exceeded the depth any real markup uses, which means the text is
    /// not markup.
    UmqZaid,
}

impl SababNasq {
    const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::WasmMaftuh => "وسم مفتوح لم يُغلق",
            Self::WasmMughlaqZaid => "وسم إغلاق بلا وسم مفتوح يقابله",
            Self::QeemaTalifa => "قيمة خاصية غير صالحة",
            Self::MawdiTalif => "عنصر استبدال غير مكتمل",
            Self::UmqZaid => "تداخل وسوم أعمق مما يظهر في نص حقيقي",
        }
    }

    const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::WasmMaftuh => "an opened tag was never closed",
            Self::WasmMughlaqZaid => "a closing tag matches no open tag",
            Self::QeemaTalifa => "an attribute value is not valid",
            Self::MawdiTalif => "an incomplete format placeholder",
            Self::UmqZaid => "tags nested deeper than real markup ever is",
        }
    }
}

impl std::fmt::Display for SababNasq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.wasf_injilizi())
    }
}

impl Tafsir for KhataSaff {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::SAFF
                + match self {
                    Self::ArdGhayrSalih { .. } => 0,
                    Self::HajmGhayrSalih { .. } => 1,
                    Self::NitaqKharij { .. } => 2,
                    Self::HaddNitaqTalif { .. } => 3,
                    Self::NitaqMutadakhil { .. } => 4,
                    Self::NasqTalif { .. } => 5,
                    Self::UmqIttijahTajawuz { .. } => 6,
                    Self::TashkeelFashil { .. } => 7,
                    Self::SilsilaFarigha => 8,
                    Self::TaadhurAlqat { .. } => 9,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::TaadhurAlqat { .. } => Khutura::Tanbeeh,
            Self::TashkeelFashil { .. } | Self::SilsilaFarigha => Khutura::Fadih,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::ArdGhayrSalih { .. } => {
                "العرض المتاح للنص صفر أو أقل، فلا يمكن ترتيب أي سطر داخله.".to_owned()
            },
            Self::HajmGhayrSalih { hajm } => {
                format!("حجم الخط {hajm} خارج المدى الذي يمكن رسمه.")
            },
            Self::NitaqKharij { .. } | Self::HaddNitaqTalif { .. } => {
                "أحد نطاقات التنسيق يشير خارج النص أو يقطع حرفًا في منتصفه.".to_owned()
            },
            Self::NitaqMutadakhil { .. } => {
                "نطاقا تنسيق متداخلان من النوع نفسه، ولا يمكن تحديد أيهما يسري.".to_owned()
            },
            Self::NasqTalif { sabab, .. } => {
                format!("وسوم النص غير سليمة: {}.", sabab.wasf_arabi())
            },
            Self::UmqIttijahTajawuz { .. } => {
                "تداخل اتجاهات النص أعمق مما تسمح به خوارزمية الاتجاهين.".to_owned()
            },
            Self::TashkeelFashil { .. } => {
                "لم يُنتج الخط أي حرف لهذا النص. الخط لا يدعم هذه الكتابة فعليًا رغم اجتيازه \
                 الفحص المبدئي؛ اختر خطًا آخر."
                    .to_owned()
            },
            Self::SilsilaFarigha => "لم يُحدَّد أي خط لهذا النص، فلا شيء يمكن الرسم به.".to_owned(),
            Self::TaadhurAlqat { .. } => {
                "كلمة أطول من المساحة المتاحة ولا يوجد موضع قطع داخلها.".to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::ArdGhayrSalih { .. } => {
                "The width available for text is zero or less, so no line can be laid out in it."
                    .to_owned()
            },
            Self::HajmGhayrSalih { hajm } => {
                format!("Font size {hajm} is outside the range that can be rasterized.")
            },
            Self::NitaqKharij { .. } | Self::HaddNitaqTalif { .. } => {
                "A style span points outside the text, or splits a character in half.".to_owned()
            },
            Self::NitaqMutadakhil { .. } => {
                "Two style spans of the same kind overlap, so neither can be resolved.".to_owned()
            },
            Self::NasqTalif { sabab, .. } => format!("The text's markup is malformed: {sabab}."),
            Self::UmqIttijahTajawuz { .. } => {
                "Directional nesting is deeper than the bidirectional algorithm allows.".to_owned()
            },
            Self::TashkeelFashil { .. } => {
                "The font produced no glyphs for this text. It does not really support this \
                 script despite passing the initial check; choose another font."
                    .to_owned()
            },
            Self::SilsilaFarigha => {
                "No font was given for this text, so there is nothing to draw with.".to_owned()
            },
            Self::TaadhurAlqat { .. } => {
                "A word is wider than the space available and has no break opportunity inside it."
                    .to_owned()
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::TashkeelFashil { .. } | Self::SilsilaFarigha => Khutwa::IkhtiyarKhattAakhar,
            Self::TaadhurAlqat { .. } => Khutwa::FathTaqreerTajawuz,
            Self::NasqTalif { .. }
            | Self::NitaqKharij { .. }
            | Self::HaddNitaqTalif { .. }
            | Self::NitaqMutadakhil { .. }
            | Self::UmqIttijahTajawuz { .. } => Khutwa::FathNusus,
            Self::ArdGhayrSalih { .. } | Self::HajmGhayrSalih { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::ArdGhayrSalih { ard } => {
                let _ = siyaq.insert("ard".to_owned(), QeemaSiyaq::Kasr(f64::from(*ard)));
            },
            Self::HajmGhayrSalih { hajm } => {
                let _ = siyaq.insert("hajm".to_owned(), QeemaSiyaq::Kasr(f64::from(*hajm)));
            },
            Self::NitaqKharij {
                id,
                bidaya,
                nihaya,
                tul,
            } => {
                let _ = siyaq.insert("nitaq".to_owned(), QeemaSiyaq::Raqm(i64::from(*id)));
                let _ = siyaq.insert("bidaya".to_owned(), QeemaSiyaq::Raqm(i64::from(*bidaya)));
                let _ = siyaq.insert("nihaya".to_owned(), QeemaSiyaq::Raqm(i64::from(*nihaya)));
                let _ = siyaq.insert("tul".to_owned(), QeemaSiyaq::Raqm(i64::from(*tul)));
            },
            Self::HaddNitaqTalif { id, mawqi } => {
                let _ = siyaq.insert("nitaq".to_owned(), QeemaSiyaq::Raqm(i64::from(*id)));
                let _ = siyaq.insert("mawqi".to_owned(), QeemaSiyaq::Raqm(i64::from(*mawqi)));
            },
            Self::NitaqMutadakhil { awwal, thani } => {
                let _ = siyaq.insert("awwal".to_owned(), QeemaSiyaq::Raqm(i64::from(*awwal)));
                let _ = siyaq.insert("thani".to_owned(), QeemaSiyaq::Raqm(i64::from(*thani)));
            },
            Self::NasqTalif { mawqi, sabab } => {
                let _ = siyaq.insert("mawqi".to_owned(), QeemaSiyaq::Raqm(i64::from(*mawqi)));
                let _ = siyaq.insert(
                    "sabab".to_owned(),
                    QeemaSiyaq::Nass(sabab.wasf_injilizi().to_owned()),
                );
            },
            Self::UmqIttijahTajawuz { aqsa } => {
                let _ = siyaq.insert("aqsa".to_owned(), QeemaSiyaq::Raqm(i64::from(*aqsa)));
            },
            Self::TashkeelFashil { tul, script } => {
                let _ = siyaq.insert("tul".to_owned(), QeemaSiyaq::Raqm(i64::from(*tul)));
                let _ = siyaq.insert(
                    "script".to_owned(),
                    QeemaSiyaq::Nass(String::from_utf8_lossy(script).into_owned()),
                );
            },
            Self::SilsilaFarigha => {},
            Self::TaadhurAlqat { ard, mutah } => {
                let _ = siyaq.insert("ard".to_owned(), QeemaSiyaq::Kasr(f64::from(*ard)));
                let _ = siyaq.insert("mutah".to_owned(), QeemaSiyaq::Kasr(f64::from(*mutah)));
            },
        }
        siyaq
    }
}

/// Failures of font resources.
///
/// Every variant names the specific thing that is missing, because the whole
/// point of validating a font before it is used is to replace "the patch looks
/// broken" with a sentence the person choosing the font can act on.
#[derive(Debug, thiserror::Error)]
pub enum KhataKhatt {
    /// The bytes are not a font Taarib can parse.
    #[error("not a readable font file")]
    TahleelFashil {
        /// What the parser reported, kept as context rather than as the message.
        tafsil: String,
    },

    /// The face index is beyond the number of faces in a font collection.
    #[error("face {fahras} of a collection with {adad} faces")]
    FahrasKharij {
        /// The requested index.
        fahras: u32,
        /// How many faces the collection has.
        adad: u32,
    },

    /// A table Arabic shaping cannot work without is missing.
    #[error("required table {jadwal} is missing")]
    JadwalMafqud {
        /// The four-character table tag.
        jadwal: &'static str,
    },

    /// A required Arabic OpenType feature is absent from `GSUB` or `GPOS`.
    ///
    /// This is the exact failure Decision 6 exists to catch. A font without
    /// `init`/`medi`/`fina`/`isol` does not draw ugly Arabic; it draws
    /// twenty-eight disconnected letterforms, silently.
    #[error("required Arabic feature {sifa} is missing")]
    SifaMafquda {
        /// The four-character feature tag.
        sifa: &'static str,
    },

    /// The font does not cover the Arabic characters the caller declared it
    /// would be asked for.
    #[error("{adad} declared characters are not in the font, starting at U+{awwal:04X}")]
    TaghtiyaNaqisa {
        /// How many declared characters are absent.
        adad: u32,
        /// The first one, so the message can name a real character.
        awwal: u32,
    },

    /// A variation axis was set that the font does not have.
    #[error("unknown variation axis {mihwar}")]
    MihwarMajhul {
        /// The four-character axis tag.
        mihwar: String,
    },

    /// A named instance was requested that the font does not define.
    #[error("unknown named instance {fahras}")]
    InstansMajhula {
        /// The requested instance index.
        fahras: u32,
    },

    /// The font chain holds more fonts than a glyph can name.
    ///
    /// A positioned glyph carries the index of the font it came from in a
    /// single byte, so a chain longer than 256 would contain fonts no output
    /// could ever refer to. Refusing is better than truncating: a caller that
    /// built a chain that long has a bug, and silently dropping the tail would
    /// make it surface later as missing glyphs.
    #[error("font chain has {adad} fonts; a glyph can name at most 256")]
    SilsilaTaweela {
        /// How many fonts were given.
        adad: u32,
    },
}

impl Tafsir for KhataKhatt {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::KHATT
                + match self {
                    Self::TahleelFashil { .. } => 0,
                    Self::FahrasKharij { .. } => 1,
                    Self::JadwalMafqud { .. } => 2,
                    Self::SifaMafquda { .. } => 3,
                    Self::TaghtiyaNaqisa { .. } => 4,
                    Self::MihwarMajhul { .. } => 5,
                    Self::InstansMajhula { .. } => 6,
                    Self::SilsilaTaweela { .. } => 7,
                },
        )
    }

    fn arabi(&self) -> String {
        match self {
            Self::TahleelFashil { .. } => "هذا الملف ليس خطًا يمكن قراءته، أو أنه تالف.".to_owned(),
            Self::FahrasKharij { adad, .. } => {
                format!("الخط المطلوب غير موجود داخل الملف؛ يحتوي على {adad} خط فقط.")
            },
            Self::JadwalMafqud { jadwal } => format!(
                "هذا الخط لا يحتوي جدول {jadwal}، وبدونه لا يمكن تشكيل العربية إطلاقًا. \
                 اختر خطًا عربيًا كاملًا."
            ),
            Self::SifaMafquda { sifa } => format!(
                "هذا الخط لا يحتوي خاصية {sifa} المسؤولة عن وصل الحروف العربية. سيظهر النص \
                 حروفًا منفصلة، لذلك رُفض الخط."
            ),
            Self::TaghtiyaNaqisa { adad, awwal } => {
                format!("الخط لا يغطي {adad} حرفًا من الحروف المطلوبة، أولها U+{awwal:04X}.")
            },
            Self::MihwarMajhul { mihwar } => {
                format!("الخط لا يحتوي محور التغيّر {mihwar}.")
            },
            Self::InstansMajhula { .. } => "النمط المطلوب غير معرَّف داخل هذا الخط.".to_owned(),
            Self::SilsilaTaweela { adad } => format!(
                "سلسلة الخطوط تحتوي {adad} خطًا، والحد الأقصى ٢٥٦؛ ما بعدها لا يمكن الإشارة \
                 إليه في النص المرسوم."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::TahleelFashil { .. } => {
                "This file is not a readable font, or it is corrupt.".to_owned()
            },
            Self::FahrasKharij { adad, .. } => {
                format!("The requested face is not in the file; it contains {adad} faces.")
            },
            Self::JadwalMafqud { jadwal } => format!(
                "This font has no {jadwal} table, and without it Arabic cannot be shaped at \
                 all. Choose a complete Arabic font."
            ),
            Self::SifaMafquda { sifa } => format!(
                "This font is missing the {sifa} feature that joins Arabic letters. Text would \
                 render as disconnected letterforms, so the font was rejected."
            ),
            Self::TaghtiyaNaqisa { adad, awwal } => format!(
                "The font does not cover {adad} of the required characters, the first being \
                 U+{awwal:04X}."
            ),
            Self::MihwarMajhul { mihwar } => {
                format!("The font has no {mihwar} variation axis.")
            },
            Self::InstansMajhula { .. } => {
                "The requested named style is not defined in this font.".to_owned()
            },
            Self::SilsilaTaweela { adad } => format!(
                "The font chain has {adad} fonts and the maximum is 256; anything past that \
                 could never be referred to by a drawn glyph."
            ),
        }
    }

    fn khutura(&self) -> Khutura {
        Khutura::Khatar
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::TahleelFashil { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MalafKhatt,
            },
            _ => Khutwa::IkhtiyarKhattAakhar,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TahleelFashil { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::FahrasKharij { fahras, adad } => {
                let _ = siyaq.insert("fahras".to_owned(), QeemaSiyaq::Raqm(i64::from(*fahras)));
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Raqm(i64::from(*adad)));
            },
            Self::JadwalMafqud { jadwal } => {
                let _ = siyaq.insert("jadwal".to_owned(), QeemaSiyaq::Nass((*jadwal).to_owned()));
            },
            Self::SifaMafquda { sifa } => {
                let _ = siyaq.insert("sifa".to_owned(), QeemaSiyaq::Nass((*sifa).to_owned()));
            },
            Self::TaghtiyaNaqisa { adad, awwal } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Raqm(i64::from(*adad)));
                let _ = siyaq.insert("awwal".to_owned(), QeemaSiyaq::Raqm(i64::from(*awwal)));
            },
            Self::MihwarMajhul { mihwar } => {
                let _ = siyaq.insert("mihwar".to_owned(), QeemaSiyaq::Nass(mihwar.clone()));
            },
            Self::InstansMajhula { fahras } => {
                let _ = siyaq.insert("fahras".to_owned(), QeemaSiyaq::Raqm(i64::from(*fahras)));
            },
            Self::SilsilaTaweela { adad } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Raqm(i64::from(*adad)));
            },
        }
        siyaq
    }
}

khata_min!(KhataSaff, KhataKhatt);
