// قاعدة — the signature database: the file that decides which byte pattern is
// tried for which game, and the reader that refuses to believe it uncritically.
//
// The whole point of rung three being data is that supporting a new engine
// version costs a pull request against a JSON file rather than a rebuild of a
// plugin that ships inside somebody's game directory. That is worth having, and
// it has a price: this file is the one place in the IL2CPP adapter where the
// input is a document a human wrote, that a user can open in a text editor, that
// a patch author can extend, and that a third party could substitute. Everything
// below treats it accordingly.
//
// WHAT THE FILE LOOKS LIKE. A schema version, a data version, a description, and
// a list of entries. Each entry is keyed on four things — an engine version
// range, a text-package version range, an architecture and a platform — and
// carries the module its patterns are searched in and the targets it knows. Each
// target names a key (the same string HadafHall.MiftahBasma carries) and one or
// more variants, each a pattern, an offset, its validation predicates, where it
// came from and whether anyone has verified it against a real binary.
//
// THE SCHEMA VERSION IS READ BEFORE ANYTHING ELSE, AND A NEWER ONE IS A REFUSAL.
// Not a warning, not a best-effort parse: a refusal with a sentence telling the
// user to update Taarib. A partial parse of a newer schema is the worst outcome
// available here, because the fields this build cannot see are exactly the ones
// a future version would have added to constrain a pattern — a new predicate
// kind, a module qualifier, an exclusion. Reading the pattern and silently
// dropping the constraint that was added to keep it safe produces a scan that
// looks like it worked and detours the wrong function. So the version gate runs
// on the raw document before a single other field is interpreted.
//
// EVERY FIELD IS VALIDATED, AND UNKNOWN FIELDS ARE REFUSED. Within a schema
// version this build understands, a field name it does not recognise is a typo,
// not a feature: `"izahah": -12` next to a pattern silently leaves the offset at
// zero and resolves twelve bytes into a function, which is a detour installed
// over the middle of an instruction. Refusing the file names the typo and its
// path. This is also why the reader works over JsonDocument rather than
// deserializing into objects — a deserializer's job is to fill in what it can,
// and what this reader needs is something whose job is to object.
//
// STRUCTURAL FAILURES REFUSE THE FILE; A BAD PATTERN REFUSES ITSELF. A missing
// required field, a wrong type, an unreadable version range or an unknown
// platform means the document is not the shape this build reads, and half a
// database is not a safer thing than none. But a single pattern whose text does
// not parse — a stray character in one variant out of two hundred, for a game
// nobody on this machine owns — costs that variant and is reported as a warning
// carried on the loaded database, into the log and into the diagnostics bundle.
// It is never silent, and it never takes the other hundred and ninety-nine
// entries down with it.
//
// AN ABSENT DATABASE IS NOT AN ERROR. A build that ships without basmat.json is
// a build with two rungs instead of three, and that is a legitimate
// configuration: most games never need rung three. So absence returns a rung
// availability of false and one sentence saying so, and Sullam prints that
// sentence once instead of failing identically for every target. A database that
// is present and broken is the opposite — somebody shipped or edited something
// wrong and needs to know — and that refuses loudly.
//
// SPECIFICITY. A lookup returns every candidate that matches the running game,
// most specific first, and never merges them: the caller scans them in order and
// takes the first that resolves uniquely. Specificity is computed from the two
// version ranges — see Diqqa on MadkhalBasmat — with the engine range weighted
// above the text-package range, because the engine version decides the compiler,
// the flags and the whole binary's layout, while the package version decides
// only the shape of one function's source. A pattern keyed to 2021.3.16f1
// exactly is evidence from one build. A pattern keyed to * is a pattern nobody
// has narrowed yet. Trying them in the other order is how a general pattern
// shadows the exact one that was added precisely because the general one was
// wrong for that build.

using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Basmat
{
    /// <summary>What happened when a database was opened.</summary>
    /// <remarks>
    /// Separated from a plain boolean because the caller's response differs:
    /// <see cref="Ghaiba"/> makes rung three unavailable and is normal, while
    /// everything else is a fault somebody has to fix and reaches the log as
    /// one.
    /// </remarks>
    public enum HalatQaida
    {
        /// <summary>Read and validated.</summary>
        Mawjuda = 0,

        /// <summary>
        /// No file at that path. Rung three is unavailable; nothing is wrong.
        /// </summary>
        Ghaiba = 1,

        /// <summary>
        /// The file exists and the operating system would not let it be read, or
        /// it is implausibly large for a database.
        /// </summary>
        LaTuqra = 2,

        /// <summary>The file is not valid JSON, or not the shape this build reads.</summary>
        Talifa = 3,

        /// <summary>
        /// The file declares a schema version newer than this build understands.
        /// Refused rather than partially read.
        /// </summary>
        MukhattatAhdath = 4,
    }

    /// <summary>
    /// The stable identity of the pattern that resolved a target, as a log line,
    /// a diagnostics bundle and a bug report all quote it.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The rendering is <c>key@label/platform-architecture variant N</c>, for
    /// example <c>tmp_generate_text_mesh@2021.3/win-x64 variant 2</c>. Each part
    /// is derived, never free text: the key is the target's key, the label is the
    /// entry's lower bound (or its upper bound written with a leading
    /// <c>&lt;</c>, or <c>*</c>), the platform token comes from
    /// <see cref="Qaida.RamzMinassa"/>, and the variant is the one-based position
    /// of the pattern within its target, in file order.
    /// </para>
    /// <para>
    /// Stable means two things, and both are promises to whoever reads a bug
    /// report six months from now. Reordering variants in the database changes
    /// the numbers, so variants are appended rather than inserted — that rule
    /// lives in the file's own notes. And nothing in this rendering depends on
    /// the machine it ran on, so two users on the same game and the same build
    /// produce the same string.
    /// </para>
    /// </remarks>
    public readonly struct SijillBasma
    {
        /// <summary>Records a match.</summary>
        /// <param name="miftah">The target's key.</param>
        /// <param name="lafta">The entry's version label.</param>
        /// <param name="ramzMinassa">The platform and architecture token.</param>
        /// <param name="badil">The variant's one-based position.</param>
        /// <param name="muarrif">The entry's identifier in the database.</param>
        /// <param name="nassNamat">The pattern text, as the database wrote it.</param>
        /// <param name="unwan">The resolved address, or zero before a scan.</param>
        public SijillBasma(
            string miftah,
            string lafta,
            string ramzMinassa,
            int badil,
            string muarrif,
            string nassNamat,
            IntPtr unwan)
        {
            Miftah = miftah ?? string.Empty;
            Lafta = lafta ?? string.Empty;
            RamzMinassa = ramzMinassa ?? string.Empty;
            Badil = badil;
            Muarrif = muarrif ?? string.Empty;
            NassNamat = nassNamat ?? string.Empty;
            Unwan = unwan;
        }

        /// <summary>The target's key, as <c>HadafHall.MiftahBasma</c> carries it.</summary>
        public string Miftah { get; }

        /// <summary>The entry's label: <c>2021.3</c>, <c>&lt;2019.4</c> or <c>*</c>.</summary>
        public string Lafta { get; }

        /// <summary>The platform and architecture token: <c>win-x64</c>.</summary>
        public string RamzMinassa { get; }

        /// <summary>The variant's one-based position within its target.</summary>
        public int Badil { get; }

        /// <summary>The entry's identifier, for finding it in the file.</summary>
        public string Muarrif { get; }

        /// <summary>The pattern text exactly as the database wrote it.</summary>
        public string NassNamat { get; }

        /// <summary>The resolved address, or zero when this records a candidate.</summary>
        public IntPtr Unwan { get; }

        /// <summary>The same record with an address attached.</summary>
        /// <param name="unwan">The resolved address.</param>
        /// <returns>A copy carrying the address.</returns>
        public SijillBasma Ind(IntPtr unwan)
        {
            return new SijillBasma(Miftah, Lafta, RamzMinassa, Badil, Muarrif, NassNamat, unwan);
        }

        /// <summary>The stable short form quoted everywhere.</summary>
        /// <returns><c>tmp_generate_text_mesh@2021.3/win-x64 variant 2</c>.</returns>
        public string Wasf()
        {
            return string.Format(
                CultureInfo.InvariantCulture,
                "{0}@{1}/{2} variant {3}",
                Miftah,
                Lafta,
                RamzMinassa,
                Badil);
        }

        /// <summary>The sentence the log and the bundle carry on a success.</summary>
        /// <returns>The full line, including the address and the entry.</returns>
        public string Satr()
        {
            string unwan = Unwan == IntPtr.Zero
                ? "no address"
                : "0x" + Unwan.ToInt64().ToString("X", CultureInfo.InvariantCulture);
            return $"resolved by pattern {Wasf()} at {unwan} (entry {Muarrif})";
        }

        /// <inheritdoc/>
        public override string ToString() => Wasf();
    }

    /// <summary>One pattern for one target: the variant.</summary>
    /// <remarks>
    /// Several variants exist for one target because one entry covers a range of
    /// builds and a compiler does not emit the same bytes across all of them. The
    /// order is the author's and is preserved exactly, because it is the order
    /// they are tried in and because <see cref="Raqm"/> is quoted in bug reports.
    /// </remarks>
    public sealed class BadilBasma
    {
        /// <summary>Builds a variant.</summary>
        /// <param name="raqm">Its one-based position within its target.</param>
        /// <param name="namat">The parsed pattern, offset and predicates included.</param>
        /// <param name="masdar">Which game build the pattern was taken from.</param>
        /// <param name="muhaqqaq">
        /// Whether anyone has confirmed it against a real binary of that build.
        /// </param>
        /// <param name="mulahazat">Anything the author needed to say about it.</param>
        public BadilBasma(int raqm, Namat namat, string masdar, bool muhaqqaq, string mulahazat)
        {
            Raqm = raqm;
            Namat = namat ?? throw new ArgumentNullException(nameof(namat));
            Masdar = masdar ?? string.Empty;
            Muhaqqaq = muhaqqaq;
            Mulahazat = mulahazat ?? string.Empty;
        }

        /// <summary>Its one-based position within its target, in file order.</summary>
        public int Raqm { get; }

        /// <summary>The pattern.</summary>
        public Namat Namat { get; }

        /// <summary>The game or engine build this pattern was read out of.</summary>
        public string Masdar { get; }

        /// <summary>
        /// Whether the pattern has been confirmed against a real binary of the
        /// build named in <see cref="Masdar"/>.
        /// </summary>
        /// <remarks>
        /// False is the honest default and is not a reason to withhold a pattern:
        /// an unverified pattern still has to match uniquely and still has to
        /// pass every predicate before anything is detoured, so the cost of
        /// shipping one is a rung that declines. The flag exists so a release
        /// checklist, a diagnostics bundle and a reviewer can all see which
        /// entries are still claims rather than measurements.
        /// </remarks>
        public bool Muhaqqaq { get; }

        /// <summary>Whatever the author needed a reader to know.</summary>
        public string Mulahazat { get; }
    }

    /// <summary>One target inside one entry: a key and its variants.</summary>
    public sealed class HadafBasmat
    {
        /// <summary>Builds a target.</summary>
        /// <param name="miftah">The key, matching <c>HadafHall.MiftahBasma</c>.</param>
        /// <param name="wasf">The method's fully qualified name, for a reader.</param>
        /// <param name="badail">Its variants, in file order.</param>
        public HadafBasmat(string miftah, string wasf, BadilBasma[] badail)
        {
            Miftah = miftah ?? string.Empty;
            Wasf = wasf ?? string.Empty;
            Badail = badail ?? Array.Empty<BadilBasma>();
        }

        /// <summary>The key a resolution target is looked up by.</summary>
        public string Miftah { get; }

        /// <summary>The method this key stands for, written out.</summary>
        public string Wasf { get; }

        /// <summary>Its variants, in the order they are tried.</summary>
        public IReadOnlyList<BadilBasma> Badail { get; }
    }

    /// <summary>
    /// One entry: the four-part key, the module its patterns live in, and the
    /// targets it knows.
    /// </summary>
    public sealed class MadkhalBasmat
    {
        /// <summary>Builds an entry.</summary>
        /// <param name="muarrif">Its identifier, unique within the database.</param>
        /// <param name="madaMuharrik">The engine version range it applies to.</param>
        /// <param name="madaNusus">The text-package version range it applies to.</param>
        /// <param name="binya">The architecture it was taken from.</param>
        /// <param name="minassa">The platform it was taken from.</param>
        /// <param name="wahda">The module its patterns are searched in.</param>
        /// <param name="mulahazat">Anything the author needed to say about it.</param>
        /// <param name="ahdaf">Its targets.</param>
        public MadkhalBasmat(
            string muarrif,
            MadaIsdar madaMuharrik,
            MadaIsdar madaNusus,
            Binya binya,
            Minassa minassa,
            string wahda,
            string mulahazat,
            HadafBasmat[] ahdaf)
        {
            Muarrif = muarrif ?? string.Empty;
            MadaMuharrik = madaMuharrik;
            MadaNusus = madaNusus;
            Binya = binya;
            Minassa = minassa;
            Wahda = wahda ?? string.Empty;
            Mulahazat = mulahazat ?? string.Empty;
            Ahdaf = ahdaf ?? Array.Empty<HadafBasmat>();
            Lafta = Laftat(madaMuharrik);
        }

        /// <summary>Its identifier, unique within the database.</summary>
        public string Muarrif { get; }

        /// <summary>The engine version range it applies to.</summary>
        public MadaIsdar MadaMuharrik { get; }

        /// <summary>The text-package version range it applies to.</summary>
        public MadaIsdar MadaNusus { get; }

        /// <summary>The architecture its patterns were taken from.</summary>
        public Binya Binya { get; }

        /// <summary>The platform its patterns were taken from.</summary>
        public Minassa Minassa { get; }

        /// <summary>
        /// The module its patterns are searched in — <c>GameAssembly.dll</c> for
        /// everything IL2CPP compiled.
        /// </summary>
        public string Wahda { get; }

        /// <summary>Whatever the author needed a reader to know.</summary>
        public string Mulahazat { get; }

        /// <summary>Its targets.</summary>
        public IReadOnlyList<HadafBasmat> Ahdaf { get; }

        /// <summary>The short version label a match record quotes.</summary>
        public string Lafta { get; }

        /// <summary>
        /// How specific this entry is. Higher is tried first.
        /// </summary>
        /// <remarks>
        /// <para>
        /// The engine range's own specificity multiplied by a thousand, plus the
        /// text-package range's. The thousand is not arbitrary: no range can
        /// score above about eighty, so the multiplier makes the engine range
        /// strictly dominant and the text range a tie-break within it, which is
        /// the ordering the two versions deserve. The engine version determines
        /// the compiler, its flags, the calling convention and the layout of the
        /// entire binary; the package version determines only what one function's
        /// source looked like. An entry keyed to Unity 2021.3.16f1 with any TMP
        /// therefore beats one keyed to any Unity with TMP 3.0.6 — the first was
        /// read out of a binary very like the one being scanned, and the second
        /// was read out of a binary that may share nothing with it but a
        /// filename.
        /// </para>
        /// <para>
        /// Ties are broken by position in the file, ascending, so the order a
        /// lookup returns is fully determined by the database's own text. Two
        /// users on the same game get the same answer, and a reviewer can predict
        /// the order by reading.
        /// </para>
        /// </remarks>
        public int Diqqa => (MadaMuharrik.Diqqa * 1000) + MadaNusus.Diqqa;

        /// <summary>Whether this entry applies to a running game.</summary>
        /// <param name="muharrik">The engine version, or an unread version.</param>
        /// <param name="nusus">The text package version, or an unread version.</param>
        /// <param name="binya">The architecture of the game's process.</param>
        /// <param name="minassa">The platform it is running on.</param>
        /// <returns>Whether the entry's key matches.</returns>
        /// <remarks>
        /// A version that could not be read matches only a range written
        /// <c>*</c>, which is the written statement that the pattern was never
        /// keyed on that version. Anything else would be comparing against a
        /// number nobody measured, which is the failure Isdar.cs exists to
        /// prevent — and a TextMeshPro version in particular is genuinely hard to
        /// read in a stripped IL2CPP build, so this case is common rather than
        /// theoretical.
        /// </remarks>
        public bool Yutabiq(in Isdar muharrik, in Isdar nusus, Binya binya, Minassa minassa)
        {
            if (Binya != binya || Minassa != minassa)
            {
                return false;
            }
            if (!Wafiq(MadaMuharrik, in muharrik))
            {
                return false;
            }
            return Wafiq(MadaNusus, in nusus);
        }

        private static bool Wafiq(MadaIsdar mada, in Isdar isdar)
        {
            return isdar.Maqru ? mada.Yahwi(in isdar) : mada.Shamila;
        }

        private static string Laftat(MadaIsdar mada)
        {
            if (!mada.Asfal.Maftuh)
            {
                return mada.Asfal.Qeema.ToString();
            }
            if (!mada.Aala.Maftuh)
            {
                return "<" + mada.Aala.Qeema.ToString();
            }
            return "*";
        }
    }

    /// <summary>One candidate a lookup produced, with everything needed to try it.</summary>
    public readonly struct MurashshahBasma
    {
        /// <summary>Builds a candidate.</summary>
        /// <param name="namat">The pattern to scan for.</param>
        /// <param name="sijill">The record identifying it.</param>
        /// <param name="wahda">The module to scan.</param>
        /// <param name="diqqa">Its entry's specificity, for a log line.</param>
        /// <param name="muhaqqaq">Whether the pattern has been verified.</param>
        public MurashshahBasma(
            Namat namat,
            SijillBasma sijill,
            string wahda,
            int diqqa,
            bool muhaqqaq)
        {
            Namat = namat ?? throw new ArgumentNullException(nameof(namat));
            Sijill = sijill;
            Wahda = wahda ?? string.Empty;
            Diqqa = diqqa;
            Muhaqqaq = muhaqqaq;
        }

        /// <summary>The pattern to scan for.</summary>
        public Namat Namat { get; }

        /// <summary>Its stable record, without an address until it resolves.</summary>
        public SijillBasma Sijill { get; }

        /// <summary>The module its entry says to scan.</summary>
        public string Wahda { get; }

        /// <summary>Its entry's specificity.</summary>
        public int Diqqa { get; }

        /// <summary>Whether the pattern has been verified against a real binary.</summary>
        public bool Muhaqqaq { get; }
    }

    /// <summary>
    /// The signature database: a schema version, entries keyed by engine and
    /// package version, architecture and platform, and the lookup that orders
    /// candidates most specific first.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Immutable once loaded and safe to share. Loading happens once during
    /// plugin initialisation; nothing here is on a frame path.
    /// </para>
    /// <para>
    /// Nothing in this type reads process memory or resolves anything. It answers
    /// "which patterns might apply to this game, in what order" and stops there,
    /// so it can be exercised on a workstation against a JSON file with no game
    /// running — which is how a database is reviewed before it ships.
    /// </para>
    /// </remarks>
    public sealed class Qaida
    {
        /// <summary>The schema version this build reads.</summary>
        /// <remarks>
        /// A file declaring less is read with the same reader, because every
        /// schema version so far is this one. A file declaring more is refused;
        /// the header of this file says why a partial read is the worse answer.
        /// </remarks>
        public const int MukhattatMadum = 1;

        /// <summary>
        /// The largest database this reader will open, in bytes.
        /// </summary>
        /// <remarks>
        /// The shipped file is a few tens of kilobytes. Sixteen megabytes is far
        /// past any plausible growth and still small enough that a corrupted or
        /// substituted file cannot make a game hang on a JSON parse during
        /// startup while the player watches a loading screen.
        /// </remarks>
        public const long AqsaHajm = 16L * 1024L * 1024L;

        private static readonly string[] HuqulJudhr =
        {
            "mukhattat", "isdar", "wasf", "madakhil",
        };

        private static readonly string[] HuqulMadkhal =
        {
            "muarrif", "mada_muharrik", "mada_nusus", "binya", "minassa", "wahda",
            "mulahazat", "ahdaf",
        };

        private static readonly string[] HuqulHadaf =
        {
            "miftah", "wasf", "badail",
        };

        private static readonly string[] HuqulBadil =
        {
            "namat", "izaha", "shurut", "masdar", "muhaqqaq", "mulahazat",
        };

        private static readonly string[] HuqulShart =
        {
            "naw", "izaha", "qeema",
        };

        private readonly MadkhalBasmat[] madakhil;
        private readonly string[] tahdhirat;

        private Qaida(
            int mukhattat,
            string isdarBayanat,
            string wasf,
            string masar,
            MadkhalBasmat[] madakhil,
            string[] tahdhirat)
        {
            Mukhattat = mukhattat;
            IsdarBayanat = isdarBayanat;
            Wasf = wasf;
            Masar = masar;
            this.madakhil = madakhil;
            this.tahdhirat = tahdhirat;

            int adad = 0;
            for (int i = 0; i < madakhil.Length; i++)
            {
                IReadOnlyList<HadafBasmat> ahdaf = madakhil[i].Ahdaf;
                for (int j = 0; j < ahdaf.Count; j++)
                {
                    adad += ahdaf[j].Badail.Count;
                }
            }
            AdadBadail = adad;
        }

        /// <summary>The schema version the file declared.</summary>
        public int Mukhattat { get; }

        /// <summary>The database's own data version, as the file declared it.</summary>
        public string IsdarBayanat { get; }

        /// <summary>What the file says it is.</summary>
        public string Wasf { get; }

        /// <summary>Where it was read from, for a message.</summary>
        public string Masar { get; }

        /// <summary>Its entries, in file order.</summary>
        public IReadOnlyList<MadkhalBasmat> Madakhil => madakhil;

        /// <summary>
        /// Everything that was wrong with the file but did not justify refusing
        /// it: a variant whose pattern did not parse, a duplicate key.
        /// </summary>
        /// <remarks>
        /// Never empty and ignored. These reach the log at load and the
        /// diagnostics bundle afterwards, because a warning nobody prints is a
        /// broken pattern nobody fixes.
        /// </remarks>
        public IReadOnlyList<string> Tahdhirat => tahdhirat;

        /// <summary>How many usable patterns the database holds in total.</summary>
        public int AdadBadail { get; }

        /// <summary>
        /// Whether this database makes rung three usable, and one sentence
        /// saying so when it does not.
        /// </summary>
        /// <param name="sabab">Why the rung is unavailable, when it is.</param>
        /// <returns>Whether the rung can be tried.</returns>
        /// <remarks>
        /// A database that loaded but holds nothing is unavailable, not
        /// available-and-useless: the difference is one sentence in a log that
        /// tells its reader whether to look for a missing file or an empty one.
        /// </remarks>
        public bool Mutah(out string sabab)
        {
            if (AdadBadail == 0)
            {
                sabab = $"the signature database at {Masar} loaded but holds no usable "
                    + "pattern, so there is nothing for rung 3 to try";
                return false;
            }
            sabab = string.Empty;
            return true;
        }

        /// <summary>The sentence a rung reports when no database was shipped.</summary>
        /// <param name="masar">Where one was looked for.</param>
        /// <returns>One sentence, naming the path.</returns>
        /// <remarks>
        /// A static so that the rung can say this without a database object,
        /// which is precisely the situation it is describing.
        /// </remarks>
        public static string SababGhiyab(string masar)
        {
            return $"no signature database was shipped with this build ({masar} is not "
                + "present), so rung 3 has nothing to scan for; this is a configuration, not "
                + "a fault";
        }

        /// <summary>The platform and architecture token a match record quotes.</summary>
        /// <param name="minassa">The platform.</param>
        /// <param name="binya">The architecture.</param>
        /// <returns><c>win-x64</c>, <c>linux-x64</c>, <c>mac-arm64</c>, and so on.</returns>
        /// <remarks>
        /// Written out rather than derived from the enum names, because these
        /// strings appear in bug reports and in the database's own identifiers
        /// and must not change when a member is renamed.
        /// </remarks>
        public static string RamzMinassa(Minassa minassa, Binya binya)
        {
            string mim;
            switch (minassa)
            {
                case Minassa.Linux:
                    mim = "linux";
                    break;
                case Minassa.Mac:
                    mim = "mac";
                    break;
                case Minassa.Android:
                    mim = "android";
                    break;
                default:
                    mim = "win";
                    break;
            }

            string bin;
            switch (binya)
            {
                case Binya.X86:
                    bin = "x86";
                    break;
                case Binya.Aarch64:
                    bin = "arm64";
                    break;
                default:
                    bin = "x64";
                    break;
            }
            return mim + "-" + bin;
        }

        /// <summary>Opens a database file, refusing anything it cannot trust.</summary>
        /// <param name="masar">The path to the file.</param>
        /// <returns>The loaded database.</returns>
        /// <exception cref="KhataTaarib">
        /// The file is absent, unreadable, not valid JSON, not the shape this
        /// build reads, or declares a newer schema version.
        /// </exception>
        /// <remarks>
        /// The throwing form, for a tool that is validating a database on
        /// purpose. The plugin's rung uses <see cref="HawilFath"/>, because
        /// absence is not a failure there and because a rung must not throw.
        /// </remarks>
        public static Qaida Fath(string masar)
        {
            if (!HawilFath(masar, out Qaida? qaida, out HalatQaida halat, out string sabab))
            {
                throw Rafd(halat, masar, sabab);
            }
            return qaida;
        }

        /// <summary>Opens a database file, reporting rather than throwing.</summary>
        /// <param name="masar">The path to the file.</param>
        /// <param name="qaida">The database, when it loaded.</param>
        /// <param name="halat">What happened.</param>
        /// <param name="sabab">One sentence saying what, when it did not load.</param>
        /// <returns>Whether it loaded.</returns>
        /// <remarks>
        /// Never throws for anything a file can do to it. A rung that throws
        /// stops the rungs below it from being tried, and here there are no rungs
        /// below.
        /// </remarks>
        public static bool HawilFath(
            string masar,
            [NotNullWhen(true)] out Qaida? qaida,
            out HalatQaida halat,
            out string sabab)
        {
            qaida = null;
            if (string.IsNullOrWhiteSpace(masar))
            {
                halat = HalatQaida.Ghaiba;
                sabab = SababGhiyab("(no path)");
                return false;
            }

            string nass;
            try
            {
                FileInfo malaf = new FileInfo(masar);
                if (!malaf.Exists)
                {
                    halat = HalatQaida.Ghaiba;
                    sabab = SababGhiyab(masar);
                    return false;
                }
                if (malaf.Length > AqsaHajm)
                {
                    halat = HalatQaida.LaTuqra;
                    sabab = $"the signature database at {masar} is {malaf.Length} bytes, past "
                        + $"the {AqsaHajm}-byte limit this reader will open";
                    return false;
                }
                nass = File.ReadAllText(masar);
            }
            catch (IOException khata)
            {
                halat = HalatQaida.LaTuqra;
                sabab = $"the signature database at {masar} could not be read: {khata.Message}";
                return false;
            }
            catch (UnauthorizedAccessException khata)
            {
                halat = HalatQaida.LaTuqra;
                sabab = $"the signature database at {masar} could not be read: {khata.Message}";
                return false;
            }
            catch (System.Security.SecurityException khata)
            {
                halat = HalatQaida.LaTuqra;
                sabab = $"the signature database at {masar} could not be read: {khata.Message}";
                return false;
            }
            catch (NotSupportedException khata)
            {
                // A path with a device name or a URI scheme in it. Reported like
                // any other unreadable file rather than escaping as an exception,
                // because the caller is a rung and a rung must not throw.
                halat = HalatQaida.LaTuqra;
                sabab = $"the signature database path {masar} is not usable: {khata.Message}";
                return false;
            }
            catch (ArgumentException khata)
            {
                halat = HalatQaida.LaTuqra;
                sabab = $"the signature database path {masar} is not usable: {khata.Message}";
                return false;
            }

            return HawilMinNass(nass, masar, out qaida, out halat, out sabab);
        }

        /// <summary>Reads a database from text, refusing anything it cannot trust.</summary>
        /// <param name="nass">The file's contents.</param>
        /// <param name="masar">Where it came from, for messages.</param>
        /// <returns>The loaded database.</returns>
        /// <exception cref="KhataTaarib">It is not a database this build reads.</exception>
        public static Qaida MinNass(string nass, string masar)
        {
            if (!HawilMinNass(nass, masar, out Qaida? qaida, out HalatQaida halat,
                    out string sabab))
            {
                throw Rafd(halat, masar, sabab);
            }
            return qaida;
        }

        /// <summary>Reads a database from text, reporting rather than throwing.</summary>
        /// <param name="nass">The file's contents.</param>
        /// <param name="masar">Where it came from, for messages.</param>
        /// <param name="qaida">The database, when it loaded.</param>
        /// <param name="halat">What happened.</param>
        /// <param name="sabab">One sentence saying what, when it did not load.</param>
        /// <returns>Whether it loaded.</returns>
        public static bool HawilMinNass(
            string nass,
            string masar,
            [NotNullWhen(true)] out Qaida? qaida,
            out HalatQaida halat,
            out string sabab)
        {
            qaida = null;
            halat = HalatQaida.Talifa;
            string ism = string.IsNullOrEmpty(masar) ? "the signature database" : masar;

            if (string.IsNullOrWhiteSpace(nass))
            {
                sabab = $"{ism} is empty";
                return false;
            }

            JsonDocumentOptions khiyarat = new JsonDocumentOptions
            {
                // Comments and trailing commas are not JSON, and accepting them
                // here would produce a file the shipped schema rejects — a
                // database that loads locally and fails validation in CI is a
                // trap for the next contributor.
                CommentHandling = JsonCommentHandling.Disallow,
                AllowTrailingCommas = false,
                MaxDepth = 32,
            };

            JsonDocument wathiqa;
            try
            {
                wathiqa = JsonDocument.Parse(nass, khiyarat);
            }
            catch (JsonException khata)
            {
                sabab = $"{ism} is not valid JSON: {khata.Message}";
                return false;
            }

            using (wathiqa)
            {
                JsonElement judhr = wathiqa.RootElement;
                if (judhr.ValueKind != JsonValueKind.Object)
                {
                    sabab = $"{ism} is a {Naw(judhr.ValueKind)} at its root, not an object";
                    return false;
                }

                // THE SCHEMA VERSION FIRST, BEFORE ANY OTHER FIELD IS TOUCHED.
                if (!Sahih(judhr, "mukhattat", "$", true, 0, out int mukhattat, out sabab))
                {
                    return false;
                }
                if (mukhattat <= 0)
                {
                    sabab = $"{ism} declares schema version {mukhattat}, which is not a "
                        + "version";
                    return false;
                }
                if (mukhattat > MukhattatMadum)
                {
                    halat = HalatQaida.MukhattatAhdath;
                    sabab = $"{ism} is written to schema version {mukhattat} and this build "
                        + $"reads version {MukhattatMadum}; it is refused rather than read in "
                        + "part, because the fields this build cannot see are the ones a newer "
                        + "schema would have added to constrain a pattern, and dropping a "
                        + "constraint silently is how the wrong function gets detoured";
                    return false;
                }

                if (!LaZaid(judhr, HuqulJudhr, "$", out sabab))
                {
                    return false;
                }
                if (!NassMin(judhr, "isdar", "$", true, out string isdarBayanat, out sabab))
                {
                    return false;
                }
                if (!NassMin(judhr, "wasf", "$", false, out string wasf, out sabab))
                {
                    return false;
                }
                if (!Kian(judhr, "madakhil", JsonValueKind.Array, "$", true,
                        out JsonElement kianMadakhil, out _, out sabab))
                {
                    return false;
                }

                List<MadkhalBasmat> madakhil = new List<MadkhalBasmat>(16);
                List<string> tahdhirat = new List<string>();
                HashSet<string> muarrifat = new HashSet<string>(StringComparer.Ordinal);
                int fahras = 0;
                foreach (JsonElement kian in kianMadakhil.EnumerateArray())
                {
                    string sabil = $"$.madakhil[{fahras}]";
                    fahras++;
                    if (!Madkhal(kian, sabil, tahdhirat, out MadkhalBasmat? madkhal,
                            out sabab))
                    {
                        return false;
                    }
                    if (!muarrifat.Add(madkhal.Muarrif))
                    {
                        sabab = $"{sabil}.muarrif is '{madkhal.Muarrif}', which another entry "
                            + "already uses; identifiers name entries in bug reports and must "
                            + "be unique";
                        return false;
                    }
                    madakhil.Add(madkhal);
                }

                qaida = new Qaida(
                    mukhattat,
                    isdarBayanat,
                    wasf,
                    ism,
                    madakhil.ToArray(),
                    tahdhirat.ToArray());
                halat = HalatQaida.Mawjuda;
                sabab = string.Empty;
                return true;
            }
        }

        /// <summary>
        /// Every pattern that might resolve a target in this game, most specific
        /// first.
        /// </summary>
        /// <param name="miftah">The target's key, from <c>HadafHall.MiftahBasma</c>.</param>
        /// <param name="muharrik">The engine version, or an unread version.</param>
        /// <param name="nusus">The text package version, or an unread version.</param>
        /// <param name="binya">The architecture of the game's process.</param>
        /// <param name="minassa">The platform it is running on.</param>
        /// <returns>
        /// The candidates in the order they should be tried. Empty when the
        /// database knows nothing for this key on this build, which is a normal
        /// answer and not a fault.
        /// </returns>
        /// <remarks>
        /// Ordered by <see cref="MadkhalBasmat.Diqqa"/> descending, then by
        /// position in the file ascending, then by variant number ascending. The
        /// second and third keys are what make the order a property of the
        /// database's text rather than of a sort's stability, so two users on one
        /// game get the same order and a reviewer can predict it by reading.
        /// </remarks>
        public IReadOnlyList<MurashshahBasma> Bahth(
            string miftah,
            in Isdar muharrik,
            in Isdar nusus,
            Binya binya,
            Minassa minassa)
        {
            if (string.IsNullOrEmpty(miftah))
            {
                return Array.Empty<MurashshahBasma>();
            }

            List<Tarteeb> murashshahun = new List<Tarteeb>(8);
            for (int i = 0; i < madakhil.Length; i++)
            {
                MadkhalBasmat madkhal = madakhil[i];
                if (!madkhal.Yutabiq(in muharrik, in nusus, binya, minassa))
                {
                    continue;
                }
                string ramz = RamzMinassa(madkhal.Minassa, madkhal.Binya);
                IReadOnlyList<HadafBasmat> ahdaf = madkhal.Ahdaf;
                for (int j = 0; j < ahdaf.Count; j++)
                {
                    HadafBasmat hadaf = ahdaf[j];
                    if (!string.Equals(hadaf.Miftah, miftah, StringComparison.Ordinal))
                    {
                        continue;
                    }
                    IReadOnlyList<BadilBasma> badail = hadaf.Badail;
                    for (int k = 0; k < badail.Count; k++)
                    {
                        BadilBasma badil = badail[k];
                        SijillBasma sijill = new SijillBasma(
                            hadaf.Miftah,
                            madkhal.Lafta,
                            ramz,
                            badil.Raqm,
                            madkhal.Muarrif,
                            badil.Namat.Nass,
                            IntPtr.Zero);
                        murashshahun.Add(new Tarteeb(
                            madkhal.Diqqa,
                            i,
                            badil.Raqm,
                            new MurashshahBasma(
                                badil.Namat,
                                sijill,
                                madkhal.Wahda,
                                madkhal.Diqqa,
                                badil.Muhaqqaq)));
                    }
                }
            }

            murashshahun.Sort(Tarteeb.Qaran);
            MurashshahBasma[] natija = new MurashshahBasma[murashshahun.Count];
            for (int i = 0; i < murashshahun.Count; i++)
            {
                natija[i] = murashshahun[i].Murashshah;
            }
            return natija;
        }

        /// <summary>The database's own summary, as lines for the log and a bundle.</summary>
        /// <returns>One line of totals, one per warning, one per entry.</returns>
        public IReadOnlyList<string> Taqreer()
        {
            List<string> sutur = new List<string>(madakhil.Length + tahdhirat.Length + 1);
            sutur.Add(
                $"signature database {IsdarBayanat} (schema {Mukhattat}) from {Masar}: "
                + $"{madakhil.Length} entries, {AdadBadail} patterns");
            for (int i = 0; i < tahdhirat.Length; i++)
            {
                sutur.Add("warning: " + tahdhirat[i]);
            }
            for (int i = 0; i < madakhil.Length; i++)
            {
                MadkhalBasmat madkhal = madakhil[i];
                int adad = 0;
                int muhaqqaq = 0;
                IReadOnlyList<HadafBasmat> ahdaf = madkhal.Ahdaf;
                for (int j = 0; j < ahdaf.Count; j++)
                {
                    IReadOnlyList<BadilBasma> badail = ahdaf[j].Badail;
                    adad += badail.Count;
                    for (int k = 0; k < badail.Count; k++)
                    {
                        if (badail[k].Muhaqqaq)
                        {
                            muhaqqaq++;
                        }
                    }
                }
                sutur.Add(
                    $"{madkhal.Muarrif}: engine {madkhal.MadaMuharrik}, text "
                    + $"{madkhal.MadaNusus}, {RamzMinassa(madkhal.Minassa, madkhal.Binya)}, "
                    + $"{madkhal.Ahdaf.Count} targets, {adad} patterns ({muhaqqaq} verified), "
                    + $"specificity {madkhal.Diqqa}");
            }
            return sutur;
        }

        private readonly struct Tarteeb
        {
            public Tarteeb(int diqqa, int fahras, int badil, MurashshahBasma murashshah)
            {
                Diqqa = diqqa;
                Fahras = fahras;
                Badil = badil;
                Murashshah = murashshah;
            }

            public int Diqqa { get; }

            public int Fahras { get; }

            public int Badil { get; }

            public MurashshahBasma Murashshah { get; }

            public static int Qaran(Tarteeb alfa, Tarteeb beta)
            {
                int farq = beta.Diqqa.CompareTo(alfa.Diqqa);
                if (farq != 0)
                {
                    return farq;
                }
                farq = alfa.Fahras.CompareTo(beta.Fahras);
                return farq != 0 ? farq : alfa.Badil.CompareTo(beta.Badil);
            }
        }

        private static KhataTaarib Rafd(HalatQaida halat, string masar, string sabab)
        {
            switch (halat)
            {
                case HalatQaida.Ghaiba:
                    return new KhataTaarib(
                        Ramz.GhayrMadum,
                        "TAARIB-E-6502",
                        $"قاعدة البصمات غير موجودة في {masar}؛ {sabab}",
                        $"The signature database is not present at {masar}. {sabab}",
                        Khutwa.IadatTarkibIttar);
                case HalatQaida.LaTuqra:
                    return new KhataTaarib(
                        Ramz.KhataAam,
                        "TAARIB-E-6503",
                        $"تعذّرت قراءة قاعدة البصمات: {sabab}",
                        $"The signature database could not be read: {sabab}",
                        Khutwa.IadatTarkibIttar);
                case HalatQaida.MukhattatAhdath:
                    return new KhataTaarib(
                        Ramz.IsdarGhayrMutawafiq,
                        "TAARIB-E-6504",
                        $"قاعدة البصمات مكتوبة بمخطط أحدث من هذه النسخة: {sabab}",
                        "The signature database is written to a newer schema than this build "
                        + $"reads: {sabab}",
                        Khutwa.TahdithTaarib);
                default:
                    return new KhataTaarib(
                        Ramz.QeemaBatila,
                        "TAARIB-E-6505",
                        $"قاعدة البصمات غير صالحة: {sabab}",
                        $"The signature database is not usable: {sabab}",
                        Khutwa.IblaghLilMalik);
            }
        }

        private static bool Madkhal(
            JsonElement kian,
            string sabil,
            List<string> tahdhirat,
            [NotNullWhen(true)] out MadkhalBasmat? madkhal,
            out string sabab)
        {
            madkhal = null;
            if (kian.ValueKind != JsonValueKind.Object)
            {
                sabab = $"{sabil} is a {Naw(kian.ValueKind)}, not an object";
                return false;
            }
            if (!LaZaid(kian, HuqulMadkhal, sabil, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "muarrif", sabil, true, out string muarrif, out sabab))
            {
                return false;
            }
            if (!MadaMin(kian, "mada_muharrik", sabil, out MadaIsdar madaMuharrik, out sabab))
            {
                return false;
            }
            if (!MadaMin(kian, "mada_nusus", sabil, out MadaIsdar madaNusus, out sabab))
            {
                return false;
            }
            if (!BinyaMin(kian, sabil, out Binya binya, out sabab))
            {
                return false;
            }
            if (!MinassaMin(kian, sabil, out Minassa minassa, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "wahda", sabil, true, out string wahda, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "mulahazat", sabil, false, out string mulahazat, out sabab))
            {
                return false;
            }
            if (!Kian(kian, "ahdaf", JsonValueKind.Array, sabil, true,
                    out JsonElement kianAhdaf, out _, out sabab))
            {
                return false;
            }

            List<HadafBasmat> ahdaf = new List<HadafBasmat>(8);
            HashSet<string> mafatih = new HashSet<string>(StringComparer.Ordinal);
            int fahras = 0;
            foreach (JsonElement wahid in kianAhdaf.EnumerateArray())
            {
                string sabilHadaf = $"{sabil}.ahdaf[{fahras}]";
                fahras++;
                if (!Hadaf(wahid, sabilHadaf, binya, muarrif, tahdhirat,
                        out HadafBasmat? hadaf, out sabab))
                {
                    return false;
                }
                if (!mafatih.Add(hadaf.Miftah))
                {
                    sabab = $"{sabilHadaf}.miftah is '{hadaf.Miftah}', which this entry already "
                        + "declares; a key that appears twice in one entry makes the order "
                        + "patterns are tried in depend on which copy a reader reaches first";
                    return false;
                }
                ahdaf.Add(hadaf);
            }

            madkhal = new MadkhalBasmat(
                muarrif,
                madaMuharrik,
                madaNusus,
                binya,
                minassa,
                wahda,
                mulahazat,
                ahdaf.ToArray());
            sabab = string.Empty;
            return true;
        }

        private static bool Hadaf(
            JsonElement kian,
            string sabil,
            Binya binya,
            string muarrif,
            List<string> tahdhirat,
            [NotNullWhen(true)] out HadafBasmat? hadaf,
            out string sabab)
        {
            hadaf = null;
            if (kian.ValueKind != JsonValueKind.Object)
            {
                sabab = $"{sabil} is a {Naw(kian.ValueKind)}, not an object";
                return false;
            }
            if (!LaZaid(kian, HuqulHadaf, sabil, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "miftah", sabil, true, out string miftah, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "wasf", sabil, false, out string wasf, out sabab))
            {
                return false;
            }
            if (!Kian(kian, "badail", JsonValueKind.Array, sabil, true,
                    out JsonElement kianBadail, out _, out sabab))
            {
                return false;
            }

            List<BadilBasma> badail = new List<BadilBasma>(4);
            int raqm = 0;
            foreach (JsonElement wahid in kianBadail.EnumerateArray())
            {
                raqm++;
                string sabilBadil = $"{sabil}.badail[{raqm - 1}]";
                if (!Badil(wahid, sabilBadil, binya, raqm, out BadilBasma? badil,
                        out string khalal, out bool banyawi))
                {
                    if (banyawi)
                    {
                        sabab = khalal;
                        return false;
                    }

                    // A pattern that does not parse costs itself and nothing
                    // else. The warning names the entry, the key and the variant
                    // number, which is everything needed to find the line.
                    tahdhirat.Add(
                        $"{muarrif}/{miftah} variant {raqm} was dropped: {khalal}");
                    continue;
                }
                badail.Add(badil);
            }

            if (raqm == 0)
            {
                // Structural: a target declaring no variant is a key that
                // promises a pattern and carries none, and a lookup that returns
                // it would report "no pattern for this build" for a target the
                // database appears to cover.
                sabab = $"{sabil}.badail is empty; a target with no pattern is a key that "
                    + "cannot resolve anything";
                return false;
            }
            if (badail.Count == 0)
            {
                tahdhirat.Add(
                    $"{muarrif}/{miftah} has {raqm} variants and none of them parsed, so this "
                    + "target is unreachable for every game the entry covers");
            }

            hadaf = new HadafBasmat(miftah, wasf, badail.ToArray());
            sabab = string.Empty;
            return true;
        }

        private static bool Badil(
            JsonElement kian,
            string sabil,
            Binya binya,
            int raqm,
            [NotNullWhen(true)] out BadilBasma? badil,
            out string sabab,
            out bool banyawi)
        {
            badil = null;
            banyawi = true;
            if (kian.ValueKind != JsonValueKind.Object)
            {
                sabab = $"{sabil} is a {Naw(kian.ValueKind)}, not an object";
                return false;
            }
            if (!LaZaid(kian, HuqulBadil, sabil, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "namat", sabil, true, out string nassNamat, out sabab))
            {
                return false;
            }
            if (!Sahih(kian, "izaha", sabil, false, 0, out int izaha, out sabab))
            {
                return false;
            }
            if (izaha < -4096 || izaha > 4096)
            {
                // An offset is a walk from an anchor to a function entry, which is
                // tens of bytes in practice. A four-kilobyte offset is a typo or a
                // misunderstanding, and applying it would resolve an address in an
                // unrelated function that then passes every predicate.
                sabab = $"{sabil}.izaha is {izaha}, which is further than a prologue anchor "
                    + "is ever from its function";
                return false;
            }
            if (!NassMin(kian, "masdar", sabil, true, out string masdar, out sabab))
            {
                return false;
            }
            if (!Mantiq(kian, "muhaqqaq", sabil, true, false, out bool muhaqqaq, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "mulahazat", sabil, false, out string mulahazat, out sabab))
            {
                return false;
            }

            List<SharkTahaqquq> shurut = new List<SharkTahaqquq>(4);
            if (!Kian(kian, "shurut", JsonValueKind.Array, sabil, false,
                    out JsonElement kianShurut, out bool wujidShurut, out sabab))
            {
                return false;
            }
            if (wujidShurut)
            {
                int fahras = 0;
                foreach (JsonElement wahid in kianShurut.EnumerateArray())
                {
                    string sabilShart = $"{sabil}.shurut[{fahras}]";
                    fahras++;
                    if (!Shart(wahid, sabilShart, out SharkTahaqquq shart, out sabab))
                    {
                        return false;
                    }
                    shurut.Add(shart);
                }
            }

            banyawi = false;
            if (!Namat.HawilHallil(nassNamat, izaha, shurut, binya, out Namat? namat,
                    out string khalal))
            {
                sabab = $"{sabil}.namat is not usable: {khalal}";
                return false;
            }

            badil = new BadilBasma(raqm, namat, masdar, muhaqqaq, mulahazat);
            sabab = string.Empty;
            return true;
        }

        private static bool Shart(
            JsonElement kian,
            string sabil,
            out SharkTahaqquq shart,
            out string sabab)
        {
            shart = default;
            if (kian.ValueKind != JsonValueKind.Object)
            {
                sabab = $"{sabil} is a {Naw(kian.ValueKind)}, not an object";
                return false;
            }
            if (!LaZaid(kian, HuqulShart, sabil, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "naw", sabil, true, out string naw, out sabab))
            {
                return false;
            }
            if (!Sahih(kian, "izaha", sabil, false, 0, out int izaha, out sabab))
            {
                return false;
            }
            if (!NassMin(kian, "qeema", sabil, false, out string qeema, out sabab))
            {
                return false;
            }

            NawShart nawShart;
            switch (naw)
            {
                case "qism_tanfidhi":
                    nawShart = NawShart.QismTanfidhi;
                    break;
                case "muhadhah":
                    nawShart = NawShart.Muhadhah;
                    break;
                case "muqaddima":
                    nawShart = NawShart.Muqaddima;
                    break;
                case "bayt_ind_izaha":
                    nawShart = NawShart.BaytIndIzaha;
                    break;
                case "lays_bayt_ind_izaha":
                    nawShart = NawShart.LaysBaytIndIzaha;
                    break;
                default:
                    sabab = $"{sabil}.naw is '{naw}', which is not a predicate this build "
                        + "understands; a predicate that cannot be checked is a guard that "
                        + "would be silently removed";
                    return false;
            }

            bool yalzamQeema = nawShart == NawShart.BaytIndIzaha
                || nawShart == NawShart.LaysBaytIndIzaha;
            if (yalzamQeema && qeema.Length == 0)
            {
                sabab = $"{sabil}.naw is '{naw}' but no qeema was given, so there is nothing "
                    + "to compare the bytes against";
                return false;
            }
            if (!yalzamQeema && qeema.Length != 0)
            {
                sabab = $"{sabil}.naw is '{naw}', which takes no qeema, but one was given; a "
                    + "value that is silently ignored is a check the author believes is "
                    + "running";
                return false;
            }
            if (yalzamQeema
                && !Namat.HawilHallil(qeema, 0, Array.Empty<SharkTahaqquq>(), Binya.X8664,
                    out _, out string khalal))
            {
                sabab = $"{sabil}.qeema is not a usable pattern: {khalal}";
                return false;
            }

            shart = new SharkTahaqquq(nawShart, izaha, qeema);
            sabab = string.Empty;
            return true;
        }

        private static bool MadaMin(
            JsonElement kian,
            string ism,
            string sabil,
            out MadaIsdar mada,
            out string sabab)
        {
            mada = default;
            if (!NassMin(kian, ism, sabil, true, out string nass, out sabab))
            {
                return false;
            }
            if (!MadaIsdar.HawilHallil(nass, out mada, out string khalal))
            {
                sabab = $"{sabil}.{ism} is '{nass}', which is not a version range: {khalal}";
                return false;
            }
            return true;
        }

        private static bool BinyaMin(
            JsonElement kian,
            string sabil,
            out Binya binya,
            out string sabab)
        {
            binya = Binya.X8664;
            if (!NassMin(kian, "binya", sabil, true, out string nass, out sabab))
            {
                return false;
            }
            switch (nass)
            {
                case "x86":
                    binya = Binya.X86;
                    return true;
                case "x8664":
                    binya = Binya.X8664;
                    return true;
                case "aarch64":
                    binya = Binya.Aarch64;
                    return true;
                default:
                    sabab = $"{sabil}.binya is '{nass}'; the architectures are x86, x8664 and "
                        + "aarch64";
                    return false;
            }
        }

        private static bool MinassaMin(
            JsonElement kian,
            string sabil,
            out Minassa minassa,
            out string sabab)
        {
            minassa = Minassa.Windows;
            if (!NassMin(kian, "minassa", sabil, true, out string nass, out sabab))
            {
                return false;
            }
            switch (nass)
            {
                case "windows":
                    minassa = Minassa.Windows;
                    return true;
                case "linux":
                    minassa = Minassa.Linux;
                    return true;
                case "mac":
                    minassa = Minassa.Mac;
                    return true;
                case "android":
                    minassa = Minassa.Android;
                    return true;
                default:
                    sabab = $"{sabil}.minassa is '{nass}'; the platforms are windows, linux, "
                        + "mac and android";
                    return false;
            }
        }

        private static bool Kian(
            JsonElement wala,
            string ism,
            JsonValueKind naw,
            string sabil,
            bool matlub,
            out JsonElement qeema,
            out bool wujid,
            out string sabab)
        {
            if (!wala.TryGetProperty(ism, out qeema))
            {
                wujid = false;
                if (matlub)
                {
                    sabab = $"{sabil}.{ism} is missing";
                    return false;
                }
                sabab = string.Empty;
                return true;
            }
            wujid = true;
            if (qeema.ValueKind != naw)
            {
                sabab = $"{sabil}.{ism} is a {Naw(qeema.ValueKind)}, and must be a {Naw(naw)}";
                return false;
            }
            sabab = string.Empty;
            return true;
        }

        private static bool NassMin(
            JsonElement wala,
            string ism,
            string sabil,
            bool matlub,
            out string qeema,
            out string sabab)
        {
            qeema = string.Empty;
            if (!Kian(wala, ism, JsonValueKind.String, sabil, matlub,
                    out JsonElement kian, out bool wujid, out sabab))
            {
                return false;
            }
            if (!wujid)
            {
                return true;
            }
            qeema = kian.GetString() ?? string.Empty;
            if (matlub && qeema.Length == 0)
            {
                sabab = $"{sabil}.{ism} is an empty string";
                return false;
            }
            return true;
        }

        private static bool Sahih(
            JsonElement wala,
            string ism,
            string sabil,
            bool matlub,
            int iftiradi,
            out int qeema,
            out string sabab)
        {
            qeema = iftiradi;
            if (!Kian(wala, ism, JsonValueKind.Number, sabil, matlub,
                    out JsonElement kian, out bool wujid, out sabab))
            {
                return false;
            }
            if (!wujid)
            {
                return true;
            }
            if (!kian.TryGetInt32(out qeema))
            {
                qeema = iftiradi;
                sabab = $"{sabil}.{ism} is not a whole number that fits 32 bits";
                return false;
            }
            return true;
        }

        private static bool Mantiq(
            JsonElement wala,
            string ism,
            string sabil,
            bool matlub,
            bool iftiradi,
            out bool qeema,
            out string sabab)
        {
            qeema = iftiradi;
            if (!wala.TryGetProperty(ism, out JsonElement kian))
            {
                if (matlub)
                {
                    sabab = $"{sabil}.{ism} is missing";
                    return false;
                }
                sabab = string.Empty;
                return true;
            }
            if (kian.ValueKind != JsonValueKind.True && kian.ValueKind != JsonValueKind.False)
            {
                sabab = $"{sabil}.{ism} is a {Naw(kian.ValueKind)}, and must be true or false";
                return false;
            }
            qeema = kian.ValueKind == JsonValueKind.True;
            sabab = string.Empty;
            return true;
        }

        private static bool LaZaid(
            JsonElement wala,
            string[] masmuh,
            string sabil,
            out string sabab)
        {
            foreach (JsonProperty khasiya in wala.EnumerateObject())
            {
                bool wujid = false;
                for (int i = 0; i < masmuh.Length; i++)
                {
                    if (string.Equals(masmuh[i], khasiya.Name, StringComparison.Ordinal))
                    {
                        wujid = true;
                        break;
                    }
                }
                if (!wujid)
                {
                    sabab = $"{sabil}.{khasiya.Name} is not a field this build reads; within a "
                        + "schema version it understands, an unrecognised name is a typo, and "
                        + "a typo that is ignored is a constraint that silently does not apply";
                    return false;
                }
            }
            sabab = string.Empty;
            return true;
        }

        private static string Naw(JsonValueKind naw)
        {
            switch (naw)
            {
                case JsonValueKind.Object:
                    return "object";
                case JsonValueKind.Array:
                    return "array";
                case JsonValueKind.String:
                    return "string";
                case JsonValueKind.Number:
                    return "number";
                case JsonValueKind.True:
                case JsonValueKind.False:
                    return "boolean";
                case JsonValueKind.Null:
                    return "null";
                default:
                    return "nothing";
            }
        }
    }
}
