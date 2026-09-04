# الأشكال التقديمية — why Taarib never produces a presentation form

This is the decision a newcomer will most want to undo. It is Decision 1 in
[`ROADMAP.md`](../ROADMAP.md) section 2, it is the reason several things in this
codebase are harder than they look, and it is load-bearing for the entire
product. Read this before proposing a change that touches shaping, the atlas, or
any adapter's draw path.

The short version: **every contextual form, every ligature, and every diacritic
position in Taarib comes from the font's own OpenType `GSUB` and `GPOS` tables
through a real shaping engine. There is no code path that maps a codepoint to a
Unicode Presentation Form, and no fallback that does so when something goes
wrong.**

---

## 1. What a presentation form is, and why it looks like the answer

Unicode blocks U+FB50–U+FDFF (Arabic Presentation Forms-A) and U+FE70–U+FEFF
(Arabic Presentation Forms-B) contain codepoints for Arabic letters in their
positional shapes. U+0628 is ARABIC LETTER BEH — the letter. U+FE92 is ARABIC
LETTER BEH MEDIAL FORM — the same letter, pre-shaped for the middle of a word.

That block exists for round-trip compatibility with legacy codepages. It was
never intended as a way to write Arabic. But it is enormously tempting, because
it converts a hard problem into a lookup table:

1. Walk the string. For each letter, look at its neighbours.
2. Decide isolated, initial, medial or final from a small table.
3. Emit the presentation codepoint for that shape.
4. Reverse the string so it reads right to left.
5. Hand the result to the game as ordinary left-to-right text.

Five steps, no shaping engine, no bidirectional algorithm, no font tables. It
produces something that looks like Arabic in a screenshot. It is what almost
every Arabic game patch in existence is built on, and it is why Arabic in
modding communities has a reputation for looking slightly wrong in a way nobody
can name.

## 2. What that pipeline actually throws away

The lookup table in step 2 knows one thing: this letter and its two neighbours.
A real font knows considerably more, and expresses all of it in tables the table
cannot see.

**Required ligatures (`rlig`).** Lam-alef is not decorative. ل + ا must render
as لا; a reader does not accept the two letters drawn separately, and the
combination has no letter codepoint to enumerate — it exists only as a
substitution rule. Presentation Forms-B does contain four lam-alef forms, so the
simplest case survives. A Naskh face carries hundreds of further required
ligatures, and none of those have codepoints at all.

**Contextual alternates (`calt`).** This is the difference between a Naskh font
and something that merely uses Naskh's letters. The vertical stacking, the
alternate tooth widths, the way a `jeem` descends differently after a `meem` —
all of it is `calt`, all of it fires on context the table never examines, and
none of it can be addressed by a codepoint.

**Mark attachment (`mark` and `mkmk`).** A diacritic is a combining mark
positioned by the font's own anchor tables, against the specific glyph it lands
on, and stacked against other marks when several share a base. Presentation
Forms has nothing for this. A pipeline that produces them either drops the
diacritics or leaves them to float at the renderer's default advance, where they
collide with the letter or land on the wrong one.

**Cursive attachment (`curs`).** Where the baseline of a joined pair actually
meets. Fonts that model a real pen use it heavily.

**Language-specific forms (`locl`).** Persian and Urdu share the Arabic script
and share almost none of its joining behaviour or ligature set. Persian `heh`
joins differently. Urdu Nastaliq is a wholly different arrangement of the same
letters. Neither is representable in Presentation Forms at all — the block was
built for Arabic, and only for a subset of it. A presentation-form product can
never grow beyond one language.

Every one of these is lost silently. The output is still a valid Unicode string,
so nothing errors, nothing warns, and the loss is only visible to someone who
reads the script.

## 3. The result is text that lies about what it is

This is the part that matters more than the rendering, and the part that is
easiest to miss when the screenshot looks acceptable.

A string of presentation forms is not Arabic text. It is a picture of Arabic
text encoded as characters. Concretely:

- **It does not search.** A player typing a word into an inventory filter will
  not match it, because the stored string contains different codepoints from the
  ones a keyboard produces.
- **It does not sort.** Any collation is against the wrong values.
- **It does not copy.** Pasting it into a browser or a chat window yields
  something that renders inconsistently and cannot be edited.
- **It is unreadable to a screen reader**, and no accessibility technology can
  recover the original from it.
- **Its numbers are backwards.** Step 4 above — reversing the string — is not
  the bidirectional algorithm. Arabic runs right to left, but digits inside it
  run left to right, embedded Latin runs left to right, punctuation takes its
  direction from context, and brackets mirror. A blanket reverse renders `250`
  as `052`. In a game full of item counts, damage numbers, percentages and
  timers, that is not an edge case; it is constant, and it is the single most
  common visible defect in existing Arabic patches.
- **It is not stable input.** Feed it back into any tool — translation memory, a
  glossary, a diff, another patch — and it compares unequal to the same sentence
  written normally.

The practice survived long enough to become a habit precisely because it looks
approximately right to a sighted reader glancing at a screenshot, and every one
of these failures is invisible there.

## 4. What Taarib does instead

`crates/taarib-saff` runs a fixed pipeline, in this order, and skips no stage:

| module | stage |
| --- | --- |
| `nasq` | markup and format placeholders out of the text, into spans and atoms |
| `ittijah` | the full Unicode Bidirectional Algorithm — paragraph level, embeddings, overrides, isolates, weak and neutral resolution, BD16 bracket pairs |
| `taqtee` | UAX #14 line break opportunities, computed on the *logical* text |
| `wasl` | shaping through HarfRust, one call per direction-and-style run |
| `qiyas` | measurement and reflow from real shaped advances |
| `kashida` | justification by elongation, ranked from real joining behaviour |
| `tashkeel` | the diacritic guarantee across every stage |
| `rasm` | rasterization through skrifa |

Shaping happens in `wasl`, and the module header states the contract directly
(`crates/taarib-saff/src/wasl.rs`):

> There is no table in this file mapping a codepoint to a presentation form, no
> fallback that draws isolated letters when a font disappoints, and no path by
> which a codepoint survives past this module.

The order is not stylistic. Break opportunities are a property of logical text;
joining is a property of shaped runs; elongation is a property of joining.
Computing any of them out of sequence produces an answer that is wrong in a way
that only shows up on real sentences.

## 5. How the refusal is made structural rather than promised

A rule in a comment is worth what the code makes true. Four mechanisms carry
this one.

**The type system removes the parameter.** From the layout stage onward there is
no function anywhere in the product that accepts a codepoint for drawing. What
leaves `wasl` is `HarfMashkul`, and a `HarfMashkul` has no field that could hold
a character. What reaches an adapter is `Harf` — a glyph identifier, an x, a y,
an advance, a cluster index, a style span id. A codepoint cannot reach a draw
call because the draw call has no argument for one. `crates/taarib-saff/src/natija.rs`
notes that this is deliberate: nothing downstream can be tempted to draw from a
character, because no character is there to draw from.

**The atlas is keyed by glyph, never by character.** `MiftahShakl` in
`crates/taarib-lawha/src/khareeta.rs` is `(font index, glyph id, pixel size,
rasterization mode, subpixel bucket)`. An atlas keyed by codepoint could hold
one image per character, which is another way of saying it could not hold
Arabic — no lam-alef, no contextual alternate, no positioned mark.

**The glyph set comes from shaping, never from a range.**
`crates/taarib-lawha/src/tafrigh.rs` collects the glyphs a patch needs by
shaping the patch's real strings at the real sizes the game draws them. The
module header explains that range enumeration is wrong in both directions at
once: it *misses* every ligature and contextual form (which have no codepoints
to enumerate), and it *includes* tens of thousands of images for a repertoire
that uses a few hundred. Note that the Arabic Presentation Forms blocks are
themselves listed there as something a naive range enumeration would sweep in —
so even the accidental route to a presentation form is closed.

**Presentation forms are read in exactly one place, to destroy them.**
`lugha::hall_ashkal_taqdimiya` in `crates/taarib-saff/src/lugha.rs` is the only
function in the product that looks at U+FB50–U+FDFF or U+FE70–U+FEFF. It exists
because translators paste text from legacy tools and what they paste *looks*
like Arabic. It decomposes those characters back to canonical form immediately,
through `icu_normalizer`'s NFKD restricted to those two blocks, then recomposes
with NFC. Canonical characters are what leaves. Nothing writes back into those
blocks anywhere.

The restriction matters: NFKD over the whole string would also flatten
superscripts, fullwidth Latin, and every other compatibility character present —
a different transform that nobody asked for.

## 6. What it costs, said plainly

Refusing the shortcut has a price, and pretending otherwise would be the same
dishonesty this document is about.

**Shaping cannot be skipped, so a font without the tables is a hard failure.**
HarfRust has no Arabic fallback shaper. A font lacking `GSUB` with the Arabic
joining features does not produce ugly output; it produces nothing, or isolated
letters, silently. Decision 6 turns that into a loud failure: every bundled font
is validated at build time and again at load time, and a user-supplied font is
rejected with the specific missing table or feature named.

**Every patch carries an atlas.** Because glyphs are addressed by glyph id and
rasterized from Taarib's own fonts, the patch has to ship the images. A
presentation-form patch ships text and nothing else. This is the direct cost of
Decision 5 as well — Taarib never touches the game's font assets — and it is
what makes the rendering path survive engine version bumps that break every tool
built on rewriting asset bundles.

**Some engines cannot be handed glyphs at all.** Godot 3 draws through
`Font::draw_char` with no shaping and no reordering. GameMaker's font resources
are addressed by character code. For those, `crates/taarib-lawha/src/naql.rs`
provides a transport: real HarfRust shaping runs first on the real logical text,
and the resulting glyph identifiers are assigned private-use codepoints in a
generated glyph table whose entry for each slot is Taarib's own rasterized image
of that exact glyph.

That module's header exists to answer the objection this document invites, and
it should be read in full before anyone concludes it is a presentation-form
pipeline wearing a different name. The distinction is not a matter of degree:

- A presentation-form pipeline **maps codepoints to codepoints**, and to do it
  must throw the shaping away and re-derive it from a table that cannot see the
  font.
- The transport **maps already-shaped glyph identifiers to opaque slots**.
  Nothing re-derives anything. Every ligature the font formed is one glyph id
  and gets one slot. The private-use codepoints carry no Unicode meaning of any
  kind; they are integers in an address space the standard reserves for exactly
  this, meaningful only to the one glyph table generated in the same build.

The invariants that keep that true are structural, not promised: the slot key
has no character field, there is no inverse from a slot into text, a transported
sequence's `Debug` prints `U+F0000` rather than characters so nobody can copy
one out of a log and mistake it for Arabic, and a slot cannot be an ill-formed
scalar.

And `naql.rs` names its own losses rather than hiding them. A transported string
is not searchable, not selectable, not readable by a screen reader, cannot be
re-shaped at another size, and cannot be concatenated. Which is exactly why that
rung is the last one in the Godot 3 ladder and not the first, why the capability
report tells the user which path was taken before they install, and why it is
called a transport everywhere it appears.

## 7. If you still want to change this

You would be reopening Decision 1, which is settled and which the codebase is
built around. Concretely, a change would have to:

- add a public constructor somewhere that lets a codepoint into a draw path —
  which does not exist and is a visible addition in review;
- re-key the atlas by character, which breaks every ligature and every
  positioned mark;
- replace `tafrigh`'s shaping-derived glyph set with a range enumeration, which
  is wrong in both directions as its own header explains;
- and accept that the product can never render Persian or Urdu, that its output
  is unsearchable and uncopyable, and that its numbers are backwards.

The gain would be a smaller patch file and no font validation step.

That trade was made once, deliberately, at the foundation, and the reason it is
written down in this much detail is so that the next person to consider it can
see the whole cost rather than only the part that shows in a screenshot.
