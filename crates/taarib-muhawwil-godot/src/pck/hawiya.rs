//! الحاوية — Godot's `.pck`, in both format versions, loose and embedded.
//!
//! One container, two versions, three ways of arriving. Version 1 is Godot 3's
//! and version 2 is Godot 4's; either may be a file of its own next to the
//! executable, or appended to the end of the executable itself. This module
//! reads all of those and writes one of them — the additive patch package the
//! adapter ships.
//!
//! ## The header
//!
//! ```text
//! PCK header — version 1 (Godot 3), little-endian
//!
//!   offset  size  field       type       meaning
//!        0     4  sihr        [u8; 4]    "GDPC"
//!        4     4  isdar       u32        the pack format version, 1
//!        8     4  muharrik_r  u32        the engine's major version
//!       12     4  muharrik_s  u32        the engine's minor version
//!       16     4  muharrik_t  u32        the engine's patch version
//!       20    64  mahjuz      [u32; 16]  reserved, written as zero
//!       84     4  adad        u32        how many entries follow
//!       88   ...  madakhil    ...        that many entries
//! ```
//!
//! ```text
//! PCK header — version 2 (Godot 4), little-endian
//!
//!   offset  size  field       type       meaning
//!        0     4  sihr        [u8; 4]    "GDPC"
//!        4     4  isdar       u32        the pack format version, 2
//!        8     4  muharrik_r  u32        the engine's major version
//!       12     4  muharrik_s  u32        the engine's minor version
//!       16     4  muharrik_t  u32        the engine's patch version
//!       20     4  alam        u32        pack flags; bit 0 = the index is encrypted
//!       24     8  qaida       u64        file base: where the content region starts
//!       32    64  mahjuz      [u32; 16]  reserved, written as zero
//!       96     4  adad        u32        how many entries follow
//!      100   ...  madakhil    ...        that many entries
//! ```
//!
//! The two differ by exactly twelve bytes — `alam` and `qaida` — inserted before
//! the reserved run, and by four bytes on the end of every entry. Both versions
//! carry the same sixteen reserved words, which this module preserves rather
//! than regenerates: they are zero in every package the engine writes, and a
//! package that puts something there is a package this build does not
//! understand well enough to rewrite from scratch.
//!
//! `qaida` is why a version 2 entry's offset is not a file offset. The packer
//! writes the index first with a placeholder, then the content, then goes back
//! and fills in where the content began; every entry offset is measured from
//! there. Version 1 has no such field and stores offsets from the start of the
//! package. So the one expression that turns an entry into a place in the file
//! is [`Hawiya::izahat_asas`] plus the entry's own offset, and it is written
//! once.
//!
//! ## The entry
//!
//! ```text
//! Madkhal — variable, little-endian
//!
//!   offset  size  field      type       meaning
//!        0     4  tul_masar  u32        stored path length, PADDING INCLUDED
//!        4   ...  masar      [u8]       tul_masar bytes: UTF-8, then NUL padding
//!      ...     8  izaha      u64        offset, measured from the content base
//!      ...     8  hajm       u64        how many bytes are stored for this entry
//!      ...    16  basma      [u8; 16]   MD5 of the entry's plaintext, or all zero
//!      ...     4  alam       u32        version 2 only; bit 0 = entry is encrypted
//! ```
//!
//! Two things about `tul_masar` catch every reader that guesses. It is the
//! length **after** padding, not the length of the text: the engine rounds the
//! UTF-8 path up to a multiple of four and adds the difference to the field, so
//! a reader that treats it as a character count walks off the end of one entry
//! and into the middle of the next. And the padding is NUL bytes that are part
//! of the stored run, so the text is what remains after they are trimmed — this
//! module trims them and then decodes, rather than decoding and hoping the NULs
//! survive.
//!
//! The stored path begins `res://`, which is Godot's own name for "inside the
//! project", and is the exact string the engine's file API will be asked for at
//! runtime. This module is tolerant about that on read — a package built by a
//! third-party tool with some other prefix is still readable, and refusing it
//! would help nobody — and strict about it on write, because a patch entry whose
//! path is not the path the game asks for is an entry the game never sees.
//!
//! ## Embedded packages, and why the magic appears twice
//!
//! A self-contained export is one executable with the package appended to it.
//! The bytes at offset zero are then a PE, ELF or Mach-O header, and the package
//! begins at some offset nobody recorded in a place a loader could find — the
//! executable's own headers describe the executable, and appending to a file
//! does not update them.
//!
//! So the format records it from the other end:
//!
//! ```text
//! Dhayl — the last twelve bytes of an executable with a package appended
//!
//!   offset       size  field   type      meaning
//!   tul - 12        8  masafa  u64       distance from the package's first byte
//!                                        to this record's first byte
//!   tul -  4        4  sihr    [u8; 4]   "GDPC", a second time
//! ```
//!
//! Detection therefore reads the **tail first**: the last four bytes, then the
//! eight before them, then back `masafa` bytes to where the package's own magic
//! must be. The magic appears twice on purpose and the two occurrences do
//! different jobs. The one at the tail says "there is a package in this file at
//! all" — it is the only thing distinguishing an executable with an appended
//! package from an executable with a few kilobytes of anything else stuck on the
//! end. The one at `masafa` bytes back confirms the distance actually landed on
//! a package header rather than on the middle of some asset, which is what turns
//! a corrupt or truncated file into a refusal instead of a parse of arbitrary
//! bytes as a file table.
//!
//! That trailing record is the entire mechanism behind a single-file export.
//! Without it the engine would need the package to be a separate file, because
//! nothing else in an executable can tell it where the archive starts.
//!
//! **Taarib never writes an embedded package and never modifies one.** An
//! executable with a package inside it is read and left exactly as it is; the
//! patch is a separate `.pck` mounted alongside. Rewriting an executable would
//! break its code signature on macOS and Windows and turn a removable patch into
//! a modified binary.
//!
//! ## Encrypted packages
//!
//! Godot encrypts with AES-256 in CBC mode, and the same framing wraps both an
//! encrypted index and an encrypted entry:
//!
//! ```text
//! Itar tashfeer — the first forty bytes of an encrypted run
//!
//!   offset  size  field     type       meaning
//!        0    16  basma     [u8; 16]   MD5 of the plaintext
//!       16     8  tul       u64        the plaintext's length in bytes
//!       24    16  muttajih  [u8; 16]   the CBC initialisation vector
//!       40   ...  shifra    [u8]       ciphertext, rounded up to a 16 byte block
//! ```
//!
//! `tul` is the *plaintext* length and the ciphertext is longer, because CBC
//! works in whole blocks; the trailing bytes of the last block are discarded
//! after decryption rather than being interpreted as padding. The inner `basma`
//! is what proves the key was right — a wrong key decrypts to noise, and noise
//! does not hash to the recorded digest. That is the only test there is, and it
//! is why a wrong key is detected rather than silently producing garbage.
//!
//! When the pack flags say the *index* is encrypted, this whole frame sits
//! between the entry count and the first entry, and no path can be read at all
//! without the key.
//!
//! Taarib never goes looking for the key. Godot stores it in the export
//! template, and a tool that recovered it from a shipped binary would be a tool
//! with a second purpose. The user supplies it or the package is not read.
//!
//! ## What a key gets you
//!
//! The framing is parsed completely — the digest, the length, the vector and the
//! exact span of the ciphertext are located and bounds-checked — and
//! [`fukk_tashfeer`] then decrypts with AES-256 in CBC mode.
//!
//! Godot zero-fills the final block and states the plaintext length separately,
//! so the decryptor is asked for **no** padding scheme and the result is trimmed
//! to the stated length. Asking for PKCS#7 would make it read a zero byte as a
//! padding length and refuse every correct package.
//!
//! A wrong key is never detected by the cipher: AES decrypts anything with
//! anything and yields noise. It is detected by the plaintext failing to hash to
//! the MD5 the frame recorded, which [`Hawiya::istakhrij`] checks before
//! the bytes go anywhere. **No decryption is faked and no ciphertext is ever
//! handed back as if it were content.**
//!
//! ## Digests
//!
//! Every entry carries an MD5 of its plaintext. This module verifies it whenever
//! it is not all zero, and a mismatch is [`KhataGodot::BasmaGhayrMutabaqa`] — see
//! [`super::basma_sifriya`] for why zero means "not recorded" rather than "the
//! empty digest".
//!
//! ## Writing: the additive patch package
//!
//! [`BinaHawiya`] writes one thing: an uncompressed, unencrypted version 2
//! package holding the patch's own resources. It is mounted at runtime with
//! `ProjectSettings.load_resource_pack`, which the engine supports natively.
//!
//! The semantics of that call are the reason the whole delivery works. A mounted
//! pack is layered **over** the packs already open, and a path present in more
//! than one resolves to the most recently mounted — so a patch that carries
//! `res://locale/ar.translation` supplies it whether or not the game had one,
//! and a patch that carries a path the game also has replaces it for as long as
//! the patch is mounted. Nothing in the game's own package is read, rewritten or
//! even opened for writing.
//!
//! Removal is therefore deleting one file. There is no uninstaller, no backup to
//! restore and no way for a half-finished removal to leave a game that will not
//! start: the patch package is either present and mounted, or absent and the
//! game is exactly what it was.

use std::path::{Path, PathBuf};

use aes::Aes256;
use cbc::cipher::block_padding::NoPadding;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};

use crate::khata::{KhataGodot, hajm_usize, tul_u64};

use super::{
    Katib, Mawrid, Qari, basma_md5, basma_sifriya, hashw_muhadhah, iftah_khareeta, sammi_masar,
    tahaqquq_adad, uktub_malaf,
};

/// The format's name, in every refusal this module raises.
pub const ISM: &str = "Godot package";

/// The four bytes a package begins with, and ends with when it is embedded.
pub const SIHR: [u8; 4] = *b"GDPC";

/// The same four bytes read as a little-endian `u32`, which is how the engine
/// compares them.
pub const SIHR_RAQM: u32 = 0x4350_4447;

/// Godot 3's pack format version.
pub const ISDAR_AWWAL: u32 = 1;

/// Godot 4's pack format version.
pub const ISDAR_THANI: u32 = 2;

/// The highest pack format version this build reads.
pub const ISDAR_AQSA: u32 = ISDAR_THANI;

/// How many reserved words both versions carry before the entry count.
pub const ADAD_MAHJUZ: usize = 16;

/// How many bytes the trailing record of an embedded package occupies.
pub const HAJM_DHAYL: u64 = 12;

/// [`TarwisatHawiya::alam`]: the entry index is encrypted, so no path in this
/// package can be read without the key.
pub const ALAM_FAHRAS_MUSHAFFAR: u32 = 1 << 0;

/// [`MadkhalHawiya::alam`]: this entry's stored bytes are ciphertext.
pub const ALAM_MADKHAL_MUSHAFFAR: u32 = 1 << 0;

/// How many bytes precede the ciphertext of an encrypted run.
///
/// Sixteen for the digest, eight for the plaintext length, sixteen for the
/// initialisation vector.
pub const HAJM_ITAR_TASHFEER: u64 = 40;

/// How many bytes a Godot encryption key is: AES-256 takes exactly thirty-two.
pub const HAJM_MIFTAH: usize = 32;

/// The AES block size, and therefore the multiple every ciphertext is padded to.
pub const HAJM_KUTLA_TASHFEER: u64 = 16;

/// The boundary a stored path is padded up to.
pub const MUHADHAT_MASAR: usize = 4;

/// The boundary [`BinaHawiya`] starts each entry's content on.
///
/// Thirty-two, which is what Godot's own packer defaults to. The format itself
/// requires no alignment at all — the reader honours whatever offsets the index
/// gives — but matching the engine's default means a package Taarib writes and a
/// package the engine writes differ only in their contents.
pub const MUHADHAT_MUHTAWA: u64 = 32;

/// The prefix every path inside a Godot project carries.
pub const BIDAYAT_MASAR: &str = "res://";

/// The largest number of entries this build will read an index for.
///
/// Four million. The largest shipped Godot game is in the low hundreds of
/// thousands of files; four million is an order of magnitude above that and
/// still small enough that the smallest possible entry — a four byte path and
/// its fixed fields — cannot be claimed by a package that does not hold them.
pub const AQSA_MADAKHIL: u64 = 4_000_000;

/// The largest stored path length this build accepts.
///
/// Four kibibytes, comfortably above every filesystem's own path limit. A path
/// field is the one variable-length field in the index, so it is the one a
/// corrupt package uses to ask for memory.
pub const AQSA_TUL_MASAR: u64 = 4096;

/// The largest single entry this build will extract into memory.
///
/// Five hundred and twelve mebibytes. Packages contain video and audio banks far
/// larger than anything Taarib reads, and this ceiling is not a claim about what
/// a game may contain — it is a bound on what this adapter will pull into a
/// running game's address space in one allocation.
pub const AQSA_MADKHAL: u64 = 512 * 1024 * 1024;

/// How many bytes the fixed part of an entry occupies, path excluded.
///
/// Four for the path length, eight for the offset, eight for the size, sixteen
/// for the digest. Version 2 adds four more for the flags.
const AQALL_MADKHAL: u64 = 36;

/// Which pack format version a package declares.
///
/// An enumeration rather than a number because everything downstream branches on
/// it — whether the header carries `alam` and `qaida`, whether an entry carries
/// flags, whether an entry can be encrypted at all — and a `u32` compared
/// against `2` in six places is six places to get it wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsdarHawiya {
    /// Version 1: Godot 3. No pack flags, no file base, no per-entry flags, and
    /// therefore no encryption.
    Awwal,
    /// Version 2: Godot 4. Pack flags, a file base every entry offset is
    /// measured from, and a flags word on each entry.
    Thani,
}

impl IsdarHawiya {
    /// The number the header carries.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        match self {
            Self::Awwal => ISDAR_AWWAL,
            Self::Thani => ISDAR_THANI,
        }
    }

    /// The version for a declared number, or [`None`] for one this build does
    /// not read.
    #[must_use]
    pub const fn min_raqm(raqm: u32) -> Option<Self> {
        match raqm {
            ISDAR_AWWAL => Some(Self::Awwal),
            ISDAR_THANI => Some(Self::Thani),
            _ => None,
        }
    }

    /// Whether this version's header carries the flags word and the file base.
    #[must_use]
    pub const fn bi_qaida(self) -> bool {
        matches!(self, Self::Thani)
    }

    /// How many bytes this version's header occupies, entry count included.
    #[must_use]
    pub const fn hajm_tarwisa(self) -> u64 {
        // magic, version, three engine words, sixteen reserved words, count.
        let asas: u64 = 4 + 4 + 12 + 64 + 4;
        match self {
            Self::Awwal => asas,
            // The flags word and the file base.
            Self::Thani => asas + 4 + 8,
        }
    }

    /// How many bytes the fixed part of one of this version's entries occupies.
    #[must_use]
    pub const fn aqall_madkhal(self) -> u64 {
        match self {
            Self::Awwal => AQALL_MADKHAL,
            Self::Thani => AQALL_MADKHAL + 4,
        }
    }
}

/// A package header, with everything this build does not interpret preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarwisatHawiya {
    /// The pack format version.
    pub isdar: IsdarHawiya,
    /// The engine's major, minor and patch version, as the exporter recorded
    /// them.
    ///
    /// Reported, never enforced. The engine refuses a package built by a newer
    /// build than itself; Taarib is not the engine, and refusing to *read* a
    /// package because the game that ships it is newer than some number in this
    /// crate would be refusing to translate a game for no reason.
    pub muharrik: (u32, u32, u32),
    /// The pack flags. Only [`ALAM_FAHRAS_MUSHAFFAR`] is defined.
    pub alam: u32,
    /// The file base: the offset within the package where the content region
    /// begins, which every version 2 entry offset is measured from. Always zero
    /// for version 1, which has no such field.
    pub qaida: u64,
    /// The sixteen reserved words, verbatim.
    ///
    /// Carried rather than checked. They are zero in every package the engine
    /// writes, but a package that puts something there is describing something
    /// this build does not know about, and zeroing it on a rewrite would discard
    /// that silently.
    pub mahjuz: [u32; ADAD_MAHJUZ],
}

impl TarwisatHawiya {
    /// Whether the entry index is encrypted.
    #[must_use]
    pub const fn fahras_mushaffar(&self) -> bool {
        self.alam & ALAM_FAHRAS_MUSHAFFAR != 0
    }

    /// The engine version as it is normally written.
    #[must_use]
    pub fn muharrik_nass(&self) -> String {
        let (rais, thani, thalith) = self.muharrik;
        format!("{rais}.{thani}.{thalith}")
    }
}

/// One file inside a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalHawiya {
    /// The path the engine will be asked for, normally beginning `res://`.
    pub masar: String,
    /// The entry's offset, measured from the content base and not from the file.
    /// See [`Hawiya::izahat_asas`].
    pub izaha: u64,
    /// How many bytes are stored. For an encrypted entry this counts the forty
    /// byte frame and the block-padded ciphertext, not the plaintext.
    pub hajm: u64,
    /// MD5 of the entry's plaintext, or sixteen zero bytes when the exporter did
    /// not record one.
    pub basma: [u8; 16],
    /// The entry flags. Only [`ALAM_MADKHAL_MUSHAFFAR`] is defined, and version 1
    /// entries have no flags word at all, so this is zero for every one of them.
    pub alam: u32,
    /// The path length exactly as the index stored it, padding included.
    ///
    /// Preserved so that writing an index back produces the bytes it was read
    /// from. The padded length is derivable — round the UTF-8 length up to four —
    /// but only for a packer that padded the way the engine does, and a package
    /// that padded further would come back four bytes shorter with every offset
    /// after it still pointing where it used to.
    tul_masar_makhzun: u32,
}

impl MadkhalHawiya {
    /// A new entry for a path and its content, with the digest computed.
    ///
    /// The only constructor that computes a digest, because it is the only one
    /// that has the plaintext in hand. Everything read from a package carries the
    /// digest the package recorded.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when `masar` does not begin with `res://`, and
    /// [`KhataGodot::HajmMufrit`] when its UTF-8 form is longer than
    /// [`AQSA_TUL_MASAR`] once padded.
    pub fn jadeed(masar: &str, muhtawa: &[u8], izaha: u64) -> Result<Self, KhataGodot> {
        if !masar.starts_with(BIDAYAT_MASAR) {
            return Err(KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "a patch entry path that does not begin with res://",
                qeema: tul_u64(masar.len()),
                hadd: tul_u64(BIDAYAT_MASAR.len()),
            });
        }
        let tul_masar_makhzun = tul_masar_madfu(masar)?;
        Ok(Self {
            masar: masar.to_owned(),
            izaha,
            hajm: tul_u64(muhtawa.len()),
            basma: basma_md5(muhtawa),
            alam: 0,
            tul_masar_makhzun,
        })
    }

    /// Whether this entry's stored bytes are ciphertext.
    #[must_use]
    pub const fn mushaffar(&self) -> bool {
        self.alam & ALAM_MADKHAL_MUSHAFFAR != 0
    }

    /// Whether this entry carries a digest to check against.
    #[must_use]
    pub fn bi_basma(&self) -> bool {
        !basma_sifriya(self.basma)
    }

    /// The path with its `res://` prefix removed, or the whole path when it does
    /// not carry one.
    #[must_use]
    pub fn masar_nisbi(&self) -> &str {
        self.masar
            .strip_prefix(BIDAYAT_MASAR)
            .unwrap_or(&self.masar)
    }

    /// The stored path length, padding included.
    #[must_use]
    pub const fn tul_masar_makhzun(&self) -> u32 {
        self.tul_masar_makhzun
    }

    /// How many bytes this entry occupies in the index.
    #[must_use]
    pub fn hajm_fi_al_fahras(&self, isdar: IsdarHawiya) -> u64 {
        isdar
            .aqall_madkhal()
            .saturating_add(u64::from(self.tul_masar_makhzun))
    }
}

/// The padded length a path is stored with, refusing one too long to store.
fn tul_masar_madfu(masar: &str) -> Result<u32, KhataGodot> {
    let khaam = masar.len();
    let madfu = khaam.saturating_add(hashw_muhadhah(khaam, MUHADHAT_MASAR));
    let tul = tul_u64(madfu);
    if tul > AQSA_TUL_MASAR {
        return Err(KhataGodot::HajmMufrit {
            haql: "a stored entry path",
            qeema: tul,
            saqf: AQSA_TUL_MASAR,
        });
    }
    u32::try_from(madfu).map_err(|_| KhataGodot::HajmMufrit {
        haql: "a stored entry path",
        qeema: tul,
        saqf: AQSA_TUL_MASAR,
    })
}

/// A length rounded up to a whole AES block, or [`None`] on overflow.
const fn ila_kutla(tul: u64) -> Option<u64> {
    let zaid = tul & (HAJM_KUTLA_TASHFEER - 1);
    if zaid == 0 {
        return Some(tul);
    }
    tul.checked_add(HAJM_KUTLA_TASHFEER - zaid)
}

/// The forty byte frame in front of every encrypted run.
///
/// Parsed in full whether or not this build can decrypt, because the frame is
/// what makes an encrypted package *describable*: it says how long the plaintext
/// is and exactly which bytes are ciphertext, so a report can tell a user what
/// the package holds and what supplying a key would get them, without any
/// pretence of having read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItarMushaffar {
    /// MD5 of the plaintext. The only test that a key was the right one.
    pub basma: [u8; 16],
    /// The plaintext's length, which is shorter than the ciphertext because CBC
    /// works in whole blocks.
    pub tul: u64,
    /// The CBC initialisation vector.
    pub muttajih: [u8; 16],
    /// Where the ciphertext begins, measured from the frame's first byte. Always
    /// [`HAJM_ITAR_TASHFEER`]; named so that no caller re-derives it.
    pub izahat_shifra: u64,
    /// How many ciphertext bytes there are: `tul` rounded up to a whole block.
    pub tul_shifra: u64,
}

impl ItarMushaffar {
    /// Reads the frame at the start of `bayt`.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] when the frame or its ciphertext runs past the
    /// end of `bayt`, [`KhataGodot::HajmMufrit`] when the declared plaintext
    /// length is above [`AQSA_MADKHAL`], and [`KhataGodot::HawiyaTalifa`] when
    /// rounding that length up to a block overflows.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot> {
        let mut qari = Qari::jadeed(ISM, bayt);
        let basma = qari.iqra_masfufa::<16>("an encrypted run's digest")?;
        let tul = qari.iqra_u64("an encrypted run's plaintext length")?;
        let muttajih = qari.iqra_masfufa::<16>("an encrypted run's initialisation vector")?;
        if tul > AQSA_MADKHAL {
            return Err(KhataGodot::HajmMufrit {
                haql: "an encrypted run's plaintext length",
                qeema: tul,
                saqf: AQSA_MADKHAL,
            });
        }
        let tul_shifra = ila_kutla(tul)
            .ok_or_else(|| qari.talif("an encrypted run's ciphertext length", tul, AQSA_MADKHAL))?;
        // Bounded here rather than at decryption time so that a frame naming
        // more ciphertext than exists is refused while it is still just a frame.
        let _ = qari.iqra_bayt("an encrypted run's ciphertext", tul_shifra)?;
        Ok(Self {
            basma,
            tul,
            muttajih,
            izahat_shifra: HAJM_ITAR_TASHFEER,
            tul_shifra,
        })
    }

    /// The ciphertext itself, given the run the frame introduces.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] when `bayt` is shorter than the frame said.
    pub fn shifra<'a>(&self, bayt: &'a [u8]) -> Result<&'a [u8], KhataGodot> {
        let mut qari = Qari::jadeed(ISM, bayt);
        qari.takhatta("an encrypted run's frame", self.izahat_shifra)?;
        qari.iqra_bayt("an encrypted run's ciphertext", self.tul_shifra)
    }
}

/// The header and the entry index: everything a package says about itself before
/// any content is touched.
///
/// This is the unit the round-trip contract applies to. [`Mawrid::min_bayt`]
/// takes a slice whose first byte is the package's magic — for a loose package
/// that is the file, for an embedded one it is the file from
/// [`Hawiya::bidaya`] — and [`Mawrid::ila_bayt`] returns the bytes from that
/// magic through the last entry, which for an unedited index is exactly the run
/// it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fahras {
    tarwisa: TarwisatHawiya,
    madakhil: Vec<MadkhalHawiya>,
    tul: u64,
}

impl Fahras {
    /// The header.
    #[must_use]
    pub const fn tarwisa(&self) -> &TarwisatHawiya {
        &self.tarwisa
    }

    /// Every entry, in the order the index lists them.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalHawiya] {
        &self.madakhil
    }

    /// How many bytes the header and the index occupy together.
    #[must_use]
    pub const fn tul(&self) -> u64 {
        self.tul
    }

    /// Reads one entry.
    fn madkhal(qari: &mut Qari<'_>, isdar: IsdarHawiya) -> Result<MadkhalHawiya, KhataGodot> {
        let tul_masar_makhzun = qari.iqra_u32("an entry's path length")?;
        let tul_masar = u64::from(tul_masar_makhzun);
        if tul_masar > AQSA_TUL_MASAR {
            return Err(KhataGodot::HajmMufrit {
                haql: "an entry's path length",
                qeema: tul_masar,
                saqf: AQSA_TUL_MASAR,
            });
        }
        let khaam = qari.iqra_bayt("an entry's path", tul_masar)?;
        // The declared length counts the NUL padding, so the text is what is
        // left after it is trimmed. Trimming before decoding rather than after
        // is deliberate: a decoder handed the NULs produces a String with
        // embedded NULs, which compares unequal to the path the engine asks for
        // and fails a lookup that should have succeeded.
        let bila_hashw = khaam
            .iter()
            .rposition(|wahid| *wahid != 0)
            .map_or(0, |akhir| akhir.saturating_add(1));
        let masar = qari.nass(khaam.get(..bila_hashw).unwrap_or(&[]))?;
        let izaha = qari.iqra_u64("an entry's offset")?;
        let hajm = qari.iqra_u64("an entry's size")?;
        let basma = qari.iqra_masfufa::<16>("an entry's digest")?;
        let alam = if isdar.bi_qaida() {
            qari.iqra_u32("an entry's flags")?
        } else {
            0
        };
        // Checked here so that no later arithmetic on this entry can wrap. The
        // *containment* check — does this run actually lie inside the package —
        // belongs to `Hawiya`, which is the only thing that knows how long the
        // container is.
        if izaha.checked_add(hajm).is_none() {
            return Err(qari.talif("an entry whose offset plus size overflows", izaha, u64::MAX));
        }
        Ok(MadkhalHawiya {
            masar,
            izaha,
            hajm,
            basma,
            alam,
            tul_masar_makhzun,
        })
    }
}

impl Mawrid for Fahras {
    const ISM: &'static str = ISM;

    /// Reads a package's header and its whole index.
    ///
    /// The checks run in this order, and not in a more convenient one, because
    /// each depends on the one before it being true:
    ///
    /// 1. **magic** — the cheapest way to reject a file that is not a package,
    ///    and what stops every later message from being confusing;
    /// 2. **version** — before any *other* field is interpreted, because the
    ///    meaning of every offset below is a property of the version. A parse
    ///    that reads fields first and checks the version afterwards has already
    ///    read fields whose meaning it guessed;
    /// 3. **encryption** — before the entry count is used for anything, because
    ///    an encrypted index means the bytes after the count are ciphertext and
    ///    parsing them as entries would produce a table of noise;
    /// 4. **entry count** — against [`AQSA_MADAKHIL`] and against the bytes
    ///    actually left, before a single element is reserved for.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::SihrGhayrMutabaq`] when `bayt` does not begin with the
    /// magic — with an empty path, because a slice reader cannot know where its
    /// bytes came from; see [`super::sammi_masar`]. [`KhataGodot::IsdarGhayrMadum`]
    /// for a format version outside 1 and 2. [`KhataGodot::PckMushaffar`] when
    /// the index is encrypted. [`KhataGodot::MalafQaseer`] when a field runs past
    /// the end, [`KhataGodot::HajmMufrit`] for a count or a path length above its
    /// ceiling, [`KhataGodot::HawiyaTalifa`] for a field inconsistent with the
    /// package, and [`KhataGodot::NassGhayrSalih`] for a path that is not valid
    /// UTF-8.
    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot> {
        let mut qari = Qari::jadeed(ISM, bayt);
        let sihr = qari.iqra_masfufa::<4>("the package magic")?;
        if sihr != SIHR {
            return Err(KhataGodot::SihrGhayrMutabaq {
                masar: PathBuf::new(),
            });
        }
        let raqm = qari.iqra_u32("the pack format version")?;
        let isdar = IsdarHawiya::min_raqm(raqm).ok_or(KhataGodot::IsdarGhayrMadum {
            wujid: raqm,
            aqsa: ISDAR_AQSA,
        })?;
        let muharrik = (
            qari.iqra_u32("the engine major version")?,
            qari.iqra_u32("the engine minor version")?,
            qari.iqra_u32("the engine patch version")?,
        );
        let (alam, qaida) = if isdar.bi_qaida() {
            (
                qari.iqra_u32("the pack flags")?,
                qari.iqra_u64("the file base")?,
            )
        } else {
            (0, 0)
        };
        let mut mahjuz = [0u32; ADAD_MAHJUZ];
        for kalima in &mut mahjuz {
            *kalima = qari.iqra_u32("a reserved header word")?;
        }
        let adad = qari.iqra_u32("the entry count")?;
        let tarwisa = TarwisatHawiya {
            isdar,
            muharrik,
            alam,
            qaida,
            mahjuz,
        };

        if tarwisa.fahras_mushaffar() {
            // Validated and then refused. Reading the frame proves the package
            // really is an encrypted one rather than a corrupt one, so the user
            // is told "supply the key" instead of "this file is damaged".
            let _ = ItarMushaffar::min_bayt(qari.baqiya())?;
            return Err(KhataGodot::PckMushaffar {
                masar: PathBuf::new(),
            });
        }

        let matlub = tahaqquq_adad(
            ISM,
            "the entry count",
            u64::from(adad),
            AQSA_MADAKHIL,
            isdar.aqall_madkhal(),
            qari.baqi(),
        )?;
        let mut madakhil = Vec::with_capacity(matlub);
        for _ in 0..matlub {
            madakhil.push(Self::madkhal(&mut qari, isdar)?);
        }
        Ok(Self {
            tarwisa,
            madakhil,
            tul: tul_u64(qari.mawqi()),
        })
    }

    /// Writes the header and the index back out.
    ///
    /// Byte-identical to what [`Mawrid::min_bayt`] read when nothing was edited:
    /// the reserved words are the ones the package carried, each path is written
    /// at the padded length the package stored, and every offset and digest is
    /// the one the package recorded.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when there are more entries than a `u32` can
    /// count or a path is longer than its stored length can hold, and
    /// [`KhataGodot::HawiyaTalifa`] when an entry carries flags in a version 1
    /// package, which has nowhere to put them.
    fn ila_bayt(&self) -> Result<Vec<u8>, KhataGodot> {
        let isdar = self.tarwisa.isdar;
        let adad = u32::try_from(self.madakhil.len()).map_err(|_| KhataGodot::HajmMufrit {
            haql: "the entry count",
            qeema: tul_u64(self.madakhil.len()),
            saqf: AQSA_MADAKHIL,
        })?;
        let mut katib = Katib::bi_siaa(hajm_usize(self.tul).unwrap_or(0));
        katib.uktub_bayt(&SIHR);
        katib.uktub_u32(isdar.raqm());
        let (rais, thani, thalith) = self.tarwisa.muharrik;
        katib.uktub_u32(rais);
        katib.uktub_u32(thani);
        katib.uktub_u32(thalith);
        if isdar.bi_qaida() {
            katib.uktub_u32(self.tarwisa.alam);
            katib.uktub_u64(self.tarwisa.qaida);
        } else if self.tarwisa.alam != 0 {
            return Err(KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "pack flags on a version 1 package, which has no flags field",
                qeema: u64::from(self.tarwisa.alam),
                hadd: 0,
            });
        }
        for kalima in self.tarwisa.mahjuz {
            katib.uktub_u32(kalima);
        }
        katib.uktub_u32(adad);
        for madkhal in &self.madakhil {
            uktub_madkhal(&mut katib, madkhal, isdar)?;
        }
        Ok(katib.ila_vec())
    }
}

/// Writes one entry, padding its path to the length the index declares.
fn uktub_madkhal(
    katib: &mut Katib,
    madkhal: &MadkhalHawiya,
    isdar: IsdarHawiya,
) -> Result<(), KhataGodot> {
    let makhzun = madkhal.tul_masar_makhzun;
    let khaam = madkhal.masar.as_bytes();
    let hashw = usize::try_from(makhzun)
        .ok()
        .and_then(|makhzun| makhzun.checked_sub(khaam.len()))
        .ok_or_else(|| KhataGodot::HajmMufrit {
            haql: "an entry path longer than its stored length",
            qeema: tul_u64(khaam.len()),
            saqf: u64::from(makhzun),
        })?;
    katib.uktub_u32(makhzun);
    katib.uktub_bayt(khaam);
    katib.uktub_asfar(hashw);
    katib.uktub_u64(madkhal.izaha);
    katib.uktub_u64(madkhal.hajm);
    katib.uktub_bayt(&madkhal.basma);
    if isdar.bi_qaida() {
        katib.uktub_u32(madkhal.alam);
    } else if madkhal.alam != 0 {
        return Err(KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql: "entry flags on a version 1 package, which has no flags field",
            qeema: u64::from(madkhal.alam),
            hadd: 0,
        });
    }
    Ok(())
}

/// Finds where the package starts, reading the tail before the head.
///
/// Returns the offset of the package's magic and whether it was found through
/// the trailing record — that is, whether this is an executable with a package
/// appended rather than a package of its own.
///
/// The order is the whole trick. A loose `.pck` announces itself at offset zero;
/// an executable announces a PE, ELF or Mach-O header there and says nothing at
/// all about the archive glued to its end, because appending bytes to a file
/// does not update the headers at its start. So when offset zero is not the
/// magic, the last twelve bytes are read instead: four bytes of magic saying a
/// package is present, and before them a distance back to where it begins. That
/// distance is then confirmed by finding the magic a second time at the place it
/// points to — which is what separates a real embedded package from twelve bytes
/// that happen to end in `GDPC`, and from a file that was truncated in transit.
///
/// # Errors
///
/// [`KhataGodot::SihrGhayrMutabaq`] with an empty path when neither the head nor
/// the tail carries the magic, or when the distance does not land on it;
/// [`KhataGodot::MalafQaseer`] when the file is too short to hold the record;
/// and [`KhataGodot::HawiyaTalifa`] when the distance points outside the file.
pub fn ijad_bidaya(bayt: &[u8]) -> Result<(u64, bool), KhataGodot> {
    if bayt.get(..SIHR.len()) == Some(SIHR.as_slice()) {
        return Ok((0, false));
    }
    let tul = tul_u64(bayt.len());
    let izahat_dhayl = tul.checked_sub(HAJM_DHAYL).ok_or(KhataGodot::MalafQaseer {
        haql: "the trailing record of an embedded package",
        tul,
        matlub: HAJM_DHAYL,
    })?;
    let mut qari = Qari::jadeed(ISM, bayt);
    qari.iqfiz("the trailing record of an embedded package", izahat_dhayl)?;
    let masafa = qari.iqra_u64("the embedded package's distance")?;
    let sihr = qari.iqra_masfufa::<4>("the trailing magic")?;
    if sihr != SIHR {
        return Err(KhataGodot::SihrGhayrMutabaq {
            masar: PathBuf::new(),
        });
    }
    let bidaya = izahat_dhayl
        .checked_sub(masafa)
        .ok_or(KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql: "the embedded package's distance, which reaches before the file begins",
            qeema: masafa,
            hadd: izahat_dhayl,
        })?;
    qari.iqfiz("the embedded package's start", bidaya)?;
    let mukarrar = qari.iqra_masfufa::<4>("the embedded package's magic")?;
    if mukarrar != SIHR {
        return Err(KhataGodot::SihrGhayrMutabaq {
            masar: PathBuf::new(),
        });
    }
    Ok((bidaya, true))
}

/// A package located inside a container and validated against it.
///
/// [`Fahras`] answers what the index *says*; this answers whether the container
/// agrees. Every entry's absolute range is computed once, here, and checked
/// against the real length of the bytes it was found in — so nothing downstream
/// re-derives an offset and nothing downstream can read past the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hawiya {
    fahras: Fahras,
    bidaya: u64,
    mudmaj: bool,
    tul: u64,
}

impl Hawiya {
    /// Locates, reads and validates a package inside `bayt`.
    ///
    /// `bayt` is the whole container: a loose `.pck`, or the executable a
    /// package is embedded in. It is not the package alone, because an embedded
    /// package cannot be found without its file's last twelve bytes.
    ///
    /// # Errors
    ///
    /// Everything [`ijad_bidaya`] and [`Fahras::min_bayt`](Mawrid::min_bayt)
    /// raise, and [`KhataGodot::HawiyaTalifa`] when an entry's offset and size
    /// place it outside the container — which is the check that makes every
    /// later read of this package a read of bytes that exist.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot> {
        let (bidaya, mudmaj) = ijad_bidaya(bayt)?;
        let mawqi = hajm_usize(bidaya).ok_or_else(|| KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql: "the package's start",
            qeema: bidaya,
            hadd: tul_u64(bayt.len()),
        })?;
        let hawiya = bayt.get(mawqi..).ok_or_else(|| KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql: "the package's start",
            qeema: bidaya,
            hadd: tul_u64(bayt.len()),
        })?;
        let fahras = Fahras::min_bayt(hawiya)?;
        let natija = Self {
            fahras,
            bidaya,
            mudmaj,
            tul: tul_u64(bayt.len()),
        };
        for madkhal in natija.fahras.madakhil() {
            let _ = natija.mada(madkhal)?;
        }
        Ok(natija)
    }

    /// The header and index.
    #[must_use]
    pub const fn fahras(&self) -> &Fahras {
        &self.fahras
    }

    /// The header.
    #[must_use]
    pub const fn tarwisa(&self) -> &TarwisatHawiya {
        self.fahras.tarwisa()
    }

    /// Every entry, in index order.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalHawiya] {
        self.fahras.madakhil()
    }

    /// Where the package's magic sits in the container. Zero for a loose one.
    #[must_use]
    pub const fn bidaya(&self) -> u64 {
        self.bidaya
    }

    /// Whether the package was found through the trailing record, meaning it is
    /// appended to an executable.
    ///
    /// The one property that changes what this adapter is allowed to do: an
    /// embedded package is read and never touched, because rewriting it means
    /// rewriting the executable and invalidating its signature.
    #[must_use]
    pub const fn mudmaj(&self) -> bool {
        self.mudmaj
    }

    /// The offset every entry's own offset is measured from.
    ///
    /// The package's own start, plus the file base for a version 2 package. This
    /// is the only place the two are added together; everything else asks here.
    #[must_use]
    pub const fn izahat_asas(&self) -> u64 {
        self.bidaya.saturating_add(self.fahras.tarwisa().qaida)
    }

    /// The entry for an exact `res://` path, if the package holds it.
    #[must_use]
    pub fn madkhal(&self, masar: &str) -> Option<&MadkhalHawiya> {
        self.madakhil()
            .iter()
            .find(|madkhal| madkhal.masar == masar)
    }

    /// Where an entry's stored bytes begin and end in the container.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when the range overflows or reaches past the
    /// end of the container.
    pub fn mada(&self, madkhal: &MadkhalHawiya) -> Result<(u64, u64), KhataGodot> {
        let bidaya =
            self.izahat_asas()
                .checked_add(madkhal.izaha)
                .ok_or(KhataGodot::HawiyaTalifa {
                    ism: ISM,
                    haql: "an entry offset that overflows the container",
                    qeema: madkhal.izaha,
                    hadd: self.tul,
                })?;
        let nihaya = bidaya
            .checked_add(madkhal.hajm)
            .filter(|nihaya| *nihaya <= self.tul);
        let nihaya = nihaya.ok_or_else(|| KhataGodot::HawiyaTalifa {
            ism: ISM,
            haql: "an entry that reaches past the end of the container",
            qeema: bidaya.saturating_add(madkhal.hajm),
            hadd: self.tul,
        })?;
        Ok((bidaya, nihaya))
    }

    /// An entry's stored bytes, without decrypting or verifying anything.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when the entry does not lie inside `bayt`,
    /// which is also raised when `bayt` is not the container this package was
    /// read from.
    pub fn bayt_madkhal<'a>(
        &self,
        bayt: &'a [u8],
        madkhal: &MadkhalHawiya,
    ) -> Result<&'a [u8], KhataGodot> {
        let (bidaya, nihaya) = self.mada(madkhal)?;
        let awwal = hajm_usize(bidaya);
        let akhir = hajm_usize(nihaya);
        match (awwal, akhir) {
            (Some(awwal), Some(akhir)) => {
                bayt.get(awwal..akhir)
                    .ok_or_else(|| KhataGodot::HawiyaTalifa {
                        ism: ISM,
                        haql: "an entry that reaches past the end of the container",
                        qeema: nihaya,
                        hadd: tul_u64(bayt.len()),
                    })
            },
            _ => Err(KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "an entry whose range does not fit this platform's addresses",
                qeema: nihaya,
                hadd: tul_u64(bayt.len()),
            }),
        }
    }
}

impl Hawiya {
    /// Reads one entry out: decrypted if it was encrypted, verified against its
    /// recorded digest, and returned as plaintext.
    ///
    /// The order is: bound, then locate, then decrypt, then verify. Verification
    /// last is not a preference — an entry's digest is over its *plaintext*,
    /// because that is what the exporter hashed before it encrypted anything, so
    /// there is nothing to check until the plaintext exists. An unencrypted
    /// entry's stored bytes are its plaintext and the two steps collapse.
    ///
    /// An encrypted entry is checked twice, against two independent digests: the
    /// one inside the frame, which is what proves the key was right, and the one
    /// in the index, which is what proves the package was not altered. They are
    /// the same value in a package the engine wrote, and checking both costs one
    /// comparison and catches a package where they disagree.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when the entry is larger than [`AQSA_MADKHAL`],
    /// [`KhataGodot::HawiyaTalifa`] when it does not lie inside `bayt`,
    /// [`KhataGodot::PckMushaffar`] when it is encrypted and `miftah` is [`None`],
    /// [`KhataGodot::MiftahGhayrSalih`] when a key was supplied — see
    /// [`fukk_tashfeer`] for what this build can and cannot do with one — and
    /// [`KhataGodot::BasmaGhayrMutabaqa`] naming the entry when its bytes do not
    /// match a digest it recorded.
    pub fn istakhrij(
        &self,
        bayt: &[u8],
        madkhal: &MadkhalHawiya,
        miftah: Option<&[u8; HAJM_MIFTAH]>,
    ) -> Result<Vec<u8>, KhataGodot> {
        if madkhal.hajm > AQSA_MADKHAL {
            return Err(KhataGodot::HajmMufrit {
                haql: "an entry's stored size",
                qeema: madkhal.hajm,
                saqf: AQSA_MADKHAL,
            });
        }
        let makhzun = self.bayt_madkhal(bayt, madkhal)?;
        let wadih = if madkhal.mushaffar() {
            let itar = ItarMushaffar::min_bayt(makhzun)?;
            let miftah = miftah.ok_or(KhataGodot::PckMushaffar {
                masar: PathBuf::new(),
            })?;
            let wadih = fukk_tashfeer(miftah, &itar, itar.shifra(makhzun)?)?;
            if tul_u64(wadih.len()) != itar.tul || basma_md5(&wadih) != itar.basma {
                return Err(KhataGodot::MiftahGhayrSalih {
                    masar: PathBuf::new(),
                    sabab: "the decrypted bytes do not match the digest inside the entry, \
                            which is what a wrong key looks like",
                });
            }
            wadih
        } else {
            makhzun.to_vec()
        };
        if madkhal.bi_basma() && basma_md5(&wadih) != madkhal.basma {
            return Err(KhataGodot::BasmaGhayrMutabaqa {
                madkhal: madkhal.masar.clone(),
            });
        }
        Ok(wadih)
    }

    /// Reads out the entry at an exact `res://` path.
    ///
    /// # Errors
    ///
    /// Everything [`Hawiya::istakhrij`] raises, and
    /// [`KhataGodot::HawiyaTalifa`] naming the path when the package does not
    /// hold it.
    pub fn istakhrij_masar(
        &self,
        bayt: &[u8],
        masar: &str,
        miftah: Option<&[u8; HAJM_MIFTAH]>,
    ) -> Result<Vec<u8>, KhataGodot> {
        let madkhal = self
            .madkhal(masar)
            .ok_or_else(|| KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "a path this package does not hold",
                qeema: tul_u64(masar.len()),
                hadd: tul_u64(self.madakhil().len()),
            })?;
        self.istakhrij(bayt, madkhal, miftah)
    }
}

/// Decrypts one AES-256-CBC run.
///
/// The cipher is `aes` and `cbc` from this crate's manifest, driven with **no**
/// padding scheme: Godot zero-fills the final block and states the plaintext
/// length in the frame separately, so asking the decryptor to strip PKCS#7 would
/// make it read a zero byte as a padding length and refuse every correct
/// package. The result is trimmed to the stated length instead.
///
/// Nothing here can tell a right key from a wrong one. AES decrypts any bytes
/// with any key and produces noise, so this function's success means "the
/// ciphertext was a whole number of blocks and the cipher ran", and the only
/// real test is the plaintext hashing to the digest the frame recorded — which
/// [`Hawiya::istakhrij`] performs, against both the frame's digest and the
/// index's, before the bytes go anywhere.
///
/// # Errors
///
/// [`KhataGodot::MiftahGhayrSalih`] when the ciphertext is not a whole number of
/// sixteen-byte blocks, when the frame states a plaintext longer than its own
/// ciphertext, or when the cipher refuses the buffer; and
/// [`KhataGodot::HajmMufrit`] when the stated plaintext length does not fit this
/// platform's addresses. Each is raised with an empty path — the caller that
/// knows the file's name fills it in through [`super::sammi_masar`].
pub fn fukk_tashfeer(
    miftah: &[u8; HAJM_MIFTAH],
    itar: &ItarMushaffar,
    shifra: &[u8],
) -> Result<Vec<u8>, KhataGodot> {
    // CBC consumes whole blocks. A ciphertext that is not a multiple of the
    // block size did not come out of a correct writer, and feeding it to the
    // decryptor would produce a partial final block of noise that the digest
    // check would then blame on the key.
    let tul_shifra = tul_u64(shifra.len());
    if shifra.is_empty() || tul_shifra & (HAJM_KUTLA_TASHFEER - 1) != 0 {
        return Err(KhataGodot::MiftahGhayrSalih {
            masar: PathBuf::new(),
            sabab: "the ciphertext is not a whole number of 16-byte blocks, which no \
                    correctly written package produces",
        });
    }

    // The frame states the plaintext length separately because Godot pads the
    // final block with zeros rather than with a self-describing scheme, so the
    // padding cannot be recovered from the plaintext. A stated length longer
    // than the ciphertext is a frame that disagrees with itself.
    if itar.tul > tul_shifra {
        return Err(KhataGodot::MiftahGhayrSalih {
            masar: PathBuf::new(),
            sabab: "the frame states a plaintext longer than its own ciphertext",
        });
    }
    let Some(siaa) = hajm_usize(itar.tul) else {
        return Err(KhataGodot::HajmMufrit {
            haql: "an encrypted run's plaintext length",
            qeema: itar.tul,
            saqf: tul_u64(usize::MAX),
        });
    };

    let mut hajz = shifra.to_vec();
    let mufakkik = MufakkikCbc::new(miftah.into(), (&itar.muttajih).into());

    // NoPadding, deliberately: Godot zero-fills the last block and relies on
    // the stated length, so asking the decryptor to strip PKCS#7 would make it
    // read a zero byte as a padding length and refuse every correct package.
    // The returned slice is the whole buffer, so it is discarded and the
    // buffer is trimmed to the stated length instead.
    mufakkik
        .decrypt_padded_mut::<NoPadding>(&mut hajz)
        .map_err(|_| KhataGodot::MiftahGhayrSalih {
            masar: PathBuf::new(),
            sabab: "the ciphertext could not be decrypted with the supplied key",
        })?;
    hajz.truncate(siaa);

    // The caller compares this against both the frame's digest and the index's,
    // which is the only honest test of a key: AES will decrypt anything with
    // any key and produce noise, so a wrong key is detected by the plaintext
    // not hashing to what the package recorded, never by the cipher failing.
    Ok(hajz)
}

/// AES-256 in CBC mode, which is what Godot encrypts a package with.
type MufakkikCbc = cbc::Decryptor<Aes256>;

/// A package opened from a path, with the mapping it was read through.
///
/// The mapping has to be held for the package to stay readable: [`Hawiya`] is an
/// index into bytes, not a copy of them, which is the entire point on a container
/// measured in gigabytes.
#[derive(Debug)]
pub struct HawiyaMaftuha {
    masar: PathBuf,
    khareeta: memmap2::Mmap,
    hawiya: Hawiya,
}

impl HawiyaMaftuha {
    /// Maps a package and reads its index.
    ///
    /// # Errors
    ///
    /// Everything [`super::iftah_khareeta`] and [`Hawiya::min_bayt`] raise, with
    /// the path filled into the refusals that could not name it.
    pub fn iftah(masar: &Path) -> Result<Self, KhataGodot> {
        let khareeta = iftah_khareeta(masar)?;
        let hawiya = Hawiya::min_bayt(&khareeta).map_err(|khata| sammi_masar(khata, masar))?;
        Ok(Self {
            masar: masar.to_path_buf(),
            khareeta,
            hawiya,
        })
    }

    /// Where the package was opened from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// The package.
    #[must_use]
    pub const fn hawiya(&self) -> &Hawiya {
        &self.hawiya
    }

    /// The mapped container.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.khareeta
    }

    /// Reads out the entry at an exact `res://` path.
    ///
    /// # Errors
    ///
    /// Everything [`Hawiya::istakhrij_masar`] raises, with this package's path
    /// filled in.
    pub fn istakhrij(
        &self,
        masar: &str,
        miftah: Option<&[u8; HAJM_MIFTAH]>,
    ) -> Result<Vec<u8>, KhataGodot> {
        self.hawiya
            .istakhrij_masar(self.bayt(), masar, miftah)
            .map_err(|khata| sammi_masar(khata, &self.masar))
    }
}

/// The largest package [`BinaHawiya`] will produce.
///
/// Five hundred and twelve mebibytes, and it is a bound on Taarib's own output
/// rather than on the format. A patch package holds a font, a glyph atlas and a
/// translation resource; anything approaching this size is a mistake upstream,
/// caught before the buffer is assembled rather than when the write fails.
pub const AQSA_HAWIYA_MAKTUBA: u64 = 512 * 1024 * 1024;

/// Padding for a `u64` length, aligned the same way [`hashw_muhadhah`] aligns a
/// `usize` one.
const fn hashw_u64(tul: u64, muhadhah: u64) -> u64 {
    let qinaa = muhadhah.wrapping_sub(1);
    if muhadhah == 0 || muhadhah & qinaa != 0 {
        return 0;
    }
    let zaid = tul & qinaa;
    if zaid == 0 {
        0
    } else {
        muhadhah.saturating_sub(zaid)
    }
}

/// Builds the additive patch package.
///
/// Uncompressed and unencrypted, by design in both cases. The PCK format has no
/// per-entry compression at all — a Godot package stores whatever bytes the
/// resource itself compressed into — so there is nothing to choose there. And
/// the patch is not encrypted because encrypting it would need the game's own
/// key, which Taarib does not have and does not look for; a mounted pack does not
/// have to match the encryption of the pack it is mounted over.
///
/// The result is mounted with `ProjectSettings.load_resource_pack`. See this
/// module's header for what that means: the patch's paths are layered over the
/// game's, the game's own package is never opened for writing, and removing the
/// patch is deleting the file.
#[derive(Debug, Clone)]
pub struct BinaHawiya {
    isdar: IsdarHawiya,
    muharrik: (u32, u32, u32),
    muhadhah: u64,
    madakhil: Vec<(String, Vec<u8>)>,
    majmu: u64,
}

impl BinaHawiya {
    /// A builder for a package the given engine version will mount.
    ///
    /// The version written into the header is the game's own, not Taarib's.
    /// Godot refuses a package built by a newer engine than itself, so a patch
    /// that declared the latest release would be refused by every older game —
    /// which is most of them.
    #[must_use]
    pub const fn jadeed(isdar: IsdarHawiya, muharrik: (u32, u32, u32)) -> Self {
        Self {
            isdar,
            muharrik,
            muhadhah: MUHADHAT_MUHTAWA,
            madakhil: Vec::new(),
            majmu: 0,
        }
    }

    /// Changes the boundary each entry's content starts on.
    ///
    /// Ignored unless it is a power of two, because the padding is computed with
    /// a mask; an alignment that is not one would otherwise produce offsets the
    /// header does not describe.
    #[must_use]
    pub const fn bi_muhadhah(mut self, muhadhah: u64) -> Self {
        if muhadhah != 0 && muhadhah & muhadhah.wrapping_sub(1) == 0 {
            self.muhadhah = muhadhah;
        }
        self
    }

    /// Adds one file at a `res://` path.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when the path does not begin with `res://` or
    /// the package already holds it, and [`KhataGodot::HajmMufrit`] when the path
    /// is longer than [`AQSA_TUL_MASAR`], there are already [`AQSA_MADAKHIL`]
    /// entries, or the contents together exceed [`AQSA_HAWIYA_MAKTUBA`].
    pub fn daa(&mut self, masar: &str, muhtawa: Vec<u8>) -> Result<(), KhataGodot> {
        if !masar.starts_with(BIDAYAT_MASAR) {
            return Err(KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "a patch entry path that does not begin with res://",
                qeema: tul_u64(masar.len()),
                hadd: tul_u64(BIDAYAT_MASAR.len()),
            });
        }
        if self
            .madakhil
            .iter()
            .any(|(mawjud, _)| mawjud.as_str() == masar)
        {
            // Two entries at one path is a package whose meaning depends on
            // which one the engine happens to reach first. Refused while it is
            // still a mistake in the patch's own manifest.
            return Err(KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "a patch entry path added twice",
                qeema: tul_u64(masar.len()),
                hadd: tul_u64(self.madakhil.len()),
            });
        }
        let _ = tul_masar_madfu(masar)?;
        if tul_u64(self.madakhil.len()) >= AQSA_MADAKHIL {
            return Err(KhataGodot::HajmMufrit {
                haql: "the patch package's entry count",
                qeema: tul_u64(self.madakhil.len()).saturating_add(1),
                saqf: AQSA_MADAKHIL,
            });
        }
        let majmu = self.majmu.saturating_add(tul_u64(muhtawa.len()));
        if majmu > AQSA_HAWIYA_MAKTUBA {
            return Err(KhataGodot::HajmMufrit {
                haql: "the patch package's total content size",
                qeema: majmu,
                saqf: AQSA_HAWIYA_MAKTUBA,
            });
        }
        self.majmu = majmu;
        self.madakhil.push((masar.to_owned(), muhtawa));
        Ok(())
    }

    /// How many entries the package will hold.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// Whether nothing has been added yet.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.madakhil.is_empty()
    }

    /// Assembles the package.
    ///
    /// Two passes, because the header cannot be written until the index length is
    /// known and the index cannot be written until every offset is known. The
    /// first pass measures each entry's padded path and adds up the index; the
    /// second builds the real [`Fahras`] with the offsets that follow from it and
    /// asks *that* to write the header and index. Writing the index through the
    /// same code the reader round-trips against is what keeps the two from
    /// drifting apart — a writer with its own private layout is a writer that can
    /// be wrong in a way no read-write-compare test would ever notice.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when a path or the assembled package exceeds
    /// its ceiling, and [`KhataGodot::HawiyaTalifa`] when a computed offset
    /// overflows, which cannot happen below the ceilings and is checked anyway.
    pub fn ila_bayt(&self) -> Result<Vec<u8>, KhataGodot> {
        let isdar = self.isdar;
        let mut tul_fahras = isdar.hajm_tarwisa();
        for (masar, _) in &self.madakhil {
            let tul_masar = u64::from(tul_masar_madfu(masar)?);
            tul_fahras = tul_fahras
                .checked_add(isdar.aqall_madkhal())
                .and_then(|majmu| majmu.checked_add(tul_masar))
                .ok_or(KhataGodot::HajmMufrit {
                    haql: "the patch package's index",
                    qeema: u64::MAX,
                    saqf: AQSA_HAWIYA_MAKTUBA,
                })?;
        }
        let bidayat_muhtawa = tul_fahras
            .checked_add(hashw_u64(tul_fahras, self.muhadhah))
            .ok_or(KhataGodot::HajmMufrit {
                haql: "the patch package's content base",
                qeema: u64::MAX,
                saqf: AQSA_HAWIYA_MAKTUBA,
            })?;
        // Version 2 measures entry offsets from the file base; version 1 has no
        // such field and measures them from the start of the package.
        let qaida = if isdar.bi_qaida() { bidayat_muhtawa } else { 0 };

        let mut madakhil = Vec::with_capacity(self.madakhil.len());
        let mut mawqi = bidayat_muhtawa;
        for (masar, muhtawa) in &self.madakhil {
            let izaha = mawqi.checked_sub(qaida).ok_or(KhataGodot::HawiyaTalifa {
                ism: ISM,
                haql: "a computed entry offset below the file base",
                qeema: mawqi,
                hadd: qaida,
            })?;
            madakhil.push(MadkhalHawiya::jadeed(masar, muhtawa, izaha)?);
            let baad = mawqi
                .checked_add(tul_u64(muhtawa.len()))
                .ok_or(KhataGodot::HajmMufrit {
                    haql: "the patch package's total size",
                    qeema: u64::MAX,
                    saqf: AQSA_HAWIYA_MAKTUBA,
                })?;
            mawqi =
                baad.checked_add(hashw_u64(baad, self.muhadhah))
                    .ok_or(KhataGodot::HajmMufrit {
                        haql: "the patch package's total size",
                        qeema: u64::MAX,
                        saqf: AQSA_HAWIYA_MAKTUBA,
                    })?;
        }
        if mawqi > AQSA_HAWIYA_MAKTUBA {
            return Err(KhataGodot::HajmMufrit {
                haql: "the patch package's total size",
                qeema: mawqi,
                saqf: AQSA_HAWIYA_MAKTUBA,
            });
        }

        let tarwisa = TarwisatHawiya {
            isdar,
            muharrik: self.muharrik,
            alam: 0,
            qaida,
            mahjuz: [0u32; ADAD_MAHJUZ],
        };
        let fahras = Fahras {
            tarwisa,
            madakhil,
            tul: tul_fahras,
        };
        let mut katib = Katib::bi_siaa(hajm_usize(mawqi).unwrap_or(0));
        katib.uktub_bayt(&fahras.ila_bayt()?);
        let muhadhah = hajm_usize(self.muhadhah).unwrap_or(1);
        katib.hadhi(muhadhah);
        for (_, muhtawa) in &self.madakhil {
            katib.uktub_bayt(muhtawa);
            katib.hadhi(muhadhah);
        }
        Ok(katib.ila_vec())
    }

    /// Assembles the package and writes it.
    ///
    /// # Errors
    ///
    /// Everything [`BinaHawiya::ila_bayt`] raises, and
    /// [`KhataGodot::KhataMalaf`] naming the path.
    pub fn uktub(&self, masar: &Path) -> Result<(), KhataGodot> {
        uktub_malaf(masar, &self.ila_bayt()?)
    }
}
