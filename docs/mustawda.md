# المستودع — the registry repository, and how to make it live

The registry is a public Git repository. There is no server to run, no database
to pay for, and no API to keep up: the client reads static files over a forge's
raw-content endpoint, a CDN mirror of the same repository, a directory shipped
inside the application, or a mounted network share — and all four resolve the
same layout, because `taarib_mustawda::masadir::MasdarMustawda` gives none of
them a privilege the others lack.

This file is the layout's contract. It is derived from the reader, not the other
way round: every path, field and hash below is what
`crates/taarib-mustawda/src/{fahras,masadir,jalb,tanzeel}.rs` already demands.
A registry that disagrees with this file is a registry that client will refuse,
usually with `KhataMustawda::BasmaGhayrMutabaqa` or `ShareehaMajhula`.

Verified end to end against a real client on 2026-09-04: see §8.

---

## 1. The tree

```
<repository root>/
  bayan.json                       the global manifest
  sharaih/
    00.json  01.json  …  ff.json   all 256 shards, always
  sahb/
    qaima.json                     the owner-signed revocation list
  isdar/
    <ruqaa-id>/<slug>-r<n>.ruqaa   release assets (see §5 for the forge form)
```

| path | who reads it | constant |
| --- | --- | --- |
| `bayan.json` | `jalb::jalb_bayan` | `masadir::MASAR_BAYAN` |
| `sharaih/{raqm:02x}.json` | `jalb::jalb_shareeha` | `masadir::masar_shareeha` |
| the revocation list | `jalb::jalb_qaimat_sahb` | named by `bayan.rabt_qaimat_sahb` |
| release assets | `tanzeel::nazzil` | named by each listing's `rabt` |

Every repository path a source resolves must satisfy
`masadir::masar_salih`: at most 256 characters, no leading or trailing `/`, no
segment beginning with `.`, and every byte one of `a`–`z`, `0`–`9`, `.`, `-`,
`_`, `/`. Uppercase is refused, so the `{raqm:02x}` shard names are the only
spelling that resolves. Release assets are exempt — they are fetched by absolute
URL, not resolved against a source root — but keeping them inside the same byte
set is what lets an offline mirror carry them as ordinary files.

## 2. `bayan.json` — the manifest

```jsonc
{
  "isdar": 1,                          // fahras::ISDAR_BAYAN; a client refuses any other
  "tasalsul": 1,                       // monotonic; a client never accepts a lower one
  "waqt": "2026-09-04T00:00:00Z",      // RFC 3339
  "sharaih": { "0": "<64 hex>", … },   // shard index -> BLAKE3 of that shard's bytes
  "rabt_qaimat_sahb": "sahb/qaima.json",
  "tajawuzat": [ … ]                   // optional; omitted entirely when empty
}
```

Both this object and `tajawuzat`'s entries carry `#[serde(deny_unknown_fields)]`
(`crates/taarib-mustawda/src/fahras.rs:40`, `:56`), so a field this build does not
know is `BayanTalif` and not a forward-compatible extension. Add nothing here that
`BayanMustawda` does not declare.

`rabt_qaimat_sahb` is **repository-relative and nothing else**. An absolute
address there would be a manifest aiming the fetch at a host of its choosing,
and no offline mirror could serve it; `jalb_qaimat_sahb` runs `masar_salih` over
it and refuses with `BayanTalif` if it is anything more.

`tasalsul` is the rollback guard. The caller passes the cached value into
`jalb_bayan`; a manifest below it is `TasalsulLilkhalf`. **Increment it on every
publish**, including one that only changes a release asset — a client that has
seen 8 will refuse 7 forever.

### All 256 shards are published, empty ones included

This is not a stylistic choice. `jalb::jalb_sharaih` asks for every shard the
user's games fall in, and `jalb_shareeha` treats a shard that does not resolve
as a failure of the **whole** index fetch, not as an empty answer. A catalogue
that published only its non-empty shards would show *nothing at all* to any user
who owns one game in an empty shard — which, on a young registry, is every user.

The cost is nil. An empty shard is 23 bytes:

```json
{"ruqaa":{},"aswat":{}}
```

and the full manifest, with 256 hashes in it, is 20 241 bytes. A client fetches
the manifest plus one shard per distinct bucket its library touches — 17 games
in the verification below fell in 17 distinct buckets — and an unchanged
manifest costs zero requests, because a cached shard whose hash still matches is
read from `makhbaa/mustawda/sharaih/` and never requested.

### `tajawuzat` — patches published over their own coverage gate

Optional, and absent from the document rather than written as `[]` when there are
none (`fahras.rs:74`). A manifest cast before the field existed still parses, and an
ordinary catalogue's bytes are unchanged by it, which is why adding it did not move
`ISDAR_BAYAN` off 1.

An entry is written when an operator publishes a patch its own coverage gate
refused. That is a legitimate thing to do — publishing your own unfinished work is
allowed — and doing it *quietly* is not, because a listing that says nothing looks
exactly like one the gate passed. So the override travels with the catalogue and is
read by every client on every index refresh, instead of living in a terminal nobody
kept.

```jsonc
{
  "ruqaa": "<patch lineage uuid>",     // RuqaaId
  "murajaa": 3,                        // RuqaaRevision — the exact revision published
  "asbab": [ "…", "…" ],               // every BLOCKING cause the gate named, verbatim
  "sabab": "the sentence the operator had to write"
}
```

`asbab` holds the gate's own sentences, rendered by
`taarib_tarqee::taghtiya_ruqaa::SababAdamAlnashr` rather than paraphrased, so the
record cannot understate what was overridden. Advisory causes are left out: they did
not refuse the package, and listing them would pad the record with things nobody
overrode.

`sabk` writes it. `--tajawuz <why>` follows a `--huzma`
(`crates/taarib-mustawda/src/bin/sabk.rs:627`), the gate is consulted at
`bawwabat_taghtiya` (`:238`), and the entry is built at `:282` and collected into the
manifest at `:405`. Two refusals guard it in both directions: a package whose gate
already permits it is told it needs no override (`:247`), and a package whose gate
refuses it will not publish without one (`:275`).

`BayanMustawda::tajawuz(ruqaa, murajaa)` (`fahras.rs:97`) is the lookup, keyed on the
revision as well as the lineage — an override is granted for the package that was in
front of the operator, and a later revision of the same patch has to earn its own.

**Nothing reads it yet.** That accessor has no caller outside its own crate: no
client surface shows the override, so today the field is a durable record in the
catalogue and not something a user sees. Publishing an override is therefore honest
in the artifact and invisible in the application, which is half of what it was added
for.

## 3. `sharaih/{raqm:02x}.json` — a shard

The shard index is the **first byte of the BLAKE3 hash of the game's UUID**:

```rust
fahras::shareeha(luba) = blake3(luba.uuid().as_bytes()).as_bytes()[0]
```

and the game's UUID is `LubaId::min_masdar(&MasdarLuba::Steam(appid), name)` —
a UUIDv5 over `steam:<appid>` and the normalized title. The publisher must
compute it the same way; nothing in the shard file names the shard it is in, so
a listing filed in the wrong bucket is simply never found.

```jsonc
{
  "ruqaa": { "<luba-uuid>": [ MulakhkhasRuqaa, … ] },
  "aswat": { "<luba-uuid>": [ MulakhkhasSawt,  … ] }
}
```

`#[serde(deny_unknown_fields)]`: an extra key anywhere in the document makes the
whole shard `ShareehaTalifa`. `aswat` may be omitted (it is `#[serde(default)]`);
`ruqaa` may not.

The listing fields are `taarib_mustalahat::ruqaa::MulakhkhasRuqaa` verbatim. Six
of them are load-bearing and the rest are display:

| field | why it matters |
| --- | --- |
| `bina_manassa` | launcher build ids; the exact-match tier of `mutabaqa` |
| `basmat` | content fingerprints; the fingerprint-match tier |
| `hajm` | the size `tanzeel` refuses to exceed, known before a byte is fetched |
| `basmat_muhtawa` | BLAKE3 of the package file; verified **while** it downloads |
| `rabt` | the release asset, `https` only |
| `rabt_mira` | a second `https` source, tried when the first fails |

`musahim` is a `MusahimId`, which refuses anything that is not 64 lowercase hex
— it is the contributor's **public signing key**. Derive it from the package's
own signature block; a listing cannot then claim an identity the seal does not
back.

Everything else in a listing should be read out of the package's own metadata
section (`Ruqaa::bayan_json`) rather than typed in: `id`, `murajaa`, `wasf`,
`muharrik`, `irtibat`, `taghtiya` are all there, and a publisher that copies
them cannot disagree with the file it is publishing.

## 4. `sahb/qaima.json` — the revocation list

The only constructor is `taarib_aman::qaimat_sahb::QaimatSahb::min_bayt`, which
verifies the owner signature before returning; there is no `Deserialize`. The
producer side has to build the canonical form by hand:

```
"taarib.qaimat-sahb.v1\0"
u32  isdar            (1)
u64  tasalsul
u64 len + bytes       usdirat        (RFC 3339)
u64 len + bytes       salih_hatta    (RFC 3339)
u64 count, then for each revoked key, in BTreeMap order over the 32 raw bytes:
    32 bytes key, u64+bytes reason, u64+bytes timestamp
u64 count, then each revoked lineage:  16-byte UUID, u64+bytes reason, u64+bytes time
u64 count, then each revoked hash:     32 bytes,     u64+bytes reason, u64+bytes time
```

All lengths little-endian. Entries are walked in `BTreeMap` order — a list
signed in any other order verifies against nothing. The signature is Ed25519 by
the owner key, hex, in the `tawqee` field. `tasalsul` has the same rollback rule
as the manifest's, enforced by `QaimatSahb::baad`.

## 5. Release assets

A listing's `rabt` is an absolute `https` URL and nothing else: `tanzeel`'s
`rabt_amin` refuses anything that is not `https://`, longer than 2048
characters, or carrying a control character or a space — **before a socket is
opened**. On GitHub the natural home is a release asset:

```
https://github.com/<owner>/taarib-registry/releases/download/<tag>/<name>.ruqaa
```

with `rabt_mira` pointing at the CDN copy of the same repository. Both must
answer `Range` requests: `nazzil` resumes with `Range: bytes=<n>-` and reads the
`Content-Range` back, and a source that answers `200` to a range request is
restarted from zero rather than appended to. GitHub's release CDN and jsDelivr
both do. A source that honours neither costs a restart, not a corruption.

## 6. Publishing checklist

1. Compile and seal the package (`taarib-tarqee`, `taarib-khatm`).
2. Compute `blake3(file)` → `basmat_muhtawa`; note the file size → `hajm`.
3. Read `id`, `murajaa`, `wasf`, `muharrik`, `irtibat`, `taghtiya` out of the
   package's own `BAYAN` section; read the signing key out of its signature
   block → `musahim`.
4. Upload the asset; record its URL → `rabt`, and the mirror's → `rabt_mira`.
5. Insert the listing into `sharaih/{shareeha(luba):02x}.json`.
6. Rewrite **every** shard file (they are canonical JSON; only the changed one
   differs), hash each, and write `bayan.json` from those hashes with
   `tasalsul` incremented.
7. Commit and push. The client picks it up on its next manifest check.

Steps 2–6 are mechanical and must be done by a tool, not by hand: the manifest
and the shards have to agree byte for byte or the client refuses the shard.
`crates/taarib-mustawda/src/bin/sabk.rs` is that tool; it is now in-tree, and it
is the same code the verification below was run against.

## 7. The two things that are not provisioned

### 7.1 The release signing key

`crates/taarib-khatm/src/bin/isdar.rs` mints it. On the owner's machine, with
the owner's keychain unlocked:

```
cargo run -p taarib-khatm --bin isdar -- wallid --mirsa mirsa-isdar.txt
```

It generates a 32-byte seed from the OS CSPRNG, stores it in the platform
credential store under the service `taarib.tawqee` and the account `malik`,
reads it back, and writes **one** file: 64 hexadecimal characters, the public
half. There is no argument that writes the private half and no branch that
could; the seed is not printed and not put in the environment. The tool refuses
an anchor path whose name reads as a private-key file, refuses to overwrite an
existing anchor, and asks for the keychain passphrase twice before it writes.

The passphrase is the **keychain's**. Taarib has exactly one custody mechanism
for private keys — the platform store, through `keyring` — and this tool does
not derive the key from the passphrase or encrypt anything with it, because
either would put a second and weaker custody path beside the one the product
already has. It prints which store answered, so the owner can see whether the
seed is behind a passphrase or behind nothing.

`isdar mirsa` prints the stored key's public half; `isdar tahaqquq --mirsa <f>`
checks that the pair still matches. `crates/taarib-khatm/src/bin/malik.rs`
imports a seed on a second machine, and refuses any seed that does not derive
the build's own anchor.

Then every release build carries it:

```
cargo clean -p taarib-khatm
TAARIB_MIFTAH_ISDAR=$(cat mirsa-isdar.txt) \
  cargo build --release --features taarib-khatm/isdar
```

`option_env!` is not something cargo tracks for rebuilds. In a target directory
that has built this crate before, the clean is not optional: without it cargo
reuses the object file compiled against the previous anchor and says nothing,
and the binary that comes out trusts a key the owner has just retired. The mint
tool prints the clean as part of the command for that reason.

#### Rotating or retiring an anchor

There is no export and no recovery. If the owner's machine loses its keychain,
the anchor is retired and a new one published — every package signed under the
old key becomes uninstallable by clients built against it, which is the property
that makes the anchor worth anything. Rotation is therefore: mint a new key
under a new anchor file, `cargo clean -p taarib-khatm`, rebuild, re-sign and
republish everything the registry serves, and only then remove the old keychain
entry. `taarib-isdar` deliberately has no delete subcommand — a tool that can
both mint and destroy signing keys is one wrong argument away from ending the
release identity — so the removal is done with the platform's own credential
manager.

#### What Phase 28 Stage 1 verified, and how

Run on 2026-09-05 on Linux (WSL2), against a *throwaway* release identity minted
by `isdar wallid` under the keychain account `ikhtibar.muaqqat.marhala28` and
destroyed at the end. The owner's real key cannot be minted by anyone but the
owner; everything below is the mechanism around it, proved with a stand-in.

| what | how | result |
| --- | --- | --- |
| mint | `isdar wallid --ism <throwaway> --mirsa <file>` | one 65-byte file written (64 hex + newline); a whole-`$HOME` before/after diff showed the only other file touched was the credential store's own database, `~/.local/share/keyrings/login.keyring` |
| custody | read the keyring database directly | no plaintext seed and no plaintext account name; the only readable tokens are gnome-keyring's MD5 attribute hashes (`md5("taarib.tawqee")`, `md5("malik")`), and the probe account `isdar.jass` was absent, so the reachability probe really does delete itself |
| round trip | `isdar tahaqquq` | the stored key still derives the anchor in the file |
| refusal by name | a package re-sealed under `MIFTAH_TATWIR`, verified by `taarib_aman::tahaqquq_tawqee::tahaqquq` under the test anchor | `SababTawqee::TawqeeTatwir`, and the whole gate `taarib_aman::fahs` refuses with the same sentence. The same package re-sealed by an unrelated key gives `MiftahMajhul` instead — so the development key is named, not merely unknown |
| the interface | `MIRSAT_MALIK.hawiya.wasm()` in both builds | `"isdar"` under the release anchor, `"tatwir"` without it. `main.rs::maalumat_taarib` publishes that string and `maktaba.tsx:1221` draws the warning band only on `tatwir`. The wording is honest: en "Development build — trusts the published development key, not the release key", ar «نسخة تطوير — تثق بمفتاح التطوير المعلن لا بمفتاح الإصدار» |
| the const assertion | four builds, each after `cargo clean -p taarib-khatm` | all four fail at compile time with `E0080`: the development key as the anchor (`malik.rs:120`), the `isdar` feature with the variable unset (`malik.rs:132`), an anchor that is not 64 characters and one with a non-hex digit (both `malik.rs:119`) |
| Phase 26, local half | a package sealed with a run-local key, verified against `MirsatThiqa { miftah: <that key>, hawiya: MIRSAT_MALIK.hawiya }` — the anchor `tilqai_awamir.rs::jahhiz` builds — under a release build | accepted; `fahs` mints the permit. The one-button flow still installs what it just built |
| Phase 26, downloaded half | the development-signed package against `MIRSAT_MALIK`, which is what `tathbeet_awamir.rs:1362` uses | refused, `TawqeeTatwir`. And the locally built patch against `MIRSAT_MALIK` is refused as `MiftahMajhul` — which is exactly why the local install is not anchored there |
| destruction | delete the throwaway entry, then re-read | `isdar mirsa --ism <throwaway>` refuses; the keyring database is back to its original size; the owner's key still reads back `e4260a5f…4cea` |

The tree was swept for key material at the same time: every 64-hex token in all
711 files outside `target/`, `vendor/` and `node_modules/` (817 of them) was run
through Ed25519 and none derives `MIFTAH_TATWIR`, so no file holds the
development *seed*. The only key-shaped 64-hex string in the tree is the
development **public** key quoted in `README.md:154`, which is published by
design. There are no `.pem`, `.key`, `.p12`, `.jks`, `.env` or similarly named
files, no `-----BEGIN` block anywhere, no hard-coded value for
`TAARIB_MIFTAH_ISDAR`, and exactly one `[u8; 32]` key constant —
`MIFTAH_TATWIR`, the public half. `.gitignore` already covers all of those
names; note that it is currently inert, because this working tree is **not** a
git repository and so has no history to scan either.

#### One thing that should change, outside this file's ownership

`taarib_aman::tahaqquq_tawqee::tahaqquq` never reads `mirsa.hawiya`. It returns
`SababTawqee::TawqeeTatwir` whenever the signing key is `MIFTAH_TATWIR` and the
anchor is anything else — and that refusal's own sentence says «وهذه نسخة إصدار
ترفضه بالاسم» / "which a release build refuses by name". Under the Phase 26
local anchor the two can disagree: a *development* build whose anchor is the
run's own key answers a development-signed package with a sentence asserting it
is a release build. `taarib_tahdith::bayan::ihlil` already guards the same
refusal with `matches!(mirsa.hawiya, HawiyatThiqa::Isdar)`; the signature gate
should do the same, after which the package falls through to `MusahimFaqat` or
`MiftahMajhul`, which is the honest answer. One condition in
`crates/taarib-aman/src/tahaqquq_tawqee.rs:106`.

### 7.2 The forge OAuth client id

Submission (`taarib-taqdeem`) uses GitHub's device authorization flow. Two
settings are `None` until the registry operator provisions them, both in
`taarib_usus::idadat::IdadatMasadir`:

| field | what it holds | where it comes from |
| --- | --- | --- |
| `muarrif_amil` | the OAuth **client id**, e.g. `Iv23li…` | github.com → Settings → Developer settings → **OAuth Apps** → New OAuth App. Enable **Device flow**. Scope requested is `public_repo`. No client *secret* is used or stored — the device flow is a public-client flow, which is why this is a client id and not a credential |
| `rabt_tajheez` | the staging upload endpoint template, `{ism}` for the file name | a **staging** release on the registry repository, e.g. `https://uploads.github.com/repos/<owner>/taarib-registry/releases/<id>/assets?name=tajheez-{ism}`. The template must contain `tajheez` and must not contain `isdar`; `IdadatMustawda::tahaqquq` refuses both ways round, so a submitted package can never land in the public release area |

Everything else the transport needs is already compiled in
(`apps/studio/src-tauri/src/taqdeem_awamir.rs::idadat_tawthiq`): the device-code
endpoint `https://github.com/login/device/code`, the token endpoint
`https://github.com/login/oauth/access_token`, the identity endpoint
`https://api.github.com/user`, the scope `public_repo`, and the keychain account
`taqdeem` the token is stored under. The user sets the two fields in
Settings → Sources, or they are shipped as defaults in a release build.

Without `muarrif_amil` the Studio refuses with
`KhataTaqdeemAmr::IrsalGhayrMuhayya { naqis: "muarrif_amil" }` and submission
stays a local handoff that says so.

## 8. What was verified, and how

Two walks, both against a real Steam library and the same game: R.E.P.O. (Steam
3241660, build 23363152), one of 17 installed titles, patched with a container
compiled from that game's own extracted strings. Neither uses a fixture, a stub
or a mock at any rung.

### 8.1 Against a local server — `scratchpad/mustawda_hayy`, 2026-09-04

Casts this layout from a real sealed package, serves it over TCP and TLS from
one directory, and drives an unmodified client through it, with a 45 472-byte
patch.

| step | code exercised | result |
| --- | --- | --- |
| index | `jalb_fahras`, `BayanMustawda::min_bayt`, `ShareehaMuwaththaqa::min_bayt` | manifest #1, 17 of 256 shards fetched and hash-verified; a second fetch hit the cache and no source |
| revocation | `jalb_qaimat_sahb`, `QaimatSahb::min_bayt` | 551 bytes, signature verified, one revoked key found and the publishing key found not revoked |
| match | `MutabiqBina::luba`, `afdal_ruqaa` | `Tamma` — build id and content fingerprint both matched, over 19 containers |
| download | `tanzeel::nazzil` | transfer cut at 16 384 bytes, resumed with `Range: bytes=16384-`; then cut again and completed **from the mirror** at the same offset; an `http` source refused before a socket opened |
| verify | `blake3`, `taarib_aman::tahaqquq_tawqee::tahaqquq` | hash equal to the listing's; signature accepted under the development anchor, refused as `TawqeeTatwir` under a release anchor |
| install | `tathbeet_bilnaqra::thabbit_bilnaqra` | quarantine → `fahs` → permit → `thabbit`; anti-cheat clean, 8 pieces of multiplayer evidence, install verified against its own manifest |
| restore | `taraju::istiada_nass` | 208 files, 1 492 605 039 bytes, **byte-for-byte identical** to the pristine copy |

That walk recorded one gap: the index could not be fetched over the network from
a *local* server, because `masadir::bina_amil` built its own `reqwest::Client`
inside a private `OnceCell` and reqwest's `rustls-tls` compiles in the Mozilla
root set, so a certificate minted for a test could not be trusted and there was
no seam to add one through. It asked for a `SilsilatMasadir::bi_amil`
constructor taking a prebuilt client. **That constructor now exists**
(`crates/taarib-mustawda/src/masadir.rs`), and 8.2 uses it.

### 8.2 Against the live registry — `scratchpad/mustawda_hayy_2`, 2026-09-05

Every rung below reads `https://github.com/cc1a2b/taarib-registry` at
`nashr-2` — manifest sequence 2, all 256 shards, one 44 976-byte patch attached
as a release asset. Nothing in the catalogue is served from this machine. The one
local component is a CONNECT proxy that forwards ciphertext and holds no key,
which exists only so a transfer can be broken on purpose; the client still
completes its own handshake with GitHub and still validates GitHub's certificate.

| step | code exercised | result |
| --- | --- | --- |
| layout audit | this document, field by field | manifest schema 1, sequence 2, all 256 indices declared and present, every shard path passing `masar_salih`, `rabt` and `rabt_mira` both `https` and inside the length and byte-set limits; **all 256 shards fetched over the network and hash-verified**, the 255 empty ones in the 23-byte form |
| index, product client | `SilsilatMasadir::jadida`, `jalb_fahras` | manifest #2 and the 17 shards this library touches, fetched from `raw.githubusercontent.com` and verified against the manifest — **the 8.1 gap closed: no local server anywhere in this path** |
| index, injected client | `SilsilatMasadir::bi_amil`, `jalb_fahras` | the same fetch through a client routed via the proxy: 17 CONNECT tunnels to `raw.githubusercontent.com`, every one on the record, and both fetches agree on the manifest |
| cache | `jalb_fahras` against a chain naming a tag nobody pushed | the chain is genuinely dead (jsDelivr answers 404 for `@la-yujad-abadan`), and all 17 shards still resolve — from the cache, with no source able to answer |
| rollback | `BayanMustawda::min_bayt` | a client that has seen #3 refuses the live #2 |
| revocation | `jalb_qaimat_sahb`, `QaimatSahb::min_bayt` | 323 bytes fetched, accepted under the compiled anchor, 0 entries, the publishing key found not revoked; one flipped byte is refused, and the same bytes under a different valid owner key are refused |
| match | `MutabiqBina::maa_idafat`, `afdal_ruqaa` | 19 containers of the installed game hashed through the package's own recipe → `fdbe7864…9efa`, `Tamma`, installable |
| download, cut | `tanzeel::nazzil` | the proxy cut the origin at 30 000 bytes of ciphertext; 16 384 bytes survived on disk with a sidecar naming the URL, size and hash |
| download, resume | `tanzeel::nazzil` | the retry resumed rather than restarted: the origin sent about 8 479 bytes for a 44 976-byte file, so the `Range` was honoured; the partial file and its sidecar are gone |
| failover | `tanzeel::nazzil` | cut again with the mirror behind the primary — GitHub's release host broke and **jsDelivr carried the rest from the same offset** |
| clear text | `tanzeel::nazzil` | the same asset over `http` is refused before a socket opens; the proxy saw zero tunnels for it |
| verify | `blake3`, `taarib_aman::tahaqquq_tawqee::tahaqquq` | the fetched bytes hash to the listing's `basmat_muhtawa` and are identical to the package that was cast; sealed under the development key, **accepted** under this build's anchor and **refused by name** under a release anchor |
| install | `tathbeet_bilnaqra::thabbit_bilnaqra` | against a copy: quarantine → `fahs` → permit → `thabbit`; anti-cheat clean, 8 pieces of multiplayer evidence, one content file placed, install verified against its own manifest — 2 paths differ from pristine, both the patch's own |
| restore | `taraju::istiada_nass` | 208 files, 1 492 605 039 bytes, **byte-for-byte identical** to the pristine copy; nothing left over |

The install ran against a copy, and the copy was the game: it fingerprints equal
to the installed original. Name, size and mtime of every entry under
`steamapps/common/REPO` — 231 of them — are unchanged from before the walk, and
nothing named `taarib` exists anywhere under the library.

### 8.3 What 8.2 does not cover

The patch it walks is published over its own coverage gate. Its metadata records
`qabila_lil_nashr: false` — no capture session has recorded this game's opening,
so first-hour coverage was never measured — and `bayan.json` carries the
`tajawuzat` record saying so, an optional array `sabk` writes when `--tajawuz`
is used and omits otherwise. **§2 above does not yet describe that field**; its
shape is `fahras::TajawuzNashr` and the caster refuses to publish a package its
gate rejects without one. The walk proves the transport, the verification and
the install; it does not prove that this particular patch is fit for a player,
and the catalogue says so in public.

Voice packs are not exercised: `MuhtawaShareeha::aswat` is empty in every shard
of the live catalogue, so `jalb_sharaih` has parsed the field but nothing has
been fetched, matched or installed through it.

The signing key throughout is `MIFTAH_TATWIR`, the committed development key.
What a release-anchored client does with a *validly release-signed* package is
covered by 8.2 only in the negative — it refuses the development key by name.
