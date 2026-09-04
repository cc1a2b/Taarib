// نظام واجهة يونتي — the legacy UnityEngine.UI.Text takeover.
//
// uGUI's text path is older and simpler than TextMeshPro's, and it fails at
// Arabic in a different place. A Text component asks a TextGenerator to lay the
// string out; the generator asks the Font for a bitmap per character out of a
// dynamic atlas the engine manages, and writes one UIVertex quad per character
// into a list the graphic then pours into a mesh. There is no OpenType layout
// anywhere in that chain — no GSUB, so no contextual forms and no lam-alef
// ligature; no GPOS, so no mark attachment; no bidirectional algorithm, so a
// mixed sentence stays in logical order.
//
// WHY Font.RequestCharactersInTexture IS NOT USED, AND MUST NOT BE. That call
// is the whole of the engine's dynamic font atlas: hand it a string and a size
// and it rasterizes one bitmap per CHARACTER into a texture it owns, then
// raises Font.textureRebuilt when the atlas moves and every glyph's texture
// coordinates change. Two things make it useless here and one makes it
// actively wrong. It is keyed by codepoint, so what comes back for an Arabic
// letter is that letter's ISOLATED form — the shape a letter has standing
// alone, which is the wrong shape in the overwhelming majority of words, and
// the exact defect that makes so-called Arabic support in Unity games read as
// disconnected letters. It has no notion of a mark's attachment point, so
// tashkeel lands on the baseline instead of on the letter it belongs to. And
// it belongs to a Font asset of the game's, which Decision 5 forbids reading.
// So Taarib rasterizes its own glyphs, from its own validated bundled font,
// into its own atlas, and never subscribes to Font.textureRebuilt — a takeover
// that did would be rebuilding coordinates into a texture it does not draw.
//
// WHY THE SEAM IS OnPopulateMesh. Graphic.DoMeshGeneration calls
// OnPopulateMesh(VertexHelper) to get the geometry, then runs every
// IMeshModifier on the object — that is what Outline, Shadow, PositionAsUV1 and
// every BaseMeshEffect subclass a game ships are — and only then pours the
// helper into the mesh the CanvasRenderer receives. Filling the helper from a
// prefix and returning false therefore puts Taarib's vertices exactly where the
// engine expects a graphic's own vertices to be, and every mesh effect in the
// game keeps working over the Arabic instead of being bypassed. Patching
// further down, at TextGenerator.Populate, would be the wrong seam twice over:
// the generator's output is character quads that would still have to be thrown
// away, and the modifiers would then run over vertices some other code already
// replaced.
//
// THE COMPILE-TIME RULE. UnityEngine.UI is a package rather than an engine
// module, so Text, Graphic and VertexHelper are not in UnityEngine.Modules and
// this assembly cannot name them even if it wanted to. They are resolved by
// name through reflection, once, at startup, and cached as strongly typed
// delegates. UIVertex, Color32, Vector3 and CanvasRenderer are engine module
// types and are named directly.

using System;
using System.Collections.Generic;
using System.Reflection;
using HarmonyLib;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mono.Suluk;
using Taarib.Unity.Mushtarak;
using UnityEngine;

namespace Taarib.Unity.Mono.Anzimat
{
    /// <summary>
    /// وصل واجهة يونتي — every legacy uGUI type and member this takeover
    /// touches, resolved once at startup and held as delegates.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Two of these bindings decide whether the takeover can run at all: the
    /// <c>OnPopulateMesh</c> patch target, without which there is no
    /// interception, and one of the two ways of filling a
    /// <c>VertexHelper</c>. Everything else degrades on its own — a build with
    /// no <c>supportRichText</c> loses markup handling for uGUI text and
    /// nothing else.
    /// </para>
    /// <para>
    /// <b>Two fill paths, and why both are bound.</b>
    /// <c>VertexHelper.AddUIVertexStream(List&lt;UIVertex&gt;, List&lt;int&gt;)</c>
    /// appends a whole mesh in one call and is what every version since 5.2
    /// has; <c>AddVert</c> plus <c>AddTriangle</c> is the older per-vertex
    /// form. The stream is preferred because it is one delegate invocation for
    /// a whole label rather than ten per glyph, and the per-vertex pair is kept
    /// as the fallback so a build missing the stream method loses throughput
    /// rather than the text.
    /// </para>
    /// </remarks>
    public sealed class WaslWajiha
    {
        /// <summary>
        /// <c>HorizontalWrapMode.Overflow</c>, the value that means the
        /// component refuses to wrap.
        /// </summary>
        public const int TajawuzUfuqi = 1;

        private WaslWajiha()
        {
            Type? nawMusaid = Rabt.Naw("UnityEngine.UI.VertexHelper");
            NawNass = Rabt.Naw("UnityEngine.UI.Text");
            NawRasm = Rabt.Naw("UnityEngine.UI.Graphic");
            NawMusaid = nawMusaid;

            QariNass = Rabt.Qari<string>(NawNass, "text");
            QariHajm = Rabt.Qari<int>(NawNass, "fontSize");
            QariLawn = Rabt.Qari<Color>(NawNass, "color");
            QariNasqGhani = Rabt.Qari<bool>(NawNass, "supportRichText");
            QariMuhadhaha = Rabt.Qari<int>(NawNass, "alignment");
            QariTajawuzUfuqi = Rabt.Qari<int>(NawNass, "horizontalOverflow");
            QariMustatil = Rabt.Qari<RectTransform>(NawNass, "rectTransform");
            QariMadda = Rabt.Qari<Material>(NawNass, "material");
            KatibMadda = Rabt.Katib<Material>(NawNass, "material");
            AmrIttisakhRuus = Rabt.Amr(NawNass, "SetVerticesDirty");
            AmrIttisakhMadda = Rabt.Amr(NawNass, "SetMaterialDirty");

            AmrMasah = Rabt.Amr(nawMusaid, "Clear");
            AmrTayyar = RabtZaid.Amr<List<UIVertex>, List<int>>(
                nawMusaid, "AddUIVertexStream");
            AmrRas = RabtZaid.Amr<Vector3, Color32, Vector2>(nawMusaid, "AddVert");
            AmrMuthallath = RabtZaid.Amr<int, int, int>(nawMusaid, "AddTriangle");

            HadafMala = nawMusaid is null
                ? null
                : RabtZaid.HadafMuarraf(NawNass, "OnPopulateMesh", nawMusaid);
            HadafLawha = RabtZaid.HadafKhasiya(NawNass, "mainTexture", katib: false);
        }

        /// <summary>uGUI's text component.</summary>
        public Type? NawNass { get; }

        /// <summary>Its base class, which owns the material and the rectangle.</summary>
        public Type? NawRasm { get; }

        /// <summary>The vertex accumulator a graphic is asked to fill.</summary>
        public Type? NawMusaid { get; }

        /// <summary>Reads the component's raw string, markup and all.</summary>
        public Func<object, string>? QariNass { get; }

        /// <summary>Reads its font size in points, which for a canvas is pixels.</summary>
        public Func<object, int>? QariHajm { get; }

        /// <summary>Reads its colour, which every glyph inherits unless a span sets one.</summary>
        public Func<object, Color>? QariLawn { get; }

        /// <summary>Whether the component parses markup in its string.</summary>
        public Func<object, bool>? QariNasqGhani { get; }

        /// <summary>
        /// Reads its anchor as an integer. <c>TextAnchor</c> packs both axes
        /// into one value — the horizontal edge is the remainder modulo three
        /// and the vertical edge is the quotient — and reading it as its
        /// underlying integer is what lets both be decoded without depending on
        /// the enum being present under that name.
        /// </summary>
        public Func<object, int>? QariMuhadhaha { get; }

        /// <summary>Whether the component wraps at the rectangle's width.</summary>
        public Func<object, int>? QariTajawuzUfuqi { get; }

        /// <summary>Reads the rectangle it draws into.</summary>
        public Func<object, RectTransform>? QariMustatil { get; }

        /// <summary>Reads the material the graphic currently draws with.</summary>
        public Func<object, Material>? QariMadda { get; }

        /// <summary>
        /// Assigns the material the graphic draws with. This is how Taarib's
        /// own shader reaches the canvas: the component's own material is
        /// recorded first and given back when the takeover is removed.
        /// </summary>
        public Action<object, Material>? KatibMadda { get; }

        /// <summary>Marks the graphic's geometry dirty.</summary>
        public Action<object>? AmrIttisakhRuus { get; }

        /// <summary>Marks the graphic's material dirty.</summary>
        public Action<object>? AmrIttisakhMadda { get; }

        /// <summary>Empties the vertex accumulator before it is refilled.</summary>
        public Action<object>? AmrMasah { get; }

        /// <summary>Appends a whole mesh to the accumulator in one call.</summary>
        public Action<object, List<UIVertex>, List<int>>? AmrTayyar { get; }

        /// <summary>Appends one vertex, for a build with no stream method.</summary>
        public Action<object, Vector3, Color32, Vector2>? AmrRas { get; }

        /// <summary>Appends one triangle, for the same builds.</summary>
        public Action<object, int, int, int>? AmrMuthallath { get; }

        /// <summary>
        /// <c>Text.OnPopulateMesh(VertexHelper)</c>, as a patch target. This is
        /// the interception point; without it there is no takeover.
        /// </summary>
        public MethodInfo? HadafMala { get; }

        /// <summary>
        /// <c>Text.mainTexture</c>'s getter, as a patch target. uGUI answers
        /// that property with the font's own atlas texture, and the canvas
        /// binds whatever it answers; a postfix is what puts Taarib's atlas
        /// there instead for the components Taarib drew.
        /// </summary>
        public MethodInfo? HadafLawha { get; }

        /// <summary>
        /// Whether enough resolved to draw at all: the string, the rectangle,
        /// the size, the colour, the interception point, and some way of
        /// filling the accumulator.
        /// </summary>
        public bool Muakkad =>
            NawNass is not null
            && NawMusaid is not null
            && HadafMala is not null
            && QariNass is not null
            && QariMustatil is not null
            && QariHajm is not null
            && QariLawn is not null
            && AmrMasah is not null
            && (AmrTayyar is not null
                || (AmrRas is not null && AmrMuthallath is not null));

        /// <summary>
        /// Binds uGUI's text path, or reports why it could not be bound and
        /// returns <c>null</c>.
        /// </summary>
        /// <returns>
        /// The binding, or <c>null</c> when this game has no legacy uGUI text —
        /// deliberately silent in that case, because a game built entirely on
        /// TextMeshPro is the ordinary modern case and is not a fault worth a
        /// line in anybody's log.
        /// </returns>
        public static WaslWajiha? Iqran()
        {
            if (Rabt.Naw("UnityEngine.UI.Text") is null)
            {
                return null;
            }

            WaslWajiha wasl = new WaslWajiha();
            if (!wasl.Muakkad)
            {
                Rabt.Ballagh(
                    "UnityEngine.UI.Text is present but its drawing members did not resolve; "
                    + "the legacy uGUI takeover is off and every other text system in this "
                    + "game is unaffected.");
                return null;
            }
            if (wasl.HadafLawha is null)
            {
                Rabt.Ballagh(
                    "Text.mainTexture did not resolve in this build; uGUI text is drawn "
                    + "through Taarib's material, which carries the atlas itself, and the "
                    + "canvas's own texture binding is left as the game set it.");
            }
            if (wasl.KatibMadda is null)
            {
                Rabt.Ballagh(
                    "Graphic.material has no setter in this build; uGUI text cannot be given "
                    + "Taarib's shader and the legacy takeover would draw a single-channel "
                    + "atlas through a four-channel shader, so it is off.");
                return null;
            }
            if (wasl.AmrTayyar is null)
            {
                Rabt.Ballagh(
                    "VertexHelper.AddUIVertexStream did not resolve in this build; uGUI text "
                    + "is filled one vertex at a time instead, which is correct and slower.");
            }
            return wasl;
        }

        /// <summary>
        /// The horizontal half of a <c>TextAnchor</c>, as the layout engine's
        /// own value.
        /// </summary>
        /// <param name="anchor">The anchor value.</param>
        /// <returns>Where the line sits inside the available width.</returns>
        /// <remarks>
        /// Left maps to <see cref="Muhadhaha.Bidaya"/> rather than to an
        /// absolute edge, because Bidaya already means the leading edge — which
        /// is the right edge of a right-to-left line. Mapping it to an absolute
        /// edge and flipping it somewhere else is how a label ends up correctly
        /// aligned in Arabic and mirrored in the English label beside it.
        /// </remarks>
        public static Muhadhaha MuhadhahaMin(int anchor)
        {
            int ufuqi = ((anchor % 3) + 3) % 3;
            if (ufuqi == 1)
            {
                return Muhadhaha.Wasat;
            }
            return ufuqi == 2 ? Muhadhaha.Nihaya : Muhadhaha.Bidaya;
        }

        /// <summary>
        /// The vertical half of a <c>TextAnchor</c>, as the value
        /// <see cref="HayyizRasm.Muhadhaha"/> takes.
        /// </summary>
        /// <param name="anchor">The anchor value.</param>
        /// <returns>Where the block sits inside the rectangle.</returns>
        public static MuhadhahaRasiya RasiyaMin(int anchor)
        {
            int rasi = anchor < 0 ? 0 : anchor / 3;
            if (rasi == 1)
            {
                return MuhadhahaRasiya.Wasat;
            }
            return rasi >= 2 ? MuhadhahaRasiya.Asfal : MuhadhahaRasiya.Ala;
        }
    }

    /// <summary>
    /// نظام واجهة يونتي — the legacy uGUI takeover: the patches, the
    /// interception, and the fill.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The two patch targets.</b>
    /// <c>UnityEngine.UI.Text.OnPopulateMesh(VertexHelper)</c> is where a
    /// graphic hands its geometry to the canvas, and a prefix that fills the
    /// helper and returns <c>false</c> replaces that geometry while leaving
    /// every <c>IMeshModifier</c> on the object running afterwards over the
    /// result — which is exactly what a game's Outline or Shadow component is,
    /// and exactly what a takeover further down the stack would have silently
    /// bypassed. <c>Text.mainTexture</c>'s getter is the second: uGUI answers
    /// that property with the font's own dynamic atlas and the canvas binds
    /// whatever it answers, so without the postfix Taarib's vertices would be
    /// drawn sampling the engine's font texture.
    /// </para>
    /// <para>
    /// <b>The material.</b> Taarib's atlas is a single-channel <c>R8</c>
    /// texture and uGUI's stock shader samples all four channels, so the
    /// component's material is replaced with Taarib's the first time it is
    /// drawn — and its own material is recorded and given back when the
    /// takeover is removed, so a disabled patch leaves the scene exactly as it
    /// found it. Assigning a material marks the graphic's material dirty, which
    /// schedules a rebuild, so the assignment happens only when the material is
    /// not already Taarib's: written the other way it would dirty the graphic
    /// from inside the graphic's own rebuild, once per frame, forever.
    /// </para>
    /// <para>
    /// <b>Rich text.</b> When <c>supportRichText</c> is on, the string the
    /// component holds carries markup, and markup must be lifted into spans
    /// before anything directional looks at the text: to the bidirectional
    /// algorithm a tag is not a tag, it is a run of mixed-direction characters,
    /// and <c>&lt;color=#ff0000&gt;</c> inside an Arabic sentence comes back
    /// reversed and lodged in the middle of a word. That lifting belongs to
    /// <see cref="Nasq"/> and is called rather than reimplemented — with
    /// <see cref="NawNasq.WajihatUnity"/>, not TextMeshPro's dialect, because
    /// the two genuinely disagree: <c>&lt;sprite=3&gt;</c> is a sprite in one
    /// and eleven characters a player is meant to read in the other. See
    /// <see cref="NassMuhaddar"/> for when the call is skipped, which is
    /// whenever the patch compiler already did the work.
    /// </para>
    /// <para>
    /// <b>What is knowingly left behind.</b> <c>Text.preferredWidth</c> and
    /// <c>preferredHeight</c> still come from the engine's own
    /// <c>TextGenerator</c> over the untranslated string, so a
    /// <c>ContentSizeFitter</c> above a taken-over label sizes it against Latin
    /// metrics. Correcting that is container reflow's job and it reads the
    /// patch's own measurements; this file does not pretend the generator's
    /// answer is right, it simply does not draw from it.
    /// </para>
    /// </remarks>
    public sealed class NizamWajiha : IDisposable
    {
        [ThreadStatic]
        private static bool hars;

        private static NizamWajiha? hali;

        private readonly WaslWajiha wasl;
        private readonly MasdarAshkal masdar;
        private readonly Ruqaa ruqaa;
        private readonly Takhtit? takhtit;
        private readonly MaqbadSiyaq? siyaq;
        private readonly MaqbadSilsila? silsila;
        private readonly MakhzanRusum makhzan;
        private readonly NassMuhaddar muhaddar;
        private readonly HashSet<int> mamlukat;
        private readonly Dictionary<Component, Material> maddatAsliya;
        private readonly List<UIVertex> ruusWajiha;
        private readonly List<int> fahrasWajiha;
        private readonly List<MethodInfo> hidaf;
        private readonly Harmony harmoni;

        private bool tayyarMumkin;
        private bool amil;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        private NizamWajiha(
            Harmony harmoni,
            WaslWajiha wasl,
            MasdarAshkal masdar,
            Ruqaa ruqaa,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit)
        {
            this.harmoni = harmoni;
            this.wasl = wasl;
            this.masdar = masdar;
            this.ruqaa = ruqaa;
            this.siyaq = siyaq;
            this.silsila = silsila;
            this.takhtit = takhtit;
            makhzan = new MakhzanRusum();
            muhaddar = new NassMuhaddar();
            mamlukat = new HashSet<int>();
            maddatAsliya = new Dictionary<Component, Material>();
            ruusWajiha = new List<UIVertex>(256);
            fahrasWajiha = new List<int>(384);
            hidaf = new List<MethodInfo>(2);
            tayyarMumkin = wasl.AmrTayyar is not null;
            amil = true;
        }

        /// <summary>
        /// The live takeover the patch methods reach, or <c>null</c> when uGUI
        /// text was never taken over in this process.
        /// </summary>
        public static NizamWajiha? Hali => hali;

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <summary>
        /// Discovers uGUI's text path, installs the patches, and returns the
        /// takeover.
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
        /// The takeover, or <c>null</c> when this game has no legacy uGUI text,
        /// which is not an error.
        /// </returns>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="harmoni"/>, <paramref name="ruqaa"/> or
        /// <paramref name="masdar"/> is null.
        /// </exception>
        public static NizamWajiha? Ibda(
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

            WaslWajiha? wasl = WaslWajiha.Iqran();
            if (wasl is null)
            {
                return null;
            }

            NizamWajiha nizam = new NizamWajiha(
                harmoni, wasl, masdar, ruqaa, siyaq, silsila, takhtit);
            hali = nizam;

            bool mala = nizam.Rakkib(
                wasl.HadafMala, typeof(TarqeeMalaNasij), nameof(TarqeeMalaNasij.Sabiq),
                "Text.OnPopulateMesh", sabiq: true);
            nizam.Rakkib(
                wasl.HadafLawha, typeof(TarqeeLawhaAsasiya), nameof(TarqeeLawhaAsasiya.Lahiq),
                "Text.get_mainTexture", sabiq: false);

            if (!mala)
            {
                Rabt.Ballagh(
                    "لم يُركَّب ترقيع Text.OnPopulateMesh؛ نصوص واجهة يونتي القديمة تبقى بلغة اللعبة وبقية الأنظمة تعمل. | "
                    + "The Text.OnPopulateMesh patch could not be installed; legacy uGUI text "
                    + "stays in the game's original language and every other text system "
                    + "keeps working.");
                nizam.Dispose();
                return null;
            }
            return nizam;
        }

        /// <summary>
        /// Whether this takeover drew the component's current geometry, which
        /// is the question the texture postfix asks before it substitutes
        /// Taarib's atlas for the font's own.
        /// </summary>
        /// <param name="mukawwin">The text component.</param>
        /// <returns>Whether Taarib owns this component's draw.</returns>
        public bool Yamlik(object? mukawwin)
        {
            if (!amil || mukawwin is not Component juz)
            {
                return false;
            }
            return mamlukat.Contains(juz.GetInstanceID());
        }

        /// <summary>The atlas page Taarib draws uGUI text from.</summary>
        /// <returns>The texture, or <c>null</c> when no page is resident.</returns>
        public Texture2D? Lawha()
        {
            return masdar.LawhatSafha(true, 0) ?? masdar.LawhatSafha(false, 0);
        }

        /// <summary>
        /// The whole interception: read the string, look it up, and either fill
        /// the accumulator with Taarib's geometry or hand the component
        /// straight back to uGUI.
        /// </summary>
        /// <param name="mukawwin">The text component being asked to draw.</param>
        /// <param name="musaid">The vertex accumulator it was handed.</param>
        /// <returns>
        /// Whether Taarib filled it. <c>false</c> means the component is
        /// untouched and uGUI's own generation must run, which is the correct
        /// answer for every string this patch does not cover.
        /// </returns>
        public bool Yarsum(object? mukawwin, object? musaid)
        {
            if (!amil || hars || mukawwin is null || musaid is null || !masdar.Amil)
            {
                return false;
            }
            hars = true;
            try
            {
                return Rassim(mukawwin, musaid);
            }
            catch (KhataTaarib khata)
            {
                Rabt.Ballagh("uGUI Text: drawing one string failed", khata);
                Utruk(mukawwin);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("drawing a uGUI Text component threw", khata);
                return false;
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// Marks one component dirty so the canvas rebuilds it through Taarib.
        /// </summary>
        /// <remarks>
        /// Called for the labels already on screen when the takeover is
        /// installed: a graphic that is not dirty is never rebuilt, and would
        /// keep drawing the untranslated mesh it generated before the patches
        /// existed. It runs inside the re-entrancy guard, because marking a
        /// graphic dirty schedules the very rebuild whose prefix this class
        /// installs.
        /// </remarks>
        /// <param name="mukawwin">The text component.</param>
        public void Ajjij(object? mukawwin)
        {
            Action<object>? ruus = wasl.AmrIttisakhRuus;
            Action<object>? madda = wasl.AmrIttisakhMadda;
            if (!amil || hars || mukawwin is null)
            {
                return;
            }
            hars = true;
            try
            {
                ruus?.Invoke(mukawwin);
                madda?.Invoke(mukawwin);
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Marking a uGUI Text component dirty failed", khata);
            }
            finally
            {
                hars = false;
            }
        }

        /// <summary>
        /// Removes this takeover's patches, gives every component its own
        /// material back, and stops drawing. The game is then exactly as it was
        /// before the takeover, in its original language.
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

            Action<object, Material>? katib = wasl.KatibMadda;
            foreach (KeyValuePair<Component, Material> zawj in maddatAsliya)
            {
                // A component destroyed with its scene compares equal to null
                // through Unity's own operator; writing to it would throw
                // during shutdown, which is the one moment a plugin must not
                // add an exception to somebody's log.
                if (katib is null || zawj.Key == null)
                {
                    continue;
                }
                try
                {
                    katib(zawj.Key, zawj.Value);
                }
                catch (Exception khata)
                {
                    Rabt.Ballagh("Restoring a uGUI Text material failed", khata);
                }
            }
            maddatAsliya.Clear();
            mamlukat.Clear();
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
        }

        private bool Rassim(object mukawwin, object musaid)
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

            // MiftahMinNass encodes the string to UTF-8 to hash it: a stackalloc
            // under 512 bytes and one byte array above it. A uGUI label longer
            // than that is a paragraph, redrawn when it changes rather than per
            // frame, and refusing to hash it would mean refusing to translate
            // exactly the longest strings a patch exists for.
            int fahras = ruqaa.JidNass(Ruqaa.MiftahMinNass(khaam!));
            if (fahras < 0)
            {
                // A miss means the game is drawing something this patch does
                // not cover. Leave it alone: do not lay it out, do not draw it,
                // and do not guess. uGUI renders it exactly as it always did.
                Utruk(mukawwin);
                return false;
            }

            RectTransform? mustatil = wasl.QariMustatil!(mukawwin);
            if (mustatil is null)
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
            Color lawnKamil = wasl.QariLawn!(mukawwin);
            Func<object, int>? qariMuhadhaha = wasl.QariMuhadhaha;
            int anchor = qariMuhadhaha is null ? 0 : qariMuhadhaha(mukawwin);

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            float hajmFili;
            float irtifaTakhtit;

            // The exact quarter-pixel size, never the nearest: a layout measured
            // at one size and drawn into a box the game sizes at another is how
            // text that fitted in the compiler's measurement overflows on a
            // player's screen.
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
                fahras, hajm, itar.width, itar.height, anchor, mukawwin,
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

            float hajmLawha = masdar.HajmLawha(hajmFili);
            bool tathbit = masdar.Namat == NamatLawha.Taghtiya;
            if (!minRuqaa && !masdar.Aqim(huruf, hajmLawha, tathbit))
            {
                Utruk(mukawwin);
                return false;
            }

            HayyizRasm hayyiz = default;
            // The pivot term is folded into the offsets rather than left in
            // MihwarS and MihwarA so that the vertical centring term reads the
            // same box the layout was given. A canvas maps one layout pixel to
            // one canvas unit, which is why Qiyas is one and not a ratio.
            hayyiz.Ard = itar.width;
            hayyiz.Irtifa = itar.height;
            hayyiz.MihwarS = 0f;
            hayyiz.MihwarA = 1f;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;
            hayyiz.IzahaS = -mihwar.x * itar.width;
            hayyiz.IzahaA = (1f - mihwar.y) * itar.height;
            hayyiz.Muhadhaha = WaslWajiha.RasiyaMin(anchor);

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
            // fraction already folded into its coverage, so the quad lands on
            // the integer grid and the subpixel bucket must be the one the
            // rasterizer used. A distance-field atlas holds one bitmap for
            // every size, so nothing is snapped and every bucket is zero.
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
                        "uGUI Text: the mesh buffer refused a second time after growing to "
                        + "the size it asked for; this string is left to uGUI.");
                    Utruk(mukawwin);
                    return false;
                }
            }

            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            if (!Aabbir(juz, mukawwin))
            {
                Utruk(mukawwin);
                return false;
            }
            Imla(musaid, in natija);
            mamlukat.Add(juz.GetInstanceID());
            return true;
        }

        /// <summary>
        /// Lays a string out at run time, for a size the compiler never
        /// produced a layout at.
        /// </summary>
        /// <remarks>
        /// The options come from the patch's constraint row rather than from
        /// the component, because the compiler measured this slot and recorded
        /// the direction, the justification mode, the diacritics policy and the
        /// digits policy the translator chose for it. Reading them off the
        /// component instead would lay this one string out with defaults and
        /// make it visibly disagree with the precomputed text beside it about
        /// all four. Only when there is no row at all does the component's own
        /// anchor and wrap mode stand in.
        /// </remarks>
        private bool Khattit(
            int fahras,
            float hajm,
            float ardMutah,
            float irtifaMutah,
            int anchor,
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
            NawNasq lahja = nasqGhani ? NawNasq.WajihatUnity : NawNasq.Bila;
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
            }
            else
            {
                khiyarat = default;
                khiyarat.Muhadhaha = WaslWajiha.MuhadhahaMin(anchor);
                Func<object, int>? qariTajawuz = wasl.QariTajawuzUfuqi;
                if (qariTajawuz is not null
                    && qariTajawuz(mukawwin) == WaslWajiha.TajawuzUfuqi)
                {
                    khiyarat.Alam |= Alamat.KhiyarSatrWahid;
                }
                // uGUI states line spacing as a multiple of the font's own line
                // height, and Taarib states it in pixels. There is no honest
                // conversion between the two without reading the game's font,
                // which Decision 5 forbids, so a multiplier other than one is
                // left to the font's metrics rather than turned into a number
                // invented here. The patch's constraint row carries the real
                // figure for every string the compiler measured.
            }
            if ((khiyarat.Alam & Alamat.KhiyarSatrWahid) != 0)
            {
                ardTalab = 0f;
            }

            TalabTakhtit talab = default;
            talab.Nass = nass;
            talab.Nitaqat = nitaqatNass;
            talab.Sifat = ReadOnlySpan<TaaribSifa>.Empty;
            talab.Hajm = hajm;
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
        /// Pours the built geometry into the accumulator the graphic handed
        /// over.
        /// </summary>
        /// <remarks>
        /// The stream form is one delegate invocation for a whole label and is
        /// tried first. It is the one place in this file that touches
        /// <see cref="UIVertex"/>'s own fields, and that field set is not stable
        /// across engine versions — <c>uv0</c> widened from two components to
        /// four in 2020 — so a build whose <see cref="UIVertex"/> does not match
        /// what this assembly was compiled against throws when the fill method
        /// is first compiled. That is caught once, the reason is named, and
        /// every fill afterwards goes through <c>AddVert</c> and
        /// <c>AddTriangle</c>, whose signatures have not changed since uGUI
        /// shipped. The text is identical either way.
        /// </remarks>
        private void Imla(object musaid, in NatijaNasij natija)
        {
            wasl.AmrMasah!(musaid);
            Action<object, List<UIVertex>, List<int>>? tayyar = wasl.AmrTayyar;
            if (tayyarMumkin && tayyar is not null)
            {
                try
                {
                    Jammi(in natija);
                    tayyar(musaid, ruusWajiha, fahrasWajiha);
                    return;
                }
                catch (MissingFieldException khata)
                {
                    TarkTayyar(khata);
                }
                catch (MissingMethodException khata)
                {
                    TarkTayyar(khata);
                }
                catch (TypeLoadException khata)
                {
                    TarkTayyar(khata);
                }
                wasl.AmrMasah!(musaid);
            }
            Wahdan(musaid, in natija);
        }

        /// <summary>
        /// Copies the built geometry into the two lists the stream form takes.
        /// The lists are reused, so after the first few labels of a scene this
        /// appends into capacity that already exists and allocates nothing.
        /// </summary>
        private void Jammi(in NatijaNasij natija)
        {
            ruusWajiha.Clear();
            fahrasWajiha.Clear();

            UIVertex ras = default;
            // uGUI's own quad producer writes these two constants on every
            // vertex; a canvas shader ignores both, and a game's mesh effect
            // that reads them expects exactly these values.
            ras.normal = new Vector3(0f, 0f, -1f);
            ras.tangent = new Vector4(1f, 0f, 0f, -1f);

            Vector3[] mawadi = makhzan.Ruus;
            Vector2[] malamis = makhzan.Malamis;
            Color32[] alwan = makhzan.Alwan;
            for (int i = 0; i < natija.AdadRuus; i++)
            {
                ras.position = mawadi[i];
                ras.uv0 = malamis[i];
                ras.color = alwan[i];
                ruusWajiha.Add(ras);
            }

            int[] muthallathat = makhzan.Muthallathat;
            for (int i = 0; i < natija.AdadMuthallathat; i++)
            {
                fahrasWajiha.Add(muthallathat[i]);
            }
        }

        /// <summary>
        /// Fills the accumulator one vertex and one triangle at a time — the
        /// form whose signatures every uGUI version shares.
        /// </summary>
        private void Wahdan(object musaid, in NatijaNasij natija)
        {
            Action<object, Vector3, Color32, Vector2>? ras = wasl.AmrRas;
            Action<object, int, int, int>? muthallath = wasl.AmrMuthallath;
            if (ras is null || muthallath is null)
            {
                Awqif(
                    "neither VertexHelper fill method resolved",
                    new MissingMethodException("UnityEngine.UI.VertexHelper", "AddVert"));
                return;
            }

            Vector3[] mawadi = makhzan.Ruus;
            Vector2[] malamis = makhzan.Malamis;
            Color32[] alwan = makhzan.Alwan;
            for (int i = 0; i < natija.AdadRuus; i++)
            {
                ras(musaid, mawadi[i], alwan[i], malamis[i]);
            }

            int[] muthallathat = makhzan.Muthallathat;
            for (int i = 0; i + 2 < natija.AdadMuthallathat; i += 3)
            {
                muthallath(musaid, muthallathat[i], muthallathat[i + 1], muthallathat[i + 2]);
            }
        }

        private void TarkTayyar(Exception khata)
        {
            if (!tayyarMumkin)
            {
                return;
            }
            tayyarMumkin = false;
            ruusWajiha.Clear();
            fahrasWajiha.Clear();
            Rabt.Ballagh(
                "VertexHelper.AddUIVertexStream could not be used against this engine's "
                + "UIVertex layout; uGUI text is filled one vertex at a time from now on, "
                + "which draws the identical mesh more slowly", khata);
        }

        /// <summary>
        /// Gives the component Taarib's material, recording its own the first
        /// time so it can be handed back.
        /// </summary>
        private bool Aabbir(Component juz, object mukawwin)
        {
            Material? madda = masdar.Madda(true, 0) ?? masdar.Madda(false, 0);
            Action<object, Material>? katib = wasl.KatibMadda;
            Func<object, Material>? qari = wasl.QariMadda;
            if (madda is null || katib is null)
            {
                return false;
            }
            Material? haliya = qari is null ? null : qari(mukawwin);
            if (ReferenceEquals(haliya, madda))
            {
                return true;
            }
            if (haliya is not null && !maddatAsliya.ContainsKey(juz))
            {
                maddatAsliya[juz] = haliya;
            }
            // Assigning marks the graphic's material dirty, which schedules a
            // rebuild. The reference check above is what keeps that from
            // happening inside every rebuild, forever.
            katib(mukawwin, madda);
            return true;
        }

        private void Utruk(object mukawwin)
        {
            if (mukawwin is Component juz)
            {
                mamlukat.Remove(juz.GetInstanceID());
            }
        }

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
                    + "of the uGUI takeover continues", khata);
                return false;
            }
        }

        /// <summary>
        /// The component's colour as the packed word
        /// <see cref="LawnRasm.Min"/> unpacks: red in the high byte, alpha in
        /// the low one. The order is the ABI's and it is not Unity's, which is
        /// why this conversion has a name instead of being open coded with the
        /// shifts written from memory at each call site.
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
                "تخطيط نص واجهة يونتي يمسّ أكثر من صفحة لوحة واحدة؛ رُسمت الصفحة الأولى فقط، وميزانية اللوحة في الرقعة أصغر مما يحتاجه النص المعروض. | "
                + "A uGUI text layout touches more than one atlas page; only the first was "
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
                "لا يوجد سياق تخطيط حيّ، فنصوص واجهة يونتي التي لم يُخطّطها المُصرِّف عند هذا الحجم تبقى بلغة اللعبة. | "
                + "There is no live layout context, so a uGUI string the compiler did not lay "
                + "out at the size it is being drawn at is left in the game's own language "
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
                "أُوقف الاستيلاء على نصوص واجهة يونتي: " + sabab + "؛ عادت إلى لغة اللعبة وبقية الأنظمة تعمل. | "
                + "The legacy uGUI takeover switched itself off: " + sabab
                + "; its text is back in the game's own language and every other text system "
                + "keeps working.",
                khata);
        }
    }

    /// <summary>
    /// ترقيع ملء النسيج — the prefix on
    /// <c>UnityEngine.UI.Text.OnPopulateMesh(VertexHelper)</c>, the point where
    /// a legacy uGUI label hands its geometry to the canvas.
    /// </summary>
    /// <remarks>
    /// This is the right seam rather than a convenient one. Everything above it
    /// — <c>TextGenerator.Populate</c>, the font's dynamic atlas, the character
    /// quads — is work whose answer would have to be discarded. Everything
    /// below it, in <c>Graphic.DoMeshGeneration</c>, is the game's own
    /// <c>IMeshModifier</c> components running over the finished vertices, and
    /// they must keep running: an Outline or a Shadow on an Arabic label is the
    /// game's design, not an obstacle.
    /// </remarks>
    public static class TarqeeMalaNasij
    {
        /// <summary>
        /// Fills the accumulator through Taarib when the patch covers the
        /// component's string.
        /// </summary>
        /// <param name="__instance">
        /// The component, injected by Harmony and typed as
        /// <see cref="object"/> because this assembly cannot name
        /// <c>UnityEngine.UI.Text</c> at compile time.
        /// </param>
        /// <param name="__0">
        /// The <c>VertexHelper</c> the graphic was handed, injected positionally
        /// for the same reason.
        /// </param>
        /// <returns>
        /// <c>false</c> to skip uGUI's own generation, which is what stops a
        /// pipeline with no Arabic OpenType layout in it from choosing glyphs;
        /// <c>true</c> to let it run untouched.
        /// </returns>
        public static bool Sabiq(object __instance, object __0)
        {
            NizamWajiha? nizam = NizamWajiha.Hali;
            return nizam is null || !nizam.Yarsum(__instance, __0);
        }
    }

    /// <summary>
    /// ترقيع اللوحة الأساسية — the postfix on
    /// <c>UnityEngine.UI.Text.mainTexture</c>'s getter.
    /// </summary>
    /// <remarks>
    /// uGUI answers that property with the <c>Font</c>'s own dynamic atlas — a
    /// texture of per-character bitmaps with no shaping in it, produced by
    /// <c>Font.RequestCharactersInTexture</c> and belonging to an asset
    /// Decision 5 forbids reading. The canvas binds whatever the property
    /// answers, so for a component Taarib drew the answer has to be Taarib's
    /// own atlas page instead; every glyph would otherwise sample the rectangle
    /// of whichever Latin character the engine's atlas happens to hold there.
    /// </remarks>
    public static class TarqeeLawhaAsasiya
    {
        /// <summary>
        /// Substitutes Taarib's atlas page for the font's own texture.
        /// </summary>
        /// <param name="__instance">The component, injected by Harmony.</param>
        /// <param name="__result">
        /// The texture uGUI was about to answer with, replaced in place when
        /// Taarib owns this component's draw and left exactly as it was
        /// otherwise.
        /// </param>
        public static void Lahiq(object __instance, ref Texture __result)
        {
            NizamWajiha? nizam = NizamWajiha.Hali;
            if (nizam is null || !nizam.Yamlik(__instance))
            {
                return;
            }
            Texture2D? lawha = nizam.Lawha();
            if (lawha is not null)
            {
                __result = lawha;
            }
        }
    }
}
