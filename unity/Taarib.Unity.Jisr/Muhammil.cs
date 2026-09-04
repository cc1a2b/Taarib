// المحمّل — the loader, and the version refusal.
//
// A Unity game process gives no guarantees about where a dynamic loader will
// look, and one guarantee about what it will not do: it will not search
// BepInEx/plugins/Taarib/jisr/<arch>/ on its own. So the loader does not
// hope. It resolves the one correct file for this process — by pointer size
// and process architecture, from the patch's own directory — and loads it by
// absolute path before any DllImport in Jisr runs. The bare import name
// "taarib_jisr" then binds to the module already resident in the process:
// on Windows the loader matches loaded modules by base name before searching;
// on Linux glibc matches the DT_SONAME (libtaarib_jisr.so) the Rust build
// stamps into the library; on macOS dyld matches the install name
// (libtaarib_jisr.dylib) recorded the same way. If any of that fails, the
// failure is caught here and rethrown naming the file that was loaded and
// the binding that did not happen — never left as a bare
// DllNotFoundException three frames deep in a takeover patch.
//
// The first native call ever made is taarib_abi_isdar, and a major version
// mismatch refuses to initialise with a sentence a user can read in the
// BepInEx log. A minor ahead of ours is fine — additive by contract. A major
// apart means the structs in Anwa.cs describe memory the library does not,
// and every call after that point would be silent corruption in someone
// else's game; refusal is the only honest behaviour.

using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// Loads the native <c>taarib_jisr</c> library that matches this process
    /// — architecture and platform — from the installed patch directory, and
    /// verifies the ABI major version before any other native call is
    /// possible. Everything public in this assembly that reaches native code
    /// funnels through <see cref="Taakkad"/>, so there is no path around the
    /// refusal.
    /// </summary>
    public static class Muhammil
    {
        /// <summary>
        /// The ABI major version this assembly's structs and imports were
        /// written against — the frozen layouts of
        /// <c>crates/taarib-jisr/src/anwa.rs</c>. Bumped only together with
        /// them.
        /// </summary>
        public const uint IsdarKabirMadum = 1;

        private static readonly object Qufl = new object();
        private static bool muhammal;
        private static string? masarAsli;
        private static uint kabir;
        private static uint sagheer;

        /// <summary>
        /// Whether the library is loaded and its ABI major verified. False
        /// both before <see cref="Tahmeel"/> and after a refused mismatch.
        /// </summary>
        public static bool Muhammal
        {
            get
            {
                lock (Qufl)
                {
                    return muhammal;
                }
            }
        }

        /// <summary>The verified ABI major version of the loaded library.</summary>
        /// <exception cref="KhataTaarib">The library is not loaded.</exception>
        public static uint IsdarKabir
        {
            get
            {
                lock (Qufl)
                {
                    TahaqqaqMuhammal();
                    return kabir;
                }
            }
        }

        /// <summary>The ABI minor version of the loaded library.</summary>
        /// <exception cref="KhataTaarib">The library is not loaded.</exception>
        public static uint IsdarSagheer
        {
            get
            {
                lock (Qufl)
                {
                    TahaqqaqMuhammal();
                    return sagheer;
                }
            }
        }

        /// <summary>
        /// The absolute path of the library file that was loaded, for the
        /// startup log line — a support thread that can see which file a
        /// machine actually loaded is a support thread that ends.
        /// </summary>
        /// <exception cref="KhataTaarib">The library is not loaded.</exception>
        public static string MasarMaktaba
        {
            get
            {
                lock (Qufl)
                {
                    TahaqqaqMuhammal();
                    return masarAsli!;
                }
            }
        }

        /// <summary>
        /// Loads and verifies the native library from an installed patch
        /// directory — <c>BepInEx/plugins/Taarib</c>, the directory this
        /// assembly itself sits in. The library is expected at
        /// <c>jisr/&lt;arch&gt;/</c> under it (with <c>&lt;arch&gt;/</c>
        /// accepted directly for a hand-assembled layout), where
        /// <c>&lt;arch&gt;</c> is <c>x86</c>, <c>x64</c> or <c>arm64</c> as
        /// this process requires. Idempotent for the same directory; a
        /// second call naming a different one refuses, because two copies of
        /// the native library in one process would leave every import bound
        /// to whichever loaded first.
        /// </summary>
        /// <param name="dalilRuqaa">The patch directory, absolute.</param>
        /// <exception cref="ArgumentNullException"><paramref name="dalilRuqaa"/> is null.</exception>
        /// <exception cref="KhataTaarib">
        /// No library file exists for this architecture (the message names
        /// every path probed and how the architecture was decided); the file
        /// exists but the operating system refused it (wrong architecture,
        /// missing dependency); the import name failed to bind to the loaded
        /// module; or the ABI major does not match
        /// <see cref="IsdarKabirMadum"/>.
        /// </exception>
        public static void Tahmeel(string dalilRuqaa)
        {
            if (dalilRuqaa is null)
            {
                throw new ArgumentNullException(nameof(dalilRuqaa));
            }
            lock (Qufl)
            {
                TahmeelDakhili(dalilRuqaa);
            }
        }

        /// <summary>
        /// Ensures the library is loaded and verified, resolving the patch
        /// directory from this assembly's own location when
        /// <see cref="Tahmeel"/> has not been called explicitly. Every
        /// public entry in this assembly that can be the first native call
        /// runs through here, which is what makes the version check
        /// unavoidable rather than merely recommended.
        /// </summary>
        /// <exception cref="KhataTaarib">Loading or verification failed; see <see cref="Tahmeel"/>.</exception>
        public static void Taakkad()
        {
            lock (Qufl)
            {
                if (muhammal)
                {
                    return;
                }
                TahmeelDakhili(DalilTilqai());
            }
        }

        /// <summary>
        /// The loaded library's human-readable version string, for the
        /// startup log line beside <see cref="MasarMaktaba"/>.
        /// </summary>
        /// <returns>The version string, or empty if the library declines to say.</returns>
        /// <exception cref="KhataTaarib">The library is not loaded.</exception>
        public static string IsdarNass()
        {
            lock (Qufl)
            {
                TahaqqaqMuhammal();
            }
            return Jisr.IqraIsdarNass();
        }

        // -------------------------------------------------------------------
        // Resolution
        // -------------------------------------------------------------------

        private static void TahaqqaqMuhammal()
        {
            if (!muhammal)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    string.Empty,
                    "مكتبة تعريب الأصلية لم تُحمَّل بعد؛ يجب استدعاء Muhammil.Tahmeel بمجلد الرقعة أولًا.",
                    "The Taarib native library is not loaded yet; Muhammil.Tahmeel must run with the patch directory first.",
                    Khutwa.IadatTarkibIttar);
            }
        }

        private static void TahmeelDakhili(string dalil)
        {
            string dalilKamil = Path.GetFullPath(dalil);

            if (masarAsli is not null)
            {
                // A native module is already resident. Same directory: just
                // re-verify (a refused mismatch stays refused, loudly, every
                // time). Different directory: refuse — the resident module
                // is the one every import is bound to, and pretending to
                // swap it would be lying to the caller.
                string dalilAsli = Path.GetDirectoryName(Path.GetDirectoryName(masarAsli)) ?? string.Empty;
                bool nafsuh = masarAsli.StartsWith(dalilKamil, StringComparison.OrdinalIgnoreCase)
                    || dalilAsli.StartsWith(dalilKamil, StringComparison.OrdinalIgnoreCase);
                if (!nafsuh)
                {
                    throw new KhataTaarib(
                        Ramz.QeemaBatila,
                        string.Empty,
                        $"مكتبة تعريب الأصلية محمَّلة أصلًا من \"{masarAsli}\"، ولا يمكن تحميل نسخة أخرى من \"{dalilKamil}\" في العملية نفسها.",
                        $"The Taarib native library is already resident from \"{masarAsli}\"; a second copy from \"{dalilKamil}\" cannot be loaded into the same process.",
                        Khutwa.IadatTarkibIttar);
                }
                TahaqqaqIsdar(masarAsli);
                return;
            }

            (string imara, string bayanImara) = ImaratAmalia();
            string ism = IsmMaktaba();

            string awwal = Path.Combine(dalilKamil, "jisr", imara, ism);
            string thani = Path.Combine(dalilKamil, imara, ism);
            string masar;
            if (File.Exists(awwal))
            {
                masar = awwal;
            }
            else if (File.Exists(thani))
            {
                masar = thani;
            }
            else
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    string.Empty,
                    $"لم يوجد ملف مكتبة تعريب لمعمارية هذه العملية ({bayanImara}). بُحث في: \"{awwal}\" ثم \"{thani}\". أعد تثبيت الرقعة من تعريب ستوديو.",
                    $"No Taarib native library exists for this process ({bayanImara}). Looked in: \"{awwal}\" then \"{thani}\". Reinstall the patch from Taarib Studio.",
                    Khutwa.IadatTarkibIttar);
            }

            HammilAsli(masar, bayanImara);
            masarAsli = masar;
            TahaqqaqIsdar(masar);
        }

        /// <summary>
        /// The patch directory when the adapter did not name one: the
        /// directory this assembly was loaded from, which the install layout
        /// guarantees is <c>BepInEx/plugins/Taarib</c>. An assembly loaded
        /// from memory has no location; that case falls back to the process
        /// base directory, and if the library is not there either, the
        /// probe-failure message names both candidates so the fix — call
        /// <see cref="Tahmeel"/> explicitly — is evident.
        /// </summary>
        private static string DalilTilqai()
        {
            string mawqi = typeof(Muhammil).Assembly.Location;
            if (!string.IsNullOrEmpty(mawqi))
            {
                string? dalil = Path.GetDirectoryName(mawqi);
                if (!string.IsNullOrEmpty(dalil))
                {
                    return dalil;
                }
            }
            return AppContext.BaseDirectory;
        }

        /// <summary>
        /// Decides the architecture subdirectory. Pointer size is the
        /// authority on bitness — it is a property of the running process
        /// and cannot misreport — and <see cref="RuntimeInformation"/>
        /// distinguishes the ARM family from x86, where older Mono builds
        /// have been known to answer for the operating system rather than
        /// the process. A 32-bit Unity process therefore always gets the
        /// 32-bit library, whatever the OS is.
        /// </summary>
        private static (string Imara, string Bayan) ImaratAmalia()
        {
            Architecture mimari = RuntimeInformation.ProcessArchitecture;
            bool muashirat64 = IntPtr.Size == 8;

            if (!muashirat64 && mimari == Architecture.Arm)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMadum,
                    string.Empty,
                    "معمارية ARM ذات 32 بت ليست من أهداف تعريب؛ لا توجد مكتبة أصلية لها.",
                    "32-bit ARM is not a Taarib target; no native library exists for it.",
                    Khutwa.FathTashkhis);
            }

            string imara;
            if (muashirat64 && mimari == Architecture.Arm64)
            {
                imara = "arm64";
            }
            else if (muashirat64)
            {
                imara = "x64";
            }
            else
            {
                imara = "x86";
            }

            string bayan = $"{IntPtr.Size * 8}-bit, {mimari}, chose \"{imara}\"";
            return (imara, bayan);
        }

        private static string IsmMaktaba()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return "taarib_jisr.dll";
            }
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                return "libtaarib_jisr.so";
            }
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                return "libtaarib_jisr.dylib";
            }
            throw new KhataTaarib(
                Ramz.GhayrMadum,
                string.Empty,
                "هذه المنصة ليست من منصات تعريب (وندوز، لينكس، ماك)؛ لا توجد مكتبة أصلية لها.",
                "This platform is not a Taarib platform (Windows, Linux, macOS); no native library exists for it.",
                Khutwa.FathTashkhis);
        }

        // -------------------------------------------------------------------
        // The native load
        // -------------------------------------------------------------------

        private static void HammilAsli(string masar, string bayanImara)
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                IntPtr maqbad = LoadLibraryW(masar);
                if (maqbad == IntPtr.Zero)
                {
                    int raqm = Marshal.GetLastWin32Error();
                    // 193, ERROR_BAD_EXE_FORMAT: the file is real but built
                    // for the other architecture — the exact mistake a
                    // by-hand install of the wrong subdirectory makes.
                    string arabi;
                    string injilizi;
                    if (raqm == 193)
                    {
                        arabi = $"ملف المكتبة \"{masar}\" مبني لمعمارية أخرى غير معمارية هذه العملية ({bayanImara}).";
                        injilizi = $"The library file \"{masar}\" is built for a different architecture than this process ({bayanImara}).";
                    }
                    else
                    {
                        arabi = $"رفض النظام تحميل \"{masar}\" (رمز وندوز {raqm}).";
                        injilizi = $"The operating system refused to load \"{masar}\" (Windows error {raqm}).";
                    }
                    throw new KhataTaarib(Ramz.GhayrMuhayyaa, string.Empty, arabi, injilizi, Khutwa.IadatTarkibIttar);
                }
                // The handle is deliberately dropped: the module is never
                // unloaded, because native code the process may still enter
                // must never be unmapped under it.
                return;
            }

            // RTLD_NOW so a missing dependency fails here, with a path in
            // hand, instead of at the first call into an unresolved symbol
            // mid-frame. RTLD_GLOBAL so the module's symbols and identity
            // are visible to the runtime's own dlopen when the bare import
            // name is resolved.
            bool mac = RuntimeInformation.IsOSPlatform(OSPlatform.OSX);
            int alam = mac ? 0x2 | 0x8 : 0x2 | 0x100;
            IntPtr natij = mac ? DlopenMac(masar, alam) : DlopenLinux(masar, alam);
            if (natij == IntPtr.Zero)
            {
                IntPtr sabab = mac ? DlerrorMac() : DlerrorLinux();
                string tafsir = sabab == IntPtr.Zero
                    ? "dlerror reported nothing"
                    : Marshal.PtrToStringAnsi(sabab) ?? "dlerror reported nothing";
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    string.Empty,
                    $"رفض النظام تحميل \"{masar}\" ({bayanImara}): {tafsir}",
                    $"The operating system refused to load \"{masar}\" ({bayanImara}): {tafsir}",
                    Khutwa.IadatTarkibIttar);
            }
            // As on Windows: the dlopen handle is dropped, never dlclosed.
        }

        // -------------------------------------------------------------------
        // The version refusal
        // -------------------------------------------------------------------

        private static void TahaqqaqIsdar(string masar)
        {
            uint k;
            uint s;
            int halat;
            try
            {
                halat = Jisr.taarib_abi_isdar(out k, out s);
            }
            catch (DllNotFoundException asl)
            {
                throw JisrLamYartabit(masar, asl);
            }
            catch (EntryPointNotFoundException asl)
            {
                throw JisrLamYartabit(masar, asl);
            }
            catch (BadImageFormatException asl)
            {
                throw JisrLamYartabit(masar, asl);
            }

            if (halat != Ramz.Najah)
            {
                throw new KhataTaarib(
                    halat,
                    string.Empty,
                    $"أعادت \"{masar}\" حالة فشل من استعلام الإصدار نفسه؛ الملف ليس مكتبة تعريب سليمة.",
                    $"\"{masar}\" returned a failing status from the version query itself; the file is not a healthy Taarib library.",
                    Khutwa.IadatTarkibIttar);
            }

            if (k != IsdarKabirMadum)
            {
                // Refused, and stays refused: muhammal remains false, so
                // every later Taakkad rethrows this sentence instead of
                // letting a single mismatched struct cross the boundary.
                throw new KhataTaarib(
                    Ramz.IsdarGhayrMutawafiq,
                    string.Empty,
                    $"إصدار واجهة مكتبة تعريب في \"{masar}\" هو {k}.{s}، وهذا الملحق مبني على الإصدار {IsdarKabirMadum}؛ رفض التحميل لأن تخطيطات الذاكرة بين الإصدارين مختلفة. حدِّث تعريب وأعد تثبيت الرقعة حتى يتطابقا.",
                    $"The Taarib ABI in \"{masar}\" is version {k}.{s}, and this plugin was built against major {IsdarKabirMadum}; loading is refused because the two disagree about memory layouts. Update Taarib and reinstall the patch so they match.",
                    Khutwa.TahdithTaarib);
            }

            kabir = k;
            sagheer = s;
            muhammal = true;
        }

        private static KhataTaarib JisrLamYartabit(string masar, Exception asl)
        {
            // The file loaded, but the runtime could not bind the bare
            // import name "taarib_jisr" to it. On Linux that binding rides
            // on the DT_SONAME (libtaarib_jisr.so) stamped at build time; on
            // macOS, on the install name; on Windows, on the module's base
            // name. Naming the mechanism turns an afternoon of guessing
            // into one readelf.
            return new KhataTaarib(
                Ramz.GhayrMuhayyaa,
                string.Empty,
                $"حُمِّل الملف \"{masar}\" لكن تعذّر ربط اسم الاستيراد \"taarib_jisr\" به ({asl.GetType().Name}: {asl.Message}).",
                $"The file \"{masar}\" was loaded, but the import name \"taarib_jisr\" could not be bound to it ({asl.GetType().Name}: {asl.Message}). On Linux this binding relies on the library's DT_SONAME, on macOS on its install name, on Windows on its base name.",
                Khutwa.IadatTarkibIttar);
        }

        // -------------------------------------------------------------------
        // Platform loaders
        // -------------------------------------------------------------------

        [DllImport("kernel32", EntryPoint = "LoadLibraryW", SetLastError = true, CharSet = CharSet.Unicode, ExactSpelling = true)]
        private static extern IntPtr LoadLibraryW([MarshalAs(UnmanagedType.LPWStr)] string masar);

        [DllImport("libdl.so.2", EntryPoint = "dlopen")]
        private static extern IntPtr DlopenLinux(string masar, int alam);

        [DllImport("libdl.so.2", EntryPoint = "dlerror")]
        private static extern IntPtr DlerrorLinux();

        [DllImport("/usr/lib/libSystem.B.dylib", EntryPoint = "dlopen")]
        private static extern IntPtr DlopenMac(string masar, int alam);

        [DllImport("/usr/lib/libSystem.B.dylib", EntryPoint = "dlerror")]
        private static extern IntPtr DlerrorMac();
    }
}
