// تخصيص — executable memory placed close enough to jump to in five bytes.
//
// An x86-64 `JMP rel32` reaches a signed 2 GB from the instruction after it.
// That is the whole reason a detour can be five bytes instead of fourteen: if
// the trampoline lives within +/-2 GB of the target we jump to it with a
// rel32, and if it does not we need a full `movabs`-into-register-then-jump,
// which is fourteen bytes and cannot fit in a short prologue. So the
// trampoline's ADDRESS is not incidental — it is a correctness constraint, and
// meeting it means allocating pages near a given address rather than wherever
// the allocator pleases. This file probes the address space outward from the
// target until it finds a free span in range, and reserves it there.
//
// On ARM64 the reach is +/-128 MB for a `B`/`BL`, so a branch island is the
// normal case rather than the exception: a great many hooks will find the
// nearest free page is already outside 128 MB, and the detour then goes
// target -> island (a `B` in range) -> replacement (a full 64-bit load-and-jump
// in the island). The island still has to be within 128 MB; this allocator is
// what puts it there.
//
// W^X — WRITE XOR EXECUTE. A page is never mapped writable and executable at the
// same time. Not once, not briefly. We reserve it readable/writable, write the
// trampoline bytes, and only then reprotect it to readable/executable. The
// reason is not theoretical: a page that is writable and executable together is
// the single clearest signature of injected code, and it is exactly what every
// anti-cheat scanner and every hardened kernel (grsecurity, Windows CFG/ACG,
// Apple's hardened runtime) is watching for. Taarib is a guest in someone
// else's process — a text renderer, not an exploit — and a guest that maps RWX
// pages gets the host process killed by its own protections, which reads to the
// player as "this mod crashes my game". Never RWX; RW then RX.
//
// APPLE SILICON is the exception to the reprotect model, not to the rule. Its
// JIT pages are mapped once with `MAP_JIT` as read/execute and stay that way;
// writability is toggled per thread with `pthread_jit_write_protect_np` around
// each write, so the page is writable on this thread for the span of a memcpy
// and executable everywhere else. It is still never both at once from the CPU's
// point of view — the mechanism differs, the invariant holds.
//
// CACHE MAINTENANCE. After writing instructions you must make the instruction
// stream see them. On x86-64 the instruction cache is coherent with stores and
// the protection change already serialises, so nothing extra is needed. On ARM
// the I-cache is NOT coherent with the D-cache: the bytes are in memory, the
// core keeps executing a stale copy from its instruction cache, and the hook
// runs old garbage — intermittently, only on ARM, only sometimes, which is the
// worst kind of bug to be handed. So every write of code is followed by an
// explicit flush: `FlushInstructionCache` on Windows, `sys_icache_invalidate`
// on Apple platforms, `__clear_cache` on Linux. A missing flush is invisible on
// the x86 machine it was written on and a hard crash on the ARM handset it
// ships to.
//
// WHY THIS PROJECT MAY CALL P/INVOKE AT ALL. The Mono adapter forbids a single
// DllImport, and the IL2CPP resolver routes everything through Taarib's one C
// ABI. This file is the deliberate exception, and it is a narrow one: reserving
// executable memory at a chosen address, changing page protection, toggling JIT
// writability and flushing an instruction cache are operating-system services
// with no managed equivalent and no place in Taarib's own C ABI, which knows
// nothing about detours. There is no portable .NET API for "give me an
// executable page within 2 GB of this address". So the imports live here, in
// one file, split into clearly marked per-platform regions, and nowhere else in
// the Khatf subsystem.

using System;
using System.Runtime.InteropServices;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Khatf
{
    /// <summary>
    /// One reserved run of executable memory near a target address — a
    /// trampoline, or an ARM branch island. Reserved writable, written, then
    /// sealed to read/execute; freed on dispose.
    /// </summary>
    /// <remarks>
    /// The lifecycle is fixed and one-way: allocate through
    /// <see cref="Takhsis.Ihjuz"/>, write with <see cref="Iktub"/> as many times
    /// as needed, call <see cref="Uhkim"/> once to make it executable, then use
    /// <see cref="Unwan"/>. Writing after <see cref="Uhkim"/> is refused, because
    /// on every platform but one the page is no longer writable and the write
    /// would fault; forbidding it uniformly means the W^X invariant reads the
    /// same in the code as it does in the hardware.
    /// </remarks>
    public sealed unsafe class Jazira : IDisposable
    {
        private IntPtr unwan;
        private readonly nuint hajm;
        private readonly bool jit;
        private bool muhkam;
        private bool muharrar;

        internal Jazira(IntPtr unwan, nuint hajm, bool jit)
        {
            this.unwan = unwan;
            this.hajm = hajm;
            this.jit = jit;
        }

        /// <summary>The base address of the reserved run.</summary>
        public IntPtr Unwan => unwan;

        /// <summary>Its size in bytes, rounded up to whole pages.</summary>
        public nuint Hajm => hajm;

        /// <summary>Whether <see cref="Uhkim"/> has sealed it to read/execute.</summary>
        public bool Muhkam => muhkam;

        /// <summary>
        /// Copies bytes into the run at an offset, handling the Apple Silicon
        /// JIT write-protect toggle so the page is writable for exactly the span
        /// of the copy and executable otherwise.
        /// </summary>
        /// <param name="bayt">The bytes to write.</param>
        /// <param name="izaha">The offset from <see cref="Unwan"/> to write at.</param>
        /// <exception cref="ObjectDisposedException">The run was freed.</exception>
        /// <exception cref="InvalidOperationException">
        /// <see cref="Uhkim"/> already sealed it; it is no longer writable.
        /// </exception>
        /// <exception cref="ArgumentOutOfRangeException">
        /// The write would fall outside the reserved run, which would be a stray
        /// store into whatever page follows it.
        /// </exception>
        public void Iktub(ReadOnlySpan<byte> bayt, int izaha)
        {
            if (muharrar)
            {
                throw new ObjectDisposedException(nameof(Jazira));
            }
            if (muhkam)
            {
                throw new InvalidOperationException(
                    "The island was sealed to execute-only; write before sealing it.");
            }
            if (izaha < 0 || (long)izaha + bayt.Length > (long)hajm)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(izaha), "The write would fall outside the reserved run.");
            }
            if (bayt.IsEmpty)
            {
                return;
            }

            if (jit)
            {
                // Enter writable state on THIS thread only; the page stays
                // execute-protected on every other thread, so the pair is never
                // observably writable-and-executable at once.
                Takhsis.JitWritable(true);
            }
            // Sound: `unwan` addresses `hajm` bytes this object reserved and
            // owns, and the bounds check above guarantees the destination span
            // lies wholly inside that reservation, so the copy cannot touch a
            // neighbouring page.
            Span<byte> wajha = new Span<byte>((byte*)unwan + izaha, bayt.Length);
            bayt.CopyTo(wajha);
            if (jit)
            {
                Takhsis.JitWritable(false);
            }
        }

        /// <summary>
        /// Seals the run to read/execute and flushes the instruction cache over
        /// it. Idempotent.
        /// </summary>
        /// <exception cref="ObjectDisposedException">The run was freed.</exception>
        /// <exception cref="KhataTaarib">
        /// The protection change was refused by the operating system, or the
        /// instruction cache could not be flushed on an architecture that
        /// requires it.
        /// </exception>
        public void Uhkim()
        {
            if (muharrar)
            {
                throw new ObjectDisposedException(nameof(Jazira));
            }
            if (muhkam)
            {
                return;
            }

            if (!jit)
            {
                // The RW -> RX transition. Never a widening to RWX; the old
                // protection is discarded, not extended.
                Takhsis.Himaya(unwan, hajm, tanfidh: true);
            }

            Takhsis.MashDhakira(unwan, hajm);
            muhkam = true;
        }

        /// <summary>Frees the reserved run.</summary>
        public void Dispose()
        {
            if (muharrar)
            {
                return;
            }
            muharrar = true;
            if (unwan != IntPtr.Zero)
            {
                Takhsis.Harrir(unwan, hajm);
                unwan = IntPtr.Zero;
            }
        }
    }

    /// <summary>
    /// Reserves executable memory within a bounded distance of a target address,
    /// and owns the per-platform mechanics of doing so under W^X.
    /// </summary>
    /// <remarks>
    /// Used only during hook installation. The probing walk it performs is
    /// O(free regions near the target) of cheap query syscalls and is not on any
    /// frame path.
    /// </remarks>
    public static class Takhsis
    {
        /// <summary>
        /// The reach of an x86-64 <c>JMP rel32</c>, less a 64 KB margin so the
        /// far edge of a multi-page island still lands inside the +/-2 GB window.
        /// </summary>
        public const long MadaX64 = 0x7FFF0000L;

        /// <summary>
        /// The reach of an ARM64 <c>B</c>/<c>BL</c>, less a 1 MB margin — +/-128 MB
        /// is why islands are routine on ARM rather than exceptional.
        /// </summary>
        public const long MadaArm64 = 0x07F00000L;

        private static readonly bool Nafidha =
            RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
        private static readonly bool Makintush =
            RuntimeInformation.IsOSPlatform(OSPlatform.OSX);
        private static readonly bool AppleSilicon =
            RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
            && RuntimeInformation.OSArchitecture == Architecture.Arm64;
        private static readonly bool Arm64 =
            RuntimeInformation.OSArchitecture == Architecture.Arm64;

        /// <summary>
        /// Reserves a writable run of at least <paramref name="hajm"/> bytes whose
        /// whole extent is within <paramref name="aqsaMada"/> of
        /// <paramref name="qurb"/>.
        /// </summary>
        /// <param name="qurb">The address the run must stay close to — the target.</param>
        /// <param name="hajm">The minimum size in bytes; rounded up to whole pages.</param>
        /// <param name="aqsaMada">
        /// The maximum distance any byte of the run may sit from
        /// <paramref name="qurb"/>: <see cref="MadaX64"/> for a rel32 trampoline,
        /// <see cref="MadaArm64"/> for an ARM island.
        /// </param>
        /// <returns>A writable run; call <see cref="Jazira.Uhkim"/> after writing.</returns>
        /// <exception cref="KhataTaarib">
        /// No free region was found in range, or the platform is one this
        /// allocator does not implement. A near-allocation failure is a refusal:
        /// a trampoline out of rel32 range cannot be reached by the patch, so
        /// installing one anyway would be a jump to nowhere.
        /// </exception>
        public static Jazira Ihjuz(IntPtr qurb, nuint hajm, long aqsaMada)
        {
            bool jit = AppleSilicon;
            nuint mahjuz;
            IntPtr p;
            if (Nafidha)
            {
                p = IhjuzNafidha(qurb, hajm, aqsaMada, out mahjuz);
            }
            else
            {
                p = IhjuzPosix(qurb, hajm, aqsaMada, jit, out mahjuz);
            }

            if (p == IntPtr.Zero)
            {
                throw new KhataTaarib(
                    Ramz.Dhakira,
                    "TAARIB-E-6520",
                    "تعذّر حجز ذاكرةٍ تنفيذيّةٍ ضمن مدى القفزة من الدالة الهدف؛ فضاء العناوين "
                        + "قريبها ممتلئ. رُفض التركيب بدل وضع المنطقة الوسيطة خارج المدى حيث "
                        + "لا تصلها القفزة.",
                    "Could not reserve executable memory within jump range of the target; "
                        + "the address space near it is full. The hook is refused rather than "
                        + "placing the trampoline out of range where the patch cannot reach it.",
                    Khutwa.FathTashkhis);
            }

            return new Jazira(p, mahjuz, jit);
        }

        // ===================================================================
        // P/Invoke — Windows
        // Every import is Cdecl-free (Win32 is stdcall on 32-bit, and the
        // default marshalling picks the platform convention correctly for these
        // kernel32 entry points, so no CallingConvention override is stated).
        // These bind to kernel32.dll, already resident in every process.
        // ===================================================================

        private const uint MEM_COMMIT = 0x1000;
        private const uint MEM_RESERVE = 0x2000;
        private const uint MEM_RELEASE = 0x8000;
        private const uint MEM_FREE = 0x10000;
        private const uint PAGE_READWRITE = 0x04;
        private const uint PAGE_EXECUTE_READ = 0x20;

        [StructLayout(LayoutKind.Sequential)]
        private struct SystemInfo
        {
            public ushort ProcessorArchitecture;
            public ushort Reserved;
            public uint PageSize;
            public IntPtr MinimumApplicationAddress;
            public IntPtr MaximumApplicationAddress;
            public UIntPtr ActiveProcessorMask;
            public uint NumberOfProcessors;
            public uint ProcessorType;
            public uint AllocationGranularity;
            public ushort ProcessorLevel;
            public ushort ProcessorRevision;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct MemoryBasicInformation
        {
            public IntPtr BaseAddress;
            public IntPtr AllocationBase;
            public uint AllocationProtect;
            public uint Alignment1;
            public UIntPtr RegionSize;
            public uint State;
            public uint Protect;
            public uint Type;
            public uint Alignment2;
        }

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern void GetSystemInfo(out SystemInfo info);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr VirtualAlloc(
            IntPtr address, nuint size, uint allocationType, uint protect);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool VirtualFree(IntPtr address, nuint size, uint freeType);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool VirtualProtect(
            IntPtr address, nuint size, uint newProtect, out uint oldProtect);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern nuint VirtualQuery(
            IntPtr address, out MemoryBasicInformation buffer, nuint length);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool FlushInstructionCache(
            IntPtr process, IntPtr baseAddress, nuint size);

        [DllImport("kernel32.dll")]
        private static extern IntPtr GetCurrentProcess();

        // ===================================================================
        // P/Invoke — POSIX (Linux and macOS)
        // libc for mmap/munmap/mprotect; libSystem for the two Apple-only cache
        // and JIT services; libgcc_s for the Linux instruction-cache flush.
        // The Apple and libgcc imports are declared unconditionally and called
        // only on the platform that has them, so a Linux process never binds the
        // Apple symbols and a Windows process binds none of this region.
        // ===================================================================

        private const int PROT_READ = 0x1;
        private const int PROT_WRITE = 0x2;
        private const int PROT_EXEC = 0x4;
        private const int MAP_PRIVATE = 0x2;
        private const int MAP_ANON_LINUX = 0x20;
        private const int MAP_ANON_OSX = 0x1000;
        private const int MAP_FIXED_NOREPLACE = 0x100000; // Linux >= 4.17
        private const int MAP_JIT = 0x800;                // macOS

        private static readonly IntPtr MAP_FAILED = new IntPtr(-1);

        [DllImport("libc", SetLastError = true, EntryPoint = "mmap")]
        private static extern IntPtr mmap(
            IntPtr addr, nuint length, int prot, int flags, int fd, nint offset);

        [DllImport("libc", SetLastError = true, EntryPoint = "munmap")]
        private static extern int munmap(IntPtr addr, nuint length);

        [DllImport("libc", SetLastError = true, EntryPoint = "mprotect")]
        private static extern int mprotect(IntPtr addr, nuint len, int prot);

        private static class Msx
        {
            [DllImport("libSystem.dylib", EntryPoint = "sys_icache_invalidate")]
            internal static extern void sys_icache_invalidate(IntPtr start, nuint len);

            [DllImport("libSystem.dylib", EntryPoint = "pthread_jit_write_protect_np")]
            internal static extern void pthread_jit_write_protect_np(int enabled);
        }

        [DllImport("libgcc_s.so.1", EntryPoint = "__clear_cache")]
        private static extern void __clear_cache(IntPtr begin, IntPtr end);

        // ===================================================================
        // Windows probing
        // ===================================================================

        private static IntPtr IhjuzNafidha(IntPtr qurb, nuint hajm, long mada, out nuint mahjuz)
        {
            GetSystemInfo(out SystemInfo si);
            long gran = si.AllocationGranularity == 0 ? 0x10000 : si.AllocationGranularity;
            nuint need = RoundSize(hajm, si.PageSize);
            mahjuz = need;

            long markaz = qurb.ToInt64();
            long lo = Math.Max(markaz - mada, si.MinimumApplicationAddress.ToInt64());
            long hiRaw = si.MaximumApplicationAddress.ToInt64() - (long)need;
            long hi = Math.Min(markaz + mada, hiRaw);
            nuint mbiSize = (nuint)Marshal.SizeOf<MemoryBasicInformation>();

            for (long delta = 0; delta <= mada; delta += gran)
            {
                for (int dir = -1; dir <= 1; dir += 2)
                {
                    long cand = AlignDown(markaz + (dir * delta), gran);
                    if (cand < lo || cand > hi)
                    {
                        continue;
                    }
                    IntPtr candPtr = new IntPtr(cand);
                    nuint hal = VirtualQuery(candPtr, out MemoryBasicInformation mbi, mbiSize);
                    if (hal == UIntPtr.Zero)
                    {
                        continue;
                    }
                    if (mbi.State != MEM_FREE || (ulong)mbi.RegionSize < (ulong)need)
                    {
                        continue;
                    }
                    IntPtr got = VirtualAlloc(
                        candPtr, need, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
                    if (got != IntPtr.Zero && FiMada(got, need, markaz, mada))
                    {
                        return got;
                    }
                    if (got != IntPtr.Zero)
                    {
                        VirtualFree(got, UIntPtr.Zero, MEM_RELEASE);
                    }
                    if (delta == 0)
                    {
                        break;
                    }
                }
            }
            return IntPtr.Zero;
        }

        // ===================================================================
        // POSIX probing
        // ===================================================================

        private static IntPtr IhjuzPosix(
            IntPtr qurb, nuint hajm, long mada, bool jit, out nuint mahjuz)
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Linux) && !Makintush)
            {
                mahjuz = 0;
                throw new KhataTaarib(
                    Ramz.GhayrMadum,
                    "TAARIB-E-6521",
                    "لا يدعم هذا الإصدار حجز الذاكرة التنفيذيّة على هذه المنصّة؛ لا يمكن تركيب "
                        + "تحويلٍ أصليّ هنا.",
                    "This build does not implement executable-memory reservation on this "
                        + "platform; a native detour cannot be installed here.",
                    Khutwa.FathTashkhis);
            }

            int ps = Environment.SystemPageSize;
            long gran = Math.Max(ps, 0x10000);
            nuint need = RoundSize(hajm, (nuint)ps);
            mahjuz = need;
            long markaz = qurb.ToInt64();
            long lo = Math.Max(markaz - mada, gran);
            long hi = markaz + mada;

            int anon = Makintush ? MAP_ANON_OSX : MAP_ANON_LINUX;
            int prot = jit ? (PROT_READ | PROT_EXEC) : (PROT_READ | PROT_WRITE);
            int baseFlags = MAP_PRIVATE | anon | (jit ? MAP_JIT : 0);

            for (long delta = 0; delta <= mada; delta += gran)
            {
                for (int dir = -1; dir <= 1; dir += 2)
                {
                    long cand = AlignDown(markaz + (dir * delta), gran);
                    if (cand < lo || cand > hi)
                    {
                        continue;
                    }

                    int flags = baseFlags;
                    IntPtr hint = new IntPtr(cand);
                    // Linux can demand the exact page without clobbering an
                    // existing mapping (MAP_FIXED_NOREPLACE). macOS has no such
                    // flag, so the address is a hint and the result is
                    // range-checked and discarded if the kernel placed it
                    // elsewhere — the probe-then-map fallback.
                    if (!Makintush)
                    {
                        flags |= MAP_FIXED_NOREPLACE;
                    }

                    IntPtr got = mmap(hint, need, prot, flags, -1, 0);
                    if (got == MAP_FAILED)
                    {
                        continue;
                    }
                    if (FiMada(got, need, markaz, mada))
                    {
                        return got;
                    }
                    // Out of range (older Linux ignoring the flag, or a macOS
                    // hint the kernel did not honour): give it back and keep
                    // probing rather than keeping a mapping the patch can't jump
                    // to.
                    munmap(got, need);
                    if (delta == 0)
                    {
                        break;
                    }
                }
            }
            return IntPtr.Zero;
        }

        // ===================================================================
        // Protection, freeing and cache maintenance
        // ===================================================================

        internal static void JitWritable(bool masmuh)
        {
            // Apple Silicon only: 0 makes the calling thread's JIT pages
            // writable, 1 returns them to execute-only. The toggle is per
            // thread, which is what keeps the page from ever being writable and
            // executable together as far as any other thread can observe.
            Msx.pthread_jit_write_protect_np(masmuh ? 0 : 1);
        }

        internal static void Himaya(IntPtr unwan, nuint hajm, bool tanfidh)
        {
            uint himaya = tanfidh ? PAGE_EXECUTE_READ : PAGE_READWRITE;
            if (Nafidha)
            {
                if (!VirtualProtect(unwan, hajm, himaya, out uint _))
                {
                    throw RfudHimaya();
                }
                return;
            }
            int prot = tanfidh ? (PROT_READ | PROT_EXEC) : (PROT_READ | PROT_WRITE);
            if (mprotect(unwan, hajm, prot) != 0)
            {
                throw RfudHimaya();
            }
        }

        /// <summary>
        /// Overwrites existing code — the target's prologue when installing, the
        /// saved original when removing — by dropping the page to writable,
        /// copying, restoring the page's own protection and flushing the
        /// instruction cache.
        /// </summary>
        /// <remarks>
        /// Execute is removed for the span of the write rather than adding write
        /// to an executable page, so the page is never writable-and-executable
        /// at once. The cost of that choice is a window in which the target page
        /// is not executable, which is only safe because no other thread is
        /// running the target's code while it is patched — the guarantee
        /// <see cref="Mihmaz"/> secures by patching at initialisation, before the
        /// game drives its own code.
        /// </remarks>
        /// <param name="hadaf">The address to overwrite.</param>
        /// <param name="bayt">The bytes to write there.</param>
        /// <exception cref="KhataTaarib">The protection change was refused.</exception>
        internal static unsafe void IktubFawqShifra(IntPtr hadaf, ReadOnlySpan<byte> bayt)
        {
            if (bayt.IsEmpty)
            {
                return;
            }
            long ps = Environment.SystemPageSize;
            long start = hadaf.ToInt64();
            long alignedStart = start & ~(ps - 1);
            long endAddr = start + bayt.Length;
            nuint span = (nuint)(endAddr - alignedStart);
            IntPtr aligned = new IntPtr(alignedStart);
            nuint tul = (nuint)bayt.Length;

            if (Nafidha)
            {
                if (!VirtualProtect(aligned, span, PAGE_READWRITE, out uint qadim))
                {
                    throw RfudHimaya();
                }
                // Sound: the page range [aligned, endAddr) was just made writable
                // and wholly contains [hadaf, hadaf+len), so this copy stays
                // inside pages we can write.
                bayt.CopyTo(new Span<byte>((byte*)hadaf, bayt.Length));
                VirtualProtect(aligned, span, qadim, out uint _);
                FlushInstructionCache(GetCurrentProcess(), hadaf, tul);
                return;
            }

            if (mprotect(aligned, span, PROT_READ | PROT_WRITE) != 0)
            {
                throw RfudHimaya();
            }
            // Sound: mprotect just made the whole page range covering
            // [hadaf, hadaf+len) writable, and the destination span is exactly
            // that byte range, so the copy cannot reach an unwritable page.
            bayt.CopyTo(new Span<byte>((byte*)hadaf, bayt.Length));
            mprotect(aligned, span, PROT_READ | PROT_EXEC);
            MashDhakira(hadaf, tul);
        }

        internal static void Harrir(IntPtr unwan, nuint hajm)
        {
            if (Nafidha)
            {
                VirtualFree(unwan, UIntPtr.Zero, MEM_RELEASE);
            }
            else
            {
                munmap(unwan, hajm);
            }
        }

        internal static void MashDhakira(IntPtr unwan, nuint hajm)
        {
            if (Nafidha)
            {
                FlushInstructionCache(GetCurrentProcess(), unwan, hajm);
                return;
            }
            if (Makintush)
            {
                Msx.sys_icache_invalidate(unwan, hajm);
                return;
            }

            // Linux. On x86 the I-cache is coherent and a missing flush is
            // harmless, so an absent libgcc_s is tolerated; on ARM the flush is
            // mandatory and its absence must fail loudly rather than ship a hook
            // that runs stale bytes on some cores.
            IntPtr end = new IntPtr(unwan.ToInt64() + (long)hajm);
            try
            {
                __clear_cache(unwan, end);
            }
            catch (Exception khata) when (
                khata is DllNotFoundException || khata is EntryPointNotFoundException)
            {
                if (Arm64)
                {
                    throw new KhataTaarib(
                        Ramz.GhayrMadum,
                        "TAARIB-E-6522",
                        "تعذّر مسح ذاكرة التعليمات بعد كتابة الرقعة على معمار ARM؛ من دون "
                            + "هذا المسح قد تنفّذ النواة بايتاتٍ قديمة، فرُفض التركيب بدل "
                            + "خطرٍ يظهر على بعض الأجهزة فقط.",
                        "The instruction cache could not be flushed after writing the patch "
                            + "on ARM; without it a core may execute stale bytes, so the hook "
                            + "is refused rather than risking a fault seen only on some "
                            + "devices.",
                        Khutwa.FathTashkhis);
                }
            }
        }

        private static KhataTaarib RfudHimaya()
        {
            return new KhataTaarib(
                Ramz.Dhakira,
                "TAARIB-E-6523",
                "رفض نظام التشغيل تغيير حماية صفحة المنطقة الوسيطة إلى التنفيذ؛ قد تمنع ذلك "
                    + "سياسةُ حمايةٍ مشدَّدة. رُفض التركيب بدل ترك صفحةٍ قابلةٍ للكتابة والتنفيذ "
                    + "معًا.",
                "The operating system refused to change the trampoline page's protection to "
                    + "executable; a hardened protection policy can forbid it. The hook is "
                    + "refused rather than leaving a page both writable and executable.",
                Khutwa.ManhSalahiya);
        }

        private static bool FiMada(IntPtr unwan, nuint hajm, long markaz, long mada)
        {
            long a = unwan.ToInt64();
            long b = a + (long)hajm - 1;
            return a >= markaz - mada && b <= markaz + mada;
        }

        private static nuint RoundSize(nuint hajm, nuint page)
        {
            if (page == UIntPtr.Zero)
            {
                return hajm;
            }
            ulong p = page;
            ulong h = hajm;
            ulong r = (h + p - 1) / p * p;
            return (nuint)r;
        }

        private static long AlignDown(long v, long gran)
        {
            return v - (((v % gran) + gran) % gran);
        }
    }
}
