//! The artifact matrix of docs/tawzee.md §3, executed row by row.

use std::path::PathBuf;

use taarib_usus::manassa::{Mimariya, NizamTashghil};

use crate::hadaf::{Hadaf, hamulat_alalaab, muthallath_hamula};
use crate::khata::NatijatTajmee;
use crate::nasakh::Mustaqarr;
use crate::qufl::Qufl;

/// The Unity generations a component directory is built for.
const AJYAL: [&str; 3] = ["qadeem", "wasat", "hadith"];

/// The two Unity scripting backends.
const KHALFIYAT: [&str; 2] = ["mono", "il2cpp"];

/// Where the whole staged tree of one component's payload lives inside it.
///
/// `TawziMukawwin::Wahid` deploys a bepinex component verbatim beside the
/// game's executable, so the Taarib payload has to sit at the path BepInEx
/// itself will look in once it is there.
const DAKHIL_BEPINEX: &str = "BepInEx/plugins/Taarib";

/// Where inside the Ren'Py component the Arabic face it ships lives.
///
/// Mirrors `taarib_tathbeet::tarkib::MUJALLAD_KHATT_RENPY`, which is private.
/// `tarkib::renpy` copies this component's whole tree into the game's `game/`
/// directory, so a face staged here arrives at `game/taarib/khutut/` with no
/// deployment step of its own, and `tarkib::khatt_renpy` reads it back from
/// exactly that path. The two spellings must not drift, or the bundle carries
/// a font no install ever registers.
const DAKHIL_KHATT_RENPY: &str = "taarib/khutut";

/// The face the Ren'Py component ships and the licence that travels with it,
/// as the lock entry that already fetches them and the name each lands under.
///
/// Noto Naskh Arabic because `tarkib::TARTIB_KHATT_RENPY` ranks it first: a
/// visual novel's text is body copy read in paragraphs, and the Arabic
/// tradition for body copy is Naskh. That list ranks rather than filters, so
/// this is the default the bundle chooses and not a requirement it satisfies —
/// a component staged with some other family would still register that one.
///
/// The licence is not optional. These bytes leave the machine a second time,
/// inside somebody's game rather than beside the Studio, and the OFL binds
/// every redistribution rather than the first.
const KHATT_RENPY: [(&str, &str); 2] = [
    (
        "NotoNaskhArabic/NotoNaskhArabic[wght].ttf",
        "NotoNaskhArabic[wght].ttf",
    ),
    ("NotoNaskhArabic/rukhsa", "OFL.txt"),
];

/// Where inside the Unreal component the face it ships lives.
///
/// Mirrors `taarib_tathbeet::tarkib::MUJALLAD_KHATT_UNREAL`, which is private.
/// `tarkib::khatt_unreal` reads the face back from exactly this path and the
/// deployment carries it inside the additive container it writes beside the
/// game's own; the two spellings must not drift, or the container names no
/// face and Arabic draws from the engine's last-resort font as boxes.
const DAKHIL_KHATT_UNREAL: &str = "khutut";

/// The face the Unreal component ships and the licence that travels with it.
///
/// IBM Plex Sans Arabic because `tarkib::TARTIB_KHATT_UNREAL` ranks it first:
/// a game's menus and prompts are interface text drawn at interface sizes, and
/// a sans reads at those sizes where a book Naskh does not. A static face, not
/// a variable one, because the engine that reads a localized fallback-font
/// name is Unreal 4.13 to about 4.19, whose `FreeType` predates variation
/// tables; a variable face would load its default instance on some of those
/// and nothing on others.
///
/// The licence is not optional, for the reason [`KHATT_RENPY`] gives.
const KHATT_UNREAL: [(&str, &str); 2] = [
    (
        "IBMPlexSansArabic/IBMPlexSansArabic-Regular.ttf",
        "IBMPlexSansArabic-Regular.ttf",
    ),
    ("IBMPlexSansArabic/rukhsa", "IBMPlex-OFL.txt"),
];

/// The staging inputs that are not built by cargo.
#[derive(Debug, Clone)]
pub(crate) struct MasadirTajmee {
    /// The workspace root.
    pub(crate) jidhr: PathBuf,
    /// Cargo's target directory.
    pub(crate) ahdaf: PathBuf,
    /// Whether network fetches are permitted this run.
    pub(crate) jalb: bool,
}

impl MasadirTajmee {
    /// A built artifact's path for one target triple.
    fn bina(&self, muthallath: &str, ism: &str) -> PathBuf {
        self.ahdaf.join(muthallath).join("release").join(ism)
    }
}

/// Stages every row of the matrix for one target.
///
/// Refusals are collected rather than raised: the caller prints all of them.
///
/// # Errors
///
/// [`KhataTajmee::KhataMalaf`] when the staging tree itself cannot be written.
pub(crate) fn jammi(
    hadaf: Hadaf,
    masadir: &MasadirTajmee,
    mustaqarr: &mut Mustaqarr,
) -> NatijatTajmee<()> {
    saf_bepinex(hadaf, masadir, mustaqarr)?;
    saf_mudkhal(hadaf, masadir, mustaqarr)?;
    // One lock read once, because row J1's faces have two destinations: the
    // bundle's own font directory, which the Studio draws from, and the Ren'Py
    // component, which carries one face a second time so that a visual novel
    // has a face to be told about at all.
    let khutut = Qufl::iqra(&masadir.jidhr.join("assets/aqfal/qufl_khutut.json"))?;
    saf_mulhaq(masadir, &khutut, mustaqarr)?;
    saf_khutut(masadir, &khutut, mustaqarr)?;
    // Row N1: the redistribution obligations of everything above, in one file
    // the bundle carries and the diagnostics screen can open.
    mustaqarr.insakh("N1", &masadir.jidhr.join("assets/NOTICES.md"), "NOTICES.md")?;
    Ok(())
}

/// Rows B, C, D, K: every BepInEx component, with the Taarib payload inside it.
fn saf_bepinex(
    hadaf: Hadaf,
    masadir: &MasadirTajmee,
    mustaqarr: &mut Mustaqarr,
) -> NatijatTajmee<()> {
    let qufl = Qufl::iqra(&masadir.jidhr.join("assets/aqfal/qufl_bepinex.json"))?;

    for (nizam, mimariya) in hamulat_alalaab(hadaf) {
        for khalfiya in KHALFIYAT {
            for jeel in AJYAL {
                let mukawwin = format!(
                    "mukawwinat/bepinex/{}/{khalfiya}-{jeel}-{}",
                    ism_hadaf(nizam),
                    mimariya.mujallad()
                );

                // The redistributable itself, unpacked verbatim.
                let muarrif = muarrif_bepinex(khalfiya, nizam, mimariya);
                qufl.ifragh(
                    "D1",
                    &muarrif,
                    &mukawwin,
                    &masadir.jidhr,
                    masadir.jalb,
                    mustaqarr,
                )?;

                // The Taarib payload, at the path BepInEx resolves once the
                // component is deployed beside the game's executable.
                let dakhil = format!("{mukawwin}/{DAKHIL_BEPINEX}");
                for (ism, saf, itar) in tajmiaat(khalfiya) {
                    let masdar = masadir
                        .jidhr
                        .join("unity")
                        .join(ism)
                        .join("bin/Release")
                        .join(itar)
                        .join(format!("{ism}.dll"));
                    mustaqarr.insakh(saf, &masdar, &format!("{dakhil}/{ism}.dll"))?;
                }

                // The C-ABI core, where Muhammil.cs derives it from the
                // assembly's own directory.
                let ism_jisr = nizam.ism_maktaba("taarib_jisr");
                let masdar = masadir.bina(muthallath_hamula(nizam, mimariya), &ism_jisr);
                mustaqarr.insakh(
                    saf_jisr(nizam),
                    &masdar,
                    &format!("{dakhil}/jisr/{}/{ism_jisr}", mimariya.mujallad()),
                )?;

                // The IL2CPP signature database, beside the assembly that
                // reads it.
                if khalfiya == "il2cpp" {
                    let masdar = masadir.jidhr.join("assets/basmat/basmat.json");
                    mustaqarr.insakh("K1", &masdar, &format!("{dakhil}/basmat.json"))?;
                }
            }
        }
    }
    Ok(())
}

/// Rows E, F, G: Taarib's own loader and the payloads it opens.
fn saf_mudkhal(
    hadaf: Hadaf,
    masadir: &MasadirTajmee,
    mustaqarr: &mut Mustaqarr,
) -> NatijatTajmee<()> {
    for (nizam, mimariya) in hamulat_alalaab(hadaf) {
        let mukawwin = format!(
            "mukawwinat/mudkhal/{}/{}",
            ism_hadaf(nizam),
            mimariya.mujallad()
        );
        let muthallath = muthallath_hamula(nizam, mimariya);

        // The loader, under muhammil/ because a split component deploys only
        // that subdirectory beside the game's executable. Renamed from the
        // crate's own output to the name the game's loader will look for.
        let mabni = nizam.ism_maktaba("taarib_mudkhal");
        let masmi = match nizam {
            NizamTashghil::Windows => "version.dll".to_owned(),
            NizamTashghil::Linux | NizamTashghil::Mac => nizam.ism_maktaba("taarib_muhammil"),
        };
        mustaqarr.insakh(
            saf_mudkhal_lil(nizam),
            &masadir.bina(muthallath, &mabni),
            &format!("{mukawwin}/muhammil/{masmi}"),
        )?;

        // The payloads, at the component root: everything that is not the
        // loader lands in the game's own Taarib directory.
        for (saf, asas) in [
            ("F1", "taarib_tabaqa"),
            ("G1", "taarib_muhawwil_unreal"),
            ("G2", "taarib_muhawwil_godot"),
        ] {
            let ism = nizam.ism_maktaba(asas);
            // Each of these must carry its bootstrap, or the loader opens it
            // and the game runs untranslated with nothing reporting anything.
            mustaqarr.insakh_bi_shart(
                saf,
                &masadir.bina(muthallath, &ism),
                &format!("{mukawwin}/{ism}"),
                true,
            )?;
        }
    }
    Ok(())
}

/// Rows H, I2, I3: the script-engine adapters and the wasm core they carry.
fn saf_mulhaq(
    masadir: &MasadirTajmee,
    khutut: &Qufl,
    mustaqarr: &mut Mustaqarr,
) -> NatijatTajmee<()> {
    // The wasm pair, built by wasm-bindgen with --out-name taarib_core; the
    // adapter loads exactly these two names and cannot be parameterised.
    let wasm = masadir.jidhr.join("target/wasm-bindgen/taarib_core.js");
    let wasm_bg = masadir
        .jidhr
        .join("target/wasm-bindgen/taarib_core_bg.wasm");

    // RPG Maker needs one component per generation: rpg_maker() resolves
    // mulhaq/rpgmaker/mv and mulhaq/rpgmaker/mz separately.
    let mabni = masadir.jidhr.join("target/adapters/rpgmaker/taarib.js");
    for jeel in ["mv", "mz"] {
        let mukawwin = format!("mukawwinat/mulhaq/rpgmaker/{jeel}");
        mustaqarr.insakh("I2", &mabni, &format!("{mukawwin}/taarib.js"))?;
        mustaqarr.insakh("H1", &wasm, &format!("{mukawwin}/taarib/taarib_core.js"))?;
        mustaqarr.insakh(
            "H1",
            &wasm_bg,
            &format!("{mukawwin}/taarib/taarib_core_bg.wasm"),
        )?;
    }

    // Electron: the renderer runtime and the same wasm pair. The crate that
    // embeds it takes it as a caller-supplied string, so the component store
    // is where that caller reads it from.
    let mabni_electron = masadir.jidhr.join("target/adapters/electron/taarib.js");
    mustaqarr.insakh(
        "I1",
        &mabni_electron,
        "mukawwinat/mulhaq/electron/taarib.js",
    )?;
    mustaqarr.insakh(
        "H1",
        &wasm,
        "mukawwinat/mulhaq/electron/taarib/taarib_core.js",
    )?;
    mustaqarr.insakh(
        "H1",
        &wasm_bg,
        "mukawwinat/mulhaq/electron/taarib/taarib_core_bg.wasm",
    )?;

    // Ren'Py takes the package directory intact: __init__.py uses relative
    // imports, so a flattened copy raises ImportError at game start.
    let masdar = masadir.jidhr.join("adapters-script/renpy/taarib_renpy");
    mustaqarr.insakh_mujallad("I3", &masdar, "mukawwinat/mulhaq/renpy/taarib_renpy")?;

    // The face that component registers. Ren'Py resolves a font path against
    // `game/`, and this component's whole tree is copied there, so the face has
    // to travel inside the component rather than beside it — there is no other
    // step that would put it in the game.
    //
    // Without it `tarkib::khatt_renpy` answers `None` and the install registers
    // no font, which leaves the game drawing Arabic in whatever face it already
    // ships: correct on the few visual novels whose GUI font covers Arabic,
    // empty boxes on the rest.
    //
    // Row J1, because these are J1's bytes: the same lock entry, already
    // fetched and hash-verified for the bundle's own font directory, staged to
    // a second destination rather than downloaded again.
    for (muarrif, ism) in KHATT_RENPY {
        khutut.ifragh(
            "J1",
            muarrif,
            &format!("mukawwinat/mulhaq/renpy/{DAKHIL_KHATT_RENPY}/{ism}"),
            &masadir.jidhr,
            masadir.jalb,
            mustaqarr,
        )?;
    }

    // The face the Unreal additive container carries. `tarkib::unreal` reads
    // it out of this component and the deployment writes it inside the
    // container beside the game's own, under the directory the engine reads
    // its fallback face from; there is no other step that would put a face
    // where that engine looks. Row J1 again, for the same reason as above.
    for (muarrif, ism) in KHATT_UNREAL {
        khutut.ifragh(
            "J1",
            muarrif,
            &format!("mukawwinat/mulhaq/unreal/{DAKHIL_KHATT_UNREAL}/{ism}"),
            &masadir.jidhr,
            masadir.jalb,
            mustaqarr,
        )?;
    }

    Ok(())
}

/// Row J1: the bundled fonts and every license text that travels with them.
fn saf_khutut(
    masadir: &MasadirTajmee,
    khutut: &Qufl,
    mustaqarr: &mut Mustaqarr,
) -> NatijatTajmee<()> {
    khutut.ifragh_kul("J1", "khutut", &masadir.jidhr, masadir.jalb, mustaqarr)
}

/// The three Unity assemblies one backend ships, each with its matrix row and
/// the target-framework directory `dotnet build` writes it into.
///
/// The framework is per project rather than one constant. Three assemblies
/// target `netstandard2.1`; `Taarib.Unity.Il2cpp` targets `net6.0`, because
/// BepInEx's IL2CPP host loads a `CoreCLR` runtime it ships itself rather than the
/// game's Mono, and that is stated in the project file. Reading all four out of
/// `bin/Release/netstandard2.1` finds three and records the IL2CPP assembly
/// absent, which withholds the manifest from a build that in fact succeeded.
fn tajmiaat(khalfiya: &str) -> [(&'static str, &'static str, &'static str); 3] {
    let khass = if khalfiya == "il2cpp" {
        ("Taarib.Unity.Il2cpp", "C3", "net6.0")
    } else {
        ("Taarib.Unity.Mono", "C2", "netstandard2.1")
    };
    [
        ("Taarib.Unity.Jisr", "C1", "netstandard2.1"),
        ("Taarib.Unity.Mushtarak", "C4", "netstandard2.1"),
        khass,
    ]
}

/// The lock entry a BepInEx component takes its redistributable from.
///
/// macOS collapses to one identifier: upstream publishes a universal build for
/// the Mono line and an x64-only build for the IL2CPP line, and neither has a
/// separate arm64 asset to pin.
fn muarrif_bepinex(khalfiya: &str, nizam: NizamTashghil, mimariya: Mimariya) -> String {
    let arch = match nizam {
        NizamTashghil::Mac => "x64",
        NizamTashghil::Windows | NizamTashghil::Linux => mimariya.mujallad(),
    };
    format!("bepinex-{khalfiya}-{}-{arch}", ism_hadaf(nizam))
}

/// The matrix row a C-ABI core belongs to on one platform.
const fn saf_jisr(nizam: NizamTashghil) -> &'static str {
    match nizam {
        NizamTashghil::Windows => "B1",
        NizamTashghil::Linux => "B2",
        NizamTashghil::Mac => "B3",
    }
}

/// The matrix row a loader belongs to on one platform.
const fn saf_mudkhal_lil(nizam: NizamTashghil) -> &'static str {
    match nizam {
        NizamTashghil::Windows => "E1",
        NizamTashghil::Linux => "E2",
        NizamTashghil::Mac => "E3",
    }
}

/// The component store's directory name for a payload target.
///
/// Mirrors `taarib_tathbeet::tarkib::ism_hadaf`, which is private; the two must
/// stay identical or the installer looks where nothing was staged.
const fn ism_hadaf(nizam: NizamTashghil) -> &'static str {
    match nizam {
        NizamTashghil::Windows => "windows",
        NizamTashghil::Linux => "linux",
        NizamTashghil::Mac => "mac",
    }
}
