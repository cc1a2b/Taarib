//! Three-tier build matching, applied per game against the installed build.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::Serialize;
use taarib_mustalahat::bina::{Basma, BinaId, MutabaqaBina};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_mustalahat::sawt::MulakhkhasSawt;
use taarib_tarqee::irtibat::{HukmIrtibat, IrtibatBina, MukhattatBasma, NitaqBina, SababMutabaqa};

use crate::fahras::ShareehaMuwaththaqa;
use crate::khata::{KhataMustawda, NatijatMustawda};

/// How many declared build identifiers a refusal sentence names before it stops.
const HADD_ASMAA_BINA: usize = 3;

/// The verdict of a listing whose binding could not be constructed.
const HUKM_MURFUD: HukmIrtibat = HukmIrtibat { sabab: SababMutabaqa::BilaTatabuq, naqis: false };

/// What a full metadata record adds to the bindings an index listing carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct IdafatIrtibat {
    /// The compatibility range the contributor declared, where there is one.
    pub nitaq: Option<NitaqBina>,
    /// How many files the package's fingerprint recipe named.
    pub adad_malaffat: u32,
}

impl IdafatIrtibat {
    /// A range with no recipe size behind it.
    #[must_use]
    pub const fn min_nitaq(nitaq: NitaqBina) -> Self {
        Self { nitaq: Some(nitaq), adad_malaffat: 0 }
    }
}

/// Why a package does not apply to the installed build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SababGhayrTawafuq {
    /// The launcher build identifiers the package declares.
    pub bina_muallan: Vec<String>,
    /// The launcher build identifier installed, where the launcher reports one.
    pub bina_mawjud: Option<String>,
    /// How many content fingerprints the package declares.
    pub adad_basmat: usize,
    /// The installed build's fingerprint, shortened.
    pub basma_mawjuda: String,
    /// Whether the package declares no build binding at all.
    pub bila_irtibat: bool,
    /// Whether a declared range excluded this build.
    pub kharij_nitaq: bool,
}

impl SababGhayrTawafuq {
    /// The sentence the incompatible row carries, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        if self.bila_irtibat {
            return "لا تعلن هذه الحزمة أيّ ارتباط ببناء، فلا سبيل إلى التحقّق من توافقها."
                .to_owned();
        }
        if self.kharij_nitaq {
            return match self.bina_mawjud.as_deref() {
                Some(mawjud) => format!("بناء لعبتك {mawjud} خارج المدى الذي أعلنه المساهم."),
                None => "لا يذكر مشغّلك رقم بناء، والمدى الذي أعلنه المساهم أرقام بناء.".to_owned(),
            };
        }
        let basmat = format!(
            "ولا تطابق بصمة ملفّاتك ({}) أيًّا من {} بصمة تعلنها",
            self.basma_mawjuda, self.adad_basmat
        );
        match self.bina_mawjud.as_deref() {
            Some(mawjud) => format!(
                "بُنيت هذه الحزمة على {} ولديك {mawjud}، {basmat}.",
                self.binaat_maktuba()
            ),
            None => format!("لا يذكر مشغّلك رقم بناء، {basmat}."),
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        if self.bila_irtibat {
            return "this package declares no build binding, so nothing can be checked against it"
                .to_owned();
        }
        if self.kharij_nitaq {
            return match self.bina_mawjud.as_deref() {
                Some(mawjud) => {
                    format!("your build {mawjud} is outside the range the contributor declared")
                }
                None => {
                    "your launcher reports no build id and the range is over build ids".to_owned()
                }
            };
        }
        let basmat = format!(
            "and your files fingerprint to {}, which is none of the {} it declares",
            self.basma_mawjuda, self.adad_basmat
        );
        match self.bina_mawjud.as_deref() {
            Some(mawjud) => format!(
                "this package was built against {} and you have {mawjud}, {basmat}",
                self.binaat_maktuba()
            ),
            None => format!("your launcher reports no build id, {basmat}"),
        }
    }

    fn binaat_maktuba(&self) -> String {
        let mut maktub = self
            .bina_muallan
            .iter()
            .take(HADD_ASMAA_BINA)
            .cloned()
            .collect::<Vec<String>>()
            .join("، ");
        if self.bina_muallan.len() > HADD_ASMAA_BINA {
            maktub.push('…');
        }
        maktub
    }
}

/// A verdict for one published package — a patch or a voice pack — against one
/// installed build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MutabaqatRuqaa {
    /// The package's lineage.
    pub id: RuqaaId,
    /// The revision judged.
    pub murajaa: RuqaaRevision,
    /// The verdict and the reason that produced it.
    pub hukm: HukmIrtibat,
    /// Why it does not apply, present exactly when the tier is
    /// [`MutabaqaBina::Ghayr`].
    pub sabab_ghayr: Option<SababGhayrTawafuq>,
}

impl MutabaqatRuqaa {
    /// The tier the interface shows.
    #[must_use]
    pub const fn mutabaqa(&self) -> MutabaqaBina {
        self.hukm.mutabaqa()
    }

    /// Why the tier came out as it did.
    #[must_use]
    pub const fn sabab(&self) -> SababMutabaqa {
        self.hukm.sabab
    }

    /// Whether the client will install this at all.
    #[must_use]
    pub const fn qabila_lil_tathbeet(&self) -> bool {
        self.hukm.qabila_lil_tathbeet()
    }

    /// Whether installing requires an acknowledgement first.
    #[must_use]
    pub const fn yahtaj_iqrar(&self) -> bool {
        self.hukm.yahtaj_iqrar()
    }

    /// Whether the installation fingerprinted fewer files than the package's
    /// recipe names.
    #[must_use]
    pub const fn naqis(&self) -> bool {
        self.hukm.naqis
    }

    /// The whole verdict as one sentence, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let tabaqa = self.mutabaqa().wasf_arabi();
        match (&self.sabab_ghayr, self.hukm.naqis) {
            (Some(sabab), _) => format!("{tabaqa} — {}", sabab.wasf_arabi()),
            (None, true) => format!(
                "{tabaqa} — ويبدو أنّ لعبتك لم تكتمل بعد، فعدد ملفّاتها أقلّ ممّا تتوقّعه الحزمة."
            ),
            (None, false) => format!("{tabaqa} — {}", self.hukm.sabab.wasf_arabi()),
        }
    }

    /// The same in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let tabaqa = self.mutabaqa().wasf_injilizi();
        match (&self.sabab_ghayr, self.hukm.naqis) {
            (Some(sabab), _) => format!("{tabaqa} — {}", sabab.wasf_injilizi()),
            (None, true) => format!(
                "{tabaqa} — and your game fingerprinted fewer files than this package expects, so \
                 it may still be downloading"
            ),
            (None, false) => format!("{tabaqa} — {}", self.hukm.sabab.wasf_injilizi()),
        }
    }
}

/// Every verdict for one game, patches and voice packs alike.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MutabaqatLuba {
    /// The game judged.
    pub luba: LubaId,
    /// Every patch the shard lists for it, in listing order.
    pub ruqaa: Vec<MutabaqatRuqaa>,
    /// Every voice pack the shard lists for it, in listing order.
    pub aswat: Vec<MutabaqatRuqaa>,
}

impl MutabaqatLuba {
    /// Whether the registry lists nothing at all for this game.
    #[must_use]
    pub const fn faragh(&self) -> bool {
        self.ruqaa.is_empty() && self.aswat.is_empty()
    }

    /// The patches this build can install.
    pub fn ruqaa_mutawafiqa(&self) -> impl Iterator<Item = &MutabaqatRuqaa> {
        mutawafiqa(&self.ruqaa)
    }

    /// The patches it cannot, each carrying the reason.
    pub fn ruqaa_ghayr_mutawafiqa(&self) -> impl Iterator<Item = &MutabaqatRuqaa> {
        ghayr_mutawafiqa(&self.ruqaa)
    }

    /// The voice packs this build can install.
    pub fn aswat_mutawafiqa(&self) -> impl Iterator<Item = &MutabaqatRuqaa> {
        mutawafiqa(&self.aswat)
    }

    /// The voice packs it cannot, each carrying the reason.
    pub fn aswat_ghayr_mutawafiqa(&self) -> impl Iterator<Item = &MutabaqatRuqaa> {
        ghayr_mutawafiqa(&self.aswat)
    }

    /// The best-matching installable patch.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::LaMutabaqa`] when every listed patch is incompatible.
    pub fn afdal_ruqaa(&self) -> NatijatMustawda<&MutabaqatRuqaa> {
        afdal(&self.ruqaa)
    }

    /// The best-matching installable voice pack.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::LaMutabaqa`] when every listed voice pack is
    /// incompatible.
    pub fn afdal_sawt(&self) -> NatijatMustawda<&MutabaqatRuqaa> {
        afdal(&self.aswat)
    }

    /// The best tier any listed patch reaches, for the library card's ring.
    #[must_use]
    pub fn tabaqat_ruqaa(&self) -> Option<MutabaqaBina> {
        afdal_mutabaqa(&self.ruqaa)
    }

    /// The same for voice packs.
    #[must_use]
    pub fn tabaqat_sawt(&self) -> Option<MutabaqaBina> {
        afdal_mutabaqa(&self.aswat)
    }
}

/// The installed build, and everything known about the packages judged against
/// it beyond what the index carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutabiqBina {
    bina: BinaId,
    idafat: BTreeMap<RuqaaId, IdafatIrtibat>,
}

impl MutabiqBina {
    /// A matcher for one installed build, judging from index listings alone.
    #[must_use]
    pub const fn jadeed(bina: BinaId) -> Self {
        Self { bina, idafat: BTreeMap::new() }
    }

    /// The same, with the bindings full metadata records added.
    #[must_use]
    pub const fn maa_idafat(bina: BinaId, idafat: BTreeMap<RuqaaId, IdafatIrtibat>) -> Self {
        Self { bina, idafat }
    }

    /// The build every verdict is measured against.
    #[must_use]
    pub const fn bina(&self) -> &BinaId {
        &self.bina
    }

    /// Records what a full metadata record adds for one package, returning what
    /// it replaced.
    pub fn sajjil_idafa(&mut self, id: RuqaaId, idafa: IdafatIrtibat) -> Option<IdafatIrtibat> {
        self.idafat.insert(id, idafa)
    }

    /// What is known about one package beyond its listing.
    #[must_use]
    pub fn idafa(&self, id: RuqaaId) -> IdafatIrtibat {
        self.idafat.get(&id).copied().unwrap_or_default()
    }

    /// Judges one patch listing.
    #[must_use]
    pub fn ruqaa(&self, mulakhkhas: &MulakhkhasRuqaa) -> MutabaqatRuqaa {
        self.ihkum(
            mulakhkhas.id,
            mulakhkhas.murajaa,
            &mulakhkhas.bina_manassa,
            &mulakhkhas.basmat,
        )
    }

    /// Judges one voice pack listing.
    #[must_use]
    pub fn sawt(&self, mulakhkhas: &MulakhkhasSawt) -> MutabaqatRuqaa {
        self.ihkum(
            mulakhkhas.id,
            mulakhkhas.murajaa,
            &mulakhkhas.bina_manassa,
            &mulakhkhas.basmat,
        )
    }

    /// Judges every patch listing, keeping the incompatible ones and their
    /// reasons.
    #[must_use]
    pub fn ruqaa_kul(&self, mulakhkhasat: &[MulakhkhasRuqaa]) -> Vec<MutabaqatRuqaa> {
        mulakhkhasat.iter().map(|wahid| self.ruqaa(wahid)).collect()
    }

    /// Judges every voice pack listing, on the same terms.
    #[must_use]
    pub fn aswat_kul(&self, mulakhkhasat: &[MulakhkhasSawt]) -> Vec<MutabaqatRuqaa> {
        mulakhkhasat.iter().map(|wahid| self.sawt(wahid)).collect()
    }

    /// Judges everything one verified shard lists for one game.
    #[must_use]
    pub fn luba(&self, shareeha: &ShareehaMuwaththaqa, luba: LubaId) -> MutabaqatLuba {
        MutabaqatLuba {
            luba,
            ruqaa: self.ruqaa_kul(shareeha.ruqaa(luba)),
            aswat: self.aswat_kul(shareeha.aswat(luba)),
        }
    }

    fn ihkum(
        &self,
        id: RuqaaId,
        murajaa: RuqaaRevision,
        manassat: &[String],
        basmat: &[Basma],
    ) -> MutabaqatRuqaa {
        let idafa = self.idafa(id);
        let hukm = irtibat_min_qaima(manassat, basmat, idafa)
            .map_or(HUKM_MURFUD, |irtibat| irtibat.ihkum(&self.bina));
        let sabab_ghayr = (hukm.mutabaqa() == MutabaqaBina::Ghayr).then(|| SababGhayrTawafuq {
            bina_muallan: manassat.to_vec(),
            bina_mawjud: self.bina.manassa.clone(),
            adad_basmat: basmat.len(),
            basma_mawjuda: self.bina.basma.mukhtasara(),
            bila_irtibat: manassat.is_empty() && basmat.is_empty(),
            kharij_nitaq: idafa.nitaq.is_some(),
        });
        MutabaqatRuqaa { id, murajaa, hukm, sabab_ghayr }
    }
}

/// The packages a build can install.
pub fn mutawafiqa(ahkam: &[MutabaqatRuqaa]) -> impl Iterator<Item = &MutabaqatRuqaa> {
    ahkam.iter().filter(|hukm| hukm.qabila_lil_tathbeet())
}

/// The packages it cannot, each carrying the reason it does not apply.
pub fn ghayr_mutawafiqa(ahkam: &[MutabaqatRuqaa]) -> impl Iterator<Item = &MutabaqatRuqaa> {
    ahkam.iter().filter(|hukm| !hukm.qabila_lil_tathbeet())
}

/// The best-matching installable package: best tier, then newest revision.
///
/// # Errors
///
/// [`KhataMustawda::LaMutabaqa`] when every verdict is
/// [`MutabaqaBina::Ghayr`].
pub fn afdal(ahkam: &[MutabaqatRuqaa]) -> NatijatMustawda<&MutabaqatRuqaa> {
    mutawafiqa(ahkam)
        .min_by_key(|hukm| (hukm.mutabaqa(), Reverse(hukm.murajaa)))
        .ok_or(KhataMustawda::LaMutabaqa)
}

/// The best tier any package reaches, for a badge that has no package to name.
#[must_use]
pub fn afdal_mutabaqa(ahkam: &[MutabaqatRuqaa]) -> Option<MutabaqaBina> {
    ahkam.iter().map(MutabaqatRuqaa::mutabaqa).min()
}

/// Builds the binding an index listing implies, for [`IrtibatBina::ihkum`].
fn irtibat_min_qaima(
    manassat: &[String],
    basmat: &[Basma],
    idafa: IdafatIrtibat,
) -> Option<IrtibatBina> {
    // `ihkum` reads neither the recipe nor the fonts, and a listing carries neither.
    let mukhattat = MukhattatBasma::min_masarat(std::iter::empty::<&str>()).ok()?;
    Some(IrtibatBina {
        manassat: manassat.to_vec(),
        basmat: basmat.to_vec(),
        adad_malaffat: idafa.adad_malaffat,
        mukhattat,
        nitaq: idafa.nitaq,
        khutut: Vec::new(),
    })
}
