//! الموارد — Unreal's localization resources, and the round-trip they must survive.
//!
//! A game's translated text does not live in one file. It lives in a **resource
//! set**: a `.locmeta` naming the native culture and listing the cultures that
//! were compiled, one `.locres` per culture holding every translated string
//! under a namespace and a key, and a scattering of `StringTable` assets holding
//! source strings that the engine resolves through the same namespace and key.
//! Any one of those read on its own is a fragment. Read together they answer the
//! only three questions this adapter has: which cultures exist, which strings
//! exist, and what the game currently shows for each.
//!
//! ```text
//! <Project>/Content/Localization/
//!   Game/
//!     Game.locmeta                 native culture, native locres, compiled cultures
//!     en/Game.locres               one compiled resource per culture
//!     fr/Game.locres
//!     ar/Game.locres               what Taarib adds
//!   <plugin>/...                   the same shape again, per localization target
//! <Project>/Content/**/*.uasset    StringTable assets: source strings, in-place
//! ```
//!
//! ## The round-trip contract
//!
//! Every reader in this module is one half of a pair, and the pair has one
//! property that everything else depends on:
//!
//! > Reading a file and writing it straight back, with no edit in between,
//! > produces **byte-identical** output.
//!
//! That is not a nicety. Reinjection edits a handful of strings in a file the
//! game shipped and puts the result back in front of the engine. If writing
//! regenerated anything — a hash, an offset, a string's encoding, the order of a
//! table, a byte of padding — then the difference between "Taarib changed the
//! strings it was asked to change" and "Taarib rewrote the file and one of the
//! things it rewrote was wrong" would be undetectable. With the contract, the
//! test is trivial and mechanical: read, write, compare. A single differing byte
//! is a bug in this module, found on a build machine rather than in somebody's
//! game.
//!
//! Three consequences follow, and each one shapes a type here:
//!
//! * **Stored hashes are carried, never recomputed on write.** A `.locres`
//!   entry's namespace and key hashes are read from the file and written back
//!   unchanged. Recomputing them would make the round-trip depend on this
//!   build's hash agreeing with the hash the engine's own compiler used, and a
//!   file whose hashes disagree with Unreal's loads without error and then
//!   resolves nothing — the worst failure this module can produce, because it
//!   looks like success. Hashes are computed only for entries Taarib *adds*, by
//!   [`locres`]'s explicit implementations of the engine's own functions.
//! * **A string's encoding is part of its value.** See below.
//! * **Regions this build does not interpret are preserved verbatim.** Where a
//!   format has a gap between two structures, or a tail after the last one, the
//!   bytes are kept and written back exactly. Regenerating a region whose
//!   meaning is unknown is guessing, and the guess would be invisible until an
//!   engine version that reads it refused the file.
//!
//! ## `FString`, and the sign convention that breaks third-party readers
//!
//! Every string in every format here is an Unreal `FString`, serialized as a
//! signed 32-bit length followed by that many *units*:
//!
//! ```text
//! FString — variable, little-endian
//!
//!   offset  size  field   type   meaning
//!        0     4  tul     i32    POSITIVE: that many ANSI bytes follow, NUL included
//!                                NEGATIVE: -tul UTF-16LE code units follow, NUL included
//!                                ZERO:     the empty string, and nothing follows
//!        4   ...  wahdat  ...    the units, the last of which is the terminator
//! ```
//!
//! Reading `tul` as unsigned is the single most common bug in third-party
//! `.locres` readers, and it is worth being precise about why it is so hard to
//! notice. English source text is pure ASCII, so Unreal writes it ANSI, so `tul`
//! is positive, so a reader that treats the field as `u32` works perfectly on
//! every English file anyone tests it against. The first time it sees a
//! translated file — the first Japanese, Russian or Arabic string, which is the
//! only kind of string this product ever touches — `tul` is negative, the
//! unsigned reading is about four billion, and the reader either allocates until
//! the machine gives up or walks off the end of the buffer. This module reads it
//! as `i32`, branches on the sign, and bounds both branches against the real
//! file length before a byte is reserved.
//!
//! The terminator is included in the count and is **required**: a declared
//! length of five ANSI bytes is a four-character string, and a fifth byte that
//! is not zero is a file this module refuses rather than silently trims. The
//! empty string is the one case with no terminator at all, because Unreal writes
//! a length of zero and nothing else — which is why [`TarmizNass`] has three
//! variants and not two.
//!
//! ## Why the encoding is part of the value
//!
//! [`NassMukhazzan`] carries the text *and* the form it was stored in. Unreal
//! writes a string ANSI when it is pure ASCII and UTF-16 otherwise, so a reader
//! that decoded to `String` and re-encoded by that rule would round-trip almost
//! everything — and would silently rewrite the exceptions: an empty string
//! stored as a lone NUL, an ANSI block holding a byte above 0x7F, a string a
//! different tool wrote UTF-16 for no reason. Each of those is a byte-level
//! difference in a file the engine is about to load, produced by a reader that
//! believed it had changed nothing.
//!
//! The ANSI form is decoded as Latin-1, one byte to one code point, because that
//! is what the engine does: `FString`'s ANSI constructor widens each byte, so
//! byte 0xE9 becomes U+00E9. Decoding it as UTF-8 would be a guess that differs
//! from the engine's reading and does not survive re-encoding.
//!
//! Editing a string through [`NassMukhazzan::ghayyir`] re-picks the encoding by
//! the engine's own rule, which is exactly when the round-trip is *supposed* to
//! break: an Arabic translation cannot be stored ANSI, and the file must change.
//!
//! ## Every count is judged against the file before it sizes anything
//!
//! These files arrive with somebody's game, and the compiler also reads files a
//! translator or a mod tool produced. Every namespace count, key count, string
//! count and length in them is a number that decides how much memory this
//! process is about to take, so each one passes [`tahaqquq_adad`] first: it is
//! refused above a named ceiling with [`KhataUnreal::HajmMufrit`], and refused
//! again with [`KhataUnreal::MawridTalif`] when the bytes it implies cannot fit
//! in what is left of the file. Only then does it reach a `with_capacity`.
//!
//! ## Slices, not files
//!
//! Everything works over `&[u8]`, so the caller decides whether the bytes came
//! from a memory map, from a `.pak` entry decompressed in memory, or from an
//! IoStore chunk. The formats here are found inside containers at least as often
//! as they are found loose, and a reader that could only open a path would be
//! unusable for the packaged case — which is most games. [`iqra_malaf`] and
//! [`uktub_malaf`] are the only two functions in this module that touch the
//! filesystem, and each format's `min_malaf` is a thin convenience over the
//! first of them.

pub mod iostore;
pub mod jadwal;
pub mod locmeta;
pub mod locres;
pub mod pak;

use std::path::Path;

use crate::khata::{KhataUnreal, hajm_usize, tul_u64};

/// The largest file [`iqra_malaf`] will read off disk.
///
/// Half a gibibyte. A `.locres` for a large role-playing game with forty
/// thousand strings is single-digit megabytes; a `StringTable` `.uasset` is
/// kilobytes. The ceiling exists because the convenience readers take a path
/// from a user's configuration, and a path is a place a mistake can point.
pub const AQSA_MALAF: u64 = 512 * 1024 * 1024;

/// The largest number of ANSI bytes, or UTF-16 code units, one string may
/// declare.
///
/// Sixty-four mebibytes, which is four orders of magnitude above the longest
/// string any game ships and still small enough that a corrupt length cannot
/// exhaust a game process's address space. Checked against the declared length
/// before the bytes are taken.
pub const AQSA_TUL_NASS: u64 = 64 * 1024 * 1024;

/// The most a writer reserves up front, however large its estimate was.
///
/// Sixty-four mebibytes. Past that the buffer grows as it is written, which
/// costs a few reallocations on a file no game ships and is the right trade
/// against reserving hundreds of megabytes on an estimate.
const AQSA_HAJZ: usize = 64 * 1024 * 1024;

/// A localization resource that reads and writes as one whole file.
///
/// The trait exists to state the round-trip contract in code rather than only in
/// prose: anything implementing it promises that `min_bayt` followed by
/// `ila_bayt`, with no edit between them, returns the input bytes exactly. The
/// three implementors are [`locres::MawridLocres`], [`locmeta::MawridLocmeta`]
/// and [`jadwal::JadwalNusus`].
pub trait Mawrid: Sized {
    /// The format's name, as it appears in every refusal this module raises.
    const ISM: &'static str;

    /// Reads and validates the whole resource.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] when a field runs past the end,
    /// [`KhataUnreal::SihrGhayrMutabaq`] when the magic is not this format's,
    /// [`KhataUnreal::IsdarGhayrMadum`] for a version above what this build
    /// reads, [`KhataUnreal::HajmMufrit`] for a count above its ceiling,
    /// [`KhataUnreal::MawridTalif`] for a field inconsistent with the file, and
    /// [`KhataUnreal::NassGhayrSalih`] for a string that is not valid UTF-16.
    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataUnreal>;

    /// Writes the resource back out.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::HajmMufrit`] when a string is longer than the format's
    /// length field can express, and [`KhataUnreal::MawridTalif`] when a value
    /// held in memory cannot be expressed in the version this resource declares.
    fn ila_bayt(&self) -> Result<Vec<u8>, KhataUnreal>;
}

/// The form a string was stored in, which is part of its value here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TarmizNass {
    /// Serialized as a length of zero and nothing else. Unreal's own spelling of
    /// the empty string, and distinct from an empty string stored as a lone NUL.
    Khali,
    /// Serialized as a positive length and that many bytes, terminator included.
    /// Decoded as Latin-1, because that is how the engine widens it.
    Ansi,
    /// Serialized as a negative length and that many UTF-16LE code units,
    /// terminator included.
    Utf16,
}

/// A string exactly as the file holds it: the text, and the encoding it had.
///
/// Both halves are needed. The text is what a translator edits; the encoding is
/// what makes writing the file back byte-identical when nothing was edited. See
/// this module's header for why re-deriving the encoding from the text is not
/// good enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NassMukhazzan {
    nass: String,
    tarmiz: TarmizNass,
}

impl NassMukhazzan {
    /// A new string, encoded the way Unreal would encode it.
    ///
    /// Empty becomes [`TarmizNass::Khali`], pure ASCII becomes
    /// [`TarmizNass::Ansi`], and anything else becomes [`TarmizNass::Utf16`] —
    /// which is the rule `FString`'s own serializer applies, so a string this
    /// module adds is indistinguishable from one the engine wrote.
    #[must_use]
    pub fn jadeed(nass: &str) -> Self {
        let tarmiz = if nass.is_empty() {
            TarmizNass::Khali
        } else if nass.is_ascii() {
            TarmizNass::Ansi
        } else {
            TarmizNass::Utf16
        };
        Self {
            nass: nass.to_owned(),
            tarmiz,
        }
    }

    /// The text.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.nass
    }

    /// The form it was stored in.
    #[must_use]
    pub const fn tarmiz(&self) -> TarmizNass {
        self.tarmiz
    }

    /// Whether the text is empty, however it was stored.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.nass.is_empty()
    }

    /// Replaces the text, re-picking the encoding by the engine's rule.
    ///
    /// The one call in this module that deliberately breaks the byte-identical
    /// round-trip, because that is what an edit is: an Arabic translation cannot
    /// be stored ANSI, so the file must change, and the change must be the one
    /// Unreal itself would have written.
    pub fn ghayyir(&mut self, nass: &str) {
        *self = Self::jadeed(nass);
    }

    /// How many bytes this string occupies when serialized, terminator and
    /// length field included, or [`None`] when it cannot be expressed.
    ///
    /// What the `StringTable` writer uses to decide whether an edited payload
    /// still fits the space the `.uasset` reserved for it.
    #[must_use]
    pub fn tul_musalsal(&self) -> Option<u64> {
        let jism = match self.tarmiz {
            TarmizNass::Khali => 0u64,
            TarmizNass::Ansi => tul_u64(self.nass.chars().count()).checked_add(1)?,
            TarmizNass::Utf16 => tul_u64(self.nass.encode_utf16().count())
                .checked_add(1)?
                .checked_mul(2)?,
        };
        jism.checked_add(4)
    }

    /// The length field this string serializes with, or [`None`] when the string
    /// is longer than an `i32` can express.
    fn tul_muallan(&self) -> Option<i32> {
        match self.tarmiz {
            TarmizNass::Khali => Some(0),
            TarmizNass::Ansi => {
                let adad = self.nass.chars().count().checked_add(1)?;
                i32::try_from(adad).ok()
            },
            TarmizNass::Utf16 => {
                let adad = self.nass.encode_utf16().count().checked_add(1)?;
                i32::try_from(adad).ok().and_then(i32::checked_neg)
            },
        }
    }
}

/// A bounded cursor over one resource's bytes.
///
/// Every read names the field it is reading, so a truncated file produces a
/// refusal that says which field ran out rather than a generic short-read. The
/// cursor also counts strings, so [`KhataUnreal::NassGhayrSalih`] can say which
/// string in the file was not valid UTF-16 — the twelve thousandth string in a
/// `.locres` is a thing a person can go and look at.
#[derive(Debug)]
pub struct Qari<'a> {
    ism: &'static str,
    bayt: &'a [u8],
    mawqi: usize,
    fahras: u32,
}

impl<'a> Qari<'a> {
    /// A cursor at the start of `bayt`, refusing in the name of format `ism`.
    #[must_use]
    pub const fn jadeed(ism: &'static str, bayt: &'a [u8]) -> Self {
        Self {
            ism,
            bayt,
            mawqi: 0,
            fahras: 0,
        }
    }

    /// The format this cursor refuses in the name of.
    #[must_use]
    pub const fn ism(&self) -> &'static str {
        self.ism
    }

    /// The whole buffer, from byte zero.
    #[must_use]
    pub const fn kull(&self) -> &'a [u8] {
        self.bayt
    }

    /// How far in the cursor is.
    #[must_use]
    pub const fn mawqi(&self) -> usize {
        self.mawqi
    }

    /// How many bytes are left.
    #[must_use]
    pub fn baqi(&self) -> u64 {
        tul_u64(self.bayt.len().saturating_sub(self.mawqi))
    }

    /// Everything from the cursor to the end, without consuming it.
    ///
    /// What a reader preserves verbatim when a format has a tail this build does
    /// not interpret.
    #[must_use]
    pub fn baqiya(&self) -> &'a [u8] {
        let bayt = self.bayt;
        bayt.get(self.mawqi..).unwrap_or(&[])
    }

    /// Moves the cursor to an absolute offset, refusing one outside the file.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] naming `haql`, because an offset a file
    /// declares that points outside that file is a corrupt field and not a short
    /// read — the distinction matters to whoever reads the message.
    pub fn iqfiz(&mut self, haql: &'static str, izaha: u64) -> Result<(), KhataUnreal> {
        let tul = self.bayt.len();
        let mawqi = hajm_usize(izaha)
            .filter(|mawqi| *mawqi <= tul)
            .ok_or_else(|| KhataUnreal::MawridTalif {
                ism: self.ism,
                haql,
                qeema: izaha,
                hadd: tul_u64(tul),
            })?;
        self.mawqi = mawqi;
        Ok(())
    }

    /// Takes `adad` bytes.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] naming `haql` when they are not there.
    pub fn iqra_bayt(&mut self, haql: &'static str, adad: u64) -> Result<&'a [u8], KhataUnreal> {
        let bayt = self.bayt;
        let matlub = tul_u64(self.mawqi).saturating_add(adad);
        let mada = hajm_usize(adad).ok_or_else(|| qaseer(haql, bayt.len(), matlub))?;
        let nihaya = self
            .mawqi
            .checked_add(mada)
            .ok_or_else(|| qaseer(haql, bayt.len(), matlub))?;
        let khana = bayt
            .get(self.mawqi..nihaya)
            .ok_or_else(|| qaseer(haql, bayt.len(), matlub))?;
        self.mawqi = nihaya;
        Ok(khana)
    }

    /// Takes a fixed-length run.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] naming `haql`.
    pub fn iqra_masfufa<const N: usize>(
        &mut self,
        haql: &'static str,
    ) -> Result<[u8; N], KhataUnreal> {
        let khana = self.iqra_bayt(haql, tul_u64(N))?;
        khana
            .try_into()
            .map_err(|_| qaseer(haql, self.bayt.len(), tul_u64(N)))
    }

    /// Takes one byte.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] naming `haql`.
    pub fn iqra_u8(&mut self, haql: &'static str) -> Result<u8, KhataUnreal> {
        let khana: [u8; 1] = self.iqra_masfufa(haql)?;
        Ok(khana.first().copied().unwrap_or(0))
    }

    /// Takes four little-endian bytes as unsigned.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] naming `haql`.
    pub fn iqra_u32(&mut self, haql: &'static str) -> Result<u32, KhataUnreal> {
        Ok(u32::from_le_bytes(self.iqra_masfufa::<4>(haql)?))
    }

    /// Takes four little-endian bytes as signed.
    ///
    /// Signed because Unreal's counts and its `FString` lengths are `int32`, and
    /// reading them unsigned is the bug this module's header describes.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] naming `haql`.
    pub fn iqra_i32(&mut self, haql: &'static str) -> Result<i32, KhataUnreal> {
        Ok(i32::from_le_bytes(self.iqra_masfufa::<4>(haql)?))
    }

    /// Takes eight little-endian bytes as signed.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] naming `haql`.
    pub fn iqra_i64(&mut self, haql: &'static str) -> Result<i64, KhataUnreal> {
        Ok(i64::from_le_bytes(self.iqra_masfufa::<8>(haql)?))
    }

    /// Takes one `FString`, in either encoding, preserving which one it was.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MalafQaseer`] when the units are not there,
    /// [`KhataUnreal::HajmMufrit`] when the declared length is above
    /// [`AQSA_TUL_NASS`], [`KhataUnreal::MawridTalif`] when the length is
    /// `i32::MIN` — which has no negation — or when the last unit is not the NUL
    /// terminator the format requires, and [`KhataUnreal::NassGhayrSalih`] when
    /// the UTF-16 form holds an unpaired surrogate.
    pub fn iqra_nass(&mut self, haql: &'static str) -> Result<NassMukhazzan, KhataUnreal> {
        let fahras = self.fahras;
        self.fahras = self.fahras.saturating_add(1);
        let tul = self.iqra_i32(haql)?;
        if tul == 0 {
            return Ok(NassMukhazzan {
                nass: String::new(),
                tarmiz: TarmizNass::Khali,
            });
        }
        if tul > 0 {
            let adad = u64::try_from(tul).unwrap_or(0);
            tahaqquq_tul_nass(haql, adad)?;
            let khaam = self.iqra_bayt(haql, adad)?;
            let (akhir, jism) = khaam.split_last().unwrap_or((&0, &[]));
            self.tahaqquq_khatima(haql, u64::from(*akhir))?;
            let nass = jism.iter().map(|wahda| char::from(*wahda)).collect();
            return Ok(NassMukhazzan {
                nass,
                tarmiz: TarmizNass::Ansi,
            });
        }
        let adad = tul
            .checked_neg()
            .and_then(|adad| u64::try_from(adad).ok())
            .ok_or_else(|| KhataUnreal::MawridTalif {
                ism: self.ism,
                haql,
                // i32::MIN is the one length with no negation, so it can never
                // name a number of code units. Refused by value, not clamped.
                qeema: u64::from(i32::MAX.unsigned_abs()).saturating_add(1),
                hadd: AQSA_TUL_NASS,
            })?;
        tahaqquq_tul_nass(haql, adad)?;
        let khaam = self.iqra_bayt(haql, adad.saturating_mul(2))?;
        let wahdat: Vec<u16> = khaam.chunks_exact(2).map(wahda_min_zawj).collect();
        let (akhir, jism) = wahdat.split_last().unwrap_or((&0, &[]));
        self.tahaqquq_khatima(haql, u64::from(*akhir))?;
        let nass = min_utf16(jism, fahras)?;
        Ok(NassMukhazzan {
            nass,
            tarmiz: TarmizNass::Utf16,
        })
    }

    /// Refuses a string whose last unit is not the NUL the format includes in
    /// the count.
    ///
    /// Trimming it instead would mean a file that lies about its own lengths
    /// still reads, and every string after it would be shifted by one unit with
    /// nothing to say so.
    const fn tahaqquq_khatima(&self, haql: &'static str, akhir: u64) -> Result<(), KhataUnreal> {
        if akhir != 0 {
            return Err(KhataUnreal::MawridTalif {
                ism: self.ism,
                haql,
                qeema: akhir,
                hadd: 1,
            });
        }
        Ok(())
    }
}

/// A growing buffer that writes the same shapes the cursor reads.
///
/// Deliberately infallible on the buffer side — a `Vec<u8>` does not run out
/// mid-write — so the only errors it raises are about values that cannot be
/// expressed in the format at all.
#[derive(Debug, Default)]
pub struct Katib {
    bayt: Vec<u8>,
}

impl Katib {
    /// An empty buffer.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { bayt: Vec::new() }
    }

    /// An empty buffer with room already reserved.
    /// The reservation is capped: the size a caller estimates is derived from
    /// data a reader already bounded, but an estimate is still not a reason to
    /// ask the allocator for a gigabyte before the first byte is written.
    #[must_use]
    pub fn bi_siaa(siaa: usize) -> Self {
        Self {
            bayt: Vec::with_capacity(siaa.min(AQSA_HAJZ)),
        }
    }

    /// How many bytes have been written.
    #[must_use]
    pub const fn mawqi(&self) -> usize {
        self.bayt.len()
    }

    /// The bytes written so far.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }

    /// The finished buffer.
    #[must_use]
    pub fn ila_vec(self) -> Vec<u8> {
        self.bayt
    }

    /// Appends a run of bytes verbatim.
    pub fn uktub_bayt(&mut self, khana: &[u8]) {
        self.bayt.extend_from_slice(khana);
    }

    /// Appends one byte.
    pub fn uktub_u8(&mut self, qeema: u8) {
        self.bayt.push(qeema);
    }

    /// Appends four little-endian bytes.
    pub fn uktub_u32(&mut self, qeema: u32) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    /// Appends four little-endian bytes, signed.
    pub fn uktub_i32(&mut self, qeema: i32) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    /// Appends eight little-endian bytes, signed.
    pub fn uktub_i64(&mut self, qeema: i64) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    /// Overwrites eight little-endian bytes already written.
    ///
    /// How a forward reference is resolved: the string-array offset in a
    /// `.locres` header is written as a placeholder and filled in once the
    /// namespace table has been emitted and its real end is known.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] when `izaha` is not eight bytes inside what
    /// has already been written, which would mean the writer lost track of its
    /// own header.
    pub fn uktub_i64_fi(
        &mut self,
        ism: &'static str,
        haql: &'static str,
        izaha: usize,
        qeema: i64,
    ) -> Result<(), KhataUnreal> {
        let hadd = tul_u64(self.bayt.len());
        let nihaya = izaha
            .checked_add(8)
            .ok_or_else(|| talif_izaha(ism, haql, izaha, hadd))?;
        let khana = self
            .bayt
            .get_mut(izaha..nihaya)
            .ok_or_else(|| talif_izaha(ism, haql, izaha, hadd))?;
        khana.copy_from_slice(&qeema.to_le_bytes());
        Ok(())
    }

    /// Appends one `FString` in the encoding it was stored with.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::HajmMufrit`] when the string is longer than the `i32`
    /// length field can express, and [`KhataUnreal::MawridTalif`] when a string
    /// marked ANSI holds a code point above U+00FF — which cannot be produced
    /// through this module's own constructors and would mean the value was
    /// assembled by something that ignored the encoding it claimed.
    pub fn uktub_nass(
        &mut self,
        ism: &'static str,
        haql: &'static str,
        nass: &NassMukhazzan,
    ) -> Result<(), KhataUnreal> {
        let tul = nass.tul_muallan().ok_or_else(|| KhataUnreal::HajmMufrit {
            haql,
            qeema: tul_u64(nass.nass.len()),
            saqf: AQSA_TUL_NASS,
        })?;
        // The units are built before anything is appended, so a string that
        // cannot be expressed leaves the buffer exactly as it was rather than
        // half-written behind a length field that now lies.
        let mut wahdat = Vec::new();
        match nass.tarmiz {
            TarmizNass::Khali => {},
            TarmizNass::Ansi => {
                for harf in nass.nass.chars() {
                    let wahda =
                        u8::try_from(u32::from(harf)).map_err(|_| KhataUnreal::MawridTalif {
                            ism,
                            haql,
                            qeema: u64::from(u32::from(harf)),
                            hadd: 0x100,
                        })?;
                    wahdat.push(wahda);
                }
                wahdat.push(0);
            },
            TarmizNass::Utf16 => {
                for wahda in nass.nass.encode_utf16() {
                    wahdat.extend_from_slice(&wahda.to_le_bytes());
                }
                wahdat.extend_from_slice(&0u16.to_le_bytes());
            },
        }
        self.uktub_i32(tul);
        self.bayt.extend_from_slice(&wahdat);
        Ok(())
    }
}

/// Validates a declared count before it sizes anything, and returns it as a
/// `usize` fit to reserve with.
///
/// Two checks, and both are needed. The ceiling stops a number that is absurd on
/// its own — a namespace count of four billion in an eight kilobyte file. The
/// second stops a number that is merely impossible for *this* file: `aqall` is
/// the smallest number of bytes one element can occupy, so `adad * aqall` is the
/// least the file must still hold, and a count that fails it is refused before a
/// single element is parsed rather than at element ninety thousand.
///
/// # Errors
///
/// [`KhataUnreal::HajmMufrit`] naming `haql` and `saqf` when the count is above
/// the ceiling, and [`KhataUnreal::MawridTalif`] naming `haql` when the bytes it
/// implies are not in the file.
pub fn tahaqquq_adad(
    ism: &'static str,
    haql: &'static str,
    adad: u64,
    saqf: u64,
    aqall: u64,
    baqi: u64,
) -> Result<usize, KhataUnreal> {
    if adad > saqf {
        return Err(KhataUnreal::HajmMufrit {
            haql,
            qeema: adad,
            saqf,
        });
    }
    let matlub = adad.checked_mul(aqall).ok_or(KhataUnreal::HajmMufrit {
        haql,
        qeema: adad,
        saqf,
    })?;
    if matlub > baqi {
        return Err(KhataUnreal::MawridTalif {
            ism,
            haql,
            qeema: adad,
            hadd: baqi,
        });
    }
    hajm_usize(adad).ok_or(KhataUnreal::HajmMufrit {
        haql,
        qeema: adad,
        saqf,
    })
}

/// A signed count as a `u64`, refusing a negative one.
///
/// Unreal writes its counts as `int32`, and a negative count is a corrupt field
/// rather than a large one — the distinction that keeps a sign error from
/// becoming a four-billion-element reservation.
///
/// # Errors
///
/// [`KhataUnreal::MawridTalif`] naming `haql` when the count is below zero.
pub fn adad_musir(ism: &'static str, haql: &'static str, adad: i32) -> Result<u64, KhataUnreal> {
    u64::try_from(adad).map_err(|_| KhataUnreal::MawridTalif {
        ism,
        haql,
        qeema: u64::from(adad.unsigned_abs()),
        hadd: u64::from(i32::MAX.unsigned_abs()),
    })
}

/// Reads a whole file, bounded by [`AQSA_MALAF`].
///
/// **This is one of the two functions in this module that touch the
/// filesystem.** Everything else works over `&[u8]` so that the same reader
/// serves a loose file, a `.pak` entry and an IoStore chunk.
///
/// # Errors
///
/// [`KhataUnreal::KhataMalaf`] naming the path when it cannot be opened or read,
/// and [`KhataUnreal::HajmMufrit`] when the file is larger than [`AQSA_MALAF`] —
/// checked against the directory entry's size, before the read.
pub fn iqra_malaf(masar: &Path) -> Result<Vec<u8>, KhataUnreal> {
    let bayanat = std::fs::metadata(masar).map_err(|sabab| KhataUnreal::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    if bayanat.len() > AQSA_MALAF {
        return Err(KhataUnreal::HajmMufrit {
            haql: "the file's own length",
            qeema: bayanat.len(),
            saqf: AQSA_MALAF,
        });
    }
    std::fs::read(masar).map_err(|sabab| KhataUnreal::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

/// Fills a path into a refusal raised by a reader that had no path to name.
///
/// The slice readers cannot know where their bytes came from — that is the whole
/// point of taking a slice — so [`KhataUnreal::SihrGhayrMutabaq`] leaves the path
/// empty. The convenience readers do know, and repair the message on the way
/// out, because "a file does not carry the .locmeta magic" is a worse message
/// than the same sentence with the file named.
#[must_use]
pub fn sammi_masar(khata: KhataUnreal, masar: &Path) -> KhataUnreal {
    match khata {
        KhataUnreal::SihrGhayrMutabaq { ism, .. } => KhataUnreal::SihrGhayrMutabaq {
            malaf: masar.to_path_buf(),
            ism,
        },
        akhar => akhar,
    }
}

/// Writes a whole file.
///
/// **This is the second of the two functions in this module that touch the
/// filesystem**, and it is here rather than in each format so that there is one
/// place to look when asking what this adapter writes to disk.
///
/// # Errors
///
/// [`KhataUnreal::KhataMalaf`] naming the path.
pub fn uktub_malaf(masar: &Path, bayt: &[u8]) -> Result<(), KhataUnreal> {
    std::fs::write(masar, bayt).map_err(|sabab| KhataUnreal::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

/// Decodes UTF-16 code units, naming the first unit that is not valid.
///
/// Refused rather than replaced. `String::from_utf16_lossy` would put U+FFFD
/// into a game's dialogue and hand it onward as a translation, and the file it
/// came from would go on being treated as readable.
fn min_utf16(wahdat: &[u16], fahras: u32) -> Result<String, KhataUnreal> {
    let mut nass = String::with_capacity(wahdat.len());
    let mut mawqi = 0usize;
    while let Some(wahda) = wahdat.get(mawqi).copied() {
        if (0xD800..0xDC00).contains(&wahda) {
            let thani = wahdat
                .get(mawqi.saturating_add(1))
                .copied()
                .ok_or_else(|| ghayr_salih(fahras, mawqi))?;
            if !(0xDC00..0xE000).contains(&thani) {
                return Err(ghayr_salih(fahras, mawqi));
            }
            let raqm = 0x1_0000u32
                .saturating_add(u32::from(wahda.wrapping_sub(0xD800)) << 10)
                .saturating_add(u32::from(thani.wrapping_sub(0xDC00)));
            nass.push(char::from_u32(raqm).ok_or_else(|| ghayr_salih(fahras, mawqi))?);
            mawqi = mawqi.saturating_add(2);
        } else if (0xDC00..0xE000).contains(&wahda) {
            return Err(ghayr_salih(fahras, mawqi));
        } else {
            nass.push(char::from_u32(u32::from(wahda)).ok_or_else(|| ghayr_salih(fahras, mawqi))?);
            mawqi = mawqi.saturating_add(1);
        }
    }
    Ok(nass)
}

/// The refusal for an offset that is not inside what has already been written.
fn talif_izaha(ism: &'static str, haql: &'static str, izaha: usize, hadd: u64) -> KhataUnreal {
    KhataUnreal::MawridTalif {
        ism,
        haql,
        qeema: tul_u64(izaha),
        hadd,
    }
}

/// The invalid-UTF-16 refusal, naming the record and the code unit.
fn ghayr_salih(fahras: u32, mawqi: usize) -> KhataUnreal {
    KhataUnreal::NassGhayrSalih {
        fahras,
        mawqi: u32::try_from(mawqi).unwrap_or(u32::MAX),
    }
}

/// The short-file refusal, in one spelling for the whole module.
fn qaseer(haql: &'static str, tul: usize, matlub: u64) -> KhataUnreal {
    KhataUnreal::MalafQaseer {
        haql,
        tul: tul_u64(tul),
        matlub,
    }
}

/// Refuses a declared string length above the ceiling, before the units are
/// taken and therefore before anything is reserved for them.
const fn tahaqquq_tul_nass(haql: &'static str, adad: u64) -> Result<(), KhataUnreal> {
    if adad > AQSA_TUL_NASS {
        return Err(KhataUnreal::HajmMufrit {
            haql,
            qeema: adad,
            saqf: AQSA_TUL_NASS,
        });
    }
    Ok(())
}

/// One little-endian UTF-16 code unit from a two-byte pair.
fn wahda_min_zawj(zawj: &[u8]) -> u16 {
    let adna = zawj.first().copied().unwrap_or(0);
    let aala = zawj.get(1).copied().unwrap_or(0);
    u16::from_le_bytes([adna, aala])
}
