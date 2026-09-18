//! The first-run acknowledgement, recorded once and re-shown when it changes.

use std::path::Path;

use serde::{Deserialize, Serialize};
use taarib_usus::Lugha;

use crate::khata::KhataAman;

/// The version of the acknowledgement statement this build shows.
///
/// Two, because version one enumerated what Taarib does to the files on the
/// machine and said nothing about what leaves it. A fresh installation elects
/// the built-in free provider — `IdadatMuzawwidin::muntakhab_aw_majjani` — so
/// pressing translate sends the game's own text to Google, and somebody who
/// accepted version one accepted a statement in which that did not appear.
/// That is a new obligation rather than a new wording of an old one, so every
/// existing record stops covering the statement and each user is asked exactly
/// once more; [`SijillIqrar::yaltazim`] is where that happens.
pub const ISDAR_NASS: u32 = 2;

/// The acknowledgement statement, in Arabic.
pub const NASS_ARABI: &str = "\
يعدّل تعريب ملفات ألعاب تملكها لإضافة العربية إليها. قبل أي تعديل يحفظ نسخة أصلية \
دقيقة تُمكّن من إرجاع اللعبة تمامًا إلى حالتها. لا يُثبَّت تعريب في لعبة تعمل بنظام \
مكافحة غش، ويُحذّر صراحةً قبل تعديل لعبة متعدّدة اللاعبين. تُرسل الترجمة الآلية نصَّ \
اللعبة خارج هذا الجهاز إلى خدمة ترجمة تتلقّاه مع عنوان IP الخاص بك، وما لم تُضِف \
مزوّدًا خاصًّا بك فهي خدمة ترجمة Google المجانية المضمّنة، ونموذجٌ محلي تشغّله على \
جهازك لا يُرسل شيئًا خارجه.";

/// The same statement, in English.
///
/// The two say the same five things in the same order, and both are shipped
/// because this is the one sentence in the product a person is asked to *agree
/// to* rather than merely read. A session running in English was shown the
/// Arabic and asked to acknowledge it, which is asking somebody to accept terms
/// they may not be able to read — and an acknowledgement given that way is worth
/// nothing to the person giving it.
///
/// [`ISDAR_NASS`] was deliberately *not* bumped when that second rendering was
/// added: the obligations were unchanged, only restated in another language,
/// and re-asking every existing user to accept the same statement again would
/// train them to click through it. It is bumped when the obligations themselves
/// change, which is what the fifth one is.
pub const NASS_INJILIZI: &str = "\
Taarib modifies the files of games you own in order to add Arabic to them. \
Before any modification it saves an exact copy of the original, so the game can \
be returned to precisely the state it was in. Taarib is never installed into a \
game that runs an anti-cheat system, and it warns you explicitly before \
modifying a multiplayer game. Translating new text sends the game's text off \
this machine to a translation service, which receives that text along with your \
IP address; until you add a provider of your own that service is the built-in \
free Google Translate web service, and a local model you run on this machine \
sends nothing off it.";

/// A recorded acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillIqrar {
    /// Which statement version was acknowledged.
    pub isdar_nass: u32,
    /// When, RFC 3339, as the caller supplied it.
    pub waqt: String,
    /// The product version that showed the statement.
    pub isdar_taarib: String,
    /// Which of the two renderings the person actually read, when a caller was
    /// in a position to say.
    ///
    /// [`None`] is not a default rendering, it is the absence of the answer:
    /// every record written before this field existed carries it, as does every
    /// record written by a caller that displayed nothing. Defaulting it to
    /// Arabic instead would make an English session's record claim the person
    /// accepted words they may never have seen, which is precisely what the two
    /// renderings exist to prevent.
    #[serde(default)]
    pub lugha_nass: Option<Lugha>,
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
        },
    };
    serde_json::from_slice(&bayt)
        .map(Some)
        .map_err(|khata| KhataAman::KhataIqrar {
            masar: masar.to_path_buf(),
            amal: "parsed",
            sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, khata.to_string()),
        })
}

/// Records an acknowledgement of the current statement, without naming which
/// rendering was read.
///
/// [`ahfaz_bi_lugha`] with no rendering named: the same write under the same
/// rollback rule, for a caller with no interface in front of it and therefore
/// no rendering to name. A screen has one, and must use the other.
///
/// # Errors
///
/// [`KhataAman::KhataIqrar`] when the record cannot be serialized or written.
pub fn ahfaz(masar: &Path, waqt: String, isdar_taarib: String) -> Result<SijillIqrar, KhataAman> {
    ahfaz_bi_lugha(masar, waqt, isdar_taarib, None)
}

/// Records an acknowledgement of the current statement, naming the rendering
/// the person read and never replacing a record of a newer statement.
///
/// `waqt` is an RFC 3339 timestamp the caller supplies; this module calls no
/// clock. `lugha_nass` is what a screen adds: it displayed one of [`NASS_ARABI`]
/// and [`NASS_INJILIZI`] and is the only party that knows which, and a record
/// that cannot say which text was accepted cannot show that the person was
/// asked in a language they read.
///
/// Read and write are one operation here, not two, because the composite is
/// where the only real failure lives. Nothing holds the record between a
/// caller's [`iqra`] and its write: two Taarib processes can each read "not
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
pub fn ahfaz_bi_lugha(
    masar: &Path,
    waqt: String,
    isdar_taarib: String,
    lugha_nass: Option<Lugha>,
) -> Result<SijillIqrar, KhataAman> {
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

    let sijill = SijillIqrar {
        isdar_nass: ISDAR_NASS,
        waqt,
        isdar_taarib,
        lugha_nass,
    };
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
    taarib_usus::masarat::kitaba_dharra(masar, &bayt).map_err(|khata| KhataAman::KhataIqrar {
        masar: masar.to_path_buf(),
        amal: "written",
        sabab: std::io::Error::other(khata.injilizi),
    })?;
    Ok(sijill)
}

/// Whether the current statement still needs acknowledging.
#[must_use]
pub fn yahtaj_iqrar(sijill: Option<&SijillIqrar>) -> bool {
    !sijill.is_some_and(SijillIqrar::yaltazim)
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// A record as a build shipping statement version one wrote it: no
    /// `lugha_nass` field at all, because none existed.
    const SIJILL_AWWAL: &str = r#"{
      "isdar_nass": 1,
      "waqt": "2026-01-01T00:00:00Z",
      "isdar_taarib": "0.1.0"
    }"#;

    fn masar(masrah: &tempfile::TempDir) -> std::path::PathBuf {
        masrah.path().join("bayanat").join("iqrar.json")
    }

    /// The statement a version-one record was given to said nothing about the
    /// network, so that record does not cover this one and the gate asks again.
    #[test]
    fn sijill_al_isdar_al_awwal_la_yughni_an_al_jadeed() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let malaf = masar(&masrah);
        std::fs::create_dir_all(malaf.parent().ok_or("the record has no parent directory")?)?;
        std::fs::write(&malaf, SIJILL_AWWAL)?;

        let qadeem = iqra(&malaf)?.ok_or("the version-one record did not read back")?;
        assert_eq!(qadeem.isdar_nass, 1);
        assert_eq!(
            qadeem.lugha_nass, None,
            "a record written before the field existed names no rendering"
        );
        assert!(!qadeem.yaltazim(), "version one does not cover version two");
        assert!(
            yahtaj_iqrar(Some(&qadeem)),
            "the gate asks again after the bump"
        );
        Ok(())
    }

    /// The re-ask fires exactly once: the record this build writes over the old
    /// one covers the statement, and the next read asks nothing.
    #[test]
    fn iqrar_jadeed_yuhill_mahall_al_qadeem_marra_wahida() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let malaf = masar(&masrah);
        std::fs::create_dir_all(malaf.parent().ok_or("the record has no parent directory")?)?;
        std::fs::write(&malaf, SIJILL_AWWAL)?;

        let jadeed = ahfaz_bi_lugha(
            &malaf,
            "2026-09-18T00:00:00Z".to_owned(),
            "0.2.0".to_owned(),
            Some(Lugha::Injilizi),
        )?;
        assert_eq!(jadeed.isdar_nass, ISDAR_NASS);
        assert_eq!(jadeed.lugha_nass, Some(Lugha::Injilizi));

        let mahfuz = iqra(&malaf)?.ok_or("the new record did not read back")?;
        assert_eq!(mahfuz, jadeed, "what was returned is what is on disk");
        assert!(mahfuz.yaltazim());
        assert!(
            !yahtaj_iqrar(Some(&mahfuz)),
            "the statement is not asked for twice"
        );

        // The rollback rule: a second press keeps the record already covering
        // this statement rather than rewriting its timestamp or its rendering.
        let thani = ahfaz_bi_lugha(
            &malaf,
            "2026-09-19T00:00:00Z".to_owned(),
            "0.2.0".to_owned(),
            Some(Lugha::Arabi),
        )?;
        assert_eq!(thani, mahfuz, "a covering record is kept, not replaced");
        Ok(())
    }

    /// A caller with no interface names no rendering, and that absence reads
    /// back as an absence rather than as Arabic.
    #[test]
    fn iqrar_bila_wajiha_la_yaddai_lughatan() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let malaf = masar(&masrah);

        let sijill = ahfaz(
            &malaf,
            "2026-09-18T00:00:00Z".to_owned(),
            "0.2.0".to_owned(),
        )?;
        assert_eq!(sijill.lugha_nass, None);
        assert_eq!(
            iqra(&malaf)?
                .ok_or("the record did not read back")?
                .lugha_nass,
            None
        );
        Ok(())
    }

    /// Both renderings state the network obligation, and neither states it in
    /// the other's language.
    #[test]
    fn al_nassan_yadhkuran_al_shabaka_bil_lughatayn() {
        assert!(NASS_ARABI.contains("خدمة ترجمة Google المجانية"));
        assert!(NASS_ARABI.contains("عنوان IP"));
        assert!(NASS_ARABI.contains("نموذجٌ محلي"));
        assert!(NASS_INJILIZI.contains("Google Translate web service"));
        assert!(NASS_INJILIZI.contains("IP address"));
        assert!(NASS_INJILIZI.contains("a local model you run on this machine"));
        assert!(
            !NASS_INJILIZI.chars().any(|harf| matches!(
                harf,
                '\u{0600}'..='\u{06ff}' | '\u{0750}'..='\u{077f}'
            )),
            "the English rendering must hold no Arabic"
        );
    }
}
