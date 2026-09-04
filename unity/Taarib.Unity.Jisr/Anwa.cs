// الأنواع — every type that crosses the boundary, mirrored field for field
// against crates/taarib-jisr/src/anwa.rs and the generated include/taarib.h.
//
// These layouts are frozen on the Rust side. A field may be appended only
// behind a major version bump, a field is never reordered or resized, and
// nothing here is a type whose representation a compiler is free to choose.
// Every struct is LayoutKind.Sequential with default packing, which follows
// the same alignment algorithm a C compiler applies to the repr(C) original —
// so the two sides agree about every offset, including the one place the
// original carries alignment padding (before TaaribTalab.Khiyarat on 64-bit).
//
// Discriminants cross as unsigned 32-bit numbers, never as C enums, because a
// C enum's width is implementation-defined. The C# enums below are explicitly
// ": uint" so a field typed with one is bit-identical to the u32 it mirrors,
// and the native side treats any unrecognised number as the documented
// default rather than as undefined behaviour.

using System;
using System.Runtime.InteropServices;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// The flag constants for every <c>alam</c> bitfield that crosses the
    /// boundary. Mirrors the <c>TAARIB_*</c> constants in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>, value for value.
    /// </summary>
    public static class Alamat
    {
        /// <summary>
        /// <see cref="TaaribHarf.Alam"/>: this glyph is a combining mark,
        /// positioned onto a base rather than advancing the pen. Marks carry a
        /// zero advance and never contribute to width; drawing code that
        /// treated one as a letter would double-count the line.
        /// </summary>
        public const byte HarfAlama = 1 << 0;

        /// <summary>
        /// <see cref="TaaribSatr.Alam"/>: this is the last line of its
        /// paragraph, which is what stops justification stretching it across
        /// the full width.
        /// </summary>
        public const uint SatrAkhir = 1u << 0;

        /// <summary>
        /// <see cref="TaaribSatr.Alam"/>: this line's base direction is right
        /// to left.
        /// </summary>
        public const uint SatrYameen = 1u << 1;

        /// <summary>
        /// <see cref="TaaribKhiyarat.Alam"/>: this text is dialogue, which is
        /// what the keep-diacritics-in-dialogue policy keys on.
        /// </summary>
        public const uint KhiyarHiwar = 1u << 0;

        /// <summary>
        /// <see cref="TaaribKhiyarat.Alam"/>: the caller forbids wrapping
        /// entirely, as a single-line input field does.
        /// </summary>
        public const uint KhiyarSatrWahid = 1u << 1;

        /// <summary>
        /// <see cref="TaaribMakhzanTakhtit.Alam"/>: the layout's overall
        /// direction is right to left.
        /// </summary>
        public const uint TakhtitYameen = 1u << 0;

        /// <summary>
        /// <see cref="TaaribMakhzanTakhtit.Alam"/>: the overflow policy
        /// truncated the text.
        /// </summary>
        public const uint TakhtitMaqsus = 1u << 1;

        /// <summary>
        /// <see cref="TaaribMakhzanTakhtit.Alam"/>: the text did not fit, and
        /// <see cref="TaaribMakhzanTakhtit.Tajawuz"/> describes by how much.
        /// </summary>
        public const uint TakhtitTajawuz = 1u << 2;

        /// <summary>
        /// <see cref="TaaribMakhzanTakhtit.Alam"/>: this layout came from the
        /// cache, so nothing was shaped. Exposed because an adapter's own
        /// diagnostics want the hit rate, and because a cache that never hits
        /// is a cache whose key is wrong.
        /// </summary>
        public const uint TakhtitMakhzan = 1u << 3;

        /// <summary>
        /// <see cref="TaaribNitaqUslub.Alam"/>: this span is italic or
        /// slanted.
        /// </summary>
        public const uint UslubMaail = 1u << 0;

        /// <summary>
        /// <see cref="TaaribNitaqUslub.Alam"/>: this span is an opaque atom —
        /// a format placeholder or an inline sprite — and is never shaped.
        /// </summary>
        public const uint UslubDharra = 1u << 1;

        /// <summary>
        /// <see cref="TaaribNitaqUslub.Alam"/>: this span sets a font index.
        /// </summary>
        public const uint UslubKhatt = 1u << 2;

        /// <summary>
        /// <see cref="TaaribNitaqUslub.Alam"/>: this span sets a weight.
        /// </summary>
        public const uint UslubWazn = 1u << 3;

        /// <summary>
        /// <see cref="TaaribNitaqUslub.Alam"/>: this span sets a size.
        /// </summary>
        public const uint UslubHajm = 1u << 4;

        /// <summary>
        /// <see cref="TaaribNitaqUslub.Alam"/>: this span sets a colour.
        /// </summary>
        public const uint UslubLawn = 1u << 5;
    }

    /// <summary>
    /// How the base direction of a paragraph is decided. Mirrors the
    /// discriminant read by <c>ittijah_min_raqm</c> in <c>anwa.rs</c>; the
    /// native side treats any other number as <see cref="Tilqai"/>.
    /// </summary>
    public enum IttijahAsas : uint
    {
        /// <summary>From the first strong character, per the algorithm.</summary>
        Tilqai = 0,

        /// <summary>Right to left, forced.</summary>
        Yameen = 1,

        /// <summary>Left to right, forced.</summary>
        Yasar = 2,
    }

    /// <summary>
    /// The declared language of the text, which drives <c>locl</c> so Persian
    /// and Urdu get their correct local forms of kaf, yeh and heh. Mirrors
    /// <c>lugha_min_raqm</c> in <c>anwa.rs</c>; unrecognised numbers mean
    /// <see cref="Tilqai"/>.
    /// </summary>
    public enum LughaNass : uint
    {
        /// <summary>Detect from the text itself.</summary>
        Tilqai = 0,

        /// <summary>Arabic.</summary>
        Arabi = 1,

        /// <summary>Persian.</summary>
        Farisi = 2,

        /// <summary>Urdu.</summary>
        Urdu = 3,

        /// <summary>Latin.</summary>
        Latini = 4,
    }

    /// <summary>
    /// How surplus width is absorbed when a line is justified. Mirrors
    /// <c>dabt_min_raqm</c> in <c>anwa.rs</c>; unrecognised numbers mean
    /// <see cref="Bila"/>.
    /// </summary>
    public enum NamatDabt : uint
    {
        /// <summary>No justification.</summary>
        Bila = 0,

        /// <summary>Stretch spaces only — the only mode used for Latin runs.</summary>
        Masafat = 1,

        /// <summary>Elongate letters at ranked kashida opportunities only.</summary>
        Kashida = 2,

        /// <summary>Kashida first, then spaces — the default for Arabic.</summary>
        KashidaThummaMasafat = 3,
    }

    /// <summary>
    /// Where a line sits inside the available width. Mirrors
    /// <c>muhadhaha_min_raqm</c> in <c>anwa.rs</c>; unrecognised numbers mean
    /// <see cref="Bidaya"/>.
    /// </summary>
    public enum Muhadhaha : uint
    {
        /// <summary>The leading edge — the right edge for right-to-left text.</summary>
        Bidaya = 0,

        /// <summary>The trailing edge.</summary>
        Nihaya = 1,

        /// <summary>Centred.</summary>
        Wasat = 2,

        /// <summary>Filled to both edges by justification.</summary>
        Dabt = 3,
    }

    /// <summary>
    /// What happens to diacritics carried by the source text. Mirrors
    /// <c>tashkeel_min_raqm</c> in <c>anwa.rs</c>; unrecognised numbers mean
    /// <see cref="Ibqa"/>.
    /// </summary>
    public enum SiyasatTashkeel : uint
    {
        /// <summary>Keep every mark.</summary>
        Ibqa = 0,

        /// <summary>Strip marks — interface text where they cost width.</summary>
        Hadhf = 1,

        /// <summary>
        /// Keep marks only when the request carries
        /// <see cref="Alamat.KhiyarHiwar"/>.
        /// </summary>
        IbqaFilHiwar = 2,
    }

    /// <summary>
    /// How digits are presented — a per-locale decision translators control
    /// and record in the patch. Mirrors <c>arqam_min_raqm</c> in
    /// <c>anwa.rs</c>; unrecognised numbers mean <see cref="KamaHiya"/>.
    /// </summary>
    public enum SiyasatArqam : uint
    {
        /// <summary>Leave digits exactly as the text carries them.</summary>
        KamaHiya = 0,

        /// <summary>Map to European digits.</summary>
        Latini = 1,

        /// <summary>Map to Arabic-Indic digits.</summary>
        Arabi = 2,

        /// <summary>Map to Eastern Arabic-Indic digits, for Persian and Urdu.</summary>
        Farisi = 3,
    }

    /// <summary>
    /// What happens when text cannot fit the available bounds. Mirrors
    /// <c>tajawuz_min_raqm</c> in <c>anwa.rs</c>; unrecognised numbers mean
    /// <see cref="Ballagh"/>, on purpose — a caller that sent a wrong number
    /// gets a layout plus an honest measurement of the overflow rather than
    /// silently shrunk or truncated text.
    /// </summary>
    public enum SiyasatTajawuz : uint
    {
        /// <summary>Lay out anyway and report the overflow.</summary>
        Ballagh = 0,

        /// <summary>
        /// Shrink the size to fit, no smaller than
        /// <see cref="TaaribKhiyarat.HajmAdna"/>.
        /// </summary>
        Taqlis = 1,

        /// <summary>Truncate with the correct Arabic ellipsis behaviour.</summary>
        Ikhtisar = 2,
    }

    /// <summary>
    /// How an atlas rasterizes its glyphs. Mirrors the <c>namat</c>
    /// discriminant of <c>taarib_lawha_insha</c> and
    /// <see cref="TaaribSafha.Namat"/>: 0 coverage, 1 signed distance field.
    /// The choice is per patch, recorded in the patch, and the adapter obeys
    /// it without asking.
    /// </summary>
    public enum NamatLawha : uint
    {
        /// <summary>
        /// Antialiased coverage — sharper at fixed small sizes, the default
        /// for interface text.
        /// </summary>
        Taghtiya = 0,

        /// <summary>
        /// Signed distance field — one atlas serves every size, which is what
        /// makes auto-sizing and world-space text work.
        /// </summary>
        Misafa = 1,
    }

    /// <summary>
    /// One positioned glyph, ready to become two triangles. Mirrors
    /// <c>TaaribHarf</c> in <c>crates/taarib-jisr/src/anwa.rs</c>: twenty-four
    /// bytes, four-byte aligned, no padding.
    /// </summary>
    /// <remarks>
    /// Note what is absent: a codepoint. Nothing downstream of shaping is
    /// given the character a glyph came from, because a field that carried it
    /// would eventually be drawn from — which is the presentation-form
    /// pipeline this product exists to make impossible.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribHarf
    {
        /// <summary>
        /// The glyph identifier, in the font named by <see cref="Khatt"/>.
        /// Mirrors <c>muarrif: u32</c>.
        /// </summary>
        public uint Muarrif;

        /// <summary>
        /// Byte offset into the caller's UTF-8 text of the cluster this glyph
        /// belongs to. Mirrors <c>anqud: u32</c>.
        /// </summary>
        public uint Anqud;

        /// <summary>
        /// Horizontal position of the glyph's origin, in pixels from the
        /// layout's left edge. Already in visual order. Mirrors <c>s: f32</c>.
        /// </summary>
        public float S;

        /// <summary>
        /// Vertical position of the origin, in pixels down from the layout's
        /// top. Mirrors <c>a: f32</c>.
        /// </summary>
        public float A;

        /// <summary>
        /// The advance this glyph contributed. Zero for marks. Mirrors
        /// <c>taqaddum: f32</c>.
        /// </summary>
        public float Taqaddum;

        /// <summary>
        /// The style span this glyph inherited, so colour survives
        /// reordering. Mirrors <c>nitaq: u16</c>.
        /// </summary>
        public ushort Nitaq;

        /// <summary>
        /// Index into the font chain of the font this glyph belongs to.
        /// Mirrors <c>khatt: u8</c>.
        /// </summary>
        public byte Khatt;

        /// <summary>
        /// <see cref="Alamat.HarfAlama"/>. Mirrors <c>alam: u8</c>.
        /// </summary>
        public byte Alam;
    }

    /// <summary>
    /// One laid-out line. Mirrors <c>TaaribSatr</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>: forty-eight bytes, no padding.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribSatr
    {
        /// <summary>
        /// Index of this line's first glyph in the glyph buffer. Mirrors
        /// <c>awwal_harf: u32</c>.
        /// </summary>
        public uint AwwalHarf;

        /// <summary>How many glyphs this line has. Mirrors <c>adad_huruf: u32</c>.</summary>
        public uint AdadHuruf;

        /// <summary>
        /// First byte of the logical text this line covers. Mirrors
        /// <c>bidayat_mantiqi: u32</c>.
        /// </summary>
        public uint BidayatMantiqi;

        /// <summary>One past the last byte. Mirrors <c>nihayat_mantiqi: u32</c>.</summary>
        public uint NihayatMantiqi;

        /// <summary>
        /// The baseline's vertical position, in pixels from the layout's top.
        /// Mirrors <c>asas: f32</c>.
        /// </summary>
        public float Asas;

        /// <summary>
        /// Where the line box begins horizontally, after alignment. Mirrors
        /// <c>bidaya: f32</c>.
        /// </summary>
        public float Bidaya;

        /// <summary>The measured width of the line's content. Mirrors <c>ard: f32</c>.</summary>
        public float Ard;

        /// <summary>The line box's height. Mirrors <c>irtifa: f32</c>.</summary>
        public float Irtifa;

        /// <summary>
        /// How far the tallest content rises above the baseline. Mirrors
        /// <c>suud: f32</c>.
        /// </summary>
        public float Suud;

        /// <summary>
        /// How far the deepest content falls below it. Mirrors <c>hubut: f32</c>.
        /// </summary>
        public float Hubut;

        /// <summary>
        /// How much width justification added to this line. Mirrors
        /// <c>dabt: f32</c>.
        /// </summary>
        public float Dabt;

        /// <summary>
        /// <see cref="Alamat.SatrAkhir"/>, <see cref="Alamat.SatrYameen"/>.
        /// Mirrors <c>alam: u32</c>.
        /// </summary>
        public uint Alam;
    }

    /// <summary>
    /// What overflowed, by how much. Mirrors <c>TaaribTaqreerTajawuz</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    /// <remarks>
    /// Not an error: a measurement the caller asked for. The patch compiler
    /// turns it into the overflow report, the review console sorts by it, and
    /// the workspace shows it beside the string as it is typed.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribTaqreerTajawuz
    {
        /// <summary>The widest line's measured width, in pixels. Mirrors <c>ard: f32</c>.</summary>
        public float Ard;

        /// <summary>The width that was available. Mirrors <c>ard_mutah: f32</c>.</summary>
        public float ArdMutah;

        /// <summary>The total height the text needed. Mirrors <c>irtifa: f32</c>.</summary>
        public float Irtifa;

        /// <summary>
        /// The height that was available, or zero when none was given.
        /// Mirrors <c>irtifa_mutah: f32</c>.
        /// </summary>
        public float IrtifaMutah;

        /// <summary>The first line that exceeded the width. Mirrors <c>awwal_satr: u32</c>.</summary>
        public uint AwwalSatr;

        /// <summary>How many lines exceeded it. Mirrors <c>adad_sutur: u32</c>.</summary>
        public uint AdadSutur;
    }

    /// <summary>
    /// The caller-owned buffer a layout is written into. Mirrors
    /// <c>TaaribMakhzanTakhtit</c> in <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    /// <remarks>
    /// The caller owns both arrays and their lifetime; the native library
    /// never allocates them, never frees them, and never keeps a pointer to
    /// them past the call. The <c>Siaat</c> fields are capacity in elements
    /// and are read; the <c>Adad</c> fields are how many were written and are
    /// written. When either array is too small, nothing is written, the
    /// <c>Adad</c> fields receive the required counts, and the call returns
    /// <see cref="Ramz.SiatQasira"/> — the negotiation
    /// <see cref="Takhtit"/> performs so no adapter has to. Managed code does
    /// not populate this struct by hand; <see cref="Takhtit"/> assembles it
    /// over its own pinned arrays.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct TaaribMakhzanTakhtit
    {
        /// <summary>The caller's glyph array. Mirrors <c>huruf: *mut TaaribHarf</c>.</summary>
        public TaaribHarf* Huruf;

        /// <summary>Its capacity, in elements. Mirrors <c>siaat_huruf: usize</c>.</summary>
        public nuint SiaatHuruf;

        /// <summary>
        /// How many glyphs were written, or how many are required. Mirrors
        /// <c>adad_huruf: usize</c>.
        /// </summary>
        public nuint AdadHuruf;

        /// <summary>The caller's line array. Mirrors <c>sutur: *mut TaaribSatr</c>.</summary>
        public TaaribSatr* Sutur;

        /// <summary>Its capacity, in elements. Mirrors <c>siaat_sutur: usize</c>.</summary>
        public nuint SiaatSutur;

        /// <summary>
        /// How many lines were written, or how many are required. Mirrors
        /// <c>adad_sutur: usize</c>.
        /// </summary>
        public nuint AdadSutur;

        /// <summary>The width of the widest line. Mirrors <c>ard: f32</c>.</summary>
        public float Ard;

        /// <summary>The total height of every line box. Mirrors <c>irtifa: f32</c>.</summary>
        public float Irtifa;

        /// <summary>
        /// The size the text was finally laid out at, which differs from the
        /// size requested when the overflow policy shrank it to fit. Mirrors
        /// <c>hajm: f32</c>.
        /// </summary>
        public float Hajm;

        /// <summary>
        /// <see cref="Alamat.TakhtitYameen"/>, <see cref="Alamat.TakhtitMaqsus"/>,
        /// <see cref="Alamat.TakhtitTajawuz"/>, <see cref="Alamat.TakhtitMakhzan"/>.
        /// Mirrors <c>alam: u32</c>.
        /// </summary>
        public uint Alam;

        /// <summary>
        /// Valid only when <see cref="Alamat.TakhtitTajawuz"/> is set. Mirrors
        /// <c>tajawuz: TaaribTaqreerTajawuz</c>.
        /// </summary>
        public TaaribTaqreerTajawuz Tajawuz;
    }

    /// <summary>
    /// Text measured without positioning a single glyph. Mirrors
    /// <c>TaaribQiyasNass</c> in <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribQiyasNass
    {
        /// <summary>The width of the widest line. Mirrors <c>ard: f32</c>.</summary>
        public float Ard;

        /// <summary>The total height. Mirrors <c>irtifa: f32</c>.</summary>
        public float Irtifa;

        /// <summary>The first line's ascent. Mirrors <c>suud: f32</c>.</summary>
        public float Suud;

        /// <summary>The last line's descent. Mirrors <c>hubut: f32</c>.</summary>
        public float Hubut;

        /// <summary>How many lines the text needed. Mirrors <c>adad_sutur: u32</c>.</summary>
        public uint AdadSutur;

        /// <summary>
        /// Padding to a multiple of eight. Always written as zero; ignore it.
        /// Mirrors <c>hashw: u32</c>.
        /// </summary>
        public uint Hashw;
    }

    /// <summary>
    /// A style span over the caller's text. Mirrors <c>TaaribNitaqUslub</c>
    /// in <c>crates/taarib-jisr/src/anwa.rs</c>: fifty-two bytes.
    /// </summary>
    /// <remarks>
    /// Only what changes layout is read. Colour is carried through untouched,
    /// because the engine does not draw — but it must survive reordering, or
    /// a coloured word inside a sentence loses its colour the moment the line
    /// is put into visual order.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct TaaribNitaqUslub
    {
        /// <summary>
        /// Byte offset of the span's first byte in the caller's UTF-8 text.
        /// Mirrors <c>bidaya: u32</c>.
        /// </summary>
        public uint Bidaya;

        /// <summary>Length in bytes. Mirrors <c>tul: u32</c>.</summary>
        public uint Tul;

        /// <summary>
        /// Colour as <c>0xRRGGBBAA</c>, read only when
        /// <see cref="Alamat.UslubLawn"/> is set. Mirrors <c>lawn: u32</c>.
        /// </summary>
        public uint Lawn;

        /// <summary>
        /// <see cref="Alamat.UslubMaail"/> and the rest of the span flags.
        /// Mirrors <c>alam: u32</c>.
        /// </summary>
        public uint Alam;

        /// <summary>
        /// A size override in pixels, read only when
        /// <see cref="Alamat.UslubHajm"/> is set. Mirrors <c>hajm: f32</c>.
        /// </summary>
        public float Hajm;

        /// <summary>
        /// Extra letter spacing for this span, in pixels. Mirrors
        /// <c>tabaud: f32</c>.
        /// </summary>
        public float Tabaud;

        /// <summary>
        /// A vertical offset from the baseline, in pixels. Mirrors
        /// <c>izaha: f32</c>.
        /// </summary>
        public float Izaha;

        /// <summary>
        /// An atom's width in pixels, read only when
        /// <see cref="Alamat.UslubDharra"/> is set. Mirrors <c>ard_dharra: f32</c>.
        /// </summary>
        public float ArdDharra;

        /// <summary>An atom's height in pixels. Mirrors <c>irtifa_dharra: f32</c>.</summary>
        public float IrtifaDharra;

        /// <summary>
        /// How far above the baseline an atom's bottom edge sits. Mirrors
        /// <c>asas_dharra: f32</c>.
        /// </summary>
        public float AsasDharra;

        /// <summary>
        /// The caller's own reference to an atom, carried through untouched.
        /// Mirrors <c>marja_dharra: u32</c>.
        /// </summary>
        public uint MarjaDharra;

        /// <summary>
        /// A variable-font weight, read only when
        /// <see cref="Alamat.UslubWazn"/> is set. Mirrors <c>wazn: u16</c>.
        /// </summary>
        public ushort Wazn;

        /// <summary>Identifies this span in the output. Mirrors <c>id: u16</c>.</summary>
        public ushort Id;

        /// <summary>
        /// The font index this span prefers, read only when
        /// <see cref="Alamat.UslubKhatt"/> is set. Mirrors <c>khatt: u8</c>.
        /// </summary>
        public byte Khatt;

        /// <summary>
        /// Padding to a multiple of four. Always write zero; ignore it on
        /// read. Mirrors <c>hashw: [u8; 3]</c>.
        /// </summary>
        public fixed byte Hashw[3];
    }

    /// <summary>
    /// An OpenType feature the caller wants on or off beyond the defaults.
    /// Mirrors <c>TaaribSifa</c> in <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct TaaribSifa
    {
        /// <summary>
        /// The four-character tag, for example <c>ss01</c>, in writing order.
        /// Mirrors <c>wasm: [u8; 4]</c>.
        /// </summary>
        public fixed byte Wasm[4];

        /// <summary>The value. Zero turns the feature off. Mirrors <c>qeema: u32</c>.</summary>
        public uint Qeema;

        /// <summary>
        /// Builds a feature setting from its tag, so no call site writes byte
        /// indices by hand and gets the tag order wrong silently — a wrong
        /// tag does not fail, it simply never fires.
        /// </summary>
        /// <param name="wasm">Exactly four ASCII characters.</param>
        /// <param name="qeema">The value; zero turns the feature off.</param>
        /// <returns>The populated setting.</returns>
        /// <exception cref="ArgumentException">
        /// The tag is not exactly four characters, or a character is outside
        /// ASCII — such a tag cannot exist in a font and would only ever fail
        /// silently at shaping time.
        /// </exception>
        public static TaaribSifa Min(string wasm, uint qeema)
        {
            if (wasm is null || wasm.Length != 4)
            {
                throw new ArgumentException(
                    "An OpenType feature tag is exactly four ASCII characters, for example \"ss01\".",
                    nameof(wasm));
            }
            TaaribSifa sifa = default;
            for (int i = 0; i < 4; i++)
            {
                char h = wasm[i];
                if (h > 0x7F)
                {
                    throw new ArgumentException(
                        "An OpenType feature tag is ASCII; a tag outside ASCII cannot exist in a font.",
                        nameof(wasm));
                }
                sifa.Wasm[i] = (byte)h;
            }
            sifa.Qeema = qeema;
            return sifa;
        }
    }

    /// <summary>
    /// The decisions a caller makes once and records into a patch. Mirrors
    /// <c>TaaribKhiyarat</c> in <c>crates/taarib-jisr/src/anwa.rs</c>. The
    /// discriminant fields are typed with the enums above; each is
    /// bit-identical to the <c>u32</c> it mirrors.
    /// </summary>
    /// <remarks>
    /// The <see cref="Sifat"/> pointer is populated by <see cref="Takhtit"/>
    /// from a caller span for the duration of one call only; managed code
    /// never stores a pointer here that outlives the call it was pinned for.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct TaaribKhiyarat
    {
        /// <summary>Extra feature settings. Mirrors <c>sifat: *const TaaribSifa</c>.</summary>
        public TaaribSifa* Sifat;

        /// <summary>How many. Mirrors <c>adad_sifat: usize</c>.</summary>
        public nuint AdadSifat;

        /// <summary>How the base direction is decided. Mirrors <c>ittijah: u32</c>.</summary>
        public IttijahAsas Ittijah;

        /// <summary>The language. Mirrors <c>lugha: u32</c>.</summary>
        public LughaNass Lugha;

        /// <summary>How surplus width is absorbed. Mirrors <c>dabt: u32</c>.</summary>
        public NamatDabt Dabt;

        /// <summary>Where a line sits. Mirrors <c>muhadhaha: u32</c>.</summary>
        public Muhadhaha Muhadhaha;

        /// <summary>The diacritics policy. Mirrors <c>tashkeel: u32</c>.</summary>
        public SiyasatTashkeel Tashkeel;

        /// <summary>The digits policy. Mirrors <c>arqam: u32</c>.</summary>
        public SiyasatArqam Arqam;

        /// <summary>The overflow policy. Mirrors <c>tajawuz: u32</c>.</summary>
        public SiyasatTajawuz Tajawuz;

        /// <summary>
        /// The smallest size shrink-to-fit may use, in pixels. Mirrors
        /// <c>hajm_adna: f32</c>.
        /// </summary>
        public float HajmAdna;

        /// <summary>
        /// Line height in pixels; zero or less means the font's own metrics
        /// decide. Mirrors <c>irtifa_satr: f32</c>.
        /// </summary>
        public float IrtifaSatr;

        /// <summary>
        /// Extra spacing between every pair of glyphs, applied after shaping
        /// so it never disturbs joining. Mirrors <c>tabaud_ahruf: f32</c>.
        /// </summary>
        public float TabaudAhruf;

        /// <summary>
        /// Extra spacing added to every space. Mirrors <c>tabaud_kalimat: f32</c>.
        /// </summary>
        public float TabaudKalimat;

        /// <summary>
        /// <see cref="Alamat.KhiyarHiwar"/>, <see cref="Alamat.KhiyarSatrWahid"/>.
        /// Mirrors <c>alam: u32</c>.
        /// </summary>
        public uint Alam;
    }

    /// <summary>
    /// One layout request. Mirrors <c>TaaribTalab</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    /// <remarks>
    /// Managed code does not assemble this struct by hand:
    /// <see cref="Takhtit"/> builds it over pinned buffers for the duration
    /// of one native call, which is the only lifetime the pointers inside it
    /// are valid for.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct TaaribTalab
    {
        /// <summary>
        /// UTF-8 text in logical order, with markup and placeholders already
        /// lifted into spans. Not required to be NUL-terminated. Mirrors
        /// <c>nass: *const u8</c>.
        /// </summary>
        public byte* Nass;

        /// <summary>Its length in bytes. Mirrors <c>tul_nass: usize</c>.</summary>
        public nuint TulNass;

        /// <summary>The style spans. Mirrors <c>nitaqat: *const TaaribNitaqUslub</c>.</summary>
        public TaaribNitaqUslub* Nitaqat;

        /// <summary>How many. Mirrors <c>adad_nitaqat: usize</c>.</summary>
        public nuint AdadNitaqat;

        /// <summary>
        /// The font chain to shape and draw with — the raw value of a
        /// <see cref="MaqbadSilsila"/>, written by <see cref="Takhtit"/>
        /// while it holds a reference on the handle. Mirrors
        /// <c>silsila: TaaribSilsila</c>.
        /// </summary>
        public IntPtr Silsila;

        /// <summary>The size in pixels. Mirrors <c>hajm: f32</c>.</summary>
        public float Hajm;

        /// <summary>
        /// The width available in pixels; zero or less means one line of
        /// whatever width the text needs. Mirrors <c>ard_mutah: f32</c>.
        /// </summary>
        public float ArdMutah;

        /// <summary>
        /// The height available in pixels; zero or less means unbounded.
        /// Mirrors <c>irtifa_mutah: f32</c>.
        /// </summary>
        public float IrtifaMutah;

        /// <summary>The decisions. Mirrors <c>khiyarat: TaaribKhiyarat</c>.</summary>
        public TaaribKhiyarat Khiyarat;
    }

    /// <summary>
    /// How a context is built. Mirrors <c>TaaribKhiyaratSiyaq</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribKhiyaratSiyaq
    {
        /// <summary>
        /// The layout cache's budget in bytes. Zero disables the cache
        /// entirely, which is what an offline compiler wants and what a game
        /// never does. Mirrors <c>mizaniyat_makhzan: usize</c>.
        /// </summary>
        public nuint MizaniyatMakhzan;

        /// <summary>
        /// How many glyph buffers to keep pooled for the allocation-free
        /// path. Mirrors <c>adad_makhazin: u32</c>.
        /// </summary>
        public uint AdadMakhazin;

        /// <summary>Reserved, must be zero. Mirrors <c>hashw: u32</c>.</summary>
        public uint Hashw;
    }

    /// <summary>
    /// A font's metrics, scaled to a pixel size. Mirrors
    /// <c>TaaribQiyasatKhatt</c> in <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribQiyasatKhatt
    {
        /// <summary>How far the font rises above the baseline. Mirrors <c>suud: f32</c>.</summary>
        public float Suud;

        /// <summary>
        /// How far it falls below, as a positive number. Mirrors <c>hubut: f32</c>.
        /// </summary>
        public float Hubut;

        /// <summary>
        /// The gap the font asks for between lines. Mirrors <c>fajwa: f32</c>.
        /// </summary>
        public float Fajwa;

        /// <summary>
        /// The line height the font recommends. Mirrors <c>irtifa_satr: f32</c>.
        /// </summary>
        public float IrtifaSatr;

        /// <summary>Cap height. Mirrors <c>uluw_kabital: f32</c>.</summary>
        public float UluwKabital;

        /// <summary>x-height. Mirrors <c>uluw_saghir: f32</c>.</summary>
        public float UluwSaghir;

        /// <summary>Units per em, as declared by the font. Mirrors <c>wahdat: u32</c>.</summary>
        public uint Wahdat;

        /// <summary>
        /// Padding to a multiple of eight. Always written as zero; ignore it.
        /// Mirrors <c>hashw: u32</c>.
        /// </summary>
        public uint Hashw;
    }

    /// <summary>
    /// Everything that makes one rasterized image of a glyph distinct.
    /// Mirrors <c>TaaribMiftahShakl</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>: eight bytes.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribMiftahShakl
    {
        /// <summary>The glyph identifier. Mirrors <c>muarrif: u32</c>.</summary>
        public uint Muarrif;

        /// <summary>The pixel size in quarter-pixels. Mirrors <c>hajm_rubi: u16</c>.</summary>
        public ushort HajmRubi;

        /// <summary>Index into the font chain. Mirrors <c>khatt: u8</c>.</summary>
        public byte Khatt;

        /// <summary>The subpixel bucket. Mirrors <c>bakat: u8</c>.</summary>
        public byte Bakat;
    }

    /// <summary>
    /// Where a glyph lives in the atlas, and how to draw it. Mirrors
    /// <c>TaaribMawdiShakl</c> in <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribMawdiShakl
    {
        /// <summary>The glyph's advance at this size. Mirrors <c>taqaddum: f32</c>.</summary>
        public float Taqaddum;

        /// <summary>Left edge in the page, in pixels. Mirrors <c>s: u16</c>.</summary>
        public ushort S;

        /// <summary>Top edge in the page, in pixels. Mirrors <c>a: u16</c>.</summary>
        public ushort A;

        /// <summary>Width in pixels. Mirrors <c>ard: u16</c>.</summary>
        public ushort Ard;

        /// <summary>Height in pixels. Mirrors <c>irtifa: u16</c>.</summary>
        public ushort Irtifa;

        /// <summary>
        /// Left bearing: how far right of the pen the image starts. Mirrors
        /// <c>izaha_s: i16</c>.
        /// </summary>
        public short IzahaS;

        /// <summary>
        /// Top bearing: how far above the baseline the image's top edge sits.
        /// Mirrors <c>izaha_a: i16</c>.
        /// </summary>
        public short IzahaA;

        /// <summary>Which page. Mirrors <c>safha: u16</c>.</summary>
        public ushort Safha;

        /// <summary>
        /// Padding to a multiple of four. Always written as zero; ignore it.
        /// Mirrors <c>hashw: u16</c>.
        /// </summary>
        public ushort Hashw;
    }

    /// <summary>
    /// One texture page, borrowed. Mirrors <c>TaaribSafha</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    /// <remarks>
    /// <see cref="Bayt"/> points into the atlas and stays valid until the
    /// next call that can change the atlas — any glyph lookup, or destroying
    /// the atlas. The caller uploads it and does not keep it.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct TaaribSafha
    {
        /// <summary>
        /// The texels, one byte each, row-major from the top, no row padding.
        /// Mirrors <c>bayt: *const u8</c>.
        /// </summary>
        public byte* Bayt;

        /// <summary>
        /// How many bytes, which is width times height. Mirrors
        /// <c>tul: usize</c>.
        /// </summary>
        public nuint Tul;

        /// <summary>Width in texels. Mirrors <c>ard: u16</c>.</summary>
        public ushort Ard;

        /// <summary>Height in texels. Mirrors <c>irtifa: u16</c>.</summary>
        public ushort Irtifa;

        /// <summary>
        /// <see cref="NamatLawha.Taghtiya"/> or <see cref="NamatLawha.Misafa"/>.
        /// Mirrors <c>namat: u32</c>.
        /// </summary>
        public uint Namat;

        /// <summary>
        /// The texels as a span over the borrowed native memory, so an upload
        /// path can hand them to a texture without copying through a managed
        /// intermediate — a copy here would be an allocation on the path that
        /// uploads atlas growth mid-game.
        /// </summary>
        /// <returns>
        /// The page bytes; empty when the page pointer is null. Valid exactly
        /// as long as <see cref="Bayt"/> is.
        /// </returns>
        public ReadOnlySpan<byte> Muhtawa()
        {
            if (Bayt == null || Tul == 0)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            return new ReadOnlySpan<byte>(Bayt, checked((int)Tul));
        }
    }

    /// <summary>
    /// What the atlas has been doing. Mirrors <c>TaaribIhsaatLawha</c> in
    /// <c>crates/taarib-jisr/src/anwa.rs</c>.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribIhsaatLawha
    {
        /// <summary>Glyphs served without rasterizing anything. Mirrors <c>isabat: u64</c>.</summary>
        public ulong Isabat;

        /// <summary>
        /// Glyphs that had to be rasterized and packed. A number that keeps
        /// climbing after the first minutes of play means the patch compiler
        /// missed strings. Mirrors <c>ikhfaqat: u64</c>.
        /// </summary>
        public ulong Ikhfaqat;

        /// <summary>Rectangles reclaimed to make room. Mirrors <c>ikhlaat: u64</c>.</summary>
        public ulong Ikhlaat;

        /// <summary>Pages opened after the first. Mirrors <c>ahdath_namu: u64</c>.</summary>
        public ulong AhdathNamu;

        /// <summary>What the pages currently cost, in bytes. Mirrors <c>bayt: u64</c>.</summary>
        public ulong Bayt;

        /// <summary>The byte budget. Mirrors <c>mizaniya: u64</c>.</summary>
        public ulong Mizaniya;

        /// <summary>How many glyphs are mapped. Mirrors <c>ashkal: u32</c>.</summary>
        public uint Ashkal;

        /// <summary>How many pages are open. Mirrors <c>safahat: u32</c>.</summary>
        public uint Safahat;
    }

    /// <summary>
    /// What the layout cache has been doing. Mirrors the statistics struct of
    /// <c>crates/taarib-jisr/src/khazina.rs</c>, laid out under the same
    /// frozen rules as <c>anwa.rs</c>: fixed-width fields, widest first,
    /// forty-eight bytes, no padding.
    /// </summary>
    /// <remarks>
    /// The cache's budget is in bytes rather than entries because a cache
    /// measured in entries is a cache that eventually holds a thousand
    /// paragraphs; these counters are what lets an adapter's diagnostics say
    /// whether the budget is sized right for the game it is inside.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct TaaribIhsaatKhazina
    {
        /// <summary>
        /// Layouts served from the cache without shaping. Mirrors
        /// <c>isabat: u64</c>.
        /// </summary>
        public ulong Isabat;

        /// <summary>
        /// Layouts that had to be shaped. A menu that redraws every frame
        /// should raise <see cref="Isabat"/>, not this — a miss rate that
        /// never falls means the cache key is churning. Mirrors
        /// <c>ikhfaqat: u64</c>.
        /// </summary>
        public ulong Ikhfaqat;

        /// <summary>
        /// Layouts evicted to stay inside the budget. Mirrors
        /// <c>ikhlaat: u64</c>.
        /// </summary>
        public ulong Ikhlaat;

        /// <summary>What the cache currently holds, in bytes. Mirrors <c>bayt: u64</c>.</summary>
        public ulong Bayt;

        /// <summary>The byte budget. Mirrors <c>mizaniya: u64</c>.</summary>
        public ulong Mizaniya;

        /// <summary>
        /// How many layouts are held right now. Mirrors <c>madkhalat: u32</c>.
        /// </summary>
        /// <remarks>
        /// A <see cref="uint"/> followed by <see cref="Hashw"/>, not a single
        /// <see cref="ulong"/>. The two shapes are the same 48 bytes and, on a
        /// little-endian machine with the padding zeroed, even read the same
        /// number — which is exactly why the wrong one would ship: it works
        /// until the padding is given a meaning, and then it fails silently on
        /// the field that was quietly absorbing it.
        /// </remarks>
        public uint Madkhalat;

        /// <summary>
        /// Padding to a multiple of eight. Always written as zero; ignore it.
        /// Mirrors <c>hashw: u32</c>.
        /// </summary>
        public uint Hashw;
    }

    /// <summary>
    /// The runtime capture sink
    /// <see cref="MaqbadSiyaq.ShaghghilIltiqat"/> registers. Mirrors
    /// <c>TaaribRaddIltiqat</c> in <c>crates/taarib-jisr/src/hayat.rs</c>:
    /// <c>void (*)(void* mustakhdim, const uint8_t* nass, size_t tul)</c>,
    /// cdecl like every other crossing in this binding — the attribute is
    /// stated because the platform default in a 32-bit Windows process is
    /// StdCall, and a convention mismatch on a callback does not fail
    /// loudly; it corrupts the stack of whichever frame captures next.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <paramref name="nass"/> and <paramref name="tul"/> are the captured
    /// string as UTF-8 bytes with an explicit length — not NUL-terminated,
    /// and borrowed only for the duration of the invocation: a sink that
    /// wants the text copies it before returning.
    /// </para>
    /// <para>
    /// The sink runs on whichever thread called layout, while that
    /// context's lock is held. Its contract has two halves the implementor
    /// must keep: return promptly, because a frame is waiting on it, and
    /// never call back into Taarib on any handle, because the lock it
    /// would need is the lock it is already running under — a sink that
    /// does deadlocks the calling thread, in a game the render thread,
    /// with no diagnostic beyond a frozen process.
    /// </para>
    /// </remarks>
    /// <param name="mustakhdim">
    /// The pointer registered alongside the sink, handed back verbatim and
    /// never dereferenced by the library.
    /// </param>
    /// <param name="nass">The captured UTF-8 bytes; borrowed, not NUL-terminated.</param>
    /// <param name="tul">How many bytes.</param>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void TaaribRaddIltiqat(IntPtr mustakhdim, IntPtr nass, nuint tul);
}
