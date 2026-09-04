//! The global manifest and the shard type that cannot exist unverified.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_mustalahat::sawt::MulakhkhasSawt;

use crate::khata::KhataMustawda;

/// Number of shards the catalogue is split into.
pub const ADAD_SHARAIH: u16 = 256;

/// The manifest schema this build reads.
pub const ISDAR_BAYAN: u32 = 1;

/// Which shard a game's identity falls in.
#[must_use]
pub fn shareeha(luba: LubaId) -> u16 {
    let basma = blake3::hash(luba.uuid().as_bytes());
    u16::from(basma.as_bytes().first().copied().unwrap_or(0))
}

/// One patch published over a coverage gate that refused it.
///
/// Every package carries its own verdict — `taghtiya.qabila_lil_nashr` in its
/// sealed metadata — and the caster refuses a package whose verdict is `false`.
/// That refusal can be overridden, because an operator publishing their own
/// unfinished work is a legitimate thing to do; publishing it *quietly* is not,
/// and a listing that says nothing looks exactly like one the gate passed.
///
/// So an override writes one of these into the manifest, where it travels with
/// the catalogue and is read by every client on every index refresh rather than
/// living in a terminal nobody kept. The causes are the gate's own sentences,
/// rendered by `SababAdamAlnashr` rather than paraphrased, so the record cannot
/// understate what was overridden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TajawuzNashr {
    /// The lineage that was published over the refusal.
    pub ruqaa: RuqaaId,
    /// The revision it was published at.
    pub murajaa: RuqaaRevision,
    /// Every **blocking** cause the package's coverage gate named, in its own
    /// words. Advisory causes are left out: they did not refuse the package and
    /// listing them would pad the record with things nobody overrode.
    pub asbab: Vec<String>,
    /// The sentence the operator had to write to get past the refusal.
    pub sabab: String,
}

/// The global manifest: one small document naming every shard's hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BayanMustawda {
    /// The manifest schema version.
    pub isdar: u32,
    /// Monotonic revision; a manifest with a lower number is never accepted
    /// over a cached one.
    pub tasalsul: u64,
    /// When it was published, RFC 3339.
    pub waqt: String,
    /// Shard index to the hash of that shard's bytes.
    pub sharaih: BTreeMap<u16, Basma>,
    /// Where the revocation list is fetched from.
    pub rabt_qaimat_sahb: String,
    /// Patches published over their own coverage gate's refusal.
    ///
    /// Absent from the document when there are none, which is why a manifest
    /// cast before this field existed still parses and why an ordinary
    /// catalogue's bytes are unchanged by it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tajawuzat: Vec<TajawuzNashr>,
}

impl BayanMustawda {
    /// The declared hash of one shard.
    #[must_use]
    pub fn basmat_shareeha(&self, raqm: u16) -> Option<&Basma> {
        self.sharaih.get(&raqm)
    }

    /// Whether a cached shard is still current.
    #[must_use]
    pub fn muhaddatha(&self, raqm: u16, mukhazzana: &Basma) -> bool {
        self.basmat_shareeha(raqm) == Some(mukhazzana)
    }

    /// The override recorded for one published revision, when there is one.
    ///
    /// Keyed on the revision as well as the lineage: an override is granted for
    /// the package that was in front of the operator, and a later revision of
    /// the same patch has to earn its own.
    #[must_use]
    pub fn tajawuz(&self, ruqaa: RuqaaId, murajaa: RuqaaRevision) -> Option<&TajawuzNashr> {
        self.tajawuzat
            .iter()
            .find(|tajawuz| tajawuz.ruqaa == ruqaa && tajawuz.murajaa == murajaa)
    }

    /// Parses a manifest, refusing a schema this build does not read and a
    /// revision older than the cached one.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::BayanTalif`] when the bytes are not a manifest this
    /// build reads, and [`KhataMustawda::TasalsulLilkhalf`] when `tasalsul` is
    /// below `mukhazzan`.
    pub fn min_bayt(bayt: &[u8], mukhazzan: Option<u64>) -> Result<Self, KhataMustawda> {
        let bayan: Self = serde_json::from_slice(bayt).map_err(|khata| {
            KhataMustawda::BayanTalif { sabab: khata.to_string() }
        })?;
        if bayan.isdar != ISDAR_BAYAN {
            return Err(KhataMustawda::BayanTalif {
                sabab: format!("manifest schema {} is not the {ISDAR_BAYAN} this build reads",
                    bayan.isdar),
            });
        }
        if let Some(sabiq) = mukhazzan
            && bayan.tasalsul < sabiq {
                return Err(KhataMustawda::TasalsulLilkhalf {
                    wujid: bayan.tasalsul,
                    mukhazzan: sabiq,
                });
            }
        Ok(bayan)
    }
}

/// One shard's contents: every patch and voice pack for the games in it.
///
/// `Default` is the empty shard, and it exists so that callers outside this
/// crate never have to write an exhaustive struct literal: a literal naming
/// every field turns "add a listing to a shard" into a breaking change for
/// every crate that stages one. Adding a field here should cost this file and
/// nothing else.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MuhtawaShareeha {
    /// Patches, keyed by the game they target.
    pub ruqaa: BTreeMap<LubaId, Vec<MulakhkhasRuqaa>>,
    /// Voice packs, keyed the same way.
    #[serde(default)]
    pub aswat: BTreeMap<LubaId, Vec<MulakhkhasSawt>>,
}

/// A shard whose bytes hashed to what the manifest declared.
///
/// No public constructor, no public fields, no `Deserialize`. The only way to
/// obtain one is [`ShareehaMuwaththaqa::min_bayt`], which hashes the bytes and
/// compares them to the manifest's entry before parsing a single record — so
/// index content that has not been checked against the manifest cannot be
/// represented, let alone read.
#[derive(Debug)]
pub struct ShareehaMuwaththaqa {
    raqm: u16,
    basma: Basma,
    muhtawa: MuhtawaShareeha,
}

impl ShareehaMuwaththaqa {
    /// Verifies shard bytes against the manifest and parses them.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::ShareehaMajhula`] when the manifest declares no hash
    /// for `raqm`, [`KhataMustawda::BasmaGhayrMutabaqa`] when the bytes hash to
    /// something else, and [`KhataMustawda::ShareehaTalifa`] when verified bytes
    /// do not parse.
    pub fn min_bayt(
        raqm: u16,
        bayt: &[u8],
        bayan: &BayanMustawda,
    ) -> Result<Self, KhataMustawda> {
        let muallana = bayan
            .basmat_shareeha(raqm)
            .ok_or(KhataMustawda::ShareehaMajhula { raqm })?;
        let mahsuba = Basma::min_bayt(*blake3::hash(bayt).as_bytes());
        if &mahsuba != muallana {
            return Err(KhataMustawda::BasmaGhayrMutabaqa {
                raqm,
                muallana: muallana.to_string(),
                mahsuba: mahsuba.to_string(),
            });
        }
        let muhtawa = serde_json::from_slice(bayt).map_err(|khata| {
            KhataMustawda::ShareehaTalifa { raqm, sabab: khata.to_string() }
        })?;
        Ok(Self { raqm, basma: mahsuba, muhtawa })
    }

    /// Which shard this is.
    #[must_use]
    pub const fn raqm(&self) -> u16 {
        self.raqm
    }

    /// The verified hash, for the cache.
    #[must_use]
    pub const fn basma(&self) -> Basma {
        self.basma
    }

    /// The verified contents.
    #[must_use]
    pub const fn muhtawa(&self) -> &MuhtawaShareeha {
        &self.muhtawa
    }

    /// Every patch targeting one game.
    #[must_use]
    pub fn ruqaa(&self, luba: LubaId) -> &[MulakhkhasRuqaa] {
        self.muhtawa.ruqaa.get(&luba).map_or(&[], Vec::as_slice)
    }

    /// Every voice pack targeting one game.
    #[must_use]
    pub fn aswat(&self, luba: LubaId) -> &[MulakhkhasSawt] {
        self.muhtawa.aswat.get(&luba).map_or(&[], Vec::as_slice)
    }
}
