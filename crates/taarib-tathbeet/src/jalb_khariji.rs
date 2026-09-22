//! جلب الرقعة الخارجية — fetching somebody else's artifacts and proving them
//! against their pins, with the game directory still untouched.
//!
//! Everything here happens in a staging directory that is not inside a game.
//! That is the whole point of the module existing separately from the install:
//! a third-party artifact is a file off the internet, and the moment it is
//! allowed to land in a game directory is the moment a mismatch stops being a
//! refusal and starts being damage. So the order is fixed — fetch into staging,
//! prove the size, prove the digest, and only then hand the install a
//! [`QitaaMuhaqqaqa`], which is the only type in this crate that says an
//! artifact matched what the registry pinned.
//!
//! ## Both halves of the pin, in that order
//!
//! [`taarib_mustalahat::khariji::QitaatTanzeel`] carries a size **and** a
//! sha256, and both are checked. The size first, because it costs one `stat`
//! and a release that moved is overwhelmingly a different size; the digest
//! second, over the bytes on disk rather than over a buffer the transport
//! handed back, because what the install is about to unpack is the file and not
//! the transport's account of it.
//!
//! A mismatch is refused **by name** —
//! [`crate::khata::KhataTathbeet::HajmQitaaGhayrMutabiq`] and
//! [`crate::khata::KhataTathbeet::BasmatQitaaGhayrMutabiqa`] both carry the
//! artifact, the address, what was pinned and what arrived — and the staged copy
//! is deleted rather than kept. A wrong file left in staging is a wrong file a
//! later run resumes from, and there is no setting, flag or acknowledgement
//! anywhere that turns either check off. Re-pinning a release is the registry
//! owner's decision, taken in the review console, never here.
//!
//! ## Where the bytes come from
//!
//! The author's own endpoint, always, unless two separate things are both true:
//! the entry says [`HalatMira::MinAlsijill`] *and* the permit the gate minted
//! says mirroring was permitted for it. Either alone leaves the fetch on the
//! author's endpoint. The permit is the authority rather than the entry,
//! because the entry is a document that can be edited between the gate and the
//! install and the permit cannot.
//!
//! ## Why the transport is a parameter
//!
//! This crate installs files. It holds no HTTP client, no TLS stack and no
//! async runtime, and acquiring all three so that one module can issue a GET
//! would put a network stack inside the component whose other job is restoring
//! people's games. So the transfer is a port, exactly as the setting writer
//! [`crate::taraju::RadIdad`] and the signature checker
//! `taarib_ruqaa::tawqee::MudaqqiqTawqee` already are: the application supplies
//! the stack it already has, and the verification — the part that must not vary
//! — lives here where it can be proved without a network.

use std::fs::{self, File};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use taarib_aman::IdhnTathbeetKhariji;
use taarib_mustalahat::khariji::{HalatMira, QitaatTanzeel};
use taarib_usus::masarat;

use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io, tul_u64};
use crate::mawdi::WajhatLuba;

/// The largest artifact this build will fetch, in bytes.
///
/// Two gibibytes. Checked against the size the *pin* declares, before anything
/// is fetched, so a pathological entry is a refusal naming the number rather
/// than a filled disk. RTEA's own largest artifact is 25 MB and unpacks to 52;
/// the ceiling is here for the entry nobody has written yet.
pub const AQSA_HAJM_QITAA: u64 = 2 * 1024 * 1024 * 1024;

/// How much is read at a time while hashing a staged artifact.
const HAJM_QITA: usize = 256 * 1024;

/// Where a third-party artifact's bytes come from.
///
/// One method, taking an address and a destination, because that is the whole
/// of what this crate needs and anything more would be a network API defined by
/// a file installer. The implementation is the application's: it already holds
/// an HTTP stack, a proxy configuration and whatever the user set as their
/// connection policy, and none of those are facts a game installer should hold
/// a second copy of.
///
/// An implementation must write the body and nothing else to `hadaf`, and must
/// not create anything anywhere else. It is not asked to verify anything: the
/// size and the digest are checked here, over the file it left behind, so that
/// a transport cannot be the component that decides an artifact matched.
pub trait NaqilKhariji {
    /// Fetches one address into one file.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::TanzeelMutaadhdhir`] naming the artifact, the address
    /// and what the transport reported, for anything that stopped the transfer
    /// from completing.
    fn ijlib(&mut self, ism: &str, rabt: &str, hadaf: &Path) -> NatijatTathbeet<()>;
}

/// One artifact that is in staging and matched its pin.
///
/// There is no public constructor. The only way to hold one is for
/// [`ijlib_qitaa`] to have fetched the bytes and reproduced both halves of the
/// pin over them, so an install step that takes one of these cannot have been
/// handed an unverified file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QitaaMuhaqqaqa {
    ism: String,
    masar: PathBuf,
    hajm: u64,
    sha256: String,
}

impl QitaaMuhaqqaqa {
    /// The artifact's file name, as the pin names it.
    #[must_use]
    pub fn ism(&self) -> &str {
        &self.ism
    }

    /// Where it is in the staging area.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// Its size, as measured on disk after the transfer.
    #[must_use]
    pub const fn hajm(&self) -> u64 {
        self.hajm
    }

    /// Its digest, as computed on disk after the transfer, lowercase hex.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Fetches every artifact the permit approved and proves each against its pin.
///
/// Nothing reaches a game directory here, and nothing in this function knows
/// where a game directory is. The artifacts are the permit's, not the entry's:
/// an entry edited between the gate and this call authorises nothing, because
/// what is fetched and what it is checked against both come out of
/// [`IdhnTathbeetKhariji`].
///
/// A failure part of the way through leaves the artifacts that had already been
/// proven in the staging directory; [`nazzif_marhala`] over the same directory
/// takes them out, and the caller runs it on both paths. Nothing is resumed from
/// a staged file either way — a previous run's copy under the final name is
/// deleted before the transport is asked for the body, because a partial body
/// under a finished name is a file the pin check would have to be trusted to
/// catch rather than a file that never existed.
///
/// # Errors
///
/// [`KhataTathbeet::BasmaGhayrSaliha`] when a pin is not a digest,
/// [`KhataTathbeet::HajmMufrit`] when a pin declares more than
/// [`AQSA_HAJM_QITAA`], [`KhataTathbeet::MasarKharij`] when an artifact's name
/// is not a bare file name, [`KhataTathbeet::IdhnKharijiGhayrMutabiq`] when the
/// entry asks for a mirror the permit did not allow,
/// [`KhataTathbeet::TanzeelMutaadhdhir`] when the transfer does not complete,
/// and [`KhataTathbeet::HajmQitaaGhayrMutabiq`] or
/// [`KhataTathbeet::BasmatQitaaGhayrMutabiqa`] when what arrived is not what was
/// pinned — in which case the copy that disagreed has been deleted.
pub fn ijlib_qitaa(
    naqil: &mut dyn NaqilKhariji,
    idhn: &IdhnTathbeetKhariji,
    mira: &HalatMira,
    jidhr_marhala: &Path,
) -> NatijatTathbeet<Vec<QitaaMuhaqqaqa>> {
    // Every shape question is answered over the whole list before the first
    // byte moves, so an entry with a malformed pin on its second artifact does
    // not fetch the first one.
    for qitaa in idhn.qitaa() {
        tahaqquq_shakl(qitaa)?;
    }

    insha_aw_khata(jidhr_marhala)?;
    let mut muhaqqaqa = Vec::with_capacity(idhn.qitaa().len());
    for qitaa in idhn.qitaa() {
        let rabt = mawdi_jalb(qitaa, mira, idhn.mira_masmuha())?;
        let hadaf = jidhr_marhala.join(&qitaa.ism);
        // A file left by an interrupted run is not resumed from: the transport
        // is not asked to append, and a partial body under the final name would
        // be hashed as though it were whole.
        hadhf_in_wujid(&hadaf)?;
        naqil.ijlib(&qitaa.ism, &rabt, &hadaf)?;
        muhaqqaqa.push(haqqiq(&hadaf, qitaa, &rabt)?);
    }
    Ok(muhaqqaqa)
}

/// Removes what a staging run left behind.
///
/// The files directly inside the run's own directory, then the directory itself
/// — one level, `remove_dir` rather than a recursive delete, because this is
/// called with the per-entry directory [`ijlib_qitaa`] created and nothing here
/// has any business walking a tree a caller nominated. A partial run is cleaned
/// by the same call as a finished one, which is why it takes the directory and
/// not a list: a transfer that failed on the second artifact left the first one
/// staged, and nothing is holding the list.
///
/// Failures are swallowed on purpose. This runs after an install has already
/// succeeded or already failed, and a staging directory that could not be
/// emptied is a disk-space report rather than a reason to turn a finished
/// install into a failure.
pub fn nazzif_marhala(jidhr_marhala: &Path) {
    let Ok(madakhil) = fs::read_dir(jidhr_marhala) else {
        return;
    };
    for madkhal in madakhil.flatten() {
        if madkhal.file_type().is_ok_and(|naw| naw.is_file()) {
            drop(fs::remove_file(madkhal.path()));
        }
    }
    drop(fs::remove_dir(jidhr_marhala));
}

/// Answers every question about a pin that can be answered without fetching.
fn tahaqquq_shakl(qitaa: &QitaatTanzeel) -> NatijatTathbeet<()> {
    if !qitaa.basma_salima() {
        return Err(KhataTathbeet::BasmaGhayrSaliha {
            ism: qitaa.ism.clone(),
            sha: qitaa.sha256.clone(),
        });
    }
    if qitaa.hajm > AQSA_HAJM_QITAA {
        return Err(KhataTathbeet::HajmMufrit {
            haql: "a third-party artifact's pinned size",
            qeema: qitaa.hajm,
            saqf: AQSA_HAJM_QITAA,
        });
    }
    // The name becomes a file name in the staging directory, so it goes through
    // the same validator a destination inside a game does — and then has to
    // still be one component afterwards, because a name carrying a directory
    // would stage one artifact under another's path.
    let wajha = WajhatLuba::jadeed(&qitaa.ism)?;
    if wajha.nisbi() != qitaa.ism || wajha.ism() != qitaa.ism {
        return Err(KhataTathbeet::MasarKharij {
            masar: PathBuf::from(&qitaa.ism),
            jidhr: PathBuf::new(),
            sabab: "a third-party artifact's name is not a bare file name".to_owned(),
        });
    }
    Ok(())
}

/// The address one artifact is fetched from.
///
/// The author's endpoint unless the entry asks for the registry's copy *and*
/// the permit says that was allowed. A mirror address names the entry's
/// directory on the registry, so the artifact's own name is joined onto it —
/// unless the recorded address already ends in that name, which is the shape a
/// single-artifact entry is naturally written in.
fn mawdi_jalb(qitaa: &QitaatTanzeel, mira: &HalatMira, masmuha: bool) -> NatijatTathbeet<String> {
    let HalatMira::MinAlsijill { rabt } = mira else {
        return Ok(qitaa.rabt.clone());
    };
    if !masmuha {
        return Err(KhataTathbeet::IdhnKharijiGhayrMutabiq {
            sabab: format!(
                "the entry asks for {rabt} on the registry's mirror and the permit records no \
                 permission to mirror it; the author's own endpoint is the only source"
            ),
        });
    }
    let munaqqa = rabt.trim_end_matches('/');
    if munaqqa.ends_with(&format!("/{}", qitaa.ism)) || munaqqa == qitaa.ism {
        return Ok(munaqqa.to_owned());
    }
    Ok(format!("{munaqqa}/{}", qitaa.ism))
}

/// Proves one staged file against its pin, deleting it when it disagrees.
fn haqqiq(hadaf: &Path, qitaa: &QitaatTanzeel, rabt: &str) -> NatijatTathbeet<QitaaMuhaqqaqa> {
    let bayanat = fs::metadata(hadaf).map_err(|sabab| {
        // A transport that reported success and left nothing is the same
        // failure as one that could not connect: no bytes arrived.
        KhataTathbeet::TanzeelMutaadhdhir {
            ism: qitaa.ism.clone(),
            rabt: rabt.to_owned(),
            sabab: format!("nothing was left at {}: {sabab}", hadaf.display()),
        }
    })?;

    let hajm = bayanat.len();
    if hajm != qitaa.hajm {
        hadhf_in_wujid(hadaf)?;
        return Err(KhataTathbeet::HajmQitaaGhayrMutabiq {
            ism: qitaa.ism.clone(),
            rabt: rabt.to_owned(),
            hajm_mudam: qitaa.hajm,
            hajm_wasil: hajm,
        });
    }

    let mahsuba = sha256_malaf(hadaf)?;
    if !mahsuba.eq_ignore_ascii_case(&qitaa.sha256) {
        hadhf_in_wujid(hadaf)?;
        return Err(KhataTathbeet::BasmatQitaaGhayrMutabiqa {
            ism: qitaa.ism.clone(),
            rabt: rabt.to_owned(),
            sha_mudam: qitaa.sha256.to_lowercase(),
            sha_wasil: mahsuba,
        });
    }

    Ok(QitaaMuhaqqaqa {
        ism: qitaa.ism.clone(),
        masar: hadaf.to_path_buf(),
        hajm,
        sha256: mahsuba,
    })
}

/// The sha256 of a file, read in chunks, as lowercase hex.
///
/// sha256 rather than the BLAKE3 the manifest uses everywhere else, because
/// this digest is not Taarib's: it is the number a maintainer took over
/// somebody else's published file and can check against the author's own
/// announcement.
fn sha256_malaf(masar: &Path) -> NatijatTathbeet<String> {
    let mut malaf = File::open(masar)
        .map_err(|sabab| min_khata_io(masar, "opening a staged artifact", sabab))?;
    let mut hashib = Sha256::new();
    let mut lawh = vec![0_u8; HAJM_QITA];
    loop {
        let maqru = malaf
            .read(&mut lawh)
            .map_err(|sabab| min_khata_io(masar, "hashing a staged artifact", sabab))?;
        if maqru == 0 {
            break;
        }
        match lawh.get(..maqru) {
            Some(qitaa) => hashib.update(qitaa),
            // Unreachable: `read` never reports more than the buffer holds. It
            // is an error rather than a slice because the alternative to
            // returning one is indexing, and indexing is what would turn a
            // hostile `Read` into a panic inside somebody's install.
            None => {
                return Err(KhataTathbeet::KhataMalaf {
                    masar: masar.to_path_buf(),
                    amal: "hashing a staged artifact",
                    sabab: std::io::Error::other(format!(
                        "the reader reported {maqru} byte(s) into a {} byte buffer",
                        tul_u64(lawh.len())
                    )),
                });
            },
        }
    }
    Ok(hex::encode(hashib.finalize()))
}

/// Deletes a file when it is there, naming the real reason when it cannot be.
fn hadhf_in_wujid(masar: &Path) -> NatijatTathbeet<()> {
    match fs::remove_file(masar) {
        Ok(()) => Ok(()),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(sabab) => Err(min_khata_io(
            masar,
            "removing a staged artifact that did not match its pin",
            sabab,
        )),
    }
}

/// Creates the staging directory, naming the real reason when it cannot be.
fn insha_aw_khata(masar: &Path) -> NatijatTathbeet<()> {
    masarat::insha_mujallad(masar).map_err(|khata| KhataTathbeet::KhataMalaf {
        masar: masar.to_path_buf(),
        amal: "creating the staging directory",
        sabab: std::io::Error::other(khata.injilizi),
    })
}

#[cfg(test)]
mod ikhtibarat {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use taarib_mustalahat::khariji::{HalatMira, QitaatTanzeel};

    use super::{NaqilKhariji, mawdi_jalb, sha256_malaf, tahaqquq_shakl};
    use crate::khata::{KhataTathbeet, NatijatTathbeet};

    /// A transport that serves bytes from a table instead of from a network.
    #[derive(Debug, Default)]
    struct NaqilThabit {
        mawad: BTreeMap<String, Vec<u8>>,
        talabat: Vec<String>,
    }

    impl NaqilKhariji for NaqilThabit {
        fn ijlib(&mut self, ism: &str, rabt: &str, hadaf: &Path) -> NatijatTathbeet<()> {
            self.talabat.push(rabt.to_owned());
            let Some(bayt) = self.mawad.get(rabt) else {
                return Err(KhataTathbeet::TanzeelMutaadhdhir {
                    ism: ism.to_owned(),
                    rabt: rabt.to_owned(),
                    sabab: "no such address in the fixture".to_owned(),
                });
            };
            std::fs::write(hadaf, bayt).map_err(|sabab| KhataTathbeet::KhataMalaf {
                masar: hadaf.to_path_buf(),
                amal: "writing a fixture body",
                sabab,
            })
        }
    }

    fn qitaa(ism: &str, sha256: &str, hajm: u64) -> QitaatTanzeel {
        QitaatTanzeel {
            ism: ism.to_owned(),
            rabt: format!("https://rt.example/{ism}"),
            hajm,
            sha256: sha256.to_owned(),
            abniya: Vec::new(),
        }
    }

    const BASMA: &str = "ffd54019f146b8db8a02d6413c592222ae9de397c522631d3b65fe882b3e1343";

    #[test]
    fn basma_ghayr_salima_turfad_qabl_al_jalb() {
        let khata = tahaqquq_shakl(&qitaa("update.zip", "not-a-digest", 10))
            .err()
            .map(|khata| matches!(khata, KhataTathbeet::BasmaGhayrSaliha { .. }));
        assert_eq!(khata, Some(true));
    }

    #[test]
    fn ism_bi_masar_marfud() {
        let khata = tahaqquq_shakl(&qitaa("../update.zip", BASMA, 10))
            .err()
            .map(|khata| matches!(khata, KhataTathbeet::MasarKharij { .. }));
        assert_eq!(khata, Some(true));
        let khata = tahaqquq_shakl(&qitaa("lml/update.zip", BASMA, 10))
            .err()
            .map(|khata| matches!(khata, KhataTathbeet::MasarKharij { .. }));
        assert_eq!(khata, Some(true));
        assert!(tahaqquq_shakl(&qitaa("update.zip", BASMA, 10)).is_ok());
    }

    /// Mirroring needs the permit's word, not the entry's.
    #[test]
    fn al_mira_bila_idhn_marfuda() {
        let wahid = qitaa("update.zip", BASMA, 10);
        let sijill = HalatMira::MinAlsijill {
            rabt: "https://mira.example/rtea".to_owned(),
        };
        let khata = mawdi_jalb(&wahid, &sijill, false)
            .err()
            .map(|khata| matches!(khata, KhataTathbeet::IdhnKharijiGhayrMutabiq { .. }));
        assert_eq!(khata, Some(true));

        assert_eq!(
            mawdi_jalb(&wahid, &sijill, true).ok(),
            Some("https://mira.example/rtea/update.zip".to_owned())
        );
        assert_eq!(
            mawdi_jalb(&wahid, &HalatMira::MinAlmuallif, true).ok(),
            Some("https://rt.example/update.zip".to_owned())
        );
        // An address already naming the artifact is used as it stands.
        assert_eq!(
            mawdi_jalb(
                &wahid,
                &HalatMira::MinAlsijill {
                    rabt: "https://mira.example/rtea/update.zip".to_owned(),
                },
                true
            )
            .ok(),
            Some("https://mira.example/rtea/update.zip".to_owned())
        );
    }

    /// The digest half of the pin, over a body whose size is right.
    ///
    /// A size that matches is the case a size-only check would wave through, so
    /// it is the one worth proving: the refusal names the artifact, says what
    /// was pinned and what arrived, and the copy that disagreed is gone rather
    /// than left where a later run would find it and skip the fetch.
    #[test]
    fn basma_mukhtalifa_turfad_bil_ism_wa_tuhdhaf() -> Result<(), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let hadaf = mujallad.path().join("update.zip");
        std::fs::write(&hadaf, b"abc")?;

        let wahid = qitaa("update.zip", BASMA, 3);
        let khata = super::haqqiq(&hadaf, &wahid, "https://rt.example/update.zip").err();
        let musamma = match &khata {
            Some(KhataTathbeet::BasmatQitaaGhayrMutabiqa {
                ism,
                sha_mudam,
                sha_wasil,
                ..
            }) => Some((ism.clone(), sha_mudam.clone(), sha_wasil.clone())),
            _ => None,
        };
        assert_eq!(
            musamma,
            Some((
                "update.zip".to_owned(),
                BASMA.to_owned(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned(),
            )),
            "the refusal names the artifact, the pin and what arrived"
        );
        assert!(!hadaf.exists(), "the copy that disagreed is not kept");
        Ok(())
    }

    /// The cheap half, and the one that must fire before the bytes are read.
    #[test]
    fn hajm_mukhtalif_yurfad_qabl_al_basma() -> Result<(), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let hadaf = mujallad.path().join("update.zip");
        std::fs::write(&hadaf, b"abcd")?;

        let wahid = qitaa("update.zip", BASMA, 3);
        let khata = super::haqqiq(&hadaf, &wahid, "https://rt.example/update.zip").err();
        assert!(matches!(
            khata,
            Some(KhataTathbeet::HajmQitaaGhayrMutabiq {
                hajm_mudam: 3,
                hajm_wasil: 4,
                ..
            })
        ));
        assert!(!hadaf.exists());
        Ok(())
    }

    /// A transport that reports success and leaves nothing behind is a
    /// transport that fetched nothing, and is reported as that rather than as a
    /// pin that disagreed.
    #[test]
    fn la_shay_fi_al_marhala_yuqra_tanzeelan_mutaadhdhiran()
    -> Result<(), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let hadaf = mujallad.path().join("update.zip");
        let khata = super::haqqiq(
            &hadaf,
            &qitaa("update.zip", BASMA, 3),
            "https://rt.example/update.zip",
        )
        .err();
        assert!(matches!(
            khata,
            Some(KhataTathbeet::TanzeelMutaadhdhir { .. })
        ));
        Ok(())
    }

    /// The fixture transport serves from a table, so the whole fetch path can be
    /// proved without a network.
    #[test]
    fn al_naqil_al_thabit_yaktub_wa_yusajjil() -> Result<(), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let mut naqil = NaqilThabit::default();
        let _ = naqil
            .mawad
            .insert("https://rt.example/update.zip".to_owned(), b"abc".to_vec());

        let hadaf = mujallad.path().join("update.zip");
        naqil.ijlib("update.zip", "https://rt.example/update.zip", &hadaf)?;
        assert_eq!(std::fs::read(&hadaf)?, b"abc");
        assert_eq!(
            naqil.talabat,
            vec!["https://rt.example/update.zip".to_owned()]
        );

        let khata = naqil
            .ijlib("extra.zip", "https://rt.example/extra.zip", &hadaf)
            .err();
        assert!(matches!(
            khata,
            Some(KhataTathbeet::TanzeelMutaadhdhir { .. })
        ));
        Ok(())
    }

    #[test]
    fn sha256_yutabiq_al_qeema_al_maruufa() -> Result<(), Box<dyn std::error::Error>> {
        let mujallad = tempfile::tempdir()?;
        let masar: PathBuf = mujallad.path().join("farigh");
        std::fs::write(&masar, b"")?;
        assert_eq!(
            sha256_malaf(&masar)?,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        std::fs::write(&masar, b"abc")?;
        assert_eq!(
            sha256_malaf(&masar)?,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        Ok(())
    }
}
