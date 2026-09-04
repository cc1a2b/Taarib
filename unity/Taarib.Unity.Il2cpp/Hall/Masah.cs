// مسح — rung three: find the function by what its compiled bytes look like.
//
// Rungs one and two both read global-metadata.dat by name, so they fail
// together on a title whose metadata is encrypted, packed, or has had its name
// tables blanked. The game still runs, because IL2CPP dispatches through token
// indices and never needs a name. What is left to find TMP_Text.GenerateTextMesh
// with is the shape of the machine code the compiler emitted for it, and that
// is what this rung matches.
//
// It is also the only rung that can be confidently, silently wrong, so
// everything here is arranged around narrowing where it is allowed to look and
// refusing when it cannot tell.
//
// WHY THE EXECUTABLE SECTION AND NOT THE WHOLE MAPPED IMAGE. Two reasons, and
// the second is the one that matters.
//
//   Speed: GameAssembly.dll for a large title is several hundred megabytes
//   mapped, of which the code section is a fraction. Scanning the rest is time
//   spent on bytes that cannot be the answer, once per target, at startup, on
//   the player's machine.
//
//   Correctness: a byte pattern is a short string of bytes, and short strings
//   of bytes occur in data. String literals, metadata blobs, relocation tables,
//   the import directory and the .rdata constant pool are all full of plausible
//   x86 prologue sequences that are not code. A match there produces an address
//   that is real, non-null, inside the module, and not a function — and rung
//   three's caller does not merely read that address, it detours it, which means
//   writing a jump over live read-only data or over something the process reads
//   every frame. Restricting the search to the section the loader marked
//   executable is what turns "a pattern that happens to occur somewhere" into
//   "a pattern that occurs in code", and it is not an optimisation.
//
// WHY THE HEADERS ARE PARSED RATHER THAN THE SECTION BEING GUESSED. The
// executable range is not the module base plus a constant on any of the three
// platforms, and it is not the first section either: Unity ships binaries with
// multiple executable sections, and protectors add more. The loader already
// decided which ranges are executable and wrote that decision into the headers
// that are mapped at the module base, so this file reads it back rather than
// assuming. On Windows that is the PE section table, on Linux the ELF program
// headers, on macOS the Mach-O load commands.
//
// THE SIGNATURE DATABASE IS NOT IN THIS FILE. It is data, versioned separately
// and shipped in the framework payload, and it lives in Basmat/. This file
// declares the seam it consumes — IMazwadNamat below — and Basmat/Qaida.cs is
// expected to implement that interface over Basmat/Namat.cs and Basmat/Isdar.cs.
// The seam is declared here rather than inferred from the database's own types
// so that neither side has to guess the other's shape: a scanner needs bytes, a
// mask, an offset, a name to report and a predicate, and nothing else about how
// the database decided which patterns apply to this engine version, this text
// system version, this architecture and this platform.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using Taarib.Unity.Il2cpp.Basmat;

namespace Taarib.Unity.Il2cpp.Hall
{
    /// <summary>
    /// One pattern the signature database offered for a target.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Carries a <see cref="Namat"/> rather than raw bytes and a mask, and that
    /// is load-bearing. <c>Namat</c> owns the scan: a Boyer-Moore-Horspool
    /// search over the pattern's longest wildcard-free run, a mandatory
    /// uniqueness check, the offset, and the data-driven validation predicates.
    /// A second scanner here would be a second set of those rules, and the one
    /// exercised less often would be the one that was wrong — which for a
    /// signature scanner means detouring a function that merely looked like the
    /// target.
    /// </para>
    /// <para>
    /// So the division is: this file finds the module and its executable
    /// sections, because that is platform work and needs PE, ELF and Mach-O
    /// headers; <c>Namat</c> searches inside the range it is handed, because
    /// that is pattern work. Neither does the other's job.
    /// </para>
    /// </remarks>
    public readonly struct BasmaMurashaha
    {
        /// <summary>Records one candidate.</summary>
        /// <param name="ism">
        /// Its stable name, for the diagnostics bundle — for example
        /// <c>tmp_generate_text_mesh@2021.3/win-x64 variant 2</c>.
        /// </param>
        /// <param name="namat">The pattern, which owns its own scan.</param>
        /// <param name="wahda">
        /// The module to search by file name, or an empty string for the IL2CPP
        /// module this rung discovered for itself.
        /// </param>
        public BasmaMurashaha(string ism, Namat namat, string wahda)
        {
            Ism = ism ?? string.Empty;
            Namat = namat;
            Wahda = wahda ?? string.Empty;
        }

        /// <summary>Its stable name, for the diagnostics bundle.</summary>
        public string Ism { get; }

        /// <summary>The pattern.</summary>
        public Namat? Namat { get; }

        /// <summary>The module to search, or empty for the IL2CPP module.</summary>
        public string Wahda { get; }

        /// <summary>Whether this candidate names a pattern at all.</summary>
        public bool Salih => Namat is not null;
    }

    /// <summary>
    /// The seam between the scanner and the signature database.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Declared here, implemented in <c>Basmat/Qaida.cs</c> over the pattern and
    /// version types in <c>Basmat/Namat.cs</c> and <c>Basmat/Isdar.cs</c>. The
    /// scanner knows nothing about how candidates were selected — engine version
    /// range, text system version range, architecture, platform — and the
    /// database knows nothing about how a module is found or a section is
    /// parsed. That split is why a new game version is a data change.
    /// </para>
    /// <para>
    /// Implementations must not throw from either member. <see cref="Masah"/>
    /// guards against it anyway, because the contract forbids a rung throwing,
    /// but an implementation that relies on that guard is an implementation
    /// whose failures all read as "the pattern provider threw".
    /// </para>
    /// </remarks>
    public interface IMazwadNamat
    {
        /// <summary>
        /// Whether the database can answer at all in this process.
        /// </summary>
        /// <param name="sabab">
        /// Why not, in one sentence, when it cannot — "no signature database
        /// shipped with this build", "no entry for Unity 2022.3 on win-x64".
        /// </param>
        /// <returns>Whether to ask it for candidates.</returns>
        /// <remarks>
        /// Asked once, at ladder construction, so a build that ships no database
        /// says so once rather than once per target.
        /// </remarks>
        bool Mutah(out string sabab);

        /// <summary>
        /// The candidate patterns for one target, most likely first.
        /// </summary>
        /// <param name="miftahBasma">
        /// The target's key, from <see cref="HadafHall.MiftahBasma"/>. Never
        /// null and never empty when this is called.
        /// </param>
        /// <returns>
        /// The candidates, or an empty list when the database has no entry for
        /// this key under the current engine version, architecture and platform.
        /// Never null.
        /// </returns>
        IReadOnlyList<BasmaMurashaha> Murashahat(string miftahBasma);
    }

    /// <summary>A module found in this process, and its mapped extent.</summary>
    public readonly struct WahdaMasah
    {
        /// <summary>Records a discovered module.</summary>
        /// <param name="ism">Its file name, as the operating system reports it.</param>
        /// <param name="qaida">Its mapped base address.</param>
        /// <param name="hajm">Its mapped size in bytes.</param>
        public WahdaMasah(string ism, IntPtr qaida, long hajm)
        {
            Ism = ism ?? string.Empty;
            Qaida = qaida;
            Hajm = hajm;
        }

        /// <summary>The module's file name.</summary>
        public string Ism { get; }

        /// <summary>Its mapped base address.</summary>
        public IntPtr Qaida { get; }

        /// <summary>Its mapped size in bytes.</summary>
        public long Hajm { get; }

        /// <summary>Whether anything was found.</summary>
        public bool Wujid => Qaida != IntPtr.Zero && Hajm > 0;
    }

    /// <summary>One executable range inside a module.</summary>
    public readonly struct Qitaa
    {
        /// <summary>Records one executable range.</summary>
        /// <param name="ism">The section or segment name from the headers.</param>
        /// <param name="unwan">Its first byte, as mapped in this process.</param>
        /// <param name="hajm">Its length in bytes.</param>
        public Qitaa(string ism, IntPtr unwan, long hajm)
        {
            Ism = ism ?? string.Empty;
            Unwan = unwan;
            Hajm = hajm;
        }

        /// <summary>The section or segment name.</summary>
        public string Ism { get; }

        /// <summary>Its first mapped byte.</summary>
        public IntPtr Unwan { get; }

        /// <summary>Its length in bytes.</summary>
        public long Hajm { get; }

        /// <summary>Whether the range holds anything.</summary>
        public bool Salih => Unwan != IntPtr.Zero && Hajm > 0;

        /// <summary>Whether an address falls inside this range.</summary>
        /// <param name="unwan">The address.</param>
        /// <returns>Whether it is in range.</returns>
        public bool Yashmal(IntPtr unwan)
        {
            long qeema = unwan.ToInt64();
            long bidaya = Unwan.ToInt64();
            return qeema >= bidaya && qeema < bidaya + Hajm;
        }
    }

    /// <summary>
    /// Rung three of the ladder: locates the IL2CPP module, narrows to its
    /// executable sections, and matches the signature database's byte patterns
    /// there.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Construct once at plugin initialisation with the pattern provider from
    /// <c>Basmat/Qaida.cs</c>, and hand to <see cref="Sullam"/> last.
    /// Construction discovers the module and its executable sections once; the
    /// per-target path scans those ranges and does no discovery.
    /// </para>
    /// <para>
    /// Not thread-safe. Every target is resolved on one thread at load time.
    /// </para>
    /// </remarks>
    public sealed class Masah : IRutbatHall
    {
        /// <summary>
        /// The largest executable range this rung will scan, in bytes.
        /// </summary>
        /// <remarks>
        /// A sanity bound on the headers rather than on real binaries: no
        /// shipped GameAssembly has a quarter-gigabyte text section, so a
        /// section header claiming one has been corrupted or misparsed, and
        /// scanning it would read outside the mapping. The clamp turns that into
        /// a short scan that finds nothing instead of a fault this process
        /// cannot catch.
        /// </remarks>
        public const long AqsaHajmQitaa = 512L * 1024L * 1024L;

        /// <summary>
        /// The module names an IL2CPP game's compiled managed code lives in, per
        /// platform, in the order they are tried.
        /// </summary>
        /// <remarks>
        /// Windows is always GameAssembly.dll. Linux ships GameAssembly.so for
        /// desktop players and libil2cpp.so for the Android-derived layout some
        /// titles keep. macOS ships GameAssembly.dylib in the app bundle, while
        /// a title built from the iOS pipeline links everything into a single
        /// UnityFramework image; both occur in the wild, so both are probed.
        /// </remarks>
        private static readonly string[] AsmaWindows = { "GameAssembly.dll" };

        private static readonly string[] AsmaLinux =
        {
            "GameAssembly.so", "libil2cpp.so", "libgameassembly.so",
        };

        private static readonly string[] AsmaMac =
        {
            "GameAssembly.dylib", "UnityFramework", "libil2cpp.dylib",
        };

        private readonly IMazwadNamat mazwad;
        private readonly bool mutah;
        private readonly string sababAdamTawafur;
        private readonly WahdaMasah wahda;
        private readonly List<Qitaa> qitaat = new List<Qitaa>(2);

        /// <summary>
        /// Which pattern variant resolved which target, by qualified name.
        /// </summary>
        private readonly Dictionary<string, string> mutabaqat =
            new Dictionary<string, string>(StringComparer.Ordinal);

        /// <summary>
        /// Discovers the IL2CPP module and its executable sections, once, and
        /// decides whether this rung can run.
        /// </summary>
        /// <param name="mazwad">
        /// The signature database, from <c>Basmat/Qaida.cs</c>.
        /// </param>
        /// <exception cref="ArgumentNullException"><paramref name="mazwad"/> is null.</exception>
        /// <remarks>
        /// Discovery never throws out of here: a platform whose module list
        /// cannot be read is this rung being unavailable, which the ladder
        /// reports and continues past.
        /// </remarks>
        public Masah(IMazwadNamat mazwad)
        {
            this.mazwad = mazwad ?? throw new ArgumentNullException(nameof(mazwad));
            mutah = QarrirTawafur(out sababAdamTawafur, out wahda);
        }

        /// <inheritdoc/>
        public Rutba Rutba => Rutba.Basmat;

        /// <summary>The IL2CPP module this rung searches.</summary>
        public WahdaMasah Wahda => wahda;

        /// <summary>Its executable ranges, in header order.</summary>
        public IReadOnlyList<Qitaa> Qitaat => qitaat;

        /// <summary>
        /// Which pattern variant matched for each resolved target, by qualified
        /// name.
        /// </summary>
        /// <remarks>
        /// Read when the diagnostics bundle is written. Sullam records which
        /// rung fired; this records which of that rung's several candidates did,
        /// which is the difference between "resolved by signature" and "resolved
        /// by tmp_generate_2022_3_x64_v2, so the v1 pattern is now dead code in
        /// the database".
        /// </remarks>
        public IReadOnlyDictionary<string, string> Mutabaqat => mutabaqat;

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

            // The contract forbids throwing. This rung is the last one, so a
            // throw here does not cost the ladder another attempt — but Sullam
            // records a throw as a bug rather than as a decline, and a database
            // with one malformed entry should read as one bad entry, not as a
            // broken scanner.
            try
            {
                if (!mutah)
                {
                    sabab = sababAdamTawafur;
                    return false;
                }

                if (hadaf.MiftahBasma.Length == 0)
                {
                    sabab = "this target has no signature key, so no pattern exists for it";
                    return false;
                }

                IReadOnlyList<BasmaMurashaha> murashahat;
                try
                {
                    murashahat = mazwad.Murashahat(hadaf.MiftahBasma);
                }
                catch (Exception khata)
                {
                    sabab = $"the signature database threw looking up "
                        + $"\"{hadaf.MiftahBasma}\" ({khata.GetType().Name}: {khata.Message})";
                    return false;
                }

                if (murashahat is null || murashahat.Count == 0)
                {
                    sabab = $"the signature database has no pattern for "
                        + $"\"{hadaf.MiftahBasma}\" on this engine version, architecture "
                        + "and platform";
                    return false;
                }

                string asbab = string.Empty;
                for (int i = 0; i < murashahat.Count; i++)
                {
                    BasmaMurashaha murashah = murashahat[i];
                    if (JarribMurashah(murashah, out IntPtr natija, out string sababWahid))
                    {
                        unwan = natija;
                        mutabaqat[hadaf.Ism] = murashah.Ism;
                        return true;
                    }
                    string ism = murashah.Ism.Length == 0
                        ? "variant " + i.ToString(CultureInfo.InvariantCulture)
                        : murashah.Ism;
                    asbab = asbab.Length == 0
                        ? $"{ism}: {sababWahid}"
                        : asbab + "; " + $"{ism}: {sababWahid}";
                }

                sabab = $"no pattern for \"{hadaf.MiftahBasma}\" matched in "
                    + $"{wahda.Ism} ({asbab})";
                return false;
            }
            catch (Exception khata)
            {
                sabab = $"scanning for {hadaf.Ism} threw {khata.GetType().Name}: "
                    + khata.Message;
                return false;
            }
        }

        // -------------------------------------------------------------------
        // Availability: module and section discovery, once
        // -------------------------------------------------------------------

        private bool QarrirTawafur(out string sabab, out WahdaMasah wahdatuh)
        {
            wahdatuh = default;

            bool mutahMazwad;
            string sababMazwad;
            try
            {
                mutahMazwad = mazwad.Mutah(out sababMazwad);
            }
            catch (Exception khata)
            {
                sabab = $"the signature database threw when asked whether it is available "
                    + $"({khata.GetType().Name}: {khata.Message})";
                return false;
            }

            if (!mutahMazwad)
            {
                sabab = string.IsNullOrEmpty(sababMazwad)
                    ? "the signature database is not available in this build"
                    : sababMazwad;
                return false;
            }

            if (!IktishafWahda(out WahdaMasah mawjuda, out string sababWahda))
            {
                sabab = sababWahda;
                return false;
            }
            wahdatuh = mawjuda;

            if (!QitaatTanfeedh(mawjuda, qitaat, out string sababQitaa))
            {
                sabab = sababQitaa;
                return false;
            }

            sabab = string.Empty;
            return true;
        }

        /// <summary>
        /// Finds the module holding this game's compiled managed code, on
        /// whichever platform the process is running.
        /// </summary>
        /// <param name="wahdatuh">The module, when one was found.</param>
        /// <param name="sabab">Why not, when none was.</param>
        /// <returns>Whether a module was found.</returns>
        public static bool IktishafWahda(out WahdaMasah wahdatuh, out string sabab)
        {
            wahdatuh = default;

            try
            {
                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                {
                    return WahdaWindows(AsmaWindows, out wahdatuh, out sabab);
                }
                if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
                {
                    return WahdaLinux(AsmaLinux, out wahdatuh, out sabab);
                }
                if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
                {
                    return WahdaMac(AsmaMac, out wahdatuh, out sabab);
                }
            }
            catch (Exception khata)
            {
                sabab = $"listing this process's modules threw {khata.GetType().Name}: "
                    + khata.Message;
                return false;
            }

            sabab = "this platform is not Windows, Linux or macOS, so no module layout "
                + "this build knows how to read applies";
            return false;
        }

        /// <summary>
        /// Windows: the loader keeps a module list per process and the BCL
        /// exposes it, base address and mapped size included.
        /// </summary>
        /// <remarks>
        /// ProcessModule.ModuleMemorySize is the loader's own SizeOfImage, which
        /// is what bounds the header walk below — a section header claiming a
        /// range past the end of the mapping is then clamped rather than
        /// followed. Matching is on ModuleName rather than FileName because a
        /// game launched through a shim can have the module resident under a
        /// path that is not where it was installed.
        /// </remarks>
        private static bool WahdaWindows(string[] asma, out WahdaMasah wahdatuh, out string sabab)
        {
            wahdatuh = default;
            using (Process amaliya = Process.GetCurrentProcess())
            {
                ProcessModuleCollection wahdat = amaliya.Modules;
                for (int i = 0; i < asma.Length; i++)
                {
                    for (int j = 0; j < wahdat.Count; j++)
                    {
                        ProcessModule wahda = wahdat[j];
                        string? ism = wahda.ModuleName;
                        if (ism is null
                            || !string.Equals(ism, asma[i], StringComparison.OrdinalIgnoreCase))
                        {
                            continue;
                        }
                        if (wahda.BaseAddress == IntPtr.Zero || wahda.ModuleMemorySize <= 0)
                        {
                            continue;
                        }
                        wahdatuh = new WahdaMasah(ism, wahda.BaseAddress, wahda.ModuleMemorySize);
                        sabab = string.Empty;
                        return true;
                    }
                }

                sabab = $"no module named {string.Join(" or ", asma)} is loaded in this "
                    + $"process (it has {wahdat.Count} modules), so this game's compiled "
                    + "code could not be located to scan";
                return false;
            }
        }

        /// <summary>
        /// Linux: /proc/self/maps is the kernel's own record of every mapping,
        /// which is more trustworthy here than any library's model of it.
        /// </summary>
        /// <remarks>
        /// <para>
        /// A shared object appears as several consecutive mappings with
        /// different protections — one for the code, one for read-only data, one
        /// for the writable data — all naming the same file. The base is the
        /// lowest start among them and the extent runs to the highest end, which
        /// together bound the ELF header walk.
        /// </para>
        /// <para>
        /// The path is taken as everything from the first '/' on the line: the
        /// five fields before it are an address range, a permission string, a
        /// hex offset, a "major:minor" device and a decimal inode, none of which
        /// can contain a slash, and a game installed under a path with spaces in
        /// it would defeat splitting on whitespace. Lines with no '/' are
        /// anonymous or pseudo-mappings and are skipped.
        /// </para>
        /// </remarks>
        private static bool WahdaLinux(string[] asma, out WahdaMasah wahdatuh, out string sabab)
        {
            wahdatuh = default;

            string[] sutur;
            try
            {
                sutur = File.ReadAllLines("/proc/self/maps");
            }
            catch (IOException khata)
            {
                sabab = $"/proc/self/maps could not be read ({khata.Message}), so this "
                    + "process's mappings are unknown";
                return false;
            }
            catch (UnauthorizedAccessException khata)
            {
                sabab = $"/proc/self/maps could not be read ({khata.Message}), so this "
                    + "process's mappings are unknown";
                return false;
            }

            for (int i = 0; i < asma.Length; i++)
            {
                long adna = long.MaxValue;
                long aqsa = 0;
                string ism = asma[i];

                for (int j = 0; j < sutur.Length; j++)
                {
                    string satr = sutur[j];
                    int mailan = satr.IndexOf('/');
                    if (mailan <= 0)
                    {
                        continue;
                    }
                    string masar = satr.Substring(mailan).TrimEnd();
                    if (!MutabiqIsmMilaff(masar, ism))
                    {
                        continue;
                    }
                    if (!MadaSatr(satr, out long bidaya, out long nihaya))
                    {
                        continue;
                    }
                    if (bidaya < adna)
                    {
                        adna = bidaya;
                    }
                    if (nihaya > aqsa)
                    {
                        aqsa = nihaya;
                    }
                }

                if (adna != long.MaxValue && aqsa > adna)
                {
                    wahdatuh = new WahdaMasah(ism, new IntPtr(adna), aqsa - adna);
                    sabab = string.Empty;
                    return true;
                }
            }

            sabab = $"no mapping in /proc/self/maps names {string.Join(" or ", asma)}, so "
                + "this game's compiled code could not be located to scan";
            return false;
        }

        /// <summary>Parses the "start-end" field at the head of a maps line.</summary>
        private static bool MadaSatr(string satr, out long bidaya, out long nihaya)
        {
            bidaya = 0;
            nihaya = 0;
            int sharta = satr.IndexOf('-');
            if (sharta <= 0)
            {
                return false;
            }
            int masafa = satr.IndexOf(' ', sharta);
            if (masafa <= sharta)
            {
                return false;
            }

            return long.TryParse(
                       satr.AsSpan(0, sharta),
                       NumberStyles.HexNumber,
                       CultureInfo.InvariantCulture,
                       out bidaya)
                   && long.TryParse(
                       satr.AsSpan(sharta + 1, masafa - sharta - 1),
                       NumberStyles.HexNumber,
                       CultureInfo.InvariantCulture,
                       out nihaya);
        }

        /// <summary>Whether a mapped path's file name is the one being looked for.</summary>
        private static bool MutabiqIsmMilaff(string masar, string ism)
        {
            int fasil = masar.LastIndexOf('/');
            string milaff = fasil < 0 ? masar : masar.Substring(fasil + 1);
            return string.Equals(milaff, ism, StringComparison.OrdinalIgnoreCase);
        }

        /// <summary>
        /// macOS: dyld's own image list, which is the only complete record of
        /// what is mapped in a process that may have loaded frameworks from
        /// inside an app bundle.
        /// </summary>
        /// <remarks>
        /// The header address dyld reports is where the Mach-O header is mapped,
        /// which is what the load-command walk needs. The size is not reported
        /// alongside it, so it is summed from the segment commands during the
        /// header walk; until then the extent is left as the largest value the
        /// section clamp will accept, and the walk narrows it.
        /// </remarks>
        private static bool WahdaMac(string[] asma, out WahdaMasah wahdatuh, out string sabab)
        {
            wahdatuh = default;
            uint adad = DyldImageCount();

            for (int i = 0; i < asma.Length; i++)
            {
                for (uint j = 0; j < adad; j++)
                {
                    IntPtr khamIsm = DyldGetImageName(j);
                    if (khamIsm == IntPtr.Zero)
                    {
                        continue;
                    }
                    string? masar = Marshal.PtrToStringAnsi(khamIsm);
                    if (masar is null || !MutabiqIsmMilaff(masar, asma[i]))
                    {
                        continue;
                    }
                    IntPtr rasiya = DyldGetImageHeader(j);
                    if (rasiya == IntPtr.Zero)
                    {
                        continue;
                    }
                    // The extent is refined by the Mach-O walk; AqsaHajmQitaa is
                    // the clamp every section is checked against anyway, so
                    // starting here costs nothing and avoids inventing a size.
                    wahdatuh = new WahdaMasah(asma[i], rasiya, AqsaHajmQitaa);
                    sabab = string.Empty;
                    return true;
                }
            }

            sabab = $"dyld reports no image named {string.Join(" or ", asma)} among "
                + $"{adad} loaded images, so this game's compiled code could not be "
                + "located to scan";
            return false;
        }

        // -------------------------------------------------------------------
        // Header parsing: which bytes of this module are code
        // -------------------------------------------------------------------

        /// <summary>
        /// Reads the module's headers and collects every range the loader
        /// marked executable.
        /// </summary>
        /// <param name="wahda">The module.</param>
        /// <param name="natija">Receives the ranges, in header order.</param>
        /// <param name="sabab">Why not, when none could be read.</param>
        /// <returns>Whether at least one executable range was found.</returns>
        public static bool QitaatTanfeedh(WahdaMasah wahda, List<Qitaa> natija, out string sabab)
        {
            if (natija is null)
            {
                throw new ArgumentNullException(nameof(natija));
            }
            if (!wahda.Wujid)
            {
                sabab = "no module was found, so its headers cannot be read";
                return false;
            }

            bool qura;
            try
            {
                unsafe
                {
                    // SOUND: the module's headers are mapped readable at its
                    // base for the life of the process — a loaded module is
                    // never unmapped by this plugin and IL2CPP's own module is
                    // never unloaded by the game. Every read below is bounded
                    // against the module's reported extent before it happens, so
                    // a corrupt or hostile header cannot walk this loop off the
                    // end of the mapping.
                    byte* qaida = (byte*)wahda.Qaida;
                    qura = QaraRaesiya(qaida, wahda.Hajm, natija, out sabab);
                }
            }
            catch (Exception khata)
            {
                sabab = $"reading {wahda.Ism}'s headers threw {khata.GetType().Name}: "
                    + khata.Message;
                return false;
            }

            if (!qura)
            {
                return false;
            }

            if (natija.Count == 0)
            {
                sabab = $"{wahda.Ism} declares no executable section, which no loadable "
                    + "module does — its headers were misread or have been tampered with";
                return false;
            }

            sabab = string.Empty;
            return true;
        }

        private static unsafe bool QaraRaesiya(
            byte* qaida, long hajm, List<Qitaa> natija, out string sabab)
        {
            // SOUND: qaida is the module's mapped base and hajm its reported
            // extent; every read below is bounded against hajm before it runs,
            // and a loaded module's headers stay mapped for the process's life.
            if (hajm < 64)
            {
                sabab = "the module's mapped extent is too small to hold any header";
                return false;
            }

            ushort dos = Iqra16(qaida, 0);
            uint sihr = Iqra32(qaida, 0);

            if (dos == 0x5A4D)
            {
                return QitaatPe(qaida, hajm, natija, out sabab);
            }
            if (sihr == 0x464C457FU)
            {
                return QitaatElf(qaida, hajm, natija, out sabab);
            }
            if (sihr == 0xFEEDFACFU || sihr == 0xFEEDFACEU)
            {
                return QitaatMachO(qaida, hajm, natija, out sabab);
            }

            sabab = $"the module's first bytes are 0x{sihr:X8}, which is neither PE, ELF "
                + "nor Mach-O, so its executable range cannot be determined";
            return false;
        }

        /// <summary>
        /// Windows: walk the PE section table and take every section whose
        /// characteristics carry IMAGE_SCN_MEM_EXECUTE.
        /// </summary>
        /// <remarks>
        /// Every offset here is against the mapped image rather than the file,
        /// which is why VirtualAddress and not PointerToRawData is added to the
        /// base: the loader has already applied the section alignment, and a
        /// scan against raw file offsets would be reading whatever the mapping
        /// put there instead. VirtualSize is the authority on length, with
        /// SizeOfRawData standing in only when VirtualSize is zero, which some
        /// old linkers emit.
        /// </remarks>
        private static unsafe bool QitaatPe(
            byte* qaida, long hajm, List<Qitaa> natija, out string sabab)
        {
            // SOUND: as QaraRaesiya. The DOS stub, the PE header and the whole
            // section table are bounds-checked against hajm before any field of
            // any of them is read.
            long lfanew = Iqra32(qaida, 0x3C);
            if (lfanew <= 0 || lfanew + 24 > hajm)
            {
                sabab = "the PE header offset in the DOS stub points outside the mapping";
                return false;
            }
            if (Iqra32(qaida, lfanew) != 0x00004550U)
            {
                sabab = "the module has a DOS stub but no PE signature where it points";
                return false;
            }

            int adadQitaat = Iqra16(qaida, lfanew + 6);
            int hajmIkhtiyari = Iqra16(qaida, lfanew + 20);
            long jadwal = lfanew + 24 + hajmIkhtiyari;

            if (adadQitaat <= 0 || jadwal + ((long)adadQitaat * 40) > hajm)
            {
                sabab = $"the PE section table ({adadQitaat} sections) does not fit inside "
                    + "the module's mapped extent";
                return false;
            }

            for (int i = 0; i < adadQitaat; i++)
            {
                long madkhal = jadwal + ((long)i * 40);
                uint khasais = Iqra32(qaida, madkhal + 36);
                const uint QabilTanfeedh = 0x20000000U;
                if ((khasais & QabilTanfeedh) == 0)
                {
                    continue;
                }

                uint hajmWahmi = Iqra32(qaida, madkhal + 8);
                uint unwanWahmi = Iqra32(qaida, madkhal + 12);
                uint hajmKham = Iqra32(qaida, madkhal + 16);
                long tul = hajmWahmi != 0 ? hajmWahmi : hajmKham;
                Adif(natija, IsmQitaaPe(qaida, madkhal), qaida, unwanWahmi, tul, hajm);
            }

            sabab = string.Empty;
            return true;
        }

        private static unsafe string IsmQitaaPe(byte* qaida, long madkhal)
        {
            // SOUND: madkhal is a section-table entry the caller already
            // bounded against the module's extent, and exactly the entry's eight
            // name bytes are read. The stackalloc is eight chars on this frame.
            // Section names are eight bytes, NUL-padded rather than
            // NUL-terminated, so a full eight-character name has no terminator
            // and PtrToStringAnsi would run into the next field.
            char* huruf = stackalloc char[8];
            int tul = 0;
            for (int i = 0; i < 8; i++)
            {
                byte b = *(qaida + madkhal + i);
                if (b == 0)
                {
                    break;
                }
                huruf[tul++] = (char)b;
            }
            return new string(huruf, 0, tul);
        }

        /// <summary>
        /// Linux: walk the ELF program headers and take every PT_LOAD segment
        /// whose flags carry PF_X.
        /// </summary>
        /// <remarks>
        /// <para>
        /// Program headers rather than section headers, and deliberately: a
        /// stripped shared object need not have its section headers mapped at
        /// all — they live past the last PT_LOAD and the loader has no reason to
        /// bring them in — while the program headers are what the loader itself
        /// read, so they are always present at run time.
        /// </para>
        /// <para>
        /// The load bias is computed rather than assumed. For a position
        /// independent object the first PT_LOAD has p_vaddr zero and the bias is
        /// the mapped base, but a prelinked or non-PIE object carries real
        /// virtual addresses, and adding those to the base would land a scan
        /// twice as far into the address space as the module reaches. Bias =
        /// base - (lowest PT_LOAD p_vaddr) is correct in both cases.
        /// </para>
        /// </remarks>
        private static unsafe bool QitaatElf(
            byte* qaida, long hajm, List<Qitaa> natija, out string sabab)
        {
            // SOUND: as QaraRaesiya. e_ident[4] lies within the sixty-four
            // bytes QaraRaesiya required, and the program header table is
            // bounds-checked against hajm before any entry is read.
            byte fia = *(qaida + 4);
            bool sitta = fia == 2;
            if (fia != 1 && fia != 2)
            {
                sabab = $"the ELF class byte is {fia}, which is neither 32-bit nor 64-bit";
                return false;
            }

            long phoff = sitta ? (long)Iqra64(qaida, 32) : Iqra32(qaida, 28);
            int phentsize = Iqra16(qaida, sitta ? 54 : 42);
            int phnum = Iqra16(qaida, sitta ? 56 : 44);
            int adnaHajm = sitta ? 56 : 32;

            if (phoff <= 0 || phnum <= 0 || phentsize < adnaHajm
                || phoff + ((long)phnum * phentsize) > hajm)
            {
                sabab = $"the ELF program header table ({phnum} entries of {phentsize} "
                    + "bytes) does not fit inside the module's mapped extent";
                return false;
            }

            const uint Tahmeel = 1;
            const uint Tanfeedh = 1;

            long adnaAsli = long.MaxValue;
            for (int i = 0; i < phnum; i++)
            {
                long madkhal = phoff + ((long)i * phentsize);
                if (Iqra32(qaida, madkhal) != Tahmeel)
                {
                    continue;
                }
                long vaddr = sitta ? (long)Iqra64(qaida, madkhal + 16) : Iqra32(qaida, madkhal + 8);
                if (vaddr < adnaAsli)
                {
                    adnaAsli = vaddr;
                }
            }
            if (adnaAsli == long.MaxValue)
            {
                sabab = "the ELF program headers declare no loadable segment";
                return false;
            }

            long inhiraf = (long)qaida - adnaAsli;

            for (int i = 0; i < phnum; i++)
            {
                long madkhal = phoff + ((long)i * phentsize);
                if (Iqra32(qaida, madkhal) != Tahmeel)
                {
                    continue;
                }

                uint alam = sitta ? Iqra32(qaida, madkhal + 4) : Iqra32(qaida, madkhal + 24);
                if ((alam & Tanfeedh) == 0)
                {
                    continue;
                }

                long vaddr = sitta
                    ? (long)Iqra64(qaida, madkhal + 16)
                    : Iqra32(qaida, madkhal + 8);
                long memsz = sitta
                    ? (long)Iqra64(qaida, madkhal + 40)
                    : Iqra32(qaida, madkhal + 20);
                long unwan = inhiraf + vaddr;
                AdifMutlaq(natija, ".text", new IntPtr(unwan), memsz, qaida, hajm);
            }

            sabab = string.Empty;
            return true;
        }

        /// <summary>
        /// macOS: walk the Mach-O load commands and take the __text section of
        /// every segment whose initial protection carries VM_PROT_EXECUTE.
        /// </summary>
        /// <remarks>
        /// The __TEXT segment is not all code: it also holds __cstring,
        /// __const, __unwind_info and the Objective-C metadata, all of which are
        /// mapped executable because the whole segment is, and all of which are
        /// exactly the read-only data a pattern can match in by accident. So the
        /// walk descends into the segment's section table and takes __text
        /// specifically, falling back to the whole segment only when a segment
        /// declares no sections at all.
        /// </remarks>
        private static unsafe bool QitaatMachO(
            byte* qaida, long hajm, List<Qitaa> natija, out string sabab)
        {
            // SOUND: as QaraRaesiya. The Mach-O header lies within the
            // sixty-four bytes QaraRaesiya required, and every load command is
            // checked to fit inside hajm before any of its fields is read.
            uint sihr = Iqra32(qaida, 0);
            bool sitta = sihr == 0xFEEDFACFU;
            long raes = sitta ? 32 : 28;
            int adadAwamir = (int)Iqra32(qaida, 16);

            if (adadAwamir <= 0 || raes >= hajm)
            {
                sabab = "the Mach-O header declares no load commands";
                return false;
            }

            const uint QitaaSitta = 0x19;
            const uint QitaaArbaa = 0x01;
            const int Tanfeedh = 0x04;

            long mawqi = raes;
            for (int i = 0; i < adadAwamir; i++)
            {
                if (mawqi + 8 > hajm)
                {
                    sabab = "the Mach-O load command list runs past the module's mapped "
                        + "extent";
                    return false;
                }

                uint amr = Iqra32(qaida, mawqi);
                long hajmAmr = Iqra32(qaida, mawqi + 4);
                if (hajmAmr < 8 || mawqi + hajmAmr > hajm)
                {
                    sabab = $"a Mach-O load command declares a length of {hajmAmr} bytes, "
                        + "which does not fit inside the module's mapped extent";
                    return false;
                }

                bool huwaQitaa = sitta ? amr == QitaaSitta : amr == QitaaArbaa;
                if (huwaQitaa)
                {
                    int himaya = sitta
                        ? (int)Iqra32(qaida, mawqi + 60)
                        : (int)Iqra32(qaida, mawqi + 44);
                    if ((himaya & Tanfeedh) != 0)
                    {
                        QitaaMachOWahid(qaida, hajm, natija, mawqi, hajmAmr, sitta);
                    }
                }

                mawqi += hajmAmr;
            }

            sabab = string.Empty;
            return true;
        }

        private static unsafe void QitaaMachOWahid(
            byte* qaida, long hajm, List<Qitaa> natija, long mawqi, long hajmAmr, bool sitta)
        {
            // SOUND: the caller checked that this load command's whole cmdsize
            // fits inside hajm, and each section entry is re-checked against
            // both the command's end and hajm before it is read.
            long vmaddr = sitta ? (long)Iqra64(qaida, mawqi + 24) : Iqra32(qaida, mawqi + 24);
            long vmsize = sitta ? (long)Iqra64(qaida, mawqi + 32) : Iqra32(qaida, mawqi + 28);
            int nsects = sitta ? (int)Iqra32(qaida, mawqi + 64) : (int)Iqra32(qaida, mawqi + 48);
            long raesQitaa = sitta ? 72 : 56;
            long hajmQism = sitta ? 80 : 68;

            // The slide is the difference between where the header is mapped and
            // where the __TEXT segment says it should be. dyld reports it
            // separately, but deriving it from the segment that contains the
            // header needs no second call and cannot disagree with the addresses
            // being read out of that same header.
            long inhiraf = 0;
            if (vmaddr != 0 && vmsize != 0)
            {
                inhiraf = (long)qaida - vmaddr;
            }

            bool wujidNass = false;
            for (int j = 0; j < nsects; j++)
            {
                long madkhal = mawqi + raesQitaa + ((long)j * hajmQism);
                if (madkhal + hajmQism > mawqi + hajmAmr || madkhal + hajmQism > hajm)
                {
                    break;
                }
                if (!IsmuhuNass(qaida, madkhal))
                {
                    continue;
                }
                long unwan = sitta
                    ? (long)Iqra64(qaida, madkhal + 32)
                    : Iqra32(qaida, madkhal + 32);
                long tul = sitta
                    ? (long)Iqra64(qaida, madkhal + 40)
                    : Iqra32(qaida, madkhal + 36);
                AdifMutlaq(natija, "__text", new IntPtr(inhiraf + unwan), tul, qaida, hajm);
                wujidNass = true;
            }

            if (!wujidNass)
            {
                AdifMutlaq(natija, "__TEXT", new IntPtr(inhiraf + vmaddr), vmsize, qaida, hajm);
            }
        }

        /// <summary>Whether a Mach-O section entry is named "__text".</summary>
        private static unsafe bool IsmuhuNass(byte* qaida, long madkhal)
        {
            // SOUND: madkhal is a section entry the caller bounded against both
            // the load command's end and the module's extent, and exactly seven
            // bytes of its sixteen-byte name field are compared.
            ReadOnlySpan<byte> matlub = new byte[] { 0x5F, 0x5F, 0x74, 0x65, 0x78, 0x74, 0x00 };
            for (int i = 0; i < matlub.Length; i++)
            {
                if (*(qaida + madkhal + i) != matlub[i])
                {
                    return false;
                }
            }
            return true;
        }

        /// <summary>Adds a range expressed as an offset from the module base.</summary>
        private static unsafe void Adif(
            List<Qitaa> natija, string ism, byte* qaida, long izaha, long tul, long hajmWahda)
        {
            // SOUND: nothing is dereferenced here. qaida is used only as an
            // integer base for address arithmetic, and AdifMutlaq clamps the
            // result to the module's extent before it becomes a scannable range.
            AdifMutlaq(natija, ism, new IntPtr((long)qaida + izaha), tul, qaida, hajmWahda);
        }

        /// <summary>
        /// Adds a range expressed as an absolute address, clamped to what the
        /// module actually occupies.
        /// </summary>
        /// <remarks>
        /// The clamp is the whole defence against a malformed or hostile header.
        /// A section header claiming a length past the end of the mapping is not
        /// a theoretical case — packers rewrite these fields deliberately — and
        /// reading past a mapping in .NET is not an exception that can be
        /// caught: it is a signal the runtime turns into process death. So the
        /// range is trimmed to the module's extent here, before any byte of it
        /// is read, rather than trusted and guarded afterwards.
        /// </remarks>
        private static unsafe void AdifMutlaq(
            List<Qitaa> natija, string ism, IntPtr unwan, long tul, byte* qaida, long hajmWahda)
        {
            // SOUND: nothing is dereferenced here either. qaida and hajmWahda
            // describe the module's extent and are used only to reject or trim a
            // range, which is what stops a bad header producing a bad scan.
            if (tul <= 0 || unwan == IntPtr.Zero)
            {
                return;
            }

            long bidayaWahda = (long)qaida;
            long nihayaWahda = bidayaWahda + hajmWahda;
            long bidaya = unwan.ToInt64();
            if (bidaya < bidayaWahda || bidaya >= nihayaWahda)
            {
                return;
            }

            long baqi = nihayaWahda - bidaya;
            if (tul > baqi)
            {
                tul = baqi;
            }
            if (tul > AqsaHajmQitaa)
            {
                tul = AqsaHajmQitaa;
            }
            if (tul <= 0)
            {
                return;
            }

            natija.Add(new Qitaa(ism, unwan, tul));
        }

        // -------------------------------------------------------------------
        // Bounded little-endian reads over the mapped headers
        // -------------------------------------------------------------------

        private static unsafe ushort Iqra16(byte* qaida, long izaha)
        {
            // SOUND: every caller has already bounded izaha against the module's
            // reported extent, and the module's headers are mapped readable for
            // the life of the process. The reads are byte-wise rather than a
            // cast so an unaligned header field cannot fault on ARM64, where an
            // unaligned load of a wider type is not merely slow.
            byte* p = qaida + izaha;
            return (ushort)(p[0] | (p[1] << 8));
        }

        private static unsafe uint Iqra32(byte* qaida, long izaha)
        {
            // SOUND: as Iqra16 — bounded by the caller, mapped for the process
            // lifetime, byte-wise to tolerate unaligned header fields.
            byte* p = qaida + izaha;
            return (uint)(p[0] | (p[1] << 8) | (p[2] << 16) | (p[3] << 24));
        }

        private static unsafe ulong Iqra64(byte* qaida, long izaha)
        {
            // SOUND: as Iqra16.
            return Iqra32(qaida, izaha) | ((ulong)Iqra32(qaida, izaha + 4) << 32);
        }

        // -------------------------------------------------------------------
        // The scan
        // -------------------------------------------------------------------

        /// <summary>
        /// Tries one candidate pattern across every executable range of its
        /// module.
        /// </summary>
        /// <remarks>
        /// Exactly one surviving match is a resolution. Zero is a decline that
        /// lets the next candidate run. More than one is a refusal, because the
        /// scanner has no basis for preferring one address over another and
        /// taking the first would silently detour whichever function the linker
        /// happened to place lower — a coin flip made once, at install time, in
        /// a way nothing downstream can detect.
        /// </remarks>
        private bool JarribMurashah(BasmaMurashaha murashah, out IntPtr unwan, out string sabab)
        {
            unwan = IntPtr.Zero;

            Namat? namat = murashah.Namat;
            if (namat is null)
            {
                sabab = "the database offered a candidate with no pattern in it";
                return false;
            }

            List<Qitaa> nitaq;
            WahdaMasah hadafWahda = wahda;
            if (murashah.Wahda.Length == 0
                || string.Equals(murashah.Wahda, wahda.Ism, StringComparison.OrdinalIgnoreCase))
            {
                nitaq = qitaat;
            }
            else
            {
                nitaq = new List<Qitaa>(2);
                bool wujidat = IktishafWahdaBiIsm(
                    murashah.Wahda, out WahdaMasah ukhra, out string sababUkhra);
                if (!wujidat)
                {
                    sabab = sababUkhra;
                    return false;
                }
                if (!QitaatTanfeedh(ukhra, nitaq, out string sababQitaa))
                {
                    sabab = sababQitaa;
                    return false;
                }
                hadafWahda = ukhra;
            }

            // Each executable range is searched separately, and a pattern that
            // matched in one must not match in another: uniqueness is a property
            // of the module, not of one section. Namat proves it within a range;
            // this loop proves it across them.
            IntPtr wahid = IntPtr.Zero;
            int najin = 0;
            string asbab = string.Empty;

            for (int i = 0; i < nitaq.Count; i++)
            {
                Qitaa qitaa = nitaq[i];
                if (!qitaa.Salih)
                {
                    continue;
                }

                MadaMasah mada = new MadaMasah(
                    qitaa.Unwan,
                    qitaa.Hajm,
                    hadafWahda.Qaida,
                    hadafWahda.Hajm,
                    hadafWahda.Ism,
                    BinyatHadhihiAlAala());
                if (!mada.Salih(out string sababMada))
                {
                    asbab = Adif(asbab, qitaa.Ism, sababMada);
                    continue;
                }

                NatijatMasah natija;
                try
                {
                    natija = namat.Masah(in mada);
                }
                catch (Exception khata)
                {
                    asbab = Adif(
                        asbab, qitaa.Ism,
                        $"the pattern threw {khata.GetType().Name}: {khata.Message}");
                    continue;
                }

                if (!natija.Wujid)
                {
                    asbab = Adif(asbab, qitaa.Ism, natija.Sabab);
                    continue;
                }

                najin++;
                if (najin == 1)
                {
                    wahid = natija.Unwan;
                    continue;
                }

                sabab = "the pattern matched in more than one executable range, so choosing "
                    + "one would be a coin flip";
                return false;
            }

            if (najin == 1)
            {
                unwan = wahid;
                sabab = string.Empty;
                return true;
            }

            sabab = asbab.Length == 0
                ? $"no match in {nitaq.Count} executable range(s)"
                : asbab;
            return false;
        }

        /// <summary>Joins one range's reason onto the accumulated sentence.</summary>
        private static string Adif(string asbab, string ism, string sabab)
        {
            string wahid = $"{ism}: "
                + (string.IsNullOrEmpty(sabab) ? "no match" : sabab);
            return asbab.Length == 0 ? wahid : asbab + "; " + wahid;
        }

        /// <summary>This process's architecture, as the pattern layer names it.</summary>
        /// <remarks>
        /// Taken from the running process rather than from the database entry,
        /// because the entry says which architecture a pattern was written for
        /// and this says which one is actually executing. A mismatch between
        /// them means the wrong entry was selected, and the alignment rule the
        /// scan applies has to follow the machine.
        /// </remarks>
        private static Binya BinyatHadhihiAlAala()
        {
            return RuntimeInformation.ProcessArchitecture switch
            {
                Architecture.Arm64 => Binya.Aarch64,
                Architecture.X64 => Binya.X8664,
                _ => Binya.X8664,
            };
        }


        /// <summary>Whether one window of the range satisfies the masked pattern.</summary>
        private static bool Yutabiq(
            ReadOnlySpan<byte> nafidha, ReadOnlySpan<byte> namat, ReadOnlySpan<byte> qina)
        {
            for (int i = 0; i < namat.Length; i++)
            {
                if (qina[i] != 0 && nafidha[i] != namat[i])
                {
                    return false;
                }
            }
            return true;
        }

        /// <summary>
        /// Finds a module the signature database named explicitly, rather than
        /// the IL2CPP module this rung discovered.
        /// </summary>
        /// <remarks>
        /// For the small number of targets that are not in GameAssembly at all —
        /// a function in UnityPlayer, or in the engine's own text module. The
        /// database names the file and this resolves it the same way, on the
        /// same three platforms.
        /// </remarks>
        private static bool IktishafWahdaBiIsm(
            string ism, out WahdaMasah wahdatuh, out string sabab)
        {
            string[] wahid = { ism };
            wahdatuh = default;
            try
            {
                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                {
                    return WahdaWindows(wahid, out wahdatuh, out sabab);
                }
                if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
                {
                    return WahdaLinux(wahid, out wahdatuh, out sabab);
                }
                if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
                {
                    return WahdaMac(wahid, out wahdatuh, out sabab);
                }
            }
            catch (Exception khata)
            {
                sabab = $"looking for module {ism} threw {khata.GetType().Name}: "
                    + khata.Message;
                return false;
            }

            sabab = $"module {ism} cannot be looked for on this platform";
            return false;
        }

        // -------------------------------------------------------------------
        // dyld, on macOS only
        // -------------------------------------------------------------------
        //
        // These are operating-system loader queries, not the Taarib ABI, so they
        // do not belong in Taarib.Unity.Jisr — that file is the single surface
        // for taarib_* and stays that way. They are declared against
        // libSystem.B.dylib by absolute path, as Muhammil does, because a bare
        // "libSystem" does not resolve on every macOS version the games run on.

        [DllImport("/usr/lib/libSystem.B.dylib", EntryPoint = "_dyld_image_count")]
        private static extern uint DyldImageCount();

        [DllImport("/usr/lib/libSystem.B.dylib", EntryPoint = "_dyld_get_image_name")]
        private static extern IntPtr DyldGetImageName(uint fahras);

        [DllImport("/usr/lib/libSystem.B.dylib", EntryPoint = "_dyld_get_image_header")]
        private static extern IntPtr DyldGetImageHeader(uint fahras);
    }
}
