//! الجدول — the normalized string table, and the identity model everything
//! downstream depends on.
//!
//! Every extractor in this crate produces this. Phase 13 translates it, Phase 14
//! compiles it, and Phase 15 re-matches it against a newer build. It is the one
//! shape the five engine families agree on.
//!
//! ## Identity is the load-bearing decision
//!
//! A string's identity has to survive a game update that moved it. That rules
//! out the two obvious choices immediately:
//!
//! - **A byte offset** is invalidated by any store update. A patch keyed on
//!   offsets is a patch that breaks the first time the publisher ships a
//!   hotfix, and the whole re-matching promise depends on identity outliving
//!   exactly that event.
//! - **An array index** is worse, because it survives *syntactically*. A string
//!   inserted at position 40 renumbers everything after it, so the patch still
//!   applies and now writes the wrong Arabic into every subsequent line. A
//!   broken patch is recoverable; a silently misapplied one is not.
//!
//! So identity is derived from **content and structural position**:
//! [`NassId::min_mawqi`] over the container, the location expressed in the
//! engine's own structural terms, and the source text. A string that moved
//! inside its file keeps its identity as long as its text and its field did
//! not change, which is the property re-matching needs.
//!
//! ### Where the engine already has a stable key, that key wins
//!
//! Unreal's namespace-and-key, Unity Localization's table-and-entry id, Ren'Py's
//! statement identifier, Godot's message id — these are stable by the engine's
//! own contract, chosen by the developer, and they survive edits Taarib's
//! derivation would not. [`MawqiNass::miftah_muharrik`] carries them, and
//! [`MudkhalMustakhraj::huwiya`] uses one when present. That the engine supplied
//! the key is recorded, not silently absorbed, because a patch whose identities
//! came from the game's own localization system can be re-matched against a
//! build Taarib has never seen.
//!
//! ## Nothing is ever dropped
//!
//! Most strings in a game's data are not user-facing. The table keeps them
//! anyway, classified and scored, and the interface filters. See
//! [`taarib_mustalahat::nass::TasnifNass`] for the argument; the short version
//! is that silent dropping is how a game ships with one untranslated menu whose
//! source nobody can find.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use taarib_mustalahat::muraja::SijillMuraja;
use taarib_mustalahat::nass::{
    MasdarIstikhraj, MudkhalNass, NassId, NawNasq, NitaqNasq, QuyudNass, SiyaqNass, TasnifNass,
};

use crate::khata::KhataIstikhraj;

/// Where a string sits, in terms a contributor would recognize.
///
/// Structural rather than positional, and that is the whole point: every field
/// here survives the container being rewritten, whereas a byte offset does not.
/// `mawqi` is what [`NassId::min_mawqi`] hashes, so what goes in it decides
/// whether an identity is stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MawqiNass {
    /// The container, relative to the game's root: `Data/Map001.json`,
    /// `pakchunk0-WindowsNoEditor.pak`, `resources.assets`.
    pub hawiya: String,
    /// The asset inside it, where the container holds several.
    pub asl: Option<String>,
    /// The location inside that asset, in the engine's own structural terms.
    ///
    /// An object path and field for Unity, a namespace and key for Unreal, an
    /// event command's page and index for RPG Maker, a statement label for
    /// Ren'Py. **Never a byte offset and never a bare array index** — see this
    /// module's header for why the second one is the more dangerous of the two.
    pub mawqi: String,
    /// The field or property name, where the container has one.
    pub haql: Option<String>,
    /// The engine's own stable key for this string, when it has one.
    ///
    /// Present for a real localization system and absent for a string dug out
    /// of a serialized field. When present it *replaces* the derived identity
    /// rather than supplementing it, because the developer chose it and the
    /// developer will keep it stable across the builds Taarib has to re-match.
    pub miftah_muharrik: Option<String>,
}

impl MawqiNass {
    /// The identity this location and text produce.
    ///
    /// The engine's own key wins when there is one. The container is still
    /// mixed in, because two different Unreal `.pak` files can legitimately
    /// carry the same namespace and key for different builds of a shared asset,
    /// and collapsing those would give one identity to two strings.
    #[must_use]
    pub fn huwiya(&self, masdar: &str) -> NassId {
        match self.miftah_muharrik.as_deref() {
            Some(miftah) => NassId::min_mawqi(&self.hawiya, miftah, ""),
            None => NassId::min_mawqi(&self.hawiya, &self.mawqi_kamil(), masdar),
        }
    }

    /// Whether this identity came from the engine rather than from Taarib.
    #[must_use]
    pub const fn min_almuharrik(&self) -> bool {
        self.miftah_muharrik.is_some()
    }

    /// The location as one path, asset and field included.
    #[must_use]
    pub fn mawqi_kamil(&self) -> String {
        let mut kamil = String::with_capacity(self.mawqi.len().saturating_add(32));
        if let Some(asl) = self.asl.as_deref() {
            kamil.push_str(asl);
            kamil.push('\u{1}');
        }
        kamil.push_str(&self.mawqi);
        if let Some(haql) = self.haql.as_deref() {
            kamil.push('\u{1}');
            kamil.push_str(haql);
        }
        kamil
    }

    /// The sentence the workshop shows under a string.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut ajza = vec![self.hawiya.clone()];
        if let Some(asl) = self.asl.as_deref() {
            ajza.push(asl.to_owned());
        }
        ajza.push(self.mawqi.clone());
        if let Some(haql) = self.haql.as_deref() {
            ajza.push(haql.to_owned());
        }
        ajza.join(" › ")
    }
}

/// One extracted string, before it becomes a project row.
///
/// Carries what extraction knows and nothing about translation state. The
/// project turns this into a [`MudkhalNass`] and owns everything after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MudkhalMustakhraj {
    /// Where it came from.
    pub mawqi: MawqiNass,
    /// The source text exactly as stored, before markup is lifted.
    pub khaam: String,
    /// The clean text, with markup and placeholders lifted into spans.
    pub naqi: String,
    /// Those spans.
    pub nasq: Vec<NitaqNasq>,
    /// What kind of string this is.
    pub tasnif: TasnifNass,
    /// How sure the extractor is, zero to a hundred.
    pub thiqa: u8,
    /// How it reached the table.
    pub masdar: MasdarIstikhraj,
    /// The encoding the container declared, when it declared one.
    pub tarmiz: Option<String>,
    /// What the engine enforces and what runtime capture measured.
    pub quyud: QuyudNass,
    /// The speaker, for dialogue.
    pub mutakallim: Option<String>,
    /// The lines around it, which is what makes machine translation of dialogue
    /// worth anything.
    pub jiwar: Vec<String>,
    /// The scene it was captured in, when it was captured.
    pub mashhad: Option<String>,
}

impl MudkhalMustakhraj {
    /// This string's identity.
    #[must_use]
    pub fn huwiya(&self) -> NassId {
        self.mawqi.huwiya(&self.khaam)
    }

    /// Whether a player is believed to see this string.
    ///
    /// Runtime capture outranks everything: a string that was actually observed
    /// on screen *is* user-facing, whatever the classifier concluded from the
    /// field it came out of. That is the strongest signal available and it is
    /// the only one that is direct evidence rather than inference.
    #[must_use]
    pub const fn yaraha_allaib(&self) -> bool {
        if self.masdar.multaqat() {
            return true;
        }
        self.tasnif.yaraha_allaib()
    }

    /// Turns this into the project row the rest of the product works on.
    #[must_use]
    pub fn ila_mudkhal(&self) -> MudkhalNass {
        MudkhalNass {
            id: self.huwiya(),
            masdar: self.naqi.clone(),
            hadaf: None,
            muraja: SijillMuraja::jadeed(),
            siyaq: SiyaqNass {
                hawiya: self.mawqi.hawiya.clone(),
                mawqi: self.mawqi.mawqi_kamil(),
                haql: self.mawqi.haql.clone(),
                mutakallim: self.mutakallim.clone(),
                jiwar: self.jiwar.clone(),
                mashhad: self.mashhad.clone(),
                laqta: None,
            },
            quyud: self.quyud.clone(),
            nasq_masdar: self.nasq.clone(),
            nasq_hadaf: Vec::new(),
            takrar: 1,
            majmua: None,
            alamat: Vec::new(),
            tareeqa: None,
            muzawwid: None,
            muharrir: None,
            akhir_tabdeel: None,
            tasnif: self.tasnif,
            thiqat_tasnif: self.thiqa,
            masdar_istikhraj: self.masdar,
            tarmiz: self.tarmiz.clone(),
        }
    }
}

/// The normalized table.
///
/// Keyed by identity, so a second sighting of the same string folds into the
/// first rather than appearing twice. Insertion order is not preserved and does
/// not matter: the workshop sorts by whatever the translator asked for, and a
/// table that depended on extraction order would reorder itself whenever a
/// container was read in a different sequence.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JadwalNusus {
    madakhil: BTreeMap<NassId, MudkhalMustakhraj>,
    /// How many times each distinct source text occurs.
    ///
    /// Kept beside the entries rather than on them, because the count is a
    /// property of the *text* and several entries share one text. Writing it on
    /// each entry would be the same number stored fifty times and wrong the
    /// moment one of them changed.
    takrar: BTreeMap<String, u32>,
}

impl JadwalNusus {
    /// An empty table.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            madakhil: BTreeMap::new(),
            takrar: BTreeMap::new(),
        }
    }

    /// How many distinct strings the table holds.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// Whether anything was extracted at all.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.madakhil.is_empty()
    }

    /// Every entry, in identity order.
    pub fn madakhil(&self) -> impl Iterator<Item = &MudkhalMustakhraj> {
        self.madakhil.values()
    }

    /// One entry by identity.
    #[must_use]
    pub fn madkhal(&self, id: NassId) -> Option<&MudkhalMustakhraj> {
        self.madakhil.get(&id)
    }

    /// Adds a string, folding it into an existing entry when the identity
    /// already exists.
    ///
    /// Folding rather than replacing, and the direction matters: a later
    /// sighting contributes its *provenance* and its *measured constraints*,
    /// and never overwrites the source text. Static extraction read the bytes;
    /// runtime capture saw a rendering of them, which for a string with a
    /// substituted name is a different string. Letting capture overwrite would
    /// put `Hello, Alice` in the table as the source text of `Hello, {name}`.
    pub fn adif(&mut self, madkhal: MudkhalMustakhraj) {
        let id = madkhal.huwiya();
        let adad = self.takrar.entry(madkhal.khaam.clone()).or_insert(0);
        *adad = adad.saturating_add(1);

        match self.madakhil.get_mut(&id) {
            Some(mawjud) => {
                let muttafiq = mawjud.khaam == madkhal.khaam;
                mawjud.masdar = mawjud.masdar.adif(madkhal.masdar, muttafiq);
                dammij_quyud(&mut mawjud.quyud, &madkhal.quyud);

                // The stronger classification wins, and confidence decides
                // which is stronger. A capture that saw the string on screen
                // arrives with high confidence and correctly overrides a
                // static guess of "internal".
                if madkhal.thiqa > mawjud.thiqa {
                    mawjud.tasnif = madkhal.tasnif;
                    mawjud.thiqa = madkhal.thiqa;
                }
                if mawjud.mashhad.is_none() {
                    mawjud.mashhad = madkhal.mashhad;
                }
                if mawjud.mutakallim.is_none() {
                    mawjud.mutakallim = madkhal.mutakallim;
                }
                if mawjud.jiwar.is_empty() {
                    mawjud.jiwar = madkhal.jiwar;
                }
            },
            None => {
                let _ = self.madakhil.insert(id, madkhal);
            },
        }
    }

    /// How many times a source text occurs across the whole game.
    #[must_use]
    pub fn takrar(&self, khaam: &str) -> u32 {
        self.takrar.get(khaam).copied().unwrap_or(0)
    }

    /// How many entries fall in each classification.
    #[must_use]
    pub fn tawzee_tasnif(&self) -> BTreeMap<TasnifNass, usize> {
        let mut tawzee = BTreeMap::new();
        for madkhal in self.madakhil.values() {
            *tawzee.entry(madkhal.tasnif).or_insert(0_usize) += 1;
        }
        tawzee
    }

    /// How many entries a player is believed to see.
    #[must_use]
    pub fn adad_zahir(&self) -> usize {
        self.madakhil.values().filter(|q| q.yaraha_allaib()).count()
    }

    /// Turns the table into project rows, with duplicate groups resolved.
    ///
    /// Every entry sharing a source text is pointed at one group leader, so a
    /// translator writing the Arabic for `Yes` once serves all four hundred
    /// occurrences of it. The leader is the lowest identity in the group, which
    /// is arbitrary and — the part that matters — *stable*: re-extracting the
    /// same build picks the same leader, so a project does not reshuffle its
    /// own groups on every scan.
    #[must_use]
    pub fn ila_mudkhalat(&self) -> Vec<MudkhalNass> {
        let mut qada: BTreeMap<&str, NassId> = BTreeMap::new();
        for (id, madkhal) in &self.madakhil {
            let qaid = qada.entry(madkhal.khaam.as_str()).or_insert(*id);
            if *id < *qaid {
                *qaid = *id;
            }
        }

        self.madakhil
            .iter()
            .map(|(id, madkhal)| {
                let mut mudkhal = madkhal.ila_mudkhal();
                mudkhal.takrar = self.takrar(&madkhal.khaam);
                mudkhal.majmua = qada
                    .get(madkhal.khaam.as_str())
                    .copied()
                    .filter(|qaid| qaid != id);
                mudkhal
            })
            .collect()
    }

    /// Merges another table into this one.
    ///
    /// Used to fold a capture session into a static extraction. Every entry goes
    /// through [`JadwalNusus::adif`], so the provenance and constraint folding
    /// rules apply exactly once and in one place.
    pub fn dammij(&mut self, akhar: Self) {
        for madkhal in akhar.madakhil.into_values() {
            self.adif(madkhal);
        }
    }
}

/// Folds measured constraints from a second sighting into the first.
///
/// **The tightest observed value wins for widths**, because the constraint that
/// matters is the smallest box the string ever had to fit in. A string drawn
/// once in a wide panel and once in a narrow tooltip overflows in the tooltip,
/// and recording the panel's width would tell Phase 14 the translation fits when
/// it does not.
///
/// Character limits behave the same way. Font size and the rectangle take the
/// first measurement rather than the tightest — they describe an occurrence, not
/// a bound, and averaging or minimising them would produce a rectangle no
/// occurrence actually had.
const fn dammij_quyud(hadaf: &mut QuyudNass, min: &QuyudNass) {
    hadaf.aqsa_ahruf = asghar(hadaf.aqsa_ahruf, min.aqsa_ahruf);
    hadaf.aqsa_ard = asghar_ashri(hadaf.aqsa_ard, min.aqsa_ard);
    hadaf.aqsa_irtifa = asghar_ashri(hadaf.aqsa_irtifa, min.aqsa_irtifa);
    if hadaf.hajm_khatt.is_none() {
        hadaf.hajm_khatt = min.hajm_khatt;
    }
    if hadaf.mustatil.is_none() {
        hadaf.mustatil = min.mustatil;
    }
    hadaf.satr_wahid = hadaf.satr_wahid || min.satr_wahid;
}

/// The smaller of two optional bounds, treating absence as "no bound".
const fn asghar(awwal: Option<u32>, thani: Option<u32>) -> Option<u32> {
    match (awwal, thani) {
        (Some(a), Some(b)) => Some(if a < b { a } else { b }),
        (Some(a), None) => Some(a),
        (None, thani) => thani,
    }
}

/// The same for a measured width.
///
/// `f32::min` rather than a comparison, because it propagates correctly when a
/// measurement arrives as NaN — which a capture from a component with a
/// degenerate layout does produce — and `float_cmp` is denied for the good
/// reason that `<` on two NaNs answers nothing useful.
const fn asghar_ashri(awwal: Option<f32>, thani: Option<f32>) -> Option<f32> {
    match (awwal, thani) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, thani) => thani,
    }
}

/// What re-extracting against a newer build changed.
///
/// The whole point of stable identity, and the reason Phase 15 can promise that
/// a store update does not cost a translator their work.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FarqJadwal {
    /// Strings in the new build that were not in the old one.
    pub mudafa: Vec<NassId>,
    /// Strings in the old build that are gone.
    ///
    /// **Not deleted from the project.** Their translations are kept and marked
    /// orphaned, because a string that vanished from one build routinely returns
    /// in the next, and a translator who lost four hundred lines to a hotfix
    /// would not use the product again.
    pub mahdhufa: Vec<NassId>,
    /// Strings whose identity survived and whose source text changed.
    ///
    /// Impossible when the identity was derived by Taarib — the text is part of
    /// the hash — and entirely possible when the engine supplied the key, which
    /// is exactly what an engine key is for.
    pub mughayyara: Vec<NassId>,
    /// Strings that came through untouched, whose translations migrate
    /// automatically.
    pub thabita: Vec<NassId>,
}

impl FarqJadwal {
    /// Compares an old table against a new one.
    #[must_use]
    pub fn qarin(qadeem: &JadwalNusus, jadeed: &JadwalNusus) -> Self {
        let mut farq = Self::default();

        for (id, madkhal) in &jadeed.madakhil {
            match qadeem.madakhil.get(id) {
                Some(sabiq) if sabiq.khaam == madkhal.khaam => farq.thabita.push(*id),
                Some(_) => farq.mughayyara.push(*id),
                None => farq.mudafa.push(*id),
            }
        }
        for id in qadeem.madakhil.keys() {
            if !jadeed.madakhil.contains_key(id) {
                farq.mahdhufa.push(*id);
            }
        }
        farq
    }

    /// Whether anything changed at all.
    #[must_use]
    pub const fn bila_taghyeer(&self) -> bool {
        self.mudafa.is_empty() && self.mahdhufa.is_empty() && self.mughayyara.is_empty()
    }

    /// How many entries migrate without a translator touching them.
    #[must_use]
    pub const fn adad_muhajjar(&self) -> usize {
        self.thabita.len()
    }

    /// The sentence the update screen shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} unchanged and migrated automatically, {} new, {} changed, {} no longer in this \
             build (their translations are kept)",
            self.thabita.len(),
            self.mudafa.len(),
            self.mughayyara.len(),
            self.mahdhufa.len()
        )
    }
}

/// Lifts markup out of raw text into spans over clean text.
///
/// Delegates to `taarib_saff::nasq`, which is the one markup parser in this
/// product. A second one here would be a second opinion about what `{0}` means,
/// and the two would disagree the first time either learned a new dialect.
///
/// ## The two models are not the same shape, and the difference is deliberate
///
/// `saff` produces what the **layout engine** needs: a [`Uslub`] per span
/// carrying font, weight, slant, size, colour and an optional opaque atom with
/// its measured box. The vocabulary's [`NawNasq`] is what a **translator and a
/// verifier** need: which markup construct this was, and what its raw text was
/// so a translation can be checked for having kept it.
///
/// So this is a real translation between two models rather than a rename. A
/// single `saff` span carrying both italic and a size override becomes two
/// vocabulary spans, because the vocabulary names one construct per span; and
/// an atom's raw text comes from the atom table rather than from the span,
/// because `saff` replaced it in the clean text with a placeholder character
/// and only the atom table remembers what was there.
///
/// # Errors
///
/// [`KhataIstikhraj::NasqTalif`] when the markup is malformed in a way the
/// parser refuses — an unclosed placeholder, most often — naming the string so
/// a contributor can look at it.
pub fn irfa_nasq(khaam: &str) -> Result<(String, Vec<NitaqNasq>), KhataIstikhraj> {
    use taarib_saff::nasq::{KhiyaratNasq, istakhrij};

    let naqi =
        istakhrij(khaam, &KhiyaratNasq::default()).map_err(|khata| KhataIstikhraj::NasqTalif {
            nass: khaam.chars().take(64).collect(),
            sabab: khata.injilizi,
        })?;

    // The atom table first, keyed by the span that carries it. An atom's span
    // says only "this is opaque and this wide"; the raw text a translation is
    // checked against lives here.
    let mut dharrat: BTreeMap<u16, &taarib_saff::nasq::DharraMustakhraja> = BTreeMap::new();
    for dharra in &naqi.dharrat {
        let _ = dharrat.insert(dharra.nitaq, dharra);
    }

    let mut nitaqat = Vec::with_capacity(naqi.nitaqat.len());
    for nitaq in &naqi.nitaqat {
        let zakhrafa = naqi.zakhrafat_nitaq(nitaq.id);
        for naw in anwa_min_uslub(&nitaq.uslub, dharrat.get(&nitaq.id).copied(), zakhrafa) {
            nitaqat.push(NitaqNasq {
                id: nitaq.id,
                bidaya: nitaq.bidaya,
                tul: nitaq.tul,
                naw,
            });
        }
    }
    Ok((naqi.nass, nitaqat))
}

/// Every vocabulary span one layout span implies.
///
/// A list rather than a single value because the two models disagree about
/// cardinality: `saff` carries every property of a span in one [`Uslub`], and
/// the vocabulary names one construct per span. Bold-and-red is one span there
/// and two here.
///
/// An atom short-circuits the rest. A placeholder is opaque — it has no colour
/// and no slant that a translator can act on — and emitting style spans over it
/// would invite a verifier to check the styling of something whose whole
/// contract is that it is passed through untouched.
fn anwa_min_uslub(
    uslub: &taarib_saff::talab::Uslub,
    dharra: Option<&taarib_saff::nasq::DharraMustakhraja>,
    zakhrafa: Option<taarib_saff::nasq::Zakhrafa>,
) -> Vec<NawNasq> {
    use taarib_saff::nasq::{NawDharra, Zakhrafa};

    if let Some(dharra) = dharra {
        return vec![match dharra.naw {
            // A sprite is drawn by the engine and occupies width; a placeholder,
            // a variable and a command are all text the engine substitutes. The
            // vocabulary distinguishes only those two, because that is the
            // distinction the layout engine acts on.
            NawDharra::Sura => NawNasq::Sura {
                marja: dharra.khaam.clone(),
            },
            NawDharra::Mawdi | NawDharra::Mutaghayyir | NawDharra::Amr => NawNasq::Mawdi {
                khaam: dharra.khaam.clone(),
            },
        }];
    }

    let mut anwa = Vec::new();
    if uslub.maail {
        anwa.push(NawNasq::Maail);
    }
    if let Some(hajm) = uslub.hajm {
        anwa.push(NawNasq::Hajm { qeema: hajm });
    }
    if let Some(lawn) = uslub.lawn {
        anwa.push(NawNasq::Lawn {
            qeema: sittasi(lawn),
        });
    }
    if let Some(khatt) = uslub.khatt {
        // The chain index, not a family name: `saff` resolves families to chain
        // positions before layout and the name is gone by here. Recorded as the
        // index it is rather than invented as a name, so a writer putting this
        // back can look the index up in the same chain.
        anwa.push(NawNasq::Khatt {
            ism: format!("#{khatt}"),
        });
    }
    if let Some(wazn) = uslub.wazn {
        // A variable-font weight at or above semibold is what the markup
        // dialects mean by bold. Below it there is no vocabulary span to emit —
        // the weight is a layout property with no translator-facing meaning.
        if wazn >= 600 {
            anwa.push(NawNasq::Ghaliz);
        }
    }
    // A rule drawn over the run, which `Uslub` has no field for because it
    // changes no metric — so it travels beside the style rather than inside it.
    // Emitted last, after the properties that do change metrics, only so the
    // order a reader sees matches the order they are described above.
    match zakhrafa {
        Some(Zakhrafa::TahtKhat) => anwa.push(NawNasq::TahtKhat),
        Some(Zakhrafa::Shatb) => anwa.push(NawNasq::Shatb),
        None => {},
    }
    anwa
}

/// A colour as `#rrggbb` or `#rrggbbaa`.
///
/// The alpha channel is dropped when it is fully opaque, because that is how
/// every dialect writes it and a round trip that turned `#ff0000` into
/// `#ff0000ff` would fail a byte-faithful comparison in Phase 14.
fn sittasi(lawn: [u8; 4]) -> String {
    let [ahmar, akhdar, azraq, alfa] = lawn;
    if alfa == 0xFF {
        format!("#{ahmar:02x}{akhdar:02x}{azraq:02x}")
    } else {
        format!("#{ahmar:02x}{akhdar:02x}{azraq:02x}{alfa:02x}")
    }
}
