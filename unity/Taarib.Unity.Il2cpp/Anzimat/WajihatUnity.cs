// نظام واجهة يونتي — the legacy UnityEngine.UI.Text takeover, on the IL2CPP
// backend.
//
// THIS FILE DECIDES NOTHING ABOUT RENDERING EITHER. It reads a string, asks
// Ruqaa whether the patch covers it, lays it out through Takhtit when the
// compiler did not, builds vertices through Nasij, and pours them into the
// accumulator uGUI handed over. Every one of those calls is the same call the
// Mono adapter makes with the same arguments, which is what makes the two
// backends draw the same pixels by construction. The three places that could
// silently diverge are the same three named in TextMeshPro.cs — the quantized
// size, the HayyizRasm fields, and the packed colour — plus one that belongs to
// this file alone: the markup dialect. NawNasq.WajihatUnity and
// NawNasq.TextMeshPro genuinely disagree, because <sprite=3> is a sprite in one
// and eleven characters a player is meant to read in the other, so passing the
// wrong one here would translate a string correctly and draw it wrongly.
//
// WHY uGUI FAILS AT ARABIC, AND WHERE. A Text component asks a TextGenerator to
// lay the string out; the generator asks the Font for a bitmap per CHARACTER out
// of a dynamic atlas the engine manages, and writes one UIVertex quad per
// character. There is no OpenType layout anywhere in that chain — no GSUB, so no
// contextual forms and no lam-alef ligature; no GPOS, so no mark attachment; no
// bidirectional algorithm, so a mixed sentence stays in logical order.
//
// WHY Font.RequestCharactersInTexture IS NOT USED, AND MUST NOT BE. That call is
// the whole of the engine's dynamic font atlas, and three things make it wrong
// here. It is keyed by codepoint, so what comes back for an Arabic letter is
// that letter's ISOLATED form — the shape a letter has standing alone, which is
// the wrong shape in the overwhelming majority of words and the exact defect
// that makes so-called Arabic support in Unity games read as disconnected
// letters. It has no notion of a mark's attachment point, so tashkeel lands on
// the baseline. And it belongs to a Font asset of the game's, which Decision 5
// forbids reading. Taarib rasterizes its own glyphs from its own validated
// bundled font into its own atlas, and never subscribes to Font.textureRebuilt.
//
// WHY THE SEAM IS OnPopulateMesh. Graphic.DoMeshGeneration calls
// OnPopulateMesh(VertexHelper) to get the geometry, then runs every
// IMeshModifier on the object — that is what an Outline, a Shadow or any
// BaseMeshEffect a game ships is — and only then pours the helper into the mesh
// the CanvasRenderer receives. Filling the helper here and suppressing uGUI's
// own fill therefore puts Taarib's vertices exactly where the engine expects a
// graphic's own vertices to be, and every mesh effect in the game keeps working
// over the Arabic instead of being bypassed. Intercepting further down, at
// TextGenerator.Populate, would be the wrong seam twice over: the generator's
// output is character quads that would have to be thrown away, and the modifiers
// would then run over vertices some other code already replaced.
//
// UnityEngine.UI IS ABSENT FROM MOST MODERN GAMES, AND THAT IS NOT A FAULT. uGUI
// ships as a package rather than an engine module, and a title built entirely on
// TextMeshPro simply does not contain it. Mawjud therefore answers false and
// nothing is logged: a line per absent text system would fill every player's
// BepInEx log with notes about things that were never wrong. When it IS present
// under IL2CPP, Il2CppInterop's generated facade calls it Il2CppUnityEngine.UI —
// the generator prefixes the namespaces of the assemblies it emits — while the
// runtime and the binary know it only as UnityEngine.UI. Every HadafHall below
// therefore carries the NATIVE namespace, because rungs two and three ask the
// runtime, and rung one maps that name onto the generated assembly itself.
//
// THE TWO INTERCEPTIONS, AND WHY ONE OF THEM IS ALWAYS A DETOUR.
// Text.OnPopulateMesh returns void and takes an object, so a managed prefix can
// express it and HarmonyX takes it whenever rung one resolved it.
// Text.get_mainTexture returns a Texture, and substituting the return value from
// a managed postfix would mean naming UnityEngine.Texture at compile time, which
// this assembly cannot do without binding itself to one game's generated
// assemblies. So that one is always taken with Mihmaz, where replacing an object
// pointer is the ordinary thing a native replacement does. That is stated here
// rather than discovered from the log, because "one of my two hooks is always
// the fragile kind" is a fact about this file that should not have to be
// inferred.

using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using HarmonyLib;
using Il2CppInterop.Runtime.InteropTypes;
using Taarib.Unity.Il2cpp.Hall;
using Taarib.Unity.Il2cpp.Maqbad;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Il2cpp.Anzimat
{
    /// <summary>
    /// وصل واجهة يونتي — every legacy uGUI member this takeover touches,
    /// resolved once at startup through <see cref="Sullam"/>.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Two of these decide whether the takeover can run: the
    /// <c>OnPopulateMesh</c> target, without which there is no interception, and
    /// the pair that fills a <c>VertexHelper</c>. Everything else degrades on its
    /// own — a build with no <c>supportRichText</c> loses markup handling for
    /// uGUI text and nothing else.
    /// </para>
    /// <para>
    /// <b>Only the per-vertex fill is bound, and that is deliberate.</b> The Mono
    /// adapter prefers <c>VertexHelper.AddUIVertexStream(List&lt;UIVertex&gt;,
    /// List&lt;int&gt;)</c>, which appends a whole label in one call. Under
    /// IL2CPP that would mean constructing two generic <c>Il2CppSystem.List</c>
    /// instantiations whose generated types this assembly cannot name, filling
    /// them element by element across the heap boundary anyway, and holding them
    /// rooted between frames. The per-vertex pair is one native call per vertex
    /// and per triangle, allocates nothing, has had the same signature since uGUI
    /// shipped, and produces the identical mesh. The cost is throughput on a
    /// path that only runs when a label's text changes.
    /// </para>
    /// </remarks>
    public sealed class WaslWajiha
    {
        /// <summary>The IL2CPP assembly legacy uGUI ships as.</summary>
        public const string Tajammu = "UnityEngine.UI";

        /// <summary>uGUI's native namespace, which is not the generated one.</summary>
        public const string Fadaa = "UnityEngine.UI";

        /// <summary>
        /// <c>HorizontalWrapMode.Overflow</c>, the value that means the component
        /// refuses to wrap.
        /// </summary>
        public const int TajawuzUfuqi = 1;

        private WaslWajiha(WaslMuharrik wasl)
        {
            QariNass = wasl.Hall(Tajammu, Fadaa, "Text", "get_text", 0, string.Empty);
            QariHajm = wasl.Hall(Tajammu, Fadaa, "Text", "get_fontSize", 0, string.Empty);
            QariNasqGhani = wasl.Hall(
                Tajammu, Fadaa, "Text", "get_supportRichText", 0, string.Empty);
            QariMuhadhaha = wasl.Hall(Tajammu, Fadaa, "Text", "get_alignment", 0, string.Empty);
            QariTajawuzUfuqi = wasl.Hall(
                Tajammu, Fadaa, "Text", "get_horizontalOverflow", 0, string.Empty);

            QariLawn = wasl.Hall(Tajammu, Fadaa, "Graphic", "get_color", 0, string.Empty);
            QariMustatil = wasl.Hall(
                Tajammu, Fadaa, "Graphic", "get_rectTransform", 0, string.Empty);
            QariMadda = wasl.Hall(Tajammu, Fadaa, "Graphic", "get_material", 0, string.Empty);
            KatibMadda = wasl.Hall(Tajammu, Fadaa, "Graphic", "set_material", 1, string.Empty);
            AmrIttisakhRuus = wasl.Hall(
                Tajammu, Fadaa, "Graphic", "SetVerticesDirty", 0, string.Empty);
            AmrIttisakhMadda = wasl.Hall(
                Tajammu, Fadaa, "Graphic", "SetMaterialDirty", 0, string.Empty);

            AmrMasah = wasl.Hall(Tajammu, Fadaa, "VertexHelper", "Clear", 0, string.Empty);
            AmrRas = wasl.Hall(Tajammu, Fadaa, "VertexHelper", "AddVert", 3, string.Empty);
            AmrMuthallath = wasl.Hall(
                Tajammu, Fadaa, "VertexHelper", "AddTriangle", 3, string.Empty);

            HadafMala = wasl.Hall(
                Tajammu, Fadaa, "Text", "OnPopulateMesh", 1, "ugui.text.populate");
            HadafLawha = wasl.Hall(
                Tajammu, Fadaa, "Text", "get_mainTexture", 0, "ugui.text.maintexture");
        }

        // Readonly fields, not get-only properties, for the reason given on
        // WaslTmp's members in Anzimat/TextMeshPro.cs: `in binding.X` at a
        // takeover point needs a variable, and a property would make every read
        // a struct copy on the per-frame path.

        /// <summary>Reads the component's raw string, markup and all.</summary>
        public readonly TabiaMahlula QariNass;

        /// <summary>Reads its font size in points, which for a canvas is pixels.</summary>
        public readonly TabiaMahlula QariHajm;

        /// <summary>Whether the component parses markup in its string.</summary>
        public readonly TabiaMahlula QariNasqGhani;

        /// <summary>
        /// Reads its anchor as an integer. <c>TextAnchor</c> packs both axes into
        /// one value — the horizontal edge is the remainder modulo three and the
        /// vertical edge is the quotient — and reading it as its underlying
        /// integer is what lets both be decoded without depending on the enum
        /// being present under that name.
        /// </summary>
        public readonly TabiaMahlula QariMuhadhaha;

        /// <summary>Whether the component wraps at the rectangle's width.</summary>
        public readonly TabiaMahlula QariTajawuzUfuqi;

        /// <summary>Reads its colour, which every glyph inherits unless a span sets one.</summary>
        public readonly TabiaMahlula QariLawn;

        /// <summary>Reads the rectangle it draws into.</summary>
        public readonly TabiaMahlula QariMustatil;

        /// <summary>Reads the material the graphic currently draws with.</summary>
        public readonly TabiaMahlula QariMadda;

        /// <summary>
        /// Assigns the material the graphic draws with. This is how Taarib's own
        /// shader reaches the canvas: the component's own material is recorded
        /// first and given back when the takeover is removed.
        /// </summary>
        public readonly TabiaMahlula KatibMadda;

        /// <summary>Marks the graphic's geometry dirty.</summary>
        public readonly TabiaMahlula AmrIttisakhRuus;

        /// <summary>Marks the graphic's material dirty.</summary>
        public readonly TabiaMahlula AmrIttisakhMadda;

        /// <summary>Empties the vertex accumulator before it is refilled.</summary>
        public readonly TabiaMahlula AmrMasah;

        /// <summary>Appends one vertex: position, colour, texture coordinate.</summary>
        public readonly TabiaMahlula AmrRas;

        /// <summary>Appends one triangle by vertex index.</summary>
        public readonly TabiaMahlula AmrMuthallath;

        /// <summary>
        /// <c>Text.OnPopulateMesh(VertexHelper)</c>, as a hook target. This is
        /// the interception point; without it there is no takeover.
        /// </summary>
        public readonly TabiaMahlula HadafMala;

        /// <summary>
        /// <c>Text.mainTexture</c>'s getter, as a hook target. uGUI answers that
        /// property with the font's own atlas texture and the canvas binds
        /// whatever it answers, so for a component Taarib drew the answer has to
        /// be Taarib's own atlas page instead.
        /// </summary>
        public readonly TabiaMahlula HadafLawha;

        /// <summary>
        /// Whether enough resolved to draw at all: the string, the rectangle, the
        /// size, the interception point, and both halves of the fill.
        /// </summary>
        public bool Muakkad =>
            HadafMala.Wujid
            && QariNass.Wujid
            && QariMustatil.Wujid
            && QariHajm.Wujid
            && AmrMasah.Wujid
            && AmrRas.Wujid
            && AmrMuthallath.Wujid;

        /// <summary>
        /// Binds uGUI's text path, or reports why it could not be bound and
        /// returns <c>null</c>.
        /// </summary>
        /// <param name="wasl">The engine binding, which owns the ladder.</param>
        /// <returns>The binding, or <c>null</c>.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="wasl"/> is null.</exception>
        public static WaslWajiha? Iqran(WaslMuharrik wasl)
        {
            if (wasl is null)
            {
                throw new ArgumentNullException(nameof(wasl));
            }

            WaslWajiha binding = new WaslWajiha(wasl);
            if (!binding.Muakkad)
            {
                wasl.Sijill.LogWarning(
                    "واجهة يونتي القديمة موجودة ولم تُحلَّ أعضاء الرسم فيها؛ تبقى نصوصها بلغة اللعبة وبقية الأنظمة تعمل. | "
                    + "UnityEngine.UI.Text is present but its drawing members did not "
                    + "resolve; its text is left in the game's original language and every "
                    + "other text system in this game is unaffected.");
                return null;
            }
            if (!binding.KatibMadda.Wujid)
            {
                wasl.Sijill.LogWarning(
                    "لا يمكن إسناد مادة إلى Graphic في هذه البِنية؛ أُوقف الاستيلاء على واجهة يونتي القديمة. | "
                    + "Graphic.material cannot be assigned in this build; uGUI text cannot "
                    + "be given Taarib's shader, and the legacy takeover is off rather than "
                    + "drawing a single-channel atlas through a four-channel shader.");
                return null;
            }
            if (!binding.HadafLawha.Wujid)
            {
                wasl.Sijill.LogWarning(
                    "Text.mainTexture did not resolve in this build; uGUI text is drawn "
                    + "through Taarib's material, which carries the atlas itself, and the "
                    + "canvas's own texture binding is left as the game set it.");
            }
            return binding;
        }

        /// <summary>
        /// The horizontal half of a <c>TextAnchor</c>, as the layout engine's own
        /// value.
        /// </summary>
        /// <param name="anchor">The anchor value.</param>
        /// <returns>Where the line sits inside the available width.</returns>
        /// <remarks>
        /// Left maps to <see cref="Muhadhaha.Bidaya"/> rather than to an absolute
        /// edge, because Bidaya already means the leading edge — which is the
        /// right edge of a right-to-left line. Mapping it to an absolute edge and
        /// flipping it somewhere else is how a label ends up correctly aligned in
        /// Arabic and mirrored in the English label beside it.
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

        /// <summary>The vertical half of a <c>TextAnchor</c>.</summary>
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
    /// نظام واجهة يونتي — the legacy uGUI takeover: the interceptions, the
    /// draw, and the fill.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The material.</b> Taarib's atlas is a single-channel texture and uGUI's
    /// stock shader samples all four channels, so the component's material is
    /// replaced with Taarib's the first time it is drawn, and its own material is
    /// recorded and given back when the takeover is removed. Assigning a material
    /// marks the graphic's material dirty, which schedules a rebuild, so the
    /// assignment happens only when the material is not already Taarib's:
    /// written the other way it would dirty the graphic from inside the graphic's
    /// own rebuild, once per frame, forever.
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
    public sealed class NizamWajiha : INizamIl2cpp
    {
        [ThreadStatic]
        private static bool hars;

        private static NizamWajiha? hali;
        private static IntPtr muaqqatMala;
        private static IntPtr muaqqatLawha;

        private readonly List<Mirbat> marabit = new List<Mirbat>(2);
        private readonly HashSet<int> mamlukat = new HashSet<int>();
        private readonly Dictionary<int, MaddaAsliya> maddatAsliya =
            new Dictionary<int, MaddaAsliya>();

        private MasdarMushtarak? mushtarak;
        private MawaridIl2cpp? mawarid;
        private WaslMuharrik? wasl;
        private WaslWajiha? waslWajiha;
        private MasdarAshkal? masdar;
        private MakhzanRusum? makhzan;
        private NassMuhaddar? muhaddar;
        private byte[] nassUtf8 = new byte[512];
        private bool amil;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        /// <inheritdoc/>
        public string Ism => "UnityEngine.UI.Text";

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <summary>
        /// The live takeover the managed patch method reaches, or <c>null</c>
        /// when uGUI text was never taken over in this process.
        /// </summary>
        internal static NizamWajiha? Hali => hali;

        /// <inheritdoc/>
        /// <remarks>
        /// One resolution through the ladder, and no log line when it fails: a
        /// game built entirely on TextMeshPro is the ordinary modern case and is
        /// not a fault worth a line in anybody's log.
        /// </remarks>
        public bool Mawjud(Sullam sullam)
        {
            if (sullam is null)
            {
                return false;
            }
            HadafHall hadaf = new HadafHall(
                WaslWajiha.Tajammu, WaslWajiha.Fadaa, "Text", "OnPopulateMesh", 1,
                "ugui.text.populate");
            return sullam.Hall(in hadaf).Wujid;
        }

        /// <inheritdoc/>
        public unsafe void Rakkib(MawaridIl2cpp mawaridJadida, Harmony tarqee)
        {
            if (mawaridJadida is null)
            {
                throw new ArgumentNullException(nameof(mawaridJadida));
            }
            if (tarqee is null)
            {
                throw new ArgumentNullException(nameof(tarqee));
            }
            if (hali is not null)
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6724",
                    "رُكّب نظام واجهة يونتي القديمة مرتين في العملية نفسها؛ هذا خلل في الملحق.",
                    "The legacy uGUI takeover was installed twice in one process; this is a "
                    + "bug in the plugin.",
                    Khutwa.IblaghLilMalik);
            }

            MasdarMushtarak? holder = MasdarMushtarak.Ihjiz(mawaridJadida);
            if (holder is null)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6721",
                    "تعذّر تجهيز ربط المحرّك أو لوحة الأشكال؛ تبقى نصوص واجهة يونتي بلغة اللعبة.",
                    "The engine binding or the glyph source could not be prepared; legacy "
                    + "uGUI text is left in the game's original language.",
                    Khutwa.FathTashkhis);
            }

            mushtarak = holder;
            mawarid = mawaridJadida;
            wasl = holder.Wasl;
            masdar = holder.Masdar;

            WaslWajiha? binding = WaslWajiha.Iqran(holder.Wasl);
            if (binding is null)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6722",
                    "واجهة يونتي القديمة موجودة ولم تُحلَّ أعضاء الرسم فيها.",
                    "UnityEngine.UI.Text is present but its drawing members did not resolve.",
                    Khutwa.FathTashkhis);
            }

            waslWajiha = binding;
            makhzan = new MakhzanRusum();
            muhaddar = new NassMuhaddar();
            amil = true;
            hali = this;

            bool mala = Rabbit(
                in binding.HadafMala, typeof(TarqeeMalaNasij), nameof(TarqeeMalaNasij.Sabiq),
                false,
                (IntPtr)(void*)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr, void>)
                    &BadilMala,
                tarqee, ref muaqqatMala);

            // Always a native detour: replacing an object-typed return value from
            // a managed postfix would mean naming UnityEngine.Texture, which this
            // assembly cannot do. Passing no managed patch name is what tells
            // Mirbat to skip the HarmonyX path entirely rather than try and fail.
            Rabbit(
                in binding.HadafLawha, typeof(NizamWajiha), string.Empty, true,
                (IntPtr)(void*)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr>)&BadilLawha,
                tarqee, ref muaqqatLawha);

            if (!mala)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6720",
                    "لم يُركَّب اعتراض Text.OnPopulateMesh؛ تبقى نصوص واجهة يونتي القديمة بلغة اللعبة.",
                    "The Text.OnPopulateMesh interception could not be installed; legacy "
                    + "uGUI text stays in the game's original language and every other text "
                    + "system keeps working.",
                    Khutwa.FathTashkhis);
            }
        }

        /// <inheritdoc/>
        public void Fukk()
        {
            amil = false;
            for (int i = marabit.Count - 1; i >= 0; i--)
            {
                marabit[i].Dispose();
            }
            marabit.Clear();
            muaqqatMala = IntPtr.Zero;
            muaqqatLawha = IntPtr.Zero;

            WaslWajiha? binding = waslWajiha;
            foreach (KeyValuePair<int, MaddaAsliya> zawj in maddatAsliya)
            {
                IntPtr juz = zawj.Value.Juz.Kaen;
                IntPtr madda = zawj.Value.Madda.Kaen;
                if (binding is not null && binding.KatibMadda.Wujid
                    && juz != IntPtr.Zero && madda != IntPtr.Zero)
                {
                    // A component destroyed with its scene reports dead through
                    // its weak handle; writing to it would be a write into freed
                    // memory, which is the one thing a shutdown path must not do.
                    WaslMuharrik.Nida(in binding.KatibMadda, juz, madda);
                }
                zawj.Value.Juz.Dispose();
                zawj.Value.Madda.Dispose();
            }
            maddatAsliya.Clear();
            mamlukat.Clear();

            makhzan?.Dispose();
            makhzan = null;
            muhaddar = null;
            waslWajiha = null;
            masdar = null;
            wasl = null;
            mawarid = null;
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
            mushtarak?.Sarrih();
            mushtarak = null;
        }

        /// <summary>
        /// The whole interception: read the string, look it up, and either fill
        /// the accumulator with Taarib's geometry or hand the component straight
        /// back to uGUI.
        /// </summary>
        /// <param name="kaen">The text component, as an IL2CPP object pointer.</param>
        /// <param name="musaid">The vertex accumulator it was handed.</param>
        /// <returns>
        /// Whether Taarib filled it. <c>false</c> means the component is
        /// untouched and uGUI's own generation must run, which is the correct
        /// answer for every string this patch does not cover.
        /// </returns>
        public bool Yarsum(IntPtr kaen, IntPtr musaid)
        {
            MasdarAshkal? source = masdar;
            if (!amil || hars || kaen == IntPtr.Zero || musaid == IntPtr.Zero
                || source is null || !source.Amil)
            {
                return false;
            }
            hars = true;
            try
            {
                return Rassim(kaen, musaid);
            }
            catch (KhataTaarib khata)
            {
                mawarid?.Sijill.LogWarning(
                    "تعذّر رسم نص واحد في واجهة يونتي القديمة. | Drawing one uGUI string "
                    + "failed. " + khata.Injilizi);
                Utruk(kaen);
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
        /// Whether this takeover drew the component's current geometry, which is
        /// the question the texture interception asks before it substitutes
        /// Taarib's atlas for the font's own.
        /// </summary>
        /// <param name="kaen">The text component.</param>
        /// <returns>Whether Taarib owns this component's draw.</returns>
        public bool Yamlik(IntPtr kaen)
        {
            WaslMuharrik? engine = wasl;
            if (!amil || engine is null || kaen == IntPtr.Zero)
            {
                return false;
            }
            return mamlukat.Contains(engine.Muarrif(kaen));
        }

        /// <summary>The atlas page Taarib draws uGUI text from.</summary>
        /// <returns>The texture object, or zero when no page is resident.</returns>
        public IntPtr Lawha()
        {
            MasdarAshkal? source = masdar;
            if (source is null)
            {
                return IntPtr.Zero;
            }
            source.AwwalMadda(out _, out IntPtr lawhaSafha);
            return lawhaSafha;
        }

        private bool Rassim(IntPtr kaen, IntPtr musaid)
        {
            WaslMuharrik engine = wasl!;
            WaslWajiha binding = waslWajiha!;
            MasdarAshkal source = masdar!;
            MawaridIl2cpp shared = mawarid!;
            MakhzanRusum buffers = makhzan!;

            IntPtr nassKaen = WaslMuharrik.Qeema<IntPtr>(in binding.QariNass, kaen);
            if (nassKaen == IntPtr.Zero || !IqraNass(nassKaen, out ReadOnlySpan<byte> utf8)
                || utf8.IsEmpty)
            {
                Utruk(kaen);
                return false;
            }

            // Taarib's own bytes from here on; the IL2CPP string is never
            // referred to again, so everything below is free to allocate.
            ulong miftah = Ruqaa.MiftahMinNass(utf8);

            IntPtr mustatilKaen = WaslMuharrik.Qeema<IntPtr>(in binding.QariMustatil, kaen);
            MustatilMuharrik itar = engine.Mustatil(mustatilKaen);
            float hajm = WaslMuharrik.Qeema<int>(in binding.QariHajm, kaen);
            int anchor = binding.QariMuhadhaha.Wujid
                ? WaslMuharrik.Qeema<int>(in binding.QariMuhadhaha, kaen)
                : 0;
            bool yaltaff = !binding.QariTajawuzUfuqi.Wujid
                || WaslMuharrik.Qeema<int>(in binding.QariTajawuzUfuqi, kaen)
                    != WaslWajiha.TajawuzUfuqi;

            if (shared.Yaltaqit)
            {
                Iltaqit(shared, nassKaen, miftah, engine.Itar(), itar, hajm, yaltaff);
                Utruk(kaen);
                return false;
            }

            int fahras = shared.Ruqaa.JidNass(miftah);
            if (fahras < 0)
            {
                // A miss means the game is drawing something this patch does not
                // cover. Leave it alone: do not lay it out, do not draw it, and
                // do not guess. uGUI renders it exactly as it always did.
                Utruk(kaen);
                return false;
            }

            if (mustatilKaen == IntPtr.Zero || !(hajm > 0f))
            {
                Utruk(kaen);
                return false;
            }

            Muttajih2 mihwar = engine.Mihwar(mustatilKaen);
            LawnKamil lawnKamil = binding.QariLawn.Wujid
                ? WaslMuharrik.Qeema<LawnKamil>(in binding.QariLawn, kaen)
                : Abyad();

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            float hajmFili;
            float irtifaTakhtit;

            // The exact quarter-pixel size, never the nearest, and always through
            // Nasij.HajmRubi: a layout measured at one size and drawn into a box
            // the game sizes at another is how text that fitted in the compiler's
            // measurement overflows on a player's screen.
            ushort hajmRubi = Nasij.HajmRubi(hajm);
            bool minRuqaa = shared.Ruqaa.JidTakhtit(fahras, hajmRubi, out MadkhalTakhtit madkhal);
            if (minRuqaa)
            {
                huruf = shared.Ruqaa.HurufTakhtit(in madkhal);
                sutur = shared.Ruqaa.SuturTakhtit(in madkhal);
                nitaqat = buffers.Hawwil(shared.Ruqaa.NitaqatNass(fahras));
                hajmFili = madkhal.Hajm;
                irtifaTakhtit = madkhal.Irtifa;
            }
            else if (!Khattit(
                kaen, fahras, hajm, itar.Ard, itar.Irtifa, anchor,
                out huruf, out sutur, out nitaqat, out hajmFili, out irtifaTakhtit))
            {
                Utruk(kaen);
                return false;
            }

            if (huruf.IsEmpty || !(hajmFili > 0f))
            {
                Utruk(kaen);
                return false;
            }

            float hajmLawha = source.HajmLawha(hajmFili);
            bool tathbit = source.Namat == NamatLawha.Taghtiya;
            if (!minRuqaa && !source.Aqim(huruf, hajmLawha, tathbit))
            {
                Utruk(kaen);
                return false;
            }

            HayyizRasm hayyiz = default;
            // The pivot term is folded into the offsets rather than left in
            // MihwarS and MihwarA so that the vertical centring term reads the
            // same box the layout was given. A canvas maps one layout pixel to
            // one canvas unit, which is why Qiyas is one and not a ratio.
            hayyiz.Ard = itar.Ard;
            hayyiz.Irtifa = itar.Irtifa;
            hayyiz.MihwarS = 0f;
            hayyiz.MihwarA = 1f;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;
            hayyiz.IzahaS = -mihwar.S * itar.Ard;
            hayyiz.IzahaA = (1f - mihwar.A) * itar.Irtifa;
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
            // A coverage atlas holds one bitmap per size with the pen's fraction
            // already folded into its coverage, so the quad lands on the integer
            // grid and the subpixel bucket must be the one the rasterizer used. A
            // distance-field atlas holds one bitmap for every size, so nothing is
            // snapped and every bucket is zero.
            talab.Tathbit = tathbit;

            if (!buffers.Wassi(huruf.Length, engine))
            {
                Utruk(kaen);
                return false;
            }
            NatijaNasij natija = Nasij.Ibni(
                in talab, source.Khareeta(minRuqaa), buffers.Makhzan());
            if (!natija.Kafa)
            {
                if (!buffers.Wassi(natija.MatlubRuus / Nasij.RuusLiShakl, engine))
                {
                    Utruk(kaen);
                    return false;
                }
                natija = Nasij.Ibni(in talab, source.Khareeta(minRuqaa), buffers.Makhzan());
                if (!natija.Kafa)
                {
                    shared.Sijill.LogWarning(
                        "uGUI Text: the mesh buffer refused a second time after growing to "
                        + "the size it asked for; this string is left to uGUI.");
                    Utruk(kaen);
                    return false;
                }
            }

            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            if (!Aabbir(engine, binding, source, kaen))
            {
                Utruk(kaen);
                return false;
            }

            buffers.Amsah(in natija);
            Imla(binding, musaid, in natija);
            mamlukat.Add(engine.Muarrif(kaen));
            return true;
        }

        /// <summary>
        /// Lays a string out at run time, for a size the compiler never produced
        /// a layout at.
        /// </summary>
        /// <remarks>
        /// The options come from the patch's constraint row rather than from the
        /// component, because the compiler measured this slot and recorded the
        /// direction, the justification mode, the diacritics policy and the
        /// digits policy the translator chose for it. Reading them off the
        /// component instead would lay this one string out with defaults and make
        /// it visibly disagree with the precomputed text beside it about all
        /// four. Only when there is no row at all does the component's own anchor
        /// and wrap mode stand in. uGUI's own line spacing is a multiple of the
        /// font's line height and Taarib states spacing in pixels; there is no
        /// honest conversion between the two without reading the game's font,
        /// which Decision 5 forbids, so a multiplier other than one is left to
        /// the font's metrics rather than turned into a number invented here.
        /// </remarks>
        private bool Khattit(
            IntPtr kaen,
            int fahras,
            float hajm,
            float ardMutah,
            float irtifaMutah,
            int anchor,
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

            MawaridIl2cpp shared = mawarid!;
            WaslWajiha binding = waslWajiha!;
            Takhtit buffer = shared.Takhtit;

            bool nasqGhani = binding.QariNasqGhani.Wujid
                && WaslMuharrik.Qeema<byte>(in binding.QariNasqGhani, kaen) != 0;
            NawNasq lahja = nasqGhani ? NawNasq.WajihatUnity : NawNasq.Bila;
            if (!muhaddar!.Hayyi(
                shared.Ruqaa, fahras, makhzan!, lahja, hajm,
                out ReadOnlySpan<char> nass, out ReadOnlySpan<TaaribNitaqUslub> nitaqatNass,
                shared.Sijill))
            {
                BallighTakhtit();
                return false;
            }

            KhiyaratTakhtit khiyarat;
            float ardTalab = ardMutah;
            if (shared.Ruqaa.JidQayd(fahras, out MadkhalQayd qayd))
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
                if (binding.QariTajawuzUfuqi.Wujid
                    && WaslMuharrik.Qeema<int>(in binding.QariTajawuzUfuqi, kaen)
                        == WaslWajiha.TajawuzUfuqi)
                {
                    khiyarat.Alam |= Alamat.KhiyarSatrWahid;
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
            talab.Hajm = hajm;
            talab.ArdMutah = ardTalab;
            talab.IrtifaMutah = irtifaMutah;
            talab.Khiyarat = khiyarat;
            buffer.Khattit(shared.Siyaq, shared.Silsila, in talab);

            huruf = buffer.Huruf;
            sutur = buffer.Sutur;
            nitaqat = nitaqatNass;
            hajmFili = buffer.Hajm;
            irtifaTakhtit = buffer.Irtifa;
            return true;
        }

        /// <summary>
        /// Pours the built geometry into the accumulator the graphic handed over,
        /// one vertex and one triangle at a time.
        /// </summary>
        /// <remarks>
        /// Allocates nothing. The spans are over managed arrays the mesh builder
        /// just wrote, and each call is one native transition with three
        /// by-value arguments; the vertex count is a label's worth, not a
        /// scene's.
        /// </remarks>
        private void Imla(WaslWajiha binding, IntPtr musaid, in NatijaNasij natija)
        {
            WaslMuharrik.Nida(in binding.AmrMasah, musaid);

            ReadOnlySpan<NuqtaRasm> mawadi = makhzan!.RuusMahalliya;
            ReadOnlySpan<NuqtaMulmas> malamis = makhzan.MalamisMahalliya;
            ReadOnlySpan<LawnRasm> alwan = makhzan.AlwanMahalliya;
            int adadRuus = natija.AdadRuus;
            for (int i = 0; i < adadRuus; i++)
            {
                Muttajih3 mawdi;
                mawdi.S = mawadi[i].S;
                mawdi.A = mawadi[i].A;
                mawdi.Z = mawadi[i].Z;
                Muttajih2 mulmas;
                mulmas.S = malamis[i].U;
                mulmas.A = malamis[i].V;
                WaslMuharrik.Nida(in binding.AmrRas, musaid, mawdi, alwan[i], mulmas);
            }

            ReadOnlySpan<int> muthallathat = makhzan.MuthallathatMahalliya;
            int adadFahras = natija.AdadMuthallathat;
            for (int i = 0; i + 2 < adadFahras; i += 3)
            {
                WaslMuharrik.Nida(
                    in binding.AmrMuthallath, musaid,
                    muthallathat[i], muthallathat[i + 1], muthallathat[i + 2]);
            }
        }

        /// <summary>
        /// Gives the component Taarib's material, recording its own the first
        /// time so it can be handed back.
        /// </summary>
        private bool Aabbir(
            WaslMuharrik engine, WaslWajiha binding, MasdarAshkal source, IntPtr kaen)
        {
            if (!source.AwwalMadda(out IntPtr madda, out _) || !binding.KatibMadda.Wujid)
            {
                return false;
            }
            IntPtr haliya = binding.QariMadda.Wujid
                ? WaslMuharrik.Qeema<IntPtr>(in binding.QariMadda, kaen)
                : IntPtr.Zero;
            if (haliya == madda)
            {
                return true;
            }
            if (haliya != IntPtr.Zero)
            {
                SajjilMaddaAsliya(engine, kaen, haliya);
            }
            // Assigning marks the graphic's material dirty, which schedules a
            // rebuild. The reference check above is what keeps that from
            // happening inside every rebuild, forever.
            WaslMuharrik.Nida(in binding.KatibMadda, kaen, madda);
            return true;
        }

        /// <summary>
        /// Copies the component's string out of the IL2CPP heap, growing the
        /// buffer once when a string is longer than any before it.
        /// </summary>
        /// <remarks>
        /// The pointer arrived as a return value in this same call, nothing has
        /// allocated since, and the copy itself allocates nothing — so there is
        /// no moment at which the collector could reclaim or relocate the string
        /// between reading its character pointer and finishing with it.
        /// </remarks>
        private bool IqraNass(IntPtr nassKaen, out ReadOnlySpan<byte> utf8)
        {
            utf8 = ReadOnlySpan<byte>.Empty;
            if (!Nasakh.IlaUtf8(nassKaen, nassUtf8.AsSpan(), out int maktub, out int matlub))
            {
                if (matlub <= 0 || matlub > Nasakh.AqsaTul * 4)
                {
                    return false;
                }
                nassUtf8 = new byte[matlub];
                if (!Nasakh.IlaUtf8(nassKaen, nassUtf8.AsSpan(), out maktub, out _))
                {
                    return false;
                }
            }
            utf8 = new ReadOnlySpan<byte>(nassUtf8, 0, maktub);
            return true;
        }

        /// <summary>
        /// Remembers a component's own material the first time Taarib replaces
        /// it, so <see cref="Fukk"/> can give it back.
        /// </summary>
        /// <remarks>
        /// The component is held weakly and the material strongly, for the same
        /// reasons the TextMeshPro takeover gives: a strong root on the component
        /// would keep every label the game ever created resident, and a weak root
        /// on the material would let the only thing worth restoring be collected
        /// the moment Taarib took its last reference away.
        /// </remarks>
        private void SajjilMaddaAsliya(WaslMuharrik engine, IntPtr kaen, IntPtr asliya)
        {
            int muarrif = engine.Muarrif(kaen);
            if (muarrif == 0 || maddatAsliya.ContainsKey(muarrif))
            {
                return;
            }
            try
            {
                maddatAsliya[muarrif] = new MaddaAsliya(Marja.Daeef(kaen), Marja.Qawi(asliya));
            }
            catch (KhataTaarib khata)
            {
                mawarid?.Sijill.LogDebug(
                    "recording a uGUI Text material failed: " + khata.Injilizi);
            }
        }

        private void Iltaqit(
            MawaridIl2cpp shared,
            IntPtr nassKaen,
            ulong miftah,
            long raqmItar,
            MustatilMuharrik itar,
            float hajm,
            bool yaltaff)
        {
            JalsatIltiqat? jalsa = shared.Jalsa;
            if (jalsa is null)
            {
                return;
            }
            // Allocates one managed string per sighting, deliberately: capture is
            // a translator's tool that runs instead of replacement, never beside
            // it, so the frame budget it spends is not a player's.
            string? asl = Nasakh.Nass(nassKaen);
            if (asl is null)
            {
                return;
            }
            TalabIltiqat talab = default;
            talab.Huwiya = miftah;
            talab.Asl = asl;
            talab.Nizam = NizamNass.WajihatUnity;
            talab.Itar = raqmItar;
            talab.Mustatil = MustatilIltiqat.Min(
                NuqtaIltiqat.Min(itar.S, itar.A), NuqtaIltiqat.Min(itar.Ard, itar.Irtifa));
            talab.Hajm = hajm;
            talab.Yaltaff = yaltaff;
            jalsa.Sajjil(in talab);
        }

        private bool Rabbit(
            in TabiaMahlula hadaf,
            Type hamil,
            string ismBadil,
            bool lahiq,
            IntPtr badilKham,
            Harmony tarqee,
            ref IntPtr muaqqat)
        {
            Mirbat? mirbat = Mirbat.Rakkib(
                in hadaf, hamil, ismBadil, lahiq, badilKham, tarqee, mawarid!.Sijill);
            if (mirbat is null)
            {
                return false;
            }
            marabit.Add(mirbat);
            muaqqat = mirbat.Muaqqat;
            mawarid.Sijill.LogInfo(
                $"{hadaf.Ism}: intercepted by "
                + (mirbat.BiTarqee ? "HarmonyX" : "a native detour")
                + $", resolved on rung {(int)hadaf.Rutba}.");
            return true;
        }

        private void Utruk(IntPtr kaen)
        {
            WaslMuharrik? engine = wasl;
            if (engine is null || kaen == IntPtr.Zero)
            {
                return;
            }
            mamlukat.Remove(engine.Muarrif(kaen));
        }

        private static LawnKamil Abyad()
        {
            LawnKamil lawn;
            lawn.Ahmar = 1f;
            lawn.Akhdar = 1f;
            lawn.Azraq = 1f;
            lawn.Shaffafiya = 1f;
            return lawn;
        }

        /// <summary>
        /// The component's colour as the packed word <see cref="LawnRasm.Min"/>
        /// unpacks: red in the high byte, alpha in the low one. The order is the
        /// ABI's and it is not Unity's, which is why this conversion has a name
        /// instead of being open coded with the shifts written from memory at
        /// each call site.
        /// </summary>
        private static uint LawnMuazzam(LawnKamil lawn)
        {
            uint r = Bayt(lawn.Ahmar);
            uint g = Bayt(lawn.Akhdar);
            uint b = Bayt(lawn.Azraq);
            uint a = Bayt(lawn.Shaffafiya);
            return (r << 24) | (g << 16) | (b << 8) | a;
        }

        private static uint Bayt(float qeema)
        {
            float mahsur = qeema < 0f ? 0f : (qeema > 1f ? 1f : qeema);
            return (uint)(mahsur * 255f + 0.5f);
        }

        private void BallighSafahat()
        {
            if (ballaghSafahat)
            {
                return;
            }
            ballaghSafahat = true;
            mawarid?.Sijill.LogWarning(
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
            mawarid?.Sijill.LogWarning(
                "تعذّر تحضير نص واجهة يونتي لم يُخطّطه المُصرِّف عند هذا الحجم؛ يبقى بلغة اللعبة. | "
                + "A uGUI string the compiler did not lay out at the size it is being drawn "
                + "at could not be prepared, so it is left in the game's own language rather "
                + "than laid out with default options that would disagree with the text "
                + "beside it.");
        }

        private void Awqif(string sabab, Exception khata)
        {
            if (!amil)
            {
                return;
            }
            amil = false;
            mamlukat.Clear();
            mawarid?.Sijill.LogError(
                "أُوقف الاستيلاء على نصوص واجهة يونتي: " + sabab + "؛ عادت إلى لغة اللعبة وبقية الأنظمة تعمل. | "
                + "The legacy uGUI takeover switched itself off: " + sabab
                + "; its text is back in the game's own language and every other text system "
                + "keeps working.");
            mawarid?.Sijill.LogDebug(khata.ToString());
        }

        private readonly struct MaddaAsliya
        {
            public MaddaAsliya(Marja juz, Marja madda)
            {
                Juz = juz;
                Madda = madda;
            }

            public Marja Juz { get; }

            public Marja Madda { get; }
        }

        // -------------------------------------------------------------------
        // The native replacements. Signature: the target's managed signature,
        // the implicit `this` in front, and IL2CPP's trailing MethodInfo* at the
        // end. Nothing may escape these: an exception crossing back into native
        // code terminates the game rather than being logged.
        // -------------------------------------------------------------------

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        private static unsafe void BadilMala(IntPtr kaen, IntPtr musaid, IntPtr tabia)
        {
            try
            {
                NizamWajiha? nizam = hali;
                if (nizam is not null && nizam.Yarsum(kaen, musaid))
                {
                    return;
                }
            }
            catch (Exception)
            {
                // Yarsum already reports and disables itself; anything reaching
                // here is beyond reporting and must still let uGUI draw.
            }
            IntPtr muaqqat = muaqqatMala;
            if (muaqqat != IntPtr.Zero)
            {
                // SOUND: the trampoline holds the target's relocated prologue
                // followed by a jump back into the untouched rest of it, so it
                // expects exactly the register state the function was entered
                // with — the same arguments, including the trailing MethodInfo*.
                ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr, void>)muaqqat)(
                    kaen, musaid, tabia);
            }
        }

        /// <summary>
        /// Substitutes Taarib's atlas page for the font's own texture, after
        /// letting uGUI compute the answer it would have given.
        /// </summary>
        /// <remarks>
        /// uGUI answers <c>mainTexture</c> with the Font's dynamic atlas — a
        /// texture of per-character bitmaps with no shaping in it, belonging to
        /// an asset Decision 5 forbids reading — and the canvas binds whatever
        /// the property answers. For a component Taarib drew the answer has to be
        /// Taarib's own page instead; every glyph would otherwise sample the
        /// rectangle of whichever Latin character the engine's atlas holds there.
        /// The original is still called first, so a component Taarib does not own
        /// gets exactly the texture it always did.
        /// </remarks>
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        private static unsafe IntPtr BadilLawha(IntPtr kaen, IntPtr tabia)
        {
            IntPtr asli = IntPtr.Zero;
            IntPtr muaqqat = muaqqatLawha;
            if (muaqqat != IntPtr.Zero)
            {
                // SOUND: as BadilMala.
                asli = ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr>)muaqqat)(kaen, tabia);
            }
            try
            {
                NizamWajiha? nizam = hali;
                if (nizam is null || !nizam.Yamlik(kaen))
                {
                    return asli;
                }
                IntPtr lawha = nizam.Lawha();
                return lawha != IntPtr.Zero ? lawha : asli;
            }
            catch (Exception)
            {
                return asli;
            }
        }
    }

    /// <summary>
    /// ترقيع ملء النسيج — the managed prefix on
    /// <c>UnityEngine.UI.Text.OnPopulateMesh(VertexHelper)</c>, used when rung
    /// one resolved it and HarmonyX can weave the generated method.
    /// </summary>
    /// <remarks>
    /// This is the right seam rather than a convenient one. Everything above it
    /// — <c>TextGenerator.Populate</c>, the font's dynamic atlas, the character
    /// quads — is work whose answer would have to be discarded. Everything below
    /// it, in <c>Graphic.DoMeshGeneration</c>, is the game's own
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
        /// The component, injected by Harmony as the Il2CppInterop proxy whose
        /// <c>Pointer</c> is the native object this adapter works in terms of.
        /// </param>
        /// <param name="__0">
        /// The <c>VertexHelper</c> the graphic was handed, injected positionally
        /// because this assembly cannot name its generated proxy type.
        /// </param>
        /// <returns>
        /// <c>false</c> to skip uGUI's own generation, which is what stops a
        /// pipeline with no Arabic OpenType layout in it from choosing glyphs;
        /// <c>true</c> to let it run untouched.
        /// </returns>
        public static bool Sabiq(Il2CppObjectBase __instance, Il2CppObjectBase __0)
        {
            NizamWajiha? nizam = NizamWajiha.Hali;
            return nizam is null || !nizam.Yarsum(__instance.Pointer, __0.Pointer);
        }
    }
}
