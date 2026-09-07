//! النمو — the atlas that grows and forgets, inside a running game.
//!
//! The patch compiler saw every string it was given. It did not see the player's
//! name, the item a mod added, the number that reached five digits for the first
//! time, or the line a runtime capture pulled out of a menu nobody extracted. So
//! there is a second atlas, in the game process, that rasterizes what it is asked
//! for, packs it, and — when it runs out of room — forgets the glyphs nobody has
//! drawn recently.
//!
//! ## Eviction is by rectangle, never by page
//!
//! A 4096-square page holds several thousand glyphs. Reclaiming one to make room
//! for one is a trade of thousands for one, and the thousands come straight back
//! — they were being drawn a frame ago — so the atlas thrashes: every frame
//! discards a page, every frame re-rasterizes it, and the game loses its frame
//! budget to a cache that is technically working. So this evictor takes back
//! *rectangles*, in least-recently-used order, until the new glyph fits.
//!
//! Rectangle eviction has an honest cost, and it is fragmentation: shelf packing
//! reclaims a freed item for the next rectangle that fits it, and freeing the
//! two least-recently-used glyphs on two different pages does not make room for
//! one glyph on either. The evictor keeps going until the allocation succeeds,
//! which means it can free noticeably more area than the new glyph needs. That
//! is what [`IhsaatNamu::ikhlaat`] counts, and a patch whose eviction count is
//! high relative to its miss count is a patch whose atlas is too small for its
//! working set rather than one with a packing problem.
//!
//! ## Eviction can never touch the frame being drawn
//!
//! This is the defect the pinning exists to delete, and it is worth stating
//! precisely because it is close to undiagnosable from the outside.
//!
//! A frame asks for `alef`, gets a rectangle, and writes its texture coordinates
//! into a vertex buffer. Later in the same frame it asks for `qaf`, the atlas is
//! full, and the least-recently-used rectangle happens to be the one `alef` is
//! sitting in. `qaf` is rasterized into it. The draw call then runs, and the
//! quad that was going to sample `alef` samples `qaf`. The player sees one wrong
//! letter, in one word, for one frame — and only when the atlas happens to be
//! full, which is only in certain scenes, which is only for some players. The
//! bug report reads "sometimes a letter is wrong". Nothing reproduces it.
//!
//! Making that *unlikely* is not good enough, so it is made impossible instead.
//! Every glyph [`LawhaHayya::shakl`] hands out is pinned for the rest of the
//! frame, automatically — the caller cannot forget, because the caller is not
//! asked. [`LawhaHayya::ibda_itar`] clears the pins at the top of the next
//! frame. A pinned rectangle is never a candidate, so a rectangle referenced by
//! the frame being built cannot be reassigned underneath it.
//!
//! When every rectangle in the atlas is pinned and one more glyph is needed, the
//! honest answer is a refusal: [`KhataLawha::LaShayLilIkhla`] says that the text
//! on screen at one instant is larger than the atlas it was given. A missing
//! glyph reported with a reason is a defect somebody can fix. A wrong glyph with
//! no reason is not.
//!
//! Atomicity comes free from the borrow: the pages, the allocator and the glyph
//! map are all behind one `&mut self`, so no reader can observe a state where a
//! key still points at a rectangle another glyph has already been written into.
//! The map entry is removed before the page is touched, in the same exclusive
//! borrow.
//!
//! ## Growth events are counted because they are a compiler report
//!
//! An atlas that grows at runtime is doing work the patch compiler was supposed
//! to have done. Every page opened after the first means a glyph the compiler
//! never saw, which means a string it never extracted, which means text in the
//! game that will fall back to whatever the engine does with an unknown glyph.
//! [`IhsaatNamu::ahdath_namu`] and [`IhsaatNamu::ikhfaqat`] are what the
//! Diagnostics screen reads to say so out loud, rather than absorbing the cost
//! quietly and letting the patch ship with holes in its coverage.
//!
//! ## The budget is bytes
//!
//! Not pages and not entries. A budget in pages means nothing until you know the
//! page size, and a budget in entries is a budget that will eventually hold ten
//! thousand tiny glyphs or four enormous ones and behave completely differently
//! in the two cases. The number a game's memory plan is written in is bytes, so
//! that is the number this takes, and the page count is derived from it.

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use taarib_saff::khatt::SilsilatKhutut;
use taarib_saff::rasm::{MAWADI_TAHAZZUZ, NamatRasm, Rassam, SurahHarf};
use taarib_usus::khata::{Khata, Natija, Ramz, Tafsir};

use crate::khareeta::{KhareetatAshkal, MawdiShakl, MiftahShakl, NamatSafha};
use crate::khata::KhataLawha;
use crate::misafa::{KhiyaratMisafa, masafa_shakl};
use crate::rasf::{KhiyaratRasf, Rasif, Safha, abaad_masmuha};
use crate::tafrigh::miftah_qiyasi;

/// What the Diagnostics screen reads.
///
/// Every counter here is cumulative over the atlas's life and survives
/// [`LawhaHayya::amsah`], because the question they answer — "is this patch's
/// atlas doing work its compiler should have done?" — is about the session, not
/// about the current contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IhsaatNamu {
    /// Glyphs served from the atlas without rasterizing anything.
    pub isabat: u64,
    /// Glyphs that had to be rasterized and packed.
    ///
    /// A number that keeps climbing after the first minutes of play is the
    /// signal that the patch compiler is missing strings.
    pub ikhfaqat: u64,
    /// Rectangles reclaimed to make room.
    pub ikhlaat: u64,
    /// Pages opened after the first.
    pub ahdath_namu: u64,
    /// What the pages currently cost, in bytes.
    pub bayt: u64,
    /// The byte budget this atlas was built with.
    pub mizaniya: u64,
    /// How many pages are open.
    pub safahat: u16,
    /// How many glyphs are mapped.
    pub ashkal: u32,
    /// How many glyphs the frame being drawn has referenced.
    pub mathbut: u32,
}

impl IhsaatNamu {
    /// The share of requests served without rasterizing, `0.0` when nothing has
    /// been asked for yet.
    ///
    /// The one number worth putting on a screen: a healthy patch settles above
    /// 0.99 within seconds of a menu opening and stays there.
    #[must_use]
    pub fn nisbat_isaba(&self) -> f32 {
        let kull = self.isabat.saturating_add(self.ikhfaqat);
        if kull == 0 {
            return 0.0;
        }
        ila_kasr(self.isabat) / ila_kasr(kull)
    }

    /// The share of the byte budget currently held by pages.
    #[must_use]
    pub fn nisbat_mizaniya(&self) -> f32 {
        if self.mizaniya == 0 {
            return 0.0;
        }
        ila_kasr(self.bayt) / ila_kasr(self.mizaniya)
    }
}

/// One glyph's place in the atlas, as the evictor needs to know it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QaydShakl {
    safha: u16,
    /// The packer's allocation identifier, or [`None`] for a glyph with no
    /// image — a space, a joiner, a mark the font draws nothing for. Those hold
    /// no rectangle, so evicting one frees nothing and the evictor skips them.
    takhsees: Option<u32>,
    /// The tick this glyph was last handed out at.
    akhir: u64,
}

/// A glyph atlas that grows and evicts while a game is running.
pub struct LawhaHayya {
    rasif: Rasif,
    khareeta: KhareetatAshkal,
    quyud: FxHashMap<MiftahShakl, QaydShakl>,
    mathbutat: FxHashSet<MiftahShakl>,
    misafa: KhiyaratMisafa,
    tik: u64,
    ihsaat: IhsaatNamu,
}

impl core::fmt::Debug for LawhaHayya {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Counts, not contents. A debug line that lists every glyph in a runtime
        // atlas is a line nobody reads twice.
        matbaa
            .debug_struct("LawhaHayya")
            .field("rasif", &self.rasif)
            .field("ashkal", &self.quyud.len())
            .field("mathbut", &self.mathbutat.len())
            .field("ihsaat", &self.ihsaat)
            .finish_non_exhaustive()
    }
}

impl LawhaHayya {
    /// Builds a runtime atlas inside a byte budget.
    ///
    /// The page count is derived: as many pages of the size `khiyarat` implies
    /// as `mizaniyat_bayt` pays for, never more than `khiyarat.aqsa_safahat`.
    /// The first page is opened immediately, so the budget is being spent from
    /// the moment the atlas exists rather than at the first surprise inside a
    /// frame.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::AbaadSafhaGhayrSaliha`] for page dimensions the packer
    /// cannot use.
    ///
    /// [`KhataLawha::MizaniyaAsghurMinSafha`] when the budget will not pay for a
    /// single page. An atlas with no page cannot hold a glyph, and finding that
    /// out at construction is better than finding it out per glyph, forever.
    pub fn jadeeda(
        khiyarat: KhiyaratRasf,
        namat: NamatSafha,
        mizaniyat_bayt: usize,
    ) -> Natija<Self> {
        Self::jadeeda_bi_misafa(khiyarat, namat, mizaniyat_bayt, KhiyaratMisafa::default())
    }

    /// [`LawhaHayya::jadeeda`], with the distance-field options stated.
    ///
    /// Ignored entirely for a coverage atlas. For a distance-field atlas these
    /// are the options every glyph is generated with, and the cell each one uses
    /// is the pixel size its own key carries — which is what makes an SDF key's
    /// size mean something different from a coverage key's: the cell it is
    /// stored in, not the size it is drawn at.
    ///
    /// # Errors
    ///
    /// As [`LawhaHayya::jadeeda`].
    pub fn jadeeda_bi_misafa(
        mut khiyarat: KhiyaratRasf,
        namat: NamatSafha,
        mizaniyat_bayt: usize,
        misafa: KhiyaratMisafa,
    ) -> Natija<Self> {
        let (ard, irtifa) = abaad_masmuha(khiyarat)?;
        let hajm_safha = u64::from(ard).saturating_mul(u64::from(irtifa));
        let mizaniya = u64::try_from(mizaniyat_bayt).unwrap_or(u64::MAX);

        let masmuh = mizaniya.checked_div(hajm_safha).unwrap_or(0);
        if masmuh == 0 {
            return Err(KhataLawha::MizaniyaAsghurMinSafha {
                mizaniya,
                hajm_safha,
            }
            .into());
        }
        // The budget decides, unless the caller asked for fewer pages than it
        // pays for. A caller that asked for none is refused by the packer rather
        // than quietly given one.
        khiyarat.aqsa_safahat = u16::try_from(masmuh)
            .unwrap_or(u16::MAX)
            .min(khiyarat.aqsa_safahat);

        let rasif = Rasif::jadeed(khiyarat, namat)?;
        let mut khareeta = KhareetatAshkal::jadeeda();
        for (fahras, safha) in rasif.safahat().iter().enumerate() {
            khareeta.sajjil_safha(ila_raqm_safha(fahras), safha.ard, safha.irtifa);
        }

        let ihsaat = IhsaatNamu {
            bayt: rasif.bayt(),
            mizaniya,
            safahat: ila_raqm_safha(rasif.adad_safahat()),
            ..IhsaatNamu::default()
        };

        Ok(Self {
            rasif,
            khareeta,
            quyud: FxHashMap::default(),
            mathbutat: FxHashSet::default(),
            misafa,
            tik: 0,
            ihsaat,
        })
    }

    /// Where a glyph is, rasterizing and packing it if it is not there yet.
    ///
    /// The returned glyph is **pinned for the rest of the frame**. That is not
    /// an option the caller enables; it is what makes the eviction safe, and a
    /// caller that could forget it would eventually forget it. Call
    /// [`LawhaHayya::ibda_itar`] once per frame to release the previous frame's
    /// pins, or the atlas will fill with unevictable glyphs and stop.
    ///
    /// `rassam` must be bound to the font `miftah.khatt` names. Nothing here can
    /// check that — a [`Rassam`] carries a font, not a chain position — so the
    /// entry point that *can* check it is
    /// [`LawhaHayya::shakl_min_silsila`], and adapters should prefer it.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::RasmFashil`], carrying the whole key and the rasterizer's
    /// own failure nested underneath it.
    ///
    /// [`KhataLawha::ShaklAkbarMinSafha`] for a glyph larger than a page.
    ///
    /// [`KhataLawha::LaShayLilIkhla`] when the atlas is full and every rectangle
    /// left in it belongs to the frame being drawn.
    pub fn shakl(&mut self, miftah: MiftahShakl, rassam: &Rassam) -> Natija<MawdiShakl> {
        let miftah = miftah_qiyasi(miftah);
        self.tik = self.tik.saturating_add(1);
        let tik = self.tik;

        if self.quyud.contains_key(&miftah) {
            if let Some(qayd) = self.quyud.get_mut(&miftah) {
                qayd.akhir = tik;
            }
            if let Some(mawdi) = self.khareeta.mawdi(miftah).copied() {
                self.ihsaat.isabat = self.ihsaat.isabat.saturating_add(1);
                let _ = self.mathbutat.insert(miftah);
                self.ihsaat.mathbut = ila_adad(self.mathbutat.len());
                return Ok(mawdi);
            }
            // The evictor's book and the map disagreed, which can only happen if
            // something outside this type reached into the map. Drop the record
            // and rebuild the glyph rather than hand back a position that points
            // at nothing.
            self.ansi(miftah);
        }

        self.ihsaat.ikhfaqat = self.ihsaat.ikhfaqat.saturating_add(1);
        let surah = self.irsim(miftah, rassam)?;
        let mawdi = self.udkhil(miftah, &surah, tik)?;
        let _ = self.mathbutat.insert(miftah);
        self.ihsaat.mathbut = ila_adad(self.mathbutat.len());
        Ok(mawdi)
    }

    /// [`LawhaHayya::shakl`], resolving the font through the chain the layout
    /// was produced with.
    ///
    /// This is the entry point that can catch the one disagreement nothing else
    /// can: a glyph whose key names a font index the chain does not have means
    /// the atlas and the chain were built from different patches, and anything
    /// drawn from there would be the wrong letter rather than a missing one.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::KhattKharijSilsila`] when the key names a font past the end
    /// of the chain, whatever [`Rassam::jadeed`] reports for a font with no
    /// outlines, and then everything [`LawhaHayya::shakl`] reports.
    pub fn shakl_min_silsila(
        &mut self,
        miftah: MiftahShakl,
        silsila: &SilsilatKhutut,
    ) -> Natija<MawdiShakl> {
        let Some(khatt) = silsila.khatt(miftah.khatt) else {
            return Err(KhataLawha::KhattKharijSilsila {
                khatt: miftah.khatt,
                adad: u32::try_from(silsila.adad()).unwrap_or(u32::MAX),
            }
            .into());
        };
        // Cheap: a `Rassam` holds a reference count on the one shared font
        // buffer and parses a table directory over it, which is what Decision 4
        // made cheap on purpose.
        let rassam = Rassam::jadeed(khatt)?;
        self.shakl(miftah, &rassam)
    }

    /// Whether the atlas already holds a glyph.
    #[must_use]
    pub fn yahwi(&self, miftah: MiftahShakl) -> bool {
        self.quyud.contains_key(&miftah_qiyasi(miftah))
    }

    /// Pins a glyph for the rest of the frame.
    ///
    /// [`LawhaHayya::shakl`] already pins everything it returns, so this is for
    /// a caller that read a position straight out of [`LawhaHayya::khareeta`]
    /// and is about to reference it — a mesh builder walking a cached layout, an
    /// adapter re-emitting a vertex buffer it built last frame. Pinning a key
    /// the atlas does not hold is allowed and costs nothing, so a batch can pin
    /// everything it is about to ask for before it asks.
    pub fn ithbit(&mut self, miftah: MiftahShakl) {
        let _ = self.mathbutat.insert(miftah_qiyasi(miftah));
        self.ihsaat.mathbut = ila_adad(self.mathbutat.len());
    }

    /// Releases one pin early.
    ///
    /// For a caller that finished with a glyph inside a frame — a text object
    /// rebuilt mid-frame, a tooltip that closed — and would rather let a large
    /// batch behind it succeed than hold a rectangle it no longer draws.
    pub fn atliq(&mut self, miftah: MiftahShakl) {
        let _ = self.mathbutat.remove(&miftah_qiyasi(miftah));
        self.ihsaat.mathbut = ila_adad(self.mathbutat.len());
    }

    /// Begins a frame: releases every pin the last frame took.
    ///
    /// Must be called once per frame. An adapter that never calls it will fill
    /// the atlas with unevictable glyphs and then fail with
    /// [`KhataLawha::LaShayLilIkhla`] — which is a loud, specific failure rather
    /// than a silent leak, and is the reason the pin set is cleared wholesale
    /// here instead of being reference-counted per glyph.
    pub fn ibda_itar(&mut self) {
        self.mathbutat.clear();
        self.ihsaat.mathbut = 0;
        self.tik = self.tik.saturating_add(1);
    }

    /// Where every glyph currently in the atlas lives.
    ///
    /// Read-only on purpose. An adapter looks a glyph up here to build a quad;
    /// it must not be able to record a placement of its own, because a rectangle
    /// the evictor does not know about is a rectangle it will hand to another
    /// glyph while something is still drawing from it.
    #[must_use]
    pub const fn khareeta(&self) -> &KhareetatAshkal {
        &self.khareeta
    }

    /// The pages, ready to upload.
    #[must_use]
    pub fn safahat(&self) -> &[Safha] {
        self.rasif.safahat()
    }

    /// The pages, mutably, for an uploader that tracks its own dirty regions.
    pub fn safahat_mut(&mut self) -> &mut [Safha] {
        self.rasif.safahat_mut()
    }

    /// The counters.
    #[must_use]
    pub const fn ihsaat(&self) -> IhsaatNamu {
        self.ihsaat
    }

    /// The fraction of allocated atlas area that glyph pixels cover.
    #[must_use]
    pub fn istighlal(&self) -> f32 {
        self.rasif.istighlal()
    }

    /// Empties the atlas, keeping one page and every lifetime counter.
    ///
    /// For a level transition, where the working set is about to be replaced
    /// wholesale and evicting it one rectangle at a time would cost a frame. The
    /// counters survive because the question they answer is about the session.
    pub fn amsah(&mut self) {
        self.rasif.amsah();
        self.khareeta.amsah();
        for (fahras, safha) in self.rasif.safahat().iter().enumerate() {
            self.khareeta
                .sajjil_safha(ila_raqm_safha(fahras), safha.ard, safha.irtifa);
        }
        self.quyud.clear();
        self.mathbutat.clear();
        self.ihsaat.bayt = self.rasif.bayt();
        self.ihsaat.safahat = ila_raqm_safha(self.rasif.adad_safahat());
        self.ihsaat.ashkal = 0;
        self.ihsaat.mathbut = 0;
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Rasterizes one key, in whichever mode the atlas holds.
    ///
    /// No variation coordinates are passed, and that is a property of the key
    /// rather than an omission: [`MiftahShakl`] names a font by its position in
    /// the patch's chain, so the instance a glyph is drawn at is the instance the
    /// chain's resource *is*. A patch that wants a variable font at a non-default
    /// weight ships the resource pinned to it, and shaping and rasterization then
    /// agree by construction — which is the whole of Decision 4. A key carrying
    /// its own axis values would let those two drift apart, and the symptom of
    /// that drift is a line whose advances no longer add up to the width it was
    /// measured at.
    fn irsim(&self, miftah: MiftahShakl, rassam: &Rassam) -> Natija<SurahHarf> {
        let natija = match self.rasif.namat() {
            NamatSafha::Taghtiya => rassam.irsim(
                miftah.muarrif,
                miftah.hajm(),
                NamatRasm::Taghtiya,
                izahat_baka(miftah.bakat),
                &[],
            ),
            NamatSafha::Masafa => {
                // An SDF key's size is the *cell* it is stored in, not a size it
                // is drawn at, so it goes in as the cell and the configured
                // spread and source resolution ride along unchanged.
                let khiyarat = KhiyaratMisafa {
                    hajm_khalyia: ila_khalyia(miftah.hajm()),
                    ..self.misafa
                };
                masafa_shakl(rassam, miftah.muarrif, khiyarat)
            },
        };
        natija.map_err(|sabab| {
            Khata::min_tafsir(&KhataLawha::RasmFashil {
                khatt: miftah.khatt,
                muarrif: miftah.muarrif,
                hajm_rubi: miftah.hajm_rubi,
            })
            .bi_sabab(sabab)
        })
    }

    /// Packs a rasterized glyph, evicting until it fits, and records it.
    fn udkhil(&mut self, miftah: MiftahShakl, surah: &SurahHarf, tik: u64) -> Natija<MawdiShakl> {
        if surah.khali() {
            // A space, a joiner, a mark with no outline. Roughly a fifth of the
            // glyphs in a line of Arabic prose land here, and they occupy no
            // rectangle at all — recording them is what stops the atlas
            // rasterizing every space in the game once per frame.
            let mawdi = MawdiShakl {
                safha: 0,
                s: 0,
                a: 0,
                ard: 0,
                irtifa: 0,
                izaha_s: 0,
                izaha_a: 0,
                taqaddum: surah.taqaddum,
            };
            self.sajjil(miftah, mawdi, None, tik);
            return Ok(mawdi);
        }

        let ard = u16::try_from(surah.ard).unwrap_or(u16::MAX);
        let irtifa = u16::try_from(surah.irtifa).unwrap_or(u16::MAX);
        let qabl = self.rasif.adad_safahat();

        let (safha, s, a, takhsees) = loop {
            match self.rasif.khassis_li(Some(miftah), ard, irtifa) {
                Ok(mawdi) => break mawdi,
                Err(khata) => {
                    if !khata.yahmil(ramz_imtila()) {
                        // Larger than a page, or a dimension the packer refuses.
                        // Neither is fixed by freeing anything.
                        return Err(khata);
                    }
                    if !self.ikhla_aqdam() {
                        return Err(Khata::min_tafsir(&KhataLawha::LaShayLilIkhla {
                            mathbut: ila_adad(self.mathbutat.len()),
                            matlub_bayt: u32::from(ard).saturating_mul(u32::from(irtifa)),
                        })
                        .bi_sabab(khata));
                    }
                    self.ihsaat.ikhlaat = self.ihsaat.ikhlaat.saturating_add(1);
                },
            }
        };

        let baad = self.rasif.adad_safahat();
        if baad > qabl {
            for fahras in qabl..baad {
                if let Some(waraqa) = self.rasif.safahat().get(fahras) {
                    let (ard_safha, irtifa_safha) = (waraqa.ard, waraqa.irtifa);
                    self.khareeta
                        .sajjil_safha(ila_raqm_safha(fahras), ard_safha, irtifa_safha);
                }
            }
            let jadeed = u64::try_from(baad.saturating_sub(qabl)).unwrap_or(0);
            self.ihsaat.ahdath_namu = self.ihsaat.ahdath_namu.saturating_add(jadeed);
            self.ihsaat.bayt = self.rasif.bayt();
            self.ihsaat.safahat = ila_raqm_safha(baad);
        }

        if let Err(khata) = self.rasif.irsim_fi(safha, s, a, surah) {
            // Give the rectangle back rather than leaving a hole nothing owns:
            // a leaked allocation is area this atlas can never reclaim, and the
            // failure it eventually causes names the wrong thing.
            self.rasif.harrir(safha, takhsees);
            return Err(khata);
        }

        let mawdi = MawdiShakl {
            safha,
            s,
            a,
            ard,
            irtifa,
            izaha_s: ila_izaha(surah.izaha_s),
            izaha_a: ila_izaha(surah.izaha_a),
            taqaddum: surah.taqaddum,
        };
        self.sajjil(miftah, mawdi, Some(takhsees), tik);
        Ok(mawdi)
    }

    /// Records a placed glyph in both the map and the evictor's book.
    fn sajjil(&mut self, miftah: MiftahShakl, mawdi: MawdiShakl, takhsees: Option<u32>, tik: u64) {
        self.khareeta.daa(miftah, mawdi);
        let _ = self.quyud.insert(
            miftah,
            QaydShakl {
                safha: mawdi.safha,
                takhsees,
                akhir: tik,
            },
        );
        self.ihsaat.ashkal = ila_adad(self.quyud.len());
    }

    /// Forgets one glyph without touching the packer.
    fn ansi(&mut self, miftah: MiftahShakl) {
        let _ = self.khareeta.ihdhif(miftah);
        let _ = self.quyud.remove(&miftah);
        self.ihsaat.ashkal = ila_adad(self.quyud.len());
    }

    /// Reclaims the least recently used rectangle the frame is not holding.
    ///
    /// Returns `false` when there is nothing left to take, which is what turns
    /// into [`KhataLawha::LaShayLilIkhla`].
    ///
    /// The scan is linear in the number of mapped glyphs. That is deliberate: a
    /// priority queue would need its key updated on every *hit*, which is the
    /// hot path — several hundred times a frame — to speed up eviction, which by
    /// construction is the cold one. Paying on the rare path is the right trade.
    ///
    /// Ties in the tick are broken by the key itself, so the eviction order is a
    /// function of the data rather than of hash-map iteration order. Two runs of
    /// the same sequence evict the same rectangles.
    fn ikhla_aqdam(&mut self) -> bool {
        let mut mureshah: Option<(u64, MiftahShakl, u16, u32)> = None;
        for (miftah, qayd) in &self.quyud {
            let Some(takhsees) = qayd.takhsees else {
                // No rectangle to reclaim; freeing it would buy nothing.
                continue;
            };
            if self.mathbutat.contains(miftah) {
                continue;
            }
            let mumkin = (qayd.akhir, *miftah, qayd.safha, takhsees);
            let afdal = mureshah.is_none_or(|(akhir, sabiq, _, _)| {
                qayd.akhir < akhir || (qayd.akhir == akhir && *miftah < sabiq)
            });
            if afdal {
                mureshah = Some(mumkin);
            }
        }

        let Some((_, miftah, safha, takhsees)) = mureshah else {
            return false;
        };
        // The map entry goes first. Nothing can look the key up and find a
        // rectangle that is about to hold a different letter, because both this
        // removal and the write that follows it happen inside one exclusive
        // borrow of the atlas.
        self.ansi(miftah);
        self.rasif.harrir(safha, takhsees);
        true
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The code a full packer reports, taken from the error type itself rather than
/// written out, so the two can never drift apart.
fn ramz_imtila() -> Ramz {
    KhataLawha::SafahatNafidat { adad: 0, hadd: 0 }.ramz()
}

/// The fractional pen position a subpixel bucket stands for.
///
/// The inverse of [`taarib_saff::rasm::bakat_tahazzuz`]: quantizing this value
/// again returns the same bucket, so the bitmap that comes back and the key that
/// asked for it describe the same subpixel position.
fn izahat_baka(bakat: u8) -> f32 {
    if MAWADI_TAHAZZUZ == 0 {
        return 0.0;
    }
    f32::from(bakat.min(MAWADI_TAHAZZUZ.saturating_sub(1))) / f32::from(MAWADI_TAHAZZUZ)
}

/// The cell size a distance-field key names, kept inside a usable range.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 1.0..=65535.0 before the conversion, so the number \
              converted is a positive whole number inside u16's range"
)]
fn ila_khalyia(hajm: f32) -> u16 {
    if !hajm.is_finite() {
        return 1;
    }
    hajm.round().clamp(1.0, f32::from(u16::MAX)) as u16
}

/// A bearing, saturated rather than wrapped.
fn ila_izaha(qeema: i32) -> i16 {
    i16::try_from(qeema).unwrap_or(if qeema < 0 { i16::MIN } else { i16::MAX })
}

/// A page index as the `u16` every glyph position carries.
fn ila_raqm_safha(fahras: usize) -> u16 {
    u16::try_from(fahras).unwrap_or(u16::MAX)
}

/// A count, saturated into the width the counters carry.
fn ila_adad(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

/// The `f32` form of a counter, for a ratio.
#[expect(
    clippy::cast_precision_loss,
    reason = "these ratios are diagnostics; a counter large enough to lose precision here has \
              long since made the ratio's exact value irrelevant"
)]
const fn ila_kasr(qeema: u64) -> f32 {
    qeema as f32
}
