// النسق — the markup bridge: a game's rich text taken apart before anything
// directional is allowed to look at it.
//
// A game does not store the string it draws. It stores
// `<color=#ff0000>{0}</color> أصاب <b>%s</b>` and expects its own renderer to
// turn that into a red number, a name in bold, and Arabic in between. This file
// turns that raw string into two things the rest of the takeover can use: clean
// text with no markup left in it at all, and a table of style spans whose
// offsets index that clean text.
//
// WHY THE JOB IS SHAPED THIS WAY. There are three obvious approaches and two of
// them are wrong in ways that only show up on the strings a patch is for:
//
//   - Shape the raw string with the tags still in it. The bidirectional
//     algorithm then does exactly what it is defined to do: `<` and `>` are
//     Other Neutrals, `=` is a Common Separator, `#` is a European Terminator,
//     and `f` and `0` are a Left-to-Right letter and a European Number. The tag
//     is not a tag to the algorithm — it is nine characters of mixed
//     directional class sitting inside a right-to-left paragraph, and rule L2
//     reverses the runs it lands in. `<color=#ff0000>` comes back out as
//     `>0000ff#=roloc<`, or split in two, or with its digits moved relative to
//     its `#`, and the tag ends up in the middle of a word. Every
//     mixed-direction string the game owns is then corrupted in a way that
//     looks, from outside, like the Arabic itself is broken.
//   - Strip the tags and keep nothing. Every colour, size and weight in the
//     game is silently gone, which reads as "rich text stopped working".
//   - Strip the tags and keep the spans as CHARACTER ranges over the ORIGINAL
//     string. Those offsets name positions in a string that no longer exists,
//     and they stop pointing at the right glyphs the instant reordering runs.
//
// What survives all three failures is the arrangement below: strip to clean
// text, record every span as a BYTE range over the clean UTF-8, and let the
// layout engine carry the span identifier on every glyph so colour follows the
// glyph after it has been moved. That last part is not this file's to do —
// TaaribHarf.Nitaq is the field, Nasij reads it, and this file is where the
// number it carries is assigned.
//
// WHY BYTES AND NOT CHARACTERS. Every stage downstream of here — shaping,
// bidi, cluster mapping, TaaribHarf.Anqud — is expressed in UTF-8 byte offsets
// into the text handed to the layout call. A span table in UTF-16 code units
// agrees with all of them for ASCII and disagrees for every Arabic string in
// the patch, which is the exact set of strings this product exists for.
//
// WHY AN ATOM IS EXACTLY ONE U+FFFC. A format placeholder is not text and it is
// not nothing: `{0}` becomes `147` or `3` at runtime, a width the layout cannot
// know and a direction it must not guess. So each one occupies exactly one
// U+FFFC OBJECT REPLACEMENT CHARACTER in the clean text — one position for the
// line breaker and the reordering pass to reason about, an Other Neutral
// directional class so the atom takes the direction of whatever surrounds it
// rather than imposing one, and a codepoint no real font draws, so no later
// stage can shape it into something visible by forgetting to skip it.
//
// WHAT ALLOCATES. Nothing on the per-frame path. Hallil writes into spans the
// caller owns and its only internal state is a fixed-size open-tag stack in
// stack memory. The allocating members are named as such on each one, and all
// of them belong to the editor, the compiler and the log — never to a frame.
//
// WHAT THIS FILE REFUSES TO REFUSE. Malformed markup is not an error here. A
// shipped game's string table contains unclosed tags, stray `<`, and tags no
// parser ever knew; the game draws all of it without complaint because its own
// parser is lenient in exactly these ways. Matching that leniency is the whole
// point — see the Nasq class for the rule per case and for why refusing would
// put a patch in front of a player with less text on screen than the game had.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mushtarak
{
    /// <summary>
    /// Which markup dialect a string is written in. One per string, chosen by
    /// the component the string came from, because the dialects genuinely
    /// conflict and guessing between them invents markup rather than finding
    /// it.
    /// </summary>
    /// <remarks>
    /// The clearest conflict is NGUI against everything else: NGUI reads
    /// <c>[FF0000]</c> as a colour, and a string that says <c>[FF0000]</c> in
    /// any other dialect is six characters a player is meant to read. Parse one
    /// as the other and either a colour survives as literal text or six
    /// characters of the game's own writing disappear.
    /// </remarks>
    public enum NawNasq : uint
    {
        /// <summary>
        /// No dialect. Tags are not looked for and every <c>&lt;</c> is a
        /// character; only format placeholders are lifted, because those are
        /// the game's formatter's business rather than any renderer's. The
        /// correct choice for a plain string table and for any component whose
        /// rich-text switch is off.
        /// </summary>
        Bila = 0,

        /// <summary>
        /// TextMeshPro's rich text — the largest dialect here, and the only one
        /// with <c>&lt;noparse&gt;</c>, <c>&lt;sprite&gt;</c>,
        /// <c>&lt;link&gt;</c>, percentage and relative sizes
        /// (<c>&lt;size=120%&gt;</c>, <c>&lt;size=+2&gt;</c>) and the bare
        /// colour form <c>&lt;#RRGGBB&gt;</c>.
        /// </summary>
        TextMeshPro = 1,

        /// <summary>
        /// Unity's legacy UI text, which parses a strict subset:
        /// <c>&lt;b&gt;</c>, <c>&lt;i&gt;</c>, <c>&lt;size=n&gt;</c>,
        /// <c>&lt;color=…&gt;</c>, <c>&lt;material=n&gt;</c> and
        /// <c>&lt;quad …/&gt;</c>, and nothing else at all. What distinguishes
        /// it is what it does with the rest: a <c>&lt;sprite=3&gt;</c> is not a
        /// sprite here, it is eleven characters the component draws.
        /// </summary>
        WajihatUnity = 2,

        /// <summary>
        /// NGUI's bracket markup: <c>[RRGGBB]</c> pushes a colour,
        /// <c>[-]</c> pops it, and <c>[b]</c>, <c>[i]</c>, <c>[u]</c>,
        /// <c>[s]</c>, <c>[c]</c> and <c>[url=…]</c> are the named tags. The
        /// colour form is exactly six hex digits with no <c>#</c>, which is why
        /// this dialect must never be enabled for a string that is not NGUI's.
        /// </summary>
        NGui = 3,

        /// <summary>
        /// FairyGUI's HTML-ish subset, distinguished by putting its styling in
        /// quoted attributes on a <c>&lt;font&gt;</c> element —
        /// <c>&lt;font color='#ff0000' size='20'&gt;</c> — and by writing its
        /// inline images as void elements, <c>&lt;img src='icon'/&gt;</c>.
        /// </summary>
        FairyGui = 4,

        /// <summary>
        /// Generic HTML as an engine-agnostic web view or an in-game browser
        /// parses it. What distinguishes it from the others is that character
        /// entities are real here — <c>&amp;amp;</c> is one character of clean
        /// text — and that colour arrives inside a CSS declaration on a
        /// <c>style</c> attribute rather than as a tag of its own.
        /// </summary>
        Html = 5,
    }

    /// <summary>
    /// What an atom stands for. Every one of these is opaque: never shaped,
    /// never reordered internally, never broken across a line.
    /// </summary>
    public enum NawDharra : uint
    {
        /// <summary>
        /// A format placeholder — <c>{0}</c>, <c>{name}</c>, <c>%s</c>,
        /// <c>%1$d</c>. The game substitutes into it after Taarib has run, so
        /// its characters must reach the game's formatter in the order and the
        /// spelling they were written in.
        /// </summary>
        Mawdi = 0,

        /// <summary>
        /// An inline sprite or icon — <c>&lt;sprite=3&gt;</c>,
        /// <c>&lt;img src='x'/&gt;</c>. It occupies a position and a width, and
        /// the engine draws it from its own asset table.
        /// </summary>
        Sura = 1,

        /// <summary>
        /// A run the dialect was told not to interpret —
        /// <c>&lt;noparse&gt;</c> and each dialect's equivalent. Unlike the
        /// other two it keeps its own text in the clean string rather than
        /// collapsing to one replacement character, because the game means that
        /// text to be read; what it does not keep is any right to be reordered
        /// or shaped.
        /// </summary>
        Khaam = 2,
    }

    /// <summary>
    /// What one reinsertion record put back. Read only by the two rebuild
    /// paths, and the reason they can tell an opening tag from a closing one
    /// without re-parsing anything.
    /// </summary>
    public enum NawSijill : uint
    {
        /// <summary>
        /// The opening tag of the span named by <see cref="SijillNasq.Nitaq"/>.
        /// </summary>
        Iftitah = 0,

        /// <summary>The closing tag of that span.</summary>
        Ikhtitam = 1,

        /// <summary>
        /// An atom's own raw text, standing over the one replacement character
        /// it became — or, for <see cref="NawDharra.Khaam"/>, over the run it
        /// kept.
        /// </summary>
        Dharra = 2,

        /// <summary>
        /// An escape that became a literal character: <c>{{</c> to <c>{</c>,
        /// <c>%%</c> to <c>%</c>, <c>&amp;amp;</c> to <c>&amp;</c>. Carried
        /// because rebuilding the original has to put the doubled form back;
        /// emitting the single character would produce a string that formats
        /// differently from the one the game shipped.
        /// </summary>
        Harf = 3,

        /// <summary>
        /// Raw text that was removed and belongs to no span — a line break tag,
        /// an alignment tag, a tag whose effect this file does not model. It
        /// left nothing behind and it goes back exactly where it was.
        /// </summary>
        Munfarid = 4,
    }

    /// <summary>
    /// Flags on <see cref="NatijaNasq.Alam"/>: what the scan met, so a caller
    /// can act on it without re-reading the string.
    /// </summary>
    /// <remarks>
    /// The four leniency flags are reported and never thrown. They exist so
    /// that the compiler's review pass and the in-game diagnostics can count
    /// the strings a game ships with broken markup, while the runtime path goes
    /// on rendering them the way the game itself does.
    /// </remarks>
    public static class AlamatNasq
    {
        /// <summary>Nothing was extracted; the clean text is the input.</summary>
        public const uint NaqiAsasan = 1u << 0;

        /// <summary>At least one atom is present.</summary>
        public const uint FihDharrat = 1u << 1;

        /// <summary>At least one span sets a colour.</summary>
        public const uint FihAlwan = 1u << 2;

        /// <summary>
        /// A tag was opened and never closed. Closed at end of string, which is
        /// what every dialect here does.
        /// </summary>
        public const uint WasmMaftuh = 1u << 3;

        /// <summary>
        /// A closing tag arrived with nothing open to match it. Dropped.
        /// </summary>
        public const uint WasmMughlaqZaid = 1u << 4;

        /// <summary>
        /// A tag's attribute value could not be read — <c>&lt;color=#gg&gt;</c>
        /// and its kind. The tag was left in the clean text as literal
        /// characters, which is what the game's own parser does with it.
        /// </summary>
        public const uint QeemaTalifa = 1u << 5;

        /// <summary>
        /// A tag name this dialect does not know. Left as literal text.
        /// </summary>
        public const uint WasmMajhul = 1u << 6;

        /// <summary>
        /// The open-tag stack, the span table or an output span filled up and
        /// the rest of the string was scanned with that kind of output no
        /// longer being recorded. Never a refusal; see
        /// <see cref="NatijaNasq.Kafa"/> for the capacity negotiation.
        /// </summary>
        public const uint Farada = 1u << 7;

        /// <summary>
        /// A <c>&lt;noparse&gt;</c> run, or a dialect's equivalent, is present.
        /// </summary>
        public const uint FihKhaam = 1u << 8;
    }

    /// <summary>
    /// One piece of raw markup that was removed, and everything needed to put
    /// it back. This struct is the whole of the reinsertion record; nothing
    /// else about the parse is consulted by either rebuild path.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Two units live in this struct and they are not interchangeable.</b>
    /// <see cref="MawdiNaqi"/> and <see cref="Badeel"/> are UTF-8 <em>byte</em>
    /// offsets and lengths into the clean text, because that is the unit every
    /// span and every glyph cluster downstream is expressed in.
    /// <see cref="AslBidaya"/> and <see cref="AslTul"/> are UTF-16
    /// <em>character</em> offsets into the caller's own raw string, because
    /// that string is a <c>ReadOnlySpan&lt;char&gt;</c> the caller still owns
    /// and never a copy this file made. The names are deliberately different so
    /// no call site can pass one where the other belongs; they agree for ASCII
    /// and disagree for every string this product exists to translate, which is
    /// the worst possible way for a unit mistake to behave.
    /// </para>
    /// <para>
    /// Nothing here holds a string. The raw bytes stay in the caller's original
    /// span and this record names a window into it, which is what lets the
    /// whole parse run without an allocation.
    /// </para>
    /// </remarks>
    public readonly struct SijillNasq
    {
        /// <summary>Builds one record.</summary>
        /// <param name="naw">What was removed.</param>
        /// <param name="nitaq">
        /// The span it opens or closes, or <see cref="Nasq.BilaNitaq"/>.
        /// </param>
        /// <param name="mawdiNaqi">Byte offset into the clean text.</param>
        /// <param name="badeel">Clean bytes standing in its place.</param>
        /// <param name="aslBidaya">Character offset into the raw string.</param>
        /// <param name="aslTul">Its length in characters.</param>
        public SijillNasq(
            NawSijill naw,
            ushort nitaq,
            uint mawdiNaqi,
            uint badeel,
            int aslBidaya,
            int aslTul)
        {
            Naw = naw;
            Nitaq = nitaq;
            MawdiNaqi = mawdiNaqi;
            Badeel = badeel;
            AslBidaya = aslBidaya;
            AslTul = aslTul;
        }

        /// <summary>What this record put back.</summary>
        public NawSijill Naw { get; }

        /// <summary>
        /// The style span this record opens or closes, matching
        /// <see cref="TaaribNitaqUslub.Id"/>, or <see cref="Nasq.BilaNitaq"/>
        /// when it belongs to no span. This field is what makes reinsertion
        /// around translated text possible at all: it lets a rebuild find the
        /// raw tag for a span by identity rather than by an offset that the
        /// translation has invalidated.
        /// </summary>
        public ushort Nitaq { get; }

        /// <summary>
        /// Byte offset into the clean UTF-8 text at which this text was
        /// removed. Non-decreasing across the record table, which is what lets
        /// a rebuild consume the clean text once from left to right.
        /// </summary>
        public uint MawdiNaqi { get; }

        /// <summary>
        /// How many bytes of clean text stand in the removed text's place: zero
        /// for a tag that left nothing behind, three for an atom that became
        /// one U+FFFC, one for an escape that became a single ASCII character.
        /// </summary>
        public uint Badeel { get; }

        /// <summary>
        /// Where the raw text begins in the caller's original string, in UTF-16
        /// characters.
        /// </summary>
        public int AslBidaya { get; }

        /// <summary>Its length there, in UTF-16 characters.</summary>
        public int AslTul { get; }
    }

    /// <summary>
    /// One atom: an opaque run that occupies a position in the layout and whose
    /// contents belong to the game rather than to Taarib.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Why a placeholder is an atom.</b> <c>{0}</c> is not a word. The game
    /// substitutes into it after Taarib has run, so the braces, the digits and
    /// any format specifier between them have to reach the game's formatter
    /// spelled exactly as they were written. Let the bidirectional algorithm
    /// treat them as ordinary neutrals inside an Arabic paragraph and the
    /// closing brace moves ahead of the opening one; the game's formatter then
    /// either throws on a malformed format string or, worse, matches a
    /// different index and substitutes the wrong value into a sentence that
    /// still looks well-formed.
    /// </para>
    /// <para>
    /// <b>Why a sprite is an atom.</b> Its glyph slot has to survive to the
    /// mesh builder, because the picture inside it lives in the game's own
    /// sprite asset and only the game can draw it. The span carrying this atom
    /// is flagged <see cref="Alamat.UslubDharra"/>, which is what makes
    /// <c>Nasij</c> emit a <c>DharraMawduaa</c> for it and no geometry.
    /// </para>
    /// </remarks>
    public readonly struct DharraNasq
    {
        /// <summary>Builds one atom record.</summary>
        /// <param name="naw">What it stands for.</param>
        /// <param name="nitaq">The span that carries it.</param>
        /// <param name="mawdiNaqi">Its byte offset in the clean text.</param>
        /// <param name="tulNaqi">Its length there, in bytes.</param>
        /// <param name="aslBidaya">Its character offset in the raw string.</param>
        /// <param name="aslTul">Its length there, in characters.</param>
        /// <param name="fahras">A sprite index, or <c>-1</c>.</param>
        /// <param name="tarteeb">A placeholder's written index, or <c>-1</c>.</param>
        /// <param name="ismBidaya">A sprite name's character offset, or zero.</param>
        /// <param name="ismTul">Its length in characters, or zero.</param>
        public DharraNasq(
            NawDharra naw,
            ushort nitaq,
            uint mawdiNaqi,
            uint tulNaqi,
            int aslBidaya,
            int aslTul,
            int fahras,
            int tarteeb,
            int ismBidaya,
            int ismTul)
        {
            Naw = naw;
            Nitaq = nitaq;
            MawdiNaqi = mawdiNaqi;
            TulNaqi = tulNaqi;
            AslBidaya = aslBidaya;
            AslTul = aslTul;
            Fahras = fahras;
            Tarteeb = tarteeb;
            IsmBidaya = ismBidaya;
            IsmTul = ismTul;
        }

        /// <summary>What it stands for.</summary>
        public NawDharra Naw { get; }

        /// <summary>
        /// The span carrying it, matching <see cref="TaaribNitaqUslub.Id"/>.
        /// That span covers exactly this atom's clean bytes and carries the
        /// width, height and baseline the layout reserves for it.
        /// </summary>
        public ushort Nitaq { get; }

        /// <summary>Byte offset of the atom in the clean UTF-8 text.</summary>
        public uint MawdiNaqi { get; }

        /// <summary>
        /// How many clean bytes it occupies: three for the one U+FFFC a
        /// placeholder or a sprite became, the run's own length for
        /// <see cref="NawDharra.Khaam"/>.
        /// </summary>
        public uint TulNaqi { get; }

        /// <summary>
        /// Where the atom's raw text begins in the caller's original string, in
        /// UTF-16 characters, delimiters included. This window is what a
        /// translation is validated against: an atom that is missing,
        /// duplicated or altered is a rejected translation, and the check is a
        /// comparison of these windows rather than of anything reconstructed.
        /// </summary>
        public int AslBidaya { get; }

        /// <summary>Its length there, in UTF-16 characters.</summary>
        public int AslTul { get; }

        /// <summary>
        /// The sprite index the tag wrote, or <c>-1</c> when it named a sprite
        /// by name instead or is not a sprite at all.
        /// </summary>
        public int Fahras { get; }

        /// <summary>
        /// The positional index a placeholder wrote, or <c>-1</c> for
        /// <c>%s</c> and <c>{name}</c>, which write none.
        /// </summary>
        /// <remarks>
        /// Kept exactly as the dialect wrote it and never renumbered between
        /// dialects: <c>{0}</c> is zero-based and <c>%1$s</c> is one-based, and
        /// normalising either to match the other would make a reordering check
        /// compare two different numbering schemes and report a defect that is
        /// not there.
        /// </remarks>
        public int Tarteeb { get; }

        /// <summary>
        /// Where a sprite's name begins in the raw string, in UTF-16
        /// characters; zero when it has none. Named rather than copied for the
        /// same reason everything else here is: the caller owns the string, and
        /// copying it out would be an allocation on a path that has none.
        /// </summary>
        public int IsmBidaya { get; }

        /// <summary>The name's length in characters, or zero.</summary>
        public int IsmTul { get; }
    }

    /// <summary>
    /// How much room a parse needs, in the four destinations it writes to.
    /// </summary>
    /// <remarks>
    /// The same negotiation <see cref="Ramz.SiatQasira"/> defines at the ABI
    /// and <c>Nasij</c> repeats for meshes: on a shortfall the parse still
    /// completes, the requirements are reported, and the caller grows once and
    /// calls again. One growth idiom across the whole assembly rather than
    /// three.
    /// </remarks>
    public struct QiyasNasq
    {
        /// <summary>Clean text, in UTF-8 bytes.</summary>
        public int TulNass;

        /// <summary>Style spans.</summary>
        public int AdadNitaqat;

        /// <summary>Atoms.</summary>
        public int AdadDharrat;

        /// <summary>Reinsertion records.</summary>
        public int AdadSijillat;
    }

    /// <summary>
    /// The caller-owned destination a parse writes into. A <c>ref struct</c>
    /// because every field is a window into memory that lives exactly as long
    /// as the call — a pooled array, or a <c>stackalloc</c> in the takeover's
    /// own frame.
    /// </summary>
    /// <remarks>
    /// Sizing it is the caller's decision and <see cref="Nasq.Ihsi"/> answers
    /// it exactly. A takeover that has already parsed a string once holds the
    /// answer and never asks again, because the parse is keyed on a string that
    /// does not change while the game is running.
    /// </remarks>
    public ref struct MakhzanNasq
    {
        /// <summary>
        /// The clean text, as UTF-8. An upper bound that never needs a count is
        /// three times the input's character length; the exact figure comes
        /// from <see cref="QiyasNasq.TulNass"/>.
        /// </summary>
        public Span<byte> Nass;

        /// <summary>
        /// The style spans, in the order the layout wants them: by start offset
        /// ascending, and by length descending within one start, so an
        /// enclosing span always precedes the spans nested inside it.
        /// </summary>
        public Span<TaaribNitaqUslub> Nitaqat;

        /// <summary>
        /// The atoms, in the order they appear. May be left empty when the
        /// caller has no use for them; the spans still carry
        /// <see cref="Alamat.UslubDharra"/> and the layout is unaffected.
        /// </summary>
        public Span<DharraNasq> Dharrat;

        /// <summary>
        /// The reinsertion records. May be left empty by a caller that will
        /// never hand the string back to the game's own component — but a
        /// caller that leaves it empty cannot rebuild anything afterwards, and
        /// there is no second pass that could recover it, because the raw
        /// string is not kept.
        /// </summary>
        public Span<SijillNasq> Sijillat;
    }

    /// <summary>
    /// What the scanner is allowed to recognise, and the numbers it needs for
    /// the values it cannot compute on its own.
    /// </summary>
    public struct KhiyaratNasq
    {
        /// <summary>The dialect. See <see cref="NawNasq"/> for why it is one.</summary>
        public NawNasq Naw;

        /// <summary>
        /// The size, in pixels, that a relative size tag is relative to —
        /// normally the component's own font size.
        /// </summary>
        /// <remarks>
        /// <c>&lt;size=120%&gt;</c> and <c>&lt;size=+2&gt;</c> cannot be
        /// resolved without it, and <see cref="TaaribNitaqUslub.Hajm"/> is
        /// absolute pixels by definition. Zero or less means the span is still
        /// produced and still rebuilds, but carries no size override at all,
        /// because inventing a size is worse than declining to set one: a
        /// wrongly sized run is a line that wraps differently from the game's,
        /// and a run at the component's own size is merely a missing emphasis.
        /// </remarks>
        public float HajmAsas;

        /// <summary>
        /// The width an atom occupies when nothing better is known, in pixels
        /// at the layout's own size.
        /// </summary>
        /// <remarks>
        /// Zero means unknown and is the honest default: this file cannot
        /// measure a sprite it has never seen or a placeholder that has not
        /// been substituted yet. An adapter holding the engine's own sprite
        /// metrics passes the real width and the layout is right; one that
        /// passes nothing gets an atom of zero width, which is visibly wrong
        /// rather than subtly wrong, and that is the correct way for a missing
        /// measurement to fail.
        /// </remarks>
        public float ArdDharraIftiradi;

        /// <summary>
        /// The height an atom occupies when nothing better is known. Zero means
        /// the line box decides, which is what <c>Nasij</c> falls back to.
        /// </summary>
        public float IrtifaDharraIftiradi;

        /// <summary>
        /// How far above the baseline an atom's bottom edge sits, when nothing
        /// better is known. Zero puts it on the baseline.
        /// </summary>
        public float AsasDharraIftiradi;

        /// <summary>
        /// How many tags may be open at once. Zero selects
        /// <see cref="Nasq.UmqIftiradi"/>.
        /// </summary>
        /// <remarks>
        /// Not quite a nesting limit, because in these dialects a tag need not
        /// be closed at all: <c>&lt;color=a&gt;x&lt;color=b&gt;y</c> leaves two
        /// tags open with nothing nested, and TextMeshPro's own style stacks
        /// behave the same way. The bound exists so that a hostile or corrupted
        /// string cannot make this scanner hold unbounded state inside a game's
        /// frame.
        /// </remarks>
        public byte AqsaUmq;

        /// <summary>
        /// Options for one dialect, with the defaults every other field wants.
        /// </summary>
        /// <param name="naw">The dialect.</param>
        /// <returns>The options.</returns>
        public static KhiyaratNasq Min(NawNasq naw)
        {
            KhiyaratNasq khiyarat = default;
            khiyarat.Naw = naw;
            khiyarat.AqsaUmq = Nasq.UmqIftiradi;
            return khiyarat;
        }
    }

    /// <summary>
    /// Clean text, the spans over it, the atoms in it, and enough to put all of
    /// the markup back. A <c>readonly ref struct</c> because every table in it
    /// is a window into the caller's own buffers.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The offsets.</b> <see cref="Nass"/> is UTF-8, and every
    /// <see cref="TaaribNitaqUslub.Bidaya"/> and
    /// <see cref="TaaribNitaqUslub.Tul"/> in <see cref="Nitaqat"/> is a byte
    /// offset and a byte length into it. Nothing here indexes the raw string
    /// the parse was given except <see cref="SijillNasq"/> and
    /// <see cref="DharraNasq"/>, both of which say so in their own units.
    /// </para>
    /// <para>
    /// <b>The lifetime.</b> This struct borrows; it owns nothing and frees
    /// nothing. It is valid exactly as long as the
    /// <see cref="MakhzanNasq"/> it was written into is, which for a pooled
    /// buffer means until the buffer is returned and for a
    /// <c>stackalloc</c> means until the frame that made it returns.
    /// </para>
    /// </remarks>
    public readonly ref struct NatijaNasq
    {
        /// <summary>Assembles the result. Called only by <see cref="Nasq"/>.</summary>
        /// <param name="nass">The clean UTF-8 text.</param>
        /// <param name="nitaqat">The style spans.</param>
        /// <param name="dharrat">The atoms.</param>
        /// <param name="sijillat">The reinsertion records.</param>
        /// <param name="kafa">Whether every destination was large enough.</param>
        /// <param name="matlub">What was needed.</param>
        /// <param name="alam">The <see cref="AlamatNasq"/> flags.</param>
        /// <param name="aslTul">The raw input's length, in characters.</param>
        internal NatijaNasq(
            ReadOnlySpan<byte> nass,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            ReadOnlySpan<DharraNasq> dharrat,
            ReadOnlySpan<SijillNasq> sijillat,
            bool kafa,
            QiyasNasq matlub,
            uint alam,
            int aslTul)
        {
            Nass = nass;
            Nitaqat = nitaqat;
            Dharrat = dharrat;
            Sijillat = sijillat;
            Kafa = kafa;
            Matlub = matlub;
            Alam = alam;
            AslTul = aslTul;
        }

        /// <summary>
        /// The clean text: no tags, no escapes, one U+FFFC per placeholder and
        /// per sprite. This is what goes to the layout call, and it is the only
        /// form of the string the shaper, the bidirectional pass and the line
        /// breaker ever see.
        /// </summary>
        public ReadOnlySpan<byte> Nass { get; }

        /// <summary>
        /// The style spans over <see cref="Nass"/>, sorted the way the layout
        /// wants them and the way <c>Nasij</c>'s innermost-wins colour
        /// resolution depends on.
        /// </summary>
        public ReadOnlySpan<TaaribNitaqUslub> Nitaqat { get; }

        /// <summary>Every atom, in the order it appeared.</summary>
        public ReadOnlySpan<DharraNasq> Dharrat { get; }

        /// <summary>
        /// What was removed and where, in the order it was removed. Consumed
        /// only by the two rebuild paths.
        /// </summary>
        public ReadOnlySpan<SijillNasq> Sijillat { get; }

        /// <summary>
        /// Whether every destination was large enough. When false the tables
        /// hold as much as fit, <see cref="Matlub"/> holds what was needed, and
        /// <see cref="AlamatNasq.Farada"/> is set — but the parse still ran to
        /// the end of the string, so the counts are exact rather than a lower
        /// bound the caller has to guess past.
        /// </summary>
        public bool Kafa { get; }

        /// <summary>What the four destinations needed, exactly.</summary>
        public QiyasNasq Matlub { get; }

        /// <summary>The <see cref="AlamatNasq"/> flags.</summary>
        public uint Alam { get; }

        /// <summary>The raw input's length, in UTF-16 characters.</summary>
        public int AslTul { get; }

        /// <summary>
        /// Whether nothing at all was extracted, in which case the clean text
        /// is the input and the tables are empty.
        /// </summary>
        public bool NaqiAsasan => (Alam & AlamatNasq.NaqiAsasan) != 0;
    }

    /// <summary>
    /// The markup bridge: raw rich text in, clean UTF-8 and byte-ranged style
    /// spans out, and the record that puts the markup back afterwards.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>What allocates and what does not.</b>
    /// <see cref="Hallil"/> and <see cref="Ihsi"/> allocate nothing: they write
    /// into spans the caller owns and their only internal state is a
    /// fixed-size open-tag stack in stack memory. So do
    /// <see cref="AadBinaaAsl"/> and <see cref="AadBinaaTarjama"/>, which write
    /// into a caller-owned <c>Span&lt;char&gt;</c>. The two convenience
    /// overloads that return a <see cref="string"/> allocate exactly one, are
    /// named for it, and exist for the log and the editor — never for a frame.
    /// </para>
    /// <para>
    /// <b>Malformed markup is not an error, and refusing would be wrong.</b>
    /// Every dialect here ships a lenient parser and this file matches each one
    /// rather than being stricter than the thing it is imitating:
    /// </para>
    /// <list type="bullet">
    /// <item><description>
    /// An unclosed tag closes at the end of the string. TextMeshPro's style
    /// stacks behave exactly this way — <c>&lt;color=red&gt;</c> with no
    /// closing tag colours the rest of the label — so producing a span that
    /// stops early would render differently from the game.
    /// </description></item>
    /// <item><description>
    /// A closing tag with nothing open to match it is dropped, silently, as
    /// every one of these parsers drops it.
    /// </description></item>
    /// <item><description>
    /// An unrecognised tag, and a tag whose attribute value cannot be read, is
    /// left in the clean text as literal characters — because that is precisely
    /// what the game's own parser does with it, and because the alternative is
    /// worse in a way that hides. A patch that silently ate a tag the game
    /// meant to <em>display</em> removes text from somebody's screen with no
    /// diagnostic anywhere; a patch that shows a tag the game meant to
    /// interpret is visible on the first playthrough and reported.
    /// </description></item>
    /// <item><description>
    /// A stray <c>&lt;</c> that never finds its <c>&gt;</c> within
    /// <see cref="HaddWasm"/> characters is punctuation. Without that bound a
    /// single stray bracket in a long paragraph costs a scan of the whole
    /// paragraph, once per parse.
    /// </description></item>
    /// </list>
    /// <para>
    /// Each leniency sets a flag on <see cref="NatijaNasq.Alam"/> instead of
    /// throwing, so the compiler's review pass and the in-game diagnostics can
    /// count the strings a game ships with broken markup while the runtime path
    /// goes on rendering them the way the game does. Nothing in this file
    /// throws <see cref="KhataTaarib"/> for the content of a string; the one
    /// refusal it has is <see cref="TahaqquqNitaqat"/>, which is about a span
    /// table that arrived from somewhere else.
    /// </para>
    /// <para>
    /// <b>Span order.</b> Spans are emitted in the order their tags opened,
    /// which is start-ascending by construction, and — for properly nested
    /// markup — length-descending within one start, which is the order the
    /// layout and <c>Nasij</c>'s innermost-wins colour resolution both expect.
    /// For crossed tags, which every dialect here permits
    /// (<c>&lt;b&gt;&lt;i&gt;x&lt;/b&gt;y&lt;/i&gt;</c>), open order is what
    /// "innermost" actually means, and it is what the last-match rule in
    /// <c>Nasij.HallLawn</c> resolves to. Identifiers are handed out in order
    /// from zero, so a span's identifier is its own index in the table — which
    /// is the fast path <c>Nasij.MawdiNitaq</c> takes.
    /// </para>
    /// </remarks>
    public static class Nasq
    {
        /// <summary>
        /// The character one atom occupies in the clean text: U+FFFC OBJECT
        /// REPLACEMENT CHARACTER.
        /// </summary>
        /// <remarks>
        /// Three properties make it the only correct choice, and all three are
        /// load-bearing. It is exactly one position, so the line breaker and
        /// the reordering pass have something to reason about that cannot be
        /// split through the middle. Its bidirectional class is Other Neutral,
        /// so an atom takes the direction of whatever surrounds it rather than
        /// imposing one — <c>{0}</c> between two Arabic words stays inside the
        /// Arabic run, and the same <c>{0}</c> between two Latin words stays
        /// inside the Latin run. And no real font draws it, it is a letter in
        /// no script and it carries no joining behaviour, so it cannot join to
        /// an Arabic neighbour, cannot absorb a kashida, and cannot be shaped
        /// into something visible by a later stage that forgot to skip it.
        /// </remarks>
        public const char BadeelDharra = '￼';

        /// <summary>How many UTF-8 bytes <see cref="BadeelDharra"/> occupies.</summary>
        public const uint HajmBadeel = 3;

        /// <summary>
        /// The identifier <see cref="SijillNasq.Nitaq"/> carries when a record
        /// belongs to no span. Not a valid span identifier, because a table
        /// that reached this many spans has already hit
        /// <see cref="AqsaNitaqat"/>.
        /// </summary>
        public const ushort BilaNitaq = ushort.MaxValue;

        /// <summary>
        /// The most spans one string may produce. One below
        /// <see cref="BilaNitaq"/>, so the sentinel can never collide with a
        /// real identifier, and far above anything a game string contains.
        /// </summary>
        public const int AqsaNitaqat = ushort.MaxValue - 1;

        /// <summary>
        /// How many tags may be open at once when the caller sets no limit.
        /// Sixty-four is far above anything hand-written or engine-generated,
        /// and far below the point where a hostile string could make this
        /// scanner do interesting amounts of work inside a frame.
        /// </summary>
        public const byte UmqIftiradi = 64;

        /// <summary>
        /// How far past a <c>&lt;</c> or <c>[</c> the scanner looks for the
        /// closing bracket before deciding the character was punctuation.
        /// TextMeshPro imposes a comparable limit on its own tag buffer.
        /// </summary>
        public const int HaddWasm = 256;

        /// <summary>
        /// The weight <c>&lt;b&gt;</c> resolves to, in the units
        /// <see cref="TaaribNitaqUslub.Wazn"/> uses.
        /// </summary>
        public const ushort WaznGhaliz = 700;

        /// <summary>
        /// Parses one string into clean text and spans.
        /// </summary>
        /// <remarks>
        /// <para>
        /// Allocation-free from end to end. The common case — a string with no
        /// markup at all — is one scan for the handful of characters that could
        /// begin a construct, followed by one UTF-8 encode of the string, and
        /// nothing else happens on that path.
        /// </para>
        /// <para>
        /// The scan always runs to the end of the string, even when a
        /// destination filled up. That is what makes
        /// <see cref="NatijaNasq.Matlub"/> exact rather than a lower bound the
        /// caller has to grow past repeatedly, and it costs nothing: the work
        /// that was skipped is the writing, not the scanning.
        /// </para>
        /// </remarks>
        /// <param name="khaam">The raw string, exactly as the game holds it.</param>
        /// <param name="khiyarat">The dialect and the numbers it needs.</param>
        /// <param name="makhzan">
        /// The caller's destinations. Taken by value rather than by reference
        /// because the result borrows from it, and a by-value ref struct
        /// parameter is the form whose lifetime rules permit that.
        /// </param>
        /// <returns>
        /// The clean text, the tables over it, and what would have been needed.
        /// </returns>
        public static NatijaNasq Hallil(
            ReadOnlySpan<char> khaam,
            in KhiyaratNasq khiyarat,
            MakhzanNasq makhzan)
        {
            byte umq = khiyarat.AqsaUmq == 0 ? UmqIftiradi : khiyarat.AqsaUmq;
            Span<Qaid> kadas = stackalloc Qaid[umq];
            Massah massah = new Massah(khaam, in khiyarat, makhzan, kadas);
            massah.Imsah();

            QiyasNasq matlub = massah.Matlub;
            uint alam = massah.Alam;
            if (matlub.AdadSijillat == 0)
            {
                alam |= AlamatNasq.NaqiAsasan;
            }
            bool kafa = matlub.TulNass <= makhzan.Nass.Length
                && matlub.AdadNitaqat <= makhzan.Nitaqat.Length
                && matlub.AdadDharrat <= makhzan.Dharrat.Length
                && matlub.AdadSijillat <= makhzan.Sijillat.Length;

            // The result is assembled here, from the parameter, rather than
            // inside the scanner. The scanner holds a stackalloc'd tag stack,
            // which pins its own lifetime to this frame; a result built from
            // its fields would inherit that lifetime and could not be returned.
            // Built from `makhzan` — a by-value parameter — the spans outlive
            // the call exactly as long as the caller's buffers do.
            return new NatijaNasq(
                makhzan.Nass.Slice(0, Adna(matlub.TulNass, makhzan.Nass.Length)),
                makhzan.Nitaqat.Slice(0, Adna(matlub.AdadNitaqat, makhzan.Nitaqat.Length)),
                makhzan.Dharrat.Slice(0, Adna(matlub.AdadDharrat, makhzan.Dharrat.Length)),
                makhzan.Sijillat.Slice(0, Adna(matlub.AdadSijillat, makhzan.Sijillat.Length)),
                kafa,
                matlub,
                alam,
                khaam.Length);
        }

        /// <summary>The smaller of two counts.</summary>
        private static int Adna(int awwal, int thani) => awwal < thani ? awwal : thani;

        /// <summary>
        /// Counts exactly what <see cref="Hallil"/> would write, writing
        /// nothing.
        /// </summary>
        /// <remarks>
        /// Walks the identical code with every destination empty, so the two
        /// cannot answer differently about the same string — which a separately
        /// written counter eventually would, on the one dialect nobody
        /// remembered to update.
        /// </remarks>
        /// <param name="khaam">The raw string.</param>
        /// <param name="khiyarat">The dialect and the numbers it needs.</param>
        /// <returns>The exact capacity the four destinations need.</returns>
        public static QiyasNasq Ihsi(ReadOnlySpan<char> khaam, in KhiyaratNasq khiyarat)
        {
            MakhzanNasq farigh = default;
            return Hallil(khaam, in khiyarat, farigh).Matlub;
        }

        /// <summary>
        /// Rebuilds the original raw string, character for character, from the
        /// clean text plus the record table.
        /// </summary>
        /// <remarks>
        /// <para>
        /// This is the half of the contract everything else rests on. If the
        /// round trip is exact, markup can be lifted out before a translator or
        /// a machine translation provider ever sees the string — so neither can
        /// damage a tag it was never shown — and put back mechanically
        /// afterwards rather than by pattern-matching the translated text. A
        /// reconstruction that were merely usually right would put a visible
        /// <c>&lt;color=#ff0000&gt;</c> on somebody's screen a few thousand
        /// strings into a patch, which is the class of defect this whole design
        /// exists to make impossible.
        /// </para>
        /// <para>
        /// The walk is linear because the record table is: records are produced
        /// in the order they were removed and their offsets never decrease, so
        /// the clean text is consumed once, from left to right, each record
        /// splicing its raw characters back in and skipping whatever stood in
        /// their place.
        /// </para>
        /// </remarks>
        /// <param name="asl">
        /// The raw string the parse was given. Records name windows into it and
        /// hold no text of their own, so this must be the same string — a
        /// different one produces a different rebuild with no complaint.
        /// </param>
        /// <param name="naqi">The clean UTF-8 text.</param>
        /// <param name="sijillat">The record table.</param>
        /// <param name="kharij">The destination.</param>
        /// <param name="tul">
        /// How many characters the rebuild needs, whether or not it fit.
        /// </param>
        /// <returns><c>false</c> when <paramref name="kharij"/> is too small.</returns>
        public static bool AadBinaaAsl(
            ReadOnlySpan<char> asl,
            ReadOnlySpan<byte> naqi,
            ReadOnlySpan<SijillNasq> sijillat,
            Span<char> kharij,
            out int tul)
        {
            Katib katib = new Katib(kharij);
            int sabiq = 0;
            for (int i = 0; i < sijillat.Length; i++)
            {
                SijillNasq sijill = sijillat[i];
                int mawqi = sijill.MawdiNaqi > (uint)naqi.Length
                    ? naqi.Length
                    : (int)sijill.MawdiNaqi;
                if (mawqi > sabiq)
                {
                    katib.Bayt(naqi.Slice(sabiq, mawqi - sabiq));
                }
                katib.Nass(Nafidha(asl, sijill.AslBidaya, sijill.AslTul));
                long baad = (long)mawqi + sijill.Badeel;
                sabiq = baad > naqi.Length ? naqi.Length : (int)baad;
            }
            if (sabiq < naqi.Length)
            {
                katib.Bayt(naqi.Slice(sabiq));
            }
            tul = katib.Tul;
            return katib.Kafa;
        }

        /// <summary>
        /// Re-emits the original markup around translated text.
        /// </summary>
        /// <remarks>
        /// <para>
        /// <b>Why this exists.</b> A takeover does not always draw. It declines
        /// on components it cannot safely own, on effects it does not model,
        /// and whenever the patch says to leave a string alone — and in every
        /// one of those cases the string has to go back to the game's own
        /// component, in the game's own markup, with the Arabic in it. That is
        /// this method. Without it, declining to draw would mean handing back
        /// text with every tag stripped, which is a worse outcome than not
        /// translating the string at all.
        /// </para>
        /// <para>
        /// <b>Why spans are matched by index and not by offset.</b> The
        /// translated text is a different string of a different length with its
        /// boundaries in different places. Its span table comes from the patch,
        /// where a translator placed the emphasis on the Arabic words that
        /// carry it — which are not at the source string's offsets and, for a
        /// right-to-left translation of a left-to-right sentence, are not even
        /// in the same order. So nothing here reuses a source offset. Each
        /// translated span carries an identifier; the record table is searched
        /// for the source records that opened and closed the span with that
        /// identifier; and those exact raw characters are emitted at the
        /// translated span's own boundaries. Matching by offset instead would
        /// put <c>&lt;b&gt;</c> around whatever happened to sit at the source's
        /// byte range, which in Arabic is usually the middle of a different
        /// word.
        /// </para>
        /// <para>
        /// <b>Atoms.</b> A translated span flagged
        /// <see cref="Alamat.UslubDharra"/> covers the one U+FFFC standing for
        /// an atom. Its raw text — <c>{0}</c>, <c>&lt;sprite=3&gt;</c> — is
        /// taken from the source record with the same identifier and written in
        /// place of the replacement character, so the string the game's
        /// formatter receives has the placeholder it is going to substitute
        /// into, spelled the way it was written.
        /// </para>
        /// <para>
        /// Allocation-free, and bounded by the span count rather than by the
        /// text length: the walk visits only the offsets where a span begins or
        /// ends, and finds each next boundary by one pass over the span table.
        /// </para>
        /// </remarks>
        /// <param name="asl">
        /// The raw source string the records name windows into.
        /// </param>
        /// <param name="sijillat">The source record table.</param>
        /// <param name="tarjama">The translated clean text, as UTF-8.</param>
        /// <param name="nitaqat">
        /// The translation's own spans, whose offsets index
        /// <paramref name="tarjama"/> and whose identifiers match the source's.
        /// </param>
        /// <param name="kharij">The destination.</param>
        /// <param name="tul">How many characters the result needs.</param>
        /// <returns><c>false</c> when <paramref name="kharij"/> is too small.</returns>
        public static bool AadBinaaTarjama(
            ReadOnlySpan<char> asl,
            ReadOnlySpan<SijillNasq> sijillat,
            ReadOnlySpan<byte> tarjama,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            Span<char> kharij,
            out int tul)
        {
            Katib katib = new Katib(kharij);
            int nihaya = tarjama.Length;
            int mawqi = 0;
            int hadd = TaliHadd(nitaqat, -1, nihaya);

            while (true)
            {
                if (hadd > mawqi)
                {
                    katib.Bayt(tarjama.Slice(mawqi, hadd - mawqi));
                    mawqi = hadd;
                }

                for (int i = nitaqat.Length - 1; i >= 0; i--)
                {
                    TaaribNitaqUslub nitaq = nitaqat[i];
                    if ((nitaq.Alam & Alamat.UslubDharra) != 0)
                    {
                        continue;
                    }
                    if ((long)nitaq.Bidaya + nitaq.Tul == hadd && nitaq.Tul != 0)
                    {
                        katib.Nass(KhaamSijill(asl, sijillat, nitaq.Id, NawSijill.Ikhtitam));
                    }
                }

                for (int i = 0; i < nitaqat.Length; i++)
                {
                    TaaribNitaqUslub nitaq = nitaqat[i];
                    if (nitaq.Bidaya != (uint)hadd)
                    {
                        continue;
                    }
                    if ((nitaq.Alam & Alamat.UslubDharra) != 0)
                    {
                        katib.Nass(KhaamSijill(asl, sijillat, nitaq.Id, NawSijill.Dharra));
                        long baad = (long)hadd + nitaq.Tul;
                        int qafz = baad > nihaya ? nihaya : (int)baad;
                        if (qafz > mawqi)
                        {
                            mawqi = qafz;
                        }
                    }
                    else if (nitaq.Tul != 0)
                    {
                        katib.Nass(KhaamSijill(asl, sijillat, nitaq.Id, NawSijill.Iftitah));
                    }
                }

                if (hadd >= nihaya)
                {
                    break;
                }
                hadd = TaliHadd(nitaqat, hadd, nihaya);
            }

            if (mawqi < nihaya)
            {
                katib.Bayt(tarjama.Slice(mawqi));
            }
            tul = katib.Tul;
            return katib.Kafa;
        }

        /// <summary>
        /// <see cref="AadBinaaTarjama"/> into a new string. Allocates exactly
        /// one, which is why it is a separate member with the cost in its
        /// documentation rather than an overload someone reaches for inside a
        /// frame: this is the form a takeover uses when it is about to assign
        /// the result to the game's own text property, which allocates a string
        /// anyway.
        /// </summary>
        /// <param name="asl">The raw source string.</param>
        /// <param name="sijillat">The source record table.</param>
        /// <param name="tarjama">The translated clean text, as UTF-8.</param>
        /// <param name="nitaqat">The translation's own spans.</param>
        /// <returns>The translated text with the game's markup back around it.</returns>
        public static string NassTarjama(
            ReadOnlySpan<char> asl,
            ReadOnlySpan<SijillNasq> sijillat,
            ReadOnlySpan<byte> tarjama,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat)
        {
            AadBinaaTarjama(asl, sijillat, tarjama, nitaqat, Span<char>.Empty, out int tul);
            if (tul <= 0)
            {
                return string.Empty;
            }
            char[] mahal = new char[tul];
            AadBinaaTarjama(asl, sijillat, tarjama, nitaqat, mahal, out int kutib);
            return new string(mahal, 0, kutib < tul ? kutib : tul);
        }

        /// <summary>
        /// Confirms that every span lands inside the clean text and on UTF-8
        /// character boundaries.
        /// </summary>
        /// <remarks>
        /// <see cref="Hallil"/> produces spans that satisfy this by
        /// construction — every offset it records is the length of the clean
        /// text at a moment when the clean text was valid UTF-8 — so this is
        /// never called on its output. It exists for the other direction: span
        /// tables that arrived from a patch file, from a capture, or from a
        /// translation whose markup was reapplied by something other than
        /// <see cref="AadBinaaTarjama"/>. A boundary inside a UTF-8 sequence
        /// splits a character, which splits a cluster, which puts a diacritic
        /// in one style and its base letter in another — and that renders, so
        /// nothing downstream will report it.
        /// </remarks>
        /// <param name="naqi">The clean UTF-8 text.</param>
        /// <param name="nitaqat">The span table to check.</param>
        /// <exception cref="KhataTaarib">
        /// A span reaches past the end of the text, or a boundary falls inside
        /// a character.
        /// </exception>
        public static void TahaqquqNitaqat(
            ReadOnlySpan<byte> naqi,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat)
        {
            for (int i = 0; i < nitaqat.Length; i++)
            {
                TaaribNitaqUslub nitaq = nitaqat[i];
                long nihaya = (long)nitaq.Bidaya + nitaq.Tul;
                if (nitaq.Bidaya > (uint)naqi.Length || nihaya > naqi.Length)
                {
                    throw new KhataTaarib(
                        Ramz.QeemaBatila,
                        string.Empty,
                        $"يتجاوز نطاق الأسلوب رقم {nitaq.Id} نهاية النص النقي "
                            + $"({nitaq.Bidaya}+{nitaq.Tul} من {naqi.Length} بايت)؛ "
                            + "رُفض التخطيط بدل القراءة خارج النص.",
                        $"Style span {nitaq.Id} reaches past the end of the clean text "
                            + $"({nitaq.Bidaya}+{nitaq.Tul} of {naqi.Length} bytes); layout is "
                            + "refused rather than reading outside the text.",
                        Khutwa.FathTashkhis);
                }
                if (HaddHarf(naqi, (int)nitaq.Bidaya) && HaddHarf(naqi, (int)nihaya))
                {
                    continue;
                }
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    string.Empty,
                    $"يقع أحد حدّي نطاق الأسلوب رقم {nitaq.Id} داخل حرف UTF-8 ولا على حدّه؛ "
                        + "هذا يشطر العنقود فيفصل الحركة عن حرفها، فرُفض بدل أن يُرسم.",
                    $"A boundary of style span {nitaq.Id} falls inside a UTF-8 character "
                        + "rather than on one; that splits the cluster and separates a mark "
                        + "from its base letter, so it is refused rather than drawn.",
                    Khutwa.FathTashkhis);
            }
        }

        /// <summary>
        /// The smallest span boundary strictly greater than
        /// <paramref name="baad"/>, or the text's end when there is none.
        /// </summary>
        private static int TaliHadd(
            ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            int baad,
            int nihaya)
        {
            int tali = nihaya;
            for (int i = 0; i < nitaqat.Length; i++)
            {
                TaaribNitaqUslub nitaq = nitaqat[i];
                long bidaya = nitaq.Bidaya;
                if (bidaya > baad && bidaya < tali)
                {
                    tali = (int)bidaya;
                }
                long akhir = bidaya + nitaq.Tul;
                if (akhir > baad && akhir < tali)
                {
                    tali = (int)akhir;
                }
            }
            return tali;
        }

        /// <summary>
        /// The raw characters of the record that opened, closed or stood for a
        /// given span. Empty when the source had none, which is the right
        /// answer for a span a translator added that the source string never
        /// had: it simply carries no markup back.
        /// </summary>
        private static ReadOnlySpan<char> KhaamSijill(
            ReadOnlySpan<char> asl,
            ReadOnlySpan<SijillNasq> sijillat,
            ushort id,
            NawSijill naw)
        {
            for (int i = 0; i < sijillat.Length; i++)
            {
                if (sijillat[i].Nitaq == id && sijillat[i].Naw == naw)
                {
                    return Nafidha(asl, sijillat[i].AslBidaya, sijillat[i].AslTul);
                }
            }
            return ReadOnlySpan<char>.Empty;
        }

        /// <summary>
        /// A window into the raw string, clamped. Clamped rather than checked
        /// because a record built by anything other than <see cref="Hallil"/>
        /// can name a window that does not exist, and a rebuild that dropped
        /// one tag is better than one that threw halfway through a string the
        /// game is waiting for.
        /// </summary>
        private static ReadOnlySpan<char> Nafidha(ReadOnlySpan<char> asl, int bidaya, int tul)
        {
            if (tul <= 0 || bidaya < 0 || bidaya >= asl.Length)
            {
                return ReadOnlySpan<char>.Empty;
            }
            int baqi = asl.Length - bidaya;
            return asl.Slice(bidaya, tul < baqi ? tul : baqi);
        }

        /// <summary>
        /// Whether a byte offset falls on a UTF-8 character boundary. The end
        /// of the text counts as one.
        /// </summary>
        private static bool HaddHarf(ReadOnlySpan<byte> naqi, int mawqi)
        {
            if (mawqi <= 0 || mawqi >= naqi.Length)
            {
                return mawqi == 0 || mawqi == naqi.Length;
            }
            return (naqi[mawqi] & 0xC0) != 0x80;
        }

        /// <summary>
        /// A counting writer over a caller's character buffer: it writes while
        /// there is room, counts always, and never throws. The same negotiation
        /// the parse and the mesh builder use, in the one place the rebuilds
        /// need it.
        /// </summary>
        private ref struct Katib
        {
            private readonly Span<char> _kharij;

            /// <summary>Starts a writer over a destination.</summary>
            public Katib(Span<char> kharij)
            {
                _kharij = kharij;
                Tul = 0;
                Kafa = true;
            }

            /// <summary>How many characters have been asked for.</summary>
            public int Tul;

            /// <summary>Whether every one of them fit.</summary>
            public bool Kafa;

            /// <summary>Writes one character.</summary>
            public void Harf(char harf)
            {
                if (Tul < _kharij.Length)
                {
                    _kharij[Tul] = harf;
                }
                else
                {
                    Kafa = false;
                }
                Tul++;
            }

            /// <summary>Writes a run of characters.</summary>
            public void Nass(ReadOnlySpan<char> nass)
            {
                for (int i = 0; i < nass.Length; i++)
                {
                    Harf(nass[i]);
                }
            }

            /// <summary>
            /// Decodes UTF-8 and writes the characters, emitting one U+FFFD per
            /// malformed byte rather than throwing.
            /// </summary>
            /// <remarks>
            /// Text produced by <see cref="Hallil"/> is valid UTF-8 by
            /// construction, so this substitution only ever fires on bytes that
            /// came from a patch file or a capture. A rebuild that threw there
            /// would take down the string the game is waiting for over one bad
            /// byte, and a replacement character is both visible and
            /// survivable.
            /// </remarks>
            public void Bayt(ReadOnlySpan<byte> bayt)
            {
                int i = 0;
                while (i < bayt.Length)
                {
                    byte awwal = bayt[i];
                    uint qeema;
                    int tul;
                    if (awwal < 0x80)
                    {
                        qeema = awwal;
                        tul = 1;
                    }
                    else if ((awwal & 0xE0) == 0xC0)
                    {
                        qeema = (uint)(awwal & 0x1F);
                        tul = 2;
                    }
                    else if ((awwal & 0xF0) == 0xE0)
                    {
                        qeema = (uint)(awwal & 0x0F);
                        tul = 3;
                    }
                    else if ((awwal & 0xF8) == 0xF0)
                    {
                        qeema = (uint)(awwal & 0x07);
                        tul = 4;
                    }
                    else
                    {
                        Harf('�');
                        i++;
                        continue;
                    }

                    if (i + tul > bayt.Length)
                    {
                        Harf('�');
                        i++;
                        continue;
                    }

                    bool salih = true;
                    for (int j = 1; j < tul; j++)
                    {
                        byte tabi = bayt[i + j];
                        if ((tabi & 0xC0) != 0x80)
                        {
                            salih = false;
                            break;
                        }
                        qeema = (qeema << 6) | (uint)(tabi & 0x3F);
                    }
                    if (!salih)
                    {
                        Harf('�');
                        i++;
                        continue;
                    }

                    i += tul;
                    if (qeema > 0x10FFFF || (qeema >= 0xD800 && qeema <= 0xDFFF))
                    {
                        Harf('�');
                    }
                    else if (qeema >= 0x10000)
                    {
                        uint fawq = qeema - 0x10000;
                        Harf((char)(0xD800 + (fawq >> 10)));
                        Harf((char)(0xDC00 + (fawq & 0x3FF)));
                    }
                    else
                    {
                        Harf((char)qeema);
                    }
                }
            }
        }

        /// <summary>
        /// A tag that has been opened and not yet closed.
        /// </summary>
        /// <remarks>
        /// <see cref="Lisan"/> is carried so that <c>[b]</c> cannot be closed
        /// by <c>&lt;/b&gt;</c>. Only one bracket syntax is live per dialect
        /// here, so the check is cheap insurance rather than a live concern —
        /// but matching a close against the wrong syntax would silently move a
        /// style span to cover the wrong words, and silent is the property that
        /// makes it worth a byte.
        /// </remarks>
        private struct Qaid
        {
            /// <summary>The span identifier reserved for it.</summary>
            public ushort Id;

            /// <summary>Where the tag's name starts in the raw string.</summary>
            public int WasmBidaya;

            /// <summary>The name's length, in characters.</summary>
            public int WasmTul;

            /// <summary>The clean byte offset the span opened at.</summary>
            public int BidayaNaqi;

            /// <summary>Which bracket syntax opened it: 0 angle, 1 square.</summary>
            public byte Lisan;

            /// <summary>
            /// Whether it set a colour. Carried on the stack rather than read
            /// back from the span table because NGUI's <c>[-]</c> pops the
            /// innermost colour specifically, and the span table may have
            /// overflowed — at which point the entry to read it from does not
            /// exist and the pop would silently target the wrong tag.
            /// </summary>
            public bool Lawn;
        }

        /// <summary>
        /// What one dialect made of a tag it was shown. The single shape every
        /// dialect resolver answers in, so the scanner around them contains no
        /// dialect knowledge at all — which is what keeps five dialects from
        /// becoming five scanners that drift apart.
        /// </summary>
        private struct NatijaWasm
        {
            /// <summary>
            /// The tag name is known but its value could not be read —
            /// <c>&lt;color=#gg&gt;</c> and its kind. The whole tag goes into
            /// the clean text as literal characters, exactly as the game's own
            /// parser leaves it. A resolver reports an unknown <em>name</em> by
            /// returning <c>false</c> instead; the two are distinguished only
            /// so the flag raised on the result names the real cause.
            /// </summary>
            public bool Marfud;

            /// <summary>
            /// A void tag: it has no closing form, so it opens no span and is
            /// simply removed.
            /// </summary>
            public bool Farigh;

            /// <summary>
            /// The single character this tag stands for, or <c>'\0'</c> when it
            /// stands for none: a newline for <c>&lt;br&gt;</c>, a space for
            /// <c>&lt;space&gt;</c>, a no-break space for <c>&lt;nbsp&gt;</c>.
            /// See <c>Massah.HallTextMeshPro</c> for why a character beats
            /// dropping the tag's effect.
            /// </summary>
            public char Mufrad;

            /// <summary>It is an inline sprite.</summary>
            public bool Sura;

            /// <summary>It opens a run the dialect must not interpret.</summary>
            public bool BilaTahleel;

            /// <summary>The <see cref="Alamat"/> span flags it sets.</summary>
            public uint Alam;

            /// <summary>Its colour, as <c>0xRRGGBBAA</c>.</summary>
            public uint Lawn;

            /// <summary>Its size in pixels.</summary>
            public float Hajm;

            /// <summary>Its weight.</summary>
            public ushort Wazn;

            /// <summary>A sprite index, or <c>-1</c>.</summary>
            public int Fahras;

            /// <summary>Where a sprite's name starts in the raw string.</summary>
            public int IsmBidaya;

            /// <summary>The name's length, in characters.</summary>
            public int IsmTul;
        }

        /// <summary>
        /// The scanner. One instance per parse, entirely on the stack, holding
        /// nothing but cursors into the caller's memory.
        /// </summary>
        private ref struct Massah
        {
            private readonly ReadOnlySpan<char> _khaam;
            private readonly KhiyaratNasq _khiyarat;
            private readonly MakhzanNasq _makhzan;
            private readonly Span<Qaid> _kadas;
            private int _mawdi;
            private int _tulNaqi;
            private int _adadNitaqat;
            private int _adadDharrat;
            private int _adadSijillat;
            private int _umq;
            private uint _alam;

            /// <summary>Starts a scan.</summary>
            public Massah(
                ReadOnlySpan<char> khaam,
                in KhiyaratNasq khiyarat,
                MakhzanNasq makhzan,
                Span<Qaid> kadas)
            {
                _khaam = khaam;
                _khiyarat = khiyarat;
                _makhzan = makhzan;
                _kadas = kadas;
                _mawdi = 0;
                _tulNaqi = 0;
                _adadNitaqat = 0;
                _adadDharrat = 0;
                _adadSijillat = 0;
                _umq = 0;
                _alam = 0;
            }

            /// <summary>
            /// Walks the string once. Every construct in every dialect here
            /// begins with one of a handful of ASCII characters, so an ordinary
            /// letter costs one comparison and one encode, which is what the
            /// overwhelming majority of a game's strings are made of.
            /// </summary>
            public void Imsah()
            {
                while (_mawdi < _khaam.Length)
                {
                    bool ultuqit;
                    switch (_khaam[_mawdi])
                    {
                        case '<':
                            ultuqit = Zawiya();
                            break;
                        case '[':
                            ultuqit = Murabba();
                            break;
                        case '{':
                            ultuqit = Muqawwas();
                            break;
                        case '}':
                            ultuqit = Mughlaq();
                            break;
                        case '%':
                            ultuqit = Miawi();
                            break;
                        case '&':
                            ultuqit = Kayan();
                            break;
                        default:
                            ultuqit = false;
                            break;
                    }
                    if (!ultuqit)
                    {
                        UktubHarfWahid();
                    }
                }
                AghliqMaftuh();
            }

            /// <summary>
            /// What the four destinations needed. Exact, because the scan runs
            /// to the end of the string whether or not anything fit.
            /// </summary>
            public QiyasNasq Matlub
            {
                get
                {
                    QiyasNasq matlub;
                    matlub.TulNass = _tulNaqi;
                    matlub.AdadNitaqat = _adadNitaqat;
                    matlub.AdadDharrat = _adadDharrat;
                    matlub.AdadSijillat = _adadSijillat;
                    return matlub;
                }
            }

            /// <summary>The <see cref="AlamatNasq"/> flags the scan raised.</summary>
            public uint Alam => _alam;

            // ---- output -----------------------------------------------------

            /// <summary>
            /// Appends one UTF-8 byte, writing while there is room and counting
            /// always. The count is the authority: an offset recorded past the
            /// end of the destination is still the offset the text would have
            /// had, which is what makes the reported requirement exact.
            /// </summary>
            private void Bayt(byte bayt)
            {
                if (_tulNaqi < _makhzan.Nass.Length)
                {
                    _makhzan.Nass[_tulNaqi] = bayt;
                }
                _tulNaqi++;
            }

            /// <summary>Encodes one code point as UTF-8. Returns its byte length.</summary>
            private int UktubQeema(uint qeema)
            {
                if (qeema < 0x80)
                {
                    Bayt((byte)qeema);
                    return 1;
                }
                if (qeema < 0x800)
                {
                    Bayt((byte)(0xC0 | (qeema >> 6)));
                    Bayt((byte)(0x80 | (qeema & 0x3F)));
                    return 2;
                }
                if (qeema < 0x10000)
                {
                    Bayt((byte)(0xE0 | (qeema >> 12)));
                    Bayt((byte)(0x80 | ((qeema >> 6) & 0x3F)));
                    Bayt((byte)(0x80 | (qeema & 0x3F)));
                    return 3;
                }
                Bayt((byte)(0xF0 | (qeema >> 18)));
                Bayt((byte)(0x80 | ((qeema >> 12) & 0x3F)));
                Bayt((byte)(0x80 | ((qeema >> 6) & 0x3F)));
                Bayt((byte)(0x80 | (qeema & 0x3F)));
                return 4;
            }

            /// <summary>
            /// Copies the character at the cursor into the clean text and
            /// advances past it, pairing a surrogate with its partner.
            /// </summary>
            /// <remarks>
            /// An unpaired surrogate is encoded as U+FFFD rather than as the
            /// three-byte CESU-8 form its value would produce, because those
            /// three bytes are not valid UTF-8 and every consumer downstream —
            /// the native layout call above all — is entitled to assume the
            /// text it is handed is.
            /// </remarks>
            private void UktubHarfWahid()
            {
                char h = _khaam[_mawdi];
                if (char.IsHighSurrogate(h)
                    && _mawdi + 1 < _khaam.Length
                    && char.IsLowSurrogate(_khaam[_mawdi + 1]))
                {
                    int adna = _khaam[_mawdi + 1] - 0xDC00;
                    UktubQeema((uint)(((h - 0xD800) << 10) + adna + 0x10000));
                    _mawdi += 2;
                    return;
                }
                UktubQeema(char.IsSurrogate(h) ? 0xFFFDu : h);
                _mawdi++;
            }

            /// <summary>
            /// Copies a window of the raw string into the clean text. Returns
            /// how many bytes it produced.
            /// </summary>
            private int UktubMada(int bidaya, int tul)
            {
                int qabl = _tulNaqi;
                int nihaya = bidaya + tul;
                int i = bidaya;
                while (i < nihaya && i < _khaam.Length)
                {
                    char h = _khaam[i];
                    if (char.IsHighSurrogate(h)
                        && i + 1 < nihaya
                        && i + 1 < _khaam.Length
                        && char.IsLowSurrogate(_khaam[i + 1]))
                    {
                        uint fawq =
                            (uint)(((h - 0xD800) << 10) + (_khaam[i + 1] - 0xDC00) + 0x10000);
                        UktubQeema(fawq);
                        i += 2;
                        continue;
                    }
                    UktubQeema(char.IsSurrogate(h) ? 0xFFFDu : h);
                    i++;
                }
                return _tulNaqi - qabl;
            }

            /// <summary>
            /// Leaves a construct in the clean text as literal characters and
            /// advances past it: the leniency every dialect here has, applied
            /// in the one place, so no handler can forget to advance the cursor
            /// and spin.
            /// </summary>
            private bool Harfiyyan(int tul, uint alam)
            {
                _alam |= alam;
                UktubMada(_mawdi, tul);
                _mawdi += tul;
                return true;
            }

            /// <summary>Appends one reinsertion record.</summary>
            private void Sajjil(
                NawSijill naw,
                ushort nitaq,
                int mawdiNaqi,
                int badeel,
                int aslBidaya,
                int aslTul)
            {
                if (_adadSijillat < _makhzan.Sijillat.Length)
                {
                    _makhzan.Sijillat[_adadSijillat] = new SijillNasq(
                        naw,
                        nitaq,
                        (uint)mawdiNaqi,
                        (uint)badeel,
                        aslBidaya,
                        aslTul);
                }
                _adadSijillat++;
            }

            // ---- spans ------------------------------------------------------

            /// <summary>
            /// Opens a span, reserving its identifier and writing everything
            /// about it but its length, which is not known until it closes.
            /// </summary>
            /// <remarks>
            /// Identifiers come from a running count rather than from the
            /// destination's length, so they stay dense and stay equal to their
            /// own index even when the span table overflowed and the entry was
            /// not written. A caller that grows and calls again gets the same
            /// identifiers for the same string, which is what lets a patch
            /// compiled from one parse be applied against another.
            /// </remarks>
            private void Iftah(
                in NatijaWasm wasm,
                int wasmBidaya,
                int wasmTul,
                byte lisan,
                int tagBidaya,
                int tagTul)
            {
                if (_umq >= _kadas.Length || _adadNitaqat >= AqsaNitaqat)
                {
                    _alam |= AlamatNasq.Farada;
                    Sajjil(NawSijill.Munfarid, BilaNitaq, _tulNaqi, 0, tagBidaya, tagTul);
                    return;
                }

                ushort id = (ushort)_adadNitaqat;
                if (_adadNitaqat < _makhzan.Nitaqat.Length)
                {
                    TaaribNitaqUslub nitaq = default;
                    nitaq.Id = id;
                    nitaq.Bidaya = (uint)_tulNaqi;
                    nitaq.Tul = 0;
                    nitaq.Alam = wasm.Alam;
                    nitaq.Lawn = wasm.Lawn;
                    nitaq.Hajm = wasm.Hajm;
                    nitaq.Wazn = wasm.Wazn;
                    _makhzan.Nitaqat[id] = nitaq;
                }
                _adadNitaqat++;

                Qaid qaid;
                qaid.Id = id;
                qaid.WasmBidaya = wasmBidaya;
                qaid.WasmTul = wasmTul;
                qaid.BidayaNaqi = _tulNaqi;
                qaid.Lisan = lisan;
                qaid.Lawn = (wasm.Alam & Alamat.UslubLawn) != 0;
                _kadas[_umq] = qaid;
                _umq++;

                if (qaid.Lawn)
                {
                    _alam |= AlamatNasq.FihAlwan;
                }
                Sajjil(NawSijill.Iftitah, id, _tulNaqi, 0, tagBidaya, tagTul);
            }

            /// <summary>
            /// Closes the innermost open tag with a matching name and bracket
            /// syntax, and patches its length.
            /// </summary>
            /// <remarks>
            /// The match need not be the top of the stack. Crossed tags —
            /// <c>&lt;b&gt;&lt;i&gt;x&lt;/b&gt;y&lt;/i&gt;</c> — are legal to
            /// every parser here, so the matched tag is removed from the middle
            /// of the stack and the ones above it stay open, which is what the
            /// game renders. Popping to the match instead would end
            /// <c>&lt;i&gt;</c> early and lose the emphasis on <c>y</c>.
            /// </remarks>
            private void Aghliq(int wasmBidaya, int wasmTul, byte lisan, int tagBidaya, int tagTul)
            {
                for (int i = _umq - 1; i >= 0; i--)
                {
                    if (_kadas[i].Lisan != lisan
                        || !Yutabiq(
                            _khaam.Slice(_kadas[i].WasmBidaya, _kadas[i].WasmTul),
                            _khaam.Slice(wasmBidaya, wasmTul)))
                    {
                        continue;
                    }

                    AghliqQaid(i, tagBidaya, tagTul);
                    return;
                }

                _alam |= AlamatNasq.WasmMughlaqZaid;
                Sajjil(NawSijill.Munfarid, BilaNitaq, _tulNaqi, 0, tagBidaya, tagTul);
            }

            /// <summary>
            /// Closes the innermost open colour, which is what NGUI's
            /// <c>[-]</c> does. A <c>[-]</c> with no colour open is dropped,
            /// exactly as NGUI drops it.
            /// </summary>
            private void AghliqLawn(int tagBidaya, int tagTul)
            {
                for (int i = _umq - 1; i >= 0; i--)
                {
                    if (!_kadas[i].Lawn)
                    {
                        continue;
                    }
                    AghliqQaid(i, tagBidaya, tagTul);
                    return;
                }
                _alam |= AlamatNasq.WasmMughlaqZaid;
                Sajjil(NawSijill.Munfarid, BilaNitaq, _tulNaqi, 0, tagBidaya, tagTul);
            }

            /// <summary>
            /// Ends the span held at one stack position, patching its length,
            /// removing it from the middle of the stack, and recording the
            /// closing tag against its identifier.
            /// </summary>
            private void AghliqQaid(int mawqi, int tagBidaya, int tagTul)
            {
                ushort id = _kadas[mawqi].Id;
                if (id < _makhzan.Nitaqat.Length)
                {
                    _makhzan.Nitaqat[id].Tul = (uint)(_tulNaqi - _kadas[mawqi].BidayaNaqi);
                }
                for (int j = mawqi; j < _umq - 1; j++)
                {
                    _kadas[j] = _kadas[j + 1];
                }
                _umq--;
                Sajjil(NawSijill.Ikhtitam, id, _tulNaqi, 0, tagBidaya, tagTul);
            }

            /// <summary>
            /// Closes whatever is still open at the end of the string, which is
            /// what every dialect here does with an unclosed tag.
            /// </summary>
            /// <remarks>
            /// Each closing record names an empty window into the raw string,
            /// because there was no closing tag to name. That is what makes the
            /// rebuild exact: a source string that ended mid-emphasis comes back
            /// out ending mid-emphasis, rather than acquiring a
            /// <c>&lt;/b&gt;</c> the game never wrote.
            /// </remarks>
            private void AghliqMaftuh()
            {
                while (_umq > 0)
                {
                    _umq--;
                    ushort id = _kadas[_umq].Id;
                    if (id < _makhzan.Nitaqat.Length)
                    {
                        _makhzan.Nitaqat[id].Tul = (uint)(_tulNaqi - _kadas[_umq].BidayaNaqi);
                    }
                    _alam |= AlamatNasq.WasmMaftuh;
                    Sajjil(NawSijill.Ikhtitam, id, _tulNaqi, 0, _khaam.Length, 0);
                }
            }

            // ---- atoms ------------------------------------------------------

            /// <summary>
            /// Emits one atom: its span, its clean-text stand-in, its entry in
            /// the atom table and its reinsertion record.
            /// </summary>
            /// <remarks>
            /// <para>
            /// A placeholder and a sprite collapse to one
            /// <see cref="BadeelDharra"/>. A <see cref="NawDharra.Khaam"/> run
            /// keeps its own bytes instead, because the game means that text to
            /// be read — what the atom flag takes away from it is the right to
            /// be shaped or reordered, not the right to be drawn.
            /// </para>
            /// <para>
            /// That distinction is a seam an adapter has to honour: a span
            /// carrying <see cref="Alamat.UslubDharra"/> is an engine-drawn
            /// sprite only when its atom's <see cref="DharraNasq.Naw"/> says
            /// so. <see cref="TaaribNitaqUslub.ArdDharra"/> is left at zero for
            /// a <see cref="NawDharra.Khaam"/> run, which means "measure it"
            /// rather than "no width", and the run's own glyphs supply the
            /// measurement.
            /// </para>
            /// </remarks>
            private void AdrijDharra(
                NawDharra naw,
                int aslBidaya,
                int aslTul,
                int fahras,
                int tarteeb,
                int ismBidaya,
                int ismTul,
                int matnBidaya,
                int matnTul)
            {
                int bidayaNaqi = _tulNaqi;
                bool masaha = _adadNitaqat < AqsaNitaqat;
                ushort id = masaha ? (ushort)_adadNitaqat : BilaNitaq;
                if (masaha)
                {
                    _adadNitaqat++;
                }
                else
                {
                    _alam |= AlamatNasq.Farada;
                }

                int tulNaqi = naw == NawDharra.Khaam
                    ? UktubMada(matnBidaya, matnTul)
                    : UktubQeema(BadeelDharra);

                if (masaha && id < _makhzan.Nitaqat.Length)
                {
                    TaaribNitaqUslub nitaq = default;
                    nitaq.Id = id;
                    nitaq.Bidaya = (uint)bidayaNaqi;
                    nitaq.Tul = (uint)tulNaqi;
                    nitaq.Alam = Alamat.UslubDharra;
                    nitaq.MarjaDharra = (uint)_adadDharrat;
                    if (naw != NawDharra.Khaam)
                    {
                        nitaq.ArdDharra = _khiyarat.ArdDharraIftiradi;
                        nitaq.IrtifaDharra = _khiyarat.IrtifaDharraIftiradi;
                        nitaq.AsasDharra = _khiyarat.AsasDharraIftiradi;
                    }
                    _makhzan.Nitaqat[id] = nitaq;
                }

                if (_adadDharrat < _makhzan.Dharrat.Length)
                {
                    _makhzan.Dharrat[_adadDharrat] = new DharraNasq(
                        naw,
                        id,
                        (uint)bidayaNaqi,
                        (uint)tulNaqi,
                        aslBidaya,
                        aslTul,
                        fahras,
                        tarteeb,
                        ismBidaya,
                        ismTul);
                }
                _adadDharrat++;

                _alam |= AlamatNasq.FihDharrat;
                if (naw == NawDharra.Khaam)
                {
                    _alam |= AlamatNasq.FihKhaam;
                }
                Sajjil(NawSijill.Dharra, id, bidayaNaqi, tulNaqi, aslBidaya, aslTul);
            }

            /// <summary>
            /// Writes the one character a void tag stands for — a newline for
            /// <c>&lt;br&gt;</c>, a space for <c>&lt;space&gt;</c> — and
            /// records the tag that asked for it.
            /// </summary>
            /// <remarks>
            /// These records are restored by <see cref="AadBinaaAsl"/> and not
            /// by <see cref="AadBinaaTarjama"/>, and that asymmetry is
            /// deliberate. Their whole effect is already in the clean text as a
            /// character, so a translation carries it in its own text, at its
            /// own place; re-emitting the source's <c>&lt;br&gt;</c> at the
            /// source's offset would put a line break wherever the English
            /// happened to break, which in Arabic is the middle of a phrase.
            /// </remarks>
            private void HarfMufrad(char harf, int tagBidaya, int tagTul)
            {
                int bidayaNaqi = _tulNaqi;
                int badeel = UktubQeema(harf);
                Sajjil(NawSijill.Munfarid, BilaNitaq, bidayaNaqi, badeel, tagBidaya, tagTul);
            }

            // ---- constructs -------------------------------------------------

            /// <summary>
            /// An angle-bracket tag, for the four dialects that have them.
            /// </summary>
            /// <remarks>
            /// The scanner here knows about brackets, closing forms,
            /// self-closing forms and nothing else. Which names exist and what
            /// they mean is the dialect resolver's business, so adding a
            /// dialect is one resolver rather than a second scanner.
            /// </remarks>
            private bool Zawiya()
            {
                NawNasq naw = _khiyarat.Naw;
                if (naw != NawNasq.TextMeshPro
                    && naw != NawNasq.WajihatUnity
                    && naw != NawNasq.FairyGui
                    && naw != NawNasq.Html)
                {
                    return false;
                }

                int nihaya = TaliQaws('<', '>');
                if (nihaya < 0)
                {
                    return false;
                }

                int muhtawaBidaya = _mawdi + 1;
                int muhtawaTul = nihaya - muhtawaBidaya;
                int tagBidaya = _mawdi;
                int tagTul = (nihaya - _mawdi) + 1;
                if (muhtawaTul <= 0)
                {
                    return false;
                }

                if (_khaam[muhtawaBidaya] == '/')
                {
                    int wasmBidaya = muhtawaBidaya + 1;
                    int wasmTul = muhtawaTul - 1;
                    Qass(ref wasmBidaya, ref wasmTul);
                    if (wasmTul <= 0
                        || !YurafWasm(naw, _khaam.Slice(wasmBidaya, wasmTul)))
                    {
                        return Harfiyyan(tagTul, AlamatNasq.WasmMajhul);
                    }
                    _mawdi = nihaya + 1;
                    Aghliq(wasmBidaya, wasmTul, 0, tagBidaya, tagTul);
                    return true;
                }

                int jismTul = muhtawaTul;
                bool dhati = _khaam[muhtawaBidaya + jismTul - 1] == '/';
                if (dhati)
                {
                    jismTul--;
                }
                if (jismTul <= 0)
                {
                    return Harfiyyan(tagTul, AlamatNasq.WasmMajhul);
                }

                ReadOnlySpan<char> jism = _khaam.Slice(muhtawaBidaya, jismTul);
                int tulWasm = 0;
                while (tulWasm < jism.Length && !HadFasl(jism[tulWasm]))
                {
                    tulWasm++;
                }
                if (tulWasm == 0)
                {
                    return Harfiyyan(tagTul, AlamatNasq.WasmMajhul);
                }

                NatijaWasm wasm = default;
                wasm.Fahras = -1;
                int jb = muhtawaBidaya;
                bool urifa = naw switch
                {
                    NawNasq.TextMeshPro => HallTextMeshPro(jism, jb, tulWasm, ref wasm),
                    NawNasq.WajihatUnity => HallWajihatUnity(jism, jb, tulWasm, ref wasm),
                    NawNasq.FairyGui => HallFairyGui(jism, jb, tulWasm, ref wasm),
                    _ => HallHtml(jism, jb, tulWasm, ref wasm),
                };

                if (!urifa)
                {
                    return Harfiyyan(tagTul, AlamatNasq.WasmMajhul);
                }
                if (wasm.Marfud)
                {
                    return Harfiyyan(tagTul, AlamatNasq.QeemaTalifa);
                }

                _mawdi = nihaya + 1;

                if (wasm.BilaTahleel)
                {
                    return KhaamMada(tagBidaya, tagTul, muhtawaBidaya, tulWasm);
                }
                if (wasm.Sura)
                {
                    AdrijDharra(
                        NawDharra.Sura,
                        tagBidaya,
                        tagTul,
                        wasm.Fahras,
                        -1,
                        wasm.IsmBidaya,
                        wasm.IsmTul,
                        0,
                        0);
                    return true;
                }
                if (wasm.Mufrad != '\0')
                {
                    HarfMufrad(wasm.Mufrad, tagBidaya, tagTul);
                    return true;
                }
                if (wasm.Farigh || dhati)
                {
                    Sajjil(NawSijill.Munfarid, BilaNitaq, _tulNaqi, 0, tagBidaya, tagTul);
                    return true;
                }
                Iftah(in wasm, muhtawaBidaya, tulWasm, 0, tagBidaya, tagTul);
                return true;
            }

            /// <summary>
            /// Takes an uninterpreted run — <c>&lt;noparse&gt;…&lt;/noparse&gt;</c>
            /// and its equivalents — up to its own closing tag.
            /// </summary>
            /// <remarks>
            /// Nothing inside is scanned. That is the whole meaning of the tag:
            /// the game is being told that the characters between are
            /// characters, and a scanner that looked at them anyway would eat
            /// the very <c>&lt;color&gt;</c> the string was written to display.
            /// An unclosed run reaches the end of the string, which is what
            /// TextMeshPro does with it.
            /// </remarks>
            private bool KhaamMada(int tagBidaya, int tagTul, int wasmBidaya, int wasmTul)
            {
                int matnBidaya = _mawdi;
                int ghalq = IbhathGhalq(wasmBidaya, wasmTul);
                int matnTul;
                int kullTul;
                if (ghalq < 0)
                {
                    matnTul = _khaam.Length - matnBidaya;
                    kullTul = _khaam.Length - tagBidaya;
                    _mawdi = _khaam.Length;
                    _alam |= AlamatNasq.WasmMaftuh;
                }
                else
                {
                    matnTul = ghalq - matnBidaya;
                    int ghalqTul = wasmTul + 3;
                    kullTul = (ghalq + ghalqTul) - tagBidaya;
                    _mawdi = ghalq + ghalqTul;
                }
                AdrijDharra(
                    NawDharra.Khaam,
                    tagBidaya,
                    kullTul,
                    -1,
                    -1,
                    0,
                    0,
                    matnBidaya,
                    matnTul);
                return true;
            }

            /// <summary>
            /// Where <c>&lt;/name&gt;</c> begins at or after the cursor, or
            /// <c>-1</c>.
            /// </summary>
            private int IbhathGhalq(int wasmBidaya, int wasmTul)
            {
                ReadOnlySpan<char> wasm = _khaam.Slice(wasmBidaya, wasmTul);
                int hadd = _khaam.Length - (wasmTul + 3);
                for (int i = _mawdi; i <= hadd; i++)
                {
                    if (_khaam[i] != '<'
                        || _khaam[i + 1] != '/'
                        || _khaam[i + wasmTul + 2] != '>')
                    {
                        continue;
                    }
                    if (Yutabiq(_khaam.Slice(i + 2, wasmTul), wasm))
                    {
                        return i;
                    }
                }
                return -1;
            }

            /// <summary>
            /// The index of the closing bracket, or <c>-1</c> when this bracket
            /// is punctuation rather than the start of a tag.
            /// </summary>
            /// <remarks>
            /// The search stops at a newline and at a second opening bracket.
            /// Stopping at the second bracket is what makes <c>a &lt; b &lt;i&gt;</c>
            /// recover: the first <c>&lt;</c> becomes the character it plainly
            /// is, and the real tag after it is still found. Running to the
            /// first <c>&gt;</c> instead would swallow the text between as if it
            /// were a tag body.
            /// </remarks>
            private int TaliQaws(char fath, char ghalq)
            {
                int hadd = _mawdi + 1 + HaddWasm;
                if (hadd > _khaam.Length)
                {
                    hadd = _khaam.Length;
                }
                for (int i = _mawdi + 1; i < hadd; i++)
                {
                    char h = _khaam[i];
                    if (h == ghalq)
                    {
                        return i;
                    }
                    if (h == fath || h == '\n' || h == '\r')
                    {
                        return -1;
                    }
                }
                return -1;
            }

            /// <summary>Trims ASCII whitespace from both ends of a window.</summary>
            private void Qass(ref int bidaya, ref int tul)
            {
                while (tul > 0 && Faragh(_khaam[bidaya]))
                {
                    bidaya++;
                    tul--;
                }
                while (tul > 0 && Faragh(_khaam[bidaya + tul - 1]))
                {
                    tul--;
                }
            }

            /// <summary>
            /// NGUI's bracket markup: a pushed colour, its <c>[-]</c> pop, the
            /// named toggles, and <c>[[</c> for a literal bracket.
            /// </summary>
            private bool Murabba()
            {
                if (_khiyarat.Naw != NawNasq.NGui)
                {
                    return false;
                }

                if (_mawdi + 1 < _khaam.Length && _khaam[_mawdi + 1] == '[')
                {
                    HarfMahrub('[', 2);
                    return true;
                }

                int nihaya = TaliQaws('[', ']');
                if (nihaya < 0)
                {
                    return false;
                }

                int muhtawaBidaya = _mawdi + 1;
                int muhtawaTul = nihaya - muhtawaBidaya;
                if (muhtawaTul <= 0)
                {
                    return false;
                }

                int tagBidaya = _mawdi;
                int tagTul = (nihaya - _mawdi) + 1;
                ReadOnlySpan<char> muhtawa = _khaam.Slice(muhtawaBidaya, muhtawaTul);

                if (muhtawaTul == 1 && muhtawa[0] == '-')
                {
                    _mawdi = nihaya + 1;
                    AghliqLawn(tagBidaya, tagTul);
                    return true;
                }

                if (muhtawa[0] == '/')
                {
                    int wasmBidaya = muhtawaBidaya + 1;
                    int wasmTul = muhtawaTul - 1;
                    Qass(ref wasmBidaya, ref wasmTul);
                    if (wasmTul <= 0
                        || !YurafWasm(NawNasq.NGui, _khaam.Slice(wasmBidaya, wasmTul)))
                    {
                        return Harfiyyan(tagTul, AlamatNasq.WasmMajhul);
                    }
                    _mawdi = nihaya + 1;
                    Aghliq(wasmBidaya, wasmTul, 1, tagBidaya, tagTul);
                    return true;
                }

                NatijaWasm wasm = default;
                wasm.Fahras = -1;
                if (!HallNGui(muhtawa, ref wasm))
                {
                    return Harfiyyan(tagTul, AlamatNasq.WasmMajhul);
                }
                if (wasm.Marfud)
                {
                    return Harfiyyan(tagTul, AlamatNasq.QeemaTalifa);
                }

                _mawdi = nihaya + 1;
                int tulWasm = 0;
                while (tulWasm < muhtawaTul && muhtawa[tulWasm] != '=')
                {
                    tulWasm++;
                }
                Iftah(in wasm, muhtawaBidaya, tulWasm, 1, tagBidaya, tagTul);
                return true;
            }

            /// <summary>
            /// A brace construct: the <c>{{</c> escape, or a format
            /// placeholder.
            /// </summary>
            /// <remarks>
            /// Recognised in every dialect, including
            /// <see cref="NawNasq.Bila"/>, because a placeholder is the game
            /// formatter's business rather than any renderer's: a string with
            /// rich text switched off still gets <c>string.Format</c> called on
            /// it, and a <c>{0}</c> whose braces were reordered by the
            /// bidirectional algorithm is either an exception out of the
            /// formatter or a value substituted into the wrong sentence.
            /// </remarks>
            private bool Muqawwas()
            {
                if (_mawdi + 1 < _khaam.Length && _khaam[_mawdi + 1] == '{')
                {
                    HarfMahrub('{', 2);
                    return true;
                }

                int nihaya = TaliQaws('{', '}');
                if (nihaya < 0)
                {
                    return false;
                }
                int muhtawaBidaya = _mawdi + 1;
                int muhtawaTul = nihaya - muhtawaBidaya;
                if (muhtawaTul <= 0
                    || !MawdiSalih(_khaam.Slice(muhtawaBidaya, muhtawaTul), out int tarteeb))
                {
                    return false;
                }

                int aslBidaya = _mawdi;
                int tul = (nihaya - _mawdi) + 1;
                _mawdi = nihaya + 1;
                AdrijDharra(NawDharra.Mawdi, aslBidaya, tul, -1, tarteeb, 0, 0, 0, 0);
                return true;
            }

            /// <summary>The <c>}}</c> escape. A lone brace is a brace.</summary>
            private bool Mughlaq()
            {
                if (_mawdi + 1 >= _khaam.Length || _khaam[_mawdi + 1] != '}')
                {
                    return false;
                }
                HarfMahrub('}', 2);
                return true;
            }

            /// <summary>
            /// A percent construct: the <c>%%</c> escape, or a printf-style
            /// placeholder.
            /// </summary>
            private bool Miawi()
            {
                if (_mawdi + 1 < _khaam.Length && _khaam[_mawdi + 1] == '%')
                {
                    HarfMahrub('%', 2);
                    return true;
                }
                if (!TabaSalih(out int tul, out int tarteeb))
                {
                    return false;
                }
                int aslBidaya = _mawdi;
                _mawdi += tul;
                AdrijDharra(NawDharra.Mawdi, aslBidaya, tul, -1, tarteeb, 0, 0, 0, 0);
                return true;
            }

            /// <summary>An HTML character entity.</summary>
            private bool Kayan()
            {
                if (_khiyarat.Naw != NawNasq.Html)
                {
                    return false;
                }
                int awwal = _mawdi + 1;
                int hadd = awwal + 12;
                if (hadd > _khaam.Length)
                {
                    hadd = _khaam.Length;
                }
                int fasl = -1;
                for (int i = awwal; i < hadd; i++)
                {
                    char h = _khaam[i];
                    if (h == ';')
                    {
                        fasl = i;
                        break;
                    }
                    if (h == '&' || Faragh(h))
                    {
                        break;
                    }
                }
                if (fasl <= awwal)
                {
                    return false;
                }

                ReadOnlySpan<char> ism = _khaam.Slice(awwal, fasl - awwal);
                uint qeema;
                if (ism[0] == '#')
                {
                    if (!QeematKayan(ism.Slice(1), out qeema))
                    {
                        return false;
                    }
                }
                else if (!KayanMusamma(ism, out qeema))
                {
                    return false;
                }

                int bidayaNaqi = _tulNaqi;
                int badeel = UktubQeema(qeema);
                Sajjil(
                    NawSijill.Harf,
                    BilaNitaq,
                    bidayaNaqi,
                    badeel,
                    _mawdi,
                    (fasl - _mawdi) + 1);
                _mawdi = fasl + 1;
                return true;
            }

            /// <summary>
            /// Writes the single character a doubled escape stands for, and
            /// records the doubled form so the rebuild puts it back.
            /// </summary>
            /// <remarks>
            /// Putting it back matters more than it looks. Emitting <c>{</c>
            /// where the game wrote <c>{{</c> hands the game's formatter a
            /// string that means something else — an unmatched brace, and an
            /// exception rather than a character.
            /// </remarks>
            private void HarfMahrub(char harf, int tul)
            {
                int bidayaNaqi = _tulNaqi;
                int badeel = UktubQeema(harf);
                Sajjil(NawSijill.Harf, BilaNitaq, bidayaNaqi, badeel, _mawdi, tul);
                _mawdi += tul;
            }

            /// <summary>
            /// Whether the text between a pair of braces is a format item.
            /// </summary>
            /// <remarks>
            /// The shape checked is the composite format item every .NET
            /// formatter accepts — <c>index[,alignment][:format]</c> — widened
            /// to allow a name where an index goes, because engines that build
            /// their own formatter on top of it use names. The check is strict
            /// about the name deliberately: letters, digits, underscore, dot
            /// and hyphen only, and no whitespace. Prose in braces is a real
            /// thing in game text — a stage direction, a bracketed aside — and
            /// turning one into an opaque atom would take a sentence off the
            /// translator's screen and put an unbreakable run into the layout.
            /// </remarks>
            private static bool MawdiSalih(ReadOnlySpan<char> muhtawa, out int tarteeb)
            {
                tarteeb = -1;
                int n = muhtawa.Length;
                int i = 0;
                while (i < n && muhtawa[i] != ',' && muhtawa[i] != ':')
                {
                    i++;
                }
                if (i == 0)
                {
                    return false;
                }

                ReadOnlySpan<char> ism = muhtawa.Slice(0, i);
                if (ism[0] == '.' || ism[0] == '-')
                {
                    return false;
                }
                bool arqam = true;
                for (int j = 0; j < ism.Length; j++)
                {
                    char h = ism[j];
                    if (Raqm(h))
                    {
                        continue;
                    }
                    arqam = false;
                    if (!Abjadi(h) && h != '_' && h != '.' && h != '-')
                    {
                        return false;
                    }
                }
                if (arqam)
                {
                    tarteeb = SaheehMin(ism);
                }

                if (i < n && muhtawa[i] == ',')
                {
                    i++;
                    if (i < n && (muhtawa[i] == '-' || muhtawa[i] == '+'))
                    {
                        i++;
                    }
                    int qabl = i;
                    while (i < n && Raqm(muhtawa[i]))
                    {
                        i++;
                    }
                    if (i == qabl)
                    {
                        return false;
                    }
                }

                return i >= n || muhtawa[i] == ':';
            }

            /// <summary>
            /// Whether a printf-style placeholder starts at the cursor, and how
            /// long it is.
            /// </summary>
            /// <remarks>
            /// <para>
            /// The shape is the C one:
            /// <c>%[argnum$][flags][width][.precision][length]conversion</c>.
            /// Two deliberate narrowings keep it from eating ordinary prose,
            /// because a percent sign is a character English and Arabic both
            /// use.
            /// </para>
            /// <para>
            /// <b>Three conversions are not recognised at all.</b> <c>o</c>
            /// (octal), <c>a</c> and <c>A</c> (hexadecimal float) and
            /// <c>n</c> (write-back) are absent from real game strings and
            /// present at the start of a great many English words, so
            /// <c>50%off</c> would otherwise become an atom and lose three
            /// letters.
            /// </para>
            /// <para>
            /// <b>The bare two-character form must end on a boundary.</b>
            /// <c>%s</c> is a placeholder when the next character is not an
            /// ASCII letter, and <c>%save</c> is a word. Anything carrying a
            /// flag, a width, a precision or an argument number is unambiguous
            /// and skips the test. This trades a false negative — an unprotected
            /// placeholder in a string like <c>%dmg</c> — against a false
            /// positive, which would delete letters a player is meant to read;
            /// and a false negative is the one a translator can see and report.
            /// </para>
            /// </remarks>
            private bool TabaSalih(out int tul, out int tarteeb)
            {
                tul = 0;
                tarteeb = -1;
                int n = _khaam.Length;
                int i = _mawdi + 1;
                if (i >= n)
                {
                    return false;
                }

                bool zukhruf = false;
                int qabl = i;
                while (i < n && Raqm(_khaam[i]))
                {
                    i++;
                }
                if (i > qabl && i < n && _khaam[i] == '$')
                {
                    tarteeb = SaheehMin(_khaam.Slice(qabl, i - qabl));
                    i++;
                    zukhruf = true;
                }
                else
                {
                    i = qabl;
                }

                while (i < n && Alama(_khaam[i]))
                {
                    i++;
                    zukhruf = true;
                }
                while (i < n && Raqm(_khaam[i]))
                {
                    i++;
                    zukhruf = true;
                }
                if (i < n && _khaam[i] == '.')
                {
                    i++;
                    zukhruf = true;
                    while (i < n && Raqm(_khaam[i]))
                    {
                        i++;
                    }
                }
                while (i < n && Tuli(_khaam[i]))
                {
                    i++;
                }

                if (i >= n || !TahweelSalih(_khaam[i]))
                {
                    return false;
                }
                i++;
                if (!zukhruf && i < n && Abjadi(_khaam[i]))
                {
                    return false;
                }
                tul = i - _mawdi;
                return true;
            }

            // ---- dialects ---------------------------------------------------

            /// <summary>
            /// Whether a dialect knows a tag name, for deciding what to do with
            /// a closing tag.
            /// </summary>
            /// <remarks>
            /// Answered by running the dialect's own resolver over the bare
            /// name rather than by a second list of names beside it. Two lists
            /// of the same names is one of them being wrong after the next tag
            /// is added — and the symptom would be a <c>&lt;/color&gt;</c>
            /// printed on screen while its <c>&lt;color&gt;</c> worked.
            /// </remarks>
            private bool YurafWasm(NawNasq naw, ReadOnlySpan<char> wasm)
            {
                NatijaWasm munt = default;
                munt.Fahras = -1;
                return naw switch
                {
                    NawNasq.TextMeshPro => HallTextMeshPro(wasm, 0, wasm.Length, ref munt),
                    NawNasq.WajihatUnity => HallWajihatUnity(wasm, 0, wasm.Length, ref munt),
                    NawNasq.FairyGui => HallFairyGui(wasm, 0, wasm.Length, ref munt),
                    NawNasq.Html => HallHtml(wasm, 0, wasm.Length, ref munt),
                    NawNasq.NGui => HallNGui(wasm, ref munt),
                    _ => false,
                };
            }

            /// <summary>
            /// TextMeshPro's rich text.
            /// </summary>
            /// <remarks>
            /// <para>
            /// Four groups, and the difference between them is what the tag
            /// does at the end of a string rather than what it looks like.
            /// Tags that open a span have a closing form and an unclosed one
            /// runs to the end, which is TextMeshPro's own behaviour. Void tags
            /// have no closing form. Sprites and <c>&lt;noparse&gt;</c> are
            /// atoms. Everything else is characters.
            /// </para>
            /// <para>
            /// <c>&lt;font="Bold"&gt;</c> opens a span that sets nothing.
            /// <see cref="TaaribNitaqUslub.Khatt"/> is an index into the font
            /// chain the patch built, and a family name from the game's own
            /// asset table cannot be turned into one here without a lookup this
            /// assembly has no way to perform. The span still exists, so the
            /// tag round-trips exactly; what it does not do is guess a font,
            /// which would render a run in the wrong typeface with no
            /// diagnostic.
            /// </para>
            /// <para>
            /// <c>&lt;space=8&gt;</c> becomes one ordinary space. Taarib has no
            /// way to inject a measured gap at this seam, and a space is a
            /// closer approximation of the author's intent than deleting the
            /// gap entirely — while the record still restores the exact tag if
            /// the string is handed back to the game.
            /// </para>
            /// </remarks>
            private bool HallTextMeshPro(
                ReadOnlySpan<char> jism,
                int jismBidaya,
                int tulWasm,
                ref NatijaWasm wasm)
            {
                ReadOnlySpan<char> ism = jism.Slice(0, tulWasm);
                if (ism[0] == '#')
                {
                    if (!LawnMin(ism, out uint lawnMubashir))
                    {
                        wasm.Marfud = true;
                        return true;
                    }
                    wasm.Alam = Alamat.UslubLawn;
                    wasm.Lawn = lawnMubashir;
                    return true;
                }

                if (Yutabiq(ism, "b"))
                {
                    wasm.Alam = Alamat.UslubWazn;
                    wasm.Wazn = WaznGhaliz;
                    return true;
                }
                if (Yutabiq(ism, "i"))
                {
                    wasm.Alam = Alamat.UslubMaail;
                    return true;
                }
                if (Yutabiq(ism, "noparse"))
                {
                    wasm.BilaTahleel = true;
                    return true;
                }
                if (Yutabiq(ism, "br"))
                {
                    wasm.Mufrad = '\n';
                    return true;
                }
                if (Yutabiq(ism, "sprite"))
                {
                    wasm.Sura = true;
                    QarraSifat(jism, jismBidaya, ref wasm);
                    return true;
                }
                if (Yutabiq(ism, "color"))
                {
                    return LawnSifa(jism, ref wasm);
                }
                if (Yutabiq(ism, "size"))
                {
                    return HajmSifa(jism, ref wasm);
                }
                if (Yutabiq(ism, "space"))
                {
                    wasm.Mufrad = ' ';
                    return true;
                }
                if (Yutabiq(ism, "nbsp"))
                {
                    wasm.Mufrad = '\u00A0';
                    return true;
                }
                if (Yutabiq(ism, "pos") || Yutabiq(ism, "page"))
                {
                    wasm.Farigh = true;
                    return true;
                }
                return Yutabiq(ism, "u")
                    || Yutabiq(ism, "s")
                    || Yutabiq(ism, "sup")
                    || Yutabiq(ism, "sub")
                    || Yutabiq(ism, "mark")
                    || Yutabiq(ism, "link")
                    || Yutabiq(ism, "font")
                    || Yutabiq(ism, "align")
                    || Yutabiq(ism, "alpha")
                    || Yutabiq(ism, "style")
                    || Yutabiq(ism, "width")
                    || Yutabiq(ism, "indent")
                    || Yutabiq(ism, "margin")
                    || Yutabiq(ism, "cspace")
                    || Yutabiq(ism, "mspace")
                    || Yutabiq(ism, "voffset")
                    || Yutabiq(ism, "rotate")
                    || Yutabiq(ism, "nobr")
                    || Yutabiq(ism, "gradient")
                    || Yutabiq(ism, "material")
                    || Yutabiq(ism, "allcaps")
                    || Yutabiq(ism, "smallcaps")
                    || Yutabiq(ism, "uppercase")
                    || Yutabiq(ism, "lowercase")
                    || Yutabiq(ism, "line-height")
                    || Yutabiq(ism, "strikethrough")
                    || Yutabiq(ism, "underline");
            }

            /// <summary>
            /// Unity's legacy UI text, which knows six tags and treats
            /// everything else as characters.
            /// </summary>
            /// <remarks>
            /// The narrowness is the point. A game whose labels are the legacy
            /// component and whose strings contain <c>&lt;sprite=3&gt;</c> is
            /// showing its players the eleven characters <c>&lt;sprite=3&gt;</c>,
            /// and a bridge that helpfully parsed it would delete text the game
            /// draws.
            /// </remarks>
            private bool HallWajihatUnity(
                ReadOnlySpan<char> jism,
                int jismBidaya,
                int tulWasm,
                ref NatijaWasm wasm)
            {
                ReadOnlySpan<char> ism = jism.Slice(0, tulWasm);
                if (Yutabiq(ism, "b"))
                {
                    wasm.Alam = Alamat.UslubWazn;
                    wasm.Wazn = WaznGhaliz;
                    return true;
                }
                if (Yutabiq(ism, "i"))
                {
                    wasm.Alam = Alamat.UslubMaail;
                    return true;
                }
                if (Yutabiq(ism, "color"))
                {
                    return LawnSifa(jism, ref wasm);
                }
                if (Yutabiq(ism, "size"))
                {
                    return HajmSifa(jism, ref wasm);
                }
                if (Yutabiq(ism, "quad"))
                {
                    wasm.Sura = true;
                    QarraSifat(jism, jismBidaya, ref wasm);
                    return true;
                }
                return Yutabiq(ism, "material");
            }

            /// <summary>
            /// A colour tag whose value is its own attribute:
            /// <c>&lt;color=#ff0000&gt;</c>, <c>&lt;color="red"&gt;</c>,
            /// <c>&lt;color=red&gt;</c>.
            /// </summary>
            private bool LawnSifa(ReadOnlySpan<char> jism, ref NatijaWasm wasm)
            {
                if (!QeematWasm(jism, out ReadOnlySpan<char> qeema)
                    || !LawnMin(qeema, out uint lawn))
                {
                    wasm.Marfud = true;
                    return true;
                }
                wasm.Alam = Alamat.UslubLawn;
                wasm.Lawn = lawn;
                return true;
            }

            /// <summary>
            /// A size tag: absolute (<c>24</c>), relative (<c>+2</c>) or
            /// proportional (<c>120%</c>).
            /// </summary>
            /// <remarks>
            /// A value that is not a number at all leaves the tag as literal
            /// text. A value that is a number but cannot be resolved — a
            /// relative or proportional size with no
            /// <see cref="KhiyaratNasq.HajmAsas"/> to be relative to — still
            /// opens its span, carrying no size override. The span is what
            /// makes the tag round-trip; the missing override is the honest
            /// answer to a question the caller did not supply the input for.
            /// </remarks>
            private bool HajmSifa(ReadOnlySpan<char> jism, ref NatijaWasm wasm)
            {
                if (!QeematWasm(jism, out ReadOnlySpan<char> qeema)
                    || !HajmMin(qeema, out float hajm))
                {
                    wasm.Marfud = true;
                    return true;
                }
                if (hajm > 0f)
                {
                    wasm.Alam = Alamat.UslubHajm;
                    wasm.Hajm = hajm;
                }
                return true;
            }

            /// <summary>
            /// FairyGUI's HTML-ish subset, whose styling lives in quoted
            /// attributes on <c>&lt;font&gt;</c> rather than in tags of its
            /// own, and whose inline images are void <c>&lt;img&gt;</c>
            /// elements.
            /// </summary>
            private bool HallFairyGui(
                ReadOnlySpan<char> jism,
                int jismBidaya,
                int tulWasm,
                ref NatijaWasm wasm)
            {
                ReadOnlySpan<char> ism = jism.Slice(0, tulWasm);
                if (Yutabiq(ism, "b"))
                {
                    wasm.Alam = Alamat.UslubWazn;
                    wasm.Wazn = WaznGhaliz;
                    return true;
                }
                if (Yutabiq(ism, "i"))
                {
                    wasm.Alam = Alamat.UslubMaail;
                    return true;
                }
                if (Yutabiq(ism, "br"))
                {
                    wasm.Mufrad = '\n';
                    return true;
                }
                if (Yutabiq(ism, "img"))
                {
                    wasm.Sura = true;
                    QarraSifat(jism, jismBidaya, ref wasm);
                    return true;
                }
                if (Yutabiq(ism, "font"))
                {
                    return SifatFont(jism, true, ref wasm);
                }
                return Yutabiq(ism, "u")
                    || Yutabiq(ism, "s")
                    || Yutabiq(ism, "a")
                    || Yutabiq(ism, "p")
                    || Yutabiq(ism, "div")
                    || Yutabiq(ism, "span")
                    || Yutabiq(ism, "sub")
                    || Yutabiq(ism, "sup");
            }

            /// <summary>
            /// Generic HTML, as an embedded web view or an in-game browser
            /// parses it.
            /// </summary>
            /// <remarks>
            /// <b>HTML's <c>size</c> attribute is never read as pixels.</b>
            /// <c>&lt;font size="3"&gt;</c> is position three on a seven-step
            /// scale, not three pixels — and reading it as pixels would lay a
            /// paragraph out at a size no glyph is legible at, which then
            /// renders perfectly and looks like a font problem. Size in this
            /// dialect comes only from a CSS <c>font-size</c> with a unit,
            /// where the number means what it says.
            /// </remarks>
            private bool HallHtml(
                ReadOnlySpan<char> jism,
                int jismBidaya,
                int tulWasm,
                ref NatijaWasm wasm)
            {
                ReadOnlySpan<char> ism = jism.Slice(0, tulWasm);
                if (Yutabiq(ism, "br") || Yutabiq(ism, "hr"))
                {
                    wasm.Mufrad = '\n';
                    return true;
                }
                if (Yutabiq(ism, "img"))
                {
                    wasm.Sura = true;
                    QarraSifat(jism, jismBidaya, ref wasm);
                    return true;
                }
                if (Yutabiq(ism, "b") || Yutabiq(ism, "strong"))
                {
                    wasm.Alam = Alamat.UslubWazn;
                    wasm.Wazn = WaznGhaliz;
                }
                else if (Yutabiq(ism, "i") || Yutabiq(ism, "em") || Yutabiq(ism, "cite"))
                {
                    wasm.Alam = Alamat.UslubMaail;
                }
                else if (!Yutabiq(ism, "font")
                    && !Yutabiq(ism, "span")
                    && !Yutabiq(ism, "div")
                    && !Yutabiq(ism, "p")
                    && !Yutabiq(ism, "a")
                    && !Yutabiq(ism, "u")
                    && !Yutabiq(ism, "s")
                    && !Yutabiq(ism, "ins")
                    && !Yutabiq(ism, "del")
                    && !Yutabiq(ism, "sub")
                    && !Yutabiq(ism, "sup")
                    && !Yutabiq(ism, "big")
                    && !Yutabiq(ism, "code")
                    && !Yutabiq(ism, "pre")
                    && !Yutabiq(ism, "mark")
                    && !Yutabiq(ism, "small")
                    && !Yutabiq(ism, "strike"))
                {
                    return false;
                }
                return SifatFont(jism, false, ref wasm);
            }

            /// <summary>
            /// NGUI's bracket markup. <c>[RRGGBB]</c> and <c>[RRGGBBAA]</c>
            /// push a colour that <c>[-]</c> pops; the rest are named toggles.
            /// </summary>
            /// <remarks>
            /// The colour form carries no <c>#</c> and no tag name, which is
            /// exactly why <see cref="NawNasq.NGui"/> must never be enabled for
            /// a string that is not NGUI's: in every other dialect
            /// <c>[FF0000]</c> is six characters a player is meant to read, and
            /// parsing them as a colour would delete them from the screen.
            /// </remarks>
            private static bool HallNGui(ReadOnlySpan<char> muhtawa, ref NatijaWasm wasm)
            {
                if ((muhtawa.Length == 6 || muhtawa.Length == 8)
                    && SittasiKamil(muhtawa)
                    && LawnMin(muhtawa, out uint lawn))
                {
                    wasm.Alam = Alamat.UslubLawn;
                    wasm.Lawn = lawn;
                    return true;
                }

                int tulWasm = 0;
                while (tulWasm < muhtawa.Length && muhtawa[tulWasm] != '=')
                {
                    tulWasm++;
                }
                if (tulWasm == 0)
                {
                    return false;
                }

                ReadOnlySpan<char> ism = muhtawa.Slice(0, tulWasm);
                if (Yutabiq(ism, "b"))
                {
                    wasm.Alam = Alamat.UslubWazn;
                    wasm.Wazn = WaznGhaliz;
                    return true;
                }
                if (Yutabiq(ism, "i"))
                {
                    wasm.Alam = Alamat.UslubMaail;
                    return true;
                }
                return Yutabiq(ism, "u")
                    || Yutabiq(ism, "s")
                    || Yutabiq(ism, "c")
                    || Yutabiq(ism, "url");
            }

            // ---- attributes -------------------------------------------------

            /// <summary>
            /// The value attached to the tag's own name:
            /// <c>&lt;color=#ff0000&gt;</c> yields <c>#ff0000</c>.
            /// </summary>
            private static bool QeematWasm(ReadOnlySpan<char> jism, out ReadOnlySpan<char> qeema)
            {
                int i = 0;
                if (TaliSifa(jism, ref i, out _, out _, out int qb, out int qt) && qt > 0)
                {
                    qeema = jism.Slice(qb, qt);
                    return true;
                }
                qeema = ReadOnlySpan<char>.Empty;
                return false;
            }

            /// <summary>
            /// Reads a sprite's identity from wherever the dialect put it:
            /// attached to the tag name (<c>&lt;sprite=3&gt;</c>), in an
            /// <c>index</c>, or in a <c>name</c> or <c>src</c>.
            /// </summary>
            /// <remarks>
            /// A value attached to the name is an index when it parses as one
            /// and a sheet name when it does not, which is TextMeshPro's own
            /// reading of <c>&lt;sprite="Icons" index=2&gt;</c>. Neither is
            /// resolved here — the sprite lives in the game's asset table and
            /// only the adapter can reach it — so both are carried through
            /// untouched on the atom.
            /// </remarks>
            private static void QarraSifat(
                ReadOnlySpan<char> jism,
                int jismBidaya,
                ref NatijaWasm wasm)
            {
                int i = 0;
                bool awwal = true;
                while (TaliSifa(jism, ref i, out int mb, out int mt, out int qb, out int qt))
                {
                    ReadOnlySpan<char> miftah = jism.Slice(mb, mt);
                    ReadOnlySpan<char> qeema =
                        qt > 0 ? jism.Slice(qb, qt) : ReadOnlySpan<char>.Empty;
                    if (awwal)
                    {
                        awwal = false;
                        if (qt <= 0)
                        {
                            continue;
                        }
                        if (SaheehKamil(qeema, out int fahrasAwwal))
                        {
                            wasm.Fahras = fahrasAwwal;
                        }
                        else
                        {
                            wasm.IsmBidaya = jismBidaya + qb;
                            wasm.IsmTul = qt;
                        }
                        continue;
                    }
                    if (Yutabiq(miftah, "index"))
                    {
                        if (SaheehKamil(qeema, out int fahras))
                        {
                            wasm.Fahras = fahras;
                        }
                    }
                    else if (Yutabiq(miftah, "name") || Yutabiq(miftah, "src"))
                    {
                        wasm.IsmBidaya = jismBidaya + qb;
                        wasm.IsmTul = qt;
                    }
                }
            }

            /// <summary>
            /// Reads styling out of a tag's attributes: <c>color</c>, a
            /// <c>size</c> when the dialect measures it in pixels, and a CSS
            /// <c>style</c> declaration.
            /// </summary>
            private bool SifatFont(
                ReadOnlySpan<char> jism,
                bool hajmBilBiksel,
                ref NatijaWasm wasm)
            {
                int i = 0;
                bool awwal = true;
                while (TaliSifa(jism, ref i, out int mb, out int mt, out int qb, out int qt))
                {
                    if (awwal)
                    {
                        awwal = false;
                        continue;
                    }
                    ReadOnlySpan<char> miftah = jism.Slice(mb, mt);
                    ReadOnlySpan<char> qeema =
                        qt > 0 ? jism.Slice(qb, qt) : ReadOnlySpan<char>.Empty;

                    if (Yutabiq(miftah, "color"))
                    {
                        if (!LawnMin(qeema, out uint lawn))
                        {
                            wasm.Marfud = true;
                            return true;
                        }
                        wasm.Alam |= Alamat.UslubLawn;
                        wasm.Lawn = lawn;
                    }
                    else if (hajmBilBiksel && Yutabiq(miftah, "size"))
                    {
                        if (HajmMin(qeema, out float hajm) && hajm > 0f)
                        {
                            wasm.Alam |= Alamat.UslubHajm;
                            wasm.Hajm = hajm;
                        }
                    }
                    else if (Yutabiq(miftah, "style"))
                    {
                        QarraCss(qeema, ref wasm);
                    }
                }
                return true;
            }

            /// <summary>
            /// Reads the four CSS declarations that change layout or colour out
            /// of a <c>style</c> attribute, and ignores the rest.
            /// </summary>
            /// <remarks>
            /// A <c>font-size</c> in <c>em</c> is turned into a percentage and
            /// resolved against <see cref="KhiyaratNasq.HajmAsas"/> by the same
            /// path a percentage takes; a size with no unit at all is refused,
            /// because CSS has no such thing and a bare number there is a
            /// stylesheet bug whose most likely reading — pixels — is also the
            /// most damaging one to get wrong.
            /// </remarks>
            private void QarraCss(ReadOnlySpan<char> css, ref NatijaWasm wasm)
            {
                int i = 0;
                int n = css.Length;
                while (i < n)
                {
                    int bidaya = i;
                    while (i < n && css[i] != ';')
                    {
                        i++;
                    }
                    ReadOnlySpan<char> jumla = css.Slice(bidaya, i - bidaya);
                    if (i < n)
                    {
                        i++;
                    }

                    int fasl = jumla.IndexOf(':');
                    if (fasl <= 0 || fasl + 1 >= jumla.Length)
                    {
                        continue;
                    }
                    ReadOnlySpan<char> miftah = Qasas(jumla.Slice(0, fasl));
                    ReadOnlySpan<char> qeema = Qasas(jumla.Slice(fasl + 1));
                    if (miftah.Length == 0 || qeema.Length == 0)
                    {
                        continue;
                    }

                    if (Yutabiq(miftah, "color"))
                    {
                        if (LawnMin(qeema, out uint lawn))
                        {
                            wasm.Alam |= Alamat.UslubLawn;
                            wasm.Lawn = lawn;
                        }
                    }
                    else if (Yutabiq(miftah, "font-weight"))
                    {
                        bool ghaliz = Yutabiq(qeema, "bold")
                            || Yutabiq(qeema, "bolder")
                            || (SaheehKamil(qeema, out int wazn) && wazn >= 600);
                        if (ghaliz)
                        {
                            wasm.Alam |= Alamat.UslubWazn;
                            wasm.Wazn = WaznGhaliz;
                        }
                    }
                    else if (Yutabiq(miftah, "font-style"))
                    {
                        if (Yutabiq(qeema, "italic") || Yutabiq(qeema, "oblique"))
                        {
                            wasm.Alam |= Alamat.UslubMaail;
                        }
                    }
                    else if (Yutabiq(miftah, "font-size"))
                    {
                        if (WahdaCss(qeema) && HajmMin(qeema, out float hajm) && hajm > 0f)
                        {
                            wasm.Alam |= Alamat.UslubHajm;
                            wasm.Hajm = hajm;
                        }
                    }
                }
            }

            /// <summary>
            /// Steps to the next <c>key</c> or <c>key=value</c> pair in a tag
            /// body, with the value optionally quoted with either quote
            /// character. Offsets are relative to the body.
            /// </summary>
            /// <remarks>
            /// The tag's own name arrives as the first pair, with whatever was
            /// attached to it by <c>=</c> as its value. That is what lets
            /// <c>&lt;color=red&gt;</c> and <c>&lt;font color="red"&gt;</c> be
            /// read by the same code rather than by two spellings of it.
            /// </remarks>
            private static bool TaliSifa(
                ReadOnlySpan<char> jism,
                ref int i,
                out int miftahBidaya,
                out int miftahTul,
                out int qeemaBidaya,
                out int qeemaTul)
            {
                miftahBidaya = 0;
                miftahTul = 0;
                qeemaBidaya = 0;
                qeemaTul = 0;

                int n = jism.Length;
                while (i < n && Faragh(jism[i]))
                {
                    i++;
                }
                if (i >= n)
                {
                    return false;
                }

                miftahBidaya = i;
                while (i < n && !Faragh(jism[i]) && jism[i] != '=')
                {
                    i++;
                }
                miftahTul = i - miftahBidaya;
                if (miftahTul == 0)
                {
                    i++;
                    return false;
                }

                int rajaa = i;
                while (i < n && Faragh(jism[i]))
                {
                    i++;
                }
                if (i >= n || jism[i] != '=')
                {
                    i = rajaa;
                    return true;
                }

                i++;
                while (i < n && Faragh(jism[i]))
                {
                    i++;
                }
                if (i >= n)
                {
                    return true;
                }

                char iqtibas = jism[i];
                if (iqtibas == '"' || iqtibas == '\'')
                {
                    i++;
                    qeemaBidaya = i;
                    while (i < n && jism[i] != iqtibas)
                    {
                        i++;
                    }
                    qeemaTul = i - qeemaBidaya;
                    if (i < n)
                    {
                        i++;
                    }
                    return true;
                }

                qeemaBidaya = i;
                while (i < n && !Faragh(jism[i]))
                {
                    i++;
                }
                qeemaTul = i - qeemaBidaya;
                return true;
            }

            /// <summary>
            /// Resolves a size value against
            /// <see cref="KhiyaratNasq.HajmAsas"/>. Returns <c>false</c> only
            /// when the value is not a number at all; a number that cannot be
            /// resolved yields zero, which the callers read as "set no size".
            /// </summary>
            private bool HajmMin(ReadOnlySpan<char> qeema, out float hajm)
            {
                hajm = 0f;
                if (!AdadMin(qeema, out float adad, out bool nisbi, out bool miawi))
                {
                    return false;
                }
                float asas = _khiyarat.HajmAsas;
                if (miawi)
                {
                    hajm = asas > 0f ? asas * adad * 0.01f : 0f;
                }
                else if (nisbi)
                {
                    hajm = asas > 0f ? asas + adad : 0f;
                }
                else
                {
                    hajm = adad;
                }
                return true;
            }
        }

        // ---- character and value parsing ------------------------------------

        /// <summary>ASCII whitespace.</summary>
        private static bool Faragh(char h) =>
            h == ' ' || h == '\t' || h == '\r' || h == '\n' || h == '\f' || h == '\v';

        /// <summary>What ends a tag name.</summary>
        private static bool HadFasl(char h) => Faragh(h) || h == '=' || h == '/';

        /// <summary>An ASCII digit.</summary>
        private static bool Raqm(char h) => h >= '0' && h <= '9';

        /// <summary>An ASCII letter.</summary>
        private static bool Abjadi(char h) =>
            (h >= 'a' && h <= 'z') || (h >= 'A' && h <= 'Z');

        /// <summary>A printf flag character.</summary>
        private static bool Alama(char h) =>
            h == '-' || h == '+' || h == ' ' || h == '#' || h == '0' || h == '\'';

        /// <summary>A printf length modifier.</summary>
        private static bool Tuli(char h) =>
            h == 'l' || h == 'h' || h == 'L' || h == 'z' || h == 'j' || h == 't';

        /// <summary>
        /// A printf conversion this file recognises. See
        /// <c>Massah.TabaSalih</c> for the three that are deliberately absent
        /// and the English words they would otherwise eat.
        /// </summary>
        private static bool TahweelSalih(char h)
        {
            switch (h)
            {
                case 'd':
                case 'i':
                case 'u':
                case 'f':
                case 'F':
                case 'e':
                case 'E':
                case 'g':
                case 'G':
                case 's':
                case 'S':
                case 'x':
                case 'X':
                case 'c':
                case 'C':
                case 'p':
                    return true;
                default:
                    return false;
            }
        }

        /// <summary>
        /// Case-insensitive ASCII comparison. Deliberately ASCII-only: a tag
        /// name is ASCII in every dialect here, and a culture-aware comparison
        /// would make the Turkish dotless i decide whether
        /// <c>&lt;I&gt;</c> is italic, on the machines whose locale is set that
        /// way and nowhere else.
        /// </summary>
        private static bool Yutabiq(ReadOnlySpan<char> awwal, ReadOnlySpan<char> thani)
        {
            if (awwal.Length != thani.Length)
            {
                return false;
            }
            for (int i = 0; i < awwal.Length; i++)
            {
                char a = awwal[i];
                char b = thani[i];
                if (a >= 'A' && a <= 'Z')
                {
                    a = (char)(a + 32);
                }
                if (b >= 'A' && b <= 'Z')
                {
                    b = (char)(b + 32);
                }
                if (a != b)
                {
                    return false;
                }
            }
            return true;
        }

        /// <summary>Trims ASCII whitespace from both ends of a window.</summary>
        private static ReadOnlySpan<char> Qasas(ReadOnlySpan<char> nass)
        {
            int bidaya = 0;
            int nihaya = nass.Length;
            while (bidaya < nihaya && Faragh(nass[bidaya]))
            {
                bidaya++;
            }
            while (nihaya > bidaya && Faragh(nass[nihaya - 1]))
            {
                nihaya--;
            }
            return nass.Slice(bidaya, nihaya - bidaya);
        }

        /// <summary>
        /// Reads a run of digits, saturating rather than overflowing. Used for
        /// a placeholder's written index, where a hostile value must not wrap
        /// into a small one that collides with a real argument position.
        /// </summary>
        private static int SaheehMin(ReadOnlySpan<char> nass)
        {
            long qeema = 0;
            for (int i = 0; i < nass.Length; i++)
            {
                if (!Raqm(nass[i]))
                {
                    break;
                }
                qeema = (qeema * 10) + (nass[i] - '0');
                if (qeema > int.MaxValue)
                {
                    return int.MaxValue;
                }
            }
            return (int)qeema;
        }

        /// <summary>Whether the whole window is a non-negative integer.</summary>
        private static bool SaheehKamil(ReadOnlySpan<char> nass, out int qeema)
        {
            qeema = 0;
            if (nass.Length == 0)
            {
                return false;
            }
            for (int i = 0; i < nass.Length; i++)
            {
                if (!Raqm(nass[i]))
                {
                    return false;
                }
            }
            qeema = SaheehMin(nass);
            return true;
        }

        /// <summary>Whether every character is a hexadecimal digit.</summary>
        private static bool SittasiKamil(ReadOnlySpan<char> nass)
        {
            if (nass.Length == 0)
            {
                return false;
            }
            for (int i = 0; i < nass.Length; i++)
            {
                if (!Sittasi(nass[i], out _))
                {
                    return false;
                }
            }
            return true;
        }

        /// <summary>One hexadecimal digit's value.</summary>
        private static bool Sittasi(char h, out int qeema)
        {
            if (h >= '0' && h <= '9')
            {
                qeema = h - '0';
                return true;
            }
            if (h >= 'a' && h <= 'f')
            {
                qeema = (h - 'a') + 10;
                return true;
            }
            if (h >= 'A' && h <= 'F')
            {
                qeema = (h - 'A') + 10;
                return true;
            }
            qeema = 0;
            return false;
        }

        /// <summary>
        /// Whether a CSS value carries a unit, which is the difference between
        /// a size and a stylesheet bug.
        /// </summary>
        private static bool WahdaCss(ReadOnlySpan<char> qeema)
        {
            if (qeema.Length == 0)
            {
                return false;
            }
            char akhir = qeema[qeema.Length - 1];
            return akhir == '%' || Abjadi(akhir);
        }

        /// <summary>
        /// Parses a colour: <c>#RGB</c>, <c>#RGBA</c>, <c>#RRGGBB</c>,
        /// <c>#RRGGBBAA</c>, the same four without the <c>#</c> — which is
        /// NGUI's form — or one of the names TextMeshPro knows.
        /// </summary>
        /// <remarks>
        /// The result is packed <c>0xRRGGBBAA</c>, which is
        /// <see cref="TaaribNitaqUslub.Lawn"/>'s order and not the byte order
        /// <c>LawnRasm</c> stores. The two are reconciled in exactly one place,
        /// <c>LawnRasm.Min</c>, for the reason stated there.
        /// </remarks>
        private static bool LawnMin(ReadOnlySpan<char> qeema, out uint lawn)
        {
            lawn = 0;
            if (qeema.Length == 0)
            {
                return false;
            }

            bool marqum = qeema[0] == '#';
            ReadOnlySpan<char> raqm = marqum ? qeema.Slice(1) : qeema;
            int tul = raqm.Length;
            if ((tul == 3 || tul == 4 || tul == 6 || tul == 8) && SittasiKamil(raqm))
            {
                lawn = SittasiIlaLawn(raqm);
                return true;
            }
            return !marqum && LawnMusamma(qeema, out lawn);
        }

        /// <summary>Expands a hexadecimal colour to <c>0xRRGGBBAA</c>.</summary>
        private static uint SittasiIlaLawn(ReadOnlySpan<char> raqm)
        {
            Span<int> khanat = stackalloc int[8];
            for (int i = 0; i < raqm.Length; i++)
            {
                Sittasi(raqm[i], out khanat[i]);
            }

            int ahmar;
            int akhdar;
            int azraq;
            int shaffafiya = 0xFF;
            if (raqm.Length <= 4)
            {
                ahmar = (khanat[0] * 16) + khanat[0];
                akhdar = (khanat[1] * 16) + khanat[1];
                azraq = (khanat[2] * 16) + khanat[2];
                if (raqm.Length == 4)
                {
                    shaffafiya = (khanat[3] * 16) + khanat[3];
                }
            }
            else
            {
                ahmar = (khanat[0] * 16) + khanat[1];
                akhdar = (khanat[2] * 16) + khanat[3];
                azraq = (khanat[4] * 16) + khanat[5];
                if (raqm.Length == 8)
                {
                    shaffafiya = (khanat[6] * 16) + khanat[7];
                }
            }
            return ((uint)ahmar << 24)
                | ((uint)akhdar << 16)
                | ((uint)azraq << 8)
                | (uint)shaffafiya;
        }

        /// <summary>
        /// The colour names TextMeshPro accepts, and the legacy UI accepts a
        /// subset of. Kept to that list on purpose: a name this file resolved
        /// and the game did not would colour a run the game draws in its own
        /// default, and the two would disagree on exactly the strings nobody
        /// checks twice.
        /// </summary>
        private static bool LawnMusamma(ReadOnlySpan<char> ism, out uint lawn)
        {
            lawn = 0xFFFFFFFF;
            if (Yutabiq(ism, "white"))
            {
                return true;
            }
            if (Yutabiq(ism, "black"))
            {
                lawn = 0x000000FF;
                return true;
            }
            if (Yutabiq(ism, "red"))
            {
                lawn = 0xFF0000FF;
                return true;
            }
            if (Yutabiq(ism, "green"))
            {
                lawn = 0x008000FF;
                return true;
            }
            if (Yutabiq(ism, "blue"))
            {
                lawn = 0x0000FFFF;
                return true;
            }
            if (Yutabiq(ism, "yellow"))
            {
                lawn = 0xFFFF00FF;
                return true;
            }
            if (Yutabiq(ism, "orange"))
            {
                lawn = 0xFFA500FF;
                return true;
            }
            if (Yutabiq(ism, "purple"))
            {
                lawn = 0x800080FF;
                return true;
            }
            if (Yutabiq(ism, "cyan") || Yutabiq(ism, "aqua"))
            {
                lawn = 0x00FFFFFF;
                return true;
            }
            if (Yutabiq(ism, "magenta") || Yutabiq(ism, "fuchsia"))
            {
                lawn = 0xFF00FFFF;
                return true;
            }
            if (Yutabiq(ism, "grey") || Yutabiq(ism, "gray"))
            {
                lawn = 0x808080FF;
                return true;
            }
            if (Yutabiq(ism, "lightblue"))
            {
                lawn = 0xADD8E6FF;
                return true;
            }
            if (Yutabiq(ism, "darkblue") || Yutabiq(ism, "navy"))
            {
                lawn = 0x000080FF;
                return true;
            }
            if (Yutabiq(ism, "brown"))
            {
                lawn = 0xA52A2AFF;
                return true;
            }
            if (Yutabiq(ism, "lime"))
            {
                lawn = 0x00FF00FF;
                return true;
            }
            if (Yutabiq(ism, "maroon"))
            {
                lawn = 0x800000FF;
                return true;
            }
            if (Yutabiq(ism, "olive"))
            {
                lawn = 0x808000FF;
                return true;
            }
            if (Yutabiq(ism, "pink"))
            {
                lawn = 0xFFC0CBFF;
                return true;
            }
            if (Yutabiq(ism, "silver"))
            {
                lawn = 0xC0C0C0FF;
                return true;
            }
            if (Yutabiq(ism, "teal"))
            {
                lawn = 0x008080FF;
                return true;
            }
            lawn = 0;
            return false;
        }

        /// <summary>
        /// Parses a size value: an optional sign, a number, and an optional
        /// unit.
        /// </summary>
        /// <remarks>
        /// <paramref name="nisbi"/> is set by an explicit leading sign, which
        /// is what makes <c>+2</c> an offset from the base size rather than a
        /// size of two. <paramref name="miawi"/> covers both <c>%</c> and the
        /// em-family units, the latter multiplied by a hundred first so that
        /// one resolution path serves both — <c>1.2em</c> and <c>120%</c> mean
        /// the same thing and should not be able to drift apart. A unit that is
        /// neither recognised nor absent is refused, because guessing at
        /// <c>20vw</c> is guessing at the width of a viewport this process does
        /// not have.
        /// </remarks>
        private static bool AdadMin(
            ReadOnlySpan<char> qeema,
            out float adad,
            out bool nisbi,
            out bool miawi)
        {
            adad = 0f;
            nisbi = false;
            miawi = false;

            int n = qeema.Length;
            int i = 0;
            bool salib = false;
            if (i < n && (qeema[i] == '+' || qeema[i] == '-'))
            {
                nisbi = true;
                salib = qeema[i] == '-';
                i++;
            }

            double majmu = 0.0;
            int khanat = 0;
            while (i < n && Raqm(qeema[i]))
            {
                majmu = (majmu * 10.0) + (qeema[i] - '0');
                if (majmu > 1000000.0)
                {
                    majmu = 1000000.0;
                }
                i++;
                khanat++;
            }
            if (i < n && qeema[i] == '.')
            {
                i++;
                double wazn = 0.1;
                while (i < n && Raqm(qeema[i]))
                {
                    majmu += (qeema[i] - '0') * wazn;
                    wazn *= 0.1;
                    i++;
                    khanat++;
                }
            }
            if (khanat == 0)
            {
                return false;
            }

            if (i < n)
            {
                ReadOnlySpan<char> wahda = Qasas(qeema.Slice(i));
                if (wahda.Length == 1 && wahda[0] == '%')
                {
                    miawi = true;
                }
                else if (Yutabiq(wahda, "em") || Yutabiq(wahda, "rem") || Yutabiq(wahda, "ex"))
                {
                    miawi = true;
                    majmu *= 100.0;
                }
                else if (!Yutabiq(wahda, "px") && !Yutabiq(wahda, "pt") && wahda.Length != 0)
                {
                    return false;
                }
            }

            adad = (float)(salib ? -majmu : majmu);
            return true;
        }

        /// <summary>A numeric HTML entity, decimal or hexadecimal.</summary>
        private static bool QeematKayan(ReadOnlySpan<char> raqm, out uint qeema)
        {
            qeema = 0;
            if (raqm.Length == 0)
            {
                return false;
            }

            bool sittasi = raqm[0] == 'x' || raqm[0] == 'X';
            ReadOnlySpan<char> arqam = sittasi ? raqm.Slice(1) : raqm;
            if (arqam.Length == 0 || arqam.Length > 8)
            {
                return false;
            }

            long majmu = 0;
            for (int i = 0; i < arqam.Length; i++)
            {
                int khana;
                if (sittasi)
                {
                    if (!Sittasi(arqam[i], out khana))
                    {
                        return false;
                    }
                    majmu = (majmu * 16) + khana;
                }
                else
                {
                    if (!Raqm(arqam[i]))
                    {
                        return false;
                    }
                    majmu = (majmu * 10) + (arqam[i] - '0');
                }
                if (majmu > 0x10FFFF)
                {
                    return false;
                }
            }

            if (majmu == 0 || (majmu >= 0xD800 && majmu <= 0xDFFF))
            {
                return false;
            }
            qeema = (uint)majmu;
            return true;
        }

        /// <summary>
        /// The named HTML entities a game's text actually carries. An
        /// unrecognised name is left alone rather than dropped, so an
        /// <c>&amp;</c> that was never an entity stays the character it is.
        /// </summary>
        private static bool KayanMusamma(ReadOnlySpan<char> ism, out uint qeema)
        {
            qeema = 0;
            if (Yutabiq(ism, "amp"))
            {
                qeema = '&';
            }
            else if (Yutabiq(ism, "lt"))
            {
                qeema = '<';
            }
            else if (Yutabiq(ism, "gt"))
            {
                qeema = '>';
            }
            else if (Yutabiq(ism, "quot"))
            {
                qeema = '"';
            }
            else if (Yutabiq(ism, "apos"))
            {
                qeema = '\'';
            }
            else if (Yutabiq(ism, "nbsp"))
            {
                qeema = 0x00A0;
            }
            else if (Yutabiq(ism, "copy"))
            {
                qeema = 0x00A9;
            }
            else if (Yutabiq(ism, "reg"))
            {
                qeema = 0x00AE;
            }
            else if (Yutabiq(ism, "deg"))
            {
                qeema = 0x00B0;
            }
            else if (Yutabiq(ism, "middot"))
            {
                qeema = 0x00B7;
            }
            else if (Yutabiq(ism, "times"))
            {
                qeema = 0x00D7;
            }
            else if (Yutabiq(ism, "ndash"))
            {
                qeema = 0x2013;
            }
            else if (Yutabiq(ism, "mdash"))
            {
                qeema = 0x2014;
            }
            else if (Yutabiq(ism, "laquo"))
            {
                qeema = 0x00AB;
            }
            else if (Yutabiq(ism, "raquo"))
            {
                qeema = 0x00BB;
            }
            else if (Yutabiq(ism, "hellip"))
            {
                qeema = 0x2026;
            }
            else
            {
                return false;
            }
            return true;
        }
    }
}

