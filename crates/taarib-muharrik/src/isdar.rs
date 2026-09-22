//! الإصدار — the version an installed game declares about itself.
//!
//! A third-party patch pins the game's *own* version — RTEA supports Red Dead
//! Redemption 2 builds 1311, 1436 and 1491 — while a launcher reports its own
//! build identifier, which is a different number written by a different party.
//! `taarib_mustalahat::khariji` states how an entry declares the relationship
//! between the two and resolves it; what it cannot do is open a file, because it
//! is the shared vocabulary and has no reader in it.
//!
//! This module is the other half: the one implementation of
//! [`QariIsdar`], backed by [`crate::dalail::tanfidhi`] — the single reader of a
//! Windows PE image and its version resource, which already documents the three
//! ways that read is done wrong and is pinned against real shipped binaries.
//! There is no second PE parser here and there must not be one.

use std::path::Path;

use taarib_mustalahat::khariji::QariIsdar;

use crate::dalail::tanfidhi;

/// Reads a game's own version out of a Windows executable's version resource.
///
/// What `taarib_mustalahat::khariji::FapsBina::MawridIsdar` names, doing the
/// reading the vocabulary crate declines to do. Stateless: it holds no handle
/// and caches nothing, so a caller builds one wherever it needs one.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QariMawridIsdar;

impl QariIsdar for QariMawridIsdar {
    fn isdar(&self, masar: &Path) -> Option<String> {
        tanfidhi::isdar_muallan(masar)
    }
}

#[cfg(test)]
mod ikhtibarat {
    use std::path::Path;

    use taarib_mustalahat::khariji::{FapsBina, TahdidBina};

    use super::QariMawridIsdar;
    use crate::dalail::tanfidhi::isdar_muallan;
    use crate::dalail::tanfidhi::suwar::{Sura, sawwir};

    /// How RDR2 declares itself, and the component its build lives in.
    fn faps_rdr2() -> FapsBina {
        FapsBina::MawridIsdar {
            masar: "RDR2.exe".to_owned(),
            juz: 2,
        }
    }

    /// One `String` entry of a version block.
    ///
    /// ```text
    /// WORD  wLength       the whole structure
    /// WORD  wValueLength  the value, in UTF-16 code units, terminator included
    /// WORD  wType         1 for text
    /// WCHAR szKey[]       NUL-terminated
    /// WORD  Padding[]     to the next 32-bit boundary
    /// WCHAR Value[]       NUL-terminated
    /// ```
    ///
    /// Built rather than copied out of a shipped binary, unlike the two fixtures
    /// in `tanfidhi` itself: those exist to pin how a *real* resource is laid
    /// out, and what is being pinned here is which component of a version string
    /// an entry's declaration selects. No RDR2 install exists on this machine to
    /// copy a block from.
    fn madkhal_nass(miftah: &str, qeema: &str) -> Vec<u8> {
        let mut jism: Vec<u8> = miftah
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        // The value sits on the next 32-bit boundary counting from the start of
        // the structure, which is six bytes of header in front of the key.
        while !jism.len().saturating_add(6).is_multiple_of(4) {
            jism.push(0);
        }
        jism.extend(qeema.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));

        let wahdat = u16::try_from(qeema.encode_utf16().count().saturating_add(1)).unwrap_or(0);
        let tul = u16::try_from(jism.len().saturating_add(6)).unwrap_or(0);
        let mut bayt: Vec<u8> = Vec::with_capacity(jism.len().saturating_add(6));
        bayt.extend_from_slice(&tul.to_le_bytes());
        bayt.extend_from_slice(&wahdat.to_le_bytes());
        bayt.extend_from_slice(&1_u16.to_le_bytes());
        bayt.extend_from_slice(&jism);
        bayt
    }

    /// A version block carrying the entries given, in order.
    ///
    /// The eight leading bytes stand in for the `VS_VERSION_INFO` header a real
    /// block opens with: the reader requires a key to have a structure in front
    /// of it, which is what stops it matching two halves of two other characters
    /// that happened to spell one.
    fn kutla(madakhil: &[(&str, &str)]) -> Vec<u8> {
        let mut bayt: Vec<u8> = vec![0; 8];
        for (miftah, qeema) in madakhil {
            bayt.extend(madkhal_nass(miftah, qeema));
            while !bayt.len().is_multiple_of(4) {
                bayt.push(0);
            }
        }
        bayt
    }

    /// Writes a game root holding one executable with the version block given.
    fn jidhr(masrah: &Path, madakhil: &[(&str, &str)]) -> std::io::Result<()> {
        std::fs::write(
            masrah.join("RDR2.exe"),
            sawwir(&Sura {
                alat: 0x8664,
                rdata: b"",
                ism_mawarid: b".rsrc",
                hashw: 0,
                kutla: &kutla(madakhil),
            }),
        )
    }

    /// The probe takes the component the entry names, out of a real image.
    ///
    /// `1.0.1491.50` with `juz: 2` is 1491 — the build RTEA pins files for — and
    /// not 1, not 0 and not 50.
    #[test]
    fn juz_min_mawrid_isdar() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        jidhr(masrah.path(), &[("FileVersion", "1.0.1491.50")])?;

        assert_eq!(
            isdar_muallan(&masrah.path().join("RDR2.exe")).as_deref(),
            Some("1.0.1491.50")
        );
        assert_eq!(
            faps_rdr2().iqra(masrah.path(), &QariMawridIsdar),
            Some("1491".to_owned())
        );
        Ok(())
    }

    /// `FileVersion` is read before `ProductVersion`, and a version written with
    /// commas is still four components.
    ///
    /// Both halves come from the same real defect: `DarkSoulsRemastered.exe`
    /// declares `1,0,0,0` and `1`. A reader that preferred the product version,
    /// or that only knew dots, would have one component to index into and would
    /// answer nothing for an install it can read perfectly well.
    #[test]
    fn isdar_almalaf_yasbiq_almuntaj() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        jidhr(
            masrah.path(),
            &[("FileVersion", "1,0,1491,50"), ("ProductVersion", "1")],
        )?;

        assert_eq!(
            isdar_muallan(&masrah.path().join("RDR2.exe")).as_deref(),
            Some("1,0,1491,50")
        );
        assert_eq!(
            faps_rdr2().iqra(masrah.path(), &QariMawridIsdar),
            Some("1491".to_owned())
        );
        Ok(())
    }

    /// A game root with no such executable answers nothing, rather than failing.
    ///
    /// Nothing determined is what `RuqaaKharijiya::halat_bina` renders as
    /// `HalatBina::Majhula` — pinned in `taarib_mustalahat`'s `faps_samit_majhul`
    /// — so the card still draws and says which builds the patch declares.
    #[test]
    fn malaf_mafqud_la_yujib() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        let tahdid = TahdidBina {
            faps: Some(faps_rdr2()),
            ..TahdidBina::default()
        };

        assert_eq!(isdar_muallan(&masrah.path().join("RDR2.exe")), None);
        assert_eq!(tahdid.hall(None, masrah.path(), &QariMawridIsdar), None);
        Ok(())
    }

    /// Neither does a file that is not a PE image at all.
    ///
    /// A publisher's launcher shim, a text file somebody renamed, a download
    /// caught half-written: all of them are read as "not determined".
    #[test]
    fn malaf_laysa_sura_la_yujib() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        std::fs::write(
            masrah.path().join("RDR2.exe"),
            b"this is not a Windows executable",
        )?;
        let tahdid = TahdidBina {
            faps: Some(faps_rdr2()),
            ..TahdidBina::default()
        };

        assert_eq!(isdar_muallan(&masrah.path().join("RDR2.exe")), None);
        assert_eq!(tahdid.hall(None, masrah.path(), &QariMawridIsdar), None);
        Ok(())
    }

    /// An image with no version resource among its resources answers nothing
    /// either, which is a third way the same page has to stay drawable.
    #[test]
    fn sura_bila_mawrid_la_tujib() -> std::io::Result<()> {
        let masrah = tempfile::tempdir()?;
        std::fs::write(
            masrah.path().join("RDR2.exe"),
            sawwir(&Sura {
                alat: 0x8664,
                rdata: b"",
                ism_mawarid: b".rsrc",
                hashw: 0,
                kutla: b"",
            }),
        )?;

        assert_eq!(isdar_muallan(&masrah.path().join("RDR2.exe")), None);
        Ok(())
    }
}
