//! The owner key: the compiled trust anchor, its identity, and its keychain custody.

use crate::khata::KhataKhatm;
use crate::mafatih;
use crate::tawqee::{MiftahAam, MiftahKhass};

/// Which signing identity a build trusts. The distinction is structural: the
/// two identities anchor to different keys, so neither build can be talked
/// into accepting the other's signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HawiyatThiqa {
    /// A development build, anchored to the committed [`MIFTAH_TATWIR`].
    Tatwir,
    /// A release build, anchored to a key injected at build time and absent
    /// from this repository.
    Isdar,
}

impl HawiyatThiqa {
    /// The wire label the interface reads.
    #[must_use]
    pub const fn wasm(self) -> &'static str {
        match self {
            Self::Tatwir => "tatwir",
            Self::Isdar => "isdar",
        }
    }
}

/// The compiled trust anchor: the public key and which identity it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirsatThiqa {
    /// The public key a package must be signed under to install.
    pub miftah: [u8; 32],
    /// Which identity that key belongs to.
    pub hawiya: HawiyatThiqa,
}

/// The committed development public key.
///
/// Its private half lives in a developer keychain, unprotected by design, so
/// nothing signed under it may reach a user: a release client names and
/// refuses this key specifically rather than treating it as merely unknown.
pub const MIFTAH_TATWIR: [u8; 32] = [
    0xe4, 0x26, 0x0a, 0x5f, 0x02, 0x02, 0x9a, 0x64, 0xb6, 0xa4, 0x1a, 0x31, 0xa5, 0xdb, 0x53,
    0xe0, 0xaf, 0x74, 0xd6, 0x11, 0x01, 0x22, 0xd9, 0xa3, 0x52, 0xa7, 0x21, 0xa2, 0x43, 0x8b,
    0x4c, 0xea,
];

/// The value of one hexadecimal digit.
///
/// The guard is an assertion rather than a fallible return because the only
/// caller runs during const evaluation, where a malformed digit must stop the
/// build outright instead of travelling any further as a value.
const fn qeemat_khana(harf: u8) -> u8 {
    assert!(
        harf.is_ascii_hexdigit(),
        "TAARIB_MIFTAH_ISDAR must be exactly 64 hexadecimal characters"
    );
    match harf {
        b'0'..=b'9' => harf - b'0',
        b'a'..=b'f' => harf - b'a' + 10,
        _ => harf - b'A' + 10,
    }
}

/// The 32 bytes a 64-digit hexadecimal key spells out.
#[expect(
    clippy::indexing_slicing,
    reason = "storing into an array at a computed position has no const-stable \
              alternative; the length assertion pins the loop to 32 iterations, \
              and an out-of-range write here would fail the build, not a run"
)]
const fn fakk_sittashari(nassi: &str) -> [u8; 32] {
    let mut khaam = nassi.as_bytes();
    assert!(khaam.len() == 64, "TAARIB_MIFTAH_ISDAR must be exactly 64 hexadecimal characters");
    let mut bayt = [0u8; 32];
    let mut ayn = 0;
    while let Some((&[aala, adna], baqi)) = khaam.split_first_chunk::<2>() {
        bayt[ayn] = qeemat_khana(aala) * 16 + qeemat_khana(adna);
        khaam = baqi;
        ayn += 1;
    }
    bayt
}

/// Whether two anchors hold the same bytes.
///
/// Walked a byte at a time because `==` is not available in a `const` context,
/// and this decides at compile time whether a build is telling the truth about
/// the identity it trusts.
const fn nafs_almiftah(awwal: &[u8; 32], thani: &[u8; 32]) -> bool {
    let mut baqi_awwal = awwal.as_slice();
    let mut baqi_thani = thani.as_slice();
    while let (Some((&sadr_awwal, dhayl_awwal)), Some((&sadr_thani, dhayl_thani))) =
        (baqi_awwal.split_first(), baqi_thani.split_first())
    {
        if sadr_awwal != sadr_thani {
            return false;
        }
        baqi_awwal = dhayl_awwal;
        baqi_thani = dhayl_thani;
    }
    true
}

/// The trust anchor compiled into this build.
///
/// A release build injects the release *public* key through the build-time
/// variable `TAARIB_MIFTAH_ISDAR`; the private half is generated inside the
/// owner's passphrase-protected keychain on the owner's machine and never
/// leaves it — no file, no fixture, no environment variable ever holds it.
/// Without the injection the build is a development build anchored to
/// [`MIFTAH_TATWIR`]. A malformed injected key, or one equal to the
/// development key, fails the build rather than producing a client that lies
/// about what it trusts.
pub const MIRSAT_MALIK: MirsatThiqa = match option_env!("TAARIB_MIFTAH_ISDAR") {
    Some(nassi) => {
        let miftah = fakk_sittashari(nassi);
        assert!(
            !nafs_almiftah(&miftah, &MIFTAH_TATWIR),
            "a release build cannot anchor to the committed development key"
        );
        MirsatThiqa { miftah, hawiya: HawiyatThiqa::Isdar }
    }
    None => MirsatThiqa { miftah: MIFTAH_TATWIR, hawiya: HawiyatThiqa::Tatwir },
};

// The release pipeline builds with this feature on, so a forgotten injection
// fails the build instead of shipping a client that trusts the development key.
#[cfg(feature = "isdar")]
const _: () = assert!(
    matches!(MIRSAT_MALIK.hawiya, HawiyatThiqa::Isdar),
    "a build with the `isdar` feature must be built with TAARIB_MIFTAH_ISDAR set"
);

/// The keychain entry name the owner's private half is stored under.
pub const ISM_MIFTAH_MALIK: &str = "malik";

/// Retrieves the owner's signing key from the keychain.
///
/// # Errors
///
/// [`KhataKhatm::MiftahMafqud`] when this machine's keychain holds no owner
/// key — the ordinary state on every machine but the owner's —
/// [`KhataKhatm::MiftahTalif`] when the stored seed does not derive this
/// build's trust anchor, and whatever the keychain itself refuses.
pub fn hat_malik() -> Result<MiftahKhass, KhataKhatm> {
    let khass = mafatih::hat(ISM_MIFTAH_MALIK)?;
    if khass.aam().bayt() != MIRSAT_MALIK.miftah {
        return Err(KhataKhatm::MiftahTalif);
    }
    Ok(khass)
}

/// Imports the owner's seed into this machine's keychain.
///
/// Refuses a seed that does not derive this build's trust anchor: importing
/// any other key would store an "owner key" no published package verifies
/// under, and in a release build that refusal covers the development seed too.
///
/// # Errors
///
/// [`KhataKhatm::MiftahTalif`] when the seed derives a different public key,
/// and whatever the keychain refuses.
pub fn khzin_malik(bidhra: &[u8; 32]) -> Result<MiftahAam, KhataKhatm> {
    let khass = MiftahKhass::min_bayt(bidhra);
    let aam = khass.aam();
    if aam.bayt() != MIRSAT_MALIK.miftah {
        return Err(KhataKhatm::MiftahTalif);
    }
    mafatih::khzin(ISM_MIFTAH_MALIK, &khass)?;
    Ok(aam)
}

/// Whether this machine holds the owner's private half.
#[must_use]
pub fn huwa_malik() -> bool {
    hat_malik().is_ok()
}
