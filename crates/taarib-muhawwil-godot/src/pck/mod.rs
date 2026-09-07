//! الحاويات — Godot's package archive, and the translation resource inside it.
//!
//! A Godot game is a `.pck`: one file holding every asset the project exported,
//! addressed by the `res://` paths the engine's own file API uses. There is no
//! directory hierarchy in the container — the path *is* the key, stored whole,
//! and the archive is a flat table of `(path, offset, size, digest)` followed by
//! one undifferentiated run of bytes that every entry points into.
//!
//! ```text
//! game.pck
//!   GDPC                      the four magic bytes, at offset 0
//!   header                    format version, engine version, and framing
//!   index                     one entry per file: path, offset, size, MD5
//!   content                   every file's bytes, back to back
//! ```
//!
//! That flatness is the whole reason this module exists in the shape it does.
//! There is nothing to walk and nothing to recurse into: the index is read once,
//! validated against the real length of the file, and then every later question
//! — where is `res://locale/ar.translation`, how long is it, does it match its
//! digest — is answered from memory without touching the container again.
//!
//! ## What this module owns
//!
//! * [`hawiya`] — the package itself: PCK v1 (Godot 3) and v2 (Godot 4),
//!   packages appended to an executable with their trailing offset record, and
//!   the complete framing of encrypted entries.
//! * [`tarjama`] — the `.translation` resource: the plain `Translation` message
//!   list, the `OptimizedTranslation` hash table, and the `RSRC` / `RSCC`
//!   binary-resource wrapper both of them arrive inside.
//!
//! ## The round-trip contract
//!
//! Every reader here is one half of a pair, and the pair has one property that
//! everything above it depends on:
//!
//! > Reading a structure and writing it straight back, with no edit in between,
//! > produces **byte-identical** output.
//!
//! Taarib's Godot delivery is an *additive* patch package mounted over the
//! game's own through `ProjectSettings.load_resource_pack`. The patch carries a
//! `.translation` that must be exactly the shape the engine's own exporter would
//! have produced, because a resource the engine accepts and then reads wrongly
//! is indistinguishable from a resource it accepts and reads correctly — until a
//! player sees the wrong dialogue. With the contract, the test is mechanical:
//! read a resource the engine wrote, write it back, compare. One differing byte
//! is a bug in this module, found on a build machine.
//!
//! Two consequences shape the types below:
//!
//! * **Stored digests are carried, never silently recomputed.** A package
//!   entry's MD5 is read from the index and written back unchanged when the
//!   entry's bytes were not touched. Recomputing it would make a round trip
//!   depend on this build's digest agreeing with whatever the game's exporter
//!   recorded, and packages exist in the wild whose digests are zero because the
//!   exporter never filled them in. A digest is *verified* when it is non-zero
//!   and *preserved* either way.
//! * **A field this build does not interpret is preserved verbatim.** The
//!   sixteen reserved header words, the resource wrapper's reserved run, an
//!   unrecognised flag bit — each is kept and written back exactly. Regenerating
//!   a region whose meaning is unknown is guessing, and the guess stays
//!   invisible until an engine version that reads it refuses the file.
//!
//! ## Every count is judged against the file before it sizes anything
//!
//! These files arrive with somebody's game, and the same readers are pointed at
//! files a mod tool or a translator produced. Every file count, string count,
//! table length and path length in them is a number that decides how much memory
//! this process is about to take — and that process is frequently the game
//! itself, already most of the way through its address space. So each number
//! passes [`tahaqquq_adad`] first: refused above a named ceiling with
//! [`KhataGodot::HajmMufrit`], and refused again with [`KhataGodot::HawiyaTalifa`]
//! when the bytes it implies cannot fit in what is left of the file. Only then
//! does it reach a `with_capacity`.
//!
//! ## Slices, not files
//!
//! Everything works over `&[u8]`. A `.translation` is found loose on disk about
//! as often as it is found inside a package, and a reader that could only open a
//! path would be useless for the packaged case — which is every shipped game.
//! [`iqra_malaf`], [`uktub_malaf`] and [`iftah_khareeta`] are the only three
//! functions in this module that touch the filesystem, and each format's
//! `min_malaf` is a thin convenience over one of them.
//!
//! Packages are mapped rather than read. A `.pck` for a large game is measured
//! in gigabytes and the index is a few hundred kilobytes at the front of it;
//! reading the whole file to find one `res://` path would cost more memory than
//! the game has left.

pub mod hawiya;
pub mod tarjama;

use std::path::Path;

use crate::khata::{KhataGodot, hajm_usize, tul_u64};

/// The largest file [`iqra_malaf`] will read into memory whole.
///
/// Five hundred and twelve mebibytes. A `.translation` for a fifty thousand
/// string script is single-digit megabytes, and the largest loose resource this
/// adapter has any reason to open is smaller than that by an order of magnitude.
/// The ceiling exists because the convenience readers take a path from a user's
/// configuration, and a path is a place a mistake can point.
pub const AQSA_MALAF: u64 = 512 * 1024 * 1024;

/// The largest package [`iftah_khareeta`] will map.
///
/// Sixty-four gibibytes, which is not a memory budget — a mapping reserves
/// address space, not pages — but a sanity bound on a length taken from a
/// directory entry. A file above it is not a game package.
pub const AQSA_HAWIYA: u64 = 64 * 1024 * 1024 * 1024;

/// The most any writer in this module reserves up front, however large its
/// estimate was.
///
/// Sixty-four mebibytes. Past that the buffer grows as it is written, which
/// costs a few reallocations on a file no game ships and is the right trade
/// against reserving hundreds of megabytes on an estimate derived from a header.
pub const AQSA_HAJZ: usize = 64 * 1024 * 1024;

/// The largest number of bytes any single length field may declare.
///
/// Two hundred and fifty-six mebibytes. Every string, every packed array and
/// every path in these formats is checked against it *before* the bytes behind
/// it are taken, so a corrupt length is refused at the field that carried it
/// rather than at the allocator.
pub const AQSA_TUL_HAQL: u64 = 256 * 1024 * 1024;

/// A structure that reads and writes as one whole run of bytes.
///
/// The trait exists to state the round-trip contract in code rather than only in
/// prose: anything implementing it promises that [`Mawrid::min_bayt`] followed by
/// [`Mawrid::ila_bayt`], with no edit between them, returns the input bytes
/// exactly. The implementors are [`hawiya::Fahras`] — the package header and its
/// index, which is the region a patch tool rewrites — and the two translation
/// resources in [`tarjama`].
pub trait Mawrid: Sized {
    /// The format's name, as it appears in every refusal this module raises.
    const ISM: &'static str;

    /// Reads and validates the whole structure.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] when a field runs past the end,
    /// [`KhataGodot::SihrGhayrMutabaq`] when the magic is not this format's,
    /// [`KhataGodot::IsdarGhayrMadum`] for a version above what this build
    /// reads, [`KhataGodot::HajmMufrit`] for a count above its ceiling,
    /// [`KhataGodot::HawiyaTalifa`] for a field inconsistent with the file, and
    /// [`KhataGodot::NassGhayrSalih`] for a string that is not valid UTF-8.
    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataGodot>;

    /// Writes the structure back out.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when a value held in memory is longer than the
    /// format's length field can express, and [`KhataGodot::HawiyaTalifa`] when a
    /// value cannot be expressed in the version this structure declares.
    fn ila_bayt(&self) -> Result<Vec<u8>, KhataGodot>;
}

/// A bounded cursor over one structure's bytes.
///
/// Every read names the field it is reading, so a truncated file produces a
/// refusal that says which field ran out rather than a generic short read. The
/// cursor also counts strings, so [`KhataGodot::NassGhayrSalih`] can say which
/// string in the file was not valid UTF-8 — the twelve thousandth message in a
/// `.translation` is a thing a person can go and look at.
///
/// The cursor never wraps and never indexes: every advance is a checked add and
/// every take is a `get`, so a hostile length produces a refusal rather than a
/// panic inside a game process that has no business dying because a package was
/// edited.
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
    ///
    /// What a reader needs when a structure carries an absolute offset into its
    /// own container rather than a relative one — which is every offset in a
    /// package index and every internal-resource offset in an `RSRC`.
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
    #[must_use]
    pub fn baqiya(&self) -> &'a [u8] {
        self.bayt.get(self.mawqi..).unwrap_or(&[])
    }

    /// How many strings this cursor has read.
    #[must_use]
    pub const fn adad_nusus(&self) -> u32 {
        self.fahras
    }

    /// Moves the cursor to an absolute offset, refusing one outside the buffer.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] naming `haql`, because an offset a file
    /// declares that points outside that file is a corrupt field and not a short
    /// read — the distinction matters to whoever reads the message.
    pub fn iqfiz(&mut self, haql: &'static str, izaha: u64) -> Result<(), KhataGodot> {
        let tul = self.bayt.len();
        let mawqi = hajm_usize(izaha)
            .filter(|mawqi| *mawqi <= tul)
            .ok_or_else(|| KhataGodot::HawiyaTalifa {
                ism: self.ism,
                haql,
                qeema: izaha,
                hadd: tul_u64(tul),
            })?;
        self.mawqi = mawqi;
        Ok(())
    }

    /// Skips `adad` bytes without looking at them.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql` when they are not there. Skipped
    /// bytes are still bounded: a reserved run this build does not interpret is
    /// a run whose *presence* the format still promises.
    pub fn takhatta(&mut self, haql: &'static str, adad: u64) -> Result<(), KhataGodot> {
        let _ = self.iqra_bayt(haql, adad)?;
        Ok(())
    }

    /// Takes `adad` bytes.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql` when they are not there.
    pub fn iqra_bayt(&mut self, haql: &'static str, adad: u64) -> Result<&'a [u8], KhataGodot> {
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
    /// [`KhataGodot::MalafQaseer`] naming `haql`.
    pub fn iqra_masfufa<const N: usize>(
        &mut self,
        haql: &'static str,
    ) -> Result<[u8; N], KhataGodot> {
        let tul = self.bayt.len();
        let khana = self.iqra_bayt(haql, tul_u64(N))?;
        khana.try_into().map_err(|_| qaseer(haql, tul, tul_u64(N)))
    }

    /// Takes one byte.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql`.
    pub fn iqra_u8(&mut self, haql: &'static str) -> Result<u8, KhataGodot> {
        let khana: [u8; 1] = self.iqra_masfufa(haql)?;
        Ok(khana.first().copied().unwrap_or(0))
    }

    /// Takes four little-endian bytes.
    ///
    /// Every integer in both of these formats is little-endian on every platform
    /// Godot exports to, and the one field that could say otherwise — the binary
    /// resource wrapper's endianness word — is refused rather than honoured. See
    /// [`tarjama`].
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql`.
    pub fn iqra_u32(&mut self, haql: &'static str) -> Result<u32, KhataGodot> {
        Ok(u32::from_le_bytes(self.iqra_masfufa::<4>(haql)?))
    }

    /// Takes eight little-endian bytes.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql`.
    pub fn iqra_u64(&mut self, haql: &'static str) -> Result<u64, KhataGodot> {
        Ok(u64::from_le_bytes(self.iqra_masfufa::<8>(haql)?))
    }

    /// Takes four little-endian bytes as signed.
    ///
    /// The `hash_table` and `bucket_table` of an `OptimizedTranslation` are
    /// `PackedInt32Array`, and the empty-slot sentinel in the first of them is
    /// `-1`. Reading those words unsigned and comparing against `0xFFFF_FFFF` is
    /// the same value by a different name; reading them signed is what the
    /// format says, so that is what this does.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql`.
    pub fn iqra_i32(&mut self, haql: &'static str) -> Result<i32, KhataGodot> {
        Ok(i32::from_le_bytes(self.iqra_masfufa::<4>(haql)?))
    }
}

impl Qari<'_> {
    /// Decodes a run this cursor already took as UTF-8, counting it as one
    /// string.
    ///
    /// Refused rather than replaced. `String::from_utf8_lossy` would put U+FFFD
    /// into a game's dialogue and hand the result onward as a translation, and
    /// the file it came from would go on being treated as readable. The index of
    /// the string and the byte inside it are both named, because "string 11 842
    /// is invalid at byte 7" is a thing a person can go and look at and "invalid
    /// UTF-8" is not.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::NassGhayrSalih`] naming the string's index and the first
    /// byte that is not valid.
    pub fn nass(&mut self, khaam: &[u8]) -> Result<String, KhataGodot> {
        let fahras = self.fahras;
        self.fahras = self.fahras.saturating_add(1);
        match std::str::from_utf8(khaam) {
            Ok(nass) => Ok(nass.to_owned()),
            Err(khata) => Err(KhataGodot::NassGhayrSalih {
                fahras,
                mawqi: u32::try_from(khata.valid_up_to()).unwrap_or(u32::MAX),
            }),
        }
    }

    /// Takes `adad` bytes and decodes them as one UTF-8 string.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::MalafQaseer`] naming `haql` when the bytes are not there,
    /// [`KhataGodot::HajmMufrit`] when `adad` is above [`AQSA_TUL_HAQL`] — checked
    /// before the bytes are taken, so a corrupt length reserves nothing — and
    /// [`KhataGodot::NassGhayrSalih`] when the bytes are not valid UTF-8.
    pub fn iqra_nass(&mut self, haql: &'static str, adad: u64) -> Result<String, KhataGodot> {
        tahaqquq_tul(haql, adad)?;
        let khaam = self.iqra_bayt(haql, adad)?;
        self.nass(khaam)
    }

    /// The refusal this cursor raises for a field that contradicts the file.
    ///
    /// Public because the format readers raise it about fields the cursor itself
    /// never sees — an entry offset that lands past the end of the package, a
    /// bucket index outside the bucket table — and every one of those refusals
    /// should name the same format the cursor names.
    #[must_use]
    pub const fn talif(&self, haql: &'static str, qeema: u64, hadd: u64) -> KhataGodot {
        KhataGodot::HawiyaTalifa {
            ism: self.ism,
            haql,
            qeema,
            hadd,
        }
    }
}

/// A growing buffer that writes the same shapes [`Qari`] reads.
///
/// Deliberately infallible on the buffer side — a `Vec<u8>` does not run out
/// mid-write — so the only errors raised anywhere near it are about values that
/// cannot be expressed in the format at all.
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
    ///
    /// The reservation is capped at [`AQSA_HAJZ`]: the size a caller estimates is
    /// derived from data a reader already bounded, and an estimate is still not a
    /// reason to ask the allocator for a gigabyte before the first byte exists.
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

    /// Appends eight little-endian bytes.
    pub fn uktub_u64(&mut self, qeema: u64) {
        self.bayt.extend_from_slice(&qeema.to_le_bytes());
    }

    /// Appends `adad` zero bytes.
    ///
    /// The one call that writes bytes with no meaning: a reserved header word, a
    /// path's alignment padding, the gap between two content entries. Every one
    /// of them is zero because the engine's own writer makes them zero, and a
    /// patch package that filled them with anything else would differ from an
    /// engine-written package in a way no test would explain.
    pub fn uktub_asfar(&mut self, adad: usize) {
        self.bayt.resize(self.bayt.len().saturating_add(adad), 0);
    }

    /// Overwrites four little-endian bytes already written.
    ///
    /// How a forward reference is resolved: the entry count in a package header
    /// is written as a placeholder and filled in once every entry is known.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when `izaha` is not four bytes inside what has
    /// already been written, which would mean the writer lost track of its own
    /// header.
    pub fn uktub_u32_fi(
        &mut self,
        ism: &'static str,
        haql: &'static str,
        izaha: usize,
        qeema: u32,
    ) -> Result<(), KhataGodot> {
        let hadd = tul_u64(self.bayt.len());
        let nihaya = izaha
            .checked_add(4)
            .ok_or_else(|| talif_izaha(ism, haql, izaha, hadd))?;
        let khana = self
            .bayt
            .get_mut(izaha..nihaya)
            .ok_or_else(|| talif_izaha(ism, haql, izaha, hadd))?;
        khana.copy_from_slice(&qeema.to_le_bytes());
        Ok(())
    }

    /// Overwrites eight little-endian bytes already written.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HawiyaTalifa`] when `izaha` is not eight bytes inside what
    /// has already been written.
    pub fn uktub_u64_fi(
        &mut self,
        ism: &'static str,
        haql: &'static str,
        izaha: usize,
        qeema: u64,
    ) -> Result<(), KhataGodot> {
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

    /// Pads the buffer up to the next multiple of `muhadhah` with zeros.
    ///
    /// Both formats align: a package pads every stored path so the index stays
    /// four-byte aligned, and a binary resource pads every `PackedByteArray` the
    /// same way. The alignment is always a power of two, so the remainder is a
    /// mask and not a division — which is also why this cannot divide by zero.
    pub fn hadhi(&mut self, muhadhah: usize) {
        let qinaa = muhadhah.saturating_sub(1);
        if muhadhah == 0 || muhadhah & qinaa != 0 {
            return;
        }
        let zaid = self.bayt.len() & qinaa;
        if zaid != 0 {
            self.uktub_asfar(muhadhah.saturating_sub(zaid));
        }
    }
}

/// Validates a declared count before it sizes anything, and returns it as a
/// `usize` fit to reserve with.
///
/// Two checks, and both are needed. The ceiling stops a number that is absurd on
/// its own — a file count of four billion in an eight kilobyte package. The
/// second stops a number that is merely impossible for *this* file: `aqall` is
/// the smallest number of bytes one element can occupy, so `adad * aqall` is the
/// least the file must still hold, and a count that fails it is refused before a
/// single element is parsed rather than at element ninety thousand with a
/// half-built table already in memory.
///
/// # Errors
///
/// [`KhataGodot::HajmMufrit`] naming `haql` and `saqf` when the count is above
/// the ceiling, and [`KhataGodot::HawiyaTalifa`] naming `haql` when the bytes it
/// implies are not in the file.
pub fn tahaqquq_adad(
    ism: &'static str,
    haql: &'static str,
    adad: u64,
    saqf: u64,
    aqall: u64,
    baqi: u64,
) -> Result<usize, KhataGodot> {
    if adad > saqf {
        return Err(KhataGodot::HajmMufrit {
            haql,
            qeema: adad,
            saqf,
        });
    }
    let matlub = adad.checked_mul(aqall).ok_or(KhataGodot::HajmMufrit {
        haql,
        qeema: adad,
        saqf,
    })?;
    if matlub > baqi {
        return Err(KhataGodot::HawiyaTalifa {
            ism,
            haql,
            qeema: adad,
            hadd: baqi,
        });
    }
    hajm_usize(adad).ok_or(KhataGodot::HajmMufrit {
        haql,
        qeema: adad,
        saqf,
    })
}

/// Refuses a declared byte length above [`AQSA_TUL_HAQL`], before the bytes are
/// taken and therefore before anything is reserved for them.
///
/// # Errors
///
/// [`KhataGodot::HajmMufrit`] naming `haql`.
pub const fn tahaqquq_tul(haql: &'static str, adad: u64) -> Result<(), KhataGodot> {
    if adad > AQSA_TUL_HAQL {
        return Err(KhataGodot::HajmMufrit {
            haql,
            qeema: adad,
            saqf: AQSA_TUL_HAQL,
        });
    }
    Ok(())
}

/// How many bytes of padding follow a run of `tul` bytes aligned to `muhadhah`.
///
/// Godot's own `_get_pad`: nothing when the run already lands on the boundary,
/// and the difference otherwise. Written with a mask rather than a remainder
/// because the workspace denies integer division and because every alignment
/// either format uses is a power of two; a `muhadhah` that is not one returns
/// zero rather than a wrong answer.
#[must_use]
pub const fn hashw_muhadhah(tul: usize, muhadhah: usize) -> usize {
    let qinaa = muhadhah.wrapping_sub(1);
    if muhadhah == 0 || muhadhah & qinaa != 0 {
        return 0;
    }
    let zaid = tul & qinaa;
    if zaid == 0 { 0 } else { muhadhah - zaid }
}

/// Reads a whole file, bounded by [`AQSA_MALAF`].
///
/// **One of the three functions in this module that touch the filesystem.**
/// Everything else works over `&[u8]` so that the same reader serves a loose
/// resource and one lifted out of a package.
///
/// # Errors
///
/// [`KhataGodot::KhataMalaf`] naming the path when it cannot be opened or read,
/// and [`KhataGodot::HajmMufrit`] when the file is larger than [`AQSA_MALAF`] —
/// checked against the directory entry's size, before the read.
pub fn iqra_malaf(masar: &Path) -> Result<Vec<u8>, KhataGodot> {
    let bayanat = std::fs::metadata(masar).map_err(|sabab| KhataGodot::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    if bayanat.len() > AQSA_MALAF {
        return Err(KhataGodot::HajmMufrit {
            haql: "the file's own length",
            qeema: bayanat.len(),
            saqf: AQSA_MALAF,
        });
    }
    std::fs::read(masar).map_err(|sabab| KhataGodot::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

/// Writes a whole file.
///
/// **The second of the three functions in this module that touch the
/// filesystem**, and it is here rather than in each format so that there is one
/// place to look when asking what this adapter writes to disk. It is used for
/// exactly one thing: the additive patch package. No file a game shipped is ever
/// passed to it.
///
/// # Errors
///
/// [`KhataGodot::KhataMalaf`] naming the path.
pub fn uktub_malaf(masar: &Path, bayt: &[u8]) -> Result<(), KhataGodot> {
    std::fs::write(masar, bayt).map_err(|sabab| KhataGodot::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

/// Maps a package read-only.
///
/// **The third of the three functions in this module that touch the
/// filesystem**, and the only way a package is ever opened. A `.pck` for a large
/// game is measured in gigabytes; its index is a few hundred kilobytes at the
/// front. Reading the whole file to answer "where is `res://locale/ar.translation`"
/// would cost more memory than a running game has left, and this adapter is
/// frequently *inside* that game.
///
/// The mapping is read-only and the file is never written through it. A package
/// a game shipped is never modified in place — the patch is a separate file the
/// engine mounts over it.
///
/// # Errors
///
/// [`KhataGodot::KhataMalaf`] naming the path when it cannot be opened or
/// mapped, and [`KhataGodot::HajmMufrit`] when the file is larger than
/// [`AQSA_HAWIYA`].
pub fn iftah_khareeta(masar: &Path) -> Result<memmap2::Mmap, KhataGodot> {
    let malaf = std::fs::File::open(masar).map_err(|sabab| KhataGodot::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    let bayanat = malaf.metadata().map_err(|sabab| KhataGodot::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    if bayanat.len() > AQSA_HAWIYA {
        return Err(KhataGodot::HajmMufrit {
            haql: "the package's own length",
            qeema: bayanat.len(),
            saqf: AQSA_HAWIYA,
        });
    }
    // SAFETY: `memmap2` cannot promise a mapping stays valid if another process
    // truncates the file underneath it, and no API can. The exposure is bounded
    // the same way the rest of this module is bounded: the mapping is read-only,
    // nothing is written through it, and every read of it goes through `Qari`,
    // which takes slices with `get` and never indexes — so a shortened mapping
    // produces a refusal naming a field, not an out-of-bounds read. The
    // alternative, reading a multi-gigabyte package into a game process's
    // address space to look at its first few hundred kilobytes, is the worse
    // risk of the two.
    let khareeta = unsafe { memmap2::Mmap::map(&malaf) };
    khareeta.map_err(|sabab| KhataGodot::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })
}

/// Fills a path into a refusal raised by a reader that had no path to name.
///
/// The slice readers cannot know where their bytes came from — that is the point
/// of taking a slice — so they raise [`KhataGodot::SihrGhayrMutabaq`] and
/// [`KhataGodot::PckMushaffar`] with an empty path. The convenience readers do
/// know, and repair the message on the way out, because "a file does not carry
/// the PCK magic" is a worse message than the same sentence with the file named.
#[must_use]
pub fn sammi_masar(khata: KhataGodot, masar: &Path) -> KhataGodot {
    match khata {
        KhataGodot::SihrGhayrMutabaq { .. } => KhataGodot::SihrGhayrMutabaq {
            masar: masar.to_path_buf(),
        },
        KhataGodot::PckMushaffar { .. } => KhataGodot::PckMushaffar {
            masar: masar.to_path_buf(),
        },
        KhataGodot::MiftahGhayrSalih { sabab, .. } => KhataGodot::MiftahGhayrSalih {
            masar: masar.to_path_buf(),
            sabab,
        },
        akhar => akhar,
    }
}

/// The refusal for an offset that is not inside what has already been written.
fn talif_izaha(ism: &'static str, haql: &'static str, izaha: usize, hadd: u64) -> KhataGodot {
    KhataGodot::HawiyaTalifa {
        ism,
        haql,
        qeema: tul_u64(izaha),
        hadd,
    }
}

/// The short-file refusal, in one spelling for the whole module.
fn qaseer(haql: &'static str, tul: usize, matlub: u64) -> KhataGodot {
    KhataGodot::MalafQaseer {
        haql,
        tul: tul_u64(tul),
        matlub,
    }
}

/// The four words MD5 starts from, RFC 1321 section 3.3.
const BIDAYAT_MD5: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];

/// The per-step left rotations, RFC 1321 section 3.4.
const DAWWARAT_MD5: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, //
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, //
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, //
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// The sixty-four additive constants, `floor(2^32 * abs(sin(i + 1)))`.
///
/// Written out rather than computed. Deriving them from `f64::sin` at startup
/// would make a checksum depend on the platform's libm rounding, which is a
/// difference of one bit in one constant and a completely different digest.
const THAWABIT_MD5: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee, //
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501, //
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be, //
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821, //
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa, //
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8, //
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed, //
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a, //
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c, //
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70, //
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05, //
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665, //
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039, //
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1, //
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1, //
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

/// MD5, RFC 1321, implemented here rather than pulled in.
///
/// This is not a security primitive and is not used as one. MD5 is the digest
/// Godot's own packer records for every entry in a `.pck`, so reading a package
/// and answering "do these bytes match what the exporter put here" requires
/// exactly this function and no other. Nothing in Taarib trusts an MD5 to prove
/// anything about *authorship* — the patch container's own integrity is BLAKE3
/// and a signature, in `taarib-ruqaa`.
///
/// It is written out rather than taken as a dependency for two reasons. It is
/// a hundred lines with no configuration, and the alternative is adding a
/// crate — with its own transitive tree and its own release cadence — to a
/// library that is loaded inside somebody else's game process. The smaller that
/// tree is, the fewer things can go wrong in a place a crash is a lost save.
///
/// The state is streaming so that a large entry can be verified from a memory
/// mapping in fixed-size pieces rather than after being copied whole.
#[derive(Debug, Clone)]
pub struct HasibMd5 {
    halat: [u32; 4],
    tul: u64,
    mahjuz: [u8; 64],
    fi_al_mahjuz: usize,
}

impl Default for HasibMd5 {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl HasibMd5 {
    /// A fresh digest state.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            halat: BIDAYAT_MD5,
            tul: 0,
            mahjuz: [0; 64],
            fi_al_mahjuz: 0,
        }
    }

    /// Feeds more bytes in.
    pub fn zid(&mut self, bayt: &[u8]) {
        self.tul = self.tul.wrapping_add(tul_u64(bayt.len()));
        let mut baqi = bayt;
        if self.fi_al_mahjuz > 0 {
            let naqis = 64usize.saturating_sub(self.fi_al_mahjuz);
            let akhdh = naqis.min(baqi.len());
            let nihaya = self.fi_al_mahjuz.saturating_add(akhdh);
            if let (Some(hadaf), Some(masdar)) = (
                self.mahjuz.get_mut(self.fi_al_mahjuz..nihaya),
                baqi.get(..akhdh),
            ) {
                hadaf.copy_from_slice(masdar);
            }
            self.fi_al_mahjuz = nihaya;
            baqi = baqi.get(akhdh..).unwrap_or(&[]);
            // Short of a full block means the input ran out, not that the block
            // is ready: returning here is what keeps a partly filled buffer from
            // being overwritten by the tail handling below.
            if self.fi_al_mahjuz < 64 {
                return;
            }
            let kamila = self.mahjuz;
            Self::idfa(&mut self.halat, &kamila);
            self.fi_al_mahjuz = 0;
        }
        let mut qita = baqi.chunks_exact(64);
        for kutla in qita.by_ref() {
            Self::idfa(&mut self.halat, kutla);
        }
        let dhayl = qita.remainder();
        if let Some(hadaf) = self.mahjuz.get_mut(..dhayl.len()) {
            hadaf.copy_from_slice(dhayl);
        }
        self.fi_al_mahjuz = dhayl.len();
    }

    /// Pads, absorbs the length, and returns the sixteen byte digest.
    #[must_use]
    pub fn akhtim(mut self) -> [u8; 16] {
        // Captured before padding, because the padding is not part of the
        // message and the length field describes the message.
        let bitat = self.tul.wrapping_mul(8);
        self.zid(&[0x80]);
        let naqis = if self.fi_al_mahjuz <= 56 {
            56usize.saturating_sub(self.fi_al_mahjuz)
        } else {
            120usize.saturating_sub(self.fi_al_mahjuz)
        };
        let hashw = [0u8; 64];
        if let Some(khana) = hashw.get(..naqis) {
            self.zid(khana);
        }
        self.zid(&bitat.to_le_bytes());
        let mut natija = [0u8; 16];
        for (zawj, qeema) in natija.chunks_exact_mut(4).zip(self.halat) {
            zawj.copy_from_slice(&qeema.to_le_bytes());
        }
        natija
    }

    /// One sixty-four byte block, RFC 1321 section 3.4.
    fn idfa(halat: &mut [u32; 4], kutla: &[u8]) {
        let mut risala = [0u32; 16];
        for (khana, zawj) in risala.iter_mut().zip(kutla.chunks_exact(4)) {
            *khana = u32::from_le_bytes([
                zawj.first().copied().unwrap_or(0),
                zawj.get(1).copied().unwrap_or(0),
                zawj.get(2).copied().unwrap_or(0),
                zawj.get(3).copied().unwrap_or(0),
            ]);
        }
        let mut alif = halat.first().copied().unwrap_or(0);
        let mut ba = halat.get(1).copied().unwrap_or(0);
        let mut jim = halat.get(2).copied().unwrap_or(0);
        let mut dal = halat.get(3).copied().unwrap_or(0);
        for khatwa in 0usize..64 {
            // The message-word index is a mask rather than a remainder: sixteen
            // is a power of two, so `& 15` is the same value the specification's
            // `mod 16` gives, and the workspace denies the operator.
            let (daala, fahras) = match khatwa {
                0..16 => ((ba & jim) | (!ba & dal), khatwa),
                16..32 => (
                    (dal & ba) | (!dal & jim),
                    khatwa.wrapping_mul(5).wrapping_add(1) & 15,
                ),
                32..48 => (ba ^ jim ^ dal, khatwa.wrapping_mul(3).wrapping_add(5) & 15),
                _ => (jim ^ (ba | !dal), khatwa.wrapping_mul(7) & 15),
            };
            let thabit = THAWABIT_MD5.get(khatwa).copied().unwrap_or(0);
            let dawra = DAWWARAT_MD5.get(khatwa).copied().unwrap_or(0);
            let kalima = risala.get(fahras).copied().unwrap_or(0);
            let majmu = alif
                .wrapping_add(daala)
                .wrapping_add(thabit)
                .wrapping_add(kalima)
                .rotate_left(dawra);
            alif = dal;
            dal = jim;
            jim = ba;
            ba = ba.wrapping_add(majmu);
        }
        for (khana, qeema) in halat.iter_mut().zip([alif, ba, jim, dal]) {
            *khana = khana.wrapping_add(qeema);
        }
    }
}

/// The MD5 of one run of bytes.
#[must_use]
pub fn basma_md5(bayt: &[u8]) -> [u8; 16] {
    let mut hasib = HasibMd5::jadeed();
    hasib.zid(bayt);
    hasib.akhtim()
}

/// Whether a recorded digest is sixteen zero bytes.
///
/// A zero digest is the format's way of saying "not recorded". Godot's own
/// exporter fills it in, but packages exist — from third-party packers, from
/// tools that rebuilt an index without the content in hand — whose digests are
/// all zero. Verifying against zero would reject every one of those packages for
/// a corruption that is not there, so a zero digest is carried forward untouched
/// and not checked, and any other digest is checked and must match.
#[must_use]
pub fn basma_sifriya(basma: [u8; 16]) -> bool {
    basma.iter().all(|wahid| *wahid == 0)
}
