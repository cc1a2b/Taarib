//! الرقعة الخارجية — a patch somebody else made, that Taarib lists and installs
//! but never built.
//!
//! Most finished Arabic translations are for engines Taarib has no adapter for,
//! and they install by replacing game files. Taarib cannot compile them. It can
//! install them better than their own installers can, because everything written
//! and everything removed goes through `taarib-tathbeet`'s manifest with the
//! original bytes beside it, so the whole thing comes back. That reversibility is
//! the only reason a user would choose Taarib for a patch Taarib did not make,
//! and it is the only thing these types exist to make possible.
//!
//! ## Where these definitions belong
//!
//! In `taarib-mustalahat`, with the rest of the shared vocabulary. They are here
//! because this crate is the lowest point both the gate and the installer can
//! see — `taarib-tathbeet` depends on `taarib-aman` — and one definition that
//! both read is worth more than two that can drift. When `taarib-mustalahat`
//! carries them, this module becomes a re-export and nothing else in either
//! crate changes: the names, the fields and the shapes here are the agreed
//! contract, verbatim.
//!
//! ## What the types refuse to let a caller express
//!
//! An artifact with no pin. [`QitaatTanzeel`] carries the size **and** the
//! sha256, both required, because a size alone is trivially matched and a hash
//! alone lets a server stream forever. Both are checked before anything reaches
//! a game directory.
//!
//! A mirror by default. [`HalatMira`] defaults to the author's own endpoint and
//! the registry copy is a separate variant that has to be chosen deliberately —
//! and, per [`mira_masmuha`], can only be chosen when the permission on record
//! covers redistribution at all.

use taarib_mustalahat::ruqaa::{IdhnMasdar, MasdarKhariji};

// The definitions this module used to carry now live in `taarib-mustalahat`,
// with the rest of the shared vocabulary, exactly as the note above said they
// should. They are re-exported rather than moved out of sight because both the
// gate and the installer reach for them through this crate, and one definition
// two crates read is worth more than two that can drift.
pub use taarib_mustalahat::khariji::{
    HalatMira, QitaatTanzeel, RuqaaKharijiya, SababRafdKhariji, TahdheerKhariji, TakhtitKhariji,
};

/// Whether an entry's recorded permission covers mirroring its artifacts.
///
/// Mirroring is off by default and is the one thing here that puts somebody
/// else's bytes on Taarib's own infrastructure, so it needs a permission that
/// covers redistribution *and* a written statement from the author to read that
/// permission out of. A licence that permits redistribution with no statement
/// beside it is not enough: the licence speaks for the work, and the question
/// being asked is whether this particular author agreed to this particular copy.
///
/// All three are required. `yasmah_bilmira` is the author's grant for *hosting*
/// specifically, kept apart from the grant to republish because they are not the
/// same permission: an author may be glad for their translation to be listed and
/// installed by Taarib and still want the bytes served from their own endpoint,
/// where they can withdraw them. Mirroring also puts game-derived containers on
/// Taarib's infrastructure, which the no-game-bytes certificate must never be
/// read as covering.
#[must_use]
pub fn mira_masmuha(masdar: &MasdarKhariji) -> bool {
    let bi_katab = match &masdar.idhn {
        IdhnMasdar::Katabi { bayan } => !bayan.trim().is_empty(),
        IdhnMasdar::Rukhsa | IdhnMasdar::LamYuthbat => false,
    };
    bi_katab && masdar.rukhsa.yasmah_bil_mira() && masdar.yasmah_bilmira
}

/// Whether an entry's recorded permission is a written grant from the author.
///
/// The gate's first question about any third-party entry. A licence file is not
/// a grant, and the absence of a licence file is not a permissive one: an entry
/// whose permission is anything but a non-empty written statement is refused
/// before a byte is fetched.
#[must_use]
pub fn idhn_katabi(masdar: &MasdarKhariji) -> bool {
    match &masdar.idhn {
        IdhnMasdar::Katabi { bayan } => !bayan.trim().is_empty(),
        IdhnMasdar::Rukhsa | IdhnMasdar::LamYuthbat => false,
    }
}

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::ruqaa::RukhsaRuqaa;

    use super::{
        HalatMira, IdhnMasdar, MasdarKhariji, QitaatTanzeel, TakhtitKhariji, idhn_katabi,
        mira_masmuha,
    };

    /// Republishing and rehosting are separate grants, so the fixture takes the
    /// restrictive default: a source that mirrored without being asked to would
    /// stop these tests proving that mirroring needs its own permission.
    fn masdar(rukhsa: RukhsaRuqaa, idhn: IdhnMasdar) -> MasdarKhariji {
        masdar_bi_mira(rukhsa, idhn, false)
    }

    fn masdar_bi_mira(
        rukhsa: RukhsaRuqaa,
        idhn: IdhnMasdar,
        yasmah_bilmira: bool,
    ) -> MasdarKhariji {
        MasdarKhariji {
            ism: "Emad Adel".to_owned(),
            rabt: "https://github.com/emadadeldev/rtea".to_owned(),
            rukhsa,
            idhn,
            yasmah_bilmira,
        }
    }

    #[test]
    fn al_idhn_al_katabi_wahdahu_yahtasib() {
        assert!(idhn_katabi(&masdar(
            RukhsaRuqaa::MilkiyaKhassa,
            IdhnMasdar::Katabi {
                bayan: "the author said so on 2026-09-11, <url>".to_owned(),
            },
        )));
        // A statement that is only whitespace is a field somebody left empty.
        assert!(!idhn_katabi(&masdar(
            RukhsaRuqaa::Cc0,
            IdhnMasdar::Katabi {
                bayan: "   ".to_owned(),
            },
        )));
        assert!(!idhn_katabi(&masdar(RukhsaRuqaa::Cc0, IdhnMasdar::Rukhsa)));
        assert!(!idhn_katabi(&masdar(
            RukhsaRuqaa::Cc0,
            IdhnMasdar::LamYuthbat
        )));
    }

    #[test]
    fn al_mira_tahtaj_rukhsa_wa_katab_maan() {
        let katabi = IdhnMasdar::Katabi {
            bayan: "the author agreed, <url>, 2026-09-11".to_owned(),
        };
        assert!(mira_masmuha(&masdar_bi_mira(
            RukhsaRuqaa::CcBy,
            katabi.clone(),
            true
        )));
        // The grant to republish is not the grant to rehost: everything else
        // about this source permits mirroring, and without the hosting flag it
        // is still refused.
        assert!(!mira_masmuha(&masdar(RukhsaRuqaa::CcBy, katabi.clone())));
        // RTEA's own case: a written grant and no licence declared at all.
        assert!(!mira_masmuha(&masdar_bi_mira(
            RukhsaRuqaa::MilkiyaKhassa,
            katabi,
            true
        )));
        assert!(!mira_masmuha(&masdar_bi_mira(
            RukhsaRuqaa::CcBy,
            IdhnMasdar::Rukhsa,
            true
        )));
    }

    #[test]
    fn al_mira_mutlaqa_min_al_muallif() {
        assert!(!HalatMira::default().min_sijill());
        assert!(
            HalatMira::MinAlsijill {
                rabt: "https://mira.taarib.example/rtea/update.zip".to_owned(),
            }
            .min_sijill()
        );
    }

    #[test]
    fn al_takhtit_yutabiq_al_maqati_la_al_huruf() {
        let takhtit = TakhtitKhariji {
            yaktub: vec!["lml".to_owned(), "version.dll".to_owned()],
            yahdhif: Vec::new(),
        };
        assert!(takhtit.yasmah("lml"));
        assert!(takhtit.yasmah("lml/RTEA/Subtitles/texts/a.yldb"));
        assert!(takhtit.yasmah("version.dll"));
        // The two a raw prefix match would have let through.
        assert!(!takhtit.yasmah("lmlx/evil.dll"));
        assert!(!takhtit.yasmah("version.dll.bak"));
        assert!(!takhtit.yasmah("dinput8.dll"));
    }

    #[test]
    fn al_qitaa_bila_abniya_tantabiq_ala_al_kull() {
        let qitaa = QitaatTanzeel {
            ism: "update.zip".to_owned(),
            rabt: "https://rt.emadadeldev.workers.dev/?lml-update".to_owned(),
            hajm: 25_578_943,
            sha256: "ffd54019f146b8db8a02d6413c592222ae9de397c522631d3b65fe882b3e1343".to_owned(),
            abniya: Vec::new(),
        };
        assert!(qitaa.yantabiq("1491"));
        let khass = QitaatTanzeel {
            ism: "extra.zip".to_owned(),
            rabt: "https://rt.emadadeldev.workers.dev/?lml-extra".to_owned(),
            hajm: 1_541_075,
            sha256: "82d3008d4d77066a4336a853861603ef50b3c5d64d4cfe45f52236156ea41715".to_owned(),
            abniya: vec!["1311".to_owned(), "1436".to_owned()],
        };
        assert!(khass.yantabiq("1436"));
        assert!(!khass.yantabiq("1491"));

        let mutashabih = QitaatTanzeel {
            sha256: khass.sha256.to_uppercase(),
            ..khass.clone()
        };
        assert!(khass.yutabiq(&mutashabih), "hex case is not a mismatch");
        let mukhtalif = QitaatTanzeel {
            hajm: khass.hajm.saturating_add(1),
            ..khass.clone()
        };
        assert!(!khass.yutabiq(&mukhtalif), "a size is half the pin");
    }
}
