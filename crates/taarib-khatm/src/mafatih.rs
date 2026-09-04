//! Signing-key custody in the OS keychain; no private key ever touches disk.

use keyring::Entry;

use crate::khata::KhataKhatm;
use crate::tawqee::{MiftahAam, MiftahKhass};

/// The keychain service name every Taarib key is stored under.
pub const KHIDMA: &str = "taarib.tawqee";

fn madkhal(ism: &str) -> Result<Entry, KhataKhatm> {
    Entry::new(KHIDMA, ism).map_err(|khata| KhataKhatm::KhataMiftah {
        amal: "opened in the keychain",
        sabab: khata.to_string(),
    })
}

/// Generates a signing key and stores it under `ism`, returning its public half.
///
/// # Errors
///
/// [`KhataKhatm::KhataMiftah`] when the keychain refuses the write.
pub fn wallid(ism: &str) -> Result<MiftahAam, KhataKhatm> {
    use rand::RngCore as _;
    let mut bayt = [0u8; 32];
    rand::rng().fill_bytes(&mut bayt);
    let miftah = MiftahKhass::min_bayt(&bayt);
    khzin(ism, &miftah)?;
    Ok(miftah.aam())
}

/// Stores an existing signing key under `ism`.
///
/// # Errors
///
/// [`KhataKhatm::KhataMiftah`] when the keychain refuses the write.
pub fn khzin(ism: &str, miftah: &MiftahKhass) -> Result<(), KhataKhatm> {
    madkhal(ism)?
        .set_password(&hex::encode(miftah.bayt()))
        .map_err(|khata| KhataKhatm::KhataMiftah {
            amal: "stored in the keychain",
            sabab: khata.to_string(),
        })
}

/// Retrieves a signing key by name.
///
/// # Errors
///
/// [`KhataKhatm::MiftahMafqud`] when no key is stored under `ism`,
/// [`KhataKhatm::MaddaTalifa`] when the stored material is not a 32-byte seed,
/// and [`KhataKhatm::KhataMiftah`] when the keychain refuses the read.
pub fn hat(ism: &str) -> Result<MiftahKhass, KhataKhatm> {
    let nassi = match madkhal(ism)?.get_password() {
        Ok(nassi) => nassi,
        Err(keyring::Error::NoEntry) => {
            return Err(KhataKhatm::MiftahMafqud { ism: ism.to_owned() });
        }
        Err(khata) => {
            return Err(KhataKhatm::KhataMiftah {
                amal: "read from the keychain",
                sabab: khata.to_string(),
            });
        }
    };
    let bayt = hex::decode(&nassi).map_err(|_| KhataKhatm::MaddaTalifa { tul: nassi.len() })?;
    let bayt: [u8; 32] = bayt.as_slice().try_into().map_err(|_| KhataKhatm::MaddaTalifa {
        tul: bayt.len(),
    })?;
    Ok(MiftahKhass::min_bayt(&bayt))
}

/// Deletes a stored signing key.
///
/// # Errors
///
/// [`KhataKhatm::KhataMiftah`] when the keychain refuses the deletion. A key
/// that was already absent is not an error.
pub fn imsah(ism: &str) -> Result<(), KhataKhatm> {
    match madkhal(ism)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(khata) => Err(KhataKhatm::KhataMiftah {
            amal: "deleted from the keychain",
            sabab: khata.to_string(),
        }),
    }
}
