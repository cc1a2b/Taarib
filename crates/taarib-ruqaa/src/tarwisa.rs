//! الترويسة — the sixty-four byte header, and the section table it counts.
//!
//! These are the first bytes a reader touches and the only bytes it interprets
//! before it knows whether the file is authentic. Everything in this module is
//! written for that position: fixed offsets, fixed widths, checked arithmetic,
//! no allocation, and no dereference of anything the file points at.
//!
//! ## The header, byte for byte
//!
//! ```text
//! Tarwisa — 64 bytes, little-endian
//!
//!   offset  size  field           type       meaning
//!        0     4  sihr            [u8; 4]    the four bytes "TRQ1"
//!        4     2  isdar           u16        format version, 1
//!        6     2  alam            u16        the ALAM_* flags
//!        8     4  adad_aqsam      u32        how many section table entries follow
//!       12     8  hajm_kulli      u64        the whole file's length in bytes
//!       20    32  basma           [u8; 32]   BLAKE3 over everything after the header
//!       52    12  mahjuz          [u8; 12]   reserved, written as zero
//! ```
//!
//! Four plus two plus two plus four plus eight plus thirty-two plus twelve is
//! sixty-four exactly, which is the ROADMAP's arithmetic and not an accident.
//! It does put `hajm_kulli` at offset 12, where it is four-byte aligned and not
//! eight — so the header, like the section table, is read field by field
//! rather than overlaid. That costs six loads once per file and buys a layout
//! that is byte-identical to the specification four other languages implement.
//!
//! ## What the content hash covers, and what it deliberately does not
//!
//! `basma` is BLAKE3 over the bytes from offset 64 up to the first byte of the
//! [`Tawqee`](crate::aqsam::NawQism::Tawqee) section. Two consequences follow,
//! and both are the point:
//!
//! * The **section table is inside the hash.** An attacker who rewrites an
//!   offset, a length, or a compression flag changes the hash, so the framing
//!   that every later bounds check is performed against is itself covered.
//! * The **signature block is outside it.** It has to be: the block contains a
//!   signature over the hash, and a hash that covered the signature could never
//!   be computed. Excluding it is also what lets `taarib-khatm` seal a finished
//!   package by overwriting one hundred and twenty-eight bytes in place, without
//!   moving a single offset or recomputing anything.
//!
//! The header itself is outside the hash, which is why every field in it is
//! either checked against the file's real length (`hajm_kulli`), checked against
//! a value this build knows (`sihr`, `isdar`, `alam`), or the hash itself. There
//! is no header field an attacker can move without being caught by one of those.

use crate::aqsam::{HAJM_MADKHAL, MadkhalQism, NawDaght, NawQism};
use crate::khata::KhataRuqaa;
use crate::tawqee::HAJM_KUTLA;

/// How many bytes the header occupies.
pub const HAJM_TARWISA: usize = 64;

/// The four bytes every container begins with.
pub const SIHR: [u8; 4] = *b"TRQ1";

/// The format version this build reads and writes.
pub const ISDAR_SIYAGHA: u16 = 1;

/// Byte offset of `sihr` within the header.
pub const IZAHAT_SIHR: usize = 0;
/// Byte offset of `isdar` within the header.
pub const IZAHAT_ISDAR: usize = 4;
/// Byte offset of `alam` within the header.
pub const IZAHAT_ALAM: usize = 6;
/// Byte offset of `adad_aqsam` within the header.
pub const IZAHAT_ADAD: usize = 8;
/// Byte offset of `hajm_kulli` within the header.
pub const IZAHAT_HAJM: usize = 12;
/// Byte offset of `basma` within the header.
pub const IZAHAT_BASMA: usize = 20;
/// Byte offset of `mahjuz` within the header.
pub const IZAHAT_MAHJUZ: usize = 52;

/// The largest number of sections a container can hold: one per kind.
pub const AQSA_AQSAM: u32 = 8;

/// The largest uncompressed size any one section may declare.
///
/// Five hundred and twelve mebibytes. The number is chosen against what the
/// product itself can produce, not against what feels generous:
///
/// * The atlas is bounded by the packer's own limits — eight pages of 4096 by
///   4096 single-channel texels is 128 MiB, and that is the documented maximum
///   for the default profile.
/// * The precomputed layouts are the largest table in practice. A fifty
///   thousand string script compiled at three sizes, averaging forty glyphs a
///   string, is six million glyph records of twenty-four bytes: about 144 MiB.
/// * The string table and the constraints together are single-digit
///   megabytes at that scale.
///
/// So 512 MiB is roughly three times the largest section any honest compile
/// produces, and small enough that a container cannot exhaust the address space
/// of the process reading it — which matters more here than anywhere else in
/// the product, because that process is somebody's game, on a machine that may
/// be running a 32-bit build with less than two gigabytes of usable address
/// space, and it is already most of the way through it.
///
/// The ceiling is checked against the *declared* size, before a byte is
/// reserved, and the actual output is then checked against the declaration.
/// Both halves are needed: the first stops the allocation, the second stops a
/// frame that lies about how far it expands.
pub const AQSA_QISM_KHAAM: u64 = 512 * 1024 * 1024;

/// The largest total uncompressed size across every section in one container.
///
/// Two gibibytes. Without it, seven sections each one byte under
/// [`AQSA_QISM_KHAAM`] would be three and a half gigabytes, which is the same
/// attack spread thinly enough to pass a per-section check.
pub const AQSA_MAJMU_KHAAM: u64 = 2 * 1024 * 1024 * 1024;

/// [`Tarwisa::alam`]: the atlas carries eight-bit coverage pages.
pub const ALAM_TAGHTIYA: u16 = 1 << 0;

/// [`Tarwisa::alam`]: the atlas carries signed distance field pages.
pub const ALAM_MASAFA: u16 = 1 << 1;

/// [`Tarwisa::alam`]: the constraints carry right-to-left interface mirroring
/// hints, so an adapter may flip anchors and reverse layout groups.
pub const ALAM_MIRAT: u16 = 1 << 2;

/// [`Tarwisa::alam`]: the constraints were informed by runtime capture, so the
/// measured rectangles in `QIYUD` are real rather than inferred.
pub const ALAM_ILTIQAT: u16 = 1 << 3;

/// Every flag bit this version defines.
///
/// A header that sets anything outside this mask is refused rather than masked.
/// A reserved bit is a promise that it means nothing; a container that sets one
/// is either from a format this build does not implement — in which case
/// `isdar` should have said so — or was edited by something that did not
/// understand what it was editing.
pub const ALAM_MAARUFA: u16 = ALAM_TAGHTIYA | ALAM_MASAFA | ALAM_MIRAT | ALAM_ILTIQAT;

/// Reads two little-endian bytes, or [`None`] if they are not there.
pub(crate) fn iqra_u16(bayt: &[u8], izaha: usize) -> Option<u16> {
    let nihaya = izaha.checked_add(2)?;
    let khana: [u8; 2] = bayt.get(izaha..nihaya)?.try_into().ok()?;
    Some(u16::from_le_bytes(khana))
}

/// Reads four little-endian bytes, or [`None`] if they are not there.
pub(crate) fn iqra_u32(bayt: &[u8], izaha: usize) -> Option<u32> {
    let nihaya = izaha.checked_add(4)?;
    let khana: [u8; 4] = bayt.get(izaha..nihaya)?.try_into().ok()?;
    Some(u32::from_le_bytes(khana))
}

/// Reads eight little-endian bytes, or [`None`] if they are not there.
pub(crate) fn iqra_u64(bayt: &[u8], izaha: usize) -> Option<u64> {
    let nihaya = izaha.checked_add(8)?;
    let khana: [u8; 8] = bayt.get(izaha..nihaya)?.try_into().ok()?;
    Some(u64::from_le_bytes(khana))
}

/// Reads a fixed-length run of bytes, or [`None`] if it is not there.
pub(crate) fn iqra_masfufa<const N: usize>(bayt: &[u8], izaha: usize) -> Option<[u8; N]> {
    let nihaya = izaha.checked_add(N)?;
    bayt.get(izaha..nihaya)?.try_into().ok()
}

/// Writes two little-endian bytes. The caller has already checked the length.
pub(crate) fn uktub_u16(bayt: &mut [u8], izaha: usize, qeema: u16) {
    if let Some(khana) = bayt.get_mut(izaha..izaha.saturating_add(2)) {
        khana.copy_from_slice(&qeema.to_le_bytes());
    }
}

/// Writes four little-endian bytes. The caller has already checked the length.
pub(crate) fn uktub_u32(bayt: &mut [u8], izaha: usize, qeema: u32) {
    if let Some(khana) = bayt.get_mut(izaha..izaha.saturating_add(4)) {
        khana.copy_from_slice(&qeema.to_le_bytes());
    }
}

/// Writes eight little-endian bytes. The caller has already checked the length.
pub(crate) fn uktub_u64(bayt: &mut [u8], izaha: usize, qeema: u64) {
    if let Some(khana) = bayt.get_mut(izaha..izaha.saturating_add(8)) {
        khana.copy_from_slice(&qeema.to_le_bytes());
    }
}

/// Writes a fixed-length run. The caller has already checked the length.
pub(crate) fn uktub_masfufa(bayt: &mut [u8], izaha: usize, qeema: &[u8]) {
    if let Some(khana) = bayt.get_mut(izaha..izaha.saturating_add(qeema.len())) {
        khana.copy_from_slice(qeema);
    }
}

/// A length as a `u64`, saturating on a platform where `usize` is wider — which
/// is none this product targets, and is still not a reason to write a cast the
/// compiler cannot prove.
pub(crate) fn tul_u64(tul: usize) -> u64 {
    u64::try_from(tul).unwrap_or(u64::MAX)
}

/// A container offset as a `usize`, or [`None`] when it does not fit — which on
/// a 32-bit target is the ordinary case for a hostile length, not an edge case.
pub(crate) fn hajm_usize(qeema: u64) -> Option<usize> {
    usize::try_from(qeema).ok()
}

/// Bytes as lowercase hexadecimal, for a refusal a person can read.
///
/// One spelling for the whole crate. A hash printed one way in the header's
/// message and another way in the reader's would look like two different hashes
/// to whoever is comparing them against a registry page.
pub(crate) fn sittasi(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    bayt.iter().fold(
        String::with_capacity(bayt.len().saturating_mul(2)),
        |mut nass, wahid| {
            let _ = write!(nass, "{wahid:02x}");
            nass
        },
    )
}

/// The sixty-four byte header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tarwisa {
    /// The format version. Always [`ISDAR_SIYAGHA`] in a container this build
    /// accepts; a higher number is a refusal, never a partial parse.
    pub isdar: u16,
    /// The `ALAM_*` bits. Only [`ALAM_MAARUFA`] may be set.
    pub alam: u16,
    /// How many entries the section table has, 1..=[`AQSA_AQSAM`].
    pub adad_aqsam: u32,
    /// The whole file's length, which must equal the number of bytes actually
    /// present — no shorter, and no longer.
    pub hajm_kulli: u64,
    /// BLAKE3 over everything from the end of this header to the first byte of
    /// the signature section.
    pub basma: [u8; 32],
}

impl Tarwisa {
    /// Reads and validates the header, and nothing beyond it.
    ///
    /// The checks are in the order they are, and not in a more convenient one,
    /// because each depends on the one before it being true:
    ///
    /// 1. **length** — sixty-four bytes must exist before any offset into the
    ///    header means anything;
    /// 2. **magic** — the cheapest way to reject a file that is not a patch at
    ///    all, and the check that stops every later message from being confusing;
    /// 3. **version** — before any *other* field is interpreted, because the
    ///    meaning of every offset below is a property of the version. A parse
    ///    that reads fields first and checks the version afterwards has already
    ///    read fields whose meaning it guessed;
    /// 4. **flags**, **section count**, **total size** — each judged against a
    ///    value this build knows or against the bytes actually present.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::MalafQaseer`], [`KhataRuqaa::SihrGhayrMutabaq`],
    /// [`KhataRuqaa::IsdarGhayrMadum`], [`KhataRuqaa::AlamMajhula`],
    /// [`KhataRuqaa::AdadAqsamGhayrSalih`] or
    /// [`KhataRuqaa::HajmKulliGhayrMutabaq`], each naming the field it refused.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataRuqaa> {
        if bayt.len() < HAJM_TARWISA {
            return Err(KhataRuqaa::MalafQaseer {
                haql: "the header",
                tul: tul_u64(bayt.len()),
                matlub: tul_u64(HAJM_TARWISA),
            });
        }
        let qaseer = |haql: &'static str| KhataRuqaa::MalafQaseer {
            haql,
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(HAJM_TARWISA),
        };

        let sihr: [u8; 4] = iqra_masfufa(bayt, IZAHAT_SIHR).ok_or_else(|| qaseer("the magic"))?;
        if sihr != SIHR {
            return Err(KhataRuqaa::SihrGhayrMutabaq { wujid: sihr });
        }

        let isdar = iqra_u16(bayt, IZAHAT_ISDAR).ok_or_else(|| qaseer("the format version"))?;
        if isdar != ISDAR_SIYAGHA {
            return Err(KhataRuqaa::IsdarGhayrMadum {
                wujid: isdar,
                madum: ISDAR_SIYAGHA,
            });
        }

        let alam = iqra_u16(bayt, IZAHAT_ALAM).ok_or_else(|| qaseer("the header flags"))?;
        if alam & !ALAM_MAARUFA != 0 {
            return Err(KhataRuqaa::AlamMajhula { alam });
        }

        let adad_aqsam = iqra_u32(bayt, IZAHAT_ADAD).ok_or_else(|| qaseer("the section count"))?;
        if adad_aqsam == 0 || adad_aqsam > AQSA_AQSAM {
            return Err(KhataRuqaa::AdadAqsamGhayrSalih {
                adad: adad_aqsam,
                aqsa: AQSA_AQSAM,
            });
        }

        let hajm_kulli = iqra_u64(bayt, IZAHAT_HAJM).ok_or_else(|| qaseer("the total size"))?;
        let fili = tul_u64(bayt.len());
        if hajm_kulli != fili {
            return Err(KhataRuqaa::HajmKulliGhayrMutabaq {
                muallan: hajm_kulli,
                fili,
            });
        }

        let basma: [u8; 32] =
            iqra_masfufa(bayt, IZAHAT_BASMA).ok_or_else(|| qaseer("the content hash"))?;

        Ok(Self {
            isdar,
            alam,
            adad_aqsam,
            hajm_kulli,
            basma,
        })
    }

    /// Writes the header into the first sixty-four bytes of a buffer.
    ///
    /// The reserved twelve bytes are written as zero. They are not a place to
    /// put anything: they are outside the content hash, so a value written
    /// there is a value no signature covers.
    ///
    /// # Errors
    ///
    /// Returns [`KhataRuqaa::MalafQaseer`] when the buffer is shorter than
    /// [`HAJM_TARWISA`].
    pub fn ila_bayt(&self, bayt: &mut [u8]) -> Result<(), KhataRuqaa> {
        if bayt.len() < HAJM_TARWISA {
            return Err(KhataRuqaa::MalafQaseer {
                haql: "the header",
                tul: tul_u64(bayt.len()),
                matlub: tul_u64(HAJM_TARWISA),
            });
        }
        uktub_masfufa(bayt, IZAHAT_SIHR, &SIHR);
        uktub_u16(bayt, IZAHAT_ISDAR, self.isdar);
        uktub_u16(bayt, IZAHAT_ALAM, self.alam);
        uktub_u32(bayt, IZAHAT_ADAD, self.adad_aqsam);
        uktub_u64(bayt, IZAHAT_HAJM, self.hajm_kulli);
        uktub_masfufa(bayt, IZAHAT_BASMA, &self.basma);
        uktub_masfufa(bayt, IZAHAT_MAHJUZ, &[0u8; 12]);
        Ok(())
    }

    /// Where the section table ends, or [`None`] if the arithmetic overflows.
    #[must_use]
    pub fn nihayat_jadwal(&self) -> Option<u64> {
        u64::from(self.adad_aqsam)
            .checked_mul(tul_u64(HAJM_MADKHAL))?
            .checked_add(tul_u64(HAJM_TARWISA))
    }

    /// Whether the atlas is declared as coverage pages.
    #[must_use]
    pub const fn taghtiya(&self) -> bool {
        self.alam & ALAM_TAGHTIYA != 0
    }

    /// Whether the atlas is declared as signed distance field pages.
    #[must_use]
    pub const fn masafa(&self) -> bool {
        self.alam & ALAM_MASAFA != 0
    }

    /// Whether the patch carries right-to-left interface mirroring hints.
    #[must_use]
    pub const fn mirat(&self) -> bool {
        self.alam & ALAM_MIRAT != 0
    }

    /// Whether the constraints were informed by runtime capture.
    #[must_use]
    pub const fn iltiqat(&self) -> bool {
        self.alam & ALAM_ILTIQAT != 0
    }

    /// The content hash as lowercase hexadecimal.
    #[must_use]
    pub fn basma_nass(&self) -> String {
        sittasi(&self.basma)
    }
}

/// The validated section table.
///
/// Fixed-size and `Copy`: eight kinds, so eight slots, so no allocation. A game
/// process that maps a patch and asks where the glyph map is should not have
/// touched the allocator to find out.
///
/// Every invariant the format states is established here, once, so that no
/// later code re-derives it:
///
/// * every entry's kind and compression are values this format defines;
/// * no kind appears twice;
/// * every section begins on a sixteen-byte boundary;
/// * no section overlaps another, and none overlaps the header or the table;
/// * every section lies entirely inside the file, with the addition checked;
/// * no section declares more than [`AQSA_QISM_KHAAM`] uncompressed bytes, and
///   the sections together declare no more than [`AQSA_MAJMU_KHAAM`];
/// * the signature section is the last entry, is uncompressed, is exactly
///   [`HAJM_KUTLA`] bytes, and ends at the last byte of the file;
/// * every section [`NawQism::ilzami`] requires is present.
#[derive(Debug, Clone, Copy)]
pub struct JadwalAqsam {
    madkhalat: [Option<MadkhalQism>; 8],
    tarteeb: [u8; 8],
    adad: u8,
    nihayat_muhtawa: u64,
    majmu_khaam: u64,
}

impl JadwalAqsam {
    /// Reads and validates the whole section table.
    ///
    /// Takes the entire container so that every offset can be judged against
    /// the real file length rather than against another number from the same
    /// untrusted header. Reads nothing outside the table itself: no section's
    /// bytes are touched here, and no length from the table is used to size an
    /// allocation.
    ///
    /// # Errors
    ///
    /// One of [`KhataRuqaa::MalafQaseer`], [`KhataRuqaa::NawQismMajhul`],
    /// [`KhataRuqaa::QismMukarrar`], [`KhataRuqaa::IzahaGhayrMuhadhah`],
    /// [`KhataRuqaa::AqsamMutadakhila`], [`KhataRuqaa::QismKharij`],
    /// [`KhataRuqaa::DaghtMajhul`], [`KhataRuqaa::HajmKhaamMufrit`],
    /// [`KhataRuqaa::MajmuKhaamMufrit`], [`KhataRuqaa::TawqeeLaysAkhiran`],
    /// [`KhataRuqaa::KutlatTawqeeTalifa`] or [`KhataRuqaa::QismMafqud`],
    /// each naming the entry and the field that was refused.
    pub fn min_bayt(bayt: &[u8], tarwisa: &Tarwisa) -> Result<Self, KhataRuqaa> {
        let nihayat_jadwal = tarwisa
            .nihayat_jadwal()
            .ok_or(KhataRuqaa::AdadAqsamGhayrSalih {
                adad: tarwisa.adad_aqsam,
                aqsa: AQSA_AQSAM,
            })?;
        if nihayat_jadwal > tarwisa.hajm_kulli {
            return Err(KhataRuqaa::MalafQaseer {
                haql: "the section table",
                tul: tarwisa.hajm_kulli,
                matlub: nihayat_jadwal,
            });
        }

        let mut madkhalat: [Option<MadkhalQism>; 8] = [None; 8];
        let mut tarteeb = [0u8; 8];
        let mut majmu_khaam: u64 = 0;
        let mut nihayat_muhtawa: u64 = 0;
        let adad =
            u8::try_from(tarwisa.adad_aqsam).map_err(|_| KhataRuqaa::AdadAqsamGhayrSalih {
                adad: tarwisa.adad_aqsam,
                aqsa: AQSA_AQSAM,
            })?;

        for fahras in 0..usize::from(adad) {
            let izaha_madkhal = HAJM_TARWISA.saturating_add(fahras.saturating_mul(HAJM_MADKHAL));
            let nihayat_madkhal = izaha_madkhal.saturating_add(HAJM_MADKHAL);
            let nafidha = bayt.get(izaha_madkhal..nihayat_madkhal).ok_or_else(|| {
                KhataRuqaa::MalafQaseer {
                    haql: "the section table",
                    tul: tul_u64(bayt.len()),
                    matlub: tul_u64(nihayat_madkhal),
                }
            })?;
            let madkhal = MadkhalQism::min_bayt(nafidha)?;
            let naw = madkhal.naw.raqm();
            let khana = usize::try_from(naw).unwrap_or(usize::MAX).saturating_sub(1);

            if madkhalat.get(khana).copied().flatten().is_some() {
                return Err(KhataRuqaa::QismMukarrar { naw });
            }
            if !madkhal.muhadhah() {
                return Err(KhataRuqaa::IzahaGhayrMuhadhah {
                    naw,
                    izaha: madkhal.izaha,
                });
            }
            // Kind zero in this refusal is the framing itself: a section that
            // starts before the section table ends is claiming bytes the header
            // and the table already own.
            if madkhal.izaha < nihayat_jadwal {
                return Err(KhataRuqaa::AqsamMutadakhila {
                    awwal: 0,
                    thani: naw,
                });
            }
            let nihaya = madkhal.nihaya().ok_or_else(|| KhataRuqaa::QismKharij {
                naw,
                haql: "stored length (offset plus length overflows)",
                qeema: madkhal.tul_makhzun,
                hadd: tarwisa.hajm_kulli.saturating_sub(madkhal.izaha),
            })?;
            if nihaya > tarwisa.hajm_kulli {
                return Err(KhataRuqaa::QismKharij {
                    naw,
                    haql: "the end of the stored bytes",
                    qeema: nihaya,
                    hadd: tarwisa.hajm_kulli,
                });
            }
            if madkhal.tul_khaam > AQSA_QISM_KHAAM {
                return Err(KhataRuqaa::HajmKhaamMufrit {
                    naw,
                    muallan: madkhal.tul_khaam,
                    saqf: AQSA_QISM_KHAAM,
                });
            }
            majmu_khaam =
                majmu_khaam
                    .checked_add(madkhal.tul_khaam)
                    .ok_or(KhataRuqaa::MajmuKhaamMufrit {
                        majmu: u64::MAX,
                        saqf: AQSA_MAJMU_KHAAM,
                    })?;
            if majmu_khaam > AQSA_MAJMU_KHAAM {
                return Err(KhataRuqaa::MajmuKhaamMufrit {
                    majmu: majmu_khaam,
                    saqf: AQSA_MAJMU_KHAAM,
                });
            }

            if madkhal.naw == NawQism::Tawqee {
                Self::tahaqquq_tawqee(&madkhal, fahras, adad, nihaya, tarwisa.hajm_kulli)?;
                nihayat_muhtawa = madkhal.izaha;
            }

            if let Some(makan) = madkhalat.get_mut(khana) {
                *makan = Some(madkhal);
            }
            if let Some(makan) = tarteeb.get_mut(fahras) {
                *makan = u8::try_from(naw).unwrap_or(0);
            }
        }

        Self::tahaqquq_tadakhul(&madkhalat)?;
        for naw in NawQism::KULL {
            let khana = usize::try_from(naw.raqm())
                .unwrap_or(usize::MAX)
                .saturating_sub(1);
            if naw.ilzami() && madkhalat.get(khana).copied().flatten().is_none() {
                return Err(KhataRuqaa::QismMafqud { ism: naw.ism() });
            }
        }

        Ok(Self {
            madkhalat,
            tarteeb,
            adad,
            nihayat_muhtawa,
            majmu_khaam,
        })
    }

    /// The signature section's own rules, kept together because they are one
    /// idea: the block must be reachable, whole, and final without decompressing
    /// or trusting anything.
    fn tahaqquq_tawqee(
        madkhal: &MadkhalQism,
        fahras: usize,
        adad: u8,
        nihaya: u64,
        hajm_kulli: u64,
    ) -> Result<(), KhataRuqaa> {
        if madkhal.naw.bila_daght() && madkhal.daght != NawDaght::Bila {
            return Err(KhataRuqaa::KutlatTawqeeTalifa {
                haql: "the block is compressed, and a verifier must reach it without \
                       expanding anything",
            });
        }
        if madkhal.tul_makhzun != tul_u64(HAJM_KUTLA) {
            return Err(KhataRuqaa::KutlatTawqeeTalifa {
                haql: "the block is not exactly 128 bytes, so it cannot be filled in place",
            });
        }
        if fahras.saturating_add(1) != usize::from(adad) || nihaya != hajm_kulli {
            return Err(KhataRuqaa::TawqeeLaysAkhiran);
        }
        Ok(())
    }

    /// Refuses any two sections that share a byte.
    ///
    /// Overlap is not a corruption symptom, it is an attack: one run of bytes
    /// read as two structures lets a value be authenticated in one reading and
    /// used in another. With at most eight entries this is a fixed-size sort on
    /// the stack.
    fn tahaqquq_tadakhul(madkhalat: &[Option<MadkhalQism>; 8]) -> Result<(), KhataRuqaa> {
        let mut nitaqat: [(u64, u64, u32); 8] = [(u64::MAX, u64::MAX, 0); 8];
        let mut adad = 0usize;
        for madkhal in madkhalat.iter().flatten() {
            let nihaya = madkhal.nihaya().unwrap_or(u64::MAX);
            if let Some(khana) = nitaqat.get_mut(adad) {
                *khana = (madkhal.izaha, nihaya, madkhal.naw.raqm());
            }
            adad = adad.saturating_add(1);
        }
        // Insertion sort by starting offset. Eight elements, no allocation, and
        // a stable order so the refusal always names the same pair.
        for i in 1..adad {
            let mut j = i;
            while j > 0 {
                let sabiq = nitaqat
                    .get(j.saturating_sub(1))
                    .copied()
                    .unwrap_or_default();
                let hali = nitaqat.get(j).copied().unwrap_or_default();
                if sabiq.0 <= hali.0 {
                    break;
                }
                if let Some(khana) = nitaqat.get_mut(j.saturating_sub(1)) {
                    *khana = hali;
                }
                if let Some(khana) = nitaqat.get_mut(j) {
                    *khana = sabiq;
                }
                j = j.saturating_sub(1);
            }
        }
        for i in 1..adad {
            let sabiq = nitaqat
                .get(i.saturating_sub(1))
                .copied()
                .unwrap_or_default();
            let hali = nitaqat.get(i).copied().unwrap_or_default();
            if hali.0 < sabiq.1 {
                return Err(KhataRuqaa::AqsamMutadakhila {
                    awwal: sabiq.2,
                    thani: hali.2,
                });
            }
        }
        Ok(())
    }

    /// The entry for a kind, if the container carries it.
    #[must_use]
    pub fn qism(&self, naw: NawQism) -> Option<MadkhalQism> {
        let khana = usize::try_from(naw.raqm())
            .unwrap_or(usize::MAX)
            .saturating_sub(1);
        self.madkhalat.get(khana).copied().flatten()
    }

    /// Every entry, in the order the table lists them.
    ///
    /// This is what the asset gate and the review console walk: it says exactly
    /// which sections a package carries, where each one is, how many bytes it
    /// occupies stored, and how many it becomes — before anything is expanded.
    pub fn madkhalat(&self) -> impl Iterator<Item = MadkhalQism> + '_ {
        self.tarteeb
            .iter()
            .take(usize::from(self.adad))
            .filter_map(move |naw| {
                let khana = usize::from(*naw).checked_sub(1)?;
                self.madkhalat.get(khana).copied().flatten()
            })
    }

    /// How many sections the container carries.
    #[must_use]
    pub const fn adad(&self) -> u8 {
        self.adad
    }

    /// One past the last byte the content hash covers, which is the first byte
    /// of the signature block.
    #[must_use]
    pub const fn nihayat_muhtawa(&self) -> u64 {
        self.nihayat_muhtawa
    }

    /// The total uncompressed size every section declares, already checked
    /// against [`AQSA_MAJMU_KHAAM`].
    #[must_use]
    pub const fn majmu_khaam(&self) -> u64 {
        self.majmu_khaam
    }
}
