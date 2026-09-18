//! صفحات الرقعة — how many atlas pages one installed patch's text needs, per
//! rasterization rung, on the budget a game would give it.
//!
//! The runtime atlas opens a second page when the first one fills, and a mesh
//! samples one texture — so a string whose letters ended up on two pages cannot
//! be drawn by one mesh at all. The adapter's answer is to draw the string from
//! the page it is on when there is one, and to leave the string to the engine
//! when there is not; both of those are reported at run time, from inside a
//! game. This is the same question asked offline, over a patch on disk, so an
//! owner can size `lawha/bud` and `lawha/mizaniya` before shipping instead of
//! reading it out of a log afterwards.
//!
//! Run it against an installed patch and the font directory beside it:
//!
//! ```text
//! cargo +1.95.0 run -p taarib-lawha --example safahat_al_ruqaa -- \
//!     "/path/to/game/taarib/nusus.ruqaa" "/path/to/game/taarib/khutut" [page] [MiB]
//! ```
//!
//! Every string in the patch is shaped at each rung, its glyphs are made
//! resident one string per frame — which is what a component rebuild is — and
//! the placements are counted. What comes out per rung is: how many glyph
//! lookups landed on a page other than the first, how many strings are entirely
//! on one page that is not the first, and how many strings straddle two or more.
//! The last two are what the adapter now draws and now refuses; the first is
//! what it used to lose without saying so.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line diagnostic reports by printing, which is the whole of its job"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_lawha::rasf::KhiyaratRasf;
use taarib_lawha::{JamiAshkal, LawhaHayya};
use taarib_ruqaa::{MalafRuqaa, NawQism};
use taarib_saff::rasm::bakat_tahazzuz;
use taarib_saff::{KhiyaratTakhtit, MawridKhatt, Saff, SilsilatKhutut, TalabTakhtit};

/// The ten sizes `Nasij.HajmLawhaMulaim` quantizes an on-screen size to. The C#
/// ladder and this list are the same ten numbers, and a session meets several of
/// them at once — which is what made a second atlas page ordinary rather than
/// hypothetical.
const SALALIM: [f32; 10] = [8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0, 192.0];

/// What one rung cost.
#[derive(Debug, Default, Clone, Copy)]
struct Hasila {
    /// Strings that produced at least one drawable glyph.
    nusus: u64,
    /// Glyph lookups that succeeded.
    talabat: u64,
    /// Lookups whose glyph landed on a page other than the first.
    talabat_baida: u64,
    /// Strings entirely on one page, and that page is not the first.
    nusus_safha_ukhra: u64,
    /// Strings whose glyphs are spread over two or more pages.
    nusus_mushattata: u64,
    /// Lookups the atlas refused outright, because everything in it was pinned.
    marfudat: u64,
    /// How many pages the atlas had opened by the end of the rung.
    safahat: usize,
}

fn main() -> ExitCode {
    let wasait: Vec<String> = std::env::args().skip(1).collect();
    let (Some(masar_ruqaa), Some(masar_khutut)) = (wasait.first(), wasait.get(1)) else {
        eprintln!(
            "usage: safahat_al_ruqaa <patch.ruqaa> <fonts dir> [page size] [budget MiB]\n\
             \n\
             both paths are what an installed patch looks like: the container the installer\n\
             wrote, and the khutut/ directory beside it that carries the fonts it names."
        );
        return ExitCode::FAILURE;
    };
    let bud: u16 = wasait.get(2).and_then(|n| n.parse().ok()).unwrap_or(2048);
    let mizaniya_mib: u64 = wasait.get(3).and_then(|n| n.parse().ok()).unwrap_or(64);

    match qis(
        Path::new(masar_ruqaa),
        Path::new(masar_khutut),
        bud,
        mizaniya_mib,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(sabab) => {
            eprintln!("{sabab}");
            ExitCode::FAILURE
        },
    }
}

/// Opens the patch, builds its chain, and measures every rung.
fn qis(masar: &Path, khutut: &Path, bud: u16, mizaniya_mib: u64) -> Result<(), String> {
    let malaf =
        MalafRuqaa::iftah(masar).map_err(|khata| format!("{}: {khata}", masar.display()))?;
    let ruqaa = malaf
        .ruqaa()
        .map_err(|khata| format!("{}: {khata}", masar.display()))?;

    let qism_nusus = ruqaa
        .qism(NawQism::Nusus)
        .map_err(|khata| format!("reading the string section: {khata}"))?;
    let jadwal = taarib_ruqaa::nusus(qism_nusus.bayt())
        .map_err(|khata| format!("reading the string section: {khata}"))?;
    let nusus: Vec<&str> = jadwal
        .sijillat
        .iter()
        .filter_map(|sijill| jadwal.nass(sijill.nass))
        .filter(|nass| !nass.is_empty())
        .collect();

    let silsila = silsila(&ruqaa, khutut)?;
    let khiyarat = KhiyaratRasf {
        aqsa_ard: bud,
        aqsa_irtifa: bud,
        ..KhiyaratRasf::default()
    };
    let mizaniya = usize::try_from(mizaniya_mib.saturating_mul(1024).saturating_mul(1024))
        .map_err(|_| "the budget does not fit this machine's address space".to_owned())?;

    println!(
        "{}\n  {} string(s), {} font(s), {bud}x{bud} pages, {mizaniya_mib} MiB budget",
        masar.display(),
        nusus.len(),
        silsila.adad()
    );
    println!(
        "\n{:>6}  {:>9}  {:>9}  {:>9}  {:>9}  {:>8}  {:>6}",
        "rung", "strings", "lookups", "off pg 0", "one other", "spanning", "pages"
    );

    for hajm in SALALIM {
        let hasila = rung(&nusus, &silsila, khiyarat, mizaniya, hajm)?;
        println!(
            "{hajm:>6}  {:>9}  {:>9}  {:>9}  {:>9}  {:>8}  {:>6}",
            hasila.nusus,
            hasila.talabat,
            hasila.talabat_baida,
            hasila.nusus_safha_ukhra,
            hasila.nusus_mushattata,
            hasila.safahat
        );
        if hasila.marfudat > 0 {
            println!(
                "        {} lookup(s) at this rung were refused outright: everything resident \
                 was pinned by the frame being drawn.",
                hasila.marfudat
            );
        }
    }

    println!(
        "\n  off pg 0   glyph lookups whose image landed on a page after the first. Built for\n\
         \x20            page zero regardless, every one of these is a letter not drawn.\n  \
         one other  strings entirely on one page that is not page zero. These used to draw\n\
         \x20            nothing at all; they are now drawn from the page they are on.\n  \
         spanning   strings whose glyphs are on two or more pages. One mesh cannot sample\n\
         \x20            two textures, so these are left to the engine and counted."
    );
    Ok(())
}

/// One rung, on an atlas of its own: the placements every string's glyphs take.
///
/// A fresh atlas per rung rather than one shared across all ten, because a game
/// does not draw ten sizes of every string — it draws the sizes one scene puts
/// on screen — and an atlas carrying all ten would report a page pressure no
/// session ever meets.
fn rung(
    nusus: &[&str],
    silsila: &SilsilatKhutut,
    khiyarat: KhiyaratRasf,
    mizaniya: usize,
    hajm: f32,
) -> Result<Hasila, String> {
    let mut hayya = LawhaHayya::jadeeda(khiyarat, NamatSafha::Taghtiya, mizaniya)
        .map_err(|khata| format!("building a runtime atlas: {khata}"))?;
    let takhtit = KhiyaratTakhtit::default();
    let mut saff = Saff::jadeed();
    let mut hasila = Hasila::default();

    for nass in nusus {
        let Ok(mawqia) = saff.khattit(&TalabTakhtit {
            nass,
            khutut: silsila,
            hajm,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: &takhtit,
        }) else {
            // A string the shaper refuses is not a page-pressure measurement,
            // and it is already reported by everything that shapes for real.
            continue;
        };

        let mut jami = JamiAshkal::jadeed(NamatSafha::Taghtiya);
        for harf in &mawqia.huruf {
            jami.idif_shakl(MiftahShakl::jadeed(
                harf.khatt,
                harf.muarrif,
                hajm,
                NamatSafha::Taghtiya,
                bakat_tahazzuz(harf.s),
            ));
        }

        // One frame per string, because one component rebuild is what a string
        // gets: the pins this string takes are released before the next one.
        hayya.ibda_itar();
        let mut safahat: BTreeSet<u16> = BTreeSet::new();
        for miftah in jami.ashkal() {
            match hayya.shakl_min_silsila(miftah, silsila) {
                Ok(mawdi) => {
                    if mawdi.ard == 0 || mawdi.irtifa == 0 {
                        // A space holds no rectangle and belongs to no page.
                        continue;
                    }
                    hasila.talabat += 1;
                    if mawdi.safha > 0 {
                        hasila.talabat_baida += 1;
                    }
                    let _ = safahat.insert(mawdi.safha);
                },
                Err(_) => hasila.marfudat += 1,
            }
        }

        if safahat.is_empty() {
            continue;
        }
        hasila.nusus += 1;
        if safahat.len() > 1 {
            hasila.nusus_mushattata += 1;
        } else if safahat.first().is_some_and(|safha| *safha > 0) {
            hasila.nusus_safha_ukhra += 1;
        }
    }

    hasila.safahat = hayya.safahat().len();
    Ok(hasila)
}

/// The font chain the patch declares, loaded from the directory beside it.
fn silsila(ruqaa: &taarib_ruqaa::Ruqaa<'_>, dalil: &Path) -> Result<SilsilatKhutut, String> {
    let qism = ruqaa
        .qism(NawQism::Khatt)
        .map_err(|khata| format!("reading the font section: {khata}"))?;
    let jadwal = taarib_ruqaa::khatt(qism.bayt())
        .map_err(|khata| format!("reading the font section: {khata}"))?;

    let mut khutut: Vec<Arc<MawridKhatt>> = Vec::with_capacity(jadwal.adad());
    for sijill in jadwal.sijillat {
        let ism = jadwal
            .ism(*sijill)
            .ok_or_else(|| "a font record names no file".to_owned())?;
        let masar: PathBuf = dalil.join(ism);
        let bayt =
            std::fs::read(&masar).map_err(|sabab| format!("{}: {sabab}", masar.display()))?;
        // Face zero, always: the argument is the face index inside the file, and
        // only a `.ttc` collection has more than one.
        let mawrid = MawridKhatt::jadeed(Arc::new(bayt), 0)
            .map_err(|khata| format!("{}: {khata}", masar.display()))?;
        khutut.push(Arc::new(mawrid));
    }

    SilsilatKhutut::jadeeda(khutut).map_err(|khata| format!("building the font chain: {khata}"))
}
