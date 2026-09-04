//! السلسلة — where a card's picture comes from, in order, and what happens when
//! there is none.
//!
//! Five sources, tried in a fixed order, first success wins. The order is not a
//! preference list — each step is there because the step above it can be absent,
//! and each is strictly cheaper or strictly more correct than the one below.
//!
//! | rung | source | cost | why it is above the next one |
//! | --- | --- | --- | --- |
//! | 1 | the launcher's own artwork cache | one `read` | free, instant, offline, and it is the picture the user already associates with this game |
//! | 2 | the user's override | one `read` | *above everything*, including the cache — see below |
//! | 3 | the game's own files | a bounded read of the install | still offline, still exact, and the only source that exists for a game no service has heard of |
//! | 4 | a remote artwork service | a network round trip | the first rung that can fail slowly, be rate-limited, or be taken offline by somebody else |
//! | 5 | the generated plate | none | always succeeds, by construction |
//!
//! ## Why the override outranks the cache
//!
//! Rung 2 is listed second and evaluated *first*. That is not a contradiction:
//! the cascade's numbering describes where artwork comes from in the ordinary
//! case, and the override is the extraordinary case by definition. A user who
//! has set custom art has made an explicit decision about this one game, and a
//! cascade that let a launcher's cache win would silently undo it the next time
//! the launcher refreshed. [`Silsila::hall`] therefore checks
//! [`MartabatSura::Tajawuz`] before anything else and the table above says so
//! plainly rather than leaving the reader to notice.
//!
//! ## Why rung 3 is the one that must not be skipped
//!
//! It is the rung that carries niche and indie libraries. A game bought on
//! itch.io, a game from a defunct storefront, a game the artwork services have
//! never indexed — all of them have an executable with an icon in it, and most
//! have a splash image or an application icon shipped inside the install. That
//! is real artwork, it is already on the machine, and skipping to a generated
//! plate when it exists would make the product look worse than the launcher the
//! user came from.
//!
//! The rule that keeps it honest is [`Silsila::AQSA_TAKBIR`]: a 32-pixel icon
//! blown up to a 240-pixel card is a blurry mess, and a blurry mess is worse
//! than a clean plate. Anything that would need more than a doubling is refused
//! and the cascade continues.
//!
//! ## The plate is a designed state, not an error
//!
//! Rung 5 always succeeds. For a large indie library it will be most of the
//! grid, so it is designed to be looked at rather than to be replaced — see
//! [`crate::lawha_badila`]. Nothing in this module treats reaching rung 5 as a
//! failure, and nothing logs it as one.

use std::path::{Path, PathBuf};

use taarib_mustalahat::luba::MasdarLuba;

use crate::fahs::{MasadirSuwar, MasdarSura};

/// Which rung a picture came from.
///
/// Recorded on every resolved image, for two reasons that are not obvious. The
/// interface uses it to decide whether "change artwork" offers *revert to
/// automatic* — which only means something when the current picture is an
/// override. And the diagnostics bundle uses it to answer "why does this game
/// look like that", which is otherwise unanswerable after the fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MartabatSura {
    /// The user set this picture by hand.
    Tajawuz,
    /// Read out of a launcher's own artwork cache.
    MakhbaaMatjar,
    /// Extracted from the game's own files.
    MinAlLuba,
    /// Fetched from an artwork service.
    Khidma,
    /// Generated, because nothing else existed.
    Lawha,
}

impl MartabatSura {
    /// The rung's number in the cascade, one through five.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::MakhbaaMatjar => 1,
            Self::Tajawuz => 2,
            Self::MinAlLuba => 3,
            Self::Khidma => 4,
            Self::Lawha => 5,
        }
    }

    /// Whether this rung reached the network.
    ///
    /// Consulted by the offline-mode check and by the rate limiter, both of
    /// which need to know whether a resolution *would have* cost a request
    /// before they decide to allow it.
    #[must_use]
    pub const fn shabakiya(self) -> bool {
        matches!(self, Self::Khidma)
    }

    /// Whether the user can revert this picture to an automatic one.
    ///
    /// True only for an override, because reverting anything else would revert
    /// it to itself.
    #[must_use]
    pub const fn qabila_lil_irjaa(self) -> bool {
        matches!(self, Self::Tajawuz)
    }

    /// The name that appears in the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Tajawuz => "user override",
            Self::MakhbaaMatjar => "launcher cache",
            Self::MinAlLuba => "the game's own files",
            Self::Khidma => "artwork service",
            Self::Lawha => "generated plate",
        }
    }
}

/// Which of a game's three pictures is being resolved.
///
/// The cascade runs per picture rather than per game, because the rungs do not
/// agree about which pictures they can supply: a launcher cache usually has all
/// three, an executable icon is only ever a cover, and the plate is only ever a
/// cover. A game can therefore end with a cover from rung 3 and a banner from
/// rung 4, which is correct and is what the grid and the detail screen each want.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NawSura {
    /// The vertical cover the grid shows.
    Ghilaf,
    /// The wide banner behind the detail header.
    Batl,
    /// The small logo overlaid on the banner.
    Shiar,
}

impl NawSura {
    /// The name used in the cache key and the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Ghilaf => "cover",
            Self::Batl => "hero",
            Self::Shiar => "logo",
        }
    }

    /// The aspect ratio this picture is expected to have, as width over height.
    ///
    /// Returned as a pair rather than a float so the comparison that uses it can
    /// cross-multiply. `float_cmp` is denied workspace-wide and an aspect check
    /// written with `==` would be denied for a good reason: two ways of computing
    /// 0.666… do not agree.
    #[must_use]
    pub const fn nisba(self) -> (u32, u32) {
        match self {
            // 2:3, the standard vertical cover every storefront settled on.
            Self::Ghilaf => (2, 3),
            // 16:9, matching the header it sits behind.
            Self::Batl => (16, 9),
            // Logos have no fixed ratio; the value is the widest this build will
            // letterbox without cropping, and a logo outside it is used as-is.
            Self::Shiar => (5, 2),
        }
    }

    /// Whether a candidate's dimensions are close enough to this picture's shape.
    ///
    /// Twenty percent either way. Wide enough that a storefront's 300×450 and
    /// another's 342×482 both pass; narrow enough that a 16:9 banner never
    /// passes as a cover. Compared by cross-multiplication, never by dividing
    /// into a float.
    #[must_use]
    pub fn tunasib(self, ard: u32, irtifa: u32) -> bool {
        if ard == 0 || irtifa == 0 {
            return false;
        }
        let (n_ard, n_irtifa) = self.nisba();
        let faili = u64::from(ard).saturating_mul(u64::from(n_irtifa));
        let mutawaqqa = u64::from(irtifa).saturating_mul(u64::from(n_ard));
        let akbar = faili.max(mutawaqqa);
        let asghar = faili.min(mutawaqqa);
        // `asghar * 6 >= akbar * 5` is "the smaller is at least 5/6 of the
        // larger", which is the twenty percent band without a division.
        asghar.saturating_mul(6) >= akbar.saturating_mul(5)
    }
}

/// One candidate picture, before the cascade decides whether to take it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MurashahSura {
    /// Where it is.
    pub masdar: MasdarSura,
    /// Which rung produced it.
    pub martaba: MartabatSura,
    /// Its dimensions, when the producer knows them without decoding.
    ///
    /// [`None`] for a remote candidate, whose size is not known until it is
    /// fetched — which is exactly why remote is the second-to-last rung.
    pub abaad: Option<(u32, u32)>,
    /// What this candidate is, for the diagnostics bundle: `steam librarycache`,
    /// `PE icon group 1`, `splash.png`.
    pub wasf: String,
}

impl MurashahSura {
    /// A candidate whose dimensions are known.
    #[must_use]
    pub fn bi_abaad(
        masdar: MasdarSura,
        martaba: MartabatSura,
        ard: u32,
        irtifa: u32,
        wasf: impl Into<String>,
    ) -> Self {
        Self { masdar, martaba, abaad: Some((ard, irtifa)), wasf: wasf.into() }
    }

    /// A candidate whose dimensions are not known yet.
    #[must_use]
    pub fn bila_abaad(
        masdar: MasdarSura,
        martaba: MartabatSura,
        wasf: impl Into<String>,
    ) -> Self {
        Self { masdar, martaba, abaad: None, wasf: wasf.into() }
    }

    /// Whether this candidate is usable at the requested size.
    ///
    /// A candidate with unknown dimensions passes: it cannot be judged here and
    /// judging it as a failure would reject every remote candidate before it was
    /// ever fetched.
    #[must_use]
    pub fn kaf(&self, naw: NawSura, ard_matlub: u32) -> bool {
        let Some((ard, irtifa)) = self.abaad else {
            return true;
        };
        if ard == 0 || irtifa == 0 {
            return false;
        }
        if !naw.tunasib(ard, irtifa) && !matches!(naw, NawSura::Shiar) {
            return false;
        }
        // The upscale ceiling, and the reason rung 3 does not produce garbage.
        u64::from(ard).saturating_mul(u64::from(Silsila::AQSA_TAKBIR))
            >= u64::from(ard_matlub)
    }
}

/// A resolved picture, and the record of how it was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuraMahlula {
    /// Which picture this is.
    pub naw: NawSura,
    /// Where it came from.
    pub masdar: MasdarSura,
    /// Which rung supplied it.
    pub martaba: MartabatSura,
    /// Every rung that was tried and what it said, oldest first.
    ///
    /// Kept even on success, because "the cover came from the executable's icon
    /// because Steam's cache had a placeholder" is a sentence somebody will need
    /// and cannot reconstruct later.
    pub athar: Vec<String>,
}

/// A game's artwork the user set by hand.
///
/// Stored per game rather than per picture-type-per-game so that a user who
/// overrides a cover does not implicitly clear the automatic banner. The three
/// fields are independent and any combination is legitimate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TajawuzSuwar {
    /// A cover the user chose.
    pub ghilaf: Option<PathBuf>,
    /// A banner the user chose.
    pub batl: Option<PathBuf>,
    /// A logo the user chose.
    pub shiar: Option<PathBuf>,
}

impl TajawuzSuwar {
    /// The override for one picture, if there is one.
    #[must_use]
    pub fn li(&self, naw: NawSura) -> Option<&Path> {
        match naw {
            NawSura::Ghilaf => self.ghilaf.as_deref(),
            NawSura::Batl => self.batl.as_deref(),
            NawSura::Shiar => self.shiar.as_deref(),
        }
    }

    /// Whether the user has overridden anything at all.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.ghilaf.is_none() && self.batl.is_none() && self.shiar.is_none()
    }
}

/// Everything the cascade needs to resolve one game's pictures.
#[derive(Debug, Clone)]
pub struct TalabSilsila<'a> {
    /// Which game.
    pub masdar: &'a MasdarLuba,
    /// The game's install root, for rung 3.
    pub jidhr: &'a Path,
    /// The game's executable, for rung 3's icon extraction.
    pub tanfidhi: Option<&'a Path>,
    /// What the launcher's own catalogue said, which is rung 1 and part of 4.
    pub min_matjar: &'a MasadirSuwar,
    /// What the user set, which is rung 2.
    pub tajawuz: &'a TajawuzSuwar,
    /// The width the grid will draw at, which decides the upscale check.
    pub ard_matlub: u32,
    /// Whether rung 4 may be attempted at all.
    ///
    /// False in offline mode, false while the rate limiter is saturated, and
    /// false during the first paint — the grid shows plates and fills in
    /// afterwards, and a cascade that reached the network on the first pass
    /// would be a cascade that blocked the first paint.
    pub yasmah_bil_shabaka: bool,
}

/// The cascade.
///
/// Holds no state. It is a decision procedure over candidates that its caller
/// has already gathered, which is what lets rung 3's expensive extraction be
/// skipped entirely when rung 1 succeeded — the caller consults
/// [`Silsila::hall`] with the cheap rungs first and only gathers the expensive
/// ones if it returns [`None`].
#[derive(Debug, Clone, Copy)]
pub struct Silsila;

impl Silsila {
    /// How far a picture may be scaled up before the plate is preferred.
    ///
    /// Two. A 120-pixel icon on a 240-pixel card is soft but legible; a 32-pixel
    /// one is a smear, and a smear next to real artwork looks like a defect
    /// rather than like a small icon. The number is the whole reason rung 3 can
    /// be aggressive about finding *something* without making the grid worse.
    pub const AQSA_TAKBIR: u32 = 2;

    /// Picks the best candidate for one picture, or [`None`] to fall through.
    ///
    /// Candidates are considered in rung order, and within a rung in the order
    /// the caller supplied — which for rung 3 means the caller has already
    /// sorted by size, largest first, so the biggest usable icon wins.
    ///
    /// An override is taken **without** the size check. A user who chose a small
    /// picture chose it, and second-guessing them would make the override a
    /// suggestion rather than a decision.
    #[must_use]
    pub fn hall(
        naw: NawSura,
        murashahat: &[MurashahSura],
        ard_matlub: u32,
    ) -> Option<SuraMahlula> {
        let mut athar: Vec<String> = Vec::with_capacity(murashahat.len());

        if let Some(tajawuz) =
            murashahat.iter().find(|q| matches!(q.martaba, MartabatSura::Tajawuz))
        {
            athar.push(format!("{}: taken (user override)", tajawuz.wasf));
            return Some(SuraMahlula {
                naw,
                masdar: tajawuz.masdar.clone(),
                martaba: tajawuz.martaba,
                athar,
            });
        }

        let mut murattaba: Vec<&MurashahSura> = murashahat.iter().collect();
        // Stable, so the caller's own ordering within a rung is preserved.
        murattaba.sort_by_key(|q| q.martaba.raqm());

        for murashah in murattaba {
            if matches!(murashah.martaba, MartabatSura::Tajawuz) {
                continue;
            }
            if murashah.kaf(naw, ard_matlub) {
                athar.push(format!("{}: taken", murashah.wasf));
                return Some(SuraMahlula {
                    naw,
                    masdar: murashah.masdar.clone(),
                    martaba: murashah.martaba,
                    athar,
                });
            }
            athar.push(match murashah.abaad {
                Some((ard, irtifa)) => format!(
                    "{}: {ard}×{irtifa}, rejected — it would need more than a {}× enlargement \
                     to fill a {ard_matlub}px card, or its shape is not a {}",
                    murashah.wasf,
                    Self::AQSA_TAKBIR,
                    naw.ism()
                ),
                None => format!("{}: rejected", murashah.wasf),
            });
        }
        None
    }

    /// Resolves all three pictures from one candidate set.
    ///
    /// Returns what was resolved and the trail for what was not. A picture with
    /// no candidate is absent from the map rather than present-and-empty, so a
    /// caller cannot mistake "no cover" for "a cover that failed to load".
    #[must_use]
    pub fn hall_al_kul(
        talab: &TalabSilsila<'_>,
        murashahat: &[(NawSura, Vec<MurashahSura>)],
    ) -> (Vec<SuraMahlula>, Vec<String>) {
        let mut mahlula = Vec::with_capacity(murashahat.len());
        let mut athar = Vec::new();
        for (naw, majmua) in murashahat {
            match Self::hall(*naw, majmua, talab.ard_matlub) {
                Some(sura) => {
                    athar.extend(sura.athar.iter().map(|satr| format!("{}: {satr}", naw.ism())));
                    mahlula.push(sura);
                }
                None => athar.push(format!(
                    "{}: no candidate passed; the plate is used",
                    naw.ism()
                )),
            }
        }
        (mahlula, athar)
    }
}

/// The candidates rung 1 offers, from what the launcher's catalogue recorded.
///
/// Trivial by design. The adapters already did this work during the scan and
/// recorded it in [`MasadirSuwar`]; re-deriving it here would be a second answer
/// to a question that already has one, and the two would drift.
#[must_use]
pub fn murashahat_matjar(min_matjar: &MasadirSuwar) -> Vec<(NawSura, Vec<MurashahSura>)> {
    let mut kul = Vec::with_capacity(3);
    for (naw, masdar) in [
        (NawSura::Ghilaf, min_matjar.ghilaf.as_ref()),
        (NawSura::Batl, min_matjar.batl.as_ref()),
        (NawSura::Shiar, min_matjar.shiar.as_ref()),
    ] {
        let mut majmua = Vec::new();
        if let Some(masdar) = masdar {
            // A launcher's local file is rung 1. A launcher's *URL* is rung 4,
            // because reaching it costs a request exactly as an artwork
            // service's does — the fact that the launcher published it does not
            // make it free.
            let (martaba, wasf) = match masdar {
                MasdarSura::Malaf(masar) => (
                    MartabatSura::MakhbaaMatjar,
                    format!("launcher cache: {}", masar.display()),
                ),
                MasdarSura::Rabt(rabt) => {
                    (MartabatSura::Khidma, format!("launcher endpoint: {rabt}"))
                }
            };
            majmua.push(MurashahSura::bila_abaad(masdar.clone(), martaba, wasf));
        }
        kul.push((naw, majmua));
    }
    kul
}

/// The candidates rung 2 offers.
#[must_use]
pub fn murashahat_tajawuz(tajawuz: &TajawuzSuwar) -> Vec<(NawSura, Vec<MurashahSura>)> {
    [NawSura::Ghilaf, NawSura::Batl, NawSura::Shiar]
        .into_iter()
        .map(|naw| {
            let majmua = tajawuz.li(naw).map_or_else(Vec::new, |masar| {
                vec![MurashahSura::bila_abaad(
                    MasdarSura::Malaf(masar.to_path_buf()),
                    MartabatSura::Tajawuz,
                    format!("user override: {}", masar.display()),
                )]
            });
            (naw, majmua)
        })
        .collect()
}

/// Folds several candidate sets into one, preserving each set's internal order.
///
/// The caller builds rung 1's set, rung 2's set, and — only if those did not
/// settle it — rung 3's, then merges here. Merging rather than concatenating
/// matters because [`Silsila::hall`] sorts by rung and needs every candidate for
/// one picture in one list.
#[must_use]
pub fn admij(
    majmuat: Vec<Vec<(NawSura, Vec<MurashahSura>)>>,
) -> Vec<(NawSura, Vec<MurashahSura>)> {
    let mut kul: Vec<(NawSura, Vec<MurashahSura>)> =
        [NawSura::Ghilaf, NawSura::Batl, NawSura::Shiar]
            .into_iter()
            .map(|naw| (naw, Vec::new()))
            .collect();

    for majmua in majmuat {
        for (naw, murashahat) in majmua {
            if let Some((_, hadaf)) = kul.iter_mut().find(|(mawjud, _)| *mawjud == naw) {
                hadaf.extend(murashahat);
            }
        }
    }
    kul
}
