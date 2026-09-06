//! الخريطة — the character table `bio4.exe` carries, and the cell it selects.
//!
//! ## What this closes
//!
//! Every other module in this crate reads a file. This one does not, because the
//! thing it describes is not in a file: *Resident Evil 4* keeps the map from a
//! code point to a glyph cell **inside its own executable**, as a flat array of
//! `u32` code points whose *index* is the cell. Nothing in `BIO4/Font`,
//! `BIO4/text` or `BIO4/ImagePack` states it, which is why this crate's header
//! said for a long time that no character table existed. It exists. It is one
//! array per language, it is reachable, and the game's own text proves it: every
//! code point used by all eight shipped dictionaries appears in the table for
//! that dictionary's language, with the sole exceptions of `U+000A`, `U+000D`,
//! `U+FEFF` — which are control characters and never glyphs — and one `U+2013`
//! in `FRENCH_WIN32.dct` that the game must already draw as a blank.
//!
//! ## The chain, end to end
//!
//! Every address below is a virtual address in the Ultimate HD Edition's
//! `bio4.exe`, 9 139 840 bytes, SHA-256
//! `19aed4af0ab06a748ff8744d45ac5580fcd6be6b6b7e944b1ab8822a00c8ee4a`, PE32 with
//! an image base of `0x0040_0000`. Each was read out of the instruction bytes,
//! not inferred from behaviour.
//!
//! ```text
//! BIO4/text/<LANG>_WIN32.dct        UTF-8, or `^<decimal>^` for a code point
//!   │
//!   ├─ 0x006A_85A0  next code point: a plain UTF-8 decoder (0xF8/0xF0, 0xF0/0xE0,
//!   │               0xE0/0xC0 lead-byte tests) that also expands `^917550^` into
//!   │               U+E002E by parsing the digits, and `^^` into a literal `^`
//!   │
//!   ├─ 0x006A_7F50  code point → cell.  THE TABLE.  A `std::map<u32, u16>` built
//!   │               once from the arrays below, keyed by code point, valued by the
//!   │               array index.  Returns 0 when the code point is absent
//!   │
//!   ├─ 0x006C_260E  `mov ecx, 0x80` / `add ax, cx` — the cell becomes a
//!   │               message-stream code
//!   │
//!   ├─ 0x0071_9BF2  `mov edx, 0x80` / `sub cx, dx` — the stream code becomes a
//!   │               cell again
//!   │
//!   ├─ 0x0071_8F90  cell → `.fnt` metrics: byte offset `cell * 2` into the table
//!   │               at font descriptor `+0xD0`, giving (left, right); cell 0 is
//!   │               special-cased to offset 2 because entry 0 is the degenerate
//!   │               `(pitch, 0)` blank
//!   │
//!   └─ 0x0071_7660  cell → atlas position:
//!                     columns = texture width  / cell width
//!                     x = (cell % columns) * cell width
//!                     y = (cell / columns) * cell height
//! ```
//!
//! That last step is [`crate::shabaka`]'s grid law, and finding it in the
//! executable is what turns that law from a rule that fits thirty-two files into
//! the rule the game actually runs. The texture width it divides is the `u16` at
//! offset 2 of the embedded TPL image header, which is [`crate::tibl`]'s width;
//! the cell pitch is **not** read from the metrics table at all but passed in at
//! load time — `0x1C` for the CJK `common` fonts, `0x14` for `system`, `0x40` for
//! the Latin `common_p` — and it coincides with `metrics[0].yasar` in every
//! shipped file.
//!
//! ## The seven tables
//!
//! The language byte is at `[[0x00C0_6F40] + 8]`. Two maps are built: the main
//! one, and a second one the drawing code selects with its own flag for the CJK
//! languages only. Japanese and traditional Chinese also fall through into the
//! Latin table afterwards, and simplified Chinese does not — the duplicate keys
//! that produces are rejected, because the game inserts through
//! `std::map::insert`, which keeps the entry that was already there.
//!
//! | language byte | which map | array | entries | count immediate |
//! | --- | --- | --- | --- | --- |
//! | 0 Japanese | main | `0x00C0_FE68` | 1 016 | `0x006A_7FF0` |
//! | 0 Japanese | second | `0x00C0_FAD8` | 189 | `0x006A_8060` |
//! | 6 Chinese (traditional) | main | `0x00C0_EA50` | 881 | `0x006A_80E4` |
//! | 6 Chinese (traditional) | second | `0x00C0_E6D8` | 184 | `0x006A_8152` |
//! | 7 Chinese (simplified) | main | `0x00C0_D670` | 875 | `0x006A_81E0` |
//! | 7 Chinese (simplified) | second | `0x00C0_D2F8` | 184 | `0x006A_824C` |
//! | anything else — the five Latin scripts | main | `0x00C0_CE18` | 260 | `0x006A_82DC` |
//!
//! Only the Latin array is reproduced here, as [`RUMUZ_LATINI`]. The three CJK
//! pairs are named and not copied: 3 205 more code points would be four fifths of
//! this file and Arabic is not going into a Japanese build.
//!
//! ## Redirection, which is the part that matters
//!
//! Each array's base address is a 32-bit immediate inside a
//! `mov ecx, [eax*4 + <base>]`, and each count is a 32-bit immediate in a
//! `mov ecx, <count>` that is then compared sixteen bits wide against the loop
//! counter. For the Latin table the base sits at virtual address `0x006A_8286`,
//! file offset `0x002A_7686`, and the count at `0x006A_82DC`, file offset
//! `0x002A_76DC`; [`JADAWIL`] carries both offsets for all seven. Every base
//! immediate is covered by a base relocation, so an edited absolute address stays
//! correct if the image is rebased.
//!
//! So the table can be **pointed somewhere else**, not merely overwritten: write a
//! larger array into a new section, change one dword to its address and one dword
//! to its length, and the game reads as many code points as you gave it. That is a
//! patch. It is not a switch statement, and it is not a hard-coded range.
//!
//! And it may not be needed at all. Taarib rewrites `BIO4/text/*.dct` anyway, and
//! the shipped Latin table already offers [`ADAD_KHANAT_HAYYA`] distinct cells. An
//! Arabic patch can leave `bio4.exe` untouched, re-author the cells those code
//! points already select, and emit the matching code points — as UTF-8 or as the
//! `^<decimal>^` escape the decoder expands, which keeps the dictionary pure
//! ASCII. Not touching a signed executable is worth a great deal, and this is the
//! route that does not.
//!
//! ## Five cells you cannot reach
//!
//! The Latin array names `U+0020` at index 0 and again at 183, 185, 187 and 189,
//! and `U+002D` at index 17 and again at 32. Because the game rejects duplicate
//! keys, those five later indices are cells the atlas holds and no code point
//! selects. [`khana_hayya`] reports that, and [`khanat_hayya`] skips them, so a
//! caller allocating cells never hands one out that cannot be asked for.

/// The one thing a `.dct` says that is a control character and not a cell.
///
/// `0x006C_25F1` compares the decoded code point against `0x0A` before the table
/// is consulted and turns it into stream code 3 directly. It is the only such
/// case, and it is why a newline in a dictionary is not a missing glyph.
pub const RAMZ_SATR_JADEED: u32 = 0x0A;

/// What the game adds to a cell to make a message-stream code.
///
/// `mov ecx, 0x80` at `0x006C_260E` and `add ax, cx` behind it, undone by
/// `mov edx, 0x80` at `0x0071_9BF2` and `sub cx, dx` behind that. Codes below this
/// are the interpreter's own control codes, which is why the bias exists at all.
pub const IZAHAT_TAYYAR: u16 = 0x80;

/// How many cells the shipped Latin table addresses.
pub const ADAD_KHANAT_LATINI: u16 = 260;

/// How many of those cells a code point can actually select.
///
/// Five of the 260 repeat a code point an earlier index already claimed, and the
/// game keeps the earlier one. See this module's last section.
pub const ADAD_KHANAT_HAYYA: u16 = 255;

/// One of the seven arrays, and where the executable names it.
///
/// The two file offsets are what a patcher edits: the first holds the array's
/// address, the second holds its length. Both are little-endian `u32` in the
/// instruction stream of the Ultimate HD Edition's `bio4.exe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JadwalKharita {
    /// Which language byte selects it, and which of the two maps it fills.
    pub wasf: &'static str,
    /// The array's virtual address.
    pub unwan: u32,
    /// How many `u32` code points it holds.
    pub adad: u32,
    /// File offset of the 32-bit immediate holding [`Self::unwan`].
    pub mawdi_unwan: u32,
    /// File offset of the 32-bit immediate holding [`Self::adad`].
    pub mawdi_adad: u32,
}

/// Every character table in `bio4.exe`, in address order.
///
/// Read this rather than the module header when writing a patcher: the header
/// explains, this is the data.
pub const JADAWIL: [JadwalKharita; 7] = [
    JadwalKharita {
        wasf: "Latin scripts, main map",
        unwan: 0x00C0_CE18,
        adad: 260,
        mawdi_unwan: 0x002A_7686,
        mawdi_adad: 0x002A_76DC,
    },
    JadwalKharita {
        wasf: "Chinese (simplified), second map",
        unwan: 0x00C0_D2F8,
        adad: 184,
        mawdi_unwan: 0x002A_75F6,
        mawdi_adad: 0x002A_764C,
    },
    JadwalKharita {
        wasf: "Chinese (simplified), main map",
        unwan: 0x00C0_D670,
        adad: 875,
        mawdi_unwan: 0x002A_7586,
        mawdi_adad: 0x002A_75E0,
    },
    JadwalKharita {
        wasf: "Chinese (traditional), second map",
        unwan: 0x00C0_E6D8,
        adad: 184,
        mawdi_unwan: 0x002A_74F8,
        mawdi_adad: 0x002A_7552,
    },
    JadwalKharita {
        wasf: "Chinese (traditional), main map",
        unwan: 0x00C0_EA50,
        adad: 881,
        mawdi_unwan: 0x002A_748A,
        mawdi_adad: 0x002A_74E4,
    },
    JadwalKharita {
        wasf: "Japanese, second map",
        unwan: 0x00C0_FAD8,
        adad: 189,
        mawdi_unwan: 0x002A_7406,
        mawdi_adad: 0x002A_7460,
    },
    JadwalKharita {
        wasf: "Japanese, main map",
        unwan: 0x00C0_FE68,
        adad: 1016,
        mawdi_unwan: 0x002A_7396,
        mawdi_adad: 0x002A_73F0,
    },
];

/// The Latin character table, verbatim from `0x00C0_CE18`.
///
/// Index *i* is cell *i*. The `U+E00xx` entries are the button-prompt glyphs the
/// dictionaries write as `^917536^`-style escapes; the rest is the repertoire the
/// five Latin dictionaries use.
pub static RUMUZ_LATINI: [u32; 260] = [
    0x00020, 0xE0020, 0xE0021, 0x00030, 0x00031, 0x00032, 0x00033, 0x00034, 0x00035, 0x00036,
    0x00037, 0x00038, 0x00039, 0x0003A, 0x00025, 0x00026, 0x0002B, 0x0002D, 0x0002F, 0x0003D,
    0x0002C, 0x0002E, 0x02022, 0x02026, 0x00028, 0x00029, 0x00021, 0x0003F, 0x0201C, 0x0201D,
    0x0007E, 0xE0035, 0x0002D, 0x0003C, 0x0003E, 0x0005B, 0x0005D, 0xE0036, 0xE0037, 0x00041,
    0x00042, 0x00043, 0x00044, 0x00045, 0x00046, 0x00047, 0x00048, 0x00049, 0x0004A, 0x0004B,
    0x0004C, 0x0004D, 0x0004E, 0x0004F, 0x00050, 0x00051, 0x00052, 0x00053, 0x00054, 0x00055,
    0x00056, 0x00057, 0x00058, 0x00059, 0x0005A, 0x00061, 0x00062, 0x00063, 0x00064, 0x00065,
    0x00066, 0x00067, 0x00068, 0x00069, 0x0006A, 0x0006B, 0x0006C, 0x0006D, 0x0006E, 0x0006F,
    0x00070, 0x00071, 0x00072, 0x00073, 0x00074, 0x00075, 0x00076, 0x00077, 0x00078, 0x00079,
    0x0007A, 0x000E2, 0x000EA, 0x000EE, 0x000F4, 0x000FB, 0x000C2, 0x000CA, 0x000CE, 0x000D4,
    0x000DB, 0x000E0, 0x000E8, 0x000EC, 0x000F2, 0x000F9, 0x000C0, 0x000C8, 0x000CC, 0x000D2,
    0x000D9, 0x000E1, 0x000E9, 0x000ED, 0x000F3, 0x000FA, 0x000FD, 0x000C1, 0x000C9, 0x000CD,
    0x000D3, 0x000DA, 0x000DD, 0x000E4, 0x000EB, 0x000EF, 0x000F6, 0x000FC, 0x000FF, 0x000C4,
    0x000CB, 0x000CF, 0x000D6, 0x000DC, 0x00178, 0x000E3, 0x000F5, 0x000C3, 0x000D5, 0x000F1,
    0x000D1, 0x000E5, 0x000C5, 0x000E7, 0x000C7, 0x000F8, 0x000D8, 0x000FE, 0x000DE, 0x00161,
    0x00160, 0x000DF, 0x000D0, 0x00192, 0x000B5, 0x000F0, 0x000E6, 0x00153, 0x000C6, 0x00152,
    0x000BA, 0x000A1, 0x000BF, 0x00027, 0x02122, 0x0003B, 0x00023, 0x00040, 0xE0022, 0xE0023,
    0xE0024, 0xE0025, 0xE0026, 0x00022, 0x0201E, 0x000AE, 0xE0027, 0xE0028, 0xE0029, 0xE002A,
    0x0002A, 0x000D7, 0xE002B, 0x00020, 0xE002C, 0x00020, 0xE002D, 0x00020, 0xE002E, 0x00020,
    0xE002F, 0xE0030, 0xE0031, 0xE0032, 0xE0033, 0xE0034, 0xE0038, 0xE0039, 0xE003A, 0xE003B,
    0xE003C, 0xE003D, 0xE003E, 0xE003F, 0xE0040, 0xE0041, 0xE0042, 0xE0043, 0xE0044, 0xE0045,
    0xE0046, 0xE0047, 0xE0048, 0xE0049, 0xE004A, 0xE004B, 0xE004C, 0xE004D, 0xE004E, 0xE004F,
    0xE0050, 0xE0051, 0xE0052, 0xE0053, 0xE0054, 0xE0055, 0x00024, 0x0005C, 0x0005E, 0x0005F,
    0x00060, 0x0007B, 0x0007C, 0x0007D, 0x000B0, 0x000B1, 0x000B2, 0x000B3, 0x000B4, 0x000B6,
    0x000B7, 0x000B8, 0x000B9, 0x000A2, 0x000A3, 0x000A4, 0x000A5, 0x000A6, 0x000A7, 0x000A8,
    0x000A9, 0x000AA, 0x000AB, 0x000AC, 0x000AF, 0x000BB, 0x000BC, 0x000BD, 0x000BE, 0x000F7,
];

/// The code point that selects a cell, if any code point does.
///
/// [`None`] past the end of the table. A cell that *is* in the table but is a
/// duplicate still answers with its code point here — ask [`khana_hayya`] whether
/// asking for that code point comes back to this cell.
#[must_use]
pub fn ramz_khana(khana: u16) -> Option<u32> {
    RUMUZ_LATINI.get(usize::from(khana)).copied()
}

/// The cell a code point selects, the way the game resolves it.
///
/// A linear scan that stops at the first match, which is not an optimisation
/// choice: the game inserts these into a `std::map` in index order and
/// `std::map::insert` keeps the entry already present, so the *lowest* index
/// holding a code point is the one that wins. Any other search order would
/// disagree with the game on the five duplicated entries.
#[must_use]
pub fn khana_ramz(ramz: u32) -> Option<u16> {
    let fahras = RUMUZ_LATINI.iter().position(|&mawjud| mawjud == ramz)?;
    u16::try_from(fahras).ok()
}

/// Whether a cell can be selected by a code point at all.
///
/// False past the end of the table, and false for the five cells whose code point
/// an earlier cell already claimed.
#[must_use]
pub fn khana_hayya(khana: u16) -> bool {
    ramz_khana(khana).and_then(khana_ramz) == Some(khana)
}

/// Every reachable cell with the code point that selects it, in cell order.
///
/// This is the allocation order a patch should walk: it never yields a cell a
/// dictionary cannot ask for.
pub fn khanat_hayya() -> impl Iterator<Item = (u16, u32)> {
    RUMUZ_LATINI
        .iter()
        .enumerate()
        .filter_map(|(fahras, &ramz)| Some((u16::try_from(fahras).ok()?, ramz)))
        .filter(|&(khana, _)| khana_hayya(khana))
}

#[cfg(test)]
mod ikhtibarat {
    use super::*;

    #[test]
    fn tul_al_jadwal_yutabiq_al_shifra() {
        // The count immediate the executable compares against is 0x104.
        assert_eq!(RUMUZ_LATINI.len(), usize::from(ADAD_KHANAT_LATINI));
        assert_eq!(ADAD_KHANAT_LATINI, 0x104);
    }

    #[test]
    fn khana_sifr_hiya_al_faragh() {
        // Cell zero is the space in every table the executable carries, and it is
        // the cell whose metrics entry the drawing code refuses to read.
        assert_eq!(ramz_khana(0), Some(0x20));
        assert_eq!(khana_ramz(0x20), Some(0));
    }

    #[test]
    fn al_hija_fi_mawadiihi() {
        // 'A'..'Z' are cells 39..64 and 'a'..'z' are 65..90, contiguously.
        for (khatwa, harf) in ('A'..='Z').enumerate() {
            let mutawaqqa = u16::try_from(39 + khatwa).ok();
            assert_eq!(khana_ramz(harf as u32), mutawaqqa, "{harf}");
        }
        for (khatwa, harf) in ('a'..='z').enumerate() {
            let mutawaqqa = u16::try_from(65 + khatwa).ok();
            assert_eq!(khana_ramz(harf as u32), mutawaqqa, "{harf}");
        }
        for (khatwa, harf) in ('0'..='9').enumerate() {
            let mutawaqqa = u16::try_from(3 + khatwa).ok();
            assert_eq!(khana_ramz(harf as u32), mutawaqqa, "{harf}");
        }
    }

    #[test]
    fn al_khanat_al_mayyita_khamsa() {
        let mayyita: Vec<u16> = (0..ADAD_KHANAT_LATINI)
            .filter(|&khana| !khana_hayya(khana))
            .collect();
        assert_eq!(mayyita, vec![32, 183, 185, 187, 189]);
        assert_eq!(
            usize::from(ADAD_KHANAT_HAYYA),
            usize::from(ADAD_KHANAT_LATINI) - mayyita.len()
        );
        assert_eq!(khanat_hayya().count(), usize::from(ADAD_KHANAT_HAYYA));
    }

    #[test]
    fn al_mayyita_tukarrir_ma_qablaha() {
        // A dead cell is dead for exactly one reason: an earlier cell holds the
        // same code point. Asserting the reason keeps the list above from being a
        // number somebody once measured.
        for khana in [32_u16, 183, 185, 187, 189] {
            let ramz = ramz_khana(khana).unwrap_or_default();
            let awwal = khana_ramz(ramz).unwrap_or(khana);
            assert!(awwal < khana, "cell {khana} should repeat an earlier one");
            assert_eq!(ramz_khana(awwal), Some(ramz));
        }
    }

    #[test]
    fn kull_khana_hayya_taud_ila_nafsiha() {
        for (khana, ramz) in khanat_hayya() {
            assert_eq!(khana_ramz(ramz), Some(khana));
            assert_eq!(ramz_khana(khana), Some(ramz));
        }
    }

    #[test]
    fn ma_khilf_al_jadwal_laysa_khana() {
        assert_eq!(ramz_khana(ADAD_KHANAT_LATINI), None);
        assert_eq!(ramz_khana(u16::MAX), None);
        assert!(!khana_hayya(ADAD_KHANAT_LATINI));
        // U+0627 ARABIC LETTER ALEF is not in a table the game ships, which is the
        // whole reason the atlas has to be re-authored rather than extended.
        assert_eq!(khana_ramz(0x0627), None);
    }

    #[test]
    fn jadwal_al_jadawil_murattab_wa_ghayr_mutadakhil() {
        let mut sabiq: Option<&JadwalKharita> = None;
        for jadwal in &JADAWIL {
            if let Some(qabl) = sabiq {
                let nihaya = qabl.unwan + qabl.adad * 4;
                assert!(
                    nihaya <= jadwal.unwan,
                    "{} overlaps {}",
                    qabl.wasf,
                    jadwal.wasf
                );
            }
            sabiq = Some(jadwal);
        }
        let latini = JADAWIL.first().map(|jadwal| jadwal.adad);
        assert_eq!(latini, Some(u32::from(ADAD_KHANAT_LATINI)));
    }
}
