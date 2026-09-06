//! # عقل تعريب — one game, held whole
//!
//! Thirty crates know a great deal about a game and nothing holds it all at
//! once. The product derives the same facts in several places, and they have
//! repeatedly disagreed: two commands named different refusals for the same
//! game when three applied at once; a readiness verdict answered from a constant
//! rather than from what was present; a tier discriminant that existed in Rust
//! and was never sent, so the interface rendered a number and a hand-written
//! paragraph beside it drifted from the backend's own sentence; a library row
//! that could not tell *probed and unrecognised* from *never probed*; an
//! anti-cheat gate that skipped silently and returned a verdict byte-identical
//! to a clean game's.
//!
//! Each was fixed by hand. The shape is the same every time: **the product knows
//! something in one place and does not know it in another.**
//!
//! [`Aql`] is one type that carries everything known about one game and answers
//! every question the product asks about it, so that two surfaces **cannot**
//! disagree.
//!
//! ## It composes; it does not re-derive
//!
//! Every field of [`mudkhalat::MudkhalatAql`] is one producer's answer, kept as
//! that producer worded it. No engine table, no anti-cheat signature list, no
//! tier rule and no readiness verdict lives here. The value is the single
//! vantage point and the guaranteed consistency, not new logic — so when a
//! producer is wrong, the wrong answer is traceable to it rather than to a
//! second opinion formed here.
//!
//! ## The five questions
//!
//! | question | answer |
//! | --- | --- |
//! | Which product does this game get, and why | [`Aql::muntaj`] and [`Aql::waad`] |
//! | What is promised, in both languages | [`Aql::waad`] |
//! | What the limits are | [`Aql::hudud`] |
//! | What risks need consent before anything is written | [`Aql::makhatir`] |
//! | What blocks it outright, in order | [`Aql::mawani`] |
//!
//! **The order lives here, once.** [`mani::NawMani::rutba`] is the single
//! ordering every surface reads, and it is what stops one screen naming
//! anti-cheat while the button beside it names the publisher's own Arabic.
//!
//! ## Every answer names its input
//!
//! Each answer comes back as a [`shahid::Musnad`]: the verdict, and every held
//! input it rests on. Without that this would be a cache. With it, "why does
//! this game say that" has an answer, a wrong answer is traceable to a wrong
//! *input* instead of to a guess, and a test can construct a situation and
//! assert both the verdict and its stated reason.
//!
//! ## What it does not do
//!
//! It does not write, plan, install, fetch or launch. It does not read a game's
//! files itself — [`mudkhalat::MasahAman`] and [`mudkhalat::MasahWukala`] call
//! the producers that do, read-only, and hold what they said. It has no clock:
//! every timestamp it reports came in on a producer's answer.

pub mod khatar;
pub mod mani;
pub mod mudkhalat;
pub mod muntaj;
pub mod shahid;

use std::cmp::Reverse;
use std::collections::BTreeSet;

use taarib_aman::fahs::Rafd;
use taarib_aman::kashf_himaya::{HalatMatjar, mahmiya};
use taarib_aman::kashf_shabaka::mutaaddid;
use taarib_kashf::fahs::SimatLuba;
use taarib_mustalahat::bina::MutabaqaBina;
use taarib_mustalahat::luba::HalatLughaRasmiya;
use taarib_mustalahat::muharrik::{
    Daleel, Hadd, JahiziyatTashghil, Muharrik, TaqreerImkaniyat, WajihaRusum,
};

pub use crate::khatar::{Khatar, NawKhatar};
pub use crate::mani::{Mani, NawMani, NitaqMani};
pub use crate::mudkhalat::{
    HalatFahs, HalatMakhzan, HalatMustawda, HuwiyatLuba, MasahAman, MasahWukala, MawqifMustakhdim,
    MudkhalatAql, MukawwinMakhzan,
};
pub use crate::muntaj::{Muntaj, Waad};
pub use crate::shahid::{MasdarMarifa, Musnad, Shahid};

/// How many of the engine probe's own observations travel with a verdict.
///
/// Three. The whole evidence trail is on the report and a caller that wants it
/// has it; what a chain of transmission is for is naming the inputs a reader
/// would go and check, and a list nobody reads to the end names none of them.
const AQSA_DALAIL: usize = 3;

/// The two anti-cheat sources naming different things about the same game.
///
/// The launcher's own metadata and a scan of the game's files are independent,
/// and either can be wrong: a store listing can be years out of date, and a scan
/// can meet an anti-cheat this build has no signature for. When they disagree
/// the refusal still stands — both are evidence *for* refusing — but which of
/// them is wrong is a thing a maintainer must be able to see, and averaging them
/// into one name is how that becomes invisible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IkhtilafHimaya {
    /// What the scan of the game's own files and the store catalogue named.
    pub min_kashf: Vec<String>,
    /// What the launcher's own metadata named.
    pub min_iktishaf: Vec<String>,
}

/// What the anti-cheat question actually got as an answer.
///
/// Three values, and the third is why this is not a boolean. `mahmiya` answers
/// "did any evidence turn up", and a caller reading it as "is this game clean"
/// inherits a hole it cannot see: `NawHimaya` is a **closed set**, so a scan
/// that meets an anti-cheat this build has no signature for returns an empty
/// evidence list — byte-identical to the list a genuinely clean game returns.
///
/// That is not a hypothetical, and the example is worth keeping even though it
/// is now closed. EA SPORTS FC 26 ships `EAAntiCheat.GameServiceLauncher.exe`,
/// and the same scan that correctly blocked ELDEN RING and GTA V returned
/// **nothing** for it — permitting an install into a game whose anti-cheat bans
/// accounts — purely because EA Javelin was not in the set. The set has since
/// grown from twelve kinds to eighteen and FC 26 is refused by name; the shape
/// of the failure has not changed, and the next missing signature will look
/// exactly the same from here.
///
/// So the negative answer is [`Self::LaTawqee`] — *no signature matched* — and
/// it is worded as a fact about Taarib's signature list rather than as a fact
/// about the game. It carries the scan's own coverage limits in its chain: the
/// places it could not read, and whether the walk stopped at a bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalatHimaya {
    /// Evidence was found. Refused, and there is no override anywhere in the
    /// product.
    Mahmiya,
    /// The question could not be put at all, so silence proves nothing. VAC is
    /// declared only in the store's catalogue and leaves nothing in a game
    /// folder.
    LamYajri,
    /// The scan ran and nothing in this build's signature list matched.
    ///
    /// The only answer on which an install proceeds, and deliberately not called
    /// "clean": what it asserts is that twelve named anti-cheats were looked for
    /// and none was found, which is a smaller claim.
    LaTawqee,
}

impl HalatHimaya {
    /// Whether an install may proceed on this answer.
    #[must_use]
    pub const fn yasmah(self) -> bool {
        matches!(self, Self::LaTawqee)
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mahmiya => "mahmiya",
            Self::LamYajri => "lam_yajri",
            Self::LaTawqee => "la_tawqee",
        }
    }
}

/// Everything known about one game, and every answer the product asks of it.
///
/// Built from [`MudkhalatAql`] and then only read. It holds no handle, no
/// connection and no thread: a caller assembles the inputs, asks its questions,
/// and drops it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aql {
    mudkhalat: MudkhalatAql,
}

impl Aql {
    /// Holds one game's inputs together.
    #[must_use]
    pub const fn jadeed(mudkhalat: MudkhalatAql) -> Self {
        Self { mudkhalat }
    }

    // -----------------------------------------------------------------------
    // What is held
    // -----------------------------------------------------------------------

    /// Everything that was handed in, for a caller that needs an input this
    /// crate does not answer a question from.
    #[must_use]
    pub const fn mudkhalat(&self) -> &MudkhalatAql {
        &self.mudkhalat
    }

    /// Which game, where, and what its launcher said.
    #[must_use]
    pub const fn huwiya(&self) -> &HuwiyatLuba {
        &self.mudkhalat.huwiya
    }

    /// The capability report, when this game has been examined.
    #[must_use]
    pub const fn taqreer(&self) -> Option<&TaqreerImkaniyat> {
        self.mudkhalat.fahs.taqreer.as_ref()
    }

    /// The identified engine, when this game has been examined.
    #[must_use]
    pub fn muharrik(&self) -> Option<&Muharrik> {
        self.taqreer().map(|taqreer| &taqreer.muharrik)
    }

    /// The graphics APIs the probe read out of the game's own binaries.
    ///
    /// Read off the held report rather than kept as a second field. There is one
    /// producer for this fact, and a core that stored its own copy beside the
    /// report would be the seventh instance of the defect this crate exists to
    /// prevent — including the copy's own drift, since the probe's vocabulary is
    /// still growing.
    ///
    /// How strongly each entry is held is not on this list and is not invented
    /// here: an API read out of an import table and one inferred from a
    /// redistributable the game happens to ship are worth different amounts, and
    /// the difference is recorded where it was observed, as the weight on the
    /// engine's own [`Daleel`]. A caller that needs it reads
    /// [`Aql::muharrik`]'s evidence rather than this list.
    #[must_use]
    pub fn rusum(&self) -> &[WajihaRusum] {
        self.muharrik()
            .map_or(&[], |muharrik| muharrik.rusum.as_slice())
    }

    /// The two anti-cheat sources disagreeing, when they do.
    ///
    /// [`None`] when they agree, when only one of them spoke, or when neither
    /// did.
    #[must_use]
    pub fn ikhtilaf_himaya(&self) -> Option<IkhtilafHimaya> {
        let min_kashf: BTreeSet<String> = self
            .mudkhalat
            .aman
            .himaya
            .anwa()
            .into_iter()
            .map(|naw| naw.injilizi().to_owned())
            .collect();
        let min_iktishaf: BTreeSet<String> = self
            .huwiya()
            .simat
            .iter()
            .filter_map(|sima| match sima {
                SimatLuba::HimayaMuhtamala(ism) => Some(ism.clone()),
                SimatLuba::MuammanaVac => Some("Valve Anti-Cheat (VAC)".to_owned()),
                _ => None,
            })
            .collect();
        if min_kashf.is_empty() || min_iktishaf.is_empty() || min_kashf == min_iktishaf {
            return None;
        }
        Some(IkhtilafHimaya {
            min_kashf: min_kashf.into_iter().collect(),
            min_iktishaf: min_iktishaf.into_iter().collect(),
        })
    }

    /// The components this build asked the store for and did not find whole.
    pub fn mukawwinat_naqisa(&self) -> impl Iterator<Item = &MukawwinMakhzan> {
        self.mudkhalat.makhzan.naqisa()
    }

    /// What the anti-cheat question was actually answered with.
    ///
    /// The negative answer names what was looked for rather than claiming the
    /// game is clean, and its chain carries every place the scan could not
    /// reach — so a caller acting on it is acting on a stated claim about
    /// Taarib's signature list, not on an absence it mistook for a finding.
    #[must_use]
    pub fn hala_himaya(&self) -> Musnad<HalatHimaya> {
        let ijmaa = &self.mudkhalat.aman.himaya;
        if mahmiya(ijmaa) {
            let shawahid = ijmaa
                .adilla
                .iter()
                .map(|daleel| Shahid {
                    masdar: MasdarMarifa::KashfHimaya,
                    wasf: daleel.injilizi(),
                    mawqi: daleel
                        .masar
                        .as_ref()
                        .map(|masar| masar.display().to_string()),
                })
                .collect();
            return Musnad::jadeed(HalatHimaya::Mahmiya, shawahid);
        }
        if self.mudkhalat.aman.halat_matjar.lam_yuqra() {
            return Musnad::jadeed(
                HalatHimaya::LamYajri,
                vec![Shahid::jadeed(
                    MasdarMarifa::FahrasMatjar,
                    "the store catalogue was owed and never read",
                )],
            );
        }

        let mut shawahid = vec![Shahid::jadeed(
            MasdarMarifa::KashfHimaya,
            "the scan ran and no signature in this build's list matched",
        )];
        shawahid.extend(ijmaa.thughrat.iter().map(|thughra| Shahid {
            masdar: MasdarMarifa::KashfHimaya,
            wasf: format!("this place could not be read ({})", thughra.sabab),
            mawqi: Some(thughra.masar.display().to_string()),
        }));
        if ijmaa.mabtur {
            shawahid.push(Shahid::jadeed(
                MasdarMarifa::KashfHimaya,
                "the walk stopped at a bound before it finished",
            ));
        }
        Musnad::jadeed(HalatHimaya::LaTawqee, shawahid)
    }

    // -----------------------------------------------------------------------
    // The blockers, in the one order
    // -----------------------------------------------------------------------

    /// Everything standing between this game and Taarib, most serious first.
    ///
    /// The order is [`NawMani::rutba`] and it is not restated at any call site.
    /// A surface that needs one reason takes the first; a surface that needs the
    /// list gets it already sorted; and a surface that only writes into games
    /// filters on [`NitaqMani::Kul`], because an unfinished adapter refuses the
    /// automatic run and does not refuse a hand-installed patch.
    #[must_use]
    pub fn mawani(&self) -> Vec<Musnad<Mani>> {
        let mut mawani: Vec<Musnad<Mani>> = Vec::new();
        mawani.extend(self.mani_himaya());
        mawani.extend(self.mani_fahs_matjar());
        mawani.extend(self.mani_lugha_rasmiya());
        mawani.extend(self.mani_laysat_luba());
        mawani.extend(self.mani_muhakat_rum());
        mawani.extend(self.mani_ghayr_hadira());
        mawani.extend(self.mani_lam_yufhas());
        mawani.extend(self.mani_jahiziya());
        // Pushed in rank order above; sorted anyway so that the guarantee is the
        // type's rather than this function's reading order, and so that adding a
        // blocker in the wrong place here cannot change what a surface shows.
        mawani.sort_by_key(|mani| mani.qeema.rutba());
        mawani
    }

    /// The blockers that stop everything, as against the ones that stop only the
    /// automatic run.
    pub fn mawani_kulliya(&self) -> impl Iterator<Item = Musnad<Mani>> {
        self.mawani()
            .into_iter()
            .filter(|mani| mani.qeema.nitaq() == NitaqMani::Kul)
    }

    /// The single most serious blocker, which is the one a surface with room for
    /// one sentence shows.
    #[must_use]
    pub fn mani_awwal(&self) -> Option<Musnad<Mani>> {
        self.mawani().into_iter().next()
    }

    /// Whether Taarib will touch this game at all.
    #[must_use]
    pub fn marfuda(&self) -> bool {
        self.mawani_kulliya().next().is_some()
    }

    /// Whether a patch may be installed into this game right now: nothing blocks
    /// it outright and every risk has been answered.
    #[must_use]
    pub fn jahiz_lil_tathbeet(&self) -> bool {
        !self.marfuda() && self.makhatir().iter().all(|khatar| khatar.qeema.muqarr)
    }

    /// Whether the one-button run may start: the above, and an adapter that
    /// reaches the screen.
    #[must_use]
    pub fn jahiz_lil_tashghil(&self) -> bool {
        self.jahiz_lil_tathbeet() && self.mawani().is_empty()
    }

    // -----------------------------------------------------------------------
    // The product and its promise
    // -----------------------------------------------------------------------

    /// Which of the products this game gets.
    #[must_use]
    pub fn muntaj(&self) -> Musnad<Muntaj> {
        self.waad().hawwil(|waad| waad.muntaj)
    }

    /// What this game is promised, in both languages, in the backend's own
    /// sentences.
    ///
    /// A game something blocks outright is promised nothing, and the sentence it
    /// carries is the blocker's own — one blocker, chosen by the one ordering,
    /// so the reason on this screen is the reason on every other. That is the
    /// whole of the fix for two commands naming different refusals for one game.
    #[must_use]
    pub fn waad(&self) -> Musnad<Waad> {
        if let Some(mani) = self.mawani_kulliya().next() {
            let muntaj = if mani.qeema.naw == NawMani::LamYufhas {
                Muntaj::Majhul
            } else {
                Muntaj::LaShay
            };
            let waad = Waad {
                muntaj,
                ism_arabi: muntaj.ism_arabi(),
                ism_injilizi: muntaj.ism_injilizi(),
                sabab_arabi: mani.qeema.arabi.clone(),
                sabab_injilizi: mani.qeema.injilizi.clone(),
                // A blocked game's readiness is true and beside the point: why
                // Taarib's adapter is unfinished is not the sentence to leave
                // with somebody whose game Taarib is refusing to touch. The
                // capability report withholds it for exactly this case and so
                // does this.
                jahiziya: JahiziyatTashghil::Ghaiba,
                naqs: None,
            };
            return Musnad::jadeed(waad, mani.shawahid);
        }

        let Some(taqreer) = self.taqreer() else {
            // Unreachable while `mani_lam_yufhas` covers a missing report, and
            // written as an answer rather than as a panic because a core that
            // could stop a caller's process is a core nothing can safely ask.
            let waad = Waad {
                muntaj: Muntaj::Majhul,
                ism_arabi: Muntaj::Majhul.ism_arabi(),
                ism_injilizi: Muntaj::Majhul.ism_injilizi(),
                sabab_arabi: self.mudkhalat.fahs.sabab.arabi(),
                sabab_injilizi: self.mudkhalat.fahs.sabab.injilizi(),
                jahiziya: JahiziyatTashghil::Ghaiba,
                naqs: None,
            };
            return Musnad::jadeed(waad, vec![self.shahid_bitaqa()]);
        };

        let muntaj = Muntaj::min_tabaqa(taqreer.tabaqa);
        let waad = Waad {
            muntaj,
            ism_arabi: muntaj.ism_arabi(),
            ism_injilizi: muntaj.ism_injilizi(),
            sabab_arabi: taqreer.sabab_arabi.clone(),
            sabab_injilizi: taqreer.sabab_injilizi.clone(),
            jahiziya: taqreer.jahiziya,
            naqs: taqreer.naqs.clone(),
        };
        Musnad::jadeed(waad, self.shawahid_taqreer(taqreer))
    }

    // -----------------------------------------------------------------------
    // The limits
    // -----------------------------------------------------------------------

    /// Everything that will not work for this game, named specifically.
    ///
    /// Straight off the capability report, which is where limitations are
    /// written and argued over. A blocked game carries its blocker's sentence
    /// and nothing else, for the reason the report gives for a refused game:
    /// listing what would have been difficult about Arabizing a game nobody is
    /// going to Arabize is noise wrapped around the only sentence that matters.
    #[must_use]
    pub fn hudud(&self) -> Vec<Musnad<Hadd>> {
        if let Some(mani) = self.mawani_kulliya().next() {
            let hadd = Hadd {
                arabi: mani.qeema.arabi.clone(),
                injilizi: mani.qeema.injilizi,
            };
            return vec![Musnad::jadeed(hadd, mani.shawahid)];
        }
        let Some(taqreer) = self.taqreer() else {
            return Vec::new();
        };
        taqreer
            .hudud
            .iter()
            .map(|hadd| {
                Musnad::jadeed(
                    hadd.clone(),
                    vec![Shahid::jadeed(
                        MasdarMarifa::Imkaniyat,
                        format!(
                            "the capability report for {} lists this limitation",
                            taqreer.muharrik.aila.ism()
                        ),
                    )],
                )
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // The risks
    // -----------------------------------------------------------------------

    /// Every risk that has to be accepted before anything is written, with the
    /// answer already on record.
    ///
    /// In the order they are asked, which is [`NawKhatar::rutba`]. A risk with
    /// no answer is a real no: [`Khatar::muqarr`] is false until the user says
    /// otherwise, and nothing here derives an acceptance from anything else.
    #[must_use]
    pub fn makhatir(&self) -> Vec<Musnad<Khatar>> {
        let mut makhatir: Vec<Musnad<Khatar>> = Vec::new();
        makhatir.push(self.khatar_bayan());
        makhatir.extend(self.khatar_lugha_rasmiya());
        makhatir.extend(self.khatar_wakeel());
        makhatir.extend(self.khatar_mutabaqa());
        makhatir.extend(self.khatar_shabaka());
        makhatir.sort_by_key(|khatar| khatar.qeema.rutba());
        makhatir
    }

    /// The risks still standing between the user and a write.
    pub fn makhatir_muallaqa(&self) -> impl Iterator<Item = Musnad<Khatar>> {
        self.makhatir()
            .into_iter()
            .filter(|khatar| khatar.qeema.muallaq())
    }

    // -----------------------------------------------------------------------
    // The blockers, one at a time
    // -----------------------------------------------------------------------

    /// Anti-cheat, from either source, refused with the safety layer's own words.
    ///
    /// Both sources are consulted and both travel in the chain. The scan of the
    /// game's files is preferred for the sentence because it names evidence the
    /// user can go and look at, while the launcher's hint names a fact about a
    /// catalogue; when only the hint fired, the capability report's own refusal
    /// sentence is the one that is shown, which is what
    /// `taarib_tilqai::fahs::tahaqquq_jahiziya` also reads.
    fn mani_himaya(&self) -> Option<Musnad<Mani>> {
        let ijmaa = &self.mudkhalat.aman.himaya;
        let bil_kashf = mahmiya(ijmaa);
        let bil_taqreer = self.taqreer().is_some_and(|taqreer| taqreer.marfuda);
        if !bil_kashf && !bil_taqreer {
            return None;
        }

        let mut shawahid: Vec<Shahid> = ijmaa
            .adilla
            .iter()
            .map(|daleel| Shahid {
                masdar: MasdarMarifa::KashfHimaya,
                wasf: daleel.injilizi(),
                mawqi: daleel
                    .masar
                    .as_ref()
                    .map(|masar| masar.display().to_string()),
            })
            .collect();
        if bil_taqreer {
            shawahid.push(Shahid::jadeed(
                MasdarMarifa::Imkaniyat,
                "the capability report refuses this game outright",
            ));
            shawahid.extend(self.huwiya().simat.iter().filter_map(|sima| match sima {
                SimatLuba::HimayaMuhtamala(ism) => Some(Shahid::jadeed(
                    MasdarMarifa::Iktishaf,
                    format!("the launcher associates this game with {ism}"),
                )),
                SimatLuba::MuammanaVac => Some(Shahid::jadeed(
                    MasdarMarifa::Iktishaf,
                    "the launcher reports this game as VAC-secured",
                )),
                _ => None,
            }));
        }

        let (arabi, injilizi) = if bil_kashf {
            let rafd = Rafd::Himaya(Box::new(ijmaa.clone()));
            (rafd.arabi(), rafd.injilizi())
        } else {
            self.jumlat_rafd_taqreer()
        };
        Some(Musnad::jadeed(
            Mani::jadeed(NawMani::Himaya, arabi, injilizi),
            shawahid,
        ))
    }

    /// The refusal sentence a refused capability report carries.
    ///
    /// Its first limitation, which for a refused report is the refusal and the
    /// only entry; its own reason otherwise. This is exactly the fallback
    /// `taarib_tilqai::fahs::tahaqquq_jahiziya` takes, and it is spelled the same
    /// way so the two cannot show different text for one game.
    fn jumlat_rafd_taqreer(&self) -> (String, String) {
        self.taqreer().map_or_else(
            || (String::new(), String::new()),
            |taqreer| {
                taqreer.hudud.first().map_or_else(
                    || (taqreer.sabab_arabi.clone(), taqreer.sabab_injilizi.clone()),
                    |hadd| (hadd.arabi.clone(), hadd.injilizi.clone()),
                )
            },
        )
    }

    /// The anti-cheat question that could not be put.
    ///
    /// The gate is [`HalatMatjar::lam_yuqra`] — the producer's own predicate,
    /// not a re-reading of the variants — and the sentence is
    /// [`Rafd::FahsMatjarLamYajri`]'s. Only the destructure between them is
    /// local, because the safety layer's own mapping is private to it.
    fn mani_fahs_matjar(&self) -> Option<Musnad<Mani>> {
        let hala = &self.mudkhalat.aman.halat_matjar;
        if !hala.lam_yuqra() {
            return None;
        }
        let (masar, sabab) = match hala {
            HalatMatjar::Mutaadhdhir { masar, sabab } => (Some(masar.clone()), sabab.clone()),
            _ => (None, "no Steam root was given for a Steam game".to_owned()),
        };
        let mawqi = masar.as_ref().map(|masar| masar.display().to_string());
        let rafd = Rafd::FahsMatjarLamYajri {
            masar,
            sabab: sabab.clone(),
        };
        let shahid = Shahid {
            masdar: MasdarMarifa::FahrasMatjar,
            wasf: format!("the store catalogue was owed and not read: {sabab}"),
            mawqi,
        };
        Some(Musnad::jadeed(
            Mani::jadeed(NawMani::FahsHimayaLamYajri, rafd.arabi(), rafd.injilizi()),
            vec![shahid],
        ))
    }

    /// The publisher's own Arabic, when the user has not asked to be offered
    /// those games anyway.
    fn mani_lugha_rasmiya(&self) -> Option<Musnad<Mani>> {
        let hukm = self.mudkhalat.lugha.as_ref()?;
        if !hukm.yatakallam_arabi() || self.mudkhalat.mawqif.istibdal_lugha_rasmiya {
            return None;
        }
        let mut shawahid: Vec<Shahid> = hukm
            .dalail
            .iter()
            .take(AQSA_DALAIL)
            .map(|daleel| Shahid {
                masdar: MasdarMarifa::LughaRasmiya,
                wasf: daleel.wasf.clone(),
                mawqi: daleel.mawqi.clone(),
            })
            .collect();
        shawahid.push(Shahid::jadeed(
            MasdarMarifa::Idadat,
            "istibdal_lugha_rasmiya is off, so a game that already ships Arabic is not offered",
        ));
        let (arabi, injilizi) = jumla_lugha(hukm.hala());
        Some(Musnad::jadeed(
            Mani::jadeed(NawMani::LughaRasmiya, arabi, injilizi),
            shawahid,
        ))
    }

    /// An entry the launcher itself says is not a game.
    fn mani_laysat_luba(&self) -> Option<Musnad<Mani>> {
        let naw = self.huwiya().naw_ghayr_luba()?;
        let shahid = Shahid::jadeed(
            MasdarMarifa::Iktishaf,
            format!("the launcher marks this entry as {naw}, not a game"),
        );
        Some(Musnad::jadeed(
            Mani::jadeed(
                NawMani::LaysatLuba,
                format!(
                    "هذا المدخل ليس لعبة؛ يصنّفه المتجر على أنه {naw}. لا يُعرَّب إلا ما هو لعبة، \
                     ولن يُكتب في هذا المجلّد شيء."
                ),
                format!(
                    "This entry is not a game — the launcher classifies it as {naw}. Taarib \
                     arabizes games, and nothing will be written into this folder."
                ),
            ),
            vec![shahid],
        ))
    }

    /// A directory of ROM images, which is in the library so the library is
    /// complete and not so that it can be translated.
    fn mani_muhakat_rum(&self) -> Option<Musnad<Mani>> {
        let aila = self.huwiya().ailat_rum()?;
        let shahid = Shahid::jadeed(
            MasdarMarifa::Iktishaf,
            format!("the launcher records this entry as an emulated title ({aila})"),
        );
        Some(Musnad::jadeed(
            Mani::jadeed(
                NawMani::MuhakatRum,
                format!(
                    "هذه نسخة لعبة تعمل على محاكي ({aila})، ونصوصها داخل صورة القرص أو الخرطوشة \
                     نفسها. لا يوجد في تعريب طريق يعدّل هذا النوع، وهو في المكتبة ليكتمل عرضها \
                     لا ليُعرَّب."
                ),
                format!(
                    "This is an emulated title ({aila}), and its text lives inside the disc or \
                     cartridge image itself. Taarib has no supported path that modifies one; it \
                     is in the library so the library is complete, not so it can be arabized."
                ),
            ),
            vec![shahid],
        ))
    }

    /// A game that is not on this disk, or not all of it.
    fn mani_ghayr_hadira(&self) -> Option<Musnad<Mani>> {
        if self.mudkhalat.huwiya.muktamila && self.mudkhalat.huwiya.hala_matjar.is_none() {
            return None;
        }
        let (arabi, injilizi, wasf) = self.mudkhalat.huwiya.hala_matjar.as_ref().map_or_else(
            || {
                (
                    "لم تكتمل هذه اللعبة بعد على القرص، فملفّاتها التي سيعدّلها تعريب قد يستبدلها \
                     المتجر في أي لحظة. أكمل التنزيل ثم أعد المحاولة."
                        .to_owned(),
                    "This game is not fully downloaded, so the files Taarib would modify are \
                     files the launcher is about to overwrite. Finish the download and try \
                     again."
                        .to_owned(),
                    "the launcher reports this game as not fully downloaded".to_owned(),
                )
            },
            |sabab| (sabab.arabi(), sabab.injilizi(), sabab.injilizi()),
        );
        let shahid = Shahid::jadeed(MasdarMarifa::Iktishaf, wasf);
        Some(Musnad::jadeed(
            Mani::jadeed(NawMani::GhayrHadira, arabi, injilizi),
            vec![shahid],
        ))
    }

    /// A game nobody has examined, which is a different answer from a game the
    /// probe examined and could not recognise.
    fn mani_lam_yufhas(&self) -> Option<Musnad<Mani>> {
        if self.mudkhalat.fahs.ladayha_taqreer() {
            return None;
        }
        let sabab = self.mudkhalat.fahs.sabab;
        Some(Musnad::jadeed(
            Mani::jadeed(NawMani::LamYufhas, sabab.arabi(), sabab.injilizi()),
            vec![self.shahid_bitaqa()],
        ))
    }

    /// An adapter this build does not finish, which refuses the automatic run
    /// and does not refuse a hand-installed patch.
    ///
    /// The gate is the report's own [`JahiziyatTashghil::tasil`], and the
    /// sentence is the report's own gap. Both are read rather than recomputed,
    /// which is what keeps this in step with
    /// `taarib_tilqai::fahs::naqs_jahiziya` without either holding the other's
    /// engine table.
    fn mani_jahiziya(&self) -> Option<Musnad<Mani>> {
        let taqreer = self.taqreer()?;
        if taqreer.marfuda || taqreer.jahiziya.tasil() {
            return None;
        }
        let naqs = taqreer.naqs.as_ref()?;
        let mut shawahid = vec![Shahid::jadeed(
            MasdarMarifa::Imkaniyat,
            format!(
                "the readiness verdict for {} is {}",
                taqreer.muharrik.aila.ism(),
                taqreer.jahiziya.ism()
            ),
        )];
        shawahid.extend(self.shawahid_makhzan());
        Some(Musnad::jadeed(
            Mani::jadeed(
                NawMani::JahiziyaGhaiba,
                naqs.arabi.clone(),
                naqs.injilizi.clone(),
            ),
            shawahid,
        ))
    }

    // -----------------------------------------------------------------------
    // The risks, one at a time
    // -----------------------------------------------------------------------

    /// Taarib's own first-run statement, asked once for the whole product.
    fn khatar_bayan(&self) -> Musnad<Khatar> {
        let muqarr = self.mudkhalat.mawqif.muqarr(NawKhatar::BayanAwwal);
        let rafd = Rafd::IqrarNaqis;
        Musnad::jadeed(
            Khatar::jadeed(NawKhatar::BayanAwwal, rafd.arabi(), rafd.injilizi(), muqarr),
            vec![Shahid::jadeed(
                MasdarMarifa::Idadat,
                if muqarr {
                    "the first-run statement is acknowledged"
                } else {
                    "the first-run statement has not been acknowledged"
                },
            )],
        )
    }

    /// A publisher's own Arabic the user has chosen to patch over anyway.
    fn khatar_lugha_rasmiya(&self) -> Option<Musnad<Khatar>> {
        let hukm = self.mudkhalat.lugha.as_ref()?;
        if !hukm.yatakallam_arabi() || !self.mudkhalat.mawqif.istibdal_lugha_rasmiya {
            return None;
        }
        let hala = hukm.hala();
        let muqarr = self
            .mudkhalat
            .mawqif
            .muqarr(NawKhatar::LughaRasmiyaMutajawaza);
        let shawahid = vec![
            Shahid::jadeed(
                MasdarMarifa::LughaRasmiya,
                format!("the official-Arabic verdict is {}", hala.ism_injilizi()),
            ),
            Shahid::jadeed(
                MasdarMarifa::Idadat,
                "istibdal_lugha_rasmiya is on, so this game is offered on the user's judgement",
            ),
        ];
        Some(Musnad::jadeed(
            Khatar::jadeed(
                NawKhatar::LughaRasmiyaMutajawaza,
                format!(
                    "ينشر ناشر هذه اللعبة عربية خاصة به ({}). فعّلتَ خيار عرض هذه الألعاب، \
                     فسيحلّ التعريب الآلي محلّ ترجمة بشرية مدفوعة ومراجَعة. لن يُعرض هذا الخيار \
                     عنك؛ أنت من اختاره.",
                    hala.ism_arabi()
                ),
                format!(
                    "This game's publisher already ships Arabic of their own ({}). You turned on \
                     the setting that offers these games, so an automatic translation would \
                     replace a paid, reviewed human one. Nothing chose this for you.",
                    hala.ism_injilizi()
                ),
                muqarr,
            ),
            shawahid,
        ))
    }

    /// Somebody else's loader already holding a slot beside the game.
    fn khatar_wakeel(&self) -> Option<Musnad<Khatar>> {
        let ghurabaa: Vec<_> = self.mudkhalat.wukala.ghurabaa().collect();
        if ghurabaa.is_empty() {
            return None;
        }
        let muqarr = self.mudkhalat.mawqif.muqarr(NawKhatar::WakeelGhareeb);
        let shawahid: Vec<Shahid> = ghurabaa
            .iter()
            .map(|qaim| {
                Shahid::fi(
                    MasdarMarifa::MasahWukala,
                    qaim.wasf_injilizi(),
                    qaim.ism.clone(),
                )
            })
            .collect();
        let arabi = ghurabaa
            .iter()
            .map(|qaim| format!("• {}", qaim.wasf_arabi()))
            .collect::<Vec<_>>()
            .join("\n");
        let injilizi = ghurabaa
            .iter()
            .map(|qaim| format!("- {}", qaim.wasf_injilizi()))
            .collect::<Vec<_>>()
            .join("\n");
        Some(Musnad::jadeed(
            Khatar::jadeed(
                NawKhatar::WakeelGhareeb,
                format!(
                    "يوجد في مجلّد هذه اللعبة برنامج آخر يشغل خانة تحميل:\n{arabi}\nسيثبّت تعريب \
                     نفسه بجانبه وسيسلسل النداء إليه، وإن أزلتَ أحدهما بيدك فقد يتوقّف الآخر."
                ),
                format!(
                    "Something else already occupies a loader slot in this game's folder:\n\
                     {injilizi}\nTaarib installs beside it and chains on to it, and removing \
                     either one by hand can stop the other."
                ),
                muqarr,
            ),
            shawahid,
        ))
    }

    /// A patch that matches the installed build only approximately.
    fn khatar_mutabaqa(&self) -> Option<Musnad<Khatar>> {
        let hala = self.mudkhalat.mustawda?;
        if !hala.yahtaj_iqrar {
            return None;
        }
        let muqarr = self.mudkhalat.mawqif.muqarr(NawKhatar::MutabaqaTaqribiya);
        Some(Musnad::jadeed(
            Khatar::jadeed(
                NawKhatar::MutabaqaTaqribiya,
                MutabaqaBina::Nitaq.wasf_arabi(),
                MutabaqaBina::Nitaq.wasf_injilizi(),
                muqarr,
            ),
            vec![Shahid::jadeed(
                MasdarMarifa::Mustawda,
                format!(
                    "the best of {} listed patches matches this build by declared range only",
                    hala.adad_ruqaa
                ),
            )],
        ))
    }

    /// Online play with no anti-cheat found in it.
    ///
    /// Fires on either source. The scan of the game's files is the authority
    /// when it ran; the launcher's own listing is what a screen that has done no
    /// I/O has, and asking on it as well is the safe direction — the invariant
    /// the product keeps is that the door may add a question the verdict did not
    /// ask, never answer differently one that it did.
    fn khatar_shabaka(&self) -> Option<Musnad<Khatar>> {
        let ijmaa = &self.mudkhalat.aman.shabaka;
        let bil_kashf = mutaaddid(ijmaa);
        let bil_iktishaf = self.huwiya().jamai_online();
        if !bil_kashf && !bil_iktishaf {
            return None;
        }
        let muqarr = self.mudkhalat.mawqif.muqarr(NawKhatar::LaabJamai);
        let mut shawahid: Vec<Shahid> = ijmaa
            .dalail
            .iter()
            .map(|daleel| Shahid {
                masdar: MasdarMarifa::KashfShabaka,
                wasf: daleel.injilizi(),
                mawqi: daleel
                    .masar
                    .as_ref()
                    .map(|masar| masar.display().to_string()),
            })
            .collect();
        if bil_iktishaf {
            shawahid.push(Shahid::jadeed(
                MasdarMarifa::Iktishaf,
                "the launcher lists online multiplayer for this game",
            ));
        }
        Some(Musnad::jadeed(
            Khatar::jadeed(
                NawKhatar::LaabJamai,
                ijmaa.wasf_iqrar(),
                ijmaa.wasf_injilizi(),
                muqarr,
            ),
            shawahid,
        ))
    }

    // -----------------------------------------------------------------------
    // Chains
    // -----------------------------------------------------------------------

    /// The stored probe record's own answer about whether this game was
    /// examined.
    fn shahid_bitaqa(&self) -> Shahid {
        Shahid::jadeed(MasdarMarifa::Bitaqa, self.mudkhalat.fahs.sabab.injilizi())
    }

    /// What the capability report and the probe behind it say, for a verdict
    /// taken off them.
    fn shawahid_taqreer(&self, taqreer: &TaqreerImkaniyat) -> Vec<Shahid> {
        let muharrik = &taqreer.muharrik;
        let mut shawahid = vec![Shahid::jadeed(
            MasdarMarifa::Imkaniyat,
            format!(
                "{} at confidence {}, tier {}, readiness {}",
                muharrik.aila.ism(),
                muharrik.thiqa,
                taqreer.tabaqa.raqm(),
                taqreer.jahiziya.ism()
            ),
        )];
        shawahid.extend(
            aqwa_dalail(&muharrik.dalail)
                .into_iter()
                .map(|daleel| Shahid {
                    masdar: MasdarMarifa::DaleelMuharrik,
                    wasf: format!("{} (weight {})", daleel.wasf, daleel.wazn),
                    mawqi: daleel.mawqi.clone(),
                }),
        );
        if taqreer.jahiziya != JahiziyatTashghil::Mukammala {
            shawahid.extend(self.shawahid_makhzan());
        }
        shawahid
    }

    /// What the component store on this machine contributes to a verdict about
    /// what reaches the screen.
    ///
    /// Evidence only, never a verdict. The readiness answer is the report's and
    /// is deliberately taken without reading this directory; what this adds is
    /// the other half of "why did nothing appear" — a component this build ships
    /// and this machine does not have.
    fn shawahid_makhzan(&self) -> Vec<Shahid> {
        let makhzan = &self.mudkhalat.makhzan;
        let mut shawahid: Vec<Shahid> = makhzan
            .naqisa()
            .map(|mukawwin| Shahid {
                masdar: MasdarMarifa::MakhzanMukawwinat,
                wasf: mukawwin
                    .sabab
                    .clone()
                    .unwrap_or_else(|| format!("{} is not complete in the store", mukawwin.ism)),
                mawqi: Some(mukawwin.ism.clone()),
            })
            .collect();
        if let Some(khata) = &makhzan.khata_bayan {
            shawahid.push(Shahid::fi(
                MasdarMarifa::MakhzanMukawwinat,
                khata.clone(),
                makhzan.jidhr.display().to_string(),
            ));
        }
        shawahid
    }
}

/// The strongest of an engine's observations, heaviest first.
fn aqwa_dalail(dalail: &[Daleel]) -> Vec<&Daleel> {
    let mut murattaba: Vec<&Daleel> = dalail.iter().collect();
    murattaba.sort_by_key(|daleel| Reverse(daleel.wazn));
    murattaba.truncate(AQSA_DALAIL);
    murattaba
}

/// The two sentences a game whose publisher already ships Arabic is refused
/// with.
///
/// Written here because no producer writes them: the badge vocabulary names the
/// state and the setting's own documentation states the policy, and neither is a
/// sentence addressed to somebody looking at a game they cannot patch. It is one
/// pair, in one place, and the badge half of it is the badge's own words.
fn jumla_lugha(hala: HalatLughaRasmiya) -> (String, String) {
    (
        format!(
            "ينشر ناشر هذه اللعبة عربية خاصة به ({}). لا يعرض تعريب التعريب الآلي هنا، لأن ذلك \
             يستبدل ترجمة بشرية مدفوعة ومراجَعة بناتج آلة. إن كانت عربية اللعبة غير صالحة فعلًا \
             فيمكنك إظهار هذه الألعاب من الإعدادات.",
            hala.ism_arabi()
        ),
        format!(
            "This game's publisher already ships Arabic ({}). Taarib does not offer an automatic \
             translation over it, because that replaces a paid, reviewed human translation with \
             machine output. If the shipped Arabic really is unusable, Settings can reveal these \
             games.",
            hala.ism_injilizi()
        ),
    )
}
