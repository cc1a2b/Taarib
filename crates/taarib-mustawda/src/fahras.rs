//! The global manifest and the shard type that cannot exist unverified.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::khariji::RuqaaKharijiya;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
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
        let bayan: Self =
            serde_json::from_slice(bayt).map_err(|khata| KhataMustawda::BayanTalif {
                sabab: khata.to_string(),
            })?;
        if bayan.isdar != ISDAR_BAYAN {
            return Err(KhataMustawda::BayanTalif {
                sabab: format!(
                    "manifest schema {} is not the {ISDAR_BAYAN} this build reads",
                    bayan.isdar
                ),
            });
        }
        if let Some(sabiq) = mukhazzan
            && bayan.tasalsul < sabiq
        {
            return Err(KhataMustawda::TasalsulLilkhalf {
                wujid: bayan.tasalsul,
                mukhazzan: sabiq,
            });
        }
        Ok(bayan)
    }
}

/// One published memory share, as the index lists it.
///
/// A memory share is `taarib_warsha::mushtaraka`'s artifact: a signed,
/// per-game file of overlay readings — lines one player's overlay recognized
/// and had translated — which is **not** a patch and installs nothing. It is
/// listed here rather than distributed some other way because the sharded
/// index, the manifest hash chain and the revocation list already exist and
/// already do exactly what this needs; a second distribution path would be a
/// second thing to keep signed, mirrored and revocable.
///
/// It is a distinct type from [`MulakhkhasRuqaa`] rather than a flag on it,
/// and that distinction is the point. A patch is reviewed work somebody
/// submitted; a share is accumulated machine output that nobody reviewed. A
/// listing that could not tell them apart would put them in the same ranking,
/// and the fields here are the ones that decide whether a share is worth
/// taking: how many readings, how many of them a recognizer actually
/// measured, and the lowest measurement in the file.
///
/// The counts are **claims by the sharer**, carried so a client can rank and
/// filter before downloading. They are re-derived from the file itself on
/// import — `taarib_warsha::mushtaraka::istawrid` checks the body against the
/// signed header — so a listing that overstates its own quality wastes a
/// download and changes nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MulakhkhasDhakira {
    /// Who shared it, as a signing-key fingerprint.
    pub musahim: MusahimId,
    /// Their signing key's public half, lowercase hex.
    ///
    /// Present so a client can verify the file it downloads without a second
    /// round trip. It is the key the *index* names, and the index is hashed
    /// into the signed manifest — so substituting a key means substituting a
    /// shard hash, which the manifest refuses.
    pub miftah: String,
    /// When it was published, RFC 3339.
    pub waqt: String,
    /// How many readings it carries.
    pub adad: u64,
    /// How many of those carry a confidence a recognizer actually measured.
    pub adad_maqis: u64,
    /// The lowest measured confidence in it, absent when nothing was measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adna_thiqa: Option<u8>,
    /// BLAKE3 of the whole share file, lowercase hex.
    pub basma: Basma,
    /// The file's size in bytes, for the download budget.
    pub hajm: u64,
    /// Where the file is fetched from, relative to the release area.
    pub rabt: String,
    /// A second source for the same bytes, when the catalogue has one.
    ///
    /// Same shape as a patch listing's mirror, so
    /// [`crate::tanzeel::TalabTanzeel::min_dhakira`] gets the failover the
    /// other two artifact kinds already have rather than a narrower download.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rabt_mira: Option<String>,
}

impl MulakhkhasDhakira {
    /// How many readings carry no measurement at all.
    #[must_use]
    pub const fn adad_ghayr_maqis(&self) -> u64 {
        self.adad.saturating_sub(self.adad_maqis)
    }

    /// Whether every reading in it was measured by a recognizer.
    ///
    /// Rarely true, and saying so is the honest framing: only macOS Vision
    /// reports a per-line confidence, so a share from a Windows or Linux
    /// player answers `false` and a client should present it as unmeasured
    /// rather than as low quality — they are different things.
    #[must_use]
    pub const fn kulluha_maqisa(&self) -> bool {
        self.adad > 0 && self.adad_maqis == self.adad
    }
}

/// One shard's contents: every patch, voice pack, memory share and third-party
/// entry for the games in it.
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
    /// Memory shares, keyed the same way.
    ///
    /// `skip_serializing_if` as well as `default`, so a catalogue with no
    /// shares casts byte-identical shards to one cast before this field
    /// existed — the same care [`TajawuzNashr`] takes, and for the same
    /// reason: a shard whose bytes changed is a shard hash that changed, and
    /// that is a manifest revision every client fetches.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dhakirat: BTreeMap<LubaId, Vec<MulakhkhasDhakira>>,
    /// Patches somebody else made, keyed the same way.
    ///
    /// A fourth map rather than one list of a sum type over the kinds, for the
    /// reason [`MulakhkhasDhakira`] is its own type rather than a flag: these
    /// are not interchangeable with the others. A [`MulakhkhasRuqaa`] is a
    /// package Taarib compiled, signed, and whose own asset gate certified as
    /// carrying zero bytes of the game; a [`RuqaaKharijiya`] is an archive
    /// somebody else built *out of* the game's own containers, which Taarib
    /// fetches and verifies and never built. One list would hand every reader
    /// a mixed sequence to re-sort, and the accident that follows is one
    /// kind's guarantees being read over the other. Separate maps make that
    /// unspellable: an accessor that returns [`MulakhkhasRuqaa`] cannot name
    /// one of these, so the certificate's subject and a game-derived container
    /// never arrive through the same call.
    ///
    /// `skip_serializing_if` for the same reason `dhakirat` has it: a
    /// catalogue with no third-party entries casts byte-identical shards to
    /// one cast before this field existed, and the live registry is already
    /// published.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kharijiya: BTreeMap<LubaId, Vec<RuqaaKharijiya>>,
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
    pub fn min_bayt(raqm: u16, bayt: &[u8], bayan: &BayanMustawda) -> Result<Self, KhataMustawda> {
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
        let muhtawa =
            serde_json::from_slice(bayt).map_err(|khata| KhataMustawda::ShareehaTalifa {
                raqm,
                sabab: khata.to_string(),
            })?;
        Ok(Self {
            raqm,
            basma: mahsuba,
            muhtawa,
        })
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

    /// Every memory share published for one game.
    #[must_use]
    pub fn dhakirat(&self, luba: LubaId) -> &[MulakhkhasDhakira] {
        self.muhtawa.dhakirat.get(&luba).map_or(&[], Vec::as_slice)
    }

    /// Every third-party entry listed for one game.
    ///
    /// Deliberately not folded into [`Self::ruqaa`]. A caller asking for the
    /// patches Taarib built gets exactly those, and one asking for work
    /// somebody else made has to say so.
    #[must_use]
    pub fn kharijiya(&self, luba: LubaId) -> &[RuqaaKharijiya] {
        self.muhtawa.kharijiya.get(&luba).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod fuhus {
    #![allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "a test reports failure by panicking; the lints are written for library \
                  code, and honouring them here would mean a test that cannot fail"
    )]

    use std::error::Error;

    use super::{MuhtawaShareeha, MulakhkhasDhakira};
    use crate::khariji::badhrat_rtea;
    use taarib_mustalahat::bina::Basma;
    use taarib_mustalahat::khariji::{FapsBina, TahdidBina};
    use taarib_mustalahat::luba::{LubaId, MasdarLuba};
    use taarib_mustalahat::musahim::MusahimId;

    /// What the tests added since the module-level allow was written return, so
    /// a setup failure propagates with `?` rather than through a panic.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// A catalogue with no memory shares and no third-party entries casts
    /// exactly the bytes it cast before either field existed.
    ///
    /// This is the compatibility claim those two `skip_serializing_if`s make,
    /// and it is worth a test because getting it wrong changes every shard's
    /// hash, which changes the manifest, which every client on every machine
    /// then refetches.
    #[test]
    fn shareeha_bila_dhakirat_tabqa_kama_kanat() {
        let farigha = MuhtawaShareeha::default();
        let bayt = match serde_json::to_vec(&farigha) {
            Ok(bayt) => bayt,
            Err(khata) => panic!("an empty shard would not serialize: {khata}"),
        };
        assert_eq!(String::from_utf8_lossy(&bayt), r#"{"ruqaa":{},"aswat":{}}"#);
    }

    /// A shard cast before these fields existed still parses,
    /// `deny_unknown_fields` notwithstanding — the absent key is the default.
    ///
    /// The one that matters most: the live registry is already published, and
    /// every shard in it is exactly these twenty-three bytes or a longer
    /// document with the same two keys.
    #[test]
    fn shareeha_qadeema_tuqra() {
        let bayt = br#"{"ruqaa":{},"aswat":{}}"#;
        let muhtawa: MuhtawaShareeha = match serde_json::from_slice(bayt) {
            Ok(muhtawa) => muhtawa,
            Err(khata) => panic!("an older shard no longer parses: {khata}"),
        };
        assert!(muhtawa.dhakirat.is_empty());
        assert!(muhtawa.kharijiya.is_empty());
    }

    /// A published shard carrying real listings and neither of the two newer
    /// keys parses too, so the claim is about catalogues and not only about
    /// the empty document.
    #[test]
    fn shareeha_manshura_bila_kharijiya_tuqra() {
        let bayt = br#"{
            "ruqaa": {},
            "aswat": {"51e4d2a0-0000-4000-8000-000000000001": []}
        }"#;
        let muhtawa: MuhtawaShareeha = match serde_json::from_slice(bayt) {
            Ok(muhtawa) => muhtawa,
            Err(khata) => panic!("a published shard no longer parses: {khata}"),
        };
        assert_eq!(muhtawa.aswat.len(), 1);
        assert!(muhtawa.kharijiya.is_empty());
    }

    /// A cast including a third-party entry produces a shard a client reads
    /// back whole, keyed by the game — and reads back as the third-party kind,
    /// not as a patch Taarib built.
    #[test]
    fn shareeha_bi_ruqaa_kharijiya_tadur() {
        let madkhal = badhrat_rtea();
        let luba = madkhal.luba;
        let mut muhtawa = MuhtawaShareeha::default();
        let _ = muhtawa.kharijiya.insert(luba, vec![madkhal.clone()]);

        let bayt = match serde_json::to_vec(&muhtawa) {
            Ok(bayt) => bayt,
            Err(khata) => panic!("the shard would not serialize: {khata}"),
        };
        let raji: MuhtawaShareeha = match serde_json::from_slice(&bayt) {
            Ok(raji) => raji,
            Err(khata) => panic!("the shard would not parse back: {khata}"),
        };
        assert_eq!(raji.kharijiya.get(&luba), Some(&vec![madkhal]));
        assert!(
            raji.ruqaa.is_empty(),
            "a third-party entry is not a patch Taarib built"
        );
    }

    /// The build declaration survives the shard, which is the whole of it: a
    /// probe the cast drops resolves to `Majhula` on every machine that fetches
    /// the catalogue, and the feature is dead without one line changing colour.
    #[test]
    fn tahdid_albina_yanju_min_alshareeha() -> NatijatIkhtibar {
        let madkhal = badhrat_rtea();
        let luba = madkhal.luba;
        let mut muhtawa = MuhtawaShareeha::default();
        let _ = muhtawa.kharijiya.insert(luba, vec![madkhal.clone()]);

        let bayt = serde_json::to_vec(&muhtawa)?;
        let raji: MuhtawaShareeha = serde_json::from_slice(&bayt)?;
        let awwal = raji
            .kharijiya
            .get(&luba)
            .and_then(|qaima| qaima.first())
            .ok_or("the shard lost the entry")?;
        assert_eq!(awwal.tahdid_bina, madkhal.tahdid_bina);
        assert_eq!(
            awwal.tahdid_bina.faps,
            Some(FapsBina::MawridIsdar {
                masar: "RDR2.exe".to_owned(),
                juz: 2,
            })
        );
        Ok(())
    }

    /// A shard whose entries predate the field parses, and the entry that comes
    /// out declares nothing rather than failing to exist.
    ///
    /// The document is a real cast with the key deleted rather than JSON typed
    /// by hand: a hand-written fixture stops representing a published shard the
    /// moment any other field's wire form moves, and the claim being made here
    /// is about catalogues that are already live.
    #[test]
    fn shareeha_bi_madkhal_bila_tahdid_bina_tuqra() -> NatijatIkhtibar {
        let luba = badhrat_rtea().luba;
        let mut muhtawa = MuhtawaShareeha::default();
        let _ = muhtawa.kharijiya.insert(luba, vec![badhrat_rtea()]);
        let mut wathiqa = serde_json::to_value(&muhtawa)?;

        let madkhal = wathiqa
            .get_mut("kharijiya")
            .and_then(|qaima| qaima.get_mut(luba.to_string()))
            .and_then(|qaima| qaima.get_mut(0))
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("the shard carries no entry")?;
        assert!(
            madkhal.remove("tahdid_bina").is_some(),
            "the cast wrote no tahdid_bina to remove"
        );

        let raji: MuhtawaShareeha = serde_json::from_value(wathiqa)?;
        let awwal = raji
            .kharijiya
            .get(&luba)
            .and_then(|qaima| qaima.first())
            .ok_or("the shard lost the entry")?;
        assert_eq!(awwal.tahdid_bina, TahdidBina::default());
        assert!(awwal.tahdid_bina.faps.is_none());
        Ok(())
    }

    /// A listed share round-trips whole, keyed by the game it came from.
    #[test]
    fn shareeha_bi_dhakira_tadur() {
        let luba = LubaId::min_masdar(&MasdarLuba::Steam(1_245_620), "ELDEN RING");
        let musahim = match MusahimId::jadeed("e".repeat(64)) {
            Ok(id) => id,
            Err(khata) => panic!("bad fixture identity: {khata}"),
        };
        let mulakhkhas = MulakhkhasDhakira {
            musahim,
            miftah: "f".repeat(64),
            waqt: "2026-09-05T10:00:00Z".to_owned(),
            adad: 812,
            adad_maqis: 812,
            adna_thiqa: Some(63),
            basma: Basma::min_bayt([7; 32]),
            hajm: 41_920,
            rabt: "dhakirat/elden-ring-1.dhakira".to_owned(),
            rabt_mira: None,
        };
        let mut muhtawa = MuhtawaShareeha::default();
        let _ = muhtawa.dhakirat.insert(luba, vec![mulakhkhas.clone()]);

        let bayt = match serde_json::to_vec(&muhtawa) {
            Ok(bayt) => bayt,
            Err(khata) => panic!("the shard would not serialize: {khata}"),
        };
        let raji: MuhtawaShareeha = match serde_json::from_slice(&bayt) {
            Ok(raji) => raji,
            Err(khata) => panic!("the shard would not parse back: {khata}"),
        };
        assert_eq!(raji.dhakirat.get(&luba), Some(&vec![mulakhkhas]));
        assert!(raji.ruqaa.is_empty());
    }

    /// The measured count is a fact about the file, not a quality score, and
    /// an all-unmeasured share says so rather than reading as a bad one.
    #[test]
    fn kulluha_maqisa_taqul_alhaqiqa() {
        let asas = |adad: u64, maqis: u64| MulakhkhasDhakira {
            musahim: match MusahimId::jadeed("a".repeat(64)) {
                Ok(id) => id,
                Err(khata) => panic!("bad fixture identity: {khata}"),
            },
            miftah: "b".repeat(64),
            waqt: "2026-09-05T10:00:00Z".to_owned(),
            adad,
            adad_maqis: maqis,
            adna_thiqa: None,
            basma: Basma::min_bayt([0; 32]),
            hajm: 1,
            rabt: "x".to_owned(),
            rabt_mira: None,
        };
        assert!(asas(10, 10).kulluha_maqisa());
        assert!(!asas(10, 0).kulluha_maqisa());
        assert_eq!(asas(10, 0).adad_ghayr_maqis(), 10);
        // An empty share is not "all measured".
        assert!(!asas(0, 0).kulluha_maqisa());
    }
}
