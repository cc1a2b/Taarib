// التقاط — every string the takeover saw, and the box the game drew it in.
//
// A translator cannot translate a game whose strings they have never seen. For
// the engines with a readable string container that is a static extraction
// problem, solved elsewhere; for procedurally composed text, for strings behind
// encryption, and for the games whose text is assembled at the moment it is
// drawn, the only place the string exists in one piece is the takeover point.
// Capture mode runs the adapter there in observe-only posture: every string
// that reaches a takeover point is recorded with enough context to find it
// again, and nothing is replaced.
//
// WHY THE RECTANGLE AND THE SIZE ARE THE POINT OF THIS FILE. Recording the
// string alone would be the easy half and the useless half. The measured
// rectangle is what turns a guess into a constraint. Without it the patch
// compiler has to assume how much room a string has — from the string's own
// length, from a sibling element, from a default — and every overflow report it
// produces is speculation dressed as a measurement, which is worse than no
// report at all, because a translator believes it. With the rectangle and the
// size the game actually drew the original at, the compiler measures the shaped
// Arabic against the real box: the overflow report then names strings that will
// really clip, at the size they will really clip at, and a translator who fixes
// every line in it has fixed the patch. That is the whole reason capture exists.
// It is also why "constraints informed by runtime capture" is a distinct bit in
// the container header — AlamatRuqaa.TalmihatIltiqat — rather than an
// implication of the constraints section being present: a patch whose boxes were
// measured and a patch whose boxes were assumed are two different objects, and a
// reviewer looking at an overflow report has to be able to tell which one they
// are holding.
//
// WHY THIS FILE DEFINES ITS OWN RECTANGLE AND ITS OWN POINT. This assembly may
// not name a single UnityEngine type — not Vector2, not Rect, not Transform, not
// GameObject, not Scene. Under Mono every one of those is a type in the game's
// own UnityEngine assemblies; under IL2CPP the same thing reaches managed code
// as an Il2CppInterop proxy generated from that specific game's metadata, in an
// assembly with a different identity. An assembly that named either could not
// load in the other's process, and the takeover logic has to compile once and
// run in both. So Iltiqat owns plain value types for a rectangle and a point,
// with a stated coordinate convention, and each adapter performs the two-line
// conversion from whatever its runtime calls a rectangle. The alternative — a
// capture type per backend — would be the one place where the two adapters could
// disagree about what a measurement means, which is the class of divergence
// Mushtarak exists to make impossible.
//
// CAPTURE HAS TO BE CHEAP ENOUGH TO LEAVE ON. A game draws thousands of strings
// a second and almost all of them are the same string again: the same menu
// label, redrawn because the canvas was dirtied; the same health counter, at the
// same size, in the same rectangle, sixty times a second. Recording every
// occurrence would produce a hundred megabytes of near-identical rows for a
// five-minute session, and the translator's first job would be to deduplicate it
// — badly, because the reader has less context than the recorder did. So the
// session deduplicates on identity, keeps one record per distinct string with an
// occurrence count, and never allocates again once a string has been seen once.
// The one thing a repeat sighting can change is the geometry, and it changes it
// only upward; see JalsatIltiqat.Sajjil for why the largest box wins.
//
// THE FILE THIS PRODUCES IS READ BY ANOTHER PROGRAM. `barid` — the Studio-side
// ingest — reads the format written out in full on the Iltiqat class below. Two
// programs that have to agree on a format, that ship separately, and that a user
// updates independently, need a version on the first line and a field count
// beside it; the argument is on Iltiqat.Wasm and it is not a formality.

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mushtarak
{
    /// <summary>
    /// Which text system a captured string came from.
    /// </summary>
    /// <remarks>
    /// Recorded rather than inferred. The same string can reach two different
    /// systems in one game — a label in TextMeshPro and the same label in a
    /// legacy uGUI tooltip — and the two do not measure text the same way, do
    /// not wrap it the same way, and do not fail the same way when it is too
    /// long. A capture row that did not say which one it came from would send
    /// the compiler's measurement through the wrong model, and the resulting
    /// overflow report would be wrong in exactly the cases it exists for.
    /// </remarks>
    public enum NizamNass : byte
    {
        /// <summary>
        /// The adapter could not name the system. Recorded honestly rather
        /// than guessed: a wrong system is worse than an absent one, because
        /// the compiler acts on the first and asks about the second.
        /// </summary>
        Majhul = 0,

        /// <summary><c>TMP_Text</c>, both the world-space and the canvas form.</summary>
        TextMeshPro = 1,

        /// <summary><c>UnityEngine.UI.Text</c>, the legacy uGUI label.</summary>
        WajihatUnity = 2,

        /// <summary>NGUI's <c>UILabel</c>.</summary>
        Ngui = 3,

        /// <summary>FairyGUI's <c>TextField</c> and <c>GTextField</c>.</summary>
        FairyGui = 4,

        /// <summary>World-space <c>TextMesh</c>, which has no rectangle of its own.</summary>
        NassAalami = 5,
    }

    /// <summary>
    /// The bits of a <see cref="SijillIltiqat"/>'s flag field, and of the
    /// <c>alam</c> column of a capture line.
    /// </summary>
    /// <remarks>
    /// Two booleans in one integer column rather than two columns, because the
    /// set will grow — a later build that learns to record "this element is
    /// auto-sized" or "this element was mirrored" takes the next free bit and
    /// the field count on the version line does not change, which means an
    /// older reader keeps working. A new column would not have that property.
    /// </remarks>
    public static class AlamatSijill
    {
        /// <summary>The component wraps text rather than letting it run past its edge.</summary>
        public const uint Iltifaf = 1u << 0;

        /// <summary>
        /// The takeover point was marked sensitive, so this record carries a
        /// length and a hash where the source text would be. See
        /// <see cref="TalabIltiqat.Hassas"/>.
        /// </summary>
        public const uint Hassas = 1u << 1;
    }

    /// <summary>
    /// One point in the space the adapter measured in: two floats, and nothing
    /// this assembly is forbidden to name.
    /// </summary>
    /// <remarks>
    /// An adapter builds one of these from whatever its runtime calls a
    /// two-component vector. The conversion is a field copy in each direction
    /// and it belongs in the adapter, because the type on the other side of it
    /// is the type Mushtarak cannot mention.
    /// </remarks>
    public struct NuqtaIltiqat
    {
        /// <summary>Horizontal position, rightward.</summary>
        public float S;

        /// <summary>Vertical position, upward.</summary>
        public float A;

        /// <summary>Builds a point from its two components.</summary>
        /// <param name="s">Horizontal, rightward.</param>
        /// <param name="a">Vertical, upward.</param>
        /// <returns>The point.</returns>
        public static NuqtaIltiqat Min(float s, float a)
        {
            NuqtaIltiqat natija;
            natija.S = s;
            natija.A = a;
            return natija;
        }
    }

    /// <summary>
    /// The rectangle a string was drawn into: left edge, bottom edge, width and
    /// height. Y-up, with the same field names and the same convention
    /// <see cref="MustatilRasm"/> uses.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The convention is copied from the mesh path deliberately. Two rectangle
    /// types in one assembly that disagreed about which way Y grows would be a
    /// sign error waiting for the first person who moved a value from one to
    /// the other, and the symptom of that sign error is a capture whose boxes
    /// are mirrored about the component's middle — which reads as a plausible
    /// layout and is not one. One convention, stated twice, is cheaper than one
    /// convention discovered twice.
    /// </para>
    /// <para>
    /// The units are the component's own: canvas pixels for a canvas element,
    /// world units for world-space text. The compiler needs the rectangle and
    /// <see cref="TalabIltiqat.Hajm"/> to be in the <em>same</em> units, which
    /// they are, because both come from the same component — it does not need
    /// them in any particular unit, because a ratio between a measured Arabic
    /// width and a measured box width has no unit at all.
    /// </para>
    /// <para>
    /// A component with no rectangle — world-space <see cref="NizamNass.NassAalami"/>
    /// is the case — records zeroes. A zero box is not a constraint and the
    /// compiler reads it as "unbounded", which is the truth about a text object
    /// hanging in a scene with nothing to clip it.
    /// </para>
    /// </remarks>
    public struct MustatilIltiqat
    {
        /// <summary>The left edge.</summary>
        public float Yasar;

        /// <summary>The bottom edge.</summary>
        public float Asfal;

        /// <summary>
        /// Width. Never negative in a stored record; see
        /// <see cref="Iltiqat.Sahih(in MustatilIltiqat)"/>.
        /// </summary>
        public float Ard;

        /// <summary>Height. Never negative in a stored record.</summary>
        public float Irtifa;

        /// <summary>
        /// Builds a rectangle from the corner and size an adapter reads off its
        /// runtime's own rectangle type.
        /// </summary>
        /// <param name="rukn">The bottom-left corner.</param>
        /// <param name="qiyas">The width and height, as a point.</param>
        /// <returns>The rectangle.</returns>
        public static MustatilIltiqat Min(NuqtaIltiqat rukn, NuqtaIltiqat qiyas)
        {
            MustatilIltiqat natija;
            natija.Yasar = rukn.S;
            natija.Asfal = rukn.A;
            natija.Ard = qiyas.S;
            natija.Irtifa = qiyas.A;
            return natija;
        }

        /// <summary>The bottom-left corner, for an adapter converting back.</summary>
        /// <returns>The corner as a point.</returns>
        public readonly NuqtaIltiqat Rukn()
        {
            return NuqtaIltiqat.Min(Yasar, Asfal);
        }

        /// <summary>
        /// The centre, which is what a screenshot crop of where a string
        /// appeared is taken around.
        /// </summary>
        /// <returns>The centre as a point.</returns>
        public readonly NuqtaIltiqat Markaz()
        {
            return NuqtaIltiqat.Min(Yasar + (Ard * 0.5f), Asfal + (Irtifa * 0.5f));
        }

        /// <summary>
        /// The area, which is the ordering <see cref="Iltiqat.Aakbar"/> compares
        /// two sightings by. Defined here and called from there, so the
        /// comparison and the quantity it compares cannot drift apart.
        /// </summary>
        /// <returns>Width times height.</returns>
        public readonly float Misaha()
        {
            return Ard * Irtifa;
        }
    }

    /// <summary>
    /// One sighting, as the adapter observed it at a takeover point. A plain
    /// struct passed by <c>in</c>: nothing here is owned by this assembly and
    /// nothing here is copied until the session decides to keep it.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Every string field is nullable and null is read as empty. That is not
    /// defensiveness for its own sake: an adapter that could not resolve a
    /// scene name — because the scene is being unloaded, because the game
    /// composes its UI outside any scene — says so by passing null, and a
    /// record with an empty scene is honest. An adapter that invented a name to
    /// avoid a null would put a fiction in the translator's workspace.
    /// </para>
    /// <para>
    /// <b>What the caller must have done before calling.</b>
    /// <see cref="Masar"/> and <see cref="Mashhad"/> must already exist as
    /// strings; they must not be built on the frame that draws. Walking a
    /// transform chain and concatenating names is the single most expensive
    /// thing at a takeover point, it produces the same string every frame, and
    /// it is garbage in somebody else's frame budget. Build the path once when
    /// the takeover point is bound to its component, keep it beside the
    /// binding, and hand the same reference every frame. The session stores the
    /// reference and copies nothing.
    /// </para>
    /// </remarks>
    public struct TalabIltiqat
    {
        /// <summary>
        /// What makes this sighting the same thing as an earlier one.
        /// </summary>
        /// <remarks>
        /// <para>
        /// For an ordinary takeover point this is the string-table key of the
        /// source text: the first eight bytes of its BLAKE3 hash, little-endian
        /// — the number <see cref="Iltiqat.Huwiya(string)"/> computes and the
        /// same number <see cref="MadkhalNass.Miftah"/> holds. The adapter
        /// already has it, because it had to compute it to ask the patch
        /// whether this string has a translation, so capture costs no hashing
        /// at all on the frame path. It also means a capture file joins against
        /// a compiled container on the identity column with no second hashing
        /// rule to keep in step, and two hashing rules that must agree are one
        /// of them being wrong after the next change.
        /// </para>
        /// <para>
        /// For a point marked <see cref="Hassas"/> this must instead be
        /// <see cref="Iltiqat.HuwiyatMawdi"/> — the identity of the
        /// <em>place</em>, computed once when the point was bound. See
        /// <see cref="Hassas"/> for why the place and not the text.
        /// </para>
        /// <para>
        /// Zero is accepted like any other value rather than refused, because
        /// refusing would drop a string for a reason the translator never sees.
        /// It is worth recognising in a capture file though: one record at
        /// identity <c>0000000000000000</c> with an enormous occurrence count
        /// means the adapter is not filling this field, and every string in
        /// that session collapsed into it.
        /// </para>
        /// </remarks>
        public ulong Huwiya;

        /// <summary>
        /// The source string, exactly as it reached the takeover point,
        /// including any markup the game embedded in it.
        /// </summary>
        /// <remarks>
        /// Markup is kept rather than stripped. A string carrying
        /// <c>&lt;color&gt;</c> or a format placeholder is a different
        /// translation problem from the same words without it, the compiler's
        /// markup bridge needs the tags to produce style spans, and a stripped
        /// string cannot be put back. Required even when <see cref="Hassas"/>
        /// is set: the session needs its length and its hash, and neither is
        /// written to the file as text.
        /// </remarks>
        public string? Asl;

        /// <summary>
        /// The component's path in the scene hierarchy, as the adapter names it
        /// — root name, then each child name, separated however the adapter
        /// separates them, so long as it is consistent within a session.
        /// </summary>
        /// <remarks>
        /// This is the field that makes a capture actionable. An identity tells
        /// the compiler which strings are the same; the path tells a person
        /// where to go and look at one. A capture whose paths are empty is a
        /// list of strings with no way back to the screen they were on, and the
        /// translator's question about a two-word label — is this a button or a
        /// heading? — has no answer in it.
        /// </remarks>
        public string? Masar;

        /// <summary>The scene the component was in, by name.</summary>
        public string? Mashhad;

        /// <summary>Which text system drew it.</summary>
        public NizamNass Nizam;

        /// <summary>
        /// The rectangle the component drew into, in the component's own units.
        /// The measurement this whole file exists to carry.
        /// </summary>
        public MustatilIltiqat Mustatil;

        /// <summary>
        /// The font size the string was drawn at, in the same units as
        /// <see cref="Mustatil"/>. After auto-sizing, not before: the compiler
        /// is measuring against what the player saw, and a component that shrank
        /// its text to fit shrank it for a reason that applies to the Arabic too.
        /// </summary>
        public float Hajm;

        /// <summary>
        /// Whether the component wraps. A single-line field and a wrapping field
        /// with the same box are two different constraints — one overflows in
        /// width and the other in height — and the compiler cannot tell them
        /// apart from the geometry.
        /// </summary>
        public bool Yaltaff;

        /// <summary>
        /// Whether this takeover point may show text that belongs to the person
        /// playing. When set, the session records a length and a hash where the
        /// source text would be, and the text itself is never written anywhere.
        /// </summary>
        /// <remarks>
        /// <para>
        /// A string reaching a takeover point can be a player's own name, the
        /// path of their save file, a message someone typed to them in chat, or
        /// the name of a character they made up. A capture file is a file a
        /// translator sends to a project, and none of that belongs in it. A
        /// translator does not need a player's name to translate the label
        /// beside it — they need to know that a name goes there, roughly how
        /// long it can be, and what the box around it is, and all three survive
        /// redaction intact.
        /// </para>
        /// <para>
        /// <b>Mark the point, not the string.</b> The decision is made once,
        /// when the adapter binds a takeover point to a component it recognises
        /// as an input field, a chat line, a save-slot label or a character-name
        /// display. It cannot be made by looking at the text, because text that
        /// looks like a name is a name only sometimes and a name that looks like
        /// a menu label is still a name.
        /// </para>
        /// <para>
        /// <b>Set <see cref="Huwiya"/> to the place.</b> A sensitive point must
        /// carry <see cref="Iltiqat.HuwiyatMawdi"/> rather than a content hash,
        /// for two reasons that both matter. It is what makes the record a
        /// record of a slot — one row for "the chat line", not one row per
        /// message — so a chat window does not consume the whole session cap in
        /// a minute. And a content hash of a low-entropy string written into a
        /// shared file is a value anybody holding the file can confirm a guess
        /// against, which would give back most of what redaction removed.
        /// </para>
        /// </remarks>
        public bool Hassas;

        /// <summary>
        /// The frame this sighting happened on, as the adapter's own frame
        /// counter reads it. Any monotonically increasing per-frame number will
        /// do; the compiler uses differences between records, never the value.
        /// </summary>
        public long Itar;
    }

    /// <summary>
    /// One distinct string, as the session kept it: the string, where it was
    /// drawn, the largest box it was ever drawn in, and how often it was seen.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>A record is one real sighting plus two aggregates.</b> Everything
    /// describing where and how — <see cref="Nizam"/>, <see cref="Masar"/>,
    /// <see cref="Mashhad"/>, <see cref="Mustatil"/>, <see cref="Hajm"/>,
    /// <see cref="Yaltaff"/> — is replaced together, as a group, whenever a
    /// larger box arrives, and therefore always describes one sighting that
    /// really happened. Stitching one sighting's path to another's rectangle
    /// would produce a row describing a draw that never occurred, and it is the
    /// kind of wrongness nobody catches by reading the file.
    /// </para>
    /// <para>
    /// The two aggregates are <see cref="Adad"/>, which counts every sighting,
    /// and — for a redacted record only — <see cref="TulAsl"/> with
    /// <see cref="BasmatAsl"/>, which track the longest text the slot ever held.
    /// That exception is deliberate: for a sensitive point the text differs
    /// between sightings by definition, and the constraint the label beside it
    /// has to survive is the longest name that ever appeared, not the first one.
    /// </para>
    /// <para>
    /// <see cref="Tasalsul"/> and <see cref="Itar"/> are the first sighting's,
    /// and a later sighting never moves them. Sorting a capture file by
    /// <see cref="Tasalsul"/> therefore reconstructs the order in which strings
    /// first appeared, which is the order a translator walked the game in — the
    /// one ordering that lets a workspace present a capture as a play session
    /// rather than as an alphabetical list.
    /// </para>
    /// </remarks>
    public struct SijillIltiqat
    {
        /// <summary>The identity this record was deduplicated on.</summary>
        public ulong Huwiya;

        /// <summary>
        /// The source string, or <see cref="string.Empty"/> when
        /// <see cref="Hassas"/> is set. A redacted record never holds the text,
        /// not in memory and not in the file.
        /// </summary>
        public string Asl;

        /// <summary>
        /// The BLAKE3-derived hash of the longest text this slot held, when
        /// <see cref="Hassas"/> is set; zero otherwise.
        /// </summary>
        /// <remarks>
        /// It is a change detector and it is not anonymisation. It tells the
        /// compiler that two sightings at one slot held different text, and it
        /// is exactly as guessable as the text is: anyone holding this file and
        /// a list of candidate names can confirm a guess against it. The
        /// privacy boundary is that the text is never written — not that this
        /// number is opaque. Saying otherwise in a document somewhere would be
        /// how someone comes to rely on it.
        /// </remarks>
        public ulong BasmatAsl;

        /// <summary>
        /// The length of the source text in UTF-8 bytes; for a redacted record,
        /// the longest such length seen. Bytes rather than UTF-16 units because
        /// that is the unit every other length in this product is in, and a
        /// character count would disagree with all of them about a surrogate
        /// pair and about a mark.
        /// </summary>
        public int TulAsl;

        /// <summary>The component path of the sighting this record describes.</summary>
        public string Masar;

        /// <summary>The scene name of that sighting.</summary>
        public string Mashhad;

        /// <summary>The text system of that sighting.</summary>
        public NizamNass Nizam;

        /// <summary>The largest box this string was drawn in.</summary>
        public MustatilIltiqat Mustatil;

        /// <summary>The font size it was drawn at in that box.</summary>
        public float Hajm;

        /// <summary>Whether that component wraps.</summary>
        public bool Yaltaff;

        /// <summary>Whether the text is redacted.</summary>
        public bool Hassas;

        /// <summary>
        /// The order this string first appeared in, counting from zero and
        /// advancing once per distinct string.
        /// </summary>
        public long Tasalsul;

        /// <summary>The frame the first sighting happened on.</summary>
        public long Itar;

        /// <summary>How many sightings there have been, including the first.</summary>
        public long Adad;

        /// <summary>
        /// The two boolean fields as the flag word a capture line carries.
        /// </summary>
        /// <returns>A combination of <see cref="AlamatSijill"/> bits.</returns>
        public readonly uint Alam()
        {
            uint alam = 0;
            if (Yaltaff)
            {
                alam |= AlamatSijill.Iltifaf;
            }
            if (Hassas)
            {
                alam |= AlamatSijill.Hassas;
            }
            return alam;
        }
    }

    /// <summary>
    /// What <see cref="JalsatIltiqat.Sajjil"/> did with a sighting.
    /// </summary>
    public enum NatijatSajjil
    {
        /// <summary>A string nothing had seen before; a record was created.</summary>
        Jadeed = 0,

        /// <summary>
        /// A string already in the session; its count advanced and its geometry
        /// was kept or replaced.
        /// </summary>
        Mukarrar = 1,

        /// <summary>
        /// Nothing was recorded, because the session reached its cap and stopped.
        /// See <see cref="JalsatIltiqat.Mumtali"/>.
        /// </summary>
        Mutawaqqif = 2,
    }

    /// <summary>
    /// The first line of a capture file, parsed.
    /// </summary>
    public readonly struct RasIltiqat
    {
        /// <summary>Records a parsed version line.</summary>
        /// <param name="isdar">The format version.</param>
        /// <param name="adadHuqul">Fields per record line.</param>
        /// <param name="adadSijillat">How many record lines follow.</param>
        /// <param name="tam">Whether the session that wrote it was complete.</param>
        /// <param name="adadMuhmal">Sightings dropped after the cap.</param>
        public RasIltiqat(int isdar, int adadHuqul, int adadSijillat, bool tam, long adadMuhmal)
        {
            Isdar = isdar;
            AdadHuqul = adadHuqul;
            AdadSijillat = adadSijillat;
            Tam = tam;
            AdadMuhmal = adadMuhmal;
        }

        /// <summary>The format version, which this build reads only one of.</summary>
        public int Isdar { get; }

        /// <summary>How many tab-separated fields each record line has.</summary>
        public int AdadHuqul { get; }

        /// <summary>How many record lines the writer said it wrote.</summary>
        public int AdadSijillat { get; }

        /// <summary>
        /// Whether the session recorded everything it saw. False means the
        /// session hit its cap and stopped, and strings the player saw after
        /// that moment are not in the file.
        /// </summary>
        public bool Tam { get; }

        /// <summary>
        /// How many sightings were refused after the cap. Zero when
        /// <see cref="Tam"/> is true. It is not a count of missing strings —
        /// most of those sightings were repeats of strings that are in the file
        /// — but it is the only number the file carries about how much play
        /// happened after capture stopped.
        /// </summary>
        public long AdadMuhmal { get; }
    }

    /// <summary>
    /// The capture line format, and the two computations a capture row's
    /// identity is built from. Stateless; every member is safe to call from any
    /// thread.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The format, in full.</b> A capture file is UTF-8 with no byte order
    /// mark, one record per line, fields separated by a single tab
    /// (<c>U+0009</c>), lines terminated by a single line feed
    /// (<c>U+000A</c>) — never a carriage return and line feed. Those three
    /// choices are stated rather than inherited: a byte order mark turns the
    /// first field of the first line into a field that does not compare equal
    /// to itself, and a platform-default line terminator would leave a stray
    /// <c>U+000D</c> on the end of every last field for a reader that splits on
    /// line feed. Both are the kind of defect that survives every test written
    /// on the machine that produced the file.
    /// </para>
    /// <para>
    /// <b>The first line is the version line</b>, six tab-separated fields:
    /// </para>
    /// <code>
    ///     taarib-iltiqat &lt;TAB&gt; isdar &lt;TAB&gt; adad_huqul &lt;TAB&gt;
    ///     adad_sijillat &lt;TAB&gt; tam &lt;TAB&gt; adad_muhmal
    ///
    ///     taarib-iltiqat   the format name, verbatim
    ///     isdar            the format version, decimal; 1 today
    ///     adad_huqul       fields per record line, decimal; 15 today
    ///     adad_sijillat    how many record lines follow, decimal
    ///     tam              1 when the session recorded everything it saw,
    ///                      0 when it reached its cap and stopped
    ///     adad_muhmal      sightings refused after the cap, decimal
    /// </code>
    /// <para>
    /// <b>Why a format two programs share needs a version on its first line.</b>
    /// The two programs ship separately and on different schedules. One of them
    /// is inside a patch installed in somebody's game and updates when they
    /// reinstall it; the other is Studio's ingest and updates when they update
    /// Studio. A user will run mismatched versions of the pair, not as an edge
    /// case but as the ordinary state of the world. Without a version on line
    /// one, the day a field is inserted into the middle of a record is the day
    /// every reader silently shifts by one column: rectangles are read as font
    /// sizes, scene names as component paths, and the overflow report is then
    /// computed from numbers that mean nothing — and it still looks like an
    /// overflow report. With it, the mismatch is one refusal naming the version
    /// found and the version understood, raised before a single row is parsed.
    /// The field count beside it catches the other half of the same problem: a
    /// version that did not change but a row that did.
    /// </para>
    /// <para>
    /// <b>Each following line is one record</b>, fifteen tab-separated fields,
    /// in this order:
    /// </para>
    /// <code>
    ///      1  huwiya     identity, exactly 16 lowercase hex digits, no prefix
    ///      2  tasalsul   first-sighting order, decimal, from 0
    ///      3  itar       frame of the first sighting, decimal
    ///      4  adad       occurrences, decimal, at least 1
    ///      5  nizam      NizamNass as its decimal discriminant
    ///      6  alam       AlamatSijill bits, decimal
    ///      7  hajm       font size, invariant round-trip decimal
    ///      8  yasar      rectangle left edge, invariant round-trip decimal
    ///      9  asfal      rectangle bottom edge
    ///     10  ard        rectangle width
    ///     11  irtifa     rectangle height
    ///     12  tul        source length in UTF-8 bytes, decimal
    ///     13  mashhad    scene name, escaped
    ///     14  masar      component path, escaped
    ///     15  asl        source text, escaped -- or, when bit 1 of alam is
    ///                    set, exactly 16 lowercase hex digits: the hash that
    ///                    stands in for redacted text
    /// </code>
    /// <para>
    /// The identity is hex rather than decimal because it is 64 bits unsigned
    /// and half the values in that range print as a negative number through a
    /// reader that took it for a signed integer — hex has no signed form to get
    /// wrong — and because a fixed sixteen columns is greppable. The three free
    /// text fields are last so that a malformed row still yields its numbers,
    /// and the longest field is at the end where a truncated line is obvious.
    /// The enum is written as its number, not its name, for the same reason the
    /// ABI's enums cross as numbers: a name would need a second table both
    /// programs agree on, and the discriminant already is that table.
    /// </para>
    /// <para>
    /// <b>Escaping</b> applies to fields 13, 14 and 15, and to nothing else.
    /// Backslash introduces every escape and there are exactly six:
    /// </para>
    /// <code>
    ///     \\        a single backslash
    ///     \t        U+0009, the separator
    ///     \n        U+000A, the line terminator
    ///     \r        U+000D
    ///     \xHH      any other character below U+0020, and U+007F,
    ///               with two lowercase hex digits
    ///     \uHHHH    an unpaired surrogate, with four lowercase hex digits
    /// </code>
    /// <para>
    /// Everything else is written verbatim as UTF-8, including every non-ASCII
    /// character: a capture of an Arabic or Japanese game is readable in an
    /// editor, which matters because a person will open one. A backslash
    /// followed by anything else is malformed and a reader refuses the line
    /// rather than guessing. An empty field is two adjacent tabs and is
    /// perfectly legal — a component with no scene writes one.
    /// </para>
    /// <para>
    /// The last two escapes are not padding. <c>\xHH</c> exists because game
    /// strings genuinely carry control characters — a stray <c>U+0000</c> from a
    /// fixed-size buffer, a <c>U+000B</c> a designer typed — and one of those
    /// written raw makes the whole file look binary to every tool that sniffs
    /// content, breaks <c>grep</c>, and renders as nothing in the editor the
    /// translator opens it in. <c>\uHHHH</c> exists because a .NET string can
    /// hold an unpaired surrogate and game strings sliced by character count
    /// routinely do; UTF-8 encoding replaces one with <c>U+FFFD</c> silently, so
    /// writing the field raw would change the text on its way to the file and
    /// the round trip would not be one.
    /// </para>
    /// </remarks>
    public static class Iltiqat
    {
        /// <summary>The format name, the first field of the first line.</summary>
        public const string Wasm = "taarib-iltiqat";

        /// <summary>The one format version this build writes and reads.</summary>
        public const int IsdarSigha = 1;

        /// <summary>The field separator.</summary>
        public const char Fasil = '\t';

        /// <summary>The line terminator, always exactly this and never the platform's.</summary>
        public const string NihayatSatr = "\n";

        /// <summary>Fields per record line.</summary>
        public const int AdadHuqul = 15;

        /// <summary>Fields on the version line.</summary>
        public const int AdadHuqulRas = 6;

        /// <summary>
        /// The most UTF-8 bytes <see cref="Huwiya(ReadOnlySpan{char})"/> encodes
        /// on the stack before renting a buffer. Five hundred and twelve covers
        /// every label, menu entry and item name a game has; a dialogue
        /// paragraph goes through the pool, which is the cold half of a call
        /// that is already off the frame path.
        /// </summary>
        public const int HaddMakdas = 512;

        private const string Khanat = "0123456789abcdef";

        /// <summary>
        /// The string-table key of a piece of text: the first eight bytes of its
        /// BLAKE3 hash, read little-endian.
        /// </summary>
        /// <param name="nassUtf8">The text, already UTF-8.</param>
        /// <returns>The identity.</returns>
        /// <remarks>
        /// One hash function, and it is the container's. A capture row and a
        /// compiled string table therefore agree on identity by construction
        /// rather than by two implementations of one rule staying in step —
        /// which they would not, and the symptom of them drifting is a capture
        /// that joins against nothing and looks like an empty session.
        /// </remarks>
        public static ulong Huwiya(ReadOnlySpan<byte> nassUtf8)
        {
            return Basma.Ihsib64(nassUtf8);
        }

        /// <summary>
        /// The string-table key of a managed string, encoded to UTF-8 first.
        /// </summary>
        /// <param name="nass">The text.</param>
        /// <returns>The identity.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="nass"/> is null.</exception>
        /// <remarks>
        /// Provided for callers that do not already hold the key. An adapter on
        /// the frame path normally does — it computed one to ask the patch for a
        /// translation — and should pass that rather than call this.
        /// </remarks>
        public static ulong Huwiya(string nass)
        {
            if (nass is null)
            {
                throw new ArgumentNullException(nameof(nass));
            }
            return Huwiya(nass.AsSpan());
        }

        /// <summary>
        /// The string-table key of a run of characters.
        /// </summary>
        /// <param name="nass">The text.</param>
        /// <returns>The identity.</returns>
        /// <remarks>
        /// The encoder substitutes <c>U+FFFD</c> for an unpaired surrogate, so
        /// two strings differing only there hash alike. That is the right
        /// behaviour and not a compromise: the patch compiler encodes the same
        /// string the same way when it builds the container's key, so the two
        /// still agree, and the capture line itself preserves the surrogate
        /// through the <c>\uHHHH</c> escape, so nothing is lost from the record.
        /// </remarks>
        public static ulong Huwiya(ReadOnlySpan<char> nass)
        {
            int tul = Encoding.UTF8.GetByteCount(nass);
            if (tul <= HaddMakdas)
            {
                Span<byte> baytat = stackalloc byte[HaddMakdas];
                int maktub = Encoding.UTF8.GetBytes(nass, baytat);
                return Basma.Ihsib64(baytat.Slice(0, maktub));
            }

            byte[] mustaar = ArrayPool<byte>.Shared.Rent(tul);
            try
            {
                int maktub = Encoding.UTF8.GetBytes(nass, mustaar);
                return Basma.Ihsib64(new ReadOnlySpan<byte>(mustaar, 0, maktub));
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(mustaar);
            }
        }

        /// <summary>
        /// The identity of a takeover <em>point</em> rather than of a string:
        /// the hash of the scene, the component path and the text system.
        /// </summary>
        /// <param name="mashhad">The scene name; null is read as empty.</param>
        /// <param name="masar">The component path; null is read as empty.</param>
        /// <param name="nizam">The text system.</param>
        /// <returns>The identity of the place.</returns>
        /// <remarks>
        /// <para>
        /// This is what a sensitive takeover point records under. It gives the
        /// session one row for a chat line instead of one row per message, and
        /// it keeps a low-entropy content hash out of a file that leaves the
        /// machine. Compute it once, when the point is bound to its component,
        /// and keep it beside the binding: it costs a hash over two strings,
        /// which is nothing once and is not nothing sixty times a second.
        /// </para>
        /// <para>
        /// The three parts are joined with <c>U+001F</c>, the unit separator,
        /// which no scene name and no transform name contains. Joining with a
        /// character that could occur in either would make two different points
        /// collide — a scene called <c>a</c> with a path <c>b/c</c> and a scene
        /// called <c>a/b</c> with a path <c>c</c> — and a collision here merges
        /// two slots into one row that describes neither.
        /// </para>
        /// </remarks>
        public static ulong HuwiyatMawdi(string? mashhad, string? masar, NizamNass nizam)
        {
            ReadOnlySpan<char> m = (mashhad ?? string.Empty).AsSpan();
            ReadOnlySpan<char> p = (masar ?? string.Empty).AsSpan();
            int tulM = Encoding.UTF8.GetByteCount(m);
            int tulP = Encoding.UTF8.GetByteCount(p);
            int tul = tulM + tulP + 2;

            if (tul <= HaddMakdas)
            {
                Span<byte> baytat = stackalloc byte[HaddMakdas];
                Ijma(m, p, nizam, baytat);
                return Basma.Ihsib64(baytat.Slice(0, tul));
            }

            byte[] mustaar = ArrayPool<byte>.Shared.Rent(tul);
            try
            {
                Ijma(m, p, nizam, mustaar);
                return Basma.Ihsib64(new ReadOnlySpan<byte>(mustaar, 0, tul));
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(mustaar);
            }
        }

        /// <summary>
        /// Whether one rectangle should replace another as the constraint a
        /// record carries: larger by area, ties broken by width.
        /// </summary>
        /// <param name="jadeed">The rectangle just measured.</param>
        /// <param name="qadeem">The rectangle the record already holds.</param>
        /// <returns>Whether the new one wins.</returns>
        /// <remarks>
        /// <para>
        /// One rule, stated once, because the alternative is two call sites
        /// rounding a comparison differently. Area rather than either dimension
        /// alone because neither dimension alone orders the cases that occur: a
        /// box that is wider and shorter than another contains neither and is
        /// contained by neither, and a componentwise maximum of the two would
        /// invent a box the game never drew.
        /// </para>
        /// <para>
        /// <b>Why larger wins and not smaller.</b> A conservative reading says
        /// the smallest box a string was ever drawn in is the constraint that
        /// binds, and for two genuinely different slots holding the same string
        /// that is true. It is not what the sightings are. The overwhelming
        /// majority of repeat sightings are the same slot measured again on a
        /// frame where the layout had not settled: a rectangle that is zero by
        /// zero on the frame the object was created, then a partial size while
        /// the canvas runs its first layout pass, then the real size. Taking the
        /// smallest pins every record to the frame before layout ran, which is
        /// almost always a zero-width box, and an overflow report computed
        /// against zero-width boxes says every string in the game clips — which
        /// is the same as saying nothing, and takes a day to work out. Taking
        /// the largest discards exactly those half-formed measurements and lands
        /// on the box the game settled on.
        /// </para>
        /// </remarks>
        public static bool Aakbar(in MustatilIltiqat jadeed, in MustatilIltiqat qadeem)
        {
            float misahatJ = jadeed.Misaha();
            float misahatQ = qadeem.Misaha();
            if (misahatJ > misahatQ)
            {
                return true;
            }
            if (misahatJ < misahatQ)
            {
                return false;
            }
            return jadeed.Ard > qadeem.Ard;
        }

        /// <summary>
        /// A float fit to store: the value itself when it is finite and not
        /// negative zero, and zero otherwise.
        /// </summary>
        /// <param name="qeema">The measurement as the adapter read it.</param>
        /// <returns>A finite value.</returns>
        /// <remarks>
        /// A component mid-teardown, a layout group that divided by a zero
        /// count, a rect driven by an animation that overshot — each of them
        /// hands back a NaN or an infinity, and each of them reaches a takeover
        /// point in a real game. A NaN stored in a record is not merely a bad
        /// number in a file; it is permanent, because every comparison against
        /// NaN is false, so <see cref="Aakbar"/> would never replace it and that
        /// record would carry the garbage for the rest of the session. It also
        /// serialises as <c>NaN</c>, which the reader on the other side does not
        /// parse. Sanitising at the door costs one test per field.
        /// </remarks>
        public static float Sahih(float qeema)
        {
            if (!float.IsFinite(qeema))
            {
                return 0f;
            }
            return qeema == 0f ? 0f : qeema;
        }

        /// <summary>
        /// A rectangle with every field sanitised and no negative extent.
        /// </summary>
        /// <param name="mustatil">The rectangle as measured.</param>
        /// <returns>The rectangle fit to store.</returns>
        /// <remarks>
        /// A negative width is not a small box, it is a box expressed from the
        /// other corner, and some components hand one back when their transform
        /// has a negative scale. Left as it is, its area is negative, so
        /// <see cref="Aakbar"/> ranks it below an empty box and the record keeps
        /// nothing. Normalising moves the origin to the corner that is actually
        /// lower-left and makes the extent positive, which is the same rectangle
        /// said correctly.
        /// </remarks>
        public static MustatilIltiqat Sahih(in MustatilIltiqat mustatil)
        {
            float yasar = Sahih(mustatil.Yasar);
            float asfal = Sahih(mustatil.Asfal);
            float ard = Sahih(mustatil.Ard);
            float irtifa = Sahih(mustatil.Irtifa);
            if (ard < 0f)
            {
                yasar += ard;
                ard = -ard;
            }
            if (irtifa < 0f)
            {
                asfal += irtifa;
                irtifa = -irtifa;
            }

            MustatilIltiqat natija;
            natija.Yasar = yasar;
            natija.Asfal = asfal;
            natija.Ard = ard;
            natija.Irtifa = irtifa;
            return natija;
        }

        /// <summary>
        /// The UTF-8 byte length of a string, without encoding it anywhere.
        /// </summary>
        /// <param name="nass">The text; null is read as empty.</param>
        /// <returns>How many bytes its UTF-8 form occupies.</returns>
        public static int TulUtf8(string? nass)
        {
            if (string.IsNullOrEmpty(nass))
            {
                return 0;
            }
            return Encoding.UTF8.GetByteCount(nass.AsSpan());
        }

        /// <summary>
        /// Packs the scene, the path and the system into one buffer with unit
        /// separators, for <see cref="HuwiyatMawdi"/>.
        /// </summary>
        private static void Ijma(
            ReadOnlySpan<char> mashhad,
            ReadOnlySpan<char> masar,
            NizamNass nizam,
            Span<byte> hadaf)
        {
            int mawdi = Encoding.UTF8.GetBytes(mashhad, hadaf);
            hadaf[mawdi] = 0x1F;
            mawdi++;
            mawdi += Encoding.UTF8.GetBytes(masar, hadaf.Slice(mawdi));
            hadaf[mawdi] = (byte)nizam;
        }

        /// <summary>
        /// Writes the version line, terminator included.
        /// </summary>
        /// <param name="katib">The destination.</param>
        /// <param name="adadSijillat">How many record lines will follow.</param>
        /// <param name="tam">Whether the session recorded everything it saw.</param>
        /// <param name="adadMuhmal">Sightings refused after the cap.</param>
        /// <exception cref="ArgumentNullException"><paramref name="katib"/> is null.</exception>
        /// <remarks>
        /// The terminator is written as a literal, never through
        /// <see cref="TextWriter.WriteLine()"/>, so the writer's own
        /// <see cref="TextWriter.NewLine"/> — which is the platform's, which on
        /// Windows is a carriage return and a line feed — cannot reach the file.
        /// </remarks>
        public static void AktubRas(TextWriter katib, int adadSijillat, bool tam, long adadMuhmal)
        {
            if (katib is null)
            {
                throw new ArgumentNullException(nameof(katib));
            }
            katib.Write(Wasm);
            katib.Write(Fasil);
            AktubSaheeh(katib, IsdarSigha);
            katib.Write(Fasil);
            AktubSaheeh(katib, AdadHuqul);
            katib.Write(Fasil);
            AktubSaheeh(katib, adadSijillat);
            katib.Write(Fasil);
            katib.Write(tam ? '1' : '0');
            katib.Write(Fasil);
            AktubSaheeh(katib, adadMuhmal);
            katib.Write(NihayatSatr);
        }

        /// <summary>
        /// Writes one record as one line, terminator included.
        /// </summary>
        /// <param name="katib">The destination.</param>
        /// <param name="sijill">The record.</param>
        /// <exception cref="ArgumentNullException"><paramref name="katib"/> is null.</exception>
        /// <remarks>
        /// The number formatting allocates a string per numeric field, which is
        /// deliberate and confined here: this is the flush path, it runs when a
        /// session is written out rather than when a frame is drawn, and the
        /// allocation-free guarantee belongs to <see cref="JalsatIltiqat.Sajjil"/>.
        /// Every float goes through the round-trip format under the invariant
        /// culture — round-trip so that reading the file back recovers the same
        /// bits, invariant so that a session captured on a machine whose decimal
        /// separator is a comma does not write <c>12,5</c> into a field the
        /// reader parses with a dot.
        /// </remarks>
        public static void AktubSijill(TextWriter katib, in SijillIltiqat sijill)
        {
            if (katib is null)
            {
                throw new ArgumentNullException(nameof(katib));
            }

            AktubSitta(katib, sijill.Huwiya);
            katib.Write(Fasil);
            AktubSaheeh(katib, sijill.Tasalsul);
            katib.Write(Fasil);
            AktubSaheeh(katib, sijill.Itar);
            katib.Write(Fasil);
            AktubSaheeh(katib, sijill.Adad);
            katib.Write(Fasil);
            AktubSaheeh(katib, (long)sijill.Nizam);
            katib.Write(Fasil);
            AktubSaheeh(katib, sijill.Alam());
            katib.Write(Fasil);
            AktubKasr(katib, sijill.Hajm);
            katib.Write(Fasil);
            AktubKasr(katib, sijill.Mustatil.Yasar);
            katib.Write(Fasil);
            AktubKasr(katib, sijill.Mustatil.Asfal);
            katib.Write(Fasil);
            AktubKasr(katib, sijill.Mustatil.Ard);
            katib.Write(Fasil);
            AktubKasr(katib, sijill.Mustatil.Irtifa);
            katib.Write(Fasil);
            AktubSaheeh(katib, sijill.TulAsl);
            katib.Write(Fasil);
            AktubHaql(katib, sijill.Mashhad);
            katib.Write(Fasil);
            AktubHaql(katib, sijill.Masar);
            katib.Write(Fasil);
            if (sijill.Hassas)
            {
                AktubSitta(katib, sijill.BasmatAsl);
            }
            else
            {
                AktubHaql(katib, sijill.Asl);
            }
            katib.Write(NihayatSatr);
        }

        /// <summary>
        /// One record as one line, without its terminator.
        /// </summary>
        /// <param name="sijill">The record.</param>
        /// <returns>The line.</returns>
        /// <remarks>
        /// For a caller whose transport is a message rather than a file — a
        /// capture streamed to Studio over <c>barid</c> while the session is
        /// still running. Same encoder, so a streamed row and a written row
        /// cannot differ.
        /// </remarks>
        public static string SatrMin(in SijillIltiqat sijill)
        {
            StringWriter katib = new StringWriter(CultureInfo.InvariantCulture);
            AktubSijill(katib, in sijill);
            string satr = katib.ToString();
            return satr.EndsWith(NihayatSatr, StringComparison.Ordinal)
                ? satr.Substring(0, satr.Length - NihayatSatr.Length)
                : satr;
        }

        /// <summary>
        /// Writes one free-text field with the escaping the format defines.
        /// </summary>
        /// <param name="katib">The destination.</param>
        /// <param name="nass">The field; null and empty both write nothing.</param>
        /// <exception cref="ArgumentNullException"><paramref name="katib"/> is null.</exception>
        public static void AktubHaql(TextWriter katib, string? nass)
        {
            if (katib is null)
            {
                throw new ArgumentNullException(nameof(katib));
            }
            if (string.IsNullOrEmpty(nass))
            {
                return;
            }

            string n = nass!;
            for (int i = 0; i < n.Length; i++)
            {
                char h = n[i];
                switch (h)
                {
                    case '\\':
                        katib.Write("\\\\");
                        continue;
                    case '\t':
                        katib.Write("\\t");
                        continue;
                    case '\n':
                        katib.Write("\\n");
                        continue;
                    case '\r':
                        katib.Write("\\r");
                        continue;
                    default:
                        break;
                }

                if (h < ' ' || h == '\u007F')
                {
                    katib.Write("\\x");
                    katib.Write(Khanat[(h >> 4) & 0xF]);
                    katib.Write(Khanat[h & 0xF]);
                    continue;
                }

                if (char.IsHighSurrogate(h))
                {
                    if (i + 1 < n.Length && char.IsLowSurrogate(n[i + 1]))
                    {
                        katib.Write(h);
                        katib.Write(n[i + 1]);
                        i++;
                        continue;
                    }
                    AktubWahda(katib, h);
                    continue;
                }
                if (char.IsLowSurrogate(h))
                {
                    AktubWahda(katib, h);
                    continue;
                }

                katib.Write(h);
            }
        }

        /// <summary>
        /// Undoes <see cref="AktubHaql"/>. The inverse is written out here, in
        /// code, so the escape rule has one definition rather than a definition
        /// and a description of it.
        /// </summary>
        /// <param name="haql">The field as it appears between two separators.</param>
        /// <returns>The original text.</returns>
        /// <exception cref="KhataTaarib">
        /// The field ends in a lone backslash, or a backslash introduces a
        /// sequence the format does not define. Refused rather than guessed:
        /// guessing turns one corrupt row into a plausible one.
        /// </exception>
        public static string Fukk(ReadOnlySpan<char> haql)
        {
            if (haql.Length == 0)
            {
                return string.Empty;
            }
            if (haql.IndexOf('\\') < 0)
            {
                return new string(haql);
            }

            StringBuilder bani = new StringBuilder(haql.Length);
            for (int i = 0; i < haql.Length; i++)
            {
                char h = haql[i];
                if (h != '\\')
                {
                    bani.Append(h);
                    continue;
                }
                if (i + 1 >= haql.Length)
                {
                    throw RafdSatr(
                        "a field ends in a lone backslash",
                        "ينتهي حقل بشرطة مائلة وحدها");
                }

                char baad = haql[i + 1];
                i++;
                switch (baad)
                {
                    case '\\':
                        bani.Append('\\');
                        continue;
                    case 't':
                        bani.Append('\t');
                        continue;
                    case 'n':
                        bani.Append('\n');
                        continue;
                    case 'r':
                        bani.Append('\r');
                        continue;
                    case 'x':
                        bani.Append((char)Sitta(haql, ref i, 2));
                        continue;
                    case 'u':
                        bani.Append((char)Sitta(haql, ref i, 4));
                        continue;
                    default:
                        throw RafdSatr(
                            $"a backslash introduces the undefined sequence \\{baad}",
                            $"شرطة مائلة تبدأ تسلسلًا غير معرَّف \\{baad}");
                }
            }
            return bani.ToString();
        }

        /// <summary>
        /// Parses the version line and refuses anything this build cannot read.
        /// </summary>
        /// <param name="satr">The first line of a capture file, without its terminator.</param>
        /// <returns>What the line declares.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="satr"/> is null.</exception>
        /// <exception cref="KhataTaarib">
        /// The line is not a capture version line, or declares a format version
        /// or a field count this build does not read. This is the refusal the
        /// version marker exists for, and it happens before one record is
        /// parsed.
        /// </exception>
        public static RasIltiqat IqraRas(string satr)
        {
            if (satr is null)
            {
                throw new ArgumentNullException(nameof(satr));
            }

            Span<int> bidayat = stackalloc int[AdadHuqulRas];
            Span<int> atwal = stackalloc int[AdadHuqulRas];
            int adad = Iqsim(satr, bidayat, atwal);
            if (adad != AdadHuqulRas)
            {
                throw Rafd(
                    4302, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                    $"سطر إصدار ملف الالتقاط يحوي {adad} حقلًا والمتوقع {AdadHuqulRas}؛ "
                    + "الملف ليس ملف التقاط أو بُتر أوله.",
                    $"The capture file's version line has {adad} fields; {AdadHuqulRas} were "
                    + "expected. The file is not a capture, or its start was truncated.");
            }
            if (!satr.AsSpan(bidayat[0], atwal[0]).SequenceEqual(Wasm.AsSpan()))
            {
                throw Rafd(
                    4302, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                    $"سطر إصدار ملف الالتقاط يبدأ بـ \"{satr.Substring(bidayat[0], atwal[0])}\" "
                    + $"بدل \"{Wasm}\".",
                    $"The capture file's version line starts with "
                    + $"\"{satr.Substring(bidayat[0], atwal[0])}\" rather than \"{Wasm}\".");
            }

            int isdar = (int)Saheeh(satr, bidayat[1], atwal[1], "isdar", int.MaxValue);
            if (isdar != IsdarSigha)
            {
                bool ahdath = isdar > IsdarSigha;
                throw Rafd(
                    4302, Ramz.IsdarGhayrMutawafiq,
                    ahdath ? Khutwa.TahdithTaarib : Khutwa.FathTashkhis,
                    $"إصدار صيغة الالتقاط {isdar}، وهذا البناء يقرأ الإصدار {IsdarSigha} فقط.",
                    $"The capture format version is {isdar}; this build reads version "
                    + $"{IsdarSigha} only.");
            }

            int adadHuqul = (int)Saheeh(satr, bidayat[2], atwal[2], "adad_huqul", int.MaxValue);
            if (adadHuqul != AdadHuqul)
            {
                throw Rafd(
                    4302, Ramz.IsdarGhayrMutawafiq, Khutwa.TahdithTaarib,
                    $"سطر الإصدار يعلن {adadHuqul} حقلًا لكل سجل، وهذا البناء يقرأ "
                    + $"{AdadHuqul} حقلًا، والإصدار نفسه في الملفين.",
                    $"The version line declares {adadHuqul} fields per record and this build "
                    + $"reads {AdadHuqul}, at the same format version.");
            }

            int adadSijillat =
                (int)Saheeh(satr, bidayat[3], atwal[3], "adad_sijillat", int.MaxValue);
            long tamKhaam = Saheeh(satr, bidayat[4], atwal[4], "tam", 1);
            long adadMuhmal =
                Saheeh(satr, bidayat[5], atwal[5], "adad_muhmal", long.MaxValue);
            return new RasIltiqat(isdar, adadHuqul, adadSijillat, tamKhaam != 0, adadMuhmal);
        }

        /// <summary>
        /// Parses one record line.
        /// </summary>
        /// <param name="satr">The line, without its terminator.</param>
        /// <returns>The record.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="satr"/> is null.</exception>
        /// <exception cref="KhataTaarib">
        /// The line has the wrong number of fields, or a field does not parse.
        /// Refused by name rather than skipped, because a capture with rows
        /// quietly dropped is the same failure the record cap refuses to
        /// commit: a patch missing strings nobody knows are missing.
        /// </exception>
        public static SijillIltiqat IqraSatr(string satr)
        {
            if (satr is null)
            {
                throw new ArgumentNullException(nameof(satr));
            }

            Span<int> bidayat = stackalloc int[AdadHuqul];
            Span<int> atwal = stackalloc int[AdadHuqul];
            int adad = Iqsim(satr, bidayat, atwal);
            if (adad != AdadHuqul)
            {
                throw Rafd(
                    4303, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                    $"سطر في ملف الالتقاط يحوي {adad} حقلًا والمتوقع {AdadHuqul}.",
                    $"A line in the capture file has {adad} fields; {AdadHuqul} were expected.");
            }

            SijillIltiqat sijill = default;
            sijill.Huwiya = Sittaashar(satr, bidayat[0], atwal[0], "huwiya");
            sijill.Tasalsul = Saheeh(satr, bidayat[1], atwal[1], "tasalsul", long.MaxValue);
            sijill.Itar = Saheeh(satr, bidayat[2], atwal[2], "itar", long.MaxValue);
            sijill.Adad = Saheeh(satr, bidayat[3], atwal[3], "adad", long.MaxValue);
            sijill.Nizam = (NizamNass)Saheeh(satr, bidayat[4], atwal[4], "nizam", byte.MaxValue);

            uint alam = (uint)Saheeh(satr, bidayat[5], atwal[5], "alam", uint.MaxValue);
            sijill.Yaltaff = (alam & AlamatSijill.Iltifaf) != 0;
            sijill.Hassas = (alam & AlamatSijill.Hassas) != 0;

            sijill.Hajm = Kasr(satr, bidayat[6], atwal[6], "hajm");
            sijill.Mustatil.Yasar = Kasr(satr, bidayat[7], atwal[7], "yasar");
            sijill.Mustatil.Asfal = Kasr(satr, bidayat[8], atwal[8], "asfal");
            sijill.Mustatil.Ard = Kasr(satr, bidayat[9], atwal[9], "ard");
            sijill.Mustatil.Irtifa = Kasr(satr, bidayat[10], atwal[10], "irtifa");
            sijill.TulAsl = (int)Saheeh(satr, bidayat[11], atwal[11], "tul", int.MaxValue);
            sijill.Mashhad = Fukk(satr.AsSpan(bidayat[12], atwal[12]));
            sijill.Masar = Fukk(satr.AsSpan(bidayat[13], atwal[13]));

            if (sijill.Hassas)
            {
                sijill.Asl = string.Empty;
                sijill.BasmatAsl = Sittaashar(satr, bidayat[14], atwal[14], "basma");
            }
            else
            {
                sijill.Asl = Fukk(satr.AsSpan(bidayat[14], atwal[14]));
                sijill.BasmatAsl = 0;
            }
            return sijill;
        }

        /// <summary>
        /// Reads a whole capture file into the caller's list.
        /// </summary>
        /// <param name="qari">The source, positioned at the version line.</param>
        /// <param name="hadaf">Where the records go; appended to, never cleared.</param>
        /// <returns>What the version line declared, so the caller can act on
        /// <see cref="RasIltiqat.Tam"/>.</returns>
        /// <exception cref="ArgumentNullException">Either argument is null.</exception>
        /// <exception cref="KhataTaarib">
        /// The version line is unreadable, a record line is malformed, or the
        /// file holds fewer records than its own first line declares.
        /// </exception>
        /// <remarks>
        /// The declared record count is authoritative: exactly that many lines
        /// are read, and running out first is a refusal rather than a short
        /// list. A capture cut off by a killed process is otherwise
        /// indistinguishable from a complete one, and the whole reason the cap
        /// refuses to evict is that a translator must never be handed a patch
        /// whose missing strings are invisible. Anything after the declared
        /// count is ignored, so a trailing blank line is harmless.
        /// </remarks>
        public static RasIltiqat Iqra(TextReader qari, List<SijillIltiqat> hadaf)
        {
            if (qari is null)
            {
                throw new ArgumentNullException(nameof(qari));
            }
            if (hadaf is null)
            {
                throw new ArgumentNullException(nameof(hadaf));
            }

            string? awwal = qari.ReadLine();
            if (awwal is null)
            {
                throw Rafd(
                    4302, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                    "ملف الالتقاط فارغ؛ لا سطر إصدار فيه.",
                    "The capture file is empty; it has no version line.");
            }

            RasIltiqat ras = IqraRas(awwal);
            for (int i = 0; i < ras.AdadSijillat; i++)
            {
                string? satr = qari.ReadLine();
                if (satr is null)
                {
                    throw Rafd(
                        4304, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                        $"ملف الالتقاط يعلن {ras.AdadSijillat} سجلًا ويحوي {i}؛ الملف مبتور.",
                        $"The capture file declares {ras.AdadSijillat} records and holds {i}; "
                        + "it is truncated.");
                }
                hadaf.Add(IqraSatr(satr));
            }
            return ras;
        }

        /// <summary>Writes a 64-bit value as exactly sixteen lowercase hex digits.</summary>
        private static void AktubSitta(TextWriter katib, ulong qeema)
        {
            Span<char> khanat = stackalloc char[16];
            for (int i = 15; i >= 0; i--)
            {
                khanat[i] = Khanat[(int)(qeema & 0xF)];
                qeema >>= 4;
            }
            katib.Write(khanat);
        }

        /// <summary>Writes an integer in the invariant culture.</summary>
        private static void AktubSaheeh(TextWriter katib, long qeema)
        {
            katib.Write(qeema.ToString(CultureInfo.InvariantCulture));
        }

        /// <summary>Writes a float round-trippably in the invariant culture.</summary>
        private static void AktubKasr(TextWriter katib, float qeema)
        {
            katib.Write(Sahih(qeema).ToString("R", CultureInfo.InvariantCulture));
        }

        /// <summary>Writes one code unit as <c>\uHHHH</c>.</summary>
        private static void AktubWahda(TextWriter katib, char wahda)
        {
            katib.Write("\\u");
            katib.Write(Khanat[(wahda >> 12) & 0xF]);
            katib.Write(Khanat[(wahda >> 8) & 0xF]);
            katib.Write(Khanat[(wahda >> 4) & 0xF]);
            katib.Write(Khanat[wahda & 0xF]);
        }

        /// <summary>
        /// Records where each tab-separated field starts and how long it is, and
        /// returns how many fields the line has — which the caller compares
        /// against what it expects rather than trusting.
        /// </summary>
        private static int Iqsim(string satr, Span<int> bidayat, Span<int> atwal)
        {
            int adad = 0;
            int bidaya = 0;
            for (int i = 0; i <= satr.Length; i++)
            {
                if (i != satr.Length && satr[i] != Fasil)
                {
                    continue;
                }
                if (adad < bidayat.Length)
                {
                    bidayat[adad] = bidaya;
                    atwal[adad] = i - bidaya;
                }
                adad++;
                bidaya = i + 1;
            }
            return adad;
        }

        /// <summary>Reads exactly sixteen hex digits, refusing anything else.</summary>
        private static ulong Sittaashar(string satr, int bidaya, int tul, string haql)
        {
            if (tul != 16)
            {
                throw RafdHaql(
                    haql, satr.Substring(bidaya, tul), "16 hex digits", "١٦ خانة ست عشرية");
            }
            ulong qeema = 0;
            for (int i = 0; i < 16; i++)
            {
                int khana = Khanat.IndexOf(char.ToLowerInvariant(satr[bidaya + i]));
                if (khana < 0)
                {
                    throw RafdHaql(
                        haql, satr.Substring(bidaya, tul), "16 hex digits", "١٦ خانة ست عشرية");
                }
                qeema = (qeema << 4) | (uint)khana;
            }
            return qeema;
        }

        /// <summary>Reads a non-negative decimal integer no larger than a bound.</summary>
        private static long Saheeh(string satr, int bidaya, int tul, string haql, ulong aqsa)
        {
            ReadOnlySpan<char> maqta = satr.AsSpan(bidaya, tul);
            if (!ulong.TryParse(
                    maqta, NumberStyles.None, CultureInfo.InvariantCulture, out ulong qeema)
                || qeema > aqsa)
            {
                throw RafdHaql(
                    haql, new string(maqta),
                    $"a whole number from 0 to {aqsa}", $"عددًا صحيحًا بين ٠ و{aqsa}");
            }
            return (long)qeema;
        }

        /// <summary>Reads a finite float in the invariant culture.</summary>
        private static float Kasr(string satr, int bidaya, int tul, string haql)
        {
            ReadOnlySpan<char> maqta = satr.AsSpan(bidaya, tul);
            if (!float.TryParse(
                    maqta, NumberStyles.Float, CultureInfo.InvariantCulture, out float qeema)
                || !float.IsFinite(qeema))
            {
                throw RafdHaql(
                    haql, new string(maqta), "a finite decimal number", "عددًا عشريًا منتهيًا");
            }
            return qeema;
        }

        /// <summary>Names the field, what was there, and what belonged there.</summary>
        private static KhataTaarib RafdHaql(
            string haql, string mawjud, string mutawaqqaInjilizi, string mutawaqqaArabi)
        {
            return Rafd(
                4303, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                $"حقل \"{haql}\" في ملف الالتقاط يحوي \"{mawjud}\" والمتوقع {mutawaqqaArabi}.",
                $"Field \"{haql}\" of the capture file holds \"{mawjud}\"; "
                + $"{mutawaqqaInjilizi} were expected.");
        }

        /// <summary>Refuses a line whose escaping does not follow the format.</summary>
        private static KhataTaarib RafdSatr(string injilizi, string arabi)
        {
            return Rafd(
                4303, Ramz.QeemaBatila, Khutwa.FathTashkhis,
                $"ترميز الهروب في ملف الالتقاط غير صالح: {arabi}.",
                $"The escaping in the capture file is not valid: {injilizi}.");
        }

        /// <summary>Reads the hex digits of an <c>\xHH</c> or <c>\uHHHH</c> escape.</summary>
        private static int Sitta(ReadOnlySpan<char> haql, ref int i, int adad)
        {
            if (i + adad >= haql.Length)
            {
                throw RafdSatr(
                    $"an escape needs {adad} hex digits and the field ends first",
                    $"تسلسل هروب يحتاج {adad} خانة والحقل ينتهي قبلها");
            }
            int qeema = 0;
            for (int j = 1; j <= adad; j++)
            {
                int khana = Khanat.IndexOf(char.ToLowerInvariant(haql[i + j]));
                if (khana < 0)
                {
                    throw RafdSatr(
                        $"an escape needs {adad} hex digits", $"تسلسل هروب يحتاج {adad} خانة");
                }
                qeema = (qeema << 4) | khana;
            }
            i += adad;
            return qeema;
        }

        /// <summary>
        /// Builds the refusal this file raises, with its permanent code from the
        /// adapters' own <c>4300</c> block, both sentences, and the one next
        /// action — so a capture failure reads identically in a BepInEx log and
        /// in Studio.
        /// </summary>
        internal static KhataTaarib Rafd(
            int raqm, int halat, Khutwa khutwa, string arabi, string injilizi)
        {
            string ramz = "TAARIB-E-" + raqm.ToString("D4", CultureInfo.InvariantCulture);
            return new KhataTaarib(halat, ramz, arabi, injilizi, khutwa);
        }
    }

    /// <summary>
    /// One capture session: the strings seen so far, deduplicated, bounded, and
    /// writable to the line format <see cref="Iltiqat"/> defines.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Cost per sighting.</b> A string that has been seen before costs one
    /// uncontended monitor acquisition, one lookup in a dictionary keyed on a
    /// 64-bit number, and a handful of field writes. Nothing is compared as a
    /// string — the identity is compared instead, which is a single integer
    /// comparison rather than a walk over two character arrays that are equal —
    /// and nothing is allocated at all. That is what makes capture cheap enough
    /// to leave switched on for a whole play session, which is the only way it
    /// is useful: a translator is not going to replay the game to find the one
    /// menu the capture missed.
    /// </para>
    /// <para>
    /// The dictionary's own hashing does no work, deliberately. The key is
    /// already the first eight bytes of a BLAKE3 hash, so it is uniformly
    /// distributed by construction and the default comparer for
    /// <see cref="ulong"/> — which folds the two halves together — is exactly
    /// right. A custom comparer here would add an interface call to every
    /// lookup for nothing.
    /// </para>
    /// <para>
    /// <b>The cap, and why it stops rather than evicts.</b>
    /// <see cref="HaddIftiradi"/> distinct records is the default bound, and
    /// when it is reached the session stops recording new strings, sets
    /// <see cref="Mumtali"/>, builds <see cref="KhataImtila"/>, and counts what
    /// it turns away in <see cref="AdadMuhmal"/>. It does not evict. An evicting
    /// cache would keep running and keep looking healthy, and the strings it
    /// dropped would be absent from the capture, absent from the string table,
    /// absent from the compiled patch, and absent from the overflow report — so
    /// the first person to learn about them would be a player who reached that
    /// menu and found it in English. A capture that stops is a capture that says
    /// what it did not see, on the first line of its own file, and the
    /// translator plays the rest of the game in a second session.
    /// </para>
    /// <para>
    /// <b>Thread safety, member by member.</b> Unity calls into the takeover
    /// from the thread that draws; a session may be flushed from another — a
    /// hotkey handler, an idle worker, a shutdown hook — and the two overlap.
    /// </para>
    /// <list type="bullet">
    /// <item><description>
    /// <see cref="Sajjil"/> is safe from any thread and is the only member
    /// intended for the drawing thread. It holds the record lock for the
    /// duration of one lookup and never blocks on a flush.
    /// </description></item>
    /// <item><description>
    /// <see cref="Jid"/>, <see cref="Nusakh"/> and every property are safe from
    /// any thread and hold the record lock briefly.
    /// </description></item>
    /// <item><description>
    /// <see cref="Aktub(TextWriter)"/> and <see cref="Afrigh"/> are safe from
    /// any thread and are serialised against each other by a second lock, so a
    /// flush cannot be emptied out from under itself. Neither blocks the
    /// drawing thread for more than the time it takes to copy one batch of
    /// records, because a flush that held the record lock for the length of a
    /// disk write would freeze the game for as long as the write took.
    /// </description></item>
    /// </list>
    /// <para>
    /// The two locks are always taken in the same order — the flush lock, then
    /// the record lock — and <see cref="Sajjil"/> takes only the second, so
    /// there is no cycle and no deadlock to find later.
    /// </para>
    /// <para>
    /// A flush therefore copies the session in batches, and a record written
    /// early in a file can be a few milliseconds older than one written late.
    /// That skew is real and it is not worth removing: it is milliseconds
    /// inside a session that spans minutes, it can only affect an occurrence
    /// count and a rectangle, and the alternative costs the player a visible
    /// freeze every time they press the flush key.
    /// </para>
    /// </remarks>
    public sealed class JalsatIltiqat
    {
        /// <summary>
        /// The default cap on distinct records: 131,072.
        /// </summary>
        /// <remarks>
        /// Chosen from both ends. It is comfortably above the distinct-string
        /// count of the largest games anyone captures — a hundred-hour role
        /// playing game's entire script, menus and item descriptions included,
        /// is tens of thousands of strings, so an ordinary session never comes
        /// near it and the cap is invisible. And it bounds what the session
        /// costs the game: the record array at this size is about twelve
        /// megabytes, and the scene and path strings it holds references to add
        /// tens more, which a game process can spare for a session someone
        /// deliberately started and which is far below the point where the
        /// garbage collector's work on it would show up as a stutter in the
        /// play the capture is recording.
        /// </remarks>
        public const int HaddIftiradi = 131072;

        /// <summary>
        /// The largest cap a session may be constructed with. Present so that
        /// the record array's doubling cannot overflow an <see cref="int"/>, and
        /// so that a configuration file with an extra digit in it is a refusal
        /// at startup rather than an allocation failure mid-session.
        /// </summary>
        public const int AqsaHadd = 1 << 22;

        /// <summary>The record array's initial capacity.</summary>
        public const int SiaBidaya = 1024;

        /// <summary>
        /// How many records a flush copies out per batch. Deliberately small
        /// enough that the copy stays under the large-object heap threshold, so
        /// writing a session does not leave a multi-megabyte array behind for
        /// the game's collector to deal with afterwards.
        /// </summary>
        public const int SiatDufa = 512;

        private readonly object qifl = new object();
        private readonly object qiflKitaba = new object();
        private readonly Dictionary<ulong, int> fahras;
        private SijillIltiqat[] sijillat;
        private int adad;
        private long tasalsul;
        private long muhmal;
        private bool mumtali;
        private KhataTaarib? khataImtila;

        /// <summary>Starts a session with the default cap.</summary>
        public JalsatIltiqat()
            : this(HaddIftiradi)
        {
        }

        /// <summary>Starts a session with an explicit cap.</summary>
        /// <param name="hadd">
        /// The most distinct records to keep, between one and
        /// <see cref="AqsaHadd"/>.
        /// </param>
        /// <exception cref="ArgumentOutOfRangeException">
        /// <paramref name="hadd"/> is outside that range.
        /// </exception>
        public JalsatIltiqat(int hadd)
        {
            if (hadd < 1 || hadd > AqsaHadd)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(hadd), hadd, $"A capture cap is between 1 and {AqsaHadd}.");
            }
            Hadd = hadd;
            int sia = hadd < SiaBidaya ? hadd : SiaBidaya;
            sijillat = new SijillIltiqat[sia];
            fahras = new Dictionary<ulong, int>(sia);
        }

        /// <summary>The cap this session was constructed with.</summary>
        public int Hadd { get; }

        /// <summary>How many distinct strings the session holds.</summary>
        public int Adad
        {
            get
            {
                lock (qifl)
                {
                    return adad;
                }
            }
        }

        /// <summary>
        /// How many sightings were turned away after the cap was reached. Not a
        /// count of missing strings — most of those sightings were repeats of
        /// strings already in the session — but the only measure the session has
        /// of how much play happened after it stopped listening.
        /// </summary>
        public long AdadMuhmal
        {
            get
            {
                lock (qifl)
                {
                    return muhmal;
                }
            }
        }

        /// <summary>Whether the session reached its cap and stopped recording.</summary>
        public bool Mumtali
        {
            get
            {
                lock (qifl)
                {
                    return mumtali;
                }
            }
        }

        /// <summary>
        /// Whether the session recorded everything it was offered. The negation
        /// of <see cref="Mumtali"/>, named because it is what the version line
        /// carries and what a reader acts on.
        /// </summary>
        public bool Tam => !Mumtali;

        /// <summary>
        /// The refusal built at the moment the cap was reached, or <c>null</c>
        /// while the session is still recording.
        /// </summary>
        /// <remarks>
        /// Built and handed back rather than thrown. The moment it becomes true
        /// is inside a takeover point on the drawing thread, and an exception
        /// escaping from there would propagate into the game's own render path,
        /// where the outcome is a game that stops drawing rather than a game
        /// that stops capturing. So the session records the fact, keeps
        /// rendering, and leaves the reporting to whoever asks: log it once when
        /// <see cref="Sajjil"/> first returns <see cref="NatijatSajjil.Mutawaqqif"/>,
        /// and show it beside the session in the interface. It carries the
        /// permanent code, both sentences and the next action, so that entry in
        /// a BepInEx log reads exactly as it does in Studio.
        /// </remarks>
        public KhataTaarib? KhataImtila
        {
            get
            {
                lock (qifl)
                {
                    return khataImtila;
                }
            }
        }

        /// <summary>
        /// Records one sighting: creates a record for a string the session has
        /// not seen, or folds it into the one that exists.
        /// </summary>
        /// <param name="talab">The sighting, as the adapter observed it.</param>
        /// <returns>What was done with it.</returns>
        /// <remarks>
        /// <para>
        /// <b>Allocation.</b> A repeat sighting allocates nothing whatsoever. A
        /// first sighting allocates only the dictionary entry, and, on the calls
        /// where the record array happens to be full, one array twice the size
        /// of the last — which happens a logarithmic number of times across a
        /// whole session, not per frame. The strings are stored by reference and
        /// never copied, which is why <see cref="TalabIltiqat.Masar"/> must be a
        /// string the adapter already had rather than one it built for this call.
        /// </para>
        /// <para>
        /// <b>What a repeat sighting may change.</b> The occurrence count always.
        /// The geometry group — rectangle, size, wrapping, system, path, scene —
        /// only when the rectangle is larger than the one on file, and then all
        /// of it together, so the record keeps describing a draw that really
        /// happened. For a redacted record, the length and the hash when the
        /// text got longer. Nothing else, ever: the sequence number and the
        /// frame stay at the first sighting's, because their whole use is
        /// reconstructing the order in which the player met these strings.
        /// </para>
        /// <para>
        /// <b>What it does not do.</b> It does not hash the source text. The
        /// adapter arrives holding the identity because it needed one to ask the
        /// patch for a translation, and capture reuses that number rather than
        /// computing a second one — so leaving capture on adds no hashing to the
        /// frame at all. The one exception is a redacted point whose text has
        /// grown, which rehashes; sensitive points are a handful per game and a
        /// growth is rarer still.
        /// </para>
        /// </remarks>
        public NatijatSajjil Sajjil(in TalabIltiqat talab)
        {
            MustatilIltiqat mustatil = Iltiqat.Sahih(in talab.Mustatil);
            float hajm = Iltiqat.Sahih(talab.Hajm);

            lock (qifl)
            {
                if (fahras.TryGetValue(talab.Huwiya, out int mawdi))
                {
                    ref SijillIltiqat qadeem = ref sijillat[mawdi];
                    if (qadeem.Adad < long.MaxValue)
                    {
                        qadeem.Adad++;
                    }
                    if (Iltiqat.Aakbar(in mustatil, in qadeem.Mustatil))
                    {
                        qadeem.Mustatil = mustatil;
                        qadeem.Hajm = hajm;
                        qadeem.Yaltaff = talab.Yaltaff;
                        qadeem.Nizam = talab.Nizam;
                        qadeem.Masar = talab.Masar ?? string.Empty;
                        qadeem.Mashhad = talab.Mashhad ?? string.Empty;
                    }
                    if (qadeem.Hassas)
                    {
                        int tul = Iltiqat.TulUtf8(talab.Asl);
                        if (tul > qadeem.TulAsl)
                        {
                            qadeem.TulAsl = tul;
                            qadeem.BasmatAsl = Iltiqat.Huwiya(talab.Asl ?? string.Empty);
                        }
                    }
                    return NatijatSajjil.Mukarrar;
                }

                if (mumtali)
                {
                    if (muhmal < long.MaxValue)
                    {
                        muhmal++;
                    }
                    return NatijatSajjil.Mutawaqqif;
                }
                if (adad >= Hadd)
                {
                    mumtali = true;
                    khataImtila = RafdImtila();
                    muhmal++;
                    return NatijatSajjil.Mutawaqqif;
                }

                if (adad == sijillat.Length)
                {
                    Namm();
                }

                ref SijillIltiqat jadeed = ref sijillat[adad];
                jadeed.Huwiya = talab.Huwiya;
                jadeed.Hassas = talab.Hassas;
                jadeed.Nizam = talab.Nizam;
                jadeed.Masar = talab.Masar ?? string.Empty;
                jadeed.Mashhad = talab.Mashhad ?? string.Empty;
                jadeed.Mustatil = mustatil;
                jadeed.Hajm = hajm;
                jadeed.Yaltaff = talab.Yaltaff;
                jadeed.Itar = talab.Itar;
                jadeed.Tasalsul = tasalsul;
                jadeed.Adad = 1;
                jadeed.TulAsl = Iltiqat.TulUtf8(talab.Asl);
                if (talab.Hassas)
                {
                    jadeed.Asl = string.Empty;
                    jadeed.BasmatAsl = Iltiqat.Huwiya(talab.Asl ?? string.Empty);
                }
                else
                {
                    jadeed.Asl = talab.Asl ?? string.Empty;
                    jadeed.BasmatAsl = 0;
                }

                fahras.Add(talab.Huwiya, adad);
                adad++;
                tasalsul++;
                return NatijatSajjil.Jadeed;
            }
        }

        /// <summary>
        /// The record for one identity, if the session holds it.
        /// </summary>
        /// <param name="huwiya">The identity to look for.</param>
        /// <param name="sijill">A copy of the record, or <c>default</c>.</param>
        /// <returns>Whether it was found.</returns>
        /// <remarks>
        /// A copy, not a reference. Handing out a reference into the record
        /// array would let a caller keep it across a growth, after which it
        /// points into the old array and every write to it is silently lost.
        /// </remarks>
        public bool Jid(ulong huwiya, out SijillIltiqat sijill)
        {
            lock (qifl)
            {
                if (fahras.TryGetValue(huwiya, out int mawdi))
                {
                    sijill = sijillat[mawdi];
                    return true;
                }
            }
            sijill = default;
            return false;
        }

        /// <summary>
        /// Copies a run of records into the caller's buffer, in file order.
        /// </summary>
        /// <param name="mawdi">The first record to copy, counting from zero.</param>
        /// <param name="hadaf">Where they go; as many as fit are copied.</param>
        /// <returns>
        /// How many were copied; zero when <paramref name="mawdi"/> is past the end.
        /// </returns>
        /// <exception cref="ArgumentOutOfRangeException">
        /// <paramref name="mawdi"/> is negative.
        /// </exception>
        /// <remarks>
        /// The snapshot primitive a flush is built on, and the one a caller
        /// streaming a session elsewhere should use. Records are only ever
        /// appended, never moved or removed, so an index taken from one call is
        /// still the same record on the next.
        /// </remarks>
        public int Nusakh(int mawdi, Span<SijillIltiqat> hadaf)
        {
            if (mawdi < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(mawdi), mawdi, "Not negative.");
            }
            lock (qifl)
            {
                if (mawdi >= adad || hadaf.IsEmpty)
                {
                    return 0;
                }
                int mutah = adad - mawdi;
                int adadNusakh = mutah < hadaf.Length ? mutah : hadaf.Length;
                new ReadOnlySpan<SijillIltiqat>(sijillat, mawdi, adadNusakh).CopyTo(hadaf);
                return adadNusakh;
            }
        }

        /// <summary>
        /// Writes the whole session in the capture line format: the version
        /// line, then one line per record.
        /// </summary>
        /// <param name="katib">The destination.</param>
        /// <returns>How many record lines were written.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="katib"/> is null.</exception>
        /// <remarks>
        /// The record count and the completeness flag are read once, before the
        /// first row, and the file then carries exactly that many rows. Records
        /// added by the drawing thread while the write is in progress belong to
        /// the next file, which is what the count on line one already says.
        /// </remarks>
        public int Aktub(TextWriter katib)
        {
            if (katib is null)
            {
                throw new ArgumentNullException(nameof(katib));
            }

            lock (qiflKitaba)
            {
                int adadAn;
                bool tamAn;
                long muhmalAn;
                lock (qifl)
                {
                    adadAn = adad;
                    tamAn = !mumtali;
                    muhmalAn = muhmal;
                }

                Iltiqat.AktubRas(katib, adadAn, tamAn, muhmalAn);

                SijillIltiqat[] dufa = new SijillIltiqat[SiatDufa];
                int maktub = 0;
                while (maktub < adadAn)
                {
                    int matlub = adadAn - maktub;
                    if (matlub > SiatDufa)
                    {
                        matlub = SiatDufa;
                    }
                    int nusikh = Nusakh(maktub, new Span<SijillIltiqat>(dufa, 0, matlub));
                    if (nusikh <= 0)
                    {
                        // Unreachable while the flush lock is held, because the
                        // record array only ever grows and Afrigh takes the same
                        // lock. Present so that a defect elsewhere produces a
                        // short file the reader refuses by name, rather than a
                        // loop that never ends inside somebody's game.
                        break;
                    }
                    for (int i = 0; i < nusikh; i++)
                    {
                        Iltiqat.AktubSijill(katib, in dufa[i]);
                    }
                    maktub += nusikh;
                }
                katib.Flush();
                return maktub;
            }
        }

        /// <summary>
        /// Writes the session to a file, UTF-8 with no byte order mark.
        /// </summary>
        /// <param name="masar">Where to write it; an existing file is replaced.</param>
        /// <returns>How many record lines were written.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="masar"/> is null.</exception>
        /// <exception cref="ArgumentException"><paramref name="masar"/> is empty.</exception>
        /// <exception cref="IOException">The file could not be written.</exception>
        /// <remarks>
        /// No byte order mark, because the reader on the other side splits the
        /// first line on tabs and compares the first field against a literal —
        /// and a mark makes that field a string that does not equal itself. The
        /// encoder is constructed here rather than taken from
        /// <see cref="Encoding.UTF8"/>, whose own instance emits one.
        /// </remarks>
        public int Aktub(string masar)
        {
            if (masar is null)
            {
                throw new ArgumentNullException(nameof(masar));
            }
            if (masar.Length == 0)
            {
                throw new ArgumentException("A capture path is not empty.", nameof(masar));
            }

            using (FileStream tayyar = new FileStream(
                masar, FileMode.Create, FileAccess.Write, FileShare.Read))
            using (StreamWriter katib = new StreamWriter(
                tayyar, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false)))
            {
                return Aktub(katib);
            }
        }

        /// <summary>
        /// Empties the session and lets it record again.
        /// </summary>
        /// <remarks>
        /// The only way past a reached cap, and it is destructive: everything
        /// the session holds is gone when it returns, so it must follow a
        /// successful <see cref="Aktub(string)"/> and not precede one. Numbering
        /// restarts at zero, because each file declares its own record count on
        /// its own first line and two files are two sessions, not one split in
        /// half.
        /// </remarks>
        public void Afrigh()
        {
            lock (qiflKitaba)
            {
                lock (qifl)
                {
                    Array.Clear(sijillat, 0, adad);
                    fahras.Clear();
                    adad = 0;
                    tasalsul = 0;
                    muhmal = 0;
                    mumtali = false;
                    khataImtila = null;
                }
            }
        }

        /// <summary>Doubles the record array, never past the cap.</summary>
        private void Namm()
        {
            int sia = sijillat.Length == 0 ? SiaBidaya : sijillat.Length * 2;
            if (sia > Hadd)
            {
                sia = Hadd;
            }
            Array.Resize(ref sijillat, sia);
        }

        /// <summary>
        /// The sentence a translator reads when a session fills, naming the cap,
        /// what was kept, and what to do about it.
        /// </summary>
        private KhataTaarib RafdImtila()
        {
            return Iltiqat.Rafd(
                4301, Ramz.KhataAam, Khutwa.FathTashkhis,
                $"بلغت جلسة الالتقاط حدّها ({Hadd} نصًا متمايزًا) فتوقّفت عن التسجيل، ولم "
                + "يُحذف منها شيء. احفظ الجلسة ثم أفرغها وتابع اللعب في جلسة ثانية؛ "
                + "الإفراغ قبل الحفظ يفقد ما التُقط.",
                $"The capture session reached its cap of {Hadd} distinct strings and stopped "
                + "recording; nothing already captured was discarded. Save the session, empty "
                + "it, and carry on in a second session. Emptying it before saving loses what "
                + "was captured.");
        }
    }
}
