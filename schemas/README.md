# schemas/

Generated JSON Schema, draft 2020-12, one file per vocabulary type. These are
the contract between a Taarib client, the registry repository, and anything a
third party writes against either of them.

Every file here **except one** is produced from the Rust types in
[`crates/taarib-mustalahat`](../crates/taarib-mustalahat) — plus `Khata` and
`RisalatMustakhdim` from [`crates/taarib-usus`](../crates/taarib-usus) — by the
`mukhattatat` binary in that crate. None of them is the source of truth. The
Rust type is.

The exception is [`basmat.schema.json`](basmat.schema.json), which is written by
hand because what it describes is not a Rust type at all: it is the IL2CPP
signature database the Unity adapter reads, an asset shipped as data. It is
listed at the end of the table below and it is the one file in this directory a
person may edit. Everything the rest of this page says about generation, about
`fahras.json`, and about never editing by hand applies to the other files only.

## Why they are committed

The registry is a public Git repository, not a service (Decision 7 in
[`ROADMAP.md`](../ROADMAP.md)), and its whole value depends on being readable
and forkable by someone who has `git clone` and nothing from us. A schema that
only exists after a `cargo build` fails that test. So the schemas are generated
once, committed, and served from the repository at a stable URL:

```
https://raw.githubusercontent.com/cc1a2b/taarib/main/schemas/<file>.json
```

That URL is each file's `$id`. A stored record can name the schema it was
written against, and a validator can fetch it without a Rust toolchain, without
the workspace, and without knowing that Taarib is written in Rust at all.

## Never edit these by hand

A hand edit here survives exactly until the next run of the generator, and until
then it is a lie: the file says one thing and the type that actually parses the
data says another. The registry would accept a record the client then refuses,
which is the worst failure mode this directory has.

The fix for a wrong schema is to fix the Rust type — the field name, the
`#[serde(...)]` attribute, the doc comment — and regenerate.

## Regenerating

```sh
cargo run -p taarib-mustalahat --features mukhattatat --bin mukhattatat
```

The binary resolves the workspace root from `CARGO_MANIFEST_DIR`, so it can be
run from anywhere in the tree. It rewrites every file listed in `fahras.json`
and `fahras.json` itself, prints one line per file it wrote, and then sweeps:
any `<type>.json` in this directory that no current type owns is deleted and
reported with a `removed` line. Run it after any change to a type in
`taarib-mustalahat`, and commit the result in the same commit as the type change
— a schema that lags its type by one commit is a schema that is wrong for one
commit.

**The sweep exists because a rename once published a lie.** `HalatTarjama`
became `HalatMuraja`, gaining `tarjama_aaliya` and `marfuda` and losing
`mujammada`, and an earlier generator that only wrote left `halat_tarjama.json`
sitting here, invisible to the index and still answering at its `$id`,
validating a state no build could read and rejecting two that every current
record used. Deleting the orphan was a step a reviewer had to remember, so it is
now `ihdhif_matruka` in the generator, run unconditionally. The check after
regenerating is still worth doing by eye: the set of `.json` files here is
exactly `fahras.json`'s entries plus `fahras.json` itself plus
`basmat.schema.json`, which the sweep preserves by name.

## What the generator adds

`schemars` produces the body. Two things on top of it are Taarib's:

- **`$id`**, which `schemars` does not emit at all, built from the file name.
- **Key order**: `$schema`, `$id`, `title`, `description`, the type itself, then
  `$defs`. `schemars` appends its metadata after the body, which is correct and
  unreadable; a committed file that reshuffles on every run turns a one-field
  change into an unreviewable diff.

Descriptions are the opening paragraph of the type's own doc comment, with the
source file's hard wrapping collapsed back into one line. The reasoning that
follows the opening paragraph in the Rust source stays in the Rust source: a
`description` is read inside a validator's error message and on a registry
listing, not by someone studying the design.

## The files

`fahras.json` is the index: the title, file name, `$id` and description of every
schema here. Read it first; it is the only file whose shape a consumer has to
know in advance.

The rest are one per type, named as the type is named in snake case, exactly as
`serde` would rename it. `Luba` is `luba.json`, `TaqreerImkaniyat` is
`taqreer_imkaniyat.json`, `MulakhkhasRuqaa` is `mulakhkhas_ruqaa.json`.

| File | Type | What it describes |
| --- | --- | --- |
| `luba.json` | `Luba` | A discovered game: identity, every launcher it is known by, install root, artwork, compatibility layer |
| `luba_id.json` | `LubaId` | Taarib's own UUIDv5 game identity |
| `masdar_luba.json` | `MasdarLuba` | A launcher identity, with the launcher's own identifier preserved |
| `halat_luba.json` | `HalatLuba` | The Arabization badge the library grid puts on a game |
| `hukm_lugha_rasmiya.json` | `HukmLughaRasmiya` | Whether a game's publisher already ships Arabic, with the evidence behind the verdict and what could not be seen |
| `bina_id.json` | `BinaId` | A build: the launcher's build identifier where one exists, plus the fingerprint |
| `basma.json` | `Basma` | A BLAKE3 content fingerprint over the text-bearing files |
| `mutabaqa_bina.json` | `MutabaqaBina` | Which of the three match tiers a patch reached against a build |
| `muharrik.json` | `Muharrik` | An identified engine and the evidence behind the identification |
| `taqreer_imkaniyat.json` | `TaqreerImkaniyat` | The capability report a user reads before installing anything |
| `tabaqa.json` | `Tabaqa` | Injection tier 1, 2 or 3 |
| `mudkhal_nass.json` | `MudkhalNass` | One translatable string with its context, constraints, markup and flags |
| `nass_id.json` | `NassId` | A deterministic string identity |
| `halat_muraja.json` | `HalatMuraja` | Where a translation stands: untranslated, machine output, draft, flagged, approved, rejected |
| `nitaq_nasq.json` | `NitaqNasq` | One markup or placeholder span over clean text |
| `alam_jawda.json` | `AlamJawda` | One thing wrong, or possibly wrong, with a translation |
| `mulakhkhas_ruqaa.json` | `MulakhkhasRuqaa` | The small per-patch record a registry shard carries |
| `ruqaa_id.json` | `RuqaaId` | A patch lineage, stable across revisions |
| `halat_ruqaa.json` | `HalatRuqaa` | Where a submission stands, from draft to published or revoked |
| `tareeqa_tarjama.json` | `TareeqaTarjama` | Human, machine-assisted, or machine-only |
| `rukhsa_ruqaa.json` | `RukhsaRuqaa` | The licence the translated text is published under |
| `taghtiya.json` | `Taghtiya` | Coverage by string, by occurrence, and by first hour of play |
| `musahim.json` | `Musahim` | A contributor: key fingerprint, credit line, standing |
| `musahim_id.json` | `MusahimId` | A contributor identity: a public signing key fingerprint |
| `sumaa.json` | `Sumaa` | Reputation counters derived from the registry's own review history |
| `khata.json` | `Khata` | The error model: code, severity, both sentences, next action, nested cause |
| `risalat_mustakhdim.json` | `RisalatMustakhdim` | The rendered form of an error: code, severity, one sentence, one button |
| `basmat.schema.json` | *(none — hand-written)* | The IL2CPP signature database the Unity adapter reads, as `assets/basmat/basmat.json` ships it |

`basmat.schema.json` is the odd one out in three ways, all of them deliberate:
it is written by hand, it describes an asset rather than a Rust type, and it is
absent from `fahras.json` — the index lists what the generator produced, and
listing a file the generator never wrote would put an entry there that the next
run silently deletes. A consumer looking for it has to be told it exists, which
is what this row is for. It ends in `.schema.json` rather than `.json` for the
same reason: the suffix says at a glance that this file is not one of the
generated pair `<type>.json`.

Two of the generated files are computed rather than stored — `HalatLuba` depends on what the
registry offers right now, and `RisalatMustakhdim` is built per render from a
`Khata` and the user's language — and both still get a file, because both cross
into the frontend on their own and a type an external tool can receive but
cannot validate is a gap that goes unnoticed until it matters.

A type that is only reached from one of these — `SuwarLuba`, `Mustatil`,
`Daleel`, `Ramz`, `BeeatTawafuq` and the rest — is defined under the `$defs` of
each file that reaches it, and has no file of its own. That means one type can
appear in several files, and every copy is generated from the same definition in
the same run.

## Who reads them

- **The registry repository.** Every record it publishes —
  `fahras/<source>/<shard>.json`, `ruqaa/<id>/<revision>.json`,
  `musahimun/<fingerprint>.json` — is validated against the schema for its type
  before it is committed. A record that does not validate is not published.
- **Forks and mirrors.** Anyone running their own registry, an offline mirror,
  or a LAN share validates against the same files, from the same URLs, with no
  tooling from this project.
- **External tools.** A patch checker, a coverage dashboard, a translation
  importer written in any language reads these instead of guessing at the shape
  of a record or reverse-engineering it from an example.

Taarib Studio does not read them at runtime. It has the Rust types, and the
frontend has the TypeScript generated from the same definitions, so a schema
fetched over the network would only be a slower way to ask a question it can
already answer.

## A schema change is a schema version change

Every structure Taarib persists carries the version of the schema it was written
with, in a `mukhattat` field, and reading applies migrations one step at a time
until the value matches the running build. That mechanism lives in
[`crates/taarib-usus/src/mukhattat.rs`](../crates/taarib-usus/src/mukhattat.rs),
and it is not optional for anything in this directory.

So: if a change to a type here changes what an already-written record looks like
— a renamed field, a removed field, a new required field, a narrowed enum, a
retyped value — then it is a new schema version, and it needs a migration step
in the `DhuMukhattat` implementation for that structure before it is merged. A
record written by an older build must still open. A record written by a *newer*
build is refused rather than guessed at, because a partial read silently drops
the fields this build cannot see and then writes them away for good on the next
save.

Additive changes that older data already satisfies — a new optional field, a new
enum variant nothing has emitted yet, a description reworded — do not need a
version bump. Everything else does.

Regenerating this directory without doing that leaves the schemas honest about
the new shape and silent about the old one, which is the failure the versioning
exists to prevent.
