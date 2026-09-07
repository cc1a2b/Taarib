//! المجمِّع — assembling a package, and the two things it will not move without.
//!
//! Everything upstream of here produces a report. This produces a file, and it
//! is the only function in the crate that does.
//!
//! ## The two proofs, and why they are arguments
//!
//! [`ijmaa`] takes an [`IjtiyazFuhus`] **by value**, and the atlas reaches it
//! only inside a [`crate::bawwaba::SafhatMasmuha`], also by value. Neither has
//! a public constructor, neither is [`Clone`], and neither can be
//! deserialized.
//!
//! The first means the hard checks ran and passed; the second means every byte
//! of the atlas was rasterized by Taarib from a font inside Taarib's own font
//! directory. A caller that wants to write a package without either has to
//! change this signature, which is a line in a diff. The alternative — a check
//! at the top of this function — is one deleted line away from a package built
//! past a failing check, and a deleted line that leaves no trace is not
//! something a review catches.
//!
//! Both are consumed rather than borrowed, so one set of checks writes one
//! package and one rasterization escorts one atlas. A borrowed proof could be
//! handed to a loop.
//!
//! ## Only translated strings go in
//!
//! A string with no translation has nothing to contribute: the container holds
//! source-and-target pairs, and a pair whose target is absent is a row an
//! adapter would look up, find, and draw the English from. Untranslated strings
//! are counted in the coverage report, which is where the fact belongs.
//!
//! An entry whose translation is present but empty is treated the same way, and
//! not because empty text is harmless — it is a defect, `AlamJawda::Farigh`
//! names it, and Phase 13 raises it. It is excluded here because the container
//! would otherwise carry a row that replaces a menu label with nothing, which
//! looks to a player like a patch that deleted the interface.
//!
//! ## One source string, one translation — decided here, not refused
//!
//! The container is keyed by the *clean* source text, so it holds exactly one
//! translation per source string and `taarib_ruqaa::katib::Katib::nass` refuses
//! a second that disagrees. That refusal is right for a writer and was wrong as
//! the product's answer, because a project reaches it with disagreements
//! routinely and through no fault of anybody's:
//!
//! - a project's rows are per container and per path, so a menu label like
//!   `Back` is dozens of separate rows;
//! - the extractor's duplicate group ([`taarib_mustalahat::nass::MudkhalNass`]'s
//!   `majmua`) is keyed on the **raw** text, so `Back` and `<b>Back</b>` are two
//!   groups that collapse onto one container row once markup is lifted out;
//! - and nothing in translation is group-aware — every row goes to a provider on
//!   its own, and a provider asked the same question twice may answer it twice.
//!
//! So a real 3 718-string table died on a translation that was 32 bytes where
//! the row already held one of 31. Refusing the whole compile taught the
//! contributor nothing they could act on and produced no patch.
//!
//! [`wahhid_tarajim`] resolves it instead, before anything downstream can see
//! the disagreement. Every entry sharing a clean source is given the same
//! translation *and the same target spans*, so the row the writer stores, the
//! layouts precomputation produces and the certificate's count all describe one
//! text. What was set aside is reported rather than dropped silently:
//! [`HuzmaMabniya::tawhid`] names every source string that disagreed, what
//! shipped and what did not.
//!
//! ## The compiler reads back what it wrote
//!
//! The last thing this function does is open its own output through
//! [`taarib_ruqaa::qari::Ruqaa`] and check that every table holds what was put
//! in it. That is not defensive noise: the writer and the reader are two
//! implementations of one format, the failure mode of a disagreement between
//! them is a patch that installs and does nothing, and the machine that built
//! it is the only machine where finding out is free.
//!
//! It is also the check that catches a *silent* truncation — a table that
//! wrote nine thousand rows and read back eight thousand — which no amount of
//! care in the writer prevents, because the writer's own count is the number
//! that would be wrong.

use std::borrow::Cow;
use std::cmp::{Ordering, Reverse};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use taarib_lawha::khareeta::NamatSafha;
use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::nass::{MudkhalNass, NassId};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_ruqaa::aqsam::NawQism;
use taarib_ruqaa::jadawil::{ALAM_KHATT_ASASI, ALAM_KHATT_IHTIYATI, SijillNitaq, SijillQayd};
use taarib_ruqaa::katib::{HuwiyatNass, Katib, KhattMabni, TakhtitMabni};
use taarib_ruqaa::khata::KhataRuqaa;
use taarib_ruqaa::muhadhah::BaytMuhadhah;
use taarib_ruqaa::qari::{self, Ruqaa};
use taarib_ruqaa::tarwisa::ISDAR_SIYAGHA;
use taarib_ruqaa::tawqee::{KutlatTawqee, MudaqqiqTawqee};
use taarib_saff::khatt::SilsilatKhutut;

use crate::bawwaba::{KhattMujammaa, MuhtawaMasmuh, SafhatMasmuha, ShahadatBawwaba};
use crate::bayan::{BayanHuzma, MUKHATTAT_BAYAN, MuharrikHuzma, SijillFuhus};
use crate::fuhusat::{AQSA_AMTHILA, IjtiyazFuhus, WasfHuzma};
use crate::irtibat::IrtibatBina;
use crate::khata::KhataTarqee;
use crate::taghtiya_ruqaa::TaqrirTaghtiya;
use crate::tahdid_maqasat::TaqreerMaqasat;
use crate::tahweel::{self, SiyasatHuzma};
use crate::takhtit::KhiyaratTasbeeq;

/// Everything the assembler needs that is not a proof.
///
/// A struct rather than fifteen parameters, and every field a borrow, because
/// the two things that are *not* borrowed here are exactly the two proofs — and
/// that asymmetry is the point. Reading this signature should make it obvious
/// which arguments are evidence and which are data.
#[derive(Debug)]
pub struct MudkhalatTajmee<'a> {
    /// The project's strings.
    pub nusus: &'a [MudkhalNass],
    /// The patch's identity, stable across revisions.
    pub id: RuqaaId,
    /// Which revision this is.
    pub murajaa: RuqaaRevision,
    /// What the contributor declared.
    pub wasf: &'a WasfHuzma,
    /// Which engine, backend and support tier.
    pub muharrik: MuharrikHuzma,
    /// What builds and fonts the patch binds to.
    pub irtibat: &'a IrtibatBina,
    /// Which sizes were discovered.
    pub maqasat: &'a TaqreerMaqasat,
    /// Coverage, measured.
    ///
    /// The overflow report is deliberately *not* beside it. It is produced by
    /// precomputation from the layouts that ship, and a field here would be a
    /// field a caller could fill with an empty report — which is what every
    /// caller did.
    pub taghtiya: &'a TaqrirTaghtiya,
    /// The options precomputation ran under, so the constraint records carry
    /// the same decisions the layouts were made under.
    pub khiyarat: &'a KhiyaratTasbeeq,
    /// The compression level, or [`None`] for the writer's own.
    pub mustawa: Option<i32>,
}

/// One source string whose entries did not agree on a translation, and what the
/// package shipped for it.
///
/// Kept as evidence rather than a warning string, because the contributor's next
/// action is to open the losing rows in the workshop and decide whether the
/// choice this compile made is the one they want — and a sentence in a log
/// cannot be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TawhidTarjama {
    /// The clean source text, as the container keys it.
    pub masdar: String,
    /// The translation the package holds.
    pub mukhtara: String,
    /// How many entries already carried it.
    pub muwafiqa: usize,
    /// The translations that were set aside, each with how many entries carried
    /// it, ordered by the text so two runs report them identically.
    pub matruka: Vec<(String, usize)>,
}

impl TawhidTarjama {
    /// The line a compile report shows for this string.
    #[must_use]
    pub fn wasf(&self) -> String {
        let matruka: Vec<String> = self
            .matruka
            .iter()
            .map(|(nass, adad)| format!("{nass:?} ({adad})"))
            .collect();
        format!(
            "{:?}: shipped {:?} ({} entries); set aside {}",
            self.masdar,
            self.mukhtara,
            self.muwafiqa,
            matruka.join(", ")
        )
    }
}

/// A finished package, and what it says about itself.
///
/// The manifest is returned alongside the bytes rather than left to be parsed
/// back out of them, because the caller is about to show it to a contributor
/// and re-reading it would mean decompressing a section this process wrote
/// thirty microseconds ago.
#[derive(Debug)]
pub struct HuzmaMabniya {
    /// The container, unsigned. `taarib-khatm` seals it.
    pub bayt: BaytMuhadhah,
    /// What it declares.
    pub bayan: BayanHuzma,
    /// Every source string whose entries disagreed on a translation, and what
    /// shipped for it. Empty on a project that does not repeat itself.
    ///
    /// Deliberately outside [`BayanHuzma`]: this describes the *project* the
    /// package was built from, not the package, and a player who installs the
    /// patch has nothing to do with it. The contributor is shown it once, on
    /// the build that made the choice.
    pub tawhid: Vec<TawhidTarjama>,
}

impl HuzmaMabniya {
    /// The package's size on disk, in bytes.
    #[must_use]
    pub fn hajm(&self) -> usize {
        self.bayt.bayt().len()
    }

    /// The sentence the compile report ends with.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!("{} — {} byte(s), unsigned", self.bayan.wasf(), self.hajm())
    }

    /// Fills the signature reservation with the contributor's own block.
    ///
    /// **The key is not here and never passes through this crate.** The caller
    /// supplies a block that has already been produced, and a verifier that can
    /// check it. `taarib_ruqaa::katib::khatm` refuses a block that does not
    /// commit to *this* container's content hash before writing a byte of it,
    /// so a block pasted from another patch fails on the machine that holds the
    /// key rather than on a player's.
    ///
    /// A patch is self-signed by its contributor here so that Phase 18's
    /// pre-flight gate can prove a submission arrived intact from the person
    /// who says they made it. The owner's signature is added at approval, by
    /// the owner, and never by this function — there is one signing chokepoint
    /// in the product and a compiler is not it.
    ///
    /// Nothing outside the reservation is touched, so the hash the block
    /// commits to is still the hash of what is there when this returns.
    ///
    /// # Errors
    ///
    /// [`KhataTarqee::KhatmFashil`] when the block does not seal this
    /// container, or when the container's own stored hash is not the hash of
    /// its content.
    pub fn akhtim(
        &mut self,
        kutla: &KutlatTawqee,
        mudaqqiq: &dyn MudaqqiqTawqee,
    ) -> Result<(), KhataTarqee> {
        taarib_ruqaa::katib::khatm(self.bayt.bayt_mut(), kutla, mudaqqiq).map_err(|khata| {
            KhataTarqee::KhatmFashil { sabab: khata.to_string() }
        })
    }
}

/// Compiles a project into a `.ruqaa`.
///
/// The order is forced by the format and is not a preference. Strings are added
/// first, because every other table is keyed by a string's handle and the
/// handles do not exist until then. Precomputation runs second, because it
/// needs the handles. The atlas is built inside precomputation, because the
/// glyph set is whatever shaping turned out to produce, and so is the overflow
/// report, because it is measured from those same layouts. The manifest is
/// last, because it reports on all of it.
///
/// Two entries that disagree about how one source string is translated are
/// resolved by [`wahhid_tarajim`] before the writer sees either of them, and
/// reported on [`HuzmaMabniya::tawhid`]. They are not an error.
///
/// # Errors
///
/// [`KhataTarqee::KitabatHuzmaFashila`] for anything the container writer
/// refuses — a table past what a `u32` index can name, a section past the
/// reader's ceiling.
///
/// [`KhataTarqee::BayanNaqis`] when the manifest will not serialize, which in
/// practice means a non-finite float reached a report field.
///
/// [`KhataTarqee::DawraGhayrMutabaqa`] when the finished package does not read
/// back as what was written.
///
/// Whatever [`crate::takhtit::sabbiq`] refuses: a string that will not lay out,
/// an atlas that will not build.
pub fn ijmaa(
    mudkhalat: &MudkhalatTajmee<'_>,
    ijtiyaz: IjtiyazFuhus,
    silsila: &SilsilatKhutut,
    khutut: &[KhattMujammaa],
) -> Result<HuzmaMabniya, KhataTarqee> {
    let mut katib = Katib::jadeed();
    if let Some(mustawa) = mudkhalat.mustawa {
        katib = katib.bi_mustawa(mustawa);
    }
    katib = match mudkhalat.khiyarat.namat {
        NamatSafha::Taghtiya => katib.bi_taghtiya(),
        NamatSafha::Masafa => katib.bi_masafa(),
    };
    // Declared from the data rather than from a caller's flag: the container's
    // "constraints were informed by runtime capture" bit is true exactly when
    // some string's provenance says a capture session contributed to it.
    if mudkhalat.nusus.iter().any(|mudkhal| mudkhal.masdar_istikhraj.multaqat()) {
        katib = katib.bi_iltiqat();
    }

    // Before anything: one translation per source string. Every stage below
    // reads this slice and not the caller's, so no stage has to know that the
    // project disagreed with itself.
    let (nusus, tawhid) = wahhid_tarajim(mudkhalat.nusus);
    if !tawhid.is_empty() {
        let amthila: Vec<String> =
            tawhid.iter().take(AQSA_AMTHILA).map(TawhidTarjama::wasf).collect();
        tracing::warn!(
            adad = tawhid.len(),
            ?amthila,
            "source strings whose entries disagreed on a translation were unified; the \
             container holds one row per source string and cannot carry both"
        );
    }

    let huwiyat = adif_nusus(&mut katib, &nusus, mudkhalat.khiyarat)?;
    let musbaq = crate::takhtit::sabbiq(
        &nusus,
        &huwiyat,
        mudkhalat.maqasat,
        silsila,
        khutut,
        mudkhalat.khiyarat,
    )?;

    let iqama = musbaq.iqama().clone();
    let taqreer = musbaq.taqreer().clone();
    let (takhtitat, safahat, khutut_mabniya, tajawuz) = musbaq.ikhrij();
    let muhtawa = ijma_muhtawa(&nusus, &huwiyat, takhtitat, safahat, khutut_mabniya);
    let bawwaba = ShahadatBawwaba::min_muhtawa(&muhtawa);
    uktub_muhtawa(&mut katib, muhtawa);

    let bayan = BayanHuzma {
        mukhattat: MUKHATTAT_BAYAN,
        isdar_siyagha: ISDAR_SIYAGHA,
        id: mudkhalat.id,
        murajaa: mudkhalat.murajaa,
        wasf: mudkhalat.wasf.clone(),
        muharrik: mudkhalat.muharrik,
        irtibat: mudkhalat.irtibat.clone(),
        bawwaba,
        fuhus: SijillFuhus::from(ijtiyaz),
        maqasat: mudkhalat.maqasat.clone(),
        takhtit: taqreer,
        iqama,
        taghtiya: mudkhalat.taghtiya.clone(),
        tajawuz: tajawuz.lil_huzma(),
        siyasa: SiyasatHuzma::min_khiyarat(&mudkhalat.khiyarat.takhtit),
    };

    let jeyson = bayan.ila_bayt().map_err(|_| KhataTarqee::BayanNaqis {
        haql: "a manifest every report can be serialized into",
    })?;
    let _ = katib.bayan(&jeyson);

    let bayt = katib.ikhtim().map_err(|khata| khata_katib(&khata))?;
    tahaqquq_dawra(&bayt, &bayan)?;
    Ok(HuzmaMabniya { bayt, bayan, tawhid })
}

/// Gives every entry sharing a clean source text the same translation and the
/// same target spans.
///
/// ## Why the whole entry is rewritten and not just the writer's argument
///
/// Three stages downstream read a translation, and all three key it by the same
/// handle: the container row, the precomputed layouts, and the certificate's
/// content list. Resolving the disagreement only where the row is written would
/// leave precomputation laying out text that is not in the package — a layout
/// stored under the winning string's handle whose glyphs spell the losing one.
/// The spans travel with the text for the same reason: a span table is a set of
/// byte offsets into a *particular* string, and offsets from one translation
/// applied to another cut it in the wrong places or run off its end.
///
/// ## Which translation wins
///
/// In order, and every step of it decided by the data rather than by the order
/// the caller happened to hand the entries over:
///
/// 1. **A translation a human approved beats one nobody did.** Somebody looked
///    at this string and said yes; a provider's second answer did not.
/// 2. **Then the reading the most entries already carry.** If eleven menus say
///    one thing and one says another, the player sees the eleven.
/// 3. **Then the lowest identity among the entries carrying it.** The same
///    stable tiebreak the extractor uses to pick a duplicate group's leader —
///    arbitrary, and the part that matters, unchanged when the same build is
///    extracted again.
///
/// Returns the caller's own slice untouched when the project agrees with
/// itself, which is the common case and the one that must not pay for this.
fn wahhid_tarajim(nusus: &[MudkhalNass]) -> (Cow<'_, [MudkhalNass]>, Vec<TawhidTarjama>) {
    let mut murashahat: BTreeMap<&str, BTreeMap<&str, MurashahTarjama<'_>>> = BTreeMap::new();
    for mudkhal in nusus {
        // The same two skips `adif_nusus` applies. An entry that contributes no
        // row cannot contribute a vote on what that row says either.
        let Some(hadaf) = mudkhal.hadaf.as_deref() else { continue };
        if hadaf.trim().is_empty() {
            continue;
        }
        let khana = murashahat
            .entry(mudkhal.masdar.as_str())
            .or_default()
            .entry(hadaf)
            .or_insert(MurashahTarjama {
                sahib: mudkhal,
                adad: 0,
                muakkada: false,
                awwal: mudkhal.id,
            });
        khana.adad = khana.adad.saturating_add(1);
        khana.muakkada |= mudkhal.muraja.hala() == HalatMuraja::Muakkada;
        if mudkhal.id < khana.awwal {
            khana.awwal = mudkhal.id;
            khana.sahib = mudkhal;
        }
    }

    let mut fayizun: BTreeMap<&str, &MudkhalNass> = BTreeMap::new();
    let mut tawhid: Vec<TawhidTarjama> = Vec::new();
    for (masdar, khiyarat) in murashahat {
        // One candidate is agreement, not a disagreement, and the writer folds
        // it on its own.
        if khiyarat.len() < 2 {
            continue;
        }
        let Some((fayiz, sifat)) = khiyarat.iter().max_by_key(|(_, sifa)| sifa.rutba()) else {
            continue;
        };
        let matruka: Vec<(String, usize)> = khiyarat
            .iter()
            .filter(|(nass, _)| *nass != fayiz)
            .map(|(nass, sifa)| ((*nass).to_owned(), sifa.adad))
            .collect();
        tawhid.push(TawhidTarjama {
            masdar: masdar.to_owned(),
            mukhtara: (*fayiz).to_owned(),
            muwafiqa: sifat.adad,
            matruka,
        });
        let _ = fayizun.insert(masdar, sifat.sahib);
    }

    if fayizun.is_empty() {
        return (Cow::Borrowed(nusus), tawhid);
    }

    let mut muwahhada = nusus.to_vec();
    for mudkhal in &mut muwahhada {
        // An untranslated or blank entry never voted and must not acquire a
        // translation here: the package deliberately carries no row for it, and
        // giving it one would be this function inventing coverage.
        let Some(hadaf) = mudkhal.hadaf.as_deref() else { continue };
        if hadaf.trim().is_empty() {
            continue;
        }
        let Some(fayiz) = fayizun.get(mudkhal.masdar.as_str()) else { continue };
        if fayiz.hadaf.as_deref() == Some(hadaf) {
            continue;
        }
        mudkhal.hadaf.clone_from(&fayiz.hadaf);
        mudkhal.nasq_hadaf.clone_from(&fayiz.nasq_hadaf);
    }
    (Cow::Owned(muwahhada), tawhid)
}

/// One candidate translation of a source string, and the evidence for it.
#[derive(Debug)]
struct MurashahTarjama<'a> {
    /// The lowest-identity entry carrying it, which is where the text and the
    /// target spans are taken from if it wins.
    sahib: &'a MudkhalNass,
    /// How many entries carry it.
    adad: usize,
    /// Whether a human review approved any of them.
    muakkada: bool,
    /// The lowest identity among them.
    awwal: NassId,
}

impl MurashahTarjama<'_> {
    /// The candidate's rank, greatest wins. Total, because `awwal` is the lowest
    /// of a set of distinct entry identities and two candidates never share an
    /// entry — so no two candidates can tie on it.
    const fn rutba(&self) -> (bool, usize, Reverse<NassId>) {
        (self.muakkada, self.adad, Reverse(self.awwal))
    }
}

/// Adds every translated string, its style spans and its constraint.
///
/// Returns the handle map precomputation needs. Built here rather than by the
/// layout stage because a handle only exists once the string has been added,
/// and a stage that added strings itself could attach a layout to a string the
/// assembler never wrote.
fn adif_nusus(
    katib: &mut Katib,
    nusus: &[MudkhalNass],
    khiyarat: &KhiyaratTasbeeq,
) -> Result<BTreeMap<NassId, HuwiyatNass>, KhataTarqee> {
    let mut huwiyat: BTreeMap<NassId, HuwiyatNass> = BTreeMap::new();
    // One entry per handle, not per project entry. `Katib::nass` folds two
    // entries carrying the same source and the same translation onto one handle
    // — a real game says `Back` in a dozen menus — and everything hung off that
    // handle has to fold with it. Recorded straight through, one real game
    // produced eight identical spans for `{ping} ms`, and the constraint table
    // ended up holding several records under one index, where the reader
    // binary-searches and takes whichever it lands on.
    let mut nitaqat_mudmaja: BTreeMap<HuwiyatNass, Vec<SijillNitaq>> = BTreeMap::new();
    let mut quyud_mudmaja: BTreeMap<HuwiyatNass, SijillQayd> = BTreeMap::new();

    for mudkhal in nusus {
        let Some(hadaf) = mudkhal.hadaf.as_deref() else { continue };
        if hadaf.trim().is_empty() {
            continue;
        }

        let huwiya = katib.nass(&mudkhal.masdar, hadaf).map_err(|khata| khata_katib(&khata))?;
        let _ = huwiyat.insert(mudkhal.id, huwiya);

        // Spans are a function of the translated text alone, and every entry on
        // this handle carries the same translated text — `wahhid_tarajim` moved
        // the spans across with it. So the first writing is kept and the rest
        // are the same spans said again.
        let _ = nitaqat_mudmaja.entry(huwiya).or_insert_with(|| tahweel::nitaqat_hadaf(mudkhal));

        if tahweel::qayd_mufid(&mudkhal.quyud, mudkhal.tasnif) {
            let qayd = tahweel::qayd(&mudkhal.quyud, mudkhal.tasnif, &khiyarat.takhtit);
            match quyud_mudmaja.entry(huwiya) {
                Entry::Vacant(khali) => {
                    let _ = khali.insert(qayd);
                }
                // The tightest wins, taken whole. One string drawn in two
                // places has to fit in the narrower of them, and a record
                // assembled field-by-field from two sites would describe a
                // place the game does not draw — so this keeps whichever
                // constraint actually binds rather than merging them.
                Entry::Occupied(mut mawjud) => {
                    if aqyad_min(&qayd, mawjud.get()) {
                        mawjud.insert(qayd);
                    }
                }
            }
        }
    }

    for (huwiya, nitaqat) in nitaqat_mudmaja {
        for nitaq in nitaqat {
            let _ = katib.nitaq(huwiya, nitaq);
        }
    }
    for (huwiya, qayd) in quyud_mudmaja {
        let _ = katib.qayd(huwiya, qayd);
    }
    Ok(huwiyat)
}

/// Whether the first constraint binds more tightly than the second.
///
/// Width decides, then height, because a translated line overflows sideways
/// long before it overflows downward. A dimension recorded as zero or less is
/// unbounded, which is the loosest a constraint can be rather than the
/// tightest, so it never wins the comparison.
fn aqyad_min(awwal: &SijillQayd, thani: &SijillQayd) -> bool {
    // Ordered as a pair rather than compared field by field: two widths that
    // are equal to the last bit still have to fall through to the height, and
    // asking whether two measurements in pixels are exactly equal is the one
    // question about a float that is never worth asking.
    let mudda = |qayd: &SijillQayd| {
        let hadd = |qeema: f32| if qeema > 0.0 { qeema } else { f32::INFINITY };
        (hadd(qayd.ard_mutah), hadd(qayd.irtifa_mutah))
    };
    let (ard_awwal, irtifa_awwal) = mudda(awwal);
    let (ard_thani, irtifa_thani) = mudda(thani);
    match ard_awwal.partial_cmp(&ard_thani) {
        Some(Ordering::Less) => true,
        Some(Ordering::Greater) | None => false,
        Some(Ordering::Equal) => irtifa_awwal < irtifa_thani,
    }
}

/// Turns everything the compile produced into the one type a package accepts.
///
/// Nothing reaches the writer except through here, and nothing reaches here
/// except as a [`MuhtawaMasmuh`] — which has no variant holding an
/// unattributed byte vector. The certificate is then a count over this list
/// rather than an inspection of the finished file, which is the difference
/// between proving where bytes came from and guessing what they look like.
fn ijma_muhtawa(
    nusus: &[MudkhalNass],
    huwiyat: &BTreeMap<NassId, HuwiyatNass>,
    takhtitat: Vec<TakhtitMabni>,
    safahat: Option<SafhatMasmuha>,
    khutut: Vec<KhattMabni>,
) -> Vec<MuhtawaMasmuh> {
    let mut muhtawa: Vec<MuhtawaMasmuh> =
        Vec::with_capacity(nusus.len().saturating_add(takhtitat.len()));

    // Keyed by **handle**, not by string identity, and that distinction is the
    // whole of this loop.
    //
    // Two entries extracted from two places in a game can carry the same source
    // text — a game where forty menu items read "Back" is ordinary — and
    // `Katib::nass` folds them into one row and hands back one handle. Counting
    // one permitted item per *entry* would make the certificate claim forty
    // strings where the container holds one, and the round-trip check would
    // then fail on a project whose only unusual property is that it repeats
    // itself.
    //
    // First entry wins, which is the same rule the writer applies to the
    // translation it keeps.
    let mut farida: BTreeMap<HuwiyatNass, (&str, &str)> = BTreeMap::new();
    for mudkhal in nusus {
        let Some(huwiya) = huwiyat.get(&mudkhal.id) else { continue };
        let tarjama = mudkhal.hadaf.as_deref().unwrap_or_default();
        let _ = farida.entry(*huwiya).or_insert((mudkhal.masdar.as_str(), tarjama));
    }
    muhtawa.extend(farida.into_values().map(|(asl, tarjama)| MuhtawaMasmuh::Nass {
        asl: asl.to_owned(),
        tarjama: tarjama.to_owned(),
    }));
    // The same fold, for the same reason, one level down.
    //
    // Precomputation works per extracted entry, because that is where a
    // measured size lives — and two entries sharing a source therefore each
    // produce a layout, at whichever sizes each of them was observed at. The
    // container holds one layout per string per size and the writer refuses a
    // second, so the duplicates have to go.
    //
    // Keeping the first loses nothing: the two entries have the same text and
    // the same handle, so at a given size their layouts are the same glyphs in
    // the same places. What survives is the *union* of the sizes, which is
    // exactly right — a string seen at sixteen pixels in one menu and at
    // twenty-four in another is a string this patch should be able to draw at
    // both.
    let mut layouts: BTreeMap<(HuwiyatNass, u16), TakhtitMabni> = BTreeMap::new();
    for takhtit in takhtitat {
        let miftah = (takhtit.nass, takhtit.hajm_rubi);
        let _ = layouts.entry(miftah).or_insert(takhtit);
    }
    muhtawa.extend(layouts.into_values().map(MuhtawaMasmuh::Takhtit));
    if let Some(safahat) = safahat {
        muhtawa.push(MuhtawaMasmuh::Safahat(safahat));
    }
    muhtawa.extend(khutut.into_iter().map(MuhtawaMasmuh::Khatt));
    muhtawa
}

/// Writes permitted content into the container.
///
/// The strings were already added — they had to be, to have handles — so the
/// [`MuhtawaMasmuh::Nass`] arm writes nothing here. It is not dead: the
/// certificate counts it, and removing the variant from the list to save this
/// arm would make the certificate under-report what the package holds.
fn uktub_muhtawa(katib: &mut Katib, muhtawa: Vec<MuhtawaMasmuh>) {
    let mut khatt_asasi = true;
    for wahid in muhtawa {
        match wahid {
            MuhtawaMasmuh::Takhtit(takhtit) => {
                let _ = katib.takhtit(takhtit);
            }
            MuhtawaMasmuh::Safahat(safahat) => {
                let (safahat, khareeta, _) = safahat.ikhrij();
                for safha in safahat {
                    let _ = katib.safha(safha);
                }
                for (miftah, mawdi) in khareeta {
                    let _ = katib
                        .shakl(tahweel::miftah_shakl(miftah), tahweel::mawdi_shakl(&mawdi));
                }
            }
            MuhtawaMasmuh::Khatt(khatt) => {
                // The first font of the chain is the one shaping reaches for,
                // and every one after it is a fallback. The flags are derived
                // from chain position rather than declared, because position is
                // what `SijillHarf::khatt` indexes and a declaration that
                // disagreed with it would name a different font than the one
                // the glyph was rasterized from.
                let alam = if khatt_asasi { ALAM_KHATT_ASASI } else { ALAM_KHATT_IHTIYATI };
                khatt_asasi = false;
                let _ = katib.khatt(KhattMabni { alam: khatt.alam | alam, ..khatt });
            }
            // `Nass` was written before this loop, to have handles. A container
            // difference is applied to the user's own copy at install time and
            // is carried in the manifest, not in a section: the format's eight
            // section kinds are fixed and readable by struct overlay in five
            // languages, and adding a ninth for a variable-length instruction
            // list would break that for every one of them.
            MuhtawaMasmuh::Nass { .. } | MuhtawaMasmuh::Farq(_) => {}
        }
    }
}

/// Opens the finished package and checks it holds what was written.
///
/// [`Ruqaa::iftah`] already validates the framing and the content hash, so what
/// is left to check is the part a hash cannot: that the numbers in the tables
/// are the numbers the compiler believes it produced. A container whose hash is
/// correct over the wrong contents is exactly as broken as one whose hash is
/// wrong, and only this comparison notices.
fn tahaqquq_dawra(bayt: &BaytMuhadhah, bayan: &BayanHuzma) -> Result<(), KhataTarqee> {
    let khaam = bayt.bayt();
    let ruqaa = Ruqaa::iftah(khaam).map_err(|khata| KhataTarqee::DawraGhayrMutabaqa {
        sabab: format!("the package would not reopen: {khata}"),
    })?;

    let bayanat = ruqaa.bayan_json().map_err(|khata| KhataTarqee::DawraGhayrMutabaqa {
        sabab: format!("the metadata section did not read back as JSON: {khata}"),
    })?;
    if bayanat.get("mukhattat").and_then(serde_json::Value::as_u64)
        != Some(u64::from(bayan.mukhattat))
    {
        return Err(KhataTarqee::DawraGhayrMutabaqa {
            sabab: "the metadata section read back without its schema version".to_owned(),
        });
    }

    // Each reader is handed its own section, never the whole file. A reader
    // parses a preamble at the start of whatever it is given, and given the
    // package it parsed the file header instead: the section count landed in
    // the string table's offset field, every package failed its own round-trip
    // check, and this function had never once returned `Ok`. Going through the
    // package's accessor is also what decompresses, so a zstd-packed table is
    // read as its contents rather than as its wrapper.
    let qism_nusus = qism_min(&ruqaa, NawQism::Nusus)?;
    let nusus = qari::nusus(qism_nusus.bayt()).map_err(qism_fashil(NawQism::Nusus))?;
    qaran("string", nusus.sijillat.len(), bayan.bawwaba.nusus)?;

    if bayan.bawwaba.takhtitat > 0 {
        let qism_takhtit = qism_min(&ruqaa, NawQism::Takhtit)?;
        let takhtit =
            qari::takhtit(qism_takhtit.bayt()).map_err(qism_fashil(NawQism::Takhtit))?;
        qaran("precomputed layout", takhtit.ruus.len(), bayan.bawwaba.takhtitat)?;
    }

    if bayan.bawwaba.safahat > 0 {
        let qism_lawha = qism_min(&ruqaa, NawQism::Lawha)?;
        let lawha = qari::lawha(qism_lawha.bayt()).map_err(qism_fashil(NawQism::Lawha))?;
        qaran("atlas page", lawha.safahat.len(), bayan.bawwaba.safahat)?;
        let qism_khareeta = qism_min(&ruqaa, NawQism::Khareeta)?;
        let khareeta =
            qari::khareeta(qism_khareeta.bayt()).map_err(qism_fashil(NawQism::Khareeta))?;
        // The two arrays of the glyph map are written from one sorted list, so
        // a length disagreement between them means the section's own preamble
        // is wrong — and a reader that trusted it would pair each key with a
        // rectangle belonging to some other glyph.
        qaran("glyph position", khareeta.mawadi.len(), khareeta.mafatih.len())?;
    }

    if !bayan.irtibat.khutut.is_empty() {
        let qism_khatt = qism_min(&ruqaa, NawQism::Khatt)?;
        let khatt = qari::khatt(qism_khatt.bayt()).map_err(qism_fashil(NawQism::Khatt))?;
        qaran("bundled font", khatt.sijillat.len(), bayan.irtibat.khutut.len())?;
    }

    Ok(())
}

/// One section out of the reopened package, in the round-trip failure's own
/// wording when it is absent or will not decompress.
fn qism_min<'a>(
    ruqaa: &Ruqaa<'a>,
    naw: NawQism,
) -> Result<qari::BayanatQism<'a>, KhataTarqee> {
    ruqaa.qism(naw).map_err(qism_fashil(naw))
}

/// One count against another, as the round-trip check.
fn qaran(ism: &str, wujid: usize, muntazar: usize) -> Result<(), KhataTarqee> {
    if wujid == muntazar {
        return Ok(());
    }
    Err(KhataTarqee::DawraGhayrMutabaqa {
        sabab: format!("{muntazar} {ism}(s) were written and {wujid} read back"),
    })
}

/// The round-trip failure for a section that would not read at all.
fn qism_fashil(naw: NawQism) -> impl Fn(KhataRuqaa) -> KhataTarqee {
    move |khata| KhataTarqee::DawraGhayrMutabaqa {
        sabab: format!("the {} section did not read back: {khata}", naw.ism()),
    }
}

/// The container writer's refusals, as this crate's.
///
/// Flattened to one variant deliberately. Every one of them means the same
/// thing to a contributor — the package could not be written — and the writer's
/// own message already names the field, the table and the ceiling. Fanning them
/// out into distinct variants here would be a second taxonomy of one crate's
/// errors, kept in agreement by hand.
fn khata_katib(khata: &KhataRuqaa) -> KhataTarqee {
    KhataTarqee::KitabatHuzmaFashila { sabab: khata.to_string() }
}
