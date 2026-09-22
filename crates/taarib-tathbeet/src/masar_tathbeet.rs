//! The ordered install pipeline: authorise, verify, match, back up, deploy, set, place, confirm.

use std::path::{Path, PathBuf};

use taarib_aman::IdhnTathbeet;
use taarib_muhawwil_nusus::tarkeeb::TaqreerTarkeeb;
use taarib_mustalahat::bina::{Basma, BinaId};
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_ruqaa::tawqee::MudaqqiqTawqee;
use taarib_tarqee::irtibat::{BasmatKhatt, IrtibatBina, SababMutabaqa};
use taarib_usus::manassa::{self, HalatTashghil};
use walkdir::WalkDir;

use crate::bayan::{Muthabbit, NawTathbeet, TarifLuba, Tathbeet};
use crate::itlaq;
use crate::khata::{KhataTathbeet, MasdarKhatt, NatijatTathbeet};
use crate::mawdi::WajhatLuba;
use crate::nusus::Nashir;
use crate::tahaqquq::{NatijatTahaqquq, tahaqquq_kamil};
use crate::tarkib::QararTabaqa;
use crate::tasadum::la_yatasadam_maa_khariji;

/// One patch-content placement: a validated in-game destination and its bytes.
#[derive(Debug)]
pub struct WadaMuhtawa {
    /// Where it goes, validated against the game root.
    pub wajha: WajhatLuba,
    /// What is written there.
    pub bayt: Vec<u8>,
}

/// The compatibility decision reached before any write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QararTawafuq {
    /// Exact build match.
    Tamma,
    /// Fingerprint match after the build id moved.
    BiBasma,
    /// Approximate, proceeding only on the user's acknowledgement.
    BiIqrar,
}

/// What the caller supplies to run one installation.
pub struct TalabTathbeet<'a> {
    /// The game, its launcher and its root.
    pub luba: &'a TarifLuba,
    /// The build currently installed.
    pub bina: &'a BinaId,
    /// The patch's binding, from the package.
    pub irtibat: &'a IrtibatBina,
    /// The patch content to place.
    pub muhtawa: Vec<WadaMuhtawa>,
    /// Whether the user acknowledged an approximate-match install.
    pub iqrar_taqribi: bool,
    /// The game executable's name, to refuse installing while it runs.
    pub tanfidhi: &'a str,
    /// Steam's install root, when the caller resolved one.
    ///
    /// The launch-option step needs it and nothing else here does. It is passed
    /// in rather than discovered because the caller has already resolved it —
    /// against the user's own override, which a second discovery here would not
    /// see, and a machine with Steam in two places would then be patched in one
    /// of them and told about the other.
    ///
    /// [`None`] is only correct for an install whose plan asks for no
    /// launch-time change; when one is asked for and this is [`None`],
    /// [`thabbit`] refuses rather than deploy a framework nothing will load.
    pub jidhr_steam: Option<&'a Path>,
}

impl std::fmt::Debug for TalabTathbeet<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabTathbeet")
            .field("luba", &self.luba.ism)
            .field("bina", &self.bina)
            .field("adad_muhtawa", &self.muhtawa.len())
            .field("iqrar_taqribi", &self.iqrar_taqribi)
            .finish()
    }
}

/// What one installation did.
#[derive(Debug)]
pub struct NatijatTathbeetKamil {
    /// The compatibility decision acted on.
    pub tawafuq: QararTawafuq,
    /// How many content placements were written.
    pub adad_muhtawa: usize,
    /// The post-write verification verdict.
    pub tahaqquq: NatijatTahaqquq,
    /// How many launcher account files had their launch options changed.
    ///
    /// Zero for the ordinary install, which asks for no launch-time change at
    /// all, and zero again when every account already carried the assignment —
    /// which is what a second install over the first produces. It is a count of
    /// files and not of games: one requirement applied to two signed-in Steam
    /// accounts is two.
    pub adad_idadat: usize,
    /// What the script-engine write did, when the game is on one of the four
    /// engines Taarib patches as data.
    ///
    /// [`None`] for every other game, which is most of them: a Unity install
    /// reads its translations out of the placed package at run time, and an
    /// Unreal install's are compiled into the additive container by the
    /// deployment step rather than written into the game's own files. Also
    /// [`None`] at tier 3, where the game's own text is never replaced at all.
    pub nusus: Option<TaqreerTarkeeb>,
    /// Whether the patch content was deliberately **not** placed inside the
    /// game.
    ///
    /// True at tier 3, whose product surface says the game is not modified at
    /// all — a `taarib/` directory and a package inside it is a modification of
    /// the game's directory, and nothing at that tier reads the package from
    /// there: the payload that does is the one tier 3 never deploys. Also true
    /// when the deployment step declared no tier decision, because an install
    /// that never established what it is allowed to do to a game has no warrant
    /// to put anything in it.
    pub muhtawa_matruk: bool,
}

/// Runs the full install pipeline for one game and one package.
///
/// `nashr` is the deployment step. It is handed a [`Nashir`] — the recording
/// guard every write already went through, plus the package — and it owns
/// **every** write into the game's own files: the script-engine write through
/// [`Nashir::raqqi`], and the framework and adapter through the recorder.
/// Passing it as a step keeps the per-engine table in `tarkib` and the write
/// ordering here.
///
/// That the script-engine write belongs to the deployment step is the
/// correction this signature carries. It used to run here instead —
/// unconditionally, before the step, re-deriving the engine from the game's own
/// directory — with no plan to consult and therefore no tier and no safety
/// refusal to honour. A tier-3 game, whose report had just told the player the
/// game would not be modified at all, had its `data/*.json`, its
/// `game/tl/arabic/*.rpy`, its `data.win` or its `app.asar` rewritten. The step
/// that holds the plan is the only place that write can correctly happen.
///
/// The manifest is durable before the deployment step or any placement runs,
/// because [`Tathbeet::ibda`] flushes it and [`Tathbeet`] is the only writer
/// either step is given. A failure after the first write leaves that manifest on
/// disk; the caller rolls back rather than leaving a partial install.
///
/// # Errors
///
/// [`KhataTathbeet::IdhnGhayrMutabiq`] when the safety proof covers a different
/// game or package; [`KhataTathbeet::LubaTashtaghil`] when the game is running
/// and [`KhataTathbeet::HalatLubaMajhula`] when a sandbox makes that
/// unanswerable; [`KhataTathbeet::TasadumRuqaa`] when a translation somebody
/// else made is already in the game; [`KhataTathbeet::RuqaaMarfuda`] when the
/// package fails verification;
/// [`KhataTathbeet::TawafuqMarfud`] when the build does not match and no
/// acknowledgement was given; [`KhataTathbeet::MunassaTaamal`] and
/// [`KhataTathbeet::HalatManassaMajhula`] when the deployment needs a
/// launch-option change and the launcher that rewrites that file on exit is up
/// or cannot be seen; and whatever `nashr`, the manifest or the guard raise.
pub fn thabbit<F>(
    talab: &TalabTathbeet<'_>,
    idhn: &IdhnTathbeet,
    ruqaa: &MalafRuqaa,
    mudaqqiq: &dyn MudaqqiqTawqee,
    jidhr_nusakh: &Path,
    huwiya: impl Into<String>,
    nashr: F,
) -> NatijatTathbeet<NatijatTathbeetKamil>
where
    F: FnOnce(&mut Nashir<'_>) -> NatijatTathbeet<()>,
{
    let basmat = basmat_ruqaa(&talab.luba.jidhr, ruqaa)?;
    if !idhn.yushmal(talab.luba.luba, basmat) {
        return Err(KhataTathbeet::IdhnGhayrMutabiq);
    }

    la_tashtaghil(talab.tanfidhi)?;
    // Before the package is opened, because the answer has nothing to do with
    // the package: two translations of one game overwrite each other's files,
    // and the third-party ones tell their users to delete the file Taarib's own
    // loader is published as.
    la_yatasadam_maa_khariji(&talab.luba.jidhr, jidhr_nusakh)?;
    tahaqquq_ruqaa(&talab.luba.jidhr, ruqaa, mudaqqiq)?;
    let tawafuq = qarrir_tawafuq(talab)?;

    let mut tathbeet = Tathbeet::ibda(jidhr_nusakh, NawTathbeet::Nass, talab.luba, huwiya)?;
    let (qarar, nusus) = {
        let mut nashir = Nashir::jadeed(&mut tathbeet, ruqaa, &talab.luba.jidhr);
        nashr(&mut nashir)?;
        (nashir.qarar(), nashir.nusus())
    };

    // Immediately after the deployment and before anything else, because this is
    // the step that decides whether what was just deployed ever loads, and
    // because the launcher guard inside it is the one refusal that has to happen
    // while there is least in the game to take back out again.
    let adad_idadat = naffidh_idadat(&mut tathbeet, talab.jidhr_steam)?;

    // The tier decides this too. Package content is Taarib's own file in a
    // directory Taarib creates, and at tier 3 there is no payload in the game to
    // read it — the tier deploys none — so placing it would be a directory added
    // to a game the product promised not to modify, for nothing.
    let yuktab = qarar.is_some_and(QararTabaqa::tughayyar_al_luba);
    let adad_muhtawa = if yuktab {
        ida_muhtawa(&mut tathbeet, &talab.muhtawa)?
    } else {
        0
    };

    let taqreer = tahaqquq_kamil(&talab.luba.jidhr, jidhr_nusakh, NawTathbeet::Nass)?;
    Ok(NatijatTathbeetKamil {
        tawafuq,
        adad_muhtawa,
        adad_idadat,
        tahaqquq: taqreer.natija(),
        nusus,
        muhtawa_matruk: !yuktab && !talab.muhtawa.is_empty(),
    })
}

/// Carries out the launch-time changes the deployment recorded.
///
/// The deployment states the requirement and performs none of it — see
/// [`crate::tarkib::KhuttatTarkib::talabat`] — so the manifest it just flushed
/// is where the requirement is read from. Going through the record rather than
/// through a second copy of the plan is what makes it impossible for the value
/// written into a launcher to disagree with the value an uninstall will look
/// for: they are the same string, read once.
///
/// The requirement is read out of the manifest and cloned before the recorder is
/// borrowed to write into it, which is also why the list is materialised rather
/// than iterated in place.
fn naffidh_idadat(tathbeet: &mut Tathbeet, jidhr_steam: Option<&Path>) -> NatijatTathbeet<usize> {
    let talabat = itlaq::talabat_steam(tathbeet.bayan());
    if talabat.is_empty() {
        return Ok(0);
    }
    itlaq::naffidh_talabat_steam(tathbeet, &talabat, jidhr_steam)
}

fn basmat_ruqaa(jidhr: &Path, ruqaa: &MalafRuqaa) -> NatijatTathbeet<Basma> {
    let mafateeh = ruqaa.ruqaa().map_err(|khata| khata_ruqaa(jidhr, &khata))?;
    Ok(Basma::min_bayt(mafateeh.tarwisa().basma))
}

/// Refuses when the game's own executable is running — or when that cannot be
/// established.
///
/// Both install and uninstall call this first, so a running game is named
/// before anything is touched rather than surfaced as a file lock partway
/// through. The restore path keeps its own reactive lock guard: a restore must
/// not depend on a process enumeration succeeding.
///
/// The three-state [`manassa::halat_tashghil`] is what is consulted, never the
/// raw process list. Inside a sandbox the list is the sandbox's own and comes
/// back empty for every process on the machine, so reading "no match" as "not
/// running" would silently disarm this guard exactly where it matters most —
/// a Steam Deck, where Steam and a game are the normal state of the machine.
///
/// # Errors
///
/// [`KhataTathbeet::LubaTashtaghil`] naming the process and its executable when
/// one was seen; [`KhataTathbeet::HalatLubaMajhula`] naming the sandbox when the
/// question could not be answered at all.
pub fn la_tashtaghil(tanfidhi: &str) -> NatijatTathbeet<()> {
    let ism = Path::new(tanfidhi)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(tanfidhi);
    hukm_tashghil(manassa::halat_tashghil(ism), ism, tanfidhi)
}

/// Turns one running-state verdict into the refusal it warrants.
///
/// Split from [`la_tashtaghil`] so all three verdicts can be proved without a
/// process table to arrange — the one state that matters most is the one a test
/// machine cannot produce on demand.
fn hukm_tashghil(hala: HalatTashghil, ism: &str, tanfidhi: &str) -> NatijatTathbeet<()> {
    match hala {
        HalatTashghil::LaTashtaghil => Ok(()),
        // The enumeration is repeated only here, on the path that is already
        // aborting, because the verdict alone does not carry which process
        // matched and the message is worth a second pass. A process that exited
        // in between leaves the executable Taarib was asked about, which is
        // still true and still names the right thing to close.
        HalatTashghil::Tashtaghil => Err(match manassa::amaliyat_bism(ism).into_iter().next() {
            Some(amaliya) => KhataTathbeet::LubaTashtaghil {
                amaliya: amaliya.ism,
                tanfidhi: amaliya.masar.unwrap_or_else(|| PathBuf::from(tanfidhi)),
            },
            None => KhataTathbeet::LubaTashtaghil {
                amaliya: ism.to_owned(),
                tanfidhi: PathBuf::from(tanfidhi),
            },
        }),
        HalatTashghil::GhayrMaaruf { sunduq } => Err(KhataTathbeet::HalatLubaMajhula {
            sunduq,
            tanfidhi: PathBuf::from(tanfidhi),
        }),
    }
}

fn tahaqquq_ruqaa(
    jidhr: &Path,
    ruqaa: &MalafRuqaa,
    mudaqqiq: &dyn MudaqqiqTawqee,
) -> NatijatTathbeet<()> {
    let mafateeh = ruqaa.ruqaa().map_err(|khata| khata_ruqaa(jidhr, &khata))?;
    let basma = mafateeh.tarwisa().basma;
    mafateeh
        .tawqee()
        .tahaqquq(&basma, mudaqqiq)
        .map_err(|khata| KhataTathbeet::RuqaaMarfuda {
            masar: jidhr.to_path_buf(),
            sabab: khata.to_string(),
        })
}

fn qarrir_tawafuq(talab: &TalabTathbeet<'_>) -> NatijatTathbeet<QararTawafuq> {
    let hukm = talab.irtibat.ihkum(talab.bina);
    let marfud = |yumkin_bi_iqrar: bool| KhataTathbeet::TawafuqMarfud {
        hukm: hukm.sabab.wasf_injilizi().to_owned(),
        yumkin_bi_iqrar,
    };
    match hukm.sabab {
        SababMutabaqa::MuarrifWaBasma => Ok(QararTawafuq::Tamma),
        SababMutabaqa::BasmaFaqat => Ok(QararTawafuq::BiBasma),
        SababMutabaqa::MuarrifBilaBasma | SababMutabaqa::DakhilNitaq => {
            if talab.iqrar_taqribi {
                Ok(QararTawafuq::BiIqrar)
            } else {
                Err(marfud(true))
            }
        },
        SababMutabaqa::BilaTatabuq => Err(marfud(false)),
    }
}

fn ida_muhtawa(tathbeet: &mut Tathbeet, muhtawa: &[WadaMuhtawa]) -> NatijatTathbeet<usize> {
    let mut adad = 0_usize;
    for wada in muhtawa {
        let mutlaq = tathbeet.jidhr_luba().join(wada.wajha.nisbi());
        // Package content is an addition, never a replacement: the game shipped
        // none of it. So it goes through the path that records the directories
        // it creates along with the file. The replace path also marks an absent
        // file as added, which is why this looked right, but it records no
        // directory — and a restore that deletes `taarib/nusus.ruqaa` and
        // leaves `taarib/` standing has not given the game back as shipped.
        // The add path's refusal to write over something already there is the
        // right guard here too: a file at this path that no manifest knows is
        // not Taarib's to delete on uninstall, so it is not Taarib's to replace.
        tathbeet.ansha(&mutlaq, &wada.bayt)?;
        adad = adad.saturating_add(1);
    }
    Ok(adad)
}

/// The bytes of the package as one engine's adapter can read them.
///
/// For every engine but Unity this is the sealed container itself, byte for
/// byte. For Unity it is [`taarib_ruqaa::katib::nuskha_muarra`]'s working copy:
/// the takeover assembly reads tables by casting them out of a memory map and
/// carries no decompressor, so it refuses a compressed section by name and
/// reads only what the installer placed uncompressed. The sealed container was
/// verified — hash and signature — before this is ever called, which is what
/// makes a derived copy safe to place.
///
/// The gap this closes was the last of four between the installer and the
/// Unity plugin: the plugin loaded, its native library loaded, it found the
/// package and the face, and refused at `TAARIB-E-6007` because the string
/// table was zstd.
///
/// # Errors
///
/// [`KhataTathbeet::RuqaaMarfuda`] carrying the format crate's own refusal when
/// the sealed container cannot be re-emitted.
pub fn muhtawa_ruqaa(aila: AilatMuharrik, bayt: Vec<u8>, masar: &Path) -> NatijatTathbeet<Vec<u8>> {
    if aila != AilatMuharrik::Unity {
        return Ok(bayt);
    }
    taarib_ruqaa::katib::nuskha_muarra(&bayt)
        .map(|muarra| muarra.ila_shuaa())
        .map_err(|khata| khata_ruqaa(masar, &khata))
}

/// Which font set a root holds, and therefore who is able to put a file in it.
///
/// Carried rather than inferred from position. The installer has to be able to
/// say whose problem an unresolvable face is, and "the first root is the user's"
/// is precisely the kind of agreement between two call sites that was already
/// wrong once here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NawJidhrKhutut {
    /// The user's own font directory — the only root anybody can add a face to.
    Mustakhdim,
    /// A set the build ships. Read-only, the same in every install of the same
    /// version, and deliberately never mirrored into the user's directory.
    Bina,
}

/// One font root an install resolves the faces a package names against.
///
/// The list of these is assembled by whoever owns the build's layout — the
/// studio, or an automatic run's own staging directory — because this crate
/// knows nothing about where a bundle is unpacked. What it must be given is
/// *every* root the compiler was allowed to bundle a face from: a face resolved
/// against fewer roots than it was chosen from is a package that can be built
/// and never installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JidhrKhutut {
    /// The directory searched.
    pub masar: PathBuf,
    /// Whose set it holds.
    pub naw: NawJidhrKhutut,
}

impl JidhrKhutut {
    /// A root holding the user's own imported faces.
    #[must_use]
    pub fn mustakhdim(masar: impl Into<PathBuf>) -> Self {
        Self {
            masar: masar.into(),
            naw: NawJidhrKhutut::Mustakhdim,
        }
    }

    /// A root holding a set the build ships.
    #[must_use]
    pub fn bina(masar: impl Into<PathBuf>) -> Self {
        Self {
            masar: masar.into(),
            naw: NawJidhrKhutut::Bina,
        }
    }
}

/// The font files a package names, read out of the font stores so they can be
/// placed beside the package.
///
/// Only an engine whose adapter draws text itself takes fonts this way. The
/// Unity takeover rasterises through `taarib_jisr` with the faces the patch was
/// shaped against, and reads them from `taarib/khutut/<name>` under the game
/// root — the directory the package itself lands in, so that one manifest
/// records both and one restore removes both. Every other engine either ships
/// its own text stack or is handed a face by its own deployment step (Ren'Py's
/// arrives inside its component), and for those this returns nothing.
///
/// A face is found by its exact file name anywhere under a root and accepted
/// only when its BLAKE3 equals the fingerprint the package recorded. The
/// fingerprint is not a formality: a face replaced under the same name shapes
/// differently from every layout the package precomputed, and the adapter would
/// draw with metrics that belong to a different file. So every candidate is
/// hashed and the first that matches wins, rather than the first that is named
/// right.
///
/// **Every** root is searched, which is the correction this signature carries.
/// It used to take one directory while every compile path chose faces from two
/// — the user's own directory and the read-only set the build ships — so a
/// patch built with a bundled face named a file the installer then refused to
/// find, and told the user to update Taarib, which re-shipped the same set to
/// the same place the installer was not looking. The only escape was to find the
/// `.ttf` inside the application's own installation directory and import it as
/// though it were a font of the user's own.
///
/// # Errors
///
/// [`KhataTathbeet::KhattMafqud`] naming the face when no root holds a file of
/// that name carrying that fingerprint — and saying which set the face belongs
/// to, because "update Taarib" and "add the file under Settings → Fonts" are
/// different instructions and only one of them is ever right. And
/// [`KhataTathbeet::MasarKharij`] when a recorded name is not a bare file name.
pub fn muhtawa_khutut(
    aila: AilatMuharrik,
    khutut: &[BasmatKhatt],
    judhur: &[JidhrKhutut],
) -> NatijatTathbeet<Vec<WadaMuhtawa>> {
    if aila != AilatMuharrik::Unity {
        return Ok(Vec::new());
    }
    let mut muhtawa = Vec::with_capacity(khutut.len());
    for khatt in khutut {
        let wajha = WajhatLuba::dakhil_taarib(&format!("{MUJALLAD_KHUTUT}/{}", khatt.ism))?;
        // A validated destination whose file name is not the recorded name is
        // a recorded name that carried a directory in it, which no store entry
        // has; refusing it here keeps the search below to bare names.
        if wajha.ism() != khatt.ism {
            return Err(KhataTathbeet::MasarKharij {
                masar: PathBuf::from(&khatt.ism),
                jidhr: judhur
                    .first()
                    .map(|jidhr| jidhr.masar.clone())
                    .unwrap_or_default(),
                sabab: "a font name in the package is not a bare file name".to_owned(),
            });
        }
        muhtawa.push(WadaMuhtawa {
            wajha,
            bayt: iqra_khatt(khatt, judhur)?,
        });
    }
    Ok(muhtawa)
}

/// Reads one face out of the first root that holds it under the recorded
/// fingerprint.
///
/// A candidate that hashes to something else is passed over rather than
/// accepted or refused outright: the user's directory can hold a different
/// `Amiri.ttf` from the build's, and finding the wrong one first is not a
/// reason to stop looking for the right one. What is never passed over is the
/// hash — a face resolved by name alone is a different font.
fn iqra_khatt(khatt: &BasmatKhatt, judhur: &[JidhrKhutut]) -> NatijatTathbeet<Vec<u8>> {
    let mut asbab: Vec<String> = Vec::new();
    // Which set the face belongs to, decided by which root carries the name at
    // all — the one signal on this machine that survives the file being wrong
    // or unreadable. The package itself does not record it: every build of one
    // version ships the same set, so a name a build root carries is a face
    // Taarib owes the user, and a name no build root carries is one only the
    // user can supply.
    let mut masdar = MasdarKhatt::Mustakhdim;
    for jidhr in judhur {
        for masar in jid_khutut(&jidhr.masar, &khatt.ism) {
            if jidhr.naw == NawJidhrKhutut::Bina {
                masdar = MasdarKhatt::Bina;
            }
            match std::fs::read(&masar) {
                Ok(bayt) if blake3::hash(&bayt).as_bytes() == khatt.basma.bayt() => {
                    return Ok(bayt);
                },
                Ok(_) => asbab.push(format!(
                    "{}: the file there is not the one the package was shaped against",
                    masar.display()
                )),
                Err(sabab) => {
                    asbab.push(format!("{} could not be read: {sabab}", masar.display()));
                },
            }
        }
    }
    Err(KhataTathbeet::KhattMafqud {
        ism: khatt.ism.clone(),
        masdar,
        judhur: judhur.iter().map(|jidhr| jidhr.masar.clone()).collect(),
        sabab: if asbab.is_empty() {
            format!(
                "searched {}",
                judhur
                    .iter()
                    .map(|jidhr| jidhr.masar.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            asbab.join("; ")
        },
    })
}

/// The directory under `taarib/` a Unity takeover reads its faces from.
///
/// `Taarib.Unity.Mono`'s `DalilKhutut`, spelled once on this side.
const MUJALLAD_KHUTUT: &str = "khutut";

/// How deep the font store is searched. The store is `<family>/<file>`; the
/// bound exists so a store somebody has put something else into is a miss
/// rather than a walk.
const UMQ_KHUTUT: usize = 3;

/// Every file of one exact name under a font root, in a stable order.
///
/// All of them rather than the first: a store is `<family>/<file>`, and the
/// same file name can sit under two families with only one of them carrying the
/// fingerprint the package recorded. Sorted so that two machines holding the
/// same store report the same candidate in the same refusal.
fn jid_khutut(jidhr: &Path, ism: &str) -> Vec<PathBuf> {
    WalkDir::new(jidhr)
        .max_depth(UMQ_KHUTUT)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|madkhal| madkhal.file_type().is_file() && madkhal.file_name() == ism)
        .map(walkdir::DirEntry::into_path)
        .collect()
}

fn khata_ruqaa(jidhr: &Path, khata: &taarib_ruqaa::khata::KhataRuqaa) -> KhataTathbeet {
    KhataTathbeet::RuqaaMarfuda {
        masar: jidhr.to_path_buf(),
        sabab: khata.to_string(),
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use std::path::{Path, PathBuf};

    use taarib_usus::khata::{Khutwa, Tafsir as _};
    use taarib_usus::manassa::{RuyatAmaliyat, Sunduq, ruyat_amaliyat};

    use super::{HalatTashghil, KhataTathbeet, la_tashtaghil};
    use crate::masar_tathbeet::hukm_tashghil;

    /// A path whose file name no process on any machine carries.
    const TANFIDHI_MUSTAHIL: &str = "/taarib/la-yujad-hadha-al-tanfidhi-abadan";

    /// The file name inside [`TANFIDHI_MUSTAHIL`], which is what is looked up.
    const ISM_MUSTAHIL: &str = "la-yujad-hadha-al-tanfidhi-abadan";

    /// The child test the sandbox proof re-runs this binary to reach.
    #[cfg(target_os = "linux")]
    const ISM_IBN: &str = "masar_tathbeet::ikhtibarat::ibn_al_sunduq";

    #[test]
    fn la_tashtaghil_tuqbal_ghayr_al_mawjud_faqat() {
        let maftuh = hukm_tashghil(HalatTashghil::LaTashtaghil, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL);
        assert!(
            maftuh.is_ok(),
            "a process seen absent is the one state that clears the guard"
        );
        for sunduq in [Sunduq::Flatpak, Sunduq::Snap, Sunduq::Hawiya] {
            let hala = HalatTashghil::GhayrMaaruf { sunduq };
            assert!(
                hukm_tashghil(hala, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL).is_err(),
                "a question that could not be answered is not a no"
            );
        }
        assert!(hukm_tashghil(HalatTashghil::Tashtaghil, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL).is_err());
    }

    #[test]
    fn al_rafd_yusammi_al_sunduq_wala_yaddai_al_tashghil() {
        let khata = hukm_tashghil(
            HalatTashghil::GhayrMaaruf {
                sunduq: Sunduq::Flatpak,
            },
            ISM_MUSTAHIL,
            TANFIDHI_MUSTAHIL,
        )
        .expect_err("a blind guard refuses");
        match &khata {
            KhataTathbeet::HalatLubaMajhula { sunduq, tanfidhi } => {
                assert_eq!(*sunduq, Sunduq::Flatpak);
                assert_eq!(tanfidhi, Path::new(TANFIDHI_MUSTAHIL));
            },
            akhar => panic!("expected HalatLubaMajhula, got {akhar:?}"),
        }
        // The whole point of the variant: it names where it is, and it does not
        // assert the thing it could not observe.
        assert!(khata.injilizi().contains("Flatpak"));
        assert!(khata.injilizi().contains("cannot tell whether"));
        assert!(khata.arabi().contains("فلاتباك"));
        assert!(khata.arabi().contains("تعذّر عليه معرفة"));
        assert!(!khata.injilizi().contains("is running ("));
        // Retrying inside the same sandbox produces the same refusal forever,
        // so no retry is offered and no action is invented.
        assert!(!khata.qabil_lil_iada());
        assert_eq!(khata.khutwa(), Khutwa::LaShay);
        assert_eq!(khata.masar(), Some(Path::new(TANFIDHI_MUSTAHIL)));
    }

    #[test]
    fn al_tashghil_yasqut_ila_ma_sammahu_al_muttasil() {
        // No process carries this name, so the second enumeration finds nothing
        // and the refusal names the executable the caller asked about rather
        // than inventing a process that was never seen.
        let khata = hukm_tashghil(HalatTashghil::Tashtaghil, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL)
            .expect_err("a running game refuses");
        match khata {
            KhataTathbeet::LubaTashtaghil { amaliya, tanfidhi } => {
                assert_eq!(amaliya, ISM_MUSTAHIL);
                assert_eq!(tanfidhi, PathBuf::from(TANFIDHI_MUSTAHIL));
            },
            akhar => panic!("expected LubaTashtaghil, got {akhar:?}"),
        }
    }

    #[test]
    fn al_hirasa_tatbaa_ruyat_hadhihi_al_ala() {
        // On an ordinary machine an absent process clears the guard, which is
        // the behaviour nothing here may change. On a machine that is itself
        // sandboxed the same call must refuse — that is not a reason to skip
        // the test, it is the other half of the proof.
        match ruyat_amaliyat() {
            RuyatAmaliyat::Kamila => {
                assert!(la_tashtaghil(TANFIDHI_MUSTAHIL).is_ok());
            },
            RuyatAmaliyat::Maazula { sunduq } => {
                let khata = la_tashtaghil(TANFIDHI_MUSTAHIL).expect_err("a blind guard refuses");
                assert!(matches!(
                    khata,
                    KhataTathbeet::HalatLubaMajhula { sunduq: mawjud, .. } if mawjud == sunduq
                ));
            },
        }
    }

    /// Drives the sandboxed branch for real, rather than by constructing the
    /// state by hand: a child of this same test binary carrying Flatpak's own
    /// marker, which is what `fi_sunduq` reads.
    #[cfg(target_os = "linux")]
    #[test]
    fn fi_sunduq_haqiqi_yarfud_al_tathbeet() {
        let exe = std::env::current_exe().expect("the test binary's own path");
        let natija = std::process::Command::new(exe)
            .args([
                ISM_IBN,
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("FLATPAK_ID", "org.taarib.Studio")
            .output()
            .expect("re-running this test binary");
        assert!(
            natija.status.success(),
            "the sandboxed child failed:\n{}\n{}",
            String::from_utf8_lossy(&natija.stdout),
            String::from_utf8_lossy(&natija.stderr)
        );
    }

    /// The child of [`fi_sunduq_haqiqi_yarfud_al_tathbeet`]. Ignored because it
    /// proves nothing without the marker its parent sets.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "needs the sandbox marker its parent process sets; run by that parent"]
    fn ibn_al_sunduq() {
        assert_eq!(
            taarib_usus::manassa::fi_sunduq(),
            Some(Sunduq::Flatpak),
            "the marker did not reach the child, so nothing below proves anything"
        );

        // The install guard. Before this change the same call returned Ok(()).
        let khata = la_tashtaghil(TANFIDHI_MUSTAHIL)
            .expect_err("a guard that cannot see the process table must refuse");
        match khata {
            KhataTathbeet::HalatLubaMajhula { sunduq, tanfidhi } => {
                assert_eq!(sunduq, Sunduq::Flatpak);
                assert_eq!(tanfidhi, PathBuf::from(TANFIDHI_MUSTAHIL));
            },
            akhar => panic!("expected HalatLubaMajhula, got {akhar:?}"),
        }

        // The launcher guard, through the same marker. The executable name is
        // one nothing can be running, so the only way to reach a refusal is the
        // blindness itself rather than a process that happened to be up.
        let malaf = Path::new("/taarib/localconfig.vdf");
        let khata = crate::itlaq::manassa_mughlaqa(&[ISM_MUSTAHIL], "Steam", malaf)
            .expect_err("a launcher guard that cannot see must refuse too");
        match khata {
            KhataTathbeet::HalatManassaMajhula {
                sunduq,
                manassa,
                malaf: mawdi,
            } => {
                assert_eq!(sunduq, Sunduq::Flatpak);
                assert_eq!(manassa, "Steam");
                assert_eq!(mawdi, malaf);
            },
            akhar => panic!("expected HalatManassaMajhula, got {akhar:?}"),
        }
    }
}
