//! القص والجسر — a compiled atlas is the size of its pack, and it can be handed
//! to the glyph transport.
//!
//! Two properties, both of which the code asserts about itself in prose and
//! neither of which anything checked until now.
//!
//! **The crop.** [`taarib_lawha::Rasif`] opens every page at the ceiling the
//! options allow — it has to, because the shelf allocator must open page zero
//! before it has seen a rectangle, and opening smaller would overflow later
//! glyphs into a second page. What ships is then cropped to the rows the pack
//! actually reached. The difference is not cosmetic: at the default 4096×4096 a
//! small glyph set is a hundred kilobytes of coverage inside sixteen megabytes
//! of zeros, and one published artefact was cropped by hand after the fact.
//! Nothing failed if somebody deleted the crop, so these tests fail instead.
//!
//! **The bridge.** [`taarib_lawha::naql::LawhaJahiza`] is what the last rung of
//! the Godot 3 and GameMaker ladders reads an atlas through, and until now
//! nothing turned a compiled [`taarib_lawha::Lawha`] into one.
//!
//! # The font
//!
//! IBM Plex Sans Arabic Regular, staged into `apps/studio/src/khutut/`. Nothing
//! here downloads anything, and no page is fabricated — every texel these tests
//! measure was rasterized by the same code a compile uses.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_lawha::misafa::KhiyaratMisafa;
use taarib_lawha::naql::{LawhaJahiza, MasdarLawha, MiftahKhana};
use taarib_lawha::{KhiyaratRasf, Lawha};
use taarib_saff::{MawridKhatt, SilsilatKhutut};

/// The size the glyphs are rasterized at, in pixels.
const HAJM: f32 = 24.0;

/// How many glyphs the fixture packs. Small on purpose: the whole point is that
/// a handful of glyphs must not ship a full-size page.
const ADAD: u32 = 24;

/// The staged Arabic face, found by walking up from this crate.
fn khutut() -> SilsilatKhutut {
    const MURASHAH: &str = "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf";

    let mut dalil: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    let mut masar: Option<PathBuf> = None;
    while let Some(jidhr) = dalil {
        let murashah = jidhr.join(MURASHAH);
        if murashah.is_file() {
            masar = Some(murashah);
            break;
        }
        dalil = jidhr.parent();
    }
    let Some(masar) = masar else {
        panic!("no Arabic font found at or above {}", env!("CARGO_MANIFEST_DIR"));
    };
    let bayt = fs::read(&masar).unwrap_or_else(|_| panic!("{} unreadable", masar.display()));
    let khatt = MawridKhatt::jadeed(Arc::new(bayt), 0)
        .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
    SilsilatKhutut::wahid(Arc::new(khatt)).expect("a one-font chain")
}

/// A small glyph set at one size, packed at the given ceiling.
fn lawha(khiyarat: KhiyaratRasf) -> Lawha {
    let ashkal: Vec<MiftahShakl> = (1..=ADAD)
        .map(|muarrif| MiftahShakl::jadeed(0, muarrif, HAJM, NamatSafha::Taghtiya, 0))
        .collect();
    Lawha::ibni(
        &ashkal,
        &khutut(),
        khiyarat,
        KhiyaratMisafa::default(),
        NamatSafha::Taghtiya,
    )
    .expect("a two-dozen glyph atlas builds")
}

/// A finished page is the height its pack reached, not the height it was opened
/// at.
#[test]
fn ikhtibar_al_safha_maqsusa_ila_imtidad_al_rasf() {
    let khiyarat = KhiyaratRasf::default();
    let mabniya = lawha(khiyarat);

    let Some(safha) = mabniya.safahat.first() else { panic!("the atlas has a page") };
    assert!(
        safha.irtifa < khiyarat.aqsa_irtifa,
        "two dozen glyphs at {HAJM}px filled {} of the {} rows the page was opened at",
        safha.irtifa,
        khiyarat.aqsa_irtifa
    );
    // The margin the hand-crop of a published artefact measured was 42.7x. Ten
    // is a floor generous enough that a change of packing strategy does not
    // fail this, and tight enough that a deleted crop cannot pass it.
    assert!(
        u32::from(khiyarat.aqsa_irtifa) > u32::from(safha.irtifa).saturating_mul(10),
        "the crop saved less than a factor of ten, which is not a crop"
    );
}

/// The buffer is exactly the cropped rectangle, with no row padding.
///
/// The container writer refuses a page whose byte count is not `ard × irtifa`,
/// so a crop that truncated the dimensions without truncating the buffer — or
/// the reverse — would turn every compile into a refusal at the last step.
#[test]
fn ikhtibar_bayt_al_safha_tusawi_al_mustatil_al_maqsus() {
    let mabniya = lawha(KhiyaratRasf::default());
    for (fahras, safha) in mabniya.safahat.iter().enumerate() {
        assert_eq!(
            safha.bayt.len(),
            usize::from(safha.ard) * usize::from(safha.irtifa),
            "page {fahras} is {}x{} and holds {} byte(s)",
            safha.ard,
            safha.irtifa,
            safha.bayt.len()
        );
    }
}

/// The recorded page size follows the crop, because texture coordinates are
/// derived from it.
///
/// Registering the allocation size instead would divide every coordinate by a
/// height the page no longer has, and every glyph would sample the wrong rows.
#[test]
fn ikhtibar_al_abaad_al_musajjala_tatbaa_al_qass() {
    let mabniya = lawha(KhiyaratRasf::default());
    let Some(safha) = mabniya.safahat.first() else { panic!("the atlas has a page") };
    assert_eq!(mabniya.khareeta.abaad_safha(0), Some((safha.ard, safha.irtifa)));

    let mafatih = mabniya.khareeta.mafatih();
    let Some(miftah) = mafatih.first() else { panic!("the map holds a glyph") };
    let Some(mawdi) = mabniya.khareeta.mawdi(*miftah) else { panic!("the glyph has a place") };
    let Some(ihdathiyat) = mabniya.khareeta.ihdathiyat(mawdi) else {
        panic!("the glyph's page has a recorded size")
    };
    for qeema in [ihdathiyat.s0, ihdathiyat.a0, ihdathiyat.s1, ihdathiyat.a1] {
        assert!((0.0..=1.0).contains(&qeema), "a texture coordinate outside the page: {qeema}");
    }
}

/// The conservative profile rounds the shipped height up to a power of two, and
/// still ships far less than the ceiling.
#[test]
fn ikhtibar_al_namat_al_muhafiz_yudawwir_wa_yaqusu() {
    let khiyarat = KhiyaratRasf::muhafiz();
    let mabniya = lawha(khiyarat);
    let Some(safha) = mabniya.safahat.first() else { panic!("the atlas has a page") };

    assert!(safha.irtifa.is_power_of_two(), "{} is not a power of two", safha.irtifa);
    assert!(safha.irtifa < khiyarat.aqsa_irtifa);
    assert_eq!(safha.bayt.len(), usize::from(safha.ard) * usize::from(safha.irtifa));
}

/// A compiled atlas crosses into the glyph transport, page sizes and all.
#[test]
fn ikhtibar_jisr_al_lawha_ila_al_naql() {
    let mabniya = lawha(KhiyaratRasf::default());
    let jahiza = LawhaJahiza::min_lawha(&mabniya).expect("the atlas crosses into the transport");

    assert_eq!(
        usize::from(jahiza.adad_safahat()),
        mabniya.khareeta.adad_safahat(),
        "every page came across"
    );
    assert_eq!(jahiza.adad(), mabniya.khareeta.adad(), "every glyph came across");

    for (miftah, mawdi) in mabniya.khareeta.murattaba() {
        let khana = MiftahKhana::min_miftah_shakl(miftah);
        assert_eq!(jahiza.mawdi(khana), Some(mawdi), "{khana} kept its rectangle");
        assert_eq!(
            jahiza.qiyas_safha(mawdi.safha),
            mabniya.khareeta.abaad_safha(mawdi.safha),
            "{khana}'s page kept its cropped size, which is what its texture coordinates \
             are derived from"
        );
    }
}

/// Two images that narrow onto one transport key are refused, not resolved.
#[test]
fn ikhtibar_al_jisr_yarfud_takrar_al_miftah() {
    // The same font, size and glyph in two subpixel buckets: two distinct keys
    // in a compiled atlas, one key in a transport that cannot express a bucket.
    let ashkal: Vec<MiftahShakl> = (0..2)
        .map(|bakat| MiftahShakl::jadeed(0, 5, HAJM, NamatSafha::Taghtiya, bakat))
        .collect();
    let mabniya = Lawha::ibni(
        &ashkal,
        &khutut(),
        KhiyaratRasf::default(),
        KhiyaratMisafa::default(),
        NamatSafha::Taghtiya,
    )
    .expect("two buckets of one glyph pack");
    assert_eq!(mabniya.khareeta.adad(), 2, "the compiled atlas really does hold both");

    let khata = LawhaJahiza::min_lawha(&mabniya)
        .expect_err("two images for one transport key is a refusal");
    let sabab = khata.to_string();
    assert!(
        sabab.contains("two images"),
        "the refusal names what collided rather than a category: {sabab}"
    );
}
