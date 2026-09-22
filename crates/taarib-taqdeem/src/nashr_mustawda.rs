//! النشر إلى المستودع — putting an approved package where another machine can install it.
//!
//! Approval seals a package with the owner's key and records it here. It does
//! not reach the registry: that is this module, and it is a separate act
//! because it needs a network, a forge token, and a push that can be refused
//! halfway. Folding the two would give `Manshura` two meanings — "in the
//! catalogue" on a machine that was online and "in a folder" on one that was
//! not — and the whole defect this exists to close is a state that claimed the
//! first while meaning the second.
//!
//! The cast itself is `taarib_mustawda::sabk`, unchanged and shared with the
//! `sabk` command, because a registry written two ways is a registry whose two
//! writers drift. What is here is everything around it: the ledger of what the
//! owner has approved, the checkout the cast is written into, the sequence
//! number the next cast takes, and the push.
//!
//! Nothing in this module signs a package. [`crate::nashr::waqqi`] does that at
//! approval, with the owner's key, and the bytes are carried here untouched;
//! the only signature made here is the revocation list's, through
//! `taarib_aman::qaimat_sahb::KatibQaima`, which is the same writer the `sabk`
//! command uses.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_aman::qaimat_sahb::QaimatSahb;
use taarib_khatm::MiftahKhass;
use taarib_mustalahat::khariji::RuqaaKharijiya;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_mustawda::fahras::BayanMustawda;
use taarib_mustawda::masadir::MASAR_BAYAN;
use taarib_mustawda::sabk::{
    KhiyaratSabk, MASAR_QAIMAT_SAHB, MadkhalKhariji, MadkhalManshur, Mulghayat, Mustawda, ijri,
    madkhal_khariji, madkhal_min_huzma,
};
use taarib_usus::masarat::{self, Masarat};

use crate::bawwaba::ifhas_khariji;
use crate::hawiya::SalahiyatMalik;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};

/// Where the publication ledger lives, under the data root.
pub const MASAR_SIJILL_NASHR: &str = "manshurat/sijill.json";

/// Where the registry working tree is kept, under the data root.
pub const MASAR_NUSKHAT_MUSTAWDA: &str = "manshurat/mustawda";

/// Which step of a registry publish a run reached.
///
/// Every one of them leaves the ledger untouched: an entry becomes published
/// only after the push returns, so a failure at any step here is a submission
/// that still says it is approved and not published, which is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarhalatNashrMustawda {
    /// The local registry checkout is being opened.
    Fath,
    /// The registry's current state is being fetched.
    Jalb,
    /// The catalogue is being cast into the checkout.
    Sabk,
    /// The cast is being committed.
    Iltizam,
    /// The commit is being pushed.
    Dafa,
}

impl MarhalatNashrMustawda {
    /// The step's name, for a report a person reads.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Fath => "opening the local registry checkout",
            Self::Jalb => "fetching the registry",
            Self::Sabk => "casting the catalogue",
            Self::Iltizam => "committing the cast",
            Self::Dafa => "pushing to the registry",
        }
    }
}

fn fashil(marhala: MarhalatNashrMustawda, sabab: impl Into<String>) -> KhataTaqdeem {
    KhataTaqdeem::NashrMustawdaFashil {
        marhala: marhala.ism(),
        sabab: sabab.into(),
    }
}

fn khata_git(marhala: MarhalatNashrMustawda, khata: &git2::Error) -> KhataTaqdeem {
    fashil(marhala, khata.message())
}

/// One approved package, and whether the registry has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MadkhalNashr {
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision that was approved.
    pub murajaa: RuqaaRevision,
    /// The game it targets, which decides the shard and is not recoverable
    /// from the sealed package.
    pub luba: LubaId,
    /// The game's title, which the asset's file name is slugged from. Recorded
    /// once so the address a listing carries never moves under a renamed game.
    pub ism_luba: String,
    /// Who made the patch.
    ///
    /// Recorded because approval re-seals the package with the owner's key, and
    /// after that the file itself no longer says. A catalogue built from the
    /// seal alone would credit the owner for everybody's work.
    pub musahim: MusahimId,
    /// The sealed package's file name inside the publications directory.
    pub ism_malaf: String,
    /// When the owner approved it, RFC 3339.
    pub waqt_iaatimad: String,
    /// The manifest sequence this entry reached the registry at, and [`None`]
    /// while it is approved and not yet cast.
    #[serde(default)]
    pub tasalsul: Option<u64>,
    /// The owner's written reason for publishing this package over its own
    /// coverage gate's refusal, and [`None`] when the gate permits it.
    ///
    /// Approval and the coverage gate are two different questions. The owner
    /// approves a translation; the gate asks whether the package measured
    /// enough of the game to be worth serving, and a patch whose opening was
    /// never captured refuses itself. Without somewhere to put the answer, a
    /// package the owner has approved and the gate refuses can never be
    /// published at all — and the reason is carried into the manifest, where
    /// every reader of the catalogue sees it.
    #[serde(default)]
    pub tajawuz: Option<String>,
}

impl MadkhalNashr {
    /// Whether this entry is in the registry rather than only on this machine.
    #[must_use]
    pub const fn manshura(&self) -> bool {
        self.tasalsul.is_some()
    }
}

/// One lineage the owner pulled, kept so every cast re-states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulghaNashr {
    /// The lineage.
    pub ruqaa: RuqaaId,
    /// Why, as the owner wrote it.
    pub sabab: String,
    /// When, RFC 3339.
    pub waqt: String,
}

/// Everything the owner has approved, and everything they have pulled.
///
/// The single source of truth for a cast. The registry is rebuilt from it in
/// full on every publish — all 256 shards and the whole revocation list — so an
/// entry missing from here is an entry missing from the catalogue, and a
/// revocation missing from here is one that stops being in force.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillNashr {
    #[serde(default)]
    madakhil: BTreeMap<RuqaaId, MadkhalNashr>,
    #[serde(default)]
    mulghayat: Vec<MulghaNashr>,
    /// Third-party entries the owner listed: patches somebody else made, which
    /// Taarib never built and never sealed.
    ///
    /// In the ledger for the reason everything else is: the cast rebuilds all
    /// 256 shards from it in full, so an entry the ledger does not hold is an
    /// entry the next cast deletes from the catalogue. They are kept whole
    /// rather than as a reference to a package, because there is no package —
    /// the index is the only place one of these exists.
    #[serde(default)]
    kharijiya: BTreeMap<RuqaaId, RuqaaKharijiya>,
}

impl SijillNashr {
    /// Where the ledger lives under Taarib's data root.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the path cannot be built inside the
    /// data root.
    pub fn masar(masarat: &Masarat) -> NatijatTaqdeem<PathBuf> {
        masarat::dakhil(masarat.jidhr_bayanat(), MASAR_SIJILL_NASHR).map_err(|khata| {
            KhataTaqdeem::KhataMalaf {
                masar: masarat.jidhr_bayanat().to_path_buf(),
                amal: "locating the publication ledger",
                sabab: std::io::Error::other(khata.injilizi),
            }
        })
    }

    /// Reads the ledger, treating an absent file as an empty one.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the file exists and cannot be read or
    /// does not parse — which is a named failure and never a fresh empty
    /// ledger, because an empty one casts a catalogue with nothing in it.
    pub fn iftah(masar: &Path) -> NatijatTaqdeem<Self> {
        let bayt = match fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            },
            Err(sabab) => {
                return Err(KhataTaqdeem::KhataMalaf {
                    masar: masar.to_path_buf(),
                    amal: "reading the publication ledger",
                    sabab,
                });
            },
        };
        serde_json::from_slice(&bayt).map_err(|khata| KhataTaqdeem::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "reading the publication ledger",
            sabab: std::io::Error::other(khata.to_string()),
        })
    }

    /// Writes the ledger atomically.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when it will not serialize or the write
    /// fails.
    pub fn ihfadh(&self, masar: &Path) -> NatijatTaqdeem<()> {
        let bayt = serde_json::to_vec_pretty(self).map_err(|khata| KhataTaqdeem::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "writing the publication ledger",
            sabab: std::io::Error::other(khata.to_string()),
        })?;
        masarat::kitaba_dharra(masar, &bayt).map_err(|khata| KhataTaqdeem::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "writing the publication ledger",
            sabab: std::io::Error::other(khata.injilizi),
        })
    }

    /// Records an approval, replacing any earlier revision of the same lineage.
    ///
    /// The replacement resets `tasalsul`: a new revision is not in the registry
    /// until the next cast puts it there, whatever the one it supersedes was.
    pub fn sajjil(&mut self, madkhal: MadkhalNashr) {
        let _ = self.madakhil.insert(madkhal.ruqaa, madkhal);
    }

    /// Records the owner's reason for publishing an entry over its own
    /// coverage gate's refusal.
    ///
    /// A blank sentence records nothing: an override with no reason is the
    /// gate switched off, which is the one thing `tajawuz` is not.
    pub fn tajawiz(&mut self, ruqaa: RuqaaId, sabab: &str) -> bool {
        let Some(madkhal) = self.madakhil.get_mut(&ruqaa) else {
            return false;
        };
        if sabab.trim().is_empty() {
            return false;
        }
        madkhal.tajawuz = Some(sabab.to_owned());
        true
    }

    /// Marks an entry as having reached the registry at `tasalsul`.
    pub fn nushirat(&mut self, ruqaa: RuqaaId, tasalsul: u64) {
        if let Some(madkhal) = self.madakhil.get_mut(&ruqaa) {
            madkhal.tasalsul = Some(tasalsul);
        }
    }

    /// Pulls a lineage: it leaves the catalogue and joins the revocation list.
    pub fn ilghi(&mut self, ruqaa: RuqaaId, sabab: &str, waqt: &str) {
        let _ = self.madakhil.remove(&ruqaa);
        let _ = self.kharijiya.remove(&ruqaa);
        self.mulghayat.retain(|mulgha| mulgha.ruqaa != ruqaa);
        self.mulghayat.push(MulghaNashr {
            ruqaa,
            sabab: sabab.to_owned(),
            waqt: waqt.to_owned(),
        });
    }

    /// Lists a third-party entry, replacing any earlier version of it.
    ///
    /// Runs the gate rather than trusting the caller, because this is where a
    /// whole finished work somebody else made enters a catalogue that publishes
    /// to strangers. The rules are [`ifhas_khariji`]'s, which are
    /// `taarib_aman::kharijiya`'s, which the pre-install gate applies on the
    /// other end; nothing is restated here.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::BawwabaMaghlaqa`] naming what the gate refused —
    /// permission nobody recorded, or a mirror the permission does not cover.
    pub fn sajjil_khariji(&mut self, madkhal: RuqaaKharijiya) -> NatijatTaqdeem<()> {
        let qaima = ifhas_khariji(&madkhal);
        let rasiba = qaima.rasiba();
        if !rasiba.is_empty() {
            return Err(KhataTaqdeem::BawwabaMaghlaqa {
                adad: rasiba.len(),
                amthila: rasiba
                    .iter()
                    .map(|rasib| {
                        format!("{}: {}", rasib.band.wasf_injilizi(), rasib.tafsil_injilizi)
                    })
                    .collect(),
            });
        }
        let _ = self.kharijiya.insert(madkhal.id, madkhal);
        Ok(())
    }

    /// Every third-party entry the owner has listed.
    pub fn kharijiya(&self) -> impl Iterator<Item = &RuqaaKharijiya> {
        self.kharijiya.values()
    }

    /// One third-party entry.
    #[must_use]
    pub fn khariji(&self, ruqaa: RuqaaId) -> Option<&RuqaaKharijiya> {
        self.kharijiya.get(&ruqaa)
    }

    /// Every approved package, oldest lineage first.
    pub fn madakhil(&self) -> impl Iterator<Item = &MadkhalNashr> {
        self.madakhil.values()
    }

    /// One entry.
    #[must_use]
    pub fn madkhal(&self, ruqaa: RuqaaId) -> Option<&MadkhalNashr> {
        self.madakhil.get(&ruqaa)
    }

    /// Every entry the registry does not have yet.
    #[must_use]
    pub fn muaallaqa(&self) -> Vec<&MadkhalNashr> {
        self.madakhil
            .values()
            .filter(|madkhal| !madkhal.manshura())
            .collect()
    }

    /// Every revocation still in force.
    pub fn mulghayat(&self) -> impl Iterator<Item = &MulghaNashr> {
        self.mulghayat.iter()
    }

    /// The revocations as the cast wants them.
    #[must_use]
    pub fn mulghayat_lil_sabk(&self) -> Mulghayat {
        Mulghayat {
            mafatih: Vec::new(),
            ruqa: self
                .mulghayat
                .iter()
                .map(|mulgha| (mulgha.ruqaa, mulgha.sabab.clone()))
                .collect(),
        }
    }
}

/// Where the registry is, and who is pushing to it.
pub struct TalabNashrMustawda<'a> {
    /// The registry working tree on this machine.
    pub jidhr: &'a Path,
    /// Where the sealed packages are.
    pub mujallad_manshurat: &'a Path,
    /// The registry's https clone address.
    pub rabt_git: &'a str,
    /// The branch the catalogue is served from.
    pub far: &'a str,
    /// The base every asset address is resolved against — the registry root a
    /// client reads, so that the listing's `rabt` is the address that client
    /// will actually fetch.
    pub asas_rabt: &'a str,
    /// The forge account name, for the credential callback.
    pub login: &'a str,
    /// The forge access token.
    pub sirr: &'a str,
    /// The name the commit is authored under.
    pub ism_musahim: &'a str,
    /// The address the commit is authored under.
    pub barid: &'a str,
    /// When the cast happened, RFC 3339.
    pub waqt: &'a str,
}

impl std::fmt::Debug for TalabNashrMustawda<'_> {
    /// Prints where the cast is going and never the access token.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabNashrMustawda")
            .field("jidhr", &self.jidhr)
            .field("rabt_git", &self.rabt_git)
            .field("far", &self.far)
            .field("asas_rabt", &self.asas_rabt)
            .field("login", &self.login)
            .finish_non_exhaustive()
    }
}

/// One entry the cast would not take, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalMarfud {
    /// The lineage.
    pub ruqaa: RuqaaId,
    /// What the cast said about it.
    pub sabab: String,
}

/// What a finished publish put in the registry.
#[derive(Debug, Clone)]
pub struct NatijatNashrMustawda {
    /// The manifest sequence the catalogue now sits at.
    pub tasalsul: u64,
    /// The commit that was pushed.
    pub iltizam: String,
    /// The branch it was pushed to.
    pub far: String,
    /// Every listing now in the catalogue, as the cast derived it from each
    /// sealed package — including the `rabt` a client will fetch it from.
    pub fahras: Vec<MulakhkhasRuqaa>,
    /// Every third-party entry now in the catalogue, kept apart from the
    /// listings above because they are not the same kind of thing: nothing
    /// here was compiled, sealed, or certified as carrying no byte of the game.
    pub kharijiya: Vec<RuqaaKharijiya>,
    /// Every entry the cast refused, each still approved and still unpublished.
    pub marfuda: Vec<MadkhalMarfud>,
    /// How many revocations the pushed list carries.
    pub adad_mulghayat: usize,
    /// The checkout the cast was written into, so a refused push can be
    /// finished by hand from a directory the operator can see.
    pub jidhr: PathBuf,
}

/// Casts the owner's whole catalogue into the registry and pushes it.
///
/// The sequence, and what each step guarantees:
///
/// 1. The checkout is opened, the registry's branch fetched, and the working
///    tree reset to it. Every cast therefore starts from what is actually
///    served, not from whatever a previous run left behind.
/// 2. The sequence number is taken from the served manifest and the served
///    revocation list, and the cast is one past the higher of them. A client
///    caches on that number, so a repeat publish that reused it would leave
///    every client on the old catalogue.
/// 3. The catalogue is cast from the ledger in full. An entry the cast refuses
///    is skipped and reported — unless the registry already carries it, in
///    which case skipping would delete a live listing, so the whole run stops.
/// 4. The result is committed and pushed without force. A registry someone else
///    moved in the meantime refuses the push rather than losing their work, and
///    the next run re-fetches and recomputes the sequence.
///
/// The caller marks the ledger published from the result, after this returns.
/// Nothing here writes the ledger, so a failure at any step leaves every
/// submission saying exactly what is true of it.
///
/// # Errors
///
/// [`KhataTaqdeem::NashrFashil`] when the signing key is not the owner key this
/// session proved, and [`KhataTaqdeem::NashrMustawdaFashil`] naming the step
/// for everything else: a checkout that will not open, a fetch or a push the
/// forge refused, a manifest that will not read, a cast that refused a package
/// already in the catalogue.
pub fn unshur(
    salahiya: &SalahiyatMalik,
    khass: &MiftahKhass,
    sijill: &SijillNashr,
    talab: &TalabNashrMustawda<'_>,
) -> NatijatTaqdeem<NatijatNashrMustawda> {
    // The same rule `waqqi` applies to the package: the key that casts the
    // catalogue is the owner key this session proved, not whichever key the
    // keychain happened to hand back.
    if &khass.aam().bayt() != salahiya.miftah_aam() {
        return Err(KhataTaqdeem::NashrFashil {
            marhala: crate::nashr::MarhalatNashr::Wuqqia.ism(),
            sabab: "the signing key is not the owner key this session proved".to_owned(),
        });
    }

    let mustawda = iftah_nuskha(talab)?;
    let asas = jalb_far(&mustawda, talab)?;
    nazzif(&mustawda, talab.jidhr)?;

    let tasalsul = tasalsul_talia(talab.jidhr, khass)?;
    let (madakhil, mut marfuda) = iqra_madakhil(sijill, talab)?;
    let (kharijiya, marfuda_kharijiya) = iqra_kharijiya(sijill);
    marfuda.extend(marfuda_kharijiya);
    let mulghayat = sijill.mulghayat_lil_sabk();

    // Everything the ledger holds was refused and there is nothing else to
    // serve. Casting anyway would push an empty catalogue at a fresh sequence
    // and send every client to refetch nothing, then report a publish that
    // published no patch.
    if madakhil.is_empty() && kharijiya.is_empty() && !marfuda.is_empty() && mulghayat.adad() == 0 {
        let asbab: Vec<String> = marfuda
            .iter()
            .map(|marfud| format!("{}: {}", marfud.ruqaa, marfud.sabab))
            .collect();
        return Err(fashil(
            MarhalatNashrMustawda::Sabk,
            format!(
                "every approved package was refused by the cast, so nothing would be \
                 published:\n    {}",
                asbab.join("\n    ")
            ),
        ));
    }

    let natija = ijri(
        talab.jidhr,
        madakhil,
        BTreeMap::new(),
        kharijiya,
        KhiyaratSabk {
            tasalsul,
            waqt: talab.waqt,
            // The floor is the served list's own count, read back above and
            // carried here so a rebuilt list can never quietly shed one.
            adna_ilghaat: adad_ilghaat_manshura(talab.jidhr, khass)?,
        },
        khass,
        &mulghayat,
    )
    .map_err(|sabab| fashil(MarhalatNashrMustawda::Sabk, sabab))?;

    let risala = risalat_iltizam(tasalsul, &natija, mulghayat.adad());
    let iltizam = iltazim(&mustawda, asas, talab, &risala)?;
    idfa(&mustawda, talab)?;

    tracing::info!(
        tasalsul,
        iltizam = %iltizam,
        adad = natija.madakhil.len(),
        adad_khariji = natija.kharijiya.len(),
        "the catalogue was cast and pushed to the registry"
    );

    Ok(NatijatNashrMustawda {
        tasalsul,
        iltizam,
        far: talab.far.to_owned(),
        fahras: natija
            .madakhil
            .iter()
            .map(|madkhal| madkhal.mulakhkhas.clone())
            .collect(),
        kharijiya: natija
            .kharijiya
            .iter()
            .map(|madkhal| madkhal.madkhal().clone())
            .collect(),
        marfuda,
        adad_mulghayat: mulghayat.adad(),
        jidhr: talab.jidhr.to_path_buf(),
    })
}

/// Opens the registry working tree, creating an empty one the first time.
fn iftah_nuskha(talab: &TalabNashrMustawda<'_>) -> NatijatTaqdeem<git2::Repository> {
    masarat::insha_mujallad(talab.jidhr).map_err(|khata| KhataTaqdeem::KhataMalaf {
        masar: talab.jidhr.to_path_buf(),
        amal: "creating the local registry checkout",
        sabab: std::io::Error::other(khata.injilizi),
    })?;
    git2::Repository::open(talab.jidhr)
        .or_else(|_| git2::Repository::init(talab.jidhr))
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Fath, &khata))
}

/// Fetches the served branch and moves the working tree onto it.
///
/// Returns the commit the catalogue is being built on top of, and [`None`] when
/// the registry has no branch yet — a repository nobody has pushed to, whose
/// first cast is its first commit.
fn jalb_far(
    mustawda: &git2::Repository,
    talab: &TalabNashrMustawda<'_>,
) -> NatijatTaqdeem<Option<git2::Oid>> {
    let marji_baid = format!("refs/remotes/nashr/{}", talab.far);
    let mut jalb = git2::FetchOptions::new();
    jalb.remote_callbacks(nida(talab.login, talab.sirr));
    jalb.download_tags(git2::AutotagOption::None);
    // Every head rather than the one branch: naming a branch that does not
    // exist is an error, and a registry nobody has pushed to yet has no
    // branches at all. The wildcard makes "empty repository" a fetch that
    // brings back nothing instead of a failure with a confusing sentence.
    let khariita = "+refs/heads/*:refs/remotes/nashr/*";
    mustawda
        .remote_anonymous(talab.rabt_git)
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?
        .fetch(&[khariita], Some(&mut jalb), None)
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?;

    let marji_mahalli = format!("refs/heads/{}", talab.far);
    let Ok(baid) = mustawda
        .find_reference(&marji_baid)
        .and_then(|marji| marji.peel_to_commit())
    else {
        // An empty registry: point HEAD at the branch that does not exist yet
        // so the first commit creates it.
        mustawda
            .set_head(&marji_mahalli)
            .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?;
        return Ok(None);
    };

    let _ = mustawda
        .reference(&marji_mahalli, baid.id(), true, "taarib nashr")
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?;
    mustawda
        .set_head(&marji_mahalli)
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?;
    mustawda
        .reset(baid.as_object(), git2::ResetType::Hard, None)
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?;
    Ok(Some(baid.id()))
}

/// Removes whatever an interrupted cast left untracked in the working tree.
///
/// A hard reset restores every tracked file and touches no untracked one, so a
/// run that failed after writing an asset would leave that asset behind and the
/// next commit would carry a file no listing names.
fn nazzif(mustawda: &git2::Repository, jidhr: &Path) -> NatijatTaqdeem<()> {
    let mut khiyarat = git2::StatusOptions::new();
    let _ = khiyarat
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let halat = mustawda
        .statuses(Some(&mut khiyarat))
        .map_err(|khata| khata_git(MarhalatNashrMustawda::Jalb, &khata))?;
    for hala in halat.iter() {
        if hala.status().contains(git2::Status::WT_NEW)
            && let Ok(nisbi) = hala.path()
        {
            let masar = jidhr.join(nisbi);
            fs::remove_file(&masar).map_err(|sabab| KhataTaqdeem::KhataMalaf {
                masar,
                amal: "clearing the local registry checkout",
                sabab,
            })?;
        }
    }
    Ok(())
}

/// The sequence this cast publishes at.
///
/// One past the higher of the served manifest's and the served revocation
/// list's, because the cast writes both at the same number and both are
/// compared monotonically by a client: a manifest that went backwards is
/// ignored, and a revocation list that went backwards is a replay.
fn tasalsul_talia(jidhr: &Path, khass: &MiftahKhass) -> NatijatTaqdeem<u64> {
    let bayan = iqra_bayan(jidhr)?.map_or(0, |bayan| bayan.tasalsul);
    let qaima = iqra_qaima(jidhr, khass)?.map_or(0, |qaima| qaima.tasalsul());
    Ok(bayan.max(qaima).saturating_add(1))
}

/// How many revocations the served list already carries.
fn adad_ilghaat_manshura(jidhr: &Path, khass: &MiftahKhass) -> NatijatTaqdeem<usize> {
    Ok(iqra_qaima(jidhr, khass)?.map_or(0, |qaima| qaima.adad()))
}

/// The served manifest, or [`None`] when the registry has none yet.
fn iqra_bayan(jidhr: &Path) -> NatijatTaqdeem<Option<BayanMustawda>> {
    let masar = jidhr.join(MASAR_BAYAN);
    let bayt = match fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(sabab) => {
            return Err(KhataTaqdeem::KhataMalaf {
                masar,
                amal: "reading the served manifest",
                sabab,
            });
        },
    };
    // A manifest that exists and will not read is not a reason to start the
    // sequence over: doing that publishes a catalogue every client refuses as
    // older than the one it holds.
    serde_json::from_slice(&bayt).map(Some).map_err(|khata| {
        fashil(
            MarhalatNashrMustawda::Sabk,
            format!(
                "{} is the registry's own manifest and would not read: {khata}",
                masar.display()
            ),
        )
    })
}

/// The served revocation list, verified against the owner key.
fn iqra_qaima(jidhr: &Path, khass: &MiftahKhass) -> NatijatTaqdeem<Option<QaimatSahb>> {
    let masar = jidhr.join(MASAR_QAIMAT_SAHB);
    let bayt = match fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(sabab) => {
            return Err(KhataTaqdeem::KhataMalaf {
                masar,
                amal: "reading the served revocation list",
                sabab,
            });
        },
    };
    // Verified rather than parsed: a served list this key did not sign is a
    // registry that is not this owner's, and casting over it would replace a
    // document every client checks against an anchor with one signed by
    // somebody else's key.
    QaimatSahb::min_bayt(&bayt, &khass.aam())
        .map(Some)
        .map_err(|khata| {
            fashil(
                MarhalatNashrMustawda::Sabk,
                format!(
                    "{} is served by the registry and does not verify against this owner key: \
                     {khata}",
                    masar.display()
                ),
            )
        })
}

/// Turns every ledger entry into a listing, collecting the ones the cast
/// refuses.
///
/// A refusal over an entry the registry already carries stops the run: the cast
/// rebuilds every shard from what it is given, so skipping a published entry
/// would delete its listing from the catalogue and leave the people who have it
/// installed with a patch the registry no longer knows about.
fn iqra_madakhil(
    sijill: &SijillNashr,
    talab: &TalabNashrMustawda<'_>,
) -> NatijatTaqdeem<(Vec<MadkhalManshur>, Vec<MadkhalMarfud>)> {
    let mut madakhil = Vec::new();
    let mut marfuda = Vec::new();
    for madkhal in sijill.madakhil() {
        let masar = talab.mujallad_manshurat.join(&madkhal.ism_malaf);
        let natija = madkhal_min_huzma(
            &masar,
            madkhal.luba,
            &madkhal.ism_luba,
            talab.jidhr,
            talab.asas_rabt,
            None,
            madkhal.tajawuz.as_deref(),
            Some(madkhal.musahim.clone()),
        );
        match natija {
            Ok(qaid) => madakhil.push(qaid),
            Err(sabab) if madkhal.manshura() => {
                return Err(fashil(
                    MarhalatNashrMustawda::Sabk,
                    format!(
                        "{} is already in the catalogue and this cast would drop it: {sabab}",
                        madkhal.ruqaa
                    ),
                ));
            },
            Err(sabab) => marfuda.push(MadkhalMarfud {
                ruqaa: madkhal.ruqaa,
                sabab,
            }),
        }
    }
    Ok((madakhil, marfuda))
}

/// Turns every listed third-party entry into one the cast will take,
/// collecting the ones it refuses.
///
/// No equivalent of `iqra_madakhil`'s "already in the catalogue" stop, because
/// there is nothing to compare against: a third-party entry lives only in the
/// index, so the served catalogue carries no record of it independent of this
/// ledger. What keeps one from vanishing is that nothing removes it from the
/// ledger except [`SijillNashr::ilghi`], and that the gate it is refused by is
/// the same one it passed on the way in.
fn iqra_kharijiya(sijill: &SijillNashr) -> (Vec<MadkhalKhariji>, Vec<MadkhalMarfud>) {
    let mut kharijiya = Vec::new();
    let mut marfuda = Vec::new();
    for madkhal in sijill.kharijiya() {
        let id = madkhal.id;
        match madkhal_khariji(madkhal.clone()) {
            Ok(qaid) => kharijiya.push(qaid),
            Err(sabab) => marfuda.push(MadkhalMarfud { ruqaa: id, sabab }),
        }
    }
    (kharijiya, marfuda)
}

/// What the commit says it did, so the registry's history reads without this
/// application open.
///
/// The two kinds are written under separate headings rather than in one list,
/// because a reader of this history is being told what the registry now serves
/// and the two answers are different: one is a package Taarib compiled, sealed
/// and certified as holding no byte of the game; the other is an address and a
/// digest for an archive its author built out of the game's own containers.
fn risalat_iltizam(tasalsul: u64, natija: &Mustawda, mulghayat: usize) -> String {
    use std::fmt::Write as _;
    let mut matn = format!("Cast the catalogue at sequence {tasalsul}\n\n");
    for madkhal in &natija.madakhil {
        let _ = writeln!(
            matn,
            "{} r{} — {}",
            madkhal.mulakhkhas.id,
            madkhal.mulakhkhas.murajaa.qeema(),
            madkhal.mulakhkhas.unwan
        );
    }
    if !natija.kharijiya.is_empty() {
        matn.push_str(
            "\nThird-party entries — not built by Taarib, fetched from their authors \
                       and outside the asset gate's certificate:\n",
        );
        for madkhal in &natija.kharijiya {
            let khariji = madkhal.madkhal();
            let _ = writeln!(
                matn,
                "{} {} — {} by {}",
                khariji.id,
                khariji.isdar,
                khariji.unwan,
                khariji.nasab()
            );
        }
    }
    let _ = write!(
        matn,
        "\n{} listing(s), {} third-party entr(ies), {mulghayat} revocation(s).",
        natija.madakhil.len(),
        natija.kharijiya.len()
    );
    matn
}

/// Stages the whole tree and commits it onto the fetched branch.
fn iltazim(
    mustawda: &git2::Repository,
    asas: Option<git2::Oid>,
    talab: &TalabNashrMustawda<'_>,
    risala: &str,
) -> NatijatTaqdeem<String> {
    let marhala = MarhalatNashrMustawda::Iltizam;
    let mut fahras = mustawda
        .index()
        .map_err(|khata| khata_git(marhala, &khata))?;
    fahras
        .add_all(std::iter::once(&"*"), git2::IndexAddOption::DEFAULT, None)
        .map_err(|khata| khata_git(marhala, &khata))?;
    fahras.write().map_err(|khata| khata_git(marhala, &khata))?;
    let shajara_id = fahras
        .write_tree()
        .map_err(|khata| khata_git(marhala, &khata))?;
    let shajara = mustawda
        .find_tree(shajara_id)
        .map_err(|khata| khata_git(marhala, &khata))?;

    let tawqee = git2::Signature::now(talab.ism_musahim, talab.barid)
        .map_err(|khata| khata_git(marhala, &khata))?;
    let walid = asas
        .map(|id| mustawda.find_commit(id))
        .transpose()
        .map_err(|khata| khata_git(marhala, &khata))?;
    let asl: Vec<&git2::Commit<'_>> = walid.iter().collect();
    let iltizam = mustawda
        .commit(
            Some(&format!("refs/heads/{}", talab.far)),
            &tawqee,
            &tawqee,
            risala,
            &shajara,
            &asl,
        )
        .map_err(|khata| khata_git(marhala, &khata))?;
    Ok(iltizam.to_string())
}

/// Pushes the branch, without force.
///
/// `git2` reports a server-side rejection through the update callback rather
/// than through the return value, so a push that was refused for being behind
/// looks like a success to anyone who only checks the `Result`. The callback is
/// the whole reason this is not three lines: a submission marked published over
/// a rejected push is the defect this module exists to close, in reverse.
fn idfa(mustawda: &git2::Repository, talab: &TalabNashrMustawda<'_>) -> NatijatTaqdeem<()> {
    let marhala = MarhalatNashrMustawda::Dafa;
    let rafd: RefCell<Option<String>> = RefCell::new(None);
    let mut nida_dafa = nida(talab.login, talab.sirr);
    let _ = nida_dafa.push_update_reference(|marji, hala| {
        if let Some(sabab) = hala {
            *rafd.borrow_mut() = Some(format!("{marji}: {sabab}"));
        }
        Ok(())
    });
    let mut dafa = git2::PushOptions::new();
    let _ = dafa.remote_callbacks(nida_dafa);

    let khariita = format!("refs/heads/{far}:refs/heads/{far}", far = talab.far);
    mustawda
        .remote_anonymous(talab.rabt_git)
        .map_err(|khata| khata_git(marhala, &khata))?
        .push(&[khariita.as_str()], Some(&mut dafa))
        .map_err(|khata| khata_git(marhala, &khata))?;

    let marfud = rafd.borrow().clone();
    match marfud {
        Some(sabab) => Err(fashil(
            marhala,
            format!(
                "{sabab}. The cast is in {}; nothing was marked published.",
                talab.jidhr.display()
            ),
        )),
        None => Ok(()),
    }
}

/// The credential callback every fetch and push uses.
fn nida(login: &str, sirr: &str) -> git2::RemoteCallbacks<'static> {
    let login = login.to_owned();
    let sirr = sirr.to_owned();
    let mut nida = git2::RemoteCallbacks::new();
    let _ = nida.credentials(move |_rabt, _mustakhdim, _masmuh| {
        git2::Cred::userpass_plaintext(&login, &sirr)
    });
    nida
}

#[cfg(test)]
mod fahs {
    use std::error::Error;

    use taarib_mustalahat::luba::LubaId;
    use taarib_mustalahat::musahim::MusahimId;
    use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};

    use super::{MadkhalNashr, SijillNashr};

    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// The same game for every row: the shard a listing lands in is not what
    /// these tests are about.
    const fn luba() -> LubaId {
        LubaId::min_uuid(uuid::Uuid::from_u128(
            0x0192_6d42_dc5d_74b2_b27d_5c44_41a0_3a3f,
        ))
    }

    /// One ledger row. Fallible only because a contributor identity is a key
    /// fingerprint and refuses anything else.
    fn madkhal(ruqaa: RuqaaId) -> Result<MadkhalNashr, Box<dyn Error>> {
        Ok(MadkhalNashr {
            ruqaa,
            murajaa: RuqaaRevision::jadeeda(1),
            luba: luba(),
            ism_luba: "R.E.P.O.".to_owned(),
            musahim: MusahimId::jadeed("22".repeat(32))?,
            ism_malaf: format!("{ruqaa}-r1-aaaaaaaa.ruqaa"),
            waqt_iaatimad: "2026-09-20T15:51:00Z".to_owned(),
            tasalsul: None,
            tajawuz: None,
        })
    }

    /// Approval records an entry the registry does not have yet, and it says
    /// so: `muaallaqa` is what the console counts to decide whether there is
    /// anything to publish.
    #[test]
    fn madkhal_jadeed_yabqa_muaallaqan() -> NatijatIkhtibar {
        let mut sijill = SijillNashr::default();
        let ruqaa = RuqaaId::jadeeda();
        sijill.sajjil(madkhal(ruqaa)?);
        assert_eq!(sijill.muaallaqa().len(), 1);
        sijill.nushirat(ruqaa, 3);
        assert!(sijill.muaallaqa().is_empty());
        assert_eq!(
            sijill.madkhal(ruqaa).and_then(|qaid| qaid.tasalsul),
            Some(3)
        );
        Ok(())
    }

    /// A revoked lineage leaves the catalogue and joins the list, so the next
    /// cast neither publishes it nor un-revokes it.
    #[test]
    fn al_ilgha_yanqul_min_alfahras_ila_alqaima() -> NatijatIkhtibar {
        let mut sijill = SijillNashr::default();
        let ruqaa = RuqaaId::jadeeda();
        sijill.sajjil(madkhal(ruqaa)?);
        sijill.nushirat(ruqaa, 3);
        sijill.ilghi(ruqaa, "published in error", "2026-09-21T00:00:00Z");
        assert!(sijill.madkhal(ruqaa).is_none());
        assert_eq!(sijill.mulghayat_lil_sabk().ruqa.len(), 1);
        Ok(())
    }

    /// Revoking the same lineage twice keeps one entry with the later reason,
    /// so the list never grows a duplicate that would make the count guard
    /// meaningless.
    #[test]
    fn ilgha_mukarrar_la_yudaaif_alqaida() {
        let mut sijill = SijillNashr::default();
        let ruqaa = RuqaaId::jadeeda();
        sijill.ilghi(ruqaa, "first", "2026-09-21T00:00:00Z");
        sijill.ilghi(ruqaa, "second", "2026-09-22T00:00:00Z");
        let mulghayat = sijill.mulghayat_lil_sabk();
        assert_eq!(mulghayat.ruqa.len(), 1);
        assert_eq!(
            mulghayat.ruqa.first().map(|zawj| zawj.1.as_str()),
            Some("second")
        );
    }

    /// A ledger round-trips, because it is the only record of what the
    /// catalogue contains: a field lost in serialisation is a listing lost from
    /// the next cast.
    #[test]
    fn alsijill_yaud_kama_kutib() -> NatijatIkhtibar {
        let mut sijill = SijillNashr::default();
        let ruqaa = RuqaaId::jadeeda();
        sijill.sajjil(madkhal(ruqaa)?);
        sijill.nushirat(ruqaa, 7);
        sijill.ilghi(RuqaaId::jadeeda(), "withdrawn", "2026-09-21T00:00:00Z");
        let bayt = serde_json::to_vec(&sijill)?;
        let baad: SijillNashr = serde_json::from_slice(&bayt)?;
        assert_eq!(baad, sijill);
        Ok(())
    }

    /// A new revision of a published lineage replaces the entry and starts
    /// unpublished: the registry carries the old revision until the next cast,
    /// and saying otherwise is the lie this whole module is about.
    #[test]
    fn murajaa_jadeeda_tubtil_alnashr_alsabiq() -> NatijatIkhtibar {
        let mut sijill = SijillNashr::default();
        let ruqaa = RuqaaId::jadeeda();
        sijill.sajjil(madkhal(ruqaa)?);
        sijill.nushirat(ruqaa, 3);
        let mut talia = madkhal(ruqaa)?;
        talia.murajaa = RuqaaRevision::jadeeda(2);
        sijill.sajjil(talia);
        assert_eq!(sijill.muaallaqa().len(), 1);
        Ok(())
    }
}
