//! الإعدادات — the settings command surface: the tree, the credentials, and the fonts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use taarib_saff::khatt::MawridKhatt;
use taarib_tarjama::muzawwidun::Itimad;
use taarib_usus::idadat::{Idadat, MakhzanIdadat, NawMuzawwid, Sima};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::masarat::{Masarat, insha_mujallad, kitaba_dharra};

/// One usable font in Taarib's own font directory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KhattHie {
    /// The file name the font is stored under.
    pub ism: String,
    /// Whether the font passed the Arabic validation, rather than only the
    /// Latin one.
    pub arabi: bool,
    /// The file's size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
}

/// Refuses a settings tree carrying a value the product cannot run on.
///
/// The provider checks are the ones that decide *where* a bad value is met. Each
/// of them names a tree that saves cleanly today and then fails at the moment
/// somebody presses a button that costs money — a blank local model name refused
/// by `bin_muzawwid`, a budget of zero refused by `nano_min_dolar` after the
/// verdict has already printed a ceiling of `$0.00` beside the sentence "the run
/// stops at the ceiling and never crosses it". Refusing them here costs the user
/// one corrected field on the screen they are already looking at.
fn tahaqqaq_idadat(idadat: &Idadat) -> Natija<()> {
    let mut muarrifat = BTreeSet::new();
    for tarif in &idadat.muzawwidun.qaima {
        if tarif.muarrif.trim().is_empty() {
            return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
                haql: "muzawwidun.qaima.muarrif",
                sabab: "a provider identifier is blank".to_owned(),
            }));
        }
        if !muarrifat.insert(tarif.muarrif.as_str()) {
            return Err(Khata::from(KhataIdadatAmr::MuzawwidMukarrar {
                muarrif: tarif.muarrif.clone(),
            }));
        }
        // Only for an enabled one. A half-filled entry that is switched off is a
        // draft somebody is still writing, and refusing to save the tree around
        // it would make the screen unusable between two fields.
        if tarif.mufaal && tarif.naw == NawMuzawwid::Mahalli && tarif.namudhaj.trim().is_empty() {
            return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
                haql: "muzawwidun.qaima.namudhaj",
                sabab: format!(
                    "local provider {} is enabled with no model name, which no run can use",
                    tarif.muarrif
                ),
            }));
        }
        // `None` is the documented "no ceiling at all" and stays allowed. A
        // *present* amount that is not a positive finite number is not a smaller
        // ceiling — it is no ceiling wearing the look of one.
        if tarif
            .mizaniya
            .is_some_and(|mablagh| !mablagh.is_finite() || mablagh <= 0.0)
        {
            return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
                haql: "muzawwidun.qaima.mizaniya",
                sabab: format!(
                    "the budget set for provider {} is not a positive finite amount, so it \
                     bounds nothing",
                    tarif.muarrif
                ),
            }));
        }
    }
    // A default naming nothing at all is a preference that cannot ever be
    // honoured, and the election silently uses a different provider instead —
    // the one whose bill the user then pays. Removing and renaming a provider
    // both carry the default with them, so this is reachable from a hand-edited
    // file and from an environment override, not from the screen.
    if let Some(ism) = &idadat.muzawwidun.iftiradi
        && !idadat
            .muzawwidun
            .qaima
            .iter()
            .any(|tarif| &tarif.muarrif == ism)
    {
        return Err(Khata::from(KhataIdadatAmr::MuzawwidIftiradiMajhul {
            muarrif: ism.clone(),
        }));
    }
    if idadat.tashkhis.ayyam_hifz == 0 {
        return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
            haql: "tashkhis.ayyam_hifz",
            sabab: "log retention of 0 days is below the minimum of 1".to_owned(),
        }));
    }
    if idadat.takhzin.hadd_makhbaa_mb < 64 {
        return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
            haql: "takhzin.hadd_makhbaa_mb",
            sabab: format!(
                "a cache ceiling of {} MB is below the minimum of 64",
                idadat.takhzin.hadd_makhbaa_mb
            ),
        }));
    }
    if idadat.masadir.fatra_tahdith < 5 {
        return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
            haql: "masadir.fatra_tahdith",
            sabab: format!(
                "a refresh interval of {} minutes is below the minimum of 5",
                idadat.masadir.fatra_tahdith
            ),
        }));
    }
    if !(0.0..=1.0_f32).contains(&idadat.tabaqa.shaffafiya) {
        return Err(Khata::from(KhataIdadatAmr::QeematGhayrSaliha {
            haql: "tabaqa.shaffafiya",
            sabab: format!(
                "an opacity of {} is outside 0.0..=1.0",
                idadat.tabaqa.shaffafiya
            ),
        }));
    }
    Ok(())
}

/// Whether the bytes parse as a font at all, and whether that font passes the
/// Arabic check.
fn fahs_khatt(bayt: &Arc<Vec<u8>>) -> Natija<bool> {
    if MawridKhatt::jadeed(Arc::clone(bayt), 0).is_ok() {
        Ok(true)
    } else {
        MawridKhatt::jadeed_latini(Arc::clone(bayt), 0).map(|_| false)
    }
}

/// The whole settings tree validated and persisted, answering the tree now in
/// force.
///
/// # Errors
///
/// [`KhataIdadatAmr::MuzawwidMukarrar`] and [`KhataIdadatAmr::QeematGhayrSaliha`]
/// when the tree is refused — nothing is persisted — and whatever saving the
/// settings file raises.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive owned arguments and managed state by value"
)]
pub fn haddith_idadat(
    idadat: Idadat,
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
    tatbiq: tauri::AppHandle,
) -> Result<Idadat, Khata> {
    tahaqqaq_idadat(&idadat)?;
    makhzan.ghayyir(|hali| *hali = idadat.clone())?;
    // After the store has accepted it, never before: a theme the store refused
    // must not reach the chrome and then contradict the settings screen.
    tabbiq_sima(&tatbiq, idadat.sima);
    Ok(Arc::unwrap_or_clone(makhzan.hali()))
}

/// The label `tauri.conf.json` gives the one window this application opens.
pub const NAFIDHA_RAISIYA: &str = "main";

/// Puts the native window chrome on the same palette as the document.
///
/// The titlebar is the operating system's, not the webview's, so the token
/// layer cannot reach it: `tauri.conf.json` pins it dark for the first frame
/// because dark is the default, and this moves it whenever the setting says
/// otherwise — a user on the light palette was getting a white document under a
/// black titlebar. `Nizam` hands the choice back to the platform rather than
/// resolving it here, which is the one case where the window knows more than
/// the store does. A window that will not take the theme keeps the one it had;
/// that is a cosmetic miss, not a reason to fail a settings save.
pub fn tabbiq_sima<M: tauri::Manager<tauri::Wry>>(mudir: &M, sima: Sima) {
    let Some(nafidha) = mudir.get_webview_window(NAFIDHA_RAISIYA) else {
        return;
    };
    let mawdu = match sima {
        Sima::Daken => Some(tauri::Theme::Dark),
        Sima::Fatih => Some(tauri::Theme::Light),
        Sima::Nizam => None,
    };
    if let Err(sabab) = nafidha.set_theme(mawdu) {
        tracing::warn!(sabab = %sabab, "the window chrome kept its previous theme");
    }
}

/// Stores a provider credential in the operating system's keychain and answers
/// `true`. The secret is never logged, echoed, or written anywhere else.
///
/// # Errors
///
/// Whatever the credential store raises: a blank, oversized, or
/// control-character secret is refused, and so is a platform store that cannot
/// hold the key.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive owned arguments and managed state by value"
)]
pub fn khzin_itimad_muzawwid(
    muzawwid: String,
    sirr: String,
    masarat: tauri::State<'_, Masarat>,
) -> Result<bool, Khata> {
    // The marker is the user saying "leave this machine untouched", and a
    // credential in the machine's keychain is exactly what it forbids.
    if masarat.mahmul() {
        return Err(Khata::from(KhataIdadatAmr::MahmulLaMafatih));
    }
    let _ = Itimad::khazzin(&muzawwid, &sirr).map_err(Khata::from)?;
    Ok(true)
}

/// Deletes a provider's stored credential and answers `true`; deleting one
/// that is not there succeeds, because the asked-for state already exists.
///
/// # Errors
///
/// Whatever the credential store raises refusing to delete a key it holds.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive owned arguments and managed state by value"
)]
pub fn imsah_itimad_muzawwid(muzawwid: String) -> Result<bool, Khata> {
    Itimad::imsah(&muzawwid).map_err(Khata::from)?;
    Ok(true)
}

/// Whether the operating system's keychain holds a credential for the provider.
///
/// # Errors
///
/// Whatever the credential store raises when it exists but cannot be opened.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive owned arguments and managed state by value"
)]
pub fn hal_itimad_muzawwid(muzawwid: String) -> Result<bool, Khata> {
    Ok(Itimad::min_khazina(&muzawwid)
        .map_err(Khata::from)?
        .is_some())
}

/// Every font in Taarib's own font directory, each validated for Arabic;
/// unreadable and unparseable files are skipped, never invented.
///
/// # Errors
///
/// Cannot currently fail; the signature is the uniform command contract.
#[allow(
    clippy::unnecessary_wraps,
    reason = "every command returns Result<T, Khata> so the generated TypeScript has one call \
              shape and one error shape across the whole surface; `allow` rather than `expect` \
              because a command that grows a real failure path must not then trip an unfulfilled \
              expectation"
)]
#[tauri::command]
#[specta::specta]
pub fn khutut_mutaha(masarat: tauri::State<'_, Masarat>) -> Result<Vec<KhattHie>, Khata> {
    let mut khutut = Vec::new();
    let judhur = crate::mukawwinat_tahmil::judhur_khutut(&masarat);
    for masar in crate::mukawwinat_tahmil::milaffat_khutut(&judhur) {
        let Some(ism) = masar.file_name().map(|q| q.to_string_lossy().into_owned()) else {
            continue;
        };
        let Ok(bayt) = std::fs::read(&masar) else {
            continue;
        };
        let hajm = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
        let Ok(arabi) = fahs_khatt(&Arc::new(bayt)) else {
            continue;
        };
        khutut.push(KhattHie { ism, arabi, hajm });
    }
    khutut.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
    Ok(khutut)
}

/// Imports one font file into Taarib's own font directory and answers its row.
///
/// The bytes are validated before anything is written. Importing a file whose
/// name and bytes are both already there is answered as it stands; a different
/// font owning the name is refused, never overwritten.
///
/// # Errors
///
/// [`KhataIdadatAmr::MalafTalif`] when a file exists and does not read,
/// [`KhataIdadatAmr::KhattGhayrSalih`] when the bytes are not a font,
/// [`KhataIdadatAmr::KhattMutaarid`] when a different font owns the name, and
/// whatever writing into the font directory raises.
#[tauri::command]
#[specta::specta]
pub fn ikhtar_khatt(masar: String, masarat: tauri::State<'_, Masarat>) -> Result<KhattHie, Khata> {
    let masar = PathBuf::from(masar);
    let Some(ism) = masar.file_name().map(|q| q.to_string_lossy().into_owned()) else {
        return Err(Khata::from(KhataIdadatAmr::KhattGhayrSalih {
            masar,
            sabab: "the path does not name a file".to_owned(),
        }));
    };
    let bayt = std::fs::read(&masar).map_err(|sabab| {
        Khata::from(KhataIdadatAmr::MalafTalif {
            masar: masar.clone(),
            sabab: sabab.to_string(),
        })
    })?;
    let hajm = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
    let bayt = Arc::new(bayt);
    let arabi = fahs_khatt(&bayt).map_err(|khata| {
        Khata::from(KhataIdadatAmr::KhattGhayrSalih {
            masar: masar.clone(),
            sabab: khata.injilizi,
        })
    })?;

    let mujallad = masarat.khutut();
    let hadaf = mujallad.join(&ism);
    if hadaf.is_file() {
        let mawjud = std::fs::read(&hadaf).map_err(|sabab| {
            Khata::from(KhataIdadatAmr::MalafTalif {
                masar: hadaf.clone(),
                sabab: sabab.to_string(),
            })
        })?;
        if mawjud != *bayt {
            return Err(Khata::from(KhataIdadatAmr::KhattMutaarid { ism }));
        }
    } else {
        insha_mujallad(&mujallad)?;
        kitaba_dharra(&hadaf, bayt.as_slice())?;
    }
    Ok(KhattHie { ism, arabi, hajm })
}

/// Failures of the settings surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataIdadatAmr {
    /// Two providers in the list carry the same identifier.
    #[error("provider identifier {muarrif} appears more than once")]
    MuzawwidMukarrar {
        /// The identifier that repeats.
        muarrif: String,
    },

    /// The elected default names no provider in the list.
    #[error("the default provider {muarrif} is not in the provider list")]
    MuzawwidIftiradiMajhul {
        /// The identifier the default names.
        muarrif: String,
    },

    /// A settings field holds a value outside its accepted bounds.
    #[error("settings field {haql} is invalid: {sabab}")]
    QeematGhayrSaliha {
        /// The field, as the settings tree spells it.
        haql: &'static str,
        /// What is wrong with the value.
        sabab: String,
    },

    /// A portable installation refuses to write to this machine's keychain.
    #[error("a portable installation does not write to this machine's keychain")]
    MahmulLaMafatih,

    /// The chosen file is not a font either loader can read.
    #[error("{} is not a usable font: {sabab}", masar.display())]
    KhattGhayrSalih {
        /// The file that was chosen.
        masar: PathBuf,
        /// What the font loader said.
        sabab: String,
    },

    /// A different font already owns that name in the font directory.
    #[error("a different font already owns the name {ism}")]
    KhattMutaarid {
        /// The contested file name.
        ism: String,
    },

    /// A file exists and does not read.
    #[error("{} does not read: {sabab}", masar.display())]
    MalafTalif {
        /// The file.
        masar: PathBuf,
        /// What the reader said.
        sabab: String,
    },
}

impl Tafsir for KhataIdadatAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MuzawwidMukarrar { .. } => 90,
                    Self::QeematGhayrSaliha { .. } => 91,
                    Self::KhattGhayrSalih { .. } => 92,
                    Self::KhattMutaarid { .. } => 93,
                    Self::MalafTalif { .. } => 94,
                    Self::MahmulLaMafatih => 95,
                    Self::MuzawwidIftiradiMajhul { .. } => 96,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // The tree or the name was refused whole; nothing was persisted.
            Self::MuzawwidMukarrar { .. }
            | Self::MuzawwidIftiradiMajhul { .. }
            | Self::QeematGhayrSaliha { .. }
            | Self::KhattMutaarid { .. }
            | Self::MahmulLaMafatih => Khutura::Tanbeeh,
            // The user asked for an import and did not get one.
            Self::KhattGhayrSalih { .. } | Self::MalafTalif { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MuzawwidMukarrar { muarrif } => format!(
                "المعرّف «{muarrif}» مستخدم لأكثر من مزوّد واحد، ولم يُحفظ شيء من \
                 الإعدادات. اجعل لكل مزوّد معرّفًا فريدًا ثم احفظ من جديد."
            ),
            Self::MuzawwidIftiradiMajhul { muarrif } => format!(
                "المزوّد الافتراضي «{muarrif}» ليس في قائمة المزوّدين، ولم يُحفظ شيء من \
                 الإعدادات. لو حُفظ لاستخدمت الترجمة الجديدة مزوّدًا آخر دون أن يُقال لك، \
                 وهو المزوّد الذي ستُحتسب تكلفته. اختر افتراضيًّا من القائمة أو أزل التحديد."
            ),
            Self::QeematGhayrSaliha { haql, .. } => format!(
                "قيمة الحقل {haql} خارج حدودها المقبولة، ولم يُحفظ شيء من الإعدادات. \
                 صحّح القيمة ثم احفظ من جديد."
            ),
            Self::KhattGhayrSalih { masar, .. } => format!(
                "الملف {} ليس ملف خط يمكن قراءته، ولم يُنسخ إلى مجلد الخطوط. اختر ملف \
                 TTF أو OTF سليمًا.",
                masar.display()
            ),
            Self::KhattMutaarid { ism } => format!(
                "خط آخر بمحتوى مختلف يحمل الاسم {ism} في مجلد الخطوط، ولن يُكتب فوقه. \
                 أعد تسمية الملف ثم استورده من جديد."
            ),
            Self::MalafTalif { masar, .. } => format!(
                "الملف {} موجود ولا يُقرأ. لن يُكتب فوقه؛ افحصه أو انقله ثم أعد المحاولة.",
                masar.display()
            ),
            Self::MahmulLaMafatih => "هذه نسخة محمولة، ولا تكتب شيئًا في مفاتيح هذا \
                 الجهاز. انسخ تعريب إلى الجهاز نفسه إن أردت حفظ اعتماد المزوّد فيه."
                .to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MuzawwidMukarrar { muarrif } => format!(
                "The identifier {muarrif} is used by more than one provider; nothing was \
                 saved. Give every provider a unique identifier, then save again."
            ),
            Self::MuzawwidIftiradiMajhul { muarrif } => format!(
                "The default provider {muarrif} is not in the provider list; nothing was \
                 saved. Saved as it stands, a new translation would silently use a different \
                 provider — and that is the one that would be charged for. Pick a default \
                 from the list, or clear it."
            ),
            Self::QeematGhayrSaliha { haql, sabab } => format!(
                "Settings field {haql} holds a value outside its accepted bounds \
                 ({sabab}); nothing was saved. Correct it, then save again."
            ),
            Self::KhattGhayrSalih { masar, sabab } => format!(
                "{} is not a font either loader can read ({sabab}); it was not copied \
                 into the font directory. Choose a valid TTF or OTF file.",
                masar.display()
            ),
            Self::KhattMutaarid { ism } => format!(
                "A different font already owns the name {ism} in the font directory and \
                 will not be overwritten. Rename the file, then import it again."
            ),
            Self::MalafTalif { masar, sabab } => format!(
                "{} exists and does not read ({sabab}). Nothing will be written over it; \
                 inspect or move it, then retry.",
                masar.display()
            ),
            Self::MahmulLaMafatih => "This is a portable installation, which writes nothing \
                 into this machine's keychain. Install Taarib on the machine itself to keep a \
                 provider credential there."
                .to_owned(),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MuzawwidMukarrar { .. } | Self::MuzawwidIftiradiMajhul { .. } => {
                Khutwa::FathIdadat {
                    qism: QismIdadat::Muzawwidun,
                }
            },
            Self::QeematGhayrSaliha { haql, .. } => match *haql {
                "muzawwidun.qaima.muarrif"
                | "muzawwidun.qaima.namudhaj"
                | "muzawwidun.qaima.mizaniya" => Khutwa::FathIdadat {
                    qism: QismIdadat::Muzawwidun,
                },
                "takhzin.hadd_makhbaa_mb" => Khutwa::FathIdadat {
                    qism: QismIdadat::Takhzin,
                },
                "masadir.fatra_tahdith" => Khutwa::FathIdadat {
                    qism: QismIdadat::Masadir,
                },
                "tashkhis.ayyam_hifz" => Khutwa::FathIdadat {
                    qism: QismIdadat::Tashkhis,
                },
                // The overlay has no Settings section of its own; the sentence
                // names the field.
                _ => Khutwa::LaShay,
            },
            Self::KhattGhayrSalih { .. } => Khutwa::IkhtiyarKhattAakhar,
            Self::KhattMutaarid { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Khutut,
            },
            Self::MalafTalif { .. } => Khutwa::AadaMuhawala,
            Self::MahmulLaMafatih => Khutwa::FathIdadat {
                qism: QismIdadat::Muzawwidun,
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MuzawwidMukarrar { muarrif } | Self::MuzawwidIftiradiMajhul { muarrif } => {
                let _ = siyaq.insert("muarrif".to_owned(), QeemaSiyaq::Nass(muarrif.clone()));
            },
            Self::QeematGhayrSaliha { haql, sabab } => {
                let _ = siyaq.insert("haql".to_owned(), QeemaSiyaq::Nass((*haql).to_owned()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::KhattGhayrSalih { masar, sabab } | Self::MalafTalif { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::KhattMutaarid { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            // The refusal is the whole message: a portable install keeps no
            // keyring, so there is no path, no field and no name to add.
            Self::MahmulLaMafatih => {},
        }
        siyaq
    }
}

khata_min!(KhataIdadatAmr);
