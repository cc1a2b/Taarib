//! الارتباط — what a patch declares itself compatible with, and how a client
//! decides whether it applies.
//!
//! ## Why a build identifier alone kills patches
//!
//! A storefront bumps a build identifier for a shader recompile, a new
//! storefront banner, a platform binary rebuilt against a newer runtime. None
//! of those touches a translatable string, and all of them break a patch that
//! declares only "I was built for build 14882031". This is the single most
//! common way a community translation dies: not because anybody stopped
//! maintaining it, but because the game moved a byte the patch never read.
//!
//! So a patch binds to two things. The launcher's identifier, which is exact
//! when it matches and worthless when the store moves it, and a [`Basma`] over
//! the files the patch was actually built from — which does not move when the
//! store rebuilds something the translation never touched.
//!
//! ## The recipe travels, because a fingerprint nobody can reproduce is a
//! number
//!
//! A fingerprint is only useful if the installer computes the same one. That
//! means the *selection of files* and the *order they are hashed in* are part
//! of the format, not part of whichever build of the compiler produced them.
//! [`MukhattatBasma`] is that selection, it ships inside the package, and
//! [`MukhattatBasma::ihsab`] is the one implementation both sides run.
//!
//! ## The recipe is untrusted input on the installer's side
//!
//! On this machine the recipe is something the compiler just wrote. On a
//! player's machine it arrived inside a downloaded file and it is a list of
//! paths that will cause their computer to open files. A recipe naming
//! `../../../.ssh/id_ed25519` would turn an installer into a read primitive
//! driven by a stranger.
//!
//! [`MukhattatBasma`] therefore has no public fields, one validating
//! constructor, and a `Deserialize` that goes **through** that constructor
//! rather than around it. There is no sequence of calls and no stored document
//! that produces a recipe pointing outside the game directory.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_istikhraj::mashru::BayanIstikhraj;
use taarib_mustalahat::bina::{Basma, BinaId, MutabaqaBina};

use crate::bawwaba::KhattMujammaa;
use crate::khata::{KhataTarqee, SababMasar, tul_u64};

/// The version of the fingerprint recipe this build writes and reads.
///
/// Bumped whenever the hashing procedure changes in any way that moves the
/// digest — a different domain separator, a different field order, a different
/// length prefix. A recipe declaring a version this build does not know is
/// refused rather than run, because running it would produce a number that
/// looks like a fingerprint and equals nothing.
pub const ISDAR_MUKHATTAT: u32 = 1;

/// The domain separator the fingerprint starts with.
///
/// Present so that this digest can never collide with a plain BLAKE3 of a
/// file's contents, which is what [`KhattMujammaa`] computes a few lines away
/// in this same crate. Two hashes over related data with no separator between
/// their domains is how a value from one context gets accepted in another.
const FASIL: &[u8] = b"taarib.basmat-luba.v1\0";

/// Which files a build fingerprint covers, and in what order.
///
/// Validated on construction and on deserialization: every path is relative,
/// forward-slashed, free of `.` and `..` components, non-empty, and unique.
/// Sorted, so that two machines producing a recipe from the same project
/// produce the same recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "MukhattatKhaam", try_from = "MukhattatKhaam")]
pub struct MukhattatBasma {
    hawiyat: Vec<String>,
}

/// The recipe's wire form.
///
/// Exists only so that [`MukhattatBasma`] can derive `Deserialize` without the
/// derive becoming a constructor: serde builds this, and `TryFrom` is what
/// turns it into the validated type. A `Deserialize` that skipped the validator
/// would be a public constructor for a recipe that names any path on the disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MukhattatKhaam {
    isdar: u32,
    hawiyat: Vec<String>,
}

impl From<MukhattatBasma> for MukhattatKhaam {
    fn from(mukhattat: MukhattatBasma) -> Self {
        Self {
            isdar: ISDAR_MUKHATTAT,
            hawiyat: mukhattat.hawiyat,
        }
    }
}

impl TryFrom<MukhattatKhaam> for MukhattatBasma {
    type Error = KhataTarqee;

    fn try_from(khaam: MukhattatKhaam) -> Result<Self, Self::Error> {
        if khaam.isdar != ISDAR_MUKHATTAT {
            return Err(KhataTarqee::SighatHuzmaGhayrMaduma {
                wujid: khaam.isdar,
                madum: ISDAR_MUKHATTAT,
            });
        }
        Self::min_masarat(khaam.hawiyat.iter().map(String::as_str))
    }
}

impl MukhattatBasma {
    /// Builds a recipe from relative paths, refusing any that may not appear.
    ///
    /// # Errors
    ///
    /// [`KhataTarqee::MasarGhayrSalih`] with [`SababMasar::KharijAlJidhr`] for
    /// an absolute path, a path with a `..` component, a path with a Windows
    /// prefix or root, or an empty path. Duplicates are collapsed rather than
    /// refused: naming one container twice is a redundant recipe, not a
    /// dangerous one, and a fingerprint that hashed a file twice would simply
    /// be a different number for the same installation.
    pub fn min_masarat<'a, I: IntoIterator<Item = &'a str>>(
        masarat: I,
    ) -> Result<Self, KhataTarqee> {
        let mut farida: BTreeSet<String> = BTreeSet::new();
        for khaam in masarat {
            farida.insert(tabi_masar(khaam)?);
        }
        Ok(Self {
            hawiyat: farida.into_iter().collect(),
        })
    }

    /// Builds the recipe the extraction report implies.
    ///
    /// The containers extraction actually read, and nothing else. This is the
    /// strongest binding available: the patch is fingerprinted against exactly
    /// the files whose contents decided what it contains, so a store update
    /// that rewrites an audio bank leaves the fingerprint alone and a store
    /// update that rewrites the string table does not.
    ///
    /// # Errors
    ///
    /// Whatever [`MukhattatBasma::min_masarat`] refuses.
    pub fn min_bayan(bayan: &BayanIstikhraj) -> Result<Self, KhataTarqee> {
        Self::min_masarat(bayan.turuq.iter().map(|tareeqa| tareeqa.hawiya.as_str()))
    }

    /// The paths, in hashing order.
    #[must_use]
    pub fn hawiyat(&self) -> &[String] {
        &self.hawiyat
    }

    /// How many files the recipe names.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.hawiyat.len()
    }

    /// Whether the recipe names nothing.
    ///
    /// An empty recipe hashes to a constant, which would make every
    /// installation of every game produce the same fingerprint and every patch
    /// claim to match every build. [`ihsab`](Self::ihsab) refuses one; this is
    /// how a caller finds out before it gets there.
    #[must_use]
    pub const fn faragh(&self) -> bool {
        self.hawiyat.is_empty()
    }

    /// Computes the build fingerprint of an installation.
    ///
    /// Every field is length-prefixed before it is hashed. Without that, a file
    /// named `ab` next to a file named `c` and a file named `a` next to a file
    /// named `bc` would feed the hasher the same bytes — the classic
    /// concatenation ambiguity, and the reason two different installations
    /// could otherwise agree on a fingerprint.
    ///
    /// Returns the fingerprint and the number of files that went into it, which
    /// is what makes a fingerprint computed over a partially downloaded
    /// installation recognisable as one rather than as a mismatch.
    ///
    /// # Errors
    ///
    /// [`KhataTarqee::BayanNaqis`] when the recipe is empty,
    /// [`KhataTarqee::MalafIrtibatMafqud`] when a named file is not present —
    /// never skipped, because a fingerprint that quietly omits a missing
    /// container matches builds it has never seen — and
    /// [`KhataTarqee::KhataMalaf`] when one cannot be read.
    pub fn ihsab(&self, jidhr: &Path) -> Result<(Basma, u32), KhataTarqee> {
        if self.hawiyat.is_empty() {
            return Err(KhataTarqee::BayanNaqis {
                haql: "at least one file to fingerprint",
            });
        }

        let mut hashi = blake3::Hasher::new();
        hashi.update(FASIL);
        hashi.update(&tul_u64(self.hawiyat.len()).to_le_bytes());

        for nisbi in &self.hawiyat {
            let kamil = jidhr.join(nisbi);
            let bayanat = std::fs::metadata(&kamil).map_err(|sabab| {
                if sabab.kind() == std::io::ErrorKind::NotFound {
                    KhataTarqee::MalafIrtibatMafqud {
                        masar: kamil.clone(),
                    }
                } else {
                    KhataTarqee::KhataMalaf {
                        masar: kamil.clone(),
                        sabab,
                    }
                }
            })?;
            if !bayanat.is_file() {
                return Err(KhataTarqee::MalafIrtibatMafqud { masar: kamil });
            }

            hashi.update(&tul_u64(nisbi.len()).to_le_bytes());
            hashi.update(nisbi.as_bytes());
            hashi.update(&bayanat.len().to_le_bytes());
            hashi
                .update_mmap_rayon(&kamil)
                .map_err(|sabab| KhataTarqee::KhataMalaf {
                    masar: kamil.clone(),
                    sabab,
                })?;
        }

        let adad = u32::try_from(self.hawiyat.len()).unwrap_or(u32::MAX);
        Ok((Basma::min_bayt(*hashi.finalize().as_bytes()), adad))
    }
}

/// Normalizes one recipe path, refusing anything that leaves the game root.
///
/// Backslashes become forward slashes first, because a recipe written on
/// Windows and read on Linux must name the same file, and because a check that
/// ran before that conversion would let `..\..\etc` through on the platform
/// where it is a traversal.
fn tabi_masar(khaam: &str) -> Result<String, KhataTarqee> {
    let mubaddal = khaam.replace('\\', "/");
    let kharij = |masar: &str| KhataTarqee::MasarGhayrSalih {
        masar: PathBuf::from(masar),
        sabab: SababMasar::KharijAlJidhr,
    };
    if mubaddal.is_empty() {
        return Err(kharij(khaam));
    }

    let mut ajza: Vec<&str> = Vec::new();
    for juz in Path::new(&mubaddal).components() {
        match juz {
            Component::Normal(ism) => {
                let Some(nass) = ism.to_str() else {
                    return Err(KhataTarqee::MasarGhayrSalih {
                        masar: PathBuf::from(khaam),
                        sabab: SababMasar::GhayrUtf8,
                    });
                };
                ajza.push(nass);
            },
            // `./` is meaningless here and dropping it is not a repair: it
            // names the same file either way. Everything else is a way out.
            Component::CurDir => {},
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(kharij(khaam));
            },
        }
    }
    if ajza.is_empty() {
        return Err(kharij(khaam));
    }
    Ok(ajza.join("/"))
}

/// A range of launcher build identifiers a patch declares itself good for.
///
/// Applies **only** where the launcher's identifier is a number. Steam's
/// `buildid` is; GOG reports a build hash, Epic reports a manifest id, and
/// several launchers report nothing at all. A range over a hash is not a range,
/// and treating one as though it were would let a patch claim compatibility
/// with builds nobody looked at — which is the exact failure this whole module
/// exists to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NitaqBina {
    /// The lowest build identifier, inclusive.
    pub adna: u64,
    /// The highest, inclusive.
    pub aqsa: u64,
}

impl NitaqBina {
    /// Whether a launcher's identifier falls inside the range.
    ///
    /// An identifier that is not a plain decimal number falls outside every
    /// range, including one whose bounds would have contained it if it were
    /// parsed some other way.
    #[must_use]
    pub fn yashmal(self, manassa: &str) -> bool {
        manassa
            .parse::<u64>()
            .is_ok_and(|raqm| raqm >= self.adna && raqm <= self.aqsa)
    }
}

/// A bundled font, as the package records it.
///
/// Name, hash and flags — never bytes. The framework ships the fonts once; a
/// package carries the hash so an installer can prove the file on disk is the
/// one every precomputed layout in the package was shaped against. A font
/// swapped after publication reshapes every string, and without this the result
/// is a patch that renders subtly wrong with nothing to point at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasmatKhatt {
    /// The file name, as the installer will place it.
    pub ism: String,
    /// Its content fingerprint.
    pub basma: Basma,
    /// `ALAM_KHATT_*` from `taarib_ruqaa::jadawil`.
    pub alam: u16,
}

impl BasmatKhatt {
    /// The record for a font that passed the gate.
    #[must_use]
    pub fn min_mujammaa(khatt: &KhattMujammaa) -> Self {
        Self {
            ism: khatt.ism().to_owned(),
            basma: khatt.basma(),
            alam: 0,
        }
    }
}

/// Everything a patch declares itself compatible with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrtibatBina {
    /// Launcher build identifiers this patch was built and verified against.
    pub manassat: Vec<String>,
    /// Content fingerprints of the installations it was verified against.
    pub basmat: Vec<Basma>,
    /// How many files each of those fingerprints covered.
    pub adad_malaffat: u32,
    /// The recipe that produced them, so a client reproduces rather than
    /// trusts.
    pub mukhattat: MukhattatBasma,
    /// A declared range of numeric build identifiers, where the launcher has
    /// numeric identifiers at all.
    pub nitaq: Option<NitaqBina>,
    /// The fonts every layout in this package was shaped against.
    pub khutut: Vec<BasmatKhatt>,
}

impl IrtibatBina {
    /// Builds the binding for a compile.
    ///
    /// # Errors
    ///
    /// [`KhataTarqee::BilaBina`] when the project records neither a launcher
    /// build identifier nor a fingerprint. A patch that declares nothing
    /// matches nothing safely: the installer would have to either refuse it
    /// always, which makes it useless, or accept it always, which makes it
    /// dangerous.
    ///
    /// Whatever [`MukhattatBasma::min_bayan`] refuses.
    pub fn min_bayan(
        bayan: &BayanIstikhraj,
        khutut: &[KhattMujammaa],
        nitaq: Option<NitaqBina>,
    ) -> Result<Self, KhataTarqee> {
        let mukhattat = MukhattatBasma::min_bayan(bayan)?;
        let manassat: Vec<String> = bayan.bina_manassa.iter().cloned().collect();
        let basmat: Vec<Basma> = bayan.basmat_luba.iter().copied().collect();
        if manassat.is_empty() && basmat.is_empty() {
            return Err(KhataTarqee::BilaBina);
        }

        let adad = u32::try_from(mukhattat.adad()).unwrap_or(u32::MAX);
        Ok(Self {
            manassat,
            basmat,
            adad_malaffat: adad,
            mukhattat,
            nitaq,
            khutut: khutut.iter().map(BasmatKhatt::min_mujammaa).collect(),
        })
    }

    /// Judges a patch against the build a user actually has.
    #[must_use]
    pub fn ihkum(&self, mawjud: &BinaId) -> HukmIrtibat {
        let basma_tutabiq = self.basmat.contains(&mawjud.basma);
        let muarrif_yutabiq = mawjud
            .manassa
            .as_deref()
            .is_some_and(|manassa| self.manassat.iter().any(|wahid| wahid == manassa));

        let sabab = if muarrif_yutabiq && basma_tutabiq {
            SababMutabaqa::MuarrifWaBasma
        } else if basma_tutabiq {
            SababMutabaqa::BasmaFaqat
        } else if muarrif_yutabiq {
            SababMutabaqa::MuarrifBilaBasma
        } else if self
            .nitaq
            .zip(mawjud.manassa.as_deref())
            .is_some_and(|(nitaq, manassa)| nitaq.yashmal(manassa))
        {
            SababMutabaqa::DakhilNitaq
        } else {
            SababMutabaqa::BilaTatabuq
        };

        HukmIrtibat {
            sabab,
            naqis: mawjud.adad_malaffat < self.adad_malaffat,
        }
    }
}

/// Why a patch does or does not match a build.
///
/// The tier a user is shown is derived from this by
/// [`SababMutabaqa::mutabaqa`] rather than chosen alongside it, so there is no
/// state where the reason and the verdict disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SababMutabaqa {
    /// The launcher's identifier and the content fingerprint both match.
    MuarrifWaBasma,
    /// The identifier moved but the text-bearing files did not. This is the
    /// tier that keeps a patch alive through routine store updates, and the
    /// reason the fingerprint exists at all.
    BasmaFaqat,
    /// The identifier matches and the fingerprint does not: the store rewrote
    /// files without bumping its own build number. The patch was built for
    /// this build id and the bytes underneath it are not the bytes it was built
    /// from, so this is not an exact match however much it looks like one.
    MuarrifBilaBasma,
    /// Neither matches, but the identifier falls inside a range the contributor
    /// declared and took responsibility for.
    DakhilNitaq,
    /// Nothing matches.
    BilaTatabuq,
}

impl SababMutabaqa {
    /// The tier the interface shows.
    #[must_use]
    pub const fn mutabaqa(self) -> MutabaqaBina {
        match self {
            Self::MuarrifWaBasma => MutabaqaBina::Tamma,
            Self::BasmaFaqat => MutabaqaBina::Basma,
            Self::MuarrifBilaBasma | Self::DakhilNitaq => MutabaqaBina::Nitaq,
            Self::BilaTatabuq => MutabaqaBina::Ghayr,
        }
    }

    /// The sentence explaining the verdict, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::MuarrifWaBasma => "رقم البناء والبصمة متطابقان",
            Self::BasmaFaqat => "تغيّر رقم البناء ولم تتغيّر ملفّات النصوص",
            Self::MuarrifBilaBasma => "رقم البناء متطابق لكن ملفّات النصوص تغيّرت",
            Self::DakhilNitaq => "رقم البناء داخل مدى أعلنه المساهم",
            Self::BilaTatabuq => "لا رقم البناء ولا البصمة يطابقان هذه الرقعة",
        }
    }

    /// The same in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::MuarrifWaBasma => "the build id and the content fingerprint both match",
            Self::BasmaFaqat => "the build id moved but the text files did not",
            Self::MuarrifBilaBasma => "the build id matches but the text files changed",
            Self::DakhilNitaq => "the build id is inside a range the contributor declared",
            Self::BilaTatabuq => "neither the build id nor the fingerprint matches",
        }
    }
}

/// A compatibility verdict, with the reason that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HukmIrtibat {
    /// Why.
    pub sabab: SababMutabaqa,
    /// Whether the installation fingerprinted fewer files than the patch's
    /// recipe names.
    ///
    /// Almost always a partially downloaded or partially verified install
    /// rather than a real mismatch, and worth saying so: "your game is still
    /// downloading" is a far more useful sentence than "this patch does not
    /// match your version", and a client that cannot tell them apart says the
    /// wrong one.
    pub naqis: bool,
}

impl HukmIrtibat {
    /// The tier the interface shows.
    #[must_use]
    pub const fn mutabaqa(self) -> MutabaqaBina {
        self.sabab.mutabaqa()
    }

    /// Whether the client will install this at all.
    #[must_use]
    pub const fn qabila_lil_tathbeet(self) -> bool {
        self.mutabaqa().qabila_lil_tathbeet()
    }

    /// Whether installing requires an acknowledgement first.
    #[must_use]
    pub const fn yahtaj_iqrar(self) -> bool {
        self.mutabaqa().yahtaj_iqrar()
    }
}
