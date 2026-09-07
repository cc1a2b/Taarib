//! الأيقونة — the picture an executable carries inside itself, parsed out of the
//! file's own bytes on whatever host happens to be running.
//!
//! Rung 3 of [`crate::silsila`] exists for the game no artwork service has ever
//! indexed, and the one image such a game is guaranteed to own is the icon its
//! executable ships. This module reads that icon: Windows resource directories,
//! macOS `.icns` bundles, and the freedesktop icon theme a Linux game points at.
//!
//! ## Why not one line of this calls the operating system
//!
//! The obvious implementation on Windows is `PrivateExtractIcons`, and on macOS
//! `NSWorkspace.icon(forFile:)`. Both are wrong here, for the same reason:
//! **the host and the binary do not have to agree.** On Linux the majority of a
//! Steam library is Windows `.exe` files living inside Proton prefixes, and
//! there is no Win32 to ask. On Windows a user's Heroic library can hold Linux
//! ELF builds. A macOS `.app` copied onto a NAS is read from both. An extractor
//! that dispatched on `cfg!(target_os)` would find nothing for exactly the games
//! that need rung 3 most — the ones no storefront has artwork for.
//!
//! So [`ayqunat`] dispatches on the **file's own magic bytes**. A PE is a PE
//! whichever machine is looking at it, and every parse below is pure arithmetic
//! over a byte slice.
//!
//! ## What each platform actually stores, and where
//!
//! | container | where the icon is | what has to be parsed |
//! | --- | --- | --- |
//! | PE (`.exe`, `.dll`) | the `.rsrc` section | a three-level resource tree, then `RT_GROUP_ICON` reassembled with its `RT_ICON` members into a real `.ico` |
//! | Mach-O (`.app`) | **not in the binary** — `Contents/Resources/*.icns` | `Info.plist` for `CFBundleIconFile`, then the ICNS chunk table |
//! | ELF | **not in the binary at all** | a `.desktop` file's `Icon=` key resolved against the icon theme directories |
//!
//! The ELF row is not an omission. ELF has no resource section and no
//! convention for embedding an application icon; a Linux desktop application's
//! icon lives in `/usr/share/icons` or beside the binary, and the `.desktop`
//! file is the only thing that connects the two. Pretending otherwise would
//! mean silently returning nothing for every native Linux game.
//!
//! ## Why the PE path reassembles a `.ico` instead of decoding `RT_ICON`
//!
//! An `RT_ICON` resource body is a bare DIB with no `ICONDIRENTRY` in front of
//! it, and a DIB inside an icon lies about its height — `biHeight` is twice the
//! real height because the AND mask is stacked underneath the colour data. The
//! only place the *declared* size lives is the `GRPICONDIRENTRY` in the
//! `RT_GROUP_ICON` that references it. Reassembling the two into the `.ico`
//! layout they were split from is the documented round trip, it puts the
//! declared size back next to the pixels, and it produces bytes any decoder
//! accepts — which is what makes the result checkable against a real file.
//!
//! ## The all-zero alpha channel
//!
//! A 32-bit icon whose alpha channel is entirely zero is not a hypothetical.
//! Icon editors that write BGRA without ever setting alpha are common, and the
//! resulting icon renders as fully transparent — an empty card. Every 32bpp DIB
//! here is therefore scanned first, and when no pixel has any opacity the AND
//! mask is used instead. See `fukk_dib`.
//!
//! ## Bounds
//!
//! Everything below reads a file that a stranger produced. Every loop has a
//! named ceiling and every read is bounds-checked, so a truncated, circular or
//! deliberately hostile executable costs bounded work and returns an error
//! naming what it was refused for.

use std::borrow::Cow;
use std::cmp::Reverse;
use std::fs::File;
use std::io::{Cursor, Read as _, Seek as _};
use std::path::{Path, PathBuf};

use image::{ImageFormat, ImageReader, Limits};
use object::read::{ReadCache, ReadRef};
use object::{FileKind, Object as _, ObjectSection as _};

use crate::khata::KhataKashf;

// ---------------------------------------------------------------------------
// Bounds — every one of them exists for a specific way a file can be hostile
// ---------------------------------------------------------------------------

/// How many bytes are read to identify the container.
///
/// Sixteen covers the longest magic any of these formats has (`bplist00` is
/// eight, a Mach-O fat header is eight) with room to read a second field.
const TUL_SIHR: u64 = 16;

/// Largest whole file this module will read into memory.
///
/// Reached only for the small containers — a bare `.ico`, an `.icns`, a `.png`.
/// Executables are never read whole: they go through [`ReadCache`], which fetches
/// only the header and the one section the walk needs. Thirty-two mebibytes is
/// about sixty times the largest real `.icns` and still an allocation the
/// machine absorbs without noticing.
const HADD_HAJM_HAWIYA: u64 = 32 * 1024 * 1024;

/// Largest `.rsrc` section this module will read.
///
/// Sixteen mebibytes. A game executable's resource section holds icons, a
/// version block and a manifest, and is measured in hundreds of kilobytes. A PE
/// declaring more than this for its resources is either packing an installer
/// payload in there or is not a PE this module should be spending time on.
const HADD_HAJM_RSRC: u64 = 16 * 1024 * 1024;

/// How deep the resource tree is walked.
///
/// Three, because the format defines exactly three levels — type, name,
/// language — and a fourth can only be a malformed or circular directory. A
/// subdirectory pointer that points back at its own parent is a legal encoding
/// of an infinite tree; this constant is the only thing that stops the walk.
const HADD_UMQ_SHAJARA: usize = 3;

/// How many entries are read from a single resource directory node.
///
/// The header counts named and id entries as two `u16` fields, so a crafted
/// node can declare 131 070 entries and each one costs a subtree walk. Two
/// thousand is far past any real executable — the largest resource type table
/// observed in shipping games is under a hundred.
const HADD_QUYUD_MUSTAWA: usize = 2048;

/// How many icon resources are harvested from one executable, in total.
const HADD_MAWARID: usize = 4096;

/// How many `RT_GROUP_ICON` groups are reassembled.
///
/// Sixteen. An executable normally has one; a launcher stub has two or three.
/// Past this, work is being spent on resources no card will ever show.
const HADD_MAJMUAT: usize = 16;

/// How many entries one `ICONDIR` or `GRPICONDIR` may declare.
///
/// Sixty-four is roughly four times the largest real icon set (a Windows icon
/// with every size and depth variant is about sixteen images).
const HADD_QUYUD_ICO: usize = 64;

/// How many chunks are read from one ICNS container.
const HADD_QITA_ICNS: usize = 256;

/// Longest resource name string, in UTF-16 code units.
const HADD_TUL_ISM_MAWRID: usize = 256;

/// Largest side, in pixels, an *icon* may declare.
///
/// A thousand and twenty-four, which is exactly the largest icon any of these
/// formats defines: ICNS `ic10` is 1024×1024 and a Windows `.ico` entry encodes
/// its size in one byte, so 256 is its ceiling once the zero-means-256
/// convention is applied. Anything larger is a DIB header lying about its size
/// in order to make the decoder allocate.
const HADD_BUD_AYQUNA: u32 = 1024;

/// Largest side, in pixels, a plain image decoded by this module may declare.
///
/// Separate from [`HADD_BUD_AYQUNA`] because [`bmp_ila_rgba`] is not decoding an
/// icon — it exists because this workspace builds `image` without the BMP
/// feature and an Unreal `Splash.bmp` still has to become pixels. Splash art is
/// legitimately wider than any icon.
const HADD_BUD_SURA: u32 = 8192;

/// Largest decoded buffer, in bytes, for one icon.
///
/// 1024 × 1024 × 4 exactly: the largest legal icon at four bytes a pixel.
const HADD_BAYT_AYQUNA: u64 = 4 * 1024 * 1024;

/// Largest decoded buffer, in bytes, for one plain image.
const HADD_BAYT_SURA: u64 = 64 * 1024 * 1024;

/// How many icons one call returns.
///
/// The cascade takes the first that passes [`crate::silsila::MurashahSura::kaf`],
/// so everything past the largest handful is decoded work nobody looks at.
const HADD_AYQUNAT_MURJAA: usize = 16;

/// Largest `Info.plist` read.
///
/// One mebibyte. A bundle's `Info.plist` is a few kilobytes; a megabyte of it
/// is a file that is not an `Info.plist`.
const HADD_HAJM_PLIST: u64 = 1024 * 1024;

/// Largest `.desktop` or `project.godot` file read.
const HADD_HAJM_MAQATI: u64 = 512 * 1024;

/// How many lines the section reader consumes.
const HADD_ASTUR: usize = 4096;

/// Longest line the section reader accepts, in bytes.
///
/// A longer line is not a key/value pair; it is a minified blob that happens to
/// contain an `=`.
const HADD_TUL_SATR: usize = 4096;

/// How many sections the reader keeps.
const HADD_MAQATI: usize = 128;

/// How many keys the reader keeps per section.
const HADD_MAFATIH: usize = 512;

/// How many filesystem probes the icon-theme lookup is allowed.
///
/// Four base directories times one theme times a dozen size directories times
/// two extensions is under a hundred; five hundred leaves room for a second
/// theme without ever letting a crafted `Icon=` name turn into a directory
/// sweep.
const HADD_FAHS_SIMAT: usize = 512;

/// How many entries a bounded `read_dir` here will look at.
const HADD_MUDAKHALAT_MUJALLAD: usize = 4096;

// ---------------------------------------------------------------------------
// The format's own numbers
// ---------------------------------------------------------------------------

/// `RT_ICON`, the resource type holding one icon image.
const NAW_AYQUNA: u32 = 3;

/// `RT_GROUP_ICON`, the resource type holding the directory that names which
/// [`NAW_AYQUNA`] resources belong together and at what sizes.
const NAW_MAJMUAT_AYQUNA: u32 = 14;

/// Size of `IMAGE_RESOURCE_DIRECTORY`.
const HAJM_DALIL_MAWARID: usize = 16;

/// Size of `IMAGE_RESOURCE_DIRECTORY_ENTRY`.
const HAJM_QAYD_DALIL: usize = 8;

/// Size of `IMAGE_RESOURCE_DATA_ENTRY`.
const HAJM_QAYD_BAYANAT: usize = 16;

/// The bit that marks a resource directory entry as pointing at a subdirectory
/// rather than at a data entry, and a name field as pointing at a string rather
/// than holding an integer id.
const RAYAT_FAR: u32 = 0x8000_0000;

/// Size of `ICONDIR` / `GRPICONDIR`: reserved, type, count.
const HAJM_TARWISAT_ICO: usize = 6;

/// Size of `ICONDIRENTRY`, the on-disk `.ico` entry.
const HAJM_QAYD_ICO: usize = 16;

/// Size of `GRPICONDIRENTRY`, the in-resource entry — the same twelve leading
/// bytes as [`HAJM_QAYD_ICO`], then a `u16` resource id instead of a `u32` file
/// offset.
const HAJM_QAYD_MAJMUA: usize = 14;

/// How many leading bytes `GRPICONDIRENTRY` and `ICONDIRENTRY` share verbatim.
const HAJM_MUSHTARAK_QAYD: usize = 12;

/// Size of `BITMAPINFOHEADER`, the smallest DIB header this module accepts.
const HAJM_TARWISAT_DIB: usize = 40;

/// Size of `BITMAPFILEHEADER`, which a standalone `.bmp` carries and an icon
/// payload does not.
const HAJM_TARWISAT_BMP: usize = 14;

/// The eight bytes every PNG begins with.
const SIHR_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The four bytes an ICNS container begins with.
const SIHR_ICNS: [u8; 4] = *b"icns";

/// The signature box a raw JPEG 2000 codestream inside an ICNS chunk begins
/// with. Detected only so the refusal can name the format.
const SIHR_JP2: [u8; 4] = [0x00, 0x00, 0x00, 0x0C];

/// The magic a binary property list begins with.
const SIHR_BPLIST: [u8; 8] = *b"bplist00";

/// The ICNS chunk types whose payload is a PNG in every macOS release that
/// writes them.
///
/// The number is the square side in pixels: the `@2x` variants (`ic11`..`ic14`)
/// are twice their nominal point size, which is why `ic13` and `ic09` are both
/// 256 and `ic14` and `ic10` overlap in the same way.
const ANWA_ICNS_PNG: [([u8; 4], u32); 8] = [
    (*b"ic07", 128),
    (*b"ic08", 256),
    (*b"ic09", 512),
    (*b"ic10", 1024),
    (*b"ic11", 32),
    (*b"ic12", 64),
    (*b"ic13", 256),
    (*b"ic14", 512),
];

/// The ICNS chunk types this module recognises and refuses.
///
/// Every one of them is a run-length-encoded RGB plane whose alpha lives in a
/// separate `*8mk` chunk, or a raw ARGB block. Decoding them is a second image
/// codec written for icons that have been superseded since Mac OS X 10.7, and
/// any bundle old enough to carry only these also carries no artwork worth
/// putting on a card. They are listed rather than ignored so the diagnostics
/// trail says *what* was there instead of "no icon".
const ANWA_ICNS_QADIMA: [[u8; 4]; 17] = [
    *b"ICN#", *b"icm#", *b"icm4", *b"icm8", *b"ics#", *b"ics4", *b"ics8", *b"icl4", *b"icl8",
    *b"ich#", *b"ich4", *b"ich8", *b"it32", *b"t8mk", *b"is32", *b"il32", *b"ih32",
];

/// The size directories the freedesktop icon theme lookup walks, largest first.
///
/// `scalable` is deliberately absent: it holds SVG, and see [`ayqunat_elf`] for
/// why an SVG is refused rather than guessed at.
const MUJALLADAT_MAQAYIS: [&str; 13] = [
    "1024x1024",
    "512x512",
    "384x384",
    "256x256",
    "192x192",
    "128x128",
    "96x96",
    "72x72",
    "64x64",
    "48x48",
    "32x32",
    "24x24",
    "16x16",
];

/// The icon theme searched. Only `hicolor`, which is the fallback theme every
/// freedesktop implementation is required to have and the only one a game's own
/// installer ever writes into.
const SIMA_ASASIYA: &str = "hicolor";

/// The subdirectory inside a theme size directory that application icons live
/// in.
const QISM_SIMA: &str = "apps";

// ---------------------------------------------------------------------------
// The result
// ---------------------------------------------------------------------------

/// One icon, decoded.
///
/// Carries pixels rather than a path because there is no file to point at: a PE
/// icon exists only inside the executable, and an ICNS sub-image only inside the
/// container. The caller writes it wherever it keeps artwork; this module has no
/// opinion about that and no access to the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AyqunaMustakhraja {
    /// Width in pixels.
    pub ard: u32,
    /// Height in pixels.
    pub irtifa: u32,
    /// Straight (non-premultiplied) RGBA, row-major, top row first, exactly
    /// `ard * irtifa * 4` bytes.
    pub biksilat: Vec<u8>,
    /// What this is, for the diagnostics trail: `PE icon group 1 entry 256×256`,
    /// `ICNS ic10`, `.desktop Icon=celeste → /usr/share/icons/…`.
    pub wasf: String,
}

impl AyqunaMustakhraja {
    /// Pixel count, which is how the caller ranks one icon against another.
    #[must_use]
    pub fn masaha(&self) -> u64 {
        u64::from(self.ard).saturating_mul(u64::from(self.irtifa))
    }
}

/// The one refusal this module produces.
///
/// [`KhataKashf::SuraGhayrSaliha`] rather than a new variant, because every
/// failure here means the same thing to everything downstream: this source did
/// not yield artwork, the cascade continues, and the reason belongs in the
/// trail. `rabt` carries the path or the container position so a bundle names
/// the file that was refused.
fn rafd(wasf: &str, tafsil: impl Into<String>) -> KhataKashf {
    KhataKashf::SuraGhayrSaliha {
        rabt: wasf.to_owned(),
        tafsil: tafsil.into(),
    }
}

// ---------------------------------------------------------------------------
// Bounds-checked primitive reads
// ---------------------------------------------------------------------------
//
// Every multi-byte read in this file goes through one of these. None of them
// can index out of bounds and none of them can wrap: a truncated file makes
// them return `None`, which becomes a named refusal at the call site rather
// than a panic inside a parser.

/// One byte.
fn bayt_wahid(bayt: &[u8], mawdi: usize) -> Option<u8> {
    bayt.get(mawdi).copied()
}

/// A little-endian `u16`.
fn u16_saghir(bayt: &[u8], mawdi: usize) -> Option<u16> {
    let nihaya = mawdi.checked_add(2)?;
    let qita: [u8; 2] = bayt.get(mawdi..nihaya)?.try_into().ok()?;
    Some(u16::from_le_bytes(qita))
}

/// A little-endian `u32`.
fn u32_saghir(bayt: &[u8], mawdi: usize) -> Option<u32> {
    let nihaya = mawdi.checked_add(4)?;
    let qita: [u8; 4] = bayt.get(mawdi..nihaya)?.try_into().ok()?;
    Some(u32::from_le_bytes(qita))
}

/// A little-endian `i32`.
fn i32_saghir(bayt: &[u8], mawdi: usize) -> Option<i32> {
    let nihaya = mawdi.checked_add(4)?;
    let qita: [u8; 4] = bayt.get(mawdi..nihaya)?.try_into().ok()?;
    Some(i32::from_le_bytes(qita))
}

/// A big-endian `u32`, which is what ICNS uses for every length it declares.
fn u32_kabir(bayt: &[u8], mawdi: usize) -> Option<u32> {
    let nihaya = mawdi.checked_add(4)?;
    let qita: [u8; 4] = bayt.get(mawdi..nihaya)?.try_into().ok()?;
    Some(u32::from_be_bytes(qita))
}

/// Four bytes as a fixed array, which is how every chunk tag is compared.
fn wasm_arbaa(bayt: &[u8], mawdi: usize) -> Option<[u8; 4]> {
    let nihaya = mawdi.checked_add(4)?;
    bayt.get(mawdi..nihaya)?.try_into().ok()
}

/// A run of bytes, or [`None`] when the run is not entirely inside the buffer.
fn shariha(bayt: &[u8], mawdi: usize, tul: usize) -> Option<&[u8]> {
    let nihaya = mawdi.checked_add(tul)?;
    bayt.get(mawdi..nihaya)
}

/// Whether a buffer opens with a given signature.
fn yabda_bi(bayt: &[u8], sihr: &[u8]) -> bool {
    bayt.get(..sihr.len()).is_some_and(|badiya| badiya == sihr)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Every icon a file yields, largest first.
///
/// Dispatches on the file's own magic rather than on the host, so a Windows
/// `.exe` inside a Proton prefix is read correctly from a Linux build and a
/// Linux ELF is read correctly from Windows. A macOS `.app` is a directory
/// rather than a file and is recognised as one before anything is opened.
///
/// The result is sorted by pixel count, descending, and truncated to sixteen —
/// see `HADD_AYQUNAT_MURJAA`. Descending is the order
/// [`crate::silsila::Silsila::hall`] relies on within a rung.
///
/// `judhur` is [`crate::fahs::SiyaqFahs::judhur_bayanat`], the XDG data
/// hierarchy to search. Only the ELF path consults it — a Linux executable's
/// icon usually lives in a `.desktop` entry and an icon theme rather than in the
/// binary — and passing an empty slice restricts that path to the directory the
/// executable sits in, which is what a caller scanning a Windows or macOS
/// machine wants.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the file cannot be opened or read, when
/// its magic matches no container this module parses, when the container is
/// structurally malformed, or when it parses cleanly and simply holds no icon.
/// The last case is an error rather than an empty `Ok` on purpose: the cascade's
/// diagnostics trail has to be able to say *why* a rung produced nothing, and
/// "the executable has no `.rsrc` section" and "the icon group referenced three
/// images that are not in the file" are different answers to that question.
pub fn ayqunat(masar: &Path, judhur: &[PathBuf]) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    let wasf = masar.display().to_string();

    if masar.is_dir() {
        let hiya_hazma = masar
            .extension()
            .and_then(|imtidad| imtidad.to_str())
            .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("app"));
        if hiya_hazma {
            let murattaba = rattib(ayqunat_hazma(masar, &wasf)?);
            if murattaba.is_empty() {
                return Err(rafd(&wasf, "its bundle's icons all decoded to nothing"));
            }
            return Ok(murattaba);
        }
        return Err(rafd(
            &wasf,
            "a directory is not an executable and not an .app bundle",
        ));
    }

    let mut malaf =
        File::open(masar).map_err(|sabab| rafd(&wasf, format!("cannot open it: {sabab}")))?;

    let mut sihr: Vec<u8> =
        Vec::with_capacity(usize::try_from(TUL_SIHR).unwrap_or(HAJM_TARWISAT_DIB));
    let _ = (&mut malaf)
        .take(TUL_SIHR)
        .read_to_end(&mut sihr)
        .map_err(|sabab| rafd(&wasf, format!("cannot read its first bytes: {sabab}")))?;
    malaf
        .rewind()
        .map_err(|sabab| rafd(&wasf, format!("cannot rewind after sniffing it: {sabab}")))?;

    // An if-chain rather than a slice pattern: the magics have three different
    // lengths and two of them (`ICONDIR`, Mach-O) are a set rather than a
    // constant, so one `match` over `sihr.get(..n)` would need a different `n`
    // per arm and would read worse than the table it replaced.
    let natija = if yabda_bi(&sihr, b"MZ") {
        // Every PE begins with a DOS stub; `object` follows `e_lfanew` from
        // here to the real header.
        ayqunat_pe(malaf, &wasf)
    } else if yabda_bi(&sihr, b"\x7FELF") {
        ayqunat_elf(masar, malaf, &wasf, judhur)
    } else if sihr_mach(&sihr) {
        ayqunat_mach(masar, malaf, &wasf)
    } else if yabda_bi(&sihr, &SIHR_ICNS) {
        let bayt = iqra_hawiya(malaf, &wasf)?;
        hallil_icns(&bayt, &wasf)
    } else if yabda_bi(&sihr, b"\x00\x00\x01\x00") || yabda_bi(&sihr, b"\x00\x00\x02\x00") {
        // `ICONDIR`: two reserved zero bytes, then type 1 (icon) or 2 (cursor),
        // then the count. That reserved field *is* the format's magic, which is
        // why it is checked rather than skipped.
        let bayt = iqra_hawiya(malaf, &wasf)?;
        hallil_ico(&bayt, &wasf)
    } else if yabda_bi(&sihr, b"BM") {
        let bayt = iqra_hawiya(malaf, &wasf)?;
        bmp_ila_rgba(&bayt, &wasf).map(|wahida| vec![wahida])
    } else if yabda_bi(&sihr, &SIHR_PNG) {
        let bayt = iqra_hawiya(malaf, &wasf)?;
        fukk_mushaffara(&bayt, &wasf, HADD_BUD_SURA, HADD_BAYT_SURA).map(|wahida| vec![wahida])
    } else {
        Err(rafd(
            &wasf,
            "its first bytes match no container this build reads: not PE, ELF, Mach-O, ICNS, \
             ICO, BMP or PNG",
        ))
    }?;

    let murattaba = rattib(natija);
    if murattaba.is_empty() {
        return Err(rafd(&wasf, "every icon it yielded decoded to nothing"));
    }
    Ok(murattaba)
}

/// Sorts largest first, drops empties, and applies the return cap.
fn rattib(mut ayqunat: Vec<AyqunaMustakhraja>) -> Vec<AyqunaMustakhraja> {
    ayqunat.retain(|wahida| wahida.ard > 0 && wahida.irtifa > 0 && !wahida.biksilat.is_empty());
    ayqunat.sort_by_key(|wahida| Reverse(wahida.masaha()));
    ayqunat.truncate(HADD_AYQUNAT_MURJAA);
    ayqunat
}

/// Reads a whole small container, refusing one larger than [`HADD_HAJM_HAWIYA`].
fn iqra_hawiya(malaf: File, wasf: &str) -> Result<Vec<u8>, KhataKashf> {
    let mut bayt = Vec::new();
    let _ = malaf
        .take(HADD_HAJM_HAWIYA)
        .read_to_end(&mut bayt)
        .map_err(|sabab| rafd(wasf, format!("cannot read it: {sabab}")))?;
    if bayt.is_empty() {
        return Err(rafd(wasf, "the file is empty"));
    }
    Ok(bayt)
}

/// Whether these bytes open with one of the six Mach-O magics.
///
/// Both endiannesses of the 32-bit and 64-bit thin magics, and both of the fat
/// (universal binary) magics. `0xCAFEBABE` is also the Java class-file magic,
/// which is why [`ayqunat_mach`] confirms with [`FileKind::parse`] before it
/// concludes anything.
fn sihr_mach(sihr: &[u8]) -> bool {
    const ASHKAL: [[u8; 4]; 6] = [
        [0xFE, 0xED, 0xFA, 0xCE],
        [0xFE, 0xED, 0xFA, 0xCF],
        [0xCE, 0xFA, 0xED, 0xFE],
        [0xCF, 0xFA, 0xED, 0xFE],
        [0xCA, 0xFE, 0xBA, 0xBE],
        [0xBE, 0xBA, 0xFE, 0xCA],
    ];
    ASHKAL.iter().any(|wahid| yabda_bi(sihr, wahid))
}

// ---------------------------------------------------------------------------
// PE — the Windows resource tree
// ---------------------------------------------------------------------------
//
// `object` is used for exactly two things here: identifying the file as PE32 or
// PE32+, and locating `.rsrc` together with the section table needed to turn an
// RVA into a file offset. It does not expose resource *entries*, so the tree
// below is walked by hand.
//
// The layout, which is what the walk implements:
//
//   IMAGE_RESOURCE_DIRECTORY          16 bytes, then its entries
//     u32 Characteristics
//     u32 TimeDateStamp
//     u16 MajorVersion, u16 MinorVersion
//     u16 NumberOfNamedEntries, u16 NumberOfIdEntries
//   IMAGE_RESOURCE_DIRECTORY_ENTRY     8 bytes, repeated (named first, then id)
//     u32 Name          high bit set -> offset to a UTF-16 name; else an id
//     u32 OffsetToData  high bit set -> offset to a child directory;
//                       else offset to an IMAGE_RESOURCE_DATA_ENTRY
//   IMAGE_RESOURCE_DATA_ENTRY         16 bytes
//     u32 OffsetToData  an RVA, *not* an offset into .rsrc
//     u32 Size
//     u32 CodePage, u32 Reserved
//
// Every offset in an entry is relative to the start of the resource directory —
// which is the start of `.rsrc` — except `IMAGE_RESOURCE_DATA_ENTRY.OffsetToData`,
// which is relative to the image base. Mixing those two up is the classic bug in
// a hand-written resource reader, and it is why `SiyaqMawarid::jism` exists as a
// separate function with the translation written out.
//
// Three levels: type, then name-or-id, then language. Some toolchains emit only
// two, so a data entry found at level two is accepted rather than refused.

/// One section header, reduced to the four numbers an RVA translation needs.
#[derive(Debug, Clone, Copy)]
struct QitaMulakhkhasa {
    /// Where the section begins once loaded, relative to the image base.
    rva: u32,
    /// How many bytes it occupies in memory.
    hajm_wahmi: u32,
    /// Where its bytes begin in the file.
    izahat_malaf: u64,
    /// How many of those bytes the file actually contains, which for a section
    /// with uninitialised tail is less than `hajm_wahmi`.
    hajm_malaf: u64,
}

/// What a walked resource entry turned out to be.
#[derive(Debug, Clone)]
struct QaydMawrid {
    /// The level-one key: the resource type.
    naw: u32,
    /// The level-two key when it is an integer, which is the only form a
    /// `GRPICONDIRENTRY` can reference.
    raqm: Option<u32>,
    /// The level-two key when it is a string.
    ism: Option<String>,
    /// Where the bytes are, relative to the image base.
    rva: u32,
    /// How many of them there are.
    hajm: u32,
}

impl QaydMawrid {
    /// How this entry is named in the diagnostics trail.
    fn muarrif(&self) -> String {
        match (self.raqm, self.ism.as_deref()) {
            (Some(raqm), _) => raqm.to_string(),
            (None, Some(ism)) => format!("\"{ism}\""),
            (None, None) => "?".to_owned(),
        }
    }
}

/// The keys accumulated on the way down the tree.
#[derive(Debug, Clone, Default)]
struct MafatihMawrid {
    /// Level one.
    naw: Option<u32>,
    /// Level two, integer form.
    raqm: Option<u32>,
    /// Level two, string form.
    ism: Option<String>,
}

/// Everything needed to turn a resource RVA back into bytes.
struct SiyaqMawarid<'a> {
    /// The executable, read lazily by range.
    makhzan: &'a ReadCache<File>,
    /// The `.rsrc` section's bytes, already in memory because the whole walk
    /// runs over them.
    rsrc: &'a [u8],
    /// Where `.rsrc` begins, relative to the image base.
    rsrc_rva: u32,
    /// Every section, for the rare resource whose data is not in `.rsrc`.
    qitaa: Vec<QitaMulakhkhasa>,
}

impl<'a> SiyaqMawarid<'a> {
    /// The bytes an `IMAGE_RESOURCE_DATA_ENTRY` points at.
    ///
    /// Tries `.rsrc` first because that is where a compiler puts resource data
    /// and it needs no second read. The general path exists for executables run
    /// through a packer, which relocates the payload into its own section while
    /// leaving the directory where the loader expects it.
    fn jism(&self, rva: u32, hajm: u32) -> Option<&'a [u8]> {
        if hajm == 0 || u64::from(hajm) > HADD_HAJM_RSRC {
            return None;
        }
        let hajm_hajmi = usize::try_from(hajm).ok()?;

        if let Some(dakhili) = rva.checked_sub(self.rsrc_rva)
            && let Ok(mawdi) = usize::try_from(dakhili)
            && let Some(qita) = shariha(self.rsrc, mawdi, hajm_hajmi)
        {
            return Some(qita);
        }

        for qita in &self.qitaa {
            let Some(dakhili) = rva.checked_sub(qita.rva) else {
                continue;
            };
            if u64::from(dakhili) >= u64::from(qita.hajm_wahmi) {
                continue;
            }
            let Some(nihaya) = u64::from(dakhili).checked_add(u64::from(hajm)) else {
                continue;
            };
            if nihaya > qita.hajm_malaf {
                continue;
            }
            let Some(izaha) = qita.izahat_malaf.checked_add(u64::from(dakhili)) else {
                continue;
            };
            if let Ok(bayt) = ReadRef::read_bytes_at(self.makhzan, izaha, u64::from(hajm)) {
                return Some(bayt);
            }
        }
        None
    }
}

/// Every icon inside a Windows executable.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the PE header does not parse, when there
/// is no `.rsrc` section, when the resource tree holds no icon, or when every
/// icon group it holds references resources that are not in the file — the last
/// being what a stripped or packed executable looks like from here.
fn ayqunat_pe(malaf: File, wasf: &str) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    let makhzan = ReadCache::new(malaf);

    let naw = FileKind::parse(&makhzan)
        .map_err(|sabab| rafd(wasf, format!("its PE header does not parse: {sabab}")))?;
    let ittisaa = match naw {
        FileKind::Pe32 => "PE32",
        FileKind::Pe64 => "PE32+",
        akhar => {
            return Err(rafd(
                wasf,
                format!("it begins with MZ but is {akhar:?}, not a portable executable"),
            ));
        },
    };

    let kitab = object::File::parse(&makhzan)
        .map_err(|sabab| rafd(wasf, format!("its PE structure does not parse: {sabab}")))?;
    let asas = kitab.relative_address_base();

    let mut qitaa: Vec<QitaMulakhkhasa> = Vec::new();
    for qita in kitab.sections() {
        if qitaa.len() >= HADD_QUYUD_MUSTAWA {
            break;
        }
        let (Ok(rva), Ok(hajm_wahmi)) = (
            u32::try_from(qita.address().saturating_sub(asas)),
            u32::try_from(qita.size()),
        ) else {
            continue;
        };
        let Some((izahat_malaf, hajm_malaf)) = qita.file_range() else {
            continue;
        };
        qitaa.push(QitaMulakhkhasa {
            rva,
            hajm_wahmi,
            izahat_malaf,
            hajm_malaf,
        });
    }

    let rsrc = kitab.section_by_name(".rsrc").ok_or_else(|| {
        rafd(
            wasf,
            "it has no .rsrc section, so it carries no icon resource at all",
        )
    })?;
    let rsrc_rva = u32::try_from(rsrc.address().saturating_sub(asas)).map_err(|_| {
        rafd(
            wasf,
            "its .rsrc section is placed past the four-gigabyte image limit",
        )
    })?;
    let (izaha, hajm) = rsrc.file_range().ok_or_else(|| {
        rafd(
            wasf,
            "its .rsrc section is declared but has no bytes in the file",
        )
    })?;
    if hajm == 0 {
        return Err(rafd(wasf, "its .rsrc section is present but empty"));
    }
    if hajm > HADD_HAJM_RSRC {
        return Err(rafd(
            wasf,
            format!("its .rsrc section declares {hajm} bytes, over the {HADD_HAJM_RSRC} limit"),
        ));
    }
    let bayanat_rsrc = ReadRef::read_bytes_at(&makhzan, izaha, hajm).map_err(|()| {
        rafd(
            wasf,
            format!("its .rsrc section claims {hajm} bytes at {izaha}, which is past the file end"),
        )
    })?;

    let quyud = imsah_mawarid(bayanat_rsrc, wasf)?;
    let siyaq = SiyaqMawarid {
        makhzan: &makhzan,
        rsrc: bayanat_rsrc,
        rsrc_rva,
        qitaa,
    };

    let mut ajsam: Vec<(Option<u32>, &[u8])> = Vec::new();
    let mut majmuat: Vec<(String, &[u8])> = Vec::new();
    for qayd in &quyud {
        let Some(jism) = siyaq.jism(qayd.rva, qayd.hajm) else {
            continue;
        };
        if qayd.naw == NAW_AYQUNA {
            ajsam.push((qayd.raqm, jism));
        } else if majmuat.len() < HADD_MAJMUAT {
            majmuat.push((qayd.muarrif(), jism));
        }
    }

    if ajsam.is_empty() {
        return Err(rafd(
            wasf,
            "its resource tree names RT_ICON entries whose data is not inside the file, which is \
             what a packed or stripped executable looks like from outside",
        ));
    }

    let mut hasad: Vec<AyqunaMustakhraja> = Vec::new();
    let mut asbab: Vec<String> = Vec::new();
    for (muarrif, majmua) in &majmuat {
        let wasf_majmua = format!("{wasf}: {ittisaa} icon group {muarrif}");
        match ijma_ico(majmua, &ajsam, &wasf_majmua) {
            Ok(ico) => match hallil_ico(&ico, &wasf_majmua) {
                Ok(wahida) => hasad.extend(wahida),
                Err(sabab) => asbab.push(sabab.to_string()),
            },
            Err(sabab) => asbab.push(sabab.to_string()),
        }
    }

    // No usable group, but the RT_ICON bodies are right there. This happens with
    // executables whose group resource was stripped by a packer and with a few
    // very old toolchains that emitted RT_ICON alone, and decoding the bodies
    // directly is strictly better than returning nothing.
    if hasad.is_empty() {
        for (raqm, jism) in &ajsam {
            let muarrif = match raqm {
                Some(raqm) => raqm.to_string(),
                None => "?".to_owned(),
            };
            let wasf_jism = format!("{wasf}: {ittisaa} RT_ICON {muarrif}, no group");
            if let Ok(wahida) = hallil_jism_ayquna(jism, 0, 0, &wasf_jism) {
                hasad.push(wahida);
            }
            if hasad.len() >= HADD_AYQUNAT_MURJAA {
                break;
            }
        }
    }

    if hasad.is_empty() {
        let tafsil = if asbab.is_empty() {
            "none of its icon resources decoded".to_owned()
        } else {
            asbab.join("; ")
        };
        return Err(rafd(wasf, tafsil));
    }
    Ok(hasad)
}

/// Walks the resource tree and returns every `RT_ICON` and `RT_GROUP_ICON` leaf.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the root directory is truncated or when
/// the tree contains neither resource type. A malformed *subtree* is skipped
/// rather than fatal: a resource type this module does not care about being
/// corrupt says nothing about the icons.
fn imsah_mawarid(rsrc: &[u8], wasf: &str) -> Result<Vec<QaydMawrid>, KhataKashf> {
    if rsrc.len() < HAJM_DALIL_MAWARID {
        return Err(rafd(
            wasf,
            "its .rsrc section is shorter than one resource directory header",
        ));
    }
    let mut hasad = Vec::new();
    imsah_dalil(rsrc, 0, 0, &MafatihMawrid::default(), &mut hasad);
    if hasad.is_empty() {
        return Err(rafd(
            wasf,
            "its resource tree holds no RT_ICON or RT_GROUP_ICON entry",
        ));
    }
    Ok(hasad)
}

/// Where entry `fahras` of a directory at `asas` begins.
fn izaha_qayd(asas: usize, fahras: usize) -> Option<usize> {
    let badiya = asas.checked_add(HAJM_DALIL_MAWARID)?;
    badiya.checked_add(fahras.checked_mul(HAJM_QAYD_DALIL)?)
}

/// One directory node, and its children.
///
/// Returns nothing and reports nothing: every way a node can be wrong — a
/// truncated header, an entry pointing past the section, a child offset that
/// loops back to an ancestor — means that subtree yields no icons, and the
/// caller's job is to notice that the harvest came back empty. Refusing the
/// whole executable because one unrelated resource type is damaged would throw
/// away icons that are perfectly readable.
fn imsah_dalil(
    rsrc: &[u8],
    izaha: usize,
    umq: usize,
    mafatih: &MafatihMawrid,
    hasad: &mut Vec<QaydMawrid>,
) {
    if umq >= HADD_UMQ_SHAJARA || hasad.len() >= HADD_MAWARID {
        return;
    }
    let (Some(mawdi_musamma), Some(mawdi_raqmi)) = (izaha.checked_add(12), izaha.checked_add(14))
    else {
        return;
    };
    let (Some(bi_ism), Some(bi_raqm)) = (
        u16_saghir(rsrc, mawdi_musamma),
        u16_saghir(rsrc, mawdi_raqmi),
    ) else {
        return;
    };
    let kul = usize::from(bi_ism)
        .saturating_add(usize::from(bi_raqm))
        .min(HADD_QUYUD_MUSTAWA);

    for fahras in 0..kul {
        if hasad.len() >= HADD_MAWARID {
            return;
        }
        let Some(mawdi) = izaha_qayd(izaha, fahras) else {
            return;
        };
        let (Some(ism_khaam), Some(bayanat_khaam)) = (
            u32_saghir(rsrc, mawdi),
            u32_saghir(rsrc, mawdi.saturating_add(4)),
        ) else {
            return;
        };

        let mut faraiya = mafatih.clone();
        // The high bit says the name field is an offset to a UTF-16 string
        // rather than an integer id. Both forms are real: `RT_ICON` is always
        // an id at level one, but a game's own resources are frequently named,
        // and a walk that assumed integers would mis-key every level-two entry
        // in the same tree.
        if ism_khaam & RAYAT_FAR == 0 {
            match umq {
                0 => faraiya.naw = Some(ism_khaam),
                1 => faraiya.raqm = Some(ism_khaam),
                _ => {},
            }
        } else {
            let izahat_ism = usize::try_from(ism_khaam & !RAYAT_FAR).unwrap_or(usize::MAX);
            let ism = iqra_ism_mawrid(rsrc, izahat_ism);
            match umq {
                // A named resource *type* is a custom type, never RT_ICON, so
                // the subtree is skipped outright rather than walked.
                0 => continue,
                1 => faraiya.ism = ism,
                _ => {},
            }
        }

        // At level one, everything that is not one of the two icon types is a
        // subtree this module has no business walking. Skipping here is what
        // keeps the walk proportional to the icons rather than to the whole
        // resource section.
        if umq == 0 && !matches!(faraiya.naw, Some(NAW_AYQUNA | NAW_MAJMUAT_AYQUNA)) {
            continue;
        }

        if bayanat_khaam & RAYAT_FAR == 0 {
            let Some(naw) = faraiya.naw else {
                continue;
            };
            let Ok(mawdi_bayanat) = usize::try_from(bayanat_khaam) else {
                continue;
            };
            if shariha(rsrc, mawdi_bayanat, HAJM_QAYD_BAYANAT).is_none() {
                continue;
            }
            let (Some(rva), Some(hajm)) = (
                u32_saghir(rsrc, mawdi_bayanat),
                u32_saghir(rsrc, mawdi_bayanat.saturating_add(4)),
            ) else {
                continue;
            };
            hasad.push(QaydMawrid {
                naw,
                raqm: faraiya.raqm,
                ism: faraiya.ism,
                rva,
                hajm,
            });
        } else {
            let Ok(mawdi_far) = usize::try_from(bayanat_khaam & !RAYAT_FAR) else {
                continue;
            };
            // A child that points at or before its own parent is the encoding
            // of a cycle. The depth cap already bounds the walk; this makes the
            // refusal exact instead of merely finite.
            if mawdi_far <= izaha {
                continue;
            }
            imsah_dalil(rsrc, mawdi_far, umq.saturating_add(1), &faraiya, hasad);
        }
    }
}

/// An `IMAGE_RESOURCE_DIR_STRING_U`: a `u16` length in code units, then that
/// many little-endian UTF-16 code units.
///
/// Unpaired surrogates become the replacement character rather than an error.
/// The string is a diagnostic label, and a resource name that is not valid
/// UTF-16 is still worth showing as the mangled thing it is.
fn iqra_ism_mawrid(rsrc: &[u8], izaha: usize) -> Option<String> {
    let tul = usize::from(u16_saghir(rsrc, izaha)?).min(HADD_TUL_ISM_MAWRID);
    let bidaya = izaha.checked_add(2)?;
    let bayt = shariha(rsrc, bidaya, tul.checked_mul(2)?)?;
    let wahdat: Vec<u16> = bayt
        .chunks_exact(2)
        .filter_map(|zawj| <[u8; 2]>::try_from(zawj).ok().map(u16::from_le_bytes))
        .collect();
    Some(
        char::decode_utf16(wahdat)
            .map(|harf| harf.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
    )
}

/// Rebuilds a real `.ico` from a `GRPICONDIR` and the `RT_ICON` bodies it names.
///
/// The two structures are byte-identical for their first twelve bytes —
/// `bWidth`, `bHeight`, `bColorCount`, `bReserved`, `wPlanes`, `wBitCount`,
/// `dwBytesInRes` — and differ only in the tail: a group entry ends with a
/// `u16` resource id, a file entry with a `u32` offset. So the reassembly is a
/// copy of those twelve bytes with the tail replaced, plus the bodies
/// concatenated behind the table.
///
/// `dwBytesInRes` is **rewritten** from the resource's real length rather than
/// copied. The group's copy is what the linker recorded and is not always what
/// the resource ended up being — a resource editor that replaced one image
/// without touching the group leaves the two disagreeing, and a decoder that
/// trusted the group would read past the end of one image and into the next.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the group header is truncated, declares
/// a nonsensical type, or names no resource that is actually present.
fn ijma_ico(
    majmua: &[u8],
    ajsam: &[(Option<u32>, &[u8])],
    wasf: &str,
) -> Result<Vec<u8>, KhataKashf> {
    let (Some(mahjuz), Some(naw), Some(adad_muallan)) = (
        u16_saghir(majmua, 0),
        u16_saghir(majmua, 2),
        u16_saghir(majmua, 4),
    ) else {
        return Err(rafd(wasf, "its GRPICONDIR header is truncated"));
    };
    if mahjuz != 0 || !matches!(naw, 1 | 2) {
        return Err(rafd(
            wasf,
            format!("its GRPICONDIR declares reserved={mahjuz} type={naw}, which is not an icon"),
        ));
    }
    let adad = usize::from(adad_muallan).min(HADD_QUYUD_ICO);
    if adad == 0 {
        return Err(rafd(wasf, "its GRPICONDIR declares no entries"));
    }

    // Collected first so the file offsets can be computed from the real table
    // size, which depends on how many entries survived the lookup.
    let mut mukhtara: Vec<(&[u8], &[u8])> = Vec::with_capacity(adad);
    for fahras in 0..adad {
        let Some(mawdi) = HAJM_TARWISAT_ICO.checked_add(fahras.saturating_mul(HAJM_QAYD_MAJMUA))
        else {
            break;
        };
        let Some(mushtarak) = shariha(majmua, mawdi, HAJM_MUSHTARAK_QAYD) else {
            break;
        };
        let Some(muarrif) = u16_saghir(majmua, mawdi.saturating_add(HAJM_MUSHTARAK_QAYD)) else {
            break;
        };
        let matlub = u32::from(muarrif);
        let Some((_, jism)) = ajsam.iter().find(|(raqm, _)| *raqm == Some(matlub)) else {
            continue;
        };
        mukhtara.push((mushtarak, jism));
    }

    if mukhtara.is_empty() {
        return Err(rafd(
            wasf,
            "none of the RT_ICON resources its group names are in the file",
        ));
    }

    let adad_fili = u16::try_from(mukhtara.len()).map_err(|_| {
        rafd(
            wasf,
            "its group holds more entries than an ICONDIR can address",
        )
    })?;
    let jadwal = HAJM_TARWISAT_ICO
        .checked_add(mukhtara.len().saturating_mul(HAJM_QAYD_ICO))
        .ok_or_else(|| rafd(wasf, "its group's entry table does not fit in memory"))?;

    let hajm_kulli: usize = mukhtara.iter().map(|(_, jism)| jism.len()).sum();
    let mut ico: Vec<u8> = Vec::with_capacity(jadwal.saturating_add(hajm_kulli));
    ico.extend_from_slice(&0_u16.to_le_bytes());
    ico.extend_from_slice(&naw.to_le_bytes());
    ico.extend_from_slice(&adad_fili.to_le_bytes());

    let mut mawdi_jism = jadwal;
    for (mushtarak, jism) in &mukhtara {
        let hajm_jism = u32::try_from(jism.len()).map_err(|_| {
            rafd(
                wasf,
                "one RT_ICON resource is larger than a u32 can describe",
            )
        })?;
        let izaha = u32::try_from(mawdi_jism).map_err(|_| {
            rafd(
                wasf,
                "the reassembled icon is larger than a u32 can address",
            )
        })?;
        // Bytes 0..8 verbatim (size, colour count, planes, bit count), then the
        // corrected length, then the file offset.
        let Some(ras) = mushtarak.get(..8) else {
            return Err(rafd(wasf, "one GRPICONDIRENTRY is truncated"));
        };
        ico.extend_from_slice(ras);
        ico.extend_from_slice(&hajm_jism.to_le_bytes());
        ico.extend_from_slice(&izaha.to_le_bytes());
        mawdi_jism = mawdi_jism.saturating_add(jism.len());
    }
    for (_, jism) in &mukhtara {
        ico.extend_from_slice(jism);
    }
    Ok(ico)
}

// ---------------------------------------------------------------------------
// ICO / CUR, and the DIB inside it
// ---------------------------------------------------------------------------
//
// `image` is built in this workspace with `png`, `jpeg` and `webp` only, so
// there is no ICO decoder and no BMP decoder to lean on. Both are parsed here.
//
//   ICONDIR                6 bytes
//     u16 idReserved       always 0 — this field is the format's only magic
//     u16 idType           1 icon, 2 cursor
//     u16 idCount
//   ICONDIRENTRY          16 bytes, repeated
//     u8  bWidth           0 means 256
//     u8  bHeight          0 means 256
//     u8  bColorCount      0 when the image has more than 256 colours
//     u8  bReserved
//     u16 wPlanes          for a cursor this is the hotspot x, not a plane count
//     u16 wBitCount        for a cursor this is the hotspot y
//     u32 dwBytesInRes
//     u32 dwImageOffset
//
// The payload is either a whole PNG — which Vista introduced for the 256×256
// entry and which is now normal at every size — or a DIB. The PNG case is handed
// to `image`. The DIB case is decoded below, and it is not the same thing as a
// `.bmp`: there is no BITMAPFILEHEADER, `biHeight` is doubled, and an AND mask
// follows the colour data.

/// Every image inside an `.ico` or `.cur` container.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the `ICONDIR` header is truncated or
/// declares a type that is neither icon nor cursor, when it declares no entries,
/// or when no entry decoded — the last carrying every individual entry's reason,
/// joined, because "the icon did not decode" without saying which of the six
/// images failed and how is not an answer anybody can act on.
pub fn hallil_ico(bayt: &[u8], wasf: &str) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    let (Some(mahjuz), Some(naw), Some(adad_muallan)) = (
        u16_saghir(bayt, 0),
        u16_saghir(bayt, 2),
        u16_saghir(bayt, 4),
    ) else {
        return Err(rafd(wasf, "its ICONDIR header is truncated"));
    };
    if mahjuz != 0 || !matches!(naw, 1 | 2) {
        return Err(rafd(
            wasf,
            format!("its ICONDIR declares reserved={mahjuz} type={naw}, which is not an icon"),
        ));
    }
    let adad = usize::from(adad_muallan).min(HADD_QUYUD_ICO);
    if adad == 0 {
        return Err(rafd(wasf, "its ICONDIR declares no entries"));
    }

    let mut hasad: Vec<AyqunaMustakhraja> = Vec::new();
    let mut asbab: Vec<String> = Vec::new();

    for fahras in 0..adad {
        if hasad.len() >= HADD_AYQUNAT_MURJAA {
            break;
        }
        let Some(mawdi) = fahras
            .checked_mul(HAJM_QAYD_ICO)
            .and_then(|izaha| HAJM_TARWISAT_ICO.checked_add(izaha))
        else {
            break;
        };
        let (Some(ard_bayt), Some(irtifa_bayt)) = (
            bayt_wahid(bayt, mawdi),
            bayt_wahid(bayt, mawdi.saturating_add(1)),
        ) else {
            break;
        };
        let (Some(hajm), Some(izaha)) = (
            u32_saghir(bayt, mawdi.saturating_add(8)),
            u32_saghir(bayt, mawdi.saturating_add(12)),
        ) else {
            break;
        };

        // Zero means 256. The field is one byte, so 256 is the only size that
        // cannot be written literally, and it is the most common large size —
        // an entry that read this as "a zero-pixel icon" would discard exactly
        // the image a card wants.
        let ard_muallan = if ard_bayt == 0 {
            256
        } else {
            u32::from(ard_bayt)
        };
        let irtifa_muallan = if irtifa_bayt == 0 {
            256
        } else {
            u32::from(irtifa_bayt)
        };

        let (Ok(mawdi_jism), Ok(hajm_jism)) = (usize::try_from(izaha), usize::try_from(hajm))
        else {
            continue;
        };
        let Some(jism) = shariha(bayt, mawdi_jism, hajm_jism) else {
            asbab.push(format!(
                "entry {fahras} points at {hajm} bytes at offset {izaha}, past the end of the \
                 container"
            ));
            continue;
        };

        let wasf_qayd = format!("{wasf} entry {fahras} ({ard_muallan}×{irtifa_muallan})");
        match hallil_jism_ayquna(jism, ard_muallan, irtifa_muallan, &wasf_qayd) {
            Ok(wahida) => hasad.push(wahida),
            Err(sabab) => asbab.push(sabab.to_string()),
        }
    }

    if hasad.is_empty() {
        let tafsil = if asbab.is_empty() {
            "no entry in it decoded".to_owned()
        } else {
            asbab.join("; ")
        };
        return Err(rafd(wasf, tafsil));
    }
    Ok(hasad)
}

/// The dimensions an `.ico` declares, read from its entry table alone.
///
/// Header-only by construction: it reads the six-byte `ICONDIR` and the
/// sixteen-byte entries, never a payload. That is what lets a library scan look
/// at hundreds of `.ico` files without decoding one of them — see
/// [`crate::fann_luba`], which is the only caller.
///
/// The size reported is the entry's own declared size, including the
/// zero-means-256 convention. An entry whose payload is a PNG whose `IHDR`
/// disagrees with the entry is malformed, and trusting the entry is what every
/// icon loader does.
#[must_use]
pub fn abaad_ico(bayt: &[u8]) -> Option<(u32, u32)> {
    if u16_saghir(bayt, 0)? != 0 || !matches!(u16_saghir(bayt, 2)?, 1 | 2) {
        return None;
    }
    let adad = usize::from(u16_saghir(bayt, 4)?).min(HADD_QUYUD_ICO);
    let mut akbar: Option<(u32, u32)> = None;
    for fahras in 0..adad {
        let mawdi = fahras
            .checked_mul(HAJM_QAYD_ICO)
            .and_then(|izaha| HAJM_TARWISAT_ICO.checked_add(izaha))?;
        let (Some(ard_bayt), Some(irtifa_bayt)) = (
            bayt_wahid(bayt, mawdi),
            bayt_wahid(bayt, mawdi.saturating_add(1)),
        ) else {
            break;
        };
        let ard = if ard_bayt == 0 {
            256
        } else {
            u32::from(ard_bayt)
        };
        let irtifa = if irtifa_bayt == 0 {
            256
        } else {
            u32::from(irtifa_bayt)
        };
        let masaha = u64::from(ard).saturating_mul(u64::from(irtifa));
        let ahsan = akbar.is_none_or(|(qadim_ard, qadim_irtifa)| {
            masaha > u64::from(qadim_ard).saturating_mul(u64::from(qadim_irtifa))
        });
        if ahsan {
            akbar = Some((ard, irtifa));
        }
    }
    akbar
}

/// One icon payload: a PNG, or a DIB with its mask.
fn hallil_jism_ayquna(
    jism: &[u8],
    ard_muallan: u32,
    irtifa_muallan: u32,
    wasf: &str,
) -> Result<AyqunaMustakhraja, KhataKashf> {
    if yabda_bi(jism, &SIHR_PNG) {
        return fukk_mushaffara(jism, wasf, HADD_BUD_AYQUNA, HADD_BAYT_AYQUNA);
    }
    fukk_dib_ayquna(jism, ard_muallan, irtifa_muallan, wasf)
}

/// A `BITMAPINFOHEADER`, in exactly the fields this module reads.
#[derive(Debug, Clone, Copy)]
struct TarwisatDib {
    /// `biSize`. Forty for `BITMAPINFOHEADER`, 108 for V4, 124 for V5. Used as
    /// the offset of the palette, so a V4 or V5 header lands the palette in the
    /// right place instead of forty bytes early.
    hajm_tarwisa: usize,
    /// `biWidth`, already checked positive.
    ard: u32,
    /// `biHeight` as written: positive means the rows are stored bottom-up, and
    /// for an icon it is twice the real height.
    irtifa_khaam: i32,
    /// `biBitCount`.
    bit_lil_biksil: u16,
    /// `biClrUsed`. Zero means "as many as the bit depth allows".
    alwan_mustakhdama: u32,
}

/// Reads and validates a DIB header, refusing every encoding this module does
/// not decode by name.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] naming the exact refusal: a
/// `BITMAPCOREHEADER`, a compression this module does not implement, a bit
/// depth outside {1, 4, 8, 24, 32}, or a non-positive width.
fn iqra_tarwisat_dib(bayt: &[u8], izaha: usize, wasf: &str) -> Result<TarwisatDib, KhataKashf> {
    let Some(hajm_khaam) = u32_saghir(bayt, izaha) else {
        return Err(rafd(wasf, "its DIB header is truncated"));
    };
    let hajm_tarwisa = usize::try_from(hajm_khaam)
        .map_err(|_| rafd(wasf, "its DIB header declares an impossible size"))?;
    if hajm_tarwisa == 12 {
        return Err(rafd(
            wasf,
            "it uses a BITMAPCOREHEADER, the OS/2 1.x layout with 16-bit dimensions and a \
             three-byte palette; no icon or splash written this century uses it",
        ));
    }
    if !(HAJM_TARWISAT_DIB..=1024).contains(&hajm_tarwisa) {
        return Err(rafd(
            wasf,
            format!("its DIB header declares a size of {hajm_tarwisa}, which is not a real one"),
        ));
    }

    let (Some(ard_khaam), Some(irtifa_khaam)) = (
        i32_saghir(bayt, izaha.saturating_add(4)),
        i32_saghir(bayt, izaha.saturating_add(8)),
    ) else {
        return Err(rafd(
            wasf,
            "its DIB header is truncated before the dimensions",
        ));
    };
    let (Some(bit_lil_biksil), Some(daght)) = (
        u16_saghir(bayt, izaha.saturating_add(14)),
        u32_saghir(bayt, izaha.saturating_add(16)),
    ) else {
        return Err(rafd(
            wasf,
            "its DIB header is truncated before the compression field",
        ));
    };
    let alwan_mustakhdama = u32_saghir(bayt, izaha.saturating_add(32)).unwrap_or(0);

    let ard = u32::try_from(ard_khaam)
        .map_err(|_| rafd(wasf, format!("its DIB declares a width of {ard_khaam}")))?;
    if ard == 0 || irtifa_khaam == 0 {
        return Err(rafd(wasf, "its DIB declares a zero dimension"));
    }

    match daght {
        // BI_RGB.
        0 => {},
        1 | 2 => {
            return Err(rafd(
                wasf,
                "it is run-length encoded (BI_RLE8/BI_RLE4); RLE appears in no icon this decoder \
                 has a reason to support and implementing it here would be a second codec",
            ));
        },
        3 => {
            return Err(rafd(
                wasf,
                "it uses BI_BITFIELDS, where the channel masks live in a block after the header \
                 rather than in a fixed layout; a decoder that assumed BGRA would read the \
                 channels in the wrong order and silently produce wrong colours",
            ));
        },
        4 => {
            return Err(rafd(
                wasf,
                "it is a BI_JPEG payload, which is not a bitmap at all",
            ));
        },
        5 => {
            return Err(rafd(
                wasf,
                "it is a BI_PNG payload inside a DIB header; a PNG icon carries its own signature \
                 and is detected before the header is read, so reaching here means the container \
                 is inconsistent",
            ));
        },
        akhar => {
            return Err(rafd(
                wasf,
                format!("it declares compression {akhar}, which no DIB layout defines"),
            ));
        },
    }

    match bit_lil_biksil {
        1 | 4 | 8 | 24 | 32 => {},
        16 => {
            return Err(rafd(
                wasf,
                "it is 16 bits per pixel, whose 5-5-5 and 5-6-5 channel layouts are only \
                 distinguishable from a BI_BITFIELDS block this decoder refuses",
            ));
        },
        akhar => {
            return Err(rafd(
                wasf,
                format!("it is {akhar} bits per pixel, which no DIB layout defines"),
            ));
        },
    }

    Ok(TarwisatDib {
        hajm_tarwisa,
        ard,
        irtifa_khaam,
        bit_lil_biksil,
        alwan_mustakhdama,
    })
}

/// Everything one DIB decode needs, gathered so the function keeps a signature a
/// person can read.
struct TalabFakk<'a> {
    /// The buffer the DIB lives in.
    bayt: &'a [u8],
    /// The validated header.
    tarwisa: TarwisatDib,
    /// Where the palette begins, which is immediately after the header.
    bidayat_lawha: usize,
    /// Where the colour data begins.
    bidayat_alwan: usize,
    /// The real output height, after the icon doubling has been undone.
    irtifa: u32,
    /// Whether the rows are stored bottom-up.
    min_asfal: bool,
    /// Whether an AND mask follows the colour data.
    maa_qina: bool,
    /// The decoded-size ceiling that applies to this image.
    aqsa_bayt: u64,
    /// The label every refusal is reported against.
    wasf: &'a str,
}

/// Decodes a DIB into straight RGBA, top row first.
///
/// ## The alpha rule, which is the whole reason this is not fifteen lines
///
/// For every depth below 32 the DIB has no alpha channel at all, and
/// transparency lives entirely in the 1-bit AND mask that follows the colour
/// data: a set bit means "let the background through". That part is
/// unambiguous.
///
/// At 32 bits the colour data *does* carry alpha, and the AND mask is still
/// present and still written — normally as all zeros, meaning fully opaque.
/// The failure this function exists to survive is the other way round: a great
/// many shipped icons carry a 32bpp BGRA image whose alpha bytes were never
/// set, so every pixel reads as fully transparent. Rendered faithfully, that
/// icon is an empty rectangle and the game's card looks broken. So the alpha
/// plane is scanned first, and when nothing in it is non-zero the AND mask is
/// used instead — and when there is no mask either, the image is taken as
/// opaque, which is the only remaining interpretation that shows anything.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the declared geometry does not fit in
/// the buffer, when the row stride overflows, or when the decoded size would
/// exceed the caller's ceiling.
fn fukk_dib(talab: &TalabFakk<'_>) -> Result<Vec<u8>, KhataKashf> {
    let ard = talab.tarwisa.ard;
    let irtifa = talab.irtifa;
    let bit = talab.tarwisa.bit_lil_biksil;
    let wasf = talab.wasf;

    let takhsis = u64::from(ard)
        .saturating_mul(u64::from(irtifa))
        .saturating_mul(4);
    if takhsis > talab.aqsa_bayt {
        return Err(rafd(
            wasf,
            format!(
                "{ard}×{irtifa} would decode to about {takhsis} bytes, over the \
                 {} byte ceiling",
                talab.aqsa_bayt
            ),
        ));
    }

    let (Ok(ard_hajmi), Ok(irtifa_hajmi)) = (usize::try_from(ard), usize::try_from(irtifa)) else {
        return Err(rafd(
            wasf,
            "its dimensions do not fit this machine's address space",
        ));
    };

    let Some(tul) = tul_satr(ard, bit) else {
        return Err(rafd(wasf, "its row stride overflows"));
    };
    let Some(tul_qina) = tul_satr(ard, 1) else {
        return Err(rafd(wasf, "its mask row stride overflows"));
    };
    let Some(hajm_alwan) = tul.checked_mul(irtifa_hajmi) else {
        return Err(rafd(wasf, "its colour plane overflows"));
    };
    if shariha(talab.bayt, talab.bidayat_alwan, hajm_alwan).is_none() {
        return Err(rafd(
            wasf,
            format!(
                "its colour plane needs {hajm_alwan} bytes from offset {}, and the payload is \
                 {} bytes long",
                talab.bidayat_alwan,
                talab.bayt.len()
            ),
        ));
    }

    let lawha = iqra_lawha(talab);

    let bidayat_qina = talab.bidayat_alwan.checked_add(hajm_alwan);
    let hajm_qina = tul_qina.checked_mul(irtifa_hajmi);
    let qina = match (talab.maa_qina, bidayat_qina, hajm_qina) {
        (true, Some(bidaya), Some(hajm)) => {
            shariha(talab.bayt, bidaya, hajm).map(|_| (bidaya, tul_qina))
        },
        _ => None,
    };

    // The pre-pass described above. Only 32bpp can be affected, and only a full
    // scan can distinguish "deliberately transparent everywhere" from "alpha was
    // never written" — which is the same bit pattern.
    let alfa_min_qina = bit == 32
        && alfa_khaliya(
            talab.bayt,
            talab.bidayat_alwan,
            tul,
            ard_hajmi,
            irtifa_hajmi,
        );

    let Ok(sia) = usize::try_from(takhsis) else {
        return Err(rafd(
            wasf,
            "its decoded size does not fit this machine's address space",
        ));
    };
    let mut kharij: Vec<u8> = Vec::with_capacity(sia);

    for saf in 0..irtifa_hajmi {
        let saf_masdar = if talab.min_asfal {
            irtifa_hajmi.saturating_sub(1).saturating_sub(saf)
        } else {
            saf
        };
        let Some(mawdi_satr) = saf_masdar
            .checked_mul(tul)
            .and_then(|izaha| talab.bidayat_alwan.checked_add(izaha))
        else {
            return Err(rafd(wasf, "its row offset overflows"));
        };
        let Some(satr) = shariha(talab.bayt, mawdi_satr, tul) else {
            return Err(rafd(
                wasf,
                format!("row {saf_masdar} is not inside the payload"),
            ));
        };

        let satr_qina = qina.and_then(|(bidaya, tul_q)| {
            saf_masdar
                .checked_mul(tul_q)
                .and_then(|izaha| bidaya.checked_add(izaha))
                .and_then(|mawdi| shariha(talab.bayt, mawdi, tul_q))
        });

        for x in 0..ard_hajmi {
            let Some((ahmar, akhdar, azraq, alfa_khaam)) = biksil_min_satr(satr, x, bit, &lawha)
            else {
                return Err(rafd(
                    wasf,
                    format!("pixel {x} of row {saf_masdar} is truncated"),
                ));
            };
            // A set mask bit means transparent. When the mask is absent the
            // pixel keeps whatever alpha its own encoding gave it, which for
            // every depth below 32 is opaque.
            let mashfuf = satr_qina.and_then(|q| bit_qina(q, x)).unwrap_or(false);
            let alfa = if bit == 32 && !alfa_min_qina {
                alfa_khaam
            } else if mashfuf {
                0
            } else {
                0xFF
            };
            kharij.push(ahmar);
            kharij.push(akhdar);
            kharij.push(azraq);
            kharij.push(alfa);
        }
    }

    Ok(kharij)
}

/// The DIB inside an icon, whose height is doubled and whose mask is mandatory.
///
/// `ard_muallan` and `irtifa_muallan` come from the `ICONDIRENTRY`, which is the
/// only place the *real* height is recorded. Zero for both means the caller has
/// no entry to consult — the fallback path that decodes a bare `RT_ICON` — and
/// the doubling is then inferred from `biHeight` alone.
///
/// # Errors
///
/// Every refusal of [`iqra_tarwisat_dib`] and [`fukk_dib`], plus one of its own:
/// a declared side over [`HADD_BUD_AYQUNA`].
fn fukk_dib_ayquna(
    jism: &[u8],
    ard_muallan: u32,
    irtifa_muallan: u32,
    wasf: &str,
) -> Result<AyqunaMustakhraja, KhataKashf> {
    let tarwisa = iqra_tarwisat_dib(jism, 0, wasf)?;

    let min_asfal = tarwisa.irtifa_khaam > 0;
    let mutlaq = tarwisa
        .irtifa_khaam
        .checked_abs()
        .and_then(|qeema| u32::try_from(qeema).ok())
        .ok_or_else(|| rafd(wasf, "its DIB height has no absolute value"))?;

    // Three ways to land on the real height, in order of how much the source
    // is trusted. The entry's declared height wins when it agrees with either
    // reading of `biHeight`, because it is the only field written by something
    // that knew what the image was.
    let (irtifa, maa_qina) = if irtifa_muallan > 0 && mutlaq == irtifa_muallan.saturating_mul(2) {
        (irtifa_muallan, true)
    } else if irtifa_muallan > 0 && mutlaq == irtifa_muallan {
        (mutlaq, false)
    } else if mutlaq & 1 == 0 {
        // `>> 1` rather than `/ 2`: this workspace denies integer division, and
        // halving a known-even value is a shift in every sense.
        (mutlaq >> 1, true)
    } else {
        (mutlaq, false)
    };

    if tarwisa.ard > HADD_BUD_AYQUNA || irtifa > HADD_BUD_AYQUNA {
        return Err(rafd(
            wasf,
            format!(
                "it declares {}×{irtifa}, over the {HADD_BUD_AYQUNA} pixel icon limit",
                tarwisa.ard
            ),
        ));
    }
    if ard_muallan > 0 && tarwisa.ard != ard_muallan {
        // Not fatal. A disagreement here means the directory and the image were
        // written by different tools; the image's own header is what the pixels
        // actually follow, so it wins and the entry is treated as a label.
        tracing::debug!(
            wasf,
            muallan = ard_muallan,
            fili = tarwisa.ard,
            "an icon entry's declared width disagrees with its own DIB header"
        );
    }

    let bidayat_lawha = tarwisa.hajm_tarwisa;
    let bidayat_alwan = bidayat_lawha.saturating_add(hajm_lawha(&tarwisa));

    let biksilat = fukk_dib(&TalabFakk {
        bayt: jism,
        tarwisa,
        bidayat_lawha,
        bidayat_alwan,
        irtifa,
        min_asfal,
        maa_qina,
        aqsa_bayt: HADD_BAYT_AYQUNA,
        wasf,
    })?;

    Ok(AyqunaMustakhraja {
        ard: tarwisa.ard,
        irtifa,
        biksilat,
        wasf: wasf.to_owned(),
    })
}

/// A standalone `.bmp` file, decoded to RGBA.
///
/// This exists because the workspace builds `image` with `png`, `jpeg` and
/// `webp` only, and Unreal ships its splash screen as `Content/Splash/Splash.bmp`.
/// Without this the one artwork file a large family of games reliably has would
/// be unreadable.
///
/// Differs from the icon path in three ways, all of them the reason it is a
/// separate function: there is a fourteen-byte `BITMAPFILEHEADER` in front,
/// `biHeight` is the real height with no doubling, and there is no AND mask.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the file does not begin with `BM`, when
/// the DIB header is one this module refuses — run-length encoded,
/// `BI_BITFIELDS`, 16 bits per pixel, or an OS/2 `BITMAPCOREHEADER` — or when
/// the declared geometry does not fit the file.
pub fn bmp_ila_rgba(bayt: &[u8], wasf: &str) -> Result<AyqunaMustakhraja, KhataKashf> {
    if !yabda_bi(bayt, b"BM") {
        return Err(rafd(wasf, "it does not begin with the BM signature"));
    }
    let tarwisa = iqra_tarwisat_dib(bayt, HAJM_TARWISAT_BMP, wasf)?;

    let min_asfal = tarwisa.irtifa_khaam > 0;
    let irtifa = tarwisa
        .irtifa_khaam
        .checked_abs()
        .and_then(|qeema| u32::try_from(qeema).ok())
        .ok_or_else(|| rafd(wasf, "its height has no absolute value"))?;

    if tarwisa.ard > HADD_BUD_SURA || irtifa > HADD_BUD_SURA {
        return Err(rafd(
            wasf,
            format!(
                "it declares {}×{irtifa}, over the {HADD_BUD_SURA} pixel limit",
                tarwisa.ard
            ),
        ));
    }

    let bidayat_lawha = HAJM_TARWISAT_BMP.saturating_add(tarwisa.hajm_tarwisa);
    let mahsub = bidayat_lawha.saturating_add(hajm_lawha(&tarwisa));
    // `bfOffBits` is authoritative when it is inside the file: writers pad
    // between the palette and the pixels, and a computed offset would land in
    // that padding. It is ignored when it is nonsense, which is what a
    // hand-truncated file looks like.
    let muallan = u32_saghir(bayt, 10)
        .and_then(|qeema| usize::try_from(qeema).ok())
        .filter(|qeema| *qeema >= mahsub && *qeema < bayt.len());
    let bidayat_alwan = muallan.unwrap_or(mahsub);

    let biksilat = fukk_dib(&TalabFakk {
        bayt,
        tarwisa,
        bidayat_lawha,
        bidayat_alwan,
        irtifa,
        min_asfal,
        maa_qina: false,
        aqsa_bayt: HADD_BAYT_SURA,
        wasf,
    })?;

    Ok(AyqunaMustakhraja {
        ard: tarwisa.ard,
        irtifa,
        biksilat,
        wasf: wasf.to_owned(),
    })
}

/// The dimensions a `.bmp` declares, read from its header alone.
///
/// Handles the `BITMAPCOREHEADER` case that [`iqra_tarwisat_dib`] refuses,
/// because measuring one is free and refusing a candidate for its dimensions is
/// a different decision from refusing to decode it.
#[must_use]
pub fn abaad_bmp(bayt: &[u8]) -> Option<(u32, u32)> {
    if !yabda_bi(bayt, b"BM") {
        return None;
    }
    let hajm_tarwisa = u32_saghir(bayt, HAJM_TARWISAT_BMP)?;
    let (ard, irtifa) = if hajm_tarwisa == 12 {
        // OS/2 1.x: two *unsigned* 16-bit dimensions instead of two signed
        // 32-bit ones, and consequently no top-down row order to represent.
        (
            u32::from(u16_saghir(bayt, 18)?),
            u32::from(u16_saghir(bayt, 20)?),
        )
    } else {
        let ard = u32::try_from(i32_saghir(bayt, 18)?).ok()?;
        let irtifa = u32::try_from(i32_saghir(bayt, 22)?.checked_abs()?).ok()?;
        (ard, irtifa)
    };
    (ard > 0 && irtifa > 0).then_some((ard, irtifa))
}

/// The palette, as RGB triples in index order.
///
/// An index past the end of a short palette resolves to opaque black at lookup
/// time rather than failing the decode. A truncated palette is a damaged file,
/// but the pixels it does describe are still the game's icon, and a black
/// speck is a better outcome than no artwork.
fn iqra_lawha(talab: &TalabFakk<'_>) -> Vec<[u8; 3]> {
    let bit = talab.tarwisa.bit_lil_biksil;
    if bit > 8 {
        return Vec::new();
    }
    let aqsa = 1_u32 << bit;
    let adad = if talab.tarwisa.alwan_mustakhdama == 0 {
        aqsa
    } else {
        talab.tarwisa.alwan_mustakhdama.min(aqsa)
    };
    let adad = usize::try_from(adad).unwrap_or(0);

    let mut lawha = Vec::with_capacity(adad);
    for fahras in 0..adad {
        let Some(mawdi) = fahras
            .checked_mul(4)
            .and_then(|izaha| talab.bidayat_lawha.checked_add(izaha))
        else {
            break;
        };
        let Some(qayd) = shariha(talab.bayt, mawdi, 4) else {
            break;
        };
        let Ok([azraq, akhdar, ahmar, _]) = <[u8; 4]>::try_from(qayd) else {
            break;
        };
        lawha.push([ahmar, akhdar, azraq]);
    }
    lawha
}

/// How many bytes the palette occupies, which is where the colour data starts.
fn hajm_lawha(tarwisa: &TarwisatDib) -> usize {
    if tarwisa.bit_lil_biksil > 8 {
        return 0;
    }
    let aqsa = 1_u32 << tarwisa.bit_lil_biksil;
    let adad = if tarwisa.alwan_mustakhdama == 0 {
        aqsa
    } else {
        tarwisa.alwan_mustakhdama.min(aqsa)
    };
    usize::try_from(adad).unwrap_or(0).saturating_mul(4)
}

/// The padded length of one row, in bytes.
///
/// Every DIB row is padded up to a four-byte boundary. `(bits + 31) >> 5 << 2`
/// is that rounding written as shifts, because this workspace denies integer
/// division and because the arithmetic is exactly a round-up to whole 32-bit
/// words.
fn tul_satr(ard: u32, bit: u16) -> Option<usize> {
    let bitat = u64::from(ard).checked_mul(u64::from(bit))?;
    let kalimat = bitat.checked_add(31)? >> 5;
    usize::try_from(kalimat.checked_mul(4)?).ok()
}

/// One pixel out of a decoded row, in whichever depth the DIB uses.
///
/// Returns `(red, green, blue, alpha)`. DIB channel order is BGR, so every arm
/// destructures rather than indexes and reverses as it goes; a slice pattern is
/// also the only way to read three or four consecutive bytes without tripping
/// the workspace's ban on indexing.
fn biksil_min_satr(satr: &[u8], x: usize, bit: u16, lawha: &[[u8; 3]]) -> Option<(u8, u8, u8, u8)> {
    match bit {
        32 => {
            let qita = shariha(satr, x.checked_mul(4)?, 4)?;
            let [azraq, akhdar, ahmar, alfa] = <[u8; 4]>::try_from(qita).ok()?;
            Some((ahmar, akhdar, azraq, alfa))
        },
        24 => {
            let qita = shariha(satr, x.checked_mul(3)?, 3)?;
            let [azraq, akhdar, ahmar] = <[u8; 3]>::try_from(qita).ok()?;
            Some((ahmar, akhdar, azraq, 0xFF))
        },
        8 => {
            let fahras = usize::from(*satr.get(x)?);
            let [ahmar, akhdar, azraq] = lawha.get(fahras).copied().unwrap_or([0, 0, 0]);
            Some((ahmar, akhdar, azraq, 0xFF))
        },
        4 => {
            let zawj = *satr.get(x >> 1)?;
            // The high nibble is the even pixel, the low nibble the odd one.
            let fahras = usize::from(if x & 1 == 0 { zawj >> 4 } else { zawj & 0x0F });
            let [ahmar, akhdar, azraq] = lawha.get(fahras).copied().unwrap_or([0, 0, 0]);
            Some((ahmar, akhdar, azraq, 0xFF))
        },
        1 => {
            let thumn = *satr.get(x >> 3)?;
            let izaha = 7_u32.saturating_sub(u32::try_from(x & 7).ok()?);
            let fahras = usize::from((thumn >> izaha) & 1);
            let [ahmar, akhdar, azraq] = lawha.get(fahras).copied().unwrap_or([0, 0, 0]);
            Some((ahmar, akhdar, azraq, 0xFF))
        },
        _ => None,
    }
}

/// One bit out of an AND mask row. A set bit means the pixel is transparent.
fn bit_qina(satr: &[u8], x: usize) -> Option<bool> {
    let thumn = *satr.get(x >> 3)?;
    let izaha = 7_u32.saturating_sub(u32::try_from(x & 7).ok()?);
    Some((thumn >> izaha) & 1 == 1)
}

/// Whether a 32bpp colour plane has no opacity anywhere.
///
/// Returns `true` for a truncated plane as well, which is the conservative
/// answer: a row that cannot be read cannot be shown to contain opacity, and
/// falling back to the AND mask for a damaged image is better than rendering it
/// entirely invisible.
fn alfa_khaliya(bayt: &[u8], bidaya: usize, tul: usize, ard: usize, irtifa: usize) -> bool {
    for saf in 0..irtifa {
        let Some(mawdi) = saf
            .checked_mul(tul)
            .and_then(|izaha| bidaya.checked_add(izaha))
        else {
            return true;
        };
        let Some(satr) = shariha(bayt, mawdi, tul) else {
            return true;
        };
        for x in 0..ard {
            let Some(mawdi_alfa) = x.checked_mul(4).and_then(|izaha| izaha.checked_add(3)) else {
                return true;
            };
            if satr.get(mawdi_alfa).is_some_and(|qeema| *qeema != 0) {
                return false;
            }
        }
    }
    true
}

/// Decodes bytes in one of the three formats `image` is built with here.
///
/// Dimensions are read from the header and checked *before* the decoder is
/// given the bytes, because the check exists to prevent an allocation and an
/// allocation that already happened cannot be prevented. The decoder is then
/// additionally held to its own [`Limits`], in case a format ever lets a header
/// understate what it expands to.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the bytes match no format, match one
/// this build does not decode, declare dimensions past the caller's ceiling, or
/// fail to decode.
fn fukk_mushaffara(
    bayt: &[u8],
    wasf: &str,
    aqsa_bud: u32,
    aqsa_bayt: u64,
) -> Result<AyqunaMustakhraja, KhataKashf> {
    let qari = ImageReader::new(Cursor::new(bayt))
        .with_guessed_format()
        .map_err(|sabab| rafd(wasf, sabab.to_string()))?;
    let sigha = qari
        .format()
        .ok_or_else(|| rafd(wasf, "the bytes match no image format"))?;
    if !matches!(
        sigha,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(rafd(
            wasf,
            format!("{sigha:?} is not one of the three formats this build decodes"),
        ));
    }

    let (ard, irtifa) = qari
        .into_dimensions()
        .map_err(|sabab| rafd(wasf, sabab.to_string()))?;
    let takhsis = u64::from(ard)
        .saturating_mul(u64::from(irtifa))
        .saturating_mul(4);
    if ard == 0 || irtifa == 0 || ard > aqsa_bud || irtifa > aqsa_bud || takhsis > aqsa_bayt {
        return Err(rafd(
            wasf,
            format!(
                "it declares {ard}×{irtifa}, which would allocate about {takhsis} bytes before a \
                 single pixel is examined"
            ),
        ));
    }

    let mut qari = ImageReader::new(Cursor::new(bayt))
        .with_guessed_format()
        .map_err(|sabab| rafd(wasf, sabab.to_string()))?;
    let mut hudud = Limits::no_limits();
    hudud.max_image_width = Some(aqsa_bud);
    hudud.max_image_height = Some(aqsa_bud);
    hudud.max_alloc = Some(aqsa_bayt);
    qari.limits(hudud);

    let sura = qari
        .decode()
        .map_err(|sabab| rafd(wasf, sabab.to_string()))?;
    let biksilat = sura.to_rgba8();
    Ok(AyqunaMustakhraja {
        ard: biksilat.width(),
        irtifa: biksilat.height(),
        biksilat: biksilat.into_raw(),
        wasf: wasf.to_owned(),
    })
}

// ---------------------------------------------------------------------------
// Mach-O — the icon that is not in the binary
// ---------------------------------------------------------------------------
//
// A Mach-O executable carries no icon. macOS applications are directories, and
// the icon is a file inside one:
//
//   Foo.app/
//     Contents/
//       Info.plist                 <key>CFBundleIconFile</key><string>AppIcon</string>
//       MacOS/Foo                  the Mach-O this module was handed
//       Resources/AppIcon.icns     the actual artwork
//
// So the binary is parsed only far enough to confirm what it is, and everything
// after that is a walk up to the bundle root and two file reads.
//
// The ICNS container itself:
//
//   'icns'                         4 bytes
//   u32 be                         total length, including these eight bytes
//   then, repeated:
//     [4-byte type][u32 be length including this 8-byte header][payload]
//
// Every modern icon type carries a whole PNG as its payload, which is why this
// module can support the sizes that matter without a second image codec.

/// Every icon a macOS application bundle holds, reached from its executable.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the binary is not really a Mach-O, when
/// no `.app` bundle encloses it, or for any refusal of [`ayqunat_hazma`].
fn ayqunat_mach(
    masar: &Path,
    malaf: File,
    wasf: &str,
) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    let makhzan = ReadCache::new(malaf);
    let naw = FileKind::parse(&makhzan)
        .map_err(|sabab| rafd(wasf, format!("its Mach-O header does not parse: {sabab}")))?;
    if !matches!(
        naw,
        FileKind::MachO32 | FileKind::MachO64 | FileKind::MachOFat32 | FileKind::MachOFat64
    ) {
        return Err(rafd(
            wasf,
            format!(
                "its magic is one Mach-O shares with other formats and it turned out to be \
                 {naw:?}; 0xCAFEBABE is also a Java class file"
            ),
        ));
    }

    let hazma = hazmat_min_tanfidhi(masar).ok_or_else(|| {
        rafd(
            wasf,
            "it is a Mach-O binary with no .app bundle above it, and a bare Mach-O carries no \
             icon of its own — the icon of a macOS application is a file in its bundle",
        )
    })?;
    ayqunat_hazma(&hazma, wasf)
}

/// The `.app` directory enclosing an executable, if there is one.
///
/// Four levels is exactly what the layout needs — `Foo.app/Contents/MacOS/Foo`
/// is three — with one spare for a bundle that nests a helper binary one deeper.
/// A directory is accepted either because it is named `*.app` or because it
/// actually contains `Contents/Info.plist`, so a bundle whose extension was
/// stripped by an archiver is still found.
fn hazmat_min_tanfidhi(masar: &Path) -> Option<PathBuf> {
    masar
        .ancestors()
        .skip(1)
        .take(4)
        .find(|jadd| {
            let musamma = jadd
                .extension()
                .and_then(|imtidad| imtidad.to_str())
                .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("app"));
            musamma || jadd.join("Contents").join("Info.plist").is_file()
        })
        .map(Path::to_path_buf)
}

/// Every icon inside a macOS application bundle.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when `Contents/Info.plist` is missing or
/// unreadable, when it is a binary property list, when it names no icon and no
/// `.icns` is present, when the named icon is not in `Contents/Resources`, or
/// for any refusal of [`hallil_icns`].
fn ayqunat_hazma(hazma: &Path, wasf: &str) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    let mawarid = hazma.join("Contents").join("Resources");
    let plist = hazma.join("Contents").join("Info.plist");

    let ism = match ism_ayquna_min_plist(&plist, wasf) {
        Ok(ism) => Some(ism),
        Err(sabab) => {
            // A bundle with a damaged or absent Info.plist still usually has
            // exactly one .icns sitting in Resources, and taking it is better
            // than reporting nothing. The plist's own reason is kept so the
            // trail says why the direct route failed.
            tracing::debug!(hazma = %hazma.display(), sabab = %sabab, "falling back to a scan \
                of Contents/Resources for an .icns");
            None
        },
    };

    let masar_ayquna = match ism {
        Some(ism) => {
            let ma_imtidad = if Path::new(&ism)
                .extension()
                .and_then(|imtidad| imtidad.to_str())
                .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("icns"))
            {
                ism
            } else {
                // `CFBundleIconFile` is documented as being allowed to omit the
                // extension, and most bundles do omit it.
                format!("{ism}.icns")
            };
            let murashah = mawarid.join(&ma_imtidad);
            if murashah.is_file() {
                Some(murashah)
            } else {
                awwal_icns(&mawarid)
            }
        },
        None => awwal_icns(&mawarid),
    };

    let masar_ayquna = masar_ayquna.ok_or_else(|| {
        rafd(
            wasf,
            format!(
                "its bundle holds no .icns file under {}; a bundle whose icon lives only in a \
                 compiled Assets.car asset catalogue has no icon this module can read",
                mawarid.display()
            ),
        )
    })?;

    let bayt = iqra_bi_hadd(&masar_ayquna, HADD_HAJM_HAWIYA, wasf)?;
    hallil_icns(&bayt, &format!("{wasf}: {}", masar_ayquna.display()))
}

/// The first `.icns` in a bundle's `Resources`, by a bounded directory read.
fn awwal_icns(mawarid: &Path) -> Option<PathBuf> {
    let mudakhalat = std::fs::read_dir(mawarid).ok()?;
    for mudkhal in mudakhalat.take(HADD_MUDAKHALAT_MUJALLAD).flatten() {
        let masar = mudkhal.path();
        let hiya = masar
            .extension()
            .and_then(|imtidad| imtidad.to_str())
            .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("icns"));
        if hiya && masar.is_file() {
            return Some(masar);
        }
    }
    None
}

/// Reads a file with a byte ceiling.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the file cannot be opened or read, or
/// when it is empty.
fn iqra_bi_hadd(masar: &Path, hadd: u64, wasf: &str) -> Result<Vec<u8>, KhataKashf> {
    let malaf = File::open(masar)
        .map_err(|sabab| rafd(wasf, format!("cannot open {}: {sabab}", masar.display())))?;
    let mut bayt = Vec::new();
    let _ = malaf
        .take(hadd)
        .read_to_end(&mut bayt)
        .map_err(|sabab| rafd(wasf, format!("cannot read {}: {sabab}", masar.display())))?;
    if bayt.is_empty() {
        return Err(rafd(wasf, format!("{} is empty", masar.display())));
    }
    Ok(bayt)
}

/// `CFBundleIconFile` out of an `Info.plist`.
///
/// ## Why the binary form is refused rather than guessed at
///
/// About half of shipped bundles carry `Info.plist` as a binary property list —
/// an offset table, a trailer, and a typed object graph with variable-width
/// integers. It is a whole second format, and a parser that guessed at it would
/// either be a third of this file or would be wrong. The refusal names the
/// format so the trail says `binary property list` instead of `no icon`, and
/// the caller falls back to scanning `Resources` for an `.icns`, which is where
/// the answer would have led anyway.
///
/// ## Why only top-level keys count
///
/// `CFBundleIconFile` is defined at the root dictionary. The same string
/// appears inside `CFBundleDocumentTypes` entries, where it names the icon for
/// a *document* type rather than for the application, and a reader that took
/// the first match anywhere would return a file-format icon for a game.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the file cannot be read, is a binary
/// property list, is not well-formed XML, or declares no `CFBundleIconFile`.
fn ism_ayquna_min_plist(masar: &Path, wasf: &str) -> Result<String, KhataKashf> {
    let bayt = iqra_bi_hadd(masar, HADD_HAJM_PLIST, wasf)?;
    if yabda_bi(&bayt, &SIHR_BPLIST) {
        return Err(rafd(
            wasf,
            "its Info.plist is a binary property list (bplist00), which is a different format \
             from the XML one and is not parsed here",
        ));
    }

    let nass = String::from_utf8_lossy(&bayt);
    let nass = nass
        .strip_prefix('\u{feff}')
        .unwrap_or_else(|| nass.as_ref());

    let mut qari = quick_xml::Reader::from_str(nass);
    qari.config_mut().trim_text(true);

    let mut umq = 0_usize;
    let mut fi_miftah = false;
    let mut fi_qeema = false;
    let mut miftah = String::new();
    let mut qeema = String::new();
    let mut bil_ism: Option<String> = None;

    loop {
        match qari.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(marka)) => match marka.local_name().as_ref() {
                "dict" => umq = umq.saturating_add(1),
                "key" if umq == 1 => {
                    fi_miftah = true;
                    miftah.clear();
                },
                "string" if umq == 1 => {
                    fi_qeema = true;
                    qeema.clear();
                },
                _ => {},
            },
            Ok(quick_xml::events::Event::Text(nass_marka)) => {
                if fi_miftah || fi_qeema {
                    let qita = nass_marka.xml10_content();
                    if fi_miftah {
                        miftah.push_str(qita.as_ref());
                    } else {
                        qeema.push_str(qita.as_ref());
                    }
                }
            },
            // A reference is a separate event, not part of the text around it,
            // so a reader that matched only on text would drop every `&amp;`.
            Ok(quick_xml::events::Event::GeneralRef(marja)) => {
                if fi_miftah || fi_qeema {
                    let qita = taarib_usus::kayanat::hall_marja(&marja)
                        .map_or_else(|| taarib_usus::kayanat::nass_marja(&marja), Cow::into_owned);
                    if fi_miftah {
                        miftah.push_str(&qita);
                    } else {
                        qeema.push_str(&qita);
                    }
                }
            },
            Ok(quick_xml::events::Event::End(marka)) => match marka.local_name().as_ref() {
                "dict" => umq = umq.saturating_sub(1),
                "key" => fi_miftah = false,
                "string" => {
                    fi_qeema = false;
                    let mahsub = qeema.trim().to_owned();
                    match miftah.trim() {
                        "CFBundleIconFile" if !mahsub.is_empty() => return Ok(mahsub),
                        "CFBundleIconName" if bil_ism.is_none() && !mahsub.is_empty() => {
                            bil_ism = Some(mahsub);
                        },
                        _ => {},
                    }
                    miftah.clear();
                },
                _ => {},
            },
            Ok(_) => {},
            Err(sabab) => {
                return Err(rafd(
                    wasf,
                    format!(
                        "its Info.plist is not well-formed XML at byte {}: {sabab}",
                        qari.buffer_position()
                    ),
                ));
            },
        }
    }

    // `CFBundleIconName` names an image *set* inside a compiled asset catalogue
    // (`Assets.car`), not a file. There is nothing on disk to open.
    if let Some(ism) = bil_ism {
        return Err(rafd(
            wasf,
            format!(
                "its Info.plist names CFBundleIconName={ism} and no CFBundleIconFile, so the \
                 icon lives inside a compiled Assets.car asset catalogue rather than as a file"
            ),
        ));
    }
    Err(rafd(wasf, "its Info.plist declares no CFBundleIconFile"))
}

/// Every PNG-bearing image inside an ICNS container.
///
/// The legacy chunk types are recognised and refused by name rather than
/// ignored. They are run-length-encoded RGB planes whose alpha lives in a
/// separate `*8mk` chunk, or raw ARGB blocks — a second image codec, written for
/// icons superseded in Mac OS X 10.7, on bundles that in practice also carry a
/// modern chunk. Naming them is what lets the trail say `it32 (legacy RLE)`
/// instead of `no icon`.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the signature or length field is
/// missing, or when no chunk yielded an image — carrying every chunk's own
/// reason, joined.
pub fn hallil_icns(bayt: &[u8], wasf: &str) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    if !yabda_bi(bayt, &SIHR_ICNS) {
        return Err(rafd(wasf, "it does not begin with the icns signature"));
    }
    let tul_muallan =
        u32_kabir(bayt, 4).ok_or_else(|| rafd(wasf, "its length field is truncated"))?;
    // The declared length is a claim; the buffer is the fact. Taking the
    // smaller means a truncated download reads what is there instead of
    // walking off the end, and a padded file stops where it said it would.
    let nihaya = usize::try_from(tul_muallan)
        .unwrap_or(usize::MAX)
        .min(bayt.len());

    let mut hasad: Vec<AyqunaMustakhraja> = Vec::new();
    let mut asbab: Vec<String> = Vec::new();
    let mut mawdi = 8_usize;

    for _ in 0..HADD_QITA_ICNS {
        if mawdi >= nihaya || hasad.len() >= HADD_AYQUNAT_MURJAA {
            break;
        }
        let (Some(naw), Some(hajm_khaam)) = (
            wasm_arbaa(bayt, mawdi),
            u32_kabir(bayt, mawdi.saturating_add(4)),
        ) else {
            break;
        };
        let Ok(hajm) = usize::try_from(hajm_khaam) else {
            break;
        };
        // The length includes the eight-byte header, so anything below eight is
        // a length that cannot advance the cursor — the encoding of a loop.
        let Some(hajm_himl) = hajm.checked_sub(8) else {
            break;
        };
        let Some(himl) = shariha(bayt, mawdi.saturating_add(8), hajm_himl) else {
            break;
        };
        let ism_naw = String::from_utf8_lossy(&naw).into_owned();

        if let Some((_, qiyas)) = ANWA_ICNS_PNG.iter().find(|(wasm, _)| *wasm == naw) {
            let wasf_qita = format!("{wasf}: ICNS {ism_naw} (nominally {qiyas}×{qiyas})");
            if yabda_bi(himl, &SIHR_PNG) {
                match fukk_mushaffara(himl, &wasf_qita, HADD_BUD_AYQUNA, HADD_BAYT_AYQUNA) {
                    Ok(wahida) => hasad.push(wahida),
                    Err(sabab) => asbab.push(sabab.to_string()),
                }
            } else if yabda_bi(himl, &SIHR_JP2) {
                asbab.push(format!(
                    "{ism_naw} carries a JPEG 2000 codestream, which this build has no decoder \
                     for and which macOS stopped writing after 10.7"
                ));
            } else {
                asbab.push(format!(
                    "{ism_naw} carries neither a PNG nor a JPEG 2000 payload, so it is a raw \
                     ARGB block this module does not decode"
                ));
            }
        } else if ANWA_ICNS_QADIMA.contains(&naw) {
            asbab.push(format!(
                "{ism_naw} is a legacy chunk: a run-length-encoded RGB plane whose alpha lives \
                 in a separate mask chunk, which is a second codec and is not implemented"
            ));
        } else if naw == *b"ic04" || naw == *b"ic05" {
            asbab.push(format!(
                "{ism_naw} is a raw or run-length-encoded ARGB block, which is not implemented"
            ));
        }
        // Everything else — `TOC `, `icnV`, `name`, `info`, and any type a
        // future macOS adds — is skipped in silence. A container is allowed to
        // hold metadata, and reporting each metadata chunk as a failure would
        // bury the one line that matters.

        let Some(taali) = mawdi.checked_add(hajm) else {
            break;
        };
        mawdi = taali;
    }

    if hasad.is_empty() {
        let tafsil = if asbab.is_empty() {
            "it holds no icon chunk this build decodes".to_owned()
        } else {
            asbab.join("; ")
        };
        return Err(rafd(wasf, tafsil));
    }
    Ok(hasad)
}

// ---------------------------------------------------------------------------
// ELF — the format that carries no icon, and what to do about it
// ---------------------------------------------------------------------------
//
// An ELF binary has no resource section, no icon segment, and no convention for
// embedding one. There is nothing to parse. Saying so plainly is better than a
// function that pretends to look and always returns nothing.
//
// What a Linux application's icon actually is:
//
//   * a `.desktop` file — an INI-like entry file — whose `Icon=` key holds
//     either an absolute path or a *name*;
//   * that name resolved against the icon theme directories, which are
//     `$XDG_DATA_HOME/icons` and every `$XDG_DATA_DIRS` entry's `icons`
//     subdirectory, laid out as `<theme>/<size>/apps/<name>.png`;
//   * or, for a game that never installed itself, a `.png` sitting beside the
//     executable, which is what itch.io and GOG's own Linux archives ship.
//
// All three are tried, and none of them is a guess.
//
// SVG is refused. There is no vector rasterizer anywhere in this workspace,
// writing one is a project rather than a function, and an SVG renamed to `.png`
// would fail in the decoder rather than here. The refusal is named so a
// diagnostics trail says `scalable/apps/celeste.svg, refused: no rasterizer`
// instead of leaving the reader to wonder why an icon that plainly exists was
// not used.

/// Every icon a Linux game's own files point at.
///
/// The ELF itself is parsed only to confirm that it is one. Everything that
/// follows is filesystem work, and on a host that is not Linux none of the
/// theme directories exist, so the sibling-file probes are what remain — which
/// is correct rather than degraded: those are the ones a portable game archive
/// actually ships.
///
/// # Errors
///
/// [`KhataKashf::SuraGhayrSaliha`] when the ELF header does not parse, or when
/// no candidate decoded — carrying every refusal, joined, including the SVG and
/// theme-lookup ones, so the trail names what was found and rejected.
fn ayqunat_elf(
    masar: &Path,
    malaf: File,
    wasf: &str,
    judhur: &[PathBuf],
) -> Result<Vec<AyqunaMustakhraja>, KhataKashf> {
    let makhzan = ReadCache::new(malaf);
    let naw = FileKind::parse(&makhzan)
        .map_err(|sabab| rafd(wasf, format!("its ELF header does not parse: {sabab}")))?;
    if !matches!(naw, FileKind::Elf32 | FileKind::Elf64) {
        return Err(rafd(
            wasf,
            format!("it begins with \\x7FELF but is {naw:?}"),
        ));
    }

    let mut asbab: Vec<String> = Vec::new();
    let mut murashahat: Vec<(PathBuf, String)> = Vec::new();

    murashahat.extend(murashahat_mujawira(masar, &mut asbab));

    for desktop in masarat_desktop(masar, judhur) {
        let Ok(nass) = std::fs::read_to_string(&desktop) else {
            continue;
        };
        if nass.len() > usize::try_from(HADD_HAJM_MAQATI).unwrap_or(usize::MAX) {
            asbab.push(format!(
                "{} is too large to be an entry file",
                desktop.display()
            ));
            continue;
        }
        let Some(ism) = ayquna_min_desktop(&nass) else {
            continue;
        };
        let (min_sima, asbab_sima) = hall_ayqunat_sima(&ism, judhur);
        asbab.extend(asbab_sima);
        let min_ayn = desktop.display().to_string();
        for (masar_sima, wasf_sima) in min_sima {
            murashahat.push((masar_sima, format!("{min_ayn} Icon={ism} → {wasf_sima}")));
        }
    }

    let mut hasad: Vec<AyqunaMustakhraja> = Vec::new();
    for (murashah, wasf_murashah) in murashahat {
        if hasad.len() >= HADD_AYQUNAT_MURJAA {
            break;
        }
        let wasf_kamil = format!("{wasf}: {wasf_murashah}");
        match iqra_bi_hadd(&murashah, HADD_HAJM_HAWIYA, &wasf_kamil) {
            Ok(bayt) => match fukk_mushaffara(&bayt, &wasf_kamil, HADD_BUD_SURA, HADD_BAYT_SURA) {
                Ok(wahida) => hasad.push(wahida),
                Err(sabab) => asbab.push(sabab.to_string()),
            },
            Err(sabab) => asbab.push(sabab.to_string()),
        }
    }

    if hasad.is_empty() {
        let tafsil = if asbab.is_empty() {
            "an ELF binary carries no icon, and no .desktop entry or sibling image points at one"
                .to_owned()
        } else {
            asbab.join("; ")
        };
        return Err(rafd(wasf, tafsil));
    }
    Ok(hasad)
}

/// Images sitting beside a Linux executable, which is what a game distributed
/// as an archive rather than as a package actually ships.
fn murashahat_mujawira(masar: &Path, asbab: &mut Vec<String>) -> Vec<(PathBuf, String)> {
    let Some(mujallad) = masar.parent() else {
        return Vec::new();
    };
    let jidhr = masar
        .file_stem()
        .and_then(|jidhr| jidhr.to_str())
        .unwrap_or("");
    let ism_mujallad = mujallad
        .file_name()
        .and_then(|ism| ism.to_str())
        .unwrap_or("");

    let mut murashahat = Vec::new();
    let mut asmaa: Vec<String> = vec!["icon".to_owned(), "logo".to_owned()];
    if !jidhr.is_empty() {
        asmaa.push(jidhr.to_owned());
    }
    if !ism_mujallad.is_empty() {
        asmaa.push(ism_mujallad.to_owned());
    }

    for ism in &asmaa {
        for imtidad in ["png", "webp", "jpg", "jpeg"] {
            let murashah = mujallad.join(format!("{ism}.{imtidad}"));
            if murashah.is_file() {
                let wasf = format!("{ism}.{imtidad} beside the executable");
                murashahat.push((murashah, wasf));
            }
        }
        let matjah = mujallad.join(format!("{ism}.svg"));
        if matjah.is_file() {
            asbab.push(format!(
                "{} exists but is SVG, and this workspace has no vector rasterizer",
                matjah.display()
            ));
        }
    }
    murashahat
}

/// The `.desktop` files worth reading for one executable.
///
/// Two sources, and no third. Beside the executable, because that is where a
/// self-contained archive puts one; and `applications/<stem>.desktop` under
/// each XDG data root, because that is where an installed one lives. What is
/// deliberately *not* done is a sweep of every `.desktop` on the system looking
/// for an `Exec=` that mentions this binary — that is a read of several hundred
/// files to answer a question two `stat` calls already answer for the games
/// that have an entry at all.
///
/// `judhur` is [`crate::fahs::SiyaqFahs::judhur_bayanat`]. This module used to
/// resolve the XDG data hierarchy itself, which made it a second answer to a
/// question the scan context already owns — and a different one, because it
/// accepted a relative `$XDG_DATA_HOME` that the context refuses.
fn masarat_desktop(masar: &Path, judhur: &[PathBuf]) -> Vec<PathBuf> {
    let mut masarat: Vec<PathBuf> = Vec::new();

    if let Some(mujallad) = masar.parent()
        && let Ok(mut mudakhalat) = std::fs::read_dir(mujallad)
    {
        for _ in 0..HADD_MUDAKHALAT_MUJALLAD {
            let Some(Ok(mudkhal)) = mudakhalat.next() else {
                break;
            };
            let murashah = mudkhal.path();
            let hiya = murashah
                .extension()
                .and_then(|imtidad| imtidad.to_str())
                .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("desktop"));
            if hiya && murashah.is_file() {
                masarat.push(murashah);
            }
            if masarat.len() >= 8 {
                break;
            }
        }
    }

    if let Some(jidhr) = masar.file_stem().and_then(|jidhr| jidhr.to_str()) {
        for asas in judhur {
            let murashah = asas.join("applications").join(format!("{jidhr}.desktop"));
            if murashah.is_file() {
                masarat.push(murashah);
            }
        }
    }

    masarat.sort();
    masarat.dedup();
    masarat
}

/// The `Icon=` value of a `.desktop` file's `[Desktop Entry]` group.
///
/// Localized variants (`Icon[ar]=`) are ignored: an icon has no language, and
/// the specification does not define the key as localizable.
#[must_use]
pub fn ayquna_min_desktop(nass: &str) -> Option<String> {
    let malaf = iqra_maqati(nass);
    let khaam = malaf.qeema("Desktop Entry", "Icon")?;
    let qeema = fukk_tahreeb_desktop(khaam);
    (!qeema.is_empty()).then_some(qeema)
}

/// Turns an `Icon=` value into the files it names.
///
/// A value containing a separator is a path and is used directly — the
/// specification allows it and games that install themselves by hand use it. A
/// bare name is a theme lookup: `<root>/hicolor/<size>/apps/<name>.<ext>`, walked
/// largest size first so the first hit is the best one available.
///
/// Returns the candidates it found and the refusals worth reporting, which is
/// how an SVG-only icon becomes a named line in the trail rather than silence.
fn hall_ayqunat_sima(ism: &str, judhur: &[PathBuf]) -> (Vec<(PathBuf, String)>, Vec<String>) {
    let mut murashahat: Vec<(PathBuf, String)> = Vec::new();
    let mut asbab: Vec<String> = Vec::new();

    if ism.contains('/') || ism.contains('\\') {
        let masar = PathBuf::from(ism);
        if masar
            .extension()
            .and_then(|imtidad| imtidad.to_str())
            .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("svg"))
        {
            asbab.push(format!(
                "{ism} is an SVG path, and there is no rasterizer here"
            ));
        } else if masar.is_file() {
            murashahat.push((masar, format!("{ism} (an absolute Icon= path)")));
        } else {
            asbab.push(format!("{ism} is an Icon= path that does not exist"));
        }
        return (murashahat, asbab);
    }

    let mut fahs = 0_usize;
    for asas in judhur {
        let simat = asas.join("icons").join(SIMA_ASASIYA);
        for qiyas in MUJALLADAT_MAQAYIS {
            let mujallad = simat.join(qiyas).join(QISM_SIMA);
            for imtidad in ["png", "webp", "jpg"] {
                fahs = fahs.saturating_add(1);
                if fahs > HADD_FAHS_SIMAT {
                    return (murashahat, asbab);
                }
                let murashah = mujallad.join(format!("{ism}.{imtidad}"));
                if murashah.is_file() {
                    let wasf = murashah.display().to_string();
                    murashahat.push((murashah, wasf));
                }
            }
        }

        // The one place an SVG is expected rather than accidental.
        let matjah = simat
            .join("scalable")
            .join(QISM_SIMA)
            .join(format!("{ism}.svg"));
        if matjah.is_file() {
            asbab.push(format!(
                "{} is the only theme entry for this icon and it is SVG; rasterizing it is out \
                 of scope for this workspace",
                matjah.display()
            ));
        }

        // `pixmaps` is the flat pre-theme directory, and a surprising number of
        // game installers still write into it.
        for imtidad in ["png", "webp", "jpg"] {
            let murashah = asas.join("pixmaps").join(format!("{ism}.{imtidad}"));
            if murashah.is_file() {
                let wasf = murashah.display().to_string();
                murashahat.push((murashah, wasf));
            }
        }
        let xpm = asas.join("pixmaps").join(format!("{ism}.xpm"));
        if xpm.is_file() {
            asbab.push(format!(
                "{} is an XPM, a C source literal that is not an image format this build reads",
                xpm.display()
            ));
        }
    }

    (murashahat, asbab)
}

/// Undoes the four escape sequences a `.desktop` value may contain.
///
/// `\s`, `\n`, `\t`, `\r` and `\\`, and nothing else. An unrecognised escape
/// keeps its backslash rather than being swallowed, so a Windows-style path
/// that ended up in an `Icon=` key survives instead of becoming mangled.
fn fukk_tahreeb_desktop(qeema: &str) -> String {
    let mut kharij = String::with_capacity(qeema.len());
    let mut huruf = qeema.chars();
    while let Some(harf) = huruf.next() {
        if harf != '\\' {
            kharij.push(harf);
            continue;
        }
        match huruf.next() {
            Some('s') => kharij.push(' '),
            Some('n') => kharij.push('\n'),
            Some('t') => kharij.push('\t'),
            Some('r') => kharij.push('\r'),
            Some('\\') | None => kharij.push('\\'),
            Some(akhar) => {
                kharij.push('\\');
                kharij.push(akhar);
            },
        }
    }
    kharij
}

// ---------------------------------------------------------------------------
// The strict section reader
// ---------------------------------------------------------------------------

/// A key/value file split into named sections.
///
/// ## The subset this reads, exactly
///
/// * Lines are split on `\n`; a trailing `\r` is dropped, so CRLF files work.
/// * A line whose first non-space character is `#` or `;` is a comment.
/// * A blank line is skipped.
/// * `[Name]` opens a section. Whitespace around `Name` is trimmed.
/// * `Key=Value` records a pair inside the open section. The key is everything
///   before the **first** `=`; the value is everything after it, both trimmed.
///   A key may therefore not contain `=`, and a value may contain any number.
/// * Keys before the first `[Section]` are recorded under the empty section
///   name, which is how `project.godot`'s leading `config_version=5` is kept
///   instead of being dropped on the floor.
/// * A localized key keeps its brackets verbatim — `Name[ar]` is a different
///   key from `Name` — so a caller that wants one can ask for it and a caller
///   that does not is never given it by accident.
///
/// ## What it deliberately does not do
///
/// No line continuations, no multi-line values, no `include` directives, no
/// duplicate-key merging, and **no value interpretation at all**. Escapes and
/// quoting differ between the two formats that use this — a `.desktop` value
/// uses `\s`-style escapes, a `project.godot` value is a quoted string with
/// `res://` paths — and a reader that tried to normalise both would be wrong
/// for at least one. Each caller unwraps its own values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MalafMaqati {
    /// Sections in file order, each with its keys in file order.
    maqati: Vec<(String, Vec<(String, String)>)>,
}

impl MalafMaqati {
    /// The first value of one key inside one section.
    ///
    /// Section names are compared case-sensitively, because both formats that
    /// use this reader define theirs that way and a case-insensitive match
    /// would make `[Application]` and `[application]` the same group when
    /// Godot treats them as different.
    #[must_use]
    pub fn qeema(&self, maqta: &str, miftah: &str) -> Option<&str> {
        self.maqati
            .iter()
            .filter(|(ism, _)| ism == maqta)
            .flat_map(|(_, mafatih)| mafatih.iter())
            .find(|(mawjud, _)| mawjud == miftah)
            .map(|(_, qeema)| qeema.as_str())
    }

    /// The first value of one key in whichever section it appears in.
    ///
    /// For a format that moved a key between sections across versions —
    /// Godot's boot splash settings are the reason this exists — asking by key
    /// alone is the difference between reading two generations of the file and
    /// reading one.
    #[must_use]
    pub fn qeema_ay(&self, miftah: &str) -> Option<&str> {
        self.maqati
            .iter()
            .flat_map(|(_, mafatih)| mafatih.iter())
            .find(|(mawjud, _)| mawjud == miftah)
            .map(|(_, qeema)| qeema.as_str())
    }

    /// Every key of one section, in file order.
    #[must_use]
    pub fn maqta(&self, ism: &str) -> &[(String, String)] {
        self.maqati
            .iter()
            .find(|(mawjud, _)| mawjud == ism)
            .map_or(&[], |(_, mafatih)| mafatih.as_slice())
    }

    /// Every section name, in file order. The first is the empty name that
    /// holds keys written before any `[Section]`.
    pub fn asma(&self) -> impl Iterator<Item = &str> {
        self.maqati.iter().map(|(ism, _)| ism.as_str())
    }

    /// Whether nothing at all was recorded.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.maqati.iter().all(|(_, mafatih)| mafatih.is_empty())
    }
}

/// Reads the subset described on [`MalafMaqati`].
///
/// Never fails. A line that is neither a comment, a section header nor a
/// key/value pair is skipped, because both formats that use this are written by
/// hand as often as by a tool and a reader that refused a whole `project.godot`
/// over one stray line would lose the icon it came for.
#[must_use]
pub fn iqra_maqati(nass: &str) -> MalafMaqati {
    let mut maqati: Vec<(String, Vec<(String, String)>)> = vec![(String::new(), Vec::new())];

    for satr in nass.lines().take(HADD_ASTUR) {
        if satr.len() > HADD_TUL_SATR {
            continue;
        }
        let satr = satr.trim();
        if satr.is_empty() || satr.starts_with('#') || satr.starts_with(';') {
            continue;
        }

        if let Some(dakhil) = satr
            .strip_prefix('[')
            .and_then(|baqi| baqi.strip_suffix(']'))
        {
            if maqati.len() >= HADD_MAQATI {
                break;
            }
            maqati.push((dakhil.trim().to_owned(), Vec::new()));
            continue;
        }

        let Some((miftah, qeema)) = satr.split_once('=') else {
            continue;
        };
        let Some((_, hali)) = maqati.last_mut() else {
            continue;
        };
        if hali.len() >= HADD_MAFATIH {
            continue;
        }
        hali.push((miftah.trim().to_owned(), qeema.trim().to_owned()));
    }

    MalafMaqati { maqati }
}
