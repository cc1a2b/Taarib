//! الخزينة — the layout cache: the reason a menu that redraws every frame
//! shapes once.
//!
//! `taarib_takhtit` consults this before it touches the engine. The key is
//! a 64-bit digest of the *complete shape of the request*; the value is the
//! finished [`TakhtitNass`], exactly as `saff` produced it. On a hit the entry
//! point copies positioned glyphs out of the cached layout into the caller's
//! buffer and sets [`TAARIB_TAKHTIT_MAKHZAN`](crate::anwa::TAARIB_TAKHTIT_MAKHZAN);
//! shaping, bidi, breaking and justification all cost nothing. On a miss the
//! engine runs and the result is stored on its way out.
//!
//! ## The key: everything that changes the answer, nothing that does not
//!
//! Both failure modes are real. A field left *out* of the key makes two
//! different requests collide on purpose, and the second one is served the
//! first one's layout — wrong glyphs, drawn confidently, forever. A field put
//! *into* the key that does not change the output splits one entry into many,
//! and the miss rate climbs in exactly the workload the cache exists for.
//! When a field is genuinely uncertain the rule is to include it: the cost of
//! over-keying is a duplicate entry, the cost of under-keying is a wrong
//! layout, and those are not the same size of mistake.
//!
//! Field by field, from [`TalabTakhtit`] and [`KhiyaratTakhtit`]:
//!
//! | request field | keyed? | why |
//! | --- | --- | --- |
//! | `nass` (the text) | yes — length, then bytes | the text is the layout |
//! | `khutut` (the chain handle) | **no — identities instead** | see below |
//! | font identities, in chain order | yes — count, then each | which outlines exist, which fallback wins, what `Harf::khatt` indexes |
//! | `hajm` (pixel size) | yes — bit pattern | every advance and position scales with it |
//! | `ard_mutah` (available width) | yes — presence, then bits | decides where lines break and how far justification stretches |
//! | `irtifa_mutah` (available height) | yes — presence, then bits | decides vertical overflow: the [`TaqreerTajawuz`](taarib_saff::natija::TaqreerTajawuz) stored *inside* the cached layout quotes it |
//! | span `id` | yes | carried into every glyph as [`Harf::nitaq`] — it is part of the output |
//! | span `bidaya`, `tul` | yes | which glyphs inherit which id, and where shaping runs split |
//! | span `khatt`, `wazn`, `maail`, `hajm`, `tabaud`, `izaha` | yes | each one moves glyphs or changes which glyphs exist |
//! | span atom `ard`, `irtifa`, `asas` | yes | the hole an atom reserves positions everything after it |
//! | span atom `marja` | **no** | never enters the output — an atom emits no glyph at all; the adapter recovers it through the span id |
//! | span `lawn` (colour value) | **no** | see below |
//! | `khiyarat.ittijah`, `lugha` | yes | base direction; `locl` letterforms and segmentation tailoring |
//! | `khiyarat.dabt`, `muhadhaha` | yes | justification mode and line placement |
//! | `khiyarat.tashkeel`, `arqam` | yes | which marks survive; which digit glyphs are shaped |
//! | `khiyarat.tajawuz` (with `adna`) | yes | report, shrink or truncate are three different layouts |
//! | `khiyarat.irtifa_satr`, `tabaud_ahruf`, `tabaud_kalimat` | yes | line height and spacing move every baseline and pen position |
//! | `khiyarat.hiwar`, `satr_wahid` | yes | dialogue keys the diacritic policy; single-line forbids wrapping |
//! | `khiyarat.sifat`, in given order | yes — count, then each | feature settings change which lookups fire |
//!
//! `hiwar` is keyed even when the diacritic policy is not
//! [`SiyasatTashkeel::IbqaFilHiwar`], and every span field is keyed whether or
//! not some other field would already split the entry. Conditional keying is
//! cleverness that buys a handful of merged entries and risks the wrong-layout
//! failure the moment the engine grows a new use for a field; unconditional
//! keying is checkable by reading one function.
//!
//! ### Font identity, not the chain handle
//!
//! [`MiftahTakhtit::min_talab`] takes the identities as a slice of
//! [`HuwiyatKhatt`](taarib_saff::khatt::HuwiyatKhatt) values and ignores
//! `talab.khutut` entirely, so an address cannot leak into the key even by
//! accident. A handle — a pointer, a slab index, a generation tag — names one
//! *construction* of a chain. Adapters rebuild chains: a level load tears the
//! chain down and builds it again from the same patch fonts, and under a
//! handle-keyed cache every entry ever stored just became unreachable, at the
//! exact moment a loading screen is about to relayout every string in the
//! interface. Identity is the hash of the font's bytes and face index
//! ([`MawridKhatt::huwiya`](taarib_saff::khatt::MawridKhatt::huwiya)), so the
//! same chain rebuilt from the same fonts produces the same key and hits.
//! Order is part of the identity list because order is part of the answer:
//! fallback resolution is positional, and [`Harf::khatt`] is an index into
//! the chain, so `[naskh, latin]` and `[latin, naskh]` are different requests.
//!
//! ### A style span's colour is not in the key — but its shape is
//!
//! Colour changes the *output* without changing a single glyph position,
//! because the engine never reads it: [`Uslub::lawn`] is carried to the
//! screen through the span **id** that every glyph records in
//! [`Harf::nitaq`], and the adapter resolves id to colour from the request it
//! is holding at draw time. The cached [`TakhtitNass`] contains ids and no
//! colours. So the split falls exactly on that line:
//!
//! - The colour **value** stays out. Keying it would miss on every colour
//!   change, and games change text colour per frame — a damage flash, a
//!   fading subtitle, a pulsing prompt. That workload must hit, and it does:
//!   a red word and a blue word over the same span table share one entry, and
//!   both draw correctly because each caller resolves the shared ids against
//!   its own colours.
//! - The span's **shape** — id, start, length — stays in. The ids inside a
//!   cached layout are meaningful only against the span table the layout was
//!   built from. Serve that layout to a request whose spans differ in extent
//!   or id and every `nitaq` field in it points at the wrong span of the new
//!   request: the wrong word turns red. That is the under-keying failure, and
//!   it is why "span shape" is in the key while "span paint" is not.
//!
//! The atom reference [`Dharra::marja`](taarib_saff::talab::Dharra::marja)
//! falls out the same way, for the same reason: an atom advances the pen and
//! emits no glyph, so `marja` never enters the layout and is recovered
//! through the span id like colour is.
//!
//! ## The key is a hash; the text is not stored
//!
//! The cache never keeps a copy of the string. A cache that stored every
//! string it was ever asked about would grow without bound in exactly the
//! workload it exists for — a long session touching thousands of distinct
//! lines of dialogue — and comparing kilobytes of text on every probe would
//! put the text back on the hot path the hash removed it from.
//!
//! The consequence must be said plainly: **two different requests that hash
//! to the same 64 bits are one entry, and the second is served the first's
//! layout.** Nothing detects it, because the material to detect it with is
//! precisely what is not stored. The trade is taken with numbers, not faith.
//! By the birthday bound, the chance that *any* two of `n` uniformly
//! distributed 64-bit keys collide is about `n² / 2⁶⁵`:
//!
//! | distinct keys in play | chance of any collision |
//! | --- | --- |
//! | 4,096 (a 4 MiB budget of short strings) | ~4.5 × 10⁻¹³ |
//! | 65,536 (a large budget, a text-heavy game) | ~1.2 × 10⁻¹⁰ |
//! | 1,048,576 (every distinct request of a long session) | ~3.0 × 10⁻⁸ |
//!
//! One session in roughly thirty million, at a million distinct requests,
//! might contain a single wrongly served layout — and only if the colliding
//! pair are alive in the same window. The hasher is `FxHasher` from the
//! workspace's `rustc-hash` 2 line, whose folded-multiply finisher mixes
//! every input bit across the full 64-bit state; it is the hash the font
//! identities themselves are built from, and deliberately not the standard
//! library's default hasher, whose per-process random seed would make the
//! same request key differently on every run — unusable in diagnostics,
//! uncomparable across processes — to buy resistance against hostile map
//! keys, a threat this cache does not face: the text comes from the patch
//! the player installed. `FxHasher` is not cryptographic, and the bound
//! above assumes uniformity, not an adversary; like
//! [`HuwiyatKhatt`](taarib_saff::khatt::HuwiyatKhatt), a key is valid within
//! one process and is never written to disk. Floats enter the key by bit
//! pattern, so `-0.0` and `0.0`, or two NaN payloads, are distinct keys —
//! the harmless direction: a duplicate entry, never a wrong hit.
//!
//! ## The budget is bytes, not entries
//!
//! An entry-counted cache holds a thousand three-word button labels or a
//! thousand full paragraphs and calls them the same size. The number a
//! game's memory plan is written in is bytes, so that is the number this
//! takes — the same rule as [`IhsaatNamu`](taarib_lawha::namu::IhsaatNamu)'s
//! atlas, whose statistics this type's are the sibling of. An entry is
//! charged exactly what it owns: the glyph vector's **capacity** times the
//! size of [`Harf`], the line vector's capacity times the size of
//! [`SatrMansuq`], plus the fixed footprint of the slab node that embeds the
//! layout and the key-to-slot pair in the index. Capacity, not length,
//! because capacity is what is resident — a layout built through a reused
//! buffer carries slack, and charging length would under-count exactly when
//! it matters. The charge is recomputed whenever an entry is replaced, since
//! two equal layouts can own unequal allocations. What is *not* charged is
//! bounded and says so: vacant slab slots and the index's table slack are
//! proportional to the entry high-water mark, not to the layouts, and the
//! layouts are where the unbounded growth lives.
//!
//! ## Eviction is least-recently-used, in constant time
//!
//! Recency is an intrusive doubly-linked list threaded through the slab:
//! each node carries two `u32` neighbour indices, the map points key → slot,
//! and head and tail are the most and least recently touched entries. A hit
//! is a map probe plus an unlink and a relink — a handful of index writes. An
//! insert is a slot reuse or push plus a relink, and eviction pops the tail
//! until the budget is met. One large insert can evict several entries, but
//! each eviction is constant-time and every evicted entry was paid for by
//! the insert that stored it, so the work is amortized constant — and, the
//! part that matters, *finding* the victim never costs a scan. The
//! scan-the-map-for-the-oldest-timestamp implementation everybody writes
//! first pays O(entries) per eviction and turns the cache into a bottleneck
//! at precisely the size where it starts mattering. A
//! clock/second-chance ring was the other constant-time candidate and was
//! rejected with reasons: its sweep is only *amortized* constant — a ring
//! full of recently referenced entries is walked in full inside one insert,
//! a stall this library is not entitled to inside somebody's frame — and it
//! evicts an approximation of the oldest entry when the exact answer costs
//! eight bytes a node. The cost of the list is those eight bytes and the
//! index discipline of link surgery, paid here once. Indices rather than
//! pointers keep the whole structure free of `unsafe`, and `u32` slots are
//! not a limit: an entry costs over a hundred bytes before its first glyph,
//! so four billion of them could not fit any budget this takes.
//!
//! ## A hit returns a borrow, and what that does to the recency update
//!
//! [`Khazina::ijlib`] returns `Option<&TakhtitNass>`. Cloning on a hit would
//! copy every glyph of a thousand-glyph paragraph to report that no work was
//! needed — for short strings, slower than shaping them, a cache that costs
//! more than its miss. But a borrow of an entry and a mutation of the
//! recency list fight over the same structure. The resolution is ordering
//! inside one `&mut self` call: `ijlib` takes the cache mutably, performs
//! the unlink-relink *first*, and only then hands out a shared borrow of the
//! entry. The returned reference keeps the whole cache borrowed for its
//! lifetime, so no insert, eviction or clear can run — which is not a
//! limitation but the safety property itself: an eviction cannot free a
//! layout the caller is still copying out of. No interior mutability, no
//! reference counting, no deferred touch queue; the borrow checker enforces
//! at compile time what those would enforce at runtime.
//!
//! ## A zero budget is the cache switched off
//!
//! The offline patch compiler lays every string out exactly once; for it, a
//! cache is pure overhead. Built with a budget of zero,
//! [`Khazina::mufaala`] answers `false`, [`Khazina::ijlib`] misses without
//! recording anything, [`Khazina::daa`] drops the layout, and no allocation
//! is ever made — the empty map and vectors of construction allocate
//! nothing, and every path that would touch them returns first. The
//! counters stay at zero deliberately: a diagnostics screen must see a
//! cache that was never consulted, not a cache with a broken key and a
//! zero hit rate.
//!
//! One more deliberate absence: this type is single-threaded, `&mut self`
//! throughout, because a recency update makes every read a write. The
//! concurrency story — the context's lock, its scope, its sharding — belongs
//! to the owner in `hayat`, which knows the call pattern; a lock buried here
//! would be the wrong lock at the wrong scope.

use std::fmt;
use std::hash::Hasher as _;

use rustc_hash::{FxHashMap, FxHasher};
use taarib_saff::natija::{Harf, SatrMansuq, TakhtitNass};
use taarib_saff::talab::{
    IttijahAsas, KhiyaratTakhtit, LughaNass, Muhadhaha, NamatDabt, NitaqUslub, SiyasatArqam,
    SiyasatTajawuz, SiyasatTashkeel, TalabTakhtit, Uslub,
};

/// The absent-neighbour sentinel of the recency list.
///
/// A real slot can never carry this value: reaching it would require four
/// billion live entries, and each entry costs more than a hundred bytes
/// before its first glyph.
const LA_SHAY: u32 = u32::MAX;

/// What one entry costs beyond its two vectors: the slab node (which embeds
/// the layout's own header, the key, the charge, and the two list links) and
/// the key-to-slot pair in the index.
///
/// The index's control bytes and load-factor slack are not in this number.
/// They are bounded by the entry high-water mark and cannot be attributed to
/// one entry exactly; the budget's job is to bound the layouts, and it does.
const KULFA_THABITA: usize = size_of::<Uqda>() + size_of::<u64>() + size_of::<u32>();

// ---------------------------------------------------------------------------
// The key
// ---------------------------------------------------------------------------

/// The 64-bit key of one layout request: the digest of everything that
/// changes the answer, and of nothing that does not.
///
/// Built by [`MiftahTakhtit::min_talab`]; the module documentation carries
/// the field-by-field account of what enters it and why. Like the font
/// identity it partly consists of, a key is stable within one process and is
/// never written to disk.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MiftahTakhtit(u64);

impl MiftahTakhtit {
    /// Digests a layout request and the identities of its font chain.
    ///
    /// `huwiyat` is the chain's fonts **in chain order**, each as
    /// [`HuwiyatKhatt::qeema`](taarib_saff::khatt::HuwiyatKhatt::qeema) —
    /// which the caller obtains by walking
    /// [`SilsilatKhutut::khutut`](taarib_saff::khatt::SilsilatKhutut::khutut).
    /// Order matters: fallback resolution is positional and
    /// [`Harf::khatt`] indexes the chain, so a reordered chain is a
    /// different request. `talab.khutut` itself is ignored entirely, so no
    /// handle, pointer or generation tag can reach the key.
    ///
    /// Every variable-length section is written length-first, so two
    /// requests cannot collide merely by moving bytes across a section
    /// boundary. Enum discriminants are written as the same stable numbers
    /// the ABI uses in `anwa`, so a reader can cross-check the two by eye.
    #[must_use]
    pub fn min_talab(talab: &TalabTakhtit<'_>, huwiyat: &[u64]) -> Self {
        let mut hashi = FxHasher::default();

        hashi.write_usize(talab.nass.len());
        hashi.write(talab.nass.as_bytes());

        hashi.write_usize(huwiyat.len());
        for huwiya in huwiyat {
            hashi.write_u64(*huwiya);
        }

        hashi.write_u32(talab.hajm.to_bits());
        iktub_kasr_ikhtiyari(&mut hashi, talab.ard_mutah);
        iktub_kasr_ikhtiyari(&mut hashi, talab.irtifa_mutah);

        hashi.write_usize(talab.nitaqat.len());
        for nitaq in talab.nitaqat {
            iktub_nitaq(&mut hashi, nitaq);
        }

        iktub_khiyarat(&mut hashi, talab.khiyarat);

        Self(hashi.finish())
    }

    /// The raw 64-bit value, for the index and for diagnostics.
    #[must_use]
    pub const fn qeema(self) -> u64 {
        self.0
    }
}

impl fmt::Display for MiftahTakhtit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl fmt::Debug for MiftahTakhtit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MiftahTakhtit({self})")
    }
}

/// Writes an optional float as a presence byte then its bit pattern.
///
/// The presence byte keeps `Some(0.0)` and `None` distinct; the bit pattern
/// keeps the write total for every float including NaN. Two NaN payloads make
/// two keys — a duplicate entry, never a wrong hit.
fn iktub_kasr_ikhtiyari(hashi: &mut FxHasher, qeema: Option<f32>) {
    match qeema {
        Some(qeema) => {
            hashi.write_u8(1);
            hashi.write_u32(qeema.to_bits());
        },
        None => hashi.write_u8(0),
    }
}

/// Writes one span: its identity, its extent, and its layout-affecting style.
fn iktub_nitaq(hashi: &mut FxHasher, nitaq: &NitaqUslub) {
    hashi.write_u16(nitaq.id);
    hashi.write_u32(nitaq.bidaya);
    hashi.write_u32(nitaq.tul);
    iktub_uslub(hashi, &nitaq.uslub);
}

/// Writes what a style does to layout — and deliberately not what it paints.
///
/// `lawn` is skipped: the colour value never enters the engine or its
/// output, which carries only the span id. An atom's `marja` is skipped for
/// the same reason — an atom emits no glyph, so the reference is recovered
/// through the span table, not the layout. Everything else here either moves
/// glyphs or changes which glyphs exist.
fn iktub_uslub(hashi: &mut FxHasher, uslub: &Uslub) {
    match uslub.khatt {
        Some(khatt) => {
            hashi.write_u8(1);
            hashi.write_u8(khatt);
        },
        None => hashi.write_u8(0),
    }
    match uslub.wazn {
        Some(wazn) => {
            hashi.write_u8(1);
            hashi.write_u16(wazn);
        },
        None => hashi.write_u8(0),
    }
    hashi.write_u8(u8::from(uslub.maail));
    iktub_kasr_ikhtiyari(hashi, uslub.hajm);
    iktub_kasr_ikhtiyari(hashi, uslub.tabaud);
    iktub_kasr_ikhtiyari(hashi, uslub.izaha);
    match uslub.dharra {
        Some(dharra) => {
            hashi.write_u8(1);
            hashi.write_u32(dharra.ard.to_bits());
            hashi.write_u32(dharra.irtifa.to_bits());
            hashi.write_u32(dharra.asas.to_bits());
        },
        None => hashi.write_u8(0),
    }
}

/// Writes every decision of [`KhiyaratTakhtit`], unconditionally.
///
/// `hiwar` goes in even when the diacritic policy does not read it: a
/// conditional key is a wrong-layout bug waiting for the engine to grow a
/// second use of the field, and the price of unconditional inclusion is one
/// duplicate entry for callers that toggle it meaninglessly.
fn iktub_khiyarat(hashi: &mut FxHasher, khiyarat: &KhiyaratTakhtit) {
    hashi.write_u8(raqm_ittijah(khiyarat.ittijah));
    hashi.write_u8(raqm_lugha(khiyarat.lugha));
    hashi.write_u8(raqm_dabt(khiyarat.dabt));
    hashi.write_u8(raqm_muhadhaha(khiyarat.muhadhaha));
    hashi.write_u8(raqm_tashkeel(khiyarat.tashkeel));
    hashi.write_u8(raqm_arqam(khiyarat.arqam));
    match khiyarat.tajawuz {
        SiyasatTajawuz::Ballagh => hashi.write_u8(0),
        SiyasatTajawuz::Taqlis { adna } => {
            hashi.write_u8(1);
            hashi.write_u32(adna.to_bits());
        },
        SiyasatTajawuz::Ikhtisar => hashi.write_u8(2),
    }
    iktub_kasr_ikhtiyari(hashi, khiyarat.irtifa_satr);
    hashi.write_u32(khiyarat.tabaud_ahruf.to_bits());
    hashi.write_u32(khiyarat.tabaud_kalimat.to_bits());
    hashi.write_u8(u8::from(khiyarat.hiwar));
    hashi.write_u8(u8::from(khiyarat.satr_wahid));
    hashi.write_usize(khiyarat.sifat.len());
    for sifa in &khiyarat.sifat {
        hashi.write(&sifa.wasm);
        hashi.write_u32(sifa.qeema);
    }
}

/// The direction discriminant, matching `anwa::ittijah_min_raqm`.
const fn raqm_ittijah(qeema: IttijahAsas) -> u8 {
    match qeema {
        IttijahAsas::Tilqai => 0,
        IttijahAsas::Yameen => 1,
        IttijahAsas::Yasar => 2,
    }
}

/// The language discriminant, matching `anwa::lugha_min_raqm`.
const fn raqm_lugha(qeema: LughaNass) -> u8 {
    match qeema {
        LughaNass::Tilqai => 0,
        LughaNass::Arabi => 1,
        LughaNass::Farisi => 2,
        LughaNass::Urdu => 3,
        LughaNass::Latini => 4,
    }
}

/// The justification discriminant, matching `anwa::dabt_min_raqm`.
const fn raqm_dabt(qeema: NamatDabt) -> u8 {
    match qeema {
        NamatDabt::Bila => 0,
        NamatDabt::Masafat => 1,
        NamatDabt::Kashida => 2,
        NamatDabt::KashidaThummaMasafat => 3,
    }
}

/// The alignment discriminant, matching `anwa::muhadhaha_min_raqm`.
const fn raqm_muhadhaha(qeema: Muhadhaha) -> u8 {
    match qeema {
        Muhadhaha::Bidaya => 0,
        Muhadhaha::Nihaya => 1,
        Muhadhaha::Wasat => 2,
        Muhadhaha::Dabt => 3,
    }
}

/// The diacritic discriminant, matching `anwa::tashkeel_min_raqm`.
const fn raqm_tashkeel(qeema: SiyasatTashkeel) -> u8 {
    match qeema {
        SiyasatTashkeel::Ibqa => 0,
        SiyasatTashkeel::Hadhf => 1,
        SiyasatTashkeel::IbqaFilHiwar => 2,
    }
}

/// The digit discriminant, matching `anwa::arqam_min_raqm`.
const fn raqm_arqam(qeema: SiyasatArqam) -> u8 {
    match qeema {
        SiyasatArqam::KamaHiya => 0,
        SiyasatArqam::Latini => 1,
        SiyasatArqam::Arabi => 2,
        SiyasatArqam::Farisi => 3,
    }
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// What the Diagnostics screen reads — the sibling of
/// [`IhsaatNamu`](taarib_lawha::namu::IhsaatNamu), for the same screen and
/// under the same rule.
///
/// Hit and miss counters are cumulative over the cache's life and survive
/// [`Khazina::amsah`], because the question they answer — "is this cache's key
/// shaped right for this game?" — is about the session, not the current
/// contents.
///
/// `#[repr(C)]`, widest fields first, with explicit padding, so an ABI entry
/// point can hand it across by copy under the layout law of `anwa`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IhsaatKhazina {
    /// Layouts served without shaping anything.
    pub isabat: u64,
    /// Requests that had to be laid out in full.
    ///
    /// A miss count that keeps pace with the hit count in a menu that
    /// redraws every frame means the key is absorbing something that changes
    /// per frame — which is the defect
    /// [`TAARIB_TAKHTIT_MAKHZAN`](crate::anwa::TAARIB_TAKHTIT_MAKHZAN)
    /// exists to make visible.
    pub ikhfaqat: u64,
    /// Entries evicted to make room.
    pub ikhlaat: u64,
    /// What the stored layouts currently cost, in bytes.
    pub bayt: u64,
    /// The byte budget. Zero means the cache is disabled and every other
    /// number here is necessarily zero too.
    pub mizaniya: u64,
    /// How many layouts are stored right now.
    pub madkhalat: u32,
    /// Padding to a multiple of eight. Always written as zero; ignore it.
    pub hashw: u32,
}

impl IhsaatKhazina {
    /// The share of requests served from the cache, `0.0` when nothing has
    /// been asked for yet — or when the cache is disabled, which records
    /// nothing rather than reporting a hit rate of zero that would send
    /// someone hunting a key bug that does not exist.
    #[must_use]
    pub fn nisbat_isaba(&self) -> f32 {
        let kull = self.isabat.saturating_add(self.ikhfaqat);
        if kull == 0 {
            return 0.0;
        }
        ila_kasr(self.isabat) / ila_kasr(kull)
    }
}

// ---------------------------------------------------------------------------
// The cache
// ---------------------------------------------------------------------------

/// One slab slot: an entry's key, its layout, its charge, and its two
/// neighbours in the recency list.
struct Uqda {
    /// The key, kept so eviction can remove the index entry it belongs to.
    miftah: u64,
    /// The stored layout. In a vacant slot this is an empty layout owning
    /// nothing, so a slot on the free list holds no heap memory the budget
    /// is not told about.
    takhtit: TakhtitNass,
    /// The bytes this entry was charged at insertion or last replacement.
    bayt: usize,
    /// The neighbour toward the head — more recently used — or [`LA_SHAY`].
    sabiq: u32,
    /// The neighbour toward the tail — less recently used — or [`LA_SHAY`].
    tali: u32,
}

/// The layout cache: exact least-recently-used, bounded by a byte budget,
/// constant-time on both the hit and the insert.
///
/// The module documentation carries the design in full — the key, the hash
/// trade, the byte accounting, the intrusive recency list, the borrow rule
/// and the disabled mode. This type is deliberately a plain single-threaded
/// structure; its owner in `hayat` decides the locking.
pub struct Khazina {
    /// key → slab slot.
    faharis: FxHashMap<u64, u32>,
    /// The slab the recency list is threaded through.
    uqad: Vec<Uqda>,
    /// Vacated slots awaiting reuse.
    faragh: Vec<u32>,
    /// The most recently used entry, or [`LA_SHAY`] when empty.
    ras: u32,
    /// The least recently used entry — the eviction candidate.
    dhayl: u32,
    /// The byte budget. Zero disables the cache entirely.
    mizaniya: usize,
    /// The bytes currently charged to stored entries.
    bayt: usize,
    /// Hits over the cache's life. Survives [`Khazina::amsah`].
    isabat: u64,
    /// Misses over the cache's life. Survives [`Khazina::amsah`].
    ikhfaqat: u64,
    /// Evictions over the cache's life. Survives [`Khazina::amsah`].
    ikhlaat: u64,
}

impl fmt::Debug for Khazina {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Counts, not contents. A debug line that dumps every cached
        // paragraph is a line nobody reads twice.
        matbaa
            .debug_struct("Khazina")
            .field("mizaniya", &self.mizaniya)
            .field("bayt", &self.bayt)
            .field("madkhalat", &self.faharis.len())
            .field("isabat", &self.isabat)
            .field("ikhfaqat", &self.ikhfaqat)
            .field("ikhlaat", &self.ikhlaat)
            .finish_non_exhaustive()
    }
}

impl Khazina {
    /// Builds a cache with a byte budget.
    ///
    /// A budget of zero builds the disabled cache the offline patch compiler
    /// runs with: no allocation is ever made — the empty index and slab of
    /// construction allocate nothing, and every path that could grow them
    /// returns before touching them.
    #[must_use]
    pub fn jadeeda(mizaniya: usize) -> Self {
        Self {
            faharis: FxHashMap::default(),
            uqad: Vec::new(),
            faragh: Vec::new(),
            ras: LA_SHAY,
            dhayl: LA_SHAY,
            mizaniya,
            bayt: 0,
            isabat: 0,
            ikhfaqat: 0,
            ikhlaat: 0,
        }
    }

    /// Whether the cache is doing anything at all.
    ///
    /// `false` means a zero budget: every lookup misses, every store is
    /// dropped, and nothing is counted.
    #[must_use]
    pub const fn mufaala(&self) -> bool {
        self.mizaniya != 0
    }

    /// Looks a request up, refreshing its recency on a hit.
    ///
    /// Returns a borrow, not a clone: the whole point of a hit is that it
    /// costs nothing, and cloning a thousand-glyph layout to report that no
    /// work was needed would make the cache slower than shaping for short
    /// strings. The recency update happens *before* the borrow is handed
    /// out, inside this one `&mut self` call, and the returned reference
    /// then keeps the cache borrowed for its lifetime — so no insert or
    /// eviction can free the layout while the caller is still copying out
    /// of it. That is enforced by the borrow checker, not by discipline.
    #[must_use]
    pub fn ijlib(&mut self, miftah: MiftahTakhtit) -> Option<&TakhtitNass> {
        if self.mizaniya == 0 {
            // Disabled: miss without counting, so diagnostics show a cache
            // that was never consulted rather than one that is failing.
            return None;
        }
        let Some(&uqda) = self.faharis.get(&miftah.qeema()) else {
            self.ikhfaqat = self.ikhfaqat.saturating_add(1);
            return None;
        };
        self.isabat = self.isabat.saturating_add(1);
        if self.ras != uqda {
            // Unlink from wherever it sits and relink at the head. Skipped
            // when it already is the head, which in a menu redrawing the
            // same handful of strings is most hits.
            self.ifsal(uqda);
            self.sadr(uqda);
        }
        self.uqad
            .get(ila_fahras(uqda))
            .map(|mawjud| &mawjud.takhtit)
    }

    /// Stores a finished layout under its key, evicting from the cold end
    /// until the budget is met again.
    ///
    /// The contract is simple to state: after this call the cache's answer
    /// for `miftah` is `takhtit`, or nothing when it cannot be afforded —
    /// never a stale predecessor. So a layout whose charge alone exceeds the
    /// whole budget is not stored (storing it would first evict everything
    /// else and still not fit), and any existing entry under the key is
    /// removed rather than left to answer for it. Replacing an entry
    /// re-computes its charge, because two equal layouts can own unequal
    /// allocations. Dropping an unstorable layout is not a failure: the
    /// request was already answered before the cache was consulted, and the
    /// only consequence is a future miss.
    pub fn daa(&mut self, miftah: MiftahTakhtit, takhtit: TakhtitNass) {
        if self.mizaniya == 0 {
            return;
        }
        let kulfa = kulfat_madkhal(&takhtit);
        if kulfa > self.mizaniya {
            self.ihdhif(miftah.qeema());
            return;
        }

        if let Some(&mawjud) = self.faharis.get(&miftah.qeema()) {
            if let Some(uqda) = self.uqad.get_mut(ila_fahras(mawjud)) {
                let sabiq = std::mem::replace(&mut uqda.bayt, kulfa);
                uqda.takhtit = takhtit;
                self.bayt = self.bayt.saturating_sub(sabiq).saturating_add(kulfa);
            }
            if self.ras != mawjud {
                self.ifsal(mawjud);
                self.sadr(mawjud);
            }
        } else {
            let Some(uqda) = self.khassis(miftah.qeema(), takhtit, kulfa) else {
                return;
            };
            self.sadr(uqda);
            self.bayt = self.bayt.saturating_add(kulfa);
        }

        while self.bayt > self.mizaniya {
            if !self.ikhla_dhayl() {
                break;
            }
        }
    }

    /// The counters, as one copyable snapshot.
    #[must_use]
    pub fn ihsaat(&self) -> IhsaatKhazina {
        IhsaatKhazina {
            isabat: self.isabat,
            ikhfaqat: self.ikhfaqat,
            ikhlaat: self.ikhlaat,
            bayt: u64::try_from(self.bayt).unwrap_or(u64::MAX),
            mizaniya: u64::try_from(self.mizaniya).unwrap_or(u64::MAX),
            madkhalat: u32::try_from(self.faharis.len()).unwrap_or(u32::MAX),
            hashw: 0,
        }
    }

    /// Empties the cache, keeping the lifetime counters.
    ///
    /// For the moments when every stored layout just became wrong at once —
    /// the font chain was swapped, a settings change altered the digit
    /// policy for the whole patch. Every layout is dropped and every charge
    /// released; the hit, miss and eviction counters survive, exactly as the
    /// atlas's do, because the question they answer is about the session.
    /// The slab's and index's own empty shells are kept for reuse.
    pub fn amsah(&mut self) {
        self.faharis.clear();
        self.uqad.clear();
        self.faragh.clear();
        self.ras = LA_SHAY;
        self.dhayl = LA_SHAY;
        self.bayt = 0;
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Unlinks a slot from the recency list, stitching its neighbours
    /// together. The slot's own links are left stale; every caller either
    /// relinks it at the head or retires it to the free list.
    fn ifsal(&mut self, uqda: u32) {
        let Some((sabiq, tali)) = self
            .uqad
            .get(ila_fahras(uqda))
            .map(|mawjud| (mawjud.sabiq, mawjud.tali))
        else {
            return;
        };
        if sabiq == LA_SHAY {
            self.ras = tali;
        } else if let Some(mawjud) = self.uqad.get_mut(ila_fahras(sabiq)) {
            mawjud.tali = tali;
        }
        if tali == LA_SHAY {
            self.dhayl = sabiq;
        } else if let Some(mawjud) = self.uqad.get_mut(ila_fahras(tali)) {
            mawjud.sabiq = sabiq;
        }
    }

    /// Links an unlinked slot at the head of the recency list.
    fn sadr(&mut self, uqda: u32) {
        let ras = self.ras;
        if let Some(mawjud) = self.uqad.get_mut(ila_fahras(uqda)) {
            mawjud.sabiq = LA_SHAY;
            mawjud.tali = ras;
        }
        if ras == LA_SHAY {
            self.dhayl = uqda;
        } else if let Some(mawjud) = self.uqad.get_mut(ila_fahras(ras)) {
            mawjud.sabiq = uqda;
        }
        self.ras = uqda;
    }

    /// Places a new entry into a vacated slot or a fresh one, records it in
    /// the index, and returns the slot — unlinked, for the caller to head.
    ///
    /// Returns [`None`] only when the slab has reached the sentinel index,
    /// which no realizable budget permits; the layout is dropped and the
    /// cache simply declines to store, which is always a legal answer.
    fn khassis(&mut self, miftah: u64, takhtit: TakhtitNass, kulfa: usize) -> Option<u32> {
        let uqda = if let Some(hurr) = self.faragh.pop() {
            let mawjud = self.uqad.get_mut(ila_fahras(hurr))?;
            mawjud.miftah = miftah;
            mawjud.takhtit = takhtit;
            mawjud.bayt = kulfa;
            mawjud.sabiq = LA_SHAY;
            mawjud.tali = LA_SHAY;
            hurr
        } else {
            let jadeed = u32::try_from(self.uqad.len()).ok()?;
            if jadeed == LA_SHAY {
                return None;
            }
            self.uqad.push(Uqda {
                miftah,
                takhtit,
                bayt: kulfa,
                sabiq: LA_SHAY,
                tali: LA_SHAY,
            });
            jadeed
        };
        let _ = self.faharis.insert(miftah, uqda);
        Some(uqda)
    }

    /// Removes an entry by key, if present: out of the index, out of the
    /// list, its layout dropped and its charge released.
    fn ihdhif(&mut self, miftah: u64) {
        let Some(uqda) = self.faharis.remove(&miftah) else {
            return;
        };
        self.ifsal(uqda);
        self.afrigh(uqda);
    }

    /// Evicts the least recently used entry. Returns `false` when there is
    /// nothing to evict, which ends the caller's loop.
    fn ikhla_dhayl(&mut self) -> bool {
        let dhayl = self.dhayl;
        if dhayl == LA_SHAY {
            return false;
        }
        let Some(miftah) = self.uqad.get(ila_fahras(dhayl)).map(|mawjud| mawjud.miftah) else {
            return false;
        };
        self.ifsal(dhayl);
        let _ = self.faharis.remove(&miftah);
        self.afrigh(dhayl);
        self.ikhlaat = self.ikhlaat.saturating_add(1);
        true
    }

    /// Retires an already unlinked, already unindexed slot: drops its
    /// layout's allocations, releases its charge, and free-lists the slot.
    fn afrigh(&mut self, uqda: u32) {
        let kulfa = {
            let Some(mawjud) = self.uqad.get_mut(ila_fahras(uqda)) else {
                return;
            };
            let kulfa = mawjud.bayt;
            mawjud.bayt = 0;
            // An empty layout owns nothing, so assigning it drops the old
            // vectors here and now — a vacant slot must not hold memory the
            // budget no longer accounts for.
            let ittijah = mawjud.takhtit.ittijah;
            mawjud.takhtit = TakhtitNass::farigh(ittijah, 0.0);
            kulfa
        };
        self.bayt = self.bayt.saturating_sub(kulfa);
        self.faragh.push(uqda);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// What one entry costs: its glyphs, its lines, and the fixed footprint.
///
/// Capacity, not length — capacity is the memory actually resident, and a
/// layout built through a reused buffer carries slack that length would hide.
const fn kulfat_madkhal(takhtit: &TakhtitNass) -> usize {
    let huruf = takhtit.huruf.capacity().saturating_mul(size_of::<Harf>());
    let sutur = takhtit
        .sutur
        .capacity()
        .saturating_mul(size_of::<SatrMansuq>());
    KULFA_THABITA.saturating_add(huruf).saturating_add(sutur)
}

/// A slot index widened for the slab. [`LA_SHAY`] widens to a position no
/// slab reaches, so a lookup with it simply finds nothing.
fn ila_fahras(uqda: u32) -> usize {
    usize::try_from(uqda).unwrap_or(usize::MAX)
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
