//! أخطاء الرقعة — what a `.ruqaa` can be wrong about, and what a person can do
//! about it.
//!
//! Every variant here is a *refusal*, and every refusal names the field that
//! caused it. That is not politeness: a `.ruqaa` arrives over a network from a
//! stranger, and the difference between "this patch is corrupt" and "section 3
//! declares an uncompressed size of 1099511627776 bytes, and the ceiling is
//! 536870912" is the difference between a user who reinstalls forever and a
//! maintainer who can see an attack in a log.
//!
//! Three severities are used deliberately:
//!
//! * [`Khutura::Khatar`] for structural failures — a truncated file, a bad
//!   offset, an unknown section kind. Something is broken.
//! * [`Khutura::Fadih`] for trust failures — a content hash that does not
//!   match, a signature that does not verify, a patch that carries no
//!   signature at all. Something may be hostile, and Decision 8 says the client
//!   refuses it rather than asking the user to decide.
//! * [`Khutura::Tanbeeh`] is never used here. There is no partial success in
//!   reading a container: either every offset checked out and the hash matched,
//!   or nothing in the body is trustworthy.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Everything that can go wrong reading or writing a `.ruqaa`.
#[derive(Debug, thiserror::Error)]
pub enum KhataRuqaa {
    /// The file is shorter than the framing it must contain.
    #[error("the file is {tul} bytes; {matlub} are needed for {haql}")]
    MalafQaseer {
        /// What the reader was trying to reach when it ran out of file.
        haql: &'static str,
        /// The length actually available.
        tul: u64,
        /// The length required.
        matlub: u64,
    },

    /// The first four bytes are not `TRQ1`.
    #[error("magic is {wujid:?}, not \"TRQ1\"")]
    SihrGhayrMutabaq {
        /// The four bytes that were found.
        wujid: [u8; 4],
    },

    /// The container declares a format version this build does not implement.
    ///
    /// Checked before any other field is interpreted, because every offset in
    /// the format is only meaningful under a version whose layout is known. A
    /// best-effort parse of an unknown version is a parse of bytes whose meaning
    /// was guessed.
    #[error("format version {wujid}; this build reads version {madum}")]
    IsdarGhayrMadum {
        /// The version the container declares.
        wujid: u16,
        /// The version this build implements.
        madum: u16,
    },

    /// The header sets flag bits this version does not define.
    ///
    /// Reserved bits are required to be zero, so a container that sets one is
    /// either from a future format or was edited by something that did not know
    /// what it was editing. Both are refusals.
    #[error("header flags {alam:#06x} set bits reserved in this version")]
    AlamMajhula {
        /// The whole flag word, so the offending bits are visible.
        alam: u16,
    },

    /// The section count is zero or beyond the eight kinds the format defines.
    #[error("section count {adad} is outside 1..={aqsa}")]
    AdadAqsamGhayrSalih {
        /// The declared count.
        adad: u32,
        /// The most sections the format can hold, one per kind.
        aqsa: u32,
    },

    /// `total_size` disagrees with the length of the bytes actually present.
    ///
    /// Strict equality, in both directions. A file longer than `total_size`
    /// carries trailing bytes nothing accounts for and nothing hashes, which is
    /// where a second payload hides.
    #[error("header declares {muallan} total bytes; {fili} are present")]
    HajmKulliGhayrMutabaq {
        /// What the header says.
        muallan: u64,
        /// What is there.
        fili: u64,
    },

    /// A section's offset or length runs past the end of the container.
    ///
    /// Carries the field name because a section table has four numeric fields
    /// and knowing which one lied is most of the diagnosis.
    #[error("section {naw}: {haql} is {qeema}, past the limit of {hadd}")]
    QismKharij {
        /// The section kind, as the table declares it.
        naw: u32,
        /// Which field of the entry is out of range.
        haql: &'static str,
        /// The value that was rejected.
        qeema: u64,
        /// The largest value that would have been accepted.
        hadd: u64,
    },

    /// A section does not begin on a sixteen-byte boundary.
    ///
    /// Alignment is not decoration. The POD tables are overlaid directly onto
    /// mapped pages in four other languages, and an overlay onto an unaligned
    /// address is undefined behaviour in C# and a hard fault on the aarch64
    /// targets this product ships to.
    #[error("section {naw} begins at {izaha}, which is not a multiple of 16")]
    IzahaGhayrMuhadhah {
        /// The section kind.
        naw: u32,
        /// The offending offset.
        izaha: u64,
    },

    /// Two sections claim overlapping bytes.
    ///
    /// Overlap is an aliasing attack on the content hash: one region of bytes
    /// read as two different structures means a value can be authenticated in
    /// one interpretation and used in another.
    #[error("sections {awwal} and {thani} overlap")]
    AqsamMutadakhila {
        /// The earlier section's kind.
        awwal: u32,
        /// The later section's kind.
        thani: u32,
    },

    /// The section table names a kind outside the eight the format defines.
    #[error("unknown section kind {naw}")]
    NawQismMajhul {
        /// The number that was found.
        naw: u32,
    },

    /// The section table names the same kind twice.
    ///
    /// One section per kind. Two would leave every reader free to pick a
    /// different one, which is a format that means two things at once.
    #[error("section kind {naw} appears more than once")]
    QismMukarrar {
        /// The duplicated kind.
        naw: u32,
    },

    /// A section the format requires is not in the table.
    #[error("required section {ism} is missing")]
    QismMafqud {
        /// The missing kind's name.
        ism: &'static str,
    },

    /// A section declares a compression method this version does not define.
    #[error("section {naw} declares compression {daght}")]
    DaghtMajhul {
        /// The section kind.
        naw: u32,
        /// The compression discriminant found.
        daght: u32,
    },

    /// A section declares an uncompressed size beyond the documented ceiling.
    ///
    /// This is the zstd bomb, refused before a single byte is allocated: the
    /// declared size is read from the section table, compared against the
    /// ceiling, and rejected — no buffer is reserved, no frame is opened.
    #[error("section {naw} declares {muallan} uncompressed bytes; the ceiling is {saqf}")]
    HajmKhaamMufrit {
        /// The section kind.
        naw: u32,
        /// The size the section declares.
        muallan: u64,
        /// The ceiling it exceeded.
        saqf: u64,
    },

    /// Every section is under the per-section ceiling, but together they are not.
    ///
    /// Seven sections each one byte under the per-section ceiling would be
    /// several gigabytes, which is the same attack spread across the table.
    #[error("sections declare {majmu} uncompressed bytes in total; the ceiling is {saqf}")]
    MajmuKhaamMufrit {
        /// The sum of every section's declared uncompressed size.
        majmu: u64,
        /// The aggregate ceiling.
        saqf: u64,
    },

    /// zstd refused the frame.
    #[error("section {naw} could not be decompressed")]
    FakkFashil {
        /// The section kind.
        naw: u32,
        /// What the decoder reported, kept as context rather than as the message.
        tafsil: String,
    },

    /// A section decompressed to a length other than the one it declared.
    ///
    /// The declared size is what the ceiling was checked against, so a frame
    /// that expands past it must be refused even though the ceiling check
    /// already passed. Checking only the declaration would make the ceiling a
    /// suggestion.
    #[error("section {naw} declared {muallan} uncompressed bytes and produced {fili}")]
    HajmKhaamGhayrMutabaq {
        /// The section kind.
        naw: u32,
        /// The declared size.
        muallan: u64,
        /// The size the frame actually produced.
        fili: u64,
    },

    /// The BLAKE3 hash of the body does not match the one in the header.
    #[error("content hash is {mahsuba}; the header declares {muallana}")]
    BasmaGhayrMutabaqa {
        /// The hash the header carries, in hexadecimal.
        muallana: String,
        /// The hash the bytes actually produce.
        mahsuba: String,
    },

    /// The signature block is present but is not the last section.
    ///
    /// The block's position is load-bearing: a verifier must be able to find it
    /// at a known offset from the end without decompressing anything, so that a
    /// streaming download is rejected before its last block is written and so
    /// that safety verification never expands a package it does not yet trust.
    #[error("the signature block is not the last section")]
    TawqeeLaysAkhiran,

    /// The signature block does not have the shape the format defines.
    #[error("the signature block is malformed: {haql}")]
    KutlatTawqeeTalifa {
        /// Which field of the block is wrong.
        haql: &'static str,
    },

    /// The container carries a signature block that has never been signed.
    ///
    /// Legitimate between the compiler writing a package and `taarib-khatm`
    /// sealing it, and never legitimate for anything a client installs
    /// (Decision 8).
    #[error("the patch carries no signature")]
    GhayrMuwaqqaa,

    /// The signature does not verify against the signer's public key.
    #[error("the signature does not verify under key {miftah}")]
    TawqeeGhayrSalih {
        /// The public key it was checked against, in hexadecimal.
        miftah: String,
    },

    /// A table's own header is internally inconsistent — an array that starts
    /// past the end of its section, or a count that overflows when multiplied by
    /// the record size.
    #[error("section {naw}: {haql} is {qeema}, past the limit of {hadd}")]
    JadwalTalif {
        /// The section kind.
        naw: u32,
        /// Which field of the table header is wrong.
        haql: &'static str,
        /// The value that was rejected.
        qeema: u64,
        /// The largest value that would have been accepted.
        hadd: u64,
    },

    /// A table declares a record size other than the one this format froze.
    #[error("section {naw}: {haql} is {wujid} bytes; the format fixes it at {muntazar}")]
    SijillGhayrMutabaq {
        /// The section kind.
        naw: u32,
        /// Which record size is wrong.
        haql: &'static str,
        /// The size the container declares.
        wujid: u32,
        /// The size the format defines.
        muntazar: u32,
    },

    /// Records could not be read in place because they are not on a boundary
    /// their type may be read from.
    ///
    /// Two different causes reach here, and the message names both because the
    /// remedy differs. Either the offset inside the section is wrong — a
    /// container written by something that did not respect the layout — or the
    /// container's own bytes are not aligned, which is what happens when a
    /// caller reads a patch into a `Vec<u8>` and hands that over. The second is
    /// far more likely and is the caller's to fix: map the file, or use
    /// `BaytMuhadhah`.
    #[error(
        "section {naw}: {haql} at offset {izaha} is not on a boundary its records can be read \
         from; either the offset is wrong or the container's bytes are not 16-byte aligned"
    )]
    MuhadhahaGhayrSaliha {
        /// The section kind.
        naw: u32,
        /// Which array could not be read.
        haql: &'static str,
        /// The offending offset, relative to the start of the section.
        izaha: u64,
    },

    /// A record refers to an element that does not exist.
    #[error("{haql} index {fahras} of {adad}")]
    FahrasKharij {
        /// What is being indexed — a string, a line, a glyph, a page.
        haql: &'static str,
        /// The index that was asked for.
        fahras: u32,
        /// How many elements there are.
        adad: u32,
    },

    /// A string in the text blob is not valid UTF-8.
    ///
    /// Refused rather than replaced. A lossy conversion would put replacement
    /// characters into a game's dialogue and call it a translation.
    #[error("string {fahras} is not valid UTF-8 at byte {mawqi}")]
    NassGhayrSalih {
        /// Which record's text is bad.
        fahras: u32,
        /// The first byte that is not valid, relative to the string's start.
        mawqi: u32,
    },

    /// The metadata section is not the JSON record the format expects.
    #[error("the metadata section could not be read")]
    BayanTalif {
        /// What the JSON reader reported.
        tafsil: String,
    },

    /// An atlas is present but the header declares neither coverage nor SDF.
    ///
    /// Without a declared mode the pages are bytes of unknown meaning, and an
    /// adapter that guessed would produce a different result from the one the
    /// compiler measured — which would make every overflow report in the patch
    /// a lie.
    #[error("the container carries atlas pages but declares no rasterization mode")]
    NamatGhayrMuarraf,

    /// A page descriptor's texels run past the end of the atlas section.
    #[error("page {safha}: texels at {izaha}..{nihaya} of an atlas of {tul} bytes")]
    SafhaTalifa {
        /// The page index.
        safha: u32,
        /// Where its texels claim to start.
        izaha: u64,
        /// Where they claim to end.
        nihaya: u64,
        /// How large the atlas section actually is.
        tul: u64,
    },

    /// A section could not be compressed while writing.
    #[error("section {naw} could not be compressed")]
    DaghtFashil {
        /// The section kind.
        naw: u32,
        /// What the encoder reported.
        tafsil: String,
    },

    /// A file could not be opened, mapped, read or written.
    #[error("{masar} could not be read as a patch")]
    KhataMalaf {
        /// The path that failed.
        masar: PathBuf,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },
}

impl Tafsir for KhataRuqaa {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::RUQAA
                + match self {
                    Self::MalafQaseer { .. } => 0,
                    Self::SihrGhayrMutabaq { .. } => 1,
                    Self::IsdarGhayrMadum { .. } => 2,
                    Self::AlamMajhula { .. } => 3,
                    Self::AdadAqsamGhayrSalih { .. } => 4,
                    Self::HajmKulliGhayrMutabaq { .. } => 5,
                    Self::QismKharij { .. } => 6,
                    Self::IzahaGhayrMuhadhah { .. } => 7,
                    Self::AqsamMutadakhila { .. } => 8,
                    Self::NawQismMajhul { .. } => 9,
                    Self::QismMukarrar { .. } => 10,
                    Self::QismMafqud { .. } => 11,
                    Self::DaghtMajhul { .. } => 12,
                    Self::HajmKhaamMufrit { .. } => 13,
                    Self::MajmuKhaamMufrit { .. } => 14,
                    Self::FakkFashil { .. } => 15,
                    Self::HajmKhaamGhayrMutabaq { .. } => 16,
                    Self::BasmaGhayrMutabaqa { .. } => 17,
                    Self::TawqeeLaysAkhiran => 18,
                    Self::KutlatTawqeeTalifa { .. } => 19,
                    Self::GhayrMuwaqqaa => 20,
                    Self::TawqeeGhayrSalih { .. } => 21,
                    Self::JadwalTalif { .. } => 22,
                    Self::SijillGhayrMutabaq { .. } => 23,
                    Self::MuhadhahaGhayrSaliha { .. } => 24,
                    Self::FahrasKharij { .. } => 25,
                    Self::NassGhayrSalih { .. } => 26,
                    Self::BayanTalif { .. } => 27,
                    Self::NamatGhayrMuarraf => 28,
                    Self::SafhaTalifa { .. } => 29,
                    Self::DaghtFashil { .. } => 30,
                    Self::KhataMalaf { .. } => 31,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // A patch whose bytes do not hash to what it claims, or whose
            // signature is absent or wrong, is not a damaged patch — it is a
            // patch nobody can vouch for, and Decision 8 makes that the loudest
            // thing this crate can say.
            Self::BasmaGhayrMutabaqa { .. }
            | Self::GhayrMuwaqqaa
            | Self::TawqeeGhayrSalih { .. }
            | Self::TawqeeLaysAkhiran
            | Self::HajmKhaamMufrit { .. }
            | Self::MajmuKhaamMufrit { .. }
            | Self::HajmKhaamGhayrMutabaq { .. }
            | Self::AqsamMutadakhila { .. } => Khutura::Fadih,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MalafQaseer { .. } => {
                "هذا الملف أقصر من أن يكون رقعة تعريب؛ يبدو أنه ناقص أو أن تنزيله لم يكتمل."
                    .to_owned()
            },
            Self::SihrGhayrMutabaq { .. } => {
                "هذا الملف ليس رقعة تعريب. لا يبدأ بالعلامة التي تبدأ بها كل رقعة.".to_owned()
            },
            Self::IsdarGhayrMadum { wujid, .. } => format!(
                "هذه الرقعة بصيغة الإصدار {wujid}، وهذه النسخة من تعريب لا تقرأها. حدِّث \
                 التطبيق ثم أعد المحاولة."
            ),
            Self::AlamMajhula { .. } => {
                "ترويسة الرقعة تحمل خصائص غير معروفة في هذا الإصدار، فلا يمكن الوثوق بمعناها."
                    .to_owned()
            },
            Self::AdadAqsamGhayrSalih { adad, aqsa } => {
                format!("الرقعة تعلن {adad} قسمًا، والصيغة تعرّف {aqsa} أقسام على الأكثر.")
            },
            Self::HajmKulliGhayrMutabaq { .. } => {
                "حجم الملف لا يطابق ما تعلنه الترويسة؛ إما أن التنزيل لم يكتمل أو أن بايتات \
                 أُلحقت بالملف بعد إنشائه."
                    .to_owned()
            },
            Self::QismKharij { .. } | Self::JadwalTalif { .. } => {
                "أحد أقسام الرقعة يشير خارج حدود الملف. الملف تالف أو عُدِّل بعد توقيعه.".to_owned()
            },
            Self::IzahaGhayrMuhadhah { .. } | Self::MuhadhahaGhayrSaliha { .. } => {
                "أحد الأقسام لا يبدأ عند حدٍّ صالح في الذاكرة، ولا يمكن قراءته مباشرة كما \
                 تتطلب الصيغة."
                    .to_owned()
            },
            Self::AqsamMutadakhila { .. } => {
                "قسمان في الرقعة يتشاركان البايتات نفسها، وهو ما لا تنتجه أي رقعة سليمة.".to_owned()
            },
            Self::NawQismMajhul { naw } => {
                format!("الرقعة تحتوي قسمًا من نوع غير معروف ({naw}).")
            },
            Self::QismMukarrar { naw } => {
                format!("نوع القسم {naw} مكرر داخل الرقعة، ولا يمكن تحديد أيهما المقصود.")
            },
            Self::QismMafqud { .. } => "الرقعة ينقصها قسم أساسي لا تكتمل بدونه.".to_owned(),
            Self::DaghtMajhul { .. } => {
                "أحد الأقسام مضغوط بطريقة لا تعرفها هذه النسخة من تعريب.".to_owned()
            },
            Self::HajmKhaamMufrit { .. } | Self::MajmuKhaamMufrit { .. } => {
                "الرقعة تطلب حجمًا في الذاكرة أكبر بكثير مما تحتاجه أي رقعة حقيقية، ورُفضت \
                 قبل حجز أي بايت."
                    .to_owned()
            },
            Self::FakkFashil { .. } => "تعذّر فك ضغط أحد أقسام الرقعة؛ الملف تالف.".to_owned(),
            Self::HajmKhaamGhayrMutabaq { .. } => {
                "أحد الأقسام أنتج بعد فك الضغط حجمًا غير الذي أعلنه، وهو ما لا يحدث إلا في \
                 ملف مُعدَّل عمدًا."
                    .to_owned()
            },
            Self::BasmaGhayrMutabaqa { .. } => {
                "بصمة محتوى الرقعة لا تطابق المعلن في ترويستها. إما أن التنزيل تلف، وإما أن \
                 الملف عُدِّل. لن يُقرأ أي شيء من داخله."
                    .to_owned()
            },
            Self::TawqeeLaysAkhiran => {
                "كتلة التوقيع ليست في موضعها من الملف، فلا يمكن التحقق منها قبل قراءة \
                 المحتوى."
                    .to_owned()
            },
            Self::KutlatTawqeeTalifa { .. } => {
                "كتلة التوقيع داخل الرقعة تالفة أو ناقصة.".to_owned()
            },
            Self::GhayrMuwaqqaa => {
                "هذه الرقعة غير موقَّعة. لا يثبّت تعريب إلا ما وقّعه مالك المشروع بعد \
                 مراجعته."
                    .to_owned()
            },
            Self::TawqeeGhayrSalih { .. } => {
                "توقيع هذه الرقعة غير صحيح. المحتوى لا يطابق ما وُقِّع عليه، ولن تُثبَّت.".to_owned()
            },
            Self::SijillGhayrMutabaq { .. } => {
                "أحد جداول الرقعة يعلن حجم سجل مخالفًا لما تثبّته الصيغة، فلا يمكن قراءته.".to_owned()
            },
            Self::FahrasKharij { .. } => "أحد سجلات الرقعة يشير إلى عنصر غير موجود.".to_owned(),
            Self::NassGhayrSalih { .. } => {
                "أحد النصوص داخل الرقعة ليس ترميز UTF-8 صالحًا، ولن يُستبدل بحروف بديلة.".to_owned()
            },
            Self::BayanTalif { .. } => "بيانات الرقعة الوصفية غير قابلة للقراءة.".to_owned(),
            Self::NamatGhayrMuarraf => {
                "الرقعة تحمل صفحات رسم دون أن تعلن طريقة رسمها، ولا يجوز تخمينها.".to_owned()
            },
            Self::SafhaTalifa { .. } => "إحدى صفحات اللوحة تشير خارج حدود بياناتها.".to_owned(),
            Self::DaghtFashil { .. } => "تعذّر ضغط أحد أقسام الرقعة أثناء كتابتها.".to_owned(),
            Self::KhataMalaf { .. } => "تعذّر فتح ملف الرقعة أو قراءته.".to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MalafQaseer { haql, tul, matlub } => format!(
                "This file is too short to be a patch: {tul} bytes, and {matlub} are needed \
                 to read {haql}."
            ),
            Self::SihrGhayrMutabaq { .. } => {
                "This file is not a Taarib patch; it does not begin with the marker every \
                 patch begins with."
                    .to_owned()
            },
            Self::IsdarGhayrMadum { wujid, madum } => format!(
                "This patch is in format version {wujid} and this build reads version \
                 {madum}. Update Taarib and try again."
            ),
            Self::AlamMajhula { alam } => format!(
                "The header sets flag bits ({alam:#06x}) that this format version does not \
                 define, so its meaning cannot be trusted."
            ),
            Self::AdadAqsamGhayrSalih { adad, aqsa } => {
                format!("The patch declares {adad} sections; the format defines at most {aqsa}.")
            },
            Self::HajmKulliGhayrMutabaq { muallan, fili } => format!(
                "The header declares {muallan} bytes and {fili} are present: the download is \
                 incomplete, or bytes were appended after the patch was made."
            ),
            Self::QismKharij {
                naw,
                haql,
                qeema,
                hadd,
            } => format!(
                "Section {naw} points outside the file: {haql} is {qeema} against a limit of \
                 {hadd}. The file is corrupt or was edited after it was signed."
            ),
            Self::JadwalTalif {
                naw,
                haql,
                qeema,
                hadd,
            } => format!(
                "Section {naw}'s table header is inconsistent: {haql} is {qeema} against a \
                 limit of {hadd}."
            ),
            Self::IzahaGhayrMuhadhah { naw, izaha } => format!(
                "Section {naw} begins at byte {izaha}, which is not a 16-byte boundary, so \
                 its tables cannot be read in place."
            ),
            Self::MuhadhahaGhayrSaliha { naw, haql, izaha } => format!(
                "Section {naw}: {haql} at offset {izaha} could not be read in place. Either \
                 the offset is wrong, or the container's bytes were loaded into a buffer \
                 that is not 16-byte aligned."
            ),
            Self::AqsamMutadakhila { awwal, thani } => format!(
                "Sections {awwal} and {thani} share the same bytes, which no honest patch \
                 does."
            ),
            Self::NawQismMajhul { naw } => {
                format!("The patch contains a section of unknown kind ({naw}).")
            },
            Self::QismMukarrar { naw } => {
                format!("Section kind {naw} appears twice, so neither can be resolved.")
            },
            Self::QismMafqud { ism } => {
                format!("The patch is missing its {ism} section and is incomplete without it.")
            },
            Self::DaghtMajhul { naw, daght } => format!(
                "Section {naw} is compressed with method {daght}, which this build does not \
                 know."
            ),
            Self::HajmKhaamMufrit { naw, muallan, saqf } => format!(
                "Section {naw} declares {muallan} uncompressed bytes against a ceiling of \
                 {saqf}; it was refused before any memory was reserved."
            ),
            Self::MajmuKhaamMufrit { majmu, saqf } => format!(
                "The sections declare {majmu} uncompressed bytes in total against a ceiling \
                 of {saqf}."
            ),
            Self::FakkFashil { naw, .. } => {
                format!("Section {naw} could not be decompressed; the file is corrupt.")
            },
            Self::HajmKhaamGhayrMutabaq { naw, muallan, fili } => format!(
                "Section {naw} declared {muallan} uncompressed bytes and produced {fili}, \
                 which only happens in a deliberately edited file."
            ),
            Self::BasmaGhayrMutabaqa { .. } => {
                "The patch's content hash does not match the one in its header. The download \
                 is damaged or the file was modified; nothing inside it will be read."
                    .to_owned()
            },
            Self::TawqeeLaysAkhiran => {
                "The signature block is not at the end of the file, so it cannot be verified \
                 before the content is read."
                    .to_owned()
            },
            Self::KutlatTawqeeTalifa { haql } => {
                format!("The signature block is malformed: {haql}.")
            },
            Self::GhayrMuwaqqaa => {
                "This patch carries no signature. Taarib installs only what the project owner \
                 signed after reviewing it."
                    .to_owned()
            },
            Self::TawqeeGhayrSalih { miftah } => format!(
                "This patch's signature does not verify under key {miftah}. Its contents are \
                 not what was signed, and it will not be installed."
            ),
            Self::SijillGhayrMutabaq {
                naw,
                haql,
                wujid,
                muntazar,
            } => format!(
                "Section {naw} declares {haql} as {wujid} bytes; the format fixes it at \
                 {muntazar}, so the table cannot be read."
            ),
            Self::FahrasKharij { haql, fahras, adad } => {
                format!("A record refers to {haql} {fahras} of {adad}.")
            },
            Self::NassGhayrSalih { fahras, mawqi } => format!(
                "String {fahras} is not valid UTF-8 at byte {mawqi}, and will not be \
                 substituted with replacement characters."
            ),
            Self::BayanTalif { .. } => "The patch's metadata record could not be read.".to_owned(),
            Self::NamatGhayrMuarraf => {
                "The patch carries atlas pages but declares no rasterization mode, and \
                 guessing one is not allowed."
                    .to_owned()
            },
            Self::SafhaTalifa {
                safha,
                izaha,
                nihaya,
                tul,
            } => format!(
                "Page {safha} claims texels at {izaha}..{nihaya} of an atlas that is {tul} \
                 bytes long."
            ),
            Self::DaghtFashil { naw, .. } => {
                format!("Section {naw} could not be compressed while writing the patch.")
            },
            Self::KhataMalaf { masar, .. } => {
                format!(
                    "{} could not be opened or read as a patch.",
                    masar.display()
                )
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MalafRuqaa),
            Self::MalafQaseer { .. } | Self::SihrGhayrMutabaq { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MalafRuqaa,
            },
            Self::IsdarGhayrMadum { .. } => Khutwa::TahdithTaarib,
            // A hash mismatch is a corrupt download far more often than it is an
            // attack, and re-fetching resolves the common case in one click.
            Self::BasmaGhayrMutabaqa { .. } | Self::HajmKulliGhayrMutabaq { .. } => {
                Khutwa::AadaMuhawala
            },
            Self::GhayrMuwaqqaa
            | Self::TawqeeGhayrSalih { .. }
            | Self::TawqeeLaysAkhiran
            | Self::KutlatTawqeeTalifa { .. }
            | Self::AqsamMutadakhila { .. }
            | Self::HajmKhaamGhayrMutabaq { .. } => Khutwa::IblaghLilMalik,
            _ => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::MalafQaseer { haql, tul, matlub } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("tul", QeemaSiyaq::Hajm(*tul));
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
            },
            Self::SihrGhayrMutabaq { wujid } => {
                daa("wujid", QeemaSiyaq::Nass(format!("{wujid:02x?}")));
            },
            Self::IsdarGhayrMadum { wujid, madum } => {
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("madum", QeemaSiyaq::Raqm(i64::from(*madum)));
            },
            Self::AlamMajhula { alam } => {
                daa("alam", QeemaSiyaq::Raqm(i64::from(*alam)));
            },
            Self::AdadAqsamGhayrSalih { adad, aqsa } => {
                daa("adad", QeemaSiyaq::Raqm(i64::from(*adad)));
                daa("aqsa", QeemaSiyaq::Raqm(i64::from(*aqsa)));
            },
            Self::HajmKulliGhayrMutabaq { muallan, fili } => {
                daa("muallan", QeemaSiyaq::Hajm(*muallan));
                daa("fili", QeemaSiyaq::Hajm(*fili));
            },
            Self::QismKharij {
                naw,
                haql,
                qeema,
                hadd,
            }
            | Self::JadwalTalif {
                naw,
                haql,
                qeema,
                hadd,
            } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("hadd", QeemaSiyaq::Hajm(*hadd));
            },
            Self::IzahaGhayrMuhadhah { naw, izaha } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("izaha", QeemaSiyaq::Hajm(*izaha));
            },
            Self::MuhadhahaGhayrSaliha { naw, haql, izaha } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("izaha", QeemaSiyaq::Hajm(*izaha));
            },
            Self::AqsamMutadakhila { awwal, thani } => {
                daa("awwal", QeemaSiyaq::Raqm(i64::from(*awwal)));
                daa("thani", QeemaSiyaq::Raqm(i64::from(*thani)));
            },
            Self::NawQismMajhul { naw } | Self::QismMukarrar { naw } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
            },
            Self::QismMafqud { ism } => {
                daa("qism", QeemaSiyaq::Nass((*ism).to_owned()));
            },
            Self::DaghtMajhul { naw, daght } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("daght", QeemaSiyaq::Raqm(i64::from(*daght)));
            },
            Self::HajmKhaamMufrit { naw, muallan, saqf } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("muallan", QeemaSiyaq::Hajm(*muallan));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            },
            Self::MajmuKhaamMufrit { majmu, saqf } => {
                daa("majmu", QeemaSiyaq::Hajm(*majmu));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            },
            Self::FakkFashil { naw, tafsil } | Self::DaghtFashil { naw, tafsil } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::HajmKhaamGhayrMutabaq { naw, muallan, fili } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("muallan", QeemaSiyaq::Hajm(*muallan));
                daa("fili", QeemaSiyaq::Hajm(*fili));
            },
            Self::BasmaGhayrMutabaqa { muallana, mahsuba } => {
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            },
            Self::KutlatTawqeeTalifa { haql } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
            },
            Self::TawqeeGhayrSalih { miftah } => {
                daa("miftah", QeemaSiyaq::Nass(miftah.clone()));
            },
            Self::SijillGhayrMutabaq {
                naw,
                haql,
                wujid,
                muntazar,
            } => {
                daa("naw", QeemaSiyaq::Raqm(i64::from(*naw)));
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("muntazar", QeemaSiyaq::Raqm(i64::from(*muntazar)));
            },
            Self::FahrasKharij { haql, fahras, adad } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("fahras", QeemaSiyaq::Raqm(i64::from(*fahras)));
                daa("adad", QeemaSiyaq::Raqm(i64::from(*adad)));
            },
            Self::NassGhayrSalih { fahras, mawqi } => {
                daa("fahras", QeemaSiyaq::Raqm(i64::from(*fahras)));
                daa("mawqi", QeemaSiyaq::Raqm(i64::from(*mawqi)));
            },
            Self::BayanTalif { tafsil } => {
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::SafhaTalifa {
                safha,
                izaha,
                nihaya,
                tul,
            } => {
                daa("safha", QeemaSiyaq::Raqm(i64::from(*safha)));
                daa("izaha", QeemaSiyaq::Hajm(*izaha));
                daa("nihaya", QeemaSiyaq::Hajm(*nihaya));
                daa("tul", QeemaSiyaq::Hajm(*tul));
            },
            Self::KhataMalaf { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                for (miftah, qeema) in siyaq_io(sabab) {
                    daa(&miftah, qeema);
                }
            },
            Self::TawqeeLaysAkhiran | Self::GhayrMuwaqqaa | Self::NamatGhayrMuarraf => {},
        }
        siyaq
    }
}

khata_min!(KhataRuqaa);
