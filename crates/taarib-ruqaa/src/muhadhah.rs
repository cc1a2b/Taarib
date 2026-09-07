//! المحاذاة — owned bytes the fixed-layout tables can actually be read out of.
//!
//! This module exists because of one fact that is easy to miss until it breaks
//! everything: **`Vec<u8>` is one-byte aligned**. Its allocation carries no
//! stronger guarantee, because nothing about `u8` requires one.
//!
//! Every table in [`crate::jadawil`] is read by casting a byte range to an array
//! of structs, and those structs need four- or eight-byte alignment. Casting a
//! slice that does not satisfy it is not a slow path or a subtle inaccuracy —
//! `bytemuck` refuses it outright, which is the whole reason to use `bytemuck`.
//! So a section decompressed into a plain `Vec<u8>` would be a section whose
//! tables cannot be read at all, on a machine where the allocator happened to
//! return an odd address, and would appear to work everywhere the allocator
//! happened not to. That is the worst shape a bug can have.
//!
//! Three ways of holding a patch's bytes come up in this product:
//!
//! * a memory map, which is page-aligned and therefore fine;
//! * a decompressed section, which is where the problem is;
//! * a whole container read into memory by the compiler or the review console,
//!   which has exactly the same problem.
//!
//! [`BaytMuhadhah`] answers the last two. It is a byte buffer backed by an array
//! of sixteen-byte cells, so its first byte sits on a sixteen-byte boundary —
//! the same boundary the container's own sections are aligned to, which means a
//! table at a valid offset inside a valid section is castable by construction
//! rather than by luck.
//!
//! Sixteen and not eight: [`crate::jadawil::MUHADHAT_JADWAL`] is the guarantee
//! the format makes about section starts, and a buffer that promised less than
//! the format does would be a second number to keep in step with the first.

use bytemuck::{Pod, Zeroable};

use crate::jadawil::MUHADHAT_JADWAL;
use crate::khata::KhataRuqaa;

/// One aligned cell. Never seen outside this module.
///
/// The alignment attribute is doing all the work; the array inside is only
/// there to give the cell a size.
///
/// The two `unsafe impl`s below are written out rather than derived because
/// `bytemuck`'s derive is documented against plain `#[repr(C)]` types, and an
/// alignment modifier is exactly the kind of thing a derive is entitled to
/// refuse. The obligations are trivially met here and are stated so that a
/// future reader does not have to re-derive them.
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
struct Kutla([u8; MUHADHAT_JADWAL]);

// SAFETY: `Kutla` is a newtype over `[u8; 16]`. Every bit pattern of sixteen
// bytes is a valid value of it, so an all-zero one is valid.
unsafe impl Zeroable for Kutla {}

// SAFETY: `Kutla` is `#[repr(C)]` over a single `[u8; 16]` field, so its size is
// exactly sixteen and it contains no padding — the assertions below hold the
// compiler to both. Its only field is `Pod`, every bit pattern is valid, and it
// has no interior mutability, no references and no `Drop`. Raising the alignment
// to sixteen cannot introduce padding in a type whose size already equals its
// alignment, and a stricter alignment is never unsound for a `Pod` cast: it can
// only make a cast that needs less alignment succeed where it would otherwise
// have been refused, which is the entire purpose of this type.
unsafe impl Pod for Kutla {}

const _: () = assert!(size_of::<Kutla>() == MUHADHAT_JADWAL);
const _: () = assert!(align_of::<Kutla>() == MUHADHAT_JADWAL);

/// A byte buffer whose first byte is on a sixteen-byte boundary.
///
/// Behaves like a `Vec<u8>` for every purpose this crate has: it derefs to
/// `[u8]`, it can be handed to a hash function, written to a file, or passed to
/// [`crate::qari::Ruqaa::iftah`]. What it adds is the alignment the table casts
/// need, and it keeps its own length so that a buffer of, say, ninety-nine bytes
/// is ninety-nine bytes rather than being rounded up to a hundred and twelve.
#[derive(Clone)]
pub struct BaytMuhadhah {
    kutal: Vec<Kutla>,
    tul: usize,
}

impl std::fmt::Debug for BaytMuhadhah {
    /// Prints the length, never the contents.
    ///
    /// A patch is megabytes of atlas and script, and a `Debug` that dumped it
    /// would turn one stray `tracing` call into a log line nobody can page past
    /// — inside a game, on a player's machine, in a diagnostics bundle they are
    /// about to send somewhere.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaytMuhadhah")
            .field("tul", &self.tul)
            .finish_non_exhaustive()
    }
}

impl BaytMuhadhah {
    /// A zeroed buffer of exactly `tul` bytes.
    #[must_use]
    pub fn sifr(tul: usize) -> Self {
        let khalaya = tul.div_ceil(MUHADHAT_JADWAL);
        Self {
            kutal: vec![Kutla([0; MUHADHAT_JADWAL]); khalaya],
            tul,
        }
    }

    /// A copy of a byte slice, on an aligned buffer.
    ///
    /// The copy is the point. A caller with bytes from anywhere — a download, a
    /// `std::fs::read`, an archive entry — uses this to obtain bytes the tables
    /// can be cast out of.
    #[must_use]
    pub fn min_bayt(bayt: &[u8]) -> Self {
        let mut muhadhah = Self::sifr(bayt.len());
        muhadhah.bayt_mut().copy_from_slice(bayt);
        muhadhah
    }

    /// Reads a whole file into an aligned buffer.
    ///
    /// For the compiler, the review console and the registry's verifier — every
    /// caller that wants the bytes in memory rather than mapped. A game adapter
    /// uses [`crate::qari::MalafRuqaa`] instead and never pays for the copy.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::KhataMalaf`] naming the path.
    pub fn iqra(masar: &std::path::Path) -> Result<Self, KhataRuqaa> {
        let khaam = std::fs::read(masar).map_err(|sabab| KhataRuqaa::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })?;
        Ok(Self::min_bayt(&khaam))
    }

    /// Writes the buffer to a file.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::KhataMalaf`] naming the path.
    pub fn uktub(&self, masar: &std::path::Path) -> Result<(), KhataRuqaa> {
        std::fs::write(masar, self.bayt()).map_err(|sabab| KhataRuqaa::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })
    }

    /// The bytes.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        // Casting to `u8` cannot fail: the target's alignment is one and its
        // size divides the cell's exactly. The length is trimmed because the
        // backing store is rounded up to whole cells.
        bytemuck::cast_slice::<Kutla, u8>(&self.kutal)
            .get(..self.tul)
            .unwrap_or(&[])
    }

    /// The bytes, mutably.
    #[must_use]
    pub fn bayt_mut(&mut self) -> &mut [u8] {
        let tul = self.tul;
        bytemuck::cast_slice_mut::<Kutla, u8>(&mut self.kutal)
            .get_mut(..tul)
            .unwrap_or(&mut [])
    }

    /// How many bytes the buffer holds.
    #[must_use]
    pub const fn tul(&self) -> usize {
        self.tul
    }

    /// Whether it holds none.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.tul == 0
    }

    /// Copies the bytes into an ordinary vector.
    ///
    /// For a caller crossing an interface that insists on `Vec<u8>` — a HTTP
    /// body, a database blob. Anything that is going to read tables out of the
    /// bytes should keep the aligned buffer instead.
    #[must_use]
    pub fn ila_shuaa(&self) -> Vec<u8> {
        self.bayt().to_vec()
    }
}

impl std::ops::Deref for BaytMuhadhah {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.bayt()
    }
}

impl AsRef<[u8]> for BaytMuhadhah {
    fn as_ref(&self) -> &[u8] {
        self.bayt()
    }
}

impl From<&[u8]> for BaytMuhadhah {
    fn from(bayt: &[u8]) -> Self {
        Self::min_bayt(bayt)
    }
}
