//! القاموس — Capcom's `DICT` string dictionary, read by structure.
//!
//! One small container, used by the PC ports of the GameCube-era Capcom titles:
//! a fixed header, a flat bucket array of `(hash, pointer)` pairs, and a blob of
//! NUL-terminated strings. Resident Evil 4 ships eight of them, one per
//! language, under `BIO4/text/<LANGUAGE>_WIN32.dct`.
//!
//! Ten kilobytes of menu text is exactly the size at which a byte-pattern scan
//! looks like it works. It does not: 68 of the 403 buckets are empty, a Chinese
//! build points two of its buckets into the *middle* of another string so that
//! one run of bytes is two entries, and two of the eight files carry a tail of
//! padding after the last one. A scan would report 336 strings where the file
//! defines 335 entries, would silently merge the two that overlap, and would
//! have no way to write any of them back — a string's address is not in the
//! blob, it is in a bucket, computed. So every byte below is reached through a
//! field that the game's own loader reads.
//!
//! ## The layout, and where each claim comes from
//!
//! ```text
//! 0x00  u32   magic       'DICT'
//! 0x04  u32   version     0x1000
//! 0x08  u32   seed        the CRC seed the lookup hashes keys with
//! 0x0c  u32   buckets     self-relative pointer to the bucket array
//! 0x10  i32   count       how many buckets
//! ...         bucket array: count × { u32 hash; u32 pointer; }
//! ...         string blob: NUL-terminated bytes
//! ```
//!
//! Every pointer in the file is **self-relative with a one-byte bias**: the
//! target is `&field + value + 1`. That is not a guess. `bio4.exe` carries the
//! function that converts a loaded dictionary's pointers back to file offsets,
//! and it subtracts `field_address + 1` from each one; the header's own bucket
//! pointer is `7`, and `0x0c + 7 + 1` is `0x14`, where the array is. The bias
//! falls out of how the blob is laid out — a leading NUL, then each string
//! followed by its terminator — so a pointer names the NUL in front of its
//! string rather than the string's first byte.
//!
//! A value of `0` is a null pointer, not "the string at blob offset 0". The
//! loader tests for it and leaves it alone, and the English file relies on that:
//! its 68 empty buckets store `0`.
//!
//! ## The hash is over the key name, and the key name is not in the file
//!
//! [`basmat_miftah`] is the reflected CRC-32 polynomial `0xEDB88320`, seeded
//! with the header's own `seed` field, run over the key's bytes without its
//! terminator, with **no** final complement. The game hashes a key name with it
//! and then scans the bucket array linearly for a match.
//!
//! Two independent proofs, both from real bytes:
//!
//! * The bucket array is byte-identical in all eight language files, whose text
//!   is entirely different. Whatever is hashed is language-independent, so it is
//!   not the string.
//! * Two hundred and forty-nine of the 335 occupied buckets are reproduced
//!   exactly by hashing an ASCII string found in `bio4.exe` — `text_f12` hashes
//!   to the bucket holding `"F12"`, `text_cancel_loading` to the bucket holding
//!   `"Cancel loading?…"`, and so on for two hundred and forty-seven more.
//!
//! So write-back can add a key: hash its name and store the pair. What it cannot
//! do is recover the name of an existing key, because a CRC is not invertible.
//! The name is therefore not the identity this module publishes — the hash is,
//! which is what the engine itself looks strings up by and what survives both a
//! text edit and a rebuild.
//!
//! ## The bucket index is a hash table the loader does not use
//!
//! Every occupied bucket in all eight files sits at `hash % count`, or at the
//! next free slot after it. The writer built an open-addressed table; the
//! shipped loader ignores that and scans. Both facts are load-bearing in
//! opposite directions: the probe rule is a free integrity check on a file this
//! module claims to understand, and the linear scan is why [`Qamus::abhath`]
//! answers exactly what the game would answer even for a file whose buckets are
//! in an order the probe rule would reject.
//!
//! ## Two of Resident Evil 4's eight files were written by a different tool
//!
//! The six original localizations round-trip through [`Qamus::ibni`] to the
//! byte. The two Chinese ones do not, and the differences are worth naming
//! because they are what a reader invents assumptions about:
//!
//! | | six originals | `CHINESE_S`, `CHINESE_T` |
//! | --- | --- | --- |
//! | empty buckets | pointer `0` | a real pointer, hash `0` |
//! | blob | leading NUL sentinel | none |
//! | tail | ends at the last terminator | padded to a 32-byte boundary |
//! | sharing | none | `CHINESE_T` aims two buckets at another string's suffix |
//! | encoding | UTF-8 | UTF-8 **mixed with** Big5, in 7 and 30 strings |
//!
//! [`Qamus::iqra`] reads all eight, and [`Qamus::uktub`] returns all eight
//! byte-identically, because it keeps the blob and the stored pointers verbatim
//! rather than deriving them. The mixed encoding is the one thing this module
//! refuses, and it refuses the whole file: naming a code page for the 7 strings
//! that are not UTF-8 would be a guess, and extracting the other 321 would be
//! the partial read [`crate::rafd`] exists to forbid.
//!
//! ## `^983047^` is a button glyph, not markup
//!
//! The strings carry tokens like `^983047^`, which is decimal `0xF0007` — a
//! Unicode private-use code point the game swaps for a controller or keyboard
//! glyph at draw time. `taarib_saff`'s markup parser does not know this dialect,
//! so the tokens stay in the clean text and reach a translator as-is. That is
//! visible and fixable; lifting them here with a second parser would be the
//! second opinion about placeholders that [`crate::jadwal::irfa_nasq`] exists to
//! prevent.

use std::path::{Path, PathBuf};

use crate::jadwal::{JadwalNusus, MawqiNass};
use crate::rafd::{SababRafd, TaqreerRafd};
use crate::tasnif::{TalabMudkhal, ansha_mudkhal};

/// The four bytes a dictionary starts with.
pub const TAWQEE: [u8; 4] = *b"DICT";

/// The only version the shipped loader accepts.
///
/// It compares this field for exact equality and refuses everything else, so a
/// reader that accepted more would be reading files the game will not.
pub const ISDAR_MADUM: u32 = 0x1000;

/// The header, in bytes.
pub const TUL_TARWISA: usize = 20;

/// One bucket: a hash and a pointer.
pub const HAJM_MUDKHAL: usize = 8;

/// Where the version sits.
const MAWQI_ISDAR: usize = 4;

/// Where the CRC seed sits.
const MAWQI_BIDHRA: usize = 8;

/// Where the pointer to the bucket array sits.
const MAWQI_MUASHIR: usize = 12;

/// Where the bucket count sits.
const MAWQI_ADAD: usize = 16;

/// The largest bucket count this build will allocate for.
///
/// Resident Evil 4 declares 403. A file declaring a million buckets is damaged
/// or hostile, and either way the answer is to refuse before reserving rather
/// than after.
pub const AQSA_MADAKHIL: u32 = 1 << 20;

/// The largest dictionary this build will read.
///
/// Sixteen mebibytes against a real file of ten kilobytes. The ceiling is here
/// to stop a mislabelled file, not to bound a real one.
pub const AQSA_MALAF: u64 = 16 * 1024 * 1024;

/// How deep the walk for dictionaries goes.
pub const UMQ_MASH: usize = 24;

/// How many filesystem entries the walk will look at.
pub const AQSA_MALAFAT: usize = 400_000;

/// The hash the game looks a key up by.
///
/// The reflected CRC-32 polynomial `0xEDB88320`, seeded with `bidhra` — which is
/// the dictionary's own header field and not a constant — over the key's bytes,
/// terminator excluded, with no final complement.
///
/// Written bitwise rather than from a 256-entry table on purpose. The table is a
/// kilobyte of transcribed constants that no reviewer checks; the polynomial is
/// the definition, and there is nothing in it to transcribe wrongly.
///
/// It is deliberately *not* [`taarib_muhawwil_unreal::mawarid::locres::basmat_crc`],
/// which shares the polynomial and nothing else: that one widens to UTF-16, and
/// seeds and complements with `u32::MAX`. Two functions that agree on a table and
/// disagree on everything around it are two functions.
#[must_use]
pub fn basmat_miftah(miftah: &str, bidhra: u32) -> u32 {
    let mut basma = bidhra;
    for wahda in miftah.as_bytes() {
        basma = crc_bayt(basma, *wahda);
    }
    basma
}

/// One byte through the reflected CRC-32 polynomial.
fn crc_bayt(basma: u32, wahda: u8) -> u32 {
    let mut hali = basma ^ u32::from(wahda);
    let mut bit = 0_u8;
    while bit < 8 {
        hali = if hali & 1 == 0 {
            hali >> 1
        } else {
            (hali >> 1) ^ 0xEDB8_8320
        };
        bit = bit.saturating_add(1);
    }
    hali
}

/// One bucket.
///
/// The stored pointer is kept beside the resolved blob offset rather than
/// recomputed from it, so that [`Qamus::uktub`] writes back the number the file
/// actually held. A writer that re-derived it would be asserting that its idea
/// of the layout matches the tool that produced the file, which for two of
/// Resident Evil 4's own eight files is false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MudkhalQamus {
    /// The key's hash, and the file's only identity for this string.
    basma: u32,
    /// The pointer field exactly as stored.
    izaha: u32,
    /// Where the string starts in the blob, or [`None`] for a null pointer.
    mawqi: Option<u32>,
}

impl MudkhalQamus {
    /// The key's hash.
    #[must_use]
    pub const fn basma(self) -> u32 {
        self.basma
    }

    /// The pointer field exactly as the file stores it.
    #[must_use]
    pub const fn izaha(self) -> u32 {
        self.izaha
    }

    /// Where this bucket's string starts in the blob.
    ///
    /// [`None`] for a null pointer, which is how the six original Resident
    /// Evil 4 files mark an empty bucket.
    #[must_use]
    pub const fn mawqi(self) -> Option<u32> {
        self.mawqi
    }

    /// Whether the bucket holds a key at all.
    ///
    /// A hash of zero is how a bucket says "empty" — the lookup would never
    /// match it, because a key hashing to zero is a key whose CRC happened to
    /// land there and the shipped loader special-cases the seed instead.
    #[must_use]
    pub const fn mashghul(self) -> bool {
        self.basma != 0
    }
}

/// A parsed dictionary.
///
/// Holds enough to write the file back byte for byte: the header's own fields,
/// any bytes between the header and the bucket array, every bucket's stored
/// pointer, and the string blob verbatim. Nothing is normalised on the way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qamus {
    /// The version field, preserved rather than assumed.
    isdar: u32,
    /// The CRC seed the file's own keys were hashed with.
    bidhra: u32,
    /// The stored pointer to the bucket array, kept so the header round-trips.
    muashir: u32,
    /// Whatever sits between the header and the bucket array.
    ///
    /// Empty in every shipped file — the array starts at [`TUL_TARWISA`]. Kept
    /// because the pointer field is what decides where the array is, and a file
    /// that puts it further along is one this module can still return unchanged.
    hashw: Vec<u8>,
    /// The buckets, in file order.
    madakhil: Vec<MudkhalQamus>,
    /// The string blob, exactly as stored.
    kutla: Vec<u8>,
}

impl Qamus {
    /// Reads a dictionary.
    ///
    /// # Errors
    ///
    /// A [`SababRafd`] rather than an error type, because every way this can
    /// fail is a reason to skip the container and none of them stops an
    /// extraction — see [`crate::rafd`]. [`SababRafd::SighaMajhula`] when the
    /// magic is not `DICT`, [`SababRafd::IsdarGhayrMadum`] when the version is
    /// one the game itself would refuse, [`SababRafd::TajawuzHadd`] when the
    /// file or the declared bucket count is above this build's ceiling, and
    /// [`SababRafd::Talif`] for every structural failure: a truncated bucket
    /// array, a pointer outside the blob, a string with no terminator before the
    /// end of the file.
    pub fn iqra(bayt: &[u8]) -> Result<Self, SababRafd> {
        let tul = bayt.len();
        let hajm = u64::try_from(tul).unwrap_or(u64::MAX);
        if hajm > AQSA_MALAF {
            return Err(SababRafd::TajawuzHadd {
                hadd: "a DICT dictionary".to_owned(),
                qeema: hajm,
                saqf: AQSA_MALAF,
            });
        }
        let Some(sihr) = bayt.get(..TAWQEE.len()) else {
            return Err(SababRafd::SighaMajhula {
                wujid: format!("a {tul}-byte file, shorter than a DICT signature"),
            });
        };
        if sihr != TAWQEE {
            return Err(SababRafd::SighaMajhula {
                wujid: "a file that does not carry Capcom's DICT signature".to_owned(),
            });
        }

        let (Some(isdar), Some(bidhra), Some(muashir), Some(adad)) = (
            kalima(bayt, MAWQI_ISDAR),
            kalima(bayt, MAWQI_BIDHRA),
            kalima(bayt, MAWQI_MUASHIR),
            kalima(bayt, MAWQI_ADAD),
        ) else {
            return Err(SababRafd::Talif {
                sabab: format!("the header needs {TUL_TARWISA} bytes and the file has {tul}"),
            });
        };

        if isdar != ISDAR_MADUM {
            return Err(SababRafd::IsdarGhayrMadum {
                sigha: "Capcom DICT".to_owned(),
                wujid: format!("{isdar:#010x}"),
                madum: format!("{ISDAR_MADUM:#010x}"),
            });
        }

        // Signed in the loader, which compares the count against zero with a
        // signed branch and reads nothing when it is negative. A file declaring
        // a negative count is damaged rather than empty, and saying so is more
        // use than silently agreeing it holds no strings.
        let adad_i = i32::from_le_bytes(adad.to_le_bytes());
        if adad_i < 0 {
            return Err(SababRafd::Talif {
                sabab: format!("the bucket count is {adad_i}"),
            });
        }
        if adad > AQSA_MADAKHIL {
            return Err(SababRafd::TajawuzHadd {
                hadd: "a DICT bucket array".to_owned(),
                qeema: u64::from(adad),
                saqf: u64::from(AQSA_MADAKHIL),
            });
        }

        let bidayat_jadwal = mawqi_jadwal(muashir, adad, tul)?;
        let nihayat_jadwal = nihayat_jadwal(bidayat_jadwal, adad, tul)?;
        let Some(hashw) = bayt.get(TUL_TARWISA..bidayat_jadwal) else {
            return Err(SababRafd::Talif {
                sabab: "the bucket array is declared to start inside the header".to_owned(),
            });
        };
        let Some(kutla) = bayt.get(nihayat_jadwal..) else {
            return Err(SababRafd::Talif {
                sabab: "the bucket array runs past the end of the file".to_owned(),
            });
        };

        let mut madakhil = Vec::with_capacity(hajm_usize(adad));
        for fahras in 0..hajm_usize(adad) {
            madakhil.push(iqra_mudkhal(bayt, bidayat_jadwal, nihayat_jadwal, fahras)?);
        }

        Ok(Self {
            isdar,
            bidhra,
            muashir,
            hashw: hashw.to_vec(),
            madakhil,
            kutla: kutla.to_vec(),
        })
    }

    /// The version field.
    #[must_use]
    pub const fn isdar(&self) -> u32 {
        self.isdar
    }

    /// The CRC seed this file's keys were hashed with.
    ///
    /// Read from the file rather than hardcoded, because the shipped lookup
    /// reads it from the file: it loads the header's `+8` word and passes it
    /// straight to the hash as the starting value.
    #[must_use]
    pub const fn bidhra(&self) -> u32 {
        self.bidhra
    }

    /// Every bucket, in file order.
    #[must_use]
    pub fn madakhil(&self) -> &[MudkhalQamus] {
        &self.madakhil
    }

    /// How many buckets the file declares.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// The string blob, exactly as stored.
    #[must_use]
    pub fn kutla(&self) -> &[u8] {
        &self.kutla
    }

    /// One bucket's string, as raw bytes.
    ///
    /// [`None`] for a null pointer. Bytes rather than `&str` because two of
    /// Resident Evil 4's own dictionaries hold strings that are not UTF-8, and a
    /// reader that decoded lossily here would hand a translator a replacement
    /// character to faithfully carry into the translation.
    #[must_use]
    pub fn bayt(&self, madkhal: MudkhalQamus) -> Option<&[u8]> {
        let mawqi = hajm_usize(madkhal.mawqi?);
        let baqi = self.kutla.get(mawqi..)?;
        let tul = baqi.iter().position(|wahda| *wahda == 0)?;
        baqi.get(..tul)
    }

    /// One bucket's string, decoded.
    ///
    /// [`None`] for a null pointer *and* for bytes that are not UTF-8; the two
    /// are told apart by [`Qamus::bayt`] answering `Some` for the second.
    #[must_use]
    pub fn nass(&self, madkhal: MudkhalQamus) -> Option<&str> {
        std::str::from_utf8(self.bayt(madkhal)?).ok()
    }

    /// The string the game would return for a key name.
    ///
    /// Reproduces the shipped lookup exactly, including its two refusals: a key
    /// whose hash equals the seed — which is every empty key, since the hash of
    /// an empty string is the seed unchanged — and a bucket whose string is
    /// empty both answer [`None`], and the game falls back to drawing the key
    /// name itself.
    ///
    /// A linear scan, because that is what the game does. The buckets are laid
    /// out as an open-addressed table and the loader ignores it.
    #[must_use]
    pub fn abhath(&self, miftah: &str) -> Option<&[u8]> {
        let basma = basmat_miftah(miftah, self.bidhra);
        if basma == self.bidhra {
            return None;
        }
        let madkhal = *self
            .madakhil
            .iter()
            .find(|madkhal| madkhal.basma == basma)?;
        let nass = self.bayt(madkhal)?;
        if nass.is_empty() { None } else { Some(nass) }
    }

    /// Writes the dictionary back.
    ///
    /// Byte-identical to what [`Qamus::iqra`] read, for every file, because the
    /// header fields, the stored pointers and the blob all come back out
    /// unchanged. Nothing here recomputes a pointer — see [`Qamus::ibni`] for
    /// the path that does.
    #[must_use]
    pub fn uktub(&self) -> Vec<u8> {
        let hajm = TUL_TARWISA
            .saturating_add(self.hashw.len())
            .saturating_add(self.madakhil.len().saturating_mul(HAJM_MUDKHAL))
            .saturating_add(self.kutla.len());
        let mut makhruj = Vec::with_capacity(hajm);
        makhruj.extend_from_slice(&TAWQEE);
        makhruj.extend_from_slice(&self.isdar.to_le_bytes());
        makhruj.extend_from_slice(&self.bidhra.to_le_bytes());
        makhruj.extend_from_slice(&self.muashir.to_le_bytes());
        let adad = u32::try_from(self.madakhil.len()).unwrap_or(u32::MAX);
        makhruj.extend_from_slice(&adad.to_le_bytes());
        makhruj.extend_from_slice(&self.hashw);
        for madkhal in &self.madakhil {
            makhruj.extend_from_slice(&madkhal.basma.to_le_bytes());
            makhruj.extend_from_slice(&madkhal.izaha.to_le_bytes());
        }
        makhruj.extend_from_slice(&self.kutla);
        makhruj
    }

    /// Builds a dictionary from keys and strings, laying the blob out afresh.
    ///
    /// This is the write-back path: it is what a translated dictionary is made
    /// with, and it is the only thing here that computes a pointer. The layout
    /// it produces is the one the game's own localization tool produced — a
    /// leading NUL, then every bucket's string in bucket order, each followed by
    /// its terminator — which is why the six original Resident Evil 4 files come
    /// back byte-identical through it and not only through [`Qamus::uktub`].
    ///
    /// The two Chinese files do not, and cannot: they open the blob without the
    /// sentinel, end it with padding no bucket points at, and in one case aim
    /// two buckets at another string's suffix so that one run of bytes serves
    /// two entries. Rebuilding them produces a dictionary the game reads
    /// correctly and a file that is not the one that was read, which is why
    /// round-tripping goes through [`Qamus::uktub`] and translating goes through
    /// here.
    ///
    /// `madakhil` is the bucket array in order: a hash, and the string it points
    /// at or [`None`] for a null pointer. Nothing reorders it — the bucket order
    /// is the file's own and a caller replacing text keeps it.
    ///
    /// # Errors
    ///
    /// [`SababRafd::TajawuzHadd`] when the bucket count is above
    /// [`AQSA_MADAKHIL`], when the strings do not fit a blob a `u32` pointer can
    /// reach, or when a string carries an interior NUL — which no pointer could
    /// name the whole of, so accepting it would silently truncate.
    pub fn ibni(bidhra: u32, madakhil: &[(u32, Option<&[u8]>)]) -> Result<Self, SababRafd> {
        let adad = u32::try_from(madakhil.len()).unwrap_or(u32::MAX);
        if adad > AQSA_MADAKHIL {
            return Err(SababRafd::TajawuzHadd {
                hadd: "a DICT bucket array".to_owned(),
                qeema: u64::from(adad),
                saqf: u64::from(AQSA_MADAKHIL),
            });
        }

        let nihayat_jadwal = TUL_TARWISA
            .checked_add(madakhil.len().saturating_mul(HAJM_MUDKHAL))
            .ok_or_else(|| hadd_muashir(u64::MAX))?;

        // The sentinel the shipped writer opens the blob with. A pointer names
        // the NUL in front of its string, so the first string needs one in front
        // of it that is not some other string's terminator.
        let mut kutla = vec![0_u8];
        let mut mabniya = Vec::with_capacity(madakhil.len());
        for (fahras, (basma, nass)) in madakhil.iter().enumerate() {
            let Some(nass) = *nass else {
                mabniya.push(MudkhalQamus {
                    basma: *basma,
                    izaha: 0,
                    mawqi: None,
                });
                continue;
            };
            if nass.contains(&0) {
                return Err(SababRafd::TajawuzHadd {
                    hadd: "a DICT string with an interior NUL".to_owned(),
                    qeema: u64::try_from(nass.len()).unwrap_or(u64::MAX),
                    saqf: 0,
                });
            }
            let mawqi = u32::try_from(kutla.len())
                .map_err(|_| hadd_muashir(u64::try_from(kutla.len()).unwrap_or(u64::MAX)))?;
            let haql = mawqi_haql(TUL_TARWISA, fahras).ok_or_else(|| hadd_muashir(u64::MAX))?;
            let hadaf = nihayat_jadwal
                .checked_add(hajm_usize(mawqi))
                .ok_or_else(|| hadd_muashir(u64::MAX))?;
            let izaha = taqyeed_muashir(haql, hadaf)
                .ok_or_else(|| hadd_muashir(u64::try_from(hadaf).unwrap_or(u64::MAX)))?;
            kutla.extend_from_slice(nass);
            kutla.push(0);
            mabniya.push(MudkhalQamus {
                basma: *basma,
                izaha,
                mawqi: Some(mawqi),
            });
        }

        Ok(Self {
            isdar: ISDAR_MADUM,
            bidhra,
            // `0x0c + 7 + 1` is `0x14`, which is where a freshly built array
            // goes. Written through the same arithmetic the reader resolves it
            // with rather than as the literal 7, so the two cannot drift.
            muashir: taqyeed_muashir(MAWQI_MUASHIR, TUL_TARWISA)
                .ok_or_else(|| hadd_muashir(u64::try_from(TUL_TARWISA).unwrap_or(u64::MAX)))?,
            hashw: Vec::new(),
            madakhil: mabniya,
            kutla,
        })
    }

    /// Whether every occupied bucket sits where open addressing would put it.
    ///
    /// A free integrity check rather than a requirement: the shipped loader
    /// scans linearly and would read a file that failed this. It holds for all
    /// eight of Resident Evil 4's dictionaries, across 2,680 buckets, which is
    /// what makes the count field's meaning certain — a modulus that produced
    /// the observed layout by accident 2,680 times running does not exist.
    #[must_use]
    pub fn muttasiq(&self) -> bool {
        let adad = self.madakhil.len();
        if adad == 0 {
            return true;
        }
        let mashghul: Vec<bool> = self
            .madakhil
            .iter()
            .map(|madkhal| madkhal.mashghul())
            .collect();
        for (fahras, madkhal) in self.madakhil.iter().enumerate() {
            if !madkhal.mashghul() {
                continue;
            }
            let Some(bayt) = hajm_usize(madkhal.basma).checked_rem(adad) else {
                return false;
            };
            let mut khana = bayt;
            let mut khutwa = 0_usize;
            while khana != fahras && mashghul.get(khana).copied().unwrap_or(false) && khutwa <= adad
            {
                khana = khana.saturating_add(1) % adad;
                khutwa = khutwa.saturating_add(1);
            }
            if khana != fahras {
                return false;
            }
        }
        true
    }
}

/// Reads one bucket and resolves its pointer.
fn iqra_mudkhal(
    bayt: &[u8],
    bidayat_jadwal: usize,
    nihayat_jadwal: usize,
    fahras: usize,
) -> Result<MudkhalQamus, SababRafd> {
    let haql = mawqi_haql(bidayat_jadwal, fahras).ok_or_else(|| SababRafd::Talif {
        sabab: format!("bucket {fahras} is past the end of addressable memory"),
    })?;
    // The hash sits in the four bytes in front of the pointer field, which is
    // the only place the bucket's start is needed.
    let haql_basma = haql.saturating_sub(4);
    let (Some(basma), Some(izaha)) = (kalima(bayt, haql_basma), kalima(bayt, haql)) else {
        return Err(SababRafd::Talif {
            sabab: format!("bucket {fahras} runs past the end of the file"),
        });
    };

    if izaha == 0 {
        return Ok(MudkhalQamus {
            basma,
            izaha,
            mawqi: None,
        });
    }

    let hadaf = hall_muashir(haql, izaha).ok_or_else(|| SababRafd::Talif {
        sabab: format!("bucket {fahras} points past the end of addressable memory"),
    })?;
    if hadaf < nihayat_jadwal || hadaf >= bayt.len() {
        return Err(SababRafd::Talif {
            sabab: format!(
                "bucket {fahras} points at {hadaf:#x}, outside the string blob \
                 [{nihayat_jadwal:#x}, {:#x})",
                bayt.len()
            ),
        });
    }
    // A string with no terminator before the end of the file is the one damage
    // this format cannot survive: the loader hands the pointer to the renderer
    // and the renderer reads until it finds a zero, which is somewhere else in
    // the process.
    let Some(baqi) = bayt.get(hadaf..) else {
        return Err(SababRafd::Talif {
            sabab: format!("bucket {fahras} points past the end of the file"),
        });
    };
    if !baqi.contains(&0) {
        return Err(SababRafd::Talif {
            sabab: format!("bucket {fahras}'s string has no terminator before the end of the file"),
        });
    }

    let mawqi =
        u32::try_from(hadaf.saturating_sub(nihayat_jadwal)).map_err(|_| SababRafd::Talif {
            sabab: format!("bucket {fahras}'s string is past a u32 offset"),
        })?;
    Ok(MudkhalQamus {
        basma,
        izaha,
        mawqi: Some(mawqi),
    })
}

/// Where the bucket array starts, from the header's own self-relative pointer.
fn mawqi_jadwal(muashir: u32, adad: u32, tul: usize) -> Result<usize, SababRafd> {
    if muashir == 0 {
        // The loader treats a zero here as a null pointer and never touches the
        // array. With buckets declared, that is a file whose table is missing.
        if adad == 0 {
            return Ok(TUL_TARWISA);
        }
        return Err(SababRafd::Talif {
            sabab: format!("{adad} buckets are declared and the pointer to them is null"),
        });
    }
    let bidaya = hall_muashir(MAWQI_MUASHIR, muashir).ok_or_else(|| SababRafd::Talif {
        sabab: "the pointer to the bucket array is past the end of addressable memory".to_owned(),
    })?;
    if bidaya < TUL_TARWISA || bidaya > tul {
        return Err(SababRafd::Talif {
            sabab: format!(
                "the bucket array is declared at {bidaya:#x}, outside \
                 [{TUL_TARWISA:#x}, {tul:#x}]"
            ),
        });
    }
    Ok(bidaya)
}

/// Where the bucket array ends, refusing one that runs past the file.
fn nihayat_jadwal(bidaya: usize, adad: u32, tul: usize) -> Result<usize, SababRafd> {
    let hajm = hajm_usize(adad)
        .checked_mul(HAJM_MUDKHAL)
        .and_then(|hajm| bidaya.checked_add(hajm));
    let Some(nihaya) = hajm else {
        return Err(SababRafd::Talif {
            sabab: format!("{adad} buckets do not fit in addressable memory"),
        });
    };
    if nihaya > tul {
        return Err(SababRafd::Talif {
            sabab: format!("{adad} buckets need {nihaya} bytes and the file has {tul}"),
        });
    }
    Ok(nihaya)
}

/// The address of bucket `fahras`'s pointer field.
///
/// The hash sits four bytes in front of it. This names the pointer rather than
/// the bucket because every pointer in the format is relative to the field that
/// holds it, so the field's own address is the number the arithmetic needs.
fn mawqi_haql(bidayat_jadwal: usize, fahras: usize) -> Option<usize> {
    bidayat_jadwal
        .checked_add(fahras.checked_mul(HAJM_MUDKHAL)?)?
        .checked_add(4)
}

/// A stored pointer resolved against the field that holds it.
///
/// `&field + value + 1`. The bias is the game's, not this module's: its
/// serializer subtracts `field_address + 1` from every pointer on the way out.
fn hall_muashir(mawqi_haql: usize, qeema: u32) -> Option<usize> {
    mawqi_haql.checked_add(hajm_usize(qeema))?.checked_add(1)
}

/// The inverse: the value to store for a pointer from `mawqi_haql` to `hadaf`.
fn taqyeed_muashir(mawqi_haql: usize, hadaf: usize) -> Option<u32> {
    u32::try_from(hadaf.checked_sub(mawqi_haql)?.checked_sub(1)?).ok()
}

/// The refusal a pointer that will not fit produces.
fn hadd_muashir(qeema: u64) -> SababRafd {
    SababRafd::TajawuzHadd {
        hadd: "a DICT self-relative pointer".to_owned(),
        qeema,
        saqf: u64::from(u32::MAX),
    }
}

/// A little-endian `u32` at `mawqi`, or [`None`] when it does not fit.
fn kalima(bayt: &[u8], mawqi: usize) -> Option<u32> {
    let nihaya = mawqi.checked_add(4)?;
    let qita = bayt.get(mawqi..nihaya)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(qita).ok()?))
}

/// A `u32` as a length, saturating where `usize` is narrower.
fn hajm_usize(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(usize::MAX)
}

/// Pulls every string out of a game's `DICT` dictionaries.
///
/// Walks the game's root for `.dct` files, reads each one, and records what it
/// read and what it refused. A file that does not carry the signature is
/// refused by name rather than skipped in silence, because `.dct` is a common
/// enough extension that a game carrying an unrelated one deserves the sentence
/// saying so.
#[must_use]
pub fn istakhrij(jidhr: &Path) -> (JadwalNusus, TaqreerRafd) {
    let mut jadwal = JadwalNusus::jadeed();
    let mut taqreer = TaqreerRafd::jadeed();

    for masar in masarat_lil_mash(jidhr) {
        let hawiya = nisbi(jidhr, &masar);
        match qira_malaf(&masar) {
            Ok(bayt) => sajjil_qamus(&mut jadwal, &mut taqreer, &hawiya, &bayt),
            Err(sabab) => taqreer.sajjil(hawiya, None, sabab),
        }
    }

    (jadwal, taqreer)
}

/// Every `.dct` under the game's root, sorted so two runs agree.
fn masarat_lil_mash(jidhr: &Path) -> Vec<PathBuf> {
    let mut masarat = Vec::new();
    let sayr = walkdir::WalkDir::new(jidhr)
        .max_depth(UMQ_MASH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .take(AQSA_MALAFAT);

    for madkhal in sayr {
        if !madkhal.file_type().is_file() {
            continue;
        }
        let masar = madkhal.into_path();
        let lahiqa = masar
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_ascii_lowercase);
        if lahiqa.as_deref() == Some("dct") {
            masarat.push(masar);
        }
    }

    masarat.sort();
    masarat
}

/// Reads one dictionary into the table, or records why it was not read.
fn sajjil_qamus(jadwal: &mut JadwalNusus, taqreer: &mut TaqreerRafd, hawiya: &str, bayt: &[u8]) {
    let qamus = match Qamus::iqra(bayt) {
        Ok(qamus) => qamus,
        Err(sabab) => {
            taqreer.sajjil(hawiya.to_owned(), None, sabab);
            return;
        },
    };

    // Refused whole, never in part. Two of Resident Evil 4's dictionaries hold
    // a handful of Big5 strings among UTF-8 ones, and there is nothing in the
    // file that says so — no encoding field, no per-string flag. Naming a code
    // page for them would be a guess, and extracting the rest would hand a
    // translator a table that is missing strings it cannot see are missing.
    let ghayr_salih = qamus
        .madakhil()
        .iter()
        .filter(|madkhal| madkhal.mashghul())
        .filter(|madkhal| qamus.bayt(**madkhal).is_some() && qamus.nass(**madkhal).is_none())
        .count();
    if ghayr_salih > 0 {
        taqreer.sajjil(
            hawiya.to_owned(),
            None,
            SababRafd::HadUlBina {
                wujid: format!("{hawiya} ({ghayr_salih} of its strings are not UTF-8)"),
                sabab: "the dictionary declares no encoding and this one mixes UTF-8 with a \
                        legacy code page, so the text cannot be decoded without guessing"
                    .to_owned(),
            },
        );
        return;
    }

    let lugha = lugha_min_masar(hawiya);
    let mut adad = 0_usize;
    let mut farigha = 0_usize;
    for madkhal in qamus.madakhil() {
        if !madkhal.mashghul() {
            continue;
        }
        let Some(khaam) = qamus.nass(*madkhal) else {
            continue;
        };
        if khaam.trim().is_empty() {
            farigha = farigha.saturating_add(1);
            continue;
        }
        adif_nass(jadwal, hawiya, *madkhal, khaam);
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya.to_owned(), None, SababRafd::BilaNusus);
        return;
    }
    taqreer.sajjil_qira(
        hawiya.to_owned(),
        adad,
        format!(
            "Capcom DICT {lugha} dictionary, {} buckets, {adad} strings, {farigha} empty",
            qamus.adad()
        ),
    );
}

/// Adds one bucket's string to the table.
fn adif_nass(jadwal: &mut JadwalNusus, hawiya: &str, madkhal: MudkhalQamus, khaam: &str) {
    // The hash is the engine's own key, so it goes in `miftah_muharrik` and the
    // identity stops depending on the text: a line the publisher rewrites under
    // an unchanged key comes back as *changed* rather than as removed and added,
    // and the translator keeps their work. The key's plaintext name is not
    // recoverable from the file — a CRC is not invertible — so the hash is
    // written as the number it is rather than as a name this module invented.
    //
    // The language is not folded into the key because the container path already
    // separates the eight files, and `MawqiNass::huwiya` mixes it in.
    let miftah = format!("{:#010x}", madkhal.basma());
    let mawqi = MawqiNass {
        hawiya: hawiya.to_owned(),
        asl: None,
        mawqi: miftah.clone(),
        haql: None,
        miftah_muharrik: Some(miftah),
    };
    jadwal.adif(ansha_mudkhal(
        TalabMudkhal::jadeed(mawqi, khaam).bi_nizam_tawtin(),
    ));
}

/// The language a dictionary's file name names.
///
/// Resident Evil 4 spells them `ENGLISH_WIN32.dct` and `CHINESE_S_WIN32.dct`, so
/// the platform suffix comes off and whatever is in front of it is the language.
/// A file named anything else answers with its own stem, which is the honest
/// thing to show a user rather than a language this module made up.
fn lugha_min_masar(hawiya: &str) -> String {
    let ism = hawiya.rsplit('/').next().unwrap_or(hawiya);
    let jidh = ism.rsplit_once('.').map_or(ism, |(jidh, _)| jidh);
    match jidh.rsplit_once('_') {
        Some((lugha, mansa)) if mansa.eq_ignore_ascii_case("WIN32") && !lugha.is_empty() => {
            lugha.to_owned()
        },
        _ => jidh.to_owned(),
    }
}

/// A path relative to the game's root, with forward slashes on every platform.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    let juz = masar.strip_prefix(jidhr).unwrap_or(masar);
    juz.components()
        .map(|qism| qism.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Reads a whole dictionary, refusing one above [`AQSA_MALAF`] before reserving.
fn qira_malaf(masar: &Path) -> Result<Vec<u8>, SababRafd> {
    let bayanat = std::fs::metadata(masar).map_err(|sabab| SababRafd::TaadhurQira {
        sabab: sabab.to_string(),
    })?;
    if bayanat.len() > AQSA_MALAF {
        return Err(SababRafd::TajawuzHadd {
            hadd: "a DICT dictionary".to_owned(),
            qeema: bayanat.len(),
            saqf: AQSA_MALAF,
        });
    }
    std::fs::read(masar).map_err(|sabab| SababRafd::TaadhurQira {
        sabab: sabab.to_string(),
    })
}
