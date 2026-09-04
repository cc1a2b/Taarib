//! الكاتب — building a patch, byte for byte the same every time.
//!
//! The compiler in `taarib-tarqee` hands this module strings, layouts, glyph
//! images and constraints, and gets a `.ruqaa` back. Everything about where a
//! section lands, how it is compressed, what the section table says and what the
//! content hash covers is decided here and nowhere else, so that the writer and
//! [`crate::qari`] cannot drift apart into a file only one of them believes in.
//!
//! ## Why this is a builder and not a set of setters
//!
//! Four of the container's tables are keyed by a string's **index** in the
//! sorted string table, and the string table is sorted by a hash. So a string's
//! index is not known until every string has been added — which means a caller
//! that computed indices itself would be computing them from an order that does
//! not exist yet.
//!
//! [`Katib`] therefore hands out a [`HuwiyatNass`] when a string is added, takes
//! that handle everywhere an index would go, and resolves handles to indices
//! once, at [`Katib::ikhtim`], after the sort. The caller cannot produce an
//! unsorted table, cannot produce a dangling index, and cannot produce a layout
//! attached to the wrong string, because there is no API through which it could
//! express any of those.
//!
//! ## Determinism, and exactly how far it goes
//!
//! The same inputs, the same flags and the same compression level produce
//! byte-identical output. There is no timestamp in the container, no path, no
//! machine name, and nothing is ordered by a hash map's iteration: every table
//! is sorted by a key the format defines, and the string pool is built in
//! first-seen order.
//!
//! What that does *not* survive is a change of zstd version. The content hash
//! covers the stored bytes, which are the compressed bytes, so an encoder that
//! improves its ratio changes the hash of a patch built from identical inputs.
//! This is stated rather than worked around, because the alternative — hashing
//! the uncompressed bytes — would mean a verifier had to decompress a section
//! before it could decide whether to trust it, and expanding untrusted input is
//! the one thing the reader's validation order exists to prevent.
//!
//! The consequence for the registry is small and worth being clear about: a
//! signature is over a built artifact, not over a recipe. Rebuilding a patch on
//! a different toolchain produces a patch that must be signed again.
//!
//! ## The signature reservation
//!
//! Every container this module writes ends with a signature section of exactly
//! [`crate::tawqee::HAJM_KUTLA`] bytes, and it is written unsigned. That is not
//! an omission: this crate cannot sign, because signing requires the owner's
//! private key, and a key reachable from the compiler is a key that leaks with
//! it. [`khatm`] fills the reservation in place afterwards, which works precisely
//! because the block sits outside the content hash — sealing a patch moves no
//! offset and recomputes nothing.
//!
//! A patch that reaches a player unsigned is refused by the installer. A patch
//! that reaches this module unsigned is normal, and is what the compiler
//! produces on every build.

use std::collections::HashMap;

use bytemuck::Pod;

use crate::aqsam::{HAJM_MADKHAL, MUHADHAT_QISM, MadkhalQism, NawDaght, NawQism};
use crate::jadawil::{
    HAJM_TASDIR, HAJM_TASDIR_KABIR, HAJM_TASDIR_KABIR_U32, HAJM_TASDIR_U32, MarjaNass, SijillHarf,
    SijillKhatt, SijillMawdiShakl, SijillMiftahShakl, SijillNass, SijillNitaq, SijillQayd,
    SijillSafha, SijillSatr, SijillTakhtit, TarwisatKhareeta, TarwisatKhatt, TarwisatLawha,
    TarwisatNusus, TarwisatQiyud, TarwisatTakhtit,
};
use crate::khata::KhataRuqaa;
use crate::muhadhah::BaytMuhadhah;
use crate::qari::miftah_min_nass;
use crate::tarwisa::{
    ALAM_ILTIQAT, ALAM_MASAFA, ALAM_MIRAT, ALAM_TAGHTIYA, AQSA_AQSAM, AQSA_MAJMU_KHAAM,
    AQSA_QISM_KHAAM, HAJM_TARWISA, ISDAR_SIYAGHA, JadwalAqsam, Tarwisa, hajm_usize, sittasi,
    tul_u64,
};
use crate::tawqee::{DawrMiftah, HAJM_KUTLA, KutlatTawqee, MudaqqiqTawqee};

/// The compression level the compiler uses unless it is told otherwise.
///
/// Nineteen rather than the encoder's default of three. A patch is compressed
/// once, on a build machine, and then downloaded by every player who installs
/// it and decompressed once per launch — the asymmetry is enormous, and zstd's
/// decompression speed barely moves with the level while its ratio does. Level
/// twenty and above are the long-window modes, which cost the *reader* memory,
/// and the reader here is a game.
pub const MUSTAWA_DAGHT: i32 = 19;

/// A string's handle while the patch is being built.
///
/// Not an index into anything, and deliberately not convertible to one: the
/// index a string will have is decided by the sort at [`Katib::ikhtim`], and a
/// number handed to the caller before then would be a number that stops being
/// true. Opaque, `Copy`, and cheap to carry around a compiler's own data
/// structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HuwiyatNass(u32);

/// One precomputed layout, as the compiler produces it.
///
/// The glyphs and lines are owned here rather than referenced because the writer
/// concatenates every layout's glyphs into one array and rewrites each head's
/// start index accordingly — which it can only do once it has them all.
#[derive(Debug, Clone)]
pub struct TakhtitMabni {
    /// The string this laid out.
    pub nass: HuwiyatNass,
    /// The size it was laid out at, in quarter pixels.
    pub hajm_rubi: u16,
    /// The width of the widest line, in pixels.
    pub ard: f32,
    /// The total height of every line box, in pixels.
    pub irtifa: f32,
    /// `ALAM_TAKHTIT_*`.
    pub alam: u16,
    /// The glyphs, in visual order.
    pub huruf: Vec<SijillHarf>,
    /// The lines.
    pub sutur: Vec<SijillSatr>,
}

/// One atlas page, as the packer produces it.
#[derive(Debug, Clone)]
pub struct SafhaMabniya {
    /// Width in texels.
    pub ard: u16,
    /// Height in texels.
    pub irtifa: u16,
    /// The texels: one byte each, row-major from the top, no row padding.
    pub bayt: Vec<u8>,
}

/// One font of the chain, as the font resolver produces it.
#[derive(Debug, Clone)]
pub struct KhattMabni {
    /// The file name the installer will place beside the plugin.
    pub ism: String,
    /// The file's BLAKE3 content hash, so a font swapped after installation is
    /// caught rather than shaped with.
    pub basma: [u8; 32],
    /// `ALAM_KHATT_*`.
    pub alam: u16,
}

/// The writer.
#[derive(Debug)]
pub struct Katib {
    alam: u16,
    mustawa: i32,
    bayan: Option<Vec<u8>>,
    mafatih: Vec<u64>,
    tarajim: Vec<String>,
    marajie: HashMap<u64, HuwiyatNass>,
    nitaqat: Vec<(HuwiyatNass, SijillNitaq)>,
    takhtitat: Vec<TakhtitMabni>,
    qiyud: Vec<(HuwiyatNass, SijillQayd)>,
    ashkal: Vec<(SijillMiftahShakl, SijillMawdiShakl)>,
    safahat: Vec<SafhaMabniya>,
    khutut: Vec<KhattMabni>,
}

impl Default for Katib {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl Katib {
    /// An empty writer.
    #[must_use]
    pub fn jadeed() -> Self {
        Self {
            alam: 0,
            mustawa: MUSTAWA_DAGHT,
            bayan: None,
            mafatih: Vec::new(),
            tarajim: Vec::new(),
            marajie: HashMap::new(),
            nitaqat: Vec::new(),
            takhtitat: Vec::new(),
            qiyud: Vec::new(),
            ashkal: Vec::new(),
            safahat: Vec::new(),
            khutut: Vec::new(),
        }
    }

    /// Sets the compression level.
    #[must_use]
    pub const fn bi_mustawa(mut self, mustawa: i32) -> Self {
        self.mustawa = mustawa;
        self
    }

    /// Declares the atlas as eight-bit coverage pages.
    #[must_use]
    pub const fn bi_taghtiya(mut self) -> Self {
        self.alam |= ALAM_TAGHTIYA;
        self
    }

    /// Declares the atlas as signed distance field pages.
    #[must_use]
    pub const fn bi_masafa(mut self) -> Self {
        self.alam |= ALAM_MASAFA;
        self
    }

    /// Declares that the constraints carry right-to-left mirroring hints.
    #[must_use]
    pub const fn bi_mirat(mut self) -> Self {
        self.alam |= ALAM_MIRAT;
        self
    }

    /// Declares that the constraints were informed by runtime capture.
    #[must_use]
    pub const fn bi_iltiqat(mut self) -> Self {
        self.alam |= ALAM_ILTIQAT;
        self
    }

    /// The metadata section, as the JSON bytes `taarib-mustalahat` produced.
    pub fn bayan(&mut self, jeyson: &[u8]) -> &mut Self {
        self.bayan = Some(jeyson.to_vec());
        self
    }

    /// Adds a translated string and returns its handle.
    ///
    /// Adding the same source twice returns the same handle and keeps the first
    /// translation. That is not a convenience: two rows with one key would make
    /// the container's lookup answer depend on which one a binary search landed
    /// on, and the reader refuses it. Resolving it here, where the second
    /// translation is still visible, is the only place a compiler can be told
    /// which string collided.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::SijillGhayrMutabaq`] when the same source arrives with a
    /// *different* translation, because that is a real disagreement in the
    /// project rather than a duplicate entry, and silently keeping one of them
    /// would ship a patch that contradicts the project it came from.
    ///
    /// [`KhataRuqaa::JadwalTalif`] when the table would exceed what a `u32`
    /// index can name.
    pub fn nass(&mut self, asl: &str, tarjama: &str) -> Result<HuwiyatNass, KhataRuqaa> {
        let miftah = miftah_min_nass(asl);
        if let Some(huwiya) = self.marajie.get(&miftah).copied() {
            let sabiqa = self.tarajim.get(usize::try_from(huwiya.0).unwrap_or(usize::MAX));
            if sabiqa.is_some_and(|sabiqa| sabiqa == tarjama) {
                return Ok(huwiya);
            }
            return Err(KhataRuqaa::SijillGhayrMutabaq {
                naw: NawQism::Nusus.raqm(),
                haql: "two different translations for one source string",
                wujid: u32::try_from(tarjama.len()).unwrap_or(u32::MAX),
                muntazar: sabiqa.map_or(0, |s| u32::try_from(s.len()).unwrap_or(u32::MAX)),
            });
        }

        let raqm = u32::try_from(self.mafatih.len()).map_err(|_| KhataRuqaa::JadwalTalif {
            naw: NawQism::Nusus.raqm(),
            haql: "adad_nusus",
            qeema: tul_u64(self.mafatih.len()),
            hadd: u64::from(u32::MAX),
        })?;
        let huwiya = HuwiyatNass(raqm);
        self.mafatih.push(miftah);
        self.tarajim.push(tarjama.to_owned());
        self.marajie.insert(miftah, huwiya);
        Ok(huwiya)
    }

    /// Adds a style span over a string's translated text.
    ///
    /// The span's `nass` field is ignored and overwritten at [`Katib::ikhtim`];
    /// the handle is what binds it. A field the caller cannot usefully set is
    /// still present on the record because the record's layout is frozen.
    pub fn nitaq(&mut self, nass: HuwiyatNass, nitaq: SijillNitaq) -> &mut Self {
        self.nitaqat.push((nass, nitaq));
        self
    }

    /// Adds a precomputed layout.
    pub fn takhtit(&mut self, takhtit: TakhtitMabni) -> &mut Self {
        self.takhtitat.push(takhtit);
        self
    }

    /// Adds the constraint recorded for one string.
    ///
    /// The record's `nass` field is ignored and overwritten at
    /// [`Katib::ikhtim`], as with [`Katib::nitaq`].
    pub fn qayd(&mut self, nass: HuwiyatNass, qayd: SijillQayd) -> &mut Self {
        self.qiyud.push((nass, qayd));
        self
    }

    /// Adds one rasterized glyph image to the map.
    pub fn shakl(&mut self, miftah: SijillMiftahShakl, mawdi: SijillMawdiShakl) -> &mut Self {
        self.ashkal.push((miftah, mawdi));
        self
    }

    /// Adds one atlas page.
    pub fn safha(&mut self, safha: SafhaMabniya) -> &mut Self {
        self.safahat.push(safha);
        self
    }

    /// Adds one font to the chain, at the next position.
    ///
    /// The position is the order fonts are added in, and it is what
    /// [`SijillHarf::khatt`] indexes, so the compiler must add them in the order
    /// it shaped with.
    pub fn khatt(&mut self, khatt: KhattMabni) -> &mut Self {
        self.khutut.push(khatt);
        self
    }

    /// Assembles the container.
    ///
    /// Sorts every table, resolves every handle to the index the sort produced,
    /// builds each section, compresses what benefits from it, writes the header
    /// and the section table, reserves the signature block, and hashes the bytes
    /// the signature will commit to. The result is a complete, readable,
    /// unsigned patch: [`crate::qari::Ruqaa::iftah`] accepts it, and an installer
    /// refuses it until [`khatm`] has sealed it.
    ///
    /// Returned as a [`BaytMuhadhah`] rather than a `Vec<u8>` so that the
    /// compiler can read back what it just wrote — verifying its own output is
    /// the last thing `taarib-tarqee` does — without the tables failing to cast
    /// on an allocation that happened to land on an odd address.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::QismMafqud`] when the metadata section was never given,
    /// [`KhataRuqaa::NamatGhayrMuarraf`] when pages are present but neither
    /// rasterization mode was declared, [`KhataRuqaa::SijillGhayrMutabaq`] on a
    /// duplicate glyph key, [`KhataRuqaa::HajmKhaamMufrit`] or
    /// [`KhataRuqaa::MajmuKhaamMufrit`] when a section or the whole container
    /// exceeds what the reader will accept, and [`KhataRuqaa::DaghtFashil`] when
    /// the encoder refuses a section.
    pub fn ikhtim(&self) -> Result<BaytMuhadhah, KhataRuqaa> {
        let bayan = self
            .bayan
            .as_deref()
            .ok_or(KhataRuqaa::QismMafqud { ism: NawQism::Bayan.ism() })?;
        if !self.safahat.is_empty() && self.alam & (ALAM_TAGHTIYA | ALAM_MASAFA) == 0 {
            return Err(KhataRuqaa::NamatGhayrMuarraf);
        }

        let fahras = self.rattib()?;
        let mut khaam: Vec<(NawQism, Vec<u8>)> = Vec::with_capacity(7);
        khaam.push((NawQism::Bayan, bayan.to_vec()));
        khaam.push((NawQism::Nusus, self.ibni_nusus(&fahras)?));
        if !self.takhtitat.is_empty() {
            khaam.push((NawQism::Takhtit, self.ibni_takhtit(&fahras)?));
        }
        if !self.safahat.is_empty() {
            khaam.push((NawQism::Lawha, self.ibni_lawha()?));
        }
        if !self.ashkal.is_empty() {
            khaam.push((NawQism::Khareeta, self.ibni_khareeta()?));
        }
        if !self.khutut.is_empty() {
            khaam.push((NawQism::Khatt, self.ibni_khatt()?));
        }
        if !self.qiyud.is_empty() {
            khaam.push((NawQism::Qiyud, self.ibni_qiyud(&fahras)?));
        }
        khaam.sort_by_key(|(naw, _)| naw.raqm());

        self.urkub(khaam)
    }

    /// The index every string will have once the table is sorted.
    ///
    /// Returned as a lookup from handle to index rather than applied in place,
    /// because four tables need it and each of them is built separately.
    fn rattib(&self) -> Result<Vec<u32>, KhataRuqaa> {
        let adad = self.mafatih.len();
        let mut tarteeb: Vec<u32> = (0..u32::try_from(adad).unwrap_or(u32::MAX)).collect();
        tarteeb.sort_by_key(|raqm| {
            self.mafatih.get(usize::try_from(*raqm).unwrap_or(usize::MAX)).copied().unwrap_or(0)
        });

        // A duplicate key survives to here only if two different source strings
        // hashed to the same sixty-four bits. It is astronomically unlikely and
        // it is still refused, because the alternative is a patch where one of
        // the two strings is unreachable and nothing says which.
        for nafidha in tarteeb.windows(2) {
            let (Some(awwal), Some(thani)) = (nafidha.first(), nafidha.get(1)) else { continue };
            let a = self.miftah_raqm(*awwal);
            let b = self.miftah_raqm(*thani);
            if a == b {
                return Err(KhataRuqaa::SijillGhayrMutabaq {
                    naw: NawQism::Nusus.raqm(),
                    haql: "two source strings share a 64-bit key",
                    wujid: *awwal,
                    muntazar: *thani,
                });
            }
        }

        let mut fahras = vec![0u32; adad];
        for (makan, raqm) in tarteeb.iter().enumerate() {
            if let Some(khana) = fahras.get_mut(usize::try_from(*raqm).unwrap_or(usize::MAX)) {
                *khana = u32::try_from(makan).unwrap_or(u32::MAX);
            }
        }
        Ok(fahras)
    }

    /// One string's key, by handle number.
    fn miftah_raqm(&self, raqm: u32) -> u64 {
        self.mafatih.get(usize::try_from(raqm).unwrap_or(usize::MAX)).copied().unwrap_or(0)
    }

    /// Resolves a handle to its final index.
    fn hall(fahras: &[u32], huwiya: HuwiyatNass) -> Result<u32, KhataRuqaa> {
        fahras.get(usize::try_from(huwiya.0).unwrap_or(usize::MAX)).copied().ok_or_else(|| {
            KhataRuqaa::FahrasKharij {
                haql: "string handle",
                fahras: huwiya.0,
                adad: u32::try_from(fahras.len()).unwrap_or(u32::MAX),
            }
        })
    }

    fn ibni_nusus(&self, fahras: &[u32]) -> Result<Vec<u8>, KhataRuqaa> {
        let naw = NawQism::Nusus;
        let adad = self.mafatih.len();

        let mut sijillat = vec![SijillNass::default(); adad];
        let mut hawd: Vec<u8> = Vec::new();
        let mut mudkhal: HashMap<&str, MarjaNass> = HashMap::new();
        for raqm in 0..u32::try_from(adad).unwrap_or(u32::MAX) {
            let makan = usize::try_from(Self::hall(fahras, HuwiyatNass(raqm))?)
                .unwrap_or(usize::MAX);
            let tarjama = self
                .tarajim
                .get(usize::try_from(raqm).unwrap_or(usize::MAX))
                .map_or("", String::as_str);
            let marja = adkhil_hawd(naw, &mut hawd, &mut mudkhal, tarjama)?;
            if let Some(khana) = sijillat.get_mut(makan) {
                *khana = SijillNass { miftah: self.miftah_raqm(raqm), nass: marja };
            }
        }

        let mut nitaqat: Vec<SijillNitaq> = Vec::with_capacity(self.nitaqat.len());
        for (huwiya, nitaq) in &self.nitaqat {
            nitaqat.push(SijillNitaq { nass: Self::hall(fahras, *huwiya)?, ..*nitaq });
        }
        nitaqat.sort_by_key(|nitaq| (nitaq.nass, nitaq.bidaya, nitaq.id));

        let adad_nusus = adad_u32(naw, "adad_nusus", sijillat.len())?;
        let adad_nitaqat = adad_u32(naw, "adad_nitaqat", nitaqat.len())?;
        let izahat_nusus = HAJM_TASDIR_KABIR_U32;
        let izahat_nitaqat = baad(naw, izahat_nusus, adad_nusus, size_of::<SijillNass>())?;
        let izahat_hawd = baad(naw, izahat_nitaqat, adad_nitaqat, size_of::<SijillNitaq>())?;
        let tul_hawd = adad_u32(naw, "tul_hawd", hawd.len())?;

        let tasdir = TarwisatNusus {
            adad_nusus,
            adad_nitaqat,
            izahat_nusus,
            izahat_nitaqat,
            izahat_hawd,
            tul_hawd,
            mahjuz: 0,
        };
        let mut jism = Vec::with_capacity(HAJM_TASDIR_KABIR);
        udfu(&mut jism, naw, bytemuck::bytes_of(&tasdir))?;
        udfu_sijillat(&mut jism, naw, &sijillat)?;
        udfu_sijillat(&mut jism, naw, &nitaqat)?;
        udfu(&mut jism, naw, &hawd)?;
        Ok(jism)
    }

    fn ibni_takhtit(&self, fahras: &[u32]) -> Result<Vec<u8>, KhataRuqaa> {
        let naw = NawQism::Takhtit;

        let mut ruus: Vec<SijillTakhtit> = Vec::with_capacity(self.takhtitat.len());
        let mut huruf: Vec<SijillHarf> = Vec::new();
        let mut sutur: Vec<SijillSatr> = Vec::new();
        let mut murattaba: Vec<(u32, u16, usize)> = Vec::with_capacity(self.takhtitat.len());
        for (makan, takhtit) in self.takhtitat.iter().enumerate() {
            murattaba.push((Self::hall(fahras, takhtit.nass)?, takhtit.hajm_rubi, makan));
        }
        murattaba.sort_by_key(|(nass, hajm_rubi, _)| (*nass, *hajm_rubi));

        for nafidha in murattaba.windows(2) {
            let (Some(awwal), Some(thani)) = (nafidha.first(), nafidha.get(1)) else { continue };
            if (awwal.0, awwal.1) == (thani.0, thani.1) {
                return Err(KhataRuqaa::SijillGhayrMutabaq {
                    naw: naw.raqm(),
                    haql: "two layouts for one string at one size",
                    wujid: awwal.0,
                    muntazar: u32::from(awwal.1),
                });
            }
        }

        for (nass, hajm_rubi, makan) in murattaba {
            let Some(takhtit) = self.takhtitat.get(makan) else { continue };
            let awwal_harf = adad_u32(naw, "awwal_harf", huruf.len())?;
            let awwal_satr = adad_u32(naw, "awwal_satr", sutur.len())?;
            huruf.extend_from_slice(&takhtit.huruf);
            sutur.extend_from_slice(&takhtit.sutur);
            ruus.push(SijillTakhtit {
                nass,
                awwal_harf,
                adad_huruf: adad_u32(naw, "adad_huruf", takhtit.huruf.len())?,
                awwal_satr,
                adad_sutur: adad_u32(naw, "adad_sutur", takhtit.sutur.len())?,
                ard: takhtit.ard,
                irtifa: takhtit.irtifa,
                hajm_rubi,
                alam: takhtit.alam,
            });
        }

        let adad_takhtitat = adad_u32(naw, "adad_takhtitat", ruus.len())?;
        let adad_huruf = adad_u32(naw, "adad_huruf", huruf.len())?;
        let adad_sutur = adad_u32(naw, "adad_sutur", sutur.len())?;
        let izahat_takhtitat = HAJM_TASDIR_KABIR_U32;
        let izahat_huruf =
            baad(naw, izahat_takhtitat, adad_takhtitat, size_of::<SijillTakhtit>())?;
        let izahat_sutur = baad(naw, izahat_huruf, adad_huruf, size_of::<SijillHarf>())?;

        let tasdir = TarwisatTakhtit {
            adad_takhtitat,
            izahat_takhtitat,
            adad_huruf,
            izahat_huruf,
            adad_sutur,
            izahat_sutur,
            mahjuz: 0,
        };
        let mut jism = Vec::with_capacity(HAJM_TASDIR_KABIR);
        udfu(&mut jism, naw, bytemuck::bytes_of(&tasdir))?;
        udfu_sijillat(&mut jism, naw, &ruus)?;
        udfu_sijillat(&mut jism, naw, &huruf)?;
        udfu_sijillat(&mut jism, naw, &sutur)?;
        Ok(jism)
    }

    fn ibni_khareeta(&self) -> Result<Vec<u8>, KhataRuqaa> {
        let naw = NawQism::Khareeta;
        let mut murattaba = self.ashkal.clone();
        murattaba.sort_by_key(|(miftah, _)| miftah.raqm());
        for nafidha in murattaba.windows(2) {
            let (Some(awwal), Some(thani)) = (nafidha.first(), nafidha.get(1)) else { continue };
            if awwal.0.raqm() == thani.0.raqm() {
                return Err(KhataRuqaa::SijillGhayrMutabaq {
                    naw: naw.raqm(),
                    haql: "two atlas entries for one glyph, size, font and bucket",
                    wujid: awwal.0.muarrif,
                    muntazar: u32::from(awwal.0.hajm_rubi),
                });
            }
        }

        let mafatih: Vec<SijillMiftahShakl> =
            murattaba.iter().map(|(miftah, _)| *miftah).collect();
        let mawadi: Vec<SijillMawdiShakl> = murattaba.iter().map(|(_, mawdi)| *mawdi).collect();

        let adad = adad_u32(naw, "adad", mafatih.len())?;
        let izahat_mafatih = HAJM_TASDIR_U32;
        let izahat_mawadi = baad(naw, izahat_mafatih, adad, size_of::<SijillMiftahShakl>())?;
        let tasdir = TarwisatKhareeta { adad, izahat_mafatih, izahat_mawadi, mahjuz: 0 };

        let mut jism = Vec::with_capacity(HAJM_TASDIR);
        udfu(&mut jism, naw, bytemuck::bytes_of(&tasdir))?;
        udfu_sijillat(&mut jism, naw, &mafatih)?;
        udfu_sijillat(&mut jism, naw, &mawadi)?;
        Ok(jism)
    }

    fn ibni_qiyud(&self, fahras: &[u32]) -> Result<Vec<u8>, KhataRuqaa> {
        let naw = NawQism::Qiyud;
        let mut sijillat: Vec<SijillQayd> = Vec::with_capacity(self.qiyud.len());
        for (huwiya, qayd) in &self.qiyud {
            sijillat.push(SijillQayd { nass: Self::hall(fahras, *huwiya)?, ..*qayd });
        }
        sijillat.sort_by_key(|qayd| qayd.nass);
        for nafidha in sijillat.windows(2) {
            let (Some(awwal), Some(thani)) = (nafidha.first(), nafidha.get(1)) else { continue };
            if awwal.nass == thani.nass {
                return Err(KhataRuqaa::SijillGhayrMutabaq {
                    naw: naw.raqm(),
                    haql: "two constraints for one string",
                    wujid: awwal.nass,
                    muntazar: thani.nass,
                });
            }
        }

        let adad = adad_u32(naw, "adad", sijillat.len())?;
        let tasdir = TarwisatQiyud { adad, izaha: HAJM_TASDIR_U32, mahjuz: 0 };
        let mut jism = Vec::with_capacity(HAJM_TASDIR);
        udfu(&mut jism, naw, bytemuck::bytes_of(&tasdir))?;
        udfu_sijillat(&mut jism, naw, &sijillat)?;
        Ok(jism)
    }

    fn ibni_lawha(&self) -> Result<Vec<u8>, KhataRuqaa> {
        let naw = NawQism::Lawha;
        let adad_safahat = adad_u32(naw, "adad_safahat", self.safahat.len())?;
        let izahat_safahat = HAJM_TASDIR_U32;
        let mut izaha = baad(naw, izahat_safahat, adad_safahat, size_of::<SijillSafha>())?;

        let mut sijillat: Vec<SijillSafha> = Vec::with_capacity(self.safahat.len());
        for safha in &self.safahat {
            let madum = u32::from(safha.ard).checked_mul(u32::from(safha.irtifa)).ok_or(
                KhataRuqaa::SafhaTalifa {
                    safha: adad_u32(naw, "safha", sijillat.len())?,
                    izaha: u64::from(izaha),
                    nihaya: u64::from(izaha),
                    tul: tul_u64(safha.bayt.len()),
                },
            )?;
            let tul = adad_u32(naw, "tul", safha.bayt.len())?;
            if madum != tul {
                return Err(KhataRuqaa::SafhaTalifa {
                    safha: adad_u32(naw, "safha", sijillat.len())?,
                    izaha: u64::from(izaha),
                    nihaya: u64::from(izaha).saturating_add(u64::from(tul)),
                    tul: u64::from(madum),
                });
            }
            sijillat.push(SijillSafha {
                izaha,
                tul,
                ard: safha.ard,
                irtifa: safha.irtifa,
                hashw: 0,
            });
            izaha = izaha.checked_add(tul).ok_or_else(|| KhataRuqaa::HajmKhaamMufrit {
                naw: naw.raqm(),
                muallan: u64::MAX,
                saqf: AQSA_QISM_KHAAM,
            })?;
        }

        let tasdir = TarwisatLawha { adad_safahat, izahat_safahat, mahjuz: 0 };
        let mut jism = Vec::with_capacity(HAJM_TASDIR);
        udfu(&mut jism, naw, bytemuck::bytes_of(&tasdir))?;
        udfu_sijillat(&mut jism, naw, &sijillat)?;
        for safha in &self.safahat {
            udfu(&mut jism, naw, &safha.bayt)?;
        }
        Ok(jism)
    }

    fn ibni_khatt(&self) -> Result<Vec<u8>, KhataRuqaa> {
        let naw = NawQism::Khatt;
        let adad_khutut = adad_u32(naw, "adad_khutut", self.khutut.len())?;
        let izahat_khutut = HAJM_TASDIR_U32;
        // The hashes sit between the records and the name pool: fixed size, so
        // their offsets are arithmetic rather than a second pool to bound.
        let izahat_basmat = baad(naw, izahat_khutut, adad_khutut, size_of::<SijillKhatt>())?;
        let izahat_hawd = baad(naw, izahat_basmat, adad_khutut, 32)?;

        let mut hawd: Vec<u8> = Vec::new();
        let mut mudkhal: HashMap<&str, MarjaNass> = HashMap::new();
        let mut sijillat: Vec<SijillKhatt> = Vec::with_capacity(self.khutut.len());
        for (makan, khatt) in self.khutut.iter().enumerate() {
            let raqm = adad_u32(naw, "fahras", makan)?;
            let fahras = u16::try_from(raqm).map_err(|_| KhataRuqaa::JadwalTalif {
                naw: naw.raqm(),
                haql: "fahras",
                qeema: u64::from(raqm),
                hadd: u64::from(u16::MAX),
            })?;
            let ism = adkhil_hawd(naw, &mut hawd, &mut mudkhal, &khatt.ism)?;
            let izahat_basma = izahat_basmat
                .checked_add(raqm.saturating_mul(32))
                .ok_or_else(|| KhataRuqaa::JadwalTalif {
                    naw: naw.raqm(),
                    haql: "izahat_basma",
                    qeema: u64::from(izahat_basmat),
                    hadd: u64::from(u32::MAX),
                })?;
            sijillat.push(SijillKhatt { ism, izahat_basma, fahras, alam: khatt.alam });
        }
        // The pool's offset was computed before its contents existed, which is
        // fine because both arrays before it are fixed size — but it has to be
        // re-checked against what the names actually came to.
        let tul_hawd = adad_u32(naw, "tul_hawd", hawd.len())?;

        let tasdir = TarwisatKhatt { adad_khutut, izahat_khutut, izahat_hawd, tul_hawd };
        let mut jism = Vec::with_capacity(HAJM_TASDIR);
        udfu(&mut jism, naw, bytemuck::bytes_of(&tasdir))?;
        udfu_sijillat(&mut jism, naw, &sijillat)?;
        for khatt in &self.khutut {
            udfu(&mut jism, naw, &khatt.basma)?;
        }
        udfu(&mut jism, naw, &hawd)?;
        Ok(jism)
    }

    /// Compresses a section, unless compressing it would not pay.
    ///
    /// A section is stored compressed only when the compressed form is actually
    /// smaller. Incompressible input — an atlas of distance-field pages is close
    /// to it — otherwise pays for a zstd frame header and a decompression pass
    /// at every launch in exchange for nothing.
    fn idghat(&self, naw: NawQism, khaam: &[u8]) -> Result<(NawDaght, Vec<u8>), KhataRuqaa> {
        if naw.bila_daght() || khaam.is_empty() {
            return Ok((NawDaght::Bila, khaam.to_vec()));
        }
        let madghut = zstd::bulk::compress(khaam, self.mustawa).map_err(|khata| {
            KhataRuqaa::DaghtFashil { naw: naw.raqm(), tafsil: khata.to_string() }
        })?;
        if madghut.len() < khaam.len() {
            Ok((NawDaght::Zstd, madghut))
        } else {
            Ok((NawDaght::Bila, khaam.to_vec()))
        }
    }

    /// Lays the sections out, writes the framing, and hashes the result.
    fn urkub(&self, khaam: Vec<(NawQism, Vec<u8>)>) -> Result<BaytMuhadhah, KhataRuqaa> {
        let mut makhzuna: Vec<(NawQism, NawDaght, Vec<u8>, u64)> = Vec::with_capacity(8);
        let mut majmu_khaam: u64 = 0;
        for (naw, bayt) in khaam {
            let tul_khaam = tul_u64(bayt.len());
            if tul_khaam > AQSA_QISM_KHAAM {
                return Err(KhataRuqaa::HajmKhaamMufrit {
                    naw: naw.raqm(),
                    muallan: tul_khaam,
                    saqf: AQSA_QISM_KHAAM,
                });
            }
            majmu_khaam = majmu_khaam.checked_add(tul_khaam).ok_or(mufrit_majmu())?;
            let (daght, makhzun) = self.idghat(naw, &bayt)?;
            makhzuna.push((naw, daght, makhzun, tul_khaam));
        }

        // The signature block counts toward the total the reader checks, so it
        // is added here rather than being quietly exempt. A ceiling the writer
        // and the reader compute differently is a ceiling that eventually lets
        // through a file one of them refuses.
        majmu_khaam = majmu_khaam.checked_add(tul_u64(HAJM_KUTLA)).ok_or(mufrit_majmu())?;
        if majmu_khaam > AQSA_MAJMU_KHAAM {
            return Err(KhataRuqaa::MajmuKhaamMufrit {
                majmu: majmu_khaam,
                saqf: AQSA_MAJMU_KHAAM,
            });
        }

        let adad = makhzuna.len().saturating_add(1);
        let adad_aqsam = u32::try_from(adad)
            .ok()
            .filter(|adad| *adad <= AQSA_AQSAM)
            .ok_or(KhataRuqaa::AdadAqsamGhayrSalih { adad: u32::MAX, aqsa: AQSA_AQSAM })?;
        let mut izaha = tul_u64(HAJM_TARWISA)
            .checked_add(u64::from(adad_aqsam).saturating_mul(tul_u64(HAJM_MADKHAL)))
            .ok_or(mufrit_majmu())?;

        let mut madkhalat: Vec<MadkhalQism> = Vec::with_capacity(adad);
        let mut jism: Vec<(u64, Vec<u8>)> = Vec::with_capacity(adad);
        for (naw, daght, bayt, tul_khaam) in makhzuna {
            izaha = muhadhah(izaha)?;
            let tul_makhzun = tul_u64(bayt.len());
            madkhalat.push(MadkhalQism { naw, izaha, tul_makhzun, tul_khaam, daght });
            jism.push((izaha, bayt));
            izaha = izaha.checked_add(tul_makhzun).ok_or(mufrit_majmu())?;
        }

        let izahat_tawqee = muhadhah(izaha)?;
        madkhalat.push(MadkhalQism {
            naw: NawQism::Tawqee,
            izaha: izahat_tawqee,
            tul_makhzun: tul_u64(HAJM_KUTLA),
            tul_khaam: tul_u64(HAJM_KUTLA),
            daght: NawDaght::Bila,
        });
        let hajm_kulli = izahat_tawqee.checked_add(tul_u64(HAJM_KUTLA)).ok_or(mufrit_majmu())?;
        let siaa = hajm_usize(hajm_kulli)
            .ok_or(KhataRuqaa::MajmuKhaamMufrit { majmu: hajm_kulli, saqf: AQSA_MAJMU_KHAAM })?;

        let mut muhadhah_malaf = BaytMuhadhah::sifr(siaa);
        let malaf = muhadhah_malaf.bayt_mut();

        for (fahras, madkhal) in madkhalat.iter().enumerate() {
            let bidaya = HAJM_TARWISA.saturating_add(fahras.saturating_mul(HAJM_MADKHAL));
            let nihaya = bidaya.saturating_add(HAJM_MADKHAL);
            let nafidha = malaf.get_mut(bidaya..nihaya).ok_or_else(|| KhataRuqaa::MalafQaseer {
                haql: "the section table",
                tul: hajm_kulli,
                matlub: tul_u64(nihaya),
            })?;
            madkhal.ila_bayt(nafidha)?;
        }

        for (izaha, bayt) in jism {
            unsakh(malaf, izaha, &bayt, hajm_kulli)?;
        }

        // The reservation is written under the contributor role, not the
        // owner's. Nothing turns on it — `Khwarizmiya::Ghayr` is what makes
        // `KutlatTawqee::tahaqquq` refuse the block, whatever the role says, and
        // the sealer overwrites all one hundred and twenty-eight bytes including
        // this one. It is the less privileged of the two values, and a byte
        // claiming ownership that nobody put there is not a byte worth writing.
        let mut hajz = [0u8; HAJM_KUTLA];
        KutlatTawqee::hajz(DawrMiftah::Musahim).ila_bayt(&mut hajz);
        unsakh(malaf, izahat_tawqee, &hajz, hajm_kulli)?;

        let nihayat_muhtawa = hajm_usize(izahat_tawqee).ok_or(KhataRuqaa::MalafQaseer {
            haql: "the signature block",
            tul: hajm_kulli,
            matlub: izahat_tawqee,
        })?;
        let muhtawa = malaf.get(HAJM_TARWISA..nihayat_muhtawa).ok_or(KhataRuqaa::MalafQaseer {
            haql: "the hashed content",
            tul: hajm_kulli,
            matlub: izahat_tawqee,
        })?;
        let basma = *blake3::hash(muhtawa).as_bytes();

        let tarwisa = Tarwisa {
            isdar: ISDAR_SIYAGHA,
            alam: self.alam,
            adad_aqsam,
            hajm_kulli,
            basma,
        };
        let ras = malaf.get_mut(..HAJM_TARWISA).ok_or_else(|| KhataRuqaa::MalafQaseer {
            haql: "the header",
            tul: hajm_kulli,
            matlub: tul_u64(HAJM_TARWISA),
        })?;
        tarwisa.ila_bayt(ras)?;

        Ok(muhadhah_malaf)
    }
}

/// Fills a built container's signature reservation, in place.
///
/// This is what `taarib-khatm` calls once the owner's key has produced a block.
///
/// The block is verified against *this* container's hash before a byte of it is
/// written. That check is the point of the function: a signature is over the
/// ninety-seven byte message [`KutlatTawqee::risala`] builds, which carries the
/// content hash, so a block made for one patch will not verify against another
/// — and without this check, pasting it in would produce a file that looks
/// sealed to anything that only glances at the header and is refused by every
/// client that actually verifies. Failing here, on the machine that holds the
/// key, is the only place that failure is cheap.
///
/// Nothing outside the reservation is touched, so the hash the block commits to
/// is still the hash of what is there when the function returns.
///
/// # Errors
///
/// Whatever the header or the section table refuses,
/// [`KhataRuqaa::BasmaGhayrMutabaqa`] when the container's stored hash is not
/// the hash of its own content, and [`KhataRuqaa::GhayrMuwaqqaa`] or
/// [`KhataRuqaa::TawqeeGhayrSalih`] when the block does not seal this container.
pub fn khatm(
    malaf: &mut [u8],
    kutla: &KutlatTawqee,
    mudaqqiq: &dyn MudaqqiqTawqee,
) -> Result<(), KhataRuqaa> {
    let tarwisa = Tarwisa::min_bayt(malaf)?;
    let jadwal = JadwalAqsam::min_bayt(malaf, &tarwisa)?;
    let madkhal = jadwal
        .qism(NawQism::Tawqee)
        .ok_or(KhataRuqaa::QismMafqud { ism: NawQism::Tawqee.ism() })?;

    let nihayat_muhtawa =
        hajm_usize(jadwal.nihayat_muhtawa()).ok_or(KhataRuqaa::KutlatTawqeeTalifa {
            haql: "the block's offset does not fit this platform's address space",
        })?;
    let tul_malaf = tul_u64(malaf.len());
    let muhtawa = malaf.get(HAJM_TARWISA..nihayat_muhtawa).ok_or_else(|| {
        KhataRuqaa::MalafQaseer {
            haql: "the hashed content",
            tul: tul_malaf,
            matlub: jadwal.nihayat_muhtawa(),
        }
    })?;
    let mahsuba = *blake3::hash(muhtawa).as_bytes();
    if mahsuba != tarwisa.basma {
        return Err(KhataRuqaa::BasmaGhayrMutabaqa {
            muallana: tarwisa.basma_nass(),
            mahsuba: sittasi(&mahsuba),
        });
    }
    kutla.tahaqquq(&mahsuba, mudaqqiq)?;

    let bidaya = hajm_usize(madkhal.izaha).ok_or(KhataRuqaa::KutlatTawqeeTalifa {
        haql: "the block's offset does not fit this platform's address space",
    })?;
    let nihaya = bidaya.checked_add(HAJM_KUTLA).ok_or(KhataRuqaa::KutlatTawqeeTalifa {
        haql: "the block's offset plus its length overflows",
    })?;
    // Read before the mutable borrow: the closure would have to hold `malaf`
    // shared while `get_mut` holds it uniquely.
    let tul_malaf = tul_u64(malaf.len());
    let nafidha = malaf.get_mut(bidaya..nihaya).ok_or_else(|| KhataRuqaa::MalafQaseer {
        haql: "the signature block",
        tul: tul_malaf,
        matlub: tul_u64(nihaya),
    })?;
    kutla.ila_bayt(nafidha);
    Ok(())
}

/// The refusal for arithmetic that would exceed what a container may declare.
const fn mufrit_majmu() -> KhataRuqaa {
    KhataRuqaa::MajmuKhaamMufrit { majmu: u64::MAX, saqf: AQSA_MAJMU_KHAAM }
}

/// The next sixteen-byte boundary at or after an offset.
fn muhadhah(izaha: u64) -> Result<u64, KhataRuqaa> {
    let baqi = izaha % MUHADHAT_QISM;
    if baqi == 0 {
        return Ok(izaha);
    }
    izaha.checked_add(MUHADHAT_QISM - baqi).ok_or_else(mufrit_majmu)
}

/// An offset plus a record array's extent, within one section.
fn baad(naw: NawQism, izaha: u32, adad: u32, khatwa: usize) -> Result<u32, KhataRuqaa> {
    let talif = || KhataRuqaa::JadwalTalif {
        naw: naw.raqm(),
        haql: "izaha",
        qeema: u64::from(izaha),
        hadd: u64::from(u32::MAX),
    };
    let khatwa = u32::try_from(khatwa).map_err(|_| talif())?;
    adad.checked_mul(khatwa).and_then(|tul| izaha.checked_add(tul)).ok_or_else(talif)
}

/// A count or a length as the `u32` the preambles store.
fn adad_u32(naw: NawQism, haql: &'static str, adad: usize) -> Result<u32, KhataRuqaa> {
    u32::try_from(adad).map_err(|_| KhataRuqaa::JadwalTalif {
        naw: naw.raqm(),
        haql,
        qeema: tul_u64(adad),
        hadd: u64::from(u32::MAX),
    })
}

/// Adds a string to a section's pool, storing identical strings once.
///
/// Deduplication is not a size optimisation so much as a correctness
/// convenience: a script where forty menu entries say the same four words
/// produces one pool entry and forty equal references to it, which is what makes
/// a diff between two builds readable.
fn adkhil_hawd<'a>(
    naw: NawQism,
    hawd: &mut Vec<u8>,
    mudkhal: &mut HashMap<&'a str, MarjaNass>,
    nass: &'a str,
) -> Result<MarjaNass, KhataRuqaa> {
    if nass.is_empty() {
        return Ok(MarjaNass::default());
    }
    if let Some(marja) = mudkhal.get(nass) {
        return Ok(*marja);
    }
    let izaha = adad_u32(naw, "izahat_hawd", hawd.len())?;
    let tul = adad_u32(naw, "tul_hawd", nass.len())?;
    if izaha.checked_add(tul).is_none() {
        return Err(KhataRuqaa::JadwalTalif {
            naw: naw.raqm(),
            haql: "tul_hawd",
            qeema: u64::from(tul),
            hadd: u64::from(u32::MAX),
        });
    }
    hawd.extend_from_slice(nass.as_bytes());
    let marja = MarjaNass { izaha, tul };
    mudkhal.insert(nass, marja);
    Ok(marja)
}

/// Appends bytes to a section body, refusing one that outgrows the ceiling.
fn udfu(jism: &mut Vec<u8>, naw: NawQism, bayt: &[u8]) -> Result<(), KhataRuqaa> {
    let mufrit = |qeema: u64| KhataRuqaa::HajmKhaamMufrit {
        naw: naw.raqm(),
        muallan: qeema,
        saqf: AQSA_QISM_KHAAM,
    };
    let baad_dafa = tul_u64(jism.len())
        .checked_add(tul_u64(bayt.len()))
        .ok_or_else(|| mufrit(u64::MAX))?;
    if baad_dafa > AQSA_QISM_KHAAM {
        return Err(mufrit(baad_dafa));
    }
    jism.extend_from_slice(bayt);
    Ok(())
}

/// Appends a record array to a section body, in its stored byte form.
fn udfu_sijillat<T: Pod>(
    jism: &mut Vec<u8>,
    naw: NawQism,
    sijillat: &[T],
) -> Result<(), KhataRuqaa> {
    let bayt: &[u8] = bytemuck::try_cast_slice(sijillat).map_err(|_| {
        KhataRuqaa::MuhadhahaGhayrSaliha {
            naw: naw.raqm(),
            haql: "sijillat",
            izaha: tul_u64(jism.len()),
        }
    })?;
    udfu(jism, naw, bayt)
}

/// Writes a section's bytes at its offset in the assembled file.
fn unsakh(malaf: &mut [u8], izaha: u64, bayt: &[u8], hajm: u64) -> Result<(), KhataRuqaa> {
    let qaseer = |matlub: u64| KhataRuqaa::MalafQaseer {
        haql: "a section's extent",
        tul: hajm,
        matlub,
    };
    let bidaya = hajm_usize(izaha).ok_or_else(|| qaseer(izaha))?;
    let nihaya = bidaya.checked_add(bayt.len()).ok_or_else(|| qaseer(izaha))?;
    let nafidha = malaf.get_mut(bidaya..nihaya).ok_or_else(|| qaseer(tul_u64(nihaya)))?;
    nafidha.copy_from_slice(bayt);
    Ok(())
}
