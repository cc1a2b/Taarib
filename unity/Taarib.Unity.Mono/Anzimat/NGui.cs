// نظام NGUI — the takeover for UILabel.
//
// NGUI is the oldest text system Taarib supports and the one most likely to be
// found in a shipped title nobody is maintaining any more. It predates uGUI, it
// was the default UI toolkit for Unity games from roughly 2012 onward, and
// enough studios standardised on it that it is still shipping in live products
// today. Every one of those builds carries a different NGUI revision, none of
// them is on NuGet, and no two of them agree about the exact spelling of the
// members below. That is why this file names no NGUI type at compile time and
// resolves everything it touches once, at startup, through Suluk.Rabt.
//
// WHERE THE TAKEOVER GOES IN. NGUI's rendering is a two-stage affair. A UIPanel
// collects the widgets that share a material, and for each one calls
// UIWidget.WriteToBuffers, which calls the virtual UIWidget.OnFill(verts, uvs,
// cols) — and OnFill is where a widget decides what it draws. UILabel's
// override is the point at which NGUI turns a string into quads, so a prefix on
// that override is the exact interception point: refuse the original, write our
// own quads into the same three lists, and the panel batches them without
// knowing anything happened.
//
// The lists are APPENDED to, not filled from zero. One draw call holds every
// widget that shares a material, so a label's quads begin at whatever `size`
// the lists already carry, and a fill that wrote from index zero would draw
// this label's text over the previous widget's geometry.
//
// NGUI ALSO WANTS NO TRIANGLES. A UIDrawCall generates its own index buffer on
// the assumption that every four vertices are one quad, in the order
// bottom-left, top-left, top-right, bottom-right. That is the same order Nasij
// emits — it is Unity's own quad order, and Nasij documents why it follows it —
// so the geometry lands correctly with the index buffer NGUI builds for itself,
// and the triangle indices Nasij writes go into the shared scratch array and
// are discarded. Handing Nasij an empty triangle span instead would make it
// report a capacity shortfall and draw nothing at all.
//
// WHY BetterList IS REACHED FIELD BY FIELD. BetterList<T> is NGUI's own list
// type and is not System.Collections.Generic.List<T>. It exposes a public
// `buffer` array and a public `size` count, and it exists precisely because its
// author did not want an indexer between the caller and the array. Going
// through a reflected indexer here would be one reflection call per vertex, per
// label, per rebuild — four calls per glyph, times a screen of dialogue — which
// is the shape of dispatch and allocation cost this whole assembly is arranged
// to avoid. So `buffer` is read once per fill as an array reference, the vertex
// span is reinterpreted over it with MemoryMarshal.Cast, and Nasij writes the
// whole label in one pass into memory the component already owns. The two field
// accessors are compiled once at startup by System.Linq.Expressions; a
// FieldInfo.GetValue per fill would box `size` on every call, which is a
// measurable amount of garbage in someone else's frame budget for a number that
// fits in a register.
//
// NGUI 3.11 and later replaced some of these BetterLists with List<T>. That
// costs nothing here: List<T> has exactly the same two fields under the names
// `_items` and `_size`, so the same pair of compiled accessors serves both and
// the only difference is which names were resolved.
//
// THE FONT IS NEVER READ. A UILabel draws through a UIFont, which is either a
// bitmap font — glyphs baked into a UIAtlas texture at one size, with NGUI's
// own kerning table beside them — or a dynamic font, which is a
// UnityEngine.Font that Unity rasterizes on demand into a font texture through
// Font.RequestCharactersInTexture. Neither is read here, for glyphs or for
// metrics or for anything else, and that is Decision 5 rather than an oversight.
// It is also not a loss. A bitmap font built for a game that was never
// Arabized contains no Arabic at all, so there is nothing in it to draw with;
// and the dynamic path performs no shaping whatsoever, so asking it for Arabic
// returns the isolated form of every letter, unjoined, which is the exact
// failure this product exists to prevent. What a UIFont is therefore affects
// exactly one thing in this file: which material and which texture the draw
// call must bind, and both of those are answered by substitution rather than by
// inspection.

using System;
using System.Collections.Generic;
using System.Linq.Expressions;
using System.Reflection;
using System.Runtime.InteropServices;
using HarmonyLib;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mono.Suluk;
using Taarib.Unity.Mushtarak;
using UnityEngine;

namespace Taarib.Unity.Mono.Anzimat
{
    /// <summary>
    /// وصل NGUI — every NGUI member this takeover touches, resolved once at
    /// startup and held as a strongly typed delegate.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Nothing in this class runs on a frame. That is its entire reason for
    /// existing: a <see cref="PropertyInfo.GetValue(object)"/> allocates an
    /// argument array and boxes its result, and a
    /// <see cref="FieldInfo.GetValue(object)"/> boxes an <see cref="int"/>
    /// count — doing either once per label per geometry rebuild is garbage a
    /// game's collector pays for in a hitch nobody attributes to a translation
    /// plugin. A delegate built once costs a call and a cast.
    /// </para>
    /// <para>
    /// A member that does not resolve leaves its delegate <c>null</c> and
    /// <see cref="Muakkad"/> false, and <see cref="NizamNGui"/> then refuses to
    /// install with one named sentence in the log. It does not throw and it
    /// does not partially patch: a takeover that owns the fill but cannot read
    /// the label's size draws every string at the wrong size, which is worse
    /// than leaving NGUI alone.
    /// </para>
    /// <para>
    /// The field accessors are compiled expression trees rather than
    /// <see cref="FieldInfo"/> calls. This assembly ships only to the Mono
    /// backend, where the runtime has a JIT and
    /// <see cref="Expression{TDelegate}.Compile()"/> produces real code; the
    /// IL2CPP adapter, which cannot rely on that, is a different assembly and
    /// does not use this class.
    /// </para>
    /// </remarks>
    public sealed class WaslNGui
    {
        private const BindingFlags Alamat =
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;

        private WaslNGui(Type nawLawha, MethodInfo hadafMala)
        {
            NawLawha = nawLawha;
            HadafMala = hadafMala;

            QariNass = Rabt.Qari<string>(nawLawha, "text");
            QariLawn = Rabt.Qari<Color>(nawLawha, "color");
            QariMihwar = Rabt.Qari<Vector2>(nawLawha, "pivotOffset");
            QariArd = Rabt.Qari<int>(nawLawha, "width");
            QariIrtifa = Rabt.Qari<int>(nawLawha, "height");
            QariTarmiz = Rabt.Qari<bool>(nawLawha, "supportEncoding");

            // NGUI 3 spells the drawn size `fontSize`; NGUI 2 has no such
            // member and reports the size it actually printed at through
            // `printedSize`. Either answers the only question this file asks.
            QariHajm = Rabt.Qari<int>(nawLawha, "fontSize")
                ?? Rabt.Qari<int>(nawLawha, "printedSize");

            HadafMadda = RabtZaid.HadafKhasiya(nawLawha, "material", katib: false);
            HadafLawha = RabtZaid.HadafKhasiya(nawLawha, "mainTexture", katib: false);

            ParameterInfo[] muamalat = hadafMala.GetParameters();
            NawRuus = muamalat[0].ParameterType;
            NawMalamis = muamalat[1].ParameterType;
            NawAlwan = muamalat[2].ParameterType;
            AlwanAshira = Unsur(NawAlwan) == typeof(Color);

            QariRuus = Makhzan<Vector3[]>(NawRuus);
            KatibRuus = MakhzanKatib<Vector3[]>(NawRuus);
            QariAdadRuus = Adad(NawRuus);
            KatibAdadRuus = AdadKatib(NawRuus);

            QariMalamis = Makhzan<Vector2[]>(NawMalamis);
            KatibMalamis = MakhzanKatib<Vector2[]>(NawMalamis);
            QariAdadMalamis = Adad(NawMalamis);
            KatibAdadMalamis = AdadKatib(NawMalamis);

            QariAdadAlwan = Adad(NawAlwan);
            KatibAdadAlwan = AdadKatib(NawAlwan);
            if (AlwanAshira)
            {
                QariAlwanAshira = Makhzan<Color[]>(NawAlwan);
                KatibAlwanAshira = MakhzanKatib<Color[]>(NawAlwan);
            }
            else
            {
                QariAlwan = Makhzan<Color32[]>(NawAlwan);
                KatibAlwan = MakhzanKatib<Color32[]>(NawAlwan);
            }
        }

        /// <summary>The game's own <c>UILabel</c> type.</summary>
        public Type NawLawha { get; }

        /// <summary>
        /// <c>UILabel.OnFill</c> — the override that turns a string into quads,
        /// and the one method this takeover replaces.
        /// </summary>
        public MethodInfo HadafMala { get; }

        /// <summary>
        /// The getter of <c>material</c>, or <c>null</c> when this revision
        /// does not declare one reachable from the label.
        /// </summary>
        public MethodInfo? HadafMadda { get; }

        /// <summary>The getter of <c>mainTexture</c>, or <c>null</c>.</summary>
        public MethodInfo? HadafLawha { get; }

        /// <summary>The declared type of the vertex list parameter.</summary>
        public Type NawRuus { get; }

        /// <summary>The declared type of the texture coordinate list parameter.</summary>
        public Type NawMalamis { get; }

        /// <summary>The declared type of the colour list parameter.</summary>
        public Type NawAlwan { get; }

        /// <summary>
        /// Whether the colour list holds <see cref="Color"/> rather than
        /// <see cref="Color32"/>. NGUI 2 used the four-float form; NGUI 3 moved
        /// to the packed one. Both are supported because the difference is one
        /// conversion loop and the alternative is refusing to draw in every
        /// game still on the older revision.
        /// </summary>
        public bool AlwanAshira { get; }

        /// <summary>Reads the label's text, exactly as the game set it.</summary>
        public Func<object, string>? QariNass { get; }

        /// <summary>
        /// Reads the label's tint, which is the colour a glyph with no span of
        /// its own takes.
        /// </summary>
        public Func<object, Color>? QariLawn { get; }

        /// <summary>
        /// Reads the widget's pivot as a pair in the unit square, with one
        /// meaning the right edge and one meaning the top edge — the same order
        /// <see cref="HayyizRasm.MihwarS"/> and <see cref="HayyizRasm.MihwarA"/>
        /// use, so the value passes straight through without an inversion that
        /// could be applied twice.
        /// </summary>
        public Func<object, Vector2>? QariMihwar { get; }

        /// <summary>Reads the widget's width in pixels.</summary>
        public Func<object, int>? QariArd { get; }

        /// <summary>Reads the widget's height in pixels.</summary>
        public Func<object, int>? QariIrtifa { get; }

        /// <summary>Reads the size the label prints at, in pixels.</summary>
        public Func<object, int>? QariHajm { get; }

        /// <summary>
        /// Whether this label parses NGUI's bracket markup. A label with it off
        /// draws <c>[FF0000]</c> as eight characters a player is meant to read,
        /// and lifting markup out of such a string would delete text from the
        /// screen.
        /// </summary>
        public Func<object, bool>? QariTarmiz { get; }

        /// <summary>Reads the vertex list's backing array.</summary>
        public Func<object, Vector3[]>? QariRuus { get; }

        /// <summary>Replaces the vertex list's backing array, which is how it grows.</summary>
        public Action<object, Vector3[]>? KatibRuus { get; }

        /// <summary>Reads how many vertices the list already carries.</summary>
        public Func<object, int>? QariAdadRuus { get; }

        /// <summary>Writes the vertex count after this label's quads were appended.</summary>
        public Action<object, int>? KatibAdadRuus { get; }

        /// <summary>Reads the texture coordinate list's backing array.</summary>
        public Func<object, Vector2[]>? QariMalamis { get; }

        /// <summary>Replaces the texture coordinate list's backing array.</summary>
        public Action<object, Vector2[]>? KatibMalamis { get; }

        /// <summary>Reads the texture coordinate count.</summary>
        public Func<object, int>? QariAdadMalamis { get; }

        /// <summary>Writes the texture coordinate count.</summary>
        public Action<object, int>? KatibAdadMalamis { get; }

        /// <summary>Reads the packed colour list's backing array.</summary>
        public Func<object, Color32[]>? QariAlwan { get; }

        /// <summary>Replaces the packed colour list's backing array.</summary>
        public Action<object, Color32[]>? KatibAlwan { get; }

        /// <summary>Reads the float colour list's backing array, on NGUI 2.</summary>
        public Func<object, Color[]>? QariAlwanAshira { get; }

        /// <summary>Replaces the float colour list's backing array, on NGUI 2.</summary>
        public Action<object, Color[]>? KatibAlwanAshira { get; }

        /// <summary>Reads the colour count.</summary>
        public Func<object, int>? QariAdadAlwan { get; }

        /// <summary>Writes the colour count.</summary>
        public Action<object, int>? KatibAdadAlwan { get; }

        /// <summary>
        /// Whether everything the fill path needs resolved. The two texture
        /// substitutions are checked separately by <see cref="NizamNGui"/>,
        /// because a takeover that can fill but cannot bind its own atlas would
        /// draw Taarib's glyph rectangles sampled from the game's font texture,
        /// which is legible noise rather than text.
        /// </summary>
        public bool Muakkad =>
            QariNass is not null
            && QariLawn is not null
            && QariMihwar is not null
            && QariArd is not null
            && QariIrtifa is not null
            && QariHajm is not null
            && QariRuus is not null
            && KatibRuus is not null
            && QariAdadRuus is not null
            && KatibAdadRuus is not null
            && QariMalamis is not null
            && KatibMalamis is not null
            && QariAdadMalamis is not null
            && KatibAdadMalamis is not null
            && QariAdadAlwan is not null
            && KatibAdadAlwan is not null
            && (AlwanAshira
                ? QariAlwanAshira is not null && KatibAlwanAshira is not null
                : QariAlwan is not null && KatibAlwan is not null);

        /// <summary>
        /// Binds NGUI, or returns <c>null</c> when this game has none.
        /// </summary>
        /// <returns>
        /// The binding, or <c>null</c>. A <c>null</c> return with nothing in
        /// the log is the ordinary case: almost no game ships more than one or
        /// two of the text systems Taarib supports, and a missing one is not a
        /// failure to report.
        /// </returns>
        public static WaslNGui? Iqran()
        {
            Type? nawLawha = Rabt.Naw("UILabel");
            if (nawLawha is null)
            {
                return null;
            }

            MethodInfo? hadafMala = Mala(nawLawha);
            if (hadafMala is null)
            {
                Rabt.Ballagh(
                    "‏UILabel موجود لكن OnFill لم يُحلّ عليه ولا على UIWidget؛ تُترك نصوص NGUI للعبة وتبقى بقية الأنظمة تعمل. | "
                    + "UILabel is present but no OnFill(verts, uvs, cols) resolved on it or "
                    + "on UIWidget; NGUI labels are left to draw themselves and every other "
                    + "text system in this game is unaffected.");
                return null;
            }

            ParameterInfo[] muamalat = hadafMala.GetParameters();
            Type? unsurRuus = Unsur(muamalat[0].ParameterType);
            Type? unsurMalamis = Unsur(muamalat[1].ParameterType);
            Type? unsurAlwan = Unsur(muamalat[2].ParameterType);
            if (unsurRuus != typeof(Vector3)
                || unsurMalamis != typeof(Vector2)
                || (unsurAlwan != typeof(Color32) && unsurAlwan != typeof(Color)))
            {
                Rabt.Ballagh(
                    "‏OnFill في هذا الإصدار يأخذ قوائم من أنواع غير متوقَّعة؛ تُترك نصوص NGUI للعبة بدل الكتابة في ذاكرة بشكل مجهول. | "
                    + "UILabel.OnFill in this build takes lists of "
                    + (unsurRuus?.Name ?? "?") + ", " + (unsurMalamis?.Name ?? "?")
                    + " and " + (unsurAlwan?.Name ?? "?")
                    + " rather than Vector3, Vector2 and Color32; NGUI labels are left alone "
                    + "rather than written into buffers of an unknown shape.");
                return null;
            }

            WaslNGui wasl = new WaslNGui(nawLawha, hadafMala);
            if (!wasl.Muakkad)
            {
                Rabt.Ballagh(
                    "‏UILabel موجود لكن " + wasl.Naqis()
                    + " لم يُحلّ في هذا الإصدار؛ تُترك نصوص NGUI للعبة وتبقى بقية الأنظمة تعمل. | "
                    + "UILabel is present but " + wasl.Naqis()
                    + " did not resolve in this build of NGUI; NGUI labels are left to draw "
                    + "themselves and the rest of the takeover is unaffected.");
                return null;
            }
            return wasl;
        }

        /// <summary>
        /// Names the first member that failed to resolve, for the one line a
        /// refusal writes. A log that says "a member did not resolve" sends the
        /// next person to read all of them.
        /// </summary>
        /// <returns>The member's name, in NGUI's own spelling.</returns>
        public string Naqis()
        {
            if (QariNass is null) { return "UILabel.text"; }
            if (QariLawn is null) { return "UIWidget.color"; }
            if (QariMihwar is null) { return "UIWidget.pivotOffset"; }
            if (QariArd is null) { return "UIWidget.width"; }
            if (QariIrtifa is null) { return "UIWidget.height"; }
            if (QariHajm is null) { return "UILabel.fontSize or UILabel.printedSize"; }
            if (QariRuus is null || KatibRuus is null) { return NawRuus.Name + ".buffer"; }
            if (QariAdadRuus is null || KatibAdadRuus is null) { return NawRuus.Name + ".size"; }
            if (QariMalamis is null || KatibMalamis is null)
            {
                return NawMalamis.Name + ".buffer";
            }
            if (QariAdadMalamis is null || KatibAdadMalamis is null)
            {
                return NawMalamis.Name + ".size";
            }
            if (QariAdadAlwan is null || KatibAdadAlwan is null)
            {
                return NawAlwan.Name + ".size";
            }
            return NawAlwan.Name + ".buffer";
        }

        /// <summary>
        /// Whether a live object is a <c>UILabel</c>, which is the test every
        /// patch body runs before touching anything. A prefix installed on
        /// <c>UIWidget.OnFill</c>, because this revision did not override it on
        /// the label, sees every sprite in the panel as well.
        /// </summary>
        /// <param name="lawha">The candidate object.</param>
        /// <returns>Whether it is one.</returns>
        public bool Yantami(object? lawha)
        {
            return lawha is not null && NawLawha.IsInstanceOfType(lawha);
        }

        /// <summary>
        /// <c>OnFill</c> as this build declares it: searched on the label first
        /// and then up the chain to <c>UIWidget</c>, because a revision that did
        /// not override it on the label still fills through the base.
        /// </summary>
        private static MethodInfo? Mala(Type nawLawha)
        {
            try
            {
                for (Type? hali = nawLawha; hali is not null; hali = hali.BaseType)
                {
                    MethodInfo[] turuq = hali.GetMethods(Alamat | BindingFlags.DeclaredOnly);
                    for (int i = 0; i < turuq.Length; i++)
                    {
                        MethodInfo tariqa = turuq[i];
                        if (tariqa.IsStatic
                            || tariqa.Name != "OnFill"
                            || tariqa.GetParameters().Length != 3)
                        {
                            continue;
                        }
                        return tariqa;
                    }
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Binding UILabel.OnFill failed", khata);
            }
            return null;
        }

        /// <summary>The element type of a <c>BetterList</c> or a <c>List</c>.</summary>
        private static Type? Unsur(Type qaima)
        {
            if (!qaima.IsGenericType)
            {
                return null;
            }
            Type[] wusut = qaima.GetGenericArguments();
            return wusut.Length == 1 ? wusut[0] : null;
        }

        /// <summary>
        /// The backing array field of a list type: <c>buffer</c> for NGUI's own
        /// <c>BetterList</c>, <c>_items</c> for the <c>List</c> that later
        /// revisions substituted for it.
        /// </summary>
        private static string IsmMakhzan(Type qaima)
        {
            return qaima.IsGenericType
                && qaima.GetGenericTypeDefinition() == typeof(List<>)
                ? "_items"
                : "buffer";
        }

        /// <summary>The count field of a list type.</summary>
        private static string IsmAdad(Type qaima)
        {
            return qaima.IsGenericType
                && qaima.GetGenericTypeDefinition() == typeof(List<>)
                ? "_size"
                : "size";
        }

        private static Func<object, TMakhzan>? Makhzan<TMakhzan>(Type qaima)
        {
            return QariHaql<TMakhzan>(qaima, IsmMakhzan(qaima));
        }

        private static Action<object, TMakhzan>? MakhzanKatib<TMakhzan>(Type qaima)
        {
            return KatibHaql<TMakhzan>(qaima, IsmMakhzan(qaima));
        }

        private static Func<object, int>? Adad(Type qaima)
        {
            return QariHaql<int>(qaima, IsmAdad(qaima));
        }

        private static Action<object, int>? AdadKatib(Type qaima)
        {
            return KatibHaql<int>(qaima, IsmAdad(qaima));
        }

        /// <summary>
        /// A field getter as a delegate, compiled once. There is no
        /// <see cref="Delegate.CreateDelegate(Type, object, string)"/> for a
        /// field, and <see cref="FieldInfo.GetValue(object)"/> on a per-fill
        /// path would box every count it reads.
        /// </summary>
        private static Func<object, TQeema>? QariHaql<TQeema>(Type naw, string ism)
        {
            FieldInfo? haql = Haql(naw, ism, typeof(TQeema));
            if (haql is null)
            {
                return null;
            }
            try
            {
                ParameterExpression hadaf = Expression.Parameter(typeof(object), "hadaf");
                return Expression.Lambda<Func<object, TQeema>>(
                    Expression.Field(Expression.Convert(hadaf, haql.DeclaringType!), haql),
                    hadaf).Compile();
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Compiling a getter for " + naw.Name + "." + ism + " failed", khata);
                return null;
            }
        }

        /// <summary>A field setter as a delegate, compiled once.</summary>
        private static Action<object, TQeema>? KatibHaql<TQeema>(Type naw, string ism)
        {
            FieldInfo? haql = Haql(naw, ism, typeof(TQeema));
            if (haql is null || haql.IsInitOnly)
            {
                return null;
            }
            try
            {
                ParameterExpression hadaf = Expression.Parameter(typeof(object), "hadaf");
                ParameterExpression qeema = Expression.Parameter(typeof(TQeema), "qeema");
                return Expression.Lambda<Action<object, TQeema>>(
                    Expression.Assign(
                        Expression.Field(Expression.Convert(hadaf, haql.DeclaringType!), haql),
                        qeema),
                    hadaf, qeema).Compile();
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Compiling a setter for " + naw.Name + "." + ism + " failed", khata);
                return null;
            }
        }

        private static FieldInfo? Haql(Type naw, string ism, Type nawQeema)
        {
            try
            {
                for (Type? hali = naw; hali is not null; hali = hali.BaseType)
                {
                    FieldInfo? haql = hali.GetField(ism, Alamat | BindingFlags.DeclaredOnly);
                    if (haql is not null && !haql.IsStatic && haql.FieldType == nawQeema)
                    {
                        return haql;
                    }
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Binding " + naw.Name + "." + ism + " failed", khata);
            }
            return null;
        }
    }

    /// <summary>
    /// نظام NGUI — the NGUI takeover: three patches, one lookup rule, and a
    /// refusal to guess.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The three patch targets.</b> A prefix on <c>UILabel.OnFill</c>, which
    /// writes Taarib's quads and skips NGUI's; a postfix on the label's
    /// <c>material</c> getter and a postfix on its <c>mainTexture</c> getter,
    /// which hand back Taarib's atlas for the labels this object drew. The last
    /// two are not garnish. NGUI batches widgets by material and keys some of
    /// its own bookkeeping on the texture, and a label whose geometry came from
    /// Taarib's atlas while its draw call still binds the game's font texture
    /// samples glyph rectangles out of the wrong image — legible noise rather
    /// than text, and a failure that looks like the atlas is corrupt when it is
    /// not.
    /// </para>
    /// <para>
    /// <b>The lookup never guesses.</b> The label's text is hashed with
    /// <see cref="Ruqaa.MiftahMinNass(string)"/> and looked up in the installed
    /// patch. A miss means this label is drawing something the patch does not
    /// cover — a debug counter, a player's own name, a string the compiler never
    /// saw — and the prefix returns <c>true</c> so NGUI draws it exactly as it
    /// always did. Nothing here reaches for a nearby string, a normalised form
    /// or a nearest size.
    /// </para>
    /// <para>
    /// <b>Where the markup goes.</b> NGUI's dialect is <c>[RRGGBB]</c> to push
    /// a colour, <c>[-]</c> to pop one, and <c>[b]</c>, <c>[i]</c>, <c>[u]</c>
    /// and <c>[s]</c> for the named styles. It is lifted by
    /// <see cref="Nasq.Hallil"/> through <see cref="NassMuhaddar"/> with
    /// <see cref="NawNasq.NGui"/>, and it is not re-implemented here: one
    /// parser per dialect, in the assembly both backends share, is the only
    /// arrangement in which the Mono and IL2CPP builds cannot come to disagree
    /// about what <c>[FF0000]</c> means. The dialect is passed only when the
    /// label's own <c>supportEncoding</c> is on, because a label with it off
    /// draws those six characters and lifting them would delete text a player
    /// was meant to read. The lift happens before shaping for the reason
    /// <c>Nasq</c> sets out at length: to the bidirectional algorithm a bracket
    /// tag is not a tag, it is a run of mixed-direction characters, and
    /// <c>[FF0000]</c> inside an Arabic sentence comes back out reordered and
    /// lodged in the middle of a word.
    /// </para>
    /// <para>
    /// <b>What allocates.</b> <see cref="Ibda"/> allocates: it resolves types,
    /// compiles accessors and installs patches, once, at load. The fill path
    /// allocates in exactly two circumstances, both of them a growth that
    /// happens a handful of times in a session and then never again — the
    /// shared geometry buffers reaching the longest string seen so far, and a
    /// <c>BetterList</c> backing array being replaced with a larger one because
    /// the label needs more vertices than the list was holding, which is the
    /// same growth <c>BetterList.Add</c> would have performed itself.
    /// Everything else on that path is a span over memory that already exists.
    /// </para>
    /// </remarks>
    public sealed class NizamNGui : IDisposable
    {
        private static NizamNGui? hali;

        private readonly WaslNGui wasl;
        private readonly MasdarAshkal masdar;
        private readonly Ruqaa ruqaa;
        private readonly Takhtit? takhtit;

        /// <summary>
        /// The capture session, or <c>null</c> when this takeover is replacing
        /// text rather than recording it. Non-null puts it in observe-only
        /// posture: record what the game is about to draw, and let the game
        /// draw it.
        /// </summary>
        private readonly JalsatIltiqat? jalsa;

        /// <summary>
        /// Records one sighting into the capture session.
        /// </summary>
        /// <remarks>
        /// The string is recorded exactly as the game handed it over, markup
        /// and all, because that is the string the takeover will be asked to
        /// match next time — a capture that stored a cleaned form would build a
        /// patch keyed on something no component ever draws.
        /// <para>
        /// Allocating a managed string per sighting is deliberate. Capture runs
        /// instead of replacement, never beside it, so the frame budget it
        /// spends is a translator's rather than a player's.
        /// </para>
        /// </remarks>
        /// <param name="asl">The string the game is about to draw.</param>
        /// <param name="miftah">Its key, so a sighting is counted once.</param>
        private void Iltaqit(string asl, ulong miftah, Component? juz)
        {
            JalsatIltiqat? hali = jalsa;
            if (hali is null || string.IsNullOrEmpty(asl))
            {
                return;
            }
            TalabIltiqat talab = default;
            talab.Huwiya = miftah;
            talab.Asl = asl;
            talab.Nizam = NizamNass.Ngui;
            talab.Itar = Time.frameCount;
            // The path and the scene are what make a capture actionable: the
            // reader's fold key is (text, component path), so leaving the path
            // empty collapses every widget that draws the same word into one
            // record with one measurement, and a translator reading the session
            // has no way back to the screen a string was on.
            talab.Masar = Masar(juz);
            talab.Mashhad = juz is null
                ? null
                : UnityEngine.SceneManagement.SceneManager.GetActiveScene().name;
            hali.Sajjil(in talab);
        }

        /// <summary>The component's path in its scene, as a translator reads it.</summary>
        /// <param name="juz">The component, or null when the caller has none.</param>
        /// <returns>The path, or the empty string.</returns>
        private static string Masar(Component? juz)
        {
            if (juz is null)
            {
                return string.Empty;
            }
            Transform? tahwil = juz.transform;
            if (tahwil is null)
            {
                return string.Empty;
            }
            System.Text.StringBuilder bani = new System.Text.StringBuilder(64);
            bani.Append(tahwil.name);
            Transform? walid = tahwil.parent;
            // Bounded because a path is a label, not a proof: a scene graph
            // deeper than this is pathological and the top of the path is the
            // part that identifies the screen.
            for (int umq = 0; umq < 16 && walid is not null; umq++)
            {
                bani.Insert(0, '/').Insert(0, walid.name);
                walid = walid.parent;
            }
            return bani.ToString();
        }

        private readonly MaqbadSiyaq? siyaq;
        private readonly MaqbadSilsila? silsila;
        private readonly MakhzanRusum makhzan;
        private readonly NassMuhaddar muhaddar;
        private readonly Dictionary<int, bool> mamlukat;
        private readonly List<MethodInfo> hidaf;
        private readonly Harmony harmoni;

        private bool amil;
        private bool hars;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        private NizamNGui(
            Harmony harmoni,
            WaslNGui wasl,
            MasdarAshkal masdar,
            Ruqaa ruqaa,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit,
            JalsatIltiqat? jalsa)
        {
            this.harmoni = harmoni;
            this.wasl = wasl;
            this.masdar = masdar;
            this.ruqaa = ruqaa;
            this.siyaq = siyaq;
            this.silsila = silsila;
            this.takhtit = takhtit;
            this.jalsa = jalsa;
            makhzan = new MakhzanRusum();
            muhaddar = new NassMuhaddar();
            mamlukat = new Dictionary<int, bool>(64);
            hidaf = new List<MethodInfo>(3);
            amil = true;
        }

        /// <summary>The live NGUI takeover, or <c>null</c> when none is installed.</summary>
        public static NizamNGui? Hali => hali;

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <summary>
        /// Discovers NGUI, installs the patches, and returns the takeover.
        /// </summary>
        /// <param name="harmoni">The plugin's Harmony instance.</param>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="masdar">The glyph source and its uploaded atlas.</param>
        /// <param name="siyaq">
        /// The engine context, for text the compiler never laid out.
        /// <c>null</c> leaves precomputed layouts working and declines every
        /// string with no precomputed layout at the size it is drawn at.
        /// </param>
        /// <param name="silsila">The font chain runtime layout shapes with.</param>
        /// <param name="takhtit">
        /// A layout buffer used by this takeover and nothing else. A
        /// <see cref="Takhtit"/> exposes its results as spans over one reused
        /// pair of arrays, so sharing one with another takeover would let one
        /// label's layout overwrite the glyphs another is halfway through
        /// turning into triangles.
        /// </param>
        /// <returns>
        /// The takeover, or <c>null</c> when this game has no NGUI, which is
        /// not an error.
        /// </returns>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="harmoni"/>, <paramref name="ruqaa"/> or
        /// <paramref name="masdar"/> is null.
        /// </exception>
        public static NizamNGui? Ibda(
            Harmony harmoni,
            Ruqaa ruqaa,
            MasdarAshkal masdar,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit,
            JalsatIltiqat? jalsa)
        {
            if (harmoni is null)
            {
                throw new ArgumentNullException(nameof(harmoni));
            }
            if (ruqaa is null)
            {
                throw new ArgumentNullException(nameof(ruqaa));
            }
            if (masdar is null)
            {
                throw new ArgumentNullException(nameof(masdar));
            }
            if (hali is not null)
            {
                return hali;
            }

            WaslNGui? wasl = WaslNGui.Iqran();
            if (wasl is null)
            {
                return null;
            }
            if (wasl.HadafMadda is null && wasl.HadafLawha is null)
            {
                Rabt.Ballagh(
                    "‏UILabel لا يعلن material ولا mainTexture في هذا الإصدار، فلا سبيل لربط لوحة تعريب؛ تُترك نصوص NGUI للعبة. | "
                    + "UILabel declares neither material nor mainTexture in this build, so "
                    + "Taarib's atlas cannot be bound to the draw call; NGUI labels are left "
                    + "alone rather than drawn from the game's own font texture.");
                return null;
            }

            NizamNGui nizam = new NizamNGui(
                harmoni, wasl, masdar, ruqaa, siyaq, silsila, takhtit, jalsa);
            hali = nizam;

            bool mala = nizam.Rakkib(
                wasl.HadafMala, typeof(TarqeeMalaNGui), nameof(TarqeeMalaNGui.Sabiq),
                "UILabel.OnFill", sabiq: true);
            nizam.Rakkib(
                wasl.HadafMadda, typeof(TarqeeMaddaNGui), nameof(TarqeeMaddaNGui.Lahiq),
                "UILabel.get_material", sabiq: false);
            nizam.Rakkib(
                wasl.HadafLawha, typeof(TarqeeLawhaNGui), nameof(TarqeeLawhaNGui.Lahiq),
                "UILabel.get_mainTexture", sabiq: false);

            if (!mala)
            {
                Rabt.Ballagh(
                    "لم يُركَّب ترقيع UILabel.OnFill؛ نصوص NGUI تبقى بلغة اللعبة وبقية الأنظمة تعمل. | "
                    + "The UILabel.OnFill patch could not be installed; NGUI text stays in "
                    + "the game's original language and every other text system keeps "
                    + "working.");
                nizam.Dispose();
                return null;
            }
            return nizam;
        }

        /// <summary>
        /// Whether this takeover drew the component's current geometry, which
        /// is the question the two texture postfixes ask before they substitute
        /// Taarib's atlas for the font's own.
        /// </summary>
        /// <param name="lawha">The label.</param>
        /// <returns>Whether Taarib owns this label's draw.</returns>
        public bool Yamlik(object? lawha)
        {
            return amil && lawha is Component juz
                && mamlukat.ContainsKey(juz.GetInstanceID());
        }

        /// <summary>Taarib's atlas material for one label.</summary>
        /// <remarks>
        /// The other atlas is never offered in its place. Quads laid out from
        /// the patch index the patch's own atlas and quads laid out at run time
        /// index the atlas this session rasterized into; the two are different
        /// pictures with different glyphs at different coordinates, so a label
        /// built from one and sampled from the other paints solid blocks. A
        /// label whose own atlas has no material yet is left to NGUI instead.
        /// </remarks>
        /// <param name="lawha">The label, which decides which atlas it was drawn from.</param>
        /// <returns>The material, or <c>null</c> when that atlas is not resident.</returns>
        public Material? Madda(object? lawha)
        {
            return masdar.Madda(MinRuqaa(lawha), 0);
        }

        /// <summary>Taarib's atlas page for one label.</summary>
        /// <param name="lawha">The label.</param>
        /// <returns>The texture, or <c>null</c> when that atlas's page is not resident.</returns>
        public Texture2D? Lawha(object? lawha)
        {
            return masdar.LawhatSafha(MinRuqaa(lawha), 0);
        }

        /// <summary>
        /// The whole interception: read the string, look it up, and either
        /// append Taarib's geometry to the draw call's three lists or hand the
        /// label straight back to NGUI.
        /// </summary>
        /// <param name="lawha">The label being asked to draw.</param>
        /// <param name="ruus">The vertex list.</param>
        /// <param name="malamis">The texture coordinate list.</param>
        /// <param name="alwan">The colour list.</param>
        /// <returns>
        /// Whether Taarib filled them. <c>false</c> means the lists are exactly
        /// as they were and NGUI's own fill must run, which is the correct
        /// answer for every string this patch does not cover.
        /// </returns>
        public bool Yarsum(object? lawha, object? ruus, object? malamis, object? alwan)
        {
            if (!amil || hars || lawha is null || ruus is null || malamis is null
                || alwan is null || !masdar.Amil || !wasl.Yantami(lawha))
            {
                return false;
            }
            hars = true;
            try
            {
                return Rassim(lawha, ruus, malamis, alwan);
            }
            catch (KhataTaarib khata)
            {
                Rabt.Ballagh("NGUI UILabel: drawing one string failed", khata);
                Utruk(lawha);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("drawing an NGUI label threw", khata);
                return false;
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// Switches the takeover off, naming the reason once. Everything after
        /// this declines, which leaves NGUI drawing exactly as it did before
        /// the plugin loaded.
        /// </summary>
        /// <param name="sabab">What failed and what it costs.</param>
        /// <param name="khata">The exception behind it.</param>
        public void Awqif(string sabab, Exception khata)
        {
            if (!amil)
            {
                return;
            }
            amil = false;
            mamlukat.Clear();
            Rabt.Ballagh(
                "NGUI UILabel: " + sabab + "; NGUI labels are returned to the game's own "
                + "renderer and every other text system is unaffected", khata);
        }

        /// <summary>
        /// Removes the patches and forgets every label. The geometry already on
        /// screen is NGUI's to rebuild: a panel repopulates its draw calls when
        /// a widget next changes, and the two texture postfixes stop answering
        /// with Taarib's atlas the moment <see cref="Amil"/> is false.
        /// </summary>
        public void Dispose()
        {
            amil = false;
            for (int i = 0; i < hidaf.Count; i++)
            {
                try
                {
                    harmoni.Unpatch(hidaf[i], HarmonyPatchType.All, harmoni.Id);
                }
                catch (Exception khata)
                {
                    Rabt.Ballagh("Removing the patch on " + hidaf[i].Name + " failed", khata);
                }
            }
            hidaf.Clear();
            mamlukat.Clear();
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
        }

        /// <summary>The whole draw, inside the re-entrancy guard.</summary>
        private bool Rassim(object lawha, object ruus, object malamis, object alwan)
        {
            string? khaam = wasl.QariNass!(lawha);
            if (string.IsNullOrEmpty(khaam))
            {
                Utruk(lawha);
                return false;
            }

            // MiftahMinNass encodes the string to UTF-8 to hash it. Under 512
            // bytes that encode is a stackalloc, so the lookup that decides
            // whether Taarib owns a label allocates nothing at all — which
            // matters because it runs for every label the panel rebuilds,
            // including all the ones the patch does not cover.
            ulong miftah = Ruqaa.MiftahMinNass(khaam!);

            // Capture mode observes and never replaces: the session records what
            // the game is about to draw and the game then draws it. This is the
            // only way text reaches a patch for a game whose components no
            // static reader can open — and on this backend the session existed,
            // was announced, and was never handed to a text system, so it
            // recorded nothing at all.
            if (jalsa is not null)
            {
                Iltaqit(khaam!, miftah, lawha as Component);
                Utruk(lawha);
                return false;
            }
            int fahras = ruqaa.JidNass(miftah);
            if (fahras < 0)
            {
                Rabt.Fawt(khaam!, miftah);
                Utruk(lawha);
                return false;
            }

            // Counted so the log can answer "how much of this game does the
            // patch cover", which a list of misses alone cannot.
            Rabt.Isaba(miftah);

            float hajm = wasl.QariHajm!(lawha);
            if (!(hajm > 0f))
            {
                Utruk(lawha);
                return false;
            }
            float ard = wasl.QariArd!(lawha);
            float irtifa = wasl.QariIrtifa!(lawha);

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            float hajmFili;
            float irtifaTakhtit;

            // The exact quarter-pixel size, never the nearest. Drawing a layout
            // measured at one size into a widget the game sized at another is
            // how text that fitted in the compiler's measurement overflows on a
            // player's screen — and the compiler's overflow report, which said
            // it fitted, would be wrong.
            ushort hajmRubi = Nasij.HajmRubi(hajm);
            bool minRuqaa = ruqaa.JidTakhtit(fahras, hajmRubi, out MadkhalTakhtit madkhal);
            if (minRuqaa)
            {
                huruf = ruqaa.HurufTakhtit(in madkhal);
                sutur = ruqaa.SuturTakhtit(in madkhal);
                nitaqat = makhzan.Hawwil(ruqaa.NitaqatNass(fahras));
                hajmFili = madkhal.Hajm;
                irtifaTakhtit = madkhal.Irtifa;
            }
            else if (!Khattit(
                lawha, fahras, hajm, ard, irtifa,
                out huruf, out sutur, out nitaqat, out hajmFili, out irtifaTakhtit))
            {
                Utruk(lawha);
                return false;
            }

            if (huruf.IsEmpty || !(hajmFili > 0f))
            {
                Utruk(lawha);
                return false;
            }

            float hajmLawha = masdar.HajmLawha(
                hajmFili, QiyasShasha.BikselLilWahda(lawha), minRuqaa);
            bool tathbit = Nasij.YuthabbatQalam(masdar.Namat, hajmFili, hajmLawha);
            if (!minRuqaa && !masdar.Aqim(huruf, hajmLawha, tathbit))
            {
                Utruk(lawha);
                return false;
            }

            HayyizRasm hayyiz = default;
            hayyiz.Ard = ard;
            hayyiz.Irtifa = irtifa;
            Vector2 mihwar = wasl.QariMihwar!(lawha);
            hayyiz.MihwarS = mihwar.x;
            hayyiz.MihwarA = mihwar.y;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;

            // NGUI centres a label's text block inside its widget rectangle
            // vertically whatever the horizontal alignment is, and Taarib
            // already resolved the horizontal side while laying out, so this is
            // the one placement decision left to make here.
            hayyiz.Muhadhaha = MuhadhahaRasiya.Wasat;

            TalabNasij talab = default;
            talab.Huruf = huruf;
            talab.Sutur = sutur;
            talab.Nitaqat = nitaqat;
            talab.Hayyiz = hayyiz;
            talab.Lawn = LawnRasm.Min(LawnMuazzam(wasl.QariLawn!(lawha)));
            talab.Hajm = hajmFili;
            talab.HajmLawha = hajmLawha;
            talab.Safha = 0;

            // A coverage atlas holds one bitmap per size with the pen's
            // fractional position already folded into its coverage, and an NGUI
            // widget maps one layout pixel to one local unit, so the quad must
            // land on the integer grid or that fraction is applied twice and
            // the whole line blurs. A distance-field atlas holds one bitmap for
            // every size, so nothing is snapped and every bucket is zero.
            talab.Tathbit = tathbit;

            return Aktub(lawha, ruus, malamis, alwan, in talab, minRuqaa, huruf.Length);
        }

        /// <summary>
        /// Appends the geometry to the three lists the draw call already owns.
        /// </summary>
        private bool Aktub(
            object lawha,
            object ruus,
            object malamis,
            object alwan,
            in TalabNasij talab,
            bool minRuqaa,
            int adadHuruf)
        {
            int bidaya = wasl.QariAdadRuus!(ruus);
            if (bidaya != wasl.QariAdadMalamis!(malamis) || bidaya != wasl.QariAdadAlwan!(alwan))
            {
                // Three parallel lists that disagree about their own length is
                // not a state this file can write into safely, and it is not
                // one Taarib caused. Decline, and let NGUI fill: it is the only
                // action here that cannot make the frame worse.
                Utruk(lawha);
                return false;
            }

            int matlub = adadHuruf * Nasij.RuusLiShakl;
            Vector3[] makhzanRuus = Wassi(wasl.QariRuus!(ruus), bidaya, matlub);
            Vector2[] makhzanMalamis = Wassi(wasl.QariMalamis!(malamis), bidaya, matlub);
            wasl.KatibRuus!(ruus, makhzanRuus);
            wasl.KatibMalamis!(malamis, makhzanMalamis);

            Color32[] makhzanAlwan = Array.Empty<Color32>();
            Color[] makhzanAlwanAshira = Array.Empty<Color>();
            if (wasl.AlwanAshira)
            {
                makhzanAlwanAshira = Wassi(wasl.QariAlwanAshira!(alwan), bidaya, matlub);
                wasl.KatibAlwanAshira!(alwan, makhzanAlwanAshira);
            }
            else
            {
                makhzanAlwan = Wassi(wasl.QariAlwan!(alwan), bidaya, matlub);
                wasl.KatibAlwan!(alwan, makhzanAlwan);
            }

            // The shared buffers supply the triangle span NGUI does not want
            // and, on NGUI 2, the packed colours its float list cannot receive
            // directly. Sizing them is what makes both spans exist at all:
            // Nasij refuses a build outright rather than writing a partial one.
            makhzan.Wassi(adadHuruf);

            MakhzanNasij hadaf = default;
            hadaf.Ruus = MemoryMarshal.Cast<Vector3, NuqtaRasm>(
                new Span<Vector3>(makhzanRuus, bidaya, matlub));
            hadaf.Malamis = MemoryMarshal.Cast<Vector2, NuqtaMulmas>(
                new Span<Vector2>(makhzanMalamis, bidaya, matlub));
            hadaf.Alwan = wasl.AlwanAshira
                ? MemoryMarshal.Cast<Color32, LawnRasm>(makhzan.Alwan.AsSpan())
                : MemoryMarshal.Cast<Color32, LawnRasm>(
                    new Span<Color32>(makhzanAlwan, bidaya, matlub));
            hadaf.Muthallathat = makhzan.Muthallathat.AsSpan();
            hadaf.Dharrat = Span<DharraMawduaa>.Empty;

            NatijaNasij natija = Nasij.Ibni(in talab, masdar.Khareeta(minRuqaa), hadaf);
            if (!natija.Kafa)
            {
                // Every span above was sized from the same upper bound Nasij
                // demands, so a shortfall here means the two disagree about
                // that bound rather than that the label is long.
                Rabt.Ballagh(
                    "NGUI UILabel: the mesh buffer refused a build sized to Nasij's own "
                    + "upper bound; this label is left to NGUI and the takeover stays "
                    + "installed.");
                Utruk(lawha);
                return false;
            }
            if (natija.AdadAshkal == 0)
            {
                Utruk(lawha);
                return false;
            }
            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            if (wasl.AlwanAshira)
            {
                for (int i = 0; i < natija.AdadRuus; i++)
                {
                    Color32 mahzum = makhzan.Alwan[i];
                    makhzanAlwanAshira[bidaya + i] = new Color(
                        mahzum.r / 255f, mahzum.g / 255f, mahzum.b / 255f, mahzum.a / 255f);
                }
            }

            int nihaya = bidaya + natija.AdadRuus;
            wasl.KatibAdadRuus!(ruus, nihaya);
            wasl.KatibAdadMalamis!(malamis, nihaya);
            wasl.KatibAdadAlwan!(alwan, nihaya);

            if (lawha is Component juz)
            {
                mamlukat[juz.GetInstanceID()] = minRuqaa;
            }
            return true;
        }

        /// <summary>
        /// Lays the patch's translation out at a size the compiler never
        /// measured, using the constraint the compiler recorded for the slot.
        /// </summary>
        private bool Khattit(
            object lawha,
            int fahras,
            float hajm,
            float ard,
            float irtifa,
            out ReadOnlySpan<TaaribHarf> huruf,
            out ReadOnlySpan<TaaribSatr> sutur,
            out ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            out float hajmFili,
            out float irtifaTakhtit)
        {
            huruf = ReadOnlySpan<TaaribHarf>.Empty;
            sutur = ReadOnlySpan<TaaribSatr>.Empty;
            nitaqat = ReadOnlySpan<TaaribNitaqUslub>.Empty;
            hajmFili = hajm;
            irtifaTakhtit = 0f;

            Takhtit? buffer = takhtit;
            MaqbadSiyaq? qab = siyaq;
            MaqbadSilsila? chain = silsila;
            if (buffer is null || qab is null || chain is null)
            {
                BallighTakhtit();
                return false;
            }

            Func<object, bool>? qariTarmiz = wasl.QariTarmiz;
            bool yurammiz = qariTarmiz is null || qariTarmiz(lawha);
            NawNasq lahja = yurammiz ? NawNasq.NGui : NawNasq.Bila;
            if (!muhaddar.Hayyi(
                ruqaa, fahras, makhzan, lahja, hajm,
                out ReadOnlySpan<char> nass, out ReadOnlySpan<TaaribNitaqUslub> nitaqatNass))
            {
                return false;
            }

            KhiyaratTakhtit khiyarat;
            float ardTalab = ard;
            float irtifaTalab = irtifa;
            if (ruqaa.JidQayd(fahras, out MadkhalQayd qayd))
            {
                khiyarat = qayd.Khiyarat();
                if (qayd.ArdMutah > 0f)
                {
                    ardTalab = qayd.ArdMutah;
                }
                if (qayd.IrtifaMutah > 0f)
                {
                    irtifaTalab = qayd.IrtifaMutah;
                }
                if (qayd.HajmTilqai && qayd.Hajm > 0f)
                {
                    hajmFili = qayd.Hajm;
                }
            }
            else
            {
                // No constraint row for this string. The widget's own rectangle
                // is the honest fallback for the width, and every policy takes
                // its documented default rather than a guess at what the game
                // meant — a wrong justification mode is a flaw, and text laid
                // out against a width nothing measured is a paragraph that
                // wraps in the wrong places.
                khiyarat = default;
            }

            try
            {
                TalabTakhtit talab = default;
                talab.Nass = nass;
                talab.Nitaqat = nitaqatNass;
                talab.Sifat = ReadOnlySpan<TaaribSifa>.Empty;
                talab.Hajm = hajmFili;
                talab.ArdMutah = ardTalab;
                talab.IrtifaMutah = irtifaTalab;
                talab.Khiyarat = khiyarat;
                buffer.Khattit(qab, chain, talab);
            }
            catch (KhataTaarib khata)
            {
                // Attributable to this one string and these parameters, so the
                // takeover stays installed and the next label still draws.
                Rabt.Ballagh("NGUI UILabel: " + khata.Arabi + " | " + khata.Injilizi);
                return false;
            }

            huruf = buffer.Huruf;
            sutur = buffer.Sutur;
            nitaqat = nitaqatNass;
            hajmFili = buffer.Hajm;
            irtifaTakhtit = buffer.Irtifa;
            return !huruf.IsEmpty;
        }

        /// <summary>
        /// Forgets a label, so the two texture postfixes stop substituting
        /// Taarib's atlas for a draw call that no longer carries Taarib's
        /// glyphs.
        /// </summary>
        private void Utruk(object? lawha)
        {
            if (lawha is Component juz)
            {
                mamlukat.Remove(juz.GetInstanceID());
            }
        }

        /// <summary>Which atlas a label was last drawn from.</summary>
        private bool MinRuqaa(object? lawha)
        {
            return lawha is Component juz
                && mamlukat.TryGetValue(juz.GetInstanceID(), out bool min)
                && min;
        }

        /// <summary>Installs one patch and records its target for removal.</summary>
        private bool Rakkib(MethodInfo? hadaf, Type hamil, string ism, string wasf, bool sabiq)
        {
            if (hadaf is null)
            {
                return false;
            }
            try
            {
                MethodInfo? tariqa = hamil.GetMethod(
                    ism, BindingFlags.Public | BindingFlags.Static);
                if (tariqa is null)
                {
                    Rabt.Ballagh("The patch method for " + wasf + " is missing from this build.");
                    return false;
                }
                HarmonyMethod tarqee = new HarmonyMethod(tariqa);
                if (sabiq)
                {
                    harmoni.Patch(hadaf, prefix: tarqee);
                }
                else
                {
                    harmoni.Patch(hadaf, postfix: tarqee);
                }
                hidaf.Add(hadaf);
                return true;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(
                    "Patching " + wasf + " failed; that one interception is off and the rest "
                    + "of the NGUI takeover continues", khata);
                return false;
            }
        }

        /// <summary>
        /// Reports a label whose glyphs need more than one atlas page, once. An
        /// NGUI widget produces one draw call and a draw call binds one
        /// texture, so the glyphs on the other pages cannot be drawn from here;
        /// the fix is a compiler-side page budget and not anything a player can
        /// act on, which is why this is said once and not per frame.
        /// </summary>
        private void BallighSafahat()
        {
            if (ballaghSafahat)
            {
                return;
            }
            ballaghSafahat = true;
            Rabt.Ballagh(
                "حروف بعض نصوص NGUI موزَّعة على أكثر من صفحة أطلس، وودجت NGUI ترسم صفحة واحدة؛ تُرسم حروف الصفحة الأولى وتغيب البقية. | "
                + "Some NGUI labels have glyphs on more than one atlas page and an NGUI "
                + "widget draws one page; the glyphs on the first page are drawn and the "
                + "rest are missing from the labels that need them.");
        }

        /// <summary>
        /// Reports, once, that runtime layout is unavailable. Without it a
        /// string the compiler laid out at one size draws only at that size,
        /// which in NGUI is common: a label's fontSize is edited per prefab and
        /// two menus rarely agree.
        /// </summary>
        private void BallighTakhtit()
        {
            if (ballaghTakhtit)
            {
                return;
            }
            ballaghTakhtit = true;
            Rabt.Ballagh(
                "لا يوجد تخطيط زمن تشغيل لنصوص NGUI، فالنصوص التي لم يقِسها المترجم بهذا الحجم تبقى بلغة اللعبة. | "
                + "No runtime layout is available to the NGUI takeover, so a string the "
                + "compiler did not measure at the size a label draws at stays in the game's "
                + "original language.");
        }

        /// <summary>
        /// Unity's colour as the packed byte order Nasij writes. Named rather
        /// than open coded because the implicit conversion clamps and rounds,
        /// and doing that in two places is how two call sites come to disagree
        /// about a colour by one unit.
        /// </summary>
        private static uint LawnMuazzam(Color lawn)
        {
            Color32 mahzum = lawn;
            return ((uint)mahzum.r << 24) | ((uint)mahzum.g << 16)
                | ((uint)mahzum.b << 8) | mahzum.a;
        }

        /// <summary>
        /// Grows a list's backing array to hold <paramref name="matlub"/> more
        /// elements past <paramref name="mashghul"/>, preserving what is
        /// already there.
        /// </summary>
        /// <remarks>
        /// This is the one allocation the fill path can make, and it is the
        /// same one <c>BetterList.Add</c> would have made: NGUI's list grows by
        /// replacing its buffer. Capacity doubles rather than fitting exactly,
        /// so a label that gains a word next frame does not allocate again.
        /// </remarks>
        private static T[] Wassi<T>(T[]? mawjud, int mashghul, int matlub)
        {
            int lazim = mashghul + matlub;
            if (mawjud is not null && mawjud.Length >= lazim)
            {
                return mawjud;
            }
            int siaa = mawjud is null || mawjud.Length < 4 ? 4 : mawjud.Length;
            while (siaa < lazim)
            {
                siaa *= 2;
            }
            T[] jadid = new T[siaa];
            if (mawjud is not null && mashghul > 0)
            {
                Array.Copy(mawjud, jadid, mashghul > mawjud.Length ? mawjud.Length : mashghul);
            }
            return jadid;
        }
    }

    /// <summary>
    /// ترقيع ملء النسيج — the prefix on <c>UILabel.OnFill</c>.
    /// </summary>
    /// <remarks>
    /// The three list parameters are declared as <see cref="object"/> because
    /// <c>BetterList&lt;T&gt;</c> cannot be named at compile time in an
    /// assembly that must also load in a game without NGUI. Harmony binds a
    /// patch parameter positionally through <c>__0</c>, <c>__1</c> and
    /// <c>__2</c> and permits a reference type to be received as
    /// <see cref="object"/>, so the widening costs nothing at run time.
    /// </remarks>
    public static class TarqeeMalaNGui
    {
        /// <summary>
        /// Appends Taarib's quads when the patch covers the label's string.
        /// </summary>
        /// <param name="__instance">The widget, injected by Harmony.</param>
        /// <param name="__0">The vertex list it was handed.</param>
        /// <param name="__1">The texture coordinate list.</param>
        /// <param name="__2">The colour list.</param>
        /// <returns>
        /// <c>false</c> to skip NGUI's own fill, which is what stops a pipeline
        /// with no Arabic OpenType layout in it from choosing glyphs;
        /// <c>true</c> to let it run untouched, which is the answer for every
        /// widget that is not a label and every string the patch does not
        /// cover.
        /// </returns>
        public static bool Sabiq(object __instance, object __0, object __1, object __2)
        {
            NizamNGui? nizam = NizamNGui.Hali;
            return nizam is null || !nizam.Yarsum(__instance, __0, __1, __2);
        }
    }

    /// <summary>
    /// ترقيع المادة — the postfix on <c>UILabel.material</c>'s getter.
    /// </summary>
    /// <remarks>
    /// NGUI answers that property with the UIFont's own material and then
    /// batches every widget that shares it into one draw call. For a label
    /// Taarib drew, the answer has to be Taarib's atlas material instead:
    /// otherwise the quads written from Taarib's atlas rectangles are sampled
    /// out of the font's texture, which puts a different letter — or a piece of
    /// one — in every position.
    /// </remarks>
    public static class TarqeeMaddaNGui
    {
        /// <summary>
        /// Substitutes Taarib's atlas material for the font's own.
        /// </summary>
        /// <param name="__instance">The label, injected by Harmony.</param>
        /// <param name="__result">
        /// The material NGUI was about to answer with, replaced in place when
        /// Taarib owns this label's draw and left exactly as it was otherwise.
        /// </param>
        public static void Lahiq(object __instance, ref Material __result)
        {
            NizamNGui? nizam = NizamNGui.Hali;
            if (nizam is null || !nizam.Yamlik(__instance))
            {
                return;
            }
            Material? madda = nizam.Madda(__instance);
            if (madda != null)
            {
                __result = madda;
            }
        }
    }

    /// <summary>
    /// ترقيع اللوحة — the postfix on <c>UILabel.mainTexture</c>'s getter.
    /// </summary>
    /// <remarks>
    /// NGUI keys part of its own batching and its draw call rebuild on the
    /// texture rather than the material, so both have to answer with Taarib's
    /// atlas or a label lands in a draw call with the wrong one bound. The
    /// texture NGUI would have answered with is a UIAtlas page or a dynamic
    /// Font texture, both of which Decision 5 forbids reading and neither of
    /// which holds a shaped Arabic glyph.
    /// </remarks>
    public static class TarqeeLawhaNGui
    {
        /// <summary>
        /// Substitutes Taarib's atlas page for the font's own texture.
        /// </summary>
        /// <param name="__instance">The label, injected by Harmony.</param>
        /// <param name="__result">
        /// The texture NGUI was about to answer with, replaced in place when
        /// Taarib owns this label's draw.
        /// </param>
        public static void Lahiq(object __instance, ref Texture __result)
        {
            NizamNGui? nizam = NizamNGui.Hali;
            if (nizam is null || !nizam.Yamlik(__instance))
            {
                return;
            }
            Texture2D? lawha = nizam.Lawha(__instance);
            if (lawha is not null)
            {
                __result = lawha;
            }
        }
    }
}
