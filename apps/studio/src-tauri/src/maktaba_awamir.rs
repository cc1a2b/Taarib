//! المكتبة — what the scan learned about each launcher, kept and shown rather than dropped.

use std::collections::BTreeMap;

use taarib_kashf::fahs::{HalatFahsMatjar, Matjar, NatijatFahs, NatijatMatjar, TanbihFahs};
use taarib_makhzan::sijillat::{MatjarMukhzan, SijillFahs, TanbihMukhzan};
use taarib_makhzan::wasl::Makhzan;
use taarib_usus::khata::Khata;

/// One launcher's verdict, as the interface receives it.
///
/// The same three answers as [`HalatFahsMatjar`], spelled for the wire. Sent as
/// its own discriminant rather than folded into a sentence so the interface can
/// group launchers by it without matching on display text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum HalatMatjarHie {
    /// Not on this machine.
    GhayrMuthabbat,
    /// Installed, and its catalogue was read end to end.
    Tamma,
    /// Installed, and some part of its catalogue could not be read. Its stored
    /// games were not marked absent on this scan's word.
    Naqisa,
}

impl HalatMatjarHie {
    /// The wire form of a scan verdict.
    const fn min_hala(hala: HalatFahsMatjar) -> Self {
        match hala {
            HalatFahsMatjar::GhayrMuthabbat => Self::GhayrMuthabbat,
            HalatFahsMatjar::Tamma => Self::Tamma,
            HalatFahsMatjar::Naqisa => Self::Naqisa,
        }
    }

    /// The verdict a stored launcher row implies.
    const fn min_mukhzan(matjar: &MatjarMukhzan) -> Self {
        if !matjar.mawjud {
            Self::GhayrMuthabbat
        } else if matjar.najah {
            Self::Tamma
        } else {
            Self::Naqisa
        }
    }
}

/// One warning a launcher's scan produced, as the interface receives it.
///
/// The sentence is the adapter's own, in English, naming the file and the
/// reason: it is a technical detail beside a bilingual summary, the same way a
/// log line sits beside an error's two sentences.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TanbihFahsHie {
    /// The file or entry, as specifically as the adapter could name it.
    pub mawdi: String,
    /// What was wrong.
    pub sabab: String,
    /// Whether this gap means games may exist that the scan did not see.
    ///
    /// Known for a scan that just ran; `None` for one read back from the
    /// store, which keeps the launcher's verdict but not each warning's kind.
    pub yukhfi_alaab: Option<bool>,
}

/// What one launcher's scan came to, with a sentence about it in each language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MatjarMaktabaHie {
    /// The launcher's identifier: `steam`, `epic`, `xbox`, …
    pub muarrif: String,
    /// Its name as the interface writes it in Arabic.
    pub ism_arabi: String,
    /// Its name in English.
    pub ism_injilizi: String,
    /// The verdict.
    pub hala: HalatMatjarHie,
    /// Where the launcher was found, when a root was; `None` for a launcher
    /// that keeps its catalogue in the registry, and for a row read back from
    /// the store, which does not keep it.
    pub jidhr: Option<String>,
    /// How many games it produced.
    pub adad_alaab: u32,
    /// Every warning, in the order the adapter raised them.
    pub tanbihat: Vec<TanbihFahsHie>,
    /// How long its scan took.
    #[specta(type = specta_typescript::Number)]
    pub muddat_ms: u64,
    /// The one-line summary, in Arabic.
    pub wasf_arabi: String,
    /// The same summary in English.
    pub wasf_injilizi: String,
}

/// The most recent scan, as the diagnostics screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FahsAkhirHie {
    /// The scan's generation number.
    #[specta(type = specta_typescript::Number)]
    pub raqm: u64,
    /// When it started, RFC 3339.
    pub bidaya: String,
    /// When it finished, or `None` if it never did — a crash mid-scan.
    pub nihaya: Option<String>,
    /// Whether it covered every launcher or refreshed one.
    pub kamil: bool,
    /// How many games it saw.
    pub adad_alaab: u32,
    /// Every launcher it recorded, in adapter order, with its warnings.
    pub matajir: Vec<MatjarMaktabaHie>,
}

/// A launcher's two names, keyed by identifier, from the adapters themselves —
/// the only place the names live.
fn asmaa(matajir: &[Box<dyn Matjar>]) -> BTreeMap<&'static str, (&'static str, &'static str)> {
    matajir
        .iter()
        .map(|matjar| {
            (
                matjar.muarrif(),
                (matjar.ism_arabi(), matjar.ism_injilizi()),
            )
        })
        .collect()
}

/// The one-line summary, built from the same facts in both languages so the
/// two cannot drift.
fn wasf(
    ism_arabi: &str,
    ism_injilizi: &str,
    hala: HalatMatjarHie,
    adad_alaab: u32,
    adad_tanbihat: u32,
) -> (String, String) {
    match hala {
        HalatMatjarHie::GhayrMuthabbat => (
            format!("{ism_arabi}: غير مثبَّت على هذا الجهاز."),
            format!("{ism_injilizi}: not installed on this machine."),
        ),
        HalatMatjarHie::Tamma if adad_tanbihat == 0 => (
            format!("{ism_arabi}: قُرئ فهرسه كاملًا، ووُجد فيه {adad_alaab} من الألعاب."),
            format!("{ism_injilizi}: catalogue read end to end; {adad_alaab} game(s) found."),
        ),
        HalatMatjarHie::Tamma => (
            format!(
                "{ism_arabi}: قُرئ فهرسه كاملًا، ووُجد فيه {adad_alaab} من الألعاب، وتعذّرت قراءة \
                 {adad_tanbihat} من مدخلاته."
            ),
            format!(
                "{ism_injilizi}: catalogue read end to end; {adad_alaab} game(s) found, and \
                 {adad_tanbihat} entry/entries could not be read."
            ),
        ),
        HalatMatjarHie::Naqisa => (
            format!(
                "{ism_arabi}: مثبَّت، لكن تعذّرت قراءة فهرسه كاملًا ({adad_tanbihat} من \
                 التنبيهات)؛ وُجد فيه {adad_alaab} من الألعاب، ولم تُعلَّم ألعابه المخزّنة غائبة."
            ),
            format!(
                "{ism_injilizi}: installed, but its catalogue could not be read end to end \
                 ({adad_tanbihat} warning(s)); {adad_alaab} game(s) found, and its stored games \
                 were not marked absent."
            ),
        ),
    }
}

/// Assembles one launcher's row from its facts.
fn satr(
    asmaa: &BTreeMap<&'static str, (&'static str, &'static str)>,
    muarrif: &str,
    hala: HalatMatjarHie,
    jidhr: Option<String>,
    adad_alaab: u32,
    tanbihat: Vec<TanbihFahsHie>,
    muddat_ms: u64,
) -> MatjarMaktabaHie {
    // An identifier this build has no adapter for — a row an older or newer
    // build wrote — keeps its identifier as its name rather than vanishing.
    let (ism_arabi, ism_injilizi) = asmaa.get(muarrif).copied().unwrap_or((muarrif, muarrif));
    let adad_tanbihat = u32::try_from(tanbihat.len()).unwrap_or(u32::MAX);
    let (wasf_arabi, wasf_injilizi) =
        wasf(ism_arabi, ism_injilizi, hala, adad_alaab, adad_tanbihat);
    MatjarMaktabaHie {
        muarrif: muarrif.to_owned(),
        ism_arabi: ism_arabi.to_owned(),
        ism_injilizi: ism_injilizi.to_owned(),
        hala,
        jidhr,
        adad_alaab,
        tanbihat,
        muddat_ms,
        wasf_arabi,
        wasf_injilizi,
    }
}

/// Every launcher's row for a scan that just ran, in the order the adapters
/// ran, with the two names each adapter carries.
#[must_use]
pub fn matajir_hie(matajir: &[Box<dyn Matjar>], natija: &NatijatFahs) -> Vec<MatjarMaktabaHie> {
    let asmaa = asmaa(matajir);
    natija
        .matajir
        .iter()
        .map(|wahid| {
            satr(
                &asmaa,
                wahid.matjar,
                HalatMatjarHie::min_hala(wahid.hala()),
                wahid
                    .jidhr_matjar
                    .as_ref()
                    .map(|jidhr| jidhr.to_string_lossy().into_owned()),
                u32::try_from(wahid.alaab.len()).unwrap_or(u32::MAX),
                wahid
                    .tanbihat
                    .iter()
                    .map(|tanbih| TanbihFahsHie {
                        mawdi: tanbih.mawdi.clone(),
                        sabab: tanbih.sabab.clone(),
                        yukhfi_alaab: Some(tanbih.yukhfi_alaab()),
                    })
                    .collect(),
                u64::try_from(wahid.muddat.as_millis()).unwrap_or(u64::MAX),
            )
        })
        .collect()
}

/// Every launcher's row for a scan read back from the store.
///
/// Adapter order first, so the screen reads the same whether the scan just ran
/// or is being looked up; a family this build has no adapter for follows, under
/// its identifier.
fn matajir_hie_min_makhzan(
    matajir: &[Box<dyn Matjar>],
    sufuf: &[MatjarMukhzan],
    tanbihat: &[TanbihMukhzan],
) -> Vec<MatjarMaktabaHie> {
    let asmaa = asmaa(matajir);
    let mut bil_aila: BTreeMap<&str, Vec<TanbihFahsHie>> = BTreeMap::new();
    for tanbih in tanbihat {
        bil_aila
            .entry(tanbih.aila.as_str())
            .or_default()
            .push(TanbihFahsHie {
                mawdi: tanbih.mawdi.clone(),
                sabab: tanbih.sabab.clone(),
                yukhfi_alaab: None,
            });
    }

    let mut murattaba: Vec<&MatjarMukhzan> = Vec::with_capacity(sufuf.len());
    for matjar in matajir {
        murattaba.extend(sufuf.iter().filter(|saf| saf.aila == matjar.muarrif()));
    }
    murattaba.extend(
        sufuf
            .iter()
            .filter(|saf| !asmaa.contains_key(saf.aila.as_str())),
    );

    murattaba
        .into_iter()
        .map(|saf| {
            satr(
                &asmaa,
                &saf.aila,
                HalatMatjarHie::min_mukhzan(saf),
                None,
                saf.adad_alaab,
                bil_aila.remove(saf.aila.as_str()).unwrap_or_default(),
                saf.muddat_milli,
            )
        })
        .collect()
}

/// One launcher's result as the store records it: the row, and every warning.
///
/// `najah` is the verdict, and only [`HalatFahsMatjar::Tamma`] earns it — the
/// store's absence sweep reads this column and refuses on anything else.
#[must_use]
pub fn mukhzan_min_natija(natija: &NatijatMatjar) -> (MatjarMukhzan, Vec<TanbihMukhzan>) {
    let hala = natija.hala();
    let matjar = MatjarMukhzan {
        aila: natija.matjar.to_owned(),
        mawjud: hala != HalatFahsMatjar::GhayrMuthabbat,
        najah: hala == HalatFahsMatjar::Tamma,
        adad_alaab: u32::try_from(natija.alaab.len()).unwrap_or(u32::MAX),
        adad_tanbihat: u32::try_from(natija.tanbihat.len()).unwrap_or(u32::MAX),
        muddat_milli: u64::try_from(natija.muddat.as_millis()).unwrap_or(u64::MAX),
    };
    let tanbihat = natija
        .tanbihat
        .iter()
        .map(|tanbih| TanbihMukhzan {
            aila: tanbih.matjar.to_owned(),
            mawdi: tanbih.mawdi.clone(),
            sabab: tanbih.sabab.clone(),
        })
        .collect();
    (matjar, tanbihat)
}

/// Every launcher's result as the store records it, in scan order.
#[must_use]
pub fn mukhzan_min_fahs(natija: &NatijatFahs) -> Vec<(MatjarMukhzan, Vec<TanbihMukhzan>)> {
    natija.matajir.iter().map(mukhzan_min_natija).collect()
}

/// Writes every launcher's verdict and every warning to the log.
///
/// The rolling log is what the diagnostics screen tails, so this is the one
/// surface on which a warning is readable the moment the scan ends. A
/// catalogue-level gap is logged at `warn`; one entry that degraded at `info`,
/// because a library of six hundred games has a few of those on every scan and
/// they must not bury the one line that says a drive was unplugged.
pub fn dawwin(natija: &NatijatFahs) {
    for wahid in &natija.matajir {
        tracing::info!(
            matjar = wahid.matjar,
            hala = ?wahid.hala(),
            alaab = wahid.alaab.len(),
            tanbihat = wahid.tanbihat.len(),
            muddat_ms = u64::try_from(wahid.muddat.as_millis()).unwrap_or(u64::MAX),
            "launcher scanned"
        );
        for tanbih in &wahid.tanbihat {
            dawwin_tanbih(tanbih);
        }
    }
}

/// One warning, at the level its kind earns.
fn dawwin_tanbih(tanbih: &TanbihFahs) {
    if tanbih.yukhfi_alaab() {
        tracing::warn!(
            matjar = tanbih.matjar,
            mawdi = %tanbih.mawdi,
            sabab = %tanbih.sabab,
            "a launcher's catalogue could not be read end to end"
        );
    } else {
        tracing::info!(
            matjar = tanbih.matjar,
            mawdi = %tanbih.mawdi,
            sabab = %tanbih.sabab,
            "a catalogue entry degraded"
        );
    }
}

/// The most recent scan, with what every launcher contributed to it and every
/// warning it left, or `None` before the first scan.
///
/// Read from the store rather than remembered from the last `maktaba` call, so
/// the diagnostics screen answers the same after a restart, and so a bundle
/// built for a maintainer can carry the launcher record of a scan they did not
/// watch.
///
/// # Errors
///
/// Whatever the store raises reading the scan ledger.
#[tauri::command]
#[specta::specta]
pub fn fahs_akhir(qaida: tauri::State<'_, Makhzan>) -> Result<Option<FahsAkhirHie>, Khata> {
    let kashif = taarib_kashf::Kashif::jadeed();
    let hasila = qaida.bil_qira(|ittisal| {
        let sijill = SijillFahs::jadeed(ittisal);
        let Some(fahs) = sijill.akhir()? else {
            return Ok(None);
        };
        let matajir = sijill.matajir(fahs.raqm)?;
        let tanbihat = sijill.tanbihat(fahs.raqm)?;
        Ok(Some((fahs, matajir, tanbihat)))
    })?;
    Ok(hasila.map(|(fahs, matajir, tanbihat)| FahsAkhirHie {
        raqm: fahs.raqm,
        bidaya: fahs.bidaya,
        nihaya: fahs.nihaya,
        kamil: fahs.kamil,
        adad_alaab: fahs.adad_alaab,
        matajir: matajir_hie_min_makhzan(kashif.matajir(), &matajir, &tanbihat),
    }))
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::path::PathBuf;
    use std::time::Duration;

    use taarib_kashf::fahs::TanbihFahs;

    use super::*;

    /// Every test returns this so that a missing row reads as a sentence
    /// rather than a panic, which the workspace denies in tests too.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// The three verdicts as the store must receive them: only a complete read
    /// earns `najah`, and every warning crosses verbatim — the sentence the
    /// Xbox adapter writes about `WindowsApps` is the sentence the store keeps.
    #[test]
    fn al_mukhzan_yahmil_al_hukm_wa_kull_tanbih() {
        let (ghayr, tanbihat) = mukhzan_min_natija(&NatijatMatjar::ghayr_mutah("gog"));
        assert!(!ghayr.mawjud);
        assert!(!ghayr.najah);
        assert!(tanbihat.is_empty());

        let mut tamma = NatijatMatjar::muthabbat("xbox", Some(PathBuf::from("/WindowsApps")));
        tamma.tanbihat.push(TanbihFahs::jadeed(
            "xbox",
            "/WindowsApps",
            "Windows denies listing",
        ));
        let (saf, tanbihat) = mukhzan_min_natija(&tamma);
        assert!(saf.mawjud);
        assert!(saf.najah, "one named entry does not unmake a complete read");
        assert_eq!(saf.adad_tanbihat, 1);
        assert_eq!(
            tanbihat,
            vec![TanbihMukhzan {
                aila: "xbox".to_owned(),
                mawdi: "/WindowsApps".to_owned(),
                sabab: "Windows denies listing".to_owned(),
            }]
        );

        let naqisa = NatijatMatjar::naqisa(
            "epic",
            Some(PathBuf::from("/Epic")),
            "/Epic",
            "no Manifests",
        );
        let (saf, tanbihat) = mukhzan_min_natija(&naqisa);
        assert!(saf.mawjud);
        assert!(!saf.najah, "an unread catalogue must not authorise a sweep");
        assert_eq!(tanbihat.len(), 1);
    }

    /// The interface gets the launcher's two names and a sentence in each
    /// language built from the same facts, and every warning beside them.
    #[test]
    fn al_wajiha_taqra_al_ismayn_wa_al_wasfayn_wa_al_tanbihat() -> NatijatIkhtibar {
        let kashif = taarib_kashf::Kashif::jadeed();
        let mut naqisa = NatijatMatjar::naqisa(
            "epic",
            Some(PathBuf::from("/Epic")),
            "/Epic",
            "no Manifests",
        );
        naqisa.muddat = Duration::from_millis(7);
        let natija = NatijatFahs {
            matajir: vec![
                NatijatMatjar::muthabbat("steam", Some(PathBuf::from("/Steam"))),
                naqisa,
                NatijatMatjar::ghayr_mutah("gog"),
            ],
            muddat: Duration::ZERO,
        };

        let sufuf = matajir_hie(kashif.matajir(), &natija);
        assert_eq!(sufuf.len(), 3);
        let epic = sufuf
            .iter()
            .find(|saf| saf.muarrif == "epic")
            .ok_or("the Epic row is missing")?;
        assert_eq!(epic.hala, HalatMatjarHie::Naqisa);
        assert_eq!(epic.ism_injilizi, "Epic Games");
        assert_eq!(epic.ism_arabi, "إيبك");
        assert_eq!(epic.jidhr.as_deref(), Some("/Epic"));
        assert_eq!(epic.muddat_ms, 7);
        assert!(epic.wasf_injilizi.starts_with("Epic Games: installed, but"));
        assert!(epic.wasf_arabi.starts_with("إيبك: مثبَّت، لكن"));
        assert_eq!(epic.tanbihat.len(), 1);
        assert_eq!(
            epic.tanbihat.first().map(|tanbih| tanbih.sabab.as_str()),
            Some("no Manifests")
        );
        assert_eq!(
            epic.tanbihat.first().and_then(|tanbih| tanbih.yukhfi_alaab),
            Some(true)
        );

        let gog = sufuf
            .iter()
            .find(|saf| saf.muarrif == "gog")
            .ok_or("the GOG row is missing")?;
        assert_eq!(gog.hala, HalatMatjarHie::GhayrMuthabbat);
        assert!(
            gog.wasf_injilizi
                .ends_with("not installed on this machine.")
        );
        Ok(())
    }

    /// A scan read back from the store reaches the same shape, in adapter
    /// order, with each launcher's warnings attributed to it and the kind of
    /// each warning honestly unknown.
    #[test]
    fn al_qiraa_min_al_makhzan_tuayid_al_tarteeb_wa_al_tanbihat() -> NatijatIkhtibar {
        let kashif = taarib_kashf::Kashif::jadeed();
        let sufuf = vec![
            MatjarMukhzan {
                aila: "xbox".to_owned(),
                mawjud: true,
                najah: false,
                adad_alaab: 0,
                adad_tanbihat: 1,
                muddat_milli: 3,
            },
            MatjarMukhzan {
                aila: "steam".to_owned(),
                mawjud: true,
                najah: true,
                adad_alaab: 18,
                adad_tanbihat: 0,
                muddat_milli: 40,
            },
        ];
        let tanbihat = vec![TanbihMukhzan {
            aila: "xbox".to_owned(),
            mawdi: "C:\\Program Files\\WindowsApps".to_owned(),
            sabab: "cannot list this package folder".to_owned(),
        }];

        let hie = matajir_hie_min_makhzan(kashif.matajir(), &sufuf, &tanbihat);
        let asmaa: Vec<&str> = hie.iter().map(|saf| saf.muarrif.as_str()).collect();
        assert_eq!(asmaa, ["steam", "xbox"], "adapter order, not the store's");

        let xbox = hie
            .iter()
            .find(|saf| saf.muarrif == "xbox")
            .ok_or("the Xbox row is missing")?;
        assert_eq!(xbox.hala, HalatMatjarHie::Naqisa);
        assert_eq!(xbox.tanbihat.len(), 1);
        assert_eq!(
            xbox.tanbihat.first().and_then(|tanbih| tanbih.yukhfi_alaab),
            None
        );
        assert!(xbox.jidhr.is_none());
        Ok(())
    }
}
