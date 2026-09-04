// رقعة — reading the installed patch container, inside somebody else's game.
//
// The `.ruqaa` format is defined by crates/taarib-ruqaa, and it is shaped the
// way it is for exactly one reason: five languages have to read it, and one of
// them is running on a frame budget it does not own. So every table in it is a
// fixed-layout POD array with 4-byte indices, 16-byte aligned, little-endian,
// and the intended reader is not a parser. It is a cast.
//
// That is what this file does. The file is memory-mapped, so a 40 MB patch
// costs address space and page cache rather than 40 MB of managed heap that a
// game's garbage collector then has to walk; the tables are read as
// ReadOnlySpan<T> over the mapping through MemoryMarshal.Cast, so a layout the
// compiler precomputed reaches the mesh builder without one allocation, one
// copy, or one byte of parsing. A patch that was compiled correctly is
// therefore free to read and the per-frame path never touches this file's
// code except to index into it.
//
// FOUR OF THE RECORDS HERE ARE THE ABI'S RECORDS. TaaribSatr, TaaribHarf,
// TaaribMiftahShakl and TaaribMawdiShakl are read straight out of the mapping,
// with no conversion, because the container stores them byte for byte as the
// ABI defines them. That is the single most useful property of this format: a
// layout the compiler precomputed and a layout taarib_takhtit produced at run
// time are the same bytes in the same order, so Nasij has one input shape and
// there is no second code path for precompiled text to drift away from. This
// file therefore declares no line, glyph, key or position struct of its own —
// declaring one would be creating exactly the drift the format prevents.
//
// THE OTHER HALF OF THE JOB IS REFUSAL. A `.ruqaa` is a file that arrived from
// a stranger over the internet. Nothing in it is believed before it is checked
// against the one fact this process actually knows — the length of the file on
// disk. A section table claiming a 2^40-byte section is not an allocation that
// fails somewhere later with an unhelpful exception; it is a refusal naming the
// field, at open time, before the game has drawn a frame. Every bound in this
// file exists because the alternative is an out-of-range read inside a process
// Taarib is a guest in.
//
// WHAT THIS ASSEMBLY VERIFIES, AND WHAT IT TRUSTS. Stated here because the
// honest answer is not "everything", and a reader who assumes otherwise would
// be wrong in the direction that matters:
//
//   Verified here, in managed code, with no dependency:
//     - the magic, the format version, and every structural bound;
//     - the BLAKE3 content hash of the body against the header's recorded
//       hash, computed by Basma below;
//     - the sort order of every table this file binary-searches, because a
//       table that is searched has to be sorted and checking it once at load
//       is cheaper than every lookup silently returning the wrong row;
//     - the signer's public key against a key the caller supplies, byte for
//       byte, when the caller supplies one.
//
//   Trusted to the installer, and never claimed here:
//     - the Ed25519 signature over that content hash. .NET Standard 2.1 has
//       no Ed25519, this assembly carries no package reference by design
//       (see unity/README.md), and Taarib.Unity.Jisr exposes no verification
//       entry point. Pretending to verify a signature would be worse than
//       not verifying it, so this file parses the signature block, exposes
//       every field of it, composes the exact byte string that was signed —
//       and says plainly that it did not check it. `taarib-khatm` checked it
//       before `taarib-tathbeet` wrote the file, per Decision 8.
//
// Those two halves compose into something stronger than either. The installer
// verified a signature over the content hash. This file verifies that the
// bytes on disk right now still hash to that same value. Without the second
// check the first one decays the moment the file is written: a container that
// passed verification at install time and was modified afterwards — by a
// crashed updater, a failing disk, or somebody editing the string table —
// would load without a word. With it, the installer's signature check reaches
// forward to the bytes this process is about to draw from.
//
// WHAT THE CONTENT HASH COVERS. From the first byte after the 64-byte header
// to the first byte of the TAWQEE section — not to the end of the file. Two
// exclusions, each for its own reason. The header is excluded because it
// carries the hash and a hash cannot cover itself; every other header field is
// checked against something outside an attacker's control instead. The
// signature section is excluded because the signature is over the hash, so a
// hash covering the signature could never be computed — and that exclusion is
// also what lets taarib-khatm seal a finished package by overwriting 128 bytes
// in place, moving no offset and recomputing nothing. Hashing to the end of
// the file instead would disagree with every patch the compiler produces and
// would reject all of them.
//
// COMPRESSION. The compiler compresses every section but TAWQEE with zstd when
// compressing it makes it smaller. There is no zstd in .NET Standard 2.1 and
// there is no decompression entry point in the ABI, so this reader accepts only
// `compression == 0` and refuses anything else by name. That is not a
// limitation being worked around; it is the division of labour. `taarib-tathbeet`
// already links zstd, already runs on a machine that is not rendering anything,
// and already writes into BepInEx/plugins/Taarib/ — so it decompresses once, at
// install time, into the working copy this file maps. The alternative would mean
// either a second P/Invoke surface (forbidden: Jisr is the only assembly in this
// solution that imports the Taarib native library) or decompressing 40 MB into the managed
// heap at load, which would discard the entire reason the format is
// fixed-layout and mmap-friendly in the first place. See Ruqaa.Iftah for the
// refusal.

using System;
using System.Buffers.Binary;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mushtarak
{
    /// <summary>
    /// The section kinds of a <c>.ruqaa</c> container, as the <c>kind</c> field
    /// of a section-table entry carries them.
    /// </summary>
    /// <remarks>
    /// A kind this build does not recognise is bounds-checked like every other
    /// section and then ignored, rather than refused: a container written by a
    /// newer compiler that added a section is still readable for everything it
    /// shares with this one, and refusing it would turn an additive format
    /// change into a broken install.
    /// </remarks>
    public enum NawQism : uint
    {
        /// <summary>Patch metadata, as a JSON document.</summary>
        Bayan = 1,

        /// <summary>The string table, its style spans, and their UTF-8 pool.</summary>
        Nusus = 2,

        /// <summary>Precomputed layouts: heads, glyphs and lines.</summary>
        Takhtit = 3,

        /// <summary>Atlas pages: a page table and the texels it describes.</summary>
        Lawha = 4,

        /// <summary>The glyph map: a sorted key array and a parallel position array.</summary>
        Khareeta = 5,

        /// <summary>The font chain's records. Never the font files themselves.</summary>
        Khatt = 6,

        /// <summary>Per-string constraints and reflow hints.</summary>
        Qiyud = 7,

        /// <summary>The signature block. Never compressed, always last.</summary>
        Tawqee = 8,
    }

    /// <summary>How a section's bytes are stored.</summary>
    public enum NamatDaght : uint
    {
        /// <summary>Stored exactly as it is read.</summary>
        Bila = 0,

        /// <summary>A single zstd frame. Refused by this reader; see the file header.</summary>
        Zstd = 1,
    }

    /// <summary>Who sealed a patch.</summary>
    /// <remarks>
    /// The distinction is the whole of Decision 8. A client installs a patch
    /// only when this says the owner sealed it; a contributor's self-signature
    /// proves only that the submission arrived intact from the person who made
    /// it, which is what the review queue needs and is not permission to
    /// install anything.
    /// </remarks>
    public enum DawrMiftah : byte
    {
        /// <summary>The project owner's key. The only role a client will install.</summary>
        Malik = 0,

        /// <summary>A contributor signing their own submission on the way into review.</summary>
        Musahim = 1,
    }

    /// <summary>Which signature scheme sealed a patch.</summary>
    public enum Khwarizmiya : byte
    {
        /// <summary>Nothing has sealed it. The compiler writes this into its reservation.</summary>
        Ghayr = 0,

        /// <summary>Ed25519 over the 97-byte message.</summary>
        Ed25519 = 1,
    }

    /// <summary>How far verification of the signature block got in this process.</summary>
    public enum HalatTawqee : uint
    {
        /// <summary>Nothing has been read yet.</summary>
        Bila = 0,

        /// <summary>Parsed and well formed. Its signature was not checked.</summary>
        Maqru = 1,

        /// <summary>Its key matches the installer's record. Still not a signature check.</summary>
        MutabiqLilMiftah = 2,

        /// <summary>The block carries no signature at all.</summary>
        GhayrMuwaqqaa = 3,
    }

    // How a patch's atlas was rasterized is Taarib.Unity.Jisr.NamatLawha, not a
    // second enumeration declared here. The container's header flag and the
    // rasterizer's mode are the same fact, and two types for one fact is how a
    // patch comes to be uploaded as coverage and drawn as a distance field.

    /// <summary>The header's flag bits.</summary>
    public static class AlamatRuqaa
    {
        /// <summary>The atlas carries eight-bit coverage pages.</summary>
        public const ushort Taghtiya = 1 << 0;

        /// <summary>The atlas carries signed distance field pages.</summary>
        public const ushort Misafa = 1 << 1;

        /// <summary>The constraints carry right-to-left interface mirroring hints.</summary>
        public const ushort MiraatYameen = 1 << 2;

        /// <summary>The constraints were informed by a runtime capture session.</summary>
        public const ushort TalmihatIltiqat = 1 << 3;

        /// <summary>Every bit this build defines. Anything else is a refusal.</summary>
        public const ushort Marufa = Taghtiya | Misafa | MiraatYameen | TalmihatIltiqat;
    }

    /// <summary>The flag bits of a style span.</summary>
    public static class AlamatNitaq
    {
        /// <summary>The span is italic or slanted.</summary>
        public const ushort Maail = 1 << 0;

        /// <summary>The span is an opaque atom and is never shaped or reordered.</summary>
        public const ushort Dharra = 1 << 1;

        /// <summary>The colour field is meaningful. Without it, zero means nothing.</summary>
        public const ushort Lawn = 1 << 2;

        /// <summary>The span is bold or heavier than the run around it.</summary>
        public const ushort Aswad = 1 << 3;

        /// <summary>The span is an inline sprite; its identity names the sprite.</summary>
        public const ushort Sura = 1 << 4;
    }

    /// <summary>The flag bits of a precomputed layout.</summary>
    public static class AlamatTakhtit
    {
        /// <summary>The layout's overall direction is right to left.</summary>
        public const ushort Yameen = 1 << 0;

        /// <summary>The overflow policy truncated the text.</summary>
        public const ushort Maqsus = 1 << 1;

        /// <summary>The text did not fit its measured bounds, and shipped anyway.</summary>
        public const ushort Tajawuz = 1 << 2;

        /// <summary>The overflow policy shrank the text below the requested size.</summary>
        public const ushort Musaghghar = 1 << 3;
    }

    /// <summary>The permission bits of a constraint record.</summary>
    public static class AlamatQayd
    {
        /// <summary>This element may be mirrored for a right-to-left interface.</summary>
        public const uint Mirah = 1u << 0;

        /// <summary>Its container may be grown horizontally to fit.</summary>
        public const uint NamuArd = 1u << 1;

        /// <summary>Its container may be grown vertically to fit.</summary>
        public const uint NamuIrtifa = 1u << 2;

        /// <summary>Its anchoring may be re-pinned when it is mirrored or grown.</summary>
        public const uint IadatTathbit = 1u << 3;

        /// <summary>The engine refuses to wrap this string.</summary>
        public const uint SatrWahid = 1u << 4;

        /// <summary>This string is dialogue, which the diacritics policy keys on.</summary>
        public const uint Hiwar = 1u << 5;

        /// <summary>Auto-sizing is on, so the minimum size field means something.</summary>
        public const uint HajmTilqai = 1u << 6;
    }

    /// <summary>The flag bits of a font record.</summary>
    public static class AlamatKhatt
    {
        /// <summary>The primary font: the one a glyph is looked for in first.</summary>
        public const ushort Asasi = 1 << 0;

        /// <summary>A fallback, consulted in chain order when the primary has no glyph.</summary>
        public const ushort Ihtiyati = 1 << 1;

        /// <summary>
        /// This font was verified to carry complete Arabic OpenType tables when
        /// the patch was compiled. Decision 6 refuses to ship a font without
        /// them, and this bit records that the check ran rather than being
        /// assumed.
        /// </summary>
        public const ushort JadawilKamila = 1 << 2;
    }

    /// <summary>Where a validated section sits in the mapped file.</summary>
    public readonly struct MawdiQism
    {
        /// <summary>Records a validated section.</summary>
        /// <param name="naw">Its kind.</param>
        /// <param name="izaha">Its first byte, as an offset from the start of the file.</param>
        /// <param name="tul">Its length in bytes.</param>
        public MawdiQism(NawQism naw, long izaha, long tul)
        {
            Naw = naw;
            Izaha = izaha;
            Tul = tul;
        }

        /// <summary>The section kind.</summary>
        public NawQism Naw { get; }

        /// <summary>Byte offset of the section's first byte from the start of the file.</summary>
        public long Izaha { get; }

        /// <summary>The section's length in bytes; negative when the section is absent.</summary>
        public long Tul { get; }

        /// <summary>Whether the container carries this section at all.</summary>
        public bool Mawjud => Tul >= 0;
    }

    /// <summary>
    /// One entry of the <see cref="NawQism.Nusus"/> string table: the key of the
    /// source string, and where its translation sits in the section's UTF-8
    /// pool. Sixteen bytes, read by overlay.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The array is sorted ascending by <see cref="Miftah"/>, so a lookup is a
    /// binary search over the mapped file with no hash map built at load and no
    /// allocation at all. That ordering is part of the format, not an
    /// implementation detail: a container whose keys were unsorted would not be
    /// searchable by any of the five languages that read it, and
    /// <see cref="Ruqaa"/> refuses one at open.
    /// </para>
    /// <para>
    /// The source text itself is not stored. Sixty-four bits of BLAKE3 over a
    /// hundred thousand strings gives a collision probability around one in
    /// twenty billion, and carrying every source string would roughly double the
    /// pool for a check no consumer performs at run time.
    /// </para>
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public readonly struct MadkhalNass
    {
        /// <summary>
        /// The first eight bytes of the BLAKE3 hash of the source string's UTF-8
        /// bytes, read little-endian. Computed on this side by
        /// <see cref="Ruqaa.MiftahMinNass(string)"/>.
        /// </summary>
        public readonly ulong Miftah;

        /// <summary>Byte offset of the translation in the section's UTF-8 pool.</summary>
        public readonly uint Izaha;

        /// <summary>Its length in bytes.</summary>
        public readonly uint Tul;
    }

    /// <summary>
    /// One style span over a string's translated text. Twenty-four bytes, read
    /// by overlay, sorted ascending by <see cref="Nass"/>.
    /// </summary>
    /// <remarks>
    /// <see cref="Id"/> is what <see cref="TaaribHarf.Nitaq"/> carries, and it is
    /// why colour survives the bidirectional algorithm: a glyph moved to the
    /// other end of the line still names the span it came from, so the mesh
    /// builder colours it correctly without knowing anything about where it
    /// started.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public readonly struct MadkhalNitaq
    {
        /// <summary>Index into the string table of the string this span belongs to.</summary>
        public readonly uint Nass;

        /// <summary>Byte offset of the span's first byte within the translated text.</summary>
        public readonly uint Bidaya;

        /// <summary>Its length in bytes.</summary>
        public readonly uint Tul;

        /// <summary>Colour as <c>0xRRGGBBAA</c>. Meaningless without the colour flag.</summary>
        public readonly uint Lawn;

        /// <summary>A size override in pixels, or zero for none.</summary>
        public readonly float Hajm;

        /// <summary>The span's identity, which is what a glyph's span field names.</summary>
        public readonly ushort Id;

        /// <summary>The flags, per <see cref="AlamatNitaq"/>.</summary>
        public readonly ushort Alam;

        /// <summary>Whether the span is italic or slanted.</summary>
        public bool Maail => (Alam & AlamatNitaq.Maail) != 0;

        /// <summary>Whether the span is an opaque atom that is never shaped.</summary>
        public bool Dharra => (Alam & AlamatNitaq.Dharra) != 0;

        /// <summary>Whether <see cref="Lawn"/> carries a colour.</summary>
        public bool LahuLawn => (Alam & AlamatNitaq.Lawn) != 0;

        /// <summary>Whether the span is bold.</summary>
        public bool Aswad => (Alam & AlamatNitaq.Aswad) != 0;

        /// <summary>Whether the span is an inline sprite.</summary>
        public bool Sura => (Alam & AlamatNitaq.Sura) != 0;
    }

    /// <summary>
    /// One entry of the <see cref="NawQism.Takhtit"/> table: one string laid out
    /// once, at one size, by the patch compiler. Thirty-two bytes, read by
    /// overlay.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The glyphs and lines this entry names are stored as
    /// <see cref="TaaribHarf"/> and <see cref="TaaribSatr"/> — the ABI's own
    /// structs, byte for byte — so a layout the compiler precomputed and a
    /// layout <c>taarib_takhtit</c> produced at run time reach the mesh builder
    /// as the same bytes.
    /// </para>
    /// <para>
    /// The array is sorted ascending by <see cref="Nass"/> and then by
    /// <see cref="HajmRubi"/>, which is what makes
    /// <see cref="Ruqaa.JidTakhtit"/> a binary search and what lets one string
    /// carry several layouts. It has to: a game draws the same label at one size
    /// in a menu and another in a tooltip.
    /// </para>
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public readonly struct MadkhalTakhtit
    {
        /// <summary>Index into the string table of the string this laid out.</summary>
        public readonly uint Nass;

        /// <summary>Index of the first glyph in the section's shared glyph array.</summary>
        public readonly uint AwwalHarf;

        /// <summary>How many glyphs the layout has.</summary>
        public readonly uint AdadHuruf;

        /// <summary>Index of the first line in the section's shared line array.</summary>
        public readonly uint AwwalSatr;

        /// <summary>How many lines it has.</summary>
        public readonly uint AdadSutur;

        /// <summary>The width of the widest line, in pixels.</summary>
        public readonly float Ard;

        /// <summary>The total height of every line box, in pixels.</summary>
        public readonly float Irtifa;

        /// <summary>
        /// The size this was laid out at, in quarter-pixels — the same unit and
        /// the same quantization as <see cref="TaaribMiftahShakl.HajmRubi"/>, so
        /// a layout and the glyph images it refers to can never disagree about
        /// what size means.
        /// </summary>
        public readonly ushort HajmRubi;

        /// <summary>The flags, per <see cref="AlamatTakhtit"/>.</summary>
        public readonly ushort Alam;

        /// <summary>The size this was laid out at, in pixels.</summary>
        public float Hajm => HajmRubi / 4.0f;

        /// <summary>Whether the layout's overall direction is right to left.</summary>
        public bool Yameen => (Alam & AlamatTakhtit.Yameen) != 0;

        /// <summary>Whether the overflow policy truncated the text.</summary>
        public bool Maqsus => (Alam & AlamatTakhtit.Maqsus) != 0;

        /// <summary>Whether the text did not fit the space the compiler measured it in.</summary>
        public bool Mutajawiz => (Alam & AlamatTakhtit.Tajawuz) != 0;

        /// <summary>Whether the overflow policy shrank the text to reach this size.</summary>
        public bool Musaghghar => (Alam & AlamatTakhtit.Musaghghar) != 0;
    }

    /// <summary>
    /// One entry of the <see cref="NawQism.Qiyud"/> table: everything the
    /// compiler measured about the space one string is drawn into, and what the
    /// patch permits an adapter to change about it. Forty-eight bytes, read by
    /// overlay, sorted ascending by <see cref="Nass"/>.
    /// </summary>
    /// <remarks>
    /// This is the row that makes runtime text behave like precompiled text.
    /// When a string reaches a takeover point that the compiler never saw — a
    /// player name, a composed sentence, a number substituted into a format
    /// placeholder — the adapter still knows the slot it is going into, and
    /// <see cref="Khiyarat"/> turns this row straight into the layout options
    /// for it. Without that, unknown text would be laid out with defaults and
    /// would visibly disagree with the text beside it about alignment,
    /// justification and diacritics.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public readonly struct MadkhalQayd
    {
        /// <summary>Index into the string table of the string this constrains.</summary>
        public readonly uint Nass;

        /// <summary>The width available in pixels; zero or less means unbounded.</summary>
        public readonly float ArdMutah;

        /// <summary>The height available in pixels; zero or less means unbounded.</summary>
        public readonly float IrtifaMutah;

        /// <summary>The size the original is drawn at, in pixels.</summary>
        public readonly float Hajm;

        /// <summary>The smallest size auto-sizing may use, in pixels.</summary>
        public readonly float HajmAdna;

        /// <summary>Line height in pixels; zero or less lets the font's metrics decide.</summary>
        public readonly float IrtifaSatr;

        /// <summary>Extra spacing between glyphs, in pixels.</summary>
        public readonly float TabaudAhruf;

        /// <summary>Extra spacing added to every space, in pixels.</summary>
        public readonly float TabaudKalimat;

        /// <summary>The permissions, per <see cref="AlamatQayd"/>.</summary>
        public readonly uint Alam;

        /// <summary>Where the line sits, as a <see cref="Muhadhaha"/> value.</summary>
        public readonly byte MuhadhahaKhaam;

        /// <summary>How base direction is decided, as an <see cref="IttijahAsas"/> value.</summary>
        public readonly byte IttijahKhaam;

        /// <summary>How surplus width is absorbed, as a <see cref="NamatDabt"/> value.</summary>
        public readonly byte DabtKhaam;

        /// <summary>The diacritics policy, as a <see cref="SiyasatTashkeel"/> value.</summary>
        public readonly byte TashkeelKhaam;

        /// <summary>The digits policy, as a <see cref="SiyasatArqam"/> value.</summary>
        public readonly byte ArqamKhaam;

        /// <summary>The overflow policy, as a <see cref="SiyasatTajawuz"/> value.</summary>
        public readonly byte TajawuzKhaam;

        /// <summary>The declared language, as a <see cref="LughaNass"/> value.</summary>
        public readonly byte LughaKhaam;

        /// <summary>Padding to a multiple of four. Always zero; ignore it.</summary>
        public readonly byte Hashw0;

        /// <summary>Padding to a multiple of eight. Always zero; ignore it.</summary>
        public readonly uint Hashw1;

        /// <summary>Whether this element may be mirrored for a right-to-left interface.</summary>
        public bool Yumrah => (Alam & AlamatQayd.Mirah) != 0;

        /// <summary>Whether its container may be grown horizontally to fit.</summary>
        public bool YanmuArdan => (Alam & AlamatQayd.NamuArd) != 0;

        /// <summary>Whether its container may be grown vertically to fit.</summary>
        public bool YanmuIrtifaan => (Alam & AlamatQayd.NamuIrtifa) != 0;

        /// <summary>Whether its anchoring may be re-pinned when it is mirrored or grown.</summary>
        public bool YuadTathbituh => (Alam & AlamatQayd.IadatTathbit) != 0;

        /// <summary>Whether the engine refuses to wrap this string.</summary>
        public bool SatrWahid => (Alam & AlamatQayd.SatrWahid) != 0;

        /// <summary>Whether this string is dialogue.</summary>
        public bool Hiwar => (Alam & AlamatQayd.Hiwar) != 0;

        /// <summary>Whether auto-sizing is on for it.</summary>
        public bool HajmTilqai => (Alam & AlamatQayd.HajmTilqai) != 0;

        /// <summary>
        /// This row as the layout options a runtime layout of unknown text in
        /// the same slot must use. A struct returned by value: nothing here
        /// allocates, so the call is safe from a takeover point.
        /// </summary>
        /// <returns>The options, with the discriminants widened from their stored bytes.</returns>
        /// <remarks>
        /// The discriminants are widened rather than validated. Every enum the
        /// ABI defines documents an unrecognised number as resolving to its own
        /// default on the native side, so a container from a newer compiler that
        /// declares a policy this build has never heard of lays the text out
        /// with the default policy instead of refusing to draw it. Text in the
        /// wrong justification mode is a flaw; no text is a broken patch.
        /// </remarks>
        public KhiyaratTakhtit Khiyarat()
        {
            KhiyaratTakhtit kh = default;
            kh.Ittijah = (IttijahAsas)IttijahKhaam;
            kh.Lugha = (LughaNass)LughaKhaam;
            kh.Dabt = (NamatDabt)DabtKhaam;
            kh.Muhadhaha = (Muhadhaha)MuhadhahaKhaam;
            kh.Tashkeel = (SiyasatTashkeel)TashkeelKhaam;
            kh.Arqam = (SiyasatArqam)ArqamKhaam;
            kh.Tajawuz = (SiyasatTajawuz)TajawuzKhaam;
            kh.HajmAdna = HajmAdna;
            kh.IrtifaSatr = IrtifaSatr;
            kh.TabaudAhruf = TabaudAhruf;
            kh.TabaudKalimat = TabaudKalimat;
            if (SatrWahid)
            {
                kh.Alam |= Alamat.KhiyarSatrWahid;
            }
            if (Hiwar)
            {
                kh.Alam |= Alamat.KhiyarHiwar;
            }
            return kh;
        }
    }

    /// <summary>
    /// One entry of the <see cref="NawQism.Lawha"/> page table: where one atlas
    /// page's texels sit inside the section, and how big the page is. Sixteen
    /// bytes, read by overlay.
    /// </summary>
    /// <remarks>
    /// The texels are one byte per texel, row-major from the top, with no row
    /// padding — the same buffer shape <c>taarib_lawha_safha</c> hands back for a
    /// page rasterized at run time, so an adapter has one upload path rather
    /// than two.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public readonly struct MadkhalSafha
    {
        /// <summary>Byte offset of the page's texels from the start of its section.</summary>
        public readonly uint Izaha;

        /// <summary>
        /// How many texel bytes, which must equal <see cref="Ard"/> times
        /// <see cref="Irtifa"/> — checked at open, because a page whose byte
        /// count disagrees with its dimensions uploads a texture whose rows are
        /// offset from the rectangles the glyph map names, and every glyph then
        /// draws a little wrong in a way that reads as a font bug.
        /// </summary>
        public readonly uint Tul;

        /// <summary>Width in texels.</summary>
        public readonly ushort Ard;

        /// <summary>Height in texels.</summary>
        public readonly ushort Irtifa;

        /// <summary>Padding to a multiple of eight. Always zero; ignore it.</summary>
        public readonly uint Hashw;
    }

    /// <summary>
    /// One entry of the <see cref="NawQism.Khatt"/> record: which font the patch
    /// declares at one position of its chain, under what name, and with what
    /// content hash. Sixteen bytes, read by overlay.
    /// </summary>
    /// <remarks>
    /// A record, not a font. The container carries no font bytes and this reader
    /// loads none: the files themselves live in
    /// <c>BepInEx/plugins/Taarib/khutut/</c>, put there by the installer, and the
    /// adapter hands their bytes to <c>taarib_khatt_min_dhakira</c>. What this
    /// section provides is the name to look for, the position in the chain to
    /// load it at, and the hash to check it against — so a font file replaced on
    /// disk after installation is caught rather than shaped with.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public readonly struct MadkhalKhatt
    {
        /// <summary>Byte offset of the file name in the section's UTF-8 pool.</summary>
        public readonly uint IzahatIsm;

        /// <summary>Its length in bytes.</summary>
        public readonly uint TulIsm;

        /// <summary>
        /// Byte offset from the start of the section of this font's 32-byte
        /// BLAKE3 content hash.
        /// </summary>
        public readonly uint IzahatBasma;

        /// <summary>
        /// The position this font takes in the patch's font chain, which is what
        /// <see cref="TaaribHarf.Khatt"/> indexes into.
        /// </summary>
        public readonly ushort Fahras;

        /// <summary>The record flags, per <see cref="AlamatKhatt"/>.</summary>
        public readonly ushort Alam;

        /// <summary>Whether this is the primary font of the chain.</summary>
        public bool Asasi => (Alam & AlamatKhatt.Asasi) != 0;

        /// <summary>Whether this is a fallback.</summary>
        public bool Ihtiyati => (Alam & AlamatKhatt.Ihtiyati) != 0;

        /// <summary>Whether the compiler verified its Arabic OpenType tables.</summary>
        public bool JadawilKamila => (Alam & AlamatKhatt.JadawilKamila) != 0;
    }

    /// <summary>
    /// بصمة — BLAKE3, in managed code, because there is no other way to have
    /// it here.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The container records a BLAKE3 hash of its own body and this assembly
    /// has to check it. .NET Standard 2.1 offers MD5, SHA-1, SHA-2 and nothing
    /// newer, the project carries no <c>PackageReference</c> by design, and
    /// the ABI exports no hashing entry point — so the choice was between
    /// implementing the function and skipping the check. Skipping it would
    /// have quietly thrown away the only thing that keeps the installer's
    /// signature verification meaningful at load time, which is the argument
    /// set out at the top of this file.
    /// </para>
    /// <para>
    /// This is the reference construction, single-threaded and portable: a
    /// tree of 1 KiB chunks, each chunk a chain of 64-byte compressions, merged
    /// through a stack of at most 54 chaining values — 54 because 2^54 chunks
    /// of 1 KiB is 2^64 bytes, so the stack cannot overflow for any input a
    /// <c>Span</c> can describe. Nothing here allocates: every working buffer
    /// is <c>stackalloc</c>, and the input is the memory-mapped file itself.
    /// One pass over a 40 MB patch costs a fraction of the time the game
    /// spends loading its own first scene, and it happens once, on the loading
    /// thread, before a frame is drawn.
    /// </para>
    /// <para>
    /// No SIMD, no unrolling beyond the round function, no parallelism. This
    /// code runs once per patch load and is read by people auditing what a
    /// plugin does to their game; a version that is fast and hard to check
    /// against the specification would be the wrong trade in both directions.
    /// </para>
    /// </remarks>
    internal static class Basma
    {
        /// <summary>The digest length in bytes.</summary>
        public const int Tul = 32;

        private const int TulKutla = 64;
        private const int TulQitaa = 1024;
        private const int AqsaMakdas = 54;

        private const uint BidayatQitaa = 1;
        private const uint NihayatQitaa = 2;
        private const uint Waalid = 4;
        private const uint Jidhr = 8;

        private const uint Asas0 = 0x6A09E667u;
        private const uint Asas1 = 0xBB67AE85u;
        private const uint Asas2 = 0x3C6EF372u;
        private const uint Asas3 = 0xA54FF53Au;
        private const uint Asas4 = 0x510E527Fu;
        private const uint Asas5 = 0x9B05688Cu;
        private const uint Asas6 = 0x1F83D9ABu;
        private const uint Asas7 = 0x5BE0CD19u;

        /// <summary>
        /// Hashes the input into a 32-byte digest.
        /// </summary>
        /// <param name="madkhal">The bytes to hash; may be empty.</param>
        /// <param name="kharij">The destination, exactly <see cref="Tul"/> bytes.</param>
        /// <exception cref="ArgumentException">
        /// <paramref name="kharij"/> is not <see cref="Tul"/> bytes long.
        /// </exception>
        public static void Ihsib(ReadOnlySpan<byte> madkhal, Span<byte> kharij)
        {
            if (kharij.Length != Tul)
            {
                throw new ArgumentException("A BLAKE3 digest is exactly 32 bytes.", nameof(kharij));
            }

            Span<uint> makdas = stackalloc uint[AqsaMakdas * 8];
            Span<uint> silsila = stackalloc uint[8];
            Span<uint> kalimat = stackalloc uint[16];
            Span<uint> halat = stackalloc uint[16];
            int adadMakdas = 0;
            ulong raqm = 0;
            int mawdi = 0;
            int baqi = madkhal.Length;

            // Every chunk but the last is folded into the stack as it closes.
            // The last one stays open, because only it can carry the root flag.
            while (baqi > TulQitaa)
            {
                QitaaKamil(madkhal.Slice(mawdi, TulQitaa), raqm, silsila, kalimat, halat);
                Adkhil(makdas, ref adadMakdas, silsila, raqm + 1, kalimat, halat);
                mawdi += TulQitaa;
                baqi -= TulQitaa;
                raqm++;
            }

            Hayyi(silsila);
            int muqaddam = baqi == 0 ? 0 : (baqi - 1) / TulKutla;
            for (int i = 0; i < muqaddam; i++)
            {
                IqraKalimat(madkhal.Slice(mawdi + (i * TulKutla), TulKutla), kalimat);
                Idghat(silsila, kalimat, raqm, TulKutla, i == 0 ? BidayatQitaa : 0u, halat);
                halat.Slice(0, 8).CopyTo(silsila);
            }

            int tulAkhira = baqi - (muqaddam * TulKutla);
            IqraJuz(madkhal.Slice(mawdi + (muqaddam * TulKutla), tulAkhira), kalimat);
            uint alam = (muqaddam == 0 ? BidayatQitaa : 0u) | NihayatQitaa;
            uint tulAlan = (uint)tulAkhira;

            // Fold the stack from the top down. Each step turns the still-open
            // node into the right child of a parent, so the parent is what is
            // open next and the root flag lands on the last one of all.
            while (adadMakdas > 0)
            {
                adadMakdas--;
                Idghat(silsila, kalimat, raqm, tulAlan, alam, halat);
                makdas.Slice(adadMakdas * 8, 8).CopyTo(kalimat.Slice(0, 8));
                halat.Slice(0, 8).CopyTo(kalimat.Slice(8, 8));
                Hayyi(silsila);
                raqm = 0;
                tulAlan = TulKutla;
                alam = Waalid;
            }

            Idghat(silsila, kalimat, raqm, tulAlan, alam | Jidhr, halat);
            for (int i = 0; i < 8; i++)
            {
                BinaryPrimitives.WriteUInt32LittleEndian(kharij.Slice(i * 4, 4), halat[i]);
            }
        }

        /// <summary>
        /// Hashes the input and returns the first eight bytes as a
        /// little-endian number — the string-table key of the container.
        /// </summary>
        /// <param name="madkhal">The bytes to hash.</param>
        /// <returns>The truncated digest as a number.</returns>
        public static ulong Ihsib64(ReadOnlySpan<byte> madkhal)
        {
            Span<byte> basma = stackalloc byte[Tul];
            Ihsib(madkhal, basma);
            return BinaryPrimitives.ReadUInt64LittleEndian(basma);
        }

        /// <summary>
        /// Compares two byte strings without an early exit.
        /// </summary>
        /// <param name="awwal">One side.</param>
        /// <param name="thani">The other.</param>
        /// <returns>Whether they are the same length and the same bytes.</returns>
        /// <remarks>
        /// <see cref="MemoryExtensions.SequenceEqual{T}(Span{T}, ReadOnlySpan{T})"/>
        /// would be correct and faster. This is used for the content hash and
        /// the signer's public key, and a comparison that returns as soon as
        /// it finds a difference is a habit worth not having in the two places
        /// in this assembly where an attacker chooses one of the inputs.
        /// </remarks>
        public static bool Yutabiq(ReadOnlySpan<byte> awwal, ReadOnlySpan<byte> thani)
        {
            if (awwal.Length != thani.Length)
            {
                return false;
            }
            int farq = 0;
            for (int i = 0; i < awwal.Length; i++)
            {
                farq |= awwal[i] ^ thani[i];
            }
            return farq == 0;
        }

        private static void Hayyi(Span<uint> silsila)
        {
            silsila[0] = Asas0;
            silsila[1] = Asas1;
            silsila[2] = Asas2;
            silsila[3] = Asas3;
            silsila[4] = Asas4;
            silsila[5] = Asas5;
            silsila[6] = Asas6;
            silsila[7] = Asas7;
        }

        private static void QitaaKamil(
            ReadOnlySpan<byte> qitaa, ulong raqm, Span<uint> silsila,
            Span<uint> kalimat, Span<uint> halat)
        {
            Hayyi(silsila);
            for (int i = 0; i < 16; i++)
            {
                IqraKalimat(qitaa.Slice(i * TulKutla, TulKutla), kalimat);
                uint alam = (i == 0 ? BidayatQitaa : 0u) | (i == 15 ? NihayatQitaa : 0u);
                Idghat(silsila, kalimat, raqm, TulKutla, alam, halat);
                halat.Slice(0, 8).CopyTo(silsila);
            }
        }

        private static void Adkhil(
            Span<uint> makdas, ref int adad, Span<uint> silsila, ulong majmu,
            Span<uint> kalimat, Span<uint> halat)
        {
            Span<uint> mabda = stackalloc uint[8];
            // The stack depth is an invariant of the tree, not an input, so the
            // `adad > 0` guard can never fire. It is here so that a defect
            // elsewhere in this file produces a wrong hash — which the caller
            // refuses on — rather than an index outside a stack buffer.
            while ((majmu & 1) == 0 && adad > 0)
            {
                adad--;
                makdas.Slice(adad * 8, 8).CopyTo(kalimat.Slice(0, 8));
                silsila.CopyTo(kalimat.Slice(8, 8));
                Hayyi(mabda);
                Idghat(mabda, kalimat, 0, TulKutla, Waalid, halat);
                halat.Slice(0, 8).CopyTo(silsila);
                majmu >>= 1;
            }
            silsila.CopyTo(makdas.Slice(adad * 8, 8));
            adad++;
        }

        private static void Idghat(
            ReadOnlySpan<uint> silsila, ReadOnlySpan<uint> kalimat, ulong addad,
            uint tulKutla, uint alam, Span<uint> khuruj)
        {
            silsila.Slice(0, 8).CopyTo(khuruj);
            khuruj[8] = Asas0;
            khuruj[9] = Asas1;
            khuruj[10] = Asas2;
            khuruj[11] = Asas3;
            khuruj[12] = (uint)addad;
            khuruj[13] = (uint)(addad >> 32);
            khuruj[14] = tulKutla;
            khuruj[15] = alam;

            Span<uint> m = stackalloc uint[16];
            kalimat.Slice(0, 16).CopyTo(m);
            for (int j = 0; j < 7; j++)
            {
                Jawla(khuruj, m);
                if (j < 6)
                {
                    Baddil(m);
                }
            }
            for (int i = 0; i < 8; i++)
            {
                khuruj[i] ^= khuruj[i + 8];
                khuruj[i + 8] ^= silsila[i];
            }
        }

        private static void Jawla(Span<uint> h, ReadOnlySpan<uint> m)
        {
            Ghayyir(h, 0, 4, 8, 12, m[0], m[1]);
            Ghayyir(h, 1, 5, 9, 13, m[2], m[3]);
            Ghayyir(h, 2, 6, 10, 14, m[4], m[5]);
            Ghayyir(h, 3, 7, 11, 15, m[6], m[7]);
            Ghayyir(h, 0, 5, 10, 15, m[8], m[9]);
            Ghayyir(h, 1, 6, 11, 12, m[10], m[11]);
            Ghayyir(h, 2, 7, 8, 13, m[12], m[13]);
            Ghayyir(h, 3, 4, 9, 14, m[14], m[15]);
        }

        private static void Ghayyir(Span<uint> h, int a, int b, int c, int d, uint ms, uint ma)
        {
            unchecked
            {
                h[a] = h[a] + h[b] + ms;
                h[d] = Dawwir(h[d] ^ h[a], 16);
                h[c] = h[c] + h[d];
                h[b] = Dawwir(h[b] ^ h[c], 12);
                h[a] = h[a] + h[b] + ma;
                h[d] = Dawwir(h[d] ^ h[a], 8);
                h[c] = h[c] + h[d];
                h[b] = Dawwir(h[b] ^ h[c], 7);
            }
        }

        private static uint Dawwir(uint qeema, int adad)
        {
            return (qeema >> adad) | (qeema << (32 - adad));
        }

        private static void Baddil(Span<uint> m)
        {
            Span<uint> n = stackalloc uint[16];
            n[0] = m[2];
            n[1] = m[6];
            n[2] = m[3];
            n[3] = m[10];
            n[4] = m[7];
            n[5] = m[0];
            n[6] = m[4];
            n[7] = m[13];
            n[8] = m[1];
            n[9] = m[11];
            n[10] = m[12];
            n[11] = m[5];
            n[12] = m[9];
            n[13] = m[14];
            n[14] = m[15];
            n[15] = m[8];
            n.CopyTo(m);
        }

        private static void IqraKalimat(ReadOnlySpan<byte> bayt, Span<uint> kalimat)
        {
            for (int i = 0; i < 16; i++)
            {
                kalimat[i] = BinaryPrimitives.ReadUInt32LittleEndian(bayt.Slice(i * 4, 4));
            }
        }

        private static void IqraJuz(ReadOnlySpan<byte> bayt, Span<uint> kalimat)
        {
            Span<byte> kutla = stackalloc byte[TulKutla];
            kutla.Clear();
            bayt.CopyTo(kutla);
            IqraKalimat(kutla, kalimat);
        }
    }

    /// <summary>
    /// An installed <c>.ruqaa</c> container, mapped and validated.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Open one when the plugin loads and dispose it when the plugin unloads.
    /// Between those two points every accessor is a bounds-checked index into
    /// the mapping, allocates nothing, and is safe to call from a takeover point
    /// while a frame is waiting.
    /// </para>
    /// <para>
    /// Thread safety: this type is immutable after construction and every
    /// accessor is a read. Several threads may read one container at once. The
    /// one exception is <see cref="Dispose"/>, which must not race a read — the
    /// mapping is released and a span handed out before it would be pointing at
    /// unmapped pages.
    /// </para>
    /// <para>
    /// Note what this type cannot do, structurally rather than by convention:
    /// the only file it opens is the path it was given, the only bytes it
    /// exposes are that file's, and there is no method on it that takes a font,
    /// a texture, an asset bundle, or anything belonging to the game. That is
    /// Decision 5 expressed as an absence of code paths.
    /// </para>
    /// </remarks>
    public sealed unsafe class Ruqaa : IDisposable
    {
        /// <summary>The four magic bytes <c>TRQ1</c>, read as a little-endian number.</summary>
        public const uint Wasm = 0x3151_5254u;

        /// <summary>The one format version this build reads.</summary>
        public const ushort IsdarMafhum = 1;

        /// <summary>The header's length in bytes.</summary>
        public const int TulRas = 64;

        /// <summary>One section-table entry's length in bytes.</summary>
        public const int TulMadkhalQism = 32;

        /// <summary>The alignment every section's first byte satisfies.</summary>
        public const int Muhadhat = 16;

        /// <summary>
        /// The most sections a container may declare, which is one per kind.
        /// </summary>
        /// <remarks>
        /// Eight, matching the format, and not a larger number left over for
        /// growth. A kind this build does not know is still skipped rather than
        /// refused, so an additive change stays readable — but a kind is a
        /// number in <see cref="NawQism"/>, there are eight of them, and a
        /// container declaring nine sections is declaring one of them twice.
        /// </remarks>
        public const int AqsaAqsam = 8;

        /// <summary>
        /// The largest container this reader will map, in bytes.
        /// </summary>
        /// <remarks>
        /// Half a gigabyte, which is stricter than the format's own two-gibibyte
        /// ceiling, deliberately. This reader runs inside a game — sometimes a
        /// 32-bit build with less than two gigabytes of usable address space,
        /// already most of the way through it — and a mapping that succeeds and
        /// then starves the game of address space is worse than a refusal that
        /// names the file. The compiler's largest plausible output is well under
        /// this: eight full 4096-square atlas pages is 128 MB.
        /// </remarks>
        public const long AqsaHajm = 512L * 1024 * 1024;

        /// <summary>The content hash's length in bytes.</summary>
        public const int TulBasma = 32;

        /// <summary>An Ed25519 public key's length in bytes.</summary>
        public const int TulMiftahAam = 32;

        /// <summary>An Ed25519 signature's length in bytes.</summary>
        public const int TulTawqee = 64;

        /// <summary>The signature block's length in bytes.</summary>
        public const int TulKutlatTawqee = 128;

        /// <summary>The four magic bytes the signature block begins with, <c>TWQ1</c>.</summary>
        public const uint WasmTawqee = 0x3151_5754u;

        /// <summary>The one signature-block version this build reads.</summary>
        public const ushort IsdarKutla = 1;

        /// <summary>
        /// The length of the byte string the signature was made over.
        /// </summary>
        /// <remarks>
        /// Ninety-seven: a 23-byte domain separator, the 32-byte content hash,
        /// the role byte, the algorithm byte, the 8-byte timestamp, and the
        /// 32-byte public key. Every one of those is signed, and the role
        /// especially — without it in the message, a contributor's genuine
        /// self-signature could be relabelled as the owner's by flipping one
        /// byte of the block, and the signature would still verify.
        /// </remarks>
        public const int TulRisalatTawqee = 97;

        /// <summary>The stride of a <see cref="MadkhalNass"/>.</summary>
        public const int TulMadkhalNass = 16;

        /// <summary>The stride of a <see cref="MadkhalNitaq"/>.</summary>
        public const int TulMadkhalNitaq = 24;

        /// <summary>The stride of a <see cref="MadkhalTakhtit"/>.</summary>
        public const int TulMadkhalTakhtit = 32;

        /// <summary>The stride of a <see cref="MadkhalQayd"/>.</summary>
        public const int TulMadkhalQayd = 48;

        /// <summary>The stride of a <see cref="MadkhalSafha"/>.</summary>
        public const int TulMadkhalSafha = 16;

        /// <summary>The stride of a <see cref="MadkhalKhatt"/>.</summary>
        public const int TulMadkhalKhatt = 16;

        /// <summary>The stride of a <see cref="TaaribMiftahShakl"/>.</summary>
        public const int TulMiftahShakl = 8;

        /// <summary>The stride of a <see cref="TaaribMawdiShakl"/>.</summary>
        public const int TulMawdiShakl = 20;

        /// <summary>The stride of a <see cref="TaaribHarf"/>.</summary>
        public const int TulHarf = 24;

        /// <summary>The stride of a <see cref="TaaribSatr"/>.</summary>
        public const int TulSatr = 48;

        /// <summary>The length of the preamble a single-array section opens with.</summary>
        public const int TulTasdir = 16;

        /// <summary>The length of the preamble a multi-array section opens with.</summary>
        public const int TulTasdirKabir = 32;

        private const int AqsaNaw = 8;

        private static readonly byte[] NitaqTawqee =
            Encoding.ASCII.GetBytes("taarib-ruqaa/tawqee/v1\0");

        // Nullable because they genuinely are null when construction failed
        // part-way, which is exactly the state Atlif has to cope with. Declaring
        // them non-nullable and letting Atlif read them anyway would be telling
        // the compiler something untrue about the one path where it matters.
        private readonly FileStream? milaff;
        private readonly MemoryMappedFile? khareetatMilaff;
        private readonly MemoryMappedViewAccessor? manzar;
        private readonly SafeMemoryMappedViewHandle? miqbad;
        private readonly long[] izahatAqsam = new long[AqsaNaw + 1];
        private readonly long[] tulAqsam = new long[AqsaNaw + 1];
        private byte* asas;
        private bool mughlaq;

        private int adadNusus;
        private long izahatNusus;
        private int adadNitaqat;
        private long izahatNitaqat;
        private long izahatHawdNusus;
        private int tulHawdNusus;

        private int adadTakhtitat;
        private long izahatTakhtitat;
        private int adadHuruf;
        private long izahatHuruf;
        private int adadSutur;
        private long izahatSutur;

        private int adadAshkal;
        private long izahatMafatih;
        private long izahatMawadi;

        private int adadQiyud;
        private long izahatQiyud;

        private int adadSafahat;
        private long izahatSafahat;

        private int adadKhutut;
        private long izahatKhutut;
        private long izahatBasmatKhutut;
        private long izahatHawdKhutut;
        private int tulHawdKhutut;

        private readonly byte[] basmaMuallana = new byte[TulBasma];
        private readonly byte[] miftahMuwaqqi = new byte[TulMiftahAam];
        private readonly byte[] tawqeeKhaam = new byte[TulTawqee];

        private Ruqaa(string masar, ReadOnlySpan<byte> miftahMutawaqqa)
        {
            if (masar is null)
            {
                throw new ArgumentNullException(nameof(masar));
            }
            if (masar.Length == 0)
            {
                throw new ArgumentException("A patch path is not empty.", nameof(masar));
            }
            if (!miftahMutawaqqa.IsEmpty && miftahMutawaqqa.Length != TulMiftahAam)
            {
                throw new ArgumentException(
                    "An Ed25519 public key is exactly 32 bytes.", nameof(miftahMutawaqqa));
            }
            TahaqqaqMinAlBina();

            Masar = masar;
            for (int i = 0; i <= AqsaNaw; i++)
            {
                tulAqsam[i] = -1;
            }

            FileStream tayyar = new FileStream(
                masar, FileMode.Open, FileAccess.Read,
                FileShare.ReadWrite | FileShare.Delete, 1, FileOptions.None);
            milaff = tayyar;
            try
            {
                Tul = tayyar.Length;
                if (Tul < TulRas)
                {
                    throw Rafd(
                        6003, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"ملف الرقعة أقصر من ترويسته ({Tul} بايت)؛ التثبيت ناقص.",
                        $"The patch file is shorter than its own header ({Tul} bytes); the "
                        + "installation is incomplete.");
                }
                if (Tul > AqsaHajm)
                {
                    throw Rafd(
                        6003, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"ملف الرقعة أكبر من الحد المسموح ({Tul} بايت مقابل {AqsaHajm})؛ رُفض "
                        + "قبل تعيينه في الذاكرة.",
                        $"The patch file is larger than the permitted maximum ({Tul} bytes "
                        + $"against {AqsaHajm}); it was refused before being mapped.");
                }

                MemoryMappedFile khareeta = MemoryMappedFile.CreateFromFile(
                    tayyar, null, 0, MemoryMappedFileAccess.Read,
                    HandleInheritability.None, leaveOpen: true);
                khareetatMilaff = khareeta;
                MemoryMappedViewAccessor nafidha =
                    khareeta.CreateViewAccessor(0, Tul, MemoryMappedFileAccess.Read);
                manzar = nafidha;
                SafeMemoryMappedViewHandle maqbad = nafidha.SafeMemoryMappedViewHandle;
                miqbad = maqbad;
                byte* muashir = null;
                maqbad.AcquirePointer(ref muashir);
                if (muashir == null)
                {
                    throw Rafd(
                        6003, Ramz.KhataAam, Khutwa.IadatTarkibIttar,
                        "تعذّر تعيين ملف الرقعة في الذاكرة.",
                        "The patch file could not be mapped into memory.");
                }
                asas = muashir + nafidha.PointerOffset;

                IqraRas();
                IqraJadwalAqsam();
                TahaqqaqMinBasma();
                IqraTawqee(miftahMutawaqqa);
                HallJadawil();
            }
            catch
            {
                Atlif();
                throw;
            }
        }

        /// <summary>
        /// Opens, maps and validates an installed container, verifying its
        /// content hash before any of its body is read.
        /// </summary>
        /// <param name="masar">The absolute path of the <c>.ruqaa</c> file.</param>
        /// <returns>The open container. Dispose it when the plugin unloads.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="masar"/> is null.</exception>
        /// <exception cref="ArgumentException"><paramref name="masar"/> is empty.</exception>
        /// <exception cref="IOException">The file could not be opened or mapped.</exception>
        /// <exception cref="KhataTaarib">
        /// The container was refused. The exception names the field that failed
        /// and carries a permanent code from the <c>6000</c> block, so the same
        /// refusal reads identically in a BepInEx log and in Studio.
        /// </exception>
        /// <remarks>
        /// Content-hash verification is not optional and there is no overload
        /// that skips it. Decision 8 says a client refuses anything unsigned,
        /// mismatched or revoked, and a verification that a configuration file
        /// can turn off is a verification that will be off on the machine where
        /// it mattered.
        /// </remarks>
        public static Ruqaa Iftah(string masar)
        {
            return new Ruqaa(masar, ReadOnlySpan<byte>.Empty);
        }

        /// <summary>
        /// As <see cref="Iftah(string)"/>, and additionally binds the container
        /// to an expected signer.
        /// </summary>
        /// <param name="masar">The absolute path of the <c>.ruqaa</c> file.</param>
        /// <param name="miftahMutawaqqa">
        /// The 32-byte Ed25519 public key the installer recorded for this patch,
        /// or empty to skip the binding.
        /// </param>
        /// <returns>The open container.</returns>
        /// <exception cref="ArgumentException">
        /// <paramref name="masar"/> is empty, or the key is neither empty nor 32
        /// bytes.
        /// </exception>
        /// <exception cref="KhataTaarib">
        /// The container was refused, including because its signature block
        /// names a different key.
        /// </exception>
        /// <remarks>
        /// This is a binding, not a signature check, and the distinction is the
        /// whole of what this assembly can honestly claim. Comparing the key
        /// proves that the container in the plugin folder was signed by the same
        /// party the installer approved; it proves nothing about whether that
        /// party actually signed <em>these</em> bytes. What makes the pair
        /// meaningful is the content hash: the installer verified a signature
        /// over the hash, this reader verifies the hash over the bytes, and a
        /// container swapped for one signed by another key is caught here rather
        /// than drawn from.
        /// </remarks>
        public static Ruqaa Iftah(string masar, ReadOnlySpan<byte> miftahMutawaqqa)
        {
            return new Ruqaa(masar, miftahMutawaqqa);
        }

        /// <summary>
        /// The key the string table is sorted and searched by: the first eight
        /// bytes of BLAKE3 over a source string's UTF-8, read little-endian.
        /// </summary>
        /// <param name="nass">The source string, exactly as the game had it.</param>
        /// <returns>Its key.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="nass"/> is null.</exception>
        /// <remarks>
        /// Defined here rather than left to each caller because five languages
        /// compute this and a key computed two ways is a patch half the
        /// consumers cannot search. Note that it hashes the bytes it is given
        /// with no normalization of any kind: the compiler hashed the game's
        /// exact bytes, so this must too.
        /// </remarks>
        public static ulong MiftahMinNass(string nass)
        {
            if (nass is null)
            {
                throw new ArgumentNullException(nameof(nass));
            }
            int tul = Encoding.UTF8.GetByteCount(nass);
            if (tul <= 512)
            {
                Span<byte> mu = stackalloc byte[tul];
                Encoding.UTF8.GetBytes(nass, mu);
                return MiftahMinNass(mu);
            }
            return MiftahMinNass(Encoding.UTF8.GetBytes(nass));
        }

        /// <summary>
        /// As <see cref="MiftahMinNass(string)"/>, for a caller that already has
        /// the UTF-8 bytes and would rather not encode them again.
        /// </summary>
        /// <param name="nass">The source string's UTF-8 bytes.</param>
        /// <returns>Its key.</returns>
        public static ulong MiftahMinNass(ReadOnlySpan<byte> nass)
        {
            return Basma.Ihsib64(nass);
        }

        /// <summary>
        /// Whether a file's bytes hash to a content hash the container recorded.
        /// </summary>
        /// <param name="bayt">The file's bytes.</param>
        /// <param name="mutawaqqa">The 32-byte hash to check against.</param>
        /// <returns>Whether they match.</returns>
        /// <remarks>
        /// Exposed because the adapter assemblies have to check the font files
        /// the container names before shaping with them, and the BLAKE3
        /// implementation this needs is internal to this assembly by design —
        /// there is exactly one copy of it in the solution and it is not a thing
        /// callers should be reimplementing. The comparison is constant-time,
        /// which matters less here than it does for a signature and costs
        /// nothing.
        /// </remarks>
        public static bool TahaqqaqBasma(
            ReadOnlySpan<byte> bayt, ReadOnlySpan<byte> mutawaqqa)
        {
            if (mutawaqqa.Length != TulBasma)
            {
                return false;
            }
            Span<byte> mahsuba = stackalloc byte[TulBasma];
            Basma.Ihsib(bayt, mahsuba);
            return Basma.Yutabiq(mahsuba, mutawaqqa);
        }

        /// <summary>The path this container was opened from.</summary>
        public string Masar { get; }

        /// <summary>The file's length in bytes.</summary>
        public long Tul { get; }

        /// <summary>The container's format version.</summary>
        public ushort IsdarSigha { get; private set; }

        /// <summary>The header's flag word, per <see cref="AlamatRuqaa"/>.</summary>
        public ushort Alam { get; private set; }

        /// <summary>
        /// How this patch's atlas was rasterized, taken from the header's flags.
        /// </summary>
        /// <remarks>
        /// A container that declares both modes at once is refused rather than
        /// resolved to one of them: it would draw with the wrong shader and
        /// produce text that is legible in screenshots and wrong in motion.
        /// </remarks>
        public NamatLawha Namat { get; private set; }

        /// <summary>Whether the patch carries right-to-left interface mirroring hints.</summary>
        public bool MiraatYameen => (Alam & AlamatRuqaa.MiraatYameen) != 0;

        /// <summary>Whether the patch carries hints from a runtime capture session.</summary>
        public bool TalmihatIltiqat => (Alam & AlamatRuqaa.TalmihatIltiqat) != 0;

        /// <summary>How far verification of the signature block got in this process.</summary>
        public HalatTawqee HalatTawqee { get; private set; }

        /// <summary>The role of the key that sealed this container.</summary>
        public DawrMiftah DawrMuwaqqi { get; private set; }

        /// <summary>Which scheme sealed it, or <see cref="Khwarizmiya.Ghayr"/> if none.</summary>
        public Khwarizmiya KhwarizmiyatTawqee { get; private set; }

        /// <summary>The signing time, in seconds since the Unix epoch.</summary>
        public long WaqtTawqee { get; private set; }

        /// <summary>The content hash the header records and this reader verified.</summary>
        public ReadOnlySpan<byte> BasmaMuhtawa => basmaMuallana;

        /// <summary>The public key the signature block names.</summary>
        public ReadOnlySpan<byte> MiftahMuwaqqi => miftahMuwaqqi;

        /// <summary>The signature itself, unverified by this assembly.</summary>
        public ReadOnlySpan<byte> Tawqee => tawqeeKhaam;

        /// <summary>Whether a client may install a patch sealed under this role.</summary>
        public bool YusmahBilTathbeet =>
            KhwarizmiyatTawqee == Khwarizmiya.Ed25519 && DawrMuwaqqi == DawrMiftah.Malik;

        /// <summary>Whether the container carries a section of this kind.</summary>
        /// <param name="naw">The kind.</param>
        /// <returns>Whether it is present.</returns>
        public bool Yahwi(NawQism naw)
        {
            int raqm = (int)naw;
            return raqm >= 1 && raqm <= AqsaNaw && tulAqsam[raqm] >= 0;
        }

        /// <summary>Where a section sits in the mapped file.</summary>
        /// <param name="naw">The kind.</param>
        /// <returns>Its position, with a negative length when it is absent.</returns>
        public MawdiQism Mawdi(NawQism naw)
        {
            int raqm = (int)naw;
            if (raqm < 1 || raqm > AqsaNaw)
            {
                return new MawdiQism(naw, 0, -1);
            }
            return new MawdiQism(naw, izahatAqsam[raqm], tulAqsam[raqm]);
        }

        /// <summary>A section's bytes, borrowed from the mapping.</summary>
        /// <param name="naw">The kind.</param>
        /// <returns>Its bytes, empty when the section is absent.</returns>
        public ReadOnlySpan<byte> Qism(NawQism naw)
        {
            int raqm = (int)naw;
            if (raqm < 1 || raqm > AqsaNaw || tulAqsam[raqm] < 0)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            return Bayt(izahatAqsam[raqm], tulAqsam[raqm]);
        }

        /// <summary>The string table, sorted ascending by key.</summary>
        public ReadOnlySpan<MadkhalNass> Nusus =>
            MemoryMarshal.Cast<byte, MadkhalNass>(
                Bayt(izahatNusus, (long)adadNusus * TulMadkhalNass));

        /// <summary>Every style span, sorted ascending by the string it belongs to.</summary>
        public ReadOnlySpan<MadkhalNitaq> Nitaqat =>
            MemoryMarshal.Cast<byte, MadkhalNitaq>(
                Bayt(izahatNitaqat, (long)adadNitaqat * TulMadkhalNitaq));

        /// <summary>The UTF-8 pool the translations live in.</summary>
        public ReadOnlySpan<byte> HawdNusus => Bayt(izahatHawdNusus, tulHawdNusus);

        /// <summary>The layout heads, sorted ascending by string and then by size.</summary>
        public ReadOnlySpan<MadkhalTakhtit> Takhtitat =>
            MemoryMarshal.Cast<byte, MadkhalTakhtit>(
                Bayt(izahatTakhtitat, (long)adadTakhtitat * TulMadkhalTakhtit));

        /// <summary>Every glyph of every precomputed layout, in head order.</summary>
        public ReadOnlySpan<TaaribHarf> Huruf =>
            MemoryMarshal.Cast<byte, TaaribHarf>(Bayt(izahatHuruf, (long)adadHuruf * TulHarf));

        /// <summary>Every line of every precomputed layout, in head order.</summary>
        public ReadOnlySpan<TaaribSatr> Sutur =>
            MemoryMarshal.Cast<byte, TaaribSatr>(Bayt(izahatSutur, (long)adadSutur * TulSatr));

        /// <summary>The glyph map's keys, sorted ascending. Searched.</summary>
        public ReadOnlySpan<TaaribMiftahShakl> MafatihAshkal =>
            MemoryMarshal.Cast<byte, TaaribMiftahShakl>(
                Bayt(izahatMafatih, (long)adadAshkal * TulMiftahShakl));

        /// <summary>The glyph map's positions. Position <c>n</c> belongs to key <c>n</c>.</summary>
        public ReadOnlySpan<TaaribMawdiShakl> MawadiAshkal =>
            MemoryMarshal.Cast<byte, TaaribMawdiShakl>(
                Bayt(izahatMawadi, (long)adadAshkal * TulMawdiShakl));

        /// <summary>The constraints, sorted ascending by string index.</summary>
        public ReadOnlySpan<MadkhalQayd> Qiyud =>
            MemoryMarshal.Cast<byte, MadkhalQayd>(
                Bayt(izahatQiyud, (long)adadQiyud * TulMadkhalQayd));

        /// <summary>The atlas page table.</summary>
        public ReadOnlySpan<MadkhalSafha> Safahat =>
            MemoryMarshal.Cast<byte, MadkhalSafha>(
                Bayt(izahatSafahat, (long)adadSafahat * TulMadkhalSafha));

        /// <summary>The font chain's records, in chain order.</summary>
        public ReadOnlySpan<MadkhalKhatt> Khutut =>
            MemoryMarshal.Cast<byte, MadkhalKhatt>(
                Bayt(izahatKhutut, (long)adadKhutut * TulMadkhalKhatt));

        /// <summary>
        /// Finds a string by the key of its source text.
        /// </summary>
        /// <param name="miftah">The key, from <see cref="MiftahMinNass(string)"/>.</param>
        /// <returns>Its index in the string table, or <c>-1</c> when there is none.</returns>
        /// <remarks>
        /// A binary search over the mapped key column. The index is the return
        /// value rather than the record, because the index is what the layout
        /// and constraint tables are keyed by and every caller that found a
        /// string then wants one of those.
        /// </remarks>
        public int JidNass(ulong miftah)
        {
            ReadOnlySpan<MadkhalNass> jadwal = Nusus;
            int adna = 0;
            int aqsa = jadwal.Length - 1;
            while (adna <= aqsa)
            {
                int wasat = adna + ((aqsa - adna) >> 1);
                ulong mawjud = jadwal[wasat].Miftah;
                if (mawjud == miftah)
                {
                    return wasat;
                }
                if (mawjud < miftah)
                {
                    adna = wasat + 1;
                }
                else
                {
                    aqsa = wasat - 1;
                }
            }
            return -1;
        }

        /// <summary>The translated text of one string, as UTF-8 bytes.</summary>
        /// <param name="fahras">Its index, from <see cref="JidNass"/>.</param>
        /// <returns>Its bytes, empty when the index is out of range.</returns>
        /// <remarks>
        /// Bytes rather than a <see cref="string"/>, because a string would be
        /// an allocation on a path a game runs while a frame is waiting, and
        /// because the layout engine wants UTF-8 anyway. Use
        /// <see cref="NassMansi"/> when a managed string is genuinely needed.
        /// </remarks>
        public ReadOnlySpan<byte> Tarjama(int fahras)
        {
            ReadOnlySpan<MadkhalNass> jadwal = Nusus;
            if ((uint)fahras >= (uint)jadwal.Length)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            MadkhalNass madkhal = jadwal[fahras];
            ReadOnlySpan<byte> hawd = HawdNusus;
            if (madkhal.Izaha > (uint)hawd.Length
                || madkhal.Tul > (uint)hawd.Length - madkhal.Izaha)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            return hawd.Slice((int)madkhal.Izaha, (int)madkhal.Tul);
        }

        /// <summary>The translated text of one string, as a managed string.</summary>
        /// <param name="fahras">Its index, from <see cref="JidNass"/>.</param>
        /// <returns>The text, or an empty string when the index is out of range.</returns>
        /// <remarks>
        /// This allocates. It exists for the takeover points that must hand a
        /// <see cref="string"/> back to the game's own component — an input
        /// field's value, a tooltip the game will re-parse — and it should not
        /// be called from a path that runs every frame.
        /// </remarks>
        public string NassMansi(int fahras)
        {
            ReadOnlySpan<byte> bayt = Tarjama(fahras);
            return bayt.IsEmpty ? string.Empty : Encoding.UTF8.GetString(bayt);
        }

        /// <summary>The style spans belonging to one string.</summary>
        /// <param name="fahras">Its index, from <see cref="JidNass"/>.</param>
        /// <returns>Its spans, as a contiguous run of the span array.</returns>
        /// <remarks>
        /// Two partition points rather than a scan, because the array is sorted
        /// by string index and a string's spans are therefore contiguous.
        /// </remarks>
        public ReadOnlySpan<MadkhalNitaq> NitaqatNass(int fahras)
        {
            if (fahras < 0)
            {
                return ReadOnlySpan<MadkhalNitaq>.Empty;
            }
            uint nass = (uint)fahras;
            ReadOnlySpan<MadkhalNitaq> jadwal = Nitaqat;
            int awwal = HadduAdna(jadwal, nass);
            if (awwal >= jadwal.Length || jadwal[awwal].Nass != nass)
            {
                return ReadOnlySpan<MadkhalNitaq>.Empty;
            }
            int akhir = awwal;
            while (akhir < jadwal.Length && jadwal[akhir].Nass == nass)
            {
                akhir++;
            }
            return jadwal.Slice(awwal, akhir - awwal);
        }

        /// <summary>
        /// The layout the compiler produced for one string at one size.
        /// </summary>
        /// <param name="fahras">The string's index, from <see cref="JidNass"/>.</param>
        /// <param name="hajmRubi">The size in quarter-pixels.</param>
        /// <param name="madkhal">The layout, when one exists.</param>
        /// <returns>Whether one exists.</returns>
        /// <remarks>
        /// The exact size, not the nearest. Drawing a layout measured at one
        /// size into a box the game sizes at another is how text that fitted in
        /// the compiler's measurement overflows on a player's screen — and the
        /// compiler's overflow report, which said it fitted, would be wrong.
        /// Use <see cref="TakhtitatNass"/> when a caller is willing to decide
        /// for itself that a nearby size is acceptable.
        /// </remarks>
        public bool JidTakhtit(int fahras, ushort hajmRubi, out MadkhalTakhtit madkhal)
        {
            madkhal = default;
            if (fahras < 0)
            {
                return false;
            }
            ulong matlub = ((ulong)(uint)fahras << 16) | hajmRubi;
            ReadOnlySpan<MadkhalTakhtit> jadwal = Takhtitat;
            int adna = 0;
            int aqsa = jadwal.Length - 1;
            while (adna <= aqsa)
            {
                int wasat = adna + ((aqsa - adna) >> 1);
                MadkhalTakhtit hali = jadwal[wasat];
                ulong mawjud = ((ulong)hali.Nass << 16) | hali.HajmRubi;
                if (mawjud == matlub)
                {
                    madkhal = hali;
                    return true;
                }
                if (mawjud < matlub)
                {
                    adna = wasat + 1;
                }
                else
                {
                    aqsa = wasat - 1;
                }
            }
            return false;
        }

        /// <summary>Every layout the compiler produced for one string, at whatever sizes.</summary>
        /// <param name="fahras">The string's index, from <see cref="JidNass"/>.</param>
        /// <returns>Its layouts, a contiguous run of the array, ascending by size.</returns>
        public ReadOnlySpan<MadkhalTakhtit> TakhtitatNass(int fahras)
        {
            if (fahras < 0)
            {
                return ReadOnlySpan<MadkhalTakhtit>.Empty;
            }
            uint nass = (uint)fahras;
            ReadOnlySpan<MadkhalTakhtit> jadwal = Takhtitat;
            int awwal = 0;
            int aqsa = jadwal.Length;
            while (awwal < aqsa)
            {
                int wasat = awwal + ((aqsa - awwal) >> 1);
                if (jadwal[wasat].Nass < nass)
                {
                    awwal = wasat + 1;
                }
                else
                {
                    aqsa = wasat;
                }
            }
            int akhir = awwal;
            while (akhir < jadwal.Length && jadwal[akhir].Nass == nass)
            {
                akhir++;
            }
            return jadwal.Slice(awwal, akhir - awwal);
        }

        /// <summary>The glyphs of one precomputed layout.</summary>
        /// <param name="madkhal">The layout head.</param>
        /// <returns>Its glyphs, in visual order, borrowed from the mapping.</returns>
        /// <remarks>
        /// <c>scoped</c>: the returned span borrows the mapping, never the
        /// entry, so a caller may pass a local without the ref-safety rules
        /// clamping the result's lifetime to that local's scope.
        /// </remarks>
        public ReadOnlySpan<TaaribHarf> HurufTakhtit(scoped in MadkhalTakhtit madkhal)
        {
            ReadOnlySpan<TaaribHarf> kull = Huruf;
            if (madkhal.AwwalHarf > (uint)kull.Length
                || madkhal.AdadHuruf > (uint)kull.Length - madkhal.AwwalHarf)
            {
                return ReadOnlySpan<TaaribHarf>.Empty;
            }
            return kull.Slice((int)madkhal.AwwalHarf, (int)madkhal.AdadHuruf);
        }

        /// <summary>The lines of one precomputed layout.</summary>
        /// <param name="madkhal">The layout head.</param>
        /// <returns>Its lines, borrowed from the mapping.</returns>
        /// <remarks>
        /// <c>scoped</c> for the same reason as <see cref="HurufTakhtit"/>.
        /// </remarks>
        public ReadOnlySpan<TaaribSatr> SuturTakhtit(scoped in MadkhalTakhtit madkhal)
        {
            ReadOnlySpan<TaaribSatr> kull = Sutur;
            if (madkhal.AwwalSatr > (uint)kull.Length
                || madkhal.AdadSutur > (uint)kull.Length - madkhal.AwwalSatr)
            {
                return ReadOnlySpan<TaaribSatr>.Empty;
            }
            return kull.Slice((int)madkhal.AwwalSatr, (int)madkhal.AdadSutur);
        }

        /// <summary>Finds one glyph image in the atlas.</summary>
        /// <param name="miftah">The glyph, size, font and subpixel bucket.</param>
        /// <param name="mawdi">Where it sits, when the patch carries it.</param>
        /// <returns>Whether the patch carries it.</returns>
        /// <remarks>
        /// A binary search over the packed key column, which is why the keys are
        /// a separate array from the positions: eight-byte keys put seven of
        /// them in a cache line, and interleaving them with their twenty-byte
        /// positions would put two.
        /// </remarks>
        public bool JidShakl(TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi)
        {
            mawdi = default;
            ulong matlub = RaqmMiftah(miftah);
            ReadOnlySpan<TaaribMiftahShakl> mafatih = MafatihAshkal;
            int adna = 0;
            int aqsa = mafatih.Length - 1;
            while (adna <= aqsa)
            {
                int wasat = adna + ((aqsa - adna) >> 1);
                ulong mawjud = RaqmMiftah(mafatih[wasat]);
                if (mawjud == matlub)
                {
                    ReadOnlySpan<TaaribMawdiShakl> mawadi = MawadiAshkal;
                    if (wasat >= mawadi.Length)
                    {
                        return false;
                    }
                    mawdi = mawadi[wasat];
                    return true;
                }
                if (mawjud < matlub)
                {
                    adna = wasat + 1;
                }
                else
                {
                    aqsa = wasat - 1;
                }
            }
            return false;
        }

        /// <summary>The constraint the compiler recorded for one string.</summary>
        /// <param name="fahras">The string's index, from <see cref="JidNass"/>.</param>
        /// <param name="qayd">The constraint, when one was recorded.</param>
        /// <returns>Whether one was recorded.</returns>
        public bool JidQayd(int fahras, out MadkhalQayd qayd)
        {
            qayd = default;
            if (fahras < 0)
            {
                return false;
            }
            uint nass = (uint)fahras;
            ReadOnlySpan<MadkhalQayd> jadwal = Qiyud;
            int adna = 0;
            int aqsa = jadwal.Length - 1;
            while (adna <= aqsa)
            {
                int wasat = adna + ((aqsa - adna) >> 1);
                uint mawjud = jadwal[wasat].Nass;
                if (mawjud == nass)
                {
                    qayd = jadwal[wasat];
                    return true;
                }
                if (mawjud < nass)
                {
                    adna = wasat + 1;
                }
                else
                {
                    aqsa = wasat - 1;
                }
            }
            return false;
        }

        /// <summary>One atlas page's texels.</summary>
        /// <param name="fahras">The page index.</param>
        /// <returns>
        /// Its texels: one byte each, row-major from the top, no row padding.
        /// Empty when the index is out of range.
        /// </returns>
        public ReadOnlySpan<byte> TexelatSafha(int fahras)
        {
            ReadOnlySpan<MadkhalSafha> jadwal = Safahat;
            if ((uint)fahras >= (uint)jadwal.Length)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            MadkhalSafha safha = jadwal[fahras];
            long tulQism = tulAqsam[(int)NawQism.Lawha];
            if (safha.Izaha > (ulong)tulQism || safha.Tul > (ulong)tulQism - safha.Izaha)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            return Bayt(izahatAqsam[(int)NawQism.Lawha] + safha.Izaha, safha.Tul);
        }

        /// <summary>One font's file name, as a managed string.</summary>
        /// <param name="madkhal">The font record.</param>
        /// <returns>The name, or an empty string when the reference is out of range.</returns>
        /// <remarks>
        /// This allocates, and that is acceptable: font names are read once when
        /// the chain is loaded, not per frame.
        /// </remarks>
        public string IsmKhatt(in MadkhalKhatt madkhal)
        {
            ReadOnlySpan<byte> hawd = Bayt(izahatHawdKhutut, tulHawdKhutut);
            if (madkhal.IzahatIsm > (uint)hawd.Length
                || madkhal.TulIsm > (uint)hawd.Length - madkhal.IzahatIsm)
            {
                return string.Empty;
            }
            ReadOnlySpan<byte> ism = hawd.Slice((int)madkhal.IzahatIsm, (int)madkhal.TulIsm);
            return ism.IsEmpty ? string.Empty : Encoding.UTF8.GetString(ism);
        }

        /// <summary>One font's expected content hash.</summary>
        /// <param name="fahras">The font's position in the chain.</param>
        /// <returns>Its 32-byte BLAKE3 hash, empty when the index is out of range.</returns>
        /// <remarks>
        /// What the adapter checks the file on disk against before shaping with
        /// it, so a font replaced after installation is caught rather than used.
        /// </remarks>
        public ReadOnlySpan<byte> BasmaKhatt(int fahras)
        {
            if ((uint)fahras >= (uint)adadKhutut)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            return Bayt(izahatBasmatKhutut + ((long)fahras * TulBasma), TulBasma);
        }

        /// <summary>
        /// Composes the exact ninety-seven bytes the signature was made over.
        /// </summary>
        /// <param name="hadaf">A buffer of at least <see cref="TulRisalatTawqee"/> bytes.</param>
        /// <returns>How many bytes were written.</returns>
        /// <exception cref="ArgumentException">The buffer is too small.</exception>
        /// <remarks>
        /// This assembly cannot verify the signature, but it can say precisely
        /// what would have to be verified — so a caller that does have Ed25519
        /// (Studio, the installer, a diagnostics tool) can check this container
        /// without re-deriving the message format and getting one field's order
        /// wrong. Five implementations construct these bytes and they must agree
        /// exactly; this is the C# one.
        /// </remarks>
        public int KtubRisalatTawqee(Span<byte> hadaf)
        {
            if (hadaf.Length < TulRisalatTawqee)
            {
                throw new ArgumentException(
                    $"The signed message is {TulRisalatTawqee} bytes.", nameof(hadaf));
            }
            hadaf = hadaf.Slice(0, TulRisalatTawqee);
            hadaf.Clear();
            NitaqTawqee.AsSpan().CopyTo(hadaf);
            basmaMuallana.AsSpan().CopyTo(hadaf.Slice(23, TulBasma));
            hadaf[55] = (byte)DawrMuwaqqi;
            hadaf[56] = (byte)KhwarizmiyatTawqee;
            BinaryPrimitives.WriteInt64LittleEndian(hadaf.Slice(57, 8), WaqtTawqee);
            miftahMuwaqqi.AsSpan().CopyTo(hadaf.Slice(65, TulMiftahAam));
            return TulRisalatTawqee;
        }

        /// <summary>Releases the mapping and the file handle.</summary>
        /// <remarks>
        /// Every span this type ever handed out points into the mapping, so a
        /// span outliving the container is a read of unmapped memory. Dispose
        /// when the plugin unloads, not between scenes, and never while a
        /// takeover point might still be drawing.
        /// </remarks>
        public void Dispose()
        {
            Atlif();
            GC.SuppressFinalize(this);
        }

        /// <summary>Releases the mapping if <see cref="Dispose"/> never ran.</summary>
        ~Ruqaa()
        {
            Atlif();
        }

        private static ulong RaqmMiftah(TaaribMiftahShakl miftah)
        {
            return miftah.Muarrif
                | ((ulong)miftah.HajmRubi << 32)
                | ((ulong)miftah.Khatt << 48)
                | ((ulong)miftah.Bakat << 56);
        }

        private static int HadduAdna(ReadOnlySpan<MadkhalNitaq> jadwal, uint nass)
        {
            int adna = 0;
            int aqsa = jadwal.Length;
            while (adna < aqsa)
            {
                int wasat = adna + ((aqsa - adna) >> 1);
                if (jadwal[wasat].Nass < nass)
                {
                    adna = wasat + 1;
                }
                else
                {
                    aqsa = wasat;
                }
            }
            return adna;
        }

        private static KhataTaarib Rafd(
            int raqm, int halat, Khutwa khutwa, string arabi, string injilizi)
        {
            return new KhataTaarib(halat, $"TAARIB-E-{raqm}", arabi, injilizi, khutwa);
        }

        /// <summary>
        /// Refuses at load if this build's structs are not the sizes the format
        /// froze.
        /// </summary>
        /// <remarks>
        /// A binding whose struct is a byte larger or smaller than the writer's
        /// produces plausible garbage — every field shifted by one record — and
        /// nothing about the output says so. Checking <c>sizeof</c> against the
        /// documented stride turns that into a refusal at load, on the machine
        /// where it is wrong, rather than a rendering bug nobody can reproduce.
        /// </remarks>
        private static void TahaqqaqMinAlBina()
        {
            LazimHajm(sizeof(MadkhalNass), TulMadkhalNass, nameof(MadkhalNass));
            LazimHajm(sizeof(MadkhalNitaq), TulMadkhalNitaq, nameof(MadkhalNitaq));
            LazimHajm(sizeof(MadkhalTakhtit), TulMadkhalTakhtit, nameof(MadkhalTakhtit));
            LazimHajm(sizeof(MadkhalQayd), TulMadkhalQayd, nameof(MadkhalQayd));
            LazimHajm(sizeof(MadkhalSafha), TulMadkhalSafha, nameof(MadkhalSafha));
            LazimHajm(sizeof(MadkhalKhatt), TulMadkhalKhatt, nameof(MadkhalKhatt));
            LazimHajm(sizeof(TaaribMiftahShakl), TulMiftahShakl, nameof(TaaribMiftahShakl));
            LazimHajm(sizeof(TaaribMawdiShakl), TulMawdiShakl, nameof(TaaribMawdiShakl));
            LazimHajm(sizeof(TaaribHarf), TulHarf, nameof(TaaribHarf));
            LazimHajm(sizeof(TaaribSatr), TulSatr, nameof(TaaribSatr));
        }

        private static void LazimHajm(int fili, int madum, string ism)
        {
            if (fili == madum)
            {
                return;
            }
            throw Rafd(
                6015, Ramz.IsdarGhayrMutawafiq, Khutwa.TahdithTaarib,
                $"بنية {ism} في هذا البناء {fili} بايت والصيغة تثبّتها عند {madum}؛ لا يمكن "
                + "قراءة الرقعة بأمان.",
                $"The {ism} struct is {fili} bytes in this build and the format fixes it at "
                + $"{madum}; the patch cannot be read safely.");
        }

        private ReadOnlySpan<byte> Bayt(long izaha, long tul)
        {
            if (mughlaq || asas == null)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            if (izaha < 0 || tul < 0 || izaha > Tul || tul > Tul - izaha || tul > int.MaxValue)
            {
                return ReadOnlySpan<byte>.Empty;
            }
            return new ReadOnlySpan<byte>(asas + izaha, (int)tul);
        }

        private void IqraRas()
        {
            ReadOnlySpan<byte> ras = Bayt(0, TulRas);
            uint wasm = BinaryPrimitives.ReadUInt32LittleEndian(ras.Slice(0, 4));
            if (wasm != Wasm)
            {
                throw Rafd(
                    6001, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    "الملف الموجود بجوار الملحق ليس رقعة تعريب: التوقيع السحري في أوله لا يطابق "
                    + "\"TRQ1\".",
                    "The file beside the plugin is not a Taarib patch: the magic at its start "
                    + "is not \"TRQ1\".");
            }

            IsdarSigha = BinaryPrimitives.ReadUInt16LittleEndian(ras.Slice(4, 2));
            if (IsdarSigha != IsdarMafhum)
            {
                bool ahdath = IsdarSigha > IsdarMafhum;
                throw Rafd(
                    6002, Ramz.IsdarGhayrMutawafiq,
                    ahdath ? Khutwa.TahdithTaarib : Khutwa.IadatTarkibIttar,
                    $"إصدار صيغة الرقعة {IsdarSigha}، وهذا البناء يقرأ الإصدار {IsdarMafhum} "
                    + "فقط (format_version).",
                    $"The patch format version is {IsdarSigha}; this build reads version "
                    + $"{IsdarMafhum} only (format_version).");
            }

            Alam = BinaryPrimitives.ReadUInt16LittleEndian(ras.Slice(6, 2));
            if ((Alam & ~AlamatRuqaa.Marufa) != 0)
            {
                throw Rafd(
                    6016, Ramz.QeemaBatila, Khutwa.TahdithTaarib,
                    $"ترويسة الرقعة تحمل خصائص غير معروفة في هذا البناء (flags = {Alam}).",
                    $"The patch header carries flags this build does not define (flags = "
                    + $"{Alam}).");
            }
            bool misafa = (Alam & AlamatRuqaa.Misafa) != 0;
            bool taghtiya = (Alam & AlamatRuqaa.Taghtiya) != 0;
            if (misafa && taghtiya)
            {
                throw Rafd(
                    6016, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    "ترويسة الرقعة تعلن نمطي لوحة معًا: تغطية ومسافة (flags).",
                    "The patch header declares both atlas modes at once, coverage and signed "
                    + "distance field (flags).");
            }
            Namat = misafa ? NamatLawha.Misafa : NamatLawha.Taghtiya;

            ulong hajmKulli = BinaryPrimitives.ReadUInt64LittleEndian(ras.Slice(12, 8));
            if (hajmKulli != (ulong)Tul)
            {
                throw Rafd(
                    6003, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"ترويسة الرقعة تعلن طولًا {hajmKulli} بايت والملف على القرص {Tul} بايت "
                    + "(total_size)؛ الملف مبتور أو مُذيَّل.",
                    $"The patch header declares {hajmKulli} bytes and the file on disk is "
                    + $"{Tul} bytes (total_size); it is truncated or has been appended to.");
            }

            ras.Slice(20, TulBasma).CopyTo(basmaMuallana);
        }

        private void IqraJadwalAqsam()
        {
            uint adad = BinaryPrimitives.ReadUInt32LittleEndian(Bayt(8, 4));
            if (adad == 0 || adad > AqsaAqsam)
            {
                throw Rafd(
                    6004, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"عدد أقسام الرقعة {adad}، خارج المدى المقبول 1..{AqsaAqsam} "
                    + "(section_count).",
                    $"The patch declares {adad} sections, outside the accepted range "
                    + $"1..{AqsaAqsam} (section_count).");
            }

            long tulJadwal = (long)adad * TulMadkhalQism;
            if (tulJadwal > Tul - TulRas)
            {
                throw Rafd(
                    6005, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"جدول أقسام الرقعة ({adad} مدخلًا) لا يتسع في الملف (section_count).",
                    $"The patch section table ({adad} entries) does not fit in the file "
                    + "(section_count).");
            }
            long awwalMumkin = TulRas + tulJadwal;

            Span<long> bidayat = stackalloc long[AqsaAqsam];
            Span<long> nihayat = stackalloc long[AqsaAqsam];
            for (int i = 0; i < (int)adad; i++)
            {
                ReadOnlySpan<byte> m = Bayt(TulRas + ((long)i * TulMadkhalQism), TulMadkhalQism);
                uint naw = BinaryPrimitives.ReadUInt32LittleEndian(m.Slice(0, 4));
                ulong izaha = BinaryPrimitives.ReadUInt64LittleEndian(m.Slice(4, 8));
                ulong tulMukhazzan = BinaryPrimitives.ReadUInt64LittleEndian(m.Slice(12, 8));
                ulong tulKhaam = BinaryPrimitives.ReadUInt64LittleEndian(m.Slice(20, 8));
                uint daght = BinaryPrimitives.ReadUInt32LittleEndian(m.Slice(28, 4));

                if (daght != (uint)NamatDaght.Bila || tulMukhazzan != tulKhaam)
                {
                    throw Rafd(
                        6007, Ramz.GhayrMadum, Khutwa.IadatTarkibIttar,
                        $"القسم {naw} في الرقعة مضغوط (compression = {daght})، والملحق يقرأ "
                        + "النسخة العاملة غير المضغوطة التي يكتبها المثبِّت فقط.",
                        $"Section {naw} of the patch is compressed (compression = {daght}); "
                        + "the plugin reads only the uncompressed working copy the installer "
                        + "writes.");
                }
                if (izaha > (ulong)Tul || tulMukhazzan > (ulong)Tul - izaha)
                {
                    throw Rafd(
                        6005, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"القسم {naw} يعلن مدى خارج الملف: offset {izaha} وطول "
                        + $"{tulMukhazzan} في ملف طوله {Tul} بايت.",
                        $"Section {naw} declares a range outside the file: offset {izaha}, "
                        + $"length {tulMukhazzan}, in a file of {Tul} bytes.");
                }
                long bidaya = (long)izaha;
                long tulQism = (long)tulMukhazzan;
                if (bidaya < awwalMumkin || (bidaya % Muhadhat) != 0)
                {
                    throw Rafd(
                        6006, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"القسم {naw} يبدأ عند {bidaya}، وهو داخل الترويسة أو غير محاذٍ إلى "
                        + $"{Muhadhat} بايت (offset).",
                        $"Section {naw} starts at {bidaya}, which is inside the header or not "
                        + $"aligned to {Muhadhat} bytes (offset).");
                }

                bidayat[i] = bidaya;
                nihayat[i] = bidaya + tulQism;
                for (int j = 0; j < i; j++)
                {
                    if (bidaya < nihayat[j] && bidayat[j] < nihayat[i])
                    {
                        throw Rafd(
                            6006, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                            $"قسمان في الرقعة متداخلان في البايتات: المدخلان {j} و{i}.",
                            $"Two patch sections overlap in bytes: entries {j} and {i}.");
                    }
                }

                if (naw == (uint)NawQism.Tawqee
                    && (i != (int)adad - 1 || nihayat[i] != Tul))
                {
                    throw Rafd(
                        6011, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"كتلة التوقيع ليست آخر قسم في الملف (المدخل {i} من {adad}).",
                        $"The signature block is not the last section in the file (entry "
                        + $"{i} of {adad}).");
                }

                // A kind this build does not know is bounds-checked like every
                // other section and then ignored, so a container from a newer
                // compiler stays readable for everything it shares with this one.
                if (naw >= 1 && naw <= AqsaNaw)
                {
                    if (tulAqsam[naw] >= 0)
                    {
                        throw Rafd(
                            6006, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                            $"القسم {naw} معلن مرتين في جدول أقسام الرقعة (kind).",
                            $"Section kind {naw} is declared twice in the patch section table "
                            + "(kind).");
                    }
                    izahatAqsam[naw] = bidaya;
                    tulAqsam[naw] = tulQism;
                }
            }

            LazimQism(NawQism.Bayan, "BAYAN");
            LazimQism(NawQism.Nusus, "NUSUS");
            LazimQism(NawQism.Tawqee, "TAWQEE");
        }

        private void LazimQism(NawQism naw, string ism)
        {
            if (tulAqsam[(int)naw] < 0)
            {
                throw Rafd(
                    6009, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"الرقعة لا تحوي قسم {ism} وهو قسم لازم.",
                    $"The patch has no {ism} section, and that section is required.");
            }
        }

        private void TahaqqaqMinBasma()
        {
            // From the end of the header to the first byte of TAWQEE. Not to the
            // end of the file: the signature is over the hash, so a hash that
            // covered the signature could never have been computed, and hashing
            // to the end here would disagree with every patch the compiler
            // produces and reject all of them. See the file header.
            long nihayatMuhtawa = izahatAqsam[(int)NawQism.Tawqee];
            Span<byte> mahsuba = stackalloc byte[TulBasma];
            Basma.Ihsib(Bayt(TulRas, nihayatMuhtawa - TulRas), mahsuba);
            if (!Basma.Yutabiq(mahsuba, basmaMuallana))
            {
                throw Rafd(
                    6008, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    "بصمة محتوى الرقعة لا تطابق بايتاتها على القرص (content_hash)؛ تغيّر الملف "
                    + "بعد تثبيته.",
                    "The patch content hash does not match its bytes on disk (content_hash); "
                    + "the file changed after it was installed.");
            }
        }

        private void IqraTawqee(ReadOnlySpan<byte> miftahMutawaqqa)
        {
            long tulKutla = tulAqsam[(int)NawQism.Tawqee];
            if (tulKutla != TulKutlatTawqee)
            {
                throw Rafd(
                    6011, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"كتلة توقيع الرقعة طولها {tulKutla} بايت والمتوقع {TulKutlatTawqee} "
                    + "(TAWQEE).",
                    $"The patch signature block is {tulKutla} bytes; {TulKutlatTawqee} were "
                    + "expected (TAWQEE).");
            }

            ReadOnlySpan<byte> kutla = Qism(NawQism.Tawqee);
            uint wasm = BinaryPrimitives.ReadUInt32LittleEndian(kutla.Slice(0, 4));
            ushort isdar = BinaryPrimitives.ReadUInt16LittleEndian(kutla.Slice(4, 2));
            if (wasm != WasmTawqee || isdar != IsdarKutla)
            {
                throw Rafd(
                    6010, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    "كتلة التوقيع داخل الرقعة تالفة: علامتها أو إصدارها غير متوقَّع.",
                    "The signature block inside the patch is malformed: its magic or version "
                    + "is not what this build expects.");
            }

            byte dawr = kutla[6];
            byte khwarizmiya = kutla[7];
            if (dawr > (byte)DawrMiftah.Musahim || khwarizmiya > (byte)Khwarizmiya.Ed25519)
            {
                // Not defaulted. Everywhere else an unknown discriminant
                // degrades to something sensible so a newer patch still loads;
                // here the sensible value would have to be one of two, and
                // guessing "owner" would let a byte nobody wrote decide that a
                // patch is trusted.
                throw Rafd(
                    6010, Ramz.QeemaBatila, Khutwa.TahdithTaarib,
                    $"كتلة التوقيع تعلن دورًا أو خوارزمية لا يعرفهما هذا البناء "
                    + $"(dawr = {dawr}, khwarizmiya = {khwarizmiya}).",
                    $"The signature block declares a role or algorithm this build does not "
                    + $"know (dawr = {dawr}, khwarizmiya = {khwarizmiya}).");
            }
            DawrMuwaqqi = (DawrMiftah)dawr;
            KhwarizmiyatTawqee = (Khwarizmiya)khwarizmiya;
            WaqtTawqee = BinaryPrimitives.ReadInt64LittleEndian(kutla.Slice(8, 8));
            kutla.Slice(16, TulMiftahAam).CopyTo(miftahMuwaqqi);
            kutla.Slice(48, TulTawqee).CopyTo(tawqeeKhaam);

            if (KhwarizmiyatTawqee == Khwarizmiya.Ghayr)
            {
                HalatTawqee = HalatTawqee.GhayrMuwaqqaa;
                throw Rafd(
                    6013, Ramz.QeemaBatila, Khutwa.IlghaTathbeet,
                    "هذه الرقعة غير موقَّعة. لا يثبّت تعريب إلا ما وقّعه مالك المشروع.",
                    "This patch is unsigned. Taarib installs only what the project owner "
                    + "signed.");
            }
            HalatTawqee = HalatTawqee.Maqru;

            if (miftahMutawaqqa.IsEmpty)
            {
                return;
            }
            if (!Basma.Yutabiq(miftahMuwaqqi, miftahMutawaqqa))
            {
                throw Rafd(
                    6012, Ramz.QeemaBatila, Khutwa.IlghaTathbeet,
                    "وقّع الرقعةَ مفتاحٌ غير الذي سجّله المثبِّت لها (TAWQEE.miftah_aam).",
                    "The patch was signed by a different key from the one the installer "
                    + "recorded for it (TAWQEE.miftah_aam).");
            }
            HalatTawqee = HalatTawqee.MutabiqLilMiftah;
        }

        /// <summary>
        /// Reads every section's preamble and binds its arrays.
        /// </summary>
        /// <remarks>
        /// Step four of the validation order, and it runs only after the content
        /// hash has been checked — every count and offset here is a number that
        /// decides how much memory a span covers, and checking the hash after
        /// binding them would be checking it after the damage.
        /// </remarks>
        private void HallJadawil()
        {
            HallNusus();
            HallTakhtit();
            HallKhareeta();
            HallQiyud();
            HallLawha();
            HallKhatt();
        }

        private void HallNusus()
        {
            long qism = izahatAqsam[(int)NawQism.Nusus];
            long tulQism = tulAqsam[(int)NawQism.Nusus];
            ReadOnlySpan<byte> tasdir = TasdirQism(NawQism.Nusus, "NUSUS", TulTasdirKabir);

            adadNusus = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(0, 4)), "NUSUS.adad_nusus");
            adadNitaqat = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(4, 4)), "NUSUS.adad_nitaqat");
            uint izNusus = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(8, 4));
            uint izNitaqat = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(12, 4));
            uint izHawd = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(16, 4));
            uint tulHawd = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(20, 4));

            izahatNusus = qism + MadaSalih(
                izNusus, (long)adadNusus * TulMadkhalNass, tulQism, "NUSUS.izahat_nusus");
            izahatNitaqat = qism + MadaSalih(
                izNitaqat, (long)adadNitaqat * TulMadkhalNitaq, tulQism, "NUSUS.izahat_nitaqat");
            izahatHawdNusus = qism + MadaSalih(izHawd, tulHawd, tulQism, "NUSUS.izahat_hawd");
            tulHawdNusus = AdadSalih(tulHawd, "NUSUS.tul_hawd");

            // Both arrays are searched, so both must be sorted, and both are
            // checked once here rather than trusted on every lookup.
            ReadOnlySpan<MadkhalNass> nusus = Nusus;
            for (int i = 1; i < nusus.Length; i++)
            {
                if (nusus[i - 1].Miftah >= nusus[i].Miftah)
                {
                    throw RafdTarteeb("NUSUS", i);
                }
            }
            ReadOnlySpan<MadkhalNitaq> nitaqat = Nitaqat;
            for (int i = 1; i < nitaqat.Length; i++)
            {
                if (nitaqat[i - 1].Nass > nitaqat[i].Nass)
                {
                    throw RafdTarteeb("NUSUS.nitaqat", i);
                }
            }
        }

        private void HallTakhtit()
        {
            if (!Yahwi(NawQism.Takhtit))
            {
                return;
            }
            long qism = izahatAqsam[(int)NawQism.Takhtit];
            long tulQism = tulAqsam[(int)NawQism.Takhtit];
            ReadOnlySpan<byte> tasdir = TasdirQism(NawQism.Takhtit, "TAKHTIT", TulTasdirKabir);

            adadTakhtitat = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(0, 4)),
                "TAKHTIT.adad_takhtitat");
            uint izTakhtitat = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(4, 4));
            adadHuruf = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(8, 4)), "TAKHTIT.adad_huruf");
            uint izHuruf = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(12, 4));
            adadSutur = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(16, 4)), "TAKHTIT.adad_sutur");
            uint izSutur = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(20, 4));

            izahatTakhtitat = qism + MadaSalih(
                izTakhtitat, (long)adadTakhtitat * TulMadkhalTakhtit, tulQism,
                "TAKHTIT.izahat_takhtitat");
            izahatHuruf = qism + MadaSalih(
                izHuruf, (long)adadHuruf * TulHarf, tulQism, "TAKHTIT.izahat_huruf");
            izahatSutur = qism + MadaSalih(
                izSutur, (long)adadSutur * TulSatr, tulQism, "TAKHTIT.izahat_sutur");

            ReadOnlySpan<MadkhalTakhtit> ruus = Takhtitat;
            for (int i = 0; i < ruus.Length; i++)
            {
                if (i > 0)
                {
                    MadkhalTakhtit qabl = ruus[i - 1];
                    MadkhalTakhtit hali = ruus[i];
                    bool murattab = qabl.Nass < hali.Nass
                        || (qabl.Nass == hali.Nass && qabl.HajmRubi < hali.HajmRubi);
                    if (!murattab)
                    {
                        throw RafdTarteeb("TAKHTIT", i);
                    }
                }

                // Every head's runs are proved in range once, here, so that
                // drawing one is a slice rather than a bounds check on a path
                // that runs while a frame is waiting.
                MadkhalTakhtit ras = ruus[i];
                if (ras.AwwalHarf > (uint)adadHuruf
                    || ras.AdadHuruf > (uint)adadHuruf - ras.AwwalHarf
                    || ras.AwwalSatr > (uint)adadSutur
                    || ras.AdadSutur > (uint)adadSutur - ras.AwwalSatr
                    || ras.Nass >= (uint)adadNusus)
                {
                    throw Rafd(
                        6014, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"التخطيط {i} في الرقعة يشير إلى حروف أو سطور أو نص خارج جداولها.",
                        $"Layout {i} in the patch names glyphs, lines or a string outside its "
                        + "own tables.");
                }
            }
        }

        private void HallKhareeta()
        {
            if (!Yahwi(NawQism.Khareeta))
            {
                return;
            }
            long qism = izahatAqsam[(int)NawQism.Khareeta];
            long tulQism = tulAqsam[(int)NawQism.Khareeta];
            ReadOnlySpan<byte> tasdir = TasdirQism(NawQism.Khareeta, "KHAREETA", TulTasdir);

            adadAshkal = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(0, 4)), "KHAREETA.adad");
            uint izMafatih = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(4, 4));
            uint izMawadi = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(8, 4));

            izahatMafatih = qism + MadaSalih(
                izMafatih, (long)adadAshkal * TulMiftahShakl, tulQism, "KHAREETA.izahat_mafatih");
            izahatMawadi = qism + MadaSalih(
                izMawadi, (long)adadAshkal * TulMawdiShakl, tulQism, "KHAREETA.izahat_mawadi");

            ReadOnlySpan<TaaribMiftahShakl> mafatih = MafatihAshkal;
            for (int i = 1; i < mafatih.Length; i++)
            {
                if (RaqmMiftah(mafatih[i - 1]) >= RaqmMiftah(mafatih[i]))
                {
                    throw RafdTarteeb("KHAREETA", i);
                }
            }
        }

        private void HallQiyud()
        {
            if (!Yahwi(NawQism.Qiyud))
            {
                return;
            }
            long qism = izahatAqsam[(int)NawQism.Qiyud];
            long tulQism = tulAqsam[(int)NawQism.Qiyud];
            ReadOnlySpan<byte> tasdir = TasdirQism(NawQism.Qiyud, "QIYUD", TulTasdir);

            adadQiyud = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(0, 4)), "QIYUD.adad");
            uint iz = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(4, 4));
            izahatQiyud = qism + MadaSalih(
                iz, (long)adadQiyud * TulMadkhalQayd, tulQism, "QIYUD.izaha");

            ReadOnlySpan<MadkhalQayd> quyud = Qiyud;
            for (int i = 1; i < quyud.Length; i++)
            {
                if (quyud[i - 1].Nass >= quyud[i].Nass)
                {
                    throw RafdTarteeb("QIYUD", i);
                }
            }
        }

        private void HallLawha()
        {
            if (!Yahwi(NawQism.Lawha))
            {
                return;
            }
            long qism = izahatAqsam[(int)NawQism.Lawha];
            long tulQism = tulAqsam[(int)NawQism.Lawha];
            ReadOnlySpan<byte> tasdir = TasdirQism(NawQism.Lawha, "LAWHA", TulTasdir);

            adadSafahat = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(0, 4)), "LAWHA.adad_safahat");
            uint iz = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(4, 4));
            izahatSafahat = qism + MadaSalih(
                iz, (long)adadSafahat * TulMadkhalSafha, tulQism, "LAWHA.izahat_safahat");

            ReadOnlySpan<MadkhalSafha> safahat = Safahat;
            for (int i = 0; i < safahat.Length; i++)
            {
                MadkhalSafha safha = safahat[i];
                long madum = (long)safha.Ard * safha.Irtifa;
                if (safha.Izaha > (ulong)tulQism
                    || safha.Tul > (ulong)tulQism - safha.Izaha
                    || madum != safha.Tul)
                {
                    throw Rafd(
                        6017, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"صفحة اللوحة {i} تعلن {safha.Tul} بايت لأبعاد {safha.Ard}×"
                        + $"{safha.Irtifa}، أو تشير خارج قسمها.",
                        $"Atlas page {i} declares {safha.Tul} bytes for dimensions "
                        + $"{safha.Ard}x{safha.Irtifa}, or points outside its section.");
                }
            }
        }

        private void HallKhatt()
        {
            if (!Yahwi(NawQism.Khatt))
            {
                return;
            }
            long qism = izahatAqsam[(int)NawQism.Khatt];
            long tulQism = tulAqsam[(int)NawQism.Khatt];
            ReadOnlySpan<byte> tasdir = TasdirQism(NawQism.Khatt, "KHATT", TulTasdir);

            adadKhutut = AdadSalih(
                BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(0, 4)), "KHATT.adad_khutut");
            uint izKhutut = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(4, 4));
            uint izHawd = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(8, 4));
            uint tulHawd = BinaryPrimitives.ReadUInt32LittleEndian(tasdir.Slice(12, 4));

            izahatKhutut = qism + MadaSalih(
                izKhutut, (long)adadKhutut * TulMadkhalKhatt, tulQism, "KHATT.izahat_khutut");
            izahatHawdKhutut = qism + MadaSalih(izHawd, tulHawd, tulQism, "KHATT.izahat_hawd");
            tulHawdKhutut = AdadSalih(tulHawd, "KHATT.tul_hawd");

            // The hashes sit between the records and the name pool: a fixed
            // 32 bytes each, at an offset the format fixes rather than a field
            // the container gets to choose.
            long baadSijillat = izKhutut + ((long)adadKhutut * TulMadkhalKhatt);
            if (baadSijillat + ((long)adadKhutut * TulBasma) > tulQism)
            {
                throw Rafd(
                    6014, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    "بصمات الخطوط في الرقعة تتجاوز حدود قسمها (KHATT).",
                    "The patch's font hashes run past the end of their section (KHATT).");
            }
            izahatBasmatKhutut = qism + baadSijillat;

            ReadOnlySpan<MadkhalKhatt> khutut = Khutut;
            for (int i = 0; i < khutut.Length; i++)
            {
                if (khutut[i].Fahras != i)
                {
                    throw Rafd(
                        6014, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                        $"سجل الخط {i} يعلن موضعًا {khutut[i].Fahras} في السلسلة، وهو ليس "
                        + "موضعه في الجدول.",
                        $"Font record {i} declares chain position {khutut[i].Fahras}, which is "
                        + "not its position in the table.");
                }
            }
        }

        private ReadOnlySpan<byte> TasdirQism(NawQism naw, string ism, int hajm)
        {
            long tulQism = tulAqsam[(int)naw];
            if (tulQism < hajm)
            {
                throw Rafd(
                    6014, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"قسم {ism} في الرقعة أقصر من ترويسته ({tulQism} بايت مقابل {hajm}).",
                    $"The patch's {ism} section is shorter than its own preamble ({tulQism} "
                    + $"bytes against {hajm}).");
            }
            return Bayt(izahatAqsam[(int)naw], hajm);
        }

        private static int AdadSalih(uint adad, string haql)
        {
            if (adad > int.MaxValue)
            {
                throw Rafd(
                    6014, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"الحقل {haql} يعلن {adad}، وهو أكبر مما يمكن فهرسته.",
                    $"Field {haql} declares {adad}, which is more than can be indexed.");
            }
            return (int)adad;
        }

        private static long MadaSalih(uint izaha, long tul, long tulQism, string haql)
        {
            if (tul < 0 || izaha > tulQism || tul > tulQism - izaha)
            {
                throw Rafd(
                    6014, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                    $"الحقل {haql} يشير إلى مدى خارج قسمه: {izaha}+{tul} في قسم طوله "
                    + $"{tulQism} بايت.",
                    $"Field {haql} names a range outside its section: {izaha}+{tul} in a "
                    + $"section of {tulQism} bytes.");
            }
            return izaha;
        }

        private static KhataTaarib RafdTarteeb(string ism, int fahras)
        {
            return Rafd(
                6018, Ramz.QeemaBatila, Khutwa.IadatTarkibIttar,
                $"جدول {ism} في الرقعة غير مرتَّب عند المدخل {fahras}، ولا يمكن البحث فيه.",
                $"The patch's {ism} table is not sorted at entry {fahras}, so it cannot be "
                + "searched.");
        }

        private void Atlif()
        {
            if (mughlaq)
            {
                return;
            }
            mughlaq = true;
            asas = null;
            if (miqbad != null)
            {
                try
                {
                    miqbad.ReleasePointer();
                }
                catch (InvalidOperationException)
                {
                    // The pointer was never acquired, because construction
                    // failed before AcquirePointer. Releasing one that was not
                    // taken is the only thing this can mean, and it is not a
                    // failure worth propagating out of a cleanup path.
                }
            }
            manzar?.Dispose();
            khareetatMilaff?.Dispose();
            milaff?.Dispose();
        }
    }
}
