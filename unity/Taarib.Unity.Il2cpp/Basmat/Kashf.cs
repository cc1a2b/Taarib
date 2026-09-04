// كشف — finding out which engine this is, without asking the engine.
//
// The signature database is keyed by Unity version. The signature database
// exists for games whose global-metadata.dat cannot be read. Those two sentences
// together are the whole reason this file is not three lines long.
//
// The obvious way to learn the engine version is UnityEngine.Application.
// unityVersion. Under IL2CPP that property is reached through Il2CppInterop's
// generated assemblies, which are generated from the metadata — so on exactly
// the games where the version decides which byte pattern to use, asking the
// engine returns nothing. A version lookup that depends on metadata is a version
// lookup that is unavailable precisely when it is needed.
//
// So the order here is: the file system first, the engine last.
//
//   1. <game>_Data/globalgamemanagers — Unity writes the build's version string
//      into this file's header as a NUL-terminated ASCII run, at a small and
//      predictable offset. It is present in every non-Android desktop build and
//      it is readable with no engine involvement at all.
//   2. <game>_Data/data.unity3d — the same string, for builds that pack
//      globalgamemanagers into the bundle instead of writing it beside them.
//   3. The executable's own version resource on Windows, which Unity stamps
//      with the engine version for most build configurations.
//   4. UnityEngine.Application.unityVersion, last, for the case where the file
//      system said nothing but the metadata is fine — which is the case where
//      the signature database will not be consulted anyway.
//
// Every step that fails says why, and the sentence travels to the caller, so a
// game that resolves to the wrong pattern set can be diagnosed from a log rather
// than from a guess.
//
// A VERSION THAT CANNOT BE READ IS NOT ZERO. It is unread, and Isdar carries
// that distinction in Maqru. A version comparing as 0.0.0 would match every
// range with an open lower bound, which would apply a pattern written for a
// completely different engine — and a byte pattern applied to the wrong engine
// resolves to whatever happens to look similar, which is a detour installed on
// an unrelated function.

using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;

namespace Taarib.Unity.Il2cpp.Basmat
{
    /// <summary>
    /// Determines the engine version, the text package version, the
    /// architecture and the platform, without depending on the metadata.
    /// </summary>
    public static class Kashf
    {
        /// <summary>
        /// How far into <c>globalgamemanagers</c> the version string may begin.
        /// </summary>
        /// <remarks>
        /// Unity's serialized-file header puts the version at a small offset
        /// that has moved between format versions, so the search is bounded
        /// rather than fixed: a bounded scan of the first kilobyte finds it
        /// across every format this product targets, and refusing to scan the
        /// whole file is what keeps a corrupt or unrelated file from yielding
        /// something that merely looks like a version.
        /// </remarks>
        public const int AqsaIzahatIsdar = 1024;

        /// <summary>The most bytes of a candidate version string to accept.</summary>
        public const int AqsaTulIsdar = 32;

        private static string sababAkhir = string.Empty;

        /// <summary>
        /// The Unity version this game was built with.
        /// </summary>
        /// <param name="sabab">
        /// Empty when a version was read; otherwise why every source declined,
        /// in one sentence.
        /// </param>
        /// <returns>
        /// The version, or an unread <see cref="Isdar"/> whose
        /// <see cref="Isdar.Maqru"/> is false.
        /// </returns>
        /// <remarks>
        /// Never throws. A version that cannot be determined is a signature
        /// database that will decline every lookup with a reason, which is the
        /// correct outcome — far better than a version guessed wrongly, which is
        /// a byte pattern applied to an engine it was not written for.
        /// </remarks>
        public static Isdar IsdarMuharrik(out string sabab)
        {
            sababAkhir = string.Empty;

            string? min = MinGlobalGameManagers();
            if (min is null)
            {
                min = MinDataUnity3d();
            }
            if (min is null)
            {
                min = MinMawaridAlTanfidhi();
            }
            if (min is null)
            {
                min = MinAlMuharrik();
            }

            if (min is null)
            {
                sabab = sababAkhir.Length == 0
                    ? "no source could supply the engine version"
                    : sababAkhir;
                return default;
            }

            if (!Isdar.HawilHallil(min, out Isdar isdar, out string sababHall))
            {
                sabab = $"the engine reported \"{min}\", which is not a version this build "
                    + $"can parse: {sababHall}";
                return default;
            }

            sabab = string.Empty;
            return isdar;
        }

        /// <summary>
        /// The TextMeshPro version this game ships, when it can be told.
        /// </summary>
        /// <returns>
        /// The version, or an unread <see cref="Isdar"/>.
        /// </returns>
        /// <remarks>
        /// Under IL2CPP a package version is not written anywhere the file
        /// system can see: TextMeshPro is compiled into GameAssembly with
        /// everything else, and the <c>package.json</c> that named its version
        /// was a build-time artifact. So this reads it from the generated
        /// assembly's own version attribute when Il2CppInterop produced one, and
        /// otherwise returns unread.
        ///
        /// That is honest rather than unfortunate. A database entry that pins a
        /// package version can only apply to a game whose package version is
        /// known, and an entry that does not pin one applies regardless — so an
        /// unread package version narrows the candidate set to the entries that
        /// never depended on it, which is exactly right.
        /// </remarks>
        public static Isdar IsdarNusus()
        {
            try
            {
                Assembly[] tajammuat = AppDomain.CurrentDomain.GetAssemblies();
                for (int i = 0; i < tajammuat.Length; i++)
                {
                    AssemblyName ism = tajammuat[i].GetName();
                    if (!string.Equals(ism.Name, "Il2CppTextMeshPro", StringComparison.Ordinal)
                        && !string.Equals(ism.Name, "Unity.TextMeshPro", StringComparison.Ordinal))
                    {
                        continue;
                    }
                    Version? raqm = ism.Version;
                    if (raqm is null)
                    {
                        continue;
                    }
                    string nass = $"{raqm.Major}.{raqm.Minor}.{(raqm.Build < 0 ? 0 : raqm.Build)}";
                    if (Isdar.HawilHallil(nass, out Isdar isdar, out _))
                    {
                        return isdar;
                    }
                }
            }
            catch (Exception)
            {
                // The package version is optional by design; a reflection
                // failure here narrows the candidate set rather than breaking
                // the lookup, so it is not worth a log line at startup.
            }
            return default;
        }

        /// <summary>The architecture this process is executing as.</summary>
        /// <returns>The architecture the database is keyed by.</returns>
        /// <remarks>
        /// The process, not the operating system. A 32-bit build running under
        /// WOW64 on a 64-bit Windows has 32-bit code in it, and a pattern
        /// written against x86-64 prologues will not match a single function in
        /// it.
        /// </remarks>
        public static Binya BinyatAlAamaliya()
        {
            return RuntimeInformation.ProcessArchitecture switch
            {
                Architecture.Arm64 => Binya.Aarch64,
                Architecture.X64 => Binya.X8664,
                Architecture.X86 => Binya.X86,
                _ => Binya.X8664,
            };
        }

        /// <summary>The platform this process is running on.</summary>
        /// <returns>The platform the database is keyed by.</returns>
        /// <remarks>
        /// Part of the key because the compiler differs: MSVC, Clang and NDK
        /// Clang emit different prologues for the same C++, so a pattern that
        /// matches on Windows will not match the same function on Linux.
        /// </remarks>
        public static Minassa MinassatAlAala()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return Minassa.Windows;
            }
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                return Minassa.Mac;
            }
            if (OperatingSystem.IsAndroid())
            {
                return Minassa.Android;
            }
            return Minassa.Linux;
        }

        /// <summary>
        /// The game's data folder, or <c>null</c> when it cannot be located.
        /// </summary>
        /// <returns>The folder.</returns>
        /// <remarks>
        /// Derived from the executable's own path rather than from any engine
        /// call, for the same reason as everything else here.
        /// </remarks>
        public static string? DalilBayanat()
        {
            try
            {
                ProcessModule? wahda = Process.GetCurrentProcess().MainModule;
                string? tanfidhi = wahda?.FileName;
                if (string.IsNullOrEmpty(tanfidhi))
                {
                    return null;
                }
                string? dalil = Path.GetDirectoryName(tanfidhi);
                string ism = Path.GetFileNameWithoutExtension(tanfidhi);
                if (string.IsNullOrEmpty(dalil) || string.IsNullOrEmpty(ism))
                {
                    return null;
                }

                string maa = Path.Combine(dalil, ism + "_Data");
                if (Directory.Exists(maa))
                {
                    return maa;
                }

                // macOS puts the data inside the bundle rather than beside the
                // executable, and the executable is two levels down from it.
                string? contents = Path.GetDirectoryName(dalil);
                if (contents is not null)
                {
                    string maaMac = Path.Combine(contents, "Resources", "Data");
                    if (Directory.Exists(maaMac))
                    {
                        return maaMac;
                    }
                }
                return null;
            }
            catch (Exception)
            {
                return null;
            }
        }

        private static string? MinGlobalGameManagers()
        {
            string? dalil = DalilBayanat();
            if (dalil is null)
            {
                sababAkhir = "the game's data folder could not be located from the executable "
                    + "path";
                return null;
            }
            string masar = Path.Combine(dalil, "globalgamemanagers");
            if (!File.Exists(masar))
            {
                sababAkhir = $"{masar} does not exist";
                return null;
            }
            return MinRasMalaf(masar);
        }

        private static string? MinDataUnity3d()
        {
            string? dalil = DalilBayanat();
            if (dalil is null)
            {
                return null;
            }
            string masar = Path.Combine(dalil, "data.unity3d");
            if (!File.Exists(masar))
            {
                return null;
            }
            return MinRasMalaf(masar);
        }

        /// <summary>
        /// Reads the first printable NUL-terminated run that looks like a Unity
        /// version out of a file's head.
        /// </summary>
        /// <remarks>
        /// Bounded to the first kilobyte and to thirty-two bytes of candidate,
        /// and it accepts a run only if it looks like a Unity version — digits,
        /// dots, and one of the stage letters. Accepting any printable run would
        /// return the file's format tag or a compression name, both of which sit
        /// nearby, and either would parse into something that is not a version.
        /// </remarks>
        private static string? MinRasMalaf(string masar)
        {
            try
            {
                byte[] ras = new byte[AqsaIzahatIsdar];
                int maqru;
                using (FileStream tayyar = new FileStream(
                    masar, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
                {
                    maqru = tayyar.Read(ras, 0, ras.Length);
                }
                if (maqru <= 0)
                {
                    sababAkhir = $"{masar} is empty";
                    return null;
                }

                for (int i = 0; i < maqru; i++)
                {
                    if (!Raqm(ras[i]))
                    {
                        continue;
                    }
                    int j = i;
                    while (j < maqru && j - i < AqsaTulIsdar && Harf(ras[j]))
                    {
                        j++;
                    }
                    if (j >= maqru || ras[j] != 0)
                    {
                        continue;
                    }
                    string murashah = Encoding.ASCII.GetString(ras, i, j - i);
                    if (YushbihIsdar(murashah))
                    {
                        return murashah;
                    }
                    i = j;
                }

                sababAkhir = $"{masar} carries no version string in its first "
                    + $"{AqsaIzahatIsdar} bytes";
                return null;
            }
            catch (Exception khata)
            {
                sababAkhir = $"{masar} could not be read: {khata.Message}";
                return null;
            }
        }

        private static string? MinMawaridAlTanfidhi()
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return null;
            }
            try
            {
                ProcessModule? wahda = Process.GetCurrentProcess().MainModule;
                string? tanfidhi = wahda?.FileName;
                if (string.IsNullOrEmpty(tanfidhi))
                {
                    return null;
                }
                FileVersionInfo maalumat = FileVersionInfo.GetVersionInfo(tanfidhi);

                // Unity stamps the engine version into the product version for
                // most build configurations, and into the file version for the
                // rest. Neither is guaranteed, so both are tried and neither is
                // trusted beyond looking like a version.
                string? mintaj = maalumat.ProductVersion;
                if (mintaj is not null && YushbihIsdar(mintaj.Trim()))
                {
                    return mintaj.Trim();
                }
                string? malaf = maalumat.FileVersion;
                if (malaf is not null && YushbihIsdar(malaf.Trim()))
                {
                    return malaf.Trim();
                }
                return null;
            }
            catch (Exception)
            {
                return null;
            }
        }

        /// <summary>
        /// The last resort: ask the engine, which only works when the metadata
        /// is readable — that is, when the answer was least needed.
        /// </summary>
        private static string? MinAlMuharrik()
        {
            try
            {
                Assembly[] tajammuat = AppDomain.CurrentDomain.GetAssemblies();
                for (int i = 0; i < tajammuat.Length; i++)
                {
                    Type? naw = tajammuat[i].GetType("UnityEngine.Application", false);
                    PropertyInfo? khasiya = naw?.GetProperty(
                        "unityVersion", BindingFlags.Public | BindingFlags.Static);
                    if (khasiya is null)
                    {
                        continue;
                    }
                    object? qeema = khasiya.GetValue(null);
                    string? nass = qeema as string;
                    if (!string.IsNullOrEmpty(nass) && YushbihIsdar(nass))
                    {
                        return nass;
                    }
                }
                return null;
            }
            catch (Exception)
            {
                return null;
            }
        }

        private static bool YushbihIsdar(string nass)
        {
            if (nass.Length < 5 || nass.Length > AqsaTulIsdar)
            {
                return false;
            }
            if (nass[0] < '0' || nass[0] > '9')
            {
                return false;
            }
            int nuqat = 0;
            for (int i = 0; i < nass.Length; i++)
            {
                char h = nass[i];
                if (h == '.')
                {
                    nuqat++;
                    continue;
                }
                if (h >= '0' && h <= '9')
                {
                    continue;
                }
                if (h == 'f' || h == 'b' || h == 'a' || h == 'p' || h == 'c' || h == 'x')
                {
                    continue;
                }
                return false;
            }
            return nuqat >= 2;
        }

        private static bool Raqm(byte b)
        {
            return b >= (byte)'0' && b <= (byte)'9';
        }

        private static bool Harf(byte b)
        {
            return b >= 0x20 && b < 0x7F;
        }
    }
}
