//! وصل الترجمة — the overlay's provider seam wired onto the workspace's own
//! translation pipeline.
//!
//! **Compiled only under the `tarjama` feature, and the payload does not turn it
//! on.** `taarib-tarjama` brings a Tokio runtime, `reqwest` over rustls, a
//! bundled `SQLite` and a keyring, and every one of those is correct in Studio and
//! wrong inside a `cdylib` loaded into somebody else's game. The default build
//! of this crate contains none of it and reaches a provider through
//! [`crate::mutarjim::MutarjimTabaqa`] instead. This module is what a caller that
//! *does* have a runtime hands over.
//!
//! ## The context is passed through, not rebuilt
//!
//! [`crate::mutarjim::TalabSatr::siyaq`] is already a
//! [`taarib_mustalahat::nass::SiyaqNass`], which is the field
//! `taarib_tarjama::siyaq::SiyaqTalab` carries and whose `jiwar` its prompt
//! builder writes into the message verbatim. So the conversion below moves it
//! rather than translating between two shapes — which is the whole reason the
//! overlay adopted that type instead of inventing a neighbouring-lines carrier
//! of its own.
//!
//! ## Why the classification is sent as a guess
//!
//! `thiqat_tasnif` is set below
//! [`taarib_tarjama::siyaq::AQALL_THIQAT_TASNIF`], which makes the prompt say
//! out loud that the classification may be wrong and pulls the register toward
//! the conservative one. That is the honest value here and it is not a hedge:
//! nothing extracted this string. A user drew a rectangle around part of a
//! screen and chose what they thought was in it, which is a weaker claim than a
//! Phase 12 extractor reading a dialogue table, and telling the model otherwise
//! would be asserting a confidence nothing measured.
//!
//! ## Why the box is sent
//!
//! The overlay knows something almost nothing else in this product knows: the
//! exact rectangle the translation has to fit into, because it is the rectangle
//! the original was read out of and the one the Arabic will be drawn back into.
//! It goes into [`taarib_mustalahat::nass::QuyudNass::mustatil`] and
//! `aqsa_ard`, with `satr_wahid` set — the tier draws one line per box, as
//! [`crate::talqeem`] lays out and refuses to wrap.
//!
//! ## Blocking is the point
//!
//! [`crate::mutarjim::MutarjimTabaqa::tarjim`] is synchronous because it is
//! called from the worker thread [`crate::qissa::KhaytQissa`] owns, which exists
//! to be blocked on. The runtime here is a current-thread one: a provider call
//! is one HTTP request and a multi-threaded scheduler for it would be a pool of
//! idle threads inside a process that is not ours.

use std::sync::Arc;

use taarib_mustalahat::nass::{Mustatil, QuyudNass, TasnifNass};
use taarib_tarjama::dhakira::ThiqatQira;
use taarib_tarjama::hima::ihmi;
use taarib_tarjama::mulahazat::{DhakiraMushtaraka, MulahazaTabaqa, RaddTabaqa};
use taarib_tarjama::muzawwidun::Muzawwid;
use taarib_tarjama::siyaq::{AQALL_THIQAT_TASNIF, SiyaqTalab};

use crate::khata::KhataTabaqa;
use crate::mutarjim::{
    DhakiraTabaqa, MutarjimTabaqa, QaydTabaqa as QaydTabaqaSajjil, RaddSatr,
    TalabDhakira as TalabDhakiraTabaqa, TalabSatr,
};

/// A workspace translation provider, behind the overlay's seam.
///
/// Owns its own current-thread runtime, so a caller does not have to be inside
/// one and the overlay's worker does not have to know that a runtime exists.
#[derive(Debug)]
pub struct MutarjimMuzawwid {
    muzawwid: Arc<dyn Muzawwid>,
    zaman: tokio::runtime::Runtime,
}

impl MutarjimMuzawwid {
    /// Wraps a provider, building the runtime it will be driven on.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the runtime cannot be built, which on
    /// a machine out of threads is a real answer — the overlay declines to
    /// translate and still draws, rather than taking the game down.
    pub fn jadeed(muzawwid: Arc<dyn Muzawwid>) -> Result<Self, KhataTabaqa> {
        let zaman = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_name("taarib-tabaqa-tarjama")
            .build()
            .map_err(|sabab| KhataTabaqa::MawridFashil {
                mawrid: "translation runtime",
                sabab: sabab.to_string(),
            })?;
        Ok(Self { muzawwid, zaman })
    }

    /// The provider underneath, for a caller reading its cost counters.
    #[must_use]
    pub fn muzawwid(&self) -> &dyn Muzawwid {
        self.muzawwid.as_ref()
    }
}

impl MutarjimTabaqa for MutarjimMuzawwid {
    fn ism(&self) -> &str {
        self.muzawwid.ism()
    }

    fn mutah(&self) -> bool {
        // A provider that has spent its confirmed allowance refuses every
        // further request at the meter, before anything is sent. Asking it four
        // times a second for the rest of a three-hour session would be asking a
        // question whose answer is already known — so it is asked here once per
        // settled line instead.
        //
        // A ceiling of zero is a *free* provider, not an exhausted one: that is
        // what `TakalifJarya::majani` produces, and it is the local model arm
        // this seam exists to make work for a user with no API key.
        let takalif = self.muzawwid.takalif();
        takalif.saqf() == 0 || takalif.munfaq() < takalif.saqf()
    }

    fn tarjim(&self, talab: &TalabSatr<'_>) -> Result<RaddSatr, KhataTabaqa> {
        // No spans: the source came off a screen, so there is no extractor's
        // markup to protect. `ihmi` is still the only way to build the value a
        // provider accepts, and going through it means the overlay's text takes
        // exactly the same route to the wire as everything else does.
        let mahmi = ihmi(talab.asl, &[]).map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "translation request",
            sabab: khata.to_string(),
        })?;

        let siyaq = SiyaqTalab {
            tasnif: talab.tasnif,
            siyaq: talab.siyaq.clone(),
            quyud: quyud_min_sunduq(talab),
            // Left empty, and the emptiness is a statement rather than a gap.
            // A glossary term and a memory match are both keyed on a source
            // string that somebody extracted; the overlay's source string was
            // read off a picture and may itself be wrong, and offering a near
            // match of an uncertain reading is how a player gets fluent Arabic
            // saying something the game did not say. The overlay's own exact
            // lookup happens in `crate::qissa` before this is ever reached.
            mustalahat: Vec::new(),
            dhakira: Vec::new(),
            ism_luba: talab.ism_luba.map(str::to_owned),
            thiqat_tasnif: AQALL_THIQAT_TASNIF.saturating_sub(1),
        };

        let natija = self
            .zaman
            .block_on(self.muzawwid.tarjim(&mahmi, &siyaq))
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "translation provider",
                sabab: khata.to_string(),
            })?;

        // Restored even though nothing was protected. A model that invented a
        // token would otherwise leave `⟦0⟧` on screen, and this is the one call
        // that refuses it rather than drawing it.
        let mustaad = taarib_tarjama::hima::istaridd(&mahmi, natija.matn()).map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "translation restoration",
                sabab: khata.to_string(),
            }
        })?;
        Ok(RaddSatr::aaliya(mustaad.naqi))
    }
}

/// The constraints the overlay actually measured for one line.
///
/// Every field here comes from the rectangle the recognizer read the line out
/// of, which is the same rectangle the Arabic will be drawn into. Nothing is
/// estimated and nothing that was not measured is filled in: the character limit
/// stays [`None`] because no engine imposed one, and the font size stays [`None`]
/// because the overlay does not know it — the box is the ink the recognizer
/// found, not a line box.
const fn quyud_min_sunduq(talab: &TalabSatr<'_>) -> QuyudNass {
    let mustatil = Mustatil {
        s: madaa(talab.mawdi.yasar),
        a: madaa(talab.mawdi.aala),
        ard: madaa(talab.mawdi.ard),
        irtifa: madaa(talab.mawdi.irtifa),
    };
    QuyudNass {
        aqsa_ahruf: None,
        aqsa_ard: Some(mustatil.ard),
        aqsa_irtifa: Some(mustatil.irtifa),
        hajm_khatt: None,
        mustatil: Some(mustatil),
        // The tier draws one line per recognized box and shrinks rather than
        // wrapping — see `crate::talqeem::takhtit_iftiradi`. Telling the model
        // otherwise would invite a translation that only fits on two lines.
        satr_wahid: true,
    }
}

/// A pixel dimension as a float.
const fn madaa(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

// ---------------------------------------------------------------------------
// The shared translation memory, behind the overlay's seam
// ---------------------------------------------------------------------------

/// The workspace's shared translation memory, behind
/// [`crate::mutarjim::DhakiraTabaqa`].
///
/// The memory keys on the recognized text and stores the provenance the overlay
/// hands it, which is why this adapter is small: the two contracts were written
/// against each other. What it does add is the one translation that matters —
/// [`crate::mutarjim::QaydTabaqa::thiqa`] is `Option<u8>` whose absence means
/// "the engine reports no confidence", and `ThiqatQira::Ghayr` is the memory's
/// name for the same state. Mapping `None` onto a number here would launder the
/// stand-in constant into a store that refuses to hold one.
///
/// `NawAsl::bashari` is what decides whether a hit is presented as a person's
/// work; everything else the memory returns came off somebody's screen, and
/// [`crate::sijill_qira::MasdarTarjama::Mulahaza`] is what says so in the
/// reading history rather than letting it pass as this session's own machine
/// translation.
#[derive(Debug)]
pub struct DhakiraMushtarakaTabaqa {
    dhakira: Arc<DhakiraMushtaraka>,
    tasnif: TasnifNass,
}

impl DhakiraMushtarakaTabaqa {
    /// Wraps the shared memory for one session.
    ///
    /// `tasnif` is what the overlay's regions hold, and it is stamped onto every
    /// pair stored rather than left at the memory's own default: a suggestion
    /// card that cannot say whether a stored line was dialogue or a menu label
    /// is a card a translator cannot use.
    #[must_use]
    pub const fn jadeeda(dhakira: Arc<DhakiraMushtaraka>, tasnif: TasnifNass) -> Self {
        Self { dhakira, tasnif }
    }

    /// The memory underneath, for a caller reading its counters.
    #[must_use]
    pub fn dhakira(&self) -> &DhakiraMushtaraka {
        self.dhakira.as_ref()
    }
}

impl DhakiraTabaqa for DhakiraMushtarakaTabaqa {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait returns `&str` so an implementation can name itself after something \
                  it owns; an impl signature cannot narrow the trait's"
    )]
    fn ism(&self) -> &str {
        "shared translation memory"
    }

    fn ibhath(&self, talab: &TalabDhakiraTabaqa<'_>) -> Option<RaddSatr> {
        // A refusal from the store is a miss, not a failure. The lookup runs on
        // the worker between a recognition and a translation, and a session that
        // stopped translating because a database file was momentarily locked
        // would be a session that stopped drawing Arabic over a disk problem.
        let radd = self.dhakira.istafhim(talab.asl).ok()?;
        match radd {
            RaddTabaqa::Jahiza { arabi, naw, .. } => Some(if naw.bashari() {
                RaddSatr::basharia(arabi)
            } else {
                // Everything the memory holds that a person did not write came
                // off a screen — that is what this store is for — so it is
                // reported as an observation rather than as a translation of
                // the game's own string.
                RaddSatr::mulahaza(arabi)
            }),
            RaddTabaqa::Majhula => None,
        }
    }

    fn sajjil(&self, qayd: &QaydTabaqaSajjil<'_>) -> Result<(), KhataTabaqa> {
        let thiqa = qayd
            .thiqa
            .map_or(ThiqatQira::Ghayr, ThiqatQira::maqisa);
        let mut mulahaza =
            MulahazaTabaqa::jadeeda(qayd.asl, qayd.arabi, qayd.luba, thiqa)
                .bi_tasnif(self.tasnif)
                .bi_muharrikayn(
                    qayd.qari.map(str::to_owned),
                    qayd.muzawwid.map(str::to_owned),
                );
        if let Some(ism) = qayd.ism_luba {
            mulahaza = mulahaza.bi_ism_luba(ism);
        }
        if let Some(mintaqa) = qayd.mintaqa {
            mulahaza = mulahaza.bi_mintaqa(mintaqa);
        }
        let _ = self.dhakira.sajjil(&mulahaza).map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "shared translation memory",
                sabab: khata.to_string(),
            }
        })?;
        Ok(())
    }

    fn daaima(&self) -> bool {
        self.dhakira.daaima()
    }
}
