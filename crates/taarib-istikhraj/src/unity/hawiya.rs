//! الحاويات — the two containers Unity ships text inside, and the one boolean
//! that decides whether any of that text is readable at all.
//!
//! A Unity game's data is two nested formats and nothing else:
//!
//! ```text
//! game_Data/
//!   resources.assets          a SerializedFile, loose on disk
//!   sharedassets0.assets      another one
//!   level0                    another one, no extension
//!   globalgamemanagers        another one
//!   StreamingAssets/
//!     aa/StandaloneWindows64/
//!       ui_assets_all.bundle  a UnityFS bundle, holding SerializedFiles
//! ```
//!
//! [`Huzma`] is the outer one: a signature, a header, a table of compressed
//! blocks that concatenate into one blob, and a directory naming the files
//! inside that blob. [`Mulsal`] is the inner one: a header, a list of *types*, a
//! list of *objects*, and a run of object bytes.
//!
//! ## The type list is the whole game
//!
//! A `Mulsal`'s object table says, for each object, where its bytes start, how
//! many there are and which type index it has. It does **not** say what those
//! bytes mean. The meaning lives in the type list, as a [`ShajaratAnwa`] — a
//! flat sequence of nodes naming every field of the type in serialization order,
//! with its type name, its size and its alignment flag.
//!
//! Unity's build settings can strip that. An IL2CPP release build routinely
//! ships with `m_EnableTypeTree` false, and then the object table is a list of
//! byte ranges with no field names, no field types and no field boundaries.
//!
//! **There is no recovering that from the bytes.** A `SInt32` holding `5`
//! followed by five ASCII bytes is byte-identical to a `string` of length five,
//! and byte-identical again to two integers whose values happen to look like
//! that. Nothing in the file distinguishes them, so nothing in this module
//! attempts to. [`Mulsal::ladayhi_shajarat_anwa`] answers the question once, and
//! every caller in [`crate::unity::kaain`] routes on it: with a tree, objects
//! are walked; without one, objects are refused with
//! [`SababRafd::BilaShajaratAnwa`][crate::rafd::SababRafd::BilaShajaratAnwa] and
//! the strings are recovered by running the game with capture on, where they are
//! drawn on screen and can simply be read.
//!
//! ## What this build reads, and what it refuses by name
//!
//! | container | versions read | versions refused |
//! | --- | --- | --- |
//! | `UnityFS` bundle | format 6, 7, 8 | anything else, as `IsdarGhayrMadum` |
//! | `UnityArchive` bundle | format 6, 7, 8 | anything else, as `IsdarGhayrMadum` |
//! | `UnityRaw` bundle | format 1 – 5 | anything else, as `IsdarGhayrMadum` |
//! | `UnityWeb` bundle | none | all, as `SighaMajhula` — the payload is LZMA |
//! | block compression | none, LZ4, LZ4HC | LZMA, as `SighaMajhula` |
//! | `SerializedFile` | format 5 – 23 | anything else, as `IsdarGhayrMadum` |
//!
//! ### Why LZMA is refused rather than decompressed
//!
//! Unity's LZMA blocks are raw streams with a five-byte `lzma_alone` property
//! header and no end marker, and the only Rust decoders for that are either a
//! bindgen wrapper around the C library or a pure-Rust crate carrying its own
//! range coder. Both are a dependency this crate would take on purely to read a
//! layout Unity stopped defaulting to in 5.3, in 2015. LZ4 covers every bundle a
//! game shipped this decade.
//!
//! So an LZMA bundle is named and refused, not attempted. That refusal is
//! [`SababRafd::SighaMajhula`][crate::rafd::SababRafd::SighaMajhula], which
//! [`yuslihuhu_iltiqat`][crate::rafd::SababRafd::yuslihuhu_iltiqat] answers true
//! for — so the user is offered the capture path rather than a dead end, which
//! is the correct remedy for a container whose strings exist and are drawn.
//!
//! ## Every read is bounds-checked, and a short file is a refusal
//!
//! These bytes arrive with somebody's game, and the same readers are pointed at
//! files a mod tool wrote. Every count, size and offset in them is a number that
//! decides how much memory this process is about to take. [`Qari`] never
//! indexes, never wraps and never trusts a length: each read names the field it
//! is reading, so a truncated bundle produces "the block table ran out at block
//! 91 of a declared 400" rather than a panic, and each count passes a named
//! ceiling before it sizes anything.

use std::sync::OnceLock;

use rustc_hash::FxHashMap;

// ---------------------------------------------------------------------------
// Ceilings
//
// Each is a refusal boundary, not a memory budget. A container declaring more
// than one of these is damaged or hostile, and either way the answer is to
// refuse before allocating rather than after.
// ---------------------------------------------------------------------------

/// The largest single file this build maps.
///
/// Four gibibytes. `data.unity3d` for a large mobile port reaches two, and a
/// `SerializedFile`'s own `fileSize` field was a `u32` until format 22, so
/// nothing this reader understands can legitimately exceed it.
pub const AQSA_HAJM_MALAF: u64 = 4 * 1024 * 1024 * 1024;

/// The largest decompressed bundle payload this build materialises.
///
/// Two gibibytes. Unlike the file ceiling this one *is* a memory budget: the
/// blocks are decompressed into one contiguous `Vec` because the directory
/// addresses that blob as a flat range, and the machine doing the extraction is
/// frequently also running the game.
pub const AQSA_HAJM_MAFTUH: u64 = 2 * 1024 * 1024 * 1024;

/// The most storage blocks a bundle may declare.
///
/// A block is at most 128 KiB uncompressed in every bundle Unity writes, so this
/// permits the full two-gibibyte payload above with room to spare, and refuses a
/// count that would make the block table itself larger than most bundles.
pub const AQSA_KUTAL: u32 = 262_144;

/// The most directory nodes a bundle may declare.
///
/// A bundle holds a handful of `SerializedFile`s and their `.resS` sidecars.
/// Sixty-five thousand is four orders of magnitude past anything observed and
/// still small enough that the path strings cannot exhaust memory.
pub const AQSA_UQAD: u32 = 65_536;

/// The most serialized types one `SerializedFile` may declare.
///
/// Every distinct `MonoBehaviour` script in the file gets its own entry, so a
/// large game's `globalgamemanagers` reaches four figures. Sixteen thousand is
/// the refusal point.
pub const AQSA_ANWA: u32 = 16_384;

/// The most objects one `SerializedFile` may declare.
///
/// A scene file for an open-world game reaches a few hundred thousand. Two
/// million refuses a corrupt count before the object table is sized.
pub const AQSA_KAAINAT: u32 = 2_000_000;

/// The most nodes one type tree may declare.
///
/// A deeply nested `MonoBehaviour` with generic collections produces a few
/// thousand. A quarter of a million is the refusal point, and it is checked
/// before the node vector is reserved because the count is read straight from
/// the file.
pub const AQSA_UQAD_SHAJARA: u32 = 262_144;

/// The largest a bundle's decompressed blocks-info table may be.
///
/// Sixty-four mebibytes. The table is sixteen bytes of hash, ten bytes per
/// block and a path string per node — under a megabyte for the largest bundle
/// observed. The ceiling exists because the table's *uncompressed* size is a
/// `u32` the file supplies, and it is used to size the decompression buffer
/// before a single byte has been decoded.
pub const AQSA_DALIL: u32 = 64 * 1024 * 1024;

/// The largest string buffer a flat type tree may declare.
///
/// Sixteen mebibytes of field names. The real figure for a large file is tens of
/// kilobytes; this refuses a corrupt length before the buffer is copied.
pub const AQSA_HAJZ_ASMA: u32 = 16 * 1024 * 1024;

/// The longest NUL-terminated name this build reads.
///
/// Four kibibytes. Unity's own limit on an asset path is shorter than this on
/// every platform, and the ceiling exists so that a missing terminator is a
/// refusal after four kilobytes rather than a scan to the end of a two-gigabyte
/// mapping.
pub const AQSA_TUL_ISM: usize = 4096;

/// How deep the recursive form of the type tree may nest.
///
/// Sixty-four. Unity's own serializer refuses past a shallower depth than this;
/// the ceiling is here because the pre-format-12 tree is read recursively and a
/// corrupt child count is otherwise a stack overflow, which is a crash this
/// process cannot catch.
pub const AQSA_UMQ_SHAJARA: u8 = 64;

/// The most external file references one `SerializedFile` may declare.
pub const AQSA_KHARIJIYAT: u32 = 16_384;

/// The most script-type references one `SerializedFile` may declare.
pub const AQSA_ANWA_SCRIPTS: u32 = 65_536;

// ---------------------------------------------------------------------------
// Width conversions
//
// Written once, here, because every one of them is a place where a cast would
// have silently produced a wrong number on a 32-bit target.
// ---------------------------------------------------------------------------

/// A length as a `u64`, saturating rather than wrapping.
///
/// Saturating is correct for the one use: comparing a real length against a
/// declared one. A saturated value is larger than any declared length that
/// passed a ceiling, so the comparison refuses, which is the safe direction.
#[must_use]
pub fn tul_u64(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

/// A declared size as a `usize`, or `None` when this target cannot hold it.
///
/// `None` rather than a saturated value: a 32-bit build asked to address a
/// six-gigabyte offset must refuse, and a saturated `usize::MAX` would instead
/// produce a bounds failure whose message blamed the file.
#[must_use]
pub fn hajm_usize(qeema: u64) -> Option<usize> {
    usize::try_from(qeema).ok()
}

/// The most any table in this module reserves up front, however large the count
/// the container declared.
///
/// Sixty-five thousand entries. Every count reaching a `with_capacity` here has
/// already passed a ceiling — but those ceilings are set where a *real* game
/// stops being plausible, and [`AQSA_KAAINAT`] alone would let a two-million
/// entry object table be reserved from a number a corrupt file supplied. A table
/// that really is larger grows as it is filled, at the cost of a handful of
/// reallocations on a container no game ships.
pub const AQSA_HAJZ_MABDAI: usize = 65_536;

/// How much to reserve for a declared count.
///
/// Clamped to [`AQSA_HAJZ_MABDAI`], because the count came out of the file and
/// the allocation happens before a single byte of the table has been validated.
#[must_use]
pub fn hajz_mahdud(adad: u32) -> usize {
    usize::try_from(adad)
        .unwrap_or(AQSA_HAJZ_MABDAI)
        .min(AQSA_HAJZ_MABDAI)
}

// ---------------------------------------------------------------------------
// Refusal
// ---------------------------------------------------------------------------

/// Why a container could not be read.
///
/// Narrow on purpose. Each variant maps to exactly one
/// [`SababRafd`][crate::rafd::SababRafd] in [`crate::unity`], and each carries
/// the field name that failed rather than a position — "the block table ran out
/// at block 91 of a declared 400" is something a contributor can act on, and
/// "short read at 0x4c1a" is not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KhataQira {
    /// The bytes named by a field are not in the file.
    #[error("{sigha}: field `{haql}` needs {matlub} byte(s), {mutah} available")]
    MalafQaseer {
        /// The container format, for the message.
        sigha: &'static str,
        /// The field that ran out.
        haql: &'static str,
        /// How many bytes it needed.
        matlub: u64,
        /// How many were left.
        mutah: u64,
    },
    /// A field holds a value the format cannot mean.
    #[error("{sigha}: field `{haql}` is not valid: {sabab}")]
    HaqlTalif {
        /// The container format.
        sigha: &'static str,
        /// The field.
        haql: &'static str,
        /// What was wrong with it.
        sabab: String,
    },
    /// A declared count is above a ceiling this build enforces.
    #[error("{hadd} declares {qeema}, above this build's ceiling of {saqf}")]
    TajawuzHadd {
        /// Which ceiling.
        hadd: &'static str,
        /// What the container declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },
    /// The container is a shape this build does not read.
    #[error("{wujid} is not a format this build reads")]
    SighaMajhula {
        /// What was found, as specifically as it could be named.
        wujid: String,
    },
    /// The container says it is encrypted.
    ///
    /// Distinct from [`KhataQira::SighaMajhula`] because the evidence is
    /// different in kind: this is the container's own declaration, not an
    /// inference from its bytes. The entropy heuristic in [`crate::unity`] never
    /// runs for a container that reaches this — a stated fact outranks a
    /// measurement of randomness, and the two can disagree, because an
    /// encrypted bundle whose blocks were compressed first is high entropy and
    /// one that was `XORed` with a short key is not.
    #[error("{wasf}")]
    Mushaffar {
        /// What the encryption looks like, as specifically as it can be named.
        wasf: String,
    },
    /// A format this build reads, at a version it does not.
    #[error("{sigha} version {wujid} is not one this build handles; it reads {madum}")]
    IsdarGhayrMadum {
        /// The format.
        sigha: &'static str,
        /// The version found.
        wujid: String,
        /// The versions this build handles.
        madum: &'static str,
    },
    /// A string in the container is not valid UTF-8.
    ///
    /// Refused rather than replaced. `String::from_utf8_lossy` would put U+FFFD
    /// into a game's dialogue and hand the result onward as a translation, and
    /// the file it came from would go on being treated as readable.
    #[error("{sigha}: field `{haql}` is not valid UTF-8 at byte {mawqi}")]
    NassGhayrSalih {
        /// The container format.
        sigha: &'static str,
        /// The field.
        haql: &'static str,
        /// The first byte that was not valid.
        mawqi: usize,
    },
    /// A compressed block did not decompress to the size its header declared.
    ///
    /// Named separately from a corrupt field because the remedy differs: a
    /// container whose block table is right and whose payload is wrong is a
    /// truncated download, and telling the user to verify the game's files
    /// actually fixes it.
    #[error("{namat} block {fahras} decompressed to {wujid} bytes, not the declared {mutawaqqa}")]
    DaghtTalif {
        /// The compression the block declared.
        namat: &'static str,
        /// Which block.
        fahras: u32,
        /// What came out.
        wujid: u64,
        /// What the block table said would come out.
        mutawaqqa: u64,
    },
    /// A type tree node names a field whose name this build cannot resolve.
    ///
    /// The flat type tree stores names as offsets into either the file's own
    /// string buffer or a fixed table Unity ships in the engine binary. An
    /// offset into the second one that this build does not have an entry for
    /// leaves the node unnamed — and an unnamed node cannot be read, because the
    /// name is what says whether the next four bytes are a length prefix or an
    /// integer. The object is refused rather than read with a guessed name.
    #[error("type tree node {fahras} names field at common-string offset {izaha}, unknown here")]
    IsmMajhul {
        /// Which node.
        fahras: u32,
        /// The offset that did not resolve.
        izaha: u32,
    },
}

impl KhataQira {
    /// A short read for `haql`, given what was available and what was wanted.
    #[must_use]
    fn qaseer(sigha: &'static str, haql: &'static str, mutah: usize, matlub: u64) -> Self {
        Self::MalafQaseer {
            sigha,
            haql,
            matlub,
            mutah: tul_u64(mutah),
        }
    }
}

/// Refuses a declared count above `saqf`.
///
/// Written as a free function rather than a method on [`Qari`] because the same
/// check guards counts read from a decompressed side buffer, which has no
/// cursor of its own.
///
/// # Errors
///
/// [`KhataQira::TajawuzHadd`] naming the ceiling, or [`KhataQira::HaqlTalif`]
/// when the count is negative — which is a different fault with a different
/// message, since a negative count is a misread structure and an oversized one
/// is a damaged or hostile file.
pub fn tahaqquq_adad(
    sigha: &'static str,
    hadd: &'static str,
    qeema: i32,
    saqf: u32,
) -> Result<u32, KhataQira> {
    let mujab = u32::try_from(qeema).map_err(|_| KhataQira::HaqlTalif {
        sigha,
        haql: hadd,
        sabab: format!("negative count {qeema}"),
    })?;
    if mujab > saqf {
        return Err(KhataQira::TajawuzHadd {
            hadd,
            qeema: u64::from(mujab),
            saqf: u64::from(saqf),
        });
    }
    Ok(mujab)
}

// ---------------------------------------------------------------------------
// The cursor
// ---------------------------------------------------------------------------

/// Which end a multi-byte integer starts at.
///
/// Not a constant, because a `SerializedFile` declares its own: the header is
/// always big-endian and everything after it follows the byte in the header. A
/// file written on a big-endian console and read on a little-endian PC is a real
/// case, and reading it with the wrong order produces field values that are
/// wrong by a byte swap and structures that parse anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nihaya {
    /// Least significant byte first. Every desktop and mobile target.
    Sagheer,
    /// Most significant byte first. Every bundle header, and consoles.
    Kabeer,
}

impl Nihaya {
    /// The order a `SerializedFile` header byte names.
    ///
    /// Unity writes `1` for big-endian and `0` for little. Any other value is a
    /// misread header rather than a third byte order, and the caller checks it.
    #[must_use]
    pub const fn min_bayt(qeema: u8) -> Self {
        if qeema == 0 {
            Self::Sagheer
        } else {
            Self::Kabeer
        }
    }
}

/// A bounded cursor over one container's bytes.
///
/// Three properties matter, and each of them is a bug this reader would
/// otherwise have shipped:
///
/// * **Every read names its field.** A truncated `.bundle` is a common thing —
///   an interrupted store download produces one — and "the directory's node 12
///   path ran out" is a report a user can act on.
/// * **Alignment is absolute, not relative.** Unity's `AlignStream` rounds the
///   position *within the whole file* up to a multiple of four. A cursor handed
///   a slice starting at an odd offset and aligning relative to that slice
///   desynchronises after the first string, and every field after it reads
///   garbage that still parses. [`Qari::asas`] carries the slice's absolute
///   start so [`Qari::hadhi`] rounds the real position.
/// * **Nothing wraps.** Every advance is a checked add and every take is a
///   `get`, so a hostile length is a refusal rather than a panic in a process
///   that has no business dying because a bundle was edited.
#[derive(Debug)]
pub struct Qari<'a> {
    sigha: &'static str,
    bayt: &'a [u8],
    asas: usize,
    mawqi: usize,
    nihaya: Nihaya,
}

impl<'a> Qari<'a> {
    /// A cursor at the start of `bayt`, refusing in the name of format `sigha`.
    ///
    /// Big-endian, because every container in this module starts that way: a
    /// bundle header is big-endian throughout and a `SerializedFile` header is
    /// big-endian up to the byte that says what the rest of it is.
    #[must_use]
    pub const fn jadeed(sigha: &'static str, bayt: &'a [u8]) -> Self {
        Self {
            sigha,
            bayt,
            asas: 0,
            mawqi: 0,
            nihaya: Nihaya::Kabeer,
        }
    }

    /// A cursor over a slice that begins at absolute offset `asas` in its file.
    ///
    /// The only way to read an object's fields correctly: alignment padding is
    /// computed from the absolute position, so a slice whose start is not a
    /// multiple of four still aligns where Unity aligned.
    #[must_use]
    pub const fn jadeed_bi_asas(
        sigha: &'static str,
        bayt: &'a [u8],
        asas: usize,
        nihaya: Nihaya,
    ) -> Self {
        Self {
            sigha,
            bayt,
            asas,
            mawqi: 0,
            nihaya,
        }
    }

    /// Reads the rest of this cursor in the given byte order.
    ///
    /// A `SerializedFile` header is big-endian and declares, in its last byte,
    /// what the rest of the file is. This is that switch.
    pub const fn ittajih(&mut self, nihaya: Nihaya) {
        self.nihaya = nihaya;
    }

    /// The order this cursor is currently reading in.
    #[must_use]
    pub const fn nihaya(&self) -> Nihaya {
        self.nihaya
    }

    /// The format this cursor refuses in the name of.
    #[must_use]
    pub const fn sigha(&self) -> &'static str {
        self.sigha
    }

    /// How far in the cursor is, relative to the slice.
    #[must_use]
    pub const fn mawqi(&self) -> usize {
        self.mawqi
    }

    /// The whole slice, from its own byte zero.
    ///
    /// What a reader needs when a structure carries an offset relative to the
    /// container rather than to the cursor — which is every object offset in a
    /// `SerializedFile` and every node offset in a bundle directory.
    #[must_use]
    pub const fn kull(&self) -> &'a [u8] {
        self.bayt
    }

    /// How many bytes are left.
    #[must_use]
    pub fn baqi(&self) -> u64 {
        tul_u64(self.bayt.len().saturating_sub(self.mawqi))
    }

    /// Moves the cursor to an offset in the slice, refusing one outside it.
    ///
    /// # Errors
    ///
    /// [`KhataQira::HaqlTalif`] naming `haql`. An offset a file declares that
    /// points outside that file is a corrupt field, not a short read, and the
    /// distinction matters to whoever reads the message.
    pub fn iqfiz(&mut self, haql: &'static str, izaha: u64) -> Result<(), KhataQira> {
        let tul = self.bayt.len();
        let mawqi = hajm_usize(izaha)
            .filter(|mawqi| *mawqi <= tul)
            .ok_or_else(|| KhataQira::HaqlTalif {
                sigha: self.sigha,
                haql,
                sabab: format!("offset {izaha} is outside a {tul}-byte container"),
            })?;
        self.mawqi = mawqi;
        Ok(())
    }

    /// Skips `adad` bytes without looking at them.
    ///
    /// Bounded even though the bytes are not read: a reserved run this build
    /// does not interpret is a run whose *presence* the format still promises,
    /// and skipping past the end silently would move a later field's failure
    /// somewhere it cannot be diagnosed.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn takhatta(&mut self, haql: &'static str, adad: u64) -> Result<(), KhataQira> {
        let _ = self.iqra_bayt(haql, adad)?;
        Ok(())
    }

    /// Rounds the cursor up to the next multiple of `wahda` in absolute terms.
    ///
    /// Unity calls this `AlignStream`, and it is the single most common source
    /// of a desynchronised read: the padding after a string, after a boolean and
    /// after any field whose type tree node carries the align flag is not
    /// described anywhere in the data, only implied by the field's own metadata.
    ///
    /// Aligning past the end of the slice is not an error — Unity's own writer
    /// emits the last field of an object without its trailing padding when the
    /// object ends there — so this clamps rather than refusing.
    pub fn hadhi(&mut self, wahda: usize) {
        if wahda == 0 {
            return;
        }
        let mutlaq = self.asas.saturating_add(self.mawqi);
        let baqi = mutlaq % wahda;
        if baqi != 0 {
            let taqaddum = wahda.saturating_sub(baqi);
            self.mawqi = self.mawqi.saturating_add(taqaddum).min(self.bayt.len());
        }
    }

    /// Takes `adad` bytes.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql` when they are not there.
    pub fn iqra_bayt(&mut self, haql: &'static str, adad: u64) -> Result<&'a [u8], KhataQira> {
        let bayt = self.bayt;
        let mutah = bayt.len().saturating_sub(self.mawqi);
        let mada =
            hajm_usize(adad).ok_or_else(|| KhataQira::qaseer(self.sigha, haql, mutah, adad))?;
        let nihaya = self
            .mawqi
            .checked_add(mada)
            .ok_or_else(|| KhataQira::qaseer(self.sigha, haql, mutah, adad))?;
        let khana = bayt
            .get(self.mawqi..nihaya)
            .ok_or_else(|| KhataQira::qaseer(self.sigha, haql, mutah, adad))?;
        self.mawqi = nihaya;
        Ok(khana)
    }

    /// Takes a fixed-length run.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_masfufa<const N: usize>(
        &mut self,
        haql: &'static str,
    ) -> Result<[u8; N], KhataQira> {
        let mutah = self.bayt.len().saturating_sub(self.mawqi);
        let khana = self.iqra_bayt(haql, tul_u64(N))?;
        khana
            .try_into()
            .map_err(|_| KhataQira::qaseer(self.sigha, haql, mutah, tul_u64(N)))
    }

    /// Takes one byte.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_u8(&mut self, haql: &'static str) -> Result<u8, KhataQira> {
        let khana: [u8; 1] = self.iqra_masfufa(haql)?;
        Ok(khana.first().copied().unwrap_or(0))
    }

    /// Takes one signed byte.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_i8(&mut self, haql: &'static str) -> Result<i8, KhataQira> {
        Ok(i8::from_le_bytes([self.iqra_u8(haql)?]))
    }

    /// Takes a boolean, which Unity serializes as one byte.
    ///
    /// Any non-zero byte is true. Unity writes `0` and `1` and nothing else, but
    /// a `bool` field inside a `MonoBehaviour` whose C# type was changed from an
    /// enum can hold anything, and refusing on it would refuse an object the
    /// engine itself loads.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_bool(&mut self, haql: &'static str) -> Result<bool, KhataQira> {
        Ok(self.iqra_u8(haql)? != 0)
    }

    /// Takes two bytes, unsigned, in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_u16(&mut self, haql: &'static str) -> Result<u16, KhataQira> {
        let khana = self.iqra_masfufa::<2>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => u16::from_le_bytes(khana),
            Nihaya::Kabeer => u16::from_be_bytes(khana),
        })
    }

    /// Takes two bytes, signed, in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_i16(&mut self, haql: &'static str) -> Result<i16, KhataQira> {
        let khana = self.iqra_masfufa::<2>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => i16::from_le_bytes(khana),
            Nihaya::Kabeer => i16::from_be_bytes(khana),
        })
    }

    /// Takes four bytes, unsigned, in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_u32(&mut self, haql: &'static str) -> Result<u32, KhataQira> {
        let khana = self.iqra_masfufa::<4>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => u32::from_le_bytes(khana),
            Nihaya::Kabeer => u32::from_be_bytes(khana),
        })
    }

    /// Takes four bytes, signed, in this cursor's order.
    ///
    /// Signed because the format says signed: every count in a `SerializedFile`
    /// is an `int`, and reading them unsigned would turn a corrupt `-1` into
    /// four billion and hand that to a `with_capacity`.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_i32(&mut self, haql: &'static str) -> Result<i32, KhataQira> {
        let khana = self.iqra_masfufa::<4>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => i32::from_le_bytes(khana),
            Nihaya::Kabeer => i32::from_be_bytes(khana),
        })
    }

    /// Takes eight bytes, unsigned, in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_u64(&mut self, haql: &'static str) -> Result<u64, KhataQira> {
        let khana = self.iqra_masfufa::<8>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => u64::from_le_bytes(khana),
            Nihaya::Kabeer => u64::from_be_bytes(khana),
        })
    }

    /// Takes eight bytes, signed, in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_i64(&mut self, haql: &'static str) -> Result<i64, KhataQira> {
        let khana = self.iqra_masfufa::<8>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => i64::from_le_bytes(khana),
            Nihaya::Kabeer => i64::from_be_bytes(khana),
        })
    }

    /// Takes a single-precision float in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_f32(&mut self, haql: &'static str) -> Result<f32, KhataQira> {
        let khana = self.iqra_masfufa::<4>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => f32::from_le_bytes(khana),
            Nihaya::Kabeer => f32::from_be_bytes(khana),
        })
    }

    /// Takes a double-precision float in this cursor's order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::MalafQaseer`] naming `haql`.
    pub fn iqra_f64(&mut self, haql: &'static str) -> Result<f64, KhataQira> {
        let khana = self.iqra_masfufa::<8>(haql)?;
        Ok(match self.nihaya {
            Nihaya::Sagheer => f64::from_le_bytes(khana),
            Nihaya::Kabeer => f64::from_be_bytes(khana),
        })
    }

    /// Takes a NUL-terminated name.
    ///
    /// Bundle headers, directory paths and the pre-format-12 type tree all store
    /// names this way. The scan is bounded by [`AQSA_TUL_ISM`] so a missing
    /// terminator refuses after four kilobytes instead of walking a two-gigabyte
    /// mapping to its end.
    ///
    /// # Errors
    ///
    /// [`KhataQira::HaqlTalif`] when no terminator appears within the ceiling,
    /// and [`KhataQira::NassGhayrSalih`] when the bytes are not UTF-8.
    pub fn iqra_nass_munahi(&mut self, haql: &'static str) -> Result<String, KhataQira> {
        let baqiya = self.bayt.get(self.mawqi..).unwrap_or(&[]);
        let mada = baqiya.len().min(AQSA_TUL_ISM);
        let nitaq = baqiya.get(..mada).unwrap_or(&[]);
        let tul = nitaq
            .iter()
            .position(|bayt| *bayt == 0)
            .ok_or_else(|| KhataQira::HaqlTalif {
                sigha: self.sigha,
                haql,
                sabab: format!("no NUL terminator within {AQSA_TUL_ISM} bytes"),
            })?;
        let khana = nitaq.get(..tul).unwrap_or(&[]);
        let nass = self.ila_nass(haql, khana)?;
        self.mawqi = self.mawqi.saturating_add(tul).saturating_add(1);
        Ok(nass)
    }

    /// Takes a length-prefixed string and the padding that follows it.
    ///
    /// This is Unity's `ReadAlignedString`: a signed 32-bit length, that many
    /// UTF-8 bytes, then padding to the next multiple of four. The padding is
    /// consumed here rather than left to the caller because forgetting it is the
    /// single most common way a type-tree walk desynchronises, and a walk that
    /// desynchronises does not fail — it reads the next field's bytes as this
    /// field's and produces a string that looks like a string.
    ///
    /// A negative or oversized length is refused rather than clamped: it means
    /// the walk is already off, and clamping would hide that and carry on.
    ///
    /// # Errors
    ///
    /// [`KhataQira::HaqlTalif`] for a negative length, [`KhataQira::MalafQaseer`]
    /// when the bytes are not there, and [`KhataQira::NassGhayrSalih`] when they
    /// are not UTF-8.
    pub fn iqra_nass_muhadhah(&mut self, haql: &'static str) -> Result<String, KhataQira> {
        let tul = self.iqra_i32(haql)?;
        let mada = u32::try_from(tul).map_err(|_| KhataQira::HaqlTalif {
            sigha: self.sigha,
            haql,
            sabab: format!("negative string length {tul}"),
        })?;
        let khana = self.iqra_bayt(haql, u64::from(mada))?;
        let nass = self.ila_nass(haql, khana)?;
        self.hadhi(4);
        Ok(nass)
    }

    /// Decodes a run this cursor already took, naming the field on failure.
    ///
    /// # Errors
    ///
    /// [`KhataQira::NassGhayrSalih`] with the offset of the first bad byte,
    /// because "field `m_Name` is not valid UTF-8 at byte 7" is a thing a person
    /// can go and look at and "invalid UTF-8" is not.
    fn ila_nass(&self, haql: &'static str, khana: &[u8]) -> Result<String, KhataQira> {
        std::str::from_utf8(khana)
            .map(str::to_owned)
            .map_err(|khata| KhataQira::NassGhayrSalih {
                sigha: self.sigha,
                haql,
                mawqi: khata.valid_up_to(),
            })
    }
}

// ---------------------------------------------------------------------------
// The bundle
// ---------------------------------------------------------------------------

/// The name a bundle refusal is issued under.
pub const SIGHA_HUZMA: &str = "UnityFS bundle";

/// The four signatures Unity has shipped for a bundle.
///
/// Held as an enum rather than a string because the signature decides the whole
/// header layout: `UnityFS` and `UnityArchive` carry a block table, `UnityRaw`
/// and `UnityWeb` carry a level table, and the two shapes share only the first
/// three fields.
///
/// Each variant is the signature with its `Unity` prefix dropped, which the type
/// name already supplies; [`TawqiHuzma::ism`] spells the on-disk form back out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TawqiHuzma {
    /// The current one, since Unity 5.3. A block table and a directory.
    Fs,
    /// The same layout under a different name, written by some 2017-era builds.
    Archive,
    /// Pre-5.3, uncompressed payload. Read.
    Raw,
    /// Pre-5.3, LZMA payload. Refused — see this module's header.
    Web,
}

impl TawqiHuzma {
    /// The signature this bundle declared, or `None` for bytes that are not a
    /// bundle at all.
    #[must_use]
    pub fn min_bayt(bayt: &[u8]) -> Option<Self> {
        for (tawqi, naw) in [
            (&b"UnityFS\0"[..], Self::Fs),
            (&b"UnityArchive\0"[..], Self::Archive),
            (&b"UnityRaw\0"[..], Self::Raw),
            (&b"UnityWeb\0"[..], Self::Web),
        ] {
            if bayt.get(..tawqi.len()) == Some(tawqi) {
                return Some(naw);
            }
        }
        None
    }

    /// The signature as it appears in the file, for a refusal a person reads.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Fs => "UnityFS",
            Self::Archive => "UnityArchive",
            Self::Raw => "UnityRaw",
            Self::Web => "UnityWeb",
        }
    }

    /// Whether this signature uses the block-table header.
    #[must_use]
    pub const fn hadeetha(self) -> bool {
        matches!(self, Self::Fs | Self::Archive)
    }
}

/// How a block or a blocks-info table was compressed.
///
/// The low six bits of the flags word, and the same encoding is used per block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamatDaght {
    /// Stored. The payload is the block.
    Bila,
    /// LZMA, `lzma_alone` framing. Refused — see this module's header.
    Lzma,
    /// LZ4 block format, uncompressed size known from the block table.
    Lz4,
    /// The same format at a higher compression level. Identical to decode.
    Lz4hc,
    /// A mode this build does not know.
    ///
    /// Unity reserves `4` for LZHAM and has never shipped it in a released
    /// player. A bundle declaring one of these is refused by number rather than
    /// decoded as LZ4 on the assumption that the enum is contiguous.
    Majhul(u8),
}

impl NamatDaght {
    /// The mode the low six bits of a flags word name.
    #[must_use]
    pub fn min_rayat(rayat: u32) -> Self {
        let masked = rayat & 0x3F;
        match masked {
            0 => Self::Bila,
            1 => Self::Lzma,
            2 => Self::Lz4,
            3 => Self::Lz4hc,
            // `try_from` rather than a cast: the mask already guarantees the
            // value fits, and saying so with a conversion that cannot silently
            // truncate keeps the guarantee checkable rather than commented.
            akhar => Self::Majhul(u8::try_from(akhar).unwrap_or(0)),
        }
    }

    /// The mode's name, for a refusal and for a corrupt-block message.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Bila => "uncompressed",
            Self::Lzma => "LZMA",
            Self::Lz4 => "LZ4",
            Self::Lz4hc => "LZ4HC",
            Self::Majhul(_) => "an unknown compression mode",
        }
    }
}

/// The bundle header's flags word, read as the bits it actually is.
///
/// Wrapped rather than passed around as a bare `u32` because two of these bits
/// change where the blocks-info table *is*, not merely how it is encoded, and a
/// caller testing the wrong one reads a table from the middle of the payload and
/// gets a block count in the millions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RayatHuzma(u32);

impl RayatHuzma {
    /// Wraps the word the header declared.
    #[must_use]
    pub const fn jadeeda(qeema: u32) -> Self {
        Self(qeema)
    }

    /// The raw word, for a diagnostic that has to quote it.
    #[must_use]
    pub const fn qeema(self) -> u32 {
        self.0
    }

    /// How the blocks-info table itself is compressed.
    #[must_use]
    pub fn namat_daght(self) -> NamatDaght {
        NamatDaght::min_rayat(self.0)
    }

    /// Bit 6 — the block table and the directory are one structure.
    ///
    /// True for every bundle this decade. Recorded because a bundle without it
    /// would put the directory somewhere this reader does not look, and that is
    /// worth refusing over rather than reading an empty directory from.
    #[must_use]
    pub const fn dalil_mudmaj(self) -> bool {
        self.0 & 0x40 != 0
    }

    /// Bit 7 — the blocks-info table lives at the very end of the file.
    ///
    /// A real and common layout, not an exotic one: Unity writes it this way for
    /// bundles built for streaming, so the player can begin decompressing block
    /// zero while the tail is still arriving. A reader that assumes the table
    /// follows the header reads the first compressed block as a table.
    #[must_use]
    pub const fn dalil_fi_alnihaya(self) -> bool {
        self.0 & 0x80 != 0
    }

    /// Bit 8 — the old web-plugin compatibility layout.
    #[must_use]
    pub const fn tawafuq_qadeem(self) -> bool {
        self.0 & 0x100 != 0
    }

    /// Bit 9 — sixteen bytes of alignment padding precede the first block.
    #[must_use]
    pub const fn hashw_bidaya(self) -> bool {
        self.0 & 0x200 != 0
    }

    /// Bit 10 — Unity's own `AssetBundle` encryption is in use.
    ///
    /// **The strongest evidence of encryption this product can have**, because
    /// the container says so itself. When this is set the bundle is refused with
    /// [`SababRafd::Mushaffar`][crate::rafd::SababRafd::Mushaffar] without any
    /// entropy heuristic being consulted: a declared cipher is a fact, and the
    /// entropy test in [`crate::unity`] exists only for the bundles that do not
    /// declare one.
    #[must_use]
    pub const fn mushaffar(self) -> bool {
        self.0 & 0x400 != 0
    }
}

/// One compressed block of a bundle's payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KutlaHuzma {
    /// How many bytes this block becomes.
    pub hajm_maftuh: u32,
    /// How many bytes it occupies in the file.
    pub hajm_madghut: u32,
    /// The block's own flags, whose low six bits are its compression mode.
    pub rayat: u16,
}

impl KutlaHuzma {
    /// How this block is compressed.
    ///
    /// Per block, not per bundle: Unity compresses the `SerializedFile` blocks
    /// with LZ4 and leaves an already-compressed texture block stored, in the
    /// same bundle, and a reader that took the header's mode for all of them
    /// would hand LZ4 an uncompressed block.
    #[must_use]
    pub fn namat(self) -> NamatDaght {
        NamatDaght::min_rayat(u32::from(self.rayat))
    }
}

/// One file inside a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UqdatHuzma {
    /// Where its bytes start in the decompressed payload.
    pub izaha: i64,
    /// How many bytes they are.
    pub hajm: i64,
    /// Unity's own flags for the entry. Bit 2 marks a `SerializedFile`.
    pub rayat: u32,
    /// The name Unity's file API resolves for it, such as
    /// `CAB-4f1e…` or `sharedassets0.assets.resS`.
    pub masar: String,
}

impl UqdatHuzma {
    /// Whether Unity marked this entry as a `SerializedFile`.
    ///
    /// A hint and not a guarantee, which is why [`Huzma`]'s caller checks the
    /// entry's own header too: bundles exist in the wild whose flags are zero on
    /// every node, written by tools that never set them, and refusing those
    /// would refuse a container this reader can read perfectly well.
    #[must_use]
    pub const fn mulsal(&self) -> bool {
        self.rayat & 0x4 != 0
    }
}

/// A bundle, with its payload decompressed and its directory read.
///
/// The payload is materialised as one `Vec` rather than decompressed lazily per
/// node, because a node's byte range routinely spans a block boundary — Unity's
/// block splitter cuts at 128 KiB without regard to file boundaries — so a lazy
/// reader would have to decompress a run of blocks per node anyway and would
/// decompress the shared ones repeatedly.
#[derive(Debug)]
pub struct Huzma {
    tawqi: TawqiHuzma,
    isdar: u32,
    isdar_muharrik_ala: String,
    isdar_muharrik: String,
    rayat: RayatHuzma,
    kutal: Vec<KutlaHuzma>,
    uqad: Vec<UqdatHuzma>,
    bayanat: Vec<u8>,
}

impl Huzma {
    /// Reads a bundle from bytes.
    ///
    /// # Errors
    ///
    /// [`KhataQira::SighaMajhula`] for bytes that are not a bundle and for the
    /// two payload encodings this build refuses by name — LZMA blocks and the
    /// `UnityWeb` signature. [`KhataQira::IsdarGhayrMadum`] for a format version
    /// outside the table in this module's header.
    /// [`KhataQira::TajawuzHadd`] for a declared block, node or payload size
    /// above a ceiling. [`KhataQira::MalafQaseer`] and
    /// [`KhataQira::HaqlTalif`] for a truncated or self-contradicting header.
    /// [`KhataQira::DaghtTalif`] when a block does not decompress to its
    /// declared size, which is what a half-downloaded bundle looks like.
    pub fn iqra(bayt: &[u8]) -> Result<Self, KhataQira> {
        let tawqi = TawqiHuzma::min_bayt(bayt).ok_or_else(|| KhataQira::SighaMajhula {
            wujid: "a file with no Unity bundle signature".to_owned(),
        })?;
        let mut qari = Qari::jadeed(SIGHA_HUZMA, bayt);
        let _ = qari.iqra_nass_munahi("signature")?;
        let isdar = qari.iqra_u32("format version")?;
        let isdar_muharrik_ala = qari.iqra_nass_munahi("unity version")?;
        let isdar_muharrik = qari.iqra_nass_munahi("unity revision")?;

        if tawqi.hadeetha() {
            Self::iqra_hadeetha(qari, tawqi, isdar, isdar_muharrik_ala, isdar_muharrik)
        } else {
            Self::iqra_qadeema(qari, tawqi, isdar, isdar_muharrik_ala, isdar_muharrik)
        }
    }

    /// The signature this bundle declared.
    #[must_use]
    pub const fn tawqi(&self) -> TawqiHuzma {
        self.tawqi
    }

    /// The bundle's format version.
    #[must_use]
    pub const fn isdar(&self) -> u32 {
        self.isdar
    }

    /// The engine generation string, such as `5.x.x`.
    #[must_use]
    pub fn isdar_muharrik_ala(&self) -> &str {
        &self.isdar_muharrik_ala
    }

    /// The exact engine build, such as `2021.3.16f1`.
    ///
    /// Carried because it is the one piece of provenance a refusal report can
    /// give a contributor that lets them reproduce the case: "this build strips
    /// the type tree" is far more actionable beside the engine version that
    /// wrote the file.
    #[must_use]
    pub fn isdar_muharrik(&self) -> &str {
        &self.isdar_muharrik
    }

    /// The header's flags.
    #[must_use]
    pub const fn rayat(&self) -> RayatHuzma {
        self.rayat
    }

    /// The block table, in file order.
    #[must_use]
    pub fn kutal(&self) -> &[KutlaHuzma] {
        &self.kutal
    }

    /// Every file inside.
    #[must_use]
    pub fn uqad(&self) -> &[UqdatHuzma] {
        &self.uqad
    }

    /// One entry's bytes, or `None` when its declared range is not inside the
    /// payload.
    ///
    /// `None` rather than a refusal because one bad node in a bundle of twelve
    /// should cost that node and not the bundle: the caller records the node's
    /// name and reads the next one.
    #[must_use]
    pub fn mihtawa(&self, uqda: &UqdatHuzma) -> Option<&[u8]> {
        let bidaya = hajm_usize(u64::try_from(uqda.izaha).ok()?)?;
        let mada = hajm_usize(u64::try_from(uqda.hajm).ok()?)?;
        let nihaya = bidaya.checked_add(mada)?;
        self.bayanat.get(bidaya..nihaya)
    }

    /// The whole decompressed payload.
    #[must_use]
    pub fn bayanat(&self) -> &[u8] {
        &self.bayanat
    }

    /// The `UnityFS` and `UnityArchive` layout: a block table and a directory.
    fn iqra_hadeetha(
        mut qari: Qari<'_>,
        tawqi: TawqiHuzma,
        isdar: u32,
        isdar_muharrik_ala: String,
        isdar_muharrik: String,
    ) -> Result<Self, KhataQira> {
        if !(6..=8).contains(&isdar) {
            return Err(KhataQira::IsdarGhayrMadum {
                sigha: SIGHA_HUZMA,
                wujid: isdar.to_string(),
                madum: "6, 7 and 8",
            });
        }

        let hajm_malaf = qari.iqra_i64("bundle size")?;
        let hajm_madghut = qari.iqra_u32("compressed blocks-info size")?;
        let hajm_maftuh = qari.iqra_u32("uncompressed blocks-info size")?;
        let rayat = RayatHuzma::jadeeda(qari.iqra_u32("archive flags")?);

        if rayat.mushaffar() {
            return Err(KhataQira::Mushaffar {
                wasf: format!(
                    "Unity AssetBundle encryption, declared by flag 0x400 in a format-{isdar} \
                     bundle written by {isdar_muharrik}"
                ),
            });
        }
        if u64::try_from(hajm_malaf).unwrap_or(0) > tul_u64(qari.kull().len()) {
            return Err(KhataQira::HaqlTalif {
                sigha: SIGHA_HUZMA,
                haql: "bundle size",
                sabab: format!(
                    "declares {hajm_malaf} bytes, the file holds {}",
                    qari.kull().len()
                ),
            });
        }

        // Format 7 introduced sixteen-byte alignment before the blocks-info
        // table. Reading a format-7 bundle without it lands in the middle of the
        // compressed table and produces a block count that passes no sanity
        // check, which is at least loud — but on a format-6 bundle the same
        // alignment skips into the first block, and *that* can parse.
        if isdar >= 7 {
            qari.hadhi(16);
        }

        let madghut = if rayat.dalil_fi_alnihaya() {
            let kull = qari.kull();
            // The table is the last `hajm_madghut` bytes of the file. A declared
            // size larger than the file is refused here rather than saturated to
            // zero, because saturating would hand the decompressor the whole
            // bundle and blame the failure on the payload.
            if u64::from(hajm_madghut) > tul_u64(kull.len()) {
                return Err(KhataQira::qaseer(
                    SIGHA_HUZMA,
                    "trailing blocks-info table",
                    kull.len(),
                    u64::from(hajm_madghut),
                ));
            }
            let bidaya = tul_u64(kull.len()).saturating_sub(u64::from(hajm_madghut));
            let mawqi = hajm_usize(bidaya).ok_or_else(|| KhataQira::TajawuzHadd {
                hadd: "blocks-info offset",
                qeema: bidaya,
                saqf: tul_u64(usize::MAX),
            })?;
            kull.get(mawqi..).ok_or_else(|| {
                KhataQira::qaseer(
                    SIGHA_HUZMA,
                    "trailing blocks-info table",
                    kull.len(),
                    u64::from(hajm_madghut),
                )
            })?
        } else {
            qari.iqra_bayt("blocks-info table", u64::from(hajm_madghut))?
        };

        let maftuh = fukk_daght(
            rayat.namat_daght(),
            0,
            madghut,
            u64::from(hajm_maftuh),
            AQSA_DALIL,
        )?;

        let (kutal, uqad) = Self::iqra_dalil(&maftuh)?;

        // Bit 9 asks for sixteen-byte padding before the first block. It is
        // measured from the cursor's position after the inline blocks-info
        // table, which is why it is applied here rather than at the top: with
        // the table at the end of the file the cursor never moved past the
        // header and the padding is relative to that.
        if rayat.hashw_bidaya() {
            qari.hadhi(16);
        }

        let bayanat = Self::fukk_kutal(&mut qari, &kutal)?;
        Ok(Self {
            tawqi,
            isdar,
            isdar_muharrik_ala,
            isdar_muharrik,
            rayat,
            kutal,
            uqad,
            bayanat,
        })
    }

    /// The block table and the directory, out of the decompressed blocks-info.
    fn iqra_dalil(maftuh: &[u8]) -> Result<(Vec<KutlaHuzma>, Vec<UqdatHuzma>), KhataQira> {
        let mut dalil = Qari::jadeed(SIGHA_HUZMA, maftuh);
        // Sixteen bytes of hash over the uncompressed payload. Read past rather
        // than verified: Unity's own player does not check it, bundles exist
        // whose writers left it zero, and refusing on a hash mismatch would
        // refuse containers the engine loads.
        dalil.takhatta("uncompressed-data hash", 16)?;

        let adad_kutal = tahaqquq_adad(
            SIGHA_HUZMA,
            "block count",
            dalil.iqra_i32("block count")?,
            AQSA_KUTAL,
        )?;
        let mut kutal = Vec::with_capacity(hajz_mahdud(adad_kutal));
        for _ in 0..adad_kutal {
            kutal.push(KutlaHuzma {
                hajm_maftuh: dalil.iqra_u32("block uncompressed size")?,
                hajm_madghut: dalil.iqra_u32("block compressed size")?,
                rayat: dalil.iqra_u16("block flags")?,
            });
        }

        let adad_uqad = tahaqquq_adad(
            SIGHA_HUZMA,
            "node count",
            dalil.iqra_i32("node count")?,
            AQSA_UQAD,
        )?;
        let mut uqad = Vec::with_capacity(hajz_mahdud(adad_uqad));
        for _ in 0..adad_uqad {
            uqad.push(UqdatHuzma {
                izaha: dalil.iqra_i64("node offset")?,
                hajm: dalil.iqra_i64("node size")?,
                rayat: dalil.iqra_u32("node flags")?,
                masar: dalil.iqra_nass_munahi("node path")?,
            });
        }
        Ok((kutal, uqad))
    }

    /// Decompresses every block into one contiguous payload.
    fn fukk_kutal(qari: &mut Qari<'_>, kutal: &[KutlaHuzma]) -> Result<Vec<u8>, KhataQira> {
        let mut kulli: u64 = 0;
        for kutla in kutal {
            kulli = kulli.saturating_add(u64::from(kutla.hajm_maftuh));
        }
        if kulli > AQSA_HAJM_MAFTUH {
            return Err(KhataQira::TajawuzHadd {
                hadd: "decompressed bundle payload",
                qeema: kulli,
                saqf: AQSA_HAJM_MAFTUH,
            });
        }

        let mut bayanat = Vec::with_capacity(hajm_usize(kulli).unwrap_or(0));
        for (fahras, kutla) in kutal.iter().enumerate() {
            let raqm = u32::try_from(fahras).unwrap_or(u32::MAX);
            let madghut = qari.iqra_bayt("block payload", u64::from(kutla.hajm_madghut))?;
            let maftuh = fukk_daght(
                kutla.namat(),
                raqm,
                madghut,
                u64::from(kutla.hajm_maftuh),
                u32::MAX,
            )?;
            bayanat.extend_from_slice(&maftuh);
        }
        Ok(bayanat)
    }

    /// The `UnityRaw` and `UnityWeb` layout: a level table and a flat directory.
    ///
    /// `UnityWeb`'s payload is LZMA and is refused here by name rather than
    /// half-read. `UnityRaw`'s is stored, so it is read in full — those bundles
    /// are old but they are still on sale, and a 2014 game whose text is
    /// perfectly readable should not be refused for the company it keeps.
    fn iqra_qadeema(
        mut qari: Qari<'_>,
        tawqi: TawqiHuzma,
        isdar: u32,
        isdar_muharrik_ala: String,
        isdar_muharrik: String,
    ) -> Result<Self, KhataQira> {
        if !(1..=5).contains(&isdar) {
            return Err(KhataQira::IsdarGhayrMadum {
                sigha: SIGHA_HUZMA,
                wujid: isdar.to_string(),
                madum: "1 through 5",
            });
        }
        if tawqi == TawqiHuzma::Web {
            return Err(KhataQira::SighaMajhula {
                wujid: "a UnityWeb bundle, whose payload is LZMA".to_owned(),
            });
        }

        if isdar >= 4 {
            qari.takhatta("bundle hash", 16)?;
            let _ = qari.iqra_u32("bundle crc")?;
        }
        let _ = qari.iqra_u32("minimum streamed bytes")?;
        let hajm_tarwisa = qari.iqra_u32("header size")?;
        let _ = qari.iqra_u32("levels before streaming")?;
        let adad_mustawayat = tahaqquq_adad(
            SIGHA_HUZMA,
            "level count",
            qari.iqra_i32("level count")?,
            AQSA_KUTAL,
        )?;

        let mut kutal = Vec::with_capacity(hajz_mahdud(adad_mustawayat));
        for _ in 0..adad_mustawayat {
            kutal.push(KutlaHuzma {
                hajm_madghut: qari.iqra_u32("level compressed size")?,
                hajm_maftuh: qari.iqra_u32("level uncompressed size")?,
                rayat: 0,
            });
        }
        if isdar >= 2 {
            let _ = qari.iqra_u32("complete file size")?;
        }
        if isdar >= 3 {
            let _ = qari.iqra_u32("file-info header size")?;
        }

        qari.iqfiz("header size", u64::from(hajm_tarwisa))?;
        let baqi = qari.baqi();
        let bayanat = qari.iqra_bayt("bundle payload", baqi)?.to_vec();
        if tul_u64(bayanat.len()) > AQSA_HAJM_MAFTUH {
            return Err(KhataQira::TajawuzHadd {
                hadd: "decompressed bundle payload",
                qeema: tul_u64(bayanat.len()),
                saqf: AQSA_HAJM_MAFTUH,
            });
        }

        // The directory sits at the front of the payload and its node offsets
        // are relative to the payload's own start — which is why the whole
        // payload is kept and the offsets are not rebased.
        let mut dalil = Qari::jadeed(SIGHA_HUZMA, &bayanat);
        let adad_uqad = tahaqquq_adad(
            SIGHA_HUZMA,
            "node count",
            dalil.iqra_i32("node count")?,
            AQSA_UQAD,
        )?;
        let mut uqad = Vec::with_capacity(hajz_mahdud(adad_uqad));
        for _ in 0..adad_uqad {
            let masar = dalil.iqra_nass_munahi("node path")?;
            let izaha = dalil.iqra_u32("node offset")?;
            let hajm = dalil.iqra_u32("node size")?;
            uqad.push(UqdatHuzma {
                izaha: i64::from(izaha),
                hajm: i64::from(hajm),
                // The old directory has no flags word. Marking every entry as a
                // SerializedFile would be a claim the format does not make, so
                // the flags are zero and the caller identifies each entry by its
                // own header.
                rayat: 0,
                masar,
            });
        }

        Ok(Self {
            tawqi,
            isdar,
            isdar_muharrik_ala,
            isdar_muharrik,
            rayat: RayatHuzma::jadeeda(0),
            kutal,
            uqad,
            bayanat,
        })
    }
}

/// Decompresses one block, refusing every mode this build does not decode.
///
/// `mutawaqqa` is the size the block table declared, and it is trusted only far
/// enough to size the output buffer — the decoder is then required to fill
/// exactly that many bytes. A block that produces fewer is a truncated download
/// and is refused with [`KhataQira::DaghtTalif`], because the alternative is a
/// payload with a hole in it whose `SerializedFile` header still parses.
///
/// # Errors
///
/// [`KhataQira::SighaMajhula`] for LZMA and for a mode this build does not know,
/// [`KhataQira::TajawuzHadd`] when the declared size is above `saqf`, and
/// [`KhataQira::DaghtTalif`] when the decoder disagrees with the block table.
pub fn fukk_daght(
    namat: NamatDaght,
    fahras: u32,
    madghut: &[u8],
    mutawaqqa: u64,
    saqf: u32,
) -> Result<Vec<u8>, KhataQira> {
    if mutawaqqa > u64::from(saqf) {
        return Err(KhataQira::TajawuzHadd {
            hadd: "block uncompressed size",
            qeema: mutawaqqa,
            saqf: u64::from(saqf),
        });
    }
    if mutawaqqa > AQSA_HAJM_MAFTUH {
        return Err(KhataQira::TajawuzHadd {
            hadd: "block uncompressed size",
            qeema: mutawaqqa,
            saqf: AQSA_HAJM_MAFTUH,
        });
    }
    let mada = hajm_usize(mutawaqqa).ok_or_else(|| KhataQira::TajawuzHadd {
        hadd: "block uncompressed size",
        qeema: mutawaqqa,
        saqf: tul_u64(usize::MAX),
    })?;

    match namat {
        NamatDaght::Bila => {
            // A stored block whose two sizes disagree is a corrupt table, not a
            // short read: the payload is exactly as long as the file says it is.
            if tul_u64(madghut.len()) != mutawaqqa {
                return Err(KhataQira::DaghtTalif {
                    namat: namat.ism(),
                    fahras,
                    wujid: tul_u64(madghut.len()),
                    mutawaqqa,
                });
            }
            Ok(madghut.to_vec())
        },
        NamatDaght::Lz4 | NamatDaght::Lz4hc => {
            let mut maftuh = vec![0_u8; mada];
            let kutib = lz4_flex::block::decompress_into(madghut, &mut maftuh).map_err(|_| {
                KhataQira::DaghtTalif {
                    namat: namat.ism(),
                    fahras,
                    wujid: 0,
                    mutawaqqa,
                }
            })?;
            if tul_u64(kutib) != mutawaqqa {
                return Err(KhataQira::DaghtTalif {
                    namat: namat.ism(),
                    fahras,
                    wujid: tul_u64(kutib),
                    mutawaqqa,
                });
            }
            Ok(maftuh)
        },
        NamatDaght::Lzma => Err(KhataQira::SighaMajhula {
            wujid: "an LZMA-compressed Unity bundle block".to_owned(),
        }),
        NamatDaght::Majhul(raqm) => Err(KhataQira::SighaMajhula {
            wujid: format!("a Unity bundle block using compression mode {raqm}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// The built-in field names
// ---------------------------------------------------------------------------

/// The field names Unity keeps in the player binary instead of in the file.
///
/// The flat type tree stores each node's type name and field name as a 32-bit
/// offset. With the high bit clear the offset indexes the file's own string
/// buffer; **with the high bit set it indexes this table**, which the engine
/// ships and the file does not. Without it, `m_Name`, `m_Script`, `string`,
/// `vector` and every other common name reads as a number, and a reader that
/// printed the number as the name would produce a type tree whose fields cannot
/// be matched to anything.
///
/// The offsets are the cumulative byte positions of these entries in one
/// NUL-separated buffer, which is why the **order is load-bearing** and why the
/// table is stored as an ordered list with the offsets derived rather than as a
/// hand-written offset map: a transcription error in a hand-written offset would
/// silently rename one field to its neighbour, and a `MonoBehaviour` read with
/// `m_Enabled` where `m_GameObject` belongs desynchronises four bytes later.
///
/// An offset that does not land exactly on an entry start in this table does not
/// resolve. It is recorded as unknown and the object is refused — see
/// [`KhataQira::IsmMajhul`].
pub const ASMA_MUSHTARAKA: &[&str] = &[
    "AABB",
    "AnimationClip",
    "AnimationCurve",
    "AnimationState",
    "Array",
    "Base",
    "BitField",
    "bitset",
    "bool",
    "char",
    "ColorRGBA",
    "Component",
    "data",
    "deque",
    "double",
    "dynamic_array",
    "FastPropertyName",
    "first",
    "float",
    "Font",
    "GameObject",
    "Generic Mono",
    "GradientNEW",
    "GUID",
    "GUIStyle",
    "int",
    "list",
    "long long",
    "map",
    "Matrix4x4f",
    "MdFour",
    "MonoBehaviour",
    "MonoScript",
    "m_ByteSize",
    "m_Curve",
    "m_EditorClassIdentifier",
    "m_EditorHideFlags",
    "m_Enabled",
    "m_ExtensionPtr",
    "m_GameObject",
    "m_Index",
    "m_IsArray",
    "m_IsStatic",
    "m_MetaFlag",
    "m_Name",
    "m_ObjectHideFlags",
    "m_PrefabInternal",
    "m_PrefabParentObject",
    "m_Script",
    "m_StaticEditorFlags",
    "m_Type",
    "m_Version",
    "Object",
    "pair",
    "PPtr<Component>",
    "PPtr<GameObject>",
    "PPtr<Material>",
    "PPtr<MonoBehaviour>",
    "PPtr<MonoScript>",
    "PPtr<Object>",
    "PPtr<Prefab>",
    "PPtr<Sprite>",
    "PPtr<TextAsset>",
    "PPtr<Texture>",
    "PPtr<Texture2D>",
    "PPtr<Transform>",
    "Prefab",
    "Quaternionf",
    "Rectf",
    "RectInt",
    "RectOffset",
    "second",
    "set",
    "short",
    "size",
    "SInt16",
    "SInt32",
    "SInt64",
    "SInt8",
    "staticvector",
    "string",
    "TextAsset",
    "TextMesh",
    "Texture",
    "Texture2D",
    "Transform",
    "TypelessData",
    "UInt16",
    "UInt32",
    "UInt64",
    "UInt8",
    "unsigned int",
    "unsigned long long",
    "unsigned short",
    "vector",
    "Vector2f",
    "Vector3f",
    "Vector4f",
    "m_ScriptingClassIdentifier",
    "Gradient",
    "Type*",
    "int2_storage",
    "int3_storage",
    "BoundsInt",
    "m_CorrespondingSourceObject",
    "m_PrefabInstance",
    "m_PrefabAsset",
    "FileSize",
    "Hash128",
];

/// The offset-to-name map, derived once from [`ASMA_MUSHTARAKA`].
///
/// Built lazily rather than at compile time because the offsets are a running
/// sum over the entries' lengths and a `const` sum would have to be written out
/// by hand — which is the transcription error this whole arrangement exists to
/// avoid. One `OnceLock` per process, consulted once per type tree node.
fn kharitat_asma() -> &'static FxHashMap<u32, &'static str> {
    static KHAREETA: OnceLock<FxHashMap<u32, &'static str>> = OnceLock::new();
    KHAREETA.get_or_init(|| {
        let mut khareeta = FxHashMap::default();
        let mut izaha: u32 = 0;
        for ism in ASMA_MUSHTARAKA {
            let _ = khareeta.insert(izaha, *ism);
            // Every entry is followed by its NUL, exactly as it is in the
            // engine's buffer. Dropping the terminator would shift every offset
            // after the first entry by one and resolve every name to its
            // neighbour's tail.
            let tul = u32::try_from(ism.len()).unwrap_or(u32::MAX);
            izaha = izaha.saturating_add(tul).saturating_add(1);
        }
        khareeta
    })
}

/// Resolves one type tree name offset.
///
/// Returns `None` for an offset into the built-in table that this build does not
/// have an entry for, and for one into the file's own buffer that is not
/// NUL-terminated inside it. Both are "the name is unknown", and the caller
/// turns that into a refusal rather than a placeholder.
fn ism_min_izaha(izaha: u32, hajz: &[u8]) -> Option<String> {
    if izaha & 0x8000_0000 == 0 {
        let bidaya = usize::try_from(izaha).ok()?;
        let baqiya = hajz.get(bidaya..)?;
        let mada = baqiya.len().min(AQSA_TUL_ISM);
        let nitaq = baqiya.get(..mada)?;
        let tul = nitaq.iter().position(|bayt| *bayt == 0)?;
        return std::str::from_utf8(nitaq.get(..tul)?)
            .ok()
            .map(str::to_owned);
    }
    kharitat_asma()
        .get(&(izaha & 0x7FFF_FFFF))
        .map(|ism| (*ism).to_owned())
}

// ---------------------------------------------------------------------------
// The type tree
// ---------------------------------------------------------------------------

/// The name a `SerializedFile` refusal is issued under.
pub const SIGHA_MULSAL: &str = "Unity SerializedFile";

/// The lowest `SerializedFile` format version this build reads.
///
/// Five. Below it the metadata has no `unityVersion` string and no target
/// platform, and the object record's shape is not one this reader has ever seen
/// in a shipped game — Unity 3.4 and earlier. A file declaring less is refused
/// rather than read on the assumption that the missing fields simply are not
/// there.
pub const ISDAR_ADNA: u32 = 5;

/// The highest `SerializedFile` format version this build reads.
///
/// Twenty-three, which Unity 6 writes. A file declaring more is refused with
/// [`KhataQira::IsdarGhayrMadum`] rather than parsed on the assumption that
/// nothing moved: format 22 alone widened three header fields from 32 to 64 bits,
/// and a reader that guessed wrong there reads the type list from the middle of
/// the object data.
pub const ISDAR_AQSA: u32 = 23;

/// The meta-flag bit that says "pad to a four-byte boundary after this field".
///
/// **The single most common source of a desynchronised read.** The padding is
/// not in the data and is not implied by the field's size; it exists only
/// because this bit is set on the node. A walk that ignores it reads the next
/// field one to three bytes early, which does not fail — it produces a length
/// prefix that is off by a byte and then a string of the wrong length that is
/// still valid UTF-8 often enough to reach a translator.
pub const RAYA_MUHADHAHA: u32 = 0x4000;

/// One field of one type, in serialization order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UqdatShajara {
    /// The serializer version this field was written by.
    pub isdar: u16,
    /// How deep it sits. The root is zero and its fields are one.
    pub mustawa: u8,
    /// Whether this node is the `Array` marker of a vector.
    pub masfufa: bool,
    /// The field's type name, or `None` when the name did not resolve.
    pub naw: Option<String>,
    /// The field's own name, or `None` when the name did not resolve.
    pub ism: Option<String>,
    /// The field's fixed width, or `-1` for one whose width depends on its
    /// contents.
    pub hajm: i32,
    /// Unity's own index for the field.
    pub fahras: i32,
    /// The metadata bits, of which [`RAYA_MUHADHAHA`] is the one that matters.
    pub rayat_bayanat: u32,
}

impl UqdatShajara {
    /// Whether the reader must pad to four bytes after this field.
    #[must_use]
    pub const fn yuhadhi(&self) -> bool {
        self.rayat_bayanat & RAYA_MUHADHAHA != 0
    }

    /// The type name, or the empty string when it did not resolve.
    ///
    /// Only for diagnostics. The reader itself matches on
    /// [`UqdatShajara::naw`] so that an unresolved name cannot be mistaken for a
    /// type whose name happens to be empty.
    #[must_use]
    pub fn naw_aw_faragh(&self) -> &str {
        self.naw.as_deref().unwrap_or("")
    }
}

/// One type's fields, flattened.
///
/// Flat rather than a tree of owned children because that is how the file stores
/// it from format 12 onward and because the reader walks it as a cursor: a node
/// plus every following node at a deeper level *is* its subtree, and
/// [`ShajaratAnwa::nihayat_farr`] is the only operation that needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShajaratAnwa {
    uqad: Vec<UqdatShajara>,
    majhula: Option<(u32, u32)>,
}

impl ShajaratAnwa {
    /// The nodes, in serialization order.
    #[must_use]
    pub fn uqad(&self) -> &[UqdatShajara] {
        &self.uqad
    }

    /// Whether this tree has no nodes at all.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.uqad.is_empty()
    }

    /// The first node whose name did not resolve, as `(node index, offset)`.
    ///
    /// `Some` means every object of this type is refused. Both an unresolved
    /// *type* name and an unresolved *field* name count, for different reasons
    /// that reach the same place: the first makes the field's width unknown, so
    /// nothing after it can be read; the second makes the field's identity
    /// unknown, and [`MawqiNass::haql`][crate::jadwal::MawqiNass::haql] feeds
    /// [`NassId`][taarib_mustalahat::nass::NassId]. Two unnamed fields in one
    /// type would derive one identity for two different strings, and the second
    /// would silently overwrite the first's translation.
    #[must_use]
    pub const fn majhula(&self) -> Option<(u32, u32)> {
        self.majhula
    }

    /// Where the subtree rooted at `fahras` ends.
    ///
    /// The index of the first following node at the same level or shallower, or
    /// the node count when there is none. This is the whole of the tree walk's
    /// navigation: `fahras + 1 .. nihayat_farr(fahras)` is exactly the children
    /// of `fahras` and their descendants.
    #[must_use]
    pub fn nihayat_farr(&self, fahras: usize) -> usize {
        let Some(jidhr) = self.uqad.get(fahras) else {
            return self.uqad.len();
        };
        let mut nihaya = fahras.saturating_add(1);
        while let Some(uqda) = self.uqad.get(nihaya) {
            if uqda.mustawa <= jidhr.mustawa {
                break;
            }
            nihaya = nihaya.saturating_add(1);
        }
        nihaya
    }

    /// The index of a direct child of `fahras` named `ism`, if there is one.
    ///
    /// Direct children only, deliberately: a `MonoBehaviour` with a nested
    /// struct that also has an `m_Name` would otherwise resolve the wrong one,
    /// and the wrong one is a different string in a different place.
    #[must_use]
    pub fn ibn(&self, fahras: usize, ism: &str) -> Option<usize> {
        let mustawa = self.uqad.get(fahras)?.mustawa;
        let nihaya = self.nihayat_farr(fahras);
        let mut i = fahras.saturating_add(1);
        while i < nihaya {
            let uqda = self.uqad.get(i)?;
            if uqda.mustawa == mustawa.saturating_add(1) && uqda.ism.as_deref() == Some(ism) {
                return Some(i);
            }
            i = i.saturating_add(1);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// The serialized file
// ---------------------------------------------------------------------------

/// One type the file serializes objects of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NawMulsal {
    /// Unity's class id. `1` is `GameObject`, `4` `Transform`, `49`
    /// `TextAsset`, `114` `MonoBehaviour`, `115` `MonoScript`.
    pub sanf: i32,
    /// Whether the build stripped this type's definition.
    pub mahdhuf: bool,
    /// Which entry of the file's script-type list this type belongs to, or `-1`.
    pub fahras_script: i16,
    /// The `MonoScript`'s own hash, present only for `MonoBehaviour` types.
    pub bassmat_script: Option<[u8; 16]>,
    /// The type's structural hash, which Unity uses to detect layout drift.
    pub bassmat_naw: Option<[u8; 16]>,
    /// The field layout, when the build kept one.
    ///
    /// **`None` is the refusal that drives this whole module.** It means the
    /// build was made with `m_EnableTypeTree` off, and nothing in the file says
    /// what this type's bytes mean.
    pub shajara: Option<ShajaratAnwa>,
    /// The managed class name, for a reference type in format 21 and above.
    pub ism_sanf: Option<String>,
    /// Its namespace.
    pub nitaq_asma: Option<String>,
    /// Its assembly.
    pub ism_tajmee: Option<String>,
    /// The class ids this type's layout refers to.
    pub taabiyat: Vec<i32>,
}

impl NawMulsal {
    /// Whether this type's objects can be read at all.
    ///
    /// False when the tree is absent and false when it is present but names a
    /// field this build cannot resolve. Both produce
    /// [`SababRafd::BilaShajaratAnwa`][crate::rafd::SababRafd::BilaShajaratAnwa],
    /// because from the caller's side they are the same fact: the layout of
    /// these bytes is not in this file.
    #[must_use]
    pub fn maqru(&self) -> bool {
        self.shajara
            .as_ref()
            .is_some_and(|shajara| !shajara.khali() && shajara.majhula().is_none())
    }
}

/// One object's place in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadkhalKaain {
    /// The object's identity within this file. Stable across builds only when
    /// the build is deterministic, which is why it never becomes a
    /// [`NassId`][taarib_mustalahat::nass::NassId] on its own.
    pub hawiya_masar: i64,
    /// Where its bytes start, absolute in this file.
    pub bidaya: u64,
    /// How many bytes they are.
    pub hajm: u32,
    /// Which entry of the type list describes it.
    pub fahras_naw: i32,
    /// Unity's class id, resolved through the type list.
    pub sanf: i32,
    /// Whether the build stripped this object.
    pub mahdhuf: bool,
}

/// One entry of the script-type table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarjaScript {
    /// Which external file the script lives in; `0` is this one.
    pub fahras_malaf: i32,
    /// The script object's path id in that file.
    pub hawiya_masar: i64,
}

/// One file this one refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarjaKhariji {
    /// A field Unity writes empty in every shipped build, preserved because it
    /// is part of the record's width and skipping it by a hard-coded length
    /// would break the moment a build wrote something in it.
    pub masar_muaqqat: String,
    /// The referenced asset's GUID.
    pub muarrif: [u8; 16],
    /// Unity's own reference kind.
    pub naw: i32,
    /// The referenced file's path, such as `library/unity default resources`.
    pub masar: String,
}

/// A `SerializedFile`: a header, a type list, an object list, and object bytes.
///
/// Borrows its bytes rather than owning them. A loose `resources.assets` is
/// mapped and a bundled one is a range of an already-decompressed payload; in
/// both cases copying the object data would double the memory for no gain,
/// since every read from it is a slice.
#[derive(Debug)]
pub struct Mulsal<'a> {
    bayt: &'a [u8],
    isdar: u32,
    nihaya: Nihaya,
    hajm_bayanat_wasfiya: u64,
    hajm_malaf: u64,
    izahat_bayanat: u64,
    isdar_muharrik: String,
    minassa: i32,
    shajarat_anwa_muftaala: bool,
    anwa: Vec<NawMulsal>,
    anwa_marjiiya: Vec<NawMulsal>,
    kaainat: Vec<MadkhalKaain>,
    scripts: Vec<MarjaScript>,
    kharijiyat: Vec<MarjaKhariji>,
}

impl<'a> Mulsal<'a> {
    /// Whether these bytes plausibly begin a `SerializedFile`.
    ///
    /// A cheap sniff for dispatch, not a validation. `resources.assets`,
    /// `level0` and `globalgamemanagers` have no magic number at all — the
    /// header is four big-endian lengths — so the only available test is whether
    /// those four lengths are consistent with each other and with the file's
    /// real size. That is enough to tell a `SerializedFile` from a texture blob
    /// and not enough to promise it parses, which is why the caller still reads
    /// it through [`Mulsal::iqra`] and still handles the refusal.
    #[must_use]
    pub fn yabdu_mulsalan(bayt: &[u8]) -> bool {
        let mut qari = Qari::jadeed(SIGHA_MULSAL, bayt);
        let Ok(hajm_wasfiya) = qari.iqra_u32("metadata size") else {
            return false;
        };
        let Ok(hajm_malaf) = qari.iqra_u32("file size") else {
            return false;
        };
        let Ok(isdar) = qari.iqra_u32("format version") else {
            return false;
        };
        let Ok(izaha) = qari.iqra_u32("data offset") else {
            return false;
        };
        if !(ISDAR_ADNA..=ISDAR_AQSA).contains(&isdar) {
            return false;
        }
        // Format 22 moved the real lengths into a second header and leaves
        // zeroes in the first, so the size test is skipped for it and the
        // version test carries the decision on its own.
        if isdar >= 22 {
            return hajm_wasfiya == 0 || u64::from(hajm_wasfiya) <= tul_u64(bayt.len());
        }
        hajm_wasfiya > 0
            && u64::from(hajm_wasfiya) <= tul_u64(bayt.len())
            && u64::from(izaha) <= tul_u64(bayt.len())
            && u64::from(hajm_malaf) <= tul_u64(bayt.len())
    }

    /// Reads a `SerializedFile` from bytes.
    ///
    /// The object *data* is not read: only the header, the type list, the object
    /// table and the external references. Walking the objects is
    /// [`crate::unity::kaain`]'s job and it is done per object, so one
    /// unreadable object costs that object rather than the file.
    ///
    /// # Errors
    ///
    /// [`KhataQira::IsdarGhayrMadum`] outside format 5 – 23,
    /// [`KhataQira::TajawuzHadd`] for a declared type, object, node or external
    /// count above a ceiling, [`KhataQira::MalafQaseer`] for a truncated file,
    /// [`KhataQira::HaqlTalif`] for a header that contradicts itself, and
    /// [`KhataQira::NassGhayrSalih`] for a name that is not UTF-8.
    pub fn iqra(bayt: &'a [u8]) -> Result<Self, KhataQira> {
        let mut qari = Qari::jadeed(SIGHA_MULSAL, bayt);
        let mut hajm_bayanat_wasfiya = u64::from(qari.iqra_u32("metadata size")?);
        let mut hajm_malaf = u64::from(qari.iqra_u32("file size")?);
        let isdar = qari.iqra_u32("format version")?;
        let mut izahat_bayanat = u64::from(qari.iqra_u32("data offset")?);

        if !(ISDAR_ADNA..=ISDAR_AQSA).contains(&isdar) {
            return Err(KhataQira::IsdarGhayrMadum {
                sigha: SIGHA_MULSAL,
                wujid: isdar.to_string(),
                madum: "5 through 23",
            });
        }

        let nihaya = if isdar >= 9 {
            let bayt_nihaya = qari.iqra_u8("endianness")?;
            qari.takhatta("header reserved", 3)?;
            Nihaya::min_bayt(bayt_nihaya)
        } else {
            // Before format 9 the metadata sits at the *end* of the file and the
            // endianness byte is its first. The cursor moves there and stays:
            // everything after this point is read from that position, not from
            // byte 20.
            let bidaya = hajm_malaf
                .checked_sub(hajm_bayanat_wasfiya)
                .ok_or_else(|| KhataQira::HaqlTalif {
                    sigha: SIGHA_MULSAL,
                    haql: "metadata size",
                    sabab: format!(
                        "metadata is {hajm_bayanat_wasfiya} bytes in a {hajm_malaf}-byte file"
                    ),
                })?;
            qari.iqfiz("metadata offset", bidaya)?;
            Nihaya::min_bayt(qari.iqra_u8("endianness")?)
        };

        if isdar >= 22 {
            // Format 22 is where a Unity file stopped fitting in 32 bits. The
            // first header is left in place for old readers and its values are
            // meaningless; these are the real ones.
            hajm_bayanat_wasfiya = u64::from(qari.iqra_u32("metadata size (large)")?);
            let malaf = qari.iqra_i64("file size (large)")?;
            let izaha = qari.iqra_i64("data offset (large)")?;
            qari.takhatta("large header reserved", 8)?;
            hajm_malaf = u64::try_from(malaf).map_err(|_| KhataQira::HaqlTalif {
                sigha: SIGHA_MULSAL,
                haql: "file size (large)",
                sabab: format!("negative size {malaf}"),
            })?;
            izahat_bayanat = u64::try_from(izaha).map_err(|_| KhataQira::HaqlTalif {
                sigha: SIGHA_MULSAL,
                haql: "data offset (large)",
                sabab: format!("negative offset {izaha}"),
            })?;
        }

        if izahat_bayanat > tul_u64(bayt.len()) {
            return Err(KhataQira::HaqlTalif {
                sigha: SIGHA_MULSAL,
                haql: "data offset",
                sabab: format!(
                    "object data starts at {izahat_bayanat} in a {}-byte file",
                    bayt.len()
                ),
            });
        }

        qari.ittajih(nihaya);
        let isdar_muharrik = if isdar >= 7 {
            qari.iqra_nass_munahi("unity version")?
        } else {
            String::new()
        };
        let minassa = if isdar >= 8 {
            qari.iqra_i32("target platform")?
        } else {
            0
        };
        let shajarat_anwa_muftaala = if isdar >= 13 {
            qari.iqra_bool("type tree enabled")?
        } else {
            true
        };

        let adad_anwa = tahaqquq_adad(
            SIGHA_MULSAL,
            "type count",
            qari.iqra_i32("type count")?,
            AQSA_ANWA,
        )?;
        let mut anwa = Vec::with_capacity(hajz_mahdud(adad_anwa));
        for _ in 0..adad_anwa {
            anwa.push(iqra_naw(
                &mut qari,
                isdar,
                shajarat_anwa_muftaala,
                QaimatAnwa::Asliya,
            )?);
        }

        // Formats 7 through 13 carry a flag saying whether path ids are 64-bit.
        // Reading the flag and then reading a 32-bit id anyway is how a reader
        // ends up one object table behind for the whole file.
        let hawiyat_kabeera = if (7..14).contains(&isdar) {
            qari.iqra_i32("big id enabled")? != 0
        } else {
            false
        };

        let adad_kaainat = tahaqquq_adad(
            SIGHA_MULSAL,
            "object count",
            qari.iqra_i32("object count")?,
            AQSA_KAAINAT,
        )?;
        let mut kaainat = Vec::with_capacity(hajz_mahdud(adad_kaainat));
        for _ in 0..adad_kaainat {
            kaainat.push(iqra_madkhal_kaain(
                &mut qari,
                isdar,
                hawiyat_kabeera,
                izahat_bayanat,
                &anwa,
            )?);
        }

        let mut scripts = Vec::new();
        if isdar >= 11 {
            let adad = tahaqquq_adad(
                SIGHA_MULSAL,
                "script type count",
                qari.iqra_i32("script type count")?,
                AQSA_ANWA_SCRIPTS,
            )?;
            scripts.reserve(hajz_mahdud(adad));
            for _ in 0..adad {
                let fahras_malaf = qari.iqra_i32("script file index")?;
                let hawiya_masar = if isdar < 14 {
                    i64::from(qari.iqra_i32("script path id")?)
                } else {
                    qari.hadhi(4);
                    qari.iqra_i64("script path id")?
                };
                scripts.push(MarjaScript {
                    fahras_malaf,
                    hawiya_masar,
                });
            }
        }

        let adad_kharijiyat = tahaqquq_adad(
            SIGHA_MULSAL,
            "external count",
            qari.iqra_i32("external count")?,
            AQSA_KHARIJIYAT,
        )?;
        let mut kharijiyat = Vec::with_capacity(hajz_mahdud(adad_kharijiyat));
        for _ in 0..adad_kharijiyat {
            let masar_muaqqat = if isdar >= 6 {
                qari.iqra_nass_munahi("external placeholder")?
            } else {
                String::new()
            };
            let (muarrif, naw) = if isdar >= 5 {
                (
                    qari.iqra_masfufa::<16>("external guid")?,
                    qari.iqra_i32("external type")?,
                )
            } else {
                ([0_u8; 16], 0)
            };
            let masar = qari.iqra_nass_munahi("external path")?;
            kharijiyat.push(MarjaKhariji {
                masar_muaqqat,
                muarrif,
                naw,
                masar,
            });
        }

        let mut anwa_marjiiya = Vec::new();
        if isdar >= 20 {
            let adad = tahaqquq_adad(
                SIGHA_MULSAL,
                "reference type count",
                qari.iqra_i32("reference type count")?,
                AQSA_ANWA,
            )?;
            anwa_marjiiya.reserve(hajz_mahdud(adad));
            for _ in 0..adad {
                anwa_marjiiya.push(iqra_naw(
                    &mut qari,
                    isdar,
                    shajarat_anwa_muftaala,
                    QaimatAnwa::Marjiiya,
                )?);
            }
        }

        Ok(Self {
            bayt,
            isdar,
            nihaya,
            hajm_bayanat_wasfiya,
            hajm_malaf,
            izahat_bayanat,
            isdar_muharrik,
            minassa,
            shajarat_anwa_muftaala,
            anwa,
            anwa_marjiiya,
            kaainat,
            scripts,
            kharijiyat,
        })
    }

    /// **Whether this file describes its own layout.**
    ///
    /// The one boolean every caller routes on. False means the build was made
    /// with the type tree stripped, and every `MonoBehaviour` in the file is
    /// refused with
    /// [`SababRafd::BilaShajaratAnwa`][crate::rafd::SababRafd::BilaShajaratAnwa]
    /// rather than scanned for byte patterns that look like strings.
    ///
    /// It is deliberately *not* the header flag alone. A file can declare the
    /// flag and still carry empty trees — Unity writes exactly that for a build
    /// where the flag was flipped after the types were serialized — so this
    /// answers true only when at least one type actually carries usable nodes.
    #[must_use]
    pub fn ladayhi_shajarat_anwa(&self) -> bool {
        self.shajarat_anwa_muftaala && self.anwa.iter().any(NawMulsal::maqru)
    }

    /// Whether the header claimed a type tree, whatever the types turned out to
    /// hold.
    ///
    /// Reported beside [`Mulsal::ladayhi_shajarat_anwa`] because the two
    /// disagreeing is worth saying out loud in a refusal: "the build says it
    /// kept the layout and did not" is a different diagnosis from "the build
    /// stripped it", and only the first is a sign of a broken build rather than
    /// an intentional one.
    #[must_use]
    pub const fn yudai_shajarat_anwa(&self) -> bool {
        self.shajarat_anwa_muftaala
    }

    /// The format version this file declares.
    #[must_use]
    pub const fn isdar(&self) -> u32 {
        self.isdar
    }

    /// The byte order everything after the header is in.
    #[must_use]
    pub const fn nihaya(&self) -> Nihaya {
        self.nihaya
    }

    /// The engine build that wrote it, such as `2021.3.16f1`.
    #[must_use]
    pub fn isdar_muharrik(&self) -> &str {
        &self.isdar_muharrik
    }

    /// Unity's own target platform number.
    #[must_use]
    pub const fn minassa(&self) -> i32 {
        self.minassa
    }

    /// How large the file says it is.
    #[must_use]
    pub const fn hajm_malaf(&self) -> u64 {
        self.hajm_malaf
    }

    /// How large the file says its metadata is.
    #[must_use]
    pub const fn hajm_bayanat_wasfiya(&self) -> u64 {
        self.hajm_bayanat_wasfiya
    }

    /// Where object data begins.
    #[must_use]
    pub const fn izahat_bayanat(&self) -> u64 {
        self.izahat_bayanat
    }

    /// The type list.
    #[must_use]
    pub fn anwa(&self) -> &[NawMulsal] {
        &self.anwa
    }

    /// The reference-type list, for formats 20 and above.
    #[must_use]
    pub fn anwa_marjiiya(&self) -> &[NawMulsal] {
        &self.anwa_marjiiya
    }

    /// The object table.
    #[must_use]
    pub fn kaainat(&self) -> &[MadkhalKaain] {
        &self.kaainat
    }

    /// The script-type table.
    #[must_use]
    pub fn scripts(&self) -> &[MarjaScript] {
        &self.scripts
    }

    /// The files this one refers to.
    #[must_use]
    pub fn kharijiyat(&self) -> &[MarjaKhariji] {
        &self.kharijiyat
    }

    /// The type describing one object.
    ///
    /// Resolved through the type index for format 16 and above and by class-id
    /// search below it, which is how the format itself works: before 16 the
    /// object's `typeID` *is* a class id and the type list is keyed by class.
    #[must_use]
    pub fn naw_kaain(&self, madkhal: &MadkhalKaain) -> Option<&NawMulsal> {
        if self.isdar >= 16 {
            let fahras = usize::try_from(madkhal.fahras_naw).ok()?;
            return self.anwa.get(fahras);
        }
        self.anwa.iter().find(|naw| naw.sanf == madkhal.fahras_naw)
    }

    /// One object's bytes and the absolute offset they start at.
    ///
    /// The offset is returned with the slice and is not decoration: Unity's
    /// alignment padding is computed from the position in the *file*, so a
    /// reader handed only the slice aligns to the wrong boundary whenever the
    /// object does not start on a multiple of four. See [`Qari::hadhi`].
    ///
    /// `None` when the object's declared range is not inside the file, which
    /// costs that object and not the file.
    #[must_use]
    pub fn bayt_kaain(&self, madkhal: &MadkhalKaain) -> Option<(&'a [u8], usize)> {
        let bidaya = hajm_usize(madkhal.bidaya)?;
        let mada = hajm_usize(u64::from(madkhal.hajm))?;
        let nihaya = bidaya.checked_add(mada)?;
        let khana = self.bayt.get(bidaya..nihaya)?;
        Some((khana, bidaya))
    }

    /// A cursor positioned at one object, in the file's own byte order.
    ///
    /// # Errors
    ///
    /// [`KhataQira::HaqlTalif`] when the object's declared range is not inside
    /// the file — a corrupt object table rather than a short read, since the
    /// table said where the bytes were and they are not there.
    pub fn qari_kaain(&self, madkhal: &MadkhalKaain) -> Result<Qari<'a>, KhataQira> {
        let (khana, asas) = self
            .bayt_kaain(madkhal)
            .ok_or_else(|| KhataQira::HaqlTalif {
                sigha: SIGHA_MULSAL,
                haql: "object byte range",
                sabab: format!(
                    "object {} claims {} byte(s) at {} in a {}-byte file",
                    madkhal.hawiya_masar,
                    madkhal.hajm,
                    madkhal.bidaya,
                    self.bayt.len()
                ),
            })?;
        Ok(Qari::jadeed_bi_asas(SIGHA_MULSAL, khana, asas, self.nihaya))
    }
}

/// Which of the two type lists an entry is being read out of.
///
/// The distinction is not cosmetic: a reference type carries its script hash
/// under a different condition and ends with three name strings where an
/// ordinary type ends with a dependency array. Reading one as the other consumes
/// the wrong number of bytes and every later type in the file is read from the
/// wrong place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QaimatAnwa {
    /// The serialized file's own type list.
    Asliya,
    /// The reference-type list, written after it.
    Marjiiya,
}

/// Reads one entry of the type list, or of the reference-type list.
fn iqra_naw(
    qari: &mut Qari<'_>,
    isdar: u32,
    shajarat_muftaala: bool,
    qaima: QaimatAnwa,
) -> Result<NawMulsal, KhataQira> {
    let sanf = qari.iqra_i32("type class id")?;
    let mahdhuf = if isdar >= 16 {
        qari.iqra_bool("type stripped")?
    } else {
        false
    };
    let fahras_script = if isdar >= 17 {
        qari.iqra_i16("type script index")?
    } else {
        -1
    };

    let mut bassmat_script = None;
    let bassmat_naw = if isdar >= 13 {
        // The condition changed shape at format 16: before it a MonoBehaviour
        // type had a negative class id, after it the class id is literally 114.
        // Testing only one of the two skips or invents sixteen bytes.
        let lahu_script = if qaima == QaimatAnwa::Marjiiya {
            fahras_script >= 0
        } else {
            (isdar < 16 && sanf < 0) || (isdar >= 16 && sanf == 114)
        };
        if lahu_script {
            bassmat_script = Some(qari.iqra_masfufa::<16>("type script hash")?);
        }
        Some(qari.iqra_masfufa::<16>("type hash")?)
    } else {
        None
    };

    let mut shajara = None;
    let mut ism_sanf = None;
    let mut nitaq_asma = None;
    let mut ism_tajmee = None;
    let mut taabiyat = Vec::new();

    if shajarat_muftaala {
        // Format 10 wrote the flat form and formats 11 went back to the
        // recursive one, so this is a membership test rather than a threshold.
        let mabnaa = if isdar >= 12 || isdar == 10 {
            iqra_shajara_musattaha(qari, isdar)?
        } else {
            iqra_shajara_mutakarrira(qari, isdar)?
        };
        if isdar >= 21 {
            if qaima == QaimatAnwa::Marjiiya {
                ism_sanf = Some(qari.iqra_nass_munahi("reference class name")?);
                nitaq_asma = Some(qari.iqra_nass_munahi("reference namespace")?);
                ism_tajmee = Some(qari.iqra_nass_munahi("reference assembly")?);
            } else {
                let adad = tahaqquq_adad(
                    SIGHA_MULSAL,
                    "type dependency count",
                    qari.iqra_i32("type dependency count")?,
                    AQSA_ANWA,
                )?;
                taabiyat.reserve(hajz_mahdud(adad));
                for _ in 0..adad {
                    taabiyat.push(qari.iqra_i32("type dependency")?);
                }
            }
        }
        shajara = Some(mabnaa);
    }

    Ok(NawMulsal {
        sanf,
        mahdhuf,
        fahras_script,
        bassmat_script,
        bassmat_naw,
        shajara,
        ism_sanf,
        nitaq_asma,
        ism_tajmee,
        taabiyat,
    })
}

/// One node of the flat type tree, before its names are resolved.
///
/// Held separately because the names are offsets into a buffer that has not been
/// read yet: the node array comes first and the string buffer follows it, so
/// resolution is a second pass and cannot be folded into the first.
#[derive(Debug, Clone, Copy)]
struct UqdaKhaam {
    isdar: u16,
    mustawa: u8,
    rayat_naw: u8,
    izahat_naw: u32,
    izahat_ism: u32,
    hajm: i32,
    fahras: i32,
    rayat_bayanat: u32,
}

/// The flat type tree: a node array, then one string buffer they index.
fn iqra_shajara_musattaha(qari: &mut Qari<'_>, isdar: u32) -> Result<ShajaratAnwa, KhataQira> {
    let adad = tahaqquq_adad(
        SIGHA_MULSAL,
        "type tree node count",
        qari.iqra_i32("type tree node count")?,
        AQSA_UQAD_SHAJARA,
    )?;
    let hajz_asma = tahaqquq_adad(
        SIGHA_MULSAL,
        "type tree string buffer size",
        qari.iqra_i32("type tree string buffer size")?,
        AQSA_HAJZ_ASMA,
    )?;

    let mut khaam = Vec::with_capacity(hajz_mahdud(adad));
    for _ in 0..adad {
        let uqda = UqdaKhaam {
            isdar: qari.iqra_u16("type tree node version")?,
            mustawa: qari.iqra_u8("type tree node level")?,
            rayat_naw: qari.iqra_u8("type tree node type flags")?,
            izahat_naw: qari.iqra_u32("type tree node type offset")?,
            izahat_ism: qari.iqra_u32("type tree node name offset")?,
            hajm: qari.iqra_i32("type tree node byte size")?,
            fahras: qari.iqra_i32("type tree node index")?,
            rayat_bayanat: qari.iqra_u32("type tree node meta flag")?,
        };
        if isdar >= 19 {
            let _ = qari.iqra_u64("type tree node ref hash")?;
        }
        khaam.push(uqda);
    }

    let hajz = qari.iqra_bayt("type tree string buffer", u64::from(hajz_asma))?;

    let mut uqad = Vec::with_capacity(khaam.len());
    let mut majhula = None;
    for (fahras, uqda) in khaam.iter().enumerate() {
        let naw = ism_min_izaha(uqda.izahat_naw, hajz);
        let ism = ism_min_izaha(uqda.izahat_ism, hajz);
        if majhula.is_none() {
            let raqm = u32::try_from(fahras).unwrap_or(u32::MAX);
            if naw.is_none() {
                majhula = Some((raqm, uqda.izahat_naw));
            } else if ism.is_none() {
                majhula = Some((raqm, uqda.izahat_ism));
            }
        }
        uqad.push(UqdatShajara {
            isdar: uqda.isdar,
            mustawa: uqda.mustawa,
            masfufa: uqda.rayat_naw & 0x1 != 0,
            naw,
            ism,
            hajm: uqda.hajm,
            fahras: uqda.fahras,
            rayat_bayanat: uqda.rayat_bayanat,
        });
    }
    Ok(ShajaratAnwa { uqad, majhula })
}

/// The pre-format-12 type tree: a node followed by its children, recursively.
///
/// Flattened into the same node list the newer form produces, so the reader in
/// [`crate::unity::kaain`] has one shape to walk. The level field carries the
/// nesting that the recursion expressed structurally, which is exactly the
/// transformation Unity itself made when it changed the format.
fn iqra_shajara_mutakarrira(qari: &mut Qari<'_>, isdar: u32) -> Result<ShajaratAnwa, KhataQira> {
    let mut uqad = Vec::new();
    iqra_uqda_mutakarrira(qari, isdar, 0, &mut uqad)?;
    Ok(ShajaratAnwa {
        uqad,
        majhula: None,
    })
}

/// One node of the recursive tree and everything under it.
fn iqra_uqda_mutakarrira(
    qari: &mut Qari<'_>,
    isdar: u32,
    mustawa: u8,
    uqad: &mut Vec<UqdatShajara>,
) -> Result<(), KhataQira> {
    if mustawa > AQSA_UMQ_SHAJARA {
        return Err(KhataQira::TajawuzHadd {
            hadd: "type tree depth",
            qeema: u64::from(mustawa),
            saqf: u64::from(AQSA_UMQ_SHAJARA),
        });
    }
    if tul_u64(uqad.len()) >= u64::from(AQSA_UQAD_SHAJARA) {
        return Err(KhataQira::TajawuzHadd {
            hadd: "type tree node count",
            qeema: tul_u64(uqad.len()),
            saqf: u64::from(AQSA_UQAD_SHAJARA),
        });
    }

    let naw = qari.iqra_nass_munahi("type tree node type")?;
    let ism = qari.iqra_nass_munahi("type tree node name")?;
    let hajm = qari.iqra_i32("type tree node byte size")?;
    if isdar == 2 {
        let _ = qari.iqra_i32("type tree node variable count")?;
    }
    // Format 3 dropped the index and the meta flag entirely. Reading them anyway
    // consumes eight bytes that belong to the next node's type name, and the
    // name that comes out is a fragment that resolves to nothing.
    let fahras = if isdar == 3 {
        -1
    } else {
        qari.iqra_i32("type tree node index")?
    };
    let rayat_naw = qari.iqra_i32("type tree node type flags")?;
    let isdar_uqda = qari.iqra_i32("type tree node version")?;
    let rayat_bayanat = if isdar == 3 {
        0
    } else {
        qari.iqra_i32("type tree node meta flag")?
    };

    uqad.push(UqdatShajara {
        isdar: u16::try_from(isdar_uqda).unwrap_or(0),
        mustawa,
        masfufa: rayat_naw & 0x1 != 0,
        naw: Some(naw),
        ism: Some(ism),
        hajm,
        fahras,
        rayat_bayanat: u32::from_le_bytes(rayat_bayanat.to_le_bytes()),
    });

    let adad = tahaqquq_adad(
        SIGHA_MULSAL,
        "type tree child count",
        qari.iqra_i32("type tree child count")?,
        AQSA_UQAD_SHAJARA,
    )?;
    for _ in 0..adad {
        iqra_uqda_mutakarrira(qari, isdar, mustawa.saturating_add(1), uqad)?;
    }
    Ok(())
}

/// Reads one object table entry.
///
/// Every version branch here is a place where a wrong guess costs the whole
/// table rather than one field: the entries are packed with no separators, so a
/// reader that is four bytes out on entry zero is four bytes out on every entry
/// after it and produces two million plausible-looking objects at wrong offsets.
fn iqra_madkhal_kaain(
    qari: &mut Qari<'_>,
    isdar: u32,
    hawiyat_kabeera: bool,
    izahat_bayanat: u64,
    anwa: &[NawMulsal],
) -> Result<MadkhalKaain, KhataQira> {
    let hawiya_masar = if hawiyat_kabeera {
        qari.iqra_i64("object path id")?
    } else if isdar < 14 {
        i64::from(qari.iqra_i32("object path id")?)
    } else {
        // From format 14 the path id is eight bytes and is preceded by padding
        // to a four-byte boundary. The padding is not in any field's size and
        // is easy to miss; missing it puts every subsequent entry two bytes out.
        qari.hadhi(4);
        qari.iqra_i64("object path id")?
    };

    let bidaya_nisbiya = if isdar >= 22 {
        let khaam = qari.iqra_i64("object byte start")?;
        u64::try_from(khaam).map_err(|_| KhataQira::HaqlTalif {
            sigha: SIGHA_MULSAL,
            haql: "object byte start",
            sabab: format!("negative offset {khaam}"),
        })?
    } else {
        u64::from(qari.iqra_u32("object byte start")?)
    };
    let bidaya =
        bidaya_nisbiya
            .checked_add(izahat_bayanat)
            .ok_or_else(|| KhataQira::TajawuzHadd {
                hadd: "object byte start",
                qeema: bidaya_nisbiya,
                saqf: u64::MAX.saturating_sub(izahat_bayanat),
            })?;

    let hajm = qari.iqra_u32("object byte size")?;
    let fahras_naw = qari.iqra_i32("object type id")?;

    let sanf = if isdar < 16 {
        i32::from(qari.iqra_u16("object class id")?)
    } else {
        // A type index that does not resolve gets `-1` and not the index itself.
        // Unity has no class `-1`, so the object simply matches nothing and is
        // refused — whereas passing the index through could make object 49 of a
        // corrupt file look like a `TextAsset` and be read as one.
        usize::try_from(fahras_naw)
            .ok()
            .and_then(|fahras| anwa.get(fahras))
            .map_or(-1, |naw| naw.sanf)
    };
    if isdar <= 11 {
        let _ = qari.iqra_u16("object is destroyed")?;
    }
    if (11..=17).contains(&isdar) {
        let _ = qari.iqra_i16("object script type index")?;
    }
    let mahdhuf = if isdar == 15 || isdar == 16 {
        qari.iqra_u8("object stripped")? != 0
    } else {
        false
    };

    Ok(MadkhalKaain {
        hawiya_masar,
        bidaya,
        hajm,
        fahras_naw,
        sanf,
        mahdhuf,
    })
}

// ---------------------------------------------------------------------------
// Class ids
//
// Only the handful this crate reads. Unity has several hundred and enumerating
// them all here would be a table nobody maintains; the ones below are each
// used by name in `kaain`.
// ---------------------------------------------------------------------------

/// `GameObject` — the node whose name a component's structural path is built
/// from.
pub const SANF_GAMEOBJECT: i32 = 1;

/// `Transform` — the parent link that turns those names into `Canvas/Panel/Label`.
pub const SANF_TRANSFORM: i32 = 4;

/// `TextAsset` — a whole file stored as an asset, with its bytes in `m_Script`.
pub const SANF_TEXTASSET: i32 = 49;

/// `MonoBehaviour` — every script-authored component, and the one that carries
/// almost all of a Unity game's user-facing text.
pub const SANF_MONOBEHAVIOUR: i32 = 114;

/// `MonoScript` — the class name a `MonoBehaviour` points at, which is how a
/// component gets a name a translator would recognise.
pub const SANF_MONOSCRIPT: i32 = 115;

/// `RectTransform` — a `Transform` for UI, with the same parent link.
pub const SANF_RECTTRANSFORM: i32 = 224;
