// نظام تكست ميش برو — the TextMeshPro takeover, on the IL2CPP backend.
//
// THIS FILE DECIDES NOTHING ABOUT RENDERING. Every decision about what Arabic
// looks like was made once, in Taarib.Unity.Mushtarak, and is compiled into
// both plugins unchanged: Ruqaa reads the patch, Takhtit lays the text out
// through jisr, Nasij turns a layout into vertices, Lawha owns atlas
// residency, Nasq lifts markup into spans. The Mono adapter calls exactly
// those functions with exactly these arguments, and so does this file. That is
// what makes identical rendering between the two backends a structural fact
// rather than a testing result.
//
// WHAT WOULD BREAK THAT PROPERTY, NAMED SO IT CAN BE WATCHED FOR. Four things,
// and only four. (1) Computing a quantized size here instead of calling
// Nasij.HajmRubi, so the two adapters round differently and one draws a layout
// the compiler measured at another size. (2) Filling HayyizRasm differently —
// the pivot term folded into MihwarS/MihwarA on one side and into IzahaS/IzahaA
// on the other, which moves a middle-aligned label by half its margin. (3)
// Packing the colour by hand instead of through LawnMuazzam, because the ABI's
// byte order is red-high and Unity's Color32 is not. (4) Reading layout options
// off the component when the patch carries a constraint row, which would make
// one runtime-laid-out string disagree with the precomputed text beside it
// about direction, justification, diacritics and digits. Every one of those is
// a place where this file could quietly diverge from the Mono adapter while
// still rendering something, so every one of them is a single call to a shared
// function here and nowhere else.
//
// WHY TMP'S PIPELINE IS BYPASSED RATHER THAN CORRECTED — the same reason as
// under Mono, restated because it is the load-bearing decision. TMP is a
// shaper, not a renderer with a layout step bolted on: it walks the string,
// looks each character up in its own TMP_FontAsset glyph table, applies its own
// kerning pairs and writes a quad per character. That pipeline contains no
// OpenType layout at all — no GSUB, so no init/medi/fina/isol substitution and
// no lam-alef ligature; no GPOS, so tashkeel sits on the baseline instead of on
// its letter; no bidirectional algorithm, so a mixed sentence stays in logical
// order. A postfix that moved TMP's quads around would be permuting the wrong
// glyphs: correctly shaped Arabic is not a permutation of what TMP chose, it is
// a different set of glyphs. So the interception returns "do not run" and the
// geometry comes from Nasij instead.
//
// WHAT IS ACTUALLY DIFFERENT UNDER IL2CPP, WHICH IS ALL THIS FILE ADDS.
//
//   (a) HOW A METHOD IS FOUND. There is no Assembly-CSharp to reflect over.
//       Every address comes from Sullam, the three-rung ladder, and every
//       resolution it performs is recorded so the diagnostics bundle can say
//       which rung answered. Nothing here calls Type.GetMethod to choose a hook
//       target; the one place that touches managed reflection at all is
//       Mirbat.MinBayanat, and it runs only after the ladder has already
//       decided, only to recover the managed handle HarmonyX needs, and refuses
//       on any ambiguity.
//
//   (b) HOW IT IS HOOKED. Two mechanisms, chosen by which rung answered.
//       Rung one means Il2CppInterop generated a real managed facade for the
//       method, so HarmonyX can weave it and the hook cooperates with every
//       other plugin in the process. Any other rung means the address is real
//       and the managed facade is not, so the only way in is Mihmaz, a native
//       detour over the function's prologue. Mihmaz is the fallback and not the
//       default because byte patching cannot be shared: two frameworks that
//       both patch one prologue corrupt each other, which is why Mihmaz refuses
//       to stack rather than chaining.
//
//   (c) THE HIDDEN TRAILING MethodInfo*. Every method IL2CPP compiles takes one
//       more argument than its managed signature shows: a MethodInfo* appended
//       after the declared parameters and after the implicit `this`. A native
//       replacement must therefore be declared
//           static void Badil(IntPtr self, <declared params...>, IntPtr tabia)
//       under the platform's ordinary calling convention, and must hand the
//       same trailing pointer back to the trampoline when it wants the
//       original. Declare it wrong and nothing crashes cleanly: the original
//       reads its MethodInfo out of whatever register the mismatch left, and
//       the corruption surfaces three calls later. The same argument is needed
//       in the other direction — calling an engine method through a resolved
//       entry point means supplying that pointer — which is why WaslMuharrik
//       carries a MethodInfo* beside every address it holds.
//
//   (d) HOW OBJECTS ARE HELD AND HOW STRINGS CROSS. No raw IL2CPP pointer is
//       stored in a field here. Anything that outlives a call goes through
//       Maqbad/Marja.cs, which installs a real root the IL2CPP collector knows
//       about, and is re-read through the handle rather than cached. Text is
//       copied out of the IL2CPP heap into a Taarib-owned buffer before
//       anything can allocate, because the layout call is where buffers grow
//       and a relocated Il2CppString under an in-flight shaping call is a
//       corrupted paragraph with no stack trace on it.
//
// WHAT ALLOCATES, STATED PLAINLY. On the per-frame draw path: nothing. Growth
// happens in MakhzanRusum.Wassi, NassMuhaddar's pooled buffers and
// MasdarAshkal's page upload, all of which stop after the first few frames of a
// scene. Capture mode allocates one managed string per newly seen text through
// Nasakh.Nass, deliberately: it is a translator's tool, not a play path. The
// GetComponents fallback in WaslMuharrik allocates an IL2CPP array per call and
// is used only on engines older than TryGetComponent, which is said again where
// it lives.
//
// DECISION 5, MECHANICALLY. Nothing below reads fontAsset, fontSharedMaterial,
// the game's SDF atlas, a sprite asset or any serialized asset of the game's.
// Taarib uploads its own atlas pages into its own Texture2D, builds its own
// Material from its own shader, and binds that for the draw it owns.

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using BepInEx.Logging;
using HarmonyLib;
using Il2CppInterop.Runtime;
using Il2CppInterop.Runtime.InteropTypes;
using Taarib.Unity.Il2cpp.Hall;
using Taarib.Unity.Il2cpp.Khatf;
using Taarib.Unity.Il2cpp.Maqbad;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Il2cpp.Anzimat
{
    /// <summary>Two floats, laid out as <c>UnityEngine.Vector2</c>.</summary>
    /// <remarks>
    /// This assembly has no reference to UnityEngine and must not acquire one:
    /// under IL2CPP the engine's types reach managed code as Il2CppInterop
    /// proxies whose assembly identity is generated per game, so an adapter
    /// bound to one game's proxies would not load in another. The engine's
    /// value types are therefore mirrored here as plain blittable structs and
    /// passed by value through unmanaged function pointers, which makes the
    /// runtime apply the platform ABI to them exactly as the C++ compiler did.
    /// The layouts are fixed by Unity's own public API and have never changed.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct Muttajih2
    {
        /// <summary>The horizontal component.</summary>
        public float S;

        /// <summary>The vertical component.</summary>
        public float A;
    }

    /// <summary>Three floats, laid out as <c>UnityEngine.Vector3</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Muttajih3
    {
        /// <summary>The horizontal component.</summary>
        public float S;

        /// <summary>The vertical component.</summary>
        public float A;

        /// <summary>The depth component.</summary>
        public float Z;
    }

    /// <summary>Four floats, laid out as <c>UnityEngine.Vector4</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Muttajih4
    {
        /// <summary>The first component.</summary>
        public float S;

        /// <summary>The second component.</summary>
        public float A;

        /// <summary>The third component.</summary>
        public float Z;

        /// <summary>The fourth component.</summary>
        public float W;
    }

    /// <summary>Four floats, laid out as <c>UnityEngine.Color</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct LawnKamil
    {
        /// <summary>Red, 0 to 1.</summary>
        public float Ahmar;

        /// <summary>Green, 0 to 1.</summary>
        public float Akhdar;

        /// <summary>Blue, 0 to 1.</summary>
        public float Azraq;

        /// <summary>Alpha, 0 to 1.</summary>
        public float Shaffafiya;
    }

    /// <summary>Four floats, laid out as <c>UnityEngine.Rect</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct MustatilMuharrik
    {
        /// <summary>The left edge in local units.</summary>
        public float S;

        /// <summary>The bottom edge in local units.</summary>
        public float A;

        /// <summary>The width.</summary>
        public float Ard;

        /// <summary>The height.</summary>
        public float Irtifa;
    }

    /// <summary>
    /// A centre and a half-extent, laid out as <c>UnityEngine.Bounds</c>.
    /// </summary>
    /// <remarks>
    /// The second member is the HALF size, not the size. Unity's
    /// <c>Bounds(center, size)</c> constructor halves its argument, and this
    /// struct is the field layout rather than the constructor — writing a full
    /// size here doubles every text object's bounding box, which does not
    /// misdraw anything and therefore never gets noticed.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct HududMuharrik
    {
        /// <summary>The centre.</summary>
        public Muttajih3 Markaz;

        /// <summary>Half the size on each axis.</summary>
        public Muttajih3 Nisf;
    }

    /// <summary>
    /// One IL2CPP method this adapter uses: its entry point, the hidden
    /// <c>MethodInfo*</c> that entry point expects as its last argument, and
    /// which rung of the ladder produced the address.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The address always comes from <see cref="Sullam"/>. The
    /// <c>MethodInfo*</c> is not an address to jump to and is never treated as
    /// one — it is a datum the ABI requires, obtained from the runtime's own
    /// metadata tables, and it is carried beside the entry point rather than
    /// looked up per call because a per-call lookup on a frame path would be a
    /// dictionary probe per glyph.
    /// </para>
    /// <para>
    /// <see cref="Tabia"/> may legitimately be zero on a title whose metadata
    /// cannot be searched by name — the case rung three exists for — and the
    /// binding is still usable there. What that costs is exact and worth
    /// stating: IL2CPP passes the pointer so that generic-shared code can
    /// recover its type arguments and so that a throwing method can build a
    /// stack trace. Every member bound through this type is a non-generic
    /// member of a non-generic engine class called with arguments that cannot
    /// make it throw, so the pointer is unread. A binding that ever stops
    /// satisfying that description must not use a zero here.
    /// </para>
    /// </remarks>
    public readonly struct TabiaMahlula
    {
        /// <summary>Records one resolved method.</summary>
        /// <param name="ism">Its qualified name, for a log line and a refusal.</param>
        /// <param name="unwan">Its entry point, from the ladder.</param>
        /// <param name="tabia">Its native <c>MethodInfo*</c>, or zero.</param>
        /// <param name="rutba">Which rung produced the address.</param>
        public TabiaMahlula(string ism, IntPtr unwan, IntPtr tabia, Rutba rutba)
        {
            Ism = ism ?? string.Empty;
            Unwan = unwan;
            Tabia = tabia;
            Rutba = rutba;
        }

        /// <summary>The qualified name.</summary>
        public string Ism { get; }

        /// <summary>The compiled entry point, from <see cref="Sullam"/> alone.</summary>
        public IntPtr Unwan { get; }

        /// <summary>The trailing <c>MethodInfo*</c> the ABI requires, or zero.</summary>
        public IntPtr Tabia { get; }

        /// <summary>Which rung of the ladder produced the address.</summary>
        public Rutba Rutba { get; }

        /// <summary>Whether anything resolved.</summary>
        public bool Wujid => Unwan != IntPtr.Zero;

        /// <summary>Whether the managed facade exists, which is what HarmonyX needs.</summary>
        public bool MinBayanat => Rutba == Rutba.Bayanat;
    }

    /// <summary>
    /// وصل المحرّك — every UnityEngine member the takeovers in this namespace
    /// call, resolved once through <see cref="Sullam"/> and invoked through
    /// unmanaged function pointers.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Why this exists at all.</b> Under Mono the adapter names
    /// <c>Mesh</c>, <c>Material</c> and <c>CanvasRenderer</c> directly, because
    /// UnityEngine.Modules is a reference package that matches every engine
    /// version. Under IL2CPP those same types reach managed code only as
    /// Il2CppInterop proxies generated per game, so naming them would bind this
    /// plugin to one title. Everything the engine has to do for Taarib is
    /// therefore a resolved address plus a calling convention, and this class is
    /// the whole of that surface — about three dozen members, listed in one
    /// place so a reader can see exactly how much of somebody's engine this
    /// plugin touches.
    /// </para>
    /// <para>
    /// <b>Overloads that share an arity, which is the real hazard here.</b>
    /// <see cref="HadafHall"/> separates overloads by parameter count, and
    /// several engine members have two overloads with the same count —
    /// <c>Material.SetFloat(string, float)</c> beside
    /// <c>SetFloat(int, float)</c>, <c>CanvasRenderer.SetMaterial(Material,
    /// int)</c> beside <c>SetMaterial(Material, Texture)</c>. Resolving one of
    /// those and calling it with the other's argument types is not a caught
    /// error, it is a pointer passed where an integer was expected. So members
    /// with an ambiguous arity are either avoided in favour of a uniquely
    /// counted equivalent — <c>set_mainTexture</c> for <c>_MainTex</c>,
    /// <c>set_color</c> for <c>_Color</c>, the private <c>*Impl</c> forms Unity
    /// funnels both public overloads into — or resolved and then checked with
    /// <see cref="NawWaseet"/>, which reads the parameter's class name out of
    /// the runtime and selects the call shape from it.
    /// </para>
    /// <para>
    /// Not thread-safe, built once during plugin initialisation, and every call
    /// on it afterwards happens on Unity's main thread inside a takeover point.
    /// </para>
    /// </remarks>
    public sealed class WaslMuharrik : IDisposable
    {
        /// <summary>The engine assembly most of these members live in.</summary>
        public const string TajammuAsas = "UnityEngine.CoreModule";

        /// <summary>The engine assembly the canvas renderer lives in.</summary>
        public const string TajammuWajiha = "UnityEngine.UIModule";

        /// <summary>The engine namespace, which Il2CppInterop does not rename.</summary>
        public const string FadaaMuharrik = "UnityEngine";

        /// <summary>
        /// Where an IL2CPP array's elements begin, relative to the array object.
        /// </summary>
        /// <remarks>
        /// An <c>Il2CppArray</c> is an <c>Il2CppObject</c> (a class pointer and a
        /// monitor) followed by a bounds pointer and a length, then the
        /// elements. That is four pointer-sized fields on both 32- and 64-bit
        /// builds, which is why this is expressed as a multiple of
        /// <see cref="IntPtr.Size"/> rather than as a constant: a constant would
        /// be right on x86-64 and silently wrong on a 32-bit Android build.
        /// </remarks>
        public static readonly int IzahatUnsur = 4 * IntPtr.Size;

        private readonly Sullam sullam;
        private readonly ManualLogSource sijill;
        private readonly Dictionary<string, List<IntPtr>> suwar =
            new Dictionary<string, List<IntPtr>>(StringComparer.Ordinal);
        private readonly List<IntPtr> kulSuwar = new List<IntPtr>();
        private readonly Dictionary<IntPtr, Marja> anwaKaen = new Dictionary<IntPtr, Marja>();

        private TabiaMahlula jidSudfa;
        private TabiaMahlula muarrifKhasiya;
        private TabiaMahlula atlif;
        private TabiaMahlula muarrifKaen;
        private TabiaMahlula alamatIkhfa;
        private TabiaMahlula itarHali;

        private TabiaMahlula lawhaBani;
        private TabiaMahlula lawhaIrfa;
        private TabiaMahlula lawhaTabbiq;
        private TabiaMahlula lawhaTasfiya;
        private TabiaMahlula lawhaLaff;

        private TabiaMahlula maddaBani;
        private TabiaMahlula maddaLawhaAsasiya;
        private TabiaMahlula maddaLawn;
        private TabiaMahlula maddaAshari;
        private TabiaMahlula maddaMuttajih;
        private TabiaMahlula maddaLawhaBiMuarrif;

        private TabiaMahlula jarrib;
        private TabiaMahlula jamiMukawwinat;
        private TabiaMahlula tahwil;
        private TabiaMahlula adadAtfal;
        private TabiaMahlula tifl;

        private TabiaMahlula mustatil;
        private TabiaMahlula mihwar;

        private TabiaMahlula nasijImsah;
        private TabiaMahlula nasijRuus;
        private TabiaMahlula nasijMalamis;
        private TabiaMahlula nasijAlwan;
        private TabiaMahlula nasijMuthallathat;
        private TabiaMahlula nasijHudud;

        private TabiaMahlula rassamNasij;
        private TabiaMahlula rassamAdadMawad;
        private TabiaMahlula rassamMadda;
        private TabiaMahlula rassamLawha;
        private TabiaMahlula rassamImsah;

        private TabiaMahlula murashshihNasij;
        private TabiaMahlula murashshihNasijQira;
        private TabiaMahlula mujassamMaddaQira;
        private TabiaMahlula mujassamMaddaKitaba;

        private IntPtr sanfMuttajih3;
        private IntPtr sanfMuttajih2;
        private IntPtr sanfLawn32;
        private IntPtr sanfSahih;
        private IntPtr sanfRassam;
        private IntPtr sanfMurashshih;
        private IntPtr sanfMujassam;

        private bool maddaBiMuarrif;
        private bool rassamMaddaBiLawha;
        private bool mutakhalla;

        private WaslMuharrik(Sullam sullam, ManualLogSource sijill)
        {
            this.sullam = sullam;
            this.sijill = sijill;
        }

        /// <summary>Whether the engine members the draw path needs all resolved.</summary>
        public bool Muakkad { get; private set; }

        /// <summary>
        /// Whether a distance-field patch can be drawn, which needs the two
        /// shader parameters no property shortcut can set.
        /// </summary>
        public bool YadamMisafa => maddaAshari.Wujid && maddaMuttajih.Wujid;

        /// <summary>The class pointer for <c>UnityEngine.CanvasRenderer</c>.</summary>
        public IntPtr SanfRassam => sanfRassam;

        /// <summary>The class pointer for <c>UnityEngine.MeshFilter</c>.</summary>
        public IntPtr SanfMurashshih => sanfMurashshih;

        /// <summary>The class pointer for <c>UnityEngine.MeshRenderer</c>.</summary>
        public IntPtr SanfMujassam => sanfMujassam;

        /// <summary>Where a takeover writes its log lines.</summary>
        public ManualLogSource Sijill => sijill;

        /// <summary>
        /// Binds the engine, or reports why it could not be bound and returns
        /// <c>null</c>.
        /// </summary>
        /// <param name="sullam">The resolution ladder. Every address comes from it.</param>
        /// <param name="sijill">Where to report a refusal.</param>
        /// <returns>The binding, or <c>null</c>.</returns>
        /// <exception cref="ArgumentNullException">Either argument is null.</exception>
        public static WaslMuharrik? Iqran(Sullam sullam, ManualLogSource sijill)
        {
            if (sullam is null)
            {
                throw new ArgumentNullException(nameof(sullam));
            }
            if (sijill is null)
            {
                throw new ArgumentNullException(nameof(sijill));
            }

            WaslMuharrik wasl = new WaslMuharrik(sullam, sijill);
            if (!wasl.IbniSuwar())
            {
                return null;
            }
            wasl.Iqrar();
            if (!wasl.Muakkad)
            {
                sijill.LogWarning(
                    "لم تُحلَّ أعضاء المحرّك التي يحتاجها الرسم؛ يبقى النص بلغة اللعبة. | The "
                    + "engine members the draw path needs did not resolve; text is left in "
                    + "the game's original language.");
                wasl.Dispose();
                return null;
            }
            return wasl;
        }

        /// <summary>
        /// Resolves one method: its address through the ladder, its
        /// <c>MethodInfo*</c> through the runtime.
        /// </summary>
        /// <param name="tajammu">The IL2CPP assembly name, without extension.</param>
        /// <param name="fadaa">The native namespace, empty for the global one.</param>
        /// <param name="sanf">The declaring type's native name.</param>
        /// <param name="tabia">The method's name.</param>
        /// <param name="adadWasait">How many parameters it declares.</param>
        /// <param name="miftahBasma">Its key in the signature database, or empty.</param>
        /// <returns>The binding, which may be unresolved.</returns>
        /// <remarks>
        /// The names are the NATIVE ones. Il2CppInterop prefixes the namespaces
        /// of generated assemblies — <c>TMPro</c> becomes <c>Il2CppTMPro</c>,
        /// <c>UnityEngine.UI</c> becomes <c>Il2CppUnityEngine.UI</c> — but rungs
        /// two and three ask the runtime and the binary, which know only what
        /// the metadata says. Passing a prefixed name here would make every
        /// target fail on exactly the games where the prefix is the only thing
        /// that exists.
        /// </remarks>
        public TabiaMahlula Hall(
            string tajammu, string fadaa, string sanf, string tabia, int adadWasait,
            string miftahBasma)
        {
            HadafHall hadaf = new HadafHall(
                tajammu, fadaa, sanf, tabia, adadWasait, miftahBasma);
            NatijatHall natija = sullam.Hall(in hadaf);
            if (!natija.Wujid)
            {
                return new TabiaMahlula(hadaf.Ism, IntPtr.Zero, IntPtr.Zero, Rutba.Bila);
            }

            IntPtr sanfKham = Sanf(tajammu, fadaa, sanf);
            IntPtr tabiaKham = IntPtr.Zero;
            if (sanfKham != IntPtr.Zero)
            {
                try
                {
                    tabiaKham = IL2CPP.il2cpp_class_get_method_from_name(
                        sanfKham, tabia, adadWasait);
                }
                catch (Exception)
                {
                    tabiaKham = IntPtr.Zero;
                }
            }
            return new TabiaMahlula(hadaf.Ism, natija.Unwan, tabiaKham, natija.Rutba);
        }

        /// <summary>Finds one native class, or zero.</summary>
        /// <param name="tajammu">The assembly name, tried first.</param>
        /// <param name="fadaa">The namespace, empty for the global one.</param>
        /// <param name="ism">The class name.</param>
        /// <returns>The <c>Il2CppClass*</c>, or zero.</returns>
        /// <remarks>
        /// The named image is tried first and every loaded image afterwards,
        /// because a type that moved between assemblies between game versions
        /// should still be found. The empty namespace is passed as an empty
        /// string and never as null: the runtime compares it against an interned
        /// empty string, and a null there resolves every global-namespace type
        /// in the game to nothing.
        /// </remarks>
        public IntPtr Sanf(string tajammu, string fadaa, string ism)
        {
            string fadaaAmin = fadaa ?? string.Empty;
            if (string.IsNullOrEmpty(ism))
            {
                return IntPtr.Zero;
            }

            try
            {
                if (!string.IsNullOrEmpty(tajammu)
                    && suwar.TryGetValue(
                        tajammu.ToLowerInvariant(), out List<IntPtr>? musamma))
                {
                    for (int i = 0; i < musamma.Count; i++)
                    {
                        IntPtr wahid = IL2CPP.il2cpp_class_from_name(
                            musamma[i], fadaaAmin, ism);
                        if (wahid != IntPtr.Zero)
                        {
                            return wahid;
                        }
                    }
                }

                for (int i = 0; i < kulSuwar.Count; i++)
                {
                    IntPtr wahid = IL2CPP.il2cpp_class_from_name(kulSuwar[i], fadaaAmin, ism);
                    if (wahid != IntPtr.Zero)
                    {
                        return wahid;
                    }
                }
            }
            catch (Exception khata)
            {
                sijill.LogDebug(
                    $"looking up {fadaaAmin}.{ism} in the IL2CPP runtime threw: {khata.Message}");
            }
            return IntPtr.Zero;
        }

        /// <summary>
        /// A <c>System.Type</c> object for a native class, rooted for the life of
        /// this binding.
        /// </summary>
        /// <param name="sanf">The native class.</param>
        /// <returns>The reflection object, or zero.</returns>
        /// <remarks>
        /// Rooted through a <see cref="Marja"/> rather than kept as a pointer
        /// because it is held across frames, which is exactly the case the
        /// collector is entitled to reclaim. It is also cached: producing one
        /// per call would allocate an IL2CPP object per component per frame, on
        /// the draw path, for a value that never changes.
        /// </remarks>
        public IntPtr NawKaen(IntPtr sanf)
        {
            if (sanf == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }
            if (anwaKaen.TryGetValue(sanf, out Marja? mahfuz))
            {
                return mahfuz.Kaen;
            }

            try
            {
                IntPtr naw = IL2CPP.il2cpp_class_get_type(sanf);
                if (naw == IntPtr.Zero)
                {
                    return IntPtr.Zero;
                }
                IntPtr kaen = IL2CPP.il2cpp_type_get_object(naw);
                if (kaen == IntPtr.Zero)
                {
                    return IntPtr.Zero;
                }
                Marja marja = Marja.Qawi(kaen);
                anwaKaen[sanf] = marja;
                return marja.Kaen;
            }
            catch (KhataTaarib khata)
            {
                sijill.LogDebug(
                    $"rooting a reflection type object failed: {khata.Injilizi}");
                return IntPtr.Zero;
            }
        }

        /// <summary>
        /// The class name of one parameter of a resolved method, for separating
        /// two overloads that share an arity.
        /// </summary>
        /// <param name="tabia">The binding.</param>
        /// <param name="fahras">Which parameter, from zero.</param>
        /// <returns>The class's short name, or <c>null</c> when it cannot be read.</returns>
        public string? NawWaseet(in TabiaMahlula tabia, int fahras)
        {
            if (tabia.Tabia == IntPtr.Zero || fahras < 0)
            {
                return null;
            }
            try
            {
                IntPtr naw = IL2CPP.il2cpp_method_get_param(tabia.Tabia, (uint)fahras);
                if (naw == IntPtr.Zero)
                {
                    return null;
                }
                IntPtr sanf = IL2CPP.il2cpp_class_from_type(naw);
                if (sanf == IntPtr.Zero)
                {
                    return null;
                }
                IntPtr kham = IL2CPP.il2cpp_class_get_name(sanf);
                return kham == IntPtr.Zero ? null : Marshal.PtrToStringAnsi(kham);
            }
            catch (Exception)
            {
                return null;
            }
        }

        /// <summary>Releases every reflection type object this binding rooted.</summary>
        public void Dispose()
        {
            if (mutakhalla)
            {
                return;
            }
            mutakhalla = true;
            foreach (KeyValuePair<IntPtr, Marja> zawj in anwaKaen)
            {
                zawj.Value.Dispose();
            }
            anwaKaen.Clear();
            Muakkad = false;
        }

        // -------------------------------------------------------------------
        // The call shapes. Every one of these is the platform's ordinary
        // calling convention with the trailing MethodInfo* appended, which is
        // exactly the ABI IL2CPP compiled the target against.
        // -------------------------------------------------------------------

        /// <summary>Calls an instance method taking nothing and returning nothing.</summary>
        /// <param name="tabia">The binding.</param>
        /// <param name="kaen">The instance.</param>
        public static unsafe void Nida(in TabiaMahlula tabia, IntPtr kaen)
        {
            // SOUND: Unwan came from the ladder and is a compiled entry point in
            // this process's own image; the signature below is the target's
            // managed signature plus the trailing MethodInfo* IL2CPP appends to
            // every compiled method, which is the calling convention the target
            // was generated with. Callers check Wujid before arriving here.
            ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)tabia.Unwan)(kaen, tabia.Tabia);
        }

        /// <summary>Calls an instance method taking nothing and returning a value.</summary>
        /// <typeparam name="TNatija">The return type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="kaen">The instance.</param>
        /// <returns>What it returned.</returns>
        public static unsafe TNatija Qeema<TNatija>(in TabiaMahlula tabia, IntPtr kaen)
            where TNatija : unmanaged
        {
            // SOUND: as Nida. A blittable return type makes the runtime apply
            // the same aggregate-return rules the C++ compiler applied, so a
            // 16-byte Rect comes back through the hidden pointer on Windows x64
            // and in the SSE pair under System V, without this file encoding
            // either rule.
            return ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, TNatija>)tabia.Unwan)(
                kaen, tabia.Tabia);
        }

        /// <summary>Calls an instance method taking one argument.</summary>
        /// <typeparam name="T1">The argument type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="kaen">The instance.</param>
        /// <param name="awwal">The argument.</param>
        public static unsafe void Nida<T1>(in TabiaMahlula tabia, IntPtr kaen, T1 awwal)
            where T1 : unmanaged
        {
            // SOUND: as Nida.
            ((delegate* unmanaged[Cdecl]<IntPtr, T1, IntPtr, void>)tabia.Unwan)(
                kaen, awwal, tabia.Tabia);
        }

        /// <summary>Calls an instance method taking two arguments.</summary>
        /// <typeparam name="T1">The first argument type, blittable.</typeparam>
        /// <typeparam name="T2">The second argument type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="kaen">The instance.</param>
        /// <param name="awwal">The first argument.</param>
        /// <param name="thani">The second argument.</param>
        public static unsafe void Nida<T1, T2>(
            in TabiaMahlula tabia, IntPtr kaen, T1 awwal, T2 thani)
            where T1 : unmanaged
            where T2 : unmanaged
        {
            // SOUND: as Nida.
            ((delegate* unmanaged[Cdecl]<IntPtr, T1, T2, IntPtr, void>)tabia.Unwan)(
                kaen, awwal, thani, tabia.Tabia);
        }

        /// <summary>Calls an instance method taking three arguments.</summary>
        /// <typeparam name="T1">The first argument type, blittable.</typeparam>
        /// <typeparam name="T2">The second argument type, blittable.</typeparam>
        /// <typeparam name="T3">The third argument type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="kaen">The instance.</param>
        /// <param name="awwal">The first argument.</param>
        /// <param name="thani">The second argument.</param>
        /// <param name="thalith">The third argument.</param>
        public static unsafe void Nida<T1, T2, T3>(
            in TabiaMahlula tabia, IntPtr kaen, T1 awwal, T2 thani, T3 thalith)
            where T1 : unmanaged
            where T2 : unmanaged
            where T3 : unmanaged
        {
            // SOUND: as Nida.
            ((delegate* unmanaged[Cdecl]<IntPtr, T1, T2, T3, IntPtr, void>)tabia.Unwan)(
                kaen, awwal, thani, thalith, tabia.Tabia);
        }

        /// <summary>Calls an instance method taking one argument and returning a value.</summary>
        /// <typeparam name="T1">The argument type, blittable.</typeparam>
        /// <typeparam name="TNatija">The return type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="kaen">The instance.</param>
        /// <param name="awwal">The argument.</param>
        /// <returns>What it returned.</returns>
        public static unsafe TNatija Qeema<T1, TNatija>(
            in TabiaMahlula tabia, IntPtr kaen, T1 awwal)
            where T1 : unmanaged
            where TNatija : unmanaged
        {
            // SOUND: as Nida.
            return ((delegate* unmanaged[Cdecl]<IntPtr, T1, IntPtr, TNatija>)tabia.Unwan)(
                kaen, awwal, tabia.Tabia);
        }

        /// <summary>Calls a static method taking one argument and returning a value.</summary>
        /// <typeparam name="T1">The argument type, blittable.</typeparam>
        /// <typeparam name="TNatija">The return type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="awwal">The argument.</param>
        /// <returns>What it returned.</returns>
        /// <remarks>
        /// A static method has no implicit <c>this</c>, so the trailing
        /// <c>MethodInfo*</c> is the second argument rather than the third.
        /// Writing this shape with a placeholder <c>self</c> would shift every
        /// argument by one register and read the first parameter out of nothing.
        /// </remarks>
        public static unsafe TNatija QeemaSakina<T1, TNatija>(in TabiaMahlula tabia, T1 awwal)
            where T1 : unmanaged
            where TNatija : unmanaged
        {
            // SOUND: as Nida, minus the implicit this.
            return ((delegate* unmanaged[Cdecl]<T1, IntPtr, TNatija>)tabia.Unwan)(
                awwal, tabia.Tabia);
        }

        /// <summary>Calls a static method taking nothing and returning a value.</summary>
        /// <typeparam name="TNatija">The return type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <returns>What it returned.</returns>
        public static unsafe TNatija QeemaSakina<TNatija>(in TabiaMahlula tabia)
            where TNatija : unmanaged
        {
            // SOUND: as Nida, minus the implicit this.
            return ((delegate* unmanaged[Cdecl]<IntPtr, TNatija>)tabia.Unwan)(tabia.Tabia);
        }

        /// <summary>Calls a static method taking one argument and returning nothing.</summary>
        /// <typeparam name="T1">The argument type, blittable.</typeparam>
        /// <param name="tabia">The binding.</param>
        /// <param name="awwal">The argument.</param>
        public static unsafe void NidaSakina<T1>(in TabiaMahlula tabia, T1 awwal)
            where T1 : unmanaged
        {
            // SOUND: as Nida, minus the implicit this.
            ((delegate* unmanaged[Cdecl]<T1, IntPtr, void>)tabia.Unwan)(awwal, tabia.Tabia);
        }

        // -------------------------------------------------------------------
        // The engine, as operations rather than as addresses
        // -------------------------------------------------------------------

        /// <summary>Unity's current frame number, or -1 when it cannot be read.</summary>
        /// <returns>The frame count.</returns>
        public int Itar()
        {
            return itarHali.Wujid ? QeemaSakina<int>(in itarHali) : -1;
        }

        /// <summary>One engine object's instance identifier.</summary>
        /// <param name="kaen">The object.</param>
        /// <returns>Its identifier, or zero.</returns>
        /// <remarks>
        /// The identifier and not the pointer is what a takeover keys its
        /// ownership set by. A pointer is a heap address the collector owns and
        /// two objects at two times can share one; an instance identifier is
        /// stable for the object's life and means nothing to the collector,
        /// which is exactly the property a set held across frames needs.
        /// </remarks>
        public int Muarrif(IntPtr kaen)
        {
            if (kaen == IntPtr.Zero || !muarrifKaen.Wujid)
            {
                return 0;
            }
            return Qeema<int>(in muarrifKaen, kaen);
        }

        /// <summary>Destroys an object this plugin created.</summary>
        /// <param name="kaen">The object.</param>
        public void Atlif(IntPtr kaen)
        {
            if (kaen == IntPtr.Zero || !atlif.Wujid)
            {
                return;
            }
            NidaSakina(in atlif, kaen);
        }

        /// <summary>Finds a shader by name.</summary>
        /// <param name="ism">The shader's name.</param>
        /// <returns>The shader object, or zero when the game has no such shader.</returns>
        public IntPtr Sudfa(string ism)
        {
            if (!jidSudfa.Wujid)
            {
                return IntPtr.Zero;
            }
            // SOUND: the IL2CPP string is created and consumed inside this one
            // call with no allocating call between the two, which is the case
            // Maqbad/Marja.cs names as safe for a raw pointer. Rooting it would
            // install and release a GC handle for a value that never survives
            // the statement.
            IntPtr nass = IL2CPP.ManagedStringToIl2Cpp(ism);
            return nass == IntPtr.Zero
                ? IntPtr.Zero
                : QeemaSakina<IntPtr, IntPtr>(in jidSudfa, nass);
        }

        /// <summary>The shader property identifier for a name.</summary>
        /// <param name="ism">The property name, such as <c>_MainTex</c>.</param>
        /// <returns>Its identifier, or zero.</returns>
        public int MuarrifKhasiya(string ism)
        {
            if (!muarrifKhasiya.Wujid)
            {
                return 0;
            }
            // SOUND: as Sudfa.
            IntPtr nass = IL2CPP.ManagedStringToIl2Cpp(ism);
            return nass == IntPtr.Zero ? 0 : QeemaSakina<IntPtr, int>(in muarrifKhasiya, nass);
        }

        /// <summary>
        /// Creates a single-channel texture Taarib owns, with no mip chain and
        /// linear sampling.
        /// </summary>
        /// <param name="ard">Width in texels.</param>
        /// <param name="irtifa">Height in texels.</param>
        /// <returns>The texture object, or zero.</returns>
        /// <remarks>
        /// Linear rather than sRGB because the atlas holds coverage or a signed
        /// distance, both of which are quantities and not colours: marking it
        /// sRGB pushes every value through a gamma curve and makes thin strokes
        /// visibly lighter than thick ones. <c>HideAndDontSave</c> because a
        /// texture this plugin created must not be serialized into anybody's
        /// scene and must not be destroyed when one is unloaded.
        /// </remarks>
        public unsafe IntPtr LawhaJadida(int ard, int irtifa)
        {
            if (!lawhaBani.Wujid || sanfLawha == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }
            IntPtr kaen = IL2CPP.il2cpp_object_new(sanfLawha);
            if (kaen == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }

            // SOUND: the object was allocated one statement ago and nothing
            // between here and the constructor call allocates, so the collector
            // has had no opportunity to move or reclaim it. TextureFormat.R8 is
            // 63 and FilterMode.Bilinear is 1 and TextureWrapMode.Clamp is 1 in
            // every engine version this plugin runs in; they are enum values in
            // the engine's own public API, which is versioned and does not
            // renumber.
            ((delegate* unmanaged[Cdecl]<IntPtr, int, int, int, byte, byte, IntPtr, void>)
                lawhaBani.Unwan)(kaen, ard, irtifa, 63, 0, 1, lawhaBani.Tabia);

            if (lawhaTasfiya.Wujid)
            {
                Nida(in lawhaTasfiya, kaen, 1);
            }
            if (lawhaLaff.Wujid)
            {
                Nida(in lawhaLaff, kaen, 1);
            }
            Ikhfi(kaen);
            return kaen;
        }

        /// <summary>Uploads one page of texels into a texture and applies it.</summary>
        /// <param name="lawha">The texture.</param>
        /// <param name="texelat">The texels, one byte each.</param>
        /// <returns>Whether the upload ran.</returns>
        public unsafe bool IrfaTexelat(IntPtr lawha, ReadOnlySpan<byte> texelat)
        {
            if (lawha == IntPtr.Zero || !lawhaIrfa.Wujid || texelat.IsEmpty)
            {
                return false;
            }
            fixed (byte* asas = texelat)
            {
                // SOUND: the pointer is handed to LoadRawTextureData, which
                // copies the bytes synchronously before returning, and the fixed
                // block keeps the managed page alive and unmoved for exactly
                // that call. Nothing retains the address afterwards.
                Nida(in lawhaIrfa, lawha, (IntPtr)asas, texelat.Length);
            }
            if (lawhaTabbiq.Wujid)
            {
                Nida(in lawhaTabbiq, lawha, (byte)0, (byte)0);
            }
            return true;
        }

        /// <summary>Creates a material over a shader, owned by Taarib.</summary>
        /// <param name="sudfa">The shader.</param>
        /// <returns>The material object, or zero.</returns>
        public IntPtr MaddaJadida(IntPtr sudfa)
        {
            if (sudfa == IntPtr.Zero || !maddaBani.Wujid || sanfMadda == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }
            IntPtr kaen = IL2CPP.il2cpp_object_new(sanfMadda);
            if (kaen == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }
            // SOUND: as LawhaJadida — allocated one statement ago, nothing
            // allocating in between.
            Nida(in maddaBani, kaen, sudfa);
            Ikhfi(kaen);
            return kaen;
        }

        /// <summary>Binds the atlas page a material samples.</summary>
        /// <param name="madda">The material.</param>
        /// <param name="muarrif">The property identifier for <c>_MainTex</c>.</param>
        /// <param name="lawha">The texture.</param>
        public void DaAlLawha(IntPtr madda, int muarrif, IntPtr lawha)
        {
            if (madda == IntPtr.Zero || lawha == IntPtr.Zero)
            {
                return;
            }
            if (maddaBiMuarrif && maddaLawhaBiMuarrif.Wujid)
            {
                Nida(in maddaLawhaBiMuarrif, madda, muarrif, lawha);
                return;
            }
            if (maddaLawhaAsasiya.Wujid)
            {
                Nida(in maddaLawhaAsasiya, madda, lawha);
            }
        }

        /// <summary>Sets a material's tint.</summary>
        /// <param name="madda">The material.</param>
        /// <param name="lawn">The colour.</param>
        /// <remarks>
        /// Always opaque white in this plugin. The real colour is per vertex —
        /// <see cref="Nasij"/> resolves it from the style span so a colour tag
        /// survives bidirectional reordering — and a material tint would
        /// multiply over the top of that and flatten every coloured word in the
        /// game.
        /// </remarks>
        public void DaAlLawn(IntPtr madda, LawnKamil lawn)
        {
            if (madda == IntPtr.Zero || !maddaLawn.Wujid)
            {
                return;
            }
            Nida(in maddaLawn, madda, lawn);
        }

        /// <summary>Sets one float shader parameter.</summary>
        /// <param name="madda">The material.</param>
        /// <param name="muarrif">The property identifier.</param>
        /// <param name="qeema">The value.</param>
        /// <returns>Whether it could be set.</returns>
        public bool DaAlAshari(IntPtr madda, int muarrif, float qeema)
        {
            if (madda == IntPtr.Zero || !maddaAshari.Wujid)
            {
                return false;
            }
            Nida(in maddaAshari, madda, muarrif, qeema);
            return true;
        }

        /// <summary>Sets one vector shader parameter.</summary>
        /// <param name="madda">The material.</param>
        /// <param name="muarrif">The property identifier.</param>
        /// <param name="qeema">The value.</param>
        /// <returns>Whether it could be set.</returns>
        public bool DaAlMuttajih(IntPtr madda, int muarrif, Muttajih4 qeema)
        {
            if (madda == IntPtr.Zero || !maddaMuttajih.Wujid)
            {
                return false;
            }
            Nida(in maddaMuttajih, madda, muarrif, qeema);
            return true;
        }

        /// <summary>Finds one component on the same object as another.</summary>
        /// <param name="juz">The component to search from.</param>
        /// <param name="sanf">The component class wanted.</param>
        /// <returns>The component, or zero when there is none.</returns>
        /// <remarks>
        /// <c>TryGetComponent</c> is preferred and allocates nothing.
        /// <c>GetComponents</c> is the fallback for engines older than it, and
        /// allocates one IL2CPP array per call — which is named here rather than
        /// hidden, because it is the one allocation this plugin makes on a draw
        /// path and it exists only on engines where there is no alternative.
        /// </remarks>
        public unsafe IntPtr Mukawwin(IntPtr juz, IntPtr sanf)
        {
            if (juz == IntPtr.Zero || sanf == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }
            IntPtr naw = NawKaen(sanf);
            if (naw == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }

            if (jarrib.Wujid)
            {
                IntPtr natija = IntPtr.Zero;
                // SOUND: the out parameter is an IL2CPP `Component**` and the
                // address handed over is a local on this thread's stack, which
                // the runtime writes once and does not retain. The local is read
                // immediately afterwards with nothing allocating in between.
                byte wujid =
                    ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, IntPtr*, IntPtr, byte>)
                        jarrib.Unwan)(juz, naw, &natija, jarrib.Tabia);
                return wujid != 0 ? natija : IntPtr.Zero;
            }

            if (!jamiMukawwinat.Wujid)
            {
                return IntPtr.Zero;
            }
            IntPtr masfufa = Qeema<IntPtr, IntPtr>(in jamiMukawwinat, juz, naw);
            if (masfufa == IntPtr.Zero)
            {
                return IntPtr.Zero;
            }
            // SOUND: the array was produced by the call one statement ago and
            // nothing allocates before its first element is read. The element
            // offset is derived from IntPtr.Size rather than hard-coded, and the
            // length is read from the runtime rather than assumed.
            if (IL2CPP.il2cpp_array_length(masfufa) == 0)
            {
                return IntPtr.Zero;
            }
            return *(IntPtr*)((byte*)masfufa + IzahatUnsur);
        }

        /// <summary>The transform of the object a component sits on.</summary>
        /// <param name="juz">The component.</param>
        /// <returns>The transform, or zero.</returns>
        public IntPtr Tahwil(IntPtr juz)
        {
            if (juz == IntPtr.Zero || !tahwil.Wujid)
            {
                return IntPtr.Zero;
            }
            return Qeema<IntPtr>(in tahwil, juz);
        }

        /// <summary>How many children a transform has.</summary>
        /// <param name="tahwilHali">The transform.</param>
        /// <returns>The child count, or zero.</returns>
        public int AdadAtfal(IntPtr tahwilHali)
        {
            if (tahwilHali == IntPtr.Zero || !adadAtfal.Wujid)
            {
                return 0;
            }
            return Qeema<int>(in adadAtfal, tahwilHali);
        }

        /// <summary>One child of a transform.</summary>
        /// <param name="tahwilHali">The transform.</param>
        /// <param name="fahras">Which child.</param>
        /// <returns>The child transform, or zero.</returns>
        public IntPtr Tifl(IntPtr tahwilHali, int fahras)
        {
            if (tahwilHali == IntPtr.Zero || !tifl.Wujid)
            {
                return IntPtr.Zero;
            }
            return Qeema<int, IntPtr>(in tifl, tahwilHali, fahras);
        }

        /// <summary>A rect transform's local rectangle.</summary>
        /// <param name="mustatilHali">The rect transform.</param>
        /// <returns>The rectangle, or all zeroes.</returns>
        public MustatilMuharrik Mustatil(IntPtr mustatilHali)
        {
            if (mustatilHali == IntPtr.Zero || !mustatil.Wujid)
            {
                return default;
            }
            return Qeema<MustatilMuharrik>(in mustatil, mustatilHali);
        }

        /// <summary>A rect transform's pivot, in normalized rectangle units.</summary>
        /// <param name="mustatilHali">The rect transform.</param>
        /// <returns>The pivot, or all zeroes.</returns>
        public Muttajih2 Mihwar(IntPtr mustatilHali)
        {
            if (mustatilHali == IntPtr.Zero || !mihwar.Wujid)
            {
                return default;
            }
            return Qeema<Muttajih2>(in mihwar, mustatilHali);
        }

        /// <summary>Empties a mesh of the geometry it currently holds.</summary>
        /// <param name="nasij">The mesh.</param>
        /// <remarks>
        /// Before every write, because the indices still on the mesh describe
        /// the previous string and would name vertices past the end of the array
        /// about to be assigned.
        /// </remarks>
        public void ImsahNasij(IntPtr nasij)
        {
            if (nasij == IntPtr.Zero || !nasijImsah.Wujid)
            {
                return;
            }
            Nida(in nasijImsah, nasij, (byte)0);
        }

        /// <summary>Assigns the four vertex streams and the bounding box.</summary>
        /// <param name="nasij">The mesh.</param>
        /// <param name="ruus">The positions array.</param>
        /// <param name="malamis">The texture coordinates array.</param>
        /// <param name="alwan">The colours array.</param>
        /// <param name="muthallathat">The index array.</param>
        /// <param name="hudud">The bounding box, as a centre and a half-extent.</param>
        /// <remarks>
        /// The bounds are assigned rather than recalculated. The arrays are
        /// quantized and their cleared tail sits at the origin, so a recalculated
        /// box would stretch from the text to the pivot and the label would
        /// disappear at the edge of the view frustum.
        /// </remarks>
        public void AktubNasij(
            IntPtr nasij,
            IntPtr ruus,
            IntPtr malamis,
            IntPtr alwan,
            IntPtr muthallathat,
            HududMuharrik hudud)
        {
            if (nasij == IntPtr.Zero)
            {
                return;
            }
            if (nasijRuus.Wujid && ruus != IntPtr.Zero)
            {
                Nida(in nasijRuus, nasij, ruus);
            }
            if (nasijMalamis.Wujid && malamis != IntPtr.Zero)
            {
                Nida(in nasijMalamis, nasij, malamis);
            }
            if (nasijAlwan.Wujid && alwan != IntPtr.Zero)
            {
                Nida(in nasijAlwan, nasij, alwan);
            }
            if (nasijMuthallathat.Wujid && muthallathat != IntPtr.Zero)
            {
                Nida(in nasijMuthallathat, nasij, muthallathat);
            }
            if (nasijHudud.Wujid)
            {
                Nida(in nasijHudud, nasij, hudud);
            }
        }

        /// <summary>Creates an IL2CPP array of a given element class.</summary>
        /// <param name="sanfUnsur">The element class.</param>
        /// <param name="adad">How many elements.</param>
        /// <returns>The array object, or zero.</returns>
        public IntPtr MasfufaJadida(IntPtr sanfUnsur, int adad)
        {
            if (sanfUnsur == IntPtr.Zero || adad < 0)
            {
                return IntPtr.Zero;
            }
            return IL2CPP.il2cpp_array_new(sanfUnsur, (ulong)(uint)adad);
        }

        /// <summary>Copies a span into an IL2CPP array's elements.</summary>
        /// <typeparam name="T">The element type, blittable and matching the array's.</typeparam>
        /// <param name="masfufa">The array object.</param>
        /// <param name="masdar">What to copy.</param>
        /// <returns>Whether the copy ran.</returns>
        /// <remarks>
        /// One bulk copy per stream per draw, into an array whose length is the
        /// quantized capacity and therefore changes only when
        /// <see cref="MakhzanRusum.Wassi"/> grows it. Writing the geometry
        /// straight into the IL2CPP array instead would mean pinning four
        /// objects for the life of the takeover, which costs the collector more
        /// than the copy costs the frame.
        /// </remarks>
        public unsafe bool Ansikh<T>(IntPtr masfufa, ReadOnlySpan<T> masdar)
            where T : unmanaged
        {
            if (masfufa == IntPtr.Zero)
            {
                return false;
            }
            uint tul = IL2CPP.il2cpp_array_length(masfufa);
            if (tul < (uint)masdar.Length)
            {
                return false;
            }
            // SOUND: the array pointer was re-read from a live root by the
            // caller in this same call, the element offset is the IL2CPP array
            // header expressed in pointer-sized units, and the destination
            // length is the length the runtime itself reports. Nothing between
            // forming the span and finishing the copy allocates.
            Span<T> hadaf = new Span<T>((byte*)masfufa + IzahatUnsur, (int)tul);
            masdar.CopyTo(hadaf);
            return true;
        }

        /// <summary>The element class for a mesh position array.</summary>
        public IntPtr SanfMuttajih3 => sanfMuttajih3;

        /// <summary>The element class for a mesh texture-coordinate array.</summary>
        public IntPtr SanfMuttajih2 => sanfMuttajih2;

        /// <summary>The element class for a mesh colour array.</summary>
        public IntPtr SanfLawn32 => sanfLawn32;

        /// <summary>The element class for a mesh index array.</summary>
        public IntPtr SanfSahih => sanfSahih;

        /// <summary>Gives a canvas renderer the mesh, material and atlas Taarib built.</summary>
        /// <param name="rassam">The canvas renderer.</param>
        /// <param name="nasij">The mesh, or zero to leave it alone.</param>
        /// <param name="madda">The material.</param>
        /// <param name="lawha">The atlas page, or zero.</param>
        /// <returns>Whether the material was bound.</returns>
        public bool Aabbir(IntPtr rassam, IntPtr nasij, IntPtr madda, IntPtr lawha)
        {
            if (rassam == IntPtr.Zero || madda == IntPtr.Zero)
            {
                return false;
            }
            if (nasij != IntPtr.Zero && rassamNasij.Wujid)
            {
                Nida(in rassamNasij, rassam, nasij);
            }
            if (!rassamMadda.Wujid)
            {
                return false;
            }
            if (rassamMaddaBiLawha)
            {
                // The overload this engine exposes takes the texture rather than
                // an index, and binds both in one call.
                Nida(in rassamMadda, rassam, madda, lawha);
                return true;
            }
            if (rassamAdadMawad.Wujid)
            {
                Nida(in rassamAdadMawad, rassam, 1);
            }
            Nida(in rassamMadda, rassam, madda, 0);
            if (lawha != IntPtr.Zero && rassamLawha.Wujid)
            {
                Nida(in rassamLawha, rassam, lawha);
            }
            return true;
        }

        /// <summary>Empties a canvas renderer, so a sub-mesh child draws nothing.</summary>
        /// <param name="rassam">The canvas renderer.</param>
        public void ImsahRassam(IntPtr rassam)
        {
            if (rassam == IntPtr.Zero || !rassamImsah.Wujid)
            {
                return;
            }
            Nida(in rassamImsah, rassam);
        }

        /// <summary>Points a mesh filter at a mesh.</summary>
        /// <param name="murashshih">The mesh filter.</param>
        /// <param name="nasij">The mesh.</param>
        public void DaAlNasijMushtarak(IntPtr murashshih, IntPtr nasij)
        {
            if (murashshih == IntPtr.Zero || nasij == IntPtr.Zero || !murashshihNasij.Wujid)
            {
                return;
            }
            Nida(in murashshihNasij, murashshih, nasij);
        }

        /// <summary>The mesh a mesh filter currently points at.</summary>
        /// <param name="murashshih">The mesh filter.</param>
        /// <returns>The mesh, or zero.</returns>
        /// <remarks>
        /// Read rather than written on the sub-mesh path: a TextMeshPro sub-mesh
        /// child has to be emptied of the geometry TMP generated for the
        /// untranslated string, and emptying its own mesh is the one way to do
        /// that without destroying an object TMP will reuse.
        /// </remarks>
        public IntPtr NasijMushtarak(IntPtr murashshih)
        {
            if (murashshih == IntPtr.Zero || !murashshihNasijQira.Wujid)
            {
                return IntPtr.Zero;
            }
            return Qeema<IntPtr>(in murashshihNasijQira, murashshih);
        }

        /// <summary>A renderer's current shared material.</summary>
        /// <param name="mujassam">The renderer.</param>
        /// <returns>The material, or zero.</returns>
        public IntPtr MaddaMushtaraka(IntPtr mujassam)
        {
            if (mujassam == IntPtr.Zero || !mujassamMaddaQira.Wujid)
            {
                return IntPtr.Zero;
            }
            return Qeema<IntPtr>(in mujassamMaddaQira, mujassam);
        }

        /// <summary>Replaces a renderer's shared material.</summary>
        /// <param name="mujassam">The renderer.</param>
        /// <param name="madda">The material.</param>
        public void DaAlMaddaMushtaraka(IntPtr mujassam, IntPtr madda)
        {
            if (mujassam == IntPtr.Zero || !mujassamMaddaKitaba.Wujid)
            {
                return;
            }
            Nida(in mujassamMaddaKitaba, mujassam, madda);
        }

        private IntPtr sanfLawha;
        private IntPtr sanfMadda;

        private void Ikhfi(IntPtr kaen)
        {
            if (!alamatIkhfa.Wujid)
            {
                return;
            }
            // HideFlags.HideAndDontSave is 61: hidden in the hierarchy, not
            // saved with a scene, and not destroyed when one is unloaded. A
            // texture this plugin created must satisfy all three or it becomes
            // part of somebody's save data.
            Nida(in alamatIkhfa, kaen, 61);
        }

        private bool IbniSuwar()
        {
            IntPtr majal;
            try
            {
                majal = IL2CPP.il2cpp_domain_get();
            }
            catch (Exception khata)
            {
                sijill.LogWarning(
                    "تعذّر الوصول إلى زمن تشغيل IL2CPP؛ أُوقف الاستيلاء على النصوص. | The "
                    + $"IL2CPP runtime could not be reached ({khata.GetType().Name}); the "
                    + "text takeovers are off.");
                return false;
            }

            if (majal == IntPtr.Zero)
            {
                sijill.LogWarning(
                    "زمن تشغيل IL2CPP لا يعلن أي مجال؛ أُوقف الاستيلاء على النصوص. | The "
                    + "IL2CPP runtime reports no domain; the text takeovers are off.");
                return false;
            }

            uint adad = 0;
            try
            {
                unsafe
                {
                    // SOUND: the runtime returns a pointer to its own array of
                    // Il2CppAssembly* and writes the length through the ref
                    // parameter. The array belongs to the domain and outlives
                    // this loop by the whole process; nothing is written through
                    // it and the loop is bounded by the runtime's own count.
                    IntPtr* tajammuat = IL2CPP.il2cpp_domain_get_assemblies(majal, ref adad);
                    if (tajammuat == null || adad == 0)
                    {
                        sijill.LogWarning(
                            "لا يعلن مجال IL2CPP أي تجميعة؛ أُوقف الاستيلاء على النصوص. | The "
                            + "IL2CPP domain reports no assemblies; the takeovers are off.");
                        return false;
                    }
                    for (uint i = 0; i < adad; i++)
                    {
                        IntPtr sura = tajammuat[i] == IntPtr.Zero
                            ? IntPtr.Zero
                            : IL2CPP.il2cpp_assembly_get_image(tajammuat[i]);
                        if (sura == IntPtr.Zero)
                        {
                            continue;
                        }
                        kulSuwar.Add(sura);
                        SajjilSura(sura);
                    }
                }
            }
            catch (Exception khata)
            {
                sijill.LogWarning(
                    "تعذّر تعداد تجميعات IL2CPP؛ أُوقف الاستيلاء على النصوص. | Enumerating "
                    + $"the IL2CPP assemblies threw {khata.GetType().Name}; the takeovers "
                    + "are off.");
                return false;
            }

            return kulSuwar.Count != 0;
        }

        private void SajjilSura(IntPtr sura)
        {
            IntPtr kham = IL2CPP.il2cpp_image_get_name(sura);
            string? ism = kham == IntPtr.Zero ? null : Marshal.PtrToStringAnsi(kham);
            if (string.IsNullOrEmpty(ism))
            {
                return;
            }
            string mujarrad = ism!;
            int nuqta = mujarrad.LastIndexOf('.');
            if (nuqta > 0)
            {
                mujarrad = mujarrad.Substring(0, nuqta);
            }
            mujarrad = mujarrad.ToLowerInvariant();
            if (!suwar.TryGetValue(mujarrad, out List<IntPtr>? qaima))
            {
                qaima = new List<IntPtr>(1);
                suwar[mujarrad] = qaima;
            }
            qaima.Add(sura);
        }

        private void Iqrar()
        {
            jidSudfa = Hall(TajammuAsas, FadaaMuharrik, "Shader", "Find", 1, string.Empty);
            muarrifKhasiya = Hall(
                TajammuAsas, FadaaMuharrik, "Shader", "PropertyToID", 1, string.Empty);
            atlif = Hall(TajammuAsas, FadaaMuharrik, "Object", "Destroy", 1, string.Empty);
            muarrifKaen = Hall(
                TajammuAsas, FadaaMuharrik, "Object", "GetInstanceID", 0, string.Empty);
            alamatIkhfa = Hall(
                TajammuAsas, FadaaMuharrik, "Object", "set_hideFlags", 1, string.Empty);
            itarHali = Hall(
                TajammuAsas, FadaaMuharrik, "Time", "get_frameCount", 0, string.Empty);

            sanfLawha = Sanf(TajammuAsas, FadaaMuharrik, "Texture2D");
            sanfMadda = Sanf(TajammuAsas, FadaaMuharrik, "Material");
            lawhaBani = Hall(TajammuAsas, FadaaMuharrik, "Texture2D", ".ctor", 5, string.Empty);
            lawhaIrfa = Hall(
                TajammuAsas, FadaaMuharrik, "Texture2D", "LoadRawTextureData", 2, string.Empty);
            lawhaTabbiq = Hall(
                TajammuAsas, FadaaMuharrik, "Texture2D", "Apply", 2, string.Empty);
            lawhaTasfiya = Hall(
                TajammuAsas, FadaaMuharrik, "Texture", "set_filterMode", 1, string.Empty);
            lawhaLaff = Hall(
                TajammuAsas, FadaaMuharrik, "Texture", "set_wrapMode", 1, string.Empty);

            maddaBani = Hall(TajammuAsas, FadaaMuharrik, "Material", ".ctor", 1, string.Empty);
            maddaLawhaAsasiya = Hall(
                TajammuAsas, FadaaMuharrik, "Material", "set_mainTexture", 1, string.Empty);
            maddaLawn = Hall(
                TajammuAsas, FadaaMuharrik, "Material", "set_color", 1, string.Empty);

            // Unity funnels both public SetFloat overloads into one private
            // method whose arity is unique, which is the only form of these that
            // can be resolved without risking the string overload. Where the
            // engine is older than that funnel, the two shader parameters a
            // distance-field patch needs cannot be set and MasdarAshkal refuses
            // that patch by name rather than drawing it with unset parameters,
            // which renders as solid rectangles.
            maddaAshari = Hall(
                TajammuAsas, FadaaMuharrik, "Material", "SetFloatImpl", 2, string.Empty);
            maddaMuttajih = Hall(
                TajammuAsas, FadaaMuharrik, "Material", "SetVectorImpl", 2, string.Empty);
            maddaLawhaBiMuarrif = Hall(
                TajammuAsas, FadaaMuharrik, "Material", "SetTextureImpl", 2, string.Empty);
            maddaBiMuarrif = maddaLawhaBiMuarrif.Wujid
                && string.Equals(NawWaseet(in maddaLawhaBiMuarrif, 0), "Int32",
                    StringComparison.Ordinal);

            jarrib = Hall(
                TajammuAsas, FadaaMuharrik, "Component", "TryGetComponent", 2, string.Empty);
            jamiMukawwinat = Hall(
                TajammuAsas, FadaaMuharrik, "Component", "GetComponents", 1, string.Empty);
            tahwil = Hall(
                TajammuAsas, FadaaMuharrik, "Component", "get_transform", 0, string.Empty);
            adadAtfal = Hall(
                TajammuAsas, FadaaMuharrik, "Transform", "get_childCount", 0, string.Empty);
            tifl = Hall(TajammuAsas, FadaaMuharrik, "Transform", "GetChild", 1, string.Empty);

            mustatil = Hall(
                TajammuAsas, FadaaMuharrik, "RectTransform", "get_rect", 0, string.Empty);
            mihwar = Hall(
                TajammuAsas, FadaaMuharrik, "RectTransform", "get_pivot", 0, string.Empty);

            nasijImsah = Hall(TajammuAsas, FadaaMuharrik, "Mesh", "Clear", 1, string.Empty);
            nasijRuus = Hall(
                TajammuAsas, FadaaMuharrik, "Mesh", "set_vertices", 1, string.Empty);
            nasijMalamis = Hall(TajammuAsas, FadaaMuharrik, "Mesh", "set_uv", 1, string.Empty);
            nasijAlwan = Hall(
                TajammuAsas, FadaaMuharrik, "Mesh", "set_colors32", 1, string.Empty);
            nasijMuthallathat = Hall(
                TajammuAsas, FadaaMuharrik, "Mesh", "set_triangles", 1, string.Empty);
            nasijHudud = Hall(
                TajammuAsas, FadaaMuharrik, "Mesh", "set_bounds", 1, string.Empty);

            rassamNasij = Hall(
                TajammuWajiha, FadaaMuharrik, "CanvasRenderer", "SetMesh", 1, string.Empty);
            rassamAdadMawad = Hall(
                TajammuWajiha, FadaaMuharrik, "CanvasRenderer", "set_materialCount", 1,
                string.Empty);
            rassamMadda = Hall(
                TajammuWajiha, FadaaMuharrik, "CanvasRenderer", "SetMaterial", 2, string.Empty);
            rassamLawha = Hall(
                TajammuWajiha, FadaaMuharrik, "CanvasRenderer", "SetTexture", 1, string.Empty);
            rassamImsah = Hall(
                TajammuWajiha, FadaaMuharrik, "CanvasRenderer", "Clear", 0, string.Empty);

            // SetMaterial has two overloads of the same arity — (Material, int)
            // and (Material, Texture) — so which one resolved decides the call
            // shape rather than being assumed. Passing a texture pointer where
            // an index was expected is not a caught error.
            rassamMaddaBiLawha = rassamMadda.Wujid
                && !string.Equals(
                    NawWaseet(in rassamMadda, 1), "Int32", StringComparison.Ordinal);

            murashshihNasij = Hall(
                TajammuAsas, FadaaMuharrik, "MeshFilter", "set_sharedMesh", 1, string.Empty);
            murashshihNasijQira = Hall(
                TajammuAsas, FadaaMuharrik, "MeshFilter", "get_sharedMesh", 0, string.Empty);
            mujassamMaddaQira = Hall(
                TajammuAsas, FadaaMuharrik, "Renderer", "get_sharedMaterial", 0, string.Empty);
            mujassamMaddaKitaba = Hall(
                TajammuAsas, FadaaMuharrik, "Renderer", "set_sharedMaterial", 1, string.Empty);

            sanfMuttajih3 = Sanf(TajammuAsas, FadaaMuharrik, "Vector3");
            sanfMuttajih2 = Sanf(TajammuAsas, FadaaMuharrik, "Vector2");
            sanfLawn32 = Sanf(TajammuAsas, FadaaMuharrik, "Color32");
            sanfSahih = Sanf("mscorlib", "System", "Int32");
            sanfRassam = Sanf(TajammuWajiha, FadaaMuharrik, "CanvasRenderer");
            sanfMurashshih = Sanf(TajammuAsas, FadaaMuharrik, "MeshFilter");
            sanfMujassam = Sanf(TajammuAsas, FadaaMuharrik, "MeshRenderer");

            Muakkad = jidSudfa.Wujid
                && lawhaBani.Wujid
                && lawhaIrfa.Wujid
                && maddaBani.Wujid
                && (maddaBiMuarrif || maddaLawhaAsasiya.Wujid)
                && (jarrib.Wujid || jamiMukawwinat.Wujid)
                && mustatil.Wujid
                && sanfLawha != IntPtr.Zero
                && sanfMadda != IntPtr.Zero;
        }
    }

    /// <summary>
    /// مخزن الرسوم — the pooled geometry one takeover writes a mesh through,
    /// on both sides of the IL2CPP boundary.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Two sets of buffers, and why.</b> <see cref="Nasij"/> writes into
    /// spans of its own struct types, so the managed arrays here are declared
    /// as exactly those types and handed over with no cast and no conversion.
    /// Unity's mesh setters take IL2CPP arrays, which live in the other heap
    /// and cannot be spanned into from managed code without being rooted. So
    /// the geometry is built into the managed arrays and copied once per stream
    /// per draw into IL2CPP arrays that are created only when the capacity
    /// grows. One bulk copy of a few kilobytes is cheaper than pinning four
    /// objects for the life of the takeover, which is what writing straight
    /// into the IL2CPP arrays would require and which a compacting collector
    /// has to arrange the whole heap around.
    /// </para>
    /// <para>
    /// <b>Why the capacity is quantized.</b> Unity's oldest and most portable
    /// mesh API derives the vertex count from the array's length, so a buffer
    /// that is merely large enough would draw its tail as garbage. Growing to
    /// an exact fit would allocate four arrays in each heap every time a label
    /// gains or loses a glyph, which on a dialogue box is every frame. So
    /// capacity is rounded up to a multiple of <see cref="Kutla"/> glyphs and
    /// the tail past what was written is cleared: cleared positions collapse to
    /// the origin, cleared colours are transparent, and cleared indices name a
    /// degenerate triangle the rasterizer discards.
    /// </para>
    /// <para>
    /// <b>What allocates.</b> <see cref="Wassi"/> and <see cref="Hawwil"/>,
    /// and only when something is longer than anything before it. Every other
    /// member here is copy-only.
    /// </para>
    /// </remarks>
    public sealed class MakhzanRusum : IDisposable
    {
        /// <summary>
        /// How many glyphs the capacity is rounded up to. Sixteen: small enough
        /// that a short label does not carry a page of dead vertices, large
        /// enough that a typewriter effect revealing one letter per frame
        /// re-allocates once every sixteen frames rather than every frame.
        /// </summary>
        public const int Kutla = 16;

        private NuqtaRasm[] ruus;
        private NuqtaMulmas[] malamis;
        private LawnRasm[] alwan;
        private int[] muthallathat;
        private DharraMawduaa[] dharrat;
        private TaaribNitaqUslub[] nitaqat;

        private Marja? masfufatRuus;
        private Marja? masfufatMalamis;
        private Marja? masfufatAlwan;
        private Marja? masfufatMuthallathat;

        private int siaatAshkal;

        /// <summary>Creates the buffers empty; the first draw sizes them.</summary>
        public MakhzanRusum()
        {
            ruus = Array.Empty<NuqtaRasm>();
            malamis = Array.Empty<NuqtaMulmas>();
            alwan = Array.Empty<LawnRasm>();
            muthallathat = Array.Empty<int>();
            dharrat = new DharraMawduaa[8];
            nitaqat = new TaaribNitaqUslub[8];
        }

        /// <summary>The glyph capacity currently allocated.</summary>
        public int SiaatAshkal => siaatAshkal;

        /// <summary>The IL2CPP positions array, re-read from its root.</summary>
        public IntPtr Ruus => masfufatRuus?.Kaen ?? IntPtr.Zero;

        /// <summary>The IL2CPP texture-coordinate array, re-read from its root.</summary>
        public IntPtr Malamis => masfufatMalamis?.Kaen ?? IntPtr.Zero;

        /// <summary>The IL2CPP colour array, re-read from its root.</summary>
        public IntPtr Alwan => masfufatAlwan?.Kaen ?? IntPtr.Zero;

        /// <summary>The IL2CPP index array, re-read from its root.</summary>
        public IntPtr Muthallathat => masfufatMuthallathat?.Kaen ?? IntPtr.Zero;

        /// <summary>The positions, as the span a build writes into.</summary>
        public ReadOnlySpan<NuqtaRasm> RuusMahalliya => ruus;

        /// <summary>The texture coordinates, as a span.</summary>
        public ReadOnlySpan<NuqtaMulmas> MalamisMahalliya => malamis;

        /// <summary>The colours, as a span.</summary>
        public ReadOnlySpan<LawnRasm> AlwanMahalliya => alwan;

        /// <summary>The indices, as a span.</summary>
        public ReadOnlySpan<int> MuthallathatMahalliya => muthallathat;

        /// <summary>
        /// Grows both sets of buffers so that <paramref name="adadHuruf"/>
        /// glyphs fit, rounded up to <see cref="Kutla"/>.
        /// </summary>
        /// <param name="adadHuruf">How many glyphs the layout holds.</param>
        /// <param name="wasl">The engine binding, for the IL2CPP arrays.</param>
        /// <returns>Whether both sets are present at the requested capacity.</returns>
        /// <remarks>
        /// <see cref="Nasij"/> demands four vertices and six indices per glyph
        /// as an upper bound, because knowing the exact count would cost an
        /// atlas lookup per glyph before the first vertex is written.
        /// </remarks>
        public bool Wassi(int adadHuruf, WaslMuharrik wasl)
        {
            if (wasl is null)
            {
                throw new ArgumentNullException(nameof(wasl));
            }
            if (adadHuruf <= siaatAshkal && masfufatRuus is not null)
            {
                return true;
            }

            int matlub = ((adadHuruf + Kutla - 1) / Kutla) * Kutla;
            if (matlub < Kutla)
            {
                matlub = Kutla;
            }
            int adadRuus = matlub * Nasij.RuusLiShakl;
            int adadFahras = matlub * Nasij.FahrasLiShakl;

            ruus = new NuqtaRasm[adadRuus];
            malamis = new NuqtaMulmas[adadRuus];
            alwan = new LawnRasm[adadRuus];
            muthallathat = new int[adadFahras];

            Marja? jadidRuus = Ihjiz(wasl, wasl.SanfMuttajih3, adadRuus);
            Marja? jadidMalamis = Ihjiz(wasl, wasl.SanfMuttajih2, adadRuus);
            Marja? jadidAlwan = Ihjiz(wasl, wasl.SanfLawn32, adadRuus);
            Marja? jadidFahras = Ihjiz(wasl, wasl.SanfSahih, adadFahras);
            if (jadidRuus is null || jadidMalamis is null
                || jadidAlwan is null || jadidFahras is null)
            {
                jadidRuus?.Dispose();
                jadidMalamis?.Dispose();
                jadidAlwan?.Dispose();
                jadidFahras?.Dispose();
                return false;
            }

            // The previous roots are released only once the new ones exist, so
            // a failed growth leaves the takeover drawing at the old capacity
            // rather than holding four null arrays.
            masfufatRuus?.Dispose();
            masfufatMalamis?.Dispose();
            masfufatAlwan?.Dispose();
            masfufatMuthallathat?.Dispose();

            masfufatRuus = jadidRuus;
            masfufatMalamis = jadidMalamis;
            masfufatAlwan = jadidAlwan;
            masfufatMuthallathat = jadidFahras;
            siaatAshkal = matlub;
            return true;
        }

        /// <summary>
        /// The destination <see cref="Nasij.Ibni{TKhareeta}"/> writes into.
        /// </summary>
        /// <returns>The destination, as spans over the managed arrays.</returns>
        public MakhzanNasij Makhzan()
        {
            MakhzanNasij makhzan = default;
            makhzan.Ruus = ruus.AsSpan();
            makhzan.Malamis = malamis.AsSpan();
            makhzan.Alwan = alwan.AsSpan();
            makhzan.Muthallathat = muthallathat.AsSpan();
            makhzan.Dharrat = dharrat.AsSpan();
            return makhzan;
        }

        /// <summary>
        /// Clears everything past what a build wrote, so the fixed-length arrays
        /// a mesh upload takes carry no stale geometry from the previous string.
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
        /// Copies the built geometry into the IL2CPP arrays the mesh setters
        /// take.
        /// </summary>
        /// <param name="wasl">The engine binding.</param>
        /// <returns>Whether all four streams crossed.</returns>
        public bool Anfidh(WaslMuharrik wasl)
        {
            if (wasl is null)
            {
                throw new ArgumentNullException(nameof(wasl));
            }
            return wasl.Ansikh<NuqtaRasm>(Ruus, ruus)
                && wasl.Ansikh<NuqtaMulmas>(Malamis, malamis)
                && wasl.Ansikh<LawnRasm>(Alwan, alwan)
                && wasl.Ansikh<int>(Muthallathat, muthallathat);
        }

        /// <summary>
        /// Converts the patch container's own span records into the ABI's span
        /// struct, which is what <see cref="Nasij"/> and <see cref="Takhtit"/>
        /// both read.
        /// </summary>
        /// <param name="madakhil">The container's spans for one string.</param>
        /// <returns>The spans as the layout and the mesh builder read them.</returns>
        /// <remarks>
        /// The two records genuinely differ: <see cref="MadkhalNitaq"/> is keyed
        /// by string index and <see cref="TaaribNitaqUslub"/> carries a byte
        /// range and the ABI's flag word. The conversion is a loop over a
        /// handful of entries into a pooled array, so it allocates only when a
        /// string carries more spans than any string before it. This is a copy
        /// of the Mono adapter's conversion for one reason and it is worth
        /// naming: the two must produce identical flags, and a shared
        /// implementation would have to live in Mushtarak, which is where it
        /// belongs the moment a third adapter exists.
        /// </remarks>
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

        /// <summary>Releases the four roots. The arrays are then collectable.</summary>
        public void Dispose()
        {
            masfufatRuus?.Dispose();
            masfufatMalamis?.Dispose();
            masfufatAlwan?.Dispose();
            masfufatMuthallathat?.Dispose();
            masfufatRuus = null;
            masfufatMalamis = null;
            masfufatAlwan = null;
            masfufatMuthallathat = null;
            siaatAshkal = 0;
        }

        private static Marja? Ihjiz(WaslMuharrik wasl, IntPtr sanf, int adad)
        {
            IntPtr masfufa = wasl.MasfufaJadida(sanf, adad);
            if (masfufa == IntPtr.Zero)
            {
                return null;
            }
            try
            {
                // Rooted the instant it exists, with nothing allocating in
                // between: an array held across frames with no root is exactly
                // what Maqbad/Marja.cs exists to forbid.
                return Marja.Qawi(masfufa);
            }
            catch (KhataTaarib)
            {
                return null;
            }
        }
    }

    /// <summary>
    /// مصدر الأشكال — where a glyph image comes from, and the engine objects
    /// that put it on the GPU: one texture per atlas page, one material per
    /// page, and the residency pass that brings a runtime glyph in before a
    /// mesh is built against it.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Two atlases, never mixed.</b> A patch carries its own compiled atlas
    /// keyed by glyph identifiers in the patch's own font chain; the runtime
    /// atlas holds whatever the compiler never saw, keyed by identifiers in the
    /// chain this process loaded. Those are not one numbering space: glyph 412
    /// of the patch's chain and glyph 412 of the runtime chain are the same
    /// number naming two different letters whenever the chains differ by so
    /// much as a fallback font. So a precomputed layout is only ever drawn from
    /// the patch atlas and a runtime layout only from the runtime atlas, and
    /// there is no path here that consults one after the other.
    /// </para>
    /// <para>
    /// <b>What is set on the material.</b> <c>_MainTex</c> is the atlas page.
    /// <c>_Color</c> is opaque white, because the real colour is per vertex and
    /// a material tint would multiply over it and flatten every coloured word
    /// in the game. For a distance-field patch two more: <c>_TaaribMisafa</c>,
    /// the spread in texels the rasterizer used, and <c>_TaaribQiyas</c>, the
    /// page's texel size as <c>(1/w, 1/h, w, h)</c>. Nothing else, and in
    /// particular nothing read from the game's own font asset, its shared
    /// material or its SDF atlas.
    /// </para>
    /// <para>
    /// <b>The shader is Taarib's own, and a missing one is a refusal.</b> The
    /// atlas is a single-channel texture. Unity's stock UI shader samples all
    /// four channels, so a page drawn through it renders every glyph as an
    /// opaque red rectangle — visibly worse than the untranslated game. So a
    /// missing shader switches this source off with a sentence rather than
    /// falling back to one that would draw the wrong thing confidently.
    /// </para>
    /// <para>
    /// <b>What is not here that is in the Mono adapter.</b> The Mono source
    /// uploads only the dirty rectangle of a page through a staging texture and
    /// <c>Graphics.CopyTexture</c>. This one re-uploads the whole page when
    /// anything in it changed. That is a bandwidth difference and not a
    /// rendering difference — the texels are identical either way — and it is
    /// taken deliberately, because the partial path needs a second texture, a
    /// sub-rectangle copy and a platform capability query, which is three more
    /// engine members resolved through the ladder for a saving that matters
    /// only while an atlas is still filling.
    /// </para>
    /// </remarks>
    public sealed class MasdarAshkal : IDisposable
    {
        /// <summary>Taarib's coverage shader, by name.</summary>
        public const string IsmSudfatTaghtiya = "Taarib/Taghtiya";

        /// <summary>Taarib's signed-distance-field shader, by name.</summary>
        public const string IsmSudfatMisafa = "Taarib/Misafa";

        private readonly WaslMuharrik wasl;
        private readonly Ruqaa ruqaa;
        private readonly MaqbadSilsila? silsila;
        private readonly Lawha? lawha;
        private readonly ManualLogSource sijill;
        private readonly float misafa;
        private readonly int muarrifLawha;
        private readonly int muarrifMisafa;
        private readonly int muarrifQiyas;

        private Marja? marjaSudfa;
        private Marja?[] lawhatRuqaa;
        private Marja?[] maddatRuqaa;
        private Marja?[] lawhatHayya;
        private Marja?[] maddatHayya;
        private QiyasSafha[] qiyasatHayya;
        private SijillRafa[] talabat = new SijillRafa[8];
        private ulong akhirJeel;
        private int akhirItar;
        private bool amil;

        private MasdarAshkal(
            WaslMuharrik wasl,
            Ruqaa ruqaa,
            MaqbadSilsila? silsila,
            Lawha? lawha,
            ManualLogSource sijill,
            float misafa)
        {
            this.wasl = wasl;
            this.ruqaa = ruqaa;
            this.silsila = silsila;
            this.lawha = lawha;
            this.sijill = sijill;
            this.misafa = misafa;
            muarrifLawha = wasl.MuarrifKhasiya("_MainTex");
            muarrifMisafa = wasl.MuarrifKhasiya("_TaaribMisafa");
            muarrifQiyas = wasl.MuarrifKhasiya("_TaaribQiyas");
            lawhatRuqaa = Array.Empty<Marja?>();
            maddatRuqaa = Array.Empty<Marja?>();
            lawhatHayya = Array.Empty<Marja?>();
            maddatHayya = Array.Empty<Marja?>();
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
        /// <returns>
        /// The drawn size itself for a coverage atlas, which holds one bitmap
        /// per size; the atlas's own canonical size for a distance-field atlas,
        /// which holds one bitmap for every size.
        /// </returns>
        public float HajmLawha(float hajm)
        {
            if (ruqaa.Namat != NamatLawha.Misafa)
            {
                return hajm;
            }
            ReadOnlySpan<TaaribMiftahShakl> mafatih = ruqaa.MafatihAshkal;
            return mafatih.IsEmpty ? hajm : mafatih[0].HajmRubi / 4.0f;
        }

        /// <summary>
        /// Builds the source: checks the vertex layout, finds the shader the
        /// patch's atlas mode requires, and uploads every compiled page.
        /// </summary>
        /// <param name="wasl">The engine binding.</param>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="silsila">
        /// The font chain runtime glyphs are shaped with. <c>null</c> leaves the
        /// patch's compiled atlas working and declines every glyph the compiler
        /// never rasterized.
        /// </param>
        /// <param name="lawha">The runtime atlas and its residency, or <c>null</c>.</param>
        /// <param name="sijill">Where to report a refusal.</param>
        /// <returns>The source, or <c>null</c> when it refused with a logged reason.</returns>
        /// <exception cref="ArgumentNullException">A required argument is null.</exception>
        public static MasdarAshkal? Insha(
            WaslMuharrik wasl,
            Ruqaa ruqaa,
            MaqbadSilsila? silsila,
            Lawha? lawha,
            ManualLogSource sijill)
        {
            if (wasl is null)
            {
                throw new ArgumentNullException(nameof(wasl));
            }
            if (ruqaa is null)
            {
                throw new ArgumentNullException(nameof(ruqaa));
            }
            if (sijill is null)
            {
                throw new ArgumentNullException(nameof(sijill));
            }

            try
            {
                // The one seam the type system cannot express: Mushtarak writes
                // vertices whose layout must match the engine's. Under Mono the
                // check reads Marshal.SizeOf over the real Unity structs; there
                // is no such type to measure here, so the engine's own fixed
                // layouts are asserted against Mushtarak's constants instead.
                // What this catches is Mushtarak changing its vertex layout
                // without this adapter's mirrors changing with it.
                Nasij.TahaqquqTakhtit(
                    Marshal.SizeOf<Muttajih3>(),
                    Marshal.SizeOf<Muttajih2>(),
                    Marshal.SizeOf<LawnRasm>());
            }
            catch (KhataTaarib khata)
            {
                sijill.LogWarning(khata.Arabi + " | " + khata.Injilizi);
                return null;
            }

            bool misafi = ruqaa.Namat == NamatLawha.Misafa;
            if (misafi && !wasl.YadamMisafa)
            {
                sijill.LogWarning(
                    "لا يمكن ضبط وسائط صدفة المسافة في هذا المحرّك؛ أُوقف الاستيلاء بدل رسم لوحة مسافة بوسائط غير مضبوطة. | "
                    + "The distance-field shader parameters cannot be set on this engine "
                    + "build, so the takeover is off rather than drawing a distance-field "
                    + "atlas with unset parameters, which renders every glyph as a "
                    + "rectangle.");
                return null;
            }

            string ism = misafi ? IsmSudfatMisafa : IsmSudfatTaghtiya;
            IntPtr sudfa = wasl.Sudfa(ism);
            if (sudfa == IntPtr.Zero)
            {
                sijill.LogWarning(
                    "لم تُوجد صدفة تعريب (" + ism + ")؛ أُوقف الاستيلاء بدل الرسم بصدفة تقرأ القنوات الأربع من لوحة أحادية القناة. | "
                    + "Taarib's own shader (" + ism + ") was not found; the takeover is off "
                    + "rather than drawing a single-channel atlas through a shader that "
                    + "samples four channels, which would render every glyph as a solid "
                    + "rectangle.");
                return null;
            }

            // The spread a distance-field atlas was rasterized with, in texels.
            // Four is the rasterizer's own default and the only value a patch
            // records implicitly.
            MasdarAshkal masdar = new MasdarAshkal(wasl, ruqaa, silsila, lawha, sijill, 4f);
            try
            {
                masdar.marjaSudfa = Marja.Qawi(sudfa);
            }
            catch (KhataTaarib khata)
            {
                sijill.LogWarning(
                    "تعذّر تثبيت مرجع صدفة تعريب. | Taarib's shader could not be rooted. "
                    + khata.Injilizi);
                return null;
            }

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
        /// <param name="minRuqaa">Whether the layout being drawn came out of the patch.</param>
        /// <returns>The map, as a struct so no interface call is dispatched
        /// virtually inside the vertex loop.</returns>
        public KhareetaMuakkada Khareeta(bool minRuqaa)
        {
            return new KhareetaMuakkada(this, minRuqaa);
        }

        /// <summary>The material one atlas page draws through.</summary>
        /// <param name="minRuqaa">Whether the page belongs to the patch atlas.</param>
        /// <param name="fahras">The page index.</param>
        /// <returns>The material object, or zero.</returns>
        public IntPtr Madda(bool minRuqaa, ushort fahras)
        {
            Marja?[] madat = minRuqaa ? maddatRuqaa : maddatHayya;
            return fahras < madat.Length ? madat[fahras]?.Kaen ?? IntPtr.Zero : IntPtr.Zero;
        }

        /// <summary>The texture one atlas page was uploaded into.</summary>
        /// <param name="minRuqaa">Whether the page belongs to the patch atlas.</param>
        /// <param name="fahras">The page index.</param>
        /// <returns>The texture object, or zero.</returns>
        public IntPtr LawhatSafha(bool minRuqaa, ushort fahras)
        {
            Marja?[] lawhat = minRuqaa ? lawhatRuqaa : lawhatHayya;
            return fahras < lawhat.Length ? lawhat[fahras]?.Kaen ?? IntPtr.Zero : IntPtr.Zero;
        }

        /// <summary>The first material and page that exist, patch atlas first.</summary>
        /// <param name="madda">The material, or zero.</param>
        /// <param name="lawhaSafha">The texture, or zero.</param>
        /// <returns>Whether a material was found.</returns>
        public bool AwwalMadda(out IntPtr madda, out IntPtr lawhaSafha)
        {
            madda = Madda(true, 0);
            lawhaSafha = LawhatSafha(true, 0);
            if (madda == IntPtr.Zero)
            {
                madda = Madda(false, 0);
                lawhaSafha = LawhatSafha(false, 0);
            }
            return madda != IntPtr.Zero;
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
                // per-glyph miss, which Nasij counts and draws nothing for. It
                // is emphatically not a reason to switch the takeover off: the
                // rest of the sentence is still correct.
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
        /// <param name="huruf">The layout's glyphs.</param>
        /// <param name="hajmLawhaHali">The size the atlas rasterizes at.</param>
        /// <param name="tathbit">Whether the pen is snapped, which selects the
        /// subpixel bucket a glyph is rasterized for.</param>
        /// <returns>Whether the atlas is ready to be drawn from.</returns>
        /// <remarks>
        /// Residency is a separate pass from mesh building on purpose:
        /// rasterizing inside the vertex loop would mean a native transition per
        /// glyph and an atlas-full failure thrown halfway through a mesh,
        /// leaving the destination arrays holding half a sentence at full
        /// confidence. <see cref="Lawha.Ibda"/> is called at most once per
        /// engine frame, because that call is what protects every glyph looked
        /// up afterwards from eviction until the next frame; calling it again
        /// mid-frame drops that protection for the text already built, and the
        /// symptom is one glyph in one menu flickering into a different letter.
        /// </remarks>
        public bool Aqim(ReadOnlySpan<TaaribHarf> huruf, float hajmLawhaHali, bool tathbit)
        {
            Lawha? hayya = lawha;
            MaqbadSilsila? chain = silsila;
            if (!amil || hayya is null || chain is null)
            {
                return false;
            }

            try
            {
                int itar = wasl.Itar();
                if (itar != akhirItar)
                {
                    hayya.Ibda();
                    akhirItar = itar;
                }

                ushort hajmRubi = Nasij.HajmRubi(hajmLawhaHali);
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
                sijill.LogWarning(
                    "تعذّر إحضار أشكال هذا النص إلى اللوحة. | Making this string's runtime "
                    + "glyphs resident failed. " + khata.Injilizi);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("making runtime glyphs resident threw", khata);
                return false;
            }
        }

        /// <summary>
        /// Switches the whole source off, naming the reason once. Every takeover
        /// that draws through it then leaves its text in the game's original
        /// language rather than drawing nothing.
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
            sijill.LogError(
                "تعذّر تجهيز لوحة تعريب: " + sabab + "؛ عاد النص إلى لغته الأصلية. | "
                + "Taarib's atlas could not be prepared: " + sabab
                + "; text is left in the game's original language.");
            sijill.LogDebug(khata.ToString());
        }

        /// <summary>
        /// Destroys the textures and materials this source created. Nothing of
        /// the game's is destroyed, because nothing of the game's was ever
        /// created or retained.
        /// </summary>
        public void Dispose()
        {
            amil = false;
            Atlif(lawhatRuqaa, maddatRuqaa);
            Atlif(lawhatHayya, maddatHayya);
            lawhatRuqaa = Array.Empty<Marja?>();
            maddatRuqaa = Array.Empty<Marja?>();
            lawhatHayya = Array.Empty<Marja?>();
            maddatHayya = Array.Empty<Marja?>();
            qiyasatHayya = Array.Empty<QiyasSafha>();
            marjaSudfa?.Dispose();
            marjaSudfa = null;
        }

        private void Atlif(Marja?[] lawhat, Marja?[] madat)
        {
            for (int i = 0; i < madat.Length; i++)
            {
                Marja? wahid = madat[i];
                if (wahid is null)
                {
                    continue;
                }
                wasl.Atlif(wahid.Kaen);
                wahid.Dispose();
                madat[i] = null;
            }
            for (int i = 0; i < lawhat.Length; i++)
            {
                Marja? wahid = lawhat[i];
                if (wahid is null)
                {
                    continue;
                }
                wasl.Atlif(wahid.Kaen);
                wahid.Dispose();
                lawhat[i] = null;
            }
        }

        private bool ArfaSafahatRuqaa()
        {
            ReadOnlySpan<MadkhalSafha> jadwal = ruqaa.Safahat;
            if (jadwal.IsEmpty)
            {
                sijill.LogWarning(
                    "الرقعة المثبَّتة لا تحمل صفحات لوحة؛ لا شيء يمكن رسمه منها. | "
                    + "The installed patch carries no atlas pages; there is nothing in it "
                    + "to draw from.");
                return false;
            }
            lawhatRuqaa = new Marja?[jadwal.Length];
            maddatRuqaa = new Marja?[jadwal.Length];
            for (int i = 0; i < jadwal.Length; i++)
            {
                if (!Arfa(
                    lawhatRuqaa, maddatRuqaa, i, ruqaa.TexelatSafha(i),
                    jadwal[i].Ard, jadwal[i].Irtifa))
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

            // Only the pages that changed. Re-uploading every page whenever one
            // glyph was rasterized would be megabytes of bus traffic for a
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
                if (!Arfa(
                    lawhatHayya, maddatHayya, fahras, hayya.Texelat(fahras),
                    talab.Ard, talab.Irtifa))
                {
                    return false;
                }
                qiyasatHayya[fahras].Ard = (ushort)talab.Ard;
                qiyasatHayya[fahras].Irtifa = (ushort)talab.Irtifa;
                hayya.Rufia(fahras);
            }
            return true;
        }

        private bool Arfa(
            Marja?[] lawhat,
            Marja?[] madat,
            int fahras,
            ReadOnlySpan<byte> texelat,
            int ard,
            int irtifa)
        {
            if (ard <= 0 || irtifa <= 0 || texelat.Length < ard * irtifa)
            {
                sijill.LogWarning(
                    "صفحة لوحة رقم " + fahras + " تعلن أبعادًا لا تطابق عدد بايتاتها؛ رُفض رفعها بدل رسم صفوف مزاحة. | "
                    + "Atlas page " + fahras + " declares dimensions its byte count does not "
                    + "match; the upload is refused rather than drawing rows offset from the "
                    + "rectangles the glyph map names.");
                return false;
            }

            try
            {
                Marja? qadeema = lawhat[fahras];
                IntPtr lawhaKaen = qadeema?.Kaen ?? IntPtr.Zero;
                if (lawhaKaen == IntPtr.Zero)
                {
                    lawhaKaen = wasl.LawhaJadida(ard, irtifa);
                    if (lawhaKaen == IntPtr.Zero)
                    {
                        return false;
                    }
                    qadeema?.Dispose();
                    lawhat[fahras] = Marja.Qawi(lawhaKaen);
                    lawhaKaen = lawhat[fahras]!.Kaen;
                }

                if (!wasl.IrfaTexelat(lawhaKaen, texelat.Slice(0, ard * irtifa)))
                {
                    return false;
                }

                IntPtr maddaKaen = madat[fahras]?.Kaen ?? IntPtr.Zero;
                if (maddaKaen == IntPtr.Zero)
                {
                    IntPtr sudfa = marjaSudfa?.Kaen ?? IntPtr.Zero;
                    maddaKaen = wasl.MaddaJadida(sudfa);
                    if (maddaKaen == IntPtr.Zero)
                    {
                        return false;
                    }
                    madat[fahras]?.Dispose();
                    madat[fahras] = Marja.Qawi(maddaKaen);
                    maddaKaen = madat[fahras]!.Kaen;
                }

                wasl.DaAlLawha(maddaKaen, muarrifLawha, lawhaKaen);
                LawnKamil abyad;
                abyad.Ahmar = 1f;
                abyad.Akhdar = 1f;
                abyad.Azraq = 1f;
                abyad.Shaffafiya = 1f;
                wasl.DaAlLawn(maddaKaen, abyad);
                if (ruqaa.Namat == NamatLawha.Misafa)
                {
                    wasl.DaAlAshari(maddaKaen, muarrifMisafa, misafa);
                    Muttajih4 qiyas;
                    qiyas.S = 1f / ard;
                    qiyas.A = 1f / irtifa;
                    qiyas.Z = ard;
                    qiyas.W = irtifa;
                    wasl.DaAlMuttajih(maddaKaen, muarrifQiyas, qiyas);
                }
                return true;
            }
            catch (KhataTaarib khata)
            {
                Awqif("uploading atlas page " + fahras + " was refused", khata);
                return false;
            }
            catch (Exception khata)
            {
                Awqif("uploading atlas page " + fahras + " threw", khata);
                return false;
            }
        }
    }

    /// <summary>
    /// خريطة مؤكدة — the glyph map <see cref="Nasij"/> is generic over, bound
    /// once to one atlas.
    /// </summary>
    /// <remarks>
    /// A <c>readonly struct</c> rather than a class, because
    /// <see cref="Nasij.Ibni{TKhareeta}"/> constrains its parameter with no
    /// class constraint: a value-type implementation is dispatched statically
    /// and never boxed, which on a screen full of dialogue is the difference
    /// between one interface call per glyph and none.
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
    /// The patch container stores a translation as UTF-8 and the layout call
    /// takes <see cref="char"/>, so one decode into a pooled buffer stands
    /// between them. That decode is the only work on the common path.
    /// </para>
    /// <para>
    /// <b>When Nasq runs, and when it must not.</b> A patch compiled with the
    /// markup bridge already lifted every tag out of the translation and
    /// recorded the spans, so <see cref="Ruqaa.NitaqatNass"/> answering with
    /// anything at all means the work is done and re-parsing would find tags in
    /// a string that has none. Only when the container carries no spans and the
    /// component's own rich-text switch is on is <see cref="Nasq.Hallil"/>
    /// called. Parsing it here rather than letting it reach the shaper is not a
    /// nicety: to the bidirectional algorithm a tag is not a tag, it is a run of
    /// mixed-direction characters, and a colour tag inside an Arabic sentence
    /// comes back out reversed and lodged in the middle of a word.
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

        /// <summary>Prepares one string of the patch for a layout call.</summary>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="fahras">The string's index, from <see cref="Ruqaa.JidNass"/>.</param>
        /// <param name="makhzan">The geometry buffers, whose span-conversion
        /// buffer is reused so the container's own span records need no second
        /// pool here.</param>
        /// <param name="lahja">The markup dialect of the component this string is
        /// drawn by, or <see cref="NawNasq.Bila"/> when its rich-text switch is
        /// off.</param>
        /// <param name="hajmAsas">The component's own size in pixels, which is
        /// what a relative size tag is relative to.</param>
        /// <param name="nass">The text to lay out, in logical order.</param>
        /// <param name="nitaqatKharij">The style spans over its UTF-8 encoding.</param>
        /// <param name="sijill">Where to report a shortfall.</param>
        /// <returns>Whether the string could be prepared at all.</returns>
        public bool Hayyi(
            Ruqaa ruqaa,
            int fahras,
            MakhzanRusum makhzan,
            NawNasq lahja,
            float hajmAsas,
            out ReadOnlySpan<char> nass,
            out ReadOnlySpan<TaaribNitaqUslub> nitaqatKharij,
            ManualLogSource sijill)
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
                // here is this build's bug rather than the string's. Falling
                // back to the unparsed translation draws the tags as text, which
                // is visible and reportable; dropping the string would not be.
                sijill.LogWarning(
                    "Nasq needed more room than Ihsi reported for a translated string; the "
                    + "markup is drawn as literal text for this one string.");
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
    /// مربط — one installed interception, whichever of the two mechanisms
    /// installed it, and the one operation that takes it back off.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Which mechanism, and why it is decided by the rung.</b> HarmonyX can
    /// only weave a method it can name, which under IL2CPP means a method
    /// Il2CppInterop generated a managed facade for — exactly the condition
    /// <see cref="Rutba.Bayanat"/> reports. When that facade exists, HarmonyX is
    /// strictly better: the patch composes with every other plugin's patch on
    /// the same method, nothing is written over the function's prologue, and
    /// removal is a managed operation. When it does not exist — a title whose
    /// metadata the generator could not read, an overload the generator declined
    /// — the address is still real and <see cref="Mihmaz"/> writes a detour over
    /// it. Mihmaz is the fallback rather than the default because byte patching
    /// cannot be shared: two frameworks that both patch one prologue leave a
    /// function neither can restore, which is why Mihmaz refuses to stack.
    /// </para>
    /// <para>
    /// <b>What the managed lookup here is and is not.</b>
    /// <see cref="TabiaMudara"/> does not choose a hook target. The ladder chose
    /// it, and this runs only when the ladder's answer came from rung one, which
    /// means the generated type and the uniquely named method with that arity
    /// both exist and were what rung one read. The lookup repeats rung one's
    /// procedure — the same type name, the same declared-only walk, the same
    /// refusal on ambiguity — solely to recover the managed handle HarmonyX
    /// needs, and any disagreement at all falls back to the native detour rather
    /// than patching something the ladder did not name.
    /// </para>
    /// </remarks>
    public sealed class Mirbat : IDisposable
    {
        private readonly string ism;
        private readonly Harmony? tarqee;
        private readonly MethodBase? hadafMudar;
        private readonly Mihmaz? mihmaz;
        private bool mafkuk;

        private Mirbat(string ism, Harmony? tarqee, MethodBase? hadafMudar, Mihmaz? mihmaz)
        {
            this.ism = ism;
            this.tarqee = tarqee;
            this.hadafMudar = hadafMudar;
            this.mihmaz = mihmaz;
        }

        /// <summary>The target's qualified name, for the log.</summary>
        public string Ism => ism;

        /// <summary>Whether HarmonyX installed this one.</summary>
        public bool BiTarqee => tarqee is not null;

        /// <summary>
        /// The trampoline that runs the original, or zero when HarmonyX
        /// installed the interception and the original is reached by returning
        /// <c>true</c> from the prefix instead.
        /// </summary>
        public IntPtr Muaqqat => mihmaz?.Muaqqat ?? IntPtr.Zero;

        /// <summary>Installs an interception over one resolved target.</summary>
        /// <param name="tabia">The target, as the ladder resolved it.</param>
        /// <param name="hamil">The type declaring the managed patch method.</param>
        /// <param name="ismBadilMudar">The managed patch method's name.</param>
        /// <param name="lahiq">Whether the managed patch is a postfix.</param>
        /// <param name="badilKham">
        /// The native replacement's entry point, for the detour path. Zero means
        /// this target can only be taken managed, and the installation is
        /// refused rather than half-done when HarmonyX cannot reach it.
        /// </param>
        /// <param name="tarqee">The plugin's Harmony instance.</param>
        /// <param name="sijill">Where to report a refusal.</param>
        /// <returns>The interception, or <c>null</c> when neither mechanism could
        /// install it.</returns>
        public static Mirbat? Rakkib(
            in TabiaMahlula tabia,
            Type hamil,
            string ismBadilMudar,
            bool lahiq,
            IntPtr badilKham,
            Harmony tarqee,
            ManualLogSource sijill)
        {
            if (!tabia.Wujid)
            {
                return null;
            }

            if (tabia.MinBayanat && !string.IsNullOrEmpty(ismBadilMudar))
            {
                MethodBase? hadaf = TabiaMudara(in tabia, sijill);
                MethodInfo? badil = hamil.GetMethod(
                    ismBadilMudar, BindingFlags.Public | BindingFlags.Static);
                if (hadaf is not null && badil is not null)
                {
                    try
                    {
                        HarmonyMethod tarqeeBadil = new HarmonyMethod(badil);
                        if (lahiq)
                        {
                            tarqee.Patch(hadaf, postfix: tarqeeBadil);
                        }
                        else
                        {
                            tarqee.Patch(hadaf, prefix: tarqeeBadil);
                        }
                        return new Mirbat(tabia.Ism, tarqee, hadaf, null);
                    }
                    catch (Exception khata)
                    {
                        // A managed patch that would not install is not a reason
                        // to give up on the target: the address is still real
                        // and the detour still reaches it.
                        sijill.LogWarning(
                            $"HarmonyX could not patch {tabia.Ism} ({khata.GetType().Name}: "
                            + $"{khata.Message}); falling back to a native detour.");
                    }
                }
            }

            if (badilKham == IntPtr.Zero)
            {
                sijill.LogWarning(
                    $"لا سبيل مُدار إلى {tabia.Ism} ولا بديل أصليّ له. | {tabia.Ism} has no "
                    + "managed facade and no native replacement, so it cannot be "
                    + "intercepted.");
                return null;
            }

            try
            {
                Mihmaz munazzal = Mihmaz.Rakkib(tabia.Unwan, badilKham);
                return new Mirbat(tabia.Ism, null, null, munazzal);
            }
            catch (KhataTaarib khata)
            {
                sijill.LogWarning(
                    $"{khata.RamzDaim}: {khata.Arabi} | {khata.Injilizi}");
                return null;
            }
        }

        /// <summary>Removes the interception, whichever mechanism installed it.</summary>
        /// <remarks>
        /// Idempotent, and never throws. A native detour that is not removed
        /// outlives the plugin and points at freed code, which is a crash on the
        /// next call; a Harmony patch that is not removed keeps a dead delegate
        /// in somebody else's method.
        /// </remarks>
        public void Dispose()
        {
            if (mafkuk)
            {
                return;
            }
            mafkuk = true;
            try
            {
                if (tarqee is not null && hadafMudar is not null)
                {
                    // By name rather than UnpatchSelf, because the plugin's
                    // Harmony instance is shared with every other text system's
                    // takeover and unpatching all of it would silently switch off
                    // the ones that were working.
                    tarqee.Unpatch(hadafMudar, HarmonyPatchType.All, tarqee.Id);
                }
                mihmaz?.Dispose();
            }
            catch (Exception)
            {
                // Removal runs on shutdown paths, where an exception replaces the
                // failure that is actually worth reporting.
            }
        }

        /// <summary>
        /// Recovers the managed handle for a target the ladder already resolved
        /// on rung one.
        /// </summary>
        /// <param name="tabia">The target.</param>
        /// <param name="sijill">Where to note a disagreement.</param>
        /// <returns>The generated method, or <c>null</c> to use the detour.</returns>
        private static MethodBase? TabiaMudara(in TabiaMahlula tabia, ManualLogSource sijill)
        {
            int nuqta = tabia.Ism.LastIndexOf('.');
            if (nuqta <= 0)
            {
                return null;
            }
            string ismNaw = tabia.Ism.Substring(0, nuqta);
            string ismTabia = tabia.Ism.Substring(nuqta + 1);

            Type? naw = null;
            Assembly[] tajammuat = AppDomain.CurrentDomain.GetAssemblies();
            for (int i = 0; i < tajammuat.Length && naw is null; i++)
            {
                try
                {
                    naw = tajammuat[i].GetType(ismNaw, throwOnError: false, ignoreCase: false);
                }
                catch (Exception)
                {
                    // A generated assembly whose dependencies did not load is a
                    // type this lookup does not have, which is the detour's cue.
                    naw = null;
                }
            }
            if (naw is null)
            {
                return null;
            }

            const BindingFlags alamat =
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance
                | BindingFlags.Static | BindingFlags.DeclaredOnly;
            for (Type? hali = naw; hali is not null; hali = hali.BaseType)
            {
                MethodInfo? wahid = null;
                int adad = 0;
                MethodInfo[] tabaia;
                try
                {
                    tabaia = hali.GetMethods(alamat);
                }
                catch (Exception)
                {
                    return null;
                }
                for (int i = 0; i < tabaia.Length; i++)
                {
                    if (!string.Equals(tabaia[i].Name, ismTabia, StringComparison.Ordinal)
                        || tabaia[i].IsGenericMethodDefinition)
                    {
                        continue;
                    }
                    wahid = tabaia[i];
                    adad++;
                }
                if (adad == 1)
                {
                    return wahid;
                }
                if (adad > 1)
                {
                    // Rung one refuses on ambiguity, so an ambiguity here means
                    // this lookup is not seeing what rung one saw. Patching
                    // either candidate would be patching a method the ladder did
                    // not name.
                    sijill.LogWarning(
                        $"{tabia.Ism} is ambiguous on the generated type; the native detour "
                        + "is used instead of guessing which overload the ladder resolved.");
                    return null;
                }
            }
            return null;
        }
    }

    /// <summary>
    /// وصل تكست ميش برو — every TextMeshPro member this takeover touches,
    /// resolved once at startup through <see cref="Sullam"/>.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Nothing here throws when a member is missing. TextMeshPro has shipped as
    /// an asset-store package, as a built-in package and as three major versions
    /// with renamed members in each, and a build missing one accessor should
    /// cost that one behaviour rather than the whole text system.
    /// </para>
    /// <para>
    /// <b>The patch targets are resolved on the concrete classes.</b>
    /// <c>GenerateTextMesh</c> and <c>UpdateMaterial</c> are virtual on
    /// <c>TMP_Text</c> and overridden by both concrete classes. Under Mono a
    /// patch on the base declaration never runs, because every call site
    /// dispatches to the override; under IL2CPP the failure is worse, because
    /// the base declaration has its own compiled entry point that nothing calls,
    /// so a detour installs over live code, reports success and never fires. So
    /// each concrete class is resolved on its own declaration.
    /// </para>
    /// <para>
    /// <b>The namespace is the native one.</b> Il2CppInterop's generated facade
    /// calls this namespace <c>Il2CppTMPro</c>, but rungs two and three ask the
    /// runtime and the binary, which know it as <c>TMPro</c>. Rung one maps the
    /// native name onto the generated assembly itself.
    /// </para>
    /// </remarks>
    public sealed class WaslTmp
    {
        /// <summary>The IL2CPP assembly TextMeshPro usually ships as.</summary>
        public const string Tajammu = "Unity.TextMeshPro";

        /// <summary>TextMeshPro's native namespace.</summary>
        public const string Fadaa = "TMPro";

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

        private WaslTmp(WaslMuharrik wasl)
        {
            QariNass = wasl.Hall(Tajammu, Fadaa, "TMP_Text", "get_text", 0, string.Empty);
            QariHajm = wasl.Hall(Tajammu, Fadaa, "TMP_Text", "get_fontSize", 0, string.Empty);
            QariLawn = wasl.Hall(Tajammu, Fadaa, "TMP_Text", "get_color", 0, string.Empty);
            QariMuhadhaha = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_alignment", 0, string.Empty);
            QariHamish = wasl.Hall(Tajammu, Fadaa, "TMP_Text", "get_margin", 0, string.Empty);
            QariMustatil = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_rectTransform", 0, string.Empty);
            QariNasij = wasl.Hall(Tajammu, Fadaa, "TMP_Text", "get_mesh", 0, string.Empty);
            QariNasqGhani = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_richText", 0, string.Empty);
            QariLaff = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_enableWordWrapping", 0, string.Empty);
            QariTabaudAhruf = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_characterSpacing", 0, string.Empty);
            QariTabaudKalimat = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_wordSpacing", 0, string.Empty);
            QariHajmTilqai = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_enableAutoSizing", 0, string.Empty);
            QariHajmAdna = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "get_fontSizeMin", 0, string.Empty);
            AmrIttisakh = wasl.Hall(
                Tajammu, Fadaa, "TMP_Text", "SetAllDirty", 0, string.Empty);

            HadafNasijSath = wasl.Hall(
                Tajammu, Fadaa, "TextMeshProUGUI", "GenerateTextMesh", 0,
                "tmp.ugui.generate");
            HadafNasijAalam = wasl.Hall(
                Tajammu, Fadaa, "TextMeshPro", "GenerateTextMesh", 0, "tmp.world.generate");
            HadafMaddaSath = wasl.Hall(
                Tajammu, Fadaa, "TextMeshProUGUI", "UpdateMaterial", 0, "tmp.ugui.material");
            HadafMaddaAalam = wasl.Hall(
                Tajammu, Fadaa, "TextMeshPro", "UpdateMaterial", 0, "tmp.world.material");

            SanfFariSath = wasl.Sanf(Tajammu, Fadaa, "TMP_SubMeshUI");
            SanfFariAalam = wasl.Sanf(Tajammu, Fadaa, "TMP_SubMesh");
        }

        // Readonly fields, not get-only properties. Every takeover point reads
        // these as `in binding.X` so that WaslMuharrik.Qeema takes the binding
        // by reference; C# only allows an argument spelled `in` over a variable,
        // and a property would silently become a struct copy per member per
        // call on the per-frame path.

        /// <summary>Reads the component's raw string, markup and all.</summary>
        public readonly TabiaMahlula QariNass;

        /// <summary>Reads its font size, in the component's own local units.</summary>
        public readonly TabiaMahlula QariHajm;

        /// <summary>Reads its colour, which every glyph inherits unless a span sets one.</summary>
        public readonly TabiaMahlula QariLawn;

        /// <summary>
        /// Reads its alignment as an integer. The property is an enum whose two
        /// halves are packed into one word, and reading it as its underlying
        /// integer is what lets both halves be decoded without naming
        /// TextMeshPro's enums.
        /// </summary>
        public readonly TabiaMahlula QariMuhadhaha;

        /// <summary>Reads its margins as left, top, right, bottom.</summary>
        public readonly TabiaMahlula QariHamish;

        /// <summary>Reads the rectangle it draws into.</summary>
        public readonly TabiaMahlula QariMustatil;

        /// <summary>
        /// Reads the mesh the component already owns. Taarib writes into that
        /// object rather than creating one, so the renderer the component wired
        /// up in Awake keeps pointing at the geometry it draws.
        /// </summary>
        public readonly TabiaMahlula QariNasij;

        /// <summary>Whether the component parses markup in its string.</summary>
        public readonly TabiaMahlula QariNasqGhani;

        /// <summary>Whether the component wraps at the rectangle's width.</summary>
        public readonly TabiaMahlula QariLaff;

        /// <summary>Reads its extra letter spacing, in font units per em.</summary>
        public readonly TabiaMahlula QariTabaudAhruf;

        /// <summary>Reads its extra word spacing, in the same units.</summary>
        public readonly TabiaMahlula QariTabaudKalimat;

        /// <summary>Whether auto-sizing is on for this component.</summary>
        public readonly TabiaMahlula QariHajmTilqai;

        /// <summary>The smallest size auto-sizing may use.</summary>
        public readonly TabiaMahlula QariHajmAdna;

        /// <summary>Marks the component dirty so the canvas rebuilds it.</summary>
        public readonly TabiaMahlula AmrIttisakh;

        /// <summary>The canvas-space mesh generation method, as a hook target.</summary>
        public readonly TabiaMahlula HadafNasijSath;

        /// <summary>The world-space mesh generation method, as a hook target.</summary>
        public readonly TabiaMahlula HadafNasijAalam;

        /// <summary>The canvas-space material assignment method, as a hook target.</summary>
        public readonly TabiaMahlula HadafMaddaSath;

        /// <summary>The world-space material assignment method, as a hook target.</summary>
        public readonly TabiaMahlula HadafMaddaAalam;

        /// <summary>The canvas-space sub-mesh class TextMeshPro splits into.</summary>
        public IntPtr SanfFariSath { get; }

        /// <summary>The world-space sub-mesh class.</summary>
        public IntPtr SanfFariAalam { get; }

        /// <summary>
        /// Whether enough resolved to draw at all: the string, the mesh, the
        /// rectangle, the size, and at least one of the two generation methods.
        /// </summary>
        public bool Muakkad =>
            QariNass.Wujid
            && QariNasij.Wujid
            && QariMustatil.Wujid
            && QariHajm.Wujid
            && (HadafNasijSath.Wujid || HadafNasijAalam.Wujid);

        /// <summary>Binds TextMeshPro, or returns <c>null</c> when it is not usable.</summary>
        /// <param name="wasl">The engine binding, which owns the ladder.</param>
        /// <returns>The binding, or <c>null</c>.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="wasl"/> is null.</exception>
        public static WaslTmp? Iqran(WaslMuharrik wasl)
        {
            if (wasl is null)
            {
                throw new ArgumentNullException(nameof(wasl));
            }
            WaslTmp waslTmp = new WaslTmp(wasl);
            if (!waslTmp.Muakkad)
            {
                wasl.Sijill.LogWarning(
                    "تكست ميش برو موجود ولم تُحلَّ أعضاؤه؛ يبقى نصه بلغة اللعبة وبقية الأنظمة تعمل. | "
                    + "TextMeshPro is present but its drawing members did not resolve; its "
                    + "text is left in the game's original language and every other text "
                    + "system in this game is unaffected.");
                return null;
            }
            if (!waslTmp.QariMuhadhaha.Wujid)
            {
                wasl.Sijill.LogWarning(
                    "TMP_Text.alignment did not resolve in this build; alignment for "
                    + "runtime-laid-out strings comes from the patch's constraint row only, "
                    + "which is right for every string the compiler measured and falls back "
                    + "to the leading edge for the rest.");
            }
            return waslTmp;
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
        /// both directions. Mapping it to an absolute edge and flipping it
        /// somewhere else is how a label ends up correctly aligned in Arabic and
        /// mirrored in the English tooltip beside it.
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

        /// <summary>The vertical alignment half of the packed word.</summary>
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
    /// نظام تكست ميش برو — the takeover itself: the interceptions, the draw,
    /// and the two ways out of it.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The four interception points.</b>
    /// <c>TextMeshProUGUI.GenerateTextMesh</c> and
    /// <c>TextMeshPro.GenerateTextMesh</c> are where each component decides what
    /// to draw, and suppressing them is what stops TMP's own shaper from running
    /// for a string Taarib owns. <c>UpdateMaterial</c> on both is the second
    /// pair and is not optional: on the canvas path it runs <em>after</em> the
    /// geometry in the same rebuild, so a takeover that only intercepted the
    /// geometry would put Taarib's vertices on screen with the game's font atlas
    /// bound, and every glyph would sample a rectangle belonging to a Latin
    /// letter.
    /// </para>
    /// <para>
    /// <b>The re-entrancy guard.</b> A patch that can trigger the thing it
    /// patches is a stack overflow inside somebody's game. <c>SetAllDirty</c> —
    /// which this takeover calls so text already on screen redraws through
    /// Taarib rather than waiting for the game to change it — schedules a canvas
    /// rebuild, and that rebuild calls <c>GenerateTextMesh</c> again. So the
    /// whole draw runs inside <see cref="hars"/>, a thread-static flag: while it
    /// is set every interception hands straight back to TMP and the recursion
    /// terminates at depth one. Thread-static rather than a plain field because
    /// Unity's canvas rebuild can be driven from a job thread in some engine
    /// versions, and a shared flag would then have one text object's draw
    /// suppressing another's.
    /// </para>
    /// <para>
    /// <b>TMP_TextInfo is deliberately not written</b>, exactly as under Mono.
    /// <c>characterInfo</c> and <c>meshInfo</c> are inputs to a pipeline that no
    /// longer runs here, and writing them would cost a resolved member per field
    /// for nothing. What that costs is honest and worth naming: game code that
    /// reads <c>textInfo.characterCount</c> sees TMP's last answer for the
    /// untranslated string.
    /// </para>
    /// <para>
    /// <b>What runs on the native side.</b> The four replacements below are
    /// <c>[UnmanagedCallersOnly]</c> statics whose signatures are the targets'
    /// managed signatures plus IL2CPP's trailing <c>MethodInfo*</c>. They are
    /// reached only when <see cref="Mirbat"/> installed a detour rather than a
    /// managed patch; each catches everything, because an exception crossing a
    /// native frame boundary terminates the process rather than being logged.
    /// </para>
    /// </remarks>
    public sealed class NizamTmp : INizamIl2cpp
    {
        [ThreadStatic]
        private static bool hars;

        private static NizamTmp? hali;
        private static IntPtr muaqqatNasijSath;
        private static IntPtr muaqqatNasijAalam;
        private static IntPtr muaqqatMaddaSath;
        private static IntPtr muaqqatMaddaAalam;

        private readonly List<Mirbat> marabit = new List<Mirbat>(4);
        private readonly HashSet<int> mamlukat = new HashSet<int>();
        private readonly Dictionary<int, MaddaAsliya> maddatAsliya =
            new Dictionary<int, MaddaAsliya>();

        private MasdarMushtarak? mushtarak;
        private MawaridIl2cpp? mawarid;
        private WaslMuharrik? wasl;
        private WaslTmp? waslTmp;
        private MasdarAshkal? masdar;
        private MakhzanRusum? makhzan;
        private NassMuhaddar? muhaddar;
        private byte[] nassUtf8 = new byte[512];
        private bool amil;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        /// <inheritdoc/>
        public string Ism => "TextMeshPro";

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <inheritdoc/>
        /// <remarks>
        /// Probes through the ladder rather than through a type lookup, which is
        /// what makes the answer correct on a game whose metadata cannot be
        /// read: a system that is present but invisible to reflection is still
        /// present. The two targets probed are the two the takeover cannot
        /// proceed without, and <see cref="Sullam"/> caches its answers, so this
        /// costs nothing when <see cref="Rakkib"/> asks for them again.
        /// </remarks>
        public bool Mawjud(Sullam sullam)
        {
            if (sullam is null)
            {
                return false;
            }
            HadafHall sath = new HadafHall(
                WaslTmp.Tajammu, WaslTmp.Fadaa, "TextMeshProUGUI", "GenerateTextMesh", 0,
                "tmp.ugui.generate");
            HadafHall aalam = new HadafHall(
                WaslTmp.Tajammu, WaslTmp.Fadaa, "TextMeshPro", "GenerateTextMesh", 0,
                "tmp.world.generate");
            return sullam.Hall(in sath).Wujid || sullam.Hall(in aalam).Wujid;
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
                    "TAARIB-E-6710",
                    "رُكّب نظام تكست ميش برو مرتين في العملية نفسها؛ هذا خلل في الملحق.",
                    "The TextMeshPro takeover was installed twice in one process; this is a "
                    + "bug in the plugin.",
                    Khutwa.IblaghLilMalik);
            }

            MasdarMushtarak? holder = MasdarMushtarak.Ihjiz(mawaridJadida);
            if (holder is null)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6702",
                    "تعذّر تجهيز ربط المحرّك أو لوحة الأشكال؛ يبقى نص تكست ميش برو بلغة اللعبة.",
                    "The engine binding or the glyph source could not be prepared; "
                    + "TextMeshPro text is left in the game's original language.",
                    Khutwa.FathTashkhis);
            }

            mushtarak = holder;
            mawarid = mawaridJadida;
            wasl = holder.Wasl;
            masdar = holder.Masdar;

            WaslTmp? binding = WaslTmp.Iqran(holder.Wasl);
            if (binding is null)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6703",
                    "تكست ميش برو موجود ولم تُحلَّ أعضاء الرسم فيه.",
                    "TextMeshPro is present but its drawing members did not resolve.",
                    Khutwa.FathTashkhis);
            }

            waslTmp = binding;
            makhzan = new MakhzanRusum();
            muhaddar = new NassMuhaddar();
            amil = true;
            hali = this;

            bool shayun = false;
            shayun |= Rabbit(
                in binding.HadafNasijSath, typeof(TarqeeNasijSath),
                nameof(TarqeeNasijSath.Sabiq), false,
                (IntPtr)(void*)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)
                    &BadilNasijSath,
                tarqee, ref muaqqatNasijSath);
            shayun |= Rabbit(
                in binding.HadafNasijAalam, typeof(TarqeeNasijAalam),
                nameof(TarqeeNasijAalam.Sabiq), false,
                (IntPtr)(void*)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)
                    &BadilNasijAalam,
                tarqee, ref muaqqatNasijAalam);
            Rabbit(
                in binding.HadafMaddaSath, typeof(TarqeeMaddaSath),
                nameof(TarqeeMaddaSath.Sabiq), false,
                (IntPtr)(void*)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)
                    &BadilMaddaSath,
                tarqee, ref muaqqatMaddaSath);
            Rabbit(
                in binding.HadafMaddaAalam, typeof(TarqeeMaddaAalam),
                nameof(TarqeeMaddaAalam.Sabiq), false,
                (IntPtr)(void*)(delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)
                    &BadilMaddaAalam,
                tarqee, ref muaqqatMaddaAalam);

            if (!shayun)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6701",
                    "لم يُركَّب أي اعتراض على توليد نسيج تكست ميش برو؛ يبقى نصه بلغة اللعبة.",
                    "No interception could be installed on TextMeshPro's mesh generation; "
                    + "its text stays in the game's original language and every other text "
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
            muaqqatNasijSath = IntPtr.Zero;
            muaqqatNasijAalam = IntPtr.Zero;
            muaqqatMaddaSath = IntPtr.Zero;
            muaqqatMaddaAalam = IntPtr.Zero;

            WaslMuharrik? engine = wasl;
            foreach (KeyValuePair<int, MaddaAsliya> zawj in maddatAsliya)
            {
                // A renderer destroyed with its scene reports dead through its
                // weak handle; writing to it would be a write into freed memory,
                // which is the one thing a shutdown path must not do.
                IntPtr mujassam = zawj.Value.Mujassam.Kaen;
                IntPtr madda = zawj.Value.Madda.Kaen;
                if (engine is not null && mujassam != IntPtr.Zero && madda != IntPtr.Zero)
                {
                    engine.DaAlMaddaMushtaraka(mujassam, madda);
                }
                zawj.Value.Mujassam.Dispose();
                zawj.Value.Madda.Dispose();
            }
            maddatAsliya.Clear();
            mamlukat.Clear();

            makhzan?.Dispose();
            makhzan = null;
            muhaddar = null;
            waslTmp = null;
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
        /// The whole interception: read the string, look it up, and either draw
        /// it or hand the component straight back to TextMeshPro.
        /// </summary>
        /// <param name="kaen">The text component, as an IL2CPP object pointer.</param>
        /// <param name="sathi">Whether this is the canvas-space component.</param>
        /// <returns>
        /// Whether Taarib drew. <c>false</c> means the component is untouched and
        /// TextMeshPro's own generation must run, which is the correct answer for
        /// every string this patch does not cover.
        /// </returns>
        /// <remarks>
        /// The pointer is used entirely within this call and is never stored,
        /// which is the one shape Maqbad/Marja.cs names as safe for a raw IL2CPP
        /// pointer. Everything that outlives the call — the original material of
        /// a world-space renderer, the atlas pages, the mesh arrays — is held
        /// through a handle instead.
        /// </remarks>
        public bool Yarsum(IntPtr kaen, bool sathi)
        {
            MasdarAshkal? source = masdar;
            if (!amil || hars || kaen == IntPtr.Zero || source is null || !source.Amil)
            {
                return false;
            }
            hars = true;
            try
            {
                return Rassim(kaen, sathi);
            }
            catch (KhataTaarib khata)
            {
                // Attributable to this one string: a layout that failed on a
                // player name is not a reason to stop translating the menu.
                mawarid?.Sijill.LogWarning(
                    "تعذّر رسم نص واحد في تكست ميش برو. | Drawing one TextMeshPro string "
                    + "failed. " + khata.Injilizi);
                Utruk(kaen);
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
        /// which is what the material interceptions do in place of
        /// TextMeshPro's own assignment.
        /// </summary>
        /// <param name="kaen">The text component.</param>
        /// <param name="sathi">Whether it is the canvas-space component.</param>
        /// <returns>Whether the material was bound.</returns>
        public bool Aabbir(IntPtr kaen, bool sathi)
        {
            WaslMuharrik? engine = wasl;
            MasdarAshkal? source = masdar;
            if (!amil || engine is null || source is null || kaen == IntPtr.Zero)
            {
                return false;
            }
            if (!mamlukat.Contains(engine.Muarrif(kaen)))
            {
                return false;
            }
            if (!source.AwwalMadda(out IntPtr madda, out IntPtr lawhaSafha))
            {
                return false;
            }

            try
            {
                if (sathi)
                {
                    IntPtr rassam = engine.Mukawwin(kaen, engine.SanfRassam);
                    return rassam != IntPtr.Zero
                        && engine.Aabbir(rassam, IntPtr.Zero, madda, lawhaSafha);
                }
                IntPtr mujassam = engine.Mukawwin(kaen, engine.SanfMujassam);
                if (mujassam == IntPtr.Zero)
                {
                    return false;
                }
                SajjilMaddaAsliya(engine, mujassam);
                engine.DaAlMaddaMushtaraka(mujassam, madda);
                return true;
            }
            catch (Exception khata)
            {
                Awqif("binding Taarib's material to a TextMeshPro renderer threw", khata);
                return false;
            }
        }

        private bool Rassim(IntPtr kaen, bool sathi)
        {
            WaslMuharrik engine = wasl!;
            WaslTmp binding = waslTmp!;
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

            // The string is now Taarib's own bytes, and the IL2CPP string it came
            // from is never referred to again. Everything below can allocate.
            ulong miftah = Ruqaa.MiftahMinNass(utf8);

            IntPtr mustatilKaen = WaslMuharrik.Qeema<IntPtr>(in binding.QariMustatil, kaen);
            MustatilMuharrik itar = engine.Mustatil(mustatilKaen);
            float hajm = binding.QariHajm.Wujid
                ? WaslMuharrik.Qeema<float>(in binding.QariHajm, kaen)
                : 0f;
            bool yaltaff = binding.QariLaff.Wujid
                && WaslMuharrik.Qeema<byte>(in binding.QariLaff, kaen) != 0;

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
                // cover — a player name, a number, a string the compiler never
                // saw. Leave it alone: do not lay it out, do not draw it, and do
                // not guess at a translation.
                Utruk(kaen);
                return false;
            }

            IntPtr nasij = WaslMuharrik.Qeema<IntPtr>(in binding.QariNasij, kaen);
            if (nasij == IntPtr.Zero || mustatilKaen == IntPtr.Zero || !(hajm > 0f))
            {
                Utruk(kaen);
                return false;
            }

            Muttajih2 mihwar = engine.Mihwar(mustatilKaen);
            Muttajih4 hamish = binding.QariHamish.Wujid
                ? WaslMuharrik.Qeema<Muttajih4>(in binding.QariHamish, kaen)
                : default;
            float ardMutah = itar.Ard - hamish.S - hamish.Z;
            float irtifaMutah = itar.Irtifa - hamish.A - hamish.W;
            ardMutah = ardMutah > 0f ? ardMutah : 0f;
            irtifaMutah = irtifaMutah > 0f ? irtifaMutah : 0f;

            int muhadhaha = binding.QariMuhadhaha.Wujid
                ? WaslMuharrik.Qeema<int>(in binding.QariMuhadhaha, kaen)
                : WaslTmp.RasiAla;
            LawnKamil lawnKamil = binding.QariLawn.Wujid
                ? WaslMuharrik.Qeema<LawnKamil>(in binding.QariLawn, kaen)
                : Abyad();

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            float hajmFili;
            float irtifaTakhtit;

            // The exact quarter-pixel size, never the nearest, and always through
            // Nasij.HajmRubi. Drawing a layout measured at one size into a box the
            // game sizes at another is how text that fitted in the compiler's
            // measurement overflows on a player's screen — and the compiler's
            // overflow report, which said it fitted, would be wrong.
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
                kaen, fahras, hajm, ardMutah, irtifaMutah, muhadhaha,
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
            // MihwarS and MihwarA, because the pivot is expressed against the
            // full rectangle while Ard and Irtifa here are the box after margins
            // — and the vertical centring term reads Irtifa. Written the other
            // way, a middle-aligned label with a top margin sits half the margin
            // too high, which reads as a font metrics problem.
            hayyiz.Ard = ardMutah;
            hayyiz.Irtifa = irtifaMutah;
            hayyiz.MihwarS = 0f;
            hayyiz.MihwarA = 1f;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;
            hayyiz.IzahaS = (-mihwar.S * itar.Ard) + hamish.S;
            hayyiz.IzahaA = ((1f - mihwar.A) * itar.Irtifa) - hamish.A;
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
            // rasterizer used. A distance-field atlas holds one bitmap for every
            // size, so nothing is snapped and every bucket is zero.
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
                    mawarid!.Sijill.LogWarning(
                        "TextMeshPro: the mesh buffer refused a second time after growing to "
                        + "the size it asked for; this string is left to TextMeshPro.");
                    Utruk(kaen);
                    return false;
                }
            }

            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            buffers.Amsah(in natija);
            if (!buffers.Anfidh(engine))
            {
                Utruk(kaen);
                return false;
            }
            engine.ImsahNasij(nasij);
            engine.AktubNasij(
                nasij, buffers.Ruus, buffers.Malamis, buffers.Alwan, buffers.Muthallathat,
                Hudud(in natija));

            NazzifFuruu(engine, kaen, sathi);
            if (!Wassil(engine, source, kaen, nasij, sathi))
            {
                Utruk(kaen);
                return false;
            }

            mamlukat.Add(engine.Muarrif(kaen));
            return true;
        }

        /// <summary>
        /// Lays a string out at run time, for a size the compiler never produced
        /// a layout at.
        /// </summary>
        /// <remarks>
        /// The options come from the patch's constraint row rather than from the
        /// component, and that is the whole point of the row existing: the
        /// compiler measured this slot and recorded the direction, the
        /// justification mode, the diacritics policy and the digits policy the
        /// translator chose for it. Taking them from the component instead would
        /// lay this string out with defaults and make it visibly disagree with
        /// the precomputed text beside it about all four. Only when there is no
        /// row at all does the component's own alignment stand in.
        /// </remarks>
        private bool Khattit(
            IntPtr kaen,
            int fahras,
            float hajm,
            float ardMutah,
            float irtifaMutah,
            int muhadhaha,
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
            WaslTmp binding = waslTmp!;
            Takhtit buffer = shared.Takhtit;

            bool nasqGhani = binding.QariNasqGhani.Wujid
                && WaslMuharrik.Qeema<byte>(in binding.QariNasqGhani, kaen) != 0;
            NawNasq lahja = nasqGhani ? NawNasq.TextMeshPro : NawNasq.Bila;
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
                if (qayd.HajmTilqai && qayd.Hajm > 0f)
                {
                    hajmFili = qayd.Hajm;
                }
            }
            else
            {
                khiyarat = default;
                khiyarat.Muhadhaha = WaslTmp.MuhadhahaMin(muhadhaha);
                if (binding.QariLaff.Wujid
                    && WaslMuharrik.Qeema<byte>(in binding.QariLaff, kaen) == 0)
                {
                    khiyarat.Alam |= Alamat.KhiyarSatrWahid;
                }

                // TextMeshPro states both spacings in font units of one
                // hundredth of an em, which is a ratio and converts exactly; its
                // lineSpacing is an addition to a line height derived from its
                // own font asset, which does not, so it is left alone and the
                // font's own metrics decide. Inventing a pixel line height from
                // a number that means "a bit more than whatever that font said"
                // would space Arabic lines differently from the Latin lines
                // beside them, which is worse than not honouring it.
                if (binding.QariTabaudAhruf.Wujid)
                {
                    khiyarat.TabaudAhruf =
                        WaslMuharrik.Qeema<float>(in binding.QariTabaudAhruf, kaen)
                        * hajm * 0.01f;
                }
                if (binding.QariTabaudKalimat.Wujid)
                {
                    khiyarat.TabaudKalimat =
                        WaslMuharrik.Qeema<float>(in binding.QariTabaudKalimat, kaen)
                        * hajm * 0.01f;
                }

                if (binding.QariHajmTilqai.Wujid && binding.QariHajmAdna.Wujid
                    && WaslMuharrik.Qeema<byte>(in binding.QariHajmTilqai, kaen) != 0)
                {
                    // Shrink-to-fit over Taarib's own measured widths rather than
                    // over TextMeshPro's binary search, which measures the
                    // unshaped string and would stop at a size the shaped text
                    // does not fit in.
                    khiyarat.Tajawuz = SiyasatTajawuz.Taqlis;
                    khiyarat.HajmAdna = WaslMuharrik.Qeema<float>(in binding.QariHajmAdna, kaen);
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
            buffer.Khattit(shared.Siyaq, shared.Silsila, in talab);

            huruf = buffer.Huruf;
            sutur = buffer.Sutur;
            nitaqat = nitaqatNass;
            hajmFili = buffer.Hajm;
            irtifaTakhtit = buffer.Irtifa;
            return true;
        }

        /// <summary>
        /// Copies the component's string out of the IL2CPP heap, growing the
        /// buffer once when a string is longer than any before it.
        /// </summary>
        /// <remarks>
        /// This is the whole of rule (d) at this takeover point. The pointer
        /// arrived as a return value in this same call, nothing has allocated
        /// since, and <see cref="Nasakh"/>'s copy allocates nothing either — so
        /// there is no moment at which the collector could reclaim or relocate
        /// the string between reading its character pointer and finishing with
        /// it. The
        /// short-buffer answer reports the required capacity rather than
        /// throwing, so the retry is one growth and not a probe loop.
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

        private bool Wassil(
            WaslMuharrik engine, MasdarAshkal source, IntPtr kaen, IntPtr nasij, bool sathi)
        {
            if (!source.AwwalMadda(out IntPtr madda, out IntPtr lawhaSafha))
            {
                return false;
            }

            if (sathi)
            {
                IntPtr rassam = engine.Mukawwin(kaen, engine.SanfRassam);
                return rassam != IntPtr.Zero && engine.Aabbir(rassam, nasij, madda, lawhaSafha);
            }

            IntPtr murashshih = engine.Mukawwin(kaen, engine.SanfMurashshih);
            IntPtr mujassam = engine.Mukawwin(kaen, engine.SanfMujassam);
            if (murashshih == IntPtr.Zero || mujassam == IntPtr.Zero)
            {
                return false;
            }
            engine.DaAlNasijMushtarak(murashshih, nasij);
            SajjilMaddaAsliya(engine, mujassam);
            engine.DaAlMaddaMushtaraka(mujassam, madda);
            return true;
        }

        /// <summary>
        /// Clears the sub-mesh children TextMeshPro created for the string it
        /// drew before Taarib took over.
        /// </summary>
        /// <remarks>
        /// TMP splits its geometry by material: a second font asset, a sprite
        /// asset or a fallback each get a sub-mesh child with its own renderer.
        /// Taarib emits one atlas page through one material, so it needs no
        /// split — but the children TMP created for the untranslated string are
        /// still in the scene with their meshes populated, and a renderer nobody
        /// cleared goes on drawing. The symptom is the original English sitting
        /// behind the Arabic, faintly, in exactly the menus that had a fallback
        /// font. Only a child carrying TMP's own sub-mesh component is touched,
        /// so an ordinary image parented under a label is never cleared.
        /// </remarks>
        private void NazzifFuruu(WaslMuharrik engine, IntPtr kaen, bool sathi)
        {
            WaslTmp binding = waslTmp!;
            IntPtr sanfFari = sathi ? binding.SanfFariSath : binding.SanfFariAalam;
            if (sanfFari == IntPtr.Zero)
            {
                return;
            }
            IntPtr jidhr = engine.Tahwil(kaen);
            if (jidhr == IntPtr.Zero)
            {
                return;
            }
            int adad = engine.AdadAtfal(jidhr);
            for (int i = 0; i < adad; i++)
            {
                IntPtr tifl = engine.Tifl(jidhr, i);
                if (tifl == IntPtr.Zero || engine.Mukawwin(tifl, sanfFari) == IntPtr.Zero)
                {
                    continue;
                }
                if (sathi)
                {
                    IntPtr rassam = engine.Mukawwin(tifl, engine.SanfRassam);
                    if (rassam != IntPtr.Zero)
                    {
                        engine.ImsahRassam(rassam);
                    }
                    continue;
                }
                IntPtr murashshih = engine.Mukawwin(tifl, engine.SanfMurashshih);
                IntPtr fari = murashshih == IntPtr.Zero
                    ? IntPtr.Zero
                    : engine.NasijMushtarak(murashshih);
                if (fari != IntPtr.Zero)
                {
                    // TMP's own transient render data, which TMP regenerates the
                    // moment its generation runs again. Nothing serialized is
                    // altered.
                    engine.ImsahNasij(fari);
                }
            }
        }

        /// <summary>
        /// Remembers a world-space renderer's own material the first time Taarib
        /// replaces it, so <see cref="Fukk"/> can give it back.
        /// </summary>
        /// <remarks>
        /// The renderer is held weakly and the material strongly. Weakly for the
        /// renderer because a strong root would keep every text object the game
        /// ever created resident for the life of the process, and a renderer
        /// destroyed with its scene has nothing to restore. Strongly for the
        /// material because Taarib has just taken away the only reference the
        /// scene held to it, and a restore that finds a collected material is
        /// not a restore.
        /// </remarks>
        private void SajjilMaddaAsliya(WaslMuharrik engine, IntPtr mujassam)
        {
            int muarrif = engine.Muarrif(mujassam);
            if (muarrif == 0 || maddatAsliya.ContainsKey(muarrif))
            {
                return;
            }
            IntPtr asliya = engine.MaddaMushtaraka(mujassam);
            if (asliya == IntPtr.Zero)
            {
                return;
            }
            try
            {
                maddatAsliya[muarrif] = new MaddaAsliya(
                    Marja.Daeef(mujassam), Marja.Qawi(asliya));
            }
            catch (KhataTaarib khata)
            {
                mawarid?.Sijill.LogDebug(
                    "recording a TextMeshPro renderer's original material failed: "
                    + khata.Injilizi);
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
            talab.Nizam = NizamNass.TextMeshPro;
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

        private static HududMuharrik Hudud(in NatijaNasij natija)
        {
            HududMuharrik hudud = default;
            hudud.Markaz.S = natija.Hudud.Yasar + (natija.Hudud.Ard * 0.5f);
            hudud.Markaz.A = natija.Hudud.Asfal + (natija.Hudud.Irtifa * 0.5f);
            hudud.Markaz.Z = 0f;
            hudud.Nisf.S = natija.Hudud.Ard * 0.5f;
            hudud.Nisf.A = natija.Hudud.Irtifa * 0.5f;
            hudud.Nisf.Z = 0f;
            return hudud;
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
        /// ABI's and it is not Unity's, which is exactly why this conversion
        /// lives in one named place rather than being open coded with the shifts
        /// written from memory.
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
            mawarid?.Sijill.LogWarning(
                "تعذّر تحضير نص لم يُخطّطه المُصرِّف عند هذا الحجم؛ يبقى بلغة اللعبة. | "
                + "A string the compiler did not lay out at the size it is being drawn at "
                + "could not be prepared, so it is left in the game's own language rather "
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
                "أُوقف الاستيلاء على تكست ميش برو: " + sabab + "؛ عادت نصوصه إلى لغة اللعبة وبقية الأنظمة تعمل. | "
                + "The TextMeshPro takeover switched itself off: " + sabab
                + "; its text is back in the game's own language and every other text system "
                + "keeps working.");
            mawarid?.Sijill.LogDebug(khata.ToString());
        }

        private readonly struct MaddaAsliya
        {
            public MaddaAsliya(Marja mujassam, Marja madda)
            {
                Mujassam = mujassam;
                Madda = madda;
            }

            public Marja Mujassam { get; }

            public Marja Madda { get; }
        }

        // -------------------------------------------------------------------
        // The native replacements. Signature: the target's managed signature,
        // the implicit `this` in front, and IL2CPP's trailing MethodInfo* at the
        // end. Nothing may escape these: an exception crossing back into native
        // code terminates the game rather than being logged.
        // -------------------------------------------------------------------

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        private static unsafe void BadilNasijSath(IntPtr kaen, IntPtr tabia)
        {
            try
            {
                NizamTmp? nizam = hali;
                if (nizam is not null && nizam.Yarsum(kaen, sathi: true))
                {
                    return;
                }
            }
            catch (Exception)
            {
                // Yarsum already reports and disables itself; anything reaching
                // here is beyond reporting and must still let TMP draw.
            }
            IntPtr muaqqat = muaqqatNasijSath;
            if (muaqqat != IntPtr.Zero)
            {
                // SOUND: the trampoline holds the target's relocated prologue
                // followed by a jump back into the untouched rest of it, so it
                // expects exactly the register state the function was entered
                // with — the same arguments, including the trailing MethodInfo*.
                ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)muaqqat)(kaen, tabia);
            }
        }

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        private static unsafe void BadilNasijAalam(IntPtr kaen, IntPtr tabia)
        {
            try
            {
                NizamTmp? nizam = hali;
                if (nizam is not null && nizam.Yarsum(kaen, sathi: false))
                {
                    return;
                }
            }
            catch (Exception)
            {
            }
            IntPtr muaqqat = muaqqatNasijAalam;
            if (muaqqat != IntPtr.Zero)
            {
                // SOUND: as BadilNasijSath.
                ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)muaqqat)(kaen, tabia);
            }
        }

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        private static unsafe void BadilMaddaSath(IntPtr kaen, IntPtr tabia)
        {
            try
            {
                NizamTmp? nizam = hali;
                if (nizam is not null && nizam.Aabbir(kaen, sathi: true))
                {
                    return;
                }
            }
            catch (Exception)
            {
            }
            IntPtr muaqqat = muaqqatMaddaSath;
            if (muaqqat != IntPtr.Zero)
            {
                // SOUND: as BadilNasijSath.
                ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)muaqqat)(kaen, tabia);
            }
        }

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        private static unsafe void BadilMaddaAalam(IntPtr kaen, IntPtr tabia)
        {
            try
            {
                NizamTmp? nizam = hali;
                if (nizam is not null && nizam.Aabbir(kaen, sathi: false))
                {
                    return;
                }
            }
            catch (Exception)
            {
            }
            IntPtr muaqqat = muaqqatMaddaAalam;
            if (muaqqat != IntPtr.Zero)
            {
                // SOUND: as BadilNasijSath.
                ((delegate* unmanaged[Cdecl]<IntPtr, IntPtr, void>)muaqqat)(kaen, tabia);
            }
        }

        /// <summary>
        /// The live takeover the managed patch methods reach, or <c>null</c> when
        /// TextMeshPro was never taken over in this process.
        /// </summary>
        /// <remarks>
        /// A static because a Harmony patch method is static and receives only
        /// what Harmony injects. One takeover exists per process by construction
        /// — <see cref="Rakkib"/> refuses a second — so this cannot become the
        /// wrong one.
        /// </remarks>
        internal static NizamTmp? Hali => hali;
    }

    /// <summary>
    /// ترقيع نسيج السطح — the managed prefix on
    /// <c>TMPro.TextMeshProUGUI.GenerateTextMesh</c>, used when rung one
    /// resolved it and HarmonyX can weave the generated method.
    /// </summary>
    public static class TarqeeNasijSath
    {
        /// <summary>Draws the component through Taarib when the patch covers it.</summary>
        /// <param name="__instance">
        /// The component, injected by Harmony as the Il2CppInterop proxy whose
        /// <c>Pointer</c> is the native object this adapter works in terms of.
        /// </param>
        /// <returns>
        /// <c>false</c> to skip TextMeshPro's own generation entirely, which is
        /// what stops a pipeline with no Arabic OpenType layout in it from
        /// shaping the string; <c>true</c> to let it run untouched.
        /// </returns>
        public static bool Sabiq(Il2CppObjectBase __instance)
        {
            NizamTmp? nizam = NizamTmp.Hali;
            return nizam is null || !nizam.Yarsum(__instance.Pointer, sathi: true);
        }
    }

    /// <summary>
    /// ترقيع نسيج العالم — the managed prefix on
    /// <c>TMPro.TextMeshPro.GenerateTextMesh</c>, the world-space component.
    /// </summary>
    public static class TarqeeNasijAalam
    {
        /// <summary>Draws the component through Taarib when the patch covers it.</summary>
        /// <param name="__instance">The component, injected by Harmony.</param>
        /// <returns><c>false</c> to skip TextMeshPro's own generation.</returns>
        public static bool Sabiq(Il2CppObjectBase __instance)
        {
            NizamTmp? nizam = NizamTmp.Hali;
            return nizam is null || !nizam.Yarsum(__instance.Pointer, sathi: false);
        }
    }

    /// <summary>
    /// ترقيع مادة السطح — the managed prefix on
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
        public static bool Sabiq(Il2CppObjectBase __instance)
        {
            NizamTmp? nizam = NizamTmp.Hali;
            return nizam is null || !nizam.Aabbir(__instance.Pointer, sathi: true);
        }
    }

    /// <summary>
    /// ترقيع مادة العالم — the managed prefix on
    /// <c>TMPro.TextMeshPro.UpdateMaterial</c>.
    /// </summary>
    public static class TarqeeMaddaAalam
    {
        /// <summary>Binds Taarib's material instead of the font asset's.</summary>
        /// <param name="__instance">The component, injected by Harmony.</param>
        /// <returns><c>false</c> when Taarib owns this component's draw.</returns>
        public static bool Sabiq(Il2CppObjectBase __instance)
        {
            NizamTmp? nizam = NizamTmp.Hali;
            return nizam is null || !nizam.Aabbir(__instance.Pointer, sathi: false);
        }
    }
}
