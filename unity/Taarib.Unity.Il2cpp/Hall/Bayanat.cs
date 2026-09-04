// بيانات — rung one: Il2CppInterop's generated assemblies.
//
// When global-metadata.dat is present and of a version Il2CppInterop
// understands, BepInEx has already run the generator and written a managed
// facade for every IL2CPP type into BepInEx/interop/. TMPro.TMP_Text is then a
// real System.Type in a real managed assembly, its methods are real MethodInfo
// objects, and — this is the part that matters — the generator stamped into
// each generated method a static IntPtr field holding the native MethodInfo*
// the metadata said that method compiled to. So the address does not have to be
// searched for. It was written down at generation time by a tool that could
// read the metadata, and this rung's whole job is to look it up.
//
// That is why rung one is exact and why it is tried first. Rung three matches
// bytes and can match the wrong ones; rung two asks the runtime and gets the
// right answer for the class it was asked about, which is only the right answer
// if the class name was right. Rung one is the only rung where the pointer and
// the name came out of the same file.
//
// WHY EVERYTHING HERE IS REFLECTIVE. Two reasons, and they are different.
//
//   Il2CppClassPointerStore<T>.NativeClassPtr is the documented route from a
//   generated managed type to its native Il2CppClass*. It is generic, and T is
//   the generated type — which this code only ever has as a System.Type
//   discovered at runtime from a string in a HadafHall. There is no C# syntax
//   for "instantiate this open generic over a Type value"; MakeGenericType plus
//   a field read is the language's answer, and it is reflection by necessity
//   rather than by taste.
//
//   Il2CppInteropUtils and the store type itself are reached through
//   Type.GetType rather than a compile-time reference so that a process where
//   Il2CppInterop.Runtime is absent, or where these members were renamed
//   between interop releases, declines with a sentence instead of throwing a
//   TypeLoadException while the plugin's own type is being loaded. A rung that
//   cannot be constructed cannot report why it is unavailable, and "unavailable
//   because Il2CppInterop.Runtime does not export Il2CppInteropUtils in this
//   build" is a bug report; a MissingMethodException out of a static
//   constructor is not.
//
// Everything reflected is resolved exactly once, in the constructor, and held
// in readonly fields. The per-target path performs one dictionary lookup per
// generated type and no member lookup that has been done before. Resolution
// runs once per target during plugin initialisation and never on a frame path,
// but a resolver that reflects per call would still be a resolver whose cost
// grows with the target list for no reason.

using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;

namespace Taarib.Unity.Il2cpp.Hall
{
    /// <summary>
    /// Rung one of the ladder: resolves a target through the managed facade
    /// Il2CppInterop generated from <c>global-metadata.dat</c>, which is the
    /// only rung whose address and whose name came from the same file.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Construct once at plugin initialisation and hand to
    /// <see cref="Sullam"/>. Construction resolves every reflected member it
    /// will ever need and decides, once, whether the rung can run at all.
    /// </para>
    /// <para>
    /// Not thread-safe. Every target is resolved on one thread before any
    /// patch is installed, and the type caches it fills are plain dictionaries
    /// on that assumption.
    /// </para>
    /// </remarks>
    public sealed class Bayanat : IRutbatHall
    {
        /// <summary>
        /// The open generic whose static member carries a generated type's
        /// native <c>Il2CppClass*</c>. Assembly-qualified because
        /// <see cref="Type.GetType(string)"/> without a qualifier searches only
        /// this assembly and mscorlib.
        /// </summary>
        private const string IsmMakhzanSanf =
            "Il2CppInterop.Runtime.Il2CppClassPointerStore`1, Il2CppInterop.Runtime";

        /// <summary>
        /// The interop helper that maps a generated <see cref="MethodBase"/>
        /// back to the static field holding its native <c>MethodInfo*</c>.
        /// </summary>
        private const string IsmAdawat =
            "Il2CppInterop.Runtime.Il2CppInteropUtils, Il2CppInterop.Runtime";

        /// <summary>BepInEx's owner of the generated-assembly directory.</summary>
        private const string IsmMudirInterop =
            "BepInEx.Unity.IL2CPP.Il2CppInteropManager, BepInEx.Unity.IL2CPP";

        private const string IsmUdwSanf = "NativeClassPtr";
        private const string IsmTabiaHaql =
            "GetIl2CppMethodInfoPointerFieldForGeneratedMethod";
        private const string IsmMasarInterop = "IL2CPPInteropAssemblyPath";

        private const BindingFlags SakinAam =
            BindingFlags.Public | BindingFlags.Static;

        private const BindingFlags KulMuarrafFaqat =
            BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance
            | BindingFlags.Static | BindingFlags.DeclaredOnly;

        /// <summary>
        /// The open <c>Il2CppClassPointerStore&lt;&gt;</c>, or null when the
        /// interop runtime is not in this process.
        /// </summary>
        private readonly Type? makhzanSanf;

        /// <summary>
        /// <c>GetIl2CppMethodInfoPointerFieldForGeneratedMethod</c>, bound once.
        /// </summary>
        private readonly MethodInfo? jalibHaqlTabia;

        /// <summary>Resolved once in the constructor and never recomputed.</summary>
        private readonly bool mutah;

        /// <summary>The sentence <see cref="Mutah"/> reports, when it refuses.</summary>
        private readonly string sababAdamTawafur;

        /// <summary>
        /// Generated types by their qualified name, including the misses.
        /// </summary>
        /// <remarks>
        /// A null value is a remembered miss. Without it, a build whose target
        /// list names a type this game does not have would rescan every loaded
        /// assembly once per target that mentions it, and a game with two
        /// hundred loaded assemblies makes that measurable even at load time.
        /// </remarks>
        private readonly Dictionary<string, Type?> anwa =
            new Dictionary<string, Type?>(StringComparer.Ordinal);

        /// <summary>
        /// The native <c>Il2CppClass*</c> per generated type, so
        /// <c>MakeGenericType</c> and the member read happen once per type
        /// rather than once per target.
        /// </summary>
        private readonly Dictionary<Type, IntPtr> asnaf =
            new Dictionary<Type, IntPtr>();

        /// <summary>
        /// Binds every reflected member this rung uses and decides whether it
        /// can run in this process.
        /// </summary>
        /// <remarks>
        /// Never throws. A failure to bind is the rung being unavailable, which
        /// is a reportable state, not an error — the ladder is built even in a
        /// process where no rung above three can work.
        /// </remarks>
        public Bayanat()
        {
            makhzanSanf = NawBiIsm(IsmMakhzanSanf);
            Type? adawat = NawBiIsm(IsmAdawat);
            jalibHaqlTabia = adawat?.GetMethod(
                IsmTabiaHaql, SakinAam, null, new[] { typeof(MethodBase) }, null);

            mutah = QarrirTawafur(out sababAdamTawafur);
        }

        /// <inheritdoc/>
        public Rutba Rutba => Rutba.Bayanat;

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

            // The contract forbids throwing out of this method: a rung that
            // throws stops the ladder before rungs two and three have been
            // tried, turning a target this process could have found by pattern
            // into a target it cannot find at all. Every failure below is a
            // false return with a sentence, and the catch is the backstop for
            // the reflection surface, which can throw for reasons this code
            // cannot enumerate in advance (a generated assembly whose
            // dependencies did not load, a TypeLoadException surfaced from
            // GetTypes on an assembly that was fine to load and not fine to
            // enumerate).
            try
            {
                if (!mutah)
                {
                    sabab = sababAdamTawafur;
                    return false;
                }

                Type? naw = JalibNaw(in hadaf);
                if (naw is null)
                {
                    sabab = $"no generated type named {IsmKamil(in hadaf)} is loaded "
                        + $"(looked in assembly {hadaf.Tajammu} and every loaded assembly)";
                    return false;
                }

                if (!JalibSanf(naw, out IntPtr sanf, out string sababSanf))
                {
                    sabab = sababSanf;
                    return false;
                }

                MethodInfo? tabia = JalibTabia(naw, in hadaf, out string sababTabia);
                if (tabia is null)
                {
                    sabab = sababTabia;
                    return false;
                }

                if (jalibHaqlTabia is null)
                {
                    sabab = "Il2CppInterop.Runtime exposes no "
                        + IsmTabiaHaql + ", so a generated method cannot be mapped "
                        + "back to its native MethodInfo";
                    return false;
                }

                object? khamHaql;
                try
                {
                    khamHaql = jalibHaqlTabia.Invoke(null, new object?[] { tabia });
                }
                catch (TargetInvocationException khata)
                {
                    Exception dakhili = khata.InnerException ?? khata;
                    sabab = $"Il2CppInterop refused to map {hadaf.Ism} to a native "
                        + $"MethodInfo ({dakhili.GetType().Name}: {dakhili.Message})";
                    return false;
                }

                if (khamHaql is not FieldInfo haql)
                {
                    // The generator emits the pointer field only for methods it
                    // actually bound. A method present on the proxy type with no
                    // field behind it is one the generator declined — an
                    // explicit interface implementation, a generic definition, a
                    // signature it could not express — which is precisely the
                    // case rung two exists to answer.
                    sabab = $"{hadaf.Ism} exists on the generated type but carries no "
                        + "native MethodInfo field, so the generator declined to bind it";
                    return false;
                }

                object? qeema = haql.GetValue(null);
                if (qeema is not IntPtr muashirTabia || muashirTabia == IntPtr.Zero)
                {
                    sabab = $"the native MethodInfo field for {hadaf.Ism} is null, which "
                        + "means the generated assembly was written for a class this "
                        + "runtime did not load";
                    return false;
                }

                if (!Waqt.UnwanTabia(muashirTabia, out IntPtr natija, out string sababUnwan))
                {
                    sabab = sababUnwan;
                    return false;
                }

                // The class pointer is not needed to produce the address; it is
                // read to prove the managed type really is a generated IL2CPP
                // proxy and not an unrelated managed type that happens to share
                // a full name. A game shipping its own TMPro.TMP_Text shim in a
                // genuinely managed assembly would otherwise resolve here to a
                // managed method that has no native entry point at all.
                if (sanf == IntPtr.Zero)
                {
                    sabab = $"{IsmKamil(in hadaf)} resolved to a managed type with no "
                        + "native class behind it, so it is not an IL2CPP proxy";
                    return false;
                }

                unwan = natija;
                return true;
            }
            catch (Exception khata)
            {
                sabab = $"resolving {hadaf.Ism} through the generated assemblies threw "
                    + $"{khata.GetType().Name}: {khata.Message}";
                return false;
            }
        }

        // -------------------------------------------------------------------
        // Availability, decided once
        // -------------------------------------------------------------------

        /// <summary>
        /// Whether this rung can run at all, answered once at construction.
        /// </summary>
        /// <param name="sabab">The sentence, when it cannot.</param>
        /// <returns>Whether to try this rung.</returns>
        /// <remarks>
        /// The interesting case is the third one. A title whose metadata is
        /// encrypted, packed, or of a version the generator does not know
        /// produces an empty interop directory: every generated type is absent,
        /// so every target would fail here with its own near-identical "no
        /// generated type named X" line. Saying it once, structurally, is what
        /// the contract's Mutah is for, and it is the difference between a log
        /// that names a cause and a log that lists forty symptoms.
        /// </remarks>
        private bool QarrirTawafur(out string sabab)
        {
            if (makhzanSanf is null)
            {
                sabab = "Il2CppInterop.Runtime is not loaded in this process, so no "
                    + "generated assemblies exist to look in";
                return false;
            }

            if (jalibHaqlTabia is null)
            {
                sabab = "Il2CppInterop.Runtime is loaded but exposes no "
                    + IsmTabiaHaql + "(MethodBase), so this build cannot read a "
                    + "generated method's native MethodInfo";
                return false;
            }

            string? masar = MasarInterop();
            if (masar is not null && masar.Length != 0)
            {
                bool mawjud;
                try
                {
                    mawjud = Directory.Exists(masar)
                        && Directory.EnumerateFiles(masar, "*.dll").GetEnumerator().MoveNext();
                }
                catch (IOException)
                {
                    mawjud = true;
                }
                catch (UnauthorizedAccessException)
                {
                    mawjud = true;
                }

                if (!mawjud)
                {
                    sabab = $"no assemblies were generated into \"{masar}\", which means "
                        + "this game's global-metadata.dat could not be read at all — "
                        + "encrypted, packed, or of an unsupported version";
                    return false;
                }
            }

            sabab = string.Empty;
            return true;
        }

        /// <summary>
        /// BepInEx's generated-assembly directory, or null when it cannot be
        /// asked.
        /// </summary>
        /// <returns>The directory, or null.</returns>
        /// <remarks>
        /// Reflective and forgiving on purpose: a host that does not expose this
        /// property is a host this rung still works under, so the answer to "I
        /// could not ask" is to stay available rather than to refuse. The
        /// property is read once, at construction, and its value is not cached
        /// beyond that because it is not consulted again.
        /// </remarks>
        private static string? MasarInterop()
        {
            Type? mudir = NawBiIsm(IsmMudirInterop);
            PropertyInfo? khasiya = mudir?.GetProperty(
                IsmMasarInterop,
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static);
            if (khasiya is null || !khasiya.CanRead)
            {
                return null;
            }

            try
            {
                return khasiya.GetValue(null) as string;
            }
            catch (TargetInvocationException)
            {
                return null;
            }
            catch (MethodAccessException)
            {
                return null;
            }
        }

        // -------------------------------------------------------------------
        // Type lookup
        // -------------------------------------------------------------------

        /// <summary>The qualified name a generated type is registered under.</summary>
        /// <param name="hadaf">The target.</param>
        /// <returns>Namespace and name, or the bare name for the global namespace.</returns>
        private static string IsmKamil(in HadafHall hadaf)
        {
            return hadaf.Fadaa.Length == 0 ? hadaf.Sanf : hadaf.Fadaa + "." + hadaf.Sanf;
        }

        /// <summary>
        /// Finds the generated managed type for a target, remembering both hits
        /// and misses.
        /// </summary>
        /// <param name="hadaf">The target.</param>
        /// <returns>The type, or null.</returns>
        /// <remarks>
        /// The named assembly is tried first, and under two names: BepInEx emits
        /// most assemblies under their original name and prefixes the ones whose
        /// names would collide with a framework assembly (mscorlib becomes
        /// Il2Cppmscorlib, System becomes Il2CppSystem). Only when both miss
        /// does this fall back to scanning every loaded assembly, because a game
        /// that has moved a type between assemblies between versions should
        /// still resolve rather than fail on a name the patch was written
        /// against.
        /// </remarks>
        private Type? JalibNaw(in HadafHall hadaf)
        {
            string ism = IsmKamil(in hadaf);
            if (anwa.TryGetValue(ism, out Type? mahfuz))
            {
                return mahfuz;
            }

            Type? naw = null;
            Assembly[] tajammuat = AppDomain.CurrentDomain.GetAssemblies();

            if (hadaf.Tajammu.Length != 0)
            {
                naw = MinTajammuBiIsm(tajammuat, hadaf.Tajammu, ism)
                    ?? MinTajammuBiIsm(tajammuat, "Il2Cpp" + hadaf.Tajammu, ism);
            }

            if (naw is null)
            {
                for (int i = 0; i < tajammuat.Length && naw is null; i++)
                {
                    naw = NawMinTajammu(tajammuat[i], ism);
                }
            }

            anwa[ism] = naw;
            return naw;
        }

        /// <summary>Looks a type up in the one loaded assembly with a given name.</summary>
        private static Type? MinTajammuBiIsm(Assembly[] tajammuat, string ismTajammu, string ism)
        {
            for (int i = 0; i < tajammuat.Length; i++)
            {
                string? ismuh = tajammuat[i].GetName().Name;
                if (ismuh is null
                    || !string.Equals(ismuh, ismTajammu, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }
                Type? naw = NawMinTajammu(tajammuat[i], ism);
                if (naw is not null)
                {
                    return naw;
                }
            }
            return null;
        }

        /// <summary>
        /// One assembly's answer for a type name, with the exceptions a game's
        /// own assemblies can raise absorbed.
        /// </summary>
        /// <remarks>
        /// Assembly.GetType can throw for a type whose dependencies did not
        /// load, and in an IL2CPP process there are always such assemblies: the
        /// generator emits proxies for the whole metadata, including modules the
        /// game never ships. Treating that as "not here" is correct — a type
        /// that cannot be loaded is a type this rung cannot use.
        /// </remarks>
        private static Type? NawMinTajammu(Assembly tajammu, string ism)
        {
            try
            {
                return tajammu.GetType(ism, throwOnError: false, ignoreCase: false);
            }
            catch (FileNotFoundException)
            {
                return null;
            }
            catch (FileLoadException)
            {
                return null;
            }
            catch (BadImageFormatException)
            {
                return null;
            }
            catch (TypeLoadException)
            {
                return null;
            }
        }

        /// <summary>Resolves a type name, tolerating an absent assembly.</summary>
        private static Type? NawBiIsm(string ism)
        {
            try
            {
                return Type.GetType(ism, throwOnError: false, ignoreCase: false);
            }
            catch (FileNotFoundException)
            {
                return null;
            }
            catch (FileLoadException)
            {
                return null;
            }
            catch (BadImageFormatException)
            {
                return null;
            }
            catch (TypeLoadException)
            {
                return null;
            }
        }

        // -------------------------------------------------------------------
        // The native class pointer
        // -------------------------------------------------------------------

        /// <summary>
        /// Reads <c>Il2CppClassPointerStore&lt;T&gt;.NativeClassPtr</c> for a
        /// generated type, once per type.
        /// </summary>
        /// <param name="naw">The generated managed type.</param>
        /// <param name="sanf">Its native <c>Il2CppClass*</c>.</param>
        /// <param name="sabab">Why not, when it could not be read.</param>
        /// <returns>Whether the pointer was read.</returns>
        /// <remarks>
        /// <para>
        /// The store is generic over the generated type, and the generated type
        /// arrives here as a <see cref="Type"/> value discovered from a string.
        /// C# cannot instantiate an open generic over a runtime Type — there is
        /// no syntax for it and no non-reflective API that offers one — so
        /// MakeGenericType plus a static member read is the only route. That is
        /// the whole reason this rung reflects at all.
        /// </para>
        /// <para>
        /// Both a field and a property are accepted because Il2CppInterop has
        /// shipped NativeClassPtr as each across releases, and a rung that
        /// refuses one of the two would be a rung that stops working on an
        /// interop update for no reason a user could act on.
        /// </para>
        /// </remarks>
        private bool JalibSanf(Type naw, out IntPtr sanf, out string sabab)
        {
            if (asnaf.TryGetValue(naw, out sanf))
            {
                sabab = string.Empty;
                return true;
            }

            sanf = IntPtr.Zero;
            if (makhzanSanf is null)
            {
                sabab = "Il2CppInterop.Runtime is not loaded in this process";
                return false;
            }

            Type mabni;
            try
            {
                mabni = makhzanSanf.MakeGenericType(naw);
            }
            catch (ArgumentException khata)
            {
                // A generated type with a generic constraint the store cannot
                // satisfy, or a by-ref-like type. Rung two answers those.
                sabab = $"{naw.FullName} cannot instantiate Il2CppClassPointerStore "
                    + $"({khata.Message})";
                return false;
            }

            object? qeema = null;
            FieldInfo? haql = mabni.GetField(IsmUdwSanf, SakinAam);
            if (haql is not null)
            {
                qeema = haql.GetValue(null);
            }
            else
            {
                PropertyInfo? khasiya = mabni.GetProperty(IsmUdwSanf, SakinAam);
                if (khasiya is null || !khasiya.CanRead)
                {
                    sabab = "Il2CppClassPointerStore exposes NativeClassPtr as neither a "
                        + "public static field nor a readable public static property in "
                        + "this Il2CppInterop build";
                    return false;
                }
                try
                {
                    qeema = khasiya.GetValue(null);
                }
                catch (TargetInvocationException khata)
                {
                    Exception dakhili = khata.InnerException ?? khata;
                    sabab = $"reading NativeClassPtr for {naw.FullName} threw "
                        + $"{dakhili.GetType().Name}: {dakhili.Message}";
                    return false;
                }
            }

            if (qeema is not IntPtr muashir)
            {
                sabab = $"NativeClassPtr for {naw.FullName} is not a pointer value";
                return false;
            }

            asnaf[naw] = muashir;
            sanf = muashir;
            sabab = string.Empty;
            return true;
        }

        // -------------------------------------------------------------------
        // Overload disambiguation
        // -------------------------------------------------------------------

        /// <summary>
        /// Finds the one method on a generated type matching the target's name
        /// and parameter count, and refuses when more than one matches.
        /// </summary>
        /// <param name="naw">The generated type.</param>
        /// <param name="hadaf">The target.</param>
        /// <param name="sabab">Why not, when it did not resolve to exactly one.</param>
        /// <returns>The method, or null.</returns>
        /// <remarks>
        /// <para>
        /// THE FAILURE THIS GUARDS AGAINST. TMP_Text.GenerateTextMesh has had
        /// zero-parameter and multi-parameter forms across TextMeshPro versions,
        /// and the engine calls whichever one the game's build declares.
        /// Type.GetMethod(name) on a type with two of them throws; a resolver
        /// that answered it by taking the first match would return a real,
        /// non-null, perfectly valid address for a method the game never calls.
        /// The detour would install, report success, and never fire — and the
        /// symptom is a game that renders its original text with a plugin log
        /// full of green lines. That is worse than not resolving, because
        /// nothing about it looks broken. So the parameter count is required by
        /// HadafHall rather than optional, and an ambiguity after filtering on
        /// it is a refusal, never a choice.
        /// </para>
        /// <para>
        /// The walk is DeclaredOnly up the base chain rather than a flat
        /// GetMethods so that an override and the virtual it overrides are not
        /// both candidates: the most derived declaration wins, which is the one
        /// whose native MethodInfo the generator stamped for this type.
        /// </para>
        /// </remarks>
        private static MethodInfo? JalibTabia(Type naw, in HadafHall hadaf, out string sabab)
        {
            List<MethodInfo> murashahun = new List<MethodInfo>(2);
            for (Type? hali = naw; hali is not null; hali = hali.BaseType)
            {
                MethodInfo[] tabaia;
                try
                {
                    tabaia = hali.GetMethods(KulMuarrafFaqat);
                }
                catch (TypeLoadException)
                {
                    break;
                }
                catch (FileNotFoundException)
                {
                    break;
                }

                for (int i = 0; i < tabaia.Length; i++)
                {
                    MethodInfo wahid = tabaia[i];
                    if (!string.Equals(wahid.Name, hadaf.Tabia, StringComparison.Ordinal))
                    {
                        continue;
                    }
                    if (wahid.GetParameters().Length != hadaf.AdadWasait)
                    {
                        continue;
                    }
                    if (wahid.IsGenericMethodDefinition)
                    {
                        // An uninstantiated generic has no single native entry
                        // point to hand back. Rung two resolves the concrete
                        // instantiation the game actually compiled.
                        continue;
                    }
                    murashahun.Add(wahid);
                }

                if (murashahun.Count != 0)
                {
                    break;
                }
            }

            if (murashahun.Count == 0)
            {
                sabab = $"the generated type {naw.FullName} declares no {hadaf.Tabia} "
                    + $"taking {hadaf.AdadWasait} parameter(s)";
                return null;
            }

            if (murashahun.Count > 1)
            {
                sabab = $"{naw.FullName} declares {murashahun.Count} overloads of "
                    + $"{hadaf.Tabia} taking {hadaf.AdadWasait} parameter(s), and picking "
                    + "one of them would resolve the wrong method silently";
                return null;
            }

            sabab = string.Empty;
            return murashahun[0];
        }
    }
}
