// إصدار — a version, and a range of versions, for the two products whose
// version numbers decide which byte pattern applies: the engine and the text
// package compiled into it.
//
// Unity writes its versions as 2021.3.16f1, 2022.3.0b4, 6000.0.23f1 — a year, a
// stream, a patch, and a release-stage letter followed by an iteration. The
// letter is not decoration. 2021.2.0a5 and 2021.2.0f1 are months apart and the
// second is a different compiler's output from the first, so a pattern taken
// from one has no business being applied to the other. The stages order
//
//     a (alpha)  <  b (beta)  <  f (final)  =  c (China release)  <  p (patch)
//
// which is the one ordering nothing about the strings themselves suggests: 'a'
// sorts before 'b' and before 'f' alphabetically by luck, and 'p' sorts after
// all three by luck, and 'c' — which is a released China build, contemporary
// with the 'f' it carries — sorts in the wrong place entirely. Sorting version
// strings as strings gets three of these right and one of them wrong, and the
// one it gets wrong is the one that only shows up in a Chinese player's bug
// report.
//
// UNITY 6 RENUMBERED. After 2023.2 the version stream became 6000.x, and 6000
// is numerically greater than 2023 but lexicographically smaller than it: as
// strings, "6000.0.23f1" sorts after "2023.1.0f1" only because '6' > '2', and
// the day Unity 7 ships as 7000.x that accident still holds — but "10000.x",
// if it ever exists, would sort before both. The comparison here is numeric on
// every component for that reason, and there is no path through this file where
// two versions are compared as text.
//
// TextMeshPro is the other half of the key and versions plainly: 3.0.6, 2.1.6,
// 1.4.1. It ships as a package with its own cadence, so one Unity version can
// carry several TMP versions and one TMP version spans several Unity versions —
// which is exactly why the database is keyed on both and not on the engine
// alone. TMP 3.0.6's GenerateTextMesh is not TMP 3.0.1's.
//
// WHY AN UNPARSEABLE VERSION IS A REFUSAL AND NOT A ZERO.
//
// This is the failure this file exists to prevent, and it is worth stating in
// full because the wrong behaviour is the easy one to write.
//
// Suppose a game reports its engine version as something this parser does not
// understand — a custom engine fork, a version string a protector rewrote, an
// empty string because the field was stripped. The tempting response is to
// return a default version and carry on. A default version is 0.0.0. And
// 0.0.0 satisfies every range with an open lower bound, which is most of them:
// a pattern written for "[2021.3, 2023.1)" is not selected, but a pattern
// written for "(, 2019.4]" is, and so is anything written as "*".
//
// So the game whose version could not be read gets handed a byte pattern from a
// completely different engine, on the strength of a comparison against a number
// nobody ever measured. If the pattern happens to match something — and the
// uniqueness rule in Namat.cs is the only thing standing between "happens to
// match" and "gets detoured" — the outcome is a crash in a function that has
// nothing to do with text, in a game whose version Taarib never actually knew.
//
// Every entry point here therefore has two forms: one that throws a refusal
// naming the string it could not read, and one that returns false and a
// sentence. Neither of them has a default. There is no Isdar.Sifr.

using System;
using System.Globalization;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Basmat
{
    /// <summary>The release stage a Unity version carries after its patch number.</summary>
    /// <remarks>
    /// <see cref="Isdar.Martaba(TawrIsdar)"/> is the ordering, and it is not the
    /// declaration order: <see cref="Sini"/> ranks equal to <see cref="Nihai"/> because a
    /// China build is a released build of the same source, shipped the same
    /// week, and ordering it between final and patch would place a player's
    /// 2021.3.30f1c1 outside a range that was written to include their build.
    /// </remarks>
    public enum TawrIsdar
    {
        /// <summary>Alpha: <c>2021.2.0a5</c>.</summary>
        Alfa = 0,

        /// <summary>Beta: <c>2022.3.0b4</c>.</summary>
        Beta = 1,

        /// <summary>Final: <c>2021.3.16f1</c>. What nearly every shipped game carries.</summary>
        Nihai = 2,

        /// <summary>A released China build: <c>2021.3.30f1c1</c>.</summary>
        Sini = 3,

        /// <summary>A patch release: <c>2019.4.9p2</c>. Later than the final it patches.</summary>
        Tanqih = 4,

        /// <summary>
        /// No stage was written, as in a TextMeshPro version such as
        /// <c>3.0.6</c>. Ranks with <see cref="Nihai"/>: a package version with
        /// no stage letter is a released version, and ranking it below one would
        /// put every TMP version below every hypothetical prerelease of itself.
        /// </summary>
        Bila = 5,
    }

    /// <summary>
    /// One version of Unity or of TextMeshPro, parsed into components that can
    /// be ordered.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Carries how many components were actually written, because a bound in a
    /// range means something different depending on that: <c>2021.3</c> as a
    /// bound denotes the whole set of versions beginning <c>2021.3</c>, and that
    /// set cannot be recovered from a version that has silently had zeroes
    /// filled in.
    /// </para>
    /// <para>
    /// There is deliberately no default value with meaning. <c>default(Isdar)</c>
    /// has <see cref="Maqru"/> false and every comparison against it is
    /// meaningless; the header of this file says why that matters.
    /// </para>
    /// </remarks>
    public readonly struct Isdar : IComparable<Isdar>, IEquatable<Isdar>
    {
        private Isdar(
            int kabir,
            int sagheer,
            int tasheeh,
            TawrIsdar tawr,
            int raqmTawr,
            int raqmSini,
            int adadMukawwinat,
            string khaam)
        {
            Kabir = kabir;
            Sagheer = sagheer;
            Tasheeh = tasheeh;
            Tawr = tawr;
            RaqmTawr = raqmTawr;
            RaqmSini = raqmSini;
            AdadMukawwinat = adadMukawwinat;
            Khaam = khaam;
            Maqru = true;
        }

        /// <summary>The first component: Unity's year, or TextMeshPro's major.</summary>
        public int Kabir { get; }

        /// <summary>The second component: Unity's stream, or a package's minor.</summary>
        public int Sagheer { get; }

        /// <summary>The third component: the patch number.</summary>
        public int Tasheeh { get; }

        /// <summary>The release stage, or <see cref="TawrIsdar.Bila"/>.</summary>
        public TawrIsdar Tawr { get; }

        /// <summary>The iteration after the stage letter: the 1 in <c>f1</c>.</summary>
        public int RaqmTawr { get; }

        /// <summary>The iteration after a China build's <c>c</c>: the 1 in <c>f1c1</c>.</summary>
        public int RaqmSini { get; }

        /// <summary>
        /// How many numeric components were written: 1, 2 or 3. A bound uses
        /// this to decide how much of a version it is talking about.
        /// </summary>
        public int AdadMukawwinat { get; }

        /// <summary>The string exactly as it arrived, for a message and a bundle.</summary>
        public string Khaam { get; }

        /// <summary>
        /// Whether this value came from a string that parsed. False for
        /// <c>default(Isdar)</c>, which is not a version and must never be
        /// compared as though it were.
        /// </summary>
        public bool Maqru { get; }

        /// <summary>Parses a version, refusing anything it cannot read.</summary>
        /// <param name="nass">The version string, as the engine or package reports it.</param>
        /// <returns>The parsed version.</returns>
        /// <exception cref="KhataTaarib">
        /// The string is empty, or is not a version this build understands. It
        /// is refused rather than defaulted, because a version that compares as
        /// zero satisfies every range with an open lower bound and would select
        /// a pattern written for a different engine entirely.
        /// </exception>
        public static Isdar Hallil(string nass)
        {
            if (!HawilHallil(nass, out Isdar isdar, out string sabab))
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6506",
                    $"تعذّرت قراءة رقم الإصدار «{nass}»: {sabab}",
                    $"The version string \"{nass}\" could not be read: {sabab}",
                    Khutwa.FathTashkhis);
            }
            return isdar;
        }

        /// <summary>Parses a version, reporting rather than throwing.</summary>
        /// <param name="nass">The version string.</param>
        /// <param name="isdar">The parsed version, when it parsed.</param>
        /// <param name="sabab">Why not, when it did not.</param>
        /// <returns>Whether it parsed.</returns>
        /// <remarks>
        /// Accepts <c>2021</c>, <c>2021.3</c>, <c>2021.3.16</c>,
        /// <c>2021.3.16f1</c>, <c>2021.3.30f1c1</c> and <c>3.0.6</c>. Rejects a
        /// trailing build hash, a leading <c>v</c>, and a component that is not
        /// a number, because each of those is a string somebody assumed rather
        /// than measured, and this file's job is to refuse assumptions.
        /// </remarks>
        public static bool HawilHallil(string nass, out Isdar isdar, out string sabab)
        {
            isdar = default;
            if (string.IsNullOrWhiteSpace(nass))
            {
                sabab = "it is empty";
                return false;
            }

            string khaam = nass.Trim();
            int i = 0;
            int tul = khaam.Length;
            int[] mukawwinat = new int[3];
            int adad = 0;

            while (adad < 3)
            {
                if (!Adad(khaam, ref i, out int qeema, out string khalal))
                {
                    sabab = adad == 0
                        ? $"it does not begin with a number ({khalal})"
                        : $"component {adad + 1} is not a number ({khalal})";
                    return false;
                }
                mukawwinat[adad] = qeema;
                adad++;
                if (i < tul && khaam[i] == '.')
                {
                    i++;
                    continue;
                }
                break;
            }

            TawrIsdar tawr = TawrIsdar.Bila;
            int raqmTawr = 0;
            int raqmSini = 0;

            if (i < tul)
            {
                char harf = khaam[i];
                switch (harf)
                {
                    case 'a':
                        tawr = TawrIsdar.Alfa;
                        break;
                    case 'b':
                        tawr = TawrIsdar.Beta;
                        break;
                    case 'f':
                        tawr = TawrIsdar.Nihai;
                        break;
                    case 'p':
                        tawr = TawrIsdar.Tanqih;
                        break;
                    case 'c':
                        tawr = TawrIsdar.Sini;
                        break;
                    default:
                        sabab = $"'{harf}' is not a release stage; Unity writes a, b, f, p or c";
                        return false;
                }
                i++;
                if (!Adad(khaam, ref i, out raqmTawr, out string khalalTawr))
                {
                    sabab = $"the stage letter '{harf}' is not followed by a number "
                        + $"({khalalTawr})";
                    return false;
                }

                // 2021.3.30f1c1: a China build names its own iteration after the
                // final it was built from. Both numbers are kept, and the China
                // one only breaks a tie, because the build is contemporary with
                // its f1 rather than later than it.
                if (i < tul && khaam[i] == 'c' && tawr != TawrIsdar.Sini)
                {
                    i++;
                    if (!Adad(khaam, ref i, out raqmSini, out string khalalSini))
                    {
                        sabab = $"the China build marker 'c' is not followed by a number "
                            + $"({khalalSini})";
                        return false;
                    }
                }
            }

            if (i != tul)
            {
                sabab = $"'{khaam.Substring(i)}' is left over after the version; a build hash "
                    + "or a suffix is not part of a version number this build can order";
                return false;
            }

            isdar = new Isdar(
                mukawwinat[0],
                mukawwinat[1],
                mukawwinat[2],
                tawr,
                raqmTawr,
                raqmSini,
                adad,
                khaam);
            sabab = string.Empty;
            return true;
        }

        /// <summary>The ordering rank of a stage, which is not its declaration order.</summary>
        /// <param name="tawr">The stage.</param>
        /// <returns>alpha 0, beta 1, final and China and none 2, patch 3.</returns>
        public static int Martaba(TawrIsdar tawr)
        {
            switch (tawr)
            {
                case TawrIsdar.Alfa:
                    return 0;
                case TawrIsdar.Beta:
                    return 1;
                case TawrIsdar.Tanqih:
                    return 3;
                default:
                    // Nihai, Sini and Bila are all released builds of the same
                    // patch level and rank together; RaqmTawr and RaqmSini break
                    // the tie below.
                    return 2;
            }
        }

        /// <summary>Orders two versions, numerically on every component.</summary>
        /// <param name="akhar">The other version.</param>
        /// <returns>Negative, zero or positive.</returns>
        /// <remarks>
        /// Components that were not written compare as zero, and a missing stage
        /// ranks as released. Never compares the raw strings, because 6000.x
        /// against 2023.x is the case where text ordering and version ordering
        /// disagree.
        /// </remarks>
        public int CompareTo(Isdar akhar)
        {
            int farq = Kabir.CompareTo(akhar.Kabir);
            if (farq != 0)
            {
                return farq;
            }
            farq = Sagheer.CompareTo(akhar.Sagheer);
            if (farq != 0)
            {
                return farq;
            }
            farq = Tasheeh.CompareTo(akhar.Tasheeh);
            if (farq != 0)
            {
                return farq;
            }
            farq = Martaba(Tawr).CompareTo(Martaba(akhar.Tawr));
            if (farq != 0)
            {
                return farq;
            }
            farq = RaqmTawr.CompareTo(akhar.RaqmTawr);
            if (farq != 0)
            {
                return farq;
            }
            return RaqmSini.CompareTo(akhar.RaqmSini);
        }

        /// <summary>
        /// Orders two versions over the first <paramref name="adad"/> numeric
        /// components only, ignoring the stage unless both name one.
        /// </summary>
        /// <param name="akhar">The other version.</param>
        /// <param name="adad">How many components to compare: 1, 2 or 3.</param>
        /// <param name="maaTawr">Whether to compare the stage as well.</param>
        /// <returns>Negative, zero or positive.</returns>
        /// <remarks>
        /// What makes a partially written bound mean the set of versions sharing
        /// its prefix: <c>2021.3</c> compared over two components against
        /// <c>2021.3.16f1</c> is equal, so the bracket alone decides whether that
        /// build is in or out of the range.
        /// </remarks>
        public int QaranBadia(Isdar akhar, int adad, bool maaTawr)
        {
            int farq = Kabir.CompareTo(akhar.Kabir);
            if (farq != 0 || adad <= 1)
            {
                return farq;
            }
            farq = Sagheer.CompareTo(akhar.Sagheer);
            if (farq != 0 || adad <= 2)
            {
                return farq;
            }
            farq = Tasheeh.CompareTo(akhar.Tasheeh);
            if (farq != 0 || !maaTawr)
            {
                return farq;
            }
            farq = Martaba(Tawr).CompareTo(Martaba(akhar.Tawr));
            if (farq != 0)
            {
                return farq;
            }
            farq = RaqmTawr.CompareTo(akhar.RaqmTawr);
            if (farq != 0)
            {
                return farq;
            }
            return RaqmSini.CompareTo(akhar.RaqmSini);
        }

        /// <summary>Whether two versions are the same version.</summary>
        /// <param name="akhar">The other version.</param>
        /// <returns>Whether they compare equal.</returns>
        public bool Equals(Isdar akhar)
        {
            return Maqru == akhar.Maqru && CompareTo(akhar) == 0;
        }

        /// <inheritdoc/>
        public override bool Equals(object? shay)
        {
            return shay is Isdar akhar && Equals(akhar);
        }

        /// <inheritdoc/>
        public override int GetHashCode()
        {
            unchecked
            {
                int h = 17;
                h = (h * 31) + Kabir;
                h = (h * 31) + Sagheer;
                h = (h * 31) + Tasheeh;
                h = (h * 31) + Martaba(Tawr);
                h = (h * 31) + RaqmTawr;
                h = (h * 31) + RaqmSini;
                return h;
            }
        }

        /// <summary>Renders the version the way it was written.</summary>
        /// <returns>The raw string, or a rendering of the components.</returns>
        public override string ToString()
        {
            // Guarded against null rather than against empty, because a struct's
            // default value bypasses every constructor and leaves Khaam null; a
            // refusal sentence rendering an unread version must not itself throw.
            if (!string.IsNullOrEmpty(Khaam))
            {
                return Khaam;
            }
            return Maqru
                ? string.Format(
                    CultureInfo.InvariantCulture, "{0}.{1}.{2}", Kabir, Sagheer, Tasheeh)
                : "(unread)";
        }

        /// <summary>Whether one version is earlier than another.</summary>
        /// <param name="alfa">The left version.</param>
        /// <param name="beta">The right version.</param>
        /// <returns>Whether the left is earlier.</returns>
        public static bool operator <(Isdar alfa, Isdar beta) => alfa.CompareTo(beta) < 0;

        /// <summary>Whether one version is later than another.</summary>
        /// <param name="alfa">The left version.</param>
        /// <param name="beta">The right version.</param>
        /// <returns>Whether the left is later.</returns>
        public static bool operator >(Isdar alfa, Isdar beta) => alfa.CompareTo(beta) > 0;

        /// <summary>Whether one version is not later than another.</summary>
        /// <param name="alfa">The left version.</param>
        /// <param name="beta">The right version.</param>
        /// <returns>Whether the left is earlier or the same.</returns>
        public static bool operator <=(Isdar alfa, Isdar beta) => alfa.CompareTo(beta) <= 0;

        /// <summary>Whether one version is not earlier than another.</summary>
        /// <param name="alfa">The left version.</param>
        /// <param name="beta">The right version.</param>
        /// <returns>Whether the left is later or the same.</returns>
        public static bool operator >=(Isdar alfa, Isdar beta) => alfa.CompareTo(beta) >= 0;

        /// <summary>Whether two versions are the same version.</summary>
        /// <param name="alfa">The left version.</param>
        /// <param name="beta">The right version.</param>
        /// <returns>Whether they are equal.</returns>
        public static bool operator ==(Isdar alfa, Isdar beta) => alfa.Equals(beta);

        /// <summary>Whether two versions are different versions.</summary>
        /// <param name="alfa">The left version.</param>
        /// <param name="beta">The right version.</param>
        /// <returns>Whether they differ.</returns>
        public static bool operator !=(Isdar alfa, Isdar beta) => !alfa.Equals(beta);

        private static bool Adad(string nass, ref int i, out int qeema, out string sabab)
        {
            qeema = 0;
            int bidaya = i;
            while (i < nass.Length && nass[i] >= '0' && nass[i] <= '9')
            {
                i++;
            }
            int tul = i - bidaya;
            if (tul == 0)
            {
                sabab = i < nass.Length
                    ? $"'{nass[i]}' is not a digit"
                    : "the string ends where a number was expected";
                return false;
            }
            if (tul > 6)
            {
                // 6000 is four digits and a Unity year is four; anything past six
                // is a string that is not a version number, and parsing it would
                // overflow before it failed.
                sabab = $"'{nass.Substring(bidaya, tul)}' has {tul} digits, which is not a "
                    + "version component";
                return false;
            }
            if (!int.TryParse(
                    nass.Substring(bidaya, tul),
                    NumberStyles.None,
                    CultureInfo.InvariantCulture,
                    out qeema))
            {
                sabab = $"'{nass.Substring(bidaya, tul)}' is not a number";
                return false;
            }
            sabab = string.Empty;
            return true;
        }
    }

    /// <summary>One end of a version range.</summary>
    /// <remarks>
    /// <para>
    /// A bound is either open — no limit at this end — or a version plus a
    /// bracket saying whether that version's set is inside the range.
    /// </para>
    /// <para>
    /// THE PREFIX RULE, which is the whole of this type's semantics: a bound
    /// written with fewer than three components denotes every version that
    /// begins with it, and the bracket decides whether that set is in or out. So
    /// <c>[2021.3, 2023.1)</c> contains every 2021.3.x through every 2023.0.x and
    /// no 2023.1.x at all, and <c>(2021.3, 2023.1]</c> contains no 2021.3.x and
    /// every 2023.1.x. Written out this way it is one sentence; written as
    /// fabricated zeroes and fabricated maxima it is four cases and two of them
    /// are wrong.
    /// </para>
    /// </remarks>
    public readonly struct HaddIsdar
    {
        private HaddIsdar(Isdar qeema, bool shamil, bool maftuh)
        {
            Qeema = qeema;
            Shamil = shamil;
            Maftuh = maftuh;
        }

        /// <summary>The version at this end. Meaningless when <see cref="Maftuh"/>.</summary>
        public Isdar Qeema { get; }

        /// <summary>Whether the bound's own versions are inside the range.</summary>
        public bool Shamil { get; }

        /// <summary>Whether this end has no limit at all.</summary>
        public bool Maftuh { get; }

        /// <summary>An end with no limit.</summary>
        public static HaddIsdar Maftuha => new HaddIsdar(default, false, true);

        /// <summary>Builds a closed end.</summary>
        /// <param name="qeema">The version.</param>
        /// <param name="shamil">Whether its versions are inside the range.</param>
        /// <returns>The bound.</returns>
        public static HaddIsdar Mughlaqa(Isdar qeema, bool shamil)
        {
            return new HaddIsdar(qeema, shamil, false);
        }

        /// <summary>
        /// How many components this bound names, which is what specificity is
        /// counted from. Zero when open.
        /// </summary>
        public int Diqqa
        {
            get
            {
                if (Maftuh)
                {
                    return 0;
                }
                // A written stage counts as a fourth component: naming
                // 2021.3.16f1 is a narrower claim than naming 2021.3.16, and the
                // difference is exactly the alpha and beta builds of that patch.
                return Qeema.AdadMukawwinat + (Qeema.Tawr == TawrIsdar.Bila ? 0 : 1);
            }
        }

        /// <summary>Whether a version is above this lower bound.</summary>
        /// <param name="isdar">The version.</param>
        /// <returns>Whether it passes.</returns>
        public bool FawqAsfal(in Isdar isdar)
        {
            if (Maftuh)
            {
                return true;
            }
            int farq = isdar.QaranBadia(
                Qeema, Qeema.AdadMukawwinat, Qeema.Tawr != TawrIsdar.Bila);
            return farq > 0 || (farq == 0 && Shamil);
        }

        /// <summary>Whether a version is below this upper bound.</summary>
        /// <param name="isdar">The version.</param>
        /// <returns>Whether it passes.</returns>
        public bool TahtAala(in Isdar isdar)
        {
            if (Maftuh)
            {
                return true;
            }
            int farq = isdar.QaranBadia(
                Qeema, Qeema.AdadMukawwinat, Qeema.Tawr != TawrIsdar.Bila);
            return farq < 0 || (farq == 0 && Shamil);
        }

        /// <summary>Renders this end as the left bracket of a range.</summary>
        /// <returns>The text.</returns>
        public string NassAsfal()
        {
            return Maftuh ? "(" : (Shamil ? "[" : "(") + Qeema.ToString();
        }

        /// <summary>Renders this end as the right bracket of a range.</summary>
        /// <returns>The text.</returns>
        public string NassAala()
        {
            return Maftuh ? ")" : Qeema.ToString() + (Shamil ? "]" : ")");
        }
    }

    /// <summary>
    /// A range of versions, written <c>[2021.3, 2023.1)</c>, <c>[2021.3,)</c>,
    /// <c>(,2019.4]</c>, <c>=2021.3.16f1</c> or <c>*</c>.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <see cref="Diqqa"/> is what orders candidate patterns most specific
    /// first. It counts the components both ends name, favours a range that is
    /// bounded at both ends over one that is not, and puts an exact pin above
    /// everything — the reasoning is on the property itself.
    /// </para>
    /// <para>
    /// A range never contains a version that was not read.
    /// <see cref="Yahwi"/> refuses an <see cref="Isdar"/> whose
    /// <see cref="Isdar.Maqru"/> is false rather than comparing it, which is the
    /// last line of defence behind the parser's refusal.
    /// </para>
    /// </remarks>
    public readonly struct MadaIsdar : IEquatable<MadaIsdar>
    {
        private MadaIsdar(HaddIsdar asfal, HaddIsdar aala, bool mutabaqTam, string khaam)
        {
            Asfal = asfal;
            Aala = aala;
            MutabaqTam = mutabaqTam;
            Khaam = khaam;
        }

        /// <summary>The lower end.</summary>
        public HaddIsdar Asfal { get; }

        /// <summary>The upper end.</summary>
        public HaddIsdar Aala { get; }

        /// <summary>Whether this range pins one exact version.</summary>
        public bool MutabaqTam { get; }

        /// <summary>The range as it was written in the database.</summary>
        public string Khaam { get; }

        /// <summary>
        /// Whether this range is open at both ends, which is the written claim
        /// that the thing it keys does not depend on this version at all.
        /// </summary>
        /// <remarks>
        /// Distinguished from a range that merely happens to contain a version,
        /// because it is the one case where a lookup may proceed with a version
        /// it could not read: <c>*</c> says the pattern was never keyed on that
        /// version, so there is nothing to compare and nothing to get wrong.
        /// </remarks>
        public bool Shamila => Asfal.Maftuh && Aala.Maftuh;

        /// <summary>The range that contains every version, written <c>*</c>.</summary>
        /// <remarks>
        /// Its <see cref="Diqqa"/> is zero, so a pattern keyed on it is always
        /// tried last. That is the correct place for it: <c>*</c> means "this
        /// pattern has been seen to work and nobody has narrowed it yet", which
        /// is useful and is not evidence.
        /// </remarks>
        public static MadaIsdar Ayy =>
            new MadaIsdar(HaddIsdar.Maftuha, HaddIsdar.Maftuha, false, "*");

        /// <summary>
        /// How specific this range is. Higher is more specific and is tried
        /// first.
        /// </summary>
        /// <remarks>
        /// <para>
        /// Ten points per component named by either end, two points for being
        /// bounded at both ends, one point for pinning an exact version.
        /// </para>
        /// <para>
        /// The weights encode a claim worth defending. Components dominate
        /// because naming <c>2021.3.16f1</c> is a statement about one build and
        /// naming <c>2021</c> is a statement about a year of builds, and the
        /// first is evidence where the second is a guess that has held so far.
        /// Both-ends-bounded is the tie-break because <c>[2021, 2022)</c> and
        /// <c>[2021.3,)</c> name the same number of components but the first
        /// cannot silently start applying to Unity 6000 when Unity 6000 ships.
        /// The pin's single point only separates <c>=2021.3.16f1</c> from
        /// <c>[2021.3.16f1, 2021.3.16f1]</c>, which are the same range written
        /// two ways.
        /// </para>
        /// </remarks>
        public int Diqqa
        {
            get
            {
                int natija = (Asfal.Diqqa + Aala.Diqqa) * 10;
                if (!Asfal.Maftuh && !Aala.Maftuh)
                {
                    natija += 2;
                }
                if (MutabaqTam)
                {
                    natija += 1;
                }
                return natija;
            }
        }

        /// <summary>Parses a range, refusing anything it cannot read.</summary>
        /// <param name="nass">The range text.</param>
        /// <returns>The parsed range.</returns>
        /// <exception cref="KhataTaarib">The text is not a range this build reads.</exception>
        public static MadaIsdar Hallil(string nass)
        {
            if (!HawilHallil(nass, out MadaIsdar mada, out string sabab))
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6507",
                    $"تعذّرت قراءة نطاق الإصدارات «{nass}»: {sabab}",
                    $"The version range \"{nass}\" could not be read: {sabab}",
                    Khutwa.TahdithTaarib);
            }
            return mada;
        }

        /// <summary>Parses a range, reporting rather than throwing.</summary>
        /// <param name="nass">The range text.</param>
        /// <param name="mada">The parsed range, when it parsed.</param>
        /// <param name="sabab">Why not, when it did not.</param>
        /// <returns>Whether it parsed.</returns>
        /// <remarks>
        /// Four forms: <c>*</c> for any version; <c>=2021.3.16f1</c> or a bare
        /// <c>2021.3.16f1</c> for an exact pin; and the bracket form
        /// <c>[lo, hi)</c> with either end optionally empty for an open end. An
        /// inverted range — a lower bound above its upper bound — is refused
        /// here rather than silently matching nothing, because a range that
        /// matches nothing is indistinguishable from a range that was never
        /// consulted.
        /// </remarks>
        public static bool HawilHallil(string nass, out MadaIsdar mada, out string sabab)
        {
            mada = default;
            if (string.IsNullOrWhiteSpace(nass))
            {
                sabab = "it is empty; write * for a range that applies to every version";
                return false;
            }

            string khaam = nass.Trim();
            if (khaam == "*")
            {
                mada = Ayy;
                sabab = string.Empty;
                return true;
            }

            char awwal = khaam[0];
            if (awwal != '[' && awwal != '(')
            {
                string wahid = awwal == '=' ? khaam.Substring(1).Trim() : khaam;
                if (!Isdar.HawilHallil(wahid, out Isdar mathbut, out string khalal))
                {
                    sabab = $"'{wahid}' is not a version ({khalal}), and is not a bracketed "
                        + "range either";
                    return false;
                }
                mada = new MadaIsdar(
                    HaddIsdar.Mughlaqa(mathbut, true),
                    HaddIsdar.Mughlaqa(mathbut, true),
                    true,
                    khaam);
                sabab = string.Empty;
                return true;
            }

            char akhir = khaam[khaam.Length - 1];
            if (akhir != ']' && akhir != ')')
            {
                sabab = $"it opens with '{awwal}' but does not close with ']' or ')'";
                return false;
            }

            string jawf = khaam.Substring(1, khaam.Length - 2);
            int fasila = jawf.IndexOf(',');
            if (fasila < 0)
            {
                sabab = "a bracketed range needs a comma between its two ends; write "
                    + "[2021.3, 2023.1) or =2021.3.16f1 for one version";
                return false;
            }
            if (jawf.IndexOf(',', fasila + 1) >= 0)
            {
                sabab = "a bracketed range has exactly two ends, and this one has more than "
                    + "one comma";
                return false;
            }

            string nassAsfal = jawf.Substring(0, fasila).Trim();
            string nassAala = jawf.Substring(fasila + 1).Trim();

            HaddIsdar asfal;
            if (nassAsfal.Length == 0)
            {
                if (awwal == '[')
                {
                    sabab = "an open lower end is written with '(' rather than '[', because "
                        + "there is no version for '[' to include";
                    return false;
                }
                asfal = HaddIsdar.Maftuha;
            }
            else
            {
                if (!Isdar.HawilHallil(nassAsfal, out Isdar qeema, out string khalalAsfal))
                {
                    sabab = $"the lower end '{nassAsfal}' is not a version ({khalalAsfal})";
                    return false;
                }
                asfal = HaddIsdar.Mughlaqa(qeema, awwal == '[');
            }

            HaddIsdar aala;
            if (nassAala.Length == 0)
            {
                if (akhir == ']')
                {
                    sabab = "an open upper end is written with ')' rather than ']', because "
                        + "there is no version for ']' to include";
                    return false;
                }
                aala = HaddIsdar.Maftuha;
            }
            else
            {
                if (!Isdar.HawilHallil(nassAala, out Isdar qeema, out string khalalAala))
                {
                    sabab = $"the upper end '{nassAala}' is not a version ({khalalAala})";
                    return false;
                }
                aala = HaddIsdar.Mughlaqa(qeema, akhir == ']');
            }

            if (!asfal.Maftuh && !aala.Maftuh && asfal.Qeema.CompareTo(aala.Qeema) > 0)
            {
                sabab = $"the lower end {asfal.Qeema} is above the upper end {aala.Qeema}, so "
                    + "the range contains nothing";
                return false;
            }

            bool pin = !asfal.Maftuh
                && !aala.Maftuh
                && asfal.Shamil
                && aala.Shamil
                && asfal.Qeema.CompareTo(aala.Qeema) == 0;

            mada = new MadaIsdar(asfal, aala, pin, khaam);
            sabab = string.Empty;
            return true;
        }

        /// <summary>Whether a version falls inside this range.</summary>
        /// <param name="isdar">The version.</param>
        /// <returns>Whether it is contained.</returns>
        /// <remarks>
        /// A version that was never read is contained by nothing, including
        /// <see cref="Ayy"/>. That is deliberate: an unread version reaching a
        /// range check means a caller skipped the parser's refusal, and the
        /// worst possible answer at that point is "yes, apply the pattern".
        /// </remarks>
        public bool Yahwi(in Isdar isdar)
        {
            if (!isdar.Maqru)
            {
                return false;
            }
            return Asfal.FawqAsfal(in isdar) && Aala.TahtAala(in isdar);
        }

        /// <summary>Whether two ranges are written the same way.</summary>
        /// <param name="akhar">The other range.</param>
        /// <returns>Whether they are equal.</returns>
        public bool Equals(MadaIsdar akhar)
        {
            return string.Equals(ToString(), akhar.ToString(), StringComparison.Ordinal);
        }

        /// <inheritdoc/>
        public override bool Equals(object? shay)
        {
            return shay is MadaIsdar akhar && Equals(akhar);
        }

        /// <inheritdoc/>
        public override int GetHashCode()
        {
            return StringComparer.Ordinal.GetHashCode(ToString());
        }

        /// <summary>Renders the range the way it was written.</summary>
        /// <returns>The raw text, or a canonical rendering.</returns>
        public override string ToString()
        {
            if (!string.IsNullOrEmpty(Khaam))
            {
                return Khaam;
            }
            if (Asfal.Maftuh && Aala.Maftuh)
            {
                return "*";
            }
            return $"{Asfal.NassAsfal()}, {Aala.NassAala()}";
        }

        /// <summary>Whether two ranges are written differently.</summary>
        /// <param name="alfa">The left range.</param>
        /// <param name="beta">The right range.</param>
        /// <returns>Whether they differ.</returns>
        public static bool operator !=(MadaIsdar alfa, MadaIsdar beta) => !alfa.Equals(beta);

        /// <summary>Whether two ranges are written the same way.</summary>
        /// <param name="alfa">The left range.</param>
        /// <param name="beta">The right range.</param>
        /// <returns>Whether they are equal.</returns>
        public static bool operator ==(MadaIsdar alfa, MadaIsdar beta) => alfa.Equals(beta);
    }
}
