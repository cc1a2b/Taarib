//! التنفيذي — the one reader of a Windows PE image and its version resource.
//!
//! Two detectors need the same four facts out of a shipped binary: what
//! architecture it was built for, what name the engine gives itself, who wrote
//! it, and what version it declares. [`crate::dalail::bio4`] needs them out of
//! `bio4.exe`; [`crate::dalail::khassa`] needs them out of six in-house engines'
//! own modules. Until this module existed each had its own copy of the parser,
//! and the two copies disagreed — which is the whole reason for this file, and
//! is worth writing down rather than leaving as a note in a commit message.
//!
//! ## The three ways a version resource is read wrong
//!
//! **A zero-length value reads as the next key.** A `String` entry whose
//! `wValueLength` is zero has no value at all. A reader that steps from the key
//! over its terminator and its padding and then decodes whatever is there walks
//! into the next entry's header and its key. `InternalName` in EA's shipped
//! `Engine.Render.Core2.PlatformPcDx12.dll` is such an entry, and the reader
//! that ignored `wValueLength` reported the engine's internal name as
//! `t(\u{1}LegalCopyright`. [`qeemat_mawrid`] reads the declared length first,
//! so the answer is [`None`] — which is what an empty field means.
//!
//! **The version block is not near the front of `.rsrc`.** It is wherever the
//! resource compiler put it, and a game that ships artwork in the same section
//! pushes it to the back. Measured on this machine:
//!
//! | binary | `.rsrc` length | where the version block starts |
//! | --- | --- | --- |
//! | `Engine.Render.Core2.PlatformPcDx12.dll` | 1 536 | 160 |
//! | `GTA5_Enhanced.exe` | 188 928 | 1 408 |
//! | `eldenring.exe` | 220 160 | 218 896 |
//! | `DarkSoulsRemastered.exe` | 1 767 936 | 1 746 352 |
//! | `Social-Club-Setup.exe` | 128 158 208 | 128 057 480 |
//!
//! A reader that took the first megabyte of the section found nothing in the
//! last two. A reader that took the first megabyte and the last would find the
//! fourth and still lose a block that landed in the middle of the fifth.
//!
//! **The block found by searching may belong to something else.** An installer
//! carries other people's binaries in its resources, and each of those carries
//! its own version block. Searching `Rockstar-Games-Launcher.exe` for
//! `CompanyName` finds Microsoft's `apisetstub` — a real version resource, in
//! the right format, belonging to a payload rather than to the image being read.
//!
//! ## What this module does instead
//!
//! It follows the path Windows itself follows. The optional header's data
//! directory names the resource tree's address; the tree is three levels of
//! [`IMAGE_RESOURCE_DIRECTORY`] indexed by type, then by name, then by language;
//! the leaf is an address and a length. Type sixteen is `RT_VERSION`. That walk
//! is bounded — the deepest directory touched across the 400 shipped binaries
//! measured here ended 7 648 bytes into the section — it lands on the image's
//! *own* version block, and it finds it wherever it is, including in the three
//! packed binaries here whose resources are not in a section called `.rsrc` at
//! all: `FC26.exe` keeps them in `.trace`, `CrimsonDesert.exe` in `.data` and
//! `afop.exe` in `.xcode`. Every one of those reads correctly now and read as
//! nothing before.
//!
//! [`IMAGE_RESOURCE_DIRECTORY`]: https://learn.microsoft.com/windows/win32/menurc/resource-types

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use taarib_usus::manassa::Mimariya;

// ---------------------------------------------------------------------------
// Bounds. Every one is a ceiling, never a target.
// ---------------------------------------------------------------------------

/// Bytes taken from the front of an image.
///
/// Enough for the DOS stub, the PE signature, the COFF header, the optional
/// header and a section table of any size a linker produces.
const HAJM_TARWISA_PE: usize = 4 * 1024;

/// The most sections a PE image may declare before the file is refused.
const AQSA_MAQATI: usize = 96;

/// The most of a resource tree that is read to find the version block.
///
/// Sixty-four kilobytes. The deepest node touched across the 400 shipped
/// binaries measured for this module ended 7 648 bytes in, so this is eight
/// times the worst case observed and still a fixed cost on a section that can be
/// a hundred megabytes.
const HAJM_DALEEL_MAWARID: usize = 64 * 1024;

/// The most of a version block that is read.
///
/// The blocks measured here run from 528 to 1 188 bytes. Sixty-four kilobytes is
/// far above any of them and bounds a length field that says otherwise.
const AQSA_KUTLAT_ISDAR: usize = 64 * 1024;

/// The most entries one resource-directory node may declare.
///
/// The tree read here is indexed by type, then by name, then by language, and
/// none of those three fans out: what fans out is the icon table, which this
/// module never descends into.
const AQSA_MADAKHIL_DALEEL: usize = 1024;

/// The most UTF-16 code units accepted as one version-resource value.
const AQSA_QEEMAT_MAWRID: usize = 256;

// ---------------------------------------------------------------------------
// The constants the format itself defines
// ---------------------------------------------------------------------------

/// `RT_VERSION` — the resource type the version block is filed under.
const NAW_MAWRID_ISDAR: u32 = 16;

/// The bit a directory entry sets to say it points at another directory rather
/// than at a leaf.
const QINAA_FAR: u32 = 0x8000_0000;

/// The `wType` a `String` entry carries when its value is text.
///
/// Checked, because `VarFileInfo`'s `Translation` is a `String`-shaped structure
/// whose value is two binary words, and reading those as characters would put
/// two arbitrary code points into an evidence line.
const NAW_NASS: u16 = 1;

/// The optional header magic for a 32-bit image, and for a 64-bit one.
///
/// The two layouts differ only in the width of five address fields ahead of the
/// data directory, which is why the offset of that table is all this module
/// needs to know about the difference.
const SIHR_IKHTIYARI_32: u16 = 0x010B;

/// See [`SIHR_IKHTIYARI_32`].
const SIHR_IKHTIYARI_64: u16 = 0x020B;

/// Where the data directory starts inside a 32-bit optional header.
const IZAHAT_JADWAL_32: usize = 96;

/// Where the data directory starts inside a 64-bit optional header.
const IZAHAT_JADWAL_64: usize = 112;

/// The data directory's index for the resource tree.
const RUTBAT_MAWARID: usize = 2;

/// One data-directory entry: an address and a length.
const TUL_MADKHAL_JADWAL: usize = 8;

/// One section header.
const TUL_TARWISAT_QISM: usize = 40;

/// One resource-directory node's fixed header, ahead of its entries.
const TUL_TARWISAT_DALEEL: usize = 16;

/// One resource-directory entry: a name-or-id and an offset.
const TUL_MADKHAL_DALEEL: usize = 8;

// ---------------------------------------------------------------------------
// What a reader gets back
// ---------------------------------------------------------------------------

/// What a binary's version resource said about itself.
///
/// Six fields rather than the four one caller wants or the five the other does,
/// because the block is read once and every field in it costs a search of a
/// kilobyte. Splitting them per caller would buy nothing and would put the
/// question "which reader am I using" back into the detectors.
#[derive(Debug, Default)]
pub(crate) struct BayanTanfidhi {
    /// The image's architecture, from the COFF machine field.
    pub(crate) mimariya: Option<Mimariya>,
    /// `InternalName`.
    pub(crate) ism_dakhili: Option<String>,
    /// `ProductName`.
    pub(crate) ism_muntaj: Option<String>,
    /// `ProductVersion`.
    pub(crate) isdar_muntaj: Option<String>,
    /// `CompanyName`.
    pub(crate) sharika: Option<String>,
    /// `LegalCopyright`.
    pub(crate) huquq: Option<String>,
    /// `FileVersion`.
    pub(crate) isdar_malaf: Option<String>,
}

/// One section of a PE image: its file offset and how many bytes of it are on
/// disk.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Qism {
    /// Offset of the section's data in the file.
    pub(crate) mawdi: u64,
    /// How many bytes of it the file actually carries.
    pub(crate) hajm: usize,
}

/// One section header, as the table declares it.
#[derive(Debug, Clone, Copy)]
struct TarwisatQism {
    /// The eight-byte name field, padding included.
    ism: [u8; 8],
    /// Where the section is mapped, relative to the image base.
    mawdi_wahmi: u32,
    /// Where its bytes are in the file.
    mawdi: u32,
    /// How many of them there are.
    hajm: u32,
}

/// The parts of a PE header this crate reads.
#[derive(Debug)]
struct TarwisaPe {
    /// The architecture, when it is one this product installs into.
    mimariya: Option<Mimariya>,
    /// Every section the table declares.
    maqati: Vec<TarwisatQism>,
    /// The resource tree's address, when the data directory names one.
    mawarid: Option<u32>,
}

// ---------------------------------------------------------------------------
// What a caller asks for
// ---------------------------------------------------------------------------

/// Reads what a binary's version resource says about itself.
///
/// Returns [`None`] only for something that is not a PE image at all; a PE with
/// no version resource still yields its architecture, which is a fact about the
/// binary worth having on its own.
pub(crate) fn bayan(masar: &Path) -> Option<BayanTanfidhi> {
    let tarwisa = iqra_nafidha(masar, 0, HAJM_TARWISA_PE)?;
    let mafkuk = tarwisa_pe(&tarwisa)?;
    let mut bayan = BayanTanfidhi {
        mimariya: mafkuk.mimariya,
        ..BayanTanfidhi::default()
    };

    let Some(kutla) = kutlat_isdar(masar, &mafkuk) else {
        return Some(bayan);
    };
    bayan.ism_dakhili = qeemat_mawrid(&kutla, "InternalName");
    bayan.ism_muntaj = qeemat_mawrid(&kutla, "ProductName");
    bayan.isdar_muntaj = qeemat_mawrid(&kutla, "ProductVersion");
    bayan.sharika = qeemat_mawrid(&kutla, "CompanyName");
    bayan.huquq = qeemat_mawrid(&kutla, "LegalCopyright");
    bayan.isdar_malaf = qeemat_mawrid(&kutla, "FileVersion");
    Some(bayan)
}

/// The dotted version a binary declares about itself, or [`None`] when it
/// declares none, is not a PE image, or is not there at all.
///
/// The one field of [`BayanTanfidhi`] a caller outside this crate needs, exposed
/// as a field rather than as the whole record: a game's own build number lives
/// in its executable's version resource — which is how RDR2 is versioned — and
/// nothing outside engine identification has any business with the other five.
///
/// `FileVersion` before `ProductVersion` because the first is the four-part
/// number a build lives in and the second is whatever marketing wrote.
/// `DarkSoulsRemastered.exe`, one of the two images this module's own tests are
/// pinned against, declares `1,0,0,0` for the first and `1` for the second;
/// preferring the product version would leave a caller indexing into a single
/// component.
#[must_use]
pub fn isdar_muallan(masar: &Path) -> Option<String> {
    let bayan = bayan(masar)?;
    bayan.isdar_malaf.or(bayan.isdar_muntaj)
}

/// The file offset and on-disk length of the first section whose name begins
/// with `ism`.
///
/// Compared by prefix rather than by the whole eight-byte field, because a
/// section name shorter than eight bytes is padded with zeros and one longer is
/// a `/nn` reference into the string table that no shipped image uses for the
/// names looked for here.
///
/// Nothing here resolves a virtual address, because a caller of this function
/// wants the section's raw bytes: a string constant is read out of `.rdata`
/// exactly as it sits on disk.
pub(crate) fn qism(masar: &Path, ism: &[u8]) -> Option<Qism> {
    let tarwisa = iqra_nafidha(masar, 0, HAJM_TARWISA_PE)?;
    let mafkuk = tarwisa_pe(&tarwisa)?;
    for qism in &mafkuk.maqati {
        if !qism.ism.starts_with(ism) || qism.hajm == 0 || qism.mawdi == 0 {
            continue;
        }
        return Some(Qism {
            mawdi: u64::from(qism.mawdi),
            hajm: usize::try_from(qism.hajm).ok()?,
        });
    }
    None
}

// ---------------------------------------------------------------------------
// The headers
// ---------------------------------------------------------------------------

/// Parses the DOS stub, the COFF header, the optional header's data directory
/// and the section table.
///
/// A section whose header runs off the end of the window read stops the table
/// rather than discarding it: a truncated read is less evidence, not bad
/// evidence.
fn tarwisa_pe(tarwisa: &[u8]) -> Option<TarwisaPe> {
    if !tarwisa.starts_with(b"MZ") {
        return None;
    }
    let bidayat_pe = usize::try_from(raqm32_sagheer(tarwisa, 0x3C)?).ok()?;
    if !tarwisa.get(bidayat_pe..)?.starts_with(b"PE\0\0") {
        return None;
    }

    let mimariya = mimariyat_pe(raqm16_sagheer(tarwisa, bidayat_pe.checked_add(4)?)?);
    let adad = usize::from(raqm16_sagheer(tarwisa, bidayat_pe.checked_add(6)?)?);
    let hajm_ikhtiyari = usize::from(raqm16_sagheer(tarwisa, bidayat_pe.checked_add(20)?)?);
    if adad == 0 || adad > AQSA_MAQATI {
        return None;
    }
    let ikhtiyari = bidayat_pe.checked_add(24)?;
    let mawarid = daleel_mawarid(tarwisa, ikhtiyari, hajm_ikhtiyari);
    let jadwal = ikhtiyari.checked_add(hajm_ikhtiyari)?;

    let mut maqati: Vec<TarwisatQism> = Vec::with_capacity(adad);
    for raqm in 0..adad {
        let Some(madkhal) = raqm
            .checked_mul(TUL_TARWISAT_QISM)
            .and_then(|izaha| jadwal.checked_add(izaha))
        else {
            break;
        };
        let Some(ism) = madkhal
            .checked_add(8)
            .and_then(|nihaya| tarwisa.get(madkhal..nihaya))
            .and_then(|khaam| <[u8; 8]>::try_from(khaam).ok())
        else {
            break;
        };
        let (Some(mawdi_wahmi), Some(hajm), Some(mawdi)) = (
            madkhal
                .checked_add(12)
                .and_then(|izaha| raqm32_sagheer(tarwisa, izaha)),
            madkhal
                .checked_add(16)
                .and_then(|izaha| raqm32_sagheer(tarwisa, izaha)),
            madkhal
                .checked_add(20)
                .and_then(|izaha| raqm32_sagheer(tarwisa, izaha)),
        ) else {
            break;
        };
        maqati.push(TarwisatQism {
            ism,
            mawdi_wahmi,
            mawdi,
            hajm,
        });
    }
    Some(TarwisaPe {
        mimariya,
        maqati,
        mawarid,
    })
}

/// The resource tree's address, out of the optional header's data directory.
///
/// This is the field Windows itself reads to find resources, and reading it is
/// what lets the tree be found in a section the packer renamed.
fn daleel_mawarid(tarwisa: &[u8], ikhtiyari: usize, hajm_ikhtiyari: usize) -> Option<u32> {
    let izahat_jadwal = match raqm16_sagheer(tarwisa, ikhtiyari)? {
        SIHR_IKHTIYARI_32 => IZAHAT_JADWAL_32,
        SIHR_IKHTIYARI_64 => IZAHAT_JADWAL_64,
        _ => return None,
    };
    // `NumberOfRvaAndSizes` sits immediately in front of the table it counts,
    // and an image is allowed to declare fewer entries than the sixteen the
    // format defines.
    let adad = raqm32_sagheer(
        tarwisa,
        ikhtiyari.checked_add(izahat_jadwal.checked_sub(4)?)?,
    )?;
    if usize::try_from(adad).ok()? <= RUTBAT_MAWARID {
        return None;
    }
    let madkhal = izahat_jadwal.checked_add(RUTBAT_MAWARID.checked_mul(TUL_MADKHAL_JADWAL)?)?;
    if madkhal.checked_add(TUL_MADKHAL_JADWAL)? > hajm_ikhtiyari {
        return None;
    }
    let wahmi = raqm32_sagheer(tarwisa, ikhtiyari.checked_add(madkhal)?)?;
    (wahmi != 0).then_some(wahmi)
}

/// The file offset a mapped address resolves to, when its bytes are on disk.
///
/// A section's virtual span may be longer than its raw one — the tail is zeros
/// the loader supplies and the file does not carry — so an address past the raw
/// length resolves to nothing rather than to a byte belonging to the next
/// section.
fn mawdi_min_wahmi(maqati: &[TarwisatQism], wahmi: u32) -> Option<u64> {
    for qism in maqati {
        if qism.mawdi_wahmi == 0 || qism.mawdi == 0 {
            continue;
        }
        let Some(dakhil) = wahmi.checked_sub(qism.mawdi_wahmi) else {
            continue;
        };
        if dakhil >= qism.hajm {
            continue;
        }
        return u64::from(qism.mawdi).checked_add(u64::from(dakhil));
    }
    None
}

/// The COFF machine field, for the architectures this product installs into.
const fn mimariyat_pe(alat: u16) -> Option<Mimariya> {
    match alat {
        0x014C => Some(Mimariya::X86),
        0x8664 => Some(Mimariya::X8664),
        0xAA64 => Some(Mimariya::Aarch64),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The resource tree
// ---------------------------------------------------------------------------

/// The bytes of the image's own `RT_VERSION` resource.
fn kutlat_isdar(masar: &Path, mafkuk: &TarwisaPe) -> Option<Vec<u8>> {
    let jidhr = mawdi_min_wahmi(&mafkuk.maqati, mafkuk.mawarid?)?;
    let shajara = iqra_nafidha(masar, jidhr, HAJM_DALEEL_MAWARID)?;
    let (mawdi, hajm) = warqat_isdar(&shajara, &mafkuk.maqati)?;
    iqra_nafidha(masar, mawdi, hajm)
}

/// Descends type, then name, then language, to the version resource's leaf.
///
/// Three levels exactly, because that is how many the format has. An entry that
/// points at a directory where a leaf belongs, or at a leaf where a directory
/// belongs, is skipped rather than followed: the next language of the same
/// resource may well be well-formed.
fn warqat_isdar(shajara: &[u8], maqati: &[TarwisatQism]) -> Option<(u64, usize)> {
    for (naw, ila_anwaa) in madakhil_daleel(shajara, 0) {
        if naw != NAW_MAWRID_ISDAR {
            continue;
        }
        let Some(anwaa) = far_daleel(ila_anwaa) else {
            continue;
        };
        for (_, ila_asma) in madakhil_daleel(shajara, anwaa) {
            let Some(asma) = far_daleel(ila_asma) else {
                continue;
            };
            for (_, ila_warqa) in madakhil_daleel(shajara, asma) {
                if far_daleel(ila_warqa).is_some() {
                    continue;
                }
                let Ok(warqa) = usize::try_from(ila_warqa) else {
                    continue;
                };
                let (Some(wahmi), Some(hajm)) = (
                    raqm32_sagheer(shajara, warqa),
                    warqa
                        .checked_add(4)
                        .and_then(|izaha| raqm32_sagheer(shajara, izaha)),
                ) else {
                    continue;
                };
                let Ok(hajm) = usize::try_from(hajm) else {
                    continue;
                };
                let Some(mawdi) = mawdi_min_wahmi(maqati, wahmi) else {
                    continue;
                };
                if hajm == 0 {
                    continue;
                }
                return Some((mawdi, hajm.min(AQSA_KUTLAT_ISDAR)));
            }
        }
    }
    None
}

/// One resource-directory node's entries, as `(name or id, offset)`.
///
/// Named entries come first and id entries after them, and both are returned:
/// the caller compares the id it wants and a named entry can never equal it,
/// because a name has the high bit set and an id is a small number.
fn madakhil_daleel(shajara: &[u8], izaha: usize) -> Vec<(u32, u32)> {
    let mut kharij: Vec<(u32, u32)> = Vec::new();
    let (Some(bi_ism), Some(bi_raqm), Some(awwal)) = (
        izaha
            .checked_add(12)
            .and_then(|mawdi| raqm16_sagheer(shajara, mawdi)),
        izaha
            .checked_add(14)
            .and_then(|mawdi| raqm16_sagheer(shajara, mawdi)),
        izaha.checked_add(TUL_TARWISAT_DALEEL),
    ) else {
        return kharij;
    };
    let adad = usize::from(bi_ism)
        .saturating_add(usize::from(bi_raqm))
        .min(AQSA_MADAKHIL_DALEEL);

    for raqm in 0..adad {
        let Some(madkhal) = raqm
            .checked_mul(TUL_MADKHAL_DALEEL)
            .and_then(|izaha| awwal.checked_add(izaha))
        else {
            break;
        };
        let (Some(ism), Some(ila)) = (
            raqm32_sagheer(shajara, madkhal),
            madkhal
                .checked_add(4)
                .and_then(|izaha| raqm32_sagheer(shajara, izaha)),
        ) else {
            break;
        };
        kharij.push((ism, ila));
    }
    kharij
}

/// The subdirectory an entry points at, when it points at one.
fn far_daleel(ila: u32) -> Option<usize> {
    if ila & QINAA_FAR == 0 {
        return None;
    }
    usize::try_from(ila & !QINAA_FAR).ok()
}

// ---------------------------------------------------------------------------
// The version block
// ---------------------------------------------------------------------------

/// One `StringFileInfo` value out of a version block, by its key.
///
/// The layout is Microsoft's `String` structure, and this reads the two fields
/// in front of the key rather than only the text behind it:
///
/// ```text
/// -6   WORD  wLength       the whole structure
/// -4   WORD  wValueLength  the value, in UTF-16 code units, terminator included
/// -2   WORD  wType         1 for text
///  0   WCHAR szKey[]       NUL-terminated
///      WORD  Padding[]     to the next 32-bit boundary
///      WCHAR Value[]       NUL-terminated
/// ```
///
/// `wValueLength` is why. A reader that steps straight from the key to the text
/// behind it returns the *next* entry's header and key for any entry whose value
/// is empty, and an empty `InternalName` is not rare — EA ships one. Reading the
/// declared length first turns that into [`None`], which is what it is.
///
/// Both the key and its terminator sit on even offsets, so the padding is either
/// nothing or one zero code unit — which is why stepping over at most one is
/// exact rather than approximate.
pub(crate) fn qeemat_mawrid(kutla: &[u8], miftah: &str) -> Option<String> {
    let ibra: Vec<u8> = miftah.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mawqi = mawdi_zawji(kutla, &ibra)?;
    let tul_qeema = raqm16_sagheer(kutla, mawqi.checked_sub(4)?)?;
    if raqm16_sagheer(kutla, mawqi.checked_sub(2)?)? != NAW_NASS || tul_qeema < 2 {
        return None;
    }
    let hadd = usize::from(tul_qeema).min(AQSA_QEEMAT_MAWRID);

    let mut baad = mawqi.checked_add(ibra.len())?;
    if !zawj_sifr(kutla, baad) {
        return None;
    }
    baad = baad.checked_add(2)?;
    if zawj_sifr(kutla, baad) {
        baad = baad.checked_add(2)?;
    }
    nass_utf16(kutla, baad, hadd)
}

/// The first occurrence of `ibra` at an even offset at least six bytes in.
///
/// Even because UTF-16 code units are, and a match straddling one would be two
/// halves of two other characters that happened to spell the key. Six because
/// the `String` structure's three header fields sit in front of the key and a
/// key with nothing in front of it is not one.
fn mawdi_zawji(kawm: &[u8], ibra: &[u8]) -> Option<usize> {
    if ibra.is_empty() || ibra.len() > kawm.len() {
        return None;
    }
    kawm.windows(ibra.len())
        .enumerate()
        .find(|(mawqi, nafidha)| *mawqi >= 6 && mawqi.is_multiple_of(2) && *nafidha == ibra)
        .map(|(mawqi, _)| mawqi)
}

/// Whether the two bytes at `mawdi` are a zero UTF-16 code unit.
fn zawj_sifr(kawm: &[u8], mawdi: usize) -> bool {
    mawdi
        .checked_add(2)
        .and_then(|nihaya| kawm.get(mawdi..nihaya))
        .is_some_and(|zawj| zawj == [0, 0])
}

/// A NUL-terminated UTF-16 string, bounded, trimmed, and decoded lossily.
///
/// Lossily because a company name or a copyright line is written by whichever
/// build machine produced the binary and can carry anything; one unpaired
/// surrogate must cost a character rather than the whole observation. A run that
/// reaches `aqsa` without a terminator is refused, because the entry's own
/// header already said how long the value is and a longer one contradicts it.
fn nass_utf16(kawm: &[u8], bidaya: usize, aqsa: usize) -> Option<String> {
    let mut wahdat: Vec<u16> = Vec::new();
    let mut mawdi = bidaya;
    loop {
        let zawj = kawm.get(mawdi..mawdi.checked_add(2)?)?;
        let wahda = u16::from_le_bytes(<[u8; 2]>::try_from(zawj).ok()?);
        if wahda == 0 {
            break;
        }
        wahdat.push(wahda);
        if wahdat.len() > aqsa {
            return None;
        }
        mawdi = mawdi.checked_add(2)?;
    }
    let nass = String::from_utf16_lossy(&wahdat).trim().to_owned();
    (!nass.is_empty()).then_some(nass)
}

// ---------------------------------------------------------------------------
// Bounded reads and fields
// ---------------------------------------------------------------------------

/// Reads at most `hadd` bytes of a file starting at `izaha`.
///
/// Opened and read rather than mapped, for the reason [`crate::dalail::unity`]
/// gives: a probe runs across a library where Steam is mid-update on one title
/// while another is being read, and a mapped page whose backing file is
/// truncated underneath it raises a signal no `Result` catches.
pub(crate) fn iqra_nafidha(masar: &Path, izaha: u64, hadd: usize) -> Option<Vec<u8>> {
    let mut malaf = File::open(masar).ok()?;
    if izaha > 0 && malaf.seek(SeekFrom::Start(izaha)).is_err() {
        return None;
    }
    let mut bayt = Vec::new();
    let _ = malaf
        .take(u64::try_from(hadd).ok()?)
        .read_to_end(&mut bayt)
        .ok()?;
    Some(bayt)
}

/// A little-endian `u32` at a checked offset.
pub(crate) fn raqm32_sagheer(qita: &[u8], izaha: usize) -> Option<u32> {
    let bayt = qita.get(izaha..izaha.checked_add(4)?)?;
    <[u8; 4]>::try_from(bayt).ok().map(u32::from_le_bytes)
}

/// A little-endian `u16` at a checked offset.
pub(crate) fn raqm16_sagheer(qita: &[u8], izaha: usize) -> Option<u16> {
    let bayt = qita.get(izaha..izaha.checked_add(2)?)?;
    <[u8; 2]>::try_from(bayt).ok().map(u16::from_le_bytes)
}

// ---------------------------------------------------------------------------
// Images for the tests of the modules that read them
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod suwar {
    //! Synthetic PE images carrying real resource bytes.
    //!
    //! The wrapper is built here and the payloads inside it are not. What a
    //! fixture has to reproduce is the structure a reader walks — a data
    //! directory, a three-level resource tree, a leaf whose address resolves
    //! through a section header — and a ninety-megabyte executable in the source
    //! tree would test the same thing and cost a thousand times as much to read.
    //!
    //! One builder rather than one per test module, for the same reason there is
    //! one reader: two builders drift, and a fixture that drifts stops being
    //! evidence.

    use super::{
        IZAHAT_JADWAL_32, IZAHAT_JADWAL_64, NAW_MAWRID_ISDAR, QINAA_FAR, SIHR_IKHTIYARI_32,
        SIHR_IKHTIYARI_64, TUL_MADKHAL_DALEEL, TUL_TARWISAT_DALEEL,
    };

    /// Where the PE signature goes, behind a DOS stub of zeros.
    const BIDAYAT_PE: usize = 0x80;

    /// The COFF machine field a 32-bit image declares.
    const ALAT_32: u16 = 0x014C;

    /// The optional header size a 32-bit image declares, and a 64-bit one.
    const HAJM_IKHTIYARI_32: usize = 224;

    /// See [`HAJM_IKHTIYARI_32`].
    const HAJM_IKHTIYARI_64: usize = 240;

    /// How many data-directory entries a built image declares.
    const ADAD_JADWAL: u32 = 16;

    /// Where the first section's bytes go.
    const BIDAYAT_KHAAM: usize = 0x400;

    /// What a file offset is rounded up to.
    const MUHADHAT_MALAF: usize = 0x200;

    /// What a mapped address is rounded up to.
    const MUHADHAT_DHAKIRA: usize = 0x1000;

    /// Where the first section is mapped.
    const AWWAL_WAHMI: usize = 0x1000;

    /// One directory node and the single entry it holds.
    const TUL_UQDA: usize = TUL_TARWISAT_DALEEL + TUL_MADKHAL_DALEEL;

    /// One `IMAGE_RESOURCE_DATA_ENTRY`: an address, a length, a code page and a
    /// reserved word.
    const TUL_WARQA: usize = 16;

    /// How many bytes of tree sit in front of the version block: three
    /// single-entry directory nodes and one leaf.
    const TUL_SHAJARA: usize = TUL_UQDA * 3 + TUL_WARQA;

    /// The language a built resource is filed under. `0x0409` is what every
    /// binary measured for this module uses.
    const LUGHAT_MAWRID: u32 = 0x0409;

    /// What a built image is to contain.
    #[derive(Debug)]
    pub(crate) struct Sura<'a> {
        /// The COFF machine field. `0x014C` builds a 32-bit image.
        pub(crate) alat: u16,
        /// The `.rdata` section's bytes; empty means the image has no such
        /// section.
        pub(crate) rdata: &'a [u8],
        /// The name to give the section the resource tree lives in.
        pub(crate) ism_mawarid: &'a [u8],
        /// Bytes of padding between the resource tree and the version block, so
        /// that a test can put the block where a real binary puts it.
        pub(crate) hashw: usize,
        /// The `VS_VERSION_INFO` block; empty builds a resource section with no
        /// version resource in it, which is what a real module with no version
        /// resource looks like.
        pub(crate) kutla: &'a [u8],
    }

    /// Builds the image `wasf` describes.
    pub(crate) fn sawwir(wasf: &Sura<'_>) -> Vec<u8> {
        let thalathi = wasf.alat == ALAT_32;
        let hajm_ikhtiyari = if thalathi {
            HAJM_IKHTIYARI_32
        } else {
            HAJM_IKHTIYARI_64
        };
        let izahat_jadwal = if thalathi {
            IZAHAT_JADWAL_32
        } else {
            IZAHAT_JADWAL_64
        };
        let sihr = if thalathi {
            SIHR_IKHTIYARI_32
        } else {
            SIHR_IKHTIYARI_64
        };
        let simat: u16 = if thalathi { 0x0102 } else { 0x0022 };

        // The resource section is mapped behind `.rdata`, so where its block
        // lands is known before either section's bytes exist.
        let qabl = if wasf.rdata.is_empty() {
            0
        } else {
            wasf.rdata.len().next_multiple_of(MUHADHAT_DHAKIRA)
        };
        let wahmi_mawarid = AWWAL_WAHMI.saturating_add(qabl);

        let mut aqsam: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        if !wasf.rdata.is_empty() {
            aqsam.push((musamma(b".rdata"), wasf.rdata.to_vec()));
        }
        aqsam.push((musamma(wasf.ism_mawarid), qism_mawarid(wasf, wahmi_mawarid)));

        let mut mawaqi: Vec<(usize, usize, usize)> = Vec::new();
        let mut wahmi = AWWAL_WAHMI;
        let mut khaam = BIDAYAT_KHAAM;
        for (_, bayanat) in &aqsam {
            mawaqi.push((wahmi, khaam, bayanat.len()));
            wahmi = wahmi.saturating_add(bayanat.len().next_multiple_of(MUHADHAT_DHAKIRA));
            khaam = khaam.saturating_add(bayanat.len().next_multiple_of(MUHADHAT_MALAF));
        }
        let hajm_mawarid = mawaqi.last().map_or(0, |(_, _, tul)| *tul);

        let mut bayt: Vec<u8> = Vec::new();
        bayt.extend_from_slice(b"MZ");
        bayt.resize(0x3C, 0);
        bayt.extend_from_slice(&raqm(BIDAYAT_PE).to_le_bytes());
        bayt.resize(BIDAYAT_PE, 0);
        bayt.extend_from_slice(b"PE\0\0");
        bayt.extend_from_slice(&wasf.alat.to_le_bytes());
        bayt.extend_from_slice(&u16::try_from(aqsam.len()).unwrap_or(0).to_le_bytes());
        bayt.resize(BIDAYAT_PE.saturating_add(20), 0);
        bayt.extend_from_slice(&u16::try_from(hajm_ikhtiyari).unwrap_or(0).to_le_bytes());
        bayt.extend_from_slice(&simat.to_le_bytes());

        let ikhtiyari = BIDAYAT_PE.saturating_add(24);
        bayt.extend_from_slice(&sihr.to_le_bytes());
        bayt.resize(ikhtiyari.saturating_add(izahat_jadwal.saturating_sub(4)), 0);
        bayt.extend_from_slice(&ADAD_JADWAL.to_le_bytes());
        bayt.resize(
            ikhtiyari.saturating_add(izahat_jadwal).saturating_add(16),
            0,
        );
        bayt.extend_from_slice(&raqm(wahmi_mawarid).to_le_bytes());
        bayt.extend_from_slice(&raqm(hajm_mawarid).to_le_bytes());
        bayt.resize(ikhtiyari.saturating_add(hajm_ikhtiyari), 0);

        for ((ism, _), (wahmi, khaam, tul)) in aqsam.iter().zip(&mawaqi) {
            bayt.extend_from_slice(ism);
            bayt.extend_from_slice(&raqm(*tul).to_le_bytes());
            bayt.extend_from_slice(&raqm(*wahmi).to_le_bytes());
            bayt.extend_from_slice(&raqm(*tul).to_le_bytes());
            bayt.extend_from_slice(&raqm(*khaam).to_le_bytes());
            bayt.resize(bayt.len().saturating_add(16), 0);
        }
        for ((_, bayanat), (_, khaam, _)) in aqsam.iter().zip(&mawaqi) {
            bayt.resize(*khaam, 0);
            bayt.extend_from_slice(bayanat);
        }
        bayt
    }

    /// The resource section: a three-level tree, then padding, then the block.
    ///
    /// `wahmi` is where the section itself is mapped, which is what turns the
    /// block's position inside the section into the address the leaf declares.
    ///
    /// An empty block builds the tree's root as an empty directory, which is the
    /// shape of a module that has resources and no version among them —
    /// `Engine.Render.Core2.PlatformVulkan.dll` ships exactly that.
    fn qism_mawarid(wasf: &Sura<'_>, wahmi: usize) -> Vec<u8> {
        let mut bayt: Vec<u8> = Vec::new();
        if wasf.kutla.is_empty() {
            bayt.resize(TUL_TARWISAT_DALEEL, 0);
            return bayt;
        }
        let wahmi_kutla = wahmi.saturating_add(TUL_SHAJARA).saturating_add(wasf.hashw);

        daleel(&mut bayt, NAW_MAWRID_ISDAR, QINAA_FAR | raqm(TUL_UQDA));
        daleel(&mut bayt, 1, QINAA_FAR | raqm(TUL_UQDA.saturating_mul(2)));
        daleel(&mut bayt, LUGHAT_MAWRID, raqm(TUL_UQDA.saturating_mul(3)));
        bayt.extend_from_slice(&raqm(wahmi_kutla).to_le_bytes());
        bayt.extend_from_slice(&raqm(wasf.kutla.len()).to_le_bytes());
        bayt.extend_from_slice(&0_u32.to_le_bytes());
        bayt.extend_from_slice(&0_u32.to_le_bytes());
        bayt.resize(bayt.len().saturating_add(wasf.hashw), 0);
        bayt.extend_from_slice(wasf.kutla);
        bayt
    }

    /// One directory node holding one id entry.
    fn daleel(bayt: &mut Vec<u8>, ism: u32, ila: u32) {
        bayt.resize(bayt.len().saturating_add(12), 0);
        bayt.extend_from_slice(&0_u16.to_le_bytes());
        bayt.extend_from_slice(&1_u16.to_le_bytes());
        bayt.extend_from_slice(&ism.to_le_bytes());
        bayt.extend_from_slice(&ila.to_le_bytes());
    }

    /// A section name padded out to the eight bytes the header field holds.
    fn musamma(ism: &[u8]) -> Vec<u8> {
        let mut kharij = ism.to_vec();
        kharij.resize(8, 0);
        kharij
    }

    /// A length or an offset as the field that holds it.
    fn raqm(qeema: usize) -> u32 {
        u32::try_from(qeema).unwrap_or(0)
    }
}

#[cfg(test)]
mod ikhtibarat {
    use super::suwar::{Sura, sawwir};
    use super::*;

    // -----------------------------------------------------------------------
    // Fixtures.
    //
    // Every block below is a whole `VS_VERSION_INFO` resource copied out of a
    // shipped install on the machine this module was written on, from the offset
    // the image's own resource tree gives, for exactly the length the leaf
    // declares. None of it is reconstructed from a specification.
    // -----------------------------------------------------------------------

    /// `FC 26/Engine.Render.Core2.PlatformPcDx12.dll`, 740 bytes at 1 564 320.
    ///
    /// Here because of `InternalName`, whose `wValueLength` is zero. That is the
    /// entry a reader which only looks behind the key gets wrong, and the bytes
    /// behind it — `74 00 28 00 01 00` and then `LegalCopyright` — are what such
    /// a reader returns as the name.
    const KUTLAT_FROSTBITE: &str = "\
        E40234000000560053005F00560045005200530049004F004E005F0049004E0046004F0000000000\
        BD04EFFE000001002A000200000005002A000200000005003F000000010000000400040002000000\
        00000000000000000000000042020000010053007400720069006E006700460069006C0065004900\
        6E0066006F0000001E02000001003000340030003900300034006200300000004000100001004300\
        6F006D00700061006E0079004E0061006D0065000000000045006C0065006300740072006F006E00\
        69006300200041007200740073000000720025000100460069006C00650044006500730063007200\
        69007000740069006F006E0000000000460072006F00730074006200690074006500200072006500\
        6E0064006500720069006E006700200070006C006100740066006F0072006D002000730075007000\
        70006F0072007400000000002E0007000100460069006C006500560065007200730069006F006E00\
        0000000032002E00340032002E0035000000000020000000010049006E007400650072006E006100\
        6C004E0061006D00650000007400280001004C006500670061006C0043006F007000790072006900\
        670068007400000043006F0070007900720069006700680074002000280043002900200032003000\
        32003000200045006C0065006300740072006F006E00690063002000410072007400730020004900\
        6E0063002E0000002800000001004F0072006900670069006E0061006C00460069006C0065006E00\
        61006D006500000034000A000100500072006F0064007500630074004E0061006D00650000000000\
        460072006F007300740062006900740065000000320007000100500072006F006400750063007400\
        560065007200730069006F006E00000032002E00340032002E003500000000004400000001005600\
        61007200460069006C00650049006E0066006F00000000002400040000005400720061006E007300\
        6C006100740069006F006E00000000000904B004";

    /// `DARK SOULS REMASTERED/DarkSoulsRemastered.exe`, 936 bytes at 31 860 656
    /// — which is 1 746 352 bytes into a `.rsrc` section of 1 767 936.
    ///
    /// Here because of where it is. A reader that took the first megabyte of the
    /// section read none of this.
    const KUTLAT_DANTELION: &str = "\
        A80334000000560053005F00560045005200530049004F004E005F0049004E0046004F0000000000\
        BD04EFFE00000100030001000000010000000100000000003F000000000000000400040001000000\
        00000000000000000000000008030000010053007400720069006E006700460069006C0065004900\
        6E0066006F000000E402000001003000340030003900300034006200300000001800000001004300\
        6F006D006D0065006E0074007300000050001800010043006F006D00700061006E0079004E006100\
        6D006500000000004E0041004D0043004F002000420041004E004400410049002000470061006D00\
        65007300200049006E0063002E0000005E001B000100460069006C00650044006500730063007200\
        69007000740069006F006E00000000004400410052004B00200053004F0055004C00530028005400\
        4D0029003A002000520045004D004100530054004500520045004400000000003000080001004600\
        69006C006500560065007200730069006F006E000000000031002C0030002C0030002C0030000000\
        34000A00010049006E007400650072006E0061006C004E0061006D00650000004400410052004B00\
        53004F0055004C00530000003800080001004F0072006900670069006E0061006C00460069006C00\
        65004E0061006D00650000004400530052002E006500780065000000EE00650001004C0065006700\
        61006C0043006F00700079007200690067006800740000004400410052004B00200053004F005500\
        4C005300280054004D0029003A002000520045004D00410053005400450052004500440020002600\
        200028004300290032003000310032002000420041004E0044004100490020004E0041004D004300\
        4F00200045006E007400650072007400610069006E006D0065006E007400200049006E0063002E00\
        200028004300290032003000310031002D0032003000310038002000460072006F006D0053006F00\
        6600740077006100720065002C00200049006E0063002E00000000004E0017000100500072006F00\
        64007500630074004E0061006D006500000000004400410052004B00200053004F0055004C005300\
        3A002000520045004D00410053005400450052004500440000000000280002000100500072006F00\
        6400750063007400560065007200730069006F006E00000031000000440000000100560061007200\
        460069006C00650049006E0066006F00000000002400040000005400720061006E0073006C006100\
        740069006F006E00000000000904B004";

    /// Where that block starts inside the section that carries it.
    const IZAHAT_DANTELION: usize = 1_746_352;

    /// Decodes a hex fixture, dropping anything that is not a hex digit.
    fn min_sitteen(nass: &str) -> Vec<u8> {
        let arqam: Vec<u8> = nass
            .bytes()
            .filter(u8::is_ascii_hexdigit)
            .map(qeemat_raqm)
            .collect();
        arqam
            .chunks_exact(2)
            .filter_map(|zawj| match zawj {
                [aala, adna] => Some((aala << 4) | adna),
                _ => None,
            })
            .collect()
    }

    /// One hex digit's value.
    fn qeemat_raqm(harf: u8) -> u8 {
        match harf {
            b'0'..=b'9' => harf.saturating_sub(b'0'),
            b'a'..=b'f' => harf.saturating_sub(b'a').saturating_add(10),
            _ => harf.saturating_sub(b'A').saturating_add(10),
        }
    }

    /// Writes an image into a fresh directory and reads its version resource
    /// back out.
    fn iqra(wasf: &Sura<'_>) -> std::io::Result<BayanTanfidhi> {
        let masrah = tempfile::tempdir()?;
        let masar = masrah.path().join("sura.exe");
        std::fs::write(&masar, sawwir(wasf))?;
        Ok(bayan(&masar).unwrap_or_default())
    }

    /// A zero-length value reads as absent rather than as the next key.
    ///
    /// The first of the two defects this module was extracted to fix.
    /// `InternalName` in EA's real resource has `wValueLength` zero, and a reader
    /// that steps from the key straight to the text behind it returns
    /// `t(\u{1}LegalCopyright` — a plausible-looking string that is two struct
    /// headers and somebody else's key. The neighbouring entries are asserted
    /// too, because a reader that returned [`None`] for everything would satisfy
    /// the first assertion on its own.
    #[test]
    fn qeema_faragha_tuqra_ka_ghiyab() {
        let kutla = min_sitteen(KUTLAT_FROSTBITE);
        assert_eq!(
            qeemat_mawrid(&kutla, "InternalName"),
            None,
            "an empty value is absent"
        );
        assert_eq!(
            qeemat_mawrid(&kutla, "ProductName").as_deref(),
            Some("Frostbite")
        );
        assert_eq!(
            qeemat_mawrid(&kutla, "ProductVersion").as_deref(),
            Some("2.42.5")
        );
        assert_eq!(
            qeemat_mawrid(&kutla, "CompanyName").as_deref(),
            Some("Electronic Arts")
        );
        assert_eq!(
            qeemat_mawrid(&kutla, "LegalCopyright").as_deref(),
            Some("Copyright (C) 2020 Electronic Arts Inc.")
        );
    }

    /// A version block a megabyte and a half into its section is still read.
    ///
    /// The second defect, at the offset the real binary puts it at: 1 746 352
    /// bytes into a section of 1 767 936. A reader bounded to the front of the
    /// section reads nothing here at all.
    #[test]
    fn kutla_khalfa_megabyte_tuqra() -> std::io::Result<()> {
        let kutla = min_sitteen(KUTLAT_DANTELION);
        let bayan = iqra(&Sura {
            alat: 0x8664,
            rdata: b"",
            ism_mawarid: b".rsrc",
            hashw: IZAHAT_DANTELION,
            kutla: &kutla,
        })?;

        assert_eq!(bayan.mimariya, Some(Mimariya::X8664));
        assert_eq!(bayan.ism_dakhili.as_deref(), Some("DARKSOULS"));
        assert_eq!(bayan.ism_muntaj.as_deref(), Some("DARK SOULS: REMASTERED"));
        assert_eq!(bayan.sharika.as_deref(), Some("NAMCO BANDAI Games Inc."));
        assert!(
            bayan
                .huquq
                .as_deref()
                .is_some_and(|huquq| huquq.contains("FromSoftware")),
            "the copyright behind the block is read whole"
        );
        Ok(())
    }

    /// The resource tree is found in a section the packer renamed.
    ///
    /// `FC26.exe` keeps its resources in `.trace`, `CrimsonDesert.exe` in
    /// `.data` and `afop.exe` in `.xcode`. All three carry a version resource and
    /// a reader that looked for a section called `.rsrc` read none of them.
    #[test]
    fn mawarid_kharij_qism_musamma() -> std::io::Result<()> {
        let kutla = min_sitteen(KUTLAT_FROSTBITE);
        let bayan = iqra(&Sura {
            alat: 0x8664,
            rdata: b"",
            ism_mawarid: b".trace",
            hashw: 0,
            kutla: &kutla,
        })?;

        assert_eq!(bayan.ism_muntaj.as_deref(), Some("Frostbite"));
        assert_eq!(bayan.isdar_muntaj.as_deref(), Some("2.42.5"));
        Ok(())
    }

    /// An image with resources and no version among them yields its architecture
    /// and nothing else.
    ///
    /// `Engine.Render.Core2.PlatformVulkan.dll` ships exactly that shape, and it
    /// is why the detector that reads it reads more than one module. The
    /// architecture is a fact about the binary and survives; every string is
    /// absent rather than guessed.
    #[test]
    fn sura_bila_isdar_tuti_almimariya() -> std::io::Result<()> {
        let bayan = iqra(&Sura {
            alat: 0x014C,
            rdata: b"",
            ism_mawarid: b".rsrc",
            hashw: 0,
            kutla: b"",
        })?;

        assert_eq!(bayan.mimariya, Some(Mimariya::X86));
        assert!(bayan.ism_muntaj.is_none());
        assert!(bayan.ism_dakhili.is_none());
        assert!(bayan.sharika.is_none());
        assert!(bayan.huquq.is_none());
        assert!(bayan.isdar_malaf.is_none());
        Ok(())
    }

    /// A section is still found by name, and its raw bytes come back whole.
    ///
    /// That is what the marker scan in `crate::dalail::khassa` asks of this
    /// module, and it is a different question from where the resource tree is.
    #[test]
    fn qism_bi_ismihi() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let masar = masrah.path().join("sura.exe");
        let rdata = b"Dantelion2 lives here".as_slice();
        let sura = sawwir(&Sura {
            alat: 0x8664,
            rdata,
            ism_mawarid: b".rsrc",
            hashw: 0,
            kutla: b"",
        });
        std::fs::write(&masar, sura)?;

        let maqta = qism(&masar, b".rdata").unwrap_or(Qism { mawdi: 0, hajm: 0 });
        assert_eq!(maqta.hajm, rdata.len());
        let maqru = iqra_nafidha(&masar, maqta.mawdi, maqta.hajm).unwrap_or_default();
        assert_eq!(maqru, rdata);
        assert!(qism(&masar, b".nope").is_none());
        Ok(())
    }
}
