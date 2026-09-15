// نظام تكست ميش برو — the TextMeshPro takeover.
//
// TMP is the text system most modern Unity games ship, and it is the hardest
// one to take over honestly. The reason is not the API surface; it is that TMP
// is not a renderer with a layout step bolted on, it is a shaper. It walks the
// string, looks each character up in its own TMP_FontAsset glyph table, applies
// its own kerning pairs, and writes a quad per character into TMP_MeshInfo.
// That pipeline has no OpenType layout in it at all: no GSUB, so no init/medi/
// fina/isol substitution and no lam-alef ligature; no GPOS mark attachment, so
// tashkeel sits on the baseline instead of on its letter; no Unicode
// bidirectional algorithm, so a mixed sentence comes out in logical order.
//
// WHY TMP'S PIPELINE IS BYPASSED RATHER THAN CORRECTED. The tempting shape is
// a postfix: let TMP generate, then move the quads into the right places. It
// cannot work, and the reason is worth stating once because every previous
// attempt at Arabic in Unity has tried it. TMP has already chosen glyphs by
// then. An Arabic word shaped correctly is not a permutation of the glyphs TMP
// chose — it is a different set of glyphs: four contextual forms per letter
// where TMP found one, one ligature glyph where TMP found two letters, a mark
// glyph positioned by an anchor pair that TMP's font asset does not carry. A
// postfix can reorder wrong glyphs into a wrong word. Correcting the output of
// a pipeline that answered a different question is correcting a wrong answer.
//
// So the prefix on GenerateTextMesh returns false, TMP's generation never runs
// for a string Taarib owns, and the geometry comes from Jisr's layout through
// Mushtarak's Nasij instead. TMP_TextInfo, characterInfo and meshInfo are left
// exactly as TMP last wrote them; nothing here reads or writes them, and the
// comment on NizamTmp says why that is safe and where it is not.
//
// DECISION 5 IN THIS FILE, MECHANICALLY. Nothing below touches fontAsset,
// fontSharedMaterial, the SDF atlas texture, a sprite asset, or any serialized
// asset of the game's. Taarib uploads its own atlas pages into its own
// Texture2D, builds its own Material from its own shader, and assigns that
// material to the renderer for the draw it owns. The game's font asset is not
// read for metrics, not read for a fallback, and not read for a shader. Search
// this file for "fontAsset" and the only hits are in comments.
//
// THE COMPILE-TIME RULE. This assembly has no reference to TextMeshPro and must
// not acquire one: most games do not ship it, and a hard reference would make
// Taarib.Unity.Mono fail to load in all of them. Every TMP type, method,
// property and patch target below is resolved by name through reflection, once,
// at startup, and cached as a strongly typed delegate. Unity's own core types
// — Mesh, Material, Texture2D, CanvasRenderer, MeshFilter, MeshRenderer,
// Vector3, Color32 — are named directly, because those come from
// UnityEngine.Modules and exist in every engine version this plugin runs in.

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using HarmonyLib;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mono.Suluk;
using Taarib.Unity.Mushtarak;
using UnityEngine;
using UnityEngine.Rendering;

namespace Taarib.Unity.Mono.Anzimat
{
    /// <summary>
    /// ربط زائد — the reflection bindings this namespace needs beyond the four
    /// <see cref="Rabt"/> already provides: methods that take arguments, and
    /// the <see cref="MethodInfo"/> of a patch target.
    /// </summary>
    /// <remarks>
    /// <para>
    /// It lives in this file rather than in one of its own for the same reason
    /// <see cref="Rabt"/> lives in <c>HuqulIdkhal.cs</c>: TextMeshPro is by far
    /// its heaviest user, and independence between the takeovers in this
    /// namespace is a runtime property rather than a file-count property. A
    /// binding that fails returns <c>null</c>, the takeover that asked for it
    /// switches itself off with a named reason, and the other text systems in
    /// the game never learn that anything happened.
    /// </para>
    /// <para>
    /// Every member here runs at startup and never on a frame. That is the
    /// whole reason it exists: <see cref="MethodBase.Invoke(object, object[])"/>
    /// allocates an argument array per call and boxes every value-type
    /// argument, and doing that once per text object per frame is measurable
    /// garbage-collector pressure inside somebody else's frame budget.
    /// </para>
    /// </remarks>
    public static class RabtZaid
    {
        /// <summary>
        /// Public and non-public, instance members only — the same relaxation
        /// <see cref="Rabt"/> makes, and for the same reason: several of the
        /// members these takeovers need are <c>protected</c> in one build of
        /// TextMeshPro and <c>public</c> in the next, and a binding that
        /// refused the former would degrade a whole text system over an access
        /// modifier.
        /// </summary>
        private const BindingFlags Alamat =
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;

        /// <summary>
        /// A one-argument instance method as a delegate, or <c>null</c> when
        /// this build has no such method or its signature does not match.
        /// </summary>
        /// <typeparam name="T1">The argument type.</typeparam>
        /// <param name="naw">The declaring type, from <see cref="Rabt.Naw"/>.</param>
        /// <param name="ism">The method name.</param>
        /// <returns>The invoker, or <c>null</c>.</returns>
        public static Action<object, T1>? Amr<T1>(Type? naw, string ism)
        {
            MethodInfo? tariqa = Wajid(naw, ism, typeof(T1));
            if (tariqa is null)
            {
                return null;
            }
            return (Action<object, T1>?)Yabni(
                nameof(AmrMuhkam1), tariqa, new[] { typeof(T1) }, ism);
        }

        /// <summary>A two-argument instance method as a delegate.</summary>
        /// <typeparam name="T1">The first argument type.</typeparam>
        /// <typeparam name="T2">The second argument type.</typeparam>
        /// <param name="naw">The declaring type.</param>
        /// <param name="ism">The method name.</param>
        /// <returns>The invoker, or <c>null</c>.</returns>
        public static Action<object, T1, T2>? Amr<T1, T2>(Type? naw, string ism)
        {
            MethodInfo? tariqa = Wajid(naw, ism, typeof(T1), typeof(T2));
            if (tariqa is null)
            {
                return null;
            }
            return (Action<object, T1, T2>?)Yabni(
                nameof(AmrMuhkam2), tariqa, new[] { typeof(T1), typeof(T2) }, ism);
        }

        /// <summary>A three-argument instance method as a delegate.</summary>
        /// <typeparam name="T1">The first argument type.</typeparam>
        /// <typeparam name="T2">The second argument type.</typeparam>
        /// <typeparam name="T3">The third argument type.</typeparam>
        /// <param name="naw">The declaring type.</param>
        /// <param name="ism">The method name.</param>
        /// <returns>The invoker, or <c>null</c>.</returns>
        public static Action<object, T1, T2, T3>? Amr<T1, T2, T3>(Type? naw, string ism)
        {
            MethodInfo? tariqa = Wajid(naw, ism, typeof(T1), typeof(T2), typeof(T3));
            if (tariqa is null)
            {
                return null;
            }
            return (Action<object, T1, T2, T3>?)Yabni(
                nameof(AmrMuhkam3), tariqa, new[] { typeof(T1), typeof(T2), typeof(T3) }, ism);
        }

        /// <summary>
        /// The <see cref="MethodInfo"/> of a patch target, searched on the
        /// named type only and never up its base chain.
        /// </summary>
        /// <remarks>
        /// Declared-only is not tidiness, it is the whole correctness of a
        /// virtual patch target. <c>TMP_Text.GenerateTextMesh</c> is a virtual
        /// method that both concrete classes override. A patch installed on the
        /// base declaration never runs, because every call site dispatches to
        /// the override — and the symptom is a takeover that resolves cleanly,
        /// logs nothing, and draws no Arabic at all. Each concrete class is
        /// patched on its own declaration, and a class that does not declare
        /// one is reported by name and skipped.
        /// </remarks>
        /// <param name="naw">The concrete type to search.</param>
        /// <param name="ism">The method name.</param>
        /// <param name="muamalat">The parameter types, in order; none for a
        /// parameterless method.</param>
        /// <returns>The method, or <c>null</c> when this build has no such
        /// declaration on this exact type.</returns>
        public static MethodInfo? HadafMuarraf(Type? naw, string ism, params Type[] muamalat)
        {
            if (naw is null || string.IsNullOrEmpty(ism))
            {
                return null;
            }
            try
            {
                return naw.GetMethod(
                    ism, Alamat | BindingFlags.DeclaredOnly, null, muamalat, null);
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Resolving " + naw.Name + "." + ism + " failed", khata);
                return null;
            }
        }

        /// <summary>
        /// The <see cref="MethodInfo"/> of a property accessor on the named
        /// type only — the patch target form of a getter such as
        /// <c>Text.get_mainTexture</c>.
        /// </summary>
        /// <param name="naw">The concrete type to search.</param>
        /// <param name="ism">The property name, without the accessor prefix.</param>
        /// <param name="katib">Whether the setter is wanted rather than the getter.</param>
        /// <returns>The accessor, or <c>null</c>.</returns>
        public static MethodInfo? HadafKhasiya(Type? naw, string ism, bool katib)
        {
            if (naw is null || string.IsNullOrEmpty(ism))
            {
                return null;
            }
            try
            {
                PropertyInfo? khasiya = naw.GetProperty(
                    ism, Alamat | BindingFlags.DeclaredOnly);
                return katib
                    ? khasiya?.GetSetMethod(nonPublic: true)
                    : khasiya?.GetGetMethod(nonPublic: true);
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Resolving " + naw.Name + "." + ism + " failed", khata);
                return null;
            }
        }

        /// <summary>
        /// A method with the given signature, searched up the inheritance
        /// chain because TextMeshPro and uGUI both declare half of what these
        /// takeovers call on a base class.
        /// </summary>
        private static MethodInfo? Wajid(Type? naw, string ism, params Type[] muamalat)
        {
            if (naw is null || string.IsNullOrEmpty(ism))
            {
                return null;
            }
            try
            {
                for (Type? hali = naw; hali is not null; hali = hali.BaseType)
                {
                    MethodInfo? tariqa = hali.GetMethod(
                        ism, Alamat | BindingFlags.DeclaredOnly, null, muamalat, null);
                    if (tariqa is not null && !tariqa.IsStatic
                        && tariqa.ReturnType == typeof(void))
                    {
                        return tariqa;
                    }
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Binding " + naw.Name + "." + ism + " failed", khata);
            }
            return null;
        }

        /// <summary>
        /// Closes one of the three shims below over the type the method was
        /// declared on, which is the only way to build a strongly typed open
        /// delegate for a type this assembly cannot name. Runs once per member,
        /// at startup.
        /// </summary>
        private static Delegate? Yabni(string shim, MethodInfo tariqa, Type[] anwa, string ism)
        {
            try
            {
                Type hadaf = tariqa.DeclaringType ?? typeof(object);
                MethodInfo? aam = typeof(RabtZaid).GetMethod(
                    shim, BindingFlags.NonPublic | BindingFlags.Static);
                if (aam is null)
                {
                    return null;
                }
                Type[] mughlaq = new Type[anwa.Length + 1];
                mughlaq[0] = hadaf;
                Array.Copy(anwa, 0, mughlaq, 1, anwa.Length);
                return aam.MakeGenericMethod(mughlaq).Invoke(null, new object[] { tariqa })
                    as Delegate;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Building a delegate for " + ism + " failed", khata);
                return null;
            }
        }

        private static Action<object, T1> AmrMuhkam1<THadaf, T1>(MethodInfo tariqa)
            where THadaf : class
        {
            var mubashir =
                (Action<THadaf, T1>)tariqa.CreateDelegate(typeof(Action<THadaf, T1>));
            return (hadaf, awwal) => mubashir((THadaf)hadaf, awwal);
        }

        private static Action<object, T1, T2> AmrMuhkam2<THadaf, T1, T2>(MethodInfo tariqa)
            where THadaf : class
        {
            var mubashir =
                (Action<THadaf, T1, T2>)tariqa.CreateDelegate(typeof(Action<THadaf, T1, T2>));
            return (hadaf, awwal, thani) => mubashir((THadaf)hadaf, awwal, thani);
        }

        private static Action<object, T1, T2, T3> AmrMuhkam3<THadaf, T1, T2, T3>(
            MethodInfo tariqa)
            where THadaf : class
        {
            var mubashir = (Action<THadaf, T1, T2, T3>)tariqa.CreateDelegate(
                typeof(Action<THadaf, T1, T2, T3>));
            return (hadaf, awwal, thani, thalith) =>
                mubashir((THadaf)hadaf, awwal, thani, thalith);
        }
    }

    /// <summary>
    /// مخزن الرسوم — the pooled geometry buffers one takeover writes a mesh
    /// through: positions, texture coordinates, colours, indices, the inline
    /// atom table, and the style spans converted out of the patch container.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Why the arrays are Unity's own types.</b> <see cref="Nasij"/> cannot
    /// name <see cref="Vector3"/>, so it writes <see cref="NuqtaRasm"/> — three
    /// sequential floats with the identical layout. These arrays are declared
    /// as the Unity types and handed to Nasij as spans reinterpreted through
    /// <see cref="MemoryMarshal.Cast{TFrom, TTo}(Span{TFrom})"/>, so the mesh
    /// upload receives the array it wants with no element-by-element copy in
    /// between. <see cref="Nasij.TahaqquqTakhtit"/> is called once at startup
    /// to confirm the three sizes really do match, because that cast is checked
    /// for element count and not for meaning.
    /// </para>
    /// <para>
    /// <b>Why the capacity is quantized.</b> Unity's oldest and most portable
    /// mesh API — <see cref="Mesh.vertices"/> and its siblings — takes the
    /// whole array and derives the vertex count from its length, so a buffer
    /// that is merely large enough would draw the tail as garbage. Growing to
    /// an exact fit instead would allocate four arrays every time a label gains
    /// or loses one glyph, which on a dialogue box is every frame. So capacity
    /// is rounded up to a multiple of <see cref="Kutla"/> glyphs and the tail
    /// past what was written is cleared: cleared positions collapse to the
    /// origin, cleared colours are transparent, and cleared indices name a
    /// degenerate triangle the rasterizer discards. The bounds are set from
    /// <see cref="NatijaNasij.Hudud"/> rather than recomputed, so the collapsed
    /// tail cannot drag the bounding box toward the origin and make the text
    /// vanish when the camera moves.
    /// </para>
    /// </remarks>
    public sealed class MakhzanRusum
    {
        /// <summary>
        /// How many glyphs the capacity is rounded up to. Sixteen: small
        /// enough that a short label does not carry a page of dead vertices,
        /// large enough that a typewriter effect revealing one letter per frame
        /// re-allocates once every sixteen frames rather than every frame.
        /// </summary>
        public const int Kutla = 16;

        private Vector3[] ruus;
        private Vector2[] malamis;
        private Color32[] alwan;
        private int[] muthallathat;
        private DharraMawduaa[] dharrat;
        private TaaribNitaqUslub[] nitaqat;
        private int siaatAshkal;

        /// <summary>Creates the buffers with room for one short label.</summary>
        public MakhzanRusum()
        {
            siaatAshkal = 0;
            ruus = Array.Empty<Vector3>();
            malamis = Array.Empty<Vector2>();
            alwan = Array.Empty<Color32>();
            muthallathat = Array.Empty<int>();
            dharrat = new DharraMawduaa[8];
            nitaqat = new TaaribNitaqUslub[8];
            Wassi(64);
        }

        /// <summary>The glyph capacity currently allocated.</summary>
        public int SiaatAshkal => siaatAshkal;

        /// <summary>The vertex positions, as the array a mesh upload takes.</summary>
        public Vector3[] Ruus => ruus;

        /// <summary>The texture coordinates.</summary>
        public Vector2[] Malamis => malamis;

        /// <summary>The vertex colours.</summary>
        public Color32[] Alwan => alwan;

        /// <summary>The triangle indices.</summary>
        public int[] Muthallathat => muthallathat;

        /// <summary>
        /// Grows the buffers so that <paramref name="adadHuruf"/> glyphs fit,
        /// rounded up to <see cref="Kutla"/>. The only allocating member of
        /// this class, and after the first few frames of a scene it stops
        /// allocating entirely.
        /// </summary>
        /// <param name="adadHuruf">
        /// How many glyphs the layout holds. <see cref="Nasij"/> demands four
        /// vertices and six indices for every one of them as an upper bound,
        /// because knowing the exact count would cost an atlas lookup per glyph
        /// before the first vertex is written.
        /// </param>
        public void Wassi(int adadHuruf)
        {
            if (adadHuruf <= siaatAshkal)
            {
                return;
            }
            int matlub = ((adadHuruf + Kutla - 1) / Kutla) * Kutla;
            ruus = new Vector3[matlub * Nasij.RuusLiShakl];
            malamis = new Vector2[matlub * Nasij.RuusLiShakl];
            alwan = new Color32[matlub * Nasij.RuusLiShakl];
            muthallathat = new int[matlub * Nasij.FahrasLiShakl];
            siaatAshkal = matlub;
        }

        /// <summary>
        /// The destination <see cref="Nasij.Ibni{TKhareeta}"/> writes into, as
        /// spans over these arrays with no copy anywhere in the path.
        /// </summary>
        /// <returns>The destination.</returns>
        public MakhzanNasij Makhzan()
        {
            MakhzanNasij makhzan = default;
            makhzan.Ruus = MemoryMarshal.Cast<Vector3, NuqtaRasm>(ruus.AsSpan());
            makhzan.Malamis = MemoryMarshal.Cast<Vector2, NuqtaMulmas>(malamis.AsSpan());
            makhzan.Alwan = MemoryMarshal.Cast<Color32, LawnRasm>(alwan.AsSpan());
            makhzan.Muthallathat = muthallathat.AsSpan();
            makhzan.Dharrat = dharrat.AsSpan();
            return makhzan;
        }

        /// <summary>
        /// Clears everything past what a build wrote, so the fixed-length
        /// arrays a mesh upload takes carry no stale geometry from the previous
        /// string.
        /// </summary>
        /// <param name="natija">What the build wrote.</param>
        public void Amsah(in NatijaNasij natija)
        {
            int baqiRuus = ruus.Length - natija.AdadRuus;
            if (baqiRuus > 0)
            {
                Array.Clear(ruus, natija.AdadRuus, baqiRuus);
                Array.Clear(malamis, natija.AdadRuus, baqiRuus);
                Array.Clear(alwan, natija.AdadRuus, baqiRuus);
            }
            int baqiFahras = muthallathat.Length - natija.AdadMuthallathat;
            if (baqiFahras > 0)
            {
                Array.Clear(muthallathat, natija.AdadMuthallathat, baqiFahras);
            }
        }

        /// <summary>
        /// Converts the patch container's own span records into the ABI's span
        /// struct, which is what <see cref="Nasij"/> and
        /// <see cref="Takhtit"/> both read.
        /// </summary>
        /// <remarks>
        /// The two records genuinely differ: <see cref="MadkhalNitaq"/> is
        /// twenty-four bytes keyed by string index, and
        /// <see cref="TaaribNitaqUslub"/> is fifty-two bytes with a byte range
        /// and the ABI's flag word. The conversion is a loop over a handful of
        /// entries into a pooled array, so it allocates only when a string
        /// carries more spans than any string before it.
        /// </remarks>
        /// <param name="madakhil">The container's spans for one string.</param>
        /// <returns>The spans as the layout and the mesh builder read them.</returns>
        public ReadOnlySpan<TaaribNitaqUslub> Hawwil(ReadOnlySpan<MadkhalNitaq> madakhil)
        {
            if (madakhil.IsEmpty)
            {
                return ReadOnlySpan<TaaribNitaqUslub>.Empty;
            }
            if (nitaqat.Length < madakhil.Length)
            {
                nitaqat = new TaaribNitaqUslub[madakhil.Length];
            }
            for (int i = 0; i < madakhil.Length; i++)
            {
                MadkhalNitaq madkhal = madakhil[i];
                TaaribNitaqUslub nitaq = default;
                nitaq.Bidaya = madkhal.Bidaya;
                nitaq.Tul = madkhal.Tul;
                nitaq.Lawn = madkhal.Lawn;
                nitaq.Hajm = madkhal.Hajm;
                nitaq.Id = madkhal.Id;
                if (madkhal.LahuLawn)
                {
                    nitaq.Alam |= Alamat.UslubLawn;
                }
                if (madkhal.Maail)
                {
                    nitaq.Alam |= Alamat.UslubMaail;
                }
                if (madkhal.Aswad)
                {
                    nitaq.Alam |= Alamat.UslubWazn;
                    nitaq.Wazn = Nasq.WaznGhaliz;
                }
                if (madkhal.Hajm > 0f)
                {
                    nitaq.Alam |= Alamat.UslubHajm;
                }
                if (madkhal.Dharra || madkhal.Sura)
                {
                    nitaq.Alam |= Alamat.UslubDharra;
                }
                nitaqat[i] = nitaq;
            }
            return new ReadOnlySpan<TaaribNitaqUslub>(nitaqat, 0, madakhil.Length);
        }
    }

    /// <summary>
    /// مصدر الأشكال — where a glyph image comes from, and the Unity objects
    /// that put it on the GPU: one <see cref="Texture2D"/> per atlas page, one
    /// <see cref="Material"/> per page, and the residency pass that brings a
    /// runtime glyph in before a mesh is built against it.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Two atlases, never mixed.</b> A patch carries its own compiled atlas,
    /// keyed by glyph identifiers in the patch's own font chain. The runtime
    /// atlas holds whatever the compiler never saw, keyed by identifiers in the
    /// chain this process loaded. Those two numbering spaces are not the same
    /// space: glyph 412 of the patch's chain and glyph 412 of the runtime chain
    /// are the same number naming two different letters whenever the two chains
    /// differ by so much as a fallback font. So the two are kept apart, a
    /// precomputed layout is only ever drawn from the patch atlas, a runtime
    /// layout is only ever drawn from the runtime atlas, and there is no code
    /// path here that consults one after the other. Merging them would produce
    /// a sentence with one letter from another alphabet in it, occasionally, on
    /// one machine.
    /// </para>
    /// <para>
    /// <b>What is set on the material, and why exactly that.</b>
    /// <c>_MainTex</c> is the atlas page. <c>_Color</c> is opaque white,
    /// because the real colour is per vertex — <see cref="Nasij"/> resolves it
    /// from the style span so a <c>&lt;color&gt;</c> survives bidirectional
    /// reordering, and a material tint would multiply over the top of that and
    /// flatten every coloured word in the game. For a distance-field patch two
    /// more are set: <c>_TaaribMisafa</c>, the spread in texels the rasterizer
    /// used, and <c>_TaaribQiyas</c>, the page's texel size as
    /// <c>(1/w, 1/h, w, h)</c> — together they are what lets the shader turn a
    /// sampled distance into a coverage value at any size without querying the
    /// texture's dimensions per fragment. Nothing else is set, and in
    /// particular nothing is read from the game's own <c>TMP_FontAsset</c>, its
    /// shared material, or its SDF atlas: Decision 5 is that the game's fonts
    /// are never touched, and that rule is kept here by there being no code
    /// that could touch one.
    /// </para>
    /// <para>
    /// <b>The shader is the engine's own, because a shader of Taarib's own can
    /// never be found.</b> <c>Shader.Find</c> returns only shaders compiled into
    /// the game's build, and this assembly ships no asset bundle — so the two
    /// names this class used to look up, <c>Taarib/Taghtiya</c> and
    /// <c>Taarib/Misafa</c>, existed nowhere and every Unity game refused at
    /// this exact line while reporting nothing. A coverage atlas is uploaded as
    /// <c>Alpha8</c> and drawn through <c>UI/Default</c>, which every Unity
    /// build includes, with <c>_TextureSampleAdd</c> set to lift the sampled
    /// RGB to white: exactly the arrangement Unity's own legacy <c>Text</c>
    /// uses for its font atlases, so the glyph's coverage lands in alpha and the
    /// vertex colour supplies the colour. A distance-field atlas needs the
    /// TextMeshPro distance-field path, which this build does not yet wire, and
    /// it refuses saying so rather than drawing a distance field as coverage.
    /// </para>
    /// </remarks>
    public sealed class MasdarAshkal : IDisposable
    {
        /// <summary>
        /// The engine's own UI shader, in every Unity build's always-included
        /// list. It draws an <c>Alpha8</c> page as coverage once
        /// <see cref="MuarrifJamAyina"/> lifts the sampled RGB to white.
        /// </summary>
        public const string IsmSudfatTaghtiya = "UI/Default";

        /// <summary>
        /// The TextMeshPro distance-field shader every TMP game carries. Named
        /// so the refusal can say what a distance-field patch would need; this
        /// build does not draw through it yet.
        /// </summary>
        public const string IsmSudfatMisafa = "TextMeshPro/Distance Field";

        private static readonly int MuarrifLawha = Shader.PropertyToID("_MainTex");
        private static readonly int MuarrifLawn = Shader.PropertyToID("_Color");
        private static readonly int MuarrifJamAyina = Shader.PropertyToID("_TextureSampleAdd");
        private static readonly int MuarrifMisafa = Shader.PropertyToID("_TaaribMisafa");
        private static readonly int MuarrifQiyas = Shader.PropertyToID("_TaaribQiyas");

        private readonly Ruqaa ruqaa;
        private readonly MaqbadSilsila? silsila;
        private readonly Lawha? lawha;
        private readonly Shader sudfa;
        private readonly float misafa;

        private Texture2D?[] lawhatRuqaa;
        private Material?[] maddatRuqaa;
        private Texture2D?[] lawhatHayya;
        private Material?[] maddatHayya;
        private QiyasSafha[] qiyasatHayya;
        private SijillRafa[] talabat = new SijillRafa[8];
        private Texture2D? marhala;
        private byte[] mustaar = new byte[4096];
        private readonly bool yansakh =
            (SystemInfo.copyTextureSupport & CopyTextureSupport.Basic) != 0;
        private ulong akhirJeel;
        private int akhirItar;
        private bool amil;

        private MasdarAshkal(
            Ruqaa ruqaa,
            MaqbadSilsila? silsila,
            Lawha? lawha,
            Shader sudfa,
            float misafa)
        {
            this.ruqaa = ruqaa;
            this.silsila = silsila;
            this.lawha = lawha;
            this.sudfa = sudfa;
            this.misafa = misafa;
            lawhatRuqaa = Array.Empty<Texture2D?>();
            maddatRuqaa = Array.Empty<Material?>();
            lawhatHayya = Array.Empty<Texture2D?>();
            maddatHayya = Array.Empty<Material?>();
            qiyasatHayya = Array.Empty<QiyasSafha>();
            akhirItar = -1;
            amil = true;
        }

        /// <summary>Whether this source is usable.</summary>
        public bool Amil => amil;

        /// <summary>How this patch's atlas was rasterized.</summary>
        public NamatLawha Namat => ruqaa.Namat;

        /// <summary>
        /// The pixel size the atlas rasterized its glyphs at, which is what
        /// <see cref="TalabNasij.HajmLawha"/> takes.
        /// </summary>
        /// <param name="hajm">The size the text is being drawn at.</param>
        /// <param name="bikselLilWahda">
        /// Screen pixels per layout unit, from <see cref="QiyasShasha"/>. One
        /// on an unscaled overlay canvas, and anything at all on a scaled
        /// canvas or a world-space surface.
        /// </param>
        /// <param name="minRuqaa">
        /// Whether this layout came out of the patch, and will therefore be
        /// drawn from the patch's own glyph map — see <see cref="Khareeta"/>.
        /// </param>
        /// <returns>
        /// For a coverage atlas, which holds one bitmap per size, the size that
        /// puts one atlas pixel on one screen pixel — chosen by
        /// <see cref="Nasij.HajmLawhaMulaim"/> so both backends choose alike.
        /// For a distance-field atlas, which holds one bitmap for every size,
        /// the atlas's own canonical size, on which the drawn size has no
        /// bearing at all.
        /// </returns>
        public float HajmLawha(float hajm, float bikselLilWahda, bool minRuqaa)
        {
            if (ruqaa.Namat != NamatLawha.Misafa)
            {
                // A layout that came out of the patch is drawn from the patch's
                // atlas, and that atlas holds the sizes the compiler chose and
                // no others. Asking it for the size the glyph covers on screen
                // would ask for a size it does not have, and the string would
                // be missing rather than soft. Only a layout this process laid
                // out, whose glyphs it also rasterizes on demand, can be given
                // a size of its own.
                return minRuqaa ? hajm : Nasij.HajmLawhaMulaim(hajm, bikselLilWahda);
            }
            ReadOnlySpan<TaaribMiftahShakl> mafatih = ruqaa.MafatihAshkal;
            return mafatih.IsEmpty ? hajm : mafatih[0].HajmRubi / 4.0f;
        }

        /// <summary>
        /// Builds the source: checks the vertex layout, finds the shader the
        /// patch's atlas mode requires, and uploads every compiled page.
        /// </summary>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="silsila">
        /// The font chain runtime glyphs are shaped with. <c>null</c> leaves
        /// the patch's compiled atlas working and declines every glyph the
        /// compiler never rasterized.
        /// </param>
        /// <param name="lawha">The runtime atlas and its residency, or <c>null</c>.</param>
        /// <returns>The source, or <c>null</c> when it refused with a reason in
        /// the log.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="ruqaa"/> is null.</exception>
        public static MasdarAshkal? Insha(
            Ruqaa ruqaa,
            MaqbadSilsila? silsila,
            Lawha? lawha)
        {
            if (ruqaa is null)
            {
                throw new ArgumentNullException(nameof(ruqaa));
            }

            try
            {
                // The one seam the type system cannot express: Mushtarak writes
                // through a reinterpreting cast that is checked for element
                // count and not for meaning, so a runtime whose Vector3 were
                // not three floats would write every vertex into the middle of
                // its neighbour. Refused here, once, before any mesh exists.
                Nasij.TahaqquqTakhtit(
                    Marshal.SizeOf<Vector3>(),
                    Marshal.SizeOf<Vector2>(),
                    Marshal.SizeOf<Color32>());
            }
            catch (KhataTaarib khata)
            {
                Rabt.Ballagh(khata.Arabi + " | " + khata.Injilizi);
                return null;
            }

            if (!SystemInfo.SupportsTextureFormat(TextureFormat.Alpha8))
            {
                Rabt.Ballagh(
                    "لا يدعم هذا الجهاز صيغة Alpha8 التي تُرفع بها لوحة تعريب؛ أُوقف الاستيلاء على النص بدل رسم اللوحة بقناة خاطئة. | "
                    + "This device does not support the Alpha8 texture format Taarib's atlas "
                    + "is uploaded as; the takeover is off rather than sampling the atlas "
                    + "from the wrong channel.");
                return null;
            }

            bool misafi = ruqaa.Namat == NamatLawha.Misafa;
            if (misafi)
            {
                Rabt.Ballagh(
                    "هذه الرقعة تحمل لوحة مسافة، وهذا البناء يرسم لوحات التغطية فقط عبر صدفة المحرّك (" + IsmSudfatTaghtiya + ")؛ لوحة المسافة تحتاج مسار " + IsmSudfatMisafa + " ولم يُوصَل بعد. أُوقف الاستيلاء بدل رسم حقل مسافة كأنه تغطية. | "
                    + "This patch carries a distance-field atlas, and this build draws only "
                    + "coverage atlases through the engine's " + IsmSudfatTaghtiya + " shader; "
                    + "a distance field needs the " + IsmSudfatMisafa + " path, which is not "
                    + "wired yet. The takeover is off rather than drawing a distance field as "
                    + "if it were coverage.");
                return null;
            }
            Shader? sudfa = Shader.Find(IsmSudfatTaghtiya);
            if (sudfa is null)
            {
                Rabt.Ballagh(
                    "لم تُوجد صدفة المحرّك " + IsmSudfatTaghtiya + " في هذا البناء من اللعبة؛ أُوقف الاستيلاء بدل الرسم بصدفة مجهولة. | "
                    + "The engine's " + IsmSudfatTaghtiya + " shader is not in this game's "
                    + "build; the takeover is off rather than drawing through a shader "
                    + "nobody chose.");
                return null;
            }

            // The spread a distance-field atlas was rasterized with, in texels.
            // Four is the rasterizer's own default and the only value a patch
            // records implicitly; a patch that ever carries its own spread will
            // put it in the manifest, and this is the one line that reads it.
            MasdarAshkal masdar = new MasdarAshkal(ruqaa, silsila, lawha, sudfa, 4f);
            if (!masdar.ArfaSafahatRuqaa())
            {
                masdar.Dispose();
                return null;
            }
            return masdar;
        }

        /// <summary>
        /// The glyph map <see cref="Nasij.Ibni{TKhareeta}"/> looks glyphs up
        /// through.
        /// </summary>
        /// <param name="minRuqaa">
        /// Whether the layout being drawn came out of the patch. A precomputed
        /// layout must be drawn from the patch atlas and a runtime layout from
        /// the runtime atlas; see the class remarks for why the two are never
        /// consulted in sequence.
        /// </param>
        /// <returns>The map, as a struct so no interface call is dispatched
        /// virtually inside the vertex loop.</returns>
        public KhareetaMuakkada Khareeta(bool minRuqaa)
        {
            return new KhareetaMuakkada(this, minRuqaa);
        }

        /// <summary>The material one atlas page draws through.</summary>
        /// <param name="minRuqaa">Whether the page belongs to the patch atlas.</param>
        /// <param name="fahras">The page index.</param>
        /// <returns>The material, or <c>null</c> when there is no such page.</returns>
        public Material? Madda(bool minRuqaa, ushort fahras)
        {
            Material?[] madat = minRuqaa ? maddatRuqaa : maddatHayya;
            return fahras < madat.Length ? madat[fahras] : null;
        }

        /// <summary>How many atlas pages one source has, for the diagnostics line.</summary>
        /// <param name="minRuqaa">Whether to count the patch atlas's pages.</param>
        /// <returns>The page count.</returns>
        public int AdadSafahat(bool minRuqaa)
        {
            return (minRuqaa ? lawhatRuqaa : lawhatHayya).Length;
        }

        /// <summary>The texture one atlas page was uploaded into.</summary>
        /// <param name="minRuqaa">Whether the page belongs to the patch atlas.</param>
        /// <param name="fahras">The page index.</param>
        /// <returns>The texture, or <c>null</c> when there is no such page.</returns>
        public Texture2D? LawhatSafha(bool minRuqaa, ushort fahras)
        {
            Texture2D?[] lawhat = minRuqaa ? lawhatRuqaa : lawhatHayya;
            return fahras < lawhat.Length ? lawhat[fahras] : null;
        }

        /// <summary>Where one glyph sits in the atlas it belongs to.</summary>
        /// <param name="minRuqaa">Whether to search the patch atlas.</param>
        /// <param name="miftah">The glyph, its font, its size and its bucket.</param>
        /// <param name="mawdi">Its rectangle, bearings and page.</param>
        /// <returns>Whether it is resident.</returns>
        public bool Shakl(bool minRuqaa, in TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi)
        {
            if (minRuqaa)
            {
                return ruqaa.JidShakl(miftah, out mawdi);
            }
            mawdi = default;
            Lawha? hayya = lawha;
            MaqbadSilsila? chain = silsila;
            if (hayya is null || chain is null)
            {
                return false;
            }
            try
            {
                hayya.Sajjil(chain, in miftah, out mawdi);
                return true;
            }
            catch (KhataTaarib)
            {
                // A full atlas or a glyph the chain has no image for is a
                // per-glyph miss, which Nasij counts and draws nothing for.
                // It is emphatically not a reason to switch the takeover off:
                // the rest of the sentence is still correct.
                return false;
            }
        }

        /// <summary>One page's dimensions, for normalizing texture coordinates.</summary>
        /// <param name="minRuqaa">Whether the page belongs to the patch atlas.</param>
        /// <param name="fahras">The page index.</param>
        /// <param name="qiyas">Its size in texels.</param>
        /// <returns>Whether there is such a page.</returns>
        public bool Safha(bool minRuqaa, ushort fahras, out QiyasSafha qiyas)
        {
            qiyas = default;
            if (minRuqaa)
            {
                ReadOnlySpan<MadkhalSafha> jadwal = ruqaa.Safahat;
                if (fahras >= jadwal.Length)
                {
                    return false;
                }
                qiyas.Ard = jadwal[fahras].Ard;
                qiyas.Irtifa = jadwal[fahras].Irtifa;
                return true;
            }
            if (fahras >= qiyasatHayya.Length)
            {
                return false;
            }
            qiyas = qiyasatHayya[fahras];
            return qiyas.Ard != 0 && qiyas.Irtifa != 0;
        }

        /// <summary>
        /// Brings every glyph of a runtime layout into the runtime atlas and
        /// re-uploads whatever grew, before a mesh is built against a map that
        /// must not miss.
        /// </summary>
        /// <remarks>
        /// <para>
        /// Residency is a separate pass from mesh building on purpose, and the
        /// reason is stated on <see cref="IKhareetatAshkal"/>: rasterizing
        /// inside the vertex loop would mean a native transition per glyph and
        /// an atlas-full failure thrown halfway through a mesh, leaving the
        /// destination arrays holding half a sentence at full confidence. This
        /// walks the layout once, brings everything in, uploads what changed,
        /// and only then is the map incapable of missing.
        /// </para>
        /// <para>
        /// <see cref="Lawha.Ibda"/> is called at most once per engine
        /// frame. That call is what protects every glyph looked up afterwards
        /// from eviction until the next frame; calling it again in the middle
        /// of a frame would drop that protection for the text already built,
        /// and the symptom is one glyph in one menu flickering into a different
        /// letter, unreproducibly.
        /// </para>
        /// </remarks>
        /// <param name="huruf">The layout's glyphs.</param>
        /// <param name="hajmLawha">The size the atlas rasterizes at.</param>
        /// <param name="tathbit">Whether the pen is snapped, which selects the
        /// subpixel bucket a glyph is rasterized for.</param>
        /// <returns>Whether the atlas is ready to be drawn from.</returns>
        public bool Aqim(ReadOnlySpan<TaaribHarf> huruf, float hajmLawha, bool tathbit)
        {
            Lawha? hayya = lawha;
            MaqbadSilsila? chain = silsila;
            if (!amil || hayya is null || chain is null)
            {
                return false;
            }

            try
            {
                int itar = Time.frameCount;
                if (itar != akhirItar)
                {
                    hayya.Ibda();
                    akhirItar = itar;
                }

                ushort hajmRubi = Nasij.HajmRubi(hajmLawha);
                for (int i = 0; i < huruf.Length; i++)
                {
                    TaaribMiftahShakl miftah;
                    miftah.Muarrif = huruf[i].Muarrif;
                    miftah.HajmRubi = hajmRubi;
                    miftah.Khatt = huruf[i].Khatt;
                    miftah.Bakat = Nasij.Bakat(huruf[i].S, tathbit);
                    hayya.Sajjil(chain, in miftah, out _);
                }

                // The generation counter moves whenever a page was added or a
                // texel changed, so one comparison decides whether anything has
                // to reach the GPU at all. On the overwhelmingly common frame —
                // every glyph already resident — this returns without touching
                // the atlas again.
                if (hayya.Jeel == akhirJeel && qiyasatHayya.Length != 0)
                {
                    return true;
                }
                akhirJeel = hayya.Jeel;
                return ArfaSafahatHayya(hayya);
            }
            catch (KhataTaarib khata)
            {
                // Attributable to this one string and this one atlas state, so
                // the string is skipped and the takeover stays on: the next
                // sentence very likely fits.
                Rabt.Ballagh("TextMeshPro: making runtime glyphs resident failed", khata);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("making runtime glyphs resident threw", khata);
                return false;
            }
        }

        /// <summary>
        /// Switches the whole source off, naming the reason once. Every
        /// takeover that draws through it then leaves its text in the game's
        /// original language rather than drawing nothing.
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
            Rabt.Ballagh(
                "تعذّر تجهيز لوحة تعريب: " + sabab + "؛ عاد النص إلى لغته الأصلية. | "
                + "Taarib's atlas could not be prepared: " + sabab
                + "; text is left in the game's original language.",
                khata);
        }

        /// <summary>
        /// Destroys the textures and materials this source created. Nothing of
        /// the game's is destroyed here, because nothing of the game's was ever
        /// created or retained.
        /// </summary>
        public void Dispose()
        {
            amil = false;
            Atlif(lawhatRuqaa, maddatRuqaa);
            Atlif(lawhatHayya, maddatHayya);
            lawhatRuqaa = Array.Empty<Texture2D?>();
            maddatRuqaa = Array.Empty<Material?>();
            lawhatHayya = Array.Empty<Texture2D?>();
            maddatHayya = Array.Empty<Material?>();
            qiyasatHayya = Array.Empty<QiyasSafha>();
            if (marhala != null)
            {
                UnityEngine.Object.Destroy(marhala);
                marhala = null;
            }
        }

        private static void Atlif(Texture2D?[] lawhat, Material?[] madat)
        {
            for (int i = 0; i < madat.Length; i++)
            {
                if (madat[i] is not null)
                {
                    UnityEngine.Object.Destroy(madat[i]);
                }
            }
            for (int i = 0; i < lawhat.Length; i++)
            {
                if (lawhat[i] is not null)
                {
                    UnityEngine.Object.Destroy(lawhat[i]);
                }
            }
        }

        private bool ArfaSafahatRuqaa()
        {
            ReadOnlySpan<MadkhalSafha> jadwal = ruqaa.Safahat;
            if (jadwal.IsEmpty)
            {
                Rabt.Ballagh(
                    "الرقعة المثبَّتة لا تحمل صفحات لوحة؛ لا شيء يمكن رسمه منها. | "
                    + "The installed patch carries no atlas pages; there is nothing in it "
                    + "to draw from.");
                return false;
            }
            lawhatRuqaa = new Texture2D?[jadwal.Length];
            maddatRuqaa = new Material?[jadwal.Length];
            for (int i = 0; i < jadwal.Length; i++)
            {
                ReadOnlySpan<byte> texelat = ruqaa.TexelatSafha(i);
                if (!Arfa(lawhatRuqaa, maddatRuqaa, i, texelat, jadwal[i].Ard, jadwal[i].Irtifa))
                {
                    return false;
                }
            }
            return true;
        }

        private bool ArfaSafahatHayya(Lawha hayya)
        {
            int adad = hayya.AdadSafahat;
            if (adad <= 0)
            {
                return false;
            }
            if (lawhatHayya.Length < adad)
            {
                Array.Resize(ref lawhatHayya, adad);
                Array.Resize(ref maddatHayya, adad);
                Array.Resize(ref qiyasatHayya, adad);
            }
            if (talabat.Length < adad)
            {
                talabat = new SijillRafa[adad];
            }

            // Only the pages that changed, and within each, only the rectangle
            // that changed. Re-uploading every page whenever one glyph was
            // rasterized would be sixteen megabytes of PCIe traffic for a
            // kilobyte of new texels, in the middle of a frame, every time a
            // dialogue line reaches a letter the atlas had not seen.
            int matlub = hayya.Iltaqit(talabat);
            for (int i = 0; i < matlub; i++)
            {
                SijillRafa talab = talabat[i];
                int fahras = talab.Safha;
                if ((uint)fahras >= (uint)lawhatHayya.Length)
                {
                    continue;
                }

                bool kamil = talab.Jadida
                    || !yansakh
                    || lawhatHayya[fahras] is null
                    || (talab.Wasikh.S == 0 && talab.Wasikh.A == 0
                        && talab.Wasikh.Ard == talab.Ard
                        && talab.Wasikh.Irtifa == talab.Irtifa);

                ReadOnlySpan<byte> texelat = hayya.Texelat(fahras);
                bool najah = kamil
                    ? Arfa(lawhatHayya, maddatHayya, fahras, texelat, talab.Ard, talab.Irtifa)
                    : ArfaJuzi(hayya, fahras, talab);
                if (!najah)
                {
                    return false;
                }

                qiyasatHayya[fahras].Ard = (ushort)talab.Ard;
                qiyasatHayya[fahras].Irtifa = (ushort)talab.Irtifa;
                hayya.Rufia(fahras);
            }
            return true;
        }

        /// <summary>
        /// Uploads one page's changed rectangle rather than the whole page.
        /// </summary>
        /// <remarks>
        /// Unity's CPU-side texture API has no partial raw upload:
        /// <c>LoadRawTextureData</c> writes a whole mip and <c>SetPixels</c>
        /// takes a <c>Color[]</c>, which for a 32-square glyph is four kilobytes
        /// of managed allocation to move one kilobyte of texels. So the
        /// rectangle goes up through a small staging texture and
        /// <c>Graphics.CopyTexture</c>, which is a real sub-rectangle copy done
        /// on the GPU. Where the platform reports no basic copy support, the
        /// caller takes the whole-page path instead and pays the bandwidth.
        /// </remarks>
        private unsafe bool ArfaJuzi(Lawha hayya, int fahras, SijillRafa talab)
        {
            Texture2D? safha = lawhatHayya[fahras];
            if (safha is null)
            {
                return false;
            }

            MustatilLawha wasikh = talab.Wasikh;
            Texture2D marhalatHali = Marhala(wasikh.Ard, wasikh.Irtifa);
            int budS = marhalatHali.width;
            int matlub = budS * marhalatHali.height;
            if (mustaar.Length != matlub)
            {
                mustaar = new byte[matlub];
            }

            // The rectangle is packed into the staging texture at the staging
            // texture's own stride, with the remainder left as whatever was
            // there. CopyTexture takes only the top-left Ard by Irtifa, so the
            // stale remainder is never read and does not have to be cleared.
            ReadOnlySpan<byte> kull = hayya.Texelat(fahras);
            for (int satr = 0; satr < wasikh.Irtifa; satr++)
            {
                int masdar = ((wasikh.A + satr) * talab.Ard) + wasikh.S;
                if (masdar < 0 || masdar > kull.Length - wasikh.Ard)
                {
                    return false;
                }
                kull.Slice(masdar, wasikh.Ard).CopyTo(
                    new Span<byte>(mustaar, satr * budS, wasikh.Ard));
            }

            fixed (byte* asas = mustaar)
            {
                marhalatHali.LoadRawTextureData((IntPtr)asas, matlub);
            }
            marhalatHali.Apply(false, false);
            Graphics.CopyTexture(
                marhalatHali, 0, 0, 0, 0, wasikh.Ard, wasikh.Irtifa,
                safha, 0, 0, wasikh.S, wasikh.A);
            return true;
        }

        /// <summary>
        /// The staging texture, grown to the largest rectangle seen and never
        /// shrunk.
        /// </summary>
        /// <remarks>
        /// A glyph atlas's dirty rectangles settle into a handful of sizes
        /// within the first few glyphs, so this stops reallocating almost
        /// immediately. Recreating it per upload would trade a managed
        /// allocation for a graphics-resource allocation, which is worse by a
        /// wide margin.
        /// </remarks>
        private Texture2D Marhala(int ard, int irtifa)
        {
            Texture2D? mawjud = marhala;
            if (mawjud != null && mawjud.width >= ard && mawjud.height >= irtifa)
            {
                return mawjud;
            }

            int budS = Mathf.NextPowerOfTwo(Mathf.Max(ard, mawjud != null ? mawjud.width : 1));
            int budA = Mathf.NextPowerOfTwo(Mathf.Max(irtifa, mawjud != null ? mawjud.height : 1));
            if (mawjud != null)
            {
                UnityEngine.Object.Destroy(mawjud);
            }
            // Same format and linearity as the pages: Graphics.CopyTexture
            // refuses a copy between textures whose formats differ.
            Texture2D jadida = new Texture2D(budS, budA, TextureFormat.Alpha8, false, true);
            jadida.filterMode = FilterMode.Bilinear;
            jadida.wrapMode = TextureWrapMode.Clamp;
            jadida.hideFlags = HideFlags.HideAndDontSave;
            marhala = jadida;
            return jadida;
        }

        /// <summary>
        /// Uploads one page and builds its material. The texture is created
        /// once per page and re-uploaded in place afterwards, so an atlas that
        /// grows mid-scene costs one upload rather than a new texture, a new
        /// material and a new draw-call batch.
        /// </summary>
        private unsafe bool Arfa(
            Texture2D?[] lawhat,
            Material?[] madat,
            int fahras,
            ReadOnlySpan<byte> texelat,
            int ard,
            int irtifa)
        {
            if (ard <= 0 || irtifa <= 0 || texelat.Length < ard * irtifa)
            {
                Rabt.Ballagh(
                    "صفحة لوحة رقم " + fahras + " تعلن أبعادًا لا تطابق عدد بايتاتها؛ رُفض رفعها بدل رسم صفوف مزاحة. | "
                    + "Atlas page " + fahras + " declares dimensions its byte count does not "
                    + "match; the upload is refused rather than drawing rows offset from the "
                    + "rectangles the glyph map names.");
                return false;
            }

            try
            {
                Texture2D? lawhat2d = lawhat[fahras];
                if (lawhat2d is null || lawhat2d.width != ard || lawhat2d.height != irtifa)
                {
                    if (lawhat2d is not null)
                    {
                        UnityEngine.Object.Destroy(lawhat2d);
                    }
                    // linear: the atlas holds coverage or a signed distance,
                    // both of which are quantities rather than colours. Marking
                    // it sRGB would push every value through a gamma curve and
                    // make thin strokes visibly lighter than thick ones.
                    lawhat2d = new Texture2D(ard, irtifa, TextureFormat.Alpha8, false, true);
                    lawhat2d.filterMode = FilterMode.Bilinear;
                    lawhat2d.wrapMode = TextureWrapMode.Clamp;
                    lawhat2d.hideFlags = HideFlags.HideAndDontSave;
                    lawhat[fahras] = lawhat2d;
                }

                fixed (byte* asas = texelat)
                {
                    lawhat2d.LoadRawTextureData((IntPtr)asas, ard * irtifa);
                }
                lawhat2d.Apply(false, false);

                Material? madda = madat[fahras];
                if (madda is null)
                {
                    madda = new Material(sudfa);
                    madda.hideFlags = HideFlags.HideAndDontSave;
                    madat[fahras] = madda;
                }
                madda.SetTexture(MuarrifLawha, lawhat2d);
                madda.SetColor(MuarrifLawn, Color.white);
                // Alpha8 samples as (0, 0, 0, a). UI/Default adds this before
                // multiplying by the vertex colour, so the glyph's coverage is
                // the alpha and the colour is the component's own — the same
                // vector Unity's canvas sets for its legacy font atlases.
                madda.SetVector(MuarrifJamAyina, new Vector4(1f, 1f, 1f, 0f));
                if (ruqaa.Namat == NamatLawha.Misafa)
                {
                    madda.SetFloat(MuarrifMisafa, misafa);
                    madda.SetVector(
                        MuarrifQiyas, new Vector4(1f / ard, 1f / irtifa, ard, irtifa));
                }
                return true;
            }
            catch (Exception khata)
            {
                Awqif("uploading atlas page " + fahras + " threw", khata);
                return false;
            }
        }
    }

    /// <summary>
    /// خريطة مؤكدة — the glyph map <see cref="Nasij"/> is generic over,
    /// bound once to one atlas.
    /// </summary>
    /// <remarks>
    /// A <c>readonly struct</c> rather than a class, because
    /// <see cref="Nasij.Ibni{TKhareeta}"/> constrains its parameter to
    /// <see cref="IKhareetatAshkal"/> with no class constraint: a value-type
    /// implementation is dispatched statically and never boxed, which on a
    /// screen full of dialogue is the difference between one interface call per
    /// glyph and none.
    /// </remarks>
    public readonly struct KhareetaMuakkada : IKhareetatAshkal
    {
        private readonly MasdarAshkal masdar;
        private readonly bool minRuqaa;

        /// <summary>Binds the map to one atlas.</summary>
        /// <param name="masdar">The glyph source.</param>
        /// <param name="minRuqaa">Whether to read the patch's compiled atlas.</param>
        public KhareetaMuakkada(MasdarAshkal masdar, bool minRuqaa)
        {
            this.masdar = masdar;
            this.minRuqaa = minRuqaa;
        }

        /// <inheritdoc/>
        public bool Shakl(in TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi)
        {
            return masdar.Shakl(minRuqaa, in miftah, out mawdi);
        }

        /// <inheritdoc/>
        public bool Safha(ushort fahras, out QiyasSafha qiyas)
        {
            return masdar.Safha(minRuqaa, fahras, out qiyas);
        }
    }

    /// <summary>
    /// نص محضّر — one string's translated text and its style spans, prepared
    /// for a layout call, in buffers that are reused rather than allocated.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The patch container stores a translation as UTF-8 in its pool and the
    /// layout call takes <see cref="char"/>, so one decode into a pooled buffer
    /// stands between them. That decode is the only work on the common path.
    /// </para>
    /// <para>
    /// <b>When Nasq runs, and when it must not.</b> A patch compiled with the
    /// markup bridge already lifted every tag out of the translation and
    /// recorded the spans, so <see cref="Ruqaa.NitaqatNass"/> answering with
    /// anything at all means the work is done and re-parsing would find tags in
    /// a string that has none. Only when the container carries no spans for a
    /// string and the component's own rich-text switch is on is
    /// <see cref="Nasq.Hallil"/> called — the case of a translation delivered
    /// with markup still in it. Parsing it here rather than letting it reach
    /// the shaper is not a nicety: to the bidirectional algorithm a tag is not
    /// a tag, it is a run of mixed-direction characters, and
    /// <c>&lt;color=#ff0000&gt;</c> inside an Arabic sentence comes back out
    /// reversed and lodged in the middle of a word.
    /// </para>
    /// <para>
    /// <b>The two encodings.</b> Nasq answers in UTF-8 with byte offsets, and
    /// the layout call takes characters and re-encodes them. Round-tripping
    /// well-formed UTF-8 through UTF-16 and back is byte-identical, so the span
    /// offsets Nasq produced still name the same bytes after the layout call
    /// re-encodes — which is the property that lets the two contracts meet
    /// without either of them changing. Both conversions run over pooled
    /// buffers and allocate only when a string is longer than any string before
    /// it.
    /// </para>
    /// </remarks>
    public sealed class NassMuhaddar
    {
        private char[] huruf;
        private char[] naqi;
        private byte[] bayt;
        private TaaribNitaqUslub[] nitaqat;
        private DharraNasq[] dharrat;
        private SijillNasq[] sijillat;

        /// <summary>Creates the buffers with room for one short string.</summary>
        public NassMuhaddar()
        {
            huruf = new char[256];
            naqi = new char[256];
            bayt = new byte[512];
            nitaqat = new TaaribNitaqUslub[8];
            dharrat = new DharraNasq[8];
            sijillat = new SijillNasq[16];
        }

        /// <summary>
        /// Prepares one string of the patch for a layout call.
        /// </summary>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="fahras">The string's index, from <see cref="Ruqaa.JidNass"/>.</param>
        /// <param name="makhzan">
        /// The geometry buffers, whose span-conversion buffer is reused so the
        /// container's own span records do not need a second pool here.
        /// </param>
        /// <param name="lahja">
        /// The markup dialect of the component this string is drawn by, or
        /// <see cref="NawNasq.Bila"/> when its rich-text switch is off.
        /// </param>
        /// <param name="hajmAsas">
        /// The component's own size in pixels, which is what a relative size
        /// tag is relative to. Zero leaves relative sizes unresolved rather
        /// than inventing one.
        /// </param>
        /// <param name="nass">The text to lay out, in logical order.</param>
        /// <param name="nitaqatKharij">The style spans over its UTF-8 encoding.</param>
        /// <returns>Whether the string could be prepared at all.</returns>
        public bool Hayyi(
            Ruqaa ruqaa,
            int fahras,
            MakhzanRusum makhzan,
            NawNasq lahja,
            float hajmAsas,
            out ReadOnlySpan<char> nass,
            out ReadOnlySpan<TaaribNitaqUslub> nitaqatKharij)
        {
            nass = ReadOnlySpan<char>.Empty;
            nitaqatKharij = ReadOnlySpan<TaaribNitaqUslub>.Empty;

            ReadOnlySpan<byte> tarjama = ruqaa.Tarjama(fahras);
            if (tarjama.IsEmpty)
            {
                return false;
            }

            int tulHuruf = System.Text.Encoding.UTF8.GetCharCount(tarjama);
            if (huruf.Length < tulHuruf)
            {
                huruf = new char[Udlu(tulHuruf, huruf.Length)];
            }
            System.Text.Encoding.UTF8.GetChars(tarjama, huruf.AsSpan());
            ReadOnlySpan<char> kamil = new ReadOnlySpan<char>(huruf, 0, tulHuruf);

            ReadOnlySpan<MadkhalNitaq> madakhil = ruqaa.NitaqatNass(fahras);
            if (!madakhil.IsEmpty)
            {
                nass = kamil;
                nitaqatKharij = makhzan.Hawwil(madakhil);
                return true;
            }
            if (lahja == NawNasq.Bila)
            {
                nass = kamil;
                return true;
            }

            KhiyaratNasq khiyarat = KhiyaratNasq.Min(lahja);
            khiyarat.HajmAsas = hajmAsas;
            QiyasNasq matlub = Nasq.Ihsi(kamil, in khiyarat);
            if (matlub.AdadNitaqat == 0 && matlub.AdadDharrat == 0)
            {
                // Nothing was extracted, so the clean text is the input and the
                // second encoding is skipped entirely — the common case for a
                // translation that carries no markup at all.
                nass = kamil;
                return true;
            }

            if (bayt.Length < matlub.TulNass)
            {
                bayt = new byte[Udlu(matlub.TulNass, bayt.Length)];
            }
            if (nitaqat.Length < matlub.AdadNitaqat)
            {
                nitaqat = new TaaribNitaqUslub[Udlu(matlub.AdadNitaqat, nitaqat.Length)];
            }
            if (dharrat.Length < matlub.AdadDharrat)
            {
                dharrat = new DharraNasq[Udlu(matlub.AdadDharrat, dharrat.Length)];
            }
            if (sijillat.Length < matlub.AdadSijillat)
            {
                sijillat = new SijillNasq[Udlu(matlub.AdadSijillat, sijillat.Length)];
            }

            MakhzanNasq wia = default;
            wia.Nass = bayt.AsSpan();
            wia.Nitaqat = nitaqat.AsSpan();
            wia.Dharrat = dharrat.AsSpan();
            wia.Sijillat = sijillat.AsSpan();
            NatijaNasq natija = Nasq.Hallil(kamil, in khiyarat, wia);
            if (!natija.Kafa)
            {
                // Ihsi counted the identical walk a moment ago, so a shortfall
                // here is this file's bug rather than the string's. Falling
                // back to the unparsed translation draws the tags as text,
                // which is visible and reportable; dropping the string would
                // not be.
                Rabt.Ballagh(
                    "Nasq needed more room than Ihsi reported for a translated string; "
                    + "the markup is drawn as literal text for this one string.");
                nass = kamil;
                return true;
            }

            int tulNaqi = System.Text.Encoding.UTF8.GetCharCount(natija.Nass);
            if (naqi.Length < tulNaqi)
            {
                naqi = new char[Udlu(tulNaqi, naqi.Length)];
            }
            System.Text.Encoding.UTF8.GetChars(natija.Nass, naqi.AsSpan());
            nass = new ReadOnlySpan<char>(naqi, 0, tulNaqi);
            nitaqatKharij = new ReadOnlySpan<TaaribNitaqUslub>(nitaqat, 0, natija.Nitaqat.Length);
            return true;
        }

        private static int Udlu(int matlub, int hali)
        {
            long siaa = hali < 1 ? 1 : hali;
            while (siaa < matlub)
            {
                siaa *= 2;
            }
            return (int)siaa;
        }
    }

    /// <summary>
    /// وصل تكست ميش برو — every TextMeshPro type and member this takeover
    /// touches, resolved once at startup and held as delegates.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Nothing here throws when a member is missing. TextMeshPro has shipped as
    /// an asset-store package, as a built-in package, and as three major
    /// versions with renamed members in each, and a build missing one accessor
    /// should cost that one behaviour rather than the whole text system. A
    /// binding that could not resolve what it genuinely needs reports itself
    /// once, by name, and <see cref="Muakkad"/> answers <c>false</c>.
    /// </para>
    /// <para>
    /// <b>The patch targets are resolved declared-only.</b>
    /// <c>GenerateTextMesh</c> and <c>UpdateMaterial</c> are virtual on
    /// <c>TMP_Text</c> and overridden by both concrete classes; a patch on the
    /// base declaration would never run. See
    /// <see cref="RabtZaid.HadafMuarraf"/> for the failure that prevents.
    /// </para>
    /// </remarks>
    public sealed class WaslTmp
    {
        /// <summary>TextMeshPro's horizontal alignment bit for a left-aligned line.</summary>
        public const int MuhadhahaYasar = 1;

        /// <summary>Its bit for a centred line.</summary>
        public const int MuhadhahaWasat = 2;

        /// <summary>Its bit for a right-aligned line.</summary>
        public const int MuhadhahaYameen = 4;

        /// <summary>Its bit for a justified line.</summary>
        public const int MuhadhahaDabt = 8;

        /// <summary>Its bit for a flush line, which justifies the last line too.</summary>
        public const int MuhadhahaMustawi = 16;

        /// <summary>Its vertical bit for a top-aligned block.</summary>
        public const int RasiAla = 256;

        /// <summary>Its vertical bit for a vertically centred block.</summary>
        public const int RasiWasat = 512;

        /// <summary>Its vertical bit for a bottom-aligned block.</summary>
        public const int RasiAsfal = 1024;

        /// <summary>Its vertical bit for a baseline-aligned block.</summary>
        public const int RasiAsas = 2048;

        private WaslTmp()
        {
            NawNass = Rabt.Naw("TMPro.TMP_Text");
            NawSath = Rabt.Naw("TMPro.TextMeshProUGUI");
            NawAalam = Rabt.Naw("TMPro.TextMeshPro");
            NawFariSath = Rabt.Naw("TMPro.TMP_SubMeshUI");
            NawFariAalam = Rabt.Naw("TMPro.TMP_SubMesh");

            QariNass = Rabt.Qari<string>(NawNass, "text");
            QariHajm = Rabt.Qari<float>(NawNass, "fontSize");
            QariLawn = Rabt.Qari<Color>(NawNass, "color");
            QariMuhadhaha = Rabt.Qari<int>(NawNass, "alignment");
            QariHamish = Rabt.Qari<Vector4>(NawNass, "margin");
            QariMustatil = Rabt.Qari<RectTransform>(NawNass, "rectTransform");
            QariNasij = Rabt.Qari<Mesh>(NawNass, "mesh");
            QariNasqGhani = Rabt.Qari<bool>(NawNass, "richText");
            QariLaff = Rabt.Qari<bool>(NawNass, "enableWordWrapping");
            QariTabaudAhruf = Rabt.Qari<float>(NawNass, "characterSpacing");
            QariTabaudKalimat = Rabt.Qari<float>(NawNass, "wordSpacing");
            QariHajmTilqai = Rabt.Qari<bool>(NawNass, "enableAutoSizing");
            QariHajmAdna = Rabt.Qari<float>(NawNass, "fontSizeMin");
            AmrIttisakh = Rabt.Amr(NawNass, "SetAllDirty");

            HadafNasijSath = RabtZaid.HadafMuarraf(NawSath, "GenerateTextMesh");
            HadafNasijAalam = RabtZaid.HadafMuarraf(NawAalam, "GenerateTextMesh");
            HadafMaddaSath = RabtZaid.HadafMuarraf(NawSath, "UpdateMaterial");
            HadafMaddaAalam = RabtZaid.HadafMuarraf(NawAalam, "UpdateMaterial");
        }

        /// <summary>TextMeshPro's abstract text component, the base of both.</summary>
        public Type? NawNass { get; }

        /// <summary>The canvas-space component, which draws through a CanvasRenderer.</summary>
        public Type? NawSath { get; }

        /// <summary>The world-space component, which draws through a MeshRenderer.</summary>
        public Type? NawAalam { get; }

        /// <summary>The canvas-space sub-mesh component TextMeshPro splits into.</summary>
        public Type? NawFariSath { get; }

        /// <summary>The world-space sub-mesh component.</summary>
        public Type? NawFariAalam { get; }

        /// <summary>Reads the component's raw string, markup and all.</summary>
        public Func<object, string>? QariNass { get; }

        /// <summary>Reads its font size, in the component's own local units.</summary>
        public Func<object, float>? QariHajm { get; }

        /// <summary>Reads its colour, which every glyph inherits unless a span sets one.</summary>
        public Func<object, Color>? QariLawn { get; }

        /// <summary>
        /// Reads its alignment as an integer. The property is an enum whose two
        /// halves are packed into one word — see the constants above — and
        /// reading it as its underlying integer is what lets both halves be
        /// decoded without naming TextMeshPro's enums at compile time.
        /// </summary>
        public Func<object, int>? QariMuhadhaha { get; }

        /// <summary>Reads its margins as left, top, right, bottom.</summary>
        public Func<object, Vector4>? QariHamish { get; }

        /// <summary>Reads the rectangle it draws into.</summary>
        public Func<object, RectTransform>? QariMustatil { get; }

        /// <summary>
        /// Reads the mesh the component already owns. Taarib writes into this
        /// object rather than creating one, so the renderer the component
        /// wired up in Awake keeps pointing at the geometry it draws.
        /// </summary>
        public Func<object, Mesh>? QariNasij { get; }

        /// <summary>Whether the component parses markup in its string.</summary>
        public Func<object, bool>? QariNasqGhani { get; }

        /// <summary>Whether the component wraps at the rectangle's width.</summary>
        public Func<object, bool>? QariLaff { get; }

        /// <summary>Reads its extra letter spacing, in font units per em.</summary>
        public Func<object, float>? QariTabaudAhruf { get; }

        /// <summary>Reads its extra word spacing, in the same units.</summary>
        public Func<object, float>? QariTabaudKalimat { get; }

        /// <summary>Whether auto-sizing is on for this component.</summary>
        public Func<object, bool>? QariHajmTilqai { get; }

        /// <summary>The smallest size auto-sizing may use.</summary>
        public Func<object, float>? QariHajmAdna { get; }

        /// <summary>
        /// Marks the component dirty so the canvas rebuilds it. Called once
        /// when the takeover starts, so text already on screen is redrawn
        /// through Taarib instead of waiting for the game to change it.
        /// </summary>
        public Action<object>? AmrIttisakh { get; }

        /// <summary>The canvas-space mesh generation method, as a patch target.</summary>
        public MethodInfo? HadafNasijSath { get; }

        /// <summary>The world-space mesh generation method, as a patch target.</summary>
        public MethodInfo? HadafNasijAalam { get; }

        /// <summary>The canvas-space material assignment method, as a patch target.</summary>
        public MethodInfo? HadafMaddaSath { get; }

        /// <summary>The world-space material assignment method, as a patch target.</summary>
        public MethodInfo? HadafMaddaAalam { get; }

        /// <summary>
        /// Whether enough resolved to draw at all: the string, the mesh, the
        /// rectangle, the size, the colour, and at least one of the two mesh
        /// generation methods.
        /// </summary>
        public bool Muakkad =>
            NawNass is not null
            && QariNass is not null
            && QariNasij is not null
            && QariMustatil is not null
            && QariHajm is not null
            && QariLawn is not null
            && (HadafNasijSath is not null || HadafNasijAalam is not null);

        /// <summary>
        /// Binds TextMeshPro, or reports why it could not be bound and returns
        /// <c>null</c>.
        /// </summary>
        /// <returns>
        /// The binding, or <c>null</c> when this game has no TextMeshPro at
        /// all — which is the ordinary case for most games and is deliberately
        /// silent, because a log line per absent text system would fill a
        /// BepInEx log with notes about things that were never wrong.
        /// </returns>
        public static WaslTmp? Iqran()
        {
            if (Rabt.Naw("TMPro.TMP_Text") is null)
            {
                return null;
            }

            WaslTmp wasl = new WaslTmp();
            if (!wasl.Muakkad)
            {
                Rabt.Ballagh(
                    "TextMeshPro is present but its drawing members did not resolve; the "
                    + "TextMeshPro takeover is off and every other text system in this game "
                    + "is unaffected.");
                return null;
            }
            if (wasl.NawSath is not null && wasl.HadafNasijSath is null)
            {
                Rabt.Ballagh(
                    "TextMeshProUGUI does not declare GenerateTextMesh in this build; "
                    + "canvas-space TextMeshPro text is left in the game's own language "
                    + "and world-space text is unaffected.");
            }
            if (wasl.NawAalam is not null && wasl.HadafNasijAalam is null)
            {
                Rabt.Ballagh(
                    "TextMeshPro does not declare GenerateTextMesh in this build; "
                    + "world-space TextMeshPro text is left in the game's own language "
                    + "and canvas text is unaffected.");
            }
            if (wasl.QariMuhadhaha is null)
            {
                Rabt.Ballagh(
                    "TMP_Text.alignment did not resolve in this build; alignment for "
                    + "runtime-laid-out strings comes from the patch's constraint row only, "
                    + "which is right for every string the compiler measured and falls back "
                    + "to the leading edge for the rest.");
            }
            return wasl;
        }

        /// <summary>
        /// The horizontal alignment half of the component's packed alignment
        /// word, as the layout engine's own value.
        /// </summary>
        /// <param name="alignment">The packed word.</param>
        /// <returns>Where the line sits inside the available width.</returns>
        /// <remarks>
        /// Note what this does <em>not</em> do: it does not invert left and
        /// right for Arabic. <see cref="Muhadhaha.Bidaya"/> already means the
        /// leading edge, which is the right edge of a right-to-left line, so
        /// mapping the component's Left to Bidaya gives correct behaviour in
        /// both directions. Mapping it to an absolute edge instead, and then
        /// flipping it somewhere else, is how a label ends up correctly aligned
        /// in Arabic and mirrored in the English tooltip beside it.
        /// </remarks>
        public static Muhadhaha MuhadhahaMin(int alignment)
        {
            if ((alignment & MuhadhahaWasat) != 0)
            {
                return Muhadhaha.Wasat;
            }
            if ((alignment & (MuhadhahaDabt | MuhadhahaMustawi)) != 0)
            {
                return Muhadhaha.Dabt;
            }
            if ((alignment & MuhadhahaYameen) != 0)
            {
                return Muhadhaha.Nihaya;
            }
            return Muhadhaha.Bidaya;
        }

        /// <summary>
        /// The vertical alignment half of the packed word, as the value
        /// <see cref="HayyizRasm.Muhadhaha"/> takes.
        /// </summary>
        /// <param name="alignment">The packed word.</param>
        /// <returns>Where the block sits inside the rectangle.</returns>
        public static MuhadhahaRasiya RasiyaMin(int alignment)
        {
            if ((alignment & RasiWasat) != 0)
            {
                return MuhadhahaRasiya.Wasat;
            }
            if ((alignment & RasiAsfal) != 0)
            {
                return MuhadhahaRasiya.Asfal;
            }
            if ((alignment & RasiAsas) != 0)
            {
                return MuhadhahaRasiya.KhattAsas;
            }
            return MuhadhahaRasiya.Ala;
        }
    }

    /// <summary>
    /// نظام تكست ميش برو — the takeover itself: the patches, the interception,
    /// and the draw.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The four patch targets.</b>
    /// <c>TextMeshProUGUI.GenerateTextMesh</c> and
    /// <c>TextMeshPro.GenerateTextMesh</c> are where each component decides
    /// what to draw, and a prefix returning <c>false</c> is what stops TMP's
    /// own shaper from running for a string Taarib owns.
    /// <c>TextMeshProUGUI.UpdateMaterial</c> and
    /// <c>TextMeshPro.UpdateMaterial</c> are where each component assigns its
    /// font asset's material to its renderer, and they must be intercepted too:
    /// on the canvas path <c>UpdateMaterial</c> runs <em>after</em>
    /// <c>UpdateGeometry</c> in the same rebuild, so a takeover that only
    /// patched the geometry would put Taarib's vertices on screen with the
    /// game's font atlas bound, and every glyph would sample a rectangle
    /// belonging to a Latin letter.
    /// </para>
    /// <para>
    /// <b>The re-entrancy guard.</b> A patch that can trigger the thing it
    /// patches is a stack overflow inside somebody's game.
    /// <c>SetAllDirty</c> — which this takeover calls once when it starts, so
    /// text already on screen redraws through Taarib rather than waiting for
    /// the game to change it — schedules a canvas rebuild, and that rebuild
    /// calls <c>GenerateTextMesh</c>, and the prefix runs again. So the whole
    /// draw runs inside <c>hars</c>, a thread-static flag: while it is
    /// set every prefix returns <c>true</c> immediately, TMP's own path runs,
    /// and the recursion terminates at depth one. It is thread-static rather
    /// than a plain field because Unity's canvas rebuild can be driven from a
    /// job thread in some engine versions, and a shared flag would then have
    /// one text object's draw suppressing another's.
    /// </para>
    /// <para>
    /// <b>TMP_TextInfo is deliberately not written.</b> <c>characterInfo</c>,
    /// <c>meshInfo</c> and <c>lineInfo</c> are TMP's own staging for the mesh
    /// it is about to build; they are inputs to a pipeline that no longer runs
    /// here. Writing them would mean either a compile-time reference to
    /// <c>TMP_MeshInfo</c>, which this assembly must not have, or boxing a
    /// struct out of an array once per draw to reach its fields — paid for
    /// nothing, because the geometry is written one step later into the mesh
    /// the renderer actually receives. What that costs is honest and worth
    /// naming: game code that reads <c>textInfo.characterCount</c> or walks
    /// <c>characterInfo</c> to place its own marker sees TMP's last answer for
    /// the untranslated string. Effects driven that way are Phase 6's
    /// typewriter and auto-size work, which own those members; this file does
    /// not pretend to.
    /// </para>
    /// <para>
    /// <b>Sub-meshes.</b> TMP splits its geometry by material: a second font
    /// asset, a sprite asset or a fallback each get a <c>TMP_SubMeshUI</c> or
    /// <c>TMP_SubMesh</c> child with its own renderer. Taarib emits one atlas
    /// page through one material, so it needs no split — but the children TMP
    /// created for the untranslated string are still in the scene with their
    /// meshes populated, and a renderer nobody cleared goes on drawing. The
    /// symptom is the original English sitting behind the Arabic, faintly, in
    /// exactly the menus that had a fallback font. So every draw clears the
    /// sub-mesh children, and only those children: a child is cleared solely
    /// when it carries TMP's own sub-mesh component, so an ordinary sibling
    /// image parented under a label is never touched. When a layout genuinely
    /// reaches a second atlas page — which means the atlas budget in the
    /// patch's manifest is too small for the text on screen — the extra pages
    /// are reported once and their glyphs are not drawn, because two pages in
    /// one index buffer would draw one of them with the other's texture.
    /// </para>
    /// </remarks>
    public sealed class NizamTmp : IDisposable
    {
        [ThreadStatic]
        private static bool hars;

        private static NizamTmp? hali;

        private readonly WaslTmp wasl;
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
            talab.Nizam = NizamNass.TextMeshPro;
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
        /// <summary>
        /// Every component this takeover owns, and which atlas its mesh was
        /// last built from — the patch's or this session's.
        /// </summary>
        private readonly Dictionary<int, bool> mamlukat;
        private readonly Dictionary<MeshRenderer, Material> maddatAsliya;
        private readonly List<MethodInfo> hidaf;
        private readonly Harmony harmoni;

        private bool amil;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        private NizamTmp(
            Harmony harmoni,
            WaslTmp wasl,
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
            mamlukat = new Dictionary<int, bool>();
            maddatAsliya = new Dictionary<MeshRenderer, Material>();
            hidaf = new List<MethodInfo>(4);
            amil = true;
        }

        /// <summary>
        /// The live takeover the patch methods reach, or <c>null</c> when
        /// TextMeshPro was never taken over in this process.
        /// </summary>
        /// <remarks>
        /// A static because a Harmony patch method is static and receives only
        /// what Harmony injects. One takeover exists per process by
        /// construction — <see cref="Ibda"/> refuses a second — so this cannot
        /// become the wrong one.
        /// </remarks>
        public static NizamTmp? Hali => hali;

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <summary>
        /// Discovers TextMeshPro, installs the patches, and returns the
        /// takeover.
        /// </summary>
        /// <param name="harmoni">The plugin's Harmony instance.</param>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="masdar">The glyph source and its uploaded atlas.</param>
        /// <param name="siyaq">
        /// The engine context, for text the compiler never laid out. <c>null</c>
        /// leaves precomputed layouts working and declines every string with no
        /// precomputed layout at the size it is being drawn at.
        /// </param>
        /// <param name="silsila">The font chain runtime layout shapes with.</param>
        /// <param name="takhtit">
        /// A layout buffer used by this takeover and nothing else. A
        /// <see cref="Takhtit"/> hands its results back as spans over one
        /// reused pair of arrays, so sharing one with an input field would let
        /// a caret query overwrite the glyphs this file is halfway through
        /// turning into triangles.
        /// </param>
        /// <returns>
        /// The takeover, or <c>null</c> when this game has no TextMeshPro —
        /// which is the ordinary case for a great many games and is not an
        /// error.
        /// </returns>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="harmoni"/>, <paramref name="ruqaa"/> or
        /// <paramref name="masdar"/> is null.
        /// </exception>
        public static NizamTmp? Ibda(
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

            WaslTmp? wasl = WaslTmp.Iqran();
            if (wasl is null)
            {
                return null;
            }

            NizamTmp nizam = new NizamTmp(harmoni, wasl, masdar, ruqaa, siyaq, silsila, takhtit, jalsa);
            hali = nizam;

            // Which declarations the two takeover points resolved to, so a
            // build of TextMeshPro that moved generation into the base class —
            // where a declared-only lookup on the subclass finds nothing — is
            // named in the log instead of leaving every menu in English with
            // "installed" above it.

            bool shayun = false;
            shayun |= nizam.RakkibBaad(
                wasl.HadafNasijSath, typeof(TarqeeNasijSathBaad),
                nameof(TarqeeNasijSathBaad.Baad), "TextMeshProUGUI.GenerateTextMesh");
            shayun |= nizam.RakkibBaad(
                wasl.HadafNasijAalam, typeof(TarqeeNasijAalamBaad),
                nameof(TarqeeNasijAalamBaad.Baad), "TextMeshPro.GenerateTextMesh");
            nizam.Rakkib(
                wasl.HadafMaddaSath, typeof(TarqeeMaddaSath), nameof(TarqeeMaddaSath.Sabiq),
                "TextMeshProUGUI.UpdateMaterial");
            nizam.Rakkib(
                wasl.HadafMaddaAalam, typeof(TarqeeMaddaAalam), nameof(TarqeeMaddaAalam.Sabiq),
                "TextMeshPro.UpdateMaterial");

            if (!shayun)
            {
                Rabt.Ballagh(
                    "لم يُركَّب أي ترقيع على تكست ميش برو؛ نصوصه تبقى بلغة اللعبة الأصلية وبقية الأنظمة تعمل. | "
                    + "No TextMeshPro patch could be installed; its text stays in the game's "
                    + "original language and every other text system keeps working.");
                nizam.Dispose();
                return null;
            }
            return nizam;
        }

        /// <summary>
        /// Whether this takeover drew the component's current geometry, which
        /// is the question the material patches ask before they refuse to let
        /// TextMeshPro bind its own font atlas over Taarib's.
        /// </summary>
        /// <param name="mukawwin">The text component.</param>
        /// <returns>Whether Taarib owns this component's draw.</returns>
        public bool Yamlik(object? mukawwin)
        {
            if (!amil || mukawwin is not Component juz)
            {
                return false;
            }
            return mamlukat.ContainsKey(juz.GetInstanceID());
        }

        /// <summary>
        /// The whole interception: read the string, look it up, and either draw
        /// it or hand the component straight back to TextMeshPro.
        /// </summary>
        /// <param name="mukawwin">The text component being asked to draw.</param>
        /// <param name="sathi">
        /// Whether this is the canvas-space component. The two differ only in
        /// how the finished mesh reaches the GPU — a
        /// <see cref="CanvasRenderer"/> on one side and a
        /// <see cref="MeshFilter"/> with a <see cref="MeshRenderer"/> on the
        /// other — and in nothing else, which is why one draw serves both.
        /// </param>
        /// <returns>
        /// Whether Taarib drew. <c>false</c> means the component is untouched
        /// and TextMeshPro's own generation must run, which is the correct
        /// answer for every string this patch does not cover.
        /// </returns>
        public bool Yarsum(object? mukawwin, bool sathi)
        {
            if (!amil || hars || mukawwin is null || !masdar.Amil)
            {
                return false;
            }
            hars = true;
            try
            {
                return Rassim(mukawwin, sathi);
            }
            catch (KhataTaarib khata)
            {
                // Attributable to this one string: a layout that failed on a
                // player name is not a reason to stop translating the menu.
                Rabt.Ballagh("TextMeshPro: drawing one string failed", khata);
                Utruk(mukawwin);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("drawing a TextMeshPro component threw", khata);
                return false;
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// Binds Taarib's own material and atlas to a component's renderer,
        /// which is what the material patches do in place of TextMeshPro's own
        /// assignment.
        /// </summary>
        /// <param name="mukawwin">The text component.</param>
        /// <param name="sathi">Whether it is the canvas-space component.</param>
        /// <returns>Whether the material was bound.</returns>
        public bool Aabbir(object? mukawwin, bool sathi)
        {
            if (!amil || mukawwin is not Component juz
                || !mamlukat.TryGetValue(juz.GetInstanceID(), out bool minRuqaa))
            {
                return false;
            }
            // The material that goes with the atlas this component's mesh was
            // last built against; see the note on Wassil.
            Material? madda = masdar.Madda(minRuqaa, 0);
            if (madda is null)
            {
                return false;
            }
            try
            {
                if (sathi)
                {
                    CanvasRenderer? rassam = juz.GetComponent<CanvasRenderer>();
                    if (rassam is null)
                    {
                        return false;
                    }
                    rassam.materialCount = 1;
                    rassam.SetMaterial(madda, 0);
                    return true;
                }

                MeshRenderer? mubassir = juz.GetComponent<MeshRenderer>();
                if (mubassir is null)
                {
                    return false;
                }
                SajjilMaddaAsliya(mubassir);
                mubassir.sharedMaterial = madda;
                return true;
            }
            catch (Exception khata)
            {
                Awqif("binding Taarib's material to a TextMeshPro renderer threw", khata);
                return false;
            }
        }

        /// <summary>
        /// Marks one component dirty so the engine rebuilds it through Taarib.
        /// </summary>
        /// <remarks>
        /// Called for the text objects already on screen when the takeover is
        /// installed, because a component that is not dirty is not rebuilt and
        /// would keep drawing the untranslated mesh it generated before the
        /// patches existed — a title screen in English behind a fully working
        /// takeover. It runs inside <c>hars</c>: marking a component
        /// dirty schedules the very rebuild whose prefix this class installs,
        /// and without the guard the two would call each other.
        /// </remarks>
        /// <param name="mukawwin">The text component.</param>
        public void Ajjij(object? mukawwin)
        {
            Action<object>? ittisakh = wasl.AmrIttisakh;
            if (!amil || hars || mukawwin is null || ittisakh is null)
            {
                return;
            }
            hars = true;
            try
            {
                ittisakh(mukawwin);
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Marking a TextMeshPro component dirty failed", khata);
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// Removes this takeover's patches, gives each world-space renderer its
        /// own material back, and stops drawing. The game is then exactly as it
        /// was before the takeover, in its original language.
        /// </summary>
        /// <remarks>
        /// Each target is unpatched by name rather than through
        /// <c>UnpatchSelf</c>, because the plugin's Harmony instance is shared
        /// with every other text system's takeover and unpatching all of it
        /// would silently switch off the ones that were working.
        /// </remarks>
        public void Dispose()
        {
            amil = false;
            for (int i = 0; i < hidaf.Count; i++)
            {
                try
                {
                    harmoni.Unpatch(hidaf[i], HarmonyPatchType.Prefix, harmoni.Id);
                    harmoni.Unpatch(hidaf[i], HarmonyPatchType.Postfix, harmoni.Id);
                }
                catch (Exception khata)
                {
                    Rabt.Ballagh(
                        "Removing the patch on " + hidaf[i].Name + " failed", khata);
                }
            }
            hidaf.Clear();

            foreach (KeyValuePair<MeshRenderer, Material> zawj in maddatAsliya)
            {
                // A renderer destroyed with its scene compares equal to null
                // through Unity's own operator; assigning to it would throw a
                // MissingReferenceException during shutdown, which is the one
                // moment a plugin must not add an exception to the log.
                if (zawj.Key != null)
                {
                    zawj.Key.sharedMaterial = zawj.Value;
                }
            }
            maddatAsliya.Clear();
            mamlukat.Clear();
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
        }

        private bool Rassim(object mukawwin, bool sathi)
        {
            if (mukawwin is not Component juz)
            {
                return false;
            }

            string? khaam = wasl.QariNass!(mukawwin);
            if (string.IsNullOrEmpty(khaam))
            {
                Utruk(mukawwin);
                return false;
            }

            // MiftahMinNass encodes the string to UTF-8 to hash it. Under 512
            // bytes that encode is a stackalloc and nothing allocates; above
            // it, one byte array per call. A string that long is a paragraph of
            // dialogue, which is redrawn when it changes rather than per frame,
            // and refusing to hash it would mean refusing to translate exactly
            // the longest strings a patch exists for.
            ulong miftah = Ruqaa.MiftahMinNass(khaam!);

            // Capture mode observes and never replaces: the session records what
            // the game is about to draw and the game then draws it. This is the
            // only way text reaches a patch for a game whose components no
            // static reader can open — and on this backend the session existed,
            // was announced, and was never handed to a text system, so it
            // recorded nothing at all.
            if (jalsa is not null)
            {
                Iltaqit(khaam!, miftah, mukawwin as Component);
                Utruk(mukawwin);
                return false;
            }
            int fahras = ruqaa.JidNass(miftah);
            if (fahras < 0)
            {
                // A miss means the game is drawing something this patch does
                // not cover — a player name, a number, a string the compiler
                // never saw. Leave it alone: do not lay it out, do not draw it,
                // and do not guess at a translation. TextMeshPro renders it
                // exactly as it always did.
                Rabt.Fawt(khaam!, miftah);
                Utruk(mukawwin);
                return false;
            }

            // Counted so the log can answer "how much of this game does the
            // patch cover", which a list of misses alone cannot.
            Rabt.Isaba(miftah);

            Mesh? nasij = wasl.QariNasij!(mukawwin);
            RectTransform? mustatil = wasl.QariMustatil!(mukawwin);
            if (nasij is null || mustatil is null)
            {
                Utruk(mukawwin);
                return false;
            }

            float hajm = wasl.QariHajm!(mukawwin);
            if (!(hajm > 0f))
            {
                Utruk(mukawwin);
                return false;
            }

            Rect itar = mustatil.rect;
            Vector2 mihwar = mustatil.pivot;
            Func<object, Vector4>? qariHamish = wasl.QariHamish;
            Vector4 hamish = qariHamish is null ? Vector4.zero : qariHamish(mukawwin);
            float ardMutah = itar.width - hamish.x - hamish.z;
            float irtifaMutah = itar.height - hamish.y - hamish.w;
            ardMutah = ardMutah > 0f ? ardMutah : 0f;
            irtifaMutah = irtifaMutah > 0f ? irtifaMutah : 0f;

            Func<object, int>? qariMuhadhaha = wasl.QariMuhadhaha;
            int muhadhaha = qariMuhadhaha is null ? WaslTmp.RasiAla : qariMuhadhaha(mukawwin);
            Color lawnKamil = wasl.QariLawn!(mukawwin);

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            float hajmFili;
            float irtifaTakhtit;

            // The exact quarter-pixel size, never the nearest. Drawing a layout
            // measured at one size into a box the game sizes at another is how
            // text that fitted in the compiler's measurement overflows on a
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
                fahras, hajm, ardMutah, irtifaMutah, muhadhaha, mukawwin,
                out huruf, out sutur, out nitaqat, out hajmFili, out irtifaTakhtit))
            {
                Utruk(mukawwin);
                return false;
            }

            if (huruf.IsEmpty || !(hajmFili > 0f))
            {
                Utruk(mukawwin);
                return false;
            }

            float hajmLawha = masdar.HajmLawha(
                hajmFili, QiyasShasha.BikselLilWahda(mustatil), minRuqaa);
            bool tathbit = Nasij.YuthabbatQalam(masdar.Namat, hajmFili, hajmLawha);
            if (!minRuqaa && !masdar.Aqim(huruf, hajmLawha, tathbit))
            {
                Utruk(mukawwin);
                return false;
            }

            HayyizRasm hayyiz = default;
            // The pivot term is folded into the offsets rather than left in
            // MihwarS and MihwarA, because the pivot is expressed against the
            // full rectangle while Ard and Irtifa here are the box after
            // margins — and the vertical centring term reads Irtifa. Written
            // the other way, a middle-aligned label with a top margin sits half
            // the margin too high, which reads as a font metrics problem.
            hayyiz.Ard = ardMutah;
            hayyiz.Irtifa = irtifaMutah;
            hayyiz.MihwarS = 0f;
            hayyiz.MihwarA = 1f;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;
            hayyiz.IzahaS = (-mihwar.x * itar.width) + hamish.x;
            hayyiz.IzahaA = ((1f - mihwar.y) * itar.height) - hamish.y;
            hayyiz.Muhadhaha = WaslTmp.RasiyaMin(muhadhaha);

            TalabNasij talab = default;
            talab.Huruf = huruf;
            talab.Sutur = sutur;
            talab.Nitaqat = nitaqat;
            talab.Hayyiz = hayyiz;
            talab.Lawn = LawnRasm.Min(LawnMuazzam(lawnKamil));
            talab.Hajm = hajmFili;
            talab.HajmLawha = hajmLawha;
            talab.Safha = 0;
            // A coverage atlas holds one bitmap per size with the pen's
            // fractional position already folded into its coverage, so the quad
            // must land on the integer grid and the bucket must be the one the
            // rasterizer used — the compiler's and this file's HajmRubi and
            // Bakat are the same two functions for exactly that reason. A
            // distance-field atlas holds one bitmap for every size, so nothing
            // is snapped and every bucket is zero.
            talab.Tathbit = tathbit;

            makhzan.Wassi(huruf.Length);
            NatijaNasij natija = Nasij.Ibni(
                in talab, masdar.Khareeta(minRuqaa), makhzan.Makhzan());
            if (!natija.Kafa)
            {
                makhzan.Wassi(natija.MatlubRuus / Nasij.RuusLiShakl);
                natija = Nasij.Ibni(in talab, masdar.Khareeta(minRuqaa), makhzan.Makhzan());
                if (!natija.Kafa)
                {
                    Rabt.Ballagh(
                        "TextMeshPro: the mesh buffer refused a second time after growing to "
                        + "the size it asked for; this string is left to TextMeshPro.");
                    Utruk(mukawwin);
                    return false;
                }
            }

            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            makhzan.Amsah(in natija);
            Aktub(nasij, in natija);
            NazzifFuruu(juz, sathi);
            if (!Wassil(juz, nasij, sathi, minRuqaa))
            {
                Utruk(mukawwin);
                return false;
            }

            mamlukat[juz.GetInstanceID()] = minRuqaa;
            return true;
        }

        /// <summary>
        /// Lays a string out at run time, for a size the compiler never
        /// produced a layout at.
        /// </summary>
        /// <remarks>
        /// The options come from the patch's constraint row rather than from
        /// the component, and that is the whole point of the row existing: the
        /// compiler measured this slot and recorded the direction, the
        /// justification mode, the diacritics policy and the digits policy the
        /// translator chose for it. Taking them from the component instead
        /// would lay this string out with defaults and make it visibly disagree
        /// with the precomputed text beside it about all four. Only when there
        /// is no row at all does the component's own alignment stand in.
        /// </remarks>
        private bool Khattit(
            int fahras,
            float hajm,
            float ardMutah,
            float irtifaMutah,
            int muhadhaha,
            object mukawwin,
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

            Func<object, bool>? qariNasq = wasl.QariNasqGhani;
            bool nasqGhani = qariNasq is not null && qariNasq(mukawwin);
            NawNasq lahja = nasqGhani ? NawNasq.TextMeshPro : NawNasq.Bila;
            if (!muhaddar.Hayyi(
                ruqaa, fahras, makhzan, lahja, hajm,
                out ReadOnlySpan<char> nass, out ReadOnlySpan<TaaribNitaqUslub> nitaqatNass))
            {
                return false;
            }

            KhiyaratTakhtit khiyarat;
            float ardTalab = ardMutah;
            if (ruqaa.JidQayd(fahras, out MadkhalQayd qayd))
            {
                khiyarat = qayd.Khiyarat();
                if (qayd.ArdMutah > 0f)
                {
                    ardTalab = qayd.ArdMutah;
                }
                if (qayd.HajmTilqai && qayd.Hajm > 0f)
                {
                    hajmFili = qayd.Hajm;
                }
            }
            else
            {
                khiyarat = default;
                khiyarat.Muhadhaha = WaslTmp.MuhadhahaMin(muhadhaha);
                Func<object, bool>? qariLaff = wasl.QariLaff;
                if (qariLaff is not null && !qariLaff(mukawwin))
                {
                    khiyarat.Alam |= Alamat.KhiyarSatrWahid;
                }

                // TextMeshPro states both spacings in font units of one
                // hundredth of an em, which is a ratio and converts exactly;
                // its lineSpacing is an addition to a line height derived from
                // its own font asset, which does not, so it is left alone and
                // the font's own metrics decide. Inventing a pixel line height
                // from a number that means "a bit more than whatever that font
                // said" would space Arabic lines differently from the Latin
                // lines beside them, which is worse than not honouring it.
                Func<object, float>? qariAhruf = wasl.QariTabaudAhruf;
                if (qariAhruf is not null)
                {
                    khiyarat.TabaudAhruf = qariAhruf(mukawwin) * hajm * 0.01f;
                }
                Func<object, float>? qariKalimat = wasl.QariTabaudKalimat;
                if (qariKalimat is not null)
                {
                    khiyarat.TabaudKalimat = qariKalimat(mukawwin) * hajm * 0.01f;
                }

                Func<object, bool>? qariTilqai = wasl.QariHajmTilqai;
                Func<object, float>? qariAdna = wasl.QariHajmAdna;
                if (qariTilqai is not null && qariAdna is not null && qariTilqai(mukawwin))
                {
                    // Shrink-to-fit over Taarib's own measured widths rather
                    // than over TextMeshPro's binary search, which measures the
                    // unshaped string and would stop at a size the shaped text
                    // does not fit in.
                    khiyarat.Tajawuz = SiyasatTajawuz.Taqlis;
                    khiyarat.HajmAdna = qariAdna(mukawwin);
                }
            }
            if ((khiyarat.Alam & Alamat.KhiyarSatrWahid) != 0)
            {
                ardTalab = 0f;
            }

            TalabTakhtit talab = default;
            talab.Nass = nass;
            talab.Nitaqat = nitaqatNass;
            talab.Sifat = ReadOnlySpan<TaaribSifa>.Empty;
            talab.Hajm = hajmFili;
            talab.ArdMutah = ardTalab;
            talab.IrtifaMutah = irtifaMutah;
            talab.Khiyarat = khiyarat;
            buffer.Khattit(qab, chain, in talab);

            huruf = buffer.Huruf;
            sutur = buffer.Sutur;
            nitaqat = nitaqatNass;
            hajmFili = buffer.Hajm;
            irtifaTakhtit = buffer.Irtifa;
            return true;
        }

        /// <summary>
        /// Writes the built geometry into the mesh the component already owns.
        /// </summary>
        /// <remarks>
        /// <see cref="Mesh.Clear(bool)"/> first, because the indices still on
        /// the mesh describe the previous string and would name vertices past
        /// the end of the array being assigned. The bounds are set from
        /// <see cref="NatijaNasij.Hudud"/> rather than recalculated: the arrays
        /// are quantized and their cleared tail sits at the origin, so a
        /// recalculated box would stretch from the text to the pivot and the
        /// label would disappear at the edge of the view frustum.
        /// </remarks>
        private void Aktub(Mesh nasij, in NatijaNasij natija)
        {
            nasij.Clear(false);
            // Only what this string actually built. The buffers are grown to a
            // block size and reused, so assigning them whole hands the engine
            // the tail of the previous, longer string as well: its triangles
            // still index vertices this string never wrote, and R.E.P.O. drew
            // them as a black wedge across the menu.
            int adadRuus = natija.AdadRuus;
            // Six indices per glyph: the field counts indices, not triangles.
            int adadFahras = natija.AdadMuthallathat;
            nasij.SetVertices(makhzan.Ruus, 0, adadRuus);
            nasij.SetUVs(0, makhzan.Malamis, 0, adadRuus);
            nasij.SetColors(makhzan.Alwan, 0, adadRuus);
            nasij.SetIndices(
                makhzan.Muthallathat, 0, adadFahras, MeshTopology.Triangles, 0, false);
            nasij.bounds = new Bounds(
                new Vector3(
                    natija.Hudud.Yasar + (natija.Hudud.Ard * 0.5f),
                    natija.Hudud.Asfal + (natija.Hudud.Irtifa * 0.5f),
                    0f),
                new Vector3(natija.Hudud.Ard, natija.Hudud.Irtifa, 0f));
        }

        /// <summary>
        /// Binds the mesh, and the material and atlas page it was built
        /// against.
        /// </summary>
        /// <remarks>
        /// <paramref name="minRuqaa"/> is not a preference. A mesh laid out
        /// from the patch indexes the patch's own atlas, and one laid out at
        /// runtime indexes the atlas this session rasterized into; the two are
        /// different pictures with different glyphs at different coordinates.
        /// Binding the patch's texture to a runtime-laid mesh — which is what
        /// preferring the patch here used to do — samples whatever happens to
        /// sit at those coordinates, and R.E.P.O. drew every translated line as
        /// a row of solid white blocks because of it.
        /// </remarks>
        private bool Wassil(Component juz, Mesh nasij, bool sathi, bool minRuqaa)
        {
            Material? madda = masdar.Madda(minRuqaa, 0);
            Texture2D? lawha = masdar.LawhatSafha(minRuqaa, 0);
            if (madda is null)
            {
                return false;
            }

            if (sathi)
            {
                CanvasRenderer? rassam = juz.GetComponent<CanvasRenderer>();
                if (rassam is null)
                {
                    return false;
                }
                rassam.SetMesh(nasij);
                rassam.materialCount = 1;
                rassam.SetMaterial(madda, 0);
                if (lawha is not null)
                {
                    rassam.SetTexture(lawha);
                }
                return true;
            }

            MeshFilter? murashshih = juz.GetComponent<MeshFilter>();
            MeshRenderer? mubassir = juz.GetComponent<MeshRenderer>();
            if (murashshih is null || mubassir is null)
            {
                return false;
            }
            if (!ReferenceEquals(murashshih.sharedMesh, nasij))
            {
                murashshih.sharedMesh = nasij;
            }
            SajjilMaddaAsliya(mubassir);
            mubassir.sharedMaterial = madda;
            return true;
        }

        /// <summary>
        /// Clears the sub-mesh children TextMeshPro created for the string it
        /// drew before Taarib took over. Only a child carrying TMP's own
        /// sub-mesh component is touched, so an ordinary image or label
        /// parented under a text object is never cleared.
        /// </summary>
        private void NazzifFuruu(Component juz, bool sathi)
        {
            Type? nawFari = sathi ? wasl.NawFariSath : wasl.NawFariAalam;
            if (nawFari is null)
            {
                return;
            }
            Transform jidhr = juz.transform;
            int adad = jidhr.childCount;
            for (int i = 0; i < adad; i++)
            {
                Transform tifl = jidhr.GetChild(i);
                if (tifl.GetComponent(nawFari) is null)
                {
                    continue;
                }
                if (sathi)
                {
                    CanvasRenderer? rassam = tifl.GetComponent<CanvasRenderer>();
                    if (rassam is not null)
                    {
                        rassam.Clear();
                    }
                    continue;
                }
                MeshFilter? murashshih = tifl.GetComponent<MeshFilter>();
                Mesh? fari = murashshih is null ? null : murashshih.sharedMesh;
                if (fari is not null)
                {
                    // TMP's own transient render data, which TMP regenerates
                    // the moment its generation runs again. Nothing serialized
                    // is altered.
                    fari.Clear(false);
                }
            }
        }

        /// <summary>
        /// Remembers a world-space renderer's own material the first time
        /// Taarib replaces it, so <see cref="Dispose"/> can give it back. The
        /// renderer itself is the key rather than its instance identifier,
        /// because an identifier cannot be turned back into the object it
        /// named and a restore that cannot reach the renderer is not a restore.
        /// </summary>
        private void SajjilMaddaAsliya(MeshRenderer mubassir)
        {
            if (maddatAsliya.ContainsKey(mubassir))
            {
                return;
            }
            Material? asliya = mubassir.sharedMaterial;
            if (asliya is not null)
            {
                maddatAsliya[mubassir] = asliya;
            }
        }

        private void Utruk(object mukawwin)
        {
            if (mukawwin is Component juz)
            {
                mamlukat.Remove(juz.GetInstanceID());
            }
        }

        /// <summary>
        /// Installs a takeover that runs <em>after</em> the engine's own
        /// generation rather than in place of it.
        /// </summary>
        /// <remarks>
        /// Skipping <c>GenerateTextMesh</c> leaves TextMeshPro's own dirty
        /// flags set, and a canvas whose element never reports itself clean is
        /// rebuilt again inside the same frame, for ever: R.E.P.O. spins at
        /// full load and never presents another frame. Letting the engine
        /// generate and replacing the mesh afterwards costs one layout the
        /// player never sees and keeps every field the game itself reads —
        /// `textInfo`, the character count a menu animates over — consistent.
        /// </remarks>
        private bool RakkibBaad(MethodInfo? hadaf, Type hamil, string ism, string wasf)
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
                harmoni.Patch(hadaf, postfix: new HarmonyMethod(tariqa));
                hidaf.Add(hadaf);
                return true;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(
                    "Patching " + wasf + " failed; that one interception is off and the rest "
                    + "of the TextMeshPro takeover continues", khata);
                return false;
            }
        }

        private bool Rakkib(MethodInfo? hadaf, Type hamil, string ism, string wasf)
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
                harmoni.Patch(hadaf, prefix: new HarmonyMethod(tariqa));
                hidaf.Add(hadaf);
                return true;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(
                    "Patching " + wasf + " failed; that one interception is off and the rest "
                    + "of the TextMeshPro takeover continues", khata);
                return false;
            }
        }

        /// <summary>
        /// The component's colour as the packed word
        /// <see cref="LawnRasm.Min"/> unpacks: red in the high byte, alpha in
        /// the low one. The order is the ABI's and it is not Unity's, which is
        /// exactly why this conversion lives in one named place rather than
        /// being open coded with the shifts written from memory.
        /// </summary>
        private static uint LawnMuazzam(Color lawn)
        {
            Color32 mudmaj = lawn;
            return ((uint)mudmaj.r << 24) | ((uint)mudmaj.g << 16)
                | ((uint)mudmaj.b << 8) | mudmaj.a;
        }

        private void BallighSafahat()
        {
            if (ballaghSafahat)
            {
                return;
            }
            ballaghSafahat = true;
            Rabt.Ballagh(
                "تخطيط تكست ميش برو يمسّ أكثر من صفحة لوحة واحدة؛ رُسمت الصفحة الأولى فقط، وميزانية اللوحة في الرقعة أصغر مما يحتاجه النص المعروض. | "
                + "A TextMeshPro layout touches more than one atlas page; only the first was "
                + "drawn, and the patch's atlas budget is smaller than the text on screen "
                + "needs. Glyphs on the other pages are missing rather than drawn with the "
                + "wrong texture.");
        }

        private void BallighTakhtit()
        {
            if (ballaghTakhtit)
            {
                return;
            }
            ballaghTakhtit = true;
            Rabt.Ballagh(
                "لا يوجد سياق تخطيط حيّ، فالنصوص التي لم يُخطّطها المُصرِّف عند هذا الحجم تبقى بلغة اللعبة. | "
                + "There is no live layout context, so a string the compiler did not lay out "
                + "at the size it is being drawn at is left in the game's own language "
                + "rather than laid out with default options that would disagree with the "
                + "text beside it.");
        }

        private void Awqif(string sabab, Exception khata)
        {
            if (!amil)
            {
                return;
            }
            amil = false;
            mamlukat.Clear();
            Rabt.Ballagh(
                "أُوقف الاستيلاء على تكست ميش برو: " + sabab + "؛ عادت نصوصه إلى لغة اللعبة وبقية الأنظمة تعمل. | "
                + "The TextMeshPro takeover switched itself off: " + sabab
                + "; its text is back in the game's own language and every other text system "
                + "keeps working.",
                khata);
        }
    }

    /// <summary>
    /// The takeover that runs after <c>TMPro.TextMeshProUGUI.GenerateTextMesh</c>,
    /// replacing the mesh the engine just built.
    /// </summary>
    public static class TarqeeNasijSathBaad
    {
        /// <param name="__instance">The component, injected by Harmony.</param>
        public static void Baad(object __instance)
        {
            NizamTmp.Hali?.Yarsum(__instance, sathi: true);
        }
    }

    /// <summary>The same, for the world-space component.</summary>
    public static class TarqeeNasijAalamBaad
    {
        /// <param name="__instance">The component, injected by Harmony.</param>
        public static void Baad(object __instance)
        {
            NizamTmp.Hali?.Yarsum(__instance, sathi: false);
        }
    }

    /// <summary>
    /// ترقيع مادة السطح — the prefix on
    /// <c>TMPro.TextMeshProUGUI.UpdateMaterial</c>.
    /// </summary>
    /// <remarks>
    /// On the canvas path this runs after the geometry in the same rebuild. Let
    /// it through and TextMeshPro binds its own font atlas over Taarib's
    /// vertices, and every glyph then samples the rectangle of whichever Latin
    /// letter the game's atlas happens to hold there.
    /// </remarks>
    public static class TarqeeMaddaSath
    {
        /// <summary>Binds Taarib's material instead of the font asset's.</summary>
        /// <param name="__instance">The component, injected by Harmony.</param>
        /// <returns><c>false</c> when Taarib owns this component's draw.</returns>
        public static bool Sabiq(object __instance)
        {
            NizamTmp? nizam = NizamTmp.Hali;
            return nizam is null || !nizam.Aabbir(__instance, sathi: true);
        }
    }

    /// <summary>
    /// ترقيع مادة العالم — the prefix on
    /// <c>TMPro.TextMeshPro.UpdateMaterial</c>.
    /// </summary>
    public static class TarqeeMaddaAalam
    {
        /// <summary>Binds Taarib's material instead of the font asset's.</summary>
        /// <param name="__instance">The component, injected by Harmony.</param>
        /// <returns><c>false</c> when Taarib owns this component's draw.</returns>
        public static bool Sabiq(object __instance)
        {
            NizamTmp? nizam = NizamTmp.Hali;
            return nizam is null || !nizam.Aabbir(__instance, sathi: false);
        }
    }
}
