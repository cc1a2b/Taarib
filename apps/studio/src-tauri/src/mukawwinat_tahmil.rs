//! تحميل المكوّنات — mirroring the bundled component tree into the store, with
//! named refusals.
//!
//! On startup [`zamin_mukawwinat`] reads the staging manifest the bundle
//! carries (`bayan_mukawwinat.json`, `docs/tawzee.md` §4), verifies every
//! listed file against its recorded size and sha256, and mirrors the
//! `mukawwinat/` subtree into the component store at [`Masarat::mukawwinat`].
//! Every copy is atomic — written to a `.juz` sibling, synced, then renamed —
//! and the store's own manifest is written last, only when every listed file
//! settled clean, so an interrupted mirror leaves the previous manifest in
//! place (`docs/tawzee.md` §5).
//!
//! Nothing here panics and nothing is skipped silently. A missing or corrupt
//! bundled artifact is a named problem inside [`NatijatZamin`]; a bundle that
//! carries no manifest at all is one named problem after which the studio
//! continues and installs refuse each absent component by name through
//! [`masar_mukawwin`]. That last one has two audiences and therefore two
//! messages — see [`BeeatBina`], which is what decides between them.
//!
//! The tauri resource directory is deliberately not resolved here: the caller
//! passes `app.path().resource_dir().join("mawarid")` in, so installs and this
//! mirror never read the bundle through two different resolutions.

// `main` declares this module private, so clippy reads every `pub(crate)` below as reachable
// only from inside it and asks for plain `pub`. Writing `pub` makes `unreachable_pub`, which
// the workspace denies, fire on the same item instead; the two rules only reconcile where the
// module is declared.
#![expect(
    clippy::redundant_pub_crate,
    reason = "`pub` here trips the workspace's denied `unreachable_pub` on a private module"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;
use taarib_usus::masarat::{Masarat, dakhil};

// The manifest's name, schema number, prefix and shape all come from
// `taarib_tathbeet::bayan_makhzan`, which is the crate the installer reaches
// the store through. Three parties need this file — `taarib-tajmee` writes it,
// this module mirrors it, the installer reads it — and the one that decides
// whether an install may proceed owns the definition.
pub(crate) use taarib_tathbeet::bayan_makhzan::{
    BADIYAT_MAKHZAN, BayanMukawwinat, ISM_MALAF_BAYAN, MUKHATTAT_MADUM, MalafMudraj,
};

/// The suffix a mirror copy carries until its bytes are verified and renamed.
const DHAYL_JUZ: &str = ".juz";

/// The largest staging manifest this build will read, in bytes. A real one is
/// a few kilobytes; anything near this limit is not a manifest.
const AQSA_HAJM_BAYAN: u64 = 8 * 1024 * 1024;

/// The component-store families the artifact matrix ships (`tawzee.md` §3):
/// BepInEx builds, Taarib's own loader, the C-ABI core, the Unity managed
/// assemblies, and the additive script-engine adapters.
const AILAT_MUKAWWINAT: [&str; 5] = ["bepinex", "mudkhal", "jisr", "unity", "mulhaq"];

// ---------------------------------------------------------------------------
// The manifest, exactly as §4 freezes it
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// What the mirror did
// ---------------------------------------------------------------------------

/// One file the mirror could not settle, named.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub(crate) struct MushkilatZamin {
    /// The matrix path exactly as the manifest lists it, or the manifest's own
    /// name when the manifest itself is the problem.
    pub masar: String,
    /// Why, as a full error: code, both sentences, and the remedy button.
    pub khata: Khata,
}

/// What the startup mirror did, in whole numbers and named problems.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub(crate) struct NatijatZamin {
    /// Files copied into the store because the store copy was absent or
    /// hash-different.
    pub munsakha: u32,
    /// Store files already byte-identical to the bundle, left untouched.
    pub mutatabiqa: u32,
    /// Bundle files that verified clean against the manifest, mirrored or not.
    pub muhaqqaqa: u32,
    /// Every file the mirror could not settle, named, in manifest order.
    pub mashakil: Vec<MushkilatZamin>,
}

impl NatijatZamin {
    /// An empty result, before any file has been looked at.
    const fn faragh() -> Self {
        Self {
            munsakha: 0,
            mutatabiqa: 0,
            muhaqqaqa: 0,
            mashakil: Vec::new(),
        }
    }

    /// A result that is one named problem and nothing else — how an absent or
    /// unreadable manifest is reported without touching the store.
    fn bi_mushkila(masar: &str, tafsir: &KhataMukawwinat) -> Self {
        let mut natija = Self::faragh();
        natija.sajjil(masar, tafsir);
        natija
    }

    /// Records one named problem against a matrix path.
    fn sajjil(&mut self, masar: &str, tafsir: &KhataMukawwinat) {
        self.mashakil.push(MushkilatZamin {
            masar: masar.to_owned(),
            khata: Khata::min_tafsir(tafsir),
        });
    }

    /// Whether every listed file settled clean and the store manifest was
    /// written.
    #[must_use]
    pub(crate) const fn salima(&self) -> bool {
        self.mashakil.is_empty()
    }

    /// The mirror as lines for the log and the diagnostics bundle.
    #[must_use]
    pub(crate) fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.mashakil.len().saturating_add(1));
        sutur.push(format!(
            "component mirror: {} copied, {} already identical, {} verified clean, {} problem(s)",
            self.munsakha,
            self.mutatabiqa,
            self.muhaqqaqa,
            self.mashakil.len()
        ));
        for mushkila in &self.mashakil {
            sutur.push(format!(
                "  {}: {}",
                mushkila.masar,
                mushkila.khata.li_sijill()
            ));
        }
        sutur
    }
}

/// How one listed file settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HalatMalaf {
    /// Verified in the bundle and copied into the store.
    Nusikha,
    /// Verified in the bundle; the store copy was already byte-identical.
    Mutatabiq,
    /// Verified in the bundle; not a store-bound entry, so nothing to mirror.
    MuhaqqaqFaqat,
}

// ---------------------------------------------------------------------------
// The startup mirror
// ---------------------------------------------------------------------------

/// Verifies the bundled component tree and mirrors it into the store.
///
/// Reads `<jidhr_mawarid>/bayan_mukawwinat.json`, checks every listed file's
/// size and streamed sha256 in the bundle, and copies into
/// [`Masarat::mukawwinat`] only the `mukawwinat/` entries whose store copy is
/// absent or hash-different — each through a `.juz` write, an fsync and a
/// rename, so the store never holds a half-written component. The store's own
/// manifest is written last and only when every listed file settled clean; on
/// any partial outcome the previous store manifest stays in place, which is
/// §5's partial-mirror rule.
///
/// This function never fails and never panics: every refusal, including a
/// bundle that carries no manifest at all — which [`BeeatBina`] then phrases
/// for whichever audience is reading it — is a named entry in
/// [`NatijatZamin::mashakil`]. The studio keeps
/// starting either way; installs that need an absent component refuse by name
/// later, at [`masar_mukawwin`] and `tarkib::hamil_mukawwin`.
#[must_use]
pub(crate) fn zamin_mukawwinat(jidhr_mawarid: &Path, masarat: &Masarat) -> NatijatZamin {
    let masar_bayan = jidhr_mawarid.join(ISM_MALAF_BAYAN);
    let bayt_bayan = match qira_bayan(&masar_bayan) {
        Ok(bayt) => bayt,
        Err(tafsir) => return sajjil_wa_arjii(NatijatZamin::bi_mushkila(ISM_MALAF_BAYAN, &tafsir)),
    };

    let bayan: BayanMukawwinat = match serde_json::from_slice(&bayt_bayan) {
        Ok(bayan) => bayan,
        Err(sabab) => {
            let tafsir = KhataMukawwinat::BayanTalif {
                masar: masar_bayan,
                tafsil: sabab.to_string(),
            };
            return sajjil_wa_arjii(NatijatZamin::bi_mushkila(ISM_MALAF_BAYAN, &tafsir));
        },
    };
    if bayan.mukhattat != MUKHATTAT_MADUM {
        let tafsir = KhataMukawwinat::MukhattatGhayrMadum {
            mukhattat: bayan.mukhattat,
        };
        return sajjil_wa_arjii(NatijatZamin::bi_mushkila(ISM_MALAF_BAYAN, &tafsir));
    }

    let jidhr_makhzan = masarat.mukawwinat();
    let mut natija = NatijatZamin::faragh();
    let mut maruda: BTreeSet<&str> = BTreeSet::new();

    for malaf in &bayan.milaffat {
        if !maruda.insert(malaf.masar.as_str()) {
            let tafsir = KhataMukawwinat::MasarMarfud {
                masar: malaf.masar.clone(),
                sabab: "the manifest lists this path twice".to_owned(),
            };
            natija.sajjil(&malaf.masar, &tafsir);
            continue;
        }
        match zamin_malaf(jidhr_mawarid, &jidhr_makhzan, malaf) {
            Ok(HalatMalaf::Nusikha) => {
                natija.munsakha = natija.munsakha.saturating_add(1);
                natija.muhaqqaqa = natija.muhaqqaqa.saturating_add(1);
            },
            Ok(HalatMalaf::Mutatabiq) => {
                natija.mutatabiqa = natija.mutatabiqa.saturating_add(1);
                natija.muhaqqaqa = natija.muhaqqaqa.saturating_add(1);
            },
            Ok(HalatMalaf::MuhaqqaqFaqat) => {
                natija.muhaqqaqa = natija.muhaqqaqa.saturating_add(1);
            },
            Err(tafsir) => natija.sajjil(&malaf.masar, &tafsir),
        }
    }

    // §5's partial-mirror rule: the store manifest is the completion marker,
    // written last and only after every listed file settled clean, so an
    // interrupted or partial mirror leaves the previous manifest in place.
    if natija.mashakil.is_empty()
        && let Err(tafsir) = iktub_bayan(&jidhr_makhzan, &bayt_bayan)
    {
        natija.sajjil(ISM_MALAF_BAYAN, &tafsir);
    }

    if natija.salima() {
        tracing::info!(
            isdar = %bayan.isdar,
            hadaf = %bayan.hadaf,
            munsakha = natija.munsakha,
            mutatabiqa = natija.mutatabiqa,
            muhaqqaqa = natija.muhaqqaqa,
            "the bundled component tree is mirrored into the store"
        );
    }
    sajjil_wa_arjii(natija)
}

/// Logs every named problem on the way out, so the log carries the diagnosis
/// even when nobody reads the returned report.
fn sajjil_wa_arjii(natija: NatijatZamin) -> NatijatZamin {
    for mushkila in &natija.mashakil {
        tracing::warn!(
            masar = %mushkila.masar,
            khata = %mushkila.khata.li_sijill(),
            "a bundled component did not settle into the store"
        );
    }
    natija
}

/// Reads the bundled manifest, distinguishing "there is none" from "there is
/// one and it cannot be read".
fn qira_bayan(masar: &Path) -> Result<Vec<u8>, KhataMukawwinat> {
    let wasf = match fs::metadata(masar) {
        Ok(wasf) => wasf,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Err(bayan_mafqud(masar));
        },
        Err(sabab) => {
            return Err(KhataMukawwinat::QiraatFashila {
                masar: masar.to_path_buf(),
                sabab,
            });
        },
    };
    if !wasf.is_file() {
        return Err(bayan_mafqud(masar));
    }
    if wasf.len() > AQSA_HAJM_BAYAN {
        return Err(KhataMukawwinat::BayanTalif {
            masar: masar.to_path_buf(),
            tafsil: format!(
                "{} byte(s) where a staging manifest of at most {AQSA_HAJM_BAYAN} was expected",
                wasf.len()
            ),
        });
    }
    match fs::read(masar) {
        Ok(bayt) => Ok(bayt),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Err(bayan_mafqud(masar)),
        Err(sabab) => Err(KhataMukawwinat::QiraatFashila {
            masar: masar.to_path_buf(),
            sabab,
        }),
    }
}

/// The absence, addressed to whoever is actually reading it.
///
/// One constructor rather than three literals so the audience can never be
/// decided in one arm of [`qira_bayan`] and forgotten in another.
fn bayan_mafqud(masar: &Path) -> KhataMukawwinat {
    KhataMukawwinat::BayanMafqud {
        masar: masar.to_path_buf(),
        beea: BeeatBina::hali(),
    }
}

/// Verifies one listed file in the bundle and settles its store copy.
///
/// # Errors
///
/// [`KhataMukawwinat::MasarMarfud`] for an entry that breaks the path or hash
/// rules, [`KhataMukawwinat::MalafGhaib`] when the bundle does not hold the
/// file, [`KhataMukawwinat::BasmaMukhtalifa`] when its size or streamed sha256
/// differs from the manifest, [`KhataMukawwinat::QiraatFashila`] when the
/// bundle cannot be read, and [`KhataMukawwinat::MakhzanGhayrKatib`] when the
/// store cannot be written.
fn zamin_malaf(
    jidhr_mawarid: &Path,
    jidhr_makhzan: &Path,
    malaf: &MalafMudraj,
) -> Result<HalatMalaf, KhataMukawwinat> {
    sahih_madkhal(malaf)?;
    let masar_huzma = dakhil_marsud(jidhr_mawarid, &malaf.masar)?;

    let wasf = match fs::metadata(&masar_huzma) {
        Ok(wasf) => wasf,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Err(KhataMukawwinat::MalafGhaib {
                masar: malaf.masar.clone(),
            });
        },
        Err(sabab) => {
            return Err(KhataMukawwinat::QiraatFashila {
                masar: masar_huzma,
                sabab,
            });
        },
    };
    // A directory or a special file where the manifest promised a regular file
    // is the promised file being absent.
    if !wasf.is_file() {
        return Err(KhataMukawwinat::MalafGhaib {
            masar: malaf.masar.clone(),
        });
    }
    if wasf.len() != malaf.hajm {
        return Err(KhataMukawwinat::BasmaMukhtalifa {
            masar: malaf.masar.clone(),
            tafsil: format!("declared {} byte(s), found {}", malaf.hajm, wasf.len()),
        });
    }

    // Only the component subtree lives in the store; fonts and the signature
    // database ride beside the binary and are verified without being mirrored.
    let Some(dhayl) = malaf.masar.strip_prefix(BADIYAT_MAKHZAN) else {
        tahaqqaq_basma(&masar_huzma, malaf)?;
        return Ok(HalatMalaf::MuhaqqaqFaqat);
    };
    let masar_makhzan = dakhil_marsud(jidhr_makhzan, dhayl)?;

    if mutatabiq(&masar_makhzan, malaf) {
        // The bundle copy is still verified: a corrupt bundled artifact is a
        // named diagnostic even when the store already holds a good copy.
        tahaqqaq_basma(&masar_huzma, malaf)?;
        return Ok(HalatMalaf::Mutatabiq);
    }
    unsakh_muhaqqaqan(&masar_huzma, &masar_makhzan, malaf)?;
    Ok(HalatMalaf::Nusikha)
}

/// Structural rules for one manifest entry: forward-slash relative path with
/// no parent, absolute, backslash, colon, dot or empty component, and a
/// declared hash that is really a sha256.
fn sahih_madkhal(malaf: &MalafMudraj) -> Result<(), KhataMukawwinat> {
    let marfud = |sabab: &str| KhataMukawwinat::MasarMarfud {
        masar: malaf.masar.clone(),
        sabab: sabab.to_owned(),
    };
    if malaf.masar.is_empty() {
        return Err(marfud("the entry's path is empty"));
    }
    if malaf.masar.contains('\0') {
        return Err(marfud("the path carries an embedded NUL"));
    }
    if malaf.masar.contains('\\') {
        return Err(marfud(
            "the path uses backslashes; the manifest is forward-slash only",
        ));
    }
    if malaf.masar.starts_with('/') {
        return Err(marfud("the path is absolute"));
    }
    if malaf.masar.contains(':') {
        return Err(marfud("the path carries a drive or stream separator"));
    }
    for juz in malaf.masar.split('/') {
        if juz.is_empty() {
            return Err(marfud("the path carries an empty component"));
        }
        if juz == "." || juz == ".." {
            return Err(marfud("the path carries a dot or parent component"));
        }
    }
    let basma_saliha = malaf.sha256.len() == 64
        && malaf
            .sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    if basma_saliha {
        Ok(())
    } else {
        Err(marfud(
            "the declared sha256 is not 64 lowercase hexadecimal characters",
        ))
    }
}

/// Joins a manifest path onto a root through the workspace's one guarded join,
/// which also refuses reserved device names and over-long results.
fn dakhil_marsud(jidhr: &Path, nisbi: &str) -> Result<PathBuf, KhataMukawwinat> {
    dakhil(jidhr, nisbi).map_err(|khata| KhataMukawwinat::MasarMarfud {
        masar: nisbi.to_owned(),
        sabab: khata.injilizi,
    })
}

/// Whether the store already holds this exact file: present, the declared
/// size, and the declared hash.
fn mutatabiq(masar_makhzan: &Path, malaf: &MalafMudraj) -> bool {
    let Ok(wasf) = fs::metadata(masar_makhzan) else {
        return false;
    };
    if !wasf.is_file() || wasf.len() != malaf.hajm {
        return false;
    }
    match basmat_malaf(masar_makhzan) {
        Ok(basma) => basma == malaf.sha256,
        Err(sabab) => {
            // Not identical is all this answer claims; if the store is truly
            // unreadable, the rewrite that follows will name it loudly.
            tracing::debug!(
                masar = %masar_makhzan.display(),
                sabab = %sabab,
                "an existing store copy could not be hashed and will be rewritten"
            );
            false
        },
    }
}

/// Streams one bundle file through sha256 and compares it to the manifest.
fn tahaqqaq_basma(masar_huzma: &Path, malaf: &MalafMudraj) -> Result<(), KhataMukawwinat> {
    match basmat_malaf(masar_huzma) {
        Ok(mahsuba) if mahsuba == malaf.sha256 => Ok(()),
        Ok(mahsuba) => Err(KhataMukawwinat::BasmaMukhtalifa {
            masar: malaf.masar.clone(),
            tafsil: format!("declared sha256 {}, computed {mahsuba}", malaf.sha256),
        }),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            Err(KhataMukawwinat::MalafGhaib {
                masar: malaf.masar.clone(),
            })
        },
        Err(sabab) => Err(KhataMukawwinat::QiraatFashila {
            masar: masar_huzma.to_path_buf(),
            sabab,
        }),
    }
}

/// The streamed sha256 of one file, as 64 lowercase hexadecimal characters.
fn basmat_malaf(masar: &Path) -> std::io::Result<String> {
    let mut qari = fs::File::open(masar)?;
    let mut hashib = Sha256::new();
    let _ = std::io::copy(&mut qari, &mut hashib)?;
    Ok(hex_saghir(hashib.finalize().as_slice()))
}

/// Feeds every written byte to both the store file and the digest, so the
/// bytes that land in the store are exactly the bytes that were hashed.
#[derive(Debug)]
struct KatibMuhashib<'a> {
    malaf: &'a mut fs::File,
    hashib: &'a mut Sha256,
    kitaba_fashilat: bool,
}

impl std::io::Write for KatibMuhashib<'_> {
    fn write(&mut self, bayt: &[u8]) -> std::io::Result<usize> {
        if let Err(sabab) = self.malaf.write_all(bayt) {
            self.kitaba_fashilat = true;
            return Err(sabab);
        }
        self.hashib.update(bayt);
        Ok(bayt.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let natija = self.malaf.flush();
        if natija.is_err() {
            self.kitaba_fashilat = true;
        }
        natija
    }
}

/// Copies one bundle file into the store atomically, hashing the very bytes
/// being written: stream to `<hadaf>.juz`, refuse a digest that is not the
/// declared one, fsync, then rename over the target.
///
/// # Errors
///
/// [`KhataMukawwinat::MalafGhaib`] when the bundle file vanished,
/// [`KhataMukawwinat::QiraatFashila`] when it cannot be read,
/// [`KhataMukawwinat::BasmaMukhtalifa`] when the streamed bytes do not hash to
/// the manifest's value, and [`KhataMukawwinat::MakhzanGhayrKatib`] when the
/// store cannot be created, written, synced or renamed into. On every failure
/// the `.juz` remainder is removed and the target is left untouched.
fn unsakh_muhaqqaqan(min: &Path, ila: &Path, malaf: &MalafMudraj) -> Result<(), KhataMukawwinat> {
    let Some(walid) = ila.parent() else {
        return Err(KhataMukawwinat::MasarMarfud {
            masar: malaf.masar.clone(),
            sabab: "the store destination has no parent directory".to_owned(),
        });
    };
    fs::create_dir_all(walid).map_err(|sabab| KhataMukawwinat::MakhzanGhayrKatib {
        masar: walid.to_path_buf(),
        sabab,
    })?;

    let mut qari = match fs::File::open(min) {
        Ok(qari) => qari,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Err(KhataMukawwinat::MalafGhaib {
                masar: malaf.masar.clone(),
            });
        },
        Err(sabab) => {
            return Err(KhataMukawwinat::QiraatFashila {
                masar: min.to_path_buf(),
                sabab,
            });
        },
    };

    let masar_juz = masar_juz(ila);
    let mut katib =
        fs::File::create(&masar_juz).map_err(|sabab| KhataMukawwinat::MakhzanGhayrKatib {
            masar: masar_juz.clone(),
            sabab,
        })?;

    let mut hashib = Sha256::new();
    let mut tawzi = KatibMuhashib {
        malaf: &mut katib,
        hashib: &mut hashib,
        kitaba_fashilat: false,
    };
    if let Err(sabab) = std::io::copy(&mut qari, &mut tawzi) {
        let kitaba = tawzi.kitaba_fashilat;
        imsah_juz(&masar_juz);
        return Err(if kitaba {
            KhataMukawwinat::MakhzanGhayrKatib {
                masar: masar_juz,
                sabab,
            }
        } else {
            KhataMukawwinat::QiraatFashila {
                masar: min.to_path_buf(),
                sabab,
            }
        });
    }

    let mahsuba = hex_saghir(hashib.finalize().as_slice());
    if mahsuba != malaf.sha256 {
        imsah_juz(&masar_juz);
        return Err(KhataMukawwinat::BasmaMukhtalifa {
            masar: malaf.masar.clone(),
            tafsil: format!("declared sha256 {}, streamed {mahsuba}", malaf.sha256),
        });
    }

    if let Err(sabab) = katib.sync_all() {
        imsah_juz(&masar_juz);
        return Err(KhataMukawwinat::MakhzanGhayrKatib {
            masar: masar_juz,
            sabab,
        });
    }
    drop(katib);
    if let Err(sabab) = fs::rename(&masar_juz, ila) {
        imsah_juz(&masar_juz);
        return Err(KhataMukawwinat::MakhzanGhayrKatib {
            masar: ila.to_path_buf(),
            sabab,
        });
    }

    // The rename is atomic, but its directory entry is not durable until the
    // directory is flushed; best effort, exactly as `masarat::kitaba_dharra`.
    #[cfg(unix)]
    if let Ok(maftuh) = fs::File::open(walid) {
        drop(maftuh.sync_all());
    }
    Ok(())
}

/// Writes the store's own copy of the manifest, `.juz` then rename, same
/// durability as every mirrored file.
///
/// # Errors
///
/// [`KhataMukawwinat::MakhzanGhayrKatib`] naming whichever step the store
/// refused; the previous manifest, if any, is left in place.
fn iktub_bayan(jidhr_makhzan: &Path, bayt: &[u8]) -> Result<(), KhataMukawwinat> {
    fs::create_dir_all(jidhr_makhzan).map_err(|sabab| KhataMukawwinat::MakhzanGhayrKatib {
        masar: jidhr_makhzan.to_path_buf(),
        sabab,
    })?;
    let hadaf = jidhr_makhzan.join(ISM_MALAF_BAYAN);
    let masar_juz = masar_juz(&hadaf);

    let natija = fs::File::create(&masar_juz).and_then(|mut katib| {
        katib.write_all(bayt)?;
        katib.sync_all()
    });
    if let Err(sabab) = natija {
        imsah_juz(&masar_juz);
        return Err(KhataMukawwinat::MakhzanGhayrKatib {
            masar: masar_juz,
            sabab,
        });
    }
    if let Err(sabab) = fs::rename(&masar_juz, &hadaf) {
        imsah_juz(&masar_juz);
        return Err(KhataMukawwinat::MakhzanGhayrKatib {
            masar: hadaf,
            sabab,
        });
    }
    #[cfg(unix)]
    if let Ok(maftuh) = fs::File::open(jidhr_makhzan) {
        drop(maftuh.sync_all());
    }
    Ok(())
}

/// The `.juz` sibling a target is staged through.
fn masar_juz(hadaf: &Path) -> PathBuf {
    let mut ism = hadaf.as_os_str().to_owned();
    ism.push(DHAYL_JUZ);
    PathBuf::from(ism)
}

/// Removes a leftover `.juz`, best effort: the store stays consistent without
/// it, and the failure that led here is already the one being reported.
fn imsah_juz(masar: &Path) {
    if let Err(sabab) = fs::remove_file(masar)
        && sabab.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!(
            masar = %masar.display(),
            sabab = %sabab,
            "a temporary mirror file was left behind"
        );
    }
}

/// Lowercase hexadecimal, the spelling the manifest and the store compare in.
fn hex_saghir(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut nass = String::with_capacity(bayt.len().saturating_mul(2));
    for wahid in bayt {
        let _ = write!(nass, "{wahid:02x}");
    }
    nass
}

// ---------------------------------------------------------------------------
// Resolving a component out of the store
// ---------------------------------------------------------------------------

/// Resolves one component directory inside the store, by the exact names
/// `taarib_tathbeet::tarkib` constructs — `bepinex/<hadaf>/<khalfiya>-<jeel>-
/// <mimariya>`, `mudkhal/<hadaf>/<mimariya>` — plus the `jisr/…`, `unity` and
/// `mulhaq/…` families from the artifact matrix.
///
/// # Errors
///
/// [`KhataMukawwinat::MukawwinGhayrMaruf`] when the name's family is not one
/// the artifact matrix ships, [`KhataMukawwinat::MasarMarfud`] when the name
/// does not join onto the store root as a contained path,
/// [`KhataMukawwinat::MukawwinMafqud`] when the directory is absent or holds
/// no regular file, and [`KhataMukawwinat::QiraatFashila`] when the store
/// cannot be walked to answer. Never panics.
#[expect(
    dead_code,
    reason = "the studio-side install path that resolves a component out of the store is not \
              wired yet; removing this would take the store's completeness gate with it"
)]
pub(crate) fn masar_mukawwin(masarat: &Masarat, ism: &str) -> Result<PathBuf, KhataMukawwinat> {
    let aila = ism.split('/').next().unwrap_or_default();
    if !AILAT_MUKAWWINAT.contains(&aila) {
        return Err(KhataMukawwinat::MukawwinGhayrMaruf {
            ism: ism.to_owned(),
        });
    }

    let jidhr_makhzan = masarat.mukawwinat();
    let masar = dakhil(&jidhr_makhzan, ism).map_err(|khata| KhataMukawwinat::MasarMarfud {
        masar: ism.to_owned(),
        sabab: khata.injilizi,
    })?;

    if !masar.is_dir() {
        return Err(KhataMukawwinat::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar,
        });
    }
    kamil_hasab_bayan(&jidhr_makhzan, ism)?;
    Ok(masar)
}

/// Proves every file the store's manifest lists for a component is present at
/// its declared size.
///
/// "Holds at least one regular file" is not completeness. A mirror interrupted
/// after three of a component's forty files leaves a directory that answers
/// yes to that question, and an install reading it deploys three files, reports
/// success, and leaves the game in English with nothing logged. The manifest is
/// the only thing that knows how many there should be, and it is written last
/// and only when every file settled clean — so its presence is what makes this
/// question answerable at all.
///
/// Sizes rather than hashes: the bytes were hashed on the way into the store,
/// and re-hashing a BepInEx tree on every install would put seconds onto a
/// confirmation screen to re-answer a question already answered. A truncated
/// or replaced file changes its length, which is what this catches.
fn kamil_hasab_bayan(jidhr_makhzan: &Path, ism: &str) -> Result<(), KhataMukawwinat> {
    let bayt = qira_bayan(&jidhr_makhzan.join(ISM_MALAF_BAYAN))?;
    let masar_bayan = jidhr_makhzan.join(ISM_MALAF_BAYAN);
    let bayan: BayanMukawwinat =
        serde_json::from_slice(&bayt).map_err(|sabab| KhataMukawwinat::BayanTalif {
            masar: masar_bayan,
            tafsil: sabab.to_string(),
        })?;
    if bayan.mukhattat != MUKHATTAT_MADUM {
        return Err(KhataMukawwinat::MukhattatGhayrMadum {
            mukhattat: bayan.mukhattat,
        });
    }

    let badiya = format!("{BADIYAT_MAKHZAN}{ism}/");
    let mut adad = 0_usize;
    for malaf in &bayan.milaffat {
        let Some(dhayl) = malaf.masar.strip_prefix(BADIYAT_MAKHZAN) else {
            continue;
        };
        if !malaf.masar.starts_with(&badiya) {
            continue;
        }
        adad = adad.saturating_add(1);
        let masar = dakhil_marsud(jidhr_makhzan, dhayl)?;
        let wasf = match fs::metadata(&masar) {
            Ok(wasf) => wasf,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Err(KhataMukawwinat::MukawwinMafqud {
                    mukawwin: format!("{ism} (missing {dhayl})"),
                    masar,
                });
            },
            Err(sabab) => return Err(KhataMukawwinat::QiraatFashila { masar, sabab }),
        };
        if !wasf.is_file() || wasf.len() != malaf.hajm {
            return Err(KhataMukawwinat::BasmaMukhtalifa {
                masar: malaf.masar.clone(),
                tafsil: format!("declared {} byte(s), found {}", malaf.hajm, wasf.len()),
            });
        }
    }

    // A component the manifest never listed is not a component this build
    // ships, whatever happens to be sitting in a directory with its name.
    if adad == 0 {
        return Err(KhataMukawwinat::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr_makhzan.join(ism),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Failures
// ---------------------------------------------------------------------------

/// Who is reading a message about a missing staging manifest.
///
/// The same absence means two unrelated things. To somebody running the studio
/// out of the source tree it is the normal state of a `cargo run`, and the only
/// action is a build step. To somebody who downloaded an installer it means the
/// package they were given is incomplete, they have no repository and no cargo,
/// and the only action is to update, reinstall, or tell the project. Sending the
/// first sentence to the second reader — which is what shipped — asks a stranger
/// to run a tool they do not have from a directory they do not own.
///
/// The evidence is the one cargo itself leaves: cargo writes a `.cargo-lock`
/// file into every output directory it links a binary into, which is the same
/// marker `tauri_utils::platform::resource_dir` uses to decide that an
/// executable is a development one. Nothing an installer produces contains it,
/// on any of the three platforms. It is checked at the moment the message is
/// built rather than compiled in, because the compile-time answer is wrong for
/// the case that matters: `tauri::is_dev` is `!cfg!(feature = "custom-protocol")`
/// and this crate declares no such feature, so it would call every build a
/// development build including the installed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeeatBina {
    /// The executable sits in a cargo output directory: a developer.
    MinCargo,
    /// Anywhere else: an installed, downloaded or otherwise distributed build.
    Muwazzaa,
}

impl BeeatBina {
    /// What this executable is, from where it is sitting.
    ///
    /// An executable whose own path cannot be resolved is treated as
    /// distributed. That is the honest default in both directions: it is what a
    /// hardened or relocated installation looks like, and a developer who sees
    /// the distributed sentence loses nothing but a line of advice they did not
    /// need, where a user who sees the development sentence is told to run a
    /// program that is not on their machine.
    fn hali() -> Self {
        let min_cargo = std::env::current_exe()
            .ok()
            .and_then(|tanfidhi| tanfidhi.parent().map(Path::to_path_buf))
            .is_some_and(|mujallad| mujallad.join(MARSAD_CARGO).exists());
        if min_cargo {
            Self::MinCargo
        } else {
            Self::Muwazzaa
        }
    }

    /// The value the log and the diagnostics bundle carry, so which audience
    /// this build decided on is a fact somebody can check rather than infer
    /// from the wording.
    const fn wasm(self) -> &'static str {
        match self {
            Self::MinCargo => "min_cargo",
            Self::Muwazzaa => "muwazzaa",
        }
    }
}

/// The file cargo leaves beside every binary it links, and the marker
/// `tauri_utils` reads for the same question.
const MARSAD_CARGO: &str = ".cargo-lock";

/// Failures of the component mirror and the component store lookup.
#[derive(Debug, thiserror::Error)]
pub(crate) enum KhataMukawwinat {
    /// The bundle carries no staging manifest. Expected from cargo, and a sign
    /// that the package is incomplete anywhere else; `beea` is which. The studio
    /// continues either way and installs refuse absent components by name later.
    #[error("the bundle carries no staging manifest at {masar}")]
    BayanMafqud {
        /// Where the manifest was looked for.
        masar: PathBuf,
        /// Who is being told about it.
        beea: BeeatBina,
    },

    /// The staging manifest exists and is not readable as §4's document.
    #[error("the staging manifest at {masar} could not be read as a manifest: {tafsil}")]
    BayanTalif {
        /// The manifest that was being read.
        masar: PathBuf,
        /// What the reader or the parser said.
        tafsil: String,
    },

    /// The manifest is written to a schema number this build does not
    /// understand.
    #[error("the staging manifest is schema {mukhattat}, which this build does not understand")]
    MukhattatGhayrMadum {
        /// The schema number the manifest declares.
        mukhattat: u32,
    },

    /// A manifest entry breaks the path or hash rules, or a component name
    /// does not join onto its root as a contained path.
    #[error("refused a manifest path: {masar} ({sabab})")]
    MasarMarfud {
        /// The offending path, as listed.
        masar: String,
        /// Which rule it broke.
        sabab: String,
    },

    /// A file the manifest lists is not in the bundle.
    #[error("the bundle does not hold {masar}, which its manifest lists")]
    MalafGhaib {
        /// The matrix path, as the manifest lists it.
        masar: String,
    },

    /// A bundle or store path could not be read while loading components.
    #[error("cannot read {masar} while loading components")]
    QiraatFashila {
        /// The path.
        masar: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A bundle file's size or streamed sha256 differs from what the manifest
    /// recorded, so its bytes never enter the store.
    #[error("{masar} does not match its manifest entry: {tafsil}")]
    BasmaMukhtalifa {
        /// The matrix path, as the manifest lists it.
        masar: String,
        /// The declared value against the measured one.
        tafsil: String,
    },

    /// The component store refused a write — creating, writing, syncing or
    /// renaming.
    #[error("cannot write the component store at {masar}")]
    MakhzanGhayrKatib {
        /// The store path that was being written.
        masar: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A component was requested under a family the artifact matrix does not
    /// ship, which is a defect in the caller rather than a damaged store.
    #[error("{ism} is not a component family the artifact matrix ships")]
    MukawwinGhayrMaruf {
        /// The requested component name.
        ism: String,
    },

    /// The component is not in the store: its directory is absent, or holds
    /// not a single regular file.
    #[error("the component {mukawwin} is not in the component store at {masar}")]
    MukawwinMafqud {
        /// The component that was asked for.
        mukawwin: String,
        /// Where it was expected.
        masar: PathBuf,
    },
}

impl Tafsir for KhataMukawwinat {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::BayanMafqud { .. } => 110,
                    Self::BayanTalif { .. } => 111,
                    Self::MukhattatGhayrMadum { .. } => 112,
                    Self::MasarMarfud { .. } => 113,
                    Self::MalafGhaib { .. } => 114,
                    Self::QiraatFashila { .. } => 115,
                    Self::BasmaMukhtalifa { .. } => 116,
                    Self::MakhzanGhayrKatib { .. } => 117,
                    Self::MukawwinGhayrMaruf { .. } => 118,
                    Self::MukawwinMafqud { .. } => 119,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Expected from cargo, where the studio continues untouched. In a
            // distributed build the same absence is an incomplete package, and
            // it drops through to the severity every other incomplete-package
            // failure carries.
            Self::BayanMafqud {
                beea: BeeatBina::MinCargo,
                ..
            } => Khutura::Tanbeeh,
            // A traversal attempt means a corrupt or hostile manifest.
            Self::MasarMarfud { .. } => Khutura::Fadih,
            Self::BayanMafqud {
                beea: BeeatBina::Muwazzaa,
                ..
            }
            | Self::BayanTalif { .. }
            | Self::MukhattatGhayrMadum { .. }
            | Self::MalafGhaib { .. }
            | Self::QiraatFashila { .. }
            | Self::BasmaMukhtalifa { .. }
            | Self::MakhzanGhayrKatib { .. }
            | Self::MukawwinGhayrMaruf { .. }
            | Self::MukawwinMafqud { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::BayanMafqud {
                beea: BeeatBina::MinCargo,
                masar,
            } => format!(
                "لا يوجد بيان مكوّنات في {}، وهذا هو الوضع المتوقَّع لبناء يعمل من cargo إذ لا حزمة \
                 له أصلًا. سيستمر تعريب، وسترفض التثبيتات لاحقًا كل مكوّن غائب باسمه. لتجربة \
                 التثبيت كاملًا شغِّل أداة الترحيل taarib-tajmee ثم أعد التشغيل.",
                masar.display()
            ),
            Self::BayanMafqud {
                beea: BeeatBina::Muwazzaa,
                ..
            } => "نسخة تعريب المثبَّتة عندك ناقصة: حُزمت بدون بيان المكوّنات، فلن تكتمل أي عملية \
                 تثبيت تحتاج مكوّنًا، وسيُرفض كل مكوّن غائب باسمه. لا شيء في جهازك سبّب هذا. \
                 حدِّث تعريب أو أعد تثبيته من صفحة الإصدارات؛ فإن ظهرت الرسالة نفسها بعد تثبيت \
                 نظيف فأبلغ مالك المشروع وأرفق حزمة التشخيص."
                .to_owned(),
            Self::BayanTalif { .. } => {
                "تعذّرت قراءة بيان المكوّنات المرفق بهذه النسخة، أو أن محتواه ليس بالصيغة \
                 المتوقعة. النسخة ناقصة أو تالفة؛ شغِّل المحدِّث أو أعد تثبيت تعريب."
                    .to_owned()
            },
            Self::MukhattatGhayrMadum { mukhattat } => format!(
                "بيان المكوّنات مكتوب بمخطط ({mukhattat}) لا تفهمه هذه النسخة من تعريب. حدِّث تعريب \
                 إلى نسخة أحدث."
            ),
            Self::MasarMarfud { masar, .. } => format!(
                "رُفض مسار في بيان المكوّنات يخالف قواعد المسارات: {masar}. هذا يدلّ على بيان تالف \
                 أو محتوى غير موثوق؛ أبلغ مالك المشروع."
            ),
            Self::MalafGhaib { masar } => format!(
                "الملف {masar} مذكور في بيان المكوّنات لكنه غير موجود في حزمة هذه النسخة. النسخة \
                 ناقصة؛ شغِّل المحدِّث أو أعد تثبيت تعريب."
            ),
            Self::QiraatFashila { masar, .. } => format!(
                "تعذّرت قراءة {} أثناء تحميل المكوّنات. أعد المحاولة؛ فإن تكرّر الأمر فشغِّل \
                 المحدِّث أو أعد تثبيت تعريب.",
                masar.display()
            ),
            Self::BasmaMukhtalifa { masar, .. } => format!(
                "بصمة الملف {masar} في الحزمة لا تطابق ما سجّله بيان المكوّنات، ولن يدخل المخزنَ \
                 شيء لم تثبت بصمته. الملف تالف؛ شغِّل المحدِّث أو أعد تثبيت تعريب."
            ),
            Self::MakhzanGhayrKatib { masar, .. } => format!(
                "تعذّرت الكتابة إلى مخزن المكوّنات عند {}. تأكد من الصلاحيات ومن المساحة المتاحة \
                 ثم أعد تشغيل تعريب.",
                masar.display()
            ),
            Self::MukawwinGhayrMaruf { ism } => format!(
                "طُلب مكوّن باسم لا يرد في مصفوفة مكوّنات تعريب: {ism}. هذا خلل في هذه النسخة لا \
                 في جهازك؛ أبلغ مالك المشروع."
            ),
            Self::MukawwinMafqud { mukawwin, .. } => format!(
                "المكوّن {mukawwin} غير موجود في مخزن مكوّنات تعريب. شغِّل المحدِّث أو أعد تثبيت \
                 تعريب ليُعاد نسخه من الحزمة."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::BayanMafqud {
                beea: BeeatBina::MinCargo,
                masar,
            } => format!(
                "No component manifest at {}, which is the expected state of a build run from \
                 cargo: it has no bundle to carry one. The studio continues, and installs will \
                 refuse each absent component by name. To exercise a real install, run the \
                 staging tool (taarib-tajmee), then relaunch.",
                masar.display()
            ),
            Self::BayanMafqud {
                beea: BeeatBina::Muwazzaa,
                ..
            } => "This installation of Taarib is incomplete: it was packaged without its \
                 component manifest, so any install that needs a component cannot finish and \
                 every absent component will be refused by name. Nothing on your machine caused \
                 this. Update Taarib, or reinstall it from the releases page; if a clean \
                 install shows the same message, report it to the project owner and attach a \
                 diagnostics bundle."
                .to_owned(),
            Self::BayanTalif { masar, tafsil } => format!(
                "The staging manifest at {} could not be read as a manifest: {tafsil}. This \
                 build is incomplete or damaged; run the updater or reinstall Taarib.",
                masar.display()
            ),
            Self::MukhattatGhayrMadum { mukhattat } => format!(
                "The staging manifest is written to schema {mukhattat}, which this build does \
                 not understand. Update Taarib to a newer version."
            ),
            Self::MasarMarfud { masar, sabab } => format!(
                "Refused a manifest path that breaks the path rules: {masar} ({sabab}). This \
                 indicates a corrupt manifest or untrusted content; report it to the project \
                 owner."
            ),
            Self::MalafGhaib { masar } => format!(
                "The manifest lists {masar} but the bundle does not hold it. This build is \
                 incomplete; run the updater or reinstall Taarib."
            ),
            Self::QiraatFashila { masar, .. } => format!(
                "Cannot read {} while loading components. Try again; if it persists, run the \
                 updater or reinstall Taarib.",
                masar.display()
            ),
            Self::BasmaMukhtalifa { masar, tafsil } => format!(
                "{masar} does not match its manifest entry ({tafsil}), so its bytes never \
                 enter the component store. The file is damaged; run the updater or reinstall \
                 Taarib."
            ),
            Self::MakhzanGhayrKatib { masar, .. } => format!(
                "Cannot write the component store at {}. Check permissions and free space, \
                 then restart Taarib.",
                masar.display()
            ),
            Self::MukawwinGhayrMaruf { ism } => format!(
                "{ism} is not a component family the artifact matrix ships. This is a defect \
                 in this build, not in your machine; report it to the project owner."
            ),
            Self::MukawwinMafqud { mukawwin, masar } => format!(
                "The component {mukawwin} is not in the component store at {}. Run the \
                 updater or reinstall Taarib so it is mirrored back from the bundle.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // From cargo there is no button that conjures a bundle. From a
            // distributed build there is one, and it is the same button every
            // other incomplete-package failure offers: get a complete build.
            Self::BayanMafqud {
                beea: BeeatBina::MinCargo,
                ..
            } => Khutwa::LaShay,
            Self::BayanMafqud {
                beea: BeeatBina::Muwazzaa,
                ..
            }
            | Self::BayanTalif { .. }
            | Self::MukhattatGhayrMadum { .. }
            | Self::MalafGhaib { .. }
            | Self::BasmaMukhtalifa { .. }
            | Self::MukawwinMafqud { .. } => Khutwa::TahdithTaarib,
            Self::MasarMarfud { .. } | Self::MukawwinGhayrMaruf { .. } => Khutwa::IblaghLilMalik,
            Self::QiraatFashila { sabab, .. } | Self::MakhzanGhayrKatib { sabab, .. } => {
                khutwa_io(sabab, MasarMatlub::MujalladRuqaa)
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::BayanMafqud { masar, beea } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                // The decision itself, so a bug report shows which audience this
                // build picked instead of leaving it to be inferred from prose.
                let _ = siyaq.insert("beea".to_owned(), QeemaSiyaq::Nass(beea.wasm().to_owned()));
            },
            Self::BayanTalif { masar, tafsil } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::MukhattatGhayrMadum { mukhattat } => {
                let _ = siyaq.insert(
                    "mukhattat".to_owned(),
                    QeemaSiyaq::Raqm(i64::from(*mukhattat)),
                );
            },
            Self::MasarMarfud { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MalafGhaib { masar } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(masar.clone()));
            },
            Self::QiraatFashila { masar, sabab } | Self::MakhzanGhayrKatib { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                siyaq.extend(siyaq_io(sabab));
            },
            Self::BasmaMukhtalifa { masar, tafsil } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(masar.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::MukawwinGhayrMaruf { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::MukawwinMafqud { mukawwin, masar } => {
                let _ = siyaq.insert("mukawwin".to_owned(), QeemaSiyaq::Nass(mukawwin.clone()));
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataMukawwinat);

/// The subtree of the bundle the fonts are staged into.
const BADIYAT_KHUTUT: &str = "khutut";

/// The bundle's resource root, recorded once at startup.
///
/// A process-wide value rather than managed state: it is a property of the
/// running bundle, identical for every command and every request, and threading
/// it through four call chains would say otherwise. Unset in a `cargo run`
/// build, where there is no bundle — every reader treats that as "no bundled
/// artifacts", which is the truth there.
static JIDHR_MAWARID: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Records the resource root. The first call wins; later ones are ignored.
pub(crate) fn sajjil_jidhr_mawarid(jidhr: PathBuf) {
    let _ = JIDHR_MAWARID.set(jidhr);
}

/// The resource root, when this process was launched from a bundle.
#[must_use]
pub(crate) fn jidhr_mawarid() -> Option<&'static Path> {
    JIDHR_MAWARID.get().map(PathBuf::as_path)
}

/// Every font root a session may read, in precedence order.
///
/// The user's own directory first, then the bundle's. Both, rather than a
/// mirror into one: the bundled set is read-only and belongs to the build, and
/// copying it into the user's directory would put thirty files a user never
/// chose where they keep the ones they did — and put them back every launch
/// after they deleted them.
#[must_use]
pub(crate) fn judhur_khutut(masarat: &Masarat) -> Vec<PathBuf> {
    let mut judhur = vec![masarat.khutut()];
    if let Some(jidhr) = jidhr_mawarid() {
        judhur.push(jidhr.join(BADIYAT_KHUTUT));
    }
    judhur
}

/// Every usable font file under a set of roots, recursively, sorted.
///
/// Recursive because the bundled tree keeps a directory per class exactly as
/// `assets/fonts/khutut.json` names them, and a flat read would see `sans` as
/// an entry with no font extension and skip a whole family.
#[must_use]
pub(crate) fn milaffat_khutut(judhur: &[PathBuf]) -> Vec<PathBuf> {
    let mut khraj = Vec::new();
    // A name claimed by an earlier root is not offered again by a later one.
    // Sorting the whole set and then dropping adjacent duplicates cannot do
    // this: two roots hold the same file name at two full paths that sort
    // nowhere near each other, so the duplicate survives and the root order
    // that decides which copy wins is destroyed by the sort.
    let mut maakhudha: std::collections::HashSet<String> = std::collections::HashSet::new();
    for jidhr in judhur {
        let mut min_jidhr = Vec::new();
        let mut tabur = vec![jidhr.clone()];
        while let Some(hali) = tabur.pop() {
            let Ok(qaima) = fs::read_dir(&hali) else {
                continue;
            };
            for dakhla in qaima.flatten() {
                let masar = dakhla.path();
                if masar.is_dir() {
                    tabur.push(masar);
                    continue;
                }
                let lahiqa = masar
                    .extension()
                    .and_then(|q| q.to_str())
                    .map(str::to_ascii_lowercase);
                if matches!(lahiqa.as_deref(), Some("ttf" | "otf")) {
                    min_jidhr.push(masar);
                }
            }
        }
        min_jidhr.sort();
        for masar in min_jidhr {
            // Case-folded, because two of the three platforms would treat
            // `Amiri.ttf` and `amiri.ttf` as one file and the third would not.
            let Some(ism) = masar.file_name() else {
                continue;
            };
            if maakhudha.insert(ism.to_string_lossy().to_lowercase()) {
                khraj.push(masar);
            }
        }
    }
    khraj
}
