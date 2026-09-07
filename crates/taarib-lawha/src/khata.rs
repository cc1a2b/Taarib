//! أخطاء اللوحة — what can go wrong between a glyph identifier and a texel, and
//! what a person can do about it.
//!
//! The atlas fails in two very different situations and both are represented
//! here. Offline, inside the patch compiler, a failure is a compile error the
//! translator sees before anything ships: a glyph too large for a page, a page
//! budget that the declared sizes cannot fit into. Online, inside somebody's
//! game process, a failure is a glyph that will not be drawn this frame, and the
//! sentence has to say so honestly rather than pretending the frame succeeded.
//!
//! Nothing here degrades quietly. The one temptation this module exists to
//! refuse is scaling a glyph down so it fits a page: it would produce a patch
//! whose measured widths and drawn widths disagree, which turns every overflow
//! measurement the compiler reported into a lie. A named error is cheaper than a
//! patch nobody can trust.
//!
//! ## Error codes
//!
//! This crate owns [`arqam::LAWHA`], which is 3000. 3000–3019 belong to the
//! atlas proper — rasterizing, packing, page budgets and runtime eviction — and
//! 3020 upward to the glyph transport in [`crate::naql`], which is a separate
//! subsystem that happens to live beside the atlas because it addresses the
//! atlas's images. A permanent code is permanent and users paste them into bug
//! reports, so the two are not interleaved: a code allocated to one of them is
//! never reused by the other.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The first code the glyph transport uses within this crate's band.
///
/// Twenty, leaving the atlas proper room to grow to twenty variants before the
/// two subsystems would collide. See this module's header.
const AWWAL_NAQL: u16 = 20;

/// Failures of the glyph atlas.
#[derive(Debug, thiserror::Error)]
pub enum KhataLawha {
    /// One glyph's bitmap, with its padding, is larger than a whole page.
    ///
    /// Never resolved by scaling the glyph down. A patch is compiled against
    /// measurements taken from real shaped advances, and a glyph drawn smaller
    /// than it was measured is a line that no longer fits the box the overflow
    /// report cleared it for.
    #[error(
        "glyph {muarrif} of font {khatt} needs {ard}x{irtifa} px, past the {hadd_ard}x\
         {hadd_irtifa} page limit"
    )]
    ShaklAkbarMinSafha {
        /// Index into the font chain the glyph came from.
        khatt: u8,
        /// The glyph identifier, as shaping produced it.
        muarrif: u32,
        /// The width the glyph needs, padding included.
        ard: u16,
        /// The height it needs, padding included.
        irtifa: u16,
        /// The widest page this atlas may create.
        hadd_ard: u16,
        /// The tallest page this atlas may create.
        hadd_irtifa: u16,
    },

    /// A page dimension, or a rectangle asked for inside a page, cannot be used.
    ///
    /// Either extent is zero, or the two multiply to an area past what the shelf
    /// allocator counts in, or the rectangle does not lie inside the page it
    /// names, or the page named does not exist. All of them are caller mistakes
    /// rather than capacity problems, and all of them are refused before a page
    /// is created or a byte is written rather than after the allocator has been
    /// handed numbers it will assert on.
    #[error("page or rectangle dimensions {ard}x{irtifa} are not usable")]
    AbaadSafhaGhayrSaliha {
        /// The width that was asked for.
        ard: u16,
        /// The height that was asked for.
        irtifa: u16,
    },

    /// Every page allowed by the budget is open and the glyph still does not
    /// fit anywhere.
    ///
    /// Offline this means the declared glyph set does not fit the page policy
    /// the patch chose. It is a real answer, and the numbers in it are the two
    /// the translator can act on: how many pages exist and how many are allowed.
    #[error("the atlas holds {adad} pages, the budget is {hadd}, and the glyph does not fit")]
    SafahatNafidat {
        /// How many pages are open.
        adad: u16,
        /// How many pages the budget allows.
        hadd: u16,
    },

    /// The atlas is full and every rectangle left in it is pinned by the frame
    /// being drawn.
    ///
    /// This is the failure that keeps the atlas honest under pressure. Evicting
    /// a rectangle the current frame has already sampled would replace one
    /// letter with another for one frame — a defect that shows up as a single
    /// wrong glyph, intermittently, and that no bug report can localise. So the
    /// evictor refuses, and the frame is told that the text it is trying to draw
    /// at once is larger than the atlas it was given.
    #[error("{mathbut} glyphs are pinned by the current frame and {matlub_bayt} bytes are needed")]
    LaShayLilIkhla {
        /// How many glyphs the frame has already referenced.
        mathbut: u32,
        /// The area, in texels, the new glyph needs.
        matlub_bayt: u32,
    },

    /// The byte budget is smaller than a single page.
    ///
    /// An atlas with no page cannot hold a glyph, so this is refused at
    /// construction instead of surfacing later as an atlas that misses
    /// everything.
    #[error("a budget of {mizaniya} bytes cannot hold one {hajm_safha}-byte page")]
    MizaniyaAsghurMinSafha {
        /// The budget, in bytes.
        mizaniya: u64,
        /// What one page of the configured size costs, in bytes.
        hajm_safha: u64,
    },

    /// A glyph that shaping produced could not be turned into pixels.
    ///
    /// The whole key travels with the failure, because the interesting question
    /// is never "which glyph" on its own — it is which glyph, from which font in
    /// the chain, at which size.
    #[error("glyph {muarrif} of font {khatt} could not be rasterized at {hajm_rubi} quarter-px")]
    RasmFashil {
        /// Index into the font chain.
        khatt: u8,
        /// The glyph identifier.
        muarrif: u32,
        /// The pixel size in quarter-pixels, exactly as the key carries it.
        hajm_rubi: u16,
    },

    /// A glyph was asked for from a font index the chain does not have.
    ///
    /// The atlas and the font chain disagree about how many fonts there are,
    /// which means one of them was built from a different patch than the other.
    /// Drawing anything at all from here would draw the wrong letter.
    #[error("font index {khatt} was asked for, and the chain holds {adad} fonts")]
    KhattKharijSilsila {
        /// The index that was asked for.
        khatt: u8,
        /// How many fonts the chain actually holds.
        adad: u32,
    },

    /// The distance-field source resolution is not a usable multiplier.
    ///
    /// It must be at least one — a source coarser than the cell would throw away
    /// the very detail supersampling exists to keep — and the cell multiplied by
    /// it must stay inside what the rasterizer will draw.
    #[error("source resolution {daqqa} is not usable for a {hajm_khalyia} px cell")]
    DaqqaGhayrSaliha {
        /// The multiplier that was asked for.
        daqqa: f32,
        /// The target cell size, in pixels.
        hajm_khalyia: u16,
    },

    /// A coverage bitmap was written into a distance-field page, or the reverse.
    ///
    /// The two put entirely different meanings in the same byte: in one, 128 is
    /// half ink; in the other, 128 is the outline itself. A page holding both is
    /// wrong in a way no shader can detect and no screenshot shows clearly — the
    /// letters simply come out the wrong weight — so the mismatch is refused at
    /// the moment of the blit.
    #[error("page mode {safha} and glyph mode {surah} disagree")]
    NamatMukhtalif {
        /// The page's mode, as the byte the patch container stores.
        safha: u8,
        /// The bitmap's mode, in the same encoding.
        surah: u8,
    },

    /// The glyph transport could not be generated.
    ///
    /// The last rung of the ladder for an engine whose only glyph-naming
    /// mechanism is a character code. When this fires, no path remains through
    /// that engine and the text stays in its own language, so the reason is
    /// carried as a sentence naming the glyph, the line or the page that
    /// stopped it rather than as a category.
    #[error("the glyph transport could not be generated: {sabab}")]
    NaqlMarfud {
        /// Why, in one sentence.
        sabab: String,
    },

    /// More glyphs were needed than the transport can address.
    ///
    /// The private-use area is finite. A script with more distinct shaped
    /// glyphs than it holds cannot be transported, and that is a refusal rather
    /// than a silent truncation — a truncated glyph table draws the wrong
    /// letters, which reads as a corrupt font rather than as a limit reached.
    #[error("the transport needs {matlub} glyph slots and holds {mutah}")]
    NaqlMumtali {
        /// How many distinct glyphs the patch needs.
        matlub: u64,
        /// How many the private-use area provides.
        mutah: u64,
    },

    /// The vertical metrics a transport was handed cannot be used.
    ///
    /// A bitmap glyph table carries an ascent, a descent and a line height as
    /// whole numbers, and derives one of them from the other two. A set that
    /// does not agree with itself — a line height below ascent plus descent, a
    /// negative descent, a value that is not finite — would draw every line of
    /// the game's text overlapping the one above it, which reads as a rendering
    /// bug in the game rather than as a bad number in a patch.
    #[error("the transport's vertical metrics could not be used: {sabab}")]
    KhattMarfud {
        /// Why, in one sentence.
        sabab: String,
    },

    /// A declared size exceeds what this build will allocate for it.
    ///
    /// Refused by the declared number, before any memory is reserved for it, so
    /// that a patch a stranger produced cannot make this crate allocate in
    /// proportion to a figure it wrote down.
    #[error("{haql} declares {qeema}, above the ceiling of {saqf}")]
    HajmMufrit {
        /// Which field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// A sink the caller opened refused the bytes written into it.
    ///
    /// This crate opens nothing: the path travels with the failure only so the
    /// message can name the file the caller was writing. See the transport's
    /// invariant 5.
    #[error("{masar} could not be written")]
    KhataMalaf {
        /// The path the caller named, which was never opened here.
        masar: PathBuf,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },
}

impl Tafsir for KhataLawha {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::LAWHA
                + match self {
                    Self::ShaklAkbarMinSafha { .. } => 0,
                    Self::AbaadSafhaGhayrSaliha { .. } => 1,
                    Self::SafahatNafidat { .. } => 2,
                    Self::LaShayLilIkhla { .. } => 3,
                    Self::MizaniyaAsghurMinSafha { .. } => 4,
                    Self::RasmFashil { .. } => 5,
                    Self::KhattKharijSilsila { .. } => 6,
                    Self::DaqqaGhayrSaliha { .. } => 7,
                    Self::NamatMukhtalif { .. } => 8,
                    Self::NaqlMarfud { .. } => AWWAL_NAQL,
                    Self::NaqlMumtali { .. } => AWWAL_NAQL + 1,
                    Self::KhattMarfud { .. } => AWWAL_NAQL + 2,
                    Self::HajmMufrit { .. } => AWWAL_NAQL + 3,
                    Self::KhataMalaf { .. } => AWWAL_NAQL + 4,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Both of these mean a glyph the layout already committed to cannot
            // be drawn at all, which leaves a hole in text the player reads.
            Self::RasmFashil { .. } | Self::KhattKharijSilsila { .. } => Khutura::Fadih,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::ShaklAkbarMinSafha {
                ard,
                irtifa,
                hadd_ard,
                hadd_irtifa,
                ..
            } => format!(
                "أحد الأشكال أكبر من صفحة اللوحة كاملة: يحتاج {ard}×{irtifa} بكسل بينما أقصى \
                 صفحة {hadd_ard}×{hadd_irtifa}. لن تُصغَّر الحروف لتدخل، لأن ذلك يجعل ما يُرسم \
                 مخالفًا لما قِيس عند الترجمة؛ اخفض حجم الخط أو ارفع حد الصفحة."
            ),
            Self::AbaadSafhaGhayrSaliha { ard, irtifa } => format!(
                "أبعاد {ard}×{irtifa} لا تصلح صفحةً ولا مستطيلًا داخلها: لا بد أن يكون البعدان \
                 أكبر من صفر وأن تبقى مساحتهما ضمن ما يعدّه الموزّع."
            ),
            Self::SafahatNafidat { adad, hadd } => format!(
                "امتلأت اللوحة: {adad} صفحة، والحد المسموح به {hadd}، ولم يبق موضع لهذا الشكل. \
                 ارفع حد الصفحات أو قلّل عدد الأحجام التي يُرسم بها النص."
            ),
            Self::LaShayLilIkhla { mathbut, .. } => format!(
                "لا يمكن إخلاء موضع في اللوحة: {mathbut} شكلًا مثبتًا للإطار الجاري، والمثبَّت لا \
                 يُزاح ما دام الإطار يرسمه. النص المعروض في مشهد واحد أكبر مما تتسع له اللوحة."
            ),
            Self::MizaniyaAsghurMinSafha {
                mizaniya,
                hajm_safha,
            } => format!(
                "ميزانية اللوحة {mizaniya} بايت، وهي أقل من صفحة واحدة تكلّف {hajm_safha} بايت. \
                 ارفع الميزانية أو اختر مقاس صفحة أصغر."
            ),
            Self::RasmFashil { khatt, muarrif, .. } => format!(
                "تعذّر رسم الشكل رقم {muarrif} من الخط رقم {khatt}. النص يطلبه فعلًا، لكن صورته \
                 لا تُستخرج من هذا الخط."
            ),
            Self::KhattKharijSilsila { khatt, adad } => format!(
                "طُلب شكل من الخط رقم {khatt}، وسلسلة الخطوط تحتوي {adad} خطًا فقط. اللوحة \
                 والسلسلة لا تصفان الرقعة نفسها."
            ),
            Self::DaqqaGhayrSaliha {
                daqqa,
                hajm_khalyia,
            } => format!(
                "دقة المصدر {daqqa} لا تصلح لخلية من {hajm_khalyia} بكسل: لا بد أن تكون واحدًا \
                 فأكثر، وأن يبقى حاصل ضربها في مقاس الخلية ضمن ما يمكن رسمه."
            ),
            Self::NamatMukhtalif { .. } => "صورة الحرف وصفحة اللوحة لا تحملان النمط نفسه: \
                 إحداهما تغطية والأخرى حقل مسافات، والبايت نفسه يعني في كل منهما شيئًا آخر."
                .to_owned(),
            Self::NaqlMarfud { .. } => {
                "تعذّر توليد جدول أشكال النقل. النقل آخر مسار للمحرّكات التي لا تسمّي الشكل إلا \
                 برمز محرف، وحين يمتنع يبقى النص بلغته الأصلية."
                    .to_owned()
            },
            Self::NaqlMumtali { matlub, mutah } => format!(
                "النصوص تحتاج {matlub} خانة شكل، ولا يتّسع جدول النقل إلا لـ{mutah}. رُفض \
                 التوليد بدل قصّ الجدول، لأن الجدول المقصوص يرسم حروفًا خاطئة تُقرأ كأن الخطّ \
                 تالف لا كأن حدًّا بُلغ."
            ),
            Self::KhattMarfud { .. } => {
                "المقاييس الرأسية المعطاة لجدول النقل لا تصلح: الصعود والهبوط وارتفاع السطر \
                 تُكتب أعدادًا صحيحة، ولا بد أن يتّسق بعضها مع بعض وإلا تراكب كل سطر على ما \
                 فوقه، وهو خلل يبدو كعيب في رسم اللعبة لا كرقم خاطئ في الرقعة."
                    .to_owned()
            },
            Self::HajmMufrit { .. } => {
                "أحد المدخلات يعلن حجمًا أكبر بكثير مما تحتاجه أي رقعة حقيقية، ورُفض قبل حجز أي \
                 ذاكرة له."
                    .to_owned()
            },
            Self::KhataMalaf { .. } => {
                "تعذّرت كتابة المورد المولَّد. اللوحة لا تفتح ملفًّا بنفسها؛ المصرف يأتي من \
                 المستدعي، والمسار هنا اسمٌ للملف الذي كان يُكتب لا ملفٌّ فُتح هنا."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::ShaklAkbarMinSafha {
                ard,
                irtifa,
                hadd_ard,
                hadd_irtifa,
                ..
            } => format!(
                "One glyph is larger than an entire atlas page: it needs {ard}x{irtifa} px and \
                 the largest page allowed is {hadd_ard}x{hadd_irtifa}. Glyphs are never scaled \
                 down to fit, because that would draw text at a size the translator never \
                 measured; lower the font size or raise the page limit."
            ),
            Self::AbaadSafhaGhayrSaliha { ard, irtifa } => format!(
                "{ard}x{irtifa} is usable neither as a page nor as a rectangle inside one: both \
                 extents must be greater than zero and their area must stay inside what the \
                 allocator counts."
            ),
            Self::SafahatNafidat { adad, hadd } => format!(
                "The atlas is full: {adad} pages, a budget of {hadd}, and no room left for this \
                 glyph. Raise the page budget or reduce the number of sizes the text is drawn at."
            ),
            Self::LaShayLilIkhla { mathbut, .. } => format!(
                "Nothing in the atlas can be evicted: {mathbut} glyphs are pinned by the frame \
                 being drawn, and a pinned glyph is never moved while the frame still samples \
                 it. The text on screen at one moment is larger than the atlas it was given."
            ),
            Self::MizaniyaAsghurMinSafha {
                mizaniya,
                hajm_safha,
            } => format!(
                "The atlas budget is {mizaniya} bytes, less than the {hajm_safha} bytes one page \
                 costs. Raise the budget or choose a smaller page size."
            ),
            Self::RasmFashil { khatt, muarrif, .. } => format!(
                "Glyph {muarrif} of font {khatt} could not be rasterized. The text really does \
                 ask for it, but no image can be drawn for it out of this font."
            ),
            Self::KhattKharijSilsila { khatt, adad } => format!(
                "A glyph was asked for from font index {khatt} and the chain holds {adad} fonts. \
                 The atlas and the chain do not describe the same patch."
            ),
            Self::DaqqaGhayrSaliha {
                daqqa,
                hajm_khalyia,
            } => format!(
                "A source resolution of {daqqa} is not usable for a {hajm_khalyia} px cell: it \
                 must be at least one, and the cell multiplied by it must stay inside what the \
                 rasterizer will draw."
            ),
            Self::NamatMukhtalif { .. } => {
                "The glyph bitmap and the atlas page do not carry the same mode: one is coverage \
                 and the other is a distance field, and the same byte means a different thing in \
                 each."
                    .to_owned()
            },
            Self::NaqlMarfud { sabab } => format!(
                "The glyph transport could not be generated, and it is the last path available \
                 to an engine that can name a glyph only by a character code. {sabab}"
            ),
            Self::NaqlMumtali { matlub, mutah } => format!(
                "The text needs {matlub} distinct glyph slots and the transport holds {mutah}. \
                 Generation was refused rather than truncated, because a truncated glyph table \
                 does not draw fewer letters — it draws the wrong ones."
            ),
            Self::KhattMarfud { sabab } => format!(
                "The vertical metrics handed to the glyph transport cannot be used, so no glyph \
                 table was generated. The ascent, the descent and the line height are written as \
                 whole numbers and have to agree with each other, or every line overlaps the one \
                 above it. {sabab}"
            ),
            Self::HajmMufrit { haql, qeema, saqf } => format!(
                "{haql} declares {qeema}, far above the {saqf} any real patch needs; it was \
                 refused before any memory was reserved for it."
            ),
            Self::KhataMalaf { masar, .. } => format!(
                "{} could not be written. The atlas opens nothing itself — the sink comes from \
                 the caller — and this path only names the file that was being written.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // A font that cannot draw a glyph its own tables produced, or a
            // chain that does not match the atlas, are both answered by
            // choosing a font — nothing in Diagnostics repairs either. The
            // transport's vertical metrics come out of the same font, and a
            // font whose ascent is not a usable number is answered the same way.
            Self::RasmFashil { .. }
            | Self::KhattKharijSilsila { .. }
            | Self::KhattMarfud { .. } => Khutwa::IkhtiyarKhattAakhar,
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladRuqaa),
            _ => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::ShaklAkbarMinSafha {
                khatt,
                muarrif,
                ard,
                irtifa,
                hadd_ard,
                hadd_irtifa,
            } => {
                let _ = siyaq.insert("khatt".to_owned(), QeemaSiyaq::Raqm(i64::from(*khatt)));
                let _ = siyaq.insert("muarrif".to_owned(), QeemaSiyaq::Raqm(i64::from(*muarrif)));
                let _ = siyaq.insert("ard".to_owned(), QeemaSiyaq::Raqm(i64::from(*ard)));
                let _ = siyaq.insert("irtifa".to_owned(), QeemaSiyaq::Raqm(i64::from(*irtifa)));
                let _ = siyaq.insert(
                    "hadd_ard".to_owned(),
                    QeemaSiyaq::Raqm(i64::from(*hadd_ard)),
                );
                let _ = siyaq.insert(
                    "hadd_irtifa".to_owned(),
                    QeemaSiyaq::Raqm(i64::from(*hadd_irtifa)),
                );
            },
            Self::AbaadSafhaGhayrSaliha { ard, irtifa } => {
                let _ = siyaq.insert("ard".to_owned(), QeemaSiyaq::Raqm(i64::from(*ard)));
                let _ = siyaq.insert("irtifa".to_owned(), QeemaSiyaq::Raqm(i64::from(*irtifa)));
            },
            Self::SafahatNafidat { adad, hadd } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Raqm(i64::from(*adad)));
                let _ = siyaq.insert("hadd".to_owned(), QeemaSiyaq::Raqm(i64::from(*hadd)));
            },
            Self::LaShayLilIkhla {
                mathbut,
                matlub_bayt,
            } => {
                let _ = siyaq.insert("mathbut".to_owned(), QeemaSiyaq::Raqm(i64::from(*mathbut)));
                let matlub = QeemaSiyaq::Hajm(u64::from(*matlub_bayt));
                let _ = siyaq.insert("matlub".to_owned(), matlub);
            },
            Self::MizaniyaAsghurMinSafha {
                mizaniya,
                hajm_safha,
            } => {
                let _ = siyaq.insert("mizaniya".to_owned(), QeemaSiyaq::Hajm(*mizaniya));
                let _ = siyaq.insert("hajm_safha".to_owned(), QeemaSiyaq::Hajm(*hajm_safha));
            },
            Self::RasmFashil {
                khatt,
                muarrif,
                hajm_rubi,
            } => {
                let _ = siyaq.insert("khatt".to_owned(), QeemaSiyaq::Raqm(i64::from(*khatt)));
                let _ = siyaq.insert("muarrif".to_owned(), QeemaSiyaq::Raqm(i64::from(*muarrif)));
                let _ = siyaq.insert(
                    "hajm".to_owned(),
                    QeemaSiyaq::Kasr(f64::from(*hajm_rubi) / 4.0),
                );
            },
            Self::KhattKharijSilsila { khatt, adad } => {
                let _ = siyaq.insert("khatt".to_owned(), QeemaSiyaq::Raqm(i64::from(*khatt)));
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Raqm(i64::from(*adad)));
            },
            Self::DaqqaGhayrSaliha {
                daqqa,
                hajm_khalyia,
            } => {
                let _ = siyaq.insert("daqqa".to_owned(), QeemaSiyaq::Kasr(f64::from(*daqqa)));
                let _ = siyaq.insert(
                    "hajm_khalyia".to_owned(),
                    QeemaSiyaq::Raqm(i64::from(*hajm_khalyia)),
                );
            },
            Self::NamatMukhtalif { safha, surah } => {
                let _ = siyaq.insert("safha".to_owned(), QeemaSiyaq::Raqm(i64::from(*safha)));
                let _ = siyaq.insert("surah".to_owned(), QeemaSiyaq::Raqm(i64::from(*surah)));
            },
            Self::NaqlMarfud { sabab } | Self::KhattMarfud { sabab } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::NaqlMumtali { matlub, mutah } => {
                let _ = siyaq.insert("matlub".to_owned(), QeemaSiyaq::Hajm(*matlub));
                let _ = siyaq.insert("mutah".to_owned(), QeemaSiyaq::Hajm(*mutah));
            },
            Self::HajmMufrit { haql, qeema, saqf } => {
                let _ = siyaq.insert("haql".to_owned(), QeemaSiyaq::Nass((*haql).to_owned()));
                let _ = siyaq.insert("qeema".to_owned(), QeemaSiyaq::Hajm(*qeema));
                let _ = siyaq.insert("saqf".to_owned(), QeemaSiyaq::Hajm(*saqf));
            },
            Self::KhataMalaf { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                siyaq.extend(siyaq_io(sabab));
            },
        }
        siyaq
    }
}

khata_min!(KhataLawha);
