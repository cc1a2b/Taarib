//! التوقيع — the signature block: its layout, what it commits to, and how it
//! is checked.
//!
//! **This module verifies. It cannot sign.** There is no secret key type here,
//! no key generation, no signing function, and no dependency that could provide
//! one — `taarib-ruqaa` does not link `ed25519-dalek` at all. Signing lives in
//! `taarib-khatm`, behind the owner's key, and Decision 8 says there is exactly
//! one place a patch can be sealed. A format crate that could also sign would
//! be a second such place.
//!
//! Even the curve arithmetic for *verification* is not here: this module
//! reconstructs the exact bytes that were signed and hands them to a
//! [`MudaqqiqTawqee`] the caller supplies. That is not a weakening. It means
//! the crate every adapter links carries no key material and no primitive that
//! could produce a signature, while still owning the one thing that must not be
//! reimplemented four times — the definition of *what* is signed.
//!
//! ## The block, byte for byte
//!
//! ```text
//! KutlatTawqee — 128 bytes, little-endian, 8-byte aligned
//!
//!   offset  size  field         type       meaning
//!        0     4  sihr          [u8; 4]    the four bytes "TWQ1"
//!        4     2  isdar         u16        block version, 1
//!        6     1  dawr          u8         0 = owner, 1 = contributor self-signed
//!        7     1  khwarizmiya   u8         0 = unsigned reservation, 1 = Ed25519
//!        8     8  waqt          i64        signing time, Unix seconds UTC
//!       16    32  miftah        [u8; 32]   the signer's Ed25519 public key
//!       48    64  tawqee        [u8; 64]   the signature
//!      112    16  mahjuz        [u8; 16]   reserved, written as zero
//! ```
//!
//! The size is fixed at one hundred and twenty-eight bytes and never varies,
//! which is what makes sealing an in-place operation: the compiler writes the
//! block as a reservation with `khwarizmiya` zero, and `taarib-khatm` later
//! overwrites those same bytes. No offset moves, no length changes, and the
//! content hash — which stops at the first byte of this section — does not have
//! to be recomputed.
//!
//! ## What the signature commits to
//!
//! The ROADMAP says the block holds "ed25519 signature over `content_hash`, the
//! signer's public key, the key's role, and the signing timestamp". That reads
//! two ways: a signature over the hash alone, with the other three merely
//! stored beside it, or a signature over all four. Only the second is safe, so
//! only the second is implemented.
//!
//! If the signature covered the hash alone, `dawr` would be an unauthenticated
//! byte sitting next to it. An attacker could take a genuine, correctly signed,
//! contributor-self-signed patch, flip that one byte from 1 to 0, and hand a
//! client a patch that verifies perfectly and claims the owner approved it —
//! which is precisely the promise Decision 8 exists to keep. The timestamp
//! would be equally free to move, defeating revocation by date.
//!
//! So the signed message is the whole commitment, in a fixed order, behind a
//! domain separator:
//!
//! ```text
//! risala — 97 bytes, the exact input to Ed25519
//!
//!   offset  size  contents
//!        0    23  b"taarib-ruqaa/tawqee/v1\0"
//!       23    32  the container's content hash
//!       55     1  dawr
//!       56     1  khwarizmiya
//!       57     8  waqt, little-endian
//!       65    32  miftah
//! ```
//!
//! The domain separator is there so that a signature made over a `.ruqaa` can
//! never be replayed as a signature over anything else Taarib signs — a
//! registry shard, a revocation record, a submission object. Every one of those
//! is a fixed-size byte string that a key holder is asked to sign, and without a
//! prefix that names which one it is, a signature over one is a signature over
//! any other of the same length.
//!
//! Including `miftah` inside the message as well as beside it is deliberate
//! belt-and-braces: it removes any question of key substitution in a future
//! multi-key setting, and it costs thirty-two bytes of hashing once per patch.

use crate::khata::KhataRuqaa;
use crate::tarwisa::{iqra_masfufa, iqra_u16, sittasi, uktub_masfufa, uktub_u16};

/// How many bytes the signature block occupies. Fixed forever.
pub const HAJM_KUTLA: usize = 128;

/// The four bytes the block begins with.
pub const SIHR_KUTLA: [u8; 4] = *b"TWQ1";

/// The block version this build reads and writes.
pub const ISDAR_KUTLA: u16 = 1;

/// Byte offset of `sihr` within the block.
pub const IZAHAT_SIHR: usize = 0;
/// Byte offset of `isdar` within the block.
pub const IZAHAT_ISDAR: usize = 4;
/// Byte offset of `dawr` within the block.
pub const IZAHAT_DAWR: usize = 6;
/// Byte offset of `khwarizmiya` within the block.
pub const IZAHAT_KHWARIZMIYA: usize = 7;
/// Byte offset of `waqt` within the block.
pub const IZAHAT_WAQT: usize = 8;
/// Byte offset of `miftah` within the block.
pub const IZAHAT_MIFTAH: usize = 16;
/// Byte offset of `tawqee` within the block.
pub const IZAHAT_TAWQEE: usize = 48;
/// Byte offset of `mahjuz` within the block.
pub const IZAHAT_MAHJUZ: usize = 112;

/// The domain separator every `.ruqaa` signature is made under.
///
/// NUL-terminated so that no other Taarib domain string can be a prefix of it.
pub const NITAQ_TAWQEE: &[u8; 23] = b"taarib-ruqaa/tawqee/v1\0";

/// How many bytes the signed message is.
pub const HAJM_RISALA: usize = 97;

/// Byte offset of the content hash within the signed message.
pub const IZAHAT_RISALA_BASMA: usize = 23;
/// Byte offset of the role byte within the signed message.
pub const IZAHAT_RISALA_DAWR: usize = 55;
/// Byte offset of the algorithm byte within the signed message.
pub const IZAHAT_RISALA_KHWARIZMIYA: usize = 56;
/// Byte offset of the timestamp within the signed message.
pub const IZAHAT_RISALA_WAQT: usize = 57;
/// Byte offset of the public key within the signed message.
pub const IZAHAT_RISALA_MIFTAH: usize = 65;

/// Who sealed a patch.
///
/// The distinction is the whole of Decision 8. A client installs a patch only
/// when this says the owner sealed it; a contributor's self-signature proves
/// only that the submission arrived intact from the person who made it, which is
/// what the review queue needs and is not permission to install anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DawrMiftah {
    /// The project owner's key. The only role a client will install.
    Malik = 0,
    /// A contributor signing their own submission on the way into review.
    Musahim = 1,
}

impl DawrMiftah {
    /// The byte that crosses into the container.
    #[must_use]
    pub const fn bayt(self) -> u8 {
        self as u8
    }

    /// Reads the byte back.
    ///
    /// Unrecognised values are **rejected**, not defaulted. Everywhere else in
    /// this product an unknown discriminant falls back to something sensible so
    /// that a newer patch degrades rather than fails — here the sensible value
    /// would have to be one of two, and guessing "owner" would let a byte nobody
    /// wrote decide that a patch is trusted.
    #[must_use]
    pub const fn min_bayt(bayt: u8) -> Option<Self> {
        match bayt {
            0 => Some(Self::Malik),
            1 => Some(Self::Musahim),
            _ => None,
        }
    }

    /// Whether a client may install a patch sealed under this role.
    #[must_use]
    pub const fn yusmah_bil_tathbeet(self) -> bool {
        matches!(self, Self::Malik)
    }
}

/// Which signature scheme sealed a patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Khwarizmiya {
    /// The block is a reservation the compiler wrote and nothing has sealed.
    Ghayr = 0,
    /// Ed25519 over the ninety-seven byte message.
    Ed25519 = 1,
}

impl Khwarizmiya {
    /// The byte that crosses into the container.
    #[must_use]
    pub const fn bayt(self) -> u8 {
        self as u8
    }

    /// Reads the byte back, rejecting anything unrecognised for the same reason
    /// [`DawrMiftah::min_bayt`] does.
    #[must_use]
    pub const fn min_bayt(bayt: u8) -> Option<Self> {
        match bayt {
            0 => Some(Self::Ghayr),
            1 => Some(Self::Ed25519),
            _ => None,
        }
    }
}

/// A parsed signature block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KutlatTawqee {
    /// Who sealed it.
    pub dawr: DawrMiftah,
    /// Which scheme sealed it.
    pub khwarizmiya: Khwarizmiya,
    /// When, in Unix seconds UTC.
    pub waqt: i64,
    /// The signer's public key.
    pub miftah: [u8; 32],
    /// The signature itself.
    pub tawqee: [u8; 64],
}

impl KutlatTawqee {
    /// An unsigned reservation, which is what the compiler writes.
    ///
    /// Every field is zero, and `khwarizmiya` being [`Khwarizmiya::Ghayr`] is
    /// what makes [`KutlatTawqee::tahaqquq`] refuse it. The block is written at
    /// full size so that sealing it later is an in-place overwrite: no offset
    /// moves and the content hash, which stops before this section, does not
    /// have to be recomputed.
    #[must_use]
    pub const fn hajz(dawr: DawrMiftah) -> Self {
        Self {
            dawr,
            khwarizmiya: Khwarizmiya::Ghayr,
            waqt: 0,
            miftah: [0; 32],
            tawqee: [0; 64],
        }
    }

    /// Whether anything has actually sealed this block.
    #[must_use]
    pub const fn muwaqqaa(&self) -> bool {
        matches!(self.khwarizmiya, Khwarizmiya::Ed25519)
    }

    /// Reads a block from the bytes of the signature section.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::KutlatTawqeeTalifa`] naming the field that is wrong: the
    /// section too short, the magic absent, the version unknown, or a role or
    /// algorithm byte that is not one of the values the format defines.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataRuqaa> {
        if bayt.len() < HAJM_KUTLA {
            return Err(KhataRuqaa::KutlatTawqeeTalifa {
                haql: "the block is short",
            });
        }
        let sihr: [u8; 4] = iqra_masfufa(bayt, IZAHAT_SIHR)
            .ok_or(KhataRuqaa::KutlatTawqeeTalifa { haql: "sihr" })?;
        if sihr != SIHR_KUTLA {
            return Err(KhataRuqaa::KutlatTawqeeTalifa { haql: "sihr" });
        }
        let isdar =
            iqra_u16(bayt, IZAHAT_ISDAR).ok_or(KhataRuqaa::KutlatTawqeeTalifa { haql: "isdar" })?;
        if isdar != ISDAR_KUTLA {
            return Err(KhataRuqaa::KutlatTawqeeTalifa { haql: "isdar" });
        }

        let dawr = bayt
            .get(IZAHAT_DAWR)
            .copied()
            .and_then(DawrMiftah::min_bayt)
            .ok_or(KhataRuqaa::KutlatTawqeeTalifa { haql: "dawr" })?;
        let khwarizmiya = bayt
            .get(IZAHAT_KHWARIZMIYA)
            .copied()
            .and_then(Khwarizmiya::min_bayt)
            .ok_or(KhataRuqaa::KutlatTawqeeTalifa {
                haql: "khwarizmiya",
            })?;

        let waqt_khana: [u8; 8] = iqra_masfufa(bayt, IZAHAT_WAQT)
            .ok_or(KhataRuqaa::KutlatTawqeeTalifa { haql: "waqt" })?;
        let miftah: [u8; 32] = iqra_masfufa(bayt, IZAHAT_MIFTAH)
            .ok_or(KhataRuqaa::KutlatTawqeeTalifa { haql: "miftah" })?;
        let tawqee: [u8; 64] = iqra_masfufa(bayt, IZAHAT_TAWQEE)
            .ok_or(KhataRuqaa::KutlatTawqeeTalifa { haql: "tawqee" })?;

        Ok(Self {
            dawr,
            khwarizmiya,
            waqt: i64::from_le_bytes(waqt_khana),
            miftah,
            tawqee,
        })
    }

    /// Writes the block into a buffer of exactly [`HAJM_KUTLA`] bytes.
    ///
    /// The reserved tail is written as zero rather than left alone: a block
    /// assembled over a reused buffer would otherwise carry whatever was there
    /// before into a file that is about to be hashed and shipped.
    pub fn ila_bayt(&self, hadaf: &mut [u8]) {
        if hadaf.len() < HAJM_KUTLA {
            return;
        }
        if let Some(khana) = hadaf.get_mut(..HAJM_KUTLA) {
            khana.fill(0);
        }
        uktub_masfufa(hadaf, IZAHAT_SIHR, &SIHR_KUTLA);
        uktub_u16(hadaf, IZAHAT_ISDAR, ISDAR_KUTLA);
        if let Some(khana) = hadaf.get_mut(IZAHAT_DAWR) {
            *khana = self.dawr.bayt();
        }
        if let Some(khana) = hadaf.get_mut(IZAHAT_KHWARIZMIYA) {
            *khana = self.khwarizmiya.bayt();
        }
        uktub_masfufa(hadaf, IZAHAT_WAQT, &self.waqt.to_le_bytes());
        uktub_masfufa(hadaf, IZAHAT_MIFTAH, &self.miftah);
        uktub_masfufa(hadaf, IZAHAT_TAWQEE, &self.tawqee);
    }

    /// Rebuilds the exact bytes that were signed.
    ///
    /// This function is the reason the module exists. Four languages verify
    /// `.ruqaa` signatures and every one of them has to construct byte-identical
    /// input; a second implementation that ordered one field differently would
    /// reject every genuine patch, and — far worse — a second implementation
    /// that omitted `dawr` would accept a forged one.
    #[must_use]
    pub fn risala(&self, basma: &[u8; 32]) -> [u8; HAJM_RISALA] {
        let mut risala = [0u8; HAJM_RISALA];
        uktub_masfufa(&mut risala, 0, NITAQ_TAWQEE);
        uktub_masfufa(&mut risala, IZAHAT_RISALA_BASMA, basma);
        if let Some(khana) = risala.get_mut(IZAHAT_RISALA_DAWR) {
            *khana = self.dawr.bayt();
        }
        if let Some(khana) = risala.get_mut(IZAHAT_RISALA_KHWARIZMIYA) {
            *khana = self.khwarizmiya.bayt();
        }
        uktub_masfufa(&mut risala, IZAHAT_RISALA_WAQT, &self.waqt.to_le_bytes());
        uktub_masfufa(&mut risala, IZAHAT_RISALA_MIFTAH, &self.miftah);
        risala
    }

    /// Checks the seal against a verifier the caller supplies.
    ///
    /// The two refusals before the curve arithmetic are the ones that matter in
    /// practice. An unsigned block is refused as [`KhataRuqaa::GhayrMuwaqqaa`]
    /// rather than as a failed signature, because the two mean different things
    /// to whoever reads the message: one is a patch that was never sealed, the
    /// other is a patch that was sealed and then altered.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::GhayrMuwaqqaa`] when nothing has sealed the block, and
    /// [`KhataRuqaa::TawqeeGhayrSalih`] naming the key when the signature does
    /// not verify under it.
    pub fn tahaqquq(
        &self,
        basma: &[u8; 32],
        mudaqqiq: &dyn MudaqqiqTawqee,
    ) -> Result<(), KhataRuqaa> {
        if !self.muwaqqaa() {
            return Err(KhataRuqaa::GhayrMuwaqqaa);
        }
        let risala = self.risala(basma);
        if mudaqqiq.tahaqquq(&self.miftah, &risala, &self.tawqee) {
            Ok(())
        } else {
            Err(KhataRuqaa::TawqeeGhayrSalih {
                miftah: sittasi(&self.miftah),
            })
        }
    }

    /// The signing time, as a count of bytes for a caller that wants to render
    /// it — the crate has no clock and no date formatter, deliberately.
    #[must_use]
    pub const fn waqt_unix(&self) -> i64 {
        self.waqt
    }

    /// The signer's key as lowercase hexadecimal, which is how a key is named in
    /// every message a user sees.
    #[must_use]
    pub fn miftah_nassi(&self) -> String {
        sittasi(&self.miftah)
    }
}

/// Somebody who can check an Ed25519 signature.
///
/// A trait rather than a function so that the crate every adapter links carries
/// no curve implementation at all. `taarib-khatm` supplies the real one; the
/// review console can supply one that logs; a caller with no verifier at all
/// simply cannot call [`KutlatTawqee::tahaqquq`], which is the correct outcome
/// for a build that has no business trusting patches.
pub trait MudaqqiqTawqee {
    /// Whether `tawqee` is a valid Ed25519 signature over `risala` under
    /// `miftah`.
    ///
    /// Implementations must not panic on a malformed key or signature — both
    /// arrive from a file a stranger produced — and must return `false` rather
    /// than reporting why, because a caller that could distinguish "bad key
    /// encoding" from "bad signature" learns nothing it can act on and an
    /// attacker learns one bit.
    fn tahaqquq(&self, miftah: &[u8; 32], risala: &[u8], tawqee: &[u8; 64]) -> bool;
}

/// A verifier that refuses everything.
///
/// Not a stub: it is what a build with no cryptography links, and it is the
/// right answer there. A caller that would otherwise have silently skipped
/// verification instead fails closed, and the failure names the missing
/// capability rather than looking like a corrupt patch.
#[derive(Debug, Clone, Copy, Default)]
pub struct MudaqqiqRafid;

impl MudaqqiqTawqee for MudaqqiqRafid {
    fn tahaqquq(&self, _miftah: &[u8; 32], _risala: &[u8], _tawqee: &[u8; 64]) -> bool {
        false
    }
}

/// The size of the signature section, which is fixed and known before anything
/// is written.
///
/// Exposed so the writer can reserve the section without constructing a block,
/// and so the reader can refuse a signature section of any other length before
/// it parses a single field.
#[must_use]
pub const fn hajm_qism() -> u64 {
    // Written out rather than converted from `HAJM_KUTLA`: `u64::try_from` is
    // not usable in a const context, and a cast here would be a cast the lint
    // configuration rightly refuses. One hundred and twenty-eight is the size in
    // both places and the constants sit six lines apart.
    128
}
