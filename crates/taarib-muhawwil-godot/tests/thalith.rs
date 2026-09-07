//! The Godot 3 path, checked against files Godot 3 itself wrote.
//!
//! Every fixture in `tests/thalith/` came out of Godot 3.6.stable
//! (`de2f0f147`), running headless, and none of it was authored by hand:
//!
//! | file | how it was produced |
//! | --- | --- |
//! | `muharrik36.ar.translation` | `Translation.new()`, four `add_message` calls, `ResourceSaver.save` |
//! | `muharrik36_phash.ar.translation` | `PHashTranslation.new()`, `generate(…)` from the same `Translation`, `ResourceSaver.save` |
//! | `muharrik36.pck` | a four-file project exported with `--export-pack`, so a real PCK v1 with the engine's own index and MD5s |
//!
//! The four messages are `Hello`, `New Game`, `Café — “quoted”` and `日本語`,
//! translated to Arabic. Two of those source strings are pure ASCII and two are
//! not, and that is not decoration — it is the control for
//! [`taarib_muhawwil_godot::pck::tarjama::basma_godot`], whose byte widening
//! only matters above 0x7F and which this suite exists in part to pin down. See
//! [`basma_tutabiq_jadwal_al_muharrik`].
//!
//! ## What is proved here and what is not
//!
//! These tests prove that Taarib reads what Godot 3 writes without losing a
//! byte, and that what Taarib writes has the shape Godot 3's own loader
//! requires — the format version it accepts, the class names it registers, the
//! property types its setters take, and the hash its lookup computes. They do
//! not, and cannot, prove that a Godot 3 binary loads the result, because
//! `cargo test` has no engine in it. That step was done separately, by running
//! the same headless 3.6 build against a delivery this code generated; the
//! transcript is in the phase report and the mechanism is
//! [`taarib_muhawwil_godot::tawseel`]'s rung one.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory; the product's own recursive deletes go through `HadafHadhf`"
)]

use std::path::{Path, PathBuf};

use taarib_muhawwil_godot::khadim_nusus::{
    Lahja, MadkhalIdad, MalafTajawuz, QeemaIdad, TarjamatLuba,
};
use taarib_muhawwil_godot::pck::Mawrid as _;
use taarib_muhawwil_godot::pck::hawiya::{Hawiya, ISDAR_AWWAL, IsdarHawiya};
use taarib_muhawwil_godot::pck::tarjama::{
    ISDAR_SIGHA_THALITH, JeelMawrid, Khasiya, MawridTarjama, NAW_BASITA, NAW_MURAKKAZA,
    NAW_MURAKKAZA_QADEEM, Qeema, ShaklRasail, THABIT_BASMA, Tarjama, TarjamaMurakkaza, basma_godot,
};
use taarib_muhawwil_godot::tawseel::{
    MIFTAH_TARJAMAT, MIFTAH_THAQAFA_IHTIYAT, MIFTAH_THAQAFA_IKHTIBAR, MawdiTarjama, TawseelThalith,
};

// ---------------------------------------------------------------------------
// The fixtures
// ---------------------------------------------------------------------------

/// A plain `Translation` saved by Godot 3.6's own `ResourceSaver`.
const MAWRID_BASIT: &[u8] = include_bytes!("thalith/muharrik36.ar.translation");

/// A `PHashTranslation` generated and saved by Godot 3.6 from that same
/// `Translation`.
const MAWRID_MURAKKAZ: &[u8] = include_bytes!("thalith/muharrik36_phash.ar.translation");

/// A PCK v1 written by Godot 3.6's own exporter.
const HAZMA: &[u8] = include_bytes!("thalith/muharrik36.pck");

/// The four messages the fixtures hold, in the order the engine stored them.
const RASAIL: [(&str, &str); 4] = [
    ("Hello", "مرحبا"),
    ("New Game", "لعبة جديدة"),
    ("Café — “quoted”", "مقهى"),
    ("日本語", "اليابانية"),
];

/// The two sources whose UTF-8 has a byte above 0x7F, which is where the
/// hash's byte widening becomes visible.
const RASAIL_GHAYR_ASCII: [&str; 2] = ["Café — “quoted”", "日本語"];

// ---------------------------------------------------------------------------
// Reading what the engine wrote
// ---------------------------------------------------------------------------

/// A resource Godot 3 wrote, read and written back, is the same bytes.
///
/// The contract the whole `pck` module rests on. It is checked here against a
/// file this code did not produce, which is the only version of the check worth
/// anything: a reader and a writer that agree with each other and disagree with
/// the engine round-trip perfectly and are both wrong.
#[test]
fn mawrid_basit_yadur_bila_taghyeer() {
    let Ok(mawrid) = MawridTarjama::min_bayt(MAWRID_BASIT) else {
        panic!("Godot 3.6's own Translation resource did not read");
    };
    assert_eq!(
        mawrid.ghilaf().isdar,
        ISDAR_SIGHA_THALITH,
        "the engine wrote format 3"
    );
    assert_eq!(
        mawrid.ghilaf().muharrik,
        (3, 6),
        "the engine recorded its own version"
    );
    assert_eq!(
        mawrid.ghilaf().mahjuz.len(),
        14,
        "format 3 carries fourteen reserved words"
    );
    assert_eq!(mawrid.naw(), NAW_BASITA);
    assert_eq!(mawrid.ghilaf().jeel(), JeelMawrid::Thalith);

    let Ok(kutiba) = mawrid.ila_bayt() else {
        panic!("the resource did not write back")
    };
    assert_eq!(
        kutiba.as_slice(),
        MAWRID_BASIT,
        "the round trip is not byte-identical"
    );
}

/// The same, for the hash-table form.
#[test]
fn mawrid_murakkaz_yadur_bila_taghyeer() {
    let Ok(mawrid) = MawridTarjama::min_bayt(MAWRID_MURAKKAZ) else {
        panic!("Godot 3.6's own PHashTranslation resource did not read");
    };
    assert_eq!(
        mawrid.naw(),
        NAW_MURAKKAZA_QADEEM,
        "Godot 3 calls it PHashTranslation"
    );
    assert!(mawrid.murakkaza());
    assert_eq!(mawrid.ghilaf().isdar, ISDAR_SIGHA_THALITH);

    let Ok(kutiba) = mawrid.ila_bayt() else {
        panic!("the resource did not write back")
    };
    assert_eq!(
        kutiba.as_slice(),
        MAWRID_MURAKKAZ,
        "the round trip is not byte-identical"
    );
}

/// Godot 3 stores `Translation::messages` as a flat `PoolStringArray`.
///
/// Checked on the value rather than taken from the documentation, because the
/// alternative — a `Dictionary`, which is what Godot 4 stores — is a value
/// Godot 3's `_set_messages(const PoolVector<String> &)` cannot convert, and
/// the failure is a silently empty translation.
#[test]
fn rasail_thalitha_masfufa_musattaha() {
    let Ok(mawrid) = MawridTarjama::min_bayt(MAWRID_BASIT) else {
        panic!("the fixture did not read");
    };
    let Some(Qeema::Nusus(nusus)) = mawrid.khasiya("messages") else {
        panic!("the engine's own messages property is not a packed string array");
    };
    assert_eq!(nusus.len(), RASAIL.len() * 2, "source and target alternate");

    let Ok(tarjama) = Tarjama::min_mawrid(&mawrid) else {
        panic!("the messages did not read")
    };
    assert_eq!(tarjama.shakl(), ShaklRasail::Masfufa);
    assert_eq!(tarjama.thaqafa(), "ar");
    for (masdar, hadaf) in RASAIL {
        assert_eq!(
            tarjama.ibhath(masdar),
            Some(hadaf),
            "{masdar} did not resolve"
        );
    }
}

// ---------------------------------------------------------------------------
// The hash, measured against the engine's own table
// ---------------------------------------------------------------------------

/// The unsigned widening — what this crate used to do, kept as the control.
///
/// The engine's loop is `d = (d * 0x1000193) ^ uint32_t(*p_str)` over a
/// `const char *`. This is what it would compute if `char` were unsigned, which
/// it is on Linux ARM and is not on any target this product ships to.
fn basma_ghayr_muwaqqa(daala: u32, nass: &str) -> u32 {
    let mut halat = if daala == 0 { THABIT_BASMA } else { daala };
    for wahid in nass.as_bytes() {
        halat = halat.wrapping_mul(THABIT_BASMA) ^ u32::from(*wahid);
    }
    halat
}

/// Every source in a table Godot 3 generated is found by this crate's lookup.
///
/// This is the test the byte-widening question turns on. The table was built by
/// the engine, so its slot assignments and its per-bucket keys are the engine's
/// arithmetic and not this crate's. If the widening here disagreed with the
/// engine's, the two ASCII sources would still be found — every byte in them is
/// below 0x80, so there is nothing to sign-extend — and the two others would
/// not be, which is exactly the shape of the failure a reader could stare past.
/// So the assertion is made twice: the lookup finds all four, and the control
/// hash disagrees with the real one on precisely the two non-ASCII sources.
#[test]
fn basma_tutabiq_jadwal_al_muharrik() {
    let Ok(jadwal) = TarjamaMurakkaza::min_bayt(MAWRID_MURAKKAZ) else {
        panic!("the engine's own hash table did not read");
    };
    assert_eq!(jadwal.thaqafa(), "ar");

    for (masdar, hadaf) in RASAIL {
        match jadwal.ibhath(masdar) {
            Ok(Some(wujid)) => assert_eq!(wujid, hadaf, "{masdar} resolved to the wrong text"),
            Ok(None) => panic!(
                "{masdar} is in a table the engine generated and this crate's lookup missed it"
            ),
            Err(khata) => panic!("{masdar} refused: {khata}"),
        }
    }

    for masdar in RASAIL_GHAYR_ASCII {
        assert_ne!(
            basma_godot(0, masdar),
            basma_ghayr_muwaqqa(0, masdar),
            "{masdar} has a byte above 0x7F, so the two widenings must disagree — if they \
             do not, this test is no longer measuring anything"
        );
    }
    for masdar in ["Hello", "New Game"] {
        assert_eq!(
            basma_godot(0, masdar),
            basma_ghayr_muwaqqa(0, masdar),
            "{masdar} is pure ASCII, so the two widenings must agree"
        );
    }
}

/// A table this crate generates answers to the same lookup.
///
/// The other direction of the same fact. A generated table is not required to be
/// byte-identical to the engine's — the seed search and the bucket order are, in
/// practice, but the pool layout follows the message order the caller supplied —
/// so what is asserted is the property the engine actually depends on.
#[test]
fn murakkaza_muwallada_tujib() {
    let Ok(asl) = Tarjama::min_bayt(MAWRID_BASIT) else {
        panic!("the fixture did not read")
    };
    let Ok(jadwal) = TarjamaMurakkaza::min_tarjama(&asl) else {
        panic!("the hash table did not generate");
    };
    for (masdar, hadaf) in RASAIL {
        assert_eq!(
            jadwal.ibhath(masdar).ok().flatten().as_deref(),
            Some(hadaf),
            "{masdar} did not resolve in a table this crate generated"
        );
    }
    assert!(
        matches!(
            jadwal.ibhath("a string that is not in this table"),
            Ok(None)
        ),
        "a miss is a miss and not an error"
    );
}

// ---------------------------------------------------------------------------
// Writing what Godot 3 will accept
// ---------------------------------------------------------------------------

/// Reads the format version out of a written resource's header.
///
/// Byte 20, after the magic, the two flags and the two engine words. Read here
/// rather than through this crate's own reader, so that a reader and a writer
/// that agreed on a wrong offset could not both pass.
fn sighat_mawrid(bayt: &[u8]) -> u32 {
    let Some(khana) = bayt.get(20..24) else {
        panic!("the written resource has no header")
    };
    let Ok(arbaa) = <[u8; 4]>::try_from(khana) else {
        panic!("four bytes are four bytes")
    };
    u32::from_le_bytes(arbaa)
}

/// A translation written for Godot 3 is a format 3 `Translation`.
///
/// Every one of these assertions is a separate way for the file to be refused
/// or misread by a Godot 3 engine, and all of them were wrong before this
/// phase: the writer produced format 4 with eleven reserved words, a flags word
/// and a UID, which `ResourceInteractiveLoaderBinary::open` rejects outright
/// with "uses a format version which is not supported by your engine version".
#[test]
fn tarjama_thalitha_bi_sighat_thalitha() {
    let mut tarjama = Tarjama::jadeeda("ar", ShaklRasail::Qamus);
    for (masdar, hadaf) in RASAIL {
        assert!(tarjama.daa(masdar, hadaf).is_ok());
    }
    let Ok(bayt) = tarjama.ila_bayt(JeelMawrid::Thalith, (3, 0)) else {
        panic!("the Godot 3 resource did not write");
    };
    assert_eq!(sighat_mawrid(&bayt), ISDAR_SIGHA_THALITH);

    let Ok(mawrid) = MawridTarjama::min_bayt(&bayt) else {
        panic!("it did not read back")
    };
    assert_eq!(mawrid.naw(), NAW_BASITA);
    assert_eq!(mawrid.ghilaf().mahjuz.len(), 14);
    assert_eq!(mawrid.ghilaf().muharrik, (3, 0));
    assert!(mawrid.ghilaf().sanf.is_none());

    // The shape follows the generation and not the value it was built with:
    // this translation was created as a Godot 4 dictionary and written for
    // Godot 3, which is what a translation lifted out of a Godot 4 game and
    // delivered to a Godot 3 one is.
    let Some(Qeema::Nusus(nusus)) = mawrid.khasiya("messages") else {
        panic!("a Godot 3 translation's messages must be a packed string array");
    };
    assert_eq!(nusus.len(), RASAIL.len() * 2);

    let Ok(qari) = Tarjama::min_mawrid(&mawrid) else {
        panic!("the messages did not read")
    };
    for (masdar, hadaf) in RASAIL {
        assert_eq!(qari.ibhath(masdar), Some(hadaf));
    }
}

/// The same translation written for Godot 4 keeps Godot 4's shape.
///
/// The control for the test above: the generation argument decides, and neither
/// answer leaks into the other.
#[test]
fn tarjama_rabia_tabqa_qamusan() {
    let mut tarjama = Tarjama::jadeeda("ar", ShaklRasail::Masfufa);
    assert!(tarjama.daa("Hello", "مرحبا").is_ok());
    let Ok(bayt) = tarjama.ila_bayt(JeelMawrid::Rabi, (4, 3)) else {
        panic!("the Godot 4 resource did not write");
    };
    assert_eq!(sighat_mawrid(&bayt), 4);
    let Ok(mawrid) = MawridTarjama::min_bayt(&bayt) else {
        panic!("it did not read back")
    };
    assert_eq!(mawrid.ghilaf().mahjuz.len(), 11);
    assert!(matches!(
        mawrid.khasiya("messages"),
        Some(Qeema::Qamus { .. })
    ));
}

/// The hash-table form written for Godot 3 is called `PHashTranslation`.
///
/// The class name is what the engine instantiates. Godot 3 has never had a
/// class called `OptimizedTranslation`, so the previous name was a resource
/// whose class the engine could not find.
#[test]
fn murakkaza_thalitha_tusamma_phash() {
    let Ok(asl) = Tarjama::min_bayt(MAWRID_BASIT) else {
        panic!("the fixture did not read")
    };
    let Ok(jadwal) = TarjamaMurakkaza::min_tarjama(&asl) else {
        panic!("the hash table did not generate");
    };

    let Ok(thalith) = jadwal.ila_bayt(JeelMawrid::Thalith, (3, 0)) else {
        panic!("the Godot 3 hash table did not write");
    };
    assert_eq!(sighat_mawrid(&thalith), ISDAR_SIGHA_THALITH);
    let Ok(mawrid) = MawridTarjama::min_bayt(&thalith) else {
        panic!("it did not read back")
    };
    assert_eq!(mawrid.naw(), NAW_MURAKKAZA_QADEEM);

    let Ok(rabi) = jadwal.ila_bayt(JeelMawrid::Rabi, (4, 3)) else {
        panic!("the Godot 4 hash table did not write");
    };
    let Ok(mawrid) = MawridTarjama::min_bayt(&rabi) else {
        panic!("it did not read back")
    };
    assert_eq!(mawrid.naw(), NAW_MURAKKAZA);
}

/// A `StringName` cannot be written into a Godot 3 resource.
///
/// Tag 44 was added in Godot 4. Godot 3's binary loader is a `switch` whose
/// default arm fails the whole file, so one such value anywhere costs the
/// entire translation — which is why this is refused at the writer rather than
/// left to be discovered by the engine.
#[test]
fn wasf_rabi_marfud_fi_sighat_thalitha() {
    let khasais = vec![Khasiya::jadeeda("locale", Qeema::IsmNass("ar".to_owned()))];
    let thalith = MawridTarjama::li_jeel(NAW_BASITA, (3, 0), JeelMawrid::Thalith, khasais.clone());
    assert!(
        thalith.ila_bayt().is_err(),
        "a StringName in a format 3 resource must be refused, not written"
    );

    let rabi = MawridTarjama::li_jeel(NAW_BASITA, (4, 3), JeelMawrid::Rabi, khasais);
    assert!(
        rabi.ila_bayt().is_ok(),
        "the same value is ordinary on Godot 4"
    );
}

// ---------------------------------------------------------------------------
// The package the engine wrote
// ---------------------------------------------------------------------------

/// A real PCK v1 reads, its index round-trips, and its entries verify.
#[test]
fn hazmat_al_muharrik_tuqra_wa_tadur() {
    let Ok(hawiya) = Hawiya::min_bayt(HAZMA) else {
        panic!("the engine's own pack did not read")
    };
    let tarwisa = hawiya.tarwisa();
    assert_eq!(tarwisa.isdar, IsdarHawiya::Awwal, "Godot 3 writes PCK v1");
    assert_eq!(tarwisa.isdar.raqm(), ISDAR_AWWAL);
    assert_eq!(tarwisa.muharrik, (3, 6, 0));
    assert_eq!(
        tarwisa.qaida, 0,
        "version 1 has no file base and measures from the package"
    );
    assert!(!tarwisa.fahras_mushaffar());
    assert!(
        !hawiya.mudmaj(),
        "this one is a loose pack, not appended to an executable"
    );
    assert_eq!(hawiya.madakhil().len(), 4);

    let Ok(kutiba) = hawiya.fahras().ila_bayt() else {
        panic!("the index did not write back")
    };
    let Some(asli) = usize::try_from(hawiya.fahras().tul())
        .ok()
        .and_then(|tul| HAZMA.get(..tul))
    else {
        panic!("the index length is outside the file it was read from");
    };
    assert_eq!(
        kutiba.as_slice(),
        asli,
        "the index round trip is not byte-identical"
    );

    // Extraction verifies each entry against the MD5 the engine recorded, so a
    // successful read is also a checksum match.
    for madkhal in hawiya.madakhil() {
        assert!(
            madkhal.bi_basma(),
            "the engine records a digest for every entry"
        );
        let Ok(bayt) = hawiya.istakhrij(HAZMA, madkhal, None) else {
            panic!("{} did not extract", madkhal.masar);
        };
        assert_eq!(u64::try_from(bayt.len()).unwrap_or(u64::MAX), madkhal.hajm);
    }
    let Ok(idadat) = hawiya.istakhrij_masar(HAZMA, "res://project.binary", None) else {
        panic!("the packed project settings did not extract");
    };
    assert!(!idadat.is_empty());
}

// ---------------------------------------------------------------------------
// The delivery
// ---------------------------------------------------------------------------

/// A directory of this test's own, removed and recreated so a rerun starts
/// clean.
fn mujallad_ikhtibar(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-godot3-{ism}"));
    let _ = std::fs::remove_dir_all(&masar);
    assert!(
        std::fs::create_dir_all(&masar).is_ok(),
        "{} could not be created",
        masar.display()
    );
    masar
}

/// Builds the delivery this suite exercises, against a directory.
fn tawseel_ikhtibar(jidhr: &Path) -> TawseelThalith {
    let Ok(mawdi) = MawdiTarjama::mustakhdim(&jidhr.join("mustakhdim"), "taarib/ar.translation")
    else {
        panic!("the user-directory location was refused");
    };
    let mut tarjama = Tarjama::jadeeda("ar", ShaklRasail::Masfufa);
    for (masdar, hadaf) in RASAIL {
        assert!(tarjama.daa(masdar, hadaf).is_ok());
    }
    TawseelThalith::jadeed(
        MalafTajawuz::fi_mujallad(jidhr),
        mawdi,
        TarjamatLuba::Maqrua(vec!["res://locale/en.translation".to_owned()]),
    )
    .bi_tarjama(tarjama)
    .bi_muharrik((3, 6))
}

/// The delivery writes a resource Godot 3 loads and an override naming it.
#[test]
fn tawseel_yaktub_ma_yaqrauhu_godot() {
    let jidhr = mujallad_ikhtibar("tawseel");
    let tawseel = tawseel_ikhtibar(&jidhr);

    let Ok(natija) = tawseel.hayyi() else {
        panic!("the delivery installed nothing")
    };
    assert!(natija.wusul(), "the translated text must reach the game");
    assert!(
        natija.mawrid.muakkad(),
        "the resource rung must be confirmed"
    );
    assert!(
        natija.thaqafa.muakkad(),
        "the locale rung must be confirmed"
    );
    assert!(
        !natija.rasm.muakkad(),
        "no takeover was declared, so the shaping concern must report unmet — a delivery \
         that claimed the text was legible would be the report this whole module exists to \
         avoid"
    );
    assert!(
        !natija.muakkad(),
        "delivery alone is not the Godot 3 path complete"
    );

    // The resource is a Godot 3 one, at the place the setting names, holding
    // the Arabic.
    let masar = tawseel.mawdi().mutlaq_masar();
    assert!(masar.is_file(), "{} was not written", masar.display());
    let Ok(bayt) = std::fs::read(masar) else {
        panic!("the written resource did not read")
    };
    assert_eq!(sighat_mawrid(&bayt), ISDAR_SIGHA_THALITH);
    let Ok(qari) = Tarjama::min_bayt(&bayt) else {
        panic!("the written resource did not parse")
    };
    assert_eq!(qari.thaqafa(), "ar");
    for (masdar, hadaf) in RASAIL {
        assert_eq!(
            qari.ibhath(masdar),
            Some(hadaf),
            "{masdar} is not in the delivered file"
        );
    }

    // The override is in Godot 3's namespace and Godot 3's spelling, and it
    // keeps the game's own translation entry.
    let Ok(nass) = std::fs::read_to_string(tawseel.tajawuz().masar()) else {
        panic!("the override was not written");
    };
    assert!(
        nass.starts_with("; taarib:tajawuz"),
        "the marker identifies the file as ours"
    );
    assert!(
        nass.contains("[locale]"),
        "Godot 3's settings live under locale/, not under \
                                        internationalization/"
    );
    assert!(nass.contains("test=\"ar\""));
    assert!(nass.contains("fallback=\"ar\""));
    assert!(
        nass.contains("PoolStringArray("),
        "Godot 3.2's VariantParser has no PackedStringArray, and a value it cannot parse is a \
         translation list the game starts without"
    );
    assert!(!nass.contains("PackedStringArray("));
    assert!(
        nass.contains("res://locale/en.translation"),
        "the game's own entry must survive"
    );
    assert!(
        nass.contains("user://taarib/ar.translation"),
        "the patch's entry must be there"
    );
    assert!(!nass.contains("internationalization/"));

    // Uninstalling is deleting the two files it wrote.
    let Ok((tajawuz, mawrid)) = tawseel.tarajua() else {
        panic!("the uninstall refused")
    };
    assert!(
        tajawuz && mawrid,
        "both files were Taarib's and both must go"
    );
    assert!(!tawseel.tajawuz().masar().exists());
    assert!(!masar.exists());
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// Declaring the takeover is what makes the report say the text will be legible.
#[test]
fn tawseel_maa_istila_yuakkid_al_rasm() {
    let jidhr = mujallad_ikhtibar("istila");
    let tawseel = tawseel_ikhtibar(&jidhr).bi_istila(true);
    let Ok(natija) = tawseel.hayyi() else {
        panic!("the delivery installed nothing")
    };
    assert!(
        natija.muakkad(),
        "delivery plus takeover is the Godot 3 path complete"
    );
    assert!(natija.rasm.muakkad());
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A delivery with no messages refuses rather than writing a settings file that
/// names a resource which is not there.
#[test]
fn tawseel_bila_tarjama_yarfud() {
    let jidhr = mujallad_ikhtibar("farigh");
    let Ok(mawdi) = MawdiTarjama::mustakhdim(&jidhr.join("mustakhdim"), "taarib/ar.translation")
    else {
        panic!("the location was refused");
    };
    let tawseel = TawseelThalith::jadeed(
        MalafTajawuz::fi_mujallad(&jidhr),
        mawdi,
        TarjamatLuba::Maqrua(Vec::new()),
    );
    assert!(tawseel.hayyi().is_err());
    assert!(!tawseel.mawdi().mutlaq_masar().exists());
    let _ = std::fs::remove_dir_all(&jidhr);
}

/// A location that would escape the game's user directory is refused.
#[test]
fn mawdi_kharij_mujallad_al_mustakhdim_marfud() {
    let jidhr = Path::new("/tmp/taarib-godot3-wahmi");
    assert!(MawdiTarjama::mustakhdim(jidhr, "../khariji.translation").is_err());
    assert!(MawdiTarjama::mustakhdim(jidhr, "taarib/../../khariji.translation").is_err());
    assert!(MawdiTarjama::mustakhdim(jidhr, "").is_err());
    assert!(MawdiTarjama::mustakhdim(jidhr, "taarib/ar.translation").is_ok());
    assert!(MawdiTarjama::mutlaq(Path::new("nisbi/ar.translation")).is_err());
    assert!(MawdiTarjama::mutlaq(Path::new("/tmp/ar.translation")).is_ok());
}

/// The list literal is spelled the way the target engine's parser reads it.
#[test]
fn qaima_bi_lahjat_kull_jeel() {
    let madkhal = MadkhalIdad::jadeed(
        MIFTAH_TARJAMAT,
        QeemaIdad::Qaima(vec!["user://taarib/ar.translation".to_owned()]),
    );
    assert_eq!(
        madkhal.qism(),
        "locale",
        "Godot splits a setting path at the first slash"
    );
    assert_eq!(madkhal.miftah_qism(), "translations");
    assert!(madkhal.satr_bi(Lahja::Thalith).contains("PoolStringArray("));
    assert!(madkhal.satr_bi(Lahja::Rabi).contains("PackedStringArray("));

    // The two locale keys are Godot 3's own, not Godot 4's namespace.
    assert_eq!(MIFTAH_THAQAFA_IKHTIBAR, "locale/test");
    assert_eq!(MIFTAH_THAQAFA_IHTIYAT, "locale/fallback");
}

/// A quote or a newline in a value cannot end the line it is written on.
#[test]
fn qeema_khabitha_la_taftah_bab_al_idadat() {
    let khabith = QeemaIdad::Nass("a\"b\nc/test=\"x".to_owned());
    let nass = khabith.nass_idad_bi(Lahja::Thalith);
    assert!(
        !nass.contains('\n'),
        "a raw newline would start a second key"
    );
    assert!(!nass.contains('\r'));
    assert!(nass.starts_with('"') && nass.ends_with('"'));

    // The property that matters is not how many quotes there are — it is that
    // none of the ones inside the value can close it. Every `"` but the two
    // delimiters must be preceded by a backslash.
    let jism: Vec<char> = nass.chars().collect();
    let akhir = jism.len().saturating_sub(1);
    for (fahras, harf) in jism.iter().enumerate() {
        if fahras == 0 || fahras == akhir || *harf != '"' {
            continue;
        }
        assert_eq!(
            jism.get(fahras.saturating_sub(1)),
            Some(&'\\'),
            "an unescaped quote at {fahras} would close the value and let the rest of it \
             become further keys in the section"
        );
    }
}
