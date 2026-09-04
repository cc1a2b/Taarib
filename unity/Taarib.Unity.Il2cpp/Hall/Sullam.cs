// سُلَّم — the resolution ladder, and the contract every rung answers to.
//
// Under Mono, finding TMP_Text.GenerateTextMesh is one line of reflection. The
// method is real IL in a real assembly with real metadata, and Type.GetMethod
// either returns it or does not.
//
// Under IL2CPP none of that is true. The managed code was compiled ahead of
// time into GameAssembly.dll — a native binary with no IL, no managed method
// table, and no reflection. What remains of the metadata lives in
// global-metadata.dat, a separate file that a shipped title may have encrypted,
// stripped, packed, or written in a version this build has never seen. Several
// commercial protectors do exactly that, and they are not rare.
//
// So resolution is a ladder of three rungs, tried in order, each cheaper and
// more trustworthy than the one below it:
//
//   1. BAYANAT — Il2CppInterop's generated assemblies. When global-metadata.dat
//      is present and its version is supported, the interop layer has already
//      built a managed facade over the native types and the method can be asked
//      for by name. This is the rung that fires for the overwhelming majority
//      of games and it is exact: the pointer comes from the metadata itself.
//
//   2. WAQT — the IL2CPP runtime's own exports. When the metadata is readable
//      but the interop layer cannot bind a particular overload — a generic
//      instantiation, an explicit interface implementation, a method whose
//      signature the generator declined — il2cpp_class_from_name and
//      il2cpp_class_get_method_from_name answer directly. This works across far
//      more engine versions than the generated bindings do, because it asks the
//      runtime that is actually loaded rather than a facade generated against
//      one version of it.
//
//   3. BASMAT — a byte pattern. When the metadata is unreadable, nothing above
//      can work and the only remaining handle on the function is what its
//      compiled prologue looks like. This is the rung that makes protected
//      titles work at all, and it is also the one that can be wrong, so its
//      results are validated before use and every pattern carries a predicate
//      the resolved address has to satisfy.
//
// WHY THE LADDER IS RECORDED AND NOT JUST THE ANSWER. A takeover that works is
// not the same as a takeover that works for the right reason. If a game
// resolves on rung three when its metadata is perfectly readable, something is
// wrong with rung one that will eventually be wrong in a way that matters —
// and nobody would ever find out, because the text renders. So every resolution
// carries the rung that produced it, that rung reaches the log and the
// diagnostics bundle, and Studio can ask a player's install which rung fired
// for which method. A silent fallback is a bug that has learned to hide.
//
// WHAT THIS FILE DOES NOT DO. It resolves addresses. It does not install
// anything: detouring what it found is khatf's job, and a resolver that also
// hooked would be a resolver nobody could run in a diagnostic mode that only
// reports.

using System;
using System.Collections.Generic;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Hall
{
    /// <summary>Which rung of the ladder produced a resolution.</summary>
    /// <remarks>
    /// Ordered by descending trustworthiness, and the numbers are stable because
    /// they cross into diagnostics bundles and into the registry's compatibility
    /// reports, where a value from an older build has to keep meaning what it
    /// meant.
    /// </remarks>
    public enum Rutba
    {
        /// <summary>Nothing resolved it.</summary>
        Bila = 0,

        /// <summary>
        /// Il2CppInterop's generated assemblies. Exact, and what should fire.
        /// </summary>
        Bayanat = 1,

        /// <summary>
        /// The IL2CPP runtime's own exports. Exact, and fires when the generated
        /// bindings could not express the target.
        /// </summary>
        Waqt = 2,

        /// <summary>
        /// A byte pattern from the signature database. The rung that makes
        /// protected titles work, and the only one that can be wrong.
        /// </summary>
        Basmat = 3,
    }

    /// <summary>
    /// One method the adapter needs to find, named the way each rung needs it.
    /// </summary>
    /// <remarks>
    /// One description rather than three, because the three rungs are three ways
    /// of answering one question and a target described differently to each of
    /// them is a target that can be resolved to three different addresses
    /// without anything noticing.
    /// </remarks>
    public readonly struct HadafHall
    {
        /// <summary>Describes a target.</summary>
        /// <param name="tajammu">The IL2CPP assembly name, without extension.</param>
        /// <param name="fadaa">The namespace, or an empty string for the global one.</param>
        /// <param name="sanf">The declaring type's name.</param>
        /// <param name="tabia">The method's name.</param>
        /// <param name="adadWasait">
        /// How many parameters it declares. The runtime's lookup takes this and
        /// it is what separates overloads, so it is required rather than
        /// optional: passing the wrong count resolves the wrong overload
        /// silently.
        /// </param>
        /// <param name="miftahBasma">
        /// The key this target has in the signature database, or an empty string
        /// when no pattern exists for it and rung three is not available.
        /// </param>
        public HadafHall(
            string tajammu,
            string fadaa,
            string sanf,
            string tabia,
            int adadWasait,
            string miftahBasma)
        {
            Tajammu = tajammu ?? string.Empty;
            Fadaa = fadaa ?? string.Empty;
            Sanf = sanf ?? string.Empty;
            Tabia = tabia ?? string.Empty;
            AdadWasait = adadWasait;
            MiftahBasma = miftahBasma ?? string.Empty;
        }

        /// <summary>The IL2CPP assembly name, without extension.</summary>
        public string Tajammu { get; }

        /// <summary>The namespace, or empty for the global namespace.</summary>
        public string Fadaa { get; }

        /// <summary>The declaring type's name.</summary>
        public string Sanf { get; }

        /// <summary>The method's name.</summary>
        public string Tabia { get; }

        /// <summary>How many parameters it declares.</summary>
        public int AdadWasait { get; }

        /// <summary>Its key in the signature database, or empty.</summary>
        public string MiftahBasma { get; }

        /// <summary>The fully qualified name, for a log line and a refusal.</summary>
        public string Ism =>
            Fadaa.Length == 0 ? $"{Sanf}.{Tabia}" : $"{Fadaa}.{Sanf}.{Tabia}";

        /// <summary>Whether this target names enough to be looked up at all.</summary>
        public bool Salih => Sanf.Length != 0 && Tabia.Length != 0;
    }

    /// <summary>What a resolution produced, and how.</summary>
    public readonly struct NatijatHall
    {
        private NatijatHall(IntPtr unwan, Rutba rutba, string sabab)
        {
            Unwan = unwan;
            Rutba = rutba;
            Sabab = sabab ?? string.Empty;
        }

        /// <summary>
        /// The method's native entry point, or <see cref="IntPtr.Zero"/> when
        /// nothing resolved it.
        /// </summary>
        public IntPtr Unwan { get; }

        /// <summary>Which rung produced it.</summary>
        public Rutba Rutba { get; }

        /// <summary>
        /// Why it failed, in one sentence, when it did. Empty on success.
        /// </summary>
        /// <remarks>
        /// The sentence names the rung that got furthest and what stopped it,
        /// because "TMP_Text.GenerateTextMesh could not be found" tells a player
        /// nothing and tells the owner even less. "Rung 1 declined: the
        /// generated assembly has no TMP_Text; rung 2 declined: the runtime
        /// reports no class TMPro.TMP_Text; rung 3 declined: no pattern for
        /// Unity 2022.3 on win-x64" is a bug report that can be acted on.
        /// </remarks>
        public string Sabab { get; }

        /// <summary>Whether anything resolved.</summary>
        public bool Wujid => Unwan != IntPtr.Zero && Rutba != Rutba.Bila;

        /// <summary>Records a successful resolution.</summary>
        /// <param name="unwan">The entry point.</param>
        /// <param name="rutba">The rung that produced it.</param>
        /// <returns>The result.</returns>
        public static NatijatHall Najah(IntPtr unwan, Rutba rutba)
        {
            return new NatijatHall(unwan, rutba, string.Empty);
        }

        /// <summary>Records a failure.</summary>
        /// <param name="sabab">Why, naming the rungs that were tried.</param>
        /// <returns>The result.</returns>
        public static NatijatHall Fashal(string sabab)
        {
            return new NatijatHall(IntPtr.Zero, Rutba.Bila, sabab);
        }
    }

    /// <summary>One rung of the ladder.</summary>
    /// <remarks>
    /// Implemented three times and used only through <see cref="Sullam"/>, so
    /// that the order the rungs are tried in lives in one place and cannot be
    /// varied per call site. A caller that could choose its own order could
    /// choose to prefer a byte pattern over the metadata, which is precisely
    /// the situation the ladder exists to prevent.
    /// </remarks>
    public interface IRutbatHall
    {
        /// <summary>Which rung this is.</summary>
        Rutba Rutba { get; }

        /// <summary>
        /// Whether this rung can run at all in this process.
        /// </summary>
        /// <returns>Whether to try it.</returns>
        /// <remarks>
        /// Asked once per rung, not per target, and it is what keeps the log
        /// honest: a rung that is unavailable for a structural reason — the
        /// metadata could not be read, the signature database was not shipped —
        /// says so once instead of failing identically for every target.
        /// </remarks>
        bool Mutah(out string sabab);

        /// <summary>Tries to resolve one target.</summary>
        /// <param name="hadaf">The target.</param>
        /// <param name="unwan">Its entry point, when this rung found it.</param>
        /// <param name="sabab">Why not, when it did not.</param>
        /// <returns>Whether it resolved.</returns>
        /// <remarks>
        /// Must not throw. A rung that throws stops the ladder before the rungs
        /// below it have been tried, which turns a recoverable miss into a
        /// method that cannot be found at all.
        /// </remarks>
        bool Hall(in HadafHall hadaf, out IntPtr unwan, out string sabab);
    }

    /// <summary>
    /// The ladder: three rungs in order, and a record of which one fired for
    /// every target this process resolved.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Build one at startup, resolve every target through it, and read
    /// <see cref="Sijill"/> when writing the diagnostics bundle. Resolution
    /// happens once per target per process; nothing here is on a frame path.
    /// </para>
    /// <para>
    /// Not thread-safe, and it does not need to be: every target is resolved
    /// during plugin initialisation, on one thread, before any patch is
    /// installed.
    /// </para>
    /// </remarks>
    public sealed class Sullam
    {
        private readonly IRutbatHall[] rutab;
        private readonly Dictionary<string, NatijatHall> sijill =
            new Dictionary<string, NatijatHall>(StringComparer.Ordinal);
        private readonly Dictionary<Rutba, string> mawani =
            new Dictionary<Rutba, string>();
        private readonly int[] adad = new int[4];

        /// <summary>Builds a ladder over the rungs it is given, in order.</summary>
        /// <param name="rutab">
        /// The rungs, most trustworthy first. Ordinarily
        /// <see cref="Rutba.Bayanat"/>, <see cref="Rutba.Waqt"/>,
        /// <see cref="Rutba.Basmat"/>.
        /// </param>
        /// <exception cref="ArgumentNullException"><paramref name="rutab"/> is null.</exception>
        /// <remarks>
        /// The order is the caller's to supply because the plugin root is the
        /// only thing that knows which rungs it managed to construct — a build
        /// with no signature database ships two rungs, not three — but it is
        /// supplied once, at construction, and no call site can vary it
        /// afterwards.
        /// </remarks>
        public Sullam(params IRutbatHall[] rutab)
        {
            this.rutab = rutab ?? throw new ArgumentNullException(nameof(rutab));
            for (int i = 0; i < this.rutab.Length; i++)
            {
                IRutbatHall rutba = this.rutab[i];
                if (rutba is null)
                {
                    continue;
                }
                if (!rutba.Mutah(out string sabab))
                {
                    mawani[rutba.Rutba] = sabab ?? string.Empty;
                }
            }
        }

        /// <summary>Every target resolved so far, by its qualified name.</summary>
        public IReadOnlyDictionary<string, NatijatHall> Sijill => sijill;

        /// <summary>Why a rung is unavailable in this process, by rung.</summary>
        public IReadOnlyDictionary<Rutba, string> Mawani => mawani;

        /// <summary>How many targets each rung resolved.</summary>
        /// <param name="rutba">The rung.</param>
        /// <returns>Its count.</returns>
        public int Adad(Rutba rutba)
        {
            int i = (int)rutba;
            return (uint)i < (uint)adad.Length ? adad[i] : 0;
        }

        /// <summary>
        /// Whether every resolution so far came from a rung that reads metadata
        /// rather than bytes.
        /// </summary>
        /// <remarks>
        /// What the diagnostics screen reports as the healthy state. A game
        /// where this is false is working, and is working for a reason worth
        /// knowing about.
        /// </remarks>
        public bool KulluhaMinBayanat => adad[(int)Rutba.Basmat] == 0;

        /// <summary>
        /// Resolves one target, trying each rung in order and recording which
        /// one produced the answer.
        /// </summary>
        /// <param name="hadaf">The target.</param>
        /// <returns>The result, which may be a failure carrying every rung's reason.</returns>
        /// <remarks>
        /// A target resolved twice returns the first answer rather than
        /// resolving again. Two calls producing two different addresses for one
        /// method — which a byte pattern matching a second time in a module that
        /// has since been relocated could genuinely do — would mean two detours
        /// on what is supposed to be one function.
        /// </remarks>
        public NatijatHall Hall(in HadafHall hadaf)
        {
            string ism = hadaf.Ism;
            if (sijill.TryGetValue(ism, out NatijatHall sabiqa))
            {
                return sabiqa;
            }

            if (!hadaf.Salih)
            {
                NatijatHall batila = NatijatHall.Fashal(
                    "the target names no type or no method, which is this build's bug rather "
                    + "than the game's");
                sijill[ism] = batila;
                return batila;
            }

            string asbab = string.Empty;
            for (int i = 0; i < rutab.Length; i++)
            {
                IRutbatHall rutba = rutab[i];
                if (rutba is null)
                {
                    continue;
                }
                if (mawani.ContainsKey(rutba.Rutba))
                {
                    asbab = Adif(asbab, rutba.Rutba, mawani[rutba.Rutba]);
                    continue;
                }

                bool wujid;
                IntPtr unwan;
                string sabab;
                try
                {
                    wujid = rutba.Hall(in hadaf, out unwan, out sabab);
                }
                catch (Exception khata)
                {
                    // A rung is not allowed to throw, and one that does must not
                    // take the rungs below it down with it: the whole point of
                    // the ladder is that a failure at one level is survivable.
                    asbab = Adif(asbab, rutba.Rutba, "threw: " + khata.Message);
                    continue;
                }

                if (wujid && unwan != IntPtr.Zero)
                {
                    NatijatHall najah = NatijatHall.Najah(unwan, rutba.Rutba);
                    sijill[ism] = najah;
                    int fahras = (int)rutba.Rutba;
                    if ((uint)fahras < (uint)adad.Length)
                    {
                        adad[fahras]++;
                    }
                    return najah;
                }
                asbab = Adif(asbab, rutba.Rutba, sabab);
            }

            NatijatHall fashal = NatijatHall.Fashal(
                asbab.Length == 0 ? "no rung was available" : asbab);
            sijill[ism] = fashal;
            return fashal;
        }

        /// <summary>
        /// Resolves a target and refuses if it cannot, naming every rung's
        /// reason.
        /// </summary>
        /// <param name="hadaf">The target.</param>
        /// <returns>Its entry point.</returns>
        /// <exception cref="KhataTaarib">Nothing resolved it.</exception>
        /// <remarks>
        /// For the targets a takeover cannot proceed without. A target it can
        /// proceed without — an optional overload, a version-specific helper —
        /// goes through <see cref="Hall"/> and is checked.
        /// </remarks>
        public IntPtr Lazim(in HadafHall hadaf)
        {
            NatijatHall natija = Hall(in hadaf);
            if (natija.Wujid)
            {
                return natija.Unwan;
            }
            throw new KhataTaarib(
                Ramz.GhayrMuhayyaa,
                "TAARIB-E-6401",
                $"تعذّر العثور على {hadaf.Ism} في هذه اللعبة. {natija.Sabab}",
                $"{hadaf.Ism} could not be found in this game. {natija.Sabab}",
                Khutwa.FathTashkhis);
        }

        /// <summary>
        /// The resolution record, as lines for the log and the diagnostics
        /// bundle.
        /// </summary>
        /// <returns>One line per target, plus one per unavailable rung.</returns>
        /// <remarks>
        /// Allocates, and is meant to: it runs once, when a bundle is written or
        /// when the diagnostics screen is opened.
        /// </remarks>
        public IReadOnlyList<string> Taqreer()
        {
            List<string> sutur = new List<string>(sijill.Count + mawani.Count + 1);
            foreach (KeyValuePair<Rutba, string> mani in mawani)
            {
                sutur.Add($"rung {(int)mani.Key} ({mani.Key}) unavailable: {mani.Value}");
            }
            foreach (KeyValuePair<string, NatijatHall> madkhal in sijill)
            {
                NatijatHall natija = madkhal.Value;
                sutur.Add(natija.Wujid
                    ? $"{madkhal.Key}: rung {(int)natija.Rutba} ({natija.Rutba}) "
                        + $"at 0x{natija.Unwan.ToInt64():X}"
                    : $"{madkhal.Key}: unresolved. {natija.Sabab}");
            }
            sutur.Add(
                $"resolved: {adad[(int)Rutba.Bayanat]} by metadata, "
                + $"{adad[(int)Rutba.Waqt]} by runtime export, "
                + $"{adad[(int)Rutba.Basmat]} by signature");
            return sutur;
        }

        private static string Adif(string asbab, Rutba rutba, string sabab)
        {
            string wahid = $"rung {(int)rutba} declined: "
                + (string.IsNullOrEmpty(sabab) ? "no reason given" : sabab);
            return asbab.Length == 0 ? wahid : asbab + "; " + wahid;
        }
    }
}
