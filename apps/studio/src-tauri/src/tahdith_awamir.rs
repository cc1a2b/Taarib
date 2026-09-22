//! التحديث — the update surface: check the signed channel, fetch, and swap.

use std::path::PathBuf;

use taarib_mustawda::masadir::{MUHLAT_ITTISAL, MUHLAT_TALAB};
use taarib_mustawda::{KhataMustawda, MasdarMustawda};
use taarib_tahdith::bayan::{BayanTahdith, MadkhalTahdith, QANAT_MUSTAQIRR, QANAT_TAJRIBI};
use taarib_tahdith::{ihlil, intiqa, jalb, tabdil};
use taarib_usus::ISDAR;
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat::Masarat;

use std::sync::Arc;

/// The channel manifest's name under the registry root.
const ISM_BAYAN: &str = "tahdith.json";

/// Its detached signature, beside it.
const ISM_TAWQEE: &str = "tahdith.json.tawqee";

/// What an update check found.
///
/// Four answers rather than an offer and an error, because the three that are
/// not an offer have nothing in common. A channel nobody has published, a
/// machine that was told never to touch the network, and a connection that
/// broke are three different facts with three different remedies, and the
/// update check used to report the first two as the third: opening Settings on
/// a registry with no `tahdith.json` produced `TAARIB-E-8100`, "the update
/// channel could not be reached", over a request that had been answered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(tag = "hala", rename_all = "snake_case")]
pub enum HalatTahdith {
    /// No source is configured to ask — offline mode, with no local copy of the
    /// registry to read instead.
    GhayrMuttasil,
    /// Every configured source answered, and none of them carries a channel
    /// manifest.
    GhayrManshura {
        /// The sources that were asked.
        masdar: String,
    },
    /// The channel carries nothing newer for this build.
    Ahdath,
    /// A newer version is offered.
    Mutah {
        /// The version being offered.
        isdar: String,
        /// Its size in bytes.
        #[specta(type = specta_typescript::Number)]
        hajm: u64,
        /// Which channel it came from.
        qanat: String,
        /// Whether this installation can replace itself, or a package manager
        /// owns it and the user updates from there.
        qabil_lil_tabdil: bool,
    },
}

/// The target triple this build runs on, as the channel manifest names it.
const fn hadaf_hali() -> &'static str {
    // A single source rather than `std::env::consts` assembled at runtime: the
    // manifest's keys are Rust target triples, and this build knows its own.
    env!("TAARIB_HADAF")
}

/// A channel manifest as the configured sources answered for it.
enum QanatMaqrua {
    /// Nothing was asked, because nothing is configured to ask.
    GhayrMuttasila,
    /// Everything was asked and no source carries the manifest.
    Ghaiba {
        /// The sources that were asked.
        masdar: String,
    },
    /// The verified manifest.
    Mawjuda(BayanTahdith),
}

/// One document as one source answered for it.
enum WathiqaMaqrua {
    /// The bytes.
    Mawjuda(Vec<u8>),
    /// The source answered, and does not have it.
    Ghaiba,
}

/// Reads the channel manifest and its detached signature from the same source.
///
/// The registry's own source chain, not a second address of the updater's own:
/// the manifest sits under the registry root, so the local copies a user
/// configured are asked first, the mirror stands in for the forge, and offline
/// mode is honoured rather than worked around. Both documents come from *one*
/// source, so a stale mirror's signature is never checked against a fresh
/// forge's manifest.
///
/// # Errors
///
/// [`taarib_tahdith::KhataTahdith::QanatGhayrMutaha`] when a source failed for
/// any reason but the document's absence,
/// [`taarib_tahdith::KhataTahdith::BayanTalif`] when a manifest is published
/// with no signature beside it, and whatever the signature check refuses.
async fn iqra_qanat(idadat: &Idadat) -> Natija<QanatMaqrua> {
    // The chain's only refusal is that nothing is configured to reach, which is
    // a setting the user chose and not a failure to report as one.
    let Ok(silsila) = crate::tathbeet_awamir::silsilat_masadir(idadat) else {
        return Ok(QanatMaqrua::GhayrMuttasila);
    };
    let masdar = silsila
        .masadir()
        .iter()
        .map(MasdarMustawda::wasf)
        .collect::<Vec<_>>()
        .join(", ");

    let amil = reqwest::Client::builder()
        .user_agent(format!(
            "Taarib/{ISDAR} (+https://github.com/cc1a2b/taarib)"
        ))
        .timeout(MUHLAT_TALAB)
        .connect_timeout(MUHLAT_ITTISAL)
        .build()
        .map_err(|khata| khata_qanat(&khata.to_string()))?;

    let mut akhir: Option<String> = None;
    for wahid in silsila.masadir() {
        let matn = match jalb_wathiqa(wahid, &amil, ISM_BAYAN).await {
            Ok(WathiqaMaqrua::Mawjuda(bayt)) => bayt,
            Ok(WathiqaMaqrua::Ghaiba) => continue,
            Err(sabab) => {
                akhir = Some(sabab);
                continue;
            },
        };
        let tawqee = match jalb_wathiqa(wahid, &amil, ISM_TAWQEE).await {
            Ok(WathiqaMaqrua::Mawjuda(bayt)) => bayt,
            Ok(WathiqaMaqrua::Ghaiba) => {
                // A manifest with no signature beside it is a broken
                // publication, never something to read anyway; the next source
                // may still carry a whole pair.
                akhir = Some(format!(
                    "{}: {ISM_BAYAN} is published with no {ISM_TAWQEE} beside it",
                    wahid.wasf()
                ));
                continue;
            },
            Err(sabab) => {
                akhir = Some(sabab);
                continue;
            },
        };

        let tawqee = String::from_utf8(tawqee).map_err(|_| {
            Khata::min_tafsir(&taarib_tahdith::KhataTahdith::BayanTalif {
                sabab: "the detached signature is not text".to_owned(),
            })
        })?;
        // The same anchor that verifies a patch verifies the channel: a release
        // client cannot be updated by anything the owner did not sign.
        let bayan = ihlil(&matn, tawqee.trim(), &taarib_khatm::MIRSAT_MALIK)
            .map_err(|khata| Khata::min_tafsir(&khata))?;
        return Ok(QanatMaqrua::Mawjuda(bayan));
    }

    match akhir {
        // Every source answered and none of them has it: the owner has not
        // published a channel, which is a state and not a transport failure.
        None => Ok(QanatMaqrua::Ghaiba { masdar }),
        Some(sabab) => Err(khata_qanat(&sabab)),
    }
}

/// Fetches one repository path from one source, telling an absent document
/// apart from a source that could not be reached.
async fn jalb_wathiqa(
    masdar: &MasdarMustawda,
    amil: &reqwest::Client,
    nisbi: &str,
) -> Result<WathiqaMaqrua, String> {
    match masdar.jalb(amil, nisbi).await {
        Ok(bayt) => Ok(WathiqaMaqrua::Mawjuda(bayt)),
        Err(khata) if ghaiba(&khata) => Ok(WathiqaMaqrua::Ghaiba),
        Err(khata) => Err(format!("{}: {khata}", masdar.wasf())),
    }
}

/// Whether a source's refusal means the document is not there, as opposed to
/// the source not being reachable.
///
/// The distinction is the whole point: `404` and a missing file are answers,
/// and an answer that says "no such document" is not a network failure however
/// it is transported.
fn ghaiba(khata: &KhataMustawda) -> bool {
    match khata {
        // 410 as well as 404: a forge that has served this path before and
        // stopped is still telling us the document is gone, not that it could
        // not be asked.
        KhataMustawda::IstijabaFashila { ramz, .. } => *ramz == 404 || *ramz == 410,
        KhataMustawda::KhataMalaf { sabab, .. } => {
            matches!(sabab.kind(), std::io::ErrorKind::NotFound)
        },
        _ => false,
    }
}

/// One transport failure, as the update crate names it.
fn khata_qanat(sabab: &str) -> Khata {
    Khata::min_tafsir(&taarib_tahdith::KhataTahdith::QanatGhayrMutaha {
        sabab: sabab.to_owned(),
    })
}

/// What the channel offers this build, if anything.
///
/// Answers [`HalatTahdith::Ahdath`] when this build is current and
/// [`HalatTahdith::GhayrManshura`] when no source carries a channel manifest at
/// all — both ordinary states, neither a failure.
///
/// # Errors
///
/// Whatever the transport, the signature, or the version comparison refuses —
/// including a channel signed by the development key under a release build,
/// which is refused by name.
#[tauri::command]
#[specta::specta]
pub async fn tahaqquq_tahdith(
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<HalatTahdith, Khata> {
    hala_tahdith(&Arc::unwrap_or_clone(makhzan.hali())).await
}

/// The channel's answer for this build, with no window or command around it, so
/// the Settings query and the startup check ask exactly the same question.
async fn hala_tahdith(idadat: &Idadat) -> Natija<HalatTahdith> {
    let bayan = match iqra_qanat(idadat).await? {
        QanatMaqrua::GhayrMuttasila => return Ok(HalatTahdith::GhayrMuttasil),
        QanatMaqrua::Ghaiba { masdar } => return Ok(HalatTahdith::GhayrManshura { masdar }),
        QanatMaqrua::Mawjuda(bayan) => bayan,
    };
    let qanat = qanat_nassiya(idadat);

    let Some(madkhal) =
        intiqa(&bayan, hadaf_hali(), &qanat, ISDAR).map_err(|khata| Khata::min_tafsir(&khata))?
    else {
        return Ok(HalatTahdith::Ahdath);
    };

    let tareeqa = tabdil::istadill(&masar_tanfidhi()?);
    Ok(HalatTahdith::Mutah {
        isdar: madkhal.isdar.clone(),
        hajm: madkhal.hajm,
        qanat: madkhal.qanat.clone(),
        qabil_lil_tabdil: tareeqa != tabdil::TareeqatTabdil::Mudar,
    })
}

/// The event a startup check raises when the channel offers a newer version.
///
/// The Settings screen asks on open through [`tahaqquq_tahdith`]; this is how a
/// user who never opens Settings still learns an update exists, and it is the
/// one thing `idadat.tahdith.fahs_ind_bad` turns on.
pub const ISM_HADATH_TAHDITH_MUTAH: &str = "taarib://tahdith-mutah";

/// Runs the update check once at launch when the user has asked for it, and
/// raises [`ISM_HADATH_TAHDITH_MUTAH`] if a newer version is offered.
///
/// A launch-time convenience, never a reason a launch fails: every outcome but
/// an actual offer — the setting being off, offline mode, an unpublished
/// channel, an unreachable one — is logged and swallowed. The check reads the
/// settings itself so a copy captured before the store was populated cannot make
/// it ask when it was turned off.
pub async fn fahs_bad_iqla(tatbiq: &tauri::AppHandle, makhzan: &Arc<MakhzanIdadat>) {
    use tauri::Emitter as _;

    let idadat = Arc::unwrap_or_clone(makhzan.hali());
    if !idadat.tahdith.fahs_ind_bad {
        return;
    }
    match hala_tahdith(&idadat).await {
        Ok(HalatTahdith::Mutah {
            isdar,
            hajm,
            qanat,
            qabil_lil_tabdil,
        }) => {
            let hadath = HalatTahdith::Mutah {
                isdar,
                hajm,
                qanat,
                qabil_lil_tabdil,
            };
            if let Err(sabab) = tatbiq.emit(ISM_HADATH_TAHDITH_MUTAH, hadath) {
                tracing::warn!(sabab = %sabab, "the update notice could not be delivered");
            }
        },
        // No offer, and nothing to interrupt the user with: the channel is
        // current, unpublished, offline, or was checked from Settings anyway.
        Ok(_) => {},
        Err(khata) => {
            tracing::warn!(khata = %khata.li_sijill(), "the startup update check did not finish");
        },
    }
}

/// Downloads the offered update, verifies it, and stages the swap.
///
/// The swap itself happens on the next launch for the formats that replace a
/// running file, so this answers the path the user can see rather than
/// restarting anything behind their back.
///
/// # Errors
///
/// [`taarib_tahdith::KhataTahdith::QanatGhayrManshura`] when no source carries
/// a channel manifest, and whatever the channel, the download, the hash check,
/// the free-space check or the swap refuses. A package-manager installation is
/// refused by name.
#[tauri::command]
#[specta::specta]
pub async fn nazzil_tahdith(
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
    masarat: tauri::State<'_, Masarat>,
) -> Result<String, Khata> {
    let idadat = Arc::unwrap_or_clone(makhzan.hali());
    let bayan = match iqra_qanat(&idadat).await? {
        QanatMaqrua::GhayrMuttasila => {
            return Err(Khata::min_tafsir(
                &taarib_tahdith::KhataTahdith::QanatGhayrManshura {
                    masdar: "nothing: offline mode is on and no local registry copy is configured"
                        .to_owned(),
                },
            ));
        },
        QanatMaqrua::Ghaiba { masdar } => {
            return Err(Khata::min_tafsir(
                &taarib_tahdith::KhataTahdith::QanatGhayrManshura { masdar },
            ));
        },
        QanatMaqrua::Mawjuda(bayan) => bayan,
    };
    let qanat = qanat_nassiya(&idadat);

    let madkhal: &MadkhalTahdith = intiqa(&bayan, hadaf_hali(), &qanat, ISDAR)
        .map_err(|khata| Khata::min_tafsir(&khata))?
        .ok_or_else(|| {
            Khata::min_tafsir(&taarib_tahdith::KhataTahdith::LaMadkhal {
                hadaf: hadaf_hali().to_owned(),
                qanat: qanat.clone(),
            })
        })?;

    let sandooq = masarat.sandooq();
    taarib_usus::masarat::insha_mujallad(&sandooq)?;
    let malaf = jalb::ijlib(madkhal, &sandooq)
        .await
        .map_err(|khata| Khata::min_tafsir(&khata))?;

    let tanfidhi = masar_tanfidhi()?;
    let tareeqa = tabdil::istadill(&tanfidhi);
    let khutta =
        tabdil::khattit(tareeqa, &tanfidhi, &malaf).map_err(|khata| Khata::min_tafsir(&khata))?;

    if khutta.tareeqa == tabdil::TareeqatTabdil::Nsis {
        // The installer replaces a running executable, so it is run at exit
        // rather than from under the process it is replacing.
        return Ok(khutta.masdar.to_string_lossy().into_owned());
    }
    tabdil::naffidh(&khutta).map_err(|khata| Khata::min_tafsir(&khata))?;
    Ok(khutta.hadaf.to_string_lossy().into_owned())
}

/// The channel the settings name, as the manifest spells it.
fn qanat_nassiya(idadat: &Idadat) -> String {
    match idadat.tahdith.qanat {
        taarib_usus::idadat::QanatTahdith::Mustaqirr => QANAT_MUSTAQIRR.to_owned(),
        taarib_usus::idadat::QanatTahdith::Tajribi => QANAT_TAJRIBI.to_owned(),
    }
}

/// This process's own executable.
fn masar_tanfidhi() -> Natija<PathBuf> {
    std::env::current_exe().map_err(|sabab| {
        Khata::min_tafsir(&taarib_tahdith::KhataTahdith::KhataMalaf {
            masar: PathBuf::from("."),
            amal: "locating the running executable",
            sabab,
        })
    })
}

#[cfg(test)]
mod ikhtibarat {
    use std::path::{Path, PathBuf};

    use taarib_khatm::{HawiyatThiqa, MiftahKhass, MirsatThiqa};
    use taarib_mustawda::{KhataMustawda, MasdarMustawda};
    use taarib_tahdith::bayan::{KatibBayan, MadkhalTahdith, QANAT_MUSTAQIRR};

    use super::{ISM_BAYAN, ISM_TAWQEE, WathiqaMaqrua, ghaiba, jalb_wathiqa};

    /// What every test here answers with, so a fixture failure propagates with
    /// `?`. `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn amil() -> NatijatIkhtibar<reqwest::Client> {
        Ok(reqwest::Client::builder().build()?)
    }

    fn mahalli(jidhr: &Path) -> MasdarMustawda {
        MasdarMustawda::MujalladMahalli {
            jidhr: jidhr.to_path_buf(),
        }
    }

    /// A registry directory with no channel manifest in it — which is every
    /// registry in existence until the owner casts one.
    #[tokio::test]
    async fn qanat_ghayr_manshura_tuqra_ghaiba_la_maqtua() -> NatijatIkhtibar {
        let mujallad = tempfile::tempdir()?;
        let natija = jalb_wathiqa(&mahalli(mujallad.path()), &amil()?, ISM_BAYAN).await;
        match natija {
            Ok(WathiqaMaqrua::Ghaiba) => Ok(()),
            Ok(WathiqaMaqrua::Mawjuda(_)) => {
                Err("an empty directory answered with a manifest".into())
            },
            Err(sabab) => {
                Err(format!("absence was reported as a transport failure: {sabab}").into())
            },
        }
    }

    /// And a registry that does carry one hands it back, so the absence above
    /// is the document's and not the source's.
    #[tokio::test]
    async fn qanat_manshura_tuqra_kamila() -> NatijatIkhtibar {
        let mujallad = tempfile::tempdir()?;
        let khass = MiftahKhass::min_bayt(&[3u8; 32]);
        let (matn, tawqee) = KatibBayan::jadeed()
            .adif(MadkhalTahdith {
                hadaf: "x86_64-pc-windows-msvc".to_owned(),
                qanat: QANAT_MUSTAQIRR.to_owned(),
                isdar: "9.9.9".to_owned(),
                rabt: "https://example.invalid/taarib.exe".to_owned(),
                hajm: 4096,
                sha256: "0123456789abcdef".repeat(4),
                adna_isdar: "0.1.0".to_owned(),
            })
            .uktub(&khass)?;
        std::fs::write(mujallad.path().join(ISM_BAYAN), &matn)?;
        std::fs::write(mujallad.path().join(ISM_TAWQEE), &tawqee)?;

        let masdar = mahalli(mujallad.path());
        let amil = amil()?;
        let wajad = matches!(
            jalb_wathiqa(&masdar, &amil, ISM_BAYAN).await,
            Ok(WathiqaMaqrua::Mawjuda(_))
        );
        assert!(wajad, "a cast manifest was not read back");
        assert!(matches!(
            jalb_wathiqa(&masdar, &amil, ISM_TAWQEE).await,
            Ok(WathiqaMaqrua::Mawjuda(_))
        ));

        let mirsa = MirsatThiqa {
            miftah: khass.aam().bayt(),
            hawiya: HawiyatThiqa::Isdar,
        };
        let bayan = taarib_tahdith::ihlil(&matn, tawqee.trim(), &mirsa)?;
        assert_eq!(bayan.madakhil.len(), 1);
        Ok(())
    }

    /// The classifier itself: only an answer that names the document as absent
    /// counts as absence. Everything else is a source that could not be
    /// reached, and is reported as one.
    #[test]
    fn al_ghiyab_yatamayyaz_an_taadhur_alwusul() {
        assert!(ghaiba(&KhataMustawda::IstijabaFashila {
            rabt: "https://example.invalid/tahdith.json".to_owned(),
            ramz: 404,
        }));
        assert!(ghaiba(&KhataMustawda::IstijabaFashila {
            rabt: "https://example.invalid/tahdith.json".to_owned(),
            ramz: 410,
        }));
        assert!(ghaiba(&KhataMustawda::KhataMalaf {
            masar: PathBuf::from("/tmp/mustawda"),
            amal: "reading a local registry source",
            sabab: std::io::Error::from(std::io::ErrorKind::NotFound),
        }));

        for ramz in [403_u16, 500, 502, 503] {
            assert!(
                !ghaiba(&KhataMustawda::IstijabaFashila {
                    rabt: "https://example.invalid/tahdith.json".to_owned(),
                    ramz,
                }),
                "{ramz} is not the document telling us it is absent"
            );
        }
        assert!(!ghaiba(&KhataMustawda::LaMasdar {
            sabab: "the name did not resolve".to_owned(),
        }));
        assert!(!ghaiba(&KhataMustawda::KhataMalaf {
            masar: PathBuf::from("/tmp/mustawda"),
            amal: "reading a local registry source",
            sabab: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }));
    }
}
