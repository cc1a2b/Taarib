//! الفحوصات الصارمة — the four checks a package cannot be produced past.
//!
//! Phase 18 runs these on submission. Running them here as well is not
//! duplication: a contributor who learns at submission that their patch has a
//! broken placeholder has already spent the evening building it, and the fix
//! is in the workshop they just closed.
//!
//! ## Why this is a token and not a function that returns a boolean
//!
//! The obvious shape is `if !fuhusat_najahat() { return }` somewhere in the
//! assembler. That shape has a defect no amount of care removes: it is one
//! forgotten call away from a package built past a failing check, and the
//! forgotten call looks like nothing. It is not a missing line that a reviewer
//! notices; it is an absence.
//!
//! So the checks mint [`IjtiyazFuhus`], the assembler takes one **by value**,
//! and there is no other way to produce it. A future edit that wants to build a
//! package without checking it has to add a constructor in this file, whose
//! entire subject is why that must not happen. This is the same construction
//! [`crate::bawwaba`] uses for provenance and Phase 13 uses for approval, for
//! the same reason: a rule a future edit can break by accident is not a rule.
//!
//! ## The two placeholder checks, and why the union is deliberate
//!
//! Phase 13's protection module refuses a translation whose placeholders came
//! back damaged, and records [`AlamJawda::NasqMaksur`] on the string. That flag
//! is authoritative for anything that went through a provider, and
//! `taarib_tarjama::alamat` is explicit that recomputing it by re-scanning text
//! would be a second verifier that disagrees with the first.
//!
//! It says nothing at all about a translation somebody typed into the workshop
//! by hand, because no protection pass ever ran on one. For those, this module
//! compares the *recorded span tables* — the atoms extraction found in the
//! source against the atoms recorded over the target. That is not a re-scan of
//! text; it is a comparison of two structured tables the project already holds,
//! and it covers exactly the origin the flag cannot.
//!
//! The two never contradict each other because they never look at the same
//! string twice for the same reason. Their union is what "no broken placeholder
//! ships" actually requires.

use std::collections::BTreeMap;

use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NawNasq, NitaqNasq};
use taarib_mustalahat::ruqaa::{RukhsaRuqaa, TareeqaTarjama};

use crate::khata::KhataTarqee;

/// How many offending strings a refusal names before it stops.
///
/// A message listing four thousand identities is a message nobody reads. Six is
/// enough to see a pattern — one container, one translator, one bad import —
/// and the count beside them is what says how big the problem is.
pub const AQSA_AMTHILA: usize = 6;

/// Proof that every hard check ran and passed.
///
/// **No public constructor, no public fields, no `Deserialize`, not `Clone`.**
///
/// The `Deserialize` matters as much as the constructor: a token that could be
/// minted from a stored document would let a project file assert that its own
/// checks passed. Not `Clone`, because one run of the checks certifies one set
/// of strings, and a copyable token would let a passing run escort a later,
/// edited set into a package.
#[derive(Debug)]
pub struct IjtiyazFuhus {
    nusus: usize,
    muakkada: usize,
}

impl IjtiyazFuhus {
    /// How many strings the checks covered.
    #[must_use]
    pub const fn nusus(&self) -> usize {
        self.nusus
    }

    /// How many of them were approved by a human review.
    #[must_use]
    pub const fn muakkada(&self) -> usize {
        self.muakkada
    }
}

/// A string that failed to lay out, as the precomputation stage reports it.
///
/// Passed in rather than recomputed, because laying a string out twice to find
/// out whether it lays out is both slow and a second implementation of the
/// question.
#[derive(Debug, Clone)]
pub struct FashalTakhtit {
    /// A short form of the string, for the message.
    pub nass: String,
    /// The size it was being laid out at, in pixels.
    pub hajm: f32,
    /// What the layout engine said.
    pub sabab: String,
}

/// What the contributor declares about a package.
///
/// Every field here is a thing a person states and no machine can infer. The
/// licence in particular: a patch whose licence is guessed is a patch whose
/// redistribution terms are guessed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WasfHuzma {
    /// The patch's title, as it appears on a listing.
    pub unwan: String,
    /// The contributor's display name.
    pub ism_musahim: String,
    /// The game's title.
    pub ism_luba: String,
    /// The licence, declared and never inferred.
    pub rukhsa: RukhsaRuqaa,
    /// How the translation was actually produced.
    pub tareeqa: TareeqaTarjama,
    /// The version of Taarib that built the package.
    pub isdar_taarib: String,
}

impl WasfHuzma {
    /// Checks that nothing required is blank.
    ///
    /// Blank rather than absent, because a `String` field is always present and
    /// an empty one is the shape a missing declaration actually takes. Trimmed
    /// before the test, so a title of three spaces fails the way a title of
    /// zero characters does.
    ///
    /// # Errors
    ///
    /// [`KhataTarqee::BayanNaqis`] naming the first blank field. One at a time
    /// rather than a list, because a contributor filling in a form fixes them
    /// one at a time and a list of six is not more actionable than the first.
    pub fn tahaqquq(&self) -> Result<(), KhataTarqee> {
        let matlub: [(&'static str, &str); 4] = [
            ("a title", &self.unwan),
            ("a contributor name", &self.ism_musahim),
            ("the game's title", &self.ism_luba),
            ("the Taarib version that built it", &self.isdar_taarib),
        ];
        for (haql, qeema) in matlub {
            if qeema.trim().is_empty() {
                return Err(KhataTarqee::BayanNaqis { haql });
            }
        }
        // A licence named `Ukhra` with an empty name is a licence field that was
        // filled in with nothing, which is worse than leaving it at a default:
        // it reads as a deliberate declaration on every listing that shows it.
        if let RukhsaRuqaa::Ukhra { ism } = &self.rukhsa
            && ism.trim().is_empty()
        {
            return Err(KhataTarqee::BayanNaqis {
                haql: "the name of the licence",
            });
        }
        Ok(())
    }
}

/// Everything the hard checks look at.
#[derive(Debug)]
pub struct MudkhalatFahs<'a> {
    /// The project's strings.
    pub madakhil: &'a [MudkhalNass],
    /// What the contributor declared.
    pub wasf: &'a WasfHuzma,
    /// Strings the precomputation stage could not lay out.
    ///
    /// Empty is the normal case, and it does **not** mean "checked and none
    /// failed". Precomputation runs inside [`crate::mujammi::ijmaa`], which
    /// needs the proof this module mints, so on a first compile nothing has
    /// been laid out yet and there is nothing to pass.
    ///
    /// The check is not skipped, only relocated:
    /// [`crate::takhtit::sabbiq`] raises [`KhataTarqee::TakhtitFashil`] itself
    /// the moment a string will not lay out, and it does so before the atlas is
    /// built and long before a byte is written. What this field is for is the
    /// caller that already has layout results in hand — the workshop's preview
    /// pass — and wants all four checks answered in one place before it starts
    /// a compile it knows will fail.
    pub takhtitat_fashila: &'a [FashalTakhtit],
}

/// Runs every hard check, and mints the proof only if all of them pass.
///
/// Order matters to the person reading the failure, not to correctness. The
/// declaration is checked first because it is the cheapest to fix and the most
/// likely to be missing on a first build; the approval check is last because it
/// is the one that should never fire.
///
/// # Errors
///
/// [`KhataTarqee::BayanNaqis`] when the declaration is incomplete,
/// [`KhataTarqee::NasqMaksur`] when any string's placeholders are damaged,
/// [`KhataTarqee::TakhtitFashil`] when any string would not lay out, and
/// [`KhataTarqee::IaatimadGhayrMashru`] when any string claims approval with no
/// human transition behind it.
pub fn ijri(mudkhalat: &MudkhalatFahs<'_>) -> Result<IjtiyazFuhus, KhataTarqee> {
    mudkhalat.wasf.tahaqquq()?;
    fahs_nasq(mudkhalat.madakhil)?;
    fahs_takhtit(mudkhalat.takhtitat_fashila)?;
    let muakkada = fahs_iaatimad(mudkhalat.madakhil)?;

    Ok(IjtiyazFuhus {
        nusus: mudkhalat.madakhil.len(),
        muakkada,
    })
}

/// The placeholder check: the recorded flag, and the recorded span tables.
fn fahs_nasq(madakhil: &[MudkhalNass]) -> Result<(), KhataTarqee> {
    let mut adad: usize = 0;
    let mut amthila: Vec<String> = Vec::new();

    for mudkhal in madakhil {
        // An untranslated string has no placeholders to have broken. Reporting
        // one would turn "this project is 40% done" into thousands of hard
        // failures and make the check useless on the day it matters most.
        let Some(hadaf) = mudkhal.hadaf.as_deref() else {
            continue;
        };
        if hadaf.is_empty() {
            continue;
        }

        let mublagh = mudkhal
            .alamat
            .iter()
            .any(|alam| matches!(alam, AlamJawda::NasqMaksur { .. }));
        let mafqud = if mublagh {
            Vec::new()
        } else {
            faraq_dharrat(mudkhal)
        };

        if !mublagh && mafqud.is_empty() {
            continue;
        }
        adad = adad.saturating_add(1);
        if amthila.len() < AQSA_AMTHILA {
            let wasf = if mublagh {
                format!(
                    "{}: the protection pass refused this translation",
                    mudkhal.id
                )
            } else {
                format!("{}: {}", mudkhal.id, mafqud.join(", "))
            };
            amthila.push(wasf);
        }
    }

    if adad == 0 {
        Ok(())
    } else {
        Err(KhataTarqee::NasqMaksur { adad, amthila })
    }
}

/// The atoms present in the source but not in the target, and the reverse.
///
/// Compared as a multiset rather than a set. A source reading `{0} of {1}` and
/// a translation reading `{0} of {0}` have identical atom *sets* and would pass
/// a set comparison, while the game's formatter is handed one argument it never
/// consumes and a second index it uses twice — which is exactly the class of
/// defect this check exists for.
///
/// Only placeholder atoms are compared. Style spans — bold, colour, italics —
/// legitimately differ in count between a source and its translation, because a
/// translator emphasises a different word than the original did, and treating
/// that as a hard failure would block correct work.
fn faraq_dharrat(mudkhal: &MudkhalNass) -> Vec<String> {
    let masdar = adud_dharrat(&mudkhal.nasq_masdar);
    let hadaf = adud_dharrat(&mudkhal.nasq_hadaf);
    if masdar == hadaf {
        return Vec::new();
    }

    let mut mafqud: Vec<String> = Vec::new();
    for (dharra, adad_masdar) in &masdar {
        let adad_hadaf = hadaf.get(dharra).copied().unwrap_or(0);
        if adad_hadaf != *adad_masdar {
            mafqud.push(format!(
                "{dharra}: {adad_masdar} in the source, {adad_hadaf} in the translation"
            ));
        }
    }
    for (dharra, adad_hadaf) in &hadaf {
        if !masdar.contains_key(dharra) {
            mafqud.push(format!(
                "{dharra}: absent from the source, {adad_hadaf} in the \
                                 translation"
            ));
        }
    }
    mafqud
}

/// One span table's placeholder atoms, counted.
fn adud_dharrat(nitaqat: &[NitaqNasq]) -> BTreeMap<String, usize> {
    let mut adad: BTreeMap<String, usize> = BTreeMap::new();
    for nitaq in nitaqat {
        let dharra = match &nitaq.naw {
            NawNasq::Mawdi { khaam } => khaam.clone(),
            NawNasq::Sura { marja } => marja.clone(),
            // Style spans are not atoms. See `faraq_dharrat`.
            _ => continue,
        };
        let khana = adad.entry(dharra).or_insert(0);
        *khana = khana.saturating_add(1);
    }
    adad
}

/// The layout check.
fn fahs_takhtit(fashila: &[FashalTakhtit]) -> Result<(), KhataTarqee> {
    let Some(awwal) = fashila.first() else {
        return Ok(());
    };
    Err(KhataTarqee::TakhtitFashil {
        nass: awwal.nass.clone(),
        hajm: awwal.hajm,
        sabab: if fashila.len() > 1 {
            let baqi = fashila.len().saturating_sub(1);
            format!(
                "{} ({baqi} more string(s) also failed to lay out)",
                awwal.sabab
            )
        } else {
            awwal.sabab.clone()
        },
    })
}

/// The approval check.
///
/// [`taarib_mustalahat::muraja::SijillMuraja`] has private fields and no way to
/// reach [`HalatMuraja::Muakkada`] except through an attestation a human review
/// constructs — but it derives `Deserialize`, and a project file is a document
/// on disk. Somebody who edits `"hala": "muakkada"` into it, or an importer
/// written against an older shape, produces a record whose state says approved
/// and whose history says nobody was ever there.
///
/// So the check is not "is the state approved" — the type already governs that
/// within this process. It is "does the history contain a transition **into**
/// approval that a person made". A record that claims the state without the
/// transition is a record no run of this product could have produced.
///
/// Returns how many strings really are approved.
fn fahs_iaatimad(madakhil: &[MudkhalNass]) -> Result<usize, KhataTarqee> {
    let mut muakkada: usize = 0;
    let mut adad: usize = 0;
    let mut amthila: Vec<String> = Vec::new();

    for mudkhal in madakhil {
        if mudkhal.muraja.hala() != HalatMuraja::Muakkada {
            continue;
        }
        let mashru = mudkhal
            .muraja
            .tareekh()
            .iter()
            .any(|intiqal| intiqal.ila == HalatMuraja::Muakkada && intiqal.musahim.is_some());
        if mashru {
            muakkada = muakkada.saturating_add(1);
            continue;
        }
        adad = adad.saturating_add(1);
        if amthila.len() < AQSA_AMTHILA {
            amthila.push(mudkhal.id.to_string());
        }
    }

    if adad == 0 {
        Ok(muakkada)
    } else {
        Err(KhataTarqee::IaatimadGhayrMashru { adad, amthila })
    }
}
