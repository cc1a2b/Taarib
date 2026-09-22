//! تعريبات المجتمع — the Arabic other teams already made for a game, credited
//! and linked. Taarib hosts none of it and installs none of it.
//!
//! Everything here reads `taarib_mustawda::mujtama`, the registry's index of
//! community translations, and projects it for two screens: a count on every
//! library card, and the credited list on the game screen with one control per
//! entry that opens the makers' own page in the browser. The count is read from
//! the cache and never from the network, because the library scan runs on a
//! thread that must not wait for one; the list is fetched on demand and the
//! same refresh keeps the cache warm for the next scan.
//!
//! Every one of those paths reads the index through the owner's key. The index
//! is what tells [`iftah_rabt`] which hosts it may hand to a browser, and a
//! verified index is the only kind this module can hold: `FahrasMujtama` has no
//! constructor but the verifier's, so there is nowhere in here for an unsigned
//! entry to be credited, counted or opened.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use taarib_khatm::MiftahAam;
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::luba::Luba;
use taarib_mustalahat::wahhid_ism;
use taarib_mustawda::masadir::SilsilatMasadir;
use taarib_mustawda::mujtama::{
    AslFahrasMujtama, FahrasMujtama, HalatTarjamaMujtama, MudifTarjama, NAFIDHAT_FAHRAS_MUJTAMA,
    NawRukhsa, NawTanzeelat, TaghtiyaMujtama, TareeqaMujtama, Tarjama, TawzeeTarjama, iqra_makhbaa,
    jalb_fahras_mujtama, tarjamat_li_luba,
};
use taarib_usus::idadat::MakhzanIdadat;
use taarib_usus::khata::{Khata, Khutura, Khutwa, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;
use taarib_usus::masarat::Masarat;

use crate::luba_awamir::{appid_steam, huwiya, ijlib_luba};
use crate::tathbeet_awamir::{bil_hajb, silsilat_masadir};

/// Hosts a page may live on whether or not the cached index links to them:
/// the mod platforms the index draws from, and the forge the registry lives
/// on. A host is admitted itself or as any subdomain of itself, because
/// `castle-of-fear.itch.io` and `itch.io` are the same platform.
const MUDIFUN_MARUFA: [&str; 6] = [
    "github.com",
    "nexusmods.com",
    "thunderstore.io",
    "gamebanana.com",
    "itch.io",
    "steamcommunity.com",
];

/// Longest address the open command accepts, the same bound the index holds
/// its own addresses to.
const HADD_TUL_RABT: usize = 2048;

/// The anchor every community index is verified under.
///
/// The same key every installed patch and every revocation list is verified
/// under, compiled into this build — `taarib_khatm::MIRSAT_MALIK`. There is no
/// second anchor for the community index and no way to run without one: an
/// anchor this build cannot read is a build that is inconsistent with itself,
/// not a user who did anything.
///
/// # Errors
///
/// [`taarib_khatm::KhataKhatm::MiftahTalif`] when the compiled anchor is not a
/// canonical Ed25519 public key.
fn miftah_malik() -> Result<MiftahAam, taarib_khatm::KhataKhatm> {
    MiftahAam::min_bayt(&taarib_khatm::MIRSAT_MALIK.miftah)
}

/// How long the background refresh waits after a fetch that failed or that
/// had to serve the stale cache: long enough not to hammer a source that is
/// down, short enough that a machine coming back online sees the index the
/// same hour.
const MUHLAT_IADA: Duration = Duration::from_hours(1);

/// One community translation, as the game screen draws it.
///
/// Every field is what the translation's own page stated on the day
/// [`Self::waqt_tahaqquq`] names, carried through the index verbatim. The
/// team's two names arrive resolved rather than as the index's reference,
/// because the screen credits the makers first and a reference is not a name.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TarjamaMujtamaHie {
    /// The entry's identifier in the index, for the row's key.
    pub muarrif: String,
    /// The team's name in Latin script, when the entry belongs to a listed team.
    pub fariq: Option<String>,
    /// The team's Arabic name, when it has one.
    pub fariq_arabi: Option<String>,
    /// The author or team as the page names them.
    pub muallif: String,
    /// Where the page lives, as a kind.
    pub mudif: MudifTarjama,
    /// The page's host, for the kinds that are somebody's own site.
    pub mudif_ism: String,
    /// The page itself, `https` only.
    pub rabt: String,
    /// Coverage, as a kind.
    pub taghtiya: TaghtiyaMujtama,
    /// Coverage, in the page's own words.
    pub taghtiya_nass: Option<String>,
    /// How it was translated.
    pub tareeqa: TareeqaMujtama,
    /// How its terms are stated.
    pub rukhsa: NawRukhsa,
    /// The SPDX identifier, when the terms are a recognised licence.
    pub rukhsa_muarrif: Option<String>,
    /// The terms as written.
    pub rukhsa_nass: Option<String>,
    /// Free, sold, or subscription-only.
    pub tawzee: TawzeeTarjama,
    /// Where it stands.
    pub hala: HalatTarjamaMujtama,
    /// The version the page states.
    pub isdar: Option<String>,
    /// When it was published, `YYYY-MM-DD`.
    pub waqt_alnashr: Option<String>,
    /// When it was last updated, `YYYY-MM-DD`.
    pub akhir_tahdith: Option<String>,
    /// The download or subscriber count the page shows.
    pub tanzeelat: Option<u32>,
    /// What that count counts.
    pub tanzeelat_naw: Option<NawTanzeelat>,
    /// The day the page was read, `YYYY-MM-DD`.
    pub waqt_tahaqquq: String,
}

/// Every community translation the index lists for one game, credited.
///
/// The index is answered from the cache while it is current, fetched through
/// the configured sources when it is not, and served stale when every source
/// refuses — and verified against the owner's key in every one of those cases
/// before a maker is credited or an address is offered. An empty answer
/// therefore means the index was read and lists nothing for this game; a
/// machine with no index at all, or one holding an index nobody signed, is
/// refused by name instead, so the screen never says "nothing is known" when
/// nobody could look and never shows a panel that is blank for no stated
/// reason.
///
/// # Errors
///
/// [`taarib_mustawda::KhataMustawda::FahrasMujtamaGhayrMutah`] when no source
/// answered and nothing is cached,
/// [`taarib_mustawda::KhataMustawda::FahrasMujtamaGhayrMuwaqqa`] when the copy
/// this machine holds carries no owner signature — the state a machine
/// upgrading from a build that did not sign the index starts in —
/// [`taarib_mustawda::KhataMustawda::FahrasMujtamaTawqeeBatil`] when a
/// signature was offered and refused, whatever the index parser refuses when
/// the one answer that came was unreadable, and whatever the game lookup
/// raises.
#[tauri::command]
#[specta::specta]
pub async fn tarjamat_mujtama(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<Vec<TarjamaMujtamaHie>, Khata> {
    let id = huwiya(muarrif)?;
    let hali = idadat.hali();
    let miftah = miftah_malik()?;
    let luba = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || ijlib_luba(&makhzan, id)).await?
    };

    // Offline mode with no local copy is a chain with nothing in it, not a
    // refusal: the fetch then answers from the cache or says there is none.
    let silsila = silsilat_masadir(&hali).unwrap_or_else(|_| SilsilatMasadir::jadida(Vec::new()));
    let majlub = jalb_fahras_mujtama(&silsila, &masarat, &miftah, Timestamp::now())
        .await
        .map_err(|khata| Khata::min_tafsir(&khata))?;
    sajjil_asl(&majlub.asl);
    dhamin_mujaddid_mujtama(&masarat, &idadat);

    Ok(li_luba(&majlub.fahras, &luba)
        .into_iter()
        .map(|tarjama| hie(&majlub.fahras, tarjama))
        .collect())
}

/// Opens a community translation's page in the default browser.
///
/// The address is checked before anything is launched: `https` only, within
/// the length the index holds its own addresses to, no whitespace or control
/// characters, no credentials, and a host the cached index links to or one of
/// the platforms in [`MUDIFUN_MARUFA`]. The interface only ever sends addresses
/// it received from [`tarjamat_mujtama`], so a refusal here is a defect and not
/// a user error, but a command that opens whatever it is handed is a command
/// that opens whatever a compromised webview hands it.
///
/// The hosts the index contributes come out of a *verified* cache and nowhere
/// else. That is the whole reason the index is signed: an allow-list read from
/// a document anybody could serve would let a compromised source name any host
/// it liked and have this command open it. A cache that does not verify
/// contributes nothing, which leaves the compiled-in platforms and refuses the
/// rest — the panel that fetched the index is where the refusal is named to the
/// user, and this command is not the place to raise it a second time.
///
/// ## The third-party panels
///
/// They call this command too, with an author's own page, and that address does
/// not come from the community index — so it used to be refused wherever the
/// author does not happen to live on a platform this file names. The allowance
/// added for them is deliberately not "these panels may open anything": it is
/// [`crate::khariji_awamir::mudifu_muallifin`], the **exact hosts** of the
/// author addresses that third-party catalogue entries name, drawn from the
/// entries compiled into this binary and from those a verified registry shard or
/// the owner's own ledger produced this session. They join `mudifun`, which is
/// matched by membership and never by suffix, so nothing here widens to a
/// subdomain and no address reaches the browser because a panel asked nicely.
///
/// # Errors
///
/// [`KhataMujtamaAmr::RabtMarfud`] when the address fails any check above,
/// [`KhataMujtamaAmr::FathRabtFashil`] when the platform would not open it, and
/// [`taarib_khatm::KhataKhatm::MiftahTalif`] when this build's own trust anchor
/// is unreadable.
#[tauri::command]
#[specta::specta]
pub async fn iftah_rabt(rabt: String, masarat: tauri::State<'_, Masarat>) -> Result<bool, Khata> {
    let masarat = Masarat::clone(&masarat);
    bil_hajb(move || {
        let miftah = miftah_malik()?;
        let mut mudifun = match iqra_makhbaa(&masarat, &miftah) {
            Ok(mukhazzan) => mukhazzan
                .map(|mukhazzan| mukhazzan.fahras.mudifun())
                .unwrap_or_default(),
            Err(khata) => {
                tracing::warn!(
                    khata = %khata,
                    "the cached community index was refused; only the known platforms may be opened"
                );
                BTreeSet::new()
            },
        };
        mudifun.extend(crate::khariji_awamir::mudifu_muallifin());
        let salim = rabt_salim(&rabt, &mudifun)?;
        tauri_plugin_opener::open_url(salim.as_str(), None::<&str>).map_err(|sabab| {
            Khata::from(KhataMujtamaAmr::FathRabtFashil {
                rabt: salim.to_string(),
                tafsil: sabab.to_string(),
            })
        })?;
        tracing::info!(rabt = %salim, "a community translation's page was opened");
        Ok(true)
    })
    .await
}

/// The cached index, for a caller on a thread that must not wait for a network.
///
/// `None` when nothing is cached, when the cache does not read, or when it does
/// not verify — the library scan draws all three as no community marks, because
/// a card cannot raise an error and a mark drawn from an unverified index is a
/// mark this product did not earn. The refresh this module runs in the
/// background is what fills it for the next scan, and the game screen is where
/// a refused index is named to the user.
pub(crate) fn fahras_mukhazzan(masarat: &Masarat) -> Option<FahrasMujtama> {
    let miftah = miftah_malik()
        .inspect_err(|khata| {
            tracing::error!(khata = %khata, "this build's trust anchor is unreadable; no community index can be verified");
        })
        .ok()?;
    match iqra_makhbaa(masarat, &miftah) {
        Ok(mukhazzan) => mukhazzan.map(|mukhazzan| mukhazzan.fahras),
        Err(khata) => {
            tracing::warn!(
                khata = %khata,
                "the cached community index was refused; the library shows no community marks until a signed one is fetched"
            );
            None
        },
    }
}

/// How many community translations the index lists for one game.
///
/// The same match the game screen's list is built from, so a card that says
/// "community Arabic" and a screen that lists none cannot both be drawn.
pub(crate) fn adad_li_luba(fahras: &FahrasMujtama, luba: &Luba) -> u32 {
    u32::try_from(li_luba(fahras, luba).len()).unwrap_or(u32::MAX)
}

/// The background refresh, started once per process by the first library scan
/// or game screen that touches the index.
///
/// Sleeps out whatever is left of the freshness window after each answer,
/// re-reading the settings on every pass so a source added or offline mode
/// switched on takes effect at the next one, and comes back sooner after an
/// answer that had to be served stale or could not be served at all.
pub(crate) fn dhamin_mujaddid_mujtama(masarat: &Masarat, idadat: &Arc<MakhzanIdadat>) {
    static MUJADDID: OnceLock<()> = OnceLock::new();
    // Read once, outside the loop: the anchor is compiled in and cannot change
    // between passes, and a build whose own anchor will not read has no refresh
    // to run rather than one that fails every hour.
    let Ok(miftah) = miftah_malik().inspect_err(|khata| {
        tracing::error!(khata = %khata, "this build's trust anchor is unreadable; the community index refresh will not run");
    }) else {
        return;
    };
    let masarat = masarat.clone();
    let idadat = Arc::clone(idadat);
    MUJADDID.get_or_init(|| {
        drop(tauri::async_runtime::spawn(async move {
            loop {
                let hali = idadat.hali();
                let silsila = silsilat_masadir(&hali)
                    .unwrap_or_else(|_| SilsilatMasadir::jadida(Vec::new()));
                let raqda =
                    match jalb_fahras_mujtama(&silsila, &masarat, &miftah, Timestamp::now()).await {
                        Ok(majlub) => {
                            sajjil_asl(&majlub.asl);
                            raqdat_asl(&majlub.asl)
                        },
                        Err(khata) => {
                            tracing::warn!(khata = %khata, "the community index refresh found nothing to serve");
                            MUHLAT_IADA
                        },
                    };
                tokio::time::sleep(raqda).await;
            }
        }));
        // The cell holds nothing; reaching it at all is the whole record.
    });
}

/// How long the background refresh sleeps after an answer.
fn raqdat_asl(asl: &AslFahrasMujtama) -> Duration {
    match asl {
        // What is left of the window, never less than a minute so a record
        // stamped at the window's edge does not turn the loop into a busy one.
        AslFahrasMujtama::Makhbaa { umr } => {
            let baqi = NAFIDHAT_FAHRAS_MUJTAMA
                .checked_sub(*umr)
                .unwrap_or(SignedDuration::ZERO);
            Duration::try_from(baqi)
                .unwrap_or(Duration::ZERO)
                .max(Duration::from_mins(1))
        },
        AslFahrasMujtama::Masdar { .. } => {
            Duration::try_from(NAFIDHAT_FAHRAS_MUJTAMA).unwrap_or(MUHLAT_IADA)
        },
        AslFahrasMujtama::MakhbaaQadeem { .. } => MUHLAT_IADA,
    }
}

/// The log line every index answer ends on, at the level its provenance deserves.
fn sajjil_asl(asl: &AslFahrasMujtama) {
    if asl.hadith() {
        tracing::info!("{}", asl.wasf());
    } else {
        tracing::warn!("{}", asl.wasf());
    }
}

/// The entries for one game: by its Steam id, and by the same normalised name
/// the store keys `ism_muwahhad` on.
fn li_luba<'a>(fahras: &'a FahrasMujtama, luba: &Luba) -> Vec<&'a Tarjama> {
    tarjamat_li_luba(fahras, appid_steam(luba), &wahhid_ism(&luba.ism))
}

/// One entry, projected for the screen.
fn hie(fahras: &FahrasMujtama, tarjama: &Tarjama) -> TarjamaMujtamaHie {
    let fariq = tarjama
        .fariq
        .as_deref()
        .and_then(|muarrif| fahras.fariq(muarrif));
    TarjamaMujtamaHie {
        muarrif: tarjama.muarrif.clone(),
        fariq: fariq.map(|fariq| fariq.ism.clone()),
        fariq_arabi: fariq.and_then(|fariq| fariq.ism_arabi.clone()),
        muallif: tarjama.muallif.clone(),
        mudif: tarjama.mudif,
        mudif_ism: mudif_ism(&tarjama.rabt),
        rabt: tarjama.rabt.clone(),
        taghtiya: tarjama.taghtiya,
        taghtiya_nass: tarjama.taghtiya_nass.clone(),
        tareeqa: tarjama.tareeqa,
        rukhsa: tarjama.rukhsa.naw,
        rukhsa_muarrif: tarjama.rukhsa.muarrif.clone(),
        rukhsa_nass: tarjama.rukhsa.nass.clone(),
        tawzee: tarjama.tawzee,
        hala: tarjama.hala,
        isdar: tarjama.isdar.clone(),
        waqt_alnashr: tarjama.waqt_alnashr.clone(),
        akhir_tahdith: tarjama.akhir_tahdith.clone(),
        // The page's own figure, held to the width the interface counts in. A
        // count past four billion is a page nobody has, and would cross as
        // that ceiling rather than as a lie in the other direction.
        tanzeelat: tarjama
            .tanzeelat
            .map(|adad| u32::try_from(adad).unwrap_or(u32::MAX)),
        tanzeelat_naw: tarjama.tanzeelat_naw,
        waqt_tahaqquq: tarjama.tahaqquq.waqt.clone(),
    }
}

/// The host a page lives on, as the screen names a site that is nobody's
/// platform: `etrdream.com` rather than `https://etrdream.com/game/…`.
fn mudif_ism(rabt: &str) -> String {
    reqwest::Url::parse(rabt)
        .ok()
        .and_then(|rabt| rabt.host_str().map(str::to_ascii_lowercase))
        .map_or_else(
            || rabt.to_owned(),
            |mudif| {
                mudif
                    .strip_prefix("www.")
                    .map_or_else(|| mudif.clone(), str::to_owned)
            },
        )
}

/// The address, when it is one this command will open.
fn rabt_salim(rabt: &str, mudifun: &BTreeSet<String>) -> Result<reqwest::Url, Khata> {
    let marfud = |sabab: &str| {
        Khata::from(KhataMujtamaAmr::RabtMarfud {
            rabt: rabt.to_owned(),
            sabab: sabab.to_owned(),
        })
    };
    if rabt.len() > HADD_TUL_RABT {
        return Err(marfud("longer than 2048 characters"));
    }
    if rabt
        .chars()
        .any(|harf| harf.is_whitespace() || harf.is_control() || harf_khafi(harf))
    {
        return Err(marfud(
            "carries whitespace, a control character or an invisible one",
        ));
    }
    if !rabt
        .get(.."https://".len())
        .is_some_and(|badiya| badiya.eq_ignore_ascii_case("https://"))
    {
        return Err(marfud("only https addresses are opened"));
    }
    let salim = reqwest::Url::parse(rabt).map_err(|khata| marfud(&khata.to_string()))?;
    if salim.scheme() != "https" {
        return Err(marfud("only https addresses are opened"));
    }
    if !salim.username().is_empty() || salim.password().is_some() {
        return Err(marfud("carries credentials"));
    }
    let mudif = salim
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| marfud("names no host"))?;
    if !mudif_masmuh(&mudif, mudifun) {
        return Err(marfud(
            "its host is not one the community index links to, nor a known mod platform",
        ));
    }
    Ok(salim)
}

/// Whether a character is invisible where an address is read.
///
/// `char::is_control` does not cover these: they are format characters, not
/// controls, so every one of them passes that check and draws as nothing at
/// all. In an address that matters twice over. The screen prints the address
/// beside the button that opens it, and a right-to-left override inside it makes
/// the printed host read as one thing while the browser goes to another — the
/// spoof this product is least entitled to pass on, because the whole panel
/// exists to send a reader to somebody else's page.
///
/// The set is the bidi controls, the zero-width characters and the byte-order
/// mark. A legitimate address needs none of them: a host with Arabic letters in
/// it reaches this function already punycoded, and a path that genuinely carries
/// one carries it percent-encoded.
const fn harf_khafi(harf: char) -> bool {
    matches!(
        harf,
        '\u{00ad}'            // soft hyphen
        | '\u{061c}'          // Arabic letter mark
        | '\u{200b}'..='\u{200f}' // zero-width space … right-to-left mark
        | '\u{202a}'..='\u{202e}' // the embedding and override controls
        | '\u{2060}'..='\u{2064}' // word joiner and the invisible operators
        | '\u{2066}'..='\u{2069}' // the isolate controls
        | '\u{feff}' // byte-order mark
    )
}

/// Whether a host is one the cached index links to, or a known platform or a
/// subdomain of one.
fn mudif_masmuh(mudif: &str, mudifun: &BTreeSet<String>) -> bool {
    mudifun.contains(mudif)
        || MUDIFUN_MARUFA.iter().any(|maruf| {
            mudif == *maruf
                || mudif
                    .strip_suffix(maruf)
                    .is_some_and(|badiya| badiya.ends_with('.'))
        })
}

/// Failures of the community translation commands.
#[derive(Debug, thiserror::Error)]
pub enum KhataMujtamaAmr {
    /// An address the open command refused before launching anything.
    #[error("{rabt} was not opened: {sabab}")]
    RabtMarfud {
        /// The address as it arrived.
        rabt: String,
        /// Which check refused it.
        sabab: String,
    },

    /// The platform would not open the browser.
    #[error("the browser would not open {rabt}: {tafsil}")]
    FathRabtFashil {
        /// The address.
        rabt: String,
        /// What the operating system said.
        tafsil: String,
    },
}

impl Tafsir for KhataMujtamaAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::RabtMarfud { .. } => 150,
                    Self::FathRabtFashil { .. } => 151,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        // Nothing was written and nothing was opened; the page is one copy
        // and paste away either way.
        Khutura::Tanbeeh
    }

    fn arabi(&self) -> String {
        match self {
            Self::RabtMarfud { .. } => {
                "رفض تعريب فتح هذا الرابط لأنه ليس عنوانًا آمنًا من صفحة يعرفها فهرس المجتمع. \
                 يمكنك نسخه وفتحه في متصفحك بنفسك."
                    .to_owned()
            },
            Self::FathRabtFashil { .. } => {
                "تعذّر فتح المتصفح لعرض هذه الصفحة. انسخ الرابط وافتحه في متصفحك بنفسك.".to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::RabtMarfud { .. } => {
                "Taarib refused to open this link: it is not a safe address on a page the \
                 community index knows. Copy it and open it in your browser yourself."
                    .to_owned()
            },
            Self::FathRabtFashil { .. } => {
                "The browser could not be opened for this page. Copy the link and open it in \
                 your browser yourself."
                    .to_owned()
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::RabtMarfud { .. } => Khutwa::LaShay,
            Self::FathRabtFashil { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::RabtMarfud { rabt, sabab } => {
                let _ = siyaq.insert("rabt".to_owned(), QeemaSiyaq::Nass(rabt.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::FathRabtFashil { rabt, tafsil } => {
                let _ = siyaq.insert("rabt".to_owned(), QeemaSiyaq::Nass(rabt.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataMujtamaAmr);

#[cfg(test)]
mod ikhtibarat {
    use super::{BTreeSet, mudif_ism, mudif_masmuh, rabt_salim};

    fn mudifun() -> BTreeSet<String> {
        ["etrdream.com", "www.nexusmods.com"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// An index host exactly, a known platform or any subdomain of it, and
    /// nothing that merely ends in a known platform's name.
    #[test]
    fn al_mudif_al_masmuh() {
        let mudifun = mudifun();
        assert!(mudif_masmuh("etrdream.com", &mudifun));
        assert!(mudif_masmuh("www.nexusmods.com", &mudifun));
        assert!(mudif_masmuh("github.com", &mudifun));
        assert!(mudif_masmuh("castle-of-fear.itch.io", &mudifun));
        assert!(!mudif_masmuh("notgithub.com", &mudifun));
        assert!(!mudif_masmuh("github.com.evil.example", &mudifun));
        assert!(!mudif_masmuh("okshopsa.net", &mudifun));
    }

    /// Everything the open command refuses, and the one shape it opens.
    #[test]
    fn al_rabt_al_salim() {
        let mudifun = mudifun();
        assert!(rabt_salim("https://etrdream.com/game/afterimage/", &mudifun).is_ok());
        assert!(rabt_salim("HTTPS://GitHub.com/cc1a2b/taarib-registry", &mudifun).is_ok());
        assert!(rabt_salim("http://etrdream.com/", &mudifun).is_err());
        assert!(rabt_salim("https://etrdream.com/a b", &mudifun).is_err());
        assert!(rabt_salim("https://etrdream.com/\u{200f}", &mudifun).is_err());
        assert!(rabt_salim("https://user:pw@etrdream.com/", &mudifun).is_err());
        assert!(rabt_salim("https://okshopsa.net/games", &mudifun).is_err());
        assert!(rabt_salim("javascript:alert(1)", &mudifun).is_err());
        assert!(rabt_salim("https://", &mudifun).is_err());
        let taweel = format!("https://etrdream.com/{}", "a".repeat(2048));
        assert!(rabt_salim(&taweel, &mudifun).is_err());
    }

    /// The host the screen names, without the `www.` nobody says aloud.
    #[test]
    fn ism_al_mudif() {
        assert_eq!(
            mudif_ism("https://www.nexusmods.com/sekiro/mods/431"),
            "nexusmods.com"
        );
        assert_eq!(mudif_ism("https://etrdream.com/game/x/"), "etrdream.com");
        assert_eq!(mudif_ism("not a url"), "not a url");
    }
}
