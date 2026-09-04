//! Destination path types: writes inside a game, and the closed set outside it.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::khata::KhataTathbeet;

/// The subdirectory of a game that belongs to Taarib.
pub const MUJALLAD_TAARIB: &str = "taarib";

/// A validated destination inside a game directory.
///
/// The only path type the pipeline accepts from package content. Holds the
/// normalized relative form; the absolute path is derived at write time.
/// `Deserialize` routes through [`WajhatLuba::jadeed`] so package metadata
/// cannot bypass validation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct WajhatLuba {
    nisbi: String,
}

impl WajhatLuba {
    /// Validates one relative destination.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MasarKharij`] for an empty path, a NUL byte, a
    /// non-UTF-8 component, a reserved Windows device name, a component ending
    /// in a dot or space, a parent or absolute component, or a path resolving
    /// to the root itself.
    pub fn jadeed(khaam: &str) -> Result<Self, KhataTathbeet> {
        let kharij = |sabab: &str| KhataTathbeet::MasarKharij {
            masar: PathBuf::from(khaam),
            jidhr: PathBuf::new(),
            sabab: sabab.to_owned(),
        };
        if khaam.is_empty() {
            return Err(kharij("empty path"));
        }
        if khaam.contains('\0') {
            return Err(kharij("path contains a NUL byte"));
        }

        // Convert before the walk: `..\\x` traverses on Windows and a check
        // run first would pass it.
        let mubaddal = khaam.replace('\\', "/");
        let mut ajza: Vec<&str> = Vec::new();
        for juz in Path::new(&mubaddal).components() {
            match juz {
                Component::Normal(ism) => {
                    let Some(nass) = ism.to_str() else {
                        return Err(kharij("a path component is not valid UTF-8"));
                    };
                    if ism_mahjuz(nass) {
                        return Err(kharij("a component is a reserved name or ends in a dot/space"));
                    }
                    ajza.push(nass);
                }
                Component::CurDir => {}
                Component::ParentDir => return Err(kharij("a parent component leaves the game")),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(kharij("an absolute path is not a destination inside a game"));
                }
            }
        }
        if ajza.is_empty() {
            return Err(kharij("path resolves to the game root itself"));
        }
        Ok(Self { nisbi: ajza.join("/") })
    }

    /// A destination under Taarib's own subdirectory of the game.
    ///
    /// # Errors
    ///
    /// As [`WajhatLuba::jadeed`], applied to the joined form.
    pub fn dakhil_taarib(khaam: &str) -> Result<Self, KhataTathbeet> {
        let dakhil = Self::jadeed(khaam)?;
        Ok(Self { nisbi: format!("{MUJALLAD_TAARIB}/{}", dakhil.nisbi) })
    }

    /// The normalized relative form the manifest stores.
    #[must_use]
    pub fn nisbi(&self) -> &str {
        &self.nisbi
    }

    /// The file name component.
    #[must_use]
    pub fn ism(&self) -> &str {
        self.nisbi.rsplit('/').next().unwrap_or(&self.nisbi)
    }
}

impl fmt::Display for WajhatLuba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.nisbi)
    }
}

impl From<WajhatLuba> for String {
    fn from(wajha: WajhatLuba) -> Self {
        wajha.nisbi
    }
}

impl TryFrom<String> for WajhatLuba {
    type Error = KhataTathbeet;

    fn try_from(khaam: String) -> Result<Self, Self::Error> {
        Self::jadeed(&khaam)
    }
}

impl TryFrom<&str> for WajhatLuba {
    type Error = KhataTathbeet;

    fn try_from(khaam: &str) -> Result<Self, Self::Error> {
        Self::jadeed(khaam)
    }
}

/// Which system location a [`WajhatNizam`] lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NawWajhatNizam {
    /// Inside a Wine or Proton prefix's `drive_c`.
    BeeatTawafuq,
}

/// A destination outside the game directory, from Taarib's own table.
///
/// The roots come from what `kashf` resolved in this process, never from
/// package content. Not serializable in either direction: a stored document
/// that minted one would name an absolute path nobody resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WajhatNizam {
    jidhr: PathBuf,
    dhayl: WajhatLuba,
    naw: NawWajhatNizam,
}

impl WajhatNizam {
    /// A destination inside a compatibility prefix's `drive_c`.
    ///
    /// `beea` is the prefix root `kashf` discovered from the launcher.
    ///
    /// # Errors
    ///
    /// Whatever [`WajhatLuba::jadeed`] refuses in `dhayl`.
    pub fn fi_beea(beea: &Path, dhayl: &str) -> Result<Self, KhataTathbeet> {
        let qurs = taarib_kashf::beea::mutabaqa_bila_hala(beea, "drive_c")
            .map_or_else(|| beea.join("drive_c"), |ism| beea.join(ism));
        Ok(Self {
            jidhr: qurs,
            dhayl: WajhatLuba::jadeed(dhayl)?,
            naw: NawWajhatNizam::BeeatTawafuq,
        })
    }

    /// The resolved root this destination is contained in.
    #[must_use]
    pub fn jidhr(&self) -> &Path {
        &self.jidhr
    }

    /// The validated tail under that root.
    #[must_use]
    pub const fn dhayl(&self) -> &WajhatLuba {
        &self.dhayl
    }

    /// Which kind of system location this is.
    #[must_use]
    pub const fn naw(&self) -> NawWajhatNizam {
        self.naw
    }

    /// The absolute path.
    ///
    /// A destination inside a compatibility prefix is resolved component by
    /// component with case folded, because the tail was written for the
    /// Windows the game believes it is running on and the host underneath is
    /// case-sensitive. Joining it verbatim would create a directory beside the
    /// one the game reads from, differing only in case: the write succeeds,
    /// the install reports success, and the game loads nothing.
    #[must_use]
    pub fn mutlaq(&self) -> PathBuf {
        match self.naw {
            NawWajhatNizam::BeeatTawafuq => {
                taarib_kashf::beea::hall_bila_hala(&self.jidhr, Path::new(self.dhayl.nisbi()))
            }
        }
    }
}

fn ism_mahjuz(nass: &str) -> bool {
    if nass.ends_with('.') || nass.ends_with(' ') {
        return true;
    }
    let jidhr = nass.split('.').next().unwrap_or(nass);
    matches!(
        jidhr.to_ascii_uppercase().as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
            | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
            | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    )
}
