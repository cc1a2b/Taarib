// حقول الإدخال — the caret, the selection, and the two edits, for
// TMP_InputField and UnityEngine.UI.InputField.
//
// This is the one file in Phase 6 where a mistake is visible to anyone typing
// Arabic within a second of pressing a key, so each answer here is written
// against the specific failure it prevents.
//
// THE CARET. Unity stores a caret as an index into a string. Taarib lays text
// out in visual order. The bridge between the two is TaaribHarf.Anqud — the
// byte offset of the cluster a glyph came from — and the rule that maps one to
// the other is TakhtitNass::mawqi_anqud in crates/taarib-saff/src/natija.rs.
// That rule is transcribed here comparison for comparison, over the same glyph
// and line arrays the native layout wrote. The answer therefore comes out of
// the layout rather than out of a second theory about where a caret belongs. A
// second theory is precisely how a field ends up with a caret that agrees with
// the drawn text in Latin and drifts by a letter in Arabic: the moment the two
// implementations disagree about a ligature, a mark, or the side of a run, the
// caret is drawn somewhere the text is not.
//
// THE SELECTION. TakhtitNass::mustatilat_tahdid already computes selection
// rectangles correctly and documents why a selection crossing a direction
// boundary is genuinely two rectangles rather than one. That walk is
// transcribed here with its tolerance constant intact, again over the native
// layout's own glyph array. Drawing such a selection as a single rectangle is
// the bug every text field that has never been used in Arabic ships with: it
// paints over text the user did not select, because the two selected runs are
// not adjacent on screen.
//
// THE EDITS. Insertion and deletion happen at logical byte positions in the
// underlying string, never at visual ones, and deletion removes a whole
// grapheme cluster. A backspace that removes one UTF-16 unit takes a fatha off
// a letter and leaves the letter; a backspace that removes one `char` from a
// letter carrying shadda and fatha takes one mark and leaves the other. Both
// are the same bug seen from two heights, and both leave the user's text in a
// state they cannot see and did not ask for.
//
// ONE HONEST NOTE ABOUT WHERE THE ANSWER COMES FROM. The C ABI of
// crates/taarib-jisr exposes the layout — every glyph with its cluster index,
// every line with its own direction flag — but it does not yet expose
// mawqi_anqud, mustatilat_tahdid or hudud_anaqid as entry points of their own.
// Until it does (one additive minor version: taarib_mawqi_anqud,
// taarib_mustatilat_tahdid, taarib_hudud_anaqid), the caret and selection
// walks below are transcriptions of the Rust, run over the native layout's own
// output, with the Rust's constants copied verbatim and each divergence point
// called out in a comment. Nothing here re-derives a position from the text;
// everything is read off the glyphs the shaper produced.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Reflection;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mono.Suluk
{
    /// <summary>
    /// ربط — the reflection binding the four behaviours of this namespace
    /// share: resolve a game type or member once at startup, hand back a
    /// delegate, and return <c>null</c> rather than throwing when the member
    /// is not in this game's build of TextMeshPro or uGUI.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Every member of this class runs at startup and never on a frame. That
    /// is the whole reason it exists: a property read through
    /// <see cref="PropertyInfo.GetValue(object)"/> allocates an argument array
    /// and boxes the result, and doing that for a caret position sixty times a
    /// second is measurable garbage-collector pressure inside somebody else's
    /// frame budget. A delegate built once costs a call and a cast.
    /// </para>
    /// <para>
    /// It lives in this file rather than in a fifth one because input fields
    /// are its heaviest user and because independence between the four
    /// behaviours is a runtime property, not a file-count property: a binding
    /// that fails returns <c>null</c>, the behaviour that asked for it
    /// switches itself off with a named reason, and the other three never
    /// learn that anything happened.
    /// </para>
    /// </remarks>
    public static class Rabt
    {
        /// <summary>
        /// Public and non-public, instance members only. Non-public is
        /// included deliberately: several of the members these behaviours need
        /// are <c>protected</c> or <c>internal</c> in one version of
        /// TextMeshPro and public in the next, and a binding that refused the
        /// former would degrade a whole behaviour over an access modifier.
        /// </summary>
        private const BindingFlags Alamat =
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;

        private static readonly Dictionary<string, Type?> Anwa = new Dictionary<string, Type?>(32);

        /// <summary>
        /// The sink a missed lookup is reported through while diagnostics are
        /// on, and null otherwise. Set by the plugin from its <c>tashkhis</c>
        /// setting, which promised this line for a build and never delivered it.
        /// </summary>
        public static Action<string>? SijillFawt { get; set; }

        private static readonly HashSet<ulong> Fawtat = new HashSet<ulong>();

        /// <summary>
        /// How many distinct misses are reported before the log goes quiet. A
        /// game that draws thousands of uncovered strings — every number, every
        /// player name — would otherwise turn the log into the string table.
        /// </summary>
        private const int HaddFawtat = 1500;

        private static readonly HashSet<ulong> Isabat = new HashSet<ulong>();

        /// <summary>
        /// How many newly-seen distinct strings pass between coverage lines.
        /// </summary>
        private const int FasilTaghtiya = 100;

        private static int munduAkhirTaqreer;

        /// <summary>
        /// Records a string the patch did cover, so the log can say how much of
        /// what this game draws the patch actually reaches.
        /// </summary>
        /// <remarks>
        /// The miss list on its own answers "what is missing" and not "how
        /// much", and "how much" is the question somebody looking at a
        /// half-Arabic screen is really asking. Distinct strings rather than
        /// draws: a menu rebuilt sixty times a second would otherwise report
        /// its own frame rate as coverage.
        /// </remarks>
        /// <param name="miftah">The key that hit.</param>
        public static void Isaba(ulong miftah)
        {
            if (SijillFawt is null)
            {
                return;
            }
            lock (Fawtat)
            {
                if (!Isabat.Add(miftah))
                {
                    return;
                }
            }
            Taqreer();
        }

        /// <summary>Writes a coverage line once every <see cref="FasilTaghtiya"/>
        /// newly-seen strings.</summary>
        private static void Taqreer()
        {
            Action<string>? sijill = SijillFawt;
            if (sijill is null)
            {
                return;
            }
            int isabat;
            int fawtat;
            lock (Fawtat)
            {
                isabat = Isabat.Count;
                fawtat = Fawtat.Count;
                int majmu = isabat + fawtat;
                if (majmu - munduAkhirTaqreer < FasilTaghtiya)
                {
                    return;
                }
                munduAkhirTaqreer = majmu;
            }
            try
            {
                int majmu = isabat + fawtat;
                int miawiya = majmu == 0 ? 0 : (int)((long)isabat * 100 / majmu);
                sijill(
                    $"التغطية: {isabat} من {majmu} نصًّا مميَّزًا رُسمت من الرقعة ({miawiya}%)"
                    + $" | coverage: {isabat} of {majmu} distinct string(s) drawn so far came from"
                    + $" the patch ({miawiya}%).");
            }
            catch (Exception)
            {
                // Same reason as below: a sink that throws is the host's
                // problem, and a coverage line is not worth a second failure.
            }
        }

        /// <summary>
        /// Reports a string the patch had no entry for, once per distinct
        /// string. Once, because the lookup runs on every rebuild and a menu
        /// rebuilt every frame would write the same line sixty times a second.
        /// </summary>
        /// <param name="nass">The string the game drew.</param>
        /// <param name="miftah">Its key, so the same string is not hashed twice.</param>
        public static void Fawt(string nass, ulong miftah)
        {
            Action<string>? sijill = SijillFawt;
            if (sijill is null)
            {
                return;
            }
            bool jadeed;
            lock (Fawtat)
            {
                if (Fawtat.Count >= HaddFawtat)
                {
                    return;
                }
                jadeed = Fawtat.Add(miftah);
            }
            if (!jadeed)
            {
                return;
            }
            Taqreer();
            try
            {
                sijill("نصّ ليس في الرقعة | not in the patch: " + nass);
            }
            catch (Exception)
            {
                // A logging sink that throws is the host's problem, and a miss
                // report is the last thing worth a second failure.
            }
        }

        /// <summary>
        /// Where a degradation is reported. Assigned once by the plugin to its
        /// BepInEx log sink; left <c>null</c> the reports are discarded, which
        /// is what a unit of this code running outside a game wants.
        /// </summary>
        public static Action<string>? Sijill { get; set; }

        public static void Ballagh(string sabab)
        {
            Action<string>? sijill = Sijill;
            if (sijill is null || string.IsNullOrEmpty(sabab))
            {
                return;
            }
            try
            {
                sijill(sabab);
            }
            catch (Exception)
            {
                // A logging sink that throws is the host's problem, and it is
                // not worth a second failure on top of the one being reported.
            }
        }

        /// <summary>
        /// Reports a degradation with the exception that caused it.
        /// </summary>
        /// <param name="sabab">What failed and what it costs.</param>
        /// <param name="khata">The exception, rendered after the sentence.</param>
        public static void Ballagh(string sabab, Exception khata)
        {
            Ballagh(khata is null ? sabab : sabab + " — " + khata);
        }

        /// <summary>
        /// Resolves a type by its full name across every assembly loaded into
        /// the game process, caching both the hits and the misses.
        /// </summary>
        /// <param name="ism">
        /// The full name, for example <c>TMPro.TMP_InputField</c>. The
        /// assembly is deliberately not named: the same type lives in
        /// <c>Unity.TextMeshPro</c>, in <c>TextMeshPro-1.0.55.55.0</c>, and in
        /// whatever the game's own build renamed it to.
        /// </param>
        /// <returns>The type, or <c>null</c> when this game has no such type.</returns>
        public static Type? Naw(string ism)
        {
            if (string.IsNullOrEmpty(ism))
            {
                return null;
            }
            if (Anwa.TryGetValue(ism, out Type? mahfuz))
            {
                return mahfuz;
            }

            Type? natija = null;
            try
            {
                Assembly[] majmuat = AppDomain.CurrentDomain.GetAssemblies();
                for (int i = 0; i < majmuat.Length && natija is null; i++)
                {
                    try
                    {
                        natija = majmuat[i].GetType(ism, throwOnError: false);
                    }
                    catch (Exception)
                    {
                        // A dynamic or partially loaded assembly may refuse to
                        // answer; the next one still can.
                    }
                }
            }
            catch (Exception khata)
            {
                Ballagh("Enumerating loaded assemblies for " + ism + " failed", khata);
            }

            Anwa[ism] = natija;
            return natija;
        }

        /// <summary>
        /// A property getter as a delegate, or <c>null</c> when this build has
        /// no such property or its type does not match.
        /// </summary>
        /// <typeparam name="TQeema">
        /// The value type read. An enum property may be read as its underlying
        /// integer, which is what lets alignment be inverted without naming
        /// TextMeshPro's enums at compile time.
        /// </typeparam>
        /// <param name="naw">The declaring type, from <see cref="Naw"/>.</param>
        /// <param name="ism">The property name.</param>
        /// <returns>The getter, or <c>null</c>.</returns>
        public static Func<object, TQeema>? Qari<TQeema>(Type? naw, string ism)
        {
            MethodInfo? tariqa = Tariqa(naw, ism, katib: false);
            if (tariqa is null || !Yatalaam(tariqa.ReturnType, typeof(TQeema)))
            {
                return null;
            }
            return (Func<object, TQeema>?)Yabni(
                nameof(QariMuhkam), tariqa, typeof(TQeema), ism);
        }

        /// <summary>
        /// A property setter as a delegate, or <c>null</c> when this build has
        /// no such property, it is read-only, or its type does not match.
        /// </summary>
        /// <typeparam name="TQeema">The value type written.</typeparam>
        /// <param name="naw">The declaring type, from <see cref="Naw"/>.</param>
        /// <param name="ism">The property name.</param>
        /// <returns>The setter, or <c>null</c>.</returns>
        public static Action<object, TQeema>? Katib<TQeema>(Type? naw, string ism)
        {
            MethodInfo? tariqa = Tariqa(naw, ism, katib: true);
            ParameterInfo[]? muamalat = tariqa?.GetParameters();
            if (tariqa is null || muamalat is null || muamalat.Length != 1)
            {
                return null;
            }
            if (!Yatalaam(muamalat[0].ParameterType, typeof(TQeema)))
            {
                return null;
            }
            return (Action<object, TQeema>?)Yabni(
                nameof(KatibMuhkam), tariqa, typeof(TQeema), ism);
        }

        /// <summary>
        /// A parameterless instance method as a delegate — what a rebuild
        /// request or a "the layout changed" notification looks like on a game
        /// component.
        /// </summary>
        /// <param name="naw">The declaring type, from <see cref="Naw"/>.</param>
        /// <param name="ism">The method name.</param>
        /// <returns>The invoker, or <c>null</c>.</returns>
        public static Action<object>? Amr(Type? naw, string ism)
        {
            if (naw is null || string.IsNullOrEmpty(ism))
            {
                return null;
            }
            try
            {
                MethodInfo? tariqa = naw.GetMethod(ism, Alamat, null, Type.EmptyTypes, null);
                if (tariqa is null || tariqa.IsStatic || tariqa.ReturnType != typeof(void))
                {
                    return null;
                }
                return (Action<object>?)Yabni(nameof(AmrMuhkam), tariqa, null, ism);
            }
            catch (Exception khata)
            {
                Ballagh("Binding " + naw.Name + "." + ism + " failed", khata);
                return null;
            }
        }

        /// <summary>
        /// Whether the runtime type of a member is usable as the requested
        /// managed type. Reference types pass on assignability; value types
        /// must match exactly, except that an enum passes as its own
        /// underlying integer — the relaxation the CLR itself applies when
        /// binding a delegate, and the one this namespace depends on to read
        /// <c>TextAlignmentOptions</c> as an <see cref="int"/>.
        /// </summary>
        private static bool Yatalaam(Type mawjud, Type matlub)
        {
            if (mawjud == matlub)
            {
                return true;
            }
            if (mawjud.IsEnum && Enum.GetUnderlyingType(mawjud) == matlub)
            {
                return true;
            }
            return !matlub.IsValueType && !mawjud.IsValueType && matlub.IsAssignableFrom(mawjud);
        }

        /// <summary>
        /// The accessor of a property, searched up the inheritance chain
        /// because uGUI and TextMeshPro both declare half of what these
        /// behaviours read on a base class.
        /// </summary>
        private static MethodInfo? Tariqa(Type? naw, string ism, bool katib)
        {
            if (naw is null || string.IsNullOrEmpty(ism))
            {
                return null;
            }
            try
            {
                for (Type? hali = naw; hali is not null; hali = hali.BaseType)
                {
                    PropertyInfo? khasiya = hali.GetProperty(ism, Alamat | BindingFlags.DeclaredOnly);
                    MethodInfo? tariqa = katib
                        ? khasiya?.GetSetMethod(nonPublic: true)
                        : khasiya?.GetGetMethod(nonPublic: true);
                    if (tariqa is not null && !tariqa.IsStatic)
                    {
                        return tariqa;
                    }
                }
            }
            catch (Exception khata)
            {
                Ballagh("Binding " + naw.Name + "." + ism + " failed", khata);
            }
            return null;
        }

        /// <summary>
        /// Closes one of the three generic shims below over the target type
        /// the member was declared on, which is the only way to build a
        /// strongly typed open delegate for a type this assembly cannot name.
        /// Runs once per member, at startup.
        /// </summary>
        private static Delegate? Yabni(string shim, MethodInfo tariqa, Type? qeema, string ism)
        {
            try
            {
                Type hadaf = tariqa.DeclaringType ?? typeof(object);
                MethodInfo? aam = typeof(Rabt).GetMethod(
                    shim, BindingFlags.NonPublic | BindingFlags.Static);
                if (aam is null)
                {
                    return null;
                }
                MethodInfo mughlaq = qeema is null
                    ? aam.MakeGenericMethod(hadaf)
                    : aam.MakeGenericMethod(hadaf, qeema);
                return mughlaq.Invoke(null, new object[] { tariqa }) as Delegate;
            }
            catch (Exception khata)
            {
                Ballagh("Building a delegate for " + ism + " failed", khata);
                return null;
            }
        }

        private static Func<object, TQeema> QariMuhkam<THadaf, TQeema>(MethodInfo tariqa)
            where THadaf : class
        {
            var mubashir =
                (Func<THadaf, TQeema>)tariqa.CreateDelegate(typeof(Func<THadaf, TQeema>));
            return hadaf => mubashir((THadaf)hadaf);
        }

        private static Action<object, TQeema> KatibMuhkam<THadaf, TQeema>(MethodInfo tariqa)
            where THadaf : class
        {
            var mubashir =
                (Action<THadaf, TQeema>)tariqa.CreateDelegate(typeof(Action<THadaf, TQeema>));
            return (hadaf, qeema) => mubashir((THadaf)hadaf, qeema);
        }

        private static Action<object> AmrMuhkam<THadaf>(MethodInfo tariqa)
            where THadaf : class
        {
            var mubashir = (Action<THadaf>)tariqa.CreateDelegate(typeof(Action<THadaf>));
            return hadaf => mubashir((THadaf)hadaf);
        }
    }
    /// <summary>
    /// ترميز — the conversion between the two offset spaces this file lives
    /// between: Unity's UTF-16 <see cref="string"/> index and the UTF-8 byte
    /// offset that <see cref="TaaribHarf.Anqud"/> and
    /// <see cref="TaaribSatr.BidayatMantiqi"/> are expressed in.
    /// </summary>
    /// <remarks>
    /// <para>
    /// These two numbers are equal for ASCII and for nothing else. An Arabic
    /// letter is one UTF-16 unit and two UTF-8 bytes; a diacritic is the same;
    /// an emoji is two units and four bytes. Handing a caret index straight to
    /// a cluster comparison therefore works perfectly in an English field and
    /// puts the caret roughly twice too far into an Arabic one — a failure
    /// that passes every test written in Latin.
    /// </para>
    /// <para>
    /// Both directions are a linear scan with three comparisons per unit and
    /// no allocation. That is affordable because an input field holds a name,
    /// a chat line or a search box, and because the scan runs when the caret
    /// moves rather than on every frame the field is open.
    /// </para>
    /// </remarks>
    public static class Tarmiz
    {
        /// <summary>
        /// The UTF-8 byte offset of a UTF-16 index.
        /// </summary>
        /// <param name="nass">The field's value, in logical order.</param>
        /// <param name="mawqi">
        /// The UTF-16 index, clamped into the text. An index that lands on the
        /// low half of a surrogate pair is treated as the pair's start, since
        /// no caret position exists inside one.
        /// </param>
        /// <returns>The byte offset, from zero to the encoded length.</returns>
        public static int Bayt(ReadOnlySpan<char> nass, int mawqi)
        {
            if (mawqi <= 0)
            {
                return 0;
            }
            if (mawqi > nass.Length)
            {
                mawqi = nass.Length;
            }
            if (mawqi < nass.Length && char.IsLowSurrogate(nass[mawqi]) && mawqi > 0
                && char.IsHighSurrogate(nass[mawqi - 1]))
            {
                mawqi--;
            }

            int bayt = 0;
            for (int i = 0; i < mawqi; i++)
            {
                char harf = nass[i];
                if (harf < 0x0080)
                {
                    bayt += 1;
                }
                else if (harf < 0x0800)
                {
                    bayt += 2;
                }
                else if (char.IsHighSurrogate(harf) && i + 1 < nass.Length
                    && char.IsLowSurrogate(nass[i + 1]))
                {
                    bayt += 4;
                    i++;
                }
                else
                {
                    // Includes a lone surrogate, which Takhtit encodes as
                    // U+FFFD rather than refusing the string — three bytes
                    // either way, so the offsets stay in step.
                    bayt += 3;
                }
            }
            return bayt;
        }

        /// <summary>
        /// The UTF-16 index of a UTF-8 byte offset — the inverse of
        /// <see cref="Bayt"/>, used to carry a cluster offset the native
        /// layout reported back into a position Unity understands.
        /// </summary>
        /// <param name="nass">The field's value, in logical order.</param>
        /// <param name="bayt">The byte offset. A value inside a UTF-8 sequence
        /// resolves to the start of that sequence rather than past it.</param>
        /// <returns>The UTF-16 index.</returns>
        public static int Harf(ReadOnlySpan<char> nass, int bayt)
        {
            if (bayt <= 0)
            {
                return 0;
            }

            int jari = 0;
            for (int i = 0; i < nass.Length; i++)
            {
                if (jari >= bayt)
                {
                    return i;
                }
                char harf = nass[i];
                if (harf < 0x0080)
                {
                    jari += 1;
                }
                else if (harf < 0x0800)
                {
                    jari += 2;
                }
                else if (char.IsHighSurrogate(harf) && i + 1 < nass.Length
                    && char.IsLowSurrogate(nass[i + 1]))
                {
                    jari += 4;
                    i++;
                }
                else
                {
                    jari += 3;
                }
            }
            return nass.Length;
        }

        /// <summary>The encoded length of the text in UTF-8 bytes.</summary>
        /// <param name="nass">The field's value.</param>
        /// <returns>The byte length.</returns>
        public static int Tul(ReadOnlySpan<char> nass)
        {
            return Bayt(nass, nass.Length);
        }
    }

    /// <summary>
    /// عناقيد — grapheme cluster boundaries over UTF-16, which is the unit a
    /// caret steps by, a selection snaps to, a delete removes, and a
    /// typewriter reveals.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The authority for this is <c>taqtee::hudud_anaqid</c> in
    /// <c>crates/taarib-saff/src/taqtee.rs</c>, which gets the answer from ICU
    /// and states the failure plainly: a caret that moves by bytes lands
    /// inside a UTF-8 sequence, and a caret that moves by <c>char</c> lands
    /// between a letter and its own tashkeel, deleting a fatha and leaving the
    /// letter it belonged to. The C ABI does not expose that function, so the
    /// UAX #29 extended grapheme cluster rules are applied here directly, over
    /// spans, with no allocation.
    /// </para>
    /// <para>
    /// What is covered, because these are the cases that produce wrong text in
    /// this product's languages: CR LF as one cluster; a base with any number
    /// of combining marks, which is the whole of tashkeel and the named
    /// failure of this phase; ZWNJ, which is <c>Extend</c> and is how Persian
    /// spells half its verbs; the Arabic <c>Prepend</c> characters U+0600 to
    /// U+0605, U+06DD, U+070F, U+0890, U+0891 and U+08E2, which bind forward
    /// to the digits that follow them; Hangul syllable composition; regional
    /// indicator pairs; and emoji joined by ZWJ.
    /// </para>
    /// <para>
    /// What is bounded: the <c>Extended_Pictographic</c> property is matched
    /// against the contiguous emoji blocks rather than enumerated in full, and
    /// combining marks outside the Basic Multilingual Plane are matched
    /// against the variation-selector and tag ranges rather than by category,
    /// because <see cref="CharUnicodeInfo"/> cannot classify a supplementary
    /// codepoint from a span in this framework. The worst outcome of both
    /// bounds is a rare emoji sequence that deletes in two presses instead of
    /// one. Neither bound can split a letter from its marks, which is the
    /// failure that matters here.
    /// </para>
    /// </remarks>
    public static class Anaqid
    {
        /// <summary>
        /// The next cluster boundary strictly after a UTF-16 index — where the
        /// caret goes when it moves one position forward through the string,
        /// and where a forward delete stops.
        /// </summary>
        /// <param name="nass">The text, in logical order.</param>
        /// <param name="mawqi">The index to move from.</param>
        /// <returns>
        /// The boundary, never less than <paramref name="mawqi"/> and never
        /// past the end. An index that sits inside a cluster moves to the end
        /// of that cluster, so a caret never stops between a letter and its own
        /// diacritic.
        /// </returns>
        public static int Tali(ReadOnlySpan<char> nass, int mawqi)
        {
            if (nass.IsEmpty || mawqi >= nass.Length)
            {
                return nass.Length;
            }
            int hali = 0;
            while (hali < nass.Length)
            {
                int tali = Khatwa(nass, hali);
                if (tali > mawqi)
                {
                    return tali;
                }
                hali = tali;
            }
            return nass.Length;
        }

        /// <summary>
        /// The cluster boundary strictly before a UTF-16 index — where the
        /// caret goes when it moves one position back, and where a backspace
        /// starts removing from.
        /// </summary>
        /// <param name="nass">The text, in logical order.</param>
        /// <param name="mawqi">The index to move from.</param>
        /// <returns>
        /// The boundary, never negative. An index inside a cluster snaps to
        /// the start of that cluster rather than stepping over it, so a caret
        /// recovers from a position some other code computed by units.
        /// </returns>
        public static int Sabiq(ReadOnlySpan<char> nass, int mawqi)
        {
            if (nass.IsEmpty || mawqi <= 0)
            {
                return 0;
            }
            int hadd = mawqi > nass.Length ? nass.Length : mawqi;
            int sabiq = 0;
            int hali = 0;
            while (hali < nass.Length)
            {
                int tali = Khatwa(nass, hali);
                if (tali >= hadd)
                {
                    return hali < hadd ? hali : sabiq;
                }
                sabiq = hali;
                hali = tali;
            }
            return sabiq;
        }

        /// <summary>
        /// The cluster boundary at or before a UTF-16 index — the snap a
        /// caret position from outside this file goes through before it is
        /// trusted as a logical position.
        /// </summary>
        /// <param name="nass">The text, in logical order.</param>
        /// <param name="mawqi">The index to snap.</param>
        /// <returns>The boundary at or before it.</returns>
        public static int Bidaya(ReadOnlySpan<char> nass, int mawqi)
        {
            if (nass.IsEmpty || mawqi <= 0)
            {
                return 0;
            }
            if (mawqi >= nass.Length)
            {
                return nass.Length;
            }
            int hali = 0;
            while (hali < nass.Length)
            {
                int tali = Khatwa(nass, hali);
                if (tali > mawqi)
                {
                    return hali;
                }
                if (tali == mawqi)
                {
                    return mawqi;
                }
                hali = tali;
            }
            return nass.Length;
        }

        /// <summary>
        /// How many grapheme clusters the text has — the number a typewriter
        /// effect counts up to, and the honest answer to "how many characters
        /// is this" for a script where a letter and its two marks are three
        /// UTF-16 units and one perceived character.
        /// </summary>
        /// <param name="nass">The text, in logical order.</param>
        /// <returns>The cluster count.</returns>
        public static int Adad(ReadOnlySpan<char> nass)
        {
            int adad = 0;
            int hali = 0;
            while (hali < nass.Length)
            {
                hali = Khatwa(nass, hali);
                adad++;
            }
            return adad;
        }

        /// <summary>
        /// The UTF-16 index that ends the first <paramref name="adad"/>
        /// clusters — the exact length of the prefix a typewriter effect must
        /// lay out to have revealed that many perceived characters.
        /// </summary>
        /// <param name="nass">The text, in logical order.</param>
        /// <param name="adad">
        /// How many clusters to include. Zero yields zero; more than the text
        /// has yields the whole text.
        /// </param>
        /// <returns>The prefix length in UTF-16 units.</returns>
        public static int Hadd(ReadOnlySpan<char> nass, int adad)
        {
            if (adad <= 0 || nass.IsEmpty)
            {
                return 0;
            }
            int hali = 0;
            int baqi = adad;
            while (hali < nass.Length && baqi > 0)
            {
                hali = Khatwa(nass, hali);
                baqi--;
            }
            return hali;
        }

        /// <summary>
        /// The end of the cluster that begins at <paramref name="bidaya"/> —
        /// the primitive every other member here is built from, exposed so
        /// that a caller enumerating clusters in order pays one walk over the
        /// text rather than one walk per cluster.
        /// </summary>
        /// <param name="nass">The text, in logical order.</param>
        /// <param name="bidaya">
        /// A cluster boundary. Zero always is one; every value this method or
        /// <see cref="Tali"/> returned is one. An index that is not a boundary
        /// still terminates and still advances, but the answer describes a
        /// cluster that does not exist in the text.
        /// </param>
        /// <returns>
        /// The next boundary, strictly greater than <paramref name="bidaya"/>
        /// unless the text is exhausted.
        /// </returns>
        public static int Nihaya(ReadOnlySpan<char> nass, int bidaya)
        {
            if (bidaya < 0)
            {
                return 0;
            }
            if (bidaya >= nass.Length)
            {
                return nass.Length;
            }
            return Khatwa(nass, bidaya);
        }

        /// <summary>
        /// The UAX #29 grapheme break classes this file distinguishes.
        /// </summary>
        private enum Fia
        {
            Aakhar,
            Marja,
            Satr,
            Tahakkum,
            Imtidad,
            Wasl,
            AlamaMasafa,
            Muqaddam,
            Iqlimi,
            HangulL,
            HangulV,
            HangulT,
            HangulLV,
            HangulLVT,
            Suwar,
        }

        /// <summary>
        /// The end of the cluster that starts at <paramref name="bidaya"/>,
        /// which must itself be a boundary. Every public member above walks
        /// forward with this, which is why they are all linear from the start
        /// of the text: rules GB11 and GB12 need left context, so a cluster
        /// cannot be measured backward from an arbitrary index without lying
        /// about emoji sequences and flag pairs.
        /// </summary>
        private static int Khatwa(ReadOnlySpan<char> nass, int bidaya)
        {
            int i = bidaya;
            int ramz = Ramz(nass, i, out int tul);
            Fia fia = Fasila(ramz);
            i += tul;

            // GB9b — Prepend binds forward, which is what makes U+0600 ARABIC
            // NUMBER SIGN one cluster with the digits it introduces.
            while (fia == Fia.Muqaddam && i < nass.Length)
            {
                Fia tali = Fasila(Ramz(nass, i, out int tulTali));
                if (tali == Fia.Marja || tali == Fia.Satr || tali == Fia.Tahakkum)
                {
                    break;
                }
                fia = tali;
                i += tulTali;
            }

            // GB3, GB4, GB5 — a CR LF pair is one cluster; anything else that
            // controls the line stands alone on both sides.
            if (fia == Fia.Marja)
            {
                if (i < nass.Length && Fasila(nass[i]) == Fia.Satr)
                {
                    i++;
                }
                return i;
            }
            if (fia == Fia.Satr || fia == Fia.Tahakkum)
            {
                return i;
            }

            // GB6, GB7, GB8 — Hangul syllable composition. The loop exits at
            // once for every other script.
            Fia halat = fia;
            while (i < nass.Length)
            {
                Fia tali = Fasila(Ramz(nass, i, out int tulTali));
                bool yasil =
                    (halat == Fia.HangulL
                        && (tali == Fia.HangulL || tali == Fia.HangulV
                            || tali == Fia.HangulLV || tali == Fia.HangulLVT))
                    || ((halat == Fia.HangulLV || halat == Fia.HangulV)
                        && (tali == Fia.HangulV || tali == Fia.HangulT))
                    || ((halat == Fia.HangulLVT || halat == Fia.HangulT)
                        && tali == Fia.HangulT);
                if (!yasil)
                {
                    break;
                }
                halat = tali;
                i += tulTali;
            }

            // GB12, GB13 — regional indicators pair up, so a flag is one
            // cluster and two flags are two, not one long ribbon.
            if (fia == Fia.Iqlimi && i < nass.Length
                && Fasila(Ramz(nass, i, out int tulIqlimi)) == Fia.Iqlimi)
            {
                i += tulIqlimi;
            }

            // GB9, GB9a, GB11 — the tail: every mark, every ZWJ, and the
            // pictograph a ZWJ joins to. This is the loop that keeps a letter
            // and its shadda and its fatha together as one deletable thing.
            bool suwar = fia == Fia.Suwar;
            while (i < nass.Length)
            {
                Fia tali = Fasila(Ramz(nass, i, out int tulTali));
                if (tali == Fia.Imtidad || tali == Fia.AlamaMasafa)
                {
                    i += tulTali;
                    continue;
                }
                if (tali != Fia.Wasl)
                {
                    break;
                }
                int baad = i + tulTali;
                if (suwar && baad < nass.Length
                    && Fasila(Ramz(nass, baad, out int tulThalith)) == Fia.Suwar)
                {
                    i = baad + tulThalith;
                    continue;
                }
                i = baad;
            }
            return i;
        }

        /// <summary>
        /// The codepoint at a UTF-16 index, and how many units it occupies. An
        /// unpaired surrogate is returned as itself and classified as a
        /// control, which is what keeps a malformed game string from making
        /// the walk loop forever.
        /// </summary>
        private static int Ramz(ReadOnlySpan<char> nass, int mawqi, out int tul)
        {
            char harf = nass[mawqi];
            if (char.IsHighSurrogate(harf) && mawqi + 1 < nass.Length
                && char.IsLowSurrogate(nass[mawqi + 1]))
            {
                tul = 2;
                return 0x10000 + ((harf - 0xD800) << 10) + (nass[mawqi + 1] - 0xDC00);
            }
            tul = 1;
            return harf;
        }

        /// <summary>The grapheme break class of one codepoint.</summary>
        private static Fia Fasila(int ramz)
        {
            switch (ramz)
            {
                case 0x000D: return Fia.Marja;
                case 0x000A: return Fia.Satr;
                case 0x200D: return Fia.Wasl;
                // ZWNJ is Extend, not a control. Persian spells half its verb
                // forms with it; a delete that treated it as its own cluster
                // would take two presses to remove one perceived letter.
                case 0x200C: return Fia.Imtidad;
                default: break;
            }
            if (ramz >= 0xFE00 && ramz <= 0xFE0F)
            {
                return Fia.Imtidad;
            }
            if (ramz >= 0xE0020 && ramz <= 0xE01EF)
            {
                return Fia.Imtidad;
            }
            if (Yuqaddam(ramz))
            {
                return Fia.Muqaddam;
            }
            if (ramz >= 0x1F1E6 && ramz <= 0x1F1FF)
            {
                return Fia.Iqlimi;
            }
            if (ramz >= 0x1100 && ramz <= 0x115F)
            {
                return Fia.HangulL;
            }
            if ((ramz >= 0x1160 && ramz <= 0x11A7) || (ramz >= 0xD7B0 && ramz <= 0xD7C6))
            {
                return Fia.HangulV;
            }
            if ((ramz >= 0x11A8 && ramz <= 0x11FF) || (ramz >= 0xD7CB && ramz <= 0xD7FB))
            {
                return Fia.HangulT;
            }
            if (ramz >= 0xA960 && ramz <= 0xA97C)
            {
                return Fia.HangulL;
            }
            if (ramz >= 0xAC00 && ramz <= 0xD7A3)
            {
                return (ramz - 0xAC00) % 28 == 0 ? Fia.HangulLV : Fia.HangulLVT;
            }
            if (Yusawwar(ramz))
            {
                return Fia.Suwar;
            }
            if (ramz > 0xFFFF)
            {
                return Fia.Aakhar;
            }
            switch (CharUnicodeInfo.GetUnicodeCategory((char)ramz))
            {
                case UnicodeCategory.NonSpacingMark:
                case UnicodeCategory.EnclosingMark:
                    return Fia.Imtidad;
                case UnicodeCategory.SpacingCombiningMark:
                    return Fia.AlamaMasafa;
                case UnicodeCategory.Control:
                case UnicodeCategory.Format:
                case UnicodeCategory.LineSeparator:
                case UnicodeCategory.ParagraphSeparator:
                case UnicodeCategory.Surrogate:
                    return Fia.Tahakkum;
                default:
                    return Fia.Aakhar;
            }
        }

        /// <summary>
        /// The <c>Prepend</c> characters. Six of the nine are Arabic, which is
        /// why this class is enumerated here rather than left to fall through
        /// to <c>Other</c>: U+0600 to U+0605 and U+06DD introduce the digits
        /// that follow them and belong to the same perceived character.
        /// </summary>
        private static bool Yuqaddam(int ramz)
        {
            return (ramz >= 0x0600 && ramz <= 0x0605)
                || ramz == 0x06DD
                || ramz == 0x070F
                || ramz == 0x0890
                || ramz == 0x0891
                || ramz == 0x08E2
                || ramz == 0x110BD
                || ramz == 0x110CD;
        }

        /// <summary>
        /// <c>Extended_Pictographic</c>, matched against the contiguous emoji
        /// blocks. The property's scattered tail below U+2000 is not
        /// enumerated; the cost of that is a legacy dingbat that deletes in
        /// two presses instead of one, and the benefit is a table small enough
        /// to read.
        /// </summary>
        private static bool Yusawwar(int ramz)
        {
            if (ramz >= 0x1F000 && ramz <= 0x1FAFF)
            {
                return true;
            }
            if (ramz >= 0x1FC00 && ramz <= 0x1FFFD)
            {
                return true;
            }
            if (ramz >= 0x2600 && ramz <= 0x27BF)
            {
                return true;
            }
            if (ramz >= 0x2B00 && ramz <= 0x2BFF)
            {
                return true;
            }
            if (ramz >= 0x2194 && ramz <= 0x21AA)
            {
                return true;
            }
            if (ramz >= 0x23E9 && ramz <= 0x23FA)
            {
                return true;
            }
            if (ramz >= 0x25AA && ramz <= 0x25FE)
            {
                return true;
            }
            return ramz == 0x00A9 || ramz == 0x00AE || ramz == 0x203C || ramz == 0x2049
                || ramz == 0x2122 || ramz == 0x2139 || ramz == 0x231A || ramz == 0x231B
                || ramz == 0x2328 || ramz == 0x23CF || ramz == 0x24C2 || ramz == 0x3030
                || ramz == 0x303D || ramz == 0x3297 || ramz == 0x3299;
        }
    }
    /// <summary>
    /// موضع المؤشر — where a caret is drawn, in the layout's own pixel space:
    /// x from the layout's left edge, y down from its top.
    /// </summary>
    /// <remarks>
    /// <see cref="Asas"/> is the value <c>TakhtitNass::mawqi_anqud</c> itself
    /// returns; <see cref="A"/> and <see cref="Irtifa"/> are derived from the
    /// line by the same two expressions <c>mustatilat_tahdid</c> uses for a
    /// selection rectangle, so a caret and the selection it sits inside always
    /// have the same top edge and the same height.
    /// </remarks>
    public readonly struct MawqiMuashir
    {
        /// <summary>Builds a caret position.</summary>
        /// <param name="s">The horizontal position of the caret.</param>
        /// <param name="a">The top edge of the caret box.</param>
        /// <param name="asas">The baseline the caret sits on.</param>
        /// <param name="irtifa">The height of the caret box.</param>
        /// <param name="yameen">Whether the line runs right to left.</param>
        public MawqiMuashir(float s, float a, float asas, float irtifa, bool yameen)
        {
            S = s;
            A = a;
            Asas = asas;
            Irtifa = irtifa;
            Yameen = yameen;
            Wujid = true;
        }

        /// <summary>
        /// Horizontal position, in pixels from the layout's left edge. For a
        /// right-to-left line this is the left edge of the glyph the caret
        /// follows in reading order, which is visually to its left.
        /// </summary>
        public float S { get; }

        /// <summary>Top edge of the caret box.</summary>
        public float A { get; }

        /// <summary>The line's baseline — the raw vertical answer.</summary>
        public float Asas { get; }

        /// <summary>Height of the caret box, which is the line box's height.</summary>
        public float Irtifa { get; }

        /// <summary>
        /// Whether the line the caret landed on runs right to left. A caret
        /// renderer needs this to decide which way a wide caret grows and
        /// which way an insertion preview slides.
        /// </summary>
        public bool Yameen { get; }

        /// <summary>
        /// Whether a position was found at all. False only for a layout with
        /// no glyphs — an empty field — where the caller places the caret at
        /// the box's leading edge itself, because there is no line to ask.
        /// </summary>
        public bool Wujid { get; }
    }

    /// <summary>
    /// مستطيل تحديد — one rectangle of a selection, in the layout's pixel
    /// space.
    /// </summary>
    /// <remarks>
    /// A selection is a list of these, not one of them. The doc comment on
    /// <c>TakhtitNass::mustatilat_tahdid</c> says why: a selection that
    /// crosses a direction boundary is genuinely two rectangles, because the
    /// two selected runs are not adjacent on screen. Painting one rectangle
    /// from the first glyph to the last covers text the user did not select,
    /// and it is the bug every text field that has never been used in Arabic
    /// ships with.
    /// </remarks>
    public readonly struct MustatilTahdid
    {
        /// <summary>Builds a rectangle.</summary>
        /// <param name="s">Left edge.</param>
        /// <param name="a">Top edge.</param>
        /// <param name="ard">Width.</param>
        /// <param name="irtifa">Height.</param>
        public MustatilTahdid(float s, float a, float ard, float irtifa)
        {
            S = s;
            A = a;
            Ard = ard;
            Irtifa = irtifa;
        }

        /// <summary>Left edge, in pixels from the layout's left edge.</summary>
        public float S { get; }

        /// <summary>Top edge, in pixels down from the layout's top.</summary>
        public float A { get; }

        /// <summary>Width.</summary>
        public float Ard { get; }

        /// <summary>Height, which is the line box's height.</summary>
        public float Irtifa { get; }
    }

    /// <summary>
    /// قياس الحقل — the layout parameters a field is measured and drawn with.
    /// </summary>
    /// <remarks>
    /// These are part of the layout cache key on the native side, so a field
    /// whose box or size changes between frames re-shapes; a field that only
    /// moves its caret does not. Keeping this struct stable while the user
    /// types is therefore not tidiness, it is the difference between one
    /// shaping call per keystroke and one per frame.
    /// </remarks>
    public struct QiyasHaql
    {
        /// <summary>The pixel size the field draws at.</summary>
        public float Hajm;

        /// <summary>
        /// The width available in pixels. Zero or less means one line of
        /// whatever width the text needs, which is what a single-line field
        /// that scrolls horizontally wants.
        /// </summary>
        public float ArdMutah;

        /// <summary>The height available in pixels; zero or less is unbounded.</summary>
        public float IrtifaMutah;

        /// <summary>
        /// The rest of the decisions — direction, language, alignment,
        /// diacritics and digits policy. A single-line field sets
        /// <see cref="Alamat.KhiyarSatrWahid"/> here.
        /// </summary>
        public KhiyaratTakhtit Khiyarat;

        /// <summary>
        /// Whether two parameter sets would produce the same layout, which is
        /// how a field decides it may reuse the glyphs already in its buffer
        /// instead of crossing the ABI again.
        /// </summary>
        /// <param name="aakhar">The parameters to compare against.</param>
        /// <returns>Whether every field matches exactly.</returns>
        public bool Yutabiq(in QiyasHaql aakhar)
        {
            return Hajm == aakhar.Hajm
                && ArdMutah == aakhar.ArdMutah
                && IrtifaMutah == aakhar.IrtifaMutah
                && Khiyarat.Ittijah == aakhar.Khiyarat.Ittijah
                && Khiyarat.Lugha == aakhar.Khiyarat.Lugha
                && Khiyarat.Dabt == aakhar.Khiyarat.Dabt
                && Khiyarat.Muhadhaha == aakhar.Khiyarat.Muhadhaha
                && Khiyarat.Tashkeel == aakhar.Khiyarat.Tashkeel
                && Khiyarat.Arqam == aakhar.Khiyarat.Arqam
                && Khiyarat.Tajawuz == aakhar.Khiyarat.Tajawuz
                && Khiyarat.HajmAdna == aakhar.Khiyarat.HajmAdna
                && Khiyarat.IrtifaSatr == aakhar.Khiyarat.IrtifaSatr
                && Khiyarat.TabaudAhruf == aakhar.Khiyarat.TabaudAhruf
                && Khiyarat.TabaudKalimat == aakhar.Khiyarat.TabaudKalimat
                && Khiyarat.Alam == aakhar.Khiyarat.Alam;
        }
    }
    /// <summary>
    /// ربط الحقل — the members of one input-field family, resolved once at
    /// startup and held as delegates.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Two families exist and they disagree about what a caret index means.
    /// <c>UnityEngine.UI.InputField</c> counts positions in the string.
    /// <c>TMPro.TMP_InputField</c> has two coordinate systems:
    /// <c>caretPosition</c> counts entries in <c>TMP_TextInfo.characterInfo</c>
    /// — which excludes rich-text tags and anything the layout dropped — while
    /// <c>stringPosition</c> counts UTF-16 units in the string. Taarib's
    /// logical positions are string positions, so <c>stringPosition</c> and
    /// <c>stringSelectPosition</c> are bound in preference and
    /// <c>caretPosition</c> is the fallback for builds too old to have them.
    /// Reading <c>caretPosition</c> as if it were a string index is off by the
    /// number of invisible characters before the caret, which is zero in a
    /// plain field and not zero the moment anything is stripped.
    /// </para>
    /// <para>
    /// Nothing here throws when a member is missing. A field family that
    /// cannot supply a caret index degrades to no caret mapping for that
    /// family alone; the other family, and the other three behaviours in this
    /// namespace, never learn about it.
    /// </para>
    /// </remarks>
    public sealed class RabtHaql
    {
        private RabtHaql(Type naw, string ism)
        {
            Naw = naw;
            Ism = ism;
            QariNass = Rabt.Qari<string>(naw, "text");
            KatibNass = Rabt.Katib<string>(naw, "text");
            QariTarkiz = Rabt.Qari<bool>(naw, "isFocused");
            QariArdMuashir = Rabt.Qari<int>(naw, "caretWidth");
            QariMukawwin = Rabt.Qari<object>(naw, "textComponent");

            Func<object, int>? qariMuashir = Rabt.Qari<int>(naw, "stringPosition");
            Action<object, int>? katibMuashir = Rabt.Katib<int>(naw, "stringPosition");
            Func<object, int>? qariMurtakaz = Rabt.Qari<int>(naw, "stringSelectPosition");
            Action<object, int>? katibMurtakaz = Rabt.Katib<int>(naw, "stringSelectPosition");

            BiMawqiNass = qariMuashir is not null && qariMurtakaz is not null;
            if (!BiMawqiNass)
            {
                qariMuashir = Rabt.Qari<int>(naw, "caretPosition");
                katibMuashir = Rabt.Katib<int>(naw, "caretPosition");
                qariMurtakaz = Rabt.Qari<int>(naw, "selectionAnchorPosition");
                katibMurtakaz = Rabt.Katib<int>(naw, "selectionAnchorPosition");
            }

            QariMuashir = qariMuashir;
            KatibMuashir = katibMuashir;
            QariMurtakaz = qariMurtakaz;
            KatibMurtakaz = katibMurtakaz;
        }

        /// <summary>The game's own input-field type.</summary>
        public Type Naw { get; }

        /// <summary>
        /// The family's short name, for the one log line a degradation writes.
        /// </summary>
        public string Ism { get; }

        /// <summary>Reads the field's value. Never <c>null</c> on a usable binding.</summary>
        public Func<object, string>? QariNass { get; }

        /// <summary>Writes the field's value, which is how an edit lands.</summary>
        public Action<object, string>? KatibNass { get; }

        /// <summary>Reads the caret's logical position.</summary>
        public Func<object, int>? QariMuashir { get; }

        /// <summary>Writes the caret's logical position.</summary>
        public Action<object, int>? KatibMuashir { get; }

        /// <summary>Reads the selection anchor's logical position.</summary>
        public Func<object, int>? QariMurtakaz { get; }

        /// <summary>Writes the selection anchor's logical position.</summary>
        public Action<object, int>? KatibMurtakaz { get; }

        /// <summary>Whether the field currently has keyboard focus.</summary>
        public Func<object, bool>? QariTarkiz { get; }

        /// <summary>The caret's width in pixels, as the game configured it.</summary>
        public Func<object, int>? QariArdMuashir { get; }

        /// <summary>
        /// The text component the field draws through — the object whose
        /// rectangle gives the available width, and whose takeover produced
        /// the layout this file reads.
        /// </summary>
        public Func<object, object>? QariMukawwin { get; }

        /// <summary>
        /// Whether the bound caret members count UTF-16 units in the string
        /// rather than entries in a text-info array. False means the caret
        /// mapping is trusting <c>caretPosition</c> as a string index, which
        /// is exact for a plain field and drifts once anything is stripped —
        /// worth a line in the log when a game's field misbehaves.
        /// </summary>
        public bool BiMawqiNass { get; }

        /// <summary>
        /// Whether enough resolved to map a caret at all: the value, the
        /// caret position and the anchor position.
        /// </summary>
        public bool Muakkad =>
            QariNass is not null && QariMuashir is not null && QariMurtakaz is not null;

        /// <summary>
        /// Binds <c>TMPro.TMP_InputField</c>, or reports why it could not be
        /// bound and returns <c>null</c>.
        /// </summary>
        /// <returns>The binding, or <c>null</c> when this game has no TextMeshPro.</returns>
        public static RabtHaql? Tmp()
        {
            return Iqran("TMPro.TMP_InputField", "TMP_InputField");
        }

        /// <summary>
        /// Binds <c>UnityEngine.UI.InputField</c>, or reports why it could not
        /// be bound and returns <c>null</c>.
        /// </summary>
        /// <returns>The binding, or <c>null</c> when this game has no uGUI input field.</returns>
        public static RabtHaql? Waajiha()
        {
            return Iqran("UnityEngine.UI.InputField", "InputField");
        }

        /// <summary>
        /// Whether a live object is an instance of this family, which is the
        /// test a takeover runs before handing an object to
        /// <see cref="HaqlIdkhal"/>.
        /// </summary>
        /// <param name="haql">The candidate object.</param>
        /// <returns>Whether it is one of these.</returns>
        public bool Yantami(object? haql)
        {
            return haql is not null && Naw.IsInstanceOfType(haql);
        }

        private static RabtHaql? Iqran(string ismKamil, string ismQasir)
        {
            Type? naw = Rabt.Naw(ismKamil);
            if (naw is null)
            {
                return null;
            }
            RabtHaql rabt = new RabtHaql(naw, ismQasir);
            if (!rabt.Muakkad)
            {
                Rabt.Ballagh(
                    ismQasir + " is present but its caret members did not resolve; "
                    + "caret and selection mapping is off for this field type only, "
                    + "and the rest of the takeover is unaffected.");
                return null;
            }
            if (!rabt.BiMawqiNass)
            {
                Rabt.Ballagh(
                    ismQasir + " has no stringPosition in this build; caret indices are "
                    + "read from caretPosition, which is exact for plain text and drifts "
                    + "by the number of characters the layout dropped when it is not.");
            }
            return rabt;
        }
    }
    /// <summary>
    /// حقل الإدخال — one bound input field: its layout, its caret, its
    /// selection rectangles, and the two edits.
    /// </summary>
    /// <remarks>
    /// <para>
    /// This object owns its <see cref="Takhtit"/> and must not share one with
    /// a renderer. A <see cref="Takhtit"/> exposes its results as spans over
    /// one reused pair of arrays, so a caret query on a shared instance would
    /// overwrite the glyphs a mesh writer was halfway through reading — a
    /// corruption that appears as a flicker of the wrong text, on a slow
    /// frame, in one menu.
    /// </para>
    /// <para>
    /// The per-frame path is: read the value, compare it and the parameters
    /// against the last layout, and if nothing changed do nothing at all. When
    /// something did change the layout crosses the ABI once; because the
    /// native layout cache keys on the text, the size and the available width,
    /// a caret walking through a string the user is not editing is served from
    /// the cache without shaping. Neither path allocates: the value is read
    /// as an existing reference, the text crosses as a span, and every result
    /// is a span over buffers allocated at construction.
    /// </para>
    /// </remarks>
    public sealed class HaqlIdkhal
    {
        private readonly MaqbadSiyaq siyaq;
        private readonly MaqbadSilsila silsila;
        private readonly Takhtit takhtit;
        private readonly RabtHaql rabt;
        private readonly object haql;

        private MustatilTahdid[] mustatilat;
        private int adadMustatilat;
        private string nass;
        private QiyasHaql qiyas;
        private bool mukhattat;
        private bool amil;

        /// <summary>
        /// Binds one live input-field object.
        /// </summary>
        /// <param name="siyaq">The engine context.</param>
        /// <param name="silsila">The font chain the field draws with.</param>
        /// <param name="takhtit">
        /// A layout buffer used by this field and nothing else.
        /// </param>
        /// <param name="rabt">The family binding, from <see cref="RabtHaql"/>.</param>
        /// <param name="haql">The field object itself.</param>
        /// <exception cref="ArgumentNullException">An argument is null.</exception>
        /// <exception cref="ArgumentException">
        /// <paramref name="haql"/> is not an instance of the bound family, or
        /// the binding never resolved its caret members.
        /// </exception>
        public HaqlIdkhal(
            MaqbadSiyaq siyaq,
            MaqbadSilsila silsila,
            Takhtit takhtit,
            RabtHaql rabt,
            object haql)
        {
            this.siyaq = siyaq ?? throw new ArgumentNullException(nameof(siyaq));
            this.silsila = silsila ?? throw new ArgumentNullException(nameof(silsila));
            this.takhtit = takhtit ?? throw new ArgumentNullException(nameof(takhtit));
            this.rabt = rabt ?? throw new ArgumentNullException(nameof(rabt));
            this.haql = haql ?? throw new ArgumentNullException(nameof(haql));
            if (!rabt.Muakkad)
            {
                throw new ArgumentException(
                    "The field binding never resolved its caret members.", nameof(rabt));
            }
            if (!rabt.Yantami(haql))
            {
                throw new ArgumentException(
                    "The object is not an instance of the bound field type.", nameof(haql));
            }
            mustatilat = new MustatilTahdid[8];
            nass = string.Empty;
            amil = true;
        }

        /// <summary>
        /// Whether this field is still taken over. A field switches itself off
        /// after a failure it cannot attribute to the text, and from then on
        /// every query answers emptily and the game's own caret is left to
        /// draw itself.
        /// </summary>
        public bool Amil => amil;

        /// <summary>The family this field belongs to.</summary>
        public RabtHaql Aila => rabt;

        /// <summary>The text the current layout was built from.</summary>
        public string Nass => nass;

        /// <summary>
        /// Whether a layout is present for the current text — false before the
        /// first successful <see cref="Jaddid(in QiyasHaql)"/> and after a
        /// failure.
        /// </summary>
        public bool Mukhattat => mukhattat;

        /// <summary>
        /// The selection rectangles for the laid-out value, as a view over this
        /// field's own buffer. Valid until the next call on this object.
        /// </summary>
        /// <remarks>
        /// Empty in this build. <see cref="Jaddid(string, in QiyasHaql, ReadOnlySpan{TaaribNitaqUslub})"/>
        /// resets the count and no code path fills the buffer, so a game draws
        /// its own selection highlight over Taarib's glyphs. The routine that
        /// would map a caret range onto cluster rectangles has not been written.
        /// </remarks>
        public ReadOnlySpan<MustatilTahdid> Mustatilat =>
            new ReadOnlySpan<MustatilTahdid>(mustatilat, 0, adadMustatilat);

        /// <summary>
        /// Reads the field's current value without allocating — the getter
        /// returns the string the field already holds.
        /// </summary>
        /// <returns>The value, or an empty string when it could not be read.</returns>
        public string NassHali()
        {
            Func<object, string>? qari = rabt.QariNass;
            if (!amil || qari is null)
            {
                return string.Empty;
            }
            try
            {
                return qari(haql) ?? string.Empty;
            }
            catch (Exception khata)
            {
                Awqif("reading the field's value threw", khata);
                return string.Empty;
            }
        }

        /// <summary>
        /// The caret's logical position as the field holds it, snapped to a
        /// grapheme cluster boundary. Snapping is not defensive tidying: a
        /// game's own script can set <c>caretPosition</c> to a number it
        /// computed by counting characters, and an index that lands between a
        /// letter and its fatha would make every answer below describe a
        /// position that cannot be drawn.
        /// </summary>
        /// <returns>The UTF-16 index of the caret, clamped into the value.</returns>
        public int MawqiMuashirHali()
        {
            return MawqiMuqayyad(rabt.QariMuashir);
        }

        /// <summary>
        /// The selection anchor's logical position, snapped the same way. The
        /// selection is the range between this and
        /// <see cref="MawqiMuashirHali"/>, in either order.
        /// </summary>
        /// <returns>The UTF-16 index of the anchor.</returns>
        public int MawqiMurtakazHali()
        {
            return MawqiMuqayyad(rabt.QariMurtakaz);
        }

        /// <summary>Whether the field currently has keyboard focus.</summary>
        /// <returns>
        /// Whether it is focused; <c>false</c> when the game's build has no
        /// such property, which costs a caret drawn on an unfocused field and
        /// nothing else.
        /// </returns>
        public bool Murakkaz()
        {
            Func<object, bool>? qari = rabt.QariTarkiz;
            if (!amil || qari is null)
            {
                return false;
            }
            try
            {
                return qari(haql);
            }
            catch (Exception khata)
            {
                Awqif("reading the field's focus state threw", khata);
                return false;
            }
        }

        /// <summary>
        /// Lays the field's current value out if anything changed since the
        /// last call, and does nothing at all if nothing did.
        /// </summary>
        /// <param name="qiyasJadid">The layout parameters.</param>
        /// <returns>Whether a usable layout is present afterwards.</returns>
        public bool Jaddid(in QiyasHaql qiyasJadid)
        {
            return Jaddid(NassHali(), qiyasJadid, ReadOnlySpan<TaaribNitaqUslub>.Empty);
        }

        /// <summary>
        /// Lays a given text out with given style spans — the entry point for
        /// a field whose value carries markup, where <c>nasq</c> has already
        /// lifted the tags out and the clean text is what cluster offsets are
        /// expressed in.
        /// </summary>
        /// <param name="nassJadid">The clean text, in logical order.</param>
        /// <param name="qiyasJadid">The layout parameters.</param>
        /// <param name="nitaqat">
        /// The style spans, with byte offsets into the UTF-8 encoding of
        /// <paramref name="nassJadid"/>. Empty means one unstyled run.
        /// </param>
        /// <returns>Whether a usable layout is present afterwards.</returns>
        public bool Jaddid(
            string nassJadid,
            in QiyasHaql qiyasJadid,
            ReadOnlySpan<TaaribNitaqUslub> nitaqat)
        {
            if (!amil)
            {
                return false;
            }
            nassJadid ??= string.Empty;

            bool thabit = mukhattat
                && qiyas.Yutabiq(qiyasJadid)
                && (ReferenceEquals(nass, nassJadid) || string.Equals(nass, nassJadid, StringComparison.Ordinal));
            if (thabit && nitaqat.IsEmpty)
            {
                return true;
            }

            try
            {
                TalabTakhtit talab = default;
                talab.Nass = nassJadid.AsSpan();
                talab.Nitaqat = nitaqat;
                talab.Sifat = ReadOnlySpan<TaaribSifa>.Empty;
                talab.Hajm = qiyasJadid.Hajm;
                talab.ArdMutah = qiyasJadid.ArdMutah;
                talab.IrtifaMutah = qiyasJadid.IrtifaMutah;
                talab.Khiyarat = qiyasJadid.Khiyarat;
                takhtit.Khattit(siyaq, silsila, talab);

                nass = nassJadid;
                qiyas = qiyasJadid;
                adadMustatilat = 0;
                mukhattat = true;
                return true;
            }
            catch (KhataTaarib khata)
            {
                // A layout failure is attributable to this one field's text
                // and parameters, so it is not a reason to switch the field
                // off permanently — the next keystroke may well succeed.
                mukhattat = false;
                Suluk.Rabt.Ballagh(
                    rabt.Ism + ": laying out the field's value failed", khata);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("laying out the field's value threw", khata);
                return false;
            }
        }

        /// <summary>
        /// Switches this field off, naming the reason once. Everything after
        /// this answers emptily, which leaves the game's own caret and
        /// selection drawing exactly as they were before the takeover.
        /// </summary>
        /// <param name="sabab">What failed, in the log's own voice.</param>
        /// <param name="khata">The exception behind it.</param>
        public void Awqif(string sabab, Exception khata)
        {
            if (!amil)
            {
                return;
            }
            amil = false;
            mukhattat = false;
            adadMustatilat = 0;
            Suluk.Rabt.Ballagh(rabt.Ism + ": " + sabab + "; this field only", khata);
        }

        private int MawqiMuqayyad(Func<object, int>? qari)
        {
            if (!amil || qari is null)
            {
                return 0;
            }
            try
            {
                string hali = NassHali();
                int mawqi = qari(haql);
                if (mawqi <= 0)
                {
                    return 0;
                }
                if (mawqi >= hali.Length)
                {
                    return hali.Length;
                }
                return Anaqid.Bidaya(hali.AsSpan(), mawqi);
            }
            catch (Exception khata)
            {
                Awqif("reading a caret index threw", khata);
                return 0;
            }
        }
    }
}
