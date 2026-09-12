// نظام FairyGUI — the takeover for TextField, GTextField and GRichTextField.
//
// FairyGUI is a UI framework built around an external editor rather than
// Unity's own, and it is the default choice in a large part of the
// Chinese-developed and mobile Unity catalogue. Two of its properties decide
// the shape of this file.
//
// IT HAS TWO OBJECT LAYERS AND ONLY ONE OF THEM DRAWS. `GTextField` — with its
// two subclasses `GBasicTextField` and `GRichTextField` — is the logical object
// a game's own script talks to: it owns the string, the text format and the
// auto-size policy. `TextField`, and its subclass `RichTextField`, is the
// display object underneath it, and that is the layer that measures lines and
// emits geometry. So the takeover goes into `TextField`, not into `GTextField`:
// a patch on the logical object would fight the game's own scripts over a
// property, while a patch on the display object replaces exactly the two
// decisions Taarib is better at than FairyGUI is — where the lines fall, and
// what the quads look like.
//
// THOSE TWO DECISIONS ARE TWO METHODS. `TextField.BuildLines` is where FairyGUI
// parses its markup, measures each character against its own font, breaks the
// text into lines and records the size the text wants to be. `OnPopulateMesh`
// is where those lines become vertices. The first is postfixed and the second
// prefixed — see NizamFairyGui.Rakkib for why the two differ — and both do the
// same lookup independently rather than one stashing a result for the other:
// the layout each needs is either two slices of a memory-mapped patch or one
// call into a layout cache that already holds the answer, so recomputing is
// cheaper than owning a per-instance cache whose invalidation would be a second
// thing to get right. Older revisions route the same work through
// `TextField.RenderContext` and a `BuildMesh` that writes into `NGraphics`
// directly; this file resolves the `OnPopulateMesh` seam and declines by name
// when a build predates it, because writing geometry into a container this file
// has not been able to inspect is how a takeover produces a corrupt mesh
// instead of no mesh.
//
// NGRAPHICS AND NTEXTURE ARE HOW THE MESH REACHES THE SCREEN. FairyGUI does not
// hand Unity a Material and a Mesh. Every display object owns an `NGraphics`,
// which owns an `NTexture` — FairyGUI's own wrapper over a Unity texture, with
// its own atlas rectangle, its own reference counting and its own material
// manager — and derives the material from it. So Taarib's atlas reaches the
// screen by wrapping the atlas page in one `NTexture`, built the first time it
// is needed and cached, and assigning it to the graphics of every text field
// this object draws. Writing correct vertices while leaving the game's own font
// texture bound would sample Taarib's glyph rectangles out of the wrong image,
// which renders as legible noise and reads as a corrupt atlas.
//
// THE MARKUP, AND WHY INLINE ELEMENTS MUST SURVIVE AS ATOMS. FairyGUI has its
// own HTML-ish dialect with its own parser: `HtmlParser` turns a string into
// `HtmlElement`s, and an element is not always text. `<img src='...'/>` is an
// inline image, and a `RichTextField` also carries input fields and links as
// elements that occupy space in the line. Mushtarak's `Nasq` lifts that dialect
// into byte-ranged style spans with `NawNasq.FairyGui` and is not
// re-implemented here — one parser per dialect, in the assembly both backends
// share, is the only arrangement in which the Mono and IL2CPP builds cannot
// come to disagree about what `<font color='#ff0000'>` means. What matters most
// in the lifting is that an inline element becomes an ATOM: one U+FFFC in the
// clean text, one style span carrying UslubDharra, opaque to shaping and to
// reordering. Let the bidirectional algorithm see the element's own characters
// and it will reorder them inside an Arabic paragraph, so `<img src='coin'/>`
// comes back out spelled backwards and FairyGUI's parser no longer recognises
// its own tag. As an atom it keeps one position, one width and one direction
// taken from its neighbours, `Nasij` emits no geometry for it and reports where
// it landed instead, and the picture inside it is drawn by FairyGUI from
// FairyGUI's own asset table — which is Decision 5, not a limitation: the image
// lives in the game's package and Taarib does not read the game's packages.

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
    /// وصل FairyGUI — every FairyGUI member this takeover touches, resolved
    /// once at startup and held as a strongly typed delegate.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Nothing here runs on a frame. A reflected property read allocates an
    /// argument array and boxes its result; performed once per text field per
    /// rebuild it is garbage a game's collector pays for, and the hitch is
    /// blamed on the game. A delegate built once costs a call and a cast.
    /// </para>
    /// <para>
    /// Two kinds of accessor are built here and they are built differently on
    /// purpose. A getter can be a plain open delegate over the accessor method,
    /// because a method returning <c>NTexture</c> binds to a delegate returning
    /// <see cref="object"/> under the CLR's own covariance rule. A setter
    /// cannot: a delegate taking <see cref="object"/> where the method takes
    /// <c>NTexture</c> is contravariance in the direction the runtime refuses,
    /// so those go through a compiled expression tree with the cast written
    /// into it. This assembly ships only to the Mono backend, which has a JIT
    /// and can compile one.
    /// </para>
    /// </remarks>
    public sealed class WaslFairyGui
    {
        private const BindingFlags Alamat =
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;

        private WaslFairyGui(
            Type nawHaql, Type nawMakhzan, MethodInfo hadafSutur, MethodInfo hadafMala)
        {
            NawHaql = nawHaql;
            NawMakhzan = nawMakhzan;
            HadafSutur = hadafSutur;
            HadafMala = hadafMala;

            QariNass = Rabt.Qari<string>(nawHaql, "text");
            QariGhani = Rabt.Qari<bool>(nawHaql, "html");
            QariArd = Rabt.Qari<float>(nawHaql, "width");
            QariIrtifa = Rabt.Qari<float>(nawHaql, "height");
            QariMuhadhahaRasiya = Rabt.Qari<int>(nawHaql, "verticalAlign");
            QariRusum = Rabt.Qari<object>(nawHaql, "graphics");
            QariTansiq = Rabt.Qari<object>(nawHaql, "textFormat");
            KatibArdNass = KatibHaql<int>(nawHaql, "_textWidth");
            KatibIrtifaNass = KatibHaql<int>(nawHaql, "_textHeight");

            Type? nawTansiq = Rabt.Naw("FairyGUI.TextFormat");
            QariHajm = Rabt.Qari<int>(nawTansiq, "size");
            QariLawn = Rabt.Qari<Color>(nawTansiq, "color");

            KatibLawha = KatibKhasiya(Rabt.Naw("FairyGUI.NGraphics"), "texture");
            BaniLawha = Bani(Rabt.Naw("FairyGUI.NTexture"));

            QariRuus = Qaima<Vector3>(nawMakhzan, "vertices");
            QariAlwan = Qaima<Color32>(nawMakhzan, "colors");
            QariMalamis = Qaima<Vector2>(nawMakhzan, "uvs");
            QariMuthallathat = Qaima<int>(nawMakhzan, "triangles");

            MakhzanRuus = Makhzan<Vector3>();
            MakhzanAlwan = Makhzan<Color32>();
            MakhzanMalamis = Makhzan<Vector2>();
            MakhzanMuthallathat = Makhzan<int>();

            AdadRuus = AdadKatib<Vector3>();
            AdadAlwan = AdadKatib<Color32>();
            AdadMalamis = AdadKatib<Vector2>();
            AdadMuthallathat = AdadKatib<int>();
        }

        /// <summary>The game's own <c>FairyGUI.TextField</c> type.</summary>
        public Type NawHaql { get; }

        /// <summary>The game's own <c>FairyGUI.VertexBuffer</c> type.</summary>
        public Type NawMakhzan { get; }

        /// <summary>
        /// <c>TextField.BuildLines</c> — where FairyGUI parses its markup,
        /// measures against its own font and decides where the lines fall.
        /// </summary>
        public MethodInfo HadafSutur { get; }

        /// <summary>
        /// <c>TextField.OnPopulateMesh</c> — where those lines become quads.
        /// </summary>
        public MethodInfo HadafMala { get; }

        /// <summary>Reads the field's text, exactly as the game set it.</summary>
        public Func<object, string>? QariNass { get; }

        /// <summary>
        /// Whether this field parses FairyGUI's markup. A field with it off
        /// draws <c>&lt;b&gt;</c> as three characters a player is meant to
        /// read, and lifting markup out of such a string would delete text from
        /// the screen.
        /// </summary>
        public Func<object, bool>? QariGhani { get; }

        /// <summary>The display object's width, in FairyGUI's own units.</summary>
        public Func<object, float>? QariArd { get; }

        /// <summary>The display object's height.</summary>
        public Func<object, float>? QariIrtifa { get; }

        /// <summary>
        /// Where the text block sits vertically in the field, as FairyGUI's own
        /// enumeration read through its underlying integer: top, middle,
        /// bottom.
        /// </summary>
        public Func<object, int>? QariMuhadhahaRasiya { get; }

        /// <summary>The field's <c>NGraphics</c>, which owns the texture and the mesh.</summary>
        public Func<object, object>? QariRusum { get; }

        /// <summary>The field's <c>TextFormat</c>, which carries the size and the colour.</summary>
        public Func<object, object>? QariTansiq { get; }

        /// <summary>The size the field draws at, in pixels, from its text format.</summary>
        public Func<object, int>? QariHajm { get; }

        /// <summary>The colour a glyph with no span of its own takes.</summary>
        public Func<object, Color>? QariLawn { get; }

        /// <summary>
        /// Records the measured width back onto the field, so the auto-size and
        /// layout code above it sees the size Arabic actually needs rather than
        /// the size FairyGUI's own measurement would have produced.
        /// </summary>
        public Action<object, int>? KatibArdNass { get; }

        /// <summary>Records the measured height back onto the field.</summary>
        public Action<object, int>? KatibIrtifaNass { get; }

        /// <summary>Binds an <c>NTexture</c> to an <c>NGraphics</c>.</summary>
        public Action<object, object>? KatibLawha { get; }

        /// <summary>Wraps a Unity texture in an <c>NTexture</c>.</summary>
        public Func<Texture, object>? BaniLawha { get; }

        /// <summary>The vertex list of a <c>VertexBuffer</c>.</summary>
        public Func<object, List<Vector3>>? QariRuus { get; }

        /// <summary>The colour list of a <c>VertexBuffer</c>.</summary>
        public Func<object, List<Color32>>? QariAlwan { get; }

        /// <summary>The texture coordinate list of a <c>VertexBuffer</c>.</summary>
        public Func<object, List<Vector2>>? QariMalamis { get; }

        /// <summary>The index list of a <c>VertexBuffer</c>.</summary>
        public Func<object, List<int>>? QariMuthallathat { get; }

        /// <summary>The backing array of a vertex list.</summary>
        public Func<List<Vector3>, Vector3[]>? MakhzanRuus { get; }

        /// <summary>The backing array of a colour list.</summary>
        public Func<List<Color32>, Color32[]>? MakhzanAlwan { get; }

        /// <summary>The backing array of a texture coordinate list.</summary>
        public Func<List<Vector2>, Vector2[]>? MakhzanMalamis { get; }

        /// <summary>The backing array of an index list.</summary>
        public Func<List<int>, int[]>? MakhzanMuthallathat { get; }

        /// <summary>Sets a vertex list's count after the geometry was written.</summary>
        public Action<List<Vector3>, int>? AdadRuus { get; }

        /// <summary>Sets a colour list's count.</summary>
        public Action<List<Color32>, int>? AdadAlwan { get; }

        /// <summary>Sets a texture coordinate list's count.</summary>
        public Action<List<Vector2>, int>? AdadMalamis { get; }

        /// <summary>Sets an index list's count.</summary>
        public Action<List<int>, int>? AdadMuthallathat { get; }

        /// <summary>
        /// Whether everything the two patch bodies need resolved. The
        /// write-back of the measured size is not part of the test: a field
        /// whose measurement cannot be recorded still draws correctly and only
        /// loses the reflow its container would have performed, which is a flaw
        /// rather than a wrong render.
        /// </summary>
        public bool Muakkad =>
            QariNass is not null
            && QariArd is not null
            && QariIrtifa is not null
            && QariRusum is not null
            && QariTansiq is not null
            && QariHajm is not null
            && QariLawn is not null
            && KatibLawha is not null
            && BaniLawha is not null
            && QariRuus is not null
            && QariAlwan is not null
            && QariMalamis is not null
            && QariMuthallathat is not null
            && MakhzanRuus is not null
            && MakhzanAlwan is not null
            && MakhzanMalamis is not null
            && MakhzanMuthallathat is not null
            && AdadRuus is not null
            && AdadAlwan is not null
            && AdadMalamis is not null
            && AdadMuthallathat is not null;

        /// <summary>
        /// Binds FairyGUI, or returns <c>null</c> when this game has none.
        /// </summary>
        /// <returns>
        /// The binding, or <c>null</c>. A <c>null</c> return with nothing in
        /// the log is the ordinary case: almost no game ships more than one or
        /// two of the text systems Taarib supports, and a missing one is not a
        /// failure to report.
        /// </returns>
        public static WaslFairyGui? Iqran()
        {
            Type? nawHaql = Rabt.Naw("FairyGUI.TextField");
            if (nawHaql is null)
            {
                return null;
            }

            Type? nawMakhzan = Rabt.Naw("FairyGUI.VertexBuffer");
            if (nawMakhzan is null)
            {
                Rabt.Ballagh(
                    "‏FairyGUI.TextField موجود بلا FairyGUI.VertexBuffer، أي أن هذا الإصدار يملأ نسيجه بمسار NGraphics القديم الذي لا يعالجه تعريب؛ تُترك نصوص FairyGUI للعبة. | "
                    + "FairyGUI.TextField is present but FairyGUI.VertexBuffer is not, so "
                    + "this build populates its mesh through the older NGraphics path that "
                    + "Taarib does not model; FairyGUI text is left to the game and every "
                    + "other text system in it is unaffected.");
                return null;
            }

            MethodInfo? hadafSutur = RabtZaid.HadafMuarraf(nawHaql, "BuildLines");
            MethodInfo? hadafMala = RabtZaid.HadafMuarraf(nawHaql, "OnPopulateMesh", nawMakhzan);
            if (hadafSutur is null || hadafMala is null)
            {
                Rabt.Ballagh(
                    "‏FairyGUI.TextField موجود لكن "
                    + (hadafSutur is null ? "BuildLines" : "OnPopulateMesh")
                    + " لم يُحلّ عليه؛ تُترك نصوص FairyGUI للعبة وتبقى بقية الأنظمة تعمل. | "
                    + "FairyGUI.TextField is present but "
                    + (hadafSutur is null ? "BuildLines()" : "OnPopulateMesh(VertexBuffer)")
                    + " did not resolve on it; FairyGUI text is left to the game and every "
                    + "other text system in it is unaffected.");
                return null;
            }

            WaslFairyGui wasl = new WaslFairyGui(nawHaql, nawMakhzan, hadafSutur, hadafMala);
            if (!wasl.Muakkad)
            {
                Rabt.Ballagh(
                    "‏FairyGUI موجود لكن " + wasl.Naqis()
                    + " لم يُحلّ في هذا الإصدار؛ تُترك نصوص FairyGUI للعبة وتبقى بقية الأنظمة تعمل. | "
                    + "FairyGUI is present but " + wasl.Naqis()
                    + " did not resolve in this build; FairyGUI text is left to the game and "
                    + "the rest of the takeover is unaffected.");
                return null;
            }
            return wasl;
        }

        /// <summary>
        /// Names the first member that failed to resolve, for the one line a
        /// refusal writes. A log that says "a member did not resolve" sends the
        /// next person to read all of them.
        /// </summary>
        /// <returns>The member's name, in FairyGUI's own spelling.</returns>
        public string Naqis()
        {
            if (QariNass is null) { return "TextField.text"; }
            if (QariArd is null) { return "DisplayObject.width"; }
            if (QariIrtifa is null) { return "DisplayObject.height"; }
            if (QariRusum is null) { return "DisplayObject.graphics"; }
            if (QariTansiq is null) { return "TextField.textFormat"; }
            if (QariHajm is null) { return "TextFormat.size"; }
            if (QariLawn is null) { return "TextFormat.color"; }
            if (BaniLawha is null) { return "NTexture(Texture)"; }
            if (KatibLawha is null) { return "NGraphics.texture"; }
            if (QariRuus is null || MakhzanRuus is null || AdadRuus is null)
            {
                return "VertexBuffer.vertices";
            }
            if (QariAlwan is null || MakhzanAlwan is null || AdadAlwan is null)
            {
                return "VertexBuffer.colors";
            }
            if (QariMalamis is null || MakhzanMalamis is null || AdadMalamis is null)
            {
                return "VertexBuffer.uvs";
            }
            return "VertexBuffer.triangles";
        }

        /// <summary>
        /// Whether a live object is a <c>TextField</c>, which is the test every
        /// patch body runs before touching anything.
        /// </summary>
        /// <param name="haql">The candidate object.</param>
        /// <returns>Whether it is one.</returns>
        public bool Yantami(object? haql)
        {
            return haql is not null && NawHaql.IsInstanceOfType(haql);
        }

        /// <summary>A <c>VertexBuffer</c> list, whether it is a field or a property.</summary>
        private static Func<object, List<TUnsur>>? Qaima<TUnsur>(Type naw, string ism)
        {
            return QariHaql<List<TUnsur>>(naw, ism) ?? Rabt.Qari<List<TUnsur>>(naw, ism);
        }

        /// <summary>
        /// A list's backing array. There is no public way to reach it in this
        /// framework — <c>CollectionsMarshal</c> arrived three versions later —
        /// and the alternative is one <c>Add</c> call per vertex, which is four
        /// calls per glyph on a path that already knows exactly how many
        /// vertices it is about to write.
        /// </summary>
        private static Func<List<TUnsur>, TUnsur[]>? Makhzan<TUnsur>()
        {
            FieldInfo? haql = Haql(typeof(List<TUnsur>), "_items", typeof(TUnsur[]));
            if (haql is null)
            {
                return null;
            }
            try
            {
                ParameterExpression hadaf = Expression.Parameter(typeof(List<TUnsur>), "hadaf");
                return Expression.Lambda<Func<List<TUnsur>, TUnsur[]>>(
                    Expression.Field(hadaf, haql), hadaf).Compile();
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Compiling a getter for List<T>._items failed", khata);
                return null;
            }
        }

        /// <summary>
        /// A list's count, written directly. This bypasses the version counter
        /// a live enumerator would check; nothing here enumerates, and the
        /// alternative is the per-vertex <c>Add</c> this exists to avoid.
        /// </summary>
        private static Action<List<TUnsur>, int>? AdadKatib<TUnsur>()
        {
            FieldInfo? haql = Haql(typeof(List<TUnsur>), "_size", typeof(int));
            if (haql is null)
            {
                return null;
            }
            try
            {
                ParameterExpression hadaf = Expression.Parameter(typeof(List<TUnsur>), "hadaf");
                ParameterExpression qeema = Expression.Parameter(typeof(int), "qeema");
                return Expression.Lambda<Action<List<TUnsur>, int>>(
                    Expression.Assign(Expression.Field(hadaf, haql), qeema),
                    hadaf, qeema).Compile();
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Compiling a setter for List<T>._size failed", khata);
                return null;
            }
        }

        /// <summary>
        /// A one-argument constructor as a delegate, for the <c>NTexture</c>
        /// this takeover wraps its atlas page in.
        /// </summary>
        private static Func<Texture, object>? Bani(Type? naw)
        {
            if (naw is null)
            {
                return null;
            }
            try
            {
                ConstructorInfo? bani = naw.GetConstructor(
                    BindingFlags.Instance | BindingFlags.Public,
                    null,
                    new[] { typeof(Texture) },
                    null);
                if (bani is null)
                {
                    return null;
                }
                ParameterExpression lawha = Expression.Parameter(typeof(Texture), "lawha");
                return Expression.Lambda<Func<Texture, object>>(
                    Expression.Convert(Expression.New(bani, lawha), typeof(object)),
                    lawha).Compile();
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Binding the NTexture constructor failed", khata);
                return null;
            }
        }

        /// <summary>
        /// A property setter whose value type cannot be named here, with the
        /// cast compiled into the delegate. An open delegate cannot express
        /// this: a setter taking <c>NTexture</c> does not bind to an
        /// <see cref="Action{T1,T2}"/> whose second parameter is
        /// <see cref="object"/>, because that is contravariance in the
        /// direction the runtime refuses.
        /// </summary>
        private static Action<object, object>? KatibKhasiya(Type? naw, string ism)
        {
            MethodInfo? katib = RabtZaid.HadafKhasiya(naw, ism, katib: true);
            if (katib is null || katib.IsStatic || katib.GetParameters().Length != 1)
            {
                return null;
            }
            try
            {
                Type nawQeema = katib.GetParameters()[0].ParameterType;
                ParameterExpression hadaf = Expression.Parameter(typeof(object), "hadaf");
                ParameterExpression qeema = Expression.Parameter(typeof(object), "qeema");
                return Expression.Lambda<Action<object, object>>(
                    Expression.Call(
                        Expression.Convert(hadaf, katib.DeclaringType!),
                        katib,
                        Expression.Convert(qeema, nawQeema)),
                    hadaf, qeema).Compile();
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Compiling a setter for NGraphics.texture failed", khata);
                return null;
            }
        }

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
    /// نظام FairyGUI — the FairyGUI takeover: two patches, one lookup rule, and
    /// a refusal to guess.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The two patch targets.</b> A postfix on <c>TextField.BuildLines</c>,
    /// which overwrites the size FairyGUI just measured against a font with no
    /// Arabic in it with the size Taarib's layout actually needs; and a prefix
    /// on <c>TextField.OnPopulateMesh</c>, which binds Taarib's atlas to the
    /// field's <c>NGraphics</c> and writes the quads into the
    /// <c>VertexBuffer</c> FairyGUI already owns. The prefix returns
    /// <c>false</c> only when it did its own work and the postfix writes
    /// nothing unless it did, so a field Taarib declines is a field FairyGUI
    /// renders exactly as it always did.
    /// </para>
    /// <para>
    /// <b>Why skipping FairyGUI's measurement is not an optimisation.</b>
    /// FairyGUI measures each character against the font the field was authored
    /// with, which in a game that was never built for Arabic contains no Arabic
    /// at all: every letter measures as a missing-glyph box, the line breaks
    /// land between the wrong words, and the field reports a width its
    /// container then lays itself out against. Recording Taarib's own
    /// measurement is what makes the container above the field agree with the
    /// text inside it.
    /// </para>
    /// <para>
    /// <b>The lookup never guesses.</b> The field's text is hashed with
    /// <see cref="Ruqaa.MiftahMinNass(string)"/> and looked up in the installed
    /// patch. A miss means this field is drawing something the patch does not
    /// cover — a score, a player's own name, a string the compiler never saw —
    /// and both interceptions stand aside. Nothing here reaches for a nearby
    /// string, a normalised form or a nearest size.
    /// </para>
    /// <para>
    /// <b>Inline elements.</b> A translated string's atoms — an
    /// <c>&lt;img&gt;</c>, an input placeholder, anything the dialect marks
    /// opaque — reach <see cref="Nasij"/> as spans carrying
    /// <see cref="Alamat.UslubDharra"/>, and it emits no geometry for them and
    /// reports where each one landed. Those rectangles are exposed through
    /// <see cref="Dharrat"/> in the field's own local space, for the caller that
    /// positions FairyGUI's <c>HtmlElement</c> objects over them. Drawing them
    /// here is not an option and not an omission: the picture lives in the
    /// game's own package, and Taarib does not read the game's packages.
    /// </para>
    /// <para>
    /// <b>What allocates.</b> <see cref="Ibda"/> allocates: it resolves types,
    /// compiles accessors and installs patches, once, at load. The first draw
    /// after installation builds one <c>NTexture</c> per atlas. Everything
    /// after that allocates only when a buffer grows — the shared geometry
    /// buffers reaching the longest string seen so far, and a
    /// <c>VertexBuffer</c> list's capacity being raised for a field with more
    /// glyphs than it has held before, which is the same growth
    /// <c>List&lt;T&gt;.Add</c> would have performed itself.
    /// </para>
    /// </remarks>
    public sealed class NizamFairyGui : IDisposable
    {
        private static NizamFairyGui? hali;

        private readonly WaslFairyGui wasl;
        private readonly MasdarAshkal masdar;
        private readonly Ruqaa ruqaa;
        private readonly Takhtit? takhtit;
        private readonly MaqbadSiyaq? siyaq;
        private readonly MaqbadSilsila? silsila;
        private readonly MakhzanRusum makhzan;
        private readonly NassMuhaddar muhaddar;
        private readonly List<MethodInfo> hidaf;
        private readonly object?[] lawhat;

        private DharraMawduaa[] dharrat;
        private int adadDharrat;
        private bool amil;
        private bool hars;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        private NizamFairyGui(
            Harmony harmoni,
            WaslFairyGui wasl,
            MasdarAshkal masdar,
            Ruqaa ruqaa,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit)
        {
            Harmoni = harmoni;
            this.wasl = wasl;
            this.masdar = masdar;
            this.ruqaa = ruqaa;
            this.siyaq = siyaq;
            this.silsila = silsila;
            this.takhtit = takhtit;
            makhzan = new MakhzanRusum();
            muhaddar = new NassMuhaddar();
            hidaf = new List<MethodInfo>(2);
            lawhat = new object?[2];
            dharrat = new DharraMawduaa[8];
            amil = true;
        }

        /// <summary>The live FairyGUI takeover, or <c>null</c> when none is installed.</summary>
        public static NizamFairyGui? Hali => hali;

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <summary>The Harmony instance the patches were installed with.</summary>
        public Harmony Harmoni { get; }

        /// <summary>
        /// Where the inline atoms of the most recently drawn field landed, in
        /// that field's own local space. Valid until the next field is drawn,
        /// which is the same lifetime the <c>VertexBuffer</c> they were written
        /// beside has.
        /// </summary>
        public ReadOnlySpan<DharraMawduaa> Dharrat =>
            new ReadOnlySpan<DharraMawduaa>(dharrat, 0, adadDharrat);

        /// <summary>
        /// Discovers FairyGUI, installs the patches, and returns the takeover.
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
        /// field's layout overwrite the glyphs another is halfway through
        /// turning into triangles.
        /// </param>
        /// <returns>
        /// The takeover, or <c>null</c> when this game has no FairyGUI, which
        /// is not an error.
        /// </returns>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="harmoni"/>, <paramref name="ruqaa"/> or
        /// <paramref name="masdar"/> is null.
        /// </exception>
        public static NizamFairyGui? Ibda(
            Harmony harmoni,
            Ruqaa ruqaa,
            MasdarAshkal masdar,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit)
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

            WaslFairyGui? wasl = WaslFairyGui.Iqran();
            if (wasl is null)
            {
                return null;
            }

            NizamFairyGui nizam = new NizamFairyGui(
                harmoni, wasl, masdar, ruqaa, siyaq, silsila, takhtit);
            hali = nizam;

            bool mala = nizam.Rakkib(
                wasl.HadafMala, typeof(TarqeeMalaFairy), nameof(TarqeeMalaFairy.Sabiq),
                "TextField.OnPopulateMesh", sabiq: true);
            nizam.Rakkib(
                wasl.HadafSutur, typeof(TarqeeSuturFairy), nameof(TarqeeSuturFairy.Baad),
                "TextField.BuildLines", sabiq: false);

            if (!mala)
            {
                Rabt.Ballagh(
                    "لم يُركَّب ترقيع TextField.OnPopulateMesh؛ نصوص FairyGUI تبقى بلغة اللعبة وبقية الأنظمة تعمل. | "
                    + "The TextField.OnPopulateMesh patch could not be installed; FairyGUI "
                    + "text stays in the game's original language and every other text "
                    + "system keeps working.");
                nizam.Dispose();
                return null;
            }
            return nizam;
        }

        /// <summary>
        /// Measures one field through Taarib and records the result on it.
        /// </summary>
        /// <param name="haql">The text field being measured.</param>
        /// <returns>
        /// Whether Taarib owns this string and has recorded its size over the
        /// one FairyGUI's own line building just wrote.
        /// </returns>
        public bool Yaqis(object? haql)
        {
            if (!amil || hars || haql is null || !masdar.Amil || !wasl.Yantami(haql))
            {
                return false;
            }
            hars = true;
            try
            {
                if (!Hall(haql, out int fahras, out float hajm))
                {
                    return false;
                }
                if (!Takhtitat(
                    haql, fahras, hajm,
                    out ReadOnlySpan<TaaribHarf> huruf, out _, out _,
                    out float ardTakhtit, out float irtifaTakhtit, out _))
                {
                    return false;
                }
                if (huruf.IsEmpty)
                {
                    return false;
                }
                wasl.KatibArdNass?.Invoke(haql, Sahih(ardTakhtit));
                wasl.KatibIrtifaNass?.Invoke(haql, Sahih(irtifaTakhtit));
                return true;
            }
            catch (KhataTaarib khata)
            {
                Rabt.Ballagh("FairyGUI: measuring one string failed", khata);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("measuring a FairyGUI text field threw", khata);
                return false;
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// The whole interception: read the string, look it up, and either
        /// write Taarib's geometry into the vertex buffer or hand the field
        /// straight back to FairyGUI.
        /// </summary>
        /// <param name="haql">The text field being asked to draw.</param>
        /// <param name="makhzanKhaam">The vertex buffer it was handed.</param>
        /// <returns>
        /// Whether Taarib filled it. <c>false</c> means the buffer is exactly
        /// as it was and FairyGUI's own generation must run.
        /// </returns>
        public bool Yarsum(object? haql, object? makhzanKhaam)
        {
            if (!amil || hars || haql is null || makhzanKhaam is null || !masdar.Amil
                || !wasl.Yantami(haql) || !wasl.NawMakhzan.IsInstanceOfType(makhzanKhaam))
            {
                return false;
            }
            hars = true;
            try
            {
                return Rassim(haql, makhzanKhaam);
            }
            catch (KhataTaarib khata)
            {
                Rabt.Ballagh("FairyGUI: drawing one string failed", khata);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("drawing a FairyGUI text field threw", khata);
                return false;
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// Switches the takeover off, naming the reason once. Everything after
        /// this declines, which leaves FairyGUI drawing exactly as it did
        /// before the plugin loaded.
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
            Rabt.Ballagh(
                "FairyGUI: " + sabab + "; FairyGUI text is returned to the game's own "
                + "renderer and every other text system is unaffected", khata);
        }

        /// <summary>
        /// Removes the patches and stops drawing. The fields already on screen
        /// are FairyGUI's to rebuild; the next time one is marked dirty its own
        /// line building and mesh population run untouched.
        /// </summary>
        public void Dispose()
        {
            amil = false;
            for (int i = 0; i < hidaf.Count; i++)
            {
                try
                {
                    Harmoni.Unpatch(hidaf[i], HarmonyPatchType.All, Harmoni.Id);
                }
                catch (Exception khata)
                {
                    Rabt.Ballagh("Removing the patch on " + hidaf[i].Name + " failed", khata);
                }
            }
            hidaf.Clear();
            lawhat[0] = null;
            lawhat[1] = null;
            adadDharrat = 0;
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
        }

        /// <summary>The whole draw, inside the re-entrancy guard.</summary>
        private bool Rassim(object haql, object makhzanKhaam)
        {
            if (!Hall(haql, out int fahras, out float hajm))
            {
                return false;
            }

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            if (!Takhtitat(
                haql, fahras, hajm, out huruf, out sutur, out nitaqat,
                out _, out float irtifaTakhtit, out float hajmFili))
            {
                return false;
            }
            if (huruf.IsEmpty || !(hajmFili > 0f))
            {
                return false;
            }

            bool minRuqaa = MinRuqaa(fahras, hajm);
            float hajmLawha = masdar.HajmLawha(hajmFili);
            bool tathbit = masdar.Namat == NamatLawha.Taghtiya;
            if (!minRuqaa && !masdar.Aqim(huruf, hajmLawha, tathbit))
            {
                return false;
            }

            object? rusum = wasl.QariRusum!(haql);
            object? lawha = Lawha(minRuqaa);
            if (rusum is null || lawha is null)
            {
                return false;
            }

            HayyizRasm hayyiz = default;
            hayyiz.Ard = wasl.QariArd!(haql);
            hayyiz.Irtifa = wasl.QariIrtifa!(haql);

            // FairyGUI's display space has its origin at the field's top-left
            // corner with Y measured downward, and its own vertex helper
            // negates Y on the way into the mesh. Writing into the buffer
            // directly means performing that flip here instead, and a pivot of
            // (0, 1) is exactly it: the left edge lands on the origin, the top
            // edge lands on the origin, and the block hangs down and to the
            // right of it — which is what Taarib's layout space already means.
            hayyiz.MihwarS = 0f;
            hayyiz.MihwarA = 1f;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;
            hayyiz.Muhadhaha = Rasiya(haql);

            TalabNasij talab = default;
            talab.Huruf = huruf;
            talab.Sutur = sutur;
            talab.Nitaqat = nitaqat;
            talab.Hayyiz = hayyiz;
            talab.Lawn = LawnRasm.Min(LawnMuazzam(Lawn(haql)));
            talab.Hajm = hajmFili;
            talab.HajmLawha = hajmLawha;
            talab.Safha = 0;

            // A coverage atlas holds one bitmap per size with the pen's
            // fractional position already folded into its coverage, and a
            // FairyGUI field maps one layout pixel to one display unit, so the
            // quad must land on the integer grid or that fraction is applied
            // twice and the line blurs. A distance-field atlas holds one bitmap
            // for every size, so nothing is snapped.
            talab.Tathbit = tathbit;

            if (!Aktub(makhzanKhaam, in talab, minRuqaa, huruf.Length))
            {
                return false;
            }

            // Bound after the geometry is written rather than before it: an
            // atlas swapped onto a field whose mesh write then failed would
            // leave FairyGUI's own glyphs sampling Taarib's atlas.
            wasl.KatibLawha!(rusum, lawha);
            return true;
        }

        /// <summary>
        /// Writes the mesh into the four lists the vertex buffer already owns.
        /// </summary>
        private bool Aktub(
            object makhzanKhaam, in TalabNasij talab, bool minRuqaa, int adadHuruf)
        {
            List<Vector3> qaimatRuus = wasl.QariRuus!(makhzanKhaam);
            List<Color32> qaimatAlwan = wasl.QariAlwan!(makhzanKhaam);
            List<Vector2> qaimatMalamis = wasl.QariMalamis!(makhzanKhaam);
            List<int> qaimatMuthallathat = wasl.QariMuthallathat!(makhzanKhaam);
            if (qaimatRuus is null || qaimatAlwan is null
                || qaimatMalamis is null || qaimatMuthallathat is null)
            {
                return false;
            }

            int bidaya = qaimatRuus.Count;
            if (bidaya != qaimatAlwan.Count || bidaya != qaimatMalamis.Count)
            {
                // Three parallel lists that disagree about their own length is
                // not a state this file can write into safely, and it is not
                // one Taarib caused. Decline, and let FairyGUI fill.
                return false;
            }

            int matlubRuus = adadHuruf * Nasij.RuusLiShakl;
            int matlubFahras = adadHuruf * Nasij.FahrasLiShakl;
            int bidayatFahras = qaimatMuthallathat.Count;
            Wassi(qaimatRuus, bidaya + matlubRuus);
            Wassi(qaimatAlwan, bidaya + matlubRuus);
            Wassi(qaimatMalamis, bidaya + matlubRuus);
            Wassi(qaimatMuthallathat, bidayatFahras + matlubFahras);

            // Read after the growth, never before: raising a capacity replaces
            // the backing array, and a span over the old one would write into
            // memory the list no longer refers to.
            Vector3[] arrRuus = wasl.MakhzanRuus!(qaimatRuus);
            Color32[] arrAlwan = wasl.MakhzanAlwan!(qaimatAlwan);
            Vector2[] arrMalamis = wasl.MakhzanMalamis!(qaimatMalamis);
            int[] arrMuthallathat = wasl.MakhzanMuthallathat!(qaimatMuthallathat);

            if (dharrat.Length < adadHuruf)
            {
                dharrat = new DharraMawduaa[adadHuruf];
            }

            MakhzanNasij hadaf = default;
            hadaf.Ruus = MemoryMarshal.Cast<Vector3, NuqtaRasm>(
                new Span<Vector3>(arrRuus, bidaya, matlubRuus));
            hadaf.Alwan = MemoryMarshal.Cast<Color32, LawnRasm>(
                new Span<Color32>(arrAlwan, bidaya, matlubRuus));
            hadaf.Malamis = MemoryMarshal.Cast<Vector2, NuqtaMulmas>(
                new Span<Vector2>(arrMalamis, bidaya, matlubRuus));
            hadaf.Muthallathat = new Span<int>(arrMuthallathat, bidayatFahras, matlubFahras);
            hadaf.Dharrat = dharrat.AsSpan();

            NatijaNasij natija = Nasij.Ibni(in talab, masdar.Khareeta(minRuqaa), hadaf);
            if (!natija.Kafa)
            {
                // Every span above was sized from the same upper bound Nasij
                // demands, so a shortfall here means the two disagree about
                // that bound rather than that the field is long.
                Rabt.Ballagh(
                    "FairyGUI: the mesh buffer refused a build sized to Nasij's own upper "
                    + "bound; this field is left to FairyGUI and the takeover stays "
                    + "installed.");
                return false;
            }
            if (natija.AdadAshkal == 0)
            {
                return false;
            }
            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            // Nasij writes indices relative to the start of the span it was
            // given. FairyGUI's buffer may already carry another object's
            // geometry, so every index moves by that object's vertex count —
            // without which this field's triangles would name its neighbour's
            // vertices and draw a fan of stretched glyphs across the screen.
            if (bidaya != 0)
            {
                for (int i = 0; i < natija.AdadMuthallathat; i++)
                {
                    arrMuthallathat[bidayatFahras + i] += bidaya;
                }
            }

            wasl.AdadRuus!(qaimatRuus, bidaya + natija.AdadRuus);
            wasl.AdadAlwan!(qaimatAlwan, bidaya + natija.AdadRuus);
            wasl.AdadMalamis!(qaimatMalamis, bidaya + natija.AdadRuus);
            wasl.AdadMuthallathat!(
                qaimatMuthallathat, bidayatFahras + natija.AdadMuthallathat);

            adadDharrat = natija.AdadDharrat < dharrat.Length
                ? natija.AdadDharrat
                : dharrat.Length;
            return true;
        }

        /// <summary>
        /// Finds the field's string in the patch and reads the size it draws
        /// at.
        /// </summary>
        private bool Hall(object haql, out int fahras, out float hajm)
        {
            fahras = -1;
            hajm = 0f;

            string? khaam = wasl.QariNass!(haql);
            if (string.IsNullOrEmpty(khaam))
            {
                return false;
            }

            object? tansiq = wasl.QariTansiq!(haql);
            if (tansiq is null)
            {
                return false;
            }
            hajm = wasl.QariHajm!(tansiq);
            if (!(hajm > 0f))
            {
                return false;
            }

            // MiftahMinNass encodes the string to UTF-8 to hash it: under 512
            // bytes that encode is a stackalloc, so the lookup that decides
            // whether Taarib owns a field allocates nothing at all — which
            // matters because it runs for every field FairyGUI rebuilds,
            // including all the ones the patch does not cover.
            ulong miftah = Ruqaa.MiftahMinNass(khaam!);
            fahras = ruqaa.JidNass(miftah);
            if (fahras < 0)
            {
                Rabt.Fawt(khaam!, miftah);
                return false;
            }
            return true;
        }

        /// <summary>
        /// The layout for one string at one size: the compiler's, when it
        /// measured that exact quarter-pixel size, and a runtime layout through
        /// Jisr otherwise.
        /// </summary>
        private bool Takhtitat(
            object haql,
            int fahras,
            float hajm,
            out ReadOnlySpan<TaaribHarf> huruf,
            out ReadOnlySpan<TaaribSatr> sutur,
            out ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            out float ardTakhtit,
            out float irtifaTakhtit,
            out float hajmFili)
        {
            // The exact quarter-pixel size, never the nearest. Drawing a layout
            // measured at one size into a field the game sized at another is
            // how text that fitted in the compiler's measurement overflows on a
            // player's screen — and the compiler's overflow report, which said
            // it fitted, would be wrong.
            ushort hajmRubi = Nasij.HajmRubi(hajm);
            if (ruqaa.JidTakhtit(fahras, hajmRubi, out MadkhalTakhtit madkhal))
            {
                huruf = ruqaa.HurufTakhtit(in madkhal);
                sutur = ruqaa.SuturTakhtit(in madkhal);
                nitaqat = makhzan.Hawwil(ruqaa.NitaqatNass(fahras));
                ardTakhtit = madkhal.Ard;
                irtifaTakhtit = madkhal.Irtifa;
                hajmFili = madkhal.Hajm;
                return true;
            }

            huruf = ReadOnlySpan<TaaribHarf>.Empty;
            sutur = ReadOnlySpan<TaaribSatr>.Empty;
            nitaqat = ReadOnlySpan<TaaribNitaqUslub>.Empty;
            ardTakhtit = 0f;
            irtifaTakhtit = 0f;
            hajmFili = hajm;

            Takhtit? buffer = takhtit;
            MaqbadSiyaq? qab = siyaq;
            MaqbadSilsila? chain = silsila;
            if (buffer is null || qab is null || chain is null)
            {
                BallighTakhtit();
                return false;
            }

            Func<object, bool>? qariGhani = wasl.QariGhani;
            bool ghani = qariGhani is null || qariGhani(haql);
            NawNasq lahja = ghani ? NawNasq.FairyGui : NawNasq.Bila;
            if (!muhaddar.Hayyi(
                ruqaa, fahras, makhzan, lahja, hajm,
                out ReadOnlySpan<char> nass, out ReadOnlySpan<TaaribNitaqUslub> nitaqatNass))
            {
                return false;
            }

            KhiyaratTakhtit khiyarat;
            float ardTalab = wasl.QariArd!(haql);
            float irtifaTalab = wasl.QariIrtifa!(haql);
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
                // No constraint row for this string. The field's own rectangle
                // is the honest fallback for the width, and every policy takes
                // its documented default rather than a guess at what the game
                // meant.
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
                // takeover stays installed and the next field still draws.
                Rabt.Ballagh("FairyGUI: " + khata.Arabi + " | " + khata.Injilizi);
                return false;
            }

            huruf = buffer.Huruf;
            sutur = buffer.Sutur;
            nitaqat = nitaqatNass;
            ardTakhtit = buffer.Ard;
            irtifaTakhtit = buffer.Irtifa;
            hajmFili = buffer.Hajm;
            return !huruf.IsEmpty;
        }

        /// <summary>
        /// Whether one string at one size is served by the patch's own compiled
        /// atlas rather than by glyphs rasterized at run time. Asked separately
        /// from the layout because the two prefixes take different paths to the
        /// same answer and the glyph map must match the layout they used.
        /// </summary>
        private bool MinRuqaa(int fahras, float hajm)
        {
            return ruqaa.JidTakhtit(fahras, Nasij.HajmRubi(hajm), out _);
        }

        /// <summary>
        /// Taarib's atlas page wrapped in an <c>NTexture</c>, built once per
        /// atlas and cached. FairyGUI derives a material from the wrapper and
        /// reference counts it, so building a fresh one per draw would churn
        /// materials every frame a field rebuilt.
        /// </summary>
        /// <remarks>
        /// Only the atlas the quads were built against. A field laid out from
        /// the patch indexes the patch's own atlas and one laid out at run time
        /// indexes this session's; the two are different pictures with different
        /// glyphs at different coordinates, so wrapping whichever page happened
        /// to exist would paint the field as solid blocks — and, because the
        /// wrapper is cached under the slot it was asked for, would keep doing
        /// it for the rest of the session.
        /// </remarks>
        private object? Lawha(bool minRuqaa)
        {
            int fahras = minRuqaa ? 0 : 1;
            object? mawjud = lawhat[fahras];
            if (mawjud is not null)
            {
                return mawjud;
            }
            Texture2D? asl = masdar.LawhatSafha(minRuqaa, 0);
            if (asl == null)
            {
                return null;
            }
            try
            {
                mawjud = wasl.BaniLawha!(asl);
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(
                    "FairyGUI: constructing an NTexture over Taarib's atlas failed; FairyGUI "
                    + "text is left to the game and every other text system is unaffected",
                    khata);
                return null;
            }
            lawhat[fahras] = mawjud;
            return mawjud;
        }

        /// <summary>The colour the field's text format carries.</summary>
        private Color Lawn(object haql)
        {
            object? tansiq = wasl.QariTansiq!(haql);
            return tansiq is null ? Color.white : wasl.QariLawn!(tansiq);
        }

        /// <summary>
        /// Where the block sits vertically inside the field, from FairyGUI's
        /// own vertical alignment. An unrecognised value resolves to the top,
        /// which is what a field with no alignment set draws.
        /// </summary>
        private MuhadhahaRasiya Rasiya(object haql)
        {
            Func<object, int>? qari = wasl.QariMuhadhahaRasiya;
            if (qari is null)
            {
                return MuhadhahaRasiya.Ala;
            }
            switch (qari(haql))
            {
                case 1: return MuhadhahaRasiya.Wasat;
                case 2: return MuhadhahaRasiya.Asfal;
                default: return MuhadhahaRasiya.Ala;
            }
        }

        /// <summary>Installs one interception and records its target for removal.</summary>
        /// <remarks>
        /// <paramref name="sabiq"/> is not a style choice. FairyGUI clears
        /// <c>_textChanged</c> as the first statement of <c>BuildLines</c> and
        /// marks its graphics' mesh dirty as the last, so a prefix that skipped
        /// the method left the field permanently dirty and never let the mesh
        /// population that draws Taarib's glyphs run at all. Measuring after the
        /// engine has measured costs one pass the player never sees and leaves
        /// every flag FairyGUI owns in the state FairyGUI expects. The mesh
        /// population is the opposite case and must stay a prefix: its output is
        /// the buffer it was handed, and letting the engine fill it first would
        /// leave FairyGUI's own quads in front of Taarib's.
        /// </remarks>
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
                    Harmoni.Patch(hadaf, prefix: tarqee);
                }
                else
                {
                    Harmoni.Patch(hadaf, postfix: tarqee);
                }
                hidaf.Add(hadaf);
                return true;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(
                    "Patching " + wasf + " failed; that one interception is off and the rest "
                    + "of the FairyGUI takeover continues", khata);
                return false;
            }
        }

        /// <summary>
        /// Reports a field whose glyphs need more than one atlas page, once.
        /// One <c>NGraphics</c> binds one <c>NTexture</c>, so the glyphs on the
        /// other pages cannot be drawn from here; the fix is a compiler-side
        /// page budget and not anything a player can act on.
        /// </summary>
        private void BallighSafahat()
        {
            if (ballaghSafahat)
            {
                return;
            }
            ballaghSafahat = true;
            Rabt.Ballagh(
                "حروف بعض نصوص FairyGUI موزَّعة على أكثر من صفحة أطلس، وكائن NGraphics يربط لوحة واحدة؛ تُرسم حروف الصفحة الأولى وتغيب البقية. | "
                + "Some FairyGUI fields have glyphs on more than one atlas page and one "
                + "NGraphics binds one texture; the glyphs on the first page are drawn and "
                + "the rest are missing from the fields that need them.");
        }

        /// <summary>
        /// Reports, once, that runtime layout is unavailable, so a string the
        /// compiler did not measure at the size a field draws at stays in the
        /// game's own language.
        /// </summary>
        private void BallighTakhtit()
        {
            if (ballaghTakhtit)
            {
                return;
            }
            ballaghTakhtit = true;
            Rabt.Ballagh(
                "لا يوجد تخطيط زمن تشغيل لنصوص FairyGUI، فالنصوص التي لم يقِسها المترجم بهذا الحجم تبقى بلغة اللعبة. | "
                + "No runtime layout is available to the FairyGUI takeover, so a string the "
                + "compiler did not measure at the size a field draws at stays in the game's "
                + "original language.");
        }

        /// <summary>
        /// Raises a list's capacity to hold a given count, preserving what is
        /// already in it. This is the one allocation the fill path can make and
        /// it is the same one <see cref="List{T}.Add"/> would have made.
        /// </summary>
        private static void Wassi<TUnsur>(List<TUnsur> qaima, int lazim)
        {
            if (qaima.Capacity >= lazim)
            {
                return;
            }
            int siaa = qaima.Capacity < 4 ? 4 : qaima.Capacity;
            while (siaa < lazim)
            {
                siaa *= 2;
            }
            qaima.Capacity = siaa;
        }

        /// <summary>A measured length as the integer FairyGUI records.</summary>
        private static int Sahih(float qeema)
        {
            return qeema > 0f ? (int)Math.Ceiling((double)qeema) : 0;
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
    }

    /// <summary>
    /// ترقيع السطور — the postfix on <c>TextField.BuildLines</c>.
    /// </summary>
    /// <remarks>
    /// FairyGUI decides here how wide and how tall the text wants to be, by
    /// measuring each character against the field's own font. For a string
    /// Taarib owns that measurement is meaningless — the font has no Arabic in
    /// it — and worse than meaningless, because the container above the field
    /// lays itself out against the answer. Taarib measures and overwrites the
    /// two numbers the engine just recorded. It runs after that pass rather
    /// than in place of it because the method clears the field's own changed
    /// flag and marks its graphics dirty, and a field that never reports itself
    /// measured never reaches the mesh population where the Arabic is drawn.
    /// </remarks>
    public static class TarqeeSuturFairy
    {
        /// <summary>
        /// Measures through Taarib when the patch covers the field's string,
        /// and leaves FairyGUI's own answer standing otherwise.
        /// </summary>
        /// <param name="__instance">The text field, injected by Harmony.</param>
        public static void Baad(object __instance)
        {
            NizamFairyGui.Hali?.Yaqis(__instance);
        }
    }

    /// <summary>
    /// ترقيع ملء النسيج — the prefix on <c>TextField.OnPopulateMesh</c>.
    /// </summary>
    /// <remarks>
    /// The buffer parameter is declared as <see cref="object"/> because
    /// <c>FairyGUI.VertexBuffer</c> cannot be named at compile time in an
    /// assembly that must also load in a game without FairyGUI. Harmony binds a
    /// patch parameter positionally through <c>__0</c> and permits a reference
    /// type to be received as <see cref="object"/>, so the widening costs
    /// nothing at run time.
    /// </remarks>
    public static class TarqeeMalaFairy
    {
        /// <summary>
        /// Fills the vertex buffer through Taarib when the patch covers the
        /// field's string.
        /// </summary>
        /// <param name="__instance">The text field, injected by Harmony.</param>
        /// <param name="__0">The vertex buffer it was handed.</param>
        /// <returns>
        /// <c>false</c> to skip FairyGUI's own generation, which is what stops
        /// a pipeline with no Arabic OpenType layout in it from choosing
        /// glyphs; <c>true</c> to let it run untouched.
        /// </returns>
        public static bool Sabiq(object __instance, object __0)
        {
            NizamFairyGui? nizam = NizamFairyGui.Hali;
            return nizam is null || !nizam.Yarsum(__instance, __0);
        }
    }
}
