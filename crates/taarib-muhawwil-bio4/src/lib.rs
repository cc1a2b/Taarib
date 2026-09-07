//! # محوّل تعريب لمحرّك بيو٤ — the BIO4 adapter
//!
//! *Resident Evil 4* (2005) draws its text from a baked glyph atlas. There is no
//! font file to replace and no shaper to hook: what exists is a `.fnt` per screen
//! holding a horizontal box per cell, a texture holding those cells in a fixed
//! grid, and a game that walks a string one code point at a time and blits one
//! cell per code point. Which cell each code point selects is decided by a table
//! inside `bio4.exe` — see [`kharita`], which reproduces it.
//!
//! An engine of that shape cannot join Arabic letters, because joining is not
//! something you can express in "one image per character". So the Arabic is
//! shaped **before** it ever reaches the game, by `taarib-saff`, and what the
//! game is given is a sequence of *cell references* rather than letters. That is
//! [`naql`], and it is the same argument — with the same costs, stated as
//! plainly — as `taarib_lawha::naql`.
//!
//! ## What was verified, and how
//!
//! Everything in [`tibl`], [`khatt`] and [`shabaka`] was read out of the
//! thirty-two `.fnt` files in `BIO4/Font` of a real install of the Ultimate HD
//! Edition, and the grid law in [`shabaka`] was then checked against the
//! *pixels* of a shipped atlas: `BIO4/ImagePack/13000009.pack` decodes to a
//! 1024×448 `DXT5` surface, and 548 of the 548 non-blank cells in it have their
//! ink starting and ending exactly where `report_zh-cn.fnt` says. That is the
//! measurement the whole module rests on, and it is why the fields are named
//! [`MadkhalKhana::yasar`] and [`MadkhalKhana::yameen`] and not "bearing" and
//! "advance", which is what they look like until you overlay them on a texture.
//!
//! ## Three things the file layout is not
//!
//! **The `.fnt` carries no texels.** Its embedded TPL declares a 1024-wide
//! `GX_TF_C4` image with a sixteen-entry `GX_TL_RGB5A3` palette, and points both
//! data offsets at the block's own end. Every one of the game's 616 standalone
//! `.tpl` files does the same and is 64, 116 or 168 bytes long. The texels live
//! in `BIO4/ImagePack`, named by the four bytes at that data offset — see
//! [`hizma`].
//!
//! **The `ImagePack` payload is not a TPL.** It is a `DDS` in `DXT5`, which is
//! the Ultimate HD Edition's replacement art. For several fonts it is twice the
//! dimensions the TPL declares, and for the traditional-Chinese `ss_*` fonts it
//! is a different shape altogether with metrics that no longer describe it. The
//! GameCube-format path in [`sura`] and the PC path in [`hizma`] are therefore
//! two real paths, not one with a wrapper.
//!
//! **There is no character table anywhere in these files** — it is in the
//! executable. `bio4.exe` holds one flat array of `u32` code points per language,
//! and a code point's *index* in that array is its cell. The routine that builds a
//! `std::map` out of it is at `0x006A_7F50`, the Latin array is at `0x00C0_CE18`
//! and holds 260 entries, and [`kharita`] reproduces that array, both directions
//! of the lookup, and the file offsets of the two immediates a patch would edit to
//! point the game at a longer one. Every code point in all eight shipped
//! dictionaries resolves through it. So the missing rung is no longer missing, and
//! [`naql`]'s cell numbering is the thing that now has to agree with it.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `tibl` | the GameCube TPL container, both byte orders, byte-identical round trip |
//! | `khatt` | the `.fnt` container: preamble, embedded TPL, metrics table |
//! | `shabaka` | the cell grid: pitch, columns, rows, and the law that ties them to the height |
//! | `sura` | `GX_TF_C4` texels and a `GX_TL_RGB5A3` palette, encoded and decoded |
//! | `hizma` | the `ImagePack` container and the `DDS`/`DXT5` payload the PC build samples |
//! | `naql` | the cell transport: shaped glyph ids to cell indices, with no path from a character |
//! | `kharita` | the character table `bio4.exe` carries: code point to cell, and back |
//! | `bina` | shape, rasterize through `taarib-lawha`, place into cells, emit metrics and texels |

pub mod bina;
pub mod hizma;
pub mod kharita;
pub mod khata;
pub mod khatt;
pub mod naql;
pub mod shabaka;
pub mod sura;
pub mod tibl;

pub use crate::bina::{KhattMabni, KhiyaratBina, TaqreerBina};
pub use crate::hizma::{DDS_SIHR, Hizma, HuwiyatHizma};
pub use crate::kharita::{JADAWIL, JadwalKharita, RUMUZ_LATINI};
pub use crate::khata::KhataBio4;
pub use crate::khatt::{KhattBio4, MadkhalKhana};
pub use crate::naql::{
    IzahatAlama, MiftahKhanaBio4, NaqlBio4, NassManqulBio4, NatijatNaqlBio4, TawzeeKhanatBio4,
};
pub use crate::shabaka::Shabaka;
pub use crate::sura::{LawhatAlwan, SuraMufakkaka};
pub use crate::tibl::{SighatLawn, SighatSura, TarteebBayt, Tibl};
