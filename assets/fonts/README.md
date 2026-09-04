# Bundled fonts

Taarib rasterizes its own glyphs from its own fonts. It never reads, modifies, or
extends a game's font assets — not a Unity `TMP_FontAsset`, not an Unreal font
object, not a GameMaker glyph page. That rule is architectural decision 5 in
[`ROADMAP.md`](../../ROADMAP.md), and the fonts in this directory are what makes
it possible.

## Why these families

Every font here was chosen for a reason that survives being questioned:

| Family | Class | Used for | Why it is here |
| --- | --- | --- | --- |
| IBM Plex Sans Arabic | sans | The Taarib Studio interface | Drawn for interface work, not display: low stroke contrast, open counters, legible at 11px, with a Latin companion that shares its vertical metrics |
| IBM Plex Sans | latin | The interface's Latin | Same family, same metrics, so a bilingual line does not step |
| IBM Plex Mono | mono | Identifiers, paths, hashes, build numbers | A slashed zero and unambiguous `1 l I`, which matters when a user reads a build id back to a maintainer |
| Noto Naskh Arabic | naskh | The default for game text | Complete mark and mark-to-mark positioning, and — with Noto Sans Arabic — the broadest Arabic-script coverage bundled: all of `U+0600-06FF`, the Supplement, and both Extended blocks |
| Amiri | naskh | Narrative and dialogue | A substitution table deep enough that it demonstrates what real shaping does, and the only bundled family with GPOS cursive attachment — letters are positioned onto each other, not merely drawn to meet |
| Noto Kufi Arabic | kufi | Interface chrome inside games | Geometric and low-contrast; the heavy weights hold up in headings |
| Reem Kufi | kufi | Titles and headings | A modern Kufi that genuinely joins — four joining forms and a required-contextual pass — so it can carry a sentence and not only a word |
| Noto Sans Arabic | sans | Constrained interfaces | Its width axis lets an overflowing string be condensed by a designed narrow instance instead of being scaled |
| Cairo | sans | Display treatments | A slant axis and a weight range wide enough to answer a game built around a heavy Latin face |
| Tajawal | sans | Tight interface boxes | Short ascenders, so Arabic fits layouts that were measured for Latin |

All ten are licensed under the SIL Open Font License 1.1, which permits bundling
and redistribution inside an application. The licence text ships in the same
directory as the faces it covers, and is included in every platform package;
which file covers which family is `malaf_rukhsa` in `khutut.json`.

## The validation rule, and why it is absolute

HarfRust has **no Arabic fallback shaper**. Nothing in the pipeline synthesises
joining behaviour when a font does not carry it. A font missing `init`, `medi`,
`fina` and `rlig` does not produce ugly Arabic — it produces isolated
letterforms or nothing at all, silently, and the user experiences that as "the
patch is broken" with no way to find out why.

`isol` is not on that list, and the omission is deliberate. A `GSUB` feature
replaces the nominal glyph, and for an Arabic letter the nominal glyph — the one
the `cmap` yields — already is the isolated form. A font defines `isol` only
when its isolated form differs from that default, so requiring the tag tests an
encoding choice rather than a capability. Two professionally commissioned
families, IBM Plex Sans Arabic and Dubai, omit it and shape correctly; requiring
it rejected both, one of which is the face this product renders its own
interface in.

Reading the feature lists of the pinned files settles it beyond those two:
**not one of the twenty-five faces here declares `isol`** — not one of the
nineteen Arabic ones, and they all shape. A rule that rejects every font a
project ships was never testing a capability.

So every font is checked three times, and the three checks answer different
questions:

1. **At staging time**, by `taarib-tajmee`, against `assets/aqfal/qufl_khutut.json`.
   This is an identity check, not a capability check: each file is fetched from
   the URL the lock pins, hashed with SHA-256, and refused unless the digest is
   the one the lock records. Nothing here parses the font. The digests of
   everything staged are then written to `bayan_mukawwinat.json` beside the tree.
2. **At bundling time**, by `apps/studio/src-tauri/tadqiq_mawarid.mjs`, which
   Tauri runs as `build.beforeBundleCommand` — between the compile and the
   bundler, so it cannot be skipped by a build that produces an installer. It
   re-checks every lock digest against the staged bytes, then parses each face's
   sfnt table directory itself and refuses the build if a face is missing a
   table, a feature or a character the rule above requires. It also holds this
   manifest to the file: **every tag a family declares in `sifat` must be in the
   font's own `GSUB`/`GPOS` feature list**, so a `khutut.json` that has drifted
   from the bytes stops a release rather than being discovered by a user.
3. **At load time**, by `taarib-saff`'s `khatt` module, against the same
   requirement. This is what also covers fonts a user imports, which no
   packaging step ever saw. A font that fails is rejected with a message naming
   the missing table or feature — never accepted with a warning, and never
   silently substituted.

Check 2 is why the `sifat` arrays below are not decoration. They were wrong for
a long time — seven families declared `isol`, eight declared `calt`, Noto Naskh
Arabic declared `curs` — and once the gate existed, that manifest produced
fifty-three refusals and no installer at all. They are now read out of the
pinned files. If you bump a pin, re-read them; the gate will tell you if you
did not, but it will tell you by failing the release build.

User-supplied fonts pass the identical check on import. A font that fails is
refused with the same specific reason, because "this font cannot render Arabic
because it has no GSUB table" is a sentence a user can act on and "something went
wrong" is not.

## What `khutut.json` records

For each family: its identifier, display family name, class, whether it is
variable and on which axes, its weights, the file for each weight and style, the
OpenType features it carries, the Unicode ranges it covers, the languages it is
correct for (Persian and Urdu need their own `locl` forms of `kaf`, `yeh` and
`heh`, which not every Arabic font provides), its licence and licence file, its
upstream source and version, and the reason it is bundled.

`sifat` is deliberately not an exhaustive dump of the font's feature list. It
holds the intersection of two sets: the tags `taarib-saff`'s shaping plan
actually drives (`SIFAT_WASL_KAMILA` in `wasl.rs`, plus `clig`), and the tags
**every** face of the family declares. Both halves matter. A face's `frac`,
`ordn` and `sups` are real and irrelevant — nothing in Taarib ever asks for
them, so listing them would bury the ones that decide whether a line shapes. And
a family-level promise has to hold for every weight in it: Tajawal declares
`calt` in Light, Medium and Bold and not in the other four, so Tajawal does not
list `calt`.

`isdar_khatt` is the face's own version, read from name ID 5 of the pinned file
— not the upstream repository's release tag, which is recorded in the lock
instead. The two are usually different and occasionally very different: the
commit `qufl_khutut.json` pins for Reem Kufi carries face version 2.000, and the
one for Cairo carries 3.130.

Note that several families list coverage of the Arabic Presentation Forms blocks
(`U+FB50-FDFF`, `U+FE70-FEFF`). That coverage is **irrelevant to Taarib**: those
codepoints are a legacy compatibility encoding, Taarib never requests them, never
produces them, and only ever *decomposes* them back to canonical characters when
a translator pastes text from a legacy source. The ranges are recorded because
they describe the font, not because anything uses them.

## Vendoring

No font binary is committed to this repository, and `assets/fonts/` never holds
one. What is committed is this README, `khutut.json`, and — one directory over —
`assets/aqfal/qufl_khutut.json`, which is the only thing that decides which
bytes are a Taarib font. Thirty-four files are pinned there: twenty-five faces
and nine licence texts.

Two tools read that lock and neither writes here:

- `cargo run -p taarib-tajmee -- --hadaf <triple> --jalb` fetches all thirty-four
  and stages them at `apps/studio/src-tauri/mawarid/khutut/`, in the class
  directories `wajha` names. That tree is what a bundle carries and what
  `mukawwinat_tahmil.rs` walks at runtime:

  ```
  mawarid/khutut/
    naskh/   NotoNaskhArabic[wght].ttf  Amiri-*.ttf        OFL.txt  Amiri-OFL.txt
    kufi/    NotoKufiArabic[wght].ttf   ReemKufi[wght].ttf OFL.txt  ReemKufi-OFL.txt
    sans/    IBMPlexSansArabic-*.ttf    NotoSansArabic[wdth,wght].ttf
             Cairo[slnt,wght].ttf       Tajawal-*.ttf
             IBMPlex-OFL.txt  OFL.txt  Cairo-OFL.txt  Tajawal-OFL.txt
    latin/   IBMPlexSans-*.ttf  IBMPlexMono-*.ttf  IBMPlex-OFL.txt
  ```

  Note what is *not* in it: `khutut.json` itself. The runtime does not need it —
  it walks the tree and asks each file what it is — so the manifest stays a
  source document rather than a shipped one.

- `python3 scripts/ijlib_khutut.py` fetches the eight IBM Plex faces the
  interface's own CSS names, from the same lock, into
  `apps/studio/src/khutut/`, where Vite fingerprints them into the webview
  bundle. Those eight are committed; see `assets/NOTICES.md` for why that is
  not a second, unlicensed copy.

The hashes do not live in this manifest. `khutut.json` describes the faces;
`assets/aqfal/qufl_khutut.json` pins the bytes, one entry per file, each with
the URL it is fetched from, its length and its SHA-256. Keeping the two apart is
what lets `wajha` in the lock be literally the `malaf` value here, so the staged
tree and the manifest cannot drift into describing different files. A file whose
digest does not match its lock entry is a staging failure — named, with both
digests — rather than a runtime surprise.

### Where each file comes from

Every `rabt` in the lock is a `raw.githubusercontent.com` URL at a **fixed
commit**, never a release tag and never a branch. Two commits cover all thirty-four
files:

| Files | Pinned at |
| --- | --- |
| The nine IBM Plex faces, and two copies of `LICENSE.txt` | `IBM/plex` at `242c4ccc`, which is release `v6.4.2` |
| The other sixteen faces, and seven `OFL.txt` files | `google/fonts` at `205859f6` |

That second row is the part worth reading twice. Amiri, Reem Kufi and Cairo are
developed at `aliftype/amiri`, `aliftype/reem-kufi` and `Gue3bara/Cairo`, and the
Noto Arabic families at `notofonts/arabic` — but Taarib does not take them from
there. It takes the copies Google Fonts has already vetted and republished under
`ofl/<family>/`, so one commit hash covers seven families and the licence text
that must travel with each. `masdar` in `khutut.json` still names the design
project, because that is where a maintainer goes to read about the font; the
lock names where the bytes came from, because that is what has to be
reproducible.

The faces taken, per family:

| Family | What the lock takes |
| --- | --- |
| IBM Plex Sans Arabic | The `ttf` static instances at 400, 500 and 600 |
| IBM Plex Sans | Regular, Italic, Medium, SemiBold |
| IBM Plex Mono | Regular and Medium |
| Noto Naskh Arabic, Noto Kufi Arabic, Noto Sans Arabic | The variable `ttf`, one per family |
| Amiri | Regular, Italic, Bold, BoldItalic |
| Reem Kufi, Cairo | The variable `ttf` |
| Tajawal | Seven static weights, 200 through 900 |

Nine licence files cover the ten families. Each of the three class directories
would receive two or three files upstream-named `OFL.txt`, so the lock's `wajha`
renames one per collision — `Amiri-OFL.txt` in `naskh/`, `ReemKufi-OFL.txt` in
`kufi/`, `Cairo-OFL.txt` and `Tajawal-OFL.txt` in `sans/` — and `malaf_rukhsa`
in `khutut.json` is the authority on the final name. The one that survives
unrenamed in each directory belongs to the Noto family there. `IBMPlex-OFL.txt`
is fetched twice, once into `sans/` and once into `latin/`, because a directory
that ships faces has to ship the licence beside them; the three Plex families
resolve to the same upstream `LICENSE.txt`, which is why nine files discharge
ten families.

Bumping a font version is a deliberate act: a new version can add glyphs, change
advance widths, or reorder glyph identifiers, and every one of those invalidates
the precomputed layouts and glyph atlases inside already-published patches. The
version in `khutut.json` is therefore part of a patch's compatibility identity,
not an implementation detail.
