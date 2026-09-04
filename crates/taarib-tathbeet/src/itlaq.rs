//! Launch integration: launch options, wrapper launchers, and the preload environment.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use taarib_kashf::matajir::vdf;
use taarib_usus::khata::Tafsir as _;
use taarib_usus::manassa::{self, HalatTashghil};
use taarib_usus::masarat;

use crate::bayan::{MahallIdad, Muthabbit, SijillIdad};
use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io};
use crate::taraju::RadIdad;

/// The launcher identifier recorded for a Steam launch-options change.
pub const MANASSA_STEAM: &str = "steam";

/// The launcher identifier recorded for a Heroic launcher-arguments change.
pub const MANASSA_HEROIC: &str = "heroic";

/// The launcher identifier recorded for a Lutris environment change.
pub const MANASSA_LUTRIS: &str = "lutris";

/// The environment variable Wine and Proton read DLL overrides from.
pub const MUTAGHAYYIR_TAJAWUZ: &str = "WINEDLLOVERRIDES";

/// The one override entry the framework's Windows loader needs: native first,
/// builtin second.
pub const TAJAWUZ_TAARIB: &str = "winhttp=n,b";

/// Steam's placeholder for the game's own command line.
pub const RAMZ_AMR: &str = "%command%";

/// The key holding a game's launch options in `localconfig.vdf`.
pub const MIFTAH_KHIYARAT: &str = "LaunchOptions";

/// The file name of the account configuration Steam keeps launch options in.
pub const ISM_MALAF_STEAM: &str = "localconfig.vdf";

/// The directory Heroic keeps one JSON file per game in.
pub const MUJALLAD_HEROIC: &str = "GamesConfig";

/// The key holding a game's command prefix in its Lutris configuration.
pub const MIFTAH_LUTRIS: &str = "prefix_command";

/// The block that holds [`MIFTAH_LUTRIS`] in a Lutris game configuration.
pub const MIFTAH_LUTRIS_NIZAM: &str = "system";

/// The program a command prefix runs the environment assignment through.
pub const AMR_BEEA: &str = "env";

/// The keys leading to the per-app map inside `localconfig.vdf`.
const MASAR_ALAAB: [&str; 5] = ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];

/// Executable names the Steam client runs under, across the three systems.
///
/// Shared with `tarkib`, which guards the same file through the same check:
/// two copies of this list are two places for the running check to be wrong in.
pub(crate) const ASMAA_STEAM: [&str; 3] = ["steam.exe", "steam", "steam_osx"];

/// Executable names the Heroic client runs under.
const ASMAA_HEROIC: [&str; 2] = ["heroic.exe", "heroic"];

/// Executable names Lutris runs under.
const ASMAA_LUTRIS: [&str; 2] = ["lutris.exe", "lutris"];

/// The largest launcher configuration file this module will rewrite, in bytes.
pub const AQSA_HAJM_IDAD: u64 = 33_554_432;

/// The name shown for Steam in a refusal a person reads.
pub(crate) const ISM_STEAM: &str = "Steam";

/// The name shown for Heroic in a refusal a person reads.
const ISM_HEROIC: &str = "Heroic";

/// The name shown for Lutris in a refusal a person reads.
const ISM_LUTRIS: &str = "Lutris";

/// Merges Taarib's override entry into an existing `WINEDLLOVERRIDES` value.
#[must_use]
pub fn tajawuz_maa(hali: Option<&str>) -> String {
    dam_tajawuz(hali, TAJAWUZ_TAARIB)
}

/// Merges one override entry into an existing `WINEDLLOVERRIDES` value,
/// leaving the value alone when it already carries it.
#[must_use]
pub fn dam_tajawuz(hali: Option<&str>, jadeed: &str) -> String {
    let sabiq = hali.unwrap_or("").trim();
    let jadeed = jadeed.trim();
    if sabiq.is_empty() {
        return jadeed.to_owned();
    }
    if jadeed.is_empty() || madakhil_tajawuz(sabiq).any(|madkhal| madkhal == jadeed) {
        return sabiq.to_owned();
    }
    format!("{sabiq};{jadeed}")
}

/// Removes Taarib's override entry from a `WINEDLLOVERRIDES` value.
#[must_use]
pub fn bidun_tajawuz(hali: &str) -> Option<String> {
    let baqi: Vec<&str> =
        madakhil_tajawuz(hali).filter(|madkhal| *madkhal != TAJAWUZ_TAARIB).collect();
    if baqi.is_empty() { None } else { Some(baqi.join(";")) }
}

/// The entries of a `WINEDLLOVERRIDES` value, empty ones dropped.
fn madakhil_tajawuz(qeema: &str) -> impl Iterator<Item = &str> {
    qeema.split(';').map(str::trim).filter(|madkhal| !madkhal.is_empty())
}

/// Builds the launch-options value that adds Taarib's preload to what the user
/// already had, extending an existing assignment in place.
#[must_use]
pub fn khiyarat_maa_tahmeel(sabiqa: Option<&str>) -> String {
    let hali = sabiqa.unwrap_or("").trim();
    let mawjuda = mawqi_tajawuz(hali).map(|(_, _, qeema)| qeema);
    let dumija = dam_tajawuz(mawjuda.as_deref(), TAJAWUZ_TAARIB);
    khiyarat_maa_mutaghayyir(sabiqa, MUTAGHAYYIR_TAJAWUZ, &dumija)
}

/// Builds the launch-options value that sets one environment variable in front
/// of the game's own command line.
#[must_use]
pub fn khiyarat_maa_mutaghayyir(sabiqa: Option<&str>, ism: &str, qeema: &str) -> String {
    let hali = sabiqa.unwrap_or("").trim();
    let badiya = format!("{ism}=\"{qeema}\"");

    let mawdi = mawqi_mutaghayyir(hali, ism);
    if let Some((bidaya, nihaya, _)) = mawdi {
        let mut nateeja = String::with_capacity(hali.len().saturating_add(badiya.len()));
        nateeja.push_str(hali.get(..bidaya).unwrap_or_default());
        nateeja.push_str(&badiya);
        nateeja.push_str(hali.get(nihaya..).unwrap_or_default());
        return nateeja;
    }
    if hali.is_empty() {
        return format!("{badiya} {RAMZ_AMR}");
    }

    match mawqi_ramz_amr(hali) {
        Some(mawqi) => {
            let mut nateeja =
                String::with_capacity(hali.len().saturating_add(badiya.len()).saturating_add(1));
            nateeja.push_str(hali.get(..mawqi).unwrap_or_default());
            nateeja.push_str(&badiya);
            nateeja.push(' ');
            nateeja.push_str(hali.get(mawqi..).unwrap_or_default());
            nateeja
        }
        // Steam appends a placeholder-free value after the command line.
        None => format!("{badiya} {RAMZ_AMR} {hali}"),
    }
}

/// Builds the launch-options value that runs the game through a wrapper
/// command.
#[must_use]
pub fn khiyarat_maa_amr(sabiqa: Option<&str>, amr: &str) -> String {
    let hali = sabiqa.unwrap_or("").trim();
    if hali.is_empty() {
        return amr.to_owned();
    }
    match mawqi_ramz_amr(hali) {
        Some(mawqi) => {
            let baad = mawqi.saturating_add(RAMZ_AMR.len());
            let mut nateeja = String::with_capacity(hali.len().saturating_add(amr.len()));
            nateeja.push_str(hali.get(..mawqi).unwrap_or_default());
            nateeja.push_str(amr);
            nateeja.push_str(hali.get(baad..).unwrap_or_default());
            nateeja
        }
        None => format!("{amr} {hali}"),
    }
}

/// Removes Taarib's preload from a launch-options value, leaving everything
/// else exactly where it was.
#[must_use]
pub fn khiyarat_bidun_tahmeel(hali: &str, kan_fih_amr: bool) -> String {
    let mut nateeja = hali.to_owned();

    let mawdi_tajawuz = mawqi_tajawuz(&nateeja);
    if let Some((bidaya, nihaya, qeema)) = mawdi_tajawuz {
        let baqi = bidun_tajawuz(&qeema);
        let mut mabniya = String::with_capacity(nateeja.len());
        mabniya.push_str(nateeja.get(..bidaya).unwrap_or_default());
        if let Some(baqi) = baqi {
            let _ = write!(mabniya, "{MUTAGHAYYIR_TAJAWUZ}=\"{baqi}\"");
        }
        mabniya.push_str(nateeja.get(nihaya..).unwrap_or_default());
        nateeja = mabniya;
    }

    if !kan_fih_amr {
        let mawdi_amr = mawqi_ramz_amr(&nateeja);
        if let Some(mawqi) = mawdi_amr {
            let baad = mawqi.saturating_add(RAMZ_AMR.len());
            let mut mabniya = String::with_capacity(nateeja.len());
            mabniya.push_str(nateeja.get(..mawqi).unwrap_or_default());
            mabniya.push_str(nateeja.get(baad..).unwrap_or_default());
            nateeja = mabniya;
        }
    }

    nateeja.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Whether a launch-options value carries Steam's command placeholder.
#[must_use]
pub fn fihi_ramz_amr(qeema: &str) -> bool {
    mawqi_ramz_amr(qeema).is_some()
}

/// Builds the command prefix a wrapper launcher runs the game through, in the
/// form a "wrapper command" field takes rather than Steam's placeholder form.
#[must_use]
pub fn badiyat_amr_maa_tahmeel(sabiqa: Option<&str>) -> String {
    let hali = sabiqa.unwrap_or("").trim();
    let mawdi = mawqi_tajawuz(hali);
    if let Some((bidaya, nihaya, qeema)) = mawdi {
        let jadeeda = tajawuz_maa(Some(&qeema));
        let mut nateeja = String::with_capacity(hali.len().saturating_add(jadeeda.len()));
        nateeja.push_str(hali.get(..bidaya).unwrap_or_default());
        let _ = write!(nateeja, "{MUTAGHAYYIR_TAJAWUZ}=\"{jadeeda}\"");
        nateeja.push_str(hali.get(nihaya..).unwrap_or_default());
        return nateeja;
    }

    let badiya = format!("{AMR_BEEA} {MUTAGHAYYIR_TAJAWUZ}=\"{TAJAWUZ_TAARIB}\"");
    if hali.is_empty() { badiya } else { format!("{badiya} {hali}") }
}

/// Removes Taarib's command prefix, leaving everything else where it was.
#[must_use]
pub fn badiyat_amr_bidun_tahmeel(hali: &str, kan_fih_beea: bool) -> String {
    let mut nateeja = hali.to_owned();

    let mawdi = mawqi_tajawuz(&nateeja);
    let mut hudhifat = false;
    if let Some((bidaya, nihaya, qeema)) = mawdi {
        let baqi = bidun_tajawuz(&qeema);
        hudhifat = baqi.is_none();
        let mut mabniya = String::with_capacity(nateeja.len());
        mabniya.push_str(nateeja.get(..bidaya).unwrap_or_default());
        if let Some(baqi) = baqi {
            let _ = write!(mabniya, "{MUTAGHAYYIR_TAJAWUZ}=\"{baqi}\"");
        }
        mabniya.push_str(nateeja.get(nihaya..).unwrap_or_default());
        nateeja = mabniya;
    }

    let mut wahdat: Vec<&str> = nateeja.split_whitespace().collect();
    if hudhifat && !kan_fih_beea && wahdat.first() == Some(&AMR_BEEA) {
        let _ = wahdat.remove(0);
    }
    wahdat.join(" ")
}

/// Whether a command prefix already runs through [`AMR_BEEA`].
#[must_use]
pub fn yabda_bi_beea(qeema: &str) -> bool {
    qeema.split_whitespace().next() == Some(AMR_BEEA)
}

/// The byte offset of the leftmost `%command%`, matched without regard to case.
fn mawqi_ramz_amr(qeema: &str) -> Option<usize> {
    let saghir = qeema.to_ascii_lowercase();
    saghir.find(RAMZ_AMR)
}

/// Locates a `WINEDLLOVERRIDES=` assignment and reads its value.
fn mawqi_tajawuz(qeema: &str) -> Option<(usize, usize, String)> {
    mawqi_mutaghayyir(qeema, MUTAGHAYYIR_TAJAWUZ)
}

/// Locates one variable's assignment and reads its value.
fn mawqi_mutaghayyir(qeema: &str, ism: &str) -> Option<(usize, usize, String)> {
    let badiya = format!("{}=", ism.to_ascii_uppercase());
    let mawqi = qeema.to_ascii_uppercase().find(&badiya)?;
    if mawqi > 0 && !qeema.get(..mawqi)?.ends_with(char::is_whitespace) {
        return None;
    }
    let baad = mawqi.checked_add(badiya.len())?;
    let baqi = qeema.get(baad..)?;

    let mut ahruf = baqi.char_indices();
    let (_, awwal) = ahruf.next().unwrap_or((0, ' '));
    if awwal == '"' || awwal == '\'' {
        let mut nihaya = None;
        for (mawdi, harf) in ahruf {
            if harf == awwal {
                nihaya = Some(mawdi);
                break;
            }
        }
        let nihaya = nihaya?;
        let dakhil = baqi.get(awwal.len_utf8()..nihaya)?.to_owned();
        return Some((mawqi, baad.checked_add(nihaya.checked_add(awwal.len_utf8())?)?, dakhil));
    }

    let tul = baqi.find(char::is_whitespace).unwrap_or(baqi.len());
    Some((mawqi, baad.checked_add(tul)?, baqi.get(..tul)?.to_owned()))
}

/// The running state of a launcher that answers to more than one executable
/// name.
///
/// The precedence is deliberate. A match is trusted wherever it is found, so a
/// single name seen running settles it; only when nothing at all matched does
/// the visibility verdict decide between "not running" and "cannot be known",
/// and one blind name is enough to make the whole answer blind.
pub(crate) fn halat_manassa(asmaa: &[&str]) -> HalatTashghil {
    let mut mahjub = None;
    for ism in asmaa {
        match manassa::halat_tashghil(ism) {
            HalatTashghil::Tashtaghil => return HalatTashghil::Tashtaghil,
            HalatTashghil::GhayrMaaruf { sunduq } => mahjub = Some(sunduq),
            HalatTashghil::LaTashtaghil => {}
        }
    }
    mahjub.map_or(HalatTashghil::LaTashtaghil, |sunduq| HalatTashghil::GhayrMaaruf { sunduq })
}

/// Refuses while a launcher that rewrites its own configuration on exit is up —
/// and refuses just as firmly when this build cannot see whether it is.
pub(crate) fn manassa_mughlaqa(
    asmaa: &[&str],
    ism: &str,
    malaf: &Path,
) -> Result<(), KhataTathbeet> {
    hukm_manassa(halat_manassa(asmaa), ism, malaf)
}

/// Turns one launcher verdict into the refusal it warrants.
///
/// Split from [`manassa_mughlaqa`] so all three verdicts can be proved without
/// arranging a process table — the state that matters most is the one a test
/// machine cannot produce on demand.
fn hukm_manassa(hala: HalatTashghil, ism: &str, malaf: &Path) -> Result<(), KhataTathbeet> {
    match hala {
        HalatTashghil::LaTashtaghil => Ok(()),
        HalatTashghil::Tashtaghil => Err(KhataTathbeet::MunassaTaamal {
            manassa: ism.to_owned(),
            malaf: malaf.to_path_buf(),
        }),
        // Not `MunassaTaamal`: this build never saw the launcher, and telling
        // somebody to close a program that may not be open is telling them
        // something Taarib does not know.
        HalatTashghil::GhayrMaaruf { sunduq } => Err(KhataTathbeet::HalatManassaMajhula {
            sunduq,
            manassa: ism.to_owned(),
            malaf: malaf.to_path_buf(),
        }),
    }
}

/// Builds the residual file failure for something that is not an I/O error.
fn khata_shakl(masar: &Path, amal: &'static str, sabab: String) -> KhataTathbeet {
    KhataTathbeet::KhataMalaf {
        masar: masar.to_path_buf(),
        amal,
        sabab: std::io::Error::other(sabab),
    }
}

/// Builds the refusal a [`RadIdad`] raises when a setting cannot be put back.
fn khata_idad(sijill: &SijillIdad, sabab: String) -> KhataTathbeet {
    KhataTathbeet::IdadGhayrMustaad {
        muarrif: sijill.muarrif.clone(),
        mahall: sijill.mahall.wasf(),
        sabab,
    }
}

/// Reads a launcher configuration file as text, refusing what cannot be
/// rewritten without changing bytes nobody asked to change.
fn iqra_idad(masar: &Path, amal: &'static str) -> NatijatTathbeet<String> {
    let bayanat = fs::metadata(masar)
        .map_err(|sabab| min_khata_io(masar, "reading a launcher's configuration", sabab))?;
    if bayanat.len() > AQSA_HAJM_IDAD {
        return Err(KhataTathbeet::HajmMufrit {
            haql: "launcher configuration",
            qeema: bayanat.len(),
            saqf: AQSA_HAJM_IDAD,
        });
    }
    let bayt = fs::read(masar)
        .map_err(|sabab| min_khata_io(masar, "reading a launcher's configuration", sabab))?;
    String::from_utf8(bayt).map_err(|_| {
        khata_shakl(
            masar,
            amal,
            "the file is not valid UTF-8; rewriting it would replace every byte this build \
             cannot decode, so it was left alone"
                .to_owned(),
        )
    })
}

/// Writes a launcher configuration file atomically.
fn iktub_idad(masar: &Path, nass: &str) -> Result<(), KhataTathbeet> {
    masarat::kitaba_dharra(masar, nass.as_bytes()).map_err(|khata| KhataTathbeet::KhataMalaf {
        masar: masar.to_path_buf(),
        amal: "writing a launcher's configuration",
        sabab: std::io::Error::other(khata.injilizi),
    })
}

/// Quotes and escapes one `KeyValues` value.
///
/// # Errors
///
/// [`KhataTathbeet::KhataMalaf`] for a value holding a NUL byte, which the
/// format has no escape for and which would truncate the file at that point.
pub fn iqtibas_vdf(masar: &Path, qeema: &str) -> NatijatTathbeet<String> {
    if qeema.contains('\0') {
        return Err(khata_shakl(
            masar,
            "escaping a KeyValues value",
            "the value holds a NUL byte, which KeyValues cannot represent".to_owned(),
        ));
    }
    let mut kharij = String::with_capacity(qeema.len().saturating_add(2));
    kharij.push('"');
    for harf in qeema.chars() {
        match harf {
            '\\' => kharij.push_str("\\\\"),
            '"' => kharij.push_str("\\\""),
            '\n' => kharij.push_str("\\n"),
            '\r' => kharij.push_str("\\r"),
            '\t' => kharij.push_str("\\t"),
            _ => kharij.push(harf),
        }
    }
    kharij.push('"');
    Ok(kharij)
}

/// One token of a `KeyValues` document, with the span it occupies.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WahdaKhaam {
    bidaya: usize,
    nihaya: usize,
    naw: NawWahda,
}

/// What a `KeyValues` token is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NawWahda {
    Fath,
    Ighlaq,
    Kalima(String),
}

/// A cursor over a `KeyValues` document that keeps every byte offset, which the
/// tree the reader in `taarib-kashf` builds does not.
struct MasihVdf<'a> {
    nass: &'a str,
    bayt: &'a [u8],
    mawqi: usize,
}

impl<'a> MasihVdf<'a> {
    fn jadeed(nass: &'a str) -> Self {
        let bayt = nass.as_bytes();
        let mawqi = usize::from(bayt.get(..3) == Some(&[0xEF, 0xBB, 0xBF])) * 3;
        Self { nass, bayt, mawqi }
    }

    fn takhatti(&mut self) {
        loop {
            match self.bayt.get(self.mawqi) {
                Some(b) if b.is_ascii_whitespace() => {
                    self.mawqi = self.mawqi.saturating_add(1);
                }
                Some(b'/') if self.bayt.get(self.mawqi.saturating_add(1)) == Some(&b'/') => {
                    while let Some(b) = self.bayt.get(self.mawqi) {
                        if *b == b'\n' {
                            break;
                        }
                        self.mawqi = self.mawqi.saturating_add(1);
                    }
                }
                _ => return,
            }
        }
    }

    fn takhatti_shart(&mut self) {
        self.takhatti();
        if self.bayt.get(self.mawqi) != Some(&b'[') {
            return;
        }
        while let Some(b) = self.bayt.get(self.mawqi) {
            self.mawqi = self.mawqi.saturating_add(1);
            if *b == b']' {
                return;
            }
        }
    }

    fn wahda(&mut self, masar: &Path) -> NatijatTathbeet<Option<WahdaKhaam>> {
        self.takhatti();
        let bidaya = self.mawqi;
        let Some(&awwal) = self.bayt.get(bidaya) else { return Ok(None) };
        match awwal {
            b'{' => {
                self.mawqi = bidaya.saturating_add(1);
                Ok(Some(WahdaKhaam { bidaya, nihaya: self.mawqi, naw: NawWahda::Fath }))
            }
            b'}' => {
                self.mawqi = bidaya.saturating_add(1);
                Ok(Some(WahdaKhaam { bidaya, nihaya: self.mawqi, naw: NawWahda::Ighlaq }))
            }
            b'"' => {
                let kalima = self.muqtabas(masar)?;
                Ok(Some(WahdaKhaam {
                    bidaya,
                    nihaya: self.mawqi,
                    naw: NawWahda::Kalima(kalima),
                }))
            }
            _ => {
                let kalima = self.mujarrad(masar)?;
                Ok(Some(WahdaKhaam {
                    bidaya,
                    nihaya: self.mawqi,
                    naw: NawWahda::Kalima(kalima),
                }))
            }
        }
    }

    fn muqtabas(&mut self, masar: &Path) -> NatijatTathbeet<String> {
        let bidaya = self.mawqi;
        self.mawqi = self.mawqi.saturating_add(1);
        let mut kharij = String::new();
        loop {
            let Some(&b) = self.bayt.get(self.mawqi) else {
                return Err(khata_talif(
                    masar,
                    self.nass,
                    bidaya,
                    "a quoted string is never closed",
                ));
            };
            self.mawqi = self.mawqi.saturating_add(1);
            match b {
                b'"' => break,
                b'\\' => {
                    let Some(&tali) = self.bayt.get(self.mawqi) else {
                        return Err(khata_talif(
                            masar,
                            self.nass,
                            bidaya,
                            "an escape sequence runs off the end of the file",
                        ));
                    };
                    self.mawqi = self.mawqi.saturating_add(1);
                    match tali {
                        b'n' => kharij.push('\n'),
                        b't' => kharij.push('\t'),
                        b'r' => kharij.push('\r'),
                        b'\\' => kharij.push('\\'),
                        b'"' => kharij.push('"'),
                        // An unknown escape keeps both bytes, as Valve's reader does.
                        _ => {
                            kharij.push('\\');
                            kharij.push(char::from(tali));
                        }
                    }
                }
                _ => {
                    let harf = self
                        .nass
                        .get(self.mawqi.saturating_sub(1)..)
                        .and_then(|baqi| baqi.chars().next());
                    match harf {
                        Some(harf) => {
                            kharij.push(harf);
                            self.mawqi =
                                self.mawqi.saturating_sub(1).saturating_add(harf.len_utf8());
                        }
                        None => {
                            return Err(khata_talif(
                                masar,
                                self.nass,
                                bidaya,
                                "a quoted string ends inside a character",
                            ));
                        }
                    }
                }
            }
        }
        Ok(kharij)
    }

    fn mujarrad(&mut self, masar: &Path) -> NatijatTathbeet<String> {
        let bidaya = self.mawqi;
        while let Some(&b) = self.bayt.get(self.mawqi) {
            if b.is_ascii_whitespace() || matches!(b, b'{' | b'}' | b'"') {
                break;
            }
            if b == b'/' && self.bayt.get(self.mawqi.saturating_add(1)) == Some(&b'/') {
                break;
            }
            self.mawqi = self.mawqi.saturating_add(1);
        }
        self.nass
            .get(bidaya..self.mawqi)
            .map(str::to_owned)
            .ok_or_else(|| khata_talif(masar, self.nass, bidaya, "a token runs past the end"))
    }
}

/// Builds the failure for a `KeyValues` document that will not parse, with the
/// line a person can open the file to.
fn khata_talif(masar: &Path, nass: &str, mawdi: usize, tafsil: &str) -> KhataTathbeet {
    let satr = nass
        .get(..mawdi.min(nass.len()))
        .map_or(1, |sabiq| sabiq.matches('\n').count().saturating_add(1));
    khata_shakl(
        masar,
        "locating a value in a KeyValues file",
        format!("{tafsil} (line {satr}, byte {mawdi})"),
    )
}

/// Where an edit to one game's launch options has to happen.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MawqiKhiyarat {
    /// The value is there: this span, quotes included, is replaced.
    Qeema { bidaya: usize, nihaya: usize },
    /// The game's block is there and has no launch options: a line is inserted
    /// at this offset with this indentation.
    DakhilLuba { mawqi: usize, izaha: String },
    /// The per-app map is there and the game is not in it: a whole block is
    /// inserted at this offset with this indentation.
    DakhilAlaab { mawqi: usize, izaha: String },
}

/// The indentation of the line an offset falls on, plus one more level.
fn izahat_ibn(nass: &str, mawqi: usize) -> String {
    let bidayat_satr = nass
        .get(..mawqi.min(nass.len()))
        .and_then(|sabiq| sabiq.rfind('\n').map(|mawdi| mawdi.saturating_add(1)))
        .unwrap_or(0);
    let satr = nass.get(bidayat_satr..mawqi.min(nass.len())).unwrap_or_default();
    let izaha: String = satr.chars().take_while(|harf| *harf == '\t' || *harf == ' ').collect();
    let dakhili = if izaha.contains(' ') && !izaha.contains('\t') { "    " } else { "\t" };
    format!("{izaha}{dakhili}")
}

/// Finds where one game's launch options live in a `localconfig.vdf`.
fn mawqi_khiyarat(masar: &Path, nass: &str, app: &str) -> NatijatTathbeet<MawqiKhiyarat> {
    let mut masih = MasihVdf::jadeed(nass);
    let mut kudus: Vec<String> = Vec::new();
    let mut baad_alaab: Option<usize> = None;
    let mut baad_luba: Option<usize> = None;

    while let Some(wahda) = masih.wahda(masar)? {
        match wahda.naw {
            NawWahda::Ighlaq => {
                if kudus.pop().is_none() {
                    return Err(khata_talif(
                        masar,
                        nass,
                        wahda.bidaya,
                        "a closing brace with no matching open brace",
                    ));
                }
            }
            NawWahda::Fath => {
                return Err(khata_talif(
                    masar,
                    nass,
                    wahda.bidaya,
                    "an open brace where a key was expected",
                ));
            }
            NawWahda::Kalima(miftah) => {
                masih.takhatti_shart();
                let Some(baad) = masih.wahda(masar)? else {
                    return Err(khata_talif(
                        masar,
                        nass,
                        wahda.bidaya,
                        "the last key in the file has no value",
                    ));
                };
                match baad.naw {
                    NawWahda::Fath => {
                        kudus.push(miftah);
                        if yutabiq(&kudus, &MASAR_ALAAB) {
                            baad_alaab = Some(baad.nihaya);
                        } else if yutabiq_luba(&kudus, app) {
                            baad_luba = Some(baad.nihaya);
                        }
                    }
                    NawWahda::Kalima(_) => {
                        masih.takhatti_shart();
                        if yutabiq_luba(&kudus, app)
                            && miftah.eq_ignore_ascii_case(MIFTAH_KHIYARAT)
                        {
                            return Ok(MawqiKhiyarat::Qeema {
                                bidaya: baad.bidaya,
                                nihaya: baad.nihaya,
                            });
                        }
                    }
                    NawWahda::Ighlaq => {
                        return Err(khata_talif(
                            masar,
                            nass,
                            baad.bidaya,
                            "a key is followed by a closing brace instead of a value",
                        ));
                    }
                }
            }
        }
    }

    if !kudus.is_empty() {
        return Err(khata_talif(masar, nass, nass.len(), "a block is never closed"));
    }
    if let Some(mawqi) = baad_luba {
        return Ok(MawqiKhiyarat::DakhilLuba { mawqi, izaha: izahat_ibn(nass, mawqi) });
    }
    if let Some(mawqi) = baad_alaab {
        return Ok(MawqiKhiyarat::DakhilAlaab { mawqi, izaha: izahat_ibn(nass, mawqi) });
    }
    Err(khata_shakl(
        masar,
        "locating a value in a KeyValues file",
        format!(
            "this file has no {} block, so it is not the account configuration Steam keeps \
             launch options in and nothing was written to it",
            MASAR_ALAAB.join("/")
        ),
    ))
}

/// Whether a key stack is exactly this path, ignoring case at every step.
fn yutabiq(kudus: &[String], masar: &[&str]) -> bool {
    kudus.len() == masar.len()
        && kudus.iter().zip(masar.iter()).all(|(hali, matlub)| hali.eq_ignore_ascii_case(matlub))
}

/// Whether a key stack is the per-app map plus this game's identifier.
fn yutabiq_luba(kudus: &[String], app: &str) -> bool {
    let Some((akhir, sabiq)) = kudus.split_last() else { return false };
    akhir.eq_ignore_ascii_case(app) && yutabiq(sabiq, &MASAR_ALAAB)
}

/// Replaces or inserts one game's launch options, changing nothing else.
fn nass_bi_khiyarat(
    masar: &Path,
    nass: &str,
    app: &str,
    qeema: &str,
) -> NatijatTathbeet<String> {
    let muqtabasa = iqtibas_vdf(masar, qeema)?;
    let mawqi = mawqi_khiyarat(masar, nass, app)?;
    let mut kharij = String::with_capacity(nass.len().saturating_add(muqtabasa.len()));

    match mawqi {
        MawqiKhiyarat::Qeema { bidaya, nihaya } => {
            kharij.push_str(nass.get(..bidaya).unwrap_or_default());
            kharij.push_str(&muqtabasa);
            kharij.push_str(nass.get(nihaya..).unwrap_or_default());
        }
        MawqiKhiyarat::DakhilLuba { mawqi, izaha } => {
            kharij.push_str(nass.get(..mawqi).unwrap_or_default());
            let _ = write!(kharij, "\n{izaha}\"{MIFTAH_KHIYARAT}\"\t\t{muqtabasa}");
            kharij.push_str(nass.get(mawqi..).unwrap_or_default());
        }
        MawqiKhiyarat::DakhilAlaab { mawqi, izaha } => {
            let dakhili = if izaha.contains('\t') {
                format!("{izaha}\t")
            } else {
                format!("{izaha}    ")
            };
            kharij.push_str(nass.get(..mawqi).unwrap_or_default());
            let _ = write!(
                kharij,
                "\n{izaha}\"{app}\"\n{izaha}{{\n{dakhili}\"{MIFTAH_KHIYARAT}\"\t\t{muqtabasa}\n\
                 {izaha}}}"
            );
            kharij.push_str(nass.get(mawqi..).unwrap_or_default());
        }
    }
    Ok(kharij)
}

/// Removes one game's launch options key entirely, leaving its block behind.
fn nass_bila_khiyarat(masar: &Path, nass: &str, app: &str) -> NatijatTathbeet<String> {
    let MawqiKhiyarat::Qeema { nihaya, .. } = mawqi_khiyarat(masar, nass, app)? else {
        return Ok(nass.to_owned());
    };

    let mut masih = MasihVdf::jadeed(nass);
    let mut sabiq: Option<usize> = None;
    let mut hadhf: Option<(usize, usize)> = None;
    while let Some(wahda) = masih.wahda(masar)? {
        if wahda.nihaya == nihaya {
            if let Some(bidaya) = sabiq {
                hadhf = Some((bidaya, wahda.nihaya));
            }
            break;
        }
        sabiq = Some(wahda.bidaya);
    }
    let Some((bidaya, nihaya)) = hadhf else {
        return Err(khata_shakl(
            masar,
            "locating a value in a KeyValues file",
            "the launch options key was found and its own name was not, so nothing was \
             removed"
                .to_owned(),
        ));
    };

    let qabl = nass.get(..bidaya).unwrap_or_default();
    let baad = nass.get(nihaya..).unwrap_or_default();
    let mut mahdhuf = qabl.trim_end_matches([' ', '\t']);
    mahdhuf = mahdhuf
        .strip_suffix("\r\n")
        .or_else(|| mahdhuf.strip_suffix('\n'))
        .unwrap_or(mahdhuf);
    let mut kharij = String::with_capacity(nass.len());
    kharij.push_str(mahdhuf);
    kharij.push_str(baad.trim_start_matches([' ', '\t']));
    Ok(kharij)
}

/// One account's launch options for one Steam application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhiyaratSteam {
    malaf: PathBuf,
    app: String,
}

impl KhiyaratSteam {
    /// A writer for one `localconfig.vdf` and one application identifier.
    #[must_use]
    pub fn jadeeda(malaf: impl Into<PathBuf>, app: impl Into<String>) -> Self {
        Self { malaf: malaf.into(), app: app.into() }
    }

    /// A writer for one account's configuration directory.
    #[must_use]
    pub fn min_hisab(tahyia: &Path, app: impl Into<String>) -> Self {
        Self::jadeeda(tahyia.join(ISM_MALAF_STEAM), app)
    }

    /// A writer per account with a profile directory under a Steam root, most
    /// recently used first, located through `taarib-kashf`.
    #[must_use]
    pub fn li_kul_hisab(jidhr_steam: &Path, app: &str) -> Vec<Self> {
        taarib_kashf::matajir::steam::mustakhdimun(jidhr_steam)
            .iter()
            .map(|mustakhdim| Self::min_hisab(&mustakhdim.tahyia(), app))
            .filter(|khiyarat| khiyarat.malaf.is_file())
            .collect()
    }

    /// The file this writer edits.
    #[must_use]
    pub fn malaf(&self) -> &Path {
        &self.malaf
    }

    /// The application identifier this writer edits.
    #[must_use]
    pub fn app(&self) -> &str {
        &self.app
    }

    /// Where a change to these launch options is recorded.
    #[must_use]
    pub fn mahall(&self) -> MahallIdad {
        MahallIdad::KhiyaratTashghil {
            manassa: MANASSA_STEAM.to_owned(),
            muarrif_luba: self.app.clone(),
        }
    }

    /// The launch options this account currently has for this game.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::KhataMalaf`] when the file cannot be read or is not
    /// valid UTF-8, [`KhataTathbeet::HajmMufrit`] when it is above
    /// [`AQSA_HAJM_IDAD`], and the I/O variants for anything else.
    pub fn qeema_haliya(&self) -> NatijatTathbeet<Option<String>> {
        let nass = iqra_idad(&self.malaf, "reading Steam's launch options")?;
        self.qeema_min(&nass)
    }

    /// Adds Taarib's preload to whatever the user already had, recording the
    /// previous value before the file is rewritten.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MunassaTaamal`] while Steam is running and
    /// [`KhataTathbeet::HalatManassaMajhula`] when a sandbox hides whether it
    /// is, whatever recording the previous value raises, and
    /// [`KhataTathbeet::KhataMalaf`] when the file will not parse, has no
    /// `apps` block, or cannot be written.
    pub fn athbit(&self, muthabbit: &mut dyn Muthabbit) -> NatijatTathbeet<bool> {
        manassa_mughlaqa(&ASMAA_STEAM, ISM_STEAM, &self.malaf)?;
        let nass = iqra_idad(&self.malaf, "reading Steam's launch options")?;
        let sabiqa = self.qeema_min(&nass)?;
        let jadeeda = khiyarat_maa_tahmeel(sabiqa.as_deref());
        if sabiqa.as_deref() == Some(jadeeda.as_str()) {
            return Ok(false);
        }

        muthabbit.sajjil_idad(self.mahall(), sabiqa, Some(jadeeda.clone()))?;
        let kharij = nass_bi_khiyarat(&self.malaf, &nass, &self.app, &jadeeda)?;
        iktub_idad(&self.malaf, &kharij)?;
        Ok(true)
    }

    /// Reads this game's launch options out of a document already in memory.
    fn qeema_min(&self, nass: &str) -> NatijatTathbeet<Option<String>> {
        let shajara = vdf::iqra_nassi(nass).map_err(|khata| {
            khata_shakl(&self.malaf, "parsing Steam's account configuration", khata.injilizi)
        })?;
        let mut masar: Vec<&str> = MASAR_ALAAB.to_vec();
        masar.push(self.app.as_str());
        masar.push(MIFTAH_KHIYARAT);
        Ok(shajara.nass_bi_masar(&masar).map(str::to_owned))
    }

    /// Refuses a record that belongs to another launcher or another game.
    fn tahaqquq_min(&self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        match &sijill.mahall {
            MahallIdad::KhiyaratTashghil { manassa, muarrif_luba }
                if manassa.eq_ignore_ascii_case(MANASSA_STEAM) && *muarrif_luba == self.app =>
            {
                Ok(())
            }
            _ => Err(khata_idad(
                sijill,
                format!(
                    "this restorer was built for {MANASSA_STEAM} application {} and was handed \
                     a record for something else, so it refused rather than edit the wrong \
                     game's options",
                    self.app
                ),
            )),
        }
    }
}

impl RadIdad for KhiyaratSteam {
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        self.tahaqquq_min(sijill)?;
        manassa_mughlaqa(&ASMAA_STEAM, ISM_STEAM, &self.malaf)?;

        let nass = iqra_idad(&self.malaf, "reading Steam's launch options")
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        let hali = self
            .qeema_min(&nass)
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;

        // A field the user cleared is not one Taarib's record may resurrect.
        let Some(hali) = hali else { return Ok(()) };

        let kharij = if sijill.qeema_maktuba.as_deref() == Some(hali.as_str()) {
            match &sijill.qeema_sabiqa {
                Some(sabiqa) => nass_bi_khiyarat(&self.malaf, &nass, &self.app, sabiqa),
                None => nass_bila_khiyarat(&self.malaf, &nass, &self.app),
            }
        } else {
            let kan_fih_amr = sijill.qeema_sabiqa.as_deref().is_some_and(fihi_ramz_amr);
            let baqiya = khiyarat_bidun_tahmeel(&hali, kan_fih_amr);
            if baqiya.is_empty() {
                nass_bila_khiyarat(&self.malaf, &nass, &self.app)
            } else {
                nass_bi_khiyarat(&self.malaf, &nass, &self.app, &baqiya)
            }
        }
        .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;

        iktub_idad(&self.malaf, &kharij)
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))
    }
}

/// The English sentence of a failure, for a refusal that carries a reason
/// rather than a source.
fn khata_ila_sabab(khata: &KhataTathbeet) -> String {
    khata.injilizi()
}

/// A launcher that puts itself between the user and the game's executable and
/// keeps one JSON file per game, such as Heroic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhiyaratMughallif {
    malaf: PathBuf,
    manassa: String,
    ism: String,
    muarrif_luba: String,
    kaain: Option<String>,
    miftah: String,
    asmaa: Vec<String>,
}

impl KhiyaratMughallif {
    /// A writer for one launcher's per-game JSON file.
    #[must_use]
    pub fn jadeeda(
        malaf: impl Into<PathBuf>,
        manassa: impl Into<String>,
        ism: impl Into<String>,
        muarrif_luba: impl Into<String>,
        kaain: Option<String>,
        miftah: impl Into<String>,
        asmaa: &[&str],
    ) -> Self {
        Self {
            malaf: malaf.into(),
            manassa: manassa.into(),
            ism: ism.into(),
            muarrif_luba: muarrif_luba.into(),
            kaain,
            miftah: miftah.into(),
            asmaa: asmaa.iter().map(|ism| (*ism).to_owned()).collect(),
        }
    }

    /// A writer for one game under a Heroic configuration root.
    #[must_use]
    pub fn li_heroic(jidhr: &Path, app: &str, miftah: impl Into<String>) -> Self {
        Self::jadeeda(
            jidhr.join(MUJALLAD_HEROIC).join(format!("{app}.json")),
            MANASSA_HEROIC,
            ISM_HEROIC,
            app,
            Some(app.to_owned()),
            miftah,
            &ASMAA_HEROIC,
        )
    }

    /// The file this writer edits.
    #[must_use]
    pub fn malaf(&self) -> &Path {
        &self.malaf
    }

    /// Where a change to this launcher's command prefix is recorded.
    #[must_use]
    pub fn mahall(&self) -> MahallIdad {
        MahallIdad::KhiyaratTashghil {
            manassa: self.manassa.clone(),
            muarrif_luba: self.muarrif_luba.clone(),
        }
    }

    /// The command prefix this launcher currently runs the game through.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::KhataMalaf`] when the file cannot be read, is not valid
    /// UTF-8, is not JSON, or holds no object for this game.
    pub fn qeema_haliya(&self) -> NatijatTathbeet<Option<String>> {
        let nass = iqra_idad(&self.malaf, "reading a launcher's command prefix")?;
        let wathiqa = self.hallil(&nass)?;
        Ok(self.qeema_min(&wathiqa))
    }

    /// Adds Taarib's preload to the launcher's command prefix.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MunassaTaamal`] while the launcher is running and
    /// [`KhataTathbeet::HalatManassaMajhula`] when a sandbox hides whether it
    /// is, whatever recording the previous value raises, and
    /// [`KhataTathbeet::KhataMalaf`] when the file will not parse or cannot be
    /// written.
    pub fn athbit(&self, muthabbit: &mut dyn Muthabbit) -> NatijatTathbeet<bool> {
        manassa_mughlaqa(&self.asmaa_marja(), &self.ism, &self.malaf)?;
        let nass = iqra_idad(&self.malaf, "reading a launcher's command prefix")?;
        let mut wathiqa = self.hallil(&nass)?;
        let sabiqa = self.qeema_min(&wathiqa);
        let jadeeda = badiyat_amr_maa_tahmeel(sabiqa.as_deref());
        if sabiqa.as_deref() == Some(jadeeda.as_str()) {
            return Ok(false);
        }

        muthabbit.sajjil_idad(self.mahall(), sabiqa, Some(jadeeda.clone()))?;
        self.daa(&mut wathiqa, Some(&jadeeda))?;
        self.iktub(&wathiqa)?;
        Ok(true)
    }

    /// The executable names as a borrowed slice for the running check.
    fn asmaa_marja(&self) -> Vec<&str> {
        self.asmaa.iter().map(String::as_str).collect()
    }

    /// Parses the file into a JSON document.
    fn hallil(&self, nass: &str) -> NatijatTathbeet<Value> {
        serde_json::from_str(nass).map_err(|sabab| {
            khata_shakl(
                &self.malaf,
                "parsing a launcher's per-game configuration",
                format!("the file is not JSON this build can read: {sabab}"),
            )
        })
    }

    /// The object holding this game's settings, when the file has one.
    fn kaain_luba<'a>(&self, wathiqa: &'a Value) -> Option<&'a Value> {
        match &self.kaain {
            Some(miftah) => wathiqa.get(miftah).filter(|qeema| qeema.is_object()),
            None => wathiqa.as_object().map(|_| wathiqa),
        }
    }

    /// The current command prefix, when the game's object records one.
    fn qeema_min(&self, wathiqa: &Value) -> Option<String> {
        self.kaain_luba(wathiqa)?
            .get(&self.miftah)?
            .as_str()
            .filter(|qeema| !qeema.is_empty())
            .map(str::to_owned)
    }

    /// Writes or removes the command prefix inside the game's object.
    fn daa(&self, wathiqa: &mut Value, qeema: Option<&str>) -> Result<(), KhataTathbeet> {
        let hadaf = match &self.kaain {
            Some(miftah) => wathiqa.get_mut(miftah).filter(|qeema| qeema.is_object()),
            None => Some(wathiqa),
        };
        let Some(kaain) = hadaf.and_then(Value::as_object_mut) else {
            return Err(khata_shakl(
                &self.malaf,
                "editing a launcher's per-game configuration",
                match &self.kaain {
                    Some(miftah) => format!(
                        "this file holds no object under {miftah}, so it is not this game's \
                         configuration and nothing was written to it"
                    ),
                    None => "this file is not a JSON object, so there is no field to set"
                        .to_owned(),
                },
            ));
        };
        match qeema {
            Some(qeema) => {
                let _ = kaain.insert(self.miftah.clone(), Value::String(qeema.to_owned()));
            }
            None => {
                let _ = kaain.remove(&self.miftah);
            }
        }
        Ok(())
    }

    /// Writes the document back, indented as a JSON editor would leave it.
    fn iktub(&self, wathiqa: &Value) -> Result<(), KhataTathbeet> {
        let mut kharij = serde_json::to_string_pretty(wathiqa).map_err(|sabab| {
            khata_shakl(
                &self.malaf,
                "writing a launcher's per-game configuration",
                sabab.to_string(),
            )
        })?;
        kharij.push('\n');
        iktub_idad(&self.malaf, &kharij)
    }

    /// Refuses a record that belongs to another launcher or another game.
    fn tahaqquq_min(&self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        match &sijill.mahall {
            MahallIdad::KhiyaratTashghil { manassa, muarrif_luba }
                if manassa.eq_ignore_ascii_case(&self.manassa)
                    && *muarrif_luba == self.muarrif_luba =>
            {
                Ok(())
            }
            _ => Err(khata_idad(
                sijill,
                format!(
                    "this restorer was built for {} game {} and was handed a record for \
                     something else, so it refused rather than edit the wrong configuration",
                    self.manassa, self.muarrif_luba
                ),
            )),
        }
    }
}

impl RadIdad for KhiyaratMughallif {
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        self.tahaqquq_min(sijill)?;
        manassa_mughlaqa(&self.asmaa_marja(), &self.ism, &self.malaf)?;

        let nass = iqra_idad(&self.malaf, "reading a launcher's command prefix")
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        let mut wathiqa =
            self.hallil(&nass).map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        let Some(hali) = self.qeema_min(&wathiqa) else { return Ok(()) };

        let matlub = if sijill.qeema_maktuba.as_deref() == Some(hali.as_str()) {
            sijill.qeema_sabiqa.clone()
        } else {
            let kan_fih_beea = sijill.qeema_sabiqa.as_deref().is_some_and(yabda_bi_beea);
            let baqiya = badiyat_amr_bidun_tahmeel(&hali, kan_fih_beea);
            if baqiya.is_empty() { None } else { Some(baqiya) }
        };

        self.daa(&mut wathiqa, matlub.as_deref())
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        self.iktub(&wathiqa).map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))
    }
}

/// One game's command prefix in its Lutris configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhiyaratLutris {
    malaf: PathBuf,
    silaa: String,
}

impl KhiyaratLutris {
    /// A writer for one Lutris game configuration file.
    #[must_use]
    pub fn jadeeda(malaf: impl Into<PathBuf>, silaa: impl Into<String>) -> Self {
        Self { malaf: malaf.into(), silaa: silaa.into() }
    }

    /// The file this writer edits.
    #[must_use]
    pub fn malaf(&self) -> &Path {
        &self.malaf
    }

    /// Where a change to this game's command prefix is recorded.
    #[must_use]
    pub fn mahall(&self) -> MahallIdad {
        MahallIdad::KhiyaratTashghil {
            manassa: MANASSA_LUTRIS.to_owned(),
            muarrif_luba: self.silaa.clone(),
        }
    }

    /// The command prefix this game currently runs through.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::KhataMalaf`] when the file cannot be read, is not valid
    /// UTF-8, or indents with tabs, which YAML forbids and which this editor
    /// refuses rather than guess the width of.
    pub fn qeema_haliya(&self) -> NatijatTathbeet<Option<String>> {
        let nass = iqra_idad(&self.malaf, "reading a Lutris command prefix")?;
        self.qeema_min(&nass)
    }

    /// Adds Taarib's preload to this game's command prefix.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MunassaTaamal`] while Lutris is running and
    /// [`KhataTathbeet::HalatManassaMajhula`] when a sandbox hides whether it
    /// is, whatever recording the previous value raises, and
    /// [`KhataTathbeet::KhataMalaf`] when the file cannot be read, indents with
    /// tabs, holds `system:` as something other than a block mapping, or cannot
    /// be written.
    pub fn athbit(&self, muthabbit: &mut dyn Muthabbit) -> NatijatTathbeet<bool> {
        manassa_mughlaqa(&ASMAA_LUTRIS, ISM_LUTRIS, &self.malaf)?;
        let nass = iqra_idad(&self.malaf, "reading a Lutris command prefix")?;
        let sabiqa = self.qeema_min(&nass)?;
        let jadeeda = badiyat_amr_maa_tahmeel(sabiqa.as_deref());
        if sabiqa.as_deref() == Some(jadeeda.as_str()) {
            return Ok(false);
        }

        muthabbit.sajjil_idad(self.mahall(), sabiqa, Some(jadeeda.clone()))?;
        let kharij = self.bi_qeema(&nass, Some(&jadeeda))?;
        iktub_idad(&self.malaf, &kharij)?;
        Ok(true)
    }

    /// Reads the command prefix out of a document already in memory.
    fn qeema_min(&self, nass: &str) -> NatijatTathbeet<Option<String>> {
        let sutur = self.sutur(nass)?;
        let Some(nizam) = mawqi_miftah(&sutur, None, MIFTAH_LUTRIS_NIZAM) else {
            return Ok(None);
        };
        if !qeemat_satr(&sutur, nizam).is_empty() {
            return Err(khata_shakl(
                &self.malaf,
                "reading a Lutris command prefix",
                format!(
                    "{MIFTAH_LUTRIS_NIZAM} holds a value on its own line rather than a block \
                     of keys, so this editor refused it rather than guess what replacing it \
                     would mean"
                ),
            ));
        }
        let Some(mawqi) = mawqi_miftah(&sutur, Some(nizam), MIFTAH_LUTRIS) else {
            return Ok(None);
        };
        let qeema = qeemat_khaam(&qeemat_satr(&sutur, mawqi));
        Ok(if qeema.is_empty() { None } else { Some(qeema) })
    }

    /// The document's lines, refusing indentation this editor will not touch.
    fn sutur<'a>(&self, nass: &'a str) -> NatijatTathbeet<Vec<&'a str>> {
        let sutur: Vec<&str> = nass.split('\n').collect();
        for satr in &sutur {
            let bila_izaha = satr.trim_start_matches([' ', '\t']);
            let tul = satr.len().saturating_sub(bila_izaha.len());
            if satr.get(..tul).is_some_and(|badiya| badiya.contains('\t')) {
                return Err(khata_shakl(
                    &self.malaf,
                    "editing a Lutris configuration",
                    "a line is indented with a tab, which YAML forbids and whose width this \
                     editor will not guess"
                        .to_owned(),
                ));
            }
        }
        Ok(sutur)
    }

    /// Writes or removes the command prefix, leaving every other line alone.
    fn bi_qeema(&self, nass: &str, qeema: Option<&str>) -> NatijatTathbeet<String> {
        let sutur = self.sutur(nass)?;
        let nihaya = if nass.ends_with('\n') { "\n" } else { "" };
        let mut mabniya: Vec<String> = sutur.iter().map(|satr| (*satr).to_owned()).collect();
        // `split('\n')` leaves a trailing empty element that `nihaya` puts back.
        if !nihaya.is_empty() {
            let _ = mabniya.pop();
        }

        let nizam = mawqi_miftah(&sutur, None, MIFTAH_LUTRIS_NIZAM);
        let mawqi = nizam.and_then(|nizam| mawqi_miftah(&sutur, Some(nizam), MIFTAH_LUTRIS));

        match (mawqi, qeema) {
            (Some(mawqi), Some(qeema)) => {
                let izaha = izahat_satr(sutur.get(mawqi).copied().unwrap_or_default());
                let muqtabasa = iqtibas_yaml(&self.malaf, qeema)?;
                if let Some(satr) = mabniya.get_mut(mawqi) {
                    *satr = format!("{izaha}{MIFTAH_LUTRIS}: {muqtabasa}");
                }
            }
            (Some(mawqi), None) => {
                if mawqi < mabniya.len() {
                    let _ = mabniya.remove(mawqi);
                }
            }
            (None, None) => {}
            (None, Some(qeema)) => {
                let muqtabasa = iqtibas_yaml(&self.malaf, qeema)?;
                if let Some(nizam) = nizam {
                    let walid = izahat_satr(sutur.get(nizam).copied().unwrap_or_default());
                    let izaha = format!("{walid}  ");
                    mabniya.insert(
                        nizam.saturating_add(1).min(mabniya.len()),
                        format!("{izaha}{MIFTAH_LUTRIS}: {muqtabasa}"),
                    );
                } else {
                    mabniya.push(format!("{MIFTAH_LUTRIS_NIZAM}:"));
                    mabniya.push(format!("  {MIFTAH_LUTRIS}: {muqtabasa}"));
                }
            }
        }

        Ok(format!("{}{nihaya}", mabniya.join("\n")))
    }

    /// Refuses a record that belongs to another launcher or another game.
    fn tahaqquq_min(&self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        match &sijill.mahall {
            MahallIdad::KhiyaratTashghil { manassa, muarrif_luba }
                if manassa.eq_ignore_ascii_case(MANASSA_LUTRIS) && *muarrif_luba == self.silaa =>
            {
                Ok(())
            }
            _ => Err(khata_idad(
                sijill,
                format!(
                    "this restorer was built for {MANASSA_LUTRIS} game {} and was handed a \
                     record for something else, so it refused rather than edit the wrong \
                     configuration",
                    self.silaa
                ),
            )),
        }
    }
}

impl RadIdad for KhiyaratLutris {
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        self.tahaqquq_min(sijill)?;
        manassa_mughlaqa(&ASMAA_LUTRIS, ISM_LUTRIS, &self.malaf)?;

        let nass = iqra_idad(&self.malaf, "reading a Lutris command prefix")
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        let hali = self
            .qeema_min(&nass)
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        let Some(hali) = hali else { return Ok(()) };

        let matlub = if sijill.qeema_maktuba.as_deref() == Some(hali.as_str()) {
            sijill.qeema_sabiqa.clone()
        } else {
            let kan_fih_beea = sijill.qeema_sabiqa.as_deref().is_some_and(yabda_bi_beea);
            let baqiya = badiyat_amr_bidun_tahmeel(&hali, kan_fih_beea);
            if baqiya.is_empty() { None } else { Some(baqiya) }
        };

        let kharij = self
            .bi_qeema(&nass, matlub.as_deref())
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?;
        iktub_idad(&self.malaf, &kharij)
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))
    }
}

/// The leading spaces of a line.
fn izahat_satr(satr: &str) -> String {
    satr.chars().take_while(|harf| *harf == ' ').collect()
}

/// The indentation of a line, in spaces, or [`None`] for a line that is blank
/// or is a whole-line comment.
fn izahat_maani(satr: &str) -> Option<usize> {
    let bila_izaha = satr.trim_start_matches(' ');
    let maani = bila_izaha.trim_end();
    if maani.is_empty() || maani.starts_with('#') {
        return None;
    }
    Some(satr.len().saturating_sub(bila_izaha.len()))
}

/// The index of the line holding a key, at the document root or inside a block.
fn mawqi_miftah(sutur: &[&str], walid: Option<usize>, miftah: &str) -> Option<usize> {
    let (bidaya, mustawa) = match walid {
        Some(walid) => (
            walid.saturating_add(1),
            izahat_maani(sutur.get(walid).copied().unwrap_or_default()).unwrap_or(0),
        ),
        None => (0, sutur.iter().copied().filter_map(izahat_maani).min().unwrap_or(0)),
    };

    let mut mustawa_ibn: Option<usize> = None;
    for (mawqi, satr) in sutur.iter().enumerate().skip(bidaya) {
        let Some(izaha) = izahat_maani(satr) else { continue };
        if walid.is_some() {
            if izaha <= mustawa {
                return None;
            }
            match mustawa_ibn {
                Some(matlub) if izaha != matlub => continue,
                Some(_) => {}
                None => mustawa_ibn = Some(izaha),
            }
        } else if izaha != mustawa {
            continue;
        }
        if miftah_satr(satr).is_some_and(|ism| ism == miftah) {
            return Some(mawqi);
        }
    }
    None
}

/// The key a mapping line declares, unquoted, or [`None`] when the line is not
/// a mapping entry.
fn miftah_satr(satr: &str) -> Option<String> {
    let maani = satr.trim();
    if maani.is_empty() || maani.starts_with('#') || maani.starts_with('-') {
        return None;
    }
    let (miftah, _) = maani.split_once(':')?;
    let miftah = miftah.trim();
    Some(miftah.trim_matches(['"', '\'']).to_owned())
}

/// The raw text after a mapping line's colon, with a trailing comment removed.
fn qeemat_satr(sutur: &[&str], mawqi: usize) -> String {
    let satr = sutur.get(mawqi).copied().unwrap_or_default().trim();
    let Some((_, baqi)) = satr.split_once(':') else { return String::new() };
    let baqi = baqi.trim();
    if baqi.starts_with('"') || baqi.starts_with('\'') {
        return baqi.to_owned();
    }
    match baqi.split_once(" #") {
        Some((qabl, _)) => qabl.trim().to_owned(),
        None => baqi.to_owned(),
    }
}

/// Unquotes a YAML scalar the way `taarib-kashf`'s reader does for the two
/// quoting styles Lutris writes.
fn qeemat_khaam(khaam: &str) -> String {
    let khaam = khaam.trim();
    if let Some(dakhil) = khaam.strip_prefix('\'').and_then(|baqi| baqi.strip_suffix('\'')) {
        return dakhil.replace("''", "'");
    }
    let Some(dakhil) = khaam.strip_prefix('"').and_then(|baqi| baqi.strip_suffix('"')) else {
        if matches!(khaam, "~" | "null" | "Null" | "NULL") {
            return String::new();
        }
        return khaam.to_owned();
    };
    let mut kharij = String::with_capacity(dakhil.len());
    let mut ahruf = dakhil.chars();
    while let Some(harf) = ahruf.next() {
        if harf != '\\' {
            kharij.push(harf);
            continue;
        }
        match ahruf.next() {
            Some('n') => kharij.push('\n'),
            Some('t') => kharij.push('\t'),
            Some('r') => kharij.push('\r'),
            Some('"') => kharij.push('"'),
            // A trailing backslash is kept as itself, exactly as an escaped one
            // is: this subset has no continuation line to fold it into.
            Some('\\') | None => kharij.push('\\'),
            Some(akhar) => {
                kharij.push('\\');
                kharij.push(akhar);
            }
        }
    }
    kharij
}

/// Quotes a YAML scalar in the one style every reader of this subset accepts.
///
/// # Errors
///
/// [`KhataTathbeet::KhataMalaf`] for a value holding a line break, which a
/// double-quoted scalar can only carry as an escape this editor does not emit
/// and which would otherwise split the mapping in two.
fn iqtibas_yaml(masar: &Path, qeema: &str) -> NatijatTathbeet<String> {
    if qeema.contains('\n') || qeema.contains('\r') {
        return Err(khata_shakl(
            masar,
            "escaping a YAML scalar",
            "the value holds a line break, which would split the mapping".to_owned(),
        ));
    }
    let mut kharij = String::with_capacity(qeema.len().saturating_add(2));
    kharij.push('"');
    for harf in qeema.chars() {
        match harf {
            '\\' => kharij.push_str("\\\\"),
            '"' => kharij.push_str("\\\""),
            '\t' => kharij.push_str("\\t"),
            _ => kharij.push(harf),
        }
    }
    kharij.push('"');
    Ok(kharij)
}

/// One environment variable in a `NAME=value` file Taarib composes the game's
/// launch environment from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdadBeea {
    malaf: PathBuf,
    ism: String,
}

impl IdadBeea {
    /// A writer for one variable in one environment file.
    #[must_use]
    pub fn jadeed(malaf: impl Into<PathBuf>, ism: impl Into<String>) -> Self {
        Self { malaf: malaf.into(), ism: ism.into() }
    }

    /// A writer for the DLL override variable Proton and Wine read.
    #[must_use]
    pub fn li_tajawuz(malaf: impl Into<PathBuf>) -> Self {
        Self::jadeed(malaf, MUTAGHAYYIR_TAJAWUZ)
    }

    /// The file this writer edits.
    #[must_use]
    pub fn malaf(&self) -> &Path {
        &self.malaf
    }

    /// The variable this writer sets.
    #[must_use]
    pub fn ism(&self) -> &str {
        &self.ism
    }

    /// Where a change to this variable is recorded.
    #[must_use]
    pub fn mahall(&self) -> MahallIdad {
        MahallIdad::MutaghayyirBeea { ism: self.ism.clone() }
    }

    /// The value the file currently assigns, or [`None`] when it assigns none
    /// or is not there yet.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::KhataMalaf`] when the file exists and cannot be read or
    /// is not valid UTF-8.
    pub fn qeema_haliya(&self) -> NatijatTathbeet<Option<String>> {
        let Some(nass) = self.iqra()? else { return Ok(None) };
        Ok(self.qeema_min(&nass))
    }

    /// Adds Taarib's override entry to this variable, keeping any the user set.
    ///
    /// # Errors
    ///
    /// Whatever recording the previous value raises, and
    /// [`KhataTathbeet::KhataMalaf`] when the file cannot be read or written.
    pub fn athbit_tajawuz(&self, muthabbit: &mut dyn Muthabbit) -> NatijatTathbeet<bool> {
        let sabiqa = self.qeema_haliya()?;
        self.athbit(muthabbit, &tajawuz_maa(sabiqa.as_deref()))
    }

    /// Sets this variable to an explicit value, recording what it was first.
    ///
    /// # Errors
    ///
    /// As [`IdadBeea::athbit_tajawuz`].
    pub fn athbit(&self, muthabbit: &mut dyn Muthabbit, qeema: &str) -> NatijatTathbeet<bool> {
        let nass = self.iqra()?.unwrap_or_default();
        let sabiqa = self.qeema_min(&nass);
        if sabiqa.as_deref() == Some(qeema) {
            return Ok(false);
        }

        muthabbit.sajjil_idad(self.mahall(), sabiqa, Some(qeema.to_owned()))?;
        let kharij = self.bi_qeema(&nass, Some(qeema));
        iktub_idad(&self.malaf, &kharij)?;
        Ok(true)
    }

    /// Reads the file, or [`None`] when it is not there.
    fn iqra(&self) -> NatijatTathbeet<Option<String>> {
        if !self.malaf.exists() {
            return Ok(None);
        }
        iqra_idad(&self.malaf, "reading the launch environment").map(Some)
    }

    /// The last assignment of this variable in a document already in memory.
    fn qeema_min(&self, nass: &str) -> Option<String> {
        nass.lines()
            .filter_map(|satr| self.qeemat_satr(satr))
            .next_back()
            .filter(|qeema| !qeema.is_empty())
    }

    /// The value a line assigns to this variable, when it assigns one.
    fn qeemat_satr(&self, satr: &str) -> Option<String> {
        let maani = satr.trim();
        let baqi = maani.strip_prefix("export ").unwrap_or(maani).trim_start();
        let (ism, qeema) = baqi.split_once('=')?;
        if !ism.trim_end().eq_ignore_ascii_case(&self.ism) {
            return None;
        }
        let qeema = qeema.trim();
        let bila_iqtibas = qeema
            .strip_prefix('"')
            .and_then(|dakhil| dakhil.strip_suffix('"'))
            .or_else(|| qeema.strip_prefix('\'').and_then(|dakhil| dakhil.strip_suffix('\'')));
        Some(bila_iqtibas.unwrap_or(qeema).to_owned())
    }

    /// Rewrites, removes or appends this variable's assignment.
    fn bi_qeema(&self, nass: &str, qeema: Option<&str>) -> String {
        let nihaya = if nass.is_empty() || nass.ends_with('\n') { "\n" } else { "" };
        let mut sutur: Vec<String> = Vec::new();
        let mut kutibat = false;
        for satr in nass.lines() {
            if self.qeemat_satr(satr).is_none() {
                sutur.push(satr.to_owned());
                continue;
            }
            if let Some(qeema) = qeema
                && !kutibat {
                    sutur.push(format!("{}={}", self.ism, iqtibas_beea(qeema)));
                    kutibat = true;
                }
        }
        if let Some(qeema) = qeema
            && !kutibat {
                sutur.push(format!("{}={}", self.ism, iqtibas_beea(qeema)));
            }
        if sutur.is_empty() {
            return String::new();
        }
        format!("{}{nihaya}", sutur.join("\n"))
    }

    /// Refuses a record for another variable.
    fn tahaqquq_min(&self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        match &sijill.mahall {
            MahallIdad::MutaghayyirBeea { ism } if ism.eq_ignore_ascii_case(&self.ism) => Ok(()),
            _ => Err(khata_idad(
                sijill,
                format!(
                    "this restorer was built for {} and was handed a record for something \
                     else, so it refused rather than clear the wrong variable",
                    self.ism
                ),
            )),
        }
    }
}

impl RadIdad for IdadBeea {
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        self.tahaqquq_min(sijill)?;
        let Some(nass) = self.iqra().map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))?
        else {
            return Ok(());
        };
        let Some(hali) = self.qeema_min(&nass) else { return Ok(()) };

        let matlub = if sijill.qeema_maktuba.as_deref() == Some(hali.as_str()) {
            sijill.qeema_sabiqa.clone()
        } else if self.ism.eq_ignore_ascii_case(MUTAGHAYYIR_TAJAWUZ) {
            bidun_tajawuz(&hali)
        } else {
            return Err(khata_idad(
                sijill,
                format!(
                    "{} now holds {hali}, which is neither what Taarib wrote nor a list this \
                     build knows how to subtract its own entry from, so it was left alone \
                     rather than overwritten",
                    self.ism
                ),
            ));
        };

        let kharij = self.bi_qeema(&nass, matlub.as_deref());
        iktub_idad(&self.malaf, &kharij)
            .map_err(|khata| khata_idad(sijill, khata_ila_sabab(&khata)))
    }
}

/// Quotes an environment value when it holds anything a reader could split on.
fn iqtibas_beea(qeema: &str) -> String {
    let basit = !qeema.is_empty()
        && qeema.chars().all(|harf| {
            harf.is_ascii_alphanumeric() || matches!(harf, '_' | '-' | '.' | '/' | ':' | '=' | ',')
        });
    if basit {
        return qeema.to_owned();
    }
    let mut kharij = String::with_capacity(qeema.len().saturating_add(2));
    kharij.push('"');
    for harf in qeema.chars() {
        if harf == '"' || harf == '\\' {
            kharij.push('\\');
        }
        kharij.push(harf);
    }
    kharij.push('"');
    kharij
}

/// The [`RadIdad`] that puts every launch setting Taarib changed back.
#[derive(Debug, Clone, Default)]
pub struct RadItlaq {
    khiyarat: Vec<KhiyaratSteam>,
    mughallifat: Vec<KhiyaratMughallif>,
    lutris: Vec<KhiyaratLutris>,
    beea: Vec<IdadBeea>,
}

impl RadItlaq {
    /// An empty restorer, which refuses everything until it is given a writer.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Adds a Steam launch-options writer.
    #[must_use]
    pub fn bi_steam(mut self, khiyarat: KhiyaratSteam) -> Self {
        self.khiyarat.push(khiyarat);
        self
    }

    /// Adds a wrapper launcher's command-prefix writer.
    #[must_use]
    pub fn bi_mughallif(mut self, mughallif: KhiyaratMughallif) -> Self {
        self.mughallifat.push(mughallif);
        self
    }

    /// Adds a Lutris command-prefix writer.
    #[must_use]
    pub fn bi_lutris(mut self, lutris: KhiyaratLutris) -> Self {
        self.lutris.push(lutris);
        self
    }

    /// Adds an environment-file writer.
    #[must_use]
    pub fn bi_beea(mut self, beea: IdadBeea) -> Self {
        self.beea.push(beea);
        self
    }

    /// How many writers this restorer holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.khiyarat
            .len()
            .saturating_add(self.mughallifat.len())
            .saturating_add(self.lutris.len())
            .saturating_add(self.beea.len())
    }
}

impl RadIdad for RadItlaq {
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        match &sijill.mahall {
            MahallIdad::KhiyaratTashghil { manassa, muarrif_luba } => {
                if manassa.eq_ignore_ascii_case(MANASSA_STEAM) {
                    if let Some(katib) =
                        self.khiyarat.iter_mut().find(|katib| katib.app == *muarrif_luba)
                    {
                        return katib.rudd(sijill);
                    }
                } else if manassa.eq_ignore_ascii_case(MANASSA_LUTRIS) {
                    if let Some(katib) =
                        self.lutris.iter_mut().find(|katib| katib.silaa == *muarrif_luba)
                    {
                        return katib.rudd(sijill);
                    }
                } else if let Some(katib) = self.mughallifat.iter_mut().find(|katib| {
                    katib.manassa.eq_ignore_ascii_case(manassa)
                        && katib.muarrif_luba == *muarrif_luba
                }) {
                    return katib.rudd(sijill);
                }
                Err(khata_idad(
                    sijill,
                    format!(
                        "this uninstall was given no writer for {manassa} game {muarrif_luba}, \
                         so the option is still in place and was not reported as restored"
                    ),
                ))
            }
            MahallIdad::MutaghayyirBeea { ism } => {
                if let Some(katib) =
                    self.beea.iter_mut().find(|katib| katib.ism.eq_ignore_ascii_case(ism))
                {
                    return katib.rudd(sijill);
                }
                Err(khata_idad(
                    sijill,
                    format!(
                        "this uninstall was given no environment file to clear {ism} from, so \
                         the variable is still set and was not reported as restored"
                    ),
                ))
            }
            MahallIdad::SijillWindows { .. } | MahallIdad::MalafIdad { .. } => Err(khata_idad(
                sijill,
                format!(
                    "this setting is not one launch integration writes; it belongs to {}",
                    sijill.mahall.masul()
                ),
            )),
        }
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
    use std::path::Path;

    use taarib_usus::khata::{Khutwa, Tafsir as _};
    use taarib_usus::manassa::{RuyatAmaliyat, Sunduq, ruyat_amaliyat};

    use super::{ASMAA_STEAM, HalatTashghil, KhataTathbeet, halat_manassa, hukm_manassa};

    /// Executable names nothing on any machine is running.
    const ASMAA_MUSTAHILA: [&str; 2] =
        ["la-tujad-hadhihi-al-manassa", "wala-hadhihi-al-manassa-aydan"];

    fn malaf() -> &'static Path {
        Path::new("/taarib/localconfig.vdf")
    }

    #[test]
    fn hukm_al_manassa_yaftah_li_hala_wahida() {
        assert!(hukm_manassa(HalatTashghil::LaTashtaghil, "Steam", malaf()).is_ok());
        assert!(hukm_manassa(HalatTashghil::Tashtaghil, "Steam", malaf()).is_err());
        for sunduq in [Sunduq::Flatpak, Sunduq::Snap, Sunduq::Hawiya] {
            assert!(
                hukm_manassa(HalatTashghil::GhayrMaaruf { sunduq }, "Steam", malaf()).is_err(),
                "a launcher that could not be seen is not a launcher seen closed"
            );
        }
    }

    #[test]
    fn al_halatan_al_rafidatan_laysata_nafs_al_jumla() {
        let taamal = hukm_manassa(HalatTashghil::Tashtaghil, "Steam", malaf())
            .expect_err("a running launcher refuses");
        assert!(matches!(taamal, KhataTathbeet::MunassaTaamal { .. }));
        assert!(taamal.injilizi().contains("Steam is running"));
        // "Close it and try again" is advice for somebody who was told a fact.
        assert!(taamal.qabil_lil_iada());
        assert_eq!(taamal.khutwa(), Khutwa::AadaMuhawala);

        let majhula = hukm_manassa(
            HalatTashghil::GhayrMaaruf { sunduq: Sunduq::Flatpak },
            "Steam",
            malaf(),
        )
        .expect_err("a blind launcher guard refuses");
        assert!(matches!(majhula, KhataTathbeet::HalatManassaMajhula { .. }));
        assert!(majhula.injilizi().contains("Flatpak"));
        assert!(majhula.injilizi().contains("cannot tell whether Steam is running"));
        assert!(!majhula.injilizi().contains("Steam is running and"));
        assert!(majhula.arabi().contains("فلاتباك"));
        assert!(majhula.arabi().contains("تعذّر عليه معرفة"));
        // Nothing the user does here changes the answer, so nothing is offered.
        assert!(!majhula.qabil_lil_iada());
        assert_eq!(majhula.khutwa(), Khutwa::LaShay);
        assert_eq!(majhula.masar(), Some(malaf()));
    }

    #[test]
    fn tayy_al_asmaa_yatbaa_ruyat_hadhihi_al_ala() {
        // Nothing on the machine carries either name, so the fold's answer is
        // decided entirely by whether this build can see the host at all.
        let hala = halat_manassa(&ASMAA_MUSTAHILA);
        match ruyat_amaliyat() {
            RuyatAmaliyat::Kamila => assert_eq!(hala, HalatTashghil::LaTashtaghil),
            RuyatAmaliyat::Maazula { sunduq } => {
                assert_eq!(hala, HalatTashghil::GhayrMaaruf { sunduq });
            }
        }
        // An empty list has nothing to look for and nothing to be blind about.
        assert_eq!(halat_manassa(&[]), HalatTashghil::LaTashtaghil);
    }

    #[test]
    fn asmaa_steam_tughatti_al_anzima_al_thalatha() {
        // The list is shared with `tarkib`, which guards the same file; a name
        // dropped from it disarms both checks on one platform at once.
        assert!(ASMAA_STEAM.contains(&"steam.exe"));
        assert!(ASMAA_STEAM.contains(&"steam"));
        assert!(ASMAA_STEAM.contains(&"steam_osx"));
    }
}
