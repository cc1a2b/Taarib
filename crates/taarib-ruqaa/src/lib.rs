//! # رقعة تعريب — the patch container
//!
//! One file holds everything a game needs to speak Arabic: the metadata, the
//! string table, the precomputed layouts, the atlas pages, the glyph map, the
//! font subset record, the per-string constraints, and the signature that makes
//! all of it trustworthy.
//!
//! The format is specified here rather than in the compiler because five
//! languages read it. Rust compiles it; C# reads it inside Unity; JavaScript
//! reads it inside RPG Maker and Electron; Python reads it inside Ren'Py; Ruby
//! reads it inside VX Ace. A format that needed a parser in each of those would
//! be five parsers to keep in agreement, so instead the tables are fixed-layout
//! POD arrays that every one of those languages can overlay directly onto
//! mapped bytes with no parsing and no allocation at all.
//!
//! Specified alongside `taarib-tarqee` in **Phase 14**, and built here in
//! **Phase 6** because the Unity adapter cannot read a patch that does not yet
//! have a shape. It is the hinge of the whole second half of the build:
//! adapters cannot read patches, the installer cannot place them, safety cannot
//! verify them, and the registry cannot distribute them until this exists.
//!
//! ## The byte layout
//!
//! ```text
//! .ruqaa — little-endian, 16-byte aligned sections, mmap-friendly
//!
//!   Header (64 bytes)
//!     magic          "TRQ1"                     4
//!     format_version u16 = 1                    2
//!     flags          u16   (sdf | coverage | rtl_mirroring | capture_hints)   2
//!     section_count  u32                        4
//!     total_size     u64                        8
//!     content_hash   [u8; 32]  BLAKE3, header exclusive, TAWQEE exclusive     32
//!     reserved                                  12
//!
//!   Section table (32 bytes each)
//!     kind u32  offset u64  length_stored u64  length_raw u64  compression u32
//!     kinds: 1 BAYAN (metadata, JSON)      2 NUSUS  (string table)
//!            3 TAKHTIT (precomputed layouts) 4 LAWHA  (atlas pages)
//!            5 KHAREETA (glyph map)          6 KHATT  (embedded font subset record)
//!            7 QIYUD  (per-string constraints and reflow hints)
//!            8 TAWQEE (signature block, never compressed, always last)
//!
//!   Every section except TAWQEE may be a single zstd frame. Whether one is
//!   compressed is a property of that section, recorded per entry, and decided
//!   by whether compressing it made it smaller — an SDF atlas usually does not.
//!
//!   NUSUS, TAKHTIT, KHAREETA and QIYUD decompress into fixed-layout POD arrays with
//!   4-byte indices, readable by struct overlay from C#, JavaScript, Rust, Python and Ruby
//!   with no parser and no allocation.
//!
//!   TAWQEE: ed25519 over a 97-byte message carrying a domain separator, the
//!           content hash, the key's role (owner | contributor-selfsigned), the
//!           algorithm, the timestamp, and the signer's public key.
//! ```
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `tarwisa` | the 64-byte header, and the validated section table built from it |
//! | `aqsam` | the eight section kinds, one table entry, and the compression descriptor |
//! | `jadawil` | the POD record definitions, and the six section preambles that frame them |
//! | `tawqee` | the signature block: layout, the exact bytes that get signed, and verification against a caller's verifier |
//! | `qari` | reading: validate framing, check the content hash, then hand out borrowed slices without copying |
//! | `katib` | writing: lay sections out, compress what benefits, hash, and reserve the signature block for `taarib-khatm` |
//! | `muhadhah` | the aligned byte buffer that makes the table casts possible off a mapping as well as on one |
//! | `khata` | every refusal this format can produce, each naming the field it refused |
//!
//! ## What the content hash covers, and why it stops early
//!
//! BLAKE3 from the first byte after the header to the first byte of the
//! signature section. Two exclusions, each for its own reason.
//!
//! The **header** is excluded because it carries the hash, and a hash cannot
//! cover itself. Nothing is lost: every other header field is checked against
//! something outside the attacker's control — `total_size` against the bytes
//! actually present, `magic`, `format_version` and `flags` against values this
//! build knows. There is no header field that can be moved without one of those
//! catching it.
//!
//! The **signature section** is excluded because the signature is over the hash.
//! That exclusion is also what makes sealing possible: `taarib-khatm` fills a
//! fixed-size reservation in place, moving no offset and recomputing nothing.
//!
//! ## Why the tables are POD and why alignment is 16
//!
//! A dialogue box in a Unity game redraws when the player presses a key, and
//! the adapter has microseconds to find the layout for a string it has already
//! been given. If finding it means deserializing a JSON object, that is an
//! allocation and a parse on a render path in a language with a garbage
//! collector. Overlaying a struct onto a mapped page is a pointer add.
//!
//! 16-byte alignment lets the sections be mapped and used in place on every
//! architecture the product targets, including the aarch64 targets where
//! unaligned access is not free, and lets `bytemuck` cast without a copy.
//! Indices are 4 bytes because no patch will hold four billion strings and
//! halving the index width halves the table.
//!
//! ## Why TAWQEE is last and uncompressed
//!
//! A signature must cover the bytes as they exist on disk, and it must be
//! verifiable by a reader that has not decompressed anything yet — so that a
//! streaming download can be rejected before its final block is written, and so
//! that safety verification never has to expand a package it does not yet
//! trust. Putting the signature at a known offset from the end, uncompressed,
//! makes both of those cheap.
//!
//! ## Hard constraints
//!
//! - The format is fixed-layout and readable by struct overlay in every
//!   consumer language. A change that requires a parser in C# is a change that
//!   is not made.
//! - `format_version` is checked before anything else is read, and a newer
//!   version is a refusal with an explanation, never a best-effort parse.
//! - Reading is bounds-checked against `total_size` and the section table
//!   before any offset is dereferenced, and the content hash is checked before
//!   any record is cast. A `.ruqaa` may arrive from anywhere and is treated as
//!   untrusted input until `taarib-aman` says otherwise.
//! - No original game asset is ever in this container. Phase 14's asset gate
//!   proves it before a package is written.
//! - Writing is deterministic: the same tables, flags and compression level
//!   produce byte-identical output, so two compiles of one project on one
//!   toolchain agree on their content hash. [`katib`] states exactly where that
//!   guarantee ends.

pub mod aqsam;
pub mod jadawil;
pub mod katib;
pub mod khata;
pub mod muhadhah;
pub mod qari;
pub mod tarwisa;
pub mod tawqee;

pub use crate::aqsam::{HAJM_MADKHAL, MUHADHAT_QISM, MadkhalQism, NawDaght, NawQism};
pub use crate::jadawil::{
    ALAM_HARF_ALAMA, ALAM_KHATT_ASASI, ALAM_KHATT_IHTIYATI, ALAM_KHATT_JADAWIL_KAMILA,
    ALAM_NITAQ_ASWAD, ALAM_NITAQ_DHARRA, ALAM_NITAQ_LAWN, ALAM_NITAQ_MAAIL, ALAM_NITAQ_SURA,
    ALAM_QAYD_HAJM_TILQAI, ALAM_QAYD_HIWAR, ALAM_QAYD_IADAT_TATHBIT, ALAM_QAYD_MIRAH,
    ALAM_QAYD_NAMU_ARD, ALAM_QAYD_NAMU_IRTIFA, ALAM_QAYD_SATR_WAHID, ALAM_SATR_AKHIR,
    ALAM_SATR_YAMEEN, ALAM_TAKHTIT_BILA_QAYD_ARD, ALAM_TAKHTIT_DHARRAT, ALAM_TAKHTIT_MAQSUS,
    ALAM_TAKHTIT_MUSAGHGHAR, ALAM_TAKHTIT_TAJAWUZ, ALAM_TAKHTIT_YAMEEN,
    HAJM_TASDIR, HAJM_TASDIR_KABIR, MUHADHAT_JADWAL, MarjaNass,
    SijillHarf, SijillKhatt, SijillMawdiShakl, SijillMiftahShakl, SijillNass, SijillNitaq,
    SijillQayd, SijillSafha, SijillSatr, SijillTakhtit, TarwisatKhareeta, TarwisatKhatt,
    TarwisatLawha, TarwisatNusus, TarwisatQiyud, TarwisatTakhtit,
};
pub use crate::katib::{
    HuwiyatNass, Katib, KhattMabni, MUSTAWA_DAGHT, SafhaMabniya, TakhtitMabni, khatm,
};
pub use crate::khata::KhataRuqaa;
pub use crate::muhadhah::BaytMuhadhah;
pub use crate::qari::{
    BayanatQism, JadwalKhareeta, JadwalKhatt, JadwalLawha, JadwalNusus, JadwalQiyud,
    JadwalTakhtit, MalafRuqaa, Ruqaa, khareeta, khatt, lawha, miftah_min_nass, nitaqat, nusus,
    qiyud, takhtit, takhtit_wahid,
};
pub use crate::tarwisa::{
    ALAM_ILTIQAT, ALAM_MAARUFA, ALAM_MASAFA, ALAM_MIRAT, ALAM_TAGHTIYA, AQSA_AQSAM,
    AQSA_MAJMU_KHAAM, AQSA_QISM_KHAAM, HAJM_TARWISA, ISDAR_SIYAGHA, JadwalAqsam, SIHR, Tarwisa,
};
pub use crate::tawqee::{
    DawrMiftah, HAJM_KUTLA, HAJM_RISALA, ISDAR_KUTLA, Khwarizmiya, KutlatTawqee, MudaqqiqRafid,
    MudaqqiqTawqee, NITAQ_TAWQEE, SIHR_KUTLA, hajm_qism,
};

/// The file extension a patch is written with, without the dot.
///
/// Named once here because it appears in the installer's path building, the
/// registry's content-type mapping, the Studio's file dialogs and the CLI's
/// argument help, and four spellings of one extension is four chances for one
/// of them to be `.ruq`.
pub const IMTIDAD: &str = "ruqaa";

/// The media type the registry serves a patch under.
pub const NAW_MUHTAWA: &str = "application/vnd.taarib.ruqaa";
