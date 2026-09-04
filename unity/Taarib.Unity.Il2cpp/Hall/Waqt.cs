// وقت — rung two: ask the IL2CPP runtime that is actually loaded.
//
// Rung one asks a facade generated against one version of the engine. This rung
// asks the engine. il2cpp_class_from_name and il2cpp_class_get_method_from_name
// are C exports of the runtime inside GameAssembly itself, they have had the
// same signatures since Unity 5, and they answer from the metadata tables the
// process is really using rather than from a file a tool wrote earlier. That is
// why this rung reaches methods rung one cannot: an explicit interface
// implementation, an overload whose signature the generator declined to
// express, a method on a type the generator skipped because one of its
// parameter types was unrepresentable. None of that troubles the runtime, which
// has to be able to find every method in the image or the game would not run.
//
// WHAT IT STILL CANNOT DO, SAID PLAINLY. This rung reads the same metadata rung
// one's generator read. When global-metadata.dat is encrypted, packed, or has
// had its string tables blanked by a protector — which several shipped titles
// do — il2cpp_class_from_name returns null for every name it is given, and it
// does so without failing: the game runs, because the runtime dispatches through
// token indices and never needs the names. So rungs one and two are not two
// independent chances. They are two readers of one file, and when that file
// cannot be read by name they fail together, for the same reason, on every
// target. That is exactly the situation rung three exists for, and it is why the
// ladder has a third rung at all rather than stopping at two.
//
// THE ONE THING THIS FILE REFUSES TO GUESS. il2cpp_class_get_method_from_name
// hands back a MethodInfo*, and the field inside it holding the compiled entry
// point is at an offset that has moved between Unity versions — the struct
// gained a virtual method pointer, lost a field, reordered around the generic
// context. Reading a wrong offset does not fail: it yields a plausible non-null
// value, which is some other field of the same struct — an invoker thunk, a
// declaring class pointer, a token — and detouring that overwrites live runtime
// data in someone else's game process. So this file never adds a constant to a
// pointer. It asks the runtime's own accessor when the runtime exports one, and
// otherwise asks Il2CppInterop's version-aware struct model, which BepInEx
// initialises from the engine version the game actually reports. If neither is
// present the rung declines with a sentence naming both, which is a bug report;
// a guessed offset is a crash dump.
//
// EXPORTS. Everything this rung needs is already on
// Il2CppInterop.Runtime.IL2CPP, so this file declares no DllImport of its own —
// the plugin has exactly one place where a native entry point is bound by name,
// and adding a second one here would mean two sets of assumptions about which
// module GameAssembly's exports live in on each platform. The single exception
// is il2cpp_method_get_pointer, which is not present in every runtime build and
// is therefore bound reflectively rather than linked: absent, it is a fallback
// this rung skips, not a load failure that takes the whole plugin down.

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using Il2CppInterop.Runtime;

namespace Taarib.Unity.Il2cpp.Hall
{
    /// <summary>
    /// Rung two of the ladder: resolves a target through the IL2CPP runtime's
    /// own exports, which answer from the metadata the process is using rather
    /// than from a facade generated against one engine version.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Construct once at plugin initialisation and hand to
    /// <see cref="Sullam"/> after <see cref="Bayanat"/>. Construction resolves
    /// every reflected member and decides availability once.
    /// </para>
    /// <para>
    /// Not thread-safe, and does not need to be: resolution happens on one
    /// thread before any patch is installed.
    /// </para>
    /// </remarks>
    public sealed class Waqt : IRutbatHall
    {
        /// <summary>
        /// Il2CppInterop's version-aware model of the runtime's own structs.
        /// </summary>
        private const string IsmMudirIsdar =
            "Il2CppInterop.Runtime.Runtime.UnityVersionHandler, Il2CppInterop.Runtime";

        private const string IsmTabiaLaff = "Wrap";
        private const string IsmUdwMuashir = "MethodPointer";
        private const string IsmTabiaMuashir = "il2cpp_method_get_pointer";

        /// <summary>
        /// <c>il2cpp_method_get_pointer(MethodInfo*)</c> when the runtime
        /// exports it, bound once. Preferred over any struct model because it
        /// is the runtime answering about its own layout.
        /// </summary>
        private static readonly MethodInfo? TabiaMuashirWaqt = JalibTabiaMuashirWaqt();

        /// <summary>
        /// <c>UnityVersionHandler.Wrap(Il2CppMethodInfo*)</c>, bound once.
        /// </summary>
        private static readonly MethodInfo? TabiaLaff = JalibTabiaLaff();

        /// <summary>
        /// The pointer type <see cref="TabiaLaff"/> takes, needed to box a raw
        /// pointer for a reflective call. Bound once beside the method.
        /// </summary>
        private static readonly Type? NawMuashirTabia = TabiaLaff?.GetParameters()[0].ParameterType;

        /// <summary>
        /// <c>INativeMethodInfoStruct.MethodPointer</c>, bound once from the
        /// wrapper's declared return type rather than from an instance, so it
        /// is resolved even before the first call.
        /// </summary>
        private static readonly PropertyInfo? UdwMuashir = JalibUdwMuashir();

        private readonly bool mutah;
        private readonly string sababAdamTawafur;

        /// <summary>
        /// Every loaded image by assembly name, lower-cased, extension
        /// stripped. Built once because il2cpp_domain_get_assemblies walks the
        /// domain's assembly list and a target list of forty methods would
        /// otherwise walk it forty times.
        /// </summary>
        /// <remarks>
        /// A list per name, not one image, because two images genuinely can
        /// share a name — a game that ships two builds of TextMeshPro, or a
        /// title whose modding framework loaded a second Assembly-CSharp. The
        /// ambiguity is preserved here so it can be reported at the point where
        /// it would otherwise be resolved by silently taking the first.
        /// </remarks>
        private readonly Dictionary<string, List<IntPtr>> suwar =
            new Dictionary<string, List<IntPtr>>(StringComparer.Ordinal);

        /// <summary>Every image in the domain, in domain order.</summary>
        private readonly List<IntPtr> kulSuwar = new List<IntPtr>();

        /// <summary>
        /// Binds the runtime, enumerates its images once, and decides whether
        /// this rung can run.
        /// </summary>
        /// <remarks>
        /// Never throws. The runtime not being up, or being up and refusing to
        /// enumerate, is this rung being unavailable — a state the ladder
        /// reports and continues past.
        /// </remarks>
        public Waqt()
        {
            mutah = QarrirTawafur(out sababAdamTawafur);
        }

        /// <inheritdoc/>
        public Rutba Rutba => Rutba.Waqt;

        /// <inheritdoc/>
        public bool Mutah(out string sabab)
        {
            sabab = sababAdamTawafur;
            return mutah;
        }

        /// <inheritdoc/>
        public bool Hall(in HadafHall hadaf, out IntPtr unwan, out string sabab)
        {
            unwan = IntPtr.Zero;
            sabab = string.Empty;

            // The contract forbids throwing: a rung that throws takes rung
            // three down with it, and rung three is the only one that can work
            // on a protected title. Every path below returns false with a
            // sentence; the catch covers the native surface, where a runtime in
            // an unexpected state can surface an AccessViolation as a managed
            // exception on some hosts.
            try
            {
                if (!mutah)
                {
                    sabab = sababAdamTawafur;
                    return false;
                }

                if (!JalibSanf(in hadaf, out IntPtr sanf, out string sababSanf))
                {
                    sabab = sababSanf;
                    return false;
                }

                IntPtr tabia = JalibTabia(sanf, in hadaf, out string sababTabia);
                if (tabia == IntPtr.Zero)
                {
                    sabab = sababTabia;
                    return false;
                }

                if (!UnwanTabia(tabia, out IntPtr natija, out string sababUnwan))
                {
                    sabab = sababUnwan;
                    return false;
                }

                unwan = natija;
                return true;
            }
            catch (Exception khata)
            {
                sabab = $"asking the IL2CPP runtime for {hadaf.Ism} threw "
                    + $"{khata.GetType().Name}: {khata.Message}";
                return false;
            }
        }

        /// <summary>
        /// Reads the compiled entry point out of a native <c>MethodInfo*</c>,
        /// without ever assuming where in that struct it lives.
        /// </summary>
        /// <param name="tabia">The native <c>MethodInfo*</c>.</param>
        /// <param name="unwan">Its compiled entry point.</param>
        /// <param name="sabab">Why not, when it could not be read.</param>
        /// <returns>Whether an entry point was produced.</returns>
        /// <remarks>
        /// <para>
        /// Public and static because rung one needs exactly this: the field
        /// Il2CppInterop stamps into a generated method is also a
        /// <c>MethodInfo*</c>, and both rungs must decode it the same way or
        /// the two rungs could disagree about one method's address. The
        /// knowledge of the runtime's struct layout belongs to the rung that
        /// talks to the runtime, so it lives here and rung one calls in.
        /// </para>
        /// <para>
        /// Two routes, in order of authority. The runtime's own
        /// <c>il2cpp_method_get_pointer</c> is preferred when the loaded build
        /// exports it, because a runtime cannot be wrong about its own struct.
        /// Failing that, Il2CppInterop's <c>UnityVersionHandler</c>, which
        /// BepInEx initialises from the engine version the game reports and
        /// which therefore models the right layout rather than a guessed one.
        /// Failing both, a refusal — never an offset. See this file's header
        /// for what a wrong offset costs.
        /// </para>
        /// </remarks>
        public static bool UnwanTabia(IntPtr tabia, out IntPtr unwan, out string sabab)
        {
            unwan = IntPtr.Zero;
            sabab = string.Empty;

            if (tabia == IntPtr.Zero)
            {
                sabab = "the native MethodInfo pointer is null";
                return false;
            }

            if (TabiaMuashirWaqt is not null)
            {
                object? natija;
                try
                {
                    natija = TabiaMuashirWaqt.Invoke(null, new object?[] { tabia });
                }
                catch (TargetInvocationException khata)
                {
                    Exception dakhili = khata.InnerException ?? khata;
                    sabab = $"il2cpp_method_get_pointer threw {dakhili.GetType().Name}: "
                        + dakhili.Message;
                    return false;
                }

                if (natija is IntPtr min && min != IntPtr.Zero)
                {
                    unwan = min;
                    return true;
                }
            }

            MethodInfo? laff = TabiaLaff;
            Type? nawMuashir = NawMuashirTabia;
            PropertyInfo? udw = UdwMuashir;
            if (laff is not null && nawMuashir is not null && udw is not null)
            {
                object? malfuf;
                try
                {
                    object mualam;
                    unsafe
                    {
                        // SOUND: Wrap's parameter is a pointer type, and
                        // reflection can only pass a pointer boxed by
                        // Pointer.Box. The pointer is not dereferenced here —
                        // it is handed straight back to Il2CppInterop, which
                        // owns the struct layout — and the value came from the
                        // runtime's own metadata tables, which outlive the
                        // process's use of them. Nothing is read through it in
                        // this block, so there is nothing here to be wrong
                        // about.
                        mualam = Pointer.Box((void*)tabia, nawMuashir);
                    }
                    malfuf = laff.Invoke(null, new object?[] { mualam });
                }
                catch (TargetInvocationException khata)
                {
                    Exception dakhili = khata.InnerException ?? khata;
                    sabab = $"UnityVersionHandler.Wrap threw {dakhili.GetType().Name}: "
                        + dakhili.Message;
                    return false;
                }
                catch (ArgumentException khata)
                {
                    sabab = "UnityVersionHandler.Wrap would not accept a boxed MethodInfo "
                        + $"pointer ({khata.Message})";
                    return false;
                }

                if (malfuf is null)
                {
                    sabab = "UnityVersionHandler.Wrap produced nothing for this MethodInfo";
                    return false;
                }

                object? qeema;
                try
                {
                    qeema = udw.GetValue(malfuf);
                }
                catch (TargetInvocationException khata)
                {
                    Exception dakhili = khata.InnerException ?? khata;
                    sabab = $"reading MethodPointer threw {dakhili.GetType().Name}: "
                        + dakhili.Message;
                    return false;
                }

                if (qeema is IntPtr minLaff && minLaff != IntPtr.Zero)
                {
                    unwan = minLaff;
                    return true;
                }

                sabab = "the runtime reports this method has no compiled entry point, "
                    + "which is what an abstract or a stripped method looks like";
                return false;
            }

            sabab = "this runtime exports no il2cpp_method_get_pointer and this "
                + "Il2CppInterop build exposes no UnityVersionHandler.Wrap, so the "
                + "compiled entry point cannot be read out of a MethodInfo without "
                + "guessing a field offset, which this build will not do";
            return false;
        }

        // -------------------------------------------------------------------
        // Availability and the image table, built once
        // -------------------------------------------------------------------

        private bool QarrirTawafur(out string sabab)
        {
            if (TabiaMuashirWaqt is null
                && (TabiaLaff is null || NawMuashirTabia is null || UdwMuashir is null))
            {
                sabab = "neither il2cpp_method_get_pointer nor "
                    + "UnityVersionHandler.Wrap(...).MethodPointer is available, so a "
                    + "MethodInfo found through the runtime could not be turned into an "
                    + "entry point";
                return false;
            }

            IntPtr majal;
            try
            {
                majal = IL2CPP.il2cpp_domain_get();
            }
            catch (Exception khata)
            {
                sabab = $"the IL2CPP runtime could not be called ({khata.GetType().Name}: "
                    + $"{khata.Message}), so it is not loaded in this process";
                return false;
            }

            if (majal == IntPtr.Zero)
            {
                sabab = "the IL2CPP runtime reports no domain, so it is not initialised "
                    + "in this process";
                return false;
            }

            if (!IbnSuwar(majal, out string sababSuwar))
            {
                sabab = sababSuwar;
                return false;
            }

            sabab = string.Empty;
            return true;
        }

        /// <summary>
        /// Walks the domain's assemblies once and indexes their images by
        /// assembly name.
        /// </summary>
        private bool IbnSuwar(IntPtr majal, out string sabab)
        {
            uint adad = 0;
            try
            {
                unsafe
                {
                    // SOUND: il2cpp_domain_get_assemblies returns a pointer to
                    // the runtime's own array of Il2CppAssembly*, and writes its
                    // length through the ref parameter. The array belongs to the
                    // runtime and stays valid for the life of the domain, which
                    // outlives this loop by the whole process; nothing is
                    // written through the pointer, and the loop is bounded by
                    // the count the runtime itself reported.
                    IntPtr* tajammuat = IL2CPP.il2cpp_domain_get_assemblies(majal, ref adad);
                    if (tajammuat == null || adad == 0)
                    {
                        sabab = "the IL2CPP domain reports no assemblies, which means the "
                            + "runtime is up but its metadata was not loaded";
                        return false;
                    }

                    for (uint i = 0; i < adad; i++)
                    {
                        IntPtr tajammu = tajammuat[i];
                        if (tajammu == IntPtr.Zero)
                        {
                            continue;
                        }
                        IntPtr sura = IL2CPP.il2cpp_assembly_get_image(tajammu);
                        if (sura == IntPtr.Zero)
                        {
                            continue;
                        }
                        kulSuwar.Add(sura);
                        Sajjil(sura);
                    }
                }
            }
            catch (Exception khata)
            {
                sabab = $"enumerating the IL2CPP domain's assemblies threw "
                    + $"{khata.GetType().Name}: {khata.Message}";
                return false;
            }

            if (kulSuwar.Count == 0)
            {
                sabab = "no IL2CPP assembly in this domain carries an image";
                return false;
            }

            sabab = string.Empty;
            return true;
        }

        /// <summary>Files one image under its assembly name.</summary>
        private void Sajjil(IntPtr sura)
        {
            IntPtr kham = IL2CPP.il2cpp_image_get_name(sura);
            string? ism = kham == IntPtr.Zero ? null : Marshal.PtrToStringAnsi(kham);
            if (string.IsNullOrEmpty(ism))
            {
                return;
            }

            // Images are named with their file extension ("Assembly-CSharp.dll")
            // and HadafHall names assemblies without one, so the extension is
            // stripped here rather than at every call site.
            string mujarrad = ism;
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

        // -------------------------------------------------------------------
        // Class lookup
        // -------------------------------------------------------------------

        /// <summary>
        /// Finds the target's native class, refusing rather than choosing when
        /// two images both define it.
        /// </summary>
        /// <param name="hadaf">The target.</param>
        /// <param name="sanf">The native <c>Il2CppClass*</c>.</param>
        /// <param name="sabab">Why not, when it was not found or was ambiguous.</param>
        /// <returns>Whether exactly one class was found.</returns>
        /// <remarks>
        /// <para>
        /// THE EMPTY NAMESPACE. il2cpp_class_from_name takes the namespace and
        /// the name as two separate C strings, and for a type in the global
        /// namespace the namespace is the empty string — never null. Passing
        /// null there marshals as a null char*, the runtime compares it against
        /// an interned empty string, the comparison fails, and every
        /// global-namespace type in the game resolves to null. NGUI's UILabel
        /// and a great many older Unity assets live in the global namespace, so
        /// getting this wrong does not fail visibly on one target; it fails on
        /// an entire class of games. HadafHall already normalises a null
        /// namespace to an empty string in its constructor, and this method
        /// relies on that rather than restating it.
        /// </para>
        /// <para>
        /// SEVERAL IMAGES WITH ONE NAME. The named-image list is tried first,
        /// in domain order. If exactly one of them defines the class, that is
        /// the answer. If more than one does, the rung refuses and names the
        /// count: two images called Assembly-CSharp that both define the type
        /// means the process loaded two builds of the game's code, and
        /// detouring the one that is not running produces a hook that installs,
        /// reports success and never fires. Only when no named image matches
        /// does the search widen to every image in the domain — a type moved
        /// between assemblies between game versions should still resolve — and
        /// the same refusal-on-ambiguity applies there.
        /// </para>
        /// </remarks>
        private bool JalibSanf(in HadafHall hadaf, out IntPtr sanf, out string sabab)
        {
            sanf = IntPtr.Zero;
            string fadaa = hadaf.Fadaa;
            string ism = hadaf.Sanf;

            if (hadaf.Tajammu.Length != 0
                && suwar.TryGetValue(hadaf.Tajammu.ToLowerInvariant(), out List<IntPtr>? musamma))
            {
                int adad = Ibhath(musamma, fadaa, ism, out IntPtr wahid);
                if (adad == 1)
                {
                    sanf = wahid;
                    sabab = string.Empty;
                    return true;
                }
                if (adad > 1)
                {
                    sabab = $"{adad} images named {hadaf.Tajammu} each define "
                        + $"{IsmSanf(fadaa, ism)}, and detouring the wrong one would "
                        + "install a hook that never fires";
                    return false;
                }
            }

            int shamil = Ibhath(kulSuwar, fadaa, ism, out IntPtr aakhar);
            if (shamil == 1)
            {
                sanf = aakhar;
                sabab = string.Empty;
                return true;
            }
            if (shamil > 1)
            {
                sabab = $"{shamil} loaded images define {IsmSanf(fadaa, ism)}, so the "
                    + "runtime cannot say which one this game runs";
                return false;
            }

            sabab = $"the runtime reports no class {IsmSanf(fadaa, ism)} in any of "
                + $"{kulSuwar.Count} loaded images; if rung 1 also declined, this game's "
                + "metadata cannot be searched by name and only a signature can find it";
            return false;
        }

        /// <summary>
        /// Counts how many of a set of images define a class, keeping the
        /// first.
        /// </summary>
        /// <remarks>
        /// Counting rather than returning on the first hit is the point: the
        /// caller needs to know about a second definition in order to refuse,
        /// and a search that stopped early could not tell it. The set is at most
        /// the domain's image count, walked once per target, which at load time
        /// is nothing.
        /// </remarks>
        private static int Ibhath(List<IntPtr> majmua, string fadaa, string ism, out IntPtr awwal)
        {
            awwal = IntPtr.Zero;
            int adad = 0;
            for (int i = 0; i < majmua.Count; i++)
            {
                IntPtr sanf = IL2CPP.il2cpp_class_from_name(majmua[i], fadaa, ism);
                if (sanf == IntPtr.Zero)
                {
                    continue;
                }
                if (adad == 0)
                {
                    awwal = sanf;
                }
                else if (sanf == awwal)
                {
                    // Two images can share one class when one forwards to the
                    // other; that is one class, not an ambiguity.
                    continue;
                }
                adad++;
            }
            return adad;
        }

        private static string IsmSanf(string fadaa, string ism)
        {
            return fadaa.Length == 0 ? ism : fadaa + "." + ism;
        }

        // -------------------------------------------------------------------
        // Method lookup
        // -------------------------------------------------------------------

        /// <summary>
        /// Finds the target method on a native class, by name and parameter
        /// count.
        /// </summary>
        /// <param name="sanf">The native class.</param>
        /// <param name="hadaf">The target.</param>
        /// <param name="sabab">Why not, when it was not found.</param>
        /// <returns>The native <c>MethodInfo*</c>, or zero.</returns>
        /// <remarks>
        /// il2cpp_class_get_method_from_name takes the parameter count as an
        /// argument, so overload separation is the runtime's job and it does it
        /// correctly. The manual sweep below runs only when that returns null,
        /// and exists for one real case: a method declared on a base class that
        /// some runtime builds do not walk to. The sweep walks the hierarchy
        /// explicitly and refuses on ambiguity rather than taking the first
        /// match, for the same reason rung one does.
        /// </remarks>
        private static IntPtr JalibTabia(IntPtr sanf, in HadafHall hadaf, out string sabab)
        {
            IntPtr mubashir = IL2CPP.il2cpp_class_get_method_from_name(
                sanf, hadaf.Tabia, hadaf.AdadWasait);
            if (mubashir != IntPtr.Zero)
            {
                sabab = string.Empty;
                return mubashir;
            }

            IntPtr wujida = IntPtr.Zero;
            int adad = 0;
            IntPtr hali = sanf;
            for (; hali != IntPtr.Zero; hali = IL2CPP.il2cpp_class_get_parent(hali))
            {
                IntPtr murur = IntPtr.Zero;
                while (true)
                {
                    IntPtr tabia = IL2CPP.il2cpp_class_get_methods(hali, ref murur);
                    if (tabia == IntPtr.Zero)
                    {
                        break;
                    }
                    if (!Yutabiq(tabia, in hadaf))
                    {
                        continue;
                    }
                    if (adad == 0)
                    {
                        wujida = tabia;
                    }
                    adad++;
                }

                if (adad != 0)
                {
                    // Stop at the most derived declaration: an override and the
                    // virtual it overrides are one method with two entries, and
                    // continuing up would report a false ambiguity.
                    break;
                }
            }

            if (adad == 1)
            {
                sabab = string.Empty;
                return wujida;
            }

            if (adad > 1)
            {
                sabab = $"the runtime reports {adad} methods named {hadaf.Tabia} taking "
                    + $"{hadaf.AdadWasait} parameter(s) on {hadaf.Sanf}, and picking one "
                    + "would resolve the wrong overload silently";
                return IntPtr.Zero;
            }

            sabab = $"the runtime reports no method {hadaf.Tabia} taking "
                + $"{hadaf.AdadWasait} parameter(s) on {hadaf.Sanf} or any of its bases";
            return IntPtr.Zero;
        }

        /// <summary>Whether one native method matches the target's name and arity.</summary>
        private static bool Yutabiq(IntPtr tabia, in HadafHall hadaf)
        {
            if (IL2CPP.il2cpp_method_get_param_count(tabia) != (uint)hadaf.AdadWasait)
            {
                return false;
            }
            IntPtr kham = IL2CPP.il2cpp_method_get_name(tabia);
            if (kham == IntPtr.Zero)
            {
                return false;
            }
            string? ism = Marshal.PtrToStringAnsi(kham);
            return ism is not null && string.Equals(ism, hadaf.Tabia, StringComparison.Ordinal);
        }

        // -------------------------------------------------------------------
        // The reflected surface, bound once per process
        // -------------------------------------------------------------------

        /// <summary>
        /// Binds <c>IL2CPP.il2cpp_method_get_pointer</c> when this Il2CppInterop
        /// build exposes it.
        /// </summary>
        /// <remarks>
        /// Reflective rather than linked because the export is absent from older
        /// runtimes and from some Il2CppInterop releases. A direct call would
        /// make this file fail to load on those, taking rung three down with it
        /// for no benefit; bound this way, its absence is one skipped branch.
        /// </remarks>
        private static MethodInfo? JalibTabiaMuashirWaqt()
        {
            try
            {
                return typeof(IL2CPP).GetMethod(
                    IsmTabiaMuashir,
                    BindingFlags.Public | BindingFlags.Static,
                    null,
                    new[] { typeof(IntPtr) },
                    null);
            }
            catch (AmbiguousMatchException)
            {
                return null;
            }
        }

        /// <summary>
        /// Binds <c>UnityVersionHandler.Wrap</c> for the method-info struct: the
        /// one overload taking a single pointer whose return type carries a
        /// MethodPointer member.
        /// </summary>
        /// <remarks>
        /// Wrap is overloaded across every runtime struct Il2CppInterop models —
        /// class, method, field, image, assembly — and they are separated only
        /// by parameter type. Matching on "one pointer parameter whose return
        /// type has MethodPointer" identifies the method-info overload without
        /// this file needing a compile-time reference to Il2CppMethodInfo, whose
        /// namespace has moved between interop releases.
        /// </remarks>
        private static MethodInfo? JalibTabiaLaff()
        {
            Type? mudir;
            try
            {
                mudir = Type.GetType(IsmMudirIsdar, throwOnError: false, ignoreCase: false);
            }
            catch (TypeLoadException)
            {
                return null;
            }
            catch (System.IO.FileNotFoundException)
            {
                return null;
            }

            if (mudir is null)
            {
                return null;
            }

            MethodInfo[] tabaia = mudir.GetMethods(BindingFlags.Public | BindingFlags.Static);
            for (int i = 0; i < tabaia.Length; i++)
            {
                MethodInfo wahid = tabaia[i];
                if (!string.Equals(wahid.Name, IsmTabiaLaff, StringComparison.Ordinal))
                {
                    continue;
                }
                ParameterInfo[] wasait = wahid.GetParameters();
                if (wasait.Length != 1 || !wasait[0].ParameterType.IsPointer)
                {
                    continue;
                }
                if (wahid.ReturnType.GetProperty(IsmUdwMuashir) is null)
                {
                    continue;
                }
                return wahid;
            }
            return null;
        }

        /// <summary>
        /// Binds the <c>MethodPointer</c> member on the wrapper's declared
        /// return type, once, before any call needs it.
        /// </summary>
        private static PropertyInfo? JalibUdwMuashir()
        {
            PropertyInfo? udw = TabiaLaff?.ReturnType.GetProperty(IsmUdwMuashir);
            if (udw is null || !udw.CanRead || udw.PropertyType != typeof(IntPtr))
            {
                return null;
            }
            return udw;
        }
    }
}
