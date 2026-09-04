//! The first-run acknowledgement, recorded once and re-shown when it changes.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::khata::KhataAman;

/// The version of the acknowledgement statement this build shows.
pub const ISDAR_NASS: u32 = 1;

/// The acknowledgement statement, in Arabic.
pub const NASS_ARABI: &str = "\
يعدّل تعريب ملفات ألعاب تملكها لإضافة العربية إليها. قبل أي تعديل يحفظ نسخة أصلية \
دقيقة تُمكّن من إرجاع اللعبة تمامًا إلى حالتها. لا يُثبَّت تعريب في لعبة تعمل بنظام \
مكافحة غش، ويُحذّر صراحةً قبل تعديل لعبة متعدّدة اللاعبين.";

/// A recorded acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillIqrar {
    /// Which statement version was acknowledged.
    pub isdar_nass: u32,
    /// When, RFC 3339, as the caller supplied it.
    pub waqt: String,
    /// The product version that showed the statement.
    pub isdar_taarib: String,
}

impl SijillIqrar {
    /// Whether this record covers the statement this build shows.
    #[must_use]
    pub const fn yaltazim(&self) -> bool {
        self.isdar_nass >= ISDAR_NASS
    }
}

/// Reads the acknowledgement record, or [`None`] when none has been made.
///
/// # Errors
///
/// [`KhataAman::KhataIqrar`] when the record exists but cannot be read or is
/// not valid JSON.
pub fn iqra(masar: &Path) -> Result<Option<SijillIqrar>, KhataAman> {
    let bayt = match std::fs::read(masar) {
        Ok(bayt) => bayt,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(sabab) => {
            return Err(KhataAman::KhataIqrar {
                masar: masar.to_path_buf(),
                amal: "read",
                sabab,
            });
        }
    };
    serde_json::from_slice(&bayt).map(Some).map_err(|khata| KhataAman::KhataIqrar {
        masar: masar.to_path_buf(),
        amal: "parsed",
        sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, khata.to_string()),
    })
}

/// Records an acknowledgement of the current statement, never replacing a
/// record of a newer one.
///
/// `waqt` is an RFC 3339 timestamp the caller supplies; this module calls no
/// clock.
///
/// Read and write are one operation here, not two, because the composite is
/// where the only real failure lives. Nothing holds the record between a
/// caller's [`iqra`] and its `ahfaz`: two Taarib processes can each read "not
/// acknowledged", each show the statement, and each write. Where both are the
/// same build that is harmless — the two records differ only in a timestamp
/// and either one is true — but where they are not, the older build's
/// `isdar_nass` overwrites the newer one, and the newer statement the user has
/// already accepted is asked for again. So the record is re-read here and kept
/// whenever it already covers a statement at least as new as this build's.
/// This is [`crate::qaimat_sahb::QaimatSahb::baad`]'s rollback rule, which
/// exists for the same reason: a write must never move the recorded state
/// backwards.
///
/// The returned record is the one now on disk, which is not always the one
/// this call was asked to write.
///
/// # Errors
///
/// [`KhataAman::KhataIqrar`] when the record cannot be serialized or written.
pub fn ahfaz(masar: &Path, waqt: String, isdar_taarib: String) -> Result<SijillIqrar, KhataAman> {
    // Deliberately no lock file, unlike `qaimat_sahb::hadith_makhbaa`. There the
    // lock is what makes the rollback rule sound, because a lost update there
    // un-revokes a compromised signing key. Here a lock could not span the race
    // anyway: the decision sits in the interface, between the `iqrar_aman` read
    // and the `sajjil_iqrar_aman` write, so all it would serialise is two writes
    // that are already atomic and — absent the rule below — both of them true.
    //
    // A record that will not read covers no statement, so it is replaced rather
    // than refused: refusing would leave the user unable to acknowledge at all.
    if let Ok(Some(qadeem)) = iqra(masar)
        && qadeem.yaltazim()
    {
        return Ok(qadeem);
    }

    let sijill = SijillIqrar { isdar_nass: ISDAR_NASS, waqt, isdar_taarib };
    let bayt = serde_json::to_vec_pretty(&sijill).map_err(|khata| KhataAman::KhataIqrar {
        masar: masar.to_path_buf(),
        amal: "serialized",
        sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, khata.to_string()),
    })?;
    if let Some(walid) = masar.parent() {
        std::fs::create_dir_all(walid).map_err(|sabab| KhataAman::KhataIqrar {
            masar: masar.to_path_buf(),
            amal: "written",
            sabab,
        })?;
    }
    // Through the atomic write, like every other record in the product: a
    // plain `write` interrupted part way leaves a truncated document, and this
    // one being unreadable makes the product ask for a first-run
    // acknowledgement the user already gave.
    taarib_usus::masarat::kitaba_dharra(masar, &bayt).map_err(|khata| {
        KhataAman::KhataIqrar {
            masar: masar.to_path_buf(),
            amal: "written",
            sabab: std::io::Error::other(khata.injilizi),
        }
    })?;
    Ok(sijill)
}

/// Whether the current statement still needs acknowledging.
#[must_use]
pub fn yahtaj_iqrar(sijill: Option<&SijillIqrar>) -> bool {
    !sijill.is_some_and(SijillIqrar::yaltazim)
}
