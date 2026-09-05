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
/// loaded into a process that may be a 32-bit game.
const AQSA_MALAF: u64 = 32 * 1024 * 1024;

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
    "xinput1_3.dll",
    "xinput1_4.dll",
    "winmm.dll",
    "winhttp.dll",
    "wininet.dll",
    "dbghelp.dll",
    SLOT_TAARIB,
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
    /// Every finding, in both languages, with its own verdict.
    pub asbab: Vec<SababQudra>,
}

impl TaqrirIstitlaa {
    /// The worst of the findings, which is the verdict.
    #[must_use]
    pub fn hukm(&self) -> HukmQudra {
        self.asbab.iter().map(|sabab| sabab.hukm).max().unwrap_or(HukmQudra::Kamila)
    }

    /// Whether the overlay can be installed into this game at all.
    #[must_use]
    pub fn yumkin(&self) -> bool {
        self.hukm().qabila()
    }

    /// Whether Taarib's own proxy slot is free, or already holds Taarib.
    #[must_use]
    pub fn slot_mutah(&self) -> bool {
        self.mashghula
            .iter()
            .find(|slot| slot.ism.eq_ignore_ascii_case(SLOT_TAARIB))
            .is_none_or(|slot| slot.taarib)
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
    #[must_use]
    pub fn sutur(&self) -> Vec<String> {
        let mut sutur = vec![format!("{}: {}", self.luba.display(), self.hukm())];
        sutur.extend(
            self.asbab.iter().map(|sabab| format!("{}: {}", sabab.hukm, sabab.injilizi)),
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
            if mustawradat.iter().any(|ism| ism.eq_ignore_ascii_case(maktaba)) {
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
    let mashghula = slotat_mashghula(mujallad);
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

    for slot in &mashghula {
        asbab.push(sabab_slot(slot));
    }
    if mashghula.is_empty() {
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
        asbab,
    })
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
    SababQudra::kamila(
        format!(
            "الاسم {} يشغله منتج آخر ({} بايت)، ولا يستخدمه تعريب ولا يمسّه. يعني ذلك أن خطّافًا \
             من طرف ثالث يعمل داخل اللعبة، وقد تجد الطبقة دالة العرض مستبدلة قبلها.",
            slot.ism, slot.hajm
        ),
        format!(
            "the {} slot is taken by another product ({} bytes at {}). Taarib does not use this \
             slot and leaves it alone. It does mean a third-party hook is live in this process, \
             so the overlay may find the presentation method already replaced — which is chained \
             onto rather than refused, and which is why Taarib never restores a function pointer \
             it did not install.",
            slot.ism,
            slot.hajm,
            slot.masar.display()
        ),
    )
}

/// The occupied proxy slots in one directory.
///
/// A directory that cannot be listed answers "none". This runs against a game
/// directory the caller pointed at, and a permission error there is not a fact
/// about proxies; the caller learns about it from the executable read, which
/// happens first and does report its own failure.
fn slotat_mashghula(mujallad: &Path) -> Vec<SlotMashghul> {
    let mut mashghula = Vec::new();
    for ism in SLOTAT_WAKEEL {
        let masar = mujallad.join(ism);
        let Ok(bayan) = std::fs::metadata(&masar) else {
            continue;
        };
        if !bayan.is_file() {
            continue;
        }
        mashghula.push(SlotMashghul {
            ism: (*ism).to_owned(),
            masar: masar.clone(),
            hajm: bayan.len(),
            taarib: huwa_mudkhal(&masar),
        });
    }
    mashghula
}

/// Whether a file on disk is Taarib's own loader.
///
/// A file too large to read under the cap answers `false`, which is the safe
/// direction: treating an unknown module as a third party's means declining to
/// overwrite it.
fn huwa_mudkhal(masar: &Path) -> bool {
    let Ok(bayt) = iqra_mahdud(masar) else {
        return false;
    };
    bayt.windows(BASMAT_MUDKHAL.len()).any(|nafidha| nafidha == BASMAT_MUDKHAL)
}

/// Reads a file, refusing one larger than [`AQSA_MALAF`].
///
/// # Errors
///
/// [`KhataTabaqa::HajmMufrit`] for a file past the cap, and
/// [`KhataTabaqa::KhataMalaf`] for anything the filesystem refuses.
fn iqra_mahdud(masar: &Path) -> Result<Vec<u8>, KhataTabaqa> {
    let bayan = std::fs::metadata(masar)
        .map_err(|sabab| KhataTabaqa::KhataMalaf { masar: masar.to_path_buf(), sabab })?;
    if bayan.len() > AQSA_MALAF {
        return Err(KhataTabaqa::HajmMufrit {
            haql: "the file this survey would read",
            qeema: bayan.len(),
            saqf: AQSA_MALAF,
        });
    }
    std::fs::read(masar)
        .map_err(|sabab| KhataTabaqa::KhataMalaf { masar: masar.to_path_buf(), sabab })
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
        return Err(ghayr("the file does not begin with the MZ signature".to_owned()));
    }
    let bidayat_pe = usize::try_from(
        iqra_u32(bayt, 0x3C).ok_or_else(|| ghayr("the DOS header is truncated".to_owned()))?,
    )
    .unwrap_or(usize::MAX);
    if iqra_u32(bayt, bidayat_pe).ok_or_else(|| ghayr("e_lfanew points past the file".to_owned()))?
        != 0x0000_4550
    {
        return Err(ghayr("e_lfanew does not point at a PE signature".to_owned()));
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
        akhar => return Err(ghayr(format!("the optional header's magic is {akhar:#06x}"))),
    };

    // Data directory entry one is the import table: eight bytes in, then its
    // relative virtual address.
    let bidayat_ustuwana = bidayat_ikhtiyari.saturating_add(izahat_adilla).saturating_add(8);
    let Some(oinwan_ustuwana) = iqra_u32(bayt, bidayat_ustuwana) else {
        return Err(ghayr("the data directories are truncated".to_owned()));
    };
    if oinwan_ustuwana == 0 {
        // A valid executable with no imports. Every game has some, but a file
        // that has none is not malformed and must not be reported as such.
        return Ok(Vec::new());
    }

    let aqsam = aqsam_pe(bayt, bidayat_ikhtiyari.saturating_add(hajm_ikhtiyari), adad_aqsam);
    let Some(mut mawdi) = izaha_min_oinwan(&aqsam, oinwan_ustuwana) else {
        return Err(ghayr("the import directory's address is not inside any section".to_owned()));
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
        aqsam.push(Qism { oinwan, madaa: madaa_wahmi.max(madaa_khaam), izaha });
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
        .find(|qism| {
            oinwan >= qism.oinwan && oinwan < qism.oinwan.saturating_add(qism.madaa)
        })
        .and_then(|qism| {
            usize::try_from(qism.izaha.saturating_add(oinwan.saturating_sub(qism.oinwan))).ok()
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
