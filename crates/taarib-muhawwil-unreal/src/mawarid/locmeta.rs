//! اللوكميتا — `.locmeta`, the manifest that says which cultures a game has.
//!
//! Beside every compiled `.locres` sits one `.locmeta` for the whole
//! localization target. It is the smallest file in this module and the one
//! `alam` reads first, because it answers the question culture registration
//! starts from: **which cultures does this game already ship, and which of them
//! does it consider native?**
//!
//! That matters more than its size suggests. Adding Arabic to a game means
//! adding a culture the game's own language list does not have, and the two
//! things that decide how to do it are both here. The native culture is the one
//! the engine falls back to when a lookup misses, so it is the text a player
//! sees when a translation is incomplete. The compiled culture list is what the
//! engine enumerates when it builds a language menu, so a culture absent from it
//! is a culture that exists on disk and never appears in the game's own UI.
//!
//! ## The file, byte for byte
//!
//! ```text
//! MawridLocmeta — little-endian
//!
//!   offset  size  field            type       in
//!        0    16  sihr             [u8;16]    v0 v1   the magic GUID
//!       16     1  isdar            u8         v0 v1   0 or 1
//!       17   var  thaqafa_asliya   FString    v0 v1   the native culture, e.g. "en"
//!        -   var  masar_asli       FString    v0 v1   the native .locres, project-relative
//!        -     4  adad_thaqafat    i32           v1   how many compiled cultures follow
//!        -   var  thaqafa          FString       v1   one culture name, repeated
//! ```
//!
//! Two versions, and the difference is one array:
//!
//! * **Version 0** ([`IsdarLocmeta::Ibtidai`]) stops after the native `.locres`
//!   path. A game with one of these has no compiled culture list anywhere in the
//!   file, and the only way to learn what it ships is to look at the directories
//!   beside it.
//! * **Version 1** ([`IsdarLocmeta::MaThaqafat`]) appends the list. This is what
//!   every engine version in current use writes.
//!
//! The layout is self-consistent in the sense that matters for a reader: every
//! field is either fixed-width or length-prefixed, nothing is offset-addressed,
//! and the file therefore ends exactly where the last field ends. Anything after
//! that is a region this build does not interpret — it is preserved in `dhayl`
//! and written back verbatim rather than dropped, and [`MawridLocmeta::mutasiq`]
//! reports whether it was empty, which it is in every file Unreal writes.
//!
//! ## Why the culture list is not simply rewritten
//!
//! [`MawridLocmeta::adif_thaqafa`] appends a culture and does nothing else. It
//! does not reorder the list, does not normalize the names already in it, and
//! does not touch the native culture — because each of those is a decision about
//! the game's behaviour rather than about its file. Reordering changes which
//! culture the engine picks when a player's system locale matches two entries;
//! normalizing `pt-BR` to `pt_BR` changes whether a lookup matches at all;
//! changing the native culture changes what every untranslated string falls back
//! to. Taarib adds `ar` to the end of a list and leaves the rest of the game's
//! own decisions where the developer put them.
//!
//! Version 0 has nowhere to put a culture, so [`MawridLocmeta::adif_thaqafa`]
//! refuses on it rather than promoting the file to version 1. Promoting would
//! write a version byte the game's own engine build may predate, which turns a
//! missing menu entry into a file the engine will not load.

use std::path::{Path, PathBuf};

use super::{
    Katib, Mawrid, NassMukhazzan, Qari, adad_musir, iqra_malaf, sammi_masar, tahaqquq_adad,
    uktub_malaf,
};
use crate::khata::{KhataUnreal, tul_u64};

/// `FTextLocalizationMetaDataResource::MagicNumber`, as the engine declares it.
///
/// `FGuid(0xA14CEE4F, 0x83554868, 0xBD464C6C, 0x7C50DA70)` — the value Unreal
/// prints as `A14CEE4F-83554868-BD464C6C-7C50DA70`. Kept as the four words the
/// engine actually writes rather than as sixteen bytes, so that this constant
/// can be checked against Unreal's source by reading it, and so that the byte
/// order is produced by [`sihr_min_kalimat`] instead of by hand.
const KALIMAT_SIHR: [u32; 4] = [0xA14C_EE4F, 0x8355_4868, 0xBD46_4C6C, 0x7C50_DA70];

/// The sixteen bytes every `.locmeta` begins with.
///
/// An `FGuid` is serialized as its four `uint32` fields in order, **each one
/// little-endian**, so every word lands in the file with its own bytes reversed.
/// The bytes are therefore not the printed hex read left to right:
///
/// ```text
///   A14CEE4F     83554868     BD464C6C     7C50DA70      as Unreal prints it
///   4F EE 4C A1  68 48 55 83  6C 4C 46 BD  70 DA 50 7C   as it sits on disk
/// ```
///
/// Derived rather than typed out, because typing it out is exactly how this
/// constant came to be wrong: sixteen literal bytes that nothing in the build
/// can check are sixteen chances to transpose a pair, and a transposed magic
/// does not crash — it makes every `.locmeta` in a real game an unknown format,
/// and it makes the file that names a game's cultures unreadable while every
/// message the user sees says the file is at fault.
///
/// Compared as bytes, never parsed as a GUID. It shares nothing with the
/// `.locres` magic, so the two formats cannot be confused for one another even
/// though they sit in the same directory with related names.
pub const SIHR: [u8; 16] = sihr_min_kalimat(KALIMAT_SIHR);

/// Serializes an `FGuid`'s four words the way `FArchive` writes them: in
/// declaration order, each one little-endian.
///
/// Destructured rather than indexed in a loop because this runs in a `const`
/// and the workspace does not index slices; four `to_le_bytes` and one array
/// literal say the same thing with the byte order visible in the result.
const fn sihr_min_kalimat(kalimat: [u32; 4]) -> [u8; 16] {
    let [awwal, thani, thalith, rabi] = kalimat;
    let [a1, a2, a3, a4] = awwal.to_le_bytes();
    let [b1, b2, b3, b4] = thani.to_le_bytes();
    let [j1, j2, j3, j4] = thalith.to_le_bytes();
    let [d1, d2, d3, d4] = rabi.to_le_bytes();
    [
        a1, a2, a3, a4, b1, b2, b3, b4, j1, j2, j3, j4, d1, d2, d3, d4,
    ]
}

/// The format's name in every refusal this module raises.
const ISM: &str = ".locmeta";

/// The highest version byte this build reads.
pub const AQSA_ISDAR: u32 = 1;

/// The largest number of compiled cultures one file may declare.
///
/// Four thousand and ninety-six. The complete IETF language-tag space a game
/// could plausibly ship is a few hundred entries; the most heavily localized
/// title this product has been pointed at declares thirty-one. The ceiling is
/// checked against the declared count before the vector is reserved, which is
/// the only reason it exists.
pub const AQSA_THAQAFAT: u64 = 4096;

/// The fewest bytes one culture name can occupy: an empty `FString`.
const AQALL_THAQAFA: u64 = 4;

/// Which of the two shapes a file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IsdarLocmeta {
    /// Version 0: the native culture and the native `.locres` path, and nothing
    /// else.
    Ibtidai,
    /// Version 1: adds the compiled culture list.
    MaThaqafat,
}

impl IsdarLocmeta {
    /// The byte the file carries.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Ibtidai => 0,
            Self::MaThaqafat => 1,
        }
    }

    /// The version for a byte, or [`None`] for one this build does not read.
    #[must_use]
    pub const fn min_raqm(raqm: u8) -> Option<Self> {
        match raqm {
            0 => Some(Self::Ibtidai),
            1 => Some(Self::MaThaqafat),
            _ => None,
        }
    }

    /// Whether the file carries a compiled culture list.
    #[must_use]
    pub const fn yahwi_thaqafat(self) -> bool {
        matches!(self, Self::MaThaqafat)
    }
}

/// A whole `.locmeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawridLocmeta {
    isdar: IsdarLocmeta,
    thaqafa_asliya: NassMukhazzan,
    masar_asli: NassMukhazzan,
    thaqafat: Vec<NassMukhazzan>,
    dhayl: Vec<u8>,
}

impl Mawrid for MawridLocmeta {
    const ISM: &'static str = ISM;

    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataUnreal> {
        let mut qari = Qari::jadeed(ISM, bayt);

        // Magic before version, and version before every other field, because
        // which fields exist is a property of the version. A reader that took
        // the culture list first and checked the version afterwards would have
        // read an array whose presence it guessed.
        let sihr: [u8; 16] = qari.iqra_masfufa("the magic")?;
        if sihr != SIHR {
            // No path here on purpose: this reader was handed bytes, not a
            // file. `min_malaf` fills the name in when it has one.
            return Err(KhataUnreal::SihrGhayrMutabaq {
                malaf: PathBuf::new(),
                ism: ISM,
            });
        }
        let raqm = qari.iqra_u8("the version byte")?;
        let isdar = IsdarLocmeta::min_raqm(raqm).ok_or_else(|| KhataUnreal::IsdarGhayrMadum {
            ism: ISM,
            wujid: u32::from(raqm),
            aqsa: AQSA_ISDAR,
        })?;

        let thaqafa_asliya = qari.iqra_nass("the native culture")?;
        let masar_asli = qari.iqra_nass("the native locres path")?;

        let thaqafat = if isdar.yahwi_thaqafat() {
            let muallan = qari.iqra_i32("the compiled culture count")?;
            let adad_muallan = adad_musir(ISM, "the compiled culture count", muallan)?;
            let adad = tahaqquq_adad(
                ISM,
                "the compiled culture count",
                adad_muallan,
                AQSA_THAQAFAT,
                AQALL_THAQAFA,
                qari.baqi(),
            )?;
            let mut thaqafat = Vec::with_capacity(adad);
            for _ in 0..adad {
                thaqafat.push(qari.iqra_nass("a compiled culture")?);
            }
            thaqafat
        } else {
            Vec::new()
        };

        let dhayl = qari.baqiya().to_vec();
        Ok(Self {
            isdar,
            thaqafa_asliya,
            masar_asli,
            thaqafat,
            dhayl,
        })
    }

    fn ila_bayt(&self) -> Result<Vec<u8>, KhataUnreal> {
        let siaa = 64usize.saturating_add(self.thaqafat.len().saturating_mul(16));
        let mut katib = Katib::bi_siaa(siaa);
        katib.uktub_bayt(&SIHR);
        katib.uktub_u8(self.isdar.raqm());
        katib.uktub_nass(ISM, "the native culture", &self.thaqafa_asliya)?;
        katib.uktub_nass(ISM, "the native locres path", &self.masar_asli)?;
        if self.isdar.yahwi_thaqafat() {
            let adad = i32::try_from(self.thaqafat.len()).map_err(|_| KhataUnreal::HajmMufrit {
                haql: "the compiled culture count",
                qeema: tul_u64(self.thaqafat.len()),
                saqf: AQSA_THAQAFAT,
            })?;
            katib.uktub_i32(adad);
            for thaqafa in &self.thaqafat {
                katib.uktub_nass(ISM, "a compiled culture", thaqafa)?;
            }
        }
        katib.uktub_bayt(&self.dhayl);
        Ok(katib.ila_vec())
    }
}

impl MawridLocmeta {
    /// A new manifest, for a target Taarib is creating rather than editing.
    ///
    /// The culture list starts empty even at version 1, because a manifest that
    /// claimed a compiled culture before the matching `.locres` had been written
    /// would describe a game state that does not exist yet.
    #[must_use]
    pub fn jadeed(isdar: IsdarLocmeta, thaqafa_asliya: &str, masar_asli: &str) -> Self {
        Self {
            isdar,
            thaqafa_asliya: NassMukhazzan::jadeed(thaqafa_asliya),
            masar_asli: NassMukhazzan::jadeed(masar_asli),
            thaqafat: Vec::new(),
            dhayl: Vec::new(),
        }
    }

    /// Reads a `.locmeta` from a path.
    ///
    /// **The convenience function.** Everything else works over `&[u8]`, because
    /// a `.locmeta` is found inside a `.pak` as often as it is found loose.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] naming the path, [`KhataUnreal::HajmMufrit`]
    /// when the file is above [`super::AQSA_MALAF`], and otherwise whatever
    /// [`Mawrid::min_bayt`] refuses.
    pub fn min_malaf(masar: &Path) -> Result<Self, KhataUnreal> {
        Self::min_bayt(&iqra_malaf(masar)?).map_err(|khata| sammi_masar(khata, masar))
    }

    /// Writes this manifest to a path.
    ///
    /// # Errors
    ///
    /// Whatever [`Mawrid::ila_bayt`] refuses, or [`KhataUnreal::KhataMalaf`]
    /// naming the path.
    pub fn ila_malaf(&self, masar: &Path) -> Result<(), KhataUnreal> {
        uktub_malaf(masar, &self.ila_bayt()?)
    }

    /// Which version this file is.
    #[must_use]
    pub const fn isdar(&self) -> IsdarLocmeta {
        self.isdar
    }

    /// The native culture's name — the one every missed lookup falls back to.
    #[must_use]
    pub fn thaqafa_asliya(&self) -> &str {
        self.thaqafa_asliya.nass()
    }

    /// The native culture's `.locres`, as the file spells it.
    ///
    /// A project-relative path with Unreal's own separators, returned unchanged.
    /// Interpreting it against a disk is the caller's business: this module does
    /// not know where the game is installed and is not the place to decide.
    #[must_use]
    pub fn masar_asli(&self) -> &str {
        self.masar_asli.nass()
    }

    /// The compiled cultures, in file order. Empty at version 0, which has none.
    #[must_use]
    pub fn thaqafat(&self) -> &[NassMukhazzan] {
        &self.thaqafat
    }

    /// The compiled culture names.
    ///
    /// **This is what `alam` reads** to learn which cultures the game already
    /// has, and therefore whether adding Arabic means adding an entry or
    /// replacing one.
    pub fn asma_thaqafat(&self) -> impl Iterator<Item = &str> + '_ {
        self.thaqafat.iter().map(NassMukhazzan::nass)
    }

    /// Whether a culture is already compiled, compared case-insensitively.
    ///
    /// Case-insensitively because culture names are IETF language tags, which
    /// are defined to be case-insensitive, and because games spell them both
    /// ways — `pt-BR` and `pt-br` name one culture and adding the second beside
    /// the first would give the engine two entries for one language.
    #[must_use]
    pub fn yahwi_thaqafa(&self, thaqafa: &str) -> bool {
        self.thaqafat
            .iter()
            .any(|mawjud| mawjud.nass().eq_ignore_ascii_case(thaqafa))
    }

    /// The bytes after the last field, preserved verbatim. Empty in every file
    /// Unreal writes.
    #[must_use]
    pub fn dhayl(&self) -> &[u8] {
        &self.dhayl
    }

    /// Whether the file was internally consistent in the ways a reader can
    /// check.
    ///
    /// Reported rather than enforced. The three conditions are: the file ended
    /// exactly where its last field ended, the native culture is not empty, and
    /// — at version 1, when the list is non-empty — the native culture is among
    /// the compiled ones. Each is true of every file Unreal writes and none of
    /// them is a rule the format states, so a file that fails one is worth
    /// surfacing to whoever is looking at the game and is not worth refusing to
    /// read.
    #[must_use]
    pub fn mutasiq(&self) -> bool {
        if !self.dhayl.is_empty() || self.thaqafa_asliya.khali() {
            return false;
        }
        if self.isdar.yahwi_thaqafat() && !self.thaqafat.is_empty() {
            return self.yahwi_thaqafa(self.thaqafa_asliya.nass());
        }
        true
    }

    /// Appends a culture to the compiled list, if it is not already there.
    ///
    /// Returns whether the list changed. Appends rather than inserts, and
    /// touches nothing else in the file — see this module's header for why the
    /// order of that list is the game's decision and not Taarib's.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] at version 0, which has no list to append
    /// to, and [`KhataUnreal::HajmMufrit`] when the list is already at
    /// [`AQSA_THAQAFAT`].
    pub fn adif_thaqafa(&mut self, thaqafa: &str) -> Result<bool, KhataUnreal> {
        if !self.isdar.yahwi_thaqafat() {
            return Err(KhataUnreal::MawridTalif {
                ism: ISM,
                haql: "the compiled culture list, which version 0 of this format has not got",
                qeema: u64::from(self.isdar.raqm()),
                hadd: u64::from(AQSA_ISDAR),
            });
        }
        if self.yahwi_thaqafa(thaqafa) {
            return Ok(false);
        }
        if tul_u64(self.thaqafat.len()) >= AQSA_THAQAFAT {
            return Err(KhataUnreal::HajmMufrit {
                haql: "the compiled culture count",
                qeema: tul_u64(self.thaqafat.len()).saturating_add(1),
                saqf: AQSA_THAQAFAT,
            });
        }
        self.thaqafat.push(NassMukhazzan::jadeed(thaqafa));
        Ok(true)
    }

    /// Removes a culture from the compiled list, if it is there.
    ///
    /// Returns whether the list changed. Present so that installing Taarib is
    /// reversible in the one file where the change is not simply a new file the
    /// user can delete.
    pub fn ihdhif_thaqafa(&mut self, thaqafa: &str) -> bool {
        let qabl = self.thaqafat.len();
        self.thaqafat
            .retain(|mawjud| !mawjud.nass().eq_ignore_ascii_case(thaqafa));
        self.thaqafat.len() != qabl
    }
}
