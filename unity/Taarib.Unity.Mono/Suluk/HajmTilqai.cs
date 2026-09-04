// حجم تلقائي — auto-sizing, taken over, because the component's own is
// measuring the wrong string in the wrong font.
//
// WHAT THE COMPONENT THINKS IT IS DOING. Every Unity text system ships a
// shrink-to-fit: TextMeshPro's enableAutoSizing with fontSizeMin and
// fontSizeMax, uGUI's resizeTextForBestFit with resizeTextMinSize and
// resizeTextMaxSize. Each of them measures the string with its own font asset,
// finds it too wide for the rectangle, drops the size and measures again.
//
// WHY THAT BREAKS THE MOMENT TAARIB IS INVOLVED. The takeover replaces what is
// drawn, not what the component believes it holds. The component's auto-size
// pass therefore runs against the ORIGINAL string — the English the game
// shipped — in the ORIGINAL font asset, and settles on a size that fits that.
// It then hands that size to a pipeline drawing different text, in a different
// font, with different advances, different joining and different line breaking.
// The number is arrived at correctly and it is an answer to a question nobody
// asked. Its two failure modes are equally bad: a size too large, and Arabic
// that clips out of the box the compiler measured; or a size too small, and a
// paragraph shrunk to fit text that was never going to be drawn.
//
// So the size is computed here, from Taarib's own measurement of the text that
// is really going to be drawn, and the component's own search is prevented from
// running. There is no way to correct the component's answer after the fact —
// it is not off by a factor, it is about a different string.
//
// WHY A BINARY SEARCH AND NOT A STEP-DOWN. The measurement is the whole cost.
// Every candidate size is a shaping pass across the ABI: bidi resolution, the
// itemizer, HarfBuzz over each run, line breaking. Everything around it —
// picking the next candidate, comparing two floats — is free by comparison, so
// the only number that matters is how many measurements the search performs.
// Unity's own auto-size steps down a point at a time, which over a ten-to-forty
// point range is thirty measurements. Halving the interval instead is five or
// six. This file searches the quarter-pixel grid rather than whole points, so
// the same range costs two bracketing probes and about seven halvings — nine
// measurements for four times the resolution, against a hundred and twenty for
// the equivalent step-down. On a menu that opens with forty auto-sized labels
// the difference is the frame the menu appears on.
//
// WHY THE RESULT IS QUANTIZED TO QUARTER-PIXELS. Because that is the unit the
// rest of the product already keys on. TaaribMiftahShakl.HajmRubi is the size
// field of a glyph image in the atlas, MadkhalTakhtit.HajmRubi is the size a
// precomputed layout was compiled at, and Nasij.HajmRubi is the one function
// that produces either. A search that settled on 23.7183 pixels would ask the
// atlas for glyph images at a key nothing has ever been rasterized under: every
// glyph of the string misses, every one of them is rasterized and packed on the
// spot, and the atlas grows — on the frame the label appeared, for a size that
// will never be asked for again because the next string lands on a different
// fraction. Snapping to the grid the atlas and the compiler already use turns
// that into a hit. Note also that this is the SAME rounding function the
// residency path calls: two implementations of one rounding rule eventually
// round differently, and the symptom of that is a glyph that was rasterized and
// is still not found.
//
// TWO RULES THAT ARE NOT OPTIMIZATIONS.
//
//   - WHERE THE PATCH ALREADY LAID THE STRING OUT, USE THAT. A compiled patch
//     may carry the string laid out at several sizes, each with its own
//     measured width and height and its own overflow flag. Picking the largest
//     of those that fits inside the permitted range answers the whole question
//     with a binary search over a memory-mapped file and no shaping at all.
//     Preferring it is also the only way the runtime and the compiler cannot
//     disagree: the compiler's overflow report was written against those
//     numbers, and a runtime that re-derived its own would occasionally choose
//     a different size than the report a translator signed off on.
//
//   - WHERE THE CONSTRAINT SAYS AUTO-SIZING IS OFF, DO NOTHING. A constraint
//     record without ALAM_QAYD_HAJM_TILQAI means the compiler observed a
//     fixed-size element. Searching anyway would shrink text the game draws at
//     a fixed size — quietly, and inconsistently with the identical label
//     beside it. The answer in that case is the requested size, quantized,
//     with MasdarHajm.Muattal saying plainly where it came from.
//
// WHAT ALLOCATES. Nothing on the search path. Takhtit.Qis reuses its own text
// buffer and returns a struct by value; every candidate is an int; the result
// is a struct. The first measurement of a string longer than any before grows
// that shared text buffer once, which is the same single growth the layout path
// pays, not an extra one.

using System;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Mono.Suluk
{
    /// <summary>
    /// مصدر الحجم — where a chosen size came from, recorded rather than
    /// inferred.
    /// </summary>
    /// <remarks>
    /// A diagnostics field, and the honest answer to "why is this label that
    /// size". Without it, a size that came from a precomputed layout and a size
    /// that was clamped to the minimum because nothing fit are the same number
    /// with the same confidence, and the second one is a string that needs a
    /// translator's attention.
    /// </remarks>
    public enum MasdarHajm
    {
        /// <summary>
        /// Auto-sizing is off for this element. The size is the one the
        /// constraint records, quantized, and nothing was measured.
        /// </summary>
        Muattal = 0,

        /// <summary>
        /// A layout the patch compiler already produced for this string, at a
        /// size inside the permitted range, that fits. Nothing was measured.
        /// </summary>
        Muhaddad = 1,

        /// <summary>The search ran and found the largest size that fits.</summary>
        Bahth = 2,

        /// <summary>
        /// Nothing in the permitted range fits, so the minimum was taken and
        /// the overflow policy applies on top of it. The string belongs in the
        /// overflow report.
        /// </summary>
        Adna = 3,

        /// <summary>
        /// A measurement failed. The requested size stands, unmodified, which
        /// is what the game would have drawn.
        /// </summary>
        Fashil = 4,
    }

    /// <summary>
    /// قيد الحجم — the search's bounds and the box it is searching inside.
    /// </summary>
    /// <remarks>
    /// Assembled from the patch's constraint record and the component's own
    /// maximum. The two halves come from different places on purpose: the
    /// minimum, the box and the layout policy are what the compiler measured
    /// and what a translator reviewed, while the maximum is whatever the live
    /// component is configured with — a game that raises fontSizeMax at run
    /// time for an accessibility setting is allowed to, and the search follows
    /// it up.
    /// </remarks>
    public struct QaydHajm
    {
        /// <summary>
        /// Whether auto-sizing is on at all. False means
        /// <see cref="MasdarHajm.Muattal"/> and no measurement.
        /// </summary>
        public bool Mumakkan;

        /// <summary>
        /// The size the element is drawn at when nothing shrinks it, in pixels.
        /// </summary>
        public float HajmMatlub;

        /// <summary>
        /// The smallest size the search may return, in pixels. Below one it is
        /// treated as one: a size of zero collapses every glyph to a point, and
        /// a search whose lower bound produces no geometry always "fits".
        /// </summary>
        public float HajmAdna;

        /// <summary>
        /// The largest size the search may return, in pixels — the component's
        /// own maximum. Zero or less falls back to
        /// <see cref="HajmMatlub"/>.
        /// </summary>
        public float HajmAqsa;

        /// <summary>The width available in pixels; zero or less is unbounded.</summary>
        public float ArdMutah;

        /// <summary>The height available in pixels; zero or less is unbounded.</summary>
        public float IrtifaMutah;

        /// <summary>
        /// The rest of the layout decisions — direction, language, alignment,
        /// diacritics, digits, spacing. Taken from the constraint so a measured
        /// candidate is measured exactly the way the string will be drawn.
        /// </summary>
        public KhiyaratTakhtit Khiyarat;

        /// <summary>
        /// Builds the bounds from a patch constraint and the live component's
        /// maximum.
        /// </summary>
        /// <param name="qayd">The constraint the compiler recorded.</param>
        /// <param name="hajmAqsa">
        /// The component's own maximum size in pixels, from
        /// <see cref="RabtHajm"/>. Zero or less means the component did not say,
        /// and the constraint's own size becomes the ceiling.
        /// </param>
        /// <returns>The bounds.</returns>
        public static QaydHajm Min(in MadkhalQayd qayd, float hajmAqsa)
        {
            QaydHajm q = default;
            q.Mumakkan = qayd.HajmTilqai;
            q.HajmMatlub = qayd.Hajm;
            q.HajmAdna = qayd.HajmAdna;
            q.HajmAqsa = hajmAqsa > 0f ? hajmAqsa : qayd.Hajm;
            q.ArdMutah = qayd.ArdMutah;
            q.IrtifaMutah = qayd.IrtifaMutah;
            q.Khiyarat = qayd.Khiyarat();
            return q;
        }
    }

    /// <summary>
    /// نتيجة الحجم — the size that was chosen, what it measured, and what it
    /// cost.
    /// </summary>
    public struct NatijatHajm
    {
        /// <summary>The chosen size in pixels, always on the quarter-pixel grid.</summary>
        public float Hajm;

        /// <summary>
        /// The same size in quarter-pixels — the number that keys the atlas and
        /// the precomputed layouts. Handed on rather than recomputed, so no
        /// caller has to round it a second time.
        /// </summary>
        public ushort HajmRubi;

        /// <summary>
        /// Whether the text fits its box at <see cref="Hajm"/>. False means the
        /// overflow policy applies on top; it is not a failure of the search but
        /// a measurement of the string.
        /// </summary>
        /// <remarks>
        /// Meaningful when <see cref="Masdar"/> is <see cref="MasdarHajm.Bahth"/>,
        /// <see cref="MasdarHajm.Adna"/> or <see cref="MasdarHajm.Muhaddad"/>.
        /// For <see cref="MasdarHajm.Muattal"/> and
        /// <see cref="MasdarHajm.Fashil"/> nothing was measured and this reads
        /// as <c>true</c> — no measurement contradicted the size, which is not
        /// the same claim as the text fitting, and <see cref="Masdar"/> is where
        /// a caller learns the difference.
        /// </remarks>
        public bool Yalaim;

        /// <summary>The measured width of the widest line at that size, in pixels.</summary>
        public float Ard;

        /// <summary>The measured total height at that size, in pixels.</summary>
        public float Irtifa;

        /// <summary>How many lines the text needed at that size.</summary>
        public int AdadSutur;

        /// <summary>
        /// How many measurements this answer cost. Zero for a size that came
        /// from the patch or from auto-sizing being off; a handful for a search.
        /// A number that climbs on a screen that is not changing means the
        /// caller is searching every frame instead of caching the answer.
        /// </summary>
        public int AdadQiyasat;

        /// <summary>Where the size came from.</summary>
        public MasdarHajm Masdar;
    }

    /// <summary>
    /// ربط الحجم — the auto-size members of one text-component family,
    /// resolved once at startup and held as delegates.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Two families, and they disagree about the type of a font size.
    /// <c>TMPro.TMP_Text</c> carries <c>fontSize</c>, <c>fontSizeMin</c> and
    /// <c>fontSizeMax</c> as <see cref="float"/>; <c>UnityEngine.UI.Text</c>
    /// carries <c>fontSize</c>, <c>resizeTextMinSize</c> and
    /// <c>resizeTextMaxSize</c> as <see cref="int"/>. A binding that asked for
    /// a float from the second would resolve nothing and switch auto-sizing off
    /// for every legacy label in the game over a type mismatch, so both widths
    /// are bound and <see cref="Sahih"/> says which one answered.
    /// </para>
    /// <para>
    /// Nothing here throws when a member is missing. A family that cannot
    /// supply a maximum degrades to using the constraint's own size as the
    /// ceiling — a smaller search over a narrower range, still correct — and
    /// the other family never learns about it.
    /// </para>
    /// </remarks>
    public sealed class RabtHajm
    {
        private RabtHajm(Type naw, string ism, string ismTamkin, string ismAdna, string ismAqsa)
        {
            Naw = naw;
            Ism = ism;
            QariTamkin = Rabt.Qari<bool>(naw, ismTamkin);
            KatibTamkin = Rabt.Katib<bool>(naw, ismTamkin);

            QariHajm = Rabt.Qari<float>(naw, "fontSize");
            KatibHajm = Rabt.Katib<float>(naw, "fontSize");
            QariAdna = Rabt.Qari<float>(naw, ismAdna);
            QariAqsa = Rabt.Qari<float>(naw, ismAqsa);

            if (QariHajm is null)
            {
                QariHajmSahih = Rabt.Qari<int>(naw, "fontSize");
                KatibHajmSahih = Rabt.Katib<int>(naw, "fontSize");
                QariAdnaSahih = Rabt.Qari<int>(naw, ismAdna);
                QariAqsaSahih = Rabt.Qari<int>(naw, ismAqsa);
                Sahih = QariHajmSahih is not null;
            }
        }

        /// <summary>The game's own text-component type.</summary>
        public Type Naw { get; }

        /// <summary>The family's short name, for the one log line a degradation writes.</summary>
        public string Ism { get; }

        /// <summary>
        /// Whether this family expresses sizes as integers. True for uGUI's
        /// <c>Text</c>, false for TextMeshPro.
        /// </summary>
        public bool Sahih { get; }

        /// <summary>Reads whether the component's own auto-sizing is switched on.</summary>
        public Func<object, bool>? QariTamkin { get; }

        /// <summary>
        /// Writes it. Used for exactly one thing: switching the component's own
        /// search off, so it cannot run its measurement of the original string
        /// alongside this one and win the last write.
        /// </summary>
        public Action<object, bool>? KatibTamkin { get; }

        /// <summary>Reads the component's size as a float.</summary>
        public Func<object, float>? QariHajm { get; }

        /// <summary>Writes it as a float.</summary>
        public Action<object, float>? KatibHajm { get; }

        /// <summary>Reads the component's configured minimum as a float.</summary>
        public Func<object, float>? QariAdna { get; }

        /// <summary>Reads the component's configured maximum as a float.</summary>
        public Func<object, float>? QariAqsa { get; }

        /// <summary>Reads the component's size as an integer.</summary>
        public Func<object, int>? QariHajmSahih { get; }

        /// <summary>Writes it as an integer.</summary>
        public Action<object, int>? KatibHajmSahih { get; }

        /// <summary>Reads the component's configured minimum as an integer.</summary>
        public Func<object, int>? QariAdnaSahih { get; }

        /// <summary>Reads the component's configured maximum as an integer.</summary>
        public Func<object, int>? QariAqsaSahih { get; }

        /// <summary>
        /// Whether enough resolved to be useful: a readable size in one of the
        /// two widths.
        /// </summary>
        public bool Muakkad => QariHajm is not null || QariHajmSahih is not null;

        /// <summary>
        /// Binds <c>TMPro.TMP_Text</c>, or reports why it could not be bound and
        /// returns <c>null</c>.
        /// </summary>
        /// <returns>The binding, or <c>null</c> when this game has no TextMeshPro.</returns>
        public static RabtHajm? Tmp()
        {
            return Iqran(
                "TMPro.TMP_Text", "TMP_Text", "enableAutoSizing", "fontSizeMin", "fontSizeMax");
        }

        /// <summary>
        /// Binds <c>UnityEngine.UI.Text</c>, or reports why it could not be
        /// bound and returns <c>null</c>.
        /// </summary>
        /// <returns>The binding, or <c>null</c> when this game has no uGUI text.</returns>
        public static RabtHajm? Waajiha()
        {
            return Iqran(
                "UnityEngine.UI.Text",
                "Text",
                "resizeTextForBestFit",
                "resizeTextMinSize",
                "resizeTextMaxSize");
        }

        /// <summary>
        /// Whether a live object is an instance of this family.
        /// </summary>
        /// <param name="mukawwin">The candidate object.</param>
        /// <returns>Whether it is one of these.</returns>
        public bool Yantami(object? mukawwin)
        {
            return mukawwin is not null && Naw.IsInstanceOfType(mukawwin);
        }

        /// <summary>
        /// The component's configured maximum size in pixels, or zero when this
        /// build does not expose one.
        /// </summary>
        /// <param name="mukawwin">The component.</param>
        /// <returns>The maximum, or zero.</returns>
        public float HajmAqsa(object mukawwin)
        {
            return Iqra(QariAqsa, QariAqsaSahih, mukawwin);
        }

        /// <summary>
        /// The component's configured minimum size in pixels, or zero when this
        /// build does not expose one.
        /// </summary>
        /// <param name="mukawwin">The component.</param>
        /// <returns>The minimum, or zero.</returns>
        public float HajmAdna(object mukawwin)
        {
            return Iqra(QariAdna, QariAdnaSahih, mukawwin);
        }

        /// <summary>The component's current size in pixels, or zero.</summary>
        /// <param name="mukawwin">The component.</param>
        /// <returns>The size, or zero.</returns>
        public float Hajm(object mukawwin)
        {
            return Iqra(QariHajm, QariHajmSahih, mukawwin);
        }

        /// <summary>
        /// Writes the size back onto the component, rounding for a family that
        /// stores it as an integer.
        /// </summary>
        /// <remarks>
        /// Rounding, not truncating. A search that settled on 17.75 written into
        /// an integer property as 17 draws a whole point smaller than it
        /// measured, which is a visible step on a small label and reads as the
        /// search being wrong rather than as the property being narrow.
        /// </remarks>
        /// <param name="mukawwin">The component.</param>
        /// <param name="hajm">The size in pixels.</param>
        /// <returns>Whether it was written.</returns>
        public bool AktubHajm(object mukawwin, float hajm)
        {
            try
            {
                if (KatibHajm is not null)
                {
                    KatibHajm(mukawwin, hajm);
                    return true;
                }
                if (KatibHajmSahih is not null)
                {
                    double mudawwar = Math.Floor((double)hajm + 0.5);
                    int sahih = mudawwar < 1.0 ? 1 : (mudawwar > int.MaxValue
                        ? int.MaxValue
                        : (int)mudawwar);
                    KatibHajmSahih(mukawwin, sahih);
                    return true;
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(Ism + ": writing the font size threw; this element keeps the "
                    + "size the game gave it", khata);
            }
            return false;
        }

        /// <summary>
        /// Switches the component's own auto-size search off, so it cannot
        /// measure the original string and overwrite this file's answer.
        /// </summary>
        /// <param name="mukawwin">The component.</param>
        /// <returns>Whether it was switched off.</returns>
        public bool AwqifBahthaHu(object mukawwin)
        {
            Action<object, bool>? katib = KatibTamkin;
            if (katib is null)
            {
                return false;
            }
            try
            {
                katib(mukawwin, false);
                return true;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(Ism + ": switching the component's own auto-sizing off threw; it "
                    + "may re-measure the original string and overwrite the chosen size", khata);
                return false;
            }
        }

        private static float Iqra(
            Func<object, float>? aashari, Func<object, int>? sahih, object mukawwin)
        {
            if (mukawwin is null)
            {
                return 0f;
            }
            try
            {
                if (aashari is not null)
                {
                    return aashari(mukawwin);
                }
                if (sahih is not null)
                {
                    return sahih(mukawwin);
                }
            }
            catch (Exception)
            {
                // A property getter on a game component that throws is the
                // game's problem; zero means "did not say", which every caller
                // here already handles.
            }
            return 0f;
        }

        private static RabtHajm? Iqran(
            string ismKamil, string ismQasir, string ismTamkin, string ismAdna, string ismAqsa)
        {
            Type? naw = Rabt.Naw(ismKamil);
            if (naw is null)
            {
                return null;
            }
            RabtHajm rabt = new RabtHajm(naw, ismQasir, ismTamkin, ismAdna, ismAqsa);
            if (!rabt.Muakkad)
            {
                Rabt.Ballagh(
                    ismQasir + " is present but its font-size members did not resolve; "
                    + "auto-sizing is off for this component type only, and the rest of the "
                    + "takeover is unaffected.");
                return null;
            }
            if (rabt.QariAqsa is null && rabt.QariAqsaSahih is null)
            {
                Rabt.Ballagh(
                    ismQasir + " has no " + ismAqsa + " in this build; the auto-size search "
                    + "uses the patch's own recorded size as its ceiling, which is the size "
                    + "the game draws the original at.");
            }
            return rabt;
        }
    }

    /// <summary>
    /// حجم تلقائي — the size search: the largest quarter-pixel size in the
    /// permitted range at which the real text fits its real box.
    /// </summary>
    /// <remarks>
    /// <para>
    /// This object owns its <see cref="Takhtit"/> and must not share one with a
    /// renderer or an input field. A <see cref="Takhtit"/> exposes its results
    /// as spans over one reused pair of arrays, and although the search only
    /// ever calls <see cref="Takhtit.Qis"/> — which writes no glyphs — it does
    /// reuse the same UTF-8 text buffer, so a measurement taken while a mesh
    /// writer is halfway through the glyph array would be encoding over memory
    /// that is being read.
    /// </para>
    /// <para>
    /// One instance serves one thread, for the same reason the layout buffer
    /// does.
    /// </para>
    /// </remarks>
    public sealed class HajmTilqai
    {
        /// <summary>
        /// A hard ceiling on halvings. The loop below is provably logarithmic,
        /// so this can only fire if the bounds themselves are corrupt — and a
        /// search that spins inside a frame is a frozen game, which is worse
        /// than a size that is off.
        /// </summary>
        public const int AqsaKhatwat = 32;

        /// <summary>
        /// The slack a measurement is allowed against its box, in pixels.
        /// </summary>
        /// <remarks>
        /// Measured widths are the sum of a few hundred floating-point
        /// advances, and a line that comes out a hundredth of a pixel over its
        /// box has not overflowed — it has accumulated rounding. Refusing it
        /// costs a whole quarter-pixel step of size, permanently, for something
        /// no eye can resolve.
        /// </remarks>
        public const float Samaha = 0.01f;

        private readonly MaqbadSiyaq siyaq;
        private readonly MaqbadSilsila silsila;
        private readonly Takhtit takhtit;

        /// <summary>
        /// Binds the search to a context, a font chain and a layout buffer of
        /// its own.
        /// </summary>
        /// <param name="siyaq">The engine context.</param>
        /// <param name="silsila">The font chain the text is drawn with.</param>
        /// <param name="takhtit">
        /// A layout buffer used by this searcher and nothing else.
        /// </param>
        /// <exception cref="ArgumentNullException">An argument is null.</exception>
        public HajmTilqai(MaqbadSiyaq siyaq, MaqbadSilsila silsila, Takhtit takhtit)
        {
            this.siyaq = siyaq ?? throw new ArgumentNullException(nameof(siyaq));
            this.silsila = silsila ?? throw new ArgumentNullException(nameof(silsila));
            this.takhtit = takhtit ?? throw new ArgumentNullException(nameof(takhtit));
        }

        /// <summary>
        /// The size for one string in one box, measured through Taarib's own
        /// measurement entry point.
        /// </summary>
        /// <param name="nass">The text that will really be drawn, in logical order.</param>
        /// <param name="qayd">The bounds and the box.</param>
        /// <param name="nitaqat">
        /// The style spans, with byte offsets into the UTF-8 encoding of
        /// <paramref name="nass"/>. Empty means one unstyled run.
        /// </param>
        /// <returns>The chosen size, what it measured, and where it came from.</returns>
        public NatijatHajm Ihsib(
            ReadOnlySpan<char> nass,
            in QaydHajm qayd,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat)
        {
            if (!qayd.Mumakkan)
            {
                // Auto-sizing is off for this element. Saying so is the whole
                // behaviour: no measurement, no shrink, and the size the
                // constraint recorded, on the grid.
                return Thabit(qayd.HajmMatlub, MasdarHajm.Muattal);
            }

            int adna = Nasij.HajmRubi(qayd.HajmAdna > 0f ? qayd.HajmAdna : 1f);
            int aqsa = Nasij.HajmRubi(qayd.HajmAqsa > 0f ? qayd.HajmAqsa : qayd.HajmMatlub);
            if (aqsa < adna)
            {
                // A constraint whose minimum is above the component's maximum
                // is a contradiction the search cannot resolve; the minimum
                // wins, because it is the number a translator reviewed.
                aqsa = adna;
            }

            NatijatHajm natija = default;
            try
            {
                if (Yalaim(nass, in qayd, nitaqat, aqsa, ref natija))
                {
                    // The common case, and the reason the top of the range is
                    // probed first: most strings fit at full size and cost one
                    // measurement rather than a whole search.
                    natija.Masdar = MasdarHajm.Bahth;
                    return natija;
                }
                if (aqsa == adna || !Yalaim(nass, in qayd, nitaqat, adna, ref natija))
                {
                    // Nothing in the permitted range fits. The minimum stands
                    // and the overflow policy — truncate, shrink further, or
                    // report — applies on top of it. This is the string the
                    // overflow report exists to name.
                    natija.Masdar = MasdarHajm.Adna;
                    return natija;
                }

                int munkhafid = adna;
                int murtafi = aqsa;
                for (int khatwa = 0; murtafi - munkhafid > 1 && khatwa < AqsaKhatwat; khatwa++)
                {
                    int wasat = munkhafid + ((murtafi - munkhafid) >> 1);
                    if (Yalaim(nass, in qayd, nitaqat, wasat, ref natija))
                    {
                        munkhafid = wasat;
                    }
                    else
                    {
                        murtafi = wasat;
                    }
                }

                if (natija.HajmRubi != munkhafid)
                {
                    // The last probe of the search was a size that did not fit,
                    // so the recorded measurement belongs to a candidate that
                    // was rejected. One more measurement puts the reported
                    // width, height and line count back in step with the size
                    // being returned — a result that reported the wrong pair
                    // would send a reflow decision the wrong numbers.
                    Yalaim(nass, in qayd, nitaqat, munkhafid, ref natija);
                }
                natija.Masdar = MasdarHajm.Bahth;
                return natija;
            }
            catch (KhataTaarib khata)
            {
                Rabt.Ballagh("HajmTilqai: measuring a candidate size failed; this element "
                    + "keeps the size the patch recorded", khata);
                return Fashil(in qayd, natija.AdadQiyasat);
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("HajmTilqai: measuring a candidate size threw; this element "
                    + "keeps the size the patch recorded", khata);
                return Fashil(in qayd, natija.AdadQiyasat);
            }
        }

        /// <summary>
        /// The size for one string, preferring a layout the patch compiler
        /// already produced over measuring anything.
        /// </summary>
        /// <remarks>
        /// The precomputed path costs a binary search over a memory-mapped file
        /// and no shaping at all. It is tried first for that reason and kept
        /// for a better one: the compiler's overflow report was written against
        /// exactly these numbers, and a runtime that re-derived its own would
        /// occasionally pick a different size than the report a translator
        /// approved.
        /// </remarks>
        /// <param name="ruqaa">
        /// The open patch, or <c>null</c> to skip straight to the search.
        /// </param>
        /// <param name="fahras">
        /// The string's index in the patch's string table, from
        /// <see cref="Ruqaa.JidNass(ulong)"/>. Negative means the string is not
        /// in the patch, which is the ordinary case for composed text.
        /// </param>
        /// <param name="nass">The text that will really be drawn.</param>
        /// <param name="qayd">The bounds and the box.</param>
        /// <param name="nitaqat">The style spans.</param>
        /// <returns>The chosen size, what it measured, and where it came from.</returns>
        public NatijatHajm Ihsib(
            Ruqaa? ruqaa,
            int fahras,
            ReadOnlySpan<char> nass,
            in QaydHajm qayd,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat)
        {
            if (!qayd.Mumakkan)
            {
                return Thabit(qayd.HajmMatlub, MasdarHajm.Muattal);
            }
            if (ruqaa is not null && fahras >= 0
                && MinRuqaa(ruqaa, fahras, in qayd, out NatijatHajm mahfuz))
            {
                return mahfuz;
            }
            return Ihsib(nass, in qayd, nitaqat);
        }

        /// <summary>
        /// The largest precomputed layout for one string that sits inside the
        /// permitted range and fits its box.
        /// </summary>
        /// <param name="ruqaa">The open patch.</param>
        /// <param name="fahras">The string's index.</param>
        /// <param name="qayd">The bounds.</param>
        /// <param name="natija">The answer, when one was found.</param>
        /// <returns>Whether the patch could answer without a measurement.</returns>
        public static bool MinRuqaa(
            Ruqaa ruqaa, int fahras, in QaydHajm qayd, out NatijatHajm natija)
        {
            natija = default;
            if (ruqaa is null || fahras < 0)
            {
                return false;
            }

            ReadOnlySpan<MadkhalTakhtit> jadwal = ruqaa.TakhtitatNass(fahras);
            if (jadwal.IsEmpty)
            {
                return false;
            }

            int adna = Nasij.HajmRubi(qayd.HajmAdna > 0f ? qayd.HajmAdna : 1f);
            int aqsa = Nasij.HajmRubi(qayd.HajmAqsa > 0f ? qayd.HajmAqsa : qayd.HajmMatlub);
            if (aqsa < adna)
            {
                aqsa = adna;
            }

            int mukhtar = -1;
            for (int i = 0; i < jadwal.Length; i++)
            {
                int rubi = jadwal[i].HajmRubi;
                if (rubi < adna || rubi > aqsa)
                {
                    continue;
                }
                // A layout the compiler flagged as overflowing did not fit the
                // box it was measured in, so choosing it would be choosing a
                // size that is known not to work.
                if (jadwal[i].Mutajawiz)
                {
                    continue;
                }
                if (mukhtar < 0 || rubi > jadwal[mukhtar].HajmRubi)
                {
                    mukhtar = i;
                }
            }
            if (mukhtar < 0)
            {
                return false;
            }

            MadkhalTakhtit madkhal = jadwal[mukhtar];
            natija.HajmRubi = madkhal.HajmRubi;
            natija.Hajm = madkhal.Hajm;
            natija.Ard = madkhal.Ard;
            natija.Irtifa = madkhal.Irtifa;
            natija.AdadSutur = (int)madkhal.AdadSutur;
            natija.Yalaim = true;
            natija.AdadQiyasat = 0;
            natija.Masdar = MasdarHajm.Muhaddad;
            return true;
        }

        /// <summary>
        /// Whether a candidate size fits, recording what it measured into the
        /// running result.
        /// </summary>
        /// <remarks>
        /// The overflow policy is forced to <see cref="SiyasatTajawuz.Ballagh"/>
        /// for every candidate, whatever the constraint says, and that is not a
        /// detail. A constraint carrying <see cref="SiyasatTajawuz.Taqlis"/>
        /// asks the native side to shrink the text until it fits, so every
        /// candidate would come back fitting and the search would return the
        /// maximum every time — a search that always answers the same thing is
        /// not a search. <see cref="SiyasatTajawuz.Ikhtisar"/> would be worse:
        /// every candidate would fit because the text was truncated to make it,
        /// and the chosen size would be the largest one that could hold an
        /// ellipsis.
        /// </remarks>
        private bool Yalaim(
            ReadOnlySpan<char> nass,
            in QaydHajm qayd,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            int rubi,
            ref NatijatHajm natija)
        {
            TalabTakhtit talab = default;
            talab.Nass = nass;
            talab.Nitaqat = nitaqat;
            talab.Sifat = ReadOnlySpan<TaaribSifa>.Empty;
            talab.Hajm = rubi / 4f;
            talab.ArdMutah = qayd.ArdMutah;
            talab.IrtifaMutah = qayd.IrtifaMutah;
            talab.Khiyarat = qayd.Khiyarat;
            talab.Khiyarat.Tajawuz = SiyasatTajawuz.Ballagh;

            TaaribQiyasNass qiyas = takhtit.Qis(siyaq, silsila, talab);
            natija.AdadQiyasat++;

            bool yalaim = (!(qayd.ArdMutah > 0f) || qiyas.Ard <= qayd.ArdMutah + Samaha)
                && (!(qayd.IrtifaMutah > 0f) || qiyas.Irtifa <= qayd.IrtifaMutah + Samaha);

            natija.HajmRubi = rubi > ushort.MaxValue ? ushort.MaxValue : (ushort)rubi;
            natija.Hajm = talab.Hajm;
            natija.Ard = qiyas.Ard;
            natija.Irtifa = qiyas.Irtifa;
            natija.AdadSutur = (int)qiyas.AdadSutur;
            natija.Yalaim = yalaim;
            return yalaim;
        }

        /// <summary>
        /// The result of a failed search: the size the patch recorded, with the
        /// measurements that were attempted before the failure kept, because a
        /// count of zero would read as "this never measured anything" and send
        /// the next person looking in the wrong place.
        /// </summary>
        private static NatijatHajm Fashil(in QaydHajm qayd, int adadQiyasat)
        {
            NatijatHajm natija = Thabit(qayd.HajmMatlub, MasdarHajm.Fashil);
            natija.AdadQiyasat = adadQiyasat;
            return natija;
        }

        /// <summary>
        /// A result that measured nothing: the given size, quantized, with the
        /// stated provenance and no claim about whether it fits.
        /// </summary>
        private static NatijatHajm Thabit(float hajm, MasdarHajm masdar)
        {
            NatijatHajm natija = default;
            natija.HajmRubi = Nasij.HajmRubi(hajm > 0f ? hajm : 1f);
            natija.Hajm = natija.HajmRubi / 4f;
            natija.Yalaim = true;
            natija.AdadQiyasat = 0;
            natija.Masdar = masdar;
            return natija;
        }
    }
}
