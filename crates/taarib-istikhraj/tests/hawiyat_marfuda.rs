//! Containers this build cannot read, and the report each one must produce.
//!
//! Every fixture is built from the container's real structure rather than from
//! a description of it: a version 11 pak with the three regions a shipped
//! UE 4.27 container carries, minus the full-directory index a cook may prune;
//! a version 5 pak written by this workspace's own writer and then altered the
//! way a damaged or an AES-locked one differs from it; a version 2 Godot package
//! whose locked members are encrypted with the cipher, the mode and the frame
//! the engine's exporter uses. Each altered fixture is first shown to be what it
//! claims — the pruned pak's twin reads to the end, the locked member decrypts
//! under its key through the reader's own two digest checks — because a test
//! that only proved the reader refuses some bytes would prove nothing about the
//! report a real game produces.
//!
//! What every test asserts is one thing from a different side: an empty table
//! with a reason is a different answer from an empty table with none.

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; the lints are written for library code, and refusing to panic here \
              would mean a test that cannot fail"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory; the product's own recursive deletes go through `HadafHadhf`"
)]

use std::path::{Path, PathBuf};

use aes::Aes256;
use cbc::cipher::block_padding::NoPadding;
use cbc::cipher::{BlockEncryptMut, KeyIvInit};
use taarib_istikhraj::rafd::{MadkhalRafd, SababRafd, TaqreerRafd};
use taarib_istikhraj::{godot, unreal};
use taarib_muhawwil_godot::pck::basma_md5;
use taarib_muhawwil_godot::pck::hawiya::{
    ALAM_MADKHAL_MUSHAFFAR, BinaHawiya, HAJM_MIFTAH, Hawiya, HawiyaMaftuha, IsdarHawiya,
};
use taarib_muhawwil_godot::pck::tarjama::{JeelMawrid, ShaklRasail, Tarjama};
use taarib_muhawwil_unreal::mawarid::locres::{IsdarLocres, MawridLocres};
use taarib_muhawwil_unreal::mawarid::pak::{HawiyatPak, KatibPak, SIHR, basma_bayt};
use taarib_muhawwil_unreal::mawarid::{Katib, Mawrid as _, NassMukhazzan};

/// Where an Unreal game keeps its one container, relative to the root.
const HAWIYAT_PAK: &str = "Atlas/Content/Paks/Atlas-WindowsNoEditor.pak";

/// The one `.locres` the version 11 fixture holds, and its two halves as the
/// full-directory index spells them.
const MASAR_LOCRES: &str = "Atlas/Content/Localization/Game/en/Game.locres";
const DALIL_LOCRES: &str = "Atlas/Content/Localization/Game/en/";
const ISM_LOCRES: &str = "Game.locres";

/// The one package the version 5 fixtures hold.
const MASAR_HIZMA: &str = "Atlas/Content/UI/ST_Menu.uasset";

/// Pak version 11, whose index is a path-hash index plus an optional
/// full-directory index.
const ISDAR_MASARAT: u32 = 11;

/// The mount point this workspace's writer and every shipped container use.
const NUQTAT_WASL: &str = "../../../";

/// A raw, unencrypted entry header is fifty-three bytes from version 3 on.
const HAJM_RAAS: usize = 53;

/// Where the encryption flag sits in that header: after three `i64`s, the
/// method word and the twenty-byte hash.
const IZAHAT_ALAM_TASHFEER: usize = 48;

/// The version 5 footer: flag, magic, version, offset, size, hash.
const HAJM_TADHYEEL_KHAMIS: usize = 45;

/// The Godot fixture's members.
const MASAR_INJILIZI: &str = "res://locale/en.translation";
const MASAR_ARABI: &str = "res://locale/ar.translation";
const MASAR_MASHHAD: &str = "res://ui/menu.tscn";

/// The export key and vector the Godot fixture is locked with.
const MIFTAH: [u8; HAJM_MIFTAH] = [0x5A; HAJM_MIFTAH];
const MUTTAJIH: [u8; 16] = [0xA5; 16];

/// A directory of this test's own, removed and recreated so a rerun starts
/// clean.
fn mujallad_ikhtibar(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-istikhraj-marfuda-{ism}"));
    let _ = std::fs::remove_dir_all(&masar);
    std::fs::create_dir_all(&masar).expect("the scratch directory");
    masar
}

/// Places a fixture, creating the directories above it.
fn uktub(masar: &Path, bayt: &[u8]) {
    if let Some(walid) = masar.parent() {
        std::fs::create_dir_all(walid).expect("the fixture's directory");
    }
    std::fs::write(masar, bayt).expect("the fixture");
}

/// Every refusal the report holds against one container.
fn marfudat<'a>(taqreer: &'a TaqreerRafd, hawiya: &str) -> Vec<&'a MadkhalRafd> {
    taqreer
        .marfuda
        .iter()
        .filter(|madkhal| madkhal.hawiya == hawiya)
        .collect()
}

/// A length as the signed field the pak format writes.
fn musir(tul: usize) -> i64 {
    i64::try_from(tul).expect("a fixture is small")
}

// ---------------------------------------------------------------------------
// Unreal: version 11, with and without its full-directory index
// ---------------------------------------------------------------------------

/// A `.locres` with two English strings, as this workspace's writer produces it.
fn locres_injilizi() -> Vec<u8> {
    let mut mawrid = MawridLocres::jadeed(IsdarLocres::MuhassanMadina);
    mawrid.adif("", "Back", "Back", "Back").expect("one entry");
    mawrid
        .adif("Menu", "Play", "Play", "Play")
        .expect("a second entry");
    mawrid.ahsi_ihsaat();
    mawrid.ila_bayt().expect("a .locres this workspace writes")
}

/// One ANSI `FString`, as every index writes it.
fn nass(katib: &mut Katib, nass: &str) {
    katib
        .uktub_nass(
            "a fixture",
            "a fixture string",
            &NassMukhazzan::jadeed(nass),
        )
        .expect("an ASCII string fits its length field");
}

/// The fifty-three byte entry header of a raw, unencrypted file.
fn raas_madkhal(izaha: i64, hajm: usize, basma: &[u8; 20]) -> Vec<u8> {
    let mut katib = Katib::jadeed();
    katib.uktub_i64(izaha);
    katib.uktub_i64(musir(hajm));
    katib.uktub_i64(musir(hajm));
    // Method index zero: stored raw. Then the hash, the encryption flag and
    // the block size, which every version from 3 on writes.
    katib.uktub_u32(0);
    katib.uktub_bayt(basma);
    katib.uktub_u8(0);
    katib.uktub_u32(0);
    katib.ila_vec()
}

/// A version 11 container holding one file, with or without the
/// full-directory index a cook may prune.
///
/// The layout is the one `UnrealPak` writes: the entry's second copy and its
/// payload, the path-hash index, the full-directory index when present, the
/// primary index, and the 221-byte footer. The primary index carries the
/// offset, length and SHA-1 of each secondary region, and the footer carries
/// the same three for the primary index, so every region is checked the way a
/// shipped container's is.
fn pak_masarat(muhtawa: &[u8], bi_dalil: bool) -> Vec<u8> {
    let mut malaf = Katib::jadeed();
    malaf.uktub_bayt(&raas_madkhal(0, muhtawa.len(), &basma_bayt(muhtawa)));
    malaf.uktub_bayt(muhtawa);

    // The path-hash index: one pair. This build checks the pair count and
    // resolves nothing through the hash itself, so the hash is a placeholder.
    let mut basmat = Katib::jadeed();
    basmat.uktub_i32(1);
    basmat.uktub_i64(0x0123_4567_89ab_cdef);
    basmat.uktub_i32(0);
    let basmat = basmat.ila_vec();
    let izahat_basmat = malaf.mawqi();
    malaf.uktub_bayt(&basmat);

    // The full-directory index: one directory holding one file, whose entry
    // is the first record of the encoded blob.
    let mut dalil = Katib::jadeed();
    dalil.uktub_i32(1);
    nass(&mut dalil, DALIL_LOCRES);
    dalil.uktub_i32(1);
    nass(&mut dalil, ISM_LOCRES);
    dalil.uktub_i32(0);
    let dalil = dalil.ila_vec();
    let izahat_dalil = malaf.mawqi();
    if bi_dalil {
        malaf.uktub_bayt(&dalil);
    }

    // The encoded entry: a 32-bit offset and a 32-bit expanded size, stored
    // raw, with no compression blocks — which is how the engine encodes an
    // uncompressed file, and what leaves it fifty-three bytes to its payload.
    let mut murammaz = Katib::jadeed();
    murammaz.uktub_u32((1 << 31) | (1 << 30));
    murammaz.uktub_u32(0);
    murammaz.uktub_u32(u32::try_from(muhtawa.len()).expect("a fixture is small"));
    let murammaz = murammaz.ila_vec();

    let mut fahras = Katib::jadeed();
    nass(&mut fahras, NUQTAT_WASL);
    fahras.uktub_i32(1);
    fahras.uktub_i64(0x5eed);
    fahras.uktub_i32(1);
    fahras.uktub_i64(musir(izahat_basmat));
    fahras.uktub_i64(musir(basmat.len()));
    fahras.uktub_bayt(&basma_bayt(&basmat));
    if bi_dalil {
        fahras.uktub_i32(1);
        fahras.uktub_i64(musir(izahat_dalil));
        fahras.uktub_i64(musir(dalil.len()));
        fahras.uktub_bayt(&basma_bayt(&dalil));
    } else {
        fahras.uktub_i32(0);
    }
    fahras.uktub_i32(i32::try_from(murammaz.len()).expect("a fixture is small"));
    fahras.uktub_bayt(&murammaz);
    fahras.uktub_i32(0);
    let fahras = fahras.ila_vec();
    let izahat_fahras = malaf.mawqi();
    malaf.uktub_bayt(&fahras);

    // The footer: the key GUID, the index-encryption flag, the magic, the
    // version, where the primary index is and its hash, then the five
    // compression-method slots.
    malaf.uktub_bayt(&[0u8; 16]);
    malaf.uktub_u8(0);
    malaf.uktub_u32(SIHR);
    malaf.uktub_u32(ISDAR_MASARAT);
    malaf.uktub_i64(musir(izahat_fahras));
    malaf.uktub_i64(musir(fahras.len()));
    malaf.uktub_bayt(&basma_bayt(&fahras));
    malaf.uktub_bayt(&[0u8; 160]);
    malaf.ila_vec()
}

/// A pak whose directory index was pruned is refused by name, not reported as
/// a container that was read and holds no text.
#[test]
fn pak_bi_dalil_mabtur_yurfad_la_yuqal_bila_nusus() {
    let jidhr = mujallad_ikhtibar("pak-mabtur");
    let locres = locres_injilizi();

    // The twin with its directory index is a container this build reads to
    // the end, which is what makes the pruned one a fixture and not a guess.
    let kamil = pak_masarat(&locres, true);
    let qari = HawiyatPak::min_bayt(kamil.clone(), Path::new(HAWIYAT_PAK), None)
        .expect("the version 11 fixture opens");
    assert!(qari.fahras().dalil_kamil());
    assert_eq!(
        qari.masarat_locres().collect::<Vec<_>>(),
        vec![MASAR_LOCRES]
    );
    uktub(&jidhr.join(HAWIYAT_PAK), &kamil);
    let (jadwal, taqreer) = unreal::istakhrij(&jidhr);
    assert_eq!(jadwal.adad(), 2, "{:?}", taqreer.taqreer());
    assert!(
        marfudat(&taqreer, HAWIYAT_PAK).is_empty(),
        "{:?}",
        taqreer.marfuda
    );

    // The same container with the index pruned names nothing, and must say so.
    let mabtur = pak_masarat(&locres, false);
    let qari = HawiyatPak::min_bayt(mabtur.clone(), Path::new(HAWIYAT_PAK), None)
        .expect("a pruned index is a container this build opens");
    assert!(!qari.fahras().dalil_kamil());
    assert_eq!(
        qari.fahras().adad_muallan(),
        1,
        "the container still declares its one file"
    );
    assert_eq!(qari.adad(), 0, "and can name none of them");
    uktub(&jidhr.join(HAWIYAT_PAK), &mabtur);
    let (jadwal, taqreer) = unreal::istakhrij(&jidhr);
    assert!(jadwal.khali());
    let marfudat = marfudat(&taqreer, HAWIYAT_PAK);
    assert_eq!(marfudat.len(), 1, "{marfudat:?}");
    let SababRafd::SighaMajhula { wujid } = &marfudat[0].sabab else {
        panic!(
            "a pruned index is not \"holds no text\": {:?}",
            marfudat[0].sabab
        )
    };
    assert!(wujid.contains("pruned"), "{wujid}");
    assert!(
        marfudat[0].asl.is_none(),
        "the refusal is about the whole container"
    );
    assert!(
        taqreer.yanfa_iltiqat(),
        "the strings a pruned index hides are drawn on screen, so capture is the remedy"
    );
    assert_eq!(taqreer.adad_khasara(), 1);
    let _ = std::fs::remove_dir_all(&jidhr);
}

// ---------------------------------------------------------------------------
// Unreal: version 5, a package that cannot be read
// ---------------------------------------------------------------------------

/// Bytes that are a package to the walk and a string table to nothing.
fn hizma_bila_jadwal() -> Vec<u8> {
    b"taarib fixture: a cooked package with no string table anywhere in it\n".repeat(8)
}

/// A version 5 container holding that one package, from this workspace's
/// own writer.
fn pak_khamis(muhtawa: &[u8]) -> Vec<u8> {
    let mut katib = KatibPak::jadeed();
    katib.daa(MASAR_HIZMA, muhtawa.to_vec()).expect("one entry");
    katib.ila_bayt().expect("a container this workspace writes")
}

/// The container with its one payload flagged encrypted, in both copies of
/// the entry header, and the index hash recomputed over the altered index —
/// which is what an AES-locked cook with a plaintext index looks like.
fn pak_mushaffar_alhizam(muhtawa: &[u8]) -> Vec<u8> {
    let mut bayt = pak_khamis(muhtawa);
    let tul = bayt.len();
    bayt[IZAHAT_ALAM_TASHFEER] = 1;

    let qari = HawiyatPak::min_bayt(bayt.clone(), Path::new(HAWIYAT_PAK), None)
        .expect("the writer's own container opens");
    let izahat_fahras = usize::try_from(qari.tadhyeel().izahat_fahras).expect("a small file");
    let hajm_fahras = usize::try_from(qari.tadhyeel().hajm_fahras).expect("a small file");
    // The index copy sits after the mount point, the count and the path,
    // each string being its length field, its bytes and a terminator.
    let izahat_raas = izahat_fahras + 4 + NUQTAT_WASL.len() + 1 + 4 + 4 + MASAR_HIZMA.len() + 1;
    bayt[izahat_raas + IZAHAT_ALAM_TASHFEER] = 1;
    let basma = basma_bayt(&bayt[izahat_fahras..izahat_fahras + hajm_fahras]);
    bayt[tul - 20..].copy_from_slice(&basma);
    bayt
}

/// A control: a package that reads and holds no table is recorded as read,
/// with nothing refused.
#[test]
fn pak_hizamuhu_bila_jadwal_yusajjal_maqruan() {
    let jidhr = mujallad_ikhtibar("pak-bila-jadwal");
    uktub(&jidhr.join(HAWIYAT_PAK), &pak_khamis(&hizma_bila_jadwal()));
    let (jadwal, taqreer) = unreal::istakhrij(&jidhr);
    assert!(jadwal.khali());
    assert!(
        marfudat(&taqreer, HAWIYAT_PAK).is_empty(),
        "{:?}",
        taqreer.marfuda
    );
    let maqru = taqreer
        .maqrua
        .iter()
        .find(|qira| qira.hawiya == HAWIYAT_PAK);
    let Some(maqru) = maqru else {
        panic!(
            "a container whose package was searched must be in the report: {:?}",
            taqreer.maqrua
        )
    };
    assert_eq!(maqru.adad, 0);
    assert!(
        maqru.wasf.contains("1 package(s) searched"),
        "{}",
        maqru.wasf
    );
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A package whose payload does not match the hash the container recorded is
/// a refusal that names it, not a container that vanishes from the report.
#[test]
fn pak_hizamuhu_talif_yuzkar_la_yakhtafi() {
    let jidhr = mujallad_ikhtibar("pak-talif");
    let mut bayt = pak_khamis(&hizma_bila_jadwal());
    bayt[HAJM_RAAS + 3] ^= 0xFF;
    uktub(&jidhr.join(HAWIYAT_PAK), &bayt);

    let (jadwal, taqreer) = unreal::istakhrij(&jidhr);
    assert!(jadwal.khali());
    assert!(
        taqreer.maqrua.iter().all(|qira| qira.hawiya != HAWIYAT_PAK),
        "nothing in it was read: {:?}",
        taqreer.maqrua
    );
    let marfudat = marfudat(&taqreer, HAWIYAT_PAK);
    assert_eq!(
        marfudat.len(),
        1,
        "a container whose only package failed to read must be in the report: {marfudat:?}"
    );
    assert!(
        matches!(marfudat[0].sabab, SababRafd::Talif { .. }),
        "{:?}",
        marfudat[0].sabab
    );
    assert_eq!(marfudat[0].asl.as_deref(), Some(MASAR_HIZMA));
    assert!(
        !taqreer.yanfa_iltiqat(),
        "a damaged container is one capture cannot help"
    );
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// An AES-locked package is refused as encrypted, which is the one refusal
/// whose remedy is capture — and the one a dropped error turned into silence.
#[test]
fn pak_hizamuhu_mushaffar_yaqtarih_al_iltiqat() {
    let jidhr = mujallad_ikhtibar("pak-mushaffar");
    let bayt = pak_mushaffar_alhizam(&hizma_bila_jadwal());

    // The fixture is what it claims: an intact container whose one entry is
    // flagged encrypted in the index this build resolves through.
    let qari = HawiyatPak::min_bayt(bayt.clone(), Path::new(HAWIYAT_PAK), None)
        .expect("the altered container still opens: only the payload is locked");
    assert!(
        qari.jid(MASAR_HIZMA)
            .is_some_and(|madkhal| madkhal.mushaffar)
    );
    uktub(&jidhr.join(HAWIYAT_PAK), &bayt);

    let (jadwal, taqreer) = unreal::istakhrij(&jidhr);
    assert!(jadwal.khali());
    assert!(taqreer.maqrua.iter().all(|qira| qira.hawiya != HAWIYAT_PAK));
    let marfudat = marfudat(&taqreer, HAWIYAT_PAK);
    assert_eq!(marfudat.len(), 1, "{marfudat:?}");
    assert!(
        matches!(marfudat[0].sabab, SababRafd::Mushaffar { .. }),
        "{:?}",
        marfudat[0].sabab
    );
    assert_eq!(marfudat[0].asl.as_deref(), Some(MASAR_HIZMA));
    assert!(
        taqreer.yanfa_iltiqat(),
        "an AES-locked container is exactly the case capture exists for"
    );
    let _ = std::fs::remove_dir_all(&jidhr);
}

// ---------------------------------------------------------------------------
// Godot: a version 2 package with encrypted members
// ---------------------------------------------------------------------------

/// A Godot 4 translation resource for one locale.
fn tarjama_rabia(thaqafa: &str, rasail: &[(&str, &str)]) -> Vec<u8> {
    let mut tarjama = Tarjama::jadeeda(thaqafa, ShaklRasail::Qamus);
    for (masdar, hadaf) in rasail {
        tarjama.daa(masdar, hadaf).expect("one message");
    }
    tarjama
        .ila_bayt(JeelMawrid::Rabi, (4, 3))
        .expect("a Godot 4 resource")
}

/// One member's stored bytes, encrypted the way Godot's exporter encrypts it:
/// the digest of the plaintext, its length, the vector, then AES-256-CBC over
/// the plaintext zero-filled to a whole block.
fn shaffir(wadih: &[u8]) -> Vec<u8> {
    let madfu = wadih.len().next_multiple_of(16);
    let mut hajz = wadih.to_vec();
    hajz.resize(madfu, 0);
    cbc::Encryptor::<Aes256>::new((&MIFTAH).into(), (&MUTTAJIH).into())
        .encrypt_padded_mut::<NoPadding>(&mut hajz, madfu)
        .expect("a whole number of blocks");
    let mut bayt = Vec::with_capacity(40 + madfu);
    bayt.extend_from_slice(&basma_md5(wadih));
    bayt.extend_from_slice(
        &u64::try_from(wadih.len())
            .expect("a small resource")
            .to_le_bytes(),
    );
    bayt.extend_from_slice(&MUTTAJIH);
    bayt.extend_from_slice(&hajz);
    bayt
}

/// A version 2 package whose Arabic translation and menu scene are encrypted
/// and whose English translation is not — what an export filter produces —
/// and the plaintext of the Arabic member, for the key check.
fn pck_bi_adaa_mushaffara() -> (Vec<u8>, Vec<u8>) {
    let arabi = tarjama_rabia("ar", &[("Hello", "مرحبا"), ("New Game", "لعبة جديدة")]);
    let injilizi = tarjama_rabia("en", &[("Hello", "Hello"), ("New Game", "New Game")]);
    let mashhad = b"[gd_scene format=3]\n\n[node name=\"Menu\" type=\"Control\"]\n\n\
                    [node name=\"Play\" type=\"Button\" parent=\".\"]\ntext = \"Play\"\n"
        .to_vec();

    let mut bina = BinaHawiya::jadeed(IsdarHawiya::Thani, (4, 3, 0));
    bina.daa(MASAR_INJILIZI, injilizi)
        .expect("the plaintext member");
    bina.daa(MASAR_ARABI, shaffir(&arabi))
        .expect("the locked translation");
    bina.daa(MASAR_MASHHAD, shaffir(&mashhad))
        .expect("the locked scene");
    let mut bayt = bina.ila_bayt().expect("a package this workspace writes");

    // The builder writes every entry plain, with the digest of the bytes it
    // was handed. Flag the two locked entries and record the digest of what
    // they decrypt to, which is what the exporter stores in the index.
    let hawiya = Hawiya::min_bayt(&bayt).expect("the builder's own package reads back");
    let mut izaha = usize::try_from(IsdarHawiya::Thani.hajm_tarwisa()).expect("a small header");
    for madkhal in hawiya.madakhil() {
        let tul_masar = usize::try_from(madkhal.tul_masar_makhzun()).expect("a short path");
        let izahat_basma = izaha + 4 + tul_masar + 8 + 8;
        let izahat_alam = izahat_basma + 16;
        let wadih = match madkhal.masar.as_str() {
            MASAR_ARABI => Some(arabi.as_slice()),
            MASAR_MASHHAD => Some(mashhad.as_slice()),
            _ => None,
        };
        if let Some(wadih) = wadih {
            bayt[izahat_basma..izahat_basma + 16].copy_from_slice(&basma_md5(wadih));
            bayt[izahat_alam..izahat_alam + 4]
                .copy_from_slice(&ALAM_MADKHAL_MUSHAFFAR.to_le_bytes());
        }
        izaha +=
            usize::try_from(madkhal.hajm_fi_al_fahras(IsdarHawiya::Thani)).expect("a short entry");
    }
    (bayt, arabi)
}

/// An encrypted member is refused with its reason, beside the members that
/// read — and two members locked one way are one line, not two.
#[test]
fn pck_adauhu_almushaffara_tudhkar_bi_sababiha() {
    let jidhr = mujallad_ikhtibar("pck-mushaffar");
    let (bayt, arabi) = pck_bi_adaa_mushaffara();
    let masar = jidhr.join("luba.pck");
    uktub(&masar, &bayt);

    // The fixture is what it claims: the locked member decrypts under its key
    // through the same reader and the same two digest checks an engine-written
    // package goes through, and is refused without one.
    let maftuha = HawiyaMaftuha::iftah(&masar).expect("the package opens: its index is plain");
    assert_eq!(
        maftuha
            .istakhrij(MASAR_ARABI, Some(&MIFTAH))
            .expect("the right key"),
        arabi
    );
    assert!(maftuha.istakhrij(MASAR_ARABI, None).is_err());
    drop(maftuha);

    let (jadwal, taqreer) = godot::istakhrij(&jidhr);
    // The plaintext member still reads, so this is a package read in part.
    assert!(
        taqreer
            .maqrua
            .iter()
            .any(|qira| qira.hawiya == "luba.pck" && qira.adad == 2),
        "{:?}",
        taqreer.maqrua
    );
    assert_eq!(jadwal.adad(), 2);
    let marfudat = marfudat(&taqreer, "luba.pck");
    assert_eq!(
        marfudat.len(),
        1,
        "two members locked one way are one line: {marfudat:?}"
    );
    let SababRafd::Mushaffar { wasf } = &marfudat[0].sabab else {
        panic!(
            "a locked member is a refusal that names encryption: {:?}",
            marfudat[0].sabab
        )
    };
    assert!(wasf.contains("2 members"), "{wasf}");
    assert_eq!(marfudat[0].asl.as_deref(), Some(MASAR_ARABI));
    assert!(
        taqreer.yanfa_iltiqat(),
        "a game decrypts its own text before drawing it"
    );
    assert_eq!(taqreer.adad_khasara(), 1);
    let _ = std::fs::remove_dir_all(&jidhr);
}
