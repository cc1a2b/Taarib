//! أخطاء الطبقة — what stops the overlay, and what merely narrows it.
//!
//! Every failure in this crate happens **inside somebody's game, while they are
//! playing it**, and that single fact sets the posture for the whole module. The
//! adapters that came before could refuse and leave the player exactly where
//! they started. This one cannot: by the time anything here fails, Taarib's code
//! is already running on the presentation path of a live process, sixteen
//! milliseconds from the next frame.
//!
//! So the ordering of preferences is inverted relative to every other crate:
//!
//! 1. **Draw nothing this frame** and try again on the next one.
//! 2. **Disable the overlay and unhook cleanly**, leaving the game running.
//! 3. **Refuse to hook at all**, before anything is installed.
//!
//! There is no fourth option. Nothing in this crate is permitted to fail in a
//! way that takes the game with it, which is why [`KhataTabaqa`] has no variant
//! meaning "the frame is lost" — a frame Taarib cannot draw into is a frame that
//! renders without Arabic, and that is a degraded overlay rather than an error
//! anybody needs to see.
//!
//! ## Three classes, deliberately unequal
//!
//! **Hook failures** — a vtable that could not be located, a swap chain whose
//! device is not the one that was hooked, a Vulkan layer the loader did not pick
//! up — are [`Khutura::Tanbeeh`]. The overlay is a fallback tier; a fallback
//! that cannot install is a capability report, not a crash. The user is told
//! plainly which API was tried and what was found.
//!
//! **Recognition failures** — no engine available, a model that will not load, a
//! region that produced nothing readable — are [`Khutura::Maluma`] or
//! [`Khutura::Tanbeeh`]. Tier 3 is *documented* as imperfect. An OCR pass that
//! returns nothing is the normal case for a frame with no text in the region,
//! and treating it as an error would produce an error per frame.
//!
//! **State-corruption risks** are the strictest thing in this crate, and they
//! are the reason [`KhataTabaqa::HalaGhayrMustaada`] and
//! [`KhataTabaqa::TahrirGhayrMutabaq`] exist. The overlay draws through the
//! game's own device. If OpenGL state cannot be restored, or a D3D12 fence says
//! the overlay's resources are still in flight when they were about to be freed,
//! the correct behaviour is to disable the overlay immediately and permanently
//! for this session — not to try again next frame with state the game no longer
//! recognises. A tier-3 overlay that corrupts a renderer has done more damage
//! than never having existed.
//!
//! ## Error codes
//!
//! This crate owns [`arqam::TABAQA`], which is 8000, in its entirety — the
//! overlay is a subsystem rather than one of the adapters, so it does not share
//! a band with anything. Codes are permanent: users paste them into bug reports
//! and search for them years later, so a variant is never renumbered and a
//! removed variant's number is never reused.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Failures of the universal overlay.
#[derive(Debug, thiserror::Error)]
pub enum KhataTabaqa {
    /// The graphics API this build was asked to hook is not one it implements.
    ///
    /// Distinct from "the hook failed": this is Metal on a Windows build, and
    /// the answer is a different tier or a different platform rather than a
    /// retry.
    ///
    /// It used to name Direct3D 9 as the other example. That is no longer true
    /// — [`crate::d3d9`] is a backend now — and the sentence is corrected here
    /// rather than left, because a documented refusal that has stopped being a
    /// refusal is the kind of stale comment somebody later treats as a
    /// requirement.
    #[error("{api} is not a graphics API this build can hook")]
    ApiGhayrMadum {
        /// The API as it was identified.
        api: String,
    },

    /// No graphics API could be identified in the process at all.
    ///
    /// A game whose renderer loads lazily produces this on an early attempt and
    /// succeeds a moment later, so the retry is worth offering rather than
    /// treating this as final.
    #[error("no supported graphics API was found in this process")]
    ApiGhayrMawjud {
        /// Which modules were looked for and what was seen, for the report.
        athar: Vec<String>,
    },

    /// A module the hook needs is not loaded, or does not export what it must.
    #[error("{maktaba} is not loaded, or does not export {ramz}")]
    MaktabaMafquda {
        /// The module's name.
        maktaba: String,
        /// The symbol that was wanted.
        ramz: String,
    },

    /// The interface's method table could not be located.
    ///
    /// COM vtable hooking needs a real instance to read the table from, which is
    /// obtained by creating a throwaway device and swap chain. When that
    /// creation fails — a headless session, an exhausted adapter, a driver that
    /// refuses a second device — there is no table to read and no hook to place.
    #[error("the method table for {wajiha} could not be located: {sabab}")]
    JadwalGhayrMawjud {
        /// The interface whose table was wanted.
        wajiha: String,
        /// What went wrong.
        sabab: String,
    },

    /// A hook could not be installed.
    #[error("the {mawdi} hook could not be installed: {sabab}")]
    KhatfFashil {
        /// Which hook point.
        mawdi: String,
        /// Why.
        sabab: String,
    },

    /// A hook could not be removed.
    ///
    /// Worse than a failed install, and treated as such. An overlay that cannot
    /// unhook cannot be disabled and cannot be unloaded, so the module is pinned
    /// for the life of the process and the user is told that a restart is what
    /// removes it.
    #[error("the {mawdi} hook could not be removed: {sabab}")]
    FakkKhatfFashil {
        /// Which hook point.
        mawdi: String,
        /// Why.
        sabab: String,
    },

    /// Memory protection could not be changed for a write the hook requires.
    #[error("page protection at {oinwan:#x} could not be changed to write {tul} byte(s)")]
    HimayaGhayrQabila {
        /// The address.
        oinwan: u64,
        /// How many bytes were to be written.
        tul: usize,
    },

    /// A graphics resource could not be created.
    #[error("the overlay's {mawrid} could not be created: {sabab}")]
    MawridFashil {
        /// Which resource — a texture, a buffer, a descriptor heap, a pipeline.
        mawrid: &'static str,
        /// The API's own account of why.
        sabab: String,
    },

    /// The device or swap chain changed underneath the overlay's resources.
    ///
    /// A resolution change, a fullscreen transition, an adapter reset. Recovered
    /// from rather than fatal: the overlay releases everything sized to the old
    /// surface and rebuilds on the next frame. It is an error type at all
    /// because the rebuild must be *forced* — resources silently outliving the
    /// surface they were sized for is how an overlay draws into freed memory.
    #[error("the presentation surface changed: {sabab}")]
    SathTaghayyar {
        /// What changed.
        sabab: String,
    },

    /// Renderer state could not be put back the way the game left it.
    ///
    /// The most serious failure this crate has. OpenGL is a global state machine
    /// and the game owns it; anything the overlay binds it must unbind, and if
    /// the restore did not complete the game is now drawing with state it did
    /// not set. The overlay disables itself for the session rather than
    /// continuing, because every subsequent frame compounds the damage.
    #[error("{hala} could not be restored after the overlay drew: {sabab}")]
    HalaGhayrMustaada {
        /// Which piece of state.
        hala: &'static str,
        /// Why the restore failed.
        sabab: String,
    },

    /// A trampoline's relocated prologue does not match what was read.
    ///
    /// Verified before the hook goes live rather than discovered afterwards. A
    /// prologue that does not round-trip means the instruction decoder
    /// mis-measured it, and installing over that is how a process crashes with a
    /// stack that names nothing.
    #[error("the relocated prologue at {mawdi} does not match the original")]
    TahrirGhayrMutabaq {
        /// Which hook point.
        mawdi: String,
        /// How many bytes disagree.
        adad: usize,
    },

    /// No text recognizer is available on this machine.
    #[error("no on-screen text recognizer is available: {sabab}")]
    QariGhayrMutah {
        /// What was tried and what was found.
        sabab: String,
    },

    /// A recognition model could not be loaded.
    #[error("the recognition model at {masar} could not be loaded: {sabab}")]
    NamudhajFashil {
        /// Where the model was expected.
        masar: PathBuf,
        /// Why it would not load.
        sabab: String,
    },

    /// Recognition ran and produced nothing usable.
    ///
    /// Not a failure of the machine. A region containing no text, text too small
    /// to resolve, or text on a background the preprocessor could not separate
    /// all land here, and all three are ordinary. It exists as a value so the
    /// control panel can say *why* a region is showing nothing.
    #[error("nothing readable was recognized in {mintaqa}")]
    LaNassMaqru {
        /// Which region.
        mintaqa: String,
        /// The recognizer's own confidence in what little it saw, if any.
        thiqa: Option<u8>,
    },

    /// A captured frame is in a format this build cannot convert.
    #[error("the backbuffer format {sigha} cannot be converted for recognition")]
    SighaGhayrMaduma {
        /// The format, as the API names it.
        sigha: String,
    },

    /// A frame could not be read back from the GPU.
    #[error("the frame could not be read back: {sabab}")]
    IltiqatFashil {
        /// Why.
        sabab: String,
    },

    /// A region is not inside the surface it was defined against.
    ///
    /// Regions are stored in normalized coordinates precisely so a resolution
    /// change does not invalidate them, so reaching this means a region was
    /// stored in pixels by an older build, or a surface reported a zero
    /// dimension.
    #[error("region {mintaqa} does not fit a {ard}×{irtifa} surface")]
    MintaqaKharij {
        /// Which region.
        mintaqa: String,
        /// The surface width.
        ard: u32,
        /// The surface height.
        irtifa: u32,
    },

    /// The overlay was asked to draw before the user was told what tier 3 is.
    ///
    /// Structural rather than defensive. The disclosure is not a dialog this
    /// crate hopes a caller shows — it is a value the caller must produce and
    /// hand over, and this is what happens if a future edit routes around it.
    /// See [`crate::sidq`].
    #[error("the overlay was enabled without the tier-3 disclosure being acknowledged")]
    IfsahMafqud,

    /// A region file or history store could not be read or written.
    #[error("{masar} could not be read or written")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// The underlying failure.
        sabab: std::io::Error,
    },

    /// A stored region set or history file is not in a shape this build knows.
    #[error("{masar} is not a {sigha} this build understands: {sabab}")]
    MalafGhayrMafhum {
        /// The path.
        masar: PathBuf,
        /// What it was expected to be.
        sigha: &'static str,
        /// What was wrong with it.
        sabab: String,
    },

    /// A declared size is larger than anything this build will allocate.
    #[error("{haql} declares {qeema}, above this build's ceiling of {saqf}")]
    HajmMufrit {
        /// Which field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },
}

impl KhataTabaqa {
    /// Whether this refusal ends recognition for the session rather than for
    /// one capture.
    ///
    /// Two variants mean the recognizer itself is gone — a language pack
    /// uninstalled mid-session, model files quarantined by an antivirus — and no
    /// later frame changes that. Every other refusal is about one capture: a
    /// degenerate region, a frame the GPU would not read back, a rectangle that
    /// no longer fits the surface. Retrying the next capture is right for the
    /// second kind and pointless for the first, and a worker that counted both
    /// as one integer could not tell a user which of the two they were looking
    /// at. See [`crate::qissa::KhaytQissa`].
    #[must_use]
    pub const fn yunhi_al_qiraa(&self) -> bool {
        matches!(
            self,
            Self::QariGhayrMutah { .. } | Self::NamudhajFashil { .. }
        )
    }
}

/// The first code this crate uses. It owns its band outright.
const AWWAL: u16 = 0;

impl Tafsir for KhataTabaqa {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::TABAQA
                + AWWAL
                + match self {
                    Self::ApiGhayrMadum { .. } => 0,
                    Self::ApiGhayrMawjud { .. } => 1,
                    Self::MaktabaMafquda { .. } => 2,
                    Self::JadwalGhayrMawjud { .. } => 3,
                    Self::KhatfFashil { .. } => 4,
                    Self::FakkKhatfFashil { .. } => 5,
                    Self::HimayaGhayrQabila { .. } => 6,
                    Self::MawridFashil { .. } => 7,
                    Self::SathTaghayyar { .. } => 8,
                    Self::HalaGhayrMustaada { .. } => 9,
                    Self::TahrirGhayrMutabaq { .. } => 10,
                    Self::QariGhayrMutah { .. } => 11,
                    Self::NamudhajFashil { .. } => 12,
                    Self::LaNassMaqru { .. } => 13,
                    Self::SighaGhayrMaduma { .. } => 14,
                    Self::IltiqatFashil { .. } => 15,
                    Self::MintaqaKharij { .. } => 16,
                    Self::IfsahMafqud => 17,
                    Self::KhataMalaf { .. } => 18,
                    Self::MalafGhayrMafhum { .. } => 19,
                    Self::HajmMufrit { .. } => 20,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Nothing was recognized, which is what an empty region looks like
            // every frame. Reporting this as a problem would report a problem
            // sixty times a second.
            Self::LaNassMaqru { .. } => Khutura::Maluma,

            // The overlay is a fallback. A fallback that cannot install is a
            // capability report — the game is untouched and still running.
            Self::ApiGhayrMadum { .. }
            | Self::ApiGhayrMawjud { .. }
            | Self::MaktabaMafquda { .. }
            | Self::JadwalGhayrMawjud { .. }
            | Self::KhatfFashil { .. }
            | Self::HimayaGhayrQabila { .. }
            | Self::MawridFashil { .. }
            | Self::SathTaghayyar { .. }
            | Self::QariGhayrMutah { .. }
            | Self::NamudhajFashil { .. }
            | Self::SighaGhayrMaduma { .. }
            | Self::IltiqatFashil { .. }
            | Self::MintaqaKharij { .. } => Khutura::Tanbeeh,

            // The overlay is running inside somebody's game and the renderer or
            // the process is at risk. Every one of these disables the overlay
            // rather than retrying.
            Self::HalaGhayrMustaada { .. }
            | Self::TahrirGhayrMutabaq { .. }
            | Self::FakkKhatfFashil { .. }
            | Self::IfsahMafqud => Khutura::Fadih,

            Self::KhataMalaf { .. } | Self::MalafGhayrMafhum { .. } | Self::HajmMufrit { .. } => {
                Khutura::Khatar
            },
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::ApiGhayrMadum { api } => {
                format!("واجهة الرسوميات ({api}) غير مدعومة في هذه النسخة من تعريب.")
            },
            Self::ApiGhayrMawjud { .. } => {
                "لم يُعثر على واجهة رسوميات مدعومة في هذه اللعبة. قد تكون اللعبة لم تُنشئ \
                 عارضها بعد؛ جرّب مرة أخرى بعد ظهور الصورة."
                    .to_owned()
            },
            Self::MaktabaMafquda { maktaba, .. } => {
                format!("المكتبة {maktaba} غير محمّلة في اللعبة، أو لا تصدّر ما تحتاجه الطبقة.")
            },
            Self::JadwalGhayrMawjud { .. } => {
                "تعذّر الوصول إلى جدول دوال العارض، ولا يمكن تركيب الطبقة بدونه.".to_owned()
            },
            Self::KhatfFashil { .. } => {
                "تعذّر تركيب الطبقة على مسار العرض. اللعبة تعمل كما هي ولم يُغيَّر فيها شيء.".to_owned()
            },
            Self::FakkKhatfFashil { .. } => {
                "تعذّر إزالة الطبقة من اللعبة أثناء تشغيلها. أغلق اللعبة وأعد تشغيلها لإزالتها \
                 تمامًا."
                    .to_owned()
            },
            Self::HimayaGhayrQabila { .. } => {
                "رفض النظام السماح بالكتابة في ذاكرة اللعبة، وقد يكون ذلك بسبب نظام حماية \
                 مضاد للعبث."
                    .to_owned()
            },
            Self::MawridFashil { .. } => {
                "تعذّر إنشاء موارد الرسم التي تحتاجها الطبقة على كرت الشاشة.".to_owned()
            },
            Self::SathTaghayyar { .. } => {
                "تغيّرت دقة الشاشة أو وضع العرض، وتُعاد تهيئة الطبقة.".to_owned()
            },
            Self::HalaGhayrMustaada { .. } => {
                "تعذّر إرجاع حالة العارض كما كانت بعد الرسم، وأُوقفت الطبقة فورًا حمايةً \
                 للعبة. أعد تشغيل اللعبة."
                    .to_owned()
            },
            Self::TahrirGhayrMutabaq { .. } => {
                "لم تتطابق بداية الدالة المُعاد توجيهها مع ما قُرئ منها، وأُلغي التركيب قبل \
                 تفعيله."
                    .to_owned()
            },
            Self::QariGhayrMutah { .. } => {
                "لا يوجد محرّك قراءة نصوص متاح على هذا الجهاز، ولا يمكن للطبقة قراءة ما على \
                 الشاشة بدونه."
                    .to_owned()
            },
            Self::NamudhajFashil { .. } => {
                "تعذّر تحميل نموذج قراءة النصوص. قد يكون ملفه ناقصًا أو تالفًا.".to_owned()
            },
            Self::LaNassMaqru { mintaqa, .. } => {
                format!("لم يُقرأ نص واضح في المنطقة ({mintaqa}).")
            },
            Self::SighaGhayrMaduma { sigha } => {
                format!("صيغة الصورة التي تعرضها اللعبة ({sigha}) غير مدعومة للقراءة.")
            },
            Self::IltiqatFashil { .. } => "تعذّر التقاط صورة الإطار من كرت الشاشة.".to_owned(),
            Self::MintaqaKharij { mintaqa, .. } => {
                format!("المنطقة ({mintaqa}) تقع خارج حدود الشاشة الحالية.")
            },
            Self::IfsahMafqud => {
                "لا يمكن تفعيل الطبقة قبل عرض شرحها على المستخدم والموافقة عليه.".to_owned()
            },
            Self::KhataMalaf { .. } => {
                "تعذّر قراءة أحد ملفات إعدادات الطبقة أو الكتابة إليه.".to_owned()
            },
            Self::MalafGhayrMafhum { .. } => {
                "أحد ملفات إعدادات الطبقة ليس بالصيغة التي تعرفها هذه النسخة.".to_owned()
            },
            Self::HajmMufrit { .. } => {
                "طُلب حجز حجم أكبر مما تسمح به هذه النسخة، ورُفض الطلب قبل حجز أي ذاكرة.".to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // The renderer may simply not exist yet. Trying again once the game
            // is drawing is the single most useful thing a user can do here.
            Self::ApiGhayrMawjud { .. } | Self::SathTaghayyar { .. } => Khutwa::AadaMuhawala,

            // Something in the process is not what this build expected. The
            // engine probe's report is where a person looks next.
            Self::ApiGhayrMadum { .. }
            | Self::MaktabaMafquda { .. }
            | Self::JadwalGhayrMawjud { .. }
            | Self::KhatfFashil { .. }
            | Self::HimayaGhayrQabila { .. }
            | Self::MawridFashil { .. }
            | Self::TahrirGhayrMutabaq { .. } => Khutwa::FathTashkhis,

            // The recognizer is a payload the framework installs. A missing or
            // damaged model is repaired by reinstalling it.
            Self::QariGhayrMutah { .. } | Self::NamudhajFashil { .. } => Khutwa::IadatTarkibIttar,

            // Nothing the user does inside the running game changes the
            // outcome: either the overlay has stopped and the process is
            // suspect, or the frame held nothing this build could read.
            Self::HalaGhayrMustaada { .. }
            | Self::FakkKhatfFashil { .. }
            | Self::LaNassMaqru { .. }
            | Self::SighaGhayrMaduma { .. }
            | Self::IltiqatFashil { .. } => Khutwa::LaShay,

            // A region the user drew no longer fits, which the region editor is
            // exactly the place to fix.
            Self::MintaqaKharij { .. } => Khutwa::TahrirMasaha,

            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),

            // A stored region set or history file this build cannot parse was
            // written by Taarib, as were the missing disclosure and the oversized
            // declaration. All three are Taarib's to explain, and none is
            // something the user can fix by choosing a different path.
            Self::MalafGhayrMafhum { .. } | Self::IfsahMafqud | Self::HajmMufrit { .. } => {
                Khutwa::IblaghLilMusahim
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        // Handled before the general closure exists, because the io helper
        // returns a *different* map and inserting the path into the map built
        // below would insert it into something that is then thrown away. The
        // same mistake was made in three adapter crates before this one; doing
        // it in this order is what makes it impossible here.
        if let Self::KhataMalaf { masar, sabab } = self {
            let mut siyaq = siyaq_io(sabab);
            let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            return siyaq;
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::ApiGhayrMadum { api } => daa("api", QeemaSiyaq::Nass(api.clone())),
            Self::ApiGhayrMawjud { athar } => daa("athar", QeemaSiyaq::Qaima(athar.clone())),
            Self::MaktabaMafquda { maktaba, ramz } => {
                daa("maktaba", QeemaSiyaq::Nass(maktaba.clone()));
                daa("ramz", QeemaSiyaq::Nass(ramz.clone()));
            },
            Self::JadwalGhayrMawjud { wajiha, sabab } => {
                daa("wajiha", QeemaSiyaq::Nass(wajiha.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::KhatfFashil { mawdi, sabab } | Self::FakkKhatfFashil { mawdi, sabab } => {
                daa("mawdi", QeemaSiyaq::Nass(mawdi.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::HimayaGhayrQabila { oinwan, tul } => {
                daa("oinwan", QeemaSiyaq::Hajm(*oinwan));
                daa("tul", QeemaSiyaq::Hajm(tul_u64(*tul)));
            },
            Self::MawridFashil { mawrid, sabab } => {
                daa("mawrid", QeemaSiyaq::Nass((*mawrid).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::SathTaghayyar { sabab }
            | Self::QariGhayrMutah { sabab }
            | Self::IltiqatFashil { sabab } => daa("sabab", QeemaSiyaq::Nass(sabab.clone())),
            Self::HalaGhayrMustaada { hala, sabab } => {
                daa("hala", QeemaSiyaq::Nass((*hala).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::TahrirGhayrMutabaq { mawdi, adad } => {
                daa("mawdi", QeemaSiyaq::Nass(mawdi.clone()));
                daa("adad", QeemaSiyaq::Hajm(tul_u64(*adad)));
            },
            Self::NamudhajFashil { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::LaNassMaqru { mintaqa, thiqa } => {
                daa("mintaqa", QeemaSiyaq::Nass(mintaqa.clone()));
                if let Some(qeema) = thiqa {
                    daa("thiqa", QeemaSiyaq::Raqm(i64::from(*qeema)));
                }
            },
            Self::SighaGhayrMaduma { sigha } => daa("sigha", QeemaSiyaq::Nass(sigha.clone())),
            Self::MintaqaKharij {
                mintaqa,
                ard,
                irtifa,
            } => {
                daa("mintaqa", QeemaSiyaq::Nass(mintaqa.clone()));
                daa("ard", QeemaSiyaq::Hajm(u64::from(*ard)));
                daa("irtifa", QeemaSiyaq::Hajm(u64::from(*irtifa)));
            },
            Self::MalafGhayrMafhum {
                masar,
                sigha,
                sabab,
            } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::HajmMufrit { haql, qeema, saqf } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            },
            // Handled above, and carries nothing beyond its own existence.
            Self::KhataMalaf { .. } | Self::IfsahMafqud => {},
        }
        siyaq
    }
}

khata_min!(KhataTabaqa);

/// A length as a `u64`, without a cast that can wrap.
///
/// `usize` is at most 64 bits on every target this product builds for, and the
/// conversion is written as a fallible one that saturates rather than as an
/// `as`, because `cast_possible_truncation` is denied workspace-wide and an
/// `#[allow]` here would be an allow in the one place lengths are reported.
#[must_use]
pub fn tul_u64(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

/// A `u64` back to a length, or [`None`] when it does not fit this target.
#[must_use]
pub fn hajm_usize(qeema: u64) -> Option<usize> {
    usize::try_from(qeema).ok()
}
