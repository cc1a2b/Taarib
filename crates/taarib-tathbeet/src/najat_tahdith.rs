//! Update survival: classify on launch what moved underneath an installed patch, on evidence only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taarib_mustalahat::bina::{Basma, BinaId};
use taarib_tarqee::irtibat::{HukmIrtibat, IrtibatBina, MukhattatBasma, SababMutabaqa};
use taarib_tarqee::khata::KhataTarqee;
use taarib_usus::khata::{Khutwa, Tafsir as _};

use crate::bayan::{NawTathbeet, Tathbeet, waqt_alaan};
use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io};
use crate::tahaqquq::{HalatMalaf, NatijatTahaqquq, SababInhiraf, TaqreerTahaqquq, tahaqquq_luba};
use crate::taraju::{RadIdad, SiyasatIstiada, TaqreerIstiada, istiada_nass, istiada_sawt};

/// A string table keyed by stable identity, for counting what a re-match would migrate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JadwalNusus {
    /// One source text per identity.
    pub nusus: BTreeMap<String, String>,
}

impl JadwalNusus {
    /// Builds a table from identity–text pairs; a later pair replaces an earlier one.
    #[must_use]
    pub fn min_azwaj<I: IntoIterator<Item = (String, String)>>(azwaj: I) -> Self {
        Self {
            nusus: azwaj.into_iter().collect(),
        }
    }

    /// How many identities the table holds.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.nusus.len()
    }

    /// Whether the table holds nothing.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.nusus.is_empty()
    }
}

/// What a re-match against the new build would do to the patch's strings, as counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IhsaHijra {
    /// Identities present in both tables with byte-identical source text: migrate unchanged.
    pub thabita: usize,
    /// Identities present in both tables whose source text differs: need re-translation review.
    pub mutaghayyira: usize,
    /// Identities only in the new build's table.
    pub jadida: usize,
    /// Identities only in the patch's table: gone from the new build.
    pub dhahiba: usize,
}

impl IhsaHijra {
    /// Counts by exact identity and byte-identical text; nothing is fuzzy-matched.
    #[must_use]
    pub fn ihsab(qadeem: &JadwalNusus, jadeed: &JadwalNusus) -> Self {
        let mut ihsa = Self {
            thabita: 0,
            mutaghayyira: 0,
            jadida: 0,
            dhahiba: 0,
        };
        for (huwiya, nass) in &qadeem.nusus {
            match jadeed.nusus.get(huwiya) {
                Some(hali) if hali == nass => {
                    ihsa.thabita = ihsa.thabita.saturating_add(1);
                },
                Some(_) => ihsa.mutaghayyira = ihsa.mutaghayyira.saturating_add(1),
                None => ihsa.dhahiba = ihsa.dhahiba.saturating_add(1),
            }
        }
        for huwiya in jadeed.nusus.keys() {
            if !qadeem.nusus.contains_key(huwiya) {
                ihsa.jadida = ihsa.jadida.saturating_add(1);
            }
        }
        ihsa
    }

    /// How many identities the patch's table held in total.
    #[must_use]
    pub const fn majmu_qadeem(&self) -> usize {
        self.thabita
            .saturating_add(self.mutaghayyira)
            .saturating_add(self.dhahiba)
    }

    /// How many identities the new build's table holds in total.
    #[must_use]
    pub const fn majmu_jadeed(&self) -> usize {
        self.thabita
            .saturating_add(self.mutaghayyira)
            .saturating_add(self.jadida)
    }

    /// The counts as one English line.
    #[must_use]
    pub fn satr(&self) -> String {
        format!(
            "{} string(s) migrate unchanged, {} new, {} changed, {} gone",
            self.thabita, self.jadida, self.mutaghayyira, self.dhahiba
        )
    }

    /// The counts as one Arabic line.
    #[must_use]
    pub fn satr_arabi(&self) -> String {
        format!(
            "تُنقل {} سلسلة دون تغيير، و{} جديدة، و{} تغيّر أصلها، و{} زالت من البناء الجديد",
            self.thabita, self.jadida, self.mutaghayyira, self.dhahiba
        )
    }
}

/// A build identity recomputed or supplied for the current install, with its verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqdeerBina {
    /// The build as it stands now.
    pub bina: BinaId,
    /// How the patch's binding judged it.
    pub hukm: HukmIrtibat,
}

impl TaqdeerBina {
    /// The verdict as report lines, Arabic and English.
    #[must_use]
    pub fn sutur(&self) -> Vec<String> {
        let mut sutur = vec![
            format!(
                "  current build {}: fingerprint {} over {} file(s) — {}",
                self.bina.wasm(),
                self.bina.basma.mukhtasara(),
                self.bina.adad_malaffat,
                self.hukm.sabab.wasf_injilizi()
            ),
            format!("  {}", self.hukm.sabab.wasf_arabi()),
        ];
        if self.hukm.naqis {
            sutur.push(
                "  fewer files were fingerprinted than the recipe names; the game may still \
                 be downloading"
                    .to_owned(),
            );
            sutur.push(
                "  عدد الملفات المبصومة أقل مما تسمّيه الوصفة؛ قد تكون اللعبة قيد التنزيل"
                    .to_owned(),
            );
        }
        sutur
    }
}

/// The evidence behind a "still applies" verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaleelTatbaq {
    /// The recomputed fingerprint matches one the patch was verified against.
    Basma(TaqdeerBina),
    /// Every fingerprint container is still byte-for-byte what Taarib wrote.
    TarqeeSalim,
}

/// The evidence behind a "text changed" verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaleelTaghayyur {
    /// The recomputed fingerprint matches none the patch was verified against.
    Basma(TaqdeerBina),
    /// A file the fingerprint recipe names is no longer on disk, so no recorded
    /// fingerprint can match this install.
    HawiyaMafquda {
        /// The absent file.
        masar: PathBuf,
    },
}

/// Why the survival check declined to pick one of the four named outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SababGhayrMahsum {
    /// Verification attributed the drift to a person, not the store.
    TaghyeerMustakhdim,
    /// Verification found drift and could not attribute it.
    InhirafGhayrMuakkad,
    /// Part of the fingerprint recipe still holds the patch's own bytes, so a
    /// recomputed fingerprint would not describe the store's build.
    BasmaMulawwatha {
        /// Recipe files whose current bytes are Taarib's write.
        mutabiqa: usize,
        /// Recipe files in total.
        kull: usize,
    },
}

impl SababGhayrMahsum {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::TaghyeerMustakhdim => "a local edit, not an update",
            Self::InhirafGhayrMuakkad => "drift with no established cause",
            Self::BasmaMulawwatha { .. } => "fingerprint not recomputable",
        }
    }

    /// The sentence an Arabic-speaking user reads.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::TaghyeerMustakhdim => NatijatTahaqquq::TaghyeerMustakhdim.arabi(),
            Self::InhirafGhayrMuakkad => NatijatTahaqquq::GhayrMuakkad.arabi(),
            Self::BasmaMulawwatha { .. } => {
                "بعض حاويات بصمة البناء ما زالت تحمل ما كتبه تعريب وبعضها لا مرجع له، فلا \
                 يمكن إعادة حساب البصمة دون تخمين، ولن يخمّن تعريب."
            },
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::TaghyeerMustakhdim => NatijatTahaqquq::TaghyeerMustakhdim.injilizi(),
            Self::InhirafGhayrMuakkad => NatijatTahaqquq::GhayrMuakkad.injilizi(),
            Self::BasmaMulawwatha { .. } => {
                "Some fingerprint containers still hold the patch's own bytes and the rest \
                 have nothing to be checked against, so the build fingerprint cannot be \
                 recomputed without guessing — and Taarib will not guess."
            },
        }
    }
}

/// What launch-time survival concluded about one installed patch.
#[derive(Debug)]
pub enum MasirRuqaa {
    /// The patch still applies. Nothing was done and nothing needs doing.
    Tatbaq {
        /// The evidence.
        daleel: DaleelTatbaq,
    },

    /// The build's text moved past the patch. Re-matching is offered, with
    /// migration counts where the caller supplied both string tables.
    NassTaghayyar {
        /// The evidence.
        daleel: DaleelTaghayyur,
        /// The counts, when comparison inputs were provided.
        hijra: Option<IhsaHijra>,
    },

    /// Recorded paths are gone from disk, so the patch cannot apply. The
    /// preserved originals were restored through `taraju`, and the attempt's
    /// own outcome is carried whole.
    LaTatbaq {
        /// What the restore did, or the failure that stopped it.
        istiada: NatijatTathbeet<TaqreerIstiada>,
    },

    /// The store rewrote patched files. Reinstall or restore is offered;
    /// nothing is ever reapplied over files the launcher just replaced.
    TabdeelMatjar {
        /// The new build's recomputed verdict, when it could be computed
        /// without reading the patch's own bytes as though they were the
        /// store's.
        taqdeer: Option<TaqdeerBina>,
    },

    /// The evidence does not decide, or the drift is not the store's doing.
    /// Reported as such; nothing is resolved by coin flip.
    GhayrMahsum {
        /// Why.
        sabab: SababGhayrMahsum,
    },
}

impl MasirRuqaa {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(&self) -> &'static str {
        match self {
            Self::Tatbaq { .. } => "still applies",
            Self::NassTaghayyar { .. } => "partially stale",
            Self::LaTatbaq { .. } => "cannot apply",
            Self::TabdeelMatjar { .. } => "store overwrote",
            Self::GhayrMahsum { .. } => "unresolved",
        }
    }

    /// Whether the patch survived the update with nothing to do.
    #[must_use]
    pub const fn tatbaq(&self) -> bool {
        matches!(self, Self::Tatbaq { .. })
    }

    /// The sentence an Arabic-speaking user reads.
    #[must_use]
    pub fn arabi(&self) -> String {
        match self {
            Self::Tatbaq { daleel } => match daleel {
                DaleelTatbaq::TarqeeSalim => {
                    "لم يمسّ التحديث أيّ حاوية نصوص مرقّعة؛ الترجمة ما زالت في مكانها ولا \
                     شيء يلزم فعله."
                        .to_owned()
                },
                DaleelTatbaq::Basma(taqdeer) => {
                    if matches!(taqdeer.hukm.sabab, SababMutabaqa::MuarrifWaBasma) {
                        "رقم البناء وبصمة النصوص متطابقان؛ لم يتغيّر شيء تحت الرقعة.".to_owned()
                    } else {
                        "تغيّر رقم البناء ولم تتغيّر ملفات النصوص؛ الرقعة تنطبق على البناء \
                         الجديد كما هي."
                            .to_owned()
                    }
                },
            },
            Self::NassTaghayyar { .. } => {
                "تغيّرت ملفات النصوص في البناء الجديد فلا تنطبق الرقعة كما هي. تُعرض إعادة \
                 المطابقة مع البناء الجديد، ولا يُخمَّن أيّ تطابق."
                    .to_owned()
            },
            Self::LaTatbaq { istiada } => match istiada {
                Ok(taqreer) => format!(
                    "ملفات مسجّلة للرقعة لم تعد موجودة على القرص فلا يمكن تطبيقها. أُعيدت \
                     الأصول المحفوظة ({} ملفًا)؛ تحقّق من سلامة ملفات اللعبة من متجرها.",
                    taqreer.mustaada
                ),
                Err(khata) => format!(
                    "ملفات مسجّلة للرقعة لم تعد موجودة على القرص فلا يمكن تطبيقها، وفشلت \
                     الاستعادة نفسها أيضًا. {}",
                    khata.arabi()
                ),
            },
            Self::TabdeelMatjar { .. } => {
                "حدّث المتجر اللعبة وأعاد كتابة ملفات كانت مرقّعة. لن يعيد تعريب تطبيق \
                 الرقعة فوقها من تلقاء نفسه: إمّا إعادة المطابقة ثم التثبيت، وإمّا الإزالة \
                 واستعادة ما بقي."
                    .to_owned()
            },
            Self::GhayrMahsum { sabab } => sabab.arabi().to_owned(),
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        match self {
            Self::Tatbaq { daleel } => match daleel {
                DaleelTatbaq::TarqeeSalim => {
                    "The update did not touch any patched text container; the translation is \
                     still in place and nothing needs doing."
                        .to_owned()
                },
                DaleelTatbaq::Basma(taqdeer) => {
                    if matches!(taqdeer.hukm.sabab, SababMutabaqa::MuarrifWaBasma) {
                        "The build id and the text fingerprint both match; nothing moved \
                         underneath the patch."
                            .to_owned()
                    } else {
                        "The build id moved and the text files did not; the patch applies to \
                         the new build exactly as it is."
                            .to_owned()
                    }
                },
            },
            Self::NassTaghayyar { .. } => {
                "The new build's text files changed, so the patch no longer applies as it \
                 is. Re-matching against the new build is offered; no match is guessed."
                    .to_owned()
            },
            Self::LaTatbaq { istiada } => match istiada {
                Ok(taqreer) => format!(
                    "Paths the patch recorded are no longer on disk, so it cannot apply. The \
                     preserved originals were restored ({} file(s)); verify the game's files \
                     through its launcher.",
                    taqreer.mustaada
                ),
                Err(khata) => format!(
                    "Paths the patch recorded are no longer on disk, so it cannot apply, and \
                     the restore itself failed too. {}",
                    khata.injilizi()
                ),
            },
            Self::TabdeelMatjar { .. } => {
                "The store updated the game and rewrote patched files. Taarib will never \
                 reapply the patch over files the launcher just replaced: either re-match \
                 and reinstall, or uninstall and restore what remains."
                    .to_owned()
            },
            Self::GhayrMahsum { sabab } => sabab.injilizi().to_owned(),
        }
    }

    /// The primary action to offer the user.
    #[must_use]
    pub fn khutwa(&self) -> Khutwa {
        match self {
            Self::Tatbaq { .. } => Khutwa::LaShay,
            Self::NassTaghayyar { .. } | Self::TabdeelMatjar { .. } => Khutwa::IadatMutabaqaBina,
            Self::LaTatbaq { istiada } => match istiada {
                Ok(_) => Khutwa::TahaqquqSalamatLuba,
                Err(khata) => khata.khutwa(),
            },
            Self::GhayrMahsum { sabab } => match sabab {
                SababGhayrMahsum::TaghyeerMustakhdim => Khutwa::IlghaTathbeet,
                SababGhayrMahsum::InhirafGhayrMuakkad
                | SababGhayrMahsum::BasmaMulawwatha { .. } => Khutwa::FathTashkhis,
            },
        }
    }

    /// Every action to offer, in the order an interface shows them.
    #[must_use]
    pub fn khiyarat(&self) -> Vec<Khutwa> {
        match self {
            Self::TabdeelMatjar { .. } => {
                vec![Khutwa::IadatMutabaqaBina, Khutwa::IlghaTathbeet]
            },
            _ => vec![self.khutwa()],
        }
    }
}

/// Where the current build's identity comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasdarBina<'a> {
    /// The caller already computed it, current as of now.
    Muqaddam(&'a BinaId),
    /// Recompute it here through [`MukhattatBasma::ihsab`].
    Ihsab {
        /// The launcher's build identifier, when the launcher has one.
        manassa: Option<String>,
    },
}

/// Everything one survival check needs.
#[derive(Debug, Clone)]
pub struct TalabNajat<'a> {
    /// The game's install directory as it is now.
    pub jidhr_luba: &'a Path,
    /// The game's directory under `nusakh/`.
    pub jidhr_nusakh: &'a Path,
    /// Which installation is being checked.
    pub naw: NawTathbeet,
    /// The patch's binding, from the package.
    pub irtibat: &'a IrtibatBina,
    /// The current build's identity, supplied or recomputed.
    pub bina: MasdarBina<'a>,
    /// The patch's own string table, for migration counts.
    pub nusus_qadima: Option<&'a JadwalNusus>,
    /// The new build's extracted string table, for migration counts.
    pub nusus_jadida: Option<&'a JadwalNusus>,
    /// How a restore treats files the store already replaced.
    pub siyasa: SiyasatIstiada,
}

/// One survival check's outcome and the report a person reads.
#[derive(Debug)]
pub struct TaqreerNajat {
    /// The game.
    pub luba: String,
    /// Which installation was checked.
    pub naw: NawTathbeet,
    /// When the patch was installed, RFC 3339, as the manifest recorded it.
    pub waqt_tathbeet: String,
    /// The build fingerprint the patch was installed against, when recorded.
    pub basma_tathbeet: Option<Basma>,
    /// The verification sweep the classification ran on.
    pub tahaqquq: TaqreerTahaqquq,
    /// The classified outcome.
    pub masir: MasirRuqaa,
}

impl TaqreerNajat {
    /// The primary action to offer the user.
    #[must_use]
    pub fn khutwa(&self) -> Khutwa {
        self.masir.khutwa()
    }

    /// The result as lines for the log, the bundle and the game's detail screen.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(16);
        sutur.push(format!(
            "{} [{}] installed {}: update survival — {}",
            self.luba,
            self.naw.ism(),
            self.waqt_tathbeet,
            self.masir.ism()
        ));
        if let Some(basma) = self.basma_tathbeet {
            sutur.push(format!(
                "  installed against build fingerprint {}",
                basma.mukhtasara()
            ));
        }
        sutur.push(format!("  {}", self.masir.arabi()));
        sutur.push(format!("  {}", self.masir.injilizi()));

        match &self.masir {
            MasirRuqaa::Tatbaq { daleel } => {
                if let DaleelTatbaq::Basma(taqdeer) = daleel {
                    sutur.extend(taqdeer.sutur());
                }
                if self.tahaqquq.adad_mustaad > 0 {
                    sutur.push(format!(
                        "  {} path(s) are back to the store's original bytes; reinstalling \
                         reapplies the patch exactly",
                        self.tahaqquq.adad_mustaad
                    ));
                    sutur.push(format!(
                        "  {} من الملفات عاد إلى أصله كما يشحنه المتجر؛ إعادة التثبيت تعيد \
                         الرقعة عليه تمامًا",
                        self.tahaqquq.adad_mustaad
                    ));
                }
            },
            MasirRuqaa::NassTaghayyar { daleel, hijra } => {
                match daleel {
                    DaleelTaghayyur::Basma(taqdeer) => sutur.extend(taqdeer.sutur()),
                    DaleelTaghayyur::HawiyaMafquda { masar } => {
                        sutur.push(format!(
                            "  {} is named by the fingerprint recipe and is not on disk",
                            masar.display()
                        ));
                    },
                }
                match hijra {
                    Some(ihsa) => {
                        sutur.push(format!("  {}", ihsa.satr_arabi()));
                        sutur.push(format!("  {}", ihsa.satr()));
                    },
                    None => sutur.push(
                        "  no comparison inputs were provided, so migration counts are not \
                         available"
                            .to_owned(),
                    ),
                }
            },
            MasirRuqaa::LaTatbaq { istiada } => {
                for (masar, hala) in &self.tahaqquq.halat {
                    if matches!(hala, HalatMalaf::Mafqud | HalatMalaf::MujalladMafqud) {
                        sutur.push(format!("  {masar}: {}", hala.ism()));
                    }
                }
                match istiada {
                    Ok(taqreer) => sutur.extend(taqreer.taqreer()),
                    Err(khata) => {
                        sutur.push(format!("  الاستعادة فشلت: {}", khata.arabi()));
                        sutur.push(format!("  the restore failed: {}", khata.injilizi()));
                    },
                }
            },
            MasirRuqaa::TabdeelMatjar { taqdeer } => {
                if let SababInhiraf::Matjar { adad, .. } = self.tahaqquq.sabab {
                    sutur.push(format!(
                        "  {adad} patched file(s) were rewritten in one pass"
                    ));
                }
                match taqdeer {
                    Some(taqdeer) => {
                        sutur.extend(taqdeer.sutur());
                        if matches!(
                            taqdeer.hukm.sabab,
                            SababMutabaqa::MuarrifWaBasma | SababMutabaqa::BasmaFaqat
                        ) {
                            sutur.push(
                                "  the new build's text files match a build this patch was \
                                 verified against, so reinstalling is exact"
                                    .to_owned(),
                            );
                            sutur.push(
                                "  ملفات نصوص البناء الجديد تطابق بناءً تحقّقت منه الرقعة، \
                                 فإعادة التثبيت مطابقة تمامًا"
                                    .to_owned(),
                            );
                        } else {
                            sutur.push(
                                "  the new build's text files match no verified build; \
                                 re-match before reinstalling"
                                    .to_owned(),
                            );
                            sutur.push(
                                "  ملفات نصوص البناء الجديد لا تطابق أيّ بناء متحقَّق منه؛ \
                                 أعد المطابقة قبل التثبيت"
                                    .to_owned(),
                            );
                        }
                    },
                    None => sutur.push(
                        "  the new build's fingerprint was not recomputed: part of the \
                         recipe still holds the patch's bytes"
                            .to_owned(),
                    ),
                }
                sutur.push("  الخياران: إعادة المطابقة ثم التثبيت، أو الإزالة والاستعادة".to_owned());
                sutur
                    .push("  offered: re-match and reinstall, or uninstall and restore".to_owned());
            },
            MasirRuqaa::GhayrMahsum { sabab } => {
                sutur.push(format!("  undecided as: {}", sabab.ism()));
                if let SababGhayrMahsum::BasmaMulawwatha { mutabiqa, kull } = sabab {
                    sutur.push(format!(
                        "  {mutabiqa} of {kull} fingerprint container(s) still hold the \
                         patch's bytes"
                    ));
                }
                if let SababInhiraf::GhayrMuakkad { sabab: tafsil, .. } = self.tahaqquq.sabab {
                    sutur.push(format!("  undecided because {tafsil}"));
                }
            },
        }

        sutur
    }
}

/// Classifies what an update did to one installed patch.
///
/// Acts only where the evidence is exact: a fingerprint match is reported,
/// missing targets are restored through `taraju`, and everything else becomes an
/// offer or a named refusal to guess.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] or [`KhataTathbeet::IsdarBayanMajhul`] when the
/// manifest cannot be read or trusted, [`KhataTathbeet::MasarKharij`] when it
/// names a path outside the game, the I/O variants when a file cannot be read
/// while verifying or fingerprinting, and [`KhataTathbeet::TawafuqMarfud`] when
/// the package's fingerprint recipe names no files. A restore that fails is not
/// an error here: it is carried inside [`MasirRuqaa::LaTatbaq`], because the
/// classification stands whatever the restore then managed.
pub fn fahs_najat(talab: &TalabNajat<'_>, radd: &mut dyn RadIdad) -> NatijatTathbeet<TaqreerNajat> {
    let tathbeet = Tathbeet::istanif(talab.jidhr_luba, talab.jidhr_nusakh, talab.naw)?;
    let basma_tathbeet = tathbeet.bayan().basma_bina;
    drop(tathbeet);

    let tahaqquq = tahaqquq_luba(talab.jidhr_luba, talab.jidhr_nusakh, talab.naw)?;
    let masir = sannif(talab, &tahaqquq, radd)?;

    Ok(TaqreerNajat {
        luba: tahaqquq.luba.clone(),
        naw: talab.naw,
        waqt_tathbeet: tahaqquq.waqt_tathbeet.clone(),
        basma_tathbeet,
        tahaqquq,
        masir,
    })
}

/// Turns the verification verdict into the survival outcome.
fn sannif(
    talab: &TalabNajat<'_>,
    tahaqquq: &TaqreerTahaqquq,
    radd: &mut dyn RadIdad,
) -> NatijatTathbeet<MasirRuqaa> {
    match tahaqquq.natija() {
        NatijatTahaqquq::Naqis => Ok(MasirRuqaa::LaTatbaq {
            istiada: istaid(talab, radd),
        }),
        NatijatTahaqquq::MustabdalMinAlmatjar => Ok(MasirRuqaa::TabdeelMatjar {
            taqdeer: taqdeer_bad_altabdeel(talab, tahaqquq),
        }),
        NatijatTahaqquq::TaghyeerMustakhdim => Ok(MasirRuqaa::GhayrMahsum {
            sabab: SababGhayrMahsum::TaghyeerMustakhdim,
        }),
        NatijatTahaqquq::GhayrMuakkad => Ok(MasirRuqaa::GhayrMahsum {
            sabab: SababGhayrMahsum::InhirafGhayrMuakkad,
        }),
        NatijatTahaqquq::Salim => sannif_salim(talab, tahaqquq),
    }
}

/// Decides between "still applies", "text changed" and "not recomputable" for
/// an installation whose patched files are all intact.
fn sannif_salim(talab: &TalabNajat<'_>, tahaqquq: &TaqreerTahaqquq) -> NatijatTathbeet<MasirRuqaa> {
    let kull = talab.irtibat.mukhattat.adad();
    let mutabiqa = adad_mutabiq(&talab.irtibat.mukhattat, &tahaqquq.halat);

    if kull > 0 && mutabiqa == kull {
        return Ok(MasirRuqaa::Tatbaq {
            daleel: DaleelTatbaq::TarqeeSalim,
        });
    }
    if mutabiqa > 0 {
        // A recipe file still holding Taarib's write makes any current-disk
        // fingerprint non-evidence about the store's build.
        return Ok(MasirRuqaa::GhayrMahsum {
            sabab: SababGhayrMahsum::BasmaMulawwatha { mutabiqa, kull },
        });
    }

    match bina_hali(&talab.bina, talab.irtibat, talab.jidhr_luba) {
        Ok(bina) => {
            let hukm = talab.irtibat.ihkum(&bina);
            let taqdeer = TaqdeerBina { bina, hukm };
            match hukm.sabab {
                SababMutabaqa::MuarrifWaBasma | SababMutabaqa::BasmaFaqat => {
                    Ok(MasirRuqaa::Tatbaq {
                        daleel: DaleelTatbaq::Basma(taqdeer),
                    })
                },
                SababMutabaqa::MuarrifBilaBasma
                | SababMutabaqa::DakhilNitaq
                | SababMutabaqa::BilaTatabuq => Ok(MasirRuqaa::NassTaghayyar {
                    daleel: DaleelTaghayyur::Basma(taqdeer),
                    hijra: hijrat(talab),
                }),
            }
        },
        Err(KhataTarqee::MalafIrtibatMafqud { masar }) => Ok(MasirRuqaa::NassTaghayyar {
            daleel: DaleelTaghayyur::HawiyaMafquda { masar },
            hijra: hijrat(talab),
        }),
        Err(khata) => Err(min_khata_tarqee(talab.jidhr_luba, khata)),
    }
}

/// The new build's verdict after a store overwrite, or [`None`] when computing
/// one would read the patch's surviving bytes as though they were the store's.
///
/// Always recomputed rather than taken from [`MasdarBina::Muqaddam`]: an
/// identity computed before the overwrite describes the build the overwrite
/// replaced.
fn taqdeer_bad_altabdeel(
    talab: &TalabNajat<'_>,
    tahaqquq: &TaqreerTahaqquq,
) -> Option<TaqdeerBina> {
    if adad_mutabiq(&talab.irtibat.mukhattat, &tahaqquq.halat) > 0 {
        return None;
    }
    let manassa = match &talab.bina {
        MasdarBina::Muqaddam(bina) => bina.manassa.clone(),
        MasdarBina::Ihsab { manassa } => manassa.clone(),
    };
    let (basma, adad) = talab.irtibat.mukhattat.ihsab(talab.jidhr_luba).ok()?;
    let bina = BinaId {
        manassa,
        basma,
        adad_malaffat: adad,
        waqt: waqt_alaan(),
    };
    let hukm = talab.irtibat.ihkum(&bina);
    Some(TaqdeerBina { bina, hukm })
}

/// Restores the checked installation through the one restore path this crate has.
fn istaid(talab: &TalabNajat<'_>, radd: &mut dyn RadIdad) -> NatijatTathbeet<TaqreerIstiada> {
    match talab.naw {
        NawTathbeet::Nass => istiada_nass(talab.jidhr_luba, talab.jidhr_nusakh, talab.siyasa, radd),
        NawTathbeet::Sawt => istiada_sawt(talab.jidhr_luba, talab.jidhr_nusakh, talab.siyasa, radd),
    }
}

/// The current build's identity, supplied or recomputed from the recipe.
fn bina_hali(
    masdar: &MasdarBina<'_>,
    irtibat: &IrtibatBina,
    jidhr_luba: &Path,
) -> Result<BinaId, KhataTarqee> {
    match masdar {
        MasdarBina::Muqaddam(bina) => Ok((*bina).clone()),
        MasdarBina::Ihsab { manassa } => {
            let (basma, adad) = irtibat.mukhattat.ihsab(jidhr_luba)?;
            Ok(BinaId {
                manassa: manassa.clone(),
                basma,
                adad_malaffat: adad,
                waqt: waqt_alaan(),
            })
        },
    }
}

/// How many recipe files currently hold exactly what Taarib wrote.
fn adad_mutabiq(mukhattat: &MukhattatBasma, halat: &BTreeMap<String, HalatMalaf>) -> usize {
    mukhattat
        .hawiyat()
        .iter()
        .filter(|masar| matches!(halat.get(masar.as_str()), Some(HalatMalaf::Mutabiq)))
        .count()
}

/// Migration counts, when the caller supplied both tables.
fn hijrat(talab: &TalabNajat<'_>) -> Option<IhsaHijra> {
    match (talab.nusus_qadima, talab.nusus_jadida) {
        (Some(qadeem), Some(jadeed)) => Some(IhsaHijra::ihsab(qadeem, jadeed)),
        _ => None,
    }
}

/// Turns a fingerprint-recomputation failure into this crate's vocabulary.
fn min_khata_tarqee(jidhr_luba: &Path, khata: KhataTarqee) -> KhataTathbeet {
    match khata {
        KhataTarqee::KhataMalaf { masar, sabab } => {
            min_khata_io(&masar, "recomputing the build fingerprint", sabab)
        },
        KhataTarqee::BayanNaqis { haql } => KhataTathbeet::TawafuqMarfud {
            hukm: format!("the package's fingerprint recipe needs {haql} and names none"),
            yumkin_bi_iqrar: false,
        },
        akhar => KhataTathbeet::KhataMalaf {
            masar: jidhr_luba.to_path_buf(),
            amal: "recomputing the build fingerprint",
            sabab: std::io::Error::other(akhar.injilizi()),
        },
    }
}
