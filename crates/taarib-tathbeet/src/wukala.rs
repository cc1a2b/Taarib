//! وكلاء التحميل — the loader slots beside a game's executable, and who holds them.
//!
//! A Windows game loads a module by name, and the loader looks in the
//! executable's own directory before it looks in `System32`. That is the whole
//! mechanism behind every DLL-proxy mod: put a file called `dinput8.dll` beside
//! the game, export what the real one exports, forward each call to the system
//! copy, and do your own work on the way past. `taarib-mudkhal` is exactly such
//! a proxy, published as `version.dll`.
//!
//! Which means the slot is a resource, and two mods cannot share one. A second
//! `dinput8.dll` written over the first does not chain — it *replaces* it, the
//! first mod is silently gone, and its own files are left behind pointing at a
//! loader that no longer exists. So before Taarib places its loader it has to
//! know what is already beside the game.
//!
//! ## What counts as a slot, and what does not
//!
//! [`WUKALA_NIZAM`] is the list this module answers from, and its membership
//! rule is narrow on purpose: a module that **ships with Windows and is not in
//! any redistributable**. Nothing in a game's own installation puts a
//! `winmm.dll` or a `dsound.dll` next to the executable, because there is
//! nowhere legitimate for one to come from — so a file with that name beside a
//! game is a proxy, with no guessing involved.
//!
//! The names deliberately left out are the ones a game *does* ship legitimately
//! and that a naive "is this a system-sounding DLL" test would flag: the
//! DirectX end-user redistributables (`d3dx9_43.dll`, `x3daudio1_7.dll`,
//! `xinput1_3.dll`, `xaudio2_*.dll`, `d3dcompiler_*.dll`), the Visual C++
//! runtimes (`msvcp*.dll`, `vcruntime*.dll`), and middleware
//! (`steam_api.dll`, `binkw32.dll`, `openal32.dll`). Every one of those is
//! normal beside a game and none is reported here. A survey that cried proxy
//! over `xinput1_3.dll` would be a survey nobody reads.
//!
//! ## What this module does not claim
//!
//! It does not identify *which* mod holds a slot, and it does not read the
//! occupying file. A name and a size and whatever configuration files sit
//! beside it under the same stem is the whole of the evidence, and that is
//! deliberately as far as it goes: naming the mod would mean a signature list
//! that is wrong the moment somebody renames a build, and the decisions that
//! rest on this — refuse when Taarib's own slot is taken, tell the user what
//! else is in their game before they agree to an install — need the slot and
//! not the identity.

use std::path::Path;

/// Windows modules that a game's own installation has no legitimate way to
/// place beside its executable.
///
/// Base names, lower-case, without the `.dll`. Every one of these ships as part
/// of Windows and appears in no redistributable a game may carry, so a file
/// with one of these names in a game directory was put there by a mod. The list
/// is the union of the slots the mod-loading ecosystem actually uses — the
/// graphics ones for `ReShade` and its relatives, the input and audio ones for
/// the ASI loaders, `winhttp` for BepInEx, `version` for Taarib's own loader.
///
/// `opengl32` carries one honest caveat: a handful of 1990s titles shipped a
/// vendor `MiniGL` driver under that name. Such a file would be reported here as
/// a slot in use, which is true — the loader really would prefer it — even
/// though nobody modded that game.
pub const WUKALA_NIZAM: [&str; 21] = [
    "d3d8",
    "d3d9",
    "d3d10",
    "d3d11",
    "d3d12",
    "dbghelp",
    "dciman32",
    "ddraw",
    "dinput",
    "dinput8",
    "dsound",
    "dxgi",
    "iphlpapi",
    "msacm32",
    "opengl32",
    "version",
    "winhttp",
    "wininet",
    "winmm",
    "ws2_32",
    "wsock32",
];

/// The extension a proxy's own configuration or log carries, when it has one.
///
/// A proxy that reads settings names them after itself — `dinput8.ini` beside
/// `dinput8.dll` — because the file it is impersonating decides its own name.
/// Finding one is what turns "a module with a system name is here" into "a mod
/// with settings is here", and it is the detail that makes the line the user
/// reads before an install specific enough to act on.
const LAWAHIQ_RAFIQ: [&str; 5] = ["ini", "log", "cfg", "toml", "json"];

/// One loader slot beside a game's executable, and the file holding it.
///
/// Serialisable because this is the one thing in the plan the user has to be
/// shown *before* they agree to an install, which means it crosses into the
/// Studio's command layer; a hand-written projection of four plain fields would
/// be a second place for the same four fields to drift.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct WakeelQaim {
    /// The file's name exactly as it is on disk, which is not necessarily the
    /// spelling [`WUKALA_NIZAM`] uses: Windows filesystems are case-insensitive
    /// and a mod's installer writes whatever case it likes.
    pub ism: String,

    /// The slot, folded to the spelling in [`WUKALA_NIZAM`].
    pub wakeel: String,

    /// The file's size in bytes.
    pub hajm: u64,

    /// Files beside it sharing its stem — `dinput8.ini`, `dinput8.log` — sorted
    /// by name.
    pub rifaq: Vec<String>,
}

impl WakeelQaim {
    /// The entry as one line of English, for the install report and the
    /// confirmation screen.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let hajm = self.hajm;
        if self.rifaq.is_empty() {
            format!("{} ({hajm} byte(s))", self.ism)
        } else {
            format!("{} ({hajm} byte(s), with {})", self.ism, self.rifaq.join(", "))
        }
    }

    /// The same line in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let hajm = self.hajm;
        if self.rifaq.is_empty() {
            format!("{} ({hajm} بايت)", self.ism)
        } else {
            format!("{} ({hajm} بايت، ومعه {})", self.ism, self.rifaq.join("، "))
        }
    }
}

/// Whether a file name occupies one of the slots in [`WUKALA_NIZAM`].
///
/// Case-folded on both halves, because the name on disk is whatever a mod's
/// installer wrote and Windows treats `DInput8.DLL` and `dinput8.dll` as one
/// file.
#[must_use]
pub fn wakeel_nizam(ism: &str) -> Option<&'static str> {
    let (asas, imtidad) = ism.rsplit_once('.')?;
    if !imtidad.eq_ignore_ascii_case("dll") {
        return None;
    }
    WUKALA_NIZAM.into_iter().find(|maaruf| asas.eq_ignore_ascii_case(maaruf))
}

/// Every loader slot in use in one directory, sorted by slot name.
///
/// A directory that does not exist holds no slots and is reported as such
/// rather than as a failure: this runs while a plan is being built, and a
/// missing directory is answered by the step that needed it to exist, not by
/// the survey that walked past it.
///
/// # Errors
///
/// Whatever reading the directory raises, apart from
/// [`std::io::ErrorKind::NotFound`]. An entry whose metadata cannot be read is
/// skipped rather than failing the survey — it is a file the installer will not
/// be writing over, and refusing to plan an install because one unrelated file
/// in the game directory could not be stat'ed would be a refusal with no
/// subject.
pub fn masah(mujallad: &Path) -> std::io::Result<Vec<WakeelQaim>> {
    let qira = match std::fs::read_dir(mujallad) {
        Ok(qira) => qira,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(sabab) => return Err(sabab),
    };

    let mut asmaa: Vec<String> = Vec::new();
    for madkhal in qira {
        let madkhal = madkhal?;
        // A directory called `version.dll` is not a module the loader can map,
        // and `file_type` answers without a second syscall on every platform
        // this builds for.
        if !madkhal.file_type().is_ok_and(|naw| naw.is_file()) {
            continue;
        }
        if let Some(ism) = madkhal.file_name().to_str() {
            asmaa.push(ism.to_owned());
        }
    }

    let mut qaima: Vec<WakeelQaim> = Vec::new();
    for ism in &asmaa {
        let Some(wakeel) = wakeel_nizam(ism) else { continue };
        let hajm =
            std::fs::metadata(mujallad.join(ism)).map_or(0, |bayan| bayan.len());
        qaima.push(WakeelQaim {
            ism: ism.clone(),
            wakeel: wakeel.to_owned(),
            hajm,
            rifaq: rifaq_bi_asas(&asmaa, wakeel),
        });
    }
    qaima.sort();
    Ok(qaima)
}

/// The configuration and log files a proxy named after its slot, sorted.
fn rifaq_bi_asas(asmaa: &[String], wakeel: &str) -> Vec<String> {
    let mut rifaq: Vec<String> = asmaa
        .iter()
        .filter(|ism| {
            ism.rsplit_once('.').is_some_and(|(asas, imtidad)| {
                asas.eq_ignore_ascii_case(wakeel)
                    && LAWAHIQ_RAFIQ.iter().any(|maqbul| imtidad.eq_ignore_ascii_case(maqbul))
            })
        })
        .cloned()
        .collect();
    rifaq.sort();
    rifaq
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              honouring it here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use std::fs;

    use super::{masah, wakeel_nizam};

    /// The layout of `Resident Evil 4`'s `Bin32/` on the machine this was
    /// written against: `re4_tweaks` holding two slots, the game's own files
    /// beside it, and Taarib's slot free.
    ///
    /// Names and sizes only — none of these is the real file, and nothing here
    /// reads one.
    fn bin32(jidhr: &std::path::Path) {
        for (ism, hajm) in [
            ("bio4.exe", 9_139_840_usize),
            ("dinput8.dll", 11_760_128),
            ("dinput8.ini", 22_044),
            ("dinput8.log", 42_726),
            ("steam_api.dll", 106_408),
            ("steam_appid.txt", 6),
            ("winmm.dll", 80_384),
        ] {
            fs::write(jidhr.join(ism), vec![0_u8; hajm.min(64)]).expect("writing a fixture");
        }
        fs::create_dir(jidhr.join("re4_tweaks")).expect("the mod's own directory");
        fs::write(jidhr.join("re4_tweaks/trainer.ini"), b"x").expect("a file inside it");
    }

    #[test]
    fn al_wakeel_yuaraf_bila_hala() {
        assert_eq!(wakeel_nizam("dinput8.dll"), Some("dinput8"));
        assert_eq!(wakeel_nizam("DInput8.DLL"), Some("dinput8"));
        assert_eq!(wakeel_nizam("version.dll"), Some("version"));
        // The redistributables a game ships legitimately. Reporting any of
        // these would make the survey noise.
        for barii in [
            "steam_api.dll",
            "d3dx9_43.dll",
            "xinput1_3.dll",
            "x3daudio1_7.dll",
            "msvcp140.dll",
            "vcruntime140.dll",
            "binkw32.dll",
        ] {
            assert_eq!(wakeel_nizam(barii), None, "{barii} is a game's own file, not a slot");
        }
        // Not a module at all.
        assert_eq!(wakeel_nizam("dinput8.ini"), None);
        assert_eq!(wakeel_nizam("dinput8"), None);
    }

    #[test]
    fn masah_yajid_huqn_re4_tweaks_wala_yalmis_malaffat_alluba() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        bin32(muaqqat.path());

        let qaima = masah(muaqqat.path()).expect("the survey reads a directory that exists");
        let asmaa: Vec<&str> = qaima.iter().map(|wakeel| wakeel.ism.as_str()).collect();
        assert_eq!(
            asmaa,
            vec!["dinput8.dll", "winmm.dll"],
            "re4_tweaks holds two slots; the game's own files hold none"
        );

        let dinput = qaima.first().expect("the first slot");
        assert_eq!(dinput.wakeel, "dinput8");
        assert_eq!(
            dinput.rifaq,
            vec!["dinput8.ini".to_owned(), "dinput8.log".to_owned()],
            "the proxy's settings and its log are named after the slot it stands in"
        );
        // The second proxy carries no settings of its own, and saying so is the
        // difference between a survey and a guess.
        let winmm = qaima.get(1).expect("the second slot");
        assert!(winmm.rifaq.is_empty());

        // Taarib's own slot is free in this game, which is the finding the
        // install decision rests on.
        assert!(
            !qaima.iter().any(|wakeel| wakeel.wakeel == "version"),
            "nothing in this directory holds the slot taarib-mudkhal is published as"
        );
    }

    #[test]
    fn wakeel_taarib_yuzhar_hin_yushghal() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        bin32(muaqqat.path());
        fs::write(muaqqat.path().join("version.dll"), b"another mod's loader")
            .expect("a foreign proxy in Taarib's slot");

        let qaima = masah(muaqqat.path()).expect("the survey");
        let taarib = qaima
            .iter()
            .find(|wakeel| wakeel.wakeel == "version")
            .expect("the occupied slot is reported");
        assert_eq!(taarib.hajm, 20);
        assert!(taarib.wasf_injilizi().contains("version.dll"));
        assert!(taarib.wasf_arabi().contains("بايت"));
    }

    #[test]
    fn mujallad_ghayr_mawjud_la_yahmil_shayan() {
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        let ghayib = muaqqat.path().join("la-yujad");
        assert_eq!(
            masah(&ghayib).expect("a directory that is not there holds no slots"),
            Vec::new(),
            "a missing directory is answered by whoever needed it, not by the survey"
        );
    }

    #[test]
    fn mujallad_bi_ism_wakeel_laysa_wahda() {
        // A *directory* called `dxgi.dll` is not something the loader can map,
        // and reporting it as an occupied slot would refuse an install over a
        // file that cannot hold one.
        let muaqqat = tempfile::tempdir().expect("a scratch directory");
        fs::create_dir(muaqqat.path().join("dxgi.dll")).expect("a directory with a module's name");
        assert_eq!(masah(muaqqat.path()).expect("the survey"), Vec::new());
    }
}
