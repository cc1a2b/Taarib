//! التصادم — when Taarib's own work and a patch somebody else made both want
//! one game.
//!
//! Two translations of one game cannot be installed at once, and the reason is
//! not politeness. A patch that replaces a game's text files and a patch that
//! replaces the same game's text files write over each other; a loader at
//! `dinput8.dll` and a loader at `version.dll` are two processes hooking one
//! game; and — the case this module exists for — RTEA's own instructions tell
//! the user to delete `version.dll`, which is the file Taarib's loader is
//! published as. Installing one over the other does not produce a game with two
//! patches. It produces a game with one patch, a directory full of orphaned
//! files belonging to the other, and two manifests that each believe they
//! describe the game.
//!
//! So the refusal runs in **both** directions, before either install writes
//! anything, and it names what it found rather than saying "a conflict was
//! detected".
//!
//! ## Why a name is sometimes proof and sometimes is not
//!
//! The same rule [`crate::wukala`] settles loader slots with. `lml/`,
//! `lml.ini`, `ScriptHookRDR2.dll`, `vfs.asi`, `ModManager.Core.dll` and
//! `redteamassets/` are files no game ships and no general-purpose mod creates;
//! one of them beside a game is a third-party translation loader, with nothing
//! to infer. `dinput8.dll` is not: `re4_tweaks` is installed under exactly that
//! name, and a survey that cried "another translation is installed" over it
//! would be a survey that refuses to install Taarib into a game somebody has
//! merely modded. It is therefore reported as corroboration and never as proof,
//! which is the same split [`crate::wukala::WUKALA_MUSHTARAKA`] exists for.
//!
//! On Taarib's own side the strongest evidence is not a file name at all: it is
//! a manifest. [`crate::bayan::Tathbeet::mawjud`] answers "is one of Taarib's
//! two installations in this game's backup directory" from a record Taarib
//! wrote itself, and a third-party entry installed through Taarib leaves the
//! same kind of record under [`crate::khariji::MUJALLAD_KHARIJI`]. Those are
//! facts. The game-directory markers are what catches the other case — a patch
//! installed by its own installer, which left no record anywhere.
//!
//! ## Case
//!
//! Every lookup folds case per component, through
//! [`taarib_kashf::beea::hall_bila_hala`]. The games this applies to are Windows
//! games, the archives were packed on Windows, and on a Proton install the host
//! filesystem underneath is case-sensitive while the game is not: a check for
//! `lml` that missed `LML` would report a clean game and then install over one.

use std::path::{Path, PathBuf};

use taarib_kashf::beea::hall_bila_hala;
use taarib_usus::khata::Khutwa;

use crate::bayan::{NawTathbeet, Tathbeet};
use crate::khariji::kharijiyat_mathbita;
use crate::khata::{KhataTathbeet, NatijatTathbeet};
use crate::mawdi::MUJALLAD_TAARIB;

/// Files and directories only a third-party translation loader puts in a game.
///
/// Relative to the game root. Membership rule: a name no publisher ships and no
/// general-purpose mod creates, so the name alone settles the question.
pub const ALAMAT_KHARIJIYA: [&str; 6] = [
    "lml",
    "lml.ini",
    "ScriptHookRDR2.dll",
    "vfs.asi",
    "ModManager.Core.dll",
    "redteamassets",
];

/// Names a third-party loader set takes that something else may legitimately own.
///
/// Reported only beside at least one of [`ALAMAT_KHARIJIYA`]. `dinput8.dll` is
/// the whole of this list and the whole of its reason: it is RTEA's loader slot
/// and it is also `re4_tweaks`', and a game that merely has a mod in it is not a
/// game with another translation in it.
pub const ALAMAT_MUSHTARAKA: [&str; 1] = ["dinput8.dll"];

/// What proves Taarib's own work is inside a game directory.
///
/// The package directory every engine's install places content in, and the
/// plugin directory the Unity takeover is deployed to. Both are Taarib's by
/// name; neither is a name anything else uses.
pub const ALAMAT_TAARIB: [&str; 2] = [MUJALLAD_TAARIB, "BepInEx/plugins/Taarib"];

/// Which side of a collision is already installed, and therefore which was
/// refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JihatTasadum {
    /// A patch somebody else made is in the game, and a Taarib install was
    /// refused.
    Khariji,
    /// Taarib's own patch is in the game, and a third-party install was refused.
    Taarib,
}

impl JihatTasadum {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Khariji => "a third-party patch",
            Self::Taarib => "a Taarib patch",
        }
    }

    /// What is installed, as a sentence in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Khariji => {
                "a translation somebody else made is already installed, by its own installer or \
                 by Taarib"
            },
            Self::Taarib => "Taarib's own patch is already installed",
        }
    }

    /// The same, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Khariji => "في هذه اللعبة تعريبٌ من صنع غير تعريب، ثبّته مثبِّته أو ثبّته تعريب",
            Self::Taarib => "في هذه اللعبة رقعة تعريب نفسه",
        }
    }

    /// What has to happen before the refused install can proceed, in English.
    #[must_use]
    pub const fn amal_injilizi(self) -> &'static str {
        match self {
            Self::Khariji => {
                "Two translations of one game overwrite each other's files, and that patch's own \
                 instructions tell you to delete the file Taarib's loader is published as. Remove \
                 it first — through Taarib if Taarib installed it, which puts the game back byte \
                 for byte, or through whatever installed it otherwise. Nothing was written."
            },
            Self::Taarib => {
                "Uninstall the Taarib patch first — that restores the game exactly, and this \
                 patch can then be installed over a clean game. Nothing was written, and nothing \
                 of Taarib's was touched."
            },
        }
    }

    /// The same, in Arabic.
    #[must_use]
    pub const fn amal_arabi(self) -> &'static str {
        match self {
            Self::Khariji => {
                "تعريبان للعبة واحدة يكتب كلٌّ منهما فوق ملفّات الآخر، وتعليمات تلك الرقعة نفسها \
                 تأمر بحذف الملفّ الذي يُنشر محمِّل تعريب باسمه. أزِلها أوّلًا — من تعريب إن كان \
                 هو من ثبّتها، فتعود اللعبة بايتًا بايت، أو ممّا ثبّتها وإلّا. لم يُكتب شيء."
            },
            Self::Taarib => {
                "أزِل رقعة تعريب أوّلًا: الإزالة تعيد اللعبة إلى حالها تمامًا، ثمّ تُثبَّت هذه \
                 الرقعة على لعبة نظيفة. لم يُكتب شيء ولم يُمَسّ شيء ممّا ثبّته تعريب."
            },
        }
    }

    /// The action offered beside the refusal.
    ///
    /// Taarib's own patch comes off with Taarib's own button. Somebody else's
    /// mod does not: removing it is their decision, this product has no
    /// authority over their files, and a button that deleted them would be
    /// taking that decision away.
    #[must_use]
    pub const fn khutwa(self) -> Khutwa {
        match self {
            Self::Khariji => Khutwa::LaShay,
            Self::Taarib => Khutwa::IlghaTathbeet,
        }
    }
}

/// One reading of a game: which side is installed and what proves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtharTasadum {
    /// Which side was found.
    pub jiha: JihatTasadum,
    /// What was found, in the order it was looked for.
    pub alamat: Vec<String>,
}

impl AtharTasadum {
    /// The refusal this reading warrants.
    #[must_use]
    pub fn khata(self, jidhr_luba: &Path) -> KhataTathbeet {
        KhataTathbeet::TasadumRuqaa {
            jiha: self.jiha,
            jidhr: jidhr_luba.to_path_buf(),
            alamat: self.alamat,
        }
    }
}

/// What says a patch somebody else made is installed in this game, if anything.
///
/// Two sources, in the order that costs least: a third-party manifest under the
/// game's backup directory, which is a record Taarib wrote and therefore a fact;
/// and then the game's own directory, which is the only thing that can catch a
/// patch installed by its own installer.
#[must_use]
pub fn athar_khariji(jidhr_luba: &Path, jidhr_nusakh: &Path) -> Option<AtharTasadum> {
    let mut alamat: Vec<String> = kharijiyat_mathbita(jidhr_nusakh)
        .iter()
        .map(|masar| {
            format!(
                "{} (a third-party install Taarib recorded)",
                masar.display()
            )
        })
        .collect();

    let khassa: Vec<&str> = ALAMAT_KHARIJIYA
        .into_iter()
        .filter(|alama| mawjud(jidhr_luba, alama))
        .collect();
    if alamat.is_empty() && khassa.is_empty() {
        // `dinput8.dll` on its own is a mod, not a translation. Reporting it
        // here would refuse Taarib on every game that merely has one.
        return None;
    }

    alamat.extend(khassa.iter().map(|alama| (*alama).to_owned()));
    alamat.extend(
        ALAMAT_MUSHTARAKA
            .into_iter()
            .filter(|alama| mawjud(jidhr_luba, alama))
            .map(str::to_owned),
    );
    Some(AtharTasadum {
        jiha: JihatTasadum::Khariji,
        alamat,
    })
}

/// What says Taarib's own patch is installed in this game, if anything.
///
/// The manifests first, for the same reason: they are what Taarib wrote about
/// itself. The directory markers catch a game whose backup directory has been
/// moved or deleted while the patch is still in place.
#[must_use]
pub fn athar_taarib(jidhr_luba: &Path, jidhr_nusakh: &Path) -> Option<AtharTasadum> {
    let mut alamat: Vec<String> = NawTathbeet::KULL
        .into_iter()
        .filter(|naw| Tathbeet::mawjud(jidhr_nusakh, *naw))
        .map(|naw| {
            format!(
                "{} (the {} installation's manifest)",
                jidhr_nusakh.join(naw.ism_bayan()).display(),
                naw.ism()
            )
        })
        .collect();

    alamat.extend(
        ALAMAT_TAARIB
            .into_iter()
            .filter(|alama| mawjud(jidhr_luba, alama))
            .map(str::to_owned),
    );

    if alamat.is_empty() {
        return None;
    }
    Some(AtharTasadum {
        jiha: JihatTasadum::Taarib,
        alamat,
    })
}

/// Refuses when a patch somebody else made is already in this game.
///
/// Called before any Taarib install, and again before a third-party install —
/// the second is what stops two third-party entries, or one entry installed
/// twice, from landing on top of each other.
///
/// # Errors
///
/// [`KhataTathbeet::TasadumRuqaa`] naming every marker that was found and which
/// one has to go first.
pub fn la_yatasadam_maa_khariji(jidhr_luba: &Path, jidhr_nusakh: &Path) -> NatijatTathbeet<()> {
    match athar_khariji(jidhr_luba, jidhr_nusakh) {
        Some(athar) => Err(athar.khata(jidhr_luba)),
        None => Ok(()),
    }
}

/// Refuses when Taarib's own patch is already in this game.
///
/// Called before a third-party install. The remedy here is Taarib's own
/// uninstall, which restores the game exactly, so the third-party patch then
/// installs over a clean game rather than over Taarib's files.
///
/// # Errors
///
/// [`KhataTathbeet::TasadumRuqaa`] naming the manifests and markers found.
pub fn la_yatasadam_maa_taarib(jidhr_luba: &Path, jidhr_nusakh: &Path) -> NatijatTathbeet<()> {
    match athar_taarib(jidhr_luba, jidhr_nusakh) {
        Some(athar) => Err(athar.khata(jidhr_luba)),
        None => Ok(()),
    }
}

/// Whether one marker is in a game directory, folding case per component.
///
/// `symlink_metadata`, so a marker that is a dangling link still reads as
/// something being there: a broken `lml` link is evidence of an install just as
/// much as a directory is.
fn mawjud(jidhr_luba: &Path, nisbi: &str) -> bool {
    hall_bila_hala(jidhr_luba, Path::new(nisbi))
        .symlink_metadata()
        .is_ok()
}

/// Every marker of one side that is present, for a report that is not a refusal.
///
/// The library view shows what is in a game before anybody presses anything, and
/// it needs the same answer the refusal is built from rather than a second
/// reading that could disagree with it.
#[must_use]
pub fn masah_tasadum(jidhr_luba: &Path, jidhr_nusakh: &Path) -> Vec<AtharTasadum> {
    let mut athar = Vec::with_capacity(2);
    athar.extend(athar_taarib(jidhr_luba, jidhr_nusakh));
    athar.extend(athar_khariji(jidhr_luba, jidhr_nusakh));
    athar
}

/// The game directories a marker sweep reads, for a caller building a report.
#[must_use]
pub fn alamat_kull() -> Vec<PathBuf> {
    ALAMAT_KHARIJIYA
        .into_iter()
        .chain(ALAMAT_MUSHTARAKA)
        .chain(ALAMAT_TAARIB)
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod ikhtibarat {
    use std::fs;
    use std::path::Path;

    use super::{JihatTasadum, athar_khariji, athar_taarib, masah_tasadum};

    fn ansha(jidhr: &Path, nisbi: &str) -> Result<(), std::io::Error> {
        let masar = jidhr.join(nisbi);
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid)?;
        }
        fs::write(masar, b"x")
    }

    /// The whole reason the shared list exists: a game with `re4_tweaks` in it is
    /// a modded game, not a game with another translation in it, and Taarib
    /// installs into it.
    #[test]
    fn dinput8_wahdahu_laysa_tariban() -> Result<(), std::io::Error> {
        let mujallad = tempfile::tempdir()?;
        let luba = mujallad.path().join("luba");
        let nusakh = mujallad.path().join("nusakh");
        fs::create_dir_all(&luba)?;
        fs::create_dir_all(&nusakh)?;

        ansha(&luba, "dinput8.dll")?;
        ansha(&luba, "dinput8.ini")?;
        assert_eq!(athar_khariji(&luba, &nusakh), None);

        // One name that proves it, and the shared name is then reported beside
        // it rather than instead of it.
        ansha(&luba, "vfs.asi")?;
        let athar = athar_khariji(&luba, &nusakh);
        assert!(athar.as_ref().is_some_and(|athar| {
            athar.jiha == JihatTasadum::Khariji
                && athar.alamat.iter().any(|alama| alama == "vfs.asi")
                && athar.alamat.iter().any(|alama| alama == "dinput8.dll")
        }));
        Ok(())
    }

    /// The host under Proton is case-sensitive and the game is not.
    #[test]
    fn al_alamat_tuqra_bila_hala() -> Result<(), std::io::Error> {
        let mujallad = tempfile::tempdir()?;
        let luba = mujallad.path().join("luba");
        let nusakh = mujallad.path().join("nusakh");
        fs::create_dir_all(luba.join("LML"))?;
        fs::create_dir_all(&nusakh)?;
        assert!(athar_khariji(&luba, &nusakh).is_some());
        Ok(())
    }

    #[test]
    fn athar_taarib_yaqra_al_bayan_wal_mujallad() -> Result<(), std::io::Error> {
        let mujallad = tempfile::tempdir()?;
        let luba = mujallad.path().join("luba");
        let nusakh = mujallad.path().join("nusakh");
        fs::create_dir_all(&luba)?;
        fs::create_dir_all(&nusakh)?;
        assert_eq!(athar_taarib(&luba, &nusakh), None);

        fs::write(nusakh.join("bayan-nass.json"), b"{}")?;
        let athar = athar_taarib(&luba, &nusakh);
        assert!(
            athar
                .as_ref()
                .is_some_and(|athar| athar.jiha == JihatTasadum::Taarib
                    && athar.alamat.iter().any(|alama| alama.contains("text")))
        );

        fs::create_dir_all(luba.join("taarib"))?;
        let athar = athar_taarib(&luba, &nusakh);
        assert!(athar.is_some_and(|athar| athar.alamat.len() == 2));
        Ok(())
    }

    /// A game holding both is reported as holding both, rather than as whichever
    /// the sweep happened to look for first.
    #[test]
    fn al_masah_yajma_al_jihatayn() -> Result<(), std::io::Error> {
        let mujallad = tempfile::tempdir()?;
        let luba = mujallad.path().join("luba");
        let nusakh = mujallad.path().join("nusakh");
        fs::create_dir_all(&luba)?;
        fs::create_dir_all(&nusakh)?;
        assert!(masah_tasadum(&luba, &nusakh).is_empty());

        ansha(&luba, "lml.ini")?;
        fs::create_dir_all(luba.join("taarib"))?;
        let athar = masah_tasadum(&luba, &nusakh);
        assert_eq!(athar.len(), 2);
        assert_eq!(
            athar.first().map(|athar| athar.jiha),
            Some(JihatTasadum::Taarib)
        );
        assert_eq!(
            athar.get(1).map(|athar| athar.jiha),
            Some(JihatTasadum::Khariji)
        );
        Ok(())
    }
}
