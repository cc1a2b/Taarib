//! الاستطلاع — what a game's own files say about the overlay, before anything
//! is installed into them.
//!
//! [`crate::qudra`] is the vocabulary a live backend reports in, and every
//! backend produces one of its verdicts from a device it is already attached to.
//! This module produces the same vocabulary from a directory on disk, and it
//! exists because two of the ways tier 3 fails are visible there and nowhere
//! else — early enough to be a refusal with a reason rather than a player
//! staring at a game with nothing drawn on it.
//!
//! ## The proxy slot, which is the one that must be refused
//!
//! Taarib reaches a game with no third-party framework by placing
//! `taarib-mudkhal` beside its executable under a name the game's own loader
//! already resolves — `version.dll` — so that Windows brings it in and it opens
//! the payloads next to it. That is **one** name, and it is a name other
//! products use too.
//!
//! Games of the Direct3D 9 era attract those products: a `dinput8.dll` that is
//! really a mod loader, a `d3d9.dll` that is really a shader injector, a
//! `winmm.dll` that is really a frame limiter. Having one is the ordinary case
//! rather than the exceptional one. And a proxy name is not a shared resource —
//! whatever occupies it *is* that module to the game — so writing Taarib's
//! loader over one deletes the other product, and there is no second copy of the
//! name to write beside it.
//!
//! So the rule is absolute and it is checked here: **Taarib does not take a
//! proxy slot another product has taken.** The verdict is
//! [`crate::qudra::HukmQudra::Mustaheela`] when [`SLOT_TAARIB`] holds something
//! that is not Taarib's own module, and the reason names the file and its size
//! so the user can recognise what they already installed.
//!
//! The rule's twin lives in [`crate::khataf::Khataf::fukk`] and the two are one
//! policy: Taarib does not restore a function pointer it did not install.
//! Chaining a hook onto an existing one is fine and is what happens when another
//! overlay is already live in the process — it is *unhooking out of order* that
//! leaves the other product calling into memory that has been unmapped, and that
//! is refused by reading the slot back before writing to it.
//!
//! ## A slot that could not be read is not a free slot
//!
//! The rule above is only as good as the read behind it. A `version.dll` an
//! antivirus is holding open, one past the size this survey will load, one
//! behind a symlink loop or an ACL the survey cannot traverse — each answers the
//! question "what is in Taarib's slot?" with *nothing*, and nothing is exactly
//! what an empty slot answers too. So every slot that could not be read is
//! carried in [`TaqrirIstitlaa::thughrat`] with the reason, [`halat_slot`]
//! answers with its own state for it, and the verdict for Taarib's slot is
//! [`crate::qudra::HukmQudra::Majhula`] — not "taken", which would be a claim
//! about a file nothing read, and not "free", which would be the overwrite
//! this module exists to prevent.
//!
//! [`halat_slot`]: TaqrirIstitlaa::halat_slot
//!
//! ## The import table
//!
//! Which graphics API a game uses is not knowable with certainty from its files
//! — a game can import `d3d11.dll` and render through a translation layer — but
//! which ones it *links* is, and that is the difference between "this is
//! probably reachable, by this API first" and no information at all.
//!
//! The import directory is walked by hand rather than through a PE crate. This
//! crate is loaded into game processes, and a parser it does not need at run
//! time is a parser it should not carry into somebody's address space. The walk
//! is bounded at both loops, every read is bounds-checked, and it reads the
//! module names and nothing else: no exports, no relocations, no resources, and
//! **no delay-loaded imports** — so a game that delay-loads `d3d9.dll` reports
//! as importing nothing, which is the same "not final" answer a game with a
//! run-time `LoadLibrary` gets, and the report says so in those words rather
//! than claiming certainty it does not have.

use std::path::{Path, PathBuf};

use crate::khata::KhataTabaqa;
use crate::qudra::{HukmQudra, SababQudra};
use crate::wajiha::WajihatRusum;

/// The name `taarib-tathbeet` places `taarib-mudkhal` under on Windows.
///
/// Spelled here rather than depended on: reaching into the installer crate would
/// drag discovery, signing and backup machinery into a crate that is loaded
/// inside game processes. A rename there without a rename here would make this
/// module check the wrong slot, which is why the constant carries this note.
pub const SLOT_TAARIB: &str = "version.dll";

/// A byte string that appears in `taarib-mudkhal` and in nothing else.
///
/// `mudkhal.sijill` is the name that module writes its own log under, compiled
/// into it as an ASCII literal. Searching a candidate proxy for it answers "did
/// Taarib put this here?" without a signature check, an installation manifest,
/// or a second file that could disagree with the first — and it is described as
/// the heuristic it is. The consequence of a false negative is declining to
/// overwrite a file, which is the safe direction; a false positive would need
/// somebody to have put that string into their own DLL deliberately.
const BASMAT_MUDKHAL: &[u8] = b"mudkhal.sijill";

/// The largest file this module will read into memory to inspect.
///
/// Thirty-two mebibytes covers every game executable and every proxy DLL there
/// is. The cap exists because this runs against a directory the user chose, and
/// a path that resolves to something enormous should be declined rather than
/// loaded into a process that may be a 32-bit game. A proxy slot past it is
/// reported as unread, never as free and never as somebody else's.
pub const AQSA_MALAF: u64 = 32 * 1024 * 1024;

/// How many import descriptors are walked before the table is called malformed.
const AQSA_MUSTAWRADAT: usize = 4_096;

/// How long an imported module's name may be.
const AQSA_ISM: usize = 256;

/// The proxy names a Windows game's own loader resolves out of its directory.
///
/// Every one of these is a **system** DLL: the real implementation lives under
/// `System32`, so a file of the same name beside the executable is found first
/// and is a proxy by construction. That is the whole test, and it is why
/// redistributables a game legitimately ships beside itself — `d3dx9_43.dll` and
/// its siblings, the Visual C++ runtimes — are deliberately absent from this
/// list. They live nowhere else, so their presence says nothing at all.
///
/// Graphics and input names first, because those are what a Direct3D 9 era mod
/// takes, and Taarib's own slot last, so a report reads as "here is what is
/// already here, and here is what that means for us".
pub const SLOTAT_WAKEEL: &[&str] = &[
    "d3d8.dll",
    "d3d9.dll",
    "d3d10.dll",
    "d3d11.dll",
    "ddraw.dll",
    "dxgi.dll",
    "opengl32.dll",
    "dinput.dll",
    "dinput8.dll",
    "dsound.dll",
    // `xinput1_3.dll` and `xinput1_4.dll` used to be here and are now in
    // [`SLOTAT_MUSHTARAKA`]. They failed this list's own membership rule: the
    // DirectX end-user redistributable installs both beside a game, so a file
    // with either name is Microsoft's far more often than it is a mod, and a
    // name that proves nothing does not belong on a list whose whole premise is
    // that the name is the proof.
    "winmm.dll",
    "winhttp.dll",
    "wininet.dll",
    "dbghelp.dll",
    SLOT_TAARIB,
];

/// The proxy names that are a graphics module.
///
/// The separation matters because the two kinds of occupied slot ask different
/// questions. A `dinput8.dll` belonging to a mod loader means a third party is
/// live in the process, which is a disclosure. A `dxgi.dll` belonging to
/// `ReShade` means a third party is **on the presentation path**, which is the
/// question of whether the overlay draws at all.
const SLOTAT_RUSUM: &[&str] = &[
    "d3d8.dll",
    "d3d9.dll",
    "d3d10.dll",
    "d3d11.dll",
    "d3d12.dll",
    "ddraw.dll",
    "dxgi.dll",
    "opengl32.dll",
];

/// Names a game may legitimately ship, which are reported only once identified.
///
/// `xinput1_3.dll` is Microsoft's DirectX redistributable in most games and
/// Ultimate ASI Loader in a few, and a survey that called every copy of it a
/// third-party hook would be a survey nobody reads. `taarib_tathbeet::wukala`
/// draws the same line over a longer list and for the same reason.
const SLOTAT_MUSHTARAKA: &[&str] = &["xinput1_3.dll", "xinput1_4.dll"];

/// How a product that already owns a graphics slot reaches presentation.
///
/// This is the field the overlay's own prospects turn on, and the three answers
/// are genuinely different outcomes rather than shades of one risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TariqatJar {
    /// It hands the game an object of its own.
    ///
    /// `ReShade`'s proxy returns its own `IDXGISwapChain` and `ENBSeries` its
    /// own `IDirect3DDevice9`. The game's pointer is that C++ class, so its
    /// method table is the product's and not the system's — and the documented
    /// way to find a method table, which is to make a device of your own and
    /// read the pointers out of it, reads the *system* table. Hooking it
    /// succeeds, verifies, and is never called. Nothing crashes and nothing is
    /// drawn.
    Ghilaf,
    /// It patches the system function where it lives.
    ///
    /// The method table still holds the system entry point and the patch sits
    /// behind it, so an overlay that hooks the table runs in front of the patch
    /// and both draw. This is the order that composes.
    Ruqaa,
    /// It replaces the implementation outright.
    ///
    /// DXVK's `d3d9.dll` *is* Direct3D 9 in that process: there is no original
    /// behind it and nothing is hooked. `Direct3DCreate9` resolves to DXVK for
    /// the overlay exactly as it did for the game, so the method table the
    /// overlay reads is the one the game's device uses. One implementation, one
    /// table, and the hook composes.
    Istibdal,
    /// It is not on the presentation path at all.
    LaShay,
}

/// One product's signature, as far as the drawing question needs it.
struct TawqiRusum {
    ism: &'static str,
    ism_arabi: &'static str,
    slotat: &'static [&'static str],
    basmat: &'static [&'static [u8]],
    tariqa: TariqatJar,
}

/// The products that take a graphics slot, and how each reaches presentation.
///
/// Deliberately a second, narrower table than the one in
/// `taarib_tathbeet::wukala`, and the duplication is the lesser evil. This
/// crate is loaded into game processes; depending on the installer to borrow a
/// lookup table would drag discovery, signing and backup machinery into
/// somebody's address space. The two answer different questions — that one asks
/// which mod is installed, this one asks whether the overlay will draw — and
/// this one is limited to the products that can make the answer *no*.
static TAWAQI_RUSUM: &[TawqiRusum] = &[
    TawqiRusum {
        ism: "ReShade",
        ism_arabi: "ReShade",
        slotat: &[
            "d3d9.dll",
            "d3d10.dll",
            "d3d11.dll",
            "d3d12.dll",
            "ddraw.dll",
            "dxgi.dll",
            "opengl32.dll",
        ],
        basmat: &[b"ReShade", b"reshade-shaders"],
        tariqa: TariqatJar::Ghilaf,
    },
    TawqiRusum {
        ism: "ENBSeries",
        ism_arabi: "ENBSeries",
        slotat: &[
            "d3d8.dll",
            "d3d9.dll",
            "d3d11.dll",
            "d3d12.dll",
            "ddraw.dll",
            "dxgi.dll",
        ],
        basmat: &[b"ENBSeries", b"enbseries.ini", b"Boris Vorontsov"],
        tariqa: TariqatJar::Ghilaf,
    },
    TawqiRusum {
        ism: "Special K",
        ism_arabi: "Special K",
        slotat: &[
            "d3d8.dll",
            "d3d9.dll",
            "d3d11.dll",
            "ddraw.dll",
            "dxgi.dll",
            "opengl32.dll",
        ],
        basmat: &[b"SpecialK", b"Special K", b"Kaldaien"],
        // Both, depending on how it was installed, so the worse of the two is
        // what the verdict is written against.
        tariqa: TariqatJar::Ghilaf,
    },
    TawqiRusum {
        ism: "DXVK",
        ism_arabi: "DXVK",
        slotat: &["d3d8.dll", "d3d9.dll", "d3d10.dll", "d3d11.dll", "dxgi.dll"],
        basmat: &[b"DxvkInstance", b"zlib/libpng license"],
        tariqa: TariqatJar::Istibdal,
    },
    TawqiRusum {
        ism: "d3d8to9",
        ism_arabi: "d3d8to9",
        slotat: &["d3d8.dll"],
        basmat: &[b"d3d8to9"],
        tariqa: TariqatJar::Istibdal,
    },
    TawqiRusum {
        ism: "an ASI plugin loader",
        ism_arabi: "مُحمِّل إضافات ASI",
        slotat: &[
            "d3d8.dll",
            "d3d9.dll",
            "d3d11.dll",
            "ddraw.dll",
            "dinput8.dll",
            "dsound.dll",
            "dxgi.dll",
            "winmm.dll",
            "wininet.dll",
            "xinput1_3.dll",
            "xinput1_4.dll",
            SLOT_TAARIB,
        ],
        basmat: &[
            b"Ultimate ASI Loader",
            b"ThirteenAG",
            b"asiloader",
            b"Alexander Blade",
        ],
        tariqa: TariqatJar::LaShay,
    },
    TawqiRusum {
        ism: "BepInEx (Unity Doorstop)",
        ism_arabi: "‏BepInEx (عبر Doorstop)",
        slotat: &["dxgi.dll", "winhttp.dll", SLOT_TAARIB],
        basmat: &[b"doorstop", b"NeighTools"],
        tariqa: TariqatJar::LaShay,
    },
];

/// One occupied proxy slot in a game's directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotMashghul {
    /// The name, which is also the system module it stands in front of.
    pub ism: String,
    /// Where it is.
    pub masar: PathBuf,
    /// How large it is, in bytes.
    ///
    /// Carried because it is the one number that tells somebody at a glance what
    /// they are looking at: a forwarding shim is tens of kilobytes and a mod
    /// framework is megabytes.
    pub hajm: u64,
    /// Whether this is Taarib's own loader rather than a third party's.
    pub taarib: bool,
    /// The product holding it, when marks inside the module name one.
    ///
    /// [`None`] is "nothing in that module identified it", reported as exactly
    /// that. A packed module says nothing, an unreleased build says nothing,
    /// and inventing a name for either would be worse than the gap.
    pub muntaj: Option<&'static str>,
    /// The same, in Arabic.
    pub muntaj_arabi: Option<&'static str>,
    /// How that product reaches presentation, if it is on that path.
    ///
    /// An unidentified module in a graphics slot answers [`TariqatJar::Ghilaf`]
    /// — the pessimistic answer, because a `d3d9.dll` that is not the system's
    /// is a wrapper far more often than it is anything else, and being told the
    /// overlay may not draw and then having it draw is the harmless direction.
    pub tariqa: TariqatJar,
}

/// One proxy slot the survey could not read, and why.
///
/// A present file is a fact even when its bytes are not: the name is taken by
/// *something*, and the survey does not know what. Carried separately from
/// [`SlotMashghul`] because every field that type promises — the size, whether
/// it is Taarib's, the product holding it — is exactly what was not learned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThughraIstitlaa {
    /// The slot's name.
    pub ism: String,
    /// Where it is.
    pub masar: PathBuf,
    /// Its size, when the metadata was readable and only the bytes were not.
    pub hajm: Option<u64>,
    /// What refused, in the filesystem's own words.
    pub sabab: String,
}

/// What Taarib's own proxy slot holds, as far as the survey could see.
///
/// Four states rather than a `bool`, because the fourth is the one a `bool`
/// would have to fold into one of the others — and folding it into "free" is
/// the overwrite this module exists to refuse, while folding it into "taken"
/// asserts a competitor that may not exist. [`TaqrirIstitlaa::slot_mutah`] is
/// the fail-safe fold for a caller that needs only a yes or no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalatSlot {
    /// Nothing is there.
    Hurr,
    /// Taarib's own loader is there, so this is a reinstall.
    Taarib,
    /// Another product's module is there, and Taarib will not write over it.
    Mashghul {
        /// The file, as the survey saw it.
        slot: SlotMashghul,
    },
    /// Something is there and the survey could not read it.
    GhayrMaqru {
        /// What could not be read, and why.
        thughra: ThughraIstitlaa,
    },
}

/// What a game's files say, before anything is installed into them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqrirIstitlaa {
    /// The executable that was read.
    pub luba: PathBuf,
    /// The graphics APIs it links, in this crate's own probe order.
    ///
    /// Suggestive rather than conclusive, exactly as [`WajihatRusum::maktabat`]
    /// says. It orders the search; it does not settle it.
    pub wajihat: Vec<WajihatRusum>,
    /// The graphics modules the import table names.
    pub mustawradat: Vec<String>,
    /// The proxy slots already occupied, in the order this module looks.
    pub mashghula: Vec<SlotMashghul>,
    /// The proxy slots that hold something the survey could not read.
    ///
    /// Never empty because nothing went wrong *and* never empty because nothing
    /// was looked at: every name in [`SLOTAT_WAKEEL`] is either absent,
    /// occupied, or here.
    pub thughrat: Vec<ThughraIstitlaa>,
    /// Every finding, in both languages, with its own verdict.
    pub asbab: Vec<SababQudra>,
}

impl TaqrirIstitlaa {
    /// The worst of the findings, which is the verdict.
    #[must_use]
    pub fn hukm(&self) -> HukmQudra {
        self.asbab
            .iter()
            .map(|sabab| sabab.hukm)
            .max()
            .unwrap_or(HukmQudra::Kamila)
    }

    /// Whether the overlay can be installed into this game at all.
    ///
    /// False for a taken slot and false for an unread one: an installer that
    /// cannot see what it is about to write over does not write.
    #[must_use]
    pub fn yumkin(&self) -> bool {
        self.hukm().qabila()
    }

    /// What Taarib's own proxy slot holds.
    ///
    /// An unread slot answers before an occupied one, because a name that could
    /// not be read is in neither list's terms and must not be described in
    /// them.
    #[must_use]
    pub fn halat_slot(&self) -> HalatSlot {
        if let Some(thughra) = self
            .thughrat
            .iter()
            .find(|thughra| thughra.ism.eq_ignore_ascii_case(SLOT_TAARIB))
        {
            return HalatSlot::GhayrMaqru {
                thughra: thughra.clone(),
            };
        }
        match self
            .mashghula
            .iter()
            .find(|slot| slot.ism.eq_ignore_ascii_case(SLOT_TAARIB))
        {
            None => HalatSlot::Hurr,
            Some(slot) if slot.taarib => HalatSlot::Taarib,
            Some(slot) => HalatSlot::Mashghul { slot: slot.clone() },
        }
    }

    /// Whether Taarib's own proxy slot is free, or already holds Taarib.
    ///
    /// The fail-safe fold of [`TaqrirIstitlaa::halat_slot`]: a slot that could
    /// not be read answers `false` here, the same way a process table that
    /// could not be read answers "treat it as running". A caller that tells the
    /// user the difference between "taken" and "could not be read" — which is
    /// the difference between "remove that product" and "retry in a moment" —
    /// wants the four-state answer by name.
    #[must_use]
    pub fn slot_mutah(&self) -> bool {
        matches!(self.halat_slot(), HalatSlot::Hurr | HalatSlot::Taarib)
    }

    /// The occupied slots that belong to somebody else.
    ///
    /// The distinction matters to a caller and to nobody else: a directory with
    /// Taarib's own `version.dll` in it is a directory Taarib has already been
    /// installed into, which is a reinstall rather than a conflict.
    #[must_use]
    pub fn ghurabaa(&self) -> Vec<&SlotMashghul> {
        self.mashghula.iter().filter(|slot| !slot.taarib).collect()
    }

    /// Whether this is a game the Direct3D 9 backend would answer for.
    #[must_use]
    pub fn tisaa(&self) -> bool {
        self.wajihat.contains(&WajihatRusum::Direct3D9)
    }

    /// The report as the lines a diagnostics bundle carries.
    ///
    /// What could not be read comes before what was found, so a bundle is never
    /// read as clean by somebody who stopped at the first few lines.
    #[must_use]
    pub fn sutur(&self) -> Vec<String> {
        let mut sutur = vec![format!("{}: {}", self.luba.display(), self.hukm())];
        sutur.extend(self.thughrat.iter().map(|thughra| {
            format!(
                "unread: {} at {} — {}",
                thughra.ism,
                thughra.masar.display(),
                thughra.sabab
            )
        }));
        sutur.extend(
            self.asbab
                .iter()
                .map(|sabab| format!("{}: {}", sabab.hukm, sabab.injilizi)),
        );
        sutur
    }
}

/// Reads a game's executable and its directory, and reports what is possible.
///
/// `malaf_luba` is the game's own executable. Its parent directory is what the
/// proxy-slot scan looks at, because that is where a Windows loader resolves a
/// module name from before it reaches `System32`.
///
/// # Errors
///
/// [`KhataTabaqa::KhataMalaf`] when the executable cannot be read,
/// [`KhataTabaqa::HajmMufrit`] when it is past [`AQSA_MALAF`], and
/// [`KhataTabaqa::MalafGhayrMafhum`] when it is not a Portable Executable this
/// walk understands. All three are refusals rather than empty reports: a report
/// that said "no graphics API" about a file it could not parse would be a report
/// that looked like an answer.
pub fn istatli(malaf_luba: &Path) -> Result<TaqrirIstitlaa, KhataTabaqa> {
    let bayt = iqra_mahdud(malaf_luba)?;
    let mustawradat = mustawradat_pe(malaf_luba, &bayt)?;

    let mut wajihat = Vec::new();
    let mut asmaa: Vec<String> = Vec::new();
    for wajiha in WajihatRusum::jamee() {
        let mut wujidat = false;
        for maktaba in wajiha.maktabat() {
            if mustawradat
                .iter()
                .any(|ism| ism.eq_ignore_ascii_case(maktaba))
            {
                if !asmaa.iter().any(|ism| ism.eq_ignore_ascii_case(maktaba)) {
                    asmaa.push((*maktaba).to_owned());
                }
                wujidat = true;
            }
        }
        if wujidat {
            wajihat.push(wajiha);
        }
    }

    let mujallad = malaf_luba.parent().unwrap_or_else(|| Path::new("."));
    let (mashghula, thughrat) = slotat_mashghula(mujallad);
    let mut asbab = Vec::new();

    if wajihat.is_empty() {
        asbab.push(SababQudra::naqisa(
            "لا يستورد هذا الملف التنفيذي أيًّا من المكتبات الرسومية التي تعرفها الطبقة. قد \
             تُحمَّل مكتبة العرض أثناء التشغيل، فهذا ليس حكمًا نهائيًا، لكن لا شيء في الملفات \
             يؤكد أن الطبقة ستصل.",
            "this executable imports none of the graphics modules the overlay knows. A renderer \
             loaded at run time imports nothing either, so this is not final — but nothing in \
             the files says the overlay will reach this game.",
        ));
    } else {
        asbab.push(SababQudra::kamila(
            format!("يستورد الملف التنفيذي {}.", asmaa.join("، ")),
            format!(
                "this executable imports {}, so the overlay would try {} in that order",
                asmaa.join(", "),
                wajihat
                    .iter()
                    .map(|wajiha| wajiha.ism().to_owned())
                    .collect::<Vec<_>>()
                    .join(" then ")
            ),
        ));
    }

    if wajihat.contains(&WajihatRusum::Direct3D9) {
        asbab.push(SababQudra::kamila(
            "تُركَّب الطبقة على جهاز اللعبة نفسه وترسم داخل الإطار الذي توشك على عرضه، فوضع \
             ملء الشاشة الحصري يُعامَل كوضع النافذة. ما يتغيّر في ملء الشاشة هو فقدان الجهاز: \
             جهاز Direct3D 9 العادي يُفقد عند كل تبديل نافذة، ولا يُعرف أعادي هو أم ممتد إلا \
             بعد تشغيل اللعبة.",
            "the overlay attaches to the game's own device and draws into the frame it is about \
             to present, so exclusive fullscreen is composed over exactly as windowed mode is. \
             What exclusive fullscreen changes is device loss: a plain Direct3D 9 device is lost \
             on every alt-tab, and whether this game makes a plain or an extended device is only \
             knowable once it is running.",
        ));
    }

    for thughra in &thughrat {
        asbab.push(sabab_thughra(thughra));
    }
    for slot in &mashghula {
        asbab.push(sabab_slot(slot));
    }
    // The all-clear is a claim about every name, and it is only true when every
    // name was actually read. A gap beside it would make this sentence the
    // "empty result reads as clean" the gap list exists to prevent.
    if mashghula.is_empty() && thughrat.is_empty() {
        asbab.push(SababQudra::kamila(
            "لا يشغل أي منتج آخر أسماء التحميل المجاورة لهذا الملف التنفيذي.",
            "none of the loader names beside this executable are taken by another product",
        ));
    }

    Ok(TaqrirIstitlaa {
        luba: malaf_luba.to_path_buf(),
        wajihat,
        mustawradat: asmaa,
        mashghula,
        thughrat,
        asbab,
    })
}

/// The finding one unread slot produces.
///
/// Reported as the worst the slot could be, where that worst is a limit, and as
/// not determined where it is a refusal — see [`crate::qudra`]'s header. Nothing
/// in a slot Taarib does not use can stop an installation: a graphics name is
/// at worst a wrapper on the presentation path, any other name is at worst a
/// hook live in the process, and both are what an *identified* module in that
/// slot already produces. Taarib's own slot is different, because the worst it
/// could hold is another product, and the survey will neither claim that nor
/// deny it about bytes it did not read.
fn sabab_thughra(thughra: &ThughraIstitlaa) -> SababQudra {
    let hajm = thughra
        .hajm
        .map_or_else(String::new, |hajm| format!(", {hajm} bytes"));
    let hajm_arabi = thughra
        .hajm
        .map_or_else(String::new, |hajm| format!("، {hajm} بايت"));
    if thughra.ism.eq_ignore_ascii_case(SLOT_TAARIB) {
        return SababQudra::majhula(
            format!(
                "الاسم {} يحمل ملفًا لم يتمكّن الاستطلاع من قراءته ({}{hajm_arabi}). هذا هو \
                 الاسم الوحيد الذي يستخدمه تعريب، ولا يكتب تعريب فوق ملف لم يقرأه: قد يكون \
                 مُحمّل تعريب نفسه محجوزًا لحظةً، وقد يكون منتجًا آخر. أعد المحاولة بعد زوال \
                 السبب.",
                thughra.ism, thughra.sabab
            ),
            format!(
                "the {} slot holds a file this survey could not read ({}{hajm}, at {}). This is \
                 the one name Taarib uses, and Taarib does not write over a file it could not \
                 read: it may be Taarib's own loader held open for a moment, and it may be \
                 another product. Retry once the cause is gone; whether the slot is free is \
                 not known until then.",
                thughra.ism,
                thughra.sabab,
                thughra.masar.display()
            ),
        );
    }
    if SLOTAT_RUSUM
        .iter()
        .any(|slot| slot.eq_ignore_ascii_case(&thughra.ism))
    {
        return SababQudra::naqisa(
            format!(
                "الاسم {} يحمل ملفًا لم يتمكّن الاستطلاع من قراءته ({}{hajm_arabi})، وهو على \
                 مسار العرض. أسوأ ما قد يكون هو غلاف يعترض جهاز اللعبة — فتُركَّب الطبقة ولا \
                 تُستدعى — وعلى ذلك يُحكم. لا يستخدم تعريب هذا الاسم ولا يمسّه.",
                thughra.ism, thughra.sabab
            ),
            format!(
                "the {} slot holds a file this survey could not read ({}{hajm}, at {}), and it \
                 is on the presentation path. The worst it can be is a wrapper around the \
                 game's device — the hook installs and is never called — so that is what it is \
                 treated as. Taarib does not use this slot and leaves it alone.",
                thughra.ism,
                thughra.sabab,
                thughra.masar.display()
            ),
        );
    }
    SababQudra::kamila(
        format!(
            "الاسم {} يحمل ملفًا لم يتمكّن الاستطلاع من قراءته ({}{hajm_arabi}). لا يستخدم \
             تعريب هذا الاسم ولا يمسّه؛ أسوأ ما قد يكون هو خطّاف من طرف ثالث يعمل داخل اللعبة، \
             ولا يفكّ تعريب خطّاف غيره.",
            thughra.ism, thughra.sabab
        ),
        format!(
            "the {} slot holds a file this survey could not read ({}{hajm}, at {}). Taarib does \
             not use this slot and leaves it alone; the worst it can be is a third-party hook \
             live in the process, which is chained onto rather than refused.",
            thughra.ism,
            thughra.sabab,
            thughra.masar.display()
        ),
    )
}

/// The finding one occupied slot produces.
///
/// Three cases and only one of them stops an installation. Taarib's own module
/// in Taarib's own slot is a reinstall. Another product in a slot Taarib does
/// not use is worth reporting — it means a third-party hook will be live in the
/// process — and costs nothing. Another product in Taarib's slot is the refusal
/// this module exists for.
fn sabab_slot(slot: &SlotMashghul) -> SababQudra {
    if slot.taarib {
        return SababQudra::kamila(
            format!(
                "الاسم {} يحمل مُحمّل تعريب نفسه ({} بايت)، فهذه إعادة تركيب لا تعارض.",
                slot.ism, slot.hajm
            ),
            format!(
                "the {} slot already holds Taarib's own loader ({} bytes), so this is a \
                 reinstall rather than a conflict",
                slot.ism, slot.hajm
            ),
        );
    }
    if slot.ism.eq_ignore_ascii_case(SLOT_TAARIB) {
        return SababQudra::mustaheela(
            format!(
                "الاسم {} يشغله منتج آخر ({} بايت). هذا هو الاسم الوحيد الذي يستخدمه تعريب، \
                 ولا يأخذ تعريب اسمًا شغله غيره: الكتابة فوقه تحذف ذلك المنتج، ولا توجد نسخة \
                 ثانية من الاسم تُكتب بجانبه.",
                slot.ism, slot.hajm
            ),
            format!(
                "the {} slot is taken by another product ({} bytes at {}). This is the one name \
                 Taarib uses, and Taarib does not take a slot another product has taken — \
                 overwriting it would delete that product, and there is no second copy of the \
                 name to write beside it. Removing that product, or asking its author for a \
                 chain-loading name, is what changes this answer.",
                slot.ism,
                slot.hajm,
                slot.masar.display()
            ),
        );
    }
    if slot.tariqa == TariqatJar::Ghilaf {
        return sabab_ghilaf(slot);
    }
    let man = slot
        .muntaj
        .map_or_else(String::new, |ism| format!(", which is {ism},"));
    let man_arabi = slot
        .muntaj_arabi
        .map_or_else(String::new, |ism| format!("، وهو {ism}،"));
    SababQudra::kamila(
        format!(
            "الاسم {} يشغله منتج آخر ({} بايت){man_arabi} ولا يستخدمه تعريب ولا يمسّه. يعني ذلك \
             أن خطّافًا من طرف ثالث يعمل داخل اللعبة، وقد تجد الطبقة دالة العرض مستبدلة قبلها.",
            slot.ism, slot.hajm
        ),
        format!(
            "the {} slot is taken by another product ({} bytes at {}){man} Taarib does not use \
             this slot and leaves it alone. It does mean a third-party hook is live in this \
             process, so the overlay may find the presentation method already replaced — which \
             is chained onto rather than refused, and which is why Taarib never restores a \
             function pointer it did not install.",
            slot.ism,
            slot.hajm,
            slot.masar.display()
        ),
    )
}

/// The finding a wrapper on the presentation path produces.
///
/// This is the one collision that is not a crash and not a refusal, and it is
/// the reason the whole survey exists in this crate rather than only in the
/// installer.
///
/// The overlay finds a method table the documented way: create a device and a
/// swap chain of its own against a hidden window, read the pointers out, and
/// destroy them. Those pointers belong to the *class*, so they are the same
/// ones the game's own swap chain uses — which is true right up until something
/// hands the game an object of a different class. `ReShade`'s proxy returns its
/// own `IDXGISwapChain`; `ENBSeries` returns its own `IDirect3DDevice9`. The
/// overlay's hook is then installed in a table nothing calls: it succeeds, it
/// verifies, the game runs, and nothing is drawn.
///
/// The order is what decides it, and the order is not the overlay's to choose.
/// A product in a proxy slot is loaded by the Windows loader before the game's
/// first instruction; the overlay attaches afterwards. So Taarib is always the
/// later hook here, and the later hook is the one that loses when the earlier
/// one wraps. This is also why it is [`HukmQudra::Naqisa`] and not
/// [`HukmQudra::Mustaheela`]: for a *patching* product — Special K's global
/// injector, `re4_tweaks` — the later hook is the outer one and both draw. Only
/// wrapping breaks it, and which of the two a given build does is not knowable
/// from the file.
///
/// What it is emphatically **not** is a reason to unhook anything. The rule in
/// [`crate::khataf::Khataf::fukk`] holds unchanged: Taarib never restores a
/// function pointer it did not install. An overlay that responded to "my hook
/// is not being called" by writing over somebody else's would be the failure
/// this product is built not to have.
fn sabab_ghilaf(slot: &SlotMashghul) -> SababQudra {
    let man = slot.muntaj.unwrap_or("an unidentified product");
    let man_arabi = slot.muntaj_arabi.unwrap_or("منتج غير معروف");
    SababQudra::naqisa(
        format!(
            "الاسم {} يشغله {man_arabi} ({} بايت)، وهو على مسار العرض نفسه. تُحمَّل هذه \
             المنتجات قبل أول تعليمة في اللعبة، فتعريب دائمًا الخطّاف المتأخّر — وإن كان \
             المنتج يغلّف جهاز اللعبة بكائن من صنعه، فجدول الدوال الذي تقرؤه الطبقة ليس \
             الجدول الذي تناديه اللعبة، فيُركَّب الخطّاف ولا يُستدعى: لا تعطّل ولا رسم. \
             وإن كان يرقّع الدالة في مكانها فكلاهما يرسم. ولا يُعرف أيّهما من الملف. لا \
             يفكّ تعريب خطّاف غيره في الحالتين.",
            slot.ism, slot.hajm
        ),
        format!(
            "the {} slot is taken by {man} ({} bytes at {}), which is on the presentation path \
             the overlay uses. A product in a proxy slot is loaded before the game's first \
             instruction, so Taarib is always the later hook here. If that product wraps the \
             game's device in an object of its own — ReShade and ENBSeries both do — then the \
             method table the overlay reads is not the one the game calls, and the hook installs, \
             verifies, and is never called: no crash, no Arabic. If it patches the function in \
             place instead, both draw. Which of the two a given build does is not knowable from \
             the file, so the overlay is offered and this is said first. Taarib does not unhook \
             the other product in either case.",
            slot.ism,
            slot.hajm,
            slot.masar.display()
        ),
    )
}

/// The occupied proxy slots in one directory, and the ones that would not be
/// read.
///
/// Each name is asked two questions — is something there, and what is it — and
/// a refusal to either is a gap, never a "no". Only [`std::io::ErrorKind::NotFound`]
/// means the name is free; a permission refusal, a symlink loop, a sharing
/// violation from a scanner holding the file, or a file past [`AQSA_MALAF`] each
/// mean the name is *taken by something this survey did not see*. The directory
/// itself is not listed, so an untraversable directory reaches this as a gap
/// per name rather than as an empty answer — and the executable read, which
/// happens first, has already refused for that case.
fn slotat_mashghula(mujallad: &Path) -> (Vec<SlotMashghul>, Vec<ThughraIstitlaa>) {
    let mut mashghula = Vec::new();
    let mut thughrat = Vec::new();
    for ism in SLOTAT_WAKEEL.iter().chain(SLOTAT_MUSHTARAKA.iter()) {
        let masar = mujallad.join(ism);
        let bayan = match std::fs::metadata(&masar) {
            Ok(bayan) => bayan,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => continue,
            Err(sabab) => {
                thughrat.push(ThughraIstitlaa {
                    ism: (*ism).to_owned(),
                    masar,
                    hajm: None,
                    sabab: sabab.to_string(),
                });
                continue;
            },
        };
        if !bayan.is_file() {
            continue;
        }
        let bayt = match iqra_mahdud(&masar) {
            Ok(bayt) => bayt,
            Err(khata) => {
                thughrat.push(ThughraIstitlaa {
                    ism: (*ism).to_owned(),
                    masar,
                    hajm: Some(bayan.len()),
                    sabab: khata.to_string(),
                });
                continue;
            },
        };
        let taarib = fihi(&bayt, BASMAT_MUDKHAL);
        let tawqi = if taarib {
            None
        } else {
            tawqi_min_bayt(ism, &bayt)
        };
        // A name a game may legitimately ship is Microsoft's own file until
        // something inside it says otherwise, and reporting it unidentified
        // would be the survey crying wolf over a redistributable.
        if tawqi.is_none() && SLOTAT_MUSHTARAKA.contains(ism) {
            continue;
        }
        let rusum = SLOTAT_RUSUM.contains(ism);
        mashghula.push(SlotMashghul {
            ism: (*ism).to_owned(),
            masar: masar.clone(),
            hajm: bayan.len(),
            taarib,
            muntaj: tawqi.map(|tawqi| tawqi.ism),
            muntaj_arabi: tawqi.map(|tawqi| tawqi.ism_arabi),
            tariqa: match (tawqi, taarib, rusum) {
                (Some(tawqi), _, _) => tawqi.tariqa,
                // Taarib's own loader is not a wrapper, and a slot that is not
                // a graphics module is not on the presentation path at all.
                (None, true, _) | (None, false, false) => TariqatJar::LaShay,
                (None, false, true) => TariqatJar::Ghilaf,
            },
        });
    }
    (mashghula, thughrat)
}

/// The product a module's own bytes name, if any.
///
/// Marks are matched in ASCII and in the UTF-16 a Windows version resource
/// stores its strings in — which is where the string that identifies `ReShade`
/// or Special K actually lives — and case-insensitively, because Alexander
/// Blade's ASI loader spells itself `asiloader` where the ecosystem writes
/// `ASI Loader`.
///
/// The slot is a guard rather than a hint. A `re4_tweaks` proxy contains the
/// string `DXVK` because it reports whether DXVK is in use, and DXVK is never
/// installed as `dinput8`.
fn tawqi_min_bayt(ism: &str, bayt: &[u8]) -> Option<&'static TawqiRusum> {
    let mut wajid: Option<&'static TawqiRusum> = None;
    for tawqi in TAWAQI_RUSUM {
        if !tawqi
            .slotat
            .iter()
            .any(|slot| slot.eq_ignore_ascii_case(ism))
        {
            continue;
        }
        if !tawqi.basmat.iter().any(|basma| fihi(bayt, basma)) {
            continue;
        }
        // Two products' marks in one module and nothing to separate them —
        // Special K ships ReShade integration and carries its name — so the
        // module is left unnamed rather than assigned to whichever came first
        // in a table.
        if wajid.is_some() {
            return None;
        }
        wajid = Some(tawqi);
    }
    wajid
}

/// Whether a mark appears in a buffer, as ASCII or as UTF-16, ignoring case.
fn fihi(kawm: &[u8], basma: &[u8]) -> bool {
    if basma.is_empty() {
        return false;
    }
    if kawm.len() >= basma.len()
        && kawm
            .windows(basma.len())
            .any(|nafidha| nafidha.eq_ignore_ascii_case(basma))
    {
        return true;
    }
    let tul = basma.len().saturating_mul(2);
    if kawm.len() < tul {
        return false;
    }
    kawm.windows(tul).any(|nafidha| {
        basma.iter().enumerate().all(|(fahras, harf)| {
            let mawdi = fahras.saturating_mul(2);
            nafidha
                .get(mawdi)
                .is_some_and(|bayt| bayt.eq_ignore_ascii_case(harf))
                && nafidha.get(mawdi.saturating_add(1)) == Some(&0)
        })
    })
}

// `huwa_mudkhal` — "is this file Taarib's own loader?" — used to live here and
// read the file a second time to answer the same question. `slotat_mashghula`
// now reads each module once and asks every question of the one buffer, which
// matters because that buffer can be eleven megabytes and there are seventeen
// names to try. Its unreadable-file answer used to survive in the caller as
// "treated as a third party's" — the safe direction for the write, and a false
// sentence about the file, since an unread `version.dll` may be Taarib's own
// loader held open by a scanner. It is now a `ThughraIstitlaa`, which refuses
// the write for the same reason and says what actually happened.

/// Reads a file, refusing one larger than [`AQSA_MALAF`].
///
/// # Errors
///
/// [`KhataTabaqa::HajmMufrit`] for a file past the cap, and
/// [`KhataTabaqa::KhataMalaf`] for anything the filesystem refuses.
fn iqra_mahdud(masar: &Path) -> Result<Vec<u8>, KhataTabaqa> {
    let bayan = std::fs::metadata(masar).map_err(|sabab| KhataTabaqa::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    if bayan.len() > AQSA_MALAF {
        return Err(KhataTabaqa::HajmMufrit {
            haql: "the file this survey would read",
            qeema: bayan.len(),
            saqf: AQSA_MALAF,
        });
    }
    std::fs::read(masar).map_err(|sabab| KhataTabaqa::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

// ---------------------------------------------------------------------------
// The import table, walked by hand
// ---------------------------------------------------------------------------

/// One section's mapping from relative virtual addresses to file offsets.
#[derive(Debug, Clone, Copy)]
struct Qism {
    /// Where the section starts in the address space of the loaded image.
    oinwan: u32,
    /// How many bytes of that address space it covers.
    madaa: u32,
    /// Where the same bytes start in the file.
    izaha: u32,
}

/// The module names in a Portable Executable's import directory.
///
/// See this module's header for what this deliberately does not read. Every
/// access goes through [`slice::get`], every offset saturates, and both loops
/// are capped — so a malformed or hostile executable produces a short list or a
/// refusal, never a read outside the buffer and never a loop that does not end.
///
/// # Errors
///
/// [`KhataTabaqa::MalafGhayrMafhum`] when the file is not a PE, when its headers
/// point outside itself, or when its import directory is not inside any section.
/// A file with no import directory at all is **not** an error — that is a valid
/// executable with no imports, and it answers with an empty list.
fn mustawradat_pe(masar: &Path, bayt: &[u8]) -> Result<Vec<String>, KhataTabaqa> {
    let ghayr = |sabab: String| KhataTabaqa::MalafGhayrMafhum {
        masar: masar.to_path_buf(),
        sigha: "Portable Executable",
        sabab,
    };

    if iqra_u16(bayt, 0).ok_or_else(|| ghayr("the file is empty".to_owned()))? != 0x5A4D {
        return Err(ghayr(
            "the file does not begin with the MZ signature".to_owned(),
        ));
    }
    let bidayat_pe = usize::try_from(
        iqra_u32(bayt, 0x3C).ok_or_else(|| ghayr("the DOS header is truncated".to_owned()))?,
    )
    .unwrap_or(usize::MAX);
    if iqra_u32(bayt, bidayat_pe)
        .ok_or_else(|| ghayr("e_lfanew points past the file".to_owned()))?
        != 0x0000_4550
    {
        return Err(ghayr(
            "e_lfanew does not point at a PE signature".to_owned(),
        ));
    }

    // The COFF header is twenty bytes and the optional header follows it.
    let bidayat_coff = bidayat_pe.saturating_add(4);
    let adad_aqsam = usize::from(
        iqra_u16(bayt, bidayat_coff.saturating_add(2))
            .ok_or_else(|| ghayr("the COFF header is truncated".to_owned()))?,
    );
    let hajm_ikhtiyari = usize::from(
        iqra_u16(bayt, bidayat_coff.saturating_add(16))
            .ok_or_else(|| ghayr("the COFF header is truncated".to_owned()))?,
    );
    let bidayat_ikhtiyari = bidayat_coff.saturating_add(20);

    // `0x10b` is PE32 and `0x20b` is PE32+, and the two differ by sixteen bytes
    // of wider fields before the data directories — which is the only thing this
    // walk cares about. It is derived from the magic rather than from the
    // machine type, which would be right for every file anybody has and wrong
    // in principle.
    let izahat_adilla = match iqra_u16(bayt, bidayat_ikhtiyari)
        .ok_or_else(|| ghayr("the optional header is truncated".to_owned()))?
    {
        0x010B => 96,
        0x020B => 112,
        akhar => {
            return Err(ghayr(format!(
                "the optional header's magic is {akhar:#06x}"
            )));
        },
    };

    // Data directory entry one is the import table: eight bytes in, then its
    // relative virtual address.
    let bidayat_ustuwana = bidayat_ikhtiyari
        .saturating_add(izahat_adilla)
        .saturating_add(8);
    let Some(oinwan_ustuwana) = iqra_u32(bayt, bidayat_ustuwana) else {
        return Err(ghayr("the data directories are truncated".to_owned()));
    };
    if oinwan_ustuwana == 0 {
        // A valid executable with no imports. Every game has some, but a file
        // that has none is not malformed and must not be reported as such.
        return Ok(Vec::new());
    }

    let aqsam = aqsam_pe(
        bayt,
        bidayat_ikhtiyari.saturating_add(hajm_ikhtiyari),
        adad_aqsam,
    );
    let Some(mut mawdi) = izaha_min_oinwan(&aqsam, oinwan_ustuwana) else {
        return Err(ghayr(
            "the import directory's address is not inside any section".to_owned(),
        ));
    };

    let mut asmaa = Vec::new();
    for _ in 0..AQSA_MUSTAWRADAT {
        // An import descriptor is twenty bytes and the table ends at an
        // all-zero one. The module name's address is at offset twelve.
        let Some(oinwan_ism) = iqra_u32(bayt, mawdi.saturating_add(12)) else {
            break;
        };
        let asli = iqra_u32(bayt, mawdi).unwrap_or(0);
        let awwal = iqra_u32(bayt, mawdi.saturating_add(16)).unwrap_or(0);
        if oinwan_ism == 0 && asli == 0 && awwal == 0 {
            break;
        }
        if let Some(izaha) = izaha_min_oinwan(&aqsam, oinwan_ism)
            && let Some(ism) = nass_ascii(bayt, izaha)
        {
            asmaa.push(ism);
        }
        mawdi = mawdi.saturating_add(20);
    }
    Ok(asmaa)
}

/// The section table, as the mapping the import walk needs.
///
/// A section whose header is truncated ends the list rather than failing the
/// parse: the sections before it are still a valid mapping, and an import
/// directory that lands in one of them is still findable.
fn aqsam_pe(bayt: &[u8], bidaya: usize, adad: usize) -> Vec<Qism> {
    let mut aqsam = Vec::new();
    // Forty bytes per section header, capped by the count the COFF header
    // declares — which is itself a `u16`, so this cannot run away.
    for fahras in 0..adad {
        let mawdi = bidaya.saturating_add(fahras.saturating_mul(40));
        let (Some(madaa_wahmi), Some(oinwan), Some(madaa_khaam), Some(izaha)) = (
            iqra_u32(bayt, mawdi.saturating_add(8)),
            iqra_u32(bayt, mawdi.saturating_add(12)),
            iqra_u32(bayt, mawdi.saturating_add(16)),
            iqra_u32(bayt, mawdi.saturating_add(20)),
        ) else {
            break;
        };
        // The larger of the two spans. A section's virtual size is often larger
        // than its raw size — the loader zero-fills the tail — and a mapping
        // built from the raw size alone would miss an import directory a linker
        // placed in a section's padded region.
        aqsam.push(Qism {
            oinwan,
            madaa: madaa_wahmi.max(madaa_khaam),
            izaha,
        });
    }
    aqsam
}

/// A relative virtual address as an offset into the file's bytes.
///
/// [`None`] when the address is in no section, which for a well-formed
/// executable does not happen and for a crafted one is what stops the walk.
fn izaha_min_oinwan(aqsam: &[Qism], oinwan: u32) -> Option<usize> {
    aqsam
        .iter()
        .find(|qism| oinwan >= qism.oinwan && oinwan < qism.oinwan.saturating_add(qism.madaa))
        .and_then(|qism| {
            usize::try_from(
                qism.izaha
                    .saturating_add(oinwan.saturating_sub(qism.oinwan)),
            )
            .ok()
        })
}

/// A NUL-terminated ASCII string at an offset, capped at [`AQSA_ISM`].
///
/// [`None`] when the offset is past the file or the string is not terminated
/// inside the cap. An imported module's name is a handful of characters, so a
/// name that runs to the cap is a malformed table rather than a long name.
fn nass_ascii(bayt: &[u8], izaha: usize) -> Option<String> {
    let baqi = bayt.get(izaha..)?;
    let tul = baqi.iter().take(AQSA_ISM).position(|harf| *harf == 0)?;
    let khaam = baqi.get(..tul)?;
    if khaam.is_empty() || !khaam.is_ascii() {
        return None;
    }
    Some(String::from_utf8_lossy(khaam).into_owned())
}

/// A little-endian `u16` at an offset, or [`None`] past the end.
fn iqra_u16(bayt: &[u8], izaha: usize) -> Option<u16> {
    let khaam: [u8; 2] = bayt.get(izaha..izaha.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(khaam))
}

/// A little-endian `u32` at an offset, or [`None`] past the end.
fn iqra_u32(bayt: &[u8], izaha: usize) -> Option<u32> {
    let khaam: [u8; 4] = bayt.get(izaha..izaha.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(khaam))
}
